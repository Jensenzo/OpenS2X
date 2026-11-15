#!/usr/bin/env python3
"""The changelog for people outside (docs/releasing.md, "The changelog").

CHANGELOG.md is written by hand, one short section per release, newest
first: a heading `## OpenS2X X.Y.Z (YYYY-MM-DD)`, then any of the
subsections "What is new", "Fixed" and "For projects built on OpenS2X:
breaking changes". The release notes on GitHub are the release's section
with a two-line preface.

  scripts/changelog.py --check X.Y.Z [--file F]   exit 1 unless the changelog
                                                  has the release's section
  scripts/changelog.py --notes X.Y.Z [--file F]   print the release notes
"""

import argparse
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

PREFACE = (
    "OpenS2X is an open-source engine for *The Settlers II: 10th Anniversary*, written from "
    "scratch in Rust. It is not affiliated with Ubisoft. It needs your own, legally obtained "
    "copy of the game (sold on GOG; tested with build 11757) and contains no files from it.\n"
    "\n"
    "**To play on Windows:** download `OpenS2X-{version}-windows-x64.zip`, unzip it, run "
    "`OpenS2X.exe` and choose the folder of your game that contains `data`. See the "
    "[README](https://github.com/Jensenzo/OpenS2X#readme) and `PLAYING.md` in the archive."
)


def section(text, version):
    """The body of the release's section (without its heading), or None."""
    heading = re.compile(r"^## OpenS2X " + re.escape(version) + r" \(\d{4}-\d\d-\d\d\)\s*$", re.M)
    m = heading.search(text)
    if not m:
        return None
    rest = text[m.end():]
    nxt = re.search(r"^## ", rest, re.M)
    body = rest[: nxt.start()] if nxt else rest
    return body.strip("\n")


def main():
    ap = argparse.ArgumentParser()
    group = ap.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", metavar="X.Y.Z")
    group.add_argument("--notes", metavar="X.Y.Z")
    ap.add_argument("--file", default=os.path.join(ROOT, "CHANGELOG.md"))
    a = ap.parse_args()
    version = a.check or a.notes
    text = open(a.file, encoding="utf-8").read()
    body = section(text, version)
    if not body:
        print(f"{a.file} has no section '## OpenS2X {version} (YYYY-MM-DD)': write it first",
              file=sys.stderr)
        return 1
    if a.notes:
        print(PREFACE.format(version=version))
        print()
        print(body)
    return 0


if __name__ == "__main__":
    sys.exit(main())
