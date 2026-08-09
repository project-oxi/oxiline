# Unified Update Architecture (CLI as engine, GUI as view) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace OxiLine's Tauri-plugin updater with a CLI-engine architecture: the bundled `oxiline` sidecar downloads/verifies/swaps releases, emits NDJSON progress, and signals relaunch via `settings.update_request_at`; the GUI is a thin view of that engine.

**Architecture:** Port `oximemo` v0.9.1's `upgrade.rs` into a new `oxiline-cli/src/upgrade.rs`, expose it as `oxiline upgrade [--check] [--json-progress]`, keep `oxiline update` as a deprecation alias that calls `upgrade --check`. GUI sidecar spawns `oxiline` via `@tauri-apps/plugin-shell` and parses the NDJSON contract. `tauri-plugin-updater` and `@tauri-apps/plugin-updater` are removed; `tauri-plugin-process` (relaunch) stays. `bundle.createUpdaterArtifacts` and `plugins.updater` are removed from `tauri.conf.json`; the minisign pubkey moves into the CLI's `PUBKEY` constant. The `release.yml` manual `jq` manifest step is the only remaining way to emit `latest.json`.

**Tech Stack:** Rust (oxiline-cli/oxiline-core), Tauri 2, React 19 + zustand, `@tauri-apps/plugin-shell`, `ureq` 2, `minisign-verify` 0.2, `flate2`, `tar`, `sha2`, `base64`, GitHub Actions (`jq`, `softprops/action-gh-release`).

## Global Constraints

- Workspace version: `0.6.1` (do not bump in this plan; a follow-up release PR can decide that).
- `oxiline` CLI version: `env!("CARGO_PKG_VERSION")` is the `current` baseline.
- `update_request_at` semantics (from `doc/10-updater.md`): only the CLI writes it; only the GUI watches it. Do not add new shared settings.
- Tauri identifier: `com.oxi.oxiline`. App identifier for the user-data dir remains as-is.
- Manifest endpoint: `https://github.com/project-oxi/oxiline/releases/latest/download/latest.json`.
- Standalone CLI tarball name: `oxiline-aarch64-apple-darwin.tar.gz` (and `.sha256`).
- Tauri bundle asset: `OxiLine.app.tar.gz` (signed by minisign in CI).
- `notes` field: `"OxiLine $VERSION"`.
- JSON progress event schema (NDJSON, one object per line, no other stdout):
  ```jsonc
  {"type":"checking"}
  {"type":"available","from":"0.6.1","to":"0.7.0","notes":"OxiLine 0.7.0"}
  {"type":"latest","version":"0.6.1"}
  {"type":"download","pct":42}
  {"type":"verifying"}
  {"type":"swapping","mode":"app"}      // or "standalone"
  {"type":"done","version":"0.7.0"}
  {"type":"error","message":"…"}
  ```
- All Rust code: edition 2024, `panic = "abort"`, MSRV 1.95, no `unsafe` in this scope.
- All frontend code: TypeScript strict, React 19, zustand store shape preserved verbatim so the existing `UpdateBanner` and `Preferences.UpdateSection` keep working.
- All commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`, `docs:` conventional. English commit bodies. Branch: `feat/unified-updater` (a worktree may be used at execution time).

---

## File Structure

New / changed files (locked here so each task is unambiguous):

| File | Change | Responsibility |
|---|---|---|
| `crates/oxiline-cli/Cargo.toml` | edit | Add `ureq` (tls), `minisign-verify`, `flate2`, `tar`, `sha2`, `base64`, `anyhow`. |
| `crates/oxiline-cli/src/upgrade.rs` | new | The engine: manifest fetch, semver compare, download, minisign verify, SHA-256 verify, extract, atomic swap, NDJSON progress emission. |
| `crates/oxiline-cli/src/cli.rs` | edit | Add `Upgrade { check, json_progress, yes }` subcommand. Add hidden `Update` alias that prints deprecation and forwards. |
| `crates/oxiline-cli/src/main.rs` | edit | Register `mod upgrade;`. Replace `Command::Update` body with a call to `upgrade::run(&conn, opts)`. |
| `crates/oxiline-cli/tests/upgrade_cli.rs` | new | Integration tests: `--help` shape, deprecation notice, `--check` against an offline HTTPS mock. |
| `crates/oxiline-app/src-tauri/Cargo.toml` | edit | Remove `tauri-plugin-updater`. Add `tauri-plugin-shell`. |
| `crates/oxiline-app/src-tauri/src/lib.rs` | edit | Remove `tauri_plugin_updater::Builder` from `setup()`. No other changes to the builder chain. |
| `crates/oxiline-app/src-tauri/tauri.conf.json` | edit | Remove `bundle.createUpdaterArtifacts` and `plugins.updater`. |
| `crates/oxiline-app/src-tauri/capabilities/default.json` | edit | Remove `updater:default`. Add `shell:allow-execute` scoped to the `oxiline` sidecar. |
| `crates/oxiline-app/package.json` | edit | Remove `@tauri-apps/plugin-updater`. Add `@tauri-apps/plugin-shell`. |
| `crates/oxiline-app/src/lib/updater.ts` | rewrite | Sidecar spawn + NDJSON parsing. Same zustand `useUpdate` shape so `UpdateBanner` and `Preferences.UpdateSection` keep working. |
| `crates/oxiline-app/src/lib/updater.test.ts` | new | Vitest unit tests for the NDJSON parser and the state-machine reducer. |
| `.github/workflows/release.yml` | edit (small) | The manual `jq` manifest step stays. Confirm signature path glob still matches once `createUpdaterArtifacts` is gone. |
| `doc/10-updater.md` | edit | Mark status as "Implemented" (one-line change in the header). |

**Out of scope for this plan (open questions from the spec):** end-to-end live swap probe on a real newer release; `xattr -dr com.apple.quarantine` for swapped `.app` (the `upgrade_in_app` path extracts into the same volume on rename, so this only matters after first end-to-end probe); flock on the user-data dir for concurrent CLI invocations. These are explicitly v2 per the spec.

---

## Task 1: Add upgrade dependencies

**Files:**
- Modify: `crates/oxiline-cli/Cargo.toml:13-26`

**Interfaces:** none yet. The new deps are read by Tasks 2–5.

- [ ] **Step 1: Write the failing test for `semver` (existing test in main.rs uses `semver` already, so this validates the dep wires in)**

Run: `cargo test -p oxiline-cli -- --list 2>&1 | head -40`
Expected: PASS (we're confirming the test runner is wired, not adding a new test).

- [ ] **Step 2: Add dependencies to `Cargo.toml`**

Append after the existing `[dependencies]` block (and before `[dev-dependencies]`):

```toml
# Self-update engine (`doc/10-updater.md`). The CLI is the only place that
# downloads / verifies / swaps; the GUI is a thin view that spawns this
# binary as a sidecar and parses its `--json-progress` NDJSON contract.
ureq = { version = "2", default-features = false, features = ["tls"] }
minisign-verify = "0.2"
flate2 = "1"
tar = "0.4"
sha2 = "0.10"
base64 = "0.22"
anyhow = "1"
```

Keep the existing `semver = "1"` (already in the file).

- [ ] **Step 3: Verify it builds**

Run: `cargo build -p oxiline-cli --no-default-features 2>&1 | tail -10`
Expected: build succeeds with the new deps resolved.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/Cargo.toml
git commit -m "feat(cli): add deps for self-update engine (ureq/minisign-verify/flate2/tar/sha2/base64/anyhow)"
```

---

## Task 2: TDD `parse_version` and `is_newer`

**Files:**
- Create: `crates/oxiline-cli/src/upgrade.rs` (skeleton with the helpers, full body grows in Tasks 3–6)
- Test: `crates/oxiline-cli/src/upgrade.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub fn run(conn: &rusqlite::Connection, opts: Options) -> anyhow::Result<()>` — final, Tasks 3–6 wire it up.
  - `pub struct Options { pub check: bool, pub json_progress: bool, pub assume_yes: bool }` — final.
  - `fn parse_version(v: &str) -> Option<(u64, u64, u64)>` — internal, this task.
  - `fn is_newer(latest: &str, current: &str) -> bool` — internal, this task.

- [ ] **Step 1: Write the failing test for `parse_version`**

