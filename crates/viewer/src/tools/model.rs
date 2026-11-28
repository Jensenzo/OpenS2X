//! Model preview (key M, `--model <name>`): one Granny model
//! (`specs/GFX-granny-format.md`) in 3D, textured, static, with an orbit
//! camera. No animation or skinning: meshes are drawn as stored.
//!
//! Display choices, not game rules: Max's Z-up axes are turned to the
//! view's Y-up (the game's own basis conversion is Q-047). Normals are scaled
//! to unit length and texture V is used as stored (GFX-GR2-7.3, 7.4).
//!
//! `--model-shots` takes one picture of every model the way the game would
//! frame it: the basis conversion of GFX-VIEW-1.2, buildings turned by
//! their heading (GFX-VIEW-4.3), and the start camera of GFX-VIEW-3.6 at a
//! zoom step, on flat dry ground at the model's origin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use s2_assets::gr2::{self, Gr2};

/// The models offered first.
pub const FIRST_MODELS: [&str; 2] = ["PB_R_Sawmill01", "RS_TreeBirch01"];

/// Which view is on screen.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewMode {
    #[default]
    Map,
    Model,
}

/// The model list and what is shown.
#[derive(Resource)]
pub struct ModelView {
    /// Every `.gr2` of the install, sorted by name.
    pub files: Vec<PathBuf>,
    pub current: usize,
    pub reload: bool,
    /// Lower-case base name of every texture file of the install, to the
    /// file a model takes when its own folder has none of that name
    /// (GFX-GR2-5.1, [`texture_index`]).
    pub textures: HashMap<String, PathBuf>,
    /// The names more than one folder of the install has a texture of, to
    /// every such file (a model takes the one in its own folder first).
    pub texture_copies: HashMap<String, Vec<PathBuf>>,
    /// What the info panel says about the model.
    pub status: Vec<String>,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    /// `--model-shots`: the pictures still to take.
    pub shots: Option<ModelShots>,
}

/// `--model-shots`: where the pictures go, the camera's zoom step and how
/// far the current one is.
#[derive(Debug)]
pub struct ModelShots {
    pub dir: PathBuf,
    pub zoom: u8,
    /// Frames the current model has been on screen.
    frames: u32,
    /// Pictures asked for and pictures saved (set by the capture).
    asked: usize,
    saved: Arc<AtomicUsize>,
}

impl ModelShots {
    pub fn new(dir: PathBuf, zoom: u8) -> Self {
        Self {
            dir,
            zoom,
            frames: 0,
            asked: 0,
            saved: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// Frames a model is on screen before its picture: the window shows
/// nothing in its first frames, so the first model waits longer.
const SHOT_FIRST_FRAMES: u32 = 30;
const SHOT_FRAMES: u32 = 3;

/// The background of the pictures, one flat colour (cut away by tools
/// that crop to the model).
pub const SHOT_BACKGROUND: Color = Color::srgb(0.16, 0.18, 0.21);

/// `--model-list`: the `.gr2` files named in `list`, one per line, relative
/// to `game_dir` unless absolute; empty lines and `#` comments skipped.
pub fn read_model_list(list: &Path, game_dir: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let text = std::fs::read_to_string(list).map_err(|e| e.to_string())?;
    let files: Vec<PathBuf> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let path = PathBuf::from(l);
            match game_dir {
                Some(root) if path.is_relative() => root.join(path),
                _ => path,
            }
        })
        .collect();
    if files.is_empty() {
        return Err("no model in the list".into());
    }
    Ok(files)
}

/// The game's ground under a picture: flat, 20 units above the water, so
/// the camera's floor (GFX-VIEW-3.3) is the dry-ground one.
struct FlatGround;

const SHOT_GROUND_RAW: i32 = 20_000;

impl crate::render::world3d::Heights for FlatGround {
    fn frame(&self) -> crate::render::world3d::Frame {
        crate::render::world3d::Frame::new(64, 64)
    }

