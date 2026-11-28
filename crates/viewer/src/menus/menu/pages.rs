//! The main menu and the setup pages: background, buttons, the single
//! player and free play pages, Select Map (UI-MENU-1, 2, UI-SETUP-1.8).

use super::*;

/// The free game maps of an install, sorted by name.
pub fn free_maps(maps: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = maps
        .iter()
        .filter(|p| {
            p.parent()
                .and_then(Path::file_name)
                .is_some_and(|d| d.to_string_lossy().eq_ignore_ascii_case("freeGameMaps"))
        })
        .cloned()
        .collect();
    out.sort_by_key(|p| map_sort_key(p));
    out
}

/// UI-MENU-2.1 (Q-211 f, T20): a map's place in Select Map's list: its
/// file name cut at the first `.`, `a`..`z` turned into `A`..`Z`, compared
/// byte by byte, shorter first when one is the start of the other (so `_`
/// sorts after every letter: `AB` before `A_B`).
pub fn map_sort_key(p: &Path) -> Vec<u8> {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let cut = name.split('.').next().unwrap_or_default();
    cut.bytes().map(|b| b.to_ascii_uppercase()).collect()
}

/// UI-MENU-2.1 "How the rows look" (T22): a row's text colour by the
/// map's kind. Ours lists the free game folder only: a file name starting
/// with `MP_` (case as written) is tan, `sp` and a digit pale pink-grey,
/// any other (253, 139, 107); a file that is gone red.
pub fn map_row_colour(p: &Path) -> [u8; 3] {
    if !p.as_os_str().is_empty() && p.is_absolute() && !p.exists() {
        return MAP_ROW_MISSING;
    }
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let b = name.as_bytes();
    if name.starts_with("MP_") {
        MAP_ROW_MP
    } else if b.len() > 2 && b.starts_with(b"sp") && b[2].is_ascii_digit() {
        MAP_ROW_SP
    } else {
        MAP_ROW_OTHER
    }
}

/// UI-MENU-2.1: the rows' text colours by kind.
pub const MAP_ROW_MP: [u8; 3] = [238, 189, 115];
pub const MAP_ROW_SP: [u8; 3] = [219, 199, 186];
pub const MAP_ROW_OTHER: [u8; 3] = [253, 139, 107];
pub const MAP_ROW_MISSING: [u8; 3] = [255, 0, 0];
/// UI-MENU-2.1: a row's bar, `uiElements0` (8, 8, 248 x 20), and under the
/// pointer (8, 32).
pub const MAP_ROW_BAR: [u32; 4] = [8, 8, 248, 20];
pub const MAP_ROW_BAR_HOVER: [u32; 2] = [8, 32];
/// UI-MENU-2.1: the text's left edge in the bar ("about 9 px").
pub const MAP_ROW_TEXT_X: f32 = 9.0;
/// UI-MENU-2.1, 1.8 b: the text's line top in the bar: the bar's middle
/// (10) less half of `light_system`'s 16 px line.
pub const MAP_ROW_TEXT_DROP: f32 = 2.0;

pub(crate) fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The 1024 x 768 parchment's corner, centred (UI-MENU-1.1).
pub(crate) fn desktop_origin(screen: [f32; 2]) -> [f32; 2] {
    [
        ((screen[0] - 1024.0) / 2.0).floor(),
        ((screen[1] - 768.0) / 2.0).floor(),
    ]
}

/// The part of the main menu's scroll that holds the publisher logos
/// (panel x, y, width, height), covered from `uiDesktop0`.
pub(crate) const LOGO_PATCH: [u32; 4] = [690, 595, 230, 100];

/// The main menu's parchment and logo, without its buttons: the
/// parchment `uiTitle` with its publisher logos covered (OBS-19;
/// the town picture on the left sheet is not drawn, Q-211).
pub fn main_menu_background(screen: [f32; 2]) -> Panel {
    let mut p = Panel::new(PanelId::Menu, [0.0, 0.0]);
    background(&mut p, screen);
    let o = desktop_origin(screen);
    p.sized(Atlas::Title, [0, 0, 1024, 768], o, [1024.0, 768.0]);
    // UI-MENU-0.1 (project decision): no publisher logos. The scroll's
    // bottom is taken from the single-player parchment, which has the same
    // scroll without them.
    p.sized(
        Atlas::Desktop0,
        LOGO_PATCH,
        [o[0] + LOGO_PATCH[0] as f32, o[1] + LOGO_PATCH[1] as f32],
        [LOGO_PATCH[2] as f32, LOGO_PATCH[3] as f32],
    );
    // UI-MENU-1.1a: the town film, 616 x 665 at (56, 62), 1:1.
    p.sized(
        Atlas::Film(crate::films::video::FILM_TITLE),
        [0, 0, 616, 665],
        [o[0] + 56.0, o[1] + 62.0],
        [616.0, 665.0],
    );
    p.sized(
        Atlas::Logo,
        [0, 0, 256, 256],
        [o[0] + 677.0, o[1] + 17.0],
        [256.0, 256.0],
    );
    p
}

/// UI-MENU-1.1's background, `ui_back`, stretched over the whole screen
/// (Q-189 c).
pub fn background(p: &mut Panel, screen: [f32; 2]) {
    p.sized(Atlas::Back, [0, 0, 512, 512], [0.0, 0.0], screen);
}

/// The full-screen background and the parchment.
pub(crate) fn desktop(screen: [f32; 2]) -> Panel {
    let mut p = Panel::new(PanelId::Menu, [0.0, 0.0]);
    background(&mut p, screen);
    p.sized(
        Atlas::Desktop0,
        [0, 0, 1024, 768],
        desktop_origin(screen),
        [1024.0, 768.0],
    );
    p
}

