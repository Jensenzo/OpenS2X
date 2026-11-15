//! What the interface draws, as plain data (`specs/UI-ingame.md`
//! UI-GAME-1): panels of sprites cut from the original atlases, buttons,
//! texts and sliders at pixel positions. The builders in the other modules
//! describe the screen with these; `render` turns them into Bevy nodes.

use s2_sim::grid::Point;

/// The atlases the interface cuts its sprites from (`data/gfx`,
/// `data/language/<lang>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Atlas {
    Game,
    Elements0,
    Elements1,
    Elements3,
    Elements4,
    Elements5,
    /// `uiBuildings<tribe>.dds`.
    Buildings(u8),
    SmallDialog,
    MediumDialog,
    /// UI-GAME-1.8, 4.3: the military window's frame picture.
    Medium2Dialog,
    BigDialog,
    XxlDialog,
    /// UI-GAME-8.5d: the buildings statistics page's frame.
    XxlDialogTabbed,
    /// UI-GAME-6 (E-ui-036): the economy dialogs' frame.
    DepotLong,
    /// UI-MENU-4.0: the settings pages' frame picture.
    Depot,
    Desktop0,
    /// UI-MENU-1.6: the parchment with the pale town in fog (the tutorial
    /// page).
    Desktop1,
    /// OBS-19: the main menu's parchment with the publisher logos on its
    /// scroll (`uiTitle.dds`).
    Title,
    /// UI-MENU-0.2: the start screen's parchment with the town by a pond.
    Loading0,
    Back,
    Logo,
    DetailTransport,
    MinimapCursor,
    /// CAM: the book dialog's scroll (`uibriefing.dds`).
    Book,
    /// CAM-4.2: the campaign map (`uicampaign.dds`).
    Campaign,
    /// CAM-5.3: the tutorial's pointer and circle.
    TutorialPointer,
    TutorialCircle,
    /// UI-GAME-11: the game screen's frame (`uiframe.dds`).
    Frame,
    /// UI-GAME-4.5: the ship window's porthole mask (`uiShipMask.dds`).
    ShipMask,
    /// CAM-1.3, 3.6: a briefing picture, by its index in [`PICTURES`].
    Picture(u8),
    /// A film's picture (`video::FILM_TITLE`, `FILM_INTRO`), made by the
    /// program.
    Film(u8),
}

/// CAM-1.3, 3.6: the briefing pictures (`data/gfx/<name>.dds`).
pub const PICTURES: [&str; 15] = [
    "ui_briefing_screen_1",
    "ui_briefing_screen_2",
    "ui_briefing_screen_3",
    "ui_briefing_screen_4",
    "ui_briefing_screen_5",
    "ui_briefing_screen_6",
    "ui_briefing_screen_7",
    "ui_briefing_screen_8",
    "ui_briefing_screen_9",
    "ui_briefing_screen_10",
    "ui_briefing_screen_1b",
    "ui_briefing_screen_2b",
    "ui_briefing_screen_3b",
    "ui_briefing_screen_won",
    "ui_briefing_screen_lost",
];

/// The picture atlas of a name the scripts give, if it is one.
pub fn picture(name: &str) -> Option<Atlas> {
    PICTURES
        .iter()
        .position(|p| p.eq_ignore_ascii_case(name))
        .map(|i| Atlas::Picture(i as u8))
}

