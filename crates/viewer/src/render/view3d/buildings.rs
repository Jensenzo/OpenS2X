//! Building animations (doors, work, production animals) and the wares lying at flags.

use super::*;

/// One building's animations: its door, its working clip and its
/// production animals (GFX-ANIM-6), drawn with materials of its own once
/// something plays.
pub(crate) struct BuildingAnim {
    pub(crate) entity: Entity,
    /// The building's field.
    pub(crate) field: s2_sim::grid::Point,
    /// GFX-OBJ-8.7 (1): drawn without its ground decal (a collapsing house),
    /// so the decal part has no entity among its children.
    pub(crate) no_decal: bool,
    pub(crate) model: crate::render::models3d::LoadedModel,
    pub(crate) door_open: Option<std::sync::Arc<crate::animation::anim::Clip>>,
    pub(crate) door_close: Option<std::sync::Arc<crate::animation::anim::Clip>>,
    pub(crate) working_clip: Option<std::sync::Arc<crate::animation::anim::Clip>>,
    pub(crate) production: Option<s2_assets::model_tables::ProductionModel>,
    pub(crate) door: crate::animation::object_anim::Door,
    /// When the Working clip started (real seconds), while it plays.
    pub(crate) working: Option<f32>,
    /// The parts' own materials, once the building animates.
    pub(crate) materials: Option<Vec<Option<Handle<crate::render::models3d::ObjectMaterial>>>>,
    /// The production animals made, in `boneLink` order, and their poses.
    pub(crate) animals: Vec<(Entity, crate::animation::anim::PoseKey, usize)>,
    /// GFX-OBJ-1.6a: the productivity (hundredths) the shown animals were
    /// counted for; forgotten while production is off or the worker away.
    pub(crate) animals_for: Option<i32>,
    /// GFX-OBJ-1.6a, 8.7 (2) (b): how many of the animals show (the first
    /// ones in `boneLink` order), as the last count left it.
    pub(crate) animals_shown: usize,
    /// Where it stands (game frame), for its sounds; its Working clip's
    /// file and the clock its events were last checked at.
    pub(crate) at: [f32; 3],
    pub(crate) working_file: Option<String>,
    pub(crate) working_events_at: f32,
}

impl BuildingAnim {
    /// A door or Working clip playing or production animals showing
    /// (GFX-OBJ-8.7 (2): what a collapsing house goes on with).
    pub(crate) fn playing(&self) -> bool {
        !matches!(self.door, crate::animation::object_anim::Door::Closed)
            || self.working.is_some()
            || self.animals_shown > 0
    }
}

/// The animated buildings by field and serial, and the tick they were
/// last updated for.
#[derive(Resource, Default)]
pub struct BuildingAnims {
    pub(crate) states: std::collections::HashMap<(s2_sim::grid::Point, u32), BuildingAnim>,
    /// GFX-OBJ-8.7 (2), T39: the clips of finished buildings that left the
    /// world while one played, by serial: they go on, on the collapsing
    /// house, until its slot is freed; none starts or closes any more.
    pub(crate) collapsing: std::collections::HashMap<u32, BuildingAnim>,
    pub(crate) tick: Option<u64>,
}

impl BuildingAnims {
    /// F55: the palette slots the buildings' own parts hold (`--bench`).
    pub(crate) fn slot_count(&self) -> usize {
        self.states
            .values()
            .chain(self.collapsing.values())
            .flat_map(|s| s.materials.iter().flatten())
            .filter(|m| m.is_some())
            .count()
    }
}

/// GFX-OBJ-1.6a: how many of a building's `animals` production animals
/// show at `productivity` hundredths: round(productivity x N), adding a half
/// and cutting off; none before the first product (-100).
pub(crate) fn shown_animals(productivity: i32, animals: usize) -> usize {
    if productivity < 0 {
        return 0;
    }
    let n = animals as i64;
    ((i64::from(productivity) * n + 50) / 100).clamp(0, n) as usize
}

