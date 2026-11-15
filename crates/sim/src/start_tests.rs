//! Tests of `specs/SIM-game-start.md` (the start of a free game).

use crate::military_tests::{rules, CASTLE};
use crate::test_support::*;
use crate::{Command, GameSetup, PlayerId, Rng, World};

const P1: PlayerId = PlayerId(1);
const STARTS: [(u32, u32); 4] = [(10, 11), (30, 11), (10, 31), (30, 31)];

fn setup(shuffle: bool) -> GameSetup {
    GameSetup {
        players: vec![(P0, STARTS[0]), (P1, STARTS[1])],
        local_player: P0,
        shuffle_starts: shuffle,
        ..Default::default()
    }
}

fn castles(world: &World) -> Vec<(PlayerId, (u32, u32))> {
    world
        .buildings()
        .filter(|b| b.building_type == CASTLE)
        .map(|b| (b.owner, b.at))
        .collect()
}

// T4: start positions in order, no draw.
// spec: SIM-START-3.1 (T4), SIM-START-1.1
#[test]
fn start_t4_positions_in_slot_order() {
    let world = World::new_game(5, flat(50, 50), rules(), &setup(false));
    let mut got = castles(&world);
    got.sort();
    assert_eq!(got, vec![(P0, STARTS[0]), (P1, STARTS[1])]);
    assert_eq!(world.rng().draws(), 0);
}

// ECO-PLACE T17 (ECO-PLACE-5.3, Q-295): the start castles remove the map
// decorations of their footprint and flag fields in the first update (gone
// at counter 0 in E-trace-006), not before.
// spec: ECO-PLACE-5.3 (ECO-PLACE T17)
#[test]
fn t17_start_castles_clear_their_decorations_in_the_first_update() {
    let mut world = World::new_game(5, flat(50, 50), rules(), &setup(false));
    assert!(world.decorations_cleared().is_empty());
    world.step();
    for at in STARTS[..2].iter().copied() {
        let size = world.building_at(at).unwrap().size;
        let mut fields = world.footprint_of(at, size);
        fields.push(world.terrain().neighbour(at, 1).unwrap());
        assert_eq!(world.decoration_cleared_fields(at), fields, "{at:?}");
    }
    let cleared = world.decorations_cleared().clone();
    world.step();
    assert_eq!(
        world.decorations_cleared(),
        &cleared,
        "only the first update"
    );
}

// T5: shuffled, each player in slot order draws among the positions left;
// the draws are the game's first (the reset comes before them).
// spec: SIM-START-3.2 (T5), SIM-START-1.1, SIM-RNG-2.2
#[test]
fn start_t5_shuffled_positions_draw_in_slot_order() {
    let mut s = setup(true);
    s.players = vec![(P1, STARTS[1]), (P0, STARTS[0]), (PlayerId(2), STARTS[2])];
    s.players.push((PlayerId(3), STARTS[3]));
    let world = World::new_game(5, flat(50, 50), rules(), &s);
    // The game seed of `new_game` (5 here; 0x5432ACE0 in a real game).
    let mut rng = Rng::new(5);
    let mut free = vec![STARTS[1], STARTS[0], STARTS[2], STARTS[3]];
    let mut want = Vec::new();
    for p in 0..4 {
        let k = rng.draw() as usize % free.len();
        want.push((PlayerId(p), free.remove(k)));
    }
    let mut got = castles(&world);
    got.sort();
    assert_eq!(got, want);
    assert_eq!(world.rng().draws(), 4);
}

// T2 (generator) and 1.3: nothing drawn through the first ticks of a game
// without animals; the garrison soldier is made by the first update.
// spec: SIM-START-1.3, SIM-START-2.2 (T2)
#[test]
fn start_t2_first_update_makes_the_garrison() {
    let mut world = World::new_game(5, flat(50, 50), rules(), &setup(false));
    assert_eq!(world.settlers().count(), 0);
    world.step();
    assert_eq!(world.settlers().count(), 2);
    world.step();
    assert_eq!(world.rng().draws(), 0);
}

/// The test rules with the castle's door north-west (ECO-CAT-2.4).
fn rules_with_door() -> crate::Rules {
    let mut rules = rules();
    for t in &mut rules.building_types {
        if t.id == CASTLE {
            t.doors = [Some(4); 3];
        }
    }
    rules
}

