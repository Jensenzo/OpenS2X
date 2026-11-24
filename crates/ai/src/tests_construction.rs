//! Tests of the construction part (`AI-construction.md` T1 to T13).

use s2_sim::{Command, PlayerId, World};

use crate::cells::cell_of;
use crate::construction::{Order, Outcome, Slot, Tag};
use crate::needs::needs_score;
use crate::sites::Area;
use crate::testutil::*;
use crate::view::Economy;
use crate::AiSystem;

const P0: PlayerId = PlayerId(0);

fn start(world: &World, profile: u8) -> AiSystem {
    start_with(world, config(), profile)
}

fn start_with(world: &World, config: crate::AiConfig, profile: u8) -> AiSystem {
    let mut ai = AiSystem::new(config, world);
    assert!(ai.activate_with(world, &mut s2_sim::Numbers::new(0), P0, profile));
    ai
}

fn run_to_seconds(world: &mut World, seconds: f32) {
    while world.game_seconds() <= seconds {
        world.step();
    }
}

fn count(ai: &mut AiSystem, _world: &World, good: u32) -> usize {
    ai.player_mut(P0)
        .plan
        .needs
        .iter()
        .filter(|n| n.good == good)
        .count()
}

// T1: Strong profile (3 sawmills), t = 0: 1 stone need and 3 plank needs
// attached to the headquarters. SIM-START T14: each is a record of its own,
// four object numbers (E-trace-012: 90188 to 90191); the Weak profile's
// two.
// spec: AI-CON-2.1 (T1), SIM-START-4.4 (T14)
#[test]
fn t1_start_needs() {
    let world = world(&[0]);
    let mut config = config();
    config.profiles[2].start_sawmills = 3;
    let mut ai = start_with(&world, config, 3);
    let made = ai.player_mut(P0).plan.made();
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, STONE), 1);
    assert_eq!(count(&mut ai, &world, PLANK), 3);
    assert_eq!(ai.player_mut(P0).plan.made(), made + 4, "four records");
    let home = ai.player_mut(P0).home;
    assert!(ai
        .player_mut(P0)
        .plan
        .needs
        .iter()
        .all(|n| n.consumer == Some(home)));
    // The weak profile still gets one sawmill need.
    let mut weak = start(&world, 1);
    let made = weak.player_mut(P0).plan.made();
    weak.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut weak, &world, PLANK), 1);
    assert_eq!(weak.player_mut(P0).plan.made(), made + 2, "two records");
}

// T2: Weak profile (max smithies 2, max mints 0): after t = 300 with
// producers of stone, wood and plank, a sword and a beer need; at t >= 1200
// no coin need.
// spec: AI-CON-2.2, AI-CON-2.3 (T2)
#[test]
fn t2_weapon_needs_and_no_coins_for_weak() {
    let mut world = world(&[0]);
    let mut config = config();
    config.names.sword = 20;
    config.names.beer = 21;
    config.names.coin = 22;
    config.names.donkey = 23;
    config.profiles[0].max_smithies = 2;
    config.profiles[0].max_mints = 0;
    let mut ai = start_with(&world, config, 1);
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    let cell = cell_of(ai.player_mut(P0).home);
    for good in [STONE, WOOD, PLANK] {
        let id = ai.player_mut(P0).plan.add_production(good, cell, None);
        let p = &mut ai.player_mut(P0).plan.prods;
        p.iter_mut().find(|x| x.id == id).unwrap().building = Some((1, 1));
    }
    run_to_seconds(&mut world, 300.0);
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, 20), 1);
    assert_eq!(count(&mut ai, &world, 21), 1);
    run_to_seconds(&mut world, 800.0);
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, 23), 1);
    run_to_seconds(&mut world, 1200.0);
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, 22), 0);
    assert_eq!(ai.player_mut(P0).need_stage, 4);
}

// AI-CON-5.4 (Q-281): the route to a candidate is a free-walk path
// (TRN-FREE-2, 3), counted in steps: a field behind a row of trees is
// reached around it (a tree's field costs 2 more a step), one step more
// than the straight way through.
// spec: AI-CON-5.4, TRN-FREE-2.5
#[test]
fn the_site_route_is_a_free_walk_path() {
    let (start, to) = ((30, 30), (34, 30));
    let trees: Vec<(u32, u32)> = (31..34).map(|x| (x, 30)).collect();
    let world = world_with(
        &[0],
        &Extras {
            trees,
            rocks: Vec::new(),
            fish: Vec::new(),
        },
    );
    assert_eq!(s2_sim::grid::distance(start, to), 4);
    assert_eq!(crate::sites::route_length(&world, start, to), Some(5));
    let next = world.terrain().neighbour(start, 3).unwrap();
    assert_eq!(crate::sites::route_length(&world, start, next), Some(1));
    assert_eq!(crate::sites::route_length(&world, start, start), Some(0));
}

// AI-CON-5.4b: the score is ((2R - len) / 2R) x v in single precision, in
// that order: with 2R = 24, value 2.5 at 5 steps scores above value 9.5 at
// 19 steps (1.9791667 against 1.9791666), which v x (2R - len) / 2R ties.
// spec: AI-CON-5.4b
#[test]
fn the_site_score_divides_first() {
    use crate::sites::score;
    assert!(score(2.5, 5, 24) > score(9.5, 19, 24));
    assert_eq!(2.5f32 * 19.0 / 24.0, 9.5f32 * 5.0 / 24.0);
    // A limit of 16 divides exactly either way.
    assert_eq!(score(45.5, 7, 16), 45.5 * 9.0 / 16.0);
}

// T3: S = 1536, 1 plank producer, stock(plank) 40: when t crosses 600,
// trunc(1536 x 0.03125) = 48 > 40 so a plank need is created; the next check
// is at t > 1200. SIM-START-4.4 g: that need is a record of its own (an
// object number) although the castle has its plank need from the start.
// spec: AI-CON-2.4, AI-CON-1.8 (T3), SIM-START-4.4
#[test]
fn t3_stock_check_doubles() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    let planks_at_start = count(&mut ai, &world, PLANK);
    let cell = cell_of(ai.player_mut(P0).home);
    ai.player_mut(P0).plan.add_production(PLANK, cell, None);
    ai.player_mut(P0).need_stage = 4;
    run_to_seconds(&mut world, 600.0);
    let tick = world.tick();
    {
        let p = ai.player_mut(P0);
        p.statistic = (tick, 1536);
        p.econ = Economy::from_counts(&[(PLANK, 40), (STONE, 1000)], &[]);
    }
    let made = ai.player_mut(P0).plan.made();
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, PLANK), planks_at_start + 1);
    assert_eq!(ai.player_mut(P0).plan.made(), made + 1, "a record");
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, PLANK), planks_at_start + 1);
    assert_eq!(ai.player_mut(P0).stock_check, 2);
}

// AI-CON T86 (AI-CON-2.4, corrected 2026-10-30): the stock checks divide by
// the needs of the good standing in cells, not by its producers: S = 883,
// stone stock 39, three stone productions but one stone need in a cell:
// trunc(883 x 0.0625) / 1 = 55 > 39, a stone need (by the producers,
// 55 / 3 = 18, none).
// spec: AI-CON-2.4 (AI-CON T86)
#[test]
fn t86_the_stock_check_divides_by_the_needs_in_cells() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    let stones_at_start = count(&mut ai, &world, STONE);
    let cell = cell_of(ai.player_mut(P0).home);
    {
        let p = ai.player_mut(P0);
        for _ in 0..3 {
            p.plan.add_production(STONE, cell, None);
        }
        p.plan.needs.retain(|n| n.good != STONE);
        let need = p.plan.add_need(STONE, None, 0.0);
        p.plan.need_mut(need).unwrap().cell = Some(cell);
        p.need_stage = 4;
    }
    let stones_before = count(&mut ai, &world, STONE);
    assert!(stones_before <= stones_at_start);
    run_to_seconds(&mut world, 600.0);
    let tick = world.tick();
    {
        let p = ai.player_mut(P0);
        p.statistic = (tick, 883);
        p.econ = Economy::from_counts(&[(PLANK, 1000), (STONE, 39)], &[]);
    }
    ai.with_ctx(&world, P0, |p, ctx| p.need_creation(ctx));
    assert_eq!(count(&mut ai, &world, STONE), stones_before + 1);
}

// AI-CON T108: a needs order for the stone need `later` (given the cell
// second) places its site; another stone need `first` was given the cell
// before it: the site's production goes to `first`, `later` stays waiting.
// With no other waiting need it goes to the order's own need (T4, T88).
// spec: AI-CON-2.5b (T108), AI-CON-4.3, AI-CON-7.1
#[test]
fn t108_a_needs_order_site_goes_to_the_cells_first_need() {
    let world = world(&[0]);
    let mut config = config();
    config.producer.insert(STONE, STONE_PIT);
    let mut ai = start_with(&world, config, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    let (first, later) = {
        let p = ai.player_mut(P0);
        let later = p.plan.add_need(STONE, Some((1, 1)), 0.0);
        let first = p.plan.add_need(STONE, Some((1, 1)), 0.0);
        p.plan.give_cell(first, cell);
        p.plan.give_cell(later, cell);
        (first, later)
    };
    let order = Order {
        slot: Slot::Needs,
        area: Area::Cell(cell),
        building: STONE_PIT,
        weight: 10,
        time: 0.0,
        tag: Tag::Need { need: later },
    };
    let at = (9, 9);
    ai.with_ctx(&world, P0, |p, ctx| {
        p.needs_order_done(order, Outcome::Placed(at), ctx)
    });
    let p = ai.player_mut(P0);
    let prod = p.plan.need(first).unwrap().production.expect("tied");
    assert_eq!(p.plan.production(prod).unwrap().need, Some(first));
    assert_eq!(p.plan.production(prod).unwrap().building, Some(at));
    assert_eq!(p.plan.need(later).unwrap().production, None);
}

// T4: needs: bread D = 2, A = 0, priority 70, stock 0; coal D = 1, A = 1,
// priority 100, stock 3: bread scores 693, coal 0; bread is chosen. T88:
// the order makes no production; the placed site does.
// spec: AI-CON-4.1, AI-CON-4.2, AI-CON-4.3 (T4), AI-CON-7.1 (AI-CON T88)
#[test]
fn t4_needs_score() {
    let bread = needs_score(2.0, 0.0, 70.0, 0, 1.0);
    assert!((bread - 693.0).abs() < 0.01, "{bread}");
    assert_eq!(needs_score(1.0, 1.0, 100.0, 3, 1.0), 0.0);

    let world = world(&[0]);
    let mut config = config();
    config.producer.insert(BREAD, BAKERY);
    let mut ai = start_with(&world, config, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    {
        let p = ai.player_mut(P0);
        for _ in 0..2 {
            let id = p.plan.add_need(BREAD, Some((1, 1)), 0.0);
            p.plan.need_mut(id).unwrap().cell = Some(cell);
        }
        let id = p.plan.add_need(COAL, Some((1, 1)), 0.0);
        p.plan.need_mut(id).unwrap().cell = Some(cell);
        let prod = p.plan.add_production(COAL, cell, Some(id));
        p.plan.need_mut(id).unwrap().production = Some(prod);
        p.econ = Economy::from_counts(&[(COAL, 3), (STONE, 50)], &[]);
        p.statistic = (1, 300);
    }
    ai.with_ctx(&world, P0, |p, ctx| p.needs_construction(ctx));
    let order = ai.player_mut(P0).build.slot(Slot::Needs).copied().unwrap();
    assert_eq!(order.building, BAKERY);
    assert_eq!(order.weight, 10);
    assert_eq!(order.area, Area::Cell(cell));
    assert!(matches!(order.tag, Tag::Need { .. }));
    // While that order is pending nothing else is ordered, and the order
    // makes no production (AI-CON-7.1, T88): the bread need stays without
    // one until the site is placed.
    ai.with_ctx(&world, P0, |p, ctx| p.needs_construction(ctx));
    assert_eq!(ai.player_mut(P0).plan.producers(BREAD), 0);
    let Tag::Need { need } = order.tag else {
        unreachable!()
    };
    assert_eq!(ai.player_mut(P0).plan.need(need).unwrap().production, None);
    // Placed: the site's production record, for that need.
    let at = (9, 9);
    ai.with_ctx(&world, P0, |p, ctx| {
        p.needs_order_done(order, Outcome::Placed(at), ctx)
    });
    let p = ai.player_mut(P0);
    assert_eq!(p.plan.producers(BREAD), 1);
    let prod = p.plan.need(need).unwrap().production.unwrap();
    let made = p.plan.production(prod).unwrap();
    assert_eq!(
        (made.building, made.need, made.cell),
        (Some(at), Some(need), cell)
    );
}

// T5: a cell with 21 needs (AI-CON T71: needs, not productions) does not
// accept a need for plank, but does for coal.
// spec: AI-CON-3.3, AI-CON-1.1 (T5, AI-CON T71)
#[test]
fn t5_cell_capacity() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    for _ in 0..21 {
        let p = ai.player_mut(P0);
        let id = p.plan.add_need(SAW, None, 0.0);
        p.plan.need_mut(id).unwrap().cell = Some(cell);
    }
    let (plank, coal) = {
        let (r, _) = ai.with_ctx(&world, P0, |p, ctx| {
            (
                p.cell_accepts(ctx, cell, PLANK),
                p.cell_accepts(ctx, cell, COAL),
            )
        });
        r
    };
    // COAL is the test's coal good; the config names it.
    assert!(!plank);
    assert!(coal);
}

// T6: idle construction system, L = 0, C = 0, orders pending: at the first
// step an order is taken at once; after a placement L = 2; the count then
// rises step by step before the next order is taken.
// spec: AI-CON-5.2 (T6)
#[test]
fn t6_budget() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    let order = |slot, time| Order {
        slot,
        area: Area::Cell(cell),
        building: STONE_PIT,
        weight: 10,
        time,
        tag: Tag::None,
    };
    {
        let p = ai.player_mut(P0);
        p.build.issue(order(Slot::Needs, -1.0));
        p.econ = Economy::from_counts(&[(PLANK, 20), (STONE, 20)], &[]);
    }
    ai.with_ctx(&world, P0, |p, ctx| p.budget_step(ctx));
    assert!(ai.player_mut(P0).build.exec.is_some());
    assert!(ai.player_mut(P0).build.slot(Slot::Needs).is_none());
    // A placement raises the limit: L = 2 L + 2.
    {
        let p = ai.player_mut(P0);
        p.build.exec = None;
        p.build.limit = 2 * p.build.limit + 2;
        p.build.issue(order(Slot::Needs, -1.0));
    }
    assert_eq!(ai.player_mut(P0).build.limit, 2);
    for expected in [1, 2] {
        ai.with_ctx(&world, P0, |p, ctx| p.budget_step(ctx));
        assert_eq!(ai.player_mut(P0).build.count, expected);
        assert!(ai.player_mut(P0).build.exec.is_none());
    }
    ai.with_ctx(&world, P0, |p, ctx| p.budget_step(ctx));
    assert!(ai.player_mut(P0).build.exec.is_some());
    // The reset every 600 ticks.
    ai.player_mut(P0).budget_reset();
    assert_eq!(
        (ai.player_mut(P0).build.limit, ai.player_mut(P0).build.count),
        (0, 0)
    );
}

// The order with the largest (now - time) x weight is taken first, ties by
// the slot order.
// spec: AI-CON-5.1, AI-CON-5.2
#[test]
fn pending_orders_are_taken_by_age_times_weight() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    for _ in 0..1000 {
        world.step();
    }
    let order = |slot, time, weight| Order {
        slot,
        area: Area::Cell(cell),
        building: STONE_PIT,
        weight,
        time,
        tag: Tag::None,
    };
    {
        let p = ai.player_mut(P0);
        p.econ = Economy::from_counts(&[(PLANK, 20)], &[]);
        let now = world.game_seconds();
        p.build.issue(order(Slot::Needs, now - 10.0, 10));
        p.build.issue(order(Slot::Forester, now - 1.0, 50));
        p.build.issue(order(Slot::Hunter, now - 90.0, 10));
    }
    // ages in game seconds x weights: needs 100, forester 50, hunter 900.
    ai.with_ctx(&world, P0, |p, ctx| p.budget_step(ctx));
    let exec = ai.player_mut(P0).build.exec.as_ref().unwrap().order;
    assert_eq!(exec.slot, Slot::Hunter);
}

// AI-CON-5.2: an order made in this very tick (age 0) is not taken, and of
// two equal values the earlier source in the order needs, cell, military,
// ... wins, though the cell system's slot comes first.
// spec: AI-CON-5.2
#[test]
fn the_pick_wants_an_age_above_zero_and_keeps_ties_in_source_order() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    for _ in 0..100 {
        world.step();
    }
    let now = world.game_seconds();
    let order = |slot, time| Order {
        slot,
        area: Area::Cell(cell),
        building: STONE_PIT,
        weight: 10,
        time,
        tag: Tag::None,
    };
    ai.player_mut(P0).build.issue(order(Slot::Hunter, now));
    ai.with_ctx(&world, P0, |p, ctx| p.budget_step(ctx));
    assert!(ai.player_mut(P0).build.exec.is_none(), "age 0 is not taken");
    {
        let p = ai.player_mut(P0);
        p.build.issue(order(Slot::Cell, now - 5.0));
        p.build.issue(order(Slot::Needs, now - 5.0));
    }
    ai.with_ctx(&world, P0, |p, ctx| p.budget_step(ctx));
    let taken = ai.player_mut(P0).build.exec.as_ref().unwrap().order;
    assert_eq!(taken.slot, Slot::Needs);
}

// AI-CON-5.3a: taken in P and nothing else; the checks and the search's set
// up in P + 1 with no field tested; one search step a tick in P + 2 to
// P + 1 + n; the result in P + 2 + n; the site placed and L doubled in
// P + 3 + n; idle again from the next tick.
// spec: AI-CON-5.2, AI-CON-5.3a, AI-CON-5.4
#[test]
fn the_execution_advances_one_state_per_tick() {
    let rocks: Vec<(u32, u32)> = (0..8u32).map(|i| (22 + i, 27)).collect();
    let world = world_with(
        &[0],
        &Extras {
            trees: Vec::new(),
            rocks,
            fish: Vec::new(),
        },
    );
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    {
        let p = ai.player_mut(P0);
        p.econ = Economy::from_counts(&[(PLANK, 20), (STONE, 20)], &[]);
        p.build.issue(Order {
            slot: Slot::Needs,
            area: Area::Cell(cell),
            building: STONE_PIT,
            weight: 10,
            time: -1.0,
            tag: Tag::None,
        });
    }
    // The search's step count, from a search of its own.
    let mut n = 0;
    {
        let (_, _) = ai.with_ctx(&world, P0, |p, ctx| {
            let env = crate::sites::SiteEnv {
                world: ctx.world,
                config: ctx.config,
                player: p.id,
                own: ctx.snapshot.of(p.id),
            };
            let mut scan = crate::sites::Scan::new(ctx.world, Area::Cell(cell), STONE_PIT, None);
            while !scan.done() {
                scan.step(&env);
                n += 1;
            }
        });
    }
    assert!(n > 26, "fields passing the cheap tests cost steps: {n}");
    let stage = |ai: &mut AiSystem| ai.player_mut(P0).build.stage().map(|(s, _)| s);
    // Tick P: the budget step runs after the system's own tick.
    let (_, out) = ai.with_ctx(&world, P0, |p, ctx| {
        p.construction_tick(ctx);
        p.budget_step(ctx);
    });
    assert!(out.is_empty());
    assert_eq!(stage(&mut ai), Some("taken"));
    // P + 1: set up, no field tested.
    ai.with_ctx(&world, P0, |p, ctx| p.construction_tick(ctx));
    assert_eq!(ai.player_mut(P0).build.stage(), Some(("scanning", 0)));
    // P + 2 .. P + 1 + n.
    for step in 1..=n {
        ai.with_ctx(&world, P0, |p, ctx| p.construction_tick(ctx));
        let (s, tested) = ai.player_mut(P0).build.stage().unwrap();
        assert_eq!(s, "scanning", "step {step}");
        assert!(tested <= 10 * step);
    }
    // P + 2 + n: the result.
    let (_, out) = ai.with_ctx(&world, P0, |p, ctx| p.construction_tick(ctx));
    assert!(out.is_empty());
    assert_eq!(stage(&mut ai), Some("found"));
    // P + 3 + n: placed in this tick; a budget step now does nothing.
    let (_, out) = ai.with_ctx(&world, P0, |p, ctx| {
        p.construction_tick(ctx);
        p.budget_step(ctx);
    });
    assert!(
        matches!(
            out[..],
            [Command::PlanBuilding {
                building: STONE_PIT,
                ..
            }]
        ),
        "{out:?}"
    );
    assert_eq!(ai.player_mut(P0).build.limit, 2);
    assert_eq!(ai.player_mut(P0).build.count, 0);
    assert_eq!(stage(&mut ai), Some("ended"));
    ai.with_ctx(&world, P0, |p, ctx| p.construction_tick(ctx));
    assert_eq!(stage(&mut ai), None);
}

