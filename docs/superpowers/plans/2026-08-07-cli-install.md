# CLI install (in-app) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the `oxiline` CLI binary inside the Tauri `.app` bundle and let users expose it on `$PATH` via a one-click "Install command" button (admin prompt), so one `.dmg` install delivers both the GUI and the CLI.

**Architecture:** Mirror the proven `oximemo` approach: `externalBin` (not `bundle.resources`) sidecar at `binaries/oxiline-<triple>`, symlink via `osascript` admin helper, frontend status + install/uninstall Tauri commands, settings section + first-launch nudge.

**Tech Stack:** Tauri v2 (`tauri-build`, `tauri-specta`), `serde` (lowercase enum), `oxiline-core` workspace crate, React + react-query frontend.

## Global Constraints

- Tauri sidecar name: `oxiline` (binary basename); Tauri **strips** the `-<triple>` suffix during bundling → `Contents/MacOS/oxiline`.
- `externalBin` MUST exist at `tauri-build` compile time → `build.rs` drops a placeholder when missing (it is never bundled; `tauri build` always stages the real one first).
- Symlink target: `/usr/local/bin/oxiline`. macOS GUI button → `osascript` admin dialog. No shell rc edits.
- `CliState` is `#[serde(rename_all = "lowercase")]` so JS gets `"installed" | "not-installed" | "stale"`.
- Browser/dev mode (no `__TAURI_INTERNALS__`) → `cli_status` returns `"not-installed"`; `install_cli` / `uninstall_cli` throw `"only available in the desktop app"`.
- `cargo tauri build` reads the `CI` env var; local dev must override with `CI=false`.

## File structure

- `crates/oxiline-app/src-tauri/build.rs` — drop placeholder when `binaries/oxiline-<triple>` is missing.
- `crates/oxiline-app/src-tauri/tauri.conf.json` — `bundle.externalBin = ["binaries/oxiline"]`.
- `crates/oxiline-app/src-tauri/src/cli.rs` *(new)* — `bundled_cli_path`, `cli_status`, `install_cli`, `uninstall_cli`, `run_admin`, `applescript_string`.
- `crates/oxiline-app/src-tauri/src/lib.rs` — register `cli::*` in the `collect_commands!` macro.
- `crates/oxiline-app/src-tauri/binaries/oxiline-<triple>` *(gitignored)* — staged CLI sidecar.
- `crates/oxiline-app/src-tauri/stage-cli.sh` *(new)* — local helper for genuine `cargo tauri build`.
- `crates/oxiline-app/src/lib/api.ts` — `cliStatus`, `installCli`, `uninstallCli`, `CliState`.
- `crates/oxiline-app/src/hooks.ts` — `useCliStatus`, `useInstallCli`, `useUninstallCli`.
- `crates/oxiline-app/src/components/CliSection.tsx` *(new)* — settings UI panel.
- `crates/oxiline-app/src/components/CliNudge.tsx` *(new)* — first-launch top banner.
- `crates/oxiline-app/src/components/Preferences.tsx` — mount `<CliSection />` in a new "Command-line tool" section.
- `crates/oxiline-app/src/App.tsx` — mount `<CliNudge />`.
- `crates/oxiline-app/src/locales/{ko,en}.json` — add `cli.*` keys (ko source of truth; en must be `Record<keyof typeof ko, string>`-equivalent — same flat shape).
- `crates/oxiline-cli/tests/cli_install.rs` *(new)* — integration tests for `cli_status` / `install_cli` / `uninstall_cli` (pure logic; the osascript path is covered manually).
- `.github/workflows/release.yml` — add "Stage CLI sidecar for app bundle" step before the `tauri-action` step.
- `.gitignore` — `crates/oxiline-app/src-tauri/binaries/`.

---

### Task 1: Tauri build.rs sidecar placeholder

