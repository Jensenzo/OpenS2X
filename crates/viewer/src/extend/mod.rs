//! The extension points for projects built on this engine:
//! rules overlays, asset folders, render options, Bevy plugins, and the
//! hooks of the menus, start-up, music, camera, window, settings and the
//! things on the map, given
//! to [`run_with`](crate::run_with). Not part of the original game; with
//! [`Extensions::default`] everything is exactly the original's (the
//! golden images prove it). What may change and what must not:
//! `docs/extending.md`.

pub(crate) mod menu;
pub(crate) mod things;

use bevy::prelude::{App, DetectChanges, Message, Res, ResMut, Resource, Vec3};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use crate::menus::settings::Settings;
pub use menu::{MenuEntry, MenuPage, OnChange, OnClick, PageWidget};
pub use things::{MapHit, MapThing, MapThings};

/// A project's change to the app: systems, resources, plugins.
pub type AppHook = Box<dyn FnOnce(&mut App)>;

/// What a project adds to the game. The default adds nothing.
#[derive(Default)]
pub struct Extensions {
    /// Lua run after the original property scripts, in their environment
    /// (`s2_rules::Overlay`): changed or added rules.
    pub rules_overlays: Vec<s2_rules::Overlay>,
    /// Folders laid out like the install (`data/...`): a file there takes
    /// the place of the install's file of the same path (models, textures,
    /// sounds, interface pictures, property scripts). The last folder wins.
    pub asset_folders: Vec<PathBuf>,
    /// How the 3D view draws.
    pub render: RenderOptions,
    /// Called with the app after the game's own plugins and systems, before
    /// it runs: more systems, resources, plugins (drawing and input only;
    /// the simulation changes through commands, `docs/extending.md`).
    pub plugins: Vec<AppHook>,
    /// Main menu entries after the original ones, each opening its page
    /// (two fit; default none).
    pub menu: Vec<MenuEntry>,
    /// A line of the project's own on the start screen and on the main
    /// menu (default none).
    pub captions: Captions,
    /// Start at the start screen: no intro film (the publisher films are
    /// never shown, UI-MENU-0.1). Default false: the intro, then the start
    /// screen.
    pub skip_launch_films: bool,
    /// The project plays the music: the viewer's own player is silent
    /// while it still sends every [`MusicSlot`]. Default false: the
    /// viewer plays the original's music (SND-3).
    pub music_external: bool,
    /// The game window's title (from the start and after every map load)
    /// and that of its message boxes; the game folder picker's title starts
    /// with it. Default `OpenS2X` (the program's public name).
    pub window_title: Option<String>,
    /// Where the player's settings are written (default: as the game,
    /// SET-1.2).
    pub settings: SettingsOptions,
    /// Called with the player's settings after every change (a settings
    /// page, the first run's game folder, a mission won), whether it is
    /// written or not. Default none.
    pub on_settings_change: Option<SettingsHook>,
    /// Keep [`MapThings`]: the settlers, wares and animals under the
    /// pointer and where each is drawn, read only. Default false: the
    /// resource is not there and nothing of it is computed.
    pub map_things: bool,
    /// The folder of the viewer's F12 screenshots (`<map>-<unix
    /// seconds>.png`). Default none: the `--screenshot-dir` option, else
    /// `screens` below the working folder.
    pub screenshot_dir: Option<PathBuf>,
    /// The folder the project's games are saved and autosaved in, and
    /// listed from; the original's `.sav` files are still listed from
    /// `<Documents>\S2\saves`. Default none: `<Documents>\S2\saves` for
    /// both, as the game (`--save-dir` wins over either).
    pub save_dir: Option<PathBuf>,
    /// The project's name, written with its rules overlays (their names and
    /// text hashes) in the mark of its saves (`s2_ai::save::SaveMark`): a
    /// save of another mark does not load. Default none: s2-reborn, whose
    /// mark is empty without overlays.
    pub project_name: Option<String>,
}

/// Called with the player's settings after a change
/// ([`Extensions::on_settings_change`]).
pub type SettingsHook = Box<dyn Fn(&Settings) + Send + Sync>;

