// Hook W17b: the terrain's depth and normal prepass for a map camera with a
// project's screen-space ambient occlusion. The positions as terrain.wesl
// places them (Bevy's frame). The normal written is the surface's own, made
// from the screen derivatives of the world position and turned toward the
// camera, so that it agrees exactly with the depth the occlusion reads (the
// smoothed grid normals of GFX-VIEW-2.1a do not, and made open ground look
// occluded by some 20 %); Bevy's encoding, n x 0.5 + 0.5. Never used by
// the original's look (no prepass on its cameras).

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::{direction_world_to_view, position_world_to_clip};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(v.position, 1.0));
    out.clip = position_world_to_clip(world_position.xyz);
    out.world_position = world_position.xyz;
    return out;
}

// The surface's normal at the pixel, toward the camera (view space z > 0).
fn surface_normal(p: vec3<f32>) -> vec3<f32> {
    var n = normalize(cross(dpdx(p), dpdy(p)));
    if direction_world_to_view(n).z < 0.0 {
        n = -n;
    }
    return n;
}

@if(!NORMAL_PREPASS)
@fragment
fn fragment(in: VertexOutput) {
}

@if(NORMAL_PREPASS)
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(surface_normal(in.world_position) * 0.5 + vec3<f32>(0.5), 1.0);
}
