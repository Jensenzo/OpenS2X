#!/usr/bin/env bash
# Make a release of OpenS2X (docs/maintaining.md, docs/releasing.md):
#
#   scripts/release.sh 0.3.0 "one line saying what the release brings"
#
# 1. checks: on main, nothing uncommitted under crates/ docs/ scripts/,
#    the version is new and larger than the last tag (semver x.y.z), and
#    CHANGELOG.md has the release's section, written by hand and committed
#    first (scripts/changelog.py --check);
# 2. sets the workspace version in Cargo.toml (and Cargo.lock);
# 3. records the release in docs/releases.txt (how many implementation lines
#    of the progress log it covers), regenerates PROGRESS-LOG.md and the
#    README's fidelity chart (docs/images/fidelity.svg);
# 4. runs cargo fmt --check, clippy and the tests;
# 5. commits those files by path and makes the annotated tag vX.Y.Z;
# 6. writes the release notes, target/dist/OpenS2X-X.Y.Z-notes.md.
#
# It does not push: check the commit, then `git push origin main vX.Y.Z`
# (the harness first, as for every push).
set -euo pipefail
cd "$(dirname "$0")/.."
version="${1:?usage: scripts/release.sh X.Y.Z \"summary\"}"
summary="${2:?usage: scripts/release.sh X.Y.Z \"summary\"}"
log="${S2_PROGRESS_LOG:-/mnt/c/Users/${USER:-$(id -un)}/Projects/s2-tracker/progress-log.md}"

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a version: $version"; exit 2; }
[[ "$(git rev-parse --abbrev-ref HEAD)" == "main" ]] || { echo "not on main"; exit 2; }
if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
  echo "v$version exists already"; exit 2
fi
last="$(git tag -l 'v*' --sort=-v:refname | head -1)"
if [[ -n "$last" ]] && [[ "$(printf '%s\n%s\n' "${last#v}" "$version" | sort -V | tail -1)" != "$version" ]]; then
  echo "v$version is not above $last"; exit 2
fi
if ! git diff --quiet HEAD -- crates docs scripts Cargo.toml Cargo.lock CHANGELOG.md; then
  echo "uncommitted changes under crates/, docs/, scripts/, Cargo files or CHANGELOG.md"; exit 2
fi
python3 -B scripts/changelog.py --check "$version" || exit 2

sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null
count="$(grep -c '| implementation |' "$log")"
echo "$version $count" >> docs/releases.txt
python3 -B scripts/progress-log.py
python3 -B scripts/fidelity-chart.py

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# RELEASE_TRAILER: an optional last line for the commit message (a
# Co-Authored-By line when a session makes the release).
git commit -q -m "Release $version: $summary${RELEASE_TRAILER:+

$RELEASE_TRAILER}" -- Cargo.toml Cargo.lock docs/releases.txt PROGRESS-LOG.md docs/images/fidelity.svg
git tag -a "v$version" -m "OpenS2X $version: $summary"
mkdir -p target/dist
python3 -B scripts/changelog.py --notes "$version" > "target/dist/OpenS2X-$version-notes.md"
echo "tagged v$version; notes in target/dist/OpenS2X-$version-notes.md; push with: git push origin main v$version"
