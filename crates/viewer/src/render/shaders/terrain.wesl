// The 3D terrain (specs/GFX-world-view.md GFX-VIEW-2, GFX-terrain-patterns.md
// GFX-PAT-3, GFX-water.md GFX-WATER-2). Positions arrive in Bevy's frame,
// where z is the game's z negated; texture coordinates use the game's x and z.
// Colour math happens on display values, as the original's shaders do, and
// the shader writes the stored value itself (GFX-VIEW-2.6 c1, `stored.rs`).

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{globals, lights, view};
import bevy_pbr::render::shadows;
@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::render::mesh_view_bindings::screen_space_ambient_occlusion_texture;

const MAX_LAYERS: u32 = 64u;
// MAP-GRID-5.3: field spacing along a row and between rows.
const FIELD_STEP: f32 = 6.0;
const ROW_STEP: f32 = 5.1961501;

struct TerrainParams {
    // Per layer: texScale u, v and nrmScale u, v (GFX-PAT-3.1, 3.3).
    layers: array<vec4<f32>, 64>,
    // Per layer: the pattern's render type (GFX-WATER-2.1), its priority
    // (GFX-PAT-5.1) and its transition mask set (GFX-PAT-5.4).
    kinds: array<vec4<u32>, 64>,
    // GFX-VIEW-2.4a: per layer the highlight's intensity (x) and power
    // (y), zero but for type-3 patterns. The fields' order is the Rust
    // struct's (`materials::TerrainParams`).
    specular: array<vec4<f32>, 64>,
    // x, z of field (0, 0) in the game frame; width and height in fields.
    lattice: vec4<f32>,
    // GFX-VIEW-5.1: x0, z0, x1, z1 of the world texture.
    world_rect: vec4<f32>,
    // GFX-VIEW-2.3: the light's direction (Bevy frame; w: the shadow
    // factor), colour, ambient.
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    // GFX-WATER-2.5: x 1 while the caustics pass is drawn.
    caustics: vec4<f32>,
};

@group(3) @binding(0) var<uniform> params: TerrainParams;
@group(3) @binding(1) var colors: texture_2d_array<f32>;
@group(3) @binding(2) var colors_sampler: sampler;
// GFX-VIEW-2.4b: the normal maps at their own sizes, one array per size
// (128, 256, 512, 1024; `materials::NORMAL_SIZES`), each with its own mips.
@group(3) @binding(3) var normals: texture_2d_array<f32>;
@group(3) @binding(4) var normals_sampler: sampler;
@group(3) @binding(12) var normals_256: texture_2d_array<f32>;
@group(3) @binding(13) var normals_512: texture_2d_array<f32>;
@group(3) @binding(14) var normals_1024: texture_2d_array<f32>;
@group(3) @binding(5) var fields: texture_2d<u32>;
@group(3) @binding(6) var world: texture_2d<f32>;
@group(3) @binding(7) var world_sampler: sampler;
@group(3) @binding(8) var masks: texture_2d_array<f32>;
@group(3) @binding(9) var masks_sampler: sampler;
@group(3) @binding(10) var caustics: texture_2d<f32>;
@group(3) @binding(11) var caustics_sampler: sampler;

// GFX-WATER-2.5: the caustics on the underwater ground (game frame g) at
// time t, as `world3d::caustics_frames`, `caustics_uv` and
// `caustics_scale`: the red channel of frames f - 1 and f blended, times
// the light colour, 0.7, the wave term twice and the depth term; nothing at
// or above y -0.1. The original draws it as an additive second pass over the
// ground; added here to the ground's colour, before the fog of war, it
// gives the same pixels.
fn caustics_at(g: vec3<f32>, t: f32) -> vec3<f32> {
    if params.caustics.x == 0.0 || g.y >= -0.1 {
        return vec3<f32>(0.0);
    }
    let phase = fract(0.2 * t) * 16.0;
    let f = min(floor(phase), 15.0);
    let w = phase - floor(phase);
    let base = vec2<f32>(g.x / 16.0 * 0.04, g.z * 0.04);
    let now = textureSample(caustics, caustics_sampler, fract(base + vec2<f32>(f / 16.0, 0.0))).r;
    let before = textureSample(caustics, caustics_sampler,
        fract(base + vec2<f32>((f + 15.0) % 16.0 / 16.0, 0.0))).r;
    let s = sin(t);
    let wave = 1.0 - sin(0.4 * g.x + s) * sin(0.4 * g.y + s) * sin(0.4 * g.z + s);
    let depth = 1.0 - min(1.0, -0.15 * g.y);
    return mix(before, now, w) * params.light_color.rgb * 0.7 * wave * wave * depth;
}

