//! Nothing the safety net starts may write the player's own files, not even
//! the same bytes (`scripts/player-files-guard.sh`): the guard reads only the
//! names and modified times of the player's settings file, profiles and
//! saves, and fails a run in which one of them changed. `golden-images.sh`
//! gives each scene a profile folder of its own and fails a scene whose
//! program wrote the player's files.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("s2-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The player's three places under `dir`, with a settings file, a profile
/// and a save.
fn player(dir: &Path) {
    std::fs::create_dir_all(dir.join("users")).unwrap();
    std::fs::create_dir_all(dir.join("saves")).unwrap();
    std::fs::write(dir.join("settings.txt"), "game_dir=C:\\game\r\n").unwrap();
    std::fs::write(dir.join("users/default01.cfg"), [1u8; 16]).unwrap();
    std::fs::write(dir.join("saves/secret game name.sav"), [2u8; 16]).unwrap();
}

/// Sets a file's modified time to `seconds` past the epoch: the same bytes,
/// another time.
fn touch(file: &Path, seconds: u64) {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds);
    std::fs::File::options()
        .write(true)
        .open(file)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

fn with_player(command: &mut Command, dir: &Path) -> Output {
    command
        .env("S2_PLAYER_SETTINGS", dir.join("settings.txt"))
        .env("S2_PLAYER_USERS", dir.join("users"))
        .env("S2_PLAYER_SAVES", dir.join("saves"))
        .output()
        .unwrap()
}

/// Runs `body` in bash after sourcing the guard, the player's files in `dir`.
fn guard(dir: &Path, body: &str) -> Output {
    let script = format!(
        "set -euo pipefail\nsource '{}'\n{body}",
        repo().join("scripts/player-files-guard.sh").display()
    );
    with_player(Command::new("bash").arg("-c").arg(script), dir)
}

