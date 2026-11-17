use super::*;
use crate::test_support::*;
use crate::{
    BuildingSize, BuildingType, Command, GameSetup, GoodRules, Roles, Rules, SettlerList, Tick,
};
use std::collections::BTreeMap;

pub const PLANK: u32 = 1;
pub const STONE: u32 = 2;
pub const AXE: u32 = 3;
pub const HAMMER: u32 = 4;
pub const SHOVEL: u32 = 5;
pub const HELPER: u32 = 10;
pub const BUILDER: u32 = 11;
pub const LEVELLER: u32 = 12;
pub const WOODCUTTER: u32 = 13;
pub const CASTLE: u32 = 100;
pub const HUT: u32 = 101;
pub const SAWMILL: u32 = 102;
pub const SHED: u32 = 103;
pub const DEPOT: u32 = 104;
pub const HQ: Point = (10, 10);
/// The headquarters' flag: south-east of (10, 10).
pub const HQ_FLAG: Point = (10, 9);

pub fn rules(stock: &[(u32, u32)]) -> Rules {
    let mut castle = BuildingType::new(CASTLE, BuildingSize::Large);
    castle.territory_range = 9;
    castle.explore_radius = 12;
    castle.worker_limit = 0;
    castle.warehouse = true;
    castle.instant = true;
    let mut hut = BuildingType::new(HUT, BuildingSize::Small);
    hut.frame_costs = vec![(PLANK, 1)];
    hut.costs = vec![(PLANK, 1)];
    hut.worker = Some(WOODCUTTER);
    let mut sawmill = BuildingType::new(SAWMILL, BuildingSize::Medium);
    sawmill.frame_costs = vec![(PLANK, 2)];
    sawmill.costs = vec![(STONE, 2)];
    let mut shed = BuildingType::new(SHED, BuildingSize::Small);
    shed.frame_costs = vec![(PLANK, 1)];
    let mut depot = BuildingType::new(DEPOT, BuildingSize::Small);
    depot.worker_limit = 0;
    depot.warehouse = true;
    depot.instant = true;
    let costs = |list: &[(u32, u32)]| GoodRules {
        costs: list.to_vec(),
        ..GoodRules::default()
    };
    let goods = BTreeMap::from([
        (BUILDER, costs(&[(HAMMER, 1), (HELPER, 1)])),
        (LEVELLER, costs(&[(HELPER, 1), (SHOVEL, 1)])),
        (WOODCUTTER, costs(&[(AXE, 1), (HELPER, 1)])),
        (HELPER, costs(&[(HELPER, 1)])),
    ]);
    Rules {
        building_types: vec![castle, hut, sawmill, shed, depot],
        transport_priorities: BTreeMap::from([(PLANK, 7), (STONE, 8)]),
        headquarters: Some(CASTLE),
        start_stock: stock.to_vec(),
        plank: Some(PLANK),
        stone: Some(STONE),
        goods,
        roles: Roles {
            helper: Some(HELPER),
            builder: Some(BUILDER),
            leveller: Some(LEVELLER),
            donkey: None,
            coin: None,
            boat_carrier: None,
        },
        ..Rules::empty()
    }
}

pub fn full_stock() -> Vec<(u32, u32)> {
    vec![
        (PLANK, 22),
        (STONE, 34),
        (HELPER, 26),
        (BUILDER, 5),
        (LEVELLER, 3),
        (WOODCUTTER, 4),
        (AXE, 3),
        (HAMMER, 8),
        (SHOVEL, 2),
    ]
}

pub fn game_with(terrain: crate::map::Terrain, stock: &[(u32, u32)]) -> World {
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    World::new_game(9, terrain, rules(stock), &setup)
}

pub fn game(stock: &[(u32, u32)]) -> World {
    game_with(flat(40, 40), stock)
}

pub fn count(world: &World, good: u32) -> u32 {
    world.warehouse(HQ).unwrap().count(good)
}

/// Apply commands together in one step; all must succeed.
pub fn apply_all(world: &mut World, commands: Vec<Command>) -> Tick {
    let tick = world.tick();
    for command in commands {
        world.submit(command);
    }
    world.step();
    for result in world.last_results() {
        assert_eq!(result.outcome, Ok(()), "{:?}", result.command);
    }
    tick
}

fn flag_cmd(at: Point) -> Command {
    Command::PlaceFlag { player: P0, at }
}

pub fn road_cmd(from: Point, to: Point) -> Command {
    Command::BuildRoad {
        player: P0,
        from,
        to,
    }
}

pub fn place(building: u32, at: Point) -> Command {
    Command::PlaceBuilding {
        player: P0,
        building,
        at,
    }
}

fn arriving(world: &World) -> usize {
    world
        .carriers()
        .iter()
        .filter(|c| matches!(c.task, CarrierTask::Arrive { .. }))
        .count()
}

// ECO-REQ T1
// UI-GAME-11.1, 11.2: only "found", "no resources", "building without
// worker" and "lost territory" are kept to one per field (the old one
// shown on from now); the others are added each time. A message lasts
// 36 000 ticks (600 timer units at 1/60 s a tick).
#[test]
fn message_duplicates_and_lifetime() {
    let mut world = World::with_map(1, flat(8, 8), Rules::empty(), P0);
    world.message(P0, (2, 2), "Building without worker.");
    world.message(P0, (2, 2), "Building without worker.");
    world.message(P0, (2, 2), "Found Coal!");
    world.message(P0, (2, 2), "Found Coal!");
    world.message(P0, (2, 2), "Building constructed");
    world.message(P0, (2, 2), "Building constructed");
    let count = |w: &World, text: &str| w.messages().iter().filter(|m| m.text == text).count();
    assert_eq!(count(&world, "Building without worker."), 1);
    assert_eq!(count(&world, "Found Coal!"), 1);
    assert_eq!(count(&world, "Building constructed"), 2);
    assert!(world.messages().iter().all(|m| m.until == m.tick + 36_000));
}

// UI-GAME T36b, ECO-REQ-3.8: an unserved request's "Building without
// worker" is one row whose timer is reset to 600 units (36 000 ticks)
// at each failed try, so it never runs out; "Lost Territory." is not
// added again while one stands, and not reset either (11.1).
#[test]
fn t36b_unserved_request_keeps_its_message() {
    let mut world = World::with_map(1, flat(8, 8), Rules::empty(), P0);
    world.message(P0, (2, 2), "Building without worker.");
    world.tick = 20_000;
    world.message(P0, (2, 2), "Building without worker.");
    world.tick = 50_000;
    world.message(P0, (2, 2), "Building without worker.");
    let rows: Vec<_> = world.messages().iter().map(|m| (m.tick, m.until)).collect();
    assert_eq!(rows, [(0, 86_000)]);
    let mut world = World::with_map(1, flat(8, 8), Rules::empty(), P0);
    world.message(P0, (2, 2), "Lost Territory.");
    world.tick = 20_000;
    world.message(P0, (2, 2), "Lost Territory.");
    let rows: Vec<_> = world.messages().iter().map(|m| (m.tick, m.until)).collect();
    assert_eq!(rows, [(0, 36_000)]);
}

// spec: ECO-REQ-1.1, ECO-REQ-2.4 (T1), ECO-TIME-1.5
#[test]
fn t1_road_requests_a_helper() {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![flag_cmd((14, 9))]);
    let n = apply_all(&mut world, vec![road_cmd(HQ_FLAG, (14, 9))]);
    assert_eq!(world.requests(P0).len(), 1, "filed in tick N");
    assert_eq!(world.carriers().len(), 0);
    world.step();
    assert_eq!(world.tick() - 1, n + 1);
    assert!(world.requests(P0).is_empty());
    assert_eq!(arriving(&world), 1, "sent in tick N+1");
    assert_eq!(count(&world, HELPER), 25);
    assert_eq!(world.warehouse(HQ).unwrap().cooldown, 30);
    // ECO-TIME-1.5 (Q-246): sent by the transport's part, after the
    // settlers' movement, it stands at progress 0 in that tick and first
    // moves in the next.
    let carrier = &world.carriers()[0];
    assert_eq!(carrier.walker.progress_bits, 0);
    world.step();
    assert!(world.carriers()[0].walker.progress() > 0.0);
}

// TRN-CARRY-1.3 (T20): a road built from (14, 9) to the headquarters'
// flag has that flag last. Its carrier walks out to it, which is its road,
// stands there one tick and walks on to the idle field in the next tick at
// progress 0, never over its road to the first flag.
// spec: TRN-CARRY-1.3 (T20), TRN-CARRY-1.4, ECO-REQ-3.1, ECO-WORK-3.8c
#[test]
fn a_carrier_stands_on_its_roads_last_flag() {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![flag_cmd((14, 9))]);
    apply_all(&mut world, vec![road_cmd((14, 9), HQ_FLAG)]);
    let path = world.roads().next().unwrap().path.clone();
    assert_eq!((path[0], path[path.len() - 1]), ((14, 9), HQ_FLAG));
    let idle = path[path.len() / 2];
    run_until(&mut world, 200, |w| {
        w.carriers()
            .first()
            .is_some_and(|c| c.walker.at == HQ_FLAG && c.walker.next.is_none())
    });
    let carrier = &world.carriers()[0];
    assert!(matches!(carrier.task, CarrierTask::Arrive { .. }));
    world.step();
    let carrier = &world.carriers()[0];
    assert_eq!(carrier.walker.at, HQ_FLAG);
    assert!(carrier.walker.is_walking(), "the walk ordered");
    assert_eq!(carrier.walker.progress_bits, 0);
    assert_eq!(carrier.walker.destination(), idle);
    world.step();
    assert!(world.carriers()[0].walker.progress() > 0.0);
    let reached = run_until(&mut world, 200, |w| {
        let walker = &w.carriers()[0].walker;
        assert_ne!(walker.at, path[0], "not over its road to the first flag");
        walker.at == idle && walker.next.is_none()
    });
    assert!(reached > 0);
}

// TRN-ROAD-4.2: a road split while its carrier comes out of the
// headquarters at the road's last flag: its field is not on the first half
// (from the road's first flag to the new flag), so it gets the second half,
// which has the headquarters' flag, and idles on it.
// spec: TRN-ROAD-4.2, TRN-CARRY-1.3, TRN-CARRY-1.4
#[test]
fn a_split_keeps_the_arriving_carrier_with_its_flag() {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![flag_cmd((16, 9))]);
    apply_all(&mut world, vec![road_cmd((16, 9), HQ_FLAG)]);
    run_until(&mut world, 10, |w| !w.carriers().is_empty());
    apply_all(&mut world, vec![flag_cmd((13, 9))]);
    assert_eq!(world.roads().count(), 2);
    let carrier = world.carriers()[0].clone();
    assert!(matches!(carrier.task, CarrierTask::Arrive { .. }));
    let road = world.road(carrier.road).unwrap();
    assert!(road.path.contains(&HQ_FLAG), "{:?}", road.path);
    let idle = road.path[road.path.len() / 2];
    run_until(&mut world, 200, |w| {
        let walker = &w.carriers()[0].walker;
        walker.at == idle && !walker.is_walking()
    });
}

// ECO-REQ T2
// spec: ECO-REQ-3.5, ECO-REQ-3.7 (T2)
#[test]
fn t2_one_settler_per_30_ticks() {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![flag_cmd((14, 9)), flag_cmd((10, 13))]);
    let n = apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, (14, 9)), road_cmd(HQ_FLAG, (10, 13))],
    );
    let mut sent = Vec::new();
    for _ in 0..40 {
        let before = world.carriers().len();
        world.step();
        if world.carriers().len() > before {
            sent.push(world.tick() - 1 - n);
        }
    }
    assert_eq!(sent, [1, 31]);
}

/// Step until `done`; the tick in which it first held.
pub fn run_until(world: &mut World, limit: u32, done: impl Fn(&World) -> bool) -> Tick {
    for _ in 0..limit {
        world.step();
        if done(world) {
            return world.tick() - 1;
        }
    }
    panic!("not reached within {limit} ticks");
}

fn wares_of(world: &World, good: u32) -> usize {
    world.wares().filter(|(_, w)| w.good == good).count()
}

// ECO-REQ T3: leveller first; no material leaves the HQ until levelling
// is done; the 4 orders come in the leveller's last step (T29), the builder
// request in the site's next update (the leveller's last step is the
// settlers' part of the tick).
// spec: ECO-REQ-1.2, ECO-REQ-1.3, ECO-REQ-4.1 (T3, ECO-REQ T29)
#[test]
fn t3_materials_wait_for_levelling() {
    let mut terrain = flat(40, 40);
    let site = (16, 12);
    set_height(&mut terrain, site, 400);
    let mut world = game_with(terrain, &full_stock());
    let site_flag = (16, 11);
    apply_all(&mut world, vec![place(SAWMILL, site)]);
    assert_eq!(world.requests(P0)[0].profession, LEVELLER);
    assert!(world.orders().is_empty());
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, site_flag)]);
    let levelled = run_until(&mut world, 3000, |w| {
        assert_eq!(wares_of(w, PLANK) + wares_of(w, STONE), 0);
        assert_eq!(count(w, PLANK), 22);
        w.site(site).unwrap().levelled()
    });
    // ECO-REQ T29 (ECO-REQ-4.1): the orders come in the leveller's last
    // step, the tick the levelling completes; the builder request in the
    // site's next update (ECO-REQ-1.3).
    let site_state = world.site(site).unwrap();
    assert!(site_state.orders_filed, "in tick {levelled}");
    assert_eq!(world.orders().len(), 4, "orders in tick {levelled}");
    let builder = |w: &World| {
        w.requests(P0).iter().any(|r| r.profession == BUILDER)
            || w.site(site).unwrap().builder_on_way
    };
    assert!(!builder(&world), "no builder in tick {levelled}");
    world.step();
    assert!(builder(&world));
}

// ECO-REQ T4: a small site is levelled at once and orders at once.
// spec: ECO-REQ-4.1, ECO-HQ-4.2, ECO-REQ-3.8 (T4)
#[test]
fn t4_small_site_orders_at_once() {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    let site = world.site((16, 12)).unwrap();
    assert!(site.levelled());
    let goods: Vec<u32> = world.orders().iter().map(|o| o.good).collect();
    assert_eq!(goods, [PLANK, PLANK]);
    // Not connected: the builder request waits in the queue.
    assert_eq!(world.requests(P0)[0].profession, BUILDER);
}