fn game_at(at: (u32, u32)) -> World {
    let setup = GameSetup {
        players: vec![(P0, at)],
        local_player: P0,
        ..Default::default()
    };
    World::new_game(5, flat(80, 80), rules_with_door(), &setup)
}

// MIL T16, T17: the present soldier stands on the door field at its sample,
// with Stand, from the end of the first update.
// spec: MIL-SOL-6.5 (T16, T17), ECO-WORK-3.8a, SIM-START-1.3
#[test]
fn mil_t16_present_soldier_on_the_door_field() {
    for (castle, door) in [((29, 51), (29, 52)), ((40, 40), (39, 41))] {
        let mut world = game_at(castle);
        world.step();
        let rows = world.trace_settlers();
        assert_eq!(rows.len(), 1);
        let s = &rows[0];
        assert_eq!(s.field, door);
        let sample = crate::grid::field_sample(door);
        assert_eq!(
            s.fine,
            (i64::from(sample.0) * 1000, i64::from(sample.1) * 1000)
        );
        assert_eq!(s.action, crate::trace::Action::Stand);
    }
}

// ECO-WORK T-door-1: a settler sent out of the castle appears on its door
// field and steps back onto the castle's field first, never starting on it.
// spec: ECO-WORK-3.8a (T-door-1), ECO-REQ-3.6
#[test]
fn work_t_door_1_leaving_by_the_door() {
    let castle = (29, 51);
    let world = game_at(castle);
    let walker = world.walker_leaving(castle, vec![(30, 50), (31, 50)]);
    assert_eq!(walker.at, (29, 52));
    assert_eq!(walker.path.front(), Some(&castle));
    // Without a door the walk starts on the building's field.
    let plain = World::new_game(5, flat(50, 50), rules(), &setup(false));
    let walker = plain.walker_leaving(STARTS[0], vec![(11, 10)]);
    assert_eq!(walker.at, STARTS[0]);
    assert_eq!(walker.path.front(), Some(&(11, 10)));
}

// ECO-WORK-3.8b: the door step is a walk of its own: in the tick it
// completes the walker stands on the building's field with its next step at
// progress 0, and advances from the next tick (0.045 on the level).
// spec: ECO-WORK-3.8b (T-door-2), ECO-WORK-3.8c, TRN-MOVE-2.1
#[test]
fn work_3_8b_the_door_step_is_a_walk_of_its_own() {
    let castle = (29, 51);
    let world = game_at(castle);
    let mut walker = world.walker_leaving(castle, vec![(30, 50), (31, 50)]);
    let mut ticks = 0;
    while walker.at != castle {
        walker.advance(world.terrain(), 45_000, |_| true);
        ticks += 1;
    }
    assert_eq!(ticks, 23);
    assert_eq!(walker.next, Some((30, 50)));
    assert_eq!(walker.progress(), 0.0);
    walker.advance(world.terrain(), 45_000, |_| true);
    assert_eq!(walker.progress(), 0.045);
    // ECO-WORK-3.8c: on the own flag the road walk starts at 0 again.
    while walker.at != (30, 50) {
        walker.advance(world.terrain(), 45_000, |_| true);
    }
    assert_eq!((walker.next, walker.progress()), (Some((31, 50)), 0.0));
}

