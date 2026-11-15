//! Helpers of the unit tests on top of [`crate::fixture`].

use s2_sim::grid::Point;
use s2_sim::map::state;
use s2_sim::{GameSetup, PlayerId, World};

pub use crate::fixture::*;

/// The player-0 world with a building of `kind` placed at `at` (a
/// construction site).
pub fn with_building(world: &mut World, kind: u32, at: Point) {
    world.submit(s2_sim::Command::PlaceBuilding {
        player: PlayerId(0),
        building: kind,
        at,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()), "{at:?}");
}

/// Finish the construction site on `at` in the simulation (its materials
/// and workers are put there and the world runs until it is built).
pub fn finish_site(world: &mut World, at: Point) {
    let shortfall = world.site(at).expect("a site").shortfall();
    world.submit(s2_sim::Command::DebugSupplySite {
        player: PlayerId(0),
        at,
        wares: shortfall,
        workers: true,
    });
    for _ in 0..5000 {
        world.step();
        if world.site(at).is_none() {
            return;
        }
    }
    panic!("the site on {at:?} was not finished");
}

/// The snapshot with the building on `at` finished, working and with the
/// given productivity.
pub fn finished_snapshot(world: &World, at: Point, productivity: i32) -> crate::snapshot::Snapshot {
    let mut snapshot = crate::snapshot::Snapshot::take(world);
    for b in snapshot.buildings[0].iter_mut().filter(|b| b.at == at) {
        b.under_construction = false;
        b.active = true;
        b.workers = 1;
        b.productivity = productivity;
        // ECO-OUT-3.11 (T24): the float32 value the computer player reads.
        b.productivity_bits = if productivity < 0 {
            (-1.0f32).to_bits()
        } else {
            (productivity as f32 / 100.0).to_bits()
        };
    }
    // Its worker at home (AI-CON-9.2).
    snapshot.at_home.insert(at);
    snapshot
}

/// Finish the building on `at` with the debug supply command and steps.
pub fn finish_building(world: &mut World, at: Point, wares: Vec<(u32, u32)>) {
    world.submit(s2_sim::Command::DebugSupplySite {
        player: PlayerId(0),
        at,
        wares,
        workers: true,
    });
    for _ in 0..4000 {
        world.step();
        if world.building_at(at).is_some_and(|b| !b.under_construction) {
            return;
        }
    }
    panic!("building on {at:?} not finished");
}

/// A flat 96 x 96 map with a sea at x = 28..=58 and harbour ground on its
/// west shore (x = 21..=27), one headquarters at (20, 20) and the harbour spot
/// (25, 22) listed.
pub fn sea_world() -> World {
    let mut terrain = flat(96, 96);
    for field in terrain.fields().collect::<Vec<_>>() {
        let (x, _) = field;
        let index = terrain.index(field);
        if (28..=58).contains(&x) {
            terrain.states[index] = state::WATER;
        } else if (21..=27).contains(&x) {
            terrain.states[index] = state::FOR_BUILDING | state::FOR_HARBOR;
        }
    }
    let setup = GameSetup {
        players: vec![(PlayerId(0), (20, 20))],
        local_player: PlayerId(0),
        harbour_spots: vec![(26, 22)],
        ..GameSetup::default()
    };
    World::new_game(1, terrain, rules(), &setup)
}
