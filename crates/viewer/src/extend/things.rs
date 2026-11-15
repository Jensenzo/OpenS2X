//! The things on the map for a project (`Extensions::map_things`,
//! `docs/extending.md`): the settlers, wares and animals under the pointer
//! and where each is drawn, read only. Not part of the original game; off
//! by default, and then nothing here is computed.

use crate::world::scene3d::MoverId;
use bevy::prelude::*;
use s2_sim::view::SettlerRef;
use std::collections::HashMap;

/// Something on the map, by the simulation's numbers (`docs/api.md`):
/// wares and animals are only ever added, so a number stays with its thing
/// all game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MapThing {
    /// A settler, road carrier (a donkey and a boat carrier too), builder
    /// or leveller (`World::settler_views`).
    Settler(SettlerRef),
    /// A ware lying at a flag or carried (`World::wares`).
    Ware(usize),
    /// A game animal (`World::animals`).
    Animal(usize),
}

/// A thing under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapHit {
    pub thing: MapThing,
    /// How far along the pointer's ray from the camera it enters the
    /// thing's box, in world units.
    pub distance: f32,
    /// The ware a settler carries (a road carrier's); `None` for an empty
    /// hand and for wares and animals.
    pub carrying: Option<usize>,
}

/// The things on the map this frame (a resource the viewer keeps while
/// `Extensions::map_things` is true), filled after the frame's movers and
/// wares are placed: a system in `PreUpdate` reads what the previous frame
/// drew.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MapThings {
    /// The pointer is over the map: in the window, over no part of the
    /// interface, no menu or modal dialog open, the 3D view on.
    pub over_map: bool,
    /// The things whose box the pointer's ray crosses, on fields the player
    /// has explored, any owner, nearest first. Empty while `over_map` is
    /// false.
    pub under_pointer: Vec<MapHit>,
    /// Where each thing is drawn (Bevy's world coordinates, those of
    /// [`CameraFollow`](super::CameraFollow)): a settler's or animal's
    /// feet, a lying ware at its stack place, a carried ware where it is
    /// held. A thing inside a building, used up, dead or not drawn is not
    /// here.
    pub positions: HashMap<MapThing, Vec3>,
}

impl MapThings {
    /// Where `thing` is drawn this frame.
    pub fn position(&self, thing: MapThing) -> Option<Vec3> {
        self.positions.get(&thing).copied()
    }
}

/// UI-GAME-9.3's box of a settler (`pick_map`): 0.8 wide and 2 high,
/// standing where he is drawn.
const SETTLER_HALF: Vec3 = Vec3::new(0.4, 1.0, 0.4);
/// A ware's box: 0.7 on each side, standing on the ware's point.
const WARE_HALF: Vec3 = Vec3::splat(0.35);
/// An animal's box follows its model's size, kept between these halves.
const ANIMAL_HALF: (f32, f32) = (0.25, 3.0);

/// A thing as drawn this frame: its point and its box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Drawn {
    pub(crate) thing: MapThing,
    pub(crate) at: Vec3,
    pub(crate) half: Vec3,
    pub(crate) carrying: Option<usize>,
}

impl Drawn {
    fn standing(thing: MapThing, at: Vec3, half: Vec3) -> Self {
        Self {
            thing,
            at,
            half,
            carrying: None,
        }
    }

    /// The box stands on the point.
    fn centre(&self) -> Vec3 {
        self.at + Vec3::Y * self.half.y
    }
}

/// A mover as drawn: its id, its transform, the local transform of what it
/// holds, its model's size.
pub(crate) type ShownMover = (MoverId, Transform, Option<Transform>, Option<[f32; 3]>);

/// The road carriers' wares: carrier number to ware number.
pub(crate) fn carried_wares(world: &s2_sim::World) -> HashMap<usize, usize> {
    world
        .wares()
        .filter_map(|(id, w)| match w.location {
            s2_sim::WareLocation::Carried(c) => Some((c, id)),
            s2_sim::WareLocation::Flag(_) => None,
        })
        .collect()
}

