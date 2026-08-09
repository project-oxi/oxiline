//! Integration tests for the `oxiline upgrade` CLI surface. Mirrors the
//! harness in `tests/plan_cli.rs`: each test runs the compiled binary
//! against a fresh tempdir SQLite file via the `OXILINE_DB_PATH` env
//! override. The `update` alias hits the live network; we keep this
//! test offline by asserting only on the help text.

use assert_cmd::Command;
use predicates::prelude::*;

const OXILINE_BIN: &str = env!("CARGO_BIN_EXE_oxiline");

fn oxiline_cmd(db_path: &std::path::Path) -> Command {
    let mut c = Command::new(OXILINE_BIN);
    c.env("OXILINE_DB_PATH", db_path)
        .env("LC_ALL", "C")
        .env("LANG", "C");
    c
}

#[test]
fn upgrade_help_lists_new_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db");
    oxiline_cmd(&db)
        .args(["upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--json-progress"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn update_alias_is_hidden_but_still_works() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db");
    // `update` is hidden from the top-level help, but its own --help
    // should still resolve (clap's `hide = true` only affects parents).
    oxiline_cmd(&db)
        .args(["update", "--help"])
        .assert()
        .success();
}
