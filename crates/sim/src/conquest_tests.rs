//! Tests of `specs/MIL-conquest.md`.

use crate::combat_tests::{attack, fill, privates, scene, TARGET};
use crate::conquest::{catapult_misses, turn_steps};
use crate::grid::{self, Point};
use crate::map::state;
use crate::military_tests::*;
use crate::movement::{self, Walker};
use crate::test_support::{apply, P0};
use crate::{AttackRecord, AttackStep, CatapultPhase, DefendStep, PlayerId, Settler, SettlerJob};
use crate::{BuildingSize, World, SOLDIER_RANKS};
use crate::{Command, Rejection};

fn message_count(world: &World, player: PlayerId, text: &str) -> usize {
    world
        .messages()
        .iter()
        .filter(|m| m.player == player && m.text == text)
        .count()
}

/// The soldiers of the building on `at` are dead.
fn kill_defenders(world: &mut World, at: Point) {
    let ids = std::mem::take(&mut world.military.get_mut(&at).unwrap().soldiers);
    for id in ids {
        world.settlers[id] = None;
    }
}

/// An attacker of player 0 waiting on `field` for the target on `TARGET`.
fn waiting_attacker(world: &mut World, home: Point, field: Point) -> usize {
    let home = world.target_of(home).unwrap();
    let target = world.target_of(TARGET).unwrap();
    let id = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(field),
        job: SettlerJob::Attack {
            home,
            health: 3,
            step: AttackStep::Waiting,
        },
        timer: 0,
        carrying: None,
    });
    world.attackers.push(AttackRecord {
        soldier: id,
        target,
        counter: 0,
        field: Some(field),
        opponent: None,
        hit: false,
        offset: None,
    });
    id
}

// T1: the interceptor and the soldier inside both keep the building.
// spec: MIL-CON-1.1 (T1)
#[test]
fn con_t1_soldiers_out_still_defend() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 3), (0, 3)]);
    let target = world.target_of(TARGET).unwrap();
    let ids = world.military_at(TARGET).unwrap().soldiers.clone();
    assert!(!world.can_take(target, P0));
    world.settlers[ids[1]].as_mut().unwrap().job = SettlerJob::Defend {
        home: target,
        health: 3,
        step: DefendStep::Waiting,
        opponent: None,
        intercept: true,
        defends: Some(target),
    };
    // The one inside is an opponent still.
    assert!(!world.can_take(target, P0));
    world.settlers[ids[0]] = None;
    world
        .military
        .get_mut(&TARGET)
        .unwrap()
        .soldiers
        .retain(|&s| s != ids[0]);
    // The interceptor counts although nobody is inside.
    assert!(!world.can_take(target, P0));
    world.settlers[ids[1]] = None;
    world.military.get_mut(&TARGET).unwrap().soldiers.clear();
    assert!(world.can_take(target, P0));
}

// T2: a soldier still walking in does not count.
// spec: MIL-CON-1.1, MIL-SOL-2.4 (T2)
#[test]
fn con_t2_walking_in_does_not_count() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 3)]);
    kill_defenders(&mut world, TARGET);
    let target = world.target_of(TARGET).unwrap();
    let walker = world.add_settler(Settler {
        number: 0,
        player: P1,
        profession: RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new((60, 20)),
        job: SettlerJob::ToMilitary { target, health: 3 },
        timer: 0,
        carrying: None,
    });
    world
        .military
        .get_mut(&TARGET)
        .unwrap()
        .soldiers
        .push(walker);
    assert!(world.can_take(target, P0));
    // An own building can always be entered (MIL-CON-1.1).
    let defender = garrison(&mut world, TARGET, 0, 3);
    assert!(!world.can_take(target, P0));
    assert!(world.can_take(target, P1));
    let _ = defender;
}

