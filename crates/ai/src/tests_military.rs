//! Tests of the military part (`AI-military.md` T1 to T12).

use s2_sim::grid::Point;
use s2_sim::{Command, PlayerId, World};

use crate::cells::cell_of;
use crate::config::{Mode, Weights};
use crate::construction::{Order, Outcome, Slot, Tag};
use crate::expansion::{Kind, Source, Target};
use crate::maps::{mcell_of, morigin};
use crate::military::{
    allocation, attack_count, attack_target, building_for, candidate_seen, castle_reserve,
    decide_attack, level, order_candidates, soldiers_wanted, target_value, AttackDecision,
};
use crate::sites::Area;
use crate::testutil::*;
use crate::view::Economy;
use crate::AiSystem;

const P0: PlayerId = PlayerId(0);

fn start_with(world: &World, config: crate::AiConfig, profile: u8) -> AiSystem {
    let mut ai = AiSystem::new(config, world);
    assert!(ai.activate_with(world, &mut s2_sim::Numbers::new(0), P0, profile));
    ai
}

fn with_weights() -> crate::AiConfig {
    let mut config = config();
    config.weights.insert(
        CASTLE,
        Weights {
            score: 300,
            threat: 10,
        },
    );
    config.weights.insert(
        GUARD_HOUSE,
        Weights {
            score: 0,
            threat: 30,
        },
    );
    config.weights.insert(
        CATAPULT,
        Weights {
            score: 300,
            threat: 200,
        },
    );
    config
}

// T1: Normal profile (aggressive), medium 0.3, 5 soldiers in stock, S = 128:
// 5 > 1: f = 1.25, near 1.0, medium 0.375; defense type 1.
// spec: AI-MIL-2.2, AI-MIL-2.3 (T1)
#[test]
fn t1_allocation_grows_with_soldiers_in_stock() {
    let (near, medium) = allocation(0.3, 5, 128).unwrap();
    assert!((near - 1.0).abs() < 1e-6);
    assert!((medium - 0.375).abs() < 1e-3, "{medium}");

    let world = world(&[0]);
    let mut config = config();
    config.profiles[1].mode = Mode::Aggressive;
    config.profiles[1].attack_strength = 0.75;
    let mut ai = start_with(&world, config, 2);
    ai.player_mut(P0).statistic = (1, 128);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_settings(ctx));
    let allocation_of = |class: u8| {
        commands.iter().find_map(|c| match c {
            Command::MilitaryAllocation {
                class: k,
                thousandths,
                ..
            } if *k == class => Some(*thousandths),
            _ => None,
        })
    };
    assert_eq!(allocation_of(0), Some(1000));
    assert_eq!(allocation_of(1), Some(375));
    assert!(commands.contains(&Command::MilitaryAttackStrength {
        player: P0,
        thousandths: 750
    }));
    // Defense type 1 is the default: aggressive has nothing to change, safe
    // sets type 0.
    assert!(commands
        .iter()
        .all(|c| !matches!(c, Command::MilitaryDefenseType { .. })));
    let mut safe = start_with(&world, crate::testutil::config(), 1);
    let (_, commands) = safe.with_ctx(&world, P0, |p, ctx| p.military_settings(ctx));
    assert!(commands.contains(&Command::MilitaryDefenseType {
        player: P0,
        strong: false
    }));
    // The countdown of 50 runs keeps the allocation as it is.
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_settings(ctx));
    assert!(commands
        .iter()
        .all(|c| !matches!(c, Command::MilitaryAllocation { .. })));
    assert_eq!(ai.player_mut(P0).settings_countdown, 49);
}

// AI-MIL T29 (AI-MIL-2.2, the countdown): a change sets the countdown to 50
// and every run first lowers it by 1 and tests only when it is 0 or below,
// so with soldiers in stock above S / 128 at every run the allocation
// changes at runs r, r + 50, r + 100 (each 50 runs after the one before;
// 51 apart put R6 player 1's fourth growth three runs late). A run whose
// test finds no change leaves the countdown at 0 and the next run tests
// again; a countdown saved below 0 (read as a u32) tests at once.
// spec: AI-MIL-2.2 (T29)
#[test]
fn t29_allocation_changes_come_50_runs_apart() {
    let world = world(&[0]);
    let mut ai = start_with(&world, config(), 2);
    let changes = |ai: &mut AiSystem, runs: u32| -> Vec<u32> {
        (0..runs)
            .filter(|_| {
                let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_settings(ctx));
                commands
                    .iter()
                    .any(|c| matches!(c, Command::MilitaryAllocation { .. }))
            })
            .collect()
    };
    // No soldiers above S / 128: no change, every run tests.
    ai.player_mut(P0).statistic = (1, u32::MAX);
    assert!(changes(&mut ai, 3).is_empty());
    assert_eq!(ai.player_mut(P0).settings_countdown, 0);
    // Then they are: at once, and every 50 runs.
    ai.player_mut(P0).statistic = (1, 128);
    assert_eq!(changes(&mut ai, 120), [0, 50, 100]);
    // Saved below 0: -1 read as a u32.
    ai.player_mut(P0).settings_countdown = u32::MAX;
    assert_eq!(changes(&mut ai, 2), [0]);
}

// T2: no soldiers in stock, medium 0.3: near 0.8, medium 0.24.
// spec: AI-MIL-2.2 (T2)
#[test]
fn t2_allocation_shrinks_without_soldiers() {
    let (near, medium) = allocation(0.3, 0, 128).unwrap();
    assert!((near - 0.8).abs() < 1e-6, "{near}");
    assert!((medium - 0.24).abs() < 1e-6, "{medium}");
    // Soldiers in stock, but not above S / 128: no change at all.
    assert_eq!(allocation(0.3, 1, 256), None);
}

// T3 and T4: the building type by enemy distance and resources.
// spec: AI-MIL-4.3 (T3, T4)
#[test]
fn t3_t4_building_type() {
    let config = config();
    let create = |d, underground, stone| level(d, underground, stone, None);
    // T3: enemy at 10, no resources, first attempt: level -1, max(0, -1) = 0.
    assert_eq!(
        building_for(&config, create(10, false, false).max(0)),
        Some(BARRACKS)
    );
    // T4: enemy at 30, coal within 4 squares, stone 10: -2 + 2 = 0, a barracks;
    // after one failure -1: no smaller building.
    let lv = create(30, true, false);
    assert_eq!(lv, 0);
    assert_eq!(building_for(&config, lv.max(0)), Some(BARRACKS));
    assert_eq!(building_for(&config, lv.max(0) - 1), None);
    // T4b: enemy at exactly 24 (read literally), coal, stone 10: 1 + 2 = 3, a
    // fortress; after one failure 2, a tower.
    let lv = create(24, true, false);
    assert_eq!(lv, 3);
    assert_eq!(building_for(&config, lv), Some(FORTRESS));
    assert_eq!(building_for(&config, lv - 1), Some(TOWER));
    // T4c: enemy at 30, no resources, stone 50: -2 + 1 = -1 -> 0, a barracks.
    assert_eq!(
        building_for(&config, create(30, false, true).max(0)),
        Some(BARRACKS)
    );
}

// T5: a guard house at enemy distance 12 with 35 soldiers in stock, an own
// finished building within 4 and stone 50: 2 + 1 + 1 + 1 = 5.
// spec: AI-MIL-4.3, AI-MIL-4.5 (T5)
#[test]
fn t5_upgrade_level() {
    assert_eq!(level(12, false, true, Some((true, true))), 5);
    assert_eq!(level(40, false, false, Some((false, false))), -2);
}

// T6: a headquarters at enemy distance 20 keeps 9 per rank, at 60 none.
// spec: AI-MIL-4.6 (T6)
#[test]
fn t6_castle_reserve() {
    assert_eq!(castle_reserve(20), 9);
    assert_eq!(castle_reserve(60), 0);
    assert_eq!(castle_reserve(49), 0);
    assert_eq!(castle_reserve(0), 15);
}

// spec: AI-MIL-4.6
#[test]
fn castle_settings_follow_the_enemy_distance() {
    let world = world(&[0]);
    let mut ai = start_with(&world, config(), 2);
    // The headquarters starts with 1 per rank; without enemies the reserve is 0.
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.castle_settings(ctx));
    let ranks: Vec<u8> = commands
        .iter()
        .filter_map(|c| match c {
            Command::SetCastleSoldiers {
                rank, amount: 0, ..
            } => Some(*rank),
            _ => None,
        })
        .collect();
    assert_eq!(ranks, [0, 1, 2, 3, 4]);
}

// T7: target cell: enemy score 40, construction threat 0, balance 10, C = 1,
// A = 2: (0 + 80 - 10) / 4 = 17.5.
// spec: AI-MIL-5.2 (T7)
#[test]
fn t7_target_value() {
    assert_eq!(target_value(0.0, 40.0, 10.0, 1.0, 2.0), 17.5);
}

