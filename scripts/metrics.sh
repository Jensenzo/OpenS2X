#!/usr/bin/env bash
# The progress numbers (docs/releasing.md): fidelity per pair, the
# inventory by status, open questions, stand-ins, unbuilt items and tests,
# written as JSON for the tracker. Run at the end of every run.
#
#   scripts/metrics.sh [OUT]    OUT defaults to the tracker's metrics.json
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-/mnt/c/Users/${USER:-$(id -un)}/Projects/s2-tracker/metrics.json}"

cargo build --release -q -p s2-app
fidelity="$(mktemp)"
trap 'rm -f "$fidelity"' EXIT
# A regression exits 1; its numbers are written all the same.
target/release/s2-app fidelity > "$fidelity" || echo "s2-app fidelity: exit $? (see the numbers)" >&2
python3 scripts/metrics.py --fidelity "$fidelity" --out "$out"
