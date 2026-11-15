//! `s2-app fidelity [--root DIR] [--traces DIR] [--record]`: how close our
//! game is to the original's recording (`docs/fidelity.md`). It makes our
//! traces from the program as it is (`tests/traces/README.md`; they are not
//! committed), then `trace-diff` runs on them against the original's
//! (`specs/traces/`), and the result is the first differing tick and the
//! number of ticks whose generator rows match. The record in
//! `docs/fidelity.md` holds the values of the last commit that wrote them;
//! the test below fails only when the first differing tick moves earlier
//! than recorded (a regression), never for a difference as such.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use crate::trace::{diff, open, DiffOptions, DiffReport};

/// One of our traces: its file name in the traces directory and the
/// `s2-app trace` arguments that make it (`tests/traces/README.md`).
pub struct Ours {
    pub file: &'static str,
    pub args: &'static [&'static str],
}

/// Our trace of the default game (S, B and G rows, trace ticks 0 to
/// 40 000, the range of R4): player 1 the computer of the default slot
/// strength 0, profile 1 (Weak; UI-SETUP-2.5, Q-275).
pub const OURS: Ours = Ours {
    file: "MP_2P_Storm_Coast-default-40000.csv.gz",
    args: &["MP_2P_Storm_Coast", "40000", "--ai", "1=1", "--from", "0"],
};
/// Our generator rows of the same game, trace ticks 0 to 150 000.
pub const OURS_GENERATOR: Ours = Ours {
    file: "MP_2P_Storm_Coast-default-generator-150000.csv.gz",
    args: &[
        "MP_2P_Storm_Coast",
        "150000",
        "--ai",
        "1=1",
        "--from",
        "0",
        "--kinds",
        "G",
    ],
};
/// Our trace of the one-player game (slot 1 closed, no computer player; S,
/// B and G rows, trace ticks 0 to 30 000).
pub const OURS_1P: Ours = Ours {
    file: "MP_2P_Storm_Coast-1p-30000.csv.gz",
    args: &[
        "MP_2P_Storm_Coast",
        "30000",
        "--ai",
        "none",
        "--closed",
        "1",
        "--from",
        "0",
    ],
};
/// Our trace of the default game with player 1 a computer player of
/// profile 3 (Strong) instead (S, B and G rows, trace ticks 0 to 20 000).
pub const OURS_HARD: Ours = Ours {
    file: "MP_2P_Storm_Coast-hard-20000.csv.gz",
    args: &["MP_2P_Storm_Coast", "20000", "--ai", "1=3", "--from", "0"],
};
/// Our trace of the Hard game to 150 000 for the original's long
/// recording (R3): S and B rows every 10th trace tick, G rows every tick.
pub const OURS_HARD_150000: Ours = Ours {
    file: "MP_2P_Storm_Coast-hard-150000.csv.gz",
    args: &[
        "MP_2P_Storm_Coast",
        "150000",
        "--ai",
        "1=3",
        "--from",
        "0",
        "--rows-every",
        "10",
    ],
};
/// Every trace of ours the pairs read.
pub const ALL_OURS: [&Ours; 5] = [
    &OURS,
    &OURS_GENERATOR,
    &OURS_1P,
    &OURS_HARD,
    &OURS_HARD_150000,
];
/// Where `s2-app fidelity` leaves our traces, under the repository.
pub const TRACES: &str = "target/traces";

