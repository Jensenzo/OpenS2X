//! Object numbers in the recorded game (SIM-START-4, T3, T6): the default
//! set-up of MP_2P_Storm_Coast with the computer player in slot 1, on the
//! real rules and map (`S2_GAME_DIR`; skipped when unset).

use std::collections::BTreeMap;

use s2_ai::{AiConfig, AiSystem};
use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::{PlayerId, World};

const P1: PlayerId = PlayerId(1);

/// The recorded game after its start sequence.
fn recorded_game() -> Option<(World, AiSystem)> {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let bytes = std::fs::read(dir.root().join(rel)).expect("map file");
    let map = s2m::load(rel, &bytes).expect("map");
    let setup = s2_rules::game_setup(&map, &rules, 1);
    let sim = rules.sim_rules().expect("sim rules");
    let mut world = World::new_game(
        u64::from(s2_sim::rng::SIM_SEED),
        map.terrain(),
        sim.clone(),
        &setup,
    );
    let mut ai = AiSystem::new(AiConfig::new(&rules, &sim), &world);
    // UI-SETUP-2.5: strength 0, profile 1 (Weak; Q-275).
    ai.activate_all(&mut world, &[(P1, 1)]);
    Some((world, ai))
}

/// Every building's and settler's object number with the game counter of
/// the tick it first appears in, to the end of counter tick `last`.
fn first_numbers(world: &mut World, ai: &mut AiSystem, last: u64) -> BTreeMap<u32, u64> {
    let mut seen = BTreeMap::new();
    while world.tick() <= last {
        let counter = world.tick();
        ai.step(world);
        for b in world.buildings() {
            seen.entry(b.serial).or_insert(counter);
        }
        for s in world.trace_settlers() {
            let number = s.id.parse().expect("an object number");
            seen.entry(number).or_insert(counter);
        }
    }
    seen
}

// T3, T8: castle 0 < flag 0 < castle 1 < flag 1 < soldier 0 < soldier 1.
// The counter starts at the map file's next id (90 175 here; the 261
// animals are 89 914 .. 90 174 in the file); the computer player's cell of
// its castle takes the number between its flag and the soldiers (90 181 in
// the original, before the flag; no building or settler differs).
// spec: SIM-START-4.1, SIM-START-4.2 (T3), SIM-START-4.5 (T8), SIM-START-1.1
#[test]
fn t3_the_start_objects_take_the_original_numbers() {
    let Some((mut world, mut ai)) = recorded_game() else {
        return;
    };
    let mut castles: Vec<(PlayerId, u32)> =
        world.buildings().map(|b| (b.owner, b.serial)).collect();
    castles.sort();
    assert_eq!(castles, [(PlayerId(0), 90175), (P1, 90180)]);
    assert_eq!(world.next_object(), 90186);
    let seen = first_numbers(&mut world, &mut ai, 0);
    let soldiers: Vec<u32> = seen.keys().copied().filter(|&n| n > 90180).collect();
    assert_eq!(soldiers, [90186, 90187]);
}

// T6, T9, T10: the buildings and settlers of the first 63 numbers, each in
// the original's counter tick: the needs per good at 41, the scan's cells,
// the road stubs, the next-counter carrier requests, the donkey
// renumberings of new carriers, helper and ware, and the road removal of
// 386 put every one where the original has it.
// spec: SIM-START-4.1, SIM-START-4.3, SIM-START-4.4 (T6, T9, T10), TRN-CARRY-2.1
#[test]
fn t6_the_numbers_of_the_recorded_game() {
    let Some((mut world, mut ai)) = recorded_game() else {
        return;
    };
    let seen = first_numbers(&mut world, &mut ai, 447);
    let expected = [
        (90175, 0),
        (90180, 0),
        (90186, 0),
        (90187, 0),
        (90190, 127),
        (90202, 177),
        (90204, 207),
        (90207, 242),
        (90209, 275),
        (90211, 323),
        (90231, 387),
        (90234, 417),
        (90237, 447),
    ];
    let got: Vec<(u32, u64)> = seen
        .into_iter()
        .filter(|(n, _)| expected.iter().any(|(e, _)| e == n))
        .collect();
    assert_eq!(got, expected);
}
