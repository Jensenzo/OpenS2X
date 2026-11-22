//! Tests of a game continued from a save (`specs/FMT-savegame.md`
//! FMT-SAV-3.4a, 3.9): the workers and the road carriers the save names.

use crate::requests::tests::{
    self as req, CASTLE, HELPER, HQ, HQ_FLAG, HUT, PLANK, STONE, WOODCUTTER,
};
use crate::test_support::*;
use crate::transport::{idle_field, CarrierTask};
use crate::{
    GameSetup, Restore, RestoreReport, RestoredBuilding, RestoredRoad, RestoredShip, RestoredWare,
    World,
};

/// The hut south of the headquarters' row, its flag, and the road between
/// the two flags along row 9.
const HUT_AT: (u32, u32) = (16, 10);
const HUT_FLAG: (u32, u32) = (16, 9);

/// A world of the test rules with no headquarters placed, restored with a
/// headquarters, a finished hut and the road from the headquarters' flag
/// to the hut's; `named` says whether the save names the hut's worker and
/// the road's carrier.
fn restored(named: bool) -> (World, RestoreReport) {
    restored_with(named, |_| {})
}

/// [`restored`] with `more` added to the save first.
fn restored_with(named: bool, more: impl FnOnce(&mut Restore)) -> (World, RestoreReport) {
    restored_in(|_| {}, named, more)
}

/// [`restored_with`] in a world `prepare` changed first.
fn restored_in(
    prepare: impl FnOnce(&mut World),
    named: bool,
    more: impl FnOnce(&mut Restore),
) -> (World, RestoreReport) {
    let setup = GameSetup {
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), req::rules(&req::full_stock()), &setup);
    prepare(&mut world);
    let building = |building_type, at, stock: Vec<(u32, u32)>, worker| RestoredBuilding {
        player: P0,
        building_type,
        at,
        remaining: None,
        levelled: 1.0,
        delivered: Vec::new(),
        production_on: true,
        stock,
        worker,
        number: None,
        worker_number: None,
        soldiers: Vec::new(),
    };
    let fields = world.owners.len();
    let saved = Restore {
        players: vec![P0],
        tick: 1000,
        owners: vec![Some(P0); fields],
        explored: vec![1; fields],
        buildings: vec![
            building(CASTLE, HQ, req::full_stock(), false),
            building(HUT, HUT_AT, Vec::new(), named),
        ],
        flags: vec![(P0, HQ_FLAG), (P0, HUT_FLAG)],
        roads: vec![RestoredRoad {
            player: P0,
            path: (10..=16).map(|x| (x, 9)).collect(),
            carrier: named,
            donkey: false,
            numbers: [named.then_some(4321), None],
            standing: [None, None],
        }],
        ..Default::default()
    };
    let mut saved = saved;
    more(&mut saved);
    let report = world.restore(&saved);
    (world, report)
}

// FMT-SAV-3.6, 3.9: the wares a flag's package list names lie at that flag
// again, in its order, without a destination until ECO-REQ-5 gives them one;
// one at a flag the world does not have, or not the ware's owner's, is left
// out. FMT-SAV-3.2, SIM-RNG-2.3: the generator goes on from the saved
// state and seed. FMT-SAV-3.11: the statistics go on from the saved series,
// the running totals from their last samples.
// spec: FMT-SAV-3.6, FMT-SAV-3.9, FMT-SAV-3.2, SIM-RNG-2.3, FMT-SAV-3.11, UI-GAME-8.5a
#[test]
fn wares_at_flags_the_generator_and_the_statistics_come_back() {
    let mut series: Vec<Vec<u32>> = (0..22).map(|i| vec![i, i + 1]).collect();
    series[5] = vec![3, 7];
    series[7] = vec![0, 2];
    let (world, report) = restored_with(false, |saved| {
        saved.wares = vec![
            lying(P0, HUT_FLAG, PLANK),
            lying(P0, HUT_FLAG, STONE),
            lying(P0, (30, 30), PLANK),
            lying(crate::PlayerId(1), HQ_FLAG, PLANK),
        ];
        saved.random = Some([0x1aa6_d677, 0x5432_ace0]);
        saved.statistics = vec![(P0, series.clone())];
    });
    assert_eq!((report.wares, report.wares_skipped), (2, 2));
    let at_flag: Vec<_> = world
        .wares_at(HUT_FLAG)
        .iter()
        .map(|&w| world.ware(w).expect("ware").good)
        .collect();
    assert_eq!(at_flag, [PLANK, STONE]);
    assert!(world.wares_at(HUT_FLAG).iter().all(|&w| world
        .ware(w)
        .expect("ware")
        .destination
        .is_none()));
    assert_eq!(world.rng().state(), 0x1aa6_d677);
    assert_eq!(world.rng().seed(), 0x5432_ace0);
    let stats = world.statistics(P0).expect("statistics");
    assert_eq!(stats.series[0], [0, 1]);
    assert_eq!(stats.production[13], [21, 22]);
    assert_eq!((stats.coins, stats.frags), (7, 2));
    // FMT-SAV-3.11: the saved totals, where the save has them, over the
    // last samples (a kill after the last sample).
    let (world, _) = restored_with(false, |saved| {
        saved.statistics = vec![(P0, series.clone())];
        saved.totals = vec![(P0, [9, 3])];
    });
    let stats = world.statistics(P0).expect("statistics");
    assert_eq!((stats.coins, stats.frags), (9, 3));
}

// FMT-SAV-3.5a (c), TRN-FREE-6.4 step 4 (Q-1032 c answered): a saved
// settler with no warehouse in reach goes on wandering with its saved
// failure count: 20 failures behind it, its step after this failed search
// counts 20, the search after it 21.
// spec: FMT-SAV-3.5a, TRN-FREE-6.4
#[test]
fn a_homeless_settler_wanders_on_with_its_saved_count() {
    let (mut world, report) = restored_with(true, |saved| {
        saved.homeward = vec![crate::RestoredSettler {
            player: P0,
            profession: HELPER,
            field: (36, 36),
            number: 778,
            failures: 20,
        }];
    });
    assert_eq!(report.homeward, 1);
    let job = |w: &World| {
        w.settlers()
            .find(|(_, s)| s.number == 778)
            .map(|(_, s)| s.job.clone())
            .expect("the homeless settler")
    };
    assert!(
        matches!(
            job(&world),
            crate::SettlerJob::Leaving {
                step: crate::settlers::LeavingStep::Wander { tries: 20 },
                ..
            }
        ),
        "{:?}",
        job(&world)
    );
    let mut retried = None;
    for _ in 0..40 {
        world.step();
        if let crate::SettlerJob::Leaving {
            step: crate::settlers::LeavingStep::Retry { tries },
            ..
        } = job(&world)
        {
            retried = Some(tries);
            break;
        }
    }
    assert_eq!(retried, Some(21));
}

// FMT-SAV-3.5a (T10): a package in the hands of a building's named worker
// is his product again: he holds the good inside and waits for its place at
// the flag (ECO-WORK-3.4), then carries it out. (c, T11): a settler no list
// names goes home from its field.
// spec: FMT-SAV-3.5a
#[test]
fn a_workers_product_and_the_settlers_going_home_come_back() {
    let (mut world, report) = restored_with(true, |saved| {
        saved.products = vec![(HUT_AT, PLANK)];
        saved.homeward = vec![crate::RestoredSettler {
            player: P0,
            profession: HELPER,
            field: (14, 9),
            number: 777,
            failures: 0,
        }];
    });
    assert_eq!((report.products, report.homeward), (1, 1));
    let worker = world
        .settlers()
        .find(|(_, s)| matches!(s.job, crate::SettlerJob::Worker { target, .. } if target.at == HUT_AT))
        .map(|(_, s)| (s.carrying, s.job.clone()))
        .expect("the hut's worker");
    assert_eq!(worker.0, Some(PLANK));
    assert!(matches!(
        worker.1,
        crate::SettlerJob::Worker {
            work: crate::Work::WaitRoom { .. },
            ..
        }
    ));
    let home = world
        .settlers()
        .find(|(_, s)| s.number == 777)
        .map(|(_, s)| s.job.clone())
        .expect("the settler going home");
    assert!(matches!(home, crate::SettlerJob::Home { .. }), "{home:?}");
    // The plank reaches the hut's flag.
    for _ in 0..200 {
        world.step();
    }
    let at_flag = world
        .wares_at(HUT_FLAG)
        .iter()
        .chain(world.wares_at(HQ_FLAG))
        .any(|&w| world.ware(w).is_some_and(|w| w.good == PLANK));
    let carried = world.wares().any(|(_, w)| w.good == PLANK);
    assert!(at_flag || carried, "the product went out");
}

// FMT-SAV-3.5a (e): a constructor his site names is its builder at once (as
// one who has arrived), so the site asks for no other.
// spec: FMT-SAV-3.5a
#[test]
fn a_sites_constructor_is_at_the_site() {
    const SITE: (u32, u32) = (22, 12);
    let (mut world, report) = restored_with(false, |saved| {
        let east_south = crate::grid::Direction::new(1).expect("a direction");
        let flag = crate::grid::neighbour(SITE, east_south, 40, 40).expect("its flag");
        saved.flags.push((P0, flag));
        saved.buildings.push(RestoredBuilding {
            player: P0,
            building_type: HUT,
            at: SITE,
            remaining: Some([1.0, 1.0]),
            levelled: 1.0,
            delivered: Vec::new(),
            soldiers: Vec::new(),
            production_on: true,
            stock: Vec::new(),
            worker: false,
            number: None,
            worker_number: None,
        });
        saved.builders = vec![(SITE, 4242)];
    });
    assert_eq!((report.sites, report.builders), (1, 1));
    let builder = |w: &World| {
        w.site(SITE)
            .and_then(|s| s.builder.as_ref())
            .map(|b| b.number)
    };
    assert_eq!(builder(&world), Some(4242));
    world.step();
    assert_eq!(builder(&world), Some(4242), "no other was asked for");
}

