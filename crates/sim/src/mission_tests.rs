//! Tests of `specs/CAM-campaign-missions.md` CAM-1.3, 1.4, 2 and 3 on the
//! synthetic military rules.

use crate::grid::Point;
use crate::military_tests::{game, rules, BARRACKS, CASTLE, COIN, HELPER, HUT, P1, RANK};
use crate::mission::{target, LossReason, Mission, MissionEvent, Outcome, Trigger, TriggerKind};
use crate::test_support::*;
use crate::{Cheat, Command, PlayerId, Rejection, World};

const HQ0: Point = (10, 10);

fn trigger(kind: TriggerKind, field: Point, name: &str) -> Trigger {
    Trigger {
        kind,
        field,
        player: P0,
        name: name.to_owned(),
        active: false,
        timer: 0,
    }
}

fn run(world: &mut World, ticks: u32) {
    for _ in 0..ticks {
        world.step();
    }
}

fn apply(world: &mut World, command: Command) -> Result<(), Rejection> {
    world.submit(command);
    world.step();
    world.last_results().last().unwrap().outcome
}

/// One player alone, its land around (10, 10).
fn alone(target: u8, triggers: Vec<Trigger>) -> World {
    let mut world = game(40, 40, &[(P0, HQ0)]);
    world.set_mission(Mission::new(target, triggers));
    world
}

// spec: CAM-2.1, CAM-2.2, CAM-2.3 (T2)
#[test]
fn territory_trigger_fires_at_the_five_second_evaluation() {
    // T2: the field is the player's land; the evaluation comes after 5 s.
    let mut world = alone(
        target::ENDLESS,
        vec![trigger(TriggerKind::Territory, (12, 10), "Region_K")],
    );
    run(&mut world, 49);
    assert_eq!(world.active_trigger(), None);
    run(&mut world, 1);
    assert_eq!(world.active_trigger(), Some("Region_K"));
    // T10: not cleared while the script keeps it.
    run(&mut world, 3);
    assert_eq!(world.active_trigger(), Some("Region_K"));
    assert_eq!(
        apply(
            &mut world,
            Command::RemoveTrigger {
                name: "Region_K".into()
            }
        ),
        Ok(())
    );
    assert_eq!(world.active_trigger(), None);
    assert_eq!(
        apply(
            &mut world,
            Command::RemoveTrigger {
                name: "Region_K".into()
            }
        ),
        Err(Rejection::NoSuchTrigger)
    );
}

// spec: CAM-2.1, CAM-2.3
#[test]
fn trigger_outside_the_land_stays_off_and_list_order_decides() {
    let mut world = alone(
        target::ENDLESS,
        vec![
            trigger(TriggerKind::Territory, (35, 35), "far"),
            trigger(TriggerKind::Explored, (11, 11), "seen"),
            trigger(TriggerKind::Territory, (10, 12), "near"),
        ],
    );
    run(&mut world, 50);
    // CAM-2.3: the first set flag in list order.
    assert_eq!(world.active_trigger(), Some("seen"));
    let m = world.mission().unwrap();
    assert!(!m.triggers[0].active);
    assert!(m.triggers[2].active);
}

// spec: CAM-3.1, CAM-3.3 (T3)
#[test]
fn victory_region_completes_the_map_with_value_4() {
    // T3
    let mut world = alone(
        target::PORTAL,
        vec![trigger(TriggerKind::Victory, (11, 10), "RegionVictory__0")],
    );
    run(&mut world, 50);
    assert_eq!(world.mission().unwrap().completed, Some(4));
    assert!(world
        .mission_events()
        .contains(&MissionEvent::Completed { value: 4 }));
    run(&mut world, 1);
    assert!(world.mission_events().is_empty());
}

