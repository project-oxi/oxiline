# Menu-bar Multi-Slot Display — Design Spec

> **Status:** Approved (2026-08-08, user-confirmed). Single source of truth
> for the multi-slot menu bar surface.
> **Date:** 2026-08-08
> **Scope:** Replace the single 22×22 progress-bar tray with a CodexBar-style
> multi-slot status bar (one `NSStatusItem` per enabled information slot),
> plus a Preferences section to toggle each slot and reorder them. The
> existing progress-bar icon is removed because it is illegible at 22 px and
> adds no decision-relevant signal beyond what the new slots already convey.

---

## 1. Goal

Make the macOS menu bar the primary ambient surface for OxiLine. The user
should glance at the menu bar and immediately see **what is happening now**
and **what is next**, without opening the main window. Each signal lives in
its own independent status-item slot so the menu bar stays readable as the
number of signals grows.

Out of scope for v1: arbitrary custom slots, color theme, Linux/Windows tray
stacking, the legacy 22×22 progress bar.

---

## 2. Slot catalog (v1)

Three slots, all text-only (no icons, no emoji), 9 pt monospaced bitmap
rasterized into a ~22 px tall NSStatusItem button. Korean and English labels
come from `crates/oxiline-app/src/locales/{ko,en}.json`.

| Slot id           | Visible label (ko)                                | Visible label (en)                  | Data source                                                       |
| ----------------- | ------------------------------------------------- | ----------------------------------- | ----------------------------------------------------------------- |
| `now_recording`   | `● {활동명} {Nm}`                                 | `● {activity} {Nm}`                 | Active record first; else current unresolved plan slot for today. |
| `now_next`        | `▸ {다음제목} {Nm}`                               | `▸ {next_title} {Nm}`               | Closest future unresolved plan slot today.                        |
| `state_dot`       | (color dot only, no text)                         | (color dot only, no text)           | Derived from `now_recording` + `now_next` proximity.              |

Locale controls the `{활동명}` vs `{activity}` substitution. The glyph
prefix (`●` / `▸`), `{Nm}` formatting, and bitmapped layout are identical
in both locales; only the activity/next-title text differs.

Default state on first launch: `now_recording=on`, `now_next=on`,
`state_dot=off`. Order: `[now_recording, now_next, state_dot]`.

`{Nm}` formats remaining (`now_recording`) or starts-in (`now_next`)
minutes using the same rounding as `record_rounding_minutes` (default 5).
The dot glyph (`●`, `▸`) is part of the label string — keeps `state_dot`
as the only no-text slot.

`state_dot` colors:
- 녹화 중이면 녹색 도트
- 다음 plan이 5분 이내로 임박하면 황색 도트
- 둘 다 아니면 회색 도트

---

## 3. Layout & slot semantics

### 3.1 Always-on menu slot

One additional `NSStatusItem` is **always alive** even when all three data
slots are off. It carries no label, has a small neutral dot icon, and its
menu is the existing tray context menu (`OxiLine 열기`, `지금 보기 (HUD)`,
`빠른 추가…`, `환경설정…`, `로그인 시 자동 실행`, `OxiLine 종료`). This is
the CodexBar "merged" slot pattern: a guaranteed way back into the app
without losing access to the context menu.

### 3.2 Enabled slots

For each slot whose `on == true` (after applying `order`), create one
`TrayIconBuilder::with_id("tray-slot-{id}")` with:

- `icon`: rasterized bitmap (see §5)
- `icon_as_template(true)`: lets macOS recolor for light/dark menu bar
- `title("")`: text is rendered into the bitmap, not into the NSStatusItem
  title — keeps width predictable and avoids double-rendering
- `show_menu_on_left_click(false)`
- `on_tray_icon_event`: left-click routes to `crate::tray::show_main(app)`
- `menu`: empty (the always-on menu slot owns the menu)

Order in the menu bar follows `order` ascending (left → right).

### 3.3 All-off case

When the enabled data-slot list is empty:

- All three data slots are torn down.
- The always-on menu slot stays visible.
- Preferences shows a hint: "지금은 메뉴바에 표시되는 항목이 없어요. 최소
  하나를 켜면 메뉴바에서 바로 확인할 수 있어요."

