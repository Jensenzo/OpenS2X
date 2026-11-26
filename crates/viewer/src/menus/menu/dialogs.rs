//! The menu dialogs: game menu, settings, save and load, credits
//! (UI-MENU-3 to 6).

use super::*;

/// UI-MENU-3.1, 3.1a: the game menu, centred: the logo, then the entries
/// stepping 32 with separators after Load Game and Settings; no CONTINUE
/// row (Escape closes it). The spec's list origin x 70 centres a 222 px
/// button in the panel; the bars here are 208 wide, so the column is
/// centred on the visible art.
/// UI-MENU-3.1b, 1.9: its own frame, `uiBigDialog` (8, 8, 376 x 464) drawn
/// 1:1 and centred; the art covers 364 x 448 of it from the top-left. The
/// standard dialog's title and close button (UI-GAME-1.8) by the frame's
/// width: the title centred at (188, 14) in `font_big_white`, the close
/// button at (336, 5), as Select Map, Save and Load.
pub fn game_menu(texts: &Texts, screen: [f32; 2]) -> Panel {
    let frame = [376.0, 464.0];
    let size = [364.0, 448.0];
    let mut p = Panel::new(
        PanelId::Menu,
        [
            ((screen[0] - frame[0]) / 2.0).floor(),
            ((screen[1] - frame[1]) / 2.0).floor(),
        ],
    );
    p.sprite(Atlas::BigDialog, [8, 8, 376, 464], [0.0, 0.0]);
    // UI-MENU-3.1a: closed by the dialog's close button (as UI-GAME-1.8's
    // dialogs) or Escape.
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [frame[0] - 40.0, 5.0],
        Some(texts.get("!Close this dialog.").to_owned()),
        Action::Menu(MenuAction::CloseGameMenu),
        true,
    );
    p.centred_text(
        texts.get("!Game Menu").to_uppercase(),
        Font::Big,
        [frame[0] / 2.0, 14.0],
        crate::ui::widgets::TITLE,
    );
    let logo = 176.0;
    p.sized(
        Atlas::Logo,
        [0, 0, 256, 256],
        [(size[0] - logo) / 2.0, 30.0],
        [logo, logo],
    );
    let x = ((size[0] - crate::ui::widgets::MENU_BAR.size[0]) / 2.0).floor();
    let entries: [(&str, MenuAction); 5] = [
        ("!Save Game", MenuAction::SaveGame),
        ("!Load Game", MenuAction::LoadGame),
        ("!Settings", MenuAction::Settings),
        ("!Exit to Title", MenuAction::AskLeave(false)),
        ("!Exit to Desktop", MenuAction::AskLeave(true)),
    ];
    let mut y = 208.0;
    for (i, (key, action)) in entries.into_iter().enumerate() {
        if i == 2 || i == 3 {
            p.sprite(Atlas::Elements3, [0, 432, 336, 8], [13.0, y + 4.0]);
            y += 16.0;
        }
        text_button(&mut p, texts.get(key).to_owned(), [x, y], action, true);
        y += 32.0;
    }
    p
}

/// SET-5.1a: the engine page's title.
const ENGINE_TITLE: &str = "!Engine Settings";

/// UI-MENU-4.0, 4.1 (Q-1047, E-ui-034), SET-5.2 (E-ui-035): a settings
/// page's caption, 144 px left of its control (x 24), centred on the
/// control's middle (y + 10); the original's pictures (E-trace-026) show it
/// in the drop-downs' font, `light_system`.
fn settings_label(p: &mut Panel, texts: &Texts, o: [f32; 2], key: &str, y: f32) {
    // A `light_system` line is 16 px: its middle 8 px under its top.
    p.text(
        texts.get(key),
        Font::Body,
        [o[0] + 24.0, o[1] + y + 10.0 - 8.0],
        WHITE,
    );
}

/// SET-5.5 (E-ui-035), UI-MENU-4.2: a slider's caption is the slider's own,
/// centred 22 px below the slider's top at (168, y) (the Sound and Input
/// pages of the original's pictures, E-trace-026, E-trace-028).
fn slider_label(p: &mut Panel, texts: &Texts, o: [f32; 2], key: &str, y: f32) {
    settings_label(p, texts, o, key, y + 12.0);
}

/// UI-MENU-4.1, `UI-ingame.md` UI-GAME-8.5g: a settings drop-down, one 152 x
/// 20 text button at (168, y) showing the chosen value.
fn settings_drop(p: &mut Panel, menu: &MenuState, texts: &Texts, o: [f32; 2], drop: Drop, y: f32) {
    drop_down(
        p,
        menu,
        texts,
        drop,
        [o[0] + 168.0, o[1] + y],
        crate::ui::widgets::SLOT_BAR,
    );
}