// TRN-FREE-6.5 a (T17), ECO-WORK-3.8c: the road walk is ordered on the
// warehouse's flag, so its route is found there. A builder sent while the
// site is reached only by a detour over D takes the direct road built while
// he was still on his way out to the flag.
// spec: TRN-FREE-6.5, ECO-WORK-3.8c, TRN-FREE-6.1, TRN-FREE-6.2
#[test]
fn the_road_walk_is_found_on_the_warehouses_flag() {
    let mut world = game(&full_stock());
    let (hut, hut_flag, d) = ((16, 12), (16, 11), (13, 4));
    apply_all(&mut world, vec![place(HUT, hut), flag_cmd(d)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, d), road_cmd(d, hut_flag)],
    );
    let builder = |w: &World| {
        w.settlers()
            .find(|(_, s)| matches!(s.job, SettlerJob::ToSite { builder: true, .. }))
            .map(|(_, s)| s.walker.clone())
    };
    run_until(&mut world, 200, |w| builder(w).is_some());
    let out = builder(&world).unwrap();
    assert!(out.path.contains(&d), "set out over D: {out:?}");
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, hut_flag)]);
    let mut seen = Vec::new();
    while let Some(walker) = builder(&world) {
        seen.push(walker.at);
        assert!(seen.len() < 2000, "not arrived");
        world.step();
    }
    assert!(seen.contains(&HQ_FLAG), "{seen:?}");
    assert!(!seen.contains(&d), "{seen:?}");
    assert!(world.site(hut).unwrap().builder.is_some());
}

/// TRN-FREE-6.5 T18, T19: flags A (the castle's), B, C and D two apart in
/// a row, roads A-B, B-C and C-D, D the flag of a hut site. Returned once
/// the site's builder steps from A towards B, with the fields of the roads
/// B-C and C-D.
fn builder_between_a_and_b() -> (World, [Point; 4], Vec<Point>, Vec<Point>) {
    let mut world = game(&full_stock());
    let (b, c, d, hut) = ((12, 9), (14, 9), (16, 9), (16, 10));
    apply_all(&mut world, vec![place(HUT, hut), flag_cmd(b), flag_cmd(c)]);
    assert_eq!(world.flag_of(hut), d);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, b), road_cmd(b, c), road_cmd(c, d)],
    );
    let road = |w: &World, end: Point, other: Point| {
        w.roads()
            .find(|r| r.path.contains(&end) && r.path.contains(&other))
            .unwrap()
            .path
            .clone()
    };
    let (bc, cd) = (road(&world, b, c), road(&world, c, d));
    run_until(&mut world, 400, |w| {
        builder(w).is_some_and(|walker| walker.at == HQ_FLAG && walker.next.is_some())
    });
    (world, [HQ_FLAG, b, c, d], bc, cd)
}

fn builder(world: &World) -> Option<crate::movement::Walker> {
    world
        .settlers()
        .find(|(_, s)| matches!(s.job, SettlerJob::ToSite { builder: true, .. }))
        .map(|(_, s)| s.walker.clone())
}

// TRN-FREE-6.5 c (T18): the road C-D is removed and a road B-D built while
// the builder walks A -> B. On reaching B the route is searched again: he
// goes on over B-D in the same tick, never standing on B, and never enters
// B-C.
// spec: TRN-FREE-6.5 (T18), TRN-MOVE-2.3
#[test]
fn t18_at_every_flag_the_route_is_searched_again() {
    let (mut world, [_, b, _, d], bc, cd) = builder_between_a_and_b();
    let mut commands = vec![Command::RemoveRoad {
        player: P0,
        at: cd[1],
    }];
    commands.push(road_cmd(b, d));
    apply_all(&mut world, commands);
    let mut seen = Vec::new();
    while let Some(walker) = builder(&world) {
        assert!(
            walker.at != b || walker.next.is_some(),
            "stands on B: {walker:?}"
        );
        assert!(!bc[1..bc.len() - 1].contains(&walker.at), "{seen:?}");
        seen.push(walker.at);
        assert!(seen.len() < 2000, "not arrived");
        world.step();
    }
    // On D he becomes the site's builder.
    assert!(seen.contains(&b), "{seen:?}");
    assert_eq!(world.flag_of((16, 10)), d);
    assert!(world.site((16, 10)).unwrap().builder.is_some());
}

// TRN-FREE-6.5 e (T19), TRN-FREE-6.3 (Q-272): the road C-D is removed while
// the builder walks A -> B, and no other way leads on: the search on B finds
// no route and the walk ends on B. It is not retried: the walk counts as
// done there, and as he does not stand on the site's flag he goes home at
// once, over the roads from B to the headquarters.
// spec: TRN-FREE-6.5 (T19), TRN-FREE-6.3
#[test]
fn t19_without_a_route_the_walk_ends_on_the_flag() {
    let (mut world, [a, b, _, _], _, cd) = builder_between_a_and_b();
    let number = world
        .settlers()
        .find(|(_, s)| matches!(s.job, SettlerJob::ToSite { builder: true, .. }))
        .map(|(_, s)| s.number)
        .unwrap();
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: cd[1],
        }],
    );
    run_until(&mut world, 200, |w| {
        builder(w).is_none_or(|walker| walker.at == b)
    });
    let s = world
        .settlers()
        .map(|(_, s)| s)
        .find(|s| s.number == number)
        .unwrap();
    assert_eq!(s.walker.at, b);
    assert!(matches!(s.job, SettlerJob::Home { .. }), "{:?}", s.job);
    assert_eq!(s.walker.road_to, Some(a));
    assert!(world.site((16, 10)).unwrap().builder.is_none());
}

// TRN-FREE-6.3a (T46): the builder of T19 goes home from B; the site keeps
// him as its builder on the way: no new builder request while he walks.
// His walk home ends in tick T at the headquarters; he is taken into the
// stock and unlinked in T + 1 (the settlers' update); the site files a new
// builder request in T + 2, its first building update after that (here a
// tick later each: the headquarters' door is its own field).
// spec: TRN-FREE-6.3a (T46), TRN-FREE-6.3
#[test]
fn t46_a_short_walked_builder_stays_the_sites_until_he_goes() {
    let (mut world, [_, b, _, _], _, cd) = builder_between_a_and_b();
    let hut = (16, 10);
    let number = world
        .settlers()
        .find(|(_, s)| matches!(s.job, SettlerJob::ToSite { builder: true, .. }))
        .map(|(_, s)| s.number)
        .unwrap();
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: cd[1],
        }],
    );
    let site_target = world.target_of(hut).unwrap();
    let asks = |w: &World| {
        w.queue.iter().any(|r| {
            r.requester == Requester::Site(site_target) && Some(r.profession) == w.roles.builder
        })
    };
    let him = |w: &World| {
        w.settlers()
            .map(|(_, s)| s.clone())
            .find(|s| s.number == number)
    };
    run_until(&mut world, 200, |w| {
        him(w).is_some_and(|s| matches!(s.job, SettlerJob::Home { .. }) && s.walker.at == b)
    });
    let builders = world
        .warehouse(HQ)
        .unwrap()
        .count(world.roles.builder.unwrap());
    // On his way home: still the site's, nobody asked for.
    let mut arrived_in = None;
    while let Some(s) = him(&world) {
        assert!(world.site(hut).unwrap().builder_on_way);
        assert!(!asks(&world));
        if arrived_in.is_none() && !s.walker.is_walking() && s.walker.at == world.door_field(HQ) {
            arrived_in = Some(world.tick());
        }
        world.step();
        assert!(world.tick() < 5000, "he never got home");
    }
    let gone_in = world.tick();
    // Taken in the tick after his walk home ends; where the door is the
    // building's own field (this headquarters), the last one-step walk to
    // it takes a tick of its own (TRN-FREE-6.4c (2)).
    let t = arrived_in.expect("his walk home ended at the headquarters");
    let no_door = world.door_field(HQ) == HQ;
    assert_eq!(gone_in, t + if no_door { 2 } else { 1 });
    assert_eq!(
        world
            .warehouse(HQ)
            .unwrap()
            .count(world.roles.builder.unwrap()),
        builders + 1
    );
    // Unlinked with his removal, but the buildings' update of that tick
    // came first: no request yet.
    assert!(!world.site(hut).unwrap().builder_on_way);
    assert!(!asks(&world));
    world.step();
    assert_eq!(world.tick(), gone_in + 1);
    assert!(asks(&world));
}

// TRN-FREE-6.3 (corrected 2026-11-15 night, Q-272), TRN-FREE-6.5 e: the
// woodcutter walks to his hut over A -> B -> D; the road B-D is removed
// while he walks A -> B and no other way leads on, so his walk ends on B.
// It is not retried, and his work routine does not start there: not
// standing on his building's field, he goes home at once, over the roads
// from B to the headquarters (6.4).
// spec: TRN-FREE-6.3, TRN-FREE-6.5
#[test]
fn a_worker_whose_walk_ended_short_goes_home_at_once() {
    let (mut world, _) = finished_hut(&full_stock());
    let (b, hut_flag) = ((13, 9), (16, 11));
    apply_all(&mut world, vec![flag_cmd(b)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, b), road_cmd(b, hut_flag)],
    );
    let bd = world
        .roads()
        .find(|r| r.path.contains(&b) && r.path.contains(&hut_flag))
        .unwrap()
        .path
        .clone();
    let woodcutter = |w: &World| {
        w.settlers()
            .find(|(_, s)| s.profession == WOODCUTTER)
            .map(|(_, s)| s.clone())
    };
    run_until(&mut world, 1000, |w| {
        woodcutter(w).is_some_and(|s| {
            matches!(s.job, SettlerJob::ToBuilding { .. })
                && s.walker.at == HQ_FLAG
                && s.walker.next.is_some()
        })
    });
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: bd[1],
        }],
    );
    run_until(&mut world, 200, |w| {
        woodcutter(w).is_some_and(|s| !matches!(s.job, SettlerJob::ToBuilding { .. }))
    });
    let s = woodcutter(&world).unwrap();
    assert_eq!(s.walker.at, b);
    assert!(matches!(s.job, SettlerJob::Home { .. }), "{:?}", s.job);
    assert_eq!(s.walker.road_to, Some(HQ_FLAG));
    // He reaches the headquarters and is taken into its stock.
    let before = count(&world, WOODCUTTER);
    run_until(&mut world, 400, |w| woodcutter(w).is_none());
    assert_eq!(count(&world, WOODCUTTER), before + 1);
}

// TRN-FREE-6.3a Worker (T46a, Q-1056): the woodcutter of the test above
// stays the hut's worker while he walks home: its 101-tick timer (firing in
// F + 101, F + 202, ...) files no request. He is taken into the stock and
// unlinked in tick G; the next request is filed at the timer's first firing
// after G, not at once (a firing in G itself would still count him).
// spec: TRN-FREE-6.3a (T46a), ECO-REQ-1.4
#[test]
fn t46a_a_short_walked_worker_stays_the_buildings_until_he_goes() {
    let (mut world, finished) = finished_hut(&full_stock());
    let hut = (16, 12);
    let (b, hut_flag) = ((13, 9), (16, 11));
    apply_all(&mut world, vec![flag_cmd(b)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, b), road_cmd(b, hut_flag)],
    );
    let bd = world
        .roads()
        .find(|r| r.path.contains(&b) && r.path.contains(&hut_flag))
        .unwrap()
        .path
        .clone();
    let woodcutter = |w: &World| {
        w.settlers()
            .find(|(_, s)| s.profession == WOODCUTTER)
            .map(|(_, s)| s.clone())
    };
    run_until(&mut world, 1000, |w| {
        woodcutter(w).is_some_and(|s| s.walker.at == HQ_FLAG && s.walker.next.is_some())
    });
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: bd[1],
        }],
    );
    let target = world.target_of(hut).unwrap();
    let asks = |w: &World| {
        w.queue
            .iter()
            .any(|r| r.requester == Requester::Building(target))
    };
    // While he walks home: still the hut's worker, nobody asked for.
    let mut gone_in = None;
    for _ in 0..600 {
        world.step();
        if woodcutter(&world).is_none() {
            gone_in = Some(world.tick() - 1);
            break;
        }
        assert!(!asks(&world), "asked in tick {}", world.tick() - 1);
        assert_eq!(world.building_at(hut).unwrap().workers, 1);
    }
    let g = gone_in.expect("he got home");
    assert_eq!(world.building_at(hut).unwrap().workers, 0);
    // The timer's first firing after G.
    let next = finished + 101 * ((g - finished) / 101 + 1);
    while world.tick() < next {
        assert!(!asks(&world), "asked in tick {}", world.tick() - 1);
        world.step();
    }
    assert_eq!(world.tick() - 1, next - 1);
    world.step();
    assert!(asks(&world), "no request at the firing {next}");
}