/// Every thing drawn: the movers but ships, each carrier's ware where he
/// holds it (at him when it has no model of its own) and the wares lying
/// at flags.
pub(crate) fn drawn(
    movers: impl IntoIterator<Item = ShownMover>,
    lying: impl IntoIterator<Item = (usize, Transform)>,
    carried: &HashMap<usize, usize>,
) -> Vec<Drawn> {
    let mut out = Vec::new();
    for (id, t, held, extent) in movers {
        let at = t.translation;
        let who = match id {
            MoverId::Settler(n) => SettlerRef::Settler(n),
            MoverId::Carrier(n) => SettlerRef::Carrier(n),
            MoverId::Builder(p) => SettlerRef::Builder(p),
            MoverId::Leveller(p) => SettlerRef::Leveller(p),
            MoverId::Animal(n) => {
                let [x, y, z] = extent.unwrap_or([0.0; 3]);
                let (low, high) = ANIMAL_HALF;
                let side = (x.max(z) / 2.0).clamp(low, high);
                let half = Vec3::new(side, (y / 2.0).clamp(low, high), side);
                out.push(Drawn::standing(MapThing::Animal(n), at, half));
                continue;
            }
            MoverId::Ship(_) => continue,
        };
        let ware = match id {
            MoverId::Carrier(n) => carried.get(&n).copied(),
            _ => None,
        };
        out.push(Drawn {
            carrying: ware,
            ..Drawn::standing(MapThing::Settler(who), at, SETTLER_HALF)
        });
        if let Some(ware) = ware {
            let held = held.map_or(at + Vec3::Y, |h| t.mul_transform(h).translation);
            out.push(Drawn::standing(MapThing::Ware(ware), held, WARE_HALF));
        }
    }
    for (ware, t) in lying {
        out.push(Drawn::standing(
            MapThing::Ware(ware),
            t.translation,
            WARE_HALF,
        ));
    }
    out
}

/// The frame's things: every one's position, and with the pointer's ray
/// over the map (`pointer`: origin and direction) the ones it crosses that
/// `seen` lets through, nearest first.
pub(crate) fn things_of(
    drawn: &[Drawn],
    pointer: Option<(Vec3, Vec3)>,
    seen: impl Fn(MapThing) -> bool,
) -> MapThings {
    let positions = drawn.iter().map(|d| (d.thing, d.at)).collect();
    let mut under_pointer: Vec<MapHit> = pointer
        .into_iter()
        .flat_map(|(origin, direction)| {
            drawn.iter().filter_map(move |d| {
                let distance =
                    crate::render::view3d::ray_box(origin, direction, d.centre(), d.half)?;
                Some(MapHit {
                    thing: d.thing,
                    distance,
                    carrying: d.carrying,
                })
            })
        })
        .filter(|hit| seen(hit.thing))
        .collect();
    under_pointer.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then(a.thing.cmp(&b.thing))
    });
    MapThings {
        over_map: pointer.is_some(),
        under_pointer,
        positions,
    }
}

/// The field a thing is on, for the player's exploration.
fn field_of(world: &s2_sim::World, thing: MapThing) -> Option<s2_sim::grid::Point> {
    match thing {
        MapThing::Settler(r) => world.settler_view(r).map(|v| v.walk.at),
        MapThing::Animal(n) => world.animal(n).map(|a| a.at),
        MapThing::Ware(n) => match world.ware(n)?.location {
            s2_sim::WareLocation::Flag(p) => Some(p),
            s2_sim::WareLocation::Carried(c) => {
                field_of(world, MapThing::Settler(SettlerRef::Carrier(c)))
            }
        },
    }
}

