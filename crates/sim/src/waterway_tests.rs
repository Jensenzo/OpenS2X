//! Waterways (`specs/TRN-roads-flags.md` TRN-ROAD-7) and their boat
//! carriers (ECO-REQ-1.1, TRN-FREE-6.1).

use crate::grid::{self, Point};
use crate::map::{state, Terrain};
use crate::requests::tests::{self as req, apply_all, road_cmd, HQ, HQ_FLAG};
use crate::requests::Requester;
use crate::test_support::*;
use crate::{Command, GameSetup, Rejection, World};

const BOAT_CARRIER: u32 = 30;
/// The coast flags west and east of the lake.
const WEST: Point = (12, 9);
const EAST: Point = (16, 9);

/// Flat land with a lake over x 13 to 15, y 4 to 14; the land fields next
/// to water are Coast.
fn lake() -> Terrain {
    let mut terrain = flat(40, 40);
    let water = |f: Point| (13..=15).contains(&f.0) && (4..=14).contains(&f.1);
    for f in terrain.fields() {
        let i = terrain.index(f);
        if water(f) {
            terrain.states[i] = state::WATER;
        }
    }
    for f in terrain.fields() {
        let i = terrain.index(f);
        if !water(f) && terrain.neighbours(f).into_iter().flatten().any(water) {
            terrain.states[i] |= state::COAST;
        }
    }
    terrain
}

fn game() -> World {
    let mut stock = req::full_stock();
    stock.push((BOAT_CARRIER, 2));
    let mut rules = req::rules(&stock);
    rules.roles.boat_carrier = Some(BOAT_CARRIER);
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, lake(), rules, &setup);
    apply_all(
        &mut world,
        vec![
            Command::PlaceFlag {
                player: P0,
                at: WEST,
            },
            Command::PlaceFlag {
                player: P0,
                at: EAST,
            },
        ],
    );
    world
}

fn waterway(world: &mut World, from: Point, to: Point) -> Result<(), Rejection> {
    world.submit(Command::BuildWaterway {
        player: P0,
        from,
        to,
    });
    world.step();
    world.last_results()[0].outcome
}

// TRN-ROAD-7.2, 7.3: over water fields only, cost 1 a step and 2 onto a
// Coast field: across the 3-field lake 1 + 1 + 1 + 2.
#[test]
fn waterway_crosses_the_lake() {
    let mut world = game();
    let (path, cost) = world.find_waterway(WEST, EAST).unwrap();
    assert_eq!((path[0], *path.last().unwrap()), (WEST, EAST));
    let inner = &path[1..path.len() - 1];
    assert!(inner.iter().all(|&f| world.terrain.has(f, state::WATER)));
    assert_eq!(path.len() - 1, 4, "{path:?}");
    assert_eq!(cost, 5);
    assert_eq!(waterway(&mut world, WEST, EAST), Ok(()));
    let road = world.roads().last().unwrap();
    assert!(road.water);
    assert!(inner.iter().all(|&f| world.terrain.has(f, state::STREET)));
}

// TRN-ROAD-7.1, 7.2: it starts at a Coast flag and never crosses land.
#[test]
fn waterway_needs_the_coast_and_water() {
    let mut world = game();
    let inland = (10, 13);
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: inland,
        }],
    );
    assert_eq!(waterway(&mut world, inland, EAST), Err(Rejection::NotCoast));
    // Along the coast from WEST to another coast field: land between.
    let south = (12, 13);
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: south,
        }],
    );
    let (path, _) = world.find_waterway(WEST, south).unwrap();
    assert!(path[1..path.len() - 1]
        .iter()
        .all(|&f| world.terrain.has(f, state::WATER)));
    // A waterway does not cross another one.
    assert_eq!(waterway(&mut world, WEST, EAST), Ok(()));
    let north = (14, 3);
    let far_south = (14, 15);
    apply_all(
        &mut world,
        vec![
            Command::PlaceFlag {
                player: P0,
                at: north,
            },
            Command::PlaceFlag {
                player: P0,
                at: far_south,
            },
        ],
    );
    let (path, _) = world.find_waterway(north, far_south).unwrap_or_default();
    let crossing = world.roads().last().unwrap().path.clone();
    assert!(
        path.iter()
            .all(|f| !crossing[1..crossing.len() - 1].contains(f)),
        "{path:?}"
    );
}