// GFX-PAT-5.3 (Q-091 answered): the scale of the mask coordinates per world
// unit, 1 / (4 x the sample step along a row) = 1 / 6 with the shipped Map
// scale 1.5: one mask cell spans one field edge.
const MASK_SCALE: f32 = 1.0 / 6.0;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) haze: vec4<f32>,
    // GFX-VIEW-2.1a (b): the decoded tangent bytes.
    @location(3) tangent: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) haze: vec4<f32>,
    // GFX-WATER-2.4: the type-1 ground's texture shift, made at the vertex.
    @location(3) wobble: vec2<f32>,
    @location(4) tangent: vec3<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(v.position, 1.0));
    out.world_position = world_position.xyz;
    out.clip = position_world_to_clip(world_position.xyz);
    out.normal = v.normal;
    out.tangent = v.tangent.xyz;
    out.haze = v.haze;
    // GFX-WATER-2.4: the shift from the vertex's world position (game frame:
    // z is Bevy's z negated), interpolated over the triangle like the haze.
    let g = vec3<f32>(world_position.x, world_position.y, -world_position.z);
    let a = wobble_amplitude(g.y);
    let time = globals.time;
    out.wobble = vec2<f32>(
        a * sin((time + (g.z + g.y) / 0.4) * 2.25),
        a * sin((time + (g.x + g.y) / 0.4) * 2.25));
    return out;
}

fn layer_of(field: vec2<i32>) -> u32 {
    let size = vec2<i32>(params.lattice.zw);
    let f = clamp(field, vec2<i32>(0), size - vec2<i32>(1));
    return textureLoad(fields, f, 0).r;
}

fn field_position(f: vec2<i32>) -> vec2<f32> {
    let shift = select(0.0, FIELD_STEP / 2.0, (f.y & 1) == 1);
    return vec2<f32>(f32(f.x) * FIELD_STEP + shift, f32(f.y) * ROW_STEP);
}

// The three fields of the lattice triangle that holds the plane point p
// (relative to field (0, 0)) and its barycentric weights (MAP-GRID-4.5);
// outside the lattice the weights are clamped.
struct Triangle {
    a: vec2<i32>,
    b: vec2<i32>,
    c: vec2<i32>,
    w: vec3<f32>,
};

fn barycentric(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> vec3<f32> {
    let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    let w0 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / det;
    let w1 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / det;
    return vec3<f32>(w0, w1, 1.0 - w0 - w1);
}

fn containing(p: vec2<f32>) -> Triangle {
    let height = i32(params.lattice.w);
    let y = clamp(i32(floor(p.y / ROW_STEP)), 0, max(height - 2, 0));
    let base = select(0.0, FIELD_STEP / 2.0, (y & 1) == 1);
    let first = i32(floor((p.x - base) / FIELD_STEP));
    var best: Triangle;
    var best_score = -1e9;
    for (var x = first - 1; x <= first + 1; x++) {
        let a = vec2<i32>(x, y);
        let b = vec2<i32>(x + 1, y);
        let c = vec2<i32>(x, y + 1);
        let d = vec2<i32>(x + 1, y + 1);
        for (var k = 0; k < 2; k++) {
            var t: Triangle;
            if (y & 1) == 0 {
                if k == 0 { t.a = a; t.b = b; t.c = c; } else { t.a = b; t.b = d; t.c = c; }
            } else {
                if k == 0 { t.a = a; t.b = d; t.c = c; } else { t.a = a; t.b = b; t.c = d; }
            }
            t.w = barycentric(p, field_position(t.a), field_position(t.b), field_position(t.c));
            let score = min(t.w.x, min(t.w.y, t.w.z));
            if score > best_score {
                best_score = score;
                best = t;
            }
        }
    }
    let w = max(best.w, vec3<f32>(0.0));
    best.w = w / max(w.x + w.y + w.z, 1e-6);
    return best;
}

// GFX-VIEW-2.4b: a layer's normal-map texel, from the array of its map's
// size (kinds.w: array << 8 | layer), trilinear and wrapping.
fn normal_texel(layer: u32, st: vec2<f32>, gx: vec2<f32>, gy: vec2<f32>) -> vec4<f32> {
    let at = params.kinds[layer].w;
    let index = i32(at & 255u);
    switch at >> 8u {
        case 1u: {
            return textureSampleGrad(normals_256, normals_sampler, st, index, gx, gy);
        }
        case 2u: {
            return textureSampleGrad(normals_512, normals_sampler, st, index, gx, gy);
        }
        case 3u: {
            return textureSampleGrad(normals_1024, normals_sampler, st, index, gx, gy);
        }
        default: {
            return textureSampleGrad(normals, normals_sampler, st, index, gx, gy);
        }
    }
}

fn kind_of(layer: u32) -> u32 {
    return params.kinds[layer].x;
}

fn priority_of(layer: u32) -> u32 {
    return params.kinds[layer].y;
}

fn mask_set_of(layer: u32) -> u32 {
    return params.kinds[layer].z;
}

// GFX-WATER-2.4: the wobble amplitude of underwater ground.
fn wobble_amplitude(y: f32) -> f32 {
    if y > -0.15 {
        return 0.0;
    }
    if y > -0.7 {
        return -y * 1.42 * 0.008;
    }
    return 0.008;
}

// GFX-VIEW-2.3, 2.5: the factor of the direct light at a point: 1 where
// lit, the shadow factor (light_dir.w) in a shadow; the map's light's
// shadow map is sampled with 2 x 2 percentage-closer filtering.
fn shadow_at(world_position: vec3<f32>, normal: vec3<f32>, frag: vec4<f32>) -> f32 {
    if lights.n_directional_lights == 0u {
        return 1.0;
    }
    let p = vec4<f32>(world_position, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), p);
    let lit = shadows::fetch_directional_shadow(0u, p, normal, view_z, frag.xy);
    return mix(params.light_dir.w, 1.0, lit);
}

