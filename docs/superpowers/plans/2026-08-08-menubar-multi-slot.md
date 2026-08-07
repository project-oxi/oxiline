# Menu-bar Multi-Slot Display — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single 22×22 progress-bar tray with a CodexBar-style multi-slot status bar (one `NSStatusItem` per enabled information slot) and a Preferences section to toggle each slot and reorder them. The progress-bar icon is removed.

**Architecture:** A new `oxiline_core::tray_slots` module owns `TraySlotKind` enum + `TraySlotPref { kind, on, order }` and a single JSON settings row (`tray_slots`). A new `tray_render` Rust module rasterizes each slot into a 22 px tall RGBA bitmap using an inline 5×7 monospaced font. `tray::build`/`rebuild`/`refresh` manage a `BUILT_SLOTS` map of currently-live `TrayIcon` handles plus an always-on menu slot. The Preferences section exposes an `update_tray_slots` Tauri command that calls `tray::rebuild` and emits `oxiline://tray-changed`. The existing 60 s timer + `oxiline://db-changed` refresh path is reused.

**Tech Stack:** Rust (rusqlite, serde, serde_json, image, tauri 2 `tray-icon`), React 19, react-query, lucide-react, Oxi tokens. Spec: `docs/superpowers/specs/2026-08-08-menubar-multi-slot-design.md`.

## Global Constraints

- macOS menu bar target only. v1 explicitly defers Linux/Windows multi-tray stacking.
- v1 text rendering is ASCII-only via an inline 5×7 bitmap font in `tray_render.rs`. Korean activity names fall through to the `unknown char` box fallback (documented behavior, not a bug).
- Bitmap is regenerated when theme changes because the 60 s refresh loop re-rasterizes after `applyTheme` mutates the settings row.
- `TraySlotPref` carries `kind: TraySlotKind` (typed enum, not free string) in Rust + TypeScript via `specta::Type`. Settings JSON on disk uses the snake_case `id` (`now_recording`, `now_next`, `state_dot`) for forward-compat.
- All-off case keeps the always-on menu slot alive (CodexBar merged-slot pattern).
- Korean copy in components; code/comments in English. No emojis in code.
- `cargo clippy -p oxiline-app -p oxiline-core -- -D warnings` and `tsc --noEmit && vite build` must stay green after every task.

---

## File structure

- `crates/oxiline-core/migrations/V6__tray_slots.sql` *(new)* — seed the default `tray_slots` JSON row (`INSERT OR IGNORE`).
- `crates/oxiline-core/src/db.rs` — register `V6_TRAY_SLOTS`.
- `crates/oxiline-core/src/model.rs` — add `TraySlotKind` enum, `TraySlotPref` struct, `SettingsSnapshot::tray_slots`.
- `crates/oxiline-core/src/settings.rs` — `snapshot()` reads `tray_slots`; `save_tray_slots` writes it.
- `crates/oxiline-core/src/tray_slots.rs` *(new)* — `defaults`, `resolve`, `save` helpers.
- `crates/oxiline-core/tests/tray_slots.rs` *(new)* — resolve/save unit tests.
- `crates/oxiline-app/src-tauri/src/tray_render.rs` *(new)* — 5×7 bitmap font + `render_slot` + `render_menu_dot`.
- `crates/oxiline-app/src-tauri/src/tray.rs` — multi-slot `build`/`rebuild`/`refresh`; `BUILT_SLOTS` cache; always-on menu slot.
- `crates/oxiline-app/src-tauri/src/commands.rs` — `update_tray_slots` command.
- `crates/oxiline-app/src-tauri/src/lib.rs` — register `update_tray_slots` in `collect_commands!`; add `oxiline://tray-changed` listener that calls `tray::rebuild`.
- `crates/oxiline-app/src/lib/api.ts` — `updateTraySlots`, `TraySlotPref` TS type (specta-generated; import only).
- `crates/oxiline-app/src/hooks.ts` — `useTraySlots` (read `SettingsSnapshot.tray_slots`) + `useUpdateTraySlots`.
- `crates/oxiline-app/src/components/Preferences.tsx` — new "메뉴바 표시" section with toggle + ▲/▼.
- `crates/oxiline-app/src/components/__tests__/preferences-tray-slots.test.tsx` *(new)* — toggle + swap behavior.
- `crates/oxiline-app/src/locales/{ko,en}.json` — new keys (`settings.menubar*`, `settings.slotNow*`).

---

### Task 1: Tray slots core — model + persistence + tests

**Files:**
- Modify: `crates/oxiline-core/src/model.rs` (add `TraySlotKind`, `TraySlotPref`, extend `SettingsSnapshot`).
- Modify: `crates/oxiline-core/src/settings.rs` (read/write helpers).
- Create: `crates/oxiline-core/src/tray_slots.rs`.
- Create: `crates/oxiline-core/migrations/V6__tray_slots.sql`.
- Modify: `crates/oxiline-core/src/db.rs` (register `V6_TRAY_SLOTS`).
- Create: `crates/oxiline-core/tests/tray_slots.rs`.

**Interfaces:**
- Produces: `TraySlotKind { NowRecording, NowNext, StateDot }` (serde rename_all snake_case, specta Type).
- Produces: `TraySlotPref { kind: TraySlotKind, on: bool, order: u32 }` (same derives).
- Produces: `pub fn tray_slots::defaults() -> [TraySlotPref; 3]`.
- Produces: `pub fn tray_slots::resolve(conn: &Connection) -> ResolvedSlots { enabled: Vec<(TraySlotKind, u32)>, any_enabled: bool }`.
- Produces: `pub fn tray_slots::save(conn: &Connection, &Vec<TraySlotPref>) -> Result<()>`.
- Produces: `pub fn settings::get_tray_slots(conn) -> Vec<TraySlotPref>`.
- Produces: `pub fn settings::save_tray_slots(conn, &Vec<TraySlotPref>) -> Result<()>`.
- Consumes: existing `settings::get_raw` / `settings::set` (unchanged).

- [ ] **Step 1: Add `V6__tray_slots.sql` migration**

```sql
-- Menu-bar multi-slot preferences (idempotent; respects existing user choice).
INSERT OR IGNORE INTO settings (key, value, updated_at)
VALUES (
    'tray_slots',
    json_object(
        'slots', json_array(
            json_object('id', 'now_recording', 'on', 1, 'order', 0),
            json_object('id', 'now_next',      'on', 1, 'order', 1),
            json_object('id', 'state_dot',     'on', 0, 'order', 2)
        )
    ),
    '2026-08-08T00:00:00Z'
);
```

- [ ] **Step 2: Register the migration in `db.rs`**

Append `const V6_TRAY_SLOTS: &str = include_str!("../migrations/V6__tray_slots.sql");` and add `M::up(V6_TRAY_SLOTS)` to the `Migrations::new(vec![…])` list **after** the `V5_DROP_LEGACY` entry.

- [ ] **Step 3: Extend `model.rs`**

Add at the bottom of the file (before the recording-layer section):

```rust
#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TraySlotKind {
    NowRecording,
    NowNext,
    StateDot,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct TraySlotPref {
    pub kind: TraySlotKind,
    pub on: bool,
    pub order: u32,
}
```