impl Atlas {
    /// The atlas's file name (looked up without case).
    pub fn file(self) -> &'static str {
        match self {
            Self::Game => "uiGame.dds",
            Self::Elements0 => "uiElements0.dds",
            Self::Elements1 => "uiElements1.dds",
            Self::Elements3 => "uiElements3.dds",
            Self::Elements4 => "uielements4.dds",
            Self::Elements5 => "uielements5.dds",
            Self::Buildings(0) => "uiBuildings0.dds",
            Self::Buildings(1) => "uiBuildings1.dds",
            Self::Buildings(_) => "uiBuildings2.dds",
            Self::SmallDialog => "uiSmallDialog.dds",
            Self::MediumDialog => "uiMediumDialog.dds",
            Self::Medium2Dialog => "uiMedium2Dialog.dds",
            Self::BigDialog => "uiBigDialog.dds",
            Self::XxlDialog => "uiXXLDialog.dds",
            Self::XxlDialogTabbed => "uiXXLDialog_tabbed.dds",
            Self::DepotLong => "uiDepot_long.dds",
            Self::Depot => "uiDepot.dds",
            Self::Desktop0 => "uiDesktop0.dds",
            Self::Desktop1 => "uiDesktop1.dds",
            Self::Title => "uiTitle.dds",
            Self::Loading0 => "uiLoading0.dds",
            Self::Back => "ui_back.dds",
            Self::Logo => "ui_s2logo.dds",
            Self::DetailTransport => "ui_detail_transport.dds",
            Self::MinimapCursor => "uiMinimapCursor.dds",
            Self::Book => "uibriefing.dds",
            Self::Campaign => "uicampaign.dds",
            Self::TutorialPointer => "uiTutorialPointer.dds",
            Self::TutorialCircle => "uiTutorialCircle.dds",
            Self::Frame => "uiframe.dds",
            Self::ShipMask => "uiShipMask.dds",
            Self::Picture(i) => match i {
                0 => "ui_briefing_screen_1.dds",
                1 => "ui_briefing_screen_2.dds",
                2 => "ui_briefing_screen_3.dds",
                3 => "ui_briefing_screen_4.dds",
                4 => "ui_briefing_screen_5.dds",
                5 => "ui_briefing_screen_6.dds",
                6 => "ui_briefing_screen_7.dds",
                7 => "ui_briefing_screen_8.dds",
                8 => "ui_briefing_screen_9.dds",
                9 => "ui_briefing_screen_10.dds",
                10 => "ui_briefing_screen_1b.dds",
                11 => "ui_briefing_screen_2b.dds",
                12 => "ui_briefing_screen_3b.dds",
                13 => "ui_briefing_screen_won.dds",
                _ => "ui_briefing_screen_lost.dds",
            },
            // Not a file: the program draws it.
            Self::Film(_) => "",
        }
    }
}

/// x, y, width, height on an atlas.
pub type Rect = [u32; 4];

/// The fonts of UI-GAME-1.7 (files in `data/language/<lang>/fonts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Font {
    /// Dialog headings: `small_no_alias_white`.
    Heading,
    /// Body text: `light_system`.
    Body,
    /// `light_system_outlined`.
    Outlined,
    /// `dark_system`.
    Dark,
    /// `font_big_white` (the minimap title, menu entries).
    Big,
}

impl Font {
    pub fn file(self) -> &'static str {
        match self {
            Self::Heading => "small_no_alias_white",
            Self::Body => "light_system",
            Self::Outlined => "light_system_outlined",
            Self::Dark => "dark_system",
            Self::Big => "font_big_white",
        }
    }
}

/// UI-GAME-1.7: the heading colour.
pub const HEADING: [u8; 3] = [126, 183, 174];
/// UI-GAME-1.8: the dialog title colour (0.996, 0.937, 0.486).
pub const TITLE: [u8; 3] = [254, 239, 124];
pub const WHITE: [u8; 3] = [255, 255, 255];

/// How a button shows that the mouse is over it or pressing it.
#[derive(Debug, Clone, PartialEq)]
pub enum Hover {
    None,
    /// Another rectangle of the same size and atlas (its x, y).
    Swap([u32; 2]),
}

/// Which value a slider sets (UI-GAME-6, 5.1, UI-MENU-4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SliderKind {
    Tool(u8),
    Distribution {
        table: u8,
        entry: u8,
    },
    Allocation(u8),
    AttackStrength,
    Defenders,
    AttackShare,
    /// UI-GAME-4.4: the headquarters' reserve of soldiers of a rank.
    Castle(u8),
    /// SND-5.1: 0 master, 1 music, 2 UI, 3 sound.
    Volume(u8),
    /// SET-4a, SET-5.3: an engine page slider: the scene field it writes
    /// (1-based, file order).
    Scene(u8),
    /// SET-5.5: the Input page's SCROLL SPEED (SET-2 field 7).
    ScrollSpeed,
}

/// UI-MENU-1.4: a text button's bar: its size and the top-left corners
/// of its normal, hovered and pressed looks in `uiElements0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub size: [f32; 2],
    pub normal: [u32; 2],
    pub hover: [u32; 2],
    pub pressed: [u32; 2],
}