// FMT-SAV-3.5a (b): a named worker on his way (state 4) walks on over the
// roads from his field to the building's flag and in, then works there.
// spec: FMT-SAV-3.5a
#[test]
fn a_named_worker_on_his_way_walks_on() {
    let (mut world, report) = restored_with(true, |saved| {
        saved.workers_on_way = vec![(HUT_AT, (12, 9), Some((13, 9)))];
    });
    assert_eq!((report.workers, report.workers_on_way), (1, 1));
    let job = |w: &World| {
        w.settlers()
            .find(|(_, s)| {
                matches!(s.job, crate::SettlerJob::Worker { target, .. }
                    | crate::SettlerJob::ToBuilding { target } if target.at == HUT_AT)
            })
            .map(|(_, s)| {
                (
                    s.walker.at,
                    matches!(s.job, crate::SettlerJob::Worker { .. }),
                )
            })
    };
    assert_eq!(job(&world), Some(((12, 9), false)));
    for _ in 0..300 {
        world.step();
    }
    assert_eq!(job(&world).map(|j| j.1), Some(true), "at work in his hut");
}

// FMT-SAV-3.5a (b, 2026-11-09, Q-257): a named worker producing inside
// (state 33 on his building's field, a work loop with 40 ticks left) goes on
// with his work action and ends it after those 40 ticks (ECO-WORK-3.2), then
// takes his product as at any action's end; one in a building whose worker
// does not work inside, or not in the save's list, is only seated.
// spec: FMT-SAV-3.5a, ECO-WORK-3.2
#[test]
fn a_worker_producing_inside_ends_his_action_after_the_saved_ticks() {
    let inside = |w: &mut World| {
        let kind = w.building_types.get_mut(&HUT).expect("the hut");
        kind.cycle = crate::WorkCycle::Workshop;
        kind.production_ticks = 333;
    };
    let (mut world, report) = restored_in(inside, true, |saved| {
        saved.workers_working = vec![(HUT_AT, 40)];
    });
    assert_eq!(report.workers_working, 1);
    let work = |w: &World| {
        w.settlers()
            .find_map(|(_, s)| match s.job {
                crate::SettlerJob::Worker { target, work } if target.at == HUT_AT => {
                    Some((work, s.timer))
                }
                _ => None,
            })
            .expect("the hut's worker")
    };
    assert_eq!(work(&world), (crate::Work::Working, 40));
    let mut ended = None;
    for k in 1..=60u32 {
        world.step();
        if work(&world).0 != crate::Work::Working && ended.is_none() {
            ended = Some(k);
        }
    }
    assert_eq!(ended, Some(40), "the action ends after the 40 ticks left");
    // The hut of the test rules works outside (no cycle): only seated.
    let (world, report) = restored_with(true, |saved| {
        saved.workers_working = vec![(HUT_AT, 40)];
    });
    assert_eq!(report.workers_working, 0);
    assert_eq!(work(&world), (crate::Work::Begin, 0));
}

// FMT-SAV-3.5a (b, 2026-11-08 b): a named worker on the land coming back
// (job state 36 with his product, the common ref) walks from his saved field
// to his building's flag carrying it (ECO-OUT-3.8), puts it down there and
// goes in.
// spec: FMT-SAV-3.5a, ECO-OUT-3.8
#[test]
fn a_worker_coming_back_with_his_product_walks_to_his_flag() {
    let field = (19, 8);
    let (mut world, report) = restored_with(true, |saved| {
        saved.products = vec![(HUT_AT, PLANK)];
        saved.workers_back = vec![(HUT_AT, field)];
    });
    assert_eq!((report.products, report.workers_back), (1, 1));
    let worker = |w: &World| {
        w.settlers()
            .find(|(_, s)| matches!(s.job, crate::SettlerJob::Worker { target, .. } if target.at == HUT_AT))
            .map(|(_, s)| s.clone())
            .expect("the hut's worker")
    };
    let s = worker(&world);
    assert_eq!((s.walker.at, s.carrying), (field, Some(PLANK)));
    assert!(matches!(
        s.job,
        crate::SettlerJob::Worker {
            work: crate::Work::Back { .. },
            ..
        }
    ));
    let mut put_down = false;
    let mut went_in = false;
    for _ in 0..300 {
        world.step();
        let s = worker(&world);
        put_down |= world
            .wares_at(HUT_FLAG)
            .iter()
            .any(|&w| world.ware(w).is_some_and(|w| w.good == PLANK))
            || (s.carrying.is_none() && !went_in);
        went_in |= matches!(
            s.job,
            crate::SettlerJob::Worker {
                work: crate::Work::Rest | crate::Work::In | crate::Work::StepIn,
                ..
            }
        );
    }
    assert!(put_down, "the plank put down at the flag");
    assert!(went_in, "then in");
    assert!(
        world.wares().any(|(_, w)| w.good == PLANK),
        "the plank is a ware"
    );
}

/// A save's site at (20, 16) with its constructor in a running unit (step
/// 10) with c positions ordered and r ticks of the step left.
fn constructor_in_unit(c: u32, r: u32) -> (World, RestoreReport) {
    const SITE: (u32, u32) = (20, 16);
    restored_with(false, |saved| {
        let east_south = crate::grid::Direction::new(1).expect("a direction");
        let flag = crate::grid::neighbour(SITE, east_south, 40, 40).expect("its flag");
        saved.flags.push((P0, flag));
        saved.buildings.push(RestoredBuilding {
            player: P0,
            building_type: HUT,
            at: SITE,
            remaining: Some([0.5, 1.0]),
            levelled: 1.0,
            delivered: Vec::new(),
            soldiers: Vec::new(),
            production_on: true,
            stock: Vec::new(),
            worker: false,
            number: None,
            worker_number: None,
        });
        saved.builders = vec![(SITE, 4242)];
        saved.builder_units = vec![(SITE, c, r)];
    })
}

/// The site's decrements in each of the `n` ticks after the load.
fn decrements(world: &mut World, n: usize) -> Vec<u32> {
    let done = |w: &World| w.site((20, 16)).map(|s| s.work_done).expect("the site");
    (0..n)
        .map(|_| {
            let d = done(world);
            world.step();
            done(world) - d
        })
        .collect()
}

// FMT-SAV-3.5a (e, Q-1033 answered 2026-11-08 c): a constructor in a
// running unit (step 10) with c = 2 positions ordered and r = 35 ticks of
// his step left works at position 1: 35 decrements in the 35 ticks after
// the load, the first in the first, then none while he walks to position 2
// (ECO-HQ-4.6b step 4, 4.6a). With r = 0 at c = 2 the step is over: the
// next update orders that walk (no decrement). With r = 0 at c = 4 the
// unit's extra decrement comes in the first tick and the unit ends. With
// r = 55 he walks to position c - 1 first.
// spec: FMT-SAV-3.5a, ECO-HQ-4.6a
#[test]
fn a_constructor_goes_on_with_his_running_unit() {
    let (mut world, report) = constructor_in_unit(2, 35);
    assert_eq!((report.builders, report.builder_units), (1, 1));
    let per_tick = decrements(&mut world, 40);
    assert_eq!(per_tick[..35], [1; 35], "the step's last 35 ticks");
    assert_eq!(per_tick[35], 0, "walking to position 2");
    let (mut world, _) = constructor_in_unit(2, 0);
    assert_eq!(decrements(&mut world, 1), [0], "the walk to position 2");
    let (mut world, _) = constructor_in_unit(4, 0);
    assert_eq!(decrements(&mut world, 3), [1, 0, 0], "the extra decrement");
    // r = 55: walking to position 2; the step's 55 decrements start in the
    // walk's arrival tick, then none while he walks to position 3.
    let (mut world, report) = constructor_in_unit(3, 55);
    assert_eq!(report.builder_units, 1);
    let per_tick = decrements(&mut world, 80);
    let first = per_tick.iter().position(|&d| d == 1).expect("a decrement");
    assert!(first < 10, "{per_tick:?}");
    assert_eq!(per_tick[first..first + 55], [1; 55], "{per_tick:?}");
    assert_eq!(per_tick[first + 55], 0, "walking to position 3");
    // c = 0 (no walk ordered yet) and r above 55 are not handed over.
    assert_eq!(constructor_in_unit(0, 55).1.builder_units, 0);
    assert_eq!(constructor_in_unit(1, 56).1.builder_units, 0);
}

// FMT-SAV-3.8 (T12): a finished ship of the save is launched on its water
// field with its state, counter, expedition goods and flag and last port.
// spec: FMT-SAV-3.8
#[test]
fn a_finished_ship_comes_back_on_the_water() {
    let (world, report) = restored_with(false, |saved| {
        saved.finished_ships = vec![crate::RestoredFinishedShip {
            player: P0,
            coast: (20, 20),
            water: (21, 20),
            state: 4,
            counter: 17,
            store: vec![(PLANK, 4), (STONE, 6)],
            expedition: true,
            port: Some((22, 20)),
            harbour: None,
            called: None,
        }];
    });
    assert_eq!(report.finished_ships, 1);
    let ship = &world.ships()[0];
    assert!(ship.finished() && ship.vessel.launched);
    assert_eq!(ship.vessel.walker.at, (21, 20));
    assert_eq!(ship.vessel.state, crate::navy::ShipState::Waiting);
    assert_eq!(ship.vessel.counter, 17);
    assert!(ship.vessel.expedition);
    assert_eq!(ship.vessel.aboard, [(PLANK, 4), (STONE, 6)]);
    assert_eq!(ship.vessel.port, Some((22, 20)));
}