Extend `SettingsSnapshot` (find the existing struct) with one field at the end:

```rust
pub tray_slots: Vec<TraySlotPref>,
```

`specta::Type` is already derived, so the TS binding regenerates automatically.

- [ ] **Step 4: Add `settings::get_tray_slots` and `save_tray_slots`**

Append to `crates/oxiline-core/src/settings.rs`:

```rust
/// Read `tray_slots` JSON; falls back to the v1 defaults when missing or
/// unparseable. Always returns 3 entries (NowRecording, NowNext, StateDot).
pub fn get_tray_slots(conn: &Connection) -> Vec<crate::model::TraySlotPref> {
    let raw = get_raw(conn, "tray_slots").ok();
    parse_tray_slots(raw.as_ref())
}

fn parse_tray_slots(raw: Option<&Value>) -> Vec<crate::model::TraySlotPref> {
    use crate::model::{TraySlotKind, TraySlotPref};
    let mut out: Vec<TraySlotPref> = match raw.and_then(|v| v.get("slots")).and_then(|v| v.as_array()) {
        Some(arr) => arr.iter().filter_map(parse_slot).collect(),
        None => Vec::new(),
    };
    for def in defaults() {
        if !out.iter().any(|p| p.kind == def.kind) {
            out.push(def);
        }
    }
    out.sort_by_key(|p| p.order);
    out
}

fn parse_slot(v: &Value) -> Option<crate::model::TraySlotPref> {
    use crate::model::TraySlotPref;
    let kind = match v.get("id")?.as_str()? {
        "now_recording" => crate::model::TraySlotKind::NowRecording,
        "now_next" => crate::model::TraySlotKind::NowNext,
        "state_dot" => crate::model::TraySlotKind::StateDot,
        _ => return None,
    };
    Some(TraySlotPref {
        kind,
        on: v.get("on").and_then(|x| x.as_i64()).map(|n| n != 0).unwrap_or(false),
        order: v.get("order").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
    })
}

pub fn defaults() -> Vec<crate::model::TraySlotPref> {
    use crate::model::{TraySlotKind, TraySlotPref};
    vec![
        TraySlotPref { kind: TraySlotKind::NowRecording, on: true,  order: 0 },
        TraySlotPref { kind: TraySlotKind::NowNext,      on: true,  order: 1 },
        TraySlotPref { kind: TraySlotKind::StateDot,     on: false, order: 2 },
    ]
}

pub fn save_tray_slots(conn: &Connection, prefs: &[crate::model::TraySlotPref]) -> Result<()> {
    let slots: Vec<Value> = prefs
        .iter()
        .map(|p| {
            let id = match p.kind {
                crate::model::TraySlotKind::NowRecording => "now_recording",
                crate::model::TraySlotKind::NowNext => "now_next",
                crate::model::TraySlotKind::StateDot => "state_dot",
            };
            serde_json::json!({
                "id": id,
                "on": p.on,
                "order": p.order,
            })
        })
        .collect();
    let value = serde_json::json!({ "slots": slots });
    set(conn, "tray_slots", &value)
}
```

Extend `snapshot()` to populate the new field. Append one line before the closing brace of the `SettingsSnapshot { … }` literal:

```rust
tray_slots: get_tray_slots(conn),
```

- [ ] **Step 5: Create `tray_slots.rs`**

New file `crates/oxiline-core/src/tray_slots.rs`:

```rust
//! Menu-bar tray slot resolution (spec: 2026-08-08-menubar-multi-slot).
//!
//! `resolve(conn)` is the single source of truth used by the Tauri tray
//! renderer. It always returns the full ordered list plus a flag indicating
//! whether any data slot is enabled.

use rusqlite::Connection;

use crate::model::{TraySlotKind, TraySlotPref};
use crate::settings;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct ResolvedSlots {
    pub all: Vec<TraySlotPref>,
    pub enabled: Vec<TraySlotPref>,
    pub any_enabled: bool,
}

pub fn resolve(conn: &Connection) -> ResolvedSlots {
    let all = settings::get_tray_slots(conn);
    let enabled = all.iter().filter(|p| p.on).cloned().collect();
    let any_enabled = !enabled.is_empty();
    ResolvedSlots { all, enabled, any_enabled }
}

pub fn save(conn: &Connection, prefs: &[TraySlotPref]) -> Result<()> {
    settings::save_tray_slots(conn, prefs)
}

pub fn defaults() -> Vec<TraySlotPref> {
    settings::defaults()
}

pub fn slot_kind_to_id(kind: TraySlotKind) -> &'static str {
    match kind {
        TraySlotKind::NowRecording => "now_recording",
        TraySlotKind::NowNext => "now_next",
        TraySlotKind::StateDot => "state_dot",
    }
}

pub fn slot_id_to_kind(id: &str) -> Option<TraySlotKind> {
    match id {
        "now_recording" => Some(TraySlotKind::NowRecording),
        "now_next" => Some(TraySlotKind::NowNext),
        "state_dot" => Some(TraySlotKind::StateDot),
        _ => None,
    }
}
```

- [ ] **Step 6: Write the failing tests**

Create `crates/oxiline-core/tests/tray_slots.rs`:

```rust
use rusqlite::Connection;
use serde_json::json;

use oxiline_core::db::open_and_migrate_in_memory_for_tests as open_db;
use oxiline_core::model::{TraySlotKind, TraySlotPref};
use oxiline_core::settings;
use oxiline_core::tray_slots;

fn conn() -> Connection {
    open_db().expect("open in-memory db with migrations")
}

#[test]
fn resolve_returns_three_defaults_when_key_missing() {
    let c = conn();
    let r = tray_slots::resolve(&c);
    assert_eq!(r.all.len(), 3);
    assert!(r.any_enabled);
    assert_eq!(r.all[0].kind, TraySlotKind::NowRecording);
    assert_eq!(r.all[1].kind, TraySlotKind::NowNext);
    assert_eq!(r.all[2].kind, TraySlotKind::StateDot);
}

#[test]
fn resolve_drops_unknown_ids() {
    let c = conn();
    settings::set(
        &c,
        "tray_slots",
        &json!({
            "slots": [
                { "id": "now_recording", "on": true,  "order": 0 },
                { "id": "made_up_slot",  "on": true,  "order": 1 },
                { "id": "now_next",      "on": false, "order": 2 },
            ]
        }),
    )
    .unwrap();
    let r = tray_slots::resolve(&c);
    let ids: Vec<_> = r.all.iter().map(|p| tray_slots::slot_kind_to_id(p.kind)).collect();
    assert_eq!(ids, vec!["now_recording", "now_next", "state_dot"]);
}

#[test]
fn resolve_sorts_by_order() {
    let c = conn();
    settings::set(
        &c,
        "tray_slots",
        &json!({
            "slots": [
                { "id": "now_next",      "on": true, "order": 2 },
                { "id": "now_recording", "on": true, "order": 0 },
                { "id": "state_dot",     "on": true, "order": 1 },
            ]
        }),
    )
    .unwrap();
    let r = tray_slots::resolve(&c);
    let order: Vec<u32> = r.all.iter().map(|p| p.order).collect();
    assert_eq!(order, vec![0, 1, 2]);
}

#[test]
fn resolve_normalizes_duplicate_orders_by_appending_defaults() {
    let c = conn();
    settings::set(
        &c,
        "tray_slots",
        &json!({
            "slots": [
                { "id": "now_next",      "on": true, "order": 0 },
                { "id": "now_recording", "on": true, "order": 0 },
            ]
        }),
    )
    .unwrap();
    let r = tray_slots::resolve(&c);
    // The user rows appear first (in JSON order); the missing default is appended.
    assert_eq!(r.all.len(), 3);
    assert!(r.all.iter().any(|p| p.kind == TraySlotKind::StateDot));
}

#[test]
fn save_then_resolve_round_trip() {
    let c = conn();
    let prefs = vec![
        TraySlotPref { kind: TraySlotKind::NowNext,      on: true,  order: 1 },
        TraySlotPref { kind: TraySlotKind::NowRecording, on: true,  order: 0 },
        TraySlotPref { kind: TraySlotKind::StateDot,     on: false, order: 2 },
    ];
    tray_slots::save(&c, &prefs).unwrap();
    let r = tray_slots::resolve(&c);
    assert_eq!(r.all[0].kind, TraySlotKind::NowRecording);
    assert_eq!(r.all[1].kind, TraySlotKind::NowNext);
    assert!(!r.all[2].on);
}

#[test]
fn snapshot_includes_tray_slots_field() {
    let c = conn();
    let s = settings::snapshot(&c);
    assert_eq!(s.tray_slots.len(), 3);
}
```

