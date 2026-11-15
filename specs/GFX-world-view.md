# GFX-VIEW: The 3D view: camera, terrain mesh and light, object placement, fog of war, build help

Status: draft (read from the binary, the shipped settings files and the HLSL shaders; not yet checked against the running game)
Owner question(s): plan steps 6.2 and 6.3; play-test findings F10, F11, F12, F26 (Run A); Q-047 (basis conversion and scale); Q-171, Q-172, Q-173; Q-083 to Q-086, Q-093; Q-189 (a) (map edge and sky, play-test chunk 1)
Game build: 11757

## Summary

The world is drawn in a left-handed frame where the map plane is XZ and height
is Y (`MAP-grid-geometry.md` MAP-GRID-5), centred on the world origin. Granny
models are converted on load into that frame and scaled by a per-kind units
value. The camera looks at a target point on the ground from a fixed set of
zoom steps and three yaw positions, with smoothing. Terrain is lit per pixel
from one directional light defined by the day-time table (constant in the
shipped data) and darkened by a per-field fog-of-war alpha. Objects are placed
at field or fine-grid world positions, interpolated along their moves, and face
one of six headings. Build-help icons are billboards over buildable fields.

## Rules

### GFX-VIEW-1 World frame and model conversion (Q-047)

- **GFX-VIEW-1.1 Frame.** World x runs along a field row, z across rows, y is
  up (MAP-GRID-5.2 u = x, v = z). The fine (height) grid is centred: sample
  `(c, r)` lies at `x = 1.5 (c + h) - 1.5 (W - 1) / 2`, `z = 1.2990375 r -
  1.2990375 (H - 1) / 2` with `W x H` the fine grid size and `h = 0.5` on odd
  rows; this fixes the "world origin" left open in MAP-GRID-5.2 and the
  pattern phase of `GFX-terrain-patterns.md` GFX-PAT-3.2. Confidence: high.
- **GFX-VIEW-1.2 Granny basis conversion.** Every `.gr2` is converted on load
  with Granny's basis conversion from the file's own basis (art tool info) to:
  origin (0, 0, 0), right = (0, 0, 1), up = (0, 1, 0), back = (1, 0, 0), and
  "units per metre" = a per-kind scale value (1.3). The file is transformed in
  place with the resulting affine and linear transforms, renormalising normals
  and reordering triangle indices (Granny flags 1 | 2). So a model's "right"
  points along world +z and its "back" (toward the viewer in the tool) along
  world +x; the frame is left-handed, as Direct3D's. Confidence: high
  (arguments read from the call).
- **GFX-VIEW-1.3 Scale.** The units-per-metre value is the kind's `scale`
  setting: `character/settler_config.xml` Characters scale 1.2 for settlers,
  `animals.xml` 1.5 (deer, elk) and 2.0 (rabbit), `ships.xml` 1.0, and 1.0
  for kinds without a setting (items, buildings). Granny scales the model by
  (desired units per metre) / (file units per metre), so with files authored
  at 1 unit per metre a settler is drawn 1.2 times its file size.
  Confidence: medium (one call site passes the literal 1.0, the others pass
  the config value through parameters that were not followed to the end).
- **GFX-VIEW-1.4 Ground height at a world point.** The nearest fine sample P
  is found; the lattice triangle containing the point is one of the six
  (P, n_i, n_{i+1}) fans with the hex neighbours of MAP-GRID-1.2 (even rows
  (1,0), (0,-1), (-1,-1), (-1,0), (-1,1), (0,1); odd rows (1,0), (1,-1), (0,-1),
  (-1,0), (0,1), (1,1)), chosen by two cross-product sign tests; the height is
  that triangle's plane evaluated at the point. Used for the camera and for
  objects between fields. Confidence: high.
- **GFX-VIEW-1.5 Picking (play-test 2, chunk 4).** The mouse ray (unprojected
  through the pixel, length 1000) is tested against the terrain's lattice
  triangles: first against the bounding box of every visible terrain sector,
  then against the two triangles of every cell of the sectors it hits (even
  rows (x,y)(x+1,y+1)(x,y+1) and (x,y)(x+1,y)(x+1,y+1); odd rows (x,y)(x+1,y)
  (x,y+1) and (x+1,y)(x+1,y+1)(x,y+1)). The first triangle hit gives the hit
  point; the picked field is the vertex of that triangle nearest to the hit
  point. No hit (off the map) picks nothing. There is no height sampling in
  steps and no depth read-back. Confidence: high (E-pt2-001).

### GFX-VIEW-2 Terrain mesh and lighting

- **GFX-VIEW-2.1 Cells.** The terrain is cut into cells of 4 x 4 fields
  (ceil(width / 4) x ceil(height / 4) cells; the last row and column are
  clipped to the map). Each cell is drawn per pattern; the vertices are the
  fine samples with position (1.1), normal, tangent, a colour (white with
  alpha 1 on land; the depth haze on underwater patches, `GFX-water.md`),
  pattern texture coordinates (GFX-PAT-3), transition-mask coordinates
  (GFX-PAT-5) and the world-texture coordinate (2.4). The triangle layout is
  MAP-GRID-4.5; mixed triangles are drawn per pattern as in GFX-PAT-5.
  Confidence: high (cell size and vertex layout from the shader), medium
  (index order, shared edges).
- **GFX-VIEW-2.1a The terrain vertex's normal and tangent (2026-11-09, Q-1035, E-gfx-043, terrain_shader.fx).** Every fine sample has a terrain record of its
  own beside the grid normals of 2.2: its world position, a normal and a tangent, each
  as three bytes. The terrain vertices take these, decoded as b / 127.5 - 1, not
  renormalised:
  (a) **normal** = the sample's grid-normal bytes of 2.2, copied as they are (so it is
  the quantised normal, not the float one; straight up decodes to (-0.0039, 1.0,
  -0.0039));
  (b) **tangent** = the game's +x axis made perpendicular to that *decoded* normal n:
  t = normalize(n x (x x n)) with x = (1, 0, 0), i.e. the direction of x - (n . x) n /
  |n|^2. It is stored as bytes trunc((t + 1) x 127.5) and decoded the same way;
  (c) **bitangent**: not stored. The shader makes it as **B = T x N** (tangent cross
  normal, in that order) and turns the light into the frame whose rows are (T, B, N):
  L_t = (T . L, B . L, N . L) with L = -(light direction); the pixel normalises L_t and
  lights the normal-map normal with it (2.4). An implementation that uses N x T flips
  the normal map's second axis.
  Confidence: high (the record writer, the vertex builder and the shader read).
- **GFX-VIEW-2.1b When the terrain's normals are refreshed (2026-11-09, Q-1035, E-gfx-043).**
  (a) When the map view is first set up (the first frame of a game started or loaded),
  every sample's position, normal and tangent (2.1a) is made from the heights at that
  moment.
  (b) After a height change (a levelling step, a road's evening; the change's box
  widened by 16 samples, GFX-VIEW-2.4's note): the positions of the samples in the box
  are moved to the new heights, the grid normals (2.2) are recomputed there (they serve
  objects and the camera), and then the terrain's normals and tangents are copied again
  from the grid normals **only for the samples with column 0 .. W and row 0 .. H, both
  inclusive, where W x H is the map's size in fields** (not in samples: the fine grid has
  4 W + 1 columns). This is the original's behaviour as shipped: the refresh is handed
  the field size where the sample size is meant. Every other sample keeps the normal and
  tangent it had at (a) while its position follows the new height.
  (c) So on most of the map a levelled building site, its ring and an evened road are
  drawn at their new heights but lit with the slope of the game's start (or of the
  save's load). Inside a building's ground decal this does not show (the decal covers
  the terrain); on the ring around it, it does: this is the farm's darker band of 2.4's
  note (T39).
  (d) A full rebuild (a) happens again only when the map view is set up again (the next
  game or load).
  Confidence: high (the call chain and both sizes read; the field size is the map's
  method that the same refresh uses to clamp field coordinates), medium (that no other
  path resets the set-up flag during a game).
- **GFX-VIEW-2.2 Grid normals.** A per-sample normal map of the fine grid is
  kept for object lighting and the camera: n = normalize(-(h(c+1, r) - h(c-1,
  r)) x 2 sz, 2 sx x 2 sz, -(h(c, r+1) - h(c, r-1)) x 2 sx) with heights in
  world units, sx = 1.5, sz = 1.2990375, neighbours clamped at the map edge,
  taken by grid index (the odd-row half-step is ignored); stored as three
  unsigned bytes b = trunc((n + 1) x 127.5) (0 .. 255; straight up is (127,
  255, 127)), read back as b / 127.5 - 1 (= b x 2 / 255 - 1) without
  renormalising. Confidence: high (formula and storage; corrected 2026-11-09,
  Q-1035, E-gfx-043: the earlier "signed bytes trunc(n x 127.5)" was
  wrong).
- **GFX-VIEW-2.3 Light.** One directional light from `settings/daytimes.xml`:
  entries by hour with background colour, light elevation and heading
  (degrees), ambient and light colour, shadow intensity and fog near/far. The
  environment clock advances by frame time x 0.033 (graphics.xml Enviroment
  timescale; start hour 12); the two entries around the current hour are
  interpolated linearly (heading by the shorter way). Light direction (from
  the sun down) = (sin heading cos |elevation|, -sin |elevation|, cos heading cos
  |elevation|), the size of the elevation even when it is negative (2.3d b);
  shaders use its negation as the light vector. (Correction 2026-10-24: the
  game reads this XML only when `daytimes.bin` is missing, and the shipped `.bin` is a day
  cycle whose noon heading is 50, see 2.3b.) Shipped XML file: all
  entries identical, so the light is constant: elevation 50, heading 55 ->
  direction (0.5265, -0.7660, 0.3687); ambient (0.6, 0.5, 0.6), light colour
  (0.9375, 0.898, 0.781), shadow intensity 0.9, fog 100 .. 200 world units,
  background (0.8, 0.9, 1.0). No shader reads the fog settings: there is no
  distance fog. Confidence: high.
  **Shadow factor (Run A, 2026-10-17, E-gfx-011).** The `shadowIntensity`
  value of the table is not the factor a shadowed pixel keeps. Each frame the
  game takes k = clamp((|elevation| - 20 deg) / 20 deg, 0, 1) (shadows fade
  in as the sun climbs from 20 to 40 degrees) and hands the shaders
  `shadow = 1 - k x shadowIntensity`, clamped to 0..1. The shipped table
  (elevation 50, intensity 0.9) gives k = 1 and **shadow = 0.1**: a pixel in
  shadow keeps a tenth of the direct light, `D = ambient + 0.1 x light x
  N.L`. Lit flat ground has D = (1.3182, 1.1879, 1.1983) and shadowed flat
  ground (0.6718, 0.5688, 0.6598), about half: the strong, dark shadows of
  the photos (`OBSERVED.md` OBS-21). D above 1 is not scaled down: the
  product D x texel is clamped to 1 only at the output, so lit surfaces show
  their texture about 1.2 to 1.3 times brighter, which is the brightness of
  the original (F10). Ambient and light colour each carry a fourth value,
  their luminance 0.299 r + 0.587 g + 0.114 b (ambient 0.5413, light
  0.8965), which the settler shader uses (`GFX-objects.md` GFX-OBJ-3.3).
  There is no gamma or brightness control, and the post effects of
  `graphics.xml` (bloom values, a depth-of-field blur) run only when the game
  is started with the command-line switch `-postFX`; a normal game draws
  without them. Confidence: high.
  Covers (inventory): *lighting preset* from `settings/daytimes.xml`.
- **GFX-VIEW-2.3b Which day table is used, and its binary form (2026-10-24, Q-010;
  E-sim-007).** When a map's world is made, the game
  first looks for `data/settings/<map name>.bin`; if that file exists, its table is used.
  Otherwise it uses `data/settings/daytimes.bin` if that file exists, and only if it does
  not, it reads `daytimes.xml` (2.3) and writes the table to `daytimes.bin`. **The
  shipped `daytimes.bin` exists, so the shipped `daytimes.xml` is not read**; the XML
  reader also asks for an attribute `lighteeading` (sic), so a table made from the XML
  has no heading. A `.bin` table is little-endian: an `i32` count (9 in every shipped
  file), then per entry 76 bytes: `i32` hour, background (4 `f32`, RGBA), `i32` skydome
  flag, `i32` light elevation and `i32` light heading (whole degrees), ambient (4 `f32`),
  light colour (4 `f32`), shadow intensity (`f32`), fog near and far (2 `f32`); 4 + 9 x
  76 = 688 bytes. The shipped `daytimes.bin` is a day cycle: hours 0, 1, 2, 3, 12, 20,
  21, 22, 23 with elevations -28, 20, 40, 50, 50, 50, 40, 15, -28 and headings 3, 20, 40,
  50, 50, 50, 40, 24, 3; hours 12 and 20 are the values of 2.3 except the heading (50,
  not 55); hours 21 .. 3 are evening, night (skydome on, blue ambient (0.2196, 0.3490,
  0.5882), light (0.2, 0.2275, 0.6)) and morning. Per-map tables: `campaign08`,
  `campaign10`, `MP_4P_Skull_Island`, `MP_4P_black_hole`, `sp04_lavaland`,
  `sp08_goldfever`, `sp10_rat_race` (night skies, elevation -36), `sp06_desert_eve`
  (elevation 38), `X_DesertDemoEvening`, `X_LavaDemo`; each holds the same entry for all
  nine hours, so their light is constant. `noDayCycle.bin` (nine identical noon entries)
  is named by nothing in the executable. Confidence: high for the order of the files and
  the layout (code and all twelve files); medium for the name being the map's file name
  and for the name being the map's file name. What the running game shows is measured
  in 2.3c. Covers (inventory): *lighting preset* from `data/settings/*.bin`.
