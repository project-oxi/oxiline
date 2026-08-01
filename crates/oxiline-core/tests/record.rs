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
