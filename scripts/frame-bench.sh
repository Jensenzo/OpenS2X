#!/usr/bin/env bash
# The frame-time benchmark (docs/test-coverage.md): the viewer on a late
# game, frames timed, compared with this PC's baseline.
#
#   scripts/frame-bench.sh            run it; fail when the median frame time
#                                     is more than 25 % above the baseline, or
#                                     when the game runs slower than real time
#                                     (fewer than 9.5 ticks per second at
#                                     speed 1; ECO-TIME-1.2 to 1.4: 10)
#   scripts/frame-bench.sh --bless    make this run's times the baseline
#
# The scene: MP_2P_Storm_Coast, both players computer players (normal), played
# headless to tick 100 000 (about 30 s), then the 3D view at the start camera
# over player 0's headquarters at game speed 1, 120 frames to settle and 600
# timed (`s2-viewer --bench`). The report goes to target/frame-bench/report.txt;
# the baseline is tests/bench/frame-bench.txt (this PC's timing; outside
# tests/golden, since it is not game behaviour, though like the golden pictures
# it is this PC's own). Needs S2_GAME_DIR and a desktop; leave
# the PC alone while it runs (another program's load shows in the times).
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/recording-guard.sh
refuse_while_recording
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
bless=""
[[ "${1:-}" == "--bless" ]] && bless=1
out=target/frame-bench
baseline=tests/bench/frame-bench.txt
mkdir -p "$out" "$(dirname "$baseline")"
if [[ -z "${NO_BUILD:-}" ]]; then
  cargo build --release --target x86_64-pc-windows-gnu -p s2-viewer
fi
source scripts/windows-copy.sh
windows_copy target/x86_64-pc-windows-gnu/release/s2-viewer.exe
report="$out/report.txt"
rm -f "$report"
"$copy" --game-dir "$(wslpath -w "$S2_GAME_DIR")" --map-name MP_2P_Storm_Coast \
  --ai 0=2 --ai 1=2 --start-resources 1 --run 100000 --3d --size 1280x900 \
  --bench 600 --bench-report "$(wslpath -w "$PWD/$report")" \
  </dev/null >"$out/viewer.log" 2>&1 || true
[[ -f "$report" ]] || { echo "no report (log: $out/viewer.log)" >&2; exit 1; }
tr -d '\r' <"$report"
# The median: the mean moves by a quarter between two runs of the same
# build (a few slow frames), the median by a few percent.
median() { grep '^frame-ms-median' "$1" | tr -d '\r' | awk '{print $3}'; }
if [[ -n "$bless" ]]; then
  tr -d '\r' <"$report" >"$baseline"
  echo "baseline written: $baseline"
  exit 0
fi
[[ -f "$baseline" ]] || { echo "no baseline: run with --bless first" >&2; exit 1; }
rate="$(grep '^ticks-per-second' "$report" | tr -d '\r' | awk '{print $3}')"
if awk -v r="${rate:-0}" 'BEGIN { exit !(r < 9.5) }'; then
  echo "$rate ticks per second at speed 1: the game falls behind real time (10)" >&2
  exit 1
fi
echo "$rate ticks per second at speed 1 (real time: 10)"
now="$(median "$report")"
was="$(median "$baseline")"
if awk -v now="$now" -v was="$was" 'BEGIN { exit !(now > was * 1.25) }'; then
  echo "median frame $now ms, baseline $was ms: more than 25 % slower" >&2
  exit 1
fi
echo "median frame $now ms, baseline $was ms: within 25 %"