/// UI-GAME-8.5g (a): the open drop-down's choices, one text button each,
/// 19 px apart from one step below its button at `at`, or upward from it
/// when the list would run past the screen's bottom.
fn settings_drop_list(
    p: &mut Panel,
    menu: &MenuState,
    texts: &Texts,
    drop: Drop,
    at: [f32; 2],
    screen: [f32; 2],
) {
    const STEP: f32 = 19.0;
    let entries = drop_entries(menu, texts, drop);
    let downward = at[1] + STEP * (entries.len() as f32 + 1.0) <= screen[1];
    for (n, text) in entries.into_iter().enumerate() {
        let step = STEP * (n as f32 + 1.0);
        p.widgets.push(Widget::BarButton {
            text,
            font: Font::Body,
            at: [at[0], if downward { at[1] + step } else { at[1] - step }],
            bar: crate::ui::widgets::SLOT_BAR,
            raise: 0.0,
            action: Action::Menu(MenuAction::Choose(drop, n)),
            enabled: true,
        });
    }
}

/// UI-MENU-4.0, 4.2: a settings slider at (168, y), 152 px wide (its bar art
/// is 152 x 24, medium).
fn settings_slider(p: &mut Panel, o: [f32; 2], y: f32, value: f32, kind: SliderKind) {
    // UI-MENU-4.0, UI-GAME-6.5 (b): the settings slider, 152 x 24, its
    // top-left at the row's (168, y).
    p.widgets.push(Widget::Slider {
        at: [o[0] + 168.0, o[1] + y],
        width: crate::ui::widgets::SETTINGS_SLIDER,
        value,
        kind,
        enabled: true,
    });
}

/// SET-5.3, SET-4a: the engine page: `GRAPHIC QUALITY` a drop-down of the six
/// presets, the sliders `TERRAIN QUALITY`, `SHADOW QUALITY`, `SHADOW DETAIL`,
/// `MIRROR QUALITY`, `MIRROR DETAILS`, `OBJECT DETAILS`, `OBJ DRAW DISTANCE`,
/// `SHADERDETAIL` a drop-down (Low, High), `TEXTURE QUALITY` a slider; rows
/// 32 px apart from (168, 72), the captions at x 24 (UI-MENU-4.0).
fn engine_page(p: &mut Panel, menu: &MenuState, texts: &Texts, o: [f32; 2]) -> Vec<(Drop, f32)> {
    use s2_assets::cfg::field;
    let scene = &menu.settings.cfg.scene;
    let mut drops = Vec::new();
    let mut y = 72.0;
    settings_label(p, texts, o, "!GRAPHIC QUALITY", y);
    settings_drop(p, menu, texts, o, Drop::Preset, y);
    drops.push((Drop::Preset, y));
    let sliders = [
        ("!TERRAIN QUALITY", field::TERRAIN_QUALITY),
        ("!SHADOW QUALITY", field::SHADOW_QUALITY),
        ("!SHADOW DETAIL", field::SHADOW_DETAIL),
        ("!MIRROR QUALITY", field::MIRROR_QUALITY),
        ("!MIRROR DETAILS", field::MIRROR_DETAILS),
        ("!OBJECT DETAILS", field::OBJECT_DETAILS),
        ("!OBJ DRAW DISTANCE", field::DRAW_DISTANCE),
    ];
    for (key, f) in sliders {
        y += 32.0;
        slider_label(p, texts, o, key, y);
        settings_slider(p, o, y, scene.slider(f), SliderKind::Scene(f as u8));
    }
    y += 32.0;
    settings_label(p, texts, o, "!SHADERDETAIL", y);
    settings_drop(p, menu, texts, o, Drop::ShaderDetail, y);
    drops.push((Drop::ShaderDetail, y));
    y += 32.0;
    slider_label(p, texts, o, "!TEXTURE QUALITY", y);
    settings_slider(
        p,
        o,
        y,
        scene.slider(field::TEXTURE_QUALITY),
        SliderKind::Scene(field::TEXTURE_QUALITY as u8),
    );
    drops
}

/// SET-5.2: the Graphics page: `RESOLUTION` (152 x 20) at (168, 72)
/// showing the mode as `<w> x <h>`, `MODE` (shown DISPLAY MODE) at (168,
/// 104), Fullscreen / Windowed; no Reset. Ours (a stand-in): the mode is the
/// window's size and its list holds that size only, the window is Windowed,
/// and a choice changes nothing (the game window is not a display device
/// mode). Returns the drop-downs with their rows; RESOLUTION's open list is
/// drawn here.
fn graphics_page(
    p: &mut Panel,
    menu: &MenuState,
    texts: &Texts,
    o: [f32; 2],
    screen: [f32; 2],
) -> Vec<(Drop, f32)> {
    let mode = format!("{} x {}", screen[0].round(), screen[1].round());
    settings_label(p, texts, o, "!RESOLUTION", 72.0);
    let bar = |p: &mut Panel, y: f32, action: MenuAction| {
        p.widgets.push(Widget::BarButton {
            text: mode.clone(),
            font: Font::Body,
            at: [o[0] + 168.0, o[1] + y],
            bar: crate::ui::widgets::SLOT_BAR,
            raise: 0.0,
            action: Action::Menu(action),
            enabled: true,
        });
    };
    bar(p, 72.0, MenuAction::OpenDrop(Drop::Resolution));
    settings_label(p, texts, o, "!MODE", 104.0);
    settings_drop(p, menu, texts, o, Drop::DisplayMode, 104.0);
    if menu.drop == Some(Drop::Resolution) {
        bar(p, 72.0 + 19.0, MenuAction::Choose(Drop::Resolution, 0));
    }
    vec![(Drop::DisplayMode, 104.0)]
}

