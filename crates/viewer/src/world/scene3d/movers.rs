//! Settlers, animals and ships where the 3D view draws them: positions,
//! headings, what decides their clips (GFX-VIEW-4.2, GFX-ANIM-3, 4).

use super::*;
use s2_sim::trace::Action;
use s2_sim::view::{Activity, FineStep, SettlerRef, Step, WorkPhase};

/// Something that moves: a settler, a road carrier, an animal or a ship,
/// by its index in the world's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MoverId {
    Settler(usize),
    Carrier(usize),
    Animal(usize),
    Ship(usize),
    /// The builder and the leveller of the site on a field.
    Builder(Point),
    Leveller(Point),
}

/// What decides a mover's clip.
#[derive(Debug, Clone, PartialEq)]
pub enum MoverAnim {
    /// A settler (or the donkey, `donkey`): its tick state, the carried
    /// good's name, the profession's variant name and the tribe
    /// (GFX-ANIM-1.3, 3.1).
    Settler {
        state: crate::animation::settler_anim::SettlerState,
        good: Option<String>,
        profession: Option<String>,
        tribe: u8,
        donkey: bool,
    },
    /// A fixed clip and whether it loops (animals, GFX-ANIM-5.1).
    Clip(Option<(String, bool)>),
    /// A clip played once on a pose of its own (an animal's Dying,
    /// GFX-ANIM-2.4, 5.2).
    Unique(String),
}

/// Where a mover is at this tick and which models draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct Mover {
    pub id: MoverId,
    pub file: String,
    pub units_per_metre: f32,
    /// GFX-VIEW-4.2: the tick's target position (game frame).
    pub at: [f32; 3],
    /// The heading of the step in progress, if it is walking.
    pub heading: Option<f32>,
    pub tint: [f32; 4],
    /// The model of the ware or tool it holds (GFX-VIEW-4.5) and the bone
    /// it is held at (GFX-OBJ-7.1).
    pub carried: Option<String>,
    /// GFX-OBJ-7.6: a rowing carrier's boat model; his ware then lies in
    /// the boat instead of at the bone.
    pub boat: Option<String>,
    pub link: Option<String>,
    /// The skeleton file the mesh is posed with (none: the mesh's own).
    pub skeleton: Option<String>,
    pub anim: MoverAnim,
    /// A settler's colouring (GFX-OBJ-3).
    pub paint: Option<crate::render::models3d::Paint>,
    /// A ship on the water, which bobs (GFX-ANIM-7.2).
    pub bob: bool,
    /// The shading family and owner's colour of a mover not painted as a
    /// settler (GFX-OBJ-2.8).
    pub family: models3d::Family,
    pub player: [f32; 3],
}

/// GFX-ANIM-1.1: the base animations of settlers and the donkey, by index.
pub const SETTLER_BASES: [&str; 10] = [
    "Stand",
    "Walk",
    "SwimIdle",
    "SwimFwd",
    "WorkLoop0",
    "WorkLoop1",
    "WorkLoop2",
    "Pickup",
    "PutDown",
    "Hit",
];

/// GFX-ANIM-2.2: Stand, Walk, SwimIdle and SwimFwd loop; the others play
/// once.
pub fn base_loops(base: usize) -> bool {
    base <= 3
}

/// The file of a named entry of an animation list (the first of the name).
pub(crate) fn entry_file<'a>(
    entries: &'a [s2_assets::anim_tables::AnimEntry],
    name: &str,
) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.file.as_str())
}

/// GFX-ANIM-5.1: an animal's clip, Walk while walking, Stand at rest;
/// both loop.
pub(crate) fn walk_or_stand(
    entries: &[s2_assets::anim_tables::AnimEntry],
    moving: bool,
) -> Option<(String, bool)> {
    let base = usize::from(moving);
    entry_file(entries, SETTLER_BASES[base]).map(|f| (f.to_owned(), true))
}