// spec: none (the safety net never writes the player's settings, profile or saves)
#[test]
fn the_guard_fails_when_a_player_file_gets_a_new_time() {
    let dir = scratch("player-guard");
    player(&dir);
    touch(&dir.join("users/default01.cfg"), 1_700_000_000);
    let cfg = dir.join("users/default01.cfg");
    let save = dir.join("saves/secret game name.sav");

    // Nothing changed: the check passes.
    let out = guard(&dir, "player_files_note\nplayer_files_check");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The profile rewritten with the same bytes: only its time differs.
    let body = format!(
        "player_files_note\ntouch -d @1700000100 '{}'\nplayer_files_check end-victory",
        cfg.display()
    );
    let out = guard(&dir, &body);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("end-victory wrote the player's own files"),
        "{stderr}"
    );
    assert!(stderr.contains("users: default01.cfg"), "{stderr}");
    assert_eq!(std::fs::read(&cfg).unwrap(), [1u8; 16]);

    // The settings file, and a save: the save's name is not printed.
    let body = format!(
        "player_files_note\ntouch '{}' '{}'\nplayer_files_check",
        dir.join("settings.txt").display(),
        save.display()
    );
    let out = guard(&dir, &body);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("the run wrote"), "{stderr}");
    assert!(stderr.contains("settings: settings.txt"), "{stderr}");
    assert!(stderr.contains("a file in the save folder"), "{stderr}");
    assert!(!stderr.contains("secret"), "{stderr}");

    // A file that appears (a profile's first backup) fails it too; the
    // check after compares with what the last one read.
    let body = format!(
        "player_files_note\ncp '{0}' '{0}.bak'\nplayer_files_check || echo failed\nplayer_files_check && echo then-quiet",
        cfg.display()
    );
    let out = guard(&dir, &body);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout.contains("failed") && stdout.contains("then-quiet"),
        "{stdout}{stderr}"
    );
    assert!(stderr.contains("users: default01.cfg.bak"), "{stderr}");

    // The guard never opens a file: one it cannot read is no obstacle.
    std::fs::set_permissions(&cfg, std::fs::Permissions::from_mode(0o000)).unwrap();
    let out = guard(&dir, "player_files_note\nplayer_files_check");
    std::fs::set_permissions(&cfg, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// spec: none (golden-images.sh gives each scene a profile folder of its own and fails a scene that writes the player's files)
#[test]
fn golden_images_give_each_scene_its_own_profile_folder() {
    if Command::new("wslpath").arg("-w").arg("/").output().is_err() {
        eprintln!("skipped: golden-images.sh runs from WSL (wslpath)");
        return;
    }
    let dir = scratch("golden-profile");
    let player_dir = dir.join("player");
    player(&player_dir);
    std::fs::create_dir_all(dir.join("game")).unwrap();
    // The stand-in viewer: notes its options, leaves a file in its profile
    // folder, writes the shot, and with WRITE_PLAYER set rewrites the
    // player's profile with the same bytes.
    let viewer = dir.join("viewer.exe");
    std::fs::write(
        &viewer,
        r#"#!/bin/bash
echo "$*" >> "$STAND_IN_ARGS"
while [ $# -gt 0 ]; do
  case "$1" in
    --profile-dir) mkdir -p "$(wslpath -u "$2")"; touch "$(wslpath -u "$2")/settings.txt"; shift ;;
    --shot) printf 'png' > "$(wslpath -u "$2")"; shift ;;
  esac
  shift
done
if [ -n "${WRITE_PLAYER:-}" ]; then touch "$WRITE_PLAYER"; fi
"#,
    )
    .unwrap();
    std::fs::set_permissions(&viewer, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out_dir = dir.join("out");
    let run = |write_player: bool| {
        let args = dir.join("args.txt");
        let _ = std::fs::remove_file(&args);
        let mut command = Command::new("bash");
        command
            .arg(repo().join("scripts/golden-images.sh"))
            .arg("menu-title")
            .env("NO_BUILD", "1")
            .env("S2_GAME_DIR", dir.join("game"))
            .env("S2_VIEWER_EXE", &viewer)
            .env("GOLDEN_IMAGES_OUT", &out_dir)
            .env("STAND_IN_ARGS", &args)
            .env("S2_RECORDING_FLAG", dir.join("no-flag"))
            .env("S2_SYNERGY_LOG", dir.join("no-synergy.log"))
            .env("S2_POINTER_PROBE", "echo free 1 1");
        if write_player {
            command.env("WRITE_PLAYER", player_dir.join("users/default01.cfg"));
        }
        let out = with_player(&mut command, &player_dir);
        (out, std::fs::read_to_string(&args).unwrap_or_default())
    };

    // A leftover of an earlier run is gone before the scene starts.
    let profile = out_dir.join("profile/menu-title");
    std::fs::create_dir_all(profile.join("users")).unwrap();
    std::fs::write(profile.join("users/default.cfg"), "old").unwrap();
    let (out, args) = run(false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let windows = |p: &Path| {
        let o = Command::new("wslpath").arg("-w").arg(p).output().unwrap();
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    assert!(
        args.contains(&format!("--profile-dir {}", windows(&profile))),
        "{args}"
    );
    // The menu scenes name their save folder: no second one.
    assert_eq!(args.matches("--save-dir").count(), 1, "{args}");
    assert!(
        !profile.join("users/default.cfg").exists(),
        "the old profile stayed"
    );
    assert!(profile.join("settings.txt").exists());
    // The seed profile (`s2-app profile-seed`), the same in every scene.
    let seed = std::fs::read(profile.join("users/default01.cfg")).expect("the seed");
    assert!(!seed.is_empty());
    assert!(!stderr.contains("player's own files"), "{stderr}");

    // A program that rewrites the player's profile fails the run.
    let (out, _) = run(true);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stderr.contains("menu-title wrote the player's own files")
            && stderr.contains("users: default01.cfg"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