// TRN-CARRY-3.3a: into a warehouse the carrier walks on to its door field,
// stands there with the ware only in the arrival tick A, and in A + 1 the
// warehouse has the ware and the walk back over its field to the flag
// starts at progress 0 (E-trace-005: the castle's door field (139, 52),
// carrier 90234 arriving 3290, walking back 3291).
// spec: TRN-CARRY-3.3, TRN-CARRY-3.3a
#[test]
fn into_a_warehouse_the_carrier_stands_one_tick_on_its_door_field() {
    let mut rules = rules(&full_stock());
    rules.building_types[0].doors = [Some(2); 3];
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), rules, &setup);
    let door = world.door_field(HQ);
    assert_ne!(door, HQ);
    let far = (14, 9);
    apply_all(&mut world, vec![flag_cmd(far)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    // Its carrier at work on the road.
    run_until(&mut world, 400, |w| {
        w.carriers()
            .first()
            .is_some_and(|c| c.task == CarrierTask::Idle && !c.walker.is_walking())
    });
    let stones = count(&world, STONE);
    let target = world.target_of(HQ);
    world.new_ware_at(STONE, far, Some(HQ_FLAG), target);
    run_until(&mut world, 400, |w| w.carriers()[0].walker.at == door);
    let c = |w: &World| w.carriers()[0].clone();
    assert!(!c(&world).walker.is_walking());
    assert!(c(&world).ware.is_some());
    assert_eq!(count(&world, STONE), stones);
    world.step();
    assert_eq!(c(&world).ware, None);
    assert_eq!(count(&world, STONE), stones + 1);
    assert!(c(&world).walker.has_walk());
    assert_eq!(
        (c(&world).walker.at, c(&world).walker.progress()),
        (door, 0.0)
    );
    assert_eq!(c(&world).walker.destination(), HQ_FLAG);
}

/// A woodcutter hut at (16, 12), connected, finished at once by the
/// debug supply; the tick it was finished.
fn finished_hut(stock: &[(u32, u32)]) -> (World, Tick) {
    let mut world = game(stock);
    let hut = (16, 12);
    apply_all(&mut world, vec![road_cmd_after_flag(), place(HUT, hut)]);
    apply_all(
        &mut world,
        vec![Command::DebugSupplySite {
            player: P0,
            at: hut,
            wares: vec![(PLANK, 2)],
            workers: true,
        }],
    );
    let finished = run_until(&mut world, 2000, |w| w.site(hut).is_none());
    (world, finished)
}

fn road_cmd_after_flag() -> Command {
    // The hut's flag is placed with it; the road comes in the same tick.
    flag_cmd((16, 11))
}

fn workers_out(world: &World) -> Vec<u32> {
    world
        .settlers()
        .filter(|(_, s)| matches!(s.job, SettlerJob::ToBuilding { .. }))
        .map(|(_, s)| s.profession)
        .collect()
}

fn connect_hut(world: &mut World) {
    apply_all(world, vec![road_cmd(HQ_FLAG, (16, 11))]);
}

// ECO-REQ T5: the first worker request 101 ticks after finishing.
// spec: ECO-REQ-1.4 (T5)
#[test]
fn t5_worker_request_after_101_ticks() {
    let (mut world, finished) = finished_hut(&full_stock());
    connect_hut(&mut world);
    while world.tick() <= finished + 100 {
        world.step();
        assert!(workers_out(&world).is_empty(), "tick {}", world.tick() - 1);
    }
    let before = count(&world, WOODCUTTER);
    world.step();
    assert_eq!(world.tick() - 1, finished + 101);
    assert_eq!(workers_out(&world), [WOODCUTTER]);
    assert_eq!(count(&world, WOODCUTTER), before - 1);
}

fn without(stock: &[(u32, u32)], good: u32) -> Vec<(u32, u32)> {
    stock.iter().copied().filter(|&(g, _)| g != good).collect()
}

// ECO-REQ T6: made from an axe and a helper.
// spec: ECO-REQ-3.3, ECO-REQ-3.6, ECO-PROF-2.1 (T6)
#[test]
fn t6_worker_made_from_tool() {
    let (mut world, finished) = finished_hut(&without(&full_stock(), WOODCUTTER));
    connect_hut(&mut world);
    // The road's carrier leaves first.
    world.step();
    let (axes, helpers) = (count(&world, AXE), count(&world, HELPER));
    while world.tick() <= finished + 101 {
        world.step();
    }
    assert_eq!(workers_out(&world), [WOODCUTTER]);
    assert_eq!(count(&world, AXE), axes - 1);
    // One helper became the woodcutter (the road's carrier was sent
    // earlier).
    assert_eq!(count(&world, HELPER), helpers - 1);
    assert_eq!(count(&world, WOODCUTTER), 0);
}

// ECO-REQ T7: no axe, no woodcutter: not sent, a message, the request
// stays.
// spec: ECO-REQ-3.8 (T7)
#[test]
fn t7_building_without_worker() {
    let stock = without(&without(&full_stock(), WOODCUTTER), AXE);
    let (mut world, finished) = finished_hut(&stock);
    connect_hut(&mut world);
    while world.tick() <= finished + 300 {
        world.step();
    }
    assert!(workers_out(&world).is_empty());
    let open: Vec<u32> = world.requests(P0).iter().map(|r| r.profession).collect();
    assert_eq!(open, [WOODCUTTER]);
    // The hut's "Building constructed" aside (UI-GAME-11.1).
    let messages: Vec<&str> = world
        .messages()
        .iter()
        .map(|m| m.text)
        .filter(|&t| t != "Building constructed")
        .collect();
    assert_eq!(messages, ["Building without worker."]);
}

// ECO-REQ T11: a ware nobody ordered goes to the nearest warehouse at
// the next tick with t mod 5 = 0; at the warehouse's own flag straight
// in.
// spec: ECO-REQ-5.5 (T11)
#[test]
fn t11_unclaimed_wares_are_stored() {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![flag_cmd((14, 9))]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (14, 9))]);
    let far = world.new_ware_at(STONE, (14, 9), None, None);
    let near = world.new_ware_at(STONE, HQ_FLAG, None, None);
    while !world.tick().is_multiple_of(5) {
        world.step();
        assert_eq!(world.ware(far).unwrap().destination, None);
    }
    world.step();
    let ware = world.ware(far).unwrap();
    assert_eq!(ware.destination, Some(HQ_FLAG));
    assert!(ware.target.unwrap().storage);
    assert!(world.ware(near).is_none());
    assert_eq!(count(&world, STONE), 35);
}

// ECO-REQ T12 with a site's plank order: the free plank 2 fields away is
// matched; the farther HQ sends nothing. The matching waits for the roads'
// carriers (the ware network, TRN-CARRY-5.1a).
// spec: ECO-REQ-5.3, ECO-REQ-5.4, TRN-CARRY-5.1a (T12)
#[test]
fn t12_free_ware_before_warehouse() {
    let mut world = game(&full_stock());
    let shed = (14, 12);
    let shed_flag = (14, 11);
    let free_at = (16, 11);
    apply_all(&mut world, vec![place(SHED, shed), flag_cmd(free_at)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, shed_flag), road_cmd(shed_flag, free_at)],
    );
    // Both roads in the ware network at once.
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    let plank = world.new_ware_at(PLANK, free_at, None, None);
    while !world.tick().is_multiple_of(11) {
        world.step();
    }
    world.step();
    let ware = world.ware(plank).unwrap();
    assert_eq!(ware.destination, Some(shed_flag));
    assert_eq!(ware.target.unwrap().at, shed);
    assert_eq!(count(&world, PLANK), 22);
    assert!(world.orders().is_empty());
}

// ECO-REQ-5.3e: the free plank redirected by (a) to the shed's order has
// its next flag cleared, not computed; the next re-check of ECO-DIST-1.6
// (ticks with t mod 3 = 0) computes it.
// spec: ECO-REQ-5.3e, ECO-REQ-5.3 (a), ECO-DIST-1.6
#[test]
fn a_redirected_wares_next_flag_waits_for_the_re_check() {
    let mut world = game(&full_stock());
    let shed = (14, 12);
    let shed_flag = (14, 11);
    let free_at = (16, 11);
    apply_all(&mut world, vec![place(SHED, shed), flag_cmd(free_at)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, shed_flag), road_cmd(shed_flag, free_at)],
    );
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    let plank = world.new_ware_at(PLANK, free_at, None, None);
    while !world.tick().is_multiple_of(11) {
        world.step();
    }
    // The matching of this tick (a multiple of 11 but not of 3).
    assert!(!world.tick().is_multiple_of(3));
    world.step();
    let ware = world.ware(plank).unwrap();
    assert_eq!(ware.destination, Some(shed_flag));
    assert_eq!(ware.next, None, "cleared");
    while !world.tick().is_multiple_of(3) {
        world.step();
        assert_eq!(world.ware(plank).unwrap().next, None);
    }
    world.step();
    assert_eq!(world.ware(plank).unwrap().next, Some(shed_flag));
}

// ECO-REQ-5.3e (Q-282, ECO-REQ T17): a free plank the carrier of its
// road is walking to fetch, redirected by (a) to the shed's order, loses
// that claim at once, not at the carrier's arrival; the re-check gives it
// its next flag, the carrier stands on arrival and takes it from the next
// tick.
// spec: ECO-REQ-5.3e (ECO-REQ T17), ECO-REQ-5.3 (a), ECO-DIST-1.6
#[test]
fn a_redirected_ware_loses_its_carriers_claim_at_once() {
    let mut world = game(&full_stock());
    let shed = (14, 12);
    let shed_flag = (14, 11);
    let free_at = (16, 11);
    apply_all(&mut world, vec![flag_cmd(shed_flag), flag_cmd(free_at)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, shed_flag), road_cmd(shed_flag, free_at)],
    );
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    let plank = world.new_ware_at(PLANK, free_at, None, None);
    // Stored at the headquarters first: the carrier of the second road
    // claims it and walks to fetch it.
    while world.ware(plank).unwrap().claimed_by.is_none() {
        world.step();
        assert!(world.tick() < 500, "claimed");
    }
    let carrier = world.ware(plank).unwrap().claimed_by.unwrap();
    apply_all(&mut world, vec![place(SHED, shed)]);
    while !world.tick().is_multiple_of(11) {
        world.step();
    }
    world.step();
    let ware = world.ware(plank).unwrap();
    assert_eq!(ware.destination, Some(shed_flag), "redirected");
    assert_eq!(ware.claimed_by, None, "the claim gone at once");
    assert!(matches!(
        world.carriers[carrier].task,
        CarrierTask::Fetch { .. }
    ));
    while matches!(world.carriers[carrier].task, CarrierTask::Fetch { .. }) {
        world.step();
        assert!(world.tick() < 800, "arrives");
    }
    assert_eq!(world.carriers[carrier].walker.at, free_at);
    assert_eq!(world.carriers[carrier].task, CarrierTask::Idle, "stands");
    world.step();
    assert!(matches!(
        world.carriers[carrier].task,
        CarrierTask::PickUp { .. }
    ));
    assert_eq!(world.ware(plank).unwrap().claimed_by, Some(carrier));
}

// TRN-CARRY T37 (TRN-CARRY-2.2): a new carrier that comes out of the
// warehouse onto one of its road's two flags looks once there after its
// standing tick and takes a ready ware at once, before walking to its idle
// field.
// spec: TRN-CARRY-2.2 (TRN-CARRY T37), ECO-WORK-3.8d
#[test]
fn t37_a_carrier_out_of_the_warehouse_looks_on_its_roads_flag() {
    use crate::transport::CarrierTask;
    let mut world = game(&full_stock());
    let far = (16, 9);
    apply_all(&mut world, vec![flag_cmd(far)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    let plank = world.new_ware_at(PLANK, HQ_FLAG, Some(far), None);
    run_until(&mut world, 400, |w| {
        w.carriers().iter().any(|c| {
            c.walker.at == HQ_FLAG && matches!(c.task, CarrierTask::Arrive { stand: false, .. })
        })
    });
    assert_eq!(world.ware(plank).unwrap().next, Some(far));
    world.step();
    assert_eq!(
        world.ware(plank).unwrap().claimed_by,
        Some(0),
        "claimed at once"
    );
    assert!(!matches!(world.carriers()[0].task, CarrierTask::Idle));
}

// ECO-REQ T13: 4 requests per tick, the cursor goes on next tick.
// spec: ECO-REQ-2.2 (T13)
#[test]
fn t13_four_requests_per_tick() {
    // A world without warehouses: no request can be served.
    let mut world = world(flat(40, 12));
    world.roles.helper = Some(HELPER);
    let flags: Vec<Point> = (0..7).map(|i| (2 + 5 * i, 5)).collect();
    apply_all(&mut world, flags.iter().map(|&f| flag_cmd(f)).collect());
    apply_all(
        &mut world,
        flags.windows(2).map(|p| road_cmd(p[0], p[1])).collect(),
    );
    assert_eq!(world.requests(P0).len(), 6);
    assert_eq!(world.request_queue().1, 0);
    world.step();
    assert_eq!(world.request_queue().1, 4);
    world.step();
    assert_eq!(world.request_queue().1, 2);
}

// ECO-REQ T39 (ECO-REQ-2.1, 2.2, Q-382): one queue holds the requests of
// all players in filing order, and its 4 tries a tick are counted over the
// whole queue: with three requests of each of two players, the cursor
// goes 0, 4, 2, as for six requests of one player.
// spec: ECO-REQ-2.1 (ECO-REQ T39), ECO-REQ-2.2, ECO-REQ-0.1
#[test]
fn t39_one_queue_for_all_players() {
    const P1: PlayerId = PlayerId(1);
    // A world without warehouses: no request can be served.
    let mut world = world(flat(40, 12));
    world.roles.helper = Some(HELPER);
    for y in 7..12 {
        for x in 0..40 {
            let index = world.terrain.index((x, y));
            world.owners[index] = Some(P1);
            world.explored[index] |= crate::territory::player_bit(P1);
        }
    }
    world.recompute_all_placement();
    let flags = |y| (0..4).map(move |i| (2 + 5 * i, y)).collect::<Vec<Point>>();
    let (mine, theirs) = (flags(3), flags(9));
    let place = |player, at| Command::PlaceFlag { player, at };
    let road = |player, p: &[Point]| Command::BuildRoad {
        player,
        from: p[0],
        to: p[1],
    };
    let mut commands: Vec<Command> = mine.iter().map(|&f| place(P0, f)).collect();
    commands.extend(theirs.iter().map(|&f| place(P1, f)));
    apply_all(&mut world, commands);
    let mut commands: Vec<Command> = mine.windows(2).map(|p| road(P0, p)).collect();
    commands.extend(theirs.windows(2).map(|p| road(P1, p)));
    apply_all(&mut world, commands);
    let (queue, cursor) = world.request_queue();
    let players: Vec<PlayerId> = queue.iter().map(|r| r.player).collect();
    assert_eq!(players, [P0, P0, P0, P1, P1, P1], "in filing order");
    assert_eq!(cursor, 0);
    assert_eq!((world.requests(P0).len(), world.requests(P1).len()), (3, 3));
    world.step();
    assert_eq!(world.request_queue().1, 4);
    world.step();
    assert_eq!(world.request_queue().1, 2);
}

// ECO-REQ T14: equal route costs go to the warehouse created first.
// spec: ECO-REQ-3.2 (T14)
#[test]
fn t14_first_created_warehouse_wins_ties() {
    let mut stock = without(&full_stock(), HELPER);
    stock.push((HELPER, 0));
    let mut world = game(&stock);
    let depot = (16, 10);
    let middle = (13, 9);
    apply_all(&mut world, vec![place(DEPOT, depot), flag_cmd(middle)]);
    world.warehouses.get_mut(&depot).unwrap().add(BUILDER, 3);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, middle), road_cmd((16, 9), middle)],
    );
    let hut = (13, 10);
    assert_eq!(
        world.route_cost(P0, HQ_FLAG, middle),
        world.route_cost(P0, (16, 9), middle)
    );
    apply_all(&mut world, vec![place(HUT, hut)]);
    assert_eq!(count(&world, BUILDER), 4);
    assert_eq!(world.warehouse(depot).unwrap().count(BUILDER), 3);
}

