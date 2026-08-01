//! Recording lifecycle: start, stop, inspect, and aggregate the single active session.
//!
//! `compliance(scope, now, today)` (Task 7) is the neutral weekly/daily ratio view-model
//! consumed by the CLI report and the sidebar/inspector in Plan 2. All four states
//! (`Under`/`Met`/`Over`/`Unbudgeted`) share the activity's hue — there is **no**
//! color logic in core; that's a GUI/Plan 2 concern (spec §3.6).
//!
//! `resolve_plan_for` (Task 6) is the §3.1 derived link from a record to its plan;
//! `plan::slots_for_date` (Task 7) consumes it for the timetable view-model.

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use rusqlite::{Connection, params};

use crate::error::{CoreError, Result};
use crate::model::{
    ActiveSession, Activity, Compliance, ComplianceState, Record, RecordState, Scope,
};
use crate::util::{self, new_id, round_duration};

fn timestamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        id: row.get("id")?,
        activity_id: row.get("activity_id")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        note: row.get("note")?,
    })
}

pub fn start(
    conn: &Connection,
    activity_id: &str,
    now: DateTime<Utc>,
    today: &str,
) -> Result<RecordState> {
    let now_text = timestamp(now);
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE records SET ended_at = ?1, updated_at = ?1 WHERE ended_at IS NULL",
        params![now_text],
    )?;
    tx.execute(
        "INSERT INTO records
             (id, activity_id, started_at, ended_at, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, NULL, NULL, ?3, ?3)",
        params![new_id(), activity_id, now_text],
    )?;
    tx.commit()?;
    current(conn, now, today)
}

pub fn stop(conn: &Connection, now: DateTime<Utc>, today: &str) -> Result<RecordState> {
    let now_text = timestamp(now);
    conn.execute(
        "UPDATE records SET ended_at = ?1, updated_at = ?1 WHERE ended_at IS NULL",
        params![now_text],
    )?;
    // `stop` clears the active session but still surfaces today's compliance
    // (the sidebar / inspector needs it on `record stop` so the totals update
    // without a follow-up read). Failures degrade to an empty list rather than
    // masking the stop itself.
    let today_compliance = compliance(conn, Scope::Today, now, today).unwrap_or_default();
    Ok(RecordState {
        active: None,
        today: today_compliance,
        generated_at: now_text,
    })
}

pub fn current(conn: &Connection, now: DateTime<Utc>, today: &str) -> Result<RecordState> {
    let now_text = timestamp(now);
    let mut stmt = conn.prepare(
        "SELECT id, activity_id, started_at, ended_at, note
         FROM records WHERE ended_at IS NULL
         ORDER BY started_at DESC, id DESC",
    )?;
    let records = stmt
        .query_map([], row_from)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);

    let record = records.first().cloned();
    if records.len() > 1 {
        for stale in &records[1..] {
            conn.execute(
                "UPDATE records
                 SET ended_at = CASE WHEN started_at >= ?1
                                     THEN strftime('%Y-%m-%dT%H:%M:%SZ', started_at, '+1 second')
                                     ELSE ?1 END,
                     updated_at = ?1
                 WHERE id = ?2 AND ended_at IS NULL",
                params![now_text, stale.id],
            )?;
        }
    }

    let active = if let Some(record) = record {
        let activity = crate::activities::get_activity(conn, &record.activity_id)?;
        let started_at = DateTime::parse_from_rfc3339(&record.started_at)
            .map_err(|e| CoreError::Internal(format!("invalid record started_at: {e}")))?
            .with_timezone(&Utc);
        let elapsed = (now - started_at).num_seconds().max(0) as u64;
        let increment = crate::settings::get_i64(conn, "record_rounding_minutes", 5).max(0) as u32;
        Some(ActiveSession {
            record,
            activity,
            elapsed_seconds: round_duration(elapsed, increment),
        })
    } else {
        None
    };

    // Task 7: populate `today` via `compliance`. Failures here are recovered
    // (empty list) — `current` is the live-state read and must never error out
    // because some unrelated row in the activities table misbehaved.
    let today_compliance = compliance(conn, Scope::Today, now, today).unwrap_or_default();
    Ok(RecordState {
        active,
        today: today_compliance,
        generated_at: now_text,
    })
}