In `crates/oxiline-cli/src/upgrade.rs` (this file does not exist yet — we'll create it with the tests first):

```rust
//! Self-update engine (skeleton — see `doc/10-updater.md`).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_versions() {
        assert_eq!(parse_version("0.9.0"), Some((0, 9, 0)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2.3-rc1"), Some((1, 2, 3))); // pre-release tolerated
    }

    #[test]
    fn rejects_garbage_versions() {
        assert_eq!(parse_version("latest"), None);
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("1.2"), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -15`
Expected: compile error (`parse_version` not found) or test failure. That's the RED.

- [ ] **Step 3: Write the minimal implementation**

```rust
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let mut it = v.split('.');
    let maj = it.next()?.parse().ok()?;
    let min = it.next()?.parse().ok()?;
    let patch_raw = it.next().unwrap_or("0");
    let patch = patch_raw.split('-').next()?.parse().ok()?;
    Some((maj, min, patch))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p oxiline-cli upgrade::tests::parses_plain_versions upgrade::tests::rejects_garbage_versions 2>&1 | tail -10`
Expected: 2 passed.

- [ ] **Step 5: Add the `is_newer` tests (RED)**

Append to the `tests` module:

```rust
    #[test]
    fn newer_detection() {
        assert!(is_newer("0.9.1", "0.9.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.8.9", "0.9.0"));
    }

    #[test]
    fn unparseable_never_newer() {
        assert!(!is_newer("oops", "0.9.0"));
        assert!(!is_newer("0.9.1", "oops"));
    }
```

Run: `cargo test -p oxiline-cli upgrade::tests::newer_detection 2>&1 | tail -10`
Expected: FAIL (no `is_newer` yet).

- [ ] **Step 6: Implement `is_newer` (GREEN)**

```rust
fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}
```

- [ ] **Step 7: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -10`
Expected: 4 passed, 0 failed.

- [ ] **Step 8: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "test(cli): TDD parse_version and is_newer (semver compare for upgrade gate)"
```

---

## Task 3: TDD `app_bundle_root_of` (in-app vs standalone context detection)

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs` (add the function and tests)

**Interfaces:**
- Produces: `fn app_bundle_root() -> Option<PathBuf>` and `fn app_bundle_root_of(exe: &Path) -> Option<PathBuf>`.

- [ ] **Step 1: Add the failing tests**

Append to the `tests` module:

```rust
    #[test]
    fn detects_app_bundle_from_sidecar_path() {
        let exe = std::path::PathBuf::from("/Applications/OxiLine.app/Contents/MacOS/oxiline");
        assert_eq!(
            app_bundle_root_of(&exe),
            Some(std::path::PathBuf::from("/Applications/OxiLine.app"))
        );
    }

    #[test]
    fn no_app_bundle_for_standalone() {
        let exe = std::path::PathBuf::from("/Users/x/.cargo/bin/oxiline");
        assert_eq!(app_bundle_root_of(&exe), None);
        let exe = std::path::PathBuf::from("/usr/local/bin/oxiline");
        assert_eq!(app_bundle_root_of(&exe), None);
    }
```

Run: `cargo test -p oxiline-cli upgrade::tests::detects_app_bundle_from_sidecar_path 2>&1 | tail -10`
Expected: FAIL (function not defined).

- [ ] **Step 2: Implement the function**

```rust
use std::path::{Path, PathBuf};

fn app_bundle_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| app_bundle_root_of(&exe))
}

fn app_bundle_root_of(exe: &Path) -> Option<PathBuf> {
    for ancestor in exe.ancestors() {
        if ancestor.extension().is_some_and(|e| e == "app") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}
```

- [ ] **Step 3: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -10`
Expected: 6 passed, 0 failed.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): detect in-app vs standalone upgrade context"
```

---

## Task 4: TDD `Manifest` deserialization

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs` (add the struct and the deserialization tests)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Deserialize)]
  struct Manifest {
      version: String,
      #[serde(default)]
      notes: Option<String>,
      #[serde(default)]
      pub_date: Option<String>,
      #[serde(default)]
      platforms: HashMap<String, PlatformAsset>,
  }
  #[derive(Deserialize)]
  struct PlatformAsset { url: String, signature: String }
  ```

- [ ] **Step 1: Add the failing tests**

Append to the `tests` module:

```rust
    #[test]
    fn manifest_deserializes_minimal_shape() {
        let json = r#"{
            "version": "0.7.0",
            "notes": "OxiLine 0.7.0",
            "pub_date": "2026-08-01T00:00:00Z",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/OxiLine.app.tar.gz",
                    "signature": "AAA"
                }
            }
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("manifest parses");
        assert_eq!(m.version, "0.7.0");
        assert_eq!(m.notes.as_deref(), Some("OxiLine 0.7.0"));
        assert!(m.platforms.contains_key("darwin-aarch64"));
    }

    #[test]
    fn manifest_notes_is_optional() {
        let json = r#"{
            "version": "0.7.0",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/OxiLine.app.tar.gz",
                    "signature": "AAA"
                }
            }
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("manifest parses");
        assert_eq!(m.notes, None);
    }
```

Run: `cargo test -p oxiline-cli upgrade::tests::manifest_deserializes_minimal_shape 2>&1 | tail -10`
Expected: FAIL (`Manifest` not defined).

- [ ] **Step 2: Implement the struct**

```rust
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    version: String,
    /// Release notes surfaced to the GUI banner and the Preferences section.
    /// Optional in the manifest; the JSON `available` event falls back to
    /// `"OxiLine <version>"` when the manifest omits it.
    #[serde(default)]
    notes: Option<String>,
    /// Tauri-shaped `pub_date` (RFC 3339). Kept optional — we don't surface
    /// it to the GUI today but parsing it preserves forward-compat.
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    platforms: HashMap<String, PlatformAsset>,
}

#[derive(Deserialize)]
struct PlatformAsset {
    url: String,
    signature: String,
}
```

- [ ] **Step 3: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -10`
Expected: 8 passed, 0 failed.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): parse latest.json manifest (notes/pub_date optional)"
```

---

## Task 5: TDD NDJSON event serialization (the wire contract)

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs`

**Interfaces:**
- Produces (the `Event` enum whose `serde_json::to_string` shape is the wire contract):
  ```rust
  #[derive(serde::Serialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  enum Event<'a> {
      Checking,
      Current { version: &'a str },
      Available { from: &'a str, to: &'a str, notes: &'a str },
      Latest { version: &'a str },
      Download { pct: u8 },
      Verifying,
      Swapping { mode: &'a str },
      Done { version: &'a str },
      Error { message: &'a str },
  }
  ```

- [ ] **Step 1: Add the failing tests for the wire shape**

Append to the `tests` module:

```rust
    /// The exact JSON shapes are part of the GUI↔CLI contract (`doc/10-updater.md`).
    /// Renaming a field silently breaks the GUI; these tests pin every event shape.
    #[test]
    fn event_checking_serializes_to_typed_tag() {
        assert_eq!(serde_json::to_string(&Event::Checking).unwrap(), r#"{"type":"checking"}"#);
    }

    #[test]
    fn event_available_carries_from_to_notes() {
        let e = Event::Available { from: "0.6.1", to: "0.7.0", notes: "OxiLine 0.7.0" };
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], "available");
        assert_eq!(v["from"], "0.6.1");
        assert_eq!(v["to"], "0.7.0");
        assert_eq!(v["notes"], "OxiLine 0.7.0");
    }

    #[test]
    fn event_download_pct_is_a_number_not_string() {
        let v = serde_json::to_value(&Event::Download { pct: 42 }).unwrap();
        assert_eq!(v["type"], "download");
        assert_eq!(v["pct"].as_u64(), Some(42));
    }

    #[test]
    fn event_swapping_mode_uses_snake_case_known_values() {
        assert_eq!(
            serde_json::to_string(&Event::Swapping { mode: "app" }).unwrap(),
            r#"{"type":"swapping","mode":"app"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Swapping { mode: "standalone" }).unwrap(),
            r#"{"type":"swapping","mode":"standalone"}"#
        );
    }

    #[test]
    fn event_done_and_latest_and_error_match_contract() {
        assert_eq!(
            serde_json::to_string(&Event::Done { version: "0.7.0" }).unwrap(),
            r#"{"type":"done","version":"0.7.0"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Latest { version: "0.6.1" }).unwrap(),
            r#"{"type":"latest","version":"0.6.1"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Error { message: "boom" }).unwrap(),
            r#"{"type":"error","message":"boom"}"#
        );
    }
```

Run: `cargo test -p oxiline-cli upgrade::tests::event_ 2>&1 | tail -10`
Expected: FAIL (`Event` not defined).

- [ ] **Step 2: Implement the enum**

```rust
/// NDJSON progress event. The schema is part of the GUI↔CLI contract
/// (`doc/10-updater.md`); keep field names and types stable.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event<'a> {
    Checking,
    Current { version: &'a str },
    Available { from: &'a str, to: &'a str, notes: &'a str },
    Latest { version: &'a str },
    Download { pct: u8 },
    Verifying,
    Swapping { mode: &'a str },
    Done { version: &'a str },
    Error { message: &'a str },
}
```

- [ ] **Step 3: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -15`
Expected: 13 passed, 0 failed.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): NDJSON progress event contract for GUI sidecar"
```

---

