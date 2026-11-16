//! The 3D map view (`specs/GFX-world-view.md`): the
//! terrain with its pattern textures, light, depth haze and fog of war, the
//! water surface (`specs/GFX-water.md`) and the camera. Bevy draws and
//! takes input; the geometry comes from [`crate::render::world3d`] and
//! [`crate::render::terrain3d`], the world from the simulation.

mod build;
mod buildings;
mod camera;
mod materials;
mod mirror;
mod movers;
mod objects;
mod overlays;

pub(crate) use build::*;
pub(crate) use buildings::*;
pub(crate) use camera::*;
pub(crate) use materials::*;
pub(crate) use mirror::*;
pub(crate) use movers::*;
pub(crate) use objects::*;
pub(crate) use overlays::*;

use std::collections::BTreeSet;

use bevy::asset::{uuid_handle, RenderAssetUsages};
use bevy::camera::visibility::RenderLayers;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension, VertexFormat,
};
use bevy::render::view::Tonemapping;
use bevy::shader::{Shader, ShaderRef};

use crate::render::terrain3d;
use crate::render::textures;
use crate::render::world3d::{self, CameraRig, Frame, Heights};

/// The render layer of the map's 3D entities, so that the model view's
/// camera does not draw them.
pub const MAP_LAYER: usize = 1;

pub struct View3dPlugin;

impl Plugin for View3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<TerrainMaterial>::default(),
            MaterialPlugin::<WaterMaterial>::default(),
            MaterialPlugin::<crate::render::models3d::ObjectMaterial>::default(),
        ));
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        let _ = shaders.insert(
            &TERRAIN_SHADER,
            Shader::from_wesl(
                include_str!("../shaders/terrain.wesl"),
                "s2_viewer/terrain.wesl",
            ),
        );
        let _ = shaders.insert(
            &WATER_SHADER,
            Shader::from_wesl(
                include_str!("../shaders/water.wesl"),
                "s2_viewer/water.wesl",
            ),
        );
        let _ = shaders.insert(
            &crate::render::models3d::OBJECT_SHADER,
            Shader::from_wesl(
                include_str!("../shaders/object.wesl"),
                "s2_viewer/object.wesl",
            ),
        );
        let _ = shaders.insert(
            &crate::render::models3d::OBJECT_SHADOW_SHADER,
            Shader::from_wesl(
                include_str!("../shaders/object_shadow.wesl"),
                "s2_viewer/object_shadow.wesl",
            ),
        );
        app.init_resource::<Placements>()
            .init_resource::<Movers>()
            .init_resource::<SeenMarks>()
            .init_resource::<crate::animation::animate3d::Animator>()
            .init_resource::<BuildingAnims>()
            .init_resource::<StatusIcons>()
            .init_resource::<CollapseDust>()
            .init_resource::<ClipParticles>()
            .init_resource::<Wares3d>()
            .init_resource::<Daylight>()
            .init_resource::<DayClock>();
    }
}

/// GFX-VIEW-2.3, 2.3b: the light of the map shown, from the day table it
/// uses, at the environment clock's hour.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct Daylight(pub world3d::DayLight);

/// GFX-VIEW-2.3b, 2.3c: the day table of the map shown and the environment
/// clock in hours (not wrapped; from 12 at the world's first tick, or the
/// `--hour` given for pictures).
#[derive(Resource, Debug, Clone)]
pub struct DayClock {
    pub table: Vec<s2_assets::daytimes::DayEntry>,
    pub hour: f32,
    pub start: f32,
}

impl Default for DayClock {
    fn default() -> Self {
        Self {
            table: Vec::new(),
            hour: world3d::START_HOUR,
            start: world3d::START_HOUR,
        }
    }
}

/// The materials the day light reaches.
type LitMaterials<'w> = (
    ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
    ResMut<'w, Assets<TerrainMaterial>>,
    ResMut<'w, Assets<WaterMaterial>>,
    // GFX-VIEW-6.3: the sky cap's.
    ResMut<'w, Assets<crate::render::models3d::ObjectMaterial>>,
);

/// GFX-VIEW-2.3b: the day table of the map `map_name` of the install at
/// `root`: its own `data/settings/<map name>.bin`, else `daytimes.bin`;
/// empty without either (the light of `daytimes.xml`).
pub fn day_table(
    root: Option<&std::path::Path>,
    map_name: &str,
) -> Vec<s2_assets::daytimes::DayEntry> {
    root.and_then(|root| {
        let dir = crate::extend::asset_folders()
            .iter()
            .try_fold(s2_assets::GameDir::new(root).ok()?, |dir, folder| {
                Some(dir.with_overlay(folder))
            })?;
        s2_assets::daytimes::load(&dir, map_name)
    })
    .map(|(_, entries)| entries)
    .unwrap_or_default()
}