- **GFX-VIEW-2.3c The day clock in a running free game (measured, E-trace-007,
  2026-10-24; Q-350).** MP_2P_Storm_Coast has no table of its own; the table the game
  holds equals the shipped `daytimes.bin` byte for byte (all 688 bytes). With the
  `DayNightCycle` option at its default (1, on) the environment clock starts at hour 12
  at the first tick and advances every frame by the frame's game time x 0.033 hours per
  second: 0.0033 hours per tick, one table day of 24 hours in about 727 s of game time
  (7270 ticks); after 6000 ticks it reads 31.81. The clock is not wrapped; the table is
  read at the clock mod 24, between the two entries around that hour, with the blend
  (hour - h_i) / (h_next - h_i) (so from 12 to 20 the values are constant noon, heading
  50, not 2.3's 55); the skydome flag is not blended, it is the earlier entry's (on from
  hour 23.0 to 1.0). In the recorded game: noon light (elevation 50, heading 50) to trace
  tick about 2420 (hour 20), evening, night (skydome on, elevation -28) from trace tick
  3332 (hour 23.00) to 3937 (hour 25.00), morning, noon again from about 4545 (hour 27).
  Confidence: high (clock, rate, table, lookup: every tick of 6000 recorded, 0 read
  errors); medium for the option off (from the code: the clock is set back to the start
  hour every frame; not recorded).
- **GFX-VIEW-2.3d The light as the shaders receive it (F53, 2026-10-31; E-gfx-036).** The
  whole chain from the day table to the pixel, checked end to end:
  (a) **Blend.** Every value of the two entries around the hour is blended linearly with
  the factor of 2.3c: background, ambient and light colour (all four parts), elevation,
  heading (the shorter way), fog near and far, **and the shadow intensity** (0.5 at hours
  23 and 0, 0.9 at every other hour of the shipped table). Elevation and heading are
  whole degrees in the table and blended as real numbers, without rounding. Only the
  skydome flag is not blended.
  (b) **Direction.** The light travels along (sin h cos|e|, -sin|e|, cos h cos|e|) with h
  the blended heading and e the blended elevation: a negative elevation (night, the
  hours between 22 and 1) is used by its size, so the light always comes from above.
  (c) **Constants handed over unchanged.** The ambient and light colours go to the shaders
  as blended (alpha = their luminance, 2.3). Nothing scales, clamps or gamma-corrects
  them on the shader path. Only a device without pixel shaders 2.0 halves the light and
  drops the ambient, for its fixed-function lighting.
  (d) **Shadow factor.** s = 1 - k x intensity, clamped to 0..1, with k = clamp((|e| - 20
  deg) / 20 deg, 0, 1). For |e| below 20 deg k is 0 and s is 1: **a shadowed pixel is as
  bright as a lit one, so no shadow shows**. At hour 12 s = 0.1. At hour 0 exactly
  (|e| 28, intensity 0.5) k = 0.4 and s = 0.8, so the deep night has faint shadows.
  Between about hour 0.17 and 1.0 of the morning, and from hour 22 until the evening
  blend passes |e| 20 again, there are none. The shadow renderer always takes shadow.fx's
  `Default` technique (depth in a colour target). Its hardware-shadow flag is cleared in
  the renderer's constructor just before the choice. The receivers have no
  hardware-shadow path, so every receiver tests as 2.5 (c) and keeps 1 or s.
  (e) **Hour 12 of the shipped table** (any clock from 12 to 20, mod 24): ambient (0.6,
  0.5, 0.6), light (0.9375, 0.898, 0.781), elevation 50, heading 50, direction
  (0.4924, -0.7660, 0.4132), s = 0.1. Flat ground: N . L = 0.7660, D lit (1.3182, 1.1879,
  1.1983), D shadowed (0.6718, 0.5688, 0.6598) (T14).
  (f) **Material terms per surface.**
  - Terrain: D x texel, with the transition term of GFX-PAT-5.3. The vertex colour
    multiplies every pattern except type 1, and it is white (1, 1, 1, 1) on every
    vertex of a pattern that is not type 1: **the haze colour of GFX-WATER-2.2 is
    computed only for type-1 (sea-ground) patches**, so land patterns are never tinted,
    under water or not. Type 3 adds a specular term (2.4a). The pattern table's
    `ambient` and `diffuse` (z_pavement_00's 1.0 1.2 1.3 and 1.2 1.22 1.3) reach the
    shader but **no terrain shader reads them**, so they change nothing.
  - Houses: GFX-OBJ-2.1 with N . L per vertex: D = ambient + saturate(N . L) x light
    x s, then D x texel x colour-variation texel, then the light map weighted by
    saturate(1.5 - D) per channel.
  - Trees: GFX-OBJ-2.4. The mesh's vertex colour multiplies **only the direct term**.
  - Settlers: GFX-OBJ-3.3 (luminances).
  (g) **Output.** No gamma, no tone mapping, no sRGB: 2.6.
  (h) **The pause layer.** Every stored pixel becomes round(v x 90 / 255) (alpha byte
  165; `UI-ingame.md` UI-GAME-1.11). Pictures of the paused original are that, exactly.
  **What differs from GFX-VIEW-2.3c as built** (the implementation's measurement of the
  12 pairs of look-compare 2026-10-30, 2026-10-31 night, ours re-layered on stored
  values):
  - At noon, lit ground (dirt, grass, sand, rock, path) is equal within 1.5 % per
    channel, so 2.3c and (a) to (e) are built right.
  - The pause layer was built in linear light (a pixel kept about 0.6 instead of 0.353).
  - Shadowed ground under trees is 17-20 % darker in ours. The factor is the same 0.1,
    so the difference is coverage: the original's leaf casters are cut at alpha 0.5 and
    let light through (2.5 a, c).
  - The night (hour 0.28) is darker and uneven in ours: about 1.3 on lit ground,
    1.04 on open grass. Ours built the direction from the signed elevation, so its
    light came from below the ground and lit ground got the ambient only; (b) is the
    rule. Ours' shadow factor was already right (s = 1 there, (d)).
  - Conifers are near black in ours: the vertex colour must not multiply the ambient
    (GFX-OBJ-2.4).
  - The type-3 specular term (2.4a) is not built.
  - Open: the HQ's dome and paved square are 5-9 % darker in ours, with no code
    difference found. Check the colour-variation column (`colorMapOffset`) and the light
    map's texture coordinates first.
  Confidence: high for (a) to (h) (code; the hour-12 constants and the land's white
  vertex colour also from E-trace-007's table and the patch rule); the
  measured differences are the implementation's (2026-10-31 night).
- **GFX-VIEW-2.4 Terrain pixel.** The pattern's normal map (tangent space,
  tangent/normal from the vertex) gives N; D = lightColour x saturate(N . L)
  x shadow + ambient; colour = D x pattern texel (blended with the transition
  mask where two patterns meet), then the pattern-type effects
  (`GFX-water.md` for type 1), and finally rgb x= WorldTex.a (fog, GFX-VIEW-5).
  Shadows: a shadow map with 4 taps, scale 1.0, bias -0.00325 (graphics.xml);
  the factor is 2.3's `shadow` (0.1 in the shipped data), details in 2.5.
  Confidence: high (shader source).
  **Measured (2026-11-08 b, look-compare-auto/2026-11-08 weakfogoff-006000-farm):** the original's
  bare ground in a band 3 to 4 units wide along the farm site's decal edge on the west (the levelled
  site's ring of ECO-HQ-4.8) is 0.76 to 0.88 of ours; the decal and the ground further out agree
  (1.00). It is the ground's own light, not a decal or a shadow (the band stops at the decal, which
  takes the shadow map). The original's heights there are those of ECO-HQ-4.8 and the guard house
  road's evening (T-level-10, Q-1030: equal in ours and both recordings; the evening alone
  changes the light there by 1 to 3 %, too little for the band); a
  height change rebuilds the terrain cells over the changed box widened by 16 samples and then
  recomputes the grid normals of 2.2 there (E-gfx-041). An
  implementation that differs there should first compare its heights with T-level-10.
  **Cause (2026-11-09, Q-1035):** the terrain there is lit with the normals of the game's
  start, not of the levelled heights (2.1b): the samples' columns (about 537 .. 546) lie
  beyond the refreshed corner (0 .. 170 on Storm Coast). With the start's normals the lit
  ground of the ring comes to 0.65 .. 0.92 of the current normals' (T39), as measured.
  **The look comparison's farm "dirt" box is not this band, nor dirt (2026-11-11, inv-2026-11-11
  item 4, E-gfx-050; Q-1030).** The box (1450, 600, 120 x 80 of the 2560 x 1440 picture) is fixed on
  the screen, but the three farm pictures have different cameras (eye (303.7, 27.7, -170.1) at 1500,
  (307.6, 36.1, -172.0) at 6000, (303.7, 35.0, -172.0) at 11 000; the camera's ground height settles
  differently on the way there), so it covers different things: at 1500 the bare levelled ground
  (original / program within 3 %), at 6000 a cluster of map doodads on the meadow east of the site
  (grass 0 .. 3, plants 5 .. 7, greenery 1, 15, 16, a bush; sample columns 537 .. 560, rows 233 ..
  248), and at 11 000 the finished farm building's arch. At 6000 the ground beside the doodads
  agrees within 1 to 3 % in three boxes (the dirt road 1260 .. 1380 x 700 .. 800, the meadow 1270
  .. 1390 x 460 .. 520 and 1300 .. 1400 x 560 .. 600), and the same doodads are in both pictures
  with other sizes and turns: GFX-OBJ-1.4's random yaw and scale, drawn in the making order
  (`GFX-objects.md` T32, not yet followed by the program). So the box's 19 to 34 % at 6000 measures
  those random draws, and its night value a building at night; **no ground rule is shown to
  differ.** A ground box has to be chosen per picture, or as a world rectangle projected with each
  picture's own camera.
