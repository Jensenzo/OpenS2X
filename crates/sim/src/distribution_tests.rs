//! Test cases of `specs/ECO-distribution.md` (ECO-DIST).

use std::collections::BTreeMap;

use crate::grid::Point;
use crate::requests::tests::*;
use crate::test_support::*;
use crate::{
    BuildingSize, BuildingType, Command, DistributionRules, GameSetup, GoodKind, GoodRules,
    PlayerId, Rejection, World,
};

const WOOD: u32 = 60;
const GRAIN: u32 = 61;
const FISH: u32 = 62;
const MEAT: u32 = 63;
const BREAD: u32 = 64;
const COAL: u32 = 65;
const COIN: u32 = 66;
const SWORD: u32 = 67;
const BEER: u32 = 68;
const IRON: u32 = 69;
const WATER: u32 = 70;
const SOLDIER: u32 = 71;

const CONSTRUCTION: u32 = 200;
// Consumer building types.
const MILL: u32 = 201;
const PIGGERY: u32 = 202;
const DONKEY_FARM: u32 = 203;
const BREWERY: u32 = 204;
const STONE_MINE: u32 = 205;
const COAL_MINE: u32 = 206;
const IRON_MINE: u32 = 207;
const GOLD_MINE: u32 = 208;
const ARMORY: u32 = 209;
const SMELTER: u32 = 210;
const MINT: u32 = 211;
const CONSUMERS: [u32; 11] = [
    MILL,
    PIGGERY,
    DONKEY_FARM,
    BREWERY,
    STONE_MINE,
    COAL_MINE,
    IRON_MINE,
    GOLD_MINE,
    ARMORY,
    SMELTER,
    MINT,
];

fn tables() -> Vec<DistributionRules> {
    let table = |goods: &[u32], entries: &[(u32, u32)]| DistributionRules {
        goods: goods.to_vec(),
        entries: entries.to_vec(),
    };
    vec![
        table(
            &[GRAIN],
            &[
                (MILL, 500),
                (PIGGERY, 300),
                (DONKEY_FARM, 200),
                (BREWERY, 300),
            ],
        ),
        table(&[IRON], &[(ARMORY, 800), (SMELTER, 400)]),
        table(&[WATER], &[(BREWERY, 600)]),
        table(&[PLANK], &[(CONSTRUCTION, 1000), (SMELTER, 400)]),
        table(
            &[FISH, MEAT, BREAD],
            &[
                (STONE_MINE, 300),
                (COAL_MINE, 500),
                (IRON_MINE, 700),
                (GOLD_MINE, 1000),
            ],
        ),
        table(&[COAL], &[(ARMORY, 800), (SMELTER, 600), (MINT, 1000)]),
    ]
}

/// The default ranks of the shipped scripts for the goods used here.
fn ranks() -> BTreeMap<u32, i64> {
    BTreeMap::from([
        (COIN, 0),
        (SWORD, 1),
        (BEER, 2),
        (IRON, 3),
        (COAL, 6),
        (PLANK, 7),
        (STONE, 8),
        (WOOD, 9),
        (WATER, 10),
        (GRAIN, 11),
        (FISH, 11),
        (MEAT, 11),
        (BREAD, 11),
    ])
}

fn game(stock: &[(u32, u32)]) -> World {
    let mut rules = rules(stock);
    rules.distribution = tables();
    rules.construction = Some(CONSTRUCTION);
    rules.transport_priorities = ranks();
    rules.goods.insert(
        WOOD,
        GoodRules {
            kind: GoodKind::Ware,
            ..GoodRules::default()
        },
    );
    rules.goods.insert(
        SOLDIER,
        GoodRules {
            kind: GoodKind::Settler,
            ..GoodRules::default()
        },
    );
    rules.goods.entry(WOODCUTTER).or_default().kind = GoodKind::Settler;
    for id in CONSUMERS {
        let mut kind = BuildingType::new(id, BuildingSize::Small);
        kind.worker_limit = 0;
        kind.instant = true;
        rules.building_types.push(kind);
    }
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    World::new_game(9, flat(60, 40), rules, &setup)
}

/// A road with its carrier already on it, so the warehouse's send
/// cooldown stays free for wares.
fn connect(w: &mut World, from: Point, to: Point) {
    apply_all(w, vec![road_cmd(from, to)]);
    let id = w.roads.len() - 1;
    w.spawn_carrier(id);
}

fn place_at(world: &mut World, kind: u32, at: Point) {
    apply_all(world, vec![place(kind, at)]);
}

/// A finished consumer of type `kind` on `at`, with one open order of
/// `good`.
fn consumer(world: &mut World, kind: u32, at: Point, good: u32) {
    place_at(world, kind, at);
    world.order(at, good);
}

/// Picks of table `table` among all open orders of `good`, `n` times.
fn picks(world: &mut World, table: usize, good: u32, n: usize) -> Vec<u32> {
    let group: Vec<usize> = (0..world.orders().len())
        .filter(|&i| world.orders()[i].good == good)
        .collect();
    (0..n)
        .map(|_| {
            let i = world
                .pick_by_table(P0, table, &group)
                .expect("an order is chosen");
            world
                .building_at(world.orders()[i].target.at)
                .unwrap()
                .building_type
        })
        .collect()
}

fn grain_world() -> World {
    let mut w = game(&[(HELPER, 26)]);
    consumer(&mut w, MILL, (14, 10), GRAIN);
    consumer(&mut w, PIGGERY, (14, 13), GRAIN);
    consumer(&mut w, DONKEY_FARM, (6, 10), GRAIN);
    consumer(&mut w, BREWERY, (6, 13), GRAIN);
    w
}

