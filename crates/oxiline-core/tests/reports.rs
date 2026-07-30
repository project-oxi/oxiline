//! Integration tests for oxiline-core reports (habit streak / weekly report).
//! Mirrors tests/timeline.rs setup. created_at back-dating is done via a raw
//! UPDATE here ONLY — never touches tests/timeline.rs (spec §2.1 scope note).

use oxiline_core::model::{DayBreakdown, RoutineStreak, WeekReport};

#[test]
fn report_types_serialize_to_snake_case() {
    let s = RoutineStreak {
        routine_id: "r1".into(),
        title: "아침 운동".into(),
        current: 12,
        last_done_date: Some("2026-07-29".into()),
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"routine_id\""));
    assert!(json.contains("\"last_done_date\""));

    let _d: DayBreakdown = serde_json::from_str(
        r#"{"date":"2026-07-30","done":0,"skipped":0,"not_recorded":0,"upcoming":0,
            "completion_rate":null,"categories":[]}"#,
    )
    .unwrap();
    let _: WeekReport = serde_json::from_str(
        r#"{"week_start":"2026-07-28","week_end":"2026-08-03","days":[],"totals":
            {"done":0,"skipped":0,"not_recorded":0,"upcoming":0},"completion_rate":null,
            "prev_completion_rate":null,"categories":[],"streaks":[]}"#,
    )
    .unwrap();
}

use chrono::Datelike;
use oxiline_core::reports;
use oxiline_core::tasks;
use oxiline_core::{routines, settings};
use rusqlite::params;
use tempfile::NamedTempFile;

fn fresh_db() -> (NamedTempFile, rusqlite::Connection) {
    let f = NamedTempFile::new().unwrap();
    let conn = oxiline_core::open_and_migrate(f.path()).unwrap();
    settings::ensure_defaults(&conn).unwrap();
    (f, conn)
}

/// Back-date a routine's created_at (tests/reports.rs ONLY — never touches
/// tests/timeline.rs). `ts` is an ISO-8601 UTC string.
fn backdate_created(conn: &rusqlite::Connection, id: &str, ts: &str) {
    conn.execute(
        "UPDATE routine_blocks SET created_at = ?1 WHERE id = ?2",
        params![ts, id],
    )
    .unwrap();
}

#[test]
fn scheduled_for_excludes_dates_before_created_at() {
    let (_f, conn) = fresh_db();
    // A daily routine, then back-date its creation to Wednesday 2026-07-29.
    let b = routines::create(
        &conn,
        routines::NewRoutineBlock {
            title: "X".into(),
            start_minute: 540,
            duration_minute: 30,
            weekday_mask: 0b1111111,
            category_id: None,
            effective_from: None,
            effective_until: None,
            notes: None,
        },
    )
    .unwrap();
    backdate_created(&conn, &b.id, "2026-07-29T08:00:00Z");

    let block = routines::get(&conn, &b.id).unwrap();
    // Monday/Tuesday are BEFORE created_at → not scheduled.
    assert!(!reports::scheduled_for(&block, "2026-07-27")); // Mon
    assert!(!reports::scheduled_for(&block, "2026-07-28")); // Tue
    // Wednesday onward → scheduled (weekday matches, in range).
    assert!(reports::scheduled_for(&block, "2026-07-29")); // Wed
    assert!(reports::scheduled_for(&block, "2026-08-02")); // Sun
}

#[test]
fn scheduled_for_respects_effective_from_weekday_and_active() {
    let (_f, conn) = fresh_db();
    // Mondays-only (bit0), effective from 2026-08-01.
    let b = routines::create(
        &conn,
        routines::NewRoutineBlock {
            title: "X".into(),
            start_minute: 540,
            duration_minute: 30,
            weekday_mask: 0b0000001,
            category_id: None,
            effective_from: Some("2026-08-01".into()),
            effective_until: None,
            notes: None,
        },
    )
    .unwrap();
    backdate_created(&conn, &b.id, "2026-01-01T00:00:00Z");
    let block = routines::get(&conn, &b.id).unwrap();
    assert!(!reports::scheduled_for(&block, "2026-07-27")); // Mon but before effective_from
    assert!(reports::scheduled_for(&block, "2026-08-03")); // Mon, in range
    assert!(!reports::scheduled_for(&block, "2026-08-04")); // Tue, wrong weekday
    let inactive = routines::set_active(&conn, &b.id, false).unwrap();
    assert!(!reports::scheduled_for(&inactive, "2026-08-03")); // inactive
}
