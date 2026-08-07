# In-app CLI install — implementation playbook

**Date:** 2026-08-07
**Origin:** Implemented in `oximemo`; this doc mirrors the approach for `oxiline`
(`oxiline-app` + `oxiline-cli` + `oxiline-core` share the same workspace shape).

## TL;DR

Ship the CLI binary **inside** the Tauri `.app` as an `externalBin` sidecar, then
expose it on `$PATH` through an explicit **"Install command"** button in Settings
(one-time macOS admin prompt). A first-launch nudge raises discoverability.
Headless/agent machines keep using the standalone release tarball — unchanged.

Result: one `.dmg` install → both GUI **and** `oxiline` on PATH.

## Why this design

- The CLI and the GUI already share the same on-disk data (same `oxiline-core`),
  so **no IPC or sidecar-spawning is needed** — the gap is purely "the binary
  isn't on `$PATH`."
- macOS offers no user-writable directory that is on the default `$PATH`
  *without* editing shell rc files. `/usr/local/bin` is on the default PATH
  (covers interactive **and** non-interactive/agent/SSH shells) but needs admin.
  A user-clicked button makes the one-time sudo prompt expected rather than
  creepy.
- `externalBin` (not `bundle.resources`) is mandatory: a nested Mach-O in
  `Contents/Resources/` is not reliably signed, so a signed/notarized app blocks
  it from the terminal. `externalBin` places the binary in `Contents/MacOS/` and
  signs it as part of the bundle in both signed and unsigned builds.

## Implementation (what actually changed in oximemo)

### 1. Bundle config — `tauri.conf.json`

```jsonc
"bundle": {
  // ...existing keys...
  "externalBin": ["binaries/oximemo"]   // Tauri appends -<target-triple>
}
```

### 2. Release workflow — stage the sidecar before bundling

After `cargo build --release -p oximemo-cli --target <triple>`, copy the product
to the path Tauri expects, **before** `cargo tauri build`:

```yaml
- name: Stage CLI sidecar for app bundle
  run: |
    mkdir -p apps/desktop/src-tauri/binaries
    cp "target/${{ env.TARGET }}/release/oximemo" \
       "apps/desktop/src-tauri/binaries/oximemo-${{ env.TARGET }}"
```

`binaries/` is gitignored (build artifact).

### 3. `build.rs` — keep `cargo check`/`clippy`/`tauri dev` working

**Critical gotcha:** tauri-build validates that every `externalBin` path **exists
at compile time**. Without the file, even `cargo check` fails. The release
workflow stages the real binary; local dev does not. So the crate's `build.rs`
drops a placeholder when the real binary is absent (it is never executed or
bundled — only `tauri build` bundles, and that path always stages the real one):

```rust
fn main() {
    let target = std::env::var("TARGET")
        .unwrap_or_else(|_| format!("{}-apple-darwin", std::env::consts::ARCH));
    let rel = format!("binaries/oximemo-{target}");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(&manifest).join(&rel);

    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            b"#!/bin/sh\necho 'run stage-cli.sh to build the real CLI' >&2\nexit 1\n",
        );
        println!("cargo:warning=oximemo CLI sidecar missing — created placeholder ...");
    }
    println!("cargo:rerun-if-changed={}", path.display());
    tauri_build::build();
}
```

### 4. Tauri commands — resolve the sidecar at runtime

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CliState { Installed, NotInstalled, Stale }

/// Path of the bundled CLI, derived from the running executable so it tracks
/// wherever the user installed the `.app`.
fn bundled_cli_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // externalBin sidecar lands in Contents/MacOS/ under its BASE name —
    // Tauri STRIPS the `-<triple>` suffix during bundling.
    Some(dir.join("oximemo"))
}

#[tauri::command]
pub fn cli_status() -> Result<CliState, String> {
    let link = std::path::Path::new("/usr/local/bin/oximemo");
    let Some(bundled) = bundled_cli_path() else {
        return Ok(CliState::NotInstalled);
    };
    let same = |a: &std::path::Path, b: &std::path::Path| {
        std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok()
    };
    match std::fs::read_link(link) {
        Ok(target) if same(&target, &bundled) => Ok(CliState::Installed),
        Ok(_) => Ok(CliState::Stale),
        Err(_) => Ok(if link.exists() { CliState::Stale } else { CliState::NotInstalled }),
    }
}

#[tauri::command]
pub fn install_cli() -> Result<(), String> {
    let target = bundled_cli_path()
        .ok_or_else(|| "could not locate the app bundle".to_string())?;
    if !target.exists() {
        return Err("bundled CLI binary is missing".to_string());
    }
    let q = target.display().to_string().replace('\'', "'\"'\"'");
    run_admin(&format!("ln -sf '{q}' /usr/local/bin/oximemo"))
}

