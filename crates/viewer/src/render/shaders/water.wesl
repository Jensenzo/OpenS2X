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
    // The ground study (a development switch, `ground_study.rs`): x 1 when
    // on, y the lightness step, z the variant; the shallow, middle and deep
    // colours.
    study: vec4<f32>,
    study_water: array<vec4<f32>, 3>,
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

// The ground study: the water as flat facets of the sample lattice (as
// the terrain's), each one colour by the depth at its middle (shallow,
// middle, deep), stepped by its place, under the sun and sky; opaque but for
// the fog of war.
const STUDY_STEP: f32 = 1.5;
const STUDY_ROW_STEP: f32 = 1.2990375;

fn study_water(g: vec3<f32>) -> vec4<f32> {
    let rect = params.world_rect;
    // The facet: the lattice row and column, and which half of the cell.
    let row = floor((g.z - rect.y) / STUDY_ROW_STEP);
    let shift = select(0.0, STUDY_STEP / 2.0, (i32(row) & 1) == 1);
    let u = (g.x - rect.x - shift) / STUDY_STEP;
    let v = (g.z - rect.y) / STUDY_ROW_STEP - row;
    let column = floor(u);
    let upper = select(0u, 1u, fract(u) + v > 1.0);
    let middle_x = rect.x + shift + (column + select(0.33, 0.67, upper == 1u)) * STUDY_STEP;
    let middle_z = rect.y + (row + select(0.33, 0.67, upper == 1u)) * STUDY_ROW_STEP;
    let middle_uv = (vec2<f32>(middle_x, middle_z) - rect.xy) / (rect.zw - rect.xy);
    let depth = textureSampleLevel(world, world_sampler, middle_uv, 0.0).g;
    // The depth term is 1 at the waterline, 0 deep (GFX-WATER-4.4).
    var colour = params.study_water[2].rgb;
    if depth > 0.7 {
        colour = params.study_water[0].rgb;
    } else if depth > 0.3 {
        colour = params.study_water[1].rgb;
    }
    var h = (u32(i32(column) + 65536) * 73856093u) ^ (u32(i32(row) + 65536) * 19349663u)
        ^ ((upper + 1u) * 83492791u);
    h = (h ^ (h >> 13u)) * 1274126177u;
    h = h ^ (h >> 16u);
    colour *= 1.0 + params.study.y * (f32(h % 3u) - 1.0);
    let to_light = -params.light_dir.xyz;
    let d = params.light_color.rgb * saturate(to_light.y) + params.ambient.rgb;
    let uv = (vec2<f32>(g.x, g.z) - rect.xy) / (rect.zw - rect.xy);
    let visible = textureSampleLevel(world, world_sampler, uv, 0.0).r;
    return vec4<f32>(saturate(d * colour * visible), 1.0);
}

// The ground study, round 2 (variants 4 to 6, study.z): the water one flat
// surface over the ground (drawn under it in its own colour), its colour
// pale to deep and its opacity from see-through to whole by the depth under
// it, so that the shore is the line where the surface cuts the land; under
// the sun and sky, no waves, no mirror, no highlight.
fn study_surface(g: vec3<f32>) -> vec4<f32> {
    let rect = params.world_rect;
    let uv = (vec2<f32>(g.x, g.z) - rect.xy) / (rect.zw - rect.xy);
    let texel = textureSampleLevel(world, world_sampler, uv, 0.0);
    // The depth term: 1 at the waterline, 0 deep (GFX-WATER-4.4).
    let deep = saturate(1.0 - texel.g);
    var colour = mix(params.study_water[0].rgb, params.study_water[1].rgb, saturate(deep * 2.0));
    colour = mix(colour, params.study_water[2].rgb, saturate(deep * 2.0 - 1.0));
    // Half see-through at the waterline, so the line where the surface
    // cuts the land shows; opaque only over the deepest ground.
    var alpha = mix(0.5, 1.0, smoothstep(0.0, 1.0, deep));
    if params.study.z > 6.5 {
        // Round 3 (the hybrids): see-through further from the shore (the
        // opacity rises slowly at first). The pale edge where the surface
        // meets the land is drawn on the ground (terrain.wesl).
        alpha = mix(0.3, 1.0, pow(smoothstep(0.0, 1.0, deep), 1.8));
    }
    let d = params.light_color.rgb * saturate(-params.light_dir.y) + params.ambient.rgb;
    let visible = texel.r;
    return vec4<f32>(saturate(d * colour * visible), saturate(alpha + 1.0 - visible));
}

