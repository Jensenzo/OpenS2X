//! Tests of `specs/ECO-food-mining.md` ECO-FOOD-5.7 to 5.11 (animals roam).

use crate::grid::{self, Point};
use crate::test_support::*;
use crate::{AnimalKind, AnimalState, BuildingSize, BuildingType, Rng, Rules, World};

const DEER: u32 = 1;
const RABBIT: u32 = 2;

fn animal_world() -> World {
    animal_world_of(40, 40)
}

fn animal_world_of(width: u32, height: u32) -> World {
    let mut deer = AnimalKind::new(DEER);
    deer.range = 6;
    deer.move_distance = 2;
    deer.speed = 45_000;
    let mut rabbit = AnimalKind::new(RABBIT);
    rabbit.range = 8;
    rabbit.move_distance = 4;
    rabbit.speed = 70_000;
    let rules = Rules {
        building_types: vec![BuildingType::new(SMALL, BuildingSize::Small)],
        animals: vec![deer, rabbit],
        ..Rules::empty()
    };
    World::with_map(7, flat(width, height), rules, P0)
}

fn state(world: &World, id: usize) -> AnimalState {
    world.animal(id).unwrap().state
}

// T24: a wait of 5 lasts 6 updates; the 2nd choosing update chooses.
// spec: ECO-FOOD-5.8, ECO-FOOD-5.9 (T24)
#[test]
fn food_t24_waiting_then_choosing() {
    let mut world = animal_world();
    let id = world.add_animal(DEER, (20, 20));
    world.animals[id].as_mut().unwrap().state = AnimalState::Waiting { left: 5 };
    for left in (0..5).rev() {
        world.step();
        assert_eq!(state(&world, id), AnimalState::Waiting { left });
    }
    world.step();
    assert_eq!(state(&world, id), AnimalState::Choosing { ticket: -1 });
    world.step();
    assert_eq!(state(&world, id), AnimalState::Choosing { ticket: 0 });
    world.step();
    assert!(matches!(state(&world, id), AnimalState::Walking { .. }));
}

// T25: at most two animals start per tick; the third uses `top` next tick.
// spec: ECO-FOOD-5.9 (T25)
#[test]
fn food_t25_two_starts_per_tick() {
    let mut world = animal_world();
    let ids: Vec<usize> = [(10, 10), (20, 20), (30, 30)]
        .into_iter()
        .map(|at| world.add_animal(DEER, at))
        .collect();
    world.step();
    for &id in &ids {
        assert_eq!(state(&world, id), AnimalState::Choosing { ticket: -1 });
    }
    world.step();
    for &id in &ids {
        assert_eq!(state(&world, id), AnimalState::Choosing { ticket: 0 });
    }
    world.step();
    assert!(matches!(state(&world, ids[0]), AnimalState::Walking { .. }));
    assert!(matches!(state(&world, ids[1]), AnimalState::Walking { .. }));
    assert_eq!(state(&world, ids[2]), AnimalState::Choosing { ticket: 1 });
    world.step();
    assert!(matches!(state(&world, ids[2]), AnimalState::Walking { .. }));
    assert_eq!(world.animal_top, 1);
}

/// A seed whose next draw gives `wanted` modulo `modulus`.
fn rng_giving(wanted: u64, modulus: u64) -> Rng {
    (1..)
        .map(Rng::new)
        .find(|r| u64::from(r.clone().draw()) % modulus == wanted)
        .unwrap()
}

/// The animal chooses in the next step: its destination.
fn chosen(world: &mut World, id: usize) -> Point {
    world.animals[id].as_mut().unwrap().state = AnimalState::Choosing { ticket: 0 };
    world.step();
    let animal = world.animal(id).unwrap();
    assert!(
        matches!(animal.state, AnimalState::Walking { .. }),
        "{:?}",
        animal.state
    );
    animal.walker.destination()
}

