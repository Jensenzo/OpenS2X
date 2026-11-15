# GFX-PAT: Terrain pattern textures

Status: draft (read from the shipped settings file and the binary; no in-game screenshot compared)
Owner question(s): Q-126 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Each terrain type (pattern) of `patterns.lua` names a graphics name with
`pa_setGfxName`. The game does not load `data/map/textures/<name>.dds`
directly: it looks the name up in the pattern list of
`data/settings/terrain_static_data.xml`, which gives the texture file (or
another pattern to borrow the texture from) and a texture scale. The texture
is laid over the ground by world position, so one copy of a texture covers a
fixed area of 1 / scale world units, independent of fields and triangles.

## Rules

### GFX-PAT-1 Pattern list

- **GFX-PAT-1.1** `data/settings/terrain_static_data.xml` (shipped in the
  rc00 container, `FMT-rc00-container.md`) holds a list of `Pattern`
  entries in a `Patterns` element. Each entry has a `name` and either a
  `texture` (with an optional `normal` map) or an `alias` naming another
  pattern, plus optional `texScale` and `nrmScale` (two numbers each,
  default `1 1`), `blocking`, `type`, `transition`, `specularIntensity`,
  `specularPower`, `parallaxHeight`, `ambient`, `diffuse`, `specular` (three
  numbers, default `1 1 1`) and `groundCover`. XML comments are not entries.
  Confidence: high.
- **GFX-PAT-1.2** Entries are read in file order. An entry whose name was
  already read is skipped (the first one wins). Confidence: high.
- **GFX-PAT-1.3** A terrain type's gfx name (`pa_setGfxName`) selects the
  entry with the same name. All 73 gfx names of the shipped `patterns.lua`
  have an entry; a name without an entry gets no texture (not traced
  further). Confidence: high.

- **GFX-PAT-1.4 The table as loaded (measured 2026-10-24 night, E-heap-001: the heap dumps
  of E-trace-007 at trace ticks 1 and 6000, a memory read of the running game).** The running game holds the
  73 entries of the shipped file as one array in file order (index 0 `z_border_00`), each
  with its name, texture path, `texScale`, normal map path, `nrmScale`, `transition`,
  `type`, `specularIntensity`, `specularPower`, `parallaxHeight`, `ambient`, `diffuse`,
  `specular` and `groundCover` exactly as 1.1 reads them, the defaults included (`1 1`
  and `1 1 1` where the entry gives none, `transition` and `type` 0, `groundCover` empty;
  only the three meadows `z_meadow_00`, `_01`, `_03` have `GrassBlades`, and
  `z_pavement_00` the ambient 1.0 1.2 1.3 and diffuse 1.2 1.22 1.3). An alias entry holds the
  alias's name in its texture field and no normal map: the alias is resolved when the
  textures are made (2.2), and the entry's own scales and values stay (2.3). `blocking` is
  not held in these records. Confidence: high (every field of all 73 entries).

### GFX-PAT-2 Texture and alias

- **GFX-PAT-2.1** An entry with `texture` uses that file. The path is given
  relative to the game folder with backslashes (`data\map\textures\x.dds`).
  **The file name need not match the pattern name**: for example
  `z_sand_02` uses `z_sand_01.dds`, `l_rock_00` uses `l_ground_00.dds`,
  `z_rock_07` / `08` / `09` use `z_rock_04` / `05` / `06.dds`, the border
  uses `plates.dds` and the water `waterground.dds`. Confidence: high.
  Class rule (2026-10-24): the ground textures and normal maps of `data/map/textures/`
  are the files these entries name; the water frames are GFX-WATER-7.1's and the gate
  frames GFX-OBJ-1.2a's; `grass_normal.tga` is named by nothing (no table, model, script
  or string of the executable) and is taken as unused (medium). Covers
  (inventory): *texture* from `data/map/textures/*` except
  `data/map/textures/gate_water*`, `data/map/textures/grass_normal.tga`.
- **GFX-PAT-2.2** An entry with `alias` has no texture of its own: it draws
  with the texture (and normal map) of the entry named by the alias. Only
  entries with their own texture can be alias targets (aliases are resolved
  after all texture entries, against them; in the shipped file every alias
  names an entry with a texture, so no chains occur). Confidence: high.
