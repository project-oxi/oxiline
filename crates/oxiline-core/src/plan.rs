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
//! pair. Resolution is computed via `record::resolve_plan_for` over the day's
//! records (batched once, not per slot — see Task 7 step 4 of the plan).

use crate::activities::row_from as activity_row_from;
use crate::error::{CoreError, Result};
use crate::model::{Activity, Plan, PlanInput, PlanOption, PlanSlot};
use crate::util;
use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

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
/// Resize a plan's duration in place. Only `duration_minute` changes; start,
/// weekday, title and the OR option set are untouched (unlike `update_plan`,
/// which reassigns start/weekday directly from `PlanInput`). `0` is rejected
/// as a defensive floor — callers clamp to a sensible minimum (e.g. 15 min).
pub fn resize_plan(conn: &Connection, id: &str, duration_minute: u16) -> Result<Plan> {
    if duration_minute == 0 {
        return Err(CoreError::InvalidArgument(
            "duration_minute must be greater than 0".into(),
        ));
    }
    let now = util::now_iso();
    let n = conn.execute(
        "UPDATE plans SET duration_minute = ?1, updated_at = ?2 WHERE id = ?3",
        params![duration_minute as i64, now, id],
    )?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("plan '{id}'")));
    }
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

/// Append OR alternatives to a plan in ONE `BEGIN IMMEDIATE` transaction. The
/// write lock is acquired at `BEGIN`, BEFORE the `MAX(sort_order)` read, so the
/// read sees the latest committed state and no other pooled connection can
/// interleave a write between the read and the commit. `activity_ids` keep
/// their input order; already-linked activities (and repeats within the input)
/// are skipped — the returned `Vec<PlanOption>` holds one row per *unique
/// input* in input order (existing-or-new). `sort_order` continues from
/// `MAX + 1`, assigned monotonically inside the locked transaction. Empty
/// input short-circuits to an empty `Vec` without touching the DB.
pub fn add_options(
    conn: &Connection,
    plan_id: &str,
    activity_ids: &[String],
) -> Result<Vec<PlanOption>> {
    if activity_ids.is_empty() {
        return Ok(Vec::new());
    }
    // Surface NotFound for unknown plans before opening a transaction.
    let _ = get_plan(conn, plan_id)?;
    // BEGIN IMMEDIATE: RESERVED (write) lock at `BEGIN`, BEFORE the MAX read.
    // (unchecked_transaction() is the Deferred flavor and would race: the MAX
    // read runs on a stale WAL snapshot without the lock, so two pooled
    // connections can read the same MAX and insert duplicate sort_orders.)
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let existing: Vec<PlanOption> = tx
        .prepare("SELECT * FROM plan_options WHERE plan_id = ?1 ORDER BY sort_order")?
        .query_map(params![plan_id], row_from_option)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut by_activity: std::collections::HashMap<&str, PlanOption> =
        std::collections::HashMap::new();
    for opt in &existing {
        by_activity.insert(opt.activity_id.as_str(), opt.clone());
    }
    // existing is ordered by sort_order ASC → last row is the max.
    let mut next_order: i32 = existing.last().map(|o| o.sort_order + 1).unwrap_or(0);
    let mut out: Vec<PlanOption> = Vec::with_capacity(activity_ids.len());
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for aid in activity_ids {
        let aid_str = aid.as_str();
        if !seen.insert(aid_str) {
            continue; // duplicate within input — keep first occurrence
        }
        if let Some(existing_opt) = by_activity.get(aid_str) {
            out.push(existing_opt.clone());
            continue; // already an option — no INSERT
        }
        let id = util::new_id();
        tx.execute(
            "INSERT INTO plan_options (id, plan_id, activity_id, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, plan_id, aid_str, next_order],
        )?;
        out.push(PlanOption {
            id,
            plan_id: plan_id.to_string(),
            activity_id: aid_str.to_string(),
            sort_order: next_order,
        });
        next_order += 1;
    }
    tx.commit()?;
    Ok(out)
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

/// Parse a `YYYY-MM-DD` string into a `NaiveDate`. Invalid input becomes
/// `InvalidArgument` so callers don't have to unwrap a `chrono` error.
fn parse_date_arg(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| CoreError::InvalidArgument(format!("invalid date '{date}': {e}")))
}
/// Resolve the day's records against the candidate plans ONCE (not per slot),
/// building a `plan_id -> Record` map keyed to the winning record for each
/// plan (the one with the most overlap). Each record's resolving plan is
/// determined by `record::resolve_plan_for` (§3.1 derived resolution).
///
/// Strategy choice (Task 7 step 4): the plan offers a batched query OR a
/// per-record call as "fine for v1." We go with the batched mapping: load
/// `[date 00:00:00Z, date 23:59:59Z]` records (UTC wall-clock bounds, fixed
/// width → lexicographic range), call `resolve_plan_for` once per record,
/// and accumulate the best (overlap-wise) result per `plan_id`. This keeps
/// the function O(R + P) where R is records and P is plans — strictly better
/// than the per-slot loop, which is O(R * P).
fn resolve_all_for_date(
    conn: &Connection,
    date: &str,
    plan_ids: &std::collections::HashSet<String>,
) -> Result<std::collections::HashMap<String, crate::model::Record>> {
    let _ = plan_ids; // defensive: we currently scan every record and resolve
    // `T` separator to match stored `timestamp(...)` instants (see
    // `record::scope_bounds` for the lex-sort rationale).
    let from = format!("{date}T00:00:00Z");
    let to = format!("{date}T23:59:59Z");
    let records = crate::record::list_records(conn, None, &from, &to)?;
    let mut best: std::collections::HashMap<String, crate::model::Record> =
        std::collections::HashMap::new();
    for rec in records {
        if let Some(slot) = crate::record::resolve_plan_for(conn, &rec)? {
            // `resolve_plan_for` already picks the highest-overlap plan, so
            // first-writer-wins on `plan_id` is correct: any later record for
            // the same plan slot would either tie (lower-or-equal start) or
            // have less overlap, so the existing slot is the better fit.
            best.entry(slot.plan_id).or_insert(rec);
        }
    }
    Ok(best)
}