// T26 (as corrected 2026-11-13): draw N = 8 gives the 8th acceptable field
// of the search from home. A choosing animal is registered on its own field,
// so home is not acceptable (5.10a, Q-233; the measured T31 needs it): the
// six neighbours, the first field of ring 2, then the second field of ring 2.
// spec: ECO-FOOD-5.10 (T26), ECO-FOOD-5.10a, ECO-FOOD-5.13 (T31)
#[test]
fn food_t26_the_nth_acceptable_field() {
    let mut world = animal_world();
    let home = (20, 20);
    let id = world.add_animal(DEER, home);
    // md 2: draw mod 18 + 1 = 8.
    world.rng = rng_giving(7, 18);
    // The step's own draws come after: the animal's choice is the first.
    let target = chosen(&mut world, id);
    let order = [5u8, 0, 1, 2, 3, 4];
    let ring1: Vec<Point> = order
        .iter()
        .map(|&d| world.terrain().neighbour(home, d).unwrap())
        .collect();
    let first = ring1[0];
    let expected = order
        .iter()
        .map(|&d| world.terrain().neighbour(first, d).unwrap())
        .filter(|q| *q != home && !ring1.contains(q))
        .nth(1)
        .unwrap();
    assert_eq!(target, expected);
    assert_eq!(grid::distance(home, target), 2);
}

// ECO-FOOD T48 (5.10): a field with a flag on it is not acceptable, for
// every kind, the flag counting on its own field only: with N = 1 the first
// acceptable field from home is its first neighbour in ring order; with a
// flag there it is the second, right beside the flag.
// spec: ECO-FOOD-5.10 (ECO-FOOD T48)
#[test]
fn food_t48_a_field_with_a_flag_is_no_target() {
    let mut world = animal_world();
    let home = (20, 20);
    let id = world.add_animal(DEER, home);
    let ring1: Vec<Point> = [5u8, 0, 1, 2, 3, 4]
        .iter()
        .map(|&d| world.terrain().neighbour(home, d).unwrap())
        .collect();
    // md 2: draw mod 18 + 1 = 1.
    world.rng = rng_giving(0, 18);
    assert_eq!(chosen(&mut world, id), ring1[0]);
    let id = world.add_animal(DEER, home);
    let index = world.terrain().index(ring1[0]);
    world.flags[index] = Some(P0);
    world.rng = rng_giving(0, 18);
    assert_eq!(chosen(&mut world, id), ring1[1]);
}

// T27 as corrected by 5.10 (Q-240): 9 fields from home with range 6, the
// search is centred on home; with N = 1 home itself, free, is the target.
// spec: ECO-FOOD-5.10 (T27)
#[test]
fn food_t27_far_from_home_the_search_is_centred_on_home() {
    let mut world = animal_world();
    let home = (10, 20);
    let id = world.add_animal(DEER, home);
    let away = (19, 20);
    assert_eq!(grid::distance(home, away), 9);
    {
        let a = world.animals[id].as_mut().unwrap();
        a.at = away;
        a.walker = crate::movement::Walker::new(away);
    }
    world.rng = rng_giving(0, 18);
    assert_eq!(chosen(&mut world, id), home);
}

// T37: a deer at its home (48, 60), an even row, N = 1: the first
// neighbour in ring order, direction 5, (48, 61).
// spec: ECO-FOOD-5.10 (T37)
#[test]
fn food_t37_the_first_neighbour_in_ring_order() {
    let mut world = animal_world_of(70, 80);
    let id = world.add_animal(DEER, (48, 60));
    world.rng = rng_giving(0, 18);
    assert_eq!(chosen(&mut world, id), (48, 61));
}

// T38: 3 fields from home (range 6) the search is centred on the current
// field and bound to the fields within 6 of home.
// spec: ECO-FOOD-5.10 (T38)
#[test]
fn food_t38_near_home_the_search_is_centred_on_the_animal() {
    let mut world = animal_world();
    let home = (20, 20);
    let id = world.add_animal(DEER, home);
    let away = (23, 20);
    {
        let a = world.animals[id].as_mut().unwrap();
        a.at = away;
        a.walker = crate::movement::Walker::new(away);
    }
    world.rng = rng_giving(0, 18);
    let first = world.terrain().neighbour(away, 5).unwrap();
    assert_eq!(chosen(&mut world, id), first);
    // N = 18 counts only fields within 6 of home.
    for seed in 1..30 {
        let mut world = animal_world();
        let id = world.add_animal(DEER, home);
        let a = world.animals[id].as_mut().unwrap();
        a.at = away;
        a.walker = crate::movement::Walker::new(away);
        world.rng = Rng::new(seed);
        let target = chosen(&mut world, id);
        assert!(grid::distance(home, target) <= 6, "{target:?}");
    }
}

