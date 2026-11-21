//! `--bench <frames>`: the frame-time benchmark (F51, docs/test-coverage.md).
//!
//! Once the 3D scene of the map is built and has been drawn for a while, the
//! next `frames` frames are timed: the whole frame (from one frame's start
//! to the next) and the parts of the map's update between the marks below,
//! each in milliseconds; and the render side, pass by pass, from Bevy's
//! render diagnostics (CPU and, where the device has timestamp queries,
//! GPU time; the mean of the last frames Bevy keeps). The report goes to
//! `--bench-report` (a text file of `key = value` lines), then the program
//! quits.
//!
//! `--bench-pan` (F55): while the frames are timed the arrow keys are held, as
//! a player's hand does, so the camera crosses the map back and forth at full
//! speed. The report then also has the worst frame, the 99th percentile and
//! the parts of the slowest frames.
//!
//! `--bench-seconds`, `--bench-log`, `--bench-invariants` (the long play): as
//! long as a sitting, with a line a second and one per slow frame written as
//! the run goes, and every soak invariant after every tick.

use std::time::Instant;

use bevy::app::AppExit;
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;

/// Frames drawn after the scene is built before the timing starts.
pub(crate) const WARM_UP_FRAMES: u32 = 120;

/// A frame over this is logged with its parts (`--bench-log`).
const SLOW_FRAME_MS: f64 = 33.3;