/// GFX-VIEW-4.2: a walker's position: from its field toward the next by
/// the step's progress, the height interpolated the same way, and the
/// heading of the step (GFX-VIEW-4.1). `y` of `None` keeps the ground
/// height; a ship gives the water level.
pub fn walker_point(
    world: &World,
    frame: &Frame,
    walker: &Step,
    y: Option<f32>,
) -> ([f32; 3], Option<f32>) {
    let point = |field: Point| {
        let [x, z] = frame.field_xz(field.0, field.1);
        [
            x,
            y.unwrap_or_else(|| ground_height(world.terrain(), x, z)),
            z,
        ]
    };
    let from = point(walker.at);
    let Some(next) = walker.next else {
        return (from, None);
    };
    let to = point(next);
    let f = walker.fraction();
    let mut at = [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * f);
    // GFX-VIEW-4.2 interpolates the height linearly between the fields,
    // but the ground is drawn on the finer height lattice: on a bulge
    // between two fields a walker sank into it (play-test 4). On land he
    // stands on the drawn ground (Q-207).
    if y.is_none() {
        at[1] = ground_height(world.terrain(), at[0], at[2]);
    }
    let heading = crate::render::world3d::step_direction(walker.at, next)
        .map(crate::render::world3d::heading);
    (at, heading)
}

/// GFX-VIEW-4.2 on the height lattice (TRN-FREE-5): a worker's position
/// between the sample he last reached and the next by the step's progress,
/// on the ground, and the heading he faces (GFX-VIEW-4.1).
pub fn fine_point(world: &World, frame: &Frame, fine: &FineStep) -> ([f32; 3], Option<f32>) {
    let point = |sample: Point| {
        let [x, z] = frame.sample_xz(i64::from(sample.0), i64::from(sample.1));
        [x, ground_height(world.terrain(), x, z), z]
    };
    let from = point(fine.step.at);
    let at = match fine.step.next {
        Some(next) => {
            let to = point(next);
            let f = fine.step.fraction();
            let mut at = [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * f);
            // On the drawn ground, as a walker between fields (Q-207).
            at[1] = ground_height(world.terrain(), at[0], at[2]);
            at
        }
        None => from,
    };
    (at, fine.facing.map(crate::render::world3d::heading))
}

/// GFX-ANIM-4.2: turn `current` toward `target` by at most `step` radians,
/// the shorter way round; the result is wrapped to -pi .. pi.
pub fn ease_heading(current: f32, target: f32, step: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let wrap = |a: f32| (a + PI).rem_euclid(TAU) - PI;
    let delta = wrap(target - current);
    wrap(current + delta.clamp(-step, step))
}

/// The shorter-way blend of two headings.
pub fn lerp_heading(from: f32, to: f32, t: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let delta = (to - from + PI).rem_euclid(TAU) - PI;
    from + delta * t
}

/// GFX-VIEW-4.1, GFX-ANIM-4.2: radians a heading turns per logic tick.
pub const TURN_PER_TICK: f32 = 0.439_822_97;

/// GFX-OBJ-1.7 (Q-176 b): the `settler_config.xml` character a settler is
/// drawn as, `<Tribe>_<Profession>`: its tribe and its profession's name;
/// soldiers by rank as `Soldier`, `Soldier1` .. `Soldier4`; road carriers as
/// `Carrier`. The visual kind is the settler class (GFX-ANIM-3.1, Q-089
/// answered).
pub fn character_name(tribe: u8, kind: &str) -> String {
    format!(
        "{}_{kind}",
        models3d::TRIBE_NAMES[usize::from(tribe).min(models3d::TRIBE_NAMES.len() - 1)]
    )
}

pub(crate) fn soldier_kind(rank: usize) -> String {
    if rank == 0 {
        "Soldier".into()
    } else {
        format!("Soldier{rank}")
    }
}

/// GFX-OBJ-7.1: the bone a settler holds its ware and tool at.
pub const HAND_SLOT: &str = "SLOT_HandR";

/// GFX-ANIM-3.1: the clip request of the simulation's action (the view
/// model gives requests only).
pub(crate) fn request_of(action: Action) -> Option<Request> {
    match action {
        Action::PutDown => Some(Request::PutDown),
        Action::Pickup => Some(Request::Pickup),
        Action::WorkLoop(i) => Some(Request::WorkLoop(i)),
        Action::Hit => Some(Request::Hit),
        Action::Stand | Action::Walk | Action::Dying => None,
    }
}