/// UI-MENU-1.4: a menu text button: the 208 x 32 bar with its label
/// centred, raised by 0.003 of the 768 px menu height.
pub(crate) fn text_button(
    p: &mut Panel,
    text: String,
    at: [f32; 2],
    action: MenuAction,
    enabled: bool,
) {
    p.widgets.push(Widget::BarButton {
        text,
        font: Font::Big,
        at,
        bar: crate::ui::widgets::MENU_BAR,
        raise: 0.003 * 768.0,
        action: Action::Menu(action),
        enabled,
    });
}

/// UI-MENU-1.4: a smaller text button inside a dialog: the 152 x 22 bar,
/// teal when highlighted, its label in `light_system`.
pub(crate) fn dialog_button(
    p: &mut Panel,
    text: String,
    at: [f32; 2],
    action: MenuAction,
    enabled: bool,
) {
    p.widgets.push(Widget::BarButton {
        text,
        font: Font::Body,
        at,
        bar: crate::ui::widgets::DIALOG_BAR,
        raise: 0.0,
        action: Action::Menu(action),
        enabled,
    });
}

/// UI-MENU-1: the main menu: the parchment, the logo at (677, 17), the
/// buttons from (700, 250) stepping 50. UI-MENU-1.2 (Run D): all five are
/// enabled; MULTIPLAYER has the normal bar but does nothing here (no
/// network game). Single player opens the single-player page (OBS-20).
/// A project's `entries` (`docs/extending.md`) take the two empty rows
/// between CREDITS and EXIT; the original buttons keep their places. Its
/// `footer` line is in `dark_system`, right-aligned at [`MENU_FOOTER`].
pub fn main_menu(
    texts: &Texts,
    screen: [f32; 2],
    entries: &[String],
    footer: Option<&str>,
) -> Panel {
    let mut p = main_menu_background(screen);
    let o = desktop_origin(screen);
    let rows: [(&str, Option<MenuAction>, bool); 7] = [
        ("!SINGLE PLAYER", Some(MenuAction::SinglePlayer), true),
        ("!MULTIPLAYER", None, true),
        ("!SETTINGS", Some(MenuAction::Settings), true),
        ("!CREDITS", Some(MenuAction::Credits), true),
        ("", None, false),
        ("", None, false),
        // UI-MENU-3.3a (c): it asks first.
        ("!EXIT", Some(MenuAction::ExitAsked), true),
    ];
    let mut free = entries.iter().enumerate();
    for (i, (key, action, enabled)) in rows.into_iter().enumerate() {
        let at = [o[0] + 700.0, o[1] + 250.0 + 50.0 * i as f32];
        if key.is_empty() {
            if let Some((n, label)) = free.next() {
                text_button(&mut p, label.clone(), at, MenuAction::OpenPage(n), true);
            }
            continue;
        }
        match action {
            Some(a) => text_button(&mut p, texts.get(key).to_owned(), at, a, enabled),
            None => p.widgets.push(Widget::BarButton {
                text: texts.get(key).to_owned(),
                font: Font::Big,
                at,
                bar: crate::ui::widgets::MENU_BAR,
                raise: 0.003 * 768.0,
                action: Action::None,
                enabled,
            }),
        }
    }
    if let Some(footer) = footer {
        p.right_text(
            footer,
            Font::Dark,
            [o[0] + MENU_FOOTER[0], o[1] + MENU_FOOTER[1]],
            INK,
        );
    }
    p
}

/// A project's main menu footer (`docs/extending.md`): its right end at the
/// button column's right edge (700 + 208), its top on the scroll between
/// EXIT (550) and the scroll's roll (about 695).
pub const MENU_FOOTER: [f32; 2] = [908.0, 672.0];

/// UI-MENU-1.9 (Q-185 c, E-ui-031): the frame of Select Map, Save Game and
/// Load Game: their own picture, `uiBigDialog` (8, 8, 376 x 464), drawn 1:1
/// and centred on the screen, the dialog taking the picture's size.
pub const MENU_DIALOG: [f32; 2] = [376.0, 464.0];

/// UI-MENU-1.9: a menu dialog over the parchment, the standard dialog of
/// UI-GAME-1.8 on the frame of [`MENU_DIALOG`], with its title centred at
/// (w / 2, 14) in `font_big_white` (its sheet has only capitals); its close
/// button is [`menu_close`].
pub(crate) fn menu_dialog(screen: [f32; 2], title: &str) -> (Panel, [f32; 2]) {
    menu_dialog_on(screen, title, Atlas::BigDialog, [8, 8, 376, 464])
}

/// UI-MENU-1.9, 4.0: a menu dialog on the frame picture `rect` of `atlas`
/// (376 x 464: `uiBigDialog` or the settings' `uiDepot`), drawn 1:1 and
/// centred, with the standard title.
pub(crate) fn menu_dialog_on(
    screen: [f32; 2],
    title: &str,
    atlas: Atlas,
    rect: crate::ui::widgets::Rect,
) -> (Panel, [f32; 2]) {
    let mut p = desktop(screen);
    let size = MENU_DIALOG;
    let o = [
        ((screen[0] - size[0]) / 2.0).floor(),
        ((screen[1] - size[1]) / 2.0).floor(),
    ];
    p.sprite(atlas, rect, o);
    p.centred_text(
        title.to_uppercase(),
        Font::Big,
        [o[0] + size[0] / 2.0, o[1] + 14.0],
        crate::ui::widgets::TITLE,
    );
    (p, o)
}

/// UI-GAME-1.8, UI-MENU-1.9: a menu dialog's close button `uiElements3`
/// (415, 32, 24 x 24) at (w - 40, 5) from its corner `o`, `!Close this
/// dialog.`: it goes back.
pub(crate) fn menu_close(p: &mut Panel, texts: &Texts, o: [f32; 2]) {
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [o[0] + MENU_DIALOG[0] - 40.0, o[1] + 5.0],
        Some(texts.get("!Close this dialog.").to_owned()),
        Action::Menu(MenuAction::Back),
        true,
    );
}

