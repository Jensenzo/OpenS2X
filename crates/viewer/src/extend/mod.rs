//! The extension points for projects built on this engine:
//! rules overlays, asset folders, render options, Bevy plugins, and the
//! hooks of the menus, start-up, music, camera, window, settings and the
//! things on the map, given
//! to [`run_with`](crate::run_with). Not part of the original game; with
//! [`Extensions::default`] everything is exactly the original's (the
//! golden images prove it). What may change and what must not:
//! `docs/extending.md`.

pub(crate) mod menu;
pub(crate) mod messages;
pub(crate) mod things;

use bevy::prelude::{App, DetectChanges, Message, Res, ResMut, Resource, Vec3};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use crate::menus::settings::Settings;
pub use menu::{MenuEntry, MenuPage, OnChange, OnClick, PageWidget};
pub use messages::ProjectEvent;
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
    /// Hook W18: particle effects a project plays at settlers at work (a
    /// builder's dust, a woodcutter's chips; the original has neither,
    /// GFX-OBJ-8.3b). Default none: nothing of it is computed.
    pub effects: Vec<EffectRule>,
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
    /// Hook W24: the text of the message the player gets when a project's
    /// rule finishes its work ([`ProjectEvent`]: a mine's deepening, a
    /// road's pavement), as given (no language key), listed as the
    /// original's misc messages (a jump to the place). Default none: no
    /// message, as the game.
    pub messages: Vec<(ProjectEvent, String)>,
    /// The pictures of the hooks' buttons in a building's window (the
    /// deepening's, W15, and the next recipe's, W20) and of the road menu's
    /// pave button (W19, W34). Default: the first two drawn with the
    /// boat-and-ship switch's picture, the pave button with the
    /// flag-with-hammer picture, as before.
    pub button_pictures: ButtonPictures,
    /// Hook W30: building types drawn with a project's own model (a
    /// `.glb`) in place of their entry of the install's `buildings.xml`.
    /// Default none: every type as the original draws it.
    pub building_looks: Vec<BuildingLook>,
    /// Hook W30b: tree kinds drawn with a project's own model (a `.glb`,
    /// and a far version) in place of their landscape element's. Default
    /// none: every tree as the original draws it.
    pub tree_looks: Vec<TreeLook>,
}

/// Hook W30b: a tree kind's look from a project's own model, a binary glTF
/// file (`.glb`), drawn as the game draws its trees (the instance's turn
/// and three sizes, the growing, the felling and sinking, the sway in the
/// wind, shadows, the day's light). What the file must hold:
/// `docs/extending.md`, "A tree's own model".
#[derive(Debug, Clone, PartialEq)]
pub struct TreeLook {
    /// The tree kind by its name in the rules (`dp_setName`'s: `BirchA`,
    /// `FirB`, ...): drawn with this model in place of its graphics name's
    /// element (`dp_setGfxName`, which stays as the rules have it).
    pub tree: String,
    /// The model: its path below the install (`data/...`, any case), found
    /// in the asset folders first, as the install's files.
    pub model: String,
    /// A lighter model with the same outline, drawn while the tree is
    /// small on the screen. Default none: the model at every size.
    pub far: Option<String>,
    /// The far version is drawn while the tree's size on the screen (the
    /// longest side of its box, as it stands, grows or lies) is below this
    /// share of the view's height. Default [`TREE_FAR_BELOW`].
    pub far_below: f32,
    /// How a young tree grows. Default the original's rule.
    pub growth: TreeGrowth,
}

/// Hook W30b: the default switch to a tree's far version: a quarter of the
/// view's height. The size is the tree's longest side over the view's height
/// at the tree's depth (not its foreshortened picture): in a 16:9 view
/// (29.4 degrees high, GFX-VIEW-3.5) a grown 10 m tree is 16 to 25 percent
/// at the farthest zoom (top to bottom of the view), so far there, and 22 to
/// 33 percent at the default zoom, so full there but along the far edge;
/// 33 percent and more at closer zooms.
pub const TREE_FAR_BELOW: f32 = 0.25;