// FMT-SAV-3.11, T13: a load at tick 36058 with the saved productivity count
// 59 (not 36058 mod 600 = 58): the next decay comes 541 ticks after the load
// (the 541st tick, 36598), none before; the sampling count starts at 0, so
// the first tick after the load takes the land sample, the next four the
// others, and the 601st ends the cycle (the frags roll over).
// spec: FMT-SAV-3.11, UI-GAME-8.5a, ECO-OUT-3.11
#[test]
fn a_load_keeps_the_productivity_count_and_restarts_the_sampling() {
    let series: Vec<Vec<u32>> = (0..22).map(|_| vec![1, 1]).collect();
    let (mut world, _) = restored_with(false, |saved| {
        saved.tick = 36_058;
        saved.productivity_count = Some(59);
        saved.statistics = vec![(P0, series.clone())];
    });
    assert_eq!(world.tick(), 36_058);
    let decays: Vec<u64> = (36_058..36_058 + 1200)
        .filter(|t| (t + world.productivity_phase).is_multiple_of(600))
        .collect();
    assert_eq!(decays[0], 36_058 + 540, "the 541st tick after the load");
    assert_eq!(decays[1], 36_058 + 1140);
    let land = |world: &World| world.statistics(P0).unwrap().series[0].len();
    let frags = |world: &World| world.statistics(P0).unwrap().series[7].len();
    let before = land(&world);
    world.step();
    assert_eq!(land(&world), before + 1, "count 0 in the first tick");
    for _ in 1..600 {
        world.step();
    }
    assert_eq!((land(&world), frags(&world)), (before + 1, 2));
    world.step();
    assert_eq!(frags(&world), 3, "the 601st tick ends the cycle");
    world.step();
    assert_eq!(land(&world), before + 2);
}

/// A saved package lying at `flag` without a destination.
fn lying(player: crate::PlayerId, flag: (u32, u32), good: u32) -> RestoredWare {
    RestoredWare {
        player,
        good,
        lying_at: Some(flag),
        carried_by: None,
        to: None,
        next: None,
        destination: None,
        order_stamp: None,
        recheck: 0,
        wait_score: 0,
    }
}

// FMT-SAV-3.6 (Q-1022, Q-1023): a package keeps its destination and the order
// it serves, with the tick the order was filed; lying = 0 puts it in the hands
// of the carrier its carrier ref names, carrying it to the flag it holds its
// place at; a warehouse pair's package goes to be stored. FMT-SAV-2.2,
// SIM-START-4.1: the carrier keeps its saved number.
// spec: FMT-SAV-3.6, FMT-SAV-2.2, SIM-START-4.1, ECO-REQ-5.3f, ECO-REQ-5.4
#[test]
fn packages_keep_destinations_orders_and_carriers() {
    let (world, report) = restored_with(true, |saved| {
        saved.wares = vec![
            RestoredWare {
                destination: Some((HUT_AT, false)),
                order_stamp: Some(640),
                wait_score: 3,
                ..lying(P0, HQ_FLAG, PLANK)
            },
            RestoredWare {
                lying_at: None,
                carried_by: Some((0, false)),
                to: Some(HUT_FLAG),
                destination: Some((HUT_AT, false)),
                order_stamp: Some(700),
                ..lying(P0, HQ_FLAG, STONE)
            },
            RestoredWare {
                destination: Some((HQ, true)),
                ..lying(P0, HUT_FLAG, STONE)
            },
        ];
    });
    assert_eq!(
        (
            report.wares,
            report.wares_carried,
            report.wares_with_destination,
            report.orders_served,
            report.wares_skipped
        ),
        (2, 1, 3, 2, 0)
    );
    let at_hq: Vec<_> = world
        .wares_at(HQ_FLAG)
        .iter()
        .map(|&w| world.ware(w).expect("ware").clone())
        .collect();
    assert_eq!(at_hq.len(), 1);
    assert_eq!(at_hq[0].destination, Some(HUT_FLAG));
    assert_eq!(
        at_hq[0].target.map(|t| (t.at, t.storage)),
        Some((HUT_AT, false))
    );
    assert_eq!(
        at_hq[0].order.map(|o| (o.good, o.stamp)),
        Some((PLANK, 640))
    );
    assert_eq!(at_hq[0].wait_score, 3);
    let carrier = &world.carriers()[0];
    assert_eq!(carrier.number, 4321);
    let held = world.ware(carrier.ware.expect("in hands")).expect("ware");
    assert_eq!(held.good, STONE);
    assert_eq!(held.location, crate::transport::WareLocation::Carried(0));
    assert_eq!(held.order.map(|o| o.stamp), Some(700));
    assert_eq!(
        carrier.task,
        CarrierTask::Carry {
            to: HUT_FLAG,
            entry: crate::transport::Entry::Open { stop: false }
        }
    );
    assert_eq!(carrier.walker.destination(), HUT_FLAG);
    assert!(!world.wares_at(HUT_FLAG).contains(&carrier.ware.unwrap()));
    let stored = world.ware(world.wares_at(HUT_FLAG)[0]).expect("ware");
    assert_eq!(stored.target.map(|t| (t.at, t.storage)), Some((HQ, true)));
    assert!(stored.order.is_none());
}

// FMT-SAV-3.6: the restored game plays on with its packages: the carried stone
// and the plank reach the hut, the stored stone the headquarters, and every
// soak invariant holds.
// spec: FMT-SAV-3.6, TRN-CARRY-3.3
#[test]
fn restored_packages_are_delivered() {
    let (mut world, _) = restored_with(true, |saved| {
        saved.wares = vec![
            RestoredWare {
                lying_at: None,
                carried_by: Some((0, false)),
                to: Some(HUT_FLAG),
                destination: Some((HUT_AT, false)),
                order_stamp: Some(700),
                ..lying(P0, HQ_FLAG, PLANK)
            },
            RestoredWare {
                destination: Some((HQ, true)),
                ..lying(P0, HUT_FLAG, STONE)
            },
        ];
    });
    let mut checker = crate::invariants::Checker::new(&world);
    let mut broken = Vec::new();
    for _ in 0..600 {
        world.step();
        broken.extend(checker.check(&world));
    }
    assert!(broken.is_empty(), "{broken:#?}");
    assert_eq!(world.wares().count(), 0, "both delivered");
}

// FMT-SAV-3.8, ECO-FOOD-7.3b (Q-1024): a ship of the save still being built
// stands on its coast field over its water field with the work left on its
// frame and main parts.
// spec: FMT-SAV-3.8, ECO-FOOD-7.3b, ECO-FOOD-7.4a
#[test]
fn a_ship_being_built_comes_back() {
    let (world, report) = restored_with(false, |saved| {
        saved.ships = vec![
            RestoredShip {
                player: P0,
                coast: (20, 20),
                water: (21, 20),
                frame_left: 0.0,
                main_left: 0.193,
            },
            RestoredShip {
                player: P0,
                coast: (200, 20),
                water: (201, 20),
                frame_left: 0.0,
                main_left: 0.5,
            },
        ];
    });
    assert_eq!(report.ships, 1);
    let ship = &world.ships()[0];
    assert_eq!(
        (ship.owner, ship.at, ship.water),
        (P0, (20, 20), Some((21, 20)))
    );
    assert_eq!(f32::from_bits(ship.main_work), 0.193);
    assert_eq!(ship.frame_work, 0);
    assert!(!ship.finished());
}

// The worker the save names sits in its building as one that has arrived,
// and the road's carrier stands idle on the road's idle field: nothing is
// sent from the headquarters for them.
// spec: FMT-SAV-3.4a, FMT-SAV-3.9, ECO-REQ-4.2, TRN-CARRY-1.2, TRN-CARRY-1.3
#[test]
fn named_worker_and_carrier_are_restored() {
    let (mut world, report) = restored(true);
    assert_eq!((report.workers, report.carriers, report.donkeys), (1, 1, 0));
    let path: Vec<_> = (10..=16).map(|x| (x, 9)).collect();
    assert_eq!(world.carriers().len(), 1);
    let carrier = &world.carriers()[0];
    assert_eq!(carrier.walker.at, idle_field(&path));
    assert_eq!(carrier.task, CarrierTask::Idle);
    assert_eq!(world.roads().next().unwrap().carrier, Some(0));
    let workers: Vec<_> = world
        .settlers
        .iter()
        .flatten()
        .filter(
            |s| matches!(s.job, crate::SettlerJob::Worker { target, .. } if target.at == HUT_AT),
        )
        .collect();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].profession, WOODCUTTER);
    for _ in 0..400 {
        world.step();
    }
    assert_eq!(world.carriers().len(), 1);
    assert_eq!(req::count(&world, WOODCUTTER), 4);
    assert_eq!(req::count(&world, HELPER), 26);
}

// Without them the economy sends a carrier (a helper) and a woodcutter from
// the headquarters, as before.
// spec: FMT-SAV-3.4a, FMT-SAV-3.9, ECO-REQ-1.1
#[test]
fn unnamed_settlers_come_from_the_warehouse() {
    let (mut world, report) = restored(false);
    assert_eq!((report.workers, report.carriers), (0, 0));
    for _ in 0..400 {
        world.step();
    }
    assert_eq!(world.carriers().len(), 1);
    assert_eq!(req::count(&world, WOODCUTTER), 3);
    assert_eq!(req::count(&world, HELPER), 25);
}

/// A plain generator for the damaged saves below.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }

    fn player(&mut self) -> crate::PlayerId {
        crate::PlayerId(self.below(12) as u8)
    }

    fn fraction(&mut self) -> f32 {
        [0.0, 0.5, 1.0, -1.0, 2.0, f32::NAN, f32::INFINITY][self.below(7) as usize]
    }

    fn field(&mut self) -> (u32, u32) {
        (self.below(50) as u32, self.below(50) as u32)
    }
}

