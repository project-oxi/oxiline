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

## Actual-record editing — 2026-08-05 ✅ COMPLETE

Symmetric completion of the PlanCard move/delete pattern onto *actual-record*
blocks (`ActualLane` → new `ActualBlock`). Plan:
`docs/superpowers/plans/2026-08-05-actual-record-editing.md`, executed inline.

Shipped (all green: `cargo test --workspace`, `clippy -D warnings`, `bun run build`, 14 vitest):
- **Backend**: exposed `edit_record`/`delete_record` as Tauri commands
  (`commands.rs` + `lib.rs` specta builder) — core fns already existed.
- **API/hooks**: `api.editRecord`/`deleteRecord` + `useEditRecord`/`useDeleteRecord`
  (invalidates `day-records`, `records-range`, `compliance`, `recordState`, `slots`).
- **Drag-to-move**: `ActualBlock` pointer-drag → absolute UTC delta (preserves
  duration + DST), 5-min snap, day-window clamp.
- **Hover-delete**: `×` button (group-hover), `useDeleteRecord`.
- **Live guard**: open (`ended_at IS NULL`) blocks are fixed — no move, no ×.
- **Spec**: `doc/09-ui-redesign.md` §9.4.4/§9.8/§9.11/§9.12 updated.

Commits: `14b4777` (commands) → `db2fcdb` (api+hooks) → `deb3078` (drag) →
`a921cf3` (hover-delete).

Note: the plan's `invalidateRecordDerived` listed a `["records"]` query key that
does not exist in the query graph; corrected to `["records-range"]` (the real
range-query key) to avoid a silent invalidation gap.

## Follow-ups (2026-08-05 session 3) ✅ COMPLETE

Shipped (all green: cargo workspace, clippy -D warnings, bun build, 14 vitest):

- **PlanCard Enter / Space** — focuses the card on pointerdown, then Enter or
  Space toggles recording: resolved option if present, else the first option
  (OR plans default to first). Same-activity → stop; otherwise → start. Card is
  keyboard-reachable (`tabIndex={0}`, `role="button"`, `focus-visible` ring).
- **Live hotkey reload** — new `reload_shortcuts` Tauri command calls
  `shortcuts::register_default` (which already unregisters-all + re-registers).
  Preferences' two hotkey inputs invoke it on `onBlur` after the setting write.
  Edits take effect immediately, no relaunch.
- **HUD live tick** — 1-second `setInterval` updates `tick`; `elapsed_seconds`
  and the next-slot `N분 후` countdown refresh while the HUD is open. `nowMin`
  is `baseNowMin + floor(tick/60)` so minute-resolution displays stay correct.

Commits: `95f239a` (PlanCard Enter) → `387409c` (hotkey reload) → `56498b2`
(HUD tick).

## Next session — what remains

- Legacy replacement views (Backlog / Week / Report / RoutineManager). V5
  already drops the legacy *tables* and `0.1.0` strips the legacy frontend
  modules; the legacy *view* entry points remain in the view switch pending
  recording-native replacements. Until those land, removing the legacy views
  entirely would break the app.
- **Plan-block ↔ record-block visual continuity** — landed in session 4 (see
  Phase 2 / session 4 note). The resolved PlanCard already carries the matching
  hue rail, primary-tinted dashed border, and `●` mark before `→실행` so plan ↔
  record reads as one event across the two lanes.

## Phase 2 (2026-08-05 session 5) ✅ COMPLETE

- **워크로드 톤 변화** landed: 모드 토글 바 아래의 `surface-sunken` 얇은 바가 오늘
  `PlanSlot.duration_minute` 합산과 `workload_warning_minutes`(기본 600, 0이면 바 숨김)를
  비교해 `workloadEasy`/`workloadTight`를 토글. 임계 이상이면 `--color-status-warning`
  톤. i18n 키(`timeline.plannedDur` / `workloadEasy` / `workloadTight`)는 ko/en 양쪽
  준비돼 있었음. Commit `fae0e43`. `doc/09` §9.4.1에 명세 추가, `doc/08` roadmap 체크.
