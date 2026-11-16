//! Test cases of `specs/ECO-specialists.md` (T1 to T4).

use crate::grid::{self, Point};
use crate::map::state;
use crate::requests::tests::{self as req, apply_all, road_cmd, HQ, HQ_FLAG};
use crate::specialists::{SignRule, SpecialistRules, WORK_TICKS};
use crate::test_support::*;
use crate::{Command, FieldResource, GameSetup, SettlerJob, SpecialistStep, Tick, World};

const GEOLOGIST: u32 = 20;
const SCOUT: u32 = 21;
const COAL: u32 = 30;
const WATER: u32 = 31;
const NOTHING_SIGN: u32 = 900;
const WATER_SIGN: u32 = 901;
const COAL_SMALL: u32 = 902;
const COAL_MEDIUM: u32 = 903;
const COAL_LARGE: u32 = 904;
const FLAG: Point = (14, 9);

fn game(stock: &[(u32, u32)]) -> World {
    let mut rules = req::rules(stock);
    let sign = |good, threshold, doodad| SignRule {
        good,
        threshold,
        doodad,
        life_time: Some(5000),
    };
    rules.specialists = SpecialistRules {
        geologist: Some(GEOLOGIST),
        scout: Some(SCOUT),
        water: Some(WATER),
        signs: vec![
            sign(None, 0, NOTHING_SIGN),
            sign(Some(WATER), 1, WATER_SIGN),
            sign(Some(COAL), 1, COAL_SMALL),
            sign(Some(COAL), 3, COAL_MEDIUM),
            sign(Some(COAL), 6, COAL_LARGE),
        ],
        found: vec![(COAL, "Found Coal!"), (WATER, "Found Water!")],
    };
    let setup = GameSetup {
        players: vec![(P0, HQ)],
        start_resources: 0,
        local_player: P0,
        ..Default::default()
    };
    let mut world = World::new_game(9, flat(40, 40), rules, &setup);
    apply_all(
        &mut world,
        vec![Command::PlaceFlag {
            player: P0,
            at: FLAG,
        }],
    );
    apply_all(&mut world, vec![road_cmd(HQ_FLAG, FLAG)]);
    world
}

/// ECO-SPC-2.2: mining ground (ForMining, not ForBuilding) within 6 fields
/// of the flag.
fn mining_ground(world: &mut World) {
    for f in grid::area(FLAG, 6, 40, 40) {
        world.set_bits(f, state::FOR_BUILDING, false);
        world.set_bits(f, state::FOR_MINING, true);
    }
}

fn stock(specialists: u32) -> Vec<(u32, u32)> {
    let mut stock = req::full_stock();
    stock.push((GEOLOGIST, specialists));
    stock.push((SCOUT, specialists));
    stock
}

/// The specialist's step each tick, with the tick, while he is one.
fn trace(world: &mut World, ticks: u32) -> Vec<(Tick, Point, SpecialistStep)> {
    let mut seen = Vec::new();
    for _ in 0..ticks {
        world.step();
        for (_, s) in world.settlers() {
            if let SettlerJob::Specialist { step, .. } = s.job {
                seen.push((world.tick(), s.walker.at, step));
            }
        }
    }
    seen
}

fn spots(trace: &[(Tick, Point, SpecialistStep)]) -> Vec<(Point, u32)> {
    let mut out: Vec<(Point, u32)> = Vec::new();
    let mut last = None;
    for &(_, _, step) in trace {
        if let SpecialistStep::Work { spot } = step {
            match out.last_mut() {
                Some((s, n)) if *s == spot && last == Some(step) => *n += 1,
                _ => out.push((spot, 1)),
            }
        }
        last = Some(step);
    }
    out
}