// A damaged or unknown save never stops the game: players and owners out
// of range, fields off the map, unknown types, odd fractions, roads of no
// or one field, roads whose fields are not neighbours or cross each other
// are left out, and the game runs on.
// spec: FMT-SAV-3.4a, FMT-SAV-3.9 (what a restore takes over)
#[test]
fn damaged_saves_never_panic() {
    let mut g = Lcg(0x5eed);
    let types = [CASTLE, HUT, WOODCUTTER, 0xdead_beef];
    for round in 0..200 {
        let setup = GameSetup {
            local_player: P0,
            ..Default::default()
        };
        let mut world = World::new_game(9, flat(40, 40), req::rules(&req::full_stock()), &setup);
        let fields = world.owners.len();
        let owners = if round % 3 == 0 {
            fields
        } else {
            g.below(2000) as usize
        };
        let saved = Restore {
            players: (0..g.below(4)).map(|_| g.player()).collect(),
            tick: g.below(1 << 40),
            owners: (0..owners)
                .map(|_| (g.below(3) > 0).then(|| g.player()))
                .collect(),
            explored: vec![0xff; if round % 2 == 0 { fields } else { 7 }],
            buildings: (0..g.below(12))
                .map(|_| RestoredBuilding {
                    player: g.player(),
                    building_type: types[g.below(4) as usize],
                    at: g.field(),
                    remaining: (g.below(2) == 0).then(|| [g.fraction(), g.fraction()]),
                    levelled: g.fraction(),
                    delivered: vec![(g.below(3) as u32, g.below(5) as u32)],
                    production_on: g.below(2) == 0,
                    stock: vec![(g.below(40) as u32, g.below(9) as u32)],
                    worker: g.below(2) == 0,
                    number: None,
                    worker_number: None,
                    soldiers: (0..g.below(4))
                        .map(|_| crate::RestoredSoldier {
                            number: None,
                            profession: 48 + g.below(10) as u32,
                            health: g.below(9) as u32,
                            healing: g.below(700) as u32,
                        })
                        .collect(),
                })
                .collect(),
            flags: (0..g.below(10)).map(|_| (g.player(), g.field())).collect(),
            roads: (0..g.below(10))
                .map(|_| {
                    let mut path = vec![g.field()];
                    for _ in 0..g.below(8) {
                        let &(x, y) = path.last().unwrap();
                        let step = match g.below(3) {
                            0 => (x + 1, y),
                            1 => (x, y + 1),
                            _ => g.field(),
                        };
                        path.push(step);
                    }
                    RestoredRoad {
                        player: g.player(),
                        path,
                        carrier: g.below(2) == 0,
                        donkey: g.below(2) == 0,
                        numbers: [None, None],
                        standing: [None, None],
                    }
                })
                .collect(),
            ..Default::default()
        };
        world.restore(&saved);
        for _ in 0..30 {
            world.step();
        }
    }
}

// FMT-SAV-3.4a, 3.5c (Q-299): the soldiers a military building names are present
// in it with their rank, health and healing counter, up to its limit (a profession
// that is no soldier rank is left out; a health outside the rank's scale is kept
// within it); the building has claimed its land already (TER-2.1), so its first
// update claims nothing, and it asks for no soldiers it has.
// spec: FMT-SAV-3.4a, FMT-SAV-3.5c, TER-2.1, MIL-SOL-1.1
#[test]
fn named_soldiers_are_present() {
    use crate::military_tests::{self as mil, GUARD, RANK};
    let setup = GameSetup {
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), mil::rules(), &setup);
    let fields = world.owners.len();
    let at = (20, 20);
    let saved = Restore {
        players: vec![P0],
        tick: 1000,
        // The land within 2 of the guard house is owned: its claim would
        // take the fields within 9.
        owners: (0..fields as u32)
            .map(|i| {
                let field = (i % 40, i / 40);
                (crate::grid::distance(field, at) <= 2).then_some(P0)
            })
            .collect(),
        explored: vec![1; fields],
        buildings: vec![RestoredBuilding {
            player: P0,
            building_type: GUARD,
            at,
            remaining: None,
            levelled: 1.0,
            delivered: Vec::new(),
            production_on: true,
            stock: Vec::new(),
            worker: false,
            number: None,
            worker_number: None,
            soldiers: [
                (RANK[2], 2, 17),
                (999, 1, 0),
                (RANK[0], 99, 600),
                (RANK[4], 7, 0),
                (RANK[1], 4, 3),
            ]
            .map(|(profession, health, healing)| crate::RestoredSoldier {
                number: None,
                profession,
                health,
                healing,
            })
            .to_vec(),
        }],
        flags: Vec::new(),
        roads: Vec::new(),
        ..Default::default()
    };
    let report = world.restore(&saved);
    // The guard house holds 3: the first three named, the 999 left out.
    assert_eq!(report.soldiers, 2);
    let garrison = world.garrison(at);
    let ranks: Vec<_> = garrison
        .iter()
        .map(|s| (s.rank, s.health, s.present))
        .collect();
    // The sergeant as saved; the private's 99 kept within its scale of 3.
    assert_eq!(ranks, [(2, 2, true), (0, 3, true)]);
    let healing: Vec<u32> = world
        .settlers()
        .filter_map(|(_, s)| match s.job {
            crate::SettlerJob::Garrison { heal, .. } => Some(heal),
            _ => None,
        })
        .collect();
    assert_eq!(healing, [17, 600]);
    // A health below 1 is kept within the scale too.
    let mut low = world.clone();
    assert!(low.seat_soldier(
        P0,
        crate::RestoredSoldier {
            number: None,
            profession: RANK[1],
            health: 0,
            healing: 0,
        },
        at,
    ));
    assert_eq!(low.garrison(at).last().map(|s| s.health), Some(1));
    assert!(world.military_at(at).unwrap().applied);
    let owned = |w: &World| w.owners.iter().filter(|o| **o == Some(P0)).count();
    let before = owned(&world);
    world.step();
    assert_eq!(owned(&world), before, "no new claim");
    assert_eq!(world.garrison(at).len(), 2);
}

/// ECO-FOOD-5.11b: a restored game's clock is what the running game's would
/// read at that tick, the float32 sum with its rounding, not 0.
// spec: ECO-FOOD-5.11b, FMT-SAV-3.10b
#[test]
fn a_restored_games_clock_goes_on_from_its_tick() {
    let mut running = req::fresh_game();
    while running.tick() < 1000 {
        running.step();
    }
    let mut world = req::fresh_game();
    world.restore(&Restore {
        tick: 1000,
        ..Restore::default()
    });
    assert_eq!(world.tick(), 1000);
    assert_eq!(world.game_seconds(), running.game_seconds());
    assert!((world.game_seconds() - 100.0).abs() < 0.1);
}

/// FMT-SAV-3.4b: what the file's stock holds is what lies at
/// the site. A site whose frame is done and whose first main unit is under way
/// has used those materials, so it orders nothing it has had, and the materials
/// left are the units left: none is over when it is finished (the plank of the
/// Fight autosave's site (51, 19), gone at tick 36670).
// spec: FMT-SAV-3.4b, ECO-HQ-4.5, ECO-HQ-4.6, ECO-HQ-4.7
#[test]
fn a_restored_site_orders_what_its_work_has_not_used() {
    use crate::requests::tests::{PLANK, STONE};
    let new_site = || crate::construction::ConstructionSite {
        size: crate::BuildingSize::Medium,
        levelling_needed: false,
        levelling_bits: crate::construction::LEVELLING_DONE.to_bits(),
        level_direction: 0,
        leveller: None,
        builder: None,
        frame_costs: vec![(PLANK, 2)],
        costs: vec![(STONE, 2)],
        delivered: vec![],
        on_site: vec![],
        work_done: 0,
        frame_end: 441,
        work_total: 882,
        unit_ticks: 221,
        leveller_on_way: false,
        builder_on_way: false,
        orders_filed: false,
    };
    let mut site = new_site();
    // Frame done, 209 of the main part's decrements: one unit under way.
    site.restore_work(441, 209, &[(STONE, 1)], Some(PLANK));
    assert_eq!(site.work_done, 441 + 221);
    assert_eq!(site.on_site, [(STONE, 1)]);
    assert!(site.shortfall().is_empty(), "{:?}", site.shortfall());
    // A fresh site: nothing used, all is ordered.
    let mut fresh = new_site();
    fresh.restore_work(0, 0, &[], Some(PLANK));
    assert_eq!(fresh.work_done, 0);
    assert_eq!(fresh.shortfall(), [(PLANK, 2), (STONE, 2)]);
    // A frame half done with the other frame material still to come, and a
    // main material delivered early.
    let mut early = new_site();
    early.restore_work(221, 0, &[(STONE, 1)], Some(PLANK));
    assert_eq!(early.work_done, 221);
    assert_eq!(early.shortfall(), [(PLANK, 1), (STONE, 1)]);
    // The last unit's end still counts as the whole part.
    let mut done = new_site();
    done.restore_work(441, 400, &[], Some(PLANK));
    assert_eq!(done.work_done, 882);
    assert!(done.shortfall().is_empty());
}

// FMT-SAV-2.2, SIM-START-4.1 (Q-1025): the counter is the save's next number;
// the saved buildings, workers and carriers keep their own numbers, and what
// the rebuild makes besides numbers on from the counter.
// spec: FMT-SAV-2.2, SIM-START-4.1
#[test]
fn saved_object_numbers_are_kept() {
    let (world, _) = restored_with(true, |saved| {
        saved.next_object = Some(5000);
        saved.buildings[1].number = Some(4000);
        saved.buildings[1].worker_number = Some(4001);
    });
    assert_eq!(world.building_at(HUT_AT).map(|b| b.serial), Some(4000));
    let worker = world
        .settlers()
        .find(|(_, s)| s.list == crate::SettlerList::Workers)
        .map(|(_, s)| s.number);
    assert_eq!(worker, Some(4001));
    assert_eq!(world.carriers()[0].number, 4321);
    // The headquarters had no saved number: it took the counter's next,
    // after the two flags' four each (SIM-START-4.2).
    assert_eq!(world.building_at(HQ).map(|b| b.serial), Some(5008));
    assert!(world.next_object() > 5000);
}