fn mines_world(kinds: &[u32], good: u32) -> World {
    let mut w = game(&[(HELPER, 26)]);
    for (i, &kind) in kinds.iter().enumerate() {
        const SPOTS: [Point; 6] = [(14, 10), (14, 13), (14, 7), (6, 10), (6, 13), (6, 7)];
        consumer(&mut w, kind, SPOTS[i], good);
    }
    w
}

// ECO-DIST T1
// spec: ECO-DIST-2.1, ECO-DIST-2.3 (T1)
#[test]
fn t1_grain_picks() {
    let mut w = grain_world();
    assert_eq!(
        picks(&mut w, 0, GRAIN, 13),
        [
            MILL,
            PIGGERY,
            MILL,
            BREWERY,
            MILL,
            PIGGERY,
            DONKEY_FARM,
            MILL,
            BREWERY,
            PIGGERY,
            MILL,
            DONKEY_FARM,
            BREWERY
        ]
    );
}

// ECO-DIST T2
// spec: ECO-DIST-2.3 (T2)
#[test]
fn t2_grain_shares() {
    let mut w = grain_world();
    let picked = picks(&mut w, 0, GRAIN, 1300);
    let n = |kind| picked.iter().filter(|&&k| k == kind).count();
    assert_eq!(
        (n(MILL), n(PIGGERY), n(BREWERY), n(DONKEY_FARM)),
        (473, 296, 295, 236)
    );
}

// ECO-DIST T3
// spec: ECO-DIST-2.1, ECO-DIST-2.3 (T3)
#[test]
fn t3_food_picks_and_shares() {
    let kinds = [GOLD_MINE, IRON_MINE, COAL_MINE, STONE_MINE];
    let mut w = mines_world(&kinds, FISH);
    assert_eq!(
        picks(&mut w, 4, FISH, 13),
        [
            GOLD_MINE, IRON_MINE, GOLD_MINE, COAL_MINE, IRON_MINE, GOLD_MINE, STONE_MINE,
            IRON_MINE, GOLD_MINE, COAL_MINE, IRON_MINE, GOLD_MINE, STONE_MINE
        ]
    );
    let mut w = mines_world(&kinds, FISH);
    let picked = picks(&mut w, 4, FISH, 2500);
    let n = |kind| picked.iter().filter(|&&k| k == kind).count();
    assert_eq!(
        (n(GOLD_MINE), n(IRON_MINE), n(COAL_MINE), n(STONE_MINE)),
        (834, 833, 417, 416)
    );
}

// ECO-DIST T4
// spec: ECO-DIST-2.3, ECO-DIST-2.4 (T4)
#[test]
fn t4_only_present_entries_win() {
    let mut w = mines_world(&[COAL_MINE, GOLD_MINE], BREAD);
    assert_eq!(
        picks(&mut w, 4, BREAD, 8),
        [GOLD_MINE, COAL_MINE, GOLD_MINE, COAL_MINE, GOLD_MINE, COAL_MINE, GOLD_MINE, COAL_MINE]
    );
}

// ECO-DIST T5
// spec: ECO-DIST-2.3 (T5)
#[test]
fn t5_coal_picks() {
    let mut w = mines_world(&[ARMORY, SMELTER, MINT], COAL);
    assert_eq!(
        picks(&mut w, 5, COAL, 12),
        [MINT, ARMORY, MINT, SMELTER, ARMORY, MINT, SMELTER, ARMORY, MINT, SMELTER, ARMORY, MINT]
    );
}

// ECO-DIST T6
// spec: ECO-DIST-2.2 (T6)
#[test]
fn t6_weight_has_a_floor() {
    let mut w = grain_world();
    apply_all(
        &mut w,
        vec![Command::DistributionWeight {
            player: P0,
            table: 0,
            entry: 0,
            thousandths: 0,
        }],
    );
    assert_eq!(w.distribution(P0)[0][0].weight(), 0.001);
    let r = apply(
        &mut w,
        Command::DistributionWeight {
            player: P0,
            table: 9,
            entry: 0,
            thousandths: 5,
        },
    );
    assert_eq!(r, Err(Rejection::NoSuchSetting));
}

// ECO-DIST T7
// spec: ECO-DIST-2.2 (T7)
#[test]
fn t7_reset_clears_accumulators() {
    let mut w = grain_world();
    picks(&mut w, 0, GRAIN, 13);
    apply_all(
        &mut w,
        vec![Command::DistributionReset {
            player: P0,
            table: 0,
        }],
    );
    assert_eq!(picks(&mut w, 0, GRAIN, 1), [MILL]);
    assert_eq!(w.distribution(P0)[0][0].weight(), 0.5);
}

// ECO-DIST T8: fish and bread are served from the one food table.
// spec: ECO-DIST-2.1, ECO-DIST-2.3 (T8)
#[test]
fn t8_fish_and_bread_use_the_food_table() {
    let mut w = game(&[(HELPER, 26)]);
    consumer(&mut w, IRON_MINE, (14, 10), BREAD);
    consumer(&mut w, COAL_MINE, (14, 13), FISH);
    assert_eq!(w.table_of(FISH), Some(4));
    assert_eq!(w.table_of(BREAD), Some(4));
    assert_eq!(w.table_of(MEAT), Some(4));
    assert_eq!(picks(&mut w, 4, BREAD, 1), [IRON_MINE]);
    // The accumulators are shared: the iron mine was reset, the coal
    // mine's order (fish) now wins with what was added so far.
    assert_eq!(picks(&mut w, 4, FISH, 1), [COAL_MINE]);
}

