//! The game snapshot (`docs/snapshot.md`) as a property: saved at any tick,
//! loaded and continued, a game has the hashes (world and computer players)
//! of the game that was never saved. The save ticks are
//! spread over the game by a fixed pseudo-random sequence. The fixtures
//! always run; three maps of the install with `S2_GAME_DIR`.

use s2_ai::run::Runner;

use crate::game::{restore, snapshot, Start};

/// The world's and the computer players' hashes.
fn hashes(runner: &Runner) -> (u64, u64) {
    (runner.world.state_hash(), runner.ai.state_hash())
}

/// Play `map` for `ticks` ticks, saving at `saves` ticks chosen by a fixed
/// sequence; load each save into one other runner (which must replace all
/// its state) and continue it `ahead` ticks, comparing the hashes at the
/// load, one tick later and at the end.
fn continues_as_never_saved(map: &str, ticks: u64, saves: usize, ahead: u64) {
    let start = Start::new(map, s2_sim::rng::SIM_SEED as u64);
    let mut runner = start.begin().expect("game");
    let mut at: Vec<u64> = Vec::new();
    // A linear congruential sequence: the same save ticks every run.
    let mut x: u64 = 0x2545_f491;
    while at.len() < saves {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let tick = (x >> 33) % (ticks - ahead);
        if !at.contains(&tick) {
            at.push(tick);
        }
    }
    at.sort_unstable();
    // The straight game's hashes where the loaded games are compared: at
    // the load, one tick on and `ahead` ticks on (a hash of every tick
    // costs more than the game itself in a debug build; a game that parted
    // would not meet again).
    let checked = |s: u64| [s, s + 1, s + ahead];
    let needed = |t: u64| at.iter().any(|&s| checked(s).contains(&t));
    let mut straight = std::collections::BTreeMap::new();
    let mut saved = Vec::new();
    while runner.world.tick() < ticks {
        let t = runner.world.tick();
        if at.binary_search(&t).is_ok() {
            saved.push(snapshot(&start, &runner));
        }
        if needed(t) {
            straight.insert(t, hashes(&runner));
        }
        runner.step();
    }
    let mut loaded = start.begin().expect("game");
    for (bytes, &tick) in saved.iter().zip(&at) {
        let from = restore(&mut loaded, bytes).expect("snapshot loads");
        assert_eq!(from, start);
        assert_eq!(loaded.world.tick(), tick, "{map}");
        assert_eq!(hashes(&loaded), straight[&tick], "{map}: loaded at {tick}");
        assert!(
            &snapshot(&start, &loaded) == bytes,
            "{map}: saved again at {tick}"
        );
        for _ in 0..ahead {
            loaded.step();
            let t = loaded.world.tick();
            if checked(tick).contains(&t) {
                assert_eq!(
                    hashes(&loaded),
                    straight[&t],
                    "{map}: saved at {tick}, continued to {t}"
                );
            }
        }
    }
}

// spec: none (our own game snapshot, docs/snapshot.md)
#[test]
fn fixtures_continue_as_never_saved() {
    for map in ["fixture:flat2", "fixture:town2", "fixture:town4"] {
        continues_as_never_saved(map, 3_000, 25, 40);
    }
}

// spec: none (our own game snapshot, docs/snapshot.md)
#[test]
fn maps_continue_as_never_saved() {
    if std::env::var_os("S2_GAME_DIR").is_none() {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    }
    for map in [
        "MP_2P_Storm_Coast",
        "MP_4P_crossroads",
        "MP_6P_Paradise_Island",
    ] {
        continues_as_never_saved(map, 6_000, 25, 40);
    }
}

// The long run of the property: `cargo test --release -p s2-app
// long_maps_continue -- --ignored`.
// spec: none (our own game snapshot, docs/snapshot.md)
#[test]
#[ignore]
fn long_maps_continue_as_never_saved() {
    if std::env::var_os("S2_GAME_DIR").is_none() {
        return;
    }
    for map in [
        "MP_2P_Storm_Coast",
        "MP_4P_crossroads",
        "MP_6P_Paradise_Island",
    ] {
        continues_as_never_saved(map, 60_000, 100, 200);
    }
}

// spec: none (our own game snapshot, docs/snapshot.md)
#[test]
fn other_files_are_refused() {
    let start = Start::new("fixture:flat2", 1);
    let mut runner = start.begin().unwrap();
    let bytes = snapshot(&start, &runner);
    assert!(crate::game::is_snapshot(&bytes));
    assert!(restore(&mut runner, &bytes[..bytes.len() - 1]).is_err());
    assert!(restore(&mut runner, b"s2-replay 1\n").is_err());
    let mut newer = bytes.clone();
    newer[8] = 99;
    assert!(restore(&mut runner, &newer)
        .unwrap_err()
        .contains("version 99"));
    assert!(restore(&mut runner, &bytes).is_ok());
}

// spec: none (our own save format, docs/snapshot.md: soak --save runs a save of the game program headless)
#[test]
fn a_fixture_save_runs_headless() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/saves/save1-world65.s2save"
    );
    let (start, mut runner) = crate::game::load_save(file).expect("the fixture save");
    assert_eq!(start.map, "fixture:town4");
    assert_eq!(runner.world.tick(), 9_000);
    runner.step();
    assert_eq!(runner.world.tick(), 9_001);
    // Not a save: refused with its reason.
    let not = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let Err(e) = crate::game::load_save(not) else {
        panic!("a file that is no save loads");
    };
    assert!(e.contains("not a save of s2-reborn"), "{e}");
}