// AI-CON-5.4: a step tests at most 10 fields and ends after the first that
// passes the in-map, size and flag tests; a cell whose fields all fail
// cheaply takes 26 steps (256 / 10).
// spec: AI-CON-5.4
#[test]
fn a_search_step_ends_after_the_first_field_passing_the_cheap_tests() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let home = cell_of(ai.player_mut(P0).home);
    // A cell far from the territory: every field fails cheaply.
    let far = (home.0 + 2, home.1 + 2);
    ai.with_ctx(&world, P0, |p, ctx| {
        let env = crate::sites::SiteEnv {
            world: ctx.world,
            config: ctx.config,
            player: p.id,
            own: ctx.snapshot.of(p.id),
        };
        let mut scan = crate::sites::Scan::new(ctx.world, Area::Cell(far), STONE_PIT, None);
        let mut steps = 0;
        while !scan.done() {
            scan.step(&env);
            steps += 1;
        }
        assert_eq!(steps, 26);
        // The headquarters' cell: the first step stops at its first field
        // that passes, before the tenth field when one passes earlier.
        let mut scan = crate::sites::Scan::new(ctx.world, Area::Cell(home), STONE_PIT, None);
        let fields = Area::Cell(home).fields(ctx.world);
        let first = fields
            .iter()
            .position(|&f| {
                f.0 < ctx.world.terrain().width
                    && ctx.world.field_owner(f) == Some(p.id)
                    && crate::sites::class_fits(
                        ctx.world.building_type(STONE_PIT).unwrap().size,
                        ctx.world.compute_placement(p.id, f),
                    )
            })
            .expect("a field that fits");
        let mut tested = Vec::new();
        while !scan.done() && tested.len() < 30 {
            scan.step(&env);
            tested.push(scan.tested());
        }
        // The step that reaches the first fitting field ends on it.
        assert!(tested.contains(&(first + 1)), "{first} {tested:?}");
        assert!(tested.windows(2).all(|w| w[1] - w[0] <= 10));
    });
}

fn forest_world(trees: usize, rocks: usize) -> World {
    // Fields (16..31, 16..31) is the cell of the headquarters at (20, 20).
    let spots: Vec<(u32, u32)> = (0..16u32)
        .flat_map(|y| (0..16u32).map(move |x| (16 + x, 16 + y)))
        .filter(|&(x, y)| (x, y) != (20, 20) && (x, y) != (21, 19))
        .collect();
    world_with(
        &[0],
        &Extras {
            trees: spots.iter().copied().take(trees).collect(),
            rocks: spots.iter().rev().copied().take(rocks).collect(),
            fish: Vec::new(),
        },
    )
}

// T7: a cell with 60 trees, no forester, a sawmill, stock plank + wood 10:
// no forester order (60 >= 40); with 30 trees a forester order of weight 50.
// T66: a sawmill only ordered, its site not placed, is no plank producer
// of the cell: no forester order.
// spec: AI-CON-7.1 (T7, AI-CON T66)
#[test]
fn t7_forester_order() {
    let sawmill = Some((24, 24));
    for (trees, site, ordered) in [(60, sawmill, false), (30, sawmill, true), (30, None, false)] {
        let world = forest_world(trees, 0);
        let mut ai = start(&world, 2);
        let cell = cell_of(ai.player_mut(P0).home);
        {
            let p = ai.player_mut(P0);
            let id = p.plan.add_production(PLANK, cell, None);
            let prod = p.plan.prods.iter_mut().find(|x| x.id == id).unwrap();
            prod.building = site;
            p.econ = Economy::from_counts(&[(PLANK, 5), (WOOD, 5)], &[]);
        }
        ai.with_ctx(&world, P0, |p, ctx| p.forester_construction(ctx));
        let order = ai.player_mut(P0).build.slot(Slot::Forester).copied();
        assert_eq!(order.is_some(), ordered, "{trees} trees, site {site:?}");
        if let Some(o) = order {
            assert_eq!(o.weight, 50);
            assert_eq!(o.building, FORESTER);
        }
    }
}

// T8: a cell with 1 quarry and 12 stones: 5 x 1 < 12, a stone pit order.
// The run's grid cursor is set just before the cell (it advances first).
// spec: AI-CON-7.3 (T8)
#[test]
fn t8_stone_pit_order() {
    let before = |ai: &mut AiSystem, cell: (u32, u32)| {
        let p = ai.player_mut(P0);
        let total = p.cells.cols * p.cells.rows;
        p.pit_cursor = (cell.1 * p.cells.cols + cell.0 + total - 1) % total;
    };
    let world = forest_world(0, 12);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    ai.player_mut(P0).plan.add_production(STONE, cell, None);
    before(&mut ai, cell);
    ai.with_ctx(&world, P0, |p, ctx| p.pit_and_woodcutter(ctx));
    let order = ai.player_mut(P0).build.slot(Slot::Cell).copied().unwrap();
    assert_eq!((order.building, order.weight), (STONE_PIT, 10));
    // With 8 stones: 5 < 8 still; with 4 stones 5 < 4 is false: no order.
    let world = forest_world(0, 4);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    ai.player_mut(P0).plan.add_production(STONE, cell, None);
    before(&mut ai, cell);
    ai.with_ctx(&world, P0, |p, ctx| p.pit_and_woodcutter(ctx));
    assert!(ai.player_mut(P0).build.slot(Slot::Cell).is_none());
}

fn removed(commands: &[Command]) -> bool {
    commands
        .iter()
        .any(|c| matches!(c, Command::TearDown { .. }))
}

// T9: a fisher, finished, worker inside, productivity 0, fish in radius: the
// destruction run tears it down with remains (site value above 0,
// productivity 0).
// spec: AI-CON-9.2, AI-CON-9.3 (T9)
#[test]
fn t9_fisher_without_catch_is_torn_down() {
    let fish: Vec<(u32, u32)> = (24..30).map(|x| (x, 24)).collect();
    let mut world = world_with(
        &[0],
        &Extras {
            fish,
            ..Extras::default()
        },
    );
    let mut ai = start(&world, 2);
    with_building(&mut world, FISHER, (26, 22));
    let snapshot = finished_snapshot(&world, (26, 22), 0);
    ai.update_with(&world, P0, &snapshot);
    ai.player_mut(P0).destruction_cursor = 1;
    // The first run after the castle (serial 0, the cursor its id + 1)
    // picks the fisher.
    let (_, commands) =
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.building_destruction(ctx));
    assert!(removed(&commands), "{commands:?}");
    // With a catch (productivity above 0) and fish around it stays.
    let snapshot = finished_snapshot(&world, (26, 22), 75);
    ai.player_mut(P0).destruction_cursor = 1;
    let (_, commands) =
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.building_destruction(ctx));
    assert!(!removed(&commands));
}

// AI-CON T84 (AI-CON-9.3, 3.4): a need linked to the production of a
// woodcutter that is torn down loses the production, its cell and its
// square (the square free again) and is placed again as a new need; placed
// in a cell with a standing woodcutter whose production has no need, it is
// linked to that one. A plain new need is too (AI-CON-2.5b, 3.3 corrected:
// a free production placed or not).
// spec: AI-CON-9.3 (AI-CON T84), AI-CON-3.4, AI-CON-2.5b
#[test]
fn t84_a_torn_down_producers_need_is_placed_again() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let (a, b) = ((26, 22), (14, 20));
    assert_ne!(cell_of(a), cell_of(b));
    with_building(&mut world, WOODCUTTER, a);
    with_building(&mut world, WOODCUTTER, b);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let prod_at = |p: &crate::player::AiPlayer, at| {
        p.plan
            .prods
            .iter()
            .find(|x| x.building == Some(at))
            .map(|x| x.id)
            .unwrap()
    };
    let (pa, pb) = (prod_at(p, a), prod_at(p, b));
    let need = p.plan.add_need(WOOD, None, 0.0);
    let square = ((33, 14), 1);
    let n = p.plan.need_mut(need).unwrap();
    n.cell = Some(cell_of(a));
    n.square = Some(square);
    n.production = Some(pa);
    p.plan.prods.iter_mut().find(|x| x.id == pa).unwrap().need = Some(need);
    // The destruction run's step before its demolish command (T111), then
    // the woodcutter on a goes.
    p.tear_down_needs(a);
    let n = p.plan.need(need).unwrap().clone();
    assert_eq!((n.cell, n.square, n.production), (None, None, Some(pa)));
    world.submit(Command::RemoveBuilding { player: P0, at: a });
    world.step();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let n = p.plan.need(need).unwrap().clone();
    assert_eq!((n.cell, n.square, n.production), (None, None, None));
    assert!(n.again);
    assert!(!p.plan.prods.iter().any(|x| x.id == pa));
    // Placed again in b's cell: linked to b's standing production.
    p.assign_cell(&config(), need, cell_of(b), None);
    let n = p.plan.need(need).unwrap().clone();
    assert_eq!(
        (n.cell, n.production, n.again),
        (Some(cell_of(b)), Some(pb), false)
    );
    // A plain new need there takes the standing production as well.
    p.plan.prods.iter_mut().find(|x| x.id == pb).unwrap().need = None;
    let plain = p.plan.add_need(WOOD, None, 0.0);
    p.assign_cell(&config(), plain, cell_of(b), None);
    assert_eq!(p.plan.need(plain).unwrap().production, Some(pb));
}

// AI-CON T111 (AI-CON-9.3, which removals take the cell away): a need
// linked to the production of a building removed otherwise than by the
// computer player's tear-down (here a plain removal, as land lost) keeps its
// cell and its square and loses only the production. The destruction run's
// tear-down of a fisher without a catch takes its linked need (with a
// square) off its cell and square before the demolish command; a need
// without a square keeps its cell.
// spec: AI-CON-9.3 (AI-CON T111), AI-CON-9.2
#[test]
fn t111_only_the_tear_down_takes_a_needs_cell() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let a = (26, 22);
    with_building(&mut world, WOODCUTTER, a);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let pa = p
        .plan
        .prods
        .iter()
        .find(|x| x.building == Some(a))
        .unwrap()
        .id;
    let need = p.plan.add_need(WOOD, None, 0.0);
    let square = ((33, 14), 1);
    let n = p.plan.need_mut(need).unwrap();
    n.cell = Some(cell_of(a));
    n.square = Some(square);
    n.production = Some(pa);
    world.submit(Command::RemoveBuilding { player: P0, at: a });
    world.step();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let n = p.plan.need(need).unwrap().clone();
    assert_eq!(
        (n.cell, n.square, n.production, n.again),
        (Some(cell_of(a)), Some(square), None, false)
    );
    assert!(!p.plan.prods.iter().any(|x| x.id == pa));

    // The destruction run's tear-down (the fisher of T9).
    let fish: Vec<(u32, u32)> = (24..30).map(|x| (x, 24)).collect();
    let mut world = world_with(
        &[0],
        &Extras {
            fish,
            ..Extras::default()
        },
    );
    let mut ai = start(&world, 2);
    with_building(&mut world, FISHER, a);
    let snapshot = finished_snapshot(&world, a, 0);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let pf = p
        .plan
        .prods
        .iter()
        .find(|x| x.building == Some(a))
        .unwrap()
        .id;
    let with_square = p.plan.add_need(FISH, None, 0.0);
    let n = p.plan.need_mut(with_square).unwrap();
    n.cell = Some(cell_of(a));
    n.square = Some(square);
    n.production = Some(pf);
    let without = p.plan.add_need(FISH, None, 0.0);
    let n = p.plan.need_mut(without).unwrap();
    n.cell = Some(cell_of(a));
    n.production = Some(pf);
    p.destruction_cursor = 1;
    let (_, commands) =
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.building_destruction(ctx));
    assert!(removed(&commands), "{commands:?}");
    let p = ai.player_mut(P0);
    let n = p.plan.need(with_square).unwrap().clone();
    assert_eq!(
        (n.cell, n.square, n.production, n.again),
        (None, None, Some(pf), true)
    );
    let n = p.plan.need(without).unwrap().clone();
    assert_eq!((n.cell, n.again), (Some(cell_of(a)), false));
}

// AI-CON T109 (AI-CON-9.3, the building's own needs): a sawmill's wood
// need (its consumer the sawmill) waiting for expansion, and a woodcutter
// production of another cell, not placed, linked to it: when the sawmill is
// removed, its own needs are deleted (off the need list and the waiting
// list) and the production loses its need; a need of another consumer
// stays.
// spec: AI-CON-9.3 (AI-CON T109), AI-CON-3.4
#[test]
fn t109_a_removed_buildings_own_needs_go_with_it() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let sawmill = (26, 22);
    with_building(&mut world, SAWMILL, sawmill);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let own = p.plan.add_need(WOOD, Some(sawmill), 0.0);
    p.plan.put_waiting(own);
    let home = p.home;
    let other = p.plan.add_need(WOOD, Some(home), 0.0);
    let prod = p.plan.add_production(WOOD, cell_of((14, 20)), Some(own));
    world.submit(Command::RemoveBuilding {
        player: P0,
        at: sawmill,
    });
    world.step();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    assert!(
        p.plan.need(own).is_none(),
        "the sawmill's own need is deleted"
    );
    assert!(!p.plan.waiting.contains(&own));
    assert!(!p.plan.needs.iter().any(|n| n.consumer == Some(sawmill)));
    assert_eq!(p.plan.production(prod).unwrap().need, None);
    assert!(
        p.plan.need(other).is_some(),
        "another consumer's need stays"
    );
}

// AI-CON T102 (AI-CON-2.5b): a cell ties needs and productions from either
// side. A producer's production, whatever order placed its building, is
// tied when the building is announced to the first need in its cell's need
// list whose good it supplies and that has no production yet; the list is
// in the order the needs were given the cell, not by id. So the route
// optimizer's partner of the needs' consumer is that producer (R6: player
// 3's sawmill's wood need, in the home cell from before counter 500, gets
// the woodcutter (30, 22) that the cell system placed at 622, and the
// sawmill's k = 2 partner at 6829 is that woodcutter, not the later one of
// 1786). A need given a cell takes the first free production of its good
// there, placed or not.
// spec: AI-CON-2.5b (T102), AI-CON-3.3, AI-CON-8.3
#[test]
fn t102_a_cell_ties_needs_and_productions() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let (sawmill, woodcutter, other) = ((30, 30), (26, 22), (14, 20));
    let home = cell_of(ai.player_mut(P0).home);
    assert_eq!(cell_of(woodcutter), home);
    assert_ne!(cell_of(other), home);
    let (older, newer) = {
        let p = ai.player_mut(P0);
        let older = p.plan.add_need(WOOD, Some(sawmill), 0.0);
        let newer = p.plan.add_need(WOOD, Some(sawmill), 0.0);
        // The newer need is given the home cell first.
        p.assign_cell(&config(), newer, home, None);
        p.assign_cell(&config(), older, home, None);
        assert_eq!(p.plan.need(newer).unwrap().production, None);
        assert_eq!(p.plan.first_need_in(home, |g| g == WOOD), Some(newer));
        (older, newer)
    };
    // A woodcutter no need ordered, announced in the home cell.
    with_building(&mut world, WOODCUTTER, woodcutter);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let prod = p
        .plan
        .prods
        .iter()
        .find(|x| x.building == Some(woodcutter))
        .unwrap()
        .clone();
    assert_eq!(prod.need, Some(newer), "the first in the cell's list");
    assert_eq!(p.plan.need(newer).unwrap().production, Some(prod.id));
    assert_eq!(p.plan.need(older).unwrap().production, None);
    assert_eq!(p.related_buildings(sawmill), [woodcutter]);
    // In another cell with no need of wood its production stays free; a
    // need given that cell later takes it, placed as it is.
    with_building(&mut world, WOODCUTTER, other);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    let free = p
        .plan
        .prods
        .iter()
        .find(|x| x.building == Some(other))
        .unwrap()
        .clone();
    assert_eq!(free.need, None);
    p.assign_cell(&config(), older, cell_of(other), None);
    assert_eq!(p.plan.need(older).unwrap().production, Some(free.id));
    // The partners in the consumer's own need list order (T51): the older
    // need was tied to it first.
    assert_eq!(p.related_buildings(sawmill), [other, woodcutter]);
}

// T10 (as corrected by AI-CON-9.2 and T43, 2026-10-26 night): a woodcutter
// whose trees are gone is kept if its cell has a forester and it is the
// cell's only wood production; without the forester, or with a second one,
// it is torn down.
// spec: AI-CON-9.1, AI-CON-9.2 (T10, T43)
#[test]
fn t10_last_woodcutter_stays() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    with_building(&mut world, WOODCUTTER, (26, 22));
    let snapshot = finished_snapshot(&world, (26, 22), 50);
    ai.update_with(&world, P0, &snapshot);
    ai.player_mut(P0).destruction_cursor = 1;
    let (_, commands) =
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.building_destruction(ctx));
    assert!(removed(&commands), "no forester in the cell: {commands:?}");
    let cell = cell_of((26, 22));
    ai.player_mut(P0).cells.get_mut(cell).unwrap().forester = Some((28, 22));
    ai.player_mut(P0).destruction_cursor = 1;
    let (_, commands) =
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.building_destruction(ctx));
    assert!(!removed(&commands), "{commands:?}");
    with_building(&mut world, WOODCUTTER, (24, 24));
    let snapshot = finished_snapshot(&world, (26, 22), 50);
    ai.update_with(&world, P0, &snapshot);
    ai.player_mut(P0).destruction_cursor = 1;
    let (_, commands) =
        ai.with_snapshot(&world, P0, &snapshot, |p, ctx| p.building_destruction(ctx));
    assert!(removed(&commands));
}

// T11: a flag with no building, no wares and one road without carrier is
// removed in the flag check.
// spec: AI-CON-9.5 (T11)
#[test]
fn t11_dead_end_flag_is_removed() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    for at in [(23, 24), (26, 24)] {
        world.submit(Command::PlaceFlag { player: P0, at });
    }
    world.step();
    assert!(
        world.last_results().iter().all(|r| r.outcome == Ok(())),
        "{:?}",
        world.last_results()
    );
    world.submit(Command::BuildRoad {
        player: P0,
        from: (23, 24),
        to: (26, 24),
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()), "road");
    assert_eq!(world.roads().filter(|r| r.owner == P0).count(), 1);
    let mut removed_flags = Vec::new();
    for _ in 0..40 {
        let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.flag_destruction(ctx));
        for c in commands {
            if let Command::RemoveFlag { at, .. } = c {
                removed_flags.push(at);
            }
        }
    }
    assert!(removed_flags.contains(&(23, 24)) || removed_flags.contains(&(26, 24)));
    // The headquarters' own flag (a building behind it) is never removed.
    let hq_flag = crate::view::flag_of(&world, ai.player_mut(P0).home).unwrap();
    assert!(!removed_flags.contains(&hq_flag));
}

// AI-CON-9.5 (AI-CON T50): the flag run reads the list with the flags made
// earlier in the same think: with only the headquarters' flag and the cursor
// at 1, a site placed in this think makes the list two long, so the run tests
// the new flag's place (nothing, it is new) and the cursor becomes 2; the
// next run finds the cursor at the end and starts again at the first flag.
// spec: AI-CON-9.5 (AI-CON T50)
#[test]
fn t50_the_flag_run_counts_this_thinks_flags() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    assert_eq!(world.flags_in_order(P0).len(), 1);
    ai.player_mut(P0).flag_cursor = 1;
    ai.with_ctx(&world, P0, |p, ctx| {
        ctx.out.push(Command::PlanBuilding {
            player: P0,
            building: STONE_PIT,
            at: (26, 22),
            records: 0,
        });
        p.flag_destruction(ctx);
    });
    assert_eq!(ai.player_mut(P0).flag_cursor, 2);
    // Without the new flag: at the end, again from the first.
    ai.with_ctx(&world, P0, |p, ctx| p.flag_destruction(ctx));
    assert_eq!(ai.player_mut(P0).flag_cursor, 1);
}

// AI-CON-9.5 (corrected 2026-10-27 night, AI-CON T47): a flag with one road
// is kept only while that road's carrier or donkey carries a ware: the end
// of a dead-end road from the headquarters' flag is removed while its
// assigned carrier walks empty, and kept while he carries a ware from it.
// spec: AI-CON-9.5 (AI-CON T47)
#[test]
fn a_lone_flag_is_kept_only_while_its_carrier_carries() {
    for carried in [false, true] {
        // The town rules: the headquarters has helpers to send as carriers.
        let rules = crate::fixture::town_rules();
        let config = crate::fixture::config_for(&rules);
        let mut world = crate::fixture::world_on(1, &[0], &Extras::default(), rules);
        let mut ai = start_with(&world, config, 2);
        let hq_flag = crate::view::flag_of(&world, ai.player_mut(P0).home).unwrap();
        let end = (23, 24);
        world.submit(Command::PlaceFlag {
            player: P0,
            at: end,
        });
        world.step();
        world.submit(Command::BuildRoad {
            player: P0,
            from: hq_flag,
            to: end,
        });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()), "road");
        let road = |world: &World| {
            world
                .roads()
                .find(|r| r.path.last() == Some(&end) || r.path.first() == Some(&end))
                .cloned()
                .unwrap()
        };
        for _ in 0..400 {
            if road(&world).carrier.is_some() {
                break;
            }
            world.step();
        }
        assert!(road(&world).carrier.is_some(), "a carrier assigned");
        if carried {
            world.submit(Command::DropWare {
                player: P0,
                good: WOOD,
                at: end,
                destination: hq_flag,
            });
            world.step();
            let held = |world: &World| {
                world
                    .wares()
                    .any(|(_, w)| matches!(w.location, s2_sim::WareLocation::Carried(_)))
            };
            for _ in 0..400 {
                if held(&world) {
                    break;
                }
                world.step();
            }
            assert!(held(&world), "the carrier holds the ware");
        }
        let mut removed = false;
        for _ in 0..40 {
            let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.flag_destruction(ctx));
            removed |= commands
                .iter()
                .any(|c| matches!(c, Command::RemoveFlag { at, .. } if *at == end));
        }
        assert_eq!(removed, !carried, "carried {carried}");
    }
}

