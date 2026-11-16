//! The play harness (`--play`): the real program driven with pointer,
//! clicks and keys through its own input systems, each step checked
//! against the game state. It needs the game data (`S2_GAME_DIR`) and a
//! display; without them it is skipped. `S2_PLAY_EXE` names another build
//! of the program to run (from WSL, the Windows one: a Linux build has no
//! display there), and the paths it gets are turned into Windows paths.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A path as the program under test expects it: Windows paths for a
/// Windows build run from WSL.
fn path_for(exe: &Path, path: &Path) -> String {
    let windows = exe
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        && cfg!(target_os = "linux");
    if windows {
        if let Ok(out) = Command::new("wslpath").arg("-w").arg(path).output() {
            if out.status.success() {
                return String::from_utf8_lossy(&out.stdout).trim().to_owned();
            }
        }
    }
    path.to_string_lossy().into_owned()
}

/// The game folder and the program to run, or `None` (skipped) without
/// them.
fn program() -> Option<(std::ffi::OsString, PathBuf)> {
    // `S2_PLAY_GAME_DIR`: a copy of the install where the program under
    // test reads it fast (`scripts/play-test.sh`: the Windows side's own
    // copy). Read over \\wsl.localhost, the game's first read of a sound,
    // a model or an animation stalls its frame by 30 to 150 ms, which
    // `panning_stays_smooth` would count against the program (2026-11-10).
    let game_dir = std::env::var_os("S2_PLAY_GAME_DIR").or_else(|| std::env::var_os("S2_GAME_DIR"));
    let Some(game_dir) = game_dir else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return None;
    };
    let exe = match std::env::var_os("S2_PLAY_EXE") {
        Some(exe) => PathBuf::from(exe),
        // A Linux build needs a display and its window libraries, which the
        // machines this runs on (WSL, CI) do not have: there the build to
        // drive is named explicitly (`scripts/play-test.sh`).
        None if cfg!(target_os = "linux") => {
            eprintln!("skipped: on Linux set S2_PLAY_EXE to the program to drive");
            return None;
        }
        None => PathBuf::from(env!("CARGO_BIN_EXE_s2-viewer")),
    };
    Some((game_dir, exe))
}

/// GFX-VIEW-2.3 (`docs/colour.md`): quads of known texels under the map's
/// light come out on screen as min(255, v x D), with no conversion between.
#[test]
fn colour_pipeline() {
    let Some((game_dir, exe)) = program() else {
        return;
    };
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("play");
    std::fs::create_dir_all(&out_dir).expect("the output directory");
    let report = out_dir.join("colour-probe.txt");
    let _ = std::fs::remove_file(&report);
    let status = Command::new(&exe)
        .args([
            "--game-dir",
            &path_for(&exe, Path::new(&game_dir)),
            "--free-game",
            "MP_2P_Storm_Coast",
            "--3d",
            "--size",
            "1024x768",
            "--colour-probe",
            &path_for(&exe, &report),
        ])
        .status()
        .expect("the program runs");
    let text = std::fs::read_to_string(&report).unwrap_or_else(|e| format!("no report: {e}"));
    eprintln!("{}\n{text}", exe.display());
    assert!(
        status.success() && text.contains("ok") && !text.contains("FAIL"),
        "the colour probe failed:\n{text}"
    );
}

