//! The original interface (`specs/UI-ingame.md`, `UI-menus.md`,
//! `UI-game-setup.md`): sprites cut from the game's atlases, its bitmap
//! fonts and text keys. Every game control sends the simulation's existing
//! `Command`. The viewer's keyboard panels stay behind `--debug`.

pub(crate) mod economy;
pub mod game;
pub(crate) mod info;
pub(crate) mod military;
pub mod render;
pub mod widgets;

use bevy::prelude::*;

use game::{Catalog, Context, Ctx, GameUi};
use render::{FontData, UiAssets, UiEvents, UiFrame, UiRoots};
use widgets::{Action, Atlas, Font, Global, SliderKind};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiAssets>()
            .init_resource::<UiFrame>()
            .init_resource::<UiRoots>()
            .init_resource::<UiEvents>()
            .insert_resource(GameUi::at_start())
            .init_resource::<UiCatalog>();
    }
}

/// CAM-5.3c: the cell of the open build menu (UI-GAME-3.2) an action is
/// the button of: 0 the flag button, 1, 2, ... the page's buildings.
fn build_menu_cell(
    ui: &GameUi,
    world: Option<&s2_sim::World>,
    catalog: Option<&Catalog>,
    action: &Action,
) -> Option<usize> {
    let Some(game::Context::Build { field, tab }) = ui.context else {
        return None;
    };
    match action {
        Action::Command(s2_sim::Command::PlaceFlag { at, .. }) if *at == field => Some(0),
        Action::Command(s2_sim::Command::PlaceBuilding { building, at, .. }) if *at == field => {
            let (world, catalog) = (world?, catalog?);
            let class = world.placement_at(field);
            let tabs = game::build_tabs(class);
            let tab = if tabs.contains(&tab) {
                tab
            } else {
                tabs.first().copied().unwrap_or(0)
            };
            game::build_page(catalog, class, tab)
                .iter()
                .position(|b| b.id == *building)
                .map(|i| i + 1)
        }
        _ => None,
    }
}

/// UI-GAME-4.4: the most soldiers of a rank the headquarters keeps (the
/// reserve's slider runs 0 to this).
pub(crate) const CASTLE_MOST: u32 = 15;

/// The catalog of the rules, made once.
#[derive(Resource, Default)]
pub struct UiCatalog(pub Option<Catalog>);

const ATLASES: [Atlas; 31] = [
    Atlas::Frame,
    Atlas::ShipMask,
    Atlas::Game,
    Atlas::Elements0,
    Atlas::Elements1,
    Atlas::Elements3,
    Atlas::Elements4,
    Atlas::Elements5,
    Atlas::Buildings(0),
    Atlas::Buildings(1),
    Atlas::Buildings(2),
    Atlas::SmallDialog,
    Atlas::MediumDialog,
    Atlas::Medium2Dialog,
    Atlas::BigDialog,
    Atlas::XxlDialog,
    Atlas::XxlDialogTabbed,
    Atlas::DepotLong,
    Atlas::Depot,
    Atlas::Desktop0,
    Atlas::Desktop1,
    Atlas::Title,
    Atlas::Loading0,
    Atlas::Back,
    Atlas::Logo,
    Atlas::DetailTransport,
    Atlas::MinimapCursor,
    Atlas::Book,
    Atlas::Campaign,
    Atlas::TutorialPointer,
    Atlas::TutorialCircle,
];

const FONTS: [Font; 5] = [
    Font::Heading,
    Font::Body,
    Font::Outlined,
    Font::Dark,
    Font::Big,
];

/// The language the texts and fonts are read in.
pub const LANGUAGE: &str = "gb";

/// The language files the interface takes its texts from, the first entry
/// of a key winning: CAM-1.3, 5.2 the briefings' and the tutorial's texts,
/// UI-MENU-0.3 the intro's subtitles, UI-GAME-4.4 the soldier settlers'
/// names (`dlg_PrivateSettler` ..., the headquarters' military page).
pub(crate) const TEXT_FILES: [&str; 6] = [
    "controls.txt",
    "dialogs.txt",
    "briefings.txt",
    "tutorial.txt",
    "intro.txt",
    "info.txt",
];

/// Load the atlases, fonts and texts once the install is known.
pub fn load_assets(
    viewer: Res<crate::game::Viewer>,
    mut assets: ResMut<UiAssets>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut images: ResMut<Assets<Image>>,
    mut catalog: ResMut<UiCatalog>,
) {
    if catalog.0.is_none() {
        if let Some(rules) = &viewer.rules {
            catalog.0 = Some(Catalog::new(rules, viewer.tool_goods.clone()));
        }
    }
    if !assets.atlases.is_empty() {
        return;
    }
    let pictures = (0..widgets::PICTURES.len() as u8).map(Atlas::Picture);
    for a in ATLASES.into_iter().chain(pictures) {
        let h = library.raw_image(a.file(), &mut images);
        if h.is_none() {
            eprintln!("interface atlas {} not found", a.file());
        }
        assets.atlases.insert(a, h);
    }
    let Some(root) = viewer.game_root.clone() else {
        return;
    };
    let read = |rel: &str| -> Option<Vec<u8>> {
        let path = crate::extend::find(&root, rel).ok()?;
        crate::game::read_game_file(&path)
            .map_err(|e| eprintln!("{rel}: {e}"))
            .ok()
    };
    let mut texts = s2_assets::lang::Texts::default();
    for file in TEXT_FILES {
        if let Some(data) = read(&format!("data/language/{LANGUAGE}/{file}")) {
            texts.extend(s2_assets::lang::Texts::parse(&s2_assets::lang::decode(
                &data,
            )));
        }
    }
    assets.texts = texts;
    for f in FONTS {
        let dir = format!("data/language/{LANGUAGE}/fonts");
        let data = read(&format!("{dir}/{}.fnt", f.file())).and_then(|fnt| {
            let font = s2_assets::bmfont::Font::parse(&String::from_utf8_lossy(&fnt));
            // The shipped files name no page: it is `<font>_00.tga`.
            let page_file = font
                .pages
                .first()
                .cloned()
                .unwrap_or_else(|| format!("{}_00.tga", f.file()));
            let page = read(&format!("{dir}/{page_file}"))?;
            let img = s2_assets::tga::read(&page)
                .map_err(|e| eprintln!("{page_file}: {e}"))
                .ok()?;
            Some(FontData {
                font,
                page: img.rgba,
                width: img.width,
                height: img.height,
            })
        });
        assets.fonts.insert(f, data);
    }
}

/// UI-GAME-8.5a to 8.5d, each page update of the statistics family: an
/// opening sends `STATS_MENUOPENED` (with the page's name) and makes the
/// production page read its counters; that page reads them again every 601
/// page updates (and on a toggle, `!Unmark all`, a change of span:
/// `apply_actions`).
pub fn statistics_pages(
    viewer: Res<crate::game::Viewer>,
    mut ui: ResMut<GameUi>,
    mut opened: MessageWriter<game::StatsMenuOpened>,
) {
    if let Some(event) = statistics_update(&mut ui, viewer.world.as_ref()) {
        opened.write(event);
    }
}

/// [`statistics_pages`] for one page update: the event an opening sends.
pub(crate) fn statistics_update(
    ui: &mut GameUi,
    world: Option<&s2_sim::World>,
) -> Option<game::StatsMenuOpened> {
    let page = ui.global.and_then(game::StatsPage::of);
    let mut opened = None;
    if page != ui.stats_page {
        ui.stats_page = page;
        ui.production_read = None;
        opened = page.and_then(game::StatsPage::menu_opened);
    }
    // UI-GAME-8.5d: the per-type tallies are recomputed every 22 ticks.
    if page == Some(game::StatsPage::Buildings) {
        if let Some(world) = world {
            let period = world.tick() / game::TALLY_TICKS;
            if ui.building_tally.as_ref().map(|(k, _)| *k) != Some(period) {
                let tally = game::building_counts(world, crate::game::PLAYER);
                ui.building_tally = Some((period, tally));
            }
        }
    } else {
        ui.building_tally = None;
    }
    if page != Some(game::StatsPage::Production) {
        return opened;
    }
    ui.production_updates += 1;
    if ui.production_read.is_none() || ui.production_updates >= 601 {
        let read = world
            .and_then(|w| w.statistics(crate::game::PLAYER))
            .map(|s| s.production.clone())
            .unwrap_or_default();
        ui.production_read = Some(std::sync::Arc::new(read));
        ui.production_updates = 0;
    }
    opened
}