// ECO-DIST-2.1: a site is served as `construction`, whatever its type.
#[test]
fn construction_key_is_one_type() {
    let mut w = game(&[(HELPER, 26)]);
    // Two sites of different types, both under construction.
    apply_all(&mut w, vec![place(HUT, (14, 10)), place(SAWMILL, (6, 10))]);
    w.order((14, 10), PLANK);
    w.order((6, 10), PLANK);
    let table = w.table_of(PLANK).unwrap();
    let group = [0, 1];
    // Both are under construction: both count as the construction entry.
    w.step();
    w.step();
    // The stamp of the chosen order moves to now, so the other goes next.
    let first = w.pick_by_table(P0, table, &group).unwrap();
    let second = w.pick_by_table(P0, table, &group).unwrap();
    assert_ne!(first, second);
}

// ECO-DIST T9
// spec: ECO-DIST-4.1, ECO-DIST-4.2 (T9)
#[test]
fn t9_move_rank() {
    let mut w = game(&[(HELPER, 26)]);
    apply_all(
        &mut w,
        vec![Command::PriorityMove {
            player: P0,
            from: 9,
            to: 0,
        }],
    );
    let r = |g| w.rank(P0, g);
    assert_eq!(
        [r(WOOD), r(COIN), r(SWORD), r(BEER), r(STONE), r(WATER)],
        [0, 1, 2, 3, 9, 10]
    );
    // Another player's ranks are not touched, and reset restores them.
    assert_eq!(w.rank(PlayerId(1), WOOD), 9);
    apply_all(&mut w, vec![Command::PriorityReset { player: P0 }]);
    assert_eq!(w.rank(P0, WOOD), 9);
    // Moving down: rank 1 to rank 4 shifts 2..=4 up by one.
    apply_all(
        &mut w,
        vec![Command::PriorityMove {
            player: P0,
            from: 1,
            to: 4,
        }],
    );
    assert_eq!(
        [w.rank(P0, SWORD), w.rank(P0, BEER), w.rank(P0, IRON)],
        [4, 1, 2]
    );
}

// ECO-DIST T24 (3.3a): the orders of a finished warehouse-type building
// (list 0) are chosen by the plain lowest-stamp rule even for a good with
// a distribution table: the storehouse is no consumer in the plank table,
// yet its oldest plank order is chosen.
// spec: ECO-DIST-3.3a (ECO-DIST T24)
#[test]
fn t24_list_0_orders_by_the_plain_rule() {
    let (mut w, depot, _) = depot_world(&[(HELPER, 26)]);
    assert!(w.table_of(PLANK).is_some());
    w.order(depot, PLANK);
    w.step();
    w.order(depot, PLANK);
    let group: Vec<usize> = (0..w.orders.len())
        .filter(|&i| w.orders[i].target.at == depot)
        .collect();
    assert_eq!(group.len(), 2);
    assert_eq!(w.pick_order(P0, PLANK, &group), Some(group[0]));
}

// ECO-DIST T23 (3.1, 3.3a): list 0 is matched before list 1: a site's
// older plank order and a finished storehouse's newer one, one plank in
// the headquarters: the storehouse is served, the site's order waits.
// spec: ECO-DIST-3.1 (ECO-DIST T23), ECO-DIST-3.3a
#[test]
fn t23_list_0_before_list_1() {
    let (mut w, depot, _) = depot_world(&[(HELPER, 26), (PLANK, 1)]);
    let site = (14, 13);
    apply_all(&mut w, vec![place(HUT, site)]);
    let site_flag = w.flag_of(site);
    connect(&mut w, HQ_FLAG, site_flag);
    w.orders.retain(|o| o.target.at != site);
    w.order(site, PLANK);
    w.step();
    w.order(depot, PLANK);
    run_to_multiple(&mut w, 11);
    let open: Vec<Point> = w
        .orders
        .iter()
        .filter(|o| o.good == PLANK)
        .map(|o| o.target.at)
        .collect();
    assert_eq!(open, [site], "the storehouse served first");
}

/// The HQ, a storehouse east of it and a flag between them; roads connect
/// all three. Returns (storehouse field, middle flag).
fn depot_world(stock: &[(u32, u32)]) -> (World, Point, Point) {
    let mut w = game(stock);
    let depot = (17, 10);
    place_at(&mut w, DEPOT, depot);
    let depot_flag = w.flag_of(depot);
    let middle = (13, 9);
    apply_all(
        &mut w,
        vec![Command::PlaceFlag {
            player: P0,
            at: middle,
        }],
    );
    connect(&mut w, HQ_FLAG, middle);
    connect(&mut w, middle, depot_flag);
    (w, depot, middle)
}

fn run_to_multiple(w: &mut World, period: u64) {
    loop {
        w.step();
        if (w.tick() - 1).is_multiple_of(period) {
            return;
        }
    }
}

// ECO-DIST T10
// spec: ECO-DIST-1.2, ECO-DIST-1.5 (T10)
#[test]
fn t10_stop_sends_the_ware_to_the_other_warehouse() {
    let (mut w, depot, middle) = depot_world(&[(HELPER, 26)]);
    apply_all(
        &mut w,
        vec![Command::WarehouseSet {
            player: P0,
            at: HQ,
            good: WOOD,
            stop: true,
            on: true,
        }],
    );
    let id = w.new_ware_at(WOOD, middle, None, None);
    run_to_multiple(&mut w, 5);
    let ware = w.ware(id).unwrap();
    assert_eq!(ware.destination, Some(w.flag_of(depot)));
    assert!(!w.warehouse_accepts(HQ, WOOD));
    assert!(w.warehouse_accepts(HQ, PLANK));
}

