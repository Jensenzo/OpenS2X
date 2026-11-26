// A development switch, not a hook: the look study (`render/look_study.rs`,
// `--look-study <name>`, off by default). One pass over the map camera's
// picture after the 3D passes and before the interface, on the stored
// colour values (GFX-VIEW-2.6 c1), with the depth and normal prepasses
// (hook W17b's: the surfaces' own face normals in world space). Each look
// turns the whole picture (ground, water, models, effects, day and night)
// into one way of lighting, colouring and outlining; no rule of the
// original's is involved.

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct Look {
    view_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    // xyz: toward the sun (world, Bevy's frame).
    sun: vec4<f32>,
    // The look's numbers (`look_study.rs`, `Look::params`).
    params: vec4<f32>,
    // x: the look, y: pixels per pixel of a 1080 line picture, zw: the
    // picture's size.
    info: vec4<f32>,
    // xy: the world's origin on the screen in pixels (patterns anchored to
    // the ground, so they move with it when the camera pans), z: 1 when it
    // is in front of the camera.
    anchor: vec4<f32>,
};

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var depth: texture_2d<f32>;
@group(0) @binding(2) var normals: texture_2d<f32>;
@group(0) @binding(3) var look_sampler: sampler;
@group(0) @binding(4) var<uniform> look: Look;

const STORYBOOK: u32 = 1u;
const CEL: u32 = 2u;
const INK: u32 = 3u;
const MINIATURE: u32 = 4u;
const FACETS: u32 = 5u;
const PAPERCUT: u32 = 6u;
// Round 2, the looks of the materials (`look_materials.rs`): origtoy's pass
// is a little depth of field only; cel2 has no pass.
const ORIGTOY: u32 = 7u;
const CEL2: u32 = 8u;

fn size() -> vec2<i32> {
    return vec2<i32>(look.info.zw);
}

fn clamped(p: vec2<i32>) -> vec2<i32> {
    return clamp(p, vec2<i32>(0), size() - vec2<i32>(1));
}

fn colour_at(p: vec2<i32>) -> vec3<f32> {
    return textureLoad(screen, clamped(p), 0).rgb;
}

fn colour_uv(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(screen, look_sampler, uv, 0.0).rgb;
}

// Bevy's reverse depth: 1 at the near plane, 0 at infinity (the void).
fn raw_depth(p: vec2<i32>) -> f32 {
    return textureLoad(depth, clamped(p), 0).r;
}

fn normal_at(p: vec2<i32>) -> vec3<f32> {
    let n = textureLoad(normals, clamped(p), 0).xyz * 2.0 - vec3<f32>(1.0);
    let l = length(n);
    return select(vec3<f32>(0.0, 1.0, 0.0), n / l, l > 0.1);
}

