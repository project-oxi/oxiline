//! Routine group CRUD (`03-data-model.md` §3.4).
//!
//! Groups bundle routine blocks into categories (e.g. "Morning", "Work", "Health").
//! Schema exists from V1; this module exposes the CRUD for the Phase 2 UI.

use crate::error::Result;
use crate::model::RoutineGroup;
use rusqlite::{params, Connection};

fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<RoutineGroup> {
    Ok(RoutineGroup {
        id: row.get("id")?,
        name: row.get("name")?,
        icon: row.get("icon")?,
        is_active: row.get("is_active")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<RoutineGroup>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, is_active, sort_order, created_at, updated_at
         FROM routine_groups ORDER BY sort_order, name",
    )?;
    let rows = stmt.query_map([], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> Result<RoutineGroup> {
    conn.query_row(
        "SELECT id, name, icon, is_active, sort_order, created_at, updated_at
         FROM routine_groups WHERE id = ?",
        params![id],
        row_from,
    )
    .map_err(Into::into)
}

pub struct NewRoutineGroup {
    pub name: String,
    pub icon: Option<String>,
}

pub fn create(conn: &Connection, input: NewRoutineGroup) -> Result<RoutineGroup> {
    let id = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let sort_order: i64 = conn
        .query_row("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM routine_groups", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO routine_groups (id, name, icon, is_active, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, ?4, ?5, ?5)",
        params![id, input.name, input.icon, sort_order, now],
    )?;
    get(conn, &id)
}

pub struct RoutineGroupUpdate {
    pub name: Option<String>,
    pub icon: Option<Option<String>>,
    pub sort_order: Option<i64>,
}

pub fn update(conn: &Connection, id: &str, upd: RoutineGroupUpdate) -> Result<RoutineGroup> {
    let existing = get(conn, id)?;
    let name = upd.name.unwrap_or(existing.name);
    let icon = upd.icon.unwrap_or(existing.icon);
    let sort_order = upd.sort_order.unwrap_or(existing.sort_order);
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE routine_groups SET name=?1, icon=?2, sort_order=?3, updated_at=?4 WHERE id=?5",
        params![name, icon, sort_order, now, id],
    )?;
    get(conn, id)
}

pub fn set_active(conn: &Connection, id: &str, active: bool) -> Result<RoutineGroup> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE routine_groups SET is_active=?1, updated_at=?2 WHERE id=?3",
        params![active, now, id],
    )?;
    // Also toggle all routine_blocks belonging to this group.
    conn.execute(
        "UPDATE routine_blocks SET is_active=?1 WHERE group_id=?2",
        params![active, id],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    // ON DELETE SET NULL handles routine_blocks.group_id.
    conn.execute("DELETE FROM routine_groups WHERE id = ?", params![id])?;
    Ok(())
}
