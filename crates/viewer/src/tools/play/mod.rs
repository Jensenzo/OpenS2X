//! `--play <script>`: the play harness. It runs the real program and drives
//! it the way a player's hands do: the pointer is put on the window, mouse
//! button and key events are written as Bevy's input messages, and the
//! program's own systems (`hover_3d`, the interface's focus and `interact`,
//! `map_click`, `apply_actions`, `advance_world`) do the rest. After each
//! step the game state is checked; where the pointer is, is read from the
//! system (`system_pointer`), so a warp the system did not make fails. The
//! outcome goes to the report file
//! (`--play-report`), a screenshot of every step to the screenshot
//! directory, and the exit code is 1 when a step failed.

pub mod display;
mod scripts;
mod settle;

pub(crate) use scripts::*;

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::prelude::*;
use s2_rules::RuleSet;
use s2_sim::grid::{self, Point};
use s2_sim::view::Activity;
use s2_sim::{Command, Placement, PlayerId, World};

use crate::ui::game::{Context, GameUi};
use crate::ui::render::{Hit, UiEvents, UiFrame};
use crate::ui::widgets::{Action, Global, Panel, PanelId, Widget};

/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;

/// The rectangle a widget that takes clicks covers, as the harness aims at
/// it (a text button: the 16 x 14 at its text's start it aims into).
fn widget_size(w: &Widget) -> Option<[f32; 2]> {
    match w {
        Widget::Button { size, rect, .. } => Some(size.unwrap_or([rect[2] as f32, rect[3] as f32])),
        Widget::BarButton { bar, .. } => Some(bar.size),
        Widget::TextButton { .. } => Some([16.0, 14.0]),
        Widget::Slider { width, .. } => Some(crate::ui::widgets::SliderArt::of(*width).size()),
        Widget::Picture { size, .. } => Some([*size, *size]),
        _ => None,
    }
}

/// The interface finds the enabled button of `action` at `point`: no other
/// panel's widget lies over it there (UI-GAME-1.4, `widget_at`).
fn finds(panels: &[Panel], point: [f32; 2], action: &Action) -> bool {
    let hit = crate::ui::render::widget_at(panels, point, &mut widget_size);
    matches!(&hit, Some(Hit::Button { action: a, enabled: true }) if a == action)
}

/// Fields the script chose, by the name their step gave them.
pub type Named = HashMap<&'static str, Point>;

/// What the script's resolvers and checks see of the program.
pub struct PlayView<'a> {
    /// The game's world; an empty one while a menu step runs.
    pub world: &'a World,
    pub ui: &'a GameUi,
    pub menu: &'a crate::menus::menu::MenuState,
    /// The mission's interface: its book, end screen and tutorial (CAM).
    pub mission: &'a crate::menus::campaign::MissionUi,
    /// Frames since the program started (a menu step's clock).
    pub frames: u64,
    /// The interface's goods and buildings (UI-GAME-1.6).
    pub catalog: Option<&'a crate::ui::game::Catalog>,
    pub panels: &'a [Panel],
    /// UI-GAME-9.3: the tooltip of what the pointer is over on the map.
    pub map_tooltip: Option<&'a str>,
    pub rules: Option<&'a RuleSet>,
    pub building_names: &'a HashMap<u32, String>,
    /// The models standing on the map (`has_building`).
    pub placements: &'a crate::render::view3d::Placements,
    pub overlay: bool,
    pub status: &'a str,
    pub player: PlayerId,
    view3d: &'a crate::render::view3d::View3d,
    camera: (&'a Camera, GlobalTransform),
    window: [f32; 2],
    /// Where the pointer is.
    cursor: Option<[f32; 2]>,
    /// The length of each frame a key was held (`Act::Hold`), ms, of all the
    /// script's holds so far.
    pub held_frames: &'a [f64],
}