// AI-MIL T31 (AI-MIL-5.2, the gate and the order): a cell is looked at
// only when one of its eight neighbouring cells, not the cell itself,
// holds a nonzero threat of the attacker in its own map; the enemy's threat
// in the cell does not open it. The cells are visited column by column (x
// from 0, every y from 0 for each) and the players 0 to 7 in order, so of
// equal values the first visited stays; a later one replaces it only when
// strictly greater (R6: player 2's runs of 630 .. 7681 find no cell, the
// run of 8322 the cell (4, 2) of player 3 with -10).
// spec: AI-MIL-5.2 (T31)
#[test]
fn t31_attack_targets_lie_next_to_own_threat_column_by_column() {
    use crate::maps::{MilCell, MilitaryMap};
    let (p0, p1, p2) = (PlayerId(0), PlayerId(1), PlayerId(2));
    let alive = [true, true, true];
    let threat = |t| MilCell {
        threat: t,
        ..MilCell::default()
    };
    let score = |s| MilCell {
        threat: 30,
        threat_building: 0,
        score: s,
    };
    let target = |maps: &[MilitaryMap]| {
        attack_target(maps, &alive, p0, |_| true, |_| 0.0, |_| (0.0, 0.0))
            .map(|(v, cell, p)| (v as i64, cell, p))
    };
    let mut maps = vec![MilitaryMap::new(64, 64); 3];
    maps[1].set((3, 3), score(300));
    assert_eq!(target(&maps), None, "the enemy's threat opens nothing");
    maps[0].set((3, 3), threat(30));
    assert_eq!(target(&maps), None, "nor the attacker's in the cell itself");
    maps[0].set((2, 4), threat(30));
    assert_eq!(target(&maps), Some((600, (3, 3), p1)), "beside it");
    // Column by column: (0, 2) before (2, 0), both beside (1, 1).
    let mut maps = vec![MilitaryMap::new(64, 64); 3];
    maps[0].set((1, 1), threat(30));
    maps[1].set((2, 0), score(100));
    maps[1].set((0, 2), score(100));
    assert_eq!(target(&maps), Some((200, (0, 2), p1)));
    // Players in order: player 2's equal value in (0, 2) does not replace.
    maps[2].set((0, 2), score(100));
    assert_eq!(target(&maps), Some((200, (0, 2), p1)));
    // A strictly greater value later does.
    maps[2].set((2, 0), score(150));
    assert_eq!(target(&maps), Some((300, (2, 0), p2)));
    // No value above -100: no target.
    let low = attack_target(&maps, &alive, p0, |_| true, |_| 1000.0, |_| (0.0, 0.0));
    assert_eq!(low, None);
}

// AI-MIL T31 (AI-MIL-5.3, the signed test): an attackable enemy building is
// a candidate when it is explored or its enemy distance is at most 12,
// signed: -1, which an enemy headquarters always holds, passes (R6: player
// 3's headquarters (26, 24), not explored, is player 2's only candidate at
// 8322).
// spec: AI-MIL-5.3 (T31)
#[test]
fn t31_an_enemy_headquarters_in_range_is_always_a_candidate() {
    assert!(candidate_seen(false, Some(-1)));
    assert!(candidate_seen(false, Some(0)));
    assert!(candidate_seen(false, Some(12)));
    assert!(!candidate_seen(false, Some(13)));
    assert!(!candidate_seen(false, None));
    assert!(candidate_seen(true, Some(1000)));
}

// AI-MIL T30 (a) (AI-MIL-5.5, whole counters): C and A are whole numbers
// and halving drops the remainder: a cell with C = 3 and A = 5 whose
// target is gone has C = 1 and A = 2, and 1 halves to 0.
// spec: AI-MIL-5.5 (T30)
#[test]
fn t30_halving_the_counters_drops_the_remainder() {
    let world = world(&[0]);
    let mut ai = start_with(&world, with_weights(), 2);
    let cell = (3, 3);
    let gone = (30, 30);
    assert!(world.building_at(gone).is_none());
    for ((c, a), (c2, a2)) in [((3.0, 5.0), (1.0, 2.0)), ((1.0, 1.0), (0.0, 0.0))] {
        {
            let p = ai.player_mut(P0);
            let t = p.territory.get_mut(cell).unwrap();
            (t.result, t.attempts) = (c, a);
            p.attack.target = Some((gone, cell));
        }
        ai.with_ctx(&world, P0, |p, ctx| p.attack_system(ctx));
        let p = ai.player_mut(P0);
        let t = p.territory.get(cell);
        assert_eq!((t.result, t.attempts), (c2, a2));
        assert_eq!(p.attack.target, None);
    }
}

// AI-MIL T30 (b) to (d) (AI-MIL-5.5, the last candidate): candidates g1
// (want 6), g2 (want 2), g3 (want 7), 4 soldiers in stock, 1 available
// against each, C = 0. (b) Safe: g1 and g3 are skipped (want above the
// stock), g2 reaches the need test (need 2 > 1) and is not attacked: the
// last candidate, the expansion's position, is g2, not g3. (c) Normal:
// all three are tested, none attacked: g3. (d) No candidate, or none
// that reached the test: no last candidate (A still rises by 1).
// spec: AI-MIL-5.5 (T30), AI-MIL-5.4
#[test]
fn t30_the_last_candidate_is_the_last_one_tested() {
    let (g1, g2, g3) = ((10, 10), (12, 10), (14, 10));
    let list = [(g1, 6, 1), (g2, 2, 1), (g3, 7, 1)];
    assert_eq!(
        decide_attack(Mode::Safe, &list, 4, 0.0),
        AttackDecision::NotAttacked { last: Some(g2) }
    );
    assert_eq!(
        decide_attack(Mode::Normal, &list, 4, 0.0),
        AttackDecision::NotAttacked { last: Some(g3) }
    );
    assert_eq!(
        decide_attack(Mode::Normal, &[], 4, 0.0),
        AttackDecision::NotAttacked { last: None }
    );
    assert_eq!(
        decide_attack(Mode::Safe, &[(g1, 6, 1), (g3, 7, 1)], 4, 0.0),
        AttackDecision::NotAttacked { last: None }
    );
    // With 2 available against g2 it is attacked in Safe mode, min(need,
    // available) soldiers; in Normal min(2 need, available).
    let list = [(g1, 6, 1), (g2, 2, 2), (g3, 7, 1)];
    assert_eq!(
        decide_attack(Mode::Safe, &list, 4, 0.0),
        AttackDecision::Attack { at: g2, count: 2 }
    );
    assert_eq!(
        decide_attack(Mode::Normal, &list, 4, 0.0),
        AttackDecision::Attack { at: g2, count: 2 }
    );
}

// T8 to T10: how many soldiers an attack sends.
// spec: AI-MIL-5.4 (T8 to T10)
#[test]
fn t8_to_t10_attack_counts() {
    // T8: Safe, a guard house with 2 soldiers, r = 1, defend outside 1.0, 4
    // soldiers in stock, 5 available: want = trunc(1 x 2 x 1.5 x 1.5) = 4.
    let want = soldiers_wanted(1.0, 2, 1.0);
    assert_eq!(want, 4);
    assert!(want <= 4);
    assert_eq!(attack_count(Mode::Safe, want as f32, 5), Some(4));
    // Safe does not go when it has fewer than needed available.
    assert_eq!(attack_count(Mode::Safe, 6.0, 5), None);
    // T9: Aggressive attacks with all it has even with fewer than needed.
    assert_eq!(attack_count(Mode::Aggressive, 4.0, 2), Some(2));
    assert_eq!(attack_count(Mode::Aggressive, 4.0, 0), None);
    // T10: Normal sends twice the need.
    assert_eq!(attack_count(Mode::Normal, 3.0, 10), Some(6));
    assert_eq!(attack_count(Mode::Normal, 8.0, 10), Some(10));
    assert_eq!(attack_count(Mode::Normal, 11.0, 10), None);
    // The strength ratio is capped at 5.
    assert_eq!(soldiers_wanted(9.0, 1, 0.0), 7);
}

/// Smithies and their inputs.
fn weapons_world() -> (World, AiSystem) {
    let mut world = world(&[0]);
    with_building(&mut world, SMITHY, (24, 22));
    let mut config = config();
    config.profiles[1].max_smithies = 2;
    let mut ai = start_with(&world, config, 2);
    let snapshot = finished_snapshot(&world, (24, 22), 5);
    ai.update_with(&world, P0, &snapshot);
    ai.player_mut(P0).need_stage = 4;
    // The smithy's own input needs (AI-CON-2.5) are dropped: these tests
    // look at the needs of AI-MIL-6 alone.
    ai.player_mut(P0).plan.needs.clear();
    (world, ai)
}