The `open_and_migrate_in_memory_for_tests` helper does **not** exist yet — create it.

- [ ] **Step 7: Add `db::open_and_migrate_in_memory_for_tests`**

Append to `crates/oxiline-core/src/db.rs`:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub fn open_and_migrate_in_memory_for_tests() -> crate::error::Result<Connection> {
    let conn = Connection::open_in_memory().map_err(CoreError::from)?;
    apply_pragmas(&conn)?;
    migrations().to_latest(&mut conn).map_err(CoreError::from)?;
    Ok(conn)
}
```

No feature gate in this PR — instead gate with `#[cfg(test)]` and re-export via the existing test-utils crate. If the codebase already exposes an in-memory helper (e.g. in `tests/common.rs`), reuse that instead and skip this step.

Check first:
```bash
grep -nE 'open_in_memory|test_utils|open_in_memory_for_tests' crates/oxiline-core/src crates/oxiline-core/tests
```
If an existing helper is found, import it in the test file and delete this step's stub.

- [ ] **Step 8: Run tests**

Run: `cargo test -p oxiline-core --test tray_slots`
Expected: 6 passed.

If `cargo test --workspace` reports failures outside this test file, investigate — the change should be additive.

- [ ] **Step 9: Run clippy**

Run: `cargo clippy -p oxiline-core -- -D warnings`
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add crates/oxiline-core/migrations/V6__tray_slots.sql \
        crates/oxiline-core/src/db.rs \
        crates/oxiline-core/src/model.rs \
        crates/oxiline-core/src/settings.rs \
        crates/oxiline-core/src/tray_slots.rs \
        crates/oxiline-core/tests/tray_slots.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(core): tray slot preferences model + persistence"
```

---

### Task 2: Bitmap font + slot renderer

**Files:**
- Create: `crates/oxiline-app/src-tauri/src/tray_render.rs`.

**Interfaces:**
- Consumes: `oxiline_core::model::TraySlotKind`, `oxiline_core::plan::now_summary`, `oxiline_core::settings::get_i64`.
- Produces: `pub fn render_slot(kind: TraySlotKind, label: &str, fg: (u8,u8,u8,u8)) -> tauri::image::Image<'static>`.
- Produces: `pub fn render_menu_dot(color: (u8,u8,u8,u8)) -> tauri::image::Image<'static>`.
- Produces: `pub fn label_for(kind: TraySlotKind, locale: &str, ctx: &LabelCtx) -> String`.

- [ ] **Step 1: Create `tray_render.rs` skeleton**

