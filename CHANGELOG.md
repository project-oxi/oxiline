# Changelog

All notable changes to OxiLine will be documented in this file.

## [0.7.0] - 2026-08-09

Unified self-update architecture — the CLI is the only engine; the GUI
is a thin view that spawns the bundled `oxiline` sidecar and parses its
NDJSON progress contract. Mirrors the proven `oximemo` flow (spec:
`doc/10-updater.md`, plan: `docs/superpowers/plans/2026-08-09-unified-updater.md`).
Net effect for downstream users: the standalone `oxiline` binary
(e.g. on headless / agent / CI machines) finally has a self-update path
it never had under the Tauri-only engine.

### CLI (`oxiline-cli`)

- **feat**: `oxiline upgrade [--check] [--json-progress] [-y]` — the new
  canonical self-update subcommand. Downloads the platform asset, verifies
  the minisign signature (or SHA-256 for the standalone tarball), and
  swaps in place. Emits one JSON object per line on stdout when
  `--json-progress` is set (`checking` / `available` / `download` /
  `verifying` / `swapping` / `done` / `error`); the GUI sidecar parses
  this stream verbatim.
- **feat (TDD)**: 22 unit tests in `crates/oxiline-cli/src/upgrade.rs`
  pin the wire contract, the manifest schema, the minisign verify path
  (round-trip against the well-known `minisign-verify` test vector), the
  streaming download, the tarball extract, the in-app atomic rename, and
  the standalone SHA-256 path. An `#[ignore]`'d live test verifies
  the OxiLine release signature against the public `latest.json`:
  ```sh
  cargo test -p oxiline-cli --bin oxiline --release \
    -- --include-ignored verifies_live_release_signature
  ```
- **feat**: `oxiline update` is preserved as a hidden deprecated alias
  for one release; it prints a deprecation notice and forwards to
  `oxiline upgrade`. Existing user scripts keep working.
- **refactor**: drop dead `fetch_latest_release_version` and
  `is_up_to_date` from `main.rs` (the engine in `upgrade.rs` owns these
  now; standalone CLI now uses semver `parse_version` instead of the
  loose string-compare fallback).

### Desktop app (`oxiline-app`)

- **refactor**: drop `tauri-plugin-updater` and
  `@tauri-apps/plugin-updater`; add `tauri-plugin-shell` for the sidecar
  spawn. `bundle.createUpdaterArtifacts` and `plugins.updater` removed
  from `tauri.conf.json`; the minisign pubkey lives only in
  `crates/oxiline-cli/src/upgrade.rs::LIVE_PUBKEY`. The GUI is a pure
  view — it never downloads, verifies, or swaps a release.
- **feat**: `lib/updater.ts` rewritten to spawn `binaries/oxiline`
  via `@tauri-apps/plugin-shell` `Command.sidecar`, parse the NDJSON
  stream, and drive the same `useUpdate` zustand store. 7 vitest cases
  pin the reducer behavior (full check → install sequence, monotonic
  download pct, swapping → restarting state).
- **feat**: new "restarting" status in Preferences → Updates (renders
  with `updater.restarting` i18n key, ko + en) — shown between the
  successful swap and the GUI's own `tauri-plugin-process::relaunch()`.
- **i18n**: `updater.restarting` key (ko + en).

### Release pipeline (`.github/workflows/release.yml`)

- **feat**: explicit `Package app bundle as .app.tar.gz` step tars the
  tauri-action `.app` into the canonical `OxiLine.app.tar.gz` name the
  manifest + minisign + upload steps expect (this replaces Tauri's
  removed `createUpdaterArtifacts`).
- **feat**: explicit `Sign the app bundle with minisign` step produces
  the `.app.tar.gz.sig` artifact. Requires the
  `OXILINE_MINISIGN_KEY` repository secret (base64 of the
  `minisign` secret key matching `LIVE_PUBKEY`).

### Release engineering — required action

