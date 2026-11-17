//! The harness scripts refuse to start while the original is being recorded
//! on the PC (`scripts/recording-guard.sh`, TOOL-trace TRACE-7.3). Each script
//! runs twice with a `cargo` of its own on the path that only leaves a mark:
//! with the flag file it must stop before its first build, without it it
//! must go on to that build.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `scripts/<script> <args>` with the stand-in `cargo` of `dir/bin`.
fn run(dir: &Path, script: &str, args: &[&str]) -> Output {
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
        .env("S2_GAME_DIR", dir.join("game"))
        .env("S2_SCREENS_DIR", dir.join("screens"))
        .env("CARGO_MARK", dir.join("cargo-ran"))
        // This desktop has the input (scripts/input-guard.sh,
        // crates/app/tests/input_guard.rs).
        .env("S2_SYNERGY_LOG", dir.join("no-synergy.log"))
        .env("S2_POINTER_PROBE", "echo free 1 1")
        .env_remove("NO_BUILD")
        .output()
        .unwrap()
}

// spec: TRACE-7.3
#[test]
fn harness_scripts_refuse_while_the_recording_flag_exists() {
    let dir = std::env::temp_dir().join(format!("s2-recording-guard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::create_dir_all(dir.join("game")).unwrap();
    let cargo = dir.join("bin/cargo");
    std::fs::write(&cargo, "#!/bin/sh\ntouch \"$CARGO_MARK\"\nexit 1\n").unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    let flag = dir.join("RECORDING-IN-PROGRESS");
    let mark = dir.join("cargo-ran");
    // The look comparison wants a shots folder with the original's side.
    let shots = dir.join("shots");
    std::fs::create_dir_all(shots.join("original")).unwrap();
    let shots = shots.display().to_string();
    let scripts: [(&str, &[&str]); 8] = [
        ("play-test.sh", &[]),
        ("golden-images.sh", &[]),
        ("view-map.sh", &[]),
        ("golden.sh", &[]),
        ("install-local.sh", &["--build"]),
        ("frame-bench.sh", &[]),
        ("look-compare.sh", &[shots.as_str()]),
        ("long-play.sh", &[]),
    ];
    for (script, args) in scripts {
        std::fs::write(&flag, "2026-10-23 23:40:00 pid 4242 E-trace-007\r\n").unwrap();
        let _ = std::fs::remove_file(&mark);
        let out = run(&dir, script, args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(75),
            "{script} with the flag: {stderr}"
        );
        assert!(!mark.exists(), "{script} built while the flag exists");
        assert!(stderr.contains("TRACE-7.3"), "{script}: {stderr}");
        assert!(
            stderr.contains("  2026-10-23 23:40:00 pid 4242 E-trace-007\n"),
            "{script}: {stderr}"
        );

        std::fs::remove_file(&flag).unwrap();
        let out = run(&dir, script, args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_ne!(
            out.status.code(),
            Some(75),
            "{script} without the flag: {stderr}"
        );
        assert!(
            mark.exists(),
            "{script} did not reach its build without the flag: {stderr}"
        );
    }
    // A flag older than a day still stops the script; the message says it
    // may be stale.
    std::fs::write(&flag, "2026-10-21 23:40:00 pid 4242 E-trace-007\n").unwrap();
    let two_days = std::time::Duration::from_secs(2 * 24 * 3600);
    let file = std::fs::File::options().write(true).open(&flag).unwrap();
    file.set_modified(std::time::SystemTime::now() - two_days)
        .unwrap();
    let _ = std::fs::remove_file(&mark);
    let out = run(&dir, "golden.sh", &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(75), "{stderr}");
    assert!(!mark.exists());
    assert!(
        stderr.contains("stale and may be removed by hand"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