// T12: S = 9216, stock(fish) 100, stock2 0: fishers are switched off.
// spec: AI-CON-10.1 (T12)
#[test]
fn t12_production_control() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    with_building(&mut world, FISHER, (26, 22));
    let snapshot = finished_snapshot(&world, (26, 22), 50);
    {
        let p = ai.player_mut(P0);
        p.statistic = (1, 9216);
        p.econ = Economy::from_counts(&[(FISH, 100)], &[]);
    }
    let (_, commands) = ai.with_snapshot(&world, P0, &snapshot, |p, ctx| {
        p.statistic = (ctx.world.tick().max(1), 9216);
        p.production_control(ctx)
    });
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::ProductionToggle { at: (26, 22), .. })),
        "{commands:?}"
    );
    // Below the limit a disabled fisher is switched on again: not tested here
    // as the building starts enabled; below the limit nothing changes.
    {
        let p = ai.player_mut(P0);
        p.control_cursor = 0;
        p.econ = Economy::from_counts(&[(FISH, 30)], &[]);
    }
    let (_, commands) = ai.with_snapshot(&world, P0, &snapshot, |p, ctx| {
        p.statistic = (ctx.world.tick().max(1), 9216);
        p.production_control(ctx)
    });
    assert!(commands.is_empty(), "{commands:?}");
}

// T13: tools: saw missing: a tools need is created; with the locksmithery
// finished the saw slider is 1.0 and the others 0.001.
// spec: AI-CON-10.2 (T13)
#[test]
fn t13_locksmith() {
    let (world, mut ai) = locksmith_ready(config(), &[(AXE, 3)]);
    assert_eq!(chosen_slot(&mut ai, &world), [(0, 1000), (1, 1)]);
}

/// A player with a finished locksmithery linked to the tools need, its stock
/// `stock`; the run that made the tools need has passed.
fn locksmith_ready(config: crate::AiConfig, stock: &[(u32, u32)]) -> (World, AiSystem) {
    let mut world = world(&[0]);
    let mut ai = start_with(&world, config, 2);
    ai.player_mut(P0).need_stage = 4;
    ai.player_mut(P0).econ = Economy::from_counts(stock, &[]);
    ai.with_ctx(&world, P0, |p, ctx| p.locksmith(ctx));
    assert_eq!(count(&mut ai, &world, TOOLS), 1);
    // Build the producer and link it to the need.
    with_building(&mut world, LOCKSMITH, (26, 22));
    world.submit(Command::DebugSupplySite {
        player: P0,
        at: (26, 22),
        wares: vec![(STONE, 2)],
        workers: true,
    });
    for _ in 0..4000 {
        world.step();
        if world
            .building_at((26, 22))
            .is_some_and(|b| !b.under_construction)
        {
            break;
        }
    }
    assert!(world
        .building_at((26, 22))
        .is_some_and(|b| !b.under_construction));
    {
        let p = ai.player_mut(P0);
        let cell = cell_of((26, 22));
        let prod = p.plan.add_production(TOOLS, cell, None);
        p.plan
            .prods
            .iter_mut()
            .find(|x| x.id == prod)
            .unwrap()
            .building = Some((26, 22));
        let need = p.plan.needs.iter().find(|n| n.good == TOOLS).unwrap().id;
        p.plan.need_mut(need).unwrap().production = Some(prod);
    }
    (world, ai)
}

/// One locksmith run: the slider commands sent, (slot, per mille).
fn chosen_slot(ai: &mut AiSystem, world: &World) -> Vec<(u8, u32)> {
    let (_, commands) = ai.with_ctx(world, P0, |p, ctx| p.locksmith(ctx));
    commands
        .iter()
        .filter_map(|c| match c {
            Command::ToolWeight {
                slot, per_mille, ..
            } => Some((*slot, *per_mille)),
            _ => None,
        })
        .collect()
}

// T110: the missing tools in good-id order, the id read as signed. Slots
// rolling pin (in stock), pot, fishing rod, and a tool whose id is negative
// as a signed number, all three missing. The run that made the tools need
// stores no last tool; with none handled (or the last handled not missing)
// the last of the list is chosen: the fishing rod, not the pot (the first
// after the rolling pin by slot) nor the negative id (the last unsigned).
// The rod is then the last handled; while it is still missing the next run
// goes on cyclically to the first of the list, the negative id.
// spec: AI-CON-10.2 (T110)
#[test]
fn t110_the_locksmith_takes_the_last_missing_tool() {
    const PIN: u32 = 40;
    const POT: u32 = 41;
    const ROD: u32 = 42;
    const NEG: u32 = 0x8000_0000;
    let mut config = config();
    config.tool_slots = vec![PIN, POT, ROD, NEG];
    let (world, mut ai) = locksmith_ready(config, &[(PIN, 1)]);
    assert_eq!(ai.player_mut(P0).locksmith_last, usize::MAX, "not stored");
    let rod = chosen_slot(&mut ai, &world);
    assert!(rod.contains(&(2, 1000)), "the fishing rod: {rod:?}");
    assert_eq!(ai.player_mut(P0).locksmith_last, 2);
    let next = chosen_slot(&mut ai, &world);
    assert!(
        next.contains(&(3, 1000)),
        "after the rod, the first: {next:?}"
    );
    assert_eq!(ai.player_mut(P0).locksmith_last, 3);
    // The last handled (the pin) in stock again: the last of the list.
    ai.player_mut(P0).locksmith_last = 0;
    let again = chosen_slot(&mut ai, &world);
    assert!(again.contains(&(2, 1000)), "{again:?}");
}

// AI-CON-10.2, 8.3 b (AI-CON T54): the tools need is made without a
// consumer building and is not tied to the headquarters, so the
// locksmithery's tools production adds no partner to its route task (with
// the headquarters as consumer it would be the headquarters).
// spec: AI-CON-10.2 (T54), AI-CON-8.3
#[test]
fn t54_the_tools_need_has_no_consumer() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    ai.player_mut(P0).need_stage = 4;
    ai.player_mut(P0).econ = Economy::from_counts(&[(AXE, 3)], &[]);
    ai.with_ctx(&world, P0, |p, ctx| p.locksmith(ctx));
    let p = ai.player_mut(P0);
    let need = p.plan.needs.iter().find(|n| n.good == TOOLS).unwrap();
    assert_eq!((need.consumer, need.tied), (None, 0));
    let need = need.id;
    let at = (26, 22);
    let prod = p.plan.add_production(TOOLS, cell_of(at), None);
    let x = p.plan.prods.iter_mut().find(|x| x.id == prod).unwrap();
    x.building = Some(at);
    x.need = Some(need);
    p.plan.need_mut(need).unwrap().production = Some(prod);
    assert_eq!(p.related_buildings(at), []);
    let home = p.home;
    p.plan.need_mut(need).unwrap().consumer = Some(home);
    assert_eq!(p.related_buildings(at), [home], "the old rule's partner");
}

// A whole economy start on the synthetic map: the AI orders its first
// buildings, places them and joins them to the headquarters with roads, and
// none of its commands is rejected.
// spec: AI-CON-5.3, AI-CON-5.4, AI-CON-8.1
#[test]
fn the_ai_builds_and_connects_its_first_buildings() {
    let trees: Vec<(u32, u32)> = (0..8u32).map(|i| (22 + i, 14)).collect();
    let rocks: Vec<(u32, u32)> = (0..8u32).map(|i| (22 + i, 27)).collect();
    let mut world = world_with(
        &[0],
        &Extras {
            trees,
            rocks,
            fish: Vec::new(),
        },
    );
    let mut ai = start(&world, 2);
    let mut rejected = Vec::new();
    for _ in 0..3000 {
        for c in ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        ) {
            world.submit(c);
        }
        world.step();
        for r in world.last_results() {
            // Two roads asked for in one tick may cross; that is the road
            // planner's NoRoute, not a mistake of the placement (nor its
            // refusal of a long road, AI-CON-8.1 b).
            let road = matches!(r.command, Command::PlanRoad { .. });
            if r.outcome.is_err() && !road {
                rejected.push(format!("{:?}", r));
            }
        }
    }
    assert!(rejected.is_empty(), "{rejected:?}");
    let types: Vec<u32> = world
        .buildings()
        .filter(|b| b.owner == P0)
        .map(|b| b.building_type)
        .collect();
    assert!(types.contains(&STONE_PIT), "{types:?}");
    assert!(types.contains(&SAWMILL), "{types:?}");
    assert!(world.roads().filter(|r| r.owner == P0).count() >= 2);
}

// AI-CON-8.1: a building whose flag is not joined to the warehouse network
// gets a road to the nearest flag of it.
#[test]
fn street_construction_joins_a_new_building_to_the_headquarters() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    with_building(&mut world, STONE_PIT, (24, 22));
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    // The target is remembered, five attempts measure, the sixth builds.
    let mut commands = Vec::new();
    for run in 0..8 {
        let (_, out) = ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
        if !out.is_empty() {
            assert_eq!(run, 6, "the road is built in the seventh run");
            commands = out;
            break;
        }
    }
    let hq_flag = crate::view::flag_of(&world, ai.player_mut(P0).home).unwrap();
    let flag = crate::view::flag_of(&world, (24, 22)).unwrap();
    // The headquarters' flag is first in the list; it is joined to the
    // nearest flag of another network, the new building's.
    assert!(
        commands.iter().any(|c| matches!(c,
            Command::PlanRoad { from, to, .. }
                if (*from == flag && *to == hq_flag) || (*from == hq_flag && *to == flag))),
        "{commands:?}"
    );
    for c in commands {
        world.submit(c);
    }
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    // A joined building is left alone.
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert!(commands.is_empty(), "{commands:?}");
}

// AI-CON T60 (AI-CON-8.1f): the street run's cursor is an index in the flag
// list. Working on the third flag, the second is removed: the list closes
// up, the cursor lies past its end, and the next run restarts with the
// first flag, the work on the third dropped.
// spec: AI-CON-8.1f (AI-CON T60), AI-CON-8.1d
#[test]
fn t60_the_street_cursor_is_a_list_index() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let (first, second) = ((24, 22), (21, 26));
    with_building(&mut world, STONE_PIT, first);
    with_building(&mut world, WOODCUTTER, second);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let flags = world.flags_in_order(P0);
    assert_eq!(flags.len(), 3);
    ai.player_mut(P0).street.cursor = 2;
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    let worked = |ai: &mut AiSystem| ai.player_mut(P0).street.work.as_ref().map(|w| w.flag);
    assert_eq!(worked(&mut ai), Some(flags[2]));
    world.submit(Command::RemoveFlag {
        player: P0,
        at: flags[1],
    });
    world.step();
    assert_eq!(world.flags_in_order(P0), [flags[0], flags[2]]);
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), Some(flags[0]), "restarted with the first");
    assert_eq!(ai.player_mut(P0).street.cursor, 0);
}

// AI-CON T97 (AI-CON-8.1f, Q-380): working on the third flag, the second is
// removed: the list closes up and the cursor names the fourth flag. With
// the work held the cursor moves on at once, the fourth flag gets this run
// as a new F (its T remembered), and the next run finds the cursor past the
// end and restarts with the first: no attempts on the fourth.
// spec: AI-CON-8.1f (AI-CON T97, T60), AI-CON-8.1d
#[test]
fn t97_the_street_cursor_moves_on_when_the_list_closes_up() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    for (kind, at) in [
        (STONE_PIT, (24, 22)),
        (WOODCUTTER, (21, 26)),
        (STONE_PIT, (25, 18)),
    ] {
        with_building(&mut world, kind, at);
    }
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let flags = world.flags_in_order(P0);
    assert_eq!(flags.len(), 4);
    let worked = |ai: &mut AiSystem| ai.player_mut(P0).street.work.as_ref().map(|w| w.flag);
    let cursor = |ai: &mut AiSystem| ai.player_mut(P0).street.cursor;
    ai.player_mut(P0).street.cursor = 2;
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), Some(flags[2]));
    world.submit(Command::RemoveFlag {
        player: P0,
        at: flags[1],
    });
    world.step();
    assert_eq!(world.flags_in_order(P0), [flags[0], flags[2], flags[3]]);
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), Some(flags[3]), "a new F, its T remembered");
    assert_eq!(cursor(&mut ai), 3, "the cursor moved on before");
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), Some(flags[0]), "restarted with the first");
    assert_eq!(cursor(&mut ai), 0);
}

// AI-CON T106 (AI-CON-8.1f, E-ai-074): passing on to the next flag raises
// the street cursor by one from its current value. After the T97 move the
// cursor already stands one place past the flag handled as a new F; when
// that F is passed on (here a flag without a building) the cursor ends two
// places past the old place: the flag right after F is not looked at in
// that round, and the next run restarts with the first (the old "handled
// flag's place plus one" took the stone pit's flag next).
// spec: AI-CON-8.1f (AI-CON T106, T97), AI-CON-8.1d
#[test]
fn t106_the_street_cursor_steps_on_from_where_it_is() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    with_building(&mut world, STONE_PIT, (24, 22));
    with_building(&mut world, WOODCUTTER, (21, 26));
    world.submit(Command::PlaceFlag {
        player: P0,
        at: (20, 14),
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    with_building(&mut world, STONE_PIT, (25, 18));
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let flags = world.flags_in_order(P0);
    assert_eq!(flags.len(), 5);
    assert_eq!(flags[3], (20, 14));
    let worked = |ai: &mut AiSystem| ai.player_mut(P0).street.work.as_ref().map(|w| w.flag);
    let cursor = |ai: &mut AiSystem| ai.player_mut(P0).street.cursor;
    ai.player_mut(P0).street.cursor = 2;
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), Some(flags[2]));
    world.submit(Command::RemoveFlag {
        player: P0,
        at: flags[1],
    });
    world.step();
    assert_eq!(
        world.flags_in_order(P0),
        [flags[0], flags[2], flags[3], flags[4]]
    );
    // T97: the cursor moves to 3 and the bare flag is handled as a new F;
    // it is passed on: the cursor rises from 3 to 4.
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), None);
    assert_eq!(cursor(&mut ai), 4, "two places past the old place");
    ai.with_ctx(&world, P0, |p, ctx| p.street_construction(ctx));
    assert_eq!(worked(&mut ai), Some(flags[0]), "restarted with the first");
    assert_eq!(cursor(&mut ai), 0);
}

// AI-CON T102a (AI-CON-2.5b, the list order is the order of giving): a
// need that loses its cell (its producer torn down, AI-CON-9.3) and is
// given the same cell again goes to the end of the cell's need list, behind
// a younger need, so the next producer announced there is tied to the
// younger one (R6 player 2: wood need 15177, given its cell again after the
// woodcutter (56, 22) went at 5810, stands behind 15245 in the heap of
// 8500, and the woodcutter (59, 30) of 8824 is tied to 15245).
// spec: AI-CON-2.5b (T102a), AI-CON-9.3
#[test]
fn t102a_a_need_given_its_cell_again_goes_to_the_end() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let (sawmill, first, second) = ((30, 30), (26, 22), (24, 26));
    let home = cell_of(ai.player_mut(P0).home);
    assert_eq!((cell_of(first), cell_of(second)), (home, home));
    with_building(&mut world, WOODCUTTER, first);
    ai.update_with(&world, P0, &crate::snapshot::Snapshot::take(&world));
    let (old, young) = {
        let p = ai.player_mut(P0);
        let old = p.plan.add_need(WOOD, Some(sawmill), 0.0);
        let young = p.plan.add_need(WOOD, Some(sawmill), 0.0);
        p.assign_cell(&config(), old, home, None);
        p.assign_cell(&config(), young, home, None);
        assert!(p.plan.need(old).unwrap().production.is_some());
        assert_eq!(p.plan.need(young).unwrap().production, None);
        (old, young)
    };
    // The woodcutter is torn down (AI-CON-9.3, T111: the old need holds a
    // square): the old need loses its cell and is given it again, behind
    // the young one.
    {
        let p = ai.player_mut(P0);
        p.plan.need_mut(old).unwrap().square = Some(((33, 14), 1));
        p.tear_down_needs(first);
    }
    world.submit(Command::RemoveBuilding {
        player: P0,
        at: first,
    });
    world.step();
    ai.update_with(&world, P0, &crate::snapshot::Snapshot::take(&world));
    let p = ai.player_mut(P0);
    assert_eq!(p.plan.need(old).unwrap().cell, None);
    p.assign_cell(&config(), old, home, None);
    assert_eq!(p.plan.first_need_in(home, |g| g == WOOD), Some(young));
    with_building(&mut world, WOODCUTTER, second);
    ai.update_with(&world, P0, &crate::snapshot::Snapshot::take(&world));
    let p = ai.player_mut(P0);
    let prod = p
        .plan
        .prods
        .iter()
        .find(|x| x.building == Some(second))
        .unwrap()
        .id;
    assert_eq!(p.plan.need(young).unwrap().production, Some(prod));
    assert_eq!(p.plan.need(old).unwrap().production, None);
}

// AI-CON T101 (AI-CON-8.3 a, the state goes with the task): when the
// current building's record is gone, the next run only moves the cursor to
// the next record with a higher number, keeping k, i, t, w and X, and the
// run after takes it up as B with that state (R6: player 2's hunter's hut
// torn down at 6631; the run of 6649 moves the cursor to 14678 with i 3, t 2
// and X kept, the run of 6686 asks the road to the flag at X's last entry).
// A new building on the removed one's field is a new record, taken up the
// same way. A restart and a planting building's skip clear the state.
// spec: AI-CON-8.3 (T101)
#[test]
fn t101_the_route_state_goes_with_the_task() {
    use crate::roads::RouteWork;
    let (hut, next, older) = ((46, 20), (58, 24), (30, 30));
    let left = RouteWork {
        cursor: 5,
        current: Some(hut),
        k: 0,
        i: 3,
        t: 2,
        w: 1,
        x: vec![(50, 21), (51, 20)],
    };
    let mut work = left.clone();
    // The hut's record (5) is gone; the next record is 9.
    let records = [(2, older), (9, next)];
    assert_eq!(work.take(&records, |_| false), None, "only the cursor");
    assert_eq!(
        work,
        RouteWork {
            cursor: 9,
            ..left.clone()
        }
    );
    assert_eq!(work.take(&records, |_| false), Some((9, next)));
    assert_eq!(
        work,
        RouteWork {
            cursor: 9,
            current: Some(next),
            ..left.clone()
        },
        "taken up with the state the hut left"
    );
    assert_eq!(work.take(&records, |_| false), Some((9, next)), "still B");
    // Its record gone too, a new building on its field (12): first only
    // the cursor, then taken up, the state still kept.
    let records = [(2, older), (12, next)];
    assert_eq!(work.take(&records, |_| false), None);
    assert_eq!((work.cursor, work.i, work.t), (12, 3, 2));
    assert_eq!(work.take(&records, |_| false), Some((12, next)));
    assert_eq!(work.x, left.x);
    // A planting building is skipped with the state cleared.
    let mut work = left.clone();
    let records = [(2, older), (9, next)];
    assert_eq!(work.take(&records, |b| b == next), None);
    assert_eq!(
        work,
        RouteWork {
            cursor: 10,
            ..RouteWork::default()
        }
    );
    // No record at or above the cursor: a restart, the state cleared.
    let mut work = RouteWork {
        cursor: 20,
        ..left.clone()
    };
    assert_eq!(work.take(&records, |_| false), None);
    assert_eq!(work, RouteWork::default());
}

// AI-CON T99 (AI-CON-8.3c): in the route optimizer's direct variant, G (the
// last flag of X) and Fn without a ware route between them do not count as
// too long: no road is asked, and X becomes just Fn.
// spec: AI-CON-8.3 (AI-CON T99)
#[test]
fn t99_no_ware_route_between_g_and_fn_is_not_too_long() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let hq_flag = crate::view::flag_of(&world, ai.player_mut(P0).home).unwrap();
    let lone = (24, 24);
    world.submit(Command::PlaceFlag {
        player: P0,
        at: lone,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()), "a lone flag");
    let route = &mut ai.player_mut(P0).street.route;
    (route.w, route.x) = (0, vec![lone]);
    // Run 0 looks from B's flag, whose nearest own flag is itself: Fn.
    let (_, out) = ai.with_ctx(&world, P0, |p, ctx| p.direct_variant(ctx, hq_flag, lone));
    assert!(out.is_empty(), "no road asked: {out:?}");
    assert_eq!(ai.player_mut(P0).street.route.x, [hq_flag]);
}

