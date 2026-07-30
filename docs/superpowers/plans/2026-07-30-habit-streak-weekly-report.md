# Habit Streak / Weekly Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a descriptive (non-gamified) completion-reporting layer — weekly/range completion rates with a three-bucket breakdown (done/skipped/not-recorded) and per-routine current streaks — exposed via core, CLI, and a GUI Report tab.

**Architecture:** A new pure-Rust `oxiline-core::reports` module is the single source of truth for all completion/streak math (mirroring how `timeline.rs` owns timeline merging). CLI (`oxiline report`/`streak`) and GUI (Tauri commands + `ReportView.tsx`) are thin adapters. The `created_at` scheduled-bound fix lives in `reports.rs` only — `timeline.rs` is untouched (see spec §2.1 scope note).

**Tech Stack:** Rust (edition 2024), rusqlite, chrono, specta/serde; clap (CLI); Tauri v2 + tauri-specta + React 19 + Zustand + React Query (GUI).

## Global Constraints

- **Product constraint (inviolable):** descriptive, never judgmental. No "failed/broke/missed" copy, no fire/trophy/scoreboard, no green-success vs red-failure coloring. `not_recorded` is a neutral "체크인 없음" fact. (Spec §1.)
- **Completion rate** = `done / (done + not_recorded)`; `skipped` and `upcoming`/future excluded; denominator 0 → `None` (not 0%). (Spec §2.2–2.3.)
- **`created_at` bound** is reports-module-local only. Never modify `timeline.rs` or its tests. (Spec §2.1 scope note.)
- All new public structs derive `Serialize, Deserialize, Type, Clone, Debug` with `#[serde(rename_all = "snake_case")]` (specta → TS bindings, `03-data-model.md` §3.11).
- All report functions take `today`/`now_minute` as **parameters** (never read the wall clock) so tests are deterministic.
- Test fixture `created_at` back-dating uses a raw `UPDATE` in `tests/reports.rs` only (does not touch `tests/timeline.rs`).
- GUI color discipline: `not_recorded` uses neutral muted tokens (`--text-tertiary`/`--surface-sunken`/`--border-default`), **never** `signal-rust`. Done uses the category hue; overall bar uses `--accent-oxide`. (Spec §5.)
- Run tests with `cargo test -p oxiline-core`; build GUI with `cargo build -p oxiline-app`.

**Spec:** `docs/superpowers/specs/2026-07-30-habit-streak-weekly-report-design.md`

---

## File Structure

- **Create** `crates/oxiline-core/src/reports.rs` — scheduled-set reconstruction, 3-bucket classification, week/range aggregation, streak walk. The heart of the feature.
- **Modify** `crates/oxiline-core/src/model.rs` — add 6 report structs.
- **Modify** `crates/oxiline-core/src/lib.rs` — `pub mod reports;`
- **Create** `crates/oxiline-core/tests/reports.rs` — integration tests (mirrors `tests/timeline.rs`).
- **Modify** `crates/oxiline-cli/src/cli.rs` — add `Report` + `Streak` `Command` variants.
- **Modify** `crates/oxiline-cli/src/main.rs` — dispatch the two new variants.
- **Modify** `crates/oxiline-cli/src/output.rs` — `week_report_text` / `range_report_text` / `streak_list_text` renderers.
- **Modify** `crates/oxiline-cli/src/lang.rs` — copy strings for the new output.
- **Modify** `crates/oxiline-app/src-tauri/src/commands.rs` — 3 new Tauri commands.
- **Modify** `crates/oxiline-app/src-tauri/src/lib.rs` — register the 3 commands in `collect_commands!`.
- **Modify** `crates/oxiline-app/src/types.ts` — mirror the new Rust structs.
- **Modify** `crates/oxiline-app/src/lib/api.ts` — `getWeekReport`/`getRangeReport`/`getRoutineStreaks` wrappers.
- **Modify** `crates/oxiline-app/src/lib/store.ts` — add `"report"` to `View`.
- **Create** `crates/oxiline-app/src/components/ReportView.tsx` — the Report tab.
- **Modify** `crates/oxiline-app/src/App.tsx` — render the 4th tab + keyboard `4`.
- **Modify** `crates/oxiline-app/src/locales/{ko,en}.json` — Report tab copy.

---

### Task 1: Core report domain types

**Files:**
- Modify: `crates/oxiline-core/src/model.rs` (append after `SettingsSnapshot`, ~line 160)
- Test: `crates/oxiline-core/tests/reports.rs` (create)

**Interfaces:**
- Produces: `DayBreakdown`, `CategoryBreakdown`, `DayTotals`, `WeekReport`, `RangeReport`, `RoutineStreak` — consumed by every later core task and by the CLI/GUI tasks.

- [ ] **Step 1: Write the failing test (types serialize + the module compiles)**

Create `crates/oxiline-core/tests/reports.rs`:
```rust
use oxiline_core::model::{DayBreakdown, WeekReport, RoutineStreak};

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
    ).unwrap();
    let _: WeekReport = serde_json::from_str(
        r#"{"week_start":"2026-07-28","week_end":"2026-08-03","days":[],"totals":
            {"done":0,"skipped":0,"not_recorded":0,"upcoming":0},"completion_rate":null,
            "prev_completion_rate":null,"categories":[],"streaks":[]}"#,
    ).unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oxiline-core --test reports`
Expected: FAIL — `unresolved import oxiline_core::model::{DayBreakdown, ...}`.

- [ ] **Step 3: Add the structs to `model.rs`**

Append to `crates/oxiline-core/src/model.rs` (after the `SettingsSnapshot` block):
```rust
/// Per-day completion breakdown for reports (`reports::day_breakdown`).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct DayBreakdown {
    pub date: String,
    pub done: u32,
    pub skipped: u32,
    pub not_recorded: u32,
    pub upcoming: u32,
    pub completion_rate: Option<f64>,
    pub categories: Vec<CategoryBreakdown>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct CategoryBreakdown {
    pub category_id: Option<String>,
    /// Localized at the display layer when empty (no category).
    pub category_name: String,
    pub done: u32,
    pub skipped: u32,
    pub not_recorded: u32,
    pub completion_rate: Option<f64>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct DayTotals {
    pub done: u32,
    pub skipped: u32,
    pub not_recorded: u32,
    pub upcoming: u32,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct WeekReport {
    pub week_start: String,
    pub week_end: String,
    pub days: Vec<DayBreakdown>,
    pub totals: DayTotals,
    pub completion_rate: Option<f64>,
    pub prev_completion_rate: Option<f64>,
    pub categories: Vec<CategoryBreakdown>,
    pub streaks: Vec<RoutineStreak>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RangeReport {
    pub from: String,
    pub to: String,
    pub days: Vec<DayBreakdown>,
    pub totals: DayTotals,
    pub completion_rate: Option<f64>,
    pub categories: Vec<CategoryBreakdown>,
    pub streaks: Vec<RoutineStreak>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RoutineStreak {
    pub routine_id: String,
    pub title: String,
    pub current: u32,
    pub last_done_date: Option<String>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p oxiline-core --test reports`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/model.rs crates/oxiline-core/tests/reports.rs
