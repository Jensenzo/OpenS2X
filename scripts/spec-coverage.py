#!/usr/bin/env python3
"""Spec rule coverage (docs/test-coverage.md).

Every test and every play-harness step names the spec rules it checks. This
script reads the rules each spec defines (`- **ECO-HQ-4.5** ...` in
specs/*.md) and the rules each test names, and prints the coverage per spec
file.

A test names a rule in its tag: the comment lines and attributes directly
above `#[test]`, the `fn` line, and the comment lines that open its body. A
play-harness step (crates/viewer/src/tools/play/, `step(...)` / `menu_step(...)`)
names rules in its description. Lists written the short way count in full:
"TRN-ROAD-6.1, 6.2 and 6.4" names TRN-ROAD-6.1, 6.2 and 6.4, and "GFX-ANIM T18, 4.1a" (a test case of
the spec, then a rule) names GFX-ANIM-4.1a. A test of our own tooling that
checks no spec rule says so: `// spec: none (what it checks)`. A tag naming a
section ("ECO-HQ-4") names every rule of that section.

  scripts/spec-coverage.py               the table per spec file
  scripts/spec-coverage.py --uncovered   also every rule no test names
  scripts/spec-coverage.py --untagged    also every test that names no rule
  scripts/spec-coverage.py --check       exit 1 if a test names no rule
  scripts/spec-coverage.py --markdown    the table as Markdown
"""

import glob
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
PREFIX = r"[A-Z]{2,5}(?:-[A-Z0-9]+)*"
NUMBER = r"\d+(?:\.\d+)*[a-z]?"
RULE = re.compile(r"\b(" + PREFIX + r")-(" + NUMBER + r")\b")
# Not spec rule families: hashes, sizes, Windows names and the like.
SKIP_FILES = {"OPEN_QUESTIONS.md", "INVENTORY.md", "README.md", "_TEMPLATE.md", "OBSERVED.md"}


DUPLICATES = []


def spec_rules():
    """{spec file: [rule ids in file order]}; ids a file defines twice go
    to DUPLICATES."""
    out = {}
    for path in sorted(glob.glob(os.path.join(ROOT, "specs", "*.md"))):
        name = os.path.basename(path)
        if name in SKIP_FILES:
            continue
        text = open(path, encoding="utf-8").read()
        ids = []
        for m in re.finditer(r"^\s*- \*\*(" + PREFIX + "-" + NUMBER + r")\b", text, re.M):
            if m.group(1) in ids:
                DUPLICATES.append((name, m.group(1)))
            else:
                ids.append(m.group(1))
        for m in re.finditer(r"\*\*(" + PREFIX + "-" + NUMBER + r")\b", text):
            if m.group(1) not in ids:
                ids.append(m.group(1))
        # A section heading with no numbered rule under it is a rule itself
        # (`### LUA-PROP-7 Terrain patterns`, a table of setters).
        for m in re.finditer(r"^#{2,4} (" + PREFIX + "-" + NUMBER + r")\b", text, re.M):
            section = m.group(1)
            if section not in ids and not any(i.startswith(section + ".") for i in ids):
                ids.append(section)
        if ids:
            out[name] = ids
    return out


def named_rules(text):
    """The rule ids a piece of text names, short lists expanded."""
    found = set()
    for m in RULE.finditer(text):
        prefix, number = m.group(1), m.group(2)
        found.add(prefix + "-" + number)
        # ", 6.2", " and 6.4", ", 3" right after it share the prefix.
        rest = text[m.end():]
        while True:
            more = re.match(r"(?:,\s*|\s+and\s+|\s+or\s+|\s*/\s*)(" + NUMBER + r")\b(?!-)", rest)
            if not more:
                break
            found.add(prefix + "-" + more.group(1))
            rest = rest[more.end():]
    # "GFX-ANIM T18, 4.1a": a test case, then rules of the same family.
    for m in re.finditer(r"\b(" + PREFIX + r")\s+T\d+[a-z]?((?:(?:,\s*|\s+and\s+)(?:T\d+[a-z]?|" + NUMBER + r"))*)", text):
        for number in re.findall(r"(?<![T\d.])(" + NUMBER + r")\b", m.group(2)):
            found.add(m.group(1) + "-" + number)
    return found


