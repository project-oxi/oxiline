//! Integration tests for `oxiline report` (Task 11) — neutral activity
//! compliance. Mirrors the `tests/record_cli.rs` assert_cmd harness.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::{TempDir, tempdir};

const OXILINE_BIN: &str = env!("CARGO_BIN_EXE_oxiline");

fn oxiline_cmd(db_path: &std::path::Path) -> Command {
    let mut c = Command::new(OXILINE_BIN);
    c.env("OXILINE_DB_PATH", db_path)
        .env("LC_ALL", "C")
        .env("LANG", "C");
    c
}

/// `report --week --json` after recording time ⇒ an array whose rows carry a
/// numeric `ratio`, and crucially NO `is_done` key anywhere (completion is a
/// record existing, never a stored flag).
#[test]
fn report_week_json_has_ratio_no_is_done() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    // Activity with a weekly target so the compliance row is budgeted.
    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--weekly", "1200", "--json"])
        .assert()
        .success();

    // Record some time this week (open then close).
    oxiline_cmd(&db)
        .args(["record", "start", "코딩", "--json"])
        .assert()
        .success();
    oxiline_cmd(&db)
        .args(["record", "stop", "--json"])
        .assert()
        .success();

    let out = oxiline_cmd(&db)
        .args(["report", "--week", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "report failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let raw = String::from_utf8(out.stdout).unwrap();
    assert!(
        !raw.contains("is_done"),
        "report JSON must never contain is_done"
    );

    let arr = serde_json::from_str::<Value>(&raw)
        .unwrap()
        .as_array()
        .cloned()
        .unwrap();
    assert!(!arr.is_empty(), "report must list the activity");
    let cm = arr
        .iter()
        .find(|c| c["activity"]["name"].as_str() == Some("코딩"))
        .expect("코딩 must appear in compliance");
    assert!(
        cm.get("ratio").and_then(|v| v.as_f64()).is_some(),
        "compliance row must carry a numeric ratio"
    );
    assert!(
        cm.get("state").is_some(),
        "compliance row must carry a neutral state"
    );
    let _keep_alive: TempDir = tmp;
}

/// `report --range` is deferred (no arbitrary-window Scope variant in core
/// yet): it must fail loudly with a non-zero exit, not silently emit a legacy
/// completion report.
#[test]
fn report_range_is_deferred_nonzero_exit() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    let out = oxiline_cmd(&db)
        .args(["report", "--range", "2026-08-01:2026-08-07"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "--range must exit non-zero while deferred"
    );
    let _keep_alive: TempDir = tmp;
}