/// SET-5.5: the Input page: `SCROLL SPEED` a slider at (168, 72) (field 7)
/// with its caption 22 px below the slider's top, not on its middle; then
/// the drop-downs `MOUSE SCROLLING` (field 6), `BORDER SCROLLING` (8),
/// `SMART CURSOR` (9) and `FREE CAMERA` (21) at y 104, 136, 168, 200.
fn input_page(p: &mut Panel, menu: &MenuState, texts: &Texts, o: [f32; 2]) -> Vec<(Drop, f32)> {
    // The drop-downs' captions are centred on their control's middle (y +
    // 10); the slider's is centred on y + 22, 12 px lower.
    slider_label(p, texts, o, "!SCROLL SPEED", 72.0);
    settings_slider(
        p,
        o,
        72.0,
        menu.settings.cfg.user.scroll_speed,
        SliderKind::ScrollSpeed,
    );
    let mut drops = Vec::new();
    for (row, (key, drop)) in [
        ("!MOUSE SCROLLING", Drop::MouseScrolling),
        ("!BORDER SCROLLING", Drop::BorderScrolling),
        ("!SMART CURSOR", Drop::SmartCursor),
        ("!FREE CAMERA", Drop::FreeCamera),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 104.0 + 32.0 * row as f32;
        settings_label(p, texts, o, key, y);
        settings_drop(p, menu, texts, o, drop, y);
        drops.push((drop, y));
    }
    drops
}

/// SET-5.7: the Network page: `CALLSIGN` at (168, 72), a text 152 px wide
/// in `light_system` with the player's name (field 16; ours shows it, not
/// an input: nothing of this page changes a single-player game), and
/// `INTERNET SPEED` at (168, 104), NONE to LAN, showing NONE.
fn network_page(p: &mut Panel, menu: &MenuState, texts: &Texts, o: [f32; 2]) -> Vec<(Drop, f32)> {
    settings_label(p, texts, o, "!CALLSIGN", 72.0);
    p.text(
        menu.settings.cfg.user.player_name.clone(),
        Font::Body,
        [o[0] + 168.0, o[1] + 72.0],
        WHITE,
    );
    settings_label(p, texts, o, "!INTERNET SPEED", 104.0);
    settings_drop(p, menu, texts, o, Drop::InternetSpeed, 104.0);
    vec![(Drop::InternetSpeed, 104.0)]
}

/// SET-4: the graphic quality presets' names, in field 1's order.
/// (The language files hold them as plain name keys.)
pub(crate) const PRESET_NAMES: [&str; 6] =
    ["VeryLow", "Low", "Medium", "High", "VeryHigh", "Custom"];

/// UI-MENU-4.1: the AUTOSAVE choices and their intervals in seconds.
pub(crate) const AUTOSAVES: [(u32, &str); 7] = [
    (0, "!Off"),
    (60, "!1 Minute"),
    (300, "!5 Minute"),
    (600, "!10 Minutes"),
    (1200, "!20 Minutes"),
    (2700, "!45 Minutes"),
    (3600, "!1 Hour"),
];

/// UI-MENU-4.0 (Q-1047, E-ui-034): the six settings pages in the tabs'
/// order: their tab icon ids (UI-GAME-3.5), tooltips and ours (game 0,
/// sound 1, engine 2, graphics 3, input 4, network 5).
pub const SETTINGS_TABS: [(u32, &str, Option<u8>); 6] = [
    (0, "!Graphics settings", Some(3)),
    (1, "!Engine settings", Some(2)),
    (2, "!Sound settings", Some(1)),
    (3, "!Input settings", Some(4)),
    (4, "!Game settings", Some(0)),
    (23, "!Network settings", Some(5)),
];

