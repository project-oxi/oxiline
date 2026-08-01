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

## Recording layer — Plan 2 (GUI) 🚧 IN PROGRESS

Depends on Plan 1. Builds the recording-centered desktop UI on the existing
Tauri v2 + React shell. Plan: `docs/superpowers/plans/2026-08-01-recording-gui-plan2.md`.

Key shape decisions (from the converged mockup + codebase):
- **Window**: 420 → ~1180 wide, minWidth raised, 3-pane shell
  (sidebar / main / inspector) with container-query responsive collapse.
- **Data plumbing follows the existing convention**: commands.rs gets
  `#[tauri::command]`+`#[specta::specta]` wrappers over `record/plan/activities`;
  the frontend consumes them via the hand-written `api.ts` + `types.ts`
  (bindings.ts is NOT generated today — `build.rs` is plain `tauri_build`).
- **Timetable**: `[계획|실제|둘 다]` toggle, two-lane (plan dashed/hollow vs
  actual solid/filled), consuming `PlanSlot` + `Record`.
- **Legacy demolition** is the LAST task (`V5__drop_legacy.sql` + remove
  `tasks.rs`/`routines.rs`/`timeline.rs`/`reports.rs`/`cards.rs` legacy paths).

## How to resume

```
cargo test --workspace      # Plan 1 gate — green
# Plan 2: see the plan2 doc, execute task-by-task (SDD or inline)
```
