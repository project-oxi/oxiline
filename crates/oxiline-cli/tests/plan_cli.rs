//! Integration tests for the `oxiline plan` CLI group (Task 10).
//!
//! Mirrors the `tests/record_cli.rs` harness: each test runs the compiled
//! binary against a fresh tempdir SQLite file via the `OXILINE_DB_PATH` env
//! override, asserting on the `--json` payload shape.

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

/// `plan add --options A,B` then `plan list --date <Monday> --json` shows a
/// slot whose options carry both activities (the OR choice-set).
#[test]
fn plan_add_materializes_slot_with_options() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    for name in ["코딩", "독서"] {
        oxiline_cmd(&db)
            .args(["activity", "add", name, "--json"])
            .assert()
            .success();
    }

    // Recurring Monday plan (bit 0), 11:00–13:00, 코딩 OR 독서.
    let add = oxiline_cmd(&db)
        .args([
            "plan",
            "add",
            "--at",
            "11:00",
            "--duration",
            "120",
            "--days",
            "mon",
            "--options",
            "코딩,독서",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        add.status.success(),
        "plan add failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let plan: Value = serde_json::from_slice(&add.stdout).unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();
    assert_eq!(plan["start_minute"].as_u64(), Some(660));
    assert_eq!(plan["duration_minute"].as_u64(), Some(120));
    assert_eq!(plan["weekday_mask"].as_u64(), Some(1)); // Monday = bit 0

    // Materialize for a Monday (2026-08-03).
    let list = oxiline_cmd(&db)
        .args(["plan", "list", "--date", "2026-08-03", "--json"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let slots = serde_json::from_slice::<Value>(&list.stdout)
        .unwrap()
        .as_array()
        .cloned()
        .unwrap();
    let ours = slots
        .iter()
        .find(|s| s["plan_id"].as_str() == Some(&plan_id))
        .expect("added plan must materialize for its weekday");
    let opts = ours["options"].as_array().unwrap();
    assert_eq!(opts.len(), 2, "slot must carry both OR options");
    let _keep_alive: TempDir = tmp;
}

/// `plan edit --title X` must NOT wipe start_minute/duration/weekday_mask —
/// `plan::update_plan` assigns those fields directly from `PlanInput`, so the
/// handler refills them from the current plan. This test pins that contract.
#[test]
fn plan_edit_preserves_time_duration_days() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--json"])
        .assert()
        .success();
    let add = oxiline_cmd(&db)
        .args([
            "plan",
            "add",
            "--at",
            "9:00",
            "--duration",
            "90",
            "--days",
            "wed",
            "--options",
            "코딩",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let plan: Value = serde_json::from_slice(&add.stdout).unwrap();
    let id = plan["id"].as_str().unwrap().to_string();
    assert_eq!(plan["weekday_mask"].as_u64(), Some(4)); // Wednesday = bit 2

    // Edit ONLY the title.
    let edit = oxiline_cmd(&db)
        .args(["plan", "edit", &id, "--title", "새 제목", "--json"])
        .output()
        .unwrap();
    assert!(
        edit.status.success(),
        "{}",
        String::from_utf8_lossy(&edit.stderr)
    );
    let updated: Value = serde_json::from_slice(&edit.stdout).unwrap();
    assert_eq!(
        updated["start_minute"].as_u64(),
        Some(540),
        "start_minute must survive a title-only edit"
    );
    assert_eq!(
        updated["duration_minute"].as_u64(),
        Some(90),
        "duration must survive a title-only edit"
    );
    assert_eq!(
        updated["weekday_mask"].as_u64(),
        Some(4),
        "weekday_mask must survive a title-only edit"
    );
    assert_eq!(updated["title"].as_str(), Some("새 제목"));
    let _keep_alive: TempDir = tmp;
}

/// `plan rm` removes the plan (and cascades its options).
#[test]
fn plan_rm_removes_plan() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--json"])
        .assert()
        .success();
    let add = oxiline_cmd(&db)
        .args([
            "plan",
            "add",
            "--days",
            "daily",
            "--options",
            "코딩",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(add.status.success());
    let plan: Value = serde_json::from_slice(&add.stdout).unwrap();
    let id = plan["id"].as_str().unwrap().to_string();

    oxiline_cmd(&db)
        .args(["plan", "rm", &id, "--json"])
        .assert()
        .success();

    let list = oxiline_cmd(&db)
        .args(["plan", "list", "--json"])
        .output()
        .unwrap();
    let plans = serde_json::from_slice::<Value>(&list.stdout)
        .unwrap()
        .as_array()
        .cloned()
        .unwrap();
    assert!(plans.is_empty(), "plan list must be empty after rm");
    let _keep_alive: TempDir = tmp;
}