/// UI-MENU-4.0, 4.1, 4.2, SET-5.3 (Q-1047 answered, E-ui-034): a settings
/// page: the frame `uiDepot` (8, 8, 376 x 464) centred with the standard
/// title and close button (UI-GAME-1.8; no BACK: the close button or
/// Escape closes it), the page switch (six tabs at (15, 25), 40 px apart),
/// the page's controls from (168, 72) stepping 32 px with their captions at
/// x 24, and the wide text button `!Reset` at (168, 408). Game: AUTOSAVE,
/// Infowindow and DayNightCycle drop-downs; sound: four sliders; engine:
/// SET-5.3; `!Reset` puts the page's values back (none on the engine page).
pub fn settings_screen(menu: &MenuState, texts: &Texts, screen: [f32; 2], page: u8) -> Panel {
    // SET-5.1a, 5.2, 5.5, 5.7: the pages' titles.
    let title = match page {
        0 => "!Game Settings",
        1 => "!Sound settings",
        3 => "!Gfx Settings",
        4 => "!Input Settings",
        5 => "!Network Settings",
        _ => ENGINE_TITLE,
    };
    let (mut p, o) = menu_dialog_on(screen, texts.get(title), Atlas::Depot, [8, 8, 376, 464]);
    menu_close(&mut p, texts, o);
    settings_tabs(&mut p, texts, o, page.min(5));
    let mut drops: Vec<(Drop, f32)> = Vec::new();
    if page == 3 {
        drops = graphics_page(&mut p, menu, texts, o, screen);
    } else if page == 4 {
        drops = input_page(&mut p, menu, texts, o);
    } else if page == 5 {
        drops = network_page(&mut p, menu, texts, o);
    } else if page == 0 {
        for (row, (key, drop)) in [
            ("!AUTOSAVE", Drop::Autosave),
            ("!Infowindow", Drop::InfoWindow),
            ("!DayNightCycle", Drop::DayNight),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 72.0 + 32.0 * row as f32;
            settings_label(&mut p, texts, o, key, y);
            settings_drop(&mut p, menu, texts, o, drop, y);
            drops.push((drop, y));
        }
    } else if page >= 2 {
        drops = engine_page(&mut p, menu, texts, o);
    } else {
        let labels = [
            "!MASTER VOLUME",
            "!MUSIC VOLUME",
            "!UI VOLUME",
            "!SOUND VOLUME",
        ];
        for (i, label) in labels.into_iter().enumerate() {
            let y = 72.0 + 32.0 * i as f32;
            slider_label(&mut p, texts, o, label, y);
            settings_slider(
                &mut p,
                o,
                y,
                menu.settings.volumes[i],
                SliderKind::Volume(i as u8),
            );
        }
    }
    // UI-MENU-4.0 (E-ui-034, a picture of the original): `!Reset` puts this
    // page's values back; the engine and graphics pages have none, the
    // network page's does nothing (SET-5.2, 5.7).
    // The original's pictures (E-trace-026): Reset is the dialogs' 152 x 22
    // text button.
    if matches!(page, 0 | 1 | 4 | 5) {
        dialog_button(
            &mut p,
            texts.get("!Reset").to_owned(),
            [o[0] + 168.0, o[1] + 408.0],
            MenuAction::SettingsReset(page),
            true,
        );
    }
    // The open drop-down's list, over everything else.
    if let Some((drop, y)) = drops.into_iter().find(|(d, _)| Some(*d) == menu.drop) {
        settings_drop_list(&mut p, menu, texts, drop, [o[0] + 168.0, o[1] + y], screen);
    }
    p
}

/// UI-MENU-4.0, UI-GAME-3.5: the page switch, a row of six icon tabs from
/// (15, 25), 40 px apart: `uiElements0` (8 + 40 id, 416) 40 x 27, hovered
/// (8 + 40 id, 448), the open page's with the selected overlay (8 + 40 id,
/// 480), each with its page's tooltip; a click opens that page. A page ours
/// has not got is off.
fn settings_tabs(p: &mut Panel, texts: &Texts, o: [f32; 2], open: u8) {
    for (i, (id, key, page)) in SETTINGS_TABS.into_iter().enumerate() {
        let at = [o[0] + 15.0 + 40.0 * i as f32, o[1] + 25.0];
        let x = 8 + 40 * id;
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: [x, 416, 40, 27],
            hover: Hover::Swap([x, 448]),
            pressed: Hover::None,
            at,
            size: None,
            tooltip: Some(texts.get(key).to_owned()),
            action: page.map_or(Action::None, |n| Action::Menu(MenuAction::SettingsPage(n))),
            enabled: page.is_some(),
            selected: false,
        });
        if page == Some(open) {
            p.sprite(Atlas::Elements0, [x, 480, 40, 27], at);
        }
    }
}

/// A project's page (`docs/extending.md`, `extend::menu`) has room down
/// to here: eleven rows of 32 px from 72, above where the settings pages
/// have their page switch (420).
pub const PAGE_BOTTOM: f32 = 424.0;

/// Hook W22: the y of a long page's switch, between the rows and BACK.
pub const PAGE_SWITCH_Y: f32 = 436.0;

/// The rows a row of a project's page takes (a choice one per line of
/// three options).
fn row_lines(row: &crate::extend::menu::Row) -> usize {
    match row {
        crate::extend::menu::Row::Choice { options, .. } => options.len().div_ceil(3).max(1),
        _ => 1,
    }
}

/// Hook W22: a project's page in parts, each the indices of its rows: a
/// `PageBreak` starts the next part, and so does a row that would reach
/// past [`PAGE_BOTTOM`]. A page that fits is one part (as before W22).
pub fn page_parts(rows: &[crate::extend::menu::Row]) -> Vec<Vec<usize>> {
    let mut parts: Vec<Vec<usize>> = vec![Vec::new()];
    let mut y = 72.0;
    for (i, row) in rows.iter().enumerate() {
        if matches!(row, crate::extend::menu::Row::PageBreak) {
            if !parts.last().is_some_and(Vec::is_empty) {
                parts.push(Vec::new());
            }
            y = 72.0;
            continue;
        }
        let height = 32.0 * row_lines(row) as f32;
        if y + height > PAGE_BOTTOM && !parts.last().is_some_and(Vec::is_empty) {
            parts.push(Vec::new());
            y = 72.0;
        }
        parts.last_mut().expect("a part").push(i);
        y += height;
    }
    if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) {
        parts.pop();
    }
    parts
}

