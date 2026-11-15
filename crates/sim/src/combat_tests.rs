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
    assert_eq!(world.attack_available(P0, TARGET), 3);
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

// MIL-ATK-2.3: the soldier is unlinked from its building, and one random
// number per record is drawn.
#[test]
fn atk_2_3_sending_unlinks_and_draws_once() {
    let mut world = standard(4, 1);
    let before = world.rng.clone();
    attack(&mut world, 2, false);
    let m = world.military_at((40, 20)).unwrap();
    assert_eq!(m.soldiers.len(), 2);
    assert_eq!(world.attack_records().len(), 2);
    // The commands of a step come first: one draw per record.
    let mut expected = before;
    for r in world.attack_records() {
        assert_eq!(u64::from(r.counter), u64::from(expected.draw()) % 2);
    }
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
    assert!(world.settlers[defender].is_none());
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
// spec: FMT-SAV-3.7a (T17), MIL-ATK-6.2, MIL-ATK-6.3
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
    assert!(!world.military[&home].soldiers.contains(&a));
    assert!(!world.military[&home].soldiers.contains(&b));
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
    let alive = |world: &World, id: usize| world.settlers[id].is_some();
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
