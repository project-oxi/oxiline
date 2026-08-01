# Recording Core + CLI — Implementation Plan (Plan 1 of 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the data layer + CLI for recording-centered OxiLine (activities, OR plans, records, neutral weekly compliance, 5-min rounding) as pure Rust, fully tested, before any GUI work.

**Architecture:** Additive `V4` migration introduces four new tables alongside the legacy schema. Three new `oxiline-core` modules (`activities`, `plan`, `record`) hold all logic; CLI (`oxiline-cli`) gets three new clap command groups that call them. Plan↔record linkage is **computed** (derived resolution), never stored. No `is_done`/checkbox anywhere; completion = a record exists.

**Tech Stack:** Rust (edition 2024) · `rusqlite` + `rusqlite_migration` (WAL) · `serde` + `specta::Type` · `clap` v4 (derive) · `chrono`. Sync core (no async), matching existing `reports.rs`/`tasks.rs`.

## Global Constraints (from spec `2026-08-01-record-layer-design.md`)

- All time columns: records use **ISO 8601 UTC instants** (second precision); plans use **LOCAL wall-clock minute-of-day** integers (0..1439) — the existing convention.
- **No `duration` column** on records — derived + rounded. **No `is_done`/score/streak columns.**
- `records.activity_id` is `ON DELETE RESTRICT`; `delete_activity(force)` refuses with records unless `force`.
- Single-active invariant: at most one open record; `start` is one WAL transaction (close-all-then-insert).
- Rounding: `record_rounding_minutes` setting (default 5); durations round half-up to it; 0 disables. Precise instants retained.
- Compliance is a **neutral ratio** — `Under`/`Met`/`Over`/`Unbudgeted` share the activity's hue; never status-red/green. Weekly is the primary scope.
- `specta::Type` derived on every public struct/enum (tauri-specta bindings for Plan 2). `#[serde(rename_all = "snake_case")]` on enums.
- Components consume semantic tokens only (Plan 2, GUI) — N/A to this plan.
- **Additive only:** do NOT drop or alter legacy `tasks`/`routine_blocks`/`categories` tables in `V4`. Legacy demolition is a separate later plan.

**Reference spec:** `docs/superpowers/specs/2026-08-01-record-layer-design.md` (§4 schema, §5 modules, §3 semantics). Read it before Task 1.

---

## File Structure

**Create:**
- `crates/oxiline-core/migrations/V4__record.sql` — additive schema (activities, plans, plan_options, records) + settings seeds.
- `crates/oxiline-core/src/activities.rs` — activity CRUD + `resolve_activity`.
- `crates/oxiline-core/src/plan.rs` — plan CRUD, options, `slots_for_date` (computed view-model with resolution).
- `crates/oxiline-core/src/record.rs` — start/stop/current (single-active), list/edit/delete, `resolve_plan_for`, `compliance`.
- `crates/oxiline-core/tests/activities.rs`, `tests/plan.rs`, `tests/record.rs` — integration tests (follow existing `tests/timeline.rs` harness).

**Modify:**
- `crates/oxiline-core/src/model.rs` — add domain types (§5.4 of spec).
- `crates/oxiline-core/src/util.rs` — add `round_duration`.
- `crates/oxiline-core/src/lib.rs` — `pub mod activities; pub mod plan; pub mod record;`.
- `crates/oxiline-cli/src/cli.rs` — `record`, `activity`, `plan` command groups (clap derive).
- `crates/oxiline-cli/src/output.rs` + `lang.rs` — formatters / ko-en copy for the new commands.

Each core module owns one entity (matches existing `routines.rs`/`tasks.rs`/`categories.rs` per-entity style).

---

## Phase 1 — Core (pure Rust, TDD)

### Task 1: `V4` migration + domain types

**Files:**
- Create: `crates/oxiline-core/migrations/V4__record.sql`
- Modify: `crates/oxiline-core/src/db.rs` (add `V4_RECORD` const + push into `migrations()` vec — migrations are NOT auto-discovered; each is an explicit `include_str!` registered by hand in `db.rs:11-17`)
- Modify: `crates/oxiline-core/src/model.rs` (append types)
- Test: `crates/oxiline-core/tests/record.rs` (create file; first test only — uses the `NamedTempFile` + `open_and_migrate(path)` harness from `tests/timeline.rs:13-17`, NOT `:memory:`; call `settings::ensure_defaults(&conn)` after open)