- **트레이 진행률** was already on `main` (예전 세션): `tray::render_progress_icon`
  가 22×22 진행 바를 그리고, `lib.rs` setup이 60초 sleep 루프 + `oxiline://db-changed`
  리스너로 `tray::refresh`를 호출해 day-start~end 진행률을 매 1분 / DB 변경 시점에
  갱신. 별도 작업 불필요.

## Plan-card overlap packing (2026-08-05 session 6) ✅ COMPLETE

**Bug:** plan cards sharing a time span painted on top of each other — `PlanCard`
was `absolute` with only `top` computed, no collision resolution.

**Fix — column packing (Google-Calendar style):** new pure fn `lib/layout.ts`
`packColumns(rects) → {col, cols}[]`. Groups overlapping blocks into connected
clusters (transitive interval-overlap), greedy graph-colors each cluster into
columns, half-open intervals so touching blocks don't waste a column.
`PlanLane` memoizes the layout; `PlanCard` renders
`left: calc((col/cols)*100% + 4px)`, `width: calc((1/cols)*100% - 8px)`. Y (time)
is never distorted — only the horizontal share. Non-overlapping → `cols=1`
(full width, identical to before). `ActualBlock` left unchanged (concurrent
records are impossible, so they never overlap).

TDD: 7 unit tests (empty / non-overlapping / 2- & 3-way overlap / chain reuse /
disjoint clusters / input-order preservation). Verified visually in browser via
a throwaway `audit/` Tauri-mock harness (since removed): 3 overlapping cards
fan out side-by-side with 8px gaps, disjoint afternoon card stays full-width.
Spec: `doc/09` §9.4.2 + §9.11 + §9.12 + §9.13 (P7).

## Next session — what remains

- Legacy replacement views (Backlog / Week / Report / RoutineManager). V5
  already drops the legacy *tables* and `0.1.0` strips the legacy frontend
  modules; the legacy *view* entry points remain in the view switch pending
  recording-native replacements. Until those land, removing the legacy views
  entirely would break the app.
  *(Note: `App.tsx` has no view switch today; the legacy view *modules* were
  stripped in 0.1.0 and no legacy entry point is currently mounted. The
  underlying concern is whether a replacement (e.g. a Week view) needs to be
  designed before legacy can be safely reintroduced — open question.)*
- `routine_groups` UI (Phase 2): batch on/off for routine groups. The schema
  and core hooks are not yet built; a separate brainstorming + plan is needed.
- Native macOS notifications (Phase 2): the settings exist
  (`notifications_enabled`, `notification_lead_minutes`) and `notifier.rs` is
  wired, but the opt-in toggle is partially live in Preferences. Verify the
  full flow end-to-end and tighten the UI copy.

## Context menu + HUD polish (2026-08-05 session 7) ✅ COMPLETE

User reported right-click showed only the platform's native webview menu; also
asked to refine the HUD. Done autonomously (user asleep). Spec:
`docs/superpowers/specs/2026-08-05-context-menu-and-hud-design.md`, plan:
`docs/superpowers/plans/2026-08-05-context-menu-and-hud.md`.

**Context menu** (all green: 28 vitest, tsc+vite, cargo workspace, clippy -D warnings;
runtime-verified in browser):
- App-native menu replaces the native webview menu. `lib/context-menu.ts`
  (`useContextMenu` store + pure `clampMenuPosition`) + `components/ContextMenu.tsx`
  (body portal, viewport flip+clamp, ↑↓/Enter/Esc kbd nav, close on outside/blur/scroll).
  Native suppressed via document-level `contextmenu`→`preventDefault` in `main.tsx`/`hud.tsx`.
  `.context-menu` chrome + `ctx-in` keyframe in `styles.css`.