// AI-CON T104 (AI-CON-8.1e corrected): no settler blocks a walk path: the
// street strategies' second legs may walk over the hit road's fields, and
// the road's carrier waiting on its idle field does not close the way
// across it (R6: player 2's strategy-4 leg F -> P over its idle carrier's
// field (58, 26), L 3, L2 2, best 4; attempt 6 of 9908 removes R and builds
// two roads). The settler test is withdrawn: T61 holds by T74's exclusion,
// T64 by the tree, T79's worker was walking, T80's and T95's second legs
// went over R's idle carrier.
// spec: AI-CON-8.1e (AI-CON T104)
#[test]
fn t104_no_settler_blocks_the_walk_paths() {
    let rules = crate::fixture::town_rules();
    let mut world = crate::fixture::world_on(1, &[0], &Extras::default(), rules);
    let hq_flag = crate::view::flag_of(&world, (20, 20)).unwrap();
    let end = (25, 19);
    world.submit(Command::PlaceFlag {
        player: P0,
        at: end,
    });
    world.step();
    world.submit(Command::BuildRoad {
        player: P0,
        from: hq_flag,
        to: end,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()), "road");
    let path = world.roads().next().unwrap().path.clone();
    let idle = path[path.len() / 2];
    for _ in 0..1500 {
        let carrier = world.carriers().first().cloned();
        if carrier.is_some_and(|c| c.walker.at == idle && !c.walker.is_walking()) {
            break;
        }
        world.step();
    }
    let carrier = world.carriers()[0].clone();
    assert_eq!(carrier.walker.at, idle, "on his idle field");
    assert!(!carrier.walker.is_walking());
    // Two fields on either side of a road field, off the road: the only
    // 2-step way between them crosses it.
    let terrain = world.terrain();
    let across = |q| {
        (0u8..3)
            .find_map(|d| {
                let a = terrain.neighbour(q, d)?;
                let b = terrain.neighbour(q, d + 3)?;
                (!path.contains(&a) && !path.contains(&b)).then_some((a, b))
            })
            .expect("fields beside the road")
    };
    let walk = |(a, b)| crate::roads::walk_path(&world, P0, a, b, &[], &path);
    let free = path[path.len() / 2 - 1];
    assert_eq!(walk(across(free)), Some(2), "nobody stands on {free:?}");
    assert_eq!(walk(across(idle)), Some(2), "his carrier on {idle:?}");
}

// AI-CON-8.2: the street optimizer puts a flag on a road's first waypoint
// from the second field on where one fits.
#[test]
fn street_optimizer_splits_a_long_road() {
    let mut world = world(&[0]);
    let mut config = config();
    config.profiles[1].street_optimizing = true;
    let mut ai = start_with(&world, config, 2);
    for at in [(17, 24), (24, 24)] {
        world.submit(Command::PlaceFlag { player: P0, at });
    }
    world.step();
    world.submit(Command::BuildRoad {
        player: P0,
        from: (17, 24),
        to: (24, 24),
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    let road = world.roads().next().unwrap().path.clone();
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.street_optimizer(ctx));
    let flag_at = commands.iter().find_map(|c| match c {
        Command::PlaceFlag { at, .. } => Some(*at),
        _ => None,
    });
    let at = flag_at.expect("a flag");
    assert!(road[1..road.len() - 1].contains(&at));
    // Weak (street optimizing off) does nothing.
    let mut weak = start(&world, 1);
    let (_, commands) = weak.with_ctx(&world, P0, |p, ctx| p.street_optimizer(ctx));
    assert!(commands.is_empty());
}

// AI-CON T58 (AI-CON-8.2): the street optimizer's cursor is compared with
// the road list's length only when read: after the run on the last road it
// is one past the end, so a road made since is the next one handled; with
// none made it starts again at the first.
// spec: AI-CON-8.2 (AI-CON T58)
#[test]
fn t58_the_optimizer_cursor_goes_on_to_new_roads() {
    let mut world = world(&[0]);
    let mut config = config();
    config.profiles[1].street_optimizing = true;
    let mut ai = start_with(&world, config, 2);
    let road = |world: &mut World, from: (u32, u32), to: (u32, u32)| {
        for at in [from, to] {
            world.submit(Command::PlaceFlag { player: P0, at });
        }
        world.step();
        world.submit(Command::BuildRoad {
            player: P0,
            from,
            to,
        });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()));
        world.roads().last().unwrap().path.clone()
    };
    let first = road(&mut world, (17, 24), (24, 24));
    let run = |ai: &mut AiSystem, world: &World| {
        let (_, commands) = ai.with_ctx(world, P0, |p, ctx| p.street_optimizer(ctx));
        commands.iter().find_map(|c| match c {
            Command::PlaceFlag { at, .. } => Some(*at),
            _ => None,
        })
    };
    let on = |path: &Vec<(u32, u32)>, at: Option<(u32, u32)>| {
        at.is_some_and(|at| path[1..path.len() - 1].contains(&at))
    };
    assert!(on(&first, run(&mut ai, &world)));
    assert_eq!(ai.player_mut(P0).street.optimizer_cursor, 1, "not wrapped");
    let second = road(&mut world, (17, 28), (24, 28));
    assert!(on(&second, run(&mut ai, &world)), "the new road next");
    assert_eq!(ai.player_mut(P0).street.optimizer_cursor, 2);
    assert!(on(&first, run(&mut ai, &world)), "past the end: the first");
}

// AI-CON T62 (AI-CON-8.2): the street optimizer's road list is in creation
// order, by object number, not in the order the roads are stored: a split
// road's first half keeps the removed road's place but is newer than the
// road made after it.
// spec: AI-CON-8.2 (AI-CON T62)
#[test]
fn t62_the_optimizer_goes_by_creation_order() {
    let mut world = world(&[0]);
    let mut config = config();
    config.profiles[1].street_optimizing = true;
    let mut ai = start_with(&world, config, 2);
    let road = |world: &mut World, from: (u32, u32), to: (u32, u32)| {
        for at in [from, to] {
            world.submit(Command::PlaceFlag { player: P0, at });
        }
        world.step();
        world.submit(Command::BuildRoad {
            player: P0,
            from,
            to,
        });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()));
    };
    road(&mut world, (17, 24), (24, 24));
    road(&mut world, (17, 28), (24, 28));
    // The first road split: its first half stays in its place.
    world.submit(Command::PlaceFlag {
        player: P0,
        at: (20, 24),
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()), "split");
    let stored: Vec<_> = world.roads().map(|r| (r.number, r.path[0])).collect();
    let mut by_number = stored.clone();
    by_number.sort();
    assert_ne!(stored, by_number, "slots and numbers differ");
    let oldest = world.roads().min_by_key(|r| r.number).unwrap().path.clone();
    assert_eq!(oldest[0], (17, 28), "the second road is now the oldest");
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.street_optimizer(ctx));
    let at = commands.iter().find_map(|c| match c {
        Command::PlaceFlag { at, .. } => Some(*at),
        _ => None,
    });
    assert!(
        at.is_some_and(|at| oldest[1..oldest.len() - 1].contains(&at)),
        "{commands:?}"
    );
}

// AI-CON-3.2: a need whose input has no producer and low stock first gets a
// need for the input.
#[test]
fn inputs_come_first() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    {
        let p = ai.player_mut(P0);
        p.plan.add_need(PLANK, Some((1, 1)), 0.0);
        p.econ = Economy::from_counts(&[(WOOD, 5), (PLANK, 5)], &[]);
    }
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let p = ai.player_mut(P0);
    assert!(p.plan.needs.iter().any(|n| n.good == WOOD));
    // The plank need still has no cell.
    assert!(p
        .plan
        .needs
        .iter()
        .find(|n| n.good == PLANK)
        .unwrap()
        .cell
        .is_none());
    // With enough wood in stock no need for it is made.
    let mut other = start(&world, 2);
    {
        let p = other.player_mut(P0);
        p.plan.add_need(PLANK, Some((1, 1)), 0.0);
        p.econ = Economy::from_counts(&[(WOOD, 50)], &[]);
    }
    other.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let p = other.player_mut(P0);
    assert!(!p.plan.needs.iter().any(|n| n.good == WOOD));
    assert_eq!(
        p.plan.needs.iter().find(|n| n.good == PLANK).unwrap().cell,
        Some(cell_of(p.home))
    );
}

// AI-CON-3.4 and 3.5: a wood need takes the cell with the most trees; with no
// trees in reach it waits, and the cell system sets an expansion target next
// to trees outside the land, once the trees' landmass is a conquered
// continent (the landmass of one of the player's cells, AI-2.3).
// spec: AI-CON-3.4, AI-CON-3.5, AI-CON-1.1b
#[test]
fn a_resource_need_finds_a_square_or_drives_expansion() {
    // 9 trees in the headquarters' cell.
    let trees_world = forest_world(9, 0);
    let mut ai = start(&trees_world, 2);
    ai.player_mut(P0).plan.add_need(WOOD, Some((1, 1)), 0.0);
    ai.player_mut(P0).econ = Economy::from_counts(&[(WOOD, 50)], &[]);
    ai.with_ctx(&trees_world, P0, |p, ctx| p.needs_arrangement(ctx));
    let need = ai.player_mut(P0).plan.needs[0].clone();
    assert_eq!(need.cell, Some(cell_of(ai.player_mut(P0).home)));
    assert!(need.square.is_some());
    assert!(!need.waiting);

    // Trees only outside the territory: the need waits.
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let landmass = ai.player_mut(P0).home_landmass;
    ai.player_mut(P0).resources.set_entries(
        (8, 5),
        vec![crate::resources::Entry {
            good: WOOD,
            kind: crate::resources::SURFACE,
            landmass,
            amount: 5,
        }],
    );
    ai.player_mut(P0).plan.add_need(WOOD, Some((1, 1)), 0.0);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    assert!(ai.player_mut(P0).plan.needs[0].waiting);
    // AI-CON-3.5: before the first run of the conquered-continents part
    // no entry is on a conquered continent: the need is taken off the
    // waiting list, no target.
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
    assert!(!ai.player_mut(P0).plan.needs[0].waiting);
    assert!(ai.player_mut(P0).expansion.pending[0].is_none());
    let id = ai.player_mut(P0).plan.needs[0].id;
    ai.player_mut(P0).plan.put_waiting(id);
    ai.player_mut(P0).conquered_continents();
    assert_eq!(ai.player_mut(P0).conquered, [landmass]);
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
    // The need is taken off the waiting list (AI-CON-3.4) and the target is
    // the top-left field of the square holding the trees (AI-CON-3.5).
    assert!(!ai.player_mut(P0).plan.needs[0].waiting);
    let target = ai.player_mut(P0).expansion.pending[0].expect("an expansion target");
    assert_eq!(target.kind, crate::expansion::Kind::Position);
    assert_eq!(target.at, (32, 20));
}

// AI-CON-7.4: the oldest depot-wanted marker gets a depot order of weight
// 15 and its time is reset. The headquarters' cell has none (the
// headquarters is a depot, AI-CON-1.1); a woodcutter's site in the next cell
// makes that cell with the marker (AI-CON-1.6).
// spec: AI-CON-7.4, AI-CON-1.1, AI-CON-1.6
#[test]
fn depot_order_goes_to_the_oldest_marker() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let at = (14, 22);
    with_building(&mut world, WOODCUTTER, at);
    ai.update_with(&world, P0, &crate::snapshot::Snapshot::take(&world));
    for _ in 0..50 {
        world.step();
    }
    let cell = cell_of(at);
    assert_ne!(cell, cell_of(ai.player_mut(P0).home));
    ai.with_ctx(&world, P0, |p, ctx| p.depot_orders(ctx));
    let order = ai.player_mut(P0).build.slot(Slot::Depot).copied().unwrap();
    assert_eq!((order.building, order.weight), (DEPOT, 15));
    assert_eq!(order.area, Area::Cell(cell));
    assert_eq!(
        ai.player_mut(P0).cells.get(cell).unwrap().depot_wanted,
        Some(world.tick())
    );
}

// AI-CON-1.6 / 7.6: a cell with five times more ordinary buildings than
// depots is crowded and, with no expansion target pending, asks for one.
#[test]
fn a_crowded_cell_asks_for_expansion() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    // Six ordinary buildings against one depot (the castle): 5 x 1 < 6.
    let mut placed = 0;
    'fields: for y in 18..28 {
        for x in 16..30 {
            if placed == 6 {
                break 'fields;
            }
            world.submit(Command::PlaceBuilding {
                player: P0,
                building: STONE_PIT,
                at: (x, y),
            });
            world.step();
            if world.last_results()[0].outcome == Ok(()) {
                placed += 1;
            }
        }
    }
    assert_eq!(placed, 6);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let cell = cell_of(ai.player_mut(P0).home);
    assert!(ai.player_mut(P0).cells.get(cell).unwrap().crowded.is_some());
    for _ in 0..30 {
        world.step();
    }
    ai.with_ctx(&world, P0, |p, ctx| p.crowded_cells(ctx));
    let target = ai.player_mut(P0).expansion.pending[2].expect("a crowded-cell target");
    assert_eq!(target.kind, crate::expansion::Kind::Crowded);
    assert_eq!(target.at, crate::cells::origin(cell));
    // Nothing more while one is pending.
    ai.player_mut(P0).expansion.pending[2] = None;
    ai.player_mut(P0).expansion.set(
        crate::expansion::Source::Attack,
        crate::expansion::Kind::Position,
        (1, 1),
        0,
    );
    ai.with_ctx(&world, P0, |p, ctx| p.crowded_cells(ctx));
    assert!(ai.player_mut(P0).expansion.pending[2].is_none());
}

// T14 and T15 (AI-CON-1.6): the crowded marker counts military buildings (the
// headquarters is one) against ordinary ones; a harbour adds one and gives
// the weight 100 when there is no military building.
#[test]
fn t14_t15_crowded_markers() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let home = cell_of(ai.player_mut(P0).home);
    // The cell of the headquarters: 6 ordinary buildings, 5 x 1 < 6.
    let mut placed = 0;
    'fields: for y in 17..28 {
        for x in 16..30 {
            if placed == 6 {
                break 'fields;
            }
            world.submit(Command::PlaceBuilding {
                player: P0,
                building: STONE_PIT,
                at: (x, y),
            });
            world.step();
            if world.last_results()[0].outcome == Ok(()) {
                placed += 1;
            }
        }
    }
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    let marker = ai.player_mut(P0).cells.get(home).unwrap().crowded;
    assert_eq!(marker.map(|m| m.1), Some(1));

    // A cell with a harbour and one ordinary building, no military one.
    let mut snapshot = crate::snapshot::Snapshot::take(&world);
    let template = *snapshot.buildings[0].last().unwrap();
    let mut harbour = template;
    harbour.building_type = HARBOR;
    harbour.at = (56, 56);
    harbour.serial = 900;
    let mut ordinary = template;
    ordinary.building_type = STONE_PIT;
    ordinary.at = (58, 58);
    ordinary.serial = 901;
    snapshot.buildings[0].push(harbour);
    snapshot.buildings[0].push(ordinary);
    ai.update_with(&world, P0, &snapshot);
    let marker = ai
        .player_mut(P0)
        .cells
        .get(cell_of((56, 56)))
        .unwrap()
        .crowded;
    assert_eq!(marker.map(|m| m.1), Some(100));
    // The harbour list is not the depot list: the depot wish stays.
    assert!(ai
        .player_mut(P0)
        .cells
        .get(cell_of((56, 56)))
        .unwrap()
        .depot_wanted
        .is_some());
    assert!(ai
        .player_mut(P0)
        .cells
        .get(cell_of((56, 56)))
        .unwrap()
        .harbour_wanted
        .is_none());
}

// T96 (AI-CON-1.6, 7.6): the crowded marker is evaluated when a military,
// harbour or ordinary building is added and when a military or ordinary
// one is removed, not when a harbour goes or a forester comes; a set marker
// keeps its time and weight. Of equal values 7.6 takes the first cell in
// the cell list (creation order), not the lowest.
// spec: AI-CON-1.6 (AI-CON T96), AI-CON-7.6
#[test]
fn t96_crowded_is_evaluated_again_on_removal() {
    type Marker = Option<(u64, u32)>;
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let mut snapshot = crate::snapshot::Snapshot::take(&world);
    let template = *snapshot.buildings[0].last().unwrap();
    let mut serial = 900;
    let mut add = |snapshot: &mut crate::snapshot::Snapshot, kind, at| {
        let mut b = template;
        (b.building_type, b.at, b.serial) = (kind, at, serial);
        serial += 1;
        snapshot.buildings[0].push(b);
    };
    let remove = |snapshot: &mut crate::snapshot::Snapshot, at| {
        snapshot.buildings[0].retain(|b| b.at != at);
    };
    let marker = |ai: &mut AiSystem, at| -> Marker {
        ai.player_mut(P0).cells.get(cell_of(at)).unwrap().crowded
    };
    fn run(world: &mut World, ai: &mut AiSystem, snapshot: &crate::snapshot::Snapshot) {
        world.step();
        ai.update_with(world, P0, snapshot);
    }
    // A harbour alone: crowded (5 x 0 < 0 + 1), weight 100.
    let harbour = (56, 56);
    add(&mut snapshot, HARBOR, harbour);
    run(&mut world, &mut ai, &snapshot);
    let set = marker(&mut ai, harbour);
    assert_eq!(set.map(|m| m.1), Some(100));
    // Five ordinary buildings and a military one: still crowded (5 < 6);
    // the marker keeps its time and its weight.
    let ordinary = [(50, 50), (52, 50), (54, 50), (50, 52), (52, 52)];
    for at in ordinary {
        add(&mut snapshot, STONE_PIT, at);
    }
    let military = (60, 60);
    add(&mut snapshot, BARRACKS, military);
    run(&mut world, &mut ai, &snapshot);
    assert_eq!(marker(&mut ai, harbour), set);
    // An ordinary building goes: 5 < 5 is false, cleared.
    remove(&mut snapshot, ordinary[0]);
    run(&mut world, &mut ai, &snapshot);
    assert_eq!(marker(&mut ai, harbour), None);
    // The military one goes: 0 < 5, set anew (now, weight 100).
    remove(&mut snapshot, military);
    run(&mut world, &mut ai, &snapshot);
    assert_eq!(marker(&mut ai, harbour), Some((world.tick(), 100)));

    // Another cell with a harbour alone: crowded. The harbour goes: not
    // evaluated, still set (5 x 0 < 0 is false). A forester comes: not
    // evaluated either.
    let lone = (88, 24);
    add(&mut snapshot, HARBOR, lone);
    run(&mut world, &mut ai, &snapshot);
    let set = marker(&mut ai, lone);
    assert!(set.is_some());
    remove(&mut snapshot, lone);
    run(&mut world, &mut ai, &snapshot);
    assert_eq!(marker(&mut ai, lone), set, "a harbour gone");
    add(&mut snapshot, FORESTER, (90, 26));
    run(&mut world, &mut ai, &snapshot);
    assert_eq!(marker(&mut ai, lone), set, "a forester added");

    // 7.6: both markers equal; the cell made first wins.
    let (a, b) = (cell_of(harbour), cell_of(lone));
    let p = ai.player_mut(P0);
    let order = p.cells.in_order().to_vec();
    let rank = |c| order.iter().position(|&o| o == c).unwrap();
    let first = if rank(a) < rank(b) { a } else { b };
    assert_ne!(
        first,
        std::cmp::min_by_key(a, b, |c| (c.1, c.0)),
        "the lowest cell is not the first made"
    );
    for pos in order.clone() {
        p.cells.get_mut(pos).unwrap().crowded = None;
    }
    p.cells.get_mut(a).unwrap().crowded = Some((1, 1));
    p.cells.get_mut(b).unwrap().crowded = Some((1, 1));
    p.expansion.pending[crate::expansion::Source::Crowded as usize] = None;
    ai.with_ctx(&world, P0, |p, ctx| p.crowded_cells(ctx));
    let target = ai.player_mut(P0).expansion.pending[crate::expansion::Source::Crowded as usize]
        .expect("a crowded-cell target");
    assert_eq!(target.at, crate::cells::origin(first));
}

// AI-CON-1.1a (corrected 2026-10-27 night, AI-CON T49): only a building
// added to one of the cell's four lists moves its position: a hunter's hut
// or a forester at the cell's centre leaves the headquarters' field as the
// home cell's position; a stone pit there takes it.
// spec: AI-CON-1.1a (AI-CON T49), AI-CON-1.1
#[test]
fn t49_a_hunter_or_forester_does_not_move_the_cells_position() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let home = ai.player_mut(P0).home;
    let cell = cell_of(home);
    let centre = crate::cells::centre(cell);
    assert_ne!(home, centre);
    let position = |ai: &mut AiSystem| ai.player_mut(P0).cells.get(cell).unwrap().position;
    assert_eq!(position(&mut ai), Some(home));
    let mut snapshot = crate::snapshot::Snapshot::take(&world);
    let template = *snapshot.buildings[0].last().unwrap();
    for (serial, kind) in [(900, HUNTER), (901, FORESTER)] {
        let mut b = template;
        b.building_type = kind;
        b.at = centre;
        b.serial = serial;
        snapshot.buildings[0].push(b);
        ai.update_with(&world, P0, &snapshot);
        assert_eq!(position(&mut ai), Some(home), "{kind}");
        snapshot.buildings[0].pop();
    }
    let mut pit = template;
    pit.building_type = STONE_PIT;
    pit.at = centre;
    pit.serial = 902;
    snapshot.buildings[0].push(pit);
    ai.update_with(&world, P0, &snapshot);
    assert_eq!(position(&mut ai), Some(centre));
}