/// GFX-ANIM-3.1: a walker's tick state without a request. GFX-ANIM-3.1a:
/// moving from the tick a walk is ordered, so no Stand tick shows between
/// two walks.
pub(crate) fn walking_state(world: &World, at: [f32; 3], has_walk: bool) -> SettlerState {
    SettlerState {
        request: None,
        inside: false,
        moving: has_walk,
        // The model's base below the terrain (it stands on it here).
        under: at[1] < ground_height(world.terrain(), at[0], at[2]) - 0.01,
        dying: false,
        override_base: None,
        started: None,
    }
}

/// GFX-VIEW-5.4 (a): whose seen mark: the settler's object number, which it
/// keeps while it changes jobs (SIM-START-4.2: a constructor walking to his
/// site, at it and going home is one settler), else its mover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeenKey {
    Object(u32),
    Mover(MoverId),
}

/// GFX-VIEW-5.4: a settler that carries a seen mark (every settler mover of
/// [`movers`] but the donkeys, and a settler inside a building, `shown`
/// false): its mover, whose mark it is, its tick position (game frame) and
/// whether it walks (b: its step under way or a walk ordered).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeenPoint {
    pub id: MoverId,
    pub key: SeenKey,
    pub at: [f32; 3],
    /// GFX-VIEW-5.4 (b), (f): the settler's field, tested against the
    /// frame's drawn field rectangle.
    pub field: Point,
    pub walks: bool,
    pub shown: bool,
}

/// GFX-VIEW-5.4: the settlers with a seen mark, at the positions
/// [`movers`] draws them: settlers outside a building, road carriers not on
/// a donkey, builders on their fine walk, levellers on the neighbour they
/// level. A leveller walks while the logic's walk runs, though the view
/// draws him standing (a stand-in there). Needs no models, so a tool can
/// keep the marks while it plays a game ahead (5.4 g).
pub fn seen_points(world: &World, frame: &Frame) -> Vec<SeenPoint> {
    let mut out = Vec::new();
    for view in world.settler_views() {
        let key = |id| match view.object {
            0 => SeenKey::Mover(id),
            object => SeenKey::Object(object),
        };
        let point = match view.id {
            // Inside a building: its mark is kept, not refreshed.
            SettlerRef::Settler(id) if view.hidden => SeenPoint {
                id: MoverId::Settler(id),
                key: key(MoverId::Settler(id)),
                at: walker_point(world, frame, &view.walk, None).0,
                field: view.walk.at,
                walks: false,
                shown: false,
            },
            SettlerRef::Settler(id) => {
                let at = match &view.fine {
                    Some(fine) => fine_point(world, frame, fine).0,
                    None => walker_point(world, frame, &view.walk, None).0,
                };
                let fine_walks = view.fine.as_ref().is_some_and(|f| f.step.next.is_some());
                SeenPoint {
                    id: MoverId::Settler(id),
                    key: key(MoverId::Settler(id)),
                    at,
                    field: view.walk.at,
                    walks: fine_walks || view.has_walk,
                    shown: true,
                }
            }
            SettlerRef::Carrier(id)
                if !view.donkey
                    && !matches!(view.activity, Activity::Carrier { retired: true, .. }) =>
            {
                SeenPoint {
                    id: MoverId::Carrier(id),
                    key: key(MoverId::Carrier(id)),
                    at: walker_point(world, frame, &view.walk, None).0,
                    field: view.walk.at,
                    walks: view.has_walk,
                    shown: true,
                }
            }
            SettlerRef::Builder(at) => {
                let Some(fine) = &view.fine else { continue };
                let xz = |s: Point| frame.sample_xz(i64::from(s.0), i64::from(s.1));
                let (p, walks) = match fine.step.next {
                    Some(to) => {
                        let (a, b) = (xz(fine.step.at), xz(to));
                        let f = fine.step.fraction();
                        let p = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                        (p, a != b)
                    }
                    None => (xz(fine.step.at), false),
                };
                SeenPoint {
                    id: MoverId::Builder(at),
                    key: key(MoverId::Builder(at)),
                    at: [p[0], ground_height(world.terrain(), p[0], p[1]), p[1]],
                    field: s2_sim::grid::sample_field_truncating(fine.step.at),
                    walks,
                    shown: true,
                }
            }
            SettlerRef::Leveller(at) => {
                let [x, z] = frame.field_xz(view.walk.at.0, view.walk.at.1);
                SeenPoint {
                    id: MoverId::Leveller(at),
                    key: key(MoverId::Leveller(at)),
                    at: [x, ground_height(world.terrain(), x, z), z],
                    field: view.walk.at,
                    walks: view.has_walk,
                    shown: true,
                }
            }
            _ => continue,
        };
        out.push(point);
    }
    out
}