#[tauri::command]
pub fn uninstall_cli() -> Result<(), String> {
    run_admin("rm -f /usr/local/bin/oximemo")
}

/// Run a shell snippet with administrator privileges via osascript. macOS shows
/// its standard auth dialog once; cancelling surfaces as an error.
fn run_admin(shell_script: &str) -> Result<(), String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "do shell script {} with administrator privileges",
            applescript_string(shell_script)
        ))
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn applescript_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
```

Register all three in `invoke_handler(tauri::generate_handler![ ... ])`.

### 5. Frontend

- **Typed wrappers** (`lib/api.ts`): `cliStatus()`, `installCli()`,
  `uninstallCli()` + `CliState` type.
- **Browser/dev fallback** (`lib/tauri.ts` `browserFallback`): `cli_status` →
  `"not-installed"`; `install_cli`/`uninstall_cli` throw "only available in the
  desktop app".
- **Settings section** ("Command-line tool"): queries `cli_status`, shows
  Install / Uninstall / Reinstall per state.
- **First-launch nudge** (top banner in `Shell`): shown once (until dismissed
  via `localStorage`, or once installed). Primary "Install now" + "Later".
  Gated to the real Tauri shell (`"__TAURI_INTERNALS__" in window`).
- **i18n** keys in `ko.ts` (source of truth) + `en.ts`.

### 6. Local dev helper — `src-tauri/stage-cli.sh`

Stages the real binary for a genuine local `cargo tauri build`:

```sh
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"                       # .../src-tauri/
TRIPLE=$(rustc -vV | sed -n 's/^host: //p')
ROOT="$(cd "../../../" && pwd)"            # workspace root (shared target/)
cargo build --release -p oximemo-cli
mkdir -p binaries
cp "$ROOT/target/release/oximemo" "binaries/oximemo-${TRIPLE}"
chmod +x "binaries/oximemo-${TRIPLE}"
```

## Gotchas (all learned the hard way during verification)

1. **tauri-build validates externalBin at compile time** → `cargo check`,
   `clippy`, and `tauri dev` all fail if the sidecar file is missing. Mitigated
   by the placeholder in `build.rs`.
2. **Tauri strips the `-<target-triple>` suffix when bundling.** Input
   `binaries/oximemo-aarch64-apple-darwin` → bundled `Contents/MacOS/oximemo`.
   Runtime resolution MUST use the base name (`join("oximemo")`), not the
   suffixed one. This bug is **invisible to `cargo check`** — only a real
   `tauri build` (inspecting the produced `.app`) exposes it.
3. **`cargo tauri build` reads the `CI` env var.** If set to `1` (harness/CI
   default) it errors `invalid value '1' for '--ci'`. Override with
   `CI=false cargo tauri build` locally.
4. **Resolve the sidecar from `current_exe().parent()`, never hardcode
   `/Applications/...`.** This makes the symlink correct regardless of where the
   user dragged the `.app`; the only `Stale` case is a post-install move (fixed
   by Reinstall).

## Verification checklist

- `cargo clippy -p <app> --all-targets -- -D warnings` clean.
- `tsc -b && vite build` (or equivalent) passes (i18n key parity enforced by
  `Record<keyof typeof ko, string>` on the EN dict).
- `stage-cli.sh` produces a runnable binary (`<bin> --version`).
- `CI=false cargo tauri build --debug` produces the `.app`; confirm the sidecar
  is present at `Contents/MacOS/<base-name>` — this is the step that catches
  gotcha #2.
- CI gates (`cargo check`, `cargo clippy`, frontend build) pass automatically
  thanks to the `build.rs` placeholder.

## Adapting to oxiline

oxiline has the same workspace shape, so the port is mechanical:

| oximemo | oxiline |
|---|---|
| `oximemo-cli` / binary `oximemo` | `oxiline-cli` / binary `oxiline` |
| `apps/desktop/src-tauri` | `crates/oxiline-app` (Tauri crate) |
| `tauri.conf.json` `externalBin: ["binaries/oximemo"]` | `["binaries/oxiline"]` |
| `/usr/local/bin/oximemo` symlink target | `/usr/local/bin/oxiline` |
| `join("oximemo")` runtime resolution | `join("oxiline")` |

Everything else (build.rs placeholder, the three commands, osascript helper,
frontend section + nudge, i18n, release staging step, stage script) transfers
verbatim — only the binary/app name changes.

## Remaining live check

The GUI button → macOS admin dialog → `which <bin>` flow needs a real running
`.app` plus an interactive admin password, so it cannot be exercised headlessly.
Finalize it on the next release tag (the release workflow stages the real signed
binary) or by launching a locally-built `.app` and clicking "Install command".