// ECO-DIST T11
// spec: ECO-DIST-1.6 (T11), ECO-REQ-5.7 (ECO-REQ T41)
#[test]
fn t11_stop_redirects_a_ware_on_its_way() {
    // The middle flag is nearer to the HQ than to the storehouse.
    let (mut w, depot, near) = depot_world(&[(HELPER, 26)]);
    let id = w.new_ware_at(WOOD, near, None, None);
    run_to_multiple(&mut w, 5);
    assert_eq!(w.ware(id).unwrap().destination, Some(HQ_FLAG));
    // Set "stop" now: at the next t mod 3 = 0 the ware finds no next flag
    // (its warehouse no longer accepts it), at the next t mod 5 = 0 it loses
    // its destination, and at the one after it is sent to the depot
    // (ECO-REQ-5.7 c, T41).
    w.submit(Command::WarehouseSet {
        player: P0,
        at: HQ,
        good: WOOD,
        stop: true,
        on: true,
    });
    let mut at_three = None;
    for _ in 0..4 {
        w.step();
        if (w.tick() - 1).is_multiple_of(3) && at_three.is_none() {
            at_three = Some(w.ware(id).map(|x| (x.stranded, x.destination)));
        }
    }
    assert_eq!(at_three, Some(Some((true, Some(HQ_FLAG)))));
    run_to_multiple(&mut w, 5);
    assert_eq!(w.ware(id).unwrap().destination, None, "dropped");
    run_to_multiple(&mut w, 5);
    assert_eq!(w.ware(id).unwrap().destination, Some(w.flag_of(depot)));
}

// ECO-DIST-1.6 (revised 2026-10-24): after 200 checks the counter starts
// again and the route is computed again; a ware that still has its way
// keeps its destination (no re-planning every 600 ticks).
// spec: ECO-DIST-1.6
#[test]
fn recheck_counter_starts_again_after_200_checks() {
    let mut w = game(&[(HELPER, 26)]);
    let near = (13, 9);
    apply_all(
        &mut w,
        vec![Command::PlaceFlag {
            player: P0,
            at: near,
        }],
    );
    connect(&mut w, HQ_FLAG, near);
    // Its carrier never works: the ware stays where it is.
    idle_road(&mut w, HQ_FLAG, near);
    let id = w.new_ware_at(WOOD, near, None, None);
    run_to_multiple(&mut w, 5);
    assert_eq!(w.ware(id).unwrap().destination, Some(HQ_FLAG));
    let (mut started_again, mut last) = (false, 0);
    for _ in 0..700 {
        w.step();
        let ware = w.ware(id).unwrap();
        assert_eq!(ware.destination, Some(HQ_FLAG));
        // ECO-DIST-1.6 (9d69dd5): the count reaches 200, tested before
        // it rises.
        assert!(ware.recheck <= 200 && !ware.stranded);
        started_again |= ware.recheck < last;
        last = ware.recheck;
    }
    assert!(started_again);
}

// ECO-DIST T12 (corrected 2026-10-24, Q-265): "out" on planks only, no
// orders: a helper with one plank leaves the door at t0, t0 + 35, t0 + 70,
// t0 + 105 and t0 + 140; the door's 30-tick wait makes the job skip the 7th
// ticks in between without a draw (one draw per plank); the planks go to
// the storehouse.
// spec: ECO-DIST-1.7, ECO-DIST-1.7d (T12)
#[test]
fn t12_out_takes_one_unit_per_thirty_five_ticks() {
    let (mut w, depot, _) = depot_world(&[(HELPER, 26), (PLANK, 5)]);
    apply_all(
        &mut w,
        vec![Command::WarehouseSet {
            player: P0,
            at: HQ,
            good: PLANK,
            stop: false,
            on: true,
        }],
    );
    // The ticks in which a helper with a plank appears at the door.
    let helpers_with_planks = |w: &World| {
        w.settlers()
            .filter(|(_, s)| s.profession == HELPER && s.carrying == Some(PLANK))
            .count()
    };
    let mut sent = Vec::new();
    let mut seen = 0;
    while sent.len() < 5 {
        w.step();
        let t = w.tick() - 1;
        let now = 5 - w.warehouse(HQ).unwrap().count(PLANK) as usize;
        if now > seen {
            assert!(
                helpers_with_planks(&w) >= 1,
                "the plank leaves in a helper's hands"
            );
            sent.push(t);
            seen = now;
        }
        assert!(t < 400);
    }
    let t0 = sent[0];
    assert_eq!(sent, [t0, t0 + 35, t0 + 70, t0 + 105, t0 + 140]);
    assert_eq!(w.warehouse(HQ).unwrap().cooldown, 30);
    // The planks are put down at the flag by the helpers and stored in the
    // storehouse, never in the HQ.
    for _ in 0..600 {
        w.step();
    }
    let flag = w.flag_of(depot);
    assert!(w
        .wares()
        .filter(|(_, x)| x.good == PLANK)
        .all(|(_, x)| x.destination.is_none_or(|d| d == flag)));
    assert_eq!(w.warehouse(HQ).unwrap().count(PLANK), 0);
}

// ECO-DIST-1.7c a: a warehouse whose door is blocked (a settler left less
// than 30 ticks ago) is skipped before the draw; a record with both flags
// clear counts (b).
// spec: ECO-DIST-1.7c, ECO-REQ-3.7
#[test]
fn take_out_skips_a_blocked_door() {
    let (mut w, _, _) = depot_world(&[(HELPER, 26)]);
    apply_all(
        &mut w,
        vec![Command::WarehouseSet {
            player: P0,
            at: HQ,
            good: PLANK,
            stop: false,
            on: false,
        }],
    );
    assert_eq!(w.warehouse(HQ).unwrap().flags.len(), 1);
    w.warehouses.get_mut(&HQ).unwrap().cooldown = 1;
    let before = w.rng.clone();
    w.take_out();
    assert_eq!(w.rng, before);
    w.warehouses.get_mut(&HQ).unwrap().cooldown = 0;
    w.take_out();
    assert_ne!(w.rng, before);
}

