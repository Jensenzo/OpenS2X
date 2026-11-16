//! The menus (`specs/UI-menus.md`) and the free game's player slots
//! (`specs/UI-game-setup.md`): the main menu, the single-player page with
//! the free play setup (map, start resources, players: kind, tribe, HQ,
//! team, computer strength), the game menu, game and sound settings, and
//! the save and load lists. Layouts are the specs'; where they give none,
//! the stand-in is said (Q-185, Q-211).

mod dialogs;
mod pages;

pub(crate) use dialogs::*;
pub(crate) use pages::*;

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use s2_assets::lang::Texts;
use s2_sim::PlayerId;

use crate::menus::settings::Settings;
use crate::ui::game::GameUi;
use crate::ui::widgets::{
    Action, Atlas, Bar, Drop, Font, Hover, MenuAction, Panel, PanelId, SliderKind, Widget, HEADING,
    WHITE,
};

/// Which menu screen is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// None: the game runs.
    #[default]
    None,
    /// UI-MENU-0.1, 0.3: the intro film at start-up.
    Intro,
    /// UI-MENU-0.2: the start screen with START.
    Start,
    Main,
    /// OBS-20: the single-player page: the free play setup on the left
    /// sheet, CONTINUE, LOAD GAME, CAMPAIGN, TUTORIAL on the scroll.
    SinglePlayer,
    /// CAM-4.2
    Campaign,
    /// CAM-5.1
    Tutorial,
    /// Settings page 0 game, 1 sound; `back` is where it returns to.
    Settings {
        page: u8,
        in_game: bool,
    },
    Credits,
    GameMenu,
    Save,
    Load {
        in_game: bool,
    },
    /// UI-MENU-3.3: the leave dialog, to the desktop or to the title;
    /// UI-MENU-3.3a (c): over the main menu, without a game (its `!EXIT`).
    Leave {
        desktop: bool,
        over_main: bool,
    },
    /// UI-MENU-2.1: the map list over the single-player page.
    SelectMap,
    /// A project's page: the main menu's extra entry n
    /// (`docs/extending.md`, `extend::menu`).
    Extension(usize),
}

/// UI-SETUP-1.1: a slot's kind (open slots are for network games, which
/// the program does not have).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    Human,
    Computer,
    Closed,
}

/// UI-SETUP-1.1: a slot of the setup screen; tribe, start position, team
/// and colour -1 are unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub kind: SlotKind,
    pub strength: u8,
    pub tribe: i8,
    pub start: i8,
    pub team: i8,
    /// UI-SETUP-1.8: 0..7, the slot's colour square (a new computer slot's
    /// is its number, UI-SETUP-1.6 a).
    pub colour: i8,
}

impl Slot {
    /// UI-SETUP-2.1: the start position the slot gets: its own, or its
    /// number while unset.
    pub fn start_of(&self, slot: usize) -> usize {
        usize::try_from(self.start).unwrap_or(slot)
    }

    /// The team its row shows: while unset, its own number (a team of its
    /// own is the same as no team, MIL-SOL-3.1).
    pub fn team_of(&self, slot: usize) -> usize {
        usize::try_from(self.team).unwrap_or(slot)
    }

    /// UI-SETUP-1.8: the colour its square shows and the game gets: its
    /// own, else (unset: the human's slot) its number, the set-up entry's
    /// default colour (UI-SETUP-2.3).
    pub fn colour_of(&self, slot: usize) -> usize {
        usize::try_from(self.colour).unwrap_or(slot) % 8
    }
}

/// UI-SETUP-1.8 'What a click on them does' (T18): a click on a slot's
/// emblem steps its strength, on its portrait its tribe, on its colour
/// square its colour, each wrapping: (s + 1) mod 3, (T + 1) mod 3,
/// (c + 1) mod 8 (Q-1045: a computer slot's first click gives its number
/// + 1). False for any other action.
pub fn slot_click(slots: &mut [Slot], action: &MenuAction) -> bool {
    match *action {
        MenuAction::SlotStrength(i) => {
            if let Some(s) = slots.get_mut(i) {
                s.strength = (s.strength + 1) % 3;
            }
        }
        MenuAction::SlotTribe(i) => {
            if let Some(s) = slots.get_mut(i) {
                s.tribe = (s.tribe.max(0) + 1) % 3;
            }
        }
        MenuAction::SlotColour(i) => {
            if let Some(s) = slots.get_mut(i) {
                s.colour = ((s.colour_of(i) + 1) % 8) as i8;
            }
        }
        _ => return false,
    }
    true
}

/// UI-SETUP-1.6 (a) (corrected 2026-11-14, Q-1045, E-ui-028): a new
/// single-player free game: slot 0 the human with tribe 0 (start position,
/// team and colour unset), slots 1 to 5 computers with strength 0, tribe 0
/// and start position, colour and team all the slot's number.
pub fn single_player_slots() -> [Slot; 6] {
    std::array::from_fn(|i| {
        let own = if i == 0 { -1 } else { i as i8 };
        Slot {
            kind: if i == 0 {
                SlotKind::Human
            } else {
                SlotKind::Computer
            },
            strength: 0,
            tribe: 0,
            start: own,
            team: own,
            colour: own,
        }
    })
}

/// A player of the game to start.
pub use s2_rules::SetupPlayer;

/// UI-SETUP-2.1, 2.5, 2.6: the players of the slots on a map with
/// `starts` start positions: open and closed slots make no player; a
/// computer's strength 0..2 selects AI profile 1..3; an unset tribe is 0.
pub fn setup_players(slots: &[Slot], starts: usize) -> Vec<SetupPlayer> {
    slots
        .iter()
        .enumerate()
        .take(starts)
        .filter_map(|(i, s)| {
            let profile = match s.kind {
                SlotKind::Human => None,
                SlotKind::Computer => Some(s.strength.min(2) + 1),
                SlotKind::Closed => return None,
            };
            Some(SetupPlayer {
                player: PlayerId(i as u8),
                start: s.start_of(i).min(starts - 1),
                tribe: u8::try_from(s.tribe).unwrap_or(0).min(2),
                colour: s.colour_of(i) as u8,
                team: s.team,
                profile,
            })
        })
        .collect()
}

/// UI-SETUP (start checks, E-trn-005): the game starts only when no start
/// position is taken twice.
pub fn starts_distinct(players: &[SetupPlayer]) -> bool {
    players
        .iter()
        .enumerate()
        .all(|(i, a)| players[..i].iter().all(|b| b.start != a.start))
}