/// The game screen's panels for this frame.
#[allow(clippy::too_many_arguments)]
pub fn game_frame(
    viewer: Res<crate::game::Viewer>,
    ui: Res<GameUi>,
    assets: Res<UiAssets>,
    catalog: Res<UiCatalog>,
    menu: Res<crate::menus::menu::MenuState>,
    mission: Res<crate::menus::campaign::MissionUi>,
    window: Single<&Window>,
    view: Res<crate::render::view3d::View3d>,
    camera: Single<(&Camera, &Transform), With<crate::render::view3d::MapCamera3d>>,
    map_hover: Res<crate::render::view3d::MapHover>,
    mut frame: ResMut<UiFrame>,
    real: Res<Time<Real>>,
    events: Res<UiEvents>,
) {
    frame.panels.clear();
    frame.map_tooltip = None;
    // UI-GAME-12.1 (Y), docs/compare.md: no HUD, dialogs or frame.
    let hidden = viewer.debug
        || (viewer.hide_interface
            && viewer.world.is_some()
            && menu.screen == crate::menus::menu::Screen::None);
    if hidden && !viewer.pause_layer {
        return;
    }
    // UI-MENU-1.1, UI-GAME-1.1: menus and game alike at 1:1 pixels.
    let screen = [window.width(), window.height()];
    if !hidden {
        if let Some(panels) = crate::menus::menu::panels(&menu, &assets.texts, screen, ui.focus()) {
            frame.panels = panels;
            return;
        }
    }
    let (Some(world), Some(catalog)) = (&viewer.world, &catalog.0) else {
        return;
    };
    let player_name = crate::menus::menu::player_name(&menu);
    let ctx = Ctx {
        world,
        player: crate::game::PLAYER,
        screen,
        texts: &assets.texts,
        catalog,
        click: ui.context_click,
        tutorial: mission.tutorial.is_some(),
        speed: viewer.speed,
        real_seconds: real.elapsed_secs_f64(),
        posted_messages: &viewer.autosave.messages,
        ship_names: &ui.ship_names,
        focus: ui.focus(),
        statistics_span: ui.statistics_span,
        statistics_hidden: ui.statistics_hidden,
        production_marks: ui.production_marks,
        production_span: ui.production_span,
        production_read: ui.production_read.as_deref(),
        building_tally: ui.building_tally.as_ref().map(|(_, t)| t),
        stats_list: ui.stats_list,
        player_name: &player_name,
        hovered: events.hovered.as_ref(),
    };
    // `--pause-layer` (the look comparison of a picture the original took
    // paused): the paused view's layer alone (UI-GAME-1.11).
    if hidden {
        frame.panels.push(game::pause_layer(&ctx));
        return;
    }
    // UI-GAME-11: the frame around the game view, under the interface.
    frame.panels.push(game::screen_frame(screen));
    // UI-GAME-9.3: the tooltip of what the pointer is over on the map.
    frame.map_tooltip = map_hover.0.and_then(|pick| game::map_tooltip(&ctx, pick));
    // UI-GAME-9.1: the construction view's labels, under the dialogs.
    if viewer.overlay && view.active {
        let (camera, transform) = *camera;
        let transform = GlobalTransform::from(*transform);
        let on_screen: Vec<_> = world
            .buildings()
            .filter(|b| b.owner == crate::game::PLAYER)
            .filter_map(|b| {
                let p = view.field_on_screen(world, (camera, &transform), b.at)?;
                (p.x >= 0.0 && p.y >= 0.0 && p.x <= screen[0] && p.y <= screen[1])
                    .then_some((b.at, p.to_array()))
            })
            .collect();
        frame
            .panels
            .push(game::construction_labels(&ctx, &on_screen));
    }
    frame.panels.extend(game::panels(&ctx, &ui, viewer.borders));
    frame.panels.extend(crate::menus::campaign::panels(
        &mission,
        &assets.texts,
        screen,
        world,
    ));
    // UI-GAME-1.11 (T46): paused by the player, a layer over everything.
    if viewer.pause_key && viewer.paused {
        frame.panels.push(game::pause_layer(&ctx));
    }
}

/// UI-GAME-1.10: where the smart cursor puts the pointer when a context
/// dialog opens: the centre of its default button: the flag, road and
/// deploy-flag menus' first portrait, the build menu's flag button, the
/// building, military and site windows' destroy button, the attack
/// dialog's attack button; the ship window has none.
pub fn smart_cursor_target(context: &Context, panels: &[widgets::Panel]) -> Option<[f32; 2]> {
    let wanted = |action: &Action| match context {
        Context::Flag(_) | Context::Road(_) | Context::DeployFlag(_) => {
            *action != Action::CloseContext
        }
        Context::Build { field, .. } => matches!(
            action,
            Action::Command(s2_sim::Command::PlaceFlag { at, .. }) if at == field
        ),
        Context::Building { .. } => matches!(
            action,
            Action::Command(s2_sim::Command::RemoveBuilding { .. })
        ),
        Context::Attack { .. } => *action == Action::AttackGo,
        Context::Ship(_) => false,
    };
    let panel = panels.iter().find(|p| p.id == widgets::PanelId::Context)?;
    panel.widgets.iter().find_map(|w| match w {
        widgets::Widget::Button {
            at,
            rect,
            size,
            action,
            ..
        } if wanted(action) => {
            let [w, h] = size.unwrap_or([rect[2] as f32, rect[3] as f32]);
            Some([panel.at[0] + at[0] + w / 2.0, panel.at[1] + at[1] + h / 2.0])
        }
        _ => None,
    })
}

/// UI-GAME-1.10: with the SMART CURSOR option on (SET-2 field 9), a newly
/// opened context dialog takes the pointer to its first button, and road
/// building takes it onto the flag's place on the screen; the pointer stays
/// within 1 % to 99 % of the window. When the flag menu closes (an action
/// chosen) the pointer goes back onto the flag (OBS-10; the spec leaves the
/// return untraced, Q-207). UI-GAME-1.10a, 1.10b, whatever the option:
/// when the deploy-flag menu, the road menu, the build menu or a building
/// window closes (`closed_on_field`), the pointer goes onto the dialog's
/// field (the new flag), if it is on the screen.
#[allow(clippy::too_many_arguments)]
pub fn smart_cursor(
    viewer: Res<crate::game::Viewer>,
    ui: Res<GameUi>,
    frame: Res<UiFrame>,
    menu: Res<crate::menus::menu::MenuState>,
    view: Res<crate::render::view3d::View3d>,
    camera: Single<(&Camera, &Transform), With<crate::render::view3d::MapCamera3d>>,
    mut window: Single<&mut Window>,
    mut last: Local<(Option<Context>, Option<s2_sim::grid::Point>)>,
) {
    let now = (ui.context.clone(), ui.road_from);
    if *last == now {
        return;
    }
    let opened = now.0.is_some() && now.0 != last.0;
    let road = now.1.filter(|_| now.1 != last.1);
    let flag_closed = match (&last.0, &now.0) {
        (Some(Context::Flag(flag)), None) => Some(*flag),
        _ => None,
    };
    let menu_closed = closed_on_field(last.0.as_ref(), now.0.as_ref());
    *last = now;
    let on_screen = |field| {
        let (camera, transform) = *camera;
        viewer.world.as_ref().and_then(|world| {
            view.field_on_screen(world, (camera, &GlobalTransform::from(*transform)), field)
                .map(|p| p.to_array())
        })
    };
    let warp = menu_closed;
    if warp.is_none() && !menu.settings.cfg.user.input_d {
        return;
    }
    let (w, h) = (window.width(), window.height());
    let target = if let Some(field) = warp {
        // Only when the field projects on screen; else the pointer stays.
        on_screen(field).filter(|&[x, y]| (0.0..=w).contains(&x) && (0.0..=h).contains(&y))
    } else if let Some(flag) = road.or(flag_closed) {
        on_screen(flag)
    } else if opened {
        ui.context
            .as_ref()
            .and_then(|c| smart_cursor_target(c, &frame.panels))
    } else {
        None
    };
    if let Some([x, y]) = target {
        let at = Vec2::new(x.clamp(0.01 * w, 0.99 * w), y.clamp(0.01 * h, 0.99 * h));
        window.set_cursor_position(Some(at));
    }
}