// T11: a smithy at productivity 0.05: needs attached to the smithy for the
// inputs its own input store lacks, no sword need. AI-MIL T24: its store,
// not the player's stock: coal in stock does not keep the coal need away.
// spec: AI-MIL-6.2 (T11, AI-MIL T24)
#[test]
fn t11_idle_smithy_misses_its_inputs() {
    let (world, mut ai) = weapons_world();
    ai.player_mut(P0).econ = Economy::from_counts(&[(COAL, 10)], &[]);
    let snapshot = finished_snapshot(&world, (24, 22), 5);
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    let needs: Vec<(u32, Option<(u32, u32)>)> = ai
        .player_mut(P0)
        .plan
        .needs
        .iter()
        .map(|n| (n.good, n.consumer))
        .collect();
    assert!(world.inputs((24, 22)).is_empty());
    assert_eq!(needs.len(), 2, "{needs:?}");
    assert!(needs.contains(&(IRON, Some((24, 22)))));
    assert!(needs.contains(&(COAL, Some((24, 22)))));
}

// T12: 1 smithy at 0.95, max smithies 2, S = 960, 5 soldiers in stock and
// 600 s passed: a sword need.
// spec: AI-MIL-6.2 (T12)
#[test]
fn t12_busy_smithy_asks_for_another() {
    let (mut world, mut ai) = weapons_world();
    for _ in 0..6100 {
        world.step();
    }
    ai.player_mut(P0).statistic = (world.tick(), 960);
    let snapshot = finished_snapshot(&world, (24, 22), 95);
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    let needs = &ai.player_mut(P0).plan.needs;
    assert_eq!(needs.iter().filter(|n| n.good == SWORD).count(), 1);
    // Not again at once: the last need is younger than 600 s x k.
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    assert_eq!(
        ai.player_mut(P0)
            .plan
            .needs
            .iter()
            .filter(|n| n.good == SWORD)
            .count(),
        1
    );
}

// AI-MIL T20 (AI-MIL-6.2, Q-283), T27: a smithy one of whose tied needs has
// no production (its iron need, no smelter planned) is not ready and ends
// the check: no sword need although it is busy and 600 s have passed; with
// a production for that need only ordered, not placed (T27), still not;
// with the producer's site placed it is ready, and the pass makes the need.
// spec: AI-MIL-6.2 (AI-MIL T20, AI-MIL T27)
#[test]
fn t20_a_smithy_whose_need_has_no_production_stops_the_check() {
    let (mut world, mut ai) = weapons_world();
    for _ in 0..6100 {
        world.step();
    }
    ai.player_mut(P0).statistic = (world.tick(), 960);
    let iron = ai.player_mut(P0).plan.add_need(IRON, Some((24, 22)), 0.0);
    let snapshot = finished_snapshot(&world, (24, 22), 95);
    let swords = |ai: &mut AiSystem| {
        ai.player_mut(P0)
            .plan
            .needs
            .iter()
            .filter(|n| n.good == SWORD)
            .count()
    };
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    assert_eq!(swords(&mut ai), 0, "not ready");
    let plan = &mut ai.player_mut(P0).plan;
    let cell = plan.need_mut(iron).unwrap().cell;
    let production = plan.add_production(IRON, cell.unwrap_or((1, 1)), Some(iron));
    plan.need_mut(iron).unwrap().production = Some(production);
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    assert_eq!(swords(&mut ai), 0, "only ordered: not ready");
    let plan = &mut ai.player_mut(P0).plan;
    plan.prods
        .iter_mut()
        .find(|p| p.id == production)
        .unwrap()
        .building = Some((30, 30));
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    assert_eq!(swords(&mut ai), 1, "ready");
}

// AI-CON T87 (AI-CON-10.3): an ore's mines get the food weight 1.0 when some
// need of the ore has a consumer that is ready (its tied needs' productions
// placed, finished, worker in), takes the ore as an input and holds none of
// it. The smithy makes swords from iron and coal: iron needed, none in its
// store: starved; gold is no input of it; a tied need only ordered: not
// ready.
// spec: AI-CON-10.3 (AI-CON T87), AI-MIL-6.2
#[test]
fn t87_an_ore_is_starved_at_a_ready_consumer_without_it() {
    let (world, mut ai) = smithy_with_smith();
    let smithy = (24, 22);
    let snapshot = finished_snapshot(&world, smithy, 50);
    let p = ai.player_mut(P0);
    let iron = p.plan.add_need(IRON, Some(smithy), 0.0);
    let gold = p.plan.add_need(GOLD, Some(smithy), 0.0);
    let tied: Vec<_> = p
        .plan
        .needs
        .iter()
        .filter(|n| n.consumer == Some(smithy))
        .map(|n| n.id)
        .collect();
    for (i, need) in tied.iter().enumerate() {
        let prod = p.plan.add_production(IRON, (1, 1), Some(*need));
        p.plan
            .prods
            .iter_mut()
            .find(|x| x.id == prod)
            .unwrap()
            .building = Some((30, 30 + i as u32));
        p.plan.need_mut(*need).unwrap().production = Some(prod);
    }
    let starved = |ai: &mut AiSystem, ore| {
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.ore_starved(ctx, ore))
            .0
    };
    assert!(starved(&mut ai, IRON));
    assert!(!starved(&mut ai, GOLD), "no input of a smithy");
    let _ = gold;
    // One tied need's production only ordered: not ready.
    let p = ai.player_mut(P0);
    let prod = p.plan.need(iron).unwrap().production.unwrap();
    p.plan
        .prods
        .iter_mut()
        .find(|x| x.id == prod)
        .unwrap()
        .building = None;
    assert!(!starved(&mut ai, IRON));
}

/// The smithy on (24, 22) on the town rules, finished, with a smith (the
/// sawyer's profession here, out of the headquarters' stock) on his way to
/// it; the AI as [`weapons_world`] sets it up.
fn smithy_on_its_way() -> (World, AiSystem) {
    let mut rules = crate::fixture::town_rules();
    for t in &mut rules.building_types {
        if t.id == SMITHY {
            t.worker = Some(crate::fixture::SAWYER);
        }
    }
    rules.start_stock.push((crate::fixture::SAWYER, 1));
    let mut world = crate::fixture::world_on(1, &[0], &Extras::default(), rules);
    with_building(&mut world, SMITHY, (24, 22));
    let hq_flag = crate::view::flag_of(&world, (20, 20)).unwrap();
    let flag = crate::view::flag_of(&world, (24, 22)).unwrap();
    world.submit(Command::BuildRoad {
        player: P0,
        from: hq_flag,
        to: flag,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()), "the road");
    finish_site(&mut world, (24, 22));
    let mut ai = start_with(&world, config(), 2);
    let snapshot = finished_snapshot(&world, (24, 22), 5);
    ai.update_with(&world, P0, &snapshot);
    ai.player_mut(P0).need_stage = 4;
    ai.player_mut(P0).plan.needs.clear();
    (world, ai)
}

/// AI-CON-10.3 (T98): whether the smithy's worker, on his way to it or its
/// worker, stands, not walking, on its field; `None` before he is sent.
fn smith_in(world: &World, smithy: Point) -> Option<bool> {
    world.settlers().find_map(|(_, s)| match s.job {
        s2_sim::SettlerJob::Worker { target, .. } | s2_sim::SettlerJob::ToBuilding { target }
            if target.at == smithy =>
        {
            Some(
                matches!(s.job, s2_sim::SettlerJob::Worker { .. })
                    && s.walker.at == smithy
                    && !s.walker.is_walking(),
            )
        }
        _ => None,
    })
}

/// [`smithy_on_its_way`] played on until the smith stands in the smithy.
fn smithy_with_smith() -> (World, AiSystem) {
    let (mut world, ai) = smithy_on_its_way();
    while smith_in(&world, (24, 22)) != Some(true) {
        world.step();
        assert!(world.tick() < 8000, "the smith comes");
    }
    (world, ai)
}

// AI-CON T98 (AI-CON-10.3, Q-383): "its worker in it" is a worker of the
// building standing, not walking, on its own field at the moment of the
// run: while the smith walks to the smithy the iron is not starved there;
// once he stands in it, it is.
// spec: AI-CON-10.3 (AI-CON T98, T87)
#[test]
fn t98_the_worker_must_stand_in_the_building() {
    let (mut world, mut ai) = smithy_on_its_way();
    let smithy = (24, 22);
    let p = ai.player_mut(P0);
    let iron = p.plan.add_need(IRON, Some(smithy), 0.0);
    let prod = p.plan.add_production(IRON, (1, 1), Some(iron));
    p.plan
        .prods
        .iter_mut()
        .find(|x| x.id == prod)
        .unwrap()
        .building = Some((30, 30));
    p.plan.need_mut(iron).unwrap().production = Some(prod);
    let mut seen_walking = false;
    loop {
        let snapshot = finished_snapshot(&world, smithy, 50);
        let starved = ai
            .with_snapshot(&world, P0, &snapshot, |p, ctx| p.ore_starved(ctx, IRON))
            .0;
        match smith_in(&world, smithy) {
            Some(true) => {
                assert!(starved, "he stands in it");
                break;
            }
            Some(false) => {
                seen_walking = true;
                assert!(!starved, "he walks: passed over");
            }
            None => assert!(!starved, "no smith yet"),
        }
        world.step();
        assert!(world.tick() < 8000, "the smith comes");
    }
    assert!(seen_walking);
}