- **GFX-PAT-2.3** An alias entry keeps **its own** `texScale`, `nrmScale`,
  `type`, `blocking`, `transition` and the other values; only the
  texture is borrowed. So `z_rock_01` is `z_rock_00.dds` at scale 0.062
  instead of 0.09. Confidence: high.
- **GFX-PAT-2.4** The 20 gfx names that have no `.dds` of their own
  (Q-126) are all covered by GFX-PAT-2.1 or 2.2; the table below gives the
  texture for every terrain type. Note `z_sand_01`: a file `z_sand_01.dds`
  exists, but the pattern `z_sand_01` is an alias of `z_seaground_00` and is
  drawn with `z_seaground_00.dds`. Confidence: high.

### GFX-PAT-3 Texture coordinates

- **GFX-PAT-3.1** The ground's texture coordinates are the world position in
  the map plane (`MAP-grid-geometry.md` MAP-GRID-5.2: `u` along the rows,
  `v` across them; height is ignored) multiplied by the pattern's
  `texScale`: `s = u * scale_u`, `t = v * scale_v`. The texture wraps
  (repeats). Coordinates are per vertex and continuous across fields and
  triangles: a texture is neither fitted to a field nor to a triangle.
  Confidence: high for the scale factors (set as the texture transform when
  drawing a pattern); medium for "no further offset" (the transform's
  translation is not written by that code).
- **GFX-PAT-3.2** Hence one copy of the texture spans `1 / scale_u` world
  units along a row, that is `1 / (6 * scale_u)` fields, and
  `1 / scale_v` world units across, `1 / (5.19615 * scale_v)` field rows.
  Example: meadow `z_meadow_00`, scale 0.065: 15.4 world units = 2.56 fields
  along a row, 2.96 rows. The phase follows from the world origin of
  MAP-GRID-5.2: the fine grid is centred on the world origin
  (`GFX-world-view.md` GFX-VIEW-1.1), so sample `(0, 0)` lies at
  `u = -1.5 (W - 1) / 2`, `v = -1.2990375 (H - 1) / 2`.
- **GFX-PAT-3.3** The normal map presumably uses `nrmScale` the same way
  (the shader has a second texture transform for it; its setting is not
  traced, medium). Transition masks between patterns use their own
  coordinates: GFX-PAT-5.

### GFX-PAT-4 Table

Generated by the investigation's pattern table script from the
shipped files. Fields per repeat = 1 / (6 x scale u); rows per repeat =
1 / (5.19615 x scale v).

