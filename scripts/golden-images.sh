#!/usr/bin/env bash
# The golden images (docs/test-coverage.md): fixed scenes drawn by the
# Windows build from WSL, compared with stored pictures pixel by pixel.
#
#   scripts/golden-images.sh [--bless] [NAME_PATTERN]
#
# Scenes: tests/golden/images/scenes.txt. Pictures of this run go to
# target/golden-images/actual, the differences of failed scenes to
# target/golden-images/diff (the expected picture darkened, differing pixels
# red). --bless makes this run's pictures the stored ones and rewrites
# tests/golden/images/MANIFEST (their SHA-256), the file that is committed;
# the commit must name the spec rule behind the change (as for the golden
# hashes). The stored pictures show the original's art and stay out of git.
#
# A pixel differs when a channel is more than TOLERANCE (default 8) apart; a
# scene fails when more than MAX_SHARE (default 0.002) of its pixels differ.
# Needs S2_GAME_DIR and a desktop (the window must open); leave the mouse
# alone while it runs.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/recording-guard.sh
refuse_while_recording
source scripts/input-guard.sh
refuse_without_desktop_input
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
bless=""
if [[ "${1:-}" == "--bless" ]]; then bless=1; shift; fi
pattern="${1:-}"
tolerance="${TOLERANCE:-8}"
max_share="${MAX_SHARE:-0.002}"
dir=tests/golden/images
out=target/golden-images
# NO_BUILD=1: use the executables as they are (built before; the sources
# may change meanwhile).
if [[ -z "${NO_BUILD:-}" ]]; then
  cargo build --release --target x86_64-pc-windows-gnu -p s2-viewer
  cargo build --release -p s2-app
fi
source scripts/windows-copy.sh
windows_copy target/x86_64-pc-windows-gnu/release/s2-viewer.exe
viewer="$copy"
app=target/release/s2-app
game="$(wslpath -w "$S2_GAME_DIR")"
mkdir -p "$out/actual" "$out/diff" "$out/log"
status=0
count=0
while IFS='|' read -r name args; do
  name="${name//[[:space:]]/}"
  [[ -z "$name" || "$name" == \#* ]] && continue
  [[ -n "$pattern" && "$name" != *$pattern* ]] && continue
  count=$((count + 1))
  actual="$out/actual/$name.png"
  rm -f "$actual" "$out/diff/$name.png"
  # shellcheck disable=SC2086
  if ! "$viewer" --game-dir "$game" $args --size 1024x768 \
      --shot "$(wslpath -w "$PWD/$actual")" </dev/null >"$out/log/$name.log" 2>&1 \
      || [[ ! -f "$actual" ]]; then
    echo "$name: no picture (log: $out/log/$name.log)"
    status=1
    continue
  fi
  if [[ -n "$bless" ]]; then
    cp "$actual" "$dir/$name.png"
    echo "$name: blessed"
  elif [[ ! -f "$dir/$name.png" ]]; then
    echo "$name: no stored picture (run with --bless, or get the blessed set)"
    status=1
  elif ! "$app" image-diff "$dir/$name.png" "$actual" --tolerance "$tolerance" \
      --max-share "$max_share" --diff "$out/diff/$name.png" </dev/null | sed "s|^$actual|$name|"; then
    status=1
  fi
done < "$dir/scenes.txt"
echo "$count scenes"
if [[ -n "$bless" ]]; then
  (cd "$dir" && sha256sum -- *.png > MANIFEST)
  echo "$dir/MANIFEST written: commit it, naming the spec rule behind the change"
elif [[ -f "$dir/MANIFEST" ]] && ! (cd "$dir" && sha256sum --quiet -c MANIFEST 2>/dev/null); then
  echo "note: the stored pictures are not the blessed set of $dir/MANIFEST"
fi
exit $status
