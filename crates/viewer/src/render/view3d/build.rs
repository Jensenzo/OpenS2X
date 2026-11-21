//! Building the 3D scene of a map: terrain cells, water, sky, light and the textures they use.

use super::*;

/// Hook W17b: what Bevy's occlusion (its visibility-bitmask GTAO at High
/// quality, 3 samples per slice side, 6 sectors a slice) gives a flat
/// surface: a sample lying on the surface marks the sector at the
/// hemisphere's edge, one of six, so open ground reads 1 - 1/6 (measured
/// 0.80 to 0.83 on Storm Coast's ground). The shaders divide by it
/// (`AO_FLAT` in terrain.wesl and object.wesl), so open ground is
/// unoccluded and only real occlusion darkens. The shaders hold the number;
/// this one is the tests' account of it.
#[cfg(test)]
pub(crate) const AO_FLAT: f32 = 1.0 - 1.0 / 6.0;

/// Hook W17b: the map camera's parts for a project's ambient occlusion:
/// Bevy's screen-space ambient occlusion (which brings the depth and normal
/// prepasses the terrain and object materials draw) and no multisampling,
/// which it needs; none by default.
pub(crate) fn ambient_occlusion(
    on: bool,
) -> Option<(
    bevy::pbr::ScreenSpaceAmbientOcclusion,
    bevy::render::view::Msaa,
)> {
    on.then(|| {
        (
            bevy::pbr::ScreenSpaceAmbientOcclusion {
                quality_level: bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::High,
                ..default()
            },
            bevy::render::view::Msaa::Off,
        )
    })
}

pub fn setup_3d(mut commands: Commands, menu: Res<crate::menus::menu::MenuState>) {
    let render = crate::extend::render();
    let shadow_map = drawn_shadow_map(&menu.settings);
    let mut camera = commands.spawn((
        MapCamera3d,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: 2,
            // The main texture holds stored values (GFX-VIEW-2.6 c1): the
            // background's own numbers, not converted.
            clear_color: ClearColorConfig::Custom(Color::linear_rgb(
                world3d::BACKGROUND[0],
                world3d::BACKGROUND[1],
                world3d::BACKGROUND[2],
            )),
            ..default()
        },
        // GFX-VIEW-2.6 c1: a float main texture the shaders write stored
        // values into, blended straight; turned to linear for the window
        // after the 3D passes (`stored.rs`).
        bevy::camera::Hdr,
        crate::render::stored::StoredColours,
        // GFX-VIEW-3.5: set each frame from the window's aspect.
        Projection::Perspective(PerspectiveProjection {
            fov: world3d::vertical_fov(4.0 / 3.0),
            near: world3d::NEAR,
            far: world3d::FAR,
            ..default()
        }),
        // The shaders produce the final colours themselves, undithered.
        Tonemapping::None,
        bevy::render::view::DebandDither::Disabled,
        // GFX-VIEW-2.5c: 2 x 2 percentage-closer filtering (a project's
        // filter instead, hook W17).
        render.shadow_filter.method(),
        RenderLayers::layer(MAP_LAYER),
        Transform::from_xyz(0.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    if let Some(occlusion) = ambient_occlusion(render.ambient_occlusion) {
        camera.insert(occlusion);
    }
    // GFX-VIEW-2.5: the light's shadow map (where it differs from the
    // spec: docs/shadows.md, Q-215). The shaders do their own
    // lighting (GFX-VIEW-2.3) and only read its shadow; its frustum is
    // fitted to the view (2.5d) in two cascades.
    // GFX-VIEW-2.5 e: with shadows off the map stays at its smallest and
    // the receivers' shadow factor is 1 (`receivers_light`).
    commands.insert_resource(bevy::light::DirectionalLightShadowMap {
        size: shadow_map.unwrap_or(SHADOWS_OFF_MAP),
    });
    // The light of the map shown is set when its scene is built.
    let direction = Vec3::from(world3d::to_bevy(world3d::DayLight::XML.direction()));
    commands.spawn((
        shadow_light(),
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: render.shadow_cascades,
            minimum_distance: world3d::NEAR,
            first_cascade_far_bound: 80.0,
            maximum_distance: 300.0,
            overlap_proportion: 0.2,
        }
        .build(),
        Transform::IDENTITY.looking_to(direction, Vec3::Y),
        RenderLayers::layer(MAP_LAYER),
    ));
}

/// GFX-VIEW-2.5 c, f (T43): the light of the shadow map. Its test is the
/// pixel's own light-space depth with a constant bias toward the light and
/// no offset along the surface normal, so a wall the sun only grazes, right
/// under its roof's overhang, stays in the overhang's shadow (Bevy's
/// default pushes the lookup 1.8 map texels out along the normal, which
/// lit the houses' door walls).
pub fn shadow_light() -> DirectionalLight {
    DirectionalLight {
        shadow_maps_enabled: true,
        shadow_depth_bias: SHADOW_DEPTH_BIAS,
        shadow_normal_bias: 0.0,
        ..default()
    }
}

/// GFX-VIEW-2.5 c (Q-215: an empirical bias with neither acne nor visibly
/// detached shadows): the depth bias in world units toward the light. At
/// the town camera of 12 Nov (tick 6000) 0.02 and 0.05 left acne on the
/// quarries' roofs (6 .. 12 %, 3 .. 7 % darker than the original); 0.1
/// leaves 2 .. 4.5 % and darkens the woodcutter's door wall as the
/// original's (docs/shadows.md).
pub const SHADOW_DEPTH_BIAS: f32 = 0.1;

/// GFX-VIEW-2.5 e: the map's size while shadows are off (it stays: see
/// [`receivers_light`]).
pub(super) const SHADOWS_OFF_MAP: usize = 128;

/// GFX-VIEW-2.5 d, e: the size of the shadow map drawn with these
/// settings before any game (the program's start), none with shadows off:
/// a project's own size (docs/extending.md; 0 none), else the profile's
/// ([`game_shadow_map`], [`frame_shadow_map`]). A game sets its own at its
/// start (SET-4b).
pub fn drawn_shadow_map(settings: &crate::menus::settings::Settings) -> Option<usize> {
    GameShadows::at_start(settings).now(settings)
}

/// SET-4b: what a game's view copies of the shadow settings at its start
/// (a new or a loaded game): a project's own map size (docs/extending.md:
/// fixed, the switch not read), else the profile's map size and shader
/// level ([`game_shadow_map`]). The shadows switch is read every frame
/// ([`GameShadows::now`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameShadows {
    project: Option<usize>,
    map: Option<usize>,
    /// Hook W17: a project's factor on the profile's size (`None` is 1).
    scale: Option<u32>,
}