// AI-CON-1.6: a depot ends the wish for one; losing it brings the wish
// back. The headquarters' cell has no wish from the start: the headquarters
// is a depot (AI-CON-1.1, corrected 2026-10-24).
// spec: AI-CON-1.6, AI-CON-1.1
#[test]
fn a_depot_clears_and_its_loss_restores_the_depot_marker() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let home = cell_of(ai.player_mut(P0).home);
    let wanted = |ai: &mut AiSystem, cell| {
        ai.player_mut(P0)
            .cells
            .get(cell)
            .unwrap()
            .depot_wanted
            .is_some()
    };
    assert!(!wanted(&mut ai, home));
    let at = (14, 22);
    let cell = cell_of(at);
    assert_ne!(cell, home);
    with_building(&mut world, DEPOT, at);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    assert!(!wanted(&mut ai, cell));
    world.submit(Command::RemoveBuilding { player: P0, at });
    world.step();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    ai.update_with(&world, P0, &snapshot);
    assert!(wanted(&mut ai, cell));
}

// T20 (AI-CON-1.7, corrected 2026-10-27 night, AI-MIL T18): stock(g) counts
// the stores of the player's warehouses, not the materials at a site;
// stock2(g) the wares outside buildings. The cycle counter starts
// at 0 and is advanced before the phase test (Q-332): every count reads 0
// until the player's 22nd update, which sums stock; stock2 is counted in the
// 24th; then both every 22 updates, lagging the game in between.
// spec: AI-CON-1.7 (T20, AI-MIL T18)
#[test]
fn t20_stock_counts_lag_on_a_cycle() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    with_building(&mut world, STONE_PIT, (24, 22));
    world.submit(Command::DebugSupplySite {
        player: P0,
        at: (24, 22),
        wares: vec![(PLANK, 1)],
        workers: false,
    });
    world.step();
    let flag = crate::view::flag_of(&world, (24, 22)).unwrap();
    let hq_flag = crate::view::flag_of(&world, ai.player_mut(P0).home).unwrap();
    let drop = |world: &mut World| {
        world.submit(Command::DropWare {
            player: P0,
            good: WOOD,
            at: flag,
            destination: hq_flag,
        });
        world.step();
        assert!(
            world.last_results().iter().all(|r| r.outcome == Ok(())),
            "{:?}",
            world.last_results()
        );
    };
    // Two wares on the roads.
    drop(&mut world);
    drop(&mut world);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    // Activation leaves every count at 0.
    let econ = |ai: &mut AiSystem| {
        let e = &ai.player_mut(P0).econ;
        (e.stock(PLANK), e.stock2(WOOD))
    };
    assert_eq!(econ(&mut ai), (0, 0));
    for update in 1..=46u32 {
        if update == 30 {
            // A third ware: counted at the next cycle tick 2 only.
            drop(&mut world);
        }
        ai.player_mut(P0).refresh_counts(&world, &snapshot);
        // T92: the counts are taken at the end of the think before.
        ai.player_mut(P0).prepare_counts(&world, &snapshot);
        // 20 planks at the headquarters (start stock); the one at the site
        // is not counted.
        let stock = if update >= 22 { 20 } else { 0 };
        let stock2 = match update {
            ..=23 => 0,
            24..=45 => 2,
            _ => 3,
        };
        assert_eq!(econ(&mut ai), (stock, stock2), "update {update}");
    }
}

// T26 (AI-CON-2.5): the goods a new building needs: the inputs of its
// product (not for a hunter), food for the granite mine, the planting good
// of a planting building with no product; nothing for the headquarters.
// spec: AI-CON-2.5 (T26)
#[test]
fn t26_the_goods_a_new_building_needs() {
    const FOOD: u32 = 28;
    const STONE_MINE: u32 = 120;
    let mut config = config();
    config.names.food = FOOD;
    let world = world(&[0]);
    let needs = |kind: u32| {
        let t = world.building_type(kind).expect("a fixture type");
        crate::player::AiPlayer::new_building_needs(&config, t)
    };
    assert_eq!(needs(SAWMILL), [WOOD]);
    assert_eq!(needs(SMITHY), [IRON, COAL]);
    assert_eq!(needs(FORESTER), [WOOD]);
    for kind in [CASTLE, WOODCUTTER, STONE_PIT, HUNTER, GUARD_HOUSE] {
        assert!(needs(kind).is_empty(), "type {kind}");
    }
    let mut mine = s2_sim::BuildingType::new(STONE_MINE, s2_sim::BuildingSize::Mine);
    mine.product = Some(STONE);
    assert_eq!(
        crate::player::AiPlayer::new_building_needs(&config, &mine),
        [FOOD]
    );
}

// Hook H6: a building type with a project's recipe (`Rules::recipes`) needs
// the goods of its recipe, not its product's inputs, and a site of it is
// rated by its recipe's suppliers; every other type of the same product
// keeps its product's inputs.
// spec: AI-CON-2.5 (hook H6: a project's recipe per building type)
#[test]
fn a_type_with_a_projects_recipe_needs_its_recipes_goods() {
    const BURNER: u32 = 150;
    let mut config = config();
    let mut burner = s2_sim::BuildingType::new(BURNER, s2_sim::BuildingSize::Small);
    burner.worker = Some(HELPER);
    burner.product = Some(IRON);
    burner.cycle = s2_sim::WorkCycle::Workshop;
    let before = crate::player::AiPlayer::new_building_needs(&config, &burner);
    assert_eq!(
        before,
        config
            .inputs(IRON)
            .iter()
            .map(|&(g, _)| g)
            .collect::<Vec<_>>()
    );
    config.sim.recipes.insert(BURNER, vec![(WOOD, 2)]);
    assert_eq!(
        crate::player::AiPlayer::new_building_needs(&config, &burner),
        [WOOD]
    );
    assert_eq!(config.type_inputs(BURNER, IRON), [(WOOD, 2)]);
    assert_eq!(config.type_inputs(SMITHY, IRON), config.inputs(IRON));
    let world = world(&[0]);
    let smithy = world.building_type(SMITHY).expect("a fixture type");
    assert_eq!(
        crate::player::AiPlayer::new_building_needs(&config, smithy),
        [IRON, COAL]
    );
}

// T26 (AI-CON-2.5): with no wood need, a sawmill site gets a new wood need
// tied to it, without a cell and without a stock test (the castle holds
// more than 10 wood); a second sawmill site gets another (the first has a
// consumer); a wood need without a consumer (AI-CON-3.2) is tied to the
// next one instead of a new need. A woodcutter, a stone pit, a hunter and
// the headquarters make none.
// spec: AI-CON-2.5 (T26)
#[test]
fn t26_a_new_building_gets_the_needs_of_its_inputs() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    ai.player_mut(P0).econ = Economy::from_counts(&[(WOOD, 50)], &[]);
    let announce = |world: &World, ai: &mut AiSystem| {
        let snapshot = crate::snapshot::Snapshot::take(world);
        ai.update_with(world, P0, &snapshot);
    };
    for (kind, at) in [
        (WOODCUTTER, (26, 22)),
        (STONE_PIT, (24, 26)),
        (HUNTER, (16, 26)),
    ] {
        with_building(&mut world, kind, at);
    }
    announce(&world, &mut ai);
    assert!(ai.player_mut(P0).plan.needs.is_empty());
    let wood_needs = |ai: &mut AiSystem| -> Vec<(Option<s2_sim::grid::Point>, bool)> {
        ai.player_mut(P0)
            .plan
            .needs
            .iter()
            .filter(|n| n.good == WOOD)
            .map(|n| (n.consumer, n.cell.is_some()))
            .collect()
    };
    let (a, b, c) = ((24, 22), (16, 22), (20, 26));
    with_building(&mut world, SAWMILL, a);
    announce(&world, &mut ai);
    assert_eq!(wood_needs(&mut ai), [(Some(a), false)]);
    with_building(&mut world, SAWMILL, b);
    announce(&world, &mut ai);
    assert_eq!(wood_needs(&mut ai), [(Some(a), false), (Some(b), false)]);
    ai.player_mut(P0).plan.add_need(WOOD, None, 0.0);
    with_building(&mut world, SAWMILL, c);
    announce(&world, &mut ai);
    assert_eq!(
        wood_needs(&mut ai),
        [(Some(a), false), (Some(b), false), (Some(c), false)]
    );
}

// AI-CON-6.1, T103: the first harbour-pavement field in the rings around p
// needs an own harbour (any state: a site counts) within 3 fields (4 for a
// large type), else v = 0; any other own building near the pavement does
// not count (R6: player 3's guard house on (21, 30) beside the pavement
// (18, 30) leaves (18, 30) and (19, 30) at 0, so its woodcutter of counter
// 9772 goes on (28, 24) as recorded).
// spec: AI-CON-6.1 (T103)
#[test]
fn the_pavement_rule_needs_a_building_nearby() {
    use s2_sim::map::state;
    let mut terrain = flat(96, 96);
    let index = (22 * 96 + 26) as usize;
    terrain.states[index] |= state::FOR_HARBOR;
    let setup = s2_sim::GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        ..s2_sim::GameSetup::default()
    };
    let world = World::new_game(1, terrain, rules(), &setup);
    let config = config();
    let sim = world.building_type(STONE_PIT).unwrap().clone();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let env = crate::sites::SiteEnv {
        world: &world,
        config: &config,
        player: P0,
        own: snapshot.of(P0),
    };
    // (25, 22) is next to the pavement at (26, 22); the castle is 6 away.
    assert_eq!(env.value(&sim, (25, 22)), 0.0);
    // Nowhere near the pavement the rule does not apply (value from the
    // trees count is 0 here, so compare with a type without product).
    let forester = world.building_type(FORESTER).unwrap().clone();
    assert!(env.value(&forester, (21, 24)) > 0.0);
    // Another own building within 3 of the pavement does not count.
    let mut near = *snapshot.of(P0).first().unwrap();
    near.at = (27, 23);
    let own = vec![near];
    let env = crate::sites::SiteEnv {
        world: &world,
        config: &config,
        player: P0,
        own: &own,
    };
    assert_eq!(env.value(&forester, (25, 22)), 0.0, "not a harbour");
    // An own harbour there does, its site as well.
    let mut harbour = near;
    harbour.building_type = HARBOR;
    harbour.under_construction = true;
    let own = vec![near, harbour];
    let env = crate::sites::SiteEnv {
        world: &world,
        config: &config,
        player: P0,
        own: &own,
    };
    assert!(env.value(&forester, (25, 22)) > 0.0);
}

// AI-CON T105 (AI-CON-6.3, the depot test counts sites): a depot's site
// value is 0 with an own warehouse within 10 fields in any state: a
// storehouse's or harbour's building site counts as soon as it is placed
// (R6: player 1's harbour site on (62, 49) zeroes (55, 45), 9 away, and the
// storehouse goes on (54, 44), 11 away, as recorded).
// spec: AI-CON-6.3 (AI-CON T105)
#[test]
fn t105_the_depot_test_counts_a_warehouse_site() {
    let world = world(&[0]);
    let config = config();
    let depot = world.building_type(DEPOT).unwrap().clone();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let castle = *snapshot.of(P0).first().unwrap();
    let value_with = |own: &[s2_sim::Building], at| {
        let env = crate::sites::SiteEnv {
            world: &world,
            config: &config,
            player: P0,
            own,
        };
        env.value(&depot, at)
    };
    let at = (24, 22);
    assert!(value_with(&[], at) > 0.0, "no warehouse near");
    let site = s2_sim::Building {
        building_type: HARBOR,
        at: (33, 22),
        under_construction: true,
        ..castle
    };
    assert_eq!(crate::view::dist(site.at, at), 9);
    assert_eq!(value_with(&[site], at), 0.0, "a harbour site 9 away");
    let far = s2_sim::Building {
        at: (35, 22),
        ..site
    };
    assert!(value_with(&[far], at) > 0.0, "11 away");
}

// AI-CON-6.4, T28: a non-military type's value takes x 0.4 for a Border
// field on the ring at distance exactly 2 around p; Border fields only at
// distance 1, only at 3 to 6, or on p itself with none at distance 2 give no
// factor. Far from the castle (outside the territory, no field class
// factor) the forester's value is 1.
// spec: AI-CON-6.4 (T28)
#[test]
fn t28_the_border_factor_looks_at_the_ring_at_distance_two() {
    use s2_sim::map::state;
    let p = (60, 60);
    let value_with_border = |rings: &[u32]| -> f32 {
        let mut terrain = flat(96, 96);
        for &k in rings {
            for f in s2_sim::grid::ring(p, k, 96, 96) {
                let index = (f.1 * 96 + f.0) as usize;
                terrain.states[index] |= state::BORDER;
            }
        }
        let setup = s2_sim::GameSetup {
            players: vec![(P0, (20, 20))],
            local_player: P0,
            ..s2_sim::GameSetup::default()
        };
        let world = World::new_game(1, terrain, rules(), &setup);
        let config = config();
        let forester = world.building_type(FORESTER).unwrap().clone();
        let snapshot = crate::snapshot::Snapshot::take(&world);
        let env = crate::sites::SiteEnv {
            world: &world,
            config: &config,
            player: P0,
            own: snapshot.of(P0),
        };
        env.value(&forester, p)
    };
    // AI-CON-6.2 (94d1a13): a woodcutter type exists and none is built, so
    // the forester's base value is 0.1 (no warehouse fallback).
    let plain = value_with_border(&[]);
    assert_eq!(plain, 0.1);
    assert_eq!(value_with_border(&[2]), plain * 0.4);
    assert_eq!(value_with_border(&[1]), plain);
    assert_eq!(value_with_border(&[3, 4, 5, 6]), plain);
    assert_eq!(value_with_border(&[0]), plain);
    assert_eq!(value_with_border(&[0, 1, 2, 3]), plain * 0.4);
}

// AI-CON-8.1 strategy 3: the road-stop search notes the first field of a road
// attached to T that it evaluates, and P is that road's other end; a road
// that does not end at T blocks. From (27, 21) T is labelled before any
// field of R is evaluated: no hit, no read-out, no stop (Q-245).
// spec: AI-CON-8.1, AI-2.7 (Q-239)
#[test]
fn the_road_stop_search_finds_a_road_attached_to_the_target() {
    let mut world = world(&[0]);
    let (t, p, f) = ((24, 18), (24, 24), (26, 23));
    let away = (27, 20);
    for at in [t, p, f, away] {
        world.submit(Command::PlaceFlag { player: P0, at });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()), "{at:?}");
    }
    world.submit(Command::BuildRoad {
        player: P0,
        from: t,
        to: p,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    let road = world.roads().next().unwrap().path.clone();
    let mut rng = s2_sim::Rng::new(1);
    let stop =
        crate::roads::road_stop_search(&world, &mut rng, P0, f, t, &[]).expect("a path through R");
    assert!(
        road[1..road.len() - 1].contains(&stop.hit),
        "{:?}",
        stop.hit
    );
    assert_eq!(stop.p, p);
    assert_eq!(stop.path.last(), Some(&t));
    // AI-2.7 b: the path is read out with the free walk's draws, at least
    // the first direction's (F and T are not neighbours).
    assert!(rng.draws() >= 1);
    assert!(crate::roads::road_stop_search(&world, &mut rng, P0, away, t, &[]).is_none());
    // An excluded field next to F on every side: no path.
    let around: Vec<_> = world
        .terrain()
        .neighbours(f)
        .into_iter()
        .flatten()
        .collect();
    assert!(crate::roads::road_stop_search(&world, &mut rng, P0, f, t, &around).is_none());
    // Seen from P the same road is attached to P, not to a third flag.
    let other = (17, 24);
    world.submit(Command::PlaceFlag {
        player: P0,
        at: other,
    });
    world.step();
    assert!(crate::roads::road_stop_search(&world, &mut rng, P0, f, other, &[]).is_none());
}

// AI-CON-8.1e: a walk path never enters a road field or a flag but its
// goal: across the road T - P it has to go round the road's ends, longer
// than 2 x distance, so it does not count; given R (a second leg) it walks
// straight over it. Neighbours are one step, whatever stands there; the goal
// itself may be a flag.
// spec: AI-CON-8.1e, AI-CON-8.1
#[test]
fn a_walk_path_crosses_a_road_only_when_it_may_walk_over_it() {
    let mut world = world(&[0]);
    let (t, p) = ((24, 18), (24, 24));
    for at in [t, p] {
        world.submit(Command::PlaceFlag { player: P0, at });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()), "{at:?}");
    }
    world.submit(Command::BuildRoad {
        player: P0,
        from: t,
        to: p,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    let road = world.roads().next().unwrap().path.clone();
    // Two fields either side of the road's field on row 21.
    let x = road.iter().find(|f| f.1 == 21).unwrap().0;
    let (a, b) = ((x - 1, 21), (x + 1, 21));
    let round = crate::roads::walk_path(&world, P0, a, b, &[], &[]).expect("round the road");
    assert!(round > 4, "{round}");
    assert_eq!(
        crate::roads::walk_path(&world, P0, a, b, &[], &road),
        Some(2)
    );
    // An excluded field blocks, but not when it is the goal.
    let around: Vec<_> = world
        .terrain()
        .neighbours(b)
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(
        crate::roads::walk_path(&world, P0, a, b, &around, &road),
        None
    );
    let next = around[0];
    assert_eq!(
        crate::roads::walk_path(&world, P0, next, b, &around, &road),
        Some(1)
    );
    // From a flag, and to a road field next to it.
    assert!(crate::roads::walk_path(&world, P0, t, (22, 15), &[], &[]).is_some());
    assert_eq!(
        crate::roads::walk_path(&world, P0, t, road[1], &[], &[]),
        Some(1)
    );
}

// SIM-START-4.4 b (Q-263 a), T12: a building whose type has a product makes
// its production record when the computer player learns of it, except the
// hunter's hut: the woodcutter's hut one record, the hunter's hut none
// (neither has an input, so no need record follows).
// spec: SIM-START-4.4 (T12)
#[test]
fn a_producer_but_the_hunter_makes_a_production_record() {
    let mut world = world(&[0]);
    let mut ai = start(&world, 2);
    let before = ai.player_mut(P0).plan.made();
    with_building(&mut world, WOODCUTTER, (26, 22));
    ai.update_with(&world, P0, &crate::snapshot::Snapshot::take(&world));
    assert_eq!(ai.player_mut(P0).plan.made(), before + 1);
    with_building(&mut world, HUNTER, (24, 24));
    ai.update_with(&world, P0, &crate::snapshot::Snapshot::take(&world));
    assert_eq!(ai.player_mut(P0).plan.made(), before + 1);
}

// AI-CON T29 (AI-CON-8.4, Q-269): the flag search tests each field's
// neighbours in the ring order 5, 0, 1, 2, 3, 4 (MAP-GRID-3.2): of two flags
// two steps from F, one through F's direction 0 neighbour and one through
// its direction 5 neighbour, the second is found first (the castle's flag of
// T29, reached through (139, 49)).
// spec: AI-CON-8.4 (T29), MAP-GRID-3.2
#[test]
fn t29_the_flag_search_goes_round_from_direction_5() {
    let mut world = world(&[0]);
    let f = (24, 22);
    let east = (26, 22);
    let through_5 = (25, 24);
    for at in [f, east, through_5] {
        world.submit(Command::PlaceFlag { player: P0, at });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()), "{at:?}");
    }
    let network = crate::view::Network::compute(&world, P0);
    let found = crate::roads::flag_search(&world, P0, f, &[], Some((&network, f)));
    assert_eq!(found, Some(through_5));
    // With it excluded, the other.
    let found = crate::roads::flag_search(&world, P0, f, &[through_5], Some((&network, f)));
    assert_eq!(found, Some(east));
}

/// `layer` with `p` and the fields of rows 33 and beyond marked -1.
fn split_minus(layer: &[i32], p: (u32, u32)) -> Vec<i32> {
    let mut layer = layer.to_vec();
    layer[33 * 96..].fill(-1);
    layer[(p.1 * 96 + p.0) as usize] = -1;
    layer
}

/// A one-player world on the flat 96 x 96 map, its castle on (20, 20), with
/// `continents` as the map's stored continent layer.
fn world_with_continents(continents: Vec<i32>) -> World {
    let setup = s2_sim::GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        continents,
        ..s2_sim::GameSetup::default()
    };
    World::new_game(1, flat(96, 96), rules(), &setup)
}