// T28: after the walk the animal waits about three times as many updates
// as it walked.
// spec: ECO-FOOD-5.11, ECO-FOOD-5.8 (T28)
#[test]
fn food_t28_waits_three_times_as_long() {
    let mut world = animal_world();
    let id = world.add_animal(RABBIT, (20, 20));
    world.rng = rng_giving(30, 3 * 4 * 5);
    chosen(&mut world, id);
    let mut walked = 0u32;
    while matches!(state(&world, id), AnimalState::Walking { .. }) {
        world.step();
        walked += 1;
    }
    let AnimalState::Waiting { left } = state(&world, id) else {
        panic!()
    };
    // The walk lasted `walked` updates after the choosing one.
    assert!(
        left == 3 * walked as i32 || left == 3 * walked as i32 - 1,
        "waited {left} after {walked}"
    );
    assert!(walked > 10);
}

// ECO-FOOD-5.10: fields near a building are not chosen by a kind that
// avoids buildings; a reserved animal stands still.
#[test]
fn food_5_10_avoid_buildings_and_hunted() {
    let mut world = animal_world();
    let home = (20, 20);
    // A building right next to home: the whole neighbourhood within 2 of it
    // is out, home included.
    building(&mut world, SMALL, (21, 20)).unwrap();
    let id = world.add_animal(DEER, home);
    for seed in 1..40 {
        world.rng = Rng::new(seed);
        world.animals[id].as_mut().unwrap().state = AnimalState::Choosing { ticket: 0 };
        world.animals[id].as_mut().unwrap().walker = crate::movement::Walker::new(home);
        world.animals[id].as_mut().unwrap().at = home;
        world.step();
        let target = world.animal(id).unwrap().walker.destination();
        assert!(grid::distance(target, (21, 20)) > 2, "{target:?}");
    }
    let id = world.add_animal(DEER, (5, 5));
    world.animals[id].as_mut().unwrap().state = AnimalState::Choosing { ticket: 0 };
    world.animals[id].as_mut().unwrap().hunted_by = world.target_of((21, 20));
    world.step();
    assert_eq!(state(&world, id), AnimalState::Hunted);
    assert_eq!(world.animal(id).unwrap().at, (5, 5));
}

// ECO-FOOD T47 (5.7): an animal reserved while waiting keeps its wait
// counter (546 throughout, no countdown); released, it is waiting with
// that counter in the release's tick and counts down from its next update,
// with no new wait drawn and no choosing.
// spec: ECO-FOOD-5.7 (T47)
#[test]
fn food_t47_a_reserved_waiting_animal_keeps_its_counter() {
    let mut world = animal_world();
    building(&mut world, SMALL, (21, 20)).unwrap();
    let id = world.add_animal(DEER, (5, 5));
    world.animals[id].as_mut().unwrap().state = AnimalState::Waiting { left: 546 };
    world.animals[id].as_mut().unwrap().hunted_by = world.target_of((21, 20));
    for _ in 0..10 {
        world.step();
        assert_eq!(state(&world, id), AnimalState::Waiting { left: 546 });
    }
    world.animals[id].as_mut().unwrap().hunted_by = None;
    let draws = world.rng.draws();
    world.step();
    assert_eq!(state(&world, id), AnimalState::Waiting { left: 545 });
    world.step();
    assert_eq!(state(&world, id), AnimalState::Waiting { left: 544 });
    assert_eq!(world.rng.draws(), draws, "no draw");
}

// Determinism with roaming animals.
// spec: none (determinism with roaming animals)
#[test]
fn animals_roam_deterministically() {
    let run = || {
        let mut world = animal_world();
        for i in 0..6 {
            world.add_animal(if i % 2 == 0 { DEER } else { RABBIT }, (10 + 3 * i, 20));
        }
        for _ in 0..2000 {
            world.step();
        }
        world
    };
    let (a, b) = (run(), run());
    assert_eq!(a.state_hash(), b.state_hash());
    // They have moved away from their homes.
    assert!(a.animals().any(|(_, x)| x.at != x.home));
    // And stay within their range of home.
    for (_, x) in a.animals() {
        let range = if x.kind == DEER { 6 } else { 8 };
        assert!(grid::distance(x.home, x.at) <= range + 1, "{:?}", x.at);
    }
}