- **GFX-VIEW-2.4b The normal map's N (2026-11-09 c, inv-2026-11-09c item 1, E-gfx-046;
  `terrain_shader.fx` TerrainPixelShader, `shared.inc` sampler 2).** The pattern's normal
  map is used **at its own size, with its own mip levels** (the DDS file's chain), sampled
  trilinearly (linear magnification, minification and mip filter) with wrap addressing, and
  `N = 2 x texel - 1` is used **as it is: not normalised**, neither per texel nor after the
  filter. Only the light vector of 2.1a (c) is normalised per pixel. `saturate(N . L)` is
  then taken of that unnormalised N. Two consequences an implementation must keep:
  (a) between texels the filter shortens N where neighbouring texels point differently, so
  the light falls off there; normalising brings N back to length 1 and raises N . L;
  (b) a smaller copy of the map (a resample to a common size) is a box-filtered map with
  shorter normals; normalised, it raises N . L further. Neither happens in the original.
  The effect depends on the map: a smooth map (most land maps) changes by under 1 %, a
  rough one by much more. Under the noon light of `look-compare-auto` cell63 (direction
  (0.492, -0.766, 0.413), flat ground), mean saturate(N . L) over the map (T41):
  `z_seaground_00_n` (512 x 512, 4:4:4:4) 0.567 as the original uses it, 0.629 if the
  filtered N is normalised, 0.682 if the map is first resampled to 256 x 256 and then
  normalised: D (2.4) **9.5 % brighter** (9.5, 10.2, 8.6 % in red, green, blue); `l_rock_00_n`
  (1024) 3 %; `l_ground_02_n` (1024) 1.5 %; `z_rock_00_n` (512) 1 %; the 256 x 256 maps
  (`z_meadow_00_n`, `z_sand_00_n`) unchanged. Under the night light of hour 23 (direction
  (0.129, -0.255, 0.958)) the same three readings of `z_seaground_00_n` give 0.232, 0.258,
  0.231: no difference at night. **Measured:** in the look comparison of 2026-11-09 c (the
  program at 283f182 resamples every normal map to 256 x 256 and normalises N), the pixels
  over sea-ground fields (pattern 0xBABEB00B, z_seaground_00) where the ground lies between
  -0.05 and 0 (no haze, no surface) are 7 % brighter than the original's by day in all three
  channels (weakfogoff-001500-cell63: original / ours 0.932 0.925 0.936; 6000: 0.927 0.921
  0.931) and the same at night (11000: shallow sea ground within 1 %). The sand of the
  same pictures (a 256 map) agrees within 0.5 %. Confidence: high (shader source and
  sampler states; the files' sizes and formats); medium for the texture quality setting's
  effect (the pictures' VeryHigh preset is taken to load every map at full size).
- **GFX-VIEW-2.4a Specular ground (pattern type 3; F53, 2026-10-31, E-gfx-036,
  `terrain_shader.fx` passes BaseLayerSpecular / TransitionSpecular).** A type-3 pattern's
  pixel adds a white highlight after the vertex colour and before the fog of war:
  rgb += a_n x I x S, where
  - a_n is the alpha of the pattern's normal-map texel (a mask in the texture);
  - I = specularIntensity x the first part of `specular`. It is carried in the vertex
    colour's alpha, so it is clamped to 0..1;
  - S = saturate(R . V) ^ specularPower;
  - V is the direction from the point to the camera;
  - R = normalize(2 x D x N - L), taken **as written, per channel**: D is the light
    factor vector of 2.4 (ambient included, shadow applied), N the normal-map normal and
    L the light vector. In tangent space this is (2 D_r N_x - L_x, 2 D_g N_y - L_y,
    2 D_b N_z - L_z), normalised.
  The values are the pattern's, as loaded (GFX-PAT-1.4). A pattern that gives no
  `specularIntensity` has 0 there (and `specularPower` 1), so it adds nothing. With the
  shipped table only these type-3 patterns shine: z_rock_00 (0.1, 4), z_snow_00 (0.3, 5),
  z_pavement_00 (0.6, 5), l_ground_00 (0.5, 7), l_ground_02 (0.2, 3), m_rock_04 and
  m_rock_05 (0.1, 4); z_rock_01, 02, 04, 05 are type 3 with intensity 0. m_rock_03 has an
  intensity but type 0, so it does not shine. The material block handed to the shader is
  the pattern's twelve values from specularIntensity to `specular`, in table order.
  Confidence: high (shader; the values from heap dump E-trace-007 500 for all 73 entries;
  the setter hands over 48 bytes).
- **GFX-VIEW-2.5 Shadows (Run A, 2026-10-17, E-gfx-011).** One directional
  shadow map from the light of 2.3.
  (a) Casters: drawn into the map with their own shape: buildings and sites
  (rigid and skinned), settlers and animals (skinned), trees (with their
  wind sway), flags (with their wave), stones and doodads (instanced). A map
  object type of `terrain_static_data.xml` with `shadowCast="false"` casts
  none (default true; e.g. grass and small plants). Alpha-tested casters
  (leaves) cut out texels with alpha below 0.5. The terrain itself casts no
  shadow. **The caster's alpha is a plain trilinear sample** (F53, 2026-10-31,
  E-gfx-036, `shadow.fx` ShadowMap_PS with alpha test, passes ShadowPassTree and
  ShadowPassAlpha*): the colour texture is sampled with the mip level the shadow
  map's own derivatives choose, from the texture's own mip chain (the tree
  textures carry full chains, e.g. RS_TreeFir01 512 x 512 DXT1 with 10 levels;
  the loader keeps them, `SET-settings.md` texture quality only drops the top
  levels), with linear min, mag and mip filters; the texel is discarded when its
  alpha is below 0.5, and the pass's alpha test (above 127 of 255) applies as
  well. Small or far leaves are therefore cut on averaged alpha and let light
  through; sampling the top level only makes canopies too solid. No mip LOD
  bias applies (settled 2026-11-01): the only writes of the sampler's mip-bias
  state are the fetch-4 switches on the shadow map's own sampler (E-gfx-038). So which mip a leaf's alpha comes from depends only
  on how many map texels the leaf covers: the 2048 map of (d) fitted to the
  view. An implementation whose map has a different texel density at the tree
  cuts other leaves. In the measured farm pair (`GFX-objects.md` T31's
  look-compare set, weakfogoff-001500-farm) the shade differs mostly along the
  shadow's edges, where the original's whole tree shadow lies up and to the
  left of the implementation's, and the canopy itself is offset by about 10
  pixels. Since the sway clock is game time (GFX-OBJ-2.9), that is a
  difference in the tree's pose or placement, not in the leaf alpha (inside
  the shade the ratio is 1.02 .. 1.04).
  (b) Receivers: the terrain, roads, stones, houses and sites, trees,
  doodads, animals and settlers all sample the map in their normal
  technique (`shadowReceive`, default true), and so does the ground decal
  (GFX-OBJ-2.3a d; **corrected Run D: its Default technique samples the
  map, `house_decal.fx`**). The sky and the build-help icons do not. **The
  water surface does** (corrected 2026-11-09 c, Q-1038, E-gfx-047: its passes
  `RenderWater` / `RenderWaterNoMirror` test the map in the shadow techniques, with
  one point-sampled tap, the wave offset, and a factor that only reaches 0.9;
  `GFX-water.md` GFX-WATER-4.5a); the water's map-edge skirt and the caustics pass
  do not.
  (c) Test: a pixel is lit (factor 1) when its depth in light space plus the
  bias -0.00325 is less than the stored depth, else it takes the factor of
  2.3. Four taps at the pixel's map position and one texel right, down and
  diagonal, mixed bilinearly by the position's fraction inside the texel
  (2 x 2 percentage-closer filtering): edges are soft over one texel.
  (d) Light frustum: fitted every frame to the part of the view frustum
  that holds casters and receivers (casters' boxes extended along the light
  until they meet the ground, up to 1000 units); either an orthographic fit
  or a light-space perspective warp (LiSPSM) is possible; the game always
  asks for **LiSPSM** (the only setter passes it on). The map is square,
  **128 x 2^L texels with L = round(4 q)**, q the `SHADOW QUALITY` slider
  (0, 0.25, 0.5, 0.75, 1): off (no map, (e)), 256, 512, 1024, 2048, halved until it fits
  the device's largest texture; without shader level 2 shadows are off and
  the setting is forced to 0 (`SET-settings.md` SET-4). The sample profile
  (preset VeryHigh) has q = 1.0, a 2048 map (Run D, E-gfx-016). An implementation may use any
  directional shadow map that keeps (a) to (c). On flat ground the shadow of a point h above it falls
  0.839 h away along (0.8192, 0, 0.5736) (the light's horizontal part
  normalised): the 4.1-unit woodcutter's hut throws a shadow about 3.4 units
  (half a field) long toward +x, +z (up and right on screen).
  Confidence: high for (a) to (c), medium for (d).
  (e) **Shadows off (2026-11-12, E-gfx-056, code reading).** Whether shadows are drawn is the
  scene settings' **shadows switch** (`SET-settings.md` SET-4 field 10), not the quality value
  itself: shadows are on when the switch is set and the shader level is 2 or more. Moving the
  `SHADOW QUALITY` slider sets the switch: on for any step above 0, off at 0; so through the
  settings page, quality 0 means off. Applying a preset (also the first slider move away from a
  preset, which turns it into Custom) sets the switch to "`SHADOW DETAIL` above 1/6"; every shipped
  preset with a detail above 1/6 also has a quality above 0, and VeryLow and Low have both at the
  bottom, so the two rules agree for every preset. A profile edited by hand with the switch on and
  quality 0 makes no map while the receivers still test one; no settings page produces it, and an
  implementation may treat it as off.
  With shadows off nothing is drawn into a shadow map, and **every receiver of (b) keeps its formula
  with the shadow test replaced by 1, and nothing else changes**: the ground's
  D = light x saturate(N . L) + ambient (2.4 with s = 1 everywhere), and likewise houses, trees,
  settlers, roads, the decal and the rest; the water surface's factor of `GFX-water.md`
  GFX-WATER-4.5a is 1 (no 0.9 under a shadow); the caustics pass is the same on and off (it never
  reads the map); the haze per vertex, the light and ambient colours and the mirror are untouched,
  and the shadow factor of 2.3 is then unused. So turning shadows off can only **brighten** a pixel,
  and only where a shadow lay: open water and sea ground with no caster above them do not change at
  all. One exception in the shipped shaders: the lava ground layers (pattern types 4 and 5) keep
  their shadow test in both settings (an implementation without a map takes 1 there).
  Confidence: high (every reader and writer of the switch; the shipped techniques compared pass by
  pass: each shadowless pass differs from its shadowed one only in the shadow test).
  **Seen in the original (2026-11-12, E-gfx-056, a look).** The Weak game with fog of war off
  (the set-up of E-trace-017) run to tick 1500 with the switch off and quality 0 (both read
  back in the running game), against the look comparison's picture with shadows on: from the
  default camera (the same camera in both) 91 % of the pixels are unchanged, 8.8 % brighter (the
  tree and building shadows gone; land on average x 1.033) and **none darker**; sand and shore
  water unchanged. Over the cell (6, 3) water, compared at the same ground points: within 0.3 %
  from -0.1 to -1, and x 1.012 / 1.007 / 1.005 (red, green, blue) at -2 .. -3, so the sea ground
  there lies a little in shadow with shadows on and nowhere does the water darken.
  (f) **Houses' walls in the sun's grazing light (measured 2026-11-12 night, E-gfx-058, T43).**
  The town picture at tick 6000 (noon light of 2.3 (e), shadows on, a 2048 map) against the
  program's at the same camera, each house's mesh projected into both pictures and its pixels
  grouped by the face's direction in the model (the program lights the model-frame normal of
  `GFX-objects.md` GFX-OBJ-2.1, T31, and samples the colour map as GFX-OBJ-2.2b):
  - Roofs, and walls the sun lights at more than about 0.3 (the true incidence, with the
    building's -30 degree turn), agree within 1 .. 4 % in all four buildings (the Roman
    headquarters, two quarries, the woodcutter's hut). In particular the faces of the model's -x
    side (true incidence 0.6 .. 0.7) agree within 1 .. 3 %, where a normal turned by the heading
    would make the original 7 .. 14 % brighter: **T31 holds for every house measured.**
  - **The door side** (the model's -z face and the faces up to 15 degrees from it, which look
    toward the lower right of the screen; the sun meets them at a true incidence of only 0.1 ..
    0.2 while the T31 normal gives them 0.4 .. 0.5) is **darker in the original**: the
    woodcutter's hut 0.79 .. 0.81 of the program's, the quarries 0.89 .. 0.90, the
    headquarters 0.95 .. 0.98. On the woodcutter's hut the darkening covers the whole door wall
    under the gable (its brightest 2 000 pixels 0.68 / 0.65 / 0.69 in red, green, blue), about
    what the shadow factor of 2.3 gives there (A + light x 0.48 x 0.1 against A + light x
    0.48, with the light map term: 0.6 .. 0.7). Reading: in the original the roof's overhang
    shades the door wall. A ray toward the sun leaves such a wall by only 0.11 for every 0.77
    it rises, so an overhang of a few tenths of a unit shades about two units of wall below
    it. The test is (c): the pixel's own light-space depth with the constant bias, no offset
    along the normal, so a grazing wall right under an overhang is not pushed out of the
    overhang's shadow.
  - A few small groups of faces that look away from the true sun (true incidence 0 or below)
    but get some light from the T31 normal are **brighter in the original** (quarries 1.1 ..
    1.5, about 300 pixels each): there the original shows that light without the shadow
    factor, the program with it.
  An implementation must reproduce the door-side darkening (T43). One that pushes its shadow
  lookup out along the surface normal, or adds a large depth bias, lights these walls and
  matches the original only on the faces the sun meets at a steep angle. Confidence: measured
  (one picture pair, four buildings); the mechanism (the overhang's shadow on a grazing wall)
  is inferred from the geometry and (c), not traced pixel by pixel.

- **GFX-VIEW-2.6 Colour pipeline (play test 10, F10, F11, F19; E-gfx-021).**
  How a texel becomes a pixel in the original; every 3D surface follows it.
  (a) **Textures** (DDS: DXT1, DXT3, DXT5 or 32-bit; TGA) are loaded as
  stored, with no format or colour conversion; the texture-quality setting
  may only drop mip levels (never for house light maps). A texel's value is
  its stored byte / 255. There is **no sRGB decoding** on any texture read.
  (b) **All lighting arithmetic works on those values directly**: the light
  and ambient colours of 2.3 multiply the stored texel values; the light
  factor D may exceed 1 (lit ground 1.32 / 1.19 / 1.20) and nothing is
  clamped before the output except `saturate` terms the formulas name.
  (c) **Output**: the result is clamped to 0..1 and written as round(255 x)
  into an 8-bit-per-channel back buffer (32-bit display modes). There is
  **no sRGB encoding** on write (the game never enables an sRGB write or
  read state), **no gamma ramp**, no brightness or gamma setting, **no tone
  mapping, no exposure, no bloom, no dithering** (the post effects run only
  with the command-line switch of 2.3). Alpha blending (roads, decals,
  sprites) mixes these 8-bit values linearly: src x a + dst x (1 - a).
  (c1) **Blend and alpha states of the see-through ground layers (play
  test 10 look-compare; E-gfx-028).** Roads, crossings and the
  flag-to-door strip (`street_shader.fx`, pass RenderStreet) and the
  ground decal (`house_decal.fx`) use one and the same state block, set by
  the effect pass and not changed by the code that draws them: depth test
  on, compare less-or-equal; **depth write off**; **alpha test off** (no
  alpha reference, no discard: a texel of alpha 0 is drawn and changes
  nothing); **alpha blending on, source factor = source alpha, destination
  factor = 1 - source alpha, blend operation add**, the same for the
  alpha channel (no separate alpha blend), all colour channels written.
  (Buoys: depth write on, blending off.) The shader's output for one
  pixel is
  rgb = clamp(D x texel.rgb x fow, 0, 1), a = texel.a,
  with D = ambient + light colour x saturate(N . L) x s (2.3, 2.5; for a
  road N from its normal map, for the decal the mesh normal) and fow the
  fog-of-war factor (1 in explored land); fog of war scales only rgb, never
  a. Nothing multiplies rgb by a in the shader (straight, not
  premultiplied, alpha). The blender then stores, per channel, with the
  stored 8-bit value d of the ground already drawn there:
  out = round(255 x (rgb x a + d / 255 x (1 - a))).
  The texture alpha is the DXT3 4-bit value x 17 (0, 17, ..., 255) for
  both `simple_street.dds` and `house_decal.dds`, bilinearly interpolated
  between texels. Mixing the values in linear light (an sRGB target)
  makes every half-covered pixel lighter, by up to about 3 of 255 for
  these colours (T25, T26); opaque and fully clear pixels are unchanged.
  Confidence: high (`street_shader.fx`, `house_decal.fx`, `shared.inc`
  state macros; E-gfx-028: the street draw only
  begins the effect pass, no device render state calls).
  (d) **No distance fog**: the fog values of `daytimes.xml` are read but no
  shader uses them; the only darkening by place is the fog-of-war factor
  (a multiply by the explored-map alpha, 1 in explored land, GFX-VIEW-5).
  (e) **Formulas** (L = the light vector of 2.3, shadow s = 1 lit / 0.1 in
  shadow, 2.5): terrain: (ambient + light x saturate(N . L) x s) x texel,
  N from the pattern's normal map (2.4); roads and the flag-to-door strip:
  the same with the road normal map (`GFX-objects.md` GFX-OBJ-6.3, 6.4);
  ground decal: the same with the decal mesh's own normal
  (GFX-OBJ-2.3a d); houses: D = ambient + light x saturate(N . L) x s with
  N . L per vertex, then D x texel x colour-variation texel, then the light
  map term of GFX-OBJ-2.1 (per channel weight saturate(1.5 - D)); settlers:
  one scalar D = lum(light) x saturate(N . L) x s + lum(ambient)
  (0.8965 and 0.5413) times the palette colour (GFX-OBJ-3.3).
  (f) Shaders run at reduced precision (16 to 24 bit floats); results agree
  with exact arithmetic within 1 of 255.
  A program that decodes textures as sRGB, lights in linear space and
  encodes to sRGB, or tone-maps the result, draws lit surfaces darker (the
  1.2 to 1.3 times boost of D shrinks to 1.08 to 1.13), shadows weaker (0.6
  becomes about 0.8) and colours more saturated than the original.
  Confidence: high (E-gfx-021: render and sampler states set by the
  executable, `.fx` files, texture loader).
- **GFX-VIEW-2.7 Order of the ground layers (Q-303, E-gfx-030).** The map
  draws its ground layers in this fixed order, every frame:
  1. the terrain (base and transition patches, ground cover);
  2. the street pass: land roads, road crossings and the flag-to-door
     strips (GFX-OBJ-6.4), all through the street renderer;
  3. the territory border markers;
  4. the buildings' ground decals (GFX-OBJ-2.3);
  5. the buildings: construction frames and sites, houses, add-ons, flags
     (GFX-OBJ-8);
  6. the remaining map objects.
  The settlers' sorted queues, water and the other scene parts come after
  the whole map pass. Roads, strips and decals all test depth with
  less-or-equal and write no depth (2.6 c1), so they are tested only
  against the terrain, and a later layer is blended over an earlier one
  wherever both cover a pixel. A decal is therefore mixed **on top of** a
  road or strip that runs under it: result = a_d x decal + (1 - a_d) x
  (a_s x strip + (1 - a_s) x ground), with a_d, a_s the two alphas. The
  decal's 0.01 lift plays no part in the order. Confidence: high (each of
  the three draws has exactly one call site, in this order, in the map
  pass: streets, decals, buildings).

### GFX-VIEW-3 Camera

- **GFX-VIEW-3.1 Parameters.** Horizontal field of view 50 degrees (3.5), near
  plane 1.0, far plane 500 world units (doubled while a game flag that is off
  by default is set; its setter was not found), zoom level z in integer steps
  0 .. 4 (start 3 when a game begins, 2 after a scripted move), yaw target in
  {-1.5, 0, +1.5} radians. The value 70 kept in the camera is a distance base
  (3.3), not the field of view. The `<Camera>` block of `graphics.xml`
  (moveSpeed, zoomSpeed, tilt, elevation and zoom limits, relative heights) is
  **not read** by the game: none of its attribute names occurs in the
  executable. Confidence: high (E-gfx-009).
- **GFX-VIEW-3.2 Input per frame.** step = min(1, dt x 25) x (5 x s)^2 with s
  the options' camera speed (default 0.25, `SET-settings.md` SET-2 field 7).
  Rotate-left key: yaw target = 0 if it was -1.5
  else +1.5; rotate-right: 0 if it was +1.5 else -1.5. Arrow keys and (when
  enabled in the options) the mouse at the screen edges move the target by
  2 x step along the horizontal view direction or sideways (right = (dir.z,
  0, -dir.x)); middle-button drag moves it by 40 x mouse delta x (10 s)^2
  (direction flipped by the invert option). Zoom in: z -= 1 while z > 0; zoom
  out: z += 1 while z < 4. A new target is accepted only when it lies over a
  fine sample inside the map (3.4). Keys (virtual-key codes, fixed in the code):
  rotate left = Delete, rotate right = Insert, zoom in = Page Down or mouse
  wheel forward, zoom out = Page Up or wheel back, move = arrow keys (held),
  drag = middle mouse button, free look = right mouse button **held** (3.2a);
  the option defaults (mouse scrolling on, not inverted, speed 0.25) are in
  `SET-settings.md` SET-2. Confidence: high (keys and the right button, E-sim-005).
- **GFX-VIEW-3.2a Right-button look (F46, 2026-10-23; E-sim-005).** While the **right mouse button is
  held** over the game and no scripted or follow mode runs, the camera is in its
  look mode. It is entered in the first frame the button is down (after that
  frame's key, edge and middle-button handling, skipping the zoom keys): the base
  direction is saved and replaced by the current view direction and the pointer is
  hidden. In every following frame whose frame time is at most 0.25 s:
  (a) the mouse movement turns the view: yaw by -10 x dx radians about the
  vertical axis (dx the horizontal mouse delta in the units of 3.2's middle-button
  pan, a fraction of the screen per frame, so a tenth of the screen width turns by
  1 radian = 57 degrees) and pitch by adding 6 x dy to the direction's vertical
  component, clamped to +-0.9 (at most 64 degrees up or down), the direction then
  re-normalised; one options byte (presumably the
  `Inverted` setting of SET-5.5, Q-344) flips dy. The view follows at once: no
  easing, no yaw target, no three-position snapping;
  (b) the arrow keys fly: UP / DOWN move the camera **and** the target along the
  horizontal view direction by 2 x step (step as in 3.2 but without the min(1,
  ...) clamp), LEFT / RIGHT sideways by 2 x step, a move only when the new camera
  position lies over the map; edge scrolling, the middle button and the zoom keys
  do nothing in this mode;
  (c) the pointer stays hidden and is clamped to 1 % .. 99 % of the window every
  frame; the camera never goes below max(ground under it, water level + 5).
  There is no drag threshold and no inertia. **On release** the look is dropped:
  the saved base direction is restored and the pointer is shown again where it was
  clamped; the normal camera (3.3) then eases the view back (0.4 of the remaining
  difference per frame) to the yaw-rotated base direction and re-places the camera
  from the target, so only the target moves of (b) persist. A right click without
  movement therefore changes nothing visible except that the pointer is hidden
  while the button is down. The map menus' close-on-right-button (`UI-ingame.md`
  UI-GAME-1.10a) is independent of this. Nothing of it is saved (the camera is
  rebuilt at every game start, 3.6). The other rotation inputs are Delete / Insert
  (3.2) and the jump keys (H / HOME: yaw 0). Confidence: high for the mode, its
  entry / exit and the formulas; medium for the delta unit and the sign option
  (not checked live, Q-344).
- **GFX-VIEW-3.3 View update.** Ground g = mean of the plane heights (1.4) at
  the target and at (+-10, 0), (0, +-10) around it; floor = max(g, water level
  + 5) + 5; a smoothed ground value is replaced by floor only when it is
  negative or differs by 10 or more; target.y = it. Pitch component p = -0.3
  (z = 0), -0.6 (z = 1), -0.8 (z >= 2), taken from trunc(z). The **base
  direction** b = (bx, p, bz) is re-normalised **as a whole** every frame, so
  its horizontal part converges to sqrt(1 - p^2) and the view direction is a
  unit vector whose y component is p: the pitch below the horizontal is
  asin(|p|) = 17.46 (z = 0), 36.87 (z = 1), 53.13 degrees (z >= 2) (confirmed 2026-11-08 b, Q-205:
  E-gfx-041, trunc of the zoom value picks -0.3 / -0.6 / -0.8, and only in the normal
  camera; another camera mode keeps its own direction, so a view of the sky at the closest zoom,
  OBS-1, comes from such a mode, e.g. the right-button look of 3.2a, not from zoom 0 itself). (From a
  fresh (0, -1, 1) start the horizontal part is within 1 % of the fixed point
  after three frames; implement the fixed point.) yaw += (yawTarget - yaw) x
  0.245 per frame. Direction d = normalize(cos yaw x bx - sin yaw x bz, p, cos
  yaw x bz + sin yaw x bx); the smoothed direction used for the view moves
  towards d by 0.9 of the difference when the zoom changed this frame, else
  0.4, and is normalised again. Camera position = target - normalize(d.x, 1.4
  d.y, d.z) x 0.3 x 70 x z, i.e. 21 world units per zoom step (zoom 0 puts the
  camera on the target); the elevation of that offset is atan(1.4 |p| /
  sqrt(1 - p^2)) = 23.8, 46.4, 61.8 degrees, so at zoom 2 / 3 / 4 the camera
  sits 19.9 / 29.8 / 39.7 units behind and 37.0 / 55.5 / 74.0 above the
  target. **Corrected (play test 10, E-gfx-019):** the camera's x
  and y take their new values at once; its **z** eases: z = z_new + 0.3 x
  (z_old - z_new) each frame; y never goes below max(ground at the camera +
  10, water level + 5). The view matrix looks
  from the camera position along the smoothed direction (up = +y, medium). A
  "jump to field" puts the target at the field's world point (its y is
  replaced by the floor in the same update, so the field ends up at the
  look-at point, the centre of the screen), yaws 0 and sets the direction
  (0, -1, 1) / sqrt 2; a new game jumps to the player's headquarters.
  **Corrected (play test 10, E-gfx-019):** the jump puts the
  target at the field's x and at its **z - 18**, not at the field itself; see
  3.6 for where that puts the field on screen.
  Confidence: high (E-gfx-009).
