//! Marks drawn over the 3D map: build-help icons, status icons, cursor sprites (GFX-VIEW-6, GFX-OBJ-8.3, UI-GAME-13).

use super::*;

/// Marks the entity of the build-help and cursor marks.
#[derive(Component)]
pub struct Marks3d;

/// What the marks were last drawn for: the map, the tick while the
/// overlay is on (placement classes change as the world ticks), the overlay
/// switch, the cursor field, the road's first flag and the camera.
pub(crate) type MarksKey = (
    u64,
    u64,
    bool,
    Option<(u32, u32)>,
    Option<(u32, u32)>,
    [i32; 4],
    [i32; 3],
);

/// GFX-VIEW-6.1: the build-help icons as view-aligned billboards from the
/// icon atlas (unlit, blended, depth-tested, no depth write), redrawn when the world,
/// the cursor or the camera moves; and the first flag of a road being
/// built.
#[allow(clippy::too_many_arguments)]
pub fn draw_marks(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    hover: Res<crate::input::Hover>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Single<&Transform, With<MapCamera3d>>,
    old: Query<Entity, With<Marks3d>>,
    mut drawn: Local<Option<MarksKey>>,
) {
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let Some(world) = &viewer.world else { return };
    let cursor = hover.0.map(|f| (f.x, f.y));
    let rotation = camera.rotation;
    let eye = camera.translation;
    let key = (
        viewer.loads,
        if viewer.overlay { world.tick() } else { 0 },
        viewer.overlay,
        cursor,
        viewer.road_start,
        rotation.to_array().map(|v| (v * 1000.0).round() as i32),
        eye.to_array().map(|v| (v * 10.0).round() as i32),
    );
    if *drawn == Some(key) {
        return;
    }
    *drawn = Some(key);
    for entity in &old {
        commands.entity(entity).despawn();
    }
    // UI-GAME-13: road building shows the cursor icons only; the yellow
    // ring on the start flag is the debug viewer's aid, not the game's.
    let road_start = viewer.road_start.filter(|_| viewer.debug);
    let marks = crate::world::scene3d::marks(world, &view.frame, road_start);
    if !marks.positions.is_empty() {
        let normals = vec![[0.0, 1.0, 0.0]; marks.positions.len()];
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, marks.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, marks.colors)
        .with_inserted_indices(Indices::U32(marks.indices));
        commands.spawn((
            Marks3d,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..default()
            })),
            Transform::IDENTITY,
            RenderLayers::layer(MAP_LAYER),
            bevy::light::NotShadowCaster,
        ));
    }
    if !viewer.overlay {
        return;
    }
    let icons = crate::world::scene3d::build_icons(world, &view.frame, cursor);
    let Some(atlas) = library.stored_image("infomap.dds", &mut images) else {
        return;
    };
    let right = (rotation * Vec3::X).to_array();
    let up = (rotation * Vec3::Y).to_array();
    let forward = (rotation * Vec3::NEG_Z).to_array();
    let (positions, uvs, indices) = crate::world::scene3d::billboards(&icons, right, up, forward);
    if positions.is_empty() {
        return;
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices));
    commands.spawn((
        Marks3d,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(atlas),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::IDENTITY,
        RenderLayers::layer(MAP_LAYER),
        bevy::light::NotShadowCaster,
    ));
}

/// Marks a status icon over a building (GFX-OBJ-8.3).
#[derive(Component)]
pub struct StatusIcon3d;

/// GFX-OBJ-8.3: the idle timers, the icons shown and what draws them.
#[derive(Resource, Default)]
pub struct StatusIcons {
    pub(crate) loads: u64,
    pub(crate) timers:
        std::collections::HashMap<(s2_sim::grid::Point, u32), crate::world::decor::IdleTimer>,
    pub(crate) shown: std::collections::HashMap<(s2_sim::grid::Point, u32), ShownIcon>,
    pub(crate) boxes: std::collections::HashMap<String, Option<crate::world::decor::Box3>>,
    effects:
        std::collections::HashMap<crate::world::decor::StatusIcon, Option<s2_assets::pfx::Effect>>,
    pub(crate) texture: Option<Option<Handle<Image>>>,
}