// 5.13: a game's animals start as their map records hold them: waiting
// with the wait left, choosing with their ticket, or in mid-walk.
// spec: ECO-FOOD-5.13, FMT-S2M-5.4
#[test]
fn food_5_13_animals_start_as_saved() {
    let world = animal_world();
    let mut waiting = crate::MapAnimal::new(DEER, (10, 10));
    waiting.counter = 37;
    let mut choosing = crate::MapAnimal::new(DEER, (12, 10));
    choosing.state = 1;
    choosing.counter = 4;
    let mut walking = crate::MapAnimal::new(RABBIT, (14, 10));
    walking.home = (15, 12);
    walking.state = 2;
    walking.walk = Some(((15, 10), 0.3f32.to_bits(), vec![(16, 10)]));
    assert_eq!(
        world.map_animal(&waiting).state,
        AnimalState::Waiting { left: 37 }
    );
    assert_eq!(
        world.map_animal(&choosing).state,
        AnimalState::Choosing { ticket: 4 }
    );
    let a = world.map_animal(&walking);
    assert!(matches!(a.state, AnimalState::Walking { since: 0, .. }));
    assert_eq!((a.at, a.home), ((14, 10), (15, 12)));
    assert_eq!(a.walker.next, Some((15, 10)));
    assert_eq!(a.walker.progress(), 0.3);
    assert_eq!(a.walker.destination(), (16, 10));
}

/// ECO-FOOD-5.11b: the tick clock in trace tick `t`, the float32 sum of
/// `t` terms of 0.1.
fn clock(t: u32) -> f32 {
    (0..t).fold(0.0f32, |c, _| c + 0.1f32)
}

// T36: the clock the animals read in trace tick T is the float32 sum of
// T - 1 terms; the world's clock after tick T (the recorded `next` column)
// holds T terms.
// spec: ECO-FOOD-5.11b (T36)
#[test]
fn food_t36_the_tick_clock() {
    let mut world = animal_world();
    for t in 1..=74 {
        world.step();
        assert_eq!(world.game_seconds(), clock(t), "{t}");
    }
    let read = |t: u32| clock(t - 1);
    assert_eq!(
        [read(2), read(8), read(11), read(69), read(73), read(74)],
        [0.1, 0.70000005, 1.0000001, 6.799996, 7.1999955, 7.2999954]
    );
}

// T35: the wait from the clock values the animal read when it chose (trace
// tick s) and when its last step completed (e): those of s - 1 and e - 1,
// idle factor 3.0, in single precision.
// spec: ECO-FOOD-5.11, ECO-FOOD-5.11b (T35)
#[test]
fn food_t35_the_wait_from_the_float_clock() {
    let cases = [
        ((13, 28), 44),
        ((11, 26), 44),
        ((21, 36), 44),
        ((4, 19), 45),
        ((2, 17), 45),
        ((5, 20), 45),
        ((4, 27), 68),
        ((2, 25), 69),
        ((2, 26), 72),
        ((2, 27), 74),
        ((31, 54), 68),
        ((45, 60), 44),
        ((45, 90), 134),
        ((28, 95), 200),
    ];
    for ((s, e), wait) in cases {
        assert_eq!(
            crate::hunting::animal_wait(clock(s - 1), clock(e - 1), 3000),
            wait,
            "{s} {e}"
        );
    }
}

// 5.11b in the world: an animal that chose in the update with clock c0 and
// whose step completes in the update with clock c1 waits by those values.
// spec: ECO-FOOD-5.11, ECO-FOOD-5.11b, ECO-FOOD-5.12
#[test]
fn food_5_11b_wait_in_the_world() {
    let mut world = animal_world();
    let id = world.add_animal(DEER, (20, 20));
    world.animals[id].as_mut().unwrap().state = AnimalState::Hunted;
    // As if it had chosen in this update, which reads the clock before
    // its 0.1: it advances from the next update on.
    let chose = world.game_seconds();
    world.step();
    world.animals[id].as_mut().unwrap().walker.walk([(21, 20)]);
    world.animals[id].as_mut().unwrap().state = AnimalState::Walking {
        since: chose.to_bits(),
        speed: 45_000,
        idle_thousandths: 3000,
    };
    // The step completes in an update that reads the clock before its 0.1.
    let mut read = chose;
    while matches!(state(&world, id), AnimalState::Walking { .. }) {
        read = world.game_seconds();
        world.step();
    }
    // 45 000 a tick exceeds a whole step in the 23rd update.
    assert_eq!(world.animal(id).unwrap().at, (21, 20));
    let wait = crate::hunting::animal_wait(chose, read, 3000);
    assert_eq!(state(&world, id), AnimalState::Waiting { left: wait });
    assert!(wait == 69 || wait == 68, "{wait}");
}

// ---- Respawn (ECO-FOOD-5.15) ----

const TREE: u32 = 300;
const WOOD: u32 = 30;

