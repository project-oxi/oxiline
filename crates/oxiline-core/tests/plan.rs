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
        oxiline_core::model::ActivityInput {
            name: Some(name.into()),
            ..Default::default()
        },
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

fn sort_orders(c: &Connection, plan_id: &str) -> Vec<i32> {
    c.prepare("SELECT sort_order FROM plan_options WHERE plan_id = ?1 ORDER BY sort_order")
        .unwrap()
        .query_map(rusqlite::params![plan_id], |r| r.get::<_, i32>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

#[test]
fn add_options_monotonic_unique_and_dedups_existing() {
    let (_f, c) = db();
    let a1 = mk_activity(&c, "a1");
    let a2 = mk_activity(&c, "a2");
    let a3 = mk_activity(&c, "a3");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a1.id.clone()],
        },
    )
    .unwrap(); // a1 = order 0
    let out =
        oxiline_core::plan::add_options(&c, &p.id, &[a2.id.clone(), a3.id.clone(), a1.id.clone()])
            .unwrap();
    // (a) return: input order, existing-or-new, one row per unique input
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].activity_id, a2.id);
    assert_eq!(out[0].sort_order, 1);
    assert_eq!(out[1].activity_id, a3.id);
    assert_eq!(out[1].sort_order, 2);
    assert_eq!(out[2].activity_id, a1.id);
    assert_eq!(out[2].sort_order, 0);
    // (b) DB set: monotonic, unique, a1 single row
    assert_eq!(sort_orders(&c, &p.id), vec![0, 1, 2]);
}

#[test]
fn add_options_dedups_within_input() {
    let (_f, c) = db();
    let a4 = mk_activity(&c, "a4");
    let a5 = mk_activity(&c, "a5");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a4.id.clone()],
        },
    )
    .unwrap(); // a4 = order 0
    let out =
        oxiline_core::plan::add_options(&c, &p.id, &[a4.id.clone(), a4.id.clone(), a5.id.clone()])
            .unwrap();
    assert_eq!(out.len(), 2); // within-input dup collapsed
    assert_eq!(out[0].activity_id, a4.id); // existing
    assert_eq!(out[0].sort_order, 0);
    assert_eq!(out[1].activity_id, a5.id); // new
    assert_eq!(out[1].sort_order, 1);
    assert_eq!(sort_orders(&c, &p.id), vec![0, 1]);
}

#[test]
fn add_options_empty_is_noop() {
    let (_f, c) = db();
    let a = mk_activity(&c, "a");
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
    let out = oxiline_core::plan::add_options(&c, &p.id, &[]).unwrap();
    assert!(out.is_empty());
    assert_eq!(sort_orders(&c, &p.id), vec![0]); // unchanged
}

#[test]
fn add_options_missing_plan_is_not_found() {
    let (_f, c) = db();
    let a = mk_activity(&c, "a");
    let err = oxiline_core::plan::add_options(&c, "nope", &[a.id]).unwrap_err();
    assert!(matches!(err, oxiline_core::CoreError::NotFound(_)));
}

#[test]
fn add_options_concurrent_unique_sort_order() {
    use std::sync::Arc;
    use std::thread;
    // Multiple pooled-style connections hammer add_options on the SAME plan in
    // parallel. Under BEGIN IMMEDIATE the write lock is held during the MAX
    // read → every sort_order globally unique, zero errors (busy_timeout waits).
    // A DEFERRED txn would let two connections read the same MAX and insert
    // duplicate sort_orders (or hit SQLITE_BUSY_SNAPSHOT) — fails under DEFERRED.
    let f = NamedTempFile::new().unwrap();
    let setup = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&setup).unwrap();
    let a0 = mk_activity(&setup, "seed");
    let p = oxiline_core::plan::create_plan(
        &setup,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a0.id.clone()],
        },
    )
    .unwrap();

    const THREADS: usize = 4;
    const PER_THREAD: usize = 25;
    let mut buckets: Vec<Vec<String>> = Vec::with_capacity(THREADS);
    for t in 0..THREADS {
        let mut bucket = Vec::with_capacity(PER_THREAD);
        for k in 0..PER_THREAD {
            bucket.push(mk_activity(&setup, &format!("t{t}-k{k}")).id.clone());
        }
        buckets.push(bucket);
    }
    // One connection per thread (mirrors the r2d2 pool; busy_timeout/WAL set per conn).
    let conns: Vec<Connection> = (0..THREADS)
        .map(|_| oxiline_core::open_and_migrate(f.path()).unwrap())
        .collect();
    let plan_id = Arc::new(p.id.clone());

    let handles: Vec<_> = conns
        .into_iter()
        .zip(buckets)
        .map(|(conn, bucket)| {
            let plan_id = Arc::clone(&plan_id);
            thread::spawn(move || -> Result<(), String> {
                for aid in bucket {
                    oxiline_core::plan::add_options(&conn, &plan_id, std::slice::from_ref(&aid))
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            })
        })
        .collect();

    for h in handles {
        h.join()
            .unwrap()
            .expect("add_options errored under concurrency");
    }

    let orders = sort_orders(&setup, &p.id);
    assert_eq!(orders.len(), 1 + THREADS * PER_THREAD, "row count mismatch");
    let mut seen = std::collections::HashSet::new();
    for &o in &orders {
        assert!(seen.insert(o), "duplicate sort_order {o}");
    }
}

fn today_str() -> String {
    oxiline_core::util::today_local()
}

fn one_shot_plan(c: &Connection, aid: &str, start: u16, dur: u16) -> oxiline_core::model::Plan {
    oxiline_core::plan::create_plan(
        c,
        oxiline_core::model::PlanInput {
            date: Some(today_str()),
            start_minute: start,
            duration_minute: dur,
            weekday_mask: 0,
            title: None,
            activity_ids: vec![aid.to_string()],
        },
    )
    .unwrap()
}

#[test]
fn now_summary_next_when_before_slot() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    one_shot_plan(&c, &a.id, 600, 60); // 10:00–11:00
    let s = oxiline_core::plan::now_summary(&c, 500).unwrap(); // 8:20, before slot
    assert!(s.current.is_none(), "no current before any slot");
    let next = s.next.expect("next slot");
    assert_eq!(next.title, "코딩");
    assert_eq!(next.starts_in_minute, Some(100)); // 600 - 500
}

#[test]
fn now_summary_current_slot_remaining() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    one_shot_plan(&c, &a.id, 600, 60); // 10:00–11:00
    let s = oxiline_core::plan::now_summary(&c, 610).unwrap(); // 10:10, within slot
    let cur = s.current.expect("current slot");
    assert_eq!(cur.title, "코딩");
    assert_eq!(cur.remaining_minute, Some(50)); // 660 - 610
    assert!(s.next.is_none(), "sole slot is current, no next");
}

#[test]
fn now_summary_active_record_priority_over_slot() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    one_shot_plan(&c, &a.id, 600, 60); // slot 코딩 at 10:00–11:00
    let b = mk_activity(&c, "독서");
    let _ = oxiline_core::record::start(&c, &b.id, chrono::Utc::now(), &today_str()).unwrap();
    let s = oxiline_core::plan::now_summary(&c, 610).unwrap(); // within slot, but recording
    let cur = s.current.expect("current");
    assert_eq!(cur.title, "독서"); // active record wins over the slot
    assert_eq!(cur.remaining_minute, None); // open-ended record
}

#[test]
fn now_summary_empty_when_no_plans_or_records() {
    let (_f, c) = db();
    let s = oxiline_core::plan::now_summary(&c, 500).unwrap();
    assert!(s.current.is_none());
    assert!(s.next.is_none());
}
