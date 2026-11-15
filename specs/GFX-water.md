# GFX-WATER: How the game draws water

Status: draft (read from the shipped settings and shader files and the binary; no in-game screenshot compared)
Owner question(s): none (plan request: water rendering)
Game build: 11757

## Summary

Water is not a terrain type. A field is water when its ground lies below the
water level (`MAP-grid-geometry.md` MAP-GRID-7); the ground under the sea is
ordinary terrain, normally painted with the sea-ground patterns, and is drawn
with an underwater look (depth haze, wobble, light caustics). On top of it the
game draws a flat, transparent, animated water surface at height -0.1 (raw -100), one
rectangle per terrain cell (4 x 4 fields) that reaches below the water line,
plus a 200-unit wide strip of water around the map edge. Colours, opacity and
wave textures come from a named water style in `data/settings/graphics.xml`;
every release map uses the style `Default`.

## Rules

### GFX-WATER-1 Which fields are water

- **GFX-WATER-1.1** The game rule for water is height-based only: a field is
  water when its height is below -100 (MAP-GRID-7, ECO-PLACE-1.2). The terrain
  type of the field does not matter. In the 34 release maps 98% of the water
  fields carry a sea-ground pattern (`seaground`, `seaground plants`,
  `seaground plants rock`, `seaground rocky`, `seaground rock`, `seaground sand`,
  the `m_seaground` pair), the rest a land pattern; 21,800 land fields also
  carry a sea-ground pattern (beaches, shallows). The pattern `water`
  (`z_water_00`) is defined but used on no field. Confidence: high.
- **GFX-WATER-1.2** The renderer does not use the Water field state. It decides
  per terrain cell from the raw heights (GFX-WATER-3.1), so the water surface
  can also cover low land next to the coast inside the same cell; it is hidden
  there because the land lies above the surface. Confidence: high.

### GFX-WATER-2 The ground under water

- **GFX-WATER-2.1** Every pattern entry in `terrain_static_data.xml` has a
  render `type` (default 0). Type 1 is the underwater ground. All sea-ground
  entries (`z_seaground_00` to `05`, `m_seaground_00`, `01`) and `z_water_00`
  have type 1. Other types: 2 swamp (reflecting), 3 specular (rock, snow,
  pavement, some `l_` grounds), 4 and 5 lava. Their textures and texture
  scales follow `GFX-terrain-patterns.md` unchanged. Confidence: high.
- **GFX-WATER-2.2** Every ground vertex gets a depth haze colour from its world
  height `y` (raw height x 0.001, MAP-GRID-5.2):
  1. `d = (y + 0.05) / 7`.
  2. If `d >= 0` (that is `y >= -0.05`): no haze (colour white, weight 1).
  3. If `d <= -1` (`y <= -7.05`, raw -7050 and lower): the vertex colour is
     **0 in all four channels**: haze colour black with weight 0, so the ground
     there is drawn plain black, not with the deep colour (corrected 2026-11-07,
     E-gfx-040: that case returns before the colour is mixed; the deep
     colour is only approached, never reached, by the vertices above it).
  4. Otherwise `d = sin(d x pi/2)` (then between -1 and 0), multiplied by 0.97;
     if `y < -0.1`, `d` is limited to the range -1 .. -0.07 (so any vertex more
     than 100 raw units deep gets at least 7% haze).
  5. With `k = -d` (0..1): haze colour = shallow + (deep - shallow) x k,
     weight = 1 - k.
  Shallow and deep colour come from the water style (GFX-WATER-5): `Default`
  shallow (0.0, 0.8, 0.3), deep (0.0, 0.0, 0.05). Confidence: high.
- **GFX-WATER-2.3** A type-1 ground pixel is the lit ground texture (as any
  other ground: texture x (light x normal-map term + ambient)) blended toward
  the haze colour: `result = haze + (lit - haze) x weight`. Unlike other types,
  the vertex colour does not multiply the texture. Confidence: high (shipped
  shader `terrain_shader.fx`, BaseLayerCaustics / TransitionCaustics passes).
- **GFX-WATER-2.4** Type-1 ground wobbles: its texture coordinates (colour and
  normal map alike) are shifted by `s += A sin((t + (z + y) / 0.4) x 2.25)` and
  `t_coord += A sin((t + (x + y) / 0.4) x 2.25)` where `x, y, z` is the world
  position of the vertex, `t` the render clock in seconds and the amplitude
  `A` is 0 above `y = -0.15`, `-y x 1.42 x 0.008` between -0.15 and -0.7, and
  0.008 below -0.7. Confidence: high for the shader. **The clock (answered 2026-11-08 b, Q-055;
  E-gfx-041; E-tool-005):** the shaders' timer is set at each
  frame's start from the scene clock: `t` (its second value, the one every water, caustics and
  lava formula reads) is the game's accumulated game time in seconds (`TOOL-trace.md`),
  its first value the frame's game step. So the wobble, the caustics and the wave drift
  stop while the game is paused and run 2x / 3x at speeds 2 / 3; outside a game (no world) the
  scene adds the real frame time instead.
