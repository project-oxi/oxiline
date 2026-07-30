//! Task CRUD + lazy materialization (`03-data-model.md` §3.5, §3.7).

use crate::error::{CoreError, Result};
use crate::model::{Task, TaskSource};
use crate::routines;
use crate::util;
use rusqlite::{params, Connection};

pub fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let source_str: String = row.get("source")?;
    Ok(Task {
        id: row.get("id")?,
        date: row.get("date")?,
        title: row.get("title")?,
        category_id: row.get("category_id")?,
        start_minute: row
            .get::<_, Option<i64>>("start_minute")?
            .map(|v| v as u16),
        duration_minute: row
            .get::<_, Option<i64>>("duration_minute")?
            .map(|v| v as u16),
        is_done: row.get::<_, i64>("is_done")? != 0,
        done_at: row.get("done_at")?,
        is_skipped: row.get::<_, i64>("is_skipped")? != 0,
        source: TaskSource::from_db_str(&source_str),
        source_routine_block_id: row.get("source_routine_block_id")?,
        notes: row.get("notes")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<Task> {
    conn.query_row(
        "SELECT * FROM tasks WHERE id = ?",
        params![id],
        row_from,
    )
    .map_err(CoreError::from)
}

/// Tasks for a specific date (manual + already-materialized routine rows).
pub fn list_by_date(conn: &Connection, date: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM tasks WHERE date = ? ORDER BY start_minute IS NULL, start_minute, sort_order",
    )?;
    let rows = stmt.query_map(params![date], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Backlog tasks (`date IS NULL`).
pub fn list_backlog(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM tasks WHERE date IS NULL ORDER BY sort_order, created_at",
    )?;
    let rows = stmt.query_map([], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Tasks across an inclusive `[from, to]` date range.
pub fn list_range(conn: &Connection, from: &str, to: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM tasks WHERE date IS NOT NULL AND date >= ? AND date <= ?
         ORDER BY date, start_minute IS NULL, start_minute",
    )?;
    let rows = stmt.query_map(params![from, to], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub struct NewTask {
    pub date: Option<String>,
    pub title: String,
    pub category_id: Option<String>,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub notes: Option<String>,
}

pub fn create(conn: &Connection, input: NewTask) -> Result<Task> {
    if let Some(m) = input.start_minute {
        if m > 1439 {
            return Err(CoreError::InvalidArgument(
                "start_minute must be 0..=1439".into(),
            ));
        }
    }
    if let Some(d) = input.duration_minute {
        if !(1..=1440).contains(&d) {
            return Err(CoreError::InvalidArgument(
                "duration_minute must be 1..=1440".into(),
            ));
        }
    }
    let id = util::new_id();
    let now = util::now_iso();
    let next_order: i64 = match &input.date {
        Some(d) => conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tasks WHERE date = ?",
                params![d],
                |r| r.get(0),
            )
            .unwrap_or(0),
        None => conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tasks WHERE date IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0),
    };
    conn.execute(
        "INSERT INTO tasks
         (id, date, title, category_id, start_minute, duration_minute, is_done, is_skipped,
          source, notes, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 'manual', ?7, ?8, ?9, ?9)",
        params![
            id,
            input.date,
            input.title,
            input.category_id,
            input.start_minute.map(|v| v as i64),
            input.duration_minute.map(|v| v as i64),
            input.notes,
            next_order,
            now,
        ],
    )?;
    get(conn, &id)
}

pub struct TaskUpdate {
    pub title: Option<String>,
    pub date: Option<Option<String>>, // Some(None) → move to backlog
    pub start_minute: Option<Option<u16>>,
    pub duration_minute: Option<Option<u16>>,
    pub category_id: Option<Option<String>>,
    pub notes: Option<Option<String>>,
}

pub fn update(conn: &Connection, id: &str, upd: TaskUpdate) -> Result<Task> {
    let existing = get(conn, id)?;
    let title = upd.title.unwrap_or(existing.title);
    let date = match upd.date {
        Some(v) => v,
        None => existing.date,
    };
    let start_minute = match upd.start_minute {
        Some(v) => v,
        None => existing.start_minute,
    };
    let duration_minute = match upd.duration_minute {
        Some(v) => v,
        None => existing.duration_minute,
    };
    let category_id = match upd.category_id {
        Some(v) => v,
        None => existing.category_id,
    };
    let notes = match upd.notes {
        Some(v) => v,
        None => existing.notes,
    };
    if let Some(m) = start_minute {
        if m > 1439 {
            return Err(CoreError::InvalidArgument(
                "start_minute must be 0..=1439".into(),
            ));
        }
    }
    if let Some(d) = duration_minute {
        if !(1..=1440).contains(&d) {
            return Err(CoreError::InvalidArgument(
                "duration_minute must be 1..=1440".into(),
            ));
        }
    }
    let now = util::now_iso();
    conn.execute(
        "UPDATE tasks SET title=?1, date=?2, start_minute=?3, duration_minute=?4,
         category_id=?5, notes=?6, updated_at=?7 WHERE id=?8",
        params![
            title,
            date,
            start_minute.map(|v| v as i64),
            duration_minute.map(|v| v as i64),
            category_id,
            notes,
            now,
            id,
        ],
    )?;
    get(conn, id)
}

pub fn set_done(conn: &Connection, id: &str, done: bool) -> Result<Task> {
    let now = util::now_iso();
    let done_at: Option<String> = if done { Some(now.clone()) } else { None };
    let n = conn.execute(
        "UPDATE tasks SET is_done=?1, done_at=?2, updated_at=?3 WHERE id=?4",
        params![done as i64, done_at, now, id],
    )?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("task '{id}'")));
    }
    get(conn, id)
}