/// UI-GAME-12.1: the in-game keys, read on press. A key that duplicates a
/// bar button fires the button's action: SPACE the construction view (3),
/// T the briefing (1), V the speed (2, without SHIFT or CTRL), C the
/// player statistics (4), M the minimap (5), L the tools page (6), N the
/// messages (7); I the inventory, B the buildings overview; S the settings
/// at their last page; P pauses and resumes; Y hides the interface; H or
/// HOME jumps to the headquarters at the start zoom; . and , jump to the
/// next (SHIFT: previous) military building or warehouse; O follows the
/// settler under the pointer. ESC is `map_click`'s; the arrows, INSERT,
/// DELETE and the page keys are the camera's (`view3d::camera_input`).
/// RETURN opens the chat line (UI-GAME-12.1: in a single-player game too;
/// cheats are typed there).
#[allow(clippy::too_many_arguments)]
pub fn game_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<GameUi>,
    mut events: ResMut<UiEvents>,
    mut viewer: ResMut<crate::game::Viewer>,
    menu: Res<crate::menus::menu::MenuState>,
    mission: Res<crate::menus::campaign::MissionUi>,
    mut view: ResMut<crate::render::view3d::View3d>,
    hover: Res<crate::render::view3d::MapHover>,
) {
    // UI-GAME-12.2: a text input with the keys takes them all.
    if viewer.debug
        || viewer.world.is_none()
        || menu.screen != crate::menus::menu::Screen::None
        || mission.book.is_some()
        || mission.end.is_some()
        || ui.input.is_some()
    {
        return;
    }
    // UI-GAME-12.1: RETURN opens the chat line whenever the save dialog and
    // the game menu are closed.
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        ui.chat = Some(String::new());
        ui.input = Some(widgets::Input::Chat);
        ui.caret = 0;
        ui.chat_opened = true;
        return;
    }
    let modified = [
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]
    .iter()
    .any(|k| keys.pressed(*k));
    for (key, button) in [
        (KeyCode::KeyT, 1),
        (KeyCode::KeyV, 2),
        (KeyCode::Space, 3),
        (KeyCode::KeyC, 4),
        (KeyCode::KeyM, 5),
        (KeyCode::KeyL, 6),
        (KeyCode::KeyN, 7),
    ] {
        if keys.just_pressed(key) && !(button == 2 && modified) {
            events.actions.push(Action::Hud(button));
        }
    }
    if keys.just_pressed(KeyCode::KeyI) {
        events.actions.push(Action::Hud(21));
    }
    if keys.just_pressed(KeyCode::KeyB) {
        events.actions.push(Action::Hud(22));
    }
    // UI-GAME-12.1, SET-5: S (and CTRL+SHIFT+S) opens the settings dialog
    // at its last page.
    if keys.just_pressed(KeyCode::KeyS) {
        events
            .actions
            .push(Action::Menu(widgets::MenuAction::SettingsLastPage));
    }
    // UI-GAME-12.1: . the next military building, , the next headquarters,
    // storehouse or harbour; SHIFT the previous one.
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for (key, military) in [(KeyCode::Period, true), (KeyCode::Comma, false)] {
        if !keys.just_pressed(key) {
            continue;
        }
        let Some(world) = &viewer.world else { continue };
        let cursor = &mut ui.jumped[usize::from(military)];
        let next = game::next_jump(world, crate::game::PLAYER, military, *cursor, shift);
        // A press that finds nothing only clears the key's cursor.
        *cursor = next.map(|(_, serial)| serial);
        if let Some((at, _)) = next {
            ui.context = game::context_for(world, crate::game::PLAYER, at);
            viewer.focus_request = Some(at);
        }
    }
    // UI-GAME-12.1: O with a settler under the pointer: the camera follows
    // it until the next camera input.
    if keys.just_pressed(KeyCode::KeyO) {
        use crate::ui::game::MapPick;
        use s2_sim::view::SettlerRef;
        let follow = match hover.0 {
            Some(MapPick::Settler(i)) => Some(SettlerRef::Settler(i)),
            Some(MapPick::Carrier(i)) => Some(SettlerRef::Carrier(i)),
            _ => None,
        };
        if follow.is_some() {
            view.follow = follow;
        }
    }
    if keys.just_pressed(KeyCode::KeyP) {
        viewer.paused = !viewer.paused;
        viewer.pause_key = viewer.paused;
    }
    // UI-GAME-12.1: Y hides and shows the interface (HUD and frame).
    if keys.just_pressed(KeyCode::KeyY) {
        viewer.hide_interface = !viewer.hide_interface;
    }
    if keys.just_pressed(KeyCode::KeyH) || keys.just_pressed(KeyCode::Home) {
        let hq = viewer
            .world
            .as_ref()
            .and_then(|world| crate::render::view3d::headquarters_of(world, crate::game::PLAYER));
        if let Some(at) = hq {
            view.jump_home(at);
        }
    }
}

/// SND-4.1, UI-GAME-12.2, UI-MENU-5.3: the text input with the keys takes
/// the typed keys (`widgets::edit`); each inserted or removed character
/// plays `uiKey0`. RETURN or ESC, or its dialog going away, ends the input.
/// The caret's look alternates every frame. It runs after the systems that
/// read keys for the game, which leave the keys alone while an input has
/// them.
pub fn type_text(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut ui: ResMut<GameUi>,
    mut menu: ResMut<crate::menus::menu::MenuState>,
    mut events: ResMut<UiEvents>,
) {
    use widgets::{Edit, Input};
    let open = match ui.input {
        Some(Input::SaveName) => menu.screen == crate::menus::menu::Screen::Save,
        Some(Input::ShipName(i)) => {
            ui.context == Some(Context::Ship(i)) && menu.screen == crate::menus::menu::Screen::None
        }
        Some(Input::Chat) => menu.screen == crate::menus::menu::Screen::None,
        None => false,
    };
    if !open {
        // UI-MENU-5.3: closing the dialog gives the text back.
        if ui.input.take() == Some(Input::Chat) {
            ui.chat = None;
        }
        keys.clear();
        return;
    }
    // UI-GAME-12.1: the RETURN that opened the chat line is not typed into it.
    if std::mem::take(&mut ui.chat_opened) {
        keys.clear();
        return;
    }
    ui.caret_dot = !ui.caret_dot;
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        let Some(input) = ui.input else { break };
        let mut caret = ui.caret;
        // UI-MENU-5.1, 5.3: the save name is in the restricted character mode,
        // with no length limit of its own. A ship's name: neither is given
        // (Q-256 e).
        let result = match input {
            Input::SaveName => widgets::edit(
                &mut menu.save_name,
                &mut caret,
                &key.logical_key,
                None,
                true,
            ),
            Input::ShipName(i) => {
                let mut name = ui.ship_names.get(&i).cloned().unwrap_or_default();
                let result = widgets::edit(&mut name, &mut caret, &key.logical_key, None, false);
                ui.ship_names.insert(i, name);
                result
            }
            Input::Chat => {
                let mut line = ui.chat.clone().unwrap_or_default();
                let result = widgets::edit(&mut line, &mut caret, &key.logical_key, None, false);
                ui.chat = Some(line);
                result
            }
        };
        ui.caret = caret;
        match result {
            Edit::Typed | Edit::Deleted => events.sounds.push("uiKey0"),
            // UI-MENU-5.3: RETURN gives the text back, ESC clears the
            // result (a ship keeps its name; what clearing means is not
            // given, Q-256 e).
            // UI-GAME-4.5 c (Q-258 c): a ship's typed name is never kept: the
            // window never reads its input back, and the text stays only
            // until the window refreshes.
            Edit::Done => {
                ui.input = None;
                // UI-GAME-12.1, CAM-3.7: RETURN on the chat line: in a game
                // that is not a network client the line is compared with
                // the cheat words and a match applied; the line closes.
                if input == Input::Chat {
                    let line = ui.chat.take().unwrap_or_default();
                    if let Some(cheat) = game::cheat_of(&line) {
                        events.actions.push(Action::Command(s2_sim::Command::Cheat {
                            player: crate::game::PLAYER,
                            cheat,
                        }));
                    }
                }
                // UI-MENU-5.1: RETURN saves, as `!Save` does, under a name.
                if input == Input::SaveName && !menu.save_name.is_empty() {
                    events
                        .actions
                        .push(Action::Menu(widgets::MenuAction::WriteSave));
                }
            }
            Edit::Cancel => {
                ui.input = None;
                match input {
                    Input::ShipName(i) => {
                        ui.ship_names.remove(&i);
                    }
                    Input::Chat => ui.chat = None,
                    Input::SaveName => {}
                }
            }
            Edit::Moved | Edit::Ignored => {}
        }
    }
}