// FMT-SAV-3.5h (Q-257 a): a carrier holding a saved ware goes on from the
// field it stands on (its current field), when that lies on its road.
// spec: FMT-SAV-3.5h, FMT-SAV-3.6
#[test]
fn a_carrier_with_a_ware_goes_on_from_its_saved_field() {
    let carried = |standing: Option<(u32, u32)>| {
        let (world, _) = restored_with(true, |saved| {
            saved.roads[0].standing = [standing, None];
            saved.wares = vec![RestoredWare {
                lying_at: None,
                carried_by: Some((0, false)),
                to: Some(HUT_FLAG),
                destination: Some((HUT_AT, false)),
                order_stamp: Some(700),
                ..lying(P0, HQ_FLAG, PLANK)
            }];
        });
        let walker = world.carriers()[0].walker.clone();
        (walker.at, walker.destination())
    };
    assert_eq!(carried(Some((15, 9))), ((15, 9), HUT_FLAG));
    // Off its road: its idle field.
    let idle = idle_field(&(10..=16).map(|x| (x, 9)).collect::<Vec<_>>());
    assert_eq!(carried(Some((15, 20))), (idle, HUT_FLAG));
    assert_eq!(carried(None), (idle, HUT_FLAG));
}

/// A tree kind (elevation 1, ECO-OUT-3.4) and its good.
const TREE: u32 = 300;
const WOOD: u32 = 6;
/// The field a worker's walk out ends on, and the tree's beyond it.
const OUT_FIELD: (u32, u32) = (16, 13);
const TREE_AT: (u32, u32) = (16, 14);

/// The hut as an outdoor worker's of `cycle` (ECO-OUT-3, 4: its action 55
/// ticks), with a tree standing on [`TREE_AT`] at its field's sample.
fn outdoor_hut(cycle: crate::WorkCycle) -> impl FnOnce(&mut World) {
    move |w: &mut World| {
        let kind = w.building_types.get_mut(&HUT).expect("the hut");
        kind.cycle = cycle;
        kind.production_ticks = 55;
        kind.product = (cycle == crate::WorkCycle::Gatherer).then_some(WOOD);
        w.deposit_kinds.insert(
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
        w.add_deposit(
            TREE_AT,
            crate::Deposit {
                kind: TREE,
                position: crate::grid::field_sample(TREE_AT),
                state: crate::DepositState::Growing { ripe_at: 0 },
                reserved_by: None,
                life_left: None,
            },
        );
    }
}

/// The hut's worker: his step, timer, field and place on the lattice.
fn hut_worker(w: &World) -> (crate::Work, u32, (u32, u32), Option<crate::FinePosition>) {
    w.settlers()
        .find_map(|(_, s)| match s.job {
            crate::SettlerJob::Worker { target, work } if target.at == HUT_AT => {
                Some((work, s.timer, s.walker.at, w.fine_position(s)))
            }
            _ => None,
        })
        .expect("the hut's worker")
}

/// A worker of the hut out at the tree: working with `left` ticks of his
/// action to go, or on his fine approach `walk`.
fn outdoors(left: Option<u32>, walk: Option<crate::RestoredFineWalk>) -> crate::RestoredOutdoor {
    crate::RestoredOutdoor {
        at: HUT_AT,
        field: OUT_FIELD,
        spot: TREE_AT,
        state: 33,
        left,
        walk,
        rough: None,
        animal: None,
        leg: None,
    }
}

// FMT-SAV-3.5a (b, 2026-11-09, Q-257): a woodcutter of a save cutting his
// tree with 2 ticks of the cut left stands on the field his walk out ended
// on, one lattice step short of the tree and facing it (ECO-OUT-3.4, 3.5),
// and harvests it in the 2nd tick after the load (3.6). The tree stays the
// hut's target, reserved for it (ECO-OUT-2.6). Its removal is armed as at
// the work's start (3.5): torn down at once, the hut's tree still starts its
// removal 60 ticks after the work began, 5 after the action would have
// ended (GFX-ANIM-3.8).
// spec: FMT-SAV-3.5a, ECO-OUT-2.6, ECO-OUT-3.5, ECO-OUT-3.6
#[test]
fn a_woodcutter_of_a_save_goes_on_cutting_his_tree() {
    let save = |saved: &mut Restore| saved.workers_outdoors = vec![outdoors(Some(2), None)];
    let (mut world, report) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, save);
    assert_eq!((report.workers_at_spot, report.workers_approaching), (1, 0));
    assert_eq!(world.reserved_target(HUT_AT), Some(TREE_AT));
    let reserved = |w: &World| w.deposit(TREE_AT).and_then(|d| d.reserved_by).map(|t| t.at);
    assert_eq!(reserved(&world), Some(HUT_AT));
    let (work, timer, field, fine) = hut_worker(&world);
    assert_eq!(
        (work, timer, field),
        (crate::Work::AtSpot { spot: TREE_AT }, 2, OUT_FIELD)
    );
    let tree = crate::grid::field_sample(TREE_AT);
    let fine = fine.expect("on the lattice");
    assert_eq!(fine.next, None);
    assert_eq!(
        fine.facing,
        Some(crate::movement::direction_toward(fine.at, tree))
    );
    let line = crate::movement::fine_approach(
        &world.terrain,
        crate::grid::field_sample(OUT_FIELD),
        tree,
        0,
    );
    assert_eq!(Some(&fine.at), line.iter().rev().nth(1), "one step short");
    let load = world.tick();
    world.step();
    assert!(matches!(hut_worker(&world).0, crate::Work::AtSpot { .. }));
    world.step();
    assert!(matches!(hut_worker(&world).0, crate::Work::TakeWare { .. }));
    assert_eq!(
        world.deposit(TREE_AT).map(|d| d.state),
        Some(crate::DepositState::Removing {
            since: load + 1,
            gone_at: load + 1 + 18,
        }),
        "harvested in the 2nd tick"
    );
    assert_eq!(world.reserved_target(HUT_AT), None);
    // Interrupted: the armed removal (the work began in tick load - 54).
    let (mut world, _) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, save);
    let load = world.tick();
    world.submit(crate::Command::RemoveBuilding {
        player: P0,
        at: HUT_AT,
    });
    while world.tick() < load + 6 {
        world.step();
        assert!(world.deposit(TREE_AT).is_some_and(|d| d.ripe(world.tick())));
    }
    world.step();
    assert_eq!(
        world.deposit(TREE_AT).map(|d| d.state),
        Some(crate::DepositState::Removing {
            since: load + 6,
            gone_at: load + 6 + 19,
        })
    );
}

// FMT-SAV-3.5a (b, 2026-11-09, Q-257): a forester of a save planting with 3
// ticks left: the spot's tree is the one he plants, not growing yet and
// reserved for his hut (ECO-OUT-4.5); in the 3rd tick after the load the
// planting ends and it starts growing (4.7). No removal is armed.
// spec: FMT-SAV-3.5a, ECO-OUT-4.5, ECO-OUT-4.7
#[test]
fn a_forester_of_a_save_goes_on_planting() {
    let (mut world, report) = restored_in(outdoor_hut(crate::WorkCycle::Forester), true, |s| {
        s.workers_outdoors = vec![outdoors(Some(3), None)];
    });
    assert_eq!(report.workers_at_spot, 1);
    let deposit = |w: &World| *w.deposit(TREE_AT).expect("the sapling");
    assert_eq!(deposit(&world).state, crate::DepositState::Sapling);
    assert_eq!(deposit(&world).reserved_by.map(|t| t.at), Some(HUT_AT));
    assert_eq!(world.reserved_target(HUT_AT), None);
    let load = world.tick();
    for _ in 0..2 {
        world.step();
        assert_eq!(deposit(&world).state, crate::DepositState::Sapling);
    }
    world.step();
    assert_eq!(
        deposit(&world).state,
        crate::DepositState::Growing {
            ripe_at: load + 2 + 1001
        }
    );
    assert_eq!(deposit(&world).reserved_by, None);
    for _ in 0..80 {
        world.step();
    }
    assert_eq!(world.fall_of(TREE_AT), None);
    assert!(world.deposit(TREE_AT).is_some());
}