// ECO-WORK-3.8b, entering: a settler going home ends its walk on the
// warehouse's flag and on its field (each a walk of its own, the next from
// progress 0), walks the door step and stands on the door field in its
// arrival tick; it is gone (in the stock) in the next.
// spec: ECO-WORK-3.8b, ECO-REQ-5.3c
#[test]
fn work_3_8b_entering_by_the_door() {
    let castle = (29, 51);
    let mut world = game_at(castle);
    let flag = world.flag_of(castle);
    let door = world.door_field(castle);
    let profession = world.roles.helper.unwrap();
    let before = world.warehouse(castle).unwrap().count(profession);
    let mut settler = crate::Settler {
        number: 0,
        player: P0,
        profession,
        list: crate::SettlerList::Carriers,
        walker: crate::movement::Walker::new(flag),
        job: crate::SettlerJob::Dying,
        timer: 0,
        carrying: None,
    };
    world.plan_home_excluding(&mut settler, None);
    assert_eq!(settler.walker.path, [castle, door]);
    // It starts on the flag: the walk's only pause is the castle's field.
    assert_eq!(settler.walker.pauses, [castle]);
    let id = world.add_settler(settler);
    let mut ticks = 0;
    while world
        .settlers()
        .find(|(n, _)| *n == id)
        .unwrap()
        .1
        .walker
        .at
        != castle
    {
        world.step();
        ticks += 1;
    }
    let s = world.settlers().find(|(n, _)| *n == id).unwrap().1;
    assert_eq!((s.walker.next, s.walker.progress()), (Some(door), 0.0));
    while world
        .settlers()
        .find(|(n, _)| *n == id)
        .unwrap()
        .1
        .walker
        .at
        != door
    {
        world.step();
    }
    world.step();
    assert!(world.settlers().all(|(n, _)| n != id), "gone after {ticks}");
    assert_eq!(
        world.warehouse(castle).unwrap().count(profession),
        before + 1
    );
}

// T3 without a computer player: from the counter's start value each castle
// takes one number and its flag four (its own and its three network
// entries), in slot order; the garrison soldiers, made by the first
// update, one each. (With a computer player in slot 1 its castle's cell
// takes one more, so the soldiers are 90186 and 90187 as in the original:
// `crates/ai/tests/object_numbers.rs`.)
// spec: SIM-START-4.1, SIM-START-4.2 (T3), SIM-START-1.3
#[test]
fn start_t3_the_start_objects_are_numbered_in_creation_order() {
    let setup = GameSetup {
        players: vec![(P0, (29, 51)), (P1, (60, 51))],
        local_player: P0,
        next_object: 90175,
        ..Default::default()
    };
    let mut world = World::new_game(5, flat(100, 80), rules_with_door(), &setup);
    let mut castles: Vec<(PlayerId, u32)> = world
        .buildings()
        .filter(|b| b.building_type == CASTLE)
        .map(|b| (b.owner, b.serial))
        .collect();
    castles.sort();
    assert_eq!(castles, [(P0, 90175), (P1, 90180)]);
    assert_eq!(world.next_object(), 90185);
    world.step();
    let soldiers: Vec<String> = world.trace_settlers().into_iter().map(|s| s.id).collect();
    assert_eq!(soldiers, ["90185", "90186"]);
    assert_eq!(world.next_object(), 90187);
}

// A flag takes four numbers; a road between two networks its own and one
// for each of the joined boat and carrier networks; a road inside one
// network only its own. (A test world has no helper, so no road files a
// carrier request.)
// spec: SIM-START-4.2, SIM-START-4.3
#[test]
fn start_4_3_a_road_joining_two_networks_renumbers_them() {
    let mut world = world(flat(40, 40));
    let taken = |world: &mut World, command: crate::Command| {
        let before = world.next_object();
        assert_eq!(apply(world, command), Ok(()));
        world.next_object() - before
    };
    let (a, b, c) = ((10, 10), (16, 10), (13, 16));
    for at in [a, b, c] {
        assert_eq!(taken(&mut world, Command::PlaceFlag { player: P0, at }), 4);
    }
    let road = |from, to| Command::BuildRoad {
        player: P0,
        from,
        to,
    };
    assert_eq!(taken(&mut world, road(a, b)), 1 + 2);
    assert_eq!(taken(&mut world, road(b, c)), 1 + 2);
    // c and a are in one network now.
    assert_eq!(taken(&mut world, road(c, a)), 1);
}

// A request is a number when it is filed, not again while it is open;
// material ordered files no request and takes none.
// spec: SIM-START-4.2, SIM-START-4.4, ECO-REQ-2.1
#[test]
fn start_4_4_requests_take_numbers_when_filed_material_none() {
    let mut world = world(flat(40, 40));
    assert_eq!(flag(&mut world, (10, 10)), Ok(()));
    assert_eq!(flag(&mut world, (16, 10)), Ok(()));
    assert_eq!(road(&mut world, (10, 10), (16, 10)), Ok(()));
    let requester = crate::Requester::Road(0);
    let before = world.next_object();
    world.request(P0, requester, 77);
    assert_eq!(world.next_object(), before + 1);
    world.request(P0, requester, 77);
    assert_eq!(world.next_object(), before + 1);
    assert_eq!(building(&mut world, SMALL, (20, 20)), Ok(()));
    let before = world.next_object();
    world.order((20, 20), 5);
    world.order((20, 20), 5);
    assert_eq!(world.next_object(), before);
}

