#!/usr/bin/env python3
"""The third-party notices of the Windows game, for the dist archive.

    python3 scripts/third-party-licenses.py > THIRD-PARTY-LICENSES.txt

Every crate linked into `OpenS2X.exe` (the normal dependencies of the
`s2-viewer` package for x86_64-pc-windows-gnu, not build or dev dependencies),
with its licence expression and repository, and the licence and notice files
its package ships, from the local cargo registry (run a build first, so the
sources are there). Identical texts are printed once, after the list of the
crates that ship them. A crate that ships no licence file is listed with its
expression and repository. The vendored Lua 5.1 of `lua-src` has its notice
in `lua.h` (lua-src is a build dependency of mlua-sys that compiles it into
the program), printed from there. The licences themselves are checked by
`cargo deny check licenses` (deny.toml, docs/dependencies.md).
"""

import hashlib
import json
import pathlib
import re
import subprocess
import sys

TARGET = "x86_64-pc-windows-gnu"
ROOT_PACKAGE = "s2-viewer"
NAMES = re.compile(r"^(LICEN[CS]E|COPYING|NOTICE|COPYRIGHT|UNLICENSE|AUTHORS)", re.I)


def metadata():
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--filter-platform", TARGET],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(out)


def shipped(meta):
    """The package ids reachable from the root by normal dependencies."""
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next(p["id"] for p in meta["packages"] if p["name"] == ROOT_PACKAGE)
    seen, todo = set(), [root]
    while todo:
        pid = todo.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            if any(k["kind"] is None for k in dep["dep_kinds"]):
                todo.append(dep["pkg"])
    return seen


def lua_notice(pkg_dir):
    text = (pkg_dir / "lua-5.1.5" / "lua.h").read_text(errors="replace")
    m = re.search(r"/\*+\n(\* Copyright \(C\) 1994.*?)\*+/", text, re.S)
    if not m:
        return None
    lines = [l[2:] if l.startswith("* ") else l.lstrip("*") for l in m.group(1).splitlines()]
    return "Lua 5.1.5 (vendored by lua-src, linked through mlua)\n\n" + "\n".join(lines).strip()


def main():
    meta = metadata()
    members = set(meta["workspace_members"])
    ids = shipped(meta)
    packages = sorted(
        (p for p in meta["packages"] if p["id"] in ids and p["id"] not in members),
        key=lambda p: (p["name"], p["version"]),
    )
    texts = {}  # hash -> (text, [crate labels])
    rows = []
    without = []
    extra = []
    for p in packages:
        label = f"{p['name']} {p['version']}"
        pkg_dir = pathlib.Path(p["manifest_path"]).parent
        files = sorted(f for f in pkg_dir.iterdir() if f.is_file() and NAMES.match(f.name))
        if p.get("license_file") and (pkg_dir / p["license_file"]).is_file():
            lf = pkg_dir / p["license_file"]
            if lf not in files:
                files.append(lf)
        rows.append((label, p.get("license") or "see its licence file", p.get("repository") or ""))
        if not files:
            without.append(label)
        for f in files:
            t = f.read_text(errors="replace").strip()
            h = hashlib.sha256(t.encode()).hexdigest()
            texts.setdefault(h, (t, []))[1].append(f"{label} ({f.name})")
        if p["name"] == "mlua-sys":
            # Lua's C source is built into the program by lua-src, a build
            # dependency of mlua-sys, so it is not in the walk above.
            for q in meta["packages"]:
                if q["name"] == "lua-src":
                    n = lua_notice(pathlib.Path(q["manifest_path"]).parent)
                    if n:
                        extra.append(n)

    w = sys.stdout.write
    w("Third-party software in OpenS2X.exe\n")
    w("=====================================\n\n")
    w("OpenS2X itself is licensed under the Apache License 2.0 (LICENSE).\n")
    w("The program links the following crates, each under its own licence.\n")
    w("Their licence and notice files follow the list. symphonia (MPL-2.0):\n")
    w("its source code is at https://github.com/pdeljanov/Symphonia, the\n")
    w("licence at https://mozilla.org/MPL/2.0/; OpenS2X does not change it.\n\n")
    width = max(len(r[0]) for r in rows)
    for label, lic, repo in rows:
        w(f"  {label:<{width}}  {lic}  {repo}\n")
    if without:
        w("\nThese packages ship no licence file; their licence is the expression\n")
        w("above, its text at https://spdx.org/licenses/ and their repository:\n")
        for label in without:
            w(f"  {label}\n")
    for n in extra:
        w("\n" + "-" * 78 + "\n\n" + n + "\n")
    for t, labels in sorted(texts.values(), key=lambda v: v[1][0]):
        w("\n" + "-" * 78 + "\n\n")
        w("Shipped by: " + ", ".join(labels) + "\n\n")
        w(t + "\n")


if __name__ == "__main__":
    main()
