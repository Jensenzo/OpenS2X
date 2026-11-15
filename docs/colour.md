# Colour: from texel to pixel

GFX-VIEW-2.3 says the light works on texture values as stored: a texel v under
the light D comes out as min(255, v x D), with no gamma, no exposure and no
scaling of the sum. This page follows a texel through the 3D view and lists every
place where Bevy could change it, and what we do about each.

## The chain

| Step | What happens | Where |
|---|---|---|
| Texture decode | DDS (DXT1/3/5) and TGA decoded to RGBA8 as stored, rows from the top | `s2_assets::dds`, `s2_assets::tga` |
| Resample and mips | Non-power-of-two textures resampled, mip levels averaged on the stored values (no linearising) | `textures::resample_2d`, `textures::mip_chain`, `mip_chain_2d` |
| Upload | **`Rgba8Unorm`** (not sRGB) for everything a map shader samples: terrain patterns and normal maps, object colour, light and colour maps, the settlers' palette atlas, the world texture (`Rg8Unorm`). Sampling returns v / 255 unchanged | `models3d::model_image`, `nearest_tga`, `view3d` (terrain layers, `repeating_image`, `world_image`) |
| Lighting | Our own shaders: `D = light x saturate(N . L) x shadow + ambient`, `colour = D x texel` per family (GFX-VIEW-2.4, GFX-OBJ-2, 3). D above 1 is kept; `saturate` clamps only the product. A tree's vertex colour scales only the direct term (GFX-OBJ-2.4); a type-3 ground pattern adds a white highlight masked by its normal map's alpha (GFX-VIEW-2.4a) | `shaders/terrain.wesl`, `object.wesl`, `water.wesl` |
| Shader output | `saturate(colour)`: the stored value itself, as the original's shaders write it | the end of each fragment function |
| Render target | The map camera is `Hdr`: its main texture is `Rgba16Float` and holds stored values. See-through layers blend on them with source alpha / one minus source alpha, as GFX-VIEW-2.6 (c1) says (no gamma in the mix) | `view3d::setup_3d` |
| Interface | Drawn onto the same float texture, before the conversion below. Its shader is Bevy's own with one change: the colour it computes is written as the stored (sRGB-encoded) value, so a translucent piece (the paused view's layer, glyph and sprite edges) mixes with the stored values under it, src x a + d x (1 - a), as the original's fixed-function interface does (GFX-VIEW-2.6 c, UI-GAME-1.11). Opaque pixels come out as before | `shaders/ui_stored.wesl`, `stored.rs` |
| Stored to linear | After the 3D passes and the interface (after Bevy's `ui_pass`, before `upscaling`) one full-screen pass turns every stored value v into the linear value the window's sRGB encoding gives back as v (Bevy's blit with `SRGB_TO_LINEAR`). The clear colour is put in as stored numbers (`Color::linear_rgb(BACKGROUND)`) | `stored.rs` |
| Tonemapping | `Tonemapping::None` on the map camera: Bevy's tonemapping pass returns at once | `view3d::setup_3d` |
| HDR, exposure | `Exposure` is attached by Bevy but only the lit PBR shaders read it; the few `StandardMaterial`s on the map (the black ground under the terrain, the cloud cap, the overlay icons, the status-icon particles, the placement marks) are unlit, and their textures are uploaded as `Rgba8Unorm` (`ModelLibrary::stored_image`) so they too give stored values | Bevy, `models3d.rs` |
| Colour grading | `ColorGrading` is attached by Bevy at its identity default and is applied only in the tonemapping pass, which does not run | Bevy |
| Deband dither | `DebandDither::Disabled` on the map camera (it would only run in the tonemapping pass anyway) | `view3d::setup_3d` |
| Bloom, depth of field | Not added. The original runs its post effects only with `-postFX` (GFX-VIEW-2.3) | |
| MSAA | Bevy's default 4 x; changes only pixels on triangle edges | Bevy |
| Swapchain | `Bgra8UnormSrgb` on the Windows machine (the probe reports it); Bevy's final blit writes the linear values, which the format encodes back to the stored bytes | Bevy |

The interface (menus, panels, icons) is 2D. Its sprites are uploaded as
`Rgba8UnormSrgb` and drawn by Bevy's UI, which samples them as linear; our UI
shader encodes the result back to the stored value before the blend, so an opaque
sprite's bytes come out unchanged and a translucent one mixes with the stored
bytes under it. On the map camera that is the float texture of stored values
above. The menus and the 2D map are drawn by a `Camera2d` with Bevy's
`CompositingSpace::Srgb`: its main texture (`Rgba8Unorm`) holds the stored
values, Bevy's 2D shaders write them, and its last blit decodes them for the
window. The ship window's picture (UI-GAME-4.5) is encoded the same way by its
own material shader. A project's own Bevy UI nodes go through the same shader:
their colours and pictures keep their meaning, only their blending is on stored
values (`docs/extending.md`).

**Why (the look comparison of 2026-10-30).** Before, the interface was drawn after the
conversion, so it blended in linear light. The paused view's layer, black at alpha
165 / 255, then kept enc(0.353 x dec(v)), about 0.6 of a stored value v, where the
original keeps 90 / 255 of it: every paused picture of ours was about 1.7 times
brighter than the original's, and relatively greener (dark channels are lifted
more). Measured on the 12 pairs with ours recomputed as if the layer had been
blended on stored values, lit ground, sand, rock and grass at noon agree within 3 %
per channel. The colour probe checks it on the GPU (below).

## The test

`--colour-probe <report>` (`crates/viewer/src/tools/colour_probe.rs`) starts a free
game in the 3D view and puts four rows of 1 x 1-texel quads in front of the
camera, drawn with the object shader under the map's light:

- **lit flat**, N . L = sin 50 deg: D = (1.3182, 1.1879, 1.1983);
- **shadowed flat**: an occluder that only the light sees stands 3 units toward
  the sun; D = (0.6718, 0.5688, 0.6598), the shadow factor 0.1 of GFX-VIEW-2.3;
- **edge-on** to the light: D = ambient (0.6, 0.5, 0.6);
- **a ground decal wound the other way round**: lit flat as the first row
  (GFX-OBJ-2.3a d, see below);
- **a road texel over lit grass** (T25): the strip's centre texel (86, 71, 49)
  at alpha 153, 51 and 0, with a normal-map texel pointing straight up;
- **a decal border texel over lit grass** (T26): the woodcutter's six border
  texels from alpha 255 to 0;
- **a decal texel over a road texel** (T27, in the road row's free columns):
  the decal lies between the grass and the road, farther from the camera, so
  only the fixed layer order of GFX-VIEW-2.7 (streets, then decals; the sort
  bias of `models3d::ground_layer_bias`) mixes it on top. With the bias off
  these two quads fail (the road comes out on top).

The grass under the last two rows is the texel (80, 110, 40) lit flat, the
stored pixel d = (105, 131, 48).

- **the interface over two lit quads** (GFX-VIEW-2.6 c, UI-GAME-1.11): a 12 x 12
  pixel fill of the paused view's black at alpha 165 over the lit 128 grey, (169,
  152, 153) -> (60, 54, 54), and of (200, 120, 60) at alpha 128 over the lit (90,
  140, 200), (119, 166, 240) -> (160, 143, 150). Blended in linear light the first
  would come out about (116, 104, 105).

It captures the frame and checks each quad's centre pixel against
min(255, round(v x D)), and the blended ones against round(255 x (rgb x a +
d x (1 - a))), allowing one step; the greys 40, 128 and 220 are GFX-VIEW T20
and T21. The play harness runs it (`colour_pipeline` in
`crates/viewer/tests/play.rs`). On 2026-10-18 all 37 pixels passed, for example
128 lit -> (169, 152, 153), 128 shadowed -> (86, 73, 84), the road at alpha 153
-> (110, 103, 54) (T25), the decal border at alpha 136 -> (104, 105, 43) (T26);
three are one step off (the hardware's rounding; 2.6 f allows one step). Before
the change the two blended rows failed with exactly the linear-light values the
spec predicts, (110, 106, 55) and (104, 108, 43).

## The dark ground patches (fixed)

The site patch and the HQ's paved floor are the buildings' ground decals.
They came out at ambient light only, (58, 43, 28) where the earth texel lit gives
(132, 106, 61): the object shader is two-sided and turned a back face's normal
down, and the decal meshes' triangles face away from the camera, so N . L was 0.
GFX-OBJ-2.3a (d) lights the decal with its mesh's own up normal; the shader no
longer turns it for the decal family, and the probe's fourth row keeps it so (it
fails with ambient values when the turn is put back).

## Shadows on the ground

The same frame drawn with and without the shadow map, divided pixel by pixel:
shadowed ground keeps 0.45 to 0.55 of its brightness, and the spec's figures give
0.51, 0.48 and 0.55 for red, green and blue. The HQ's shadow falls to the right,
as in `camera-start.jpg`. Where Bevy's map differs from GFX-VIEW-2.5 (bias,
cascades) is listed in `docs/shadows.md`.