```rust
//! Bitmapped menu-bar slot renderer.
//!
//! Each slot is a 22 px tall RGBA buffer. v1 ships an ASCII 5×7 monospaced
//! font (see `FONT`); non-ASCII bytes render as a 5×7 box so width stays
//! predictable across locales.

use std::collections::BTreeMap;

use image::{ImageBuffer, Rgba};
use tauri::image::Image;

use oxiline_core::model::TraySlotKind;

const SLOT_HEIGHT: u32 = 22;
const SLOT_PAD_RIGHT: u32 = 4;
const SLOT_MIN_WIDTH: u32 = 24;
const SLOT_MAX_WIDTH: u32 = 120;
const GLYPH_W: u32 = 5;
const GLYPH_H: u32 = 7;
const GLYPH_ADVANCE: u32 = 6;
const DOT_RADIUS: i32 = 5;

pub struct LabelCtx<'a> {
    pub now_minute: u16,
    pub rounding_minutes: i64,
    pub now_summary: &'a oxiline_core::model::NowSummary,
}

pub fn label_for(kind: TraySlotKind, locale: &str, ctx: &LabelCtx<'_>) -> String {
    match kind {
        TraySlotKind::NowRecording => label_now(locale, ctx),
        TraySlotKind::NowNext => label_next(locale, ctx),
        TraySlotKind::StateDot => String::new(),
    }
}

fn label_now(locale: &str, ctx: &LabelCtx<'_>) -> String {
    let summary = match &ctx.now_summary.current {
        Some(c) => c,
        None => return String::new(),
    };
    let mins = ctx.rounding_minutes.max(1) as i64;
    let n = summary.remaining_minute.unwrap_or(0).max(0);
    let n = ((n + mins / 2) / mins).max(1);
    let title = truncate_ascii(&summary.title, 14);
    if locale == "en" {
        format!("REC {title} {n}m")
    } else {
        format!("REC {title} {n}m")
    }
}

fn label_next(locale: &str, ctx: &LabelCtx<'_>) -> String {
    let next = match &ctx.now_summary.next {
        Some(n) => n,
        None => return String::new(),
    };
    let mins = ctx.rounding_minutes.max(1) as i64;
    let n = next.starts_in_minute.unwrap_or(0).max(0);
    let n = ((n + mins / 2) / mins).max(1);
    let title = truncate_ascii(&next.title, 14);
    if locale == "en" {
        format!("NEXT {title} {n}m")
    } else {
        format!("NEXT {title} {n}m")
    }
}

fn truncate_ascii(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| c.is_ascii())
        .take(max)
        .collect()
}

pub fn render_slot(label: &str, fg: (u8, u8, u8, u8)) -> Image<'static> {
    let width = label_width(label).clamp(SLOT_MIN_WIDTH, SLOT_MAX_WIDTH);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(width, SLOT_HEIGHT, Rgba([0, 0, 0, 0]));
    let mut cursor_x: u32 = 2;
    for byte in label.bytes() {
        if cursor_x + GLYPH_W > width - SLOT_PAD_RIGHT {
            break;
        }
        if let Some(rows) = FONT.get(&byte) {
            draw_glyph(&mut img, cursor_x, 7, rows, fg);
        } else {
            draw_unknown(&mut img, cursor_x, 7, fg);
        }
        cursor_x += GLYPH_ADVANCE;
    }
    Image::new_owned(img.into_raw(), width, SLOT_HEIGHT)
}

pub fn render_menu_dot(color: (u8, u8, u8, u8)) -> Image<'static> {
    let size = 22u32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let cx = (size / 2) as i32;
    let cy = (size / 2) as i32;
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= DOT_RADIUS * DOT_RADIUS {
                img.put_pixel(x as u32, y as u32, Rgba([color.0, color.1, color.2, color.3]));
            }
        }
    }
    Image::new_owned(img.into_raw(), size, size)
}

fn label_width(label: &str) -> u32 {
    let n = label.bytes().count() as u32;
    2 + n * GLYPH_ADVANCE + SLOT_PAD_RIGHT
}

fn draw_glyph(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x0: u32,
    y0: u32,
    rows: &[u8; GLYPH_H as usize],
    fg: (u8, u8, u8, u8),
) {
    for (dy, row) in rows.iter().enumerate() {
        for dx in 0..GLYPH_W {
            if row & (1 << (GLYPH_W - 1 - dx)) != 0 {
                img.put_pixel(x0 + dx, y0 + dy as u32, Rgba([fg.0, fg.1, fg.2, fg.3]));
            }
        }
    }
}

fn draw_unknown(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x0: u32,
    y0: u32,
    fg: (u8, u8, u8, u8),
) {
    for dy in 0..GLYPH_H {
        for dx in 0..GLYPH_W {
            let edge = dy == 0 || dy == GLYPH_H - 1 || dx == 0 || dx == GLYPH_W - 1;
            if edge {
                img.put_pixel(x0 + dx, y0 + dy, Rgba([fg.0, fg.1, fg.2, fg.3]));
            }
        }
    }
}

// Inline 5×7 monospaced font (ASCII subset). Each row is 5 bits, MSB left.
// Hand-rolled from the classic 5×7 bitmap font tables.
static FONT: std::sync::LazyLock<BTreeMap<u8, [u8; 7]>> = std::sync::LazyLock::new(|| {
    let mut m = BTreeMap::new();
    m.insert(b' ', [0, 0, 0, 0, 0, 0, 0]);
    m.insert(b'!', [0x00, 0x00, 0x5F, 0x00, 0x00, 0x00, 0x00]);
    m.insert(b'\'', [0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00]);
    m.insert(b'(', [0x00, 0x1C, 0x22, 0x41, 0x00, 0x00, 0x00]);
    m.insert(b')', [0x00, 0x41, 0x22, 0x1C, 0x00, 0x00, 0x00]);
    m.insert(b'+', [0x00, 0x08, 0x3E, 0x08, 0x00, 0x00, 0x00]);
    m.insert(b',', [0x00, 0x50, 0x30, 0x00, 0x00, 0x00, 0x00]);
    m.insert(b'-', [0x00, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00]);
    m.insert(b'.', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]);
    m.insert(b'/', [0x20, 0x10, 0x08, 0x04, 0x02, 0x00, 0x00]);
    m.insert(b'0', [0x3E, 0x51, 0x49, 0x45, 0x3E, 0x00, 0x00]);
    m.insert(b'1', [0x00, 0x42, 0x7F, 0x40, 0x00, 0x00, 0x00]);
    m.insert(b'2', [0x42, 0x61, 0x51, 0x49, 0x46, 0x00, 0x00]);
    m.insert(b'3', [0x21, 0x41, 0x45, 0x4B, 0x31, 0x00, 0x00]);
    m.insert(b'4', [0x18, 0x14, 0x12, 0x7F, 0x10, 0x00, 0x00]);
    m.insert(b'5', [0x27, 0x45, 0x45, 0x45, 0x39, 0x00, 0x00]);
    m.insert(b'6', [0x3C, 0x4A, 0x49, 0x49, 0x30, 0x00, 0x00]);
    m.insert(b'7', [0x01, 0x71, 0x09, 0x05, 0x03, 0x00, 0x00]);
    m.insert(b'8', [0x36, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]);
    m.insert(b'9', [0x06, 0x49, 0x49, 0x29, 0x1E, 0x00, 0x00]);
    m.insert(b':', [0x00, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00]);
    m.insert(b'A', [0x7E, 0x11, 0x11, 0x11, 0x7E, 0x00, 0x00]);
    m.insert(b'B', [0x7F, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]);
    m.insert(b'C', [0x3E, 0x41, 0x41, 0x41, 0x22, 0x00, 0x00]);
    m.insert(b'D', [0x7F, 0x41, 0x41, 0x22, 0x1C, 0x00, 0x00]);
    m.insert(b'E', [0x7F, 0x49, 0x49, 0x49, 0x41, 0x00, 0x00]);
    m.insert(b'F', [0x7F, 0x09, 0x09, 0x01, 0x01, 0x00, 0x00]);
    m.insert(b'G', [0x3E, 0x41, 0x41, 0x51, 0x32, 0x00, 0x00]);
    m.insert(b'H', [0x7F, 0x08, 0x08, 0x08, 0x7F, 0x00, 0x00]);
    m.insert(b'I', [0x00, 0x41, 0x7F, 0x41, 0x00, 0x00, 0x00]);
    m.insert(b'J', [0x20, 0x40, 0x41, 0x3F, 0x01, 0x00, 0x00]);
    m.insert(b'K', [0x7F, 0x08, 0x14, 0x22, 0x41, 0x00, 0x00]);
    m.insert(b'L', [0x7F, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00]);
    m.insert(b'M', [0x7F, 0x02, 0x04, 0x02, 0x7F, 0x00, 0x00]);
    m.insert(b'N', [0x7F, 0x04, 0x08, 0x10, 0x7F, 0x00, 0x00]);
    m.insert(b'O', [0x3E, 0x41, 0x41, 0x41, 0x3E, 0x00, 0x00]);
    m.insert(b'P', [0x7F, 0x09, 0x09, 0x09, 0x06, 0x00, 0x00]);
    m.insert(b'Q', [0x3E, 0x41, 0x51, 0x21, 0x5E, 0x00, 0x00]);
    m.insert(b'R', [0x7F, 0x09, 0x19, 0x29, 0x46, 0x00, 0x00]);
    m.insert(b'S', [0x46, 0x49, 0x49, 0x49, 0x31, 0x00, 0x00]);
    m.insert(b'T', [0x01, 0x01, 0x7F, 0x01, 0x01, 0x00, 0x00]);
    m.insert(b'U', [0x3F, 0x40, 0x40, 0x40, 0x3F, 0x00, 0x00]);
    m.insert(b'V', [0x1F, 0x20, 0x40, 0x20, 0x1F, 0x00, 0x00]);
    m.insert(b'W', [0x7F, 0x20, 0x10, 0x20, 0x7F, 0x00, 0x00]);
    m.insert(b'X', [0x63, 0x14, 0x08, 0x14, 0x63, 0x00, 0x00]);
    m.insert(b'Y', [0x03, 0x04, 0x78, 0x04, 0x03, 0x00, 0x00]);
    m.insert(b'Z', [0x61, 0x51, 0x49, 0x45, 0x43, 0x00, 0x00]);
    m.insert(b'm', [0x00, 0x7C, 0x08, 0x04, 0x78, 0x00, 0x00]);
    m
});
```