/// List records whose `started_at` lies within `[from, to]` (lexicographic
/// string compare works because timestamps are ISO 8601 UTC with fixed width).
/// Optionally filtered by `activity_id`. Ordered by `started_at` ascending so
/// callers can stream them as a timeline.
pub fn list_records(
    conn: &Connection,
    activity_id: Option<&str>,
    from: &str,
    to: &str,
) -> Result<Vec<Record>> {
    let mut out = Vec::new();
    if let Some(aid) = activity_id {
        let mut stmt = conn.prepare(
            "SELECT id, activity_id, started_at, ended_at, note
             FROM records
             WHERE activity_id = ?1 AND started_at BETWEEN ?2 AND ?3
             ORDER BY started_at",
        )?;
        for r in stmt.query_map(params![aid, from, to], row_from)? {
            out.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, activity_id, started_at, ended_at, note
             FROM records
             WHERE started_at BETWEEN ?1 AND ?2
             ORDER BY started_at",
        )?;
        for r in stmt.query_map(params![from, to], row_from)? {
            out.push(r?);
        }
    }
    Ok(out)
}

/// Edit a record's `started_at` and/or `ended_at`. `None` leaves the column
/// unchanged. Validates `ended_at > started_at` on the resulting values (the
/// schema's CHECK enforces the same constraint; we surface a typed error here).
pub fn edit_record(
    conn: &Connection,
    id: &str,
    started_at: Option<String>,
    ended_at: Option<String>,
) -> Result<Record> {
    let existing = conn
        .query_row(
            "SELECT id, activity_id, started_at, ended_at, note FROM records WHERE id = ?1",
            params![id],
            row_from,
        )
        .map_err(CoreError::from)?;

    let new_started = started_at.unwrap_or(existing.started_at.clone());
    let new_ended = ended_at.or(existing.ended_at);

    if let Some(ref e) = new_ended
        && e <= &new_started
    {
        return Err(CoreError::InvalidArgument(format!(
            "ended_at '{e}' must be greater than started_at '{new_started}'"
        )));
    }

    let now = util::now_iso();
    conn.execute(
        "UPDATE records SET started_at = ?1, ended_at = ?2, updated_at = ?3 WHERE id = ?4",
        params![new_started, new_ended, now, id],
    )?;
    conn.query_row(
        "SELECT id, activity_id, started_at, ended_at, note FROM records WHERE id = ?1",
        params![id],
        row_from,
    )
    .map_err(CoreError::from)
}

/// Delete a record by id. Missing id is `NotFound` so callers can distinguish
/// "already gone" from a SQL error.
pub fn delete_record(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM records WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("record '{id}'")));
    }
    Ok(())
}

/// Inclusive-exclusive overlap in minutes: `max(0, min(b,d) - max(a,c))`.
fn overlap_minutes(rec_start: u32, rec_end: u32, plan_start: u32, plan_end: u32) -> u32 {
    let lo = rec_start.max(plan_start);
    let hi = rec_end.min(plan_end);
    hi.saturating_sub(lo)
}

