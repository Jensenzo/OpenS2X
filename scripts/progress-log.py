#!/usr/bin/env python3
"""Write PROGRESS-LOG.md from the progress log (docs/maintaining.md).

The progress log (`S2_PROGRESS_LOG`, default the tracker's
`progress-log.md`) has one line per finished task:
`- YYYY-MM-DD | implementation | what | where | questions`. Each release in
docs/releases.txt covers the implementation lines up to its count; lines
after the last release are "Unreleased". A release's breaking changes
(docs/breaking.txt, `version | what changed and what to do`) come first in
its section. The working repository keeps the result; the public copy and
the Windows download leave it out (scripts/public/exclude.txt) and carry the
hand-written CHANGELOG.md instead (scripts/changelog.py).

  scripts/progress-log.py            write PROGRESS-LOG.md
  scripts/progress-log.py --check    exit 1 if PROGRESS-LOG.md is out of date
"""

import getpass
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
LOG = os.environ.get(
    "S2_PROGRESS_LOG",
    f"/mnt/c/Users/{os.environ.get('USER') or getpass.getuser()}/Projects/s2-tracker/progress-log.md",
)


def releases():
    out = []
    for line in open(os.path.join(ROOT, "docs", "releases.txt"), encoding="utf-8"):
        line = line.strip()
        if line and not line.startswith("#"):
            version, count = line.split()
            out.append((version, int(count)))
    return out


def breaking():
    """{version: [breaking changes]} from docs/breaking.txt."""
    out = {}
    path = os.path.join(ROOT, "docs", "breaking.txt")
    if not os.path.exists(path):
        return out
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if line and not line.startswith("#"):
            version, what = (f.strip() for f in line.split("|", 1))
            out.setdefault(version, []).append(what)
    return out


# A Windows user's folder, written as %USERPROFILE%, and the maintainer's
# private repositories without owner or address: kept from the days the log
# was the public changelog; harmless now that the export leaves it out.
USER_FOLDER = re.compile(r"\b[A-Za-z]:\\Users\\[^\\\s%$<]+")
PRIVATE_REPO = re.compile(r"(?:https?://)?(?:github\.com/)?\bJensenzo/(?!OpenS2X\b)([\w-]+)", re.I)


def entries():
    """(date, what, where) of every implementation line, in log order."""
    out = []
    for line in open(LOG, encoding="utf-8"):
        if not line.startswith("- "):
            continue
        line = PRIVATE_REPO.sub(r"\1", USER_FOLDER.sub("%USERPROFILE%", line[2:]))
        fields = [f.strip() for f in line.split("|")]
        if len(fields) >= 3 and fields[1] == "implementation":
            out.append((fields[0], fields[2], fields[3] if len(fields) > 3 else ""))
    return out


# The public name (13 Nov 2026), in the heading of every release from
# FIRST_PUBLIC on; the older ones were s2-reborn's and keep their headings.
PUBLIC_NAME = "OpenS2X"
FIRST_PUBLIC = (0, 9, 3)


def public_name(version):
    return tuple(int(p) for p in version.split(".")) >= FIRST_PUBLIC


def render():
    lines = [
        "# Progress log",
        "",
        "Generated from the progress log by `scripts/progress-log.py`; one entry",
        "per finished task, newest release first. The working repository's own",
        "record: the public copy leaves it out, and `CHANGELOG.md` is the",
        "changelog for people outside. The program is OpenS2X from 0.9.3 on; the",
        "releases before were called s2-reborn.",
        "",
    ]
    items = entries()
    notes = breaking()
    groups = []
    start = 0
    for version, count in releases():
        groups.append((version, items[start:count]))
        start = count
    if start < len(items):
        groups.append(("Unreleased", items[start:]))
    for version, group in reversed(groups):
        if not group:
            continue
        dates = f"{group[0][0]} to {group[-1][0]}" if group[0][0] != group[-1][0] else group[0][0]
        if version == "Unreleased":
            lines.append(f"## Unreleased ({dates})")
        elif public_name(version):
            lines.append(f"## {PUBLIC_NAME} {version} ({dates})")
        else:
            lines.append(f"## {version} ({dates})")
        lines.append("")
        if notes.get(version):
            lines += ["### Breaking changes", ""]
            lines += [f"- {what}" for what in notes[version]]
            lines += ["", "### Changes", ""]
        for date, what, where in reversed(group):
            where = f" ({where})" if where else ""
            lines.append(f"- {date}: {what}{where}")
        lines.append("")
    return "\n".join(lines)


def main():
    path = os.path.join(ROOT, "PROGRESS-LOG.md")
    text = render()
    if "--check" in sys.argv[1:]:
        current = open(path, encoding="utf-8").read() if os.path.exists(path) else ""
        if current != text:
            print("PROGRESS-LOG.md is out of date: run scripts/progress-log.py")
            sys.exit(1)
        return
    open(path, "w", encoding="utf-8").write(text)
    print(f"PROGRESS-LOG.md: {len(entries())} entries")


if __name__ == "__main__":
    main()
