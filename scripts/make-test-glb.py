#!/usr/bin/env python3
"""Write tests/models/facets.glb: the tiny test model of hook W30 (a project's
building model as binary glTF). Our own geometry, made here from numbers: a
box hut with a pyramid roof, a door on a hinge, a team-colour plate, a ground
patch whose border fades out, two empty points (anchor_smoke, anchor_door),
the clips DoorOpen, DoorClose (the door turns 100 degrees about its hinge in
0.4 s) and Working (the plate swings +-10 degrees in 2 s), one 4 x 4 palette
image and one roughness and metallic image. Every face is flat: its corners
are its own, with the face's normal, one palette texel and one occlusion value
(the vertex colour; the ground's alpha its fade).

The glTF frame is the one docs/extending.md asks for: +Y up, the door faces +Z,
origin at the building's field, ground at Y = 0.

Usage: python3 scripts/make-test-glb.py [out.glb]
"""

import json
import math
import struct
import sys
import zlib


def png(width, height, rgb_rows):
    """An 8-bit RGB PNG of rows of (r, g, b) tuples."""
    raw = b"".join(b"\0" + bytes(c for px in row for c in px) for row in rgb_rows)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


# The palette: column, row -> colour. Texel centres are the swatches.
PALETTE = {
    (0, 0): (200, 180, 140),  # wall
    (1, 0): (150, 70, 50),    # roof
    (2, 0): (110, 80, 50),    # door
    (3, 0): (255, 255, 255),  # team
    (0, 1): (120, 100, 70),   # earth
}
# Roughness (green) and metallic (blue); red unused (occlusion is in the
# vertex colours).
ORM = {(2, 0): (255, 120, 255)}  # the door's fittings: a little metal


def swatch(c, r):
    return ((c + 0.5) / 4.0, (r + 0.5) / 4.0)


def image(table, default):
    return png(4, 4, [[table.get((c, r), default) for c in range(4)] for r in range(4)])


class Mesh:
    def __init__(self):
        self.pos, self.nrm, self.uv, self.col = [], [], [], []

    def face(self, corners, uv, occlusion, alpha=1.0):
        """A flat polygon (a fan), its own corners, normal from its winding."""
        a, b, c = corners[0], corners[1], corners[2]
        u = [b[i] - a[i] for i in range(3)]
        v = [c[i] - a[i] for i in range(3)]
        n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
        length = math.sqrt(sum(x * x for x in n)) or 1.0
        n = [x / length for x in n]
        for k in range(1, len(corners) - 1):
            for p in (corners[0], corners[k], corners[k + 1]):
                self.pos.append(p)
                self.nrm.append(n)
                self.uv.append(uv)
                self.col.append((occlusion, occlusion, occlusion, alpha))


