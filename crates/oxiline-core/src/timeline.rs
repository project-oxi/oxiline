//! Timeline merge logic — the single place that combines virtual occurrences
//! with materialized tasks (`03-data-model.md` §3.7).
//!
//! Both the GUI and `oxiline today` call [`get_timeline_for_date`]. (`oxiline
//! now` now derives from the recording layer via `plan::now_summary`.)

use std::collections::HashSet;

use crate::error::Result;
use crate::model::{Task, TimelineItem};
use crate::{routines, tasks};
use rusqlite::Connection;

fn task_to_item(t: &Task) -> TimelineItem {
    TimelineItem {
        id: t.id.clone(),
        is_virtual: false,
        title: t.title.clone(),
        start_minute: t.start_minute,
        duration_minute: t.duration_minute,
        category_id: t.category_id.clone(),
        is_done: t.is_done,
        is_skipped: t.is_skipped,
        origin_routine_block_id: t.source_routine_block_id.clone(),
    }
}

/// The merged view for `date`: manual tasks + materialized routine rows +
/// virtual occurrences for active routines that apply on `date` and haven't
/// been materialized yet. Skipped occurrences are hidden (`03-data-model.md`
/// §3.7: skip == delete-this-occurrence).
pub fn get_timeline_for_date(conn: &Connection, date: &str) -> Result<Vec<TimelineItem>> {
    let day_tasks = tasks::list_by_date(conn, date)?;
    let mut items: Vec<TimelineItem> = Vec::with_capacity(day_tasks.len() + 8);
    let mut materialized: HashSet<String> = HashSet::new();

    for t in &day_tasks {
        // Register materialization even for skipped rows so the virtual
        // occurrence is suppressed.
        if let Some(bid) = &t.source_routine_block_id {
            materialized.insert(bid.clone());
        }
        if t.is_skipped {
            continue;
        }
        items.push(task_to_item(t));
    }

    // Virtual occurrences from active routine blocks. Uses the shared
    // `routines::scheduled_for` predicate so the timeline and reports can
    // never disagree on what counts as "scheduled on this date" — including
    // the `created_at` bound that suppresses phantom pre-existence on past
    // dates (§2.1).
    for b in routines::list(conn, true)? {
        if materialized.contains(&b.id) {
            continue;
        }
        if !routines::scheduled_for(&b, date) {
            continue;
        }
        items.push(TimelineItem {
            id: tasks::virtual_id(&b.id, date),
            is_virtual: true,
            title: b.title.clone(),
            start_minute: Some(b.start_minute),
            duration_minute: Some(b.duration_minute),
            category_id: b.category_id.clone(),
            is_done: false,
            is_skipped: false,
            origin_routine_block_id: Some(b.id.clone()),
        });
    }

    // Sort: timed items by start time, untimed (no start) last.
    items.sort_by(|a, b| match (a.start_minute, b.start_minute) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    Ok(items)
}

