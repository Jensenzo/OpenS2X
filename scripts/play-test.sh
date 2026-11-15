#!/usr/bin/env bash
# Run the play harness (crates/viewer/tests/play.rs) from WSL: the Windows
# build of the viewer is driven in a native window, since a Linux build has
# no display here. scripts/play-test.sh [cargo test options]
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/recording-guard.sh
refuse_while_recording
source scripts/input-guard.sh
refuse_without_desktop_input
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
cargo build --release --target x86_64-pc-windows-gnu -p s2-viewer
source scripts/windows-copy.sh
windows_copy target/x86_64-pc-windows-gnu/release/s2-viewer.exe
export S2_PLAY_EXE="$PWD/$copy"
# The game data the program under test reads: the Windows side's own copy
# of the install (install-local.sh --data) when it holds the same files as
# S2_GAME_DIR (names and sizes), else S2_GAME_DIR. Read over \\wsl.localhost,
# a first read of a sound, a model or an animation stalls its frame by 30 to
# 150 ms, which panning_stays_smooth counts as the program's (the same build,
# 2026-11-10: p99 35 to 38 ms over \\wsl, 23 from the copy; docs/test-coverage.md).
if [[ -z "${S2_PLAY_GAME_DIR:-}" ]]; then
  profile_w="$(powershell.exe -NoProfile -Command '[Environment]::GetFolderPath("UserProfile")' | tr -d '\r')"
  local_copy="$(wslpath -u "$profile_w")/Games/S2-data"
  listing() { (cd "$1" && find . -type f -printf '%P %s\n' | LC_ALL=C sort); }
  if [[ -d "$local_copy/data" ]] && cmp -s <(listing "$S2_GAME_DIR") <(listing "$local_copy"); then
    export S2_PLAY_GAME_DIR="$local_copy"
  elif [[ -d "$local_copy/data" ]]; then
    echo "play-test: $local_copy differs from S2_GAME_DIR; reading the game over \\\\wsl" >&2
  fi
fi
echo "play-test: game data from ${S2_PLAY_GAME_DIR:-$S2_GAME_DIR}"
# One test at a time: the play scripts move the desktop's real pointer,
# which must not wander over the colour probe's window while it measures.
cargo test -p s2-viewer --test play -- --nocapture --test-threads=1 "$@"