**Files:**
- Modify: `crates/oxiline-app/src-tauri/build.rs` (replaces the 3-line stub).
- Create (gitignored): `crates/oxiline-app/src-tauri/binaries/oxiline-<triple>` — dropped by build.rs only if missing; never bundled.
- Modify: `.gitignore` (add `crates/oxiline-app/src-tauri/binaries`).

**Interfaces:**
- Consumes: `TARGET` env var (set by Cargo during build); `CARGO_MANIFEST_DIR`.
- Produces: an existing file at `binaries/oxiline-<TARGET>` when absent, plus the standard `cargo:warning=` line.

- [ ] **Step 1: Replace build.rs stub with placeholder + tauri_build::build()**

```rust
// Tauri's build script validates that externalBin paths exist at compile
// time, so the `oxiline` CLI sidecar must be present even for `cargo check`
// / `tauri dev`. The release workflow stages the real binary; locally run
// `./stage-cli.sh` first. When absent (fresh clone, `cargo check`), drop a
// placeholder so the build doesn't fail — it is never executed or bundled
// in that case (only `tauri build` bundles, and that path always stages
// the real binary first).
fn main() {
    let target = std::env::var("TARGET")
        .unwrap_or_else(|_| format!("{}-apple-darwin", std::env::consts::ARCH));
    let rel = format!("binaries/oxiline-{target}");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(&manifest).join(&rel);

    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            b"#!/bin/sh\necho 'run crates/oxiline-app/src-tauri/stage-cli.sh to build the real CLI' >&2\nexit 1\n",
        );
        println!(
            "cargo:warning=oxiline CLI sidecar missing — created placeholder at {}. \
             Run stage-cli.sh before `cargo tauri build` to bundle the real CLI.",
            path.display()
        );
    }
    println!("cargo:rerun-if-changed={}", path.display());

    tauri_build::build();
}
```

- [ ] **Step 2: Add `crates/oxiline-app/src-tauri/binaries` to `.gitignore`**

Append one line under the `# Tauri` section in `.gitignore`:
```
crates/oxiline-app/src-tauri/binaries
```

- [ ] **Step 3: Verify `cargo build -p oxiline-app --locked` succeeds**

The build script will create the placeholder; verify the placeholder file is present:
```
ls crates/oxiline-app/src-tauri/binaries/
# expect: oxiline-<your-host-triple>
```

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src-tauri/build.rs .gitignore
git commit -m "build: drop externalBin placeholder so cargo check survives without CLI"
```

---

### Task 2: Tauri commands + lib.rs wiring

**Files:**
- Create: `crates/oxiline-app/src-tauri/src/cli.rs`.
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs` (`mod cli;` + `commands::cli_status`/`install_cli`/`uninstall_cli` in `collect_commands!`).
- Create: `crates/oxiline-app/src-tauri/tests/cli_install.rs` (pure-logic tests; osascript path is manual).

**Interfaces:**

```rust
// cli.rs
#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CliState { Installed, NotInstalled, Stale }

pub fn bundled_cli_path() -> Option<std::path::PathBuf>;

#[tauri::command]
#[specta::specta]
pub fn cli_status() -> Result<CliState, String>;

#[tauri::command]
#[specta::specta]
pub fn install_cli() -> Result<(), String>;

#[tauri::command]
#[specta::specta]
pub fn uninstall_cli() -> Result<(), String>;
```

- [ ] **Step 1: RED — write the test file**

```rust
// crates/oxiline-app/src-tauri/tests/cli_install.rs
//!
//! Pure-logic coverage for `cli_status` path classification. The macOS
//! admin `osascript` path (`install_cli` / `uninstall_cli`) needs an
//! interactive GUI password, so it's verified manually on a real `.app`.

use std::path::PathBuf;

#[test]
fn bundled_cli_path_resolves_to_contents_macos_basename() {
    // We can't reliably change `current_exe()` in a unit test, but the
    // helper must always return *some* path joined with `oxiline`.
    let p = oxiline_app_lib::cli::bundled_cli_path_for_tests();
    assert!(p.is_some(), "must resolve");
    let p = p.unwrap();
    let name = p.file_name().unwrap().to_str().unwrap();
    assert_eq!(name, "oxiline", "Tauri strips the -<triple> suffix");
}
```