- Wired per surface (mirrors existing direct manipulation — the discoverable 2nd path):
  PlanCard (toggle/delete), ActualBlock (live→stop / past→continue+delete), sidebar
  activity (toggle/delete via new `useDeleteActivity`), timeline background (today/scroll-now).
- Drag handlers guard `e.button !== 0` so right-click never starts a drag.

**HUD polish** (`hud.tsx`; window height 170→200):
- Actionable idle: scheduled → `▶ 지금 시작` (start resolved/first option); free time →
  today's total recorded time (`오늘 Nh Nm 기록`, filtered to local today).
- Active: hue left-rail (matches timeline blocks) + 22px mono elapsed + danger stop.
- Click card → `show_main_window` (new Tauri command `commands.rs` + `lib.rs` specta +
  `api.showMainWindow`); stop/start buttons `stopPropagation`.

Spec updated: `doc/09` §9.7/§9.8/§9.11/§9.12 + new §9.14.

## macOS close/reopen hardening (2026-08-06) ✅ COMPLETE

User shared a debugging lesson (saved as skill `tauri-macos-window-close-vs-
dock-reopen`): a tray-resident macOS app has **two independent OS-event
paths** — close (the red X) and reopen (dock/tray click) — and "won't reopen"
is one symptom hiding both. Both must be overridden explicitly.

Applied the lesson to oxiline (Accessory / menu-bar app, Dock hidden):
- **Close path** — already correct: `lib.rs` `on_window_event` does
  `prevent_close` + `hide` on label `main`. Verified, no change.
- **Reopen path** — real bug found and fixed. All three reopen sites now route
  through one `pub(crate) show_main` (`tray.rs`): the tray menu, the
  `show_main_window` Tauri command (HUD click-to-open — the primary path), and
  the `single_instance` callback (Finder relaunch). Previously each inlined
  `show()` + `set_focus()`. tao 0.35's macOS `set_focus` only calls
  `activateIgnoringOtherApps` when `!is_miniaturized && is_visible`, so a window
  the user **minimized** (yellow dot) stayed buried behind the active app on any
  reopen. `show_main` now does `show()` + `unminimize()` (`NSWindow::deminiaturize`)
  + `set_focus()`, so the precondition holds for both the hide path and the
  minimize path on every reopen route.
- **Tried then reverted** a left-click tray toggle
  (`show_menu_on_left_click(false)` + `on_tray_icon_event`). `TrayIcon::show_menu()`
  does not exist in tauri 2.x, so disabling menu-on-left-click left the tray
  menu (Quit/HUD/quick-add) with no trigger. Kept `show_menu_on_left_click(true)`.

Verified (static; GUI interaction can't be automated headless): `cargo build
--workspace` green, `cargo clippy -p oxiline-app -- -D warnings` clean, tao
source confirms `set_focus`→`activateIgnoringOtherApps:YES` and
`unminimize`→`deminiaturize`. **Runtime click-through (close X → tray Open /
HUD card → main / minimize → reopen) pending a user check on wake.**

Skill updated: `tauri-macos-window-close-vs-dock-reopen` (both policies, the
`set_focus` precondition trap, the missing `show_menu()` trap).

## In-app CLI install (2026-08-07) ✅ COMPLETE

Mirror of `oximemo`'s proven approach — the `oxiline` CLI is now bundled
inside the Tauri `.app` and exposed on `$PATH` via a one-click "Install
command" button (one-time macOS admin prompt). One `.dmg` install now
delivers both the GUI and `oxiline`. Spec: `docs/superpowers/specs/2026-08-07-in-app-cli-install.md`,
plan: `docs/superpowers/plans/2026-08-07-cli-install.md`.

Shipped (all green: `cargo test --workspace --locked` 41 pass + 6 new unit tests, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --all -- --check` clean, `bun run build` PASS, `bun test` 28 pass, `cargo build -p oxiline-app --locked` PASS, `CI=false cargo tauri build --debug` produces a `.app` with `Contents/MacOS/oxiline` verified):