// ECO-REQ T18 (ECO-REQ-3.2, TRN-CARRY-5.3): a settler request compares the
// warehouses by plain road lengths. T14's layout with a free carrier on the
// depot's road and none on the headquarters' road (3L in a ware network
// search) still ties, so the headquarters, created first, sends the builder.
// spec: ECO-REQ-3.2 (ECO-REQ T18), TRN-CARRY-5.3
#[test]
fn t18_settler_requests_count_plain_road_lengths() {
    let mut stock = without(&full_stock(), HELPER);
    stock.push((HELPER, 0));
    let mut world = game(&stock);
    let depot = (16, 10);
    let middle = (13, 9);
    apply_all(&mut world, vec![place(DEPOT, depot), flag_cmd(middle)]);
    world.warehouses.get_mut(&depot).unwrap().add(BUILDER, 3);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, middle), road_cmd((16, 9), middle)],
    );
    let depot_road = world
        .roads
        .iter()
        .position(|r| {
            r.as_ref()
                .is_some_and(|r| r.path[0] == (16, 9) || *r.path.last().unwrap() == (16, 9))
        })
        .unwrap();
    world.spawn_carrier(depot_road);
    // In the ware network only the depot's road counts.
    assert_eq!(world.route_cost(P0, HQ_FLAG, middle), None);
    assert!(world.route_cost(P0, (16, 9), middle).is_some());
    apply_all(&mut world, vec![place(HUT, (13, 10))]);
    assert_eq!(count(&world, BUILDER), 4, "the headquarters sent it");
    assert_eq!(world.warehouse(depot).unwrap().count(BUILDER), 3);
}

// ECO-REQ T37 (ECO-REQ-3.4, corrected 2026-10-30): B serves only when its own
// field is strictly nearer to the destination flag than A's, the buildings'
// positions, not their flags. A = the depot (16, 10) with builders, B = the
// headquarters (10, 10), which has none but can make one. To the site's flag
// (13, 10) the headquarters' flag is nearer (3 against 4), its field is not
// (3 and 3): the depot sends the builder.
// spec: ECO-REQ-3.4 (ECO-REQ T37), ECO-REQ-3.3
#[test]
fn t37_b_must_be_nearer_by_the_warehouses_own_fields() {
    let mut stock = without(&full_stock(), BUILDER);
    stock.push((BUILDER, 0));
    let mut world = game(&stock);
    let (depot, depot_flag, d, site) = ((16, 10), (16, 9), (13, 10), (12, 11));
    assert_eq!(
        (grid::distance(HQ_FLAG, d), grid::distance(depot_flag, d)),
        (3, 4)
    );
    assert_eq!((grid::distance(HQ, d), grid::distance(depot, d)), (3, 3));
    apply_all(&mut world, vec![place(DEPOT, depot), flag_cmd(d)]);
    world.warehouses.get_mut(&depot).unwrap().add(BUILDER, 3);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, d), road_cmd(depot_flag, d)],
    );
    assert_eq!(
        world.terrain.neighbour(site, super::FLAG_DIRECTION),
        Some(d)
    );
    let hammers = count(&world, HAMMER);
    apply_all(&mut world, vec![place(HUT, site)]);
    assert_eq!(
        world.warehouse(depot).unwrap().count(BUILDER),
        2,
        "the depot sent it"
    );
    assert_eq!(count(&world, HAMMER), hammers, "the headquarters made none");
}

// TRN-CARRY T26 (TRN-CARRY-5.3a, ECO-REQ-5.5; 8d46644): the storage choice
// reads the stored ware route costs. T14's layout, the two warehouses at
// equal cost from `middle`; the pair (middle, HQ flag) was asked while the
// HQ's road was longer, so its stored cost is higher and the depot is
// chosen although a fresh search ties (the HQ, created first, would win).
// spec: TRN-CARRY-5.3a (T26), ECO-REQ-5.5, ECO-REQ-3.2
#[test]
fn t26_the_storage_choice_reads_the_stored_costs() {
    let mut world = game(&full_stock());
    let (depot, depot_flag, middle) = ((16, 10), (16, 9), (13, 9));
    apply_all(&mut world, vec![place(DEPOT, depot), flag_cmd(middle)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, middle), road_cmd(depot_flag, middle)],
    );
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    let hq_road = world
        .roads
        .iter()
        .position(|r| r.as_ref().is_some_and(|r| r.path[0] == HQ_FLAG))
        .unwrap();
    world.roads[hq_road].as_mut().unwrap().length += 5;
    let stale = world.stored_ware_cost(P0, middle, HQ_FLAG).unwrap();
    world.roads[hq_road].as_mut().unwrap().length -= 5;
    let fresh = world.route_cost(P0, middle, HQ_FLAG).unwrap();
    assert_eq!(world.route_cost(P0, middle, depot_flag), Some(fresh));
    assert!(stale > fresh);
    let stone = world.new_ware_at(STONE, middle, None, None);
    while !world.tick().is_multiple_of(5) {
        world.step();
    }
    world.step();
    let ware = world.ware(stone).unwrap();
    assert_eq!(ware.destination, Some(depot_flag));
    assert!(ware.target.unwrap().storage);
}

// ECO-REQ T15: a full HQ flag sends nothing; the order is tried again
// 11 ticks later.
// spec: ECO-REQ-3.5, ECO-REQ-5.2 (T15)
#[test]
fn t15_full_flag_sends_nothing() {
    // No builder can be sent, so nothing else uses the HQ's cooldown.
    let stock = without(&without(&full_stock(), BUILDER), HAMMER);
    let mut world = game(&stock);
    let shed = (14, 12);
    apply_all(&mut world, vec![road_cmd_to_shed_flag()]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (14, 11))]);
    for _ in 0..40 {
        world.step();
    }
    // Six wares for an unconnected flag fill the HQ flag; no carrier
    // takes them and they are not stored.
    let blocking: Vec<usize> = (0..6)
        .map(|_| world.new_ware_at(STONE, HQ_FLAG, Some((30, 30)), None))
        .collect();
    apply_all(&mut world, vec![place(SHED, shed)]);
    while !world.tick().is_multiple_of(11) {
        world.step();
    }
    world.step();
    assert_eq!(count(&world, PLANK), 22);
    assert_eq!(world.orders().len(), 1);
    for ware in blocking {
        world.remove_ware(ware);
    }
    for _ in 0..10 {
        world.step();
        assert_eq!(count(&world, PLANK), 22);
    }
    world.step();
    assert_eq!(world.tick() % 11, 1);
    assert_eq!(count(&world, PLANK), 21);
    assert!(world.orders().is_empty());
}

fn road_cmd_to_shed_flag() -> Command {
    flag_cmd((14, 11))
}

/// A fresh game with a woodcutter's hut placed and joined to the
/// headquarters by a road (also the snapshot tests' economy).
pub(crate) fn fresh_game() -> World {
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    world
}

/// Everything from the HQ: the road's carrier, the builder, the planks
/// carried into the site; the builder goes home and the woodcutter
/// moves in.
// spec: ECO-REQ-1.1, ECO-REQ-1.3, ECO-REQ-1.4, ECO-REQ-4.1, ECO-HQ-4.4
#[test]
fn a_hut_built_from_a_fresh_game() {
    let mut world = fresh_game();
    let hut = (16, 12);
    let finished = run_until(&mut world, 5000, |w| w.site(hut).is_none());
    assert!(!world.building_at(hut).unwrap().under_construction);
    assert_eq!(count(&world, PLANK), 20);
    // The carrier stands on its road; the builder walks home.
    assert_eq!(world.carriers().len(), 1);
    run_until(&mut world, 2000, |w| {
        w.settlers()
            .any(|(_, s)| matches!(s.job, SettlerJob::Worker { .. }))
    });
    assert!(world.building_at(hut).unwrap().active);
    assert_eq!(count(&world, BUILDER), 5, "the builder came home");
    assert_eq!(count(&world, WOODCUTTER), 3);
    assert_eq!(count(&world, HELPER), 25);
    assert!(world.tick() > finished + 101);
    assert!(world.wares().next().is_none());
}

