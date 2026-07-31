//! Routine block CRUD (`03-data-model.md` §3.3) + weekday-mask helpers.

use crate::error::{CoreError, Result};
use crate::model::RoutineBlock;
use crate::util;
use chrono::Datelike;
use rusqlite::{Connection, params};

pub fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<RoutineBlock> {
    Ok(RoutineBlock {
        id: row.get("id")?,
        group_id: row.get("group_id")?,
        title: row.get("title")?,
        category_id: row.get("category_id")?,
        start_minute: row.get::<_, i64>("start_minute")? as u16,
        duration_minute: row.get::<_, i64>("duration_minute")? as u16,
        weekday_mask: row.get::<_, i64>("weekday_mask")? as u8,
        effective_from: row.get("effective_from")?,
        effective_until: row.get("effective_until")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        color_override: row.get("color_override")?,
        notes: row.get("notes")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list(conn: &Connection, active_only: bool) -> Result<Vec<RoutineBlock>> {
    let sql = if active_only {
        "SELECT * FROM routine_blocks WHERE is_active = 1 ORDER BY start_minute, sort_order"
    } else {
        "SELECT * FROM routine_blocks ORDER BY is_active DESC, start_minute, sort_order"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> Result<RoutineBlock> {
    conn.query_row(
        "SELECT * FROM routine_blocks WHERE id = ?",
        params![id],
        row_from,
    )
    .map_err(CoreError::from)
}

pub struct NewRoutineBlock {
    pub title: String,
    pub start_minute: u16,
    pub duration_minute: u16,
    pub weekday_mask: u8,
    pub category_id: Option<String>,
    pub effective_from: Option<String>,
    pub effective_until: Option<String>,
    pub notes: Option<String>,
}

pub fn create(conn: &Connection, input: NewRoutineBlock) -> Result<RoutineBlock> {
    validate_minute(input.start_minute)?;
    if !(1..=1440).contains(&input.duration_minute) {
        return Err(CoreError::InvalidArgument(
            "duration_minute must be 1..=1440".into(),
        ));
    }
    if input.weekday_mask == 0 {
        return Err(CoreError::InvalidArgument(
            "weekday_mask must select at least one day".into(),
        ));
    }
    let id = util::new_id();
    let now = util::now_iso();
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM routine_blocks",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO routine_blocks
         (id, title, category_id, start_minute, duration_minute, weekday_mask,
          effective_from, effective_until, is_active, notes, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11, ?11)",
        params![
            id,
            input.title,
            input.category_id,
            input.start_minute as i64,
            input.duration_minute as i64,
            input.weekday_mask as i64,
            input.effective_from,
            input.effective_until,
            input.notes,
            next_order,
            now,
        ],
    )?;
    get(conn, &id)
}

/// Partial update. `None` fields are left unchanged.
pub struct RoutineUpdate {
    pub title: Option<String>,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub weekday_mask: Option<u8>,
    pub category_id: Option<Option<String>>, // Some(None) → clear
    pub notes: Option<Option<String>>,
}

pub fn update(conn: &Connection, id: &str, upd: RoutineUpdate) -> Result<RoutineBlock> {
    let existing = get(conn, id)?;
    let title = upd.title.unwrap_or(existing.title);
    let start_minute = match upd.start_minute {
        Some(v) => {
            validate_minute(v)?;
            v
        }
        None => existing.start_minute,
    };
    let duration_minute = match upd.duration_minute {
        Some(v) => {
            if !(1..=1440).contains(&v) {
                return Err(CoreError::InvalidArgument(
                    "duration_minute must be 1..=1440".into(),
                ));
            }
            v
        }
        None => existing.duration_minute,
    };
    let weekday_mask = match upd.weekday_mask {
        Some(0) => {
            return Err(CoreError::InvalidArgument(
                "weekday_mask must select at least one day".into(),
            ));
        }
        Some(v) => v,
        None => existing.weekday_mask,
    };
    let category_id = match upd.category_id {
        Some(v) => v,
        None => existing.category_id,
    };
    let notes = match upd.notes {
        Some(v) => v,
        None => existing.notes,
    };
    let now = util::now_iso();
    conn.execute(
        "UPDATE routine_blocks SET title=?1, start_minute=?2, duration_minute=?3,
         weekday_mask=?4, category_id=?5, notes=?6, updated_at=?7 WHERE id=?8",
        params![
            title,
            start_minute as i64,
            duration_minute as i64,
            weekday_mask as i64,
            category_id,
            notes,
            now,
            id,
        ],
    )?;
    get(conn, id)
}

pub fn set_active(conn: &Connection, id: &str, active: bool) -> Result<RoutineBlock> {
    let now = util::now_iso();
    let n = conn.execute(
        "UPDATE routine_blocks SET is_active=?1, updated_at=?2 WHERE id=?3",
        params![active as i64, now, id],
    )?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("routine '{id}'")));
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM routine_blocks WHERE id=?1", params![id])?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("routine '{id}'")));
    }
    Ok(())
}

fn validate_minute(m: u16) -> Result<()> {
    if m > 1439 {
        return Err(CoreError::InvalidArgument(
            "start_minute must be 0..=1439".into(),
        ));
    }
    Ok(())
}

/// Parse a `--days` spec into a weekday mask (bit0=Mon … bit6=Sun).
///
/// Accepts presets (`weekdays`, `weekends`, `daily`) or a comma list of
/// `mon,tue,wed,thu,fri,sat,sun` (`05-cli-spec.md`).
pub fn parse_days_spec(spec: &str) -> Result<u8> {
    let lower = spec.trim().to_ascii_lowercase();
    match lower.as_str() {
        "daily" | "everyday" | "every-day" => return Ok(0b1111111),
        "weekdays" => return Ok(0b0001111),
        "weekends" => return Ok(0b1100000),
        _ => {}
    }
    let mut mask = 0u8;
    for tok in lower.split(',') {
        let bit = match tok.trim() {
            "mon" | "mo" | "m" => 0,
            "tue" | "tu" | "t" => 1,
            "wed" | "we" | "w" => 2,
            "thu" | "th" => 3,
            "fri" | "fr" | "f" => 4,
            "sat" | "sa" => 5,
            "sun" | "su" => 6,
            other => return Err(CoreError::InvalidArgument(format!("unknown day '{other}'"))),
        };
        mask |= 1 << bit;
    }
    if mask == 0 {
        return Err(CoreError::InvalidArgument(
            "weekday_mask must select at least one day".into(),
        ));
    }
    Ok(mask)
}

/// Does `mask` include the given weekday?
pub fn mask_includes(mask: u8, weekday: chrono::Weekday) -> bool {
    let bit = util::weekday_mask_bit(weekday);
    (mask & (1 << bit)) != 0
}

// ---- scheduled-on-date predicate (§2.1) -----------------------------------

/// Is routine block `b` scheduled on `date` (YYYY-MM-DD)?
///
/// Four conditions (spec §2.1): active, weekday matches, within effective
/// range, and `date >= max(effective_from, created_at)`. This bound prevents
/// phantom pre-existence occurrences in past-looking views (timeline WeekView,
/// reports). Note: `created_at` is UTC but `date`/WeekView is local — a
/// routine created near a UTC midnight boundary may be off by up to ~1 day
/// locally; acceptable for a single-user desktop app.
pub fn scheduled_for(block: &RoutineBlock, date: &str) -> bool {
    if !block.is_active {
        return false;
    }
    let d = match util::parse_date(date) {
        Ok(d) => d,
        Err(_) => return false,
    };
    if !mask_includes(block.weekday_mask, d.weekday()) {
        return false;
    }
    if !in_effective_range(&block.effective_from, &block.effective_until, date) {
        return false;
    }
    date >= bound_date(block).as_str()
}

/// `max(effective_from_date, created_at_date)` as a YYYY-MM-DD string.
pub fn bound_date(block: &RoutineBlock) -> String {
    let created_day = block
        .created_at
        .get(..10)
        .unwrap_or(&block.created_at)
        .to_string();
    match &block.effective_from {
        Some(f) if f.as_str() > created_day.as_str() => f.clone(),
        _ => created_day,
    }
}

fn in_effective_range(from: &Option<String>, until: &Option<String>, date: &str) -> bool {
    if let Some(f) = from
        && date < f.as_str()
    {
        return false;
    }
    if let Some(u) = until
        && date > u.as_str()
    {
        return false;
    }
    true
}