/// Hook W30b: the scale of a young tree of size s (0 .. 1, its growth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeGrowth {
    /// The original's (GFX-ANIM-3.8b): height s, width min(3 s, 1); a
    /// third-grown tree has its full crown.
    #[default]
    Original,
    /// Height s, width the square root of s: a third-grown tree is 0.58
    /// wide; both reach the full tree at s = 1.
    RootWidth,
}

impl TreeLook {
    /// A tree kind's look from `model`, with no far version and the
    /// original's growing.
    pub fn new(tree: &str, model: &str) -> Self {
        Self {
            tree: tree.into(),
            model: model.into(),
            far: None,
            far_below: TREE_FAR_BELOW,
            growth: TreeGrowth::Original,
        }
    }
}

/// Hook W30 (S2N-003): a building type's look from a project's own model,
/// a binary glTF file (`.glb`), for every tribe. What the file must hold
/// (its frame, the names of its objects, points and clips, the palette and
/// the vertex colours): `docs/extending.md`, "A building's own model".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingLook {
    /// The building type by its name in the rules (`bp_setName`'s:
    /// `Spring` the well, a project type's own): drawn with this model in
    /// place of its graphics name's entry (`bp_setGfxName`, which stays as
    /// the rules have it, so neither the rules nor a save's mark change).
    pub building: String,
    /// The model: its path below the install (`data/...`, any case), found
    /// in the asset folders first, as the install's files.
    pub model: String,
    /// The type whose site model stands while this one is built, by its
    /// graphics name (`Lumberjack`: the woodcutter's, of the builder's
    /// tribe); the house rises out of the ground inside it as the
    /// original's do (GFX-OBJ-2.6b). Default none: no site model, only the
    /// rising house.
    pub site_of: Option<String>,
    /// The effect (`data/scripts/effects/<name>.pfx`) that rises from the
    /// model's point `anchor_smoke` while the building works (`Smoke`).
    /// Default none: no smoke.
    pub smoke: Option<String>,
}

impl BuildingLook {
    /// A type's look from `model`, with no site model and no smoke.
    pub fn new(building: &str, model: &str) -> Self {
        Self {
            building: building.into(),
            model: model.into(),
            site_of: None,
            smoke: None,
        }
    }
}

/// The pictures of the hooks' buttons ([`Extensions::button_pictures`]);
/// each none: the picture as before.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ButtonPictures {
    /// W15: the mine window's Deepen button (none: the boat-and-ship
    /// switch's picture, `uiElements5` (256, 88)).
    pub deepen: Option<ButtonPicture>,
    /// W20: the production window's Next recipe button (none: as Deepen).
    pub recipe: Option<ButtonPicture>,
    /// W34: the road menu's Pave button (W19; none: the flag-with-hammer
    /// portrait, `uiElements3` (368, 160), lit (416, 160)).
    pub pave: Option<ButtonPicture>,
}

/// A button's picture: the 48 x 48 cell at `at` of the picture `file` (a
/// `.dds` by its file name: one of the install's interface sheets, as
/// `uiElements4.dds`, or one in a project's asset folder), and the cell
/// shown while the pointer is over it (none: the same).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonPicture {
    pub file: String,
    pub at: [u32; 2],
    pub hover: Option<[u32; 2]>,
}

/// The hooks' buttons and their pictures, kept for the program's life.
static BUTTONS: OnceLock<(ButtonPictures, Vec<&'static str>)> = OnceLock::new();

/// Keep the buttons' pictures; the first call wins.
pub(crate) fn install_buttons(pictures: ButtonPictures) {
    let mut files: Vec<&'static str> = Vec::new();
    for p in [&pictures.deepen, &pictures.recipe, &pictures.pave]
        .into_iter()
        .flatten()
    {
        if !files.iter().any(|f| f.eq_ignore_ascii_case(&p.file)) {
            files.push(Box::leak(p.file.clone().into_boxed_str()));
        }
    }
    let _ = BUTTONS.set((pictures, files));
}

/// The pictures the buttons name, each loaded once as an interface atlas
/// (`Atlas::Project(index)`).
pub(crate) fn project_pictures() -> &'static [&'static str] {
    BUTTONS.get().map_or(&[], |(_, f)| f.as_slice())
}