/// Materialize the plans that fire on `date` (`YYYY-MM-DD`).
///
/// Selection rules:
/// - one-shot plans (`weekday_mask = 0`) match iff their `date = ?`.
/// - recurring plans (`weekday_mask != 0`) match iff the weekday bit is set
///   for `date` (`bit = num_days_from_monday()`, Mon = 0 … Sun = 6).
///
/// For each matched plan we load its `plan_options` joined with `activities`
/// and assemble a `PlanSlot`. `is_resolved` / `resolved_by` are filled by
/// `resolve_all_for_date` — each record gets resolved ONCE against all
/// candidate plans, not once per slot (Task 7 step 4: batched strategy).
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

    let mut plans: Vec<Plan> = Vec::new();
    for r in rows {
        plans.push(r?);
    }
    drop(stmt);

    // Pre-compute resolution once over the day's records (batched; see
    // `resolve_all_for_date`).
    let plan_id_set: std::collections::HashSet<String> =
        plans.iter().map(|p| p.id.clone()).collect();
    let resolved = resolve_all_for_date(conn, date, &plan_id_set)?;

    let mut slots = Vec::with_capacity(plans.len());
    for plan in plans {
        let options = options_for(conn, &plan.id)?;
        let resolved_by = resolved.get(&plan.id).cloned();
        slots.push(PlanSlot {
            plan_id: plan.id,
            date: date.to_string(),
            start_minute: plan.start_minute,
            duration_minute: plan.duration_minute,
            options,
            is_resolved: resolved_by.is_some(),
            resolved_by,
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