// ECO-SPC T1, 2.2, 2.3, 3: ten spots within 5 fields, 80 ticks each, a
// sign on each by what the field holds, a message per find, then home.
#[test]
fn t1_geologist_visits_ten_spots() {
    let mut world = game(&stock(1));
    mining_ground(&mut world);
    // Coal on the mining ground around the flag.
    for (i, f) in grid::area(FLAG, 2, 40, 40).into_iter().enumerate() {
        let amount = [1u8, 4, 7][i % 3];
        world.set_resource(f, Some(FieldResource { good: COAL, amount }));
    }
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    let seen = trace(&mut world, 3000);
    let visited = spots(&seen);
    assert_eq!(visited.len(), 10, "{visited:?}");
    for &(spot, ticks) in &visited {
        assert_eq!(ticks, WORK_TICKS, "ECO-SPC-2.3: 80 ticks at {spot:?}");
        assert!(grid::distance(FLAG, spot) <= 5);
        let sign = world.sign(spot).expect("a sign");
        let expected = if world.terrain.has(spot, state::FOR_BUILDING) {
            WATER_SIGN
        } else {
            match world.resource_at(spot) {
                Some(FieldResource { amount: 1..=2, .. }) => COAL_SMALL,
                Some(FieldResource { amount: 3..=5, .. }) => COAL_MEDIUM,
                Some(_) => COAL_LARGE,
                None => NOTHING_SIGN,
            }
        };
        assert_eq!(sign.doodad, expected, "{spot:?}");
    }
    let coal_finds = visited
        .iter()
        .filter(|(s, _)| world.sign(*s).is_some_and(|g| g.good == Some(COAL)))
        .count();
    let messages = world
        .messages()
        .iter()
        .filter(|m| m.text == "Found Coal!")
        .count();
    assert!(coal_finds > 0 && messages > 0);
    // ECO-SPC-2.5: he is back in the headquarters' stock.
    assert!(world.settlers().all(|(_, s)| s.profession != GEOLOGIST));
    assert_eq!(world.warehouse(HQ).unwrap().count(GEOLOGIST), 1);
}

// ECO-SPC T5, 2.2: on all-diggable ground the spots are the nearest
// diggable fields in frontier order: outward ring by ring, each a field
// without a sign yet.
#[test]
fn t5_geologist_works_outward() {
    let mut world = game(&stock(1));
    mining_ground(&mut world);
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    let visited = spots(&trace(&mut world, 3000));
    assert_eq!(visited.len(), 10, "{visited:?}");
    let distances: Vec<u32> = visited
        .iter()
        .map(|&(s, _)| grid::distance(FLAG, s))
        .collect();
    assert!(distances.windows(2).all(|d| d[0] <= d[1]), "{distances:?}");
    assert!(
        distances.iter().all(|&d| (1..=2).contains(&d)),
        "{distances:?}"
    );
    let mut fields: Vec<Point> = visited.iter().map(|&(s, _)| s).collect();
    fields.dedup();
    assert_eq!(fields.len(), 10);
    // A field without ForMining is never dug.
    let mut world = game(&stock(1));
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    assert!(spots(&trace(&mut world, 1000)).is_empty());
}

// ECO-SPC-3.2: a spot that holds a sign keeps it; LUA-PROP-9: a sign goes
// after its doodad's lifetime.
#[test]
fn signs_stay_and_expire() {
    let mut world = game(&stock(1));
    let spot = (15, 11);
    world.set_bits(spot, state::FOR_BUILDING, false);
    world.geologist_reports(P0, spot);
    // ECO-SPC T6, 3.1: no entry: the empty sign, no message.
    assert_eq!(world.sign(spot).unwrap().doodad, NOTHING_SIGN);
    assert!(world.messages().is_empty());
    world.set_resource(
        spot,
        Some(FieldResource {
            good: COAL,
            amount: 7,
        }),
    );
    world.geologist_reports(P0, spot);
    assert_eq!(world.sign(spot).unwrap().doodad, NOTHING_SIGN);
    let gone = world.sign(spot).unwrap().gone_at.unwrap();
    while world.tick() < gone {
        world.step();
    }
    world.step();
    assert!(world.sign(spot).is_none());
    // ECO-PROD-2.5b: buildable ground reports water, amount 1.
    let meadow = (16, 11);
    world.set_bits(meadow, state::FOR_BUILDING, true);
    world.set_resource(
        meadow,
        Some(FieldResource {
            good: COAL,
            amount: 7,
        }),
    );
    world.geologist_reports(P0, meadow);
    let sign = world.sign(meadow).unwrap();
    assert_eq!(
        (sign.doodad, sign.good, sign.amount),
        (WATER_SIGN, Some(WATER), 1)
    );
    assert!(world.messages().iter().any(|m| m.text == "Found Water!"));
}

