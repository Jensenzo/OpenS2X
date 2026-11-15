# Shadows: Bevy's shadow map against GFX-VIEW-2.5

The viewer uses Bevy 0.20's own directional-light shadow map
(`crates/viewer/src/render/view3d/build.rs`, the `DirectionalLight` setup) and reads it in our
shaders through `bevy_pbr::render::shadows::fetch_directional_shadow`
(`shaders/terrain.wesl`, `shaders/object.wesl`, both `shadow_at`). Objects draw
themselves into the map with `shaders/object_shadow.wesl`. The spec allows any
directional shadow map that keeps GFX-VIEW-2.5 (a) to (c). This page lists, part
by part, where ours matches and where it differs.

| Spec | Ours | Same? |
|---|---|---|
| 2.3 factor in shadow: 1 - k x intensity (0.1 in full sun), k fading from 0 at 20 deg to 1 at 40 deg of elevation | `world3d::shadow_factor`, sent as `light_dir.w`; the shaders mix `light_dir.w .. 1` by Bevy's lit fraction | yes (tests T14, T15) |
| 2.5 a casters: buildings, sites, settlers, animals, trees (sway), flags (wave), stones, doodads; `shadowCast="false"` types excluded; alpha < 0.5 cut out; terrain casts none | `object_shadow.wesl` with the same bones, sway and wave as `object.wesl`, alpha cut at 0.5; the terrain and water materials have no shadow pass (`enable_shadows` false); `NotShadowCaster` on the sky, the marks and build help, the status icons, the ground decals, streets and `shadowCast="false"` types | yes |
| 2.5 b receivers: everything but the sky, the build help, the water's map border (GFX-WATER-6.1) and the caustics; the ground decal receives them (GFX-OBJ-2.3a d, Run D), and so does the water surface (GFX-WATER-4.5a, 2026-11-09) | `shadow_at` called by the terrain, roads and every object family, the decal too (following 2.3a d, Q-217); the water surface's own `shadow_lit` (`water.wesl`), off in the border's material (`view3d::skirt_material`) | yes |
| GFX-WATER-4.5a b water test: one point-sampled tap, no filter, at the surface point moved by the mirror's wave offset in the map's projective coordinates | one `textureLoad` tap in the cascade the ground takes at that distance (no blend), with the ground's bias and the surface's up normal; not moved by the wave offset | **no**, see 4 below |
| 2.5 c test: lit when light depth + bias -0.00325 < stored depth | Bevy offsets the sampled point instead: `shadow_depth_bias` 0.1 world units toward the light and no offset along the normal (`build::shadow_light`; 2.5 f, T43: Bevy's default 1.8 texels along the normal lit the houses' door walls) | **no**, see below |
| 2.5 c filter: 4 taps (pixel, right, down, diagonal), bilinear by the fraction inside the texel | `ShadowFilteringMethod::Hardware2x2`: one hardware comparison sample, which is that same bilinear 2 x 2 percentage-closer filter | yes |
| 2.5 d one map, fitted each frame to the view part holding casters and receivers (casters extended along the light to the ground, up to 1000 units), always LiSPSM (Run D) | two cascades (near 1 .. 80 units, 80 .. 300 units, 20 % overlap blended), each an orthographic fit of Bevy's own; Bevy has no LiSPSM, and the two cascades stand in for its finer texels near the camera | **no**, see below |
| 2.5 d map size 128 x 2^round(4 q) for `SHADOW QUALITY` q, none at 0 (Run D) | `view3d::shadow_map_size` from the profile's q (2048 at the default and sample 1.0); each cascade gets a map of that size; at 0 the light casts no shadows | yes (per cascade) |

## The differences

1. **Bias.** The spec's -0.00325 is in light-space depth, where 0 to 1 spans the
   light frustum's depth. In world units that is 0.00325 x the frustum's depth
   range, which depends on the original's fit (2.5 d) and was not traced: a range
   of 100 units gives 0.33 units, 1000 units gives 3.25. Bevy biases in world
   units (0.1 since 2026-11-12 night). Bevy's further offset of 1.8 texels along the
   receiver's normal is set to 0 (GFX-VIEW-2.5 f, T43): with it, a wall the sun only
   grazes, right under its roof's overhang (the houses' door walls), was pushed out
   of the overhang's shadow and drawn lit (the woodcutter's door wall at the town
   camera of 12 Nov, tick 6000: original 34.8, ours 44.5 with the offset, 32.3
   without). Without the offset the depth bias alone must keep acne off the roofs:
   0.02 left the quarries' roofs 6 to 12 % darker than the original's, 0.05 3 to
   7 %, 0.1 2 to 4.5 % with no visible pattern. In practice:
   - a short caster (a sapling, a settler's foot) can lose the base of its shadow
     with the original's bias where ours keeps it, or the other way round.

   Q-215 (a) asks for the depth range so the bias can be converted.
2. **Frustum and texel size.** At q = 1 two 2048 cascades make a shadow texel
   about 80 / 2048 = 0.04 units near the camera and 0.15 units at 300 units.
   The original's one 2048 map is warped by LiSPSM so that its texels are
   also finest near the camera, but by how much depends on its fit, which
   was not traced. The edge is soft over one texel in both.
3. **Cascade blend.** Bevy blends the two cascades over 20 % of the first
   cascade's range. The original has one map and no blend.
4. **The water's wave offset (Q-1039 answered).** GFX-WATER-4.5a (b) moves the
   water's lookup by reflectionNoise x (N_x, N_z) in the shadow map's projective
   coordinates before the division by w, so the water's hard shadow edges
   wobble with the waves. On Bevy's orthographic cascades (w = 1) the same
   offset would move the lookup by about 1.2 x N_x x half the cascade's width
   (several fields), so ours takes the tap at the ground point the original's
   offset reads, solved from its matrices (T21): the water point p moved on the
   water plane by R N_x (1.33, 1.37) + R N_z 0.0315 (c - p), c the eye's ground
   position + (63.0, 19.9), the default camera's values for every view (the
   original's change by about 30 % with the view). A shift of 0.2 to 1 world
   unit, so the edges wobble as the original's do.
5. **Shadows off (GFX-VIEW-2.5 e, T42).** The original makes no map and every
   receiver takes its shadow test as 1. Ours keeps a 128-texel map and sets the
   receivers' shadow factor (the light's fourth value) to 1 instead, so every
   test gives lit whatever the map holds: the same pixels. Without any light
   casting shadows, Bevy's water mirror camera drew nothing at all; its black
   image went into the water's mix and made the sea up to 31 % darker below -3
   (the Q-387 check of 2026-11-11; measured again 2026-11-12: with the fix the
   water bins equal those with shadows on). The cost is the shadow pass into
   the small map.