/// The file of project picture `index`.
pub(crate) fn project_picture(index: u8) -> &'static str {
    project_pictures()
        .get(usize::from(index))
        .copied()
        .unwrap_or("")
}

/// The buttons' pictures (none until [`install_buttons`]).
pub(crate) fn button_pictures() -> Option<&'static ButtonPictures> {
    BUTTONS.get().map(|(p, _)| p)
}

/// A building window's hook button's atlas, cell and hover cell: the
/// project's picture when it names one (`files` the loaded pictures), else
/// the boat-and-ship switch's.
pub(crate) fn button_cell(
    picture: Option<&ButtonPicture>,
    files: &[&str],
) -> (crate::ui::widgets::Atlas, [u32; 2], Option<[u32; 2]>) {
    button_cell_or(
        picture,
        files,
        (crate::ui::widgets::Atlas::Elements5, [256, 88], None),
    )
}

/// W34: the road menu's pave button's atlas, cell and hover cell: the
/// project's picture when it names one, else the flag-with-hammer portrait.
pub(crate) fn pave_cell(
    picture: Option<&ButtonPicture>,
    files: &[&str],
) -> (crate::ui::widgets::Atlas, [u32; 2], Option<[u32; 2]>) {
    button_cell_or(
        picture,
        files,
        (
            crate::ui::widgets::Atlas::Elements3,
            [368, 160],
            Some([416, 160]),
        ),
    )
}

/// The project's picture when it names one loaded, else `default`.
fn button_cell_or(
    picture: Option<&ButtonPicture>,
    files: &[&str],
    default: (crate::ui::widgets::Atlas, [u32; 2], Option<[u32; 2]>),
) -> (crate::ui::widgets::Atlas, [u32; 2], Option<[u32; 2]>) {
    picture
        .and_then(|p| {
            let i = files.iter().position(|f| f.eq_ignore_ascii_case(&p.file))?;
            Some((
                crate::ui::widgets::Atlas::Project(u8::try_from(i).ok()?),
                p.at,
                p.hover,
            ))
        })
        .unwrap_or(default)
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

/// Hook W18: a particle effect (a `.pfx` script, GFX-OBJ-8.3a) a project
/// plays at a settler of a profession while his work loop runs: started
/// when the loop begins, at his feet as drawn, played on the animation
/// clock (game time) and gone when the loop ends. Its particles are drawn
/// in the effect file's own colours (its start colour times its colour
/// curve) on the texture cell, as the collapse dust (hook W33).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRule {
    /// The effect's path in the game data (`data/scripts/effects/<name>.pfx`,
    /// of an asset folder or the install).
    pub effect: String,
    /// The settler's profession by its script name (`good_constructor` the
    /// builder, `good_woodcutter`; LUA-PROP-1.6).
    pub profession: String,
    /// The work loop it plays during (`WorkLoop<n>`, GFX-ANIM-3.8: 0 the
    /// builder's hammering and the woodcutter's chopping); `None` any.
    pub work_loop: Option<u8>,
}

