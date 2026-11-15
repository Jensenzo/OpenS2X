//! Test cases of `specs/ECO-professions.md` (ECO-PROF): new helpers and
//! professionals made only when a request is served.

use crate::grid::Point;
use crate::requests::tests::*;
use crate::test_support::*;
use crate::{GameSetup, World};

/// A game with the stock `stock` in the headquarters, whose building type
/// has the spawn flag, and the start resources setting `start_resources`.
fn game_with_spawn(stock: &[(u32, u32)], start_resources: u8) -> World {
    let mut rules = rules(stock);
    for kind in &mut rules.building_types {
        kind.spawn = kind.id == CASTLE || kind.id == DEPOT;
    }
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources,
        local_player: P0,
        ..Default::default()
    };
    World::new_game(9, flat(40, 40), rules, &setup)
}

fn helpers(world: &World, at: Point) -> u32 {
    world.warehouse(at).unwrap().count(HELPER)
}

fn run(world: &mut World, ticks: u32) {
    for _ in 0..ticks {
        world.step();
    }
}

// ECO-PROF T1, 3.1, 3.3: the first check is in the 101st update.
#[test]
fn t1_first_helper_after_101_ticks() {
    let mut world = game_with_spawn(&[(HELPER, 26)], 0);
    run(&mut world, 100);
    assert_eq!(helpers(&world, HQ), 26);
    run(&mut world, 1);
    assert_eq!(helpers(&world, HQ), 27);
    run(&mut world, 100);
    assert_eq!(helpers(&world, HQ), 27);
    run(&mut world, 1);
    assert_eq!(helpers(&world, HQ), 28);
}

// ECO-PROF T2: 74 checks bring 26 to 100, and no more after that.
// spec: ECO-PROF-3.1, ECO-PROF-3.2 (T2)
#[test]
fn t2_helpers_stop_at_one_hundred() {
    let mut world = game_with_spawn(&[(HELPER, 26)], 0);
    run(&mut world, 74 * 101);
    assert_eq!(helpers(&world, HQ), 100);
    run(&mut world, 5 * 101);
    assert_eq!(helpers(&world, HQ), 100);
}

// ECO-PROF T3, 3.2: a stock above 100 is cut at the first check.
#[test]
fn t3_start_stock_above_one_hundred_is_cut() {
    let mut world = game_with_spawn(&[(HELPER, 26)], 3);
    assert_eq!(helpers(&world, HQ), 208);
    run(&mut world, 100);
    assert_eq!(helpers(&world, HQ), 208);
    run(&mut world, 1);
    assert_eq!(helpers(&world, HQ), 100);
}

// ECO-PROF T6: a carrier request waits while the warehouse has no helper,
// and is served after the first check.
// spec: ECO-PROF-3.1, ECO-REQ-3.8 (T6)
#[test]
fn t6_carrier_request_waits_for_a_helper() {
    let mut world = game_with_spawn(&[(PLANK, 5)], 0);
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    run(&mut world, 1);
    let flag = (16, 11);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, flag)]);
    assert_eq!(helpers(&world, HQ), 0);
    run(&mut world, 40);
    assert!(world.roads().all(|r| r.carrier.is_none()));
    assert!(!world.requests(P0).is_empty());
    // The first helper appears at the 101st update of the headquarters;
    // the road's carrier leaves in the tick after it (ECO-REQ-2.2, 3.6).
    run(&mut world, 101);
    assert!(world.roads().all(|r| r.carrier.is_some()));
}

// ECO-PROF T7: each spawning warehouse fills its own stock; a depot gains
// one, a full headquarters none.
// spec: ECO-PROF-3.2, ECO-PROF-3.3 (T7)
#[test]
fn t7_each_warehouse_has_its_own_check() {
    let mut world = game_with_spawn(&[(HELPER, 100)], 0);
    let depot = (14, 14);
    assert_eq!(building(&mut world, DEPOT, depot), Ok(()));
    assert_eq!(helpers(&world, depot), 0);
    run(&mut world, 101);
    assert_eq!(helpers(&world, depot), 1);
    assert_eq!(helpers(&world, HQ), 100);
}

// ECO-PROF T8: a donkey has no cost list; no warehouse makes one.
// spec: ECO-PROF-2.2 (T8)
#[test]
fn t8_donkeys_are_not_made_in_a_warehouse() {
    let mut rules = rules(&[(HELPER, 26)]);
    let donkey = 90;
    rules.roles.donkey = Some(donkey);
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), rules, &setup);
    let road = (16, 11);
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, road)]);
    run(&mut world, 60);
    // A busy road asks for a donkey (ECO-REQ-1.1); it is never served.
    world.request(P0, crate::Requester::Road(0), donkey);
    run(&mut world, 200);
    assert_eq!(world.warehouse(HQ).unwrap().count(donkey), 0);
    assert!(world.roads().all(|r| r.donkey.is_none()));
}