// spec: CAM-3.3, CAM-3.4 (T4)
#[test]
fn conquering_three_quarters_wins_and_the_rest_lose() {
    // T4: P0's land covers more than 74 % of a small map; P1 elsewhere.
    let mut world = game(14, 14, &[(P0, (6, 6)), (P1, (60, 60))]);
    world.set_mission(Mission::new(target::CONQUER, Vec::new()));
    world.step();
    let owned = world.owners.iter().filter(|o| **o == Some(P0)).count();
    assert!(owned * 100 > 74 * 196, "{owned} fields");
    assert!(matches!(world.outcome(P0), Some(Outcome::Won { .. })));
    // P1 never had a headquarters: lost first by CAM-3.2.
    assert!(matches!(
        world.outcome(P1),
        Some(Outcome::Lost {
            reason: LossReason::CastleDestroyed,
            ..
        })
    ));
    assert_eq!(world.mission().unwrap().completed, Some(target::CONQUER));
}

// spec: CAM-3.3, CAM-3.4 (T5)
#[test]
fn the_other_team_loses_with_mission_not_completed() {
    // T5: 100 coins in the team's depots.
    let mut world = game(60, 30, &[(P0, (10, 10)), (P1, (45, 15))]);
    world.set_mission(Mission::new(target::COINS, Vec::new()));
    let add = |amount| Command::AddGoods {
        player: P0,
        good: COIN,
        amount,
    };
    assert_eq!(apply(&mut world, add(99)), Ok(()));
    assert_eq!(world.outcome(P0), None);
    assert_eq!(apply(&mut world, add(1)), Ok(()));
    assert!(matches!(world.outcome(P0), Some(Outcome::Won { .. })));
    assert_eq!(
        world.outcome(P1),
        Some(Outcome::Lost {
            tick: world.tick() - 1,
            reason: LossReason::MissionNotCompleted
        })
    );
    assert!(world
        .mission_events()
        .contains(&MissionEvent::Won { player: P0 }));
}

// spec: CAM-3.2, CAM-3.3 (T6, T7)
#[test]
fn destroying_all_enemies_wins_and_a_lost_castle_loses() {
    // T6, T7: P1's headquarters goes; P1 is lost, then P0 has won.
    let mut world = game(60, 30, &[(P0, (10, 10)), (P1, (45, 15))]);
    world.set_mission(Mission::new(target::DESTROY_ENEMIES, Vec::new()));
    world.step();
    assert_eq!(world.outcome(P0), None);
    let index = world.terrain.index((45, 15));
    world.buildings[index] = None;
    world.step();
    assert!(matches!(
        world.outcome(P1),
        Some(Outcome::Lost {
            reason: LossReason::CastleDestroyed,
            ..
        })
    ));
    assert!(matches!(world.outcome(P0), Some(Outcome::Won { .. })));
    assert_eq!(
        world.mission().unwrap().completed,
        Some(target::DESTROY_ENEMIES)
    );
}

// spec: CAM-3.1
#[test]
fn no_mission_no_results() {
    let mut world = game(60, 30, &[(P0, (10, 10)), (P1, (200, 200))]);
    run(&mut world, 3);
    assert_eq!(world.outcome(P1), None);
    assert!(world.mission_events().is_empty());
}

// spec: CAM-1.3 (T1)
#[test]
fn locked_buildings_cannot_be_placed() {
    // T1 (the locks): the type is refused for that player only.
    let mut world = alone(target::ENDLESS, Vec::new());
    let lock = Command::LockBuilding {
        player: P0,
        building: HUT,
    };
    assert_eq!(apply(&mut world, lock), Ok(()));
    assert!(world.is_locked(P0, HUT));
    assert!(!world.is_locked(P1, HUT));
    let place = Command::PlaceBuilding {
        player: P0,
        building: HUT,
        at: (14, 10),
    };
    assert_eq!(apply(&mut world, place), Err(Rejection::BuildingLocked));
}

// spec: CAM-1.3 (T1)
#[test]
fn script_stock_commands() {
    // T1: 40 soldiers of rank 0 into the headquarters; clearResources.
    let mut world = alone(target::ENDLESS, Vec::new());
    let soldiers = world.stock_soldiers(HQ0)[0];
    let add = |good| Command::AddGoods {
        player: P0,
        good,
        amount: 40,
    };
    assert_eq!(apply(&mut world, add(RANK[0])), Ok(()));
    // One may have left for the step's own needs.
    assert!(world.stock_soldiers(HQ0)[0] + 1 >= soldiers + 40);
    assert_eq!(apply(&mut world, add(COIN)), Ok(()));
    assert_eq!(world.warehouse(HQ0).unwrap().count(COIN), 40);
    assert_eq!(
        apply(&mut world, Command::ClearGoods { player: P0 }),
        Ok(())
    );
    assert!(world.stock(HQ0).unwrap().is_empty());
    let unknown = Command::AddGoods {
        player: P0,
        good: 9999,
        amount: 1,
    };
    assert_eq!(apply(&mut world, unknown), Err(Rejection::UnknownGood));
}