/// What the map selection shows of a map.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSummary {
    pub path: PathBuf,
    pub name: String,
    pub size: (u32, u32),
    pub players: usize,
    /// FMT-S2M-2.2: every start position of the map (field x, y).
    pub starts: Vec<(u32, u32)>,
}

/// A message over whatever screen is up, until its close button: a save
/// that loaded only in part or not at all. Ours: what the original does
/// with a save it cannot read is not specified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub lines: Vec<String>,
}

impl Notice {
    /// The notice for the save `name` read with `problems`: `None` when it
    /// read whole; otherwise the first part that did not read, its offset
    /// and how many more there were, titled by whether the game `loaded`.
    pub fn for_save(
        name: &str,
        problems: &[s2_assets::sav::SaveProblem],
        loaded: bool,
    ) -> Option<Notice> {
        let first = problems.first()?;
        let part = match &first.record {
            Some(record) => format!("{}, {record}", first.section),
            None => first.section.to_owned(),
        };
        let more = match problems.len() - 1 {
            0 => String::new(),
            n => format!(", and {n} more"),
        };
        let (title, lines) = if loaded {
            (
                "SAVE LOADED IN PART",
                vec![part, format!("not read from offset {}{more}", first.offset)],
            )
        } else {
            (
                "SAVE NOT LOADED",
                vec![
                    name.to_owned(),
                    format!("{part} at offset {}", first.offset),
                ],
            )
        };
        Some(Notice {
            title: title.to_owned(),
            lines,
        })
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MenuState {
    pub screen: Screen,
    /// A notice over the screen (a save loaded in part or not at all).
    pub notice: Option<Notice>,
    /// The free game maps (`data/maps/freeGameMaps`).
    pub maps: Vec<PathBuf>,
    /// The maps' names (their headers' name, the language files' key).
    pub map_names: Vec<String>,
    pub map_choice: usize,
    /// UI-MENU-2.1: the first row the Select Map list shows; the row under
    /// the pointer and its map's header (`None` when it does not read);
    /// the `!INVALID_MAP` message box is shown.
    pub map_scroll: usize,
    /// UI-MENU-2.1 "How the bar works" (Q-1042): the Select Map bar's value
    /// v, 0..1; the list's first row follows it ([`bar_first`]).
    pub map_bar: f32,
    pub map_hover: Option<(usize, Option<MapSummary>)>,
    pub invalid_map: bool,
    /// SET-5: the settings dialog's last page (UI-GAME-12.1 S opens it).
    pub settings_page: u8,
    /// The chosen map's header, read when chosen.
    pub summary: Option<MapSummary>,
    pub slots: Vec<Slot>,
    /// The free play setup's drop-down choices: start resources (setting
    /// 0..2, Q-211), victory condition, fog of war; and the open drop-down.
    pub resources: u8,
    pub victory: u8,
    pub fog: bool,
    pub drop: Option<Drop>,
    pub settings: Settings,
    /// The saves in the save folder (FMT-SAV-1.1).
    pub saves: Vec<SaveEntry>,
    pub save_choice: usize,
    /// The first row the save list shows (Q-298 c), from its bar's value v
    /// (UI-MENU-2.1, Q-1042: the same bar as Select Map's).
    pub save_scroll: usize,
    pub save_bar: f32,
    /// UI-MENU-5.1: the save dialog's name input.
    pub save_name: String,
    /// UI-MENU-5.1: `!Delete` asks `!Remove this file?`.
    pub delete_ask: bool,
    /// UI-MENU-5.1: `!Save` was pressed: the dialog shows `!Saving...` and
    /// the game is written in the next update.
    pub saving: bool,
    /// UI-MENU-5.1: under 5 MB free on the save drive when the dialog
    /// opened: `!Save` disabled with `!Disk Full!` as its caption, and
    /// `!Delete` disabled.
    pub disk_full: bool,
    /// The last game saved and the last save of ours loaded, with the
    /// state hashes (the play harness's save-quit-load check).
    pub last_save: Option<crate::game::save::SaveCheck>,
    pub last_load: Option<crate::game::save::LoadCheck>,
    /// A game has been started (the game menu can return to it).
    pub in_game: bool,
    /// Menu actions to apply at the next frame (`--free-game`).
    pub pending: Vec<MenuAction>,
    /// UI-MENU-0.3: the intro's frame on screen and its size, while it
    /// runs (`video::play_films`).
    pub intro: Option<(u32, [f32; 2])>,
    /// The action of the enabled button under the pointer (UI-MENU-1.7:
    /// the hovered mission's description).
    pub hovered: Option<crate::ui::widgets::Action>,
    /// UI-MENU-0.3: the intro's subtitles (`data/video/Intro.xml`).
    pub subtitles: s2_assets::subtitles::Subtitles,
    /// A project's main menu entries (`docs/extending.md`); the rows of
    /// the page on screen, and what the player did to them this frame
    /// (`extend::menu::run_page`).
    pub entries: Vec<String>,
    /// A project's lines on the start screen and the main menu.
    pub captions: crate::extend::Captions,
    pub page: Vec<crate::extend::menu::Row>,
    pub page_input: Vec<(usize, crate::extend::menu::PageInput)>,
    /// Hook W22: the part of a project's page on screen (0 when it opens).
    pub page_part: usize,
}

/// UI-MENU-0.1: the screen after the intro.
pub fn after_intro() -> Screen {
    Screen::Start
}

/// UI-MENU-0.1: the first screen when the game starts plainly: the intro
/// (the logo films are left out, project decision), or with a project's
/// `skip_launch_films` (`docs/extending.md`) the screen after it.
pub fn opening_screen(skip_launch_films: bool) -> Screen {
    if skip_launch_films {
        after_intro()
    } else {
        Screen::Intro
    }
}

/// UI-MENU-0.2: the start screen: `ui_back` over the screen, the panel
/// `uiLoading0` (the parchment with the town by a pond), the game logo at
/// (384, 22), `!SplashText0` at (210, 520) in `font_big_white` (229, 209,
/// 196), the legal text `!SplashText1` in a 640 x 360 box at (210, 540)
/// in `dark_system`, and START at (432, 615). No company logo. The data is
/// loaded before the window opens, so START shows at once (the
/// `!SplashText2` line is never needed). A project's `title`
/// (`docs/extending.md`) is a line of the same font and colour 24 px above
/// `!SplashText0`, at (210, 496).
pub fn start_screen(texts: &Texts, screen: [f32; 2], title: Option<&str>) -> Panel {
    let mut p = Panel::new(PanelId::Menu, [0.0, 0.0]);
    background(&mut p, screen);
    let o = desktop_origin(screen);
    p.sized(Atlas::Loading0, [0, 0, 1024, 768], o, [1024.0, 768.0]);
    p.sized(
        Atlas::Logo,
        [0, 0, 256, 256],
        [o[0] + 384.0, o[1] + 22.0],
        [256.0, 256.0],
    );
    if let Some(title) = title {
        p.text(
            title,
            Font::Big,
            [o[0] + START_TITLE[0], o[1] + START_TITLE[1]],
            SPLASH,
        );
    }
    p.text(
        texts.get("!SplashText0"),
        Font::Big,
        [o[0] + 210.0, o[1] + 520.0],
        SPLASH,
    );
    p.widgets.push(Widget::Paragraph {
        text: texts.get("!SplashText1").to_owned(),
        font: Font::Dark,
        at: [o[0] + 210.0, o[1] + 540.0],
        width: 640.0,
        colour: [255, 255, 255],
        align: crate::ui::widgets::Align::TopLeft,
    });
    text_button(
        &mut p,
        texts.get("!Start").to_owned(),
        [o[0] + 432.0, o[1] + 615.0],
        MenuAction::StartScreenDone,
        true,
    );
    p
}

/// UI-MENU-0.2: the colour of the start screen's title line.
pub const SPLASH: [u8; 3] = [229, 209, 196];
/// A project's line on the start screen (`docs/extending.md`): above
/// `!SplashText0` at (210, 520), clear of the picture's logo.
pub const START_TITLE: [f32; 2] = [210.0, 496.0];

/// UI-MENU-0.3: the intro full screen, its shape kept and centred on
/// black, and the subtitle line of the frame on screen in
/// `font_big_white`.
pub fn intro_screen(menu: &MenuState, texts: &Texts, screen: [f32; 2]) -> Panel {
    let mut p = Panel::new(PanelId::Menu, [0.0, 0.0]);
    p.widgets.push(Widget::Fill {
        at: [0.0, 0.0],
        size: screen,
        rgba: [0.0, 0.0, 0.0, 1.0],
    });
    let Some((frame, size)) = menu.intro else {
        return p;
    };
    let rect = crate::films::video::fit(size, screen);
    p.sized(
        Atlas::Film(crate::films::video::FILM_INTRO),
        [0, 0, size[0] as u32, size[1] as u32],
        [rect[0], rect[1]],
        [rect[2], rect[3]],
    );
    if let Some(key) = menu.subtitles.at(frame) {
        // The intro's keys have no `!`: looked up as they are.
        let text = texts.entry(key).unwrap_or(key);
        let text = text.strip_prefix("<hcenter>").unwrap_or(text);
        // The line's height: font_big_white is about 30 px.
        let ([left, width], middle) = crate::films::video::subtitle_box(rect, screen, 30.0);
        p.centred_paragraph(text, Font::Big, [left, middle], width, [255, 255, 255]);
    }
    p
}

impl MenuState {
    /// The state at program start: the main menu (`menu`) or a running
    /// game, or a named screen (for screenshots).
    pub fn start(
        menu: bool,
        screen: Option<&str>,
        free_game: Option<&str>,
        mission: Option<&str>,
        maps: &[PathBuf],
        start_resources: u8,
    ) -> Self {
        let mut state = Self {
            screen: if menu { Screen::Main } else { Screen::None },
            in_game: !menu,
            settings: Settings::load(),
            maps: free_maps(maps),
            // OBS-20: the setup opens on Medium, fog of war on.
            resources: 1,
            fog: true,
            ..Self::default()
        };
        if screen.is_some() {
            open_free_play(&mut state);
        }
        state.screen = match screen {
            Some("select" | "setup" | "single") => Screen::SinglePlayer,
            Some("select-map") => Screen::SelectMap,
            Some("settings") => Screen::Settings {
                page: 0,
                in_game: false,
            },
            Some("sound") => Screen::Settings {
                page: 1,
                in_game: false,
            },
            // SET-5.2, 5.3, 5.5, 5.7: the other settings pages (screenshots).
            Some("engine") => Screen::Settings {
                page: 2,
                in_game: false,
            },
            Some("graphics") => Screen::Settings {
                page: 3,
                in_game: false,
            },
            Some("input") => Screen::Settings {
                page: 4,
                in_game: false,
            },
            Some("network") => Screen::Settings {
                page: 5,
                in_game: false,
            },
            Some("save") => Screen::Save,
            Some("credits") => Screen::Credits,
            Some("campaign") => Screen::Campaign,
            Some("tutorial") => Screen::Tutorial,
            Some("start") => Screen::Start,
            Some("main") => Screen::Main,
            // A project's first page (docs/extending.md).
            Some("extension") => Screen::Extension(0),
            _ => state.screen,
        };
        // UI-MENU-5.1: the save dialog opens with the proposed name.
        if state.screen == Screen::Save {
            state.saves = list_saves();
            state.disk_full = save_disk_full();
            state.save_choice = usize::MAX;
            let map = state.summary.as_ref().map_or("", |s| s.name.as_str());
            state.save_name = default_save_name(map);
        }
        // `--free-game <map>`: the menu's own steps to a free game on it
        // with the default slots: SINGLE PLAYER, the map in its drop-down,
        // the start resources of `--start-resources`, START.
        if let Some(name) = free_game {
            if let Some(i) = state
                .maps
                .iter()
                .position(|p| stem(p).eq_ignore_ascii_case(name))
            {
                state.screen = Screen::Main;
                state.pending = vec![
                    MenuAction::FreeGame,
                    MenuAction::Choose(Drop::Map, i),
                    MenuAction::Choose(Drop::Resources, usize::from(start_resources)),
                    MenuAction::StartGame,
                ];
            }
        }
        // `--mission <name>`: a campaign mission or tutorial lesson started
        // as its menu would.
        if let Some(name) = mission {
            let name = name.to_ascii_lowercase();
            let action = if let Some(n) = name.strip_prefix("campaign").and_then(|n| n.parse().ok())
            {
                Some(MenuAction::StartMission(n))
            } else {
                crate::menus::campaign::lesson_of(&name).map(MenuAction::StartLesson)
            };
            if let Some(a) = action {
                state.screen = Screen::Main;
                state.pending = vec![a];
            }
        }
        state
    }

    pub fn open_game_menu(&mut self) {
        self.screen = Screen::GameMenu;
    }

    pub fn set_volume(&mut self, kind: u8, value: f32) {
        if let Some(v) = self.settings.volumes.get_mut(usize::from(kind)) {
            *v = value.clamp(0.0, 1.0);
        }
        self.settings.save();
    }

    /// SET-5.5: the Input page's SCROLL SPEED (SET-2 field 7, the camera
    /// speed of GFX-VIEW-3.2) moved to `value`; the profile is written.
    pub fn set_scroll_speed(&mut self, value: f32) {
        self.settings.cfg.user.scroll_speed = value.clamp(0.0, 1.0);
        self.settings.save();
    }

    /// SET-4a, SET-5.3: an engine page slider moved to `value`, on its
    /// steps (SET-4: shadow quality 5, shadow detail 7, mirror quality and
    /// details 5, object details and texture quality 4; terrain quality and
    /// draw distance have no stated steps and move freely), with the
    /// slider rules of SET-4a; the profile is written.
    pub fn set_scene_slider(&mut self, field: u8, value: f32) {
        let field = usize::from(field);
        let value = scene_slider_step(field, value);
        self.settings.cfg.scene.move_slider(field, value);
        self.settings.save();
    }
}

/// SET-4: a scene slider's value on its steps (n steps: 0, 1/(n-1), .., 1).
pub fn scene_slider_step(field: usize, value: f32) -> f32 {
    use s2_assets::cfg::field::*;
    let steps = match field {
        SHADOW_QUALITY | MIRROR_QUALITY | MIRROR_DETAILS => 5,
        SHADOW_DETAIL => 7,
        OBJECT_DETAILS | TEXTURE_QUALITY => 4,
        _ => return value.clamp(0.0, 1.0),
    };
    let last = (steps - 1) as f32;
    (value.clamp(0.0, 1.0) * last).round() / last
}

/// UI-MENU-5.1: the rows the save list shows.
pub const SAVE_ROWS: usize = 11;
/// UI-MENU-2.1: the map list's visible rows, and its rows' step (Q-256 a:
/// the plain list box's 21 px).
pub const MAP_ROWS: usize = 11;
pub const MAP_ROW_STEP: f32 = 21.0;
/// UI-MENU-2.1: the step of the `!Map Size:` / `!Players:` lines (not
/// given; the save list's 22 px).
pub const MAP_INFO_STEP: f32 = 22.0;

/// The menu panels, if a menu is up (they replace the game's).
/// The menu screen's panels; `focus` is the text input with the keys.
pub fn panels(
    menu: &MenuState,
    texts: &Texts,
    screen: [f32; 2],
    focus: Option<crate::ui::widgets::Focus>,
) -> Option<Vec<Panel>> {
    let notice = menu.notice.as_ref().map(|n| notice_box(n, screen));
    let page = match menu.screen {
        Screen::None => return notice.map(|n| vec![n]),
        Screen::Intro => intro_screen(menu, texts, screen),
        Screen::Start => start_screen(texts, screen, menu.captions.start_screen_title.as_deref()),
        Screen::Main => main_menu(
            texts,
            screen,
            &menu.entries,
            menu.captions.main_menu_footer.as_deref(),
        ),
        Screen::SinglePlayer => free_play(menu, texts, screen),
        Screen::SelectMap => select_map(menu, texts, screen),
        Screen::Campaign => crate::menus::campaign::campaign_menu(menu, texts, screen),
        Screen::Tutorial => {
            crate::menus::campaign::tutorial_menu(texts, screen, continue_save(menu).is_some())
        }
        Screen::Settings { page, .. } => settings_screen(menu, texts, screen, page),
        Screen::Credits => credits(texts, screen),
        Screen::GameMenu => game_menu(texts, screen),
        // UI-MENU-3.3a (c): over the main menu, whose `!EXIT` closes it again.
        Screen::Leave {
            desktop,
            over_main: true,
        } => {
            let main = main_menu(
                texts,
                screen,
                &menu.entries,
                menu.captions.main_menu_footer.as_deref(),
            );
            let dialog = leave_dialog(menu, texts, screen, desktop, false);
            return Some([main, dialog].into_iter().chain(notice).collect());
        }
        Screen::Leave { desktop, .. } => leave_dialog(menu, texts, screen, desktop, menu.in_game),
        Screen::Save => save_load(menu, texts, screen, true, focus),
        Screen::Load { .. } => save_load(menu, texts, screen, false, None),
        Screen::Extension(n) => extension_page(menu, texts, screen, n),
    };
    Some(std::iter::once(page).chain(notice).collect())
}

/// The header of a map file: name, size, start positions.
fn summary(path: &Path) -> Option<MapSummary> {
    let data = std::fs::read(path).ok()?;
    let map = s2_assets::s2m::load(&path.to_string_lossy(), &data).ok()?;
    Some(MapSummary {
        path: path.to_owned(),
        name: map.info.name.clone(),
        size: (map.info.width, map.info.height),
        players: map.info.start_positions.len().min(6),
        starts: map
            .info
            .start_positions
            .iter()
            .map(|p| (p.x, p.y))
            .collect(),
    })
}

/// Choose a map: its header is read, and the slots start afresh
/// (UI-SETUP-1.6 (a)).
fn choose_map(menu: &mut MenuState, i: usize) {
    menu.map_choice = i.min(menu.maps.len().saturating_sub(1));
    menu.summary = menu.maps.get(menu.map_choice).and_then(|p| summary(p));
    menu.slots = single_player_slots().to_vec();
}

/// The single-player page opens: the maps' names are read once, the first
/// map is chosen if none is.
/// UI-MENU-1.5 (seen in the original's pictures, E-trace-028): SINGLE PLAYER
/// opens its page on CAMPAIGN (the island map of 1.7); FREEPLAY GAME swaps
/// in the free play setup with the free game maps.
fn single_player_apply(menu: &mut MenuState, action: &MenuAction, maps: &[PathBuf]) {
    if *action == MenuAction::FreeGame {
        menu.maps = free_maps(maps);
        open_free_play(menu);
        menu.screen = Screen::SinglePlayer;
    } else {
        menu.screen = Screen::Campaign;
    }
}

fn open_free_play(menu: &mut MenuState) {
    if menu.map_names.len() != menu.maps.len() {
        menu.map_names = menu
            .maps
            .iter()
            .map(|p| summary(p).map_or_else(|| stem(p), |s| s.name))
            .collect();
    }
    if menu.summary.is_none() {
        choose_map(menu, 0);
    }
    if menu.slots.is_empty() {
        menu.slots = single_player_slots().to_vec();
    }
    menu.drop = None;
}

/// FMT-SAV-5.1, SET-2 field 2: the autosave's name, `Autosave` when the
/// profile's is empty.
pub fn autosave_name(menu: &MenuState) -> String {
    match menu.settings.cfg.user.autosave_name.as_str() {
        "" => "Autosave".to_owned(),
        name => name.to_owned(),
    }
}

/// UI-MENU-1.5 (Run D): CONTINUE's save: the profile's autosave name
/// (SET-2 field 2, `Autosave` when empty), if it exists: ours in `own`,
/// or the original's `.sav` in `original` (the same folder unless a project
/// has its own, W11 a); with both, the newer (ours: the original has one
/// autosave file only, FMT-SAV-5.1).
pub fn autosave_file(own: &Path, original: &Path, name: &str) -> Option<PathBuf> {
    let name = if name.is_empty() { "Autosave" } else { name };
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    [
        crate::game::save::path_of(own, name),
        original.join(format!("{name}.sav")),
    ]
    .into_iter()
    .filter(|p| p.is_file())
    .max_by_key(|p| modified(p))
}

/// CONTINUE's save for the profile in `menu`.
pub fn continue_save(menu: &MenuState) -> Option<PathBuf> {
    let own = save_folder()?;
    let original = original_save_folder().unwrap_or_else(|| own.clone());
    autosave_file(&own, &original, &menu.settings.cfg.user.autosave_name)
}

/// SET-5: a settings page chosen (and remembered as the last one), and
/// UI-GAME-12.1 (S): in a game, the settings at their last page (leaving
/// them returns to the game menu); false for every other action.
fn settings_apply(menu: &mut MenuState, action: &MenuAction) -> bool {
    match action {
        MenuAction::SettingsPage(page) => {
            if let Screen::Settings { in_game, .. } = menu.screen {
                menu.screen = Screen::Settings {
                    page: *page,
                    in_game,
                };
                menu.settings_page = *page;
            }
        }
        MenuAction::SettingsLastPage => {
            menu.screen = Screen::Settings {
                page: menu.settings_page,
                in_game: true,
            }
        }
        _ => return false,
    }
    true
}

/// UI-MENU-2.1 "How the bar works" (Q-1042, E-ui-029): the first row a
/// list of `n` entries in 11 rows shows for its bar's value v: floor(v (n -
/// 11) + 0.5); 0 while everything fits.
pub fn bar_first(v: f32, n: usize) -> usize {
    if n <= MAP_ROWS {
        return 0;
    }
    (v * (n - MAP_ROWS) as f32 + 0.5).floor() as usize
}

/// UI-MENU-2.1 (Q-1042): an arrow click moves v by 1 / floor(n / 2), down
/// (`d` > 0) plus, up minus, kept in 0..1; while everything fits v is 0.5.
pub fn bar_step(v: f32, n: usize, d: i32) -> f32 {
    if n <= MAP_ROWS {
        return 0.5;
    }
    let step = 1.0 / (n / 2) as f32;
    (v + step * d.signum() as f32).clamp(0.0, 1.0)
}

/// UI-MENU-2.1 "How the bar works" (Q-1042): one update of a pressed bar
/// (track or thumb) with the pointer `px` below its top: v moves three
/// quarters of the way to p = px / 194, kept in 0..1; while everything fits
/// v is 0.5.
pub fn bar_chase(v: f32, n: usize, px: i32) -> f32 {
    if n <= MAP_ROWS {
        return 0.5;
    }
    let p = px as f32 / 194.0;
    (v + 0.75 * (p - v)).clamp(0.0, 1.0)
}

/// UI-MENU-2.1 (Q-1042): an arrow held down steps at once, then again in
/// the first update in which a 0.05 s countdown, lowered by each update's
/// frame duration, is no longer above 0.
pub const BAR_REPEAT: f32 = 0.05;

/// [`BAR_REPEAT`]'s countdown after an update of `dt` seconds: whether the
/// held arrow steps now, and the countdown left (set again when it steps).
pub fn bar_repeat(countdown: f32, dt: f32) -> (bool, f32) {
    let left = countdown - dt;
    if left <= 0.0 {
        (true, BAR_REPEAT)
    } else {
        (false, left)
    }
}

/// UI-MENU-2.1: the actions of Select Map and of the button that opens it;
/// false for every other action. A click on a row takes its map when its
/// header reads and closes the dialog; otherwise the `!INVALID_MAP` box
/// is shown and the dialog stays. Its close button (Back) leaves the map
/// as it was.
fn select_map_apply(menu: &mut MenuState, action: &MenuAction) -> bool {
    match action {
        MenuAction::OpenSelectMap => {
            // Ours: the bar opens where the chosen map is in view.
            let last = menu.maps.len().saturating_sub(MAP_ROWS);
            let first = menu.map_choice.saturating_sub(MAP_ROWS - 1).min(last);
            menu.map_bar = if last == 0 {
                0.0
            } else {
                first as f32 / last as f32
            };
            menu.map_scroll = bar_first(menu.map_bar, menu.maps.len());
            menu.map_hover = None;
            menu.invalid_map = false;
            menu.screen = Screen::SelectMap;
        }
        MenuAction::SelectMap(_) if menu.invalid_map => {}
        MenuAction::SelectMap(i) => {
            if menu.maps.get(*i).and_then(|p| summary(p)).is_some() {
                if *i != menu.map_choice || menu.summary.is_none() {
                    choose_map(menu, *i);
                }
                menu.map_hover = None;
                menu.screen = Screen::SinglePlayer;
            } else {
                menu.invalid_map = true;
            }
        }
        MenuAction::MapScroll(d) => {
            menu.map_bar = bar_step(menu.map_bar, menu.maps.len(), *d);
            menu.map_scroll = bar_first(menu.map_bar, menu.maps.len());
        }
        MenuAction::MapTrack(px) => {
            menu.map_bar = bar_chase(menu.map_bar, menu.maps.len(), *px);
            menu.map_scroll = bar_first(menu.map_bar, menu.maps.len());
        }
        // The message box closes first, then the dialog.
        MenuAction::Back if menu.screen == Screen::SelectMap && menu.invalid_map => {
            menu.invalid_map = false;
        }
        MenuAction::Back if menu.screen == Screen::SelectMap => {
            menu.map_hover = None;
            menu.screen = Screen::SinglePlayer;
        }
        _ => return false,
    }
    menu.drop = None;
    true
}

/// UI-MENU-2.1: while the pointer is over Select Map's row `row`, that
/// map's header is read (once per row); over no row it is cleared.
pub fn hover_map(menu: &mut MenuState, row: Option<usize>) {
    if menu.map_hover.as_ref().map(|h| h.0) == row {
        return;
    }
    menu.map_hover = row.map(|i| (i, menu.maps.get(i).and_then(|p| summary(p))));
}

/// The map whose size, players and preview are shown: on Select Map the
/// one under the pointer (UI-MENU-2.1), elsewhere the chosen one.
pub fn shown_map(menu: &MenuState) -> Option<&MapSummary> {
    if menu.screen == Screen::SelectMap {
        menu.map_hover.as_ref().and_then(|h| h.1.as_ref())
    } else {
        menu.summary.as_ref()
    }
}

pub fn apply(
    action: &MenuAction,
    menu: &mut MenuState,
    viewer: &mut crate::game::Viewer,
    ui: &mut GameUi,
    mission: &mut crate::menus::campaign::MissionUi,
    exit: &mut MessageWriter<bevy::app::AppExit>,
) {
    if settings_apply(menu, action) || select_map_apply(menu, action) {
        return;
    }

    // A drop-down closes on any click but its own bar's.
    let open = menu.drop.take();
    match action {
        MenuAction::SinglePlayer | MenuAction::FreeGame => {
            single_player_apply(menu, action, &viewer.maps)
        }
        MenuAction::OpenDrop(d) => {
            if open != Some(*d) {
                menu.drop = Some(*d);
            }
        }
        MenuAction::Choose(d, n) => choose(menu, *d, *n),
        MenuAction::CloseGameMenu => menu.screen = Screen::None,
        // Taken by `select_map_apply`.
        MenuAction::OpenSelectMap
        | MenuAction::SelectMap(_)
        | MenuAction::MapScroll(_)
        | MenuAction::MapTrack(_) => {}
        MenuAction::Continue => {
            if let Some(path) = continue_save(menu) {
                viewer.save_file = Some(path);
                viewer.setup = None;
                viewer.reload = true;
                viewer.paused = false;
                *ui = ui.for_new_game();
                menu.in_game = true;
                menu.screen = Screen::None;
            }
        }
        MenuAction::StartScreenDone => menu.screen = Screen::Main,
        MenuAction::Campaign => menu.screen = Screen::Campaign,
        MenuAction::Tutorial => menu.screen = Screen::Tutorial,
        MenuAction::StartMission(n) => {
            if crate::menus::campaign::unlocked(&menu.settings, *n) {
                start_mission(&crate::menus::campaign::mission_name(*n), menu, viewer, ui);
            }
        }
        MenuAction::StartLesson(n) => {
            start_mission(&format!("tutorial{n:02}"), menu, viewer, ui);
        }
        MenuAction::BookPage(d) => {
            if let Some(b) = mission.book.as_mut() {
                b.page =
                    (b.page as i64 + i64::from(*d)).clamp(0, b.pages.len() as i64 - 1) as usize;
            }
        }
        MenuAction::CloseBook => {
            if let Some(screen) = mission.close_book() {
                viewer.paused = true;
                menu.in_game = false;
                menu.screen = screen;
            }
        }
        MenuAction::EndContinue => {
            // CAM-3.6b: the victory book's CONTINUE closes it and the
            // mission goes on (the mission panel then says VICTORY).
            let book = mission.book.as_ref().is_some_and(|b| b.end.is_some());
            if book {
                mission.book = None;
                mission.end = None;
                viewer.paused = false;
            } else if mission.end != Some(crate::menus::campaign::EndScreen::Victory(4)) {
                mission.end = None;
                viewer.paused = false;
            }
        }
        MenuAction::EndExit => {
            viewer.paused = true;
            *ui = ui.for_new_game();
            if let Some(screen) = mission.exit_map() {
                menu.in_game = false;
                menu.screen = screen;
            }
        }
        MenuAction::TutorialNext => {
            if let Some(t) = mission.tutorial.as_mut() {
                t.advance();
            }
        }
        MenuAction::TutorialPlay => mission.tutorial = None,
        // CAM-5.2a: Cancel ends the lesson; the single player page's
        // TUTORIAL page follows.
        MenuAction::TutorialCancel => {
            viewer.paused = true;
            *ui = ui.for_new_game();
            mission.tutorial = None;
            mission.end = None;
            mission.book = None;
            menu.in_game = false;
            menu.screen = Screen::Tutorial;
        }
        MenuAction::SlotTribe(_) | MenuAction::SlotStrength(_) | MenuAction::SlotColour(_) => {
            slot_click(&mut menu.slots, action);
        }
        MenuAction::StartGame => {
            let Some(summary) = menu.summary.clone() else {
                return;
            };
            let players = setup_players(&menu.slots, summary.players);
            if players.is_empty() || !starts_distinct(&players) {
                return;
            }
            viewer.start_resources = menu.resources;
            viewer.start_target = menu.victory;
            viewer.start_fog_off = !menu.fog;
            if let Some(index) = viewer.maps.iter().position(|p| *p == summary.path) {
                viewer.current = index;
            }
            viewer.setup = Some(players);
            viewer.reload = true;
            viewer.paused = false;
            *ui = ui.for_new_game();
            menu.in_game = true;
            menu.screen = Screen::None;
        }
        MenuAction::Back => {
            menu.screen = match menu.screen {
                Screen::Campaign | Screen::Tutorial | Screen::Load { in_game: false } => {
                    Screen::SinglePlayer
                }
                Screen::Settings { in_game: true, .. }
                | Screen::Save
                | Screen::Load { in_game: true } => Screen::GameMenu,
                // UI-MENU-3.3a (d): only the dialog closes; the game menu,
                // closed when it opened, does not come back.
                Screen::Leave {
                    over_main: true, ..
                } => Screen::Main,
                Screen::Leave { .. } => Screen::None,
                _ => Screen::Main,
            }
        }
        MenuAction::Settings => {
            menu.screen = Screen::Settings {
                page: 0,
                in_game: menu.screen == Screen::GameMenu,
            }
        }
        // Taken by `settings_apply`.
        MenuAction::SettingsPage(_) | MenuAction::SettingsLastPage => {}
        MenuAction::SettingsReset(page) => {
            if reset_settings_page(&mut menu.settings, *page) {
                menu.settings.save();
            }
        }
        MenuAction::Credits => menu.screen = Screen::Credits,
        MenuAction::SaveGame => {
            menu.saves = list_saves();
            menu.save_scroll = 0;
            menu.save_bar = 0.0;
            menu.save_choice = usize::MAX;
            menu.save_name = default_save_name(played_map_name(viewer));
            menu.delete_ask = false;
            menu.saving = false;
            menu.disk_full = save_disk_full();
            menu.screen = Screen::Save;
            // UI-MENU-5.1: the input has the keys from the start.
            ui.input = Some(crate::ui::widgets::Input::SaveName);
            ui.caret = menu.save_name.chars().count();
        }
        MenuAction::LoadGame => {
            menu.saves = list_saves();
            menu.save_scroll = 0;
            menu.save_bar = 0.0;
            menu.save_choice = usize::MAX;
            menu.delete_ask = false;
            menu.screen = Screen::Load {
                in_game: menu.screen == Screen::GameMenu,
            };
        }
        MenuAction::SelectSave(i) => {
            menu.save_choice = *i;
            menu.delete_ask = false;
            // UI-MENU-5.1: the name input takes the chosen file's name.
            if menu.screen == Screen::Save {
                if let Some(entry) = menu.saves.get(*i) {
                    menu.save_name = entry.name.clone();
                }
            }
        }
        // UI-MENU-5.1: the game under the typed name, then back to it; the game
        // menu does not come back. A save of the same name is written over
        // without a question. The name's characters are the restricted mode's.
        // UI-MENU-5.1: `!Save` (or RETURN, `save-game`) hides the controls
        // and shows `!Saving...`; the game is written in the next update
        // (`finish_save`).
        MenuAction::WriteSave => {
            if !menu.save_name.is_empty() && !menu.disk_full && save_folder().is_some() {
                menu.saving = true;
                ui.input = None;
            }
        }
        MenuAction::AskDelete => menu.delete_ask = menu.save_choice < menu.saves.len(),
        MenuAction::SaveScroll(d) => {
            menu.save_bar = bar_step(menu.save_bar, menu.saves.len(), *d);
            menu.save_scroll = bar_first(menu.save_bar, menu.saves.len());
        }
        MenuAction::SaveTrack(px) => {
            menu.save_bar = bar_chase(menu.save_bar, menu.saves.len(), *px);
            menu.save_scroll = bar_first(menu.save_bar, menu.saves.len());
        }
        MenuAction::Delete(yes) => {
            if let (true, Some(entry)) = (*yes, menu.saves.get(menu.save_choice)) {
                let Some(file) = save_entry_file(entry) else {
                    return;
                };
                if let Err(e) = std::fs::remove_file(file) {
                    warn!("{}: {e}", entry.name);
                }
                menu.saves = list_saves();
                menu.save_choice = usize::MAX;
                menu.save_scroll = bar_first(menu.save_bar, menu.saves.len());
                // UI-MENU-5.1: `Yes` puts the default name back.
                if menu.screen == Screen::Save {
                    menu.save_name = default_save_name(played_map_name(viewer));
                }
            }
            menu.delete_ask = false;
        }
        MenuAction::LoadSave => {
            let Some(entry) = menu.saves.get(menu.save_choice) else {
                return;
            };
            let Some(file) = save_entry_file(entry) else {
                return;
            };
            viewer.save_file = Some(file);
            viewer.setup = None;
            viewer.reload = true;
            viewer.paused = false;
            *ui = ui.for_new_game();
            menu.in_game = true;
            menu.screen = Screen::None;
        }
        MenuAction::AskLeave(_) | MenuAction::ExitAsked | MenuAction::StayInGame => {
            if let Some(next) = leave_screen(action, menu.screen) {
                menu.screen = next;
            }
        }
        MenuAction::AutosaveOnLeave => {
            menu.settings.autosave_on_leave = !menu.settings.autosave_on_leave;
            menu.settings.save();
        }
        MenuAction::ExitToTitle => {
            leave_game(menu, viewer, mission);
            viewer.paused = true;
            *ui = ui.for_new_game();
            menu.screen = Screen::Main;
        }
        MenuAction::Exit => {
            // UI-MENU-3.3a (c): from the main menu there is no game to save.
            let over_main = matches!(
                menu.screen,
                Screen::Leave {
                    over_main: true,
                    ..
                }
            );
            if !over_main {
                leave_game(menu, viewer, mission);
            }
            exit.write(bevy::app::AppExit::Success);
        }
        MenuAction::CloseNotice => menu.notice = None,
        MenuAction::OpenPage(n) => {
            menu.screen = Screen::Extension(*n);
            menu.page_part = 0;
        }
        MenuAction::PagePart(part) => menu.page_part = *part,
        MenuAction::Page(row, input) => menu.page_input.push((*row, *input)),
    }
    hold_or_run(menu, viewer, mission);
}

/// The game does not run behind the menus, a notice or an end screen; with
/// none of them it runs.
fn hold_or_run(
    menu: &MenuState,
    viewer: &mut crate::game::Viewer,
    mission: &crate::menus::campaign::MissionUi,
) {
    if menu.screen != Screen::None && menu.screen != Screen::GameMenu || menu.notice.is_some() {
        viewer.paused = true;
    }
    if menu.screen == Screen::None && menu.in_game && menu.notice.is_none() && mission.end.is_none()
    {
        viewer.paused = false;
    }
}

/// UI-MENU-3.3, 3.3a: where the leave dialog's ways in and out lead: the game
/// menu's two exits open it; the main menu's `!EXIT` opens it in its desktop mode
/// over the main menu, and closes it while it is open (c); `!No` and the close
/// button close only this dialog: the main menu stays, or the game goes on (d).
/// UI-MENU-5.1: the save asked for in the last update: the game under the
/// typed name, then back to it; the game menu does not come back. A save of
/// the same name is written over without a question. The name's characters
/// are the restricted mode's.
pub fn finish_save(
    menu: &mut MenuState,
    viewer: &mut crate::game::Viewer,
    mission: &crate::menus::campaign::MissionUi,
) {
    if !menu.saving {
        return;
    }
    menu.saving = false;
    let name = menu.save_name.clone();
    let Some(folder) = save_folder() else { return };
    match crate::game::save::save_game(viewer, mission, &folder, &name) {
        Ok(check) => {
            menu.last_save = Some(check);
            menu.screen = Screen::None;
        }
        Err(e) => {
            menu.notice = Some(Notice {
                title: "SAVE NOT WRITTEN".to_owned(),
                lines: vec![name, e],
            })
        }
    }
    // Back in the game it runs again, as after any menu action.
    hold_or_run(menu, viewer, mission);
}

pub(crate) fn leave_screen(action: &MenuAction, screen: Screen) -> Option<Screen> {
    let over_main = matches!(
        screen,
        Screen::Leave {
            over_main: true,
            ..
        }
    );
    Some(match action {
        MenuAction::AskLeave(desktop) => Screen::Leave {
            desktop: *desktop,
            over_main: false,
        },
        MenuAction::ExitAsked if over_main => Screen::Main,
        MenuAction::ExitAsked => Screen::Leave {
            desktop: true,
            over_main: true,
        },
        MenuAction::StayInGame if over_main => Screen::Main,
        MenuAction::StayInGame => Screen::None,
        _ => return None,
    })
}

/// FMT-SAV-5.3, UI-MENU-3.3: `!Yes` in the leave dialog writes the autosave when
/// its `!Autosave...` box is on (setting 22). The interval plays no part.
fn leave_game(
    menu: &MenuState,
    viewer: &crate::game::Viewer,
    mission: &crate::menus::campaign::MissionUi,
) {
    if !menu.in_game || !menu.settings.autosave_on_leave || !crate::game::save::autosaves(viewer) {
        return;
    }
    if let Some(folder) = save_folder() {
        if let Err(e) = crate::game::save::save_game(viewer, mission, &folder, &autosave_name(menu))
        {
            warn!("the autosave on leaving was not written: {e}");
        }
    }
}

/// UI-MENU-4.0 (E-ui-034, corrected 2026-11-14 b): `!Reset` acts on its own
/// page (ours: 0 game, 1 sound): the game page's autosave 10 minutes,
/// Infowindow Off (field 17 = 1), mouse scrolling on, the autosave on
/// leaving off and field 3 1.0 (DayNightCycle kept); the sound page's four
/// volumes their defaults (SET-3; the device's, SND-5.1, medium). False for
/// a page whose Reset does nothing.
pub(crate) fn reset_settings_page(s: &mut Settings, page: u8) -> bool {
    match page {
        0 => {
            s.autosave = 600;
            s.info_window = false;
            s.cfg.user.input_a = true;
            s.autosave_on_leave = false;
            s.cfg.user.unused = 1.0;
            true
        }
        1 => {
            s.volumes = [1.0, 0.3, 1.0, 1.0];
            true
        }
        // SET-5.5, UI-MENU-4.0: the Input page: field 6 off, 7 = 0.25, 8 on,
        // 9 on, 21 off.
        4 => {
            let user = &mut s.cfg.user;
            user.input_b = false;
            user.scroll_speed = 0.25;
            user.input_c = true;
            user.input_d = true;
            user.unknown_bools[1] = false;
            true
        }
        // SET-5.7: shown on the Network page and does nothing; the Graphics
        // page has none (SET-5.2).
        _ => false,
    }
}

/// Take entry `n` of a drop-down.
fn choose(menu: &mut MenuState, drop: Drop, n: usize) {
    match drop {
        Drop::Map => choose_map(menu, n),
        Drop::Resources => menu.resources = n.min(3) as u8,
        Drop::Victory => menu.victory = n.min(3) as u8,
        Drop::Fog => menu.fog = n == 0,
        // UI-MENU-4.1, SET-5.3: a settings choice is kept in the profile.
        Drop::Autosave => {
            menu.settings.autosave = AUTOSAVES.get(n).map_or(0, |(secs, _)| *secs);
            menu.settings.save();
        }
        Drop::InfoWindow => {
            menu.settings.info_window = n == 0;
            menu.settings.save();
        }
        Drop::DayNight => {
            menu.settings.day_night = n == 0;
            menu.settings.save();
        }
        // SET-4a: a preset is applied when it changes.
        Drop::Preset => {
            let preset = n.min(PRESET_NAMES.len() - 1) as u32;
            if menu.settings.cfg.scene.preset() != preset {
                menu.settings.cfg.scene.apply_preset(preset);
                menu.settings.save();
            }
        }
        Drop::ShaderDetail => {
            menu.settings
                .cfg
                .scene
                .set_shader_detail(if n == 0 { 2 } else { 3 });
            menu.settings.save();
        }
        // SET-5.5: the Input page's switches, kept in the profile (fields 6,
        // 8, 9, 21; the first entry sets each true).
        Drop::MouseScrolling | Drop::BorderScrolling | Drop::SmartCursor | Drop::FreeCamera => {
            let user = &mut menu.settings.cfg.user;
            let field = match drop {
                Drop::MouseScrolling => &mut user.input_b,
                Drop::BorderScrolling => &mut user.input_c,
                Drop::SmartCursor => &mut user.input_d,
                _ => &mut user.unknown_bools[1],
            };
            *field = n == 0;
            menu.settings.save();
        }
        // SET-5.2, 5.7: ours keeps its window as it is (stand-in) and plays
        // no network game: these choices change nothing.
        Drop::Resolution | Drop::DisplayMode | Drop::InternetSpeed => {}
        Drop::Hq(i) => {
            if let Some(s) = menu.slots.get_mut(i) {
                s.start = n.min(5) as i8;
            }
        }
        Drop::Team(i) => {
            if let Some(s) = menu.slots.get_mut(i) {
                s.team = n.min(5) as i8;
            }
        }
        // The human's slot stays the human's.
        Drop::Kind(0) => {}
        Drop::Kind(i) => {
            if let Some(s) = menu.slots.get_mut(i) {
                s.kind = if n == 0 {
                    // UI-SETUP-1.6 (b): a new computer slot has strength 0
                    // and the values its row shows.
                    if s.kind != SlotKind::Computer {
                        s.strength = 0;
                        s.tribe = s.tribe.max(0);
                    }
                    SlotKind::Computer
                } else {
                    SlotKind::Closed
                };
            }
        }
    }
}

/// CAM-4.3, 4.3a, UI-SETUP-2.4: start a campaign mission or tutorial lesson by
/// its map name: the players of the map's records, or for a lesson player 0
/// human and the others computer players; its start stock.
fn start_mission(
    name: &str,
    menu: &mut MenuState,
    viewer: &mut crate::game::Viewer,
    ui: &mut GameUi,
) {
    let Some(index) = viewer
        .maps
        .iter()
        .position(|p| stem(p).eq_ignore_ascii_case(name) && s2_rules::mission::map_kind(p) != 0)
    else {
        warn!("no map {name}");
        return;
    };
    let path = viewer.maps[index].clone();
    let kind = s2_rules::mission::map_kind(&path);
    let map = std::fs::read(&path)
        .ok()
        .and_then(|data| s2_assets::s2m::load(&path.to_string_lossy(), &data).ok());
    let Some(map) = map else {
        warn!("map {name} does not read");
        return;
    };
    viewer.current = index;
    viewer.setup = Some(s2_rules::mission::players(&map, kind));
    // CAM-4.3: nothing carries over from an earlier game.
    viewer.start_resources = s2_rules::mission::start_resources(kind);
    viewer.reload = true;
    viewer.paused = false;
    *ui = ui.for_new_game();
    menu.in_game = true;
    menu.screen = Screen::None;
}

#[cfg(test)]
mod tests;