impl PlayView<'_> {
    /// The player's headquarters: the first warehouse with soldiers.
    pub fn headquarters(&self) -> Result<Point, String> {
        self.world
            .warehouses()
            .map(|(at, _)| at)
            .find(|&at| {
                self.world
                    .building_at(at)
                    .is_some_and(|b| b.owner == self.player)
                    && self.world.military_at(at).is_some()
            })
            .ok_or_else(|| "the player has no headquarters".to_owned())
    }

    /// The flag field of a building (ECO-PLACE-3).
    pub fn flag_of(&self, building: Point) -> Point {
        self.world
            .terrain()
            .neighbour(building, FLAG_DIRECTION)
            .unwrap_or(building)
    }

    pub fn building_id(&self, name: &str) -> Result<u32, String> {
        self.building_names
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case(name))
            .map(|(id, _)| *id)
            .ok_or_else(|| format!("no building type named {name}"))
    }

    pub fn good_id(&self, name: &str) -> Result<u32, String> {
        self.rules
            .and_then(|r| r.id_named(name))
            .ok_or_else(|| format!("no good named {name}"))
    }

    /// Where a field's ground is on the screen, if it is on screen with a
    /// margin and not under the interface.
    pub fn screen(&self, field: Point) -> Option<[f32; 2]> {
        let p = self
            .view3d
            .field_on_screen(self.world, (self.camera.0, &self.camera.1), field)?;
        let [w, h] = self.window;
        let margin = 24.0;
        if p.x < margin || p.y < margin || p.x > w - margin || p.y > h - margin {
            return None;
        }
        let covered = self
            .panels
            .iter()
            .filter(|panel| !matches!(panel.id, PanelId::Labels | PanelId::Frame))
            .any(|panel| panel.bounds().is_some_and(|r| inside(p.to_array(), r)));
        (!covered).then_some(p.to_array())
    }

    /// GFX-VIEW-3.6 e: whether a scripted camera move runs.
    pub fn camera_moving(&self) -> bool {
        self.view3d.rig.moving()
    }

    /// Whether a field's ground is inside the window, under a dialog or not.
    pub fn in_view(&self, field: Point) -> bool {
        let [w, h] = self.window;
        self.view3d
            .field_on_screen(self.world, (self.camera.0, &self.camera.1), field)
            .is_some_and(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= w && p.y <= h)
    }

    /// The arrow key that brings `field` towards the free part of the screen,
    /// left of the tutorial window and above the bar. The rig's way to it when
    /// it is behind the camera.
    pub fn scroll_towards(&self, field: Point) -> Option<(KeyCode, Key)> {
        let [w, h] = self.window;
        let (want_x, want_y) = (w * 0.3, h * 0.55);
        let p = self
            .view3d
            .field_on_screen(self.world, (self.camera.0, &self.camera.1), field)?;
        let (dx, dy) = (want_x - p.x, want_y - p.y);
        Some(if dy.abs() >= dx.abs() {
            if dy > 0.0 {
                (KeyCode::ArrowUp, Key::ArrowUp)
            } else {
                (KeyCode::ArrowDown, Key::ArrowDown)
            }
        } else if dx > 0.0 {
            (KeyCode::ArrowLeft, Key::ArrowLeft)
        } else {
            (KeyCode::ArrowRight, Key::ArrowRight)
        })
    }

    /// Why [`Self::screen`] gives nothing for `field`: where it lands, and the
    /// panel over it.
    pub fn why_off_screen(&self, field: Point) -> String {
        let at = self
            .view3d
            .field_on_screen(self.world, (self.camera.0, &self.camera.1), field);
        let Some(p) = at else {
            return "no point in the view".to_owned();
        };
        let over: Vec<String> = self
            .panels
            .iter()
            .filter(|panel| !matches!(panel.id, PanelId::Labels | PanelId::Frame))
            .filter_map(|panel| {
                let r = panel.bounds()?;
                inside(p.to_array(), r).then(|| format!("{:?} {r:?}", panel.id))
            })
            .collect();
        format!(
            "at ({:.0}, {:.0}) of {:?}, target {:?}, zoom {}, under {over:?}",
            p.x, p.y, self.window, self.view3d.rig.target, self.view3d.rig.zoom
        )
    }

    /// The player's fields within `radius` of `center` that are on screen,
    /// nearest first.
    pub fn own_fields_on_screen(&self, center: Point, radius: u32) -> Vec<Point> {
        let t = self.world.terrain();
        let mut fields = grid::area(center, radius, t.width, t.height);
        fields.retain(|&f| {
            self.world.field_owner(f) == Some(self.player) && self.screen(f).is_some()
        });
        fields.sort_by_key(|&f| (grid::distance(center, f), t.index(f)));
        fields
    }

    /// The centre of the enabled button of the interface that `wanted`
    /// accepts (its action and tooltip). Text buttons are aimed at a point
    /// just inside their text's start, as their width is the text's.
    pub fn button(&self, wanted: &dyn Fn(&Action, Option<&str>) -> bool) -> Option<[f32; 2]> {
        for panel in self.panels {
            for w in &panel.widgets {
                let (at, size, action, tooltip, enabled) = match w {
                    Widget::BarButton {
                        at,
                        bar,
                        action,
                        enabled,
                        ..
                    } => (*at, bar.size, action, None, *enabled),
                    Widget::TextButton {
                        at,
                        font,
                        action,
                        enabled,
                        ..
                    } => (
                        crate::ui::render::label_top(*at, *font),
                        [16.0, 14.0],
                        action,
                        None,
                        *enabled,
                    ),
                    Widget::Button {
                        at,
                        size,
                        rect,
                        action,
                        tooltip,
                        enabled,
                        ..
                    } => (
                        *at,
                        size.unwrap_or([rect[2] as f32, rect[3] as f32]),
                        action,
                        tooltip.as_deref(),
                        *enabled,
                    ),
                    _ => continue,
                };
                if enabled && wanted(action, tooltip) {
                    let point = [
                        panel.at[0].round() + at[0].round() + size[0] / 2.0,
                        panel.at[1].round() + at[1].round() + size[1] / 2.0,
                    ];
                    // Only a point where the interface itself finds this
                    // button: another panel's widget over it would take the
                    // click (UI-GAME-1.4, `widget_at`).
                    if finds(self.panels, point, action) {
                        return Some(point);
                    }
                }
            }
        }
        None
    }

    /// What the interface has under the pointer, for a failed step's report:
    /// the pointer, the top widget there, and the UI's hovered and pressed
    /// actions.
    pub fn under_pointer(&self, events: &crate::ui::render::UiEvents) -> String {
        let hit = self
            .cursor
            .and_then(|p| crate::ui::render::widget_at(self.panels, p, &mut widget_size));
        format!(
            "pointer {:?}, widget there {hit:?}, hovered {:?}, pressed {:?}",
            self.cursor,
            events.hovered,
            events.pressed()
        )
    }

    /// The text of the refusal box, if one is shown (UI-GAME-1.4).
    pub fn refusal(&self) -> Option<&str> {
        self.ui.error.as_ref().map(|(text, _)| text.as_str())
    }

    /// The player's specialists (geologists, scouts) out of the warehouses,
    /// sent to `flag`, of the profession `good`.
    pub fn specialists_at(&self, flag: Point, good: u32) -> usize {
        self.world
            .settler_views()
            .iter()
            .filter(|v| {
                v.player == self.player
                    && v.profession == good
                    && matches!(v.activity, Activity::Specialist { flag: f } if f == flag)
            })
            .count()
    }
}