Before the next tagged release, add the `OXILINE_MINISIGN_KEY` secret
to the repository (`Settings → Secrets and variables → Actions`):
```sh
# 1) Locally, generate the minisign keypair (one-time). The public half
#    (oxi-pub.key) MUST match the inner `RWQ…u` base64 line in
#    `crates/oxiline-cli/src/upgrade.rs::LIVE_PUBKEY` — otherwise
#    every existing 0.x install will fail to verify a v0.7.0 update.
minisign -G -p oxi-pub.key -s oxi.key
diff <(awk 'NR==2' oxi-pub.key) <(echo RWQWUGOnd35Vhu5+pjNhZ5pBjd4N+1YTz8nsdTFllvnrCZ79HSav7B3u) \
  || { echo "oxi-pub.key does not match LIVE_PUBKEY — STOP"; exit 1; }

# 2) Push the secret to GitHub. The minisign `.key` file is a small
#    text file (≈ 200 bytes) with a `untrusted comment:` line + a
#    base64 line; `gh secret set` preserves newlines, so pass the raw
#    file contents (do NOT base64-encode it — `release.yml` writes the
#    secret verbatim to /tmp/oxiline.key and `minisign -S` rejects a
#    base64 blob in place of the real .key file).
gh secret set OXILINE_MINISIGN_KEY -R project-oxi/oxiline < oxi.key

# 3) Verify the round-trip locally:
gh secret get OXILINE_MINISIGN_KEY -R project-oxi/oxiline > /tmp/oxiline.key
chmod 600 /tmp/oxiline.key
minisign -V -p oxi-pub.key -m README.md 2>/dev/null && echo "secret matches pub"
rm /tmp/oxiline.key oxi.key oxi-pub.key
```

**Why raw, not base64:** `release.yml` runs
`echo "$OXILINE_MINISIGN_KEY" > /tmp/oxiline.key` and then
`minisign -S -s /tmp/oxiline.key`. minisign parses the file as the
minisign key format (`untrusted comment: …` + base64 secret) — NOT
as a base64 blob of a base64 blob. If the secret holds
`base64(oxi.key)`, the echoed file is just the base64 string with no
`untrusted comment:` header and minisign refuses it.


### Breaking changes (none at the user surface)

- CLI command rename: `oxiline update` → `oxiline upgrade` (old name is
  preserved as a hidden deprecation alias that forwards).
- Capability permission `updater:default` removed; `shell:allow-execute`
  added (the GUI now spawns the sidecar through the shell plugin).
- Frontend dep swap: `@tauri-apps/plugin-updater` removed;
  `@tauri-apps/plugin-shell` added.

## [0.6.1] - 2026-08-09

### CLI (`oxiline-cli`)

- **feat**: `oxiline update` — checks GitHub Releases for a newer version and
  asks the running app to install it (writes the `update_request_at` setting,
  mirroring the `hud` pattern). The app's updater replaces the whole `.app`
  (CLI sidecar included), so GUI and CLI advance together. `--check` reports
  without installing.
### Desktop app (`oxiline-app`)

- **feat**: react to a CLI `oxiline update` request by running the updater
  immediately (download + relaunch), so the two surfaces stay in sync.

## [0.6.0] - 2026-08-09

### Desktop app (`oxiline-app`)

- **feat**: in-app auto-update — checks GitHub Releases on launch and every
  6h; a top banner and Preferences → "Updates" offer one-click install
  (download + relaunch). Backed by `tauri-plugin-updater` +
  `tauri-plugin-process`; the release workflow signs `.app.tar.gz` and
  publishes a `latest.json` manifest.
- **fix**: Preferences → About shows the real app version (was a stale
  "0.1.0" placeholder).

### CLI (`oxiline-cli`)

- **fix**: `doctor` has a help description (was blank in `--help`).
- **fix**: `now --json` includes `generated_at` (spec §5.3) so agents can
  stamp the snapshot without re-reading the system clock.
- **fix**: `activity_rm_force_required_when_records_exist` test opened a
  second unmigrated connection; now uses the migrated handle.

## [0.5.0] - 2026-08-08

Menu-bar multi-slot display — CodexBar-style status bar replaces the single
22×22 progress-bar tray. Each enabled information slot gets its own
`NSStatusItem`, ordered left → right per Preferences.

### Desktop app (`oxiline-app`)

- **feat**: multi-slot menu bar — `now_recording` (REC … Nm), `now_next`
  (NEXT … Nm), and `state_dot` (color-coded: green=recording, amber=next
  ≤5m, gray=idle). One slot per `NSStatusItem`, ordered per Preferences.
- **feat**: always-on menu slot preserves the context menu even when all
  data slots are off (CodexBar "merged" pattern).
- **feat**: bitmapped tray renderer — inline 5×7 column-major ASCII font
  rasterizes labels into 22 px RGBA; non-ASCII falls back to a 5×7 box
  (v1 limitation, spec §6.4/§11).
- **feat**: Preferences → "메뉴바 표시" section with on/off toggle and
  ▲/▼ reorder for each slot.
- **feat**: `update_tray_slots` Tauri command + `oxiline://tray-changed`
  event for immediate rebuild on preference change.
- **fix**: `tray_slots::resolve` fills missing canonical kinds with
  defaults (spec §4 forward-compat).