| gfx name | texture file | via alias | texScale u, v | fields per repeat along a row | rows per repeat |
|---|---|---|---|---|---|
| `z_border_00` | `plates.dds` | - | 0.15, 0.15 | 1.11 | 1.28 |
| `z_water_00` | `waterground.dds` | - | 0.15, 0.15 | 1.11 | 1.28 |
| `z_acre_00` | `z_acre_00.dds` | - | 0.13, 0.13 | 1.28 | 1.48 |
| `z_meadow_00` | `z_meadow_00.dds` | - | 0.065, 0.065 | 2.56 | 2.96 |
| `z_meadow_01` | `z_meadow_01.dds` | - | 0.06, 0.06 | 2.78 | 3.21 |
| `z_meadow_02` | `z_meadow_02.dds` | - | 0.08, 0.08 | 2.08 | 2.41 |
| `z_meadow_03` | `z_meadow_03.dds` | - | 0.055, 0.055 | 3.03 | 3.50 |
| `z_meadow_04` | `z_meadow_04.dds` | - | 0.13, 0.13 | 1.28 | 1.48 |
| `z_meadow_05` | `z_meadow_05.dds` | - | 0.13, 0.13 | 1.28 | 1.48 |
| `z_meadow_06` | `z_meadow_06.dds` | - | 0.13, 0.13 | 1.28 | 1.48 |
| `z_meadow_07` | `z_meadow_07.dds` | - | 0.06, 0.06 | 2.78 | 3.21 |
| `z_meadow_09` | `z_meadow_09.dds` | - | 0.08, 0.08 | 2.08 | 2.41 |
| `z_sand_00` | `z_sand_00.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `z_sand_01` | `z_seaground_00.dds` | z_seaground_00 | 0.06, 0.06 | 2.78 | 3.21 |
| `z_sand_02` | `z_sand_01.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `z_sand_03` | `z_sand_01.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `z_sand_04` | `z_sand_01.dds` | z_sand_02 | 0.085, 0.085 | 1.96 | 2.26 |
| `z_sand_05` | `z_sand_01.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `z_sand_06` | `z_sand_01.dds` | z_sand_05 | 0.085, 0.085 | 1.96 | 2.26 |
| `z_earth_01` | `z_earth_01.dds` | - | 0.08, 0.08 | 2.08 | 2.41 |
| `z_rock_00` | `z_rock_00.dds` | - | 0.09, 0.09 | 1.85 | 2.14 |
| `z_rock_01` | `z_rock_00.dds` | z_rock_00 | 0.062, 0.062 | 2.69 | 3.10 |
| `z_rock_02` | `z_rock_00.dds` | z_rock_00 | 0.13, 0.13 | 1.28 | 1.48 |
| `z_rock_03` | `z_rock_03.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `z_rock_04` | `z_rock_00.dds` | z_rock_00 | 0.09, 0.16 | 1.85 | 1.20 |
| `z_rock_05` | `z_rock_00.dds` | z_rock_00 | 0.16, 0.09 | 1.04 | 2.14 |
| `z_rock_06` | `z_rock_03.dds` | z_rock_03 | 0.045, 0.045 | 3.70 | 4.28 |
| `z_rock_07` | `z_rock_04.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `z_rock_08` | `z_rock_05.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `z_rock_09` | `z_rock_06.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `z_snow_00` | `z_snow_00.dds` | - | 0.1, 0.1 | 1.67 | 1.92 |
| `z_seaground_00` | `z_seaground_00.dds` | - | 0.06, 0.06 | 2.78 | 3.21 |
| `z_seaground_01` | `z_seaground_01.dds` | - | 0.13, 0.13 | 1.28 | 1.48 |
| `z_seaground_02` | `z_sand_00.dds` | z_sand_00 | 0.085, 0.085 | 1.96 | 2.26 |
| `z_seaground_03` | `z_seaground_03.dds` | - | 0.13, 0.13 | 1.28 | 1.48 |
| `z_seaground_04` | `z_rock_00.dds` | z_rock_00 | 0.09, 0.09 | 1.85 | 2.14 |
| `z_seaground_05` | `z_rock_03.dds` | z_rock_03 | 0.09, 0.09 | 1.85 | 2.14 |
| `z_swamp_00` | `z_swamp_00.dds` | - | 0.035, 0.035 | 4.76 | 5.50 |
| `z_swamp_01` | `z_swamp_01.dds` | - | 0.07, 0.07 | 2.38 | 2.75 |
| `z_swamp_02` | `z_swamp_02.dds` | - | 0.065, 0.065 | 2.56 | 2.96 |
| `z_pavement_00` | `z_pavement_00.dds` | - | 0.15, 0.15 | 1.11 | 1.28 |
| `z_ground_00` | `z_ground_00.dds` | - | 0.08, 0.08 | 2.08 | 2.41 |
| `l_ground_00` | `l_ground_00.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `l_ground_01` | `l_ground_01.dds` | - | 0.095, 0.095 | 1.75 | 2.03 |
| `l_rock_00` | `l_ground_00.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `l_rock_01` | `l_ground_00.dds` | l_rock_00 | 0.075, 0.075 | 2.22 | 2.57 |
| `l_rock_02` | `l_ground_00.dds` | l_rock_00 | 0.085, 0.065 | 1.96 | 2.96 |
| `l_rock_03` | `l_rock_01.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `l_ground_02` | `l_ground_02.dds` | - | 0.06, 0.06 | 2.78 | 3.21 |
| `l_ground_03` | `l_ground_03.dds` | - | 0.095, 0.095 | 1.75 | 2.03 |
| `l_ground_04` | `l_ground_00.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `l_ground_05` | `l_ground_01.dds` | l_ground_01 | 0.095, 0.095 | 1.75 | 2.03 |
| `l_sand_00` | `l_sand_00.dds` | - | 0.085, 0.085 | 1.96 | 2.26 |
| `l_meadow_00` | `l_meadow_00.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `m_meadow_00` | `m_meadow_00.dds` | - | 0.08, 0.08 | 2.08 | 2.41 |
| `m_meadow_01` | `m_meadow_01.dds` | - | 0.07, 0.07 | 2.38 | 2.75 |
| `m_meadow_02` | `m_meadow_02.dds` | - | 0.07, 0.07 | 2.38 | 2.75 |
| `m_meadow_03` | `m_meadow_02.dds` | m_meadow_02 | 0.08, 0.08 | 2.08 | 2.41 |
| `m_ground_00` | `m_ground_00.dds` | - | 0.08, 0.08 | 2.08 | 2.41 |
| `m_ground_01` | `m_ground_01.dds` | - | 0.07, 0.07 | 2.38 | 2.75 |
| `m_rock_00` | `m_rock_00.dds` | - | 0.09, 0.09 | 1.85 | 2.14 |
| `m_rock_01` | `m_rock_00.dds` | m_rock_00 | 0.09, 0.09 | 1.85 | 2.14 |
| `m_rock_02` | `m_rock_00.dds` | m_rock_00 | 0.09, 0.09 | 1.85 | 2.14 |
| `m_rock_03` | `m_rock_01.dds` | - | 0.055, 0.055 | 3.03 | 3.50 |
| `m_rock_04` | `m_rock_01.dds` | m_rock_03 | 0.085, 0.085 | 1.96 | 2.26 |
| `m_rock_05` | `m_rock_01.dds` | m_rock_03 | 0.04, 0.04 | 4.17 | 4.81 |
| `m_seaground_00` | `m_rock_00.dds` | m_rock_00 | 0.09, 0.09 | 1.85 | 2.14 |
| `m_seaground_01` | `m_rock_01.dds` | m_rock_03 | 0.055, 0.055 | 3.03 | 3.50 |
| `m_rock_06` | `m_rock_09.dds` | m_rock_09 | 0.045, 0.045 | 3.70 | 4.28 |
| `m_rock_07` | `m_rock_07.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `m_rock_08` | `m_rock_08.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `m_rock_09` | `m_rock_09.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |
| `m_rock_10` | `m_rock_10.dds` | - | 0.075, 0.075 | 2.22 | 2.57 |


