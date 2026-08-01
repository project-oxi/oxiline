# Recording GUI — Implementation Plan (Plan 2 of 2)

> **For agentic workers:** Use superpowers:subagent-driven-development or
> superpowers:executing-plans. Plan 1 (core + CLI) is complete on `main`;
> this plan builds the desktop GUI on top of it.

**Goal:** Ship the recording-centered desktop UI — a 3-pane shell
(sidebar / timetable / inspector) with a `[계획|실제|둘 다]` two-lane
timetable, a now-recording sidebar, a neutral-compliance inspector, and a
quick switcher — replacing the legacy single-column timeline.

**Visual spec:** `docs/superpowers/specs/2026-08-01-final-mockup.html`
(canonical 3-pane layout). `…-desktop-timetable-mockup.html` is a variant.

**Tech Stack:** Tauri v2 · React 19 · TypeScript · Tailwind v4 (semantic
tokens from `src/styles.css`) · `@dnd-kit`. The frontend talks to Rust via
hand-written `api.ts` invoke wrappers (see decisions below).

## Key Decisions (verified against the codebase)

1. **No generated bindings today.** `build.rs` is plain `tauri_build::build()`;
   `src-tauri/src/commands.rs` carries `#[specta::specta]` decorators but
   `bindings.ts` is NOT generated or consumed. The frontend uses hand-written
   `src/lib/api.ts` (`invoke<T>(name, args)`) + `src/types.ts`. **Follow this
   convention** — do not wire up tauri-specta generation in this plan; add the
   new recording commands to `commands.rs` (same `#[tauri::command]`+`#[specta::specta]`+
   `Result<_, String>` shape) and their hand-written wrappers to `api.ts`/`types.ts`.
2. **Shell today:** `App.tsx` = `<Header/>` + a single switched content view
   (`DayTimeline`/`WeekView`/`BacklogView`/`ReportView`) + modals. Window is
   420×720 (minWidth 360). Task 1 enlarges this and introduces the 3-pane body.
3. **Semantic tokens only.** Components consume `var(--color-…)` / Tailwind
   utilities (`bg-surface-sunken`, `text-text-subtle`, …) — never raw hex.
   See `src/styles.css`. Activities carry a `hue_label`; render their hue via
   the palette (no status red/green — compliance is neutral).
4. **Neutral copy (load-bearing):** Under/Met/Over/Unbudgeted never use
   실패/깨짐/놓침. Over ⇒ "초과 +Xm". Match the CLI's `lang.rs` vocabulary.
5. **Legacy demolition is the LAST task.** `tasks.rs`/`routines.rs`/`timeline.rs`/
   `reports.rs`/`cards.rs` and their tables stay until the new UI fully
   replaces the timeline; then `V5__drop_legacy.sql` removes them.

## Global Constraints

- Every new `#[tauri::command]` is `#[specta::specta]` + `Result<T, String>`,
  mapping `CoreError` via the existing `map_err` (`format!("{}: {}", code, msg)`).
- Core (`oxiline-core`) is COMPLETE — do not modify it in this plan. New
  commands only WRAP existing `record::*`/`plan::*`/`activities::*`.
- Timetable minute math reuses `src/lib/timeline-math.ts` (snap, ranges).
- DnD reuses `src/lib/dnd.tsx` (`DndProvider`).
- No raw hex; no inline color logic that contradicts the neutral-compliance rule.

---

## Task 1 — Enlarge window + 3-pane shell

**Files:** `src-tauri/tauri.conf.json` (width 420→1180, minWidth 360→980,
height 720→800, minHeight 560) · `src/App.tsx` (3-pane body) ·
`src/components/Sidebar.tsx` (new, scaffolding) · `src/components/Inspector.tsx`
(new, scaffolding) · `src/styles.css` (pane grid tokens).

**Change:** Introduce a `.shell` grid: `sidebar (260px) | main (1fr) |
inspector (300px)`, collapsing to drawers/bottom-bar under a container query
(<980px → stack; sidebar/inspector become togglable). For Task 1 the panes are
**scaffolding**: Sidebar shows a placeholder now-card + "활동 · 카드" stub;
Inspector shows "충족도" stub; Main keeps the existing `DayTimeline`. Header
stays as the toolbar. Existing `view` switch (today/week/backlog/report) still
drives Main content for now.

**Acceptance:** App launches at ~1180px with three visible panes; at <980px the
sidebar/inspector collapse gracefully (no horizontal scroll); `cargo build` +
`bun run build` pass; existing timeline still renders in Main.

## Task 2 — commands.rs wrappers + api.ts/types.ts

**Files:** `src-tauri/src/commands.rs` (add record/plan/activities commands) ·
`src-tauri/src/lib.rs` (register them in the invoke handler) · `src/lib/api.ts`
· `src/types.ts`.

