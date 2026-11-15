#!/usr/bin/env python3
"""The viewer uses the simulation through its public API only (docs/api.md).

Fails when program code of crates/viewer/src (tests excluded) names one of
the simulation's internal state machines or reads their fields, instead of
the view model (`s2_sim::view`). The debug hooks are allowed only in the
debug and tool modules.

  scripts/api-boundary.py        list every violation, exit 1 if any
"""

import glob
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

# What a view must not name: the simulation's state machines and their
# internals (read `s2_sim::view` instead).
INTERNAL = [
    (r"\bSettlerJob\b", "a settler's job: use view::Activity"),
    (r"\bs2_sim::Work\b|\bWork::", "a worker's step: use view::WorkPhase"),
    (r"\bCarrierTask\b", "a carrier's task: use view::SettlerView"),
    (r"\bBuilderPose\b|\bbuilder_pose\b", "a builder's pose: use view::SettlerView"),
    (r"\bs2_sim::(Builder|Leveller)\b", "site worker internals: use view::SettlerView"),
    (r"\bAttackStep\b|\bDefendStep\b|\bSpecialistStep\b", "an errand's step: use view::Activity"),
    (r"\bFinePosition\b|\bfine_position\b", "fine positions: use view::SettlerView::fine"),
    (r"\bmovement::(Walker|ONE)\b", "walker internals: use view::Step"),
    (r"\.walker\b", "a walker field: use view::Step or step()"),
    (r"\.job\b", "a settler's job field: use view::Activity"),
    (r"\.task\b", "a carrier's task field: use view::SettlerView"),
]
# Debug hooks: only in the debug mode and the tools.
DEBUG = r"\bdebug_add_ship\b|\bdebug_launch_ship\b|\bDebugSupplySite\b"
DEBUG_ALLOWED = ("tools/", "input/edit.rs", "game/mod.rs")


def program_text(path):
    """The file without its test module (`#[cfg(test)] mod tests ...` at
    the end) and without comments."""
    text = open(path, encoding="utf-8").read()
    m = re.search(r"^#\[cfg\(test\)\]\s*\n(pub(\(crate\))? )?mod tests\b", text, re.M)
    if m:
        text = text[: m.start()]
    return [re.sub(r"//.*", "", line) for line in text.split("\n")]


def main():
    src = os.path.join(ROOT, "crates", "viewer", "src")
    found = []
    for path in sorted(glob.glob(os.path.join(src, "**", "*.rs"), recursive=True)):
        rel = os.path.relpath(path, src)
        if os.path.basename(path) == "tests.rs":
            continue
        for n, line in enumerate(program_text(path), 1):
            for pattern, why in INTERNAL:
                if re.search(pattern, line):
                    found.append(f"crates/viewer/src/{rel}:{n}: {why}: {line.strip()}")
            if re.search(DEBUG, line) and not rel.startswith(DEBUG_ALLOWED):
                found.append(f"crates/viewer/src/{rel}:{n}: debug hook outside tools and debug mode: {line.strip()}")
    for f in found:
        print(f)
    if found:
        print(f"{len(found)} uses of simulation internals (docs/api.md)")
        sys.exit(1)
    print("the viewer uses the simulation's public API only")


if __name__ == "__main__":
    main()
