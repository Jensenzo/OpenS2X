//! Tests of ships and expeditions (`AI-misc.md` T1 to T7).

use s2_sim::{Command, Landing, PlayerId, Vessel, World};

use crate::construction::Slot;
use crate::ships::{sail_decision, Sail};
use crate::testutil::*;
use crate::view::Economy;
use crate::AiSystem;

const P0: PlayerId = PlayerId(0);
const HARBOUR: (u32, u32) = (26, 22);
const SHIPYARD_AT: (u32, u32) = (23, 26);
const SEA: (u32, u32) = (33, 22);

/// The sea world with a finished harbour and shipyard of player 0.
fn harbour_world() -> World {
    let mut world = sea_world();
    with_building(&mut world, HARBOR, HARBOUR);
    finish_building(&mut world, HARBOUR, vec![(STONE, 2)]);
    with_building(&mut world, SHIPYARD, SHIPYARD_AT);
    finish_building(&mut world, SHIPYARD_AT, vec![(STONE, 2)]);
    world
}

fn start(world: &World) -> AiSystem {
    let mut ai = AiSystem::new(config(), world);
    assert!(ai.activate_with(world, &mut s2_sim::Numbers::new(0), P0, 2));
    ai
}

// AI-MISC T1: a finished shipyard in boat mode, 20 planks and fewer ships than
// two per harbour: the mode is switched to ships and production stays on;
// with as many ships as wanted it is switched off.
// spec: AI-MISC-1.1 (AI-MISC T1)
#[test]
fn t1_shipyard_makes_ships_up_to_two_per_harbour() {
    let mut world = harbour_world();
    let mut ai = start(&world);
    ai.player_mut(P0).econ = Economy::from_counts(&[(PLANK, 20)], &[]);
    world.debug_add_ship(P0, SEA);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.ship_construction(ctx));
    assert!(commands.contains(&Command::ProductionSwitch {
        player: P0,
        at: SHIPYARD_AT
    }));
    assert!(commands
        .iter()
        .all(|c| !matches!(c, Command::ProductionToggle { .. })));
    // Two harbours' worth of ships (one harbour: two ships): off.
    world.debug_add_ship(P0, (34, 24));
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.ship_construction(ctx));
    assert!(commands.contains(&Command::ProductionToggle {
        player: P0,
        at: SHIPYARD_AT
    }));
}

// AI-MISC T2: with less than 12 planks the shipyard is switched off.
// spec: AI-MISC-1.1 (AI-MISC T2)
#[test]
fn t2_shipyard_waits_for_planks() {
    let world = harbour_world();
    let mut ai = start(&world);
    ai.player_mut(P0).econ = Economy::from_counts(&[(PLANK, 10)], &[]);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.ship_construction(ctx));
    assert!(commands.contains(&Command::ProductionToggle {
        player: P0,
        at: SHIPYARD_AT
    }));
}

// AI-MISC-1.1: with a finished harbour and no shipyard, a shipyard is ordered
// in the harbour's cell (weight 10); a failed order marks the cell full.
#[test]
fn a_harbour_cell_gets_a_shipyard_order() {
    let mut world = sea_world();
    with_building(&mut world, HARBOR, HARBOUR);
    finish_building(&mut world, HARBOUR, vec![(STONE, 2)]);
    let mut ai = start(&world);
    ai.with_ctx(&world, P0, |p, ctx| p.ship_construction(ctx));
    let order = ai.player_mut(P0).build.slot(Slot::Ship).copied().unwrap();
    assert_eq!((order.building, order.weight), (SHIPYARD, 10));
    assert_eq!(
        order.area,
        crate::sites::Area::Cell(crate::cells::cell_of(HARBOUR))
    );
    // No harbour: no order.
    let world = sea_world();
    let mut ai = start(&world);
    ai.with_ctx(&world, P0, |p, ctx| p.ship_construction(ctx));
    assert!(ai.player_mut(P0).build.slot(Slot::Ship).is_none());
}

