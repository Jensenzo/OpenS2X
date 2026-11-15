//! GFX-WATER-2.6: the mirror image the water surface reflects.
//!
//! While the mirror switch is on (the 13th scene field) and the mirror
//! quality makes a texture, a second camera draws the scene into the
//! mirror texture before the map camera draws the frame (GFX-WATER-2.6a):
//! the world reflected in the water plane, then seen with the main camera's
//! view (its transform carries the reflection, a scale of -1 in y), with
//! 1.03 times the main field of view, the culling reversed with the
//! reflection and the clip plane at the water level (the terrain and object
//! shaders drop what lies below it in this view). What it draws follows the
//! main camera's pitch and `MIRROR DETAILS` (2.6b): the sky alone (the
//! cloud cap over the frame's background, 2.6c) from every default camera;
//! never the water surfaces or the caustics. The water's `RenderWater` then
//! mixes it in at the surface point's own screen place (GFX-WATER-4.3).

use super::*;
use bevy::camera::{CameraProjection, RenderTarget, SubCameraView};
use bevy::math::Vec3A;

/// GFX-WATER-2.6b: the render layers of what the mirror camera can draw,
/// each beside [`MAP_LAYER`]: the land, every object family (settlers,
/// wares, roads and the map's objects), the sky (the cloud cap), and the
/// settlers with what they carry (the scene's draw bit 8, also in the
/// object layer).
pub const MIRROR_LAYER: usize = 4;
pub const MIRROR_OBJECT_LAYER: usize = 5;
pub const MIRROR_SKY_LAYER: usize = 6;
pub const MIRROR_SETTLER_LAYER: usize = 7;

/// The layers of the land, which the mirror image shows in flat views.
pub fn mirrored_layers() -> RenderLayers {
    RenderLayers::from_layers(&[MAP_LAYER, MIRROR_LAYER])
}

/// The layers of an object, which the mirror image shows in flat views
/// with every detail.
pub fn mirrored_object_layers() -> RenderLayers {
    RenderLayers::from_layers(&[MAP_LAYER, MIRROR_OBJECT_LAYER])
}

/// The layers of a settler and what it carries, which the mirror image
/// shows in flat views from MIRROR DETAILS n = 3 on (GFX-WATER-2.6b).
pub fn mirrored_settler_layers() -> RenderLayers {
    RenderLayers::from_layers(&[MAP_LAYER, MIRROR_OBJECT_LAYER, MIRROR_SETTLER_LAYER])
}

/// The layers of the sky's cloud cap, which the mirror image always shows.
pub fn mirrored_sky_layers() -> RenderLayers {
    RenderLayers::from_layers(&[MAP_LAYER, MIRROR_SKY_LAYER])
}

/// GFX-WATER-2.6b: the mirror camera's layers for its content.
pub fn mirror_camera_layers(content: world3d::MirrorContent) -> RenderLayers {
    let mut layers = RenderLayers::layer(MIRROR_SKY_LAYER);
    if content.land {
        layers = layers.with(MIRROR_LAYER);
    }
    if content.settlers {
        layers = layers.with(MIRROR_SETTLER_LAYER);
    }
    if content.objects {
        layers = layers.with(MIRROR_OBJECT_LAYER);
    }
    layers
}

/// GFX-WATER-2.6a 1: the mirror camera's transform: the main camera's
/// reflected in the water plane, so a world point p is drawn where the main
/// camera sees p reflected (y -> 2 level - y). The reflection is a scale of
/// -1 along the view's y with the rotation conjugated by it.
pub fn mirror_transform(main: &Transform) -> Transform {
    let q = main.rotation;
    Transform {
        translation: Vec3::from(world3d::reflect_in_water(main.translation.to_array())),
        rotation: Quat::from_xyzw(-q.x, q.y, -q.z, q.w),
        scale: main.scale * Vec3::new(1.0, -1.0, 1.0),
    }
}

/// Marks the mirror camera.
#[derive(Component)]
pub struct MirrorCamera3d;

/// The mirror texture (made again, with a new handle, when its size
/// changes: the camera and the water then both take it up), whether it is
/// in use, and a 1 x 1 stand-in the water binds while the mirror is off.
#[derive(Resource)]
pub struct Mirror {
    pub(crate) image: Handle<Image>,
    size: Option<(u32, u32)>,
    pub(crate) none: Handle<Image>,
    /// `S2_MIRROR_SHOW`: the water shows the mirror image alone (a check of
    /// what the mirror camera draws).
    show_alone: bool,
}

impl Mirror {
    /// The texture the water binds and whether it is the mirror image.
    pub(crate) fn texture(&self) -> (Handle<Image>, bool) {
        match self.size {
            Some(_) => (self.image.clone(), true),
            None => (self.none.clone(), false),
        }
    }
}

/// The mirror camera's projection: the map camera's field of view and
/// aspect (the surface point's place on the screen is where the water
/// reads the texture), whatever the texture's own shape.
#[derive(Debug, Clone)]
pub struct MirrorProjection(pub PerspectiveProjection);

impl CameraProjection for MirrorProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        self.0.get_clip_from_view()
    }
    fn get_clip_from_view_for_sub(&self, sub_view: &SubCameraView) -> Mat4 {
        self.0.get_clip_from_view_for_sub(sub_view)
    }
    /// The texture's size does not set the aspect: [`place_mirror`] does.
    fn update(&mut self, _width: f32, _height: f32) {}
    fn far(&self) -> f32 {
        self.0.far
    }
    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        self.0.get_frustum_corners(z_near, z_far)
    }
}