/// UI-MENU-1.4: the menus' 208 x 32 bar: red-brown, blue when hovered,
/// orange when pressed or disabled.
pub const MENU_BAR: Bar = Bar {
    size: [208.0, 32.0],
    normal: [8, 184],
    hover: [8, 224],
    pressed: [8, 264],
};

/// UI-MENU-1.4: the 152 x 22 bar of the smaller dialog buttons, teal when
/// highlighted (the same look while pressed).
pub const DIALOG_BAR: Bar = Bar {
    size: [152.0, 22.0],
    normal: [264, 224],
    hover: [264, 248],
    pressed: [264, 248],
};

/// UI-MENU-1.4: the 224 x 19 bar with a drop-down arrow, teal when
/// highlighted.
pub const DROP_BAR: Bar = Bar {
    size: [224.0, 19.0],
    normal: [264, 8],
    hover: [264, 32],
    pressed: [264, 32],
};

/// UI-MENU-1.4: the 152 x 20 bar (with a drop-down arrow, OBS-20), teal
/// when highlighted.
pub const SLOT_BAR: Bar = Bar {
    size: [152.0, 20.0],
    normal: [264, 176],
    hover: [264, 200],
    pressed: [264, 200],
};

/// Where a text sits on its point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    TopLeft,
    Centre,
    TopCentre,
    /// The text's right end at `at`'s x, its top at `at`'s y.
    TopRight,
}

/// What a control does when clicked (UI-GAME-1.4).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Send a game command (with the refusal box on failure).
    Command(s2_sim::Command),
    Hud(u8),
    CloseContext,
    CloseGlobal,
    OpenGlobal(Global),
    BuildTab(u8),
    BuildingTab(u8),
    StartRoad(Point),
    /// TRN-ROAD-7.1: waterway building from the flag.
    StartWaterway(Point),
    Slider(SliderKind, f32),
    AttackPreset(usize),
    AttackStrong(bool),
    AttackGo,
    /// UI-GAME-11.3: scroll the message rows up (+) or down (-).
    MessagesScroll(i32),
    /// UI-GAME-11.3: switch a message kind's filter (0 military .. 3 misc).
    MessageFilter(usize),
    /// UI-GAME-11.4: a message row: left click jumps to its field, right
    /// click deletes it.
    MessageRow(super::game::MessageKey),
    DeleteMessage(super::game::MessageKey),
    /// Move the camera over a field.
    Focus(Point),
    /// UI-GAME-8.1: the next "For all wares." mode of a depot page.
    DepotAll {
        at: Point,
        settlers: bool,
    },
    /// A menu or setup action (UI-MENU, UI-SETUP).
    Menu(MenuAction),
    /// SND-4.1, UI-GAME-12.2, UI-MENU-5.3: a click on a text input gives
    /// it the keys; the x of its text's origin on screen places the caret.
    FocusInput(Input, f32),
    /// UI-GAME-4.5 d: a fleet arrow of the ship window (`right` the right
    /// one) selects a ship (by its index in the world's list): its window
    /// opens and the camera jumps to it (CAM-5.3a 3).
    ShipArrow {
        ship: usize,
        right: bool,
    },
    /// UI-GAME-8.5: a fleet row: the ship is selected as by `OpenShip` and
    /// the fleet dialog closes.
    FleetShip(usize),
    /// UI-GAME-8.5a: a span option of the player statistics page (its
    /// index in the options).
    StatisticsSpan(u8),
    /// UI-GAME-8.5a: a series option of the player statistics page (the
    /// series' index), kept by the interface.
    StatisticsSeries(u8),
    /// UI-GAME-8.5f: a player's portrait on the player statistics page:
    /// shows or hides his curve.
    StatisticsPlayer(u8),
    /// UI-GAME-8.5b: a span option of the production statistics page.
    ProductionSpan(u8),
    /// UI-GAME-8.5b: a good's toggle: its curve marked or not.
    ProductionMark(u8),
    /// UI-GAME-8.5b: `!Unmark all`.
    ProductionUnmarkAll,
    /// UI-GAME-8.1: the expedition page's "Jump to next expedition".
    /// UI-GAME-8.5g: a statistics page's drop-down button: its list opens,
    /// or closes when open.
    StatsList(u8),
    /// UI-GAME-8.1a (c): select a ship (by its index): the camera moves to
    /// it and its window opens.
    JumpToShip(usize),
    None,
}