// SIM-START-4.4 b (190c1e1, T13): a computer player's site takes its
// number, then its records' numbers, then its flag's four: the
// `PlanBuilding` command with 2 records takes 7 numbers, the building's
// the first; a plain `PlaceBuilding` 5.
// spec: SIM-START-4.4 (T13), SIM-START-4.2
#[test]
fn start_4_4_a_computer_players_site_numbers_its_records_at_once() {
    let mut world = world(flat(40, 40));
    let before = world.next_object();
    assert_eq!(building(&mut world, SMALL, (10, 10)), Ok(()));
    assert_eq!(world.next_object(), before + 5);
    let before = world.next_object();
    let planned = Command::PlanBuilding {
        player: P0,
        building: SMALL,
        at: (20, 20),
        records: 2,
    };
    assert_eq!(apply(&mut world, planned), Ok(()));
    assert_eq!(world.next_object(), before + 7);
    assert_eq!(world.building_at((20, 20)).unwrap().serial, before);
}

// SIM-START-4.3 c: a removed road with its carrier renumbers the donkey
// network of each end, then the boat and the carrier network of one end:
// four numbers, whether or not it cut its network in two.
// spec: SIM-START-4.3
#[test]
fn start_4_3_a_removed_road_renumbers_one_end() {
    let mut world = world(flat(40, 40));
    let (a, b, c) = ((10, 10), (16, 10), (13, 16));
    for at in [a, b, c] {
        assert_eq!(flag(&mut world, at), Ok(()));
    }
    for (from, to) in [(a, b), (b, c), (c, a)] {
        assert_eq!(road(&mut world, from, to), Ok(()));
    }
    for (from, to) in [(a, b), (b, c)] {
        let road = world
            .roads()
            .find(|r| r.path[0] == from && r.path[r.path.len() - 1] == to)
            .expect("the road");
        assert!(road.carrier.is_some());
        let inner = road.path[1];
        let before = world.next_object();
        let removed = Command::RemoveRoad {
            player: P0,
            at: inner,
        };
        assert_eq!(apply(&mut world, removed), Ok(()));
        assert_eq!(world.next_object(), before + 4);
    }
}

// SIM-START-4.4 j (Q-263 b), T11: a flag put on a road with its carrier and
// no donkey, no other road between its ends: the road's removal (donkey ids
// of both ends, boat and carrier of one end), the flag and its three ids,
// the first half from the old first flag (its number and both joins), the
// second from the old second flag (likewise), then the carrier's half joins
// the donkey networks: 15 numbers, as 90324 .. 90338 in T11.
// spec: SIM-START-4.4 (T11), SIM-START-4.3, TRN-ROAD-4.2
#[test]
fn start_4_4_j_a_flag_on_a_road_numbers_its_split() {
    let mut world = world(flat(40, 40));
    let (a, b, at) = ((10, 10), (16, 10), (13, 10));
    assert_eq!(flag(&mut world, a), Ok(()));
    assert_eq!(flag(&mut world, b), Ok(()));
    assert_eq!(road(&mut world, a, b), Ok(()));
    let before = world.next_object();
    assert_eq!(flag(&mut world, at), Ok(()));
    assert_eq!(world.next_object() - before, 2 + 2 + 4 + 3 + 3 + 1);
    let mut halves: Vec<Vec<crate::grid::Point>> = world.roads().map(|r| r.path.clone()).collect();
    halves.sort();
    assert_eq!(halves.len(), 2);
    assert_eq!((halves[0][0], *halves[0].last().unwrap()), (a, at));
    assert_eq!(
        (halves[1][0], *halves[1].last().unwrap()),
        (b, at),
        "from the second flag"
    );
    // The carrier keeps its work on one half; the other half has none.
    let carried = world.roads().filter(|r| r.carrier.is_some()).count();
    assert_eq!(carried, 1);
}