/// GFX-ANIM-6: at each tick open the doors walkers step through (6.1),
/// start and stop the Working clip with the worker's work step (6.2) and
/// show or remove the production animals (6.3); every frame sample each
/// building's playing clips, laid over each other by bone, into its parts'
/// palettes.
#[allow(clippy::too_many_arguments)]
pub fn animate_buildings(
    mut commands: Commands,
    clock: Res<crate::game::GameClock>,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    placements: Res<Placements>,
    mut anims: ResMut<BuildingAnims>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut animator: ResMut<crate::animation::animate3d::Animator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::render::models3d::ObjectMaterial>>,
    children: Query<&Children>,
    mut part_materials: Query<&mut MeshMaterial3d<crate::render::models3d::ObjectMaterial>>,
    mut transforms: Query<&mut Transform, With<Object3d>>,
    mut sounds: ResMut<crate::audio::SoundQueue>,
) {
    use crate::animation::object_anim::{door_triggers, Door, Stepper};
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let Some(world) = &viewer.world else { return };
    let library = &mut *library;
    let animator = &mut *animator;
    let anims = &mut *anims;
    // TOOL-1.2: the building clips (door, Working) on the animation clock;
    // the door's close time on game time as of the last tick (TRACE-5.3).
    let now = clock.seconds;
    let tick = world.tick();
    if anims.tick != Some(world.tick()) {
        anims.tick = Some(world.tick());
        // The finished buildings with animations, kept by field and serial.
        let mut keep = std::collections::HashSet::new();
        for b in world.buildings().filter(|b| !b.under_construction) {
            let key = (b.at, b.serial);
            keep.insert(key);
            if anims.states.contains_key(&key) {
                continue;
            }
            let Some(gfx) = placements.lookup.buildings.get(&b.building_type) else {
                continue;
            };
            let name = crate::render::models3d::building_entry(world.tribe(b.owner), gfx);
            let Some(entry) = library.tables.building(&name).cloned() else {
                continue;
            };
            let Some(mesh) = entry.mesh.as_ref() else {
                continue;
            };
            let Some(&(entity, _)) =
                placements
                    .placed
                    .get(&(crate::world::scene3d::Kind::Building, b.at, b.serial * 2))
            else {
                continue;
            };
            let Some(model) = library.model(
                mesh,
                crate::render::models3d::DEFAULT_SCALE,
                &mut meshes,
                &mut images,
            ) else {
                continue;
            };
            if model.skeleton.is_none() {
                continue;
            }
            let skeleton_file = model.skeleton_file.clone();
            let mut clip = |c: &Option<s2_assets::model_tables::BuildingClip>| {
                c.as_ref()
                    .and_then(|c| library.clip(&c.file, &skeleton_file))
            };
            let working_file = entry.working.as_ref().map(|w| w.file.clone());
            let door_open = clip(&entry.door_open);
            let door_close = clip(&entry.door_close);
            let working_clip = clip(&entry.working);
            anims.states.insert(
                key,
                BuildingAnim {
                    entity,
                    field: b.at,
                    no_decal: false,
                    model,
                    door_open,
                    door_close,
                    working_clip,
                    production: entry.production.clone(),
                    door: Door::Closed,
                    working: None,
                    materials: None,
                    animals: Vec::new(),
                    animals_for: None,
                    animals_shown: 0,
                    at: {
                        let [x, z] = view.frame.field_xz(b.at.0, b.at.1);
                        [x, world3d::ground_height(world.terrain(), x, z), z]
                    },
                    working_file,
                    working_events_at: 0.0,
                },
            );
        }
        let gone: Vec<_> = anims
            .states
            .keys()
            .filter(|k| !keep.contains(k))
            .copied()
            .collect();
        // GFX-OBJ-8.7 (2): the finished buildings whose houses collapse.
        let collapsing: std::collections::HashSet<u32> = placements
            .house_slots
            .collapsing()
            .map(|(house, _, _)| house.serial)
            .collect();
        for key in gone {
            if let Some(state) = anims.states.remove(&key) {
                for (_, pose, _) in &state.animals {
                    animator.detach(pose, &animal_look());
                }
                // F55 (the long play): its parts' palette slots.
                release_palettes(&state, library, &materials);
                // T39: a house collapsing keeps the clips that played.
                if collapsing.contains(&key.1) {
                    if let Some(state) = collapse_takes_over(state) {
                        anims.collapsing.insert(key.1, state);
                    }
                }
            }
        }
        let ended: Vec<u32> = anims
            .collapsing
            .keys()
            .filter(|serial| !collapsing.contains(serial))
            .copied()
            .collect();
        for serial in ended {
            if let Some(state) = anims.collapsing.remove(&serial) {
                for (_, pose, _) in &state.animals {
                    animator.detach(pose, &animal_look());
                }
                release_palettes(&state, library, &materials);
            }
        }
        // GFX-ANIM-6.1a: doors, each building with its door flag (sites
        // and mines have no door clip).
        let doors: std::collections::HashMap<_, _> = anims
            .states
            .iter()
            .filter(|(_, s)| s.door_open.is_some())
            .filter_map(|((at, _), _)| Some((*at, world.terrain().neighbour(*at, 1)?)))
            .collect();
        let views = world.settler_views();
        let steppers: Vec<Stepper> = views
            .iter()
            .filter_map(|v| {
                let id = match v.id {
                    s2_sim::view::SettlerRef::Settler(n) => n as u64,
                    s2_sim::view::SettlerRef::Carrier(n) => (1 << 32) | n as u64,
                    _ => return None,
                };
                Some(Stepper {
                    id,
                    at: v.walk.at,
                    next: v.walk.next,
                    progress: v.walk.fraction(),
                })
            })
            .collect();
        let held = door_triggers(
            &doors,
            &steppers,
            crate::animation::object_anim::ENTER_DISTANCE,
            crate::animation::object_anim::LEAVE_DISTANCE,
        );
        for at in held {
            for ((field, _), state) in anims.states.iter_mut() {
                if *field == at && state.door.open(now, tick) {
                    // SND-2.2: the door sounds (visible buildings: the
                    // audibility test decides).
                    sounds.0.push(crate::audio::sound::SoundRequest::At {
                        name: crate::audio::sound::DOOR_OPEN.into(),
                        at: state.at,
                        source: None,
                    });
                }
            }
        }
        // GFX-ANIM-6.2, 6.3: the worker's work step; producing is taken as
        // the worker being in it (Q-181).
        let working: std::collections::HashSet<_> = views
            .iter()
            .filter_map(|v| match v.activity {
                s2_sim::view::Activity::Worker {
                    building,
                    phase: s2_sim::view::WorkPhase::Working,
                    ..
                } => Some(building),
                _ => None,
            })
            .collect();
        for ((at, _), state) in anims.states.iter_mut() {
            let busy = working.contains(at);
            match (busy, state.working) {
                (true, None) => state.working = Some(now),
                (false, Some(_)) => state.working = None,
                _ => {}
            }
            // GFX-OBJ-1.6a (Q-349): the production animals, checked while
            // the building's production is on and its worker is in, and
            // changed only when its productivity has changed: the first
            // round(productivity x N) of the N in `boneLink` order show
            // (none before the first product). Otherwise the shown ones
            // stay as they were.
            let Some(prod) = state.production.clone() else {
                continue;
            };
            let Some(building) = world.building_at(*at) else {
                continue;
            };
            if !(building.production.enabled && building.active) {
                state.animals_for = None;
                continue;
            }
            if state.animals_for == Some(building.productivity) {
                continue;
            }
            state.animals_for = Some(building.productivity);
            if state.animals.is_empty() {
                make_animals(
                    state,
                    &prod,
                    &mut commands,
                    library,
                    animator,
                    &mut meshes,
                    &mut images,
                    &mut materials,
                );
            }
            state.animals_shown = shown_animals(building.productivity, prod.bone_links.len());
            show_animals(state, &mut commands);
        }
    }
    // Every frame: the clips playing, laid over the rest pose.
    for state in anims.states.values_mut() {
        if state.door.update(now, tick) {
            sounds.0.push(crate::audio::sound::SoundRequest::At {
                name: crate::audio::sound::DOOR_CLOSE.into(),
                at: state.at,
                source: None,
            });
        }
        play_working_sounds(state, library, now, &mut sounds);
        write_pose(
            state,
            now,
            library,
            &mut materials,
            &children,
            &mut part_materials,
            &mut transforms,
        );
    }
    // GFX-OBJ-8.7 (2), T39, T43: a collapsing house's clips go on (a door
    // that had begun opening runs to its end and holds; the work clip to
    // its end, its Sound events firing at their times); no door closes or
    // opens, so no door sound; its production animals, made again on the
    // collapse's entity as many as showed, loop on their bones and sink
    // with it.
    for (serial, state) in anims.collapsing.iter_mut() {
        let key = (crate::world::scene3d::Kind::Collapse, state.field, *serial);
        let Some(&(entity, _)) = placements.placed.get(&key) else {
            continue;
        };
        if state.entity != entity {
            release_palettes(state, library, &materials);
            state.entity = entity;
            state.materials = None;
            for (_, pose, _) in std::mem::take(&mut state.animals) {
                animator.detach(&pose, &animal_look());
            }
            if let Some(prod) = state.production.clone().filter(|_| state.animals_shown > 0) {
                make_animals(
                    state,
                    &prod,
                    &mut commands,
                    library,
                    animator,
                    &mut meshes,
                    &mut images,
                    &mut materials,
                );
                show_animals(state, &mut commands);
            }
        }
        play_working_sounds(state, library, now, &mut sounds);
        write_pose(
            state,
            now,
            library,
            &mut materials,
            &children,
            &mut part_materials,
            &mut transforms,
        );
    }
}