/// A project's page's panel size (the XXL panel's art; ours: the
/// original has no such page, `docs/extending.md`). The game's own
/// settings pages are on `uiDepot` (8, 8, 376 x 464) (UI-MENU-4.0).
pub const SETTINGS_DIALOG: [f32; 2] = [506.0, 540.0];

/// A project's page over the parchment: the XXL dialog art centred, with
/// its title (ours, `docs/extending.md`).
pub(crate) fn settings_dialog(screen: [f32; 2], title: &str) -> (Panel, [f32; 2]) {
    let mut p = desktop(screen);
    let size = SETTINGS_DIALOG;
    let o = [
        ((screen[0] - size[0]) / 2.0).floor(),
        ((screen[1] - size[1]) / 2.0).floor(),
    ];
    p.sized(Atlas::XxlDialog, [8, 4, 506, 540], o, size);
    p.text(title, Font::Heading, [o[0] + 200.0, o[1] + 9.0], WHITE);
    (p, o)
}

/// UI-SETUP-1.8 (Run D, from the code; OBS-20): the free play setup on
/// the left sheet, panel pixels.
pub(crate) mod layout {
    /// The map button; its caption above it (the offset is Q-301 b: the
    /// map button is not an option row, UI-SETUP-1.8's caption rule names
    /// those).
    pub const MAP_BAR: [f32; 2] = [119.0, 124.0];
    pub const CAPTION_ABOVE: f32 = 16.0;
    /// The preview's frame and the picture inside it.
    pub const PREVIEW_FRAME: [f32; 2] = [119.0, 156.0];
    pub const PREVIEW: [f32; 2] = [122.0, 159.0];
    /// "Map Size" and "Players" over their values.
    pub const INFO: [[f32; 2]; 4] = [
        [232.0, 172.0],
        [232.0, 190.0],
        [232.0, 212.0],
        [232.0, 230.0],
    ];
    /// The column rule between map and settings, the rule over the players
    /// (read from the photo, OBS-20; not in 1.8, Q-211 a).
    pub const COLUMN_RULE: [f32; 4] = [342.0, 146.0, 1.0, 150.0];
    pub const PLAYERS_RULE: [f32; 4] = [120.0, 307.0, 533.0, 1.0];
    /// The options' drop-downs, each with its caption above.
    pub const OPTIONS: [f32; 2] = [433.0, 124.0];
    pub const OPTIONS_STEP: f32 = 45.0;
    /// UI-SETUP-1.8 (Q-331 answered 2026-11-09 c, T15): an option row's
    /// caption is the drop-down's own text line, left-aligned on the bar's
    /// left edge at (0, -10 px) from its top-left, the line centred
    /// vertically there as every label's (UI-MENU-1.8 b): with
    /// `dark_system`'s line height 16 its line box runs from 18 to 2 px
    /// above the bar's top.
    pub const OPTION_CAPTION_ABOVE: f32 = 10.0;
    /// Slot i at P = SLOT + SLOT_STEP x (i mod 2, i / 2).
    pub const SLOT: [f32; 2] = [180.0, 405.0];
    pub const SLOT_STEP: [f32; 2] = [275.0, 80.0];
    /// From P: the colour square, the portrait (the emblem is at P), the
    /// three drop-downs (HQ, kind, team) stepping 22 from P.y - 22.
    pub const COLOUR: [f32; 2] = [-65.0, 20.0];
    pub const PORTRAIT: [f32; 2] = [-47.0, -25.0];
    pub const BARS: [f32; 2] = [45.0, -22.0];
    pub const BAR_STEP: f32 = 22.0;
    /// START: its position is Q-301 b; read from the photo.
    pub const START: [f32; 2] = [446.0, 649.0];
}

/// UI-SETUP-1.8a (d): the start markers' colours by colour index, and the
/// brown of a start position no slot has chosen.
pub(crate) const MARKER_COLOURS: [[u8; 3]; 8] = [
    [90, 90, 255],
    [255, 90, 90],
    [90, 255, 90],
    [255, 255, 90],
    [255, 255, 255],
    [90, 90, 90],
    [255, 90, 255],
    [192, 192, 192],
];
pub(crate) const MARKER_UNCHOSEN: [u8; 3] = [128, 64, 0];
/// UI-SETUP-1.8a (c): the black copies' offsets.
pub(crate) const MARKER_OUTLINE: [[f32; 2]; 4] = [[-2.0, 0.0], [2.0, 0.0], [0.0, 2.0], [0.0, -2.0]];

/// UI-SETUP-1.8a: one marker per start position of the map: the number
/// k + 1, at p = (122 + x 98 / W, 159 + (H - y) 98 / H) in panel pixels,
/// coloured by the first of the six slots whose start position is k (its
/// colour index, UI-SETUP-1.8), brown when no slot has it.
pub fn start_markers(summary: &MapSummary, slots: &[Slot]) -> Vec<(String, [f32; 2], [u8; 3])> {
    let (w, h) = (summary.size.0.max(1) as f32, summary.size.1.max(1) as f32);
    summary
        .starts
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| {
            let at = [
                layout::PREVIEW[0] + x as f32 * 98.0 / w,
                layout::PREVIEW[1] + (h - y as f32) * 98.0 / h,
            ];
            let colour = slots
                .iter()
                .take(6)
                .enumerate()
                .find(|(i, slot)| slot.start_of(*i) == k)
                .map_or(MARKER_UNCHOSEN, |(i, slot)| {
                    MARKER_COLOURS[slot.colour_of(i)]
                });
            ((k + 1).to_string(), at, colour)
        })
        .collect()
}

/// The labels on the parchment: `dark_system` in its own colour.
pub(crate) const INK: [u8; 3] = [255, 255, 255];