impl GameShadows {
    pub fn at_start(settings: &crate::menus::settings::Settings) -> Self {
        let render = crate::extend::render();
        Self {
            project: render.shadow_map_size,
            map: game_shadow_map(&settings.cfg.scene),
            scale: render.shadow_map_scale,
        }
    }

    /// SET-4b: the shadow map this frame draws with, none with shadows off.
    pub fn now(&self, settings: &crate::menus::settings::Settings) -> Option<usize> {
        match self.project {
            Some(0) => None,
            Some(size) => Some(size),
            None => scaled_shadow_map(frame_shadow_map(&settings.cfg.scene, self.map), self.scale),
        }
    }
}

/// Hook W17: the profile's map size times a project's factor, at most
/// [`crate::extend::MAX_SHADOW_MAP`]; none stays none (shadows off).
pub fn scaled_shadow_map(size: Option<usize>, scale: Option<u32>) -> Option<usize> {
    let factor = scale.unwrap_or(1).max(1) as usize;
    size.map(|s| (s * factor).min(crate::extend::MAX_SHADOW_MAP.max(s)))
}

/// SET-4b, GFX-VIEW-2.5 d: the shadow map a game starts with: its size
/// from `SHADOW QUALITY` and only at shader level 2 (shader detail low or
/// high), both kept until the next game; none at quality 0.
pub fn game_shadow_map(scene: &s2_assets::cfg::SceneSettings) -> Option<usize> {
    (scene.shader_detail() >= 2)
        .then(|| shadow_map_size(scene.shadow_quality()))
        .flatten()
}

/// SET-4b, GFX-VIEW-2.5 e: a frame draws the game's map while the shadows
/// switch (SET-4a field 10) is on, read every frame; a game started
/// without a map draws none (2.5 e: what the original then draws was not
/// read; taken as off).
pub fn frame_shadow_map(
    scene: &s2_assets::cfg::SceneSettings,
    game: Option<usize>,
) -> Option<usize> {
    scene.shadows().then_some(game).flatten()
}

/// GFX-VIEW-2.5 e: with shadows off every receiver's shadow test gives lit
/// and nothing else changes: the shaders' shadow factor (the light's fourth
/// value, the factor kept in a shadow) is 1, so whatever the map holds
/// changes no pixel. The map itself stays, at its smallest: with no light
/// casting shadows at all, the water's mirror camera drew nothing (Bevy
/// 0.20; its image stayed black and darkened the sea, 2026-11-12).
pub fn receivers_light(mut light: world3d::ShaderLight, shadows: bool) -> world3d::ShaderLight {
    if !shadows {
        light.direction[3] = 1.0;
    }
    light
}

/// GFX-VIEW-2.5 d, e: the shadow map of a profile's scene settings: drawn
/// only while the shadows switch is on (SET-4a field 10) and the shader
/// detail is 2 or more; `SHADOW QUALITY` only sizes it, and a switch on
/// with quality 0 (no settings page writes it) makes no map, taken as off.
/// With none every receiver's shadow test gives lit and nothing else
/// changes (2.5 e): the shaders' lookups give 1 without cascades. (A game
/// copies the size and the shader level at its start, SET-4b.)
#[cfg(test)]
pub fn scene_shadow_map(scene: &s2_assets::cfg::SceneSettings) -> Option<usize> {
    frame_shadow_map(scene, game_shadow_map(scene))
}