pub fn set_skipped(conn: &Connection, id: &str, skipped: bool) -> Result<Task> {
    let now = util::now_iso();
    let n = conn.execute(
        "UPDATE tasks SET is_skipped=?1, updated_at=?2 WHERE id=?3",
        params![skipped as i64, now, id],
    )?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("task '{id}'")));
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM tasks WHERE id=?1", params![id])?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("task '{id}'")));
    }
    Ok(())
}

/// The synthetic virtual-id prefix used by the timeline (`03-data-model.md`
/// §3.7): `virtual:{block_id}:{date}`.
pub const VIRTUAL_PREFIX: &str = "virtual:";

/// Build the virtual id for a routine block on a given date.
pub fn virtual_id(block_id: &str, date: &str) -> String {
    format!("{VIRTUAL_PREFIX}{block_id}:{date}")
}

/// If `id` is a virtual occurrence id, materialize it into a real `tasks` row
/// (snapshot from its routine block) and return the real task id; otherwise
/// return `id` unchanged after verifying it exists.
pub fn materialize_if_virtual(conn: &Connection, id: &str) -> Result<String> {
    if let Some(rest) = id.strip_prefix(VIRTUAL_PREFIX) {
        let (block_id, date) = rest
            .rsplit_once(':')
            .ok_or_else(|| CoreError::InvalidArgument(format!("bad virtual id '{id}'")))?;
        let task = materialize_occurrence(conn, block_id, date)?;
        return Ok(task.id);
    }
    get(conn, id)?;
    Ok(id.to_string())
}

/// Materialize a routine occurrence for `date` into a real `tasks` row. If a
/// row already exists for `(block_id, date)` it is returned unchanged (the
/// unique index guarantees at most one). Every occurrence interaction funnels
/// through this single function (`03-data-model.md` §3.7).
pub fn materialize_occurrence(
    conn: &Connection,
    block_id: &str,
    date: &str,
) -> Result<Task> {
    // Already materialized?
    if let Ok(existing) = conn.query_row(
        "SELECT * FROM tasks WHERE source_routine_block_id = ? AND date = ?",
        params![block_id, date],
        row_from,
    ) {
        return Ok(existing);
    }
    let block = routines::get(conn, block_id)?;
    let id = util::new_id();
    let now = util::now_iso();
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tasks WHERE date = ?",
            params![date],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO tasks
         (id, date, title, category_id, start_minute, duration_minute, is_done, is_skipped,
          source, source_routine_block_id, notes, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 'routine', ?7, ?8, ?9, ?10, ?10)",
        params![
            id,
            date,
            block.title,
            block.category_id,
            block.start_minute as i64,
            block.duration_minute as i64,
            block_id,
            block.notes,
            next_order,
            now,
        ],
    )?;
    get(conn, &id)
}
