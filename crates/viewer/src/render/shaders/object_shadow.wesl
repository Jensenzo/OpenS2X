// The objects drawn into the light's shadow map (specs/GFX-world-view.md
// GFX-VIEW-2.5 a): each with its own shape (bone palette, a tree's sway, a
// flag's wave, as object.wesl moves them), alpha-tested casters cut out
// where the texture's alpha, sampled trilinearly, is below 0.5. The same
// shader is the objects' depth and normal prepass of a map camera with a
// project's ambient occlusion (hook W17b): there it also writes the
// surface's own normal, from the screen derivatives of the world position
// (so it agrees with the depth the occlusion reads; the models' smoothed
// normals do not), turned toward the camera.

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::{direction_world_to_view, position_world_to_clip};
import bevy_render::globals::Globals;

const TREE: u32 = 3u;
const FLAG: u32 = 9u;

// The prepass view group: the view at 0, the globals at 1.
@group(0) @binding(1) var<uniform> globals: Globals;

struct ObjectParams {
    tint: vec4<f32>,
    world_rect: vec4<f32>,
    mode: vec4<f32>,
    extra: vec4<f32>,
    player: vec4<f32>,
};

@group(3) @binding(0) var<uniform> params: ObjectParams;
@group(3) @binding(1) var base: texture_2d<f32>;
@group(3) @binding(2) var base_sampler: sampler;

// F51: every material's palette in one buffer, this one's from matrix
// params.mode.y.
@group(3) @binding(5) var<storage, read> palettes: array<mat4x4<f32>>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) bone_index: vec4<u32>,
    @location(4) bone_weight: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_y: f32,
    // Hook W17b: the world position (Bevy's frame) for the normal prepass.
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let w = v.bone_weight;
    let o = u32(params.mode.y + 0.5);
    let skin = palettes[o + v.bone_index.x] * w.x + palettes[o + v.bone_index.y] * w.y
        + palettes[o + v.bone_index.z] * w.z + palettes[o + v.bone_index.w] * w.w;
    var local = skin * vec4<f32>(v.position, 1.0);
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let tx = world_from_local[3].x;
    let tz = -world_from_local[3].z;
    let m22 = length(world_from_local[1].xyz);
    let t = globals.time;
    let f = u32(params.mode.x + 0.5);
    if f == TREE {
        let p = local.xyz;
        let s = saturate(dot(p, p) * 0.003) * m22;
        local.x += s * sin(0.35 * 3.14159265 * tx + t);
        local.z -= s * cos(0.35 * 3.14159265 * tz + t);
    } else if f == FLAG {
        let p = vec3<f32>(local.x, local.y, -local.z);
        let s = 0.33 * saturate(-0.02 + p.x * p.x + p.z * p.z);
        let w = tx + tz + p.x + p.y + t;
        let c = 0.5 + 0.5 * cos(4.0 * w);
        local.z -= s * c;
        local.x += s * (1.0 - c);
    }
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(local.xyz, 1.0));
    out.clip = position_world_to_clip(world_position.xyz);
    out.uv = v.uv;
    out.world_y = world_position.y;
    out.world_position = world_position.xyz;
    return out;
}

// The shape test of both passes: a texel under the alpha cut, or the part
// of a house being built that is not yet there, is discarded.
fn cut(in: VertexOutput) {
    // GFX-VIEW-2.5 a: the caster's alpha is sampled as the original's
    // shadow pass does, trilinear with the texture's own mips chosen by the
    // shadow map's derivatives (not mip 0), then cut at 0.5: far and small
    // leaves are cut on averaged alpha and let light through.
    let alpha = textureSample(base, base_sampler, in.uv).a;
    // A house being built casts only below its rising plane (GFX-OBJ-2.6b).
    if u32(params.mode.x + 0.5) == 2u {
        if params.extra.z > 1.5 && in.world_y < params.extra.y {
            discard;
        }
        if params.extra.z > 0.5 && params.extra.z <= 1.5 && in.world_y > params.extra.y {
            discard;
        }
    }
    if alpha < 0.5 {
        discard;
    }
}

// The shadow map and a depth-only prepass write depth only.
@if(!NORMAL_PREPASS)
@fragment
fn fragment(in: VertexOutput) {
    cut(in);
}

// Hook W17b: the normal prepass (Bevy's encoding, n x 0.5 + 0.5) for the
// screen-space ambient occlusion: the surface's normal toward the camera
// (view space z > 0), the derivatives taken before the cut.
@if(NORMAL_PREPASS)
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position;
    var n = normalize(cross(dpdx(p), dpdy(p)));
    cut(in);
    if direction_world_to_view(n).z < 0.0 {
        n = -n;
    }
    return vec4<f32>(n * 0.5 + vec3<f32>(0.5), 1.0);
}
