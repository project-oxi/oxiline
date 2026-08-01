# OxiLine — Project Status (2026-08-01)

> Living snapshot. Replaces the mid-Plan-1 handoff. Maintained alongside the
> SDD ledger at `.superpowers/sdd/2026-08-01-recording-core-and-cli/progress.md`.

## Recording layer — Plan 1 ✅ COMPLETE

The data layer + CLI for recording-centered OxiLine shipped on `main`:

- **Core** (`oxiline-core`): activities, OR plans, records, neutral weekly
  compliance, 5-min rounding — additive `V4` migration, 41 tests green.
- **CLI** (`oxiline-cli`): `oxiline activity / record / plan / report` work
  headless; 13 tests green. `--json` outputs match the spec; neutral copy
  (Over = "초과 +Xm", never 실패/깨짐/놓침).
- **Acceptance**: `cargo test --workspace` all green, clippy clean, 0 warnings.

Completion commits: `9012984` (app token styling) → `f9696b3` (mockups) →
`a058877` (merge Tasks 1–9) → `e3a54cd` (plan group) → `9186afd` (report) →
`dd3213c` (plan fence fix). The `recording-core-cli` worktree is merged; it
remains at `.worktrees/recording-core-cli/` (safe to remove).

Reference: `docs/superpowers/plans/2026-08-01-recording-core-and-cli.md`,
visual spec `docs/superpowers/specs/2026-08-01-final-mockup.html`.

## Recording layer — Plan 2 (GUI) ✅ Tasks 1–7 done · Task 8 partial

Depends on Plan 1. Built the recording-centered desktop UI on the Tauri v2 +
React shell. Plan: `docs/superpowers/plans/2026-08-01-recording-gui-plan2.md`.
Acceptance gate (after each task): `cargo build --workspace` + `bun run build`
+ vitest + clippy, all green.

**Done (commits on `main`):**
- T1 `49db687` — window 420→1180 + 3-pane shell (Sidebar / main / Inspector).
- T2 `6230b18` — 16 tauri commands over the Plan 1 core + hand-written
  `api.ts`/`types.ts` (TS `Record`→`ActivityRecord` to avoid the global collision).
- T3 `57386e6` — `RecordTimeline` two-lane timetable `[계획|실제|둘 다]`
  (plan choice-groups dashed/hollow + actual solid records + now-line).
- T4+T5 `e7f9d09` — live Sidebar (now-card + activity library w/ neutral
  weekly bars) + Inspector (`[주간|오늘]` compliance + total + recent sessions).
- T6 `a4d8f40` — `ActivitySwitcher` (⌘⇧A quick record-switch). HUD (⌘⇧O)
  enrichment + date popover deferred.
- T7 `0c8b91d` — drag an activity card from the library → drop on the
  timetable → `create_plan` (one-shot at the drop minute). OR multi-select +
  resize deferred.
- T8 `e718d56` — removed dead `DayTimeline.tsx`. **Full legacy demolition
  deferred**: the legacy views (Backlog/Week/Report/RoutineManager) are still
  in the view switch and render legacy data, so dropping the legacy tables/
  modules now would break the app. Do it once recording-native replacements
  for those views land.

**Deferred items (next session):** HUD enrichment, date popover, OR-plan
multi-select + plan resize, full legacy demolition (`V5__drop_legacy.sql`).

## How to resume

```
cargo test --workspace      # Plan 1 gate — green
# Plan 2: see the plan2 doc, execute task-by-task (SDD or inline)
```
