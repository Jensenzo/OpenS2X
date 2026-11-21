//! Tests of `specs/MIL-attack-combat.md`, on the small rule set of
//! `military_tests`.

use crate::combat::{dialog_count, AttackStep, DefendStep, UNDER_ATTACK};
use crate::grid::Point;
use crate::military_tests::*;
use crate::movement::Walker;
use crate::test_support::P0;
use crate::{Command, PlayerId, Settler, SettlerJob, World};

pub(crate) const TARGET: Point = (50, 20);

/// Two headquarters far apart on a flat 100 x 50 map.
pub(crate) fn scene() -> World {
    game(100, 50, &[(P0, (10, 20)), (P1, (85, 20))])
}

/// A finished, occupied building of `kind` with soldiers `(rank, health)`.
pub(crate) fn fill(
    world: &mut World,
    player: PlayerId,
    kind: u32,
    at: Point,
    soldiers: &[(usize, u32)],
) {
    finished(world, player, kind, at);
    for &(rank, health) in soldiers {
        garrison(world, at, rank, health);
    }
    world.step();
}

pub(crate) fn privates(n: usize) -> Vec<(usize, u32)> {
    vec![(0, 3); n]
}

pub(crate) fn attack(world: &mut World, count: u32, strong_first: bool) {
    world.submit(Command::Attack {
        player: P0,
        target: TARGET,
        count,
        strong_first,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
}

/// The home building field and (rank, health) of every attacker, in record
/// order.
fn attackers(world: &World) -> Vec<(Point, usize, u32)> {
    world
        .attack_records()
        .iter()
        .map(|r| {
            let s = world.settlers[r.soldier].as_ref().expect("a soldier");
            let SettlerJob::Attack { home, health, .. } = s.job else {
                panic!("not an attacker: {:?}", s.job);
            };
            (home.at, world.soldier_rank(s.profession).unwrap(), health)
        })
        .collect()
}

fn interceptors(world: &World) -> Vec<usize> {
    world
        .settlers()
        .filter(|(_, s)| {
            matches!(
                s.job,
                SettlerJob::Defend {
                    intercept: true,
                    ..
                }
            )
        })
        .map(|(id, _)| id)
        .collect()
}

/// A guard house of player 0 ten fields from a barracks of player 1.
pub(crate) fn standard(attackers: usize, defenders: usize) -> World {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &privates(defenders));
    fill(&mut world, P0, GUARD, (40, 20), &privates(attackers));
    world
}

// T1
// spec: MIL-ATK-1.1, MIL-ATK-1.2 (T1)
#[test]
fn atk_t1_available_attackers() {
    let world = standard(3, 1);
    assert_eq!(world.attack_available(P0, TARGET), 2);
    // Only the enemy can be attacked, and a ring field must be ours.
    assert_eq!(world.attack_available(P1, TARGET), 0);
}

// T23 (MIL-ATK-1.1 item 7, corrected 2026-11-17 day, E-mil-020): a
// building's share is trunc((n - 1) x attackStrength x f(d)), f(d) =
// min(36 - d, 24) / 24: full up to 12 fields, 1/24 less per field beyond.
// R6's first attack: d 12 with 2 soldiers -> 1, d 17 with 6 -> trunc(5 x
// 19/24) = 3, d 22 with 3 -> trunc(2 x 14/24) = 1: 5 available, 5 sent by
// distance (MIL-ATK-2.1), five records and five draws (8 without the
// factor).
// spec: MIL-ATK-1.1 (T23), MIL-ATK-1.2, MIL-ATK-2.1, MIL-ATK-2.3
#[test]
fn atk_t23_the_share_falls_with_the_distance() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &privates(1));
    fill(&mut world, P0, GUARD, (38, 20), &privates(2));
    fill(&mut world, P0, TOWER, (33, 20), &privates(6));
    fill(&mut world, P0, GUARD, (28, 20), &privates(3));
    assert_eq!(world.attack_available(P0, TARGET), 5);
    let draws = world.rng.draws();
    attack(&mut world, 8, false);
    let homes: Vec<Point> = attackers(&world).into_iter().map(|a| a.0).collect();
    assert_eq!(
        homes,
        vec![(38, 20), (33, 20), (33, 20), (33, 20), (28, 20)]
    );
    assert!(world.rng.draws() >= draws + 5);
    // The factor alone: 13/24 at d 23, and a hair above each fraction.
    for (d, n, k) in [
        (12, 25, 24),
        (13, 25, 23),
        (23, 25, 13),
        (17, 6, 3),
        (22, 3, 1),
    ] {
        let f = (36u32 - d).min(24) as f32 * (1.0f32 / 24.0);
        assert_eq!(((n as f64 - 1.0) * f64::from(f)) as u32, k, "d {d}");
    }
}

// T1 (ring): without a field of ours on the ring around the target nothing
// can be sent, however close the building.
// spec: MIL-ATK-1.2 (T1)
#[test]
fn atk_1_2_needs_a_field_on_the_ring() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &privates(1));
    fill(&mut world, P0, GUARD, (40, 20), &privates(3));
    assert_eq!(world.attack_available(P0, TARGET), 2);
    for field in crate::grid::ring(TARGET, 9, 100, 50) {
        let index = world.terrain().index(field);
        if world.owners[index] == Some(P0) {
            world.owners[index] = None;
        }
    }
    assert_eq!(world.attack_available(P0, TARGET), 0);
}

// T2
// spec: MIL-ATK-1.2, MIL-SOL-7.1 (T2)
#[test]
fn atk_t2_weak_attack_strength_sends_nobody() {
    let mut world = standard(3, 1);
    world.submit(Command::MilitaryAttackStrength {
        player: P0,
        thousandths: 300,
    });
    world.step();
    assert_eq!(world.attack_available(P0, TARGET), 0);
    attack(&mut world, 2, false);
    assert!(world.attack_records().is_empty());
}

// MIL-ATK-1.1 (3, 5, 7): the headquarters, buildings out of reach and a
// lone soldier send nobody.
// spec: MIL-ATK-1.1
#[test]
fn atk_1_1_who_cannot_send() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &privates(1));
    // A lone soldier stays home; 23 away is in reach, 24 is not.
    fill(&mut world, P0, GUARD, (40, 20), &privates(1));
    fill(&mut world, P0, TOWER, (27, 20), &privates(4));
    fill(&mut world, P0, TOWER, (26, 20), &privates(4));
    assert_eq!(crate::grid::distance((27, 20), TARGET), 23);
    assert_eq!(crate::grid::distance((26, 20), TARGET), 24);
    // The headquarters stands within 40 of nothing here, but never sends.
    // Item 7's distance factor (T23): at 23 the tower's share is
    // trunc(3 x 13/24) = 1.
    assert_eq!(world.attack_available(P0, TARGET), 1);
}

// T3
// spec: MIL-ATK-1.4 (T3)
#[test]
fn atk_t3_dialog_counts() {
    let counts = crate::combat::COUNT_FRACTIONS.map(|s| dialog_count(7, s));
    assert_eq!(counts, [3, 4, 6, 7]);
    assert_eq!(dialog_count(1, 0.25), 1);
    assert_eq!(dialog_count(2, 0.25), 1);
    let world = standard(3, 1);
    assert_eq!(world.attack_counts(P0, TARGET), [1, 2, 2, 2]);
}

// T4
// spec: MIL-ATK-2.1 (T4)
#[test]
fn atk_t4_buildings_by_distance_then_list_order() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &privates(1));
    fill(&mut world, P0, GUARD, (42, 20), &privates(3)); // X: d 8, k 2
    fill(&mut world, P0, GUARD, (45, 20), &privates(2)); // Y: d 5, k 1
    fill(&mut world, P0, TOWER, (58, 20), &privates(4)); // Z: d 8, k 3
    assert_eq!(world.attack_available(P0, TARGET), 6);
    attack(&mut world, 4, false);
    let homes: Vec<Point> = attackers(&world).into_iter().map(|a| a.0).collect();
    assert_eq!(homes, vec![(45, 20), (42, 20), (42, 20), (58, 20)]);
}

fn fortress_world(soldiers: &[(usize, u32)]) -> World {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &privates(1));
    fill(&mut world, P0, FORTRESS, (40, 20), soldiers);
    world
}

// T5
// spec: MIL-ATK-2.2 (T5)
#[test]
fn atk_t5_weak_first() {
    let mut world = fortress_world(&[(0, 3), (2, 5), (2, 2), (4, 7), (0, 1)]);
    attack(&mut world, 3, false);
    let sent: Vec<(usize, u32)> = attackers(&world).iter().map(|a| (a.1, a.2)).collect();
    assert_eq!(sent, vec![(0, 3), (0, 1), (2, 5)]);
}

// T6
// spec: MIL-ATK-2.2 (T6)
#[test]
fn atk_t6_strong_first() {
    let mut world = fortress_world(&[(0, 3), (2, 5), (2, 2), (4, 7), (0, 1)]);
    attack(&mut world, 3, true);
    let sent: Vec<(usize, u32)> = attackers(&world).iter().map(|a| (a.1, a.2)).collect();
    assert_eq!(sent, vec![(4, 7), (2, 5), (2, 2)]);
}

// MIL-ATK-2.3 (corrected 2026-11-17 night, Q-1070): the soldier sent stays
// on his home building's soldier list, where he counts but is not present
// (MIL-SOL-2.4), and one random number per record is drawn.
// spec: MIL-ATK-2.3, MIL-SOL-2.4
#[test]
fn atk_2_3_sending_keeps_him_on_his_homes_list_and_draws_once() {
    let mut world = standard(4, 1);
    let before = world.rng.clone();
    let listed = world.military_at((40, 20)).unwrap().soldiers.clone();
    attack(&mut world, 2, false);
    let m = world.military_at((40, 20)).unwrap();
    assert_eq!(m.soldiers, listed, "nobody leaves the list");
    assert_eq!(world.attack_records().len(), 2);
    let sent: Vec<usize> = world.attack_records().iter().map(|r| r.soldier).collect();
    assert!(sent.iter().all(|s| listed.contains(s)));
    assert_eq!(world.present_soldiers((40, 20)), 2);
    let away: Vec<usize> = world
        .garrison((40, 20))
        .iter()
        .filter(|s| !s.present)
        .map(|s| s.id)
        .collect();
    assert_eq!(away.len(), 2);
    assert!(away.iter().all(|s| sent.contains(s)));
    // The commands of a step come first: one draw per record.
    let mut expected = before;
    for r in world.attack_records() {
        assert_eq!(u64::from(r.counter), u64::from(expected.draw()) % 2);
    }
}

fn military_requests(world: &World) -> usize {
    world
        .requests(P0)
        .iter()
        .filter(|r| matches!(r.requester, crate::requests::Requester::Military(_)))
        .count()
}

/// The building on `at` lists the settler `id` among its soldiers.
fn listed(world: &World, at: Point, id: usize) -> bool {
    world
        .military_at(at)
        .is_some_and(|m| m.soldiers.contains(&id))
}

