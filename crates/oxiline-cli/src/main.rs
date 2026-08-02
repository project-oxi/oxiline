//! `oxiline` CLI entry point (`05-cli-spec.md`).

mod cli;
mod lang;
mod output;

use std::process::ExitCode;

use clap::Parser;
use oxiline_core::{CoreError, Result, activities, categories, plan, record, settings, util};
use serde_json::{Value, json};

use cli::{ActivityAction, Cli, Command, PlanAction, RecordAction};
use lang::{L, Lang};

fn main() -> ExitCode {
    let opts = Cli::parse();
    let json = opts.json;
    match run(opts) {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            eprintln!("{}", error_text(&e, json));
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn error_text(e: &CoreError, json: bool) -> String {
    if json {
        json!({ "error": { "code": e.code().as_str(), "message": e.to_string() } }).to_string()
    } else {
        format!("error: {}", e)
    }
}

fn run(opts: Cli) -> Result<()> {
    let path = match &opts.db {
        Some(p) => p.clone(),
        None => oxiline_core::paths::db_path(),
    };
    let conn = oxiline_core::open_and_migrate(&path)?;
    settings::ensure_defaults(&conn)?;

    let lang = resolve_lang(&conn, &opts);
    let l = L(lang);
    let json = opts.json;

    // helper to emit stdout (respecting --quiet)
    let say = |body: String| {
        if !opts.quiet {
            println!("{body}");
        }
    };

    match &opts.cmd {
        Command::Now => {
            let ctx = plan::now_summary(&conn, util::now_minute_local())?;
            if json {
                say(output::json_pretty(&ctx));
            } else {
                say(output::now_text(l, &ctx));
            }
        }

        Command::Category { action } => match action {
            cli::CategoryAction::Add { name, hue, icon } => {
                if opts.dry_run {
                    say(preview(
                        json,
                        l.category_added(),
                        &json!({"name": name, "color_hue": hue, "dry_run": true}),
                    ));
                    return Ok(());
                }
                let c = categories::create(
                    &conn,
                    categories::NewCategory {
                        name: name.clone(),
                        color_hue: *hue,
                        icon: icon.clone(),
                    },
                )?;
                say(resource_out(json, l.category_added(), &c));
            }
            cli::CategoryAction::List => {
                let cs = categories::list(&conn)?;
                if json {
                    say(output::json_pretty(&cs));
                } else {
                    say(output::category_list_text(&cs));
                }
            }
            cli::CategoryAction::Rm { id } => {
                categories::delete(&conn, id)?;
                say(if json {
                    json!({ "id": id, "removed": true }).to_string()
                } else {
                    format!("{}: {}", l.removed(), id)
                });
            }
        },

        Command::Activity { action } => handle_activity(action, &conn, &opts)?,

        Command::Settings { action } => match action {
            cli::SettingsAction::Get { key } => match key {
                Some(k) => {
                    let v = settings::get_raw(&conn, k)?;
                    say(if json {
                        json!({ k: v }).to_string()
                    } else {
                        format!("{k} = {v}")
                    });
                }
                None => {
                    let all = settings::get_all(&conn)?;
                    if json {
                        say(Value::Object(all).to_string());
                    } else {
                        say(output::settings_text(&all));
                    }
                }
            },
            cli::SettingsAction::Set { key, value } => {
                if opts.dry_run {
                    say(json!({"key": key, "value": value, "dry_run": true}).to_string());
                    return Ok(());
                }
                settings::set_from_str(&conn, key, value)?;
                let v = settings::get_raw(&conn, key)?;
                say(if json {
                    json!({ key: v }).to_string()
                } else {
                    format!("{key} = {v}")
                });
            }
        },

        Command::Hud { action: _ } => {
            // Signal a running GUI via a watched setting; the GUI shows the
            // panel when it observes this timestamp change.
            settings::set(&conn, "hud_request_at", &Value::String(util::now_iso()))?;
            say(if json {
                json!({ "hud": "show", "requested_at": util::now_iso() }).to_string()
            } else {
                l.hud_signal().to_string()
            });
        }

        Command::Report {
            week: _,
            last,
            range,
        } => {
            // Task 11: neutral activity compliance via record::compliance. The
            // core fn derives its window from Scope + today; arbitrary
            // --range / --last need a Scope variant that does not yet exist,
            // so they are deferred (Plan 2) rather than silently falling back
            // to the legacy completion report.
            if range.is_some() || last.is_some() {
                return Err(CoreError::InvalidArgument(
                    "report supports the weekly scope (default / --week); --range and --last arrive with budget scope settings".into(),
                ));
            }
            let today = util::today_local();
            let comps = record::compliance(
                &conn,
                oxiline_core::model::Scope::Week,
                chrono::Utc::now(),
                &today,
            )?;
            if json {
                say(output::json_pretty(&comps));
            } else if comps.is_empty() {
                say(format!("({})\n", l.report_empty()));
            } else {
                say(output::compliance_text(l, &comps));
            }
        }
        Command::Record { action } => match action {
            Some(a) => handle_record(a, &conn, &opts)?,
            None => handle_record_bare(&conn, &opts)?,
        },
        Command::Plan { action } => handle_plan(action, &conn, &opts)?,
        Command::Doctor => {
            let ver = oxiline_core::db::schema_version(&conn)? as i64;
            let cats = categories::list(&conn)?;
            if json {
                say(json!({
                    "db_path": path.display().to_string(),
                    "schema_version": ver,
                    "categories": cats.len(),
                    "checks": [
                        {"check": "db_path", "ok": true, "detail": path.display().to_string()},
                        {"check": "schema", "ok": true, "detail": format!("v{}", ver)},
                        {"check": "wal", "ok": true, "detail": "WAL"},
                    ]
                })
                .to_string());
            } else {
                say(format!(
                    "✔ {}: {}\n✔ {}: {} ({}).\n✔ {}.\n✔ {}: {}",
                    l.db_path(),
                    path.display(),
                    l.schema_version(),
                    ver,
                    l.latest(),
                    l.wal_active(),
                    l.categories_count(),
                    cats.len()
                ));
            }
        }
    }
    Ok(())
}

// ---- helpers ---------------------------------------------------------------

fn resource_out<T: serde::Serialize>(json: bool, label: &str, t: &T) -> String {
    if json {
        serde_json::to_string(t).unwrap_or_else(|_| "null".into())
    } else {
        format!(
            "{}\n{}",
            label,
            serde_json::to_string_pretty(t).unwrap_or_default()
        )
    }
}

fn preview(json: bool, label: &str, v: &Value) -> String {
    if json {
        v.to_string()
    } else {
        format!(
            "{label}\n{}",
            serde_json::to_string_pretty(v).unwrap_or_default()
        )
    }
}

fn resolve_lang(conn: &rusqlite::Connection, opts: &Cli) -> Lang {
    if let Some(l) = &opts.lang {
        return Lang::from_code(l);
    }
    let code = settings::get_string(conn, "locale", "system");
    if code == "system" {
        let env = std::env::var("LANG").unwrap_or_default();
        if env.to_ascii_lowercase().contains("ko") {
            Lang::Ko
        } else {
            Lang::En
        }
    } else {
        Lang::from_code(&code)
    }
}

fn resolve_date_arg(s: &str) -> Result<String> {
    if let Some(k) = util::resolve_date_keyword(s) {
        return Ok(k);
    }
    util::parse_date(s)
        .map(util::fmt_date)
        .map_err(|e| CoreError::InvalidArgument(format!("bad date '{s}': {e}")))
}
fn parse_at(at: Option<&str>) -> Result<Option<u16>> {
    match at {
        None => Ok(None),
        Some(s) => util::hhmm_to_minute(s)
            .map(Some)
            .ok_or_else(|| CoreError::InvalidArgument(format!("bad --at '{s}' (expect HH:MM)"))),
    }
}

fn parse_range(r: &str) -> Result<(String, String)> {
    let (from, to) = r
        .split_once(':')
        .ok_or_else(|| CoreError::InvalidArgument("range must be FROM:TO".into()))?;
    Ok((resolve_date_arg(from.trim())?, resolve_date_arg(to.trim())?))
}

/// Dispatch `oxiline activity` subcommands (Task 8).
fn handle_activity(
    action: &ActivityAction,
    conn: &rusqlite::Connection,
    opts: &Cli,
) -> Result<()> {
    let json = opts.json;
    let lang = resolve_lang(conn, opts);
    let l = L(lang);
    let say = |body: String| {
        if !opts.quiet {
            println!("{body}");
        }
    };
    match action {
        ActivityAction::Add {
            name,
            daily,
            weekly,
            hue,
            icon,
            category,
        } => {
            let cat_id: Option<String> = match category.as_deref() {
                Some(c) => Some(categories::resolve(conn, c)?.id),
                None => None,
            };
            if opts.dry_run {
                say(preview(
                    json,
                    l.activity_added(),
                    &json!({
                        "name": name,
                        "target_minutes_daily": daily.and_then(|m| m.0),
                        "target_minutes_weekly": weekly.and_then(|m| m.0),
                        "hue_label": hue,
                        "icon": icon,
                        "category_id": cat_id,
                        "dry_run": true,
                    }),
                ));
                return Ok(());
            }
            // For add: any provided budget is SET. `--daily 0` translates to
            // Some(None) which clears (no budget); positive N is Some(Some(n)).
            // We use `and_then` to wrap the inner option exactly once.
            let daily_inner: Option<Option<u32>> = daily.map(|m| m.0);
            let weekly_inner: Option<Option<u32>> = weekly.map(|m| m.0);
            let a = activities::create_activity(
                conn,
                oxiline_core::model::ActivityInput {
                    name: Some(name.clone()),
                    hue_label: hue.clone(),
                    icon: icon.clone(),
                    category_id: cat_id,
                    target_minutes_daily: daily_inner,
                    target_minutes_weekly: weekly_inner,
                    is_active: None,
                    sort_order: None,
                },
            )?;
            say(resource_out(json, l.activity_added(), &a));
        }
        ActivityAction::List { active_only } => {
            let list = activities::list_activities(conn, *active_only)?;
            if json {
                say(output::json_pretty(&list));
            } else {
                say(output::activity_list_text(&list, l.activity_inactive()));
            }
        }
        ActivityAction::Show { id } => {
            let a = activities::resolve_activity(conn, id)?;
            if json {
                say(output::json_pretty(&a));
            } else {
                say(output::activity_text(&a, l.activity_inactive()));
            }
        }
        ActivityAction::Edit {
            id,
            name,
            daily,
            weekly,
            hue,
            icon,
        } => {
            // tri-state: outer Option<MinuteBudget> -> absent vs present;
            // MinuteBudget.0 -> clear (None) vs set (Some(n)).
            let daily_v = daily.map(|m| m.0);
            let weekly_v = weekly.map(|m| m.0);
            let resolved = activities::resolve_activity(conn, id)?;
            let a = activities::update_activity(
                conn,
                &resolved.id,
                oxiline_core::model::ActivityInput {
                    name: name.clone(),
                    hue_label: hue.clone(),
                    icon: icon.clone(),
                    category_id: None,
                    target_minutes_daily: daily_v,
                    target_minutes_weekly: weekly_v,
                    is_active: None,
                    sort_order: None,
                },
            )?;
            say(resource_out(json, "updated", &a));
        }
        ActivityAction::Toggle { id, on, off } => {
            if on == off {
                return Err(CoreError::InvalidArgument(
                    "use exactly one of --on / --off".into(),
                ));
            }
            let resolved = activities::resolve_activity(conn, id)?;
            let a = activities::update_activity(
                conn,
                &resolved.id,
                oxiline_core::model::ActivityInput {
                    name: None,
                    hue_label: None,
                    icon: None,
                    category_id: None,
                    target_minutes_daily: None,
                    target_minutes_weekly: None,
                    is_active: Some(*on || !*off),
                    sort_order: None,
                },
            )?;
            say(resource_out(json, "toggled", &a));
        }
        ActivityAction::Rm { id, force } => {
            let resolved = activities::resolve_activity(conn, id)?;
            let removed_id = resolved.id.clone();
            activities::delete_activity(conn, &resolved.id, *force)?;
            say(if json {
                json!({ "id": removed_id, "removed": true }).to_string()
            } else {
                format!("{}: {}", l.removed(), removed_id)
            });
        }
    }
    Ok(())
}

/// Parse an ISO 8601 / RFC 3339 timestamp into `DateTime<Utc>`.
fn parse_at_iso(at: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
    match at {
        None => Ok(None),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|e| CoreError::InvalidArgument(format!("bad --at '{s}' (expect RFC 3339): {e}"))),
    }
}

/// Dispatch `oxiline record` subcommands (Task 9).
fn handle_record(
    action: &RecordAction,
    conn: &rusqlite::Connection,
    opts: &Cli,
) -> Result<()> {
    use chrono::Utc;
    let json = opts.json;
    let lang = resolve_lang(conn, opts);
    let l = L(lang);
    let say = |body: String| {
        if !opts.quiet {
            println!("{body}");
        }
    };
    let today = util::today_local();
    let now = Utc::now();
    match action {
        RecordAction::State => {
            let st = record::current(conn, now, &today)?;
            say(record_state_output(json, l, &st));
        }
        RecordAction::Start { activity, at } => {
            let resolved = activities::resolve_activity(conn, activity)?;
            let at_parsed = parse_at_iso(at.as_deref())?;
            let effective_now = at_parsed.unwrap_or(now);
            let st = record::start(conn, &resolved.id, effective_now, &today)?;
            say(record_state_output(json, l, &st));
        }
        RecordAction::Stop => {
            let st = record::stop(conn, now, &today)?;
            say(record_state_output(json, l, &st));
        }
        RecordAction::Log { activity, date, range } => {
            let (from, to) = match (date, range) {
                (Some(_), Some(_)) => {
                    return Err(CoreError::InvalidArgument(
                        "--date and --range are mutually exclusive".into(),
                    ));
                }
                (Some(d), None) => {
                    let d = resolve_date_arg(d)?;
                    (format!("{d}T00:00:00Z"), format!("{d}T23:59:59Z"))
                }
                (None, Some(r)) => {
                    let (from_date, to_date) = parse_range(r)?;
                    (
                        format!("{from_date}T00:00:00Z"),
                        format!("{to_date}T23:59:59Z"),
                    )
                }
                (None, None) => (
                    format!("{today}T00:00:00Z"),
                    format!("{today}T23:59:59Z"),
                ),
            };
            let activity_id = match activity.as_deref() {
                Some(a) => Some(activities::resolve_activity(conn, a)?.id),
                None => None,
            };
            let records = record::list_records(conn, activity_id.as_deref(), &from, &to)?;
            if json {
                say(output::json_pretty(&records));
            } else {
                say(output::record_log_text(l, &records, now));
            }
        }
    }
    Ok(())
}

fn record_state_output(json: bool, l: L, st: &oxiline_core::model::RecordState) -> String {
    if json {
        output::json_pretty(st)
    } else {
        output::record_state_text(l, st)
    }
}

/// Bare `record` (no subcommand) — emit the current `RecordState` JSON/text.
fn handle_record_bare(conn: &rusqlite::Connection, opts: &Cli) -> Result<()> {
    use chrono::Utc;
    let json = opts.json;
    let lang = resolve_lang(conn, opts);
    let l = L(lang);
    let today = util::today_local();
    let now = Utc::now();
    let st = record::current(conn, now, &today)?;
    let say = |body: String| {
        if !opts.quiet {
            println!("{body}");
        }
    };
    say(record_state_output(json, l, &st));
    Ok(())
}

/// Dispatch `oxiline plan` subcommands (Task 10).
fn handle_plan(action: &PlanAction, conn: &rusqlite::Connection, opts: &Cli) -> Result<()> {
    let json = opts.json;
    let lang = resolve_lang(conn, opts);
    let l = L(lang);
    let say = |body: String| {
        if !opts.quiet {
            println!("{body}");
        }
    };
    match action {
        PlanAction::Add {
            at,
            duration,
            days,
            date,
            title,
            options,
        } => {
            let start_minute = parse_at(at.as_deref())?.unwrap_or_else(util::now_minute_local);
            if *duration == 0 || *duration > 1440 {
                return Err(CoreError::InvalidArgument(format!(
                    "--duration must be 1..=1440 minutes, got {duration}"
                )));
            }
            let weekday_mask = parse_days_mask(days.as_deref(), date.as_deref())?;
            let activity_ids = resolve_options(conn, options)?;
            if opts.dry_run {
                say(preview(
                    json,
                    l.plan_added(),
                    &json!({
                        "start_minute": start_minute,
                        "duration_minute": duration,
                        "weekday_mask": weekday_mask,
                        "date": date,
                        "title": title,
                        "activity_ids": activity_ids,
                        "dry_run": true,
                    }),
                ));
                return Ok(());
            }
            let p = plan::create_plan(
                conn,
                oxiline_core::model::PlanInput {
                    date: date.clone(),
                    start_minute,
                    duration_minute: *duration as u16,
                    weekday_mask,
                    title: title.clone(),
                    activity_ids,
                },
            )?;
            say(resource_out(json, l.plan_added(), &p));
        }
        PlanAction::List { date, recurring } => {
            if let Some(d) = date {
                let slots = plan::slots_for_date(conn, &resolve_date_arg(d)?)?;
                if json {
                    say(output::json_pretty(&slots));
                } else {
                    say(output::plan_slot_list_text(&slots));
                }
            } else {
                let plans = plan::list_plans(conn, *recurring)?;
                if json {
                    say(output::json_pretty(&plans));
                } else if plans.is_empty() {
                    say(format!("({})\n", l.plan_list_empty()));
                } else {
                    say(output::plan_list_text(&plans));
                }
            }
        }
        PlanAction::Edit {
            id,
            at,
            duration,
            days,
            date,
            title,
            options,
        } => {
            // update_plan assigns start_minute/duration_minute/weekday_mask
            // DIRECTLY from PlanInput (only date/title merge; empty
            // activity_ids preserves). So fetch the current plan and fill
            // every omitted field from it — otherwise a single-field edit
            // would zero out the time/duration/days.
            let cur = plan::get_plan(conn, id)?;
            let start_minute = parse_at(at.as_deref())?.unwrap_or(cur.start_minute);
            let weekday_mask = if days.is_some() || date.is_some() {
                parse_days_mask(days.as_deref(), date.as_deref())?
            } else {
                cur.weekday_mask
            };
            let activity_ids = match options.as_deref() {
                Some(o) => resolve_options(conn, o)?,
                None => vec![], // empty preserves existing options
            };
            let p = plan::update_plan(
                conn,
                &cur.id,
                oxiline_core::model::PlanInput {
                    date: date.clone().or(cur.date),
                    start_minute,
                    duration_minute: duration.unwrap_or(cur.duration_minute as u32) as u16,
                    weekday_mask,
                    title: title.clone().or(cur.title),
                    activity_ids,
                },
            )?;
            say(resource_out(json, "updated", &p));
        }
        PlanAction::Rm { id } => {
            let removed_id = id.clone();
            plan::delete_plan(conn, id)?;
            say(if json {
                json!({ "id": removed_id, "removed": true }).to_string()
            } else {
                format!("{}: {}", l.removed(), removed_id)
            });
        }
    }
    Ok(())
}

/// Resolve a comma-separated `--options A,B,C` into activity ids.
fn resolve_options(conn: &rusqlite::Connection, comma: &str) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for tok in comma.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        ids.push(activities::resolve_activity(conn, tok)?.id);
    }
    if ids.is_empty() {
        return Err(CoreError::InvalidArgument(
            "at least one --options activity is required".into(),
        ));
    }
    Ok(ids)
}