/// Keep [`MapThings`] after the frame's movers and wares are placed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn keep_map_things(
    viewer: Res<crate::game::Viewer>,
    view: Res<crate::render::view3d::View3d>,
    movers: Res<crate::render::view3d::Movers>,
    wares: Res<crate::render::view3d::Wares3d>,
    menu: Res<crate::menus::menu::MenuState>,
    mission: Res<crate::menus::campaign::MissionUi>,
    ui: (
        Res<crate::ui::render::UiEvents>,
        Res<crate::ui::render::UiFrame>,
    ),
    window: Single<&Window>,
    camera: Single<(&Camera, &Transform), With<crate::render::view3d::MapCamera3d>>,
    transforms: Query<&Transform, Without<crate::render::view3d::MapCamera3d>>,
    mut things: ResMut<MapThings>,
) {
    let world = viewer
        .world
        .as_ref()
        .filter(|_| view.is_built(viewer.loads));
    let Some(world) = world else {
        if *things != MapThings::default() {
            *things = MapThings::default();
        }
        return;
    };
    let carried = carried_wares(world);
    let shown = movers.states.iter().filter_map(|(id, s)| {
        let t = *transforms.get(s.entity).ok()?;
        let held = s
            .carried
            .as_ref()
            .and_then(|(_, e)| transforms.get(*e).ok().copied());
        Some((*id, t, held, s.model.as_ref().map(|m| m.extent)))
    });
    let lying = wares
        .shown
        .iter()
        .filter_map(|(id, (e, _))| Some((*id, *transforms.get(*e).ok()?)));
    let drawn = drawn(shown, lying, &carried);
    let (events, frame) = ui;
    let pointer = window.cursor_position().filter(|c| {
        !events.over_ui
            && !frame.covers(c.to_array())
            && menu.screen == crate::menus::menu::Screen::None
            && mission.book.is_none()
            && mission.end.is_none()
    });
    let (camera, transform) = *camera;
    let ray = pointer.and_then(|c| {
        let ray = camera
            .viewport_to_world(&GlobalTransform::from(*transform), c)
            .ok()?;
        Some((ray.origin, ray.direction.as_vec3()))
    });
    *things = things_of(&drawn, ray, |thing| {
        field_of(world, thing).is_some_and(|f| !viewer.fogged(world, f))
    });
}