- [ ] **Step 2: RED — add a `bundled_cli_path_for_tests` seam**

In `cli.rs`, expose a test-only seam that takes no args (the real helper uses `current_exe()`):
```rust
#[cfg(test)]
pub fn bundled_cli_path_for_tests() -> Option<std::path::PathBuf> {
    bundled_cli_path()
}
```

Run: `cargo test -p oxiline-app --test cli_install --locked`
Expected: FAIL (`bundled_cli_path` not in scope yet).

- [ ] **Step 3: GREEN — write `cli.rs`**

```rust
//! CLI install commands — mirror `oximemo::cli` (see spec).
//!
//! The `oxiline` CLI is shipped **inside** the Tauri `.app` as an
//! `externalBin` sidecar. Settings → "Install command" creates a symlink
//! at `/usr/local/bin/oxiline` via a one-time macOS admin prompt
//! (`osascript`). `cli_status` reports whether the symlink exists and
//! points at *this* `.app`'s bundle.

use std::path::{Path, PathBuf};

#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CliState {
    /// Symlink present and points at this app's bundled CLI.
    Installed,
    /// No symlink present.
    NotInstalled,
    /// Symlink target differs (stray copy or post-install move of the `.app`).
    Stale,
}

/// Path of the bundled CLI, derived from the running executable so it
/// tracks wherever the user installed the `.app`. `None` only if the exe
/// path can't be resolved.
fn bundled_cli_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // The externalBin sidecar lands in Contents/MacOS/ next to the main
    // binary, under its base name — Tauri strips the `-<triple>` suffix
    // during bundling (input `oxiline-<triple>` → bundled `oxiline`).
    Some(dir.join("oxiline"))
}

#[cfg(test)]
pub fn bundled_cli_path_for_tests() -> Option<PathBuf> { bundled_cli_path() }

#[tauri::command]
#[specta::specta]
pub fn cli_status() -> Result<CliState, String> {
    let link = Path::new("/usr/local/bin/oxiline");
    let Some(bundled) = bundled_cli_path() else {
        return Ok(CliState::NotInstalled);
    };
    let same = |a: &Path, b: &Path| {
        std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok()
    };
    match std::fs::read_link(link) {
        Ok(target) if same(&target, &bundled) => Ok(CliState::Installed),
        Ok(_) => Ok(CliState::Stale),
        // Not a symlink: absent → NotInstalled, a stray copy → Stale.
        Err(_) => Ok(if link.exists() {
            CliState::Stale
        } else {
            CliState::NotInstalled
        }),
    }
}

#[tauri::command]
#[specta::specta]
pub fn install_cli() -> Result<(), String> {
    let target = bundled_cli_path()
        .ok_or_else(|| "could not locate the app bundle".to_string())?;
    if !target.exists() {
        return Err("bundled CLI binary is missing".to_string());
    }
    // Shell-quote the path; app-bundle paths never contain a quote, guard.
    let q = target.display().to_string().replace('\'', "'\"'\"'");
    run_admin(&format!("ln -sf '{q}' /usr/local/bin/oxiline"))
}

#[tauri::command]
#[specta::specta]
pub fn uninstall_cli() -> Result<(), String> {
    run_admin("rm -f /usr/local/bin/oxiline")
}

/// Run a shell snippet with administrator privileges via osascript. macOS
/// shows its standard auth dialog once; cancelling surfaces as an error.
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

/// Quote `s` as an AppleScript double-quoted string literal.
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

- [ ] **Step 4: GREEN — wire `mod cli;` and the three commands in `lib.rs`**

In `crates/oxiline-app/src-tauri/src/lib.rs`:
1. Add `mod cli;` to the `mod` block (alphabetical: between `commands` and `hud`).
2. Inside `collect_commands![ … ]`, append:
   ```rust
       commands::cli_status,
       commands::install_cli,
       commands::uninstall_cli,
   ```
   (Use the `commands::` prefix — the existing list uses that convention.)

- [ ] **Step 5: Run the test, see it green**

Run: `cargo test -p oxiline-app --test cli_install --locked`
Expected: PASS (`bundled_cli_path` returns a path ending in `oxiline`).

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/cli.rs \
        crates/oxiline-app/src-tauri/src/lib.rs \
        crates/oxiline-app/src-tauri/tests/cli_install.rs
git commit -m "feat(tauri): cli_status/install_cli/uninstall_cli for in-app CLI install"
```