fn view_position(p: vec2<i32>, d: f32) -> vec3<f32> {
    let uv = (vec2<f32>(p) + vec2<f32>(0.5)) / look.info.zw;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let h = look.view_from_clip * vec4<f32>(ndc, max(d, 1e-7), 1.0);
    return h.xyz / h.w;
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn saturated(c: vec3<f32>, s: f32) -> vec3<f32> {
    return max(mix(vec3<f32>(luminance(c)), c, s), vec3<f32>(0.0));
}

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + vec2<f32>(33.33));
    return fract((r.x + r.y) * r.x);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (vec2<f32>(3.0) - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// The pixel's place in a frame anchored to the ground (pixels).
fn anchored(p: vec2<i32>) -> vec2<f32> {
    return vec2<f32>(p) - look.anchor.xy * look.anchor.z;
}

// A Kuwahara filter: of the four squares of (r+1)^2 samples meeting at the
// pixel (every `stride` pixels), the mean of the one with the least
// variance. Keeps edges, turns fine detail into flat patches.
fn kuwahara(p: vec2<i32>, r: i32, stride: i32) -> vec3<f32> {
    var best = colour_at(p);
    var best_var = 1e9;
    let n = f32((r + 1) * (r + 1));
    for (var q = 0; q < 4; q++) {
        let sx = select(-1, 1, (q & 1) == 1);
        let sy = select(-1, 1, (q & 2) == 2);
        var sum = vec3<f32>(0.0);
        var sum2 = vec3<f32>(0.0);
        for (var j = 0; j <= r; j++) {
            for (var i = 0; i <= r; i++) {
                let c = colour_at(p + vec2<i32>(sx * i, sy * j) * stride);
                sum += c;
                sum2 += c * c;
            }
        }
        let mean = sum / n;
        let v = sum2 / n - mean * mean;
        let var_sum = v.r + v.g + v.b;
        if var_sum < best_var {
            best_var = var_sum;
            best = mean;
        }
    }
    return best;
}

// A generalized Kuwahara filter on a disc of `radius` pixels: eight
// sectors, each the mean of its samples, weighted by how even it is. Keeps
// edges, turns fine detail into round brush patches.
fn painterly(uv: vec2<f32>, radius: f32) -> vec3<f32> {
    let c0 = colour_uv(uv);
    var total = vec3<f32>(0.0);
    var weights = 0.0;
    for (var k = 0; k < 8; k++) {
        var sum = c0;
        var sum2 = c0 * c0;
        for (var ri = 1; ri <= 3; ri++) {
            for (var ai = 0; ai < 3; ai++) {
                let a = (f32(k) + (f32(ai) + 0.5) / 3.0) * 0.7853982;
                let r = radius * f32(ri) / 3.0;
                let c = colour_uv(uv + vec2<f32>(cos(a), sin(a)) * r / look.info.zw);
                sum += c;
                sum2 += c * c;
            }
        }
        let mean = sum / 10.0;
        let v = max(sum2 / 10.0 - mean * mean, vec3<f32>(0.0));
        let sigma = sqrt(v.r + v.g + v.b);
        let w = pow(max(sigma, 0.002), -6.0);
        total += mean * w;
        weights += w;
    }
    return total / weights;
}

// The mean colour of the surface the pixel lies on, over a disc of
// `radius` pixels: only the samples whose face normal turns less than
// `normal_cos` from the pixel's and whose depth is within `tolerance` of
// its own count, and only colours near the mean so far (with a strict
// `normal_cos` that is the pixel's triangle).
fn surface_mean(p: vec2<i32>, radius: f32, normal_cos: f32, tolerance: f32) -> vec3<f32> {
    let dc = raw_depth(p);
    let nc = normal_at(p);
    var sum = colour_at(p);
    var n = 1.0;
    if dc <= 0.0 {
        return sum;
    }
    for (var i = 0; i < 32; i++) {
        let r = sqrt((f32(i) + 0.5) / 32.0) * radius;
        let a = f32(i) * 2.3999632;
        let q = p + vec2<i32>(round(vec2<f32>(cos(a), sin(a)) * r));
        let dq = raw_depth(q);
        let cq = colour_at(q);
        let near_colour = length(cq - sum / n) < 0.22;
        if abs(dq - dc) < dc * tolerance && dot(normal_at(q), nc) > normal_cos && near_colour {
            sum += cq;
            n += 1.0;
        }
    }
    return sum / n;
}

// The outlines at the pixel, at a reach of `w` pixels: x the silhouettes
// (a jump in depth: the second difference of the reverse depth, which is
// zero across any flat surface, against the pixel's own), y the creases
// (the face normals' turn, over `crease_cos`). Both 0 to 1.
fn outlines(p: vec2<i32>, w: f32, depth_edge: f32, crease_cos: f32) -> vec2<f32> {
    let k = max(i32(round(w)), 1);
    let dc = raw_depth(p);
    let dl = raw_depth(p + vec2<i32>(-k, 0));
    let dr = raw_depth(p + vec2<i32>(k, 0));
    let du = raw_depth(p + vec2<i32>(0, -k));
    let dd = raw_depth(p + vec2<i32>(0, k));
    let far = max(max(dc, max(dl, dr)), max(du, dd));
    if far <= 0.0 {
        return vec2<f32>(0.0);
    }
    // Only the nearer side of a jump draws, so a line is `w` wide.
    let lap = max(abs(dl + dr - 2.0 * dc), abs(du + dd - 2.0 * dc));
    let mn = min(min(dl, dr), min(du, dd));
    let mx = max(max(dl, dr), max(du, dd));
    let nearer = select(0.0, 1.0, dc - mn > mx - dc);
    let rel = lap / max(dc, 1e-6);
    let silhouette = smoothstep(depth_edge, depth_edge * 2.5, rel) * nearer;
    let nc = normal_at(p);
    let turn = min(
        min(dot(nc, normal_at(p + vec2<i32>(k, 0))), dot(nc, normal_at(p + vec2<i32>(-k, 0)))),
        min(dot(nc, normal_at(p + vec2<i32>(0, k))), dot(nc, normal_at(p + vec2<i32>(0, -k)))),
    );
    let crease = select(0.0, 1.0 - smoothstep(crease_cos - 0.08, crease_cos, turn), dc > 0.0);
    return vec2<f32>(silhouette, crease);
}

// A glow of the bright parts: two rings of samples.
fn glow(uv: vec2<f32>, reach: f32) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let px = reach / look.info.zw;
    for (var i = 0; i < 12; i++) {
        let a = f32(i) * 0.5235988;
        let o = vec2<f32>(cos(a), sin(a));
        let c1 = colour_uv(uv + o * px);
        let c2 = colour_uv(uv + o.yx * vec2<f32>(1.0, -1.0) * px * 2.2);
        sum += max(c1 - vec3<f32>(0.8), vec3<f32>(0.0)) + max(c2 - vec3<f32>(0.8), vec3<f32>(0.0)) * 0.6;
    }
    return sum / 12.0;
}