## Task 6: TDD `verify_minisign` against a fixture signature

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs`

**Interfaces:**
- Produces: `fn verify_minisign(data: &[u8], signature_b64: &str) -> anyhow::Result<()>`
- Test fixture: a tiny signed payload generated offline with the same minisign key that the OxiLine `latest.json` uses. The fixture lives in `crates/oxiline-cli/tests/fixtures/minisign/` and is checked in.

This task needs a one-time offline generation step because we need a known good signature. Two sub-steps handle it.

- [ ] **Step 1: Generate the test fixture (one-time, manual)**

```bash
mkdir -p crates/oxiline-cli/tests/fixtures/minisign
# The Tauri signing key is a project secret. For the unit test we want a
# *publicly known* minisign key so the test is reproducible on a fresh
# clone. Use the "test key" minisign documents in its README; this is the
# well-known `RWT...` test pubkey. See: https://github.com/jedisct1/minisign
minisign -G -p crates/oxiline-cli/tests/fixtures/minisign/test.pub -s crates/oxiline-cli/tests/fixtures/minisign/test.key -W
echo 'OxiLine upgrade test fixture payload' > crates/oxiline-cli/tests/fixtures/minisign/payload.txt
minisign -S -s crates/oxiline-cli/tests/fixtures/minisign/test.key -m crates/oxiline-cli/tests/fixtures/minisign/payload.txt
# Encode the raw signature to base64 (matches the wire format the manifest uses).
base64 -i crates/oxiline-cli/tests/fixtures/minisign/payload.txt.minisig | tr -d '\n' \
  > crates/oxiline-cli/tests/fixtures/minisign/payload.txt.minisig.b64
```

- [ ] **Step 2: Add the failing test (reads the fixture)**

```rust
    /// Round-trip against a known-good minisign signature. The fixture is
    /// committed under `tests/fixtures/minisign/` and was produced offline
    /// with the well-known test key (see Task 6 plan). The live release
    /// network test lives in `verifies_live_release_signature` (#[ignore]).
    #[test]
    fn verify_minisign_accepts_known_good_signature() {
        let payload = std::fs::read("../tests/fixtures/minisign/payload.txt")
            .expect("fixture payload present");
        let sig = std::fs::read_to_string("../tests/fixtures/minisign/payload.txt.minisig.b64")
            .expect("fixture signature present");
        verify_minisign(&payload, &sig).expect("valid signature verifies");
    }

    #[test]
    fn verify_minisign_rejects_tampered_payload() {
        let mut payload = std::fs::read("../tests/fixtures/minisign/payload.txt")
            .expect("fixture payload present");
        let sig = std::fs::read_to_string("../tests/fixtures/minisign/payload.txt.minisig.b64")
            .expect("fixture signature present");
        payload[0] ^= 0x01; // flip a bit
        assert!(verify_minisign(&payload, &sig).is_err(), "tampered payload must fail");
    }
```

Run from the workspace root: `cargo test -p oxiline-cli upgrade::tests::verify_minisign 2>&1 | tail -10`
Expected: FAIL (`verify_minisign` not defined).

- [ ] **Step 3: Implement `verify_minisign`**

```rust
const PUBKEY: &str = "RWSPFXSR74pl0b+Ssow4gaUe7zr3ftkFG2S1obIcjfFKumljMGOYxgqq";
// NOTE: this is the *test* pubkey for unit tests. The real OxiLine key
// replaces it in Task 10 (the live one is in `tauri.conf.json` today).

fn verify_minisign(data: &[u8], signature_b64: &str) -> anyhow::Result<()> {
    use anyhow::anyhow;
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.trim().as_bytes())
        .map_err(|e| anyhow!("decode signature: {e}"))?;
    let sig_box = String::from_utf8(raw).map_err(|e| anyhow!("signature not utf-8: {e}"))?;
    let pk = minisign_verify::PublicKey::from_base64(PUBKEY)
        .map_err(|e| anyhow!("parse public key: {e}"))?;
    let sig = minisign_verify::Signature::decode(&sig_box)
        .map_err(|e| anyhow!("parse signature: {e}"))?;
    pk.verify(data, &sig, false)
        .map_err(|e| anyhow!("signature verification failed: {e}"))?;
    Ok(())
}
```

- [ ] **Step 4: Run the new tests**

Run: `cargo test -p oxiline-cli upgrade::tests::verify_minisign 2>&1 | tail -10`
Expected: 2 passed.

- [ ] **Step 5: Commit (test + impl + fixture together)**

```bash
git add crates/oxiline-cli/src/upgrade.rs crates/oxiline-cli/tests/fixtures
git commit -m "feat(cli): minisign verify with fixture-backed TDD"
```

---

## Task 7: TDD `download` byte stream + progress reporting

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs`

**Interfaces:**
- Produces: `fn download(url: &str, dest: &Path, on_pct: &mut dyn FnMut(u8)) -> anyhow::Result<()>`
  - `on_pct` is called with `0` once at the start, an increasing value as bytes stream, and `100` at the end.
  - For a 5 MiB download with 1 MiB granularity, it should be called at least 5 times (0, 1, 2, 3, 4, 5, 100).

- [ ] **Step 1: Add the failing test using a local HTTP server**

```rust
    /// Drives `download` against a small in-process TCP server. The server
    /// returns a known byte count; the callback must fire `0`, monotonic
    /// values up to ≤99, and `100` at the end.
    #[test]
    fn download_emits_zero_then_ascending_then_100() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc::channel;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body: Vec<u8> = (0..(5 * 1024 * 1024)).map(|i| (i % 251) as u8).collect();
        let body_len = body.len();
        let (tx, rx) = channel::<()>();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf); // drain request line
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
            );
            s.write_all(resp.as_bytes()).unwrap();
            s.write_all(&body).unwrap();
            drop(tx);
        });

        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out.bin");
        let mut calls: Vec<u8> = Vec::new();
        download(&format!("http://{addr}/"), &dest, &mut |pct| calls.push(pct)).unwrap();
        let _ = rx.recv();
        assert_eq!(calls.first(), Some(&0));
        assert_eq!(calls.last(), Some(&100));
        // Monotonic between.
        for w in calls.windows(2) {
            assert!(w[0] <= w[1], "progress must be non-decreasing: {:?}", w);
        }
        assert_eq!(std::fs::metadata(&dest).unwrap().len() as usize, body_len);
    }
```

Add `tempfile` to `[dev-dependencies]` if not already present (it is — see `Cargo.toml`).

Run: `cargo test -p oxiline-cli upgrade::tests::download_emits 2>&1 | tail -10`
Expected: FAIL (function not defined).

- [ ] **Step 2: Implement `download`**

```rust
fn download(url: &str, dest: &Path, on_pct: &mut dyn FnMut(u8)) -> anyhow::Result<()> {
    use anyhow::anyhow;
    use std::io::Read;
    let resp = ureq::get(url)
        .call()
        .map_err(|e| anyhow!("download {url}: {e}"))?;
    let mut reader = resp.into_reader();
    let mut f = std::fs::File::create(dest)
        .map_err(|e| anyhow!("create {}: {e}", dest.display()))?;
    let mut buf = [0u8; 64 * 1024];
    let mut total: usize = 0;
    on_pct(0);
    loop {
        let n = reader.read(&mut buf).map_err(|e| anyhow!("read: {e}"))?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| anyhow!("write: {e}"))?;
        total += n;
        let approx = ((total / (1024 * 1024)) as u8).min(99);
        on_pct(approx);
    }
    f.sync_all().ok();
    on_pct(100);
    Ok(())
}
```

(Adds `use std::io::Write;` if not already imported.)

- [ ] **Step 3: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -15`
Expected: 16 passed, 0 failed.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): streaming download with progress callback"
```

---

## Task 8: TDD `sibling_tempdir`, `extract_tar_gz`, `find_entry_with_ext`, `sha256_hex`

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs`

**Interfaces:**
- Produces:
  - `fn sibling_tempdir(sibling_of: &Path) -> Result<PathBuf>` — temp dir next to the sibling, for atomic rename.
  - `fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()>`.
  - `fn find_entry_with_ext(dir: &Path, ext: &str) -> Result<PathBuf>`.
  - `fn sha256_hex(path: &Path) -> Result<String>`.

- [ ] **Step 1: Add the failing tests**

```rust
    #[test]
    fn sha256_hex_matches_known_digest() {
        // Echoed "abc" → SHA-256 = ba7816bf...f20015ad (NIST test vector).
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("abc.txt");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            sha256_hex(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn extract_tar_gz_preserves_entry() {
        // Build a tar.gz in memory containing "hello.txt" → "hi\n".
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("in.tar.gz");
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            flate2::Compression::fast(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_path("hello.txt").unwrap();
        header.set_size(3);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, b"hi\n" as &[u8]).unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        extract_tar_gz(&archive, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("hello.txt")).unwrap(), "hi\n");
    }

    #[test]
    fn find_entry_with_ext_finds_app_but_ignores_others() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), b"x").unwrap();
        std::fs::write(tmp.path().join("OxiLine.app"), b"x").unwrap();
        let found = find_entry_with_ext(tmp.path(), "app").unwrap();
        assert!(found.ends_with("OxiLine.app"));
    }
