//! Integration tests for the plans module (Task 4).
//!
//! Test harness mirrors `tests/activities.rs`: a `db()` helper that opens an
//! ephemeral SQLite file via `oxiline_core::open_and_migrate`, then runs
//! `settings::ensure_defaults` so seeded settings are present. `:memory:`
//! databases do not work with `open_and_migrate` (it takes a `&Path`).
//!
//! Note: `2026-08-03` MUST be a Monday for `slots_for_date` to materialize the
//! recurring plan below (weekday_mask = bit0 = Monday). Verified via
//! `chrono::Datelike` / Python `date.weekday()`; the recurrence predicate is
//! `(mask >> weekday_bit) & 1 == 1` with `bit = num_days_from_monday()`.

use rusqlite::Connection;
use tempfile::NamedTempFile;

fn db() -> (NamedTempFile, Connection) {
    let f = NamedTempFile::new().unwrap();
    let c = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&c).unwrap();
    (f, c)
}

#[test]
fn plan_holds_or_options_and_materializes_per_date() {
    let (_f, c) = db();
    let a1 = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("코딩".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let a2 = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("독서".into()),
            ..Default::default()
        },
    )
    .unwrap();
    // recurring plan: weekday bit for Monday (bit0), 11:00–13:00, options 코딩 OR 독서
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 11 * 60,
            duration_minute: 120,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a1.id.clone(), a2.id.clone()],
        },
    )
    .unwrap();
    let slots = oxiline_core::plan::slots_for_date(&c, "2026-08-03").unwrap(); // 2026-08-03 is a Monday
    let ours = slots.iter().find(|s| s.plan_id == p.id).unwrap();
    assert_eq!(ours.options.len(), 2);
    assert!(!ours.is_resolved); // no records yet
}