// MIL-SOL-4.2 with MIL-ATK-2.3: the attackers out still count in S, so
// their home asks for no replacement while they are away (was: unlinked at
// the sending, a request in each pass).
// spec: MIL-SOL-4.2, MIL-SOL-4.3, MIL-ATK-2.3
#[test]
fn sol_4_2_attackers_out_count_and_bring_no_replacement() {
    let mut world = standard(3, 1);
    assert_eq!(world.wanted_soldiers((40, 20)), 3);
    attack(&mut world, 2, false);
    // The passes in ticks 20, 41 and 62.
    crate::military_tests::run_to(&mut world, 63);
    assert_eq!(world.attack_records().len(), 2);
    assert_eq!(world.military_at((40, 20)).unwrap().soldiers.len(), 3);
    assert_eq!(military_requests(&world), 0);
}

// MIL-SOL-4.4 step 2 (Q-142, Q-1070; E-mil-018): a building with one
// soldier too many releases a soldier that is not present first, in list
// order, so an attacker out goes before anyone inside. He keeps his errand;
// when it ends (here: his target gone) he has no home and goes to a
// warehouse (MIL-ATK-4.4, TRN-FREE-6.4). The one still kept walks back and
// is present again on arrival, with no test of the wanted number.
// spec: MIL-SOL-4.4, MIL-ATK-2.3, MIL-ATK-4.4, MIL-SOL-2.4
#[test]
fn sol_4_4_an_attacker_out_is_released_and_goes_to_a_warehouse() {
    let home = (40, 20);
    let mut world = standard(3, 1);
    // A second enemy building near, so that the guard house still wants 3
    // when the target is gone (MIL-SOL-3.2); with one soldier it sends no
    // interceptor (MIL-ATK-3.1).
    fill(&mut world, P1, GUARD, (56, 20), &privates(1));
    attack(&mut world, 2, false);
    let sent: Vec<usize> = world.attack_records().iter().map(|r| r.soldier).collect();
    let list = world.military_at(home).unwrap().soldiers.clone();
    // One soldier more than the 3 wanted.
    let extra = garrison(&mut world, home, 0, 3);
    let first_out = *list.iter().find(|s| sent.contains(s)).unwrap();
    let kept = *sent.iter().find(|&&s| s != first_out).unwrap();
    crate::military_tests::run_to(&mut world, 21);
    assert!(!listed(&world, home, first_out), "released by step 2");
    assert!(listed(&world, home, kept));
    assert!(listed(&world, home, extra), "nobody inside goes");
    assert_eq!(world.present_soldiers(home), 2);
    // He fights on: still an attacker with his record.
    assert!(matches!(
        world.settlers[first_out].as_ref().unwrap().job,
        SettlerJob::Attack { .. }
    ));
    assert!(world
        .attack_records()
        .iter()
        .any(|r| r.soldier == first_out));
    // The target goes: their errands end (MIL-ATK-4.4).
    world.submit(Command::RemoveBuilding {
        player: P1,
        at: TARGET,
    });
    let job = |world: &World, id: usize| world.settlers[id].as_ref().map(|s| s.job.clone());
    let mut gone = false;
    for _ in 0..800 {
        world.step();
        match job(&world, first_out) {
            Some(SettlerJob::Attack {
                step: AttackStep::Home,
                ..
            })
            | Some(SettlerJob::Garrison { .. }) => panic!("the released one went back"),
            Some(SettlerJob::Attack { .. }) => {}
            _ => {
                gone = true;
                break;
            }
        }
    }
    assert!(gone, "the released one goes to a warehouse");
    assert!(!listed(&world, home, first_out));
    // The kept one is back inside and present, now one more than wanted.
    for _ in 0..800 {
        if matches!(job(&world, kept), Some(SettlerJob::Garrison { .. })) {
            break;
        }
        world.step();
    }
    assert!(matches!(
        job(&world, kept),
        Some(SettlerJob::Garrison { .. })
    ));
    assert!(listed(&world, home, kept));
    assert!(!listed(&world, home, first_out));
}

// MIL-SOL-4.4 step 2 (Q-1070 c): a defender out in a fight is not present
// either: released, he fights on, and when his fight ends he goes to a
// warehouse instead of back to the building.
// spec: MIL-SOL-4.4, MIL-SOL-2.4, MIL-ATK-6.6
#[test]
fn sol_4_4_a_released_defender_fights_on_then_goes_to_a_warehouse() {
    let (mut world, attacker, defender, _) = pair_on_field(0, 1, 4, 7);
    // The barracks (wanted 2) lists one soldier inside and the defender:
    // a third one makes one too many.
    let extra = garrison(&mut world, TARGET, 0, 3);
    let inside = world.military_at(TARGET).unwrap().soldiers[0];
    crate::military_tests::run_to(&mut world, 21);
    assert!(!listed(&world, TARGET, defender), "released by step 2");
    assert!(listed(&world, TARGET, inside) && listed(&world, TARGET, extra));
    let mut ended = None;
    for _ in 0..3000 {
        if fallen(&world, attacker) {
            ended = Some(world.tick());
            break;
        }
        assert!(
            matches!(
                world.settlers[defender].as_ref().unwrap().job,
                SettlerJob::Defend { .. }
            ),
            "he fights on"
        );
        world.step();
    }
    assert!(ended.is_some(), "the attacker fell");
    world.step();
    // T32: he acts in his update after the one that saw the pair gone.
    while matches!(
        world.settlers[defender].as_ref().unwrap().job,
        SettlerJob::Defend {
            step: DefendStep::Ended,
            ..
        }
    ) {
        world.step();
    }
    let job = &world.settlers[defender].as_ref().unwrap().job;
    assert!(
        !matches!(job, SettlerJob::Defend { .. } | SettlerJob::Garrison { .. }),
        "to a warehouse, not back: {job:?}"
    );
    assert!(!listed(&world, TARGET, defender));
}

// MIL-ATK-2.3 (Q-1070), docs/snapshot.md: a game saved before format 84
// had its attackers out off their homes' lists; loaded, each is put back at
// the end of its list, so his home counts him and he comes back to it.
// spec: MIL-ATK-2.3, MIL-SOL-4.2
#[test]
fn atk_2_3_an_older_games_attackers_go_back_on_their_lists() {
    let mut world = standard(4, 1);
    attack(&mut world, 2, false);
    let sent: Vec<usize> = world.attack_records().iter().map(|r| r.soldier).collect();
    let mut older = world.clone();
    let list = &mut older.military.get_mut(&(40, 20)).unwrap().soldiers;
    list.retain(|s| !sent.contains(s));
    let inside = list.clone();
    older.attackers_back_on_their_lists();
    let mut expected = inside;
    let mut by_id = sent.clone();
    by_id.sort_unstable();
    expected.extend(by_id);
    assert_eq!(older.military_at((40, 20)).unwrap().soldiers, expected);
    // A game of this format is read as it was.
    let loaded = World::from_snapshot(&world.snapshot()).unwrap();
    assert_eq!(
        loaded.military_at((40, 20)).unwrap().soldiers,
        world.military_at((40, 20)).unwrap().soldiers
    );
}

// MIL-SOL-4.4 (Q-1071 b; E-mil-019): an attacker released while still
// inside, waiting to walk out, no longer waits for his home's release
// cooldown: he walks out in his next update and sets none.
// spec: MIL-SOL-4.4, MIL-ATK-4.1
#[test]
fn sol_4_4_released_inside_he_walks_out_without_the_cooldown() {
    let home = (40, 20);
    let mut world = standard(4, 1);
    world.submit(Command::MilitaryDefenders {
        player: P1,
        thousandths: 0,
    });
    world.step();
    attack(&mut world, 3, false);
    for _ in 0..3 {
        world.step();
    }
    let step_of = |world: &World, id: usize| match world.settlers[id].as_ref().unwrap().job {
        SettlerJob::Attack { step, .. } => Some(step),
        _ => None,
    };
    let waiting: Vec<usize> = world
        .attack_records()
        .iter()
        .map(|r| r.soldier)
        .filter(|&id| step_of(&world, id) == Some(AttackStep::Leaving))
        .collect();
    assert_eq!(waiting.len(), 2, "one walked out, two wait");
    let cooldown = world.military_at(home).unwrap().cooldown;
    assert!(cooldown > 0);
    // Released, as MIL-SOL-4.4 step 2 does: off the list.
    let id = waiting[1];
    world
        .military
        .get_mut(&home)
        .unwrap()
        .soldiers
        .retain(|&s| s != id);
    world.step();
    assert_eq!(step_of(&world, id), Some(AttackStep::Walking));
    assert_eq!(step_of(&world, waiting[0]), Some(AttackStep::Leaving));
    assert!(
        world.military_at(home).unwrap().cooldown <= cooldown,
        "none set"
    );
}

// MIL-SOL-4.4 (Q-1071 c; E-mil-019): when a soldier's home building goes,
// an interceptor in a fight loses only his home link: he fights on, and
// when his fight is over and the building he defends has no attackers left
// he goes to a warehouse (was: sent home at once).
// spec: MIL-SOL-4.4, MIL-ATK-4.4, MIL-ATK-3.4
#[test]
fn sol_4_4_a_defender_whose_home_goes_fights_on() {
    let (mut world, attacker, defender, _) = pair_on_field(0, 1, 4, 7);
    let d_at = (56, 20);
    fill(&mut world, P1, GUARD, d_at, &privates(1));
    let target = world.target_of(TARGET).unwrap();
    let d = world.target_of(d_at).unwrap();
    world
        .military
        .get_mut(&TARGET)
        .unwrap()
        .soldiers
        .retain(|&s| s != defender);
    world
        .military
        .get_mut(&d_at)
        .unwrap()
        .soldiers
        .push(defender);
    if let SettlerJob::Defend {
        home,
        intercept,
        defends,
        ..
    } = &mut world.settlers[defender].as_mut().unwrap().job
    {
        *home = d;
        *intercept = true;
        *defends = Some(target);
    }
    world.submit(Command::RemoveBuilding {
        player: P1,
        at: d_at,
    });
    world.step();
    assert!(world.building_at(d_at).is_none_or(|b| b.serial != d.serial));
    let fights = |world: &World| {
        matches!(
            world.settlers[defender].as_ref().unwrap().job,
            SettlerJob::Defend { opponent: Some(a), .. } if a == attacker
        )
    };
    assert!(fights(&world), "he fights on");
    for _ in 0..3000 {
        if fallen(&world, attacker) {
            break;
        }
        assert!(fights(&world));
        world.step();
    }
    assert!(fallen(&world, attacker), "the attacker fell");
    world.step();
    // T32: he acts in his update after the one that saw the pair gone.
    while matches!(
        world.settlers[defender].as_ref().unwrap().job,
        SettlerJob::Defend {
            step: DefendStep::Ended,
            ..
        }
    ) {
        world.step();
    }
    let job = &world.settlers[defender].as_ref().unwrap().job;
    assert!(
        !matches!(job, SettlerJob::Defend { .. } | SettlerJob::Garrison { .. }),
        "to a warehouse: {job:?}"
    );
}

// MIL-ATK-2.3, MIL-ATK-6.6: an attacker who falls leaves his home's list.
// spec: MIL-ATK-2.3, MIL-ATK-6.6
#[test]
fn atk_2_3_a_fallen_attacker_leaves_his_homes_list() {
    let (mut world, attacker, _, _) = pair_on_field(0, 1, 4, 7);
    assert!(listed(&world, (40, 20), attacker));
    for _ in 0..3000 {
        if world.settlers[attacker].is_none() {
            break;
        }
        world.step();
    }
    assert!(world.settlers[attacker].is_none(), "the attacker fell");
    assert!(!listed(&world, (40, 20), attacker));
    assert_eq!(world.military_at((40, 20)).unwrap().soldiers.len(), 2);
}

