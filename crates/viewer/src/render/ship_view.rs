//! UI-GAME-4.5 (a): the ship window shows its ship through a porthole: the
//! ship's model alone (no water, terrain or other objects), rendered once
//! when the window is built into a 128 x 128 picture, stretched to the
//! 150 x 150 view and cut by the `uiShipMask` alpha. The camera is the
//! building windows' portrait camera: the main camera's field of view and
//! viewing direction, aimed at the centre of the model's bounding box from
//! 1.25 s / tan(f) (s its largest side, f the field of view), near plane
//! 0.1, far plane 200.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureFormat};
use bevy::shader::{Shader, ShaderRef};
use bevy::ui_render::prelude::{UiMaterial, UiMaterialPlugin};

use crate::render::world3d;
use crate::ui::game::{Context, GameUi};

/// The picture's size in pixels (drawn 150 x 150 through the mask).
pub const PIXELS: u32 = 128;
/// The portrait camera's planes.
pub const NEAR: f32 = 0.1;
pub const FAR: f32 = 200.0;
/// The render layer of the portrait's model, which only the ship camera
/// sees.
pub const SHIP_VIEW_LAYER: usize = 3;

pub const SHIP_VIEW_SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("5b0e7d2a-91c4-4f3e-8a26-d4f1b9c03e57");

/// The picture through the mask.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ShipViewMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub view: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub mask: Handle<Image>,
}

impl UiMaterial for ShipViewMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHIP_VIEW_SHADER)
    }
}

/// The ship camera's picture, and the material that shows it once the
/// mask is loaded; the ship the picture shows and its model's entity.
#[derive(Resource, Default)]
pub struct ShipView {
    pub image: Handle<Image>,
    pub material: Option<Handle<ShipViewMaterial>>,
    shown: Option<usize>,
    model: Option<Entity>,
}

#[derive(Component)]
pub struct ShipCamera;

pub struct ShipViewPlugin;

impl Plugin for ShipViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<ShipViewMaterial>::default())
            .init_resource::<ShipView>();
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        let _ = shaders.insert(
            &SHIP_VIEW_SHADER,
            Shader::from_wesl(
                include_str!("shaders/ship_view.wesl"),
                "s2_viewer/ship_view.wesl",
            ),
        );
    }
}

/// The picture and the camera that draws into it, inactive until a ship
/// window opens. It sees only the portrait's layer, with the main camera's
/// colour handling (GFX-VIEW-2.6 c1).
pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut view: ResMut<ShipView>,
) {
    let image = images.add(Image::new_target_texture(
        PIXELS,
        PIXELS,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    view.image = image.clone();
    commands.spawn((
        ShipCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: -1,
            // GFX-VIEW-2.3: the background of daytimes.xml (the ship's
            // view is not lit by the map's day table).
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
            near: NEAR,
            far: FAR,
            ..default()
        }),
        Tonemapping::None,
        bevy::render::view::DebandDither::Disabled,
        RenderLayers::layer(SHIP_VIEW_LAYER),
        Transform::IDENTITY,
    ));
}

/// The material, once the mask (`uiShipMask.dds`) is loaded; the
/// interface draws it (`UiAssets::ship_view`).
pub fn make_material(
    mut assets: ResMut<crate::ui::render::UiAssets>,
    mut view: ResMut<ShipView>,
    mut materials: ResMut<Assets<ShipViewMaterial>>,
) {
    if view.material.is_some() {
        return;
    }
    let Some(Some(mask)) = assets
        .atlases
        .get(&crate::ui::widgets::Atlas::ShipMask)
        .cloned()
    else {
        return;
    };
    let material = materials.add(ShipViewMaterial {
        view: view.image.clone(),
        mask,
    });
    assets.ship_view = Some(material.clone());
    view.material = Some(material);
}

/// The main 3D camera's place and projection.
type MainCamera<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static Projection),
    (
        With<crate::render::view3d::MapCamera3d>,
        Without<ShipCamera>,
    ),
>;

/// UI-GAME-4.5 (a), 4.3: the portrait camera's distance from the centre
/// of a model's bounding box of largest side `side` for a field of view
/// `fov` (radians): 1.25 side / tan(fov).
pub fn distance(side: f32, fov: f32) -> f32 {
    1.25 * side / fov.tan().max(1e-6)
}

