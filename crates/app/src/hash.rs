//! `s2-app hash MAP TICKS [--seed S] [--every N] [--ai ...] [--golden FILE
//! [--bless]] [--save FILE] [--snapshot FILE]`: the state hash of a game of
//! computer players every N ticks; `--save` writes the replay file of its
//! last tick, `--snapshot` the game snapshot (`docs/snapshot.md`).
//!
//! Each line is `<tick> world <hash> ai <hash>` followed by the world's
//! hash per part (`World::state_hashes`), so two runs that differ show in
//! which tick and in which part they first do. With `--golden` the output
//! is compared with the file; `--bless` writes it instead. A golden file
//! starts with the line `# hash <map> <ticks> <options>`, which is how the
//! golden test (`crates/app/tests/golden.rs`) knows how to make it again.

use std::process::ExitCode;

use crate::game::Start;

pub fn run(args: &[String]) -> ExitCode {
    match hash(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("hash: {e}");
            ExitCode::from(2)
        }
    }
}

fn hash(args: &[String]) -> Result<ExitCode, String> {
    let (Some(map), Some(ticks)) = (args.first(), args.get(1)) else {
        return Err("hash wants a map and a tick count".into());
    };
    let ticks: u64 = ticks
        .parse()
        .map_err(|_| format!("not a number: {ticks}"))?;
    let mut start = Start::new(map, s2_sim::rng::SIM_SEED as u64);
    let rest = start.options(&args[2..])?;
    let (mut every, mut golden, mut bless) = (ticks.max(1), None, false);
    let mut save: Option<String> = None;
    let mut snapshot: Option<String> = None;
    let mut it = rest.iter();
    // The options that shape the output, for the golden file's first line.
    let mut shape = Vec::new();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--every" => {
                let v = it.next().ok_or("--every wants a number")?;
                every = v.parse().map_err(|_| format!("not a number: {v}"))?;
                shape.push(format!("--every {every}"));
            }
            "--golden" => golden = Some(it.next().ok_or("--golden wants a file")?.clone()),
            "--bless" => bless = true,
            "--save" => save = Some(it.next().ok_or("--save wants a file")?.clone()),
            "--snapshot" => {
                snapshot = Some(it.next().ok_or("--snapshot wants a file")?.clone());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if bless && golden.is_none() {
        return Err("--bless wants --golden FILE".into());
    }
    let mut header = format!("# hash {} {ticks} --seed {}", start.map, start.seed);
    if start.resources != 1 {
        header += &format!(" --resources {}", start.resources);
    }
    if start.slots.is_some() {
        header += &format!(" --ai {}", start.ai_text());
    }
    if !start.closed.is_empty() {
        header += &format!(" --closed {}", start.closed_text());
    }
    for s in shape {
        header += &format!(" {s}");
    }
    let mut runner = start.begin()?;
    let out = lines(&mut runner, ticks, every);
    if let Some(file) = &snapshot {
        std::fs::write(file, crate::game::snapshot(&start, &runner))
            .map_err(|e| format!("{file}: {e}"))?;
    }
    if let Some(save) = &save {
        // A game of computer players is decided by its start: the replay
        // file is its save (docs/trace-format.md).
        std::fs::write(save, start.replay_text(ticks)).map_err(|e| format!("{save}: {e}"))?;
    }
    let text = format!("{header}\n{}", out.join("\n")) + "\n";
    let Some(golden) = golden else {
        print!("{text}");
        return Ok(ExitCode::SUCCESS);
    };
    if bless {
        if let Some(parent) = std::path::Path::new(&golden).parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{golden}: {e}"))?;
        }
        std::fs::write(&golden, &text).map_err(|e| format!("{golden}: {e}"))?;
        println!("{golden}: blessed, {} lines", out.len());
        return Ok(ExitCode::SUCCESS);
    }
    let stored = std::fs::read_to_string(&golden).map_err(|e| format!("{golden}: {e}"))?;
    match first_difference(&stored, &text) {
        None => {
            println!("{golden}: same, {} lines", out.len());
            Ok(ExitCode::SUCCESS)
        }
        Some(report) => {
            eprintln!("{golden}: differs\n{report}");
            eprintln!(
                "if the change is intended, bless it with --bless and name the spec rule \
                 that caused it in the commit message"
            );
            Ok(ExitCode::FAILURE)
        }
    }
}