// T7
// spec: MIL-ATK-3.1, MIL-ATK-3.2 (T7)
#[test]
fn atk_t7_interceptors_need_an_attacker_to_pair() {
    let mut world = standard(3, 4);
    attack(&mut world, 2, false);
    // The barracks has 4 present, defend outside 1.0: 3, but there are two
    // attackers only.
    let out = interceptors(&world);
    assert_eq!(out.len(), 2);
    for r in world.attack_records() {
        assert!(r.field.is_some());
        assert!(r.opponent.is_some_and(|o| out.contains(&o)));
    }
    // Both pairs have their own meeting field, next to the target (the
    // second one the first free field from the best one).
    let fields: Vec<Point> = world
        .attack_records()
        .iter()
        .map(|r| r.field.unwrap())
        .collect();
    assert_ne!(fields[0], fields[1]);
    for f in fields {
        assert!(crate::grid::distance(f, TARGET) <= 2);
    }
    // The barracks keeps one soldier inside.
    assert_eq!(world.present_soldiers(TARGET), 2);
}

// MIL-ATK-3.1: a smaller "Intercepting Defenders" setting sends fewer.
#[test]
fn atk_3_1_defend_outside_scales_the_interceptors() {
    let mut world = standard(3, 4);
    world.submit(Command::MilitaryDefenders {
        player: P1,
        thousandths: 340,
    });
    world.step();
    attack(&mut world, 2, false);
    // trunc(3 x 0.34) = 1
    assert_eq!(interceptors(&world).len(), 1);
}

// MIL-ATK-3.1: other buildings of the target's owner intercept too, and
// those already out are counted.
#[test]
fn atk_3_1_other_buildings_intercept() {
    let mut world = standard(4, 2);
    fill(&mut world, P1, GUARD, (60, 20), &privates(3));
    attack(&mut world, 3, false);
    // The barracks: trunc(1 x 1.0) = 1; the guard house: trunc(2 x 1.0) = 2.
    let homes: Vec<Point> = world
        .settlers()
        .filter_map(|(_, s)| match s.job {
            SettlerJob::Defend {
                home,
                intercept: true,
                ..
            } => Some(home.at),
            _ => None,
        })
        .collect();
    assert_eq!(homes.iter().filter(|&&h| h == TARGET).count(), 1);
    assert_eq!(homes.iter().filter(|&&h| h == (60, 20)).count(), 2);
    // A second order adds none: they are all out already.
    let out = interceptors(&world).len();
    attack(&mut world, 1, false);
    assert_eq!(interceptors(&world).len(), out);
}

// T13: one soldier leaves a building per 40 ticks.
// spec: MIL-ATK-4.1 (T13)
#[test]
fn atk_t13_departures_every_40_ticks() {
    let mut world = standard(4, 1);
    // No defenders come out: the interceptors would leave through the
    // target's own door.
    world.submit(Command::MilitaryDefenders {
        player: P1,
        thousandths: 0,
    });
    world.step();
    let c = world.tick();
    attack(&mut world, 3, false);
    let walking = |w: &World| {
        w.settlers()
            .filter(|(_, s)| {
                matches!(
                    s.job,
                    SettlerJob::Attack {
                        step: AttackStep::Walking,
                        ..
                    }
                )
            })
            .count()
    };
    assert_eq!(walking(&world), 0);
    let mut left = Vec::new();
    let mut seen = 0;
    for _ in 0..90 {
        let t = world.tick();
        world.step();
        if walking(&world) > seen {
            seen = walking(&world);
            left.push(t - c);
        }
    }
    assert_eq!(left, vec![1, 41, 81]);
}

// T24 (MIL-ATK-4.1a, E-mil-021): attackers and interceptors walk out
// through their own building's flag: one step from the building's field to
// the flag at progress 0 in the tick they leave; on its point, in that
// arrival tick, the next walk starts from the flag (the attacker towards the
// target or his meeting field, the interceptor to his meeting field).
// spec: MIL-ATK-4.1a (T24), MIL-ATK-4.1, MIL-ATK-4.2, MIL-ATK-3.3
#[test]
fn atk_t24_out_through_the_own_flag() {
    let mut world = standard(3, 4);
    attack(&mut world, 1, false);
    let attacker = world.attack_records()[0].soldier;
    let interceptor = interceptors(&world)[0];
    let soldiers = [(attacker, (40, 20)), (interceptor, TARGET)];
    // Per soldier: seen leaving, seen walking on from the flag.
    let mut seen = [(false, false); 2];
    for _ in 0..200 {
        world.step();
        for (i, &(id, home)) in soldiers.iter().enumerate() {
            let flag = world.flag_of(home);
            let k = world.settlers[id].as_ref().unwrap().walker.clone();
            if !seen[i].0 && k.is_walking() {
                // Leaving: the walk is the one step to the flag, at 0.
                assert_eq!(k.at, home, "{id}");
                assert_eq!(k.destination(), flag, "{id}: to his own flag first");
                assert_eq!(k.progress(), 0.0, "{id}");
                seen[i].0 = true;
            } else if seen[i].0 && !seen[i].1 && k.at == flag {
                // On the flag, in the arrival tick: the next walk, from
                // the flag, at progress 0.
                assert!(k.is_walking(), "{id}: walks on in the arrival tick");
                assert_ne!(k.destination(), flag, "{id}");
                assert_eq!(k.progress(), 0.0, "{id}");
                seen[i].1 = true;
            }
        }
        if seen.iter().all(|s| s.1) {
            break;
        }
    }
    assert_eq!(seen, [(true, true); 2]);
}

// T25 (MIL-ATK-3.3, 4.3, E-mil-022): both searches test their start field
// first, then first in, first out, a field's neighbours in the ring order of
// MAP-GRID-3.2 (directions 5, 0, 1, 2, 3, 4). A target that defends itself
// meets its first attacker on its direction-5 neighbour when that is free
// (in the order 0 .. 5 it would be the direction-0 neighbour); the next pair
// gets the first free field from there, in the same order.
// spec: MIL-ATK-3.3 (T25), MIL-ATK-4.3, MAP-GRID-3.2
#[test]
fn atk_t25_the_searches_go_in_ring_order() {
    let mut world = standard(3, 4);
    attack(&mut world, 2, false);
    let ring = crate::grid::ring(TARGET, 1, 100, 50);
    let first = ring[0];
    assert!(world.flag_at(first).is_none());
    let fields: Vec<Point> = world
        .attack_records()
        .iter()
        .map(|r| r.field.unwrap())
        .collect();
    assert_eq!(fields[0], first, "the direction-5 neighbour");
    // The second: the free-field search from the same best field, taken
    // ring by ring from it: its own direction-5 neighbour first.
    let next = crate::grid::ring(first, 1, 100, 50)
        .into_iter()
        .find(|&f| {
            world.flag_at(f).is_none() && !world.terrain().has(f, crate::movement::NOT_WALKABLE)
        })
        .unwrap();
    assert_eq!(fields[1], next);
}

// T27 (MIL-ATK-4.1a, 5.3): the target's own defender, paired with an
// attacker at its door, leaves through his building's flag as well: one
// step to the flag at progress 0, and on its point, in that tick, his walk
// to the attacker's waiting field from there.
// spec: MIL-ATK-4.1a (T27), MIL-ATK-5.3
#[test]
fn atk_t27_the_targets_defender_out_through_his_flag() {
    let mut world = standard(3, 2);
    // No interceptors: the defender comes out when the attacker is at the
    // door.
    world.submit(Command::MilitaryDefenders {
        player: P1,
        thousandths: 0,
    });
    world.step();
    attack(&mut world, 1, false);
    let flag = world.flag_of(TARGET);
    let defender = |w: &World| {
        w.settlers().find_map(|(id, s)| {
            matches!(
                s.job,
                SettlerJob::Defend {
                    intercept: false,
                    ..
                }
            )
            .then_some(id)
        })
    };
    let mut seen = (false, false);
    for _ in 0..800 {
        world.step();
        let Some(id) = defender(&world) else {
            continue;
        };
        let k = world.settlers[id].as_ref().unwrap().walker.clone();
        if !seen.0 && k.is_walking() {
            assert_eq!(k.at, TARGET);
            assert_eq!(k.destination(), flag, "to his own flag first");
            assert_eq!(k.progress(), 0.0);
            seen.0 = true;
        } else if seen.0 && !seen.1 && k.at == flag {
            assert!(k.is_walking(), "walks on in the arrival tick");
            assert_ne!(k.destination(), flag);
            assert_eq!(k.progress(), 0.0);
            seen.1 = true;
            break;
        }
    }
    assert_eq!(seen, (true, true));
}

// T28 (MIL-ATK-5.3, 6.1, E-mil-025): a defender's walk ends on the waiting
// field's point in one update (A); his arrival (the pair fighting) counts in
// his next update (A + 1), when his job sees the movement stopped.
// spec: MIL-ATK-5.3 (T28), MIL-ATK-6.1
#[test]
fn atk_t28_the_defenders_arrival_counts_in_his_next_update() {
    let mut world = standard(3, 2);
    world.submit(Command::MilitaryDefenders {
        player: P1,
        thousandths: 0,
    });
    world.step();
    attack(&mut world, 1, false);
    let defender = |w: &World| {
        w.settlers().find_map(|(id, s)| {
            matches!(
                s.job,
                SettlerJob::Defend {
                    intercept: false,
                    ..
                }
            )
            .then_some(id)
        })
    };
    let step = |w: &World, id: usize| match w.settlers[id].as_ref().unwrap().job {
        SettlerJob::Defend { step, .. } => step,
        ref job => panic!("{job:?}"),
    };
    let mut ended = None;
    for _ in 0..800 {
        world.step();
        let Some(id) = defender(&world) else {
            continue;
        };
        let r = world
            .attack_records()
            .iter()
            .find(|r| r.opponent == Some(id))
            .cloned();
        let Some(field) = r.and_then(|r| r.field) else {
            continue;
        };
        let k = &world.settlers[id].as_ref().unwrap().walker;
        if k.at == field && !k.is_walking() {
            ended = Some(id);
            break;
        }
    }
    let id = ended.expect("his walk ends on the waiting field");
    assert_eq!(step(&world, id), DefendStep::Out, "A: not arrived yet");
    world.step();
    assert_eq!(step(&world, id), DefendStep::Waiting, "A + 1: arrived");
}

// T29 (MIL-ATK-6.1a, E-mil-025): during his step off the waiting field's
// point the attacker's field (as the views show it) stays the waiting
// field; from the update the step ends it is the field his sample
// truncates to.
// spec: MIL-ATK-6.1a (T29), MAP-GRID-4.3
#[test]
fn atk_t29_the_attackers_field_after_his_step() {
    let mut world = standard(3, 4);
    attack(&mut world, 1, false);
    let attacker = world.attack_records()[0].soldier;
    let field = world.attack_records()[0].field.unwrap();
    let view = |w: &World| {
        w.settler_view(crate::view::SettlerRef::Settler(attacker))
            .expect("the attacker")
    };
    let mut during = false;
    for _ in 0..800 {
        world.step();
        let v = view(&world);
        let Some(f) = v.fine else {
            continue;
        };
        if f.step.next.is_some() {
            assert_eq!(v.walk.at, field, "during the step: the waiting field");
            during = true;
        } else if during {
            assert_eq!(
                v.walk.at,
                crate::grid::sample_field_truncating(f.step.at),
                "the step ended: his sample's field"
            );
            return;
        }
    }
    panic!("no step off the point seen (during: {during})");
}