**Change:** Add thin wrappers mirroring Plan 1's CLI surface:
`list_activities, create_activity, resolve_activity, update_activity,
delete_activity, start_record, stop_record, current_record_state,
list_records, compliance, list_plans, create_plan, slots_for_date,
update_plan, delete_plan`. Each: `#[tauri::command] #[specta::specta]`,
`State<AppState>`, `Result<_, String>` via `map_err`. Register in `lib.rs`'s
`invoke_handler!`. Add matching typed wrappers in `api.ts` + types in `types.ts`
(`Activity`, `Plan`, `PlanSlot`, `Record`, `RecordState`, `Compliance`,
`ComplianceState`, `Scope`, `ActivityInput`, `PlanInput`).

**Acceptance:** `cargo build -p oxiline-app` passes; a `bun run build` typechecks;
`api.startRecord('id')` resolves against the live backend.

## Task 3 — Timetable two-lane (`[계획|실제|둘 다]`)

**Files:** `src/components/RecordTimeline.tsx` (new) · `src/components/PlanLane.tsx`
· `src/components/ActualLane.tsx` · `src/lib/store.ts` (timetable mode state).

**Change:** A new main-pane component consuming `slots_for_date` (plan lane:
choice groups, dashed/hollow options, OR marker, picked→executed link) and
`list_records` (actual lane: solid filled blocks + now-line). Mode toggle
`[계획|실제|둘 다]`; in "둘 다" the canvas splits into two lanes (plan left,
actual right) per the mockup. 5-min rounding shown on durations.

**Acceptance:** Toggle switches the three modes; "둘 다" shows two lanes; an
active record appears as a live block with elapsed; no `is_done` anywhere.

## Task 4 — Sidebar: now-card + activity library

**Files:** `src/components/Sidebar.tsx` (fill scaffolding) ·
`src/components/NowCard.tsx` · `src/components/ActivityLibrary.tsx`.

**Change:** Now-card (active session: name, rounded elapsed, today/weekly meta,
전환/멈춤 buttons) driven by `current_record_state`. Activity library: each
active activity with its weekly bar + tick (target) + "남음 X"/"달성"/"+Xm",
drag-to-place onto the timetable (Task 7 wires the drop).

**Acceptance:** Recording an activity updates the now-card live; library bars
reflect weekly compliance (neutral labels).

## Task 5 — Inspector: compliance + total + recent sessions

**Files:** `src/components/Inspector.tsx` (fill scaffolding) ·
`src/components/ComplianceOverview.tsx` · `src/components/SessionLog.tsx`.

**Change:** `[주간|오늘]` segment → `compliance(scope)`; per-activity row
(swatch, name, %, bar+tick, "X / Y · 남음 Z"/"달성"/"+Xm"). Total line.
Recent sessions from `list_records`. Neutral copy throughout.

**Acceptance:** Switching 주간/오늘 recomputes; over-budget shows "+Xm", never
failure language.

## Task 6 — Switcher panel + enriched HUD + date popover

**Files:** `src-tauri/src/shortcuts.rs` (⌘⇧A global) · `src/components/Switcher.tsx`
(`tauri-nspanel`-style overlay, ⌘⇧A) · `src/hud.tsx` (enrich: next-up + per-goal
remaining) · `src/components/DatePopover.tsx` (calendar dropdown from the date pill).

**Acceptance:** ⌘⇧A opens the switcher (pick activity → start record); HUD shows
the current goal remaining; date pill opens a month calendar (today-recorded total).

## Task 7 — Card planning DnD

**Files:** `src/components/ActivityLibrary.tsx` (drag source) ·
`RecordTimeline.tsx` (drop target) · `src/lib/dnd.tsx`.

**Change:** Drag 1 activity → drop on the timetable → single-option plan; drop/
select 2+ → OR plan. Resize handle on plan blocks (dot labels, no vertical bar).
Mirror the existing `@dnd-kit` usage.

**Acceptance:** Dropping creates a plan (`create_plan`); resizing updates
`duration_minute`; OR plans show multiple options.

## Task 8 — Legacy demolition

**Files:** `crates/oxiline-core/migrations/V5__drop_legacy.sql` (drop
`tasks`/`routine_blocks`/`categories` if fully unused) · remove `tasks.rs`/
`routines.rs`/`timeline.rs`/`reports.rs`/`cards.rs` legacy paths + their tests ·
remove legacy CLI commands + frontend views (`BacklogView`, `ReportView` legacy,
`RoutineManager`) once the recording UI fully replaces them.

**Acceptance:** `cargo test --workspace` green; the app runs without the legacy
tables; no dead legacy code remains.

---

## Self-Review

- Spec coverage: mockup's toolbar/sidebar/main/inspector ⇒ T1 (shell), T3
  (timetable), T4 (sidebar), T5 (inspector), T6 (switcher/HUD/date), T7 (dnd).
  Data plumbing ⇒ T2. Cleanup ⇒ T8.
- The api.ts convention decision avoids a speculative bindings-gen task that
  doesn't match the current build setup.
- Legacy demolition is correctly last and gated on full UI replacement.
- Each task leaves the app building (scaffolding-first for T1/T4/T5).

Plan is ready.