- **GFX-WATER-2.5** Caustics: an extra additive pass over the underwater
  ground uses `data/map/textures/caustics.tga` (2048 x 128, a strip of 16
  frames of 128 x 128). Texture coordinates (Q-229 b, read again from
  `water_shader.fx`): the ground vertex's own texture coordinate (u, v) gives
  `u' = u / 16 x 0.04`, `v' = v x 0.04`; frame `f = floor(16 frac(0.2 t))` and
  `w = frac(16 frac(0.2 t))`; the pass samples the strip at `(u' + f / 16, v')`
  and at `(u' + f / 16 - 1 / 16, v')` **literally** (the offset is added to
  the wrapped coordinate, the strip wraps as a whole, nothing is clamped to
  the frame's slot), and mixes the two red channels as
  `r(f) x w + r(f - 1) x (1 - w)`, so the strip loops every 5 s. Brightness:
  that mix x light colour x 0.7 x `V` x `V` x `(1 - min(1, -0.15 y))`,
  with `V = 1 - sin(0.4x + s) sin(0.4y + s) sin(0.4z + s)` and `s = sin(t)`
  (V enters twice: once in the per-frame weights, once on the result);
  nothing above `y = -0.1`. Fog of war darkens it like the ground. The pass
  is drawn by the map renderer when the **shader detail** setting
  (`SET-settings.md` SET-4: the shader detail u32, 1 off, 2 low, 3 high; the
  options page's `!SHADERDETAIL` drop-down, Q-229 c) is at least 2 and the
  frame's draw flags ask for it: the visible terrain is drawn a second time
  with the `RenderUnderwaterSurface` technique, before the water surfaces.
  Confidence: high for the shader (`data/gfx/fx_20/water_shader.fx`,
  `VS_UnderwaterSurface` / `PS_UnderwaterSurface`); medium for the meaning
  of the draw flag.
- **GFX-WATER-2.6 Mirror (answered 2026-11-01, T16).** Each water quad is
  drawn with `RenderWater` (with the mirror image) when the **mirror switch**
  is on, otherwise with `RenderWaterNoMirror`. The switch is the
  `SceneSettings` bool written 13th in the settings file (`SET-settings.md`
  SET-4: the bool after the two f32 that follow the three bools; on in the
  sample file, with the graphic quality preset VeryHigh); the scene copies it
  into its water draw flags whenever it applies the settings. The shader
  level does not choose the technique (only the caustics pass of 2.5 needs
  level 2). So with those settings, which the original's pictures were taken
  with, the water is `RenderWater`. Confidence: high (E-gfx-038).
  **The mirror image.** While the switch is on, each frame before the main
  view the scene is drawn once more into the mirror texture, from the mirror
  camera, with a clip plane at the water level (y = the level of 3.3, -0.1;
  the spec said 0 until 2026-11-09) that keeps the camera's side, and with neither the water
  surfaces nor the caustics pass (the water draw is skipped in that pass).
  It draws the mirror's own set of object kinds, or the land alone when no
  water cell is in view. The texture is 128 x 2^q texels wide, q = trunc(4 x
  the 14th field, the mirror quality f32 0..1), limited to 0..4 (quality 1.0:
  2048), halved until it is no wider than the back buffer, and half as high
  (8-bit RGBA with its own depth buffer). It is made again when q changes and
  not made at q = 0. `RenderWater` reads it at the surface point's position on
  the screen (the projected position x 0.5 and y x -0.5, plus 0.5: the
  screen's top left is texel (0, 0)) and mixes it as the shader's mirror term.
  Confidence: medium (E-gfx-038).
- **GFX-WATER-2.6a The mirror camera (answered 2026-11-06, Q-1018; E-gfx-039).** The mirror pass does not use a second camera of its own: it
  wraps the main camera and changes three things.
  1. **View:** the scene is reflected in the water plane first, then seen with the main camera's
     own view of this frame: a world point (x, y, z) is drawn where the main camera would see (x,
     2 x level - y, z) (level -0.1, 3.3). So the mirror picture lines up with the
     screen: it is **not** turned left to right, and the water reads it at its own screen position
     (2.6), with no 1 - x.
  2. **Field of view:** 1.03 times the main camera's (horizontal) field of view, everything else
     (aspect, near, far, eye, target) the main camera's. The picture is therefore drawn about 3 %
     smaller around the screen centre than an exact reflection, while the water still looks it up
     with the main camera's projection (the view-projection in force at the start of the frame,
     before the mirror pass).
  3. **Culling:** the reflection reverses the triangles' winding, and the pass reverses the cull
     mode with it (counter-clockwise faces culled instead of clockwise). The net effect is the
     usual one: faces turned away from the reflected view are dropped, so the land's upper side
     is not seen in the mirror.
  The clip plane is the world plane y = level, keeping the side of the main camera's eye (above
  the water for every game camera): only what lies above the water is reflected.
  Confidence: high (code).
- **GFX-WATER-2.6b What the mirror draws (answered 2026-11-06, Q-1018; E-gfx-039).** Every frame the game picks the mirror's object set from the main view's
  direction and the `MIRROR DETAILS` setting (the `SceneSettings` f32 written 15th, `SET-settings.md`
  SET-4, 1.0 in the sample file):
  - If the main camera looks down more steeply than 30 degrees (its forward vector's y below -0.5):
    **the sky alone** (the cloud cap and the sun of `GFX-world-view.md` GFX-VIEW-6.3). This holds
    for the default camera (45 degrees down).
  - Otherwise with n = round(4 x details) (0..4): n = 0 the sky alone; n >= 1 sky and land; n >= 3
    also **the settlers** (the scene's draw bit 8 is the settler renderer: every settler with what
    it carries, taken from the main view's list of settlers to draw in that frame, so with the same
    seen marks and drawn area as `GFX-world-view.md` GFX-VIEW-5.4; answered 2026-11-07, Q-1028,
    E-gfx-040); n = 4 everything the main view draws except the water (sky,
    land, every object family).
  - With no water cell in view the pass draws only the sky (the old "land alone" was wrong).
  Neither the water surfaces nor the caustics are drawn in the pass.
  Confidence: high for the choice (code); medium for the families behind the draw bits (only bit 1
  sky and bit 0x20 land are named).
- **GFX-WATER-2.6c The mirror's background (answered 2026-11-06, Q-1018; E-gfx-039).** The mirror texture is cleared as the frame is, to the day table's background colour
  (GFX-VIEW-6.1, (0.8, 0.9, 1.0) at noon), and the cloud cap of GFX-VIEW-6.3 is drawn over it,
  reflected below the water. So over open water, seen from the default camera, the mirror image is
  the **cloud cap**: `data/enviroment/clouds.dds` (a blue sky with white clouds, mean colour (0.66,
  0.81, 0.90)) times the cap's vertex alpha over the background. The sun's quad lies below the water
  plane in world space and is clipped. Confidence: high for the order and the clip; medium for the
  clear (the pass calls the same frame start as the main view, whose clear was read for
  GFX-VIEW-6.1).
  **What this does not explain (T16a).** Against the program's stand-in (the plain background as
  mirror image) the cloud cap is darker in red and green by more than in blue, which moves the
  measured water toward the original; it cannot by itself give the original's +4 levels of blue
  over open water with red and green unchanged, and the night pictures (where the mirror weighs
  little, s = 0.8 x D_red) are 1.4 levels short of blue too. The rest of the water pixel is the
  ground (2.2, 2.3), the caustics (2.5) and the surface with its alpha (4.3, 4.4); no further term
  was found in code. What would settle it: a picture of the original with the mirror switch off
  (13th settings bool) beside one with it on, same camera and tick.
  **Searched again 2026-11-07 (T16a, E-gfx-040).** No
  further term: the surface's colours, opacity (0.85, 0.94), reflection factor and noise are the
  style's values, set before every water draw (the shader file's commented defaults are not used);
  the haze colours are the style's, unlit, packed without a channel swap; no post effect runs. The
  measured gap is in proportion to the light (blue: original / ours 1.081 at noon, 1.084 at night;
  red and green 0.97 .. 0.98), and over open water the surface weighs only about (with the old reading of 4.4, corrected 2026-11-09) 0.2 x (0.94 - 0.85
  x |V_up|), near 0.07 from the default camera, of which the mirror takes at most 0.8 x D_red: the
  mirror cannot move the pixel by 3.5 levels of blue. The difference lies in the **ground under the
  water** (the lit sea-ground texture with its haze, 2.2 and 2.3, and the caustics, 2.5). The
  original's sea ground in the noon picture (`look-compare-auto/2026-11-07` weakfogoff-001500-cell63,
  region water) shows a fine, low-contrast texture; ours shows a coarse zig-zag pattern there, so
  compare the ground pass first (texture scale and wobble of 2.4, the haze per vertex of 2.2).
  **The mirror's share measured (2026-11-08 c, T16a; looks E-trace-021 mirror on, E-trace-022
  mirror off, against E-trace-017).** The original was run twice more to tick 1500 of the same
  game (fog of war off), once with the mirror switch off (the 13th settings bool, read back as 0 in
  the running game) and once on, each with the cell63 picture. The camera moves in steps of
  1.953125 units a frame and its ground height settles only while the game runs, so the two new
  cameras were near, not equal to, E-trace-017's; each picture was therefore resampled to
  E-trace-017's camera through the water plane (level 0). The resampled mirror-on picture repeats
  E-trace-017 to within 0.6 of a level over open water (water region 47.6, 59.1, 46.9 against
  47.7, 59.1, 46.7), and the sand and rock regions agree in all three pictures, so the resampling
  holds. Region means (red, green, blue; tick 1500, noon, default camera's pitch at zoom 2):

  | Region (of E-trace-017's picture) | mirror off | mirror on | the mirror's share |
  |---|---|---|---|
  | water (100, 1150, 500, 200) | 42.3, 55.1, 44.1 | 47.6, 59.1, 46.9 | +5.3, +4.0, +2.8 |
  | open water (20, 1000, 250, 200) | 44.6, 56.7, 45.7 | 50.1, 60.9, 48.5 | +5.5, +4.2, +2.8 |
  | open water (300, 1250, 300, 150) | 41.1, 54.6, 43.1 | 46.2, 58.6, 45.9 | +5.1, +4.0, +2.8 |
  | shore film (600, 850, 300, 100) | 60.2, 52.5, 37.5 | 60.5, 52.4, 37.3 | none |
  | sand (1000, 600, 300, 200) | 71.7, 57.0, 33.0 | 71.6, 56.8, 33.0 | none |

  So over open water the mirror adds a **warm, pale** light of about +5.3 / +4.0 / +2.8 levels (red
  most, blue least: the cloud cap's colour times the warm light colour (0.94, 0.90, 0.78) and the
  surface's weighting), and nothing over the shallow film. The rest of the original's water pixel
  (mirror off) is 42.3, 55.1, 44.1. Against the program (11-06 measure: without its mirror 47.2,
  59.0, 42.4; with it 49.7, 60.8, 43.9): **the program's pixel without the mirror is too red and
  green by about 5 and 4 levels and short of blue by about 1.7**, and **its mirror adds about half
  the original's share** (+2.5, +1.8, +1.5). Both parts need work: the ground under the water
  (2.2, 2.3, 2.5) is the larger error. Confidence: high for the measured shares (three regions
  agree within 0.4); the split of the mirror-off remainder between ground, caustics and surface is
  not measured.
  **The split (2026-11-09, Q-1018).** Two things change the reading of the table: (1) the
  pictures are **paused**, so every value in it is 90 / 255 = 0.353 of the frame (`UI-ingame.md`
  UI-GAME-1.11, T46; "Paused" is in each picture): the frame's own mirror share is +15.0 /
  +11.3 / +7.9 and its mirror-off pixel 119.9, 156.1, 125.0; (2) the surface normal is the
  unnormalised mean of 4.2. A per-pixel model of the region (E-trace-017's camera; the surface
  point at level 0 and the sea ground hit by the same ray, tick-0 heights, depth -1.2 .. -3.1,
  mean -2.2; z_seaground_00; noon light) gives, in frame values:

  | Part (region water, noon) | value |
  |---|---|
  | surface alpha (view term 0.215 x depth multiplier about 1, + 0.35 x highlight 0.14; see 4.4's measured note) | 0.27 |
  | surface, no mirror: alpha x its colour (4.3 with the plain-mean normal, highlight of 4.5) | 34.3, 45.2, 54.4 |
  | ground behind it: the lit sea-ground texture hazed (2.2, 2.3), weight about 0.55 | about 101, 137, 87 (model) |
  | caustics (2.5) | about 6, 6, 5 (model) |
  | ground + caustics needed by the measurement | 116.4, 151.0, 95.9 |
  | (1 - alpha) x that ground | 85.5, 110.9, 70.4 |
  | mirror image needed (2.6b, c: the cloud cap) | 0.70, 0.83, 0.92 |

  The mirror share is warm because the cap's colour lies further above the surface's own colour
  (wave / water mix, about (0.35, 0.58, 0.72)) in red than in blue, not because of the light
  colour. **Why the program's mirror adds about half:** with the 0.2 multiplier of the old reading of 4.4 and a
  normalised normal its surface alpha is about 0.13 instead of 0.27; with the original's ground
  the program's formulas give a mirror share of about a third of the original's and a mirror-off
  pixel of (43.4, 55.3, 39.5) paused against its measured (47.2, 59.0, 42.4), so its ground is
  also a little too bright (about +8 % in red and green, +10 % in blue of the frame value).
  Confidence: high for the normal (code) and the measured shares; medium for the model's ground
  parts (normal-map light averaged, flat-ground tangents).
  **Correction to the model (2026-11-09 c):** read `z_seaground_00_n.dds` as
  5:6:5; it is 4:4:4:4 (A4R4G4B4). Read right, its mean saturate(N . L) at noon is 0.567, not
  0.625, so the rule ground of the table is about 4 % darker in its lit part; the "ground
  needed" column rests on the modelled surface and is not a reliable target. The direct
  comparison below is.
  **Which term of the program is too bright (2026-11-09 c, inv-2026-11-09c item 1, E-gfx-046).**
  With the surface rules of 4.2 .. 4.4 built (look comparison 2026-11-09 c), the program's open
  water is about 3 % and the shore film 5 to 7 % brighter than the original's by day, in all
  three channels. Binned by the height of the ground under each pixel, over sea-ground fields
  only (pattern 0xBABEB00B, z_seaground_00; tick 1500, original / program):

  | ground height | -0.05 .. 0 | -0.1 .. -0.05 | -0.2 .. -0.1 | -0.5 .. -0.2 | -1 .. -0.5 | -2 .. -1 | -3 .. -2 |
  |---|---|---|---|---|---|---|---|
  | noon (1500), red | 0.932 | 0.946 | 0.938 | 0.920 | 0.875 | 0.866 | 0.938 |
  | night (11000), red | 1.055 | 1.008 | 0.996 | 0.984 | 1.006 | 0.977 | 1.196 |

  (green and blue alike within 0.01 .. 0.02 by day.) (1) **The lit sea ground itself, by
  about 7 %**, at every depth, by day only: the program normalises the normal map's N and
  uses a 256 x 256 copy of the 512 x 512 sea-ground map, the original neither
  (`GFX-world-view.md` GFX-VIEW-2.4b, T41: D 9.5 % brighter at noon, unchanged at night). That
  alone gives the shore film's excess (no haze and no surface there) and about the open-water
  region's 3 % (the lit part is about 40 % of the pixel at -2 .. -4). The rule: 2.3's lit
  ground is GFX-VIEW-2.4's D with GFX-VIEW-2.4b's unnormalised N from the full-size map.
  (2) **A second, smaller excess between about -0.5 and -2** (a further 5 to 6 % by day, 2 %
  at night), not attributed. Not the caustics: neither the original's pictures nor the
  program's show the caustics' web at 4 x contrast (the strip's lines would add up to about
  30 paused levels), so the caustics pass is drawn in neither (2.5's draw flag; the pictures'
  settings). Its rise from -0.15 to -0.7 is where the wobble of 2.4 sets in; the original's
  sea ground is also visibly sharper there (the program's looks blurred), but the wobble
  changes texture coordinates only and keeps the texture's mean. Open (Q-386). Confidence:
  high for (1) (code, data and measurement agree, day and night); (2) measured only.
  **Measured again after 2.4b (2026-11-10, inv-2026-11-10 item 1, E-gfx-048).** With (1) built
  (look comparison 2026-11-10), the same binning, original / program, sea ground:

  | ground height | -0.05 .. 0 | -0.2 .. -0.1 | -0.5 .. -0.2 | -1 .. -0.5 | -2 .. -1 | -3 .. -2 | below -3 |
  |---|---|---|---|---|---|---|---|
  | noon (1500), r g b | 0.997 0.997 1.001 | 0.997 0.996 0.997 | 0.995 0.995 0.994 | 0.947 0.941 0.938 | 0.936 0.936 0.942 | 1.000 0.980 0.980 | 1.054 0.910 0.863 |
  | 6000, r g b | 1.003 1.002 1.006 | 0.968 0.967 0.963 | 0.967 0.968 0.968 | 0.943 0.936 0.935 | 0.953 0.953 0.956 | 1.024 0.988 0.979 | 1.070 0.815 0.721 |

  (a) (1) is gone: the shallow band is within 0.5 % at noon. (b) The band of (2) is unchanged:
  the program is 5 to 6 % too bright between -0.5 and -2 by day, equally in all three channels.
  (c) New: below about -2 the original shows more red and less green and blue than the program,
  and the gap grows with depth. (d) The look comparison's "water" region (2560 x 1440 picture,
  100, 1150, 500 x 200) lies over ground at -1 .. -3 in the picture's bottom-left corner, where
  (c) makes the program read 2 to 5 % too dark. At the same depth the ratio varies by about 10 %
  across the screen (0.89 to 1.02 in 240 x 640 tiles), so a single region does not measure open
  water. Ruled out as the cause of (b): the normal map's mip level (the file's mips have
  unit-length normals and their mean saturate(N . L) rises with the level, +3.6 % at level 1 and
  +6 % at levels 3 to 5. The original takes its level from the wobbled coordinates of 2.4, the
  program from the plain ones, so the program's level is on average no coarser; a coarser level
  is brighter, so this would make the program darker, not brighter); the colour texture's mips (means within 1 %); the haze of 2.2 (the program follows
  it); the ground's slope (the two noon pictures depend on it in opposite ways). Still open
  (Q-386). **Corrected 2026-11-11 (next paragraph): (b), most of (c) and the 10 % across the
  screen came from the original's interface in its pictures, not from the water.**
  **Measured with the interface left out (2026-11-11, inv-2026-11-11 item 1, E-gfx-049; answers
  Q-386).** The original's pictures show its in-game interface, which the program's pictures do not:
  a frame about 10 pixels wide on all four sides, an ornament in each corner (bottom left a figure up
  to about 80 pixels in from the edge and 160 up), and the button bar at the bottom centre (about
  columns 1035 .. 1525, rows 1335 .. 1440 of the 2560 x 1440 picture). The bins above took every
  pixel, so the bar (dark, over ground at -0.5 .. -2) and the bottom-left figure (brown, over the
  deepest ground) were counted as water. Binned again without them (left out: 12 pixels on every
  side; the boxes 0 .. 80 x 1280 .. 1440 and 2480 .. 2560 x 1280 .. 1440; the bar's box 1000 .. 1560
  x 1320 .. 1440), every 4th pixel in both directions, sea-ground fields only (T20 has the
  original's means):

  | ground height | -0.05 .. 0 | -0.2 .. -0.1 | -0.5 .. -0.2 | -1 .. -0.5 | -2 .. -1 | -3 .. -2 | below -3 |
  |---|---|---|---|---|---|---|---|
  | noon (1500), original / program, r g b | 0.998 0.997 1.000 | 1.001 1.001 1.003 | 1.004 1.003 1.003 | 0.995 1.000 1.003 | 0.988 0.999 1.006 | 1.029 1.013 1.012 | 1.071 1.023 1.009 |
  | 6000 | 0.998 0.997 1.001 | 0.998 0.999 1.001 | 0.997 1.000 1.003 | 0.992 0.999 1.005 | 0.997 1.001 1.004 | 1.054 1.023 1.016 | 1.065 1.018 1.003 |
  | night (11000) | 1.002 1.008 1.025 | 1.004 1.015 1.028 | 1.013 1.021 1.035 | 1.012 1.021 1.036 | 1.027 1.024 1.045 | 1.065 1.040 1.070 | (23 samples) |

  (a) **There is no band.** By day every bin from the shore down to -2 agrees within 1.5 % in all
  three channels, so the program's ground and surface rules (2.2 .. 2.4, 4.2 .. 4.5a, GFX-VIEW-2.4b)
  match the original there and no further term is needed. The "further 5 to 6 %" was the bar.
  (b) **What is left: below -2 the program is a little darker, red the most.** In bins of 0.25 the
  ratio rises smoothly from about 1.00 at -2 to 1.04 .. 1.07 in red at -2.4 .. -3.1 (green 1.5 ..
  3 %, blue 1 .. 2 %): by day 1.2 .. 2.6, 0.7 .. 1.4 and 0.6 .. 0.7 levels of the paused picture.
  Over 40-pixel tiles of the -3 .. -2 bin the program follows the original closely (correlation
  0.92 .. 0.99, slope 0.94 .. 1.04), so it is an offset of the whole deep water, not a shifted or
  weaker pattern. At night the program is 3 to 4.5 % short of blue over all water from -0.1 down
  (about 1 level of the paused picture; 1.4 % on land) and 4 to 7 % short below -2. Not attributed:
  the program's haze per sample follows 2.2 exactly, and the depth factor of 4.4 is 1 in both below
  -2.42. The red-heavy shape (about 1 : 0.6 : 0.45) is nearer "more lit ground and less haze" or a
  stronger warm mirror share (2.6c) in the original than any other single term of 2.2 .. 4.5
  (Q-387). (c) The look comparison's region "water" (100, 1150, 500 x 200) is clear of the
  interface and lies over -1 .. -3: its 4.6 % red is this deep-water remainder.
  (d) **Code reading of the remainder (2026-11-11 eve, Q-387; E-gfx-055).** The original's haze per
  vertex and its terrain and water-surface shaders were read again against the program's: they
  agree term by term (2.2's haze with its clamps, 2.3's blend toward the haze, the surface's
  light, the mirror share with the unclamped red of D, 4.4's opacity and depth term, 4.5a's
  shadow factor on colour and alpha before the highlight, the unnormalised wave normal, the
  highlight, the fog of war). No formula differs, and none changes at -2. Two details the spec
  lacked, neither large enough: the 0.97 of 2.2 step 4 applies only above -10 (below -7.05 the
  vertex is black anyway); and the haze is computed per vertex of the terrain mesh, at the patch's
  level of detail, from that vertex's own height; where a patch edge is stitched to a coarser
  neighbour, the edge's middle vertex is moved to the mean height of its two neighbours but keeps
  the colour of its own height. Linear interpolation of the haze between vertices 0.5 apart in
  height misses its curve by under 0.001 in weight at -2.5 (0.2 % of the lit ground's share). So the remainder is an **input**, not
  a formula. Candidates, each with what separates it:
  1. **The sea ground's shadow factor** (the underwater ground receives the shadow map like any
     ground). Red comes only from the lit ground (the haze has no red), so a darker light term on
     the program's deep ground gives exactly the red-heavy 1 : 0.6 : 0.45 shape. Check first on
     the program's side: its shadow factor on the -2 .. -3 pixels of the 1500 picture (the
     bottom-left corner, at the edge of the shadow map's reach); then, if needed, a pair with the
     shadow option off.
  2. **The view angle**: the deep water lies only in the bottom-left corner, so a term that
     depends on the view (4.4's |V_up|, the highlight, the mirror lookup) cannot be told from
     depth. A pair with the camera moved so that ground at -2.4 .. -3.1 lies near the screen's
     centre separates them.
  3. **The mirror image's content** (the warm share 0.8 x D_red is largest by day): 2.6c's T16c
     pair (mirror off).
  At night the shortfall of blue covers all water, where the ground's share is small, so it is a
  surface term: the mirror's background at night (2.6c) or the blue of the surface light D. The
  mirror-off pair at 11000 tells them apart.
  Confidence: high for (a) (three pictures agree once the interface is left out; the interface is
  visible in each original); (b) measured only; (d) high for the formulas (shipped shaders, the
  haze code), the candidates are not tested.
  (e) **The candidates tested on the original (2026-11-12, Q-387; E-gfx-056, looks).** The same
  game (fog of war off) was run three more times: shadows off (tick 1500), nothing changed (1500,
  6000, 11 000) and mirror off (11 000), each with the cell63 camera and a new camera `deep`
  (target (-62, -405), zoom 2: ground at -2 .. -3 fills the screen's centre). Pictures of different
  runs were compared **at the same ground points** (each pixel's ground point cast through its own
  camera and looked up in the other picture; the cameras of two runs land up to a few units apart,
  and per-bin means of two pictures taken alone mislead by several per cent). Ratios, red / green /
  blue, by the ground height under the pixel:

  | ground | shadows off / on, 1500 cell63 | the same, `deep` | mirror off / on, 1500 cell63 | mirror off / on, 11 000 cell63 | the same, `deep` |
  |---|---|---|---|---|---|
  | -0.2 .. -0.1 | 1.001 1.002 1.005 | 1.005 1.008 1.008 | 0.978 0.979 0.975 | 1.004 1.014 1.030 | 0.996 1.007 1.022 |
  | -0.5 .. -0.2 | 1.002 1.003 1.005 | 1.003 1.004 1.002 | 0.966 0.971 0.966 | 1.008 1.021 1.045 | 1.008 1.018 1.044 |
  | -1 .. -0.5 | 1.003 1.002 1.001 | 1.004 1.003 0.999 | 0.958 0.966 0.960 | 1.013 1.025 1.059 | 1.011 1.026 1.066 |
  | -2 .. -1 | 1.012 1.005 1.000 | 1.021 1.015 1.008 | 0.926 0.949 0.942 | 1.022 1.033 1.084 | 1.010 1.039 1.106 |
  | -3 .. -2 | 1.012 1.007 1.005 | 1.013 1.009 1.003 | 0.860 0.917 0.914 | 1.031 1.045 1.110 | 1.046 1.057 1.135 |
  | below -3 | | 1.000 1.001 1.001 | 0.850 0.913 0.918 | (113 samples) | 1.065 1.082 1.168 |

  (Two runs of the original with the same settings, cameras 6 units apart in height, agree within
  1 % in red and green and 1 .. 3 % in blue at night, so differences above that are the setting.)
  1. **Shadows: not the remainder.** With shadows off the deep water brightens by at most 2 % (and
     part of that is the cameras' few units of height), and at night shadows do nothing at all
     (GFX-VIEW-2.3: the factor is 1 then), while the remainder is 4 .. 7 % below -2 by day and night.
  2. **The mirror is.** The mirror's share of the original's water grows with depth (the surface
     weighs more there, 4.4) and changes sign with the hour: by day the reflected image is brighter
     than the plain surface colour, most in red (mirror off takes 14 % of the red at -2 .. -3); at
     night it is darker, most in blue (mirror off adds 11 .. 13 % of the blue below -2). The
     program's remainder has exactly this pattern: short in red by day and short in blue at night,
     growing with depth. Both fit one cause: **the program's mirror image is darker than the
     original's** (by day it adds too little warm light, at night it takes away too much blue). A
     smaller or larger mirror weight cannot do both. So the remainder lies in the mirror image's
     content (2.6c: the background colour, the cloud cap and its alpha, the reflected objects, the
     wave offset of the lookup), not in the ground, the haze or the surface formula.
  3. **What settles it:** the program's own mirror off / on pairs at the same two cameras and
     ticks (T22). Where the program's ratios differ from the table's, by about the remainder, the
     mirror image is the term; then compare the program's mirror texture content with 2.6c term by
     term. View angle (candidate 2) is left: the `deep` camera's pictures (1500, 6000, 11 000;
     mirror off at 11 000) are taken and need the program's at the same cameras.
  Confidence: high for the table (each ratio over 3 000 .. 94 000 point pairs; the run-to-run check
  above); medium for 2's conclusion (it follows from the signs, not yet from the program's pairs).
  (f) **The mirror texture, term by term (2026-11-12 night, Q-387; E-gfx-059, code reading).** What
  an implementation's mirror texture must hold, to check it against (for example with the texture
  shown alone):
  1. **Target.** 8-bit RGBA holding stored values (no sRGB step, `GFX-world-view.md` GFX-VIEW-2.6),
     128 x 2^trunc(4 q) wide, halved until no wider than the back buffer, half as high, with its
     own depth buffer; drawn every frame before the main view (2.6).
  2. **Clear.** The day table's background at the clock, mixed between the two entries by the
     blend of GFX-VIEW-2.3c: (0.8, 0.9, 1.0) from hour 3 to 20; at hour 0.275 (trace tick 11 000)
     (0.275, 0.275, 0.275), black toward the white of hour 1; black at hours 23 .. 0.
  3. **Camera.** 2.6a: the main view of the world reflected in the water plane y = -0.1, with 1.03
     times the main horizontal field of view and the main camera's aspect (not the texture's 2 : 1;
     the texture is stretched over the screen), culling reversed, the clip plane at the level keeping
     the camera's side.
  4. **Sky.** The cap of GFX-VIEW-6.3 with its corrected geometry (side 632.77, bend 5010, 64 x 64
     cells, the clouds repeating every 90.4 units), placed at the **main camera's eye** + 15 (the
     mirror pass keeps the main camera's eye; the reflection then puts the cap below the water),
     drawn first and blended over the clear colour by its vertex alpha, its colour lerp(T0, T1, b)
     of 6.3: from hour 22 to 23 and from 0 to 1 the stars and the clouds are **mixed**, not shown
     one or the other. The sun's quad lies below the water plane and is clipped. The moon (light
     elevation <= 0: hours 23 .. 0 of the default table) lies above the plane and is reflected with
     the rest.
  5. **Objects.** 2.6b: from the default cameras (looking down more than 30 degrees) the sky alone.
  6. **Lookup.** 4.3: the screen position (x 0.5, y x -0.5, + 0.5) with the noise 1.2 x (N_x, N_z)
     added before the division by w, sampled **bilinearly with wrap addressing**: a lookup pushed
     past an edge of the texture by the noise reads the opposite edge.
  7. **Worked example (T23).** The cell63 camera at tick 1500 (eye (-352.53, 48.51, -407.86),
     direction (0, -0.8, 0.6), noon): the screen centre's mirror texel (without the noise) shows the
     reflected cap 138.55 units along the ray, 83.13 units toward +z from the eye: grid (i, j) =
     (32.00, 40.41), vertex alpha 0.985, texture coordinates (3.500, 2.580), turned by 90 degrees
     to (-2.580, 3.500), i.e. the point (0.420, 0.500) of a clouds tile; the texel is 0.985 x that
     cloud texel + 0.015 x (0.8, 0.9, 1.0). With the misread geometry (side 981, bend 5024, 65
     steps) the same pixel lands at grid (32.5, 38.0) of tiles 140 units wide: 1.55 times larger
     clouds, so a region of water reflects a different, smaller part of the texture.
  8. **Against the program's mirror as built (its source read 2026-11-12):** (a) its cap has the
     misread size and bend (981 / 5024, 65 steps): the clouds are 1.55 times too large, which
     changes how much cloud and how much blue sky each water region reflects by day; (b) it shows
     the stars or the clouds, never their mix: at tick 11 000 the original's cap is 27.5 % clouds
     where the program's is all stars: mean colour (0.221, 0.255, 0.357) against (0.053, 0.044,
     0.150) (clouds (0.663, 0.809, 0.902), stars (0.053, 0.044, 0.150), both opaque), so the
     original's night mirror is brighter and bluer, which is the sign of the night's missing blue in (e) 2; (c) its lookup's
     addressing (wrap) is to be checked. (a) and (b) are the first terms to change before the
     program's mirror pairs of T22 are compared again.
  Confidence: high (shaders, the cap builder and its only call, the sky draw, the day table).
  **Measured after building (2026-11-13, the program's look comparison, Q-387 answered):** with the
  cap's size and the two-texture mix built, the deep water agrees within 1 % by day and at night
  (1500, ground -3 .. -2: program 45.7 57.8 45.9, original 45.9 57.9 46.2; 11 000, -2 .. -1: 14.7
  31.8 31.6 against 14.6 31.8 31.6), and the program's mirror off / on ratios of T22 at cell63
  (1500 -3 .. -2: 0.872 0.928 0.944; 11 000: 1.025 1.041 1.079) lie within 1 .. 3 % of the table in
  (e), inside the original's own run-to-run spread for night blue.

### GFX-WATER-3 The water surface

- **GFX-WATER-3.1** The terrain is divided into cells of 4 x 4 fields, starting
  at field (0, 0): `ceil((w - 1) / 4) x ceil((h - 1) / 4)` cells. Cell
  `(cx, cy)` spans fields `4cx .. 4cx + 4` and `4cy .. 4cy + 4` (inclusive,
  neighbouring cells share their edge; the end is clipped to the last field).
  Its height range is the minimum and maximum over all height samples from the
  first field's sample to the end field's sample (MAP-GRID-4.2), inclusive.
  Confidence: high.
- **GFX-WATER-3.2** A cell gets a water quad when its lowest sample is **below
  -100** raw (the cell is first selected for water when its minimum is at or
  below the water level, raw -100 (3.3), then the quad is emitted only when the
  minimum is below -0.1 world units). Confidence: high.
- **GFX-WATER-3.3** The water quad is a flat rectangle at the **water level,
  world height -0.1** (raw **-100**). Two triangles, six vertices. There is no
  per-field coastline geometry: the shoreline is where the ground rises through
  the plane. Confidence: high.
  **The level (corrected 2026-11-09, Q-1018, E-gfx-045, live read
  E-trace-025):** the map-geometry object's two level words are set once, when the program
  starts, by its constructor: level **-100** and reference **-3000** (raw), and nothing writes
  them later (read live at the start of campaign mission 4: -100, -3000). The surface height is
  level x 0.001 = -0.1. (The spec said 0: the earlier reading found no writer because the
  constructor runs from a static initializer.) So the surface lies at the height of the water
  rule's bound (MAP-GRID-7: a field is water below raw -100).
  **The rectangle (corrected 2026-11-09, Q-1036, E-gfx-045): neighbouring quads abut exactly, they never overlap.**
  The cell keeps two rectangles:
  - **Its box** (culling, 3.4, 3.5): from the world position of its first field
    (column `4cx`, row `4cy`, an even row) to that of its end field (3.1's end
    column and row), and widened in `u` so that it covers the odd field rows'
    half-field shift: the box's right edge is the end column's even-row `u`
    plus 3.0 (half a field step; an odd end row's own position already has
    it). So boxes of neighbouring cells overlap by 3.0 units in `u`.
  - **Its water quad:** the box with those 3.0 units taken off its right edge
    again. So the quad runs in `u` from the first column's even-row `u` to the
    end column's even-row `u` (`u0 + 24 cx` to `u0 + 24 cx + 24` inside the
    map, MAP-GRID-5.2), and in `v` from the first field row to the end field
    row. The next cell's quad starts exactly where this one ends, in `u` and in
    `v`; every point of the plane between the first and the last field column
    and row is covered once. (The spec said "the bounding rectangle of the
    cell's samples", which made neighbours overlap by the odd sample rows' 0.75
    units; drawn alpha-blended (4.1) that strip is blended twice and shows as a
    straight band at each cell column boundary. The original has no such strip.)
  - **At the map's edge (3.1's clipping):** for a cell whose end column is
    clipped to the last field column, the box's and so the quad's right edge
    move out by 1.5 units (one sample step, not a field step) for each column
    cut off. A cell clipped at the last field row is not extended in `v` (the
    game adds the row extension to the box's height, where the later height
    range overwrites it). Beyond these edges the map border of GFX-WATER-6
    takes over. Confidence: high (code, both call sites pass the cell's box);
    medium for the edge extension's visible effect (not compared).
- **GFX-WATER-3.4** Only cells in view are drawn (the visible-cell list of the
  terrain), in that list's order; quads are batched 8000 vertices at a time.
  Confidence: high.
- **GFX-WATER-3.5** A cell whose highest sample is still below the water
  level (raw -100) gets its upper bound raised to the surface height -0.1, so
  its bounding box includes the surface (for culling). Confidence: high.
- **GFX-WATER-3.6 Land under the surface; the shore (F48, play test 14;
  2026-10-29, E-gfx-034; rewritten 2026-11-09 for the level -0.1 of 3.3 and the depth term of
  4.4, Q-1018).** Nothing lifts the land or lowers the water: the surface of 3.3 covers the
  **whole rectangle** of every cell of 3.2, land included, at height **-0.1**, and the shore is
  only what the depth test and the opacity factor of 4.4 make of it. Drawn after the ground with
  depth test less-or-equal, no depth write and no depth bias (4.1), so for a ground point of raw
  height h (interpolated over its ground triangle, as the depth test is, per pixel):
  - **h > -100** (all land, including flat beaches at exactly 0 and shore fields between -100 and
    0, and sites levelled below 0 by ECO-HQ-4.8a): the ground is above the surface and **hides
    it**. No film over such ground. (This is why the look comparison's pictures of the original,
    `look-compare-auto/2026-10-30` weakfogoff-{001500,006000,011000}-cell63, show the flat sand of
    cell (6, 3) clean: Q-291's "coplanar" puzzle came from taking the surface at 0.)
  - **h = -100**: ground and surface in one plane; with less-or-equal the surface passes where
    the two depths agree. Only a line where the ground crosses -100; nothing to match.
  - **h < -100** (water): the surface is in front, at the opacity of 4.4 times the depth factor,
    which is **0.2 at the waterline and rises with the depth** (0.2 + depth below -0.1 / 2.9, 1
    from about 2.4 units below the surface on), over the hazed ground of 2.2.
  So the waterline is the ground's **-0.1 contour** (raw -100), a line that crosses the triangles
  and follows the terrain; it never runs along field or triangle edges unless vertices lie at
  exactly -100. The factor is sampled with linear filtering from the per-field terms (4.4): land
  fields store the shallow value, so towards the land the surface fades to 0.2 of its opacity,
  and the water gets more opaque with depth: a faint, soft shore, no band. Roads and
  flag-to-door strips write no depth (GFX-VIEW-2.6): under the surface they are covered like the
  ground under them. Confidence: high for the geometry, the level, the depth state and the
  factors (code, the live level read); the look of the waterline against the original is to be
  compared again with this rule (T14, T15).

### GFX-WATER-4 Surface look

All from the shipped `data/gfx/fx_20/water_shader.fx` (pass `RenderWater`, or
`RenderWaterNoMirror` without the reflection option) with the values set from
the water style each frame (GFX-WATER-5). Confidence: high unless noted.

- **GFX-WATER-4.1** Blending: alpha blend (source alpha, one minus source
  alpha), depth test less-or-equal, no depth write. Drawn after the ground.
- **GFX-WATER-4.2** Wave normals: two normal maps (`normalMap0` =
  `WaveBump1.dds` 128 x 128, `normalMap1` = `WaveBump2.dds` 512 x 512, both
  uncompressed 16-bit maps in 5:6:5 red, green, blue order (checked in the
  file headers, Q-171 f); the channels are read as a normal's x, z and up
  components scaled 0..1 to -1..1). Coordinates from the vertex world position:
  map 0 at `(x x 0.19, z x 0.17) x 0.3`, map 1 at `(z x 0.23, x x 0.19) x 0.3`.
  Each is then rotated (map 0 by `tex0Rot` = 206 degrees, map 1 by `tex1Rot` =
  145 degrees) and shifted by `phaseSpeed0` = (0.015, 0.015) resp.
  `phaseSpeed1` = (0.017, 0.017) times the elapsed render time in seconds.
  The surface normal is the mean of the two samples (each mapped from 0..1 to
  -1..1, the map's third channel taken as "up"). Confidence: high for the
  numbers; medium for the order "rotate, then shift" and for the time being
  the accumulated frame time.
  **The mean is not normalised (2026-11-09, Q-1018, `water_shader.fx` Water_PS: N = 0.5 x (N0 +
  N1)).** Every later use takes it as it is: L . N and the light of 4.3, the wave / water mix by
  its up part, the reflection noise offset and the highlight of 4.5. Its length is below 1 where
  the two samples disagree. Over the shipped maps (`WaveBump1` / `WaveBump2`, all texel pairs) its
  up part averages **0.927** (0.964 when normalised), saturate(L . N) at noon 0.705 (0.734), and
  the highlight S from the default camera over open water about **0.14** (about 0.25 when
  normalised: a normalised normal nearly doubles the highlight and its +0.35 S of opacity).
- **GFX-WATER-4.3** Light: `D = light colour x (1 - 0.6 x clamp(L.N, 0, 1)) +
  ambient` (the surface is darker where it faces the sun, by design).
  Colour without reflection: `D x mix(waveColor, waterColor, N_up)`.
  With reflection: a mirror image of the scene is sampled at the projected
  position offset by `reflectionNoise x (N_x, N_z)`; the offset is added to the
  projected position **before** the division by w (a projective lookup), so on
  the texture it is `reflectionNoise x (N_x, N_z) / w` (answered 2026-11-06,
  Q-1018, `water_shader.fx`; the projection is the one of 2.6a 2). The colour is
  `clamp(D) x mix(mix(waveColor, waterColor, N_up), mirror, reflectionFactor x D_red)`.
- **GFX-WATER-4.4** Opacity: `clamp(opacity.y - |V_up| x opacity.x, 0, 1)`
  (V = unit vector from the surface to the camera; looking straight down gives
  the most see-through water), then multiplied by the **depth factor**
  `clamp(1.2 - b, 0, 1)`, where `b` is the blue byte / 255 of the world-map
  texture, stored **per field** in two horizontally adjacent texels with odd rows
  shifted by one texel (so a field covers two texels and the hex rows line up;
  E-gfx-034) and sampled with linear filtering at the surface point.
  **The stored byte (corrected 2026-11-09, Q-1018, E-gfx-045;
  live read E-trace-025):** for a field of raw height h,
  `t = clamp((h - level) / (ref - level), 0, 1)` with **level = -100** and **ref = -3000**
  (3.3; so `t = clamp((-100 - h) / 2900, 0, 1)`), and the byte is `trunc(255 x (1 - t))`.
  So `b` is 1 (byte 255) for every field at or above -100 (all land and the
  waterline) and falls linearly to 0 at -3000; the factor is **0.2** at h >= -100,
  `0.2 + t` below (with the byte's truncation), and **1** from about h = -2420 on
  (byte <= 51). Examples: h = -500: byte 219, factor 0.341; -1200: 158, 0.580;
  -2200: 70, 0.925; -3000 and deeper: 0, 1. The bytes are rewritten for the fields of a
  box whenever heights change (the same routine). (The spec said both levels were 0, which made
  the term a step: 1 below 0, 0 above, NaN at exactly 0 packed as 0, so a factor of 0.2 over
  all water and 1 over sand at 0. The constructor that sets them runs from a static initializer
  and was missed.) Confidence: high (code and the live read).
  **Checked against the measurement (2026-11-09; looks E-trace-017, E-trace-021, E-trace-022,
  cell63 at tick 1500, pictures divided by the pause factor):** with this rule per surface point
  (E-trace-017's heights under each point, the texture filter approximated over the field
  lattice) the three open-water regions of 2.6c get mean factors 0.90, 0.84 and 0.94 (surface
  alpha 0.245, 0.258, 0.237, the earlier fit's "about 0.27 / factor about 1"). The mirror image
  they then need is (0.72, 0.85, 0.94) in all three, the cloud cap of 2.6c, and the ground left
  behind the surface is 1 to 9 % above the rule-built ground of 2.2, 2.3 and 2.5. So the rule
  explains the measured opacity; the 0.2 everywhere of the old reading is replaced.
- **GFX-WATER-4.5** Highlight: `S = clamp((H.N)^16, 0, 1)` with `H` the half
  vector of light and view; colour += `S x (1 - colour)`, alpha += `0.35 S`.
  Shadows: 4.5a (the old wording "no less than 80 %" was the shader's constant, not
  the result).
- **GFX-WATER-4.5a Shadows on the water surface (answered 2026-11-09 c, Q-1038;
  E-gfx-047; `water_shader.fx`
  Water_VS_2_0 / Water_PS_2_0 and its techniques, `shared.inc` doShadow, sampler 5).**
  (a) **Which passes.** The surface passes `RenderWater` and `RenderWaterNoMirror` sample
  the shadow map whenever the frame draws shadows; the map border's passes (6.1) and the
  caustics pass (2.5) never do. The water renderer picks its technique with the same
  two switches as every other shadow receiver of the frame (the scene hands the same
  "shadows" and "low detail" values to the terrain, the objects and the water): shadows on
  -> `Default` (shader detail 3) or `LowDefault` (detail 2), whose two surface passes test
  the map; shadows off -> `NoShadow` / `LowNoShadow`, which do not; shader level below 2 ->
  the fixed-function path (7.1), no shadows. So "when on" is exactly when the ground of
  GFX-VIEW-2.5 receives shadows (2.5 (d): the shadow quality above 0 and shader level 2).
  (b) **The lookup.** One tap, no percentage-closer filter: the surface vertex's position
  (at the surface height -0.1, 3.3) times the shadow map's matrix gives a projective map
  coordinate per vertex, interpolated; per pixel its x and y are moved by **the same wave
  offset as the mirror lookup**, reflectionNoise x (N_x, N_z) of 4.3 (added before the
  division by w), then the map is read at that point (point filtered) and the pixel is lit
  when its light-space depth z / w plus the bias of GFX-VIEW-2.5 (c) (-0.00325) is less than
  the stored depth. So the water's shadow edges are hard and wobble with the waves.
  **The offset as a shift on the ground (2026-11-11, Q-1039; E-gfx-052, a live read of the
  running original's shadow matrices).** The map's matrix is the light's view x the light
  space perspective fit x the texture bias (x 0.5, y -0.5, plus half a texel). Adding an offset
  (dx, dy) to the projective x, y reads the map where a water point moved by D on the water
  plane would be read, D solving (grad X - u grad W) . D = dx and (grad Y - v grad W) . D = dy
  (X, Y, W the projective coordinates, u = X / W, v = Y / W; the w scale matters, so an
  orthographic map cannot take the offset as it is). Read at tick 1500 (noon, light (0.4924,
  -0.7660, 0.4132)) for two views, D per **unit** of offset, in world x, z:
  - an offset in x moves the lookup by **the same vector over the whole picture**: default
    camera (eye height 67.02, the default zoom) **(1.33, 1.37)**; the cell63 view (eye height
    48.51, zoom 2) (1.77, 1.78);
  - an offset in y moves it **toward a fixed ground point c, in proportion to the distance**:
    D = k (c - p) for the water point p, with k = **0.0315** and c = the eye's ground position
    + **(63.0, 19.9)** for the default camera (k = 0.0355, c = eye + (48.2, 11.4) for the cell63
    view). For the default camera that is (1.98, -0.96) at the picture's centre, (1.98, -2.04)
    at its top edge, (1.98, -0.24) at the bottom edge, (3.21, -0.96) at the left edge and
    (0.75, -0.96) at the right edge.
  With reflectionNoise 1.2 the water point p is therefore lit or shadowed as the ground point
  p + 1.2 N_x (1.33, 1.37) + 1.2 N_z x 0.0315 (c - p) would be (default camera, noon): for a
  typical wave normal (|N_x|, |N_z| about 0.1 to 0.3) a shift of 0.2 to 1 world unit, a sixth
  of a field at most. An implementation with another map (an orthographic one, cascades) takes
  its single tap at that shifted point. The fit, and with it these numbers, change with the
  view (about 30 % between the two views read); the default camera's values are the ones to
  use where no other is known.
  (c) **The factor.** The test gives `shadow` = 1 when lit, else the receivers' shadow
  factor s of GFX-VIEW-2.3 (d) (1 - k x intensity: 0.1 by day, 0.8 at hour 0, 1 with the
  sun below 20 degrees). The surface's colour **and** alpha are both multiplied by
  clamp(shadow + 0.8, 0, 1): lit 1; in shadow **0.9 by day** (s = 0.1), and 1 (no shadow on
  the water at all) whenever s >= 0.2, so at night and while the shadows fade in. The
  darkest the water gets is therefore 0.9, never 0.8.
  (d) **Order.** Colour (4.3, the mirror term included) and alpha (4.4: the view term times
  the depth factor), then both x the factor of (c), then the highlight of 4.5 is added
  (colour += S (1 - colour), alpha += 0.35 S: the highlight itself is not shadowed), then
  the fog of war of 4.6. Over a shadow by day the surface's own share of the pixel falls to
  0.81 (0.9 x 0.9) and 1 - 0.9 a of the (shadowed) ground shows through.
  (e) The ground under the water takes the shadow map in its own pass (GFX-VIEW-2.5 (b)) at
  its own position, as any ground.
  Confidence: high for (a), (c), (d) (shader source, technique table, the scene's switches);
  high for (b)'s steps; high for the shift on the ground at the two views read (the matrices
  read live, the texture bias read with them), medium for other views.
- **GFX-WATER-4.6** Fog of war: colour x explored factor, alpha + (1 - explored
  factor), so unexplored water is opaque black like unexplored ground.

### GFX-WATER-5 Water styles

- **GFX-WATER-5.1** `graphics.xml` element `Water` holds the global settings
  (`phaseSpeed0`, `phaseSpeed1`, `tex0Scale`, `tex1Scale`, `tex0Rot`,
  `tex1Rot`, fixed-function values) and a list of `WaterProperty` entries in
  file order: index 0 `Default`, 1 `Swamp`, 2 `Braindead`. Each has
  `waterColor`, `waveColor` (three numbers), `opacity` (two), 
  `reflectionFactor`, `reflectionNoise` (two), `normalMap0`, `normalMap1`,
  `shallowColor`, `deepColor` (three). A style without `shallowColor` /
  `deepColor` takes the previous style's values; before the first style the
  built-ins (0, 0.8, 0.3) and (0, 0, 0.05) apply. Confidence: high.
- **GFX-WATER-5.2** The map selects the style by index: the **second of the
  three info values** in the map header (`FMT-s2m-map.md` FMT-S2M-2.7, read
  from MapInfo version 4 on). An index outside the list selects 0. All 34
  release maps have 0, so `Default` is the only style that occurs: water
  (0.3, 0.55, 0.7), waves (0.92, 0.97, 1.0), opacity (0.85, 0.94), reflection
  0.8, noise (1.2, 1.2), shallow (0.0, 0.8, 0.3), deep (0.0, 0.0, 0.05).
  Confidence: high.
- **GFX-WATER-5.3** `tex0Scale` / `tex1Scale` (1 1) only feed vertex texture
  coordinates that the shader path does not read; they have no visible effect
  there. Confidence: medium.

### GFX-WATER-6 Map edge

- **GFX-WATER-6.1** Around the map the game draws a skirt 200 world units wide
  along each edge and at each corner: a black, unlit ground at the lowest
  height found along the map's edge samples, with a water surface over it
  using the same shader (passes `RenderMapBorder` / `RenderMapBorderNoMirror`,
  the ground with `RenderMapBorderGround`, flat black). The pieces follow the
  edge cells: one piece per edge cell of the top row, right column, bottom row and
  left column, each spanning the cell's extent along the edge and reaching 200
  units outward, plus one square piece per corner (200 x 200) tagged with both
  edge bits (edge bits 1 top, 2 left, 4 right, 8 bottom; corners 9, 3, 12, 6).
  **The pieces exactly (read again 2026-11-11, E-gfx-051; corrects "shifted along the
  edge"):** with each cell's box x0 .. x1, z0 .. z1 (the water cells of 3.1) and the unit
  u = 2 x the sample step = **3.0** world units, every piece is a box from the lowest edge
  height up to the water level, over:
  bit 1, each cell of the first cell row: x0 .. x1, z0 - 200 .. z0 + u;
  bit 4, each cell of the last row: x0 .. x1, z1 - u .. z1 + 200;
  bit 8, the first cell of each row: x0 - 200 .. x0 + 3u, z0 .. z1;
  bit 2, the last cell of each row: x1 - 4u .. x1 + 200, z0 .. z1;
  corner 9 (first cell): x0 - 200 .. x0 + u, z0 - 200 .. z0 + u;
  corner 3 (last cell of the first row): x1 - u .. x1 + 200, z0 - 200 .. z0 + u;
  corner 12 (first cell of the last row): x0 - 200 .. x0 + u, z1 - u .. z1 + 200;
  corner 6 (last cell): x1 - u .. x1 + 200, z1 - u .. z1 + 200 + u.
  So every piece overlaps the edge cells by one unit (the column pieces by 3 and 4 units)
  to close the seams; the bits name the sides as min z (1), max x (2), max z (4), min x (8).
  Confidence: high (code read twice: E-gfx-008, E-gfx-051).

- **GFX-WATER-6.2 Skirt shading (Q-189 b).** The skirt's ground pass draws
  with the fixed pipeline, no texture and texture factor 0: flat black. The
  skirt's water passes use the water shader with the fog-of-war world texture
  sampled with **clamp** addressing in both directions (the water shader
  overrides the shared default for that sampler), so every skirt pixel takes
  the fog-of-war alpha, colour and depth term of the nearest edge field of
  the map: black while that edge field is unexplored, ordinary water once it
  is explored. No sea floor is visible in the skirt. Confidence: high
  (`water_shader.fx` lines 6-10 and passes RenderMapBorder*, E-gfx-009).

### GFX-WATER-7 Low-end path

- **GFX-WATER-7.1** Without shaders the surface uses 64 frame textures
  `data/map/textures/water00.dds` .. `water63.dds` (128 x 128 DXT1), texture
  coordinates = world position / `TextureScaleFFP` (12) plus a drift from the
  system clock divided by `TextureMoveSpeedFFP` (30000 ms), alpha
  `OpacityFFP` (132 of 255). Not needed for a shader-based implementation.
  Confidence: medium.
  Covers (inventory): *texture* from `data/map/textures/water[0-9][0-9].dds`.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| water rule | height < -100 | raw height | high | MAP-GRID-7 |
| water surface level | -100 | raw height (world -0.1) | high (corrected 2026-11-09: was 0) | E-gfx-045; live read E-trace-025; E-gfx-003 |
| depth term reference | -3000 | raw height (term 1 - clamp((-100 - h) / 2900), byte trunc(x 255)) | high | E-gfx-045; live read E-trace-025 |
| water quad test | cell minimum < -100 | raw height | high | E-gfx-003, the water renderer's constructor (-100) |
| water quad trim | 3.0 (2 x the sample step 1.5) off the box's right edge | world units | high | E-gfx-045 |
| cell box widening | + 3.0 on the right (half a field step); + 1.5 per column cut off at the map edge | world units | high | E-gfx-045 |
| terrain cell size | 4 x 4 | fields | high | E-gfx-003 |
| haze depth range | 7 | world units (7000 raw) | high | E-gfx-003 |
| haze offset | 0.05 | world units | high | E-gfx-003 |
| haze minimum below -0.1 | 0.07 | fraction | high | E-gfx-003 |
| haze scale | 0.97 | | high | E-gfx-003 |
| border skirt width | 200 | world units | high | E-gfx-003 |
| style index | map header info value 2 | | high | E-gfx-003 |
| caustics frames | 16, loop 5 s | | medium | `water_shader.fx` |
| wave rotations | 206, 145 | degrees | high | `graphics.xml`, E-gfx-003 |
| wave drift | 0.015 / 0.017 | texture units per second | medium (time unit) | `graphics.xml`, E-gfx-003 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | tutorial01 (80 x 80) | count fields with height < -100 | 2925 (equals the Water state count, FMT-S2M T8) |
| T2 | tutorial01 | cells, cells with a water quad (GFX-WATER-3.1, 3.2) | 20 x 20 = 400 cells; 259 with water; (0,0), (1,0), (2,0) among them |
| T3 | MP_6P_Paradise_Island (240 x 240) | same | 60 x 60 = 3600 cells, 2620 with water, 31702 water fields |
| T4 | MP_4P_Fight | same | 302 of 400 cells, 3690 water fields |
| T5 | all 34 release maps | style index | 0 (`Default`) everywhere |
| T6 | vertex at world height 0.0, -0.05, -0.08 | haze weight | 1, 1, 1 - 0.97 sin(0.03/7 x pi/2) ~ 0.9935 |
| T7 | vertex at -0.2 | haze | d = 0.97 sin(-0.15/7 x pi/2) = -0.03265 -> limited to -0.07; weight 0.93; colour (0, 0.744, 0.2825) |
| T8 | vertex at -3.55 (raw -3550) | haze | k = 0.97 x sin(pi/4) = 0.6859; weight 0.3141; colour (0, 0.2513, 0.1285) |
| T9 | vertex at -7.05 or deeper | haze | colour (0, 0, 0), weight 0: the ground there is black (corrected 2026-11-07, E-gfx-040; was "deep colour (0, 0, 0.05)"); a vertex at -7.0 still gets k = 0.97 sin(0.99286 x pi/2) = 0.9699, colour (0, 0.0240, 0.0575), weight 0.0301 |
| T10 | cell whose lowest sample is exactly -100 | water quad | none (test is strictly below) |
| T11 | 3.6: MP_2P_Storm_Coast, cell (6, 3) (fields x 24 .. 28, y 12 .. 16, lowest sample -723); the field (25, 14) at raw height 0, its four neighbours at 0 | the surface over (25, 14) | not visible: the ground at 0 is 0.1 above the surface (-0.1) and hides it (rewritten 2026-11-09 for 3.3's level; matches the original's pictures `weakfogoff-*-cell63` of look-compare-auto 2026-10-30: clean flat sand, no cell edges) |
| T14 | 3.6 (Q-291): the same cell, the camera of `weakfogoff-006000-cell63.json` | our picture against the original's (its pause layer of `UI-ingame.md` UI-GAME-1.11 divided out) | no translucent sheet and no pale lines over the flat sand; the water's edge soft along the samples below -100 (the -0.1 contour, 3.6) |
| T15 | 3.6 (the waterline's shape): the camera of `weakfogoff-006000-cell63.json` (cell (6, 3) of Storm Coast, the Weak game at 6000), ground samples crossing -100 on the cell's west and south side | the edge of the water surface | a soft, curved waterline along the ground's -0.1 contour that cuts through the fields, the surface at 0.2 of its opacity there and more opaque with depth (the original's picture; ours of 2026-10-30 had straight segments and a pale, fairly opaque sheet near the contour, which came from the old level 0 and the step term's factor 1 next to fields at 0; rewritten 2026-11-09) |
| T12 | 3.6: a land sample at raw -40 (e.g. (40, 18) of Storm Coast) in a water cell; a site levelled to raw -23 on a shore field in a water cell | the surface over them | not visible: both are above the surface at -0.1 (rewritten 2026-11-09: was "drawn at 0.2, a faint film") |
| T13 | 3.6, 4.4: a field at raw 0 next to one at raw -1200 | the depth factor halfway between their texels | about 0.39: bytes 255 and 158, filtered to 206.5, 1.2 - 206.5 / 255 (rewritten 2026-11-09: was 0.7 from the step term) |
| T16 | 2.6 (which technique): the settings of the sample file (preset VeryHigh, the 13th bool 1, mirror quality 1.0, shader detail 3), Storm Coast at noon, water cell (6, 3) seen from the default camera | the technique and the mirror texture | `RenderWater` with a mirror texture of 2048 x 1024 on a 2560-wide back buffer (1024 x 512 on a 1024-wide one). The original's water at noon is 9 to 13 % bluer than the implementation's `RenderWaterNoMirror` (red and green within 0 .. 4 %: look-compare 2026-10-31, the implementation's measure), which fits a pale-blue mirror image mixed in. The exact colour waits for the mirror pass's rest (open) |
| T16a | 2.6a to 2.6c (answered 2026-11-06, Q-1018): the same settings and camera as T16 (default camera, 45 degrees down, MIRROR DETAILS 1.0) | what the mirror texture holds over open water, and where the water reads it | the sky alone (forward y < -0.5): the cloud cap (`clouds.dds` over the background (0.8, 0.9, 1.0)), drawn from the reflected view with 1.03 x the field of view; no land, no objects; read at the point's own screen position (no 1 - x), offset by the wave noise / w. A camera flatter than 30 degrees down with details 1.0 would reflect land and every object family too |

| T16b | 4.2: the two wave texels (r, g, b) = (0.6, 0.5, 1.0) and (0.5, 0.6, 1.0) | the surface normal | (0.1, 1.0, 0.1) (x from red, up from blue, z from green; the plain mean, length 1.01, not normalised): its up part 1.0 enters the wave / water mix and L . N as it is |
| T16c | 2.6c, 4.4 (measured): Storm Coast, E-trace-017's cell63 camera at tick 1500, noon, mirror on and off, region water (100, 1150, 500, 200) of 2560 x 1440, unpaused frame values | the region's mean | mirror off 119.9, 156.1, 125.0; mirror on 134.9, 167.5, 132.9 (paused pictures: x 90 / 255, 42.3, 55.1, 44.1 and 47.6, 59.1, 46.9); surface alpha about 0.27 |
| T17 | 3.3 (Q-1036): MP_2P_Storm_Coast (170 x 170 fields, 681 x 681 samples, u0 = -510, v0 = -441.67275), cells (6, 3) and (7, 3) | the water quads' corners and the boxes' u range | (6, 3): u -366 .. -342, v -379.31895 .. -358.53435, box u -366 .. -339; (7, 3): u -342 .. -318, same v, box u -342 .. -315. The quads share the edge u = -342 and do not overlap; over open water no pixel is covered by two quads (no band at cell column boundaries) |
| T19 | 4.5a (Q-1038): a surface pixel with colour (0.6, 0.7, 0.8) and alpha 0.3 after 4.3 / 4.4, highlight S = 0.1; shadows on (quality 1.0, shader detail 3); (i) lit, (ii) in a shadow at noon (s = 0.1), (iii) in a shadow at hour 0 (s = 0.8), (iv) shadows off | the surface's colour and alpha before the fog of war | (i) colour 0.6 + 0.1 x 0.4 = 0.64, 0.73, 0.82, alpha 0.335; (ii) x 0.9 first: colour 0.54, 0.63, 0.72 -> 0.586, 0.667, 0.748, alpha 0.27 + 0.035 = 0.305; (iii) as (i) (clamp(0.8 + 0.8) = 1); (iv) as (i), technique NoShadow. The map-edge skirt in (ii): as (i) |
| T18 | 4.4 (Q-1018): fields at raw 0, -100, -101, -500, -1200, -2200, -2420, -3000, -6000 | the stored blue byte and the depth factor 1.2 - byte / 255 (clamped) | 255 / 0.2; 255 / 0.2; 254 / 0.204; 219 / 0.341; 158 / 0.580; 70 / 0.925; 50 or 51 / 1.0; 0 / 1.0; 0 / 1.0 |
| T21 | 4.5a (b) (Q-1039, E-gfx-052): the default camera at tick 1500, noon (eye (-331.05, 67.02, -222.47), view (0, -0.8, 0.6)); the water point under the picture's centre p = (-331.0, -172.1); wave normal N_x = 0.1, N_z = 0.2; reflectionNoise 1.2 | the ground point whose shadow the water pixel takes | c = eye + (63.0, 19.9) = (-268.05, -202.57); D = 1.2 x 0.1 x (1.33, 1.37) + 1.2 x 0.2 x 0.0315 x (c - p) = (0.160, 0.164) + (0.476, -0.230) = (0.64, -0.07): the point (-330.4, -172.2). With N_x = N_z = 0 the point itself |
| T22 | 2.6c (e) (Q-387): the program's cell63 and `deep` pictures at 1500 and 11 000, with the mirror switch on and off (the same cameras and game state; the original's pictures and camera files with the look comparison) | ratio mirror off / on by ground height, compared at the same ground points | the original's: 1500 cell63 -2 .. -3 0.860 0.917 0.914, -1 .. -2 0.926 0.949 0.942; 11 000 cell63 -2 .. -3 1.031 1.045 1.110, `deep` 1.046 1.057 1.135; the program within 1 % of each (a program that differs by the 2.6c (b) remainder has a darker mirror image) |
| T23 | 2.6c (f): the cell63 camera at tick 1500 (eye (-352.53125, 48.510056, -407.861786), direction (0, -0.8, 0.6), 50 degree horizontal field of view; mirror field of view x 1.03), water level -0.1, noon; then the same camera at tick 11 000 (clock 48.275) | the mirror texture's texel at the screen centre, without the noise | 1500: the reflected cap at 83.13 units toward +z from the eye, cap grid (32.00, 40.41), alpha 0.985, the clouds texture at (0.420, 0.500) of a tile (texture coordinates (3.500, 2.580) turned by 90 degrees), blended over (0.8, 0.9, 1.0). 11 000: cleared to (0.275, 0.275, 0.275); the cap's colour 0.725 x stars (turned by -45 degrees) + 0.275 x clouds (turned by 90), x the vertex alpha |
| T20 | 2.2 .. 4.5a as a whole (2.6c "Measured with the interface left out", E-gfx-049; Q-386): Storm Coast, fog of war off, paused pictures of 2560 x 1440, horizontal field of view 50 degrees, view direction (0, -0.8, 0.6), eye at tick 1500 (-356.4375, 49.553894, -405.908661), 6000 (-356.4375, 47.842884, -403.955536), 11000 (-354.484375, 56.865261, -405.908661). Every 4th pixel in both directions (x, y multiples of 4); its bin is the height of the ground hit by the pixel's view ray (the water surface ignored; the heights at tick 0), counted only when that ground point's nearest field has the pattern `z_seaground_00` and the pixel is outside the interface boxes of 2.6c | the original's mean per bin, levels of the paused picture, r g b (samples) | **1500:** -0.05 .. 0: 58.3 50.4 36.0 (3253); -0.1 .. -0.05: 66.2 57.6 42.0 (1152); -0.2 .. -0.1: 65.2 59.9 44.8 (2532); -0.5 .. -0.2: 63.1 61.1 46.4 (6901); -1 .. -0.5: 61.1 64.0 48.5 (8174); -2 .. -1: 52.8 62.2 48.0 (11075); -3 .. -2: 45.9 57.9 46.2 (7849); below -3: 38.2 50.6 41.7 (813). **6000:** 57.3 49.5 35.2 (3442); 65.4 56.8 41.2 (1099); 64.1 59.1 44.0 (2648); 61.9 60.3 45.9 (7014); 60.7 63.8 48.3 (7874); 54.3 63.1 48.7 (10326); 46.7 58.3 46.3 (5738); 37.6 50.8 40.2 (172). **11000:** 13.9 18.4 19.9 (2053); 17.0 22.0 24.8 (771); 16.8 24.0 26.6 (1961); 16.8 26.1 28.8 (5062); 16.1 29.3 30.3 (5689); 14.6 31.8 31.6 (6304); 11.0 29.4 29.0 (3042); below -3 too few (23). A rendering passes a bin when each channel is within 3 %, or within 1 level where the original's mean is below 30. The program on 10 Nov passes every day bin down to -2 and fails -3 .. -2 at 6000 and below -3 (Q-387), and at night the blue of -0.5 .. -2 and the green and blue of -3 .. -2 |

Values in T6..T9 with the `Default` style; computed from GFX-WATER-2.2
(the investigation's water cell survey produced T1..T5).

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-gfx-003 | code reading | `S2DNG.exe` | the water renderer's constructor: config sliders, quad threshold -100, technique and parameter handles, 64 FFP frames, caustics texture |
| E-gfx-003 | code reading | | `Water` / `WaterProperty` reader, shallow / deep fallback |
| E-gfx-003 | code reading | | select style (out of range -> 0), upload style colours |
| E-gfx-003 | code reading | | per frame: time accumulation, rotation + drift of the two normal-map transforms |
| E-gfx-003 | code reading | | per visible cell: water selected when min <= level; quad when min < -100 x height scale; quad at level height |
| E-gfx-003 | code reading | | quad vertex build (6 vertices, both paths) |
| E-gfx-003 | code reading | | terrain cells 4 x 4 fields, min / max height per cell, raise to water level |
| E-gfx-003 | code reading | | depth haze colour per vertex |
| E-gfx-003 | code reading | | world-map texture depth term |
| E-gfx-004 | code reading | | depth term level and reference both 0, step result |
| E-gfx-004 | code reading | | caustics pass condition, mirror technique choice |
| E-gfx-003 | code reading | | map edge skirt: lowest edge height, 200-unit pieces, draw |
| E-gfx-003 | code reading | | style index from MapInfo (version >= 4) |
| E-gfx-003 | data survey | the investigation's water cell survey over the 34 maps | T1..T5 |
| data | shader | `data/gfx/fx_20/water_shader.fx`, `terrain_shader.fx` (decoded) | surface, caustics, haze use, wobble |
| E-gfx-009 | code reading | | skirt draw: ground pass then water passes with the world texture; sampler 3 clamp |
| E-gfx-034 | code reading | `water_shader.fx` passes (less-or-equal, no depth write, no bias), `shared.inc` sampler 3 linear; `terrain_shader.fx` (no vertex offset) | GFX-WATER-3.6, 4.4 (T11 .. T13) |
| E-gfx-038 | code reading | the settings copied to the scene (the mirror switch), the water draw, the frame's mirror pass, the mirror texture size from trunc(4 x quality), the mirror camera with its clip plane at the water level; the scene settings' getters | GFX-WATER-2.6, T16 |
| E-gfx-044 | code reading and model | (session 2026-11-09): `water_shader.fx` Water_PS (plain-mean normal), PS_UnderwaterSurface; the investigation's model scripts | GFX-WATER-4.2, 4.4, 2.6c, T16b, T16c |
| E-gfx-045 | code reading | (session 2026-11-09 b): the cell set-up (box from the first and end field's positions, + 3.0 on the right for an even end row, + 1.5 per column cut off), field to world (+ 3.0 on odd rows), the six vertices (the box's right edge minus 2 x the x step), the water quad test and its two call sites; the map-geometry constructor (level -100, reference -3000), run by a static initializer; the stored byte (blue = 1 - clamp((h - level) / (ref - level)), x 255, truncated); live read of the two words at the start of campaign mission 4: -100, -3000 (E-trace-025 `geo_levels.log`) | GFX-WATER-3.3, 3.5, 3.6, 4.4, T11 .. T15, T17, T18 |
| E-gfx-048 | data and looks | (session 2026-11-10, no bundle) `look-compare-auto/2026-11-10` cell63 pairs binned by depth, screen tile and slope; per-mip means of `z_seaground_00_n.dds` and `z_seaground_00.dds`; `terrain_shader.fx` (the wobble added to both coordinate pairs in the vertex shader, `tex2D` in the pixel shader) | 2.6c "Measured again after 2.4b" |
| E-gfx-059 | code reading | (session 2026-11-12 night) `enviroment_shader.fx` (the sky pass mixes two textures), the sky cap's builder and its only call (5000, 5010, 64 cells, 7 x 7), the sky's per-frame placement (+15 over the camera's eye), the sky draw (the second texture: the next entry's when it differs, else the same), `water_shader.fx` (the mirror stage's wrap addressing and filter), the shipped day table | 2.6c (f), T23; `GFX-world-view.md` GFX-VIEW-6.3, T44 |
| E-gfx-056 | looks of the original (pictures kept outside the repository) | (session 2026-11-12) three runs of the Weak game with fog of war off: shadows off (1500), nothing changed (1500, 6000, 11 000), mirror off (11 000); cameras cell63 and `deep`; compared with the 2026-11-08 mirror-off picture and the look comparison's originals at the same ground points | 2.6c (e), T22 |
| E-gfx-052 | a live read of the running game | (session 2026-11-11, one unattended run of the original to tick 1500, fog of war off, paused for its two pictures; the player's files backed up and restored byte for byte) the shadow renderer's light view, light-space projection, their product and the texture bias for the default view and the cell63 view; the shift on the water plane solved from them | GFX-WATER-4.5a (b), T21 |
| E-gfx-051 | code reading | (session 2026-11-11): the skirt's pieces (the box per edge cell and per corner, the unit 2 x the sample step) | GFX-WATER-6.1 |
| E-gfx-055 | code reading | (session 2026-11-11 eve): the haze per vertex and its caller (the terrain patch builder: per mesh vertex at the patch's level of detail, the stitched edge vertices), `terrain_shader.fx` and `water_shader.fx` term by term against the program's shaders | 2.6c (d) |
| E-gfx-049 | data and looks | (session 2026-11-11, no bundle) the look comparison of 2026-11-11, cell63 pairs at 1500, 6000 and 11000: the original's interface located (frame, corner ornaments, button bar) and left out; depth bins (also in steps of 0.25 from -1.5 to -3.25), 40-pixel tile regression of the deep bins; the program's haze per sample read against 2.2; `water_shader.fx` and `terrain_shader.fx` read again | 2.6c "Measured with the interface left out", T20 |
| E-gfx-047 | code reading | (session 2026-11-09 c): `water_shader.fx` (Water_PS_2_0: one tap at the noise-offset projective coordinate, x clamp(shadow + 0.8) on colour and alpha before the highlight; bUseShadowMap true only in RenderWater / RenderWaterNoMirror of Default / LowDefault), `shared.inc` (doShadow: 1 or SHADOW_INTENSITY; ShadowTex = sampler 5, point) | GFX-WATER-4.5a, T19; GFX-VIEW-2.5 (b) |
| E-gfx-046 | data and looks | (session 2026-11-09 c, no bundle) `terrain_shader.fx`, `shared.inc` sampler 2, the normal maps' headers; the investigation's normal map model and depth bins on the 2026-11-09 c look pairs | 2.6c "Which term", `GFX-world-view.md` GFX-VIEW-2.4b |
| E-gfx-040 | code reading | (session 2026-11-07) | GFX-WATER-2.2, 2.6b, 2.6c, T16a |
| E-gfx-039 | code reading | (the session of 2026-11-06, also filed as E-snd-004, E-ui-019, E-sav-009, E-trn-031): `data/enviroment/clouds.dds` | GFX-WATER-2.6a to 2.6c, 4.3, T16a |
| data | settings | `data/settings/graphics.xml`, `terrain_static_data.xml` | styles, wave settings, pattern types |

## Open questions

- GFX-WATER-2.6c (rest): the original's open water is about 4 levels bluer than the
  program with the mirror; the cloud cap explains only part (T16a). Code has no further
  term (2026-11-07); the gap scales with the light and lies in the ground under the water
  (2.6c). Measured 2026-11-08 c (2.6c table): the mirror adds +5.3 / +4.0 / +2.8 over open water
  at noon; the remainder (42.3, 55.1, 44.1) is the target for the ground, caustics and surface.
  Open: how that remainder splits between them. **2026-11-09:** split (2.6c): the pictures are
  paused (x 0.353); the surface normal is an unnormalised mean (4.2); the measured surface alpha
  over open water is about 0.27, three times 4.4's rule. Open only: the depth byte's actual rule
  (4.4's measured note; a live read of the two levels settles it). **2026-11-11:** the depth
  byte is settled (4.4), and with the original's interface left out the water agrees within
  1.5 % by day down to -2 (2.6c, T20). Left: the deep water below -2 and the night's blue (Q-387).
- Source and unit of the shaders' render clock (`g_Timer.y`).