// T3: the whole way, from the order to the taken building.
// spec: MIL-CON-1.1, MIL-CON-1.2, MIL-CON-2.2, MIL-CON-2.3 (T3)
#[test]
fn con_t3_conquest() {
    let mut world = scene();
    fill(&mut world, P1, TOWER, TARGET, &[(0, 3)]);
    fill(&mut world, P0, GUARD, (40, 20), &privates(2));
    attack(&mut world, 1, false);
    kill_defenders(&mut world, TARGET);
    let old = world.building_at(TARGET).unwrap().serial;
    run(&mut world, 700);
    let b = world.building_at(TARGET).expect("the new tower");
    assert_eq!(b.owner, P0);
    assert_eq!(b.building_type, TOWER);
    assert!(!b.under_construction);
    assert_ne!(b.serial, old);
    assert!(world.ruins().is_empty(), "no ruin for a conquest");
    assert_eq!(
        message_count(&world, P1, "Lost Military Building to enemy."),
        1
    );
    assert_eq!(message_count(&world, P0, "Military Building conquered."), 1);
    // It claimed at once: range 10 around it.
    assert!(world.military_at(TARGET).unwrap().applied);
    assert_eq!(world.field_owner((59, 20)), Some(P0));
    assert!(world.is_explored(P0, (62, 20)));
    // The taker went in.
    assert_eq!(world.military_at(TARGET).unwrap().soldiers.len(), 1);
    assert_eq!(world.present_soldiers(TARGET), 1);
    assert!(world.attack_records().is_empty());
    // It has the creation defaults.
    let m = world.military_at(TARGET).unwrap();
    assert!(m.coin_switch && !m.move_out);
}

// MIL-CON-2.2 step 3a: the old flag goes as by the player: its roads, the
// wares lying on it. The new building has a fresh flag with no roads.
#[test]
fn con_t3a_the_old_flag_is_removed() {
    let mut world = scene();
    fill(&mut world, P1, TOWER, TARGET, &[(0, 3)]);
    fill(&mut world, P0, GUARD, (40, 20), &privates(2));
    let flag = world.flag_of(TARGET);
    let other = (flag.0 + 3, flag.1);
    let p1 = PlayerId(1);
    assert_eq!(
        apply(
            &mut world,
            Command::PlaceFlag {
                player: p1,
                at: other
            }
        ),
        Ok(())
    );
    assert_eq!(
        apply(
            &mut world,
            Command::BuildRoad {
                player: p1,
                from: flag,
                to: other
            }
        ),
        Ok(())
    );
    assert_eq!(world.roads().count(), 1);
    for _ in 0..2 {
        world.new_ware_at(1, flag, None, None);
    }
    assert_eq!(world.wares_at(flag).len(), 2);
    attack(&mut world, 1, false);
    kill_defenders(&mut world, TARGET);
    run(&mut world, 700);
    assert_eq!(world.building_at(TARGET).unwrap().owner, P0);
    assert_eq!(world.roads().count(), 0);
    assert_eq!(world.flag_of(TARGET), flag);
    assert_eq!(world.flag_at(flag), Some(P0));
    assert!(world.wares_at(flag).is_empty());
}

// TRN-ROAD T14: destroying a flag takes its roads, the building behind it
// with remains, and the wares lying on it.
// spec: TRN-ROAD-6.2, MIL-CON-4.4 (TRN-ROAD T14)
#[test]
fn road_t14_destroy_a_flag() {
    let mut world = owned_world(40, 40);
    finished(&mut world, P0, BARRACKS, (20, 20));
    let flag = world.flag_of((20, 20));
    let other = (flag.0 + 3, flag.1);
    assert_eq!(
        apply(
            &mut world,
            Command::PlaceFlag {
                player: P0,
                at: other
            }
        ),
        Ok(())
    );
    assert_eq!(
        apply(
            &mut world,
            Command::BuildRoad {
                player: P0,
                from: flag,
                to: other
            }
        ),
        Ok(())
    );
    for _ in 0..3 {
        world.new_ware_at(1, flag, None, None);
    }
    let outcome = apply(
        &mut world,
        Command::RemoveFlag {
            player: P0,
            at: flag,
        },
    );
    assert_eq!(outcome, Ok(()));
    assert_eq!(world.roads().count(), 0);
    assert!(world.building_at((20, 20)).is_none());
    assert_eq!(world.ruins().len(), 1, "removed with remains");
    assert_eq!(world.flag_at(flag), None);
    assert!(world.wares_at(flag).is_empty());
    assert_eq!(world.wares().count(), 0);
    // The other end flag stays (TRN-ROAD-6.1).
    assert_eq!(world.flag_at(other), Some(P0));
    // Not an owner's flag, not a road.
    assert_eq!(
        apply(
            &mut world,
            Command::RemoveFlag {
                player: P0,
                at: flag
            }
        ),
        Err(Rejection::NotAFlag)
    );
    assert_eq!(
        apply(
            &mut world,
            Command::RemoveRoad {
                player: P0,
                at: (21, 20)
            }
        ),
        Err(Rejection::NoRoad)
    );
}