// A disc blur of `radius` pixels (a golden-angle spiral of 24 samples).
fn disc_blur(uv: vec2<f32>, radius: f32) -> vec3<f32> {
    if radius < 0.5 {
        return colour_uv(uv);
    }
    var sum = vec3<f32>(0.0);
    for (var i = 0; i < 24; i++) {
        let r = sqrt((f32(i) + 0.5) / 24.0) * radius;
        let a = f32(i) * 2.3999632;
        sum += colour_uv(uv + vec2<f32>(cos(a), sin(a)) * r / look.info.zw);
    }
    return sum / 24.0;
}

fn sun_light(n: vec3<f32>) -> f32 {
    return max(dot(n, normalize(look.sun.xyz)), 0.0);
}

// 1. Storybook: painted patches, warm light, rich colour, a glow.
fn storybook(p: vec2<i32>, uv: vec2<f32>) -> vec3<f32> {
    let s = look.info.y;
    var c = painterly(uv, max(look.params.x * s, 1.0));
    // Warm light: the lights warmer, the shade a little warm too.
    let l = luminance(c);
    // (Blue things, the water and the sky's reflection, keep their blue.)
    let blue = smoothstep(0.0, 0.12, c.b - c.r);
    c = c * mix(vec3<f32>(1.04, 1.0, 0.91), vec3<f32>(1.0), blue)
        + vec3<f32>(0.03, 0.016, 0.0) * (1.0 - l) * (1.0 - blue);
    c = saturated(c, 1.0 + look.params.y);
    c += glow(uv, 9.0 * s) * look.params.z * vec3<f32>(1.0, 0.92, 0.75);
    return c;
}

// 2. Cel: two or three steps of light, flat colour, a thin dark silhouette.
fn cel(p: vec2<i32>) -> vec3<f32> {
    let s = look.info.y;
    // The surface's broad colour, and the light in steps by the light of
    // the pixel's own neighbourhood (the textures' grain left out).
    // Steps a factor apart (light, not the textures' brightness, makes
    // such jumps), each step's colour its middle.
    let base = surface_mean(p, 9.0 * s, 0.85, 0.03);
    let l = max(luminance(base), 1e-3);
    let steps = max(look.params.y, 2.0);
    let low = 0.12;
    let ratio = log(1.0 / low) / steps;
    let k = clamp(floor(log(l / low) / ratio), -1.0, steps - 1.0);
    let q = low * exp((k + 0.62) * ratio);
    var c = saturated(base * (q / l), 1.0 + look.params.z);
    let e = outlines(p, look.params.x * s, 0.012, 0.55);
    c = mix(c, c * 0.18, e.x);
    return c;
}