// ECO-DIST T13
// spec: ECO-DIST-1.7 (T13)
#[test]
fn t13_untouched_warehouse_draws_no_random_number() {
    let (mut w, _, _) = depot_world(&[(HELPER, 26)]);
    let before = w.rng.clone();
    w.take_out();
    assert_eq!(w.rng, before);
    apply_all(
        &mut w,
        vec![Command::WarehouseCycle {
            player: P0,
            at: HQ,
            good: PLANK,
        }],
    );
    let before = w.rng.clone();
    w.take_out();
    assert_ne!(w.rng, before);
}

// ECO-DIST T14
// spec: ECO-DIST-1.1, ECO-DIST-1.3 (T14)
#[test]
fn t14_three_clicks_cycle_the_flags() {
    let mut w = game(&[(HELPER, 26)]);
    let click = |w: &mut World| {
        apply_all(
            w,
            vec![Command::WarehouseCycle {
                player: P0,
                at: HQ,
                good: PLANK,
            }],
        );
        let f = w.warehouse(HQ).unwrap().flags_of(PLANK);
        (f.stop, f.out)
    };
    assert_eq!(
        [click(&mut w), click(&mut w), click(&mut w)],
        [(true, false), (false, true), (false, false)]
    );
}

// ECO-DIST-1.3: toggle, set, all.
#[test]
fn flag_commands() {
    let mut w = game(&[(HELPER, 26)]);
    let flags = |w: &World, g| {
        let f = w.warehouse(HQ).unwrap().flags_of(g);
        (f.stop, f.out)
    };
    apply_all(
        &mut w,
        vec![
            Command::WarehouseToggle {
                player: P0,
                at: HQ,
                good: PLANK,
                stop: true,
            },
            Command::WarehouseToggle {
                player: P0,
                at: HQ,
                good: PLANK,
                stop: false,
            },
        ],
    );
    assert_eq!(flags(&w, PLANK), (true, true));
    // Both on: a click goes to none.
    apply_all(
        &mut w,
        vec![Command::WarehouseCycle {
            player: P0,
            at: HQ,
            good: PLANK,
        }],
    );
    assert_eq!(flags(&w, PLANK), (false, false));
    // All wares out; settlers untouched.
    apply_all(
        &mut w,
        vec![Command::WarehouseAll {
            player: P0,
            at: HQ,
            mode: 2,
            settlers: false,
        }],
    );
    assert_eq!(flags(&w, WOOD), (false, true));
    assert_eq!(flags(&w, SOLDIER), (false, false));
    assert!(!w
        .warehouse(HQ)
        .unwrap()
        .flags
        .contains_key(&(SOLDIER as i32)));
    apply_all(
        &mut w,
        vec![Command::WarehouseAll {
            player: P0,
            at: HQ,
            mode: 1,
            settlers: true,
        }],
    );
    assert_eq!(flags(&w, SOLDIER), (true, false));
    assert_eq!(flags(&w, WOOD), (false, true));
    // Another player's or a missing warehouse does nothing.
    let r = apply(
        &mut w,
        Command::WarehouseCycle {
            player: PlayerId(1),
            at: HQ,
            good: PLANK,
        },
    );
    assert_eq!(r, Err(Rejection::NoBuilding));
}

/// Three consumers of wood with an open order each, connected to the HQ.
fn wood_world() -> (World, [Point; 3]) {
    let mut w = game(&[(HELPER, 26), (WOOD, 10)]);
    let at = [(14, 10), (14, 14), (6, 10)];
    for &a in &at {
        consumer(&mut w, MILL, a, WOOD);
    }
    for &a in &at {
        let flag = w.flag_of(a);
        connect(&mut w, HQ_FLAG, flag);
    }
    (w, at)
}

// ECO-DIST T15
// spec: ECO-DIST-3.1, ECO-DIST-3.2, ECO-DIST-3.3, ECO-DIST-3.3b (T15)
#[test]
fn t15_one_order_per_good_and_network() {
    let (mut w, _) = wood_world();
    assert_eq!(w.orders().len(), 3);
    run_to_multiple(&mut w, 11);
    assert_eq!(w.orders().len(), 2, "exactly one served");
    // The oldest order goes first (all three have the same stamp: the first
    // filed).
    assert!(w.orders().iter().all(|o| o.target.at != (14, 10)));
}

/// The stamps of the open orders of `wood_world`, by building.
fn stamps(w: &World, at: &[Point; 3]) -> Vec<Option<u64>> {
    at.iter()
        .map(|&a| {
            w.orders()
                .iter()
                .find(|o| o.target.at == a)
                .map(|o| o.stamp)
        })
        .collect()
}