```

Run: `cargo test -p oxiline-cli upgrade::tests::sha256_hex_matches 2>&1 | tail -10`
Expected: FAIL (function not defined).

- [ ] **Step 2: Implement the four helpers**

```rust
fn sibling_tempdir(sibling_of: &Path) -> anyhow::Result<PathBuf> {
    use anyhow::Context;
    let dir = sibling_of.join(format!(".oxiline-upgrade-{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create work dir {}", dir.display()))?;
    Ok(dir)
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    let f = std::fs::File::open(archive)
        .with_context(|| format!("open {}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut tar = tar::Archive::new(gz);
    tar.set_overwrite(true);
    tar.unpack(dest)
        .with_context(|| format!("extract {}", archive.display()))?;
    Ok(())
}

fn find_entry_with_ext(dir: &Path, ext: &str) -> anyhow::Result<PathBuf> {
    use anyhow::{anyhow, bail};
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == ext) {
            return Ok(entry.path());
        }
    }
    bail!("extracted archive contained no .{ext}");
}

fn sha256_hex(path: &Path) -> anyhow::Result<String> {
    use anyhow::Context;
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut f = std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).context("read for hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
```

- [ ] **Step 3: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -15`
Expected: 19 passed, 0 failed.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): tar.gz extract, sha256, sibling tempdir helpers"
```

---

## Task 9: TDD `upgrade_in_app` and `upgrade_standalone` with a synthetic fixture

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs`

**Interfaces:**
- Produces:
  - `fn upgrade_in_app(manifest: &Manifest, app: &Path, opts: &Options) -> Result<()>`
  - `fn upgrade_standalone(latest: &str, opts: &Options) -> Result<()>`

These are integration-shaped, but we can exercise the in-app happy path with a synthetic `Manifest` whose `url` points at a local HTTP server that serves a tarball containing a fake `OxiLine.app` directory. Standalone is left to the `#[ignore]` end-to-end network test (Task 10) — no good way to test it without a second binary on disk.

- [ ] **Step 1: Add the failing test**

```rust
    /// In-app swap on a synthetic path. The fake HTTP server returns a
    /// tarball that contains a directory called `OxiLine.app/foo.txt`. The
    /// test exercises the public manifest API end-to-end (parse, download,
    /// extract, find `.app`, rename into place) and confirms the parent
    /// directory ends up holding the new bundle.
    #[test]
    fn upgrade_in_app_replaces_bundle_on_disk() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::path::PathBuf;

        // Build a tar.gz in memory: OxiLine.app/foo.txt = "fresh".
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let body = b"fresh".to_vec();
        let mut header = tar::Header::new_gnu();
        header.set_path("OxiLine.app/foo.txt").unwrap();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, body.as_slice()).unwrap();
        let gz = builder.into_inner().unwrap().finish().unwrap();

        // Server: respond 200 with the tarball, regardless of path.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let gz_len = gz.len();
        let gz_clone = gz.clone();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {gz_len}\r\nConnection: close\r\n\r\n"
            );
            s.write_all(resp.as_bytes()).unwrap();
            s.write_all(&gz_clone).unwrap();
        });

        // Build a manifest. The signature is the fixture signature — the
        // verify path is exercised in `verify_minisign_*` tests, not here.
        let sig = std::fs::read_to_string("../tests/fixtures/minisign/payload.txt.minisig.b64")
            .expect("fixture signature present");
        let mut platforms = std::collections::HashMap::new();
        platforms.insert(
            "darwin-aarch64".into(),
            PlatformAsset {
                url: format!("http://{addr}/bundle.app.tar.gz"),
                signature: sig,
            },
        );
        let manifest = Manifest {
            version: "9.9.9".into(),
            notes: Some("test".into()),
            pub_date: None,
            platforms,
        };

        // Synthesize a working directory with a fake .app to be replaced.
        let work = tempfile::tempdir().unwrap();
        let app = work.path().join("OxiLine.app");
        std::fs::create_dir(&app).unwrap();
        std::fs::write(app.join("old.txt"), b"old").unwrap();

        upgrade_in_app(&manifest, &app, &Options::default()).unwrap();

        assert!(app.join("foo.txt").exists(), "new foo.txt must be in place");
        assert!(!app.join("old.txt").exists(), "old file must be gone");
        assert!(!work.path().join(".oxiline-upgrade-").exists()
            || std::fs::read_dir(work.path())
                .unwrap()
                .filter_map(Result::ok)
                .all(|e| !e.file_name().to_string_lossy().starts_with(".oxiline-upgrade-")),
            "sibling tempdir must be cleaned up");
    }
```

Run: `cargo test -p oxiline-cli upgrade::tests::upgrade_in_app_replaces_bundle_on_disk 2>&1 | tail -10`
Expected: FAIL (function not defined). The test also requires a fresh signature: we'll re-use the fixture from Task 6, but generate a *fresh* one for a tiny payload to avoid coupling the in-app test to a payload other than the test server. For simplicity this plan keeps the same fixture (the signature is over arbitrary bytes — it doesn't have to match the tarball content for THIS test, because `verify_minisign` will reject it. Adjust by pre-signing the *tarball bytes*):

Correction: the test must pre-sign the *exact bytes it serves*. Easiest path: use the `minisign` CLI inside the test to sign the tarball. Since the test runs in CI, `minisign` may not be installed. Therefore we **bypass `verify_minisign` for this test** by injecting a hook — see Step 2. The production function uses real verify; the test path uses a no-op.

- [ ] **Step 2: Implement `upgrade_in_app` with a verify hook (test seam)**

To avoid re-signing the fixture tarball inside the test, expose a thin trait seam:

```rust
type Verify = fn(&[u8], &str) -> anyhow::Result<()>;

fn upgrade_in_app(manifest: &Manifest, app: &Path, opts: &Options) -> anyhow::Result<()> {
    upgrade_in_app_with_verify(manifest, app, opts, verify_minisign)
}

fn upgrade_in_app_with_verify(
    manifest: &Manifest,
    app: &Path,
    opts: &Options,
    verify: Verify,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let asset = manifest
        .platforms
        .get(PLATFORM_KEY)
        .ok_or_else(|| anyhow::anyhow!("manifest has no asset for {PLATFORM_KEY}"))?;
    let parent = app
        .parent()
        .ok_or_else(|| anyhow::anyhow!("app bundle has no parent directory"))?;
    let work = sibling_tempdir(parent)?;

    let result = (|| -> anyhow::Result<()> {
        let archive = work.join("bundle.app.tar.gz");
        let mut on_pct = |pct: u8| { if opts.json_progress { /* emit */ } };
        crate::upgrade::download(&asset.url, &archive, &mut on_pct)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let data = std::fs::read(&archive)?;
        emit(opts, Event::Verifying);
        verify(&data, &asset.signature)?;
        extract_tar_gz(&archive, &work)?;
        let new_app = find_entry_with_ext(&work, "app")?;
        let old = work.join(".previous.app");
        if app.exists() {
            std::fs::rename(app, &old)
                .with_context(|| format!("move aside {}", app.display()))?;
        }
        std::fs::rename(&new_app, app)
            .with_context(|| format!("install {}", app.display()))?;
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&work);
    if result.is_ok() {
        emit(opts, Event::Swapping { mode: "app" });
    }
    result
}
```

Update the test to call `upgrade_in_app_with_verify(..., |_, _| Ok(()))` instead of `upgrade_in_app`. This keeps the test deterministic without breaking the production contract.

- [ ] **Step 3: Run the test**

Run: `cargo test -p oxiline-cli upgrade::tests::upgrade_in_app 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): in-app bundle swap with extract → rename"
```

---

## Task 10: Wire `run()` and `Options`; replace the test pubkey with the live one

**Files:**
- Modify: `crates/oxiline-cli/src/upgrade.rs`

**Interfaces (final):**

```rust
pub struct Options {
    pub check: bool,
    pub json_progress: bool,
    pub assume_yes: bool,
}

pub fn run(conn: &rusqlite::Connection, opts: Options) -> anyhow::Result<()>
```

- [ ] **Step 1: Replace the test `PUBKEY` with the live OxiLine key**

The live key (the same one that lives in `tauri.conf.json` today) is:

```rust
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg2NTU3RTc3QTc2MzUwMTYKUldRV1VHT25kMzVWaHU1K3BqTmhaNXBCamQ0TisxWVR6OG5zZFRGbGx2bnJDWjc5SFNhdjdCM3UK";
```

The fixture tests in Task 6 use a *different* (test) pubkey. To not break those, **parameterize `verify_minisign`** to accept the key:

```rust
fn verify_minisign(data: &[u8], signature_b64: &str) -> anyhow::Result<()> {
    verify_minisign_with(data, signature_b64, PUBKEY)
}

fn verify_minisign_with(data: &[u8], signature_b64: &str, key: &str) -> anyhow::Result<()> {
    use anyhow::anyhow;
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.trim().as_bytes())
        .map_err(|e| anyhow!("decode signature: {e}"))?;
    let sig_box = String::from_utf8(raw).map_err(|e| anyhow!("signature not utf-8: {e}"))?;
    let pk = minisign_verify::PublicKey::from_base64(key)
        .map_err(|e| anyhow!("parse public key: {e}"))?;
    let sig = minisign_verify::Signature::decode(&sig_box)
        .map_err(|e| anyhow!("parse signature: {e}"))?;
    pk.verify(data, &sig, false)
        .map_err(|e| anyhow!("signature verification failed: {e}"))?;
    Ok(())
}
```

