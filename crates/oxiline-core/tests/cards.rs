//! Integration tests for `cards::suggest` and the mask=0 template semantics.
//!
//! Mirrors the `tests/timeline.rs` ephemeral-DB pattern. Timestamps are
//! back-dated explicitly so the "most-recent row per title" aggregation is
//! deterministic regardless of how fast the tests run.

use oxiline_core::{cards, reports, routines, tasks, timeline};
use rusqlite::params;
use tempfile::NamedTempFile;

fn fresh_db() -> (NamedTempFile, rusqlite::Connection) {
    let f = NamedTempFile::new().unwrap();
    let conn = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&conn).unwrap();
    (f, conn)
}

/// Force a task's `updated_at` so recency ordering is deterministic.
fn set_task_updated(conn: &rusqlite::Connection, id: &str, ts: &str) {
    conn.execute(
        "UPDATE tasks SET updated_at = ?1 WHERE id = ?2",
        params![ts, id],
    )
    .unwrap();
}

fn routine(title: &str, mask: u8) -> routines::NewRoutineBlock {
    routines::NewRoutineBlock {
        title: title.into(),
        start_minute: 540,
        duration_minute: 30,
        weekday_mask: mask,
        category_id: Some("cat_work".into()),
        effective_from: None,
        effective_until: None,
        notes: None,
    }
}

fn task(title: &str, cat: &str, dur: u16) -> tasks::NewTask {
    tasks::NewTask {
        date: Some("2026-07-30".into()),
        title: title.into(),
        category_id: Some(cat.into()),
        start_minute: Some(600),
        duration_minute: Some(dur),
        notes: None,
    }
}

#[test]
fn history_collapses_to_most_recent_signature_per_title() {
    let (_f, conn) = fresh_db();
    let t1 = tasks::create(&conn, task("스탠드업", "cat_work", 15)).unwrap();
    set_task_updated(&conn, &t1.id, "2026-07-29T01:00:00Z");
    let t2 = tasks::create(&conn, task("스탠드업", "cat_personal", 30)).unwrap();
    set_task_updated(&conn, &t2.id, "2026-07-31T01:00:00Z");

    let s = cards::suggest(&conn, 100).unwrap();
    let m = s
        .iter()
        .filter(|x| x.title == "스탠드업")
        .collect::<Vec<_>>();
    assert_eq!(m.len(), 1, "title de-duplicated to one entry");
    assert!(!m[0].is_template);
    // newest row wins → cat_personal / 30
    assert_eq!(m[0].category_id.as_deref(), Some("cat_personal"));
    assert_eq!(m[0].duration_minute, Some(30));
    assert!(m[0].template_id.is_none());
}

#[test]
fn mask_zero_is_a_template_that_never_schedules() {
    let (_f, conn) = fresh_db();
    let b = routines::create(
        &conn,
        routines::NewRoutineBlock {
            title: "회의 템플릿".into(),
            start_minute: 600,
            duration_minute: 45,
            weekday_mask: 0,
            category_id: Some("cat_work".into()),
            effective_from: None,
            effective_until: None,
            notes: Some("agenda".into()),
        },
    )
    .unwrap();
    assert!(routines::is_template(&b));

    // It must NOT bleed into any date's timeline (the core safety property).
    for d in ["2026-07-27", "2026-07-30", "2026-08-02"] {
        let tl = timeline::get_timeline_for_date(&conn, d).unwrap();
        assert!(
            tl.iter().all(|i| i.title != "회의 템플릿"),
            "template leaked into timeline on {d}"
        );
    }

    // But it DOES surface in suggestions as a curated template.
    let s = cards::suggest(&conn, 100).unwrap();
    let m = s.iter().find(|x| x.title == "회의 템플릿").expect("template suggested");
    assert!(m.is_template);
    assert_eq!(m.template_id.as_deref(), Some(b.id.as_str()));
    assert_eq!(m.duration_minute, Some(45));
    assert_eq!(m.notes.as_deref(), Some("agenda"));
}

#[test]
fn template_title_shadows_history_with_same_name() {
    let (_f, conn) = fresh_db();
    routines::create(
        &conn,
        routines::NewRoutineBlock {
            title: "운동".into(),
            start_minute: 420,
            duration_minute: 30,
            weekday_mask: 0,
            category_id: Some("cat_health".into()),
            effective_from: None,
            effective_until: None,
            notes: None,
        },
    )
    .unwrap();
    let t = tasks::create(&conn, task("운동", "cat_work", 60)).unwrap();
    set_task_updated(&conn, &t.id, "2026-07-31T01:00:00Z");

    let s = cards::suggest(&conn, 100).unwrap();
    let m = s.iter().find(|x| x.title == "운동").expect("suggested");
    assert!(m.is_template, "template wins over history for the same title");
    assert_eq!(m.duration_minute, Some(30), "uses template duration, not history's");
}

#[test]
fn recurring_routine_titles_seed_history() {
    let (_f, conn) = fresh_db();
    let b = routines::create(&conn, routine("아침 루틴", 0b1111111)).unwrap();
    let s = cards::suggest(&conn, 100).unwrap();
    let m = s.iter().find(|x| x.title == "아침 루틴").expect("recurring suggested");
    assert!(!m.is_template);
    assert_eq!(m.template_id.as_deref(), Some(b.id.as_str()));
}

#[test]
fn templates_rank_above_history() {
    let (_f, conn) = fresh_db();
    routines::create(&conn, routine("템플릿A", 0)).unwrap();
    let t = tasks::create(&conn, task("과거A", "cat_work", 30)).unwrap();
    set_task_updated(&conn, &t.id, "2026-07-31T01:00:00Z");

    let s = cards::suggest(&conn, 100).unwrap();
    let ia = s.iter().position(|x| x.title == "템플릿A").unwrap();
    let ib = s.iter().position(|x| x.title == "과거A").unwrap();
    assert!(ia < ib, "templates ordered before history");
}

#[test]
fn updating_weekday_mask_to_zero_turns_routine_into_template() {
    let (_f, conn) = fresh_db();
    let b = routines::create(&conn, routine("X", 0b1111111)).unwrap();
    assert!(!routines::is_template(&b));
    let updated = routines::update(
        &conn,
        &b.id,
        routines::RoutineUpdate {
            title: None,
            start_minute: None,
            duration_minute: None,
            weekday_mask: Some(0),
            category_id: None,
            notes: None,
        },
    )
    .unwrap();
    assert!(routines::is_template(&updated));
}

#[test]
fn templates_are_excluded_from_streaks() {
    let (_f, conn) = fresh_db();
    routines::create(&conn, routine("노터치", 0)).unwrap();
    let streaks = reports::routine_streaks(&conn, "2026-07-30").unwrap();
    assert!(
        streaks.iter().all(|s| s.title != "노터치"),
        "template must not appear in routine streaks"
    );
}

#[test]
fn empty_db_yields_empty_suggestions() {
    let (_f, conn) = fresh_db();
    assert!(cards::suggest(&conn, 100).unwrap().is_empty());
}
