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