/// GFX-VIEW-2.5 d (Run D): texels per side of the shadow map for
/// `SHADOW QUALITY` q: 128 x 2^round(4 q), none (shadows off) at 0. Bevy
/// gives each of its two cascades a map of this size (docs/shadows.md).
pub fn shadow_map_size(quality: f32) -> Option<usize> {
    let level = (4.0 * quality.clamp(0.0, 1.0)).round() as u32;
    (level > 0).then(|| 128 << level)
}

/// GFX-WATER-4.5a (a), 6.1: the map border's water: the surface's material
/// (6.2: the same shader and world texture) whose passes do not test the
/// shadow map.
pub(crate) fn skirt_material(mut surface: WaterMaterial) -> WaterMaterial {
    surface.params.opacity.z = 0.0;
    surface
}

pub(crate) fn vec4(v: [f32; 3], w: f32) -> Vec4 {
    Vec4::new(v[0], v[1], v[2], w)
}

/// Texture coordinates repeat and filter linearly, with mip maps.
pub(crate) fn repeating_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: crate::extend::render().anisotropy,
        ..default()
    })
}

/// GFX-VIEW-2.4b: the normal maps' sampler: trilinear (linear
/// magnification, minification and mip filter), wrapping, no anisotropy.
pub(crate) fn normal_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    })
}

/// GFX-VIEW-2.4b: one normal-map array of [`NORMAL_SIZES`]`[class]`.
pub(crate) fn normal_array_image(layers: &Layers, class: usize) -> Image {
    let mut image = chain_array_image(&layers.normals[class], NORMAL_SIZES[class]);
    image.sampler = normal_sampler();
    image
}

/// An array texture of square RGBA8 layers with their mip chains.
pub(crate) fn array_image(layers: &[Vec<u8>], size: u32) -> Image {
    let chains: Vec<Vec<u8>> = layers
        .iter()
        .map(|layer| textures::mip_chain(layer, size).0)
        .collect();
    chain_array_image(&chains, size)
}

/// An array texture of square RGBA8 layers of `size`, each given as its
/// whole mip chain down to 1 x 1 (top level first); no layers make one
/// transparent layer.
pub(crate) fn chain_array_image(chains: &[Vec<u8>], size: u32) -> Image {
    let levels = size.ilog2() + 1;
    let data = if chains.is_empty() {
        textures::mip_chain(&textures::solid(size, [128, 128, 255, 0]), size).0
    } else {
        chains.concat()
    };
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: chains.len().max(1) as u32,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = levels;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = repeating_sampler();
    image
}

/// A single RGBA8 texture with mips, repeating.
pub(crate) fn repeating_image(texture: &s2_assets::dds::Texture) -> Image {
    let size = texture
        .width
        .max(texture.height)
        .next_power_of_two()
        .clamp(1, 1024);
    let layer = textures::resample(texture, size);
    let (chain, levels) = textures::mip_chain(&layer, size);
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = repeating_sampler();
    image
}

/// GFX-VIEW-5.1: the world texture as an image, clamped at the edges.
pub(crate) fn world_image(texture: terrain3d::WorldTexture) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture.texels,
        TextureFormat::Rg8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

/// The world texture of the player's view of `world`: GFX-VIEW-5.1. A game
/// without rules has no players, and every field is shown (as with
/// `--no-fog`: `fog` false).
pub(crate) fn fog_texture(world: &s2_sim::World, fog: bool) -> terrain3d::WorldTexture {
    let terrain = world.terrain();
    terrain3d::world_texture(
        terrain.width,
        terrain.height,
        |x, y| !fog || world.is_explored(crate::game::PLAYER, (x, y)),
        |x, y| terrain3d::water_depth_byte(terrain.height_at((x, y))),
    )
}

/// A terrain cell's mesh.
pub(crate) fn cell_bevy_mesh(data: terrain3d::CellMesh) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, data.tangents)
    .with_inserted_attribute(ATTRIBUTE_HAZE, data.haze)
    .with_inserted_indices(Indices::U32(data.indices))
}

pub(crate) fn flat_mesh(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// GFX-VIEW-2.1b: the heights the terrain is drawn from: the world's, with
/// the normals of the set-up heights outside the refreshed corner.
pub(crate) fn terrain_heights<'a>(
    viewer: &'a crate::game::Viewer,
    terrain: &'a s2_sim::map::Terrain,
) -> terrain3d::TerrainHeights<'a, s2_sim::map::Terrain, s2_sim::map::Terrain> {
    let setup = viewer
        .setup_terrain
        .as_ref()
        .filter(|s| (s.width, s.height) == (terrain.width, terrain.height));
    terrain3d::TerrainHeights {
        current: terrain,
        setup,
    }
}