**Interfaces:**
- Produces: tables `activities, plans, plan_options, records`; types `Activity, Plan, PlanOption, PlanSlot, Record, ActiveSession, Compliance, ComplianceState, RecordState, Scope, ActivityInput, PlanInput`.

- [ ] **Step 1a: Wire the migration into `db.rs`** *(without this, the `.sql` file is a no-op)*

In `crates/oxiline-core/src/db.rs`, add a `const V4_RECORD: &str = include_str!("../migrations/V4__record.sql");` alongside the existing `V1_INIT`/`V2_PHASE2`/`V3_OXI_PALETTE` consts, and append `M::up(V4_RECORD)` to the `Migrations::new(vec![…])` call. Migrations are NOT auto-discovered.
- [ ] **Step 1: Write the migration**

`crates/oxiline-core/migrations/V4__record.sql` — copy the schema block verbatim from spec §4 (activities, plans, plan_options, records, all `CREATE INDEX`es), then append settings seeds:


```sql
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
  ('record_switch_hotkey','"CmdOrCtrl+Shift+A"','2026-08-01T00:00:00Z'),
  ('record_rounding_minutes','5','2026-08-01T00:00:00Z'),
  ('record_default_stop_on_quit','true','2026-08-01T00:00:00Z'),
  ('record_stale_open_hours','12','2026-08-01T00:00:00Z'),
  ('timetable_default_mode','"both"','2026-08-01T00:00:00Z'),
  ('budget_default_scope','"weekly"','2026-08-01T00:00:00Z');
```

- [ ] **Step 2: Add domain types to `model.rs`**

Append (derive `Serialize, Deserialize, Type, Clone, Debug`; `#[serde(rename_all="snake_case")]` on enums):

```rust
pub struct Activity { pub id: String, pub name: String, pub hue_label: Option<String>,
    pub icon: Option<String>, pub category_id: Option<String>,
    pub target_minutes_daily: Option<u32>, pub target_minutes_weekly: Option<u32>,
    pub is_active: bool, pub sort_order: i32 }
pub struct Plan { pub id: String, pub date: Option<String>, pub start_minute: u16,
    pub duration_minute: u16, pub weekday_mask: u8, pub title: Option<String>, pub sort_order: i32 }
pub struct PlanOption { pub id: String, pub plan_id: String, pub activity_id: String, pub sort_order: i32 }
pub struct PlanSlot { pub plan_id: String, pub date: String, pub start_minute: u16, pub duration_minute: u16,
    pub options: Vec<Activity>, pub is_resolved: bool, pub resolved_by: Option<Record> }
pub struct Record { pub id: String, pub activity_id: String, pub started_at: String,
    pub ended_at: Option<String>, pub note: Option<String> }
pub struct ActiveSession { pub record: Record, pub activity: Activity, pub elapsed_seconds: u64 }
pub struct Compliance { pub activity: Activity, pub recorded_seconds: u64, pub target_seconds: Option<u64>,
    pub ratio: Option<f64>, pub remaining_seconds: Option<i64>, pub state: ComplianceState }
pub enum ComplianceState { Under, Met, Over, Unbudgeted }
pub struct RecordState { pub active: Option<ActiveSession>, pub today: Vec<Compliance>, pub generated_at: String }
pub enum Scope { Today, Week }
pub struct ActivityInput { pub name: Option<String>, pub hue_label: Option<String>, pub icon: Option<String>,
    pub category_id: Option<String>, pub target_minutes_daily: Option<Option<u32>>,
    pub target_minutes_weekly: Option<Option<u32>>, pub is_active: Option<bool>, pub sort_order: Option<i32> }
pub struct PlanInput { pub date: Option<String>, pub start_minute: u16, pub duration_minute: u16,
    pub weekday_mask: u8, pub title: Option<String>, pub activity_ids: Vec<String> }
```

- [ ] **Step 3: Write the failing test**