/// Where the program keeps the player's settings (SET-1.2).
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsOptions {
    /// Write the original's profile file `<Documents>\S2\users\<profile>.cfg`
    /// (keeping it as `<profile>.cfg.bak` the first time) when a setting
    /// changes. False: never write either; the changed values live in
    /// memory (and go to [`Extensions::on_settings_change`]). Default true.
    pub profile_write: bool,
    /// The folder of the program's own `settings.txt` (game folder,
    /// profile, day and night) instead of `%APPDATA%\s2-reborn`
    /// (`~/.config/s2-reborn` elsewhere). Default none.
    pub app_settings_dir: Option<PathBuf>,
    /// A folder of the project's holding a copy of the profile file,
    /// `<profile>.cfg`: read at start in place of the original's when it is
    /// there, written at every change whatever `profile_write` says.
    /// Default none.
    pub profile_dir: Option<PathBuf>,
}

impl Default for SettingsOptions {
    fn default() -> Self {
        Self {
            profile_write: true,
            app_settings_dir: None,
            profile_dir: None,
        }
    }
}

/// What the viewer's music player switched to (SND-3), sent as a Bevy
/// message (`MessageReader<MusicSlot>`) in the frame it starts or stops a
/// track, whether it plays it ([`Extensions::music_external`] false) or
/// keeps silent (true). Non-exhaustive since 0.10.0: a `match` on it needs
/// a `_` arm, so a later slot is not a breaking change.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MusicSlot {
    /// The main theme (SND-3.1): the start screen and the menus outside a
    /// game.
    Menu,
    /// The credits' music (SND-3.1).
    Credits,
    /// A track of the in-game playlist (SND-3.2), one of the tribe's sets:
    /// 0 Roman, 1 African, 2 Chinese (`World::tribe`). Sent for every new
    /// track.
    Tribe(u8),
    /// The battle loop starts (true) while the player fights, or stops
    /// (false) 20 s after the last fight (SND-3.4); the playlist then goes
    /// on with a `Tribe`.
    Battle(bool),
    /// W13 (SND-2.2): the mission's book dialog (briefing or debriefing)
    /// opened over the game; its music loops until another slot is sent
    /// (a `Tribe` when the book closes, a `Jingle` when the end dialog
    /// appears over it).
    Briefing,
    /// W13 (SND-2.2, CAM-3.6): the end dialog (and the victory or defeat
    /// screen) appeared: its jingle, won (true) or lost (false), plays once
    /// and nothing follows until the next slot.
    Jingle(bool),
}

/// A point the 3D camera follows (a resource the viewer keeps): while
/// `target` is set, the camera keeps it at the screen's centre every frame
/// at the zoom and turn it has. The player's own camera input (scrolling,
/// dragging, the wheel, zoom and turn keys, the minimap, a jump to a
/// message or home, the look mode, following a settler with O) takes the
/// camera back: `target` goes to `None` and `cleared` is true for one
/// frame. A point set while O already follows a settler ends O's follow
/// instead and takes the camera (W10). The default follows nothing.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CameraFollow {
    /// In Bevy's world coordinates (those of the scene's `Transform`s);
    /// the height is not used, the camera keeps to the ground (GFX-VIEW-3.3).
    pub target: Option<Vec3>,
    /// The player took the camera back in the last frame.
    pub cleared: bool,
}

/// The player's settings as they are now (the profile, SET), read only: a
/// resource the viewer keeps up to date, changed (for Bevy's change
/// detection) only when a setting is.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CurrentSettings(Settings);

impl CurrentSettings {
    pub(crate) fn new(settings: Settings) -> Self {
        Self(settings)
    }

    pub fn get(&self) -> &Settings {
        &self.0
    }
}

/// Keep [`CurrentSettings`] the menu's settings.
pub(crate) fn sync_settings(
    menu: Res<crate::menus::menu::MenuState>,
    mut current: ResMut<CurrentSettings>,
) {
    if menu.is_changed() && current.0 != menu.settings {
        current.0 = menu.settings.clone();
    }
}

/// Lines of a project's own on the original's screens, in the original's
/// fonts. `None` draws nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Captions {
    /// One line on the start screen (UI-MENU-0.2) in `font_big_white`, in
    /// the colour of the game's title line, with its top-left at (210, 496)
    /// of the parchment: left-aligned above that title line (210, 520).
    pub start_screen_title: Option<String>,
    /// One line on the main menu in `dark_system`, its right end at x 908
    /// (the right edge of the button column) and its top at y 672: on the
    /// scroll between EXIT and the scroll's roll.
    pub main_menu_footer: Option<String>,
}