/// UI-SETUP-1.8 (Q-211 b, T16): a slot's level: 2 for a human, the
/// computer's strength, 0 for an open or closed slot (ours has closed
/// ones).
pub(crate) fn slot_level(slot: &Slot) -> usize {
    match slot.kind {
        SlotKind::Human => 2,
        SlotKind::Computer => usize::from(slot.strength.min(2)),
        _ => 0,
    }
}

/// UI-SETUP-1.8 (Q-211 b, T16): the portrait of tribe T at level L, 46 x 80
/// from `uiElements0` (16 + 168 T + 56 L, 512); hovered and pressed 88 rows
/// lower, (16 + 168 T + 56 L, 600).
pub(crate) fn portrait(tribe: usize, level: usize) -> [u32; 4] {
    [
        16 + 168 * tribe.min(2) as u32 + 56 * level.min(2) as u32,
        512,
        46,
        80,
    ]
}

/// UI-SETUP-1.8 (Q-211 b, T16): the emblem (the severity button) of level
/// L, 48 x 48 from `uiElements0` (520 + 48 L, 8); hovered and pressed 48
/// rows lower.
pub(crate) fn emblem(level: usize) -> [u32; 4] {
    [520 + 48 * level.min(2) as u32, 8, 48, 48]
}

/// UI-SETUP-1.8 (Q-211 b, T16): the colour square of colour c, 24 x 24 from
/// `uiElements0` (520 + 24 (c mod 2), 216 + 24 floor(c / 2)), the same in
/// every state.
pub(crate) fn colour_square(colour: usize) -> [u32; 4] {
    [
        520 + 24 * (colour % 2) as u32,
        216 + 24 * ((colour / 2) % 4) as u32,
        24,
        24,
    ]
}

/// The entries of a drop-down: their texts.
pub fn drop_entries(menu: &MenuState, texts: &Texts, drop: Drop) -> Vec<String> {
    let starts = menu.summary.as_ref().map_or(1, |s| s.players).clamp(1, 6);
    let keys = |k: &[&str]| k.iter().map(|k| texts.get(k).to_owned()).collect();
    match drop {
        Drop::Map => menu
            .map_names
            .iter()
            .map(|n| texts.name(n).to_owned())
            .collect(),
        // ECO-HQ-5.2: the names of the start resources settings are open
        // (Q-211); these are the language file's.
        Drop::Resources => keys(&["!Low", "!Medium", "!High"]),
        // UI-SETUP-1.8: in the code's order.
        Drop::Victory => keys(&[
            "!Endless",
            "!Destroy all enemies",
            "!Conquer 3/4 of the Map",
            "!produce 100 coins",
        ]),
        Drop::Fog => keys(&["!On", "!Off"]),
        // UI-MENU-4.1: the autosave intervals; Infowindow and DayNightCycle
        // with On first.
        Drop::Autosave => AUTOSAVES
            .iter()
            .map(|(_, k)| texts.get(k).to_owned())
            .collect(),
        Drop::InfoWindow | Drop::DayNight => keys(&["!On", "!Off"]),
        // SET-5.3, SET-4: the presets; the shader detail Low and High.
        Drop::Preset => PRESET_NAMES
            .iter()
            .map(|n| texts.name(n).to_owned())
            .collect(),
        Drop::ShaderDetail => keys(&["!Low", "!High"]),
        // SET-5.2: the window's sizes as `<w> x <h>`; MODE value 1
        // Fullscreen, 0 Windowed, listed in that order.
        Drop::Resolution => {
            crate::menus::display::resolutions(&menu.display, menu.settings.fullscreen)
                .into_iter()
                .map(crate::menus::display::mode_text)
                .collect()
        }
        Drop::DisplayMode => keys(&["!Fullscreen", "!Windowed"]),
        // SET-5.5: Inverted (value 0) / Not Inverted (1); Activated (shown
        // Enabled, 0) / Not Activated (1).
        Drop::MouseScrolling | Drop::FreeCamera => keys(&["!Inverted", "!Not Inverted"]),
        Drop::BorderScrolling | Drop::SmartCursor => keys(&["!Activated", "!Not Activated"]),
        // SET-5.7
        Drop::InternetSpeed => keys(&["!NONE", "!MODEM", "!ISDN", "!DSL/CABLE", "!LAN"]),
        Drop::Hq(_) => (1..=starts)
            .map(|n| texts.get(&format!("!HQ{n}")).to_owned())
            .collect(),
        // The human's row shows the profile's player name (SET-2 field 16).
        Drop::Kind(0) => vec![player_name(menu)],
        Drop::Kind(_) => keys(&["!AI", "!CLOSED"]),
        Drop::Team(_) => (1..=starts)
            .map(|n| texts.get(&format!("!Team {n}")).to_owned())
            .collect(),
    }
}

pub(crate) fn player_name(menu: &MenuState) -> String {
    let name = &menu.settings.cfg.user.player_name;
    if name.is_empty() {
        "default".into()
    } else {
        name.clone()
    }
}

