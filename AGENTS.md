# AGENTS.md — OxiLine

> macOS routine app: Tauri v2 + React, menu-bar resident.
> Workspace: `crates/{oxiline-core, oxiline-cli, oxiline-app}`. Binary `oxiline-app`,
> product `OxiLine`, bundle id `com.oxi.oxiline`, frontend dist embedded at
> `crates/oxiline-app/dist`.

## Project Stack

- Tauri v2, Rust 1.85+, bun 1.3.14+ (frontend), macOS Apple Silicon.

## Commands

```bash
# Build (canonical path — no proc-macro bug)
cd crates/oxiline-app
CI=false bunx --package @tauri-apps/cli@2 tauri build
# CI=false is REQUIRED: the CI env makes bunx inject `--ci 1`, and the tauri
# CLI only accepts `--ci true|false` ("invalid value '1'").
# @tauri-apps/cli is NOT a devDependency → bunx --package, never `bun tauri`.
# Output: target/release/bundle/macos/OxiLine.app (ad-hoc signed, arm64).

# Install to /Applications
pkill -x oxiline-app 2>/dev/null
rm -rf /Applications/OxiLine.app
cp -R target/release/bundle/macos/OxiLine.app /Applications/
xattr -dr com.apple.quarantine /Applications/OxiLine.app
open /Applications/OxiLine.app
```

## Gotchas

- Kernel process name is `oxiline-app` (from `[[bin]] name`), NOT "OxiLine": `pgrep -x oxiline-app`.
- A direct `oxiline-app` binary run exits 0 silently when an instance is already up (tauri-plugin-single-instance) — that is NOT a crash.
- Fallback (only if the canonical build fails): `bun run build`; `rm -rf target/release/deps/libtauri_macros* target/release/.fingerprint/tauri-macros*`; `cargo build -p oxiline-app --release`; hand-assemble `Contents/{MacOS,Resources}` + Info.plist (`CFBundleIdentifier=com.oxi.oxiline`, `CFBundleExecutable=oxiline-app`), `cp -R dist` in, `codesign -d --force -s -`.

## Boundaries

- Do not hand-assemble Info.plist / manual codesign unless the canonical path actually fails.