    fn raw(&self, _c: u32, _r: u32) -> i32 {
        SHOT_GROUND_RAW
    }
}

/// GFX-VIEW-3.6: the start camera jumped to a field at the origin on flat
/// dry ground, at zoom step `zoom`, once it has settled (the z easing and
/// the base direction converge within a few frames). In the game frame,
/// relative to the field's point.
pub fn game_eye(zoom: u8) -> crate::render::world3d::Eye {
    let mut rig = crate::render::world3d::CameraRig::start(Some([0.0, 0.0]));
    rig.zoom = zoom.min(crate::render::world3d::MAX_ZOOM);
    let mut eye = rig.update(&FlatGround);
    for _ in 0..60 {
        eye = rig.update(&FlatGround);
    }
    eye.position[1] -= SHOT_GROUND_RAW as f32 * crate::render::world3d::HEIGHT_SCALE;
    eye
}

/// GFX-GR2-5.1: a model names its textures by file name, found by base
/// name in the game data. Five names are in two folders each, with other
/// pictures (`EselTextue.dds` in `animals` and `character/donkey`, and
/// `l_ground_00`, `l_rock_00_n`, `m_rock_01`, `z_rock_00` in `map/objects`
/// and `map/textures`). Which one a model gets was decided by the order the
/// file system lists the folders, so the same files drew a rock in another
/// colour from an NTFS copy than from the Linux one. A model takes the copy
/// in its own folder and no other (`ModelLibrary::texture_path`); the index
/// keeps the first by path for what is not a model's texture. Returns the
/// index and the names with copies.
pub fn texture_index(
    mut found: Vec<PathBuf>,
) -> (HashMap<String, PathBuf>, HashMap<String, Vec<PathBuf>>) {
    found.sort_by_key(|p| p.to_string_lossy().replace('\\', "/").to_ascii_lowercase());
    let mut textures = HashMap::new();
    let mut copies: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in found {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        copies.entry(name.clone()).or_default().push(path.clone());
        textures.entry(name).or_insert(path);
    }
    copies.retain(|_, paths| paths.len() > 1);
    (textures, copies)
}

impl ModelView {
    /// The `.gr2` and `.dds` files below `data`; `start` picks the first
    /// model by file stem or path (else the first of [`FIRST_MODELS`]).
    pub fn new(game_dir: Option<&Path>, start: Option<&str>) -> Self {
        let mut files = Vec::new();
        let mut found_textures = Vec::new();
        if let Some(root) = game_dir {
            let mut pending = vec![root.join("data")];
            while let Some(dir) = pending.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for path in entries.flatten().map(|e| e.path()) {
                    if path.is_dir() {
                        pending.push(path);
                        continue;
                    }
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_lowercase());
                    match ext.as_deref() {
                        Some("gr2") => files.push(path),
                        Some("dds") => found_textures.push(path),
                        _ => {}
                    }
                }
            }
        }
        let (mut textures, mut texture_copies) = texture_index(found_textures);
        // A project's asset folders: their textures take the place of the
        // install's of the same name (docs/extending.md).
        for folder in crate::extend::asset_folders() {
            let mut pending = vec![folder.clone()];
            while let Some(dir) = pending.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for path in entries.flatten().map(|e| e.path()) {
                    if path.is_dir() {
                        pending.push(path);
                    } else if path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("dds"))
                    {
                        let name = path.file_name().expect("a file").to_string_lossy();
                        texture_copies.remove(&name.to_ascii_lowercase());
                        textures.insert(name.to_ascii_lowercase(), path);
                    }
                }
            }
        }
        // By name, then by path: the order of the folders as the file
        // system lists them differs between file systems.
        files.sort_by_key(|p| {
            (
                p.file_name().map(|n| n.to_ascii_lowercase()),
                p.to_string_lossy().to_ascii_lowercase(),
            )
        });
        let stem = |p: &PathBuf| {
            p.file_stem()
                .map(|s| s.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default()
        };
        let wanted = start.unwrap_or(FIRST_MODELS[0]);
        let current = if Path::new(wanted).is_file() {
            files.insert(0, PathBuf::from(wanted));
            0
        } else {
            let w = wanted
                .trim_end_matches(".gr2")
                .trim_end_matches(".GR2")
                .to_ascii_lowercase();
            files.iter().position(|p| stem(p) == w).unwrap_or(0)
        };
        Self {
            files,
            texture_copies,
            current,
            reload: true,
            textures,
            status: Vec::new(),
            yaw: 0.6,
            pitch: 0.4,
            distance: 30.0,
            target: Vec3::ZERO,
            shots: None,
        }
    }
}