/// UI-MENU-1.7: the menus learn which button the pointer is over;
/// UI-MENU-2.1: Select Map reads the header of the map under it.
pub fn menu_hover(events: Res<UiEvents>, mut menu: ResMut<crate::menus::menu::MenuState>) {
    use crate::menus::menu::{hover_map, Screen};
    if menu.hovered != events.hovered {
        menu.hovered = events.hovered.clone();
    }
    if menu.screen == Screen::SelectMap {
        let row = match &events.hovered {
            Some(Action::Menu(widgets::MenuAction::SelectMap(i))) if !menu.invalid_map => Some(*i),
            _ => None,
        };
        if menu.map_hover.as_ref().map(|h| h.0) != row {
            hover_map(&mut menu, row);
        }
    }
}

/// UI-GAME-11.4: a new message plays `uiMessage1` when it is military,
/// else `uiMessage0`, while the messages window is shown.
pub fn message_sounds(
    viewer: Res<crate::game::Viewer>,
    ui: Res<GameUi>,
    mut events: ResMut<UiEvents>,
    mut seen: Local<Option<std::collections::HashSet<game::MessageKey>>>,
) {
    let Some(world) = &viewer.world else {
        *seen = None;
        return;
    };
    let now: std::collections::HashSet<game::MessageKey> = world
        .messages()
        .iter()
        .chain(&viewer.autosave.messages)
        .filter(|m| m.player == crate::game::PLAYER)
        .map(|m| (m.tick, m.at, m.text))
        .collect();
    if let Some(old) = seen.as_ref() {
        if ui.messages {
            for key in now.difference(old) {
                events.sounds.push(if game::message_kind(key.2) == 1 {
                    "uiMessage1"
                } else {
                    "uiMessage0"
                });
            }
        }
    }
    *seen = Some(now);
}

/// The actions fired this frame, applied.
#[allow(clippy::too_many_arguments)]
pub fn apply_actions(
    mut events: ResMut<UiEvents>,
    mut viewer: ResMut<crate::game::Viewer>,
    mut ui: ResMut<GameUi>,
    mut menu: ResMut<crate::menus::menu::MenuState>,
    mut mission: ResMut<crate::menus::campaign::MissionUi>,
    mut exit: MessageWriter<bevy::app::AppExit>,
    time: Res<Time>,
    assets: Res<UiAssets>,
    catalog: Res<UiCatalog>,
) {
    // UI-MENU-5.1: the save asked for in the last update, while that
    // update drew `!Saving...`.
    crate::menus::menu::finish_save(&mut menu, &mut viewer, &mission);
    // `--free-game`, `--mission`: the menu steps given at start.
    for m in std::mem::take(&mut menu.pending) {
        crate::menus::menu::apply(&m, &mut menu, &mut viewer, &mut ui, &mut mission, &mut exit);
    }
    // `--open`, `--open-global`, once the world is there.
    if let Some(world) = &viewer.world {
        if let Some(field) = viewer.open {
            ui.context = game::context_for(world, crate::game::PLAYER, field);
            if let (Some(Context::Building { tab, .. }), Some(wanted)) =
                (ui.context.as_mut(), viewer.open_tab)
            {
                *tab = wanted;
            }
        }
        for name in std::mem::take(&mut viewer.open_global) {
            match name.as_str() {
                "tools" => ui.global = Some(Global::Tools),
                "military" => ui.global = Some(Global::Military),
                "distribution" => ui.global = Some(Global::Distribution(0)),
                "transport" => ui.global = Some(Global::Transport),
                "statistics" => ui.global = Some(Global::Statistics(ui.statistics_series)),
                "production" => ui.global = Some(Global::Production),
                "inventory" => ui.global = Some(Global::Inventory(false)),
                "buildings" => ui.global = Some(Global::Buildings),
                "settlers" => ui.global = Some(Global::Inventory(true)),
                "fleet" => ui.global = Some(Global::Fleet),
                "minimap" => ui.minimap = true,
                "messages" => ui.messages = true,
                "game-menu" => menu.open_game_menu(),
                _ => {}
            }
        }
        viewer.open = None;
    }
    // UI-GAME-4.5 c: a typed ship name lasts only while that ship's window
    // shows it; reopened, the window shows the default name.
    let shown = ui.context.clone();
    ui.ship_names
        .retain(|i, _| shown == Some(Context::Ship(*i)));
    if viewer.open_building {
        let found = viewer.world.as_ref().and_then(|world| {
            world
                .buildings()
                .filter(|b| b.owner == crate::game::PLAYER && !b.under_construction)
                .find(|b| {
                    let named = viewer.building_type.as_ref().is_none_or(|wanted| {
                        viewer
                            .building_names
                            .get(&b.building_type)
                            .is_some_and(|n| n.eq_ignore_ascii_case(wanted))
                    });
                    // A type named outright may be any kind (a barracks).
                    let production = world.warehouse(b.at).is_none()
                        && world.military_at(b.at).is_none()
                        && world
                            .building_type(b.building_type)
                            .is_some_and(|k| k.worker.is_some());
                    named && (production || viewer.building_type.is_some())
                })
                .map(|b| b.at)
        });
        if let Some(at) = found {
            info!("--open-building: the building at {at:?}");
            ui.context = Some(Context::Building { at, tab: 9 });
            // UI-GAME-1.9: `--context-at` as the click it opens at.
            ui.context_click = viewer.context_at;
            viewer.focus_request = Some(at);
            viewer.open_building = false;
        }
    }
    let actions = std::mem::take(&mut events.actions);
    for action in actions {
        let opens_dialog = matches!(action, Action::OpenGlobal(_) | Action::Hud(4..=7 | 21 | 22));
        if opens_dialog {
            events.sounds.push("uiDialog0");
        }
        // CAM-5.3: the tutorial step that expects this widget advances;
        // CAM-5.3c: a build menu click by the cell it is in.
        if let Some(t) = mission.tutorial.as_mut() {
            match build_menu_cell(&ui, viewer.world.as_ref(), catalog.0.as_ref(), &action) {
                Some(cell) => t.build_menu_click(&action, cell),
                None => t.action(&action),
            }
        }
        // UI-MENU-5.3: the click puts the caret by the character it is on;
        // a ship's input starts from the name it shows (UI-GAME-4.5 c; the
        // window never reads it back).
        if let Action::FocusInput(input, text_x) = &action {
            if let (widgets::Input::ShipName(i), Some(world)) = (input, &viewer.world) {
                if ui.input != Some(*input) && !ui.ship_names.contains_key(i) {
                    let shown = game::ship_name_in(world, &assets.texts, *i);
                    ui.ship_names.insert(*i, shown);
                }
            }
            let text = match input {
                widgets::Input::SaveName => menu.save_name.clone(),
                widgets::Input::ShipName(i) => ui.ship_names.get(i).cloned().unwrap_or_default(),
                widgets::Input::Chat => ui.chat.clone().unwrap_or_default(),
            };
            let font = assets.fonts.get(&Font::Body).and_then(Option::as_ref);
            ui.caret = match (font, events.pointer) {
                (Some(f), Some(p)) => widgets::caret_at(&f.font, &text, p[0] - text_x),
                _ => text.chars().count(),
            };
        }
        apply(
            &action,
            &mut viewer,
            &mut ui,
            &mut menu,
            &mut mission,
            &mut exit,
        );
    }
    // UI-GAME-1.4: a refused command's message for 5 seconds.
    let now = time.elapsed_secs_f64();
    if let Some(text) = viewer.refusal.take() {
        events.sounds.push("uiCancel");
        ui.error = Some((text, now + 5.0));
    }
    if ui.error.as_ref().is_some_and(|(_, until)| *until < now) {
        ui.error = None;
    }
}

fn submit(viewer: &mut crate::game::Viewer, command: s2_sim::Command) {
    if let Some(world) = viewer.world.as_mut() {
        world.submit(command);
    }
}

/// UI-GAME-1.10a, 1.10b: the field the pointer goes onto when a dialog
/// closes: the deploy-flag menu's on every exit; the road menu's (Q-262 a:
/// every exit, `!Deploy flag.` among them), the build menu's (Q-262 b: its
/// flag or a type clicked, or a close request; its tabs keep it open) and
/// a building window's when they close by a button, the right button or
/// ESC. Here a click on another field opens that field's menu at once, and
/// its own jump comes first.
fn closed_on_field(last: Option<&Context>, now: Option<&Context>) -> Option<s2_sim::grid::Point> {
    match (last?, now) {
        (Context::DeployFlag(field), now) if now != last => Some(*field),
        (Context::Road(field) | Context::Build { field, .. }, None) => Some(*field),
        (Context::Building { at, .. }, None) => Some(*at),
        _ => None,
    }
}