/// A flat map of `width` x `height` with a tree kind yielding wood and the
/// respawn record `rate`, cursor (9, 9).
fn respawn_world(width: u32, height: u32, rate: u32) -> World {
    let mut world = animal_world_of(width, height);
    world.deposit_kinds.insert(
        TREE,
        crate::DepositKind {
            id: TREE,
            good: Some(WOOD),
            next: None,
            elevation: 1,
            grow_ticks: 1001,
            blocking: false,
            life_time: None,
            needs_plantable: false,
            random_offset: false,
        },
    );
    world.default_tree = Some(TREE);
    world.respawn = crate::Respawn {
        accumulator: 0,
        rate,
        cursor: (9, 9),
    };
    world
}

fn plant(world: &mut World, fields: impl IntoIterator<Item = Point>) {
    for f in fields {
        world.add_deposit(
            f,
            crate::Deposit {
                kind: TREE,
                position: grid::field_sample(f),
                state: crate::production::DepositState::Growing { ripe_at: 0 },
                reserved_by: None,
                life_left: None,
            },
        );
    }
}

/// The fields of the square of `side` from `corner`.
fn square(corner: Point, side: u32) -> Vec<Point> {
    (corner.1..corner.1 + side)
        .flat_map(|y| (corner.0..corner.0 + side).map(move |x| (x, y)))
        .collect()
}

// 5.15: on a 170 map (11 x 11 cells of 16) at rate 40 from the record's
// (9, 9), the first check is in trace tick 25, on cell (10, 9); the next,
// in tick 50, on (0, 10).
// spec: ECO-FOOD-5.15, FMT-S2M-5.5
#[test]
fn food_5_15_the_respawn_cursor() {
    let mut world = respawn_world(170, 170, 40);
    for _ in 0..24 {
        world.step();
    }
    assert_eq!(world.respawn.cursor, (9, 9));
    assert_eq!(world.respawn.accumulator, 960);
    world.step();
    assert_eq!(
        (world.respawn.cursor, world.respawn.accumulator),
        ((10, 9), 0)
    );
    for _ in 0..25 {
        world.step();
    }
    assert_eq!(world.respawn.cursor, (0, 10));
}

// 5.15 rules 2 to 4: a cell with more than 30 trees whose first sub-cell
// holds more than 12 and a field that, with its six neighbours, carries
// trees, gets one animal there: two draws, the kind (script order) and the
// wait (deer: 0 .. 132). Rule 1: more than 4 animals homed in the cell, no
// check; 30 trees or fewer, no check; neither draws.
// spec: ECO-FOOD-5.15 (T34)
#[test]
fn food_5_15_a_check_places_one_animal() {
    // A 32 x 32 map: 2 x 2 cells. The cursor from (9, 9) wraps to (0, 0)
    // at the first check... set it to the cell before (0, 0).
    let fresh = |trees: Vec<Point>, homes: usize| {
        let mut world = respawn_world(32, 32, 1000);
        world.respawn.cursor = (1, 1);
        plant(&mut world, trees);
        for i in 0..homes {
            world.add_animal(DEER, (14, 2 + 2 * i as u32));
            world.animals[i].as_mut().unwrap().state = AnimalState::Waiting { left: 1000 };
        }
        world
    };
    let forest = square((0, 0), 6);
    let mut world = fresh(forest.clone(), 0);
    let before = world.rng.draws();
    let animals = world.animals().count();
    world.step();
    assert_eq!(world.respawn.cursor, (0, 0));
    assert_eq!(world.animals().count(), animals + 1);
    assert_eq!(world.rng.draws() - before, 2);
    let (_, placed) = world.animals().last().unwrap();
    // The first field of the flood from (0, 0) with trees on it and around.
    assert!(world.tree_on(placed.at));
    assert!(world
        .terrain()
        .neighbours(placed.at)
        .iter()
        .all(|n| n.is_some_and(|n| world.tree_on(n))));
    assert_eq!(placed.home, placed.at);
    match placed.state {
        AnimalState::Waiting { left } if placed.kind == DEER => assert!((0..133).contains(&left)),
        AnimalState::Waiting { left } => assert!((0..171).contains(&left)),
        other => panic!("{other:?}"),
    }
    // Five animals homed in the cell: nothing, no draw.
    let mut world = fresh(forest.clone(), 5);
    let before = world.rng.draws();
    world.step();
    assert_eq!(world.animals().count(), 5);
    assert_eq!(world.rng.draws() - before, 0);
    // 30 trees: nothing.
    let mut world = fresh(forest[..30].to_vec(), 0);
    let before = world.rng.draws();
    world.step();
    assert_eq!(world.animals().count(), 0);
    assert_eq!(world.rng.draws() - before, 0);
}