/// The triangles of one group of a mesh, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub mesh: String,
    /// The mesh's index in the file.
    pub mesh_index: usize,
    /// The group's index among its mesh's triangle groups, in the file's
    /// order (GFX-OBJ-2.8a: a ship's part number).
    pub group: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// `TextureCoordinates1` and `2`, zero where the mesh has none.
    pub uvs1: Vec<[f32; 2]>,
    pub uvs2: Vec<[f32; 2]>,
    /// `DiffuseColor0` as RGBA, white where the mesh has none.
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    /// Base name of the texture of the group's material.
    pub texture: Option<String>,
    /// Base names of all the textures of the group's material.
    pub textures: Vec<String>,
}

/// Every mesh's triangle groups (GFX-GR2-4.2) as parts, with positions in
/// the view's Y-up axes, unit normals and V as stored. A mesh without groups is one part.
pub fn parts(model: &Gr2) -> Vec<Part> {
    parts_with(model, z_up_to_y_up, z_up_to_y_up, false)
}

/// [`parts`] with positions and normals turned by `position` and `normal`
/// (normals are scaled to unit length afterwards), and the triangles'
/// corner order reversed when `flip` is set (a mirroring transform).
pub fn parts_with(
    model: &Gr2,
    position: impl Fn([f32; 3]) -> [f32; 3],
    normal: impl Fn([f32; 3]) -> [f32; 3],
    flip: bool,
) -> Vec<Part> {
    let mut out = Vec::new();
    for (mesh_index, mesh) in model.meshes.iter().enumerate() {
        let v = &mesh.vertices;
        let set = |i: usize| -> Vec<[f32; 2]> {
            match v.uvs.get(i) {
                Some(uv) if uv.len() == v.count => uv.clone(),
                _ => vec![[0.0, 0.0]; v.count],
            }
        };
        let (uvs1, uvs2) = (set(1), set(2));
        let colors: Vec<[f32; 4]> = if v.colors.len() == v.count {
            v.colors
                .iter()
                .map(|c| {
                    let at = |i: usize, d: f32| c.get(i).copied().unwrap_or(d);
                    [at(0, 1.0), at(1, 1.0), at(2, 1.0), at(3, 1.0)]
                })
                .collect()
        } else {
            vec![[1.0; 4]; v.count]
        };
        let positions: Vec<[f32; 3]> = v.positions.iter().map(|&p| position(p)).collect();
        let normals: Vec<[f32; 3]> = if v.normals.len() == v.count {
            v.unit_normals()
                .into_iter()
                .map(|n| crate::render::world3d::normalize(normal(n)))
                .collect()
        } else {
            vec![[0.0, 1.0, 0.0]; v.count]
        };
        let uvs: Vec<[f32; 2]> = match v.uvs.first() {
            Some(uv) if uv.len() == v.count => uv.clone(),
            _ => vec![[0.0, 0.0]; v.count],
        };
        let groups = if mesh.groups.is_empty() {
            vec![gr2::TriangleGroup {
                material: 0,
                first_triangle: 0,
                triangle_count: mesh.triangle_count() as u32,
            }]
        } else {
            mesh.groups.clone()
        };
        for (group, g) in groups.into_iter().enumerate() {
            let start = 3 * g.first_triangle as usize;
            let end = (start + 3 * g.triangle_count as usize).min(mesh.indices.len());
            let mut indices: Vec<u32> = mesh.indices.get(start..end).unwrap_or_default().to_vec();
            if flip {
                indices
                    .as_chunks_mut::<3>()
                    .0
                    .iter_mut()
                    .for_each(|t| t.swap(1, 2));
            }
            let textures: Vec<String> = mesh
                .materials
                .get(g.material as usize)
                .copied()
                .flatten()
                .and_then(|m| model.materials.get(m))
                .map(|m| {
                    m.textures
                        .iter()
                        .filter_map(|&t| model.textures.get(t))
                        .map(|t| t.base_name().to_owned())
                        .collect()
                })
                .unwrap_or_default();
            out.push(Part {
                mesh: mesh.name.clone(),
                mesh_index,
                group,
                positions: positions.clone(),
                normals: normals.clone(),
                uvs: uvs.clone(),
                uvs1: uvs1.clone(),
                uvs2: uvs2.clone(),
                colors: colors.clone(),
                indices,
                texture: textures.first().cloned(),
                textures,
            });
        }
    }
    out
}