/// Render options. The defaults are the game's (GFX-VIEW-2.5,
/// docs/shadows.md); a project may draw better than the original.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Texels per side of the shadow map instead of the one `SHADOW
    /// QUALITY` gives (GFX-VIEW-2.5 d); `Some(0)` turns shadows off.
    pub shadow_map_size: Option<usize>,
    /// Shadow cascades (default 2).
    pub shadow_cascades: usize,
    /// Anisotropic filtering of the repeating colour textures (default 8;
    /// the terrain's normal maps are trilinear, GFX-VIEW-2.4b).
    pub anisotropy: u16,
    /// The colour behind everything (sRGB, default the game's dark blue).
    pub clear_color: [f32; 3],
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            shadow_map_size: None,
            shadow_cascades: 2,
            anisotropy: 8,
            clear_color: [0.02, 0.03, 0.06],
        }
    }
}

/// The parts of the extensions read before the app runs (the first-run
/// picker, the settings files), set first thing in `run_with`.
struct Program {
    window_title: Option<String>,
    settings: SettingsOptions,
    on_settings_change: Option<SettingsHook>,
}

static PROGRAM: OnceLock<Program> = OnceLock::new();

/// Keep the window title and the settings options for [`window_title`],
/// [`profile_write`], [`app_settings_dir`] and [`settings_changed`]. The
/// first call wins.
pub(crate) fn install_program(
    window_title: Option<String>,
    settings: SettingsOptions,
    on_settings_change: Option<SettingsHook>,
) {
    let _ = PROGRAM.set(Program {
        window_title,
        settings,
        on_settings_change,
    });
}

/// The profile file is written when a setting changes (true until
/// [`install_program`]).
pub(crate) fn profile_write() -> bool {
    PROGRAM.get().is_none_or(|p| p.settings.profile_write)
}

/// The project's folder for the program's own settings file.
pub(crate) fn app_settings_dir() -> Option<&'static Path> {
    PROGRAM
        .get()
        .and_then(|p| p.settings.app_settings_dir.as_deref())
}

/// The project's folder for its copy of the profile file.
pub(crate) fn profile_dir() -> Option<&'static Path> {
    PROGRAM
        .get()
        .and_then(|p| p.settings.profile_dir.as_deref())
}

/// Tell the project's hook of changed settings.
pub(crate) fn settings_changed(settings: &Settings) {
    if let Some(hook) = PROGRAM.get().and_then(|p| p.on_settings_change.as_ref()) {
        hook(settings);
    }
}

/// The game window's title: the project's, else `OpenS2X`.
pub(crate) fn window_title() -> &'static str {
    title_or_default(PROGRAM.get().and_then(|p| p.window_title.as_deref()))
}

fn title_or_default(title: Option<&str>) -> &str {
    title.unwrap_or("OpenS2X")
}

/// The first run's game folder picker's title: its question, after the
/// project's title when it has one. (The picker is Windows'.)
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn picker_title() -> String {
    picker_title_of(PROGRAM.get().and_then(|p| p.window_title.as_deref()))
}

fn picker_title_of(title: Option<&str>) -> String {
    let question = "Where is The Settlers II: 10th Anniversary installed?";
    match title {
        Some(title) => format!("{title}: {question}"),
        None => question.to_owned(),
    }
}

/// The parts of the extensions read while the game runs, set once at start.
struct Shared {
    asset_folders: Vec<PathBuf>,
    render: RenderOptions,
    music_external: bool,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

/// Keep the asset folders, render options and the music switch for
/// [`find`], [`render`] and [`music_external`]. The first call wins (one
/// game per process).
pub(crate) fn install(asset_folders: Vec<PathBuf>, render: RenderOptions, music_external: bool) {
    let _ = SHARED.set(Shared {
        asset_folders,
        render,
        music_external,
    });
}

/// The project plays the music (false until [`install`]).
pub(crate) fn music_external() -> bool {
    SHARED.get().is_some_and(|s| s.music_external)
}

/// A game file by its path below the install (`data/...`, any case): from
/// the asset folders, else from the install at `root`.
pub(crate) fn find(root: &Path, path: &str) -> Result<PathBuf, String> {
    let folders = SHARED.get().map_or(&[][..], |s| &s.asset_folders[..]);
    s2_assets::find_in(folders, root, path)
}

/// The asset folders, in the order given.
pub(crate) fn asset_folders() -> &'static [PathBuf] {
    SHARED.get().map_or(&[][..], |s| &s.asset_folders[..])
}

