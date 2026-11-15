#!/usr/bin/env bash
# Build the map viewer for Windows and open a map of the install in a native
# window, from WSL: scripts/view-map.sh [map-name] [more s2-viewer options]
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/recording-guard.sh
refuse_while_recording
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
screens="${S2_SCREENS_DIR:-/mnt/c/Users/${USER:-$(id -un)}/Projects/s2-reference/screens}"
mkdir -p "$screens"
cargo build --release --target x86_64-pc-windows-gnu -p s2-viewer
source scripts/windows-copy.sh
windows_copy target/x86_64-pc-windows-gnu/release/s2-viewer.exe
# The Windows program needs Windows paths; wslpath -w turns a WSL path into one.
"$copy" \
    --game-dir "$(wslpath -w "$S2_GAME_DIR")" \
    --screenshot-dir "$(wslpath -w "$screens")" \
    --map-name "${1:-campaign01}" "${@:2}"