/// A sawmill on uneven ground: the leveller comes and goes home first; the
/// neighbours take the site's height of its placement, 400 / 19 = 21
/// (ECO-HQ-4.8a).
// spec: ECO-REQ-1.2, ECO-HQ-4.3, ECO-HQ-4.7, ECO-HQ-4.8a
#[test]
fn a_sawmill_levelled_and_built() {
    let mut terrain = flat(40, 40);
    let site = (16, 12);
    set_height(&mut terrain, site, 400);
    let mut world = game_with(terrain, &full_stock());
    apply_all(&mut world, vec![place(SAWMILL, site)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 8000, |w| w.site(site).is_none());
    run_until(&mut world, 2000, |w| w.settlers().next().is_none());
    assert_eq!(count(&world, LEVELLER), 3);
    assert_eq!(count(&world, BUILDER), 5);
    assert_eq!((count(&world, PLANK), count(&world, STONE)), (20, 32));
    assert_eq!(world.terrain().height_at(site), 400 / 19);
    for n in world.terrain().neighbours(site).into_iter().flatten() {
        assert_eq!(world.terrain().height_at(n), 400 / 19);
    }
}

/// A removed site sends its builder home.
// spec: ECO-REQ-2.3, ECO-HQ-5.1
#[test]
fn removing_a_site_sends_its_builder_home() {
    let mut world = game(&full_stock());
    let hut = (16, 12);
    apply_all(&mut world, vec![place(HUT, hut)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 3000, |w| {
        w.site(hut).is_some_and(|s| s.builder.is_some())
    });
    apply_all(
        &mut world,
        vec![Command::RemoveBuilding {
            player: P0,
            at: hut,
        }],
    );
    run_until(&mut world, 3000, |w| w.settlers().next().is_none());
    assert_eq!(count(&world, BUILDER), 5);
}

// TRN-FREE T37, T38 (TRN-FREE-6.4b): a site removed while its builder walks
// to it: from the next tick he is going home, walks on to the next flag he
// reaches and not to the site; he stands on that flag's point.
// spec: TRN-FREE-6.4b (TRN-FREE T37, TRN-FREE T38), TRN-FREE-6.4a
#[test]
fn t37_a_builder_on_his_way_goes_home_when_the_site_goes() {
    let mut world = game(&full_stock());
    let hut = (16, 12);
    apply_all(&mut world, vec![place(HUT, hut)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    let walking = |w: &World| {
        w.settlers().any(|(_, s)| {
            matches!(s.job, SettlerJob::ToSite { builder: true, .. }) && s.walker.next.is_some()
        })
    };
    run_until(&mut world, 3000, walking);
    let id = world
        .settlers()
        .find(|(_, s)| matches!(s.job, SettlerJob::ToSite { builder: true, .. }))
        .map(|(id, _)| id)
        .unwrap();
    apply_all(
        &mut world,
        vec![Command::RemoveBuilding {
            player: P0,
            at: hut,
        }],
    );
    let s = world.settlers[id].as_ref().unwrap();
    assert!(matches!(s.job, SettlerJob::Leaving { .. }), "{:?}", s.job);
    while world.settlers[id].as_ref().unwrap().walker.is_walking() {
        world.step();
        assert!(world.tick() < 4000, "stops");
    }
    let s = world.settlers[id].as_ref().unwrap();
    assert!(
        world.flag_at(s.walker.at).is_some(),
        "on a flag: {:?}",
        s.walker.at
    );
    assert_ne!(s.walker.at, (16, 12), "not on the site");
}

// ECO-REQ T36 (ECO-REQ-2.5): the skip after a failed try is keyed by the
// profession and the land road network of the requester's flag, not by the
// requester: another road of the same network is skipped, a road whose
// flags are not connected yet is tried, another profession is tried.
// spec: ECO-REQ-2.5 (ECO-REQ T36)
#[test]
fn t36_the_skip_is_keyed_by_profession_and_network() {
    let mut world = game(&full_stock());
    let (x, y, z) = ((14, 9), (16, 11), (6, 15));
    apply_all(&mut world, vec![flag_cmd(x), flag_cmd(y), flag_cmd(z)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, x), road_cmd(x, y)]);
    let roads: Vec<usize> = (0..world.roads.len())
        .filter(|&i| world.roads[i].is_some())
        .collect();
    let (r1, r2) = (roads[0], roads[1]);
    let carrier = |road| Request {
        player: P0,
        requester: Requester::Road(road),
        profession: HELPER,
        harbour: None,
    };
    assert!(
        world.same_request_key(&carrier(r1), &carrier(r2)),
        "same network"
    );
    // A lone flag: a network of its own.
    let lone = Request {
        player: P0,
        requester: Requester::Specialist { flag: z, order: 1 },
        profession: HELPER,
        harbour: None,
    };
    assert!(!world.same_request_key(&carrier(r1), &lone));
    let other = Request {
        player: P0,
        requester: Requester::Road(r2),
        profession: BUILDER,
        harbour: None,
    };
    assert!(
        !world.same_request_key(&carrier(r1), &other),
        "another profession"
    );
}

// ECO-REQ-2.3a: a request whose link to its requester was cancelled stays
// in the queue and is deleted unserved when its turn comes; each deletion
// takes one of the tick's 4 turns and leaves the cursor where it is, so the
// next request takes its place.
// spec: ECO-REQ-2.3a, ECO-REQ-2.3, ECO-REQ-2.2
#[test]
fn an_unlinked_request_is_deleted_on_its_turn() {
    let mut world = game(&full_stock());
    let unlinked = Request {
        player: P0,
        requester: Requester::Unlinked,
        profession: BUILDER,
        harbour: None,
    };
    (world.queue, world.queue_cursor) = (vec![unlinked; 5], 0);
    world.process_requests();
    assert_eq!(world.requests(P0), [unlinked], "four turns, four deleted");
    assert_eq!(world.queue_cursor, 0, "the cursor stays");
    world.process_requests();
    assert!(world.requests(P0).is_empty());
}

// ECO-REQ T38 (ECO-REQ-2.3a, TRN-ROAD-4.2): the open carrier request of a
// road that a new flag splits stays in the queue, unlinked, in its place;
// the halves file requests of their own behind it, and the old one is
// deleted unserved on its turn.
// spec: ECO-REQ-2.3a (ECO-REQ T38), TRN-ROAD-4.2
#[test]
fn t38_a_split_roads_request_stays_unlinked_in_its_place() {
    let mut stock = without(&full_stock(), HELPER);
    stock.push((HELPER, 0));
    let mut world = game(&stock);
    let far = (16, 9);
    apply_all(&mut world, vec![flag_cmd(far)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    let old = world.requests(P0).to_vec();
    assert_eq!(old.len(), 1, "{old:?}");
    assert!(matches!(old[0].requester, Requester::Road(_)));
    let middle = world.road(0).unwrap().path[3];
    // The split alone, before the tick's turns of the queue.
    assert_eq!(world.apply(&flag_cmd(middle)), Ok(()));
    let now = world.requests(P0).to_vec();
    assert_eq!(
        now,
        [Request {
            requester: Requester::Unlinked,
            ..old[0]
        }],
        "in its place"
    );
    // The halves file theirs behind it in the tick; its turn deletes it.
    world.step();
    let now = world.requests(P0).to_vec();
    assert_eq!(now.len(), 2, "{now:?}");
    assert!(now
        .iter()
        .all(|r| matches!(r.requester, Requester::Road(_))));
}

// TRN-FREE-6.4b (Q-366 a): a site removed while its builder hammers with
// time left in his action: he shows the action in the removal tick and in
// T, the tick after (it runs on, counted down), and in T + 1 the fine walk
// back to the flag's point replaces it.
// spec: TRN-FREE-6.4b, TRN-FREE-6.4a
#[test]
fn a_removed_sites_builder_works_on_in_t() {
    use crate::construction::BuilderPose;
    use crate::trace::Action;
    let mut world = game(&full_stock());
    // Offsets from the flag's sample, as every real type has (ECO-HQ-4.6a):
    // he works off the flag's point.
    world
        .building_types
        .get_mut(&HUT)
        .unwrap()
        .construction_positions = [[(-1, 2), (-2, 1), (1, 2), (0, 2)]; 3];
    let hut = (16, 12);
    apply_all(&mut world, vec![place(HUT, hut)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 3000, |w| {
        matches!(
            w.builder_pose(hut),
            Some(BuilderPose::Working { since: 10, .. })
        )
    });
    let builder = |world: &World| {
        world
            .trace_settlers()
            .into_iter()
            .find(|r| r.job == "Home")
            .map(|r| r.action)
    };
    apply_all(
        &mut world,
        vec![Command::RemoveBuilding {
            player: P0,
            at: hut,
        }],
    );
    assert_eq!(
        builder(&world),
        Some(Action::WorkLoop(0)),
        "the removal tick"
    );
    world.step();
    assert_eq!(builder(&world), Some(Action::WorkLoop(0)), "T");
    world.step();
    assert_eq!(builder(&world), Some(Action::Walk), "T + 1");
}

// TRN-FREE-6.1 (TRN-FREE T31): of two equal road routes from F to D (over
// B or over C, three steps each road) a walker takes the one read out back
// from D by the order the roads were attached to its flags: at D the road
// built first, whichever route has the lower fields.
// spec: TRN-FREE-6.1 (TRN-FREE T31)
#[test]
fn t31_equal_routes_by_the_roads_order_at_the_destination() {
    let (f, b, c, d) = ((12, 11), (15, 11), (12, 14), (15, 14));
    for b_first in [true, false] {
        let mut world = game(&full_stock());
        let flags = [f, b, c, d].map(|at| Command::PlaceFlag { player: P0, at });
        apply_all(&mut world, flags.to_vec());
        let into_d = if b_first {
            [road_cmd(b, d), road_cmd(c, d)]
        } else {
            [road_cmd(c, d), road_cmd(b, d)]
        };
        for command in [road_cmd(f, b), road_cmd(f, c)].into_iter().chain(into_d) {
            apply_all(&mut world, vec![command]);
            assert!(world.last_results().iter().all(|r| r.outcome == Ok(())));
        }
        let length = |x: Point, y: Point| {
            world
                .roads()
                .find(|r| {
                    let ends = (r.path[0], *r.path.last().unwrap());
                    ends == (x, y) || ends == (y, x)
                })
                .map(|r| r.length)
                .unwrap()
        };
        assert_eq!(length(f, b) + length(b, d), length(f, c) + length(c, d));
        let fields = world.road_walk(P0, f, d).unwrap();
        let (via, not) = if b_first { (b, c) } else { (c, b) };
        assert!(fields.contains(&via) && !fields.contains(&not), "{b_first}");
    }
}

// TRN-FREE-6.4b (TRN-FREE T33): a site removed while its builder walks his
// approach (out three lattice steps and back, waiting for material): he
// walks it to its end in the going-home state, on the very samples the
// approach would have taken, and walks home from the flag in the tick after
// he is back on its point.
// spec: TRN-FREE-6.4b (TRN-FREE T33), ECO-HQ-4.6b
#[test]
fn t33_the_approach_walk_is_walked_to_its_end() {
    let mut world = game(&without(&full_stock(), PLANK));
    let hut = (16, 12);
    apply_all(&mut world, vec![place(HUT, hut)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 3000, |w| {
        w.approach_left(hut).is_some_and(|a| a.clock == 5)
    });
    let left = world.approach_left(hut).unwrap();
    let remaining = left.out_ticks + left.back_ticks - left.clock;
    // The approach as it would go on, in a copy of the game.
    let mut kept = world.clone();
    let mut expected = Vec::new();
    for _ in 0..remaining {
        kept.step();
        let row = kept
            .trace_settlers()
            .into_iter()
            .find(|r| r.job.starts_with("Builder"))
            .unwrap();
        expected.push((row.field, row.fine));
    }
    apply_all(
        &mut world,
        vec![Command::RemoveBuilding {
            player: P0,
            at: hut,
        }],
    );
    let home = |world: &World| {
        world
            .trace_settlers()
            .into_iter()
            .find(|r| r.job == "Home")
            .map(|r| (r.field, r.fine))
            .unwrap()
    };
    // The removal took a tick of the copy's.
    for want in &expected[1..] {
        world.step();
        assert_eq!(&home(&world), want);
    }
    // On the flag's point; the walk home starts in the next tick.
    let flag = world.terrain().neighbour(hut, 1).unwrap();
    let (c, r) = crate::grid::field_sample(flag);
    assert_eq!(home(&world).1, (i64::from(c) * 1000, i64::from(r) * 1000));
    world.step();
    world.step();
    assert_ne!(
        home(&world).1,
        (i64::from(c) * 1000, i64::from(r) * 1000),
        "on his way home"
    );
}

// TRN-FREE-6.4b (TRN-FREE T47): a site removed while its builder walks the
// approach's back leg: that walk already ends on the flag's point, so going
// home has nothing left to walk there; he arrives in A, stands in A and
// A + 1, and his walk home starts at progress 0 in A + 2 (one tick later
// than after the out leg, T33).
// spec: TRN-FREE-6.4b (TRN-FREE T47), TRN-FREE-6.4a
#[test]
fn t47_on_the_back_leg_the_walk_home_starts_a_tick_later() {
    let mut world = game(&without(&full_stock(), PLANK));
    let hut = (16, 12);
    apply_all(&mut world, vec![place(HUT, hut)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 3000, |w| {
        w.approach_left(hut)
            .is_some_and(|a| a.clock == a.out_ticks + 1 && a.back_ticks > 2)
    });
    apply_all(
        &mut world,
        vec![Command::RemoveBuilding {
            player: P0,
            at: hut,
        }],
    );
    let home = |world: &World| {
        world
            .trace_settlers()
            .into_iter()
            .find(|r| r.job == "Home")
            .map(|r| r.fine)
            .unwrap()
    };
    let flag = world.terrain().neighbour(hut, 1).unwrap();
    let (c, r) = crate::grid::field_sample(flag);
    let point = (i64::from(c) * 1000, i64::from(r) * 1000);
    let mut arrived = None;
    for _ in 0..40 {
        if home(&world) == point {
            arrived = Some(world.tick());
            break;
        }
        world.step();
    }
    let a = arrived.expect("back on the flag's point");
    // A and A + 1 standing, A + 2 the walk home at progress 0, A + 3 off it.
    world.step();
    assert_eq!(home(&world), point, "A + 1, tick {}", world.tick());
    world.step();
    assert_eq!(home(&world), point, "A + 2: progress 0");
    world.step();
    assert_ne!(home(&world), point, "A + 3: on his way home");
    assert_eq!(world.tick(), a + 3);
}

// TRN-CARRY-5.4 (TRN-CARRY T30): a ware's next flag is found as a walk's
// route is: the costs searched from the ware's flag, the route read out back
// from the destination by the order the roads were attached to each flag.
// Two equal routes from F to D over B and over C, every road with a free
// carrier: the next flag is B when D's road to B was built first, else C.
// spec: TRN-CARRY-5.4 (TRN-CARRY T30), TRN-FREE-6.1
#[test]
fn trn_carry_t30_the_next_flag_by_the_read_out() {
    for b_first in [true, false] {
        let mut world = game(&full_stock());
        let (f, b, c, d) = (2..20)
            .flat_map(|y| (2..20).map(move |x| (x, y)))
            .map(|(x, y): Point| ((x, y), (x + 4, y), (x, y + 4), (x + 4, y + 4)))
            .find(|&(f, b, c, d)| {
                [f, b, c, d]
                    .iter()
                    .all(|&at| world.check_flag(P0, at).is_ok())
            })
            .unwrap();
        let flags = [f, b, c, d].map(|at| Command::PlaceFlag { player: P0, at });
        apply_all(&mut world, flags.to_vec());
        let into_d = if b_first {
            [road_cmd(b, d), road_cmd(c, d)]
        } else {
            [road_cmd(c, d), road_cmd(b, d)]
        };
        for command in into_d.into_iter().chain([road_cmd(f, b), road_cmd(f, c)]) {
            apply_all(&mut world, vec![command]);
            assert!(world.last_results().iter().all(|r| r.outcome == Ok(())));
        }
        for road in 0..world.roads.len() {
            if world.roads[road]
                .as_ref()
                .is_some_and(|r| r.carrier.is_none())
            {
                world.spawn_carrier(road);
            }
        }
        assert_eq!(world.route_cost(P0, f, d), Some(16), "4 + 4 either way");
        let next = world.next_flag(P0, f, d);
        assert_eq!(next, Some(if b_first { b } else { c }), "{b_first}");
    }
}

// TRN-FREE-6.1 (Q-280): a split road's halves are new roads appended to
// their flags' lists, neither keeping the old road's place. Equal routes
// from F to D over B and over C; at D the road to B was built first (from
// D), so the walk goes over B; a flag put on that road makes its half at D
// the last of D's roads, and the walk goes over C. TRN-CARRY T32: the same
// order for a ware's next flag (TRN-CARRY-5.4) and the computer player's
// list of route flags (AI-CON-8.3 c), once the roads have their carriers.
// spec: TRN-FREE-6.1 (TRN-FREE T31, Q-280), TRN-ROAD-4.2, TRN-CARRY-5.4 (TRN-CARRY T32), AI-CON-8.3
#[test]
fn q280_a_split_roads_halves_come_last_in_the_order() {
    let mut world = game(&full_stock());
    // A square of four flags four fields apart inside the territory.
    let (f, b, c, d) = (2..20)
        .flat_map(|y| (2..20).map(move |x| (x, y)))
        .map(|(x, y): Point| ((x, y), (x + 4, y), (x, y + 4), (x + 4, y + 4)))
        .find(|&(f, b, c, d)| {
            [f, b, c, d]
                .iter()
                .all(|&at| world.check_flag(P0, at).is_ok())
        })
        .unwrap();
    let flags = [f, b, c, d].map(|at| Command::PlaceFlag { player: P0, at });
    apply_all(&mut world, flags.to_vec());
    assert!(world.last_results().iter().all(|r| r.outcome == Ok(())));
    for command in [
        road_cmd(d, b),
        road_cmd(c, d),
        road_cmd(f, b),
        road_cmd(f, c),
    ] {
        apply_all(&mut world, vec![command]);
        assert!(world.last_results().iter().all(|r| r.outcome == Ok(())));
    }
    let length = |world: &World, x: Point, y: Point| {
        world
            .roads()
            .find(|r| {
                let ends = (r.path[0], *r.path.last().unwrap());
                ends == (x, y) || ends == (y, x)
            })
            .map(|r| r.length)
            .unwrap()
    };
    assert_eq!(
        length(&world, f, b) + length(&world, b, d),
        length(&world, f, c) + length(&world, c, d)
    );
    let via = |world: &World, x: Point| world.road_walk(P0, f, d).unwrap().contains(&x);
    assert!(via(&world, b) && !via(&world, c));
    // Joined to the headquarters for the carriers: a road appended last at
    // D, behind both roads of the equal routes.
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, d)]);
    assert!(world.last_results().iter().all(|r| r.outcome == Ok(())));
    let manned = |w: &World| w.roads().all(|r| r.carrier.is_some());
    run_until(&mut world, 2000, manned);
    assert_eq!(world.next_flag(P0, f, d), Some(b));
    assert_eq!(world.ware_route_flags(P0, f, d), Some(vec![f, b, d]));
    // The road D-B split at its middle field.
    let middle = world
        .roads()
        .find(|r| r.path[0] == d && *r.path.last().unwrap() == b)
        .map(|r| r.path[r.path.len() / 2])
        .unwrap();
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: middle,
        }],
    );
    assert!(world.last_results().iter().all(|r| r.outcome == Ok(())));
    assert!(via(&world, c) && !via(&world, b));
    run_until(&mut world, 2000, manned);
    assert_eq!(world.next_flag(P0, f, d), Some(c));
    assert_eq!(world.ware_route_flags(P0, f, d), Some(vec![f, c, d]));
}

/// The same commands give the same world, with requests, settlers and
/// wares under way.
// spec: none (determinism of the dispatch: same commands, same world)
#[test]
fn dispatch_is_deterministic() {
    let run = || {
        let mut world = game(&full_stock());
        apply_all(&mut world, vec![place(SAWMILL, (16, 12))]);
        apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
        for _ in 0..700 {
            world.step();
        }
        world
    };
    assert_eq!(run(), run());
}

/// The first warehouse helper of ECO-REQ-5.3c (the lowest settler id
/// alive), if there is one, and its step.
fn helper(world: &World) -> Option<(crate::Settler, crate::settlers::CarryOutStep)> {
    world.settlers().find_map(|(_, s)| match s.job {
        crate::SettlerJob::CarryOut { step, .. } => Some((s.clone(), step)),
        _ => None,
    })
}

/// That settler by id.
fn helper_by_id(world: &World, id: usize) -> Option<crate::settlers::CarryOutStep> {
    world.settlers().find_map(|(n, s)| match s.job {
        crate::SettlerJob::CarryOut { step, .. } if n == id => Some(step),
        _ => None,
    })
}

// ECO-REQ-5.3c (T16): the helper stands at the door in the matching tick t
// and in t + 1, its walk is ordered in t + 2 at progress 0; at the flag it
// puts the ware down (4 ticks, the ware lies there from the end), picks up
// nothing (4), stands 4 and walks back 12 ticks after arriving. The test
// headquarters' door is its own field, as a harbour's (T23): on the field
// in his arrival tick, at the door in the next, gone in the one after, and
// the helper is back in stock.
// spec: ECO-REQ-5.3 (b), ECO-REQ-5.3c (T16, T23), ECO-WORK-3.8b
#[test]
fn t16_the_helper_carries_the_ware_out() {
    use crate::settlers::CarryOutStep as Step;
    let mut world = game(&full_stock());
    let site = (16, 12);
    apply_all(&mut world, vec![place(HUT, site)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    let t = run_until(&mut world, 200, |w| helper(w).is_some());
    let id = world
        .settlers()
        .find(|(_, s)| matches!(s.job, crate::SettlerJob::CarryOut { .. }))
        .unwrap()
        .0;
    let door = world.door_field(HQ);
    let (h, step) = helper(&world).unwrap();
    assert_eq!(
        (h.walker.at, step, h.walker.has_walk()),
        (door, Step::Door { score: 0 }, false)
    );
    assert_eq!(wares_of(&world, PLANK), 0);
    world.step();
    assert_eq!(helper(&world).unwrap().1, Step::Door { score: 0 });
    world.step();
    let (h, step) = helper(&world).unwrap();
    assert_eq!((step, h.walker.progress_bits), (Step::Out, 0));
    let arrived = run_until(&mut world, 200, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::PutDown)
    });
    assert_eq!(helper(&world).unwrap().0.walker.at, HQ_FLAG);
    for _ in 0..3 {
        world.step();
        assert_eq!(wares_of(&world, PLANK), 0);
    }
    world.step();
    assert_eq!(wares_of(&world, PLANK), 1, "the ware lies at the flag");
    assert_eq!(helper(&world).unwrap().1, Step::Pickup);
    let back = run_until(&mut world, 200, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::Back)
    });
    assert_eq!(back - arrived, 12);
    let arrived_in = run_until(&mut world, 200, |w| helper_by_id(w, id) == Some(Step::In));
    assert_eq!(world.door_field(HQ), HQ);
    let inside = count(&world, HELPER);
    world.step();
    assert_eq!(helper_by_id(&world, id), Some(Step::In), "at the door");
    world.step();
    assert!(
        helper_by_id(&world, id).is_none(),
        "gone two ticks after {arrived_in}"
    );
    assert_eq!(count(&world, HELPER), inside + 1);
    assert!(t < arrived);
}

