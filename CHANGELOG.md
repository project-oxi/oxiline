# Changelog

All notable changes to OxiLine will be documented in this file.

## [Unreleased]

## [0.2.0] - 2026-08-05

UI redesign, icon refresh, and interaction polish.

### Desktop app (`oxiline-app`)

- **feat**: redesigned UI interactions, record entry points, and visual
  hierarchy (per `oxi DESIGN.md`)
- **feat**: replace Tauri app icon with clock design
- **fix**: hide Oxide Bar now-marker for non-today dates in main window

## [0.1.0] - 2026-08-04

First official release. Recording-centered OxiLine: data layer (`oxiline-core`),
headless CLI (`oxiline-cli`), and Tauri v2 macOS desktop app (`oxiline-app`).

### Recording core (`oxiline-core`)

- **feat**: recording lifecycle (`start` / `stop` / `list` / `current`) with
  single-active-session semantics and 5-minute neutral rounding
- **feat**: OR plan slots (`add_options` w/ `BEGIN IMMEDIATE`, monotonic unique
  `sort_order`, dedup within input and against existing)
- **feat**: `resize_plan` partial-duration update
- **feat**: neutral weekly / daily compliance view-model (`Under` / `Met` /
  `Over` / `Unbudgeted`, hue shared from the activity)
- **feat**: `now_summary` derived from active record + slot
- **feat**: V4 migration (record tables) + V5 migration (drop legacy
  task / routine / timeline / reports / cards tables)
- **fix**: `add_options` sort-order race under concurrency
- **refactor**: delete legacy `tasks` / `routines` / `timeline` / `reports` /
  `cards` modules

### CLI (`oxiline-cli`)

- **feat**: `activity` / `plan` / `record` / `report` subcommands, headless
- **feat**: neutral activity report (weekly ratio per activity)
- **feat**: stable `--json` output across all commands
- **fix**: `record log --date` / `--range` timezone-aware day bounds (records
  are stored in UTC; previously the day range was hard-coded to UTC midnight,
  missing records created in the morning hours of non-UTC timezones)
- **refactor**: drop legacy `task` / `routine` / `today` / `export` / `streak`
  commands

### Desktop app (`oxiline-app`)

- **feat**: 3-pane shell (Sidebar / main / Inspector) at 1180 × 820
- **feat**: 16 Tauri commands over the recording core + hand-written
  `api.ts` / `types.ts`
- **feat**: two-lane recording timetable `[계획 | 실제 | 둘 다]` with
  plan choice-groups (dashed / hollow) and actual records (solid) + now-line
- **feat**: live Sidebar (now-card + activity library with neutral weekly bars)
- **feat**: Inspector `[주간 | 오늘]` compliance + total + recent sessions
- **feat**: `ActivitySwitcher` (⌘⇧A quick record-switch)
- **feat**: HUD (�⇧O) rework
- **feat**: drag activity card from library → drop on timetable → `create_plan`
- **feat**: OR-plan multi-select drag + drop-to-merge (`add_plan_options`)
- **feat**: plan-card resize handle
- **feat**: date popover month calendar with record markers (calendar-month
  shift, not ±35 days)
- **feat**: `addPlanOptions` / `useAddPlanOptions` bulk drop-merge
- **feat**: design tokens split into `src/tokens/` per oxi DESIGN.md §2.2
- **fix**: resize mutate outside updater + `pointercancel` cleanup
- **fix**: clear activity multi-select after drop
- **fix**: i18n dead legacy keys removed, stale references fixed
- **refactor**: strip legacy frontend (hooks / api / types) and legacy views
  (Backlog / Week / Report / RoutineManager)
- **refactor**: drop legacy commands, drop legacy backlog / block DnD branches
- **refactor**: replace `NowContext` with recording-native `now_summary`

[0.1.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.1.0

[0.2.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.2.0