git commit -m "feat(core): report domain types (DayBreakdown/WeekReport/RoutineStreak…)"
```

---

### Task 2: `reports` module skeleton + `scheduled_for` (the created_at bound)

**Files:**
- Create: `crates/oxiline-core/src/reports.rs`
- Modify: `crates/oxiline-core/src/lib.rs` (add `pub mod reports;`)
- Test: `crates/oxiline-core/tests/reports.rs` (append)

**Interfaces:**
- Produces: `reports::scheduled_for(block: &RoutineBlock, date: &str) -> bool` — the 4-condition scheduled predicate (spec §2.1). Consumed by `day_breakdown` (Task 3) and the streak walk (Task 5).
- Consumes: `model::RoutineBlock`, `util::{parse_date, weekday_mask_bit}`, `routines::mask_includes`.

- [ ] **Step 1: Write the failing test (the headline bug-fix guard)**

Append to `crates/oxiline-core/tests/reports.rs`:
```rust
use oxiline_core::model::RoutineBlock;
use oxiline_core::reports;
use oxiline_core::{routines, settings};
use chrono::Datelike;
use rusqlite::params;
use tempfile::NamedTempFile;

fn fresh_db() -> (NamedTempFile, rusqlite::Connection) {
    let f = NamedTempFile::new().unwrap();
    let conn = oxiline_core::open_and_migrate(f.path()).unwrap();
    settings::ensure_defaults(&conn).unwrap();
    (f, conn)
}

/// Back-date a routine's created_at (tests/reports.rs only — never touches
/// tests/timeline.rs). `ts` is an ISO-8601 UTC string.
fn backdate_created(conn: &rusqlite::Connection, id: &str, ts: &str) {
    conn.execute("UPDATE routine_blocks SET created_at = ?1 WHERE id = ?2", params![ts, id])
        .unwrap();
}

#[test]
fn scheduled_for_excludes_dates_before_created_at() {
    let (_f, conn) = fresh_db();
    // A daily routine, then back-date its creation to Wednesday 2026-07-29.
    let b = routines::create(conn, routines::NewRoutineBlock {
        title: "X".into(), start_minute: 540, duration_minute: 30,
        weekday_mask: 0b1111111, category_id: None,
        effective_from: None, effective_until: None, notes: None,
    }).unwrap();
    backdate_created(&conn, &b.id, "2026-07-29T08:00:00Z");

    let block = routines::get(&conn, &b.id).unwrap();
    // Monday/Tuesday are BEFORE created_at → not scheduled.
    assert!(!reports::scheduled_for(&block, "2026-07-27")); // Mon
    assert!(!reports::scheduled_for(&block, "2026-07-28")); // Tue
    // Wednesday onward → scheduled (weekday matches, in range).
    assert!(reports::scheduled_for(&block, "2026-07-29"));  // Wed
    assert!(reports::scheduled_for(&block, "2026-08-02"));  // Sun
}

