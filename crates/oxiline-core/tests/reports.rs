//! Integration tests for oxiline-core reports (habit streak / weekly report).
//! Mirrors tests/timeline.rs setup. created_at back-dating is done via a raw
//! UPDATE here ONLY — never touches tests/timeline.rs (spec §2.1 scope note).

use oxiline_core::model::{DayBreakdown, RoutineStreak, WeekReport};

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
    )
    .unwrap();
    let _: WeekReport = serde_json::from_str(
        r#"{"week_start":"2026-07-28","week_end":"2026-08-03","days":[],"totals":
            {"done":0,"skipped":0,"not_recorded":0,"upcoming":0},"completion_rate":null,
            "prev_completion_rate":null,"categories":[],"streaks":[]}"#,
    )
    .unwrap();
}