/// Resolve the `PlanSlot` that a given record fulfills. Returns the plan with
/// the most overlap between its `[start_minute, start + duration]` window and
/// the record's `[started_at, ended_at|now]` interval. Returns `None` when no
/// plan's option list contains the record's activity or when the windows do
/// not overlap.
///
/// **Wall-clock convention.** Plan windows are stored as minute-of-day
/// integers (0..1439). Records store ISO 8601 UTC instants. The two are made
/// comparable by reading the record's stored UTC instant and using its own
/// HH:MM / weekday as the wall-clock value — never `with_timezone(&Local)`,
/// which would shift by the host's offset and break TZ-portable tests (the
/// `resolve_links_record_to_plan` test pins `09:10Z` against a `09:00..10:30`
/// window).
///
/// Reads from `plans` + `plan_options` only — record resolution is computed
/// (derived), never stored. Wired into `plan::slots_for_date` in Task 7.
pub fn resolve_plan_for(conn: &Connection, rec: &Record) -> Result<Option<crate::model::PlanSlot>> {
    use chrono::Datelike;

    // 1. Load candidate plans (those whose option list includes rec.activity_id).
    let mut stmt = conn.prepare(
        "SELECT p.* FROM plans p
         INNER JOIN plan_options o ON o.plan_id = p.id
         WHERE o.activity_id = ?1",
    )?;
    let plans: Vec<crate::model::Plan> = stmt
        .query_map(params![&rec.activity_id], crate::plan::row_from)?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if plans.is_empty() {
        return Ok(None);
    }

    // Parse the record's UTC instants and use their own HH:MM / weekday as
    // wall-clock values. Avoid `with_timezone(&Local)` — see the doc comment.
    let started_utc = parse_utc(&rec.started_at)?;
    // For an open record (`ended_at IS NULL`) treat the interval as
    // `[started_at, end-of-day]` (minute 1439). This is TZ-portable: 1439 is a
    // wall-clock ceiling, not a UTC instant, and it guarantees the record
    // covers any plan window that fires later today. Without this a record
    // started at 09:10 with `now = 23:00` would compute `overlap = 0` for a
    // 09:00–10:30 plan and silently fail to resolve.
    let rec_end_min: u32 = match &rec.ended_at {
        Some(ts) => {
            let e = parse_utc(ts)?;
            e.hour() * 60 + e.minute()
        }
        None => 1439,
    };

    let rec_start_min = started_utc.hour() * 60 + started_utc.minute();
    let rec_local_date = started_utc.format("%Y-%m-%d").to_string();
    let rec_weekday_bit = 1u8 << started_utc.weekday().num_days_from_monday();
    let mut best: Option<(u32, crate::model::PlanSlot)> = None;

    for plan in &plans {
        let matches_date = if plan.weekday_mask == 0 {
            plan.date.as_deref() == Some(rec_local_date.as_str())
        } else {
            (plan.weekday_mask & rec_weekday_bit) != 0
        };
        if !matches_date {
            continue;
        }

        let plan_start = plan.start_minute as u32;
        let plan_end = plan_start + plan.duration_minute as u32;
        let overlap = overlap_minutes(
            rec_start_min as u32,
            rec_end_min as u32,
            plan_start,
            plan_end,
        );
        if overlap == 0 {
            continue;
        }

        // Load the OR alternatives for this plan (project `a.*` to avoid the
        // `plan_options.id` vs `activities.id` column-name collision noted in
        // `plan::options_for`).
        let mut opt_stmt = conn.prepare(
            "SELECT a.* FROM activities a
             INNER JOIN plan_options o ON o.activity_id = a.id
             WHERE o.plan_id = ?1
             ORDER BY o.sort_order, a.name",
        )?;
        let options: Vec<crate::model::Activity> = opt_stmt
            .query_map(params![&plan.id], crate::activities::row_from)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let slot = crate::model::PlanSlot {
            plan_id: plan.id.clone(),
            date: rec_local_date.clone(),
            start_minute: plan.start_minute,
            duration_minute: plan.duration_minute,
            options,
            is_resolved: true,
            resolved_by: Some(rec.clone()),
        };
        if best.as_ref().is_none_or(|b| overlap > b.0) {
            best = Some((overlap, slot));
        }
    }
    Ok(best.map(|(_, s)| s))
}

/// Parse a `YYYY-MM-DDTHH:MM:SSZ` UTC instant into `DateTime<Utc>`.
fn parse_utc(ts: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CoreError::Internal(format!("invalid timestamp '{ts}': {e}")))
}

// ---- Task 7: compliance (neutral, rounded, today/week) --------------------

/// Compute the `[from, to]` ISO 8601 UTC bounds (lexicographic) used to scan
/// records inside `scope`. For `Scope::Today` we pair `today` with full
/// local-day bounds. For `Scope::Week` we anchor on the user's
/// `week_starts_on` ("mon" / "sun") setting and walk 6 days forward.
///
/// Records store ISO 8601 UTC instants with fixed width, so a string-range
/// compare in `list_records` is sufficient — no datetime conversion needed.
fn scope_bounds(conn: &Connection, scope: &Scope, today: &str) -> (String, String) {
    // Stored records use `YYYY-MM-DDTHH:MM:SSZ` (the `T` separator is part of
    // the ISO 8601 instant format produced by `timestamp(...)`). Bounds must
    // use the same separator so the lexicographic `BETWEEN` in `list_records`
    // matches — `T` (0x54) > ` ` (0x20), so a space-separated bound like
    // `2026-08-03 23:59:59Z` would lex-order BEFORE any T-separated record on
    // the same date and silently exclude them.
    match scope {
        Scope::Today => (format!("{today}T00:00:00Z"), format!("{today}T23:59:59Z")),
        Scope::Week => {
            let week_start = week_start_date(conn, today);
            let end_d = week_start + chrono::Duration::days(6);
            let end_str = end_d.format("%Y-%m-%d").to_string();
            (
                format!("{}T00:00:00Z", week_start.format("%Y-%m-%d")),
                format!("{end_str}T23:59:59Z"),
            )
        }
    }
}

/// `reports::week_start_date`
/// — we duplicate the four lines here rather than enlarging the API surface.
/// If Plan 2 grows the need, lift the helper to a shared module.
fn week_start_date(conn: &Connection, today: &str) -> NaiveDate {
    let week_starts_on = crate::settings::get_string(conn, "week_starts_on", "mon");
    let d = match NaiveDate::parse_from_str(today, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch is valid"),
    };
    let offset = if week_starts_on == "sun" {
        d.weekday().num_days_from_sunday() as i64
    } else {
        d.weekday().num_days_from_monday() as i64
    };
    d - chrono::Duration::days(offset)
}

