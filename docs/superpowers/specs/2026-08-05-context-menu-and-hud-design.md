# Context Menu + HUD Polish — Design Spec

> **Status:** Approved for autonomous execution (user delegated, went to sleep
> 2026-08-05). Single source of truth for this feature surface.
> **Scope:** (A) replace the native webview right-click menu with an app-native
> context menu; (B) refine the floating HUD. Both follow the existing Oxi design
> system (`doc/06`, `src/tokens/*`) and the surface-as-interface principle
> (`doc/09` §9.1).

---

## A. Context Menu

### A.1 Problem

Right-clicking anywhere shows the platform's native webview context menu
(Reload / Inspect / …). It is not app-native, exposes nothing useful, and
undermines the "보이는 것이 작동한다" principle. There is no custom menu today.

### A.2 Approach

A single, app-wide, data-driven menu — not a per-element reimplementation.

1. **Suppress the native menu app-wide.** A document-level
   `contextmenu` → `preventDefault()` listener installed once at boot (main
   window and HUD). `preventDefault` only blocks the native menu; React's
   `onContextMenu` handlers still fire, so elements that declare a menu open it.
2. **One store, one component.** A `useContextMenu` zustand store holds
   `{ x, y, items, open, show, close }`. A single `<ContextMenu />` rendered at
   the `App` root (body portal) consumes it. Elements opt in by attaching
   `onContextMenu` that calls `show(clientX, clientY, items)`.
3. **Viewport-clamped positioning.** A pure `clampMenuPosition(x, y, menu,
   viewport)` flips the menu above/left of the cursor when it would overflow,
   then clamps inside the window. Measured at runtime via a ref so the clamp
   uses the real rendered size.
4. **Keyboard + pointer parity.** ArrowUp/Down move the active item (skipping
   separators/headers/disabled), Enter selects, Esc closes. Outside pointerdown,
   window blur, scroll, and resize all close it. Item click closes after
   `onSelect`.

### A.3 Menu model

```ts
type MenuItem =
  | { kind: "item"; label: string; icon?: ComponentType<{ size?: number }>;
      onSelect: () => void; danger?: boolean; disabled?: boolean }
  | { kind: "separator" }
  | { kind: "header"; label: string };
```

### A.4 Visual language (tokens, not new styles where possible)

Container: `bg-surface-raised`, `1px border-border`, `rounded-lg`, `--shadow-lg`,
`p-1`, `min-w-[190px]`, fade+scale-in (`--duration-fast`, `--ease-out`).
Item: `flex items-center gap-2 px-2.5 py-1.5 rounded text-[13px]`; rest
`text-text`, hover `bg-surface-sunken`; active (keyboard) `bg-surface-sunken`.
Danger: `text-status-error` rest, `bg-status-error-subtle` hover. Header:
`px-2.5 pt-1 pb-0.5 text-[10px] font-semibold uppercase tracking-wide
text-text-subtle`. Separator: `my-1 h-px bg-border`. Icon: `size-3.5
text-text-subtle` (danger → `text-status-error`). A new `.context-menu` class +
`@keyframes ctx-in` live in `styles.css` (app chrome, not portable tokens).

### A.5 Surfaces & their menus

Every menu mirrors actions that already work via direct manipulation — the
context menu is the *discoverable secondary path*, per `doc/09` §9.1. Block
handlers `stopPropagation()` so the lane-background menu does not also open.

| Surface | Items |
|---|---|
| **PlanCard** | header `HH:MM · Nm`; ▶ 지금 녹화 / ■ 녹화 중지 (toggle); ──; 🗑 삭제 |
| **ActualBlock (live)** | header name; ■ 녹화 중지 |
| **ActualBlock (past)** | header name; ▶ 이어서 녹화 (start same activity); ──; 🗑 삭제 |
| **Sidebar activity** | header name; ▶ 녹화 시작 / ■ 녹화 중지 (toggle); ──; 🗑 활동 삭제 (`force=false`, safe) |
| **Timeline background** | 오늘로 이동 (`goToToday`); 지금으로 스크롤 (`requestScroll(now)`) |

Right-click anywhere else = no menu (native suppressed, nothing opens). That is
correct: the surface dictates the action.

### A.6 Non-interference with drag

PlanCard/ActualBlock drag handlers guard on `e.button !== 0`, so a right-click
(button 2) never starts a drag. `onContextMenu` is added independently.

---

## B. HUD Polish

### B.1 Problems with current HUD

- Idle states are passive: "자유 시간" with no action; "지금 예정" with no start.
- Visual hierarchy is flat; elapsed timer is small; no accent identity.
- The oxide bar now-marker works but the card lacks the activity's hue identity.

### B.2 Improvements

1. **Actionable idle.** When idle but a current plan slot exists, show a
   `▶ 지금 시작` button that starts the resolved (or first) option — the HUD
   becomes a one-click launch, not just a glance.
2. **Informative free time.** When idle with no current slot, show today's total
   recorded time (`오늘 Nh Nm 기록`) computed from `useDayRecords`, so the glance
   always carries meaning.
3. **Accent identity + hierarchy.** Active card gets a 3px hue left-rail (the
   same stripe the timeline blocks use) and a larger mono elapsed timer.
4. **Click-to-open main.** Clicking the card calls a new `show_main_window`
   Tauri command (mirrors the `single_instance` show+focus pattern). The
   stop/start buttons `stopPropagation` so they act, not navigate.
5. **Window size** bumped 170→200px (`tauri.conf.json`) for breathing room; the
   card keeps its transparent floating style.

### B.3 `show_main_window` command (Rust)

```rust
#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
```
Registered in the specta builder; frontend `api.showMainWindow()`.

---

## C. Testing & verification

- **Unit (vitest, node, `.test.ts`):** `clampMenuPosition` edge cases (overflow
  right/bottom flips + clamps; fits → unchanged); `useContextMenu`
  show/close/replace. These are the pure-logic units; component rendering &
  pointer wiring are verified via build + browser (existing convention,
  `vitest.config.ts`).
- **Gates:** `bun run test`, `bun run build` (tsc -b + vite), `cargo build
  --workspace`, `cargo clippy --workspace -- -D warnings`.
- **Spec update:** `doc/09` §9.8 (interaction table) + new §9.13 context-menu
  note; HUD §9.7 entry refined.