---

## 4. Persistence

Single settings row, JSON-encoded:

```json
{
  "slots": [
    { "id": "now_recording", "on": true,  "order": 0 },
    { "id": "now_next",      "on": true,  "order": 1 },
    { "id": "state_dot",     "on": false, "order": 2 }
  ]
}
```

Stored under settings key `tray_slots`. Schema additions:

- New migration `crates/oxiline-core/migrations/V6__tray_slots.sql`
  inserts the default JSON with the v1 defaults above. The migration uses
  `INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
  ('tray_slots', '{...}', '...')` so the row only appears when missing
  (idempotent). This matches the seeding pattern used by the initial
  settings rows in `V1__init.sql`.

- `crates/oxiline-core/src/model.rs`: add
  `pub tray_slots: Vec<TraySlotPref>` to `SettingsSnapshot`.
- `crates/oxiline-core/src/settings.rs`:
  - `snapshot()` reads via new `get_tray_slots(conn)` helper that returns
    `Vec<TraySlotPref>` (default = v1 defaults when key missing or
    unparseable).
  - `pub fn save_tray_slots(conn, &Vec<TraySlotPref>) -> Result<()>`.
- New core module `crates/oxiline-core/src/tray_slots.rs`:
  - `TraySlotKind` enum: `NowRecording`, `NowNext`, `StateDot`.
  - `TraySlotPref { kind, on, order }` — `Type` + `Serialize`/`Deserialize`.
  - `resolve(conn) -> ResolvedSlots { enabled: Vec<…>, any_enabled: bool }`:
    reads `tray_slots`, fills missing slots with defaults, sorts by
    `order`, drops unknown ids, and returns the enabled subset plus a flag
    for whether the data-slot list is empty.

---

## 5. Render pipeline

`crates/oxiline-app/src-tauri/src/tray_render.rs` (new) exports:

```rust
pub fn render_slot(kind: TraySlotKind, ctx: &SlotRenderCtx) -> tauri::image::Image<'static>;
pub fn render_menu_dot() -> tauri::image::Image<'static>; // always-on slot
```

`render_slot` builds a 1-bit RGBA bitmap:

- **Height:** 22 px.
- **Width:** variable — measured by the text width at 9 pt monospaced; clamp
  to `[24, 120]`. Right-padded by 4 px so consecutive slots don't touch.
- **Foreground:** `--color-text` (the standard `--type-md` token).
- **Background:** fully transparent.
- **Text source:** formatted label from §2 using
  `oxiline_core::plan::now_summary(conn, now_minute)` and
  `oxiline_core::settings::get_i64(conn, "record_rounding_minutes", 5)`.

Bitmap font: ship a 5×7 monospaced font as `BTreeMap<u8, [u8; 5]>` in
`tray_render.rs` for ASCII (`0-9`, `A-Z`, `●`, `▸`, space, colon, dot).

### 5.1 Rasterize a glyph

For each character of the label string:

1. Look up the 5×7 bitmap for the byte (ASCII only).
2. Draw 7 rows × 5 cols into the RGBA buffer at the current cursor.
3. Advance cursor by 6 px (5 + 1 px gap).

Unknown characters render as a 5×7 box (1 px border) so the slot width is
predictable.

`state_dot` colors map to the existing design-system status tokens (verified
against `doc/06-design-system.md` §3.2 — `accent-oxide` is the verdigris
used by status-success):

- 녹화 중 → `--accent-oxide` (verdigris; same color family as the existing
  progress bar)
- 다음 plan이 5분 이내로 임박 → `--color-status-warning` (amber)
- 둘 다 아님 → `--color-text-subtle` (gray)

Token resolution uses `getComputedStyle(document.documentElement)` so the
bitmap is regenerated when the theme switches (Preferences `theme`
setting → `applyTheme` → 60 s refresh loop re-rasterizes).

---

## 6. Tray wiring

### 6.1 Tauri command

`crates/oxiline-app/src-tauri/src/commands.rs`:

```rust
#[tauri::command]
#[specta::specta]
pub fn update_tray_slots(
    state: State<AppState>,
    app: AppHandle,
    slots: Vec<TraySlotPref>,
) -> Result<(), String> {
    crate::tray_slots::save(&state.conn(), &slots).map_err(map_err)?;
    crate::tray::rebuild(&app);
    Ok(())
}
```