/// The output lines of a game played on for `ticks` ticks, one every
/// `every` ticks and one at the end.
pub fn lines(runner: &mut s2_ai::run::Runner, ticks: u64, every: u64) -> Vec<String> {
    let mut out = Vec::new();
    for done in 1..=ticks {
        runner.step();
        if done % every.max(1) == 0 || done == ticks {
            out.push(line(runner));
        }
    }
    out
}

/// One hash line for the game as it is now.
pub fn line(runner: &s2_ai::run::Runner) -> String {
    let world = &runner.world;
    let parts: Vec<String> = world
        .state_hashes()
        .iter()
        .map(|(name, h)| format!("{name} {h:016x}"))
        .collect();
    format!(
        "{} world {:016x} ai {:016x} {}",
        world.tick(),
        world.state_hash(),
        runner.ai.state_hash(),
        parts.join(" ")
    )
}

/// The first line in which two hash outputs differ, with the parts that do.
pub fn first_difference(expected: &str, actual: &str) -> Option<String> {
    let (e, a): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), actual.lines().collect());
    for i in 0..e.len().max(a.len()) {
        let (x, y) = (
            e.get(i).copied().unwrap_or(""),
            a.get(i).copied().unwrap_or(""),
        );
        if x == y {
            continue;
        }
        // After the tick, (name, hash) pairs.
        let words = |s: &str| -> Vec<String> { s.split(' ').skip(1).map(str::to_string).collect() };
        let (wx, wy) = (words(x), words(y));
        let parts: Vec<String> = wx
            .chunks(2)
            .zip(wy.chunks(2))
            .filter(|(p, q)| p != q && p.len() == 2)
            .map(|(p, _)| p[0].clone())
            .collect();
        return Some(format!(
            "line {}: expected\n  {x}\nfound\n  {y}\ndiffering parts: {}",
            i + 1,
            if parts.is_empty() {
                "(line count)".to_string()
            } else {
                parts.join(", ")
            }
        ));
    }
    None
}

/// `s2-app replay FILE [--to TICK] [--every N]`: play a replay file (a soak
/// failure's, a save) to its tick, or load a game snapshot
/// (`docs/snapshot.md`), and print the hash line there; with `--to`, go on
/// to that tick, printing a line every N ticks as `hash` does.
pub fn replay(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else {
        eprintln!("replay wants a replay file");
        return ExitCode::from(2);
    };
    let (mut to, mut every) = (None, None);
    let mut it = args[1..].iter();
    while let Some(arg) = it.next() {
        let value = it.next().and_then(|v| v.parse::<u64>().ok());
        match (arg.as_str(), value) {
            ("--to", Some(v)) => to = Some(v),
            ("--every", Some(v)) => every = Some(v),
            _ => {
                eprintln!("replay: unknown argument {arg} or missing number");
                return ExitCode::from(2);
            }
        }
    }
    let result = crate::game::load_game(file).map(|(_, runner)| {
        let tick = runner.world.tick();
        (runner, tick)
    });
    match result {
        Ok((mut runner, tick)) => {
            while runner.world.tick() < tick {
                runner.step();
            }
            println!("{}", line(&runner));
            let to = to.unwrap_or(tick);
            let every = every.unwrap_or(to.saturating_sub(tick).max(1));
            while runner.world.tick() < to {
                runner.step();
                let t = runner.world.tick();
                if (t - tick) % every.max(1) == 0 || t == to {
                    println!("{}", line(&runner));
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("replay: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the hash report of the safety net)
    #[test]
    fn the_first_difference_names_its_parts() {
        let a = "# h\n10 world 1 ai 2 rng 3\n20 world 4 ai 5 rng 6\n";
        let b = "# h\n10 world 1 ai 2 rng 3\n20 world 4 ai 9 rng 7\n";
        assert_eq!(first_difference(a, a), None);
        let report = first_difference(a, b).unwrap();
        assert!(report.starts_with("line 3:"), "{report}");
        assert!(report.ends_with("differing parts: ai, rng"), "{report}");
        assert!(first_difference(a, "# h\n").is_some());
    }
}
