//! Computer players on a real map with the original rules (`S2_GAME_DIR`;
//! skipped when unset). The long runs are ignored by default:
//! `cargo test --release -p s2-ai --test headless -- --ignored --nocapture`.

// The long runs report their own speed.
#![allow(clippy::disallowed_types)]

use s2_ai::run::Runner;
use s2_ai::{AiConfig, AiSystem};
use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::PlayerId;

fn game(seed: u64, slots: &[(u8, u8)]) -> Option<(Runner, Vec<PlayerId>)> {
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
    let mut world = s2_rules::new_game(seed, &map, &rules, 1).expect("game");
    let config = AiConfig::new(&rules, &rules.sim_rules().expect("sim rules"));
    let mut ai = AiSystem::new(config, &world);
    let slots: Vec<(PlayerId, u8)> = slots.iter().map(|&(s, p)| (PlayerId(s), p)).collect();
    ai.activate_all(&mut world, &slots);
    let names = rules.names.iter().map(|(n, &id)| (id, n.clone())).collect();
    let players = slots.iter().map(|&(p, _)| p).collect();
    Some((Runner::new(world, ai, names), players))
}

// A computer player alone builds, joins its buildings with roads and its
// commands are accepted (a road asked for in the same tick as another may
// find no route).
// spec: AI-3.3, AI-3.4, AI-CON-8.1
#[test]
fn a_lone_computer_player_builds_up() {
    let Some((mut runner, players)) = game(1, &[(0, 2)]) else {
        return;
    };
    let start = runner.world.owned_field_count(players[0]);
    runner.run(4_000, 4_000, |_| {});
    let placed = runner
        .stats
        .issued
        .get("PlanBuilding")
        .copied()
        .unwrap_or(0);
    assert!(placed >= 10, "{placed} buildings placed");
    assert!(runner.stats.issued.get("PlanRoad").copied().unwrap_or(0) >= 5);
    // The road builder refuses a road longer than 2 x distance
    // (AI-CON-8.1 b) after its stub and evening (TRN-ROAD-3.3a).
    for reason in runner.stats.rejected.keys() {
        assert!(reason.starts_with("PlanRoad"), "{reason}");
    }
    let roads = runner
        .world
        .roads()
        .filter(|r| r.owner == players[0])
        .count();
    assert!(roads >= 5, "{roads} roads");
    assert!(runner.world.owned_field_count(players[0]) >= start);
    print!("{}", runner.describe(&players));
}

// The same seed gives the same game, with the AI on.
// spec: none (determinism of computer players on a real map)
#[test]
fn a_game_with_computer_players_is_deterministic() {
    let run = || {
        let (mut runner, _) = game(3, &[(0, 2), (1, 3)]).expect("S2_GAME_DIR");
        runner.run(5_000, 5_000, |_| {});
        (runner.world.state_hash(), runner.stats.issued.clone())
    };
    if GameDir::from_env().is_err() {
        return;
    }
    assert_eq!(run(), run());
}

// One computer player alone for 100,000 ticks, reported at
// 10,000, 50,000 and 100,000.
// spec: AI-3.2, AI-3.3, AI-3.5
#[test]
#[ignore = "100,000 ticks: run with --release -- --ignored --nocapture"]
fn one_computer_player_alone_for_100_000_ticks() {
    let Some((mut runner, players)) = game(1, &[(0, 2)]) else {
        return;
    };
    let start = std::time::Instant::now();
    let start_fields = runner.world.owned_field_count(players[0]);
    for target in [10_000u64, 50_000, 100_000] {
        let ticks = target - runner.world.tick();
        runner.run(ticks, ticks, |_| {});
        print!("{}", runner.describe(&players));
    }
    print!("{}", runner.summary());
    let seconds = start.elapsed().as_secs_f64();
    println!(
        "100000 ticks in {seconds:.1} s: {:.0} ticks per second",
        100_000.0 / seconds
    );
    assert!(runner.world.owned_field_count(players[0]) > start_fields);
    assert!(runner.stats.attacks.is_empty(), "nobody to attack");
}

// Computer against computer for 200,000 ticks.
// spec: AI-3.5, AI-MIL-5.2
#[test]
#[ignore = "200,000 ticks: run with --release -- --ignored --nocapture"]
fn computer_against_computer_for_200_000_ticks() {
    let Some((mut runner, players)) = game(1, &[(0, 2), (1, 3)]) else {
        return;
    };
    let start = std::time::Instant::now();
    runner.run(200_000, 50_000, |r| print!("{}", r.describe(&players)));
    print!("{}", runner.summary());
    let seconds = start.elapsed().as_secs_f64();
    println!(
        "200000 ticks in {seconds:.1} s: {:.0} ticks per second",
        200_000.0 / seconds
    );
}

// The attack part in a game: the two players first expand towards each other
// with the scripted scenario until their land meets, then computer players
// take over (profiles normal and strong). Nothing is asserted about who
// wins; the run is reported.
// spec: AI-MIL-5.4, MIL-ATK-1.5
#[test]
#[ignore = "about 160,000 ticks: run with --release -- --ignored --nocapture"]
fn computer_players_that_meet_fight_over_the_border() {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let bytes = std::fs::read(dir.root().join(rel)).expect("map file");
    let map = s2m::load(rel, &bytes).expect("map");
    let mut world = s2_rules::new_game(7, &map, &rules, 3).expect("game");
    s2_rules::scenario::expand_towards_each_other(&mut world, &rules, 100_000);
    assert!(
        world.have_met(PlayerId(0), PlayerId(1)),
        "the lands did not meet"
    );
    let config = AiConfig::new(&rules, &rules.sim_rules().expect("sim rules"));
    let mut ai = AiSystem::new(config, &world);
    let slots = [(PlayerId(0), 2), (PlayerId(1), 3)];
    ai.activate_all(&mut world, &slots);
    let names = rules.names.iter().map(|(n, &id)| (id, n.clone())).collect();
    let mut runner = Runner::new(world, ai, names);
    let players = [PlayerId(0), PlayerId(1)];
    runner.run(60_000, 20_000, |r| print!("{}", r.describe(&players)));
    print!("{}", runner.summary());
}
