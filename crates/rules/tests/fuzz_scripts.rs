//! The property scripts (Lua tables of the rules) damaged: running them and
//! turning the result into the simulation's rules may fail, but never
//! panic. One script at a time is replaced by a damaged copy (bytes flipped,
//! cut, repeated, numbers made huge or negative) and the whole set is run
//! (`s2_rules::script::run_with_overlays`), then `RuleSet::sim_rules`.
//!
//! Needs the install (`S2_GAME_DIR`); in release mode:
//! `cargo test --release -p s2-rules --test fuzz_scripts -- --ignored --nocapture`
//! (`FUZZ_ROUNDS`, default 40 damaged copies per script). A damaged copy
//! that contains a loop keyword is skipped: the shipped scripts have no
//! loops, and a damaged one could run forever.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use s2_assets::{GameDir, GameDirError};
use s2_rules::script::{self, ScriptSource};
use s2_rules::InstallSource;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Damage Lua text: flip or replace bytes, cut lines out, repeat them, or
/// turn numbers into edge values.
fn mutate(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut d = seed.to_vec();
    for _ in 0..1 + rng.below(4) {
        let at = rng.below(d.len());
        match rng.below(6) {
            0 if !d.is_empty() => d[at] ^= 1 << rng.below(8),
            1 if !d.is_empty() => d[at] = b"(),{}=\"'-0123456789.e"[rng.below(21)],
            2 => {
                let end = (at + rng.below(200)).min(d.len());
                d.drain(at..end);
            }
            3 if d.len() > 2 => {
                let from = rng.below(d.len());
                let len = rng.below((d.len() - from).min(200));
                let piece = d[from..from + len].to_vec();
                d.splice(at..at, piece);
            }
            4 => {
                let edge: &[&[u8]] = &[
                    b"-1",
                    b"0",
                    b"1e308",
                    b"-1e308",
                    b"4294967296",
                    b"0/0",
                    b"1/0",
                    b"nil",
                    b"''",
                ];
                let pick = edge[rng.below(edge.len())];
                // Replace the number that starts at or after `at`.
                if let Some(start) = (at..d.len()).find(|&i| d[i].is_ascii_digit()) {
                    let end = (start..d.len())
                        .find(|&i| !(d[i].is_ascii_digit() || d[i] == b'.'))
                        .unwrap_or(d.len());
                    d.splice(start..end, pick.iter().copied());
                }
            }
            _ => d.truncate(at),
        }
    }
    d
}

fn loops(code: &[u8]) -> bool {
    let text = String::from_utf8_lossy(code);
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|w| matches!(w, "while" | "repeat" | "for" | "goto"))
}

/// The install's scripts with one replaced.
struct Damaged {
    install: InstallSource,
    path: String,
    code: Vec<u8>,
}

impl ScriptSource for Damaged {
    fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        if path.eq_ignore_ascii_case(&self.path) {
            Ok(self.code.clone())
        } else {
            self.install.read(path)
        }
    }
}

// spec: none (damaged property scripts fail without panic)
#[test]
#[ignore = "the long run on the install's scripts, in release mode"]
fn damaged_property_scripts_fail_without_panic() {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let rounds: u32 = std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(40);
    let install = || InstallSource::of(&dir);
    // Every script the entry script runs, as the load report names them.
    let (_, report) = s2_rules::load(&dir).expect("the shipped scripts run");
    let scripts: Vec<String> = report
        .scripts
        .iter()
        .filter(|s| !s.starts_with("overlay:"))
        .cloned()
        .collect();
    assert!(scripts.len() > 5, "{scripts:?}");
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut rng = Rng(3);
    let (mut runs, mut errors, mut skipped) = (0u32, 0u32, 0u32);
    let mut failures = Vec::new();
    for path in &scripts {
        let seed = install().read(path).expect("a script");
        for round in 0..rounds {
            let code = mutate(&seed, &mut rng);
            if loops(&code) {
                skipped += 1;
                continue;
            }
            let source = Rc::new(Damaged {
                install: install(),
                path: path.clone(),
                code: code.clone(),
            });
            runs += 1;
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                match script::run(source, s2_rules::ENTRY_SCRIPT) {
                    Ok((rules, _)) => rules.sim_rules().is_ok(),
                    Err(_) => false,
                }
            }));
            match outcome {
                Ok(true) => {}
                Ok(false) => errors += 1,
                Err(_) => {
                    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../target/fuzz-failures");
                    std::fs::create_dir_all(&out).ok();
                    let file = out.join(format!(
                        "{}-round{round}.lua",
                        path.replace(['\\', '/'], "_")
                    ));
                    std::fs::write(&file, &code).ok();
                    failures.push(format!("{path} round {round}: {}", file.display()));
                }
            }
        }
    }
    std::panic::set_hook(hook);
    eprintln!(
        "{} scripts: {runs} damaged runs ({errors} refused with an error), {skipped} skipped for a loop keyword",
        scripts.len()
    );
    assert!(failures.is_empty(), "panics:\n{}", failures.join("\n"));
}