// AI-MIL-6.2: no sword need during need stage 1, and none at the smithy limit.
#[test]
fn weapons_wait_for_stage_and_respect_the_limit() {
    let (world, mut ai) = weapons_world();
    ai.player_mut(P0).need_stage = 1;
    let snapshot = finished_snapshot(&world, (24, 22), 95);
    ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.weapon_production(ctx));
    assert!(ai.player_mut(P0).plan.needs.is_empty());
}

// AI-MIL-3.1 (corrected 2026-10-26 night, T17): the oldest pending target
// is taken and becomes the current target one run later; its source's slot
// keeps it (until it is finished or cancelled), and nothing else is taken
// while it is in progress.
// spec: AI-MIL-3.1 (T17)
#[test]
fn expansion_targets_are_taken_oldest_first() {
    let world = world(&[0]);
    let mut ai = start_with(&world, config(), 2);
    {
        let e = &mut ai.player_mut(P0).expansion;
        e.set(Source::Crowded, Kind::Crowded, (5, 5), 300);
        e.set(Source::Attack, Kind::Position, (9, 9), 100);
    }
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_targets(ctx));
    {
        let e = &ai.player_mut(P0).expansion;
        assert_eq!(e.taken.map(|t| t.0), Some(Source::Attack));
        assert!(e.current.is_none());
    }
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_targets(ctx));
    let e = &ai.player_mut(P0).expansion;
    assert_eq!(e.current.map(|t| t.0), Some(Source::Attack));
    assert!(
        e.pending[Source::Attack as usize].is_some(),
        "the slot keeps it"
    );
    assert!(e.pending[Source::Crowded as usize].is_some());
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_targets(ctx));
    let e = &ai.player_mut(P0).expansion;
    assert!(e.taken.is_none(), "nothing taken while one is in progress");
}

// AI-MIL-3.1: on equal times the sources go in the order waiting need,
// crowded cell, anywhere, attack.
// spec: AI-MIL-3.1
#[test]
fn equal_targets_go_by_source_order() {
    let world = world(&[0]);
    let mut ai = start_with(&world, config(), 2);
    {
        let e = &mut ai.player_mut(P0).expansion;
        e.set(Source::Attack, Kind::Position, (9, 9), 100);
        e.set(Source::Anywhere, Kind::Anywhere, (7, 7), 100);
        e.set(Source::Crowded, Kind::Crowded, (5, 5), 100);
    }
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_targets(ctx));
    let e = &ai.player_mut(P0).expansion;
    assert_eq!(e.taken.map(|t| t.0), Some(Source::Crowded));
}

// AI-MIL-4.1 to 4.3: a position target gives an order for a building at the
// best cell with the radius 12; no enemy in sight and no resources: a
// barracks (level -2).
#[test]
fn a_position_target_gives_a_guard_house_order() {
    let world = world(&[0]);
    let mut ai = start_with(&world, with_weights(), 2);
    ai.player_mut(P0).expansion.current = Some((
        Source::WaitingNeed,
        Target {
            kind: Kind::Position,
            at: (40, 40),
            since: 0,
        },
    ));
    ai.with_ctx(&world, P0, |p, ctx| p.military_building(ctx));
    let order = ai
        .player_mut(P0)
        .build
        .slot(Slot::Military)
        .copied()
        .unwrap();
    assert_eq!((order.building, order.weight), (BARRACKS, 20));
    // AI-MIL-4.2: the cells next to the one with the headquarters qualify
    // (a neighbour holds the player's threat); the one nearest to the target
    // wins.
    assert_eq!(
        order.area,
        Area::Point {
            at: (24, 24),
            radius: 12
        }
    );
    // While the order is pending, nothing more.
    ai.player_mut(P0).build.slots[Slot::Military as usize] = None;
    ai.player_mut(P0).military_cooldown = 3;
    ai.with_ctx(&world, P0, |p, ctx| p.military_building(ctx));
    assert!(ai.player_mut(P0).build.slot(Slot::Military).is_none());
    assert_eq!(ai.player_mut(P0).military_cooldown, 2);
}

// AI-MIL-4.1: a crowded-cell target uses the cell mode.
#[test]
fn a_crowded_target_orders_in_the_cell() {
    let world = world(&[0]);
    let mut ai = start_with(&world, with_weights(), 2);
    let home = ai.player_mut(P0).home;
    ai.player_mut(P0).expansion.current = Some((
        Source::Crowded,
        Target {
            kind: Kind::Crowded,
            at: crate::cells::origin(cell_of(home)),
            since: 0,
        },
    ));
    ai.with_ctx(&world, P0, |p, ctx| p.military_building(ctx));
    let order = ai
        .player_mut(P0)
        .build
        .slot(Slot::Military)
        .copied()
        .unwrap();
    assert_eq!(order.area, Area::Cell(cell_of(home)));
}

// AI-MIL-4.4, 3.2 (corrected 2026-10-26 night, T17, and 2026-10-27 night,
// T18): a failure for no site or no material orders a smaller building;
// below level 0 after no site the target is cancelled at once (its slot
// cleared, the coordinate's attempt counter raised by 1) and the cell's
// penalty rises, after no material it stays current and remembered and the
// next military run finishes it; a placement halves the penalty and starts
// the cooldown and leaves the target remembered: the first military run
// past the cooldown finishes it (slot and current target cleared, the
// counter halved) and makes no order.
// spec: AI-MIL-4.4, AI-MIL-3.2 (T17, T18), AI-MIL-4.1, AI-CON-3.5
#[test]
fn order_outcomes_update_penalty_cooldown_and_target() {
    let world = world(&[0]);
    let mut ai = start_with(&world, with_weights(), 2);
    let cell = mcell_of(ai.player_mut(P0).home);
    let target = Target {
        kind: Kind::Position,
        at: (40, 40),
        since: 0,
    };
    ai.player_mut(P0).expansion.current = Some((Source::WaitingNeed, target));
    ai.player_mut(P0).expansion.pending[Source::WaitingNeed as usize] = Some(target);
    let coordinate = crate::cells::cell_of(target.at);
    // AI-MIL-4.4 (T28): the level is worked out at the order's place at each
    // failure: no enemy (-2), coal within 4 squares (+2), 50 stones (+1): 1.
    level_one_at(&mut ai, morigin(cell));
    let order = |building, attempts| Order {
        slot: Slot::Military,
        area: Area::Point {
            at: morigin(cell),
            radius: 12,
        },
        building,
        weight: 20,
        time: 0.0,
        tag: Tag::Military {
            attempts,
            level: 3,
            cell,
            source: Source::WaitingNeed,
        },
    };
    // Fortress found no site: a smaller building, a barracks (1 - 1), is
    // ordered.
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order(FORTRESS, 0), Outcome::NoSite, ctx)
    });
    let next = ai
        .player_mut(P0)
        .build
        .slot(Slot::Military)
        .copied()
        .unwrap();
    assert_eq!(next.building, BARRACKS);
    assert!(matches!(next.tag, Tag::Military { attempts: 1, .. }));
    // Worn down to a barracks, and that fails too: the target is given up.
    ai.player_mut(P0).build.slots[Slot::Military as usize] = None;
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order(BARRACKS, 3), Outcome::NoSite, ctx)
    });
    assert!(ai.player_mut(P0).build.slot(Slot::Military).is_none());
    assert!(ai.player_mut(P0).expansion.current.is_none());
    assert!(ai.player_mut(P0).expansion.pending[Source::WaitingNeed as usize].is_none());
    assert_eq!(ai.player_mut(P0).attempts.get(&coordinate), Some(&1));
    assert_eq!(ai.player_mut(P0).territory.get(cell).penalty, 1.0);
    // "No material" re-orders like no site: a tower after the fortress.
    ai.player_mut(P0).expansion.current = Some((Source::WaitingNeed, target));
    ai.player_mut(P0).expansion.pending[Source::WaitingNeed as usize] = Some(target);
    ai.player_mut(P0).attempts.insert(coordinate, 3);
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order(FORTRESS, 0), Outcome::NoMaterial, ctx)
    });
    let next = ai
        .player_mut(P0)
        .build
        .slot(Slot::Military)
        .copied()
        .unwrap();
    assert_eq!(next.building, BARRACKS);
    assert!(matches!(next.tag, Tag::Military { attempts: 1, .. }));
    // Below 0 after no material: no order, no penalty, the target stays
    // current and remembered; the next run finishes it and makes no order.
    ai.player_mut(P0).build.slots[Slot::Military as usize] = None;
    ai.player_mut(P0).expansion.remembered = true;
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order(BARRACKS, 3), Outcome::NoMaterial, ctx)
    });
    assert!(ai.player_mut(P0).build.slot(Slot::Military).is_none());
    assert!(ai.player_mut(P0).expansion.current.is_some());
    assert!(ai.player_mut(P0).expansion.remembered);
    assert_eq!(ai.player_mut(P0).territory.get(cell).penalty, 1.0);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_building(ctx));
    assert!(ai.player_mut(P0).expansion.current.is_none());
    assert_eq!(
        ai.player_mut(P0).attempts.get(&coordinate),
        Some(&1),
        "3 halved"
    );
    assert!(commands.is_empty() && ai.player_mut(P0).build.slot(Slot::Military).is_none());
    // Success: the penalty halves, the cooldown is 3 runs, the target done.
    ai.player_mut(P0).expansion.current = Some((Source::WaitingNeed, target));
    ai.player_mut(P0).expansion.pending[Source::WaitingNeed as usize] = Some(target);
    ai.player_mut(P0).attempts.insert(coordinate, 3);
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order(GUARD_HOUSE, 0), Outcome::Placed((30, 30)), ctx)
    });
    assert_eq!(ai.player_mut(P0).territory.get(cell).penalty, 0.5);
    assert_eq!(ai.player_mut(P0).military_cooldown, 3);
    assert!(
        ai.player_mut(P0).expansion.current.is_some(),
        "only remembered"
    );
    // Three cooldown runs count down; the fourth finishes the target.
    for _ in 0..3 {
        ai.with_ctx(&world, P0, |p, ctx| p.military_building(ctx));
        assert!(ai.player_mut(P0).expansion.current.is_some());
    }
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_building(ctx));
    let p = ai.player_mut(P0);
    assert!(p.expansion.current.is_none());
    assert!(p.expansion.pending[Source::WaitingNeed as usize].is_none());
    assert_eq!(p.attempts.get(&coordinate), Some(&1), "3 halved");
    assert!(p.build.slot(Slot::Military).is_none() && commands.is_empty());
}