// TRN-CARRY T41 (TRN-CARRY-4.3): a ware counts once at its flag: as
// reserved until its put-down ends, as lying from then on, also in the
// tick a warehouse helper's put-down ends.
// spec: TRN-CARRY-4.3 (TRN-CARRY T41), ECO-REQ-5.3c
#[test]
fn t41_a_helpers_ware_counts_once_as_it_is_put_down() {
    use crate::settlers::CarryOutStep as Step;
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 400, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::PutDown)
    });
    let lying = world.wares_at(HQ_FLAG).len();
    assert_eq!(
        world.flag_load(HQ_FLAG),
        lying + 1,
        "reserved while put down"
    );
    run_until(&mut world, 50, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::Pickup)
    });
    assert_eq!(world.wares_at(HQ_FLAG).len(), lying + 1, "lying now");
    assert_eq!(world.flag_load(HQ_FLAG), lying + 1, "counted once");
}

// TRN-ROAD T32 (TRN-ROAD-6.3, corrected 2026-10-30, Q-294): the road of a
// carrier who carries a stone towards its far flag, bound to a stone lying
// there for the exchange, goes. The carried stone is destroyed; the bound
// one stays at the flag, claimed by him while he walks home, and is free in
// the tick he is taken in at the headquarters.
// spec: TRN-ROAD-6.3 (TRN-ROAD T32)
#[test]
fn t32_a_released_carriers_exchange_ware_stays_bound_until_he_is_home() {
    use crate::transport::{CarrierTask, Entry};
    let mut world = game(&full_stock());
    let far = (16, 9);
    apply_all(&mut world, vec![flag_cmd(far)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    run_until(&mut world, 600, |w| {
        w.carriers()
            .first()
            .is_some_and(|c| c.task == CarrierTask::Idle && !c.walker.is_walking())
    });
    world.new_ware_at(STONE, HQ_FLAG, Some(far), None);
    run_until(
        &mut world,
        400,
        |w| matches!(w.carriers()[0].task, CarrierTask::Carry { to, .. } if to == far),
    );
    let back = world.new_ware_at(STONE, far, Some(HQ_FLAG), None);
    run_until(&mut world, 200, |w| {
        matches!(
            w.carriers()[0].task,
            CarrierTask::Carry { entry: Entry::Exchange(x), .. } if x == back
        )
    });
    let number = world.carriers()[0].number;
    let middle = world.road(0).unwrap().path[1];
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: middle,
        }],
    );
    assert_eq!(world.carriers()[0].task, CarrierTask::Retired);
    let mut ticks = 0;
    while world.settlers().any(|(_, s)| s.number == number) {
        assert_eq!(world.ware(back).unwrap().claimed_by, Some(0));
        world.step();
        ticks += 1;
        assert!(ticks < 2000, "never home");
    }
    assert!(ticks > 1);
    assert_eq!(world.ware(back).unwrap().claimed_by, None, "free now");
}

// TRN-ROAD-6.3 (Q-294): a plain claim, a ware the released carrier was going
// to fetch, is let go as his walk home starts.
// spec: TRN-ROAD-6.3
#[test]
fn a_released_carriers_plain_claim_goes_as_he_starts_home() {
    use crate::transport::CarrierTask;
    let mut world = game(&full_stock());
    let far = (16, 9);
    apply_all(&mut world, vec![flag_cmd(far)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    run_until(&mut world, 600, |w| {
        w.carriers()
            .first()
            .is_some_and(|c| c.task == CarrierTask::Idle && !c.walker.is_walking())
    });
    let stone = world.new_ware_at(STONE, far, Some(HQ_FLAG), None);
    run_until(&mut world, 200, |w| {
        w.ware(stone).unwrap().claimed_by.is_some()
    });
    let number = world.carriers()[0].number;
    let middle = world.road(0).unwrap().path[1];
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: middle,
        }],
    );
    // He notices in his next update.
    world.step();
    assert!(
        world.settlers().any(|(_, s)| s.number == number),
        "on his way home"
    );
    assert_eq!(world.ware(stone).unwrap().claimed_by, None);
}

// ECO-REQ T22 (ECO-REQ-5.3g, Q-284): a warehouse helper whose warehouse
// and flag are removed while he walks out walks on to the flag field and
// in his arrival tick goes home at once: no put-down or pick-up, and the
// plank in his hands is destroyed, not put on the field where no flag is.
// spec: ECO-REQ-5.3g (ECO-REQ T22), TRN-ROAD-6.3, TRN-FREE-6.4
#[test]
fn t22_a_helper_whose_warehouse_is_gone_goes_home_on_arrival() {
    use crate::settlers::CarryOutStep as Step;
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 200, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::Out)
    });
    let id = world
        .settlers()
        .find(|(_, s)| matches!(s.job, crate::SettlerJob::CarryOut { .. }))
        .unwrap()
        .0;
    world.remove_flag(HQ_FLAG);
    assert!(world.flag_at(HQ_FLAG).is_none() && world.building_at(HQ).is_none());
    // The walk is not cut.
    while helper_by_id(&world, id) == Some(Step::Out) {
        assert!(world
            .settlers()
            .any(|(n, s)| n == id && s.walker.is_walking()));
        world.step();
    }
    // The arrival tick: home at once, the plank gone (with no warehouse
    // left, his first wander step is ordered, TRN-FREE-6.4 step 4).
    assert_eq!(helper_by_id(&world, id), None, "going home");
    let s = world.settlers().find(|(n, _)| *n == id).unwrap().1;
    assert_eq!((s.walker.at, s.carrying), (HQ_FLAG, None));
    assert_eq!(wares_of(&world, PLANK), 0);
}

// ECO-REQ T22 (ECO-REQ-5.3g): with only the order's building removed, the
// helper notices nothing: he puts the plank down on his warehouse's flag,
// without a destination, and 5.5 stores it (here at once: the flag is the
// warehouse's own).
// spec: ECO-REQ-5.3g (ECO-REQ T22), ECO-REQ-5.3c, ECO-REQ-5.5
#[test]
fn t22_a_helper_whose_site_is_gone_puts_the_ware_down() {
    use crate::settlers::CarryOutStep as Step;
    let mut world = game(&full_stock());
    let site = (16, 12);
    apply_all(&mut world, vec![place(HUT, site)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 200, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::Out)
    });
    assert_eq!(world.remove_building(P0, site), Ok(()));
    let stored = count(&world, PLANK);
    run_until(&mut world, 200, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::PutDown)
    });
    run_until(&mut world, 200, |w| {
        helper(w).is_some_and(|(_, s)| s == Step::Pickup)
    });
    run_until(&mut world, 50, |w| count(w, PLANK) == stored + 1);
    assert_eq!(wares_of(&world, PLANK), 0, "not destroyed: back in store");
}

// ECO-REQ-5.3f (ECO-REQ T20): a served order comes back when its ware is
// delivered, last among the open orders with the stamp it was chosen with,
// and the building drops its open order of the good that was made first:
// the older open one when the served order is newer, the served one
// itself when it is the oldest (the open orders as before).
// spec: ECO-REQ-5.3f (T20)
#[test]
fn t20_a_served_order_comes_back_at_the_delivery() {
    let set_up = || {
        let mut world = game(&full_stock());
        let site = (16, 12);
        apply_all(&mut world, vec![place(HUT, site)]);
        world.orders.clear();
        world.order(site, PLANK);
        world.order(site, PLANK);
        world.order(site, STONE);
        (world, site)
    };
    let planks = |w: &World, site| -> Vec<(u64, u64)> {
        w.orders()
            .iter()
            .filter(|o| o.target.at == site && o.good == PLANK)
            .map(|o| (o.made, o.stamp))
            .collect()
    };
    // The newer order served (chosen at 500): the older one goes.
    let (mut world, site) = set_up();
    let target = world.target_of(site).unwrap();
    let [first, second] = [0, 1].map(|i| world.orders()[i]);
    world.orders.remove(1);
    let served = crate::Order {
        stamp: 500,
        ..second
    };
    world.hand_over(target, PLANK, Some(served));
    assert_eq!(planks(&world, site), [(second.made, 500)]);
    assert_eq!(world.orders().last().unwrap().good, PLANK, "appended last");
    // The older order served: it is the one dropped again.
    let (mut world, site) = set_up();
    world.orders.remove(0);
    let served = crate::Order {
        stamp: 400,
        ..first
    };
    world.hand_over(target, PLANK, Some(served));
    assert_eq!(planks(&world, site), [(second.made, 0)]);
    assert_eq!(world.orders().len(), 2);
    // A ware without an order (stored, say) changes no order.
    let (mut world, site) = set_up();
    world.hand_over(target, PLANK, None);
    assert_eq!(planks(&world, site), [(first.made, 0), (second.made, 0)]);
}

// ECO-REQ-5.3c, TRN-CARRY-4.4 (ECO-REQ T19): the helper's walk waits for
// its ware's entry check: the check of t + 1 raises the flag's bar, the one
// of t + 2 admits the ware (its place counts from then) and orders the
// walk. A ware leaving the flag in t + 2 before the check (here: the bar
// reset after t + 1) makes that check raise the bar again: admitted and
// walking from t + 3.
// spec: ECO-REQ-5.3c (T19), TRN-CARRY-4.4, TRN-CARRY-4.3
#[test]
fn t19_the_helper_waits_for_its_entry_check() {
    use crate::settlers::CarryOutStep as Step;
    let made = || {
        let mut world = game(&full_stock());
        apply_all(&mut world, vec![place(HUT, (16, 12))]);
        apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
        run_until(&mut world, 200, |w| helper(w).is_some());
        world
    };
    let mut world = made();
    let load = world.flag_load(HQ_FLAG);
    world.step();
    assert_eq!(
        world.flag_bar.get(&HQ_FLAG),
        Some(&0),
        "t + 1 raises the bar"
    );
    assert_eq!(world.flag_load(HQ_FLAG), load, "no place yet");
    world.step();
    assert_eq!(helper(&world).unwrap().1, Step::Out);
    assert_eq!(world.flag_load(HQ_FLAG), load + 1, "admitted in t + 2");
    assert_eq!(world.flag_bar.get(&HQ_FLAG), None);
    let mut world = made();
    world.step();
    world.flag_bar.remove(&HQ_FLAG);
    world.step();
    assert_eq!(helper(&world).unwrap().1, Step::Door { score: 0 });
    assert_eq!(world.flag_bar.get(&HQ_FLAG), Some(&0));
    world.step();
    let (h, step) = helper(&world).unwrap();
    assert_eq!((step, h.walker.progress_bits), (Step::Out, 0));
}

// ECO-REQ T35 (ECO-REQ-5.3c): with the flag's bar already at 0 when the
// helper is made, his first check (t + 1) admits the ware and his walk out
// is ordered in that tick: no fixed wait at the door.
// spec: ECO-REQ-5.3c (ECO-REQ T35), TRN-CARRY-4.4
#[test]
fn t35_with_the_bar_at_0_the_helper_walks_in_t_plus_1() {
    use crate::settlers::CarryOutStep as Step;
    let mut world = game(&full_stock());
    apply_all(&mut world, vec![place(HUT, (16, 12))]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    run_until(&mut world, 200, |w| helper(w).is_some());
    world.flag_bar.insert(HQ_FLAG, 0);
    world.step();
    let (h, step) = helper(&world).unwrap();
    assert_eq!(
        (step, h.walker.progress_bits),
        (Step::Out, 0),
        "walking from t + 1"
    );
}

// TRN-FREE-6.4 steps 1 to 3 (Q-361, TRN-FREE T26): a carrier released while
// it stands in a building (here the shed's site, handing over a plank)
// cannot start a road walk there, so step 1 is skipped: step 2 looks from
// the site's flag (no road left, nothing found), step 3 finds the
// headquarters on the landmass and the carrier free-walks to its network
// flag nearest to it, the headquarters' flag.
// spec: TRN-FREE-6.4, TRN-FREE-6.5, TRN-CARRY-7.1b
#[test]
fn a_carrier_released_inside_a_building_does_not_walk_over_the_roads() {
    let mut world = game(&full_stock());
    let shed = (14, 12);
    let shed_flag = world.terrain().neighbour(shed, 1).unwrap();
    apply_all(
        &mut world,
        vec![Command::PlaceBuilding {
            player: P0,
            building: SHED,
            at: shed,
        }],
    );
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, shed_flag)]);
    let road = world.roads().count() - 1;
    let mid = world.road(road).unwrap().path[1];
    run_until(&mut world, 2000, |w| {
        w.carriers()
            .iter()
            .any(|c| c.road == road && c.walker.at == shed && c.walker.next.is_none())
    });
    let number = world
        .carriers()
        .iter()
        .find(|c| c.road == road)
        .unwrap()
        .number;
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: mid,
        }],
    );
    run_until(&mut world, 20, |w| {
        w.settlers().any(|(_, s)| {
            s.number == number
                && matches!(
                    s.job,
                    crate::SettlerJob::Released {
                        step: crate::ReleasedStep::ToFlag,
                        ..
                    }
                )
        })
    });
    let s = world
        .settlers()
        .map(|(_, s)| s)
        .find(|s| s.number == number)
        .unwrap();
    assert_eq!(s.walker.path.back(), Some(&HQ_FLAG));
}