// spec: CAM-1.3
#[test]
fn created_buildings_are_finished_and_manned() {
    let mut world = alone(target::ENDLESS, Vec::new());
    let barracks = Command::CreateBuilding {
        player: P0,
        building: BARRACKS,
        at: (16, 12),
        settler: Some(RANK[3]),
    };
    assert_eq!(apply(&mut world, barracks), Ok(()));
    let b = *world.building_at((16, 12)).unwrap();
    assert!(!b.under_construction);
    assert_eq!(world.flags().count(), 2);
    let garrison = world.garrison((16, 12));
    assert_eq!(garrison.len(), 1);
    assert_eq!(
        (garrison[0].rank, garrison[0].health, garrison[0].present),
        (3, 6, true)
    );
    let hut = Command::CreateBuilding {
        player: P0,
        building: HUT,
        at: (6, 14),
        settler: Some(HELPER),
    };
    assert_eq!(apply(&mut world, hut), Ok(()));
    assert_eq!(world.building_at((6, 14)).unwrap().workers, 1);
    let again = Command::CreateBuilding {
        player: P0,
        building: CASTLE,
        at: (6, 14),
        settler: None,
    };
    assert_eq!(apply(&mut world, again), Err(Rejection::TooCloseToBuilding));
}

// spec: CAM-3.7
#[test]
fn cheats() {
    let mut world = alone(target::COINS, Vec::new());
    let cheat = |cheat| Command::Cheat { player: P0, cheat };
    assert_eq!(apply(&mut world, cheat(Cheat::ShowMeTheWorld)), Ok(()));
    assert!(world.is_explored(P0, (39, 39)));
    let coins = world
        .stock(HQ0)
        .unwrap()
        .iter()
        .find(|g| g.0 == COIN)
        .map_or(0, |g| g.1);
    assert_eq!(apply(&mut world, cheat(Cheat::GiveMeSomeMore)), Ok(()));
    let after = world
        .stock(HQ0)
        .unwrap()
        .iter()
        .find(|g| g.0 == COIN)
        .unwrap()
        .1;
    assert_eq!(after, coins + 10);
    assert_eq!(apply(&mut world, cheat(Cheat::INeedSomeHelp)), Ok(()));
    assert_eq!(world.mission().unwrap().completed, Some(target::COINS));
    // The interface hears of it as of any completion (seen in the harness: the
    // event was cleared before the step's end).
    assert_eq!(
        world.mission_events(),
        [MissionEvent::Completed {
            value: target::COINS
        }]
    );
    assert_eq!(world.outcome(PlayerId(0)), None);
    world.step();
    assert_eq!(world.mission_events(), []);
}

// spec: none (determinism of missions: same commands, same world)
#[test]
fn missions_are_deterministic() {
    let make = || {
        let mut w = game(60, 30, &[(P0, (10, 10)), (P1, (45, 15))]);
        w.set_mission(Mission::new(
            target::CONQUER,
            vec![trigger(TriggerKind::Territory, (12, 10), "a")],
        ));
        run(&mut w, 120);
        w.state_hash()
    };
    assert_eq!(make(), make());
    let _ = rules();
}

