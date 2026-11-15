//! Buildings, deposits and decorations as 3D objects, kept in step with the world.

use super::*;

/// Marks the models standing on the map, and the road strips.
#[derive(Component)]
pub struct Object3d;

/// The models on the map by their key, with what they were placed as, and
/// what was drawn last.
#[derive(Resource, Default)]
pub struct Placements {
    pub(crate) lookup: crate::world::scene3d::Lookup,
    /// The map's doodads: type and sample (FMT-S2M-6.1), list 1, 2, 3, each
    /// by id (GFX-OBJ-1.4, T32).
    pub(crate) doodads: Vec<(u32, (u32, u32))>,
    /// GFX-OBJ-1.4 (T32): the file's deposit list (field, kind) in its
    /// order, `None` for a save of ours.
    pub(crate) load_deposits: Option<Vec<(s2_sim::grid::Point, u32)>>,
    /// GFX-OBJ-1.4 (T32): the game is a save (its flags were in the file).
    pub(crate) saved: bool,
    pub(crate) placed: std::collections::HashMap<
        (crate::world::scene3d::Kind, s2_sim::grid::Point, u32),
        (Entity, crate::world::scene3d::Placed),
    >,
    /// The deposits' version, the growth step, the borders switch, the
    /// eighth of a tick while a tree falls and the decorations' signature.
    pub(crate) drawn: Option<(u64, u64, bool, u64, u64)>,
    /// GFX-OBJ-8.1a: house boxes by building model file.
    pub(crate) house_boxes: std::collections::HashMap<String, crate::world::decor::Box3>,
    /// F55: the meshes of the street pass by part and square, with what each
    /// shows (`street_parts`); a part with no vertices has no entity.
    pub(crate) road_parts:
        std::collections::HashMap<(u8, crate::world::scene3d::RoadChunk), (u64, Option<Entity>)>,
    /// What the road meshes were last looked at for (`road_content`).
    pub(crate) road_signature: Option<u64>,
    /// Every object of the map placed again this many times (`--bench`).
    pub(crate) full_placements: u64,
    /// F55: what the doodads were last placed for, and where each stands.
    pub(crate) doodad_track: Option<DoodadTrack>,
    /// F55: the terrain cells whose heights changed since the objects were
    /// last placed (`update_scene`).
    pub(crate) height_cells: Vec<usize>,
    /// GFX-OBJ-6.2 (Q-221): the flags whose crossing has moved to crossing
    /// texture 1.
    pub(crate) donkey_crossings: std::collections::HashSet<s2_sim::grid::Point>,
    /// GFX-OBJ-6.2a (d) (T47): where each road stops short, decided when it
    /// was made.
    pub(crate) road_ends: crate::world::scene3d::RoadEnds,
    /// GFX-ANIM-3.8a: the axis each felled tree turns about, and the
    /// angle it was last drawn at while falling.
    pub(crate) fall_axes: std::collections::HashMap<s2_sim::grid::Point, f32>,
    pub(crate) fall_last: std::collections::HashMap<s2_sim::grid::Point, f32>,
    /// GFX-ANIM-3.8c: the height y0 and box height h of each deposit
    /// being removed, taken when its removal was first seen.
    pub(crate) sinks: std::collections::HashMap<s2_sim::grid::Point, (f32, f32)>,
    /// GFX-OBJ-1.4: each landscape-element instance's yaw and size, drawn
    /// once when it first appears.
    pub(crate) poses: crate::world::scene3d::Poses,
    /// GFX-OBJ-2.2a: the house renderer's slot table (a tool's play-ahead
    /// hands over its own).
    pub(crate) house_slots: crate::world::scene3d::HouseSlots,
    /// GFX-OBJ-8.7 (2): each collapsing house's model height H, taken when
    /// it was placed.
    pub(crate) collapse_heights:
        std::collections::HashMap<(crate::world::scene3d::Kind, s2_sim::grid::Point, u32), f32>,
}

