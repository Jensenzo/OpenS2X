//! `s2-app shot --map M [--ai 1=1,...] [--resources R] --tick T --camera FILE
//! --size WxH --out PNG [--viewer EXE] [--explored]`: our picture of a game at
//! trace tick T with the camera the investigation read from the original (the
//! look comparison, TOOL-trace TRACE-8, `scripts/look-compare.sh`).
//! `--explored` draws every field as explored, for a picture the original
//! took with fog of war off (the viewer's `--no-fog`; the game is the same,
//! Q-290). `--paused` draws the paused view's layer over it (UI-GAME-1.11,
//! the viewer's `--pause-layer`), for a picture the original took paused.
//!
//! The game is the headless one of `trace` and `hash` (same start, same
//! state). The picture is drawn by the viewer: `--compare` (paused, the
//! clocks standing still, no interface), the game played to tick T, the 3D
//! camera from the camera file (`--camera`), `--shot`. The viewer is
//! `--viewer`, else `S2_VIEWER`, else `s2-viewer` next to this program; a
//! Windows `.exe` run from WSL gets its paths through `wslpath -w`. The
//! viewer's game at T must be the headless game, part by part of the
//! dynamic state (`World::state_hashes`; the full hash also holds the rules
//! and the map's mission, which the viewer sets), or the shot is refused:
//! the picture is then not of the game the trace holds.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::game::Start;

pub fn run(args: &[String]) -> ExitCode {
    match shot(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("shot: {e}");
            ExitCode::FAILURE
        }
    }
}

fn shot(args: &[String]) -> Result<(), String> {
    let mut start = Start::new("", s2_sim::rng::SIM_SEED as u64);
    let rest = start.options(args)?;
    let (mut tick, mut camera, mut size, mut out, mut viewer) = (None, None, None, None, None);
    let mut explored = false;
    let mut paused = false;
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{arg} wants a value"));
        match arg.as_str() {
            "--map" => start.map = value()?,
            "--tick" => {
                let v = value()?;
                tick = Some(v.parse::<u64>().map_err(|_| format!("not a number: {v}"))?);
            }
            "--camera" => camera = Some(PathBuf::from(value()?)),
            "--size" => size = Some(value()?),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--viewer" => viewer = Some(PathBuf::from(value()?)),
            "--explored" => explored = true,
            "--paused" => paused = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if start.map.is_empty() {
        return Err("--map wanted".into());
    }
    let tick = tick.ok_or("--tick wanted")?;
    let camera = camera.ok_or("--camera wanted")?;
    let out = out.ok_or("--out wanted")?;
    // The headless game at T: the hash the viewer's game must have.
    let mut runner = start.begin()?;
    while runner.world.tick() < tick {
        runner.step();
    }
    let want: Vec<String> = runner
        .world
        .state_hashes()
        .iter()
        .map(|(name, hash)| format!("{name}={hash:016x}"))
        .collect();
    let want = want.join(" ");
    let viewer = viewer
        .or_else(|| std::env::var_os("S2_VIEWER").map(PathBuf::from))
        .or_else(|| {
            let me = std::env::current_exe().ok()?;
            Some(me.with_file_name("s2-viewer"))
        })
        .ok_or("no viewer: give --viewer or S2_VIEWER")?;
    let windows = viewer
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    let path = |p: &Path| -> Result<String, String> {
        if !windows {
            return Ok(p.display().to_string());
        }
        let absolute = std::path::absolute(p).map_err(|e| e.to_string())?;
        let converted = Command::new("wslpath")
            .arg("-w")
            .arg(&absolute)
            .output()
            .map_err(|e| format!("wslpath: {e}"))?;
        Ok(String::from_utf8_lossy(&converted.stdout)
            .trim()
            .to_string())
    };
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut command = Command::new(&viewer);
    if let Some(dir) = std::env::var_os("S2_GAME_DIR") {
        command.arg("--game-dir").arg(path(Path::new(&dir))?);
    }
    command
        .arg("--compare")
        .arg(&start.map)
        .arg("--start-resources")
        .arg(start.resources.to_string())
        .arg("--run")
        .arg(tick.to_string())
        // GFX-VIEW-2.3c: the day clock at T, started at hour 12 with the
        // game and advanced 0.033 hours per game second (10 ticks).
        .arg("--hour")
        .arg(format!("{}", 12.0 + tick as f64 / 10.0 * 0.033))
        .arg("--camera")
        .arg(path(&camera)?)
        .arg("--shot")
        .arg(path(&out)?);
    for (player, profile) in start.slots.iter().flatten() {
        command.arg("--ai").arg(format!("{}={profile}", player.0));
    }
    if let Some(size) = size {
        command.arg("--size").arg(size);
    }
    if explored {
        command.arg("--no-fog");
    }
    if paused {
        command.arg("--pause-layer");
    }
    let done = command
        .output()
        .map_err(|e| format!("{}: {e}", viewer.display()))?;
    let log =
        String::from_utf8_lossy(&done.stdout).to_string() + &String::from_utf8_lossy(&done.stderr);
    if !done.status.success() || !out.exists() {
        return Err(format!("the viewer made no picture:\n{log}"));
    }
    // The viewer logs "played to tick T, state hash H, parts P"
    // (`game::load`).
    let got = log
        .lines()
        .find_map(|l| l.split(", parts ").nth(1))
        .map(|h| h.trim().to_string());
    if got.as_deref() != Some(want.as_str()) {
        let _ = std::fs::remove_file(&out);
        let got = got.unwrap_or_else(|| "not logged".into());
        let differing: Vec<&str> = got
            .split(' ')
            .zip(want.split(' '))
            .filter(|(a, b)| a != b)
            .map(|(_, b)| b.split('=').next().unwrap_or(b))
            .collect();
        return Err(format!(
            "the viewer's game at tick {tick} is not the headless one: differing parts {} \
             (viewer {got}; headless {want})",
            differing.join(", ")
        ));
    }
    println!("{}: tick {tick}, state {want}", out.display());
    Ok(())
}