// CAM-3.1b (Q-1003, T15): at a campaign game's set-up a victory region's 19
// fields within distance 2 start explored for its player; an explored or a
// territory trigger's field does not, nor any field beyond the ring 2.
// spec: CAM-3.1b (T15)
#[test]
fn a_victory_regions_surroundings_start_explored() {
    let gate = (30, 30);
    let mut world = alone(
        target::PORTAL,
        vec![
            trigger(TriggerKind::Explored, (30, 20), "seen"),
            trigger(TriggerKind::Territory, (20, 30), "land"),
            trigger(TriggerKind::Victory, gate, "RegionVictory__0"),
        ],
    );
    let explored = |w: &World| -> Vec<Point> {
        (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|&f| w.is_explored(P0, f) && crate::grid::distance(f, HQ0) > 12)
            .collect()
    };
    assert!(explored(&world).is_empty());
    world.explore_victory_regions();
    let patch = explored(&world);
    assert_eq!(patch.len(), 19);
    assert!(patch.iter().all(|&f| crate::grid::distance(f, gate) <= 2));
    assert!(!world.is_explored(P0, (30, 20)) && !world.is_explored(P0, (20, 30)));
    assert!(!world.is_explored(P1, gate));
}

/// The ends of every road, either way round.
fn road_ends(world: &World) -> Vec<(Point, Point)> {
    world
        .roads()
        .map(|r| (r.path[0], *r.path.last().unwrap()))
        .collect()
}

fn has_road(ends: &[(Point, Point)], a: Point, b: Point) -> bool {
    ends.contains(&(a, b)) || ends.contains(&(b, a))
}

/// CAM-1.3b T16: `createStreet` between two flags builds the road; one
/// ending on a field of another road without a flag places a flag there,
/// which splits that road, and builds its road to it (tutorial03's two
/// calls, on the synthetic map).
// spec: CAM-1.3b (T16)
#[test]
fn create_street_joins_flags_and_splits_a_road_it_ends_on() {
    let mut world = alone(target::ENDLESS, Vec::new());
    let a = world.flags().next().expect("the headquarters' flag").0;
    let (c, d) = ((a.0 + 4, a.1 - 3), (a.0 + 4, a.1 + 3));
    for at in [c, d] {
        assert_eq!(
            apply(&mut world, Command::PlaceFlag { player: P0, at }),
            Ok(())
        );
    }
    let first = Command::CreateStreet {
        player: P0,
        from: c,
        to: d,
    };
    assert_eq!(apply(&mut world, first), Ok(()));
    assert!(has_road(&road_ends(&world), c, d));
    let x = world
        .roads()
        .last()
        .unwrap()
        .path
        .iter()
        .copied()
        .find(|&f| crate::grid::distance(f, c) >= 2 && crate::grid::distance(f, d) >= 2)
        .expect("a middle field");
    assert_eq!(world.flag_at(x), None);
    let second = Command::CreateStreet {
        player: P0,
        from: a,
        to: x,
    };
    assert_eq!(apply(&mut world, second), Ok(()));
    assert_eq!(world.flag_at(x), Some(P0));
    let ends = road_ends(&world);
    assert_eq!(ends.len(), 3, "{ends:?}");
    for (p, q) in [(c, x), (x, d), (a, x)] {
        assert!(has_road(&ends, p, q), "no road {p:?} - {q:?} in {ends:?}");
    }
    // The player's roads, on land (each asks for its carrier as the
    // player's own, through the same builder as the road command).
    assert!(world.roads().all(|r| r.owner == P0 && !r.water));
}

/// CAM-1.3b T17: an end on a free field that is on no road gets neither a
/// flag nor a road; and without the player's flag at the start nothing is
/// built at all.
// spec: CAM-1.3b (T17)
#[test]
fn create_street_to_a_free_field_or_from_no_flag_builds_nothing() {
    let mut world = alone(target::ENDLESS, Vec::new());
    let a = world.flags().next().expect("the headquarters' flag").0;
    let free = (a.0 + 4, a.1);
    assert_eq!(world.flag_at(free), None);
    assert!(world.road_at(free).is_none());
    let to_free = Command::CreateStreet {
        player: P0,
        from: a,
        to: free,
    };
    assert!(apply(&mut world, to_free).is_err());
    assert_eq!(world.flag_at(free), None);
    assert_eq!(world.roads().count(), 0);
    // From a field without a flag to the headquarters' flag: no flag is
    // placed at the start, no road built.
    let from_free = Command::CreateStreet {
        player: P0,
        from: free,
        to: a,
    };
    assert_eq!(apply(&mut world, from_free), Err(Rejection::NoFlagAtStart));
    assert_eq!(world.flag_at(free), None);
    assert_eq!(world.roads().count(), 0);
}