/// The entry a drop-down shows.
pub(crate) fn drop_value(menu: &MenuState, drop: Drop) -> usize {
    let slot = |i: usize| menu.slots.get(i).copied();
    match drop {
        Drop::Map => menu.map_choice,
        Drop::Resources => usize::from(menu.resources.min(2)),
        Drop::Victory => usize::from(menu.victory),
        Drop::Fog => usize::from(!menu.fog),
        Drop::Autosave => AUTOSAVES
            .iter()
            .position(|(secs, _)| *secs == menu.settings.autosave)
            .unwrap_or(0),
        Drop::InfoWindow => usize::from(!menu.settings.info_window),
        Drop::DayNight => usize::from(!menu.settings.day_night),
        Drop::Preset => (menu.settings.cfg.scene.preset() as usize).min(PRESET_NAMES.len() - 1),
        Drop::ShaderDetail => usize::from(menu.settings.cfg.scene.shader_detail() == 3),
        // SET-5.2: the window's size (always in the list).
        Drop::Resolution => {
            crate::menus::display::resolutions(&menu.display, menu.settings.fullscreen)
                .iter()
                .position(|m| *m == menu.display.window)
                .unwrap_or(0)
        }
        Drop::DisplayMode => usize::from(!menu.settings.fullscreen),
        Drop::MouseScrolling => usize::from(!menu.settings.cfg.user.input_b),
        Drop::BorderScrolling => usize::from(!menu.settings.cfg.user.input_c),
        Drop::SmartCursor => usize::from(!menu.settings.cfg.user.input_d),
        Drop::FreeCamera => usize::from(!menu.settings.cfg.user.unknown_bools[1]),
        // SET-5.7: the network system's value; a single-player game's NONE.
        Drop::InternetSpeed => 0,
        Drop::Hq(i) => slot(i).map_or(i, |s| s.start_of(i)),
        Drop::Kind(i) => slot(i).map_or(0, |s| usize::from(s.kind == SlotKind::Closed)),
        Drop::Team(i) => slot(i).map_or(i, |s| s.team_of(i)),
    }
}

/// A drop-down: its bar with the chosen entry, centred.
pub(crate) fn drop_down(
    p: &mut Panel,
    menu: &MenuState,
    texts: &Texts,
    drop: Drop,
    at: [f32; 2],
    bar: Bar,
) {
    let entries = drop_entries(menu, texts, drop);
    let text = entries
        .get(drop_value(menu, drop))
        .cloned()
        .unwrap_or_default();
    p.widgets.push(Widget::BarButton {
        text,
        font: Font::Body,
        at,
        bar,
        raise: 0.0,
        action: Action::Menu(MenuAction::OpenDrop(drop)),
        enabled: true,
    });
}

/// The open drop-down's list under its bar: one 152 x 22 row per entry
/// (the list's art is Q-211).
pub(crate) fn drop_list(
    p: &mut Panel,
    menu: &MenuState,
    texts: &Texts,
    drop: Drop,
    below: [f32; 2],
) {
    let bar = crate::ui::widgets::DIALOG_BAR;
    for (n, text) in drop_entries(menu, texts, drop).into_iter().enumerate() {
        p.widgets.push(Widget::BarButton {
            text,
            font: Font::Body,
            at: [below[0], below[1] + bar.size[1] * n as f32],
            bar,
            raise: 0.0,
            action: Action::Menu(MenuAction::Choose(drop, n)),
            enabled: true,
        });
    }
}

/// UI-MENU-1.5 (Run D): the single-player scroll: the logo at (677, 17)
/// and in UI-MENU-1.2's column CONTINUE (enabled while the autosave
/// exists), LOAD GAME, a gap, CAMPAIGN, TUTORIAL, FREEPLAY GAME, a gap,
/// BACK; the button of the open page (`open`) is drawn pressed.
pub fn single_player_scroll(
    p: &mut Panel,
    texts: &Texts,
    o: [f32; 2],
    open: MenuAction,
    can_continue: bool,
) {
    p.sized(
        Atlas::Logo,
        [0, 0, 256, 256],
        [o[0] + 677.0, o[1] + 17.0],
        [256.0, 256.0],
    );
    let rows: [(usize, &str, MenuAction, bool); 6] = [
        (0, "!CONTINUE", MenuAction::Continue, can_continue),
        (1, "!LOAD GAME", MenuAction::LoadGame, true),
        (3, "!START CAMPAIGN", MenuAction::Campaign, true),
        (4, "!TUTORIAL", MenuAction::Tutorial, true),
        (5, "!START FREE GAME", MenuAction::FreeGame, true),
        (7, "!BACK", MenuAction::Back, true),
    ];
    for (row, key, action, enabled) in rows {
        let pressed = action == open;
        text_button(
            p,
            texts.get(key).to_owned(),
            [o[0] + 700.0, o[1] + 250.0 + 50.0 * row as f32],
            action,
            enabled && !pressed,
        );
    }
}