`crates/oxiline-core/tests/record.rs` — note: every test in this file uses the same `db()` helper (defined at the top of the file per Task 1's harness note). The test uses only `oxiline_core::open_and_migrate` + raw SQL — no module imports of `activities`/`plan`/`record` are needed (those modules are added in Tasks 3/4/5 with their own `pub mod` line each; Task 1 deliberately does NOT touch `lib.rs` to avoid `E0583 file not found` errors before those modules exist):
```rust
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
    let mut names: Vec<String> = c.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap().query_map([], |r| r.get::<_,String>(0)).unwrap().map(|r| r.unwrap()).collect();
    names.sort();
    assert!(names.iter().any(|n| n == "activities"));
    assert!(names.iter().any(|n| n == "plans"));
    assert!(names.iter().any(|n| n == "plan_options"));
    assert!(names.iter().any(|n| n == "records"));
}
```

> **Test harness (use this exact shape across all core tests):** each test file defines a `db() -> (NamedTempFile, Connection)` helper using `tempfile::NamedTempFile` + `oxiline_core::open_and_migrate(f.path())` + `settings::ensure_defaults(&conn)` — same pattern as `tests/timeline.rs:13-17`. The `:memory:` shorthand does NOT work with `open_and_migrate(path)` (it takes a `&Path`).

- [ ] **Step 4: Run — verify pass**

Run: `cargo test -p oxiline-core --test record v4_creates_record_tables`
Expected: PASS. (Step 1a is what actually applies V4; the `.sql` file alone does nothing.)

- [ ] **Step 5: Commit**
```bash
git add crates/oxiline-core/migrations/V4__record.sql crates/oxiline-core/src/db.rs crates/oxiline-core/src/model.rs crates/oxiline-core/tests/record.rs
git commit -m "feat(core): V4 migration + recording domain types"
```


---

### Task 2: `util::round_duration`

**Files:** Modify `crates/oxiline-core/src/util.rs` · Test: `tests/record.rs`

**Interfaces:** Produces `pub fn round_duration(seconds: u64, increment_minutes: u32) -> u64` (0 ⇒ identity).

- [ ] **Step 1: Failing test** — append to `tests/record.rs`:
```rust
#[test]
fn round_duration_snaps_half_up() {
    use oxiline_core::util::round_duration;
    assert_eq!(round_duration(42*60, 5), 40*60);   // 2520s -> 2400s (nearest 5min, half-up)
    assert_eq!(round_duration(37*60+30, 5), 40*60); // 2250s -> 2400s (half-up at 37.5)
    assert_eq!(round_duration(42*60, 0), 42*60);    // 0 disables
    assert_eq!(round_duration(0, 5), 0);
}
```
- [ ] **Step 2: Run — verify fail** (`util::round_duration` not found).
- [ ] **Step 3: Implement** in `util.rs`:
```rust
pub fn round_duration(seconds: u64, increment_minutes: u32) -> u64 {
    if increment_minutes == 0 { return seconds; }
    let step = increment_minutes as u64 * 60;
    let q = (seconds + step / 2) / step;     // half-up
    q * step
}
```
- [ ] **Step 4: Run — verify pass.**
- [ ] **Step 5: Commit** — `feat(core): round_duration helper`.

---

### Task 3: `activities.rs` — CRUD + resolve (no delete yet)

- Create: `crates/oxiline-core/src/activities.rs` · Modify `lib.rs` (`pub mod activities;`) · Test: `tests/activities.rs` (use the `NamedTempFile` harness described in Task 1's Step 4 note)

**Interfaces:** Consumes `ActivityInput`. Produces `create_activity, list_activities(active_only), get_activity, update_activity, resolve_activity`.

- [ ] **Step 1: Failing test** — `tests/activities.rs` (same harness as Task 1):
```rust
use oxiline_core::model::ActivityInput;
use tempfile::NamedTempFile;

fn db() -> (NamedTempFile, rusqlite::Connection) {
    let f = NamedTempFile::new().unwrap();
    let c = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&c).unwrap();
    (f, c)
}

#[test]
fn create_list_resolve_activity() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(&c, oxiline_core::model::ActivityInput{
        name: Some("코딩".into()), hue_label: Some("blue".into()), icon: None, category_id: None,
        target_minutes_daily: Some(Some(240)), target_minutes_weekly: Some(Some(1200)),
        is_active: None, sort_order: None }).unwrap();
    assert_eq!(a.name, "코딩");
    let listed = oxiline_core::activities::list_activities(&c, false).unwrap();
    assert_eq!(listed.len(), 1);
    let r = oxiline_core::activities::resolve_activity(&c, "코딩").unwrap();   // case-insensitive name
    assert_eq!(r.id, a.id);
    let r2 = oxiline_core::activities::resolve_activity(&c, &a.id).unwrap();   // by id
    assert_eq!(r2.id, a.id);
}
```

- [ ] **Step 2: Run — verify fail.**
- [ ] **Step 3: Implement** `activities.rs`: `create_activity` (INSERT, return row), `list_activities` (`SELECT ... WHERE is_active` when active_only, ORDER BY sort_order), `get_activity` (by id), `update_activity` (apply `ActivityInput` fields; double-Option on targets ⇒ set/clear), `resolve_activity` (exact id, else case-insensitive exact name; ambiguous ⇒ `CoreError::Ambiguous`, none ⇒ `NotFound` — reuse the category resolution style). IDs via the existing UUID helper used in `tasks.rs`/`routines.rs`.
- [ ] **Step 4: Run — verify pass.**
- [ ] **Step 5: Commit** — `feat(core): activities CRUD + resolve`.

---

### Task 4: `plan.rs` — OR choice-sets + `slots_for_date`
- Create: `crates/oxiline-core/src/plan.rs` · Modify `lib.rs` · Test: `tests/plan.rs`

**Interfaces:** Consumes `PlanInput`. Produces `create_plan, list_plans(recurring_only), update_plan, delete_plan, add_option, remove_option, slots_for_date(date)`.

- [ ] **Step 1: Failing test** — `tests/plan.rs`:
```rust
use tempfile::NamedTempFile;

fn db() -> (NamedTempFile, rusqlite::Connection) {
    let f = NamedTempFile::new().unwrap();
    let c = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&c).unwrap();
    (f, c)
}

#[test]
fn plan_holds_or_options_and_materializes_per_date() {
    let (_f, c) = db();
    let a1 = oxiline_core::activities::create_activity(&c, oxiline_core::model::ActivityInput{name:Some("코딩".into()),..Default::default()}).unwrap();
    let a2 = oxiline_core::activities::create_activity(&c, oxiline_core::model::ActivityInput{name:Some("독서".into()),..Default::default()}).unwrap();
    // recurring plan: weekday bit for Monday (bit0), 11:00–13:00, options 코딩 OR 독서
    let p = oxiline_core::plan::create_plan(&c, oxiline_core::model::PlanInput{ date:None, start_minute:11*60, duration_minute:120,
        weekday_mask:0b0000001, title:None, activity_ids:vec![a1.id.clone(), a2.id.clone()] }).unwrap();
    let slots = oxiline_core::plan::slots_for_date(&c, "2026-08-03").unwrap(); // 2026-08-03 is a Monday
    let ours = slots.iter().find(|s| s.plan_id==p.id).unwrap();
    assert_eq!(ours.options.len(), 2);
    assert!(!ours.is_resolved); // no records yet
}
```
- [ ] **Step 2: Run — verify fail.**
- [ ] **Step 3: Implement** `plan.rs`:
  - `create_plan`: INSERT plan + each `plan_options` row (sort_order = index) in one txn.
  - `slots_for_date(date)`: SELECT plans where `(weekday_mask=0 AND date=date) OR (weekday_mask != 0 AND mask includes weekday_of(date))`; for each, load its options (JOIN activities), build `PlanSlot`. `is_resolved`/`resolved_by` left `false`/`None` here — set in Task 7 by calling `record::resolve_plan_for` (or leave a TODO-wired call; **set it in Task 7**, document in Step 3 comment).
  - `list_plans(recurring_only)`: raw rows. `update_plan`/`delete_plan` (CASCADE removes options). `add_option`/`remove_option`.
  - Weekday-of-date via `chrono::Datelike` (the existing `reports.rs` already uses this pattern).
- [ ] **Step 4: Run — verify pass** (resolution still false; that's correct with no records).
- [ ] **Step 5: Commit** — `feat(core): plans (OR choice-sets) + slots_for_date`.

---

### Task 5: `record.rs` — start/stop/current (single-active)

- Create: `crates/oxiline-core/src/record.rs` · Modify `lib.rs` · Test: append to `tests/record.rs` (same harness)

**Interfaces:** Consumes `DateTime<Utc>` + `today`. Produces `start, stop, current` returning `RecordState`.

- [ ] **Step 1: Failing test** — append to `tests/record.rs`:
```rust
use chrono::{TimeZone, Utc};

fn activity_input(name: &str) -> oxiline_core::model::ActivityInput {
    oxiline_core::model::ActivityInput {
        name: Some(name.into()),
        hue_label: None, icon: None, category_id: None,
        target_minutes_daily: None, target_minutes_weekly: None,
        is_active: None, sort_order: None,
    }
}

#[test]
fn start_switches_single_active() {
    let (_f, c) = db();   // helper at top of tests/record.rs (Task 1 harness)
    let a = oxiline_core::activities::create_activity(&c, activity_input("코딩")).unwrap();
    let b = oxiline_core::activities::create_activity(&c, activity_input("독서")).unwrap();
    let now = Utc.with_ymd_and_hms(2026,8,3,9,0,0).unwrap();
    oxiline_core::record::start(&c, &a.id, now, "2026-08-03").unwrap();
    oxiline_core::record::start(&c, &b.id, now, "2026-08-03").unwrap();  // switch
    let st = oxiline_core::record::current(&c, now, "2026-08-03").unwrap();
    assert_eq!(st.active.as_ref().unwrap().activity.id, b.id);   // B is the open one
    // exactly one open row:
    let open: i64 = c.query_row("SELECT COUNT(*) FROM records WHERE ended_at IS NULL", [], |r| r.get(0)).unwrap();
    assert_eq!(open, 1);
}
```
(`activity_input` helper lives at the top of `tests/record.rs`.)
- [ ] **Step 2: Run — verify fail.**
- [ ] **Step 3: Implement** `record.rs`:
  - `start(conn, activity_id, now, today)`: one txn — `UPDATE records SET ended_at=:now, updated_at=:now WHERE ended_at IS NULL;` then `INSERT INTO records(id, activity_id, started_at, ended_at NULL, ...)`. Return `current(...)`.
  - `stop`: `UPDATE ... SET ended_at=:now WHERE ended_at IS NULL`. No-op if none.
  - `current`: SELECT the open record (most recent `started_at`); **defensive repair** — if >1 open (corruption/race), close all but the newest. Build `RecordState{active, today: vec![]}` (compliance filled in Task 7; here `today` can be `vec![]` or a stub).
- [ ] **Step 4: Run — verify pass.**
- [ ] **Step 5: Commit** — `feat(core): record start/stop/current (single-active)`.

---

### Task 6: list/edit/delete + `resolve_plan_for` + `delete_activity(force)`

**Files:** Modify `record.rs` · Modify `activities.rs` (add `delete_activity`) · Test: append `tests/record.rs`, `tests/activities.rs`

**Interfaces:** Produces `record::{list_records, edit_record, delete_record, resolve_plan_for}`, `activities::delete_activity(force)`.

- [ ] **Step 1: Failing tests** — append to `tests/record.rs` and `tests/activities.rs`:
(Add `use chrono::{TimeZone, Utc};` to the top of `tests/record.rs` if not already present — needed for `Utc::now()` / `Utc.with_ymd_and_hms` in these tests.)
```rust
// tests/record.rs append
#[test]
fn resolve_links_record_to_plan() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(&c, activity_input("코딩")).unwrap();
    let p = oxiline_core::plan::create_plan(&c, plan_input(&a.id, 0b0000001, 9*60, 90)).unwrap();
    let now = Utc.with_ymd_and_hms(2026,8,3,9,10,0).unwrap();   // Monday 09:10, inside plan window
    oxiline_core::record::start(&c, &a.id, now, "2026-08-03").unwrap();
    let rec = oxiline_core::record::current(&c, now, "2026-08-03").unwrap().active.unwrap().record;
    let slot = oxiline_core::record::resolve_plan_for(&c, &rec).unwrap();
    assert_eq!(slot.unwrap().plan_id, p.id);
}

// tests/activities.rs append (use its db() helper)
#[test]
fn delete_activity_refuses_with_history() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(&c, activity_input("코딩")).unwrap();
    oxiline_core::record::start(&c, &a.id, chrono::Utc::now(), "2026-08-03").unwrap();
    assert!(oxiline_core::activities::delete_activity(&c, &a.id, false).is_err());   // conflict
    oxiline_core::activities::delete_activity(&c, &a.id, true).unwrap();              // force: records + activity gone
    assert!(oxiline_core::activities::list_activities(&c, false).unwrap().is_empty());
}
```
- [ ] **Step 2: Run — verify fail.**
- [ ] **Step 3: Implement:**
  - `record::resolve_plan_for(rec)`: find plans whose `plan_options` contain `rec.activity_id` AND (recurring mask matches `rec`'s local weekday) or (one-shot date == rec's local date), AND whose `[start, start+dur]` window overlaps `[rec.started_at, rec.ended_at|now]` (converted to local minutes). Return the best (most overlap) `PlanSlot` or `None`.
  - `record::list_records(activity_id?, from, to)`, `edit_record(id, started_at?, ended_at?)` (validate `ended_at > started_at`), `delete_record(id)`.
  - `activities::delete_activity(conn, id, force)`: count records for id; if >0 and !force ⇒ `Err(CoreError::Conflict{..})` (new variant `Conflict` in `error.rs`, code `conflict`); if force ⇒ txn `DELETE FROM records WHERE activity_id=id; DELETE FROM activities WHERE id=id;` (RESTRICT is the backstop). No records ⇒ plain delete.
- [ ] **Step 4: Run — verify pass.**
- [ ] **Step 5: Commit** — `feat(core): record list/edit/delete, plan resolution, activity refuse-with-history`.

---

### Task 7: `compliance` (neutral, rounded, today/week) + wire resolution into `slots_for_date`

**Files:** Modify `record.rs` (add `compliance`), `plan.rs` (call `resolve_plan_for` per slot) · Test: append `tests/record.rs`, `tests/plan.rs`

**Interfaces:** Produces `record::compliance(scope, now, today) -> Vec<Compliance>`. `current` now fills `today`.

- [ ] **Step 1: Failing tests** — append to `tests/record.rs` and `tests/plan.rs`:
```rust
// tests/record.rs append
#[test]
fn compliance_is_neutral_and_rounded() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(&c, oxiline_core::model::ActivityInput{name:Some("코딩".into()),
        target_minutes_weekly:Some(Some(1200)),..Default::default()}).unwrap(); // 20h/wk
    // record 42 min (rounds to 40) this week
    let s = Utc.with_ymd_and_hms(2026,8,3,9,0,0).unwrap();
    oxiline_core::record::start(&c, &a.id, s, "2026-08-03").unwrap();
    oxiline_core::record::stop(&c, s + chrono::Duration::minutes(42), "2026-08-03").unwrap();
    set_setting(&c, "record_rounding_minutes", "5");   // tiny test helper in tests/record.rs: oxiline_core::settings::set(conn, key, value).unwrap();
    let week = oxiline_core::record::compliance(&c, oxiline_core::model::Scope::Week, s, "2026-08-03").unwrap();
    let cm = week.iter().find(|x| x.activity.id==a.id).unwrap();
    assert_eq!(cm.recorded_seconds, 40*60);          // rounded
    assert!(matches!(cm.state, oxiline_core::model::ComplianceState::Under));
    assert_eq!(cm.ratio.unwrap(), (40.0*60.0)/(1200.0*60.0));
}

// tests/plan.rs append
```rust
use chrono::{TimeZone, Utc};

#[test]
fn slot_marked_resolved_after_record() {
    let (_f, c) = db();   // helper at top of tests/plan.rs (Task 1 harness)
    let a = oxiline_core::activities::create_activity(&c, activity_input("코딩")).unwrap();
    let p = oxiline_core::plan::create_plan(&c, plan_input(&a.id, 0b0000001, 9*60, 90)).unwrap();
    let now = Utc.with_ymd_and_hms(2026,8,3,9,10,0).unwrap();
    oxiline_core::record::start(&c, &a.id, now, "2026-08-03").unwrap();
    let slots = oxiline_core::plan::slots_for_date(&c, "2026-08-03").unwrap();
    let ours = slots.iter().find(|s| s.plan_id == p.id).unwrap();
    assert!(ours.is_resolved, "the slot should be resolved after a matching record was created in Task 7");
}
```

- [ ] **Step 2: Run — verify fail.**
- [ ] **Step 3: Implement:**
  - `compliance(scope, now, today)`: for each active activity, sum rounded record-overlap over the scope window (today = local date; week = `week_start_date` honoring `settings.week_starts_on`, reuse `reports::week_start_date`). `target_seconds` from the activity's daily/weekly target per scope. `ratio`, `remaining_seconds`, `state` (Under `<1.0`, Met `[1.0,1.05)`, Over `≥1.05`, Unbudgeted when target None). **All states share the activity hue** (no color logic here — that's GUI).
  - `current` now calls `compliance(Today,...)` to fill `RecordState.today`.
  - `plan::slots_for_date`: for each slot, set `is_resolved`/`resolved_by` via `record::resolve_plan_for` over the day's records (or a batched query; per-record call is fine for v1).
- [ ] **Step 4: Run — verify pass.**
- [ ] **Step 5: Commit** — `feat(core): neutral weekly compliance + plan resolution wiring`.

> **One-time `db.rs` change (this is the only plan task that touches `db.rs`):** wiring `V4_RECORD` into `migrations()` is required for V4 to apply at all. No later task modifies `db.rs`. Plan 2 (GUI) does not add further core migrations; if it does, it must re-extend `db.rs` the same way.
**End of Phase 1 — core is complete and fully tested.** Run `cargo test -p oxiline-core` green; `cargo build --workspace` passes.

---

## Phase 2 — CLI (wiring over Phase 1; follows `05-cli-spec.md`)

Each task: add a clap `#[derive(Subcommand)]` group in `cli.rs`, call the core fns, format via `output.rs` (+ `lang.rs` ko/en), add a `--json` smoke test asserting exit code + JSON shape. Tests use the existing CLI test pattern (check `crates/oxiline-cli/tests/` for the harness; if none, add an integration test that runs the binary via `assert_cmd` — add the dev-dep).

### Task 8: `oxiline activity` group
**Files:** Modify `cli.rs`, `output.rs`, `lang.rs` · Test: `crates/oxiline-cli/tests/activity.rs`
Subcommands: `add <NAME> [--daily M] [--weekly M] [--hue LABEL] [--icon NAME] [--category C]`, `list [--active-only]`, `show <ID|NAME>`, `edit <ID|NAME> [--name ..] [--daily M|--weekly M] (0 clears)`, `toggle <ID|NAME> --on|--off`, `rm <ID|NAME> [--force]`.
- `rm` with records → stderr `{"error":{"code":"conflict",...}}`, exit 1; `--force` proceeds.
- [ ] Steps: add clap group → wire to `activities::*` → `output` human + `--json` (emits `Activity`) → write smoke test (`activity add 코딩 --json` ⇒ exit 0, JSON has `id`) → run → commit `feat(cli): activity group`.

### Task 9: `oxiline record` group
**Files:** Modify `cli.rs`, `output.rs`, `lang.rs` · Test: `crates/oxiline-cli/tests/record_cli.rs`
Subcommands: `record` (bare: `RecordState` JSON), `record start <ACTIVITY> [--at ISO]`, `record stop`, `record log [--activity A] [--date D|--range F:T]`.
- `start --at` backdates the new record's `started_at` AND the prior record's `ended_at` to `t` (pass an overridden `now` into `record::start` — add an optional `at: Option<DateTime<Utc>>` param).
- [ ] Steps: add group → wire (`record::start/stop/current/list_records`) → output → smoke (`record start 코딩 --json` then `record --json` shows `active.activity.name=="코딩"`) → run → commit `feat(cli): record group`.

### Task 10: `oxiline plan` group
**Files:** Modify `cli.rs`, `output.rs`, `lang.rs` · Test: `crates/oxiline-cli/tests/plan_cli.rs`
Subcommands: `plan add [--at HH:MM] [--duration MIN] [--days mon,wed,..|weekdays|daily] --options A,B,C`, `plan list [--date D|--recurring]`, `plan edit <ID>`, `plan rm <ID>`.
- `--options` splits on comma → `activity_ids` (resolve each via `resolve_activity`).
- [ ] Steps: add group → wire (`plan::*`) → output → smoke (`plan add --at 11:00 --duration 120 --options 코딩,독서 --json` ⇒ exit 0, `options.len()==2`) → run → commit `feat(cli): plan group`.

### Task 11: `oxiline report` (activity compliance, neutral)
**Files:** Modify `cli.rs`, `output.rs`, `lang.rs` (and `lang.rs` copy rule: never "실패/깨짐/놓침"; over = "초과 +Xm")
Subcommands: `report [--week|--last N|--range F:T]` → `record::compliance(Week|range)` aggregated; human table + `--json`.
- [ ] Steps: add subcommand → wire to `record::compliance` → output (neutral copy) → smoke (`report --week --json` after a record ⇒ ratios present, no `is_done`) → run → commit `feat(cli): neutral activity report`.

**End of Plan 1.** Acceptance: `cargo test --workspace` green; `oxiline activity/record/plan/report` work headless against a temp DB (`OXILINE_DB_PATH=…`); `--json` outputs match the spec; exit codes per `05-cli-spec.md` §5.5.

---

## Plan 2 (next) — GUI roadmap (not in this plan)

Depends on Plan 1's tauri-specta bindings. Build on existing `DayTimeline.tsx`/`BlockView.tsx`/`@dnd-kit`/`cards.rs`; visual spec = `docs/superpowers/specs/2026-08-01-final-mockup.html`. Tasks (to detail in Plan 2):
1. Enlarge window (`tauri.conf.json` 420→~1180, minWidth) + 3-pane shell (sidebar/main/inspector) + container-query responsive collapse (drawers + bottom now-bar).
2. `commands.rs`: thin `#[tauri::command]`+`#[specta::specta]` wrappers over `record/plan/activities` (mirroring existing command style); regenerate `bindings.ts`.
3. Timetable toggle `[계획|실제|둘 다]` two-lane (plan dashed/hollow vs actual solid/filled) consuming `PlanSlot` + `Record`; now-line; recording card.
4. Sidebar: now-recording card (rounded elapsed, 주간/오늘) + activity library (drag-to-place, weekly bars).
5. Inspector: `[주간|오늘]` compliance + total + recent sessions.
6. Switcher panel (`tauri-nspanel`, `⌘⇧A`) + enriched peek HUD (`⌘⇧O`) + date dropdown popover.
7. Card planning dnd: drop 1 → single-option plan; drop/select 2+ → OR plan; resize/resize-handle (no vertical bar — dot labels).
8. Legacy demolition (drop `tasks`/`routine_blocks` tables + remove `tasks.rs`/`routines.rs`/`timeline.rs`/`reports.rs`/`cards.rs` legacy paths + their tests) — **after** the new UI fully replaces the old timeline. Separate migration `V5__drop_legacy.sql`.

---

## Self-Review

**1. Spec coverage** — §3.1 derived resolution ⇒ T6/T7; §3.2 OR plans ⇒ T4; §3.3 single-active ⇒ T5; §3.4 no-checkbox/completion ⇒ T6 (resolve) + no `is_done` anywhere; §3.5 rounding ⇒ T2 + T7; §3.6 neutral weekly compliance ⇒ T7; §4 schema ⇒ T1; §5 modules ⇒ T3–T7; §7 CLI ⇒ T8–T11; §10 tests (single-active, switch ts, resolution, completion, rounding, weekly, neutrality, partition, recurring, refuse-with-history) ⇒ covered across T5–T7 + T6 refuse. §11 edge cases (crash/stale, sleep, concurrent) ⇒ partly in core (current repair T5), partly GUI/Process (Plan 2). §1.1 legacy demolition ⇒ deferred to Plan 2 T8 (explicit). **Gap:** none material for Plan 1's scope; GUI/edge items correctly deferred to Plan 2.

**2. Placeholder scan** — no TBD/TODO/"add error handling" left. Task 4 Step 3 has a documented deferral (resolution set in T7), which is explicit wiring, not a placeholder.

**3. Type consistency** — `round_duration`, `Scope::{Today,Week}`, `ComplianceState::{Under,Met,Over,Unbudgeted}`, `ActivityInput`/`PlanInput` field names, `resolve_plan_for(&Record)->Option<PlanSlot>`, `delete_activity(force)`, `compliance(scope,now,today)` — used consistently across tasks. `current` returns `RecordState` with `today: Vec<Compliance>` (filled T7). `start` gains `at: Option<DateTime<Utc>>` in T9 (CLI `--at`) — add it to the T5 signature when implementing T9 (noted).

No issues remain. Plan is ready.