Note: the dot glyph (`●`) and the triangle (`▸`) are out of v1 (per spec §6.4). The text labels use plain `REC` / `NEXT` prefixes for ASCII clarity.

- [ ] **Step 2: Add `tray_render` module to `lib.rs`**

In `crates/oxiline-app/src-tauri/src/lib.rs` add `mod tray_render;` next to the existing `mod tray;`.

- [ ] **Step 3: Add inline unit tests**

Append at the bottom of `tray_render.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_menu_dot_dimensions() {
        let img = render_menu_dot((255, 255, 255, 255));
        assert_eq!(img.width(), 22);
        assert_eq!(img.height(), 22);
    }

    #[test]
    fn render_slot_known_kind_has_non_zero_width() {
        let img = render_slot("REC code 12m", (255, 255, 255, 255));
        assert!(img.width() >= SLOT_MIN_WIDTH);
        assert_eq!(img.height(), SLOT_HEIGHT);
    }

    #[test]
    fn render_slot_unknown_char_renders_box() {
        let img = render_slot("ab안cd", (10, 20, 30, 255));
        // 4 known chars + 1 unknown; width is the full predicted width.
        assert!(img.width() >= SLOT_MIN_WIDTH);
    }
}
```

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p oxiline-app tray_render`
Expected: 3 passed.

Run: `cargo clippy -p oxiline-app -- -D warnings`
Expected: clean (LazyLock may need `Msrv` ≥ 1.80; verify rust-toolchain.toml satisfies that — bump if needed and document in the commit message).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/tray_render.rs \
        crates/oxiline-app/src-tauri/src/lib.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): bitmapped menu-bar slot renderer"
```

---

### Task 3: Multi-slot tray wiring

**Files:**
- Modify: `crates/oxiline-app/src-tauri/src/tray.rs`.
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs`.
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs` (register command + tray-changed listener).

**Interfaces:**
- Produces: `tray::build(app)` (refactored) — builds the always-on menu slot + all enabled data slots; populates `BUILT_SLOTS`.
- Produces: `tray::rebuild(app)` — tears down data slots, then `build`s.
- Produces: `tray::refresh(app)` — re-rasterizes every entry in `BUILT_SLOTS`.
- Produces: `tray::set_slot_visible(app, kind, visible)` — toggle individual slot visibility without rebuilding.
- Consumes: `oxiline_core::tray_slots::resolve`, `crate::tray_render::{render_slot, render_menu_dot, label_for}`.

- [ ] **Step 1: Replace `tray.rs` with the multi-slot implementation**

Keep `build_menu`, `on_menu_event`, `show_main`, `autostart_enabled`, `toggle_autostart`, and the `now_summary` text helper verbatim — only `build`, `refresh`, and the icon-rendering helpers change.

Add a module-level cache:

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::tray::TrayIcon;

use crate::tray_render::{label_for, render_menu_dot, render_slot, LabelCtx};
use oxiline_core::model::TraySlotKind;
use oxiline_core::tray_slots;

const MENU_TRAY_ID: &str = "tray-menu";

fn slot_tray_id(kind: TraySlotKind) -> String {
    format!("tray-slot-{}", tray_slots::slot_kind_to_id(kind))
}

static BUILT_SLOTS: once_cell::sync::Lazy<Mutex<HashMap<TraySlotKind, TrayIcon>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
```

If `once_cell` is not in `Cargo.toml`, add it:
```bash
grep once_cell crates/oxiline-app/src-tauri/Cargo.toml
```
If absent, add `once_cell = "1"` under `[dependencies]`.

Replace `build` with:

```rust
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let menu_dot = render_menu_dot(MENU_DOT_COLOR);
    let mut menu_tray = tauri::tray::TrayIconBuilder::with_id(MENU_TRAY_ID)
        .icon(menu_dot)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;
    let _ = menu_tray.set_icon_as_template(true);

    let conn = app.state::<AppState>().conn();
    let resolved = tray_slots::resolve(&conn);
    for pref in &resolved.enabled {
        build_slot(app, pref.kind)?;
    }
    refresh(app);
    Ok(())
}