// TRN-ROAD T15: a ware on its way to a building, destroyed with its flag:
// the building's order for that good is open again.
// spec: TRN-ROAD-6.4 (TRN-ROAD T15)
#[test]
fn road_t15_destroyed_ware_is_ordered_again() {
    let mut world = owned_world(40, 40);
    finished(&mut world, P0, BARRACKS, (30, 20));
    let target = world.target_of((30, 20)).unwrap();
    let destination = world.flag_of((30, 20));
    let lost = (22, 20);
    assert_eq!(
        apply(
            &mut world,
            Command::PlaceFlag {
                player: P0,
                at: lost
            }
        ),
        Ok(())
    );
    world.new_ware_at(COIN, lost, Some(destination), Some(target));
    let before = world.orders().len();
    assert_eq!(
        apply(
            &mut world,
            Command::RemoveFlag {
                player: P0,
                at: lost
            }
        ),
        Ok(())
    );
    assert_eq!(world.wares().count(), 0);
    let new: Vec<_> = world.orders()[before..].iter().collect();
    assert_eq!(new.len(), 1);
    assert_eq!((new[0].good, new[0].target.at), (COIN, (30, 20)));
}

// T4: eight attackers wait; the taker and five others go in (wanted 6),
// the other two walk home.
// spec: MIL-CON-2.3, MIL-CON-3.1 (T4)
#[test]
fn con_t4_filling_the_taken_building() {
    let mut world = scene();
    fill(&mut world, P1, TOWER, TARGET, &[(0, 3)]);
    // An enemy near, so that the new tower wants its full limit.
    fill(&mut world, P1, GUARD, (56, 20), &privates(1));
    fill(&mut world, P0, FORTRESS, (40, 20), &[(0, 3)]);
    kill_defenders(&mut world, TARGET);
    let flag = world.flag_of(TARGET);
    let fields: Vec<Point> = (1..=2)
        .flat_map(|k| grid::ring(TARGET, k, 100, 50))
        .filter(|&f| f != flag && !world.terrain().has(f, movement::NOT_WALKABLE))
        .take(8)
        .collect();
    assert_eq!(fields.len(), 8);
    for f in fields {
        waiting_attacker(&mut world, (40, 20), f);
    }
    run(&mut world, 1000);
    assert_eq!(world.building_at(TARGET).unwrap().owner, P0);
    assert_eq!(world.military_at(TARGET).unwrap().soldiers.len(), 6);
    assert_eq!(world.present_soldiers(TARGET), 6);
    // The two that did not fit are back in the fortress.
    assert_eq!(world.military_at((40, 20)).unwrap().soldiers.len(), 3);
    assert!(world.attack_records().is_empty());
    assert_eq!(
        world
            .settlers()
            .filter(|(_, s)| matches!(s.job, SettlerJob::Attack { .. }))
            .count(),
        0
    );
}

// T5: a headquarters is destroyed; the settlers of its stock walk out.
// spec: MIL-CON-2.1, MIL-CON-4.3 (T5)
#[test]
fn con_t5_headquarters_falls() {
    let mut world = scene();
    fill(&mut world, P0, GUARD, (76, 20), &privates(3));
    let hq = (85, 20);
    world.military.get_mut(&hq).unwrap().castle = [0; SOLDIER_RANKS];
    // Its soldiers go back into the stock, which is then set by hand.
    run(&mut world, 3);
    world.warehouses.get_mut(&hq).unwrap().stock = vec![(WOODCUTTER, 4), (HELPER, 30), (PLANK, 50)];
    world.submit(crate::Command::Attack {
        player: P0,
        target: hq,
        count: 1,
        strong_first: false,
    });
    for _ in 0..800 {
        world.step();
        if world.building_at(hq).is_none() {
            break;
        }
    }
    assert!(world.building_at(hq).is_none(), "the headquarters fell");
    assert_eq!(message_count(&world, P0, "Attacked Castle destroyed."), 1);
    assert!(world.ruins().is_empty(), "the headquarters leaves no ruin");
    let walking = world
        .settlers()
        .filter(|(_, s)| s.player == P1 && s.profession == WOODCUTTER)
        .count();
    assert_eq!(walking, 4);
    // The attacker goes home.
    assert!(world
        .settlers()
        .any(|(_, s)| s.player == P0 && matches!(s.job, SettlerJob::Attack { .. })));
}

