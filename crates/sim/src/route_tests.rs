//! Test cases of `specs/TRN-roads-flags.md` TRN-ROAD-8 (T12 to T16): the
//! road built click by click.

use crate::grid::{self, Point};
use crate::map::state;
use crate::requests::tests::{self as req, apply_all, road_cmd, HQ, HQ_FLAG};
use crate::test_support::*;
use crate::{Command, GameSetup, Rejection, World};

const A: Point = HQ_FLAG;

fn game() -> World {
    let rules = req::rules(&req::full_stock());
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    World::new_game(9, flat(40, 40), rules, &setup)
}

fn run(world: &mut World, command: Command) -> Result<(), Rejection> {
    world.submit(command);
    world.step();
    world.last_results()[0].outcome
}

fn start(world: &mut World) {
    let at = A;
    assert_eq!(
        run(
            world,
            Command::StartRoute {
                player: P0,
                at,
                waterway: false,
            }
        ),
        Ok(())
    );
}

fn extend(world: &mut World, to: Point) -> Result<(), Rejection> {
    run(world, Command::ExtendRoute { player: P0, to })
}

/// A free field `n` fields east of the headquarters' flag.
fn east(n: u32) -> Point {
    (A.0 + n, A.1)
}

// TRN-ROAD T12, 8.1, 8.2: a route to a free field three fields away has
// four fields that carry Street; no road yet.
#[test]
fn t12_route_to_a_free_field() {
    let mut world = game();
    let roads = world.roads().count();
    start(&mut world);
    assert_eq!(extend(&mut world, east(3)), Ok(()));
    let route = world.route(P0).expect("the route");
    assert_eq!(route.fields.len(), 4, "{:?}", route.fields);
    assert_eq!((route.fields[0], route.end()), (A, east(3)));
    assert!(route.fields[1..]
        .iter()
        .all(|&f| world.terrain().has(f, state::STREET)));
    assert_eq!(world.roads().count(), roads);
    // 8.1: one route at a time.
    assert_eq!(
        run(
            &mut world,
            Command::StartRoute {
                player: P0,
                at: A,
                waterway: false,
            }
        ),
        Err(Rejection::RouteExists)
    );
}

// TRN-ROAD T13, 8.3: a flag on the route's end: the next tick the road
// exists and asks for a carrier, the route is gone.
#[test]
fn t13_a_flag_on_the_end_builds_the_road() {
    let mut world = game();
    start(&mut world);
    extend(&mut world, east(3)).unwrap();
    let requests = world.requests(P0).len();
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: east(3),
        }],
    );
    assert!(world.route(P0).is_none());
    let road = world.roads().last().unwrap();
    assert_eq!((road.path[0], *road.path.last().unwrap()), (A, east(3)));
    // TRN-ROAD-3.1 (T26): the end flag carries Street with its road.
    assert!(world.terrain().has(east(3), state::STREET));
    assert!(
        world.requests(P0).len() > requests,
        "a carrier is requested"
    );
}

// TRN-ROAD T14, 8.3: extended to an existing flag, the road is built at
// once.
#[test]
fn t14_a_click_on_a_flag_ends_the_road() {
    let mut world = game();
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: east(6),
        }],
    );
    start(&mut world);
    extend(&mut world, east(3)).unwrap();
    extend(&mut world, east(6)).unwrap();
    assert!(world.route(P0).is_none());
    let road = world.roads().last().unwrap();
    assert_eq!(road.path.len(), 7, "{:?}", road.path);
    assert_eq!(*road.path.last().unwrap(), east(6));
}

// TRN-ROAD T15, 8.2: a click on a field of the route cuts it back there;
// the dropped fields lose Street.
#[test]
fn t15_a_click_on_the_route_cuts_it_back() {
    let mut world = game();
    start(&mut world);
    extend(&mut world, east(3)).unwrap();
    let fields = world.route(P0).unwrap().fields.clone();
    let x = fields[1];
    extend(&mut world, x).unwrap();
    assert_eq!(world.route(P0).unwrap().fields, fields[..2].to_vec());
    assert!(world.terrain().has(x, state::STREET));
    assert!(fields[2..]
        .iter()
        .all(|&f| !world.terrain().has(f, state::STREET)));
    // `cancel-road` clears the rest.
    assert_eq!(run(&mut world, Command::CancelRoute { player: P0 }), Ok(()));
    assert!(world.route(P0).is_none());
    assert!(!world.terrain().has(x, state::STREET));
}

// TRN-ROAD T16, 8.3: a route ending on a field of another road where a
// flag fits: the flag is placed, that road is split, the road is built.
#[test]
fn t16_ending_on_another_road_splits_it() {
    let mut world = game();
    let (c, d) = ((A.0 + 4, A.1 - 3), (A.0 + 4, A.1 + 3));
    apply_all(
        &mut world,
        vec![
            Command::PlaceFlag { player: P0, at: c },
            Command::PlaceFlag { player: P0, at: d },
        ],
    );
    apply_all(&mut world, vec![road_cmd(c, d)]);
    let x = world
        .roads()
        .last()
        .unwrap()
        .path
        .iter()
        .copied()
        .find(|&f| grid::distance(f, c) >= 2 && grid::distance(f, d) >= 2)
        .expect("a middle field");
    start(&mut world);
    assert_eq!(extend(&mut world, x), Ok(()));
    assert!(world.route(P0).is_none());
    assert_eq!(world.flag_at(x), Some(P0));
    let ends: Vec<(Point, Point)> = world
        .roads()
        .map(|r| (r.path[0], *r.path.last().unwrap()))
        .collect();
    for pair in [(c, x), (x, d), (A, x)] {
        assert!(
            ends.contains(&pair) || ends.contains(&(pair.1, pair.0)),
            "{pair:?} in {ends:?}"
        );
    }
}

// TRN-ROAD-7.4: a field where no flag fits cannot end the road; a click
// there extends nothing.
#[test]
fn a_field_next_to_a_flag_extends_nothing() {
    let mut world = game();
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: east(5),
        }],
    );
    start(&mut world);
    assert_eq!(extend(&mut world, east(4)), Err(Rejection::FlagTooClose));
    assert_eq!(world.route(P0).unwrap().fields, vec![A]);
}

// TRN-ROAD-8.3: a route whose start flag is destroyed is dropped in the
// next tick, its fields cleared.
#[test]
fn the_route_goes_with_its_start_flag() {
    let mut world = game();
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: east(3),
        }],
    );
    world.submit(Command::StartRoute {
        player: P0,
        at: east(3),
        waterway: false,
    });
    world.step();
    extend(&mut world, east(6)).unwrap();
    let fields = world.route(P0).unwrap().fields.clone();
    apply_all(
        &mut world,
        vec![Command::RemoveFlag {
            player: P0,
            at: east(3),
        }],
    );
    world.step();
    assert!(world.route(P0).is_none());
    assert!(fields
        .iter()
        .all(|&f| !world.terrain().has(f, state::STREET)));
}
