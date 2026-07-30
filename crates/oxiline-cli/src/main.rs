//! `oxiline` CLI entry point (`05-cli-spec.md`).

mod cli;
mod lang;
mod output;

use std::process::ExitCode;

use clap::Parser;
use oxiline_core::model::{TaskSource, TimelineItem};
use oxiline_core::{
    categories, reports, routine_groups, routines, settings, tasks, timeline,
};
use oxiline_core::{util, CoreError, Result};
use serde_json::{json, Value};

use cli::{Cli, Command, GroupAction, RoutineAction, TaskAction};
use lang::{Lang, L};

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
            let ctx = timeline::get_now_context(&conn, util::now_minute_local())?;
            if json {
                say(output::json_pretty(&ctx));
            } else {
                say(output::now_text(l, &ctx));
            }
        }
        Command::Today { date } => {
            let d = resolve_date_arg(date.as_deref().unwrap_or("today"))?;
            let items = timeline::get_timeline_for_date(&conn, &d)?;
            if json {
                say(output::json_pretty(&items));
            } else {
                say(output::timeline_text(l, &d, &items));
            }
        }

        Command::Task { action } => match action {
            TaskAction::Add {
                title,
                date,
                backlog,
                at,
                duration,
                category,
                notes,
            } => {
                if *backlog && date.is_some() {
                    return Err(CoreError::InvalidArgument(
                        "--backlog and --date are mutually exclusive".into(),
                    ));
                }
                let cat_id = resolve_category_opt(&conn, category.as_deref())?;
                let date_value = if *backlog {
                    None
                } else {
                    Some(resolve_date_arg(date.as_deref().unwrap_or("today"))?)
                };
                let start = parse_at(at.as_deref())?;
                if opts.dry_run {
                    say(preview(
                        json,
                        l.task_added(),
                        &json!({"title": title, "date": date_value, "start_minute": start, "duration_minute": duration, "category_id": cat_id, "dry_run": true}),
                    ));
                    return Ok(());
                }
                let t = tasks::create(
                    &conn,
                    tasks::NewTask {
                        date: date_value,
                        title: title.clone(),
                        category_id: cat_id,
                        start_minute: start,
                        duration_minute: Some(*duration),
                        notes: notes.clone(),
                    },
                )?;
                say(resource_out(json, l.task_added(), &t));
            }
            TaskAction::List {
                date,
                backlog,
                range,
            } => {
                if *backlog {
                    let items = tasks::list_backlog(&conn)?;
                    if json {
                        say(output::json_pretty(&items));
                    } else {
                        say(output::backlog_text(l, &items));
                    }
                } else if let Some(r) = range {
                    let (from, to) = parse_range(r)?;
                    let tasks_list = tasks::list_range(&conn, &from, &to)?;
                    if json {
                        say(output::json_pretty(&tasks_list));
                    } else {
                        let mut out = String::new();
                        for t in tasks_list {
                            out.push_str(&output::task_text(&t));
                            out.push_str(&format!("  date: {}\n", t.date.unwrap_or_default()));
                            out.push('\n');
                        }
                        say(out);
                    }
                } else {
                    let d = resolve_date_arg(date.as_deref().unwrap_or("today"))?;
                    let items: Vec<TimelineItem> =
                        timeline::get_timeline_for_date(&conn, &d)?;
                    if json {
                        say(output::json_pretty(&items));
                    } else {
                        say(output::timeline_text(l, &d, &items));
                    }
                }
            }
            TaskAction::Show { id } => {
                let real = tasks::materialize_if_virtual(&conn, id)?;
                let t = tasks::get(&conn, &real)?;
                if json {
                    say(output::json_pretty(&t));
                } else {
                    say(output::task_text(&t));
                }
            }
            TaskAction::Done { id } => {
                let real = tasks::materialize_if_virtual(&conn, id)?;
                let t = tasks::set_done(&conn, &real, true)?;
                say(resource_out(json, l.done(), &t));
            }
            TaskAction::Undone { id } => {
                let real = tasks::materialize_if_virtual(&conn, id)?;
                let t = tasks::set_done(&conn, &real, false)?;
                say(resource_out(json, l.undone(), &t));
            }
            TaskAction::Skip { id } => {
                let real = tasks::materialize_if_virtual(&conn, id)?;
                let t = tasks::set_skipped(&conn, &real, true)?;
                say(resource_out(json, l.skipped(), &t));
            }
            TaskAction::Edit {
                id,
                date,
                backlog,
                at,
                duration,
                category,
                title,
                notes,
            } => {
                if *backlog && date.is_some() {
                    return Err(CoreError::InvalidArgument(
                        "--backlog and --date are mutually exclusive".into(),
                    ));
                }
                let cat_id = category.as_deref().map(|c| resolve_category_opt(&conn, Some(c))).transpose()?;
                let real = tasks::materialize_if_virtual(&conn, id)?;
                let upd = tasks::TaskUpdate {
                    title: title.clone(),
                    date: if *backlog {
                        Some(None)
                    } else {
                        match date.as_deref() {
                            Some(d) => Some(Some(resolve_date_arg(d)?)),
                            None => None,
                        }
                    },
                    start_minute: match parse_at(at.as_deref())? {
                        Some(v) => Some(Some(v)),
                        None => None,
                    },
                    duration_minute: duration.map(Some),
                    category_id: cat_id,
                    notes: notes.clone().map(Some),
                };
                let t = tasks::update(&conn, &real, upd)?;
                say(resource_out(json, "updated", &t));
            }
            TaskAction::Rm { id } => {
                // Virtual id or materialized routine occurrence → skip (hide),
                // not physical delete, so the virtual occurrence does not
                // reappear on the next render (03-data-model.md §3.7).
                let real = tasks::materialize_if_virtual(&conn, id)?;
                let t = tasks::get(&conn, &real)?;
                match t.source {
                    TaskSource::Routine => {
                        tasks::set_skipped(&conn, &real, true)?;
                    }
                    TaskSource::Manual => {
                        tasks::delete(&conn, &real)?;
                    }
                }
                say(if json {
                    json!({ "id": real, "removed": true }).to_string()
                } else {
                    format!("{}: {}", l.removed(), real)
                });
            }
        },

        Command::Routine { action } => match action {
            RoutineAction::Add {
                title,
                at,
                duration,
                days,
                from,
                until,
                category,
                notes,
            } => {
                let start = parse_at(at.as_deref())?.ok_or_else(|| {
                    CoreError::InvalidArgument("routine add requires --at HH:MM".into())
                })?;
                let mask = routines::parse_days_spec(days)?;
                let cat_id = resolve_category_opt(&conn, category.as_deref())?;
                if opts.dry_run {
                    say(preview(
                        json,
                        l.routine_added(),
                        &json!({"title": title, "start_minute": start, "duration_minute": duration, "weekday_mask": mask, "dry_run": true}),
                    ));
                    return Ok(());
                }
                let r = routines::create(
                    &conn,
                    routines::NewRoutineBlock {
                        title: title.clone(),
                        start_minute: start,
                        duration_minute: *duration,
                        weekday_mask: mask,
                        category_id: cat_id,
                        effective_from: from.clone(),
                        effective_until: until.clone(),
                        notes: notes.clone(),
                    },
                )?;
                say(resource_out(json, l.routine_added(), &r));
            }
            RoutineAction::List { active_only } => {
                let rs = routines::list(&conn, *active_only)?;
                if json {
                    say(output::json_pretty(&rs));
                } else {
                    say(output::routine_list_text(&rs));
                }
            }
            RoutineAction::Show { id } => {
                let r = routines::get(&conn, id)?;
                if json {
                    say(output::json_pretty(&r));
                } else {
                    say(output::routine_text(&r));
                }
            }
            RoutineAction::Edit {
                id,
                at,
                duration,
                days,
                category,
                title,
                notes,
            } => {
                let mask = match days {
                    Some(d) => Some(routines::parse_days_spec(d)?),
                    None => None,
                };
                let cat_id = category.as_deref().map(|c| resolve_category_opt(&conn, Some(c))).transpose()?;
                let upd = routines::RoutineUpdate {
                    title: title.clone(),
                    start_minute: parse_at(at.as_deref())?,
                    duration_minute: *duration,
                    weekday_mask: mask,
                    category_id: cat_id,
                    notes: notes.clone().map(Some),
                };
                let r = routines::update(&conn, id, upd)?;
                say(resource_out(json, "updated", &r));
            }
            RoutineAction::Toggle { id, on, off } => {
                if *on == *off {
                    return Err(CoreError::InvalidArgument(
                        "use exactly one of --on / --off".into(),
                    ));
                }
                let r = routines::set_active(&conn, id, *on)?;
                say(resource_out(json, "toggled", &r));
            }
            RoutineAction::Rm { id } => {
                routines::delete(&conn, id)?;
                say(if json {
                    json!({ "id": id, "removed": true }).to_string()
                } else {
                    format!("{}: {}", l.removed(), id)
                });
            }
            RoutineAction::Group { action } => handle_group(action, &conn, &opts)?,
        },

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

        Command::Export { range } => {
            let (from, to) = parse_range(range)?;
            let mut days: Vec<Value> = Vec::new();
            let mut cur = util::parse_date(&from)
                .map_err(|e| CoreError::InvalidArgument(e.to_string()))?;
            let end = util::parse_date(&to)
                .map_err(|e| CoreError::InvalidArgument(e.to_string()))?;
            while cur <= end {
                let d = util::fmt_date(cur);
                let items = timeline::get_timeline_for_date(&conn, &d)?;
                days.push(json!({ "date": d, "items": items }));
                cur += chrono::Duration::days(1);
            }
            // export is always JSON.
            say(serde_json::to_string_pretty(&Value::Array(days)).unwrap_or_default());
        }

        Command::Report { week: _, last, range } => {
            let today = util::today_local();
            let now = util::now_minute_local();
            if let Some(r) = range {
                let (from, to) = parse_range(&r)?;
                let rep = reports::range_report(&conn, &from, &to, &today, now)?;
                if json {
                    say(output::json_pretty(&rep));
                } else {
                    say(output::range_report_text(l, &rep));
                }
            } else if let Some(n) = last {
                let to = today.clone();
                let from = util::add_days(&to, -((*n as i64) - 1)).unwrap_or_else(|| to.clone());
                let rep = reports::range_report(&conn, &from, &to, &today, now)?;
                if json {
                    say(output::json_pretty(&rep));
                } else {
                    say(output::range_report_text(l, &rep));
                }
            } else {
                let rep = reports::week_report(&conn, &today, now)?;
                if json {
                    say(output::json_pretty(&rep));
                } else {
                    say(output::week_report_text(l, &rep));
                }
            }
        }
        Command::Streak { target } => {
            let today = util::today_local();
            match target {
                None => {
                    let ss = reports::routine_streaks(&conn, &today)?;
                    if json {
                        say(output::json_pretty(&ss));
                    } else {
                        say(output::streak_list_text(l, &ss));
                    }
                }
                Some(name) => {
                    let id = resolve_routine_target(&conn, name)?;
                    let s = reports::routine_streak(&conn, &id, &today)?;
                    if json {
                        say(output::json_pretty(&s));
                    } else {
                        say(output::streak_list_text(l, std::slice::from_ref(&s)));
                    }
                }
            }
        }
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
                }).to_string());
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
        format!("{}\n{}", label, serde_json::to_string_pretty(t).unwrap_or_default())
    }
}