// MIL-CON-4.3 with soldiers in the stock: helpers and wares are lost.
#[test]
fn con_4_3_stock_settlers_walk_out() {
    let mut world = scene();
    let hq = (85, 20);
    world.military.get_mut(&hq).unwrap().castle = [0; SOLDIER_RANKS];
    run(&mut world, 3);
    world.warehouses.get_mut(&hq).unwrap().stock =
        vec![(WOODCUTTER, 4), (RANK[0], 2), (HELPER, 30), (PLANK, 50)];
    world.remove_with_remains(hq);
    let mut found: Vec<u32> = world
        .settlers()
        .filter(|(_, s)| s.player == P1)
        .map(|(_, s)| s.profession)
        .collect();
    found.sort_unstable();
    assert_eq!(
        found,
        vec![RANK[0], RANK[0], WOODCUTTER, WOODCUTTER, WOODCUTTER, WOODCUTTER]
    );
}

/// A finished hut of player 0 on (20, 20) whose type costs frame 2 planks
/// and main 3 planks and 4 stones.
fn costly_hut() -> (World, Point) {
    let mut world = owned_world(40, 40);
    let hut = world.building_types.get_mut(&HUT).unwrap();
    hut.frame_costs = vec![(PLANK, 2)];
    hut.costs = vec![(PLANK, 3), (STONE, 4)];
    finished(&mut world, P0, HUT, (20, 20));
    let flag = world.flag_of((20, 20));
    (world, flag)
}

fn goods_at(world: &World, flag: Point) -> Vec<u32> {
    world
        .wares_at(flag)
        .iter()
        .map(|&w| world.ware(w).unwrap().good)
        .collect()
}

// MIL-CON T17 (4.1, 4.4; AI-CON-9.3): the computer player's demolish
// removes the building with remains. Frame round(1 x 2) = 2 planks, main
// M = 7: 3 planks, 4 stones; halved rounded up: 3 planks, 2 stones, put on
// the flag in ascending good order one per round (stone 65, plank 67): stone,
// plank, stone, plank, plank; the five wares take their numbers, then the
// ruin one more.
// spec: MIL-CON-4.1, MIL-CON-4.4 (MIL-CON T17), AI-CON-9.3, SIM-START-4.2
#[test]
fn con_t17_a_torn_down_building_leaves_material_and_a_ruin() {
    let (mut world, flag) = costly_hut();
    let before = world.next_object();
    let torn = Command::TearDown {
        player: P0,
        at: (20, 20),
    };
    assert_eq!(apply(&mut world, torn), Ok(()));
    assert!(world.building_at((20, 20)).is_none());
    assert_eq!(world.ruins().len(), 1);
    assert_eq!(goods_at(&world, flag), [STONE, PLANK, STONE, PLANK, PLANK]);
    assert_eq!(world.next_object(), before + 6);
    // Only the owner tears down.
    let (mut world, _) = costly_hut();
    let foreign = Command::TearDown {
        player: P1,
        at: (20, 20),
    };
    assert_eq!(apply(&mut world, foreign), Err(Rejection::NoBuilding));
}

// MIL-CON T14 (4.4): remains of 5 wares on a flag already holding 4: 2 are
// put down (the flag at 6), 3 lost.
// spec: MIL-CON-4.4 (MIL-CON T14)
#[test]
fn con_t14_remains_that_do_not_fit_are_lost() {
    let (mut world, flag) = costly_hut();
    for _ in 0..4 {
        world.new_ware_at(COIN, flag, None, None);
    }
    world.remove_with_remains((20, 20));
    assert_eq!(
        goods_at(&world, flag),
        [COIN, COIN, COIN, COIN, STONE, PLANK]
    );
}

