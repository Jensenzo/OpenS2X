// UI-GAME-4.5: the ship window's view of its ship, behind the porthole:
// the picture the ship camera took, let through where `uiShipMask` (white,
// its alpha the porthole's round glass) is opaque. Written as the stored
// value, as the rest of the interface (GFX-VIEW-2.6 c, `ui_stored.wesl`).

import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::color_operations::linear_to_srgb;

@group(1) @binding(0) var view_texture: texture_2d<f32>;
@group(1) @binding(1) var view_sampler: sampler;
@group(1) @binding(2) var mask_texture: texture_2d<f32>;
@group(1) @binding(3) var mask_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let view = textureSample(view_texture, view_sampler, in.uv);
    let mask = textureSample(mask_texture, mask_sampler, in.uv);
    return vec4<f32>(linear_to_srgb(view.rgb), mask.a);
}