def box(m, lo, hi, uv, occlusion_low=0.7):
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    m.face([(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], uv, 0.9)  # +z
    m.face([(x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0)], uv, 0.8)  # -z
    m.face([(x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1)], uv, 0.85)  # +x
    m.face([(x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0)], uv, occlusion_low)  # -x
    m.face([(x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0)], uv, 1.0)  # top
    m.face([(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)], uv, 0.6)  # bottom


def build():
    meshes = {}
    hut = Mesh()
    box(hut, (-1.0, -0.2, -1.0), (1.0, 1.6, 1.0), swatch(0, 0))
    apex = (0.0, 2.6, 0.0)
    eaves = [(-1.2, 1.6, 1.2), (1.2, 1.6, 1.2), (1.2, 1.6, -1.2), (-1.2, 1.6, -1.2)]
    for k, occ in enumerate((1.0, 0.95, 0.9, 0.95)):
        hut.face([eaves[k], eaves[(k + 1) % 4], apex], swatch(1, 0), occ)
    meshes["hut"] = hut
    # The door: its hinge at the node's origin (x 0.5), the leaf reaching
    # 1.0 to -x, 1.4 high, standing just before the front wall (z 1.0).
    door = Mesh()
    box(door, (-1.0, 0.0, 0.0), (0.0, 1.4, 0.06), swatch(2, 0))
    meshes["door"] = door
    plate = Mesh()
    box(plate, (-0.3, -0.4, -0.02), (0.3, 0.0, 0.02), swatch(3, 0))
    meshes["teamcolour"] = plate
    # The ground patch: an opaque centre and a ring that fades out in two
    # steps, one alpha per face.
    ground = Mesh()
    rings = [(1.6, 1.0), (2.0, 0.6), (2.4, 0.2)]
    up = lambda s: [(-s, 0.0, s), (s, 0.0, s), (s, 0.0, -s), (-s, 0.0, -s)]
    ground.face(up(rings[0][0]), swatch(0, 1), 1.0, 1.0)
    for (inner, _), (outer, alpha) in zip(rings, rings[1:]):
        a, b = up(inner), up(outer)
        for k in range(4):
            ground.face([a[k], b[k], b[(k + 1) % 4], a[(k + 1) % 4]][::-1], swatch(0, 1), 1.0, alpha)
    meshes["ground"] = ground

    nodes = [
        {"name": "hut", "mesh": 0},
        {"name": "door", "mesh": 1, "translation": [0.5, 0.0, 1.0]},
        {"name": "teamcolour", "mesh": 2, "translation": [0.0, 2.0, 1.25]},
        {"name": "ground", "mesh": 3},
        {"name": "anchor_smoke", "translation": [-0.6, 2.2, -0.3]},
        {"name": "anchor_door", "translation": [0.0, 0.0, 1.0]},
    ]
    order = ["hut", "door", "teamcolour", "ground"]

    blob = bytearray()
    views, accessors = [], []

    def add(data, target=None):
        while len(blob) % 4:
            blob.append(0)
        view = {"buffer": 0, "byteOffset": len(blob), "byteLength": len(data)}
        if target:
            view["target"] = target
        blob.extend(data)
        views.append(view)
        return len(views) - 1

    def floats(rows, kind, minmax=False, normalized_bytes=False):
        n = len(rows[0])
        if normalized_bytes:
            data = bytes(max(0, min(255, round(x * 255))) for row in rows for x in row)
            acc = {"componentType": 5121, "normalized": True}
        else:
            data = struct.pack("<%df" % (len(rows) * n), *[x for row in rows for x in row])
            acc = {"componentType": 5126}
        acc.update({"bufferView": add(data, 34962), "count": len(rows), "type": kind})
        if minmax:
            acc["min"] = [min(r[i] for r in rows) for i in range(n)]
            acc["max"] = [max(r[i] for r in rows) for i in range(n)]
        accessors.append(acc)
        return len(accessors) - 1

    gltf_meshes = []
    for name in order:
        m = meshes[name]
        count = len(m.pos)
        indices = struct.pack("<%dH" % count, *range(count))
        accessors.append({"bufferView": add(indices, 34963), "componentType": 5123,
                          "count": count, "type": "SCALAR"})
        index_accessor = len(accessors) - 1
        attributes = {
            "POSITION": floats(m.pos, "VEC3", minmax=True),
            "NORMAL": floats(m.nrm, "VEC3"),
            "TEXCOORD_0": floats(m.uv, "VEC2"),
            "COLOR_0": floats(m.col, "VEC4", normalized_bytes=True),
        }
        gltf_meshes.append({"name": name, "primitives": [
            {"attributes": attributes, "indices": index_accessor, "material": 0}]})

    def scalar_times(times):
        data = struct.pack("<%df" % len(times), *times)
        accessors.append({"bufferView": add(data), "componentType": 5126, "count": len(times),
                          "type": "SCALAR", "min": [min(times)], "max": [max(times)]})
        return len(accessors) - 1

    def quat_y(degrees):
        h = math.radians(degrees) / 2
        return (0.0, math.sin(h), 0.0, math.cos(h))

    def quat_x(degrees):
        h = math.radians(degrees) / 2
        return (math.sin(h), 0.0, 0.0, math.cos(h))

    def clip(name, node, times, quats):
        t = scalar_times(times)
        q = floats(list(quats), "VEC4")
        return {"name": name, "samplers": [{"input": t, "output": q, "interpolation": "LINEAR"}],
                "channels": [{"sampler": 0, "target": {"node": node, "path": "rotation"}}]}

    keys = [i * 0.4 / 12 for i in range(13)]
    # The door swings outwards about its hinge: +100 degrees about +Y turns
    # the leaf (reaching to -x) towards +z.
    opening = [quat_y(100.0 * k / 12) for k in range(13)]
    swing_times = [i * 2.0 / 8 for i in range(9)]
    swing = [quat_x(10.0 * math.sin(2 * math.pi * i / 8)) for i in range(9)]
    animations = [
        clip("DoorOpen", 1, keys, opening),
        clip("DoorClose", 1, keys, opening[::-1]),
        clip("Working", 2, swing_times, swing),
    ]

    palette = image(PALETTE, (90, 90, 90))
    orm = image(ORM, (255, 230, 0))
    images = [
        {"name": "palette", "mimeType": "image/png", "bufferView": add(palette)},
        {"name": "palette_orm", "mimeType": "image/png", "bufferView": add(orm)},
    ]
    while len(blob) % 4:
        blob.append(0)

    gltf = {
        "asset": {"version": "2.0", "generator": "s2-reborn scripts/make-test-glb.py"},
        "scene": 0,
        "scenes": [{"name": "facets", "nodes": list(range(len(nodes)))}],
        "nodes": nodes,
        "meshes": gltf_meshes,
        "materials": [{"name": "palette", "pbrMetallicRoughness": {
            "baseColorTexture": {"index": 0}, "metallicRoughnessTexture": {"index": 1}}}],
        "textures": [{"source": 0, "sampler": 0}, {"source": 1, "sampler": 0}],
        "samplers": [{"magFilter": 9728, "minFilter": 9728}],
        "images": images,
        "animations": animations,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(blob)}],
    }
    text = json.dumps(gltf, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    body = (struct.pack("<I", len(text)) + b"JSON" + text
            + struct.pack("<I", len(blob)) + b"BIN\0" + bytes(blob))
    return b"glTF" + struct.pack("<II", 2, 12 + len(body)) + body


def build_single(mesh, palette_table, orm_table, name):
    """A file of one mesh (node `name`, at the origin) and an empty point
    `anchor_origin`, with a 4 x 4 palette and roughness and metallic image:
    the test trees of hook W30b (scripts/make-test-trees.py)."""
    blob = bytearray()
    views, accessors = [], []

    def add(data, target=None):
        while len(blob) % 4:
            blob.append(0)
        view = {"buffer": 0, "byteOffset": len(blob), "byteLength": len(data)}
        if target:
            view["target"] = target
        blob.extend(data)
        views.append(view)
        return len(views) - 1

    def floats(rows, kind, minmax=False, normalized_bytes=False):
        n = len(rows[0])
        if normalized_bytes:
            data = bytes(max(0, min(255, round(x * 255))) for row in rows for x in row)
            acc = {"componentType": 5121, "normalized": True}
        else:
            data = struct.pack("<%df" % (len(rows) * n), *[x for row in rows for x in row])
            acc = {"componentType": 5126}
        acc.update({"bufferView": add(data, 34962), "count": len(rows), "type": kind})
        if minmax:
            acc["min"] = [min(r[i] for r in rows) for i in range(n)]
            acc["max"] = [max(r[i] for r in rows) for i in range(n)]
        accessors.append(acc)
        return len(accessors) - 1

    count = len(mesh.pos)
    indices = struct.pack("<%dH" % count, *range(count))
    accessors.append({"bufferView": add(indices, 34963), "componentType": 5123,
                      "count": count, "type": "SCALAR"})
    attributes = {
        "POSITION": floats(mesh.pos, "VEC3", minmax=True),
        "NORMAL": floats(mesh.nrm, "VEC3"),
        "TEXCOORD_0": floats(mesh.uv, "VEC2"),
        "COLOR_0": floats(mesh.col, "VEC4", normalized_bytes=True),
    }
    palette = image(palette_table, (90, 90, 90))
    orm = image(orm_table, (255, 230, 0))
    images = [
        {"name": "palette", "mimeType": "image/png", "bufferView": add(palette)},
        {"name": "palette_orm", "mimeType": "image/png", "bufferView": add(orm)},
    ]
    while len(blob) % 4:
        blob.append(0)
    gltf = {
        "asset": {"version": "2.0", "generator": "s2-reborn scripts/make-test-trees.py"},
        "scene": 0,
        "scenes": [{"name": name, "nodes": [0, 1]}],
        "nodes": [{"name": name, "mesh": 0}, {"name": "anchor_origin"}],
        "meshes": [{"name": name, "primitives": [
            {"attributes": attributes, "indices": 0, "material": 0}]}],
        "materials": [{"name": "palette", "pbrMetallicRoughness": {
            "baseColorTexture": {"index": 0}, "metallicRoughnessTexture": {"index": 1}}}],
        "textures": [{"source": 0, "sampler": 0}, {"source": 1, "sampler": 0}],
        "samplers": [{"magFilter": 9728, "minFilter": 9728}],
        "images": images,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(blob)}],
    }
    text = json.dumps(gltf, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    body = (struct.pack("<I", len(text)) + b"JSON" + text
            + struct.pack("<I", len(blob)) + b"BIN\0" + bytes(blob))
    return b"glTF" + struct.pack("<II", 2, 12 + len(body)) + body


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "tests/models/facets.glb"
    data = build()
    with open(out, "wb") as f:
        f.write(data)
    print(f"{out}: {len(data)} bytes")