- **GFX-VIEW-3.4 Scroll limits.** The target (not the camera) must map to a
  fine sample with 0 <= column < W and 0 <= row < H; moves that would leave
  the fine grid are ignored, so the target can reach the map edge exactly
  and the camera may stand outside the map. With the pitches of 3.3 and the
  vertical field of view of 3.5 the farthest ground point on screen lies 70
  (4:3) or 54 (16:9) world units beyond the target at zoom 4, i.e. inside
  the 200-unit skirt (`GFX-water.md` GFX-WATER-6): in normal play nothing
  beyond the skirt is visible. Confidence: high (limit), medium (derived
  distances).
- **GFX-VIEW-3.5 Projection.** Left-handed perspective with the 50-degree
  field of view applied **horizontally**: x scale = 1 / tan(25 deg) = 2.1445,
  y scale = x scale x (width / height), depth from near 1 to far 500. The
  vertical field of view is 2 atan(tan(25 deg) / aspect): 38.6 degrees at
  4:3, 32.5 at 16:10, 29.4 at 16:9. Wider screens see more to the sides, not
  less above and below. Menus without a camera use 45 degrees. Confidence:
  high (E-gfx-009).

- **GFX-VIEW-3.6 Start camera (play test 10, F30, OBS-31; E-gfx-019).**
  (a) **Every game start** (a new free game, a loaded game, a campaign
  mission, a tutorial lesson) creates the camera afresh when the game screen
  is built: the camera is not stored in a save. It starts at zoom **3**
  (the second farthest of 0..4, OBS-1), yaw 0 and yaw target 0 (no
  rotation: the view looks toward +z), and then, if the local player owns a
  headquarters, jumps to it (3.3): target x = the HQ field's world x,
  target z = its world z - 18, target y = the floor of 3.3 (on dry flat
  ground: ground + 5). The point used is the HQ's **field point** (the
  building's own field, MAP-GRID), not its door or model centre. Without a
  headquarters (a loaded game whose HQ was lost) the target keeps its
  initial value (0, 10, -10): the view opens at the map's origin corner.
  (b) **Where the HQ lands.** Because the camera sits along normalise(d.x,
  1.4 d.y, d.z) and looks along d, the -18 shift brings the HQ to the middle
  of the screen. With the converged pitch at zoom 3 on flat ground the HQ's
  field point is at normalised screen height +0.070 (4:3) / +0.093 (16:9)
  (positive = above the centre; 1 = top edge), i.e. 27 px above the centre on
  a 768-pixel-high screen, 50 px on a 1080-pixel one; horizontally exactly in
  the centre. A point 7.5 units above it (about half the HQ's height) is at
  +0.257 / +0.342. The look-at point is thus a little in front of the HQ.
  (c) The first frames: the camera's z eases in from the initial position
  (3.3, 70 % of the gap closed per frame, done within about ten frames) and
  the base direction reaches its fixed point within about three frames.
  (d) **Later jumps** use the same rule: H or HOME (`UI-ingame.md`) resets
  zoom to 3 and yaw to 0 and jumps to the HQ again; in tutorial lesson 4 the
  game itself jumps back to the HQ (and shows the H-key hint, CAM-5.4) when
  the camera has gone more than 150 units from it. The campaign scripts
  have no camera call.
  (e) **Scripted camera move (camera mode 5; corrected 2026-11-05, Q-1004;
  E-cam-005).** Started by the tutorial (`CAM-campaign-
  missions.md` CAM-5.3b) with a field F: the target is F. Every frame of the
  move the zoom is set to **3** (the start call writes 2, but the first frame
  overrides it, so the move ends at zoom 3), the camera position that the
  jump of (d) would give for F at zoom 3 is computed as the wanted eye, and
  the real eye moves **12.5 % of the way** to it (the view direction is kept).
  When the eye is within 0.1 of the wanted eye the move ends and the normal
  camera (mode 0) resumes. **While the move runs the player's camera input
  (scrolling, zoom keys, right-button look) is not read**, since only the
  current mode's handler runs; from 50 units away it takes about 47 frames.
  Confidence: high.
  Confidence: high (E-gfx-019); medium that the game-screen build at
  runs for loaded games too (the save holds no camera object).
- **GFX-VIEW-3.7 Follow camera (camera mode 3; added 2026-11-05, Q-1010; E-cam-004).** The O key with an own settler under the pointer
  (`UI-ingame.md` UI-GAME-9.3) switches the camera to follow him. Each frame, with P his
  position, F his facing (the forward axis of his world matrix, unit length) and S the first
  axis of that matrix (the side axis, unit length; GFX-OBJ-1.8), and the camera's **pose flag**:
  - **behind** (flag 1): wanted eye = P - 4 F + S + (0, 2, 0), looking at P + (0, 2, 0);
  - **ahead** (flag 0): wanted eye = P + 5 F + S + (0, 2, 0), looking back at P + (0, 1.25, 0)
    (his face);
  - **at a building** (he stands where the game finds a building for his position, e.g. on his
    way in or out): look-at L = P + (0, 1.1, 0); the eye starts at the building's field moved one
    step in direction 1, 1.15 above it, then moves 3 units back along the line from it to L, then
    + S; this pose has no height floor.
  The wanted look direction is the unit vector from the wanted eye to the look-at point. The real
  eye and the real view direction each move **5 % of the way** to the wanted ones every frame
  (no time step). Then, outside the building pose, the eye is raised to at least max(ground + 1,
  water level + 3) at its own position (the water level is 0, `GFX-water.md` GFX-WATER-3.3, so the eye never goes below height 3). The picture is drawn from the real eye along the real
  direction with the normal projection (3.5: 50 degrees, near 1, far 500); **the zoom level plays
  no part** while following and is not changed. So the settled eye is sqrt 21 = 4.58 units from
  him behind and sqrt 30 = 5.48 ahead (SND-8.6). **Keys while following:** PAGE DOWN / wheel up
  switches behind -> ahead (nothing when already ahead); PAGE UP / wheel down switches ahead ->
  behind, and from behind **ends the follow**. The follow also ends when he no longer exists.
  Ending puts the camera back in its normal mode (mode 0) with the base direction (0, -1, 1) and
  the smoothed ground reset; it does not move the target back. The flag is set to behind when a
  game's camera is set up and is otherwise kept: a new follow starts in the pose the last one
  ended in. Confidence: high for the offsets, the easing, the floor and the keys; medium for the
  building case's condition and for the normal camera's state after the end.

### GFX-VIEW-4 Placing objects

- **GFX-VIEW-4.1 Headings.** An object facing direction d (MAP-GRID-1.2) is
  rotated about +y by 90, 150, -150, -90, -30, 30 degrees for d = 0 .. 5. The
  current heading eases toward the target heading by 0.4398 rad per logic tick (the ease runs in the per-tick update, `GFX-animations.md` GFX-ANIM-4.3)
  (angles wrapped to +-pi). Confidence: high.
- **GFX-VIEW-4.2 Settlers and animals.** Position = from + (to - from) x f
  where from/to are the world positions (1.1) of the current and next field
  (fine samples during a free walk, `TRN-free-walk.md`) and f the movement
  progress of the simulation; height interpolates linearly too. Heading from
  the direction of the step. Scale per 1.3. Confidence: high.
- **GFX-VIEW-4.3 Buildings (corrected Run A, 2026-10-17, Q-083, E-gfx-011).**
  (a) **Heading.** Every building, site and military building is turned
  about +y by its type's `heading` (degrees). No `buildings.xml` entry sets
  it, so all use the loader's default **-30 degrees**, with the same sense as
  GFX-VIEW-4.1 (a turn by a moves +z to (sin a, 0, cos a)). The building
  models are authored with their front (door side) toward model -z; turned by
  -30 degrees the front faces world (0.5, 0, -0.866), which is exactly the
  direction from the building field to its flag (direction 1, south-east:
  world offset (+3, 0, -5.196)). So every building faces its flag, and on
  screen (camera looking along +z) the door looks toward the lower right,
  as in `hq.jpg`. The angle is fixed: it does not depend on the tribe, the
  terrain or the flag's place (the flag is always direction 1). The ground
  decal (GFX-OBJ-2.3), the site model and its frame pieces, the rising house
  and the military flags (Run B) all use the same matrix.
  (b) **Matrix.** World = R x T with R the rotation of (a) (rows (cos a, 0,
  -sin a), (0, 1, 0), (sin a, 0, cos a) in the row-vector convention, a =
  -30 deg: (0.866025, 0, 0.5), (0, 1, 0), (-0.5, 0, 0.866025)) and T the
  translation to the building field's world position; no scale (models load
  at 1.0, 1.3).
  (c) **Height.** y = the height of the building field's own sample
  (MAP-GRID-4.2: sample (4x + s, 4y), raw height x 0.001), taken when the
  visual is made and again when the terrain there changes (levelling). It is
  not the lowest, mean or interpolated height of the footprint; a small
  building (no levelling, `ECO-warehouse-construction.md` ECO-HQ-4.2) on a
  slope therefore has part of its base below the terrain on the uphill side,
  which the models allow for (their base reaches 0.15 to 0.6 units below the
  origin: woodcutter's hut -0.228, castle -0.145, guard house -0.621).
  (d) **Sinking.** A destroyed building sinks: y -= model height x t^2 with t
  growing 0.35 per second (clamped at 1), model height = the type's bounding
  box top minus bottom.
  (e) **Doors and the walk in.** A settler entering walks the logic step
  from the flag to the building field (direction 4) in a straight line;
  with (a) that line runs straight at the door. The door opens when the
  entering step's progress passes 0.3 and at once for a leaving step
  (`GFX-animations.md` door rules); he is drawn until his step ends on the
  building field (the origin). The hut's door (bone at model z -1.58) lies
  1.6 units before the origin, so he passes through it; the castle's
  recessed doors (z +2.16) lie 2.2 units beyond the origin, so he vanishes
  inside the porch, short of them. Soldiers use the same steps.
  Confidence: high (heading, matrix, height), medium (which objects share
  the matrix besides house and decal).
- **GFX-VIEW-4.4 Construction stages.** While the building is unfinished
  (frame or main work remaining) the `frame` model of `buildings.xml` (the
  construction-site mesh) is drawn with the frame technique, afterwards the
  `mesh` model with the house technique; the shaders take no progress input,
  so no intermediate stage is rendered by them. The visual keeps the frame
  progress (1 - frame remaining) and the upgrade progress for other uses.
  Confidence: medium (Q-083: the draw path that could switch meshes by
  progress was not fully read).
- **GFX-VIEW-4.5 Items and trees.** Items use the mesh per good of
  `items.xml`; a carried good sits at the settler's hand with offset (-0.5,
  0.4) and rotation 10 degrees (settler_config); a boat at (-0.25, 0.2). Trees
  and other deposits stand at their field's world position plus the random
  fine offset of `TRN-free-walk.md` TRN-FREE-5.4; growing deposits are scaled by
  height s and width min(3 s, 1) with s the deposit's size (`GFX-animations.md`
  GFX-ANIM-3.8b, Run C). Random yaw and size per
  element, the model and shader lookups, flags, border markers, wares at
  flags, roads and the object shaders: `GFX-objects.md`. Confidence: medium.