/// The render options (the defaults until [`install`]).
pub(crate) fn render() -> RenderOptions {
    SHARED.get().map(|s| s.render.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the window title hook: a project's title, else the game's)
    #[test]
    fn a_project_names_the_window() {
        assert_eq!(title_or_default(Some("Frontier")), "Frontier");
        assert_eq!(title_or_default(None), "OpenS2X");
        assert_eq!(
            picker_title_of(Some("Frontier")),
            "Frontier: Where is The Settlers II: 10th Anniversary installed?"
        );
    }

    /// The music hook's volume: `CurrentSettings` follows the menu's
    /// settings (SND-5.1's music volume among them) and changes only when
    /// they do.
    // spec: SND-5.1 (the music volume a project reads)
    #[test]
    fn current_settings_follow_the_menu() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::prelude::World;
        let mut world = World::new();
        world.insert_resource(crate::menus::menu::MenuState::default());
        world.insert_resource(CurrentSettings::new(Settings::default()));
        world
            .resource_mut::<crate::menus::menu::MenuState>()
            .settings
            .volumes[1] = 0.25;
        world.run_system_once(sync_settings).unwrap();
        assert_eq!(
            world.resource::<CurrentSettings>().get().music_volume(),
            0.25
        );
    }

    /// The screenshot folder hook: F12 writes to the project's folder; without
    /// one, to `--screenshot-dir` (its default `screens`), as before.
    // spec: none (the screenshot folder hook; F12 is a developer's tool, not TOOL-2)
    #[test]
    fn a_project_names_the_screenshot_folder() {
        use crate::tools::shots::{screenshot_path, shot_folder};
        let option = Path::new("screens");
        let photos = PathBuf::from("Documents/S2/frontier/photos");
        assert_eq!(shot_folder(Some(photos.clone()), option), photos);
        assert_eq!(
            shot_folder(Extensions::default().screenshot_dir, option),
            option
        );
        assert_eq!(
            screenshot_path(&photos, "tutorial01", 1_793_000_000),
            photos.join("tutorial01-1793000000.png")
        );
    }

    // spec: GFX-VIEW-2.5 (the defaults are the game's)
    #[test]
    fn the_defaults_change_nothing() {
        let render = RenderOptions::default();
        assert_eq!(render.shadow_map_size, None);
        assert_eq!(render.shadow_cascades, 2);
        assert_eq!(render.anisotropy, 8);
        let e = Extensions::default();
        assert!(e.rules_overlays.is_empty() && e.asset_folders.is_empty() && e.plugins.is_empty());
        assert!(e.menu.is_empty());
        assert_eq!(e.captions, Captions::default());
        assert!(!e.skip_launch_films);
        assert!(!e.music_external && !music_external());
        assert_eq!(e.window_title, None);
        assert_eq!(
            e.settings,
            SettingsOptions {
                profile_write: true,
                app_settings_dir: None,
                profile_dir: None,
            }
        );
        assert!(e.on_settings_change.is_none());
        assert!(!e.map_things);
        assert_eq!(e.screenshot_dir, None);
        assert_eq!(e.save_dir, None);
        assert_eq!(e.project_name, None);
        // No project, no overlays: s2-reborn's empty save mark.
        assert!(s2_ai::save::SaveMark::of(e.project_name.as_deref(), &e.rules_overlays).is_empty());
        assert!(profile_write() && app_settings_dir().is_none() && profile_dir().is_none());
        assert_eq!(window_title(), "OpenS2X");
        assert_eq!(
            picker_title(),
            "Where is The Settlers II: 10th Anniversary installed?"
        );
        assert!(e.captions.start_screen_title.is_none() && e.captions.main_menu_footer.is_none());
    }
}