/// GFX-VIEW-2.3b: the day light of the map `map_name` at the start hour
/// ([`world3d::START_HOUR`]); without a table, the light of `daytimes.xml`.
#[cfg(test)]
pub fn day_light(root: Option<&std::path::Path>, map_name: &str) -> world3d::DayLight {
    world3d::DayLight::at(&day_table(root, map_name), world3d::START_HOUR).unwrap_or_default()
}

/// GFX-VIEW-6.3, 6.3a: the sky cap, whose material (the object material's
/// sky family) mixes the skydome textures of the light's two entries
/// (T44). (A material type of its own, a Bevy material plugin more, left
/// the water's mirror camera drawing nothing: a black mirror image.)
#[derive(Component)]
pub struct SkyCap(pub Handle<crate::render::models3d::ObjectMaterial>);

/// GFX-VIEW-6.3, 6.3a: the skydome entries' textures: the clouds (entry 0,
/// turned by 90 degrees) and the stars (entry 1, -45 degrees).
#[derive(Resource, Clone, Debug)]
pub struct SkyTextures {
    pub clouds: Handle<Image>,
    pub stars: Handle<Image>,
}

impl SkyTextures {
    /// A skydome entry's texture and rotation; an entry other than the
    /// stars carries the clouds.
    fn of(&self, skydome: i32) -> (Handle<Image>, f32) {
        if skydome == 1 {
            (self.stars.clone(), terrain3d::STARS_ROTATION)
        } else {
            (self.clouds.clone(), terrain3d::SKY_ROTATION)
        }
    }

    /// GFX-VIEW-6.3 (T44): the cap's two textures for `light` and the
    /// shader's `extra`: T0 the texture of the entry the clock is in, T1
    /// the next entry's when it names another skydome (else T0 again),
    /// their rotations and the share of T1 (the day-time blend).
    pub fn mix(&self, light: &world3d::DayLight) -> (Handle<Image>, Handle<Image>, [f32; 4]) {
        let (first, a0) = self.of(light.skydome);
        let (second, a1) = self.of(light.next_skydome);
        (first, second, [a0, a1, light.sky_mix(), 0.0])
    }
}