/// The text inputs: the save dialog's name (UI-MENU-5.1) and a ship's name
/// (UI-GAME-4.5, by the ship's index in the world's list).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Input {
    SaveName,
    ShipName(usize),
    /// UI-GAME-12.1: the chat line (RETURN), where cheats are typed.
    Chat,
}

/// UI-MENU-5.3: the text input that has the keys, its caret (the
/// characters left of it) and the caret's look this draw: `|`, or `.` on
/// every other draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Focus {
    pub input: Input,
    pub caret: usize,
    pub dot: bool,
}

/// UI-MENU-5.3: a text input's style: its bar's size, the bar art in
/// `uiElements0` (normal, hovered, being edited) and its text's offset
/// from the bar's top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputStyle {
    pub size: [u32; 2],
    pub normal: [u32; 2],
    pub hover: [u32; 2],
    pub edited: Option<[u32; 2]>,
    pub text: [f32; 2],
}

/// UI-MENU-5.3 style 0: 256 x 20, (8, 8), (8, 32), (8, 80); text at (10, 1).
pub const INPUT_STYLE_0: InputStyle = InputStyle {
    size: [256, 20],
    normal: [8, 8],
    hover: [8, 32],
    edited: Some([8, 80]),
    text: [10.0, 1.0],
};

/// UI-MENU-5.3 style 2: the 152 x 18 bar (266, 296) / (266, 320); text at
/// (0, 1).
pub const INPUT_STYLE_2: InputStyle = InputStyle {
    size: [152, 18],
    normal: [266, 296],
    hover: [266, 320],
    edited: None,
    text: [0.0, 1.0],
};

/// What a key did to a text input's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    Typed,
    Deleted,
    /// The caret moved.
    Moved,
    /// RETURN: the input gives the keys and its text back.
    Done,
    /// ESC: the input gives the keys back and clears its result.
    Cancel,
    Ignored,
}

/// UI-MENU-5.3: one pressed key on a focused text input whose caret is
/// `caret` characters in: a printable character (code 32 and above) is
/// inserted at the caret while the text is shorter than `limit` (none:
/// no limit); LEFT / RIGHT move the caret by one character, HOME / END to
/// the ends; DELETE removes the character right of the caret, BACKSPACE
/// the one left of it; RETURN ends the input, ESC ends it and clears the
/// result (what clearing does is not given: the caller keeps the old text,
/// Q-256 e).
/// UI-MENU-5.3: what the restricted character mode lets through: space, digits,
/// A .. Z, a .. z and `äÄöÖüÜß!&()+-_çâáàôéèê`. The characters a file name cannot
/// hold are not among them.
pub fn restricted_char(c: char) -> bool {
    c == ' ' || c.is_ascii_alphanumeric() || "äÄöÖüÜß!&()+-_çâáàôéèê".contains(c)
}

pub fn edit(
    text: &mut String,
    caret: &mut usize,
    key: &bevy::input::keyboard::Key,
    limit: Option<usize>,
    restricted: bool,
) -> Edit {
    use bevy::input::keyboard::Key;
    let n = text.chars().count();
    *caret = (*caret).min(n);
    let byte = |text: &String, k: usize| text.char_indices().nth(k).map_or(text.len(), |(b, _)| b);
    let mut insert = |c: char| {
        if (c as u32) < 32
            || c.is_control()
            || limit.is_some_and(|l| n >= l)
            || (restricted && !restricted_char(c))
        {
            return Edit::Ignored;
        }
        text.insert(byte(text, *caret), c);
        *caret += 1;
        Edit::Typed
    };
    match key {
        Key::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => insert(c),
                _ => Edit::Ignored,
            }
        }
        Key::Space => insert(' '),
        Key::Backspace if *caret > 0 => {
            *caret -= 1;
            text.remove(byte(text, *caret));
            Edit::Deleted
        }
        Key::Delete if *caret < n => {
            text.remove(byte(text, *caret));
            Edit::Deleted
        }
        Key::ArrowLeft => {
            *caret = caret.saturating_sub(1);
            Edit::Moved
        }
        Key::ArrowRight => {
            *caret = (*caret + 1).min(n);
            Edit::Moved
        }
        Key::Home => {
            *caret = 0;
            Edit::Moved
        }
        Key::End => {
            *caret = n;
            Edit::Moved
        }
        Key::Enter => Edit::Done,
        Key::Escape => Edit::Cancel,
        _ => Edit::Ignored,
    }
}

