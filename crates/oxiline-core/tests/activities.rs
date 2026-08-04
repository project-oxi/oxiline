//! Integration tests for the activities module (Task 3).
//!
//! Test harness follows `tests/record.rs`: a `db()` helper that opens an
//! ephemeral SQLite file via `oxiline_core::open_and_migrate`, then runs
//! `settings::ensure_defaults` so seeded settings are present. `:memory:`
//! databases do not work with `open_and_migrate` (it takes a `&Path`).

use rusqlite::Connection;
use tempfile::NamedTempFile;

fn db() -> (NamedTempFile, Connection) {
    let f = NamedTempFile::new().unwrap();
    let c = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&c).unwrap();
    (f, c)
}

#[test]
fn create_list_resolve_activity() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("코딩".into()),
            hue_label: Some("blue".into()),
            icon: None,
            category_id: None,
            target_minutes_daily: Some(Some(240)),
            target_minutes_weekly: Some(Some(1200)),
            is_active: None,
            sort_order: None,
        },
    )
    .unwrap();
    assert_eq!(a.name, "코딩");
    let listed = oxiline_core::activities::list_activities(&c, false).unwrap();
    assert_eq!(listed.len(), 1);
    let r = oxiline_core::activities::resolve_activity(&c, "코딩").unwrap(); // case-insensitive name
    assert_eq!(r.id, a.id);
    let r2 = oxiline_core::activities::resolve_activity(&c, &a.id).unwrap(); // by id
    assert_eq!(r2.id, a.id);
}

#[test]
fn resolve_activity_skips_inactive_duplicates() {
    // The brief (line 44) requires the name lookup to be scoped to active
    // activities only — name disambiguation across an active + an inactive
    // duplicate must surface the active one, not return Ambiguous.
    let (_f, c) = db();

    let input = |name: &str| oxiline_core::model::ActivityInput {
        name: Some(name.into()),
        hue_label: None,
        icon: None,
        category_id: None,
        target_minutes_daily: None,
        target_minutes_weekly: None,
        is_active: None,
        sort_order: None,
    };

    let active = oxiline_core::activities::create_activity(&c, input("독서")).unwrap();
    let inactive = oxiline_core::activities::create_activity(&c, input("독서")).unwrap();
    // Soft-archive the duplicate.
    oxiline_core::activities::update_activity(
        &c,
        &inactive.id,
        oxiline_core::model::ActivityInput {
            is_active: Some(false),
            ..input("독서")
        },
    )
    .unwrap();

    // Resolving by name must skip the inactive row and return the active one.
    let r = oxiline_core::activities::resolve_activity(&c, "독서").unwrap();
    assert_eq!(r.id, active.id);

    // The inactive row is still directly resolvable by id.
    let _ = oxiline_core::activities::get_activity(&c, &inactive.id).unwrap();
    // But not by name (the only active match is `active`).
    let r2 = oxiline_core::activities::resolve_activity(&c, "독서").unwrap();
    assert_eq!(r2.id, active.id);
}

#[test]
fn update_activity_target_tri_state() {
    // Tri-state semantics on the double-Option target fields are the
    // most bug-prone part of activities CRUD (budgeting data). This test
    // pins all three branches: set, clear, leave-unchanged.
    let (_f, c) = db();

    // Create with both targets set to verify the "set" branch leaves them
    // populated after a leave-unchanged update.
    let created = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("코딩".into()),
            hue_label: Some("blue".into()),
            icon: None,
            category_id: None,
            target_minutes_daily: Some(Some(240)),
            target_minutes_weekly: Some(Some(1200)),
            is_active: None,
            sort_order: None,
        },
    )
    .unwrap();
    assert_eq!(created.target_minutes_daily, Some(240));
    assert_eq!(created.target_minutes_weekly, Some(1200));

    // Set the daily target to a new value; leave weekly unchanged.
    //   target_minutes_daily = Some(Some(300))  -> set to 300
    //   target_minutes_weekly = None            -> unchanged (still 1200)
    let updated = oxiline_core::activities::update_activity(
        &c,
        &created.id,
        oxiline_core::model::ActivityInput {
            name: None,
            hue_label: None,
            icon: None,
            category_id: None,
            target_minutes_daily: Some(Some(300)),
            target_minutes_weekly: None,
            is_active: None,
            sort_order: None,
        },
    )
    .unwrap();
    assert_eq!(updated.target_minutes_daily, Some(300));
    assert_eq!(
        updated.target_minutes_weekly,
        Some(1200),
        "weekly target must NOT be cleared by None"
    );

    // Clear the daily target; leave weekly unchanged.
    //   target_minutes_daily = Some(None)  -> cleared to NULL
    //   target_minutes_weekly = None       -> unchanged
    let cleared = oxiline_core::activities::update_activity(
        &c,
        &created.id,
        oxiline_core::model::ActivityInput {
            name: None,
            hue_label: None,
            icon: None,
            category_id: None,
            target_minutes_daily: Some(None),
            target_minutes_weekly: None,
            is_active: None,
            sort_order: None,
        },
    )
    .unwrap();
    assert_eq!(
        cleared.target_minutes_daily, None,
        "Some(None) must clear, not leave-unchanged"
    );
    assert_eq!(cleared.target_minutes_weekly, Some(1200));
}

#[test]
fn delete_activity_refuses_with_history() {
    let (_f, c) = db();
    let a = oxiline_core::activities::create_activity(
        &c,
        oxiline_core::model::ActivityInput {
            name: Some("코딩".into()),
            ..Default::default()
        },
    )
    .unwrap();
    // Start a record so the activity has history; refuse-without-force must fail.
    let now = chrono::Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    oxiline_core::record::start(&c, &a.id, now, &today).unwrap();

    assert!(
        oxiline_core::activities::delete_activity(&c, &a.id, false).is_err(),
        "delete without --force must error when records exist"
    );

    // Force: records + activity gone in one transaction.
    oxiline_core::activities::delete_activity(&c, &a.id, true).unwrap();
    assert!(
        oxiline_core::activities::list_activities(&c, false)
            .unwrap()
            .is_empty()
    );
}