### GFX-VIEW-5 Fog of war

- **GFX-VIEW-5.1** A world texture with one texel pair per field (width 2 W +
  1, height H) covers the map; its uv at world (x, z) = ((x - x0) / (x1 - x0),
  (z - z0) / (z1 - z0)) with the field-grid corner positions. Its rgb is the
  minimap colour with the water depth term (`GFX-water.md` GFX-WATER-2.5), its
  alpha is the visibility of the field: terrain, buildings, doodads, trees
  and settlers multiply their colour by it, so unexplored land is black and
  explored land is lit. On (re)initialisation alpha is 255 only where the
  texel was pure white, 0 elsewhere; a reveal mode inverts it. Confidence:
  high (shaders and texture builders); the per-field update as the player
  explores was not located (Q-085).
  **The per-field update (answered 2026-11-08 b, Q-085; E-gfx-041).** Every exploration region notice (5.4 d) rewrites the world
  texture over the notice's field box for the local player: a field whose byte in that player's
  exploration layer is 0 gets alpha 0 in both of its texels, any other field alpha 255 (rgb kept);
  the texel pair of a field is laid out as in GFX-WATER-4.4 (odd rows shifted by one texel; the
  extra texel closing an even row takes the row's last field's value). The map-sized fog texture
  made beside it (one 32-bit texel per field) is cleared to 0 for every explored field in the
  same notice; what reads it was not traced. In the texture's other mode (5.1's reveal mode) the
  texels are written whole: unexplored 0xFF000000, explored 0x00FFFFFF. Confidence: high (code).
- **GFX-VIEW-5.2** Unexplored terrain cells can also be drawn with a
  texture-less "unexplored" technique. Confidence: medium.
- **GFX-VIEW-5.3 "Fog of war: Off" (Q-290, 2026-10-30).** The free game's fog of
  war option (`UI-game-setup.md` UI-SETUP-1.8, drop-down n 2; the session's third
  option byte, 0 On, 1 Off) changes the drawing only: with Off the whole map is
  drawn as explored (player 1's land and the coast far from player 0 lit from the
  first tick, T29), and the game is the same game: a free game set up with fog of
  war Off equals the same set-up with it On in every recorded value. What the
  setting does inside the drawing (every field's explored value, or the test
  skipped) is not traced. Confidence: high for "the game is the same" in the
  measured game (T29), medium beyond it. **Corrected 2026-11-15 night (E-trace-027, a live read of
  the running game; Q-1049):** the game is the same **only for a while**: the Weak game set up with
  fog of war Off, run with the fixed step to counter 36 999 and read there while paused, has the
  generator state 272511491 and the next object number 93877, where the recording with fog of war
  On (E-trace-014) has 33210560 and 93959 at the same counter. So the setting reaches the
  simulation somewhere between counters 11 099 (T29, still equal) and 36 999; where is open (a
  computer player that reads the local player's exploration, for one). Pictures of the original
  taken with fog of war Off stand for the recorded game only up to counter 11 099 (the
  look comparison's `weakfogoff` pairs at 1500, 6000, 11000 are safe).
  **Where it acts on the ground (2026-11-08 b, E-gfx-041):** the
  scene keeps a switch named `FogOfWar` (a named script variable of the scene); whenever its value
  differs from the one the world texture was last filled with, the whole world texture is filled
  again: with the switch on from the local player's exploration layer (5.1), with it off every
  field lit (alpha 255). So with fog of war Off the ground, buildings and objects are lit
  everywhere through the world texture alone. The settlers' seen mark (5.4) reads the exploration
  layer itself and does not test the switch; how the option reaches the switch and whether it also
  fills the layers was not traced.

- **GFX-VIEW-5.4 A settler's seen mark (Q-293 b, 2026-10-31; E-gfx-037).** Every settler's view
  carries a "seen" mark. A settler is drawn in a frame only while the mark is set (and its
  field lies inside the frame's drawn field rectangle).
  (a) **Start:** the mark is clear when the settler's view is made, i.e. a new settler
  starts unseen.
  (b) **Refresh:** the mark is set to "the local player's visibility layer is non-zero at
  the settler's field". Three conditions must hold in that frame:
  - the drawing has the local player's layer (without one no mark is ever set; the
    fog-of-war-off pictures show settlers, so a layer is in use there too);
  - the settler's field is inside the drawn field rectangle, or it is the settler
    renderer's first frame after it was made (once per game);
  - the settler is walking (its step under way; a step frozen by the pause counts), or
    the frame is the one marked after a change of the visibility layer. Such a change
    (the world texture's region update) marks one frame in which every settler inside
    the rectangle is refreshed whether it walks or not.

  In every other frame the mark keeps its value.
  (c) **Consequence.** A settler that appeared and walked only outside the drawn area
  keeps the mark clear. When the camera then jumps to it while it stands (working or
  waiting), it stays invisible until it takes a step in view or the visibility next
  changes. The camera jump of the shots run (made while paused) shows exactly that:
  constructor 90640 of the farm 90538 (T36) appeared at counter 4729 at player 1's
  castle and walked to the site while the camera was on player 0's castle. It stood
  working when the camera moved to the farm.
  (d) **The layer and its changes (answered 2026-11-07, F54; E-gfx-040).** The layer read in (b) is the local player's
  **exploration** layer (FMT-SAV's `Map Exploration`: one byte per field, set when the player
  has ever explored it; the save of the Fight game holds it), not the current view: once a
  field is explored, a refresh there sets the mark for good. "A change of the visibility
  layer" in (b) is **any** exploration, by any player and whatever the local player sees:
  every time a player explores fields, the game marks them (for that player and the players
  not hostile to it) and the view then marks the next frame for the refresh of every settler in
  the drawn area. Explorations happen when a soldier walks next to fields its player has not
  explored, when a military building is occupied or the territory
  changes, when a ship moves (radius 3) or reaches an expedition spot (radius 6), when a
  harbour site is made, by specialists, by scripts, and at the game's start.
  How often that happens depends on the game; in a paused
  game it never does.
  **Fog of war Off:** where the option acts was not found (the layer starts all 0 and the world
  texture reads it without testing the option). The pictures show that the local player's
  layer is non-zero on the other player's land in a fog-off game: leveller 90750, walking on
  (137, 57) on player 1's land far from player 0's at tick 6000, is drawn in the farm picture,
  which needs a set byte there. So with fog of war Off treat the local player's exploration as
  complete for the seen mark (medium). (Corrected 2026-11-08 b: the figure drawn there was taken
  for constructor 90571 until then; 90571 stands at fine (552, 219), below that picture's lower
  edge.)
  (e) **The farm picture explained (Q-293 b answered 2026-11-08 b, E-trace-017 drive.log and
  shot files, E-gfx-041).** Every shot of the recording was taken paused (counters 1499,
  5999, 10999). From 1499 to 5999 the camera stood on player 0's castle (target (-331.0, -192.7),
  zoom 3) while the game ran, and the moves to the scenes were made paused. So no settler of
  player 1's land was inside a drawn area during a running tick. In the first frame at the farm,
  (b) refreshes exactly the settlers there that are walking (frozen steps): leveller 90750 and
  carrier 90554 are drawn; constructor 90640 and worker 90480 (state 36, standing) keep the clear
  mark of their walks off screen and are not drawn. No further refresh route is needed.
  (f) **The renderer's loop (E-gfx-041).** The settlers are gone through (refreshed,
  tested, drawn) only when the frame's drawn field rectangle lies wholly inside the map; the
  forced-refresh mark of (b) is cleared only after such a pass, so it waits for the next frame
  that makes one. The first-frame mark is cleared after every frame. **The rectangle (2026-11-08 b,
  Q-1031 partly; E-gfx-041):** each frame, before the settlers are drawn, the scene copies
  it from the terrain view's visible range (four field numbers of the terrain cell renderer's
  view: first and last column, first and last row), so it is the terrain's own view of the frame,
  not the settler's projection. **How the range is made (answered 2026-11-08 c, Q-1031; E-gfx-042, read live in E-trace-021):** the terrain is cut into
  cells of 4 x 4 fields (43 x 43 cells on a 170 x 170 map); cell (i, j) covers the fields from
  column 4i to min(4i + 4, width - 1) and row 4j to min(4j + 4, height - 1), **both ends
  included** (neighbouring cells share their border column and row; the last column of cells on
  a 170-wide map is 168 .. 169). Each frame the terrain's view test picks the cells whose box (the
  cell's world bounds, heights included) the camera sees, and while it lists them it keeps the
  smallest first column, smallest first row, largest last column and largest last row: that is the
  rectangle. Measured at the start view of Storm Coast (zoom 3, counter 369): 20 cells drawn,
  rectangle columns 20 .. 40, rows 44 .. 64, exactly the union of the 20 cells. A settler is inside
  when first column <= its field's column <= last column and first row <= its row <= last row
  (borders included). Since every cell lies in the map (largest last column width - 1), the
  inside-the-map test of (f) fails only when the frame draws no terrain cell at all. Confidence:
  high for the cells, the union and the inclusive test (code and live read); medium for the view
  test itself (the cell boxes against the camera's view volume; not decoded in detail).
  (g) **Tools that play a game ahead** (a scene or a look comparison of a recorded game) must
  reproduce the recording's camera: the camera on the game's start view (the local player's
  castle) for every tick played, then the paused jump to the scene. Refreshing every settler in
  the first frame is not the original's behaviour (T36a).
  Confidence: high for (a) and the refresh conditions (code: the view's motion record
  is made with the mark clear, and only writes it); low for the fog-off layer.

### GFX-VIEW-6 Build help and cursor

- **GFX-VIEW-6.1 Build-help icons.** For every field of the visible range
  except the cursor field whose build class for the player (`ECO-building-
  placement.md`: 1 flag, 2 small, 3 medium, 4 large, 5 mine, 6 harbour) is
  not 0, a billboard 1.6 x 1.6 (graphics.xml Landscape iconSize) is drawn
  standing on the field's world point: the quad is built in view space (x
  right, y up, z into the screen) with its BOTTOM edge centred on the anchor,
  reaching 1.6 up and 0.8 to each side, moved 1 unit toward the camera
  (iconOffset (0, 0, -1), a view-space offset); it is turned to face the
  camera and placed at the field's world point, with
  the icon atlas column u = 0.375 (flag), 0.25 (small), 0.125 (medium), 0.0
  (large), 0.5 (mine), 0.625 (harbour) (eight icons per atlas row), in
  batches of 50. The atlas is `data/map/textures/infomap.dds` (512 x 64,
  eight 64 x 64 icons), drawn unlit with alpha blending (source alpha), depth
  TEST on (terrain in front hides it), no depth write, no alpha test, culling
  clockwise. Confidence: high (E-pt2-001,
  infomap_shader.fx).
