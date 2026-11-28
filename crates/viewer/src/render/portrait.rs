//! UI-GAME-4.3a (T64, Q-1091): the building windows' portrait. The
//! production, site and military windows show a 128 x 128 picture of the
//! **world scene** (not the model alone) through the portrait camera of
//! UI-GAME-4.5 a: the main camera's viewing direction and field of view,
//! aimed at the centre of the building's bounding box from 1.25 s / tan(f)
//! (s its largest side, f the field of view), near plane 0.1, far plane
//! 200. It is drawn stretched to 80 x 80 at (26, 47), with no mask. The
//! picture is made once, when the window is built: settlers in it stand
//! still and a site does not grow while the window stays open. One picture
//! serves every building window, so a second window shows its building in
//! the first's portrait too.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::render::ship_view;
use crate::render::world3d;
use crate::ui::game::{Context, GameUi};

/// The picture's size in pixels (drawn 80 x 80).
pub const PIXELS: u32 = ship_view::PIXELS;
/// The frames the camera draws after it is aimed before it stops, so the
/// picture holds that moment's scene: the scene's pipelines for a new view
/// may need a few frames to be ready (the original draws it once).
pub const WARM_FRAMES: u32 = 20;
/// The largest side taken for a building whose model's box is not known.
const FALLBACK_SIDE: f32 = 3.0;

/// The portrait picture, the building it shows and the frames left to draw.
#[derive(Resource, Default)]
pub struct BuildingPortrait {
    pub image: Handle<Image>,
    shown: Option<s2_sim::grid::Point>,
    frames_left: u32,
}

#[derive(Component)]
pub struct PortraitCamera;

/// The picture and the camera that draws into it, inactive until a
/// building window opens. It sees the map's layer with the main camera's
/// colour handling (GFX-VIEW-2.6 c1); beyond its far plane the background
/// shows (the pale sky of the original's barracks picture).
pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut portrait: ResMut<BuildingPortrait>,
    mut assets: ResMut<crate::ui::render::UiAssets>,
) {
    let image = images.add(Image::new_target_texture(
        PIXELS,
        PIXELS,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    portrait.image = image.clone();
    assets.building_view = Some(image.clone());
    let render = crate::extend::render();
    commands.spawn((
        PortraitCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::linear_rgb(
                world3d::BACKGROUND[0],
                world3d::BACKGROUND[1],
                world3d::BACKGROUND[2],
            )),
            ..default()
        },
        RenderTarget::from(image),
        bevy::camera::Hdr,
        crate::render::stored::StoredColours,
        Projection::Perspective(PerspectiveProjection {
            aspect_ratio: 1.0,
            near: ship_view::NEAR,
            far: ship_view::FAR,
            ..default()
        }),
        Tonemapping::None,
        bevy::render::view::DebandDither::Disabled,
        render.shadow_filter.method(),
        RenderLayers::layer(crate::render::view3d::MAP_LAYER),
        Transform::IDENTITY,
    ));
}

/// The main 3D camera's place and projection.
type MainCamera<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static Projection),
    (
        With<crate::render::view3d::MapCamera3d>,
        Without<PortraitCamera>,
    ),
>;

/// The building a window shows the portrait of: the production, site and
/// military windows (UI-GAME-4.1, 4.2, 4.3); not the warehouses' (4.4).
fn portrait_building(ui: &GameUi, world: &s2_sim::World) -> Option<s2_sim::grid::Point> {
    let Some(Context::Building { at, .. }) = ui.context else {
        return None;
    };
    let b = world.building_at(at)?;
    let warehouse = world
        .building_type(b.building_type)
        .is_some_and(|k| k.warehouse);
    (!warehouse || b.under_construction).then_some(at)
}

/// When a building window opens (or shows another building) the camera is
/// aimed along the main camera's view of that moment at the building's
/// box and draws [`WARM_FRAMES`] frames; then it stops, and the picture
/// keeps that view. Closing the window keeps the picture (it is shared).
#[allow(clippy::too_many_arguments)]
pub fn aim(
    viewer: Res<crate::game::Viewer>,
    ui: Res<GameUi>,
    view3d: Res<crate::render::view3d::View3d>,
    placements: Res<crate::render::view3d::Placements>,
    mut portrait: ResMut<BuildingPortrait>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    placed: Query<
        &Transform,
        (
            Without<PortraitCamera>,
            Without<crate::render::view3d::MapCamera3d>,
        ),
    >,
    main: MainCamera,
    camera: Single<(&mut Camera, &mut Transform, &mut Projection), With<PortraitCamera>>,
) {
    let (mut camera, mut transform, mut projection) = camera.into_inner();
    let wanted = match &viewer.world {
        Some(world) if view3d.active => portrait_building(&ui, world),
        _ => None,
    };
    if wanted == portrait.shown {
        if portrait.frames_left > 0 {
            portrait.frames_left -= 1;
        } else {
            camera.is_active = false;
        }
        return;
    }
    let (Some(at), Some(world), Ok((main_transform, main_projection))) =
        (wanted, &viewer.world, main.single())
    else {
        portrait.shown = None;
        camera.is_active = false;
        return;
    };
    // GFX-VIEW-4: the building's model as placed on the map (its site too).
    let Some(place) = placements
        .placed
        .iter()
        .find(|((kind, field, _), _)| {
            *kind == crate::world::scene3d::Kind::Building && *field == at
        })
        .and_then(|(_, (entity, _))| placed.get(*entity).ok())
        .map(|t| t.translation)
    else {
        // UI-GAME-4.3a (d): no model, no picture (tried again next frame
        // while the scene is still being placed).
        return;
    };
    let b = world.building_at(at).expect("checked");
    // The box of the type's model: its centre and largest side (game frame).
    let bounds = placements
        .lookup
        .buildings
        .get(&b.building_type)
        .map(|gfx| crate::render::models3d::building_entry(world.tribe(b.owner), gfx))
        .and_then(|name| library.tables.building(&name).cloned())
        .and_then(|entry| entry.mesh)
        .and_then(|mesh| {
            library.model(
                &mesh,
                crate::render::models3d::DEFAULT_SCALE,
                &mut meshes,
                &mut images,
            )
        })
        .map(|m| m.bounds);
    let (centre, side) = match bounds {
        Some(b) => (
            [
                (b[0][0] + b[1][0]) / 2.0,
                (b[0][1] + b[1][1]) / 2.0,
                (b[0][2] + b[1][2]) / 2.0,
            ],
            (0..3).map(|k| b[1][k] - b[0][k]).fold(0.0f32, f32::max),
        ),
        None => ([0.0, FALLBACK_SIDE / 2.0, 0.0], FALLBACK_SIDE),
    };
    let fov = match main_projection {
        Projection::Perspective(p) => p.fov,
        _ => std::f32::consts::FRAC_PI_4,
    };
    let target = place + Vec3::from(world3d::to_bevy(centre));
    let forward = main_transform.forward().as_vec3();
    let eye = target - forward * ship_view::distance(side, fov);
    *transform = Transform::from_translation(eye).looking_to(forward, Vec3::Y);
    if let Projection::Perspective(p) = projection.as_mut() {
        p.fov = fov;
    }
    camera.is_active = true;
    portrait.shown = wanted;
    portrait.frames_left = WARM_FRAMES;
}
