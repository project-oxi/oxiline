//! Timeline merge logic — the single place that combines virtual occurrences
//! with materialized tasks (`03-data-model.md` §3.7).
//!
//! Both the GUI and `oxiline today`/`oxiline now` call [`get_timeline_for_date`]
//! / [`get_now_context`] so they can never disagree.

use std::collections::HashSet;

use crate::error::Result;
use crate::model::{NowContext, NowItem, Task, TimelineItem};
use crate::{routines, tasks, util};
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

/// "What is happening now" — shared verbatim by the HUD and `oxiline now`
/// (`05-cli-spec.md` §5.2). `now_minute` is the local minute-of-day.
pub fn get_now_context(conn: &Connection, now_minute: u16) -> Result<NowContext> {
    let date = util::today_local();
    let items = get_timeline_for_date(conn, &date)?;

    let mut current: Option<&TimelineItem> = None;
    for it in items.iter().filter(|i| !i.is_skipped) {
        if let (Some(s), Some(d)) = (it.start_minute, it.duration_minute) {
            if now_minute >= s && now_minute < s + d {
                current = Some(it);
                break;
            }
        }
    }

    let next = items
        .iter()
        .filter(|i| !i.is_skipped)
        .filter(|i| i.start_minute.is_some_and(|s| s > now_minute))
        .min_by_key(|i| i.start_minute.unwrap_or(u16::MAX));

    let to_now = |it: &TimelineItem| NowItem {
        id: it.id.clone(),
        is_virtual: it.is_virtual,
        title: it.title.clone(),
        start_minute: it.start_minute,
        duration_minute: it.duration_minute,
        category_id: it.category_id.clone(),
        remaining_minute: None,
        starts_in_minute: None,
    };

    let mut current_item = current.map(to_now);
    if let (Some(ci), Some(src)) = (&mut current_item, current) {
        if let (Some(s), Some(d)) = (src.start_minute, src.duration_minute) {
            let end = (s as i64) + (d as i64);
            ci.remaining_minute = Some(end - now_minute as i64);
        }
    }

    let mut next_item = next.map(to_now);
    if let (Some(ni), Some(src)) = (&mut next_item, next) {
        if let Some(s) = src.start_minute {
            ni.starts_in_minute = Some(s as i64 - now_minute as i64);
        }
    }

    Ok(NowContext {
        current: current_item,
        next: next_item,
        generated_at_minute: now_minute,
        generated_at: util::now_iso(),
    })
}
