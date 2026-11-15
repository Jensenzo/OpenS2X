//! The golden state hashes (`tests/golden/hash/*.txt`): every file is made
//! again with the `s2-app hash` line its first line names and must come out
//! the same. Maps from the install are skipped when `S2_GAME_DIR` is unset
//! (as on CI); the `fixture:` games always run.
//!
//! An intended change is accepted with `scripts/golden.sh --bless`, and its
//! commit message names the spec rule that caused it (CI checks for one).

use std::path::{Path, PathBuf};
use std::process::Command;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/hash")
}

/// The `hash` arguments of a golden file's first line.
fn arguments(file: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(file).unwrap();
    let first = text.lines().next().unwrap_or_default();
    let rest = first
        .strip_prefix("# hash ")
        .unwrap_or_else(|| panic!("{}: no `# hash` line", file.display()));
    rest.split(' ').map(str::to_string).collect()
}

// spec: none (golden state hashes: determinism)
#[test]
fn state_hashes_match_the_golden_files() {
    let have_game = std::env::var_os("S2_GAME_DIR").is_some();
    let mut files: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    assert!(files.len() >= 5, "golden files: {files:?}");
    let mut failed = Vec::new();
    for file in &files {
        let args = arguments(file);
        if !args[0].starts_with("fixture:") && !have_game {
            eprintln!("skipped {}: S2_GAME_DIR is not set", file.display());
            continue;
        }
        let out = Command::new(env!("CARGO_BIN_EXE_s2-app"))
            .arg("hash")
            .args(&args)
            .arg("--golden")
            .arg(file)
            .output()
            .unwrap();
        eprint!("{}", String::from_utf8_lossy(&out.stdout));
        if !out.status.success() {
            eprint!("{}", String::from_utf8_lossy(&out.stderr));
            failed.push(file.display().to_string());
        }
    }
    assert!(failed.is_empty(), "golden hashes differ: {failed:?}");
}