fn apply(
    action: &Action,
    viewer: &mut crate::game::Viewer,
    ui: &mut GameUi,
    menu: &mut crate::menus::menu::MenuState,
    mission: &mut crate::menus::campaign::MissionUi,
    exit: &mut MessageWriter<bevy::app::AppExit>,
) {
    use s2_sim::Command as C;
    let player = crate::game::PLAYER;
    // UI-GAME-8.5g: a drop-down's button opens its list or closes it; any
    // other click (a choice included) closes it.
    if let Action::StatsList(k) = action {
        ui.stats_list = (ui.stats_list != Some(*k)).then_some(*k);
        return;
    }
    ui.stats_list = None;
    match action {
        Action::Command(c) => {
            submit(viewer, c.clone());
            // A placed or removed thing ends its context dialog, and so
            // does a specialist's order (whether the original's flag menu
            // closes then is not in the spec, Q-199 b).
            if matches!(
                c,
                C::PlaceFlag { .. }
                    | C::PlaceBuilding { .. }
                    | C::RemoveFlag { .. }
                    | C::RemoveRoad { .. }
                    | C::RemoveBuilding { .. }
                    | C::Attack { .. }
                    | C::OrderGeologist { .. }
                    | C::OrderScout { .. }
            ) {
                ui.context = None;
            }
        }
        Action::Hud(n) => match n {
            1 => ui.briefing = !ui.briefing,
            // UI-GAME-2.2, 2.7: the speed button (`set-game-speed`) cycles
            // 1 -> 2 -> 3 -> 1.
            2 => viewer.speed = viewer.speed % 3 + 1,
            3 => viewer.overlay = !viewer.overlay,
            // UI-GAME-2.2, 8.5, 12.1: button 4 (and C) toggles window 25, the
            // player statistics page.
            4 => {
                ui.global = match ui.global {
                    Some(Global::Statistics(_)) => None,
                    _ => Some(Global::Statistics(ui.statistics_series)),
                }
            }
            // UI-GAME-12.1: I toggles the inventory; opening it closes the
            // other dialogs.
            21 => {
                if matches!(ui.global, Some(Global::Inventory(_))) {
                    ui.global = None;
                } else {
                    ui.global = Some(Global::Inventory(false));
                    ui.context = None;
                    ui.minimap = false;
                    ui.messages = false;
                }
            }
            // UI-GAME-12.1: B toggles the buildings overview (7.1).
            22 => {
                ui.global = match ui.global {
                    Some(Global::Buildings) => None,
                    _ => Some(Global::Buildings),
                };
            }
            5 => ui.minimap = !ui.minimap,
            6 => {
                ui.global = match ui.global {
                    Some(_) => None,
                    None => Some(Global::Tools),
                }
            }
            7 => ui.messages = !ui.messages,
            9 => menu.open_game_menu(),
            20 => {
                viewer.borders = !viewer.borders;
                viewer.world_changed = true;
            }
            _ => {}
        },
        Action::FocusInput(input, _) => ui.input = Some(*input),
        // UI-GAME-4.5 d, CAM-5.3a 3: a fleet arrow selects the ship, its
        // window opens and the camera jumps to it (GFX-VIEW-3.6 d).
        Action::ShipArrow { ship: i, .. } => {
            // UI-GAME-4.5 c: the window refreshes: the default name returns.
            ui.ship_names.clear();
            ui.context = Some(Context::Ship(*i));
            if let Some(ship) = viewer.world.as_ref().and_then(|w| w.ships().get(*i)) {
                viewer.jump_request = Some(ship.position());
            }
        }
        Action::FleetShip(i) => {
            ui.context = Some(Context::Ship(*i));
            if let Some(ship) = viewer.world.as_ref().and_then(|w| w.ships().get(*i)) {
                viewer.focus_request = Some(ship.position());
            }
            ui.global = None;
        }
        Action::CloseContext => ui.context = None,
        Action::StatisticsSpan(span) => ui.statistics_span = *span,
        // UI-GAME-8.5a: the chosen series is kept by the interface.
        Action::StatisticsSeries(series) => {
            ui.statistics_series = *series;
            ui.global = Some(Global::Statistics(*series));
        }
        // UI-GAME-8.5b: a toggle, `!Unmark all` and a change of span re-read
        // the counters.
        Action::ProductionSpan(span) => {
            ui.production_span = *span;
            ui.production_read = None;
        }
        // UI-GAME-8.5f: a click on an enabled portrait toggles its curve.
        Action::StatisticsPlayer(player) => {
            if let Some(hidden) = ui.statistics_hidden.get_mut(usize::from(*player)) {
                *hidden = !*hidden;
            }
        }
        Action::ProductionMark(counter) => {
            if let Some(mark) = ui.production_marks.get_mut(usize::from(*counter)) {
                *mark = !*mark;
            }
            ui.production_read = None;
        }
        Action::ProductionUnmarkAll => {
            ui.production_marks = Default::default();
            ui.production_read = None;
        }
        Action::CloseGlobal => ui.global = None,
        // UI-GAME-8.5a: the statistics page opens on the series kept.
        Action::OpenGlobal(Global::Statistics(_)) => {
            ui.global = Some(Global::Statistics(ui.statistics_series));
        }
        Action::OpenGlobal(g) => ui.global = Some(*g),
        Action::BuildTab(t) => {
            if let Some(Context::Build { tab, .. }) = ui.context.as_mut() {
                *tab = *t;
            }
        }
        Action::BuildingTab(t) => {
            if let Some(Context::Building { tab, .. }) = ui.context.as_mut() {
                *tab = *t;
            }
        }
        Action::StartRoad(flag) | Action::StartWaterway(flag) => {
            // UI-GAME-3.4 step 1: `start-road`.
            let waterway = matches!(action, Action::StartWaterway(_));
            submit(
                viewer,
                C::StartRoute {
                    player,
                    at: *flag,
                    waterway,
                },
            );
            ui.road_from = Some(*flag);
            ui.road_since = viewer.world.as_ref().map(|w| w.tick());
            ui.waterway = waterway;
            ui.context = None;
            viewer.road_start = Some(*flag);
        }
        Action::Slider(kind, v) => {
            let thousandths = (v.clamp(0.0, 1.0) * 1000.0).round() as u32;
            let command = match *kind {
                SliderKind::Tool(slot) => Some(C::ToolWeight {
                    player,
                    slot,
                    per_mille: thousandths,
                }),
                SliderKind::Distribution { table, entry } => Some(C::DistributionWeight {
                    player,
                    table,
                    entry,
                    thousandths,
                }),
                SliderKind::Allocation(class) => Some(C::MilitaryAllocation {
                    player,
                    class,
                    thousandths,
                }),
                SliderKind::AttackStrength => Some(C::MilitaryAttackStrength {
                    player,
                    thousandths,
                }),
                SliderKind::Defenders => Some(C::MilitaryDefenders {
                    player,
                    thousandths,
                }),
                SliderKind::Volume(kind) => {
                    menu.set_volume(kind, *v);
                    None
                }
                SliderKind::Scene(field) => {
                    menu.set_scene_slider(field, *v);
                    None
                }
                SliderKind::ScrollSpeed => {
                    menu.set_scroll_speed(*v);
                    None
                }
                SliderKind::AttackShare => {
                    if let Some(Context::Attack { share, .. }) = ui.context.as_mut() {
                        *share = *v;
                    }
                    None
                }
                // UI-GAME-4.4: the reserve of a rank, 0 to 15, in the
                // headquarters whose window is open.
                SliderKind::Castle(rank) => match ui.context {
                    Some(Context::Building { at, .. }) => Some(C::SetCastleSoldiers {
                        player,
                        at,
                        rank,
                        amount: (v.clamp(0.0, 1.0) * CASTLE_MOST as f32).round() as u32,
                    }),
                    _ => None,
                },
            };
            if let Some(c) = command {
                submit(viewer, c);
            }
        }
        Action::AttackPreset(i) => {
            if let (Some(Context::Attack { target, share, .. }), Some(world)) =
                (ui.context.as_mut(), viewer.world.as_ref())
            {
                let available = world.attack_available(player, *target);
                let count = world.attack_counts(player, *target)[*i];
                if available > 0 {
                    *share = count as f32 / available as f32;
                }
            }
        }
        Action::AttackStrong(s) => {
            if let Some(Context::Attack { strong, .. }) = ui.context.as_mut() {
                *strong = *s;
            }
        }
        Action::AttackGo => {
            if let (
                Some(Context::Attack {
                    target,
                    share,
                    strong,
                }),
                Some(world),
            ) = (ui.context.clone(), viewer.world.as_ref())
            {
                let count = game::attack_count(world.attack_available(player, target), share);
                submit(
                    viewer,
                    C::Attack {
                        player,
                        target,
                        count,
                        strong_first: strong,
                    },
                );
                ui.context = None;
            }
        }
        Action::MessagesScroll(d) => {
            ui.message_scroll = (ui.message_scroll as i64 + i64::from(*d)).max(0) as usize;
        }
        Action::Focus(field) => viewer.focus_request = Some(*field),
        // UI-GAME-8.1a (c): the camera moves to the ship, which is selected
        // and its window opens.
        Action::JumpToShip(i) => {
            if let Some(ship) = viewer.world.as_ref().and_then(|w| w.ships().get(*i)) {
                viewer.jump_request = Some(ship.position());
                ui.context = Some(Context::Ship(*i));
            }
        }
        Action::MessageFilter(k) => {
            if let Some(hidden) = ui.message_hidden.get_mut(*k) {
                *hidden = !*hidden;
            }
        }
        // UI-GAME-11.4: a row's field: the camera jumps there (zoom kept).
        // A message without a field (the autosave's) moves no camera.
        Action::MessageRow(key) => {
            if key.1 != game::NO_FIELD {
                viewer.jump_request = Some(key.1);
            }
        }
        Action::DeleteMessage(key) => {
            ui.deleted_messages.insert(*key);
        }
        Action::DepotAll { at, settlers } => {
            let mode = &mut ui.depot_modes[usize::from(*settlers)];
            *mode = (*mode + 1) % 3;
            submit(
                viewer,
                C::WarehouseAll {
                    player,
                    at: *at,
                    mode: *mode,
                    settlers: *settlers,
                },
            );
        }
        Action::Menu(m) => crate::menus::menu::apply(m, menu, viewer, ui, mission, exit),
        Action::None | Action::StatsList(_) => {}
    }
}