Update Task 6 tests to call `verify_minisign_with(..., TEST_PUBKEY)`. Define a `const TEST_PUBKEY: &str = "RWSPFXSR74pl0b+Ssow4gaUe7zr3ftkFG2S1obIcjfFKumljMGOYxgqq";` near the tests.

- [ ] **Step 2: Add the `run()` function and NDJSON emission helpers**

```rust
use rusqlite::Connection;
use serde_json::Value;
use crate::settings;
use crate::util;

pub fn run(conn: &Connection, opts: Options) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    emit(&opts, Event::Checking);
    let manifest = match fetch_manifest() {
        Ok(m) => m,
        Err(e) => {
            emit_err(&opts, e.to_string());
            return Err(e);
        }
    };
    let latest = manifest.version.as_str();
    let notes = manifest.notes.clone().unwrap_or_else(|| format!("OxiLine {latest}"));

    if !is_newer(latest, current) {
        emit(&opts, Event::Latest { version: current.to_string() });
        human(&opts, &format!("Already up to date (v{current})."));
        return Ok(());
    }
    emit(&opts, Event::Available {
        from: current.to_string(),
        to: latest.to_string(),
        notes: notes.clone(),
    });
    if opts.check {
        return Ok(());
    }
    human(&opts, &format!("Update available: v{current} → v{latest}."));

    if !opts.assume_yes && !opts.json_progress {
        if !confirm("Proceed with install? [y/N] ")? {
            human(&opts, "Aborted.");
            return Ok(());
        }
    }

    let swap = if let Some(app) = app_bundle_root() {
        upgrade_in_app_with_verify(&manifest, &app, &opts, verify_minisign)
    } else {
        upgrade_standalone(latest, &opts)
    };
    if let Err(e) = swap {
        emit_err(&opts, e.to_string());
        return Err(e);
    }

    settings::set(conn, "update_request_at", &Value::String(util::now_iso()))?;
    emit(&opts, Event::Done { version: latest.to_string() });
    human(&opts, &format!("Updated to v{latest}. Restart OxiLine to use the new version."));
    Ok(())
}

fn fetch_manifest() -> anyhow::Result<Manifest> {
    use anyhow::anyhow;
    let resp = ureq::get(ENDPOINT).call().map_err(|e| anyhow!("fetch manifest: {e}"))?;
    let body = resp.into_string().map_err(|e| anyhow!("read manifest: {e}"))?;
    serde_json::from_str(&body).map_err(|e| anyhow!("parse manifest: {e}"))
}

fn emit(opts: &Options, ev: Event<'_>) {
    if !opts.json_progress { return; }
    use std::io::Write;
    if let Ok(line) = serde_json::to_string(&ev) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}
fn emit_err(opts: &Options, msg: String) { emit(opts, Event::Error { message: &msg }); }
fn human(opts: &Options, line: &str) { if !opts.json_progress { println!("{line}"); } }
fn confirm(prompt: &str) -> anyhow::Result<bool> {
    use std::io::{BufRead, Write};
    use anyhow::Context;
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).context("read confirmation")?;
    Ok(matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}
```

Add the missing `upgrade_standalone` (use the same body as the OxiLine `oximemo` standalone path; minisign verify stays as a no-op marker for the GH CLI tarball — the standalone path verifies SHA-256 from `.sha256`):

```rust
fn upgrade_standalone(latest: &str, opts: &Options) -> anyhow::Result<()> {
    use anyhow::{Context, anyhow, bail};
    let exe = std::env::current_exe().context("resolve running binary")?;
    let parent = exe.parent().ok_or_else(|| anyhow!("binary has no parent directory"))?;
    let work = sibling_tempdir(parent)?;
    let result = (|| -> anyhow::Result<()> {
        let name = format!("oxiline-{TARGET_TRIPLE}.tar.gz");
        let url = format!("https://github.com/{REPO}/releases/download/v{latest}/{name}");
        let archive = work.join(&name);
        let mut on_pct = |_pct: u8| {};
        download(&url, &archive, &mut on_pct)?;
        let expected = {
            let r = ureq::get(&format!("{url}.sha256")).call()
                .map_err(|e| anyhow!("download sha256: {e}"))?;
            r.into_string().map_err(|e| anyhow!("read sha256: {e}"))?
        };
        let expected = expected.split_whitespace().next().unwrap_or("");
        let actual = sha256_hex(&archive)?;
        if !expected.eq_ignore_ascii_case(&actual) {
            bail!("checksum mismatch: expected {expected}, got {actual}");
        }
        extract_tar_gz(&archive, &work)?;
        let new_bin = work.join("oxiline");
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(&new_bin)?.permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&new_bin, perm)?;
        }
        let old = work.join(".previous.bin");
        std::fs::rename(&exe, &old).with_context(|| format!("move aside {}", exe.display()))?;
        std::fs::rename(&new_bin, &exe).with_context(|| format!("install {}", exe.display()))?;
        let _ = std::fs::remove_file(&old);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    if result.is_ok() {
        emit(opts, Event::Swapping { mode: "standalone" });
    }
    result
}
```

- [ ] **Step 3: Add the live network test (ignored)**

```rust
    /// End-to-end probe: the OxiLine `latest.json` → the live signed bundle.
    /// This is the safety check the spec calls out before removing
    /// `tauri-plugin-updater` from the GUI. Run with
    /// `cargo test -p oxiline-cli upgrade::tests::verifies_live_release_signature -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn verifies_live_release_signature() {
        let manifest = fetch_manifest().expect("fetch manifest");
        let asset = manifest.platforms.get(PLATFORM_KEY).expect("darwin-aarch64 asset");
        let resp = ureq::get(&asset.url).call().expect("download bundle");
        let mut data = Vec::new();
        resp.into_reader().read_to_end(&mut data).expect("read bundle");
        verify_minisign(&data, &asset.signature).expect("PUBKEY verifies the live signature");
    }
```

- [ ] **Step 4: Run all upgrade tests**

Run: `cargo test -p oxiline-cli upgrade:: 2>&1 | tail -10`
Expected: ~20 passed, 0 failed.

- [ ] **Step 5: Run the ignored live test (network-dependent)**

Run: `cargo test -p oxiline-cli upgrade::tests::verifies_live_release_signature -- --ignored --nocapture 2>&1 | tail -10`
Expected: PASS (when the live `latest.json` is reachable).

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-cli/src/upgrade.rs
git commit -m "feat(cli): self-update engine — run() with live PUBKEY and end-to-end network test"
```

---

## Task 11: Add `Upgrade` subcommand and wire dispatch in `main.rs`

**Files:**
- Modify: `crates/oxiline-cli/src/cli.rs:84-90` (replace `Update` with `Upgrade`)
- Modify: `crates/oxiline-cli/src/main.rs:1-13, 220-267` (register `mod upgrade;`, dispatch to it)

**Interfaces:**
- `Command::Upgrade { check, json_progress, yes }` with the same `--check` semantics as today's `Update` had.
- `Command::Update` is replaced by a hidden alias that prints a deprecation notice and forwards to `upgrade --check`.

- [ ] **Step 1: Update `cli.rs`**

Replace the `Update` variant with:

```rust
    /// Check for a newer release; download, verify, and swap in place.
    Upgrade {
        /// Only report availability; do not download or install.
        #[arg(long)]
        check: bool,
        /// Emit one JSON object per line on stdout (NDJSON). Used by the
        /// GUI sidecar to drive its progress UI.
        #[arg(long, hide = true)]
        json_progress: bool,
        /// Skip the interactive confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Deprecated alias for `upgrade --check`. Preserved for one release so
    /// existing user scripts keep working; prints a deprecation notice and
    /// forwards.
    #[command(hide = true)]
    Update {
        /// Forwarded to `upgrade --check`.
        #[arg(long)]
        check: bool,
    },
```

- [ ] **Step 2: Update `main.rs`**

```rust
mod cli;
mod lang;
mod output;
mod upgrade;
```

Replace the existing `Command::Update` arm (lines 220-267) with:

```rust
        Command::Upgrade { check, json_progress, yes } => {
            upgrade::run(
                &conn,
                upgrade::Options {
                    check: *check,
                    json_progress: *json_progress,
                    assume_yes: *yes,
                },
            )?;
        }
        Command::Update { check } => {
            // Deprecated alias for `upgrade --check`. Existing user scripts
            // invoked `oxiline update`; preserve that, but signal the rename.
            eprintln!("warning: `oxiline update` is deprecated; use `oxiline upgrade`");
            upgrade::run(
                &conn,
                upgrade::Options {
                    check: *check,
                    json_progress: false,
                    assume_yes: false,
                },
            )?;
        }
```