/// GFX-OBJ-8.7 (2), T39, T43: what a finished building's clips do when it
/// leaves the world and its house collapses: the clips playing (a door
/// opening or closing, the Working clip) go on, on the collapse's own
/// entity, which is drawn without the ground decal; the state is kept as
/// it was (no door closes, nothing restarts, no rest pose, the work clip's
/// events go on from where they were). Its production animals (the
/// entities went with the building's) are made again on the collapse's
/// entity, as many as showed (the count is not updated any more). None
/// when nothing plays and no animal shows: the house then sinks in its
/// rest pose, as it stood.
pub(crate) fn collapse_takes_over(mut state: BuildingAnim) -> Option<BuildingAnim> {
    if !state.playing() {
        return None;
    }
    state.entity = Entity::PLACEHOLDER;
    state.no_decal = true;
    state.materials = None;
    state.animals.clear();
    state.animals_for = None;
    Some(state)
}

/// GFX-ANIM-2.3, GFX-OBJ-8.7 (2) (c): the Sound events of the Working clip
/// (played once, `duration` animation-seconds) its clock passed since the
/// last call, each at most once; the clock is kept for the next call (and
/// reset while no work clip plays).
pub(crate) fn working_sounds(
    state: &mut BuildingAnim,
    events: &[(String, f32)],
    duration: f32,
    now: f32,
) -> Vec<String> {
    let Some(since) = state.working else {
        state.working_events_at = 0.0;
        return Vec::new();
    };
    let t = (now - since) * crate::animation::anim::GLOBAL_SPEED;
    let last = state.working_events_at.min(t);
    state.working_events_at = t;
    crate::audio::sound::events_crossed(events, last, t, duration, false)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// The Working clip's Sound events this frame, heard at the building
/// (SND-2.2: the audibility test decides).
fn play_working_sounds(
    state: &mut BuildingAnim,
    library: &crate::render::models3d::ModelLibrary,
    now: f32,
    sounds: &mut crate::audio::SoundQueue,
) {
    let (Some(clip), Some(file)) = (state.working_clip.clone(), state.working_file.clone()) else {
        state.working_events_at = 0.0;
        return;
    };
    for name in working_sounds(state, library.sounds_of(&file), clip.duration, now) {
        sounds.0.push(crate::audio::sound::SoundRequest::At {
            name,
            at: state.at,
            source: None,
        });
    }
}

/// GFX-OBJ-1.6a: a building's production animals, one per `boneLink`, each
/// with the one clip looped, children of its entity (hidden until
/// [`show_animals`]).
#[allow(clippy::too_many_arguments)]
fn make_animals(
    state: &mut BuildingAnim,
    prod: &s2_assets::model_tables::ProductionModel,
    commands: &mut Commands,
    library: &mut crate::render::models3d::ModelLibrary,
    animator: &mut crate::animation::animate3d::Animator,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<crate::render::models3d::ObjectMaterial>,
) {
    let Some(model) = library.skinned_model(
        &prod.mesh,
        prod.model.as_deref(),
        crate::render::models3d::DEFAULT_SCALE,
        meshes,
        images,
    ) else {
        return;
    };
    let Some(skeleton) = state.model.skeleton.clone() else {
        return;
    };
    for link in &prod.bone_links {
        let Some(bone) = skeleton.find(link) else {
            continue;
        };
        // The one clip, looped (GFX-OBJ-1.6a).
        let want = prod
            .anims
            .first()
            .and_then(|a| pose_key(&model, Some(&(a.clone(), true))));
        let Some(want) = want else { continue };
        let (pose, parts) =
            animator.attach(&want, &model, &animal_look(), library, images, materials);
        let (animal, _) = spawn_parts(
            commands,
            Transform::IDENTITY,
            parts,
            &mirrored_object_layers(),
        );
        commands.entity(state.entity).add_child(animal);
        state.animals.push((animal, pose, bone));
    }
}

/// The look a production animal is drawn with (plain), the one it leaves
/// its pose with (F55).
fn animal_look() -> crate::render::models3d::Look {
    crate::render::models3d::Look::plain([1.0; 4])
}

/// GFX-OBJ-1.6a: the first `animals_shown` animals show, the others not.
fn show_animals(state: &BuildingAnim, commands: &mut Commands) {
    for (i, (animal, _, _)) in state.animals.iter().enumerate() {
        let visibility = if i < state.animals_shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        commands.entity(*animal).insert(visibility);
    }
}

/// F55: a building's parts' own palette slots, given back.
fn release_palettes(
    state: &BuildingAnim,
    library: &mut crate::render::models3d::ModelLibrary,
    materials: &Assets<crate::render::models3d::ObjectMaterial>,
) {
    for m in state.materials.iter().flatten().flatten() {
        if let Some(m) = materials.get(m) {
            library
                .palettes
                .release(crate::render::models3d::palette_offset(&m.params));
        }
    }
}

/// Every frame: a building's clips playing, laid over the rest pose, into
/// its parts' palettes (and its production animals at their bones).
#[allow(clippy::too_many_arguments)]
fn write_pose(
    state: &mut BuildingAnim,
    now: f32,
    library: &mut crate::render::models3d::ModelLibrary,
    materials: &mut Assets<crate::render::models3d::ObjectMaterial>,
    children: &Query<&Children>,
    part_materials: &mut Query<&mut MeshMaterial3d<crate::render::models3d::ObjectMaterial>>,
    transforms: &mut Query<&mut Transform, With<Object3d>>,
) {
    use crate::animation::object_anim::Door;
    let door = match state.door {
        Door::Closed => None,
        Door::Open { since, .. } => state.door_open.clone().map(|c| (c, since)),
        Door::Closing { since } => state.door_close.clone().map(|c| (c, since)),
    };
    let working = state
        .working
        .and_then(|since| state.working_clip.clone().map(|c| (c, since)));
    if door.is_none() && working.is_none() && state.animals.is_empty() {
        return;
    }
    let Some(skeleton) = state.model.skeleton.clone() else {
        return;
    };
    let mut locals = skeleton.rest.clone();
    for (clip, since) in working.iter().chain(door.iter()) {
        let t = clip.time_at((now - since) * crate::animation::anim::GLOBAL_SPEED, false);
        clip.apply(&skeleton, &mut locals, t, false);
    }
    let world_pose = skeleton.world(&locals);
    let composites = state.model.converted(&skeleton.composites(&world_pose));
    // The parts' own materials, made when the building first animates
    // (its door or its worker's clip): copies of the materials the
    // parts are drawn with, so that the building keeps its look and
    // light (GFX-OBJ-2.1, F52); only their bones are their own.
    if state.materials.is_none() {
        let Ok(kids) = children.get(state.entity) else {
            return;
        };
        let mut own = Vec::new();
        let mut child = 0;
        for part in &state.model.parts {
            // The decal of a collapsing house was not made.
            if state.no_decal && !part.decal.is_empty() {
                own.push(None);
                continue;
            }
            let i = child;
            child += 1;
            // A rigid part, or the decal laid on the terrain.
            if part.palette.is_empty() || !part.decal.is_empty() {
                own.push(None);
                continue;
            }
            let m = kids.get(i).and_then(|&child| {
                let mut slot = part_materials.get_mut(child).ok()?;
                let mut copy = materials.get(&slot.0)?.clone();
                // F51: a palette slot of its own.
                let offset = library
                    .palettes
                    .alloc(&crate::render::models3d::Bones::IDENTITY);
                copy.params.mode.y = offset as f32;
                let m = materials.add(copy);
                slot.0 = m.clone();
                Some(m)
            });
            own.push(m);
        }
        state.materials = Some(own);
    }
    if let Some(own) = &state.materials {
        for (part, m) in state.model.parts.iter().zip(own) {
            if let Some(m) = m.as_ref().and_then(|m| materials.get(m)) {
                let offset = crate::render::models3d::palette_offset(&m.params);
                let bones =
                    crate::render::models3d::Bones::from_palette(&part.palette, &composites);
                library.palettes.write(offset, &bones);
            }
        }
    }
    // The production animals stand at their bones.
    for (animal, _, bone) in &state.animals {
        if let (Some(w), Ok(mut t)) = (world_pose.get(*bone), transforms.get_mut(*animal)) {
            *t = Transform::from_matrix(anim_frame(&state.model, *w));
        }
    }
}

/// The wares lying at flags: each flag's stacks and each ware's entity.
#[derive(Resource, Default)]
pub struct Wares3d {
    pub(crate) flags:
        std::collections::HashMap<s2_sim::grid::Point, crate::world::ware_stacks::FlagStacks>,
    pub(crate) shown: std::collections::HashMap<usize, (Entity, String)>,
    pub(crate) tick: Option<u64>,
}

/// GFX-OBJ-5: at each tick the wares at every flag go into their stacks
/// and are drawn there with their good's model.
#[allow(clippy::too_many_arguments)]
pub fn sync_wares(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    placements: Res<Placements>,
    mut wares: ResMut<Wares3d>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut animator: ResMut<crate::animation::animate3d::Animator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::render::models3d::ObjectMaterial>>,
    mut transforms: Query<&mut Transform, With<Object3d>>,
) {
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let Some(world) = &viewer.world else { return };
    if wares.tick == Some(world.tick()) {
        return;
    }
    wares.tick = Some(world.tick());
    let wares = &mut *wares;
    let library = &mut *library;
    let mut keep = std::collections::HashSet::new();
    let mut flags_seen = std::collections::HashSet::new();
    for (flag, _) in world.flags() {
        let here: Vec<(usize, u32)> = world
            .wares_at(flag)
            .iter()
            .filter_map(|&id| world.ware(id).map(|w| (id, w.good)))
            .collect();
        if here.is_empty() {
            continue;
        }
        flags_seen.insert(flag);
        let stacks = wares.flags.entry(flag).or_default();
        stacks.update(&here);
        let [fx, fz] = view.frame.field_xz(flag.0, flag.1);
        for (id, stack, k) in stacks.slots().collect::<Vec<_>>() {
            let Some(good) = world.ware(id).map(|w| w.good) else {
                continue;
            };
            let Some(name) = placements.lookup.goods.get(&good) else {
                continue;
            };
            let Some(item) = library.tables.item(name).cloned() else {
                continue;
            };
            let Some(model) = library.model(
                &item.mesh,
                crate::render::models3d::DEFAULT_SCALE,
                &mut meshes,
                &mut images,
            ) else {
                continue;
            };
            // GFX-OBJ-5.2: the good's layout, layout A for the others.
            let layouts = &library.tables.stack_layouts;
            let Some(layout) = layouts
                .iter()
                .find(|l| l.goods.iter().any(|g| g == name))
                .or(layouts.first())
            else {
                continue;
            };
            let (offset, heading) = crate::world::ware_stacks::ware_offset(
                stack,
                k,
                &layout.offsets,
                model.extent,
                library.tables.stack_distance,
            );
            let (x, z) = (fx + offset[0], fz + offset[2]);
            let y = world3d::ground_height(world.terrain(), x, z) + offset[1];
            let transform = Transform::from_translation(Vec3::from(world3d::to_bevy([x, y, z])))
                .with_rotation(Quat::from_rotation_y(world3d::bevy_yaw(heading)));
            keep.insert(id);
            match wares.shown.get(&id) {
                Some((entity, file)) if *file == item.mesh => {
                    if let Ok(mut t) = transforms.get_mut(*entity) {
                        *t = transform;
                    }
                }
                _ => {
                    if let Some((entity, _)) = wares.shown.remove(&id) {
                        commands.entity(entity).despawn();
                    }
                    let (_, parts) = parts_in(
                        &mut animator,
                        library,
                        &model,
                        None,
                        &crate::render::models3d::Look::plain([1.0; 4]),
                        (&mut images, &mut materials),
                    );
                    let (entity, _) =
                        spawn_parts(&mut commands, transform, parts, &mirrored_object_layers());
                    wares.shown.insert(id, (entity, item.mesh.clone()));
                }
            }
        }
    }
    wares.flags.retain(|f, _| flags_seen.contains(f));
    let gone: Vec<usize> = wares
        .shown
        .keys()
        .filter(|id| !keep.contains(*id))
        .copied()
        .collect();
    for id in gone {
        if let Some((entity, _)) = wares.shown.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::shown_animals;

    // GFX-OBJ-1.6a (Q-349): none before the first product, which sets 0.5:
    // 3 of 5 donkeys, 2 of 4 pigs; all at full productivity; round(p x N)
    // by adding a half and cutting off in between.
    // spec: GFX-OBJ-1.6a
    #[test]
    fn production_animals_by_productivity() {
        assert_eq!(shown_animals(-100, 5), 0);
        assert_eq!(shown_animals(50, 5), 3);
        assert_eq!(shown_animals(50, 4), 2);
        assert_eq!(shown_animals(100, 5), 5);
        assert_eq!(shown_animals(45, 5), 2);
        assert_eq!(shown_animals(0, 4), 0);
        assert_eq!(shown_animals(75, 4), 3);
    }
}

#[cfg(test)]
mod collapse_tests {
    use super::*;
    use crate::animation::object_anim::Door;

    fn anim(door: Door, working: Option<f32>) -> BuildingAnim {
        BuildingAnim {
            entity: Entity::from_raw_u32(7).expect("an entity"),
            field: (10, 12),
            no_decal: false,
            model: crate::render::models3d::LoadedModel {
                parts: Vec::new(),
                skeleton: None,
                skeleton_file: String::new(),
                basis: Mat4::IDENTITY,
                basis_inverse: Mat4::IDENTITY,
                extent: [0.0; 3],
                bounds: [[0.0; 3]; 2],
                house: None,
            },
            door_open: None,
            door_close: None,
            working_clip: None,
            production: None,
            door,
            working,
            materials: Some(Vec::new()),
            animals: Vec::new(),
            animals_for: Some(40),
            animals_shown: 0,
            at: [0.0; 3],
            working_file: None,
            working_events_at: 0.0,
        }
    }

    /// GFX-OBJ-8.7 (2), T39: a house torn down while its door opens or its
    /// Working clip plays keeps that state for its collapse (the door still
    /// open since the same moment, the work clip since its start: they run
    /// on as before, nothing is reset), drawn on the collapse's entity
    /// without its decal; an idle house hands nothing over.
    // spec: GFX-OBJ-8.7 (T39)
    #[test]
    fn t39_a_collapsing_house_keeps_its_clips() {
        let door = Door::Open {
            since: 3.25,
            close_tick: 120,
        };
        let kept = collapse_takes_over(anim(door, Some(1.5))).expect("its clips go on");
        assert_eq!(kept.door, door);
        assert_eq!(kept.working, Some(1.5));
        assert!(kept.no_decal && kept.materials.is_none());
        assert_eq!(kept.entity, Entity::PLACEHOLDER);
        assert_eq!(kept.field, (10, 12));
        let closing = Door::Closing { since: 2.0 };
        assert_eq!(
            collapse_takes_over(anim(closing, None)).map(|s| s.door),
            Some(closing)
        );
        assert!(collapse_takes_over(anim(Door::Closed, Some(0.0))).is_some());
        assert!(collapse_takes_over(anim(Door::Closed, None)).is_none());
    }

    /// GFX-OBJ-8.7 (2) (b), (c), T43: a donkey breeder showing 5 donkeys,
    /// its Working clip started 5.0 animation-seconds before it is torn
    /// down (events `donkey` at 10, 30, 50; the clip played once): the
    /// collapse keeps the five (made again, looping, on the collapse's
    /// entity) and its work clip's events go on from where it was: one
    /// `donkey` at animation-second 10, about 2.8 s after the removal, none
    /// at 30 or 50 within the 2.857 s; no door. An idle breeder showing
    /// donkeys hands them over too.
    // spec: GFX-OBJ-8.7 (T43)
    #[test]
    fn t43_a_collapsing_house_keeps_its_animals_and_clip_sounds() {
        use crate::animation::anim::GLOBAL_SPEED;
        let mut idle = anim(Door::Closed, None);
        idle.animals_shown = 5;
        let kept = collapse_takes_over(idle).expect("its animals go on");
        assert_eq!(kept.animals_shown, 5);
        assert!(kept.animals.is_empty() && kept.animals_for.is_none());
        let events: Vec<(String, f32)> = [10.0, 30.0, 50.0]
            .into_iter()
            .map(|t| ("donkey".to_owned(), t))
            .collect();
        let removal = 100.0;
        let mut breeder = anim(Door::Closed, Some(removal - 5.0 / GLOBAL_SPEED));
        breeder.animals_shown = 5;
        // Up to the removal the clip had reached 5.0: no event yet.
        let mut heard = Vec::new();
        let frame = 1.0 / 60.0;
        let mut now = removal - 1.0;
        while now <= removal {
            heard.extend(working_sounds(&mut breeder, &events, 60.0, now));
            now += frame;
        }
        assert!(heard.is_empty());
        let mut kept = collapse_takes_over(breeder).expect("its clip goes on");
        assert_eq!(kept.animals_shown, 5);
        let mut at = Vec::new();
        while now <= removal + 2.857 {
            for name in working_sounds(&mut kept, &events, 60.0, now) {
                at.push((name, now - removal));
            }
            now += frame;
        }
        assert_eq!(at.len(), 1);
        assert_eq!(at[0].0, "donkey");
        assert!((at[0].1 - 5.0 / GLOBAL_SPEED).abs() < 0.02, "{at:?}");
        assert_eq!(kept.door, Door::Closed);
    }
}