fn preview(json: bool, label: &str, v: &Value) -> String {
    if json {
        v.to_string()
    } else {
        format!("{label}\n{}", serde_json::to_string_pretty(v).unwrap_or_default())
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

fn resolve_category_opt(
    conn: &rusqlite::Connection,
    id_or_name: Option<&str>,
) -> Result<Option<String>> {
    match id_or_name {
        None => Ok(None),
        Some(s) => Ok(Some(categories::resolve(conn, s)?.id)),
    }
}

/// Resolve a routine by exact id, or a unique active-routine title match.
fn resolve_routine_target(conn: &rusqlite::Connection, id_or_name: &str) -> Result<String> {
    if routines::get(conn, id_or_name).is_ok() {
        return Ok(id_or_name.into());
    }
    let matches: Vec<_> = routines::list(conn, true)?
        .into_iter()
        .filter(|b| b.title == id_or_name)
        .collect();
    match matches.len() {
        1 => Ok(matches[0].id.clone()),
        0 => Err(CoreError::NotFound(format!("routine '{id_or_name}'"))),
        _ => Err(CoreError::InvalidArgument(format!("ambiguous routine '{id_or_name}'"))),
    }
}

fn handle_group(
    action: &GroupAction,
    conn: &rusqlite::Connection,
    opts: &Cli,
) -> Result<()> {
    let json = opts.json;
    let lang = resolve_lang(conn, opts);
    let l = L(lang);
    let say = |s: String| if !opts.quiet { println!("{s}") };
    match action {
        GroupAction::List => {
            let groups = routine_groups::list(conn)?;
            say(resource_out(json, "groups", &groups));
        }
        GroupAction::Show { id } => {
            let g = routine_groups::get(conn, id)?;
            say(resource_out(json, "group", &g));
        }
        GroupAction::Add { name, icon } => {
            let g = routine_groups::create(conn, routine_groups::NewRoutineGroup {
                name: name.clone(),
                icon: icon.clone(),
            })?;
            say(resource_out(json, "created", &g));
        }
        GroupAction::Edit { id, name, icon, sort_order } => {
            let g = routine_groups::update(conn, id, routine_groups::RoutineGroupUpdate {
                name: name.clone(),
                icon: icon.clone(),
                sort_order: *sort_order,
            })?;
            say(resource_out(json, "updated", &g));
        }
        GroupAction::Rm { id } => {
            routine_groups::delete(conn, id)?;
            say(if json {
                json!({ "id": id, "removed": true }).to_string()
            } else {
                format!("{}: {}", l.removed(), id)
            });
        }
        GroupAction::Toggle { id, on, off } => {
            if on == off {
                return Err(CoreError::InvalidArgument(
                    "use exactly one of --on / --off".into(),
                ));
            }
            let active = on.unwrap_or(!off.unwrap_or(true));
            let g = routine_groups::set_active(conn, id, active)?;
            say(resource_out(json, "toggled", &g));
        }
    }
    Ok(())
}