## Blending (fixed 2026-10-18)

GFX-VIEW-2.6 (c1): roads, crossings, the flag-to-door strip and the ground decal
are drawn with depth test less-or-equal, no depth write, no alpha test and
straight alpha blending, `src x a + dst x (1 - a)`, on the stored 8-bit values.
On an sRGB target the GPU would turn both colours into linear light, blend there
and encode again, so a half-covered pixel came out up to about 3 steps lighter.
Since the map camera composites on stored values (the table above), the blend is
the original's; the probe's two blended rows check it on the GPU. Our blending
state per family: `AlphaMode::Blend` for roads and decals (Bevy: source alpha /
one minus source alpha, add; depth write off; Bevy's reverse depth compares
greater-or-equal, the same test), and the shader never discards for those two
families. The order between a road and a decal where they overlap is Q-303.

## So why does it look darker and more saturated than the photos?

Not because of Bevy's colour handling: the probe shows the pipeline adds nothing.
What is left:

1. **The ground decals** were lit by ambient only (above); this was most of the
   "dark floor" and "dark rug". Fixed.
2. **The house light map (Q-219, answered: as specified).** GFX-OBJ-2.1 blends
   the `_L` texture in by saturate(1.5 - D) per channel. In full sun that weight is
   0.18 for red and 0.30 for green and blue, so a dark light map takes more green
   and blue than red away from the walls and the parts of the house mesh near
   the ground: darker and redder there. The investigation confirmed the term, its
   texture (the material's `Self-Illumination` map), set 1 and the per-channel
   weight (E-gfx-021), so the program keeps it; the original draws the same.
3. **D above 1 with a warm light.** Lit ground has D = 1.32 / 1.19 / 1.20, so red
   reaches 255 first and bright warm colours (the dome, yellow grass) clip toward
   saturated. This is the spec (2.3: no scaling of the sum).
4. **Blending** in linear light: fixed (above).
5. **Photo against screenshot.** The reference pictures are phone photos of a
   monitor. The phone's exposure and white balance, and the monitor's own
   settings, lift and grey them. The `look-compare` folder sets our screenshots
   beside them, but numbers can only be compared within one capture route.
