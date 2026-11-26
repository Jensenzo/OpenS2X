//! A development switch, not a hook: the look study
//! (`--look-study <name>[:<a>,<b>,<c>]`, off by default). The whole map
//! picture (ground, water, buildings, settlers, animals, trees, the map's
//! objects and effects, by day and at night) drawn by the real renderer and
//! then turned into one art direction by a pass after the 3D passes and
//! before the interface (`shaders/look_study.wesl`): a way of lighting,
//! colouring and outlining, for choosing a direction for a graphics set of
//! a project's own. The interface is left as it is. No rule of the
//! original's is involved; with the switch off nothing of this runs and
//! the map camera is as ever.
//!
//! The pass reads the depth and normal prepasses of hook W17b (the
//! surfaces' own face normals); `storybook` and `miniature` also switch on
//! that hook's screen-space ambient occlusion (soft contact shadows).

use std::sync::OnceLock;

use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures};
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
    ColorTargetState, ColorWrites, FilterMode, FragmentState, LoadOp, Operations, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
    SpecializedRenderPipelines, StoreOp, TextureFormat, TextureSampleType, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;

/// The looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Painted: soft warm light, rich colour, the textures' detail turned
    /// into broad patches (a Kuwahara filter), soft contact shadows, a
    /// glow on bright things.
    Storybook = 1,
    /// Clean and flat: light in steps, broad colours, a thin dark line
    /// around silhouettes.
    Cel = 2,
    /// An inked illustration: outlines heavier on silhouettes, washed
    /// colour on a warm paper tone, hatching in the shade.
    Ink = 3,
    /// A model table: stronger sun, more colour and contrast, sharp only in
    /// a middle band, a vignette, soft contact shadows.
    Miniature = 4,
    /// Every triangle lit by its own face normal, each surface its mean
    /// colours (with `--ground-study hybrid` for the ground).
    Facets = 5,
    /// Ours: layers of cut paper: flat bands of colour, each layer casting
    /// a soft shadow on what lies behind it, a light cut edge, paper grain.
    Papercut = 6,
    /// Round 2, a look of the materials (`look_materials.rs`): the
    /// original's models with every face one flat colour of its own
    /// texture, glossy toy surfaces, a toy ground, warm soft light; the
    /// pass adds only a little depth of field.
    Origtoy = 7,
    /// Round 2, a look of the materials: flat colour per face, light in
    /// two steps with a cool shadow tone, outlines as shells around each
    /// model, flat water with drawn foam; no pass after the 3D.
    Cel2 = 8,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Storybook,
        Kind::Cel,
        Kind::Ink,
        Kind::Miniature,
        Kind::Facets,
        Kind::Papercut,
        Kind::Origtoy,
        Kind::Cel2,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Storybook => "storybook",
            Kind::Cel => "cel",
            Kind::Ink => "ink",
            Kind::Miniature => "miniature",
            Kind::Facets => "facets",
            Kind::Papercut => "papercut",
            Kind::Origtoy => "origtoy",
            Kind::Cel2 => "cel2",
        }
    }

    /// The look's numbers, their names and defaults (pixels are those of a
    /// picture 1080 lines high, scaled with the picture).
    pub fn defaults(self) -> [(&'static str, f32); 3] {
        match self {
            Kind::Storybook => [("brush", 6.0), ("colour", 0.3), ("glow", 0.3)],
            Kind::Cel => [("line", 1.5), ("steps", 3.0), ("colour", 0.15)],
            Kind::Ink => [("line", 2.0), ("hatch", 5.0), ("wash", 0.55)],
            Kind::Miniature => [("blur", 7.0), ("contrast", 0.18), ("vignette", 0.35)],
            Kind::Facets => [("flatten", 10.0), ("facet", 1.0), ("colour", 0.1)],
            Kind::Papercut => [("shadow", 8.0), ("depth", 0.45), ("grain", 0.2)],
            Kind::Origtoy => [("blur", 3.0), ("colour", 1.35), ("lift", 1.12)],
            Kind::Cel2 => [("line", 3.0), ("colour", 1.5), ("lift", 1.12)],
        }
    }

    /// Whether the look switches on the soft contact shadows (hook W17b's
    /// screen-space ambient occlusion).
    pub fn contact_shadows(self) -> bool {
        matches!(self, Kind::Storybook | Kind::Miniature | Kind::Origtoy)
    }

    /// Whether the look draws a pass after the 3D (all but `cel2`, which
    /// is in the materials alone).
    pub fn has_pass(self) -> bool {
        self != Kind::Cel2
    }
}