// FMT-SAV-3.5a (b, 2026-11-09, Q-257), FMT-SAV-3.5h: a woodcutter of a save
// on his fine approach to the tree (the path from the tree back to his
// field's sample, the index naming the running step's goal) goes on from
// the saved step and progress (TRN-FREE-5.1 to 5.3) and starts cutting when
// it ends. A walk that is not the straight line of the approach is not
// taken over: he is seated in his hut.
// spec: FMT-SAV-3.5a, FMT-SAV-3.5h, TRN-FREE-5.3
#[test]
fn a_woodcutter_of_a_save_goes_on_with_his_approach() {
    let (world, _) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |_| {});
    let from = crate::grid::field_sample(OUT_FIELD);
    let tree = crate::grid::field_sample(TREE_AT);
    let line = crate::movement::fine_approach(&world.terrain, from, tree, 0);
    let steps = &line[..line.len() - 1];
    let speed = world.settings.settler_speed;
    let total = crate::movement::lattice_walk_ticks(&world.terrain, speed, from, steps);
    let (at, next, progress) =
        crate::movement::lattice_walk_at(&world.terrain, speed, from, steps, 15);
    let next = next.expect("walking");
    let j = line.iter().position(|&p| p == next).expect("on the line");
    let mut path: Vec<_> = line.iter().rev().copied().collect();
    path.push(from);
    let walk = crate::RestoredFineWalk {
        path,
        index: (line.len() - 1 - j) as u32,
        progress,
        rate: 0.09,
    };
    let (mut world, report) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |s| {
        s.workers_outdoors = vec![outdoors(None, Some(walk.clone()))];
    });
    assert_eq!((report.workers_at_spot, report.workers_approaching), (0, 1));
    let (work, timer, field, fine) = hut_worker(&world);
    assert_eq!(
        (work, field),
        (
            crate::Work::Approach {
                spot: TREE_AT,
                walk: None
            },
            OUT_FIELD
        )
    );
    assert_eq!(timer, total - 15);
    let fine = fine.expect("on the lattice");
    assert_eq!((fine.at, fine.next), (at, Some(next)));
    assert_eq!(fine.progress_bits, progress.to_bits());
    assert_eq!(world.reserved_target(HUT_AT), Some(TREE_AT));
    let mut walked = 0;
    while matches!(hut_worker(&world).0, crate::Work::Approach { .. }) {
        world.step();
        walked += 1;
    }
    assert_eq!(walked, total - 15);
    assert_eq!(hut_worker(&world).0, crate::Work::AtSpot { spot: TREE_AT });
    // Off the line: seated.
    let mut crooked = walk.clone();
    crooked.path[1] = (crooked.path[1].0 + 1, crooked.path[1].1);
    let (world, report) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |s| {
        s.workers_outdoors = vec![outdoors(None, Some(crooked))];
    });
    assert_eq!(report.workers_approaching, 0);
    assert_eq!(hut_worker(&world).0, crate::Work::Begin);
    assert_eq!(world.reserved_target(HUT_AT), None);
}

// FMT-SAV-3.5a (b, 2026-11-09 c), 3.5h (T14): a woodcutter of a save on the
// rough part of his walk out (no fine walk; his free path saved from the
// tree back to his start, the running step's index, progress and rate, the
// stop-short count 1) walks on from the saved step to the field before the
// tree (TRN-FREE-4.2, ECO-OUT-3.4), his tree reserved for his hut, then
// makes his fine approach and cuts (ECO-OUT-3.5). A list that does not hold
// together (a gap) is not taken over: he is seated inside.
// spec: FMT-SAV-3.5a, FMT-SAV-3.5h, ECO-OUT-3.4 (T14)
#[test]
fn t14_a_worker_of_a_save_walks_on_out_to_his_tree() {
    let line = |w: &World| {
        let d = (0..6)
            .find(|&d| w.terrain.neighbour(TREE_AT, d) == Some(OUT_FIELD))
            .expect("the field before the tree is its neighbour");
        let mut path = vec![TREE_AT, OUT_FIELD];
        for _ in 0..3 {
            let last = *path.last().unwrap();
            path.push(w.terrain.neighbour(last, d).expect("on the map"));
        }
        path
    };
    let (probe, _) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |_| {});
    let path = line(&probe);
    let rough = crate::RestoredRoughWalk {
        path: path.clone(),
        index: 2,
        progress: 0.5,
        rate: 0.045,
        stop_short: 1,
    };
    let walking_out = |rough: crate::RestoredRoughWalk| crate::RestoredOutdoor {
        at: HUT_AT,
        field: rough.path[rough.index as usize + 1],
        spot: TREE_AT,
        state: 33,
        left: None,
        walk: None,
        rough: Some(rough),
        animal: None,
        leg: None,
    };
    let (mut world, report) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |s| {
        s.workers_outdoors = vec![walking_out(rough.clone())];
    });
    assert_eq!(
        (
            report.workers_walking_out,
            report.workers_approaching,
            report.workers_at_spot
        ),
        (1, 0, 0)
    );
    let worker = |w: &World| {
        w.settlers()
            .find_map(|(_, s)| match s.job {
                crate::SettlerJob::Worker { target, work } if target.at == HUT_AT => {
                    Some((work, s.walker.clone()))
                }
                _ => None,
            })
            .expect("the hut's worker")
    };
    let (work, walker) = worker(&world);
    assert_eq!(work, crate::Work::ToSpot { spot: TREE_AT });
    assert_eq!((walker.at, walker.next), (path[3], Some(path[2])));
    assert_eq!(walker.progress_bits, 0.5f32.to_bits());
    assert_eq!(walker.path, [path[1]]);
    assert_eq!(world.reserved_target(HUT_AT), Some(TREE_AT));
    // On to the field before the tree (k = 1), not onto the tree's.
    let mut fields = vec![walker.at];
    let mut ticks = 0;
    while matches!(worker(&world).0, crate::Work::ToSpot { .. }) {
        world.step();
        let at = worker(&world).1.at;
        if fields.last() != Some(&at) {
            fields.push(at);
        }
        ticks += 1;
        assert!(ticks < 200, "he never got there");
    }
    assert_eq!(fields, [path[3], path[2], path[1]]);
    assert!(matches!(
        worker(&world).0,
        crate::Work::Approach { spot: TREE_AT, .. }
    ));
    while matches!(worker(&world).0, crate::Work::Approach { .. }) {
        world.step();
    }
    assert_eq!(worker(&world).0, crate::Work::AtSpot { spot: TREE_AT });
    // A gap in the list: seated.
    let mut gap = rough.clone();
    gap.path.remove(1);
    gap.index = 1;
    let (world, report) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |s| {
        s.workers_outdoors = vec![walking_out(gap)];
    });
    assert_eq!(report.workers_walking_out, 0);
    assert_eq!(worker(&world).0, crate::Work::Begin);
}

// FMT-SAV-3.5a (b, 2026-11-11, E-sav-014): a farmer of a save going out to
// harvest (harvest step 8) goes on from where the save has him, not seated
// inside: on the door step (leg 1) he finishes the step from the door field
// onto the farm's field at the saved progress, and in the tick it ends his
// walk to the flag starts as a walk of its own, its first step at progress 0
// (ECO-WORK-3.8b); on the walk to the flag (leg 0) he finishes that step.
// From the flag he walks to the farm's reserved grain (ECO-OUT-3.4), which
// stays reserved for the farm, and harvests it (3.5). A walk that is not its
// leg's (leg 1 not from the door field) is not taken over: he is seated.
// (2026-11-12, E-sav-016) A woodcutter in step 8 goes on the same way.
// spec: FMT-SAV-3.5a, ECO-WORK-3.8b, ECO-OUT-3.3, ECO-OUT-3.4
#[test]
fn a_farmer_of_a_save_going_out_to_harvest_walks_on() {
    let farm = |w: &mut World| {
        outdoor_hut(crate::WorkCycle::Farm)(w);
        let kind = w.building_types.get_mut(&HUT).expect("the hut");
        kind.doors = [Some(4); 3];
    };
    let (probe, _) = restored_in(farm, true, |_| {});
    let door = probe.door_field(HUT_AT);
    assert_ne!(door, HUT_AT, "the farm has a door field");
    let going_out = |leg: u32, path: Vec<(u32, u32)>, progress: f32| crate::RestoredOutdoor {
        at: HUT_AT,
        field: path[1],
        spot: TREE_AT,
        state: 33,
        left: None,
        walk: None,
        rough: Some(crate::RestoredRoughWalk {
            path,
            index: 0,
            progress,
            rate: 0.045,
            stop_short: 0,
        }),
        animal: None,
        leg: Some(leg),
    };
    let worker = |w: &World| {
        w.settlers()
            .find_map(|(_, s)| match s.job {
                crate::SettlerJob::Worker { target, work } if target.at == HUT_AT => {
                    Some((work, s.walker.clone()))
                }
                _ => None,
            })
            .expect("the farm's worker")
    };
    let out = crate::Work::Out {
        spot: Some(TREE_AT),
    };
    let reserved = |w: &World| w.deposit(TREE_AT).and_then(|d| d.reserved_by).map(|t| t.at);
    // Leg 1: on the door step.
    let (mut world, report) = restored_in(farm, true, |s| {
        s.workers_outdoors = vec![going_out(1, vec![HUT_AT, door], 0.225)];
    });
    assert_eq!(
        (
            report.workers_walking_out,
            report.workers_approaching,
            report.workers_at_spot
        ),
        (1, 0, 0)
    );
    let (work, walker) = worker(&world);
    assert_eq!(work, out);
    assert_eq!((walker.at, walker.next), (door, Some(HUT_AT)));
    assert_eq!(walker.progress_bits, 0.225f32.to_bits());
    assert_eq!(walker.path, [HUT_FLAG]);
    assert_eq!(world.reserved_target(HUT_AT), Some(TREE_AT));
    assert_eq!(reserved(&world), Some(HUT_AT));
    // 0.225 + 17 x 0.045 = 0.99: the door step ends in the 18th tick, and
    // the walk to the flag starts there at 0.
    let mut ticks = 0;
    while worker(&world).1.at != HUT_AT {
        world.step();
        ticks += 1;
        assert!(ticks < 40, "the door step never ended");
    }
    let (work, walker) = worker(&world);
    assert_eq!(ticks, 18);
    assert_eq!((work, walker.next), (out, Some(HUT_FLAG)));
    assert_eq!(walker.progress(), 0.0);
    world.step();
    assert!(worker(&world).1.progress() > 0.0);
    // On from the flag to his grain, and to work there.
    let mut fields = vec![HUT_AT];
    while !matches!(worker(&world).0, crate::Work::AtSpot { .. }) {
        world.step();
        let at = worker(&world).1.at;
        if fields.last() != Some(&at) {
            fields.push(at);
        }
        ticks += 1;
        assert!(ticks < 400, "he never got to his grain");
    }
    assert_eq!(fields[1], HUT_FLAG);
    assert_eq!(worker(&world).0, crate::Work::AtSpot { spot: TREE_AT });
    assert_eq!(reserved(&world), Some(HUT_AT));
    // Leg 0: on the walk from the farm's field to its flag.
    let (mut world, report) = restored_in(farm, true, |s| {
        s.workers_outdoors = vec![going_out(0, vec![HUT_FLAG, HUT_AT], 0.09)];
    });
    assert_eq!(report.workers_walking_out, 1);
    let (work, walker) = worker(&world);
    assert_eq!(work, out);
    assert_eq!((walker.at, walker.next), (HUT_AT, Some(HUT_FLAG)));
    assert_eq!(walker.progress_bits, 0.09f32.to_bits());
    assert!(walker.path.is_empty());
    // 0.09 + 20 x 0.045 = 0.99: on the flag in the 21st tick.
    for _ in 0..21 {
        world.step();
    }
    assert_eq!(worker(&world).1.at, HUT_FLAG);
    assert_eq!(
        worker(&world).0,
        crate::Work::ToSpot { spot: TREE_AT },
        "on his way to the grain"
    );
    // (2026-11-12, E-sav-016) A woodcutter in step 8 the same way: from his
    // hut's own field to its flag (leg 0), then on to his tree.
    let (mut world, report) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |s| {
        s.workers_outdoors = vec![going_out(0, vec![HUT_FLAG, HUT_AT], 0.09)];
    });
    assert_eq!(report.workers_walking_out, 1);
    assert_eq!(worker(&world).0, out);
    assert_eq!(reserved(&world), Some(HUT_AT));
    for _ in 0..21 {
        world.step();
    }
    assert_eq!(worker(&world).1.at, HUT_FLAG);
    assert_eq!(worker(&world).0, crate::Work::ToSpot { spot: TREE_AT });
    // Leg 1 not from the door field: seated, the grain not reserved.
    let (world, report) = restored_in(farm, true, |s| {
        s.workers_outdoors = vec![going_out(1, vec![HUT_AT, HUT_FLAG], 0.5)];
    });
    assert_eq!(report.workers_walking_out, 0);
    assert_eq!(worker(&world).0, crate::Work::Begin);
    assert_eq!(world.reserved_target(HUT_AT), None);
}