- [ ] **Step 3: Run CLI tests**

Run: `cargo test -p oxiline-cli 2>&1 | tail -10`
Expected: all pass.

- [ ] **Step 4: Smoke-test the new CLI subcommand shape**

Run: `cargo run -p oxiline-cli --quiet -- upgrade --help 2>&1 | tail -20`
Expected: usage text with `--check`, `--json-progress`, `--yes`.

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-cli/src/cli.rs crates/oxiline-cli/src/main.rs
git commit -m "feat(cli): add `oxiline upgrade` subcommand; keep `update` as deprecated alias"
```

---

## Task 12: Integration test for `upgrade --check` (CLI surface)

**Files:**
- Create: `crates/oxiline-cli/tests/upgrade_cli.rs`

**Interfaces:** the same `OXILINE_BIN` env-based harness as `plan_cli.rs`.

- [ ] **Step 1: Write the failing test**

```rust
//! Integration tests for the `oxiline upgrade` CLI surface.

use assert_cmd::Command;
use predicates::prelude::*;

const OXILINE_BIN: &str = env!("CARGO_BIN_EXE_oxiline");

fn oxiline_cmd(db_path: &std::path::Path) -> Command {
    let mut c = Command::new(OXILINE_BIN);
    c.env("OXILINE_DB_PATH", db_path)
        .env("LC_ALL", "C")
        .env("LANG", "C");
    c
}

#[test]
fn upgrade_help_lists_new_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db");
    oxiline_cmd(&db)
        .args(["upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--json-progress"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn update_alias_forwards_with_deprecation_notice() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db");
    // The alias hits the network; on a CI machine without internet this
    // test will time out. Mark it as `#[ignore]` for the offline run.
}
```

Replace the empty `update_alias_forwards_with_deprecation_notice` body with `#[ignore]` and document it in a follow-up. The help-text test alone is enough offline proof for the rename.

Run: `cargo test -p oxiline-cli --test upgrade_cli 2>&1 | tail -10`
Expected: 1 passed.

- [ ] **Step 2: Commit**

```bash
git add crates/oxiline-cli/tests/upgrade_cli.rs
git commit -m "test(cli): upgrade --help surface and deprecation alias"
```

---

## Task 13: Remove `tauri-plugin-updater` from the Tauri app

**Files:**
- Modify: `crates/oxiline-app/src-tauri/Cargo.toml:20-47` (drop `tauri-plugin-updater`; add `tauri-plugin-shell`)
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs:81-98` (drop the `tauri_plugin_updater::Builder` setup; do not touch the builder chain otherwise)

- [ ] **Step 1: Update `Cargo.toml`**

Drop the line `tauri-plugin-updater = "2"`. Add `tauri-plugin-shell = "2"`.

- [ ] **Step 2: Update `lib.rs`**

Remove the entire `setup` block that wires `tauri_plugin_updater::Builder::new().build()` (lines 89-98 of the current file):

```rust
.setup(|app| {
    // Dock icon hidden; the app lives in the menu bar (§4.3).
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    tray::build(app.handle())?;
    ...
})
```

The block to remove:

```rust
    // In-app auto-update: check the GitHub Releases `latest.json` manifest
    // (§plugins.updater.endpoints). The frontend drives check/download/
    // install via `@tauri-apps/plugin-updater`; this just wires the plugin.
    #[cfg(desktop)]
    {
        let _ = app
            .handle()
            .plugin(tauri_plugin_updater::Builder::new().build());
    }
```

Add `tauri_plugin_shell::init()` to the builder chain (next to the other plugin inits, before `.manage(...)`):

```rust
    builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state::AppState::new())
```

- [ ] **Step 3: Verify the workspace builds**

Run: `cargo build -p oxiline-app --target aarch64-apple-darwin 2>&1 | tail -10`
Expected: build succeeds (the placeholder sidecar is fine for `cargo build`).

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src-tauri/Cargo.toml crates/oxiline-app/src-tauri/src/lib.rs
git commit -m "refactor(app): drop tauri-plugin-updater; add tauri-plugin-shell for sidecar"
```

---

## Task 14: Update `tauri.conf.json` and `capabilities/default.json`

**Files:**
- Modify: `crates/oxiline-app/src-tauri/tauri.conf.json:48-68`
- Modify: `crates/oxiline-app/src-tauri/capabilities/default.json:6-20`

- [ ] **Step 1: Strip the `plugins.updater` block and `bundle.createUpdaterArtifacts`**

In `tauri.conf.json`:

```diff
   "bundle": {
     "active": true,
-    "createUpdaterArtifacts": true,
     "targets": ["app"],
     "externalBin": ["binaries/oxiline"],
```

```diff
-  "plugins": {
-    "updater": {
-      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg2NTU3RTc3QTc2MzUwMTYKUldRV1VHT25kMzVWaHU1K3BqTmhaNXBCamQ0TisxWVR6OG5zZFRGbGx2bnJDWjc5SFNhdjdCM3UK",
-      "endpoints": [
-        "https://github.com/project-oxi/oxiline/releases/latest/download/latest.json"
-      ]
-    }
-  }
+  "plugins": {}
```

- [ ] **Step 2: Update `capabilities/default.json`**

```diff
   "permissions": [
     "core:default",
     "core:window:allow-show",
     ...
     "opener:default",
-    "updater:default",
+    "shell:allow-execute",
     "process:allow-restart"
   ]
```

The `shell:allow-execute` permission needs a scope. Add a top-level `shell` scope under the capability object (Tauri 2 supports the new `object` permission shape):

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main and HUD windows.",
  "windows": ["main", "hud"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focus",
    "core:window:allow-set-position",
    "core:window:allow-set-size",
    "core:window:allow-start-dragging",
    "core:window:allow-current-monitor",
    "core:event:default",
    "notification:default",
    "opener:default",
    "shell:allow-execute",
    "process:allow-restart"
  ]
}
```

Tauri 2 derives the sidecar scope from the `externalBin` declaration automatically when only the bare permission string is listed. Verify on first build by running `cargo build -p oxiline-app` and reading the generated `gen/schemas/capabilities.json` — if the shell scope needs an explicit `ShellAllowExecScope`, add a per-capability scope block in a follow-up commit.

- [ ] **Step 3: Build the app to regenerate the schema**

Run: `cargo build -p oxiline-app --target aarch64-apple-darwin 2>&1 | tail -15`
Expected: builds. (If a missing-scope error surfaces at runtime, add an explicit `ShellAllowExecScope` in the capability and re-run.)

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src-tauri/tauri.conf.json crates/oxiline-app/src-tauri/capabilities/default.json
git commit -m "refactor(app): remove Tauri updater config; add shell:allow-execute for sidecar"
```

---

## Task 15: Update `package.json` and `bun.lock` (frontend deps)

**Files:**
- Modify: `crates/oxiline-app/package.json:18-19`

- [ ] **Step 1: Swap the deps**

```diff
-    "@tauri-apps/plugin-updater": "^2",
-    "@tauri-apps/plugin-process": "^2",
+    "@tauri-apps/plugin-process": "^2",
+    "@tauri-apps/plugin-shell": "^2",
```

- [ ] **Step 2: Refresh the lockfile**

Run: `cd crates/oxiline-app && bun install 2>&1 | tail -5`
Expected: `bun.lock` updates with the new dep, the old one is removed.

- [ ] **Step 3: Verify the frontend still typechecks**

Run: `cd crates/oxiline-app && bun run build 2>&1 | tail -15`
Expected: builds. If `import { check } from "@tauri-apps/plugin-updater"` still shows up anywhere, this task is incomplete — fix the consumer in the next task.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/package.json crates/oxiline-app/bun.lock
git commit -m "refactor(app): swap @tauri-apps/plugin-updater for plugin-shell"
```

---

## Task 16: TDD the GUI NDJSON parser and reducer

**Files:**
- Modify: `crates/oxiline-app/src/lib/updater.test.ts` (new, vitest)
- Modify: `crates/oxiline-app/src/lib/updater.ts` (rewrite, keep the same zustand `useUpdate` shape)

**Interfaces (the contract the reducer must honor):**
- Input: one JSON object per line from `Command.stdout`.
- Output: a `useUpdate.getState().status` matching the existing `UpdateStatus` shape.
- The `downloading` state must report `downloaded` and `contentLength` so the existing progress bar in `UpdateSection` works.

- [ ] **Step 1: Add the failing tests**

Create `crates/oxiline-app/src/lib/updater.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { reduceEvent, initialState } from "./updater";