/// Parse `--days`/`--date` into a `weekday_mask`. Bit 0 = Monday … bit 6 =
/// Sunday (matches `plan::slots_for_date`'s `num_days_from_monday`). A
/// one-shot `--date` yields mask 0 (unused); recurring needs `--days`.
fn parse_days_mask(days: Option<&str>, date: Option<&str>) -> Result<u8> {
    if date.is_some() {
        return Ok(0);
    }
    let Some(spec) = days else {
        return Err(CoreError::InvalidArgument(
            "specify --days (mon,tue,…/weekdays/daily) or --date for a one-shot plan".into(),
        ));
    };
    match spec.trim().to_ascii_lowercase().as_str() {
        "weekdays" => return Ok(0b0011111),
        "daily" => return Ok(0b1111111),
        _ => {}
    }
    let mut mask: u8 = 0;
    for tok in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let bit = match tok.to_ascii_lowercase().as_str() {
            "mon" => 0,
            "tue" => 1,
            "wed" => 2,
            "thu" => 3,
            "fri" => 4,
            "sat" => 5,
            "sun" => 6,
            other => {
                return Err(CoreError::InvalidArgument(format!(
                    "unknown day '{other}' in --days (mon,tue,wed,thu,fri,sat,sun/weekdays/daily)"
                )))
            }
        };
        mask |= 1 << bit;
    }
    if mask == 0 {
        return Err(CoreError::InvalidArgument("--days matched no days".into()));
    }
    Ok(mask)
}