// FMT-SAV-3.5a (b, 2026-11-11, E-sav-014): a farmer of a save waiting inside
// in his harvest step (7) with 27 left of the countdown goes on with that
// wait (ECO-OUT-3.2): it falls to 0 in the 27th tick after the load and he
// checks in the 28th (not 51 ticks after the load, as at the cycle's start),
// finds the farm's reserved grain, which the farm keeps (ECO-OUT-2.6), and
// goes out (3.3). (2026-11-12, E-sav-016, T15) A woodcutter's hut runs the
// same harvest state; (2026-11-09 c) a hunter's hut takes the hunter
// state's countdown instead (ECO-FOOD-5.3). A forester's hut (no target
// wait), a farm whose harvest state is not at step 7 and a countdown above
// the wait's start of 50 leave the worker seated at the start of his cycle.
// spec: FMT-SAV-3.5a, ECO-OUT-3.2, ECO-OUT-2.6, ECO-FOOD-3.2, ECO-FOOD-5.3
#[test]
fn a_farmer_of_a_save_waiting_for_his_grain_takes_the_saved_countdown() {
    let waiting = |harvest, hunt| crate::RestoredWaiting {
        at: HUT_AT,
        harvest,
        hunt,
        target: Some(TREE_AT),
    };
    let (mut world, report) = restored_in(outdoor_hut(crate::WorkCycle::Farm), true, |s| {
        s.workers_waiting = vec![waiting(Some(27), Some(9))];
    });
    assert_eq!(report.workers_waiting, 1);
    assert_eq!(
        hut_worker(&world).0,
        crate::Work::WaitTarget { countdown: 27 }
    );
    assert_eq!(world.reserved_target(HUT_AT), Some(TREE_AT));
    let reserved = |w: &World| w.deposit(TREE_AT).and_then(|d| d.reserved_by).map(|t| t.at);
    assert_eq!(reserved(&world), Some(HUT_AT));
    let mut ticks = 0;
    while matches!(hut_worker(&world).0, crate::Work::WaitTarget { .. }) {
        world.step();
        ticks += 1;
        assert!(ticks < 60, "he never checked");
    }
    assert_eq!(ticks, 28);
    assert_eq!(
        hut_worker(&world).0,
        crate::Work::Out {
            spot: Some(TREE_AT)
        }
    );
    assert_eq!(reserved(&world), Some(HUT_AT));
    // The woodcutter's hut: the harvest state's countdown, his tree his
    // hut's target; the hunter's hut: the hunter state's countdown.
    for (cycle, left) in [
        (crate::WorkCycle::Gatherer, 27),
        (crate::WorkCycle::Hunter, 9),
    ] {
        let (world, report) = restored_in(outdoor_hut(cycle), true, |s| {
            s.workers_waiting = vec![waiting(Some(27), Some(9))];
        });
        assert_eq!(report.workers_waiting, 1, "{cycle:?}");
        assert_eq!(
            hut_worker(&world).0,
            crate::Work::WaitTarget { countdown: left }
        );
    }
    let (world, _) = restored_in(outdoor_hut(crate::WorkCycle::Gatherer), true, |s| {
        s.workers_waiting = vec![waiting(Some(27), None)];
    });
    assert_eq!(world.reserved_target(HUT_AT), Some(TREE_AT));
    // Seated: a forester's hut, a farm with no harvest step 7, a countdown
    // above 50.
    for (cycle, harvest) in [
        (crate::WorkCycle::Forester, Some(27)),
        (crate::WorkCycle::Farm, None),
        (crate::WorkCycle::Farm, Some(51)),
    ] {
        let (world, report) = restored_in(outdoor_hut(cycle), true, |s| {
            s.workers_waiting = vec![waiting(harvest, Some(9))];
        });
        assert_eq!(report.workers_waiting, 0, "{cycle:?} {harvest:?}");
        assert_eq!(hut_worker(&world).0, crate::Work::Begin);
        assert_eq!(world.reserved_target(HUT_AT), None);
    }
}

/// Construction positions of the hut in these tests (ECO-HQ-4.6), from its
/// flag field's sample.
const POSITIONS: [(i32, i32); 4] = [(0, -4), (3, -6), (-3, -7), (-1, -3)];

/// A site of the hut type (its construction positions [`POSITIONS`]) with
/// its builder, 2 planks lying there, and `walk` his saved fine walk; `unit`
/// his running unit's (c, r), if any.
fn walking_constructor(
    walk: impl FnOnce(&World) -> crate::RestoredFineWalk,
    unit: Option<(u32, u32)>,
) -> (World, RestoreReport) {
    // The work state's first number: 2, the approach's leg back (kept in a
    // unit).
    walking_constructor_with(walk, unit, 2)
}

/// [`walking_constructor`] with the work state's first number `first`
/// (FMT-SAV-3.5a e: at step 13 the approach's leg, Q-1037).
fn walking_constructor_with(
    walk: impl FnOnce(&World) -> crate::RestoredFineWalk,
    unit: Option<(u32, u32)>,
    first: u32,
) -> (World, RestoreReport) {
    const SITE: (u32, u32) = (20, 16);
    let positions = |w: &mut World| {
        let kind = w.building_types.get_mut(&HUT).expect("the hut");
        kind.construction_positions = [POSITIONS; 3];
    };
    let (probe, _) = restored_in(positions, false, |_| {});
    let walk = walk(&probe);
    restored_in(positions, false, |saved| {
        let east_south = crate::grid::Direction::new(1).expect("a direction");
        let flag = crate::grid::neighbour(SITE, east_south, 40, 40).expect("its flag");
        saved.flags.push((P0, flag));
        saved.buildings.push(RestoredBuilding {
            player: P0,
            building_type: HUT,
            at: SITE,
            remaining: Some([0.5, 1.0]),
            levelled: 1.0,
            delivered: vec![(PLANK, 2)],
            soldiers: Vec::new(),
            production_on: true,
            stock: Vec::new(),
            worker: false,
            number: None,
            worker_number: None,
        });
        saved.builders = vec![(SITE, 4242)];
        saved.builder_units = unit.map(|(c, r)| (SITE, c, r)).into_iter().collect();
        saved.builder_walks = vec![(SITE, walk, first)];
    })
}

/// The saved walk of `line` (its points after `from`) with `done` steps
/// walked and `progress` into the next.
fn fine_walk(
    from: (u32, u32),
    line: &[(u32, u32)],
    done: usize,
    progress: f32,
) -> crate::RestoredFineWalk {
    let mut path: Vec<_> = line.iter().rev().copied().collect();
    path.push(from);
    crate::RestoredFineWalk {
        path,
        index: (line.len() - 1 - done) as u32,
        progress,
        rate: 0.09,
    }
}