// GFX-VIEW-2.4: one pattern lit at the pixel, its texel brightened by
// `bright` (the transition mask's term, GFX-PAT-5.3).
fn shade(layer_in: u32, g: vec3<f32>, n: vec3<f32>, t: vec3<f32>, b: vec3<f32>,
        to_light: vec3<f32>, to_eye: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>, wobble: vec2<f32>,
        haze: vec4<f32>, bright: f32, shadow: f32, occlusion: f32) -> vec3<f32> {
    let layer = min(layer_in, MAX_LAYERS - 1u);
    let scales = params.layers[layer];
    let kind = kind_of(layer);
    // GFX-PAT-3.1: world position times the pattern's own scale.
    var st = vec2<f32>(g.x, g.z) * scales.xy;
    var nst = vec2<f32>(g.x, g.z) * scales.zw;
    if kind == 1u {
        // GFX-WATER-2.4: shifted by the vertex's wobble.
        st += wobble;
        nst += wobble;
    }
    let texel = textureSampleGrad(colors, colors_sampler, st, layer,
        vec2<f32>(dx.x, dx.z) * scales.xy, vec2<f32>(dy.x, dy.z) * scales.xy).rgb;
    let bump_texel = normal_texel(layer, nst,
        vec2<f32>(dx.x, dx.z) * scales.zw, vec2<f32>(dy.x, dy.z) * scales.zw);
    // GFX-VIEW-2.4b: N = 2 x texel - 1 as filtered, not normalised.
    let n_ts = bump_texel.rgb * 2.0 - 1.0;
    // GFX-VIEW-2.1a (c): the light turned into the frame whose rows are
    // (T, B, N), normalised at the pixel, lights the normal-map normal.
    let l_ts = normalize(vec3<f32>(dot(to_light, t), dot(to_light, b), dot(to_light, n)));
    // GFX-VIEW-2.4: D = light x saturate(N . L) x shadow + ambient, not
    // scaled down above 1 (hook W17b: the ambient times the occlusion, 1
    // without it).
    let d = params.light_color.rgb * saturate(dot(n_ts, l_ts)) * shadow
        + params.ambient.rgb * occlusion;
    var lit = d * (texel + vec3<f32>(bright));
    if kind == 1u {
        // GFX-WATER-2.3: underwater ground is blended toward the haze.
        lit = haze.rgb + (lit - haze.rgb) * haze.a;
    }
    let spec = params.specular[layer];
    if kind == 3u && spec.x > 0.0 {
        // GFX-VIEW-2.4a: the white highlight of a type-3 pattern, in
        // tangent space, R = normalize(2 x D x N - L) per channel as
        // written, masked by the normal map's alpha
        // (`world3d::specular_add`).
        let v_ts = vec3<f32>(dot(to_eye, t), dot(to_eye, b), dot(to_eye, n));
        let r = normalize(2.0 * d * n_ts - l_ts);
        lit += vec3<f32>(bump_texel.a * spec.x * pow(saturate(dot(r, v_ts)), spec.y));
    }
    return lit;
}

// Hook W17b: a project's screen-space ambient occlusion at the pixel `frag`
// (its value in Bevy's occlusion texture, 1 unoccluded); 1 on every
// camera without it (the original's look, the water's mirror).
// Bevy's occlusion on a flat surface (High quality: 1 - 1/6, view3d's
// `AO_FLAT`): divided out, so open ground reads 1.
const AO_FLAT: f32 = 0.8333333;

fn ambient_occlusion(frag: vec4<f32>) -> f32 {
    var ao = 1.0;
    @if(SCREEN_SPACE_AMBIENT_OCCLUSION) {
        let p = vec2<i32>(frag.xy - view.viewport.xy);
        ao = min(textureLoad(screen_space_ambient_occlusion_texture, p, 0i).r / AO_FLAT, 1.0);
    }
    return ao;
}

