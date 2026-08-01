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
/// Tiny convenience wrapper around `settings::set` that takes a raw JSON value
/// (Task 7 brief: "wrapping `oxiline_core::settings::set(conn, key, value).unwrap()`").
/// Used by `compliance_is_neutral_and_rounded` to seed the rounding increment.
fn set_setting(conn: &Connection, key: &str, raw_json: &str) {
    let value: serde_json::Value = serde_json::from_str(raw_json)
        .unwrap_or_else(|_| serde_json::Value::String(raw_json.to_string()));
    settings::set(conn, key, &value).unwrap();
}

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

#[test]
fn resolve_links_record_to_plan() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(&c, activity_input("코딩")).unwrap();
    // Recurring plan: Monday 09:00–10:30 (start=540, duration=90), option "코딩".
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 90,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a.id.clone()],
        },
    )
    .unwrap();
    // 2026-08-03 is a Monday — start the record at 09:10Z, inside the window.
    let now = Utc.with_ymd_and_hms(2026, 8, 3, 9, 10, 0).unwrap();
    oxiline_core::record::start(&c, &a.id, now, "2026-08-03").unwrap();
    let rec = oxiline_core::record::current(&c, now, "2026-08-03")
        .unwrap()
        .active
        .unwrap()
        .record;
    let slot = oxiline_core::record::resolve_plan_for(&c, &rec).unwrap();
    assert_eq!(slot.unwrap().plan_id, p.id);
}
#[test]
fn compliance_is_neutral_and_rounded() {
    // Task 7 brief: a 42-minute weekly record must surface as 40 min (rounded
    // to the 5-min increment) with `state = Under` and a ratio that matches
    // `recorded / target`. No color or hue bias lives in core.
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("코딩".into()),
            target_minutes_weekly: Some(Some(1200)),
            ..Default::default()
        },
    )
    .unwrap();
    // Record 42 min (rounds to 40) this week.
    let s = Utc.with_ymd_and_hms(2026, 8, 3, 9, 0, 0).unwrap();
    oxiline_core::record::start(&c, &a.id, s, "2026-08-03").unwrap();
    oxiline_core::record::stop(&c, s + chrono::Duration::minutes(42), "2026-08-03").unwrap();
    set_setting(&c, "record_rounding_minutes", "5");
    let week =
        oxiline_core::record::compliance(&c, oxiline_core::model::Scope::Week, s, "2026-08-03")
            .unwrap();
    let cm = week.iter().find(|x| x.activity.id == a.id).unwrap();
    assert_eq!(cm.recorded_seconds, 40 * 60); // rounded (half-up: 2520 → 2400)
    assert!(matches!(
        cm.state,
        oxiline_core::model::ComplianceState::Under
    ));
    assert_eq!(cm.ratio.unwrap(), (40.0 * 60.0) / (1200.0 * 60.0));
    // Task 7 advisory: also assert `Scope::Today` so the live-state path used
    // by `current`/`stop` (which call `compliance(Scope::Today, …)` to fill
    // `RecordState.today`) can't regress silently. The space-vs-T bound bug
    // surfaced here, not in the Week test — `2026-08-03` falls inside the
    // same Monday week either way (so Weekly still rounded to 40 min), but
    // Today only counted record when bounds used the T separator.
    let today =
        oxiline_core::record::compliance(&c, oxiline_core::model::Scope::Today, s, "2026-08-03")
            .unwrap();
    let cm_today = today.iter().find(|x| x.activity.id == a.id).unwrap();
    assert_eq!(
        cm_today.recorded_seconds,
        40 * 60,
        "Scope::Today must surface the 42-min record as 40 min (rounded)"
    );
    // Also lock `current`'s downstream path: `RecordState.today` is what
    // the CLI `oxiline record` and the GUI sidebar consume.
    let state = oxiline_core::record::current(&c, s, "2026-08-03").unwrap();
    assert_eq!(
        state.today.len(),
        1,
        "`current(...).today` should contain exactly one Compliance for the active activity"
    );
    assert_eq!(state.today[0].recorded_seconds, 40 * 60);
}