// ECO-REQ-5.3d step 2, no H: with no warehouse holding the good, each run
// chooses an order and marks it (stamp = the run's tick) though nothing is
// sent, so the choice goes round the orders: new orders' stamp is 0
// (ECO-DIST-3.3b), the lowest stamp wins, ties in list order.
// spec: ECO-REQ-5.3d, ECO-DIST-3.3, ECO-DIST-3.3b
#[test]
fn without_a_warehouse_the_choice_goes_round_the_orders() {
    let mut w = game(&[(HELPER, 26)]);
    let at = [(14, 10), (14, 14), (6, 10)];
    for &a in &at {
        consumer(&mut w, MILL, a, WOOD);
    }
    for &a in &at {
        let flag = w.flag_of(a);
        connect(&mut w, HQ_FLAG, flag);
    }
    assert_eq!(stamps(&w, &at), vec![Some(0); 3]);
    run_to_multiple(&mut w, 11);
    let t1 = w.tick() - 1;
    assert_eq!(stamps(&w, &at), vec![Some(t1), Some(0), Some(0)]);
    run_to_multiple(&mut w, 11);
    let t2 = w.tick() - 1;
    assert_eq!(stamps(&w, &at), vec![Some(t1), Some(t2), Some(0)]);
    run_to_multiple(&mut w, 11);
    let t3 = w.tick() - 1;
    run_to_multiple(&mut w, 11);
    let t4 = w.tick() - 1;
    assert_eq!(stamps(&w, &at), vec![Some(t4), Some(t2), Some(t3)]);
}

// ECO-REQ T34 (ECO-REQ-5.3d step 2): the warehouse H found for the
// representative order's flag sends for the chosen order D2; the warehouse
// nearest to D2, other than D2's own building, only has to pass the three
// tests. The first order (A, by the headquarters) was chosen in a run
// without wood, so it is the representative but D2 is the other (B, by the
// storehouse): the headquarters sends, not the storehouse.
// spec: ECO-REQ-5.3d (ECO-REQ T34), ECO-REQ-5.3 (b)
#[test]
fn t34_the_representatives_warehouse_sends() {
    let mut w = game(&[(HELPER, 26)]);
    let depot = (6, 10);
    place_at(&mut w, DEPOT, depot);
    w.warehouses.get_mut(&depot).unwrap().add(HELPER, 5);
    consumer(&mut w, MILL, (14, 10), WOOD);
    consumer(&mut w, MILL, (6, 14), WOOD);
    let (a, b, depot_flag) = (w.flag_of((14, 10)), w.flag_of((6, 14)), w.flag_of(depot));
    connect(&mut w, HQ_FLAG, a);
    connect(&mut w, HQ_FLAG, depot_flag);
    connect(&mut w, depot_flag, b);
    run_to_multiple(&mut w, 11);
    assert_eq!(w.orders().len(), 2, "no wood: nothing sent, A marked");
    w.warehouses.get_mut(&HQ).unwrap().add(WOOD, 1);
    w.warehouses.get_mut(&depot).unwrap().add(WOOD, 1);
    run_to_multiple(&mut w, 11);
    assert_eq!(w.orders().len(), 1);
    assert!(
        w.orders().iter().all(|o| o.target.at == (14, 10)),
        "B served"
    );
    assert_eq!(w.warehouse(HQ).unwrap().count(WOOD), 0, "H sends");
    assert_eq!(w.warehouse(depot).unwrap().count(WOOD), 1);
}

// ECO-REQ-5.3d step 2: while the warehouse's send cooldown runs and no
// free ware lies in the network, no order is chosen and none is marked.
// spec: ECO-REQ-5.3d, ECO-REQ-5.3 (b), ECO-DIST-3.3b
#[test]
fn while_the_warehouse_cannot_send_no_order_is_chosen() {
    let (mut w, at) = wood_world();
    run_to_multiple(&mut w, 11);
    let t1 = w.tick() - 1;
    // The first is served and the castle's cooldown of 30 starts.
    assert_eq!(stamps(&w, &at), vec![None, Some(0), Some(0)]);
    run_to_multiple(&mut w, 11);
    run_to_multiple(&mut w, 11);
    assert_eq!(w.tick() - 1, t1 + 22);
    assert_eq!(stamps(&w, &at), vec![None, Some(0), Some(0)]);
    // At t1 + 33 it sends again, for the next in list order.
    run_to_multiple(&mut w, 11);
    assert_eq!(stamps(&w, &at), vec![None, None, Some(0)]);
}

// ECO-REQ-5.3d step 1: a free ware lies in the network, but only at the
// chosen order's own flag, so there is no ware for it. That order stays
// marked, and step 2 chooses again: the next order, which the warehouse
// serves.
// spec: ECO-REQ-5.3d, ECO-REQ-5.3 (a), ECO-DIST-3.3b
#[test]
fn a_free_ware_on_the_chosen_orders_flag_moves_the_choice_on() {
    let (mut w, at) = wood_world();
    while !(w.tick() + 1).is_multiple_of(11) {
        w.step();
    }
    let flag = w.flag_of(at[0]);
    let free = w.new_ware_at(WOOD, flag, None, None);
    run_to_multiple(&mut w, 11);
    let t = w.tick() - 1;
    assert!(w
        .ware(free)
        .is_some_and(|ware| ware.target.is_none_or(|t| t.storage)));
    assert_eq!(stamps(&w, &at), vec![Some(t), None, Some(0)]);
    assert_eq!(w.warehouse(HQ).unwrap().count(WOOD), 9);
}

// ECO-DIST-1.6: a ware a carrier has claimed (walking to fetch it) is not
// re-checked: its counter does not rise and an invalid next flag stays.
// spec: ECO-DIST-1.6, ECO-REQ-5.3e
#[test]
fn a_claimed_ware_is_not_re_checked() {
    let (mut w, at) = wood_world();
    let flag = w.flag_of(at[0]);
    let id = w.new_ware_at(WOOD, flag, Some(HQ_FLAG), None);
    w.wares[id].as_mut().unwrap().next = None;
    w.wares[id].as_mut().unwrap().claimed_by = Some(0);
    w.recheck_wares();
    let ware = w.ware(id).unwrap();
    assert_eq!((ware.next, ware.recheck), (None, 0));
    w.wares[id].as_mut().unwrap().claimed_by = None;
    w.recheck_wares();
    assert_eq!(w.ware(id).unwrap().next, Some(HQ_FLAG));
}