/// The original's recording of the default game (`specs/traces/README.md`).
pub const ORIGINAL: &str = "specs/traces/original.s2trace.gz";
pub const ORIGINAL_GENERATOR: &str = "specs/traces/original-generator.s2trace.gz";
/// The original's dense recording of the same game (E-trace-005): every
/// row of every trace tick to 6000. Its generator file holds the same G
/// rows, so the main file alone is compared.
pub const ORIGINAL_DENSE: &str = "specs/traces/original-dense-6000.s2trace.gz";
/// The original's dense recording to 20 000 (E-trace-011): every 10th trace
/// tick to 5490, then every tick 5500 to 20 000.
pub const ORIGINAL_DENSE_20000: &str = "specs/traces/original-dense-20000.s2trace.gz";
/// The original's recording of the default game to 40 000 (R4,
/// E-trace-014): every 10th trace tick to 19 990, every tick 20 000 to
/// 32 000, then every 10th to 40 000.
pub const ORIGINAL_DENSE_40000: &str = "specs/traces/original-dense-40000.s2trace.gz";
/// The original's dense recording of the Hard game (E-trace-012): player 1
/// a computer player of strength 2, profile 3 (Strong), otherwise as
/// E-trace-011, the same ticks.
pub const ORIGINAL_HARD: &str = "specs/traces/original-hard-dense-20000.s2trace.gz";
/// The original's long recording of the Hard game (R3, E-trace-013): S, B
/// and G rows every 10th trace tick to 40 000, then every 100th to 150 000;
/// and its G rows of every tick.
pub const ORIGINAL_HARD_150000: &str = "specs/traces/original-hard-150000.s2trace.gz";
pub const ORIGINAL_HARD_150000_GENERATOR: &str =
    "specs/traces/original-hard-150000-generator.s2trace.gz";
/// The settler column the R3 pair leaves out: its recorder shows some
/// settlers' animation a tick late as well (carrier 90357 standing at 5290,
/// walking in E-trace-012 and in ours), a view lag; without it the two
/// recordings agree on every common tick.
pub const HARD_150000_WITHOUT: &str = "animation";
/// The settler column the first pair leaves out: E-trace-002's recorder
/// shows some settlers' animation a tick late (TRACE-4.4: carriers at 1762,
/// the stonecutter at 3444), a view lag, not a difference of the games.
pub const FIRST_PAIR_WITHOUT: &str = "animation";
/// The original's recordings of the one-player game (E-trace-004, TRC-14).
pub const ORIGINAL_1P: &str = "specs/traces/original-1p.s2trace.gz";
pub const ORIGINAL_1P_GENERATOR: &str = "specs/traces/original-1p-generator.s2trace.gz";

/// Makes our traces in `dir` from the game data (`S2_GAME_DIR`), one thread
/// per trace: the program is deterministic, so they are the same bytes
/// every time for the same code (TOOL-trace TRACE-4.6).
pub fn make(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::thread::scope(|s| {
        let made: Vec<_> = ALL_OURS
            .iter()
            .map(|ours| {
                s.spawn(move || {
                    let mut args: Vec<String> = ours.args.iter().map(|a| a.to_string()).collect();
                    args.push("--out".into());
                    args.push(dir.join(ours.file).to_string_lossy().into_owned());
                    crate::trace::write(&args).map_err(|e| format!("{}: {e}", ours.file))
                })
            })
            .collect();
        // The scope joins any thread left after an error.
        made.into_iter()
            .try_for_each(|h| h.join().expect("a trace thread"))
    })
}

/// The fidelity record's keys, in the order written.
pub const KEYS: [&str; 40] = [
    "first-differing-tick",
    "settlers-first-differing-tick",
    "buildings-first-differing-tick",
    "generator-first-differing-tick",
    "generator-matching-ticks",
    "generator-compared-ticks",
    "one-player-first-differing-tick",
    "one-player-generator-first-differing-tick",
    "one-player-generator-matching-ticks",
    "one-player-generator-compared-ticks",
    "dense-first-differing-tick",
    "dense-settlers-first-differing-tick",
    "dense-buildings-first-differing-tick",
    "dense-generator-first-differing-tick",
    "dense-generator-matching-ticks",
    "dense-generator-compared-ticks",
    "dense-20000-first-differing-tick",
    "dense-20000-settlers-first-differing-tick",
    "dense-20000-buildings-first-differing-tick",
    "dense-20000-generator-first-differing-tick",
    "dense-20000-generator-matching-ticks",
    "dense-20000-generator-compared-ticks",
    "dense-40000-first-differing-tick",
    "dense-40000-settlers-first-differing-tick",
    "dense-40000-buildings-first-differing-tick",
    "dense-40000-generator-first-differing-tick",
    "dense-40000-generator-matching-ticks",
    "dense-40000-generator-compared-ticks",
    "hard-20000-first-differing-tick",
    "hard-20000-settlers-first-differing-tick",
    "hard-20000-buildings-first-differing-tick",
    "hard-20000-generator-first-differing-tick",
    "hard-20000-generator-matching-ticks",
    "hard-20000-generator-compared-ticks",
    "hard-150000-first-differing-tick",
    "hard-150000-settlers-first-differing-tick",
    "hard-150000-buildings-first-differing-tick",
    "hard-150000-generator-first-differing-tick",
    "hard-150000-generator-matching-ticks",
    "hard-150000-generator-compared-ticks",
];