/// A project's page (`docs/extending.md`): the menu dialog titled with its
/// entry's label, its rows laid out as the game settings (UI-MENU-4.1):
/// from (168, 72) stepping 32, a label 144 px left of its row in the
/// headings' font and colour, a toggle's Off and On and a choice's options
/// as text buttons (three to a line 100 px apart, lines 24 px apart, a row
/// of 32 px per line), the chosen one yellow; a button is the 152 x 22 bar
/// at the row. BACK at (50, 480) (ours: the settings pages have none,
/// UI-MENU-4.0). Hook W22: a page
/// longer than [`PAGE_BOTTOM`], or with a `PageBreak`, is shown a part at a
/// time ([`page_parts`]), with the switch `<`, `n/m`, `>` at
/// [`PAGE_SWITCH_Y`] (the body font's text buttons at x 168, 218 and 268;
/// the original's settings pages have no such switch: a project's look).
pub fn extension_page(menu: &MenuState, texts: &Texts, screen: [f32; 2], n: usize) -> Panel {
    use crate::extend::menu::{PageInput, Row};
    let title = menu.entries.get(n).map_or("", String::as_str);
    let (mut p, o) = settings_dialog(screen, title);
    let at = |x: f32, y: f32| [o[0] + x, o[1] + y];
    let choice = |p: &mut Panel, text: String, x: f32, y: f32, on: bool, action: MenuAction| {
        p.widgets.push(Widget::TextButton {
            text,
            font: Font::Body,
            at: at(x, y),
            colour: if on { [255, 220, 0] } else { WHITE },
            action: Action::Menu(action),
            enabled: true,
        });
    };
    let parts = page_parts(&menu.page);
    let part = menu.page_part.min(parts.len().saturating_sub(1));
    let mut y = 72.0;
    for &i in &parts[part] {
        let row = &menu.page[i];
        let lines = row_lines(row);
        let label = match row {
            Row::Caption(text) => Some(text),
            Row::Toggle { label, .. } | Row::Choice { label, .. } => Some(label),
            Row::Button(_) | Row::PageBreak => None,
        };
        if let Some(label) = label {
            p.text(label, Font::Heading, at(168.0 - 144.0, y), HEADING);
        }
        match row {
            Row::Caption(_) | Row::PageBreak => {}
            Row::Toggle { value, .. } => {
                for (x, on) in [(168.0, false), (268.0, true)] {
                    let key = if on { "!On" } else { "!Off" };
                    let action = MenuAction::Page(i, PageInput::Toggle(on));
                    choice(
                        &mut p,
                        texts.get(key).to_owned(),
                        x,
                        y,
                        *value == on,
                        action,
                    );
                }
            }
            Row::Choice { options, index, .. } => {
                for (k, option) in options.iter().enumerate() {
                    let (x, line) = (168.0 + 100.0 * (k % 3) as f32, y + 24.0 * (k / 3) as f32);
                    let action = MenuAction::Page(i, PageInput::Choose(k));
                    choice(&mut p, option.clone(), x, line, k == *index, action);
                }
            }
            Row::Button(text) => dialog_button(
                &mut p,
                text.clone(),
                at(168.0, y),
                MenuAction::Page(i, PageInput::Click),
                true,
            ),
        }
        y += 32.0 * lines as f32;
    }
    if parts.len() > 1 {
        let y = PAGE_SWITCH_Y;
        for (x, text, to) in [
            (168.0, "<", part.checked_sub(1)),
            (268.0, ">", Some(part + 1).filter(|&k| k < parts.len())),
        ] {
            p.widgets.push(Widget::TextButton {
                text: text.to_owned(),
                font: Font::Body,
                at: at(x, y),
                colour: WHITE,
                action: to.map_or(Action::None, |k| Action::Menu(MenuAction::PagePart(k))),
                enabled: to.is_some(),
            });
        }
        p.text(
            format!("{}/{}", part + 1, parts.len()),
            Font::Body,
            at(218.0, y),
            WHITE,
        );
    }
    dialog_button(
        &mut p,
        texts.get("!Back").to_owned(),
        at(50.0, 480.0),
        MenuAction::Back,
        true,
    );
    p
}