/// The heights a cell's mesh is made from, to see when levelling or a
/// road's evening changed them: its own samples and the ring of samples
/// around them, which its vertex normals read (GFX-VIEW-2.2: the row and
/// column neighbours). The original rebuilds the cells over a changed box
/// widened by 16 samples, then the grid normals (GFX-VIEW-2.4); without the
/// ring the next cell kept its stale normals along the shared edge.
pub(crate) fn cell_heights(
    terrain: &s2_sim::map::Terrain,
    frame: &Frame,
    cx: u32,
    cy: u32,
) -> Vec<i32> {
    let ([c0, c1], [r0, r1]) = terrain3d::cell_samples(frame, cx, cy);
    let (c0, r0) = (c0.saturating_sub(1), r0.saturating_sub(1));
    let (c1, r1) = (
        (c1 + 1).min(frame.columns - 1),
        (r1 + 1).min(frame.rows - 1),
    );
    (r0..=r1)
        .flat_map(|r| (c0..=c1).map(move |c| (c, r)))
        .map(|(c, r)| terrain.raw(c, r))
        .collect()
}

/// The colour and normal layers of the patterns of `map`, in id order, and
/// the layer of every field (GFX-PAT, GFX-VIEW-2.4). A pattern without a
/// texture is a flat layer of its 2D colour.
pub(crate) struct Layers {
    pub(crate) colors: Vec<Vec<u8>>,
    /// GFX-VIEW-2.4b: the normal maps' whole mip chains, by array of
    /// [`NORMAL_SIZES`].
    pub(crate) normals: [Vec<Vec<u8>>; NORMAL_SIZES.len()],
    pub(crate) params: [Vec4; MAX_LAYERS],
    pub(crate) kinds: [UVec4; MAX_LAYERS],
    /// GFX-VIEW-2.4a: per layer the highlight's intensity and power, zero
    /// but for type-3 patterns.
    pub(crate) specular: [Vec4; MAX_LAYERS],
    pub(crate) fields: Vec<u8>,
}

pub(crate) fn pattern_layers(viewer: &crate::game::Viewer, map: &s2_assets::s2m::Map) -> Layers {
    let used: BTreeSet<u32> = map.patterns.iter().copied().collect();
    let ids: Vec<u32> = used.into_iter().take(MAX_LAYERS).collect();
    let mut layers = Layers {
        colors: Vec::new(),
        normals: Default::default(),
        params: [Vec4::new(0.1, 0.1, 0.1, 0.1); MAX_LAYERS],
        // Patterns without a texture come last in priority.
        kinds: [UVec4::new(0, u32::MAX, 0, 0); MAX_LAYERS],
        specular: [Vec4::new(0.0, 1.0, 0.0, 0.0); MAX_LAYERS],
        fields: Vec::new(),
    };
    // A pattern without a normal map: flat, in the smallest array.
    let flat_normal = textures::mip_chain(
        &textures::solid(NORMAL_SIZES[0], [128, 128, 255, 255]),
        NORMAL_SIZES[0],
    )
    .0;
    // GFX-VIEW-2.4b: each normal map in the array of its own size, with its
    // own mip chain; `kinds.w` says where.
    let place_normal = |layers: &mut Layers, chain: Vec<u8>, class: usize| -> u32 {
        let list = &mut layers.normals[class];
        list.push(chain);
        ((class as u32) << 8) | (list.len() as u32 - 1)
    };
    for (i, id) in ids.iter().enumerate() {
        match viewer.patterns.get(id) {
            Some(pattern) => {
                layers
                    .colors
                    .push(textures::resample(&pattern.texture, COLOR_SIZE));
                let normal = match &pattern.normal {
                    Some(normal) => {
                        let class = normal_class(normal.width, normal.height);
                        let chain =
                            textures::chain_at(normal, &pattern.normal_levels, NORMAL_SIZES[class]);
                        place_normal(&mut layers, chain, class)
                    }
                    None => place_normal(&mut layers, flat_normal.clone(), 0),
                };
                let [su, sv] = pattern.scale;
                let [nu, nv] = pattern.normal_scale;
                layers.params[i] = Vec4::new(su, sv, nu, nv);
                layers.kinds[i] = UVec4::new(
                    pattern.kind,
                    pattern.priority,
                    pattern.transition.min(TRANSITIONS.len() as u32 - 1),
                    normal,
                );
                if pattern.kind == world3d::SPECULAR_TYPE {
                    let [intensity, power] = pattern.specular;
                    layers.specular[i] = Vec4::new(intensity, power, 0.0, 0.0);
                }
            }
            None => {
                let rgb = viewer
                    .terrain_colors
                    .get(id)
                    .copied()
                    .unwrap_or_else(|| crate::render::terrain::palette_color(*id));
                let [r, g, b] = rgb.map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
                layers
                    .colors
                    .push(textures::solid(COLOR_SIZE, [r, g, b, 255]));
                layers.kinds[i].w = place_normal(&mut layers, flat_normal.clone(), 0);
            }
        }
    }
    layers.fields = map
        .patterns
        .iter()
        .map(|p| ids.iter().position(|id| id == p).unwrap_or(0) as u8)
        .collect();
    layers
}

