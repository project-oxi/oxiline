//! Completion & streak reporting — the single source of truth for all
//! completion/streak arithmetic (design spec §3). Reports-module-local: the
//! `created_at` scheduled bound (§2.1) lives here and is NOT applied to
//! `timeline.rs` (see spec §2.1 scope note).

use chrono::Datelike;
use rusqlite::Connection;

use crate::error::Result;
use crate::model::{CategoryBreakdown, DayBreakdown, RoutineBlock};
use crate::util;
use crate::{categories, routines, tasks};

type CatMap = std::collections::HashMap<Option<String>, (u32, u32, u32)>;

// ---- scheduled-set predicate (§2.1) ---------------------------------------

/// Is routine block `b` scheduled on `date` (YYYY-MM-DD)?
///
/// Four conditions (spec §2.1): active, weekday matches, within effective
/// range, and `date >= max(effective_from, created_at)`. This bound prevents
/// phantom pre-existence occurrences in past-looking reports.
pub fn scheduled_for(block: &RoutineBlock, date: &str) -> bool {
    if !block.is_active {
        return false;
    }
    let d = match util::parse_date(date) {
        Ok(d) => d,
        Err(_) => return false,
    };
    if !routines::mask_includes(block.weekday_mask, d.weekday()) {
        return false;
    }
    if !in_effective_range(&block.effective_from, &block.effective_until, date) {
        return false;
    }
    date >= bound_date(block).as_str()
}

/// `max(effective_from_date, created_at_date)` as a YYYY-MM-DD string.
fn bound_date(block: &RoutineBlock) -> String {
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
    if let Some(f) = from {
        if date < f.as_str() {
            return false;
        }
    }
    if let Some(u) = until {
        if date > u.as_str() {
            return false;
        }
    }
    true
}

// ---- per-day breakdown (§2.2, §2.3) ---------------------------------------

/// Bucket of a single scheduled occurrence on a date.
enum Bucket {
    Done,
    Skipped,
    NotRecorded,
    Upcoming,
}

fn bucket_of(is_done: bool, is_skipped: bool) -> Bucket {
    if is_skipped {
        Bucket::Skipped
    } else if is_done {
        Bucket::Done
    } else {
        Bucket::NotRecorded
    }
}

/// Is an occurrence on `date` (relative to `today`/`now_minute`) due?
/// Untimed items on today are due (available all day); future days are not.
fn is_due(
    date: &str,
    today: &str,
    start: Option<u16>,
    dur: Option<u16>,
    now_minute: u16,
) -> bool {
    if date < today {
        return true; // past → all due
    }
    if date > today {
        return false; // future → none due
    }
    match start {
        // today
        None => true, // untimed → available all day → due
        Some(s) => s + dur.unwrap_or(0) <= now_minute,
    }
}

/// Tally one occurrence into the per-category map (or the upcoming counter).
fn tally(by_cat: &mut CatMap, upcoming: &mut u32, cid: Option<String>, b: Bucket) {
    match b {
        Bucket::Upcoming => *upcoming += 1,
        Bucket::Done => by_cat.entry(cid).or_insert((0, 0, 0)).0 += 1,
        Bucket::Skipped => by_cat.entry(cid).or_insert((0, 0, 0)).1 += 1,
        Bucket::NotRecorded => by_cat.entry(cid).or_insert((0, 0, 0)).2 += 1,
    }
}

/// Reconstruct the merged scheduled set for `date` (materialized tasks +
/// virtual occurrences with the created_at bound), classified into the three
/// buckets (§2.2) with the temporal boundary applied (§2.3).
pub fn day_breakdown(
    conn: &Connection,
    date: &str,
    today: &str,
    now_minute: u16,
) -> Result<DayBreakdown> {
    let cats = categories::list(conn)?;
    let name_of = |cid: &Option<String>| -> String {
        cats.iter()
            .find(|c| Some(&c.id) == cid.as_ref())
            .map(|c| c.name.clone())
            .unwrap_or_default()
    };

    let mut by_cat: CatMap = CatMap::new();
    let mut upcoming: u32 = 0;

    // (A) materialized tasks for this date (manual + materialized routine rows).
    let day_tasks = tasks::list_by_date(conn, date)?;
    for t in &day_tasks {
        let due = is_due(date, today, t.start_minute, t.duration_minute, now_minute);
        let b = if due {
            bucket_of(t.is_done, t.is_skipped)
        } else {
            Bucket::Upcoming
        };
        tally(&mut by_cat, &mut upcoming, t.category_id.clone(), b);
    }

    // (B) virtual occurrences: active routines scheduled on `date` with no row.
    let materialized: std::collections::HashSet<String> = day_tasks
        .iter()
        .filter_map(|t| t.source_routine_block_id.clone())
        .collect();
    for b in routines::list(conn, true)? {
        if materialized.contains(&b.id) {
            continue;
        }
        if !scheduled_for(&b, date) {
            continue;
        }
        let due = is_due(
            date,
            today,
            Some(b.start_minute),
            Some(b.duration_minute),
            now_minute,
        );
        let bk = if due {
            Bucket::NotRecorded
        } else {
            Bucket::Upcoming
        };
        tally(&mut by_cat, &mut upcoming, b.category_id.clone(), bk);
    }

    let (done, skipped, not_recorded) = sum_categories(&by_cat);
    let categories = build_cat_breakdown(&by_cat, &name_of);
    Ok(DayBreakdown {
        date: date.into(),
        done,
        skipped,
        not_recorded,
        upcoming,
        completion_rate: rate(done, not_recorded),
        categories,
    })
}

fn sum_categories(by_cat: &CatMap) -> (u32, u32, u32) {
    let mut done = 0;
    let mut skipped = 0;
    let mut not_recorded = 0;
    for (_, (d, s, n)) in by_cat {
        done += d;
        skipped += s;
        not_recorded += n;
    }
    (done, skipped, not_recorded)
}

fn rate(done: u32, not_recorded: u32) -> Option<f64> {
    let denom = done + not_recorded;
    if denom == 0 {
        None
    } else {
        Some(done as f64 / denom as f64)
    }
}

fn build_cat_breakdown(
    by_cat: &CatMap,
    name_of: &dyn Fn(&Option<String>) -> String,
) -> Vec<CategoryBreakdown> {
    let mut v: Vec<CategoryBreakdown> = by_cat
        .iter()
        .map(|(cid, (d, s, n))| CategoryBreakdown {
            category_id: cid.clone(),
            category_name: name_of(cid),
            done: *d,
            skipped: *s,
            not_recorded: *n,
            completion_rate: rate(*d, *n),
        })
        .collect();
    v.sort_by(|a, b| a.category_name.cmp(&b.category_name));
    v
}
