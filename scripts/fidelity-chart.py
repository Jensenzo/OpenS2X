#!/usr/bin/env python3
"""The README's fidelity chart from docs/fidelity.md's record.

  scripts/fidelity-chart.py [--record docs/fidelity.md] [--out docs/images/fidelity.svg]

One bar per recorded game and measure: from tick 1 to the last tick before
the first difference from the original's recording, on a track as long as
the range of ticks compared. The record
block (```fidelity) gives the first differing ticks; the compared ranges
are the recordings' own (docs/fidelity.md, "What is compared"). The SVG
has its own light or dark card (the viewer's colour scheme), so it reads
on GitHub's light and dark themes alike. scripts/release.sh
writes it again at every release.
"""

import argparse
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

# (game, measure, record keys (the earliest first difference counts), the
# last tick compared). From docs/fidelity.md, "What is compared".
ROWS = [
    ("Two players, Weak computer player", "settlers' places, buildings",
     ["dense-40000-first-differing-tick"], 40_000),
    ("", "with animation, to tick 6,000", ["dense-first-differing-tick"], 6_000),
    ("", "with animation, to tick 20,000",
     ["dense-20000-first-differing-tick"], 20_000),
    ("", "the random generator", ["generator-first-differing-tick"], 150_000),
    ("One player, no computer player", "every settler and building",
     ["one-player-first-differing-tick"], 30_000),
    ("", "the random generator", ["one-player-generator-first-differing-tick"], 30_000),
    ("Two players, Strong computer player", "every settler, with animation",
     ["hard-20000-first-differing-tick"], 20_000),
    ("", "buildings and the random generator",
     ["hard-20000-buildings-first-differing-tick",
      "hard-20000-generator-first-differing-tick"], 20_000),
    ("", "long run: settlers, buildings, generator",
     ["hard-150000-first-differing-tick", "hard-150000-generator-first-differing-tick"],
     150_000),
]


def record(path):
    text = open(path, encoding="utf-8").read()
    m = re.search(r"```fidelity\n(.*?)```", text, re.S)
    if not m:
        sys.exit(f"{path}: no ```fidelity record block")
    out = {}
    for line in m.group(1).splitlines():
        if "=" in line:
            k, v = (p.strip() for p in line.split("=", 1))
            out[k] = None if v == "none" else int(v)
    return out


def equal_ticks(rec, keys, last):
    """Ticks equal before the earliest first difference of the keys."""
    firsts = []
    for k in keys:
        if k not in rec:
            sys.exit(f"the record has no {k}")
        if rec[k] is not None:
            firsts.append(rec[k])
    return min(last, min(firsts) - 1) if firsts else last


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def render(rec):
    width, label_w, bar_x, bar_w = 800, 300, 310, 330
    top, row_h, gap = 64, 30, 14
    y = top
    parts = []
    for game, measure, keys, last in ROWS:
        if game:
            if y > top:
                y += gap
            parts.append(f'<text class="game" x="0" y="{y + 4}">{esc(game)}</text>')
            y += 20
        eq = equal_ticks(rec, keys, last)
        w = round(bar_w * eq / last, 1)
        parts.append(f'<text class="measure" x="12" y="{y + 15}">{esc(measure)}</text>')
        parts.append(f'<rect class="track" x="{bar_x}" y="{y + 4}" width="{bar_w}" height="14" rx="3"/>')
        cls = "bar all" if eq == last else "bar"
        parts.append(f'<rect class="{cls}" x="{bar_x}" y="{y + 4}" width="{w}" height="14" rx="3"/>')
        parts.append(f'<text class="value" x="{bar_x + bar_w + 10}" y="{y + 15}">{eq:,} of {last:,}</text>')
        y += row_h
    height = y + 48
    pad = 16
    note = "How long each recorded game stays equal to the original"
    sub = "Bar: tick 1 to the last tick before the first difference. Track: the range compared."
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width + 2 * pad}" height="{height + 2 * pad}" viewBox="0 0 {width + 2 * pad} {height + 2 * pad}" role="img" aria-label="Ticks equal to the original per recorded game">
<title>Ticks equal to the original per recorded game</title>
<style>
text {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; fill: #59636e; font-size: 13px; }}
.title {{ font-size: 15px; font-weight: 600; fill: #1f2328; }}
.game {{ font-weight: 600; fill: #1f2328; }}
.value {{ font-variant-numeric: tabular-nums; }}
.track {{ fill: #818b98; fill-opacity: 0.22; }}
.bar {{ fill: #0969da; }}
.bar.all {{ fill: #1a7f37; }}
.bg {{ fill: #ffffff; stroke: #d1d9e0; }}
@media (prefers-color-scheme: dark) {{
  text {{ fill: #9198a1; }}
  .title, .game {{ fill: #f0f6fc; }}
  .track {{ fill-opacity: 0.45; }}
  .bar {{ fill: #4493f8; }}
  .bar.all {{ fill: #3fb950; }}
  .bg {{ fill: #0d1117; stroke: #3d444d; }}
}}
</style>
<rect class="bg" x="0.5" y="0.5" width="{width + 2 * pad - 1}" height="{height + 2 * pad - 1}" rx="6"/>
<g transform="translate({pad},{pad})">
<text class="title" x="0" y="18">{esc(note)}</text>
<text x="0" y="40">{esc(sub)}</text>
{chr(10).join(parts)}
<text x="0" y="{height - 26}">Green: no difference in the range. The first row leaves out the settlers' animation (a recorder's lag).</text>
<text x="0" y="{height - 8}">A tick is a tenth of a second. From docs/fidelity.md's record.</text>
</g>
</svg>
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--record", default=os.path.join(ROOT, "docs", "fidelity.md"))
    ap.add_argument("--out", default=os.path.join(ROOT, "docs", "images", "fidelity.svg"))
    a = ap.parse_args()
    svg = render(record(a.record))
    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    with open(a.out, "w", encoding="utf-8") as f:
        f.write(svg)
    print(f"{os.path.relpath(a.out)}: {len(ROWS)} bars")


if __name__ == "__main__":
    main()