/// OBS-20, UI-MENU-1.5, UI-SETUP-1.8: the single-player page: the scroll
/// with FREEPLAY GAME pressed; on the left sheet the free play setup: the
/// map button with its preview, size and players, the start resources,
/// victory condition and fog of war, a row per player (emblem, colour,
/// portrait, HQ, kind, team) and START.
pub fn free_play(menu: &MenuState, texts: &Texts, screen: [f32; 2]) -> Panel {
    let mut p = Panel::new(PanelId::Menu, [0.0, 0.0]);
    background(&mut p, screen);
    let o = desktop_origin(screen);
    p.sized(Atlas::Desktop0, [0, 0, 1024, 768], o, [1024.0, 768.0]);
    single_player_scroll(
        &mut p,
        texts,
        o,
        MenuAction::FreeGame,
        continue_save(menu).is_some(),
    );
    let at = |xy: [f32; 2]| [o[0] + xy[0], o[1] + xy[1]];
    let above = |xy: [f32; 2]| at([xy[0], xy[1] - layout::CAPTION_ABOVE]);
    // The map: its button, preview, size and players.
    p.text(texts.get("!Map:"), Font::Dark, above(layout::MAP_BAR), INK);
    // UI-MENU-2.1: the map button opens Select Map (UI-SETUP-1.8 names the
    // button; that it opens the dialog is not traced, Q-256 a); it shows
    // the chosen map.
    p.widgets.push(Widget::BarButton {
        text: drop_entries(menu, texts, Drop::Map)
            .get(menu.map_choice)
            .cloned()
            .unwrap_or_default(),
        font: Font::Body,
        at: at(layout::MAP_BAR),
        bar: crate::ui::widgets::DIALOG_BAR,
        raise: 0.0,
        action: Action::Menu(MenuAction::OpenSelectMap),
        enabled: true,
    });
    p.sprite(
        Atlas::Elements0,
        [520, 104, 104, 104],
        at(layout::PREVIEW_FRAME),
    );
    p.widgets.push(Widget::Picture {
        picture: crate::ui::widgets::Picture::MapPreview,
        at: at(layout::PREVIEW),
        size: 98.0,
    });
    // UI-SETUP-1.8a: the start markers, each over four black copies 2 px
    // around it; a label's line box has its top-left at p (UI-MENU-1.8).
    // The original rebuilds them when the page refreshes (a map choice, a
    // slot change medium); here they follow the slots every frame.
    if let Some(s) = &menu.summary {
        for (text, xy, colour) in start_markers(s, &menu.slots) {
            for d in MARKER_OUTLINE {
                p.text(
                    &text,
                    Font::Heading,
                    at([xy[0] + d[0], xy[1] + d[1]]),
                    [0, 0, 0],
                );
            }
            p.text(&text, Font::Heading, at(xy), colour);
        }
    }
    if let Some(s) = &menu.summary {
        let info = [
            (texts.get("!Map Size").to_owned(), Font::Dark),
            (format!("{} x {}", s.size.0, s.size.1), Font::Outlined),
            (texts.get("!Players").to_owned(), Font::Dark),
            (s.players.to_string(), Font::Outlined),
        ];
        for ((text, font), xy) in info.into_iter().zip(layout::INFO) {
            p.text(&text, font, at(xy), INK);
        }
    }
    for r in [layout::COLUMN_RULE, layout::PLAYERS_RULE] {
        p.widgets.push(Widget::Fill {
            at: at([r[0], r[1]]),
            size: [r[2], r[3]],
            rgba: [0.45, 0.32, 0.18, 0.35],
        });
    }
    // The game's options.
    let options = [
        ("!Start resources:", Drop::Resources),
        ("!Victory condition:", Drop::Victory),
        ("!FOW", Drop::Fog),
    ];
    let option_at = |n: usize| {
        [
            layout::OPTIONS[0],
            layout::OPTIONS[1] + layout::OPTIONS_STEP * n as f32,
        ]
    };
    for (n, (key, drop)) in options.into_iter().enumerate() {
        let [x, y] = option_at(n);
        p.text(
            texts.get(key),
            Font::Dark,
            at([x, y - layout::OPTION_CAPTION_ABOVE]),
            INK,
        );
        drop_down(
            &mut p,
            menu,
            texts,
            drop,
            at(option_at(n)),
            crate::ui::widgets::DROP_BAR,
        );
    }
    // A row per start position of the map.
    let starts = menu.summary.as_ref().map_or(0, |s| s.players).min(6);
    let mut lists = Vec::new();
    for i in 0..starts {
        let slot = menu.slots.get(i).copied().unwrap_or(Slot {
            kind: SlotKind::Closed,
            strength: 0,
            tribe: -1,
            start: -1,
            team: -1,
            colour: -1,
        });
        let cell = [
            layout::SLOT[0] + layout::SLOT_STEP[0] * (i % 2) as f32,
            layout::SLOT[1] + layout::SLOT_STEP[1] * (i / 2) as f32,
        ];
        let off = |d: [f32; 2]| at([cell[0] + d[0], cell[1] + d[1]]);
        // UI-SETUP-1.8: the slot's colour square; a click steps its colour.
        let square = colour_square(slot.colour_of(i));
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: square,
            hover: Hover::None,
            pressed: Hover::None,
            at: off(layout::COLOUR),
            size: None,
            tooltip: Some(texts.get("!Select colour").to_owned()),
            action: Action::Menu(MenuAction::SlotColour(i)),
            enabled: true,
            selected: false,
        });
        if slot.kind != SlotKind::Closed {
            // UI-SETUP-1.8 (Q-211 b): the portrait and emblem of the slot's
            // level (the human the third, a computer that of its strength).
            let level = slot_level(&slot);
            let face = portrait(usize::try_from(slot.tribe).unwrap_or(0), level);
            p.widgets.push(Widget::Button {
                atlas: Atlas::Elements0,
                rect: face,
                hover: Hover::Swap([face[0], face[1] + 88]),
                pressed: Hover::None,
                at: off(layout::PORTRAIT),
                size: None,
                tooltip: Some(texts.get("!Select people").to_owned()),
                action: Action::Menu(MenuAction::SlotTribe(i)),
                enabled: true,
                selected: false,
            });
            // UI-SETUP-1.8: the emblem button, 48 x 48 at P, hovered 48
            // rows lower.
            let badge = emblem(level);
            p.widgets.push(Widget::Button {
                atlas: Atlas::Elements0,
                rect: badge,
                hover: Hover::Swap([badge[0], 56]),
                pressed: Hover::None,
                at: off([0.0, 0.0]),
                size: None,
                tooltip: (slot.kind == SlotKind::Computer)
                    .then(|| texts.get("!Select severity").to_owned()),
                action: if slot.kind == SlotKind::Computer {
                    Action::Menu(MenuAction::SlotStrength(i))
                } else {
                    Action::None
                },
                enabled: true,
                selected: false,
            });
        }
        for (n, drop) in [Drop::Hq(i), Drop::Kind(i), Drop::Team(i)]
            .into_iter()
            .enumerate()
        {
            let bar_at = off([
                layout::BARS[0],
                layout::BARS[1] + layout::BAR_STEP * n as f32,
            ]);
            if slot.kind == SlotKind::Closed && drop != Drop::Kind(i) {
                continue;
            }
            drop_down(
                &mut p,
                menu,
                texts,
                drop,
                bar_at,
                crate::ui::widgets::SLOT_BAR,
            );
            lists.push((drop, [bar_at[0], bar_at[1] + 20.0]));
        }
    }
    let players = setup_players(&menu.slots, starts);
    let distinct = starts_distinct(&players);
    text_button(
        &mut p,
        texts.get("!Start").to_uppercase(),
        at(layout::START),
        MenuAction::StartGame,
        !players.is_empty() && distinct,
    );
    // UI-SETUP-1.8 (E-ui-035): START is made with `!START_BUTTON_TOOLTIP`,
    // which the human slot's row replaces when it is filled: "Ready to
    // start" while the start positions are distinct, else "Double
    // allocation of HQ".
    p.tooltips.push((
        Action::Menu(MenuAction::StartGame),
        texts
            .get(if distinct {
                "!READY_CHECK_TOOLTIP"
            } else {
                "!READY_CHECK_UNUSABLE_TOOLTIP"
            })
            .to_owned(),
    ));
    // The open drop-down's list, over everything else.
    for (n, drop) in [Drop::Resources, Drop::Victory, Drop::Fog]
        .into_iter()
        .enumerate()
    {
        let bar = at(option_at(n));
        lists.push((drop, [bar[0], bar[1] + 19.0]));
    }
    if let Some((drop, below)) = lists.into_iter().find(|(d, _)| Some(*d) == menu.drop) {
        drop_list(&mut p, menu, texts, drop, below);
    }
    p
}