/// UI-MENU-5.3: the caret a click `x` px right of a text's origin puts:
/// before the character that ends the first prefix wider than `x` - 3 px,
/// after the last one if no prefix is.
pub fn caret_at(font: &s2_assets::bmfont::Font, text: &str, x: f32) -> usize {
    let mut width = 0;
    for (i, c) in text.chars().enumerate() {
        width += font.glyphs.get(&(c as u32)).map_or(0, |g| g.x_advance);
        if width as f32 > x - 3.0 {
            return i;
        }
    }
    text.chars().count()
}

/// The menu actions (UI-MENU, UI-SETUP).
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    /// UI-MENU-0.2: START on the start screen.
    StartScreenDone,
    SinglePlayer,
    /// The single-player choices (Q-185 (b)).
    Campaign,
    FreeGame,
    Tutorial,
    /// CAM-4.3: start mission n (1 to 10).
    StartMission(u8),
    /// CAM-5.1: start lesson n (1 to 4).
    StartLesson(u8),
    /// CAM-1.3: the book dialog's page buttons and its close.
    BookPage(i32),
    CloseBook,
    /// CAM-3.6: the end screen's EXIT MAP and CONTINUE (EXIT MAP also ends a
    /// tutorial).
    EndExit,
    EndContinue,
    /// CAM-5: the tutorial window's next and Play.
    TutorialNext,
    TutorialPlay,
    /// CAM-5.2a: the tutorial window's Cancel: back to the TUTORIAL page.
    TutorialCancel,
    Settings,
    Credits,
    Exit,
    /// OBS-20: open (or close) a drop-down of the free play setup.
    OpenDrop(Drop),
    /// Take entry n of a drop-down.
    Choose(Drop, usize),
    /// UI-SETUP-1.4: the next tribe of a slot (its portrait).
    SlotTribe(usize),
    /// The next computer strength of a slot (its emblem, UI-SETUP-1.8).
    SlotStrength(usize),
    /// UI-SETUP-1.8: the next colour of a slot (its colour square).
    SlotColour(usize),
    StartGame,
    /// UI-MENU-1.5: the single-player page's CONTINUE: the autosave.
    Continue,
    Back,
    SaveGame,
    LoadGame,
    SelectSave(usize),
    /// FMT-SAV: continue the chosen save.
    LoadSave,
    /// UI-MENU-5.1: `!Save`: the game under the typed name.
    WriteSave,
    /// UI-MENU-5.1: `!Delete` asks `!Remove this file?`; Yes (true) or No.
    AskDelete,
    Delete(bool),
    /// The save list's arrows: one row up (-1) or down (Q-298 c).
    SaveScroll(i32),
    /// UI-MENU-2.1 (Q-1042): the save list's bar pressed, the pointer this
    /// many px below the bar's top (sent every update while it is held).
    SaveTrack(i32),
    /// UI-MENU-3.3: `!Exit to Title` (false) or `!Exit to Desktop` (true) in the
    /// game menu: the leave dialog.
    AskLeave(bool),
    /// UI-MENU-3.3a (c): the main menu's `!EXIT`: the leave dialog in its
    /// desktop mode, or closed if it is open.
    ExitAsked,
    /// UI-MENU-3.3: the leave dialog's `!No` and its close button.
    StayInGame,
    /// UI-MENU-3.3: the `!Autosave...` box, setting 22.
    AutosaveOnLeave,
    ExitToTitle,
    SettingsPage(u8),
    /// UI-MENU-4.0 (E-ui-034): `!Reset` on settings page n (ours: 0 game, 1
    /// sound) puts that page's values back.
    SettingsReset(u8),
    /// UI-MENU-3.1a: the game menu's close button.
    CloseGameMenu,
    /// UI-MENU-2.1: the free play page's map button opens Select Map.
    OpenSelectMap,
    /// UI-GAME-12.1 (S), SET-5: the settings dialog at its last page.
    SettingsLastPage,
    /// UI-MENU-2.1: a map row of the list (the map's index): takes the map
    /// and closes the dialog.
    SelectMap(usize),
    /// UI-MENU-2.1: the list scrolled by rows (the list box's own
    /// scrolling, Q-256 a).
    MapScroll(i32),
    /// UI-MENU-2.1 (Q-1042): Select Map's bar pressed, the pointer this many
    /// px below the bar's top (sent every update while it is held).
    MapTrack(i32),
    /// The notice's close button.
    CloseNotice,
    /// A project's main menu entry n opens its page (`docs/extending.md`).
    OpenPage(usize),
    /// The player changed row n of a project's page.
    Page(usize, crate::extend::menu::PageInput),
}

