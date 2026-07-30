//! Integration tests for oxiline-core against an ephemeral SQLite DB.
//! Covers the Phase 0 acceptance criteria (08-roadmap.md): weekday-mask
//! boundaries, effective periods, materialize de-dup, merge behaviour.

use oxiline_core::model::{RoutineBlock, TaskSource};
use oxiline_core::{categories, routines, tasks, timeline};
use oxiline_core::settings;
use oxiline_core::util;
use chrono::Datelike;
use tempfile::NamedTempFile;

fn fresh_db() -> (NamedTempFile, rusqlite::Connection) {
    let f = NamedTempFile::new().unwrap();
    let conn = oxiline_core::open_and_migrate(f.path()).unwrap();
    settings::ensure_defaults(&conn).unwrap();
    (f, conn)
}

/// Monday 2026-07-27 (a known Monday) for weekday-mask tests.
const MON: &str = "2026-07-27";
/// Tuesday 2026-07-28.
const TUE: &str = "2026-07-28";
/// Sunday 2026-08-02.
const SUN: &str = "2026-08-02";

fn add_routine(conn: &rusqlite::Connection, mask: u8) -> RoutineBlock {
    routines::create(
        conn,
        routines::NewRoutineBlock {
            title: "Block".into(),
            start_minute: 540, // 09:00
            duration_minute: 30,
            weekday_mask: mask,
            category_id: Some("cat_work".into()),
            effective_from: None,
            effective_until: None,
            notes: None,
        },
    )
    .unwrap()
}

#[test]
fn weekday_mask_includes_only_selected_days() {
    assert_eq!(routines::parse_days_spec("daily").unwrap(), 0b1111111);
    assert_eq!(routines::parse_days_spec("weekdays").unwrap(), 0b0001111);
    assert_eq!(routines::parse_days_spec("weekends").unwrap(), 0b1100000);
    assert_eq!(routines::parse_days_spec("mon,wed,fri").unwrap(), 0b0010101);
    assert_eq!(routines::parse_days_spec("tue,thu").unwrap(), 0b0001010);
}

#[test]
fn timeline_emits_virtual_occurrence_for_matching_weekday() {
    let (_f, conn) = fresh_db();
    add_routine(&conn, 0b0001111); // weekdays only

    let mon_items = timeline::get_timeline_for_date(&conn, MON).unwrap();
    assert_eq!(mon_items.len(), 1);
    assert!(mon_items[0].is_virtual);
    assert!(mon_items[0].id.starts_with("virtual:"));
    assert_eq!(mon_items[0].start_minute, Some(540));

    // Sunday is not in the mask → no occurrence.
    let sun_items = timeline::get_timeline_for_date(&conn, SUN).unwrap();
    assert!(sun_items.is_empty(), "no occurrence expected on Sunday");
}

#[test]
fn materialize_is_idempotent_and_unique() {
    let (_f, conn) = fresh_db();
    let block = add_routine(&conn, 0b1111111);

    let first = tasks::materialize_occurrence(&conn, &block.id, TUE).unwrap();
    let second = tasks::materialize_occurrence(&conn, &block.id, TUE).unwrap();
    assert_eq!(first.id, second.id, "materializing twice returns same row");
    assert_eq!(second.source, TaskSource::Routine);

    // Timeline now shows the materialized (non-virtual) row instead of virtual.
    let items = timeline::get_timeline_for_date(&conn, TUE).unwrap();
    assert_eq!(items.len(), 1);
    assert!(!items[0].is_virtual);
    assert_eq!(items[0].origin_routine_block_id, Some(block.id));
}

#[test]
fn materialize_if_virtual_resolves_virtual_id() {
    let (_f, conn) = fresh_db();
    let block = add_routine(&conn, 0b1111111);
    let vid = tasks::virtual_id(&block.id, TUE);

    let real_id = tasks::materialize_if_virtual(&conn, &vid).unwrap();
    assert!(!real_id.starts_with("virtual:"));
    // A second resolve returns the same real id.
    let again = tasks::materialize_if_virtual(&conn, &vid).unwrap();
    assert_eq!(real_id, again);
}

#[test]
fn done_on_virtual_id_materializes_then_marks() {
    let (_f, conn) = fresh_db();
    let block = add_routine(&conn, 0b1111111);
    let vid = tasks::virtual_id(&block.id, TUE);

    let real_id = tasks::materialize_if_virtual(&conn, &vid).unwrap();
    let done = tasks::set_done(&conn, &real_id, true).unwrap();
    assert!(done.is_done);

    // Materialized+done row is not virtual in the timeline.
    let items = timeline::get_timeline_for_date(&conn, TUE).unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].is_done);
    assert!(!items[0].is_virtual);
}