/// A look of the materials (round 2): how the models, the ground and the
/// water are coloured and lit (`look_materials.rs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Materials {
    /// `origtoy` (glossy toy) or `cel2` (two steps and outlines).
    pub toy: bool,
    /// The face palettes' colour (saturation) factor and their lift.
    pub colour: f32,
    pub lift: f32,
    /// `cel2`: the outline's width in pixels of a 1080 line picture.
    pub line: f32,
}

/// The look of the materials, when one is on.
pub fn materials() -> Option<Materials> {
    let look = look()?;
    match look.kind {
        Kind::Origtoy => Some(Materials {
            toy: true,
            colour: look.params[1],
            lift: look.params[2],
            line: 0.0,
        }),
        Kind::Cel2 => Some(Materials {
            toy: false,
            colour: look.params[1],
            lift: look.params[2],
            line: look.params[0],
        }),
        _ => None,
    }
}

/// The look as switched on: its kind and its three numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub kind: Kind,
    pub params: [f32; 3],
}

static LOOK: OnceLock<Option<Look>> = OnceLock::new();

/// Switch the look on for this process (the first call wins).
pub fn install(look: Option<Look>) {
    let _ = LOOK.set(look);
}

/// The look, when switched on.
pub fn look() -> Option<Look> {
    LOOK.get().copied().flatten()
}

/// `--look-study <name>[:<a>,<b>,<c>]`: the look and its numbers in order
/// (any left out keep their defaults; `name=value` pairs also).
pub fn from_option(text: &str) -> Result<Look, String> {
    let names = Kind::ALL.map(Kind::name).join("|");
    let (name, rest) = text.split_once(':').unwrap_or((text, ""));
    let kind = Kind::ALL
        .into_iter()
        .find(|k| k.name() == name)
        .ok_or_else(|| format!("--look-study {text}: <{names}>[:<a>,<b>,<c>]"))?;
    let defaults = kind.defaults();
    let mut params = defaults.map(|(_, v)| v);
    for (i, item) in rest.split(',').enumerate().filter(|(_, s)| !s.is_empty()) {
        let (slot, value) = match item.split_once('=') {
            Some((key, value)) => {
                let slot = defaults
                    .iter()
                    .position(|(n, _)| *n == key)
                    .ok_or_else(|| {
                        let keys: Vec<_> = defaults.iter().map(|(n, _)| *n).collect();
                        format!("--look-study {text}: {key}: one of {}", keys.join(", "))
                    })?;
                (slot, value)
            }
            None if i < 3 => (i, item),
            None => return Err(format!("--look-study {text}: at most three numbers")),
        };
        params[slot] = value
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or_else(|| format!("--look-study {text}: {value}: a number of 0 or more"))?;
    }
    Ok(Look { kind, params })
}

/// The map camera's parts for the look: the prepasses the pass reads, single
/// sampled as the pass reads them (the contact shadows for the looks that
/// want them are hook W17b's own parts).
pub fn camera_parts(
    look: Look,
) -> (
    DepthPrepass,
    NormalPrepass,
    bevy::render::view::Msaa,
    LookStudyCamera,
) {
    (
        DepthPrepass,
        NormalPrepass,
        bevy::render::view::Msaa::Off,
        LookStudyCamera {
            kind: look.kind as u32,
            params: look.params,
            sun: Vec3::Y,
        },
    )
}

/// On the map camera while a look is on.
#[derive(Component, Clone, Copy, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct LookStudyCamera {
    kind: u32,
    params: [f32; 3],
    /// Toward the sun (world, Bevy's frame), from the shadow light.
    sun: Vec3,
}

pub const LOOK_STUDY_SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("5b0e7d3a-91c2-4f6e-8a41-c27d0b9e6f15");

pub struct LookStudyPlugin;

impl Plugin for LookStudyPlugin {
    fn build(&self, app: &mut App) {
        if look().is_none() {
            return;
        }
        app.add_plugins(ExtractComponentPlugin::<LookStudyCamera>::default())
            .add_systems(PostUpdate, follow_sun);
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            let _ = shaders.insert(
                &LOOK_STUDY_SHADER,
                Shader::from_wesl(
                    include_str!("shaders/look_study.wesl"),
                    "s2_viewer/look_study.wesl",
                ),
            );
        }
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<LookPipeline>>()
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(
                Render,
                prepare.in_set(RenderSystems::Prepare).ambiguous_with_all(),
            )
            .add_systems(
                Core3d,
                look_pass
                    .after(Core3dSystems::PostProcess)
                    .before(bevy::ui_render::ui_pass),
            );
    }
}

/// The sun's direction for the pass: toward the map's shadow light.
fn follow_sun(
    lights: Query<&GlobalTransform, With<DirectionalLight>>,
    mut cameras: Query<&mut LookStudyCamera>,
) {
    let Some(light) = lights.iter().next() else {
        return;
    };
    let toward = -light.forward().as_vec3();
    for mut camera in &mut cameras {
        camera.sun = toward;
    }
}

