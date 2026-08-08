//! Integration tests for the `oxiline activity` CLI group (Task 8).
//!
//! Each test runs the compiled binary against a freshly-migrated, isolated
//! SQLite file via the `OXILINE_DB_PATH` env override. `--json` is preferred
//! so assertions check the structured payload directly. Tests are run in
//! independent processes (one per `#[test]`), so each must keep its own
//! tempdir alive (its `Drop` impl removes the directory).
//!
//! This file doubles as the harness pattern that Tasks 9-11 will reuse.

use assert_cmd::Command;
use tempfile::{TempDir, tempdir};

/// Bin path to the `oxiline` binary built by `cargo test --tests`.
const OXILINE_BIN: &str = env!("CARGO_BIN_EXE_oxiline");

/// Build a fresh `Command` against `db_path`. Tests that need multiple
/// invocations against the same DB call this once per `output()` call —
/// `assert_cmd::Command::args` appends, so reusing a `Command` across calls
/// would accumulate arguments.
fn oxiline_cmd(db_path: &std::path::Path) -> Command {
    let mut c = Command::new(OXILINE_BIN);
    c.env("OXILINE_DB_PATH", db_path)
        .env("LC_ALL", "C")
        .env("LANG", "C");
    c
}

#[test]
fn activity_add_exit_zero_and_json() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    let out = oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--daily", "240", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = json
        .get("id")
        .and_then(|v| v.as_str())
        .expect("JSON must contain string `id`");
    assert!(!id.is_empty(), "id must be a non-empty string");
    assert_eq!(json.get("name").and_then(|v| v.as_str()), Some("코딩"));
    assert_eq!(
        json.get("target_minutes_daily").and_then(|v| v.as_u64()),
        Some(240)
    );
    assert_eq!(json.get("is_active").and_then(|v| v.as_bool()), Some(true));
    let _keep_alive: TempDir = tmp;
}

#[test]
fn activity_list_after_add() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    for (name, daily) in [("코딩", "240"), ("독서", "30")] {
        oxiline_cmd(&db)
            .args(["activity", "add", name, "--daily", daily, "--json"])
            .assert()
            .success();
    }
    let out = oxiline_cmd(&db)
        .args(["activity", "list", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = parsed.as_array().expect("list must be a JSON array");
    assert!(
        arr.len() >= 2,
        "expected at least 2 activities, got {}",
        arr.len()
    );
    assert!(
        arr.iter()
            .any(|a| a.get("name").and_then(|v| v.as_str()) == Some("코딩"))
    );
    assert!(
        arr.iter()
            .any(|a| a.get("name").and_then(|v| v.as_str()) == Some("독서"))
    );
    let _keep_alive: TempDir = tmp;
}

#[test]
fn activity_edit_clear_daily_via_zero() {
    // `--daily 0` must map to `Some(None)` in ActivityInput, which
    // `update_activity` interprets as "clear to NULL" (Task 3
    // `update_activity_target_tri_state` pins this behavior).
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");

    let add_out = oxiline_cmd(&db)
        .args(["activity", "add", "코딩", "--daily", "240", "--json"])
        .output()
        .unwrap();
    assert!(add_out.status.success());
    let created: serde_json::Value = serde_json::from_slice(&add_out.stdout).unwrap();
    let id = created
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    let edit_out = oxiline_cmd(&db)
        .args(["activity", "edit", &id, "--daily", "0", "--json"])
        .output()
        .unwrap();
    assert!(
        edit_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&edit_out.stderr)
    );
    let updated: serde_json::Value = serde_json::from_slice(&edit_out.stdout).unwrap();
    assert!(
        updated.get("target_minutes_daily").is_some(),
        "edit result must include target_minutes_daily field"
    );
    assert!(
        updated.get("target_minutes_daily").unwrap().is_null(),
        "expected target_minutes_daily cleared to null, got: {}",
        updated.get("target_minutes_daily").unwrap()
    );
    let _keep_alive: TempDir = tmp;
}

#[test]
fn activity_rm_force_required_when_records_exist() {
    use oxiline_core::{activities, model::ActivityInput, open_and_migrate, settings};

    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("rm.db");

    // Seed the activity + a record directly via the core API. The schema's
    // ON DELETE RESTRICT on records.activity_id is what delete_activity(force=false)
    // must catch.
    let conn = open_and_migrate(&db_path).unwrap();
    settings::ensure_defaults(&conn).unwrap();
    let a = activities::create_activity(
        &conn,
        ActivityInput {
            name: Some("코딩".into()),
            ..ActivityInput::default()
        },
    )
    .unwrap();
    conn.execute(
        "INSERT INTO records (id, activity_id, started_at, ended_at, note, created_at, updated_at) \
         VALUES (?1, ?2, ?3, NULL, NULL, ?3, ?3)",
        rusqlite::params!["seed-record-id", a.id, "2026-08-01T09:00:00Z"],
    )
    .unwrap();
    drop(conn);

    let out = oxiline_cmd(&db_path)
        .args(["activity", "rm", &a.id, "--json"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "rm without --force must fail (got exit 0)"
    );
    let stderr_json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap_or_else(|_| {
        panic!(
            "stderr must be JSON with --json flag. got: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(
        stderr_json
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str()),
        Some("conflict"),
        "expected conflict code, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out2 = oxiline_cmd(&db_path)
        .args(["activity", "rm", &a.id, "--force", "--json"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "rm --force must succeed; stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&out2.stdout).expect("rm --force --json must emit JSON");
    assert_eq!(stdout.get("removed").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        stdout.get("id").and_then(|v| v.as_str()),
        Some(a.id.as_str())
    );
    let _keep_alive: TempDir = tmp;
}