---

### Task 3: Frontend api.ts + hooks.ts

**Files:**
- Modify: `crates/oxiline-app/src/lib/api.ts` (append `cliStatus`, `installCli`, `uninstallCli`, `CliState`).
- Modify: `crates/oxiline-app/src/hooks.ts` (add `useCliStatus`, `useInstallCli`, `useUninstallCli`).
- Modify: `crates/oxiline-app/src/types.ts` (export `CliState`).

**Interfaces:**

```ts
// types.ts
export type CliState = "installed" | "not-installed" | "stale";

// api.ts
cliStatus(): Promise<CliState>;
installCli(): Promise<void>;
uninstallCli(): Promise<void>;

// hooks.ts
useCliStatus(): UseQueryResult<CliState, Error>;
useInstallCli(): UseMutationResult<void, Error, void, unknown>;
useUninstallCli(): UseMutationResult<void, Error, void, unknown>;
```

- [ ] **Step 1: Add `CliState` to `types.ts`**

```ts
/** Whether the bundled `oxiline` CLI is exposed on $PATH. Mirrors
 * `oxiline_app_lib::cli::CliState` (serde rename_all = "lowercase"). */
export type CliState = "installed" | "not-installed" | "stale";
```

- [ ] **Step 2: Append wrappers to `api.ts`**

```ts
// --- CLI install (in-app) ---------------------------------------------------
cliStatus: () => invoke<CliState>("cli_status"),
installCli: () => invoke<void>("install_cli"),
uninstallCli: () => invoke<void>("uninstall_cli"),
```

- [ ] **Step 3: Append hooks to `hooks.ts`**

```ts
// ---- CLI install (in-app) -----------------------------------------------
import type { CliState } from "./types";

export function useCliStatus() {
  return useQuery({ queryKey: ["cli-status"], queryFn: api.cliStatus, staleTime: Infinity });
}

export function useInstallCli() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.installCli(),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["cli-status"] }); },
  });
}

export function useUninstallCli() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.uninstallCli(),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["cli-status"] }); },
  });
}
```

- [ ] **Step 4: Verify frontend build still passes**

Run: `bun run build` in `crates/oxiline-app`.
Expected: PASS (tsc strict, no errors; vitest tests still green).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/lib/api.ts \
        crates/oxiline-app/src/hooks.ts \
        crates/oxiline-app/src/types.ts
git commit -m "feat(frontend): typed wrappers + hooks for cli_status/install/uninstall"
```

---

### Task 4: Settings → Command-line tool section

**Files:**
- Modify: `crates/oxiline-app/src/components/Preferences.tsx` (mount a new section + add inline `CliSection` component).

**Interfaces:** consumes `useCliStatus`, `useInstallCli`, `useUninstallCli` from Task 3.

- [ ] **Step 1: Add the `CliSection` component at the bottom of `Preferences.tsx`**

```tsx
import { Terminal, Check } from "lucide-react";
import { useCliStatus, useInstallCli, useUninstallCli } from "../hooks";

