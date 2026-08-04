//! Activity CRUD (`2026-08-01-record-layer-design.md` §5).
//!
//! Activities are the switchable, budgetable unit of work. They are referenced
//! by `plan_options` (the OR alternatives within a plan) and `records`
//! (the actual recorded sessions). Resolution accepts either an exact id or a
//! case-insensitive exact name; multiple matches → `AmbiguousCategory`,
//! none → `NotFound`. (The error variant keeps its categorical name because
//! activities reuse the same ambiguity semantics.)

use crate::error::{CoreError, Result};
use crate::model::{Activity, ActivityInput};
use crate::util;
use rusqlite::{Connection, params};

pub fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Activity> {
    Ok(Activity {
        id: row.get("id")?,
        name: row.get("name")?,
        hue_label: row.get("hue_label")?,
        icon: row.get("icon")?,
        category_id: row.get("category_id")?,
        target_minutes_daily: row
            .get::<_, Option<i64>>("target_minutes_daily")?
            .map(|v| v as u32),
        target_minutes_weekly: row
            .get::<_, Option<i64>>("target_minutes_weekly")?
            .map(|v| v as u32),
        is_active: row.get::<_, i64>("is_active")? != 0,
        sort_order: row.get("sort_order")?,
    })
}

pub fn list_activities(conn: &Connection, active_only: bool) -> Result<Vec<Activity>> {
    let mut out = Vec::new();
    if active_only {
        let mut stmt =
            conn.prepare("SELECT * FROM activities WHERE is_active = 1 ORDER BY sort_order, name")?;
        for r in stmt.query_map([], row_from)? {
            out.push(r?);
        }
    } else {
        let mut stmt = conn.prepare("SELECT * FROM activities ORDER BY sort_order, name")?;
        for r in stmt.query_map([], row_from)? {
            out.push(r?);
        }
    }
    Ok(out)
}

pub fn get_activity(conn: &Connection, id: &str) -> Result<Activity> {
    conn.query_row(
        "SELECT * FROM activities WHERE id = ?",
        params![id],
        row_from,
    )
    .map_err(CoreError::from)
}