// AI-MISC T3: no expedition ship, harbour costs in stock, a harbour with an
// idle own ship: the prepare-expedition command for that harbour.
// spec: AI-MISC-1.2 (AI-MISC T3)
#[test]
fn t3_an_idle_ship_at_a_harbour_prepares_an_expedition() {
    let mut world = harbour_world();
    world.debug_add_ship(P0, SEA);
    // The ship searches for a harbour and docks.
    let mut docked = false;
    for _ in 0..3000 {
        world.step();
        if world.ships()[0].vessel.harbour == Some(HARBOUR)
            && !world.ships()[0].vessel.walker.is_walking()
        {
            docked = true;
            break;
        }
    }
    assert!(docked, "the ship did not reach the harbour");
    let mut ai = start(&world);
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 10), (PLANK, 10)], &[]);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.expedition(ctx));
    assert_eq!(
        commands,
        [Command::ExpeditionStart {
            player: P0,
            at: HARBOUR
        }]
    );
    // Without the harbour's costs in stock nothing is done.
    ai.player_mut(P0).econ = Economy::from_counts(&[(STONE, 1)], &[]);
    let (_, commands) = ai.with_ctx(&world, P0, |p, ctx| p.expedition(ctx));
    assert!(commands.is_empty());
}

fn waiting_vessel() -> Vessel {
    let mut v = Vessel::new(SEA);
    v.launched = true;
    v.expedition = true;
    v.state = s2_sim::ShipState::Waiting;
    v
}

// AI-MISC T4: the expedition ship stands still, a valid landing spot is within
// 5 fields: the build-harbour command.
// spec: AI-MISC-1.3 (AI-MISC T4)
#[test]
fn t4_the_ship_builds_a_harbour_near_a_valid_spot() {
    let mut v = waiting_vessel();
    v.target = Some(Landing {
        spot: (36, 22),
        field: (35, 22),
    });
    assert_eq!(
        sail_decision(&v, (33, 22), |_| true, |_| 0),
        Some(Sail::Build)
    );
    // The spot is not valid: it sails on instead (to an option, if any).
    assert_eq!(sail_decision(&v, (33, 22), |_| false, |_| 0), None);
    // Too far (distance 5 is not within): sails on.
    v.options[2] = Some(Landing {
        spot: (50, 22),
        field: (49, 22),
    });
    assert_eq!(
        sail_decision(&v, (30, 22), |_| true, |_| 0),
        Some(Sail::Go(2))
    );
}

// AI-MISC T5: no spot, landing options in 2 of 6 directions: one of the two
// is chosen with probability 1/2 each.
// spec: AI-MISC-1.3 (AI-MISC T5)
#[test]
fn t5_the_direction_is_chosen_at_random_among_the_options() {
    let mut v = waiting_vessel();
    for d in [1usize, 4] {
        v.options[d] = Some(Landing {
            spot: (50, 22),
            field: (49, 22),
        });
    }
    assert_eq!(
        sail_decision(
            &v,
            (33, 22),
            |_| true,
            |n| {
                assert_eq!(n, 2);
                0
            }
        ),
        Some(Sail::Go(1))
    );
    assert_eq!(
        sail_decision(&v, (33, 22), |_| true, |_| 1),
        Some(Sail::Go(4))
    );
    // A ship that is still sailing or has no options is left alone.
    let mut moving = waiting_vessel();
    moving.state = s2_sim::ShipState::Sailing;
    assert_eq!(sail_decision(&moving, (33, 22), |_| true, |_| 0), None);
    assert_eq!(
        sail_decision(&waiting_vessel(), (33, 22), |_| true, |_| 0),
        None
    );
    // Through the simulation's generator both turn up.
    let mut rng = s2_sim::Rng::new(5);
    let mut seen = [false; 6];
    for _ in 0..40 {
        if let Some(Sail::Go(d)) = sail_decision(&v, (33, 22), |_| true, |n| rng.draw() % n) {
            seen[usize::from(d)] = true;
        }
    }
    assert!(seen[1] && seen[4]);
}
