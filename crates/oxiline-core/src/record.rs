//! Recording lifecycle: start, stop, and inspect the single active session.

use chrono::{DateTime, Timelike, Utc};
use rusqlite::{Connection, params};

use crate::error::{CoreError, Result};
use crate::model::{ActiveSession, Record, RecordState};
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

pub fn stop(conn: &Connection, now: DateTime<Utc>, _today: &str) -> Result<RecordState> {
    let now_text = timestamp(now);
    conn.execute(
        "UPDATE records SET ended_at = ?1, updated_at = ?1 WHERE ended_at IS NULL",
        params![now_text],
    )?;
    Ok(RecordState {
        active: None,
        today: vec![],
        generated_at: now_text,
    })
}

pub fn current(conn: &Connection, now: DateTime<Utc>, _today: &str) -> Result<RecordState> {
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

    Ok(RecordState {
        active,
        today: vec![],
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
        let overlap = overlap_minutes(rec_start_min as u32, rec_end_min as u32, plan_start, plan_end);
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