/// The hook in the app: with `on` the resource and its system after the
/// frame's movers and wares; without it nothing.
pub(crate) fn add(app: &mut App, on: bool) {
    if !on {
        return;
    }
    app.init_resource::<MapThings>().add_systems(
        Update,
        keep_map_things
            .after(crate::render::view3d::sync_movers)
            .after(crate::render::view3d::sync_wares)
            .run_if(in_state(crate::tools::model::ViewMode::Map)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARRIER: MapThing = MapThing::Settler(SettlerRef::Carrier(4));
    const DEER: MapThing = MapThing::Animal(2);
    /// What the carrier holds, in his frame (as GFX-OBJ-7.2's bone puts it).
    const HAND: Vec3 = Vec3::new(0.3, 1.2, 0.2);

    /// A carrier at `x` holding ware 7 (or nothing once it is delivered),
    /// a deer, a stone lying at a flag and a ship.
    fn scene(x: f32, holding: bool) -> Vec<Drawn> {
        let carrier = Transform::from_xyz(x, 2.0, -10.0);
        let carried = if holding {
            HashMap::from([(4, 7)])
        } else {
            HashMap::new()
        };
        drawn(
            [
                (
                    MoverId::Carrier(4),
                    carrier,
                    holding.then(|| Transform::from_translation(HAND)),
                    Some([0.5, 1.8, 0.5]),
                ),
                (
                    MoverId::Animal(2),
                    Transform::from_xyz(20.0, 2.0, -10.0),
                    None,
                    Some([1.6, 1.4, 0.6]),
                ),
                (
                    MoverId::Ship(0),
                    Transform::from_xyz(40.0, 0.0, -10.0),
                    None,
                    None,
                ),
            ],
            [(9, Transform::from_xyz(15.0, 2.0, -10.0))],
            &carried,
        )
    }

    /// The pointer's ray from a camera up and behind, onto `at`.
    fn ray_onto(at: Vec3) -> (Vec3, Vec3) {
        let eye = at + Vec3::new(0.0, 20.0, 20.0);
        (eye, (at - eye).normalize())
    }

    fn first(things: &MapThings) -> Option<MapThing> {
        things.under_pointer.first().map(|h| h.thing)
    }

    /// The hook W9 (docs/extending.md): over each thing it is the first
    /// under the pointer, a carrier with his ware; over the interface
    /// nothing; the carried ware's position goes with the carrier and is
    /// gone once it is delivered.
    // spec: none (the map things hook; the settler's box is UI-GAME-9.3's)
    #[test]
    fn the_things_under_the_pointer_and_where_they_are() {
        let all = |_| true;
        let things = things_of(
            &scene(10.0, true),
            Some(ray_onto(Vec3::new(20.0, 2.5, -10.0))),
            all,
        );
        assert!(things.over_map);
        assert_eq!(first(&things), Some(DEER));
        assert_eq!(things.under_pointer.len(), 1);

        // The carrier's legs: the carrier, carrying ware 7.
        let things = things_of(
            &scene(10.0, true),
            Some(ray_onto(Vec3::new(10.0, 2.3, -10.0))),
            all,
        );
        let hit = things.under_pointer[0];
        assert_eq!((hit.thing, hit.carrying), (CARRIER, Some(7)));
        // His ware where he holds it.
        let held = Vec3::new(10.0, 2.0, -10.0) + HAND;
        let things = things_of(
            &scene(10.0, true),
            Some(ray_onto(held + Vec3::Y * 0.3)),
            all,
        );
        assert_eq!(first(&things), Some(MapThing::Ware(7)));
        assert!(things.under_pointer.iter().any(|h| h.thing == CARRIER));
        // The stone at the flag; the ship is no thing.
        let things = things_of(
            &scene(10.0, true),
            Some(ray_onto(Vec3::new(15.0, 2.2, -10.0))),
            all,
        );
        assert_eq!(first(&things), Some(MapThing::Ware(9)));
        let things = things_of(
            &scene(10.0, true),
            Some(ray_onto(Vec3::new(40.0, 0.5, -10.0))),
            all,
        );
        assert!(things.over_map && things.under_pointer.is_empty());

        // Nearest first: a ray along the row meets the carrier before the
        // deer behind him.
        let along = (Vec3::new(0.0, 2.5, -10.0), Vec3::X);
        let things = things_of(&scene(10.0, true), Some(along), all);
        let order: Vec<_> = things.under_pointer.iter().map(|h| h.thing).collect();
        assert_eq!(order, [CARRIER, MapThing::Ware(9), DEER]);
        assert!(things.under_pointer[0].distance < things.under_pointer[2].distance);

        // Unexplored fields hide what is on them.
        let things = things_of(&scene(10.0, true), Some(along), |t| t != DEER);
        assert!(things.under_pointer.iter().all(|h| h.thing != DEER));

        // Over the interface (or a menu open): nothing under the pointer,
        // the positions kept.
        let things = things_of(&scene(10.0, true), None, all);
        assert!(!things.over_map && things.under_pointer.is_empty());
        assert_eq!(things.position(MapThing::Ware(7)), Some(held));
        assert_eq!(things.position(DEER), Some(Vec3::new(20.0, 2.0, -10.0)));
        assert_eq!(
            things.position(MapThing::Ware(9)),
            Some(Vec3::new(15.0, 2.0, -10.0))
        );
        assert_eq!(things.positions.len(), 4, "carrier, ware 7, deer, stone");

        // The carrier walks on: the ware with him. Delivered: gone.
        let later = things_of(&scene(12.5, true), None, all);
        assert_eq!(
            later.position(MapThing::Ware(7)),
            Some(held + Vec3::X * 2.5)
        );
        assert_eq!(later.position(CARRIER), Some(Vec3::new(12.5, 2.0, -10.0)));
        let delivered = things_of(&scene(13.0, false), None, all);
        assert_eq!(delivered.position(MapThing::Ware(7)), None);
        assert!(delivered.position(CARRIER).is_some());
    }

    /// A carrier's ware without a model of its own is at him.
    // spec: none (the map things hook)
    #[test]
    fn a_ware_without_its_model_is_at_its_carrier() {
        let drawn = drawn(
            [(
                MoverId::Carrier(1),
                Transform::from_xyz(5.0, 0.0, 5.0),
                None,
                None,
            )],
            [],
            &HashMap::from([(1, 3)]),
        );
        let things = things_of(&drawn, None, |_| true);
        assert_eq!(
            things.position(MapThing::Ware(3)),
            Some(Vec3::new(5.0, 1.0, 5.0))
        );
    }

    /// Off by default: no resource and no system; on, both.
    // spec: none (a hook off by default computes nothing)
    #[test]
    fn with_the_default_nothing_is_computed() {
        let count = |app: &mut App| app.get_schedule_mut(Update).map_or(0, |s| s.systems_len());
        let mut app = App::new();
        let before = count(&mut app);
        add(&mut app, crate::extend::Extensions::default().map_things);
        assert!(!app.world().contains_resource::<MapThings>());
        assert_eq!(count(&mut app), before);
        add(&mut app, true);
        assert!(app.world().contains_resource::<MapThings>());
        assert_eq!(count(&mut app), before + 1);
    }
}