/// Every settler outside a building, road carrier (on a donkey where the
/// road has one), builder and leveller at a site, animal and ship at its
/// tick position (GFX-VIEW-4.2), with its model, what decides its clip,
/// its colouring and the ware it carries.
pub fn movers(
    world: &World,
    lookup: &Lookup,
    tables: &ModelTables,
    animations: &s2_assets::anim_tables::SettlerAnimations,
    frame: &Frame,
    fogged: &dyn Fn(Point) -> bool,
) -> Vec<Mover> {
    use crate::world::carry::{look, Holder, Look};
    let _ = animations;
    let mut out = Vec::new();
    let tool_of = |job: &str| lookup.tool_names.get(job).cloned();
    let look_of =
        |holder: Holder, job: &str, ware: Option<u32>, moving: bool, field: Point, rowing: bool| {
            let ware = ware.and_then(|g| lookup.goods.get(&g)).map(String::as_str);
            look(holder, job, ware, &tool_of, moving, fogged(field), rowing)
        };
    let boat = tables.item("Boat").map(|item| item.mesh.clone());
    let character = |player: s2_sim::PlayerId, kind: &str| {
        let name = character_name(world.tribe(player), kind);
        tables.character(&name)
    };
    let mesh_of = |good: Option<&str>| tables.item(good?).map(|item| item.mesh.clone());
    // GFX-OBJ-3.2 (3): the ramp rows are drawn when the view makes the
    // figure, not here.
    let paint = |c: &s2_assets::model_tables::CharacterModel, player: s2_sim::PlayerId| {
        c.color
            .as_ref()
            .map(|color| crate::render::models3d::Paint {
                color: color.clone(),
                slot_map: c.slot_map.clone(),
                player: models3d::player_colour(player),
            })
    };
    let settler_scale = tables.character_scale;
    let settler_model = tables.settler_model.clone();
    let person = |id: MoverId,
                  player: s2_sim::PlayerId,
                  kind: &str,
                  at: [f32; 3],
                  heading: Option<f32>,
                  mut state: SettlerState,
                  look: Look| {
        let c = character(player, kind).or_else(|| character(player, "Carrier"))?;
        state.override_base = look.base;
        Some(Mover {
            id,
            file: c.mesh.clone(),
            units_per_metre: settler_scale,
            at,
            heading,
            tint: settler_tint(player),
            carried: mesh_of(look.held.as_deref()),
            boat: None,
            link: Some(HAND_SLOT.into()),
            skeleton: settler_model.clone(),
            anim: MoverAnim::Settler {
                state,
                good: look.good,
                profession: look.profession,
                tribe: world.tribe(player),
                donkey: false,
            },
            paint: paint(c, player),
            bob: false,
            family: models3d::Family::Plain,
            player: [0.0; 3],
        })
    };
    let views = world.settler_views();
    for view in &views {
        let SettlerRef::Settler(id) = view.id else {
            continue;
        };
        if view.hidden {
            continue;
        }
        let rank = world.soldier_rank(view.profession);
        let name = lookup.goods.get(&view.profession);
        let kind = match (rank, name) {
            (Some(rank), _) => soldier_kind(rank),
            (None, Some(name)) => name.clone(),
            (None, None) => continue,
        };
        // TRN-FREE-5: an outdoor worker's fine approach, work and taking of
        // the ware happen on the height lattice, up against the object.
        let (at, heading) = match &view.fine {
            Some(fine) => fine_point(world, frame, fine),
            None => walker_point(world, frame, &view.walk, None),
        };
        let mut state = walking_state(world, at, view.has_walk);
        if let Some(fine) = &view.fine {
            state.moving = fine.step.next.is_some();
        }
        // GFX-ANIM-3.1a, 3.8e: the free walk home, ordered in the tick the
        // fine walk back arrives, plays Walk in that tick.
        state.moving |= view.has_walk;
        // GFX-ANIM-3.1, 3.8: the worker's, the specialist's and the
        // fighting soldier's requests.
        state.request = view.request.and_then(request_of);
        state.inside = view.inside;
        state.dying = view.activity == Activity::Dying;
        // GFX-OBJ-7.4, 7.5, GFX-ANIM-1.3: the ware, else the job's tool
        // (a soldier's the Sword), held and picking the clips.
        let holder = if rank.is_some() {
            Holder::Soldier
        } else {
            Holder::Worker
        };
        let job = name.map_or("", String::as_str);
        let held = look_of(
            holder,
            job,
            view.carrying,
            state.moving,
            view.walk.at,
            false,
        );
        out.extend(person(
            MoverId::Settler(id),
            view.player,
            &kind,
            at,
            heading,
            state,
            held,
        ));
    }
    for view in &views {
        let SettlerRef::Carrier(id) = view.id else {
            continue;
        };
        if matches!(view.activity, Activity::Carrier { retired: true, .. }) {
            continue;
        }
        let player = view.player;
        let (mut at, heading) = walker_point(world, frame, &view.walk, None);
        let mut state = walking_state(world, at, view.has_walk);
        // A waterway's boat carrier stays on the water surface; he rows
        // (GFX-ANIM-3.4, GFX-OBJ-7.6).
        let rowing = view.on_water;
        if rowing {
            at[1] = at[1].max(WATER_LEVEL);
            state.under = false;
        }
        state.request = view.request.and_then(request_of);
        let ware = view.carrying;
        let ware_name = ware.and_then(|g| lookup.goods.get(&g)).map(String::as_str);
        let field = view.walk.at;
        match tables.donkey.as_ref().filter(|_| view.donkey) {
            Some(d) => out.push({
                let held = look_of(Holder::Donkey, "Donkey", ware, state.moving, field, false);
                Mover {
                    id: MoverId::Carrier(id),
                    file: d.mesh.clone(),
                    units_per_metre: d.scale,
                    at,
                    heading,
                    tint: WHITE,
                    carried: mesh_of(held.held.as_deref()),
                    boat: None,
                    link: d.link.clone(),
                    skeleton: d.model.clone(),
                    anim: MoverAnim::Settler {
                        state,
                        good: held.good,
                        profession: held.profession,
                        tribe: world.tribe(player),
                        donkey: true,
                    },
                    paint: None,
                    bob: false,
                    family: models3d::Family::Animal,
                    player: [0.0; 3],
                }
            }),
            None => {
                let held = look_of(
                    Holder::Carrier,
                    "Carrier",
                    ware,
                    state.moving,
                    field,
                    rowing,
                );
                out.extend(
                    person(
                        MoverId::Carrier(id),
                        player,
                        "Carrier",
                        at,
                        heading,
                        state,
                        held,
                    )
                    .map(|mut m| {
                        // GFX-OBJ-7.6: the ware lies in the boat.
                        if rowing {
                            m.carried = mesh_of(ware_name);
                            m.boat = boat.clone();
                        }
                        m
                    }),
                );
            }
        }
    }
    // The builder and the leveller of each site: the builder on the height
    // sample of his last walk (the site before the first), hammering while
    // a work unit runs; the leveller on the neighbour he levels, shovelling
    // while his action runs. Their walks are not drawn (a stand-in).
    // `work`: ticks since the work request was set, while it runs.
    let still = |p: [f32; 2], work: Option<u64>| {
        (
            [p[0], ground_height(world.terrain(), p[0], p[1]), p[1]],
            SettlerState {
                request: work.map(|_| Request::WorkLoop(0)),
                inside: false,
                moving: false,
                under: false,
                dying: false,
                override_base: None,
                started: work,
            },
        )
    };
    for view in &views {
        match view.id {
            SettlerRef::Builder(at) => {
                let Some(fine) = &view.fine else {
                    continue;
                };
                // ECO-HQ-4.6: he hammers at each construction position for a
                // step's action (GFX-ANIM-3.8 code 4, one request per step,
                // so the play-once clip starts again at every step, 3.8f a:
                // the step's first tick names the request) and walks between
                // them (the path is not kept: a straight line).
                let xz = |s: Point| frame.sample_xz(i64::from(s.0), i64::from(s.1));
                let (p, heading, moving) = match fine.step.next {
                    Some(to) => {
                        let (a, b) = (xz(fine.step.at), xz(to));
                        let f = fine.step.fraction();
                        let p = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                        let heading = (a != b).then(|| (b[0] - a[0]).atan2(b[1] - a[1]));
                        (p, heading, a != b)
                    }
                    None => (xz(fine.step.at), None, false),
                };
                let (pos, mut state) = still(p, view.request_age);
                state.moving = moving;
                out.extend(person(
                    MoverId::Builder(at),
                    view.player,
                    "Constructor",
                    pos,
                    heading,
                    state,
                    look_of(Holder::Builder, "Constructor", None, moving, at, false),
                ));
            }
            SettlerRef::Leveller(at) => {
                let field = view.walk.at;
                let (pos, state) = still(frame.field_xz(field.0, field.1), view.request_age);
                out.extend(person(
                    MoverId::Leveller(at),
                    view.player,
                    "Bulldozer",
                    pos,
                    None,
                    state,
                    look_of(Holder::Leveller, "Bulldozer", None, false, field, false),
                ));
            }
            _ => {}
        }
    }
    // GFX-ANIM-5.2: an animal dies when the hunter's action at it starts
    // (the logic removes it when the action ends; a stand-in for the dying
    // state, Q-181).
    let shot: std::collections::HashSet<Point> = views
        .iter()
        .filter(|v| {
            matches!(
                v.activity,
                Activity::Worker {
                    phase: WorkPhase::AtSpot,
                    ..
                }
            )
        })
        .filter_map(|v| v.work_spot)
        .collect();
    for (id, animal) in world.animals() {
        let Some(gfx) = lookup.animals.get(&animal.kind) else {
            continue;
        };
        let Some(model) = tables
            .animals
            .iter()
            .find(|a| a.kind.eq_ignore_ascii_case(gfx))
        else {
            continue;
        };
        let step = animal.step();
        let (at, heading) = walker_point(world, frame, &step, None);
        let dying = animal.hunted_by.is_some() && shot.contains(&step.at);
        let anim = match entry_file(&model.animations, "Dying").filter(|_| dying) {
            Some(file) => MoverAnim::Unique(file.to_owned()),
            None => MoverAnim::Clip(walk_or_stand(&model.animations, heading.is_some())),
        };
        out.push(Mover {
            id: MoverId::Animal(id),
            file: model.mesh.clone(),
            units_per_metre: model.scale,
            at,
            heading,
            tint: WHITE,
            carried: None,
            boat: None,
            link: None,
            skeleton: model.model.clone(),
            anim,
            paint: None,
            bob: false,
            family: models3d::Family::Animal,
            player: [0.0; 3],
        });
    }
    for (id, ship) in world.ships().iter().enumerate() {
        let tribe = models3d::TRIBE_NAMES[usize::from(world.tribe(ship.owner)).min(2)];
        let Some(model) = tables.ships.iter().find(|s| s.tribe == tribe) else {
            continue;
        };
        let (file, (at, heading)) = if ship.vessel.launched {
            (
                model.mesh.clone(),
                walker_point(world, frame, &ship.vessel.step(), Some(WATER_LEVEL)),
            )
        } else {
            let [x, z] = frame.field_xz(ship.at.0, ship.at.1);
            let site = model.frame.clone().unwrap_or_else(|| model.mesh.clone());
            let file = if ship.finished() {
                model.mesh.clone()
            } else {
                site
            };
            (file, ([x, ground_height(world.terrain(), x, z), z], None))
        };
        // GFX-ANIM-7.1: on the water the ship plays `move` while moving
        // and `idle` at rest, both looping, on its skeleton.
        let launched = ship.vessel.launched;
        let moving = ship.vessel.step().next.is_some();
        let clip = if moving { &model.moving } else { &model.idle };
        out.push(Mover {
            id: MoverId::Ship(id),
            file,
            units_per_metre: model.scale,
            at,
            heading,
            tint: WHITE,
            carried: None,
            boat: None,
            link: None,
            skeleton: model.model.clone().filter(|_| launched),
            anim: MoverAnim::Clip(clip.clone().filter(|_| launched).map(|c| (c, true))),
            paint: None,
            bob: launched,
            family: models3d::Family::Ship,
            player: models3d::player_colour(ship.owner),
        });
    }
    out
}

/// The tint of a settler drawn without its colour files.
pub(crate) fn settler_tint(player: s2_sim::PlayerId) -> [f32; 4] {
    let [r, g, b] = models3d::player_colour(player);
    [0.5 + r / 2.0, 0.5 + g / 2.0, 0.5 + b / 2.0, 1.0]
}
