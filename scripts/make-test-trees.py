#!/usr/bin/env python3
"""Write the tiny test trees of hook W30b (a tree kind drawn with a project's
own model) into tests/models: `tree_round.glb` and `tree_cone.glb`, each with
its far version (`tree_round_far.glb`, `tree_cone_far.glb`). Our own geometry,
made here from numbers, nothing from anyone's art:

- round: a six-sided trunk (0.5 across, 3 high) under a crown of two
  six-sided cups (6 across at 5 high, top at 9): 36 + 24 faces.
- cone: a four-sided trunk under three stacked six-sided cones (a fir's
  shelves), top at 10.
- each far version: the same outline with fewer faces (a three-sided trunk,
  a four-sided crown) and the same palette swatches.

As docs/extending.md asks of a tree: +Y up, the origin at the foot (the
trunk reaches 0.3 under it), one mesh, every face flat (its corners its own,
with the face's normal, one palette texel and one occlusion value in the
vertex colour), the palette image and the roughness and metallic image of
make-test-glb.py's kind embedded, an empty point `anchor_origin`.

Usage: python3 scripts/make-test-trees.py [out_dir]
"""

import importlib.util
import math
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("make_test_glb", os.path.join(_here, "make-test-glb.py"))
glb = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(glb)

# The trees' palette: column, row -> colour.
PALETTE = {
    (0, 0): (110, 85, 60),    # bark
    (1, 0): (70, 120, 55),    # leaves
    (2, 0): (45, 90, 60),     # needles
    (3, 0): (200, 195, 180),  # pale bark
}
ORM = {}


def ring(n, radius, y, turn=0.0):
    return [(radius * math.cos(turn + 2 * math.pi * k / n), y,
             -radius * math.sin(turn + 2 * math.pi * k / n)) for k in range(n)]


def prism(m, n, r0, r1, y0, y1, uv, occlusion=(0.75, 0.9)):
    """The sides of a tapering n-sided prism, wound outwards, its own faces."""
    lo, hi = ring(n, r0, y0), ring(n, r1, y1)
    for k in range(n):
        j = (k + 1) % n
        occ = occlusion[k % 2]
        m.face([lo[k], lo[j], hi[j], hi[k]], uv, occ)


def cone(m, n, radius, y0, top, uv, turn=0.0, under=0.55, sides=(0.85, 1.0)):
    """A cone of n faces from a ring at y0 to a tip, and its underside."""
    base = ring(n, radius, y0, turn)
    tip = (0.0, top, 0.0)
    for k in range(n):
        j = (k + 1) % n
        m.face([base[k], base[j], tip], uv, sides[k % 2])
    centre = (0.0, y0 + 0.4, 0.0)
    for k in range(n):
        j = (k + 1) % n
        m.face([base[j], base[k], centre], uv, under)


def cup(m, n, radius, y_rim, y_end, uv, turn=0.0, occlusion=(0.7, 0.8)):
    """A cone of n faces from a ring at y_rim to a point at y_end (below
    for the crown's underside, above for its top)."""
    rim = ring(n, radius, y_rim, turn)
    end = (0.0, y_end, 0.0)
    for k in range(n):
        j = (k + 1) % n
        corners = [rim[k], rim[j], end] if y_end > y_rim else [rim[j], rim[k], end]
        m.face(corners, uv, occlusion[k % 2])


def round_tree(far):
    m = glb.Mesh()
    sides = 3 if far else 6
    prism(m, sides, 0.25, 0.18, -0.3, 5.0, glb_swatch(0, 0))
    crown = 4 if far else 6
    cup(m, crown, 3.0, 5.0, 2.6, glb_swatch(1, 0), occlusion=(0.6, 0.7))
    cup(m, crown, 3.0, 5.0, 9.0, glb_swatch(1, 0), occlusion=(0.95, 0.85))
    return m


def cone_tree(far):
    m = glb.Mesh()
    sides = 3 if far else 4
    prism(m, sides, 0.2, 0.12, -0.3, 4.0, glb_swatch(3, 0))
    shelves = [(2.4, 2.0, 6.0), (1.9, 4.5, 8.2), (1.3, 6.8, 10.0)]
    for i, (radius, y0, top) in enumerate(shelves[:2] if far else shelves):
        if far and i == 1:
            top = 10.0
        cone(m, 4 if far else 6, radius, y0, top, glb_swatch(2, 0), turn=0.3 * i)
    return m


def glb_swatch(c, r):
    return glb.swatch(c, r)


def write(path, mesh):
    data = glb.build_single(mesh, PALETTE, ORM, os.path.basename(path)[:-4])
    with open(path, "wb") as f:
        f.write(data)
    print(f"{path}: {len(data)} bytes, {len(mesh.pos) // 3} triangles")


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "tests/models"
    for name, make in (("tree_round", round_tree), ("tree_cone", cone_tree)):
        write(os.path.join(out, f"{name}.glb"), make(False))
        write(os.path.join(out, f"{name}_far.glb"), make(True))