/// Runs play script `script` with the program's start options `start`;
/// the report and step screenshots go to `target/tmp/play`, the saves
/// (and autosaves) to `target/tmp/play/saves`, never the player's folder.
fn run_script(script: &str, start: &[&str]) {
    let Some((game_dir, exe)) = program() else {
        return;
    };
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("play");
    std::fs::create_dir_all(&out_dir).expect("the output directory");
    let report = out_dir.join(format!("{script}.txt"));
    let _ = std::fs::remove_file(&report);
    let saves = path_for(&exe, &out_dir.join("saves"));
    let status = Command::new(&exe)
        .args(["--game-dir", &path_for(&exe, Path::new(&game_dir))])
        .args(start)
        .args(["--save-dir", &saves])
        .args([
            "--size",
            "1024x768",
            "--play",
            script,
            "--play-report",
            &path_for(&exe, &report),
            "--screenshot-dir",
            &path_for(&exe, &out_dir),
        ])
        .status()
        .expect("the program runs");
    let text = std::fs::read_to_string(&report).unwrap_or_else(|e| format!("no report: {e}"));
    eprintln!("{}\n{text}", exe.display());
    assert!(
        status.success() && !text.contains("FAIL"),
        "the play script failed (screenshots in {}):\n{text}",
        out_dir.display()
    );
}

// spec: UI-GAME-3.1, UI-GAME-3.2, UI-GAME-3.4, GFX-VIEW-3.2 (and every rule its steps in crates/viewer/src/tools/play/scripts.rs name)
#[test]
fn first_ten_minutes() {
    run_script(
        "first-ten-minutes",
        &["--free-game", "MP_2P_Storm_Coast", "--3d", "--speed", "3"],
    );
}

/// UI-MENU T17 (3.3a c): the main menu's EXIT asks first, No keeps the main
/// menu, EXIT closes the question again.
// spec: UI-MENU-3.3a (T17) (and every rule its steps in crates/viewer/src/tools/play/scripts.rs name)
#[test]
fn main_exit() {
    run_script("main-exit", &["--menu-screen", "main"]);
}

/// UI-MENU-2.1: Select Map, driven from the single-player page.
#[test]
fn select_map() {
    run_script("select-map", &["--menu-screen", "single"]);
}

/// CAM-5: lesson 1 from the single-player page, each instruction followed
/// with the clicks it asks for, to its end.
// spec: CAM-5.2, CAM-5.3, UI-MENU-1.6 (and every rule its steps in crates/viewer/src/tools/play/scripts.rs name)
#[test]
fn tutorial_one() {
    run_script(
        "tutorial-one",
        &["--menu-screen", "single", "--3d", "--speed", "3"],
    );
}

/// CAM-5: lesson 2 from the single-player page, each instruction followed with
/// the clicks it asks for, to its end.
// spec: CAM-5.1, CAM-5.2, CAM-5.3, UI-MENU-1.6
#[test]
fn tutorial_two() {
    run_script(
        "tutorial-two",
        &["--menu-screen", "single", "--3d", "--speed", "3"],
    );
}

/// CAM-5: lesson 3, to its attack and the military dialog.
// spec: CAM-5.1, CAM-5.2, CAM-5.3, UI-MENU-1.6
#[test]
fn tutorial_three() {
    run_script(
        "tutorial-three",
        &["--menu-screen", "single", "--3d", "--speed", "3"],
    );
}

/// CAM-5: lesson 4: a ship, an expedition, a harbour on the far shore.
// spec: CAM-5.1, CAM-5.2, CAM-5.3, CAM-5.4, UI-MENU-1.6
#[test]
fn tutorial_four() {
    run_script(
        "tutorial-four",
        &["--menu-screen", "single", "--3d", "--speed", "3"],
    );
}

/// CAM-3.5, 3.6, 4.2: mission 1 from the island map through its end screen
/// and debriefing to mission 2; the profile stays unwritten (`--play`).
// spec: CAM-3.5, CAM-3.6, CAM-4.2, CAM-4.4, UI-MENU-1.7 (and every rule its steps in crates/viewer/src/tools/play/scripts.rs name)
#[test]
fn mission_end() {
    run_script(
        "mission-end",
        &["--menu-screen", "single", "--3d", "--speed", "3"],
    );
}

/// GFX-VIEW-3.2 (F55): the camera panned at full speed over a late game at speed
/// 3: the program's frames stay short. Fails when panning gets slow again.
// spec: GFX-VIEW-3.2
#[test]
fn panning_stays_smooth() {
    run_script(
        "panning",
        &[
            "--map-name",
            "MP_2P_Storm_Coast",
            "--ai",
            "0=2",
            "--ai",
            "1=2",
            "--start-resources",
            "1",
            "--run",
            "100000",
            "--3d",
            "--speed",
            "3",
        ],
    );
}