// ECO-SPC T2: no geologist in any depot: the request waits.
// spec: ECO-SPC-1.1, ECO-REQ-3.8 (T2)
#[test]
fn t2_request_waits_without_a_geologist() {
    let mut world = game(&stock(0));
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    for _ in 0..300 {
        world.step();
    }
    assert!(world.settlers().all(|(_, s)| s.profession != GEOLOGIST));
    assert_eq!(world.requests(P0).len(), 1);
}

// ECO-SPC T3, 2.2, 4.1: the scout's spots are unexplored fields within 10
// of the flag; after each the rectangle of radius 4 is explored. He stops
// when no unexplored field is left within 10 (the map's edge is near).
#[test]
fn t3_scout_explores() {
    let mut world = game(&stock(1));
    world.submit(Command::OrderScout {
        player: P0,
        at: FLAG,
    });
    let mut before_work = Vec::new();
    let mut last = None;
    let mut explored_after = Vec::new();
    for _ in 0..4000 {
        world.step();
        let step = world.settlers().find_map(|(_, s)| match s.job {
            SettlerJob::Specialist { step, .. } => Some(step),
            _ => None,
        });
        match (last, step) {
            (Some(SpecialistStep::ToSpot { spot }), Some(SpecialistStep::Work { .. })) => {
                before_work.push((spot, world.is_explored(P0, spot)));
            }
            (Some(SpecialistStep::Work { spot }), s) if s != last => {
                explored_after.push(spot);
            }
            _ => {}
        }
        last = step;
    }
    assert!((1..=10).contains(&before_work.len()), "{before_work:?}");
    for &(spot, explored) in &before_work {
        assert!(grid::distance(FLAG, spot) <= 10);
        assert!(!explored, "{spot:?} was unexplored when chosen");
    }
    if before_work.len() < 10 {
        assert!(world.specialist_candidates(P0, FLAG, SCOUT).is_empty());
    }
    for spot in explored_after {
        for y in spot.1.saturating_sub(4)..=(spot.1 + 4).min(39) {
            for x in spot.0.saturating_sub(4)..=(spot.0 + 4).min(39) {
                assert!(
                    world.is_explored(P0, (x, y)),
                    "{:?} around {spot:?}",
                    (x, y)
                );
            }
        }
    }
    assert_eq!(world.warehouse(HQ).unwrap().count(SCOUT), 1);
}

// ECO-SPC T4, 2.4: the flag is destroyed while he walks between spots: he
// goes home at once.
#[test]
fn t4_flag_destroyed_sends_him_home() {
    let mut world = game(&stock(1));
    mining_ground(&mut world);
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    let mut destroyed = None;
    for _ in 0..2000 {
        world.step();
        let step = world.settlers().find_map(|(_, s)| match s.job {
            SettlerJob::Specialist { step, .. } => Some(step),
            _ => None,
        });
        if destroyed.is_none() && matches!(step, Some(SpecialistStep::ToSpot { .. })) {
            world.submit(Command::RemoveFlag {
                player: P0,
                at: FLAG,
            });
            destroyed = Some(world.tick());
        } else if let Some(t) = destroyed {
            if world.tick() == t + 2 {
                assert!(
                    !matches!(
                        step,
                        Some(SpecialistStep::ToSpot { .. } | SpecialistStep::Work { .. })
                    ),
                    "{step:?}"
                );
            }
        }
    }
    assert!(destroyed.is_some());
    assert!(world.signs().count() <= 1);
    assert!(world.settlers().all(|(_, s)| s.profession != GEOLOGIST));
    assert_eq!(world.warehouse(HQ).unwrap().count(GEOLOGIST), 1);
}