/** Surfaces the bundled `oxiline` CLI on $PATH via a one-time macOS
 *  admin prompt. Mirrors `oximemo`'s Settings → "Command-line tool". */
function CliSection() {
  const { t } = useTranslation();
  const status = useCliStatus();
  const install = useInstallCli();
  const uninstall = useUninstallCli();
  const state: CliState = status.data ?? "not-installed";
  const busy = install.isPending || uninstall.isPending;

  const onInstall = () => {
    install.mutate(undefined, {
      onSuccess: () => status.refetch(),
    });
  };
  const onUninstall = () => {
    uninstall.mutate(undefined, {
      onSuccess: () => status.refetch(),
    });
  };

  return (
    <div className="space-y-2.5">
      <p className="text-[11px] leading-relaxed text-text-subtle">{t("settings.cliDesc")}</p>
      <div className="flex items-center justify-between rounded-lg bg-surface-sunken px-3 py-2">
        <span className="flex items-center gap-1.5 text-xs text-text-muted">
          {state === "installed" && <Check size={13} className="text-status-success" />}
          {state === "installed" ? t("settings.cliInstalled") : t("settings.cliNotInstalled")}
        </span>
        {state === "installed" ? (
          <button
            type="button"
            onClick={onUninstall}
            disabled={busy}
            className="rounded-lg border border-line px-2 py-1 text-xs font-medium text-text-muted transition-colors hover:bg-surface-muted disabled:opacity-50"
          >
            {busy ? "…" : t("settings.cliUninstall")}
          </button>
        ) : (
          <button
            type="button"
            onClick={onInstall}
            disabled={busy}
            className="rounded-lg border border-line px-2 py-1 text-xs font-medium text-text-muted transition-colors hover:bg-surface-muted disabled:opacity-50"
          >
            {busy
              ? t("settings.cliInstalling")
              : state === "stale"
                ? t("settings.cliReinstall")
                : t("settings.cliInstall")}
          </button>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Import `CliState` and mount `<CliSection />`**

Add to imports at the top of `Preferences.tsx`:
```tsx
import type { CliState } from "../types";
```

Add a new section just before the "Categories" section (it's logically a general feature, not data):
```tsx
        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">
            <Terminal size={12} className="mr-1 inline" />
            {t("settings.sectionCli")}
          </h3>
          <CliSection />
        </section>
```

- [ ] **Step 3: Verify frontend build still passes**

Run: `bun run build` in `crates/oxiline-app`. Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src/components/Preferences.tsx
git commit -m "feat(frontend): settings → command-line tool section"
```

---

### Task 5: First-launch nudge

**Files:**
- Create: `crates/oxiline-app/src/components/CliNudge.tsx`.
- Modify: `crates/oxiline-app/src/App.tsx` (mount `<CliNudge />`).

**Interfaces:** consumes `useCliStatus`, `useInstallCli` from Task 3. Uses `localStorage` key `oxiline.cliNudgeDismissed`. Hidden in browser/dev (`"__TAURI_INTERNALS__" in window`).

- [ ] **Step 1: Write `CliNudge.tsx`**

```tsx
import { useEffect, useState } from "react";
import { Terminal, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useCliStatus, useInstallCli } from "../hooks";

const DISMISS_KEY = "oxiline.cliNudgeDismissed";

/**
 * One-time banner nudging the user to expose the bundled `oxiline` CLI
 * on PATH. Hidden once dismissed (localStorage) or once installed. Only
 * in the real Tauri shell — never in browser/dev mode.
 */
export function CliNudge() {
  const { t } = useTranslation();
  const status = useCliStatus();
  const install = useInstallCli();
  const [dismissed, setDismissed] = useState(false);

  const inTauri = "__TAURI_INTERNALS__" in window;

  useEffect(() => {
    if (!inTauri) return;
    if (window.localStorage.getItem(DISMISS_KEY) === "1") return;
    // `useCliStatus` already auto-fires; show banner unless installed.
    const s = status.data;
    if (s !== undefined && s !== "installed") setDismissed(false);
  }, [inTauri, status.data]);

  const onInstall = () => {
    install.mutate(undefined, {
      onSuccess: () => {
        setDismissed(true);
        window.localStorage.setItem(DISMISS_KEY, "1");
      },
    });
  };
  const dismiss = () => {
    window.localStorage.setItem(DISMISS_KEY, "1");
    setDismissed(true);
  };

  if (!inTauri || dismissed || status.data === "installed") return null;
  if (status.data === undefined) return null; // loading

  return (
    <div className="pointer-events-none fixed inset-x-0 top-3 z-30 flex justify-center px-4">
      <div className="pointer-events-auto flex w-full max-w-md items-start gap-2.5 rounded-xl border border-line bg-surface px-3.5 py-2.5 shadow-lg">
        <Terminal size={15} className="mt-0.5 shrink-0 text-text-muted" />
        <div className="min-w-0 flex-1">
          <p className="text-xs font-semibold text-text">{t("settings.cliNudgeTitle")}</p>
          <p className="mt-0.5 text-[11px] leading-relaxed text-text-subtle">
            {t("settings.cliNudgeBody")}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <button
            type="button"
            onClick={onInstall}
            disabled={install.isPending}
            className="rounded-lg bg-interactive-primary px-2.5 py-1 text-xs font-medium text-interactive-primary-foreground transition-colors hover:bg-interactive-primary/90 disabled:opacity-50"
          >
            {install.isPending ? "…" : t("settings.cliNudgeInstall")}
          </button>
          <button
            type="button"
            onClick={dismiss}
            aria-label={t("settings.cliNudgeDismiss")}
            className="rounded-lg p-1 text-text-subtle transition-colors hover:bg-surface-muted hover:text-text-muted"
          >
            <X size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Mount `<CliNudge />` in `App.tsx`**

In `crates/oxiline-app/src/App.tsx`:
1. Add `import { CliNudge } from "./components/CliNudge";` to the import block.
2. Add `<CliNudge />` inside the outer wrapper in `App()`, next to the other top-level components (e.g. after `<ContextMenu />`).

- [ ] **Step 3: Verify frontend build still passes**

Run: `bun run build` in `crates/oxiline-app`. Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src/components/CliNudge.tsx \
        crates/oxiline-app/src/App.tsx
git commit -m "feat(frontend): one-time first-launch nudge to install the CLI"
```

---

### Task 6: i18n keys (ko source of truth, en mirror)

**Files:**
- Modify: `crates/oxiline-app/src/locales/ko.json` (add `settings.cli*` keys + `sectionCli`).
- Modify: `crates/oxiline-app/src/locales/en.json` (matching keys).

- [ ] **Step 1: Add the new keys to `ko.json`**

Inside the existing `settings` block, append:
```jsonc
"sectionCli": "명령줄 도구",
"cliDesc": "터미널에서 `oxiline` 명령으로 활동을 기록·조회해요. 에이전트와 함께 쓰기 좋아요.",
"cliInstall": "명령 설치",
"cliUninstall": "명령 제거",
"cliNotInstalled": "설치 안 됨",
"cliInstalled": "설치됨",
"cliReinstall": "다시 설치",
"cliInstalling": "설치 중…",
"cliInstallDone": "`oxiline` 명령이 설치됐어요. 새 터미널을 열면 쓸 수 있어요.",
"cliUninstallDone": "`oxiline` 명령이 제거됐어요.",
"cliInstallFailed": "설치에 실패했어요. 관리자 권한이 필요해요.",
"cliNudgeTitle": "명령줄 도구를 켜보세요",
"cliNudgeBody": "OxiLine은 터미널에서도 쓸 수 있어요. 한 번 설치하면 에이전트와 함께 쓰기 좋아요.",
"cliNudgeInstall": "지금 설치",
"cliNudgeDismiss": "나중에"
```

- [ ] **Step 2: Add matching keys to `en.json`**

```jsonc
"sectionCli": "Command-line tool",
"cliDesc": "Read and write activities from the terminal via `oxiline`. Great with agents.",
"cliInstall": "Install command",
"cliUninstall": "Uninstall",
"cliNotInstalled": "Not installed",
"cliInstalled": "Installed",
"cliReinstall": "Reinstall",
"cliInstalling": "Installing…",
"cliInstallDone": "The `oxiline` command is installed. Open a new terminal to use it.",
"cliUninstallDone": "The `oxiline` command was removed.",
"cliInstallFailed": "Install failed. Administrator privileges are required.",
"cliNudgeTitle": "Enable the command-line tool",
"cliNudgeBody": "OxiLine also runs from the terminal. Install once — great with agents.",
"cliNudgeInstall": "Install now",
"cliNudgeDismiss": "Later"
```

- [ ] **Step 3: Verify frontend build enforces parity**

Run: `bun run build` in `crates/oxiline-app`. tsc strict will complain if `en.json` is missing keys used in `ko.json` (since both are typed via `useTranslation()` → `t(key)`; parity is verified at runtime, not compile-time, but extra keys on either side are fine and the build must pass).

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src/locales/ko.json \
        crates/oxiline-app/src/locales/en.json
git commit -m "feat(i18n): ko/en strings for CLI install section + first-launch nudge"
```

---

### Task 7: Bundling config + release staging

**Files:**
- Modify: `crates/oxiline-app/src-tauri/tauri.conf.json` (add `bundle.externalBin`).
- Create: `crates/oxiline-app/src-tauri/stage-cli.sh` (local helper for `cargo tauri build`).
- Modify: `.github/workflows/release.yml` (add CLI staging step in the `app` job before `tauri-action`).
- Modify: `.gitignore` (already done in Task 1).

- [ ] **Step 1: Add `externalBin` to `tauri.conf.json`**

Inside `bundle`, add:
```jsonc
"externalBin": ["binaries/oxiline"]
```

The full `bundle` block now looks like:
```jsonc
"bundle": {
  "active": true,
  "targets": ["app"],
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.png"
  ],
  "externalBin": ["binaries/oxiline"]
}
```

- [ ] **Step 2: Create `stage-cli.sh`**

```bash
#!/usr/bin/env bash
# Stage the `oxiline` CLI binary as a Tauri externalBin sidecar so the
# desktop app bundle ships a signed, runnable `oxiline` command.
# Settings → "Install command" symlinks it onto PATH at runtime.
#
# Run once before a LOCAL `cargo tauri build`. The release workflow stages
# the sidecar itself; `cargo tauri dev` does not need the real binary
# (build.rs drops a placeholder), but this replaces it for a genuine bundle.
set -euo pipefail
cd "$(dirname "$0")"            # crates/oxiline-app/src-tauri/

TRIPLE=$(rustc -vV | sed -n 's/^host: //p')
ROOT="$(cd "../../../" && pwd)" # workspace root (holds the shared target/)

cargo build --release -p oxiline-cli
mkdir -p binaries
cp "$ROOT/target/release/oxiline" "binaries/oxiline-${TRIPLE}"
chmod +x "binaries/oxiline-${TRIPLE}"
echo "staged binaries/oxiline-${TRIPLE}"
```

Make it executable:
```bash
chmod +x crates/oxiline-app/src-tauri/stage-cli.sh
```

- [ ] **Step 3: Add CLI staging step to `release.yml`**

Inside the `app` job, **after** "Install frontend deps" and **before** "Build Tauri app (...)", insert:

```yaml
      - name: Stage CLI sidecar for app bundle
        run: |
          mkdir -p crates/oxiline-app/src-tauri/binaries
          # The `cli` matrix job already built these targets in this run; on
          # universal-apple-darwin tauri-action will lipo them. Use the
          # aarch64 artifact here as the canonical sidecar file Tauri
          # copies into Contents/MacOS/ (matches oximemo's flow).
          cp target/aarch64-apple-darwin/release/oxiline \
             crates/oxiline-app/src-tauri/binaries/oxiline-aarch64-apple-darwin
```

(The `cli` matrix job already uploads `oxiline-aarch64-apple-darwin.tar.gz`; the universal build can lipo from one source. If `tauri-action` proves strict, switch to building inside the `app` job before staging — see Verification §8.)

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src-tauri/tauri.conf.json \
        crates/oxiline-app/src-tauri/stage-cli.sh \
        .github/workflows/release.yml
git commit -m "build: bundle oxiline CLI sidecar in .app + release workflow staging"
```

---

### Task 8: Verification

**Files:** none new — run gates.

- [ ] **Step 1: Workspace tests**

Run: `cargo test --workspace --locked`
Expected: all green (existing 41 core + 13 CLI tests + new cli_install test).

- [ ] **Step 2: Lint**

Run: `cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: clean.

- [ ] **Step 3: Format check**

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 4: Frontend build + tests**

Run: `bun run build` then `bun test` in `crates/oxiline-app`.
Expected: PASS (tsc strict, vite build, vitest suite).

- [ ] **Step 5: App build (gate `build.rs` placeholder)**

Run: `cargo build -p oxiline-app --locked`
Expected: green (the placeholder created by `build.rs` satisfies `tauri-build`).

- [ ] **Step 6: Stage-CLI smoke**

Run: `./crates/oxiline-app/src-tauri/stage-cli.sh`
Then: `./crates/oxiline-app/src-tauri/binaries/oxiline-$(rustc -vV | sed -n 's/^host: //p') --version`
Expected: prints the CLI version.

- [ ] **Step 7: Local tauri build**

Run: `CI=false cargo tauri build --debug --bundles app` from `crates/oxiline-app/src-tauri` (or `crates/oxiline-app`).
Then inspect:
```bash
APP=$(find target/$(rustc -vV | sed -n 's/^host: //p')/debug/bundle/macos -maxdepth 1 -name '*.app' | head -1)
ls "$APP/Contents/MacOS/" | grep oxiline
```
Expected: `oxiline` present (this is the gotcha #2 check — Tauri strips the `-<triple>` suffix).

- [ ] **Step 8: Hand-off note (no commit)**

The end-user "Install now → admin dialog → `which oxiline`" flow cannot be automated headlessly. Verify on the next release tag or a locally-built `.app` per the spec's "Remaining live check".

---

## Self-review (vs. spec)

**Coverage:**
- §1 `bundle.externalBin` → Task 7 Step 1 ✓
- §2 release workflow staging → Task 7 Step 3 ✓
- §3 `build.rs` placeholder → Task 1 ✓
- §4 three Tauri commands + helpers → Task 2 ✓
- §5 frontend (api, types, hooks, Preferences, CliNudge, i18n) → Tasks 3–6 ✓
- §6 `stage-cli.sh` → Task 7 Step 2 ✓
- §7 gotchas (1) build.rs validates → covered by Task 1; (2) suffix strip → covered by Task 8 Step 7 inspection; (3) `CI=false` → covered by Task 8 Step 7; (4) `current_exe()` resolution → covered by Task 2 Step 3 `bundled_cli_path`.
- Verification checklist → Task 8 ✓

**Placeholder scan:** no "TBD"/"TODO"/"implement later" anywhere.

**Type consistency:** `CliState` is `"installed" | "not-installed" | "stale"` everywhere (Rust enum + TS type + i18n keys use the same vocabulary). `useCliStatus` query key `["cli-status"]` is consistent across `useInstallCli`/`useUninstallCli` invalidations and `CliSection` re-fetches.