// TRN-CARRY-5.3a (Q-362): stored ware route costs. A pair asked before (in
// either order) gives its stored value although the roads' costs changed;
// "no route" is stored too; a flag to itself costs 0. The player's store
// is emptied only by a ware renumbering: a road getting its carrier while
// its ends are in two ware networks (SIM-START-4.3 b), or a road with a
// carrier removed (4.3 c); a road getting its carrier inside one ware
// network leaves it.
// spec: TRN-CARRY-5.3a, SIM-START-4.3
#[test]
fn stored_route_costs_stay_until_a_ware_renumbering() {
    let mut world = game(&full_stock());
    let (far, other) = ((14, 9), (14, 13));
    apply_all(&mut world, vec![flag_cmd(far), flag_cmd(other)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    let carried = |w: &World, a: Point, b: Point| {
        w.roads().any(|r| {
            r.carrier.is_some()
                && ((r.path[0] == a && *r.path.last().unwrap() == b)
                    || (r.path[0] == b && *r.path.last().unwrap() == a))
        })
    };
    run_until(&mut world, 200, |w| carried(w, HQ_FLAG, far));
    let first = world.stored_ware_cost(P0, far, HQ_FLAG).expect("connected");
    assert_eq!(world.stored_ware_cost(P0, far, far), Some(0));
    assert_eq!(world.stored_ware_cost(P0, HQ_FLAG, other), None);
    // The road's costs change: the stored value stays, in both orders.
    let id = world
        .roads
        .iter()
        .position(|r| r.as_ref().is_some_and(|r| r.path.contains(&far)))
        .unwrap();
    world.roads[id].as_mut().unwrap().length += 5;
    assert_ne!(world.route_cost(P0, far, HQ_FLAG), Some(first));
    assert_eq!(world.stored_ware_cost(P0, HQ_FLAG, far), Some(first));
    // A new road joins `other` to the network: its carrier renumbers.
    apply_all(&mut world, vec![road_cmd(far, other)]);
    run_until(&mut world, 400, |w| carried(w, far, other));
    assert_eq!(
        world.stored_ware_cost(P0, far, HQ_FLAG),
        world.route_cost(P0, far, HQ_FLAG)
    );
    assert!(world.stored_ware_cost(P0, HQ_FLAG, other).is_some());
    // A road inside the network getting its carrier: nothing emptied.
    let kept = world.stored_ware_cost(P0, HQ_FLAG, other);
    world.roads[id].as_mut().unwrap().length += 5;
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, other)]);
    run_until(&mut world, 400, |w| carried(w, HQ_FLAG, other));
    assert_ne!(world.route_cost(P0, HQ_FLAG, other), kept);
    assert_eq!(world.stored_ware_cost(P0, HQ_FLAG, other), kept);
    // A road with a carrier removed: emptied.
    let mid = world.roads[id].as_ref().unwrap().path[1];
    apply_all(
        &mut world,
        vec![Command::RemoveRoad {
            player: P0,
            at: mid,
        }],
    );
    assert_eq!(
        world.stored_ware_cost(P0, HQ_FLAG, other),
        world.route_cost(P0, HQ_FLAG, other)
    );
}

// ECO-REQ-0.2: the settlers' part takes the six lists one after the other,
// workers, constructors, carriers, levellers, soldiers, specialists, each in
// the order its settlers were made (their object numbers). The sawmill's
// leveller is made before the road's carrier but is updated after it; the
// builder at work on the site and the leveller on it are members of the
// constructors' and the levellers' lists like the settlers on their way.
// spec: ECO-REQ-0.2, ECO-HQ-4.3, ECO-HQ-4.4
#[test]
fn the_settlers_part_takes_the_six_lists_in_order() {
    use crate::settlers::Member;
    let mut terrain = flat(40, 40);
    let site = (16, 12);
    set_height(&mut terrain, site, 400);
    let mut world = game_with(terrain, &full_stock());
    apply_all(&mut world, vec![place(SAWMILL, site)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 11))]);
    let (mut levelling, mut building) = (false, false);
    while world.site(site).is_some() {
        world.step();
        assert!(world.tick() < 8000, "the sawmill is not finished");
        let order = world.settler_order();
        let keys: Vec<_> = order
            .iter()
            .map(|&(list, number, _)| (list, number))
            .collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]), "{keys:?}");
        for &(list, _, member) in &order {
            let expected = match member {
                Member::Settler(id) => world.settlers[id].as_ref().expect("a settler").list,
                Member::Carrier(_) => SettlerList::Carriers,
                Member::Builder(_) => SettlerList::Constructors,
                Member::Leveller(_) => SettlerList::Levellers,
            };
            assert_eq!(list, expected, "{member:?}");
        }
        let carrier = order.iter().position(|m| matches!(m.2, Member::Carrier(_)));
        if let Some(leveller) = order
            .iter()
            .position(|m| matches!(m.2, Member::Leveller(_)))
        {
            let carrier = carrier.expect("the road's carrier");
            assert!(order[leveller].1 < order[carrier].1, "made first");
            assert!(carrier < leveller, "updated later");
            levelling = true;
        }
        building |= order.iter().any(|m| matches!(m.2, Member::Builder(_)));
    }
    assert!(levelling && building);
}

// ECO-REQ-0.2, FMT-SAV-3.5: a settler stays in the list it was made in while
// it changes jobs. A helper sent to a building as its worker (the well's,
// E-trace-012) is in the workers' list, and still is when the building is
// torn down and he goes home (ECO-REQ-0.2a 1: by the request's kind); a
// helper of no job of its own counts as a carrier (0.2a 2).
// spec: ECO-REQ-0.2, ECO-REQ-0.2a, FMT-SAV-3.5
#[test]
fn a_helper_working_a_building_goes_home_in_the_workers_list() {
    let mut rules = rules(&full_stock());
    let hut = rules
        .building_types
        .iter_mut()
        .find(|t| t.id == HUT)
        .expect("the hut");
    hut.worker = Some(HELPER);
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), rules, &setup);
    let at = (16, 12);
    apply_all(&mut world, vec![road_cmd_after_flag(), place(HUT, at)]);
    apply_all(
        &mut world,
        vec![Command::DebugSupplySite {
            player: P0,
            at,
            wares: vec![(PLANK, 2)],
            workers: true,
        }],
    );
    run_until(&mut world, 2000, |w| w.site(at).is_none());
    connect_hut(&mut world);
    let worker = |w: &World| {
        w.settlers()
            .find(|(_, s)| s.profession == HELPER && s.list == SettlerList::Workers)
            .map(|(_, s)| (s.number, s.job.clone()))
    };
    run_until(&mut world, 2000, |w| {
        worker(w).is_some_and(|(_, job)| matches!(job, SettlerJob::Worker { .. }))
    });
    let (number, _) = worker(&world).expect("the helper at work");
    apply_all(&mut world, vec![Command::RemoveBuilding { player: P0, at }]);
    let home = run_until(&mut world, 100, |w| {
        w.settlers()
            .any(|(_, s)| s.number == number && matches!(s.job, SettlerJob::Home { .. }))
    });
    let (_, settler) = world
        .settlers()
        .find(|(_, s)| s.number == number)
        .expect("the helper going home");
    assert_eq!(settler.list, SettlerList::Workers, "tick {home}");
    assert_eq!(world.list_of(HELPER), SettlerList::Carriers);
    assert_eq!(world.list_of(BUILDER), SettlerList::Constructors);
    assert_eq!(world.list_of(LEVELLER), SettlerList::Levellers);
    assert_eq!(world.list_of(WOODCUTTER), SettlerList::Workers);
}

// ECO-REQ-0.2a (2): a settler made at a warehouse's door without a request
// joins its profession's list: helpers and donkeys the carriers, every other
// profession not named (a boat carrier) the workers. A released boat
// carrier stays the carrier it was made.
// spec: ECO-REQ-0.2a
#[test]
fn a_settler_sent_without_a_request_joins_its_professions_list() {
    const BOAT: u32 = 14;
    const DONKEY: u32 = 15;
    let mut rules = rules(&full_stock());
    rules.roles.boat_carrier = Some(BOAT);
    rules.roles.donkey = Some(DONKEY);
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let world = World::new_game(9, flat(40, 40), rules, &setup);
    assert_eq!(world.door_list_of(BOAT), SettlerList::Workers);
    assert_eq!(world.door_list_of(HELPER), SettlerList::Carriers);
    assert_eq!(world.door_list_of(DONKEY), SettlerList::Carriers);
    assert_eq!(world.door_list_of(BUILDER), SettlerList::Constructors);
    assert_eq!(world.door_list_of(LEVELLER), SettlerList::Levellers);
    assert_eq!(world.door_list_of(WOODCUTTER), SettlerList::Workers);
    assert_eq!(world.list_of(BOAT), SettlerList::Carriers);
}

// TRN-CARRY T59 (TRN-CARRY-3.3, 3.2): the building case of the entry is
// settled only while the building takes the ware, tested every tick of the
// walk. A plank carried to the headquarters' flag for storage; the
// headquarters stops taking planks on the way: the entry is no longer the
// building's, the carrier makes the entry check, puts the plank down at
// the flag, and it keeps its destination with no next flag, its way lost
// (ECO-REQ-5.7 c).
// spec: TRN-CARRY-3.3 (TRN-CARRY T59), TRN-CARRY-3.2, ECO-REQ-5.7 (ECO-REQ T41)
#[test]
fn t59_the_building_takes_a_ware_only_while_it_accepts_the_good() {
    use crate::transport::{CarrierTask, Entry};
    let mut world = game(&full_stock());
    let far = (16, 9);
    apply_all(&mut world, vec![flag_cmd(far)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, far)]);
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    let storage = Target {
        at: HQ,
        serial: world.building_at(HQ).unwrap().serial,
        storage: true,
    };
    let plank = world.new_ware_at(PLANK, far, Some(HQ_FLAG), Some(storage));
    let building = |world: &World| {
        world.carriers.iter().any(|c| {
            c.ware == Some(plank)
                && matches!(
                    c.task,
                    CarrierTask::Carry {
                        entry: Entry::Building,
                        ..
                    }
                )
        })
    };
    while !building(&world) {
        world.step();
        assert!(world.tick() < 600, "carried in");
    }
    world.submit(Command::WarehouseSet {
        player: P0,
        at: HQ,
        good: PLANK,
        stop: true,
        on: true,
    });
    world.step();
    assert!(!building(&world), "no longer settled");
    while world.ware(plank).unwrap().location != WareLocation::Flag(HQ_FLAG) {
        world.step();
        assert!(world.tick() < 900, "put down");
    }
    let w = world.ware(plank).unwrap();
    assert_eq!((w.destination, w.next), (Some(HQ_FLAG), None));
    assert!(w.stranded, "its way lost");
    // ECO-REQ T41: lying at its own destination's flag it has no way at the
    // re-checks either, so a storage run drops the destination.
    for _ in 0..6 {
        world.step();
    }
    assert_eq!(world.ware(plank).unwrap().destination, None);
}

// ECO-REQ T27 (ECO-REQ-5.5): a ware without a destination lying at the
// flag of the warehouse it is stored in goes into the stock at the next
// storage run even when a road carrier has claimed it and walks to fetch
// it; the carrier finds no ware at the flag and is idle again.
// spec: ECO-REQ-5.5 (ECO-REQ T27), ECO-REQ-5.3e
#[test]
fn t27_a_claimed_ware_at_the_warehouses_flag_is_stored() {
    let mut world = fresh_game();
    let hut = (16, 12);
    let claimed = |w: &World| {
        w.wares().find_map(|(id, ware)| {
            let fetching = ware.claimed_by.is_some_and(|c| {
                matches!(w.carriers()[c].task, CarrierTask::Fetch { .. })
                    && w.carriers()[c].walker.at != HQ_FLAG
            });
            (ware.location == WareLocation::Flag(HQ_FLAG) && fetching)
                .then(|| (id, ware.good, ware.claimed_by.expect("claimed")))
        })
    };
    run_until(&mut world, 3000, |w| claimed(w).is_some());
    let (ware, good, carrier) = claimed(&world).expect("a claimed ware");
    // Every ware of that good lying there goes in, claimed or not.
    let lying = world
        .wares()
        .filter(|(_, w)| w.good == good && w.location == WareLocation::Flag(HQ_FLAG))
        .count() as u32;
    let before = count(&world, good);
    let removed = apply_all(
        &mut world,
        vec![Command::RemoveBuilding {
            player: P0,
            at: hut,
        }],
    );
    let stored = match world.wares[ware] {
        None => removed,
        Some(_) => run_until(&mut world, 10, |w| w.wares[ware].is_none()),
    };
    assert_eq!(stored % 5, 0, "a storage run");
    assert_eq!(count(&world, good), before + lying);
    run_until(&mut world, 100, |w| {
        w.carriers()[carrier].task == CarrierTask::Idle
            && w.carriers()[carrier].walker.at == HQ_FLAG
    });
    assert_eq!(world.carriers()[carrier].ware, None);
}

// TRN-FREE T35 (TRN-FREE-6.4 step 1): going home, a settler on a flag takes
// the warehouse with the lowest route cost, the sum of the road lengths L,
// not the one with the fewest fields. The depot's road is the shorter in
// fields but has the higher L (as a steep road): the builder on the middle
// flag walks to the headquarters.
// spec: TRN-FREE-6.4 (TRN-FREE T35)
#[test]
fn t35_going_home_takes_the_lowest_road_length() {
    let mut world = game(&full_stock());
    let depot = (16, 10);
    let middle = (14, 9);
    apply_all(&mut world, vec![place(DEPOT, depot), flag_cmd(middle)]);
    apply_all(
        &mut world,
        vec![road_cmd(HQ_FLAG, middle), road_cmd((16, 9), middle)],
    );
    let depot_road = world
        .roads
        .iter()
        .position(|r| r.as_ref().is_some_and(|r| r.path.contains(&(16, 9))))
        .unwrap();
    let hq_road = world
        .roads
        .iter()
        .position(|r| r.as_ref().is_some_and(|r| r.path.contains(&HQ_FLAG)))
        .unwrap();
    let fields = |id: usize| world.roads[id].as_ref().unwrap().path.len();
    assert!(fields(depot_road) < fields(hq_road));
    let steep = 10 * world.roads[hq_road].as_ref().unwrap().length;
    world.roads[depot_road].as_mut().unwrap().length = steep;
    let mut settler = Settler {
        number: 0,
        player: P0,
        profession: BUILDER,
        list: SettlerList::Constructors,
        walker: crate::movement::Walker::new(middle),
        job: SettlerJob::Dying,
        timer: 0,
        carrying: None,
    };
    world.plan_home_excluding(&mut settler, None);
    let SettlerJob::Home { target } = settler.job else {
        panic!("not going home: {:?}", settler.job);
    };
    assert_eq!(target.at, HQ);
}