- **GFX-VIEW-6.2 Cursor icons (road building).** Only while a road or
  waterway is being built: the cursor field gets a 32 x 32 ring sprite from
  `uiElements3` at (224, 96) projected to the screen, and each of its six
  neighbours gets a sprite: (192, 96) on fields already in the route;
  otherwise for a land road (192, 96) when the road cannot continue to that
  neighbour, else a slope arrow by the height class of MAP-GRID-6 between
  cursor and neighbour: 0, 1, 2, 3 -> x = 288, 320, 352, 384; -3, -2, -1 ->
  416, 448, 480 (y = 128); for a waterway (288, 128) where the route may
  continue and nothing otherwise. "Projected to the screen" means: the
  field's world position is projected; the sprite is 32 x 32 user interface
  units and its top-left corner is placed 22 screen pixels left of and
  above the projected point (converted to interface units by the interface
  scale), so the point lies near the sprite's centre. Outside road building
  no ring or arrows are drawn; the hovered field shows the cursor icon of
  `UI-ingame.md` UI-GAME-13 in every mode (an earlier version of this rule
  applied the ring and arrows to every mode: wrong, re-checked 2026-10-17,
  E-ui-005). Confidence: high.

### GFX-VIEW-6 Beyond the map edge, background and sky (Q-189 a)

- **GFX-VIEW-6.1 Clear colour.** The frame is cleared to the day-time table's
  background colour, (0.8, 0.9, 1.0) in the shipped data. There is no distance
  fog: the fog settings of `daytimes.xml` are uploaded but no shader reads
  them. Confidence: high.
- **GFX-VIEW-6.2 Skirt.** The 200-unit skirt around the map
  (`GFX-water.md` GFX-WATER-6.1 / 6.2) is a flat black ground at the lowest
  edge height with the water surface over it; the water takes the fog-of-war
  value of the **nearest edge field** (clamped sampling), so it is black
  wherever the edge field is unexplored and ordinary water where the player
  has explored the edge. The "dark fog" at the map edge is this fog of war of
  the border fields. Confidence: high (`water_shader.fx`, E-gfx-009).
- **GFX-VIEW-6.3 Sky plane.** A cloud cap is drawn first in every frame
  (before the terrain), alpha blended over the clear colour.
  **Geometry (corrected 2026-11-12 night, E-gfx-059; the earlier side 981 and bend 5024 were
  misread).** A grid of 64 x 64 cells, vertices i, j = 0 .. 64, over a square of side
  S = 2 sqrt(5010^2 - 5000^2) = 632.77 world units (cell 9.8871), centred on the camera's eye
  and 15 units above it: vertex (i, j) at x = 9.8871 i - 316.39, z = 9.8871 j - 316.39 from the
  eye, y = 15 - (x^2 + z^2) / 5010 (15 at the eye, -4.98 at a mid edge, -24.96 at a corner).
  Vertex colour white with alpha 1 - ((32.5 - i)^2 + (32.5 - j)^2) / 65^2: the peak lies half a
  cell off the eye toward +x and +z (0.9999 at the vertex over the eye; 0.75 at the middle of the
  low edges, 0.765 of the high ones; 0.50 .. 0.53 at the corners). Texture coordinates u = 7 i / 64
  (with +x), v = 7 (1 - j / 64) (with -z), so `data/enviroment/clouds.dds` repeats every 90.4
  units; then the `Skydome` entry's rotation a (90 degrees) turns the texture
  coordinates as u' = u cos a - v sin a, v' = u sin a + v cos a (at 90
  degrees (u, v) -> (-v, u)). The cap is depth-tested (less-equal), writes no
  depth, no alpha test, alpha blended source alpha / inverse source alpha;
  colour = vertex colour x lerp(T0, T1, b) (E-gfx-059, `enviroment_shader.fx` pass Skydome, which
  always mixes two textures): T0 is the sky texture of the day-table entry the clock is in (with
  that entry's rotation), b the day-time blend of 2.3c, and T1 the next entry's sky texture with
  its own rotation when the next entry names a different skydome, else T0 again with the same
  rotation (then the mix is T0 alone). With the default table this mixes from hour 22 to 23
  (clouds toward stars) and from hour 0 to 1 (stars toward clouds); at hour 0.275 (trace tick
  11 000) the cap is 0.725 x stars + 0.275 x clouds, over the background 0.275 (black toward
  white). Showing one texture or the other is wrong in those two hours (T44). The cap's edge
  lies 0.9 degrees below the horizontal at the middle of a side and 3.2 degrees at a corner
  (inside the 500-unit far plane), so it shows only where the top of the screen comes within
  about 1 .. 3 degrees of the horizon, at zoom 0 at most a thin band (2026-11-12 night: the
  earlier 3.8 / 4.0 degrees came from the misread size). The sun (and the moon
  while the light elevation is <= 0, never in the shipped data) is a quad
  drawn at the light direction x 100 from the camera with the sizes of
  `graphics.xml` (sun 30, moon 9), same blend, depth test always, no culling.
  **Which direction (Q-229 d; E-sim-002):** the vector is
  (0, 0, 1) turned by the heading and the absolute elevation, which gives
  the light's travel direction of 2.3, `(sin h cos e, -sin e, cos h cos e)`:
  the sun's quad is placed 100 units from the camera along the light's way
  down, so with a positive elevation it lies below the horizon and the
  terrain drawn after it hides it; the moon's quad takes the mirrored y (only
  while the elevation is <= 0). So the sun is never visible with the shipped
  day-time table, and a program that places it that way matches. Confidence:
  high for the vector, medium for the mirrored moon (never exercised).
  Confidence: high for the states and the rotation (E-pt2-001,
  enviroment_shader.fx); high for the cap geometry (the builder's arithmetic and the constants
  its only caller passes: 5000, 5010, 64 cells, 7 x 7 tiles; the +15 in the per-frame placement,
  E-gfx-059).
- **GFX-VIEW-6.3a Sky with the tables actually used (2026-10-24, GFX-VIEW-2.3b).** The
  "never" of 6.3 was read from `daytimes.xml`, which the game does not use. In the tables it does use, the skydome number is 1 (the
  `Stars` entry, `data/enviroment/starfield.tga`, rotation -45) at hours 23 and 0 of the
  default `daytimes.bin`, and at every hour on the maps with their own night tables
  (`campaign08`, `campaign10`, `MP_4P_Skull_Island`, `MP_4P_black_hole`, `sp04_lavaland`,
  `sp08_goldfever`, `sp10_rat_race`, `X_LavaDemo`, light elevation -36): there the cap
  carries the stars and, the elevation being at most 0, the moon (`data/enviroment/moon.dds`)
  is drawn. The `Cubemap` entry (`data/enviroment/cube_0.dds`) is not described here.
  Confidence: medium (the drawing rules above applied to the tables actually loaded); the
  table and the clock that select the entry are measured (2.3c: the default table's night
  entries are reached at hour 23, trace tick 3332 of a free game). Covers (inventory): *texture* from
  `data/enviroment/*` except `data/enviroment/cube_0.dds`.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| desired basis | origin 0, right +z, up +y, back +x; flags 3 | | high | E-gfx-005 |
