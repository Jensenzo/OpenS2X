#!/usr/bin/env bash
# After a change of the snapshot format (docs/snapshot.md, "Changing the
# format"): writes tests/saves/format.txt and, when this format has none yet,
# its fixture save, which never changes afterwards. Commit both with the rule.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test -p s2-ai --lib save::tests::record_the_format -- --ignored --nocapture
git status --short -- tests/saves