/// The named spans inside a frame's parts, in ms (the road rebuild, the list
/// of objects): what a slow frame was doing (F55). Filled by the systems that
/// time themselves, emptied at the frame's end.
#[derive(Resource, Default)]
pub(crate) struct FrameSpans {
    pub(crate) now: Vec<(&'static str, f64)>,
    /// The invariants' time in this frame, left out of the frame's length.
    pub(crate) check_ms: f64,
}

impl FrameSpans {
    pub(crate) fn add(&mut self, name: &'static str, since: Instant) {
        self.now
            .push((name, (Instant::now() - since).as_secs_f64() * 1000.0));
    }
}

/// `--bench-invariants`: the soak's checker over the real program's game.
#[derive(Resource, Default)]
pub(crate) struct RunCheck {
    checker: Option<s2_sim::invariants::Checker>,
    pub(crate) broken: Vec<String>,
    pub(crate) ticks: u64,
    pub(crate) seconds: f64,
}

impl RunCheck {
    /// After a tick of the game.
    pub(crate) fn after_tick(&mut self, world: &s2_sim::World, spans: Option<&mut FrameSpans>) {
        let start = Instant::now();
        let checker = self
            .checker
            .get_or_insert_with(|| s2_sim::invariants::Checker::new(world));
        for v in checker.check(world) {
            self.broken
                .push(format!("tick {}: {}: {}", v.tick, v.invariant, v.detail));
        }
        self.ticks += 1;
        let spent = (Instant::now() - start).as_secs_f64();
        self.seconds += spent;
        if let Some(spans) = spans {
            spans.check_ms += spent * 1000.0;
        }
    }
}

/// This process's memory in MB: working set and private bytes.
#[cfg(windows)]
fn memory_mb() -> (f64, f64) {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut c: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    // SAFETY: `c` is a valid struct of the size given, the handle this
    // process's own pseudo handle.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), (&raw mut c).cast(), c.cb) };
    if ok == 0 {
        return (0.0, 0.0);
    }
    const MB: f64 = 1024.0 * 1024.0;
    (c.WorkingSetSize as f64 / MB, c.PrivateUsage as f64 / MB)
}

#[cfg(not(windows))]
fn memory_mb() -> (f64, f64) {
    (0.0, 0.0)
}

/// The long run's clock and log.
struct LongRun {
    seconds: Option<f64>,
    log: Option<std::io::BufWriter<std::fs::File>>,
    started: Option<Instant>,
    /// Where the second being logged began: time, frame, tick.
    row_at: f64,
    row_frame: usize,
    row_tick: u64,
    memory: Vec<(f64, f64, f64)>,
    /// Ticks per second, a sample a second.
    rates: Vec<f64>,
    /// Frames over 100 ms: a line each.
    stalls: Vec<String>,
    /// F55: when the palette slots were last counted by owner.
    slots_at: f64,
    /// `ModelLibrary::loads` taken so far: a later entry during the timed
    /// frames is a first use during play (`models3d::preload`).
    loads_seen: usize,
    /// The first uses during play: the frame each came in and what.
    first_uses: Vec<(usize, String)>,
}

/// The frames before a slow frame whose first uses it names: the render
/// world prepares a frame one frame behind the main world, and a new model's
/// textures, meshes and materials can take a few frames to reach the
/// graphics card.
pub(crate) const FIRST_USE_FRAMES: usize = 8;

/// The first uses (`(frame, what)`, in frame order) that came in `frame` or
/// the [`FIRST_USE_FRAMES`] frames before it, for a slow frame's line: the
/// object a first-use stall is of.
pub(crate) fn first_uses_near(first_uses: &[(usize, String)], frame: usize) -> Vec<&str> {
    first_uses
        .iter()
        .filter(|(f, _)| *f <= frame && frame - *f <= FIRST_USE_FRAMES)
        .map(|(_, what)| what.as_str())
        .collect()
}

/// The parts of the map's update, between consecutive marks.
pub(crate) const PARTS: [&str; 6] = [
    "input and camera",
    "interface",
    "game and computer players",
    "scene objects",
    "settlers, buildings, wares, animation",
    "drawing lists and labels",
];

#[derive(Resource)]
pub(crate) struct Bench {
    frames: u32,
    report: Option<std::path::PathBuf>,
    /// Frames seen since the scene was built.
    seen: u32,
    frame_start: Option<Instant>,
    /// Each timed frame's length, ms.
    frame_ms: Vec<f64>,
    /// Where the last mark was set this frame.
    last_mark: Option<(usize, Instant)>,
    /// Each part's time per timed frame, ms.
    part_ms: Vec<Vec<f64>>,
    ticks_at_start: u64,
    timing: bool,
    /// `--bench-pan`: the keys are held while the frames are timed.
    pan: Option<Pan>,
    placements_at_start: u64,
    /// Where the last mark of the frame was set, for the time after the
    /// map's update up to the end of the main world's schedule.
    update_end: Option<Instant>,
    /// Each timed frame: PostUpdate (transforms, visibility, lights, layout).
    post_ms: Vec<f64>,
    long: LongRun,
}

/// `--bench-pan`: the arrow key held, and how the camera answers.
pub(crate) struct Pan {
    held: Option<KeyCode>,
    /// The way across the map: a long leg to the rim, then a short step down
    /// and back the other way.
    leg: usize,
    /// Frames of the short leg left.
    step_frames: u32,
    /// Where the rig's target was last frame.
    last_target: [f32; 2],
    /// Frames in a row the camera did not move.
    stuck: u32,
    /// The distance the camera moved, in game units.
    pub(crate) distance: f32,
    pointer_set: bool,
    /// `--bench-drag`: pixels the mouse moves per frame with the middle button
    /// held, in place of the keys.
    drag: Option<f32>,
}

impl Pan {
    fn new(drag: Option<f32>) -> Self {
        Self {
            drag,
            held: None,
            leg: 0,
            step_frames: 0,
            last_target: [0.0; 2],
            stuck: 0,
            distance: 0.0,
            pointer_set: false,
        }
    }