/// UI-MENU-2.1: "Map Size:" and "Players:" ending at x 120 from y 325, one
/// row apart, their values from 5 px to the right, in (200, 160, 111); the
/// preview at (195, 315) in the `uiElements0` (520, 104, 104 x 104) frame
/// at (-3, -3).
pub const MAP_INFO: [f32; 2] = [120.0, 325.0];
pub const MAP_INFO_COLOUR: [u8; 3] = [200, 160, 111];
pub const MAP_PREVIEW: [f32; 2] = [195.0, 315.0];

/// UI-MENU-2.1: Select Map (`!Select Map`): the list box at (50, 50) with
/// 11 visible rows of the free game maps (by their language names); under
/// it the size and players of the map under the pointer and its preview
/// (blank over no row). No OK or Back: a row takes its map, the title
/// bar's close button leaves. The list's rows are 21 px apart, its
/// vertical scroll bar right of them ("The scroll bar"); a rule
/// `uiElements3` (0, 432, 336 x 8) at (12, 300). The frame, title and close
/// button of UI-MENU-1.9 ([`menu_dialog`]). "How the rows look" (T22): each
/// row is the bar `uiElements0` (8, 8, 248 x 20) at its corner, (8, 32)
/// under the pointer, with the map's shown name left about 9 px into it,
/// centred on the bar's height, in its kind's colour; nothing else marks
/// the chosen map.
pub fn select_map(menu: &MenuState, texts: &Texts, screen: [f32; 2]) -> Panel {
    let (mut p, o) = menu_dialog(screen, texts.get("!Select Map"));
    menu_close(&mut p, texts, o);
    let at = |x: f32, y: f32| [o[0] + x, o[1] + y];
    let names = drop_entries(menu, texts, Drop::Map);
    for (row, (i, name)) in names
        .into_iter()
        .enumerate()
        .skip(menu.map_scroll)
        .take(MAP_ROWS)
        .enumerate()
    {
        let corner = at(50.0, 50.0 + MAP_ROW_STEP * row as f32);
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: MAP_ROW_BAR,
            hover: Hover::Swap(MAP_ROW_BAR_HOVER),
            pressed: Hover::None,
            at: corner,
            size: None,
            tooltip: None,
            action: Action::Menu(MenuAction::SelectMap(i)),
            enabled: true,
            selected: false,
        });
        let colour = menu
            .maps
            .get(i)
            .map_or(MAP_ROW_OTHER, |m| map_row_colour(m));
        p.widgets.push(Widget::Text {
            text: name,
            font: Font::Body,
            // UI-MENU-1.8 b: the line box (16 px) centred on the bar.
            at: [corner[0] + MAP_ROW_TEXT_X, corner[1] + MAP_ROW_TEXT_DROP],
            colour,
            align: crate::ui::widgets::Align::TopLeft,
        });
    }
    map_scroll_bar(&mut p, menu, at(50.0, 50.0));
    // UI-MENU-2.1 (Q-256 a): the rule between the list and the map's lines.
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], at(12.0, 300.0));
    let shown = super::shown_map(menu);
    let rows = [
        (
            texts.get("!Map Size:"),
            shown.map(|s| format!("{} x {}", s.size.0, s.size.1)),
        ),
        (texts.get("!Players:"), shown.map(|s| s.players.to_string())),
    ];
    for (n, (label, value)) in rows.into_iter().enumerate() {
        let y = MAP_INFO[1] + MAP_INFO_STEP * n as f32;
        p.right_text(label, Font::Body, at(MAP_INFO[0], y), MAP_INFO_COLOUR);
        if let Some(value) = value {
            p.text(value, Font::Body, at(MAP_INFO[0] + 5.0, y), MAP_INFO_COLOUR);
        }
    }
    p.sprite(
        Atlas::Elements0,
        [520, 104, 104, 104],
        at(MAP_PREVIEW[0] - 3.0, MAP_PREVIEW[1] - 3.0),
    );
    if shown.is_some() {
        p.widgets.push(Widget::Picture {
            picture: crate::ui::widgets::Picture::MapPreview,
            at: at(MAP_PREVIEW[0], MAP_PREVIEW[1]),
            size: 98.0,
        });
    }
    if menu.invalid_map {
        invalid_map_box(&mut p, texts, at(0.0, 0.0));
    }
    p
}