| units per metre | settlers 1.2, deer/elk 1.5, rabbit 2.0, ships 1.0, else 1.0 | | medium | E-gfx-005, data |
| fine grid origin | -(W-1) sx / 2, -(H-1) sz / 2 | world | high | E-gfx-005 |
| normal packing | trunc(n x 127.5) | | high | E-gfx-005 |
| terrain cell | 4 x 4 fields | | high | E-gfx-005 |
| light direction | (0.5265, -0.7660, 0.3687) (elevation 50, heading 55) | | high | E-gfx-005, daytimes.xml |
| environment clock | dt x 0.033, start hour 12 | h | high | graphics.xml |
| camera fov (horizontal) / near / far | 50 deg / 1 / 500 | deg, world | high | E-gfx-009 |
| zoom steps / start / yaw | 0..4 / 3 (2 after a scripted move) / {-1.5, 0, 1.5} rad | | high | E-gfx-005, E-gfx-009 |
| camera pitch per zoom | y = -0.3, -0.6, -0.8 of a unit vector: 17.46, 36.87, 53.13 deg | deg below horizontal | high | E-gfx-009 |
| camera offset elevation | 1.4 x y: 23.8, 46.4, 61.8 deg | deg | high | E-gfx-009 |
| camera smoothing | yaw 0.245, direction 0.9 / 0.4, height 0.3 | per frame | high | E-gfx-005 |
| camera distance | 21 x zoom (0.3 x 70 x zoom) | world | high | E-gfx-009 |
| background colour | (0.8, 0.9, 1.0) | | high | daytimes.xml |
| sky plane | 64 x 64 cells, side 632.77 (2 sqrt(5010^2 - 5000^2)), height +15, y = -(x^2+z^2)/5010, alpha 1 - ((32.5-i)^2+(32.5-j)^2)/65^2, clouds tiled 7 (90.4 units), rotation 90 (corrected 2026-11-12 night) | world | high | E-gfx-059, graphics.xml |
| sun / moon sprite distance | 100 | world | high | E-gfx-009 |
| headings | 90, 150, -150, -90, -30, 30 deg; turn 0.4398 rad per tick | | high | E-gfx-005 |
| sinking building | height x t^2, t += 0.35 per s | | high | E-gfx-005 |
| building heading | -30 deg for every type (loader default, no XML override) | deg | high | E-gfx-008, E-gfx-011 |
| building height | the building field's sample height | world | high | E-gfx-011 |
| shadow factor | 1 - clamp((\|elevation\| - 20 deg) / 20 deg, 0, 1) x shadowIntensity = 0.1 | | high | E-gfx-011 |
| light direction (day table) | (sin h cos \|e\|, -sin \|e\|, cos h cos \|e\|); hour 12: (0.4924, -0.7660, 0.4132) | | high | E-gfx-036 |
| pause layer | stored v -> round(v x 90 / 255) (alpha byte 165) | | high | E-ui-014 |
| shadow test | depth + bias (-0.00325) < stored; 2 x 2 taps, bilinear | | high | shared.inc, graphics.xml |
| ambient / light luminance (alpha) | 0.5413 / 0.8965 (0.299, 0.587, 0.114 weights) | | high | E-gfx-011 |
| post effects | off unless started with -postFX | | high | E-gfx-011 |
| build-help icon | size 1.6, offset (0,0,-1), u by class, 50 per batch | | high | E-gfx-005, graphics.xml |
| cursor sprites | uiElements3 (224,96); (192,96); (288..480, 128); 32 x 32, anchor (22, 22) | px | high | E-gfx-005, E-gfx-007 |
| build-help atlas | data/map/textures/infomap.dds 512 x 64 | | high | E-gfx-007 |
| camera keys | Delete / Insert rotate, Page Down / Up or wheel zoom, arrows move, middle drag, right button mode | VK | high | E-gfx-007 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | fine grid 401 x 401 (map 100 x 100), sx 1.5, sz 1.2990375 | world of sample (0, 0) and (400, 400) | (-300, -, -259.8) and (300, -, 259.8); sample (1, 1) at (-297.75, -, -258.5) |
| T2 | model authored right +x, up +y, back +z, 1 unit/m; settler scale 1.2 | load | vertex (1, 0, 0) becomes (0, 0, 1.2); (0, 0, 1) becomes (1.2, 0, 0) |
| T3 | flat ground at height 0, water level 0 | camera at zoom 2, yaw 0, settled | target.y = 10; view direction (0, -0.8, 0.6) (53.13 deg down); camera 42 units from the target along -normalize(0, -1.12, 0.6): 19.9 behind, 37.0 above; camera.y >= 10 |
| T4 | yaw target 0, key rotate-left pressed once, then again | two frames | target +1.5 then 0; yaw moves 24.5 % of the remaining angle per frame |
| T4a | GFX-VIEW-3.2a: right button held, the mouse moved a tenth of the screen width to the right in one frame, nothing else | that frame, then the release | the view direction's yaw turns by 1 radian in that frame (no easing), the pointer is hidden; on release the pointer reappears and the view eases back to the previous direction by 0.4 of the difference per frame, the target unchanged |
| T4b | GFX-VIEW-3.2a: right button held, UP held for one frame of 0.04 s at speed 0.25 | one frame | camera and target move 2 x (5 x 0.25)^2 x 0.04 x 25 = 3.125 units along the horizontal view direction (if that lands over the map); after release the target stays there |
| T5 | zoom 4 | zoom-out key | stays 4; zoom-in key: 3 |
| T10 | 1024 x 768 window | projection | x scale 2.1445, y scale 2.8594, vertical fov 38.6 deg; 1920 x 1080: y scale 3.8125, 29.4 deg |
| T11 | target on the map's top edge field, zoom 4, 4:3, flat map | view | the ground beyond the edge is visible up to about 70 units out: black skirt (edge fields unexplored), no sky, no clear colour |
| T12 | zoom 0, 4:3 | view | pitch 17.46 deg; the top of the screen is 1.8 deg above the horizon: cloud cap visible in a band at the top, clear colour above its far-plane cut |
| T13 | new game, player has a headquarters on flat dry ground at height 5, field point (x0, 5, z0) | camera after the first frames | zoom 3, yaw 0, target (x0, 10, z0 - 18), camera (x0, 65.533, z0 - 47.752); HQ field point at screen height +0.093 of the half height above the centre at 16:9 (+0.070 at 4:3), horizontally centred (3.6) |
| T13a | same, zoom 2 / 4 after the start | HQ field point | +0.342 / -0.050 at 16:9 (zoom 4 puts it just below the centre) |
| T13b | loaded game whose headquarters was destroyed | first frame | no jump: target (0, 10, -10) floor-corrected, zoom 3 (3.6 a) |
| T6 | settler moving from field A to neighbour B in direction 0, progress 0.25 | visual | position = A + 0.25 (B - A); heading 90 degrees |
| T7 | daytimes.xml as shipped | any hour | light direction (0.5265, -0.7660, 0.3687), ambient (0.6, 0.5, 0.6) |
| T8 | field with build class 4 (large) in view | build help | billboard at field pos + (0, 0, -1), atlas u 0.0, size 1.6 |
| T9 | unexplored field | terrain pixel | colour x 0 (black); explored: x 1 |
| T10 | mouse over a crest, the ray grazes the near triangle | pick | the near triangle's hit, field = its nearest vertex; never a field behind the crest (1.5) |
| T14 | daytimes.xml as shipped, flat ground (N = (0, 1, 0)) | terrain D | lit (1.3182, 1.1879, 1.1983); in a shadow (0.6718, 0.5688, 0.6598); a texel (0.5, 0.5, 0.5) shows (0.659, 0.594, 0.599) lit and (0.336, 0.284, 0.330) shadowed |
| T15 | light elevation 50 / 40 / 30 / 20 / 10 deg, intensity 0.9 | shadow factor | 0.1 / 0.1 / 0.55 / 1.0 / 1.0 |
| T16 | Roman headquarters on field (10, 10) of a 100 x 100 map (fine grid 401 x 401), raw height 5000 there | world matrix | rows (0.866025, 0, 0.5, 0), (0, 1, 0, 0), (-0.5, 0, 0.866025, 0), (-240, 5.0, -207.846, 1); model point (0, 0, -1) maps to direction (0.5, 0, -0.866); its flag field (10, 9) is at (-237, -, -213.042), i.e. offset (3, -5.196) along that direction |
| T17 | same HQ | door bones | Door1 (model -1.034, -0.044, 2.165) at world (-241.978, 4.956, -206.488); Door2 (0.873, -0.045, 2.164) at (-240.326, 4.955, -205.535); GROUND corner (-6.619, 0, -2.173) at (-244.646, -, -213.037) |
| T18 | Roman woodcutter's hut on field (11, 11), raw height 4200 at sample (46, 44) | world matrix | rows as T16, translation (-231, 4.2, -202.6499); door bone (0.662, 0.02, -1.581) at (-229.636, 4.22, -203.688); the hut's base (model y -0.228) at y 3.972 |
| T19 | hut field sample 4200, its six fine neighbours at 4500 (a dip) | height | y stays 4.2 (the field sample), the base is drawn 0.3 below the surrounding ground; no other height is used |
| T11 | build-help icon on a field, camera above and in front | icon | the quad's bottom edge sits on the field point moved 1 unit toward the camera, 1.6 high, facing the camera; a hill between camera and icon hides it (depth test) |
| T20 | flat explored ground, sun (elevation 50), texels 40 / 128 / 220 (grey) | lit pixel (R, G, B) | (53, 48, 48) / (169, 152, 153) / (255, 255, 255) (2.6) |
| T21 | the same in a shadow | pixel | (27, 23, 26) / (86, 73, 84) / (148, 125, 145) |
| T22 | house wall facing the sun (N . L = 1), colour variation 1, light map 1 | pixel | (62, 56, 55) / (197, 179, 177) / (255, 255, 255); with light map 0.5: (62, 53, 52) / (197, 170, 166) / (255, 255, 255) |
| T23 | the same wall in a shadow, light map 1 / 0.5 | pixel | (28, 24, 27) / (89, 75, 87) / (153, 130, 149); with 0.5: (17, 13, 16) / (53, 41, 51) / (91, 71, 88) |
| T24 | settler, up-facing normal, palette value 128 | pixel | 128 x 1.2280 = 157 lit, 128 x 0.6100 = 78 in shadow (all channels) |
| T25 | flat explored ground in sun (D = (1.3182, 1.1879, 1.1983), 2.3), grass texel (80, 110, 40) giving the stored ground pixel d = (105, 131, 48); a road drawn over it, normal-map texel pointing straight up (N . L = 0.766 as for the ground), road texel (86, 71, 49) | stored pixel | source rgb = (113.4, 84.3, 58.7) / 255. Alpha 153 (nibble 9, the strip's centre): (110, 103, 54); alpha 51 (nibble 3, near the edge): (107, 122, 50); alpha 0: (105, 131, 48) unchanged. Blending in linear light would give (110, 106, 55) and (107, 123, 50): wrong (2.6 c1) |
| T26 | same ground pixel d = (105, 131, 48), woodcutter decal's front border (GFX-OBJ-2.3a, T3g) at model z = -2.50 / -2.70 / -2.85 / -3.00 / -3.15 / -3.35, texels (88, 77, 45) a 255 / (105, 91, 47) a 204 / (78, 69, 32) a 136 / (97, 86, 44) a 85 / (94, 84, 47) a 34 / (114, 103, 60) a 0 | stored pixel | (116, 92, 54) / (132, 113, 55) / (104, 105, 43) / (113, 121, 50) / (108, 127, 49) / (105, 131, 48), each within 1; in linear light the middle three would be (133, 113, 55), (104, 108, 43), (113, 122, 50) (2.6 c1) |
| T27 | ground (100, 100, 100); a strip pixel (200, 0, 0) with alpha 0.5 and a decal pixel (0, 0, 200) with alpha 0.5 at the same place (values as blended in the frame buffer, 2.6) | draw the frame | (75, 25, 125): the strip first gives (150, 50, 50), then the decal; the reverse order would give (125, 25, 75) |
| T28 | free game on MP_2P_Storm_Coast (no map table: `daytimes.bin`), `DayNightCycle` on, fixed 0.025 s frames | environment clock and light by trace tick (2.3c, E-trace-007) | tick 1: clock 12.0, elevation 50, heading 50; tick 1001: 15.3005, noon values, blend 0.4126; tick 3001: 21.9084, elevation 17.29, heading 25.46 (between hours 21 and 22); tick 3332: 23.0019, skydome on; tick 3601: 23.8906, elevation -28, heading 3, background black; tick 3937: 25.0014, skydome off; tick 4201: 25.8735 (hour 1.8735), elevation 37.47; tick 6000: 31.8141 (hour 7.81), noon values |
| T29 | 5.3: the default two-player free game on MP_2P_Storm_Coast (player 1 a Weak computer player, the game of E-trace-014) set up with fog of war Off (E-trace-017) | its recording to counter 11 099, against E-trace-014 (fog of war On) | the generator's state, the next object number and the draw count are the same at every counter 0 .. 11 099; the buildings, settlers and flags tables are the same at all 1095 sampled ticks (every 10th). The pictures `weakfogoff-*` of look-compare-auto 2026-10-30 show player 1's land, the farm 90538 and the Storm Coast beach at cell (6, 3) lit, which the fog-on game (E-trace-016, `weak-*`) draws black for player 0 |
| T30 | 2.3d: shipped `daytimes.bin`, environment clock 48.28 (hour 0.28; the Weak game near trace tick 11 000) | the light | ambient (0.2236, 0.3168, 0.4890), light (0.2969, 0.3167, 0.5849), elevation -14.56, heading 7.76, shadow intensity 0.612, direction (0.1307, -0.2514, 0.9590) (from above, not below), k 0, s = 1; flat ground D = (0.2983, 0.3964, 0.6361) lit and in a shadow alike |
| T31 | 2.3d: clock 24.0 (hour 0 exactly) | shadow factor and flat ground | \|e\| 28, intensity 0.5: k 0.4, s 0.8; N . L 0.4695; D lit (0.3135, 0.4558, 0.8699), in a shadow (0.2947, 0.4344, 0.8135) |
| T32 | 2.3d f, GFX-OBJ-2.4: hour 12, a tree vertex facing up (N . L 0.766), texel (40, 80, 30), vertex colour 1 / 0.25 | tree pixel | lit (53, 95, 36) / (31, 54, 22); in a shadow (27, 46, 20) / (25, 41, 18). Multiplying the whole D by the vertex colour would give (13, 24, 9) for 0.25 lit: wrong |
| T33 | 2.4a: z_pavement_00 pixel (intensity 0.6, power 5), normal-map texel straight up with alpha 1, flat ground at hour 12 lit (D (1.3182, 1.1879, 1.1983)), tangent space: N = L = (0, 0, 1) as a formula check | specular add | R = (0, 0, 1); V = (0, 0, 1): S = 1, +0.6 on every channel; V = (0, 0.6, 0.8): S = 0.8^5 = 0.3277, +0.1966 |
| T34 | 2.4a: z_rock_01 (type 3, no intensity in the table) and m_rock_03 (intensity 0.1, type 0) | specular add | 0 for both |
| T35 | 2.3d f: a z_sand pattern (type 0) vertex at world height -0.5 | vertex colour | white (1, 1, 1, 1): no haze tint (the haze colour is computed only for type-1 patches) |
| T36 | 5.4 (Q-293 b): E-trace-017 (the Weak game, fog of war Off), the shots run's farm scene at trace tick 6000 (paused, the camera moved there from player 0's castle by the scene's camera move) | the constructors on the screen | 90640 (appeared at counter 4729 on (139, 52), at (135, 58) fine (543, 234) working, step -1, since before 5990) is not drawn; with a seen mark set at its appearance or refreshed at the camera move it would be. 90571 ((138, 54), working) is outside the picture (corrected 2026-11-08 b) |
| T36a | 5.4 (e), (g): the same recording played to counter 5999 with the camera on player 0's castle (zoom 3) from 1499 on, then the farm scene's camera move made paused | the settlers drawn in the farm picture | drawn: leveller 90750 (walking, step 0.18, at about (1840, 1110) of 2560 x 1440) and carrier 90554 (walking); not drawn: constructor 90640 ((135, 58), working) and worker 90480 ((139, 61), state 36, standing) |
| T38 | 2.2, 2.1a: a grid normal straight up (0, 1, 0); one of (0.6, 0.8, 0) | stored bytes, decoded normal, tangent bytes | up: bytes (127, 255, 127), decoded (-0.0039, 1.0, -0.0039), tangent about (1, 0.004, 0) with bytes (254 or 255, 127 or 128, 127) (the last bit depends on float rounding; accept 1 step); (0.6, 0.8, 0): bytes (204, 229, 127), decoded (0.6, 0.7961, -0.0039), tangent (0.7986, -0.6019, 0.0030), bytes (229, 50, 127) |
| T39 | 2.1b: E-trace-017 (Storm Coast, 170 x 170 fields, 681 x 681 samples), heights at trace tick 0 and at 6000 (= 11 100 there), hour 12 light (2.3d e), a flat normal-map texel | the terrain's normal at samples (544, 228) and (540, 231) at 6000 | (544, 228) inside the levelled site (heights around it all 7366 at 6000; 6290 / 7790 / 6140 / 7340 at the start): the start's normal (0.4133, 0.8267, 0.3818), bytes (180, 232, 176), N . L 0.27, not the current (0, 1, 0) with 0.766; (540, 231): start normal (0.2133, 0.9326, 0.2911), N . L 0.49. The lit ratio start / current over samples (539 .. 546, 226 .. 232) outside the farm's decal: 0.65 .. 0.92 (the band measured 0.76 .. 0.88 of ours) |
| T41 | 2.4b: `z_seaground_00_n.dds` (512 x 512, 4:4:4:4) on flat ground (T = (1, 0, 0), N = (0, 1, 0), B = T x N), light direction (0.492404, -0.766044, 0.413176), light colour (0.9375, 0.898, 0.781), ambient (0.6, 0.5, 0.6) | mean saturate(N . L) over the map, N = 2 x texel - 1 filtered bilinearly at random points; D = light x that + ambient | 0.567 with N as filtered (the original), D (1.132, 1.010, 1.044); 0.629 with N normalised after the filter; 0.682 with the map resampled to 256 x 256 and N normalised, D (1.239, 1.112, 1.133) = 9.5 % brighter. The same with direction (0.129134, -0.255417, 0.958168): 0.232, 0.258, 0.231 |
| T40 | 2.1b: the same map, a height change at sample (100, 100) and one at (540, 230) | which terrain normals change | (100, 100): the refreshed corner (columns and rows 0 .. 170): the terrain's normal follows the new heights; (540, 230): only the position and the grid normal (objects, camera) change; the terrain keeps the start's normal and tangent |
| T42 | 2.5 (e): the profile's shadows switch off (or `SHADOW QUALITY` moved to 0 on the settings page); noon light (2.3) | a grass pixel under a tree's shadow; a water pixel with no caster above it, every depth bin | the shadowed grass pixel reads like unshadowed grass of the same slope (D = light x saturate(N . L) + ambient); the water pixel reads the same as with shadows on; no pixel anywhere is darker than with shadows on (only formerly shadowed ones change). With the switch set by a preset: on exactly when `SHADOW DETAIL` is above 1/6 (Medium, High, VeryHigh), off for VeryLow and Low |
| T43 | 2.5 (f): the Weak game (fog of war off) at tick 6000, noon light (direction (0.4924, -0.7660, 0.4132), light (0.9375, 0.898, 0.781), ambient (0.6, 0.5, 0.6)), shadows on with a 2048 map; the town camera of the look of 2026-11-12 (eye (325.2, 68.8, -206.8), direction (0, -0.8, 0.6), 50 degree horizontal field of view, 2560 x 1440) | each house's pixels grouped by the face's direction in the model; the program's mean against the original's | the Roman woodcutter's hut on (142, 56): door side (model -z and up to 15 degrees from it) 0.79 .. 0.81 as bright as the same wall lit without a shadow (the program's picture of 2026-11-12), i.e. in the overhang's shadow over most of the wall; its roof and its -x side as the original within 1 .. 3 %. The quarries on (140, 57) and (139, 55): door side 0.89 .. 0.90; -x side within 1 .. 3 %. The headquarters on (139, 51): every face within 5 % |
| T44 | 6.3 (corrected 2026-11-12 night): the sky cap with the camera's eye at (0, 50, 0); the default day table | vertex positions, alphas, texture coordinates; the cap's colour at hours 12, 22.5, 0.275 | vertex (0, 0) at (-316.39, 25.04, -316.39) relative to the world (15 - 39.96 above the eye's height), alpha 0.500, (u, v) before the turn (0, 7); vertex (32, 32) at (0, 65, 0), alpha 0.99988, (3.5, 3.5); vertex (64, 32) at (316.39, 45.02, 0), alpha 0.765; vertex (32, 0) alpha 0.750. Colour: hour 12 clouds alone; hour 22.5 0.5 clouds + 0.5 stars; hour 0.275 0.725 stars + 0.275 clouds (each texture with its own rotation), never one texture alone in hours 22 .. 23 and 0 .. 1 |
| T37 | 3.7: flat ground at height 5; a settler standing at P = (10, 5, 10) facing +x (F = (1, 0, 0)), S = (0, 0, -1); a fresh game (flag behind) | O on him, then 200 frames | the wanted eye is (6, 7, 9) looking at (10, 7, 10); after 200 frames the eye has closed all but 0.95^200 of the gap; the zoom level is unchanged; PAGE DOWN: the wanted eye becomes (15, 7, 9) looking at (10, 6.25, 10); PAGE UP twice: back to behind, then the follow ends (mode 0). The same settler on ground at height 0: the eye is held at height 3 (water level 0 + 3) instead of 2 |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-gfx-059 | code reading | (session 2026-11-12 night) the sky cap's builder and its only call (5000, 5010, 64 cells, 7 x 7 tiles), its per-frame placement (+15 over the eye), `enviroment_shader.fx` (two textures always mixed by the day-time blend), the sky draw's choice of the second texture; the shipped day table | 6.3, T44; `GFX-water.md` GFX-WATER-2.6c (f) |
| E-gfx-058 | a look of the original compared with the program's picture, and code reading (pictures kept outside the repository) | (session 2026-11-12 night) the town picture at tick 6000 against the program's at the same camera, each house's faces projected and grouped by direction and by the sun's true and model-frame incidence; the house pass and the shadow frame read for the house normal and the colour lookup | 2.5 (f), T43; `GFX-objects.md` GFX-OBJ-2.2b |
| E-gfx-056 | code reading and a look of the original (pictures of the original kept outside the repository) | (session 2026-11-12) the scene settings' setters and getters, the presets, the settings page's slider handlers, the draw pass's shadow switch, the shadow map's size setter; every shipped `.fx` file's shadowed and shadowless techniques compared pass by pass | 2.5 (d), (e), T42; `SET-settings.md` SET-4 |
| E-gfx-050 | data and looks | (session 2026-11-11, no bundle) the look comparison of 2026-11-11, farm pairs at 1500, 6000, 11 000: the three cameras, the box (1450, 600, 120 x 80) ray-cast onto the ground, the map file's doodads under it, means of the box and of three neighbouring ground boxes in both pictures | GFX-VIEW-2.4 "The look comparison's farm dirt box" |
| E-gfx-046 | data and looks | (session 2026-11-09 c, no bundle) `terrain_shader.fx` TerrainPixelShader (N = 2 x NMap - 1, no normalize; only L normalised), `shared.inc` (sampler 2: linear / linear / linear, wrap); the normal maps' DDS headers (`z_seaground_00_n` 512 x 512 A4R4G4B4 with mips); the investigation's normal map model (T41) and depth bins (the 2026-11-09 c pairs binned by ground height and pattern) | 2.4b, T41 |
| E-gfx-043 | code reading | (session 2026-11-09): E-trace-017 `heights-full-000000.bin` / `-011100.bin` (T39) | Q-1035 |
| E-gfx-041 | code reading | (session 2026-11-08 b): E-trace-017 drive.log, settlers.csv, heights.csv | 5.4 (d) .. (g), 2.2 |
| E-gfx-042 | code reading and looks | (session 2026-11-08 c): the map renderer's terrain cull, the min / max of the drawn cells' field ranges, the inclusive field test; live read of the view rectangle in E-trace-021 (`view_rect.tsv`) | 5.4 (f) |
| E-gfx-040 | code reading | (session 2026-11-07) | 5.4 (d) |
| E-gfx-037 | code reading | | 5.4, T36 |
| E-gfx-036 | code reading + data | the blend of every value, direction from \|elevation\|; k from \|elevation\|; constants unchanged on the shader path; shadow technique always `Default`; haze colour only for type-1 patches, else white; MATERIAL semantic, 48 bytes; `terrain_shader.fx`, `tree_shader.fx`, `house_shader.fx`, `shared.inc`; `daytimes.bin`; heap dump E-trace-007 500 (pattern material values) | 2.3d, 2.4a, T30 .. T35 |
| E-trace-016, E-trace-017 | recordings and pictures | E-trace-016 (the Weak game, fog of war On, tables every 10th tick to 11 100, generator every tick, shots at 1500 / 6000 / 11 000) and `E-trace-017/` (the same game set up with fog of war Off, with the farm and cell (6, 3) scenes; `probe/` the set-up page's drop-down measurement; `SHA256SUMS-pictures`), recorded 2026-10-30 01:06 .. 01:19 and 01:25 .. 01:43 with nobody at the PC; pictures with the look comparison of 2026-10-30 | GFX-VIEW-5.3, T29; `TOOL-trace.md` TRACE-8 |
| E-gfx-030 | code reading | the map pass: terrain, street pass, borders, decals, buildings; the strips registered in the street renderer | ground layer order (2.7) |
| E-gfx-028 | code reading | `street_shader.fx`, `house_decal.fx`, `shared.inc` (ZENABLE, NOZWRITE, ABLEND, NOATEST); `house_decal.dds` / `simple_street.dds` DXT3 alpha | blend and alpha states (2.6 c1), decal edge (GFX-OBJ-2.3a) |
| E-gfx-021 | code reading | state-setting call sites (no SRGBWRITEENABLE, no SRGBTEXTURE); `house_shader.fx`, `terrain_shader.fx`, `street_shader.fx`, `house_decal.fx`, `shared.inc` | colour pipeline (2.6), house light map binding (GFX-OBJ-2.1) |
| E-gfx-019 | code reading | | start camera, jump offset z - 18, camera z easing, H key reset |
| E-cam-004 | code reading | | GFX-VIEW-3.7, T37 |
| E-gfx-005 | code reading | `S2DNG.exe` | model load and basis conversion |
| E-gfx-005 | code reading | | world frame, normals, ground height |
| E-gfx-005 | code reading | | terrain cells |
| E-gfx-005 | code reading | | daytimes and light direction |
| E-gfx-005 | code reading | | camera |
| E-gfx-005 | code reading | | headings and movement interpolation |
| E-gfx-005 | code reading | | building visuals |
| E-gfx-005 | code reading | | world texture and fog |
| E-gfx-005 | code reading | | build help and cursor icons |
| data | shaders | `data/gfx/fx_20/shared.inc`, `terrain_shader.fx`, `house_shader.fx`, `infomap_shader.fx`, `tree_shader.fx`, `doodads_shader.fx` | lighting, fog, frame technique |
| data | settings | `graphics.xml`, `daytimes.xml`, `buildings.xml`, `animals.xml`, `ships.xml`, `items.xml`, `character/settler_config.xml` | scales, light, icons |
| E-gfx-007 | code reading | | keys, options, cursor anchor, vertex colour |
| E-gfx-009 | code reading | | camera init (fov 50, near 1, far 500, zoom 3), pitch by re-normalised base vector, scroll limit |
| E-gfx-009 | code reading | | view / projection matrices, horizontal fov, effect semantics |
| E-gfx-009 | code reading | | sky plane geometry, position, draw order, sun / moon |
| E-gfx-009 | code reading | | skirt passes; far-plane flag default |
| E-gfx-011 | code reading | | building heading, matrix, height, sink, rising-wall plane |
| E-gfx-011 | code reading | | daytime to shader constants: shadow factor 1 - k x intensity, luminance alphas |
| E-gfx-011 | code reading | | shadow renderer (frustum fit, ortho / LiSPSM), debug toggles, -postFX switch |
| data | shaders | `shadow.fx`, `skin_color_mesh.fx`, `postprocess.fx`, `house_decal.fx` | casters, settler luminance light, post effects, decal states |
| E-pt2-001 | code reading | | build-help quad, picking, decal, sky dome states, jump to field |
| data | shaders | `water_shader.fx` lines 6-10 (address modes), passes RenderMapBorder*, `shared.inc` | skirt sampling clamp, fog settings unused |

## Open questions

- Q-083: answered. Heading -30 degrees for every building (4.3); the stages
  are GFX-OBJ-2.6 to 2.6d.
- Run A leftovers: the shadow map's size per `SHADOW QUALITY` step and the
  default of the ortho / LiSPSM flag (2.5 d).
- Q-084: answered in `GFX-objects.md` GFX-OBJ-1.4 (per-element rotation
  and scale ranges); the growth scaling is GFX-VIEW-4.5.
- Q-085: the function that writes a field's fog-of-war alpha into the world
  texture as the player's explored map grows, and what the separate
  map-sized "FogOfWar" texture is used for.
- Q-086: answered: 70 is a distance base (21 units per zoom step); the field
  of view is 50 degrees horizontal (GFX-VIEW-3.1, 3.5).
- Q-093: answered: near 1, far 500 (GFX-VIEW-3.1); option defaults in
  `SET-settings.md` SET-2.
- Play-test 2 chunk 1: GFX-VIEW-6.2 restricted to road building; the
  everyday cursor icon is UI-GAME-13 (E-ui-005).
- Q-189 (a): answered in GFX-VIEW-6. Open: the setter of the far-plane
  doubling flag, the look-at up vector, the skydome rotation sign, the
  post-effects (bloom / depth of field) of `graphics.xml`.