    /// The key the next frame holds, given where the camera is now: along a
    /// long leg until the rim stops it, then a short leg to the next row.
    fn next_key(&mut self, target: [f32; 2]) -> KeyCode {
        let moved = (target[0] - self.last_target[0]).hypot(target[1] - self.last_target[1]);
        self.distance += moved;
        self.last_target = target;
        self.stuck = if moved < 1e-3 { self.stuck + 1 } else { 0 };
        const STEP_FRAMES: u32 = 40;
        if self.step_frames > 0 {
            self.step_frames -= 1;
            return KeyCode::ArrowDown;
        }
        if self.stuck >= 3 {
            self.stuck = 0;
            self.leg += 1;
            self.step_frames = STEP_FRAMES;
            return KeyCode::ArrowDown;
        }
        if self.leg.is_multiple_of(2) {
            KeyCode::ArrowRight
        } else {
            KeyCode::ArrowLeft
        }
    }
}

impl Bench {
    pub(crate) fn new(
        frames: u32,
        report: Option<std::path::PathBuf>,
        pan: bool,
        drag: Option<f32>,
        seconds: Option<u32>,
        log: Option<std::path::PathBuf>,
    ) -> Self {
        let log = log.and_then(|path| match std::fs::File::create(&path) {
            Ok(file) => Some(std::io::BufWriter::new(file)),
            Err(e) => {
                error!("--bench-log {}: {e}", path.display());
                None
            }
        });
        Self {
            long: LongRun {
                seconds: seconds.map(f64::from),
                log,
                started: None,
                row_at: 0.0,
                row_frame: 0,
                row_tick: 0,
                memory: Vec::new(),
                rates: Vec::new(),
                stalls: Vec::new(),
                slots_at: 0.0,
                loads_seen: 0,
                first_uses: Vec::new(),
            },
            pan: (pan || drag.is_some()).then(|| Pan::new(drag)),
            placements_at_start: 0,
            update_end: None,
            post_ms: Vec::new(),
            frames,
            report,
            seen: 0,
            frame_start: None,
            frame_ms: Vec::new(),
            last_mark: None,
            part_ms: vec![Vec::new(); PARTS.len()],
            ticks_at_start: 0,
            timing: false,
        }
    }
}

/// What a long run reads besides the frame's parts.
type RunState<'w> = (
    Option<ResMut<'w, FrameSpans>>,
    Option<Res<'w, RunCheck>>,
    Res<'w, crate::game::GameClock>,
    Res<'w, crate::render::models3d::ModelLibrary>,
);

/// F55: who holds the palette slots (the animator's poses, the buildings).
type SlotOwners<'w> = (
    Res<'w, crate::animation::animate3d::Animator>,
    Res<'w, crate::render::view3d::BuildingAnims>,
);

/// At each frame's start: the length of the frame before, and whether this
/// one is timed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn bench_frame(
    bench: Option<ResMut<Bench>>,
    viewer: Res<crate::game::Viewer>,
    view: Res<crate::render::view3d::View3d>,
    diagnostics: Res<DiagnosticsStore>,
    render_times: Res<RenderTimes>,
    mut sounds: ResMut<crate::audio::SoundStats>,
    placements: Res<crate::render::view3d::Placements>,
    (mut spans, run_check, clock, library): RunState,
    (animator, building_anims): SlotOwners,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut bench) = bench else { return };
    let now = Instant::now();
    if bench.timing {
        if let Some(start) = bench.frame_start {
            let check_ms = spans.as_ref().map_or(0.0, |s| s.check_ms);
            let ms = (now - start).as_secs_f64() * 1000.0 - check_ms;
            bench.frame_ms.push(ms);
            let tick = viewer.world.as_ref().map_or(0, |w| w.tick());
            log_first_uses(&mut bench, &library.loads, tick);
            log_frame(
                &mut bench,
                ms,
                tick,
                clock.ticks,
                spans.as_deref(),
                &render_times,
            );
            log_second(
                &mut bench,
                tick,
                run_check.as_deref(),
                library.palettes.capacity(),
            );
            log_slots(&mut bench, &library, &animator, &building_anims);
        }
    }
    if let Some(spans) = spans.as_mut() {
        spans.now.clear();
        spans.check_ms = 0.0;
    }
    bench.frame_start = Some(now);
    bench.last_mark = None;
    if !view.is_built(viewer.loads) {
        return;
    }
    bench.seen += 1;
    if bench.seen == WARM_UP_FRAMES {
        bench.timing = true;
        render_times.0.lock().expect("bench lock").timing = true;
        bench.ticks_at_start = viewer.world.as_ref().map_or(0, |w| w.tick());
        bench.long.row_tick = bench.ticks_at_start;
        bench.long.started = Some(now);
        *sounds = default();
        bench.placements_at_start = placements.full_placements;
        // What the warm-up's frames read is not play (the timed frames).
        bench.long.loads_seen = library.loads.len().max(view.loads_before_play);
    }
    let time_up = bench.timing
        && bench
            .long
            .seconds
            .zip(bench.long.started)
            .is_some_and(|(limit, at)| (now - at).as_secs_f64() >= limit);
    if bench.frame_ms.len() >= bench.frames as usize || time_up {
        let ticks = viewer.world.as_ref().map_or(0, |w| w.tick()) - bench.ticks_at_start;
        let mut text = report(&bench, ticks, viewer.world.as_ref());
        text += &pan_report(&bench, &render_times);
        text += &long_report(&bench, run_check.as_deref());
        text += &sound_report(&bench, &sounds);
        text += &format!(
            "full-object-placements = {}\n",
            placements.full_placements - bench.placements_at_start
        );
        text += &render_parts_report(&render_times);
        text += &render_report(&diagnostics);
        match &bench.report {
            Some(path) => {
                if let Err(e) = std::fs::write(path, &text) {
                    error!("--bench-report {}: {e}", path.display());
                }
            }
            None => info!("{text}"),
        }
        exit.write(AppExit::Success);
        bench.timing = false;
        bench.frames = u32::MAX;
    }
}

