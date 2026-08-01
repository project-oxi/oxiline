//! Plan CRUD (`2026-08-01-record-layer-design.md` §5) + `slots_for_date` view-model.
//!
//! A plan is a planned time slot that holds OR alternatives (activities). The
//! schema constraint enforces the one-shot / recurring split:
//! `(weekday_mask = 0 AND date IS NOT NULL) OR weekday_mask != 0`.
//! One-shot plans match exactly one `date`; recurring plans match every date
//! whose weekday bit is set in `weekday_mask`.
//!
//! `slots_for_date` materializes the view-model: each slot carries the OR
//! alternatives as a `Vec<Activity>` plus an `is_resolved` / `resolved_by`
//! pair. Resolution wiring is deferred to Task 7 (`record::resolve_plan_for`).

use crate::activities::row_from as activity_row_from;
use crate::error::{CoreError, Result};
use crate::model::{Activity, Plan, PlanInput, PlanOption, PlanSlot};
use crate::util;
use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, params};

/// Map a `plans` row into a `Plan` (8 columns). `start_minute` /
/// `duration_minute` are `u16` and `weekday_mask` is `u8` in the model, but
/// SQLite stores them as `INTEGER`; cast via `i64` for portability.
pub fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Plan> {
    Ok(Plan {
        id: row.get("id")?,
        date: row.get("date")?,
        start_minute: row.get::<_, i64>("start_minute")? as u16,
        duration_minute: row.get::<_, i64>("duration_minute")? as u16,
        weekday_mask: row.get::<_, i64>("weekday_mask")? as u8,
        title: row.get("title")?,
        sort_order: row.get("sort_order")?,
    })
}

fn row_from_option(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlanOption> {
    Ok(PlanOption {
        id: row.get("id")?,
        plan_id: row.get("plan_id")?,
        activity_id: row.get("activity_id")?,
        sort_order: row.get("sort_order")?,
    })
}

/// Validate the one-shot / recurring split. Mirrors the schema CHECK; rejected
/// here so callers get a typed `InvalidArgument` instead of a raw SQLite error.
fn validate_plan_shape(date: &Option<String>, weekday_mask: u8) -> Result<()> {
    let one_shot = weekday_mask == 0;
    if one_shot && date.is_none() {
        return Err(CoreError::InvalidArgument(
            "plan requires a date when weekday_mask is 0 (one-shot)".into(),
        ));
    }
    if !one_shot && weekday_mask == 0 {
        // unreachable under u8, but keep the invariant readable.
        return Err(CoreError::InvalidArgument(
            "weekday_mask must be non-zero for recurring plans".into(),
        ));
    }
    Ok(())
}
/// INSERT a plan + each `plan_options` row in one WAL transaction (via
/// `unchecked_transaction`, which accepts `&Connection` rather than
/// `&mut Connection`). `activity_ids` are assigned `sort_order = index` so
/// the OR alternatives keep insertion order. The plan's own `sort_order`
/// defaults to `MAX + 1` (the spec's tail-appended semantics; `PlanInput`
/// doesn't expose it yet).
pub fn create_plan(conn: &Connection, input: PlanInput) -> Result<Plan> {
    validate_plan_shape(&input.date, input.weekday_mask)?;
    if input.activity_ids.is_empty() {
        return Err(CoreError::InvalidArgument(
            "plan requires at least one activity alternative".into(),
        ));
    }

    let id = util::new_id();
    let now = util::now_iso();
    let next_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM plans",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO plans
         (id, date, start_minute, duration_minute, weekday_mask, title,
          sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            input.date,
            input.start_minute as i64,
            input.duration_minute as i64,
            input.weekday_mask as i64,
            input.title,
            next_order,
            now,
        ],
    )?;
    for (idx, activity_id) in input.activity_ids.iter().enumerate() {
        tx.execute(
            "INSERT INTO plan_options (id, plan_id, activity_id, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![util::new_id(), id, activity_id, idx as i32],
        )?;
    }
    tx.commit()?;
    get_plan(conn, &id)
}

