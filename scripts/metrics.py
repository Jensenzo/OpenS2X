#!/usr/bin/env python3
"""The progress numbers (docs/releasing.md "Progress numbers").

Reads the output of `s2-app fidelity` (stdin or --fidelity FILE) and the
repo's own lists, and writes one JSON object:

  date, commit, dirty      when and on what (dirty: uncommitted changes
                           under crates/ or tests/)
  fidelity                 per pair of `s2-app fidelity`: its first
                           difference, the settler column it leaves out
                           (`without`, besides clip_time) and its measures
                           (first differing tick per kind, matching and
                           compared ticks)
  inventory                specs/INVENTORY.md items by status: specified
                           (a spec section), open (none), verified (traced
                           or observed by Jens), traced, observed; in all and
                           per category
  questions                specs/OPEN_QUESTIONS.md by status
  stand_ins                the stand-in marks in crates/ and the questions
                           they name
  unbuilt                  the items of docs/unbuilt.md "Not built" that are
                           not struck
  tests                    tests and harness steps (scripts/spec-coverage.py)

  scripts/metrics.py --out FILE [--fidelity FILE]
  scripts/metrics.py --test        the parsers' own tests
"""

import argparse
import datetime
import glob
import json
import os
import re
import subprocess
import sys
import unittest

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

# The record keys of `s2-app fidelity` (docs/fidelity.md "The measures"),
# by the pair they measure, in the order the command prints the pairs.
PAIR_KEYS = [
    ["first-differing-tick", "settlers-first-differing-tick", "buildings-first-differing-tick"],
    ["generator-first-differing-tick", "generator-matching-ticks", "generator-compared-ticks"],
    ["one-player-first-differing-tick"],
    [
        "one-player-generator-first-differing-tick",
        "one-player-generator-matching-ticks",
        "one-player-generator-compared-ticks",
    ],
    [
        "dense-first-differing-tick",
        "dense-settlers-first-differing-tick",
        "dense-buildings-first-differing-tick",
        "dense-generator-first-differing-tick",
        "dense-generator-matching-ticks",
        "dense-generator-compared-ticks",
    ],
    [
        "dense-20000-first-differing-tick",
        "dense-20000-settlers-first-differing-tick",
        "dense-20000-buildings-first-differing-tick",
        "dense-20000-generator-first-differing-tick",
        "dense-20000-generator-matching-ticks",
        "dense-20000-generator-compared-ticks",
    ],
    [
        "dense-40000-first-differing-tick",
        "dense-40000-settlers-first-differing-tick",
        "dense-40000-buildings-first-differing-tick",
        "dense-40000-generator-first-differing-tick",
        "dense-40000-generator-matching-ticks",
        "dense-40000-generator-compared-ticks",
    ],
    [
        "hard-20000-first-differing-tick",
        "hard-20000-settlers-first-differing-tick",
        "hard-20000-buildings-first-differing-tick",
        "hard-20000-generator-first-differing-tick",
        "hard-20000-generator-matching-ticks",
        "hard-20000-generator-compared-ticks",
    ],
    [
        "hard-150000-first-differing-tick",
        "hard-150000-settlers-first-differing-tick",
        "hard-150000-buildings-first-differing-tick",
    ],
    [
        "hard-150000-generator-first-differing-tick",
        "hard-150000-generator-matching-ticks",
        "hard-150000-generator-compared-ticks",
    ],
]


def value(text):
    """A measure: a number, or None for `none` (no difference)."""
    return None if text == "none" else int(text)


def parse_fidelity(text):
    """The pairs of `s2-app fidelity`'s output, each with its first
    difference, the settler column it leaves out (`without`, None when it
    compares them all) and its measures; measures of no known pair under
    `other`."""
    pairs, measures = [], {}
    in_record = False
    for line in text.splitlines():
        if line.startswith("```fidelity"):
            in_record = True
        elif line.startswith("```"):
            in_record = False
        elif in_record and " = " in line:
            key, v = line.split(" = ", 1)
            measures[key.strip()] = value(v.strip())
        elif not line.startswith(" ") and line.endswith(":") and " against " in line:
            ours, original = line[:-1].split(" against ", 1)
            pairs.append(
                {"ours": ours, "original": original, "first": None, "without": None, "measures": {}}
            )
        elif line.startswith("  without: ") and pairs:
            pairs[-1]["without"] = line[len("  without: "):].strip()
        elif line.startswith("  ") and pairs and pairs[-1]["first"] is None:
            pairs[-1]["first"] = line.strip()
    used = set()
    for pair, keys in zip(pairs, PAIR_KEYS):
        for key in keys:
            if key in measures:
                pair["measures"][key] = measures[key]
                used.add(key)
    other = {k: v for k, v in measures.items() if k not in used}
    return {"pairs": pairs, "other": other}


