//! Card suggestions for quick-add autocomplete.
//!
//! [`suggest`] merges on-demand *templates* (`routine_blocks` with
//! `weekday_mask == 0`) with the distinct historical card signatures drawn
//! from past `tasks` and recurring `routine_blocks`. Both the GUI palette and
//! (future) CLI quick-add call this single function so they never disagree on
//! what counts as a "previously created card".
//!
//! Zero schema change: templates reuse `routine_blocks` (mask=0 is excluded
//! from every timeline/now/report path by `routines::scheduled_for`), and
//! history is a plain read query over existing rows (`03-data-model.md` §3.1:
//! no new table for a variation like this).

use std::collections::HashSet;

use crate::error::Result;
use crate::model::{CardSuggestion, RoutineBlock};
use crate::routines;
use rusqlite::Connection;

/// Default cap on the number of suggestions returned when `limit == 0`.
pub const DEFAULT_LIMIT: usize = 100;

/// Build a ranked suggestion list for quick-add.
///
/// Order: on-demand templates first (by `updated_at` desc), then task history
/// (by most-recent use desc), then recurring-routine titles. Titles are
/// de-duplicated case-insensitively; the first source to claim a title wins
/// (templates > tasks > recurring routines).
pub fn suggest(conn: &Connection, limit: usize) -> Result<Vec<CardSuggestion>> {
    let limit = if limit == 0 { DEFAULT_LIMIT } else { limit };
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<CardSuggestion> = Vec::new();

    // Partition routine_blocks into templates (mask=0) and recurring (mask!=0).
    let mut templates: Vec<RoutineBlock> = Vec::new();
    let mut recurring: Vec<RoutineBlock> = Vec::new();
    for b in routines::list(conn, false)? {
        if routines::is_template(&b) {
            templates.push(b);
        } else {
            recurring.push(b);
        }
    }
    templates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    recurring.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    // (A) On-demand templates — curated predefined cards. `list(false)` so a
    // paused template still curates in suggestions.
    for b in &templates {
        if let Some(s) = sig_from_block(b, true)
            && seen.insert(key(&s.title))
        {
            out.push(s);
        }
    }

    // (B) History from tasks: most-recent row per distinct title.
    for s in task_signatures(conn)? {
        if seen.insert(key(&s.title)) {
            out.push(s);
        }
    }

    // (C) History from recurring routines: their titles are also "previously
    // created cards" worth autocompleting.
    for b in &recurring {
        if let Some(s) = sig_from_block(b, false)
            && seen.insert(key(&s.title))
        {
            out.push(s);
        }
    }

    out.truncate(limit);
    Ok(out)
}

/// Case-insensitive, trimmed de-dup key.
fn key(title: &str) -> String {
    title.trim().to_ascii_lowercase()
}

/// Build a suggestion from a routine block. Returns `None` for an empty title.
fn sig_from_block(b: &RoutineBlock, is_template: bool) -> Option<CardSuggestion> {
    let title = b.title.trim();
    if title.is_empty() {
        return None;
    }
    Some(CardSuggestion {
        title: title.to_string(),
        category_id: b.category_id.clone(),
        duration_minute: Some(b.duration_minute),
        notes: b.notes.clone(),
        is_template,
        template_id: Some(b.id.clone()),
        last_used_at: Some(b.updated_at.clone()),
    })
}

/// Distinct task titles with their most-recently-used category/duration/notes,
/// newest first. Bounded by [`HISTORY_CAP`] to keep the pool cheap even for
/// long-lived databases.
const HISTORY_CAP: i64 = 500;
fn task_signatures(conn: &Connection) -> Result<Vec<CardSuggestion>> {
    // Most-recent row per title: join the per-title MAX(updated_at) back to
    // the row carrying those fields. SQLite's GROUP BY would otherwise pick
    // an arbitrary row's category/duration.
    let mut stmt = conn.prepare(
        "SELECT t.title, t.category_id, t.duration_minute, t.notes, t.updated_at
         FROM tasks t
         INNER JOIN (
             SELECT title, MAX(updated_at) AS max_at
             FROM tasks GROUP BY title
         ) m ON m.title = t.title AND m.max_at = t.updated_at
         ORDER BY t.updated_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![HISTORY_CAP], |row| {
        Ok(CardSuggestion {
            title: row.get::<_, String>(0)?,
            category_id: row.get(1)?,
            duration_minute: row.get::<_, Option<i64>>(2)?.map(|v| v as u16),
            notes: row.get(3)?,
            is_template: false,
            template_id: None,
            last_used_at: Some(row.get(4)?),
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
