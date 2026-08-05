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


## UI Redesign — 2026-08-05 ✅ COMPLETE (commit `0ae3da3`)

Addressed the UX regression reported against the Plan 2 GUI: the surface felt
read-only and undifferentiated. Diagnosis + rationale: `doc/09-ui-redesign.md`.

Shipped (all green: tsc+vite build, 14 FE tests, clippy -D warnings, workspace tests):
- **Visual**: 3-tier pane surfaces (sidebar `surface-sunken` / timeline `surface` /
  inspector `surface-raised`) + header chrome bar; Oxide Bar mounted in the main
  window (was HUD-only) with click→scroll via `useUi.requestScroll`.
- **Timeline direct-create**: click empty plan lane → inline DraftBlock quick-add;
  drag → rubber-band block; sidebar-drag drop highlight (`isOver` ring).
- **Recording entry**: header transport + NowCard start CTA + per-card hover
  quick-record; **global ⌘⇧R quick-toggle** (new `quick_record_hotkey` setting +
  `oxiline://quick-record` event; resume last activity / stop).
- **Sidebar**: ＋ inline activity creation + CTA empty state.
- **HUD**: actionable stop button. **Preferences**: quick-record hotkey + full table.
- Removed orphaned `NowLine.tsx`.

Store additions (`lib/store.ts`): `lastActivityId`, `scrollTarget`/`requestScroll`.
Runtime-verified in browser: date popover, activity-add, click- and drag-to-create.

`global_hotkey`/`quick_record_hotkey` edits in Preferences still only re-register
on next launch (pre-existing behavior for `global_hotkey`); a live re-register
command is a future nicety, not a regression.

## UI polish — 2026-08-05 (session 2)

Follow-on refinements on top of the redesign (all green: cargo workspace, bun
build, 14 vitest):
- **Header collapsed to a single command row + oxide strip** (~85px returned
  to the timeline): date masthead, weekday chips with micro oxide bars, record
  hero pill, ⌘K/⚙. Calendar popover is now a body portal (drag-region-free).
- **Inline draft editor**: borderless in-place input, ⏎/esc kbd hints inline.
- **Plan-block move + delete**: drag body = move (`update_plan`, start-only,
  recurring mask preserved via `PlanSlot.weekday_mask`); hover × = delete.
- Bug fixes: ActualLane + RecentSessions show activity names (was raw ids);
  now-line spans both lanes; tab active-state contrast.
- `doc/09-ui-redesign.md` rewritten as the canonical UI/interaction spec.

Commits: `a7073e5` (draft editor), `c448fb8` (move/delete + core
`weekday_mask`), `2692e5c` (spec rewrite). Spec: `doc/09-ui-redesign.md`.

## Next session — actual-record editing

Symmetric completion: move/delete on *actual-record* blocks (ActualLane).
Plan ready at `docs/superpowers/plans/2026-08-05-actual-record-editing.md` —
core `edit_record`/`delete_record` already exist; only Tauri command exposure
(`commands.rs` + `lib.rs` builder) + frontend gestures (`ActualBlock` drag +
hover ×) remain. Execute via subagent-driven-development or executing-plans.