/// The drop-downs of the free play setup (OBS-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Drop {
    Map,
    /// UI-MENU-4.1: the game settings page's AUTOSAVE, Infowindow and
    /// DayNightCycle; SET-5.3: the engine page's GRAPHIC QUALITY and
    /// SHADERDETAIL.
    Autosave,
    InfoWindow,
    DayNight,
    Preset,
    ShaderDetail,
    /// SET-5.2: the Graphics page's RESOLUTION and MODE.
    Resolution,
    DisplayMode,
    /// SET-5.5: the Input page's MOUSE SCROLLING (field 6), BORDER
    /// SCROLLING (8), SMART CURSOR (9) and FREE CAMERA (21).
    MouseScrolling,
    BorderScrolling,
    SmartCursor,
    FreeCamera,
    /// SET-5.7: the Network page's INTERNET SPEED.
    InternetSpeed,
    /// ECO-HQ-2.2: the start resources setting.
    Resources,
    Victory,
    Fog,
    /// A slot's start position (HQ), kind and team.
    Hq(usize),
    Kind(usize),
    Team(usize),
}

/// The global dialogs of UI-GAME-6, by their tab icon id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Global {
    Tools,
    Military,
    Distribution(u8),
    Transport,
    /// UI-GAME-8.5: the player statistics, showing one of the eight series.
    Statistics(u8),
    /// UI-GAME-8.5b: the production statistics.
    Production,
    /// UI-GAME-8.5, 7.1: all goods (false) or all settlers (true) in all
    /// depots and the castle.
    Inventory(bool),
    /// UI-GAME-8.5: the fleet list.
    Fleet,
    /// UI-GAME-8.5d: the buildings overview ("Size of village and
    /// productivity").
    Buildings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Sprite {
        atlas: Atlas,
        rect: Rect,
        at: [f32; 2],
        /// Drawn at another size than the rectangle's.
        size: Option<[f32; 2]>,
        /// UI-GAME-1.5: drawn greyed, as the icon of a disabled control.
        greyed: bool,
    },
    Button {
        atlas: Atlas,
        rect: Rect,
        hover: Hover,
        pressed: Hover,
        at: [f32; 2],
        size: Option<[f32; 2]>,
        tooltip: Option<String>,
        action: Action,
        enabled: bool,
        /// Drawn in its pressed state (a chosen tab or option).
        selected: bool,
    },
    Text {
        text: String,
        font: Font,
        /// The point `align` places the text on.
        at: [f32; 2],
        colour: [u8; 3],
        align: Align,
    },
    /// A text that is a button (list rows, choices): teal while the
    /// pointer is over it (UI-MENU-1.4).
    TextButton {
        text: String,
        font: Font,
        at: [f32; 2],
        colour: [u8; 3],
        action: Action,
        enabled: bool,
    },
    /// UI-MENU-1.4: a text button on a bar of `uiElements0` with its
    /// normal, hovered and pressed (or disabled) looks, the label centred
    /// on the bar and raised by `raise` pixels.
    BarButton {
        text: String,
        font: Font,
        at: [f32; 2],
        bar: Bar,
        raise: f32,
        action: Action,
        enabled: bool,
    },
    /// A slider of `width` pixels with its value 0 to 1.
    Slider {
        at: [f32; 2],
        width: f32,
        value: f32,
        kind: SliderKind,
        enabled: bool,
    },
    /// A text wrapped at `width` pixels; `\n` and line breaks start a new
    /// line. `Align::Centre`: each line centred in the width from `at`'s
    /// x, the block's middle on `at`'s y; otherwise from the top left.
    Paragraph {
        text: String,
        font: Font,
        at: [f32; 2],
        width: f32,
        colour: [u8; 3],
        align: Align,
    },
    /// A filled rectangle (stand-ins for art the specs do not name).
    Fill {
        at: [f32; 2],
        size: [f32; 2],
        rgba: [f32; 4],
    },
    /// A picture made by the program: the minimap, a map's preview.
    Picture {
        picture: Picture,
        at: [f32; 2],
        size: f32,
    },
}