fn inside(p: [f32; 2], r: [f32; 4]) -> bool {
    p[0] >= r[0] && p[1] >= r[1] && p[0] < r[0] + r[2] && p[1] < r[1] + r[3]
}

type Resolver = Box<dyn Fn(&PlayView, &Named) -> Result<Point, String> + Send + Sync>;
type Matcher = Box<dyn Fn(&PlayView, &Named, &Action, Option<&str>) -> bool + Send + Sync>;
type Check = Box<dyn Fn(&PlayView, &Named) -> Result<(), String> + Send + Sync>;
type Preparer = Box<dyn Fn(&PlayView) -> Result<Command, String> + Send + Sync>;

/// The most key holds [`Act::Reveal`] gives a field.
const REVEAL_HOLDS: u32 = 400;
/// The frames of one such hold.
const REVEAL_FRAMES: u32 = 3;

/// One input of a step.
pub enum Act {
    /// The pointer over a field of the map, kept under the name.
    Field(&'static str, Resolver),
    /// As a person does: the arrow keys until the field is on screen and not
    /// under a window. A scripted camera move (CAM-5.3b) is waited for
    /// first; the key holds needed after it are counted for the report.
    Reveal(&'static str, Resolver),
    /// The pointer over a button of the interface (described for the
    /// report).
    Button(&'static str, Matcher),
    /// A left click where the pointer is: a press, the release next frame.
    Click,
    /// A right click where the pointer is: it closes a dialog left open
    /// (the close request of UI-GAME-1.10a) or cancels road building
    /// (UI-GAME-3.4).
    RightClick,
    /// A key pressed and released.
    Key(KeyCode, Key),
    /// A key held for some frames, each frame's length kept (the camera
    /// panned at full speed, GFX-VIEW-3.2, F55).
    Hold(KeyCode, Key, u32),
    /// One notch of the mouse wheel: forward (+1) or back (-1).
    Wheel(f32),
    /// A prepared state: a command no hand gives (a cheat of CAM-3.7, the
    /// headquarters gone), for what follows it.
    Prepare(Preparer),
}

/// A step of a script: inputs, then a check that must pass within some
/// game ticks.
pub struct Step {
    pub name: &'static str,
    pub acts: Vec<Act>,
    pub check: Check,
    /// Game ticks the check may take to pass once the inputs are given
    /// (frames for a menu step).
    pub within: u64,
    /// A step on the menu screens, before a game runs.
    pub menu: bool,
}

fn step(name: &'static str, acts: Vec<Act>, within: u64, check: Check) -> Step {
    Step {
        name,
        acts,
        check,
        within,
        menu: false,
    }
}

fn menu_step(name: &'static str, acts: Vec<Act>, within: u64, check: Check) -> Step {
    Step {
        menu: true,
        ..step(name, acts, within, check)
    }
}

/// Frames to let the program react to a pointer move or a click.
const SETTLE_FRAMES: u32 = 3;
/// Frames the scene gets after the camera went to the headquarters.
const CAMERA_FRAMES: u32 = 40;
/// Frames a game step waits for the game to run (a map loads in a few
/// seconds); then it fails and the script ends, so a game that never comes
/// (a load that did not happen) does not keep the program open.
const START_LIMIT: u32 = 3600;
/// Frames the program gets to show the menu before a menu step.
const MENU_FRAMES: u64 = 150;

/// Where the driver is in the script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The game is not running yet.
    Starting,
    /// The camera was sent to the headquarters.
    Camera,
    /// Giving the step's inputs.
    Acting,
    /// A press was given; the release follows.
    Releasing,
    /// Waiting for the step's check to pass.
    Checking,
    /// The report is written; the last screenshot is being saved.
    Finishing,
}

/// The driver's state.
#[derive(Resource)]
pub struct Play {
    steps: Vec<Step>,
    step: usize,
    act: usize,
    phase: Phase,
    wait: u32,
    /// The tick by which the current step's check must pass.
    deadline: Option<u64>,
    /// Frames since the start.
    frames: u64,
    /// Frames a game step has waited for the game to run.
    starting: u32,
    /// A key being released.
    key: Option<(KeyCode, Key)>,
    /// The key holds of the [`Act::Reveal`] going on.
    reveals: u32,
    /// The key holds of every [`Act::Reveal`] of the run, and of how many
    /// fields.
    scrolled: (u32, u32),
    /// Frames the key is still held (`Act::Hold`).
    hold: u32,
    /// When this frame started, and the length of the frames a key was held.
    frame_start: Option<std::time::Instant>,
    held_frames: Vec<f64>,
    /// The last frame's length, ms, and the run's pace (`settle::Pace`).
    frame_ms: f64,
    pace: settle::Pace,
    /// The gate before a timed act: when it began and its frames; the
    /// (step, act) whose gate passed.
    settle: Option<(std::time::Instant, settle::Settle)>,
    settled: Option<(usize, usize)>,
    /// The right button is down: a failed step's cancel click.
    cancelling: bool,
    /// Where the pointer was put, and how often it had to be put again.
    pointer: Option<([f32; 2], u32)>,
    named: Named,
    results: Vec<(String, Result<(), String>)>,
    report: Option<PathBuf>,
    /// The report's first line: what the harness found of the display
    /// (`display::hold`).
    note: Option<String>,
    shots: PathBuf,
}

impl Play {
    pub fn new(steps: Vec<Step>, report: Option<PathBuf>, shots: PathBuf) -> Self {
        Self {
            steps,
            step: 0,
            act: 0,
            phase: Phase::Starting,
            wait: 0,
            deadline: None,
            frames: 0,
            starting: 0,
            key: None,
            reveals: 0,
            scrolled: (0, 0),
            hold: 0,
            frame_start: None,
            held_frames: Vec::new(),
            frame_ms: 0.0,
            pace: settle::Pace::default(),
            settle: None,
            settled: None,
            cancelling: false,
            pointer: None,
            named: Named::new(),
            results: Vec::new(),
            report,
            note: None,
            shots,
        }
    }

    /// A line the report starts with (not a step).
    pub fn with_note(mut self, note: Option<&str>) -> Self {
        self.note = note.map(str::to_owned);
        self
    }

    fn record(&mut self, result: Result<(), String>) {
        let name = self.steps[self.step].name.to_owned();
        match &result {
            Ok(()) => info!("play: ok   {name}"),
            Err(e) => error!("play: FAIL {name}: {e}"),
        }
        self.results.push((name, result));
    }

    fn report_text(&self) -> String {
        let mut text = String::new();
        if let Some(note) = &self.note {
            text.push_str(&format!("note {note}\n"));
        }
        let mut passed = 0;
        for (name, result) in &self.results {
            match result {
                Ok(()) => {
                    passed += 1;
                    text.push_str(&format!("ok   {name}\n"));
                }
                Err(e) => text.push_str(&format!("FAIL {name}: {e}\n")),
            }
        }
        text.push_str(&format!("passed {passed} of {}\n", self.steps.len()));
        let (holds, fields) = self.scrolled;
        if holds > 0 {
            text.push_str(&format!(
                "note scrolled to {fields} marked field(s) with {holds} arrow key holds\n"
            ));
        }
        text
    }

    pub fn failed(&self) -> bool {
        self.results.len() < self.steps.len() || self.results.iter().any(|(_, r)| r.is_err())
    }
}

/// What the driver decided to do this frame, carried out once the view's
/// borrows are gone.
enum Decision {
    Nothing,
    Pointer([f32; 2]),
    Press,
    Release,
    /// A right click: the press, then the release.
    Cancel(ButtonState),
    KeyDown(KeyCode, Key),
    KeyUp(KeyCode, Key),
    Wheel(f32),
    Prepare(Command),
    StepDone(Result<(), String>),
}

/// Where the system's pointer is, in the window's logical pixels: what the
/// player sees, not what Bevy was last told (F41: the step after "Create
/// Flag" passed on Bevy's value). On Windows `GetCursorPos`; elsewhere
/// Bevy's own.
#[cfg(windows)]
fn system_pointer(
    window: &Window,
    handle: Option<&bevy::window::RawHandleWrapper>,
) -> Option<[f32; 2]> {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let RawWindowHandle::Win32(win32) = handle?.get_window_handle() else {
        return None;
    };
    let hwnd = win32.hwnd.get() as HWND;
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: `p` is a valid POINT to write to, and `hwnd` is the live
    // window of this program (a stale handle only makes the calls fail).
    let read = unsafe { GetCursorPos(&mut p) != 0 && ScreenToClient(hwnd, &mut p) != 0 };
    let scale = window.scale_factor();
    read.then(|| [p.x as f32 / scale, p.y as f32 / scale])
}

#[cfg(not(windows))]
fn system_pointer(window: &Window, _: Option<&bevy::window::RawHandleWrapper>) -> Option<[f32; 2]> {
    window.cursor_position().map(|c| c.to_array())
}

/// The driver: one input per frame, in `First` so the program's own input
/// systems see them in the same frame.
#[allow(clippy::too_many_arguments)]
pub fn drive(
    mut play: ResMut<Play>,
    mut viewer: ResMut<crate::game::Viewer>,
    ui: Res<GameUi>,
    (frame, events): (Res<UiFrame>, Res<UiEvents>),
    menu: Res<crate::menus::menu::MenuState>,
    mission: Res<crate::menus::campaign::MissionUi>,
    catalog: Res<crate::ui::UiCatalog>,
    view3d: Res<crate::render::view3d::View3d>,
    placements: Res<crate::render::view3d::Placements>,
    camera: Single<(&Camera, &Transform), With<crate::render::view3d::MapCamera3d>>,
    mut window: Single<(Entity, &mut Window, Option<&bevy::window::RawHandleWrapper>)>,
    mut mouse: MessageWriter<MouseButtonInput>,
    mut keys: MessageWriter<KeyboardInput>,
    mut wheel: MessageWriter<MouseWheel>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    play.frames += 1;
    let now = std::time::Instant::now();
    if let Some(start) = play.frame_start {
        let ms = (now - start).as_secs_f64() * 1000.0;
        play.frame_ms = ms;
        if play.hold > 0 {
            play.held_frames.push(ms);
        }
        let since = now.duration_since(*EPOCH.get_or_init(std::time::Instant::now));
        if let Some(line) = play.pace.frame(ms, since, display::now()) {
            info!("play: {line}");
        }
    }
    play.frame_start = Some(now);
    if play.wait > 0 {
        play.wait -= 1;
        return;
    }
    let (window_entity, window, handle) = &mut *window;
    let window_entity = *window_entity;
    let handle = *handle;
    let menu_step = play.steps.get(play.step).is_some_and(|s| s.menu);
    match play.phase {
        // A menu step starts once the menu is up and the window has had
        // time to show it (the first frames come before anything is drawn).
        Phase::Starting if menu_step => {
            if menu.screen == crate::menus::menu::Screen::None
                || frame.panels.is_empty()
                || play.frames < MENU_FRAMES
            {
                return;
            }
            play.phase = Phase::Acting;
            return;
        }
        Phase::Starting => {
            let running = viewer.world.is_some()
                && viewer.real_game
                && menu.screen == crate::menus::menu::Screen::None
                && viewer.frames_since_load > 5;
            if !running {
                play.starting += 1;
                if play.starting > START_LIMIT {
                    play.record(Err(format!(
                        "the game did not run within {START_LIMIT} frames"
                    )));
                    finish(&mut play);
                }
                return;
            }
            play.starting = 0;
            // The camera over the headquarters, as the player would scroll
            // there first. A tutorial is played from the camera the game
            // starts it with (GFX-VIEW-3.6), which shows its marked fields.
            if mission.tutorial.is_none() {
                let hq = viewer.world.as_ref().and_then(|w| {
                    w.warehouses().map(|(at, _)| at).find(|&at| {
                        w.building_at(at)
                            .is_some_and(|b| b.owner == crate::game::PLAYER)
                    })
                });
                viewer.focus_request = hq;
            }
            play.phase = Phase::Camera;
            play.wait = CAMERA_FRAMES;
            return;
        }
        Phase::Camera => {
            play.phase = Phase::Acting;
        }
        Phase::Finishing => {
            exit.write(if play.failed() {
                AppExit::error()
            } else {
                AppExit::Success
            });
            return;
        }
        _ => {}
    }
    if play.step >= play.steps.len() {
        finish(&mut play);
        return;
    }
    let empty;
    let held_frames = std::mem::take(&mut play.held_frames);
    let decision = {
        let world = match viewer.world.as_ref() {
            Some(world) if !menu_step => world,
            _ if menu_step => {
                empty = World::new(0);
                &empty
            }
            _ => return,
        };
        let (cam, transform) = *camera;
        let view = PlayView {
            world,
            ui: &ui,
            menu: &menu,
            mission: &mission,
            frames: play.frames,
            catalog: catalog.0.as_ref(),
            panels: &frame.panels,
            map_tooltip: frame.map_tooltip.as_deref(),
            rules: viewer.rules.as_ref(),
            building_names: &viewer.building_names,
            placements: &placements,
            overlay: viewer.overlay,
            status: &viewer.status,
            player: crate::game::PLAYER,
            view3d: &view3d,
            camera: (cam, GlobalTransform::from(*transform)),
            window: [window.width(), window.height()],
            cursor: system_pointer(window, handle),
            held_frames: &held_frames,
        };
        match decide(&mut play, &view) {
            // A failed step says what the interface had under the pointer
            // (its top widget there, the hovered and the pressed action).
            Decision::StepDone(Err(e)) => {
                Decision::StepDone(Err(format!("{e} ({})", view.under_pointer(&events))))
            }
            decision => decision,
        }
    };
    play.held_frames = held_frames;
    match decision {
        Decision::Nothing => {}
        Decision::Pointer(p) => {
            window.set_cursor_position(Some(Vec2::new(p[0], p[1])));
            let again = play
                .pointer
                .map_or(0, |(q, n)| if q == p { n + 1 } else { 0 });
            play.pointer = Some((p, again));
            play.wait = SETTLE_FRAMES;
        }
        Decision::Press => {
            mouse.write(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window: window_entity,
            });
            play.phase = Phase::Releasing;
        }
        Decision::Release => {
            mouse.write(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window: window_entity,
            });
            play.phase = Phase::Acting;
            play.wait = SETTLE_FRAMES;
        }
        Decision::Cancel(state) => {
            mouse.write(MouseButtonInput {
                button: MouseButton::Right,
                state,
                window: window_entity,
            });
            play.cancelling = state == ButtonState::Pressed;
            play.phase = if play.cancelling {
                Phase::Releasing
            } else {
                Phase::Acting
            };
            play.wait = SETTLE_FRAMES;
        }
        Decision::KeyDown(code, key) => {
            keys.write(KeyboardInput {
                key_code: code,
                logical_key: key.clone(),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: window_entity,
            });
            play.key = Some((code, key));
            play.phase = Phase::Releasing;
        }
        Decision::KeyUp(code, key) => {
            keys.write(KeyboardInput {
                key_code: code,
                logical_key: key,
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window: window_entity,
            });
            play.key = None;
            play.phase = Phase::Acting;
            play.wait = SETTLE_FRAMES;
        }
        Decision::Wheel(y) => {
            wheel.write(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y,
                window: window_entity,
                phase: bevy::input::touch::TouchPhase::Moved,
            });
            play.wait = SETTLE_FRAMES;
        }
        Decision::Prepare(command) => {
            if let Some(world) = viewer.world.as_mut() {
                world.submit(command);
            }
            play.wait = SETTLE_FRAMES;
        }
        Decision::StepDone(result) => {
            let index = play.step;
            // A failed step may leave road building or a dialog open: the
            // right button cancels both (UI-GAME-3.4), as a player would.
            if result.is_err() {
                play.cancelling = true;
            }
            let shot = play.shots.join(format!(
                "play-{:02}-{}.png",
                index + 1,
                slug(play.steps[index].name)
            ));
            crate::tools::shots::take_screenshot(&mut commands, shot);
            play.record(result);
            play.step += 1;
            play.act = 0;
            play.deadline = None;
            // From the menu into the game: wait for it as at the start.
            let into_game = play.steps.get(play.step).is_some_and(|s| !s.menu);
            play.phase = if play.steps[index].menu && into_game {
                Phase::Starting
            } else {
                Phase::Acting
            };
            play.wait = SETTLE_FRAMES;
            if play.step >= play.steps.len() {
                finish(&mut play);
            }
        }
    }
}