#[test]
fn scheduled_for_respects_effective_from_and_weekday_and_active() {
    let (_f, conn) = fresh_db();
    // Mondays-only (bit0), effective from 2026-08-01.
    let b = routines::create(conn, routines::NewRoutineBlock {
        title: "X".into(), start_minute: 540, duration_minute: 30,
        weekday_mask: 0b0000001, category_id: None,
        effective_from: Some("2026-08-01".into()), effective_until: None, notes: None,
    }).unwrap();
    backdate_created(&conn, &b.id, "2026-01-01T00:00:00Z");
    let block = routines::get(&conn, &b.id).unwrap();
    assert!(!reports::scheduled_for(&block, "2026-07-27")); // Mon but before effective_from
    assert!(reports::scheduled_for(&block, "2026-08-03"));  // Mon, in range
    assert!(!reports::scheduled_for(&block, "2026-08-04")); // Tue, wrong weekday
    let b = routines::set_active(&conn, &b.id, false).unwrap();
    assert!(!reports::scheduled_for(&b, "2026-08-03"));     // inactive
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oxiline-core --test reports`
Expected: FAIL — `unresolved module reports` / `cannot find function scheduled_for`.

- [ ] **Step 3: Create `reports.rs` with `scheduled_for`**

Create `crates/oxiline-core/src/reports.rs`:
```rust
//! Completion & streak reporting — the single source of truth for all
//! completion/streak arithmetic (design spec §3). Reports-module-local: the
//! `created_at` scheduled bound (§2.1) lives here and is NOT applied to
//! `timeline.rs` (see spec §2.1 scope note).

use crate::error::Result;
use crate::model::RoutineBlock;
use crate::util;
use chrono::Datelike;

/// Is routine block `b` scheduled on `date` (YYYY-MM-DD)?
///
/// Four conditions (spec §2.1): active, weekday matches, within effective
/// range, and `date >= max(effective_from, created_at)`. This bound prevents
/// phantom pre-existence occurrences in past-looking reports.
pub fn scheduled_for(block: &RoutineBlock, date: &str) -> bool {
    if !block.is_active {
        return false;
    }
    let d = match util::parse_date(date) {
        Ok(d) => d,
        Err(_) => return false,
    };
    if !crate::routines::mask_includes(block.weekday_mask, d.weekday()) {
        return false;
    }
    if !in_effective_range(&block.effective_from, &block.effective_until, date) {
        return false;
    }
    date >= bound_date(block).as_str()
}

/// `max(effective_from_date, created_at_date)` as a YYYY-MM-DD string.
fn bound_date(block: &RoutineBlock) -> String {
    let created_day = block.created_at.get(..10).unwrap_or(&block.created_at).to_string();
    match &block.effective_from {
        Some(f) if f.as_str() > created_day.as_str() => f.clone(),
        _ => created_day,
    }
}

fn in_effective_range(from: &Option<String>, until: &Option<String>, date: &str) -> bool {
    if let Some(f) = from {
        if date < f.as_str() {
            return false;
        }
    }
    if let Some(u) = until {
        if date > u.as_str() {
            return false;
        }
    }
    true
}
```

Add to `crates/oxiline-core/src/lib.rs` (after `pub mod paths;`):
```rust
pub mod reports;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p oxiline-core --test reports`
Expected: PASS (both `scheduled_for_*` tests + the Task 1 serialize test).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/reports.rs crates/oxiline-core/src/lib.rs crates/oxiline-core/tests/reports.rs
git commit -m "feat(core): reports module + scheduled_for with created_at bound"
```

---

### Task 3: `day_breakdown` — three-bucket classification + temporal boundary

**Files:**
- Modify: `crates/oxiline-core/src/reports.rs`
- Test: `crates/oxiline-core/tests/reports.rs` (append)

**Interfaces:**
- Produces: `reports::day_breakdown(conn, date, today, now_minute) -> Result<DayBreakdown>`. (Signature refines spec §3.2: `today` added so the temporal boundary in §2.3 is computable.)
- Consumes: `scheduled_for` (Task 2), `tasks::list_by_date`, `routines::list`, `categories::list`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/oxiline-core/tests/reports.rs`:
```rust
use oxiline_core::reports;
use oxiline_core::tasks;

fn add_dated_task(conn: &rusqlite::Connection, date: &str, title: &str, start: Option<u16>) -> oxiline_core::model::Task {
    tasks::create(conn, oxiline_core::tasks::NewTask {
        date: Some(date.into()), title: title.into(), category_id: None,
        start_minute: start, duration_minute: Some(30), notes: None,
    }).unwrap()
}

#[test]
fn day_breakdown_classifies_three_buckets_and_excludes_skipped_from_rate() {
    let (_f, conn) = fresh_db();
    let past = "2026-07-28"; // Tue (any past day works; today is fixed below)
    let today = "2026-07-30";
    // done, skipped, and an untouched virtual routine occurrence (not_recorded).
    let d = add_dated_task(&conn, past, "done one", Some(540));
    tasks::set_done(&conn, &d.id, true).unwrap();
    let s = add_dated_task(&conn, past, "skipped one", Some(600));
    tasks::set_skipped(&conn, &s.id, true).unwrap();
    let b = routines::create(&conn, routines::NewRoutineBlock {
        title: "virt".into(), start_minute: 700, duration_minute: 30,
        weekday_mask: 0b1111111, category_id: None,
        effective_from: None, effective_until: None, notes: None,
    }).unwrap();
    backdate_created(&conn, &b.id, "2026-01-01T00:00:00Z");

    let bd = reports::day_breakdown(&conn, past, today, 800).unwrap();
    assert_eq!((bd.done, bd.skipped, bd.not_recorded, bd.upcoming), (1, 1, 1, 0));
    // rate = done/(done+not_recorded) = 1/2; skipped excluded.
    assert_eq!(bd.completion_rate, Some(0.5));
}

#[test]
fn day_breakdown_untimed_today_is_due_not_upcoming() {
    let (_f, conn) = fresh_db();
    let today = "2026-07-30";
    let t = add_dated_task(&conn, today, "anytime", None); // untimed → due mid-day
    let _ = t;
    let bd = reports::day_breakdown(&conn, today, today, 600).unwrap();
    assert_eq!(bd.not_recorded, 1);
    assert_eq!(bd.upcoming, 0);
    assert_eq!(bd.completion_rate, Some(0.0));
}

#[test]
fn day_breakdown_timed_today_before_start_is_upcoming() {
    let (_f, conn) = fresh_db();
    let today = "2026-07-30";
    add_dated_task(&conn, today, "later", Some(800)); // 13:20, now is 09:00 (540)
    let bd = reports::day_breakdown(&conn, today, today, 540).unwrap();
    assert_eq!(bd.not_recorded, 0);
    assert_eq!(bd.upcoming, 1);
    assert_eq!(bd.completion_rate, None); // denominator 0
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxiline-core --test reports day_breakdown`
Expected: FAIL — `cannot find function day_breakdown`.

- [ ] **Step 3: Implement `day_breakdown`**

Append to `crates/oxiline-core/src/reports.rs` (add `use crate::model::{DayBreakdown, CategoryBreakdown};` and `use rusqlite::Connection;` to the imports):
```rust
use crate::model::{CategoryBreakdown, DayBreakdown};
use crate::{categories, routines, tasks};
use rusqlite::Connection;

/// Bucket of a single scheduled occurrence on a date.
enum Bucket { Done, Skipped, NotRecorded, Upcoming }

fn bucket_of(is_done: bool, is_skipped: bool) -> Bucket {
    if is_skipped { Bucket::Skipped }
    else if is_done { Bucket::Done }
    else { Bucket::NotRecorded }
}

/// Is a timed/untimed occurrence on `date` (relative to `today`/`now_minute`) due?
fn is_due(date: &str, today: &str, start: Option<u16>, dur: Option<u16>, now_minute: u16) -> bool {
    if date < today { return true; }              // past → all due
    if date > today { return false; }             // future → none due
    match start {                                  // today
        None => true,                             // untimed → available all day → due
        Some(s) => s + dur.unwrap_or(0) <= now_minute,
    }
}

pub fn day_breakdown(
    conn: &Connection,
    date: &str,
    today: &str,
    now_minute: u16,
) -> Result<DayBreakdown> {
    let cats = categories::list(conn)?;
    let name_of = |cid: &Option<String>| -> String {
        cats.iter().find(|c| Some(&c.id) == cid.as_ref()).map(|c| c.name.clone()).unwrap_or_default()
    };

    let mut done = 0u32; let mut skipped = 0u32;
    let mut not_recorded = 0u32; let mut upcoming = 0u32;
    // (category_id) -> (done, skipped, not_recorded)
    let mut by_cat: std::collections::HashMap<Option<String>, (u32, u32, u32)> = Default::default();

    let mut bump = |cid: Option<String>, b: &Bucket,
                    done: &mut u32, sk: &mut u32, nr: &mut u32, up: &mut u32,
                    by_cat: &mut std::collections::HashMap<Option<String>, (u32,u32,u32)>| {
        let e = by_cat.entry(cid).or_insert((0,0,0));
        match b {
            Bucket::Done => { *done += 1; e.0 += 1; }
            Bucket::Skipped => { *sk += 1; e.1 += 1; }
            Bucket::NotRecorded => { *nr += 1; e.2 += 1; }
            Bucket::Upcoming => { *up += 1; } // not in any category rate
        }
    };

    // (A) materialized tasks for this date
    let day_tasks = tasks::list_by_date(conn, date)?;
    for t in &day_tasks {
        let due = is_due(date, today, t.start_minute, t.duration_minute, now_minute);
        let b = if due { bucket_of(t.is_done, t.is_skipped) } else { Bucket::Upcoming };
        bump(t.category_id.clone(), &b, &mut done, &mut skipped, &mut not_recorded, &mut upcoming, &mut by_cat);
    }

    // (B) virtual occurrences: active routines scheduled on `date` with no materialized row
    let materialized: std::collections::HashSet<String> = day_tasks.iter()
        .filter_map(|t| t.source_routine_block_id.clone()).collect();
    for b in routines::list(conn, true)? {
        if materialized.contains(&b.id) { continue; }
        if !scheduled_for(&b, date) { continue; }
        // virtual occurrence: never done/skipped → not_recorded if due, else upcoming
        let due = is_due(date, today, Some(b.start_minute), Some(b.duration_minute), now_minute);
        let bk = if due { Bucket::NotRecorded } else { Bucket::Upcoming };
        bump(b.category_id.clone(), &bk, &mut done, &mut skipped, &mut not_recorded, &mut upcoming, &mut by_cat);
    }

    let categories = build_cat_breakdown(&by_cat, &name_of);
    Ok(DayBreakdown {
        date: date.into(), done, skipped, not_recorded, upcoming,
        completion_rate: rate(done, not_recorded), categories,
    })
}

fn rate(done: u32, not_recorded: u32) -> Option<f64> {
    let denom = done + not_recorded;
    if denom == 0 { None } else { Some(done as f64 / denom as f64) }
}

fn build_cat_breakdown(
    by_cat: &std::collections::HashMap<Option<String>, (u32, u32, u32)>,
    name_of: &dyn Fn(&Option<String>) -> String,
) -> Vec<CategoryBreakdown> {
    let mut v: Vec<CategoryBreakdown> = by_cat.iter().map(|(cid, (d, s, n))| CategoryBreakdown {
        category_id: cid.clone(),
        category_name: name_of(cid),
        done: *d, skipped: *s, not_recorded: *n,
        completion_rate: rate(*d, *n),
    }).collect();
    v.sort_by(|a, b| a.category_name.cmp(&b.category_name));
    v
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxiline-core --test reports day_breakdown`
Expected: PASS (all 3).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/reports.rs crates/oxiline-core/tests/reports.rs
git commit -m "feat(core): day_breakdown — 3-bucket classification + temporal boundary"
```

---

### Task 4: `week_report` + `range_report` aggregation

**Files:**
- Modify: `crates/oxiline-core/src/reports.rs`
- Test: `crates/oxiline-core/tests/reports.rs` (append)

**Interfaces:**
- Produces: `week_report(conn, today, now_minute) -> Result<WeekReport>` and `range_report(conn, from, to, today, now_minute) -> Result<RangeReport>`. Consumed by the CLI (Task 6) and GUI (Task 7).
- Consumes: `day_breakdown` (Task 3), `settings::snapshot` (`week_starts_on`), `routine_streaks` (Task 5 — called here; implement Task 5 first OR stub + wire). **Order note:** implement Task 5 before wiring `streaks` here; until then this task leaves `streaks: vec![]` and Task 5 fills it.

- [ ] **Step 1: Write the failing tests**

Append to `crates/oxiline-core/tests/reports.rs`:
```rust
#[test]
fn range_report_excludes_future_days_from_rate() {
    let (_f, conn) = fresh_db();
    let today = "2026-07-30";
    // A done task yesterday, nothing today/future.
    let d = add_dated_task(&conn, "2026-07-29", "done", Some(540));
    tasks::set_done(&conn, &d.id, true).unwrap();
    let r = reports::range_report(&conn, "2026-07-29", "2026-08-01", today, 540).unwrap();
    assert_eq!(r.totals.done, 1);
    // future days (07-31..08-01) contribute only upcoming, not to the rate
    assert_eq!(r.completion_rate, Some(1.0));
}

#[test]
fn week_report_uses_monday_start_and_prev_week_rate() {
    let (_f, conn) = fresh_db();
    let today = "2026-07-30"; // Thursday; week_start (Mon) = 2026-07-27
    let r = reports::week_report(&conn, today, 540).unwrap();
    assert_eq!(r.week_start, "2026-07-27");
    assert_eq!(r.week_end, "2026-08-02");
    // empty DB → no rate
    assert_eq!(r.completion_rate, None);
    assert_eq!(r.prev_completion_rate, None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxiline-core --test reports`
Expected: FAIL — `cannot find function range_report` / `week_report`.

- [ ] **Step 3: Implement aggregation**

Append to `crates/oxiline-core/src/reports.rs` (add `use crate::model::{DayTotals, RangeReport, WeekReport};` and `use crate::settings;` to imports):
```rust
use crate::model::{DayTotals, RangeReport, WeekReport};
use crate::settings;
use chrono::Datelike;

/// Iterate YYYY-MM-DD strings over an inclusive [from, to] range.
fn each_day(from: &str, to: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = match util::parse_date(from) { Ok(d) => d, Err(_) => return out };
    let end = match util::parse_date(to) { Ok(d) => d, Err(_) => return out };
    while cur <= end {
        out.push(util::fmt_date(cur));
        cur += chrono::Duration::days(1);
    }
    out
}

fn totals_of(days: &[DayBreakdown]) -> DayTotals {
    let mut t = DayTotals { done: 0, skipped: 0, not_recorded: 0, upcoming: 0 };
    for d in days {
        t.done += d.done; t.skipped += d.skipped;
        t.not_recorded += d.not_recorded; t.upcoming += d.upcoming;
    }
    t
}

pub fn range_report(
    conn: &Connection, from: &str, to: &str, today: &str, now_minute: u16,
) -> Result<RangeReport> {
    let days: Vec<DayBreakdown> = each_day(from, to).iter()
        .map(|d| day_breakdown(conn, d, today, now_minute)).collect::<Result<_>>()?;
    let totals = totals_of(&days);
    let categories = rollup_categories(&days);
    Ok(RangeReport {
        from: from.into(), to: to.into(),
        completion_rate: rate(totals.done, totals.not_recorded),
        categories, totals, days,
        streaks: routine_streaks(conn, today)?,
    })
}

pub fn week_report(conn: &Connection, today: &str, now_minute: u16) -> Result<WeekReport> {
    let ws = &settings::snapshot(conn).week_starts_on;
    let week_start = monday_or_sunday_start(today, ws);
    let week_end = util::add_days(&week_start, 6).unwrap_or_else(|| week_start.clone());
    let days: Vec<DayBreakdown> = each_day(&week_start, &week_end).iter()
        .map(|d| day_breakdown(conn, d, today, now_minute)).collect::<Result<_>>()?;
    let totals = totals_of(&days);
    let categories = rollup_categories(&days);
    // previous 7 days rate
    let prev_from = util::add_days(&week_start, -7).unwrap_or_else(|| week_start.clone());
    let prev_to = util::add_days(&week_start, -1).unwrap_or_else(|| week_start.clone());
    let prev = each_day(&prev_from, &prev_to).iter()
        .map(|d| day_breakdown(conn, d, today, now_minute)).collect::<Result<Vec<_>>>()?;
    let prev_totals = totals_of(&prev);
    Ok(WeekReport {
        week_start, week_end, days, totals,
        completion_rate: rate(totals.done, totals.not_recorded),
        prev_completion_rate: rate(prev_totals.done, prev_totals.not_recorded),
        categories,
        streaks: routine_streaks(conn, today)?,
    })
}

fn rollup_categories(days: &[DayBreakdown]) -> Vec<CategoryBreakdown> {
    let mut map: std::collections::HashMap<Option<String>, (String, u32, u32, u32)> = Default::default();
    for d in days {
        for c in &d.categories {
            let e = map.entry(c.category_id.clone())
                .or_insert((c.category_name.clone(), 0, 0, 0));
            e.1 += c.done; e.2 += c.skipped; e.3 += c.not_recorded;
        }
    }
    let mut v: Vec<CategoryBreakdown> = map.into_iter().map(|(cid, (name, d, s, n))| CategoryBreakdown {
        category_id: cid, category_name: name, done: d, skipped: s, not_recorded: n,
        completion_rate: rate(d, n),
    }).collect();
    v.sort_by(|a, b| a.category_name.cmp(&b.category_name));
    v
}

/// The week-start date for `today` honoring `week_starts_on` ("mon" default, else "sun").
fn monday_or_sunday_start(today: &str, week_starts_on: &str) -> String {
    let d = match util::parse_date(today) { Ok(d) => d, Err(_) => return today.into() };
    let offset = if week_starts_on == "sun" {
        d.weekday().num_days_from_sunday() as i64
    } else {
        d.weekday().num_days_from_monday() as i64
    };
    util::fmt_date(d - chrono::Duration::days(offset))
}
```
> **Order note:** `routine_streaks` is referenced here but defined in Task 5. Implement Task 5 next; until then temporarily replace the two `routine_streaks(conn, today)?` calls with `vec![]` so this task compiles, then restore them in Task 5's commit.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxiline-core --test reports`
Expected: PASS (range_report + week_report; with `streaks: vec![]` placeholder if Task 5 not yet done).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/reports.rs crates/oxiline-core/tests/reports.rs
git commit -m "feat(core): week_report + range_report aggregation"
```

---

### Task 5: `routine_streak` + `routine_streaks` (the streak walk)

**Files:**
- Modify: `crates/oxiline-core/src/reports.rs`
- Test: `crates/oxiline-core/tests/reports.rs` (append)

**Interfaces:**
- Produces: `routine_streak(conn, block_id, today) -> Result<RoutineStreak>` and `routine_streaks(conn, today) -> Result<Vec<RoutineStreak>>`. Consumed by `week_report`/`range_report` (Task 4 — restore the calls here) and by the CLI `streak` command (Task 6).
- Consumes: `routines::{get, list, mask_includes}`, `scheduled_for`'s `bound_date`, `util`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/oxiline-core/tests/reports.rs`:
```rust
fn daily_routine_backdated(conn: &rusqlite::Connection, created: &str) -> oxiline_core::model::RoutineBlock {
    let b = routines::create(conn, routines::NewRoutineBlock {
        title: "R".into(), start_minute: 540, duration_minute: 30,
        weekday_mask: 0b1111111, category_id: None,
        effective_from: None, effective_until: None, notes: None,
    }).unwrap();
    backdate_created(conn, &b.id, created);
    routines::get(conn, &b.id).unwrap()
}

fn done_occurrence(conn: &rusqlite::Connection, block_id: &str, date: &str) {
    let t = oxiline_core::tasks::materialize_occurrence(conn, block_id, date).unwrap();
    oxiline_core::tasks::set_done(conn, &t.id, true).unwrap();
}
fn skip_occurrence(conn: &rusqlite::Connection, block_id: &str, date: &str) {
    let t = oxiline_core::tasks::materialize_occurrence(conn, block_id, date).unwrap();
    oxiline_core::tasks::set_skipped(conn, &t.id, true).unwrap();
}

#[test]
fn streak_counts_consecutive_done_and_survives_skip() {
    let (_f, conn) = fresh_db();
    let today = "2026-07-30"; // Thu
    let b = daily_routine_backdated(&conn, "2026-07-20T00:00:00Z");
    // Mon 27 done, Tue 28 skipped (transparent), Wed 29 done → streak 2 from today backward
    done_occurrence(&conn, &b.id, "2026-07-27");
    skip_occurrence(&conn, &b.id, "2026-07-28");
    done_occurrence(&conn, &b.id, "2026-07-29");
    let s = reports::routine_streak(&conn, &b.id, today).unwrap();
    assert_eq!(s.current, 2);
    assert_eq!(s.last_done_date.as_deref(), Some("2026-07-29"));
}

#[test]
fn streak_breaks_at_past_not_recorded_but_not_at_today_undone() {
    let (_f, conn) = fresh_db();
    let today = "2026-07-30"; // Thu
    let b = daily_routine_backdated(&conn, "2026-07-20T00:00:00Z");
    done_occurrence(&conn, &b.id, "2026-07-29"); // Wed done
    // Tue 28 not recorded (gap) → streak stops at 1 even though today (Thu) is also undone
    let s = reports::routine_streak(&conn, &b.id, today).unwrap();
    assert_eq!(s.current, 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxiline-core --test reports streak`
Expected: FAIL — `cannot find function routine_streak`.

- [ ] **Step 3: Implement the streak walk**

Append to `crates/oxiline-core/src/reports.rs` (add `use crate::model::RoutineStreak;`):
```rust
use crate::model::RoutineStreak;

pub fn routine_streak(conn: &Connection, block_id: &str, today: &str) -> Result<RoutineStreak> {
    let block = routines::get(conn, block_id)?;
    let today_d = util::parse_date(today).unwrap_or_else(|_| util::parse_date("1970-01-01").unwrap());
    let bound = util::parse_date(&bound_date(&block)).unwrap_or(today_d);

    // Materialized states for this block in [bound, today]: date -> (is_done, is_skipped)
    let mut stmt = conn.prepare(
        "SELECT date, is_done, is_skipped FROM tasks
         WHERE source_routine_block_id = ?1 AND date BETWEEN ?2 AND ?3")?;
    let rows: std::collections::HashMap<String, (bool, bool)> = stmt.query_map(
        rusqlite::params![block_id, util::fmt_date(bound), util::fmt_date(today_d)],
        |r| Ok((r.get::<_, String>(0)?, (r.get::<_, i64>(1)? != 0, r.get::<_, i64>(2)? != 0))),
    )?.filter_map(|r| r.ok()).collect();

    // Scheduled dates bound..today, newest first.
    let mut dates: Vec<chrono::NaiveDate> = Vec::new();
    let mut cur = bound;
    while cur <= today_d {
        if routines::mask_includes(block.weekday_mask, cur.weekday()) {
            dates.push(cur);
        }
        cur += chrono::Duration::days(1);
    }
    dates.sort_unstable_by(|a, b| b.cmp(a));

    let mut current = 0u32;
    let mut last_done: Option<String> = None;
    for (i, d) in dates.iter().enumerate() {
        let ds = util::fmt_date(*d);
        let (is_done, is_skipped) = rows.get(&ds).copied().unwrap_or((false, false));
        if is_skipped { continue; }                // transparent
        if is_done { current += 1; last_done = Some(ds.clone()); continue; }
        // not_recorded
        if i == 0 && ds == today { continue; }     // today not over yet — don't break
        break;                                     // past gap → stop
    }
    Ok(RoutineStreak { routine_id: block.id, title: block.title, current, last_done_date: last_done })
}

pub fn routine_streaks(conn: &Connection, today: &str) -> Result<Vec<RoutineStreak>> {
    let mut out = Vec::new();
    for b in routines::list(conn, true)? {
        out.push(routine_streak(conn, &b.id, today)?);
    }
    out.sort_by(|a, b| b.current.cmp(&a.current));
    Ok(out)
}
```
Now **restore** the two `routine_streaks(conn, today)?` calls in `week_report`/`range_report` (Task 4) if they were placeholdered with `vec![]`.

- [ ] **Step 4: Run the full core suite**

Run: `cargo test -p oxiline-core`
Expected: PASS — all reports tests + existing timeline tests (unaffected).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/reports.rs crates/oxiline-core/tests/reports.rs
git commit -m "feat(core): routine_streak/streaks — consecutive-done walk (transparent skip)"
```

---

### Task 6: CLI `report` + `streak` subcommands

**Files:**
- Modify: `crates/oxiline-cli/src/cli.rs` (add `Command::Report`/`Streak`)
- Modify: `crates/oxiline-cli/src/main.rs` (dispatch)
- Modify: `crates/oxiline-cli/src/output.rs` (renderers)
- Modify: `crates/oxiline-cli/src/lang.rs` (copy strings)

**Interfaces:**
- Consumes: `reports::{week_report, range_report, routine_streak, routine_streaks}`, `util::{today_local, now_minute_local, resolve_date_keyword}`, existing `parse_range`, `resolve_category_opt` (for streak name resolution).
- Pattern: matches `main.rs` `run()` dispatch (open conn → resolve lang → `match &opts.cmd` → `say(output::json_pretty(..))` or `say(output::<text>(..))`).

- [ ] **Step 1: Add the command variants to `cli.rs`**

In `crates/oxiline-cli/src/cli.rs`, add to `enum Command` (before `Doctor`):
```rust
    /// Completion report for a week or date range.
    Report {
        /// Current week (default).
        #[arg(long)]
        week: bool,
        /// Last N days (e.g. --last 30).
        #[arg(long, value_name = "N")]
        last: Option<u32>,
        /// Explicit inclusive range FROM:TO (YYYY-MM-DD).
        #[arg(long, value_name = "FROM:TO")]
        range: Option<String>,
    },
    /// Current consecutive-done streaks (all routines, or one by id/name).
    Streak {
        /// Routine id or name. Omit for all active routines.
        target: Option<String>,
    },
```

- [ ] **Step 2: Add renderers to `output.rs`**

Append to `crates/oxiline-cli/src/output.rs`:
```rust
use oxiline_core::model::{RangeReport, RoutineStreak, WeekReport};

fn pct(r: Option<f64>) -> String {
    match r { Some(v) => format!("{}%", (v * 100.0).round() as i64), None => "—".into() }
}

pub fn week_report_text(lang: L, r: &WeekReport) -> String {
    let mut out = format!("{} ~ {} ({})\n", r.week_start, r.week_end, lang.report_this_week());
    out.push_str(&totals_line(&r.totals.done, &r.totals.skipped, &r.not_recorded(&r), &r.upcoming));
    out.push_str(&format!("{} {}   {} {}\n", lang.report_rate(), pct(r.completion_rate),
                          lang.report_prev_week(), pct(r.prev_completion_rate)));
    out.push_str(&cat_block(lang, &r.categories));
    out.push_str(&streak_block(lang, &r.streaks));
    out
}

pub fn range_report_text(lang: L, r: &RangeReport) -> String {
    let mut out = format!("{} ~ {}\n", r.from, r.to);
    out.push_str(&format!("{} {}\n", lang.report_rate(), pct(r.completion_rate)));
    out.push_str(&cat_block(lang, &r.categories));
    out.push_str(&streak_block(lang, &r.streaks));
    out
}

pub fn streak_list_text(streaks: &[RoutineStreak]) -> String {
    let mut out = String::new();
    for s in streaks {
        out.push_str(&format!("  {:<16} {}{}\n", s.title, s.current,
                              if s.current == 1 { "일" } else { "일" }));
    }
    if out.is_empty() { out = "  (no active routines)\n".into(); }
    out
}

fn cat_block(lang: L, cats: &[oxiline_core::model::CategoryBreakdown]) -> String {
    let mut out = format!("\n{}\n", lang.report_categories());
    for c in cats {
        let denom = c.done + c.not_recorded;
        out.push_str(&format!("  {:<8} {}/{}  {}\n", c.category_name, c.done, denom, pct(c.completion_rate)));
    }
    out
}
fn streak_block(lang: L, streaks: &[RoutineStreak]) -> String {
    if streaks.is_empty() { return String::new(); }
    format!("\n{}\n{}", lang.report_streaks(), streak_list_text(streaks))
}
```
> Helper `not_recorded` is a field; inline its value directly in `totals_line`. Replace the `&r.not_recorded(&r)` placeholder with `&r.totals.not_recorded`. Concrete `totals_line`:
```rust
fn totals_line(done: &u32, skipped: &u32, not_recorded: &u32, upcoming: &u32) -> String {
    format!("{} {} · {} {} · {} {} · {} {}\n",
        "완료", done, "건너뜀", skipped, "체크인 없음", not_recorded, "예정", upcoming)
}
```
> (Korean literals here are fallbacks; wire `lang.*` accessors in Task 6 Step 4 if the `L` API exposes them — see lang.rs. If `L` lacks report accessors, add them in Step 4 below.)

- [ ] **Step 3: Add copy accessors to `lang.rs`**

Open `crates/oxiline-cli/src/lang.rs`, mirror an existing accessor (e.g. `empty_timeline`), and add:
```rust
pub fn report_this_week(&self) -> &str { self.k("이번 주", "this week") }
pub fn report_prev_week(&self) -> &str { self.k("저번 주", "prev week") }
pub fn report_rate(&self) -> &str { self.k("완료율", "completion") }
pub fn report_categories(&self) -> &str { self.k("카테고리", "categories") }
pub fn report_streaks(&self) -> &str { self.k("루틴 연속", "streaks") }
```
(Use the file's existing `k(ko, en)`/equivalent helper; if the helper is named differently, adapt to match.)

- [ ] **Step 4: Wire dispatch in `main.rs`**

In `crates/oxiline-cli/src/main.rs`, inside `match &opts.cmd { ... }` (add the `reports` import to the `use oxiline_core::{...}` line), add:
```rust
        Command::Report { week, last, range } => {
            let today = util::today_local();
            let now = util::now_minute_local();
            if let Some(r) = range {
                let (from, to) = parse_range(&r)?;
                let rep = reports::range_report(&conn, &from, &to, &today, now)?;
                if json { say(output::json_pretty(&rep)); }
                else { say(output::range_report_text(l, &rep)); }
            } else if let Some(n) = last {
                let to = today.clone();
                let from = util::add_days(&to, -((n as i64) - 1))
                    .unwrap_or_else(|| to.clone());
                let rep = reports::range_report(&conn, &from, &to, &today, now)?;
                if json { say(output::json_pretty(&rep)); }
                else { say(output::range_report_text(l, &rep)); }
            } else {
                let _ = week; // default
                let rep = reports::week_report(&conn, &today, now)?;
                if json { say(output::json_pretty(&rep)); }
                else { say(output::week_report_text(l, &rep)); }
            }
        }
        Command::Streak { target } => {
            let today = util::today_local();
            match target {
                None => {
                    let ss = reports::routine_streaks(&conn, &today)?;
                    if json { say(output::json_pretty(&ss)); }
                    else { say(output::streak_list_text(&ss)); }
                }
                Some(name) => {
                    // resolve id|name like categories: exact id first, else unique title match.
                    let id = resolve_routine_target(&conn, name)?;
                    let s = reports::routine_streak(&conn, &id, &today)?;
                    if json { say(output::json_pretty(&s)); }
                    else { say(output::streak_list_text(std::slice::from_ref(&s))); }
                }
            }
        }
```
Add a `resolve_routine_target` helper near `resolve_category_opt`:
```rust
fn resolve_routine_target(conn: &rusqlite::Connection, id_or_name: &str) -> Result<String> {
    // exact id
    if routines::get(conn, id_or_name).is_ok() { return Ok(id_or_name.into()); }
    let matches: Vec<_> = routines::list(conn, true)?.into_iter()
        .filter(|b| b.title == id_or_name).collect();
    match matches.len() {
        1 => Ok(matches[0].id.clone()),
        0 => Err(CoreError::NotFound(format!("routine '{id_or_name}'"))),
        _ => Err(CoreError::InvalidArgument(format!("ambiguous routine '{id_or_name}'"))),
    }
}
```

- [ ] **Step 5: Build + smoke-test the CLI**

Run: `cargo build -p oxiline-cli`
Then (against a throwaway DB):
```bash
OXILINE_DB_PATH=$(mktemp -u) cargo run -p oxiline-cli -- report --json
OXILINE_DB_PATH=$(mktemp -u) cargo run -p oxiline-cli -- streak --json
OXILINE_DB_PATH=$(mktemp -u) cargo run -p oxiline-cli -- report --last 7
```
Expected: builds clean; empty-DB `report --json` yields `completion_rate: null` and `streaks: []`; human mode prints the week header + `완료율 —`.

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-cli/src/cli.rs crates/oxiline-cli/src/main.rs crates/oxiline-cli/src/output.rs crates/oxiline-cli/src/lang.rs
git commit -m "feat(cli): oxiline report + streak subcommands"
```

---

### Task 7: GUI Tauri commands + specta registration

**Files:**
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs`
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `get_week_report`, `get_range_report`, `get_routine_streaks` Tauri commands (TS bindings auto-emitted to `src/bindings.ts`). Consumed by `ReportView.tsx` (Task 8) via `api.ts`.
- Pattern: `#[tauri::command] #[specta::specta] pub fn name(state: State<AppState>, ...) -> Result<T, String> { ... }.map_err(map_err)`, registered in `collect_commands!`.

- [ ] **Step 1: Add the three commands to `commands.rs`**

Append to `crates/oxiline-app/src-tauri/src/commands.rs` (extend the `use oxiline_core::model::{...}` import with `RoutineStreak, RangeReport, WeekReport`; add `reports` to `use oxiline_core::{...}`):
```rust
#[tauri::command]
#[specta::specta]
pub fn get_week_report(state: State<AppState>) -> Result<WeekReport, String> {
    let conn = state.conn();
    reports::week_report(&conn, &util::today_local(), util::now_minute_local()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn get_range_report(state: State<AppState>, from: String, to: String) -> Result<RangeReport, String> {
    let conn = state.conn();
    reports::range_report(&conn, &from, &to, &util::today_local(), util::now_minute_local()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn get_routine_streaks(state: State<AppState>) -> Result<Vec<RoutineStreak>, String> {
    let conn = state.conn();
    reports::routine_streaks(&conn, &util::today_local()).map_err(map_err)
}
```

- [ ] **Step 2: Register in `lib.rs`**

In `crates/oxiline-app/src-tauri/src/lib.rs`, add to the `collect_commands![ ... ]` list:
```rust
        commands::get_week_report,
        commands::get_range_report,
        commands::get_routine_streaks,
```

- [ ] **Step 3: Build the GUI crate (regenerates bindings.ts in debug)**

Run: `cargo build -p oxiline-app`
Expected: builds clean; `crates/oxiline-app/src/bindings.ts` now contains `WeekReport`/`RangeReport`/`RoutineStreak` types and the three command bindings.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/commands.rs crates/oxiline-app/src-tauri/src/lib.rs crates/oxiline-app/src/bindings.ts
git commit -m "feat(app): Tauri commands for week/range report + routine streaks"
```

---

### Task 8: React Report tab

**Files:**
- Modify: `crates/oxiline-app/src/types.ts` (mirror Rust structs, or re-export from `bindings.ts`)
- Modify: `crates/oxiline-app/src/lib/api.ts`
- Modify: `crates/oxiline-app/src/lib/store.ts`
- Create: `crates/oxiline-app/src/components/ReportView.tsx`
- Modify: `crates/oxiline-app/src/App.tsx`
- Modify: `crates/oxiline-app/src/locales/{ko,en}.json`

**Interfaces:**
- Consumes: the three Tauri command bindings (Task 7) and `useUi` store. `ReportView` fetches via React Query and re-fetches on `onDbChanged`.
- Pattern: existing `WeekView.tsx`/`DayTimeline.tsx` for styling; Zustand `View` for tab state.

- [ ] **Step 1: Mirror types in `types.ts`**

Append to `crates/oxiline-app/src/types.ts` (match `bindings.ts` shapes):
```ts
export interface CategoryBreakdown {
  category_id: string | null; category_name: string;
  done: number; skipped: number; not_recorded: number; completion_rate: number | null;
}
export interface DayTotals { done: number; skipped: number; not_recorded: number; upcoming: number; }
export interface RoutineStreak { routine_id: string; title: string; current: number; last_done_date: string | null; }
export interface WeekReport {
  week_start: string; week_end: string; days: unknown[]; totals: DayTotals;
  completion_rate: number | null; prev_completion_rate: number | null;
  categories: CategoryBreakdown[]; streaks: RoutineStreak[];
}
export interface RangeReport {
  from: string; to: string; days: unknown[]; totals: DayTotals;
  completion_rate: number | null; categories: CategoryBreakdown[]; streaks: RoutineStreak[];
}
```

- [ ] **Step 2: Add API wrappers in `api.ts`**

In `crates/oxiline-app/src/lib/api.ts`, add to the `api` object:
```ts
  getWeekReport: () => invoke<WeekReport>("get_week_report"),
  getRangeReport: (from: string, to: string) => invoke<RangeReport>("get_range_report", { from, to }),
  getRoutineStreaks: () => invoke<RoutineStreak[]>("get_routine_streaks"),
```
(import `WeekReport, RangeReport, RoutineStreak` from `../types`.)

- [ ] **Step 3: Add `"report"` to the store `View`**

In `crates/oxiline-app/src/lib/store.ts`:
```ts
export type View = "today" | "week" | "backlog" | "report";
```

- [ ] **Step 4: Create `ReportView.tsx`**

Create `crates/oxiline-app/src/components/ReportView.tsx`. Neutral palette only (no green/red). Uses existing token classes from `styles.css` (`text-tertiary`, `surface-sunken`, `border-default`, `accent-oxide`):
```tsx
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { useUi } from "../lib/store";
import type { WeekReport } from "../types";

const pct = (r: number | null) => (r == null ? "—" : `${Math.round(r * 100)}%`);

export function ReportView() {
  const { t } = useI18n(); // existing i18n hook (see App.tsx usage)
  const { data: report } = useQuery<WeekReport>({
    queryKey: ["weekReport"],
    queryFn: api.getWeekReport,
  });
  if (!report) return <div className="p-4 text-secondary">{t("report.loading")}</div>;
  const tot = report.totals;
  return (
    <div className="p-4 space-y-5">
      <header className="flex items-baseline justify-between">
        <h2 className="text-primary text-lg">{report.week_start} ~ {report.week_end}</h2>
        <span className="text-tertiary text-sm">{t("report.thisWeek")}</span>
      </header>

      {/* overall rate bar — fill density only, oxide accent */}
      <RateBar rate={report.completion_rate} />
      <p className="text-tertiary text-sm">
        {t("report.rate")} {pct(report.completion_rate)}
        <span className="ml-3">{t("report.prevWeek")} {pct(report.prev_completion_rate)}</span>
      </p>

      {/* three neutral buckets — NO judgment colors */}
      <div className="flex gap-4 text-sm">
        <Bucket label={t("report.done")} n={tot.done} cls="text-primary" />
        <Bucket label={t("report.skipped")} n={tot.skipped} cls="text-secondary" />
        <Bucket label={t("report.notRecorded")} n={tot.not_recorded} cls="text-tertiary" />
        <Bucket label={t("report.upcoming")} n={tot.upcoming} cls="text-tertiary" />
      </div>

      <section>
        <h3 className="text-secondary text-sm mb-1">{t("report.categories")}</h3>
        {report.categories.map((c) => (
          <CatRow key={c.category_id ?? "none"} c={c} />
        ))}
      </section>

      <section>
        <h3 className="text-secondary text-sm mb-1">{t("report.streaks")}</h3>
        {report.streaks.map((s) => (
          <div key={s.routine_id} className="flex justify-between text-sm py-0.5">
            <span className="text-primary">{s.title}</span>
            <span className="text-tertiary tabular-nums">{s.current}{t("report.days")}</span>
          </div>
        ))}
      </section>
    </div>
  );
}

function RateBar({ rate }: { rate: number | null }) {
  const w = rate == null ? 0 : Math.round(rate * 100);
  return (
    <div className="h-2 rounded-full bg-surface-sunken border border-border-subtle overflow-hidden">
      <div className="h-full bg-accent-oxide transition-all" style={{ width: `${w}%` }} />
    </div>
  );
}
function Bucket({ label, n, cls }: { label: string; n: number; cls: string }) {
  return <div className={cls}><span className="tabular-nums">{n}</span> <span className="text-tertiary">{label}</span></div>;
}
function CatRow({ c }: { c: import("../types").CategoryBreakdown }) {
  const denom = c.done + c.not_recorded;
  const w = c.completion_rate == null ? 0 : Math.round(c.completion_rate * 100);
  return (
    <div className="py-1">
      <div className="flex justify-between text-sm"><span>{c.category_name || "—"}</span>
        <span className="text-tertiary tabular-nums">{c.done}/{denom} · {pct(c.completion_rate)}</span></div>
      <div className="h-1.5 rounded-full bg-surface-sunken overflow-hidden mt-0.5">
        <div className="h-full bg-accent-oxide" style={{ width: `${w}%` }} />
      </div>
    </div>
  );
}
```
> Adapt the `useI18n`/`t` import + token class names to match the project's actual conventions (check `App.tsx` and `styles.css` `@theme` aliases: `text-primary`, `text-secondary`, `text-tertiary`, `surface-sunken`, `border-subtle`, `accent-oxide` are all defined). Replace placeholder `useI18n` with the real hook.

- [ ] **Step 5: Add the tab + keyboard `4` in `App.tsx`**

In `crates/oxiline-app/src/App.tsx`: add a `report` button to the view switcher (next to `오늘/주간/백로그`), render `<ReportView />` when `view === "report"`, and bind key `4` to `setView("report")` in the existing keyboard handler (extending §7.10). Wire `onDbChanged` to invalidate the `["weekReport"]` query key (React Query `queryClient.invalidateQueries`).

- [ ] **Step 6: Add i18n keys**

In `crates/oxiline-app/src/locales/ko.json` and `en.json`, add a `report` block:
```json
"report": {
  "thisWeek": "이번 주", "prevWeek": "저번 주", "rate": "완료율",
  "done": "완료", "skipped": "건너뜀", "notRecorded": "체크인 없음", "upcoming": "예정",
  "categories": "카테고리", "streaks": "루틴 연속", "days": "일", "loading": "불러오는 중…"
}
```
English: `thisWeek` "This week", `prevWeek` "Last week", `rate` "Completion", `done` "Done", `skipped` "Skipped", `notRecorded` "No check-in", `upcoming` "Upcoming", `categories` "Categories", `streaks` "Streaks", `days` "d", `loading` "Loading…".

- [ ] **Step 7: Build + run the GUI and visually verify**

Run: `cargo build -p oxiline-app` then launch the app. Switch to the **리포트** tab. Confirm: neutral colors only (no red/green), three buckets visible, category bars, streak integers, and that checking a task in the Day view updates the Report tab within ~1s (db-changed invalidation). Empty DB shows the week header with `완료율 —`.

- [ ] **Step 8: Commit**

```bash
git add crates/oxiline-app/src/types.ts crates/oxiline-app/src/lib/api.ts crates/oxiline-app/src/lib/store.ts crates/oxiline-app/src/components/ReportView.tsx crates/oxiline-app/src/App.tsx crates/oxiline-app/src/locales/ko.json crates/oxiline-app/src/locales/en.json
git commit -m "feat(app): Report tab — neutral 3-bucket weekly report + streaks"
```

---

## Self-Review

**1. Spec coverage:**
- §1 Non-goals (no best/per-day-verdict/fire) — enforced in Global Constraints + copy rules (Task 6 Step 3, Task 8 Step 6); `RoutineStreak` has only `current`/`last_done_date` (Task 1). ✓
- §2.1 created_at bound + scope note — `scheduled_for` (Task 2), reports-local, timeline untouched. ✓
- §2.2 three-bucket + rate formula — `day_breakdown`/`rate` (Task 3). ✓
- §2.3 temporal boundary incl. untimed-today — `is_due` (Task 3), test `day_breakdown_untimed_today_is_due`. ✓
- §2.4 streak walk (transparent skip, today-not-done) — `routine_streak` (Task 5), two tests. ✓
- §3 types — Task 1. §3.2 functions — Tasks 2–5. ✓
- §4 CLI — Task 6. §5 GUI — Tasks 7–8. §7 tests — all 7 contracts covered (Tasks 2–5). ✓

**2. Placeholder scan:** Task 6 Step 2 has one inline correction note (`not_recorded(&r)` → field) and a `lang.rs` "adapt to match" note — both explicit instructions, not TBDs. Task 8 Step 4 flags `useI18n`/class-name adaptation — explicit. No "TODO/implement later".

**3. Type consistency:** `routine_streaks(conn, today)` referenced in Task 4, defined in Task 5 (with an explicit ordering note). `scheduled_for` used in Task 3/5, defined in Task 2. `DayBreakdown`/`WeekReport`/`RangeReport`/`RoutineStreak` field names match across Tasks 1/3/4/5/8. `day_breakdown` signature is `(conn, date, today, now_minute)` consistently in Tasks 3/4. ✓

No gaps found.