// A development switch, not a hook: the look study's looks of the
// materials (`look_materials.rs`; shader defs LOOK_TOY, LOOK_CEL). origtoy:
// a glossy toy blue with the waves' ripples and a sharp highlight; cel2:
// two flat blues by the depth and drawn foam: a band along the shore and a
// thin line further out, both breathing in and out.
fn look_water(in: VertexOutput, g: vec3<f32>) -> vec4<f32> {
    let rect = params.world_rect;
    let uv = (vec2<f32>(g.x, g.z) - rect.xy) / (rect.zw - rect.xy);
    let texel = textureSampleLevel(world, world_sampler, uv, 0.0);
    let deep = saturate(1.0 - texel.g);
    let visible = texel.r;
    let time = globals.time;
    let day = (params.light_color.rgb + params.ambient.rgb) / vec3<f32>(1.54, 1.40, 1.38);
    var colour = vec3<f32>(0.0);
    var alpha = 1.0;
    @if(LOOK_TOY) {
        let st0 = rotate(vec2<f32>(g.x * 0.19, g.z * 0.17) * 0.3, params.rotation.x)
            + params.drift.xy * time;
        let st1 = rotate(vec2<f32>(g.z * 0.23, g.x * 0.19) * 0.3, params.rotation.y)
            + params.drift.zw * time;
        let n = normalize(0.5 * (wave_normal(textureSample(wave0, wave0_sampler, st0).rgb)
            + wave_normal(textureSample(wave1, wave1_sampler, st1).rgb)) + vec3<f32>(0.0, 1.5, 0.0));
        var base = mix(vec3<f32>(0.42, 0.78, 0.86), vec3<f32>(0.18, 0.62, 0.77), saturate(deep * 3.0));
        base = mix(base, vec3<f32>(0.12, 0.49, 0.66), saturate(deep * 2.0 - 0.6));
        let v = normalize(view.world_position.xyz - in.world_position);
        let to_light = -params.light_dir.xyz;
        let h = normalize(to_light + v);
        let spec = pow(saturate(dot(h, n)), 90.0) * 0.9 + pow(saturate(dot(h, n)), 14.0) * 0.08;
        colour = base * day * (0.85 + 0.15 * saturate(dot(n, to_light))) + params.light_color.rgb * spec;
        alpha = mix(0.6, 0.97, saturate(deep * 4.0));
    }
    @if(LOOK_CEL) {
        colour = select(vec3<f32>(0.227, 0.635, 0.910), vec3<f32>(0.165, 0.525, 0.800), deep > 0.45);
        colour = select(vec3<f32>(0.45, 0.80, 0.95), colour, deep > 0.06);
        let breathe = sin(time * 1.1 + (g.x + g.z) * 0.15);
        let shore = deep < 0.03 + 0.012 * breathe;
        let line = abs(deep - (0.12 + 0.02 * sin(time * 0.8 + g.x * 0.21))) < 0.008;
        if shore || line {
            colour = vec3<f32>(1.0);
        }
        colour *= day;
        alpha = select(0.85, 1.0, deep > 0.06);
    }
    return vec4<f32>(saturate(colour * visible), saturate(alpha + 1.0 - visible));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let g = vec3<f32>(in.world_position.x, in.world_position.y, -in.world_position.z);
    @if(LOOK_TOY) {
        if params.mirror.x < 1.5 {
            return look_water(in, g);
        }
    }
    @if(LOOK_CEL) {
        if params.mirror.x < 1.5 {
            return look_water(in, g);
        }
    }
    if params.study.x > 0.5 && params.mirror.x < 1.5 {
        if params.study.z > 3.5 {
            return study_surface(g);
        }
        return study_water(g);
    }
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