/// AI-MIL-4.3 at `site` for player 0 of a one-player world: no enemy (-2),
/// coal within 4 squares (+2) and 50 stones in stock (+1): level 1.
fn level_one_at(ai: &mut AiSystem, site: Point) {
    let p = ai.player_mut(P0);
    p.econ = Economy::from_counts(&[(STONE, 50)], &[]);
    p.resources.set_entries(
        crate::resources::square_of(site),
        vec![crate::resources::Entry {
            good: COAL,
            kind: crate::resources::UNDERGROUND,
            landmass: 1,
            amount: 5,
        }],
    );
}

// AI-MIL T28 (AI-MIL-4.4, the level anew): at every failure the level of
// AI-MIL-4.3 is worked out again at the order's place (the point; the
// cell's origin in cell mode) as things stand at that moment, and the new
// attempt count is taken off that level, not off the level the order was
// made with. An order made at level 4 (a fortress) whose place now gives
// no enemy (-2), coal within 4 squares (+2) and 50 stones (+1) = 1: its
// first failure orders 1 - 1 = 0, a barracks (the kept level would give
// 4 - 1 = 3, a fortress again). With the stones down to 10 the level is 0,
// so the barracks' failure for no material, 0 - 2 < 0, orders nothing and
// leaves the target current and remembered without a penalty (R6: player
// 1's chain of 3726 ends at 4495, no guard house at (57, 61)). In cell
// mode the cell's origin is the place: a cell with the coal at its origin
// re-orders a barracks after its fortress failed.
// spec: AI-MIL-4.4 (T28), AI-MIL-4.3
#[test]
fn t28_the_level_is_worked_out_again_at_each_failure() {
    let world = world(&[0]);
    let mut ai = start_with(&world, with_weights(), 2);
    let cell = mcell_of(ai.player_mut(P0).home);
    let site = morigin(cell);
    let target = Target {
        kind: Kind::Position,
        at: (40, 40),
        since: 0,
    };
    {
        let p = ai.player_mut(P0);
        p.expansion.current = Some((Source::WaitingNeed, target));
        p.expansion.pending[Source::WaitingNeed as usize] = Some(target);
        p.expansion.remembered = true;
    }
    level_one_at(&mut ai, site);
    let order = Order {
        slot: Slot::Military,
        area: Area::Point {
            at: site,
            radius: 12,
        },
        building: FORTRESS,
        weight: 20,
        time: 0.0,
        tag: Tag::Military {
            attempts: 0,
            level: 4,
            cell,
            source: Source::WaitingNeed,
        },
    };
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order, Outcome::NoSite, ctx)
    });
    let next = ai
        .player_mut(P0)
        .build
        .slot(Slot::Military)
        .copied()
        .unwrap();
    assert_eq!(next.building, BARRACKS, "1 - 1, not 4 - 1");
    assert!(
        matches!(
            next.tag,
            Tag::Military {
                attempts: 1,
                level: 1,
                ..
            }
        ),
        "{:?}",
        next.tag
    );
    // The stones used up: level 0 at the next failure, 0 - 2 < 0.
    ai.player_mut(P0).build.slots[Slot::Military as usize] = None;
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 10)], &[]);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(next, Outcome::NoMaterial, ctx)
    });
    let p = ai.player_mut(P0);
    assert!(p.build.slot(Slot::Military).is_none() && commands.is_empty());
    assert_eq!(p.expansion.current, Some((Source::WaitingNeed, target)));
    assert!(p.expansion.remembered);
    assert_eq!(p.territory.get(cell).penalty, 0.0);
    // Cell mode: the place is the cell's origin.
    let home_cell = cell_of(ai.player_mut(P0).home);
    level_one_at(&mut ai, crate::cells::origin(home_cell));
    let in_cell = Order {
        area: Area::Cell(home_cell),
        ..order
    };
    ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(in_cell, Outcome::NoSite, ctx)
    });
    let next = ai
        .player_mut(P0)
        .build
        .slot(Slot::Military)
        .copied()
        .unwrap();
    assert_eq!(next.building, BARRACKS);
}

// AI-MIL T25 (AI-MIL-1.1): a building type without bp_setAiScore scores 20,
// the type's default; its threat stays 0.
// spec: AI-MIL-1.1 (AI-MIL T25)
#[test]
fn t25_a_type_without_a_score_scores_20() {
    let w = config().weights_of(WOODCUTTER);
    assert_eq!((w.score, w.threat), (20, 0));
}

// AI-MIL T26 (AI-MIL-1.2): the threat balance weighs the cell one column and
// one row further, (x + 1, y + 1), at 1, the centre and the seven others at
// 0.5: the player's own castle (threat 10) counts -10 seen from the cell up
// and left of it, -5 from its own cell.
// spec: AI-MIL-1.2 (AI-MIL T26)
#[test]
fn t26_the_threat_balance_weighs_x_plus_1_y_plus_1_at_1() {
    let world = world(&[0]);
    let config = with_weights();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let mut maps: Vec<crate::maps::MilitaryMap> = (0..crate::snapshot::SLOTS)
        .map(|_| crate::maps::MilitaryMap::new(96, 96))
        .collect();
    maps[0].refresh_all(&world, snapshot.of(P0), &config);
    let castle = mcell_of((20, 20));
    let mut alive = [false; crate::snapshot::SLOTS];
    alive[0] = true;
    let balance = |at| crate::maps::threat_balance(&world, &maps, &alive, P0, at);
    assert_eq!(balance(castle), -5.0);
    assert_eq!(balance((castle.0 - 1, castle.1 - 1)), -10.0);
    assert_eq!(balance((castle.0 + 1, castle.1 + 1)), -5.0);
}

// AI-MIL T25 (AI-MIL-6.4): while a catapult order exists the scan does not
// run: its cursor stands still.
// spec: AI-MIL-6.4 (AI-MIL T25)
#[test]
fn t25_no_catapult_scan_while_its_order_exists() {
    let world = world(&[0]);
    let mut config = with_weights();
    config.profiles[1].catapults = true;
    let mut ai = start_with(&world, config, 2);
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 30)], &[]);
    let cell = cell_of(ai.player_mut(P0).home);
    ai.player_mut(P0).plan.add_production(STONE, cell, None);
    for _ in 0..150 {
        ai.with_ctx(&world, P0, |p, ctx| p.catapult_construction(ctx));
    }
    assert!(ai.player_mut(P0).build.slot(Slot::Catapult).is_some());
    let cursor = ai.player_mut(P0).catapult_scan.cursor;
    for _ in 0..10 {
        ai.with_ctx(&world, P0, |p, ctx| p.catapult_construction(ctx));
    }
    assert_eq!(ai.player_mut(P0).catapult_scan.cursor, cursor);
}

// AI-MIL-6.4: a cell with enough score and no catapult gets a catapult order
// after a full pass over the grid.
#[test]
fn a_valuable_cell_gets_a_catapult() {
    let world = world(&[0]);
    let mut config = with_weights();
    config.profiles[1].catapults = true;
    let mut ai = start_with(&world, config, 2);
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 30)], &[]);
    let cell = cell_of(ai.player_mut(P0).home);
    ai.player_mut(P0).plan.add_production(STONE, cell, None);
    // 12 x 12 cells, one per run (max(1, 144 / 600)).
    for _ in 0..150 {
        ai.with_ctx(&world, P0, |p, ctx| p.catapult_construction(ctx));
    }
    let order = ai
        .player_mut(P0)
        .build
        .slot(Slot::Catapult)
        .copied()
        .unwrap();
    assert_eq!((order.building, order.weight), (CATAPULT, 20));
    let home = mcell_of(ai.player_mut(P0).home);
    assert_eq!(
        order.area,
        Area::Point {
            at: morigin(home),
            radius: 8
        }
    );
    // The weak profile builds none.
    let mut weak = start_with(&world, with_weights(), 1);
    weak.player_mut(P0).econ = Economy::from_counts(&[(STONE, 30)], &[]);
    weak.player_mut(P0).plan.add_production(STONE, cell, None);
    for _ in 0..150 {
        weak.with_ctx(&world, P0, |p, ctx| p.catapult_construction(ctx));
    }
    assert!(weak.player_mut(P0).build.slot(Slot::Catapult).is_none());
}

