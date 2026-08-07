//! Integration tests for `oxiline_core::tray_slots` (Task 1).
//!
//! Uses the `open_and_migrate_in_memory_for_tests` helper to spin up a fresh
//! migrated database per test (no `tempfile`, no `ensure_defaults` — V6 already
//! seeds the `tray_slots` row).

use oxiline_core::db::open_and_migrate_in_memory_for_tests;
use oxiline_core::model::{TraySlotKind, TraySlotPref};
use oxiline_core::settings;
use oxiline_core::tray_slots;

#[test]
fn resolve_returns_three_defaults_when_key_missing() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    // Remove the V6-seeded row so the resolver sees a missing key.
    conn.execute(
        "DELETE FROM settings WHERE key = 'tray_slots'",
        rusqlite::params![],
    )
    .unwrap();
    let r = tray_slots::resolve(&conn);
    assert_eq!(r.all.len(), 3);
    assert_eq!(r.all[0].kind, TraySlotKind::NowRecording);
    assert_eq!(r.all[1].kind, TraySlotKind::NowNext);
    assert_eq!(r.all[2].kind, TraySlotKind::StateDot);
    assert!(r.all[0].on);
    assert!(r.all[1].on);
    assert!(!r.all[2].on);
    assert_eq!(r.all[0].order, 0);
    assert_eq!(r.all[1].order, 1);
    assert_eq!(r.all[2].order, 2);
    assert!(r.any_enabled);
    // StateDot is off, so only 2 enabled.
    assert_eq!(r.enabled.len(), 2);
}

#[test]
fn resolve_drops_unknown_ids() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    let raw = serde_json::json!({
        "slots": [
            { "id": "now_recording", "on": true,  "order": 0 },
            { "id": "future_thing",  "on": true,  "order": 1 },
            { "id": "now_next",      "on": false, "order": 2 }
        ]
    });
    settings::set(&conn, "tray_slots", &raw).unwrap();

    let r = tray_slots::resolve(&conn);
    let kinds: Vec<_> = r.all.iter().map(|p| p.kind).collect();
    assert!(kinds.contains(&TraySlotKind::NowRecording));
    assert!(kinds.contains(&TraySlotKind::NowNext));
    // The unknown `future_thing` is dropped; the missing canonical kind
    // (StateDot) is filled in with `on = false` and `order` past the stored
    // max (per spec §4 forward-compat).
    assert_eq!(r.all.len(), 3);
    // The bogus id should not appear anywhere.
    let ids: Vec<_> = r
        .all
        .iter()
        .map(|p| tray_slots::slot_kind_to_id(p.kind))
        .collect();
    assert!(!ids.contains(&"future_thing"));
    // The filled entry is `on = false` with `order >= 2` (the stored max).
    let filled = r
        .all
        .iter()
        .find(|p| p.kind == TraySlotKind::StateDot)
        .expect("state_dot filled");
    assert!(!filled.on);
    assert!(filled.order >= 2);
}

#[test]
fn resolve_appends_defaults_for_missing_canonical_kinds() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    // Storage contains only one canonical kind; the resolver must fill the
    // other two with `on = false` and `order >= 1` (per spec §4).
    let raw = serde_json::json!({
        "slots": [
            { "id": "now_recording", "on": true, "order": 0 }
        ]
    });
    settings::set(&conn, "tray_slots", &raw).unwrap();

    let r = tray_slots::resolve(&conn);
    assert_eq!(r.all.len(), 3);
    let kinds: Vec<_> = r.all.iter().map(|p| p.kind).collect();
    assert!(kinds.contains(&TraySlotKind::NowRecording));
    assert!(kinds.contains(&TraySlotKind::NowNext));
    assert!(kinds.contains(&TraySlotKind::StateDot));
    // The stored entry is preserved verbatim.
    let recording = r
        .all
        .iter()
        .find(|p| p.kind == TraySlotKind::NowRecording)
        .unwrap();
    assert!(recording.on);
    assert_eq!(recording.order, 0);
    // The filled entries are off and use orders past the stored max (0).
    for kind in [TraySlotKind::NowNext, TraySlotKind::StateDot] {
        let filled = r
            .all
            .iter()
            .find(|p| p.kind == kind)
            .expect("filled entry");
        assert!(!filled.on);
        assert!(filled.order >= 1);
    }
    // No duplicates were introduced (the fill uses distinct ordinals).
    let mut orders: Vec<u32> = r.all.iter().map(|p| p.order).collect();
    orders.sort_unstable();
    let original = orders.clone();
    orders.dedup();
    assert_eq!(orders, original, "no duplicate orders after fill");
}