/// The measures, by key. A first differing tick is absent when nothing
/// differed.
pub type Measures = BTreeMap<&'static str, u64>;

/// One comparison: the pair's name ("ours against the original's"), the
/// settler column it leaves out besides `clip_time` (if any), the report.
pub type Comparison = (String, Option<&'static str>, DiffReport);

/// The comparisons of the pairs: our traces in `traces` (made by `make`)
/// against the original's under `root` (the repository).
pub fn measure(root: &Path, traces: &Path) -> Result<(Measures, Vec<Comparison>), String> {
    // As specs/traces/README.md says: settlers by place, the ticks both
    // hold, from trace tick 1 (tick 0 is the original's world half set up,
    // TRACE-3.8); the original has no clip_time.
    let mut options = DiffOptions {
        by_place: true,
        common_ticks: true,
        from: 1,
        ..DiffOptions::default()
    };
    options.settler_columns.retain(|&c| c != "clip_time");
    let pair_with = |a: &Ours, b: &str, options: &DiffOptions| -> Result<DiffReport, String> {
        let (pa, pb) = (traces.join(a.file), root.join(b));
        let (pa, pb) = (pa.to_string_lossy(), pb.to_string_lossy());
        diff(open(&pa)?, open(&pb)?, (&pa, &pb), options)
    };
    let pair = |a: &Ours, b: &str| pair_with(a, b, &options);
    let mut lag_free = options.clone();
    lag_free
        .settler_columns
        .retain(|&c| c != FIRST_PAIR_WITHOUT);
    let full = pair_with(&OURS, ORIGINAL, &lag_free)?;
    let generator = pair(&OURS_GENERATOR, ORIGINAL_GENERATOR)?;
    let one = pair(&OURS_1P, ORIGINAL_1P)?;
    // Our one-player trace holds S and B rows the generator recording has
    // not: only the G rows are compared.
    let generator_only = DiffOptions {
        kinds: "G".into(),
        ..options.clone()
    };
    let one_generator = pair_with(&OURS_1P, ORIGINAL_1P_GENERATOR, &generator_only)?;
    let dense = pair(&OURS, ORIGINAL_DENSE)?;
    let dense_20000 = pair(&OURS, ORIGINAL_DENSE_20000)?;
    // R4: past the dense recording, its settlers without the animation
    // column, like the first pair (positions, not the recorder's view).
    let dense_40000 = pair_with(&OURS, ORIGINAL_DENSE_40000, &lag_free)?;
    let hard = pair(&OURS_HARD, ORIGINAL_HARD)?;
    // R3: its S, B and G rows without the animation column, its generator
    // rows of every tick.
    let mut r3_lag_free = options.clone();
    r3_lag_free
        .settler_columns
        .retain(|&c| c != HARD_150000_WITHOUT);
    let hard_150000 = pair_with(&OURS_HARD_150000, ORIGINAL_HARD_150000, &r3_lag_free)?;
    let hard_150000_generator = pair_with(
        &OURS_HARD_150000,
        ORIGINAL_HARD_150000_GENERATOR,
        &generator_only,
    )?;
    let mut m = Measures::new();
    // The first differing ticks of a pair of full traces, under `prefix`.
    let firsts = |prefix: &str, report: &DiffReport, m: &mut Measures| {
        let key = |name: &str| -> &'static str {
            KEYS.iter()
                .find(|k| **k == format!("{prefix}{name}"))
                .expect("a key")
        };
        if let Some(t) = report.first_tick {
            m.insert(key("first-differing-tick"), t);
        }
        let settlers = report
            .tallies
            .iter()
            .filter(|(k, _)| *k != "buildings" && *k != "generator")
            .filter_map(|(_, t)| t.first)
            .min();
        if let Some(t) = settlers {
            m.insert(key("settlers-first-differing-tick"), t);
        }
        if let Some(t) = report.tallies.get("buildings").and_then(|t| t.first) {
            m.insert(key("buildings-first-differing-tick"), t);
        }
    };
    firsts("", &full, &mut m);
    let g = generator
        .tallies
        .get("generator")
        .cloned()
        .unwrap_or_default();
    if let Some(t) = g.first {
        m.insert("generator-first-differing-tick", t);
    }
    m.insert("generator-matching-ticks", g.compared - g.differing);
    m.insert("generator-compared-ticks", g.compared);
    if let Some(t) = one.first_tick {
        m.insert("one-player-first-differing-tick", t);
    }
    let g = one_generator
        .tallies
        .get("generator")
        .cloned()
        .unwrap_or_default();
    if let Some(t) = g.first {
        m.insert("one-player-generator-first-differing-tick", t);
    }
    m.insert(
        "one-player-generator-matching-ticks",
        g.compared - g.differing,
    );
    m.insert("one-player-generator-compared-ticks", g.compared);
    // The generator measures of a full trace's pair, under `prefix`.
    let generator_of = |prefix: &str, report: &DiffReport, m: &mut Measures| {
        let key = |name: &str| -> &'static str {
            KEYS.iter()
                .find(|k| **k == format!("{prefix}{name}"))
                .expect("a key")
        };
        let g = report.tallies.get("generator").cloned().unwrap_or_default();
        if let Some(t) = g.first {
            m.insert(key("generator-first-differing-tick"), t);
        }
        m.insert(key("generator-matching-ticks"), g.compared - g.differing);
        m.insert(key("generator-compared-ticks"), g.compared);
    };
    firsts("dense-", &dense, &mut m);
    generator_of("dense-", &dense, &mut m);
    firsts("dense-20000-", &dense_20000, &mut m);
    generator_of("dense-20000-", &dense_20000, &mut m);
    firsts("dense-40000-", &dense_40000, &mut m);
    generator_of("dense-40000-", &dense_40000, &mut m);
    firsts("hard-20000-", &hard, &mut m);
    generator_of("hard-20000-", &hard, &mut m);
    firsts("hard-150000-", &hard_150000, &mut m);
    generator_of("hard-150000-", &hard_150000_generator, &mut m);
    // Our traces by their path in the repository when they lie in it.
    let name = |a: &Ours, b: &str| {
        let ours = traces.join(a.file);
        let ours = ours
            .strip_prefix(root)
            .unwrap_or(&ours)
            .display()
            .to_string();
        format!("{ours} against {b}")
    };
    Ok((
        m,
        vec![
            (name(&OURS, ORIGINAL), Some(FIRST_PAIR_WITHOUT), full),
            (name(&OURS_GENERATOR, ORIGINAL_GENERATOR), None, generator),
            (name(&OURS_1P, ORIGINAL_1P), None, one),
            (name(&OURS_1P, ORIGINAL_1P_GENERATOR), None, one_generator),
            (name(&OURS, ORIGINAL_DENSE), None, dense),
            (name(&OURS, ORIGINAL_DENSE_20000), None, dense_20000),
            (
                name(&OURS, ORIGINAL_DENSE_40000),
                Some(FIRST_PAIR_WITHOUT),
                dense_40000,
            ),
            (name(&OURS_HARD, ORIGINAL_HARD), None, hard),
            (
                name(&OURS_HARD_150000, ORIGINAL_HARD_150000),
                Some(HARD_150000_WITHOUT),
                hard_150000,
            ),
            (
                name(&OURS_HARD_150000, ORIGINAL_HARD_150000_GENERATOR),
                None,
                hard_150000_generator,
            ),
        ],
    ))
}