/// UI-MENU-3.3: the leave dialog, `!Request!`: the question centred on (240, 70), `!Yes`
/// at (130, 110) and `!No` at (130, 144), both 5 px higher while a game runs; and
/// only then the `!Autosave...` check button at (130, 175), setting 22.
/// UI-MENU-3.3a: 480 x 224, `uiMediumDialog` (8, 8, 480 x 224) centred, the title
/// and the close button of every dialog (UI-GAME-1.8, here as the game menu
/// places them); the box is the check box of UI-MENU-1.4, 28 x 28, `uiElements0`
/// (132, 100) off and (164, 100) on, its label in `light_system` 32 px right and
/// 4 px lower. `game`: a game runs (not over the main menu, 3.3a c).
pub fn leave_dialog(
    menu: &MenuState,
    texts: &Texts,
    screen: [f32; 2],
    desktop: bool,
    game: bool,
) -> Panel {
    let size = LEAVE_DIALOG;
    let mut p = Panel::new(
        PanelId::Menu,
        [
            ((screen[0] - size[0]) / 2.0).floor(),
            ((screen[1] - size[1]) / 2.0).floor(),
        ],
    );
    p.sprite(Atlas::MediumDialog, [8, 8, 480, 224], [0.0, 0.0]);
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [size[0] - 40.0, 5.0],
        Some(texts.get("!Close this dialog.").to_owned()),
        Action::Menu(MenuAction::StayInGame),
        true,
    );
    p.centred_text(
        texts.get("!Request!"),
        Font::Heading,
        [size[0] / 2.0, 9.0],
        WHITE,
    );
    let question = if desktop {
        "!Exit to desktop?"
    } else {
        "!Exit to title?"
    };
    // UI-MENU-3.3a (e, seen in the original's picture): centred on the
    // frame's middle, on y 70 (3.3 had it from x 80).
    p.centred_text(
        texts.get(question),
        Font::Body,
        [size[0] / 2.0, 70.0],
        WHITE,
    );
    let raise = if game { 5.0 } else { 0.0 };
    dialog_button(
        &mut p,
        texts.get("!Yes").to_owned(),
        [130.0, 110.0 - raise],
        if desktop {
            MenuAction::Exit
        } else {
            MenuAction::ExitToTitle
        },
        true,
    );
    dialog_button(
        &mut p,
        texts.get("!No").to_owned(),
        [130.0, 144.0 - raise],
        MenuAction::StayInGame,
        true,
    );
    if game {
        let at = [130.0, 175.0];
        let x = if menu.settings.autosave_on_leave {
            164
        } else {
            132
        };
        p.button(
            Atlas::Elements0,
            [x, 100, 28, 28],
            Hover::None,
            at,
            None,
            Action::Menu(MenuAction::AutosaveOnLeave),
            true,
        );
        p.text(
            texts.get("!Autosave..."),
            Font::Body,
            [at[0] + 32.0, at[1] + 4.0],
            WHITE,
        );
    }
    p
}

/// UI-MENU-3.3a (a): the leave dialog's size.
pub const LEAVE_DIALOG: [f32; 2] = [480.0, 224.0];

/// UI-MENU-5: the save and load dialogs: the list of saves (11 rows from
/// (50, 50), in the file system's order); `!Save` writes the game under the typed
/// name (ours, `game::save`); `!Load` continues the chosen save, ours or the
/// original's (FMT-SAV, as far as the game can take it over, Q-204);
/// `!Delete` asks `!Remove this file?` in the place of the name and the
/// `!Save` / `!Load` button. UI-MENU-1.9: the frame, title and close button
/// of [`menu_dialog`] (no BACK button: the close button goes back); the
/// rows 21 px apart with Select Map's scroll bar (Q-1042).
pub fn save_load(
    menu: &MenuState,
    texts: &Texts,
    screen: [f32; 2],
    save: bool,
    focus: Option<crate::ui::widgets::Focus>,
) -> Panel {
    let title = if save { "!Save Game" } else { "!Load Game" };
    let (mut p, o) = menu_dialog(screen, texts.get(title));
    let at = |x: f32, y: f32| [o[0] + x, o[1] + y];
    // UI-MENU-5.1: while the game is written every control is hidden and
    // `!Saving...` shows at (50, 50).
    if save && menu.saving {
        p.text(texts.get("!Saving..."), Font::Body, at(50.0, 50.0), WHITE);
        return p;
    }
    menu_close(&mut p, texts, o);
    // UI-MENU-5.1: under 5 MB free `!Disk Full!` is the caption of the
    // `!Save` button, which is disabled with `!Delete`.
    let full = save && menu.disk_full;
    for (row, (i, entry)) in menu
        .saves
        .iter()
        .enumerate()
        .skip(menu.save_scroll)
        .take(SAVE_ROWS)
        .enumerate()
    {
        p.widgets.push(Widget::TextButton {
            text: entry.label(&menu.saves),
            font: Font::Body,
            at: at(50.0, 50.0 + MAP_ROW_STEP * row as f32),
            colour: if i == menu.save_choice {
                [255, 220, 0]
            } else {
                WHITE
            },
            action: Action::Menu(MenuAction::SelectSave(i)),
            enabled: true,
        });
    }
    // UI-MENU-2.1 (Q-1042): the list's scroll bar, the same as Select
    // Map's at the same places.
    list_scroll_bar(
        &mut p,
        menu.saves.len(),
        menu.save_bar,
        at(50.0, 50.0),
        MenuAction::SaveScroll,
        MenuAction::SaveTrack,
    );
    let chosen = menu.save_choice < menu.saves.len();
    // UI-MENU-5.1, 6.1: the name (save) or the prompt (load), the
    // separators at (12, 290) and (12, 390), Save / Load at (148, 344),
    // Delete at (148, 409); while Delete asks, its question at (80, 319)
    // with Yes (148, 339) and No (148, 364).
    // UI-MENU-5.1: the name input at (50, 316), filled with the proposed
    // name or the chosen save's; UI-MENU-5.3: its style is not given, the
    // 256 px style 0 stands in (Q-256 e).
    if menu.delete_ask {
        p.text(
            texts.get("!Remove this file?"),
            Font::Body,
            at(80.0, 319.0),
            WHITE,
        );
        dialog_button(
            &mut p,
            texts.get("Yes").to_owned(),
            at(148.0, 339.0),
            MenuAction::Delete(true),
            true,
        );
        dialog_button(
            &mut p,
            texts.get("No").to_owned(),
            at(148.0, 364.0),
            MenuAction::Delete(false),
            true,
        );
    } else {
        if save {
            p.text_input(
                crate::ui::widgets::Input::SaveName,
                crate::ui::widgets::INPUT_STYLE_0,
                &menu.save_name,
                at(50.0, 316.0),
                focus,
            );
        } else {
            p.text(
                texts.get("!Select game state to load."),
                Font::Body,
                at(40.0, 316.0),
                WHITE,
            );
        }
        let caption = match (save, full) {
            (true, true) => "!Disk Full!",
            (true, false) => "!Save",
            (false, _) => "!Load",
        };
        dialog_button(
            &mut p,
            texts.get(caption).to_owned(),
            at(148.0, 344.0),
            if save {
                MenuAction::WriteSave
            } else {
                MenuAction::LoadSave
            },
            // UI-MENU-5.1: enabled only while the input is not empty.
            if save {
                !menu.save_name.is_empty() && !full
            } else {
                chosen
            },
        );
    }
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], at(12.0, 290.0));
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], at(12.0, 390.0));
    // UI-MENU-5.1: saving, `!Delete` is enabled only while the input equals the
    // row chosen.
    let row_named = !save
        || menu
            .saves
            .get(menu.save_choice)
            .is_some_and(|e| e.name == menu.save_name);
    dialog_button(
        &mut p,
        texts.get("!Delete").to_owned(),
        at(148.0, 409.0),
        MenuAction::AskDelete,
        chosen && row_named && !menu.delete_ask && !full,
    );
    p
}