/// CAM-3.5, SET-2.2, CAM-4.2: the real program writes the campaign state into
/// a profile folder of its own (never the player's), and a new start of the
/// program reads it back with mission 2 open.
// spec: CAM-3.5, CAM-3.7, CAM-4.2, SET-2.2 (T3, T9)
#[test]
fn campaign_state_is_written_and_read_back() {
    let Some((_, exe)) = program() else {
        return;
    };
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("play")
        .join("profile");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the profile folder");
    let own = path_for(&exe, &folder);
    run_script(
        "campaign-state-write",
        &[
            "--mission",
            "campaign01",
            "--3d",
            "--speed",
            "3",
            "--profile-dir",
            &own,
        ],
    );
    // The file the program wrote: SET-2.2, the map state keyed by its name.
    let file = folder.join("users").join("default01.cfg");
    let data = std::fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    assert!(
        data.windows(10).any(|w| w == b"campaign01"),
        "{} holds no map state of campaign01",
        file.display()
    );
    run_script(
        "campaign-state-read",
        &["--menu-screen", "single", "--profile-dir", &own],
    );
}

/// CAM-4.3, ECO-HQ-2.2: a mission after a free game on High holds what setting
/// 0 gives, not the free game's stock.
// spec: CAM-4.3, ECO-HQ-2.2 (Q-1002)
#[test]
fn mission_after_a_free_game_on_high() {
    run_script(
        "mission-after-free-game",
        &[
            "--free-game",
            "MP_2P_Storm_Coast",
            "--start-resources",
            "2",
            "--3d",
            "--speed",
            "3",
        ],
    );
}

/// CAM-3.2, 3.6: the defeat screen on a prepared world.
// spec: CAM-3.2, CAM-3.6 (T6)
#[test]
fn mission_defeat() {
    run_script(
        "mission-defeat",
        &["--mission", "campaign01", "--3d", "--speed", "3"],
    );
}

/// CAM-1.1, FMT-SAV-3.12: a mission saved and loaded by the real program.
// spec: CAM-1.1, FMT-SAV-3.12, UI-MENU-5.1, UI-MENU-5.2 (T8)
#[test]
fn mission_save_quit_load() {
    let saves = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("play")
        .join("saves");
    let _ = std::fs::remove_dir_all(&saves);
    std::fs::create_dir_all(&saves).expect("the save folder");
    run_script(
        "mission-save",
        &["--mission", "campaign01", "--3d", "--speed", "3"],
    );
    run_script("mission-load", &["--menu-screen", "single"]);
}

/// UI-MENU-3.1, 5.1, 5.2: save, quit, load. A free game is saved from the
/// game menu under a typed name, the autosave follows after its interval
/// (60 s of game time for the run), and the program ends; started again,
/// the save is loaded from the single-player page's LOAD GAME and the game
/// it gives has the state hashes (world and computer players) of the game
/// saved, then plays on; a third start's CONTINUE loads the autosave the
/// same way.
// spec: UI-MENU-3.1, UI-MENU-5.1, UI-MENU-5.2, UI-MENU-5.3, UI-MENU-1.5, FMT-SAV-5.1, FMT-SAV-5.2
#[test]
fn save_quit_load() {
    let saves = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("play")
        .join("saves");
    let _ = std::fs::remove_dir_all(&saves);
    std::fs::create_dir_all(&saves).expect("the save folder");
    run_script(
        "save-game",
        &[
            "--free-game",
            "MP_2P_Storm_Coast",
            "--3d",
            "--speed",
            "3",
            "--autosave",
            "60",
        ],
    );
    run_script("load-game", &["--menu-screen", "single"]);
    run_script("continue-game", &["--menu-screen", "single"]);
}