#[derive(ShaderType, Clone, Copy, Default)]
struct LookUniform {
    view_from_clip: Mat4,
    world_from_view: Mat4,
    sun: Vec4,
    params: Vec4,
    info: Vec4,
    anchor: Vec4,
}

#[derive(Resource)]
struct LookPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen: FullscreenShader,
}

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
) {
    let entries = BindGroupLayoutEntries::sequential(
        ShaderStages::FRAGMENT,
        (
            texture_2d(TextureSampleType::Float { filterable: true }),
            texture_2d(TextureSampleType::Float { filterable: false }),
            texture_2d(TextureSampleType::Float { filterable: false }),
            sampler(SamplerBindingType::Filtering),
            uniform_buffer::<LookUniform>(false),
        ),
    );
    commands.insert_resource(LookPipeline {
        layout: BindGroupLayoutDescriptor::new("look_study_layout", &entries),
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        fullscreen: fullscreen.clone(),
    });
}

impl SpecializedRenderPipeline for LookPipeline {
    type Key = TextureFormat;

    fn specialize(&self, format: TextureFormat) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("look_study".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: LOOK_STUDY_SHADER,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
struct LookView {
    pipeline: CachedRenderPipelineId,
    uniform: UniformBuffer<LookUniform>,
}

fn prepare(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LookPipeline>>,
    pipeline: Res<LookPipeline>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &ViewTarget, &LookStudyCamera)>,
) {
    for (entity, view, target, look) in &views {
        let id = pipelines.specialize(&pipeline_cache, &pipeline, target.main_texture_format());
        let world_from_view = view.world_from_view.to_matrix();
        let clip_from_world = view.clip_from_view * world_from_view.inverse();
        let size = view.viewport.zw().as_vec2();
        let origin = clip_from_world * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let anchor = if origin.w > 1e-3 {
            let ndc = origin.truncate().truncate() / origin.w;
            Vec4::new(
                (ndc.x * 0.5 + 0.5) * size.x,
                (0.5 - ndc.y * 0.5) * size.y,
                1.0,
                0.0,
            )
        } else {
            Vec4::ZERO
        };
        let mut uniform = UniformBuffer::from(LookUniform {
            view_from_clip: view.clip_from_view.inverse(),
            world_from_view,
            sun: look.sun.extend(0.0),
            params: Vec3::from(look.params).extend(0.0),
            info: Vec4::new(look.kind as f32, size.y / 1080.0, size.x, size.y),
            anchor,
        });
        uniform.write_buffer(&device, &queue);
        commands.entity(entity).insert(LookView {
            pipeline: id,
            uniform,
        });
    }
}

fn look_pass(
    view: ViewQuery<(&ViewTarget, &LookView, &ViewPrepassTextures)>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<LookPipeline>,
    mut ctx: RenderContext,
) {
    let (target, look, prepass) = view.into_inner();
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(look.pipeline) else {
        return;
    };
    let (Some(depth), Some(normal), Some(uniform)) = (
        prepass.depth_only_view(),
        prepass.normal_view(),
        look.uniform.binding(),
    ) else {
        return;
    };
    let post = target.post_process_write();
    let bind_group = ctx.render_device().create_bind_group(
        Some("look_study_bind_group"),
        &pipeline_cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post.source, depth, normal, &pipeline.sampler, uniform)),
    );
    let pass = RenderPassDescriptor {
        label: Some("look study"),
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
    render_pass.set_pipeline(render_pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_read() {
        // spec: none (the look study: a development switch, not a hook; its names and numbers)
        let cel = from_option("cel").unwrap();
        assert_eq!(cel.kind, Kind::Cel);
        assert_eq!(cel.params, [1.5, 3.0, 0.15]);
        let ink = from_option("ink:3,,0.4").unwrap();
        assert_eq!(ink.params, [3.0, 5.0, 0.4]);
        let story = from_option("storybook:glow=0,brush=9").unwrap();
        assert_eq!(story.params, [9.0, 0.3, 0.0]);
        assert!(from_option("oil").is_err());
        assert!(from_option("cel:1,2,3,4").is_err());
        assert!(from_option("cel:line=-1").is_err());
        assert!(from_option("cel:width=2").is_err());
        for kind in Kind::ALL {
            assert_eq!(from_option(kind.name()).unwrap().kind, kind);
        }
    }

    #[test]
    fn shader_names_every_look() {
        // spec: none (the look study: each look has its branch in the shader, by the same number)
        let shader = include_str!("shaders/look_study.wesl");
        for kind in Kind::ALL {
            let constant = format!(
                "const {}: u32 = {}u;",
                kind.name().to_uppercase(),
                kind as u32
            );
            assert!(shader.contains(&constant), "{constant}");
        }
    }
}
