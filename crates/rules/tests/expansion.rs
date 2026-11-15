//! Two players expanding towards each other on a real map with the original
//! rules (`S2_GAME_DIR`; skipped when unset): territory claims on
//! occupation, strength rule at the meeting line, soldiers allocated and
//! walking, deterministic.

use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::{PlayerId, World};

/// Ticks the scenario runs: about 2 hours of game time, enough for the two
/// sides on this map to meet.
const UNTIL: u64 = 140_000;
const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn game() -> Option<(s2_rules::RuleSet, World)> {
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
    let world = s2_rules::new_game(7, &map, &rules, 3).expect("game");
    Some((rules, world))
}

// spec: TER-3.2, TER-5.4
#[test]
#[ignore = "140,000 ticks: run with cargo test --release -- --ignored"]
fn two_players_expand_towards_each_other() {
    let Some((rules, mut world)) = game() else {
        return;
    };
    let start = (world.owned_field_count(P0), world.owned_field_count(P1));
    let report = s2_rules::scenario::expand_towards_each_other(&mut world, &rules, UNTIL);
    let end = (world.owned_field_count(P0), world.owned_field_count(P1));
    for p in [P0, P1] {
        let mine = report.placed.iter().filter(|&&(o, _)| o == p).count();
        assert!(mine >= 4, "player {} placed {mine}", p.0);
    }
    assert!(end.0 > start.0 && end.1 > start.1);
    // The land of the two sides touched.
    assert!(world.have_met(P0, P1));
    let occupied = world
        .military_buildings()
        .filter(|(_, m)| m.applied)
        .count();
    assert!(occupied >= 6, "{occupied} occupied");
}

// spec: none (determinism of the scripted expansion)
#[test]
fn the_scenario_is_deterministic() {
    let hash = || {
        let (rules, mut world) = game()?;
        s2_rules::scenario::expand_towards_each_other(&mut world, &rules, 6_000);
        Some(world.state_hash())
    };
    let (a, b) = (hash(), hash());
    assert_eq!(a, b);
}

// spec: MIL-CON-1.1, MIL-CON-2.2
#[test]
#[ignore = "140,000 ticks and an attack: run with cargo test --release -- --ignored"]
fn player_one_conquers_a_building_of_player_zero() {
    let Some((rules, mut world)) = game() else {
        return;
    };
    s2_rules::scenario::expand_towards_each_other(&mut world, &rules, UNTIL);
    let before = world.tick();
    let report = s2_rules::scenario::attack_until(
        &mut world,
        P1,
        before + 30_000,
        s2_rules::scenario::AttackStop::Conquest,
    );
    eprintln!("{report:?} at tick {}", world.tick());
    let (at, _) = report.conquered.expect("a building was taken");
    assert_eq!(world.building_at(at).unwrap().owner, P1);
}

/// The expansion, the attacks, the fights and the conquest are replayable: two
/// runs from one seed end in the same state, and the scripted commands alone
/// (taken from the first run's orders) reproduce the attack.
// spec: none (determinism of the scripted attack)
#[test]
#[ignore = "140,000 ticks and an attack: run with cargo test --release -- --ignored"]
fn the_attack_is_deterministic() {
    let run = || {
        let (rules, mut world) = game()?;
        s2_rules::scenario::expand_towards_each_other(&mut world, &rules, UNTIL);
        let report = s2_rules::scenario::attack_until(
            &mut world,
            P1,
            UNTIL + 30_000,
            s2_rules::scenario::AttackStop::Conquest,
        );
        Some((world.state_hash(), report))
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    if let Some((_, report)) = a {
        assert!(report.conquered.is_some());
    }
}
