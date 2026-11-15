//! `s2-app soak MAP TICKS [--seed S] [--ai ...] [--out DIR] [--replay FILE]
//! [--snapshot FILE]`: computer players against each other, with every
//! invariant of `s2_sim::invariants` checked after every tick and no panic
//! allowed.
//!
//! A failure prints its tick and writes a replay file that reproduces the
//! game up to that tick (`s2-app soak --replay FILE` checks it again,
//! `s2-app replay FILE` plays it), and a game snapshot of the tick before
//! (`docs/snapshot.md`): `s2-app soak --snapshot FILE 1` runs the failing
//! tick again at once. The snapshot is made from the last checkpoint (one
//! every [`CHECKPOINT`] ticks, kept in memory) played on to that tick. The
//! exit code is 0 when all ticks passed, 1 for broken invariants and 3 for
//! a panic.

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::process::ExitCode;

use s2_sim::invariants::Checker;

use crate::game::Start;

/// Reports per invariant before the rest are only counted.
const SHOWN: usize = 5;

/// Ticks between the in-memory snapshots a failure's snapshot is made from.
pub const CHECKPOINT: u64 = 2_000;

pub fn run(args: &[String]) -> ExitCode {
    match soak(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("soak: {e}");
            ExitCode::from(2)
        }
    }
}

