//! The scripts that drive the desktop's pointer refuse to start while this
//! desktop does not have the keyboard and mouse (`scripts/input-guard.sh`):
//! while Synergy's active screen is the other computer, or while the pointer
//! is held at the screen centre. Each script runs with a `cargo` of its own
//! on the path that only leaves a mark, with a Synergy log and a pointer
//! probe of the test's.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `scripts/<script> <args>` with the stand-in `cargo` of `dir/bin`,
/// the log `dir/synergy.log` and the probe answer `probe`.
fn run(dir: &Path, script: &str, args: &[&str], probe: &str) -> Output {
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("bash")
        .arg(repo().join("scripts").join(script))
        .args(args)
        .env("PATH", path)
        .env("S2_RECORDING_FLAG", dir.join("RECORDING-IN-PROGRESS"))
        .env("S2_SYNERGY_LOG", dir.join("synergy.log"))
        .env("S2_POINTER_PROBE", format!("echo {probe}"))
        .env("S2_GAME_DIR", dir.join("game"))
        .env("CARGO_MARK", dir.join("cargo-ran"))
        .env_remove("NO_BUILD")
        .output()
        .unwrap()
}

const SWITCHES: &str = "\
core       [2026-10-28T22:36:57] - INFO    - switch from \"surfacelaptop7-4c8e922c\" to \"desktopx9900-69204b96\" at -2550,121\r
core       [2026-10-28T22:36:57] - INFO    - entering screen\r
core       [2026-10-28T22:38:49] - INFO    - switch from \"desktopx9900-69204b96\" to \"surfacelaptop7-4c8e922c\" at 5119,736\r
core       [2026-10-28T22:38:49] - INFO    - leaving screen\r
";

// spec: none (our harness scripts wait while Synergy sends the input to another computer)
#[test]
fn pointer_scripts_refuse_while_another_computer_has_the_input() {
    let dir = std::env::temp_dir().join(format!("s2-input-guard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::create_dir_all(dir.join("game")).unwrap();
    let cargo = dir.join("bin/cargo");
    std::fs::write(&cargo, "#!/bin/sh\ntouch \"$CARGO_MARK\"\nexit 1\n").unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    let log = dir.join("synergy.log");
    let mark = dir.join("cargo-ran");
    let shots = dir.join("shots");
    std::fs::create_dir_all(shots.join("original")).unwrap();
    let shots = shots.display().to_string();
    let scripts: [(&str, &[&str]); 3] = [
        ("play-test.sh", &[]),
        ("golden-images.sh", &[]),
        ("look-compare.sh", &[shots.as_str()]),
    ];
    let refuses = |script: &str, args: &[&str], probe: &str| -> String {
        let _ = std::fs::remove_file(&mark);
        let out = run(&dir, script, args, probe);
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(75), "{script}: {stderr}");
        assert!(!mark.exists(), "{script} built without the input");
        stderr
    };
    let goes_on = |script: &str, args: &[&str], probe: &str| {
        let _ = std::fs::remove_file(&mark);
        let out = run(&dir, script, args, probe);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_ne!(out.status.code(), Some(75), "{script}: {stderr}");
        assert!(mark.exists(), "{script} did not reach its build: {stderr}");
    };
    for (script, args) in scripts {
        // The laptop is the active screen: the last screen event is leaving.
        std::fs::write(&log, SWITCHES).unwrap();
        let stderr = refuses(script, args, "free 5 5");
        assert!(stderr.contains("Synergy's active screen"), "{stderr}");
        assert!(
            stderr.contains("[2026-10-28T22:38:49] - INFO    - leaving screen\n"),
            "{stderr}"
        );

        // Back on this desktop.
        let back = format!(
            "{SWITCHES}core       [2026-10-29T07:44:34] - INFO    - jump from \"surfacelaptop7-4c8e922c\" to \"desktopx9900-69204b96\" at 1280,720\n\
             core       [2026-10-29T07:44:34] - INFO    - entering screen\n"
        );
        std::fs::write(&log, back).unwrap();
        goes_on(script, args, "free 5 5");

        // A server start begins on this desktop.
        let restarted = format!(
            "{SWITCHES}core       [2026-10-29T07:51:22] - NOTE    - started server, waiting for clients\n"
        );
        std::fs::write(&log, restarted).unwrap();
        goes_on(script, args, "free 5 5");

        // No Synergy at all.
        std::fs::remove_file(&log).unwrap();
        goes_on(script, args, "free 5 5");

        // The pointer stays at the centre after SetCursorPos.
        let stderr = refuses(script, args, "pinned 1280 720");
        assert!(
            stderr.contains("does not follow SetCursorPos (1280 720)"),
            "{stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