/// The next thing to do for the current step.
fn decide(play: &mut Play, view: &PlayView) -> Decision {
    if play.phase == Phase::Releasing {
        if play.hold > 1 {
            play.hold -= 1;
            return Decision::Nothing;
        }
        play.hold = 0;
        if play.cancelling {
            return Decision::Cancel(ButtonState::Released);
        }
        return match play.key.clone() {
            Some((code, key)) => Decision::KeyUp(code, key),
            None => Decision::Release,
        };
    }
    if play.cancelling {
        return Decision::Cancel(ButtonState::Pressed);
    }
    let timed = play.steps[play.step]
        .acts
        .get(play.act)
        .is_some_and(|a| matches!(a, Act::Hold(..)));
    if timed && play.settled != Some((play.step, play.act)) {
        if let Some(decision) = settle_first(play) {
            return decision;
        }
    }
    let step = &play.steps[play.step];
    if play.act < step.acts.len() {
        let act = &step.acts[play.act];
        // The pointer is moved by the window system; a click waits until it
        // is where it was put (within the rounding to whole pixels).
        if let (Act::Click, Some((p, again))) = (act, play.pointer) {
            let there = view
                .cursor
                .is_some_and(|c| (c[0] - p[0]).abs() <= 2.0 && (c[1] - p[1]).abs() <= 2.0);
            if !there {
                if again >= 20 {
                    return Decision::StepDone(Err(format!(
                        "the pointer did not reach {p:?}: it is at {:?}",
                        view.cursor
                    )));
                }
                return Decision::Pointer(p);
            }
        }
        play.act += 1;
        return match act {
            Act::Field(name, resolve) => match resolve(view, &play.named) {
                Ok(field) => {
                    play.named.insert(name, field);
                    match view.screen(field) {
                        Some(p) => {
                            info!(
                                "play: pointer over {name} ({}, {}) at {p:?}",
                                field.0, field.1
                            );
                            Decision::Pointer(p)
                        }
                        None => Decision::StepDone(Err(format!(
                            "{name} ({}, {}) is not on screen: {}",
                            field.0,
                            field.1,
                            view.why_off_screen(field)
                        ))),
                    }
                }
                Err(e) => Decision::StepDone(Err(e)),
            },
            Act::Reveal(_, _) if view.camera_moving() => {
                // This act again once the move is over.
                play.act -= 1;
                Decision::Nothing
            }
            Act::Reveal(name, resolve) => match resolve(view, &play.named) {
                Ok(field) => match (view.screen(field), view.scroll_towards(field)) {
                    (Some(_), _) => {
                        if play.reveals > 0 {
                            play.scrolled.0 += play.reveals;
                            play.scrolled.1 += 1;
                        }
                        play.reveals = 0;
                        Decision::Nothing
                    }
                    (None, Some((code, key))) if play.reveals < REVEAL_HOLDS => {
                        play.reveals += 1;
                        // This act again after the hold.
                        play.act -= 1;
                        play.hold = REVEAL_FRAMES;
                        Decision::KeyDown(code, key)
                    }
                    _ => Decision::StepDone(Err(format!(
                        "{name} ({}, {}) cannot be scrolled to: {}",
                        field.0,
                        field.1,
                        view.why_off_screen(field)
                    ))),
                },
                Err(e) => Decision::StepDone(Err(e)),
            },
            Act::Button(name, wanted) => match view.button(&|a, t| wanted(view, &play.named, a, t))
            {
                Some(p) => {
                    info!("play: pointer over {name} at {p:?}");
                    Decision::Pointer(p)
                }
                None => Decision::StepDone(Err(format!("{name} is not on screen"))),
            },
            Act::Click => Decision::Press,
            Act::RightClick => {
                play.cancelling = true;
                Decision::Cancel(ButtonState::Pressed)
            }
            Act::Key(code, key) => Decision::KeyDown(*code, key.clone()),
            Act::Hold(code, key, frames) => {
                play.hold = *frames;
                Decision::KeyDown(*code, key.clone())
            }
            Act::Wheel(y) => Decision::Wheel(*y),
            Act::Prepare(command) => match command(view) {
                Ok(command) => Decision::Prepare(command),
                Err(e) => Decision::StepDone(Err(e)),
            },
        };
    }
    // The inputs are given: the check, until the deadline.
    let now = if step.menu {
        view.frames
    } else {
        view.world.tick()
    };
    let deadline = *play.deadline.get_or_insert(now + step.within);
    play.phase = Phase::Checking;
    match (step.check)(view, &play.named) {
        Ok(()) => Decision::StepDone(Ok(())),
        Err(e) if now > deadline => {
            let mut text = e;
            if let Some(r) = view.refusal() {
                text.push_str(&format!("; the game refused: {r}"));
            }
            if !view.status.is_empty() {
                text.push_str(&format!("; last status: {}", view.status));
            }
            Decision::StepDone(Err(text))
        }
        Err(_) => Decision::Nothing,
    }
}

