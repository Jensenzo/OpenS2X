//! Determinism of the simulation on a real map (`tutorial01`), run on the
//! player's own install through `S2_GAME_DIR`; skipped when it is unset.
//!
//! The same seed and the same command stream give the same state in every
//! tick (`docs/architecture.md`): two runs of the same script, and a replay
//! from the command log alone, hash identically every 1,000 ticks.

use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::grid::{self, Point};
use s2_sim::{Command, Placement, PlayerId, World};

const T01: &str = "data/maps/tutorialMaps/tutorial01.s2m";
const TICKS: u64 = 20_000;
const EVERY: u64 = 1_000;

fn game_dir() -> Option<GameDir> {
    match GameDir::from_env() {
        Ok(dir) => Some(dir),
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn game_on(rel: &str, seed: u64) -> Option<(World, s2_rules::RuleSet, s2m::Map)> {
    let dir = game_dir()?;
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let data = std::fs::read(dir.root().join(rel)).unwrap();
    let map = s2m::load(rel, &data).unwrap();
    let world = s2_rules::new_game(seed, &map, &rules, 0).unwrap();
    Some((world, rules, map))
}

fn game(seed: u64) -> Option<(World, s2_rules::RuleSet, s2m::Map)> {
    game_on(T01, seed)
}

/// The commands of a small economy, each with the tick it is submitted in:
/// a woodcutter hut near trees and a sawmill, both roaded to the
/// headquarters, a second hut and a stone pit later.
fn script(world: &World, rules: &s2_rules::RuleSet, map: &s2m::Map) -> Vec<(u64, Command)> {
    let hq = (map.info.start_positions[0].x, map.info.start_positions[0].y);
    let player = PlayerId(0);
    let hq_flag = world.terrain().neighbour(hq, 1).unwrap();
    let mut trial = world.clone();
    let mut commands = Vec::new();
    let mut taken: Vec<Point> = Vec::new();
    for (kind, min, start) in [
        ("woodCutter", Placement::Small, 1u64),
        ("sawMill", Placement::Medium, 5),
        ("woodCutter", Placement::Small, 400),
        ("stonePit", Placement::Small, 800),
    ] {
        let id = rules.id_named(kind).unwrap();
        let found = (3..=8)
            .flat_map(|k| grid::ring(hq, k, map.info.width, map.info.height))
            .filter(|f| !taken.contains(f))
            .find_map(|f| {
                let class = trial.placement_at(f);
                if class < min || class == Placement::Mine {
                    return None;
                }
                let flag = trial.terrain().neighbour(f, 1)?;
                let mut probe = trial.clone();
                for command in [
                    Command::PlaceBuilding {
                        player,
                        building: id,
                        at: f,
                    },
                    Command::BuildRoad {
                        player,
                        from: hq_flag,
                        to: flag,
                    },
                ] {
                    probe.submit(command);
                    probe.step();
                    probe.last_results()[0].outcome.ok()?;
                }
                Some((f, flag, probe))
            });
        let (f, flag, probe) = found.unwrap_or_else(|| panic!("no place for {kind}"));
        taken.push(f);
        trial = probe;
        commands.push((
            start,
            Command::PlaceBuilding {
                player,
                building: id,
                at: f,
            },
        ));
        commands.push((
            start + 1,
            Command::BuildRoad {
                player,
                from: hq_flag,
                to: flag,
            },
        ));
    }
    commands
}

/// Run `ticks` ticks submitting the (tick, command) list, returning the
/// state hash after every `EVERY` ticks and the final world.
fn run(mut world: World, log: &[(u64, Command)], ticks: u64) -> (Vec<u64>, World) {
    let mut hashes = Vec::new();
    let mut next = 0;
    for tick in 1..=ticks {
        while next < log.len() && log[next].0 <= tick {
            world.submit(log[next].1.clone());
            next += 1;
        }
        world.step();
        if tick % EVERY == 0 {
            hashes.push(world.state_hash());
        }
    }
    (hashes, world)
}

// spec: none (determinism and replay on a real map)
#[test]
fn same_seed_same_state_and_replay_matches() {
    let Some((world, rules, map)) = game(7) else {
        return;
    };
    let commands = script(&world, &rules, &map);
    // The command log of the first run: what was submitted, and when.
    let (first, first_world) = run(world, &commands, TICKS);
    assert_eq!(first.len() as u64, TICKS / EVERY);

    // A second game with the same seed and the same commands.
    let again = game(7).unwrap().0;
    let (second, second_world) = run(again, &commands, TICKS);
    assert_eq!(first, second, "two runs of the same seed");
    assert_eq!(first_world, second_world);

    // A replay from the log alone, in a fresh game (no saved state).
    let fresh = game(7).unwrap().0;
    let log = commands.clone();
    let (replayed, replay_world) = run(fresh, &log, TICKS);
    assert_eq!(first, replayed, "replay from the command log");
    assert_eq!(first_world, replay_world);

    // The economy really ran: buildings were finished and wares moved.
    assert!(
        first.windows(2).any(|w| w[0] != w[1]),
        "the state changes over time"
    );
    assert!(first_world.sites().count() < 4, "some site was finished");

    // A different seed gives a different state (the generator is part of it).
    let other = game(8).unwrap().0;
    let (different, _) = run(other, &commands, TICKS);
    assert_ne!(first.last(), different.last());
}

/// Ticks per second in release mode, on `tutorial01` and on the largest
/// map of the install, each with the small economy of `script` running:
/// `cargo test --release -p s2-rules --test determinism -- --ignored
/// --nocapture`.
// spec: none (a speed measurement)
#[test]
#[ignore = "a measurement, run in release mode"]
fn ticks_per_second() {
    use std::time::Instant;
    let Some(dir) = game_dir() else {
        return;
    };
    let mut largest = (0u64, String::new());
    for sub in ["campaignMaps", "freeGameMaps", "tutorialMaps"] {
        let folder = dir.root().join("data/maps").join(sub);
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "s2m") {
                let rel = format!(
                    "data/maps/{sub}/{}",
                    path.file_name().unwrap().to_string_lossy()
                );
                let data = std::fs::read(&path).unwrap();
                if let Ok(map) = s2m::load(&rel, &data) {
                    let size = u64::from(map.info.width) * u64::from(map.info.height);
                    if size > largest.0 {
                        largest = (size, rel);
                    }
                }
            }
        }
    }
    for rel in [T01.to_string(), largest.1.clone()] {
        let (world, rules, map) = game_on(&rel, 1).unwrap();
        let commands = script(&world, &rules, &map);
        let ticks = 20_000;
        let started = Instant::now();
        let (_, end) = run_plain(world, &commands, ticks);
        let seconds = started.elapsed().as_secs_f64();
        println!(
            "{rel}: {}x{} fields, {ticks} ticks in {seconds:.2} s = {:.0} ticks/s ({} sites left)",
            map.info.width,
            map.info.height,
            ticks as f64 / seconds,
            end.sites().count()
        );
    }
}

/// [`run`] without the state hashes.
fn run_plain(mut world: World, log: &[(u64, Command)], ticks: u64) -> ((), World) {
    let mut next = 0;
    for tick in 1..=ticks {
        while next < log.len() && log[next].0 <= tick {
            world.submit(log[next].1.clone());
            next += 1;
        }
        world.step();
    }
    ((), world)
}