// ECO-REQ T28 (ECO-REQ-5.3 (a)): the nearest free ware is measured from the
// ordering building's field, not from its flag. Two free planks lie at flags
// 2 from the shed's flag; the one made first is 3 from the shed's field, the
// other 2: the shed's order takes the second.
// spec: ECO-REQ-5.3 (ECO-REQ T28)
#[test]
fn t28_the_nearest_free_ware_is_measured_from_the_building() {
    let mut world = game(&full_stock());
    let (shed, shed_flag) = ((14, 12), (14, 11));
    apply_all(&mut world, vec![place(SHED, shed)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, shed_flag)]);
    let ring = grid::ring(shed_flag, 2, 40, 40);
    let x = *ring
        .iter()
        .find(|&&f| grid::distance(shed, f) == 3 && f.1 >= 11)
        .expect("a field 3 from the shed");
    let y = *ring
        .iter()
        .find(|&&f| grid::distance(shed, f) == 2 && grid::distance(f, x) >= 2 && f.1 >= 11)
        .expect("a field 2 from the shed");
    apply_all(&mut world, vec![flag_cmd(x), flag_cmd(y)]);
    apply_all(
        &mut world,
        vec![road_cmd(shed_flag, x), road_cmd(shed_flag, y)],
    );
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    let first = world.new_ware_at(PLANK, x, None, None);
    let second = world.new_ware_at(PLANK, y, None, None);
    while !world.tick().is_multiple_of(11) {
        world.step();
    }
    world.step();
    assert_eq!(world.ware(second).unwrap().destination, Some(shed_flag));
    assert_ne!(world.ware(first).unwrap().destination, Some(shed_flag));
}

// ECO-REQ T32, T33 (ECO-REQ-5.4a): of two free planks lying at the same
// flag, so equally near the ordering shed, the one that joined the
// free-ware list first is taken, not the one made first: the later made
// plank was put down without a destination first, the other only after its
// storage trip was dropped.
// spec: ECO-REQ-5.4a (ECO-REQ T32, T33), ECO-REQ-5.3 (a)
#[test]
fn t33_the_free_ware_list_order_decides_a_tie() {
    let mut world = game(&full_stock());
    let (shed, shed_flag) = ((14, 12), (14, 11));
    apply_all(&mut world, vec![place(SHED, shed)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, shed_flag)]);
    let x = *grid::ring(shed_flag, 2, 40, 40)
        .iter()
        .find(|&&f| f.1 >= 11)
        .expect("a field 2 from the shed's flag");
    apply_all(&mut world, vec![flag_cmd(x)]);
    apply_all(&mut world, vec![road_cmd(shed_flag, x)]);
    for road in 0..world.roads.len() {
        world.spawn_carrier(road);
    }
    // Made first, but sent to storage: not on the list.
    let storage = Target {
        at: HQ,
        serial: world.building_at(HQ).unwrap().serial,
        storage: true,
    };
    let first = world.new_ware_at(PLANK, x, Some(HQ_FLAG), Some(storage));
    let second = world.new_ware_at(PLANK, x, None, None);
    assert_eq!(world.ware(first).unwrap().free_place, None);
    // Its storage trip dropped and put down again without a destination.
    let w = world.wares[first].as_mut().unwrap();
    (w.destination, w.target) = (None, None);
    world.join_free_list(first);
    assert!(world.ware(second).unwrap().free_place < world.ware(first).unwrap().free_place);
    while !world.tick().is_multiple_of(11) {
        world.step();
    }
    world.step();
    assert_eq!(world.ware(second).unwrap().destination, Some(shed_flag));
    assert_eq!(world.ware(second).unwrap().free_place, None);
    assert_ne!(world.ware(first).unwrap().destination, Some(shed_flag));
}

// ECO-REQ T31, T32 (ECO-REQ-5.4a, 5.7 (a)): the pre-pass drops the wares
// that lost their way last collected first, so they join the free-ware list
// in the reverse order of their orders.
// spec: ECO-REQ-5.4a (ECO-REQ T31, T32), ECO-REQ-5.7
#[test]
fn t31_lost_wares_join_the_list_in_the_reverse_order_of_their_orders() {
    let mut world = game(&full_stock());
    let (shed, shed_flag) = ((14, 12), (14, 11));
    apply_all(&mut world, vec![place(SHED, shed)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, shed_flag)]);
    let target = Target {
        at: shed,
        serial: world.building_at(shed).unwrap().serial,
        storage: false,
    };
    let order = |made| Order {
        player: P0,
        good: PLANK,
        target,
        stamp: 0,
        made,
        harbour: None,
    };
    // Lower ware number, later order.
    let a = world.new_ware_at(PLANK, HQ_FLAG, Some(shed_flag), Some(target));
    let b = world.new_ware_at(PLANK, HQ_FLAG, Some(shed_flag), Some(target));
    for (id, made) in [(a, 7), (b, 3)] {
        let w = world.wares[id].as_mut().unwrap();
        w.order = Some(order(made));
        w.stranded = true;
    }
    world.drop_stranded_orders();
    let (pa, pb) = (
        world.ware(a).unwrap().free_place.unwrap(),
        world.ware(b).unwrap().free_place.unwrap(),
    );
    assert!(pa < pb, "the ware of the later order joins first");
    assert_eq!(world.ware(a).unwrap().destination, None);
}

// ECO-DIST T22 (ECO-DIST-1.6): the re-check tests the next flag against the
// destination's flag's ware network, not the ware's own flag's. A plank at
// the headquarters' flag for the flag y keeps its next flag x while the
// road x - y has a carrier; without one, y is in another network and the
// next flag is no longer valid, though x is still joined to its own flag.
// spec: ECO-DIST-1.6 (ECO-DIST T22)
#[test]
fn t22_the_recheck_tests_the_destinations_network() {
    let mut world = game(&full_stock());
    let (x, y) = ((13, 9), (16, 9));
    apply_all(&mut world, vec![flag_cmd(x), flag_cmd(y)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, x), road_cmd(x, y)]);
    let road_to = |world: &World, a: Point, b: Point| {
        world
            .roads
            .iter()
            .position(|r| {
                r.as_ref()
                    .is_some_and(|r| r.path.contains(&a) && r.path.contains(&b))
            })
            .unwrap()
    };
    let first = road_to(&world, HQ_FLAG, x);
    world.spawn_carrier(first);
    let id = world.new_ware_at(PLANK, HQ_FLAG, Some(y), None);
    world.wares[id].as_mut().unwrap().next = Some(x);
    assert!(
        !world.next_still_valid(id),
        "y is beyond a road without a carrier"
    );
    let second = road_to(&world, x, y);
    world.spawn_carrier(second);
    assert!(world.next_still_valid(id));
}

// TRN-FREE T36 (TRN-FREE-6.4 step 1, off a flag): a settler on a road's
// inner field counts each warehouse's route from the first flag his road
// walk reaches plus that first leg. On the field next to the depot's flag
// of the road between the two warehouses' flags, both routes cost nothing
// past their first flag; the shorter first leg takes him to the depot.
// spec: TRN-FREE-6.4 (TRN-FREE T36)
#[test]
fn t36_going_home_off_a_flag_counts_the_first_leg() {
    let mut world = game(&full_stock());
    let depot = (16, 10);
    apply_all(&mut world, vec![place(DEPOT, depot)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, (16, 9))]);
    let path = world
        .roads
        .iter()
        .flatten()
        .find(|r| r.path.contains(&HQ_FLAG))
        .unwrap()
        .path
        .clone();
    let near = path[path.len() - 2];
    let mut settler = Settler {
        number: 0,
        player: P0,
        profession: BUILDER,
        list: SettlerList::Constructors,
        walker: crate::movement::Walker::new(near),
        job: SettlerJob::Dying,
        timer: 0,
        carrying: None,
    };
    world.plan_home_excluding(&mut settler, None);
    let SettlerJob::Home { target } = settler.job else {
        panic!("not going home: {:?}", settler.job);
    };
    assert_eq!(target.at, depot);
}

// Hook W16 (docs/extending.md "A group good in a recipe"): a hut whose
// recipe takes 2 of the group TOOLS (members AXE, HAMMER) orders the group;
// the headquarters holds 3 axes and 1 hammer and the player keeps 2 of each
// back, so exactly one axe is handed out (3 > 2), then none (2 axes and 1
// hammer are kept); the axe arrives and counts as the group in the hut.
// The original's rules have no groups (the hashes stay).
// spec: none (hook W16; the serving is ECO-REQ-5.3's, the orders ECO-PROD-6.1's)
#[test]
fn a_recipe_group_is_served_by_a_member_above_the_kept_stock() {
    const TOOLS: u32 = 50;
    let stock: Vec<(u32, u32)> = full_stock()
        .into_iter()
        .map(|(g, n)| if g == HAMMER { (g, 1) } else { (g, n) })
        .collect();
    let mut rules = rules(&stock);
    let hut = rules
        .building_types
        .iter_mut()
        .find(|t| t.id == HUT)
        .unwrap();
    hut.stock_limit = Some(4);
    hut.product = Some(PLANK);
    hut.cycle = crate::production::WorkCycle::Workshop;
    rules.recipe_sets.insert(
        HUT,
        vec![crate::Recipe {
            product: PLANK,
            inputs: vec![(TOOLS, 2)],
        }],
    );
    rules.recipe_groups.insert(TOOLS, vec![AXE, HAMMER]);
    rules.recipe_keep.insert(AXE, 2);
    rules.recipe_keep.insert(HAMMER, 2);
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), rules, &setup);
    let hut = (16, 12);
    apply_all(&mut world, vec![road_cmd_after_flag(), place(HUT, hut)]);
    apply_all(
        &mut world,
        vec![Command::DebugSupplySite {
            player: P0,
            at: hut,
            wares: vec![(PLANK, 2)],
            workers: true,
        }],
    );
    run_until(&mut world, 2000, |w| w.site(hut).is_none());
    connect_hut(&mut world);
    world.orders.retain(|o| o.target.at != hut);
    world.file_input_orders(hut);
    assert_eq!(world.open_orders(hut, TOOLS), 4);
    let axes = count(&world, AXE);
    assert_eq!((axes, count(&world, HAMMER)), (3, 1));
    run_until(&mut world, 3000, |w| {
        w.inputs(hut).iter().any(|&(g, n)| g == TOOLS && n == 1)
    });
    for _ in 0..600 {
        world.step();
    }
    assert_eq!(count(&world, AXE), 2);
    assert_eq!(count(&world, HAMMER), 1);
    assert_eq!(world.inputs(hut), &[(TOOLS, 1)]);
    assert_eq!(world.open_orders(hut, TOOLS), 3);
    assert!(!world
        .wares()
        .any(|(_, w)| w.good == AXE || w.good == HAMMER));
    assert!(format!("{world:?}").contains("recipe_groups"));
    assert!(!format!("{:?}", game(&full_stock())).contains("recipe_groups"));
}

// Hook W19 (docs/extending.md "Paved roads"): with the rule (stone, 1 per
// field, 130 %) a road A-B of the headquarters' flag is ordered paved: as
// many stone as the road has fields leave the headquarters' stock, are
// carried to A and laid there; then the road is paved and its carrier walks
// its steps at 1.3 times the speed (the rate of a step between two of its
// fields), every other road as before. A waterway, a paved road and a road
// without the rule are refused; the snapshot keeps the pavement. The
// original's rules have no paving (the hashes stay).
// spec: none (hook W19; the transport is TRN-CARRY's, the step rate TRN-MOVE-1.2's)
#[test]
fn a_paved_road_costs_its_stone_and_is_walked_faster() {
    let mut rules = rules(&full_stock());
    rules.paving = Some(crate::PavingRule {
        good: STONE,
        per_field: 1,
        speed_percent: 130,
    });
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), rules, &setup);
    // HQ's flag - B, and the road to pave B - C (its stone walks to B).
    let (b, c) = ((13, 9), (16, 9));
    apply_all(&mut world, vec![flag_cmd(b), flag_cmd(c)]);
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, b), road_cmd(b, c)]);
    let road = world.roads().find(|r| r.path.contains(&c)).unwrap().clone();
    assert_eq!(road.path[0], b);
    let inner = road.path[1];
    let fields = road.path.len() as u32;
    let stone = count(&world, STONE);
    let pave = Command::PaveRoad {
        player: P0,
        at: inner,
    };
    assert!(world.can_pave(P0, inner));
    assert_eq!(apply(&mut world, pave.clone()), Ok(()));
    // The first ware leaves in the command's own tick (the roads' part).
    assert_eq!(
        world.paving_work(road.number),
        Some(crate::PavingWork {
            needed: fields,
            sent: 1,
            laid: 0
        })
    );
    assert_eq!(
        apply(&mut world, pave.clone()),
        Err(crate::Rejection::NotPavable)
    );
    // A stone lost on its way is sent again.
    run_until(&mut world, 400, |w| {
        w.wares()
            .any(|(_, x)| x.good == STONE && x.destination == Some(b))
    });
    let ware = world
        .wares()
        .find(|(_, x)| x.good == STONE && x.destination == Some(b))
        .map(|(id, _)| id)
        .unwrap();
    world.destroy_ware(ware);
    run_until(&mut world, 5000, |w| w.is_paved(road.number));
    assert_eq!(count(&world, STONE), stone - fields - 1);
    assert_eq!(world.paving_work(road.number), None);
    assert!(!world.can_pave(P0, inner));
    // The carrier's next step between two of the road's fields: 1.3 times.
    let id = world
        .roads()
        .position(|r| r.number == road.number)
        .expect("the road");
    let base = world.settings.settler_speed;
    let (a, next) = (road.path[1], road.path[2]);
    assert_eq!(world.paved_speed(id, a, Some(next), base), base * 130 / 100);
    assert_eq!(world.paved_speed(id, a, Some((30, 30)), base), base);
    let copy = World::from_snapshot(&world.snapshot()).unwrap();
    assert!(copy == world && copy.is_paved(road.number));
    assert!(format!("{world:?}").contains("paved"));
    assert!(!format!("{:?}", game(&full_stock())).contains("paving"));
    // Without the rule nothing can be paved.
    let mut plain = game(&full_stock());
    apply_all(&mut plain, vec![flag_cmd(b)]);
    apply_all(&mut plain, vec![road_cmd(HQ_FLAG, b)]);
    let inner = plain.roads().next().unwrap().path[1];
    let pave = Command::PaveRoad {
        player: P0,
        at: inner,
    };
    assert_eq!(apply(&mut plain, pave), Err(crate::Rejection::NotPavable));
}