// AI-CON-6.3: a military type's value counts the fields within its
// territory range that the player does not own and whose stored continent
// index (FMT-S2M-4.9) equals the site's (-1 equals -1); with no field
// counting the value is 0. The guard house (range 9) on (20, 26), 6
// from the castle, on a large field (x 0.5 for a small type).
// spec: AI-CON-6.3, FMT-S2M-4.9
#[test]
fn the_military_value_counts_the_fields_of_the_sites_stored_continent() {
    use s2_sim::Placement;
    let p = (20, 26);
    let value = |continents: Vec<i32>| -> (f32, usize) {
        let world = world_with_continents(continents.clone());
        assert_eq!(world.compute_placement(P0, p), Placement::Large);
        let config = config();
        let guard = world.building_type(GUARD_HOUSE).unwrap().clone();
        let snapshot = crate::snapshot::Snapshot::take(&world);
        let env = crate::sites::SiteEnv {
            world: &world,
            config: &config,
            player: P0,
            own: snapshot.of(P0),
        };
        let index = |f: (u32, u32)| (f.1 * 96 + f.0) as usize;
        let free = s2_sim::grid::area(p, 9, 96, 96)
            .into_iter()
            .filter(|&f| {
                world.field_owner(f) != Some(P0) && continents[index(f)] == continents[index(p)]
            })
            .count();
        (env.value(&guard, p), free)
    };
    let one = vec![1; 96 * 96];
    let (v, free) = value(one.clone());
    assert!(free > 0);
    assert_eq!(v, (free as f32 + 6.0) * 0.5);
    // The fields west of x = 18 on another continent, those of row 33 and
    // beyond marked -1: neither counts.
    let mut split = one.clone();
    for y in 0..96u32 {
        for x in 0..96u32 {
            let i = (y * 96 + x) as usize;
            if x < 18 {
                split[i] = 2;
            } else if y >= 33 {
                split[i] = -1;
            }
        }
    }
    let (v_split, free_split) = value(split);
    assert!(0 < free_split && free_split < free);
    assert_eq!(v_split, (free_split as f32 + 6.0) * 0.5);
    // Every field outside the territory -1: no field counts, value 0 (not
    // the 6 of the distance).
    let probe = world_with_continents(one.clone());
    let none: Vec<i32> = probe
        .terrain()
        .fields()
        .map(|f| {
            if probe.field_owner(f) == Some(P0) {
                1
            } else {
                -1
            }
        })
        .collect();
    assert_eq!(value(none), (0.0, 0));
    // The site's own field -1: the fields of index 1 do not count, the
    // other -1 fields do.
    let mut own_none = one.clone();
    own_none[(26 * 96 + 20) as usize] = -1;
    assert_eq!(value(own_none).0, 0.0);
    let (v_minus, free_minus) = value(split_minus(&one, p));
    assert!(free_minus > 0);
    assert_eq!(v_minus, (free_minus as f32 + 6.0) * 0.5);
}

// AI-CON-5.4a, the military clause: a building or site of any player whose
// type has a territory range within distance 4 of the field keeps a military
// site off it; buildings without a range do not.
// spec: AI-CON-5.4a, AI-CON-5.3
#[test]
fn the_military_clause_keeps_four_fields_around_buildings_with_a_range() {
    let mut world = world(&[0]);
    // The castle (range 9) on (20, 20).
    assert!(crate::sites::military_near(&world, (20, 24)));
    assert!(!crate::sites::military_near(&world, (20, 25)));
    // A guard house site (range 9) counts while under construction; a
    // woodcutter's site does not.
    with_building(&mut world, GUARD_HOUSE, (26, 20));
    with_building(&mut world, WOODCUTTER, (14, 20));
    assert!(world.site((26, 20)).is_some());
    assert!(crate::sites::military_near(&world, (30, 20)));
    assert!(!crate::sites::military_near(&world, (31, 20)));
    assert!(!crate::sites::military_near(&world, (11, 20)));
}

// AI-CON-5.4a: a guard house search over the castle's cell never ends a
// step on a field within 4 of the castle (they fail the size test
// cheaply), while a stone pit search does.
// spec: AI-CON-5.4a, AI-CON-5.4
#[test]
fn a_military_search_passes_over_the_fields_near_the_castle_cheaply() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let home = cell_of(ai.player_mut(P0).home);
    ai.with_ctx(&world, P0, |p, ctx| {
        let env = crate::sites::SiteEnv {
            world: ctx.world,
            config: ctx.config,
            player: p.id,
            own: ctx.snapshot.of(p.id),
        };
        let fields = Area::Cell(home).fields(ctx.world);
        let ends = |building: u32| -> Vec<(u32, u32)> {
            let mut scan = crate::sites::Scan::new(ctx.world, Area::Cell(home), building, None);
            let mut ends = Vec::new();
            let mut before = 0;
            while !scan.done() {
                scan.step(&env);
                // A step that tested fewer than 10 fields ended early on its
                // last one.
                if scan.tested() - before < crate::sites::FIELDS_PER_TICK && !scan.done() {
                    ends.push(fields[scan.tested() - 1]);
                }
                before = scan.tested();
            }
            ends
        };
        let near = |f: &(u32, u32)| crate::view::dist(*f, (20, 20)) <= 4;
        let guard = ends(GUARD_HOUSE);
        assert!(!guard.is_empty());
        assert!(!guard.iter().any(near), "{guard:?}");
        assert!(ends(STONE_PIT).iter().any(near));
    });
}

// AI-CON-5.4a, the flag test: one copy of p stepped through the directions
// 0, 1, 2, 3, 4 (on (30, 30): (31, 30), (31, 29), (31, 28), (30, 28),
// (29, 29)); one of them in the map, owned and without a blocking state
// passes it. The direction-1 neighbour (30, 29), the building's own flag
// field, is not among them.
// spec: AI-CON-5.4a
#[test]
fn the_flag_test_walks_one_copy_through_directions_0_to_4() {
    use s2_sim::map::state;
    let p = (30, 30);
    let walk = [(31, 30), (31, 29), (31, 28), (30, 28), (29, 29)];
    let with_open = |open: &[(u32, u32)], bits: u32| -> bool {
        let mut terrain = flat(96, 96);
        for f in terrain.fields().collect::<Vec<_>>() {
            if f != p && !open.contains(&f) {
                let i = terrain.index(f);
                terrain.states[i] |= bits;
            }
        }
        let world = World::with_map(1, terrain, rules(), P0);
        crate::sites::flag_room(&world, P0, p)
    };
    for f in walk {
        assert!(with_open(&[f], state::BLOCKED), "{f:?}");
    }
    assert!(!with_open(&[(30, 29)], state::BLOCKED));
    assert!(!with_open(&[], state::BLOCKED));
    for bits in [
        state::BORDER,
        state::WATER,
        state::ELEVATION_BLOCKED,
        state::DOODAD_BLOCKED,
        state::DEPOSIT_BLOCKED,
        state::MOVEMENT_BLOCKED,
    ] {
        assert!(!with_open(&[], bits), "{bits:#x}");
    }
    // Not owned by the player: the test fails.
    let world = World::with_map(1, flat(96, 96), rules(), PlayerId(1));
    assert!(!crate::sites::flag_room(&world, P0, p));
    // A field at the map's corner: the steps leaving the map do not pass,
    // (1, 0) and the others inside do.
    let world = World::with_map(1, flat(96, 96), rules(), P0);
    assert!(crate::sites::flag_room(&world, P0, (0, 0)));
}

// AI-CON-1.1b (b): a cell's landmass is the stored continent index at the
// first position it gets; a later, nearer position does not change it,
// unless it is still -1.
// spec: AI-CON-1.1b, AI-CON-1.1a
#[test]
fn a_cells_landmass_is_the_index_at_its_first_position() {
    // x < 24: index 1; x >= 24: index 2; (17, 17) is -1.
    let mut layer: Vec<i32> = (0..96 * 96)
        .map(|i| if i % 96 < 24 { 1 } else { 2 })
        .collect();
    layer[17 * 96 + 17] = -1;
    let world = world_with_continents(layer);
    let mut cells = crate::cells::Cells::new(96, 96);
    // Cell (1, 1): fields 16 .. 31, centre (24, 24). First (30, 30) on 2,
    // then the nearer (23, 24) on 1: the landmass stays 2.
    let cell = cells.ensure((1, 1), 0);
    assert_eq!(cell.landmass, -1);
    cell.building_added(&world, (30, 30), 0, false);
    assert_eq!((cell.position, cell.landmass), (Some((30, 30)), 2));
    cell.building_added(&world, (23, 24), 0, false);
    assert_eq!((cell.position, cell.landmass), (Some((23, 24)), 2));
    // First (17, 17) on -1, then the nearer (23, 24) on 1: it follows.
    cells.remove((1, 1));
    let cell = cells.ensure((1, 1), 0);
    cell.building_added(&world, (17, 17), 0, false);
    assert_eq!((cell.position, cell.landmass), (Some((17, 17)), -1));
    cell.building_added(&world, (23, 24), 0, false);
    assert_eq!((cell.position, cell.landmass), (Some((23, 24)), 1));
}

// AI-CON-7.3: the stones of the cell count only on the landmass of the
// cell's current position (its stored continent index, AI-CON-1.1b): the
// 12 rocks of T8 in the headquarters' cell, 8 of them on another index:
// 4 left, 5 x 1 < 4 is false, no stone pit order.
// spec: AI-CON-7.3, AI-CON-1.1b
#[test]
fn the_cells_stones_count_on_the_landmass_of_its_position() {
    let mut world = forest_world(0, 12);
    // The rocks lie on (31, 31) down to (20, 31); (24 .. 31, 31) on 2.
    let layer: Vec<i32> = (0..96 * 96u32)
        .map(|i| if i / 96 == 31 && i % 96 >= 24 { 2 } else { 1 })
        .collect();
    world.set_continents(layer);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    ai.player_mut(P0).plan.add_production(STONE, cell, None);
    let p = ai.player_mut(P0);
    let total = p.cells.cols * p.cells.rows;
    p.pit_cursor = (cell.1 * p.cells.cols + cell.0 + total - 1) % total;
    ai.with_ctx(&world, P0, |p, ctx| p.pit_and_woodcutter(ctx));
    assert!(ai.player_mut(P0).build.slot(Slot::Cell).is_none());
}

// AI-CON-1.1a, 1.1b: the validity scan makes a cell only when a field of
// its square's border is the player's and on the landmass of a neighbouring
// cell's position; then the cell gets that field as its position and its
// index as its landmass. Cell (0, 1) beside the headquarters' cell (1, 1)
// (position (20, 20), index 1): with its square on index 2 no cell is made.
// spec: AI-CON-1.1a, AI-CON-1.1b
#[test]
fn the_validity_scan_makes_no_cell_without_a_border_field() {
    for (index, made) in [(2, false), (1, true)] {
        let layer: Vec<i32> = (0..96 * 96u32)
            .map(|i| {
                if i % 96 < 16 && (16..32).contains(&(i / 96)) {
                    index
                } else {
                    1
                }
            })
            .collect();
        let world = world_with_continents(layer);
        assert!(crate::cells::corner_owned(&world, P0, (0, 1)));
        let mut cells = crate::cells::Cells::new(96, 96);
        cells
            .ensure((1, 1), 0)
            .building_added(&world, (20, 20), 0, false);
        // The cursor goes (1, 0), (2, 0), ..., (5, 0), (0, 1).
        for _ in 0..6 {
            cells.scan(&world, P0, 0);
        }
        let cell = cells.get((0, 1));
        assert_eq!(cell.is_some(), made, "index {index}");
        if let Some(cell) = cell {
            let at = cell.position.expect("a position");
            assert_eq!(world.field_owner(at), Some(P0));
            assert_eq!(cell.landmass, 1);
        }
    }
}

/// A one-player world on the flat 96 x 96 map with `land_spots` as the
/// map's land continent objects; every field index 1.
fn world_with_spots(land_spots: Vec<(i32, Vec<(u32, u32)>)>) -> World {
    let setup = s2_sim::GameSetup {
        players: vec![(P0, (20, 20))],
        local_player: P0,
        continents: vec![1; 96 * 96],
        land_spots,
        ..s2_sim::GameSetup::default()
    };
    World::new_game(1, flat(96, 96), rules(), &setup)
}

// AI-CON-1.6 harbour wanted: when a cell's landmass is set, a position of
// that land continent object inside the cell sets the marker (in that
// tick), unless the cell has a harbour; spots of another continent in the
// cell do not, nor do the landmass's spots in other cells.
// spec: AI-CON-1.6, AI-CON-1.1b
#[test]
fn the_harbour_marker_comes_with_the_cells_landmass() {
    let marker = |spots: Vec<(i32, Vec<(u32, u32)>)>, has_harbour: bool| {
        let world = world_with_spots(spots);
        let mut cells = crate::cells::Cells::new(96, 96);
        let cell = cells.ensure((1, 1), 0);
        cell.building_added(&world, (20, 20), 7, has_harbour);
        cell.harbour_wanted
    };
    assert_eq!(marker(vec![(1, vec![(26, 22)])], false), Some(7));
    assert_eq!(marker(vec![(1, vec![(26, 22)])], true), None);
    assert_eq!(marker(vec![(2, vec![(26, 22)])], false), None);
    assert_eq!(marker(vec![(1, vec![(40, 22)])], false), None);
    // A later position does not set it again: the landmass is set once.
    let world = world_with_spots(vec![(1, vec![(26, 22)])]);
    let mut cells = crate::cells::Cells::new(96, 96);
    let cell = cells.ensure((1, 1), 0);
    cell.building_added(&world, (20, 20), 7, true);
    cell.building_added(&world, (24, 24), 9, false);
    assert_eq!(cell.harbour_wanted, None);
}

// AI-CON-3.5: each resource entry is scored on its own: two entries of 2
// trees on two conquered continents in one square do not count (each
// needs more than 2), one of 3 does; an entry on a continent that is not
// conquered does not.
// spec: AI-CON-3.5, AI-CON-1.1b
#[test]
fn a_waiting_need_scores_each_entry_on_a_conquered_continent() {
    let target = |entries: Vec<(i32, u32)>, conquered: Vec<i32>| {
        let world = world(&[0]);
        let mut ai = start(&world, 2);
        ai.player_mut(P0).resources.set_entries(
            (8, 5),
            entries
                .into_iter()
                .map(|(landmass, amount)| crate::resources::Entry {
                    good: WOOD,
                    kind: crate::resources::SURFACE,
                    landmass,
                    amount,
                })
                .collect(),
        );
        let p = ai.player_mut(P0);
        p.plan.add_need(WOOD, None, 0.0);
        let id = p.plan.needs[0].id;
        p.plan.put_waiting(id);
        p.conquered = conquered;
        ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
        ai.player_mut(P0).expansion.pending[0].map(|t| t.at)
    };
    assert_eq!(target(vec![(1, 2), (2, 2)], vec![1, 2]), None);
    assert_eq!(target(vec![(1, 3)], vec![1]), Some((32, 20)));
    assert_eq!(target(vec![(1, 3)], vec![2]), None);
}

// AI-CON-3.5 (Q-365): the cell system runs only while its target slot is
// empty; the target's square is the first of the 5 x 5 squares from the
// winning cell's first (one row and column into the next cells), x outer,
// y inner, whose entries of the good sum above 0.
// spec: AI-CON-3.5
#[test]
fn the_waiting_need_target_waits_for_an_empty_slot_and_scans_x_outer() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let landmass = ai.player_mut(P0).home_landmass;
    let wood = |amount: u32| {
        vec![crate::resources::Entry {
            good: WOOD,
            kind: crate::resources::SURFACE,
            landmass,
            amount,
        }]
    };
    // Cell (2, 1): squares 8 .. 11 x 4 .. 7. Row order would take (9, 4).
    ai.player_mut(P0).resources.set_entries((9, 4), wood(3));
    ai.player_mut(P0).resources.set_entries((8, 6), wood(3));
    let p = ai.player_mut(P0);
    p.plan.add_need(WOOD, None, 0.0);
    let id = p.plan.needs[0].id;
    p.plan.put_waiting(id);
    p.conquered = vec![landmass];
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
    let target = ai.player_mut(P0).expansion.pending[0].expect("a target");
    assert_eq!(target.at, (32, 24));
    // A second waiting need while the target waits in the slot: no run.
    let p = ai.player_mut(P0);
    p.plan.add_need(WOOD, None, 0.0);
    let id = p.plan.needs[1].id;
    p.plan.put_waiting(id);
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
    assert!(ai.player_mut(P0).plan.needs[1].waiting);
    assert_eq!(ai.player_mut(P0).expansion.pending[0], Some(target));
}

// AI-CON-3.5 (Q-278): V reads a square of the scored coordinate itself,
// nearest to the square of the nearest cell's position, so it falls with
// the distance: a nearer coordinate with less wood (two squares of 3, one
// entry of 6 in the cell-coordinate map) beats a farther one with more (8).
// spec: AI-CON-3.5, AI-CON-1.1c
#[test]
fn the_waiting_need_target_prefers_the_nearer_coordinate() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let landmass = ai.player_mut(P0).home_landmass;
    let wood = |amount: u32| {
        vec![crate::resources::Entry {
            good: WOOD,
            kind: crate::resources::SURFACE,
            landmass,
            amount,
        }]
    };
    // The home cell (1, 1); coordinate (2, 1) next to it, (5, 1) farther.
    let p = ai.player_mut(P0);
    p.resources.set_entries((8, 4), wood(3));
    p.resources.set_entries((8, 5), wood(3));
    p.resources.set_entries((20, 4), wood(8));
    p.plan.add_need(WOOD, None, 0.0);
    let id = p.plan.needs[0].id;
    p.plan.put_waiting(id);
    p.conquered = vec![landmass];
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
    let target = ai.player_mut(P0).expansion.pending[0].expect("a target");
    assert_eq!(target.at, (32, 16));
}

// AI-CON-3.4 (corrected 2026-10-26 night, AI-MIL T17): putting a need on
// the waiting list is no change, so one pass can list several needs, in the
// order they come; the cell system then takes the first list entry.
// spec: AI-CON-3.4, AI-CON-3.5 (AI-MIL T17)
#[test]
fn needs_without_a_square_go_on_the_waiting_list_in_one_pass() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    p.econ = Economy::from_counts(&[(WOOD, 50)], &[]);
    let first = p.plan.add_need(COAL, Some((1, 1)), 0.0);
    let second = p.plan.add_need(WOOD, Some((1, 1)), 0.0);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    assert_eq!(ai.player_mut(P0).plan.waiting, [first, second]);
    ai.with_ctx(&world, P0, |p, ctx| p.expansion_for_waiting(ctx));
    assert_eq!(
        ai.player_mut(P0).plan.waiting,
        [second],
        "the first entry taken"
    );
}

// AI-CON-7.6, 3.3 (corrected 2026-10-26 night): a source sets a target only
// while its own slot is empty; another source's target, in progress or
// pending, does not stop it.
// spec: AI-CON-7.6, AI-CON-3.3, AI-MIL-3.1
#[test]
fn the_crowded_source_looks_only_at_its_own_slot() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let target = crate::expansion::Target {
        kind: crate::expansion::Kind::Position,
        at: (40, 40),
        since: 0,
    };
    let p = ai.player_mut(P0);
    let home = cell_of(p.home);
    p.cells.get_mut(home).unwrap().crowded = Some((0, 1));
    p.expansion.current = Some((crate::expansion::Source::WaitingNeed, target));
    p.expansion.pending[0] = Some(target);
    let mut world = world;
    for _ in 0..30 {
        world.step();
    }
    ai.with_ctx(&world, P0, |p, ctx| p.crowded_cells(ctx));
    let crowded = ai.player_mut(P0).expansion.pending[crate::expansion::Source::Crowded as usize];
    assert!(crowded.is_some(), "set beside the waiting-need target");
    let since = crowded.unwrap().since;
    for _ in 0..30 {
        world.step();
    }
    ai.with_ctx(&world, P0, |p, ctx| p.crowded_cells(ctx));
    let again = ai.player_mut(P0).expansion.pending[crate::expansion::Source::Crowded as usize];
    assert_eq!(again.map(|t| t.since), Some(since), "its own slot is full");
}

// AI-CON-1.1c (Q-365): the resource map counts every field of a square,
// explored or not; a field with a deposit gives its surface entry and no
// underground one; water is left out.
// spec: AI-CON-1.1c, AI-CON-1.1
#[test]
fn the_resource_map_counts_unexplored_fields() {
    let mut world = world(&[0]);
    let far = (88, 88);
    assert!(!world.is_explored(P0, far));
    let coal = s2_sim::FieldResource {
        good: COAL,
        amount: 5,
    };
    world.set_resource(far, Some(coal));
    let mut ai = start(&world, 2);
    let square = crate::resources::square_of(far);
    let p = ai.player_mut(P0);
    let config = config();
    p.resources.refresh_all(&world, P0, &config);
    assert_eq!(
        p.resources
            .amount(square, COAL, crate::resources::UNDERGROUND),
        5
    );
    let mut config = config;
    config.names.water = COAL;
    p.resources.refresh_all(&world, P0, &config);
    assert_eq!(
        p.resources
            .amount(square, COAL, crate::resources::UNDERGROUND),
        0
    );
}

// AI-CON T57 (AI-CON-1.1c): a deposit's surface entry counts 1 plus the
// next-deposit links of its type: a pile that becomes another pile and
// then a third counts 3, a tree 1.
// spec: AI-CON-1.1c (AI-CON T57)
#[test]
fn t57_a_pile_counts_its_next_deposit_links() {
    use crate::fixture::{rules, world_on, Extras, ROCK};
    let mut rules = rules();
    let (second, third) = (ROCK + 100, ROCK + 101);
    let mut kinds = rules.deposit_kinds.clone();
    let rock = kinds.iter().find(|k| k.id == ROCK).unwrap().clone();
    for (id, next) in [(ROCK, Some(second)), (second, Some(third)), (third, None)] {
        kinds.retain(|k| k.id != id);
        kinds.push(s2_sim::DepositKind {
            id,
            next,
            ..rock.clone()
        });
    }
    rules.deposit_kinds = kinds;
    let (pile, tree) = ((70, 70), (71, 71));
    let extras = Extras {
        trees: vec![tree],
        rocks: vec![pile],
        fish: Vec::new(),
    };
    let world = world_on(1, &[0], &extras, rules);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    p.resources.refresh_all(&world, P0, &config());
    let amount = |good| {
        let square = crate::resources::square_of(pile);
        p.resources.amount(square, good, crate::resources::SURFACE)
    };
    assert_eq!(
        crate::resources::square_of(tree),
        crate::resources::square_of(pile)
    );
    assert_eq!((amount(STONE), amount(WOOD)), (3, 1));
}