impl Placements {
    /// A building (or its construction site) stands drawn on `at`.
    pub fn has_building(&self, at: s2_sim::grid::Point) -> bool {
        self.placed
            .keys()
            .any(|(kind, field, _)| *kind == crate::world::scene3d::Kind::Building && *field == at)
    }
}

/// GFX-VIEW-4: the Bevy transform of a placed model.
pub(crate) fn placed_transform(placed: &crate::world::scene3d::Placed) -> Transform {
    let axis = world3d::to_bevy([placed.tilt_axis.cos(), 0.0, placed.tilt_axis.sin()]);
    Transform::from_translation(Vec3::from(world3d::to_bevy(placed.at)))
        .with_rotation(
            Quat::from_axis_angle(Vec3::from(axis), placed.tilt)
                * Quat::from_rotation_y(world3d::bevy_yaw(placed.heading)),
        )
        .with_scale(Vec3::from(placed.scale))
}

/// GFX-ANIM-3.8c: the height of a model's box `b` (game frame) under the
/// placement's rotation and scale: the eight corners turned, the
/// axis-aligned box taken again.
pub(crate) fn box_height(
    b: crate::world::decor::Box3,
    placed: &crate::world::scene3d::Placed,
) -> f32 {
    let t = placed_transform(placed);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for corner in 0..8 {
        let p = [0, 1, 2].map(|k| b[(corner >> k) & 1][k]);
        let y = (t.rotation * (t.scale * Vec3::from(world3d::to_bevy(p)))).y;
        lo = lo.min(y);
        hi = hi.max(y);
    }
    hi - lo
}

/// GFX-OBJ-8.1, 8.2, 8.4: what the building decorations depend on: the
/// soldiers inside, the border flag's row, the occupation and the
/// upgrades' progress.
pub(crate) fn decor_signature(world: &s2_sim::World) -> u64 {
    let mut h: u64 = 0;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x0100_0000_01b3);
    for (at, m) in world.military_buildings() {
        mix(u64::from(at.0) << 32 | u64::from(at.1));
        mix(world.present_soldiers(at) as u64);
        mix(u64::from(crate::world::decor::border_flag_row(
            m.enemy_distance,
        )));
        mix(u64::from(m.applied));
        if world.upgrading(at).is_some() {
            let u = world.site(at).map_or(0.0, |s| s.progress().1);
            mix(u64::from(u.to_bits()));
        }
    }
    h
}

/// What is on the map and no longer wanted: everything placed but the doodads
/// that is not in `keep` (F55: the doodads go by [`DoodadTrack`]).
pub(crate) fn gone_objects<'a, K>(
    placed: impl Iterator<Item = &'a (crate::world::scene3d::Kind, K, u32)>,
    keep: &std::collections::HashSet<(crate::world::scene3d::Kind, K, u32)>,
) -> Vec<(crate::world::scene3d::Kind, K, u32)>
where
    K: Copy + Eq + std::hash::Hash + 'a,
{
    placed
        .filter(|key| key.0 != crate::world::scene3d::Kind::Doodad)
        .filter(|key| !keep.contains(*key))
        .copied()
        .collect()
}

/// F55: a map holds thousands of doodads (18 000 of 22 000 objects on a late
/// game's map) and few change at a time: those a road, flag or building
/// removed (GFX-OBJ-1.4a), on a field a site cleared (ECO-PLACE-5.3), or on
/// ground that moved. This says which, so a road built does not place the map
/// again.
#[derive(Debug, Default)]
pub(crate) struct DoodadTrack {
    removed: std::collections::BTreeMap<s2_sim::grid::Point, u8>,
    /// The removals were looked at once (the first time everything is
    /// placed anyway).
    primed: bool,
    cleared: std::collections::BTreeSet<s2_sim::grid::Point>,
    at_field: std::collections::HashMap<s2_sim::grid::Point, Vec<u32>>,
    in_cell: std::collections::HashMap<(u32, u32), Vec<u32>>,
}