impl EffectRule {
    /// Whether the rule plays at a settler of profession `profession`
    /// (resolved from the rule's script name) whose logic asks for
    /// `request` this tick.
    pub fn plays(
        &self,
        profession: Option<u32>,
        settler: u32,
        request: Option<s2_sim::trace::Action>,
    ) -> bool {
        profession == Some(settler)
            && matches!(request, Some(s2_sim::trace::Action::WorkLoop(n))
                if self.work_loop.is_none_or(|w| w == n))
    }
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
    /// Hook W17: the map camera's shadow filter (default the game's 2 x 2
    /// percentage-closer filter, GFX-VIEW-2.5c).
    pub shadow_filter: ShadowFilter,
    /// Hook W17: the profile's shadow map size (`SHADOW QUALITY`, GFX-VIEW-2.5 d)
    /// times this factor, at most [`MAX_SHADOW_MAP`]; the Shadows switch,
    /// quality 0 and shader detail 1 still turn shadows off (SET-4b).
    /// `shadow_map_size` wins when both are set. Default `None` (the
    /// profile's size).
    pub shadow_map_scale: Option<u32>,
    /// Hook W18: foam on the water along the shores. Default `None` (the
    /// original has none): the water shader's foam step is not taken.
    pub shore_foam: Option<ShoreFoam>,
    /// Hook W21: the look of a paved road (W19, `World::is_paved`). Default
    /// `None` (the original has no paving): the street pass draws as ever.
    pub paved_road: Option<StreetLook>,
    /// Hook W17b: screen-space ambient occlusion on the map (Bevy's, with
    /// its depth and normal prepasses): the ground's and the objects'
    /// ambient light (GFX-VIEW-2.3's ambient colour, not the sun's) is
    /// multiplied by the occlusion at the pixel, darkening the contact of
    /// houses, trees and stones with the ground. It needs the map camera
    /// without multisampling, so edges are not smoothed while it is on.
    /// Default `false` (the original has none): no prepass, no texture read,
    /// the pixels as without the hook.
    pub ambient_occlusion: bool,
}

/// Hook W21 (S2N-030): the textures a paved road is drawn with, in place
/// of the original's street textures (GFX-OBJ-6.2; the crossings at its
/// flags stay the original's). Each is a `.dds` file name as the install's
/// textures are named (`street.dds`), found in a project's asset folder
/// (`data/textures/...`) or the install; the normal map is read as the
/// original's `street_n.dds` (tangent space, the strip's u across).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreetLook {
    /// The colour texture, its alpha the blend (as `street.dds`).
    pub texture: String,
    /// The normal map (as `street_n.dds`).
    pub normal: String,
}

/// Hook W18 (S2N-005): foam the water shader adds where the ground lies
/// less than `reach` below the water level (raw map height units of the
/// water's depth term, GFX-WATER-4.4: about 2 900 from the waterline to the
/// deepest the term tells apart), fading out towards `reach`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoreFoam {
    /// How deep below the waterline the foam reaches (raw height units; 300
    /// is a narrow band along a flat beach).
    pub reach: f32,
    /// The foam's colour (sRGB, 0 to 1).
    pub colour: [f32; 3],
    /// Its strength at the waterline, 0 to 1 (the share of the foam colour
    /// in the water's colour, and the least alpha there).
    pub strength: f32,
}

/// Hook W17: the shadow map's filter on the map camera.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShadowFilter {
    /// Bevy's 2 x 2 hardware percentage-closer filter: the game's
    /// (GFX-VIEW-2.5c).
    #[default]
    Hardware2x2,
    /// Bevy's wider Gaussian filter: soft edges over several texels.
    Gaussian,
}

impl ShadowFilter {
    /// The camera component that selects the filter.
    pub fn method(self) -> bevy::light::ShadowFilteringMethod {
        match self {
            Self::Hardware2x2 => bevy::light::ShadowFilteringMethod::Hardware2x2,
            Self::Gaussian => bevy::light::ShadowFilteringMethod::Gaussian,
        }
    }
}

/// Hook W17: the largest shadow map a [`RenderOptions::shadow_map_scale`]
/// makes (texels per side; the profile's largest is 2048).
pub const MAX_SHADOW_MAP: usize = 8192;

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            shadow_map_size: None,
            shadow_cascades: 2,
            anisotropy: 8,
            clear_color: [0.02, 0.03, 0.06],
            shadow_filter: ShadowFilter::Hardware2x2,
            shadow_map_scale: None,
            shore_foam: None,
            paved_road: None,
            ambient_occlusion: false,
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
    building_looks: Vec<BuildingLook>,
    tree_looks: Vec<TreeLook>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