/// The asset collections the 3D scene goes into.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SceneAssets<'w> {
    pub(crate) meshes: ResMut<'w, Assets<Mesh>>,
    pub(crate) images: ResMut<'w, Assets<Image>>,
    pub(crate) terrain_materials: ResMut<'w, Assets<TerrainMaterial>>,
    pub(crate) water_materials: ResMut<'w, Assets<WaterMaterial>>,
    pub(crate) standard_materials: ResMut<'w, Assets<StandardMaterial>>,
    /// GFX-VIEW-6.3: the sky cap's (the object material's sky family).
    pub(crate) object_materials: ResMut<'w, Assets<crate::render::models3d::ObjectMaterial>>,
    /// GFX-VIEW-2.3c: the day light buffer of the object materials.
    pub(crate) buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}

/// The entities of a map's 3D scene.
pub(crate) type SceneEntities = Or<(With<Terrain3d>, With<Object3d>)>;

/// Build the terrain, the water and the map-edge skirt of the map on screen
/// once the 3D view is shown for it.
#[allow(clippy::too_many_arguments)]
pub fn build_scene(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    mut view: ResMut<View3d>,
    mut assets: SceneAssets,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut placements: ResMut<Placements>,
    mut movers: ResMut<Movers>,
    mut animator: ResMut<crate::animation::animate3d::Animator>,
    mut building_anims: ResMut<BuildingAnims>,
    mut wares: ResMut<Wares3d>,
    old: Query<Entity, SceneEntities>,
    menu: Res<crate::menus::menu::MenuState>,
    mut daylight: ResMut<super::Daylight>,
    mut day_clock: ResMut<super::DayClock>,
    mut cameras: Query<&mut Camera, With<MapCamera3d>>,
    mut sun: Query<&mut Transform, With<DirectionalLight>>,
) {
    if !view.active || view.built == Some(viewer.loads) {
        return;
    }
    let (Some(map), Some(world)) = (&viewer.map, &viewer.world) else {
        return;
    };
    view.built = Some(viewer.loads);
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let terrain = world.terrain();
    let frame = terrain.frame();
    view.frame = frame;
    let (width, height) = (terrain.width, terrain.height);

    let world_texture = assets
        .images
        .add(world_image(fog_texture(world, viewer.fog_shown())));
    view.world_texture = Some(world_texture.clone());
    view.fog_tick = Some(world.tick());
    let rect = terrain3d::world_texture_rect(&frame, width, height);
    // GFX-VIEW-2.3, 2.3b: the map's day light; one light for every shader,
    // the shadow factor and the luminances in the fourth values; the clear
    // colour its background.
    let map_name = viewer
        .maps
        .get(viewer.current)
        .map(|path| crate::game::map_name(path))
        .unwrap_or_default();
    // GFX-VIEW-2.3c: the environment clock starts at hour 12.
    day_clock.table = super::day_table(viewer.game_root.as_deref(), &map_name);
    day_clock.hour = day_clock.start;
    let day = world3d::DayLight::at_clock(&day_clock.table, day_clock.hour).unwrap_or_default();
    daylight.0 = day;
    for mut camera in &mut cameras {
        camera.clear_color = ClearColorConfig::Custom(Color::linear_rgb(
            day.background[0],
            day.background[1],
            day.background[2],
        ));
    }
    for mut t in &mut sun {
        *t = Transform::IDENTITY.looking_to(Vec3::from(world3d::to_bevy(day.direction())), Vec3::Y);
    }
    // SET-4b: the game's start copies the shadow map's size and the shader
    // level; `advance_day` sets the map and follows the switch.
    view.game_shadows = GameShadows::at_start(&menu.settings);
    view.shadows_drawn = None;
    let light = receivers_light(
        day.shader(),
        view.game_shadows.now(&menu.settings).is_some(),
    );
    let (light_dir, light_color, ambient) = (
        Vec4::from_array(light.direction),
        Vec4::from_array(light.color),
        Vec4::from_array(light.ambient),
    );

    let scene_light = assets.buffers.add(crate::render::models3d::scene_light(
        light_dir,
        light_color,
        ambient,
    ));
    library.set_map(
        world_texture.clone(),
        crate::render::models3d::ObjectParams {
            tint: Vec4::ONE,
            world_rect: Vec4::from_array(rect),
            mode: Vec4::ZERO,
            extra: Vec4::ZERO,
            player: Vec4::ZERO,
        },
        scene_light,
        &mut assets.buffers,
    );
    *movers = Movers::default();
    // GFX-OBJ-3.2 (3a): the atlas is new with each game's view.
    library.new_game_atlas();
    animator.clear();
    *building_anims = BuildingAnims::default();
    *wares = Wares3d::default();
    *placements = Placements {
        lookup: viewer
            .rules
            .as_ref()
            .map(|rules| crate::world::scene3d::Lookup::new(rules, &library.tables))
            .unwrap_or_default(),
        // GFX-OBJ-1.4 (T32): list 1, 2, 3, each by id, the order a load
        // gives the view.
        doodads: crate::world::scene3d::load_doodads(&map.doodads),
        // GFX-OBJ-1.4 (T32): a map's or the original's save's deposit list
        // (its map part holds the deposits as saved, FMT-SAV-2.2); a save of
        // ours has the world's.
        load_deposits: (!viewer.own_save)
            .then(|| crate::world::scene3d::load_deposits(&map.deposits)),
        saved: viewer.saved_game,
        // GFX-OBJ-2.2a: the slots of this game's play-ahead (set with every
        // load), the ticks after it taken by the first update; else (5)
        // numbered afresh.
        house_slots: viewer
            .ahead_slots
            .as_ref()
            .filter(|s| {
                s.tick()
                    .zip(viewer.world.as_ref().map(|w| w.tick()))
                    .is_some_and(|(ahead, now)| ahead <= now)
            })
            .cloned()
            .unwrap_or_default(),
        ..default()
    };
    // What the game can show later, read now behind the loading picture
    // (`models3d::preload`): a first use during play put a model's meshes,
    // textures and materials on the graphics card in one frame.
    let mut tribes: Vec<u8> = world.players().iter().map(|&p| world.tribe(p)).collect();
    tribes.sort_unstable();
    tribes.dedup();
    let started = std::time::Instant::now();
    let preloaded = crate::render::models3d::preload::preload_game(
        &mut library,
        &placements.lookup,
        &tribes,
        &mut assets.meshes,
        &mut assets.images,
    );
    view.loads_before_play = preloaded.loads_before;
    info!(
        "preload: {} models and clips read in {:.0} ms",
        preloaded.read,
        started.elapsed().as_secs_f64() * 1000.0
    );

    let layers = pattern_layers(&viewer, map);
    let mut fields = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        layers.fields.clone(),
        TextureFormat::R8Uint,
        RenderAssetUsages::RENDER_WORLD,
    );
    fields.sampler = ImageSampler::nearest();
    let [fx, fz] = frame.field_xz(0, 0);
    // GFX-WATER-2.5: the caustics pass is drawn at shader detail 2 or more
    // (SET-4).
    let caustics_on = menu.settings.caustics();
    let caustics = library.read_tga(world3d::CAUSTICS_FILE).map(|t| {
        let mut image = Image::new(
            Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            t.rgba,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = repeating_sampler();
        image
    });
    if caustics.is_none() {
        eprintln!("{} not readable; no caustics", world3d::CAUSTICS_FILE);
    }
    let material = assets.terrain_materials.add(TerrainMaterial {
        params: TerrainParams {
            layers: layers.params,
            kinds: layers.kinds,
            specular: layers.specular,
            lattice: Vec4::new(fx, fz, width as f32, height as f32),
            world_rect: Vec4::from_array(rect),
            light_dir,
            light_color,
            ambient,
            caustics: Vec4::new(
                f32::from(u8::from(caustics.is_some() && caustics_on)),
                0.0,
                0.0,
                0.0,
            ),
        },
        caustics: assets.images.add(caustics.unwrap_or_else(|| {
            Image::new_fill(
                Extent3d::default(),
                TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            )
        })),
        colors: assets.images.add(array_image(&layers.colors, COLOR_SIZE)),
        normals: assets.images.add(normal_array_image(&layers, 0)),
        normals_256: assets.images.add(normal_array_image(&layers, 1)),
        normals_512: assets.images.add(normal_array_image(&layers, 2)),
        normals_1024: assets.images.add(normal_array_image(&layers, 3)),
        fields: assets.images.add(fields),
        world: world_texture.clone(),
        masks: assets.images.add(mask_image(viewer.game_root.as_deref())),
    });

    let (cells_x, cells_y) = terrain3d::cell_count(width, height);
    view.cells.clear();
    let drawn = terrain_heights(&viewer, terrain);
    for cy in 0..cells_y {
        for cx in 0..cells_x {
            let mesh = assets
                .meshes
                .add(cell_bevy_mesh(terrain3d::cell_mesh(&drawn, cx, cy)));
            view.cells
                .push((mesh.clone(), cell_heights(terrain, &frame, cx, cy)));
            // GFX-WATER-2.6b: the land is in the mirror image of a flat
            // view.
            commands.spawn((
                Terrain3d,
                Mesh3d(mesh),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                mirrored_layers(),
            ));
        }
    }

    // GFX-WATER-3, 4: the water surface, and the skirt around the map
    // (GFX-WATER-6.1) with its black ground.
    let mut wave = |path: &str| -> Handle<Image> {
        let image = viewer
            .game_root
            .as_ref()
            .and_then(|root| crate::extend::find(root, path).ok())
            .and_then(|path| {
                let data = crate::files::read(&path).ok()?;
                s2_assets::dds::load(&path.to_string_lossy(), &data).ok()
            })
            .map(|texture| repeating_image(&texture))
            .unwrap_or_else(|| {
                eprintln!("wave map {path} not readable; flat water");
                let mut image = Image::new_fill(
                    Extent3d::default(),
                    TextureDimension::D2,
                    &[128, 128, 255, 255],
                    TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::RENDER_WORLD,
                );
                image.sampler = repeating_sampler();
                image
            });
        assets.images.add(image)
    };
    let (wave0, wave1) = (wave(WAVE_MAPS[0]), wave(WAVE_MAPS[1]));
    // Hook W18: a project's shore foam, none by default.
    let (foam, foam_reach) =
        crate::render::view3d::materials::foam_params(crate::extend::render().shore_foam);
    let surface = WaterMaterial {
        params: WaterParams {
            water_color: vec4(WATER_COLOR, 1.0),
            wave_color: vec4(WAVE_COLOR, 1.0),
            // z: the surface passes test the shadow map (GFX-WATER-4.5a a).
            opacity: Vec4::new(OPACITY[0], OPACITY[1], 1.0, 0.0),
            rotation: Vec4::new(
                WAVE_ROTATION[0].to_radians(),
                WAVE_ROTATION[1].to_radians(),
                0.0,
                0.0,
            ),
            drift: Vec4::new(WAVE_DRIFT[0], WAVE_DRIFT[0], WAVE_DRIFT[1], WAVE_DRIFT[1]),
            world_rect: Vec4::from_array(rect),
            light_dir,
            light_color,
            ambient,
            // GFX-WATER-2.6: x the mirror image is bound (`place_mirror`).
            mirror: Vec4::new(
                0.0,
                REFLECTION_FACTOR,
                REFLECTION_NOISE[0],
                REFLECTION_NOISE[1],
            ),
            foam,
            foam_reach,
        },
        wave0,
        wave1,
        world: world_texture,
        // Bound by `place_mirror` in the same frame.
        mirror: Handle::default(),
    };
    // GFX-WATER-4.5a (a), 6.1: the map border's passes never test the
    // shadow map; its water is the surface's material with the test off.
    let border = assets.water_materials.add(skirt_material(surface.clone()));
    let water = assets.water_materials.add(surface);
    let (skirt, skirt_y) = terrain3d::skirt(terrain, width, height);
    let quads = terrain3d::water_quads(terrain, width, height);
    let ground = terrain3d::quad_mesh(&skirt, skirt_y);
    for (quads, material) in [(quads, water), (skirt, border)] {
        let (positions, indices) = terrain3d::quad_mesh(&quads, terrain3d::water_surface_y());
        if !positions.is_empty() {
            commands.spawn((
                Terrain3d,
                Mesh3d(assets.meshes.add(flat_mesh(positions, indices))),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                RenderLayers::layer(MAP_LAYER),
            ));
        }
    }
    let black = assets.standard_materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Terrain3d,
        Mesh3d(assets.meshes.add(flat_mesh(ground.0, ground.1))),
        MeshMaterial3d(black),
        Transform::IDENTITY,
        RenderLayers::layer(MAP_LAYER),
        bevy::light::NotShadowCaster,
    ));

    // GFX-VIEW-6.3, 6.3a: the cloud cap, drawn blended over the clear
    // colour and behind the ground; its material mixes the textures of the
    // day table's two skydome entries around the clock (T44: the clouds,
    // the stars or, from hour 22 to 23 and 0 to 1, both).
    let stars = library.read_tga(terrain3d::STARS_FILE).map(|t| {
        assets.images.add(Image::new(
            Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            t.rgba,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        ))
    });
    let clouds = library.stored_image("clouds.dds", &mut assets.images);
    for texture in [&clouds, &stars].into_iter().flatten() {
        if let Some(mut image) = assets.images.get_mut(texture) {
            image.sampler = repeating_sampler();
        }
    }
    if let Some(either) = clouds.clone().or(stars.clone()) {
        let textures = super::SkyTextures {
            clouds: clouds.unwrap_or(either.clone()),
            stars: stars.unwrap_or(either),
        };
        let (positions, uvs, colors, indices) = terrain3d::cloud_cap();
        let tangents = vec![[1.0, 0.0]; positions.len()];
        let normals = vec![[0.0, -1.0, 0.0]; positions.len()];
        let mut mesh = crate::render::models3d::object_mesh_with_normals(
            positions, normals, uvs, tangents, indices,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        let mesh = assets.meshes.add(mesh);
        if let Some(material) = library.sky_material(
            &mesh,
            textures.mix(&day),
            &mut assets.images,
            &mut assets.object_materials,
        ) {
            commands.spawn((
                Terrain3d,
                Sky3d,
                super::SkyCap(material.clone()),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                // GFX-WATER-2.6b, 2.6c: the cap is the mirror image's sky.
                mirrored_sky_layers(),
                bevy::light::NotShadowCaster,
            ));
        }
        commands.insert_resource(textures);
    }

    // GFX-VIEW-6.3: the sun and the moon, quads of their textures with
    // the cap's blend and no culling, placed each frame (`place_camera`).
    let sun = library.read_tga(world3d::SUN_FILE).map(|t| {
        assets.images.add(Image::new(
            Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            t.rgba,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        ))
    });
    let moon = library.stored_image(world3d::MOON_FILE, &mut assets.images);
    for (texture, is_moon) in [(sun, false), (moon, true)] {
        let Some(texture) = texture else {
            eprintln!(
                "GFX-VIEW-6.3: no texture for the {}",
                if is_moon { "moon" } else { "sun" }
            );
            continue;
        };
        commands.spawn((
            Terrain3d,
            SkySprite { moon: is_moon },
            Mesh3d(assets.meshes.add(Rectangle::new(1.0, 1.0))),
            MeshMaterial3d(assets.standard_materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                fog_enabled: false,
                ..default()
            })),
            Transform::IDENTITY,
            if is_moon && !day.moon_shown() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
            RenderLayers::layer(MAP_LAYER),
            bevy::light::NotShadowCaster,
        ));
    }

    // GFX-VIEW-3.6: every game start jumps to the local player's
    // headquarters (its field point; the target 18 units in front of it),
    // without one the target stays at (0, 10, -10). A map shown without a
    // game looks at its first start position the same way.
    let headquarters = match &viewer.world {
        Some(world) => headquarters_of(world, crate::game::PLAYER),
        None => map.info.start_positions.first().map(|s| (s.x, s.y)),
    };
    let mut rig = CameraRig::start(headquarters.map(|at| frame.field_xz(at.0, at.1)));
    if let Some((x, y, _)) = viewer.focus {
        let [fx, fz] = frame.field_xz(x, y);
        rig = CameraRig::new(fx, fz);
    }
    rig.zoom = view.start_zoom;
    view.rig = rig;
    // GFX-VIEW-3.7: a game's camera starts with the follow pose behind.
    view.follow = None;
    view.follow_eye = None;
    view.follow_pose = world3d::FollowPose::Behind;
}