/// The gate before a timed act (`settle`): a decision while the frames
/// are not settled, `None` once they are.
fn settle_first(play: &mut Play) -> Option<Decision> {
    use settle::Gate;
    let now = std::time::Instant::now();
    let frame_ms = play.frame_ms;
    let (began, gate) = play
        .settle
        .get_or_insert_with(|| (now, settle::Settle::default()));
    let since = now - *began;
    let state = display::now();
    if let Some(line) = gate.progress(since, state) {
        info!("play: {line}");
    }
    match gate.frame(frame_ms, since, state) {
        Gate::Wait => Some(Decision::Nothing),
        Gate::Wake => {
            let sent = display::wake();
            info!(
                "play: the display is {}: woken by a pointer move of 0 at {}{}",
                state.unwrap_or("not known"),
                display::utc_now(),
                if sent { "" } else { " (not possible here)" }
            );
            Some(Decision::Nothing)
        }
        Gate::Settled(line) => {
            info!("play: {line}");
            play.settle = None;
            play.settled = Some((play.step, play.act));
            None
        }
        Gate::Failed(e) => {
            play.settle = None;
            Some(Decision::StepDone(Err(e)))
        }
    }
}

/// The instant the run's frame times are counted from (`settle::Pace`).
static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn finish(play: &mut Play) {
    let text = play.report_text();
    eprint!("{text}");
    if let Some(path) = &play.report {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(path, &text) {
            error!("cannot write {}: {e}", path.display());
        }
    }
    play.phase = Phase::Finishing;
    // The last screenshot needs a few frames to be saved.
    play.wait = 30;
}