#[test]
fn resolve_sorts_by_order() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    // Inserted out of order; resolve must return them sorted by `order`.
    let raw = serde_json::json!({
        "slots": [
            { "id": "state_dot",     "on": false, "order": 2 },
            { "id": "now_recording", "on": true,  "order": 0 },
            { "id": "now_next",      "on": true,  "order": 1 }
        ]
    });
    settings::set(&conn, "tray_slots", &raw).unwrap();

    let r = tray_slots::resolve(&conn);
    let orders: Vec<_> = r.all.iter().map(|p| p.order).collect();
    assert_eq!(orders, vec![0, 1, 2]);
    let kinds: Vec<_> = r.all.iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TraySlotKind::NowRecording,
            TraySlotKind::NowNext,
            TraySlotKind::StateDot
        ]
    );
}

#[test]
fn resolve_normalizes_duplicate_orders_by_appending_defaults() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    // Two entries share order=0 → resolve must fall back to defaults.
    let raw = serde_json::json!({
        "slots": [
            { "id": "now_recording", "on": true,  "order": 0 },
            { "id": "now_next",      "on": true,  "order": 0 },
            { "id": "state_dot",     "on": true,  "order": 1 }
        ]
    });
    settings::set(&conn, "tray_slots", &raw).unwrap();

    let r = tray_slots::resolve(&conn);
    assert_eq!(r.all, tray_slots::defaults());
}

#[test]
fn save_then_resolve_round_trip() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    let prefs = vec![
        TraySlotPref {
            kind: TraySlotKind::StateDot,
            on: true,
            order: 0,
        },
        TraySlotPref {
            kind: TraySlotKind::NowRecording,
            on: false,
            order: 1,
        },
        TraySlotPref {
            kind: TraySlotKind::NowNext,
            on: true,
            order: 2,
        },
    ];
    tray_slots::save(&conn, &prefs).unwrap();

    let r = tray_slots::resolve(&conn);
    let got: Vec<_> = r.all.iter().map(|p| (p.kind, p.on, p.order)).collect();
    let expected: Vec<_> = prefs.iter().map(|p| (p.kind, p.on, p.order)).collect();
    assert_eq!(got, expected);
    // any_enabled + enabled list reflect what we saved.
    assert!(r.any_enabled);
    assert_eq!(r.enabled.len(), 2);
    // settings::get_tray_slots returns the same list (unsorted) for type-level
    // round-tripping.
    let raw = settings::get_tray_slots(&conn);
    assert_eq!(raw, prefs);
}

#[test]
fn snapshot_includes_tray_slots_field() {
    let conn = open_and_migrate_in_memory_for_tests().unwrap();
    let snap = settings::snapshot(&conn);
    // V6 migration seeds the canonical defaults.
    assert_eq!(snap.tray_slots, tray_slots::defaults());

    // Mutate then re-snapshot → the field reflects the persisted state.
    let prefs = vec![
        TraySlotPref {
            kind: TraySlotKind::NowRecording,
            on: false,
            order: 0,
        },
        TraySlotPref {
            kind: TraySlotKind::NowNext,
            on: false,
            order: 1,
        },
        TraySlotPref {
            kind: TraySlotKind::StateDot,
            on: true,
            order: 2,
        },
    ];
    tray_slots::save(&conn, &prefs).unwrap();
    let snap2 = settings::snapshot(&conn);
    assert_eq!(snap2.tray_slots, prefs);
}