// GFX-WATER-2.6a: the mirror camera (`view3d/mirror.rs`) is the only one
// whose transform holds a reflection (a negative determinant).
fn mirror_view() -> bool {
    let m = view.world_from_view;
    return dot(cross(m[0].xyz, m[1].xyz), m[2].xyz) < 0.0;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // GFX-WATER-2.6a: the mirror image's clip plane at the surface's level
    // (-0.1, GFX-WATER-3.3) keeps the side of the camera above it.
    let mirrored = mirror_view();
    if mirrored && in.world_position.y < -0.1 {
        discard;
    }
    // GFX-WATER-2.6a 3: back faces culled, the cull mode reversed in the
    // mirror view with its reflection (the pipeline culls nothing).
    if front == mirrored {
        discard;
    }
    // The game frame: z is Bevy's z negated.
    let g = vec3<f32>(in.world_position.x, in.world_position.y, -in.world_position.z);
    let p = vec2<f32>(g.x, g.z) - params.lattice.xy;
    let tri = containing(p);
    let layers = array<u32, 3>(layer_of(tri.a), layer_of(tri.b), layer_of(tri.c));

    // GFX-VIEW-2.1a: the vertex's decoded normal and tangent bytes, not
    // renormalised; the bitangent B = T x N of the game frame, which in
    // Bevy's frame (z negated, a reflection) is cross(n, t)
    // (`terrain3d::tests::the_shaders_bitangent_is_t_cross_n`).
    let n = in.normal;
    let t = in.tangent;
    let b = cross(n, t);
    let to_light = -params.light_dir.xyz;
    let to_eye = normalize(view.world_position - in.world_position);
    let dx = dpdx(g);
    let dy = dpdy(g);
    let time = globals.time;
    let shadow = shadow_at(in.world_position, normalize(n), in.clip);
    // Hook W17b: the screen-space ambient occlusion at the pixel (1
    // without the hook).
    let occlusion = ambient_occlusion(in.clip);

    // GFX-PAT-5.1, 5.2: the corner pattern of lowest priority is the base;
    // every other corner pattern is laid over it in priority order.
    var order = array<u32, 3>(0u, 1u, 2u);
    for (var i = 0u; i < 2u; i++) {
        for (var j = 0u; j < 2u - i; j++) {
            if priority_of(layers[order[j]]) > priority_of(layers[order[j + 1u]]) {
                let x = order[j];
                order[j] = order[j + 1u];
                order[j + 1u] = x;
            }
        }
    }
    let corners = array<vec2<i32>, 3>(tri.a, tri.b, tri.c);
    let base = layers[order[0]];
    var colour = shade(base, g, n, t, b, to_light, to_eye, dx, dy, in.wobble, in.haze, 0.0,
        shadow, occlusion);
    var done = base;
    var done2 = base;
    for (var k = 1u; k < 3u; k++) {
        let layer = layers[order[k]];
        if layer == done || layer == done2 {
            continue;
        }
        done2 = done;
        done = layer;
        // GFX-PAT-5.3: one corner: the mask opaque at its centre, anchored
        // there; two corners: the mask clear at its centre, anchored at the
        // third.
        var count = 0u;
        for (var c = 0u; c < 3u; c++) {
            if layers[c] == layer {
                count += 1u;
            }
        }
        var anchor = corners[order[k]];
        var h = 0.5;
        if count == 1u {
            anchor = corners[order[k]];
        } else {
            h = 0.0;
            for (var c = 0u; c < 3u; c++) {
                if layers[c] != layer {
                    anchor = corners[c];
                }
            }
        }
        let a = field_position(anchor);
        let half_texel = vec2<f32>(0.5 / 256.0, 0.5 / 128.0);
        let mask_uv = vec2<f32>(
            ((p.x - a.x) * MASK_SCALE + 1.0) * 0.25 + h,
            ((p.y - a.y) * MASK_SCALE + 1.0) * 0.5) + half_texel;
        let mask = textureSampleLevel(masks, masks_sampler, mask_uv, mask_set_of(layer), 0.0);
        let over = shade(layer, g, n, t, b, to_light, to_eye, dx, dy, in.wobble, in.haze,
            mask.r * 2.0 - 1.0, shadow, occlusion);
        colour = mix(colour, over, mask.a);
    }

    colour += caustics_at(g, time);

    // GFX-VIEW-2.4, 5.1: the field's visibility from the world texture.
    let rect = params.world_rect;
    let uv = (vec2<f32>(g.x, g.z) - rect.xy) / (rect.zw - rect.xy);
    let visible = textureSampleLevel(world, world_sampler, uv, 0.0).r;
    colour *= visible;
    return vec4<f32>(saturate(colour), 1.0);
}