/// When a ship window opens (or shows another ship) the ship's model is
/// placed alone on the portrait layer, at the ship's place in its rest
/// pose, heading 0 (the pose is not given, Q-256 g), and the camera is set
/// along the main camera's view of that
/// moment. Neither moves until the window shows another ship, so the
/// picture is the one taken when the window was built; the camera stays on
/// while the window is open only because a new view's pipelines may take a
/// few frames to be ready.
#[allow(clippy::too_many_arguments)]
pub fn aim(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    ui: Res<GameUi>,
    view3d: Res<crate::render::view3d::View3d>,
    mut view: ResMut<ShipView>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::render::models3d::ObjectMaterial>>,
    main: MainCamera,
    camera: Single<(&mut Camera, &mut Transform, &mut Projection), With<ShipCamera>>,
) {
    let (mut camera, mut transform, mut projection) = camera.into_inner();
    let wanted = match (&ui.context, &viewer.world) {
        (Some(Context::Ship(i)), Some(world)) if view3d.active && *i < world.ships().len() => {
            Some(*i)
        }
        _ => None,
    };
    if wanted == view.shown {
        return;
    }
    if let Some(old) = view.model.take() {
        commands.entity(old).despawn();
    }
    camera.is_active = false;
    // Until the ship's model can be placed (the model tables and the main
    // camera may come a few frames after the window), it is tried again.
    let (Some(i), Some(world), Ok((main_transform, main_projection))) =
        (wanted, &viewer.world, main.single())
    else {
        view.shown = None;
        return;
    };
    let ship = &world.ships()[i];
    let tribe = crate::render::models3d::TRIBE_NAMES[usize::from(world.tribe(ship.owner)).min(2)];
    let Some(entry) = library
        .tables
        .ships
        .iter()
        .find(|s| s.tribe == tribe)
        .cloned()
    else {
        return;
    };
    let Some(model) = library.model(&entry.mesh, entry.scale, &mut meshes, &mut images) else {
        return;
    };
    let mut look = crate::render::models3d::Look::family(
        crate::render::models3d::Family::Ship,
        crate::render::models3d::player_colour(ship.owner),
    );
    // GFX-OBJ-2.8a: each part's weight is its part number (`Look::part_extra`).
    look.extra[0] = 1.0;
    let parts: Vec<_> = (0..model.parts.len())
        .filter_map(|k| {
            let m = library.rest_material(&model, k, &look, &mut images, &mut materials)?;
            Some((model.parts[k].mesh.clone(), m))
        })
        .collect();
    let at = ship.position();
    let [x, z] = view3d.frame.field_xz(at.0, at.1);
    let place = Vec3::from(world3d::to_bevy([x, world3d::WATER_LEVEL, z]));
    let entity = commands
        .spawn((Transform::from_translation(place), Visibility::default()))
        .with_children(|c| {
            for (mesh, material) in parts {
                c.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    RenderLayers::layer(SHIP_VIEW_LAYER),
                ));
            }
        })
        .id();
    view.model = Some(entity);
    view.shown = wanted;
    // The box's centre and largest side (game frame), the main camera's
    // view direction and field of view.
    let b = model.bounds;
    let centre = [
        (b[0][0] + b[1][0]) / 2.0,
        (b[0][1] + b[1][1]) / 2.0,
        (b[0][2] + b[1][2]) / 2.0,
    ];
    let side = (0..3).map(|k| b[1][k] - b[0][k]).fold(0.0f32, f32::max);
    // "The field-of-view angle as the camera stores it": read as the full
    // vertical angle, not half of it (Q-256 g).
    let fov = match main_projection {
        Projection::Perspective(p) => p.fov,
        _ => std::f32::consts::FRAC_PI_4,
    };
    let target = place + Vec3::from(world3d::to_bevy(centre));
    let forward = main_transform.forward().as_vec3();
    let eye = target - forward * distance(side, fov);
    *transform = Transform::from_translation(eye).looking_to(forward, Vec3::Y);
    if let Projection::Perspective(p) = projection.as_mut() {
        p.fov = fov;
    }
    camera.is_active = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UI-GAME-4.5 (a): the portrait camera stands 1.25 s / tan(f) from
    /// the box's centre: a 4-unit ship at a 45 degree field of view from 5.
    // spec: UI-GAME-4.5
    #[test]
    fn portrait_distance() {
        assert!((distance(4.0, std::f32::consts::FRAC_PI_4) - 5.0).abs() < 1e-5);
        assert!((distance(2.0, 30f32.to_radians()) - 2.5 / 30f32.to_radians().tan()).abs() < 1e-5);
    }
}
