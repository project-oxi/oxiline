# Context Menu + HUD Polish — Implementation Plan

> **For agentic workers:** Steps use checkbox (`- [ ]`) syntax. Execute inline
> (single session). Gates after each phase: `bun run test`, `bun run build`,
> `cargo build --workspace`, `cargo clippy --workspace -- -D warnings`.

**Goal:** Replace the native webview right-click menu with an app-native
data-driven context menu, and refine the floating HUD (actionable idle, today
summary, accent identity, click-to-open).

**Architecture:** One `useContextMenu` zustand store + one `<ContextMenu />`
portal at the App root. A document-level `contextmenu`→`preventDefault`
suppresses the native menu app-wide; elements opt in via `onContextMenu`→
`show(x,y,items)`. A new `show_main_window` Tauri command lets the HUD focus the
main window.

**Tech Stack:** React 19, zustand, Tailwind v4 (Oxi tokens), lucide-react,
Tauri v2 (specta commands). Spec:
`docs/superpowers/specs/2026-08-05-context-menu-and-hud-design.md`.

## Global Constraints

- Korean copy in components (matches existing convention); code/comments in
  English. No emojis in code.
- Reuse Oxi tokens (`surface-raised`, `border`, `shadow-lg`, `status-error`,
  `interactive-primary`). New chrome class `.context-menu` in `styles.css`.
- Tests are node-only `.test.ts` (no DOM). Pure logic only.
- Block drag handlers guard `e.button !== 0` → right-click never drags.

---

## Task 1: Store + clamp helper + tests

**Files:** Create `src/lib/context-menu.ts`, `src/lib/__tests__/context-menu.test.ts`.

- [ ] `context-menu.ts`: `MenuItem`/`MenuEntry`/`MenuSeparator`/`MenuHeader`
  types, `useContextMenu` store (`{x,y,items,open,show,close}`), and
  `clampMenuPosition(x,y,{width,height},{width,height},margin=8)` (flip on
  overflow then clamp inside viewport).
- [ ] `context-menu.test.ts`: clamp fits unchanged; overflow-right flips left;
  overflow-bottom flips up; both clamp inside viewport; store show/replace/close.

## Task 2: ContextMenu component + chrome CSS

**Files:** Create `src/components/ContextMenu.tsx`; modify `src/styles.css`.

- [ ] `ContextMenu.tsx`: portal to `document.body`; reads `useContextMenu`;
  `useLayoutEffect` measures the panel via ref, clamps with
  `clampMenuPosition`; keyboard nav (Arrow/Enter/Esc over `item` entries);
  close on outside pointerdown, blur, scroll, resize. Styling per spec §A.4.
- [ ] `styles.css`: `.context-menu` + `@keyframes ctx-in` (fade+scale,
  `--duration-fast`).

## Task 3: Suppress native menu; render at root

**Files:** Modify `src/main.tsx`, `src/hud.tsx`, `src/App.tsx`.

- [ ] `main.tsx` + `hud.tsx`: `document.addEventListener("contextmenu", (e) =>
  e.preventDefault())` once at boot.
- [ ] `App.tsx`: render `<ContextMenu />` once (after overlays).

## Task 4: Wire element menus

**Files:** Modify `src/components/RecordTimeline.tsx`, `src/components/Sidebar.tsx`;
add `useDeleteActivity` to `src/hooks.ts`.

- [ ] PlanCard `onContextMenu`: header + toggle + delete.
- [ ] ActualBlock `onContextMenu`: live→stop; past→continue + delete.
- [ ] `useDeleteActivity` hook (`api.deleteActivity(id,false)`).
- [ ] DraggableActivity `onContextMenu`: header + toggle + delete.
- [ ] Timeline background `onContextMenu`: 오늘로 이동 / 지금으로 스크롤.

## Task 5: show_main_window command + api

**Files:** Modify `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`,
`src/lib/api.ts`.

- [ ] `commands.rs`: `show_main_window` (show+focus main webview window).
- [ ] `lib.rs`: register in specta `collect_commands!`.
- [ ] `api.ts`: `showMainWindow: () => invoke("show_main_window")`.

## Task 6: HUD redesign

**Files:** Modify `src/hud.tsx`, `src-tauri/tauri.conf.json` (height 170→200).

- [ ] Active: hue left-rail, larger mono elapsed, refined danger stop.
- [ ] Idle current-slot: `▶ 지금 시작` (start resolved/first option).
- [ ] Idle free: today total recorded time.
- [ ] Card click → `api.showMainWindow()`; stop/start buttons stopPropagation.

## Task 7: Verify + docs + commit

- [ ] Gates green. Update `doc/09-ui-redesign.md` (§9.8, new context-menu note),
  `HANDOFF.md`. Commit per task.
