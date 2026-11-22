#!/usr/bin/env python3
"""Write the tiny test map objects of hook W30c (a map object drawn with a
project's own model) into tests/models. Our own geometry, made here from
numbers, nothing from anyone's art:

- `stones_1.glb`, `stones_2.glb`, `stones_3.glb`: a pile of one, two and
  three stones (eight-faced blocks, 1.2 to 1.6 across), as the stonemason's
  piles get smaller.
- `grain_tuft.glb`: a field's tuft, four thin three-sided spikes (1.1 across,
  1.8 high).
- `sign_post.glb`: a geologist's sign, a post (2.0 high) with a board.

As docs/extending.md asks of a map object: +Y up, the origin at its place on
the ground (a stone reaches 0.1 under it), one mesh, every face flat (its
corners its own, with the face's normal, one palette texel and one occlusion
value in the vertex colour), an empty point `anchor_origin`. Unlike the test
trees, the palette image is 16 x 16 (the project's map objects come on a
square palette, the trees on a 512 x 320 one: the reader takes any size).

Usage: python3 scripts/make-test-objects.py [out_dir]
"""

import importlib.util
import math
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("make_test_glb", os.path.join(_here, "make-test-glb.py"))
glb = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(glb)

SIZE = 16
# Column, row -> colour, on a 16 x 16 palette.
PALETTE = {
    (0, 0): (150, 145, 135),  # stone
    (1, 0): (120, 115, 110),  # dark stone
    (2, 0): (215, 180, 90),   # grain
    (3, 0): (120, 85, 55),    # wood
    (4, 0): (230, 225, 210),  # board
}
ORM = {}


def swatch(c, r):
    return ((c + 0.5) / SIZE, (r + 0.5) / SIZE)


def square_image(table, default):
    return glb.png(SIZE, SIZE, [[table.get((c, r), default) for c in range(SIZE)]
                                for r in range(SIZE)])


def block(m, cx, cz, w, h, d, y0, uv, turn):
    """A box turned by `turn` about +Y, its top a little smaller: 6 faces."""
    def corner(sx, sz, y, k):
        x, z = sx * w * k / 2, sz * d * k / 2
        c, s = math.cos(turn), math.sin(turn)
        return (cx + c * x - s * z, y, cz + s * x + c * z)
    lo = [corner(sx, sz, y0, 1.0) for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
    hi = [corner(sx, sz, y0 + h, 0.75) for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
    for k in range(4):
        j = (k + 1) % 4
        m.face([lo[k], hi[k], hi[j], lo[j]], uv, (0.8, 0.9)[k % 2])
    m.face(list(reversed(hi)), uv, 1.0)
    m.face(lo, uv, 0.5)


def stones(n):
    m = glb.Mesh()
    places = [(0.0, 0.0, 1.6, 1.1, 1.4, 0.2), (1.1, 0.6, 1.2, 0.8, 1.1, 0.9),
              (-0.9, 0.8, 1.3, 0.9, 1.0, 2.0)]
    for i, (x, z, w, h, d, turn) in enumerate(places[:n]):
        block(m, x, z, w, h, d, -0.1, swatch(i % 2, 0), turn)
    return m


def grain_tuft():
    m = glb.Mesh()
    for k in range(4):
        a = math.pi / 2 * k + 0.4
        x, z = 0.35 * math.cos(a), -0.35 * math.sin(a)
        base = [(x + 0.12 * math.cos(a + t), 0.0, z - 0.12 * math.sin(a + t))
                for t in (0.0, 2.1, 4.2)]
        tip = (x * 1.4, 1.8, z * 1.4)
        for i in range(3):
            m.face([base[i], base[(i + 1) % 3], tip], swatch(2, 0), (0.75, 0.9, 1.0)[i])
    return m


def sign_post():
    m = glb.Mesh()
    block(m, 0.0, 0.0, 0.12, 2.1, 0.12, -0.1, swatch(3, 0), 0.0)
    block(m, 0.0, 0.08, 0.8, 0.5, 0.05, 1.4, swatch(4, 0), 0.0)
    return m


def write(path, mesh):
    glb.image = lambda table, default: square_image(table, default)
    data = glb.build_single(mesh, PALETTE, ORM, os.path.basename(path)[:-4])
    with open(path, "wb") as f:
        f.write(data)
    print(f"{path}: {len(data)} bytes, {len(mesh.pos) // 3} triangles")


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "tests/models"
    for n in (1, 2, 3):
        write(os.path.join(out, f"stones_{n}.glb"), stones(n))
    write(os.path.join(out, "grain_tuft.glb"), grain_tuft())
    write(os.path.join(out, "sign_post.glb"), sign_post())
