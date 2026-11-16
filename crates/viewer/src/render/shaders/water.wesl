// The water surface (specs/GFX-water.md GFX-WATER-4): RenderWater with the
// mirror image of GFX-WATER-2.6 while it is bound, else RenderWaterNoMirror. Positions arrive in Bevy's frame;
// texture coordinates use the game's x and z (Bevy's z negated). It writes
// stored values, blended over the ground as stored (GFX-VIEW-2.6 c1).

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{directional_shadow_textures, globals, lights, view};
import bevy_pbr::render::mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT;
import bevy_pbr::render::shadows;

struct WaterParams {
    // GFX-WATER-5.2: the water style.
    water_color: vec4<f32>,
    wave_color: vec4<f32>,
    // opacity x, y; z 1 for the surface passes, which test the shadow map
    // (GFX-WATER-4.5a a), 0 for the map border's (GFX-WATER-6.1); unused w.
    opacity: vec4<f32>,
    // GFX-WATER-4.2: rotation (radians) of map 0 and 1, then drift speeds.
    rotation: vec4<f32>,
    drift: vec4<f32>,
    world_rect: vec4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    // GFX-WATER-2.6, 4.3: x 1 while the mirror image is bound, y the
    // reflection factor, z and w the reflection noise.
    mirror: vec4<f32>,
    // Hook W18 (a project's, not the original's): the shore foam's colour
    // and strength (w, 0 none), and its reach (x of foam_reach).
    foam: vec4<f32>,
    foam_reach: vec4<f32>,
};

@group(3) @binding(0) var<uniform> params: WaterParams;
@group(3) @binding(1) var wave0: texture_2d<f32>;
@group(3) @binding(2) var wave0_sampler: sampler;
@group(3) @binding(3) var wave1: texture_2d<f32>;
@group(3) @binding(4) var wave1_sampler: sampler;
@group(3) @binding(5) var world: texture_2d<f32>;
@group(3) @binding(6) var world_sampler: sampler;
@group(3) @binding(7) var mirror: texture_2d<f32>;
@group(3) @binding(8) var mirror_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    // GFX-WATER-2.6: the projected position, divided per pixel.
    @location(1) projected: vec4<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(v.position, 1.0));
    out.world_position = world_position.xyz;
    out.clip = position_world_to_clip(world_position.xyz);
    out.projected = out.clip;
    return out;
}