/// GFX-VIEW-2.3c: each frame the environment clock advances with the
/// frame's game time while the `DayNightCycle` option is on; when the light
/// of its hour differs from the one drawn, it goes to the clear colour, the
/// sun, the terrain, water and object materials, the sky cap and the moon.
#[allow(clippy::too_many_arguments)]
pub fn advance_day(
    clock: Res<crate::game::GameClock>,
    menu: Res<crate::menus::menu::MenuState>,
    mut view: ResMut<View3d>,
    mut day: ResMut<DayClock>,
    mut daylight: ResMut<Daylight>,
    library: Res<crate::render::models3d::ModelLibrary>,
    mut materials: LitMaterials,
    mut cameras: Query<&mut Camera, With<MapCamera3d>>,
    mut sun: Query<&mut Transform, With<DirectionalLight>>,
    sky: Query<&SkyCap>,
    sky_textures: Option<Res<SkyTextures>>,
    mut sprites: Query<(&SkySprite, &mut Visibility)>,
    mut shadow_map: ResMut<bevy::light::DirectionalLightShadowMap>,
) {
    if !view.active || view.built.is_none() || day.table.is_empty() {
        return;
    }
    day.hour = world3d::advance_clock(day.hour, clock.step, menu.settings.day_night, day.start);
    let Some(light) = world3d::DayLight::at_clock(&day.table, day.hour) else {
        return;
    };
    // SET-4b: the shadows switch is read every frame; the map's size is
    // the game's start's.
    let drawn = view.game_shadows.now(&menu.settings);
    if view.shadows_drawn != Some(drawn) {
        view.shadows_drawn = Some(drawn);
        let size = drawn.unwrap_or(build::SHADOWS_OFF_MAP);
        if shadow_map.size != size {
            shadow_map.size = size;
        }
    } else if !light.differs(&daylight.0) {
        return;
    }
    daylight.0 = light;
    for mut camera in &mut cameras {
        camera.clear_color = ClearColorConfig::Custom(Color::linear_rgb(
            light.background[0],
            light.background[1],
            light.background[2],
        ));
    }
    for mut t in &mut sun {
        *t = Transform::IDENTITY
            .looking_to(Vec3::from(world3d::to_bevy(light.direction())), Vec3::Y);
    }
    let shader = receivers_light(light.shader(), drawn.is_some());
    let (dir, color, ambient) = (
        Vec4::from_array(shader.direction),
        Vec4::from_array(shader.color),
        Vec4::from_array(shader.ambient),
    );
    // The object materials read one light buffer: written in place, it
    // changes no material (F51).
    let (buffers, terrain, water, objects) = &mut materials;
    if let Some(mut buffer) = library.light.as_ref().and_then(|h| buffers.get_mut(h)) {
        *buffer = crate::render::models3d::scene_light(dir, color, ambient);
    }
    for (_, m) in terrain.iter_mut() {
        (m.params.light_dir, m.params.light_color, m.params.ambient) = (dir, color, ambient);
    }
    for (_, m) in water.iter_mut() {
        (m.params.light_dir, m.params.light_color, m.params.ambient) = (dir, color, ambient);
    }
    if let Some(textures) = sky_textures {
        let (first, second, extra) = textures.mix(&light);
        for cap in &sky {
            if let Some(mut m) = objects.get_mut(&cap.0) {
                m.base = first.clone();
                m.second = second.clone();
                m.params.extra = Vec4::from_array(extra);
            }
        }
    }
    for (sprite, mut visibility) in &mut sprites {
        if sprite.moon {
            *visibility = if light.moon_shown() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Marks the 3D map camera.
#[derive(Component)]
pub struct MapCamera3d;

/// Marks the entities of the 3D terrain and water.
#[derive(Component)]
pub struct Terrain3d;

/// GFX-VIEW-6.3: the cloud cap, moved with the camera.
#[derive(Component)]
pub struct Sky3d;

/// GFX-VIEW-6.3: the sun (false) or moon (true) quad.
#[derive(Component)]
pub struct SkySprite {
    pub moon: bool,
}

/// The state of the 3D view.
#[derive(Resource)]
pub struct View3d {
    /// The 3D view is on screen instead of the 2D one.
    pub active: bool,
    pub rig: CameraRig,
    /// `Viewer::loads` of the map the scene was built for.
    built: Option<u64>,
    pub frame: Frame,
    /// Per terrain cell: its mesh and the heights it was built from.
    cells: Vec<(Handle<Mesh>, Vec<i32>)>,
    /// F55: the map's height samples when the cells were last compared.
    height_samples: Vec<i32>,
    world_texture: Option<Handle<Image>>,
    /// The tick the fog of war was last drawn for.
    fog_tick: Option<u64>,
    /// `--zoom`: the zoom step to start each map at.
    start_zoom: u8,
    /// SND-8.1: where sounds are heard from, set with the camera.
    pub ear: Option<[f32; 3]>,
    /// UI-GAME-12.1 (O), GFX-VIEW-3.7: the settler the camera follows
    /// (camera mode 3).
    pub follow: Option<s2_sim::view::SettlerRef>,
    /// GFX-VIEW-3.7: the follow's pose, behind when a game's camera is set
    /// up and otherwise kept: a new follow starts in the last one's pose.
    pub follow_pose: world3d::FollowPose,
    /// GFX-VIEW-3.7: the follow camera's real eye and view direction while
    /// it follows.
    pub follow_eye: Option<world3d::Eye>,
    /// A project's follow (`extend::CameraFollow`): the rig's target x and
    /// z, zoom and turn as the last frame left them.
    pub(crate) placed: Option<[f32; 4]>,
    /// `--camera`: a fixed camera position and view direction in the game
    /// frame (the look comparison, TOOL-trace TRACE-8), in place of the
    /// rig's.
    pub(crate) fixed_eye: Option<([f32; 3], [f32; 3])>,
    /// SET-4b: the shadow values the game's view copied at its start.
    game_shadows: GameShadows,
    /// SET-4b: the shadow map the light was last set for (none: not yet).
    shadows_drawn: Option<Option<usize>>,
}

/// The player's headquarters: the first warehouse of theirs with soldiers.
pub fn headquarters_of(
    world: &s2_sim::World,
    player: s2_sim::PlayerId,
) -> Option<s2_sim::grid::Point> {
    world.warehouses().map(|(at, _)| at).find(|&at| {
        world.building_at(at).is_some_and(|b| b.owner == player) && world.military_at(at).is_some()
    })
}

impl View3d {
    /// GFX-VIEW-3.3: a jump to a field (UI-GAME-11.4: a message's), the
    /// zoom kept.
    pub fn jump_to_field(&mut self, field: s2_sim::grid::Point) {
        let [x, z] = self.frame.field_xz(field.0, field.1);
        self.rig.jump_to(x, z);
    }

    /// GFX-VIEW-3.6 d (H, HOME): back to the start zoom and a jump to the
    /// field (the headquarters) by the rule of 3.3.
    pub fn jump_home(&mut self, field: s2_sim::grid::Point) {
        self.rig.zoom = self.start_zoom;
        let [x, z] = self.frame.field_xz(field.0, field.1);
        self.rig.jump_to(x, z);
    }

    /// GFX-VIEW-3.6 e: a scripted move to a field (the tutorial, CAM-5.3b).
    pub fn move_to_field(&mut self, field: s2_sim::grid::Point) {
        let [x, z] = self.frame.field_xz(field.0, field.1);
        self.rig.move_to(x, z);
    }

    /// GFX-VIEW-3.7: the keys while following: `ahead` (PAGE DOWN, wheel up)
    /// switches behind to ahead; `back` (PAGE UP, wheel down) ahead to
    /// behind, and from behind ends the follow.
    pub fn follow_keys(&mut self, ahead: bool, back: bool) {
        use world3d::FollowPose;
        if ahead {
            self.follow_pose = FollowPose::Ahead;
        } else if back {
            match self.follow_pose {
                FollowPose::Ahead => self.follow_pose = FollowPose::Behind,
                FollowPose::Behind => self.end_follow(),
            }
        }
    }

    /// GFX-VIEW-3.7: the follow ends; the normal camera goes on from the
    /// follow's eye with the base direction north and the ground reset.
    pub fn end_follow(&mut self) {
        self.follow = None;
        if let Some(eye) = self.follow_eye.take() {
            self.rig.resume_from(eye);
        }
    }

    /// Move the camera's target over a field (the minimap, UI-GAME-2.5).
    pub fn focus_field(&mut self, field: (u32, u32)) {
        let [x, z] = self.frame.field_xz(field.0, field.1);
        self.rig.target[0] = x;
        self.rig.target[2] = z;
    }

    /// Where a field's ground is on the screen, if the scene is built.
    pub fn field_on_screen(
        &self,
        world: &s2_sim::World,
        camera: (&Camera, &GlobalTransform),
        field: (u32, u32),
    ) -> Option<Vec2> {
        if !self.active || self.built.is_none() || !world.terrain().contains(field) {
            return None;
        }
        let [x, z] = self.frame.field_xz(field.0, field.1);
        let y = world3d::ground_height(world.terrain(), x, z);
        let at = Vec3::from(world3d::to_bevy([x, y, z]));
        camera.0.world_to_viewport(camera.1, at).ok()
    }

    /// The 3D scene of the load `loads` is built and shown.
    pub fn is_built(&self, loads: u64) -> bool {
        self.active && self.built == Some(loads)
    }

    pub fn new(active: bool, zoom: Option<u8>) -> Self {
        Self {
            start_zoom: zoom.unwrap_or(world3d::START_ZOOM),
            ear: None,
            follow: None,
            follow_pose: world3d::FollowPose::Behind,
            follow_eye: None,
            placed: None,
            active,
            rig: CameraRig::new(0.0, 0.0),
            built: None,
            frame: Frame::new(1, 1),
            cells: Vec::new(),
            height_samples: Vec::new(),
            world_texture: None,
            fog_tick: None,
            fixed_eye: None,
            game_shadows: GameShadows::default(),
            shadows_drawn: None,
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod follow_tests {
    use super::*;

    // spec: GFX-VIEW-3.7 (T37)
    #[test]
    fn page_keys_switch_the_pose_and_end_the_follow_from_behind() {
        use world3d::FollowPose;
        let mut view = View3d::new(true, None);
        view.follow = Some(s2_sim::view::SettlerRef::Settler(7));
        assert_eq!(view.follow_pose, FollowPose::Behind);
        view.follow_keys(true, false);
        assert_eq!(view.follow_pose, FollowPose::Ahead);
        view.follow_keys(true, false);
        assert_eq!(view.follow_pose, FollowPose::Ahead, "already ahead");
        view.follow_keys(false, true);
        assert_eq!(view.follow_pose, FollowPose::Behind);
        assert!(view.follow.is_some());
        view.follow_eye = Some(world3d::Eye {
            position: [1.0, 5.0, 2.0],
            direction: [0.0, 0.0, 1.0],
        });
        view.follow_keys(false, true);
        assert_eq!(view.follow, None, "PAGE UP from behind ends it");
        assert_eq!(view.follow_eye, None);
        // The normal camera goes on from the follow's eye, the pose kept.
        assert_eq!(
            view.rig.last_eye().map(|e| e.position),
            Some([1.0, 5.0, 2.0])
        );
        view.follow = Some(s2_sim::view::SettlerRef::Settler(7));
        view.follow_keys(true, false);
        view.end_follow();
        assert_eq!(
            view.follow_pose,
            FollowPose::Ahead,
            "a new follow starts ahead"
        );
    }
}