/// What Escape does in the game's screen (without a text input).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscapeDoes {
    CloseMenu,
    CloseBook,
    OpenGameMenu,
    CloseDialog,
}

/// UI-MENU-3.1: Escape closes an open menu screen, and opens the game menu
/// when nothing else is open; with a dialog open it closes the dialog
/// (UI-GAME-3). CAM-3.1b (E-trace-025): with the briefing book open it
/// closes the book, as the book's CONTINUE does (the mission panel stays).
fn escape_does(menu_open: bool, book_open: bool, dialog_open: bool) -> EscapeDoes {
    if menu_open {
        EscapeDoes::CloseMenu
    } else if book_open {
        EscapeDoes::CloseBook
    } else if dialog_open {
        EscapeDoes::CloseDialog
    } else {
        EscapeDoes::OpenGameMenu
    }
}

/// UI-GAME-3: a click on the map opens the context dialog of what was
/// clicked; in road building (3.4) a click on an own flag or a field that
/// takes a flag ends the road there. The right mouse button cancels.
#[allow(clippy::too_many_arguments)]
pub fn map_click(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: ResMut<UiEvents>,
    hover: Res<crate::input::Hover>,
    mut viewer: ResMut<crate::game::Viewer>,
    mut ui: ResMut<GameUi>,
    window: Single<&Window>,
    mut view: ResMut<crate::render::view3d::View3d>,
    mut menu: ResMut<crate::menus::menu::MenuState>,
    mut mission: ResMut<crate::menus::campaign::MissionUi>,
    ship: Res<crate::render::view3d::ShipHover>,
) {
    if viewer.debug || viewer.world.is_none() {
        return;
    }
    if let Some(field) = viewer.focus_request.take() {
        view.focus_field(field);
    }
    if let Some(field) = viewer.jump_request.take() {
        view.jump_to_field(field);
    }
    // CAM-5.3b: a tutorial step's start moves the camera to its field.
    if let Some(field) = mission.tutorial.as_mut().and_then(|t| t.camera_move.take()) {
        view.move_to_field(field);
    }
    follow_route(&mut viewer, &mut ui);
    // UI-MENU-3.1: Escape opens the game menu when nothing else is open,
    // and closes it; it does not cancel road building (UI-GAME-3.4). With
    // a text input it ends the input (`type_text`).
    if keys.just_pressed(KeyCode::Escape) && ui.input.is_none() {
        let menu_open = menu.screen != crate::menus::menu::Screen::None;
        match escape_does(menu_open, mission.book.is_some(), ui.context.is_some()) {
            EscapeDoes::CloseMenu => {
                menu.screen = crate::menus::menu::Screen::None;
                return;
            }
            EscapeDoes::CloseBook => {
                if let Some(screen) = mission.close_book() {
                    viewer.paused = true;
                    menu.in_game = false;
                    menu.screen = screen;
                }
                return;
            }
            EscapeDoes::OpenGameMenu => {
                menu.open_game_menu();
                return;
            }
            EscapeDoes::CloseDialog => {}
        }
    }
    if menu.screen != crate::menus::menu::Screen::None {
        return;
    }
    // The book dialog and the end screen are modal.
    if mission.book.is_some() || mission.end.is_some() {
        return;
    }
    // The minimap moves the camera.
    if let Some(p) = render::minimap_press(&events, window.cursor_position()) {
        if let Some(world) = &viewer.world {
            let t = world.terrain();
            let field = (
                (p.x * t.width as f32).clamp(0.0, t.width as f32 - 1.0) as u32,
                (p.y * t.height as f32).clamp(0.0, t.height as f32 - 1.0) as u32,
            );
            view.focus_field(field);
        }
    }
    let cursor = window.cursor_position();
    // UI-GAME-3.4 step 5: the right button cancels road building.
    if buttons.just_released(MouseButton::Right) && ui.road_from.is_some() {
        cancel_route(&mut viewer, &mut ui, &mut events);
    } else if buttons.just_released(MouseButton::Right) || keys.just_pressed(KeyCode::Escape) {
        ui.context = None;
    }
    // Over a dialog the press is the dialog's.
    if map_takes_press(&buttons) && !events.over_ui {
        debug!("map press at {cursor:?}");
        map_press(
            &mut events,
            (&hover, ship.0),
            &mut viewer,
            &mut ui,
            &mut mission,
            cursor,
        );
    }
}

/// UI-GAME-1.10c: the map takes the left button's press as the frame reads
/// it: pressed since the last frame and still down. A press released again
/// within the same frame has cleared its own mark and is not seen (T45).
fn map_takes_press(buttons: &ButtonInput<MouseButton>) -> bool {
    buttons.just_pressed(MouseButton::Left) && buttons.pressed(MouseButton::Left)
}

/// UI-GAME-3, 1.10c, 9.4: a left press on the map at the screen point `at`
/// opens the dialog of what is under it (the field, and the ship whose
/// model is there), or in road building (3.4) takes the route there.
fn map_press(
    events: &mut UiEvents,
    (hover, ship): (&crate::input::Hover, Option<usize>),
    viewer: &mut crate::game::Viewer,
    ui: &mut GameUi,
    mission: &mut crate::menus::campaign::MissionUi,
    at: Option<Vec2>,
) {
    let Some(end) = at else { return };
    // UI-GAME-8.5g: a click anywhere else closes an open drop-down list.
    ui.stats_list = None;
    let Some(field) = hover.0.map(|f| (f.x, f.y)) else {
        return;
    };
    // CAM-5.3a 1: the tutorial's ship step finds the ship by its field.
    let by_field = mission
        .tutorial
        .as_ref()
        .is_some_and(|t| crate::menus::campaign::ship_by_field(t.step));
    // CAM-5.3: in a tutorial only the marked field takes a click.
    if let Some(t) = mission.tutorial.as_mut() {
        if t.field_click(field) == crate::menus::campaign::FieldClick::Refuse {
            return;
        }
    }
    let world = viewer.world.as_ref().expect("checked");
    let player = crate::game::PLAYER;
    if let Some(from) = ui.road_from {
        // UI-GAME-3.4: a click on the start flag cancels (5), on the
        // route's end opens the deploy-flag menu (4), elsewhere extends the
        // route there or cuts it back (2, 3).
        let route = world.route(player);
        let first = route.map_or(from, |r| r.fields[0]);
        let route_end = route.filter(|r| r.fields.len() > 1).map(|r| r.end());
        if field == first {
            cancel_route(viewer, ui, events);
        } else if Some(field) == route_end {
            ui.context = Some(Context::DeployFlag(field));
            ui.context_click = Some(end.to_array());
        } else {
            events.sounds.push("uiClick0");
            submit(viewer, s2_sim::Command::ExtendRoute { player, to: field });
        }
        return;
    }
    ui.context = if by_field {
        game::context_for(world, player, field)
    } else {
        game::context_for_click(world, player, field, ship)
    };
    ui.context_click = Some(end.to_array());
}