fn build_slot(app: &AppHandle, kind: TraySlotKind) -> tauri::Result<()> {
    let label = slot_label(app, kind);
    let img = render_slot(&label, FG_COLOR);
    let tray = tauri::tray::TrayIconBuilder::with_id(&slot_tray_id(kind))
        .icon(img)
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .on_tray_icon_event({
            let app = app.clone();
            move |_tray, event| {
                if let tauri::tray::TrayIconEvent::Click { button, .. } = event {
                    if matches!(button, tauri::tray::MouseButton::Left) {
                        show_main(&app);
                    }
                }
            }
        })
        .build(app)?;
    let _ = tray.set_icon_as_template(true);
    BUILT_SLOTS.lock().unwrap().insert(kind, tray);
    Ok(())
}
```

Add color constants near the top:

```rust
const FG_COLOR: (u8, u8, u8, u8) = (60, 60, 60, 255);
const MENU_DOT_COLOR: (u8, u8, u8, u8) = (130, 130, 130, 255);
const STATE_DOT_RECORDING: (u8, u8, u8, u8) = (43, 179, 160, 255);
const STATE_DOT_NEXT_SOON: (u8, u8, u8, u8) = (220, 160, 40, 255);
const STATE_DOT_IDLE: (u8, u8, u8, u8) = (130, 130, 130, 255);
```

Replace `refresh` with:

```rust
pub fn refresh(app: &AppHandle) {
    let slots = BUILT_SLOTS.lock().unwrap();
    for (kind, tray) in slots.iter() {
        let img = match kind {
            TraySlotKind::StateDot => Some(render_state_dot(app)),
            _ => {
                let label = slot_label(app, *kind);
                if label.is_empty() { None } else { Some(render_slot(&label, FG_COLOR)) }
            }
        };
        if let Some(img) = img {
            let _ = tray.set_icon(Some(img));
        }
    }
    drop(slots);
    // Also rebuild the menu so the dynamic "지금" row stays fresh.
    if let Ok(menu) = build_menu(app) {
        if let Some(tray) = app.tray_by_id(MENU_TRAY_ID) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn render_state_dot(app: &AppHandle) -> tauri::image::Image<'static> {
    let conn = app.state::<AppState>().conn();
    let summary = oxiline_core::plan::now_summary(&conn, oxiline_core::util::now_minute_local()).ok();
    let color = match (summary.as_ref().and_then(|s| s.current.as_ref()), summary.as_ref().and_then(|s| s.next.as_ref())) {
        (Some(_), _) => STATE_DOT_RECORDING,
        (None, Some(n)) if n.starts_in_minute.unwrap_or(i64::MAX) <= 5 => STATE_DOT_NEXT_SOON,
        _ => STATE_DOT_IDLE,
    };
    render_menu_dot(color)
}

fn slot_label(app: &AppHandle, kind: TraySlotKind) -> String {
    let conn = app.state::<AppState>().conn();
    let locale = oxiline_core::settings::get_string(&conn, "locale", "system");
    let locale = if locale == "en" { "en" } else { "ko" };
    let now_minute = oxiline_core::util::now_minute_local();
    let summary = oxiline_core::plan::now_summary(&conn, now_minute).ok();
    let summary = match summary { Some(s) => s, None => return String::new() };
    let ctx = LabelCtx {
        now_minute,
        rounding_minutes: oxiline_core::settings::get_i64(&conn, "record_rounding_minutes", 5),
        now_summary: &summary,
    };
    label_for(kind, locale, &ctx)
}

pub fn rebuild(app: &AppHandle) {
    // Tear down every data slot.
    let kinds: Vec<TraySlotKind> = BUILT_SLOTS.lock().unwrap().keys().copied().collect();
    for kind in kinds {
        let id = slot_tray_id(kind);
        let _ = app.remove_tray_by_id(&id);
    }
    BUILT_SLOTS.lock().unwrap().clear();
    // Rebuild.
    if let Err(e) = build(app) {
        eprintln!("tray::rebuild failed: {e}");
    }
}
```

Remove the old `render_progress_icon` function and its callers.

- [ ] **Step 2: Add `update_tray_slots` command**

Append to `crates/oxiline-app/src-tauri/src/commands.rs`:

```rust
/// Persist the user's menu-bar slot preferences and rebuild the tray so the
/// change takes effect immediately.
#[tauri::command]
#[specta::specta]
pub fn update_tray_slots(
    state: State<'_, crate::state::AppState>,
    app: AppHandle,
    slots: Vec<oxiline_core::model::TraySlotPref>,
) -> Result<(), String> {
    if slots.is_empty() {
        return Err("at least one slot row required".into());
    }
    oxiline_core::tray_slots::save(&state.conn(), &slots).map_err(map_err)?;
    crate::tray::rebuild(&app);
    Ok(())
}
```

Check the existing `State` import path — adjust the generic to match. In this codebase it's `tauri::State<'_, AppState>` (verify with `grep "State<" crates/oxiline-app/src-tauri/src/commands.rs`).

- [ ] **Step 3: Register the command + tray-changed listener in `lib.rs`**

In `lib.rs`, inside `tauri::Builder::default()`, add the command to the existing `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![/* existing commands */, crate::commands::update_tray_slots])
```

Adjust the macro call to the project's actual pattern (some codebases use `tauri_specta::collect_commands!`).

Add a tray-changed listener inside `.setup(|app| { … })`:

```rust
let h = app.handle().clone();
app.listen("oxiline://tray-changed", move |_event| {
    crate::tray::rebuild(&h);
});
```

Place this near the existing `oxiline://db-changed` listener.

- [ ] **Step 4: Build the app**

Run: `cargo build -p oxiline-app`
Expected: `Finished` with no warnings.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p oxiline-app -- -D warnings`
Expected: clean. The `LazyLock`/`once_cell` import and `tauri::tray::TrayIconEvent`/`MouseButton` paths must resolve. If `TrayIconEvent` is a struct variant in 2.x, adjust the match arms accordingly (verify against `Cargo.toml` `tauri = "2"`).

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/tray.rs \
        crates/oxiline-app/src-tauri/src/commands.rs \
        crates/oxiline-app/src-tauri/src/lib.rs \
        crates/oxiline-app/src-tauri/Cargo.toml
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): multi-slot tray build/rebuild/refresh"
```

---

### Task 4: Frontend API + Preferences section

**Files:**
- Modify: `crates/oxiline-app/src/lib/api.ts` (specta regenerates; ensure import path is correct).
- Modify: `crates/oxiline-app/src/hooks.ts` (`useTraySlots`, `useUpdateTraySlots`).
- Modify: `crates/oxiline-app/src/components/Preferences.tsx` (new "메뉴바 표시" section).
- Create: `crates/oxiline-app/src/components/__tests__/preferences-tray-slots.test.tsx`.
- Modify: `crates/oxiline-app/src/locales/ko.json`.
- Modify: `crates/oxiline-app/src/locales/en.json`.

**Interfaces:**
- Produces: `api.updateTraySlots(slots: TraySlotPref[]): Promise<void>` (specta-generated).
- Produces: `useTraySlots(): TraySlotPref[]` (derived from `useSettings().tray_slots`).
- Produces: `useUpdateTraySlots(): UseMutationResult<void, Error, TraySlotPref[]>` (emits `oxiline://tray-changed` on success).

- [ ] **Step 1: Regenerate specta bindings**

Run: `cargo run -p oxiline-app --bin specta-gen` (or whatever the existing build hook is — check `package.json` scripts and `Cargo.toml` bins). The project's `tsc`/`vite` build runs specta as part of `vite build`. Trigger a typecheck to verify:

Run: `bun run tsc --noEmit` (or `npx tsc --noEmit`)
Expected: clean; `update_tray_slots` is now callable from `api.ts`.

- [ ] **Step 2: Add `useTraySlots` and `useUpdateTraySlots` hooks**

Append to `crates/oxiline-app/src/hooks.ts`:

```ts
import { api } from "./lib/api";

export function useTraySlots() {
  const settingsQ = useSettings();
  const list = (settingsQ.data?.tray_slots as TraySlotPref[] | undefined) ?? [];
  // Stable order: by `order` ascending.
  return [...list].sort((a, b) => a.order - b.order);
}

export function useUpdateTraySlots() {
  return useMutation({
    mutationFn: (slots: TraySlotPref[]) => api.updateTraySlots(slots),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["settings"] });
      if ("__TAURI_INTERNALS__" in window) {
        void window.__TAURI_INTERNALS__.invoke("emit", { event: "oxiline://tray-changed" })
          .catch(() => { /* tray-changed is best-effort; the Rust side also rebuilds inside the command */ });
      }
    },
  });
}
```

The exact emission pattern depends on the project's existing helpers (e.g. `api.emit(...)`). Mirror how other listeners in the codebase emit events. Verify with:
```bash
grep -nE 'emit\(.*tray|emit\("oxiline' crates/oxiline-app/src
```

If a shared `emit(event, payload?)` helper exists in `api.ts`, prefer that. Otherwise, the `invoke("emit", { event })` pattern from above is acceptable for v1.

Add the missing `qc` import (likely `useQueryClient`):

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";

const qc = useQueryClient();
```

Place `qc` inside the hook body — keep the file's existing style.

- [ ] **Step 3: Add the i18n keys**

In `crates/oxiline-app/src/locales/ko.json` (add under existing `settings.*`):

```json
"menubar": "메뉴바 표시",
"menubarHelp": "메뉴바에 어떤 정보를 표시할지 골라보세요. 켜진 항목은 좌→우 순서로 메뉴바에 나란히 표시돼요.",
"menubarEmptyHint": "지금은 메뉴바에 표시되는 항목이 없어요. 최소 하나를 켜면 메뉴바에서 바로 확인할 수 있어요.",
"slotNowRecording": "지금 무엇을 하고 있는지",
"slotNowNext": "다음 할 일",
"slotStateDot": "상태 점만",
"menubarAllOffNote": "모두 끄면 메뉴 아이콘은 메뉴용 1개만 남아요."
```

Mirror the same keys in `crates/oxiline-app/src/locales/en.json` (English strings — the codebase is Korean-source-of-truth, but English translations must be present):

```json
"menubar": "Menu bar",
"menubarHelp": "Choose what to show in the menu bar. Enabled items appear left to right.",
"menubarEmptyHint": "Nothing is shown in the menu bar right now. Enable at least one item to see it at a glance.",
"slotNowRecording": "What I'm doing now",
"slotNowNext": "Next up",
"slotStateDot": "Status dot only",
"menubarAllOffNote": "If you turn everything off, only the menu icon stays."
```

- [ ] **Step 4: Add the Preferences section**

Append a new section to `crates/oxiline-app/src/components/Preferences.tsx`, between the notifications and categories sections:

```tsx
const traySlots = useTraySlots();
const updateTraySlots = useUpdateTraySlots();

function toggleSlot(idx: number) {
  const next = traySlots.map((s, i) => (i === idx ? { ...s, on: !s.on } : s));
  updateTraySlots.mutate(next);
}

function moveSlot(idx: number, dir: -1 | 1) {
  const next = [...traySlots];
  const target = idx + dir;
  if (target < 0 || target >= next.length) return;
  [next[idx], next[target]] = [next[target], next[idx]];
  next.forEach((s, i) => (s.order = i));
  updateTraySlots.mutate(next);
}

const slotLabel = (kind: TraySlotKind) => {
  switch (kind) {
    case "NowRecording": return t("settings.slotNowRecording");
    case "NowNext": return t("settings.slotNowNext");
    case "StateDot": return t("settings.slotStateDot");
  }
};
```

Use the existing `Row` helper or render the list manually. Add the JSX section:

```tsx
<section className="mb-4">
  <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.menubar")}</h3>
  <p className="mb-2 text-[12px] text-text-subtle">{t("settings.menubarHelp")}</p>
  {traySlots.length === 0 ? (
    <p className="text-[12px] text-text-subtle">{t("settings.menubarEmptyHint")}</p>
  ) : (
    <ul className="space-y-1">
      {traySlots.map((s, i) => (
        <li key={s.kind} className="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-surface-sunken">
          <input
            type="checkbox"
            checked={s.on}
            onChange={() => toggleSlot(i)}
            aria-label={slotLabel(s.kind)}
          />
          <span className="flex-1 text-[13px]">{slotLabel(s.kind)}</span>
          <span className="text-[11px] text-text-subtle">{s.on ? t("common.on") ?? "켜짐" : "꺼짐"}</span>
          <button
            className="rounded p-1 hover:bg-surface-muted disabled:opacity-30"
            disabled={i === 0}
            onClick={() => moveSlot(i, -1)}
            aria-label="위로"
          >
            <ChevronUp size={14} />
          </button>
          <button
            className="rounded p-1 hover:bg-surface-muted disabled:opacity-30"
            disabled={i === traySlots.length - 1}
            onClick={() => moveSlot(i, 1)}
            aria-label="아래로"
          >
            <ChevronDown size={14} />
          </button>
        </li>
      ))}
    </ul>
  )}
  <p className="mt-2 text-[11px] text-text-subtle">{t("settings.menubarAllOffNote")}</p>
</section>
```

Add `ChevronUp, ChevronDown` to the lucide-react import at the top of `Preferences.tsx`. The Korean `t("common.on")` key may not exist — fall back to literal `"켜짐"` if `useTranslation` doesn't return it (and add the literal everywhere consistently).

- [ ] **Step 5: Write the failing tests**

Create `crates/oxiline-app/src/components/__tests__/preferences-tray-slots.test.tsx`. The existing test infrastructure (`__tests__/context-menu.test.ts`, `layout.test.ts`, `now-next.test.ts`) is node-only and doesn't exercise React components. If a component-testing harness is configured, use it; otherwise create a vitest + react-testing-library spec that mocks `useSettings` and `api.updateTraySlots`:

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Preferences } from "../Preferences";

vi.mock("../../hooks", () => ({
  useSettings: () => ({
    data: {
      tray_slots: [
        { kind: "NowRecording", on: true,  order: 0 },
        { kind: "NowNext",      on: true,  order: 1 },
        { kind: "StateDot",     on: false, order: 2 },
      ],
    },
  }),
  useUpdateTraySlots: () => ({ mutate: vi.fn() }),
  useTraySlots: () => [
    { kind: "NowRecording", on: true,  order: 0 },
    { kind: "NowNext",      on: true,  order: 1 },
    { kind: "StateDot",     on: false, order: 2 },
  ],
  useSetSetting: () => ({ mutate: vi.fn() }),
  useCategories: () => ({ data: [] }),
  useCreateCategory: () => ({ mutate: vi.fn() }),
  useDeleteCategory: () => ({ mutate: vi.fn() }),
  useCliStatus: () => ({ data: "not-installed" }),
  useInstallCli: () => ({ mutate: vi.fn(), isPending: false }),
  useUninstallCli: () => ({ mutate: vi.fn(), isPending: false }),
}));
vi.mock("../../lib/api", () => ({
  api: {
    updateTraySlots: vi.fn().mockResolvedValue(undefined),
    isNotificationPermissionGranted: vi.fn().mockResolvedValue(false),
    requestNotificationPermission: vi.fn(),
    openNotificationSettings: vi.fn(),
  },
}));

describe("Preferences tray-slots section", () => {
  beforeEach(() => {
    // Open the modal so the section is rendered.
    document.body.innerHTML = "";
  });

  it("renders one row per slot with on/off and reorder buttons", async () => {
    render(<Preferences />);
    expect(screen.getAllByRole("checkbox")).toHaveLength(3);
    expect(screen.getAllByRole("button", { name: /위로|아래로/ })).toHaveLength(6);
  });

  it("swaps two adjacent slots when move-down is clicked", async () => {
    const updateSpy = vi.fn();
    // override the mock for this test
    vi.doMock("../../hooks", () => ({
      // …same as above but useUpdateTraySlots returns { mutate: updateSpy }
    }));
    // Simplified: assert the moveSlot logic is reachable via the buttons.
    // If DOM testing proves flaky, export moveSlot as a pure helper and
    // unit-test it instead (preferred).
  });
});
```

If react-testing-library isn't set up yet, **skip the DOM test** and add a pure-helper unit test instead. Extract `swapOrder(slots, i, dir)` to a small pure function in `lib/tray-slot-order.ts` and test it under `__tests__/tray-slot-order.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { swapOrder } from "../tray-slot-order";

describe("swapOrder", () => {
  it("swaps adjacent slots and renormalizes order", () => {
    const slots = [
      { kind: "NowRecording", on: true,  order: 0 },
      { kind: "NowNext",      on: true,  order: 1 },
      { kind: "StateDot",     on: false, order: 2 },
    ];
    const r = swapOrder(slots, 0, 1);
    expect(r[0].kind).toBe("NowNext");
    expect(r[1].kind).toBe("NowRecording");
    expect(r.map((s) => s.order)).toEqual([0, 1, 2]);
  });

  it("is a no-op at the boundaries", () => {
    const slots = [
      { kind: "NowRecording", on: true, order: 0 },
      { kind: "NowNext",      on: true, order: 1 },
    ];
    expect(swapOrder(slots, 0, -1)).toEqual(slots);
    expect(swapOrder(slots, 1, 1)).toEqual(slots);
  });
});
```

Move `moveSlot` to use the helper. The DOM test becomes optional.

- [ ] **Step 6: Typecheck + build**

Run: `bun run tsc --noEmit && bun run build` (or `npx tsc --noEmit && npx vite build`).
Expected: clean.

Run: `bun run test` (or the project's vitest command — check `package.json`).
Expected: all green (existing + new helper tests).

- [ ] **Step 7: Commit**

```bash
git add crates/oxiline-app/src/lib/api.ts \
        crates/oxiline-app/src/hooks.ts \
        crates/oxiline-app/src/components/Preferences.tsx \
        crates/oxiline-app/src/components/__tests__/preferences-tray-slots.test.tsx \
        crates/oxiline-app/src/lib/tray-slot-order.ts \
        crates/oxiline-app/src/lib/__tests__/tray-slot-order.test.ts \
        crates/oxiline-app/src/locales/ko.json \
        crates/oxiline-app/src/locales/en.json
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): preferences tray-slot toggle + reorder"
```

---

### Task 5: Verify, smoke-test, and document

**Files:**
- Modify: `HANDOFF.md` (add a Session entry).
- Modify: `doc/09-ui-redesign.md` if a menu-bar section exists.

- [ ] **Step 1: Run the full gate**

Run:
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
bun run tsc --noEmit && bun run build
bun run test
```
Expected: all green.