Routes through `crate::tray::rebuild` (see §6.3) rather than
`refresh` so toggles add or remove NSStatusItems, not just relabel them.

Existing 60 s sleep loop and `oxiline://db-changed` listener continue to
call `tray::refresh(app)`. `refresh` iterates over currently-built slots
and re-rasterizes each. Built slots are tracked in
`tray.rs::BUILT_SLOTS: Mutex<HashMap<TraySlotKind, TrayIcon>>` so refresh
knows which `TrayIcon` handles to update without re-querying by id.

A new event `oxiline://tray-changed` is emitted from the Preferences
mutation `onSuccess` so the Rust side can rebuild without polling.

### 6.3 Build / rebuild

1. Always create the menu slot (`tray-menu`) with a 6×6 dot icon colored
   `--color-text-subtle` (gray, neutral) and the existing context menu.
2. Resolve `tray_slots` from DB.
3. For each enabled slot in `order`, create or reuse `tray-slot-{id}`.
4. Call `refresh(app)`.

`tray::rebuild(app)` is `build`'s idempotent variant: tear down every
existing `tray-slot-*` first, then run `build`. `tray-menu` is preserved.

NSStatusItem teardown uses `app.remove_tray_by_id(...)` (Tauri 2.x API).
On macOS this calls `NSStatusBar.removeStatusItem(_:)`, freeing the slot.

### 6.4 Korean glyph handling (deferred-but-decided)

v1 labels are kept short and ASCII-first to avoid shipping a CJK font:

- `now_recording`: `● {activity} {Nm}` — activity names default to ASCII
  (record/plan/activity inputs already trim Korean titles; users who use
  Korean names get `●` plus a placeholder `·` until v2).
- `now_next`: same.
- No new font dependency. If a Korean activity name reaches the renderer,
  it falls through to the `unknown char` box fallback, which is
  documented and visible — users learn to use ASCII names for slot text.

v2 would ship `fontdb` + `ab_glyph` + a 9 pt CJK font; explicitly out of
scope here.

---

## 7. Preferences UI

`crates/oxiline-app/src/components/Preferences.tsx` gains a new section,
placed between `t("notifications.section")` and `t("settings.categories")`,
titled `t("settings.menubar")`:

```
┌─ 메뉴바 표시 ─────────────────────────────────────┐
│  메뉴바에 어떤 정보를 표시할지 골라보세요.        │
│  켜진 항목은 좌→우 순서로 메뉴바에 나란히 표시돼요.│
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │ ●  지금 무엇을 하고 있는지   (켜짐)  [▲][▼] │  │
│  │ ●  다음 할 일                (켜짐)  [▲][▼] │  │
│  │ ○  상태 점만                 (꺼짐)  [▲][▼] │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  모두 끄면 메뉴 아이콘은 메뉴용 1개만 남아요.      │
└────────────────────────────────────────────────────┘
```

Behavior:

- Drag-and-drop is **not** added in v1. Order changes use `▲` / `▼` icon
  buttons that swap adjacent `order` values.
- Toggle uses the existing `<input type="checkbox">` pattern from the
  Notifications section.
- One `useMutation` calls `api.updateTraySlots(slots)` and on success
  invalidates `["tray-slots"]` and emits `oxiline://tray-changed`.
- i18n keys added: `settings.menubar`, `settings.menubarHelp`,
  `settings.menubarEmptyHint`, `settings.slotNowRecording`,
  `settings.slotNowNext`, `settings.slotStateDot`.

---

## 8. Files changed