/// Sum (rounded) record-overlap seconds for `activity` over the `[from, to]`
/// window, returning the total in seconds.
///
/// "Overlap" means the **wall-clock seconds inside `[from, to]`** for each
/// record, capped by the window on both ends, then snap-rounded to
/// `record_rounding_minutes` (0 ⇒ exact seconds, no rounding).
fn recorded_seconds_in_window(
    conn: &Connection,
    activity: &Activity,
    from: &str,
    to: &str,
    now: DateTime<Utc>,
) -> Result<u64> {
    let increment = crate::settings::get_i64(conn, "record_rounding_minutes", 5).max(0) as u32;

    let mut total_secs: u64 = 0;
    let records = list_records(conn, Some(&activity.id), from, to)?;
    for rec in &records {
        // Skip still-open records that haven't started yet — `started_at`
        // is the only boundary that matters here. Open records that started
        // inside the window are capped to `now`.
        let rec_started = rec.started_at.as_str();
        if rec_started > to {
            continue;
        }
        let rec_ended: &str = rec.ended_at.as_deref().unwrap_or_default();

        let span_secs = match compute_span_secs(rec_started, rec_ended, from, to, now) {
            Some(s) => s,
            None => continue,
        };
        total_secs = total_secs.saturating_add(span_secs);
    }
    let _ = increment; // used after rounding below
    let rounded = round_duration(total_secs, increment);
    Ok(rounded)
}

/// Closed-record overlap seconds inside `[from, to]`. Open records (`None`)
/// are capped at `now` (UTC). Returns `None` if the record contributes zero
/// seconds inside the window (e.g. it falls entirely outside).
fn compute_span_secs(
    rec_started: &str,
    rec_ended: &str,
    from: &str,
    to: &str,
    now: DateTime<Utc>,
) -> Option<u64> {
    // Effective boundaries: `max(rec_started, from)`, capped at `to`, with
    // open records anchoring on `now` for the upper bound.
    let lo_str = if rec_started > from {
        rec_started
    } else {
        from
    };
    let upper_bound = if rec_ended.is_empty() {
        timestamp(now)
    } else {
        rec_ended.to_string()
    };
    let hi_str = if upper_bound.as_str() < to {
        upper_bound.as_str()
    } else {
        to
    };
    if hi_str <= lo_str {
        return None;
    }
    let lo = parse_utc(lo_str).ok()?;
    let hi = parse_utc(hi_str).ok()?;
    let dur = (hi - lo).num_seconds().max(0) as u64;
    if dur == 0 { None } else { Some(dur) }
}

/// Per-activity compliance snapshot for the given scope. Each active activity
/// gets exactly one `Compliance` row; inactive activities are skipped (they
/// have no budget and shouldn't pollute the report). The list is sorted by
/// activity name for deterministic output (CLI table layout).
///
/// **Neutrality (spec §3.6).** Every state shares the activity's own hue —
/// `core` carries no color logic; the GUI maps `state` to the activity
/// palette in Plan 2.
pub fn compliance(
    conn: &Connection,
    scope: Scope,
    now: DateTime<Utc>,
    today: &str,
) -> Result<Vec<Compliance>> {
    let (from, to) = scope_bounds(conn, &scope, today);

    let activities: Vec<Activity> = crate::activities::list_activities(conn, true)?;
    let mut out: Vec<Compliance> = Vec::with_capacity(activities.len());

    for activity in &activities {
        let recorded = recorded_seconds_in_window(conn, activity, &from, &to, now)?;

        let target_minutes = match scope {
            Scope::Today => activity.target_minutes_daily,
            Scope::Week => activity.target_minutes_weekly,
        };
        let target_seconds = target_minutes.map(|m| m as u64 * 60);

        let ratio = match (recorded, target_seconds) {
            (rec, Some(tgt)) if tgt > 0 => Some(rec as f64 / tgt as f64),
            _ => None,
        };
        let remaining_seconds: Option<i64> = target_seconds.map(|tgt| tgt as i64 - recorded as i64);
        let state = match (ratio, target_seconds) {
            (None, _) => ComplianceState::Unbudgeted,
            (Some(r), _) if r < 1.0 => ComplianceState::Under,
            (Some(r), _) if r < 1.05 => ComplianceState::Met,
            (Some(_), _) => ComplianceState::Over,
        };

        out.push(Compliance {
            activity: activity.clone(),
            recorded_seconds: recorded,
            target_seconds,
            ratio,
            remaining_seconds,
            state,
        });
    }

    out.sort_by(|a, b| a.activity.name.cmp(&b.activity.name));
    Ok(out)
}