// AI-CON-2.5a, AI-CON-3.2 (Q-365): a hunter's hut makes no production; an
// input is supplied by a production of a good of its group, but only one
// with a placed building (a needs order's planned production does not
// count).
// spec: AI-CON-2.5a, AI-CON-3.2, AI-CON-1.4
#[test]
fn an_input_counts_placed_productions_of_its_group() {
    let mut rules = crate::fixture::rules();
    rules.goods.get_mut(&COAL).unwrap().costs = vec![(TOOLS, 1)];
    let mut world = world(&[0]);
    let mut ai = start_with(&world, crate::fixture::config_for(&rules), 2);
    with_building(&mut world, HUNTER, (26, 22));
    let snapshot = finished_snapshot(&world, (26, 22), 0);
    ai.update_with(&world, P0, &snapshot);
    let p = ai.player_mut(P0);
    assert!(!p.plan.prods.iter().any(|x| x.building == Some((26, 22))));
    // A coal need whose input is the group good tools; a saw (a tool) is
    // planned but not placed: tools are missing and get a need.
    let home = cell_of(p.home);
    let saw = p.plan.add_production(SAW, home, None);
    p.plan.add_need(COAL, Some(p.home), 0.0);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let p = ai.player_mut(P0);
    assert!(p.plan.needs.iter().any(|n| n.good == TOOLS));
    assert_eq!(
        p.plan.needs.iter().find(|n| n.good == COAL).unwrap().cell,
        None
    );
    // Placed, it supplies them: the coal need goes on.
    p.plan.needs.retain(|n| n.good != TOOLS);
    p.plan
        .prods
        .iter_mut()
        .find(|x| x.id == saw)
        .unwrap()
        .building = Some((30, 30));
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let p = ai.player_mut(P0);
    assert!(!p.plan.needs.iter().any(|n| n.good == TOOLS));
}

// AI-CON T59 (AI-CON-3.3, corrected): the arrangement's distances are
// lattice distances between the cells' positions (AI-CON-1.1a), not their
// centre fields; and the supplier part S and the consumer distance C make
// S + C when both are below 100 000, S when only S is, else C.
// spec: AI-CON-3.3 (AI-CON T59), AI-CON-1.1a
#[test]
fn t59_the_arrangement_measures_between_cell_positions() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let (a, b) = ((3, 3), (4, 3));
    {
        let p = ai.player_mut(P0);
        p.cells.ensure(a, 0).position = Some((50, 50));
        p.cells.ensure(b, 0).position = Some((70, 50));
        p.plan.add_production(WOOD, a, None);
    }
    let (d, _) = ai.with_ctx(&world, P0, |p, ctx| p.supplier_distance(ctx, b, WOOD));
    // A planned producer counts half: (70, 50) to (50, 50) is 20.
    assert_eq!(d, 10);
    let centres = s2_sim::grid::distance(crate::cells::centre(a), crate::cells::centre(b));
    assert_ne!(d, centres / 2, "not the centre fields");
    use crate::needs::combined;
    assert_eq!(combined(30, 40), 70);
    assert_eq!(combined(30, 100_000), 30);
    assert_eq!(combined(100_000, 40), 40);
}

// AI-CON-8.4 (corrected, Q-360, T35): the flag search has no limit on its
// steps, only the distance 10 from the start: a flag 8 away behind a wall
// that forces a way of more than 10 steps (all within distance 10) is
// found; DepositBlocked fields are not crossed.
// spec: AI-CON-8.4 (T35)
#[test]
fn t35_the_flag_search_has_no_step_limit() {
    use s2_sim::map::state;
    let (f, t) = ((14, 24), (22, 24));
    let found = |gap_bits: u32| {
        let mut terrain = flat(96, 96);
        // A wall on column 18 from row 19 down beyond distance 10 of F; the
        // way round goes over rows 13 to 18 of the column.
        for y in 19..=35 {
            let i = terrain.index((18, y));
            terrain.states[i] |= state::BLOCKED;
        }
        for y in 13..=18 {
            let i = terrain.index((18, y));
            terrain.states[i] |= gap_bits;
        }
        let setup = s2_sim::GameSetup {
            players: vec![(P0, (20, 20))],
            local_player: P0,
            ..s2_sim::GameSetup::default()
        };
        let mut world = World::new_game(1, terrain, rules(), &setup);
        for at in [f, t] {
            world.submit(Command::PlaceFlag { player: P0, at });
            world.step();
            assert_eq!(world.last_results()[0].outcome, Ok(()), "{at:?}");
        }
        let castle_flag = crate::view::flag_of(&world, (20, 20)).unwrap();
        assert!(crate::view::dist(f, t) <= 10);
        crate::roads::flag_search(&world, P0, f, &[f, castle_flag], None)
    };
    assert_eq!(found(0), Some(t));
    assert_eq!(found(state::DEPOSIT_BLOCKED), None);
}

// AI-CON-3.1 (Q-178): pass 1 runs over the needs of the food group good
// only: a coal need gets no food sub-need; a food need without a link gets
// one drawn concrete need (fish, bread or flesh) without a consumer, once.
// spec: AI-CON-3.1
#[test]
fn only_food_needs_get_a_drawn_sub_need() {
    let world = world(&[0]);
    let mut cfg = config();
    cfg.names.food = 900;
    cfg.names.flesh = 901;
    let mut ai = start_with(&world, cfg, 2);
    let p = ai.player_mut(P0);
    p.plan.needs.clear();
    p.plan.add_need(COAL, None, 0.0);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let goods: Vec<u32> = ai
        .player_mut(P0)
        .plan
        .needs
        .iter()
        .map(|n| n.good)
        .collect();
    assert!(!goods.contains(&FISH) && !goods.contains(&BREAD) && !goods.contains(&901));
    let food = ai.player_mut(P0).plan.add_need(900, None, 0.0);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let p = ai.player_mut(P0);
    let linked = p
        .plan
        .need(food)
        .and_then(|n| n.food)
        .expect("a linked need");
    let concrete = p.plan.need(linked).expect("the linked need").clone();
    assert!([FISH, BREAD, 901].contains(&concrete.good), "{concrete:?}");
    assert_eq!(concrete.consumer, None);
    assert_eq!(
        p.plan.need(food).unwrap().cell,
        None,
        "food needs get no cell"
    );
    let count = p.plan.needs.len();
    ai.with_ctx(&world, P0, |p, ctx| p.link_food_needs_for_test(ctx));
    assert_eq!(
        ai.player_mut(P0).plan.needs.len(),
        count,
        "linked: nothing more"
    );
}

// AI-CON-3.2 (Q-178): a need with missing inputs (no producer of that good
// id, stock at most 10) gets no cell: the first missing input without a need
// that has no consumer gets one and the pass stops; when every missing input
// has one, the need is passed over.
// spec: AI-CON-3.2
#[test]
fn a_need_with_missing_inputs_gets_no_cell() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    p.plan.needs.clear();
    let consumer = p.home;
    let sword = p.plan.add_need(SWORD, Some(consumer), 0.0);
    p.econ = Economy::from_counts(&[], &[]);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let needs = |ai: &mut AiSystem| -> Vec<(u32, Option<(u32, u32)>)> {
        ai.player_mut(P0)
            .plan
            .needs
            .iter()
            .map(|n| (n.good, n.consumer))
            .collect()
    };
    assert_eq!(needs(&mut ai), [(SWORD, Some(consumer)), (IRON, None)]);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    assert_eq!(
        needs(&mut ai),
        [(SWORD, Some(consumer)), (IRON, None), (COAL, None)]
    );
    // Now both inputs have their needs: the sword need is passed over, and
    // stays without a cell however often the pass runs.
    for _ in 0..5 {
        ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    }
    let p = ai.player_mut(P0);
    assert_eq!(p.plan.need(sword).unwrap().cell, None);
    assert_eq!(p.plan.needs.iter().filter(|n| n.good == IRON).count(), 1);
}

// AI-CON T42 (7.2, Q-363): the hunter run counts the cell's animals by
// their home field, the respawn record's count (ECO-FOOD-5.15), with no
// exploration test: an animal homed on a field of the player's first cell
// that the player has not explored gives that cell a hunter order (weight
// 10); without it the cell gets none.
// spec: AI-CON-7.2 (T42), ECO-FOOD-5.15
#[test]
fn t42_the_hunter_run_counts_animals_by_home() {
    let order_for = |with_animal: bool| {
        let mut world = world(&[0]);
        let mut ai = start(&world, 2);
        let cell = ai.player_mut(P0).cells.in_order()[0];
        if with_animal {
            let (x0, y0) = (cell.0 * 16, cell.1 * 16);
            let field = (x0..x0 + 16)
                .flat_map(|x| (y0..y0 + 16).map(move |y| (x, y)))
                .find(|&f| !world.is_explored(P0, f))
                .expect("an unexplored field in the cell");
            world.add_animal(1, field);
        }
        ai.player_mut(P0).hunter_index = 0;
        ai.with_ctx(&world, P0, |p, ctx| p.hunter_construction(ctx));
        ai.player_mut(P0).build.slot(Slot::Hunter).copied()
    };
    let order = order_for(true).expect("a hunter order");
    assert_eq!((order.building, order.weight), (HUNTER, 10));
    assert!(order_for(false).is_none());
}

// AI-CON-1.2, 8.3 a (AI-CON T46): a record is a planting building only when
// its type has no product and a planting good: the forester is one, a farm
// (a product, grain, and a planting good) is not, so the route task does
// not skip it and AI-CON-9.1 does not protect it by that flag; a harbour
// has the depot flag (T45).
// spec: AI-CON-1.2 (T46), AI-CON-8.3, AI-CON-9.1
#[test]
fn t46_a_farm_is_not_a_planting_building() {
    use crate::player::Protect;
    let rules = crate::fixture::rules();
    let of = |id: u32| Protect::of(rules.building_types.iter().find(|t| t.id == id).unwrap());
    assert!(of(FORESTER).planting);
    let mut farm = s2_sim::BuildingType::new(900, s2_sim::BuildingSize::Large);
    farm.product = Some(BREAD);
    farm.planting_good = Some(BREAD);
    assert!(!Protect::of(&farm).planting);
    assert!(!Protect::of(&farm).any());
    assert!(of(HARBOR).depot && of(HARBOR).harbour);
}

// AI-CON-8.3 b, AI-CON-2.5 (AI-CON T51): a building's needs are in the order
// they were tied to it: the bakery's water need, made new, comes before the
// older flour need without a consumer that is tied after it (bread's cost
// list is water, flour), whatever their ids.
// spec: AI-CON-8.3 (T51), AI-CON-2.5
#[test]
fn t51_a_buildings_needs_in_the_order_they_were_tied() {
    const WATER: u32 = 901;
    const FLOUR: u32 = 902;
    let bakery = (30, 30);
    let mut plan = crate::plan::Plan::default();
    let flour = plan.add_need(FLOUR, None, 0.0);
    let water = plan.tie_need(WATER, bakery, 1.0);
    assert_eq!(plan.tie_need(FLOUR, bakery, 1.0), flour, "the free one");
    assert!(flour < water);
    let mut list: Vec<&crate::plan::Need> = plan
        .needs
        .iter()
        .filter(|n| n.consumer == Some(bakery))
        .collect();
    list.sort_by_key(|n| n.tied);
    let ids: Vec<_> = list.iter().map(|n| n.id).collect();
    assert_eq!(ids, [water, flour]);
}

// AI-CON T65 (AI-CON-3.4): the squares of a cell are visited sx outer, sy
// inner; a surface good takes the first square with a counting corner, not
// the one with the most; and a square given to a need is taken from that
// arrangement on, so the next need of the good takes the next free square,
// and with none left it waits.
// spec: AI-CON-3.4 (AI-CON T65)
#[test]
fn t65_each_need_takes_the_next_free_square_in_order() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    assert_eq!(cell_of(p.home), (1, 1));
    let landmass = p.home_landmass;
    for (sq, amount) in [((4, 5), 1), ((5, 4), 9), ((5, 5), 5)] {
        p.resources.set_entries(
            sq,
            vec![crate::resources::Entry {
                good: WOOD,
                kind: crate::resources::SURFACE,
                landmass,
                amount,
            }],
        );
    }
    p.econ = Economy::from_counts(&[(WOOD, 50)], &[]);
    for _ in 0..4 {
        p.plan.add_need(WOOD, Some((1, 1)), 0.0);
    }
    for _ in 0..8 {
        ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    }
    let squares: Vec<Option<(u32, u32)>> = ai
        .player_mut(P0)
        .plan
        .needs
        .iter()
        .filter(|n| n.good == WOOD)
        .map(|n| n.square.map(|(s, _)| s))
        .collect();
    assert_eq!(squares, [Some((4, 5)), Some((5, 4)), Some((5, 5)), None]);
    let last = ai
        .player_mut(P0)
        .plan
        .needs
        .iter()
        .rfind(|n| n.good == WOOD)
        .cloned();
    assert!(last.expect("the fourth need").waiting);
}

// AI-CON T94 (AI-CON-10.4 step 0): the emptiest and the fullest storehouse
// are each the first found; when the smallest count + 40 is below the
// largest, out goes on the fullest and stop is cleared on the emptiest (R5,
// counter 45647: stone 42 / 2 / 0, the third stays open).
// spec: AI-CON-10.4 (AI-CON T94)
#[test]
fn t94_the_first_emptiest_and_fullest() {
    use crate::control::first_extremes;
    let counts = [(42, (1, 1)), (2, (2, 2)), (0, (3, 3))];
    assert_eq!(first_extremes(&counts), Some(((0, (3, 3)), (42, (1, 1)))));
    // Equal counts: the first of them, at either end.
    let ties = [(5, (1, 1)), (9, (2, 2)), (5, (3, 3)), (9, (4, 4))];
    assert_eq!(first_extremes(&ties), Some(((5, (1, 1)), (9, (2, 2)))));
    assert_eq!(first_extremes(&[]), None);
}

// AI-CON T100 (AI-CON-10.4 step 0): "first found" follows the cell list and
// each cell's depots in the order they were added, not the fields; and the
// run leaves stop on every warehouse but the emptiest, stop clear on the
// emptiest and out on the fullest, also when the emptiest's record exists
// and is clear. Two empty storehouses in one cell, the one added first on
// the later field: it is the emptiest.
// spec: AI-CON-10.4 (AI-CON T100, T94)
#[test]
fn t100_step_0_in_cell_list_order_leaves_the_emptiest_open() {
    use crate::snapshot::Snapshot;
    let mut rules = crate::fixture::rules();
    rules.start_stock.retain(|&(g, _)| g != PLANK);
    rules.start_stock.push((PLANK, 60));
    let mut world = crate::fixture::world_on(1, &[0], &Extras::default(), rules);
    let mut ai = start(&world, 2);
    let hq = ai.player_mut(P0).home;
    let (first, second) = ((24, 22), (18, 19));
    assert_eq!(cell_of(first), cell_of(second));
    assert!((second.1, second.0) < (first.1, first.0));
    for at in [first, second] {
        with_building(&mut world, DEPOT, at);
        finish_site(&mut world, at);
        ai.update_with(&world, P0, &Snapshot::take(&world));
    }
    let run = |world: &mut World, ai: &mut AiSystem, step| {
        ai.player_mut(P0).goods_step = step;
        let snapshot = Snapshot::take(world);
        let (_, out) = ai.with_snapshot(world, P0, &snapshot, |p, ctx| p.goods_arrangement(ctx));
        for c in out {
            world.submit(c);
        }
        world.step();
    };
    // Step 1 makes the records, all clear; then step 0.
    run(&mut world, &mut ai, 1);
    run(&mut world, &mut ai, 0);
    let flags = |at| {
        let f = world.warehouse(at).unwrap().flags_of(PLANK);
        (f.stop, f.out)
    };
    assert_eq!(flags(hq), (true, true), "the fullest");
    assert_eq!(flags(first), (false, false), "the emptiest, first found");
    assert_eq!(flags(second), (true, false));
}

// AI-CON T93 (AI-CON-8.1e, AI-2.7 b): the computer player's searches enter
// only fields usable by the player: its inner land (not a Border field) and
// not Blocked, DepositBlocked or MovementBlocked; a tree's field is closed
// to the road-stop search too, and no path runs along the Border ring.
// spec: AI-CON-8.1e, AI-2.7 (AI-CON T93)
#[test]
fn t93_searches_enter_only_usable_fields() {
    let tree = (23, 23);
    let world = world_with(
        &[0],
        &Extras {
            trees: vec![tree],
            rocks: Vec::new(),
            fish: Vec::new(),
        },
    );
    let terrain = world.terrain();
    assert!(!crate::roads::usable(&world, P0, tree), "a tree's field");
    let open = terrain.neighbour(tree, 0).unwrap();
    assert!(crate::roads::usable(&world, P0, open));
    // The player's Border ring is owned but not usable.
    let border = (0..terrain.height)
        .flat_map(|y| (0..terrain.width).map(move |x| (x, y)))
        .find(|&f| world.field_owner(f) == Some(P0) && terrain.has(f, s2_sim::map::state::BORDER))
        .expect("a Border field of the player");
    assert!(!crate::roads::usable(&world, P0, border));
    // Not even as a walk path's goal.
    let inner = terrain
        .neighbours(border)
        .into_iter()
        .flatten()
        .find(|&f| crate::roads::usable(&world, P0, f))
        .expect("an inner neighbour");
    assert_eq!(
        crate::roads::walk_path(&world, P0, border, inner, &[], &[]),
        Some(1)
    );
    let far =
        terrain.neighbours(inner).into_iter().flatten().find(|&f| {
            crate::roads::usable(&world, P0, f) && s2_sim::grid::distance(f, border) == 2
        });
    if let Some(far) = far {
        assert_eq!(
            crate::roads::walk_path(&world, P0, border, far, &[], &[]),
            None
        );
    }
}

// AI-CON T68 (AI-CON-8.1e): a field with a tree (DepositBlocked) blocks the
// walk paths but at their goal, as a flag does: the 2-step way across the
// tree's field is closed and the path goes round.
// spec: AI-CON-8.1e (AI-CON T68)
#[test]
fn t68_a_tree_blocks_the_walk_paths() {
    let tree = (23, 23);
    let world = world_with(
        &[0],
        &Extras {
            trees: vec![tree],
            rocks: Vec::new(),
            fish: Vec::new(),
        },
    );
    let terrain = world.terrain();
    assert!(terrain.has(tree, s2_sim::map::state::DEPOSIT_BLOCKED));
    let (a, b) = (
        terrain.neighbour(tree, 0).unwrap(),
        terrain.neighbour(tree, 3).unwrap(),
    );
    let path = crate::roads::walk_path(&world, P0, a, b, &[], &[]);
    assert!(path.is_none_or(|n| n > 2), "{path:?}");
    // Its own goal is no obstacle.
    assert_eq!(
        crate::roads::walk_path(&world, P0, tree, b, &[], &[]),
        Some(1)
    );
}

// AI-CON T71 (AI-CON-3.3, Q-370): a cell accepts a plain good while it holds
// fewer than 20 needs, the needs that have it as their cell; its
// productions do not count. Coal is not limited.
// spec: AI-CON-3.3 (AI-CON T71)
#[test]
fn t71_the_accept_limit_counts_the_cells_needs() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let cell = cell_of(ai.player_mut(P0).home);
    for _ in 0..25 {
        ai.player_mut(P0).plan.add_production(PLANK, cell, None);
    }
    let accepts = |ai: &mut AiSystem, good| {
        ai.with_ctx(&world, P0, |p, ctx| p.cell_accepts(ctx, cell, good))
            .0
    };
    assert!(accepts(&mut ai, PLANK), "productions do not count");
    let p = ai.player_mut(P0);
    for _ in 0..20 {
        let id = p.plan.add_need(PLANK, None, 0.0);
        p.plan.need_mut(id).unwrap().cell = Some(cell);
    }
    assert!(!accepts(&mut ai, PLANK), "20 needs");
    assert!(accepts(&mut ai, COAL), "coal is not limited");
}

// AI-CON T70 (AI-CON-3.3, Q-370): the supplier distance is halved (rounded
// down) when the producing cell holds a free production of the good, one
// without a need or whose need has no consumer building; a production whose
// need has a consumer counts the full distance, placed or not.
// spec: AI-CON-3.3 (AI-CON T70)
#[test]
fn t70_a_free_production_halves_the_supplier_distance() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    let (home, at) = (cell_of(p.home), p.home);
    let other = (home.0 + 2, home.1);
    let need = p.plan.add_need(PLANK, Some(at), 0.0);
    let prod = p.plan.add_production(PLANK, other, Some(need));
    let distance = |ai: &mut AiSystem| {
        ai.with_ctx(&world, P0, |p, ctx| p.supplier_distance(ctx, home, PLANK))
            .0
    };
    let full = distance(&mut ai);
    assert!(full > 1 && full < 100_000, "{full}");
    // Placed or not, a production with a consumer counts in full.
    let p = ai.player_mut(P0);
    p.plan
        .prods
        .iter_mut()
        .find(|x| x.id == prod)
        .unwrap()
        .building = Some((40, 20));
    assert_eq!(distance(&mut ai), full);
    // Its need without a consumer building: free, halved.
    ai.player_mut(P0).plan.need_mut(need).unwrap().consumer = None;
    assert_eq!(distance(&mut ai), full / 2);
    // Without a need: free.
    let p = ai.player_mut(P0);
    p.plan.need_mut(need).unwrap().consumer = Some(at);
    p.plan.prods.iter_mut().find(|x| x.id == prod).unwrap().need = None;
    assert_eq!(distance(&mut ai), full / 2);
}