// ECO-REQ-1.1, TRN-CARRY-6.2: a waterway requests a boat carrier, never a
// helper or a donkey; TRN-FREE-6.1: settlers walk land roads only, wares
// go over the waterway too, once it has its boat carrier (TRN-CARRY-5.1a).
// spec: TRN-CARRY-5.1a, TRN-CARRY-3.4
#[test]
fn boat_carrier_and_routes() {
    let mut world = game();
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, WEST)]);
    assert_eq!(waterway(&mut world, WEST, EAST), Ok(()));
    let id = world.roads().count() - 1;
    let requests: Vec<_> = world
        .requests(P0)
        .iter()
        .filter(|r| r.requester == Requester::Road(id))
        .map(|r| r.profession)
        .collect();
    assert_eq!(requests, [BOAT_CARRIER]);
    // No carrier assigned yet: the waterway is in no ware network.
    assert_eq!(world.next_flag(P0, WEST, EAST), None);
    assert_eq!(world.road_walk(P0, HQ_FLAG, EAST), None);
    // The boat carrier comes from the headquarters and a ware crosses.
    for _ in 0..200 {
        world.step();
    }
    assert_eq!(world.warehouse(HQ).unwrap().count(BOAT_CARRIER), 1);
    assert_eq!(world.next_flag(P0, HQ_FLAG, EAST), Some(WEST));
    assert_eq!(world.next_flag(P0, WEST, EAST), Some(EAST));
    world.submit(Command::DropWare {
        player: P0,
        good: req::PLANK,
        at: HQ_FLAG,
        destination: EAST,
    });
    // The plank is delivered (it leaves the world at its flag) after the
    // boat carrier, in the carrier's place, took it across.
    let mut delivered = false;
    for _ in 0..600 {
        world.step();
        if world.wares().next().is_none() {
            delivered = true;
            break;
        }
    }
    assert!(delivered, "the plank crossed the lake");
    // TRN-CARRY-3.4 (T50): it counts on the waterway when the put-down is
    // over.
    while world
        .carriers()
        .iter()
        .any(|c| matches!(c.task, crate::CarrierTask::PutDown { .. }))
    {
        world.step();
    }
    let water = world.road(id).unwrap();
    assert!(water.carrier.is_some() && water.donkey.is_none());
    assert_eq!(water.traffic.transport_count, 1);
    assert!(grid::distance(WEST, EAST) == 4);
}

// TRN-ROAD T16, 7.4: a route of 4 points and a target 3 fields from its
// end: 3 + 4 - 1 = 6 is allowed, 4 fields away is not; a route of more
// than one point must end on water; a waterway from its flag reaches at
// most 6 fields.
#[test]
fn t16_a_waterway_is_at_most_six_steps() {
    let world = game();
    let end = (14, 9);
    assert!(world.waterway_end_allowed(end, 4, (14, 12)));
    assert!(!world.waterway_end_allowed(end, 4, (14, 13)));
    assert!(
        !world.waterway_end_allowed(WEST, 4, (14, 12)),
        "the end is land"
    );
    assert!(world.waterway_end_allowed(WEST, 1, (14, 4)));
    assert_eq!(
        world.find_waterway((14, 3), (14, 15)),
        Err(Rejection::WaterwayTooLong)
    );
}

/// T63's world: water over x 13 to 17 on every row, so the land east of it
/// (island C) has no land road to the headquarters' island A, with a rock
/// at (16, 9) (rock B); the land fields next to water are Coast.
fn strait() -> World {
    let mut world = game();
    let water = |f: Point| (13..=17).contains(&f.0) && f != ROCK;
    for f in world.terrain.fields().collect::<Vec<_>>() {
        let i = world.terrain.index(f);
        world.terrain.states[i] = if water(f) {
            state::WATER
        } else {
            world.terrain.states[i] & !(state::WATER | state::COAST)
        };
    }
    for f in world.terrain.fields().collect::<Vec<_>>() {
        let i = world.terrain.index(f);
        if !water(f) && world.terrain.neighbours(f).into_iter().flatten().any(water) {
            world.terrain.states[i] |= state::COAST;
        }
        // Island C is player 0's too, as far as x 26 (no border inside).
        if (13..=26).contains(&f.0) && (2..=18).contains(&f.1) {
            world.owners[i] = Some(P0);
            world.terrain.states[i] &= !state::BORDER;
        }
    }
    world
}