// T30 (MIL-ATK-3.4, E-mil-025): an attacker pairing with a waiting
// interceptor only links the two; the interceptor's walk to the attacker's
// field starts in the interceptor's own update, not inside the attacker's.
// spec: MIL-ATK-3.4 (T30), MIL-ATK-5.1
#[test]
fn atk_t30_the_pairing_only_links_the_waiting_interceptor() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 3)]);
    fill(&mut world, P0, GUARD, (40, 20), &[(0, 3), (0, 3)]);
    let target = world.target_of(TARGET).unwrap();
    let interceptor = garrison(&mut world, TARGET, 0, 3);
    let wait = (52, 21);
    {
        let s = world.settlers[interceptor].as_mut().unwrap();
        s.walker = Walker::new(wait);
        s.job = SettlerJob::Defend {
            home: target,
            health: 3,
            step: DefendStep::Waiting,
            opponent: None,
            intercept: true,
            defends: Some(target),
        };
    }
    let field = (48, 20);
    let attacker = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: crate::military_tests::RANK[0],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(field),
        job: SettlerJob::Attack {
            home: world.target_of((40, 20)).unwrap(),
            health: 3,
            step: AttackStep::Waiting,
        },
        timer: 0,
        carrying: None,
    });
    world
        .military
        .get_mut(&(40, 20))
        .unwrap()
        .soldiers
        .push(attacker);
    world.attackers.push(crate::AttackRecord {
        soldier: attacker,
        target,
        counter: 0,
        field: Some(field),
        opponent: None,
        hit: false,
        offset: None,
    });
    // The attacker (made after him, so updated after him in a tick) pairs
    // in his waiting check: the interceptor is linked, not walking yet.
    world.step();
    assert_eq!(world.attack_records()[0].opponent, Some(interceptor));
    let k = world.settlers[interceptor].as_ref().unwrap().walker.clone();
    assert!(!k.is_walking(), "no walk inside the attacker's update");
    world.step();
    let k = world.settlers[interceptor].as_ref().unwrap().walker.clone();
    assert!(k.is_walking(), "his own update starts it");
    assert_eq!(k.destination(), field);
}

// T26 (MIL-ATK-6.1a, E-mil-024): an attacker walking to the meeting field
// an interception gave him is not in his wait job: arriving (A) he only
// becomes waiting (no step); in A + 1 the checks of MIL-ATK-4.4 pair him
// (his wait job starts); in A + 2 the draw and his step off the point. (One
// paired at the door steps on arrival, T19.)
// spec: MIL-ATK-6.1a (T26), MIL-ATK-4.4, MIL-ATK-3.3
#[test]
fn atk_t26_the_interceptions_attacker_steps_two_updates_after_arriving() {
    let mut world = standard(3, 4);
    attack(&mut world, 1, false);
    let record = |w: &World| w.attack_records()[0].clone();
    let attacker = record(&world).soldier;
    let field = record(&world).field.expect("a meeting field");
    assert!(record(&world).opponent.is_some(), "the interceptor linked");
    let step = |w: &World| match w.settlers[attacker].as_ref().unwrap().job {
        SettlerJob::Attack { step, .. } => step,
        ref job => panic!("{job:?}"),
    };
    let mut arrived = None;
    for _ in 0..400 {
        world.step();
        let s = world.settlers[attacker].as_ref().unwrap();
        if s.walker.at == field && !s.walker.is_walking() {
            arrived = Some(world.tick());
            break;
        }
    }
    arrived.expect("on the meeting field");
    assert_eq!(step(&world), AttackStep::Arrived, "A: only waiting");
    assert!(record(&world).offset.is_none(), "A: no step");
    world.step();
    assert_eq!(step(&world), AttackStep::Waiting, "A + 1: his wait job");
    assert!(record(&world).offset.is_none(), "A + 1: no step yet");
    world.step();
    assert!(
        record(&world).offset.is_some(),
        "A + 2: the draw and the step"
    );
}

// ---- The duel, set up by hand ----

/// An attacker of rank `ra` (home: the guard house) and a defender of rank
/// `rd` (the barracks), both waiting on `field`, paired.
fn pair_on_field(ra: usize, ha: u32, rd: usize, hd: u32) -> (World, usize, usize, Point) {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 3)]);
    fill(&mut world, P0, GUARD, (40, 20), &[(0, 3), (0, 3)]);
    let defender = garrison(&mut world, TARGET, rd, hd);
    let field = (49, 20);
    let attacker = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[ra],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(field),
        job: SettlerJob::Attack {
            home: world.target_of((40, 20)).unwrap(),
            health: ha,
            step: AttackStep::Waiting,
        },
        timer: 0,
        carrying: None,
    });
    // MIL-ATK-2.3: he stays on his home's list.
    world
        .military
        .get_mut(&(40, 20))
        .unwrap()
        .soldiers
        .push(attacker);
    let target = world.target_of(TARGET).unwrap();
    world.attackers.push(crate::AttackRecord {
        soldier: attacker,
        target,
        counter: 0,
        field: Some(field),
        opponent: Some(defender),
        hit: false,
        // MIL-ATK-6.1a: his step off the point made (none here).
        offset: Some(crate::combat::OffsetStep {
            walk: crate::movement::LatticeWalk::standing(crate::grid::field_sample(field)),
            back: false,
        }),
    });
    let s = world.settlers[defender].as_mut().unwrap();
    s.walker = Walker::new(field);
    s.job = SettlerJob::Defend {
        home: target,
        health: hd,
        step: DefendStep::Waiting,
        opponent: Some(attacker),
        intercept: false,
        defends: None,
    };
    (world, attacker, defender, field)
}

/// MIL-ATK-6.6 (T31): the soldier fell: dying on his spot, or gone.
pub(crate) fn fallen(world: &World, id: usize) -> bool {
    world.settlers[id]
        .as_ref()
        .is_none_or(|s| matches!(s.job, SettlerJob::Dying { .. }))
}

fn health(world: &World, id: usize) -> Option<u32> {
    match world.settlers[id].as_ref()?.job {
        SettlerJob::Attack { health, .. } | SettlerJob::Defend { health, .. } => Some(health),
        _ => None,
    }
}

// T8: rounds are 17 ticks apart, the first by the record's counter.
// spec: MIL-ATK-6.2, MIL-ATK-6.4 (T8)
#[test]
fn atk_t8_rounds_every_17_ticks() {
    // Generals with 7 health cannot die within 100 ticks.
    let (mut world, ..) = pair_on_field(4, 7, 4, 7);
    let mut rounds = Vec::new();
    let mut counter = 0;
    for _ in 0..100 {
        let t = world.tick();
        world.step();
        let now = world.attack_records()[0].counter;
        if now != counter {
            rounds.push(t);
            counter = now;
        }
    }
    assert_eq!(rounds.len(), 6);
    for (i, &t) in rounds.iter().enumerate() {
        assert_eq!(t, rounds[0] + 17 * i as u64);
    }
}

// GFX-ANIM-3.8 (Q-087 answered): in each round's fight animation the
// soldier who struck plays WorkLoop0, the other Hit when the round's hit
// test scored, else WorkLoop1, both for the fighting time.
// spec: GFX-ANIM-3.8, GFX-ANIM-3.4, MIL-ATK-6.3
#[test]
fn a_rounds_clips_go_to_the_striker_and_the_struck() {
    use crate::trace::Action;
    use crate::view::SettlerRef;
    // Generals with 7 health cannot die within 600 ticks.
    let (mut world, attacker, defender, _) = pair_on_field(4, 7, 4, 7);
    let request = |w: &World, id| w.settler_view(SettlerRef::Settler(id)).unwrap().request;
    assert_eq!(request(&world, attacker), None, "before the first round");
    let (mut hits, mut misses, mut rounds) = (0, 0, 0);
    let mut counter = 0;
    for _ in 0..600 {
        world.step();
        let record = &world.attack_records()[0];
        let (a, d) = (request(&world, attacker), request(&world, defender));
        let attacker_struck = record.counter % 2 == 1;
        let (striker, other) = if attacker_struck { (a, d) } else { (d, a) };
        assert_eq!(striker, Some(Action::WorkLoop(0)), "tick {}", world.tick());
        let expected = if record.hit {
            Action::Hit
        } else {
            Action::WorkLoop(1)
        };
        assert_eq!(other, Some(expected), "tick {}", world.tick());
        if record.counter != counter {
            counter = record.counter;
            rounds += 1;
            if record.hit {
                hits += 1;
            } else {
                misses += 1;
            }
        }
    }
    assert!(
        rounds >= 30 && hits > 0 && misses > 0,
        "{rounds} {hits} {misses}"
    );
}

// MIL-ATK-6.3: the hit of the previous round lands on the soldier who
// strikes now: the attacker with an even counter, the defender with an odd.
#[test]
fn atk_6_3_the_hit_lands_on_the_next_striker() {
    let (mut world, attacker, defender, _) = pair_on_field(2, 5, 1, 4);
    world.attackers[0].hit = true;
    world.step();
    assert_eq!(health(&world, attacker), Some(4));
    assert_eq!(health(&world, defender), Some(4));
    assert_eq!(world.attack_records()[0].counter, 1);
    let (mut world, attacker, defender, _) = pair_on_field(2, 5, 1, 4);
    world.attackers[0].hit = true;
    world.attackers[0].counter = 1;
    world.step();
    assert_eq!(health(&world, attacker), Some(5));
    assert_eq!(health(&world, defender), Some(3));
}

// T11: a killing hit ends the round; the soldier dies in its next update.
// spec: MIL-ATK-6.6 (T11)
#[test]
fn atk_t11_death_comes_one_tick_after_the_hit() {
    let (mut world, attacker, defender, _) = pair_on_field(0, 3, 0, 1);
    world.attackers[0].hit = true;
    world.attackers[0].counter = 1;
    world.step();
    assert_eq!(health(&world, defender), Some(0));
    assert_eq!(world.attack_records()[0].counter, 1, "no new round");
    assert!(world.settlers[defender].is_some());
    world.step();
    // T31: he dies on his spot (TRN-CARRY-7.1), in no pair.
    assert!(matches!(
        world.settlers[defender].as_ref().unwrap().job,
        SettlerJob::Dying { stand: None }
    ));
    assert!(!world
        .military_at(TARGET)
        .unwrap()
        .soldiers
        .contains(&defender));
    // The survivor takes the next opponent: the private inside.
    let next = world.attack_records()[0].opponent.expect("a new opponent");
    assert_ne!(next, defender);
    assert!(health(&world, attacker).is_some());
}