// ECO-DIST T16
// spec: ECO-DIST-3.2 (T16)
#[test]
fn t16_one_order_in_each_network() {
    // The storehouse's network is its own: no road joins it to the HQ.
    let mut w = game(&[(HELPER, 26), (WOOD, 10)]);
    let depot = (6, 10);
    place_at(&mut w, DEPOT, depot);
    w.warehouses.get_mut(&depot).unwrap().add(WOOD, 10);
    w.warehouses.get_mut(&depot).unwrap().add(HELPER, 5);
    consumer(&mut w, MILL, (14, 10), WOOD);
    consumer(&mut w, MILL, (14, 14), WOOD);
    consumer(&mut w, MILL, (6, 14), WOOD);
    let (a, c, b) = (w.flag_of((14, 10)), w.flag_of((14, 14)), w.flag_of((6, 14)));
    let depot_flag = w.flag_of(depot);
    connect(&mut w, HQ_FLAG, a);
    connect(&mut w, HQ_FLAG, c);
    connect(&mut w, depot_flag, b);
    assert_eq!(w.orders().len(), 3);
    run_to_multiple(&mut w, 11);
    // One in the HQ's network, one in the storehouse's.
    assert_eq!(w.orders().len(), 1);
}

// ECO-DIST T17: planks are ordered by a consumer of the table (metalworks
// in the game).
// spec: ECO-DIST-1.4 (T17)
#[test]
fn t17_stop_does_not_stop_supplying() {
    let mut w = game(&[(HELPER, 26), (PLANK, 5)]);
    consumer(&mut w, SMELTER, (14, 10), PLANK);
    let flag = w.flag_of((14, 10));
    connect(&mut w, HQ_FLAG, flag);
    apply_all(
        &mut w,
        vec![Command::WarehouseSet {
            player: P0,
            at: HQ,
            good: PLANK,
            stop: true,
            on: true,
        }],
    );
    run_to_multiple(&mut w, 11);
    assert!(w.orders().is_empty());
    assert_eq!(w.warehouse(HQ).unwrap().count(PLANK), 4);
}

// ECO-DIST-3.1: goods are visited in transport rank order.
#[test]
fn goods_are_visited_by_rank() {
    let served_first = |move_wood: bool| {
        let mut w = game(&[(HELPER, 26), (PLANK, 5), (WOOD, 5)]);
        consumer(&mut w, SMELTER, (14, 10), WOOD);
        w.order((14, 10), PLANK);
        let flag = w.flag_of((14, 10));
        connect(&mut w, HQ_FLAG, flag);
        if move_wood {
            apply_all(
                &mut w,
                vec![Command::PriorityMove {
                    player: P0,
                    from: 9,
                    to: 0,
                }],
            );
        }
        // The HQ sends one ware per 30 ticks, so a run serves one good.
        run_to_multiple(&mut w, 11);
        assert_eq!(w.orders().len(), 1);
        w.orders()[0].good
    };
    // Plank 7 comes before wood 9: wood is left.
    assert_eq!(served_first(false), WOOD);
    // Wood moved to rank 0 is served first: the plank order is left.
    assert_eq!(served_first(true), PLANK);
}

// ECO-DIST-1.7 step 4: a settler marked "out" goes to another warehouse
// that accepts it.
#[test]
fn settler_out_goes_to_another_warehouse() {
    let (mut w, depot, _) = depot_world(&[(HELPER, 26), (SOLDIER, 3)]);
    apply_all(
        &mut w,
        vec![Command::WarehouseSet {
            player: P0,
            at: HQ,
            good: SOLDIER,
            stop: false,
            on: true,
        }],
    );
    run_to_multiple(&mut w, 7);
    assert_eq!(w.warehouse(HQ).unwrap().count(SOLDIER), 2);
    let soldier = w
        .settlers()
        .find(|(_, s)| s.profession == SOLDIER)
        .expect("one leaves");
    match &soldier.1.job {
        crate::SettlerJob::Home { target } => assert_eq!(target.at, depot),
        job => panic!("{job:?}"),
    }
    // It arrives and is stored there.
    for _ in 0..600 {
        w.step();
    }
    assert!(w.warehouse(depot).unwrap().count(SOLDIER) >= 1);
}

// ECO-DIST T20: a woodcutter taken out leaves by the door and goes to the
// other warehouse; the door then waits 30 ticks, as after any settler sent.
// spec: ECO-DIST-1.7a (T20)
#[test]
fn t20_settler_taken_out_blocks_the_door() {
    let (mut w, depot, _) = depot_world(&[(HELPER, 26), (WOODCUTTER, 1)]);
    apply_all(
        &mut w,
        vec![Command::WarehouseSet {
            player: P0,
            at: HQ,
            good: WOODCUTTER,
            stop: false,
            on: true,
        }],
    );
    assert_eq!(w.warehouse(HQ).unwrap().cooldown, 0);
    run_to_multiple(&mut w, 7);
    assert_eq!(w.warehouse(HQ).unwrap().count(WOODCUTTER), 0);
    let woodcutter = w
        .settlers()
        .find(|(_, s)| s.profession == WOODCUTTER)
        .expect("one leaves");
    match &woodcutter.1.job {
        crate::SettlerJob::Home { target } => assert_eq!(target.at, depot),
        job => panic!("{job:?}"),
    }
    assert_eq!(w.warehouse(HQ).unwrap().cooldown, 30);
}