/// The record block as `docs/fidelity.md` holds it.
pub fn record_text(m: &Measures) -> String {
    let mut out = String::from("```fidelity\n");
    for key in KEYS {
        match m.get(key) {
            Some(v) => out += &format!("{key} = {v}\n"),
            None => out += &format!("{key} = none\n"),
        }
    }
    out + "```\n"
}

/// The record in `docs/fidelity.md`: the block fenced as `fidelity`. Lines
/// may end in `\r\n` (a Windows checkout).
pub fn read_record(text: &str) -> Result<Measures, String> {
    let text = &text.replace("\r\n", "\n");
    let start = text.find("```fidelity\n").ok_or("no ```fidelity block")? + "```fidelity\n".len();
    let end = start
        + text[start..]
            .find("```")
            .ok_or("unclosed ```fidelity block")?;
    let mut m = Measures::new();
    for line in text[start..end].lines() {
        let (key, value) = line
            .split_once(" = ")
            .ok_or(format!("not key = value: {line}"))?;
        let key = KEYS
            .iter()
            .find(|&&k| k == key)
            .ok_or(format!("unknown key {key}"))?;
        if value != "none" {
            m.insert(
                key,
                value.parse().map_err(|_| format!("{line}: not a number"))?,
            );
        }
    }
    Ok(m)
}