// MIL-CON-4.4 step 4: a site before any work gives back nothing of its
// type's material but its unused material in full, and leaves no ruin.
// spec: MIL-CON-4.4, MIL-CON-4.1
#[test]
fn con_4_4_a_site_gives_back_its_unused_material() {
    let mut world = owned_world(40, 40);
    let hut = world.building_types.get_mut(&HUT).unwrap();
    hut.frame_costs = vec![(PLANK, 2)];
    hut.costs = vec![(PLANK, 3), (STONE, 4)];
    let placed = Command::PlaceBuilding {
        player: P0,
        building: HUT,
        at: (20, 20),
    };
    assert_eq!(apply(&mut world, placed), Ok(()));
    world.sites.get_mut(&(20, 20)).unwrap().on_site = vec![(PLANK, 3)];
    let flag = world.flag_of((20, 20));
    world.remove_with_remains((20, 20));
    assert_eq!(goods_at(&world, flag), [PLANK, PLANK, PLANK]);
    assert!(world.ruins().is_empty());
}

// MIL-CON-4.1: a burnt warehouse leaves a ruin that burns out.
#[test]
fn con_4_1_ruins_burn_out() {
    let mut world = scene();
    fill(&mut world, P1, TOWER, TARGET, &[(0, 3)]);
    world.remove_with_remains(TARGET);
    assert_eq!(world.ruins().len(), 1);
    assert!(world.building_at(TARGET).is_none());
    run(&mut world, 332);
    assert_eq!(world.ruins().len(), 1);
    run(&mut world, 2);
    assert!(world.ruins().is_empty());
}

// MIL-CON-4.1a: while the ruin burns its field is marked like a building's
// (Blocked): no building or flag can be placed on it; burnt out, the mark
// is cleared and the field takes a building again.
// spec: MIL-CON-4.1a, ECO-PLACE-5.1, MIL-CON-4.1
#[test]
fn con_4_1a_a_burning_ruin_blocks_its_field() {
    let (mut world, _) = costly_hut();
    let at = (20, 20);
    world.remove_with_remains(at);
    assert_eq!(world.ruins().len(), 1);
    assert!(world.terrain().has(at, state::BLOCKED));
    assert!(world.check_building(P0, at, BuildingSize::Small).is_err());
    let flag = Command::PlaceFlag { player: P0, at };
    assert!(apply(&mut world, flag).is_err());
    run(&mut world, 340);
    assert!(world.ruins().is_empty());
    assert!(!world.terrain().has(at, state::BLOCKED));
    assert_eq!(world.check_building(P0, at, BuildingSize::Small), Ok(()));
}

// ---- The catapult ----

const CATAPULT_AT: Point = (40, 20);

/// Player 0 owns the map; player 1 occupies barracks at `targets`; a
/// catapult of player 0 with one stone stands on `CATAPULT_AT`.
fn catapult_scene(targets: &[Point]) -> World {
    let mut world = owned_world(100, 50);
    for &t in targets {
        occupied(&mut world, P1, BARRACKS, t);
    }
    finished(&mut world, P0, CATAPULT, CATAPULT_AT);
    let index = world.terrain().index(CATAPULT_AT);
    let b = world.buildings[index].as_mut().unwrap();
    b.workers = 1;
    b.active = true;
    world.inputs.insert(CATAPULT_AT, vec![(STONE, 1)]);
    world
}

fn phase(world: &World) -> CatapultPhase {
    world
        .catapult_at(CATAPULT_AT)
        .map_or(CatapultPhase::Idle, |c| c.phase)
}

fn run_until(world: &mut World, limit: u64, done: impl Fn(&World) -> bool) -> u64 {
    for i in 0..limit {
        if done(world) {
            return i;
        }
        world.step();
    }
    panic!("not reached: {:?}", phase(world));
}