impl DoodadTrack {
    /// Where each of `doodads` stands on a map of `width` x `height` fields.
    pub(crate) fn new(doodads: &[(u32, (u32, u32))], width: u32, height: u32) -> Self {
        let mut track = Self::default();
        for (i, &(_, sample)) in doodads.iter().enumerate() {
            let i = i as u32;
            let field = crate::world::scene3d::decoration_field(sample, width, height);
            track.at_field.entry(field).or_default().push(i);
            let cell = (
                sample.0 / (4 * terrain3d::CELL_FIELDS),
                sample.1 / (4 * terrain3d::CELL_FIELDS),
            );
            track.in_cell.entry(cell).or_default().push(i);
        }
        track
    }

    /// The doodads that may have changed since the last call: on a field
    /// whose removals (GFX-OBJ-1.4a) grew, on a field newly cleared, in or
    /// next to a terrain cell `(column, row)` whose heights changed; each
    /// once, in order.
    pub(crate) fn changed(
        &mut self,
        removed: &std::collections::BTreeMap<s2_sim::grid::Point, u8>,
        cleared: &std::collections::BTreeSet<s2_sim::grid::Point>,
        height_cells: &[(u32, u32)],
    ) -> Vec<u32> {
        let mut out = Vec::new();
        if *removed != self.removed {
            for (field, which) in removed.iter().filter(|_| self.primed) {
                if self.removed.get(field) != Some(which) {
                    if let Some(list) = self.at_field.get(field) {
                        out.extend_from_slice(list);
                    }
                }
            }
            self.removed = removed.clone();
        }
        self.primed = true;
        for field in cleared.difference(&self.cleared) {
            if let Some(list) = self.at_field.get(field) {
                out.extend_from_slice(list);
            }
        }
        if cleared.len() != self.cleared.len() {
            self.cleared = cleared.clone();
        }
        for &(cx, cy) in height_cells {
            // An element's offset can carry a doodad over the cell's edge.
            for y in cy.saturating_sub(1)..=cy + 1 {
                for x in cx.saturating_sub(1)..=cx + 1 {
                    if let Some(list) = self.in_cell.get(&(x, y)) {
                        out.extend_from_slice(list);
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Ticks between two redraws of growing trees.
pub(crate) const GROWTH_TICKS: u64 = 20;

/// GFX-ANIM-3.8a, 3.8c: the falling and sinking deposits placed again at
/// `now` (ticks): a felled tree's tilt, a removed deposit's sinking. False
/// when one of them was not placed before (a full placement is due).
fn move_deposits(
    world: &s2_sim::World,
    now: f32,
    placements: &mut Placements,
    transforms: &mut Query<&mut Transform, With<Object3d>>,
) -> bool {
    let Placements {
        placed,
        fall_axes,
        fall_last,
        sinks,
        ..
    } = placements;
    let removing = world
        .deposits()
        .filter(|(_, d)| matches!(d.state, s2_sim::DepositState::Removing { .. }))
        .map(|(f, _)| f);
    let fields: std::collections::BTreeSet<_> = fall_axes.keys().copied().chain(removing).collect();
    for field in fields {
        let Some(deposit) = world.deposit(field) else {
            continue;
        };
        let key = (crate::world::scene3d::Kind::Deposit, field, deposit.kind);
        let Some((entity, old)) = placed.get_mut(&key) else {
            return false;
        };
        let mut moved = old.clone();
        if let Some(&axis) = fall_axes.get(&field) {
            moved.tilt = match crate::world::scene3d::fall_angle(world.felling_began(field), now) {
                crate::world::scene3d::Fall::Standing => 0.0,
                crate::world::scene3d::Fall::Falling(angle) => {
                    fall_last.insert(field, angle);
                    angle
                }
                crate::world::scene3d::Fall::Lying => fall_last
                    .get(&field)
                    .copied()
                    .unwrap_or(std::f32::consts::FRAC_PI_2),
            };
            moved.tilt_axis = axis;
        }
        if let s2_sim::DepositState::Removing { since, gone_at } = deposit.state {
            let Some(&(y0, h)) = sinks.get(&field) else {
                return false;
            };
            moved.at[1] = y0 - crate::world::scene3d::sink_fraction(since, gone_at, now - 1.0) * h;
        }
        if moved != *old {
            if let Ok(mut transform) = transforms.get_mut(*entity) {
                *transform = placed_transform(&moved);
            }
            *old = moved;
        }
    }
    true
}

/// GFX-OBJ-8.7 (2): each frame the collapsing houses go lower: the base the
/// ground height at the building's field now, minus H x c^2 with c the
/// collapse clock of the slot table. Only their transforms change.
fn move_collapses(
    world: &s2_sim::World,
    placements: &mut Placements,
    transforms: &mut Query<&mut Transform, With<Object3d>>,
) {
    let Placements {
        placed,
        house_slots,
        collapse_heights,
        ..
    } = placements;
    for (house, _, c) in house_slots.collapsing() {
        let key = (
            crate::world::scene3d::Kind::Collapse,
            house.at,
            house.serial,
        );
        let (Some((entity, old)), Some(&h)) = (placed.get_mut(&key), collapse_heights.get(&key))
        else {
            continue;
        };
        let base = world3d::building_position(world.terrain(), house.at.0, house.at.1)[1];
        let y = crate::world::scene3d::collapse_height(base, h, c);
        if old.at[1] != y {
            old.at[1] = y;
            old.sink = Some(c);
            if let Ok(mut transform) = transforms.get_mut(*entity) {
                *transform = placed_transform(old);
            }
        }
    }
}

/// Bring the models on the map up to date with the world: spawn what is
/// new, move what changed, remove what went.
#[allow(clippy::too_many_arguments)]
pub fn sync_objects(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    mut placements: ResMut<Placements>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::render::models3d::ObjectMaterial>>,
    mut transforms: Query<&mut Transform, With<Object3d>>,
    mut spans: Option<ResMut<crate::tools::bench::FrameSpans>>,
    time: Res<Time>,
) {
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let Some(world) = &viewer.world else { return };
    // GFX-OBJ-2.2a: the slot table follows the buildings each frame (they
    // change with the tick), its collapse clocks with the frame time.
    let tick = world.tick();
    let houses =
        (placements.house_slots.tick() != Some(tick)).then(|| crate::world::scene3d::houses(world));
    placements
        .house_slots
        .update(houses.as_deref().map(|h| (tick, h)), time.delta_secs());
    move_collapses(world, &mut placements, &mut transforms);
    // GFX-OBJ-8.7: a collapse that starts or ends places the objects again.
    let collapses = placements.house_slots.collapsing().count() as u64;
    // GFX-ANIM-3.8a: a tree falling is redrawn a few times a tick, the
    // angle interpolated between the logic ticks.
    let now = world.tick() as f32 + viewer.tick_fraction.clamp(0.0, 1.0);
    crate::world::scene3d::update_fall_axes(world, &view.frame, &mut placements.fall_axes);
    let Placements {
        fall_axes,
        fall_last,
        sinks,
        ..
    } = &mut *placements;
    fall_last.retain(|f, _| fall_axes.contains_key(f));
    sinks.retain(|&f, _| {
        world
            .deposit(f)
            .is_some_and(|d| matches!(d.state, s2_sim::DepositState::Removing { .. }))
    });
    let falling = placements.fall_axes.keys().any(|&f| {
        matches!(
            crate::world::scene3d::fall_angle(world.felling_began(f), now),
            crate::world::scene3d::Fall::Falling(_)
        )
    });
    // GFX-ANIM-3.8c: so is a deposit sinking while it is removed.
    let sinking = world
        .deposits()
        .any(|(_, d)| matches!(d.state, s2_sim::DepositState::Removing { .. }));
    let key = (
        world.deposit_version(),
        world.tick() / GROWTH_TICKS,
        viewer.borders,
        if falling || sinking {
            world.tick() * 8 + (viewer.tick_fraction.clamp(0.0, 0.99) * 8.0) as u64
        } else {
            0
        },
        decor_signature(world) ^ collapses.wrapping_mul(0x9e37_79b9_7f4a_7c15),
    );
    let road_signature = Some(crate::world::scene3d::road_content(world));
    let signature_changed = placements.road_signature != road_signature;
    if placements.drawn == Some(key) && !viewer.world_changed && !signature_changed {
        return;
    }
    // GFX-ANIM-3.8a, 3.8c: when only the eighth of a tick of a falling or
    // sinking deposit changed, only those deposits are placed again, not
    // every object of the map (F51).
    let motion_only = !viewer.world_changed
        && !signature_changed
        && placements
            .drawn
            .is_some_and(|d| (d.0, d.1, d.2, d.4) == (key.0, key.1, key.2, key.4));
    if motion_only && move_deposits(world, now, &mut placements, &mut transforms) {
        placements.drawn = Some(key);
        return;
    }
    placements.full_placements += 1;
    // F55: the doodads are placed once, and again only the few that changed;
    // the roads are built again when they or the ground under them changed.
    let (cells_x, _) = terrain3d::cell_count(world.terrain().width, world.terrain().height);
    let height_cells: Vec<(u32, u32)> = placements
        .height_cells
        .drain(..)
        .map(|i| (i as u32 % cells_x, i as u32 / cells_x))
        .collect();
    let road_chunks_moved = crate::world::scene3d::road_chunks_on_cells(world, &height_cells);
    let roads_changed =
        placements.drawn.is_none() || signature_changed || !road_chunks_moved.is_empty();
    let first_doodads = placements.doodad_track.is_none();
    let mut track = placements.doodad_track.take().unwrap_or_else(|| {
        DoodadTrack::new(
            &placements.doodads,
            world.terrain().width,
            world.terrain().height,
        )
    });
    let changed_doodads = track.changed(
        world.doodads_removed(),
        world.decorations_cleared(),
        &height_cells,
    );
    let doodads_again = first_doodads;
    placements.road_signature = road_signature;
    placements.drawn = Some(key);
    // GFX-OBJ-8.1a: the house boxes of the military buildings' models.
    let placements = &mut *placements;
    for building in world.buildings() {
        if building.under_construction || world.military_at(building.at).is_none() {
            continue;
        }
        let Some(gfx) = placements.lookup.buildings.get(&building.building_type) else {
            continue;
        };
        let entry = crate::render::models3d::building_entry(world.tribe(building.owner), gfx);
        let Some(mesh) = library.tables.building(&entry).and_then(|e| e.mesh.clone()) else {
            continue;
        };
        if let std::collections::hash_map::Entry::Vacant(e) = placements.house_boxes.entry(mesh) {
            if let Some(b) = library.house_box(e.key(), &mut meshes, &mut images) {
                e.insert(b);
            }
        }
    }
    let timer = std::time::Instant::now();
    let scenery = crate::world::scene3d::Scenery {
        elements: &library.landscape,
        doodads: if doodads_again {
            &placements.doodads
        } else {
            &[]
        },
        house_boxes: Some(&placements.house_boxes),
        by_name: Some(&library.landscape_by_name),
        load: crate::world::scene3d::Load {
            saved: placements.saved,
            deposits: placements.load_deposits.as_deref(),
            doodads: &placements.doodads,
        },
        house_slots: Some(&placements.house_slots),
    };
    let mut desired = crate::world::scene3d::objects(
        world,
        &placements.lookup,
        &library.tables,
        &scenery,
        &view.frame,
        viewer.borders,
        &mut placements.poses,
    );
    if !changed_doodads.is_empty() {
        desired.extend(crate::world::scene3d::some_doodads(
            world,
            &placements.lookup,
            &scenery,
            &view.frame,
            &placements.doodads,
            &changed_doodads,
            &mut placements.poses,
        ));
    }
    if let Some(spans) = spans.as_mut() {
        spans.add(
            if doodads_again {
                "object list with doodads"
            } else {
                "object list"
            },
            timer,
        );
    }
    let timer = std::time::Instant::now();
    let mut keep = std::collections::HashSet::new();
    for mut placed in desired {
        if placed.key.0 == crate::world::scene3d::Kind::Deposit {
            let field = placed.key.1;
            if let Some(&axis) = placements.fall_axes.get(&field) {
                // GFX-ANIM-3.8a: once lying it keeps the last angle
                // drawn while falling (pi / 2 if none was).
                placed.tilt =
                    match crate::world::scene3d::fall_angle(world.felling_began(field), now) {
                        crate::world::scene3d::Fall::Standing => 0.0,
                        crate::world::scene3d::Fall::Falling(angle) => {
                            placements.fall_last.insert(field, angle);
                            angle
                        }
                        crate::world::scene3d::Fall::Lying => placements
                            .fall_last
                            .get(&field)
                            .copied()
                            .unwrap_or(std::f32::consts::FRAC_PI_2),
                    };
                placed.tilt_axis = axis;
            }
            // GFX-ANIM-3.8c, T11a, T11b: a deposit being removed sinks
            // straight down by 0.05 of its box height per tick, the box
            // taken under its matrix when the removal starts (a felled
            // tree's lying box).
            if let Some(s2_sim::DepositState::Removing { since, gone_at }) =
                world.deposit(field).map(|d| d.state)
            {
                let (y0, h) = *placements.sinks.entry(field).or_insert_with(|| {
                    let h = library
                        .model(
                            &placed.file,
                            placed.units_per_metre,
                            &mut meshes,
                            &mut images,
                        )
                        .map_or(0.0, |m| box_height(m.bounds, &placed));
                    (placed.at[1], h)
                });
                // `now` is one past the tick that ran (the view's frames
                // after tick t show t's targets being reached).
                placed.at[1] =
                    y0 - crate::world::scene3d::sink_fraction(since, gone_at, now - 1.0) * h;
            }
        }
        // GFX-OBJ-8.7 (2): a collapsing house lowered by H x c^2, H its
        // model's height as the rising house's (2.6b).
        if let Some(c) = placed.sink {
            let h = match placements.collapse_heights.get(&placed.key) {
                Some(&h) => h,
                None => {
                    let own = library
                        .model(
                            &placed.file,
                            placed.units_per_metre,
                            &mut meshes,
                            &mut images,
                        )
                        .map_or(0.0, |m| m.extent[1]);
                    let h = library
                        .last_lod(
                            &placed.file,
                            placed.units_per_metre,
                            &mut meshes,
                            &mut images,
                        )
                        .map_or(own, |lod| lod.extent[1]);
                    placements.collapse_heights.insert(placed.key, h);
                    h
                }
            };
            placed.at[1] = crate::world::scene3d::collapse_height(placed.at[1], h, c);
        }
        keep.insert(placed.key);
        match placements.placed.get_mut(&placed.key) {
            Some((entity, old))
                if old.file == placed.file
                    && (old.tint, old.player, old.extra, old.pieces)
                        == (placed.tint, placed.player, placed.extra, placed.pieces) =>
            {
                if *old != placed {
                    if let Ok(mut transform) = transforms.get_mut(*entity) {
                        *transform = placed_transform(&placed);
                    }
                    *old = placed;
                }
            }
            _ => {
                if let Some((entity, _)) = placements.placed.remove(&placed.key) {
                    commands.entity(entity).despawn();
                }
                let Some(model) = library.model(
                    &placed.file,
                    placed.units_per_metre,
                    &mut meshes,
                    &mut images,
                ) else {
                    continue;
                };
                // GFX-OBJ-2.6a: a site shows its first s pieces, s by the
                // frame progress in `extra[3]` (1 + f).
                // GFX-OBJ-8.1: a soldier flag its first n meshes.
                let stage = (placed.extra[3] >= 1.0)
                    .then(|| {
                        let pieces = model.parts.iter().filter(|p| !p.ground).count();
                        crate::world::scene3d::site_stage(pieces, placed.extra[3] - 1.0)
                    })
                    .or(placed.pieces.map(|n| n as usize));
                let mut look = library.placed_look(&placed, &mut images);
                // GFX-OBJ-2.6b: a house being built: its walls only once the
                // main progress m is above 0, cut by the rising plane; H is
                // the height of the last LOD loaded (`...LOD0` if there is
                // one).
                // GFX-OBJ-8.4: during an upgrade the old house (extra[2] 2)
                // shows above the plane of the new house's height.
                let rising = placed.family == crate::render::models3d::Family::House
                    && placed.extra[2] > 0.5;
                let walls = !rising || placed.extra[1] > 0.0 || placed.extra[2] > 1.5;
                if rising {
                    let height_of = placed.plane_of.as_deref().unwrap_or(&placed.file);
                    let own = library
                        .model(height_of, placed.units_per_metre, &mut meshes, &mut images)
                        .map_or(model.extent[1], |m| m.extent[1]);
                    let height = library
                        .last_lod(height_of, placed.units_per_metre, &mut meshes, &mut images)
                        .map_or(own, |lod| lod.extent[1]);
                    look.extra[1] =
                        crate::world::scene3d::rising_plane(placed.at[1], height, placed.extra[1]);
                }
                let transform = placed_transform(&placed);
                let mut parts = Vec::new();
                for (i, part) in model.parts.iter().enumerate() {
                    // A laid decal has no bones (identity palette).
                    // GFX-OBJ-8.7 (1), (5): a collapsing house without its
                    // decal, a ruin's copy of the decal alone.
                    let wanted = match placed.parts {
                        crate::world::scene3d::Parts::All => true,
                        crate::world::scene3d::Parts::NoDecal => part.decal.is_empty(),
                        crate::world::scene3d::Parts::DecalOnly => !part.decal.is_empty(),
                    };
                    if !wanted {
                        continue;
                    }
                    let material = if part.decal.is_empty() {
                        library.rest_material(&model, i, &look, &mut images, &mut materials)
                    } else {
                        library.material(part, &look, &mut images, &mut materials)
                    };
                    if let Some(material) = material {
                        let mesh = if !part.decal.is_empty() {
                            // GFX-OBJ-2.3: the decal laid on the terrain.
                            let to_world = transform.to_matrix();
                            let decal: Vec<_> = part
                                .decal
                                .iter()
                                .map(|t| {
                                    t.map(|(p, uv)| {
                                        let w = to_world.transform_point3(Vec3::from(p));
                                        (world3d::from_bevy(w.to_array()), uv)
                                    })
                                })
                                .collect();
                            let drape = world3d::drape(world.terrain(), &decal);
                            // 2.3a d: the shipped GROUND meshes face
                            // straight up; turned with the building.
                            let up = to_world.transform_vector3(Vec3::Y).normalize();
                            meshes.add(crate::render::models3d::decal_mesh(
                                &drape,
                                to_world.inverse(),
                                world3d::from_bevy(up.to_array()),
                            ))
                        } else if stage.is_some_and(|s| part.piece >= s) || !walls {
                            continue;
                        } else {
                            part.mesh.clone()
                        };
                        // GFX-VIEW-2.5a: the decal and elements marked
                        // `shadowCast="false"` cast no shadow.
                        let casts = placed.shadow && part.decal.is_empty();
                        parts.push((mesh, material, casts));
                    }
                }
                let entity = commands
                    .spawn((Object3d, transform, Visibility::default()))
                    .with_children(|children| {
                        for (mesh, material, casts) in parts {
                            // GFX-WATER-2.6b: the map's objects are in the
                            // mirror image of a flat view with every detail.
                            let mut child = children.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(material),
                                mirrored_object_layers(),
                            ));
                            if !casts {
                                child.insert(bevy::light::NotShadowCaster);
                            }
                        }
                    })
                    .id();
                placements.placed.insert(placed.key, (entity, placed));
            }
        }
    }
    placements.doodad_track = Some(track);
    let mut gone = gone_objects(placements.placed.keys(), &keep);
    // F55: of the doodads, only those looked at can have gone.
    gone.extend(changed_doodads.iter().filter_map(|&i| {
        let key = (
            crate::world::scene3d::Kind::Doodad,
            placements.doodads[i as usize].1,
            i,
        );
        (!keep.contains(&key)).then_some(key)
    }));
    for key in gone {
        // GFX-OBJ-1.4: an instance that went draws afresh if it comes back.
        placements.poses.forget(&key);
        placements.collapse_heights.remove(&key);
        if let Some((entity, _)) = placements.placed.remove(&key) {
            commands.entity(entity).despawn();
        }
    }
    if let Some(spans) = spans.as_mut() {
        spans.add("objects placed", timer);
    }
    let timer = std::time::Instant::now();
    if roads_changed {
        // GFX-OBJ-6.2: the donkey roads with texture 1, drawn first, the
        // other roads with texture 0 together with the flag-to-door strips
        // (6.4, F14), and the crossings at their flags: texture 1 at a flag
        // one of whose roads has asked for a donkey, for good (Q-221).
        crate::world::scene3d::update_donkey_crossings(
            &mut placements.donkey_crossings,
            crate::world::scene3d::donkey_road_ends(world),
            |p| world.flag_at(p).is_some(),
        );
        // GFX-OBJ-6.2a (d), T47: each road's ends decided when it is made.
        crate::world::scene3d::update_road_ends(&mut placements.road_ends, world);
        // F55: only the squares whose strips or ground changed.
        let wanted = crate::world::scene3d::street_parts(
            world,
            &placements.donkey_crossings,
            &placements.road_ends,
        );
        placements.road_parts.retain(|key, (_, entity)| {
            let stays = wanted.contains_key(key);
            if let (false, Some(entity)) = (stays, *entity) {
                commands.entity(entity).despawn();
            }
            stays
        });
        for (&key, &signature) in &wanted {
            let (part, chunk) = key;
            let same = placements
                .road_parts
                .get(&key)
                .is_some_and(|(shown, _)| *shown == signature);
            if same && !road_chunks_moved.contains(&chunk) {
                continue;
            }
            if let Some((_, Some(entity))) = placements.road_parts.remove(&key) {
                commands.entity(entity).despawn();
            }
            let street = crate::world::scene3d::street_part(
                world,
                &view.frame,
                &placements.donkey_crossings,
                &placements.road_ends,
                part,
                chunk,
            );
            let mut made = None;
            if !street.positions.is_empty() {
                let mesh = meshes.add(crate::render::models3d::object_mesh_with_normals(
                    street.positions,
                    street.normals,
                    street.uvs,
                    street.tangents,
                    street.indices,
                ));
                let (texture, normal) = crate::world::scene3d::STREET_PARTS[usize::from(part)];
                if let Some(material) = library.street_material(
                    &mesh,
                    part,
                    texture,
                    normal,
                    &mut images,
                    &mut materials,
                ) {
                    made = Some(
                        commands
                            .spawn((
                                Object3d,
                                Mesh3d(mesh),
                                MeshMaterial3d(material),
                                Transform::IDENTITY,
                                // GFX-WATER-2.6b: with every object family.
                                mirrored_object_layers(),
                                // GFX-VIEW-2.5: streets receive shadows, cast none.
                                bevy::light::NotShadowCaster,
                            ))
                            .id(),
                    );
                }
            }
            placements.road_parts.insert(key, (signature, made));
        }
        if let Some(spans) = spans.as_mut() {
            spans.add("roads rebuilt", timer);
        }
    }
}