// AI-MIL-4.7: with coin arrangement on, a finished military building far from
// enemies has coins switched off and the move-out flag on.
#[test]
fn coin_arrangement_follows_the_enemy_distance() {
    let mut world = world(&[0]);
    with_building(&mut world, BARRACKS, (24, 22));
    finish_building(&mut world, (24, 22), vec![(STONE, 2)]);
    // Let the military update run so that the enemy distance is known.
    for _ in 0..50 {
        world.step();
    }
    let mut config = config();
    config.profiles[2].coin_arrangement = true;
    let mut ai = start_with(&world, config, 3);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.coin_arrangement(ctx));
    assert!(commands.contains(&Command::ToggleCoinSupply {
        player: P0,
        at: (24, 22)
    }));
    assert!(commands.contains(&Command::ToggleMoveOut {
        player: P0,
        at: (24, 22)
    }));
    // The profile without coin arrangement does nothing.
    let mut plain = start_with(&world, crate::testutil::config(), 2);
    let (_, commands) = plain.with_ctx(&world, P0, |p, ctx| p.coin_arrangement(ctx));
    assert!(commands.is_empty());
}

// T14: candidates with surrounding scores 30, 50, 50, 10: visited 50, 30, 10;
// the second 50 is dropped.
// spec: AI-MIL-5.3 (T14)
#[test]
fn t14_candidates_are_ordered_highest_first_without_equals() {
    let list = vec![
        (30.0, (1, 1)),
        (50.0, (2, 2)),
        (50.0, (3, 3)),
        (10.0, (4, 4)),
    ];
    let ordered = order_candidates(list, |p| *p);
    let scores: Vec<f32> = ordered.iter().map(|c| c.0).collect();
    assert_eq!(scores, [50.0, 30.0, 10.0]);
    // Of the equal ones the first by position stays.
    assert_eq!(ordered[0].1, (2, 2));
}

// MIL-UPG-3.2, AI-MIL-1.1: an upgrading building counts in "threat", in
// "threat under construction" and in "score".
#[test]
fn an_upgrading_building_counts_as_under_construction() {
    let mut world = world(&[0]);
    let mut config = with_weights();
    config.weights.insert(
        BARRACKS,
        Weights {
            score: 7,
            threat: 3,
        },
    );
    // Four fields from the castle: no site a player could place there
    // (ECO-PLACE-6.2), so a mission's finished building (CAM-1.3).
    let at = (23, 22);
    created(&mut world, BARRACKS, at);
    // MIL-UPG-1.2: its builder released (the end of his last action).
    for _ in 0..60 {
        world.step();
    }
    let ai = start_with(&world, config.clone(), 2);
    let cell = mcell_of(at);
    let before = ai.military[usize::from(P0.0)].get(cell);
    // The headquarters (10, 300) is in the same cell.
    assert_eq!(
        (before.threat_building, before.threat, before.score),
        (0, 13, 307)
    );
    assert_eq!(world.upgrade_target(P0, at), Ok(GUARD_HOUSE));
    world.submit(Command::UpgradeBuilding { player: P0, at });
    world.step();
    let ai = start_with(&world, config, 2);
    let after = ai.military[usize::from(P0.0)].get(cell);
    assert_eq!(
        (after.threat_building, after.threat, after.score),
        (3, 13, 307)
    );
}

/// A world with a finished barracks next to the headquarters and coal in the
/// ground near it, so that the upgrade level is 1 (-2 + 2 + 1).
fn upgrade_game() -> (World, AiSystem, Point) {
    upgrade_game_of(&[0])
}

/// A finished building of player 0 made as a mission makes one
/// (`CreateBuilding`, CAM-1.3), where ECO-PLACE-6.2 keeps a player's site
/// away.
fn created(world: &mut World, kind: u32, at: Point) {
    world.submit(Command::CreateBuilding {
        player: P0,
        building: kind,
        at,
        settler: None,
    });
    world.step();
    assert!(world.building_at(at).is_some_and(|b| !b.under_construction));
}

/// `upgrade_game` with the headquarters of `players` (player 0's barracks).
fn upgrade_game_of(players: &[u8]) -> (World, AiSystem, Point) {
    let mut world = world(players);
    // Four fields from the castle: no site a player could place there
    // (ECO-PLACE-6.2), so a mission's finished building (CAM-1.3).
    let at = (23, 22);
    created(&mut world, BARRACKS, at);
    // MIL-UPG-1.2: its builder released (the end of his last action).
    for _ in 0..60 {
        world.step();
    }
    let mut ai = start_with(&world, config(), 2);
    ai.player_mut(P0).resources.set_entries(
        crate::resources::square_of(at),
        vec![crate::resources::Entry {
            good: COAL,
            kind: crate::resources::UNDERGROUND,
            landmass: 1,
            amount: 5,
        }],
    );
    (world, ai, at)
}

// MIL-UPG T4, AI-MIL-4.5 (corrected 2026-10-27 night, AI-MIL T19): no order
// until twice the target type's costs are in stock: the guard house costs
// 2 stones, so 3 are not enough and 4 are.
// spec: AI-MIL-4.5 (AI-MIL T19), MIL-UPG-1.4 (MIL-UPG T4)
#[test]
fn t4_upgrade_waits_for_the_full_costs() {
    let (world, mut ai, at) = upgrade_game();
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 3)], &[]);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_upgrade(ctx));
    assert!(commands.is_empty(), "{commands:?}");
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 4)], &[]);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_upgrade(ctx));
    assert_eq!(commands, [Command::UpgradeBuilding { player: P0, at }]);
}

// AI-MIL-4.5a (AI-MIL T15): the run walks the world's military list of
// every player, sites included, in creation order (player 0's castle,
// player 1's castle, player 0's barracks), with the player's own cursor;
// only its own building that is not a headquarters is tested, so the
// barracks is upgraded in the third run and the fourth starts over.
// spec: AI-MIL-4.5a (T15), AI-MIL-4.5
#[test]
fn t15_the_upgrade_run_walks_every_players_military_list() {
    let (world, mut ai, at) = upgrade_game_of(&[0, 1]);
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 5)], &[]);
    let mut runs = Vec::new();
    for _ in 0..4 {
        let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.military_upgrade(ctx));
        runs.push(commands);
    }
    let upgrade = Command::UpgradeBuilding { player: P0, at };
    assert_eq!(runs, [vec![], vec![], vec![upgrade], vec![]]);
}

// AI-CON-8.3 b (AI-CON T44): a military building being upgraded counts as
// unfinished for the route optimizer's partner while the upgrade's
// construction has work left: with k = 1 it is done (finished, k = 1 takes
// the nearest own warehouse).
// spec: AI-CON-8.3 (T44), MIL-UPG-3
#[test]
fn an_upgrading_building_counts_as_unfinished_for_its_partner() {
    let (mut world, mut ai, at) = upgrade_game();
    ai.player_mut(P0).street.route.k = 1;
    let (partner, _) = ai.with_ctx(&world, P0, |p, ctx| p.partner(ctx, at));
    assert_eq!(partner, Some((20, 20)), "the headquarters");
    world.submit(Command::UpgradeBuilding { player: P0, at });
    world.step();
    assert!(world.upgrading(at).is_some());
    let (partner, _) = ai.with_ctx(&world, P0, |p, ctx| p.partner(ctx, at));
    assert_eq!(partner, None);
}

// AI-CON T69 (AI-CON-8.3 b, AI-2.1a): an upgrade the military system orders
// earlier in the same think is in effect for that think's route optimizer,
// which reads the world through the think's view: B counts as unfinished,
// so with k = 1 it is done (no partner). Without the view's command B is
// finished and has its partner.
// spec: AI-CON-8.3 (T69), AI-2.1a
#[test]
fn t69_an_upgrade_ordered_in_the_same_think_makes_b_unfinished() {
    let game = || {
        let (world, mut ai, at) = upgrade_game();
        ai.player_mut(P0).street.route.k = 1;
        ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 4)], &[]);
        // The upgrade run's first turn is the castle's (AI-MIL-4.5a).
        let (_, first) = ai.with_ctx(&world, P0, |p, ctx| p.military_upgrade(ctx));
        assert!(first.is_empty());
        (world, ai, at)
    };
    // Read without the view, B is still finished: a partner.
    let (world, mut ai, at) = game();
    let (unviewed, _) = ai.with_ctx(&world, P0, |p, ctx| {
        p.military_upgrade(ctx);
        p.partner(ctx, at)
    });
    assert!(unviewed.is_some());
    let (world, mut ai, at) = game();
    let (partner, commands) = ai.with_view(&world, P0, |p, view, lent| {
        p.military_upgrade(&mut lent.ctx(view));
        view.issue(lent.out);
        p.partner(&mut lent.ctx(view), at)
    });
    assert_eq!(commands, [Command::UpgradeBuilding { player: P0, at }]);
    assert_eq!(partner, None);
}