/// The inner field of the road from `a` to `b`, and makes its carrier one
/// that never works (still assigned to the road).
fn idle_road(w: &mut World, a: Point, b: Point) -> Point {
    let road = w
        .roads()
        .find(|r| r.path[0] == a && *r.path.last().unwrap() == b)
        .expect("the road");
    let (inner, carrier) = (road.path[1], road.carrier.expect("its carrier"));
    w.carriers[carrier].task = crate::transport::CarrierTask::Retired;
    inner
}

// ECO-REQ T16a: a plank lying at F for a consumer's order loses its way
// when the only road from F to the consumer goes with its carrier (the
// last part of tick 100): the re-check of 102 finds no next flag, the
// matching of 110 takes the plank from the order (open again; nothing
// serves it) and the storage run later in 110 sends the plank to the
// headquarters, the nearest warehouse of F's ware network.
// spec: ECO-REQ-5.7, ECO-DIST-1.6, TRN-ROAD-6.6, ECO-REQ-5.1 (ECO-REQ T16a)
#[test]
fn t16a_a_ware_whose_road_went_loses_its_order() {
    let mut w = game(&[(HELPER, 26)]);
    let f = (13, 9);
    apply_all(&mut w, vec![Command::PlaceFlag { player: P0, at: f }]);
    connect(&mut w, HQ_FLAG, f);
    let site = (17, 10);
    consumer(&mut w, SMELTER, site, PLANK);
    let fd = w.flag_of(site);
    connect(&mut w, f, fd);
    let inner = idle_road(&mut w, f, fd);
    // The order is served by a plank lying at F, as the matching does.
    w.orders.retain(|o| o.good != PLANK);
    let target = w.target_of(site).expect("the consumer");
    let plank = w.new_ware_at(PLANK, f, Some(fd), Some(target));
    assert!(!w.ware(plank).unwrap().stranded);
    while w.tick() < 100 {
        w.step();
    }
    w.step_then(|_, _, _, _| {
        vec![Command::RemoveRoad {
            player: P0,
            at: inner,
        }]
    });
    assert_eq!(w.late_results()[0].outcome, Ok(()));
    w.step();
    assert!(!w.ware(plank).unwrap().stranded, "101: not checked yet");
    w.step();
    assert!(w.ware(plank).unwrap().stranded, "102: the re-check");
    while w.tick() < 110 {
        w.step();
        assert_eq!(w.ware(plank).unwrap().destination, Some(fd));
    }
    w.step();
    let ware = w.ware(plank).unwrap();
    assert_eq!(ware.destination, Some(HQ_FLAG));
    assert!(ware.target.is_some_and(|t| t.storage && t.at == HQ));
    assert!(w
        .orders()
        .iter()
        .any(|o| o.good == PLANK && o.target.at == site));
}

// ECO-REQ-5.7 (c), 5.1 (T41): a ware lying at F on a storage trip to the
// depot loses its way when the road toward the depot goes (tick 103); the
// re-check of 105 finds no next flag and the storage run of the same tick,
// which runs after it, drops the destination; the next storage run (110)
// sends the ware to the headquarters.
// spec: ECO-REQ-5.7 (ECO-REQ T41), ECO-REQ-5.1, ECO-DIST-1.6
#[test]
fn a_storage_trip_without_a_way_is_placed_again_in_the_next_run() {
    let (mut w, depot, middle) = depot_world(&[(HELPER, 26)]);
    let depot_flag = w.flag_of(depot);
    let inner = idle_road(&mut w, middle, depot_flag);
    let mut target = w.target_of(depot).expect("the depot");
    target.storage = true;
    let wood = w.new_ware_at(WOOD, middle, Some(depot_flag), Some(target));
    while w.tick() < 103 {
        w.step();
    }
    w.step_then(|_, _, _, _| {
        vec![Command::RemoveRoad {
            player: P0,
            at: inner,
        }]
    });
    assert_eq!(w.late_results()[0].outcome, Ok(()));
    w.step();
    assert_eq!(w.ware(wood).unwrap().destination, Some(depot_flag));
    w.step();
    assert_eq!(w.ware(wood).unwrap().destination, None, "105: dropped");
    while w.tick() <= 110 {
        w.step();
    }
    let ware = w.ware(wood).unwrap();
    assert_eq!(ware.destination, Some(HQ_FLAG), "110: placed again");
    assert!(ware.target.is_some_and(|t| t.storage && t.at == HQ));
}

// ECO-WORK-3.8d: a road carrier coming out of the building it delivered a
// ware to stands one tick on the building's flag, then (as after a put-down)
// orders its walk back to the idle field at progress 0 in the next tick.
// spec: ECO-WORK-3.8d, TRN-CARRY-3.3
#[test]
fn a_carrier_stands_on_the_flag_after_a_delivery() {
    use crate::transport::CarrierTask;
    let mut w = game(&[(HELPER, 26), (PLANK, 5)]);
    let at = (14, 10);
    consumer(&mut w, SMELTER, at, PLANK);
    let flag = w.flag_of(at);
    connect(&mut w, HQ_FLAG, flag);
    let carrier = w.carriers().len() - 1;
    let mut stood = false;
    for _ in 0..1500 {
        let before = w.carriers()[carrier].task;
        w.step();
        let c = &w.carriers()[carrier];
        if matches!(before, CarrierTask::Deliver { back: true, .. }) && c.task != before {
            assert_eq!(c.task, CarrierTask::Out { flag });
            assert_eq!(c.walker.at, flag);
            assert!(!c.walker.has_walk(), "it stands on the flag");
            w.step();
            let c = &w.carriers()[carrier];
            assert_eq!(c.task, CarrierTask::Idle);
            assert!(c.walker.has_walk() && c.walker.next.is_none() && c.walker.progress_bits == 0);
            stood = true;
            break;
        }
    }
    assert!(stood, "the plank was delivered");
}