| Path                                                              | Change                                                                  |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `crates/oxiline-core/migrations/V6__tray_slots.sql`               | NEW — seed default `tray_slots` JSON                                    |
| `crates/oxiline-core/src/db.rs`                                    | MODIFY — register `V6`                                                   |
| `crates/oxiline-core/src/model.rs`                                | MODIFY — add `TraySlotKind` enum, `TraySlotPref`, `SettingsSnapshot.tray_slots` |
| `crates/oxiline-core/src/settings.rs`                             | MODIFY — `snapshot()` reads `tray_slots`; add `save_tray_slots`         |
| `crates/oxiline-core/src/tray_slots.rs`                           | NEW — `resolve` / `save` / `defaults`                                   |
| `crates/oxiline-core/tests/tray_slots.rs`                         | NEW — `resolve` unit tests (default fill, missing keys, unknown ids, order normalization) |
| `crates/oxiline-app/src-tauri/src/tray.rs`                        | MODIFY — `build` / `rebuild` / `refresh` aware of multiple slots; menu slot always alive |
| `crates/oxiline-app/src-tauri/src/tray_render.rs`                 | NEW — bitmap font + `render_slot`                                       |
| `crates/oxiline-app/src-tauri/src/commands.rs`                    | MODIFY — `update_tray_slots` command                                    |
| `crates/oxiline-app/src/lib/api.ts`                               | MODIFY — `updateTraySlots`                                              |
| `crates/oxiline-app/src/hooks.ts`                                 | MODIFY — `useTraySlots` / `useUpdateTraySlots`                          |
| `crates/oxiline-app/src/components/Preferences.tsx`              | MODIFY — new "메뉴바 표시" section                                       |
| `crates/oxiline-app/src/locales/{ko,en}.json`                     | MODIFY — new keys                                                       |
| `crates/oxiline-app/src/__tests__/preferences-tray-slots.test.tsx` | NEW — toggle/swap/mutation behavior                                     |

---

## 9. Error handling

- DB read failure during `refresh` → log `let _ = ...`, render slot with
  the previous cached bitmap (last-known state). Each slot's last
  successful bitmap is held in a `OnceLock<HashMap<SlotId, Image>>` so a
  transient DB error doesn't blank the menu bar.
- `set_icon` returning `Err` → ignored (same as today's `let _ = ...`).
- `remove_tray_by_id` returning `Err` on a non-existent id → ignored
  (rebuild is idempotent and called often).
- `update_tray_slots` rejecting an empty `slots` array → API returns
  `Err("at least one slot row required")` and the Rust side keeps the
  last known state (UI shows a small inline error). The always-on menu
  slot is **not** affected by an empty data-slot list; see §3.3.

---

## 10. Testing

### 10.1 Core unit tests

`crates/oxiline-core/tests/tray_slots.rs`:

1. `resolve_returns_three_defaults_when_key_missing`
2. `resolve_drops_unknown_ids`
3. `resolve_sorts_by_order`
4. `resolve_normalizes_duplicate_orders`
5. `save_then_resolve_round_trip`
6. `snapshot_includes_tray_slots_field`

### 10.2 Render tests

`crates/oxiline-app/src-tauri/src/tray_render.rs` (inline `#[cfg(test)]`):

1. `render_menu_dot_dimensions`
2. `render_slot_known_kind_has_non_zero_width`
3. `render_slot_unknown_char_renders_box`
4. `render_slot_korean_char_falls_through_to_box` (documents §6.4)

### 10.3 Command + Preferences tests

- vitest: Preferences "메뉴바 표시" section toggles a slot, swaps two slots,
  fires `api.updateTraySlots`, on success invalidates `["tray-slots"]`.
- vitest: empty state shows the hint string and still keeps the always-on
  menu slot section out of the section body.

### 10.4 Build / lint

- `cargo test --workspace` — green.
- `cargo clippy -p oxiline-app -p oxiline-core -- -D warnings` — clean.
- `tsc --noEmit && vite build` — clean.
- `cargo run -p oxiline-app` — manual smoke: toggle `state_dot`, verify
  macOS menu bar adds a third dot slot; toggle all off, verify only the
  menu slot remains. Click the menu slot, verify existing menu opens.

---

## 11. Open questions

None blocking. The user has confirmed:

- Multi-slot (CodexBar) layout, not single-slot with stacked info.
- All-off allowed, menu slot always present.
- Order = Preferences order, left → right.
- 60 s + DB-changed refresh cadence is reused as-is.
- Progress-bar icon is dropped.

The Korean-glyph fallback (§6.4) is a deliberate v1 simplification. If
the user wants Korean activity names to render in v1, the implementation
plan will need to add a CJK bitmap font — call this out before plan
finalization if it matters.