/// The regressions of `now` against `recorded`: a first differing tick
/// earlier than recorded (none recorded is "never differs").
pub fn regressions(recorded: &Measures, now: &Measures) -> Vec<String> {
    KEYS.iter()
        .filter(|k| k.ends_with("first-differing-tick"))
        .filter_map(|&k| {
            let never = u64::MAX;
            let (was, is) = (
                recorded.get(k).copied().unwrap_or(never),
                now.get(k).copied().unwrap_or(never),
            );
            let show = |v: u64| {
                if v == never {
                    "none".to_string()
                } else {
                    v.to_string()
                }
            };
            (is < was).then(|| format!("{k}: {}, recorded {}", show(is), show(was)))
        })
        .collect()
}

/// `text` with its record block replaced by `m`'s.
pub fn with_record(text: &str, m: &Measures) -> Result<String, String> {
    let text = &text.replace("\r\n", "\n");
    let start = text.find("```fidelity\n").ok_or("no ```fidelity block")?;
    let body = start + "```fidelity\n".len();
    let end = body
        + text[body..]
            .find("```\n")
            .ok_or("unclosed ```fidelity block")?
        + 4;
    Ok(format!(
        "{}{}{}",
        &text[..start],
        record_text(m),
        &text[end..]
    ))
}

/// `s2-app fidelity [--root DIR] [--traces DIR] [--record]`: make our
/// traces (in `--traces`, by default `target/traces` under the root),
/// measure, compare with the record and, with `--record`, write the
/// measures into it.
pub fn run(args: &[String]) -> ExitCode {
    let mut root = std::path::PathBuf::from(".");
    let mut traces = None;
    let mut record = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" if it.len() > 0 => root = it.next().expect("checked").into(),
            "--traces" if it.len() > 0 => {
                traces = Some(std::path::PathBuf::from(it.next().expect("checked")))
            }
            "--record" => record = true,
            _ => {
                eprintln!("fidelity: unknown argument {arg}");
                return ExitCode::from(2);
            }
        }
    }
    let traces = traces.unwrap_or_else(|| root.join(TRACES));
    eprintln!("fidelity: making our traces in {}", traces.display());
    if let Err(e) = make(&traces) {
        eprintln!("fidelity: {e}");
        return ExitCode::from(2);
    }
    let (m, reports) = match measure(&root, &traces) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("fidelity: {e}");
            return ExitCode::from(2);
        }
    };
    for (name, without, report) in &reports {
        println!("{name}:");
        println!("  {}", report.first.as_deref().unwrap_or("the same"));
        if let Some(column) = without {
            println!("  without: {column}");
        }
    }
    print!("{}", record_text(&m));
    let doc = root.join("docs/fidelity.md");
    if record {
        let written = std::fs::read_to_string(&doc)
            .map_err(|e| e.to_string())
            .and_then(|t| with_record(&t, &m))
            .and_then(|t| std::fs::write(&doc, t).map_err(|e| e.to_string()));
        return match written {
            Ok(()) => {
                println!("docs/fidelity.md: record written");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("docs/fidelity.md: {e}");
                ExitCode::from(2)
            }
        };
    }
    let recorded = std::fs::read_to_string(&doc)
        .map_err(|e| e.to_string())
        .and_then(|t| read_record(&t));
    match recorded {
        Ok(recorded) => {
            let worse = regressions(&recorded, &m);
            for r in &worse {
                println!("regression: {r}");
            }
            if recorded == m {
                println!("docs/fidelity.md: the same");
            } else if worse.is_empty() {
                println!(
                    "docs/fidelity.md: differs, no regression (write the block above into it)"
                );
            }
            if worse.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("docs/fidelity.md: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Our traces are made from the game data first: 200 000 ticks of game,
    // about 25 s in a release build and minutes in a debug one, so a debug
    // build leaves it out (`cargo test --release -p s2-app fidelity`; CI
    // runs `s2-app fidelity` in release, which checks the same).
    // spec: SIM-RNG-2.5, TOOL-trace TRACE-4.4, 4.5, 4.6 (the original's
    // traces against ours, made by the program as it is, E-trace-002's view
    // lag left out; fails only on a regression of the first differing tick)
    #[test]
    #[cfg_attr(debug_assertions, ignore = "a debug build is too slow")]
    fn no_earlier_difference_than_recorded() {
        if std::env::var_os("S2_GAME_DIR").is_none() {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let traces = std::env::temp_dir().join(format!("s2-fidelity-{}", std::process::id()));
        make(&traces).expect("our traces");
        let measured = measure(&root, &traces);
        std::fs::remove_dir_all(&traces).ok();
        let (m, reports) = measured.expect("the traces");
        for (name, _, report) in &reports {
            println!("{name}: {}", report.first.as_deref().unwrap_or("the same"));
        }
        print!("{}", record_text(&m));
        let text =
            std::fs::read_to_string(root.join("docs/fidelity.md")).expect("docs/fidelity.md");
        let recorded = read_record(&text).expect("the record");
        let worse = regressions(&recorded, &m);
        assert!(
            worse.is_empty(),
            "the first differing tick moved earlier: {}",
            worse.join("; ")
        );
    }

    // spec: none (the fidelity record's own format)
    #[test]
    fn the_record_round_trips() {
        let mut m = Measures::new();
        m.insert("first-differing-tick", 7);
        m.insert("generator-matching-ticks", 2);
        let text = format!("text before\n\n{}\ntext after\n", record_text(&m));
        assert_eq!(read_record(&text), Ok(m.clone()));
        assert_eq!(read_record(&text.replace('\n', "\r\n")), Ok(m.clone()));
        let mut later = m.clone();
        later.insert("first-differing-tick", 9);
        let rewritten = with_record(&text, &later).unwrap();
        assert_eq!(read_record(&rewritten), Ok(later));
        assert!(rewritten.starts_with("text before\n\n```fidelity\n"));
        assert!(rewritten.ends_with("```\n\ntext after\n"));
        let mut earlier = m.clone();
        earlier.insert("first-differing-tick", 6);
        assert_eq!(regressions(&m, &earlier).len(), 1);
        assert!(regressions(&earlier, &m).is_empty());
        // Recorded as never differing, now differing: a regression.
        let mut none = m.clone();
        none.remove("first-differing-tick");
        assert_eq!(regressions(&none, &m).len(), 1);
    }
}