// AI-CON T77 (AI-CON-3.3, Q-371): the consumer distance skips only the
// settler goods; a ware group made from the good counts: a free tools
// production (tools from planks and iron) two cells away is a plank consumer,
// its distance halved and times the plank needs of its cell plus 1.
// spec: AI-CON-3.3 (AI-CON T77)
#[test]
fn t77_a_ware_group_is_a_consumer() {
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    let home = cell_of(p.home);
    let other = (home.0 + 2, home.1);
    p.plan.prods.retain(|x| x.good != TOOLS && x.good != BOAT);
    let distance = |ai: &mut AiSystem| {
        ai.with_ctx(&world, P0, |p, ctx| p.consumer_distance(ctx, home, PLANK))
            .0
    };
    assert_eq!(distance(&mut ai), 100_000, "no plank consumer yet");
    let p = ai.player_mut(P0);
    p.plan.add_production(TOOLS, other, None);
    let one = distance(&mut ai);
    assert!(
        one > 0 && one < 100_000,
        "the tools production counts: {one}"
    );
    // Two plank needs in the consumer cell: (d / 2) x (2 + 1).
    let p = ai.player_mut(P0);
    for _ in 0..2 {
        let n = p.plan.add_need(PLANK, None, 0.0);
        p.plan.need_mut(n).unwrap().cell = Some(other);
    }
    assert_eq!(distance(&mut ai), one * 3);
}

// AI-CON T85 (AI-CON-3.3, corrected 2026-10-30): the consumer factor counts
// the consumer cell's needs whose good is the looked-up good (food for a
// member of the food group) or whose good's group is it: for a fish need the
// cell's food, fish, bread and meat needs. A free coal production (coal from
// food here) two cells away; a bread need in its cell doubles its term, a
// plank need does not.
// spec: AI-CON-3.3 (AI-CON T85)
#[test]
fn t85_the_consumer_factor_counts_the_food_group() {
    const FOOD: u32 = 77;
    let world = world(&[0]);
    let mut config = config();
    config.names.food = FOOD;
    config.sim.goods.get_mut(&COAL).unwrap().costs = vec![(FOOD, 1)];
    let mut ai = start_with(&world, config, 2);
    let p = ai.player_mut(P0);
    let home = cell_of(p.home);
    let other = (home.0 + 2, home.1);
    p.plan.prods.retain(|x| x.good != COAL);
    p.plan.add_production(COAL, other, None);
    let distance = |ai: &mut AiSystem| {
        ai.with_ctx(&world, P0, |p, ctx| p.consumer_distance(ctx, home, FISH))
            .0
    };
    let one = distance(&mut ai);
    assert!(
        one > 0 && one < 100_000,
        "the coal production counts: {one}"
    );
    let p = ai.player_mut(P0);
    let plank = p.plan.add_need(PLANK, None, 0.0);
    p.plan.need_mut(plank).unwrap().cell = Some(other);
    assert_eq!(distance(&mut ai), one, "a plank need is not counted");
    let p = ai.player_mut(P0);
    let bread = p.plan.add_need(BREAD, None, 0.0);
    p.plan.need_mut(bread).unwrap().cell = Some(other);
    assert_eq!(distance(&mut ai), one * 2, "a bread need is");
}

// AI-CON T78 (AI-CON-4.2): avgProd counts the producers' sites, with the 0.5
// their placement gives them (ECO-OUT-3.11): a finished woodcutter at 1.0
// and a woodcutter site make 0.75.
// spec: AI-CON-4.2 (AI-CON T78), ECO-OUT-3.11
#[test]
fn t78_the_mean_productivity_counts_sites() {
    let mut world = world(&[0]);
    let (done, site) = ((26, 22), (24, 24));
    with_building(&mut world, WOODCUTTER, done);
    with_building(&mut world, WOODCUTTER, site);
    let mut ai = start(&world, 2);
    // T107: the mean runs over the production records; each woodcutter has
    // one, as a placed producer does (AI-CON-2.5b).
    {
        let p = ai.player_mut(P0);
        p.plan.prods.retain(|q| q.good != WOOD);
        for at in [done, site] {
            let prod = p.plan.add_production(WOOD, cell_of(at), None);
            p.plan
                .prods
                .iter_mut()
                .find(|q| q.id == prod)
                .unwrap()
                .building = Some(at);
        }
    }
    let snapshot = finished_snapshot(&world, done, 100);
    let s = snapshot.building(P0, site).expect("the site");
    assert!(s.under_construction);
    assert_eq!(f32::from_bits(s.productivity_bits), 0.5);
    let (mean, _) = ai.with_snapshot(&world, P0, &snapshot, |p, ctx| {
        p.average_productivity(ctx, WOOD)
    });
    assert_eq!(mean, 0.75);
}

// AI-CON T107 (AI-CON-4.2, E-ai-075): avgProd runs over exactly the
// production records A(g) counts, each with its own building's value; a
// building of the good without a record (the hunter's hut, AI-CON-2.5a) does
// not count. With only a hunter's hut (a site, 0.5) meat's mean is 1.0; a
// woodcutter site with its record makes wood's 0.5.
// spec: AI-CON-4.2 (AI-CON T107), AI-CON-2.5a
#[test]
fn t107_the_mean_productivity_counts_production_records_only() {
    let mut world = world(&[0]);
    let (hut, cutter) = ((26, 22), (24, 24));
    with_building(&mut world, HUNTER, hut);
    with_building(&mut world, WOODCUTTER, cutter);
    let mut ai = start(&world, 2);
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let hut_value = snapshot
        .building(P0, hut)
        .expect("the hut")
        .productivity_bits;
    assert_eq!(f32::from_bits(hut_value), 0.5);
    let meat = FISH + 100;
    {
        let p = ai.player_mut(P0);
        p.plan.prods.retain(|q| q.good != meat && q.good != WOOD);
        let prod = p.plan.add_production(WOOD, cell_of(cutter), None);
        p.plan
            .prods
            .iter_mut()
            .find(|q| q.id == prod)
            .unwrap()
            .building = Some(cutter);
    }
    let (means, _) = ai.with_snapshot(&world, P0, &snapshot, |p, ctx| {
        (
            p.average_productivity(ctx, meat),
            p.average_productivity(ctx, WOOD),
        )
    });
    assert_eq!(means, (1.0, 0.5));
}

// AI-CON T72 (AI-CON-6.2, Q-370): a fields good's site value is the sum of
// the resource amounts of the good on all fields within the working radius,
// explored by the player or not.
// spec: AI-CON-6.2 (AI-CON T72)
#[test]
fn t72_a_fields_goods_value_sums_the_amounts() {
    let far = (70, 70);
    let fish: Vec<(u32, u32)> = vec![(70, 70), (71, 70), (70, 71)];
    let world = world_with(
        &[0],
        &Extras {
            trees: Vec::new(),
            rocks: Vec::new(),
            fish: fish.clone(),
        },
    );
    assert!(fish.iter().all(|&f| !world.is_explored(P0, f)));
    let config = config();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let env = crate::sites::SiteEnv {
        world: &world,
        config: &config,
        player: P0,
        own: snapshot.of(P0),
    };
    // Each fish field of the fixture holds 5.
    assert_eq!(env.resource_fields(far, 2, FISH), 15);
}

// AI-CON T81 (AI-CON-6.2, Q-375): a deposits good's site value counts the
// deposits over a flood from the field: it steps to a neighbour within the
// working radius that holds a deposit or is walkable. A tree within the
// radius but beyond a line of water is not reached and does not count.
// spec: AI-CON-6.2 (AI-CON T81)
#[test]
fn t81_deposits_are_counted_over_a_flood() {
    let at = (70, 70);
    let (near, beyond) = ((72, 70), (74, 70));
    let mut terrain = crate::fixture::flat(96, 96);
    for y in 60..80 {
        let i = terrain.index((73, y));
        terrain.states[i] = s2_sim::map::state::WATER;
    }
    let world = crate::fixture::world_on_terrain(
        1,
        &[0],
        &Extras {
            trees: vec![near, beyond],
            rocks: Vec::new(),
            fish: Vec::new(),
        },
        crate::fixture::rules(),
        terrain,
    );
    assert!(s2_sim::grid::distance(at, beyond) <= 4);
    let config = config();
    let snapshot = crate::snapshot::Snapshot::take(&world);
    let env = crate::sites::SiteEnv {
        world: &world,
        config: &config,
        player: P0,
        own: snapshot.of(P0),
    };
    assert_eq!(env.deposits_of(at, 4, WOOD), 1);
    assert_eq!(env.deposits_of((75, 70), 4, WOOD), 1, "from the other side");
}

// AI-CON T90 (AI-CON-6.2): the site value's deposit flood counts a sapling
// as a tree from the moment the forester makes it, before it grows; a
// deposit being removed does not count.
// spec: AI-CON-6.2 (AI-CON T90)
#[test]
fn t90_a_sapling_counts_for_the_site_value() {
    use s2_sim::DepositState;
    assert!(crate::sites::counts_for_site(&DepositState::Sapling));
    assert!(crate::sites::counts_for_site(&DepositState::Growing {
        ripe_at: 0
    }));
    assert!(!crate::sites::counts_for_site(&DepositState::Removing {
        since: 0,
        gone_at: 20
    }));
}

// AI-CON T73 (AI-CON-10.1): production control walks the goods in id order
// read as signed 32-bit numbers: ids with the top bit set come first.
// spec: AI-CON-10.1 (AI-CON T73)
#[test]
fn t73_production_control_walks_the_goods_in_signed_order() {
    let order =
        crate::player::AiPlayer::control_order(&[0x1762_6a03, 0x8198_bf23, 0x5432, 0xa948_3903]);
    assert_eq!(order, [0x8198_bf23, 0xa948_3903, 0x5432, 0x1762_6a03]);
}

// AI-CON T76 (AI-CON-1.8): the owned-fields statistic leaves out the fields
// without a continent (index -1) as well as water.
// spec: AI-CON-1.8 (AI-CON T76)
#[test]
fn t76_the_statistic_leaves_out_fields_without_a_continent() {
    let mut world = world(&[0]);
    let all = crate::player::owned_land(&world, P0);
    assert!(all > 10);
    let home = (20, 20);
    let terrain = world.terrain().clone();
    let mut continents: Vec<i32> = terrain
        .fields()
        .map(|f| world.continent_of(f).unwrap())
        .collect();
    let blocked: Vec<_> = terrain
        .fields()
        .filter(|&f| world.field_owner(f) == Some(P0) && s2_sim::grid::distance(f, home) <= 1)
        .collect();
    for &f in &blocked {
        continents[terrain.index(f)] = -1;
    }
    world.set_continents(continents);
    assert_eq!(
        crate::player::owned_land(&world, P0),
        all - blocked.len() as u32
    );
}

// AI-CON T89 (AI-CON-3.4, stone): a cell's squares are scanned once; an
// underground entry only updates the best underground square, a surface
// entry with a counting corner ends the scan. So a cell offers its first
// surface square if it has one, else its best underground square, and the
// need keeps the square's kind. (The cells then compete on cost alone,
// whatever the kind; measured on R3 in docs/fidelity.md.)
// spec: AI-CON-3.4 (AI-CON T89)
#[test]
fn t89_a_cell_offers_one_square_of_either_kind() {
    use crate::resources::{Entry, SURFACE, UNDERGROUND};
    let world = world(&[0]);
    let mut ai = start(&world, 2);
    let p = ai.player_mut(P0);
    let cell = cell_of(p.home);
    assert_eq!(cell, (1, 1));
    let landmass = p.home_landmass;
    let entry = |kind, amount| Entry {
        good: STONE,
        kind,
        landmass,
        amount,
    };
    // Underground only: the best underground square (the largest amount).
    p.resources.set_entries((5, 4), vec![entry(UNDERGROUND, 3)]);
    p.resources.set_entries((5, 5), vec![entry(UNDERGROUND, 7)]);
    let kinds = [SURFACE, UNDERGROUND];
    let (offer, _) = ai.with_ctx(&world, P0, |p, ctx| p.best_square(ctx, cell, STONE, &kinds));
    assert_eq!(offer, Some(((5, 5), UNDERGROUND)));
    // A surface square later in the scan ends it: offered with its kind.
    let p = ai.player_mut(P0);
    p.resources.set_entries((6, 4), vec![entry(SURFACE, 1)]);
    let (offer, _) = ai.with_ctx(&world, P0, |p, ctx| p.best_square(ctx, cell, STONE, &kinds));
    assert_eq!(offer, Some(((6, 4), SURFACE)));
}

// ---- Hook W12: a project's building type in the computer players' choice ----

/// A project's coal-from-wood type (the Charcoal burner's shape): a helper
/// works it, two wood make one coal.
const BURNER: u32 = 150;

fn burner_rules() -> s2_sim::Rules {
    let mut rules = town_rules();
    let mut burner = s2_sim::BuildingType::new(BURNER, s2_sim::BuildingSize::Small);
    burner.worker = Some(HELPER);
    burner.product = Some(COAL);
    burner.cycle = s2_sim::WorkCycle::Workshop;
    burner.working_radius = 4;
    burner.costs = vec![(PLANK, 1)];
    burner.stock_limit = Some(4);
    rules.building_types.push(burner);
    rules.recipes.insert(BURNER, vec![(WOOD, 2)]);
    rules
}

/// Coal wanted below 4, with 8 wood in stock, at most `most` per player.
fn burner_alternative(most: u32) -> std::collections::BTreeMap<u32, crate::AiAlternative> {
    [(
        COAL,
        crate::AiAlternative {
            building: BURNER,
            stock_below: 4,
            input: WOOD,
            input_at_least: 8,
            most,
        },
    )]
    .into()
}

fn burner_config(alternative: Option<u32>) -> crate::AiConfig {
    let mut config = config_for(&burner_rules());
    config.set_project(
        [BURNER].into(),
        alternative.map_or_else(Default::default, burner_alternative),
    );
    config
}

// W12 step 1: a type a project's overlay made is in neither producer table.
// Without that the burner would be coal's normal-table producer, and
// AI-CON-4.1 would check its helper for every coal need instead of the
// miner.
// spec: AI-CON-1.4, AI-CON-4.1 (hook W12)
#[test]
fn w12_a_projects_type_stays_out_of_the_producer_tables() {
    let unmarked = config_for(&burner_rules());
    assert_eq!(unmarked.producer.get(&COAL), Some(&BURNER));
    let plain = config_for(&town_rules());
    let config = burner_config(None);
    assert_eq!(config.producer, plain.producer);
    assert_eq!(config.mine_producer, plain.mine_producer);
    assert!(plain.project_types.is_empty() && plain.alternatives.is_empty());
}

/// A town game of players 0 and 1 with a coal need for player 0 at its
/// headquarters, played for `ticks`; the world and AI hashes every 500.
fn coal_game(
    rules: s2_sim::Rules,
    config: crate::AiConfig,
    ticks: u64,
) -> (World, AiSystem, Vec<(u64, u64)>) {
    let (mut world, _) = game_on(&[0, 1], 7, rules);
    let mut ai = AiSystem::new(config, &world);
    let slots = crate::every_player(&world);
    ai.activate_all(&mut world, &slots);
    let home = ai.player_mut(P0).home;
    ai.player_mut(P0).plan.add_need(COAL, Some(home), 0.0);
    let mut hashes = Vec::new();
    for t in 1..=ticks {
        ai.step(&mut world);
        if t % 500 == 0 {
            hashes.push((world.state_hash(), ai.state_hash()));
        }
    }
    (world, ai, hashes)
}

// W12 step 1, by play: with the project's type and no alternative the
// computer players play exactly the game they play without the type.
// spec: AI-CON-1.4, AI-CON-3.4 (hook W12)
#[test]
fn w12_without_an_alternative_the_players_leave_the_type_alone() {
    // The world's hash covers its rules, which have one more type: the
    // computer players' hashes and the buildings are compared.
    let (plain_world, _, plain) = coal_game(town_rules(), config_for(&town_rules()), 3000);
    let (world, _, with_type) = coal_game(burner_rules(), burner_config(None), 3000);
    let ai = |h: &[(u64, u64)]| h.iter().map(|&(_, a)| a).collect::<Vec<_>>();
    assert_eq!(ai(&with_type), ai(&plain));
    assert_eq!(
        world.buildings().copied().collect::<Vec<_>>(),
        plain_world.buildings().copied().collect::<Vec<_>>()
    );
    assert!(!world.buildings().any(|b| b.building_type == BURNER));
}

// W12 steps 2 to 4: a coal need no cell has a square for is arranged for
// the alternative while coal is short and wood is there (a cell, no
// square), and needs construction orders the alternative's type; with too
// little wood, coal enough, the most reached or a coal square in a cell it
// is AI-CON-3.4's as before.
// spec: AI-CON-3.2, AI-CON-3.4, AI-CON-4.1, AI-CON-4.3 (hook W12)
#[test]
fn w12_a_coal_need_without_a_square_takes_the_alternative_when_its_limits_allow() {
    let arranged = |config: crate::AiConfig, stock: &[(u32, u32)], before: usize| {
        let world = world(&[0]);
        let mut ai = start_with(&world, config, 2);
        let home = ai.player_mut(P0).home;
        let p = ai.player_mut(P0);
        for _ in 0..before {
            let id = p.plan.add_need(COAL, Some(home), 0.0);
            p.plan.need_mut(id).unwrap().cell = Some(cell_of(home));
        }
        let id = p.plan.add_need(COAL, Some(home), 0.0);
        p.econ = Economy::from_counts(stock, &[]);
        ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
        let need = ai.player_mut(P0).plan.need(id).cloned().unwrap();
        (world, ai, need)
    };
    // 20 wood: above AI-CON-3.2's limit of 10, so the recipe's input is not
    // missing.
    let wood = [(WOOD, 20), (PLANK, 20), (STONE, 20)];
    let (world, mut ai, need) = arranged(burner_config(Some(2)), &wood, 0);
    let home = ai.player_mut(P0).home;
    assert_eq!(need.cell, Some(cell_of(home)));
    assert_eq!(need.square, None);
    assert!(!need.waiting);
    ai.player_mut(P0).statistic = (1, 300);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_construction(ctx));
    let order = ai.player_mut(P0).build.slot(Slot::Needs).copied();
    assert_eq!(order.map(|o| o.building), Some(BURNER));

    // Too little wood; coal enough (on the roads counts); the most reached;
    // no alternative: the need waits, as AI-CON-3.4 has it.
    for (config, stock, before) in [
        (burner_config(Some(2)), vec![(WOOD, 7)], 0),
        (burner_config(Some(2)), vec![(WOOD, 20), (COAL, 4)], 0),
        (burner_config(Some(2)), vec![(WOOD, 20)], 2),
        (burner_config(None), vec![(WOOD, 20)], 0),
    ] {
        let (_, _, need) = arranged(config, &stock, before);
        assert!(need.waiting && need.cell.is_none(), "{stock:?} {before}");
    }
    // The second of at most two is still arranged for it.
    let (_, _, need) = arranged(burner_config(Some(2)), &[(WOOD, 20)], 1);
    assert!(need.cell.is_some() && need.square.is_none());

    // A coal square in the headquarters' cell: the mine's square, as before.
    let world = crate::fixture::world(&[0]);
    let mut ai = start_with(&world, burner_config(Some(2)), 2);
    let home = ai.player_mut(P0).home;
    let landmass = ai.player_mut(P0).home_landmass;
    let sq = crate::resources::square_of(home);
    ai.player_mut(P0).resources.set_entries(
        sq,
        vec![crate::resources::Entry {
            good: COAL,
            kind: crate::resources::UNDERGROUND,
            landmass,
            amount: 5,
        }],
    );
    let id = ai.player_mut(P0).plan.add_need(COAL, Some(home), 0.0);
    ai.player_mut(P0).econ = Economy::from_counts(&[(WOOD, 20)], &[]);
    ai.with_ctx(&world, P0, |p, ctx| p.needs_arrangement(ctx));
    let need = ai.player_mut(P0).plan.need(id).cloned().unwrap();
    assert_eq!(need.square, Some((sq, crate::resources::UNDERGROUND)));
}

// W12, by play: player 0 has no coal square and wood to spare; with the
// alternative it places the project's type (one, its most), the type gets
// its helper and makes coal from the wood.
// spec: AI-CON-3.4, AI-CON-4.3 (hook W12)
#[test]
fn w12_with_the_alternative_a_player_short_of_coal_builds_the_type_and_it_makes_coal() {
    let (world, mut ai, _) = coal_game(burner_rules(), burner_config(Some(1)), 12_000);
    let burners: Vec<_> = world
        .buildings()
        .filter(|b| b.building_type == BURNER)
        .collect();
    assert_eq!(burners.len(), 1, "{burners:?}");
    let burner = burners[0];
    assert_eq!(burner.owner, P0);
    assert!(!burner.under_construction && burner.active, "{burner:?}");
    let home = ai.player_mut(P0).home;
    let stored = world
        .stock(home)
        .and_then(|s| s.iter().find(|&&(g, _)| g == COAL))
        .map_or(0, |&(_, n)| n);
    assert!(
        stored + ai.player_mut(P0).econ.total(COAL) > 0,
        "no coal made"
    );
}