### GFX-PAT-5 Transitions between patterns (Q-170)

- **GFX-PAT-5.1 Priority.** Every pattern entry gets its position in the
  `Patterns` list as its priority (first entry = 0). Where a lattice triangle
  (MAP-GRID-4.5; two per field) has corners of different patterns, the pattern
  with the **lowest** priority among the corners is the base layer and every
  other corner pattern is drawn over it with a mask. Confidence: high.
- **GFX-PAT-5.2 Passes.** The terrain is drawn in two passes over the cells
  of `GFX-world-view.md` GFX-VIEW-2.1: the base pass draws, per pattern, every
  triangle whose three corners have that pattern plus every mixed triangle in
  which that pattern has the lowest priority; the transition pass draws, per
  other pattern P present at 1 or 2 corners of a mixed triangle, that triangle
  again with P's texture, depth test "equal", no depth write, alpha-blended by
  the mask. Patterns are processed in priority order within a cell.
  Confidence: high.
- **GFX-PAT-5.3 Mask and anchor.** The overlay triangle's mask coordinates
  are centred on an anchor corner: when P sits at one corner, that corner
  (the mask that is opaque at its centre); when P sits at two corners, the
  third corner (the mask that is transparent at its centre). With a = anchor
  (world x, z), s a scale and h = 0.5 for the one-corner case or 0 for the
  two-corner case: `u = ((x - a.x) x s + 1) x 0.25 + h`, `v = ((z - a.z) x s
  + 1) x 0.5`, plus half a texel. The mask texture is an atlas of two square
  cells side by side (left: two-corner mask, alpha 0 at the centre rising
  outward; right: one-corner mask, alpha 1 at the centre falling outward).
  Pixel: `alpha = mask.a`, `rgb = light x (texel + (mask.r x 2 - 1))` (the mask's
  red channel brightens or darkens the overlay), then the pattern-type effects
  as in GFX-VIEW-2.4. **The scale (answered 2026-11-08 b, Q-091; E-gfx-041):**
  s = 1 / (4 x the sample step along a row) = 1 / 6 per world unit with the shipped
  settings (`graphics.xml` Map scale 1.5), set when the terrain builder is set up; so one
  mask cell spans one field edge (6 units) around the anchor. Confidence: high.