/// Raw plans. `recurring_only` filters to `weekday_mask != 0`. One-shots are
/// sorted by `date`, recurrings by `start_minute`; we keep it simple and
/// fall back to `sort_order` then `start_minute` either way.
pub fn list_plans(conn: &Connection, recurring_only: bool) -> Result<Vec<Plan>> {
    let sql = if recurring_only {
        "SELECT * FROM plans WHERE weekday_mask != 0 ORDER BY sort_order, start_minute"
    } else {
        // One-shots by date NULLS LAST, then by start_minute — SQLite
        // expresses this with a CASE so NULL dates trail.
        "SELECT * FROM plans
         ORDER BY sort_order,
                  CASE WHEN date IS NULL THEN 1 ELSE 0 END,
                  date,
                  start_minute"
    };
    let mut stmt = conn.prepare(sql)?;
    let mut out = Vec::new();
    for r in stmt.query_map([], row_from)? {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_plan(conn: &Connection, id: &str) -> Result<Plan> {
    conn.query_row("SELECT * FROM plans WHERE id = ?", params![id], row_from)
        .map_err(CoreError::from)
}

/// Partial update mirroring `activities::update_activity`: `None` leaves the
/// column unchanged, `Some(v)` sets it. `activity_ids` replaces the full set
/// of OR alternatives. Wrapped in a transaction so a mid-sequence FK or
/// constraint failure rolls the option set back together with the plan row.
pub fn update_plan(conn: &Connection, id: &str, input: PlanInput) -> Result<Plan> {
    let existing = get_plan(conn, id)?;

    let new_date = input.date.or(existing.date);
    let new_start = input.start_minute;
    let new_duration = input.duration_minute;
    let new_mask = input.weekday_mask;
    let new_title = input.title.or(existing.title);

    validate_plan_shape(&new_date, new_mask)?;

    let now = util::now_iso();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE plans SET date = ?1, start_minute = ?2, duration_minute = ?3,
         weekday_mask = ?4, title = ?5, updated_at = ?6 WHERE id = ?7",
        params![
            new_date,
            new_start as i64,
            new_duration as i64,
            new_mask as i64,
            new_title,
            now,
            id,
        ],
    )?;
    if !input.activity_ids.is_empty() {
        tx.execute("DELETE FROM plan_options WHERE plan_id = ?1", params![id])?;
        for (idx, activity_id) in input.activity_ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO plan_options (id, plan_id, activity_id, sort_order)
                 VALUES (?1, ?2, ?3, ?4)",
                params![util::new_id(), id, activity_id, idx as i32],
            )?;
        }
    }
    tx.commit()?;
    get_plan(conn, id)
}
/// DELETE a plan. The schema's `ON DELETE CASCADE` removes the plan_options
/// rows; missing plan id surfaces as `NotFound`.
pub fn delete_plan(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM plans WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("plan '{id}'")));
    }
    Ok(())
}

/// Append an OR alternative to a plan with `sort_order = MAX + 1`. If the
/// activity is already an option, this is a no-op and returns the existing row.
pub fn add_option(conn: &Connection, plan_id: &str, activity_id: &str) -> Result<PlanOption> {
    // Surface NotFound for unknown plans before computing sort_order.
    let _ = get_plan(conn, plan_id)?;
    if let Some(existing) = find_option(conn, plan_id, activity_id)? {
        return Ok(existing);
    }
    let id = util::new_id();
    let next_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM plan_options WHERE plan_id = ?1",
            params![plan_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO plan_options (id, plan_id, activity_id, sort_order)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, plan_id, activity_id, next_order],
    )?;
    conn.query_row(
        "SELECT * FROM plan_options WHERE id = ?",
        params![id],
        row_from_option,
    )
    .map_err(CoreError::from)
}