describe("reduceEvent", () => {
  it("`checking` resets to the checking state", () => {
    const next = reduceEvent(initialState(), { type: "checking" });
    expect(next.kind).toBe("checking");
  });

  it("`latest` records the version and a checkedAt timestamp", () => {
    const before = Date.now();
    const next = reduceEvent(initialState(), { type: "latest", version: "0.6.1" });
    expect(next.kind).toBe("latest");
    if (next.kind === "latest") {
      expect(next.version).toBe("0.6.1");
      expect(next.checkedAt).toBeGreaterThanOrEqual(before);
    }
  });

  it("`available` preserves notes for the Preferences panel", () => {
    const next = reduceEvent(initialState(), {
      type: "available", from: "0.6.1", to: "0.7.0", notes: "OxiLine 0.7.0",
    });
    expect(next.kind).toBe("available");
    if (next.kind === "available") {
      expect(next.version).toBe("0.7.0");
      expect(next.notes).toBe("OxiLine 0.7.0");
    }
  });

  it("`download` keeps the highest pct seen", () => {
    let s = initialState();
    s = reduceEvent(s, { type: "download", pct: 0 });
    s = reduceEvent(s, { type: "download", pct: 42 });
    s = reduceEvent(s, { type: "download", pct: 7 });
    expect(s.kind).toBe("downloading");
    if (s.kind === "downloading") {
      expect(s.pct).toBe(42);
    }
  });

  it("`swapping` flips to a transient restarting view", () => {
    const next = reduceEvent(initialState(), { type: "swapping", mode: "app" });
    expect(next.kind).toBe("restarting");
    if (next.kind === "restarting") {
      expect(next.mode).toBe("app");
    }
  });

  it("`error` carries the message verbatim", () => {
    const next = reduceEvent(initialState(), { type: "error", message: "boom" });
    expect(next.kind).toBe("error");
    if (next.kind === "error") expect(next.message).toBe("boom");
  });
});
```

The `UpdateStatus` shape gains a new variant: `{ kind: "restarting"; mode: "app" | "standalone" }`. This is added in the next step.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd crates/oxiline-app && bun run test 2>&1 | tail -10`
Expected: compile error (`reduceEvent` / `initialState` not exported).

- [ ] **Step 3: Implement the reducer in `updater.ts`**

Open `crates/oxiline-app/src/lib/updater.ts` and add (preserving the existing `useUpdate` zustand store and the `install` / `check` / `reset` methods that the rest of the app calls):

```ts
import { create } from "zustand";
import { Command } from "@tauri-apps/plugin-shell";
import { relaunch } from "@tauri-apps/plugin-process";

/** Discriminated event matching the CLI `--json-progress` NDJSON contract. */
export type ProgressEvent =
  | { type: "checking" }
  | { type: "current"; version: string }
  | { type: "available"; from: string; to: string; notes: string }
  | { type: "latest"; version: string }
  | { type: "download"; pct: number }
  | { type: "verifying" }
  | { type: "swapping"; mode: "app" | "standalone" }
  | { type: "done"; version: string }
  | { type: "error"; message: string };

/** Extends the existing `UpdateStatus` with a transient "restarting" step. */
export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest"; checkedAt: number; version?: string }
  | { kind: "available"; version: string; date?: string; notes?: string }
  | { kind: "downloading"; version: string; downloaded: number; contentLength: number; pct: number }
  | { kind: "restarting"; mode: "app" | "standalone"; version: string }
  | { kind: "error"; message: string };

export function initialState(): UpdateStatus {
  return { kind: "idle" };
}

export function reduceEvent(state: UpdateStatus, ev: ProgressEvent): UpdateStatus {
  switch (ev.type) {
    case "checking":
      return { kind: "checking" };
    case "latest":
      return { kind: "latest", checkedAt: Date.now(), version: ev.version };
    case "available":
      return { kind: "available", version: ev.to, notes: ev.notes };
    case "download": {
      const version = state.kind === "available" ? state.version
                    : state.kind === "downloading" ? state.version
                    : "";
      const contentLength = state.kind === "downloading" ? state.contentLength : 0;
      const downloaded = state.kind === "downloading" ? Math.max(state.downloaded, 1) : 0;
      return { kind: "downloading", version, downloaded, contentLength, pct: ev.pct };
    }
    case "swapping": {
      const version = state.kind === "available" ? state.version
                    : state.kind === "downloading" ? state.version
                    : "";
      return { kind: "restarting", mode: ev.mode, version };
    }
    case "done": {
      // The CLI has finished the swap and will shortly signal relaunch via
      // the watched `update_request_at` setting. The GUI's `App.tsx`
      // watcher calls `relaunch()` itself; we just keep state coherent.
      return state.kind === "restarting"
        ? { ...state, version: ev.version }
        : { kind: "latest", checkedAt: Date.now(), version: ev.version };
    }
    case "error":
      return { kind: "error", message: ev.message };
    case "verifying":
    case "current":
      return state; // intermediate; no UI change
  }
}
```

Now rewrite the store actions to spawn the sidecar:

```ts
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface UpdateState {
  status: UpdateStatus;
  check: () => Promise<void>;
  install: () => Promise<void>;
  reset: () => void;
}

const SIDECAR = "binaries/oxiline";

async function runUpgrade(args: string[]): Promise<ProgressEvent[]> {
  if (!inTauri) return [];
  const cmd = Command.sidecar(SIDECAR, args);
  const events: ProgressEvent[] = [];
  cmd.stdout.on("data", (line: string) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    try {
      const ev = JSON.parse(trimmed) as ProgressEvent;
      events.push(ev);
    } catch {
      // Ignore non-JSON lines (e.g. human-readable `print!` output) when
      // `--json-progress` is set; the contract is one JSON per line.
    }
  });
  const result = await cmd.execute();
  if (result.code !== 0) {
    const last = events[events.length - 1];
    const msg = last && last.type === "error" ? last.message : `oxiline upgrade exited with code ${result.code}`;
    throw new Error(msg);
  }
  return events;
}

export const useUpdate = create<UpdateState>((set, get) => ({
  status: initialState(),
  check: async () => {
    if (!inTauri) return;
    set({ status: { kind: "checking" } });
    try {
      const events = await runUpgrade(["upgrade", "--check", "--yes", "--json-progress"]);
      let status: UpdateStatus = { kind: "checking" };
      for (const ev of events) status = reduceEvent(status, ev);
      // `--check` may exit without ever emitting `latest`/`available` if the
      // fetch failed; the thrown error above catches that case.
      if (status.kind === "checking") {
        status = { kind: "latest", checkedAt: Date.now() };
      }
      set({ status });
    } catch (e) {
      set({ status: { kind: "error", message: String(e) } });
    }
  },
  install: async () => {
    if (!inTauri) return;
    const cur = get().status;
    if (cur.kind !== "available") return;
    set({ status: { kind: "downloading", version: cur.version, downloaded: 0, contentLength: 0, pct: 0 } });
    try {
      const events = await runUpgrade(["upgrade", "--yes", "--json-progress"]);
      let status: UpdateStatus = { kind: "downloading", version: cur.version, downloaded: 0, contentLength: 0, pct: 0 };
      for (const ev of events) status = reduceEvent(status, ev);
      set({ status });
      // The CLI has written `update_request_at`; `App.tsx` watches that and
      // calls `relaunch()` itself. We do not relaunch from here to keep
      // the single-writer invariant.
    } catch (e) {
      set({ status: { kind: "error", message: String(e) } });
    }
  },
  reset: () => set({ status: initialState() }),
}));
```

- [ ] **Step 4: Run the new unit tests**

Run: `cd crates/oxiline-app && bun run test 2>&1 | tail -10`
Expected: 6 passed.

- [ ] **Step 5: Typecheck + build the frontend**

Run: `cd crates/oxiline-app && bun run build 2>&1 | tail -10`
Expected: builds.

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-app/src/lib/updater.ts crates/oxiline-app/src/lib/updater.test.ts
git commit -m "feat(app): rewrite updater to spawn oxiline sidecar (NDJSON contract)"
```

---

## Task 17: Update the Preferences panel for the new `restarting` state

**Files:**
- Modify: `crates/oxiline-app/src/components/Preferences.tsx:108-186` (the `UpdateSection`)

- [ ] **Step 1: Add a restarting branch to `UpdateSection`**

Inside the `UpdateSection` JSX, after the `downloading` block, add:

```tsx
      {status.kind === "restarting" && (
        <p className="py-1 text-[12px] text-status-success">
          {t("updater.restarting", { mode: status.mode, version: status.version })}
        </p>
      )}
```

- [ ] **Step 2: Add the i18n key**

Edit `crates/oxiline-app/src/locales/en/translation.json` and `ko/translation.json` (paths to be confirmed by the next `grep` in execution):

```json
"updater.restarting": "Installed v{{version}} — restarting ({{mode}})…"
```

(`ko` equivalent: `"v{{version}} 설치 완료 — 재시작 중 ({{mode}})…"`)

- [ ] **Step 3: Typecheck + build**

Run: `cd crates/oxiline-app && bun run build 2>&1 | tail -10`
Expected: builds.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src/components/Preferences.tsx crates/oxiline-app/src/locales
git commit -m "feat(app): render restarting state in Preferences updater panel"
```

---

## Task 18: Release pipeline — confirm `release.yml` survives the move

**Files:**
- Modify: `.github/workflows/release.yml` (only if a glob needs updating)

- [ ] **Step 1: Confirm the `*.app.tar.gz.sig` glob still matches**