// T31 (MIL-ATK-6.6, E-mil-025): a soldier killed in a duel dies as every
// settler (TRN-CARRY-7.1): from his update that sees his health at 0 (tick
// t) he stands dying where he is, an attacker on his offset sample (his
// field the one it truncates to), for the 28 ticks of the dying and one
// more update: present in t .. t + 28, gone from t + 29. Meanwhile he is in
// no pair and on no soldier list.
// spec: MIL-ATK-6.6 (T31), TRN-CARRY-7.1, MIL-ATK-6.1a
#[test]
fn atk_t31_the_dead_soldier_dies_on_his_spot() {
    use crate::view::{Activity, SettlerRef};
    let (mut world, attacker, defender, field) = pair_on_field(0, 1, 0, 3);
    // His step off the point: one lattice step south-east of it.
    let point = crate::grid::field_sample(field);
    let (columns, rows) = world.terrain.sample_size();
    let off = crate::grid::neighbour(point, crate::grid::Direction::ALL[3], columns, rows).unwrap();
    assert_ne!(off, point);
    world.attackers[0].offset = Some(crate::combat::OffsetStep {
        walk: crate::movement::LatticeWalk::standing(off),
        back: false,
    });
    // The hit lands on the attacker, who strikes in this round.
    world.attackers[0].hit = true;
    world.attackers[0].counter = 0;
    world.step();
    assert_eq!(health(&world, attacker), Some(0));
    world.step();
    let t = world.tick();
    let view = |w: &World| w.settler_view(SettlerRef::Settler(attacker));
    let v = view(&world).expect("dying, not gone");
    assert_eq!(v.activity, Activity::Dying);
    assert_eq!(v.fine.expect("on his sample").step.at, off);
    assert_eq!(v.walk.at, crate::grid::sample_field_truncating(off));
    assert!(world.attack_records().iter().all(|r| r.soldier != attacker));
    assert!(!listed(&world, (40, 20), attacker));
    let _ = defender;
    let mut last = t;
    while view(&world).is_some() {
        let v = view(&world).unwrap();
        assert_eq!(v.activity, Activity::Dying, "tick {}", world.tick());
        assert_eq!(v.fine.unwrap().step.at, off, "he does not move");
        last = world.tick();
        world.step();
        assert!(world.tick() < t + 100, "never removed");
    }
    assert_eq!(last, t + 28, "28 ticks of dying and one more update");
}

// T31a (MIL-ATK-6.6, TRN-CARRY-7.1 corrected, E-mil-027): 14 ticks into a
// dying (dying from t, in t + 14) the skeleton doodad the config names is
// put on the dying settler's sample (an attacker's offset sample, a
// defender's field point) and takes the next object number; none when a
// doodad already stands on that sample, none when the config names none.
// spec: MIL-ATK-6.6 (T31a), TRN-CARRY-7.1, SIM-START-4.2
#[test]
fn atk_t31a_the_skeleton_doodad_fourteen_ticks_into_the_dying() {
    const SKULL: u32 = 77;
    let kill_attacker = |skull: Option<u32>, lying: bool| {
        let (mut world, attacker, _, field) = pair_on_field(0, 1, 0, 3);
        world.settings.skeleton_doodad = skull;
        let point = crate::grid::field_sample(field);
        let (columns, rows) = world.terrain.sample_size();
        let off =
            crate::grid::neighbour(point, crate::grid::Direction::ALL[3], columns, rows).unwrap();
        world.attackers[0].offset = Some(crate::combat::OffsetStep {
            walk: crate::movement::LatticeWalk::standing(off),
            back: false,
        });
        if lying {
            world.skeletons.insert(
                off,
                crate::Skeleton {
                    doodad: SKULL,
                    number: 1,
                    field,
                },
            );
        }
        world.attackers[0].hit = true;
        world.attackers[0].counter = 0;
        world.step();
        world.step();
        assert!(matches!(
            world.settlers[attacker].as_ref().unwrap().job,
            SettlerJob::Dying { .. }
        ));
        (world, off, field)
    };
    let (mut world, off, field) = kill_attacker(Some(SKULL), false);
    for _ in 1..14 {
        world.step();
        assert!(world.skeletons().next().is_none(), "not before t + 14");
    }
    let before = world.next_object();
    world.step();
    let laid: Vec<_> = world.skeletons().map(|(at, s)| (at, *s)).collect();
    assert_eq!(laid.len(), 1, "one skeleton at t + 14");
    assert_eq!(laid[0].0, off, "on his sample");
    assert_eq!(laid[0].1.doodad, SKULL);
    assert_eq!(
        laid[0].1.field,
        crate::grid::sample_field_truncating(off),
        "the field he lies on"
    );
    let _ = field;
    assert!(laid[0].1.number >= before, "a new object number");
    // A skeleton already there: none made, no number taken by it.
    let (mut world, ..) = kill_attacker(Some(SKULL), true);
    for _ in 1..=14 {
        world.step();
    }
    assert_eq!(world.skeletons().count(), 1, "the one lying there only");
    // No skeleton doodad in the config: none.
    let (mut world, ..) = kill_attacker(None, false);
    for _ in 1..=30 {
        world.step();
    }
    assert_eq!(world.skeletons().count(), 0);
}

// T31a: the number is the counter's value at that moment, as any object
// made then (SIM-START-4.2); a defender dying on the field's point lays it
// on that point.
// spec: MIL-ATK-6.6 (T31a), TRN-CARRY-7.1, SIM-START-4.2
#[test]
fn atk_t31a_the_skeleton_takes_the_counters_number() {
    let (mut world, _, defender, field) = pair_on_field(0, 3, 0, 1);
    world.settings.skeleton_doodad = Some(5);
    world.attackers[0].hit = true;
    world.attackers[0].counter = 1;
    world.step();
    world.step();
    assert!(matches!(
        world.settlers[defender].as_ref().unwrap().job,
        SettlerJob::Dying { stand: None }
    ));
    for _ in 1..14 {
        world.step();
    }
    // Nothing else makes an object in this quiet scene at that tick: the
    // skeleton takes the counter's value and raises it by one.
    let before = world.next_object();
    world.step();
    let (at, s) = world.skeletons().next().expect("laid");
    assert_eq!(at, crate::grid::field_sample(field));
    assert_eq!(s.number, before);
    assert_eq!(world.next_object(), before + 1);
}

// GFX-OBJ-1.4a: a skeleton goes for good when its field becomes a road,
// flag or building field (its type deletes by street, as every doodad type
// the scripts leave so).
// spec: GFX-OBJ-1.4a, TRN-CARRY-7.1
#[test]
fn a_skeleton_goes_with_a_road_on_its_field() {
    let (mut world, ..) = pair_on_field(0, 3, 0, 3);
    world.skeletons.insert(
        (10, 10),
        crate::Skeleton {
            doodad: 5,
            number: 1,
            field: (2, 2),
        },
    );
    world.remove_doodads((3, 2), crate::DOODADS_BY_STREET);
    assert_eq!(world.skeletons().count(), 1);
    world.remove_doodads((2, 2), crate::DOODADS_BY_STREET);
    assert_eq!(world.skeletons().count(), 0);
}

// T32 (MIL-ATK-6.6 as corrected, E-mil-025, E-mil-026): the target's
// defender whose attacker dies in tick t (the dead one's update) notices it
// in his first update after the dead one's and starts home in his update
// after that: t + 1 when he comes after the dead one in the settlers'
// order, t + 2 when before. His way home is the walk home of 4.4: a free
// walk to his building's flag, on its point in one update, and in that
// update the step into the building.
// spec: MIL-ATK-6.6 (T32), MIL-ATK-4.4
#[test]
fn atk_t32_the_survivor_acts_two_updates_after_the_death_counted_from_his_own() {
    let run = |attacker_first: bool| {
        let (mut world, attacker, defender, _) = pair_on_field(0, 1, 0, 3);
        // ECO-REQ-0.2: the soldiers' list in number order.
        world.settlers[defender].as_mut().unwrap().number = 100;
        world.settlers[attacker].as_mut().unwrap().number = if attacker_first { 50 } else { 150 };
        world.attackers[0].hit = true;
        world.attackers[0].counter = 0;
        world.step();
        world.step();
        assert!(matches!(
            world.settlers[attacker].as_ref().unwrap().job,
            SettlerJob::Dying { .. }
        ));
        let t = world.tick();
        let walking_home = |w: &World| {
            matches!(
                w.settlers[defender].as_ref().unwrap().job,
                SettlerJob::Defend {
                    step: DefendStep::Back,
                    ..
                }
            )
        };
        assert!(!walking_home(&world));
        while !walking_home(&world) {
            world.step();
            assert!(world.tick() < t + 10);
        }
        let waited = world.tick() - t;
        (world, defender, waited)
    };
    // The defender after the dead attacker (a higher number): t + 1.
    let (_, _, after) = run(true);
    assert_eq!(after, 1, "after the dead one: t + 1");
    // Before him: t + 2.
    let (mut world, defender, before) = run(false);
    assert_eq!(before, 2, "before the dead one: t + 2");
    // Home through his flag: the walk ends on the flag, the step into the
    // building starts in that update.
    let flag = world.flag_of(TARGET);
    let s = world.settlers[defender].as_ref().unwrap();
    assert_eq!(s.walker.path.back(), Some(&flag), "a free walk to his flag");
    let mut on_flag = None;
    for _ in 0..200 {
        world.step();
        let s = world.settlers[defender].as_ref().unwrap();
        if on_flag.is_none() && s.walker.at == flag {
            on_flag = Some(world.tick());
            assert_eq!(
                s.walker.path.iter().copied().collect::<Vec<_>>(),
                vec![TARGET],
                "the step in ordered at once"
            );
            assert!(
                s.walker.next.is_none(),
                "at progress 0, moving from the next tick"
            );
        }
        if !matches!(s.job, SettlerJob::Defend { .. }) {
            assert_eq!(s.walker.at, TARGET, "home");
            break;
        }
    }
    assert!(on_flag.is_some(), "he passed his flag");
}