/// UI-GAME-3.4 step 5: road building is cancelled: `uiCancel`,
/// `cancel-road`, the mode ends.
fn cancel_route(viewer: &mut crate::game::Viewer, ui: &mut GameUi, events: &mut UiEvents) {
    events.sounds.push("uiCancel");
    submit(
        viewer,
        s2_sim::Command::CancelRoute {
            player: crate::game::PLAYER,
        },
    );
    end_road_mode(viewer, ui);
}

fn end_road_mode(viewer: &mut crate::game::Viewer, ui: &mut GameUi) {
    ui.road_from = None;
    ui.road_since = None;
    viewer.road_start = None;
    if matches!(ui.context, Some(Context::DeployFlag(_))) {
        ui.context = None;
    }
}

/// UI-GAME-3.4: road building follows the player's route: its end is
/// where the cursor's route search starts (GFX-VIEW-6.2); the mode ends
/// when the route is gone (built, cancelled or dropped by the logic, 8.3),
/// or when it never came (the start was refused).
fn follow_route(viewer: &mut crate::game::Viewer, ui: &mut GameUi) {
    if ui.road_from.is_none() {
        return;
    }
    let Some(world) = viewer.world.as_ref() else {
        return;
    };
    let tick = world.tick();
    match world.route(crate::game::PLAYER) {
        Some(route) => {
            ui.road_since = None;
            viewer.road_start = Some(route.end());
        }
        // The start is applied at the next tick boundary.
        None if ui.road_since.is_some_and(|t| tick <= t + 1) => {}
        None => end_road_mode(viewer, ui),
    }
}

/// The shown map's preview (UI-MENU-2.1): the `.bmp` beside the map file;
/// none while Select Map has no readable map under the pointer.
pub fn update_preview(
    menu: Res<crate::menus::menu::MenuState>,
    mut assets: ResMut<UiAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(summary) = crate::menus::menu::shown_map(&menu) else {
        if assets.preview.is_some() {
            assets.preview = None;
        }
        return;
    };
    if assets
        .preview
        .as_ref()
        .is_some_and(|(p, _)| *p == summary.path)
    {
        return;
    }
    let bmp = summary.path.with_extension("bmp");
    let Some(img) = std::fs::read(&bmp)
        .ok()
        .and_then(|d| s2_assets::bmp::read(&d))
    else {
        assets.preview = None;
        return;
    };
    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: img.width,
            height: img.height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        img.rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    assets.preview = Some((summary.path.clone(), images.add(image)));
}