fn soak(args: &[String]) -> Result<ExitCode, String> {
    let mut out_dir = PathBuf::from("target/soak");
    let mut replay = None;
    let mut snapshot = None;
    let mut save_file = None;
    let mut positional = Vec::new();
    let mut start = Start::new("", s2_sim::rng::SIM_SEED as u64);
    let rest = start.options(args)?;
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => out_dir = PathBuf::from(it.next().ok_or("--out wants a folder")?),
            "--replay" => replay = Some(it.next().ok_or("--replay wants a file")?.clone()),
            "--snapshot" => snapshot = Some(it.next().ok_or("--snapshot wants a file")?.clone()),
            "--save" => save_file = Some(it.next().ok_or("--save wants a file")?.clone()),
            _ if arg.starts_with("--") => return Err(format!("unknown argument {arg}")),
            _ => positional.push(arg.clone()),
        }
    }
    let number = |text: &String| -> Result<u64, String> {
        text.parse().map_err(|_| format!("not a number: {text}"))
    };
    let (mut runner, ticks) = if let Some(file) = &replay {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        let (from_file, tick) = Start::parse_replay(&text)?;
        start = from_file;
        (start.begin()?, tick)
    } else if let Some(file) = &snapshot {
        // The ticks after the snapshot's (default 1: the tick after it).
        let ticks = match positional.as_slice() {
            [] => 1,
            [ticks] => number(ticks)?,
            _ => return Err("soak --snapshot FILE wants at most a tick count".into()),
        };
        let (from_file, runner) = crate::game::load_game(file)?;
        start = from_file;
        (runner, ticks)
    } else if let Some(file) = &save_file {
        // A save of the game program, of this or an earlier format.
        let [ticks] = positional.as_slice() else {
            return Err("soak --save FILE wants a tick count".into());
        };
        let (from_file, runner) = crate::game::load_save(file)?;
        start = from_file;
        (runner, number(ticks)?)
    } else {
        let [map, ticks] = positional.as_slice() else {
            return Err("soak wants a map and a tick count (or --replay FILE)".into());
        };
        start.map = map.clone();
        (start.begin()?, number(ticks)?)
    };
    let mut checkpoint = crate::game::snapshot(&start, &runner);
    let mut checker = Checker::new(&runner.world);
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut first_failure: Option<u64> = None;
    let begun = std::time::Instant::now();
    let mut before_failure = None;
    for _ in 0..ticks {
        let tick = runner.world.tick() + 1;
        // A panic in the step is a failure like any other, with its tick
        // and replay file; the game cannot go on after it.
        if catch_unwind(AssertUnwindSafe(|| runner.step())).is_err() {
            let file = save(&out_dir, &start, tick)?;
            let before = save_before(&out_dir, &checkpoint, tick)?;
            eprintln!(
                "tick {tick}: panic; replay file {}, snapshot of tick {} {}",
                file.display(),
                tick - 1,
                before.display()
            );
            return Ok(ExitCode::from(3));
        }
        for v in checker.check(&runner.world) {
            let n = counts.entry(v.invariant).or_default();
            *n += 1;
            if first_failure.is_none() {
                first_failure = Some(v.tick);
                before_failure = Some(save_before(&out_dir, &checkpoint, v.tick)?);
            }
            if *n <= SHOWN {
                println!("tick {}: {}: {}", v.tick, v.invariant, v.detail);
            }
        }
        if tick % CHECKPOINT == 0 {
            checkpoint = crate::game::snapshot(&start, &runner);
        }
    }
    let seconds = begun.elapsed().as_secs_f64();
    let summary = runner.summary();
    let stats = &runner.stats;
    let placed = ["PlaceBuilding", "PlanBuilding"]
        .iter()
        .map(|kind| stats.issued.get(*kind).copied().unwrap_or(0))
        .sum::<usize>();
    let roads = ["BuildRoad", "PlanRoad"]
        .iter()
        .map(|kind| stats.issued.get(*kind).copied().unwrap_or(0))
        .sum::<usize>();
    println!(
        "{} {} ticks (seed {}) in {seconds:.1} s: {placed} buildings and {roads} roads ordered, \
         {} buildings, {} settlers, {} wares standing at the end",
        start.map,
        ticks,
        start.seed,
        runner.world.buildings().count(),
        runner.world.settlers().count() + runner.world.carriers().len(),
        runner.world.wares().count(),
    );
    // UI-GAME-8.5e, 8.5a: what the statistics counted (an older save's from
    // its load: docs/snapshot.md).
    let (mut counted, mut coins, mut frags) = (0u64, 0u64, 0u64);
    for (player, _) in s2_ai::every_player(&runner.world) {
        if let Some(s) = runner.world.statistics(player) {
            counted += s
                .production
                .iter()
                .flatten()
                .map(|&n| u64::from(n))
                .sum::<u64>();
            coins += u64::from(s.coins);
            frags += u64::from(s.frags);
        }
    }
    println!("statistics: {counted} production counts, {coins} coins, {frags} frags");
    if std::env::var_os("S2_SOAK_SUMMARY").is_some() {
        print!("{summary}");
    }
    match first_failure {
        None => {
            println!("all invariants held");
            Ok(ExitCode::SUCCESS)
        }
        Some(tick) => {
            let file = save(&out_dir, &start, tick)?;
            for (name, n) in &counts {
                println!("{name}: {n} failures");
            }
            println!(
                "first failure in tick {tick}; replay file {} (s2-app soak --replay FILE)",
                file.display()
            );
            if let Some(before) = before_failure {
                println!(
                    "snapshot of tick {} {} (s2-app soak --snapshot FILE 1)",
                    tick - 1,
                    before.display()
                );
            }
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Write the game snapshot of the tick before `tick`: the checkpoint loaded
/// and played on to it.
fn save_before(dir: &std::path::Path, checkpoint: &[u8], tick: u64) -> Result<PathBuf, String> {
    let (start, mut runner) = crate::game::load_snapshot(checkpoint)?;
    while runner.world.tick() + 1 < tick {
        runner.step();
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(format!(
        "{}-seed{}-tick{}.s2snap",
        file_name(&start.map),
        start.seed,
        runner.world.tick()
    ));
    std::fs::write(&file, crate::game::snapshot(&start, &runner))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(file)
}

fn file_name(map: &str) -> String {
    map.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Write the replay file that plays the game to `tick`.
fn save(dir: &std::path::Path, start: &Start, tick: u64) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(format!(
        "{}-seed{}-tick{tick}.replay",
        file_name(&start.map),
        start.seed
    ));
    std::fs::write(&file, start.replay_text(tick))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A failure in tick 1234 writes the snapshot of tick 1233, made from
    // the checkpoint at tick 1000: it holds the game never saved.
    // spec: none (the soak's failure snapshot, docs/snapshot.md)
    #[test]
    fn the_snapshot_before_a_failure_is_that_tick() {
        let start = Start::new("fixture:town2", 3);
        let mut runner = start.begin().unwrap();
        while runner.world.tick() < 1000 {
            runner.step();
        }
        let checkpoint = crate::game::snapshot(&start, &runner);
        while runner.world.tick() < 1233 {
            runner.step();
        }
        let dir = std::env::temp_dir().join(format!("s2-soak-{}", std::process::id()));
        let file = save_before(&dir, &checkpoint, 1234).unwrap();
        assert!(file
            .to_string_lossy()
            .ends_with("fixture_town2-seed3-tick1233.s2snap"));
        let (_, loaded) = crate::game::load_game(file.to_str().unwrap()).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(loaded.world.tick(), 1233);
        assert_eq!(loaded.world.state_hash(), runner.world.state_hash());
        assert_eq!(loaded.ai.state_hash(), runner.ai.state_hash());
    }
}