// T34 (MIL-ATK-5.3, E-mil-026): the target's own soldier paired with an
// attacker stays idle inside (with his opponent, no candidate any more)
// until, in one of his updates, his attacker stands on the target's flag
// field or has arrived (the end of his offset step, 6.1a); he turns to
// wait-for-leave in that update and leaves in his next (under the
// cooldown, through his flag). (a) Paired by an attacker waiting on his
// field: held through the attacker's step, wait-for-leave in his first
// update after it ended. (b) Paired by an attacker at the door: at once.
// spec: MIL-ATK-5.3 (T34), MIL-ATK-6.1a, MIL-ATK-4.1
#[test]
fn atk_t34_the_targets_defender_is_held_until_his_attacker_is_there() {
    let job = |w: &World, id: usize| w.settlers[id].as_ref().unwrap().job.clone();
    let left = |w: &World, id: usize| {
        let s = w.settlers[id].as_ref().unwrap();
        s.walker.at != TARGET || s.walker.is_walking()
    };
    // (a) The attacker waits unpaired on his field (49, 20), on its point.
    for attacker_first in [true, false] {
        let (mut world, attacker, defender, _) = pair_on_field(0, 3, 0, 3);
        // Undo the hand-made pairing: the defender inside, idle.
        world.attackers[0].opponent = None;
        world.attackers[0].offset = None;
        let home = world.target_of(TARGET).unwrap();
        {
            let s = world.settlers[defender].as_mut().unwrap();
            s.walker = Walker::new(TARGET);
            s.job = SettlerJob::Garrison {
                target: home,
                health: 3,
                heal: 0,
            };
        }
        // The barracks' other soldier leaves: he is the one to be paired.
        let other = world.military_at(TARGET).unwrap().soldiers[0];
        assert_ne!(other, defender);
        world
            .military
            .get_mut(&TARGET)
            .unwrap()
            .soldiers
            .retain(|&s| s != other);
        world.settlers[other] = None;
        world.settlers[defender].as_mut().unwrap().number = 100;
        world.settlers[attacker].as_mut().unwrap().number = if attacker_first { 50 } else { 150 };
        let mut paired = None;
        let mut step_ended = None;
        let mut turned = None;
        for _ in 0..200 {
            world.step();
            let t = world.tick();
            let record = world.attack_records()[0].clone();
            if paired.is_none() && record.opponent.is_some() {
                paired = Some(t);
            }
            if step_ended.is_none()
                && record
                    .offset
                    .is_some_and(|o| o.walk.is_done(&world.terrain) && !o.back)
                && record.opponent.is_some()
            {
                step_ended = Some(t);
            }
            match job(&world, defender) {
                SettlerJob::Defend {
                    step: DefendStep::Leaving,
                    ..
                } if turned.is_none() => {
                    turned = Some(t);
                    assert!(!left(&world, defender));
                }
                SettlerJob::Defend { .. } if turned.is_none() => {
                    assert!(!left(&world, defender), "held inside, tick {t}");
                    assert!(paired.is_some());
                }
                _ => {}
            }
            if left(&world, defender) {
                let (turned, step_ended) = (turned.unwrap(), step_ended.unwrap());
                // His first update after the step ended: the same tick when
                // he comes after the attacker, the next when before.
                let expect = if attacker_first {
                    step_ended
                } else {
                    step_ended + 1
                };
                assert_eq!(
                    turned, expect,
                    "wait-for-leave (attacker first: {attacker_first})"
                );
                assert_eq!(t, turned + 1, "the leave in his next update");
                assert!(paired.unwrap() + 2 < step_ended, "the step took its ticks");
                break;
            }
        }
        assert!(turned.is_some(), "he turned to wait-for-leave");
    }
    // (b) An attacker reaching the door (the target's flag) pairs there;
    // the defender, updated after him, turns in that same tick.
    let (mut world, attacker, defender, _) = pair_on_field(0, 3, 0, 3);
    world.attackers[0].opponent = None;
    world.attackers[0].offset = None;
    world.attackers[0].field = None;
    let home = world.target_of(TARGET).unwrap();
    let guard = world.target_of((40, 20)).unwrap();
    {
        let s = world.settlers[defender].as_mut().unwrap();
        s.walker = Walker::new(TARGET);
        s.number = 100;
        s.job = SettlerJob::Garrison {
            target: home,
            health: 3,
            heal: 0,
        };
    }
    let other = world.military_at(TARGET).unwrap().soldiers[0];
    world
        .military
        .get_mut(&TARGET)
        .unwrap()
        .soldiers
        .retain(|&s| s != other);
    world.settlers[other] = None;
    let door = world.flag_of(TARGET);
    {
        let s = world.settlers[attacker].as_mut().unwrap();
        s.number = 50;
        s.walker = Walker::new(door);
        s.job = SettlerJob::Attack {
            home: guard,
            health: 3,
            step: AttackStep::Walking,
        };
    }
    world.step();
    assert!(
        world.attack_records()[0].opponent.is_some(),
        "paired at the door"
    );
    assert!(matches!(
        job(&world, defender),
        SettlerJob::Defend {
            step: DefendStep::Leaving,
            ..
        }
    ));
    world.step();
    assert!(left(&world, defender), "out in the next update");
}

// T33, T33a (MIL-ATK-4.4, E-mil-025, E-mil-026): a soldier walking home
// ends the walk with the step into the building in one update U; at a
// building that is no warehouse his return ends in his next update, and
// only then is he idle (a candidate). At a castle (a warehouse, its
// soldiers on its door field) the walk on to the door field is ordered in
// U itself (no standing tick), and he is idle in the update it ends.
// spec: MIL-ATK-4.4 (T33, T33a), MIL-SOL-6.5
#[test]
fn atk_t33_home_one_update_after_the_step_in_but_not_at_a_castle() {
    let walking_in = |world: &mut World, at: Point| {
        let target = world.target_of(at).unwrap();
        let owner = world.building_at(at).unwrap().owner;
        let flag = world.flag_of(at);
        let mut walker = Walker::new(flag);
        walker.walk([at]);
        let id = world.add_settler(Settler {
            number: 1,
            player: owner,
            profession: RANK[0],
            list: crate::SettlerList::Soldiers,
            walker,
            job: SettlerJob::Defend {
                home: target,
                health: 3,
                step: DefendStep::Back,
                opponent: None,
                intercept: false,
                defends: None,
            },
            timer: 0,
            carrying: None,
        });
        world.military.get_mut(&at).unwrap().soldiers.push(id);
        id
    };
    let idle = |world: &World, id: usize| {
        matches!(
            world.settlers[id].as_ref().unwrap().job,
            SettlerJob::Garrison { .. }
        )
    };
    // A barracks: one update standing in it, then idle.
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 3)]);
    // An enemy near: the barracks wants all its soldiers (MIL-SOL-4.4).
    fill(&mut world, P0, GUARD, (40, 20), &[(0, 3), (0, 3)]);
    let id = walking_in(&mut world, TARGET);
    let mut stepped_in = None;
    for _ in 0..100 {
        world.step();
        let s = world.settlers[id].as_ref().unwrap();
        if stepped_in.is_none() && s.walker.at == TARGET && !s.walker.is_walking() {
            stepped_in = Some(world.tick());
            assert!(
                !idle(&world, id),
                "still returning in the update the step ends"
            );
        } else if let Some(u) = stepped_in {
            assert!(idle(&world, id), "idle in the next update");
            assert_eq!(world.tick(), u + 1);
            break;
        }
    }
    assert!(stepped_in.is_some());
    // The castle (P1's headquarters): on to its door field at once, idle in
    // the update that walk ends.
    let hq = (85, 20);
    // ECO-CAT-2.4: the castle's door north-west (direction 4).
    let kind = world.building_at(hq).unwrap().building_type;
    world.building_types.get_mut(&kind).unwrap().doors = [Some(4); 3];
    let door = world.door_field(hq);
    assert_ne!(door, hq);
    let id = walking_in(&mut world, hq);
    let mut on_castle = None;
    for _ in 0..100 {
        world.step();
        let s = world.settlers[id].as_ref().unwrap();
        if on_castle.is_none() && s.walker.at == hq && s.walker.next.is_none() {
            on_castle = Some(world.tick());
            assert_eq!(
                s.walker.path.iter().copied().collect::<Vec<_>>(),
                vec![door],
                "the walk on to the door field in the same update"
            );
            assert!(!idle(&world, id));
        }
        if s.walker.at == door && !s.walker.is_walking() {
            assert!(idle(&world, id), "idle in the update the walk ends");
            break;
        }
        assert!(!idle(&world, id), "not idle before the door field");
    }
    assert!(on_castle.is_some());
}

// T35 (MIL-ATK-5.1 corrected, E-mil-029): a reinforcement walking in from a
// warehouse ends his step onto the building's field in update U, standing
// there still walking in: no candidate of 5.1 in U; he is one from his next
// update (so an attacker updated before him pairs with him in U + 2).
// spec: MIL-ATK-5.1 (T35), MIL-ATK-5.2
#[test]
fn atk_t35_a_reinforcement_is_a_candidate_one_update_after_his_step_in() {
    let mut world = scene();
    // A private inside, and an enemy near (the barracks keeps its soldiers).
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 3)]);
    fill(&mut world, P0, GUARD, (40, 20), &[(0, 3), (0, 3)]);
    let private = world.military_at(TARGET).unwrap().soldiers[0];
    let target = world.target_of(TARGET).unwrap();
    let mut walker = Walker::new(world.flag_of(TARGET));
    walker.walk([TARGET]);
    // A general walking in: "prefer strong" picks him once he is a candidate.
    let id = world.add_settler(Settler {
        number: 1,
        player: P1,
        profession: RANK[4],
        list: crate::SettlerList::Soldiers,
        walker,
        job: SettlerJob::ToMilitary { target, health: 7 },
        timer: 0,
        carrying: None,
    });
    world.military.get_mut(&TARGET).unwrap().soldiers.push(id);
    let mut step_ended = None;
    for _ in 0..100 {
        world.step();
        let s = world.settlers[id].as_ref().unwrap();
        let chosen = world.pick_defender(TARGET, true);
        if step_ended.is_none() && s.walker.at == TARGET && !s.walker.is_walking() {
            step_ended = Some(world.tick());
            assert_eq!(
                chosen,
                Some(private),
                "not a candidate in the update his step ends"
            );
        } else if let Some(u) = step_ended {
            assert_eq!(chosen, Some(id), "a candidate from his next update");
            assert_eq!(world.tick(), u + 1);
            return;
        } else {
            assert_eq!(chosen, Some(private), "walking in");
        }
    }
    panic!("he never stepped in");
}

// T9, T10: hit chances.
// spec: MIL-ATK-6.5 (T9, T10)
#[test]
fn atk_t9_t10_hit_chances() {
    use crate::military::duel_hit_outcomes;
    assert_eq!(duel_hit_outcomes(4, 0), 24);
    assert_eq!(duel_hit_outcomes(0, 4), 6);
    let table: [[u32; 5]; 5] = [
        [12, 12, 12, 9, 6],
        [12, 12, 12, 9, 6],
        [16, 16, 16, 12, 8],
        [20, 20, 20, 20, 15],
        [24, 24, 24, 24, 18],
    ];
    for (s, row) in table.iter().enumerate() {
        for (o, &n) in row.iter().enumerate() {
            assert_eq!(duel_hit_outcomes(s, o), n, "{s} against {o}");
        }
    }
}

// The hit test of a round draws r1 then r2, each mod 7.
// spec: MIL-ATK-6.3
#[test]
fn atk_6_3_a_round_draws_two_numbers() {
    let (mut world, ..) = pair_on_field(0, 3, 0, 3);
    let mut rng = world.rng.clone();
    let r1 = rng.draw() % 7;
    let r2 = rng.draw() % 7;
    world.step();
    assert_eq!(world.attack_records()[0].hit, r1 <= 2 && r2 > 2);
}

// ---- The whole way ----

fn walkers_out(world: &World) -> usize {
    world
        .settlers()
        .filter(|(_, s)| matches!(s.job, SettlerJob::Attack { .. } | SettlerJob::Defend { .. }))
        .count()
}

// MIL-ATK-4, 5, 6: attacker and defender meet and fight to the end; the
// target's owner is told.
#[test]
fn atk_full_duel_with_message() {
    let mut world = standard(2, 2);
    attack(&mut world, 1, false);
    let mut fought = false;
    for _ in 0..4000 {
        world.step();
        fought |= world.attack_records().iter().any(|r| r.counter > 0);
        if fought && walkers_out(&world) < 2 {
            break;
        }
    }
    assert!(fought, "a duel was fought");
    let alerts: Vec<_> = world
        .messages()
        .iter()
        .filter(|m| m.text == UNDER_ATTACK)
        .collect();
    assert!(!alerts.is_empty());
    assert!(alerts.iter().all(|m| m.player == P1 && m.at == TARGET));
}

