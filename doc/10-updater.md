# OxiLine in-app update architecture

Status: **Implemented**. The CLI is the only engine (`oxiline upgrade`);
the GUI is a thin view that spawns the bundled `oxiline` sidecar with
`--json-progress` and parses its NDJSON progress contract. The Tauri
updater plugin is removed; the minisign pubkey lives in
`crates/oxiline-cli/src/upgrade.rs#LIVE_PUBKEY` and is verified against
the live release by the `#[ignore]`'d
`verifies_live_release_signature` test (`cargo test -- --ignored`).


## What stays the same as the canonical RFC

- The CLI is the only place that downloads, verifies, and swaps.
- The GUI is a thin view: it spawns the sidecar, parses JSON progress
  events, and shows the UI.
- The only IPC between them is `settings.update_request_at`, whose meaning
  is "I just finished swapping; please relaunch yourself."
- `tauri-plugin-process::relaunch()` is the GUI's relaunch primitive.

## What is OxiLine-specific

- Release host: `https://github.com/project-oxi/oxiline/releases/latest/download/latest.json`
- Bundle: `OxiLine.app.tar.gz` (signed, minisign, public key embedded in the
  CLI constant)
- Standalone CLI tarball: `oxiline-aarch64-apple-darwin.tar.gz` (already
  shipped today; just needs the same swap logic the oximemo CLI has)
- `latest.json` notes field: `"OxiLine $VERSION"`
- App identifier: `com.oxiline.app` (used for the user-data dir and the
  relaunch signal settings DB)
- GUI entry: `crates/oxiline-app/src-tauri/` (Tauri 2)
- CLI entry: `crates/oxiline-cli/src/main.rs::Command::Update` (the
  command that today just writes the setting)

## OxiLine today vs. unified (delta)

| Concern | OxiLine today | After unification |
|---|---|---|
| GUI check/install | `lib/updater.ts` uses `@tauri-apps/plugin-updater` | `lib/updater.ts` spawns the `oxiline` CLI as a sidecar and parses its JSON events |
| GUI relaunch | `tauri-plugin-process` | unchanged |
| GUI auto-check | boot + 6h (`useUpdate` in `main.tsx`) | unchanged (drives the sidecar spawn) |
| `update_request_at` | set by `oxiline update`, watched by `App.tsx` | set by `oxiline upgrade` after a successful swap, watched by the same `App.tsx` code |
| CLI `update` | writes the timestamp and exits; CLI does not download or verify | `oxiline upgrade` does the full download → verify → swap → writes the timestamp on success |
| Minisign pubkey | embedded in `tauri.conf.json` → `plugins.updater.pubkey` | moves into the CLI's `PUBKEY` constant |
| Tauri updater plugin | `tauri-plugin-updater`, `tauri-plugin-process` | remove `tauri-plugin-updater`; keep `tauri-plugin-process` |
| `bundle.createUpdaterArtifacts` | `true` | `false` (manifest is built with `jq` in `release.yml`, exactly as today) |
| Standalone CLI updates | impossible (GUI is the only engine) | works (CLI is the engine) |

## Migration steps for OxiLine

1. **CLI side first.** Port the oximemo `upgrade.rs` into
   `crates/oxiline-cli/src/upgrade.rs`. Same shape:
   - `fetch_manifest()` against the oxiline `latest.json` URL.
   - `verify_minisign()` with the same `pubkey` already in
     `tauri.conf.json` (move it from the Tauri config into the CLI
     constant).
   - `upgrade_in_app()` for the sidecar-running case.
   - `upgrade_standalone()` for the `~/.cargo/bin/oxiline` case.
   - JSON progress events on stdout when `--json-progress` is set.
   - `oxiline upgrade --check` already exists as `oxiline update --check`;
     rename and absorb, or add `upgrade` as the new name and keep `update`
     as a deprecation alias. Decision belongs to the migration PR.
   - On success, `settings::set(&conn, "update_request_at",
     &Value::String(util::now_iso()))` (this is the only side-effect the
     GUI cares about).
2. **GUI side.** Rewrite `lib/updater.ts` to spawn the sidecar via
   `@tauri-apps/plugin-shell` and parse the JSON contract. Keep the
   zustand store, the banner, the Preferences section, and the
   `update_request_at` watcher. Remove the import of
   `@tauri-apps/plugin-updater`.
3. **Tauri config.** Remove `tauri-plugin-updater` from
   `crates/oxiline-app/src-tauri/Cargo.toml` and from
   `package.json`. Remove `plugins.updater` and
   `bundle.createUpdaterArtifacts` from `tauri.conf.json`. Remove
   `updater:default` from `capabilities/default.json`.
4. **Release pipeline.** Keep the `release.yml` `jq` manifest step
   unchanged — it is the only remaining way to emit `latest.json` once
   `createUpdaterArtifacts` is gone. The same OxiLine signatures already
   used today continue to work.

## Open questions (same as the canonical RFC)

- **End-to-end live swap is not yet exercised** in OxiLine either. The
  oximemo CLI's verify path is covered by an ignored network test (passing
  on the live v0.9.0 bundle). The download → extract → rename → relaunch
  path is verified only at the code level. Before removing
  `tauri-plugin-updater`, someone must run a real CLI swap against a newer
  release while the GUI is running.
- **Quarantine xattr** on the freshly extracted `.app` under
  `/Applications`. The CLI must `xattr -dr com.apple.quarantine` the new
  bundle as the last step before signalling relaunch, if Gatekeeper
  objects. To confirm during the first end-to-end probe.
- **Codesign identity** after the CLI extracts the bundle into a new
  location. The ad-hoc signature is on individual binaries, not directory
  paths, so it should travel with the files. To confirm during the
  end-to-end probe.
- **Concurrent CLI invocations.** A flock on the OxiLine user-data
  directory would prevent two `oxiline upgrade` processes from racing on
  the same `.app`. Not required for v1, noted for v2.

## Why this is a win for OxiLine specifically

- **The standalone CLI on headless / agent / CI machines finally gets a
  self-update path.** Until now, an `oxiline` binary installed via
  `cargo install` or copied to `/usr/local/bin` is stranded on the version
  it was installed at. With the unification, the same CLI updates itself
  the same way `oximemo` does.
- **The GUI's existing UX does not change much.** The auto-check cadence,
  the banner, the Preferences section, and the `update_request_at`
  watcher are all already in OxiLine. The visible difference is "the
  progress UI is powered by a sidecar spawn instead of a Tauri plugin";
  the user cannot tell.
- **The verification path has one home.** Today, the Tauri updater plugin
  signs off; the CLI is not in the loop. After the unification, the CLI
  is the verifier, and the GUI is just the consumer of its verdict.