def table_rows(lines):
    """The cells of a Markdown table's body rows."""
    for line in lines:
        if line.startswith("|") and not line.startswith("|---"):
            yield [c.strip() for c in line.strip().strip("|").split("|")]


def parse_inventory(text):
    """specs/INVENTORY.md's items by status, in all and per category."""
    def counts():
        return {"items": 0, "specified": 0, "open": 0, "verified": 0, "traced": 0, "observed": 0}

    total, by_category = counts(), {}
    section, header = None, None
    for line in text.splitlines():
        if line.startswith("## "):
            section, header = line[3:].strip(), None
            continue
        if section in (None, "Totals") or not line.startswith("|") or line.startswith("|---"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if cells[0] == "Name":
            header = cells
            continue
        if header is None:
            continue
        row = dict(zip(header, cells))
        c = by_category.setdefault(section, counts())
        verified = row.get("Verified", "none")
        specified = row.get("Confidence", "-") != "-"
        for k, yes in [
            ("items", True),
            ("specified", specified),
            ("open", not specified),
            ("verified", verified != "none"),
            ("traced", verified == "traced"),
            ("observed", verified.startswith("observed")),
        ]:
            if yes:
                c[k] += 1
                total[k] += 1
    return {"all": total, "by_category": by_category}


def question_status(status):
    """One word for an open question's status cell."""
    s = status.lower()
    if s.startswith("answered in part") or s.startswith("partly"):
        return "partly"
    for word in ("answered", "closed", "open", "narrowed"):
        if s.startswith(word):
            return "open" if word == "narrowed" else word
    return "other"


def parse_questions(text):
    counts = {"total": 0, "open": 0, "partly": 0, "answered": 0, "closed": 0, "other": 0}
    for cells in table_rows(text.splitlines()):
        if not re.fullmatch(r"Q-\d+", cells[0]) or len(cells) < 4:
            continue
        counts["total"] += 1
        counts[question_status(cells[3])] += 1
    return counts


def parse_unbuilt(text):
    """The items of the "Not built" section that are not struck."""
    items, inside = [], False
    for line in text.splitlines():
        if line.startswith("## "):
            inside = line.strip() == "## Not built"
            continue
        if not inside:
            continue
        for cells in table_rows([line]):
            if cells[0] in ("Item",) or cells[0].startswith("~~"):
                continue
            items.append(cells[0])
    return {"count": len(items), "items": items}


def stand_in_marks(files):
    """Lines of code that mark a stand-in, and the questions they name."""
    marks, questions, where = 0, set(), set()
    for path, text in files:
        for line in text.splitlines():
            if re.search(r"stand-in", line, re.IGNORECASE):
                marks += 1
                where.add(path)
                questions.update(re.findall(r"Q-\d+", line))
    order = sorted(questions, key=lambda q: int(q[2:]))
    return {"marks": marks, "files": len(where), "questions": order}


def parse_tests(text):
    m = re.search(r"(\d+) tests and (\d+) harness steps", text)
    if not m:
        return {"tests": None, "harness_steps": None}
    return {"tests": int(m.group(1)), "harness_steps": int(m.group(2))}


def read(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as f:
        return f.read()


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True).stdout.strip()


def collect(fidelity_text):
    code = []
    for path in sorted(glob.glob(os.path.join(ROOT, "crates/**/*.rs"), recursive=True)):
        with open(path, encoding="utf-8") as f:
            code.append((os.path.relpath(path, ROOT), f.read()))
    coverage = subprocess.run(
        [sys.executable, os.path.join(ROOT, "scripts/spec-coverage.py")],
        capture_output=True,
        text=True,
    ).stdout
    return {
        "date": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "commit": git("rev-parse", "--short", "HEAD"),
        "dirty": bool(git("status", "--porcelain", "--", "crates", "tests")),
        "fidelity": parse_fidelity(fidelity_text),
        "inventory": parse_inventory(read("specs/INVENTORY.md")),
        "questions": parse_questions(read("specs/OPEN_QUESTIONS.md")),
        "stand_ins": stand_in_marks(code),
        "unbuilt": parse_unbuilt(read("docs/unbuilt.md")),
        "tests": parse_tests(coverage),
    }


class Tests(unittest.TestCase):
    # spec: none (the progress numbers' parsers)

    def test_fidelity_pairs_and_measures(self):
        out = "\n".join([
            "ours.csv.gz against original.s2trace.gz:",
            "  tick 297: S 7bff15ce 139,49 558000,196000",
            "  first:  S,297,c0",
            "  without: animation",
            "ours-gen.csv.gz against original-generator.s2trace.gz:",
            "  the same",
            "```fidelity",
            "first-differing-tick = 297",
            "settlers-first-differing-tick = 297",
            "buildings-first-differing-tick = 873",
            "generator-first-differing-tick = none",
            "generator-matching-ticks = 559",
            "new-measure = 4",
            "```",
            "docs/fidelity.md: the same",
        ])
        got = parse_fidelity(out)
        self.assertEqual(len(got["pairs"]), 2)
        first, second = got["pairs"]
        self.assertEqual(first["ours"], "ours.csv.gz")
        self.assertEqual(first["first"], "tick 297: S 7bff15ce 139,49 558000,196000")
        self.assertEqual(first["measures"]["buildings-first-differing-tick"], 873)
        self.assertEqual(first["without"], "animation")
        self.assertEqual(second["first"], "the same")
        self.assertIsNone(second["without"])
        self.assertEqual(
            second["measures"],
            {"generator-first-differing-tick": None, "generator-matching-ticks": 559},
        )
        self.assertEqual(got["other"], {"new-measure": 4})

    def test_inventory_by_status(self):
        text = "\n".join([
            "## Totals",
            "| Category | Items |",
            "|---|---|",
            "| ware | 3 |",
            "## ware",
            "| Name | Source | Spec section | Specs | Confidence | Verified |",
            "|---|---|---|---|---|---|",
            "| wood | x | ECO-1 | 3 | high | traced |",
            "| stone | x | ECO-2 | 1 | unstated | observed by Jens |",
            "| coin | x | - | 0 | - | none |",
        ])
        got = parse_inventory(text)
        self.assertEqual(
            got["all"],
            {"items": 3, "specified": 2, "open": 1, "verified": 2, "traced": 1, "observed": 1},
        )
        self.assertEqual(got["by_category"]["ware"]["open"], 1)

    def test_questions_by_status(self):
        text = "\n".join([
            "| Q | Question | By | Status | Spec |",
            "|---|---|---|---|---|",
            "| Q-101 | a | impl | answered 2026-10-18 | x |",
            "| Q-102 | b | impl | answered in part 2026-10-23: (a) yes | x |",
            "| Q-103 | c | impl | open (not reached) | x |",
            "| Q-104 | d | impl | narrowed to one field | x |",
            "| Q-105 | e | impl | closed | x |",
            "| Q-106 | f | impl | partly answered | x |",
        ])
        self.assertEqual(
            parse_questions(text),
            {"total": 6, "open": 2, "partly": 2, "answered": 1, "closed": 1, "other": 0},
        )

    def test_unbuilt_counts_what_is_not_struck(self):
        text = "\n".join([
            "## Not built",
            "### Interface",
            "| Item | Spec | What is missing |",
            "|---|---|---|",
            "| ~~`Inventory` dialog~~ | UI-GAME-8.5 | built |",
            "| Production statistics page | UI-GAME-8.5b | not built |",
            "## Not counted as unbuilt",
            "| Item | Spec | Why |",
            "| something | x | y |",
        ])
        self.assertEqual(parse_unbuilt(text), {"count": 1, "items": ["Production statistics page"]})

    def test_stand_in_marks(self):
        files = [
            ("a.rs", "// a stand-in (Q-262 b)\nlet x = 1;\n// Stand-in: Q-340, Q-261\n"),
            ("b.rs", "fn f() {}\n"),
        ]
        self.assertEqual(
            stand_in_marks(files),
            {"marks": 2, "files": 1, "questions": ["Q-261", "Q-262", "Q-340"]},
        )

    def test_test_count(self):
        line = "1062 tests and 50 harness steps; 1024 name at least one rule"
        self.assertEqual(parse_tests(line), {"tests": 1062, "harness_steps": 50})


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--out", help="the JSON file to write")
    p.add_argument("--fidelity", help="the output of s2-app fidelity (default: stdin)")
    p.add_argument("--test", action="store_true", help="run the parsers' tests")
    args = p.parse_args()
    if args.test:
        unittest.main(argv=[sys.argv[0]])
    if not args.out:
        p.error("--out is required")
    if args.fidelity:
        with open(args.fidelity, encoding="utf-8") as f:
            fidelity = f.read()
    else:
        fidelity = sys.stdin.read()
    metrics = collect(fidelity)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(metrics, f, indent=2)
        f.write("\n")
    fid = {k: v for pair in metrics["fidelity"]["pairs"] for k, v in pair["measures"].items()}
    print(
        f"{args.out}: commit {metrics['commit']}{' (dirty)' if metrics['dirty'] else ''}, "
        f"first differing tick {fid.get('first-differing-tick')}, "
        f"inventory {metrics['inventory']['all']['specified']} of {metrics['inventory']['all']['items']} specified, "
        f"{metrics['questions']['open']} questions open, {metrics['stand_ins']['marks']} stand-in marks, "
        f"{metrics['unbuilt']['count']} unbuilt, {metrics['tests']['tests']} tests"
    )


if __name__ == "__main__":
    main()
