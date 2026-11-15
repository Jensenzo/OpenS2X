//! The game's first run with its files elsewhere: with `--profile-dir` the
//! first run's look for the game folder reads (and, after the folder
//! question, writes) the program's settings in that folder, never the
//! player's own `settings.txt` (the download tried as a stranger gets it,
//! without touching the player's settings, profile or saves). The game is
//! `OpenS2X` since 13 Nov 2026; its settings folder keeps the name
//! `s2-reborn`, so a profile made before finds its game folder as before.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// spec: none (the program's own settings file; the folders of SET-1.1 untouched)
#[test]
fn the_profile_dir_holds_the_first_runs_settings() {
    let Some(game) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    // The player's own settings file is found by XDG_CONFIG_HOME on Linux
    // (APPDATA on Windows).
    if !cfg!(target_os = "linux") {
        eprintln!("skipped: the player's folder is set by XDG_CONFIG_HOME on Linux only");
        return;
    }
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("first-run");
    let _ = std::fs::remove_dir_all(&root);
    // The player's settings name a game folder that holds the game.
    let player = root.join("config");
    std::fs::create_dir_all(player.join("s2-reborn")).unwrap();
    let theirs = player.join("s2-reborn").join("settings.txt");
    let text = format!(
        "game_dir={}\nprofile=default\nday_night=1\n",
        PathBuf::from(&game).display()
    );
    std::fs::write(&theirs, &text).unwrap();
    let own = root.join("profile");
    std::fs::create_dir_all(&own).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_OpenS2X"))
        .env_remove("S2_GAME_DIR")
        .env("XDG_CONFIG_HOME", &player)
        .env("HOME", root.join("home"))
        .arg("--profile-dir")
        .arg(&own)
        .arg("--save-dir")
        .arg(root.join("saves"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A run that found the player's game folder would go on to open its
    // window; one that looks in its own folder finds none there, has no
    // folder picker on Linux and stops at once.
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    assert!(
        err.contains("set S2_GAME_DIR or pass --game-dir"),
        "the first run did not look for the game folder in --profile-dir \
         (exit {status:?}):\n{err}"
    );
    assert!(status.is_some_and(|s| !s.success()), "{status:?}\n{err}");
    // The player's file is as it was; nothing was written in either folder.
    assert_eq!(std::fs::read_to_string(&theirs).unwrap(), text);
    assert_eq!(std::fs::read_dir(&own).unwrap().count(), 0);
}

// spec: none (the public name OpenS2X: the program's own settings stay in the folder `s2-reborn`, SET-1.1's folders untouched)
#[test]
fn the_renamed_game_finds_the_settings_made_by_s2_reborn() {
    let Some(game) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    if !cfg!(target_os = "linux") {
        eprintln!("skipped: the player's folder is set by XDG_CONFIG_HOME on Linux only");
        return;
    }
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("first-run-renamed");
    let _ = std::fs::remove_dir_all(&root);
    // A player's settings as s2-reborn wrote them, naming the game folder.
    let player = root.join("config");
    std::fs::create_dir_all(player.join("s2-reborn")).unwrap();
    let theirs = player.join("s2-reborn").join("settings.txt");
    let text = format!(
        "game_dir={}\nprofile=default\nday_night=1\n",
        PathBuf::from(&game).display()
    );
    std::fs::write(&theirs, &text).unwrap();
    // A map name the install does not have: the program stops before its
    // window opens, after it has looked for the game folder.
    let mut child = Command::new(env!("CARGO_BIN_EXE_OpenS2X"))
        .env_remove("S2_GAME_DIR")
        .env_remove("APPDATA")
        .env("XDG_CONFIG_HOME", &player)
        .env("HOME", root.join("home"))
        .arg("--map-name")
        .arg("no-such-map")
        .arg("--save-dir")
        .arg(root.join("saves"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    // It found the game folder in s2-reborn's settings: it never fell back
    // to asking for one, and went on to look for the map.
    assert!(
        !err.contains("set S2_GAME_DIR or pass --game-dir"),
        "the renamed game did not find the folder in s2-reborn's settings:\n{err}"
    );
    assert!(
        err.contains("no map named no-such-map"),
        "{status:?}\n{err}"
    );
    assert!(status.is_some_and(|s| !s.success()), "{status:?}\n{err}");
    // The settings are as they were; nothing was saved.
    assert_eq!(std::fs::read_to_string(&theirs).unwrap(), text);
    assert!(!root.join("saves").exists());
}