// MIL-ATK-6.1, UI-GAME T36a: the attack message's timer is in timer units
// (1/60 a tick): an event while the newest message within one field has at
// least 540 of its 600 units (at most 3600 ticks old) resets it to 600;
// later a second message is added and the first left as it is.
#[test]
fn t36a_under_attack_refresh_in_timer_units() {
    let alerts = |w: &World| -> Vec<(u64, u64)> {
        w.messages()
            .iter()
            .filter(|m| m.text == UNDER_ATTACK)
            .map(|m| (m.tick, m.until))
            .collect()
    };
    let mut world = standard(2, 2);
    let target = world.target_of(TARGET).unwrap();
    for (tick, until) in [(0, 36_000), (3000, 39_000), (6500, 42_500), (6700, 42_700)] {
        world.tick = tick;
        world.under_attack(target);
        assert_eq!(alerts(&world), [(0, until)], "tick {tick}");
    }
    let mut world = standard(2, 2);
    let target = world.target_of(TARGET).unwrap();
    world.tick = 0;
    world.under_attack(target);
    // At exactly 3600 ticks the timer is 540: still refreshed.
    world.tick = 3600;
    world.under_attack(target);
    assert_eq!(alerts(&world), [(0, 39_600)]);
    let mut world = standard(2, 2);
    let target = world.target_of(TARGET).unwrap();
    world.tick = 0;
    world.under_attack(target);
    world.tick = 3601;
    world.under_attack(target);
    assert_eq!(alerts(&world), [(0, 36_000), (3601, 39_601)]);
    // Only the newest is looked at: the next event within its window
    // refreshes M2, never M1.
    world.tick = 4000;
    world.under_attack(target);
    assert_eq!(alerts(&world), [(0, 36_000), (3601, 40_000)]);
}

// MIL-ATK-5.2: the defender that comes out: the strongest with "prefer
// strong soldiers", the weakest otherwise.
#[test]
fn atk_5_2_defender_choice() {
    for (strong, expected) in [(true, 3), (false, 1)] {
        let mut world = scene();
        fill(
            &mut world,
            P1,
            TOWER,
            TARGET,
            &[(1, 4), (3, 6), (1, 2), (3, 3)],
        );
        fill(&mut world, P0, GUARD, (40, 20), &privates(3));
        world.submit(Command::MilitaryDefenseType { player: P1, strong });
        world.submit(Command::MilitaryDefenders {
            player: P1,
            thousandths: 0,
        });
        world.step();
        // One attacker reaches the door (found by running until it waits).
        attack(&mut world, 1, false);
        for _ in 0..600 {
            world.step();
            if world.attack_records()[0].opponent.is_some() {
                break;
            }
        }
        let opponent = world.attack_records()[0].opponent.expect("a defender");
        let s = world.settlers[opponent].as_ref().unwrap();
        let rank = world.soldier_rank(s.profession).unwrap();
        assert_eq!(rank, expected, "strong: {strong}");
        // On equal rank the healthier one.
        let SettlerJob::Defend { health, .. } = s.job else {
            panic!()
        };
        assert_eq!(health, if strong { 6 } else { 4 });
    }
}

// MIL-ATK-4.3 (corrected): a waiting field is no road field and no flag
// field.
#[test]
fn atk_4_3_free_field_avoids_flags_and_roads() {
    let mut world = standard(2, 1);
    let flag = world.flag_of(TARGET);
    let start = flag;
    let field = world.free_field(start, None);
    assert_ne!(field, Some(flag));
    // And not a field another waits on.
    let taken = field.unwrap();
    let target = world.target_of(TARGET).unwrap();
    world.attackers.push(crate::AttackRecord {
        soldier: 0,
        target,
        counter: 0,
        field: Some(taken),
        opponent: None,
        hit: false,
        offset: None,
    });
    assert_ne!(world.free_field(start, None), Some(taken));
}

// MIL-SOL-6.3: a headquarters with attackers and no soldier present brings
// one soldier of each rank out of the stock, general first, each only while
// the door is free (6.2, T20): in one tick only the general.
// spec: MIL-SOL-6.3, MIL-SOL-6.2 (MIL T20)
#[test]
fn atk_5_4_headquarters_brings_defenders_from_stock() {
    let mut world = scene();
    fill(&mut world, P0, GUARD, (76, 20), &privates(3));
    let hq = (85, 20);
    world.military.get_mut(&hq).unwrap().castle = [0; 5];
    for g in &RANK {
        world.warehouses.get_mut(&hq).unwrap().add(*g, 2);
    }
    // The HQ's soldiers all went back to stock; none is present, and the
    // door's wait from the start is over.
    run(&mut world, 3);
    assert_eq!(world.present_soldiers(hq), 0);
    while world.warehouses[&hq].cooldown > 0 {
        run(&mut world, 1);
    }
    world.submit(Command::Attack {
        player: P0,
        target: hq,
        count: 1,
        strong_first: false,
    });
    run(&mut world, 3);
    assert_eq!(world.last_results().len(), 0);
    assert!(
        !world.attack_records().is_empty(),
        "the order was given: {:?}",
        world.attack_available(P0, hq)
    );
    let ranks: Vec<usize> = world.present_ranks(hq).iter().map(|&(_, r)| r).collect();
    assert_eq!(ranks, [4]);
}

// Fights, conquest and the catapult are deterministic: the same seed and
// commands give the same state, tick by tick.
// spec: none (determinism of fights, conquest and catapults: same seed, same state)
#[test]
fn atk_runs_are_deterministic() {
    let run = |seed: u64| {
        let mut world = scene();
        world.rng = crate::Rng::new(seed as u32);
        fill(&mut world, P1, TOWER, TARGET, &privates(4));
        fill(&mut world, P1, GUARD, (60, 20), &privates(3));
        fill(&mut world, P0, FORTRESS, (40, 20), &privates(9));
        fill(&mut world, P0, GUARD, (44, 24), &privates(3));
        let mut hashes = Vec::new();
        let mut fought = false;
        for t in 0..6000 {
            if t == 10 || t == 1500 {
                world.submit(Command::Attack {
                    player: P0,
                    target: TARGET,
                    count: 20,
                    strong_first: t == 1500,
                });
            }
            world.step();
            fought |= world.attack_records().iter().any(|r| r.counter > 0);
            if t % 250 == 0 {
                hashes.push(world.state_hash());
            }
        }
        (hashes, world, fought)
    };
    let (a, wa, fought) = run(11);
    let (b, wb, _) = run(11);
    assert_eq!(a, b);
    assert_eq!(wa, wb);
    let (c, ..) = run(12);
    assert_ne!(a, c);
    assert!(fought, "duels were fought");
}

// FMT-SAV-3.7a (T17's restore, on our rules): a save's fights go on where
// they stood. A duel (the save's second list) between attacker 501 of the
// guard house and defender 601 of the target, round counter 3 with a hit
// pending, and attacker 502 waiting at his field without an opponent (state
// 20, counter 9): both attackers leave their home's soldiers, the defender
// stays among the target's; the pair stands on the record's field and
// fights its next round there at once: counter 3 is odd, so the defender
// strikes and the pending hit costs him his point first (MIL-ATK-6.3 step 1).
// spec: FMT-SAV-3.7a (T17), MIL-ATK-6.2, MIL-ATK-6.3, MIL-ATK-2.3
#[test]
fn a_saves_fights_go_on_where_they_stood() {
    let mut world = standard(2, 2);
    let home = (40, 20);
    let number = |world: &mut World, at: Point, i: usize, n: u32| {
        let id = world.military[&at].soldiers[i];
        world.settlers[id].as_mut().unwrap().number = n;
        id
    };
    let a = number(&mut world, home, 0, 501);
    let b = number(&mut world, home, 1, 502);
    let d = number(&mut world, TARGET, 0, 601);
    // The defender at 2 of 3.
    if let SettlerJob::Garrison { health, .. } = &mut world.settlers[d].as_mut().unwrap().job {
        *health = 2;
    }
    let (duel, waiting) = ((47, 20), (46, 22));
    // MIL-ATK-6.1a, FMT-SAV-3.7a: the duel's attacker one lattice step west
    // of the duel field's point.
    let duel_sample_off = {
        let (x, y) = crate::grid::field_sample(duel);
        (x - 1, y)
    };
    let fights = [
        crate::RestoredFight {
            attacker: 501,
            attacker_home: home,
            attacker_at: (46, 20),
            attacker_sample: Some(duel_sample_off),
            attacker_state: 18,
            target: TARGET,
            counter: 3,
            field: Some(duel),
            hit: true,
            fighting: true,
            opponent: Some(crate::RestoredOpponent {
                number: 601,
                home: TARGET,
                at: (48, 20),
                intercept: false,
            }),
        },
        crate::RestoredFight {
            attacker: 502,
            attacker_home: home,
            attacker_at: waiting,
            attacker_sample: Some(crate::grid::field_sample(waiting)),
            attacker_state: 20,
            target: TARGET,
            counter: 9,
            field: Some(waiting),
            hit: false,
            fighting: false,
            opponent: None,
        },
        // A soldier the save names but the building does not hold.
        crate::RestoredFight {
            attacker: 777,
            attacker_home: home,
            attacker_at: waiting,
            attacker_sample: None,
            attacker_state: 20,
            target: TARGET,
            counter: 0,
            field: Some(waiting),
            hit: false,
            fighting: false,
            opponent: None,
        },
    ];
    let mut report = crate::RestoreReport::default();
    world.restore_fights(&fights, &mut report);
    assert_eq!((report.fights, report.fights_skipped), (2, 1));
    let records: Vec<_> = world
        .attack_records()
        .iter()
        .map(|r| {
            (
                r.soldier,
                r.target.at,
                r.counter,
                r.field,
                r.opponent,
                r.hit,
            )
        })
        .collect();
    assert_eq!(
        records,
        [
            (a, TARGET, 3, Some(duel), Some(d), true),
            (b, TARGET, 9, Some(waiting), None, false),
        ]
    );
    // MIL-ATK-2.3 (Q-1070): the attackers stay on their home's list.
    assert!(world.military[&home].soldiers.contains(&a));
    assert!(world.military[&home].soldiers.contains(&b));
    assert!(world.military[&TARGET].soldiers.contains(&d));
    let job = |world: &World, id: usize| {
        let s = world.settlers[id].as_ref().unwrap();
        (s.job.clone(), s.walker.at)
    };
    assert!(matches!(
        job(&world, a),
        (
            SettlerJob::Attack {
                step: AttackStep::Waiting,
                health: 3,
                ..
            },
            at
        ) if at == duel
    ));
    assert!(matches!(
        job(&world, d),
        (
            SettlerJob::Defend {
                step: DefendStep::Waiting,
                opponent: Some(o),
                intercept: false,
                health: 2,
                ..
            },
            at
        ) if at == duel && o == a
    ));
    assert!(matches!(
        job(&world, b),
        (
            SettlerJob::Attack {
                step: AttackStep::Waiting,
                ..
            },
            at
        ) if at == waiting
    ));
    assert_eq!(world.duels().len(), 1);
    // MIL-ATK-6.1a: the duel's attacker on his saved sample, his step made
    // for this opponent; the waiting one on his field's point, no step yet.
    let offsets: Vec<_> = world.attack_records().iter().map(|r| r.offset).collect();
    let made = offsets[0].expect("a step made");
    assert_eq!(made.walk.to, duel_sample_off);
    assert!(!made.back);
    assert_eq!(offsets[1], None);
    let fine = world
        .fine_position(world.settlers[a].as_ref().unwrap())
        .expect("off the point");
    assert_eq!((fine.at, fine.next), (duel_sample_off, None));
    world.step();
    let r = &world.attack_records()[0];
    assert_eq!((r.soldier, r.counter), (a, 4));
    assert!(matches!(
        job(&world, d).0,
        SettlerJob::Defend { health: 1, .. }
    ));
}