/// Remove an OR alternative. Missing plan or missing option surfaces as
/// `NotFound` so callers can distinguish "never linked" from "not a plan".
pub fn remove_option(conn: &Connection, plan_id: &str, activity_id: &str) -> Result<()> {
    let _ = get_plan(conn, plan_id)?;
    let n = conn.execute(
        "DELETE FROM plan_options WHERE plan_id = ?1 AND activity_id = ?2",
        params![plan_id, activity_id],
    )?;
    if n == 0 {
        return Err(CoreError::NotFound(format!(
            "plan_options({plan_id}, {activity_id})"
        )));
    }
    Ok(())
}

fn find_option(conn: &Connection, plan_id: &str, activity_id: &str) -> Result<Option<PlanOption>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM plan_options WHERE plan_id = ?1 AND activity_id = ?2",
    )?;
    let mut rows = stmt.query(params![plan_id, activity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_from_option(row)?)),
        None => Ok(None),
    }
}

/// Parse a `YYYY-MM-DD` string into a `NaiveDate`. Invalid input becomes
/// `InvalidArgument` so callers don't have to unwrap a `chrono` error.
fn parse_date_arg(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| CoreError::InvalidArgument(format!("invalid date '{date}': {e}")))
}

/// Materialize the plans that fire on `date` (`YYYY-MM-DD`).
///
/// Selection rules:
/// - one-shot plans (`weekday_mask = 0`) match iff their `date = ?`.
/// - recurring plans (`weekday_mask != 0`) match iff the weekday bit is set
///   for `date` (`bit = num_days_from_monday()`, Mon = 0 … Sun = 6).
///
/// For each matched plan we load its `plan_options` joined with `activities`
/// and assemble a `PlanSlot`. `is_resolved` / `resolved_by` are left `false`/
/// `None` here — Task 7's `record::resolve_plan_for` will populate them.
pub fn slots_for_date(conn: &Connection, date: &str) -> Result<Vec<PlanSlot>> {
    let parsed = parse_date_arg(date)?;
    let weekday_bit = parsed.weekday().num_days_from_monday();
    let mask: u8 = 1u8 << weekday_bit;

    let mut stmt = conn.prepare(
        "SELECT * FROM plans
         WHERE (weekday_mask = 0 AND date = ?1)
            OR (weekday_mask != 0 AND (weekday_mask & ?2) != 0)",
    )?;
    let rows = stmt.query_map(params![date, mask as i64], row_from)?;

    let mut slots = Vec::new();
    for r in rows {
        let plan = r?;
        let options = options_for(conn, &plan.id)?;
        slots.push(PlanSlot {
            plan_id: plan.id,
            date: date.to_string(),
            start_minute: plan.start_minute,
            duration_minute: plan.duration_minute,
            options,
            // is_resolved wired in Task 7
            is_resolved: false,
            resolved_by: None,
        });
    }
    // Stable display order: earliest slot first, ties on duration then plan_id.
    slots.sort_by(|a, b| {
        a.start_minute
            .cmp(&b.start_minute)
            .then(a.duration_minute.cmp(&b.duration_minute))
            .then(a.plan_id.cmp(&b.plan_id))
    });
    Ok(slots)
}

/// Load the OR alternatives for `plan_id` as a `Vec<Activity>`, sorted by
/// `plan_options.sort_order`.
///
/// Important: project *only* activity columns (`SELECT a.*`). The naive
/// `SELECT * FROM plan_options JOIN activities …` would expose both
/// `plan_options.id` / `plan_options.sort_order` *and* `activities.id` /
/// `activities.sort_order` to `activities::row_from`, which reads by column
/// NAME — rusqlite picks the first matching name in the result set, so
/// without explicit projection the row mapping would silently pull
/// `plan_options`'s `id` / `sort_order` and return activities with the wrong
/// ids. Restricting to `a.*` keeps the column names unambiguous.
fn options_for(conn: &Connection, plan_id: &str) -> Result<Vec<Activity>> {
    let mut stmt = conn.prepare(
        "SELECT a.* FROM activities a
         INNER JOIN plan_options o ON o.activity_id = a.id
         WHERE o.plan_id = ?1
         ORDER BY o.sort_order, a.name",
    )?;
    let rows = stmt.query_map(params![plan_id], activity_row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}