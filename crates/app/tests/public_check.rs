//! The public copy's check (`scripts/public-check.py`, docs/releasing.md "The
//! public copy") refuses what leads a stranger nowhere: a Markdown link to a
//! file the export leaves out (`scripts/public/exclude.txt`) or that is not in
//! the tree, and a repository of the maintainer's other than the public one.
//! Each test writes a small tree with its own exclude list and runs the check
//! on it.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A tree like ours: `CLAUDE.md` and `specs/traces/` left out, the public CI
/// swapped in from `scripts/public/ci.yml`, a few published files.
fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("s2-public-check-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let base: [(&str, &str); 8] = [
        (
            "scripts/public/exclude.txt",
            "CLAUDE.md | the sessions' rules\n\
             specs/traces/ | the recordings\n\
             .github/workflows/ci.yml <- scripts/public/ci.yml | the public CI\n",
        ),
        (
            "scripts/public/ci.yml",
            "name: CI\non: push\npermissions:\n  contents: read\njobs: {}\n",
        ),
        (
            ".github/workflows/ci.yml",
            "name: CI\non: push\npermissions:\n  contents: read\njobs: {}\n",
        ),
        ("CLAUDE.md", "the sessions' rules\n"),
        ("specs/traces/README.md", "the recordings\n"),
        ("specs/MAP-grid.md", "a spec\n"),
        ("docs/index.md", "# Documentation\n\n## Start here\n"),
        ("PLAYING.md", "playing\n"),
    ];
    for (path, text) in base.iter().chain(files) {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, text).unwrap();
    }
    dir
}

/// The check's exit code and its findings (`file:line: kind: text`).
fn check(dir: &Path) -> (i32, Vec<String>) {
    let out = Command::new("python3")
        .arg(repo().join("scripts/public-check.py"))
        .arg(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let findings = stdout
        .lines()
        .filter(|l| !l.starts_with("public check:"))
        .map(str::to_string)
        .collect();
    (out.status.code().unwrap(), findings)
}

// spec: none (the public copy's check: a link to a left-out or missing file)
#[test]
fn links_to_files_the_public_copy_lacks_are_refused() {
    let dir = tree(
        "links",
        &[
            (
                "README.md",
                "# OpenS2X\n\nThe rules in [`CLAUDE.md`](CLAUDE.md).\n",
            ),
            (
                "docs/more.md",
                "# More\n\n\
                 | [`CLAUDE.md`](../CLAUDE.md) | the rules |\n\
                 A recording: [original](../specs/traces/README.md#rows).\n\
                 The CI's source: [ci.yml](/scripts/public/ci.yml).\n\
                 Gone: [old](old.md), outside: [up](../../elsewhere.md).\n\
                 [ref]: ../CLAUDE.md\n",
            ),
        ],
    );
    let (code, findings) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 1, "{findings:#?}");
    let want = [
        "README.md:3: link: CLAUDE.md (left out of the public copy)",
        "docs/more.md:3: link: ../CLAUDE.md (left out of the public copy)",
        "docs/more.md:4: link: ../specs/traces/README.md#rows (left out of the public copy)",
        "docs/more.md:5: link: /scripts/public/ci.yml (left out of the public copy)",
        "docs/more.md:6: link: old.md (not in the tree)",
        "docs/more.md:6: link: ../../elsewhere.md (outside the tree)",
        "docs/more.md:7: link: ../CLAUDE.md (left out of the public copy)",
    ];
    assert_eq!(findings, want);
}

// spec: none (the public copy's check: links that lead somewhere pass)
#[test]
fn links_into_the_public_copy_pass() {
    let dir = tree(
        "links-ok",
        &[(
            "README.md",
            "# OpenS2X\n\n\
             Every document in [`docs/index.md`](docs/index.md), [start](docs/index.md#start-here),\n\
             the [docs folder](docs/), [this](#openS2X), [playing](./PLAYING.md \"Playing\"),\n\
             the [CI](.github/workflows/ci.yml), [the site](https://github.com/Jensenzo/OpenS2X),\n\
             a [spec](<specs/MAP-grid.md>), ![a picture](https://example.org/a.png).\n\
             Not links: `[x](CLAUDE.md)` and\n\
             ```\n[x](CLAUDE.md)\n```\n",
        )],
    );
    let (code, findings) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!((code, findings), (0, Vec::<String>::new()));
}