/// The minimap picture (UI-GAME-2.5, 8.4): under the `regions` layer
/// (each owned field in its owner's colour, alpha 0x99 on border fields and
/// 0x55 inside) and the `cities` layer (buildings as 2 x 2 blocks, 0xAAAAAA,
/// military 0xFF8888) lies the terrain's colour (what the game draws
/// beneath is not said, Q-184); unexplored land is black.
pub fn update_minimap(
    viewer: Res<crate::game::Viewer>,
    ui: Res<GameUi>,
    mut assets: ResMut<UiAssets>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<Option<(u64, u64)>>,
) {
    if !ui.minimap {
        return;
    }
    let Some(world) = &viewer.world else { return };
    let key = (viewer.loads, world.tick() / 50);
    if *last == Some(key) && assets.minimap.is_some() {
        return;
    }
    *last = Some(key);
    let t = world.terrain();
    let (w, h) = (t.width, t.height);
    let map = viewer.map.as_ref();
    let mut colours: Vec<[f32; 3]> = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let field = (x, y);
            let pattern = map.map(|m| m.patterns[(y * w + x) as usize]);
            let mut c = pattern
                .and_then(|p| viewer.terrain_colors.get(&p).copied())
                .unwrap_or([0.3, 0.5, 0.2]);
            if let Some(owner) = world.field_owner(field) {
                let p = crate::render::models3d::player_colour(owner);
                let a = if t.has(field, s2_sim::map::state::BORDER) {
                    0x99 as f32 / 255.0
                } else {
                    0x55 as f32 / 255.0
                };
                c = [0, 1, 2].map(|k| c[k] + (p[k] - c[k]) * a);
            }
            colours.push(c);
        }
    }
    for b in world.buildings() {
        let c = if world.military_at(b.at).is_some() {
            [1.0, 0.533, 0.533]
        } else {
            [0.667; 3]
        };
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (x, y) = (b.at.0 + dx, b.at.1 + dy);
            if x < w && y < h {
                colours[(y * w + x) as usize] = c;
            }
        }
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut c = colours[(y * w + x) as usize];
            if viewer.fogged(world, (x, y)) {
                c = [0.0; 3];
            }
            let [r, g, b] = c.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
            rgba.extend([r, g, b, 255]);
        }
    }
    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    match &assets.minimap {
        Some(handle) => {
            if let Some(mut img) = images.get_mut(handle) {
                *img = image;
            }
        }
        None => assets.minimap = Some(images.add(image)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::ButtonState;
    use widgets::{Edit, Input};

    /// UI-MENU-5.3: a printable key or SPACE is inserted at the caret,
    /// BACKSPACE removes the character left of it and DELETE the one right
    /// of it (only when there is one), LEFT / RIGHT / HOME / END move it,
    /// RETURN and ESC end the input; a key refused at the input's limit
    /// does nothing.
    // spec: UI-MENU-5.3, SND-4.1
    #[test]
    fn text_input_keys() {
        let mut text = String::new();
        let mut caret = 0;
        let key = |text: &mut String, caret: &mut usize, k: Key| {
            widgets::edit(text, caret, &k, None, false)
        };
        assert_eq!(
            key(&mut text, &mut caret, Key::Character("S".into())),
            Edit::Typed
        );
        assert_eq!(key(&mut text, &mut caret, Key::Space), Edit::Typed);
        assert_eq!(
            key(&mut text, &mut caret, Key::Character("y".into())),
            Edit::Typed
        );
        assert_eq!((text.as_str(), caret), ("S y", 3));
        assert_eq!(key(&mut text, &mut caret, Key::ArrowLeft), Edit::Moved);
        assert_eq!(key(&mut text, &mut caret, Key::ArrowLeft), Edit::Moved);
        assert_eq!(
            key(&mut text, &mut caret, Key::Character("k".into())),
            Edit::Typed
        );
        assert_eq!((text.as_str(), caret), ("Sk y", 2));
        assert_eq!(key(&mut text, &mut caret, Key::Delete), Edit::Deleted);
        assert_eq!((text.as_str(), caret), ("Sky", 2));
        assert_eq!(key(&mut text, &mut caret, Key::Backspace), Edit::Deleted);
        assert_eq!((text.as_str(), caret), ("Sy", 1));
        assert_eq!(key(&mut text, &mut caret, Key::Home), Edit::Moved);
        assert_eq!(key(&mut text, &mut caret, Key::Backspace), Edit::Ignored);
        assert_eq!(key(&mut text, &mut caret, Key::End), Edit::Moved);
        assert_eq!(key(&mut text, &mut caret, Key::Delete), Edit::Ignored);
        assert_eq!(key(&mut text, &mut caret, Key::ArrowRight), Edit::Moved);
        assert_eq!(caret, 2);
        assert_eq!(key(&mut text, &mut caret, Key::Enter), Edit::Done);
        assert_eq!(key(&mut text, &mut caret, Key::Escape), Edit::Cancel);
        assert_eq!(key(&mut text, &mut caret, Key::F1), Edit::Ignored);
        // The call sign input's limit of 15.
        let mut full = "x".repeat(15);
        let mut end = 15;
        assert_eq!(
            widgets::edit(
                &mut full,
                &mut end,
                &Key::Character("z".into()),
                Some(15),
                false
            ),
            Edit::Ignored
        );
        assert_eq!(full.len(), 15);
    }

    /// UI-MENU-5.3: a click puts the caret before the character that ends
    /// the first prefix wider than the click's x minus 3 px.
    // spec: UI-MENU-5.3
    #[test]
    fn caret_by_click() {
        let font = s2_assets::bmfont::Font::parse(
            "common lineHeight=16 base=13\nchar id=65 x=0 y=0 width=8 height=16 xoffset=0 yoffset=0 xadvance=10 page=0\n",
        );
        assert_eq!(widgets::caret_at(&font, "AAA", 0.0), 0);
        assert_eq!(widgets::caret_at(&font, "AAA", 12.9), 0);
        assert_eq!(widgets::caret_at(&font, "AAA", 13.0), 1);
        assert_eq!(widgets::caret_at(&font, "AAA", 33.0), 3);
    }

    fn press(app: &mut App, key: Key) {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    /// UI-GAME-12.1, CAM-3.7 (Q-258 e): the chat line takes the keys; RETURN
    /// compares the line with the cheat words and applies a match; the line
    /// closes. Another line is no cheat; the RETURN that opened the line is
    /// not typed into it.
    // spec: UI-GAME-12.1, CAM-3.7
    #[test]
    fn the_chat_line_takes_the_cheats() {
        use crate::ui::game::cheat_of;
        assert_eq!(
            cheat_of("INeedSomeHelp"),
            Some(s2_sim::mission::Cheat::INeedSomeHelp)
        );
        assert_eq!(cheat_of("ineedsomehelp"), None);
        assert_eq!(cheat_of("TooFast"), None);
        let mut app = App::new();
        app.add_message::<KeyboardInput>()
            .insert_resource(GameUi {
                input: Some(Input::Chat),
                chat: Some(String::new()),
                chat_opened: true,
                ..GameUi::default()
            })
            .init_resource::<crate::menus::menu::MenuState>()
            .init_resource::<UiEvents>()
            .add_systems(Update, type_text);
        // The opening RETURN's frame: nothing typed.
        press(&mut app, Key::Enter);
        assert_eq!(app.world().resource::<GameUi>().input, Some(Input::Chat));
        for c in "ShowMeTheWorld".chars() {
            press(&mut app, Key::Character(c.to_string().into()));
        }
        assert_eq!(
            app.world().resource::<GameUi>().chat.as_deref(),
            Some("ShowMeTheWorld")
        );
        press(&mut app, Key::Enter);
        let ui = app.world().resource::<GameUi>();
        assert_eq!((ui.input, ui.chat.as_ref()), (None, None));
        assert_eq!(
            app.world().resource::<UiEvents>().actions,
            [Action::Command(s2_sim::Command::Cheat {
                player: crate::game::PLAYER,
                cheat: s2_sim::mission::Cheat::ShowMeTheWorld
            })]
        );
    }

    /// SND-4.1: with the ship's name input open, each typed key adds to the
    /// name and plays `uiKey0`; RETURN gives the keys back without a sound;
    /// the ship is not named (UI-GAME-4.5 c: a typed name is never kept, the
    /// text stays in the input until the window refreshes); keys typed after
    /// that change nothing.
    // spec: SND-4.1, UI-MENU-5.3, UI-GAME-4.5
    #[test]
    fn typing_a_ship_name_plays_ui_key0() {
        let mut app = App::new();
        app.add_message::<KeyboardInput>()
            .insert_resource(GameUi {
                context: Some(Context::Ship(2)),
                input: Some(Input::ShipName(2)),
                ..GameUi::default()
            })
            .init_resource::<crate::menus::menu::MenuState>()
            .init_resource::<UiEvents>()
            .add_systems(Update, type_text);
        press(&mut app, Key::Character("A".into()));
        press(&mut app, Key::Character("b".into()));
        assert_eq!(app.world().resource::<UiEvents>().sounds, ["uiKey0"; 2]);
        press(&mut app, Key::Enter);
        press(&mut app, Key::Character("c".into()));
        let ui = app.world().resource::<GameUi>();
        assert_eq!(ui.ship_names.get(&2).map(String::as_str), Some("Ab"));
        assert_eq!(ui.input, None);
        let events = app.world().resource::<UiEvents>();
        assert_eq!(events.sounds.len(), 2);
        assert!(events.actions.is_empty(), "no name is sent");
    }

    /// UI-GAME-1.10c (T45): the map sees a left press that is still down
    /// when the frame reads it; a press released within the same frame is
    /// not seen, a press released and pressed again is.
    // spec: UI-GAME-1.10c (T45)
    #[test]
    fn the_map_sees_a_press_once_a_frame() {
        let mut buttons = ButtonInput::<MouseButton>::default();
        buttons.press(MouseButton::Left);
        assert!(map_takes_press(&buttons));
        buttons.clear();
        // Held into the next frame: no new press.
        assert!(!map_takes_press(&buttons));
        buttons.release(MouseButton::Left);
        buttons.clear();
        // Pressed and released between two frames.
        buttons.press(MouseButton::Left);
        buttons.release(MouseButton::Left);
        assert!(!map_takes_press(&buttons));
        // Released and pressed again.
        buttons.press(MouseButton::Left);
        assert!(map_takes_press(&buttons));
    }

    /// UI-GAME-1.10b (Q-262 a, b; F41): the road menu and the build menu
    /// warp onto their field on every exit that closes them (the build
    /// menu's flag and types close it), not when a tab changes; the
    /// deploy-flag menu on every exit; a building window when it closes;
    /// the flag menu does not (its return is the smart cursor's, OBS-10).
    // spec: UI-GAME-1.10a, UI-GAME-1.10b
    #[test]
    fn the_map_menus_and_building_windows_warp_when_they_close() {
        let (field, other) = ((12, 20), (30, 31));
        let road = Context::Road(field);
        let deploy = Context::DeployFlag(field);
        let window = |tab| Context::Building { at: field, tab };
        let build = Context::Build {
            field: other,
            tab: 0,
        };
        let build_tab = Context::Build {
            field: other,
            tab: 2,
        };
        assert_eq!(closed_on_field(Some(&build), None), Some(other));
        assert_eq!(closed_on_field(Some(&build), Some(&build_tab)), None);
        assert_eq!(closed_on_field(Some(&build), Some(&road)), None);
        assert_eq!(closed_on_field(Some(&road), None), Some(field));
        assert_eq!(closed_on_field(Some(&road), Some(&build)), None);
        assert_eq!(closed_on_field(Some(&road), Some(&road)), None);
        assert_eq!(closed_on_field(Some(&deploy), None), Some(field));
        assert_eq!(closed_on_field(Some(&deploy), Some(&build)), Some(field));
        assert_eq!(closed_on_field(Some(&deploy), Some(&deploy)), None);
        assert_eq!(closed_on_field(Some(&window(0)), None), Some(field));
        assert_eq!(closed_on_field(Some(&window(0)), Some(&window(2))), None);
        assert_eq!(closed_on_field(Some(&Context::Flag(field)), None), None);
        assert_eq!(closed_on_field(None, Some(&road)), None);
    }

    /// CAM-3.1b (E-trace-025): Escape closes the briefing book (not the
    /// game menu over it); with the book closed it opens the game menu, with
    /// a dialog open it closes the dialog, with a menu screen open it closes
    /// that. UI-MENU-3.1.
    // spec: CAM-3.1b, UI-MENU-3.1
    #[test]
    fn escape_closes_the_briefing_book() {
        assert_eq!(escape_does(false, true, false), EscapeDoes::CloseBook);
        assert_eq!(escape_does(false, true, true), EscapeDoes::CloseBook);
        assert_eq!(escape_does(false, false, false), EscapeDoes::OpenGameMenu);
        assert_eq!(escape_does(false, false, true), EscapeDoes::CloseDialog);
        assert_eq!(escape_does(true, true, false), EscapeDoes::CloseMenu);
    }

    /// UI-GAME-12.2: the input gives the keys back when its dialog closes.
    #[test]
    fn closing_the_ship_window_ends_the_input() {
        let mut app = App::new();
        app.add_message::<KeyboardInput>()
            .insert_resource(GameUi {
                context: None,
                input: Some(Input::ShipName(0)),
                ..GameUi::default()
            })
            .init_resource::<crate::menus::menu::MenuState>()
            .init_resource::<UiEvents>()
            .add_systems(Update, type_text);
        press(&mut app, Key::Character("A".into()));
        assert_eq!(app.world().resource::<GameUi>().input, None);
        assert!(app.world().resource::<UiEvents>().sounds.is_empty());
    }
}