/// `--bench-pan`: holds the arrow key of the next leg while the frames are
/// timed, through the program's own keyboard input.
pub(crate) fn bench_pan(
    bench: Option<ResMut<Bench>>,
    view: Res<crate::render::view3d::View3d>,
    mut window: Single<(Entity, &mut Window), With<bevy::window::PrimaryWindow>>,
    mut keys: MessageWriter<bevy::input::keyboard::KeyboardInput>,
    mut buttons: MessageWriter<bevy::input::mouse::MouseButtonInput>,
    mut motion: MessageWriter<bevy::input::mouse::MouseMotion>,
) {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::ButtonState;
    let Some(mut bench) = bench else { return };
    if !bench.timing {
        return;
    }
    let Some(pan) = bench.pan.as_mut() else {
        return;
    };
    // The pointer over the map, as a player's: what lies under it changes
    // with the camera (F55).
    let (entity, window) = &mut *window;
    let entity = *entity;
    if !pan.pointer_set {
        pan.pointer_set = true;
        let middle = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(middle));
    }
    let target = [view.rig.target[0], view.rig.target[2]];
    let wanted = pan.next_key(target);
    // `--bench-drag`: the middle button held and the mouse thrown across the
    // desk: far faster than the keys move the camera (GFX-VIEW-3.2).
    if let Some(pixels) = pan.drag {
        if pan.held.is_none() {
            buttons.write(bevy::input::mouse::MouseButtonInput {
                button: MouseButton::Middle,
                state: ButtonState::Pressed,
                window: entity,
            });
            pan.held = Some(KeyCode::F24);
        }
        let delta = match wanted {
            KeyCode::ArrowRight => Vec2::new(-pixels, 0.0),
            KeyCode::ArrowLeft => Vec2::new(pixels, 0.0),
            _ => Vec2::new(0.0, -pixels / 4.0),
        };
        motion.write(bevy::input::mouse::MouseMotion { delta });
        return;
    }
    if pan.held == Some(wanted) {
        return;
    }
    let mut send = |key_code, state| {
        keys.write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window: entity,
        });
    };
    if let Some(old) = pan.held {
        send(old, ButtonState::Released);
    }
    send(wanted, ButtonState::Pressed);
    pan.held = Some(wanted);
}

/// After PostUpdate: how long transforms, visibility, lights and layout took.
pub(crate) fn bench_post(bench: Option<ResMut<Bench>>) {
    let Some(mut bench) = bench else { return };
    if !bench.timing {
        return;
    }
    if let Some(at) = bench.update_end.take() {
        bench
            .post_ms
            .push((Instant::now() - at).as_secs_f64() * 1000.0);
    }
}

/// The mark before part `N` (N = PARTS.len(): after the last part).
pub(crate) fn mark<const N: usize>(bench: Option<ResMut<Bench>>) {
    let Some(mut bench) = bench else { return };
    let now = Instant::now();
    if N == PARTS.len() {
        bench.update_end = Some(now);
    }
    if bench.timing {
        if let Some((part, at)) = bench.last_mark {
            if part + 1 == N && part < PARTS.len() {
                let ms = (now - at).as_secs_f64() * 1000.0;
                bench.part_ms[part].push(ms);
            }
        }
    }
    bench.last_mark = Some((N, now));
}