// 3. Ink: outlines heavier on silhouettes, washed colour on paper, hatching
// in the shade.
fn ink(p: vec2<i32>) -> vec3<f32> {
    let s = look.info.y;
    let base = kuwahara(p, 2, max(i32(round(s)), 1));
    let paper = vec3<f32>(0.955, 0.915, 0.83);
    let wash = look.params.z;
    let tint = clamp(saturated(base, 0.85) * 1.3, vec3<f32>(0.0), vec3<f32>(1.0));
    var c = paper * mix(vec3<f32>(1.0), tint, wash);
    // Hatching where the picture is dark, anchored to the ground.
    let l = luminance(base);
    let a = anchored(p) / (look.params.y * s);
    let h1 = abs(fract((a.x + a.y) * 0.7071) - 0.5);
    let h2 = abs(fract((a.x - a.y) * 0.7071) - 0.5);
    let line = 0.5 - 0.28;
    let shade1 = 1.0 - smoothstep(0.22, 0.40, l);
    let shade2 = 1.0 - smoothstep(0.10, 0.22, l);
    let ink_colour = vec3<f32>(0.16, 0.12, 0.09);
    let hatch = max(
        shade1 * (1.0 - smoothstep(line - 0.06, line, 0.5 - h1)),
        shade2 * (1.0 - smoothstep(line - 0.06, line, 0.5 - h2)),
    );
    let d = raw_depth(p);
    c = mix(c, ink_colour, hatch * 0.55 * select(0.0, 1.0, d > 0.0));
    // A light wash of the shade itself.
    c *= mix(0.78, 1.0, smoothstep(0.05, 0.45, l));
    let w = look.params.x * s;
    let heavy = outlines(p, w, 0.010, 0.6);
    let light = outlines(p, max(w * 0.5, 1.0), 0.010, 0.6);
    c = mix(c, ink_colour, max(heavy.x, light.y * 0.75));
    return c;
}

// 4. Miniature: stronger sun, more colour and contrast, sharp only in a
// middle band, a vignette.
fn miniature(p: vec2<i32>, uv: vec2<f32>) -> vec3<f32> {
    let s = look.info.y;
    let band = smoothstep(0.10, 0.42, abs(uv.y - 0.52));
    var c = disc_blur(uv, look.params.x * s * band);
    let n = normal_at(p);
    let lit = sun_light(n);
    let d = raw_depth(p);
    c *= select(1.0, 0.78 + 0.5 * lit, d > 0.0);
    c = max(mix(vec3<f32>(0.42), c, 1.0 + look.params.y), vec3<f32>(0.0));
    c = saturated(c, 1.0 + look.params.y * 1.6);
    let v = length((uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 0.75)) * 1.35;
    c *= 1.0 - look.params.z * pow(v, 2.6);
    return c;
}

// 5. Facets: each triangle lit by its own face normal, each surface its
// mean colours.
fn facets(p: vec2<i32>) -> vec3<f32> {
    let s = look.info.y;
    // The triangle's mean colour: the samples of the same face normal.
    let flat_colour = surface_mean(p, look.params.x * s, 0.9995, 0.01);
    let d = raw_depth(p);
    if d <= 0.0 {
        return flat_colour;
    }
    let n = normal_at(p);
    // The surface's smooth normal: its neighbours' face normals.
    let k = max(i32(round(7.0 * s)), 1);
    var nb = n;
    for (var i = 0; i < 8; i++) {
        let a = f32(i) * 0.7853982;
        let q = p + vec2<i32>(vec2<f32>(cos(a), sin(a)) * f32(k));
        let dq = raw_depth(q);
        if abs(dq - d) < d * 0.02 {
            nb += normal_at(q);
        }
    }
    nb = normalize(nb);
    let face = 0.42 + 0.58 * sun_light(n);
    let smooth_light = 0.42 + 0.58 * sun_light(nb);
    let ratio = mix(1.0, face / max(smooth_light, 0.05), look.params.y);
    return saturated(flat_colour * ratio, 1.0 + look.params.z);
}