/// GFX-OBJ-8.3a: a status effect on screen: its icon, when it started
/// (game seconds, TOOL-1.2) and its pool of quads with their materials.
pub(crate) struct ShownIcon {
    pub(crate) icon: crate::world::decor::StatusIcon,
    pub(crate) started: f32,
    pub(crate) quads: Vec<(Entity, Handle<StandardMaterial>)>,
}

/// GFX-OBJ-8.3: the status icons of the local player's finished buildings:
/// `Sleep` after 10 s below 25 % productivity, the stop gear while
/// production is off, the soldier and coin icons of military buildings.
/// GFX-OBJ-8.3a: each is its `.pfx` effect started at the house box's
/// centre when the state begins: a pool of particles, each a camera-facing
/// quad of its `particles_texture.tga` rectangle (the facing of the
/// "Decal" particle was not traced), moved, sized and faded by the effect
/// on the animation clock (game time, TOOL-1.2).
#[allow(clippy::too_many_arguments)]
pub fn status_icons(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    placements: Res<Placements>,
    mut icons: ResMut<StatusIcons>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Single<&Transform, (With<MapCamera3d>, Without<StatusIcon3d>)>,
    mut quads: Query<&mut Transform, With<StatusIcon3d>>,
    clock: Res<crate::game::GameClock>,
) {
    use crate::world::decor::{status_icon, StatusOf};
    let Some(world) = viewer.world.as_ref().filter(|_| view.active) else {
        return;
    };
    if icons.loads != viewer.loads {
        for (_, shown) in icons.shown.drain() {
            for (entity, material) in shown.quads {
                commands.entity(entity).despawn();
                materials.remove(&material);
            }
        }
        icons.timers.clear();
        icons.loads = viewer.loads;
    }
    // GFX-OBJ-8.3's 10 s idle timer on the game step (TOOL-trace
    // TRACE-5.3, Q-226 f).
    let dt = clock.step;
    let player = world.local_player();
    let mut wanted = std::collections::HashMap::new();
    for building in world.buildings() {
        if building.owner != player || building.under_construction {
            continue;
        }
        let Some(kind) = world.building_type(building.building_type) else {
            continue;
        };
        let key = (building.at, building.serial);
        let state = if let Some(m) = world
            .military_at(building.at)
            .filter(|_| kind.soldier_limit >= 1 && !kind.warehouse)
        {
            StatusOf::Military {
                move_out: m.move_out,
                coins: m.coin_switch && m.accumulator() >= 0.0,
            }
        } else if kind.worker.is_some() && !kind.warehouse && kind.soldier_limit == 0 {
            // GFX-OBJ-8.3 (play test 11): the timer runs only while
            // production is on.
            let productivity = building.productivity as f32 / 100.0;
            let timer = icons.timers.entry(key).or_default();
            let idle = if building.production.enabled {
                timer.update(productivity, dt)
            } else {
                false
            };
            StatusOf::Production {
                enabled: building.production.enabled,
                idle,
            }
        } else {
            continue;
        };
        if let Some(icon) = status_icon(state) {
            wanted.insert(key, (icon, building.building_type, building.owner));
        }
    }
    icons
        .timers
        .retain(|k, _| world.building_at(k.0).is_some_and(|b| b.serial == k.1));
    let gone: Vec<_> = icons
        .shown
        .iter()
        .filter(|(k, shown)| wanted.get(*k).map(|w| w.0) != Some(shown.icon))
        .map(|(k, _)| *k)
        .collect();
    for key in gone {
        if let Some(shown) = icons.shown.remove(&key) {
            for (entity, material) in shown.quads {
                commands.entity(entity).despawn();
                materials.remove(&material);
            }
        }
    }
    // TOOL-1.2: the effects run on the animation clock, game time; the
    // idle timer above counts real frame seconds (GFX-OBJ-8.3, Q-226).
    let now = clock.seconds;
    for (key, (icon, building_type, owner)) in wanted {
        let Some(gfx) = placements.lookup.buildings.get(&building_type) else {
            continue;
        };
        let entry = crate::render::models3d::building_entry(world.tribe(owner), gfx);
        let Some(entry) = library.tables.building(&entry).cloned() else {
            continue;
        };
        let Some(mesh) = entry.mesh.clone() else {
            continue;
        };
        let house = match icons.boxes.get(&mesh) {
            Some(b) => *b,
            None => {
                let b = library.house_box(&mesh, &mut meshes, &mut images);
                icons.boxes.insert(mesh, b);
                b
            }
        };
        let Some(house) = house else { continue };
        let centre = [0, 1, 2].map(|k| (house[0][k] + house[1][k]) / 2.0);
        let base = world3d::building_position(world.terrain(), key.0 .0, key.0 .1);
        // The effect starts at the box's centre, moved with the building.
        let start = crate::world::decor::building_to_world(entry.heading as f32, base, centre);
        let effect = icons
            .effects
            .entry(icon)
            .or_insert_with(|| {
                library.read_pfx(&format!("data/scripts/effects/{}.pfx", icon.effect()))
            })
            .clone();
        let Some(effect) = effect else { continue };
        if !icons.shown.contains_key(&key) {
            if icons.texture.is_none() {
                icons.texture = Some(particles_texture(&mut library, &mut images));
            }
            let Some(Some(texture)) = icons.texture.clone() else {
                continue;
            };
            let quad = meshes.add(particle_quad(effect.rect));
            let quads = (0..effect.count.max(0))
                .map(|_| {
                    let material = materials.add(StandardMaterial {
                        base_color_texture: Some(texture.clone()),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        cull_mode: None,
                        ..default()
                    });
                    let entity = commands
                        .spawn((
                            StatusIcon3d,
                            Mesh3d(quad.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::from_scale(Vec3::ZERO),
                            RenderLayers::layer(MAP_LAYER),
                            bevy::light::NotShadowCaster,
                        ))
                        .id();
                    (entity, material)
                })
                .collect();
            icons.shown.insert(
                key,
                ShownIcon {
                    icon,
                    started: now,
                    quads,
                },
            );
        }
        let Some(shown) = icons.shown.get(&key) else {
            continue;
        };
        let seed = u64::from(key.0 .0) << 32 | u64::from(key.0 .1) << 8 | u64::from(key.1);
        let pool = crate::world::decor::particles(&effect, seed, now - shown.started);
        for ((entity, material), particle) in shown.quads.iter().zip(pool) {
            let transform = match particle {
                Some(p) => {
                    let at = [0, 1, 2].map(|k| start[k] + p.at[k]);
                    if let Some(mut m) = materials.get_mut(material) {
                        m.base_color = Color::srgba(1.0, 1.0, 1.0, p.alpha);
                    }
                    Transform::from_translation(Vec3::from(world3d::to_bevy(at)))
                        .with_rotation(camera.rotation)
                        .with_scale(Vec3::splat(p.size))
                }
                None => Transform::from_scale(Vec3::ZERO),
            };
            if let Ok(mut t) = quads.get_mut(*entity) {
                *t = transform;
            }
        }
    }
}

/// GFX-OBJ-8.3a: `particles_texture.tga`, sampled as stored (the map
/// camera's values).
fn particles_texture(
    library: &mut crate::render::models3d::ModelLibrary,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    library.read_tga("data/gfx/particles_texture.tga").map(|t| {
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
        image.sampler = ImageSampler::linear();
        images.add(image)
    })
}

/// GFX-OBJ-8.3a: a particle's quad, a unit quad of its texture
/// rectangle's shape (the two-icon rectangle is wider than high).
fn particle_quad([u0, v0, u1, v1]: [f32; 4]) -> Mesh {
    let (w, h) = ((u1 - u0).abs(), (v1 - v0).abs());
    let (w, h) = (0.5 * w / w.max(h), 0.5 * h / w.max(h));
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-w, -h, 0.0], [w, -h, 0.0], [w, h, 0.0], [-w, h, 0.0]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

/// Marks a collapse's dust particle.
#[derive(Component)]
pub struct Dust3d;

/// GFX-OBJ-8.7 (3): one dust cloud on screen: the collapsing building's
/// serial (its particles' seed), where it started (game frame), when (game
/// seconds, TOOL-1.2) and its pool of quads.
type DustCloud = (u32, [f32; 3], f32, Vec<(Entity, Handle<StandardMaterial>)>);

/// GFX-OBJ-8.7 (3): the collapses whose dust has started, and the dust
/// clouds on screen.
#[derive(Resource, Default)]
pub struct CollapseDust {
    loads: u64,
    effect: Option<Option<s2_assets::pfx::Effect>>,
    texture: Option<Option<Handle<Image>>>,
    quad: Option<Handle<Mesh>>,
    started: std::collections::HashSet<u32>,
    shown: Vec<DustCloud>,
}

/// GFX-OBJ-8.7 (3): when a finished building starts to collapse, the
/// `DemolitionDust` effect starts once at the house's position (its origin
/// on the ground): its pool of camera-facing quads of the effect's
/// rectangle of `particles_texture.tga`, each the particle's colour and
/// alpha, blended, writing no depth, run on the animation clock as the
/// status icons' (8.3a). The cloud goes when its last particle has died.
#[allow(clippy::too_many_arguments)]
pub fn collapse_dust(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    placements: Res<Placements>,
    mut dust: ResMut<CollapseDust>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Single<&Transform, (With<MapCamera3d>, Without<Dust3d>)>,
    mut quads: Query<&mut Transform, With<Dust3d>>,
    clock: Res<crate::game::GameClock>,
) {
    let Some(world) = viewer.world.as_ref().filter(|_| view.active) else {
        return;
    };
    let dust = &mut *dust;
    if dust.loads != viewer.loads {
        for (_, _, _, pool) in dust.shown.drain(..) {
            for (entity, material) in pool {
                commands.entity(entity).despawn();
                materials.remove(&material);
            }
        }
        dust.started.clear();
        dust.loads = viewer.loads;
    }
    let now = clock.seconds;
    let collapsing: Vec<(u32, s2_sim::grid::Point)> = placements
        .house_slots
        .collapsing()
        .map(|(house, _, _)| (house.serial, house.at))
        .collect();
    dust.started
        .retain(|serial| collapsing.iter().any(|(s, _)| s == serial));
    for (serial, at) in collapsing {
        if !dust.started.insert(serial) {
            continue;
        }
        let effect = dust
            .effect
            .get_or_insert_with(|| library.read_pfx("data/scripts/effects/DemolitionDust.pfx"))
            .clone();
        let texture = dust
            .texture
            .get_or_insert_with(|| particles_texture(&mut library, &mut images))
            .clone();
        let (Some(effect), Some(texture)) = (effect, texture) else {
            continue;
        };
        let quad = dust
            .quad
            .get_or_insert_with(|| meshes.add(particle_quad(effect.rect)))
            .clone();
        let pool = (0..effect.count.max(0))
            .map(|_| {
                let material = materials.add(StandardMaterial {
                    base_color_texture: Some(texture.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    ..default()
                });
                let entity = commands
                    .spawn((
                        Dust3d,
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_scale(Vec3::ZERO),
                        RenderLayers::layer(MAP_LAYER),
                        bevy::light::NotShadowCaster,
                    ))
                    .id();
                (entity, material)
            })
            .collect();
        let start = world3d::building_position(world.terrain(), at.0, at.1);
        dust.shown.push((serial, start, now, pool));
    }
    let Some(Some(effect)) = dust.effect.clone() else {
        return;
    };
    let mut done = Vec::new();
    for (i, (serial, start, started, pool)) in dust.shown.iter().enumerate() {
        let elapsed = now - started;
        let particles = crate::world::decor::particles(&effect, u64::from(*serial), elapsed);
        if elapsed > 1.0 && particles.iter().all(Option::is_none) {
            done.push(i);
            continue;
        }
        for ((entity, material), particle) in pool.iter().zip(particles) {
            let transform = match particle {
                Some(p) => {
                    let at = [0, 1, 2].map(|k| start[k] + p.at[k]);
                    if let Some(mut m) = materials.get_mut(material) {
                        // The stored values of the map camera: the
                        // particle's colour as it is (GFX-VIEW-2.6 c1).
                        m.base_color =
                            Color::linear_rgba(p.colour[0], p.colour[1], p.colour[2], p.alpha);
                    }
                    Transform::from_translation(Vec3::from(world3d::to_bevy(at)))
                        .with_rotation(camera.rotation)
                        .with_scale(Vec3::splat(p.size))
                }
                None => Transform::from_scale(Vec3::ZERO),
            };
            if let Ok(mut t) = quads.get_mut(*entity) {
                *t = transform;
            }
        }
    }
    for i in done.into_iter().rev() {
        let (_, _, _, pool) = dust.shown.remove(i);
        for (entity, material) in pool {
            commands.entity(entity).despawn();
            materials.remove(&material);
        }
    }
}

/// A cursor sprite on screen.
#[derive(Component)]
pub struct CursorSprite;

/// UI-GAME-13, GFX-VIEW-6.2: the cursor's sprites from `uiElements3` with
/// their top-left corner 22 pixels left of and above the field's projected
/// position: outside road building the field's icon (13.1), while no
/// context dialog is open and the pointer is not over the interface; while
/// a road or waterway is built the road icon (13.2) with the ring and the
/// neighbour markers.
#[allow(clippy::too_many_arguments)]
pub fn cursor_sprites(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    hover: Res<crate::input::Hover>,
    ui: Res<crate::ui::game::GameUi>,
    frame: Res<crate::ui::render::UiFrame>,
    events: Res<crate::ui::render::UiEvents>,
    window: Single<&Window>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<(&Camera, &Transform), With<MapCamera3d>>,
    old: Query<Entity, With<CursorSprite>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let (Some(world), Some(cursor)) = (&viewer.world, hover.0) else {
        return;
    };
    let Some(sheet) = library.raw_image("uielements3.dds", &mut images) else {
        return;
    };
    let cursor = (cursor.x, cursor.y);
    let player = crate::game::PLAYER;
    // This frame's camera (place_camera), as the hover takes it; the global
    // transform would still be the last frame's.
    let (camera, transform) = *camera;
    let transform = &GlobalTransform::from(*transform);
    let mut sprites: Vec<(s2_sim::grid::Point, [u32; 4])> = Vec::new();
    match viewer.road_start {
        Some(from) => {
            for (field, [sx, sy]) in
                crate::world::scene3d::cursor_sprites(world, player, cursor, ui.waterway)
            {
                sprites.push((field, [sx, sy, 32, 32]));
            }
            sprites.push((
                cursor,
                crate::world::scene3d::road_icon(world, player, from, cursor, ui.waterway),
            ));
        }
        None => {
            let over_interface = events.over_ui
                || window
                    .cursor_position()
                    .is_some_and(|p| frame.covers(p.to_array()));
            if ui.context.is_some() || over_interface {
                return;
            }
            sprites.push((
                cursor,
                crate::world::scene3d::field_icon(world, player, cursor),
            ));
        }
    }
    for (field, [sx, sy, w, h]) in sprites {
        let [x, z] = view.frame.field_xz(field.0, field.1);
        let y = world3d::ground_height(world.terrain(), x, z);
        let at = Vec3::from(world3d::to_bevy([x, y, z]));
        let Ok(p) = camera.world_to_viewport(transform, at) else {
            continue;
        };
        commands.spawn((
            CursorSprite,
            ImageNode {
                image: sheet.clone(),
                rect: Some(Rect::new(
                    sx as f32,
                    sy as f32,
                    (sx + w) as f32,
                    (sy + h) as f32,
                )),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(p.x - 22.0),
                top: Val::Px(p.y - 22.0),
                width: Val::Px(w as f32),
                height: Val::Px(h as f32),
                ..default()
            },
        ));
    }
}