// AI-2.1a, 2.1b (Q-289): the think's view lays every command over the world
// at once, for the later parts and players: a site the player places and an
// upgrade alike. The world itself is not changed.
// spec: AI-2.1a, AI-2.1b
#[test]
fn the_think_view_has_every_command_in_effect_at_once() {
    let (world, _, at) = upgrade_game();
    let hash = world.state_hash();
    let mut view = crate::view::ThinkView::new(&world);
    // In the player's land ((30, 22) is not).
    let site = (26, 24);
    let issued = vec![
        Command::PlanBuilding {
            player: P0,
            building: WOODCUTTER,
            at: site,
            records: 0,
        },
        Command::UpgradeBuilding { player: P0, at },
    ];
    assert!(view.world().upgrading(at).is_none());
    view.issue(&issued[..1]);
    assert!(view.world().building_at(site).is_some());
    assert!(view.world().upgrading(at).is_none());
    view.issue(&issued);
    assert!(view.world().upgrading(at).is_some());
    assert!(view.world().building_at(site).is_some());
    assert!(view.snapshot().building(P0, at).is_some());
    assert!(view.snapshot().building(P0, site).is_some());
    assert!(world.building_at(site).is_none());
    assert_eq!(world.state_hash(), hash);
}

// AI-CON-8.3 b, AI-CON-1.2 (AI-CON T45, Q-279): "B a depot" is the type's
// depot flag, which a harbour has though its record is filed in its cell's
// harbour list. A storehouse B in cell (1, 0) filed as a harbour takes the
// depot branch: of the 3 x 3 neighbouring cells numbered 0, 1, 2, 4, 5, 6,
// 8, 9, 10 the first with a building in its depot list is the
// headquarters' cell (1, 1), number 6, so S is the headquarters and k = 6
// (the non-depot branch would take the headquarters with k = 0). A cell
// whose only warehouse is in its harbour list has no depot for S.
// spec: AI-CON-8.3 (T45), AI-CON-1.2, AI-CON-1.1
#[test]
fn t45_a_harbour_takes_the_depot_branch() {
    use crate::player::Category;
    let mut world = world(&[0]);
    let b = (23, 14);
    with_building(&mut world, DEPOT, b);
    finish_site(&mut world, b);
    let mut ai = start_with(&world, config(), 2);
    let hq = ai.player(P0).unwrap().home;
    assert_eq!((cell_of(b), cell_of(hq)), ((1, 0), (1, 1)));
    let rec = ai.player_mut(P0).records.get_mut(&b).unwrap();
    rec.category = Category::Harbour;
    assert!(rec.protect.depot);
    let (partner, _) = ai.with_ctx(&world, P0, |p, ctx| p.partner(ctx, b));
    assert_eq!(partner, Some(hq));
    assert_eq!(ai.player_mut(P0).street.route.k, 6);
    // The headquarters filed in its cell's harbour list: no depot for S.
    ai.player_mut(P0).street.route.k = 0;
    ai.player_mut(P0).records.get_mut(&hq).unwrap().category = Category::Harbour;
    let (partner, _) = ai.with_ctx(&world, P0, |p, ctx| p.partner(ctx, b));
    assert_eq!(partner, None);
}

// AI-MIL-4.4: a failed smaller building at level 0 turns into an upgrade of
// an own building in the cell: the target is done and the penalty halves.
#[test]
fn a_failed_order_upgrades_a_building_in_its_cell() {
    let (world, mut ai, at) = upgrade_game();
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 5)], &[]);
    let cell = mcell_of(at);
    ai.player_mut(P0).territory.get_mut(cell).unwrap().penalty = 4.0;
    ai.player_mut(P0).expansion.current = Some((
        Source::WaitingNeed,
        Target {
            kind: Kind::Position,
            at: (40, 40),
            since: 0,
        },
    ));
    let order = Order {
        slot: Slot::Military,
        area: Area::Cell(cell_of(at)),
        building: BARRACKS,
        weight: 20,
        time: 0.0,
        tag: Tag::Military {
            attempts: 0,
            level: 0,
            cell,
            source: Source::WaitingNeed,
        },
    };
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| {
        p.military_order_done(order, Outcome::NoSite, ctx)
    });
    assert_eq!(commands, [Command::UpgradeBuilding { player: P0, at }]);
    assert!(ai.player_mut(P0).expansion.current.is_none());
    assert_eq!(ai.player_mut(P0).territory.get(cell).penalty, 2.0);
}

/// AI-MIL-4.4: an order of `area` that failed with no site at level 0
/// (attempts 0 -> 1), handed to the order handler; its commands.
fn failed_order(world: &World, ai: &mut AiSystem, area: Area) -> Vec<Command> {
    let cell = match area {
        Area::Cell(c) => mcell_of(crate::cells::origin(c)),
        Area::Point { at, .. } => mcell_of(at),
    };
    ai.player_mut(P0).expansion.current = Some((
        Source::WaitingNeed,
        Target {
            kind: Kind::Position,
            at: (40, 40),
            since: 0,
        },
    ));
    let order = Order {
        slot: Slot::Military,
        area,
        building: BARRACKS,
        weight: 20,
        time: 0.0,
        tag: Tag::Military {
            attempts: 0,
            level: 0,
            cell,
            source: Source::WaitingNeed,
        },
    };
    let (_, commands) = ai.with_ctx(world, P0, |p, ctx| {
        p.military_order_done(order, Outcome::NoSite, ctx)
    });
    commands
}

// AI-MIL T32 (AI-MIL-4.4a, MIL-UPG-1.4 corrected): the upgrade a failed
// military order makes instead tests no level and the next type's costs
// once (the 37-tick run of 4.5: the level and twice the costs). Without an
// underground resource the barracks' upgrade level is 0 (its own level), so
// the 37-tick run leaves it, and 2 stones are the guard house's costs once:
// the order upgrades it; with 1 stone nothing passes and the target is
// cancelled.
// spec: AI-MIL-4.4a (T32), AI-MIL-4.4, MIL-UPG-1.4
#[test]
fn t32_the_upgrade_instead_tests_no_level_and_the_costs_once() {
    let (world, mut ai, at) = upgrade_game();
    ai.player_mut(P0)
        .resources
        .set_entries(crate::resources::square_of(at), Vec::new());
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 4)], &[]);
    let (level, _) = ai.with_ctx(&world, P0, |p, ctx| p.upgrade_level(ctx, at));
    assert_eq!(level, 0);
    let (_, run) = ai.with_ctx(&world, P0, |p, ctx| p.military_upgrade(ctx));
    assert!(run.is_empty(), "the 37-tick run tests the level: {run:?}");
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 2)], &[]);
    let commands = failed_order(&world, &mut ai, Area::Cell(cell_of(at)));
    assert_eq!(commands, [Command::UpgradeBuilding { player: P0, at }]);
    assert!(ai.player_mut(P0).expansion.current.is_none(), "done");
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 1)], &[]);
    let commands = failed_order(&world, &mut ai, Area::Cell(cell_of(at)));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(ai.player_mut(P0).expansion.current.is_none(), "cancelled");
}

// AI-MIL T32 (AI-MIL-4.4a): in cell mode the fields are walked row by row
// from the cell's first row, so a building on an earlier row is upgraded
// before one made earlier; in point mode the walk reaches the order's own
// radius (12 towards a position), not 6 fields.
// spec: AI-MIL-4.4a (T32)
#[test]
fn t32_the_upgrade_instead_walks_rows_and_the_orders_radius() {
    let (mut world, mut ai, at) = upgrade_game();
    let earlier_row = (25, 17);
    created(&mut world, BARRACKS, earlier_row);
    for _ in 0..60 {
        world.step();
    }
    assert_eq!(cell_of(earlier_row), cell_of(at));
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 2)], &[]);
    let commands = failed_order(&world, &mut ai, Area::Cell(cell_of(at)));
    assert_eq!(
        commands,
        [Command::UpgradeBuilding {
            player: P0,
            at: earlier_row
        }]
    );
    // Point mode: the barracks 10 fields from the order's point.
    let (world, mut ai, at) = upgrade_game();
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 2)], &[]);
    let point = (33, 22);
    assert_eq!(crate::view::dist(point, at), 10);
    let commands = failed_order(
        &world,
        &mut ai,
        Area::Point {
            at: point,
            radius: 12,
        },
    );
    assert_eq!(commands, [Command::UpgradeBuilding { player: P0, at }]);
}