// spec: none (the public copy's check: the maintainer's private repositories)
#[test]
fn the_private_repositories_are_refused() {
    let dir = tree(
        "repos",
        &[
            (
                "docs/releasing.md",
                "s2-sim = { git = \"https://github.com/Jensenzo/s2-reborn\", tag = \"v0.3.0\" }\n\
                 gh release list --repo jensenzo/s2-reborn\n\
                 the service actions.runner.Jensenzo-s2-reborn.s2-wsl\n\
                 git clone https://github.com/Jensenzo/s2-next.git\n\
                 Public: https://github.com/Jensenzo/OpenS2X, https://github.com/Jensenzo/OpenS2X.git,\n\
                 github.com/Jensenzo/OpenS2X/releases, the settings folder %APPDATA%\\s2-reborn.\n",
            ),
            (
                "crates/x/Cargo.toml",
                "[dependencies]\ns2-sim = { git = \"https://github.com/Jensenzo/s2-frontier\" }\n",
            ),
        ],
    );
    let (code, findings) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 1, "{findings:#?}");
    let want = [
        "crates/x/Cargo.toml:2: repository: github.com/Jensenzo/s2-frontier",
        "docs/releasing.md:1: repository: github.com/Jensenzo/s2-reborn",
        "docs/releasing.md:2: repository: jensenzo/s2-reborn",
        "docs/releasing.md:3: repository: Jensenzo-s2-reborn",
        "docs/releasing.md:4: repository: github.com/Jensenzo/s2-next",
        "docs/releasing.md:4: process: s2-next",
    ];
    assert_eq!(findings, want);
}

// spec: none (the progress log's entries name no private repository)
#[test]
fn the_changelog_names_no_private_repository() {
    let dir = tree("changelog", &[]);
    let log = dir.join("progress-log.md");
    std::fs::write(
        &log,
        "- 2026-10-15 | implementation | private repo jensenzo/s2-reborn created and pushed | README.md | none\n\
         - 2026-10-17 | implementation | new private repo Jensenzo/s2-next on https://github.com/Jensenzo/s2-reborn v0.1.0 | | none\n\
         - 2026-11-13 | implementation | links to Jensenzo/OpenS2X and github.com/Jensenzo/OpenS2X/releases | README.md | none\n\
         - 2026-11-13 | investigation | Jensenzo/s2-reborn | | none\n",
    )
    .unwrap();
    // -B: no bytecode beside the script in the repository.
    let out = Command::new("python3")
        .arg("-B")
        .arg("-c")
        .arg("import importlib.util as u, sys; s = u.spec_from_file_location('log', sys.argv[1]); m = u.module_from_spec(s); s.loader.exec_module(m); [print(e) for e in m.entries()]")
        .arg(repo().join("scripts/progress-log.py"))
        .env("S2_PROGRESS_LOG", &log)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let entries = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        entries.lines().collect::<Vec<_>>(),
        [
            "('2026-10-15', 'private repo s2-reborn created and pushed', 'README.md')",
            "('2026-10-17', 'new private repo s2-next on s2-reborn v0.1.0', '')",
            "('2026-11-13', 'links to Jensenzo/OpenS2X and github.com/Jensenzo/OpenS2X/releases', 'README.md')",
        ]
    );
}

// spec: none (the public copy's check: the maintainer's working process)
#[test]
fn the_working_process_is_refused() {
    let dir = tree(
        "process",
        &[
            (
                "CHANGELOG.md",
                "- impl-2026-11-14b done, as rel-2026-11-15 and inv-2026-11-12-night said\n\
                 - from the handoff of the planning chat (plan step 7.6a, plan 7.11)\n\
                 - F58 fixed; the play-test finding F7; Jens's decision\n\
                 - s2-next pins the tag\n",
            ),
            (
                "docs/keys.md",
                "F1 military settings, F10 messages, F12 screenshot; an `F32` field;\n\
                 the owner Jensenzo (github.com/Jensenzo/OpenS2X); a step of the plan;\n\
                 the release notes, a hand-written section; the next 2026-11-15 release\n",
            ),
        ],
    );
    let (code, findings) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 1, "{findings:#?}");
    let want = [
        "CHANGELOG.md:1: process: impl-2026-11-14b",
        "CHANGELOG.md:1: process: rel-2026-11-15",
        "CHANGELOG.md:1: process: inv-2026-11-12-night",
        "CHANGELOG.md:2: process: handoff",
        "CHANGELOG.md:2: process: planning chat",
        "CHANGELOG.md:2: process: plan step",
        "CHANGELOG.md:2: process: plan 7.11",
        "CHANGELOG.md:3: process: finding F7",
        "CHANGELOG.md:3: process: F58",
        "CHANGELOG.md:3: process: Jens",
        "CHANGELOG.md:4: process: s2-next",
    ];
    assert_eq!(findings, want);
}

// spec: none (the public copy's check: pictures only where allow.txt names them)
#[test]
fn pictures_only_where_the_allowed_list_names_them() {
    let dir = tree(
        "pictures",
        &[
            ("docs/images/town.jpg", "not really a picture\n"),
            ("docs/images/other.jpg", "not really a picture\n"),
            ("screens/town.jpg", "not really a picture\n"),
            (
                "scripts/public/allow.txt",
                "picture | docs/images/town.jpg | . | the README's picture\n",
            ),
        ],
    );
    let (code, findings) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 1, "{findings:#?}");
    let want = [
        "docs/images/other.jpg:0: picture: .jpg",
        "screens/town.jpg:0: picture: .jpg",
    ];
    assert_eq!(findings, want);
}