/// A step's name as a file name part.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(40).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the harness script's own consistency)
    #[test]
    fn script_steps_are_named_once() {
        for steps in [
            first_ten_minutes(),
            select_map(),
            tutorial_one(),
            tutorial_two(),
            tutorial_three(),
            tutorial_four(),
            mission_end(),
            mission_defeat(),
            mission_save(),
            mission_load(),
            campaign_state_write(),
            campaign_state_read(),
            mission_after_free_game(),
            panning(),
        ] {
            let mut names: Vec<&str> = steps.iter().map(|s| s.name).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), steps.len());
        }
        assert!(script("first-ten-minutes").is_some());
        assert!(script("nothing").is_none());
    }

    // spec: UI-GAME-1.4 (the harness clicks only where the interface finds
    // the button: a panel over it takes the click)
    #[test]
    fn a_covered_button_is_not_aimed_at() {
        let mut under = Panel::new(PanelId::Global, [100.0, 100.0]);
        under.button(
            crate::ui::widgets::Atlas::Game,
            [0, 0, 40, 20],
            crate::ui::widgets::Hover::None,
            [0.0, 0.0],
            None,
            Action::CloseGlobal,
            true,
        );
        let mut over = Panel::new(PanelId::Menu, [90.0, 95.0]);
        over.button(
            crate::ui::widgets::Atlas::Game,
            [0, 0, 40, 30],
            crate::ui::widgets::Hover::None,
            [0.0, 0.0],
            None,
            Action::CloseContext,
            true,
        );
        let centre = [120.0, 110.0];
        assert!(finds(
            std::slice::from_ref(&under),
            centre,
            &Action::CloseGlobal
        ));
        let both = [under.clone(), over];
        assert!(!finds(&both, centre, &Action::CloseGlobal));
        assert!(finds(&both, [135.0, 110.0], &Action::CloseGlobal));
    }

    // spec: none (the harness's screenshot names)
    #[test]
    fn slugs() {
        assert_eq!(
            slug("\"Build road\" starts: a road"),
            "build-road-starts-a-road"
        );
    }
}