/// The pictures the program makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Picture {
    Minimap,
    MapPreview,
    /// UI-GAME-4.5: the ship window's view of its ship, masked by
    /// `uiShipMask`.
    ShipView,
}

/// Which panel: kept between redraws so a panel is only rebuilt when it
/// changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PanelId {
    /// UI-GAME-11: the game screen's frame, under everything.
    Frame,
    /// UI-GAME-9.1: the construction view's labels over the buildings.
    Labels,
    Hud,
    /// UI-GAME-2.7: the speed indicator.
    Hud2,
    Briefing,
    Minimap,
    Messages,
    Context,
    Global,
    Error,
    /// UI-GAME-12.1: the chat line.
    Chat,
    /// CAM-5: the tutorial window and its pointer.
    Tutorial,
    /// CAM-1.3, 3.6: the book dialog and the end screen.
    Book,
    /// UI-GAME-1.11: the paused view's layer over everything.
    Pause,
    Menu,
    /// A notice over the menus or the game (a save loaded in part).
    Notice,
    Tooltip,
}

/// A panel: an origin on the screen (pixels) and its widgets relative to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub id: PanelId,
    pub at: [f32; 2],
    pub widgets: Vec<Widget>,
    /// The tooltips of the panel's bar buttons, by their action (UI-SETUP-1.8:
    /// START's).
    pub tooltips: Vec<(Action, String)>,
}

impl Panel {
    pub fn new(id: PanelId, at: [f32; 2]) -> Self {
        Self {
            id,
            at,
            widgets: Vec::new(),
            tooltips: Vec::new(),
        }
    }

    pub fn paragraph(&mut self, text: impl Into<String>, font: Font, at: [f32; 2], width: f32) {
        self.widgets.push(Widget::Paragraph {
            text: text.into(),
            font,
            at,
            width,
            colour: WHITE,
            align: Align::TopLeft,
        });
    }

    /// A wrapped text centred in the box `width` wide from `left`, its
    /// lines' block centred on `middle`.
    pub fn centred_paragraph(
        &mut self,
        text: impl Into<String>,
        font: Font,
        [left, middle]: [f32; 2],
        width: f32,
        colour: [u8; 3],
    ) {
        self.widgets.push(Widget::Paragraph {
            text: text.into(),
            font,
            at: [left, middle],
            width,
            colour,
            align: Align::Centre,
        });
    }

    pub fn sprite(&mut self, atlas: Atlas, rect: Rect, at: [f32; 2]) {
        self.widgets.push(Widget::Sprite {
            atlas,
            rect,
            at,
            size: None,
            greyed: false,
        });
    }

    pub fn sized(&mut self, atlas: Atlas, rect: Rect, at: [f32; 2], size: [f32; 2]) {
        self.widgets.push(Widget::Sprite {
            atlas,
            rect,
            at,
            size: Some(size),
            greyed: false,
        });
    }

    /// A sprite drawn over a control, greyed with it (UI-GAME-1.5).
    pub fn icon(&mut self, atlas: Atlas, rect: Rect, at: [f32; 2], size: [f32; 2], greyed: bool) {
        self.widgets.push(Widget::Sprite {
            atlas,
            rect,
            at,
            size: Some(size),
            greyed,
        });
    }

    pub fn text(&mut self, text: impl Into<String>, font: Font, at: [f32; 2], colour: [u8; 3]) {
        self.widgets.push(Widget::Text {
            text: text.into(),
            font,
            at,
            colour,
            align: Align::TopLeft,
        });
    }

    /// A text centred in x on `at`, its line's top at `at`'s y: a label
    /// is centred only horizontally (UI-MENU-1.8b).
    pub fn centred_text(
        &mut self,
        text: impl Into<String>,
        font: Font,
        at: [f32; 2],
        colour: [u8; 3],
    ) {
        self.widgets.push(Widget::Text {
            text: text.into(),
            font,
            at,
            colour,
            align: Align::TopCentre,
        });
    }