/// Rebuild the terrain cells whose heights changed (a levelled site), and
/// redraw the fog of war as the player explores.
pub fn update_scene(
    viewer: Res<crate::game::Viewer>,
    mut view: ResMut<View3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut spans: Option<ResMut<crate::tools::bench::FrameSpans>>,
    mut placements: ResMut<Placements>,
) {
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let Some(world) = &viewer.world else { return };
    let terrain = world.terrain();
    let frame = view.frame;
    // F55: the whole layer first: most changes of the world move no ground.
    if viewer.world_changed
        && (terrain.samples.is_empty() || terrain.samples != view.height_samples)
    {
        let timer = std::time::Instant::now();
        view.height_samples.clone_from(&terrain.samples);
        let (cells_x, _) = terrain3d::cell_count(terrain.width, terrain.height);
        let drawn = terrain_heights(&viewer, terrain);
        for index in 0..view.cells.len() {
            let (cx, cy) = (index as u32 % cells_x, index as u32 / cells_x);
            let heights = cell_heights(terrain, &frame, cx, cy);
            if heights == view.cells[index].1 {
                continue;
            }
            let handle = view.cells[index].0.clone();
            if let Some(mut mesh) = meshes.get_mut(&handle) {
                *mesh = cell_bevy_mesh(terrain3d::cell_mesh(&drawn, cx, cy));
            }
            view.cells[index].1 = heights;
            placements.height_cells.push(index);
        }
        if let Some(spans) = spans.as_mut() {
            spans.add("terrain cells", timer);
        }
    }
    // The exploration changes with the military buildings; ten ticks is a
    // second of game time.
    const FOG_TICKS: u64 = 10;
    let due = view
        .fog_tick
        .is_none_or(|t| world.tick() >= t + FOG_TICKS || world.tick() < t);
    if due {
        let timer = std::time::Instant::now();
        view.fog_tick = Some(world.tick());
        if let Some(handle) = view.world_texture.clone() {
            let texels = fog_texture(world, viewer.fog_shown()).texels;
            if let Some(mut image) = images.get_mut(&handle) {
                if image.data.as_deref() != Some(texels.as_slice()) {
                    image.data = Some(texels);
                }
            }
        }
        if let Some(spans) = spans.as_mut() {
            spans.add("fog", timer);
        }
    }
}

/// GFX-VIEW-6.3: a sun or moon quad's transform and which it is.
pub(crate) type SkySpriteParts = (&'static mut Transform, &'static SkySprite);
