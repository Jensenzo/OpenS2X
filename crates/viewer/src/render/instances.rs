//! How the meshes' instance data reaches the graphics card: built on the
//! CPU, one buffer a frame, not by Bevy's GPU preprocessing.
//!
//! Bevy's GPU path keeps four buffers and two bind groups per batch set
//! (pipeline, material bind group, mesh slab) per view and phase, and writes
//! or rebuilds them in every frame. With a material of its own for nearly
//! every object that is about 1,750 fresh bind groups, 2,300 buffer writes
//! and 220 new buffers a frame, and at about one in four of Windows' licence
//! polls those calls wait 100..180 ms in all (docs/test-coverage.md, "The
//! long play"). The CPU path writes the instance data as one buffer and
//! keeps the material bind groups; the pictures are the same.
//!
//! Bevy's switch `PbrPlugin::use_gpu_instance_buffer_builder` alone is not
//! enough: the render phases take their mode from the device's
//! [`GpuPreprocessingSupport`], so on a device with full support every mesh
//! is queued for multi-draw bins the CPU batcher never reads, and nothing is
//! drawn. [`CpuInstancesPlugin`] sets that support to none in the render
//! world, right after Bevy measures it at the render start.

use bevy::pbr::PbrPlugin;
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport};
use bevy::render::{init_gpu_resource, RenderApp, RenderStartup};

/// The PBR plugin with the instance buffers built on the CPU; goes with
/// [`CpuInstancesPlugin`].
pub(crate) fn pbr_plugin() -> PbrPlugin {
    PbrPlugin {
        use_gpu_instance_buffer_builder: false,
        ..default()
    }
}

/// Every render phase on the CPU instance path (see the module).
pub(crate) struct CpuInstancesPlugin;

impl Plugin for CpuInstancesPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            RenderStartup,
            no_gpu_preprocessing.after(init_gpu_resource::<GpuPreprocessingSupport>),
        );
    }
}

fn no_gpu_preprocessing(mut support: ResMut<GpuPreprocessingSupport>) {
    support.max_supported_mode = GpuPreprocessingMode::None;
}