/// The parts of the render world's work (CPU) in a frame, between the marks
/// of [`render_mark`], each between two of Bevy's render system sets.
pub(crate) const RENDER_PARTS: [&str; 6] = [
    "assets",
    "views and specialize",
    "queue",
    "phase sort",
    "prepare",
    "render graph",
];

/// The render world's part times, shared with the main world's bench (the
/// render world runs on its own thread).
#[derive(Resource, Clone, Default)]
pub(crate) struct RenderTimes(pub(crate) std::sync::Arc<std::sync::Mutex<RenderTiming>>);

#[derive(Default)]
pub(crate) struct RenderTiming {
    pub(crate) timing: bool,
    last: Option<(usize, Instant)>,
    part_ms: [Vec<f64>; RENDER_PARTS.len()],
}

/// The mark before render part `N` (N = RENDER_PARTS.len(): after the last).
pub(crate) fn render_mark<const N: usize>(times: Res<RenderTimes>) {
    let now = Instant::now();
    let mut t = times.0.lock().expect("bench lock");
    if t.timing {
        if let Some((part, at)) = t.last {
            if part + 1 == N && part < RENDER_PARTS.len() {
                t.part_ms[part].push((now - at).as_secs_f64() * 1000.0);
            }
        }
    }
    t.last = Some((N, now));
}

/// The render world's parts: mean, median and 95th percentile.
fn render_parts_report(times: &RenderTimes) -> String {
    let t = times.0.lock().expect("bench lock");
    let mut text = String::new();
    for (name, values) in RENDER_PARTS.iter().zip(&t.part_ms) {
        let (mean, median, p95) = stats(values);
        let key = name.replace(' ', "-");
        text += &format!("render-world-{key}-ms = {mean:.2} (median {median:.2}, p95 {p95:.2})\n");
    }
    text
}

/// F56: the sounds started per second and the most playing at once.
fn sound_report(bench: &Bench, sounds: &crate::audio::SoundStats) -> String {
    let seconds = bench.frame_ms.iter().sum::<f64>() / 1000.0;
    let total: u32 = sounds.started.values().sum();
    let mut by_count: Vec<_> = sounds.started.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let per_second = |n: u32| f64::from(n) / seconds.max(0.001);
    let mut text = format!(
        "sounds-per-second = {:.1}\nsounds-most-at-once = {}\n\
         sounds-refused-per-second = {:.1}\nsounds-cut-per-second = {:.1}\n",
        per_second(total),
        sounds.most_at_once,
        per_second(sounds.refused),
        per_second(sounds.cut),
    );
    for (name, n) in by_count.iter().take(8) {
        text += &format!("sounds-of {name} = {n}\n");
    }
    text
}

/// A frame over [`SLOW_FRAME_MS`]: a line with what it did. One over 100 ms
/// is also kept for the report.
fn log_frame(
    bench: &mut Bench,
    ms: f64,
    tick: u64,
    ticks: u32,
    spans: Option<&FrameSpans>,
    times: &RenderTimes,
) {
    if ms <= SLOW_FRAME_MS {
        return;
    }
    let at = bench
        .long
        .started
        .map_or(0.0, |s| (Instant::now() - s).as_secs_f64());
    let mut line = format!(
        "F {at:.2} s, frame {}, tick {tick} (+{ticks}): {ms:.1} ms:",
        bench.frame_ms.len() - 1
    );
    let i = bench.frame_ms.len() - 1;
    for (name, values) in PARTS.iter().zip(&bench.part_ms) {
        if let Some(part) = values.get(i).filter(|&&p| p >= 1.0) {
            line += &format!(" {name} {part:.1},");
        }
    }
    if let Some(part) = bench.post_ms.get(i).filter(|&&p| p >= 1.0) {
        line += &format!(" post update {part:.1},");
    }
    if let Some(spans) = spans {
        for (name, span) in spans.now.iter().filter(|s| s.1 >= 0.5) {
            line += &format!(" [{name} {span:.1}],");
        }
    }
    let t = times.0.lock().expect("bench lock");
    for (name, values) in RENDER_PARTS.iter().zip(&t.part_ms) {
        if let Some(part) = values.last().filter(|&&p| p >= 1.0) {
            line += &format!(" render {name} {part:.1},");
        }
    }
    drop(t);
    // A first use in this frame or just before: the object of a first-use
    // stall (`models3d::preload`).
    for what in first_uses_near(&bench.long.first_uses, i) {
        line += &format!(" [first use: {what}],");
    }
    if ms > 100.0 {
        bench.long.stalls.push(line.clone());
    }
    if let Some(log) = bench.long.log.as_mut() {
        use std::io::Write;
        let _ = writeln!(log, "{line}");
    }
}

