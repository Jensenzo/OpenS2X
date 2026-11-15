//! Save, load and continue gives the same game as never saving (and a
//! trace written twice is the same). A game is saved halfway (`s2-app hash
//! ... --save FILE`, the replay file: the start of a game of computer
//! players decides all of it), loaded and continued (`s2-app replay FILE
//! --to T`); its hash lines from the save on must be those of the game
//! played straight through. The fixture always runs; an install map with
//! `S2_GAME_DIR`.

use std::process::Command;

fn app(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_s2-app"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "s2-app {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn saved_and_continued_equals_straight(map: &str, half: u64, every: u64) {
    let whole = (2 * half).to_string();
    let straight = app(&["hash", map, &whole, "--every", &every.to_string()]);
    let straight: Vec<&str> = straight.lines().skip(1).collect();

    let dir = std::env::temp_dir().join(format!("s2-save-{}-{}", std::process::id(), half));
    std::fs::create_dir_all(&dir).unwrap();
    let save = dir.join("game.replay");
    let save = save.to_str().unwrap();
    app(&["hash", map, &half.to_string(), "--save", save]);
    let continued = app(&[
        "replay",
        save,
        "--to",
        &whole,
        "--every",
        &every.to_string(),
    ]);
    let continued: Vec<&str> = continued.lines().collect();
    std::fs::remove_dir_all(&dir).ok();

    // The straight game's lines from the save's tick on.
    let from_half: Vec<&str> = straight
        .iter()
        .copied()
        .filter(|l| {
            l.split(' ')
                .next()
                .and_then(|t| t.parse::<u64>().ok())
                .is_some_and(|t| t >= half)
        })
        .collect();
    assert_eq!(
        continued, from_half,
        "{map}: continued after a save at {half}"
    );
    assert!(continued.len() >= 2);
}

// spec: none (save, load and continue: determinism)
#[test]
fn a_saved_fixture_game_continues_as_if_never_saved() {
    saved_and_continued_equals_straight("fixture:town2", 2500, 500);
}

// spec: none (save, load and continue: determinism)
#[test]
fn a_saved_install_game_continues_as_if_never_saved() {
    if std::env::var_os("S2_GAME_DIR").is_none() {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    }
    saved_and_continued_equals_straight("MP_2P_Storm_Coast", 2000, 1000);
}

// docs/trace-format.md: a trace written twice is the same, and trace-diff
// says so.
// spec: none (trace writer, docs/trace-format.md)
#[test]
fn a_trace_of_the_same_game_is_the_same() {
    let dir = std::env::temp_dir().join(format!("s2-trace-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("a.csv"), dir.join("b.csv"));
    for f in [&a, &b] {
        app(&[
            "trace",
            "fixture:town2",
            "600",
            "--out",
            f.to_str().unwrap(),
        ]);
    }
    let text = std::fs::read_to_string(&a).unwrap();
    assert!(text.starts_with("# s2-trace 1\n"));
    assert!(text.lines().any(|l| l.starts_with("S,600,")));
    assert!(text.lines().any(|l| l.starts_with("B,600,")));
    let same = app(&["trace-diff", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        same.starts_with("the traces are the same (600 ticks)"),
        "{same}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

// docs/trace-format.md, TOOL-trace TRACE-4.6: `--rows-every 10` writes the
// settler and building rows of every 10th trace tick only, as the
// original's long recordings hold them, and the generator's of every tick;
// the rows it writes are the full trace's but clip_time.
// spec: TOOL-trace TRACE-4.6
#[test]
fn rows_every_writes_settlers_and_buildings_every_nth_tick() {
    let dir = std::env::temp_dir().join(format!("s2-trace-every-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (full, sparse) = (dir.join("full.csv"), dir.join("sparse.csv"));
    app(&[
        "trace",
        "fixture:town2",
        "300",
        "--out",
        full.to_str().unwrap(),
    ]);
    app(&[
        "trace",
        "fixture:town2",
        "300",
        "--rows-every",
        "10",
        "--out",
        sparse.to_str().unwrap(),
    ]);
    let text = std::fs::read_to_string(&sparse).unwrap();
    let ticks = |kind: &str| -> Vec<u64> {
        let mut t: Vec<u64> = text
            .lines()
            .filter(|l| l.starts_with(&format!("{kind},")))
            .map(|l| l.split(',').nth(1).unwrap().parse().unwrap())
            .collect();
        t.dedup();
        t
    };
    assert!(ticks("S").iter().all(|t| t % 10 == 0) && !ticks("S").is_empty());
    assert!(ticks("B").iter().all(|t| t % 10 == 0) && !ticks("B").is_empty());
    assert_eq!(ticks("G"), (1..=300).collect::<Vec<u64>>());
    // Without clip_time (column 9) every row written is the full trace's.
    let strip = |l: &str| {
        let mut c: Vec<&str> = l.split(',').collect();
        c[9] = "";
        c.join(",")
    };
    let all: std::collections::BTreeSet<String> = std::fs::read_to_string(&full)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with(['S', 'B', 'G']))
        .map(strip)
        .collect();
    let rows: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with(['S', 'B', 'G']))
        .collect();
    assert!(rows.iter().all(|l| all.contains(&strip(l))));
    std::fs::remove_dir_all(&dir).ok();
}
