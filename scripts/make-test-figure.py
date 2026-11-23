#!/usr/bin/env python3
"""Write the tiny test figure of hook W30d (a settler kind drawn as a
project's figure of rigid parts) into tests/models. Our own geometry, made
here from numbers, nothing from anyone's art:

- `figure.glb`: a box person 2.15 high, facing +Z (a nose on the head's
  front): the nodes hips (the root) > chest > head, upper_arm_r > forearm_r
  > attach_hand (an empty point at the palm), hips > thigh_l, thigh_r, and
  an empty point anchor_origin on the ground under the hips. The tunic (on
  hips and chest) is on the palette's team cell, column 4, row 4 of an
  8 x 8 palette (u, v 0.5 .. 0.625, as the project palette's `team`
  swatch); the belt, skin and nose on others. Clips: Stand (2.0 s, the
  chest sways), Walk (0.556 s, the thighs swing), WalkCarry (0.556 s, the
  thighs swing, the right arm raised to the shoulder), Pickup (0.4 s, the
  chest bows and rises). No StandCarry and no PutDown: the engine falls
  back to Stand.
- `figure_far.glb`: the same nodes, points and clips, fewer faces (no nose,
  no belt), its thigh_l node placed 0.1 lower with its mesh 0.1 higher: it
  stands where the model's does, and the engine poses it by the model's
  node of the same name.

Every face is flat: its corners are its own, with the face's normal, one
palette texel and one occlusion value (the vertex colour).

Usage: python3 scripts/make-test-figure.py [out_dir]
"""

import importlib.util
import json
import math
import os
import struct
import sys

_here = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("make_test_glb", os.path.join(_here, "make-test-glb.py"))
glb = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(glb)

SIZE = 8
PALETTE = {
    (4, 4): (255, 255, 255),  # team
    (0, 0): (200, 150, 120),  # skin
    (1, 0): (110, 80, 50),    # belt
    (2, 0): (180, 120, 90),   # nose
}
ORM = {}


def swatch(c, r):
    return ((c + 0.5) / SIZE, (r + 0.5) / SIZE)


TEAM, SKIN, BELT, NOSE = swatch(4, 4), swatch(0, 0), swatch(1, 0), swatch(2, 0)


def image(table, default):
    return glb.png(SIZE, SIZE, [[table.get((c, r), default) for c in range(SIZE)]
                                for r in range(SIZE)])


def quat_x(degrees):
    h = math.radians(degrees) / 2
    return (math.sin(h), 0.0, 0.0, math.cos(h))


def quat_z(degrees):
    h = math.radians(degrees) / 2
    return (0.0, 0.0, math.sin(h), math.cos(h))


# Node: name, parent, translation (in the parent's frame).
NODES = [
    ("hips", None, (0.0, 1.0, 0.0)),
    ("anchor_origin", "hips", (0.0, -1.0, 0.0)),
    ("chest", "hips", (0.0, 0.2, 0.0)),
    ("head", "chest", (0.0, 0.5, 0.0)),
    ("upper_arm_r", "chest", (-0.35, 0.4, 0.0)),
    ("forearm_r", "upper_arm_r", (0.0, -0.35, 0.0)),
    ("attach_hand", "forearm_r", (0.0, -0.35, 0.0)),
    ("thigh_l", "hips", (0.15, 0.0, 0.0)),
    ("thigh_r", "hips", (-0.15, 0.0, 0.0)),
]


def meshes(far):
    """Each node's mesh, in the node's own frame."""
    out = {}
    hips = glb.Mesh()
    glb.box(hips, (-0.3, -0.25, -0.18), (0.3, 0.2, 0.18), TEAM)
    if not far:
        glb.box(hips, (-0.31, 0.12, -0.19), (0.31, 0.2, 0.19), BELT)
    out["hips"] = hips
    chest = glb.Mesh()
    glb.box(chest, (-0.32, 0.0, -0.18), (0.32, 0.5, 0.18), TEAM)
    out["chest"] = chest
    head = glb.Mesh()
    glb.box(head, (-0.17, 0.0, -0.17), (0.17, 0.45, 0.17), SKIN)
    if not far:
        glb.box(head, (-0.04, 0.15, 0.17), (0.04, 0.25, 0.27), NOSE)
    out["head"] = head
    for name in ("upper_arm_r", "forearm_r"):
        arm = glb.Mesh()
        glb.box(arm, (-0.07, -0.36, -0.07), (0.07, 0.0, 0.07), SKIN)
        out[name] = arm
    for name in ("thigh_l", "thigh_r"):
        leg = glb.Mesh()
        # The far file's thigh_l node is 0.1 lower, its mesh 0.1 higher.
        lift = 0.1 if far and name == "thigh_l" else 0.0
        glb.box(leg, (-0.09, -1.0 + lift, -0.09), (0.09, lift, 0.09), SKIN)
        out[name] = leg
    return out


def build(far):
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

    parts = meshes(far)
    index = {name: i for i, (name, _, _) in enumerate(NODES)}
    nodes, gltf_meshes = [], []
    for name, parent, t in NODES:
        if far and name == "thigh_l":
            t = (t[0], t[1] - 0.1, t[2])
        node = {"name": name, "translation": list(t)}
        children = [index[n] for n, p, _ in NODES if p == name]
        if children:
            node["children"] = children
        if name in parts:
            m = parts[name]
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
            node["mesh"] = len(gltf_meshes)
            gltf_meshes.append({"name": name, "primitives": [
                {"attributes": attributes, "indices": index_accessor, "material": 0}]})
        nodes.append(node)

    def scalar_times(times):
        data = struct.pack("<%df" % len(times), *times)
        accessors.append({"bufferView": add(data), "componentType": 5126, "count": len(times),
                          "type": "SCALAR", "min": [min(times)], "max": [max(times)]})
        return len(accessors) - 1

    def clip(name, length, keys, turns):
        """`turns`: node -> function of the key's share (0 .. 1) -> quaternion."""
        times = [length * i / keys for i in range(keys + 1)]
        t = scalar_times(times)
        samplers, channels = [], []
        for node, turn in turns.items():
            q = floats([turn(i / keys) for i in range(keys + 1)], "VEC4")
            samplers.append({"input": t, "output": q, "interpolation": "LINEAR"})
            channels.append({"sampler": len(samplers) - 1,
                             "target": {"node": index[node], "path": "rotation"}})
        return {"name": name, "samplers": samplers, "channels": channels}

    swing = lambda sign: (lambda s: quat_x(sign * 25.0 * math.sin(2 * math.pi * s)))
    animations = [
        clip("Stand", 2.0, 8, {"chest": lambda s: quat_z(3.0 * math.sin(2 * math.pi * s))}),
        clip("Walk", 0.556, 8, {"thigh_l": swing(1), "thigh_r": swing(-1)}),
        clip("WalkCarry", 0.556, 8, {"thigh_l": swing(1), "thigh_r": swing(-1),
                                     "upper_arm_r": lambda s: quat_x(-150.0)}),
        clip("Pickup", 0.4, 8, {"chest": lambda s: quat_x(60.0 * math.sin(math.pi * s))}),
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
        "asset": {"version": "2.0", "generator": "s2-reborn scripts/make-test-figure.py"},
        "scene": 0,
        "scenes": [{"name": "figure", "nodes": [index["hips"]]}],
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


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "tests/models"
    for far, name in ((False, "figure.glb"), (True, "figure_far.glb")):
        path = os.path.join(out, name)
        data = build(far)
        with open(path, "wb") as f:
            f.write(data)
        print(f"{path}: {len(data)} bytes")
