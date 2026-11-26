//! `--probe-window <log>`: a second, tiny window of our own beside a timed
//! run (`scripts/long-play.sh`, `PROBE_WINDOW=1`): the same Bevy and wgpu
//! as the game, one triangle turning, held to the display's refresh, and a
//! log of its own frames with the wall clock. When the game's frame stalls
//! the probe says whether the graphics stack stalled for everyone (the
//! probe's frame stalls at the same wall-clock moment) or only for the game
//! (docs/test-coverage.md, "The long play"; 2026-11-24).
//!
//! The log's lines:
//!   T <unix ms> probe started
//!   F <unix ms> <ms>   a frame over 25 ms (its end; the frame's length)
//!   S <unix ms> <frames> frames, worst <ms>   a line a second
//! `--bench-seconds N` ends it after N seconds (default an hour), and so
//! does a file `<log>.stop`;
//! `--probe-at X,Y` places it (default -400,40: the left display here).

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use bevy::app::AppExit;
use bevy::prelude::*;

/// A probe frame over this is a line of its own.
const SLOW_MS: f64 = 25.0;

#[derive(Resource)]
struct Probe {
    log: std::io::BufWriter<std::fs::File>,
    last: Option<Instant>,
    started: Instant,
    seconds: f64,
    row_at: f64,
    row_frames: u32,
    row_worst: f64,
    stop: PathBuf,
}

#[derive(Component)]
struct Turning;

fn unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// Runs the probe window until its time is up or it is closed.
pub(crate) fn run(
    log: PathBuf,
    seconds: Option<u32>,
    at: Option<(i32, i32)>,
) -> std::process::ExitCode {
    let file = match std::fs::File::create(&log) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("--probe-window {}: {e}", log.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    let stop = PathBuf::from(format!("{}.stop", log.display()));
    let mut log = std::io::BufWriter::new(file);
    let _ = writeln!(log, "T {} probe started", unix_ms());
    let _ = log.flush();
    let (x, y) = at.unwrap_or((-400, 40));
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "s2 probe window".into(),
                resolution: (320, 180).into(),
                position: WindowPosition::At(IVec2::new(x, y)),
                present_mode: bevy::window::PresentMode::default(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Probe {
            log,
            last: None,
            started: Instant::now(),
            seconds: f64::from(seconds.unwrap_or(3600)),
            row_at: 0.0,
            row_frames: 0,
            row_worst: 0.0,
            stop,
        })
        .add_systems(Startup, setup)
        .add_systems(First, frame)
        .add_systems(Update, turn)
        .run();
    std::process::ExitCode::SUCCESS
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        Mesh2d(meshes.add(Triangle2d::new(
            Vec2::new(0.0, 60.0),
            Vec2::new(-52.0, -30.0),
            Vec2::new(52.0, -30.0),
        ))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.9, 0.6, 0.2)))),
        Transform::default(),
        Turning,
    ));
}

fn turn(time: Res<Time>, mut q: Query<&mut Transform, With<Turning>>) {
    for mut t in &mut q {
        t.rotate_z(time.delta_secs() * 1.5);
    }
}

fn frame(mut probe: ResMut<Probe>, mut exit: MessageWriter<AppExit>) {
    let now = Instant::now();
    let p = &mut *probe;
    if let Some(last) = p.last {
        let ms = (now - last).as_secs_f64() * 1000.0;
        p.row_frames += 1;
        p.row_worst = p.row_worst.max(ms);
        if ms > SLOW_MS {
            let _ = writeln!(p.log, "F {} {ms:.1}", unix_ms());
        }
    }
    p.last = Some(now);
    let at = (now - p.started).as_secs_f64();
    if at >= p.row_at + 1.0 {
        let _ = writeln!(
            p.log,
            "S {} {} frames, worst {:.1}",
            unix_ms(),
            p.row_frames,
            p.row_worst
        );
        let _ = p.log.flush();
        p.row_at = at;
        p.row_frames = 0;
        p.row_worst = 0.0;
        if p.stop.exists() {
            p.seconds = 0.0;
        }
    }
    if at >= p.seconds {
        let _ = p.log.flush();
        exit.write(AppExit::Success);
    }
}
