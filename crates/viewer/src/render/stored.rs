//! GFX-VIEW-2.6 (c1): the map camera composites on stored colour values.
//!
//! The original blends its see-through layers (roads, crossings, the
//! flag-to-door strip, the ground decal, water) on the 8-bit values its
//! shaders write: out = rgb x a + d x (1 - a), with no gamma anywhere. An
//! sRGB render target would blend in linear light instead (half-covered
//! pixels a few steps lighter, T25, T26). So the map camera renders into a
//! float main texture (`Hdr`) where every shader writes the stored value
//! itself, the hardware blends those straight, and one pass after the 3D
//! passes and before the interface turns the stored values into the linear
//! values the window's sRGB encoding gives back as the same bytes
//! (`docs/colour.md`). Bevy's tonemapping does not run (`Tonemapping::None`).
//!
//! The interface blends the same way (GFX-VIEW-2.6 c, UI-GAME-1.11): its
//! shader (`shaders/ui_stored.wesl`, Bevy's own with one change) writes
//! stored values too, so the pass runs after the interface, not before.
//! The menus' 2D camera gets the same from Bevy's `CompositingSpace::Srgb`
//! (its main texture holds stored values, its 2D shaders write them).

use bevy::camera::CompositingSpace;
use bevy::core_pipeline::blit::{BlitPipeline, BlitPipelineKey};
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::{
    BindGroup, CachedRenderPipelineId, LoadOp, Operations, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, SpecializedRenderPipelines, StoreOp,
    TextureViewId,
};
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::view::ViewTarget;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;

/// A camera whose shaders write stored colour values (the map camera).
#[derive(Component, Clone, Copy, Default, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct StoredColours;

/// The interface's shader, writing stored values (`shaders/ui_stored.wesl`).
pub const STORED_UI_SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("c4a1f3e2-7b5d-4e8a-9f60-2d3b8e71a5c9");

pub struct StoredColoursPlugin;

impl Plugin for StoredColoursPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<StoredColours>::default());
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            let _ = shaders.insert(
                &STORED_UI_SHADER,
                Shader::from_wesl(
                    include_str!("shaders/ui_stored.wesl"),
                    "s2_viewer/ui_stored.wesl",
                ),
            );
        }
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(
                RenderStartup,
                stored_interface.after(bevy::ui_render::init_ui_pipeline),
            )
            .add_systems(
                Render,
                prepare_pipelines
                    .in_set(RenderSystems::Prepare)
                    .ambiguous_with_all(),
            )
            // GFX-VIEW-2.6 c: the interface is blended on the stored
            // values too; only then do they become linear.
            .add_systems(
                Core3d,
                stored_to_linear
                    .after(Core3dSystems::PostProcess)
                    .after(bevy::ui_render::ui_pass)
                    .before(bevy::core_pipeline::upscaling::upscaling),
            );
    }
}

/// GFX-VIEW-2.6 c, UI-GAME-1.11: the interface's pipeline uses the shader
/// that writes stored values.
fn stored_interface(pipeline: Option<ResMut<bevy::ui_render::UiPipeline>>) {
    if let Some(mut pipeline) = pipeline {
        pipeline.shader = STORED_UI_SHADER;
    }
}

#[derive(Component)]
struct StoredPipeline(CachedRenderPipelineId, BlitPipelineKey);

/// The blit that reads its source as sRGB-encoded values and writes them
/// as linear (Bevy's `SRGB_TO_LINEAR` blit), into the main texture's own
/// format.
fn prepare_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit: Res<BlitPipeline>,
    views: Query<(Entity, &ViewTarget, Option<&StoredPipeline>), With<StoredColours>>,
) {
    for (entity, target, cached) in &views {
        let key = BlitPipelineKey {
            target_format: target.main_texture_format(),
            blend_state: None,
            samples: 1,
            source_space: Some(CompositingSpace::Srgb),
        };
        if cached.is_none_or(|StoredPipeline(_, k)| *k != key) {
            let id = pipelines.specialize(&pipeline_cache, &blit, key);
            // No frame without the pass: its colours would be wrong.
            pipeline_cache.block_on_render_pipeline(id);
            commands.entity(entity).insert(StoredPipeline(id, key));
        }
    }
}

#[derive(Default)]
struct BindGroupCache(Option<(TextureViewId, BindGroup)>);

/// Stored values to linear, main texture to main texture.
fn stored_to_linear(
    view: ViewQuery<(&ViewTarget, &StoredPipeline)>,
    pipeline_cache: Res<PipelineCache>,
    blit: Res<BlitPipeline>,
    mut cache: Local<BindGroupCache>,
    mut ctx: RenderContext,
) {
    let (target, StoredPipeline(id, _)) = view.into_inner();
    let Some(pipeline) = pipeline_cache.get_render_pipeline(*id) else {
        return;
    };
    let post = target.post_process_write();
    let bind_group = match &mut cache.0 {
        Some((source, group)) if *source == post.source.id() => group,
        cached => {
            let group = blit.create_bind_group(ctx.render_device(), post.source, &pipeline_cache);
            &cached.insert((post.source.id(), group)).1
        }
    };
    let pass = RenderPassDescriptor {
        label: Some("stored colours to linear"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Default::default()),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };
    let mut render_pass = ctx.command_encoder().begin_render_pass(&pass);
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}