// T6: the nearest enemy building is the target; the search comes after 301
// updates.
// spec: MIL-CON-5.2, MIL-CON-5.3 (T6)
#[test]
fn con_t6_nearest_target_after_301_updates() {
    let mut world = catapult_scene(&[(52, 20), (50, 20)]);
    let started = run_until(&mut world, 10, |w| {
        matches!(phase(w), CatapultPhase::Searching { .. })
    });
    let tick = world.tick();
    run_until(&mut world, 400, |w| {
        matches!(phase(w), CatapultPhase::Turning { .. })
    });
    // Found in the 301st update after the search step started.
    assert_eq!(world.tick() - tick, 301);
    let _ = started;
    let CatapultPhase::Turning { aim, .. } = phase(&world) else {
        unreachable!()
    };
    assert_eq!(aim, (50, 20), "distance 10, not 12");
    // Hit chance: 13 - d of 19 draws hit.
    for (d, hits) in [(1, 12), (10, 3), (12, 1)] {
        let n = (0..19)
            .filter(|&draw| !catapult_misses(12, d, draw))
            .count();
        assert_eq!(n, hits);
    }
}

// T6b: no target within 12: the search goes on 301 updates later.
// spec: MIL-CON-5.2, MIL-CON-5.3 (T6b)
#[test]
fn con_5_2_no_target_searches_again() {
    let mut world = catapult_scene(&[(53, 20)]);
    assert_eq!(grid::distance(CATAPULT_AT, (53, 20)), 13);
    run_until(&mut world, 10, |w| {
        matches!(phase(w), CatapultPhase::Searching { .. })
    });
    run(&mut world, 700);
    assert!(matches!(phase(&world), CatapultPhase::Searching { .. }));
    assert_eq!(world.catapult_stones(CATAPULT_AT), 1);
}

// T7, T8: turn steps and times.
// spec: MIL-CON-5.4 (T7, T8)
#[test]
fn con_t7_t8_turning() {
    // Facing 1, landing direction 5: 6 - 5 = 1 step, 22 ticks; back: (1 - 5)
    // mod 6 = 2 steps, 33 ticks.
    assert_eq!(turn_steps(1, 5), 1);
    assert_eq!(turn_steps(5, 1), 2);
    // Facing 1, direction 3: 2 steps.
    assert_eq!(turn_steps(1, 3), 2);
    assert_eq!(turn_steps(1, 1), 0);
    assert_eq!(turn_steps(1, 4), 2);
    assert_eq!(turn_steps(1, 0), 5);
    assert_eq!(turn_steps(1, 2), 1);
}

// T11 and the cycle: turn, throw (a stone is used), flight of 11 ticks per
// field, impact, turn back, idle.
// spec: MIL-CON-5.4 (T11)
#[test]
fn con_5_4_the_shot() {
    let mut world = catapult_scene(&[(50, 20)]);
    run_until(&mut world, 400, |w| {
        matches!(phase(w), CatapultPhase::Turning { .. })
    });
    let CatapultPhase::Turning { aim, landing, left } = phase(&world) else {
        unreachable!()
    };
    let n = movement::direction_toward(CATAPULT_AT, landing);
    assert_eq!(left, (turn_steps(1, n) + 1) * 11);
    assert_eq!(aim, (50, 20));
    assert_eq!(world.catapult_stones(CATAPULT_AT), 1);
    let turn = run_until(&mut world, 200, |w| {
        matches!(phase(w), CatapultPhase::Flying { .. })
    });
    assert_eq!(u32::try_from(turn).unwrap(), left);
    let CatapultPhase::Flying { left, .. } = phase(&world) else {
        unreachable!()
    };
    assert_eq!(left, grid::distance(CATAPULT_AT, landing) * 11);
    // The stone has left the stock.
    assert_eq!(world.catapult_stones(CATAPULT_AT), 0);
    assert_eq!(world.catapult_at(CATAPULT_AT).unwrap().facing, n);
    run_until(&mut world, 400, |w| {
        matches!(phase(w), CatapultPhase::TurningBack { .. })
    });
    run_until(&mut world, 200, |w| matches!(phase(w), CatapultPhase::Idle));
    assert_eq!(world.catapult_at(CATAPULT_AT).unwrap().facing, 1);
}