- **GFX-PAT-5.4 Mask set.** The pattern's `transition` attribute (0, 1, 2 or 6
  in the shipped file) selects an entry of the `<Transitions>` list in
  `settings/graphics.xml` (order: transition00, transition01, transition02,
  transitionC, transitionB, transitionA, transition03; files in
  `data/map/transitions`). transition00 and transition03 carry alpha
  (DXT5); transitionA/B/C have none (DXT1, alpha 1: a hard overlay with only the
  brightness term). Which function binds the mask of a pattern pair was not
  traced; the overlay pattern's own value is assumed (medium).
- **GFX-PAT-5.5 Vertex colour.** Land vertices are white with alpha 1; only
  underwater patches (type 1) carry the depth haze colour (`GFX-water.md`).
  Confidence: high.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| texScale default | 1, 1 | | high | E-trn-005 |
| field spacing | 6.0 | world units | high | MAP-GRID-5.3 |
| mask uv | ((d x s) + 1) x (0.25, 0.5), +0.5 u for one-corner masks | | high | E-gfx-007 |
| transition list | 00, 01, 02, C, B, A, 03 | index | high | graphics.xml, E-gfx-007 |
| row spacing | 5.19615 | world units | high | MAP-GRID-5.3 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | terrain type with gfx name `z_rock_01` | texture lookup | `z_rock_00.dds`, scale 0.062 0.062 |
| T2 | `l_rock_00` | texture lookup | `l_ground_00.dds`, scale 0.085 0.085 (no `l_rock_00.dds` exists; `l_rock_00_n.dds` is not used by it) |
| T3 | `z_sand_01` | texture lookup | `z_seaground_00.dds` (not `z_sand_01.dds`), scale 0.06 |
| T4 | `z_rock_04` | texture lookup | `z_rock_00.dds`, scale 0.09 along rows, 0.16 across |
| T5 | all 73 gfx names of `patterns.lua` | texture lookup | every one resolves to an existing file in `data/map/textures` |
| T6 | meadow `z_meadow_00`, two vertices 6 world units apart along a row | texture coordinate s | differs by 0.39 |
| T7 | triangle with corners meadow (list index 3), meadow, rock (index 20) | passes | base pass: meadow; transition pass: rock over it with the one-corner mask anchored at the rock corner |
| T8 | triangle with corners rock, rock, meadow | passes | base pass: meadow (lowest index); transition pass: rock with the two-corner mask anchored at the meadow corner |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-heap-001 | a memory read of the running game | E-trace-007's heap dumps at trace ticks 1 and 6000 (one record per entry: index, name, texture, scales, normal, values, colours, ground cover) | GFX-PAT-1.4 |
| E-trn-005 | code reading | `S2DNG.exe` | reads one Pattern entry: skip duplicate name; alias or texture + normal; texScale / nrmScale default 1; material values |
| E-trn-005 | code reading | | first pass: entries with a texture; second pass: aliases matched by name to those, keeping the alias's own entry index |
| E-trn-005 | code reading | the pattern draw loop | texScale u, v written to the texture matrix diagonal; alias draws with the target's texture |
| E-trn-005 | code reading | | terrain vertex: texture coordinate 0 = world position (u, v) |
| data | settings | `data/settings/terrain_static_data.xml` (decoded) | the pattern list |
| data | shader | `data/gfx/fx_20/terrain_shader.fx` | texture coordinate 0 transformed by texture transform 0 (colour) and 2 (normal) |
| E-gfx-007 | code reading | | pattern priority, transition patches, mask coordinates, draw passes |
| data | textures | `data/map/transitions/*.dds` | mask atlases (decoded and viewed) |

## Open questions

- World origin of the map plane (MAP-GRID-5.2), which sets the phase of the
  texture repeat.
- Q-091: the transition-mask scale s (GFX-PAT-5.3) and the mask binding per pattern pair (GFX-PAT-5.4).

See also `GFX-water.md`: the render `type` of a pattern entry (type 1 =
underwater ground with depth haze and wobble) and the water surface drawn
over the sea.
