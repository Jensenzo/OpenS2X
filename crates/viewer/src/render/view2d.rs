//! The 2D map view: terrain, objects and moving settlers as shapes.

use crate::game::{bevy_mesh, DepositEntity, MovingEntity, Viewer, WorldEntity};
use bevy::prelude::*;

/// Ticks between two redraws of growing trees.
pub(crate) const GROWTH_REDRAW_TICKS: u64 = 20;

/// Redraw trees and stone piles when one appeared, changed or went, and
/// every few ticks while trees grow; not while the 3D view is on (F51).
pub(crate) fn draw_deposits(
    mut commands: Commands,
    viewer: Res<Viewer>,
    view: Res<crate::render::view3d::View3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    old: Query<Entity, With<DepositEntity>>,
    mut drawn: Local<Option<(u64, u64, usize)>>,
) {
    if view.active {
        return;
    }
    let Some(world) = &viewer.world else { return };
    let key = (
        world.deposit_version(),
        world.tick() / GROWTH_REDRAW_TICKS,
        viewer.current,
    );
    if *drawn == Some(key) {
        return;
    }
    *drawn = Some(key);
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let data = crate::render::scene::deposit_mesh(world);
    if data.positions.is_empty() {
        return;
    }
    commands.spawn((
        DepositEntity,
        Mesh2d(meshes.add(bevy_mesh(data))),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        Transform::from_xyz(0.0, 0.0, crate::render::scene::DEPOSIT_LAYER),
    ));
}

/// Redraw carriers and wares after the simulation advanced; while the 3D
/// view is on, once it is off again (F51).
pub(crate) fn draw_moving(
    mut commands: Commands,
    mut viewer: ResMut<Viewer>,
    view: Res<crate::render::view3d::View3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    old: Query<Entity, With<MovingEntity>>,
    mut material: Local<Option<Handle<ColorMaterial>>>,
) {
    if !viewer.moved || view.active {
        return;
    }
    viewer.moved = false;
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let Some(world) = &viewer.world else { return };
    let mut data = crate::render::scene::moving_mesh(world);
    if viewer.borders {
        data.append(crate::render::scene::border_mesh(world, true));
    }
    if data.positions.is_empty() {
        return;
    }
    let material = material
        .get_or_insert_with(|| materials.add(ColorMaterial::default()))
        .clone();
    commands.spawn((
        MovingEntity,
        Mesh2d(meshes.add(bevy_mesh(data))),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, 0.0, crate::render::scene::MOVING_LAYER),
    ));
}

/// Rebuild the meshes of the overlay and the placed objects after a change.
pub(crate) fn draw_world(
    mut commands: Commands,
    mut viewer: ResMut<Viewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    old: Query<Entity, With<WorldEntity>>,
    view: Res<crate::render::view3d::View3d>,
    mut stale: Local<bool>,
) {
    // The 3D view has read the change (`update_scene`); the 2D layers are
    // drawn once the 2D view is on again (F51).
    if view.active {
        *stale |= std::mem::take(&mut viewer.world_changed);
        return;
    }
    if !viewer.world_changed && !std::mem::take(&mut *stale) {
        return;
    }
    viewer.world_changed = false;
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let Some(world) = &viewer.world else { return };
    let material = materials.add(ColorMaterial::default());
    let mut layers = vec![(
        crate::render::scene::object_mesh(world),
        crate::render::scene::OBJECT_LAYER,
    )];
    if viewer.borders {
        layers.push((
            crate::render::scene::border_mesh(world, false),
            crate::render::scene::BORDER_LAYER,
        ));
    }
    if viewer.territory {
        layers.push((
            crate::render::scene::territory_mesh(world),
            crate::render::scene::TERRITORY_LAYER,
        ));
    }
    if viewer.overlay {
        layers.push((
            crate::render::scene::overlay_mesh(world),
            crate::render::scene::OVERLAY_LAYER,
        ));
    }
    // An empty mesh (nothing placed yet) is not drawn at all.
    for (data, layer) in layers
        .into_iter()
        .filter(|(data, _)| !data.positions.is_empty())
    {
        commands.spawn((
            WorldEntity,
            Mesh2d(meshes.add(bevy_mesh(data))),
            MeshMaterial2d(material.clone()),
            Transform::from_xyz(0.0, 0.0, layer),
        ));
    }
}
