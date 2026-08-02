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

use chrono::{TimeZone, Utc};
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

#[test]
fn slot_marked_resolved_after_record() {
    // Task 7 brief: after starting a record at 09:10 Monday inside a
    // 09:00–10:30 plan window, `slots_for_date("2026-08-03")` must surface the
    // slot with `is_resolved == true`. The wiring (`plan::slots_for_date`
    // calling `record::resolve_plan_for` for every record on the date) is the
    // single behavioural change vs Task 4.
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("코딩".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 90,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a.id.clone()],
        },
    )
    .unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 3, 9, 10, 0).unwrap();
    oxiline_core::record::start(&c, &a.id, now, "2026-08-03").unwrap();
    let slots = oxiline_core::plan::slots_for_date(&c, "2026-08-03").unwrap();
    let ours = slots.iter().find(|s| s.plan_id == p.id).unwrap();
    assert!(
        ours.is_resolved,
        "the slot should be resolved after a matching record was created in Task 7"
    );
}

fn mk_activity(c: &Connection, name: &str) -> oxiline_core::model::Activity {
    oxiline_core::activities::create_activity(
        c,
        oxiline_core::model::ActivityInput { name: Some(name.into()), ..Default::default() },
    )
    .unwrap()
}

#[test]
fn resize_plan_updates_duration_only() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a.id.clone()],
        },
    )
    .unwrap();
    let r = oxiline_core::plan::resize_plan(&c, &p.id, 120).unwrap();
    assert_eq!(r.duration_minute, 120);
    assert_eq!(r.start_minute, 9 * 60); // unchanged
    assert_eq!(r.weekday_mask, 0b0000001); // unchanged
    // options preserved + slot reflects new duration
    let s = oxiline_core::plan::slots_for_date(&c, "2026-08-03")
        .unwrap()
        .into_iter()
        .find(|s| s.plan_id == p.id)
        .unwrap();
    assert_eq!(s.duration_minute, 120);
    assert_eq!(s.options.len(), 1);
}

#[test]
fn resize_plan_rejects_zero_and_missing() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a.id.clone()],
        },
    )
    .unwrap();
    assert!(oxiline_core::plan::resize_plan(&c, &p.id, 0).is_err());
    assert!(oxiline_core::plan::resize_plan(&c, "nope", 30).is_err());
}
