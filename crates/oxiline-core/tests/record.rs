//! Integration tests for the recording layer (V4 schema + domain types).
//!
//! Test harness (shared by Tasks 3–7): a `db()` helper that opens an ephemeral
//! SQLite file via `oxiline_core::open_and_migrate`, then runs `ensure_defaults`
//! so seeded settings are present. Mirrors `tests/timeline.rs:13-17` — `:memory:`
//! does not work with `open_and_migrate` (it takes a `&Path`).

use chrono::{TimeZone, Utc};
use rusqlite::Connection;
use tempfile::NamedTempFile;

use oxiline_core::settings;

fn db() -> (NamedTempFile, Connection) {
    let f = NamedTempFile::new().unwrap();
    let c = oxiline_core::open_and_migrate(f.path()).unwrap();
    settings::ensure_defaults(&c).unwrap();
    (f, c)
}

fn activity_input(name: &str) -> oxiline_core::model::ActivityInput {
    oxiline_core::model::ActivityInput {
        name: Some(name.into()),
        hue_label: None,
        icon: None,
        category_id: None,
        target_minutes_daily: None,
        target_minutes_weekly: None,
        is_active: None,
        sort_order: None,
    }
}

#[test]
fn v4_creates_record_tables() {
    let (_f, c) = db();
    let mut names: Vec<String> = c
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    names.sort();
    assert!(names.iter().any(|n| n == "activities"));
    assert!(names.iter().any(|n| n == "plans"));
    assert!(names.iter().any(|n| n == "plan_options"));
    assert!(names.iter().any(|n| n == "records"));

    // Sanity: the v4_creates_record_tables test only asserts the tables exist.
    // chrono::TimeZone + Utc are imported here so future tests in this file
    // (Tasks 5+) can call `Utc.with_ymd_and_hms(...)` without re-importing.
    let _ = Utc.with_ymd_and_hms(2026, 8, 3, 9, 0, 0);
}

#[test]
fn round_duration_snaps_half_up() {
    use oxiline_core::util::round_duration;
    assert_eq!(round_duration(42 * 60, 5), 40 * 60); // 2520s -> 2400s (nearest 5min, half-up)
    assert_eq!(round_duration(37 * 60 + 30, 5), 40 * 60); // 2250s -> 2400s (half-up at 37.5)
    assert_eq!(round_duration(42 * 60, 0), 42 * 60); // 0 disables
    assert_eq!(round_duration(0, 5), 0);
}

#[test]
fn start_switches_single_active() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(&c, activity_input("코딩")).unwrap();
    let b = oxiline_core::activities::create_activity(&c, activity_input("독서")).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 3, 9, 0, 0).unwrap();
    oxiline_core::record::start(&c, &a.id, now, "2026-08-03").unwrap();
    let switched_at = now + chrono::Duration::seconds(1);
    oxiline_core::record::start(&c, &b.id, switched_at, "2026-08-03").unwrap();
    let st = oxiline_core::record::current(&c, switched_at, "2026-08-03").unwrap();
    assert_eq!(st.active.as_ref().unwrap().activity.id, b.id);
    let open: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM records WHERE ended_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(open, 1);
}
