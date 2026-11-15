#!/usr/bin/env bash
# The public copy of a release (docs/releasing.md, "The public copy").
#
#   scripts/public-export.sh [--draft] COMMIT [TARGET]
#
# Writes the tree a stranger gets into TARGET (default ~/s2/public/OpenS2X,
# the public repository Jensenzo/OpenS2X's folder), a git repository of its
# own, and commits it there as one commit "OpenS2X VERSION" (VERSION from the
# release tag, else from `git describe`). A later export onto the same TARGET
# replaces the tree and adds one commit, so the public repository grows by
# one commit per release. TARGET gets no remote from here; the commit's
# author is the GitHub no-reply identity most of this repository's commits
# use, never a private address.
#
# The tree is `git archive COMMIT` without what scripts/public/exclude.txt
# leaves out (with its reason per line), with its swapped files written in
# place (the public CI). Then scripts/public-check.py must find nothing:
# else nothing is committed (the tree stays for reading; the next export
# replaces it) and the script fails, unless --draft, which
# commits anyway with the number of findings in the message (a dry run's
# folder; a draft is never pushed).
#
# This script only reads this repository; it never pushes.
set -euo pipefail
cd "$(dirname "$0")/.."

draft=0
if [[ "${1:-}" == "--draft" ]]; then
  draft=1
  shift
fi
[[ $# -eq 1 || $# -eq 2 ]] || { echo "usage: scripts/public-export.sh [--draft] COMMIT [TARGET]" >&2; exit 2; }
commit="$(git rev-parse --verify --quiet "$1^{commit}")" || { echo "not a commit: $1" >&2; exit 2; }
dst="${2:-$HOME/s2/public/OpenS2X}"

list="scripts/public/exclude.txt"
git cat-file -e "$commit:$list" 2>/dev/null \
  || { echo "$1 has no $list: export a commit made after the public copy's scripts" >&2; exit 2; }

# The version: the release tag at the commit, else git describe.
if tag="$(git describe --tags --exact-match "$commit" 2>/dev/null)"; then
  version="${tag#v}"
else
  tag=""
  version="$(git describe --tags "$commit")"
  version="${version#v}"
fi

# The identity: the GitHub no-reply address most commits use.
ident="$(git log --format='%an%x09%ae' | grep -E $'\t[0-9]+\\+[^@]+@users\\.noreply\\.github\\.com$' \
  | sort | uniq -c | sort -rn | head -1 | sed -E 's/^ *[0-9]+ //')"
name="${ident%%$'\t'*}"
email="${ident#*$'\t'}"
[[ "$email" == *@users.noreply.github.com ]] || { echo "no GitHub no-reply identity in the log" >&2; exit 1; }

# What the export leaves out and swaps, from the commit's own list.
left=()
declare -A swap=()
while IFS= read -r line; do
  [[ -z "${line// }" || "$line" =~ ^[[:space:]]*# ]] && continue
  spec="${line%% | *}"
  if [[ "$spec" == *" <- "* ]]; then
    swap["${spec%% <- *}"]="${spec##* <- }"
  else
    left+=("$spec")
  fi
done < <(git show "$commit:$list")

# The target: a new repository, or one of ours with nothing uncommitted.
if [[ -e "$dst" && ! -d "$dst/.git" ]]; then
  [[ -z "$(ls -A "$dst")" ]] || { echo "$dst exists and is not a git repository" >&2; exit 1; }
fi
if [[ ! -d "$dst/.git" ]]; then
  mkdir -p "$dst"
  git -C "$dst" init -q -b main
fi
# A failed export leaves its tree for reading, marked; the next export
# replaces it. Anything else uncommitted is someone's work: refused.
pending="$dst/.git/public-export-pending"
if [[ -n "$(git -C "$dst" status --porcelain)" && ! -e "$pending" ]]; then
  echo "$dst has uncommitted changes; commit or remove them first" >&2
  exit 1
fi
echo "$commit" >"$pending"
git -C "$dst" config user.name "$name"
git -C "$dst" config user.email "$email"

# Replace the tree: remove every file but the ignored ones (a build folder
# stays; untracked files are a failed export's, checked above), then write
# the commit's tree without the left-out paths.
git -C "$dst" ls-files -z --cached --others --exclude-standard | (cd "$dst" && xargs -0 -r rm -f --)
find "$dst" -mindepth 1 -maxdepth 1 ! -name .git ! -name target -type d -exec find {} -depth -type d -empty -delete \;
pathspec=(.)
for p in "${left[@]}"; do pathspec+=(":(exclude)${p%/}"); done
git archive --format=tar "$commit" -- "${pathspec[@]}" | tar -x -C "$dst"
for target in "${!swap[@]}"; do
  source="${swap[$target]}"
  mkdir -p "$dst/$(dirname "$target")"
  git show "$commit:$source" >"$dst/$target"
  rm -f "$dst/$source"
done

# Nothing that must stay private.
status=0
python3 "$dst/scripts/public-check.py" "$dst" >"$dst/.git/public-check.txt" 2>&1 || status=$?
findings="$(grep -c '^[^ ].*:[0-9]*: ' "$dst/.git/public-check.txt" || true)"
if [[ $status -ne 0 && $draft -eq 0 ]]; then
  head -20 "$dst/.git/public-check.txt"
  echo "public check: $findings findings (all of them: $dst/.git/public-check.txt); nothing committed." >&2
  echo "The uncommitted tree in $dst is this export's, for reading; the next export replaces it." >&2
  exit 1
fi

message="OpenS2X $version"
[[ $status -ne 0 ]] && message+=" (draft: $findings findings of scripts/public-check.py, not for publishing)"
git -C "$dst" add -A -- .
if git -C "$dst" diff --cached --quiet; then
  echo "$dst already holds this tree; nothing committed"
else
  GIT_AUTHOR_DATE="$(git log -1 --format=%aI "$commit")" git -C "$dst" commit -q -m "$message"
  echo "$dst: $(git -C "$dst" log -1 --format='%h %s')"
fi
rm -f "$pending"
if [[ -n "$tag" && $status -eq 0 ]]; then
  git -C "$dst" tag -f "$tag" >/dev/null
fi
echo "files: $(git -C "$dst" ls-files | wc -l), $(git -C "$dst" ls-files -z | (cd "$dst" && xargs -0 du -cb) | tail -1 | cut -f1) bytes;" \
  "left out: ${left[*]}; swapped: $(for t in "${!swap[@]}"; do printf '%s <- %s ' "$t" "${swap[$t]}"; done)"