def rust_tests():
    """[(file, line, name, tag text)] for every #[test] in the workspace."""
    tests = []
    for path in sorted(glob.glob(os.path.join(ROOT, "crates", "**", "*.rs"), recursive=True)):
        lines = open(path, encoding="utf-8").read().split("\n")
        for i, line in enumerate(lines):
            if line.strip() != "#[test]":
                continue
            tag = []
            j = i - 1
            while j >= 0 and (lines[j].strip().startswith("//") or lines[j].strip().startswith("#[")):
                tag.append(lines[j])
                j -= 1
            k = i + 1
            while k < len(lines) and lines[k].strip().startswith("#["):
                tag.append(lines[k])
                k += 1
            if k >= len(lines):
                continue
            fn = lines[k]
            name = re.search(r"fn\s+(\w+)", fn)
            tag.append(fn)
            k += 1
            while k < len(lines) and lines[k].strip().startswith("//"):
                tag.append(lines[k])
                k += 1
            tests.append((os.path.relpath(path, ROOT), i + 1, name.group(1) if name else "?", "\n".join(tag)))
    return tests


def harness_steps():
    """[(file, line, description)] of the play harness's steps."""
    steps = []
    for path in sorted(glob.glob(os.path.join(ROOT, "crates", "viewer", "src", "tools", "play", "*.rs"))):
        text = open(path, encoding="utf-8").read()
        for m in re.finditer(r"\b(?:menu_)?step\(\s*(?:&?format!\()?\s*\"((?:[^\"\\]|\\.)*)\"", text):
            line = text.count("\n", 0, m.start()) + 1
            steps.append((os.path.relpath(path, ROOT), line, m.group(1)))
    return steps


def covers(named, rule):
    """Whether a named id (a rule or a section) covers `rule`."""
    return rule == named or rule.startswith(named + ".") or (
        # "ECO-HQ-4" names "ECO-HQ-4.1"; "GFX-ANIM-3.1" also names "3.1a".
        re.fullmatch(re.escape(named) + r"[a-z]", rule) is not None
    )


def main():
    args = set(sys.argv[1:])
    rules = spec_rules()
    tests = rust_tests()
    steps = harness_steps()
    units = [(f, l, n, named_rules(t)) for f, l, n, t in tests]
    units += [(f, l, "step: " + d, named_rules(d)) for f, l, d in steps]
    all_named = set().union(*(u[3] for u in units)) if units else set()
    # `// spec: none (why)`: a test of our own tooling, checking no rule.
    exempt = {(f, l) for f, l, _, t in tests if "spec: none" in t}
    untagged = [u for u in units if not u[3] and (u[0], u[1]) not in exempt]

    rows = []
    total_rules = total_covered = 0
    for spec, ids in rules.items():
        covered = [r for r in ids if any(covers(n, r) for n in all_named)]
        rows.append((spec, len(ids), len(covered), [r for r in ids if r not in covered]))
        total_rules += len(ids)
        total_covered += len(covered)

    def pct(a, b):
        return f"{100 * a / b:.0f} %" if b else "-"

    if "--markdown" in args:
        print("| Spec file | Rules | Named by a test | Coverage |")
        print("|---|---:|---:|---:|")
        for spec, n, c, _ in rows:
            print(f"| `{spec}` | {n} | {c} | {pct(c, n)} |")
        print(f"| **all** | **{total_rules}** | **{total_covered}** | **{pct(total_covered, total_rules)}** |")
    else:
        width = max(len(r[0]) for r in rows)
        for spec, n, c, _ in rows:
            print(f"{spec:<{width}}  {c:>4} of {n:>4}  {pct(c, n):>5}")
        print(f"{'all':<{width}}  {total_covered:>4} of {total_rules:>4}  {pct(total_covered, total_rules):>5}")
    print()
    print(
        f"{len(tests)} tests and {len(steps)} harness steps; "
        f"{len(units) - len(untagged) - len(exempt)} name at least one rule, {len(exempt)} check no spec "
        f"rule (spec: none), {len(untagged)} name nothing"
    )
    if DUPLICATES:
        print("\nrule ids a spec file defines twice:")
        for spec, rule in DUPLICATES:
            print(f"  {spec}: {rule}")
    if "--uncovered" in args:
        print("\nrules no test names:")
        for spec, _, _, missing in rows:
            if missing:
                print(f"  {spec}: {', '.join(missing)}")
    if "--untagged" in args or "--check" in args:
        if untagged:
            print("\ntests that name no rule:")
            for f, l, n, _ in untagged:
                print(f"  {f}:{l} {n}")
    if "--check" in args and untagged:
        sys.exit(1)


if __name__ == "__main__":
    main()