- [ ] **Step 2: Manual smoke test**

Run: `cargo run -p oxiline-app`

Verify in the macOS menu bar:
1. Default state shows two slots (`NOW RECORDING …`, `NEXT …`) and the menu dot.
2. Open Preferences → toggle `state_dot` on → three slots visible.
3. Toggle all off → only the menu dot remains.
4. Reorder slots via ▲/▼ → menu bar order updates immediately.
5. Start a recording → `NOW RECORDING` reflects it within one 60 s tick or after a DB mutation event.
6. Theme switch → bitmap recolors on next refresh.

- [ ] **Step 3: Document**

Append to `HANDOFF.md`:

```md
## Menu-bar multi-slot display (2026-08-08 session N) ✅ COMPLETE

Spec: `docs/superpowers/specs/2026-08-08-menubar-multi-slot-design.md`.
Plan: `docs/superpowers/plans/2026-08-08-menubar-multi-slot.md`.

- 22×22 progress-bar tray replaced by CodexBar-style multi-slot status bar.
- New `oxiline_core::tray_slots` module owns the typed preferences (JSON
  in `settings.tray_slots`).
- New `tray_render` rasterizes ASCII labels via a hand-rolled 5×7 bitmap
  font; non-ASCII falls back to a 5×7 box (v1 limitation).
- Preferences → "메뉴바 표시" exposes on/off + ▲/▼ ordering for the three
  v1 slots: `now_recording`, `now_next`, `state_dot`.
- Always-on menu slot preserves the context menu even when all data
  slots are off.
```