// FMT-SAV-3.5a (e, 2026-11-09, Q-257): a constructor of a save approaching
// his site (step 13) walks the saved fine path on from its step and progress
// (TRN-MOVE-2.5) to its target and chooses his first unit in the tick it
// ends (ECO-HQ-4.6b step 3): the unit's material is taken and its first
// walk starts where he stands. A walk off the straight line is not taken
// over: his approach starts again.
// spec: FMT-SAV-3.5a, ECO-HQ-4.6b
#[test]
fn a_constructor_of_a_save_goes_on_with_his_approach() {
    const SITE: (u32, u32) = (20, 16);
    let start = crate::grid::field_sample((21, 17));
    let target = crate::grid::field_sample((21, 15));
    let walk_of = |w: &World| {
        let mut walk = crate::movement::LatticeWalk::standing(start);
        walk.to = target;
        let line = walk.steps(&w.terrain);
        fine_walk(start, &line, 2, 0.36)
    };
    let (mut world, report) = walking_constructor(walk_of, None);
    assert_eq!(report.builder_walks, 1);
    let builder = |w: &World| w.site(SITE).and_then(|s| s.builder).expect("its builder");
    let b = builder(&world);
    assert!(!b.in_unit);
    assert!(matches!(
        b.approach,
        crate::construction::Approach::Walking { back: true, .. }
    ));
    let (at, next, progress) = b.walk.position(&world.terrain);
    let mut whole = crate::movement::LatticeWalk::standing(start);
    whole.to = target;
    let steps = whole.steps(&world.terrain);
    assert_eq!((at, next, progress), (steps[1], Some(steps[2]), 0.36));
    let left = b
        .walk
        .ticks_left(&world.terrain, world.settings.settler_speed);
    for _ in 0..left - 1 {
        world.step();
        assert!(!builder(&world).in_unit);
    }
    world.step();
    let b = builder(&world);
    assert!(b.in_unit, "the unit chosen in the tick the walk ends");
    assert_eq!(b.at, Some(target));
    // Off the line: the approach starts again.
    let crooked = |w: &World| {
        let mut walk = walk_of(w);
        walk.path[0].0 += 3;
        walk
    };
    let (world, report) = walking_constructor(crooked, None);
    assert_eq!(report.builder_walks, 0);
    assert_eq!(
        builder(&world).approach,
        crate::construction::Approach::Pending
    );
}

// FMT-SAV-3.5a (e, corrected 2026-11-09 c, Q-1037), ECO-HQ-4.6b step 2
// (T26): a constructor saved on the way out of his approach (leg 1), walking
// from his flag field's sample to the far point 3 lattice steps in direction
// 5, walks the saved walk on to its end, then walks back to the flag field's
// sample, and only then chooses his first unit; saved on the way back (leg
// 2) he chooses where the saved walk ends.
// spec: FMT-SAV-3.5a, ECO-HQ-4.6b (T26)
#[test]
fn t26_a_constructor_saved_on_the_way_out_walks_back_first() {
    const SITE: (u32, u32) = (20, 16);
    let points = |w: &World| {
        let flag = w.terrain.neighbour(SITE, 1).expect("its flag");
        let flag_sample = crate::grid::field_sample(flag);
        let (columns, rows) = w.terrain.sample_size();
        let north_east = crate::grid::Direction::new(5).expect("direction 5");
        let mut far = flag_sample;
        for _ in 0..3 {
            far = crate::grid::neighbour(far, north_east, columns, rows).expect("on the map");
        }
        (flag_sample, far)
    };
    let walk_out = |w: &World| {
        let (flag_sample, far) = points(w);
        // (T26: from a field's sample the far point is that sample + (1, 3).)
        assert_eq!(far, (flag_sample.0 + 1, flag_sample.1 + 3));
        let mut walk = crate::movement::LatticeWalk::standing(flag_sample);
        walk.to = far;
        let line = walk.steps(&w.terrain);
        fine_walk(flag_sample, &line, 1, 0.5)
    };
    let (mut world, report) = walking_constructor_with(walk_out, None, 1);
    assert_eq!(report.builder_walks, 1);
    let (flag_sample, far) = points(&world);
    let builder = |w: &World| w.site(SITE).and_then(|s| s.builder).expect("its builder");
    assert!(matches!(
        builder(&world).approach,
        crate::construction::Approach::Walking { back: false, .. }
    ));
    let speed = world.settings.settler_speed;
    let left = builder(&world).walk.ticks_left(&world.terrain, speed);
    for _ in 0..left {
        world.step();
    }
    // The saved walk has ended at the far point: the walk back is ordered.
    let b = builder(&world);
    assert!(!b.in_unit, "no unit at the far point");
    assert!(matches!(
        b.approach,
        crate::construction::Approach::Walking { back: true, .. }
    ));
    assert_eq!((b.walk.from, b.walk.to), (far, flag_sample));
    let mut ticks = 0;
    while !builder(&world).in_unit {
        world.step();
        ticks += 1;
        assert!(ticks < 200, "he never chose");
    }
    assert_eq!(builder(&world).at, Some(flag_sample));
    // Leg 2, the same walk the other way: he chooses where it ends.
    let walk_back = |w: &World| {
        let (flag_sample, far) = points(w);
        let mut walk = crate::movement::LatticeWalk::standing(far);
        walk.to = flag_sample;
        let line = walk.steps(&w.terrain);
        fine_walk(far, &line, 1, 0.5)
    };
    let (mut world, _) = walking_constructor_with(walk_back, None, 2);
    let left = builder(&world).walk.ticks_left(&world.terrain, speed);
    for _ in 0..left {
        world.step();
    }
    let b = builder(&world);
    assert!(b.in_unit, "the unit chosen in the tick the walk ends");
    assert_eq!(b.at, Some(flag_sample));
}

// ECO-HQ-4.6b step 2 (T26, Q-1037): the flag field is the site's
// direction-1 neighbour and the approach's far point its sample + (1, 3),
// for the four sites of the Fight autosave's step-13 constructors.
// spec: ECO-HQ-4.6b (T26)
#[test]
fn t26_the_approach_points_of_the_saved_sites() {
    let terrain = crate::test_support::flat(64, 64);
    let (columns, rows) = terrain.sample_size();
    let north_east = crate::grid::Direction::new(5).expect("direction 5");
    for (site, flag, sample, far) in [
        ((22, 49), (23, 48), (92, 192), (93, 195)),
        ((21, 50), (21, 49), (86, 196), (87, 199)),
        ((56, 20), (56, 19), (226, 76), (227, 79)),
        ((21, 41), (22, 40), (88, 160), (89, 163)),
    ] {
        assert_eq!(terrain.neighbour(site, 1), Some(flag), "{site:?}");
        assert_eq!(crate::grid::field_sample(flag), sample);
        let mut out = sample;
        for _ in 0..3 {
            out = crate::grid::neighbour(out, north_east, columns, rows).expect("on the map");
        }
        assert_eq!(out, far, "{site:?}");
    }
}

// FMT-SAV-3.5a (e, 2026-11-09, Q-257): a constructor of a save in a running
// unit walking to construction position c - 1 (step 10, r = 55) goes on with
// that walk from the saved step and progress; a saved walk that is not the
// walk to that position is not taken over (it starts from the position
// before, Q-1033).
// spec: FMT-SAV-3.5a, ECO-HQ-4.6b
#[test]
fn a_constructor_of_a_save_goes_on_with_his_walk_to_a_position() {
    const SITE: (u32, u32) = (20, 16);
    let east_south = crate::grid::Direction::new(1).expect("a direction");
    let flag = crate::grid::neighbour(SITE, east_south, 40, 40).expect("its flag");
    let (c, r) = crate::grid::field_sample(flag);
    let samples = POSITIONS.map(|(x, y)| ((c as i32 + x) as u32, (r as i32 + y) as u32));
    let walk_of = |w: &World| {
        let mut walk = crate::movement::LatticeWalk::standing(samples[1]);
        walk.to = samples[2];
        let line = walk.steps(&w.terrain);
        assert!(line.len() >= 2, "{line:?}");
        fine_walk(samples[1], &line, 1, 0.5)
    };
    let (world, report) = walking_constructor(walk_of, Some((3, 55)));
    assert_eq!((report.builder_units, report.builder_walks), (1, 1));
    let b = world
        .site(SITE)
        .and_then(|s| s.builder)
        .expect("its builder");
    assert_eq!(world.construction_samples(SITE), samples);
    assert!(b.in_unit);
    assert_eq!(
        (b.walk.done, f32::from_bits(b.walk.progress_bits)),
        (1, 0.5)
    );
    // Saved as the walk to another position: not taken over.
    let (world, report) = walking_constructor(walk_of, Some((2, 55)));
    assert_eq!((report.builder_units, report.builder_walks), (1, 0));
    let b = world
        .site(SITE)
        .and_then(|s| s.builder)
        .expect("its builder");
    assert_eq!(b.walk.done, 0);
    // The walk to position 0 of a later unit starts on position 3 (step 5).
    let to_first = |w: &World| {
        let mut walk = crate::movement::LatticeWalk::standing(samples[3]);
        walk.to = samples[0];
        fine_walk(samples[3], &walk.steps(&w.terrain), 0, 0.27)
    };
    let (world, report) = walking_constructor(to_first, Some((1, 55)));
    assert_eq!(report.builder_walks, 1);
    let b = world
        .site(SITE)
        .and_then(|s| s.builder)
        .expect("its builder");
    assert_eq!(
        (b.walk.from, b.walk.to, b.at),
        (samples[3], samples[0], Some(samples[3]))
    );
}

// FMT-SAV-3.5a (b, 2026-11-09 b): a worker producing inside whose building
// has a door field of its own (ECO-WORK-3.8a) stands on that field in the
// save; he too goes on with his work action (the 2023 Coastal Storm save's
// donkey breeder).
// spec: FMT-SAV-3.5a, ECO-WORK-3.8a
#[test]
fn a_worker_producing_on_his_door_field_is_inside() {
    let door = (17, 10);
    let inside = |w: &mut World| {
        let kind = w.building_types.get_mut(&HUT).expect("the hut");
        kind.cycle = crate::WorkCycle::Workshop;
        kind.production_ticks = 333;
        kind.doors = [Some(0); 3];
    };
    let (world, report) = restored_in(inside, true, |saved| {
        saved.workers_outdoors = vec![crate::RestoredOutdoor {
            at: HUT_AT,
            field: door,
            spot: door,
            state: 33,
            left: Some(40),
            walk: None,
            rough: None,
            animal: None,
            leg: None,
        }];
    });
    assert_eq!(world.door_field(HUT_AT), door);
    assert_eq!((report.workers_working, report.workers_at_spot), (1, 0));
    let (work, timer, _, _) = hut_worker(&world);
    assert_eq!((work, timer), (crate::Work::Working, 40));
}