/// Rock B: [`EAST`]'s flag, which [`game`] has placed.
const ROCK: Point = EAST;
const ISLAND_C: Point = (18, 9);

// TRN-CARRY T63 (TRN-CARRY-5.1, Q-198 a): waterway R1 from the coast flag
// F1 of island A (joined to the headquarters by a land road) to a flag F2 on
// rock B, waterway R2 from F2 to a flag F3 on island C, a hut site on C at
// F3. R1's boat carrier comes from the headquarters; R2's walks from the
// headquarters over R1 to F2 and onto R2 (the all-roads network); a plank
// goes F1 -> R1 -> F2 -> R2 -> F3 (the ware network); the site's builder is
// never sent (settlers walk land roads only).
// spec: TRN-CARRY-5.1 (TRN-CARRY T63), TRN-FREE-6.1
#[test]
fn t63_boat_carriers_and_wares_over_two_waterways() {
    let mut world = strait();
    apply_all(
        &mut world,
        vec![
            road_cmd(HQ_FLAG, WEST),
            Command::PlaceFlag {
                player: P0,
                at: ISLAND_C,
            },
        ],
    );
    assert_eq!(world.flag_at(ROCK), Some(P0));
    assert_eq!(world.flag_at(ISLAND_C), Some(P0));
    assert_eq!(waterway(&mut world, WEST, ROCK), Ok(()));
    assert_eq!(waterway(&mut world, ROCK, ISLAND_C), Ok(()));
    let (r1, r2) = (world.roads().count() - 2, world.roads().count() - 1);
    // The hut's site on island C, its flag at F3.
    let hut = world.terrain.neighbour(ISLAND_C, 4).expect("on the map");
    assert_eq!(
        apply(
            &mut world,
            Command::PlaceBuilding {
                player: P0,
                building: req::HUT,
                at: hut,
            }
        ),
        Ok(())
    );
    let mut both = None;
    for t in 0..3000 {
        world.step();
        let has = |r: usize| world.road(r).unwrap().carrier.is_some();
        if both.is_none() && has(r1) && has(r2) {
            both = Some(t);
        }
        if world
            .carriers()
            .iter()
            .any(|c| c.road == r2 && c.walker.at == ISLAND_C)
        {
            break;
        }
    }
    assert!(
        both.is_some(),
        "both waterways have their boat carrier: {:?} {:?}, stock {}, carriers {:?}",
        world.road(r1).unwrap().carrier,
        world.road(r2).unwrap().carrier,
        world.warehouse(HQ).unwrap().count(BOAT_CARRIER),
        world
            .carriers()
            .iter()
            .map(|c| (c.road, c.walker.at, c.task))
            .collect::<Vec<_>>()
    );
    assert_eq!(world.warehouse(HQ).unwrap().count(BOAT_CARRIER), 0);
    assert_eq!(world.next_flag(P0, HQ_FLAG, ISLAND_C), Some(WEST));
    assert_eq!(world.next_flag(P0, WEST, ISLAND_C), Some(ROCK));
    assert_eq!(world.next_flag(P0, ROCK, ISLAND_C), Some(ISLAND_C));
    // A plank crosses both.
    world.submit(Command::DropWare {
        player: P0,
        good: req::PLANK,
        at: HQ_FLAG,
        destination: ISLAND_C,
    });
    let mut passed = Vec::new();
    for _ in 0..3000 {
        world.step();
        for (_, w) in world.wares() {
            if let crate::WareLocation::Flag(f) = w.location {
                if passed.last() != Some(&f) {
                    passed.push(f);
                }
            }
        }
        if world.wares().next().is_none() {
            break;
        }
    }
    assert!(
        world.wares().next().is_none(),
        "the plank arrived: {passed:?} {:?} {:?}",
        world.wares().map(|(_, w)| w.location).collect::<Vec<_>>(),
        world
            .carriers()
            .iter()
            .map(|c| (c.road, c.walker.at, c.task))
            .collect::<Vec<_>>()
    );
    assert_eq!(passed, [HQ_FLAG, WEST, ROCK], "{passed:?}");
    // The site's builder: no land road joins the headquarters to C.
    assert!(!world.settlers().any(|(_, s)| s.profession == req::BUILDER));
}