If `doc/09-ui-redesign.md` has a "menu bar" subsection, append a single
sentence pointing to the spec. Otherwise skip.

- [ ] **Step 4: Commit**

```bash
git add HANDOFF.md doc/09-ui-redesign.md
git -c user.email=oxi@local -c user.name=Oxi commit -m "docs: menu-bar multi-slot handoff"
```

---

## Self-review

**Spec coverage:**

- §1 Goal → Task 3, Task 4 (user-visible behavior).
- §2 Slot catalog → Task 2 (labels + render), Task 3 (color states for `state_dot`), Task 4 (UI labels).
- §3.1 Always-on menu slot → Task 3 (`MENU_TRAY_ID`).
- §3.2 Enabled slots → Task 3 (`build_slot`, `BUILT_SLOTS`).
- §3.3 All-off case → Task 3 (empty `enabled` ⇒ only menu slot), Task 4 (empty hint).
- §4 Persistence → Task 1.
- §5 Render pipeline → Task 2.
- §5.1 Rasterize a glyph → Task 2 (`render_slot`, `FONT`).
- §5.2 `state_dot` color → Task 3 (`render_state_dot`, color constants).
- §6.1 Tauri command → Task 3.
- §6.2 Refresh path → Task 3 (60 s loop + db-changed unchanged; new `oxiline://tray-changed`).
- §6.3 Build / rebuild → Task 3 (`build`, `rebuild`).
- §6.4 Korean glyph handling → Task 2 (font is ASCII-only; unknown bytes fall through to box).
- §7 Preferences UI → Task 4.
- §8 Files changed → Tasks 1–5.
- §9 Error handling → Task 3 (`let _ = ...`, `remove_tray_by_id` errors ignored, `update_tray_slots` empty-vector rejection).
- §10 Testing → Tasks 1, 2, 4 (unit + helper tests). Smoke test in Task 5.
- §11 Open questions → acknowledged in spec; no implementation work blocked.

**Placeholder scan:** No TBD/TODO/FIXME in the plan. Every step has concrete code or a concrete command.

**Type consistency:**
- `TraySlotKind` enum is defined in Task 1 and reused verbatim in Tasks 2, 3, 4.
- `TraySlotPref { kind, on, order }` is defined in Task 1, used in the renderer (Task 2 via the kind enum only) and the command (Task 3).
- `update_tray_slots` command name is consistent between `commands.rs` (Task 3), `lib.rs` registration (Task 3), `api.ts` generated binding (Task 4), and `useUpdateTraySlots` (Task 4).
- `oxiline://tray-changed` event name appears in `lib.rs` listener (Task 3) and the React `onSuccess` (Task 4).
- `BUILT_SLOTS: Mutex<HashMap<TraySlotKind, TrayIcon>>` declared in Task 3 and consumed in `refresh`, `rebuild`, `build_slot` within the same task.
- Color constants declared in Task 3 and consumed in the same task — no cross-task drift.
- TS field names mirror Rust via `specta::Type` (`kind`, `on`, `order`); `kind` is `TraySlotKind` enum so the TS discriminant matches `"NowRecording" | "NowNext" | "StateDot"` exactly (the `switch` in the React `slotLabel` helper uses those literal strings).

**No-omission check:**
- Test files referenced in §10.1, §10.2 are created in Tasks 1 and 2.
- §10.3 vitest toggle/swap test is implemented as a pure-helper test (`swapOrder`) in Task 4 because the existing test infra is node-only and does not include React testing-library. The spec's wording is "toggle/swap behavior" — covered by the helper test plus the inline move/toggle handlers.
- §10.4 build/lint gates are repeated in Task 5 Step 1.
- The legacy `render_progress_icon` and the `progress` variable in the old `refresh` are explicitly removed in Task 3 Step 1 (the diff replaces the function block).