// ECO-SPC-1.1, 1.2: an order needs the player's flag; two orders are two
// requests.
#[test]
fn orders_need_a_flag_and_queue_up() {
    let mut world = game(&stock(0));
    world.submit(Command::OrderGeologist {
        player: P0,
        at: (20, 20),
    });
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    world.submit(Command::OrderGeologist {
        player: P0,
        at: FLAG,
    });
    world.step();
    let outcomes: Vec<_> = world.last_results().iter().map(|r| r.outcome).collect();
    assert_eq!(
        outcomes,
        vec![Err(crate::Rejection::NotAFlag), Ok(()), Ok(())]
    );
    assert_eq!(world.requests(P0).len(), 2);
}

// ECO-SPC T7, 2.2a: a spot another specialist walks to or works at is
// marked: two geologists at one flag never choose the same field while the
// first is on his way to it or working there.
// spec: ECO-SPC-2.2a (ECO-SPC T7), ECO-SPC-2.2
#[test]
fn t7_two_geologists_never_share_a_spot() {
    let mut world = game(&stock(2));
    mining_ground(&mut world);
    for _ in 0..2 {
        world.submit(Command::OrderGeologist {
            player: P0,
            at: FLAG,
        });
    }
    let mut both = false;
    for _ in 0..4000 {
        world.step();
        let spots: Vec<Point> = world
            .settlers()
            .filter_map(|(_, s)| match s.job {
                SettlerJob::Specialist {
                    step: SpecialistStep::ToSpot { spot } | SpecialistStep::Work { spot },
                    ..
                } => Some(spot),
                _ => None,
            })
            .collect();
        if spots.len() == 2 {
            both = true;
            assert_ne!(spots[0], spots[1], "tick {}", world.tick());
        }
    }
    assert!(both, "the two never worked at once");
}

// ECO-SPC T8, 2.2b: a geologist at a flag with no diggable field within
// his radius goes home at once from the flag, no spot, no sign; one whose
// candidates run out after a spot goes home from that spot (TRN-FREE-6.4).
// spec: ECO-SPC-2.2b (ECO-SPC T8), ECO-SPC-2.5, TRN-FREE-6.4
#[test]
fn t8_no_spot_sends_him_home_at_once() {
    for one_field in [false, true] {
        let mut world = game(&stock(1));
        let spot = (16, 9);
        if one_field {
            world.set_bits(spot, state::FOR_BUILDING, false);
            world.set_bits(spot, state::FOR_MINING, true);
        }
        world.submit(Command::OrderGeologist {
            player: P0,
            at: FLAG,
        });
        let mut reached = false;
        let mut returned = false;
        for _ in 0..2000 {
            world.step();
            for (_, s) in world.settlers() {
                if s.profession != GEOLOGIST {
                    continue;
                }
                reached |= s.walker.at == FLAG;
                if let SettlerJob::Specialist {
                    step: SpecialistStep::Return { .. },
                    ..
                } = s.job
                {
                    returned = true;
                }
            }
        }
        assert!(reached, "he reached the flag");
        // At the flag with no spot: home over the roads at once, no walk
        // of a specialist's at all. After a spot, off the roads, going home
        // is TRN-FREE-6.4 step 3's free walk to the nearest flag of his
        // home's network (here the order's flag too), not told apart here.
        assert!(one_field || !returned, "no walk back to the order's flag");
        assert_eq!(world.signs().count(), usize::from(one_field));
        assert_eq!(
            world.warehouse(HQ).unwrap().count(GEOLOGIST),
            1,
            "home again ({one_field})"
        );
    }
}