#[test]
fn skip_hides_occurrence_from_timeline() {
    let (_f, conn) = fresh_db();
    let block = add_routine(&conn, 0b1111111);
    let vid = tasks::virtual_id(&block.id, TUE);

    let real_id = tasks::materialize_if_virtual(&conn, &vid).unwrap();
    tasks::set_skipped(&conn, &real_id, true).unwrap();

    // Skipped occurrence must not appear, and no virtual replacement emitted.
    let items = timeline::get_timeline_for_date(&conn, TUE).unwrap();
    assert!(items.is_empty(), "skipped occurrence is hidden");
}

#[test]
fn effective_period_bounds_visibility() {
    let (_f, conn) = fresh_db();
    // A routine only effective 2026-07-28 .. 2026-07-30.
    routines::create(
        &conn,
        routines::NewRoutineBlock {
            title: "Windowed".into(),
            start_minute: 600,
            duration_minute: 15,
            weekday_mask: 0b1111111,
            category_id: None,
            effective_from: Some("2026-07-28".into()),
            effective_until: Some("2026-07-30".into()),
            notes: None,
        },
    )
    .unwrap();

    // Before window.
    assert!(timeline::get_timeline_for_date(&conn, "2026-07-27").unwrap().is_empty());
    // In window (2026-07-28 Tue).
    assert_eq!(timeline::get_timeline_for_date(&conn, "2026-07-28").unwrap().len(), 1);
    // After window.
    assert!(timeline::get_timeline_for_date(&conn, "2026-07-31").unwrap().is_empty());
}

#[test]
fn manual_task_and_virtual_merge_and_sort() {
    let (_f, conn) = fresh_db();
    add_routine(&conn, 0b1111111); // 09:00 virtual
    tasks::create(
        &conn,
        tasks::NewTask {
            date: Some(TUE.into()),
            title: "Manual at 08:00".into(),
            category_id: None,
            start_minute: Some(480),
            duration_minute: Some(20),
            notes: None,
        },
    )
    .unwrap();

    let items = timeline::get_timeline_for_date(&conn, TUE).unwrap();
    assert_eq!(items.len(), 2);
    // Sorted ascending by start minute: 480 then 540.
    assert_eq!(items[0].start_minute, Some(480));
    assert!(!items[0].is_virtual);
    assert_eq!(items[1].start_minute, Some(540));
    assert!(items[1].is_virtual);
}

#[test]
fn backlog_tasks_have_no_date() {
    let (_f, conn) = fresh_db();
    tasks::create(
        &conn,
        tasks::NewTask {
            date: None,
            title: "장보기".into(),
            category_id: None,
            start_minute: None,
            duration_minute: None,
            notes: None,
        },
    )
    .unwrap();
    let backlog = tasks::list_backlog(&conn).unwrap();
    assert_eq!(backlog.len(), 1);
    assert!(backlog[0].date.is_none());
}

#[test]
fn category_crud_roundtrip() {
    let (_f, conn) = fresh_db();
    let seeded = categories::list(&conn).unwrap();
    assert_eq!(seeded.len(), 6, "six builtin categories seeded");

    let c = categories::create(
        &conn,
        categories::NewCategory {
            name: "사이드".into(),
            color_hue: 200.0,
            icon: Some("rocket".into()),
        },
    )
    .unwrap();
    // Resolve by name.
    let resolved = categories::resolve(&conn, "사이드").unwrap();
    assert_eq!(resolved.id, c.id);
    // Resolve by id.
    assert_eq!(categories::resolve(&conn, &c.id).unwrap().id, c.id);
    categories::delete(&conn, &c.id).unwrap();
    assert!(categories::get(&conn, &c.id).is_err());
}

#[test]
fn schema_version_is_one_after_migrate() {
    let (_f, conn) = fresh_db();
    assert_eq!(oxiline_core::db::schema_version(&conn).unwrap(), 1);
}

#[test]
fn monday_weekday_bit_matches() {
    // Sanity: util helper agrees with chrono weekday for MON date.
    let d = util::parse_date(MON).unwrap();
    assert_eq!(util::weekday_mask_bit(d.weekday()), 0);
}
