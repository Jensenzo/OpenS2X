//! The release's tools for people outside (docs/releasing.md): the release
//! notes from the hand-written changelog (`scripts/changelog.py`), the
//! download's README without the pictures only GitHub shows
//! (`scripts/zip-readme.py`) and the fidelity chart from docs/fidelity.md's
//! record (`scripts/fidelity-chart.py`).
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("s2-release-tools-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs a script of ours with python3 -B (no bytecode beside it).
fn script(name: &str, args: &[&str]) -> Output {
    Command::new("python3")
        .arg("-B")
        .arg(repo().join("scripts").join(name))
        .args(args)
        .output()
        .unwrap()
}

const CHANGELOG: &str = "# Changelog\n\n\
    What changed in each release, newest first.\n\n\
    ## OpenS2X 0.9.6 (2026-11-15)\n\n\
    ### What is new\n\n\
    - Pictures in the README.\n\n\
    ## OpenS2X 0.9.5 (2026-11-14)\n\n\
    ### Fixed\n\n\
    - Roads at flags.\n\n\
    ### For projects built on OpenS2X: breaking changes\n\n\
    - The view draws differently.\n";

// spec: none (the release notes: the preface and the release's section)
#[test]
fn the_notes_are_the_preface_and_the_section() {
    let dir = scratch("notes");
    let log = dir.join("CHANGELOG.md");
    std::fs::write(&log, CHANGELOG).unwrap();
    let path = log.to_str().unwrap();
    let out = script("changelog.py", &["--notes", "0.9.5", "--file", path]);
    let check_new = script("changelog.py", &["--check", "0.9.6", "--file", path]);
    let check_missing = script("changelog.py", &["--check", "0.9.7", "--file", path]);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let notes = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = notes.lines().collect();
    assert!(lines[0].starts_with("OpenS2X is "), "{notes}");
    assert!(
        lines[2].contains("`OpenS2X-0.9.5-windows-x64.zip`"),
        "{notes}"
    );
    assert_eq!(
        &lines[3..],
        [
            "",
            "### Fixed",
            "",
            "- Roads at flags.",
            "",
            "### For projects built on OpenS2X: breaking changes",
            "",
            "- The view draws differently.",
        ]
    );
    assert!(check_new.status.success());
    assert_eq!(check_missing.status.code(), Some(1));
}

// spec: none (the download's README: no picture lines, the title as text)
#[test]
fn the_download_readme_has_no_pictures() {
    let dir = scratch("readme");
    let readme = dir.join("README.md");
    std::fs::write(
        &readme,
        "<h1 align=\"center\">OpenS2X</h1>\n\n\
         <p align=\"center\">\n  <b>An engine.<br>\n  Written from scratch.</b>\n</p>\n\n\
         <p align=\"center\">\n  <a href=\"x\"><img alt=\"CI\" src=\"https://example.org/b.svg\"></a>\n</p>\n\n\
         <p align=\"center\">\n  <img src=\"docs/images/town.jpg\" width=\"900\" alt=\"A town\">\n</p>\n\n\
         OpenS2X is a program.\n\n\
         <p align=\"center\"><img src=\"docs/images/fidelity.svg\" width=\"760\" alt=\"chart\"></p>\n\n\
         ## Play it\n",
    )
    .unwrap();
    let out = script("zip-readme.py", &[readme.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text,
        "# OpenS2X\n\n\
         **An engine. Written from scratch.**\n\n\
         The pictures of this page are on https://github.com/Jensenzo/OpenS2X.\n\n\
         OpenS2X is a program.\n\n\
         ## Play it\n"
    );
}

// spec: none (the fidelity chart: ticks equal of ticks compared per recorded game)
#[test]
fn the_fidelity_chart_follows_the_record() {
    let dir = scratch("chart");
    let doc = dir.join("fidelity.md");
    let svg = dir.join("fidelity.svg");
    std::fs::write(
        &doc,
        "# Fidelity\n\n## The record\n\n```fidelity\n\
         first-differing-tick = none\n\
         generator-first-differing-tick = 51724\n\
         one-player-first-differing-tick = 27673\n\
         one-player-generator-first-differing-tick = none\n\
         dense-first-differing-tick = none\n\
         dense-20000-first-differing-tick = 17533\n\
         dense-40000-first-differing-tick = none\n\
         hard-20000-first-differing-tick = 7054\n\
         hard-20000-buildings-first-differing-tick = none\n\
         hard-20000-generator-first-differing-tick = none\n\
         hard-150000-first-differing-tick = 48200\n\
         hard-150000-generator-first-differing-tick = 48043\n\
         ```\n",
    )
    .unwrap();
    let out = script(
        "fidelity-chart.py",
        &[
            "--record",
            doc.to_str().unwrap(),
            "--out",
            svg.to_str().unwrap(),
        ],
    );
    let text = std::fs::read_to_string(&svg).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.starts_with("<svg "), "{text}");
    for label in [
        "40,000 of 40,000",
        "51,723 of 150,000",
        "6,000 of 6,000",
        "17,532 of 20,000",
        "27,672 of 30,000",
        "30,000 of 30,000",
        "7,053 of 20,000",
        "20,000 of 20,000",
        "48,042 of 150,000",
    ] {
        assert!(text.contains(label), "{label} missing:\n{text}");
    }
    assert!(text.contains("prefers-color-scheme: dark"), "{text}");
}