fn get_by_name(conn: &Connection, name: &str) -> Result<Vec<Activity>> {
    let mut stmt =
        conn.prepare("SELECT * FROM activities WHERE is_active = 1 AND lower(name) = lower(?)")?;
    let rows = stmt.query_map(params![name], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Resolve an activity by exact id, else by case-insensitive exact name.
/// Ambiguous names error.
pub fn resolve_activity(conn: &Connection, id_or_name: &str) -> Result<Activity> {
    // First try exact id match.
    if let Ok(a) = get_activity(conn, id_or_name) {
        return Ok(a);
    }
    // Fall back to case-insensitive name lookup.
    let by_name = get_by_name(conn, id_or_name)?;
    match by_name.len() {
        0 => Err(CoreError::NotFound(format!("activity '{id_or_name}'"))),
        1 => Ok(by_name[0].clone()),
        // Multiple activities can legitimately share a display name;
        // surface the same error class used by categories so callers
        // can disambiguate by id.
        _ => Err(CoreError::AmbiguousCategory(id_or_name.to_string())),
    }
}

pub fn create_activity(conn: &Connection, input: ActivityInput) -> Result<Activity> {
    let id = util::new_id();
    let now = util::now_iso();
    let next_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM activities",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let name = input
        .name
        .clone()
        .ok_or_else(|| CoreError::InvalidArgument("activity.name is required".into()))?;
    // Defaults supplied at the call boundary: is_active defaults to true
    // when the caller passes None; sort_order defaults to the next order.
    let is_active: i64 = if input.is_active.unwrap_or(true) {
        1
    } else {
        0
    };
    let sort_order = input.sort_order.unwrap_or(next_order);
    let daily: Option<i64> = input.target_minutes_daily.flatten().map(|v| v as i64);
    let weekly: Option<i64> = input.target_minutes_weekly.flatten().map(|v| v as i64);
    conn.execute(
        "INSERT INTO activities (id, name, hue_label, icon, category_id,
            target_minutes_daily, target_minutes_weekly,
            is_active, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            id,
            name,
            input.hue_label,
            input.icon,
            input.category_id,
            daily,
            weekly,
            is_active,
            sort_order,
            now,
        ],
    )?;
    get_activity(conn, &id)
}

/// Update an existing activity. Double-Option targets follow the spec:
/// outer `None` => leave unchanged; outer `Some(None)` => clear;
/// outer `Some(Some(v))` => set. Other fields use plain Option:
/// `None` => leave unchanged; `Some(v)` => set (including `Some(None)`
/// for nullable columns like `hue_label`).
pub fn update_activity(conn: &Connection, id: &str, input: ActivityInput) -> Result<Activity> {
    // Verify the activity exists so we can return NotFound for unknown ids
    // instead of silently no-op'ing the UPDATE.
    let _existing = get_activity(conn, id)?;

    let now = util::now_iso();
    // Build dynamic SET clauses for fields the caller touched.
    // We deliberately don't fold this into prepared-statement params
    // for set/clear tri-state, since SQLite has no UPSERT-style bind
    // for nullable columns without COALESCE gymnastics.
    let mut sets: Vec<&'static str> = Vec::new();
    let mut bind: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(name) = &input.name {
        sets.push("name = ?");
        bind.push(Box::new(name.clone()));
    }
    if let Some(hue) = &input.hue_label {
        sets.push("hue_label = ?");
        bind.push(Box::new(hue.clone()));
    }
    if let Some(icon) = &input.icon {
        sets.push("icon = ?");
        bind.push(Box::new(icon.clone()));
    }
    if let Some(cat) = &input.category_id {
        sets.push("category_id = ?");
        bind.push(Box::new(cat.clone()));
    }
    // Double-Option tri-state for nullable budget columns.
    if let Some(daily) = input.target_minutes_daily {
        sets.push("target_minutes_daily = ?");
        bind.push(Box::new(daily.map(|v| v as i64)));
    }
    if let Some(weekly) = input.target_minutes_weekly {
        sets.push("target_minutes_weekly = ?");
        bind.push(Box::new(weekly.map(|v| v as i64)));
    }
    if let Some(active) = input.is_active {
        sets.push("is_active = ?");
        bind.push(Box::new(if active { 1i64 } else { 0i64 }));
    }
    if let Some(order) = input.sort_order {
        sets.push("sort_order = ?");
        bind.push(Box::new(order));
    }
    sets.push("updated_at = ?");
    bind.push(Box::new(now));

    if sets.is_empty() {
        // Nothing to change but the caller passed an id — still bump
        // updated_at for parity with previous modules. Always set it
        // above, so this branch is unreachable in practice, but keep
        // the guard explicit for future readers.
        return get_activity(conn, id);
    }

    let sql = format!("UPDATE activities SET {} WHERE id = ?", sets.join(", "));
    let mut params_with_id: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for b in bind.iter() {
        params_with_id.push(b.as_ref());
    }
    params_with_id.push(&id);
    conn.execute(&sql, params_with_id.as_slice())?;
    get_activity(conn, id)
}

/// Delete an activity. By default refuses when records exist (history is the
/// product; see spec §11). `force=true` cascades the delete through records
/// in a single transaction — the schema's `ON DELETE RESTRICT` is the DB-level
/// backstop, this is the structured app-level guard.
pub fn delete_activity(conn: &Connection, id: &str, force: bool) -> Result<()> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM records WHERE activity_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if count > 0 && !force {
        return Err(CoreError::Conflict(format!(
            "activity {} has {} record(s); refuse without --force",
            id, count
        )));
    }

    let tx = conn.unchecked_transaction()?;
    if count > 0 {
        tx.execute("DELETE FROM records WHERE activity_id = ?1", params![id])?;
    }
    let n = tx.execute("DELETE FROM activities WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("activity '{id}'")));
    }
    tx.commit()?;
    Ok(())
}