/// GFX-WATER-2.6, 2.6c (f) 6: the mirror texture, `w` x `h`: the water
/// reads it bilinearly with wrap addressing, so a lookup the noise pushes
/// past an edge reads the opposite edge.
pub(crate) fn mirror_target(w: u32, h: u32) -> Image {
    let mut image = Image::new_target_texture(w, h, TextureFormat::Rgba8UnormSrgb, None);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// The mirror camera, inactive until the mirror is on.
pub fn setup_mirror(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let none = images.add(Image::new_fill(
        Extent3d::default(),
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let image = images.add(mirror_target(1, 1));
    commands.insert_resource(Mirror {
        image: image.clone(),
        size: None,
        none,
        show_alone: std::env::var_os("S2_MIRROR_SHOW").is_some(),
    });
    commands.spawn((
        MirrorCamera3d,
        RenderTarget::from(image),
        Camera3d::default(),
        Camera {
            is_active: false,
            // Before the map camera (2). (An `Rgba8Unorm` target without the
            // stored-colour pass, or without multisampling, drew nothing.)
            order: -2,
            clear_color: ClearColorConfig::Custom(Color::linear_rgb(
                world3d::BACKGROUND[0],
                world3d::BACKGROUND[1],
                world3d::BACKGROUND[2],
            )),
            ..default()
        },
        // The map camera's colour handling (GFX-VIEW-2.6 c1): its 8-bit sRGB
        // texture holds the stored values' bytes, which the water reads back.
        bevy::camera::Hdr,
        crate::render::stored::StoredColours,
        Projection::custom(MirrorProjection(PerspectiveProjection {
            fov: world3d::vertical_fov(4.0 / 3.0),
            aspect_ratio: 4.0 / 3.0,
            near: world3d::NEAR,
            far: world3d::FAR,
            ..default()
        })),
        Tonemapping::None,
        bevy::render::view::DebandDither::Disabled,
        RenderLayers::layer(MIRROR_SKY_LAYER),
        Transform::IDENTITY,
    ));
}

/// GFX-WATER-2.6: each frame, after the map camera is placed, the mirror
/// camera takes the main camera's view reflected in the water plane (2.6a),
/// 1.03 times its field of view and the content of 2.6b; the texture is
/// made again when its size changes (the quality, the switch or the back
/// buffer's width), and the water materials bind it.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn place_mirror(
    view: Res<View3d>,
    menu: Res<crate::menus::menu::MenuState>,
    window: Single<&Window>,
    main: Single<(&Camera, &Transform, &Projection), (With<MapCamera3d>, Without<MirrorCamera3d>)>,
    camera: Single<
        (
            &mut Camera,
            &mut Transform,
            &mut Projection,
            &mut RenderTarget,
            &mut RenderLayers,
        ),
        (With<MirrorCamera3d>, Without<MapCamera3d>),
    >,
    mut mirror: ResMut<Mirror>,
    mut images: ResMut<Assets<Image>>,
    mut water: ResMut<Assets<WaterMaterial>>,
) {
    let (main_camera, main_transform, main_projection) = main.into_inner();
    let (mut camera, mut transform, mut projection, mut target, mut layers) = camera.into_inner();
    let shown = view.active && view.built.is_some() && main_camera.is_active;
    let size = shown
        .then(|| menu.settings.mirror_size(window.physical_width()))
        .flatten();
    if size != mirror.size {
        mirror.size = size;
        if let Some((w, h)) = size {
            let old = std::mem::replace(&mut mirror.image, images.add(mirror_target(w, h)));
            images.remove(&old);
            *target = RenderTarget::from(mirror.image.clone());
        }
    }
    let (texture, on) = mirror.texture();
    let flag = match (on, mirror.show_alone) {
        (false, _) => 0.0,
        (true, false) => 1.0,
        (true, true) => 2.0,
    };
    let stale: Vec<_> = water
        .iter()
        .filter(|(_, m)| m.mirror != texture || m.params.mirror.x != flag)
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut m) = water.get_mut(id) {
            m.mirror = texture.clone();
            m.params.mirror.x = flag;
        }
    }
    if camera.is_active != on {
        camera.is_active = on;
    }

    if !on {
        return;
    }
    // Read every frame by the render world; no change to signal.
    camera.bypass_change_detection().clear_color = main_camera.clear_color;
    // GFX-WATER-2.6a 1: the main view of the reflected world.
    *transform = mirror_transform(main_transform);
    // GFX-WATER-2.6b: the content by the main camera's pitch and the
    // details.
    let content = world3d::mirror_content(
        main_transform.forward().y,
        menu.settings.cfg.scene.mirror_details(),
    );
    let wanted = mirror_camera_layers(content);
    if *layers != wanted {
        *layers = wanted;
    }
    let aspect = window.width() / window.height().max(1.0);
    let Projection::Perspective(p) = main_projection else {
        return;
    };
    // GFX-WATER-2.6a 2: 1.03 x the main field of view, the rest the main
    // camera's.
    let fov = world3d::mirror_vertical_fov(p.fov, aspect);
    let current = match projection.as_ref() {
        Projection::Custom(custom) => custom
            .get::<MirrorProjection>()
            .map(|m| (m.0.fov, m.0.aspect_ratio)),
        _ => None,
    };
    if current != Some((fov, aspect)) {
        *projection = Projection::custom(MirrorProjection(PerspectiveProjection {
            fov,
            aspect_ratio: aspect,
            near: p.near,
            far: p.far,
            ..default()
        }));
    }
}
