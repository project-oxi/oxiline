//! Integration tests for the `oxiline record` CLI group (Task 9).
//!
//! Mirrors the `tests/activity.rs` harness: each test runs the compiled binary
//! against a fresh tempdir SQLite file via the `OXILINE_DB_PATH` env override.
//! Tests assert on the structured JSON payload (`--json`) so we read the same
//! shape the GUI would see through the tauri-specta bindings.
//!
//! `.assert().success()` is preferred for negative-path checks; `.output()` is
//! used when we need both stdout and stderr (e.g. to verify an error's
//! `code` matches the spec).

use assert_cmd::Command;
use tempfile::{TempDir, tempdir};

/// Bin path to the `oxiline` binary built by `cargo test --tests`.
const OXILINE_BIN: &str = env!("CARGO_BIN_EXE_oxiline");

/// Build a fresh `Command` against `db_path`. Each invocation must construct a
/// new `Command` (args accumulate on `assert_cmd::Command`, so reusing one
/// across calls would taint later invocations).
fn oxiline_cmd(db_path: &std::path::Path) -> Command {
    let mut c = Command::new(OXILINE_BIN);
    c.env("OXILINE_DB_PATH", db_path)
        .env("LC_ALL", "C")
        .env("LANG", "C");
    c
}

#[test]
fn record_start_then_bare_returns_active() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--json"])
        .assert()
        .success();

    let out = oxiline_cmd(&db)
        .args(["record", "start", "코딩", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "record start must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let state: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let active_name = state
        .get("active")
        .and_then(|a| a.get("activity"))
        .and_then(|a| a.get("name"))
        .and_then(|v| v.as_str());
    assert_eq!(
        active_name,
        Some("코딩"),
        "expected active activity '코딩', got: {}",
        state
    );

    // Bare `record` should also surface the active session.
    let bare_out = oxiline_cmd(&db)
        .args(["record", "--json"])
        .output()
        .unwrap();
    assert!(
        bare_out.status.success(),
        "record (bare) must succeed; stderr: {}",
        String::from_utf8_lossy(&bare_out.stderr)
    );
    let bare: serde_json::Value = serde_json::from_slice(&bare_out.stdout).unwrap();
    assert_eq!(
        bare.get("active")
            .and_then(|a| a.get("activity"))
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str()),
        Some("코딩")
    );
    let _keep_alive: TempDir = tmp;
}

#[test]
fn record_at_backdates_switch() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--json"])
        .assert()
        .success();
    oxiline_cmd(&db)
        .args(["activity", "add", "독서", "--json"])
        .assert()
        .success();

    // Backdate the first record to T-90s and the second to T (clean switch).
    // The earlier --at must precede the second --at so the prior record's
    // `ended_at` (== second --at) is strictly greater than its `started_at`.
    let first_at = "2026-08-01T09:00:00Z";
    let switch_at = "2026-08-01T09:01:30Z";
    oxiline_cmd(&db)
        .args(["record", "start", "코딩", "--at", first_at, "--json"])
        .assert()
        .success();

    let out = oxiline_cmd(&db)
        .args(["record", "start", "독서", "--at", switch_at, "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "record start --at must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Inspect both records via `record log --range`. The switch should expose
    // two records: the initial 코딩 (ended at `switch_at`) and the new
    // 독서 (started at `switch_at`).
    let log_out = oxiline_cmd(&db)
        .args(["record", "log", "--range", "2026-08-01:2026-08-01", "--json"])
        .output()
        .unwrap();
    assert!(log_out.status.success());
    let records: serde_json::Value = serde_json::from_slice(&log_out.stdout).unwrap();
    let arr = records.as_array().expect("record log must be a JSON array");
    assert_eq!(arr.len(), 2, "expected 2 records, got: {}", records);

    // First record (코딩) ends at `switch_at`.
    let first = &arr[0];
    assert_eq!(
        first.get("ended_at").and_then(|v| v.as_str()),
        Some(switch_at),
        "first record's ended_at must match --at; got: {}",
        first
    );

    // Second record (독서) starts at `switch_at`.
    let second = &arr[1];
    assert_eq!(
        second.get("started_at").and_then(|v| v.as_str()),
        Some(switch_at),
        "second record's started_at must match --at; got: {}",
        second
    );
    let _keep_alive: TempDir = tmp;
}

#[test]
fn record_log_with_date_range() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--json"])
        .assert()
        .success();

    oxiline_cmd(&db)
        .args(["record", "start", "코딩", "--json"])
        .assert()
        .success();

    // Use today's local date to find the record we just made.
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let out = oxiline_cmd(&db)
        .args(["record", "log", "--date", &today, "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "record log --date must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let records: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = records.as_array().expect("record log must be a JSON array");
    assert_eq!(
        arr.len(),
        1,
        "expected exactly the record we just started, got: {}",
        records
    );
    assert!(
        arr[0].get("activity_id").and_then(|v| v.as_str()).is_some(),
        "activity_id must be present and non-null"
    );
    let _keep_alive: TempDir = tmp;
}

#[test]
fn record_stop_closes_open() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--json"])
        .assert()
        .success();
    oxiline_cmd(&db)
        .args(["record", "start", "코딩", "--json"])
        .assert()
        .success();

    let out = oxiline_cmd(&db)
        .args(["record", "stop", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "record stop must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bare_out = oxiline_cmd(&db)
        .args(["record", "--json"])
        .output()
        .unwrap();
    assert!(bare_out.status.success());
    let bare: serde_json::Value = serde_json::from_slice(&bare_out.stdout).unwrap();
    assert!(
        bare.get("active").is_none() || bare.get("active") == Some(&serde_json::Value::Null),
        "after `record stop`, active must be null; got: {}",
        bare
    );
    let _keep_alive: TempDir = tmp;
}