// ECO-DIST-1.7c b, AI-MIL-6.1 (Q-264): with no storehouse the castle, a
// depot (AI-CON-1.1), is the recruiting warehouse: the step finds or
// creates its records of the soldier's wares, flags clear; with the records
// there it sends nothing more.
// spec: ECO-DIST-1.7c, AI-MIL-6.1
#[test]
fn the_recruitment_step_makes_the_castle_s_records() {
    let mut world = world(&[0]);
    let mut ai = start_with(&world, config(), 2);
    let home = ai.player(P0).unwrap().home;
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.soldier_creation(ctx));
    let made: Vec<u32> = commands
        .iter()
        .filter_map(|c| match c {
            Command::WarehouseSet {
                at,
                good,
                stop: false,
                on: false,
                ..
            } if *at == home => Some(*good),
            _ => None,
        })
        .collect();
    assert_eq!(made, [SWORD, SHIELD, BEER]);
    for c in commands {
        world.submit(c);
    }
    world.step();
    let records = &world.warehouse(home).unwrap().flags;
    assert_eq!(records.len(), 3);
    assert!(records.values().all(|f| !f.stop && !f.out));
    let (_, again) = ai.with_ctx(&world, P0, |p, ctx| p.soldier_creation(ctx));
    assert!(again.is_empty(), "{again:?}");
}

// AI-MIL-6.1 (Q-264): the depot farthest from the enemy and nearest to
// the smithy and brewery recruits, the headquarters among the candidates;
// the "out" flag for sword, shield and beer is set in every other depot,
// cell by cell, and cleared in the chosen one last.
// spec: AI-MIL-6.1
#[test]
fn the_recruiting_storehouse_keeps_its_swords() {
    let mut world = world(&[0]);
    let (first, second) = ((27, 22), (24, 26));
    with_building(&mut world, DEPOT, first);
    finish_site(&mut world, first);
    with_building(&mut world, DEPOT, second);
    finish_site(&mut world, second);
    let mut ai = start_with(&world, config(), 2);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.soldier_creation(ctx));
    // No enemy (1000 for all), no smithy or brewery (100 + 100 for all): a
    // tie keeps the earliest, the headquarters, announced first in its
    // cell; the two storehouses get the flags, the headquarters' are
    // cleared (its records made).
    let home = ai.player(P0).unwrap().home;
    let set: Vec<(Point, u32, bool)> = commands
        .iter()
        .filter_map(|c| match c {
            Command::WarehouseSet {
                at,
                good,
                stop: false,
                on,
                ..
            } => Some((*at, *good, *on)),
            _ => None,
        })
        .collect();
    let mut expected = Vec::new();
    for (at, on) in [(first, true), (second, true), (home, false)] {
        for good in [SWORD, SHIELD, BEER] {
            expected.push((at, good, on));
        }
    }
    assert_eq!(set, expected);
}

// AI-MIL T22 (AI-MIL-6.1): S and B are the stored ware costs from a
// depot's flag to every recorded smithy and brewery, construction sites
// included, each from 100. A smithy site beside the storehouse, joined by
// roads with carriers, makes the storehouse cheaper than the headquarters
// (whose way to it goes over the storehouse's flag), so it recruits;
// counting finished smithies only, both would score 100 + 100 and the
// headquarters, announced first, would keep it.
// spec: AI-MIL-6.1 (AI-MIL T22), TRN-CARRY-5.3a
#[test]
fn t22_a_smithy_site_counts_for_the_recruiting_depot() {
    let rules = crate::fixture::town_rules();
    let config = crate::fixture::config_for(&rules);
    let mut world = crate::fixture::world_on(1, &[0], &Extras::default(), rules);
    let (store, smithy) = ((24, 26), (27, 22));
    with_building(&mut world, DEPOT, store);
    finish_site(&mut world, store);
    with_building(&mut world, SMITHY, smithy);
    let mut ai = start_with(&world, config, 2);
    let home = ai.player(P0).unwrap().home;
    let flag = |at| crate::view::flag_of(&world, at).unwrap();
    let (hq_flag, store_flag, smithy_flag) = (flag(home), flag(store), flag(smithy));
    for (from, to) in [(hq_flag, store_flag), (store_flag, smithy_flag)] {
        world.submit(Command::BuildRoad {
            player: P0,
            from,
            to,
        });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()), "road");
    }
    for _ in 0..1500 {
        if world.roads().all(|r| r.carrier.is_some()) {
            break;
        }
        world.step();
    }
    assert!(world.roads().all(|r| r.carrier.is_some()), "carriers");
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.soldier_creation(ctx));
    let cleared: Vec<Point> = commands
        .iter()
        .filter_map(|c| match c {
            Command::WarehouseSet { at, on: false, .. } => Some(*at),
            _ => None,
        })
        .collect();
    assert!(
        !cleared.is_empty() && cleared.iter().all(|&at| at == store),
        "{commands:?}"
    );
}

// AI-MIL-4.3 (corrected): the upgrade check's +1 comes from the nearest own
// finished warehouse on the site's landmass within distance < 5, not from
// any building: a barracks 3 from the castle (a warehouse) gets it, one far
// from the castle with a finished woodcutter beside it does not. Coal under
// both gives +2 (level -2 + 2 (+ 1)).
// spec: AI-MIL-4.3, AI-CON-1.1b
#[test]
fn the_upgrade_check_counts_a_warehouse_near_the_site() {
    let mut world = world(&[0]);
    let (near, far, woodcutter) = ((23, 22), (25, 24), (23, 26));
    with_building(&mut world, BARRACKS, far);
    finish_site(&mut world, far);
    // The near one, four from the castle, is no site a player could place
    // (ECO-PLACE-6.2).
    created(&mut world, BARRACKS, near);
    with_building(&mut world, WOODCUTTER, woodcutter);
    finish_site(&mut world, woodcutter);
    assert!(crate::view::dist(far, (20, 20)) >= 5 && crate::view::dist(far, woodcutter) < 5);
    let mut ai = start_with(&world, config(), 2);
    let landmass = ai.player_mut(P0).home_landmass;
    for at in [near, far] {
        ai.player_mut(P0).resources.set_entries(
            crate::resources::square_of(at),
            vec![crate::resources::Entry {
                good: COAL,
                kind: crate::resources::UNDERGROUND,
                landmass,
                amount: 5,
            }],
        );
    }
    let levels = ai.with_ctx(&world, P0, |p, ctx| {
        (p.upgrade_level(ctx, near), p.upgrade_level(ctx, far))
    });
    assert_eq!(levels.0, (1, 0));
}

// AI-CON-5.3 (1) (corrected 2026-10-27 night, AI-MIL T18): an order begins
// only with twice its type's construction cost in stock (frame and main
// parts each counted twice): a stone pit (planks 1) fails for no material
// with one plank and begins its site search with two.
// spec: AI-CON-5.3 (AI-MIL T18)
#[test]
fn an_order_needs_twice_its_costs_in_stock() {
    let world = world(&[0]);
    for (planks, begins) in [(1, false), (2, true)] {
        let mut ai = start_with(&world, config(), 2);
        let cell = cell_of(ai.player_mut(P0).home);
        let p = ai.player_mut(P0);
        p.econ = Economy::from_counts(&[(PLANK, planks)], &[]);
        p.build.issue(Order {
            slot: Slot::Needs,
            area: Area::Cell(cell),
            building: STONE_PIT,
            weight: 10,
            time: -1.0,
            tag: Tag::None,
        });
        ai.with_ctx(&world, P0, |p, ctx| {
            p.construction_tick(ctx);
            p.budget_step(ctx);
        });
        assert_eq!(
            ai.player_mut(P0).build.stage().map(|(s, _)| s),
            Some("taken")
        );
        ai.with_ctx(&world, P0, |p, ctx| p.construction_tick(ctx));
        let stage = ai.player_mut(P0).build.stage().map(|(s, _)| s);
        assert_eq!(stage == Some("scanning"), begins, "{planks} planks");
    }
}

// AI-CON-1.7 (corrected 2026-10-27 night, AI-MIL T18): stock(g) counts the
// stores of the player's warehouses only, not the materials at a
// construction site (nor a workshop's inputs).
// spec: AI-CON-1.7 (AI-MIL T18)
#[test]
fn stock_counts_the_warehouses_only() {
    let mut world = world(&[0]);
    // Five fields from the castle: ECO-PLACE-6.2 keeps a barracks site
    // off the four around it.
    let at = (24, 22);
    with_building(&mut world, BARRACKS, at);
    world.submit(Command::DebugSupplySite {
        player: P0,
        at,
        wares: vec![(STONE, 1)],
        workers: false,
    });
    world.step();
    let on_site: u32 = world
        .site(at)
        .unwrap()
        .on_site
        .iter()
        .filter(|&&(g, _)| g == STONE)
        .map(|&(_, n)| n)
        .sum();
    assert!(on_site > 0);
    let stored: u32 = world
        .warehouses()
        .flat_map(|(_, s)| s.iter())
        .filter(|&&(g, _)| g == STONE)
        .map(|&(_, n)| n)
        .sum();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let stock = Economy::count_stock(&world, P0, snapshot.of(P0));
    assert_eq!(stock.get(&STONE).copied().unwrap_or(0), stored);
}