/// Max's Z-up to the view's Y-up: (x, y, z) to (x, z, -y).
fn z_up_to_y_up([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

/// The centre and radius of everything in `parts`.
pub fn bounds(parts: &[Part]) -> (Vec3, f32) {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for p in parts.iter().flat_map(|p| &p.positions) {
        let v = Vec3::from(*p);
        min = min.min(v);
        max = max.max(v);
    }
    if min.x > max.x {
        return (Vec3::ZERO, 1.0);
    }
    ((min + max) / 2.0, ((max - min).length() / 2.0).max(0.01))
}

/// Marks the 3D camera.
#[derive(Component)]
pub struct ModelCamera;

/// Marks the entities of the model on screen.
#[derive(Component)]
pub struct ModelEntity;

pub fn setup_model_view(mut commands: Commands) {
    commands.spawn((
        ModelCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: 1,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 600.0,
        affects_lightmapped_meshes: true,
    });
}

/// M switches between the map and the model view.
pub fn toggle_view(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<State<ViewMode>>,
    mut next: ResMut<NextState<ViewMode>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        next.set(match mode.get() {
            ViewMode::Map => ViewMode::Model,
            ViewMode::Model => ViewMode::Map,
        });
    }
}

/// The root UI nodes (the info panel).
type RootPanels = (With<Node>, Without<ChildOf>);

/// The cameras the view mode switches; the ship window's camera
/// (`ship_view::aim`) and the water's mirror camera (`view3d::place_mirror`)
/// switch themselves.
type SwitchedCamera = (
    Entity,
    &'static mut Camera,
    Has<ModelCamera>,
    Has<crate::render::view3d::MapCamera3d>,
);

/// The cameras that switch themselves: the ship window's, the building
/// portrait's (`portrait::aim`) and the water's mirror camera.
type SelfSwitched = (
    Without<crate::render::ship_view::ShipCamera>,
    Without<crate::render::portrait::PortraitCamera>,
    Without<crate::render::view3d::MirrorCamera3d>,
);

/// On entering or leaving the model view: the active camera and the camera
/// the info panel is drawn on.
pub fn switch_cameras(
    mode: Res<State<ViewMode>>,
    view: Res<crate::render::view3d::View3d>,
    mut cameras: Query<SwitchedCamera, SelfSwitched>,
    mut commands: Commands,
    panels: Query<(Entity, Option<&UiTargetCamera>), RootPanels>,
) {
    let model = *mode.get() == ViewMode::Model;
    let mut target = None;
    for (entity, mut camera, is_model, is_map_3d) in &mut cameras {
        let active = match (model, is_model) {
            (true, model_camera) => model_camera,
            (false, true) => false,
            (false, false) => is_map_3d == view.active,
        };
        if camera.is_active != active {
            camera.is_active = active;
        }
        if active {
            target = Some(entity);
        }
    }
    let Some(target) = target else { return };
    for (panel, current) in &panels {
        if current.map(|c| c.0) != Some(target) {
            commands.entity(panel).insert(UiTargetCamera(target));
        }
    }
}

/// Counts frames in the model view, so that `--shot` works there too.
pub fn count_frames(mut viewer: ResMut<crate::game::Viewer>) {
    viewer.frames_since_load += 1;
}

/// N and P step through the models.
pub fn cycle_models(keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<ModelView>) {
    let count = view.files.len().max(1);
    if keys.just_pressed(KeyCode::KeyN) {
        view.current = (view.current + 1) % count;
        view.reload = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        view.current = (view.current + count - 1) % count;
        view.reload = true;
    }
}

fn texture_image(path: &Path) -> Result<Image, String> {
    let name = path.to_str().ok_or("path is not UTF-8")?;
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let t = s2_assets::dds::load(name, &data).map_err(|e| e.to_string())?;
    let mut image = Image::new(
        Extent3d {
            width: t.width,
            height: t.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        t.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    Ok(image)
}

/// Load the current model when it changed.
#[allow(clippy::too_many_arguments)]
pub fn load_model(
    mut commands: Commands,
    mut view: ResMut<ModelView>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<ModelEntity>>,
) {
    if !view.reload {
        return;
    }
    view.reload = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    view.status.clear();
    let Some(path) = view.files.get(view.current).cloned() else {
        view.status.push("no .gr2 files found (S2_GAME_DIR)".into());
        return;
    };
    let title = format!(
        "{} (model {} of {})",
        path.file_name().unwrap_or_default().to_string_lossy(),
        view.current + 1,
        view.files.len()
    );
    view.status.push(title);
    let loaded = std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|raw| {
            gr2::load(path.to_str().unwrap_or_default(), &raw).map_err(|e| e.to_string())
        });
    let model = match loaded {
        Ok(model) => model,
        Err(e) => {
            view.status.push(format!("cannot read: {e}"));
            return;
        }
    };
    // `--model-shots`: the game's frame and a building's heading.
    let (parts, rotation) = if view.shots.is_some() {
        let basis = crate::render::models3d::basis_conversion(model.art_tool.as_ref(), 1.0);
        let parts = crate::render::models3d::converted_parts(
            &model,
            crate::render::models3d::IntoBevy::into_bevy(basis),
        );
        let building = path
            .components()
            .any(|c| c.as_os_str().eq_ignore_ascii_case("buildings"));
        let rotation = if building {
            Quat::from_rotation_y(crate::render::world3d::bevy_yaw(
                (s2_assets::model_tables::DEFAULT_BUILDING_HEADING as f32).to_radians(),
            ))
        } else {
            Quat::IDENTITY
        };
        (parts, rotation)
    } else {
        (parts(&model), Quat::IDENTITY)
    };
    let triangles: usize = parts.iter().map(|p| p.indices.len() / 3).sum();
    view.status.push(format!(
        "{} meshes, {} parts, {} triangles, {} bones, {} animations",
        model.meshes.len(),
        parts.len(),
        triangles,
        model.skeletons.iter().map(|s| s.bones.len()).sum::<usize>(),
        model.animations.len()
    ));
    let mut cache: HashMap<String, Option<Handle<Image>>> = HashMap::new();
    for part in &parts {
        let texture = part.texture.as_ref().and_then(|name| {
            cache
                .entry(name.clone())
                .or_insert_with(|| {
                    let path = view.textures.get(&name.to_ascii_lowercase())?;
                    texture_image(path).ok().map(|image| images.add(image))
                })
                .clone()
        });
        if let (Some(name), None) = (&part.texture, &texture) {
            let line = format!("texture not found or not readable: {name}");
            if !view.status.contains(&line) {
                view.status.push(line);
            }
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, part.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, part.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, part.uvs.clone())
        .with_inserted_indices(Indices::U32(part.indices.clone()));
        let material = StandardMaterial {
            base_color: if texture.is_some() {
                Color::WHITE
            } else {
                Color::srgb(0.7, 0.7, 0.7)
            },
            base_color_texture: texture,
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            cull_mode: None,
            perceptual_roughness: 0.9,
            ..default()
        };
        commands.spawn((
            ModelEntity,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_rotation(rotation),
        ));
    }
    let (center, radius) = bounds(&parts);
    view.target = center;
    view.distance = radius * 2.8;
}

/// Left drag orbits, the wheel zooms, R resets.
pub fn orbit_camera(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut view: ResMut<ModelView>,
    mut camera: Single<&mut Transform, With<ModelCamera>>,
) {
    if view.shots.is_some() {
        return;
    }
    if buttons.pressed(MouseButton::Left) {
        view.yaw -= motion.delta.x * 0.01;
        view.pitch = (view.pitch + motion.delta.y * 0.01).clamp(-1.5, 1.5);
    }
    if scroll.delta.y != 0.0 {
        view.distance *= 1.15f32.powf(-scroll.delta.y.signum());
    }
    if keys.just_pressed(KeyCode::KeyR) {
        view.yaw = 0.6;
        view.pitch = 0.4;
    }
    let offset = Vec3::new(
        view.distance * view.pitch.cos() * view.yaw.sin(),
        view.distance * view.pitch.sin(),
        view.distance * view.pitch.cos() * view.yaw.cos(),
    );
    **camera = Transform::from_translation(view.target + offset).looking_at(view.target, Vec3::Y);
}

/// `--model-shots`: the game camera of [`game_eye`], its projection
/// (GFX-VIEW-3.5: 50 degrees across, near 1, far 500) and the flat
/// background.
pub fn game_camera(
    view: Res<ModelView>,
    window: Single<&Window>,
    mut camera: Single<(&mut Transform, &mut Projection, &mut Camera), With<ModelCamera>>,
) {
    let Some(shots) = &view.shots else { return };
    let (transform, projection, camera) = &mut *camera;
    let eye = game_eye(shots.zoom);
    let position = Vec3::from(crate::render::world3d::to_bevy(eye.position));
    let direction = Vec3::from(crate::render::world3d::to_bevy(eye.direction));
    **transform = Transform::from_translation(position).looking_to(direction, Vec3::Y);
    let aspect = window.width() / window.height().max(1.0);
    let fov = crate::render::world3d::vertical_fov(aspect);
    if let Projection::Perspective(p) = &mut **projection {
        if (p.fov - fov).abs() > 1e-6 || p.near != crate::render::world3d::NEAR {
            p.fov = fov;
            p.near = crate::render::world3d::NEAR;
            p.far = crate::render::world3d::FAR;
        }
    }
    if !matches!(camera.clear_color, ClearColorConfig::Custom(c) if c == SHOT_BACKGROUND) {
        camera.clear_color = ClearColorConfig::Custom(SHOT_BACKGROUND);
    }
}

/// `--model-shots`: once a model has been on screen for a few frames, save
/// its picture as `<file stem>.png`; once it is saved, the next model;
/// after the last, quit.
pub fn model_shots(
    mut commands: Commands,
    mut view: ResMut<ModelView>,
    mut exit: MessageWriter<AppExit>,
) {
    if view.reload {
        return;
    }
    let current = view.current;
    let count = view.files.len();
    let stem = view
        .files
        .get(current)
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().into_owned());
    let Some(shots) = view.shots.as_mut() else {
        return;
    };
    shots.frames += 1;
    let wait = if shots.asked == 0 {
        SHOT_FIRST_FRAMES
    } else {
        SHOT_FRAMES
    };
    let saved = shots.saved.load(Ordering::SeqCst);
    if shots.asked == current && shots.frames >= wait {
        let Some(stem) = stem else {
            exit.write(AppExit::Success);
            return;
        };
        let path = shots.dir.join(format!("{stem}.png"));
        let done = shots.saved.clone();
        commands.spawn(Screenshot::primary_window()).observe(
            move |shot: On<ScreenshotCaptured>| {
                save_to_disk(path.clone())(shot);
                done.fetch_add(1, Ordering::SeqCst);
            },
        );
        shots.asked += 1;
    } else if shots.asked == current + 1 && saved == shots.asked {
        if current + 1 < count {
            shots.frames = 0;
            view.current = current + 1;
            view.reload = true;
        } else {
            exit.write(AppExit::Success);
        }
    }
}

/// The info panel in the model view (none in `--model-shots`).
pub fn model_info(
    view: Res<ModelView>,
    info: Single<(&mut Text, &mut Visibility), With<crate::game::InfoText>>,
) {
    let (mut text, mut visibility) = info.into_inner();
    if view.shots.is_some() {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let mut lines = view.status.clone();
    lines
        .push("M back to the map  N/P next/previous model  drag orbit  wheel zoom  R reset".into());
    let new_text = lines.join("\n");
    if text.0 != new_text {
        text.0 = new_text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_assets::gr2::{Material, Mesh as GrMesh, TextureRef, TriangleGroup, Vertices};

    fn two_groups() -> Gr2 {
        let vertices = Vertices {
            count: 4,
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 0.0, 2.0],
            ],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uvs: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.25]]],
            ..Vertices::default()
        };
        Gr2 {
            textures: vec![TextureRef {
                file_name: "Z:\\a\\Wood.dds".into(),
                width: 4,
                height: 4,
            }],
            materials: vec![
                Material {
                    name: "wood".into(),
                    textures: vec![0],
                },
                Material {
                    name: "bare".into(),
                    textures: vec![],
                },
            ],
            meshes: vec![GrMesh {
                name: "House".into(),
                vertices,
                indices: vec![0, 1, 2, 0, 2, 3],
                groups: vec![
                    TriangleGroup {
                        material: 0,
                        first_triangle: 0,
                        triangle_count: 1,
                    },
                    TriangleGroup {
                        material: 1,
                        first_triangle: 1,
                        triangle_count: 1,
                    },
                ],
                materials: vec![Some(0), Some(1)],
                bone_bindings: vec![],
            }],
            ..Gr2::default()
        }
    }

    // spec: GFX-GR2-4.2, GFX-GR2-5.1
    #[test]
    fn groups_become_parts_with_their_textures() {
        let parts = parts(&two_groups());
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].indices, [0, 1, 2]);
        assert_eq!(parts[1].indices, [0, 2, 3]);
        assert_eq!(parts[0].texture.as_deref(), Some("Wood.dds"));
        assert_eq!(parts[1].texture, None);
    }

    // spec: GFX-GR2-5.1
    #[test]
    fn a_texture_name_in_two_folders_does_not_depend_on_the_listing_order() {
        let listed =
            |order: &[&str]| texture_index(order.iter().map(|p| PathBuf::from(*p)).collect());
        // As ext4 and NTFS list the folders: the same index either way.
        let ext4 = listed(&[
            "g/data/map/textures/z_rock_00.dds",
            "g/data/character/donkey/EselTextue.dds",
            "g/data/map/objects/z_rock_00.dds",
            "g/data/animals/EselTextue.dds",
            "g/data/buildings/Mill.dds",
        ]);
        let ntfs = listed(&[
            "g/data/animals/EselTextue.dds",
            "g/data/buildings/Mill.dds",
            "g/data/character/donkey/EselTextue.dds",
            "g/data/map/objects/z_rock_00.dds",
            "g/data/map/textures/z_rock_00.dds",
        ]);
        assert_eq!(ext4, ntfs);
        let (textures, copies) = ext4;
        assert_eq!(
            textures["z_rock_00.dds"],
            PathBuf::from("g/data/map/objects/z_rock_00.dds")
        );
        assert_eq!(
            textures["eseltextue.dds"],
            PathBuf::from("g/data/animals/EselTextue.dds")
        );
        assert_eq!(copies.len(), 2, "only the names in two folders");
        assert_eq!(copies["eseltextue.dds"].len(), 2);
        assert!(!copies.contains_key("mill.dds"));
    }

    // spec: GFX-VIEW-1.2, GFX-GR2-7.4
    #[test]
    fn axes_and_texture_origin() {
        let parts = parts(&two_groups());
        // Max's +Z up is the view's +Y; V as stored (GFX-GR2-7.4).
        assert_eq!(parts[0].positions[3], [0.0, 2.0, -0.0]);
        assert_eq!(parts[0].normals[0], [0.0, 1.0, -0.0]);
        assert_eq!(parts[0].uvs[3], [0.0, 0.25]);
        let (center, radius) = bounds(&parts);
        assert_eq!(center, Vec3::new(0.5, 1.0, -0.5));
        assert!((radius - 1.2247).abs() < 1e-3);
    }

    // GFX-VIEW-3.3, 3.6: at the start zoom 3 the camera looks down 53.13
    // degrees and sits 29.8 units behind and 55.5 above the target, which
    // the jump puts 18 units in front of the field at the floor, 5 above
    // dry ground.
    #[test]
    fn the_shot_camera_is_the_start_camera() {
        let eye = game_eye(3);
        let close = |a: f32, b: f32| (a - b).abs() < 0.05;
        let [x, y, z] = eye.position;
        assert!(
            close(x, 0.0) && close(y, 5.0 + 55.53) && close(z, -18.0 - 29.75),
            "{eye:?}"
        );
        let [dx, dy, dz] = eye.direction;
        assert!(
            close(dx, 0.0) && close(dy, -0.8) && close(dz, 0.6),
            "{eye:?}"
        );
        // Zoom 1: 36.87 degrees down, 21 units out.
        let eye = game_eye(1);
        assert!(close(eye.direction[1], -0.6), "{eye:?}");
    }

    // spec: none (the model shot list option)
    #[test]
    fn model_lists_are_relative_to_the_game() {
        let dir = std::env::temp_dir().join(format!("s2-model-list-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let list = dir.join("list.txt");
        std::fs::write(&list, "# models\ndata/a.GR2\n\n/abs/b.GR2\n").expect("list");
        let files = read_model_list(&list, Some(Path::new("/game"))).expect("read");
        assert_eq!(
            files,
            vec![
                PathBuf::from("/game/data/a.GR2"),
                PathBuf::from("/abs/b.GR2")
            ]
        );
        std::fs::write(&list, "\n").expect("list");
        assert!(read_model_list(&list, None).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