    /// UI-MENU-5.3: a text input of `style` at `at`: its bar (the edited
    /// look while it has the keys, the hovered one under the pointer), a
    /// click on which gives it the keys, and its text in `light_system`,
    /// with the caret while it has them.
    pub fn text_input(
        &mut self,
        input: Input,
        style: InputStyle,
        text: &str,
        at: [f32; 2],
        focus: Option<Focus>,
    ) {
        let focus = focus.filter(|f| f.input == input);
        let art = match focus {
            Some(_) => style.edited.unwrap_or(style.normal),
            None => style.normal,
        };
        let text_at = [at[0] + style.text[0], at[1] + style.text[1]];
        self.button(
            Atlas::Elements0,
            [art[0], art[1], style.size[0], style.size[1]],
            Hover::Swap(style.hover),
            at,
            None,
            Action::FocusInput(input, self.at[0] + text_at[0]),
            true,
        );
        let shown = match focus {
            Some(f) => {
                let caret = f.caret.min(text.chars().count());
                let left: String = text.chars().take(caret).collect();
                let right: String = text.chars().skip(caret).collect();
                format!("{left}{}{right}", if f.dot { '.' } else { '|' })
            }
            None => text.to_owned(),
        };
        self.text(shown, Font::Body, text_at, [255, 255, 255]);
    }

    /// A text ending at `at`'s x, its top at `at`'s y.
    pub fn right_text(
        &mut self,
        text: impl Into<String>,
        font: Font,
        at: [f32; 2],
        colour: [u8; 3],
    ) {
        self.widgets.push(Widget::Text {
            text: text.into(),
            font,
            at,
            colour,
            align: Align::TopRight,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn button(
        &mut self,
        atlas: Atlas,
        rect: Rect,
        hover: Hover,
        at: [f32; 2],
        tooltip: Option<String>,
        action: Action,
        enabled: bool,
    ) {
        self.widgets.push(Widget::Button {
            atlas,
            rect,
            hover,
            pressed: Hover::None,
            at,
            size: None,
            tooltip,
            action,
            enabled,
            selected: false,
        });
    }

    /// The screen rectangle (x, y, w, h) the panel covers: the union of
    /// its sized widgets (texts are not counted).
    pub fn bounds(&self) -> Option<[f32; 4]> {
        let mut bounds: Option<[f32; 4]> = None;
        for w in &self.widgets {
            let r = match w {
                Widget::Sprite { at, size, rect, .. } | Widget::Button { at, size, rect, .. } => {
                    let size = size.unwrap_or([rect[2] as f32, rect[3] as f32]);
                    [at[0], at[1], size[0], size[1]]
                }
                Widget::Fill { at, size, .. } => [at[0], at[1], size[0], size[1]],
                Widget::BarButton { at, bar, .. } => [at[0], at[1], bar.size[0], bar.size[1]],
                Widget::Picture { at, size, .. } => [at[0], at[1], *size, *size],
                Widget::Slider { at, width, .. } => [at[0], at[1], *width, 8.0],
                _ => continue,
            };
            let (x0, y0, x1, y1) = (
                self.at[0] + r[0],
                self.at[1] + r[1],
                self.at[0] + r[0] + r[2],
                self.at[1] + r[1] + r[3],
            );
            bounds = Some(match bounds {
                None => [x0, y0, x1, y1],
                Some(b) => [b[0].min(x0), b[1].min(y0), b[2].max(x1), b[3].max(y1)],
            });
        }
        bounds.map(|b| [b[0], b[1], b[2] - b[0], b[3] - b[1]])
    }

    /// The buttons, with their action, position and whether enabled
    /// (for tests).
    #[cfg(test)]
    pub fn buttons(&self) -> impl Iterator<Item = (&Action, [f32; 2], bool, Option<&str>)> {
        self.widgets.iter().filter_map(|w| match w {
            Widget::Button {
                action,
                at,
                enabled,
                tooltip,
                ..
            } => Some((action, *at, *enabled, tooltip.as_deref())),
            _ => None,
        })
    }
}