/// A save in the save folder: the original's (`.sav`) or ours
/// (`game::save::EXTENSION`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEntry {
    /// The file name without its extension (FMT-SAV-1.3).
    pub name: String,
    pub ours: bool,
}

impl SaveEntry {
    pub fn path(&self, folder: &Path) -> PathBuf {
        if self.ours {
            crate::game::save::path_of(folder, &self.name)
        } else {
            folder.join(format!("{}.sav", self.name))
        }
    }

    /// The row's text: the name; an original's save whose name one of ours
    /// has too (the autosave) is marked " (S2)" (ours: the list of the
    /// spec holds the original's saves only).
    pub fn label(&self, all: &[SaveEntry]) -> String {
        if !self.ours && all.iter().any(|e| e.ours && e.name == self.name) {
            format!("{} (S2)", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// A typed save name as a file name: the characters Windows does not take
/// in a file name become `_`, and spaces and dots at the ends go.
pub fn file_name(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    name.trim_matches(|c: char| c == ' ' || c == '.').to_owned()
}

/// FMT-SAV-1.3: the proposed name, `DD-MM-YYYY HH-MM-SS <map name>`, from
/// the local clock.
pub(crate) fn default_save_name(map: &str) -> String {
    let (y, mo, d, h, mi, s) = local_now();
    // A map's name may hold what a file name cannot (typed names cannot, 5.3).
    file_name(&format!(
        "{d:02}-{mo:02}-{y:04} {h:02}-{mi:02}-{s:02} {map}"
    ))
}

/// The name of the map being played, for [`default_save_name`].
pub(crate) fn played_map_name(viewer: &crate::game::Viewer) -> &str {
    viewer.map.as_ref().map_or("", |map| map.info.name.as_str())
}

/// FMT-SAV-1.3: the local clock: year, month, day, hour, minute, second.
#[cfg(windows)]
fn local_now() -> (i64, u32, u32, u32, u32, u32) {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: SYSTEMTIME is plain integers; GetLocalTime fills it.
    let mut t: windows_sys::Win32::Foundation::SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    (
        i64::from(t.wYear),
        u32::from(t.wMonth),
        u32::from(t.wDay),
        u32::from(t.wHour),
        u32::from(t.wMinute),
        u32::from(t.wSecond),
    )
}

/// The clock elsewhere: the system's, in UTC (no time zone is read).
#[cfg(not(windows))]
fn local_now() -> (i64, u32, u32, u32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, mo, d) = civil_from_days((secs / 86_400) as i64);
    let t = (secs % 86_400) as u32;
    (y, mo, d, t / 3600, t / 60 % 60, t % 60)
}

/// Days since 1970-01-01 to (year, month, day).
#[cfg(any(not(windows), test))]
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `--save-dir`: the save folder in place of the user's (the play
/// harness, tests).
static SAVE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Use `dir` as the save folder from now on.
pub fn set_save_folder(dir: PathBuf) {
    let _ = SAVE_DIR.set(dir);
}

/// `--save-dir` was given.
pub fn save_folder_set() -> bool {
    SAVE_DIR.get().is_some()
}

/// A project's folder for its saves (`Extensions::save_dir`, W11 a).
static PROJECT_SAVE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Save the project's games in `dir` (`Extensions::save_dir`): set once at
/// start, the first call wins.
pub fn set_project_save_folder(dir: PathBuf) {
    let _ = PROJECT_SAVE_DIR.set(dir);
}

/// UI-MENU-5.1 (`disk_full_limit`): the save dialog shows `!Disk Full!`
/// under 5 MB free on the save drive: 5 x 2^20 bytes, strictly fewer.
pub const DISK_FULL_LIMIT: u64 = 5 << 20;

/// UI-MENU-5.1: `free` bytes on the save drive are too few; an unknown
/// amount is not.
pub fn disk_full(free: Option<u64>) -> bool {
    free.is_some_and(|bytes| bytes < DISK_FULL_LIMIT)
}

/// UI-MENU-5.1: the save drive is full, read when the save dialog opens.
pub fn save_disk_full() -> bool {
    disk_full(save_folder().and_then(|dir| free_space(&dir)))
}

/// The bytes free to this user on the drive that holds `dir` (or its
/// nearest existing parent): Windows' `GetDiskFreeSpaceExW`. Elsewhere not
/// read.
pub fn free_space(dir: &std::path::Path) -> Option<u64> {
    let dir = dir.ancestors().find(|d| d.is_dir())?;
    free_space_of(dir)
}

#[cfg(windows)]
fn free_space_of(dir: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    let mut free = 0u64;
    // SAFETY: a NUL-terminated path and a valid out pointer; the other two
    // outputs are optional.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

#[cfg(not(windows))]
fn free_space_of(_dir: &std::path::Path) -> Option<u64> {
    None
}

/// The folder our saves are written to and read from: `--save-dir`, else
/// the project's (`Extensions::save_dir`), else the original's.
pub fn save_folder() -> Option<PathBuf> {
    own_folder(
        SAVE_DIR.get(),
        PROJECT_SAVE_DIR.get(),
        original_save_folder(),
    )
}

/// FMT-SAV-1.1: the single-player save folder, `<Documents>\S2\saves`
/// (`--save-dir` in its place): the original's `.sav` files are listed from
/// it, whatever folder ours are in.
pub fn original_save_folder() -> Option<PathBuf> {
    if let Some(dir) = SAVE_DIR.get() {
        return Some(dir.clone());
    }
    Some(
        crate::menus::settings::documents()?
            .join("S2")
            .join("saves"),
    )
}

/// Our saves' folder: the option's, else the project's, else the original's.
pub(crate) fn own_folder(
    option: Option<&PathBuf>,
    project: Option<&PathBuf>,
    original: Option<PathBuf>,
) -> Option<PathBuf> {
    option.or(project).cloned().or(original)
}

/// The saves to load: ours from [`save_folder`] and the original's `.sav`
/// files (FMT-SAV-1.3) from [`original_save_folder`], in the file system's
/// order (UI-MENU-5.1).
pub fn list_saves() -> Vec<SaveEntry> {
    saves_of(save_folder().as_deref(), original_save_folder().as_deref())
}

/// The saves of `own` and `original` (W11 a): one folder, all of its saves as
/// the file system lists them; two (a project's own folder), ours from `own`
/// and then the original's `.sav` files from `original`, each folder in the
/// file system's order.
pub(crate) fn saves_of(own: Option<&Path>, original: Option<&Path>) -> Vec<SaveEntry> {
    match (own, original) {
        (Some(own), Some(original)) if own != original => saves_in(own)
            .into_iter()
            .filter(|e| e.ours)
            .chain(saves_in(original).into_iter().filter(|e| !e.ours))
            .collect(),
        (Some(dir), _) | (None, Some(dir)) => saves_in(dir),
        (None, None) => Vec::new(),
    }
}

/// The file of a listed save: ours in [`save_folder`], the original's in
/// [`original_save_folder`].
pub fn save_entry_file(entry: &SaveEntry) -> Option<PathBuf> {
    let folder = if entry.ours {
        save_folder()
    } else {
        original_save_folder()
    }?;
    Some(entry.path(&folder))
}

/// UI-MENU-5.1: the saves in `dir`, in the order the file system returns them,
/// not sorted: on NTFS that is by name, ignoring case.
pub fn saves_in(dir: &Path) -> Vec<SaveEntry> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let ours = path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case(crate::game::save::EXTENSION));
            let sav = path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("sav"));
            if !(ours || sav) || !path.is_file() {
                return None;
            }
            Some(SaveEntry {
                name: stem(&path),
                ours,
            })
        })
        .collect()
}

/// The credits page (its content is not traced, Q-096): the parchment
/// with Back.
pub fn credits(texts: &Texts, screen: [f32; 2]) -> Panel {
    let mut p = desktop(screen);
    let o = desktop_origin(screen);
    p.text(
        texts.get("!CREDITS"),
        Font::Big,
        [o[0] + 450.0, o[1] + 300.0],
        WHITE,
    );
    text_button(
        &mut p,
        texts.get("!Back").to_owned(),
        [o[0] + 700.0, o[1] + 550.0],
        MenuAction::Back,
        true,
    );
    p
}
