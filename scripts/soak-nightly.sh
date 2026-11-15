#!/usr/bin/env bash
# The long soak for the PC (docs/test-coverage.md): computer players against
# each other on every free play map of the install, 200 000 ticks each, every
# invariant of s2_sim::invariants checked after every tick.
#
#   scripts/soak-nightly.sh [TICKS] [SEED]
#
# Logs and the replay files of failures go to target/soak/nightly-<date>/;
# the last line of each log says whether all invariants held. A failure is
# reproduced with `s2-app soak --replay <file>`. Runs half the CPUs' worth
# of maps at once.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
ticks="${1:-200000}"
seed="${2:-1}"
out="target/soak/nightly-$(date +%Y%m%d-%H%M)"
mkdir -p "$out"
cargo build --release -p s2-app
# The run keeps its own copy, so a rebuild meanwhile does not change it.
cp target/release/s2-app "$out/s2-app"
jobs=$(( $(nproc) / 2 ))
(( jobs < 1 )) && jobs=1
maps=$(ls "$S2_GAME_DIR/data/maps/freeGameMaps/" | sed -n 's/\.s2m$//p')
echo "$(echo "$maps" | wc -l) maps, $ticks ticks, seed $seed, $jobs at once: $out"
export out ticks seed
echo "$maps" | xargs -P "$jobs" -I{} bash -c \
  '"$out/s2-app" soak "{}" "$ticks" --seed "$seed" --out "$out" > "$out/{}.log" 2>&1; echo "{}: $(tail -1 "$out/{}.log")"'
failed=$(grep -L "all invariants held" "$out"/*.log || true)
if [[ -n "$failed" ]]; then
  echo "failed:"
  echo "$failed"
  exit 1
fi
echo "all maps held every invariant"
