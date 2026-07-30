//! Completion & streak reporting — the single source of truth for all
//! completion/streak arithmetic (design spec §3). Reports-module-local: the
//! `created_at` scheduled bound (§2.1) lives here and is NOT applied to
//! `timeline.rs` (see spec §2.1 scope note).

use crate::error::Result;
use crate::model::RoutineBlock;
use crate::util;
use chrono::Datelike;

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
    if !crate::routines::mask_includes(block.weekday_mask, d.weekday()) {
        return false;
    }
    if !in_effective_range(&block.effective_from, &block.effective_until, date) {
        return false;
    }
    date >= bound_date(block).as_str()
}

/// `max(effective_from_date, created_at_date)` as a YYYY-MM-DD string.
fn bound_date(block: &RoutineBlock) -> String {
    let created_day = block.created_at.get(..10).unwrap_or(&block.created_at).to_string();
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