fn rotate(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

// A wave normal map texel as a Bevy-frame normal: the map's third channel
// is up (GFX-WATER-4.2), its second the game's +z.
fn wave_normal(texel: vec3<f32>) -> vec3<f32> {
    let n = texel * 2.0 - 1.0;
    return vec3<f32>(n.x, n.z, -n.y);
}

// The mirror texture is sRGB-encoded: its bytes are the stored values
// (GFX-VIEW-2.6 c1), which a read decodes; encoded again they come back.
fn stored(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// GFX-WATER-4.5a (b), T21: the wave offset of the shadow lookup as a shift
// on the water plane. The original adds reflectionNoise x (N_x, N_z) to its
// shadow map's projective x and y; our map is another (cascades), so the tap
// is taken at the water point p moved by
// D = R N_x (1.33, 1.37) + R N_z k (c - p), c = the eye's ground position +
// (63.0, 19.9), k = 0.0315 (game frame x, z; the default camera's values, the
// ones to use where no other is known). `g_n` is the normal's game x and z,
// `noise` R for each; positions in Bevy's frame (z negated).
const SHADOW_SHIFT_X: vec2<f32> = vec2<f32>(1.33, 1.37);
const SHADOW_SHIFT_K: f32 = 0.0315;
const SHADOW_SHIFT_C: vec2<f32> = vec2<f32>(63.0, 19.9);

fn shadow_point(world_position: vec3<f32>, eye: vec3<f32>, g_n: vec2<f32>, noise: vec2<f32>) -> vec3<f32> {
    let p = vec2<f32>(world_position.x, -world_position.z);
    let c = vec2<f32>(eye.x, -eye.z) + SHADOW_SHIFT_C;
    let d = noise.x * g_n.x * SHADOW_SHIFT_X + noise.y * g_n.y * SHADOW_SHIFT_K * (c - p);
    return vec3<f32>(world_position.x + d.x, world_position.y, world_position.z - d.y);
}

// GFX-WATER-4.5a (b): one point-sampled tap of the shadow map at the shifted
// surface point (`shadow_point`; no percentage-closer filter): 1 lit, 0 in a
// shadow; 1 while the frame draws no shadows (a). The cascade is the one the
// ground takes at that distance (no blend between cascades: the test is lit
// or not), the point biased as the ground's is (docs/shadows.md), with the
// surface's up normal. Bevy's maps store reversed depth: the point is lit
// when it lies at least as near the light as the stored caster.
fn shadow_lit(world_position: vec3<f32>) -> f32 {
    if lights.n_directional_lights == 0u {
        return 1.0;
    }
    let light = &lights.directional_lights[0];
    if ((*light).flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u {
        return 1.0;
    }
    let p = vec4<f32>(world_position, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), p);
    let cascade_index = shadows::get_cascade_index(0u, view_z);
    if cascade_index >= (*light).num_cascades {
        return 1.0;
    }
    let cascade = &(*light).cascades[cascade_index];
    let normal_offset = (*light).shadow_normal_bias * (*cascade).texel_size * vec3<f32>(0.0, 1.0, 0.0);
    let depth_offset = (*light).shadow_depth_bias * (*light).direction_to_light.xyz;
    let local = shadows::world_to_directional_light_local(0u, cascade_index,
        vec4<f32>(world_position + normal_offset + depth_offset, 1.0));
    if local.w == 0.0 {
        return 1.0;
    }
    let size = vec2<i32>(textureDimensions(directional_shadow_textures).xy);
    let texel = clamp(vec2<i32>(floor(local.xy * vec2<f32>(size))), vec2<i32>(0), size - 1);
    let layer = i32((*light).depth_texture_base_index + cascade_index);
    let stored = textureLoad(directional_shadow_textures, texel, layer, 0);
    return select(0.0, 1.0, local.z >= stored);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let g = vec3<f32>(in.world_position.x, in.world_position.y, -in.world_position.z);
    let time = globals.time;
    // GFX-WATER-4.2: two maps, each scaled from the world position, rotated
    // and shifted by its drift times the render time.
    let st0 = rotate(vec2<f32>(g.x * 0.19, g.z * 0.17) * 0.3, params.rotation.x)
        + params.drift.xy * time;
    let st1 = rotate(vec2<f32>(g.z * 0.23, g.x * 0.19) * 0.3, params.rotation.y)
        + params.drift.zw * time;
    let n0 = wave_normal(textureSample(wave0, wave0_sampler, st0).rgb);
    let n1 = wave_normal(textureSample(wave1, wave1_sampler, st1).rgb);
    // GFX-WATER-4.2 (T16b): their plain mean, not normalised; every use
    // below takes it as it is.
    let n = 0.5 * (n0 + n1);

    // GFX-WATER-4.3: darker where the surface faces the sun.
    let to_light = -params.light_dir.xyz;
    let d = params.light_color.rgb * (1.0 - 0.6 * saturate(dot(to_light, n))) + params.ambient.rgb;
    let base = mix(params.wave_color.rgb, params.water_color.rgb, n.y);
    var colour = d * base;
    if params.mirror.x > 0.5 {
        // GFX-WATER-2.6, 2.6a, 4.3 (RenderWater): the mirror image at the
        // surface point's own place on the screen (x x 0.5, y x -0.5, plus
        // 0.5; the main view-projection of the frame, the mirror picture
        // lines up with the screen), offset by the reflection noise times
        // the normal's x and z added before the division by w.
        let g_n = vec2<f32>(n.x, -n.z);
        let p = in.projected.xy + params.mirror.zw * g_n;
        let screen = vec2<f32>(p.x / in.projected.w * 0.5 + 0.5, p.y / in.projected.w * -0.5 + 0.5);
        let image = stored(textureSampleLevel(mirror, mirror_sampler, screen, 0.0).rgb);
        colour = saturate(d) * mix(base, image, params.mirror.y * d.r);
        if params.mirror.x > 1.5 {
            return vec4<f32>(image, 1.0);
        }
    }

    // GFX-WATER-4.4: see-through from above, times the depth term.
    let v = normalize(view.world_position.xyz - in.world_position);
    let rect = params.world_rect;
    let uv = (vec2<f32>(g.x, g.z) - rect.xy) / (rect.zw - rect.xy);
    // GFX-WATER-6.2: the skirt around the map lies outside the world
    // texture, which is sampled with clamp addressing: each skirt pixel takes
    // the nearest edge field's fog of war and depth, black until the player
    // has explored that field.
    let texel = textureSampleLevel(world, world_sampler, uv, 0.0);
    let depth = texel.g;
    var alpha = saturate(params.opacity.y - abs(v.y) * params.opacity.x)
        * saturate(1.2 - depth);

    // GFX-WATER-4.5a (c), (d): in a shadow, colour and alpha alike times
    // clamp(s + 0.8), s the receivers' factor (GFX-VIEW-2.3 d, light_dir.w):
    // 0.9 by day, 1 (no change) whenever s >= 0.2. Before the highlight.
    if params.opacity.z > 0.5 {
        let tap = shadow_point(in.world_position, view.world_position.xyz,
            vec2<f32>(n.x, -n.z), params.mirror.zw);
        let shadow = mix(params.light_dir.w, 1.0, shadow_lit(tap));
        let factor = saturate(shadow + 0.8);
        colour *= factor;
        alpha *= factor;
    }

    // GFX-WATER-4.5: the highlight (not shadowed, 4.5a d).
    let h = normalize(to_light + v);
    let s = saturate(pow(max(dot(h, n), 0.0), 16.0));
    colour += s * (1.0 - colour);
    alpha += 0.35 * s;

    // Hook W18: a project's shore foam where the ground lies less than
    // the reach below the water level (the depth term: 1 at the waterline,
    // 0 about 2900 raw units down); none by default (strength 0).
    if params.foam.w > 0.0 {
        let below = (1.0 - depth) * 2900.0;
        let f = saturate(1.0 - below / params.foam_reach.x) * params.foam.w;
        colour = mix(colour, params.foam.rgb, f);
        alpha = max(alpha, f);
    }

    // GFX-WATER-4.6: fog of war.
    let visible = texel.r;
    colour *= visible;
    alpha += 1.0 - visible;
    return vec4<f32>(saturate(colour), saturate(alpha));
}