/// UI-MENU-2.1 "The scroll bar": the plain list box's vertical bar at
/// (252, 18) from the list's corner `list`: the track `uiElements0` (456,
/// 197) 24 x 194 with the 24 x 24 thumb (8, 104) on it; the up arrow (8,
/// 128) at (0, -19) from the bar, the down arrow (104, 128) at its foot
/// (0, 194), each with its looks 32 and 64 px to the right (highlighted,
/// pressed: which is which is medium). Q-1042 (answered, E-ui-029): the bar
/// keeps its value v, the list follows it ([`bar_first`]), an arrow steps v
/// by 1 / floor(N / 2) ([`bar_step`]), the thumb's top 170 v px down, in the
/// middle while everything fits, the arrows off at the ends and both while
/// it fits; an arrow held steps again every 0.05 s ([`super::bar_repeat`],
/// the UI's press handling), and the pressed bar (track or thumb) pulls v
/// toward the pointer ([`super::bar_chase`]).
fn map_scroll_bar(p: &mut Panel, menu: &MenuState, list: [f32; 2]) {
    list_scroll_bar(
        p,
        menu.maps.len(),
        menu.map_bar,
        list,
        MenuAction::MapScroll,
        MenuAction::MapTrack,
    );
}

/// UI-MENU-2.1 "The scroll bar" of a list box of 11 rows whose corner is
/// `list`, `count` entries, its bar's value `v`; the arrows send `scroll`,
/// the pressed track `track` (Save Game's list has the same bar, Q-1042).
pub(crate) fn list_scroll_bar(
    p: &mut Panel,
    count: usize,
    v: f32,
    list: [f32; 2],
    scroll: fn(i32) -> MenuAction,
    track: fn(i32) -> MenuAction,
) {
    const TRACK: f32 = 194.0;
    const THUMB: f32 = 24.0;
    let bar = [list[0] + 252.0, list[1] + 18.0];
    // Q-1042 (c): the whole bar takes the press (the thumb drawn over it
    // lets it through); the UI sends `track` with the pointer's offset.
    p.widgets.push(Widget::Button {
        atlas: Atlas::Elements0,
        rect: [456, 197, 24, 194],
        hover: Hover::None,
        pressed: Hover::None,
        at: bar,
        size: None,
        tooltip: None,
        action: Action::Menu(track(0)),
        enabled: true,
        selected: false,
    });
    // Q-1042 (a), (b): the thumb's top 170 v px down the bar, not rounded
    // (v 0.5 while everything fits); the up arrow off at v = 0, the down
    // arrow at v = 1, both while it fits.
    let fits = count <= MAP_ROWS;
    let v = if fits { 0.5 } else { v.clamp(0.0, 1.0) };
    let along = (TRACK - THUMB) * v;
    let enabled = [!fits && v > 0.0, !fits && v < 1.0];
    p.sprite(Atlas::Elements0, [8, 104, 24, 24], [bar[0], bar[1] + along]);
    for (k, (x, y, delta)) in [(8, -19.0, -1), (104, TRACK, 1)].into_iter().enumerate() {
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: [x, 128, 24, 24],
            hover: Hover::Swap([x + 32, 128]),
            pressed: Hover::Swap([x + 64, 128]),
            at: [bar[0], bar[1] + y],
            size: None,
            tooltip: None,
            action: Action::Menu(scroll(delta)),
            enabled: enabled[k],
            selected: false,
        });
    }
}

/// A [`Notice`]: the small panel of the `!INVALID_MAP` box below, centred
/// on the screen, its title and up to three lines, closed by its close
/// button. Ours (a save loaded in part or not at all is not specified).
pub fn notice_box(notice: &Notice, screen: [f32; 2]) -> Panel {
    let size = [288.0, 88.0];
    let mut p = Panel::new(
        PanelId::Notice,
        [
            ((screen[0] - size[0]) / 2.0).floor(),
            ((screen[1] - size[1]) / 2.0).floor(),
        ],
    );
    p.sprite(Atlas::SmallDialog, [0, 0, 288, 88], [0.0, 0.0]);
    p.centred_text(
        notice.title.clone(),
        Font::Big,
        [size[0] / 2.0, 14.0],
        crate::ui::widgets::TITLE,
    );
    for (n, line) in notice.lines.iter().take(3).enumerate() {
        p.text(
            line.clone(),
            Font::Body,
            [15.0, 38.0 + 15.0 * n as f32],
            WHITE,
        );
    }
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [size[0] - 40.0, 5.0],
        None,
        Action::Menu(MenuAction::CloseNotice),
        true,
    );
    p
}

/// UI-MENU-2.1: the message box (dialog 38) of a map whose header does not
/// read: `!INVALID_MAP` over `!INVALID_FILE`. Its layout is not given: the
/// small panel of UI-GAME-1.8 centred on the dialog, closed by its close
/// button (Q-256 a).
fn invalid_map_box(p: &mut Panel, texts: &Texts, o: [f32; 2]) {
    let size = [288.0, 88.0];
    let b = [
        o[0] + ((MENU_DIALOG[0] - size[0]) / 2.0).floor(),
        o[1] + ((MENU_DIALOG[1] - size[1]) / 2.0).floor(),
    ];
    p.sprite(Atlas::SmallDialog, [0, 0, 288, 88], b);
    p.centred_text(
        texts.get("!INVALID_MAP").to_uppercase(),
        Font::Big,
        [b[0] + size[0] / 2.0, b[1] + 14.0],
        crate::ui::widgets::TITLE,
    );
    p.text(
        texts.get("!INVALID_FILE"),
        Font::Body,
        [b[0] + 15.0, b[1] + 40.0],
        WHITE,
    );
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [b[0] + size[0] - 40.0, b[1] + 5.0],
        Some(texts.get("!Close this dialog.").to_owned()),
        Action::Menu(MenuAction::Back),
        true,
    );
}