// 6. Paper cut (ours): flat bands of colour, layers casting a soft shadow
// on what lies behind them, a light cut edge, paper grain.
fn papercut(p: vec2<i32>) -> vec3<f32> {
    let s = look.info.y;
    // Pastel paper in three bands of light.
    let base = surface_mean(p, 10.0 * s, 0.8, 0.03);
    let l = max(luminance(base), 1e-3);
    let bands = 3.0;
    let q = (floor(l * bands * 1.2) + 0.7) / (bands * 1.2);
    var c = saturated(base * (q / l), 0.9);
    // The paper's own tone, as bright as the band's light (dark at night).
    c = mix(c, vec3<f32>(0.97, 0.95, 0.9) * q * 1.6, 0.18);
    let d = raw_depth(p);
    // A layer nearer the camera up and to the left casts its shadow here.
    let reach = look.params.x * s;
    var shadow = 0.0;
    for (var i = 1; i <= 4; i++) {
        let o = vec2<i32>(vec2<f32>(-0.6, -1.0) * reach * f32(i) / 4.0);
        let dq = raw_depth(p + o);
        shadow += select(0.0, 1.0, dq > d * 1.01 + 1e-6);
    }
    c *= 1.0 - look.params.y * shadow / 4.0;
    // The cut edge of a nearer layer, lit.
    let e = outlines(p, max(s, 1.0), 0.012, 2.0);
    c = mix(c, min(c * 1.35 + vec3<f32>(0.08), vec3<f32>(1.0)), e.x * 0.8);
    let g = value_noise(anchored(p) / (2.5 * s)) * 0.6 + value_noise(anchored(p) / (9.0 * s)) * 0.4;
    c *= 1.0 + look.params.z * (g - 0.5);
    return c;
}

// origtoy: a little depth of field by distance. The picture's middle is
// in focus (the mean distance of five points around it); a pixel's blur
// grows with its distance's difference from that, as a share of it, up to
// `blur` pixels of a 1080 line picture (params.x). A disc of 16 taps; a tap
// nearer than the pixel and itself sharp does not bleed onto it.
fn view_distance(p: vec2<i32>) -> f32 {
    let d = raw_depth(p);
    if d <= 0.0 {
        return 1e6;
    }
    return length(view_position(p, d));
}

fn blur_of(distance: f32, focus: f32) -> f32 {
    let share = abs(distance - focus) / max(focus, 1e-3);
    return look.params.x * look.info.y * smoothstep(0.06, 0.5, share);
}

fn depth_of_field(p: vec2<i32>) -> vec3<f32> {
    let mid = size() / 2;
    let o = size() / 10;
    let focus = (view_distance(mid) + view_distance(mid + vec2<i32>(o.x, 0))
        + view_distance(mid - vec2<i32>(o.x, 0)) + view_distance(mid + vec2<i32>(0, o.y))
        + view_distance(mid - vec2<i32>(0, o.y))) / 5.0;
    let here = view_distance(p);
    let r = blur_of(here, focus);
    let c0 = colour_at(p);
    if r < 0.5 {
        return c0;
    }
    var sum = c0;
    var weight = 1.0;
    for (var i = 0; i < 16; i++) {
        let a = f32(i) * 2.39996;
        let t = sqrt((f32(i) + 0.5) / 16.0) * r;
        let q = p + vec2<i32>(vec2<f32>(cos(a), sin(a)) * t);
        let dq = view_distance(q);
        var w = 1.0;
        if dq < here {
            w = saturate(blur_of(dq, focus) / max(t, 1.0));
        }
        sum += colour_at(q) * w;
        weight += w;
    }
    return sum / weight;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let uv = in.uv;
    let id = u32(look.info.x);
    var c: vec3<f32>;
    switch id {
        case STORYBOOK: { c = storybook(p, uv); }
        case CEL: { c = cel(p); }
        case INK: { c = ink(p); }
        case MINIATURE: { c = miniature(p, uv); }
        case FACETS: { c = facets(p); }
        case PAPERCUT: { c = papercut(p); }
        case ORIGTOY: { c = depth_of_field(p); }
        default: { c = colour_at(p); }
    }
    return vec4<f32>(c, 1.0);
}
