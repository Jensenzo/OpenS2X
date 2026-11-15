#!/usr/bin/env bash
# The golden state hashes (tests/golden/hash, docs/test-coverage.md).
#
#   scripts/golden.sh            check every golden file with the release build,
#                                and with the Windows build (via WSL) when it
#                                can run, which must give the same lines
#   scripts/golden.sh --bless    write them again after an intended change; the
#                                commit message must name the spec rule that
#                                caused it (CI refuses a golden change without)
#
# Maps from the install need S2_GAME_DIR; without it only the fixtures run.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/recording-guard.sh
refuse_while_recording
bless=""
[[ "${1:-}" == "--bless" ]] && bless="--bless"

# name, then the hash arguments
GOLDEN=(
  "fixture-flat2 fixture:flat2 5000 --seed 1 --every 500"
  "fixture-flat4 fixture:flat4 5000 --seed 1 --every 500"
  "fixture-town2 fixture:town2 5000 --seed 1 --every 500"
  "fixture-town4 fixture:town4 5000 --seed 1 --every 500"
  "MP_2P_Storm_Coast MP_2P_Storm_Coast 5000 --seed 1 --every 500"
  "MP_4P_crossroads MP_4P_crossroads 5000 --seed 1 --every 500"
  "MP_6P_Paradise_Island MP_6P_Paradise_Island 5000 --seed 1 --every 500"
)

cargo build --release -p s2-app
linux=target/release/s2-app
if [[ -z "$bless" ]] && command -v wslpath >/dev/null; then
  cargo build --release --target x86_64-pc-windows-gnu -p s2-app
  source scripts/windows-copy.sh
  windows_copy target/x86_64-pc-windows-gnu/release/s2-app.exe
  windows="$copy"
else
  windows=""
fi
export WSLENV="${WSLENV:+$WSLENV:}S2_GAME_DIR/p"
status=0
for entry in "${GOLDEN[@]}"; do
  read -r name map rest <<<"$entry"
  file="tests/golden/hash/$name.txt"
  if [[ "$map" != fixture:* && -z "${S2_GAME_DIR:-}" ]]; then
    echo "skipped $name: S2_GAME_DIR is not set"
    continue
  fi
  # shellcheck disable=SC2086
  "$linux" hash "$map" $rest --golden "$file" $bless || status=1
  if [[ -n "$windows" ]]; then
    # shellcheck disable=SC2086
    if ! diff <("$linux" hash "$map" $rest) <("$windows" hash "$map" $rest | tr -d '\r') >/dev/null; then
      echo "$name: the Windows build differs from the Linux build"
      status=1
    else
      echo "$name: Windows build same as Linux"
    fi
  fi
done
exit $status