// T19 (MIL-ATK-6.1a): attacker A paired and on his waiting field W, the
// defender on W's point: A takes the next draw r and walks one lattice step
// from W's point in lattice direction r mod 6, 12 ticks on level ground;
// the defender makes no step and no draw; the first round comes when A's
// step has ended.
// spec: MIL-ATK-6.1a (T19), MIL-ATK-6.1, TRN-FREE-5.3, TRN-FREE-5.4
#[test]
fn atk_t19_the_attacker_steps_off_the_point_and_arrives_after_it() {
    let (mut world, attacker, defender, field) = pair_on_field(4, 7, 4, 7);
    world.attackers[0].offset = None;
    let sample = crate::grid::field_sample(field);
    let mut rng = world.rng().clone();
    let d = (rng.draw() % 6) as usize;
    let (columns, rows) = world.terrain.sample_size();
    let off = crate::grid::neighbour(sample, crate::grid::Direction::ALL[d], columns, rows)
        .expect("on the map");
    let draws = world.rng().draws();
    let start = world.tick();
    let mut first_round = None;
    let mut arrived = None;
    for _ in 0..40 {
        world.step();
        let r = &world.attack_records()[0];
        let o = r.offset.expect("a step");
        assert_eq!((o.walk.from, o.walk.to), (sample, off));
        if arrived.is_none() && o.walk.is_done(&world.terrain) {
            arrived = Some(world.tick());
        }
        if first_round.is_none() && r.counter > 0 {
            first_round = Some(world.tick());
            // Only his draw before the round's two.
            assert_eq!(world.rng().draws(), draws + 3);
        }
    }
    let (arrived, first_round) = (arrived.unwrap(), first_round.unwrap());
    // Ordered in the first tick (start + 1), 12 ticks on level ground.
    assert_eq!(arrived - (start + 1), 12, "12 ticks on level ground");
    assert_eq!(first_round, arrived, "the round in the tick he arrives");
    let fine = world
        .fine_position(world.settlers[attacker].as_ref().unwrap())
        .expect("off the point");
    assert_eq!(fine.at, off);
    // The defender stands on the point.
    let def = world.settlers[defender].as_ref().unwrap();
    assert_eq!(def.walker.at, field);
    assert!(world.fine_position(def).is_none());
}

// T21 (MIL-ATK-6.1a, Q-1050): attacker A waits on his field W unpaired:
// on W's point, no draw; paired with defender D at tick t, one draw and
// the step at t + 1; after D's death A walks the step back to W's point
// (12 ticks, no draw) and only then starts for the door of the target,
// which can now be taken.
// spec: MIL-ATK-6.1a (T21), MIL-ATK-4.4, MIL-ATK-5.1
#[test]
fn atk_t21_unpaired_no_step_and_back_to_the_point_before_the_door() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(0, 1)]);
    fill(&mut world, P0, GUARD, (40, 20), &[(0, 3)]);
    let field = (48, 20);
    let attacker = world.add_settler(Settler {
        number: 0,
        player: P0,
        profession: RANK[4],
        list: crate::SettlerList::Soldiers,
        walker: Walker::new(field),
        job: SettlerJob::Attack {
            home: world.target_of((40, 20)).unwrap(),
            health: 7,
            step: AttackStep::Waiting,
        },
        timer: 0,
        carrying: None,
    });
    let target = world.target_of(TARGET).unwrap();
    world.attackers.push(crate::AttackRecord {
        soldier: attacker,
        target,
        counter: 0,
        field: Some(field),
        opponent: None,
        hit: false,
        offset: None,
    });
    let sample = crate::grid::field_sample(field);
    // Tick t: paired, no draw, no step.
    let draws = world.rng().draws();
    world.step();
    let r = world.attack_records()[0].clone();
    assert!(r.opponent.is_some(), "paired");
    assert_eq!(r.offset, None);
    assert_eq!(world.rng().draws(), draws);
    // t + 1: the draw and the step.
    world.step();
    let o = world.attack_records()[0].offset.expect("the step");
    assert_eq!(o.walk.from, sample);
    assert!(!o.back);
    assert_eq!(world.rng().draws(), draws + 1);
    let off = o.walk.to;
    // The duel to D's death; then the walk back, then the door.
    let mut back_from = None;
    let mut before = world.rng().draws();
    for _ in 0..3000 {
        let draws_before = before;
        world.step();
        before = world.rng().draws();
        let s = world.settlers[attacker].as_ref().unwrap();
        let walking = matches!(
            s.job,
            SettlerJob::Attack {
                step: AttackStep::Walking,
                ..
            }
        );
        let r = world
            .attack_records()
            .iter()
            .find(|r| r.soldier == attacker)
            .unwrap();
        match (back_from, r.offset) {
            (None, Some(o)) if o.back => {
                assert_eq!((o.walk.from, o.walk.to), (off, sample));
                back_from = Some((world.tick(), world.rng().draws()));
            }
            (Some((t, d)), _) if walking => {
                assert_eq!(world.tick() - t, 12, "12 ticks back to the point");
                // The draws of this tick are the door walk's (TRN-FREE).
                assert_eq!(draws_before, d, "no draw on the way back");
                assert_eq!(r.field, None, "for the door");
                return;
            }
            (Some(_), o) => assert!(o.is_some_and(|o| o.back), "nothing before the point"),
            _ => {}
        }
    }
    panic!("never started for the door");
}

// T20, T17 (MIL-ATK-6.1a, 3.4, 5.1): interceptor I of the target wins his
// duel against A1 on his meeting field M; the target still has attacker
// A2 waiting on W != M, whom no idle soldier can meet: I waits where he
// stands (defense wait), is A2's opponent (5.1: the target's waiting
// interceptor first), walks to W's point and fights there; once the target
// has no attackers left he walks home.
// spec: MIL-ATK-6.1a (T20), MIL-ATK-3.4, MIL-ATK-5.1 (T17)
#[test]
fn atk_t20_the_interceptor_waits_and_fights_on_the_next_attackers_field() {
    let mut world = scene();
    fill(&mut world, P1, BARRACKS, TARGET, &[(4, 7)]);
    fill(&mut world, P0, GUARD, (40, 20), &[(0, 3)]);
    let target = world.target_of(TARGET).unwrap();
    let home = world.target_of((40, 20)).unwrap();
    let (m, w) = ((47, 20), (48, 22));
    let interceptor = world.military_at(TARGET).unwrap().soldiers[0];
    let add_attacker = |world: &mut World, at: Point, opponent: Option<usize>| {
        let id = world.add_settler(Settler {
            number: 0,
            player: P0,
            profession: RANK[0],
            list: crate::SettlerList::Soldiers,
            walker: Walker::new(at),
            job: SettlerJob::Attack {
                home,
                health: 1,
                step: AttackStep::Waiting,
            },
            timer: 0,
            carrying: None,
        });
        world.attackers.push(crate::AttackRecord {
            soldier: id,
            target,
            counter: 0,
            field: Some(at),
            opponent,
            hit: false,
            offset: opponent.map(|_| crate::combat::OffsetStep {
                walk: crate::movement::LatticeWalk::standing(crate::grid::field_sample(at)),
                back: false,
            }),
        });
        id
    };
    let a1 = add_attacker(&mut world, m, Some(interceptor));
    let a2 = add_attacker(&mut world, w, None);
    {
        let s = world.settlers[interceptor].as_mut().unwrap();
        s.walker = Walker::new(m);
        s.job = SettlerJob::Defend {
            home: target,
            health: 7,
            step: DefendStep::Waiting,
            opponent: Some(a1),
            intercept: true,
            defends: Some(target),
        };
    }
    let job = |world: &World| world.settlers[interceptor].as_ref().unwrap().job.clone();
    let alive = |world: &World, id: usize| !fallen(world, id);
    let mut waited = false;
    let mut fought_on_w = false;
    for _ in 0..4000 {
        world.step();
        let s = world.settlers[interceptor].as_ref().unwrap();
        match s.job {
            SettlerJob::Defend {
                step: DefendStep::Back,
                ..
            } => assert!(!alive(&world, a2), "home only once A2 is gone"),
            SettlerJob::Defend {
                opponent: Some(o), ..
            } if o == a2 => {
                // Paired with A2 from where he stood (M), not from home.
                if !waited {
                    assert_eq!(s.walker.at, m, "from where he stood");
                    waited = true;
                }
                // (His job names A2 until his next update after A2 fell.)
                if let Some(r) = world.attack_records().iter().find(|r| r.soldier == a2) {
                    assert_eq!(r.field, Some(w), "the duel on A2's waiting field");
                    fought_on_w |= r.counter > 0 && s.walker.at == w;
                }
            }
            _ => {}
        }
        if !alive(&world, a2) && matches!(job(&world), SettlerJob::Garrison { .. }) {
            break;
        }
    }
    assert!(waited, "paired with A2 after his first duel");
    assert!(fought_on_w, "a duel with A2 on W");
    assert!(!alive(&world, a1) && !alive(&world, a2));
    assert!(
        matches!(job(&world), SettlerJob::Garrison { .. }),
        "home again: {:?}",
        job(&world)
    );
}

// T22 (MIL-ATK-4.2, 4.3; Q-1051): an attacker without a meeting field walks
// to the target's flag, the door; there he gets his waiting field from the
// search started on the flag (not free), which never enters a building's
// footprint (MovementBlocked) nor takes a road or flag field.
// spec: MIL-ATK-4.2 (T22), MIL-ATK-4.3
#[test]
fn atk_t22_the_door_is_the_flag_and_no_building_field_is_a_waiting_field() {
    let mut world = standard(2, 1);
    attack(&mut world, 1, false);
    let target = world.target_of(TARGET).unwrap();
    let flag = world.target_flag(target).expect("a flag");
    let soldier = world.attack_records()[0].soldier;
    for _ in 0..3000 {
        world.step();
        let Some(r) = world.attack_records().iter().find(|r| r.soldier == soldier) else {
            break;
        };
        if let Some(field) = r.field {
            let s = world.settlers[soldier].as_ref().unwrap();
            assert_eq!(s.walker.at, flag, "his walk ended on the target's flag");
            assert_ne!(field, flag);
            assert_ne!(field, TARGET);
            let terrain = world.terrain();
            assert!(
                !terrain.has(field, crate::movement::NOT_WALKABLE),
                "{field:?}"
            );
            assert!(world.streets[terrain.index(field)].is_none(), "not a road");
            return;
        }
    }
    panic!("no waiting field");
}