/// The models and clips the library read since the last frame, while the
/// frames are timed: a line each (`U`), kept for the slow frames' lines.
fn log_first_uses(bench: &mut Bench, loads: &[String], tick: u64) {
    let seen = bench.long.loads_seen.min(loads.len());
    bench.long.loads_seen = loads.len();
    if !bench.timing || seen == loads.len() {
        return;
    }
    let frame = bench.frame_ms.len().saturating_sub(1);
    let at = bench
        .long
        .started
        .map_or(0.0, |s| (Instant::now() - s).as_secs_f64());
    for what in &loads[seen..] {
        if let Some(log) = bench.long.log.as_mut() {
            use std::io::Write;
            let _ = writeln!(
                log,
                "U {at:.2} s, frame {frame}, tick {tick}: first use of {what}"
            );
        }
        bench.long.first_uses.push((frame, what.clone()));
    }
}

/// A line a second: the frames, the ticks and the memory.
fn log_second(bench: &mut Bench, tick: u64, check: Option<&RunCheck>, palette_matrices: usize) {
    let Some(started) = bench.long.started else {
        return;
    };
    let at = (Instant::now() - started).as_secs_f64();
    let long = &mut bench.long;
    if at < long.row_at + 1.0 {
        return;
    }
    let seconds = at - long.row_at;
    let mut frames = bench.frame_ms[long.row_frame..].to_vec();
    frames.sort_by(f64::total_cmp);
    let of = |q: f64| {
        frames
            .get(((frames.len().max(1) - 1) as f64 * q).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let (working, private) = memory_mb();
    let rate = (tick - long.row_tick) as f64 / seconds;
    if long.memory.is_empty() || long.memory.last().is_some_and(|m| at - m.0 >= 10.0) {
        long.memory.push((at, working, private));
    }
    long.rates.push(rate);
    let broken = check.map_or(0, |c| c.broken.len());
    if let Some(log) = long.log.as_mut() {
        use std::io::Write;
        let _ = writeln!(
            log,
            "S {at:.0} s, tick {tick}, {} frames, median {:.1}, p99 {:.1}, worst {:.1} ms, {rate:.1} ticks/s, memory {working:.0} MB (private {private:.0}), palettes {:.1} MB, invariants broken {broken}",
            frames.len(),
            of(0.5),
            of(0.99),
            of(1.0),
            (palette_matrices * 64) as f64 / (1024.0 * 1024.0)
        );
        let _ = log.flush();
    }
    long.row_at = at;
    long.row_frame = bench.frame_ms.len();
    long.row_tick = tick;
}

/// F55 (the long play): a line a minute: the palette slots taken and free,
/// and who holds them: the skinned parts' rest slots, the poses' parts
/// (with the most one pose holds), the buildings' own parts, and what is
/// left (nobody counted should hold any).
fn log_slots(
    bench: &mut Bench,
    library: &crate::render::models3d::ModelLibrary,
    animator: &crate::animation::animate3d::Animator,
    buildings: &crate::render::view3d::BuildingAnims,
) {
    let Some(started) = bench.long.started else {
        return;
    };
    let at = (Instant::now() - started).as_secs_f64();
    let long = &mut bench.long;
    if at < long.slots_at + 60.0 {
        return;
    }
    long.slots_at = at;
    let (taken, free) = library.palettes.slots();
    let rest = library.rest_slot_count();
    let poses = animator.slot_counts();
    let own = buildings.slot_count();
    let other = taken as i64 - (rest + poses.slots + poses.pending + own) as i64;
    if let Some(log) = long.log.as_mut() {
        use std::io::Write;
        let _ = writeln!(
            log,
            "P {at:.0} s, palette slots {taken} taken, {free} free: rest {rest}, poses {} in {} poses (most {} in one, {} given back next frame), buildings {own}, other {other}",
            poses.slots, poses.poses, poses.most, poses.pending
        );
        let _ = log.flush();
    }
}

/// The long run's summary: ticks held, memory, invariants, stalls.
fn long_report(bench: &Bench, check: Option<&RunCheck>) -> String {
    let long = &bench.long;
    let mut text = String::new();
    if !long.rates.is_empty() {
        let mut rates = long.rates.clone();
        rates.sort_by(f64::total_cmp);
        // A minute's mean at its lowest: one second can hold a stall's catching up.
        let minutes: Vec<f64> = long
            .rates
            .chunks(60)
            .map(|c| c.iter().sum::<f64>() / c.len() as f64)
            .collect();
        text += &format!(
            "ticks-per-second-lowest-second = {:.1}\nticks-per-second-lowest-minute = {:.1}\n",
            rates[0],
            minutes.iter().copied().fold(f64::MAX, f64::min)
        );
    }
    if let (Some(first), Some(last)) = (long.memory.first(), long.memory.last()) {
        let peak = long.memory.iter().map(|m| m.2).fold(0.0, f64::max);
        text += &format!(
            "memory-private-mb-start = {:.0}\nmemory-private-mb-end = {:.0}\nmemory-private-mb-peak = {peak:.0}\nmemory-working-mb-end = {:.0}\n",
            first.2, last.2, last.1
        );
        for m in long.memory.iter().step_by(30) {
            text += &format!(
                "memory-at {:.0} s = {:.0} MB private, {:.0} working\n",
                m.0, m.2, m.1
            );
        }
    }
    if let Some(check) = check {
        text += &format!(
            "invariant-ticks = {}\ninvariant-seconds = {:.2}\ninvariants-broken = {}\n",
            check.ticks,
            check.seconds,
            check.broken.len()
        );
        for line in check.broken.iter().take(40) {
            text += &format!("broken {line}\n");
        }
    }
    text += &format!(
        "frames-over-100ms = {}\n",
        bench.frame_ms.iter().filter(|&&m| m > 100.0).count()
    );
    text += &format!("first-uses-in-play = {}\n", long.first_uses.len());
    for line in &long.stalls {
        text += &format!("stall {line}\n");
    }
    text
}

/// The distance panned, the worst frame, the 99th percentile and the parts of
/// the slowest frames (F55). The render world runs one frame behind, so a slow
/// frame's render parts are those of its frame and the one before.
fn pan_report(bench: &Bench, times: &RenderTimes) -> String {
    let t = times.0.lock().expect("bench lock");
    let mut text = match &bench.pan {
        Some(pan) => format!("pan-distance = {:.0}\n", pan.distance),
        None => String::new(),
    };
    let mut order: Vec<usize> = (0..bench.frame_ms.len()).collect();
    order.sort_by(|&a, &b| bench.frame_ms[b].total_cmp(&bench.frame_ms[a]));
    let at_rank = |q: f64| {
        let rank = ((order.len() - 1) as f64 * (1.0 - q)).round() as usize;
        order.get(rank).map_or(0.0, |&i| bench.frame_ms[i])
    };
    text += &format!(
        "frame-ms-worst = {:.2}\nframe-ms-p99 = {:.2}\n",
        at_rank(1.0),
        at_rank(0.99)
    );
    text += &format!(
        "frames-over-33ms = {}\nframes-over-50ms = {}\n",
        bench.frame_ms.iter().filter(|&&m| m > 33.3).count(),
        bench.frame_ms.iter().filter(|&&m| m > 50.0).count()
    );
    let (mean, median, p95) = stats(&bench.post_ms);
    text += &format!("part-post-update-ms = {mean:.2} (median {median:.2}, p95 {p95:.2})\n");
    for &i in order.iter().take(8) {
        text += &format!("slow-frame {i}: {:.1} ms:", bench.frame_ms[i]);
        for (name, values) in PARTS.iter().zip(&bench.part_ms) {
            if let Some(ms) = values.get(i).filter(|&&ms| ms >= 1.0) {
                text += &format!(" {name} {ms:.1},");
            }
        }
        if let Some(ms) = bench.post_ms.get(i).filter(|&&ms| ms >= 1.0) {
            text += &format!(" post update {ms:.1},");
        }
        for (name, values) in RENDER_PARTS.iter().zip(&t.part_ms) {
            for at in i.saturating_sub(1)..=i {
                if let Some(ms) = values.get(at).filter(|&&ms| ms >= 1.0) {
                    text += &format!(" render {name} [{at}] {ms:.1},");
                }
            }
        }
        text += "\n";
    }
    text
}

/// The render side: every render diagnostic's mean, in milliseconds, the
/// largest first.
fn render_report(diagnostics: &DiagnosticsStore) -> String {
    let mut rows: Vec<(String, f64)> = diagnostics
        .iter()
        .filter(|d| d.path().as_str().starts_with("render/"))
        .filter(|d| {
            d.path().as_str().ends_with("elapsed_cpu") || d.path().as_str().ends_with("elapsed_gpu")
        })
        .filter_map(|d| Some((d.path().as_str().to_string(), d.average()?)))
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    rows.iter()
        .map(|(path, ms)| format!("{} = {ms:.3}\n", path.replace('/', "-")))
        .collect()
}

fn stats(values: &[f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
    (mean, at(0.5), at(0.95))
}

fn report(bench: &Bench, ticks: u64, world: Option<&s2_sim::World>) -> String {
    let mut text = String::new();
    let (mean, median, p95) = stats(&bench.frame_ms);
    text += &format!("frames = {}\n", bench.frame_ms.len());
    text += &format!(
        "frame-ms-mean = {mean:.2}\nframe-ms-median = {median:.2}\nframe-ms-p95 = {p95:.2}\n"
    );
    text += &format!("ticks = {ticks}\n");
    // ECO-TIME-1.2 to 1.4: at speed 1 the game keeps up with real time when
    // this is 10 (TICKS_PER_SECOND).
    let seconds = bench.frame_ms.iter().sum::<f64>() / 1000.0;
    let rate = if seconds > 0.0 {
        ticks as f64 / seconds
    } else {
        0.0
    };
    text += &format!("seconds = {seconds:.2}\nticks-per-second = {rate:.2}\n");
    if let Some(world) = world {
        text += &format!("tick = {}\n", world.tick());
        text += &format!("settlers = {}\n", world.settler_views().len());
        text += &format!("buildings = {}\n", world.buildings().count());
    }
    for (name, values) in PARTS.iter().zip(&bench.part_ms) {
        let (mean, median, p95) = stats(values);
        let key = name.replace([' ', ','], "-").replace("--", "-");
        text += &format!("part-{key}-ms = {mean:.2} (median {median:.2}, p95 {p95:.2})\n");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The long play names a first-use stall by its object: a slow frame's
    /// line lists the models and clips first read in it or in the
    /// [`FIRST_USE_FRAMES`] frames before it (0.12.2's 107 ms frame came
    /// with the first well's site; `models3d::preload`).
    // spec: none (the long play's log: a slow frame's first uses)
    #[test]
    fn a_slow_frame_names_the_first_uses_just_before_it() {
        let uses = vec![
            (
                10,
                "model data/buildings/PB_R_Well01ConstructionSite.GR2".to_owned(),
            ),
            (15, "model data/buildings/PB_R_Well01.GR2".to_owned()),
            (30, "clip data/animations/x.gr2".to_owned()),
        ];
        assert_eq!(
            first_uses_near(&uses, 16),
            [
                "model data/buildings/PB_R_Well01ConstructionSite.GR2",
                "model data/buildings/PB_R_Well01.GR2"
            ]
        );
        assert_eq!(
            first_uses_near(&uses, 18),
            [
                "model data/buildings/PB_R_Well01ConstructionSite.GR2",
                "model data/buildings/PB_R_Well01.GR2"
            ]
        );
        assert_eq!(
            first_uses_near(&uses, 19),
            ["model data/buildings/PB_R_Well01.GR2"]
        );
        assert!(first_uses_near(&uses, 9).is_empty());
        assert!(first_uses_near(&uses, 29).is_empty());
        assert_eq!(first_uses_near(&uses, 30), ["clip data/animations/x.gr2"]);
    }
}
