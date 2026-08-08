//! Menu-bar tray slot preferences.
//!
//! Persisted as a single JSON row under settings key `tray_slots`. The shape
//! of the row is `{"slots": [{"id": ..., "on": bool, "order": u32}, ...]}` and
//! is decoded in [`crate::settings::parse_tray_slots`].
//!
//! The frontend (renderer + menu-bar tray) consumes the resolved slot list in
//! `order` ascending order. Unknown ids are dropped; if the stored data has
//! duplicate `order` values we fall back to the canonical defaults so the
//! renderer always sees a stable, unambiguous list.

use crate::error::Result;
use crate::model::{TraySlotKind, TraySlotPref};
use rusqlite::Connection;

/// The result of resolving the persisted tray-slot preferences against the
/// canonical set of slot kinds.
pub struct ResolvedSlots {
    /// All slots in display order, including disabled ones (so the UI can
    /// render the "off" placeholders).
    pub all: Vec<TraySlotPref>,
    /// Only the slots the user enabled, in display order.
    pub enabled: Vec<TraySlotPref>,
    /// Convenience flag for the renderer ("show anything at all?").
    pub any_enabled: bool,
}

const SLOT_KIND_IDS: [(TraySlotKind, &str); 3] = [
    (TraySlotKind::NowRecording, "now_recording"),
    (TraySlotKind::NowNext, "now_next"),
    (TraySlotKind::StateDot, "state_dot"),
];

/// Return the wire id used in the persisted JSON for `kind`.
pub fn slot_kind_to_id(kind: TraySlotKind) -> &'static str {
    match kind {
        TraySlotKind::NowRecording => "now_recording",
        TraySlotKind::NowNext => "now_next",
        TraySlotKind::StateDot => "state_dot",
    }
}

/// Reverse lookup: wire id → typed kind. Returns `None` for unknown ids so the
/// resolver can drop them silently.
pub fn slot_id_to_kind(id: &str) -> Option<TraySlotKind> {
    SLOT_KIND_IDS
        .iter()
        .find(|(_, sid)| *sid == id)
        .map(|(k, _)| *k)
}

/// Canonical v1 default: NowRecording=on, NowNext=on, StateDot=off,
/// order `[0, 1, 2]`.
pub fn defaults() -> Vec<TraySlotPref> {
    vec![
        TraySlotPref {
            kind: TraySlotKind::NowRecording,
            on: true,
            order: 0,
        },
        TraySlotPref {
            kind: TraySlotKind::NowNext,
            on: true,
            order: 1,
        },
        TraySlotPref {
            kind: TraySlotKind::StateDot,
            on: false,
            order: 2,
        },
    ]
}

/// Persist `prefs` to the `tray_slots` settings key.
pub fn save(conn: &Connection, prefs: &[TraySlotPref]) -> Result<()> {
    crate::settings::save_tray_slots(conn, prefs)
}
/// Load the persisted slot list, normalize it, and return a `ResolvedSlots`.
///
/// Normalization (per spec §4):
/// 1. Drop entries whose kind is unknown to this build.
/// 2. Fill missing canonical kinds with default values (`on = false`, `order =
///    current max + 1`) so that future builds adding a new slot kind don't
///    require a re-seed migration to surface it.
/// 3. Sort the remaining entries by `order` ascending. If duplicate `order`
///    values are detected after the fill, the stored list is replaced with
///    the canonical [`defaults`] — the renderer can't disambiguate otherwise.
pub fn resolve(conn: &Connection) -> ResolvedSlots {
    let raw = crate::settings::get_tray_slots(conn);
    let known = normalize(raw);
    let all = if has_duplicate_order(&known) {
        defaults()
    } else {
        let mut filled = fill_missing_canonical_kinds(known);
        filled.sort_by_key(|p| p.order);
        filled
    };
    let enabled = all.iter().filter(|p| p.on).cloned().collect();
    let any_enabled = all.iter().any(|p| p.on);
    ResolvedSlots {
        all,
        enabled,
        any_enabled,
    }
}

/// True if any two entries share the same `order` value.
fn has_duplicate_order(prefs: &[TraySlotPref]) -> bool {
    let mut seen = std::collections::HashSet::new();
    for p in prefs {
        if !seen.insert(p.order) {
            return true;
        }
    }
    false
}

/// Drop entries whose kind is unknown to this build.
fn normalize(prefs: Vec<TraySlotPref>) -> Vec<TraySlotPref> {
    let known: std::collections::HashSet<TraySlotKind> =
        SLOT_KIND_IDS.iter().map(|(k, _)| *k).collect();
    prefs
        .into_iter()
        .filter(|p| known.contains(&p.kind))
        .collect()
}

/// Append a default entry for every canonical kind missing from `prefs`,
/// using `on = false` and `order = (current max order) + 1` per appended
/// entry. The order is well-defined only in the absence of duplicate orders
/// (the caller checks that first).
fn fill_missing_canonical_kinds(prefs: Vec<TraySlotPref>) -> Vec<TraySlotPref> {
    let present: std::collections::HashSet<TraySlotKind> = prefs.iter().map(|p| p.kind).collect();
    let next_order = prefs.iter().map(|p| p.order).max().map_or(0, |m| m + 1);
    let mut out = prefs;
    let mut next = next_order;
    for (kind, _) in SLOT_KIND_IDS {
        if !present.contains(&kind) {
            out.push(TraySlotPref {
                kind,
                on: false,
                order: next,
            });
            next += 1;
        }
    }
    out
}