- **Bundle config** — `tauri.conf.json` `bundle.externalBin: ["binaries/oxiline"]`; `binaries/` gitignored.
- **`build.rs`** — drops a placeholder `binaries/oxiline-<triple>` when missing so `cargo check` / `clippy` / `tauri dev` survive without the real CLI; release workflow stages the real one. Verified gotcha #1 (tauri-build validates at compile time).
- **Tauri commands** — new `cli.rs` module: `CliState` (`installed` | `not-installed` | `stale`, `serde + specta rename_all = "lowercase"`), `cli_status` / `install_cli` / `uninstall_cli` (`#[tauri::command] #[specta::specta]`), plus `bundled_cli_path()` (derived from `current_exe().parent().join("oxiline")` — tracks wherever the user dropped the `.app`), `run_admin` via `osascript -e "do shell script ... with administrator privileges"`, and `applescript_string` (escapes `"` and `\`). Pure `classify()` helper is unit-tested with 5 cases (no bundle → NotInstalled; missing link → NotInstalled; stray file → Stale; matching symlink → Installed; diverging symlink → Stale). `bundled_cli_path` runtime-resolution gotcha (#4 in spec) covered.
- **Frontend** — `lib/api.ts` adds `cliStatus` / `installCli` / `uninstallCli` with an `inTauri` gate (browser/dev falls back to `"not-installed"`; install/uninstall throw). `hooks.ts` adds `useCliStatus` (staleTime Infinity) + `useInstallCli` / `useUninstallCli` (invalidate `["cli-status"]`). `types.ts` adds `CliState`.
- **Settings → Command-line tool** — new section in `Preferences.tsx` mirroring oximemo's `CliSection`: status pill (`Installed`/`Not installed`), one button (Install / Reinstall / Uninstall depending on state), disabled + "…" while the mutation is in flight.
- **First-launch nudge** — new `<CliNudge />` mounted in `App.tsx`. Shown only in the Tauri shell (gated on `"__TAURI_INTERNALS__" in window`), only when `cli_status !== "installed"`, dismissed via `localStorage["oxiline.cliNudgeDismissed"] = "1"`. "Install now" → `installCli`; "X" → dismiss.
- **i18n** — `ko.json` source of truth, `en.json` mirror. New keys: `sectionCli`, `cliDesc`, `cliInstall`/`Uninstall`/`Reinstall`/`Installing`, `cliInstalled`/`NotInstalled`, `cliInstallDone`/`Failed`/`UninstallDone`, `cliNudgeTitle`/`Body`/`Install`/`Dismiss`.
- **Local helper** — `stage-cli.sh` (executable) stages the release-mode CLI binary for a genuine local `cargo tauri build`. Smoke-tested: `binaries/oxiline-aarch64-apple-darwin --help` prints the full command tree.
- **Release workflow** — `release.yml` `app` job: new "Build CLI sidecar" + "Stage CLI sidecar for app bundle" steps ahead of `tauri-action` (aarch64 CLI binary → `binaries/oxiline-aarch64-apple-darwin`).

Verified the gotcha #2 (Tauri strips the `-<triple>` suffix when bundling):
```
ls target/debug/bundle/macos/OxiLine.app/Contents/MacOS/
# oxiline       ← the sidecar (no -<triple> suffix)
# oxiline-app   ← the main binary
```

Commits: `00fcbbc` (build.rs + externalBin) → `0160097` (cli.rs + lib.rs) →
`55f97c1` (api+hooks) → `482a38f` (Preferences+CliNudge+i18n) → `683240c`
(stage-cli.sh + release.yml) → `573f255` (cargo fmt).

**Live check pending**: GUI button → macOS admin dialog → `which oxiline`
cannot be automated headlessly (needs an interactive admin password).
Verify on the next release tag or a locally-built `.app`.