// T9: the weakest present soldier dies, the building stays.
// spec: MIL-CON-5.5 (T9)
#[test]
fn con_t9_hit_kills_the_weakest() {
    let mut world = catapult_scene(&[]);
    fill(&mut world, P1, TOWER, (52, 20), &[(2, 5), (0, 3), (0, 2)]);
    world.catapult_hit(CATAPULT_AT, (52, 20));
    let left: Vec<(usize, u32)> = world
        .garrison((52, 20))
        .iter()
        .map(|s| (s.rank, s.health))
        .collect();
    assert_eq!(left, vec![(2, 5), (0, 2)]);
    assert!(world.building_at((52, 20)).is_some());
    assert_eq!(message_count(&world, P1, "Soldier killed by Catapult"), 1);
    let m = world
        .messages()
        .iter()
        .find(|m| m.text == "Soldier killed by Catapult")
        .unwrap();
    assert_eq!(m.at, CATAPULT_AT, "at the catapult's field");
    assert_eq!(world.catapult_kills(P0), 1);
}

// T10: the last present soldier dies and the building burns down.
// spec: MIL-CON-5.5, MIL-CON-4.1 (T10)
#[test]
fn con_t10_last_soldier_burns_the_building() {
    let mut world = catapult_scene(&[]);
    fill(&mut world, P1, TOWER, (52, 20), &[(0, 3)]);
    assert_eq!(world.field_owner((52, 20)), Some(P1));
    world.catapult_hit(CATAPULT_AT, (52, 20));
    assert!(world.building_at((52, 20)).is_none());
    assert_eq!(world.ruins().len(), 1);
    assert_eq!(world.field_owner((52, 20)), None, "the land was released");
}

// MIL-CON-5.3: buildings under interceptors' protection, headquarters and
// out of reach are no targets.
#[test]
fn con_5_3_who_is_a_target() {
    let mut world = catapult_scene(&[(50, 20)]);
    let radius = 12;
    assert_eq!(
        world.catapult_target(CATAPULT_AT, P0, radius),
        Some((50, 20))
    );
    // Own buildings are no targets.
    assert_eq!(world.catapult_target(CATAPULT_AT, P1, radius), None);
    // Interceptors out and fewer than 2 present: spared.
    let target = world.target_of((50, 20)).unwrap();
    let out = world.add_settler(Settler {
        number: 0,
        player: P1,
        profession: RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new((49, 20)),
        job: SettlerJob::Defend {
            home: target,
            health: 3,
            step: DefendStep::Out,
            opponent: None,
            intercept: true,
            defends: Some(target),
        },
        timer: 0,
        carrying: None,
    });
    world
        .military
        .get_mut(&(50, 20))
        .unwrap()
        .soldiers
        .push(out);
    assert_eq!(world.catapult_target(CATAPULT_AT, P0, radius), None);
    // Two soldiers present: not spared any more.
    garrison(&mut world, (50, 20), 0, 3);
    assert_eq!(
        world.catapult_target(CATAPULT_AT, P0, radius),
        Some((50, 20))
    );
}

// MIL-CON-2.3 (Q-1052, E-mil-013): the taker stands on the target's flag,
// the door (MIL-ATK-4.2), and walks in from it as every settler enters a
// building: one step onto the building's field at the usual walking time;
// he is not present while he walks and present once the walk ends.
// spec: MIL-CON-2.3, MIL-ATK-4.2
#[test]
fn con_the_taker_walks_in_from_the_flag() {
    let mut world = scene();
    fill(&mut world, P1, TOWER, TARGET, &[(0, 3)]);
    fill(&mut world, P0, GUARD, (40, 20), &privates(2));
    attack(&mut world, 1, false);
    kill_defenders(&mut world, TARGET);
    let flag = world.flag_of(TARGET);
    let mut taken_at = None;
    for _ in 0..1000 {
        world.step();
        let ours = world.building_at(TARGET).is_some_and(|b| b.owner == P0);
        if ours && taken_at.is_none() {
            taken_at = Some(world.tick());
            let id = world.military_at(TARGET).unwrap().soldiers[0];
            let s = world.settlers[id].as_ref().unwrap();
            assert_eq!(s.walker.at, flag, "taken from the flag");
            assert!(
                matches!(s.job, SettlerJob::ToMilitary { .. }),
                "{:?}",
                s.job
            );
            assert_eq!(world.present_soldiers(TARGET), 0, "not in yet");
        }
        if let (Some(t), 1) = (taken_at, world.present_soldiers(TARGET)) {
            let walk = world.tick() - t;
            assert!((8..=40).contains(&walk), "one step's walking time: {walk}");
            return;
        }
    }
    panic!("never inside");
}