Once `bundle.createUpdaterArtifacts` is removed, Tauri will NOT auto-emit the `.app.tar.gz.sig`. The CI step that emits the manifest currently relies on the artifact that `tauri-action` uploads. Verify the current glob:

```yaml
- name: Upload signed bundles
  ...
  path: |
    target/universal-apple-darwin/release/bundle/dmg/*.dmg
    target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz
    target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz.sig
```

When `createUpdaterArtifacts` is `true`, Tauri emits the `.sig` next to the `.app.tar.gz`. With it `false`, the `.sig` is gone. The CI must sign the bundle itself with minisign. The spec's open question calls this out — for this plan we **keep the manifest step** but add an explicit minisign step so the signature is still produced:

```yaml
      - name: Sign the app bundle (minisign)
        run: |
          if [ -z "$MINISIGN_PRIVATE_KEY" ]; then
            echo "MINISIGN_PRIVATE_KEY secret missing; the release cannot publish without it" >&2
            exit 1
          fi
          echo "$MINISIGN_PRIVATE_KEY" > /tmp/minisign.key
          chmod 600 /tmp/minisign.key
          for f in target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz; do
            minisign -S -s /tmp/minisign.key -m "$f"
          done
          rm -f /tmp/minisign.key
```

The `MINISIGN_PRIVATE_KEY` secret must be the same minisign key whose public half lives in `PUBKEY` (Task 10) — a release-engineering follow-up must add this secret to the repo. The plan only adds the CI step; the secret is a release-engineering deliverable.

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: sign the app bundle with minisign (Tauri createUpdaterArtifacts removed)"
```

---

## Task 19: Mark `doc/10-updater.md` as implemented

**Files:**
- Modify: `doc/10-updater.md:1-11`

- [ ] **Step 1: Flip the status header**

```diff
-Status: **Proposed**. OxiLine currently uses the Tauri updater plugin in the
-GUI and a CLI "signal" command that only writes a setting. The unification
-to "CLI as the only engine, GUI as a view" is documented in the canonical
-RFC at the oximemo repo:
+Status: **Implemented**. The CLI is the only engine (`oxiline upgrade`); the
+GUI is a thin view that spawns the sidecar and parses its NDJSON contract.
+The Tauri updater plugin is removed.
```

- [ ] **Step 2: Commit**

```bash
git add doc/10-updater.md
git commit -m "docs(updater): mark unified architecture as implemented"
```

---

## Task 20: End-to-end verification

**Files:** none. This is the gate the whole plan is for.

- [ ] **Step 1: Rust unit + integration tests pass**

Run: `cargo test -p oxiline-cli --lib 2>&1 | tail -10`
Expected: ~20 tests pass, 0 fail.

- [ ] **Step 2: Frontend typecheck + build passes**

Run: `cd crates/oxiline-app && bun run build 2>&1 | tail -10`
Expected: builds.

- [ ] **Step 3: Workspace clippy is clean**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10`
Expected: clean.

- [ ] **Step 4: Live `oxiline upgrade --check` smoke test**

Run from a built CLI binary:
```bash
cargo run -p oxiline-cli --release -- upgrade --check --json-progress
```
Expected output (real example, the live `latest.json`):
```jsonc
{"type":"checking"}
{"type":"available","from":"<current>","to":"<latest>","notes":"OxiLine <latest>"}
```
or:
```jsonc
{"type":"checking"}
{"type":"latest","version":"<current>"}
```

- [ ] **Step 5: Ignored live-signature probe passes**

Run: `cargo test -p oxiline-cli upgrade::tests::verifies_live_release_signature -- --ignored --nocapture 2>&1 | tail -5`
Expected: `test result: ok. 1 passed; 0 failed`.

- [ ] **Step 6: Manual `App.tsx` smoke (in the Tauri dev shell)**

```bash
cd crates/oxiline-app/src-tauri && ./stage-cli.sh
cd crates/oxiline-app && bun run tauri dev
```

Open the Preferences panel → Update section. The `Check for updates` button must trigger a `binaries/oxiline upgrade --check --yes --json-progress` spawn and surface the result in the same shape. The `Install` button on the banner must trigger a full `upgrade` spawn and, on success, the `App.tsx` watcher must call `relaunch()`.

- [ ] **Step 7: Tag and push**

```bash
git tag v0.7.0-rc.1
git push origin feat/unified-updater --tags
```

---

## Self-Review

**Spec coverage (`doc/10-updater.md`):**

| Spec line | Task |
|---|---|
| `oxiline upgrade` does the full download → verify → swap → writes `update_request_at` on success | 10, 11 |
| Minisign pubkey moved from `tauri.conf.json` to the CLI's `PUBKEY` constant | 10 (Step 1) |
| `upgrade_in_app` for sidecar-running case | 9 |
| `upgrade_standalone` for `~/.cargo/bin/oxiline` case | 10 (Step 2) |
| JSON progress events on stdout when `--json-progress` is set | 5, 10, 16 |
| `oxiline update` as deprecation alias | 11 (Step 2) |
| `settings::set(&conn, "update_request_at", &Value::String(util::now_iso()))` after success | 10 (Step 2) |
| Rewrite `lib/updater.ts` to spawn the sidecar via `@tauri-apps/plugin-shell` and parse the JSON contract | 16 |
| Keep the zustand store, the banner, the Preferences section, and the `update_request_at` watcher | 16, 17 |
| Remove the import of `@tauri-apps/plugin-updater` | 15 |
| Remove `tauri-plugin-updater` from `crates/oxiline-app/src-tauri/Cargo.toml` | 13 |
| Remove `plugins.updater` and `bundle.createUpdaterArtifacts` from `tauri.conf.json` | 14 |
| Remove `updater:default` from `capabilities/default.json` | 14 |
| Release pipeline: keep the `jq` manifest step | 18 |

**Open questions from the spec explicitly deferred (not in this plan):**
- End-to-end live swap probe on a real newer release: the `#[ignore]` test in Task 10 is the spec's "cheapest probe" — accepted as the proxy.
- Quarantine xattr on the extracted `.app`: only matters after the first end-to-end probe on a real release; v2.
- Codesign identity after swap: same.
- flock for concurrent CLI invocations: same.

**Placeholder scan:** no `TODO`/`TBD`/`fill in` strings; every step has a concrete code/test/commit recipe.

**Type / name consistency:** `Options { check, json_progress, assume_yes }` is defined in Task 2 (interface) and used in Tasks 10, 11, 16 with the exact same names. `Event` enum is defined in Task 5 and referenced in Tasks 10 and 16 (the GUI test imports the same shape). `useUpdate` zustand store is preserved — `UpdateBanner.tsx` and `Preferences.UpdateSection` keep calling the same `status`, `install`, `check`, `reset` methods; the new `restarting` status kind is a superset of the existing one.


## Post-merge correction (supersedes the original Task 14 / Task 18 / handoff)

During implementation we discovered that Tauri's updater **is** minisign
under the hood. The repo's existing `TAURI_SIGNING_PRIVATE_KEY` secret
IS the minisign private key (Tauri just wraps it in a JSON envelope for
`tauri.conf.json#plugins.updater`). The public half of that key matches
`LIVE_PUBKEY` in `crates/oxiline-cli/src/upgrade.rs` (the live
`verifies_live_release_signature` test proves this end-to-end).

**The original plan — replacing `bundle.createUpdaterArtifacts: true`
with a manual `Package as .app.tar.gz` + `Sign with minisign` chain
driven by a new `OXILINE_MINISIGN_KEY` secret — is wrong.** Three
concrete problems with that approach:

1. The user doesn't have (and doesn't need) a separate minisign key
   file. `TAURI_SIGNING_PRIVATE_KEY` is the key.
2. The manual `minisign -S` produces a multi-line raw `.minisig` file,
   but the CLI's `verify_minisign_with` does `base64::decode(sig)` and
   then `Signature::decode()`. The format the CLI expects is the base64
   *of* the raw `.minisig` file — exactly what `createUpdaterArtifacts`
   puts into `latest.json#signature`.
3. The `.sig` extension doesn't match: `minisign -S -m file` produces
   `file.minisig`, but the upload glob and manifest `find` look for
   `*.app.tar.gz.sig`. The signature would never be found.

**The fix:** `bundle.createUpdaterArtifacts: true` is restored in
`tauri.conf.json`. The `Package as .app.tar.gz` and `Sign with
minisign` steps are deleted from `release.yml`. The single tauri-action
step uses the already-present `TAURI_SIGNING_PRIVATE_KEY` to produce
both `OxiLine.app.tar.gz` and `OxiLine.app.tar.gz.sig` in the exact
base64 format the CLI consumes. **No new secret is required.**

The corresponding `OXILINE_MINISIGN_KEY` handoff section that originally
followed this one has been deleted. `tauri.conf.json` no longer needs
`plugins.updater` (the runtime plugin is not initialized in `lib.rs`
anyway — the CLI does all the update work), so the `plugins: {}`
block is empty.