/// Keep the asset folders, render options, the music switch and the
/// buildings' and trees' own looks for [`find`], [`render`],
/// [`music_external`], [`building_looks`] and [`tree_looks`]. The first
/// call wins (one game per process).
pub(crate) fn install(
    asset_folders: Vec<PathBuf>,
    render: RenderOptions,
    music_external: bool,
    building_looks: Vec<BuildingLook>,
    tree_looks: Vec<TreeLook>,
) {
    let _ = SHARED.set(Shared {
        asset_folders,
        render,
        music_external,
        building_looks,
        tree_looks,
    });
}

/// Hook W30: the building types' own looks (none until [`install`]).
pub(crate) fn building_looks() -> &'static [BuildingLook] {
    SHARED.get().map_or(&[][..], |s| &s.building_looks[..])
}

/// Hook W30b: the tree kinds' own looks (none until [`install`]).
pub(crate) fn tree_looks() -> &'static [TreeLook] {
    SHARED.get().map_or(&[][..], |s| &s.tree_looks[..])
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

    // spec: none (the hooks' button pictures, W34's pave button too: a project's cell, else the picture as before)
    #[test]
    fn a_hook_button_takes_the_projects_picture() {
        use crate::ui::widgets::Atlas;
        let default = (Atlas::Elements5, [256, 88], None);
        assert_eq!(button_cell(None, &[]), default);
        let pick = ButtonPicture {
            file: "frontier_buttons.dds".into(),
            at: [48, 0],
            hover: Some([48, 48]),
        };
        let files = ["uiElements4.dds", "Frontier_Buttons.dds"];
        assert_eq!(
            button_cell(Some(&pick), &files),
            (Atlas::Project(1), [48, 0], Some([48, 48]))
        );
        // A picture that was not loaded falls back to the switch's.
        assert_eq!(button_cell(Some(&pick), &files[..1]), default);
        // W34: the pave button's, else the flag-with-hammer portrait.
        let portrait = (Atlas::Elements3, [368, 160], Some([416, 160]));
        assert_eq!(pave_cell(None, &files), portrait);
        let paving = ButtonPicture {
            file: "frontier_buttons.dds".into(),
            at: [96, 0],
            hover: Some([96, 48]),
        };
        assert_eq!(
            pave_cell(Some(&paving), &files),
            (Atlas::Project(1), [96, 0], Some([96, 48]))
        );
        assert_eq!(pave_cell(Some(&paving), &files[..1]), portrait);
    }

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

    // spec: none (hook W18: an effect plays only at its profession's work loop)
    #[test]
    fn an_effect_rule_plays_at_its_professions_work_loop() {
        use s2_sim::trace::Action;
        let dust = EffectRule {
            effect: "data/scripts/effects/StoneDust.pfx".into(),
            profession: "good_constructor".into(),
            work_loop: Some(0),
        };
        let builder = Some(11);
        assert!(dust.plays(builder, 11, Some(Action::WorkLoop(0))));
        assert!(!dust.plays(builder, 11, Some(Action::WorkLoop(1))));
        assert!(!dust.plays(builder, 11, Some(Action::Walk)));
        assert!(!dust.plays(builder, 11, None));
        assert!(!dust.plays(builder, 13, Some(Action::WorkLoop(0))));
        assert!(!dust.plays(None, 11, Some(Action::WorkLoop(0))));
        let any = EffectRule {
            work_loop: None,
            ..dust
        };
        assert!(any.plays(builder, 11, Some(Action::WorkLoop(2))));
    }

    // spec: GFX-VIEW-2.5 (the defaults are the game's)
    #[test]
    fn the_defaults_change_nothing() {
        let render = RenderOptions::default();
        assert_eq!(render.shadow_map_size, None);
        assert_eq!(render.shadow_cascades, 2);
        assert_eq!(render.anisotropy, 8);
        // W17: the game's filter and the profile's map size.
        assert_eq!(render.shadow_filter, ShadowFilter::Hardware2x2);
        assert_eq!(
            render.shadow_filter.method(),
            bevy::light::ShadowFilteringMethod::Hardware2x2
        );
        assert_eq!(render.shadow_map_scale, None);
        assert_eq!(render.shore_foam, None);
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
        assert!(e.effects.is_empty());
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