- **fix**: `tray::build` is idempotent for the menu tray — no duplicate
  `NSStatusItem` per preference toggle (spec §6.3).
- **fix**: `state_dot` keeps literal RGB via template-mode opt-out
  (spec §5 — text slots stay template-adapted).
- **fix**: legacy 22×22 progress-bar tray removed (illegible at menu-bar
  resolution).

### Recording core (`oxiline-core`)

- **feat**: `TraySlotKind` enum + `TraySlotPref { kind, on, order }` typed
  preferences persisted as a single JSON row under settings key
  `tray_slots` (migration V6).
- **feat**: `tray_slots::resolve` / `save` / `defaults` helpers.

## [0.4.0] - 2026-08-07

In-app CLI install — one `.dmg` now delivers both the GUI and the `oxiline`
command on `$PATH`. Mirrors the proven `oximemo` flow (Tauri `externalBin`
sidecar + one-time macOS admin prompt via `osascript`).

### Desktop app (`oxiline-app`)

- **feat**: bundle the `oxiline` CLI inside the `.app` as a Tauri
  `externalBin` sidecar; Settings → "Command-line tool" installs it onto
  `/usr/local/bin/oxiline` via a single macOS admin prompt
  (`cli_status` / `install_cli` / `uninstall_cli` Tauri commands + pure
  `classify()` helper with 5 unit tests)
- **feat**: first-launch nudge banner (top-center, dismissed via
  `localStorage`; only in the Tauri shell, never in browser/dev)
- **build**: `build.rs` drops a placeholder sidecar when missing so
  `cargo check` / `clippy` / `tauri dev` survive without the real CLI;
  release workflow stages the real one before `tauri-action`
- **build**: `stage-cli.sh` local helper for genuine `cargo tauri build`
- **i18n**: ko/en strings for the CLI section + nudge


## [0.3.1] - 2026-08-06

Release alignment and packaging metadata for the updated SQLite stack.

### Packaging and dependencies

- **fix**: publishable `oxiline-core` and `oxiline-cli` manifests now include crates.io descriptions
- **chore**: migrate the SQLite stack to `rusqlite` 0.40, `rusqlite_migration` 2.6, and `r2d2_sqlite` 0.35
- **chore**: raise the workspace MSRV to Rust 1.95

## [0.3.0] - 2026-08-06

Timeline interaction, recording controls, HUD polish, and macOS window behavior.

### Desktop app (`oxiline-app`)

- **feat**: single-row header + oxide strip (date masthead, weekday chips with
  micro oxide bars, record hero pill, calendar body-portal popover)
- **feat**: inline draft editor for plan quick-add (borderless in-place input,
  ⏎/esc kbd hints)
- **feat**: plan-block move + delete (drag body → `update_plan` start-only,
  recurring mask preserved; hover × → `delete_plan`)
- **feat**: actual-record block move + delete (drag → `edit_record` absolute
  UTC delta with 5-min snap + day-window clamp; hover × → `delete_record`;
  live open sessions are fixed until stopped)
- **feat**: expose `edit_record` / `delete_record` Tauri commands
- **fix**: actual lane + recent sessions show activity names (was raw ids)
- **fix**: now-line spans both lanes

## Follow-ups (2026-08-05 session 3)

- **feat**: PlanCard Enter / Space toggles recording (resolved option if present,
  else first option; OR plans default to first)
- **feat**: live re-register of `global_hotkey` / `quick_record_hotkey` from
  Preferences (no relaunch); new `reload_shortcuts` Tauri command

## H4 polish (2026-08-05 session 4)

- **polish**: resolved PlanCard now wears a 3px left rail in the resolved
  option's hue, a primary-tinted dashed border, and a `●` mark before `→실행`,
  so the plan card visually lines up with the matching ActualBlock in the
  other lane.

## Phase 2 (2026-08-05 session 5)

- **feat**: 워크로드 톤 변화 — 타임라인 모드 토글 바 아래의 `surface-sunken` 얇은 바에
  "오늘 계획 Xh Ym · 여유 있음 / 빠듯해요"를 표시. `workload_warning_minutes`(기본 600, 0이면
  비활성) 이상이면 `--color-status-warning` 톤. 클릭/모달 없음.

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

[0.3.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.3.0

[0.3.1]: https://github.com/project-oxi/oxiline/releases/tag/v0.3.1

[0.4.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.4.0
[0.5.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.5.0
[0.6.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.6.0
[0.7.0]: https://github.com/project-oxi/oxiline/releases/tag/v0.7.0
[0.6.1]: https://github.com/project-oxi/oxiline/releases/tag/v0.6.1
