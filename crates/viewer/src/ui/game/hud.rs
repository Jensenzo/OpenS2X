//! What stays on the game screen: messages, minimap, ship window, map
//! tooltips and labels, the screen frame, errors and the speed indicator
//! (UI-GAME-2, 4.5, 9, 11).

use super::*;

/// UI-GAME-11.1: a message as the window keeps it: its tick, field and
/// text.
pub type MessageKey = (u64, Point, &'static str);

/// UI-GAME-11.1: the kind of a message by its text: 1 military, 2
/// geologist, 3 economic, 4 misc, 7 the others (always shown).
pub fn message_kind(text: &str) -> u8 {
    match text {
        "Military building occupied!"
        | "Watchtower occupied!"
        | "Military Building conquered."
        | "Lost Military Building to enemy."
        | "Attacked Castle destroyed."
        | "You are under attack!"
        | "Lost Territory."
        | "Soldier killed by Catapult" => 1,
        t if t.starts_with("Found ") => 2,
        "No resources found." | "Building without worker." => 3,
        "Building constructed"
        | "Ship finished."
        | "Ship constructed"
        | "Ship waiting for Orders" => 4,
        // Hook W24: a project's message of finished work, as "Building
        // constructed".
        t if crate::extend::messages::is_project_text(t) => 4,
        _ => 7,
    }
}

/// UI-GAME-11.3: the x of a kind's icons in `uiElements3`: swords,
/// magnifier, house, exclamation mark; the speech bubble for 5 to 7.
pub(crate) fn kind_icon_x(kind: u8) -> u32 {
    match kind {
        1 => 72,
        2 => 104,
        3 => 40,
        4 => 8,
        _ => 136,
    }
}

/// UI-GAME-11.3: rows the window shows.
pub const MESSAGE_ROWS: usize = 7;

/// UI-GAME-11.1: the field of a message that has none (the autosave's,
/// FMT-SAV-5.2): a click on its row moves no camera.
pub const NO_FIELD: Point = (u32::MAX, u32::MAX);

/// UI-GAME-11.2: a message's 600 timer units at the fixed 1/60 s a tick.
pub const MESSAGE_TICKS: u64 = 36_000;

/// UI-GAME-11.1: the player's message list: the simulation's and the
/// interface's own (the autosave's, FMT-SAV-5.2), in arrival order.
pub fn all_messages<'a>(
    world: &'a [s2_sim::Message],
    posted: &'a [s2_sim::Message],
) -> Vec<&'a s2_sim::Message> {
    let mut all: Vec<&s2_sim::Message> = world.iter().chain(posted).collect();
    // Stable: the same tick keeps the simulation's first.
    all.sort_by_key(|m| m.tick);
    all
}

/// UI-GAME-11.1 to 11.3: the player's messages the window lists, oldest
/// first: shown still, not deleted, of a kind whose filter is on (kinds 5
/// to 7 always).
pub fn listed_messages<'a>(
    messages: &[&'a s2_sim::Message],
    player: PlayerId,
    tick: u64,
    hidden: [bool; 4],
    deleted: &std::collections::HashSet<MessageKey>,
) -> Vec<&'a s2_sim::Message> {
    messages
        .iter()
        .copied()
        .filter(|m| m.player == player && m.until > tick)
        .filter(|m| !deleted.contains(&(m.tick, m.at, m.text)))
        .filter(|m| {
            let kind = message_kind(m.text);
            !(1..=4).contains(&kind) || !hidden[usize::from(kind - 1)]
        })
        .collect()
}

/// UI-GAME-11.3: the rows on screen: the seven ending `scroll` rows before
/// the newest, newest at the bottom.
pub fn message_window<T>(listed: &[T], scroll: usize) -> &[T] {
    let end = listed.len().saturating_sub(scroll);
    &listed[end.saturating_sub(MESSAGE_ROWS)..end]
}

/// UI-GAME-11.1 (Q-1068): the display name of the geologist's settler type,
/// the text of its info title (the scripts' `Scholar`).
pub(crate) const GEOLOGIST_NAME: &str = "dlg_ScholarSettler";

/// UI-GAME-11.1: a row's text: the translated text, and in brackets the
/// ship's name for a message naming a ship (UI-GAME-4.5 c, "Ship is
/// awaiting orders (Aurora)"), else the building's for the messages that
/// name one.
pub(crate) fn message_text(ctx: &Ctx, m: &s2_sim::Message) -> String {
    // Hook W24: a project's text as given.
    let text = if crate::extend::messages::is_project_text(m.text) {
        m.text.to_owned()
    } else {
        ctx.t(&format!("!{}", m.text))
    };
    let ship = m.ship.map(|s| ship_name(ctx, s as usize));
    if let Some(name) = ship.filter(|n| !n.is_empty()) {
        return format!("{text} ({name})");
    }
    // UI-GAME-11.1 (Q-1068, E-trace-032 seen): a find of the geologist's
    // report names his settler type by its display name, its info title
    // (`dlg_ScholarSettler`, "Iron ore found! (Geologist)"); only the
    // geologist's report posts the five found texts.
    if m.text.starts_with("Found ") {
        return format!("{text} ({})", ctx.texts.name(GEOLOGIST_NAME));
    }
    // UI-GAME-11.1 (E-trace-033, seen): the lost building's and the lost
    // territory's rows carry no name, though the building still stands
    // when the first is posted ("Military building lost to enemy!").
    if matches!(
        m.text,
        "Lost Military Building to enemy." | "Lost Territory."
    ) {
        return text;
    }
    // E-trace-032 (seen): "No resources found (Fisherman's Hut)" names its
    // building too; E-trace-033: "You are under attack! (Barracks)".
    match m.building {
        Some(b) => format!("{text} ({})", title_of(ctx, b)),
        None => text,
    }
}

/// UI-GAME-11.3: the messages window at the top left: four filter
/// buttons, seven rows (kind icon and text, newest at the bottom) and the
/// column of scroll up, scroll down and jump to the newest on the left.
pub fn messages(ctx: &Ctx, ui: &GameUi) -> Panel {
    const TOP: f32 = 44.0;
    const PITCH: f32 = 29.0;
    let mut p = Panel::new(PanelId::Messages, [0.0, 0.0]);
    let filters = [
        "!Show military messages",
        "!Show scholar messages",
        "!Show economic messages",
        "!Show misc messages",
    ];
    for (k, key) in filters.iter().enumerate() {
        let lit = !ui.message_hidden[k];
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements3,
            rect: [
                kind_icon_x(k as u8 + 1),
                if lit { 280 } else { 248 },
                24,
                24,
            ],
            hover: Hover::None,
            pressed: Hover::None,
            at: [36.0 + 35.0 * k as f32, 10.0],
            size: Some([32.0, 32.0]),
            tooltip: Some(ctx.t(key)),
            action: Action::MessageFilter(k),
            enabled: true,
            selected: false,
        });
    }
    let listed = listed_messages(
        &all_messages(ctx.world.messages(), ctx.posted_messages),
        ctx.player,
        ctx.world.tick(),
        ui.message_hidden,
        &ui.deleted_messages,
    );
    let scroll = ui
        .message_scroll
        .min(listed.len().saturating_sub(MESSAGE_ROWS));
    let rows = message_window(&listed, scroll);
    let tip = ctx.t("!Left Click for go to, right click for delete.");
    for (i, m) in rows.iter().enumerate() {
        let y = TOP + PITCH * i as f32;
        let key = (m.tick, m.at, m.text);
        p.button(
            Atlas::Elements3,
            [kind_icon_x(message_kind(m.text)), 344, 28, 28],
            Hover::None,
            [36.0, y],
            Some(tip.clone()),
            Action::MessageRow(key),
            true,
        );
        // UI-GAME-11.4 (E-trace-030): a message read by a click on its icon
        // shows the teal frame with the tick over the icon (`uiElements3`
        // (168, 344)) and its text in grey (the interface's disabled grey);
        // a left click on the text does nothing.
        let read = ui.read_messages.contains(&key);
        if read {
            p.sprite(Atlas::Elements3, [168, 344, 28, 28], [36.0, y]);
        }
        p.widgets.push(Widget::TextButton {
            text: message_text(ctx, m),
            font: Font::Outlined,
            at: [68.0, y + 6.0],
            colour: if read {
                [120, 120, 120]
            } else {
                [255, 255, 255]
            },
            action: Action::MessageText(key),
            enabled: true,
        });
    }
    let older = listed.len() > MESSAGE_ROWS + scroll;
    for (i, (rect, key, action, enabled)) in [
        (
            [40, 152, 24, 24],
            "!Scroll up.",
            Action::MessagesScroll(1),
            older,
        ),
        (
            [136, 152, 24, 24],
            "!Scroll down.",
            Action::MessagesScroll(-1),
            scroll > 0,
        ),
        (
            [136, 152, 24, 24],
            "!Jump to last one.",
            Action::MessagesScroll(-(scroll as i32)),
            scroll > 0,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        p.button(
            Atlas::Elements0,
            rect,
            Hover::None,
            [8.0, TOP + PITCH * i as f32],
            Some(ctx.t(key)),
            action,
            enabled,
        );
    }
    p
}

/// UI-GAME-2.5: the minimap dialog at (screenWidth - 326, 10). The frame
/// rectangle is read as 336 wide (the spec's 168 would cut the art in half;
/// the original's picture shows a dialog about 325 wide to the screen's
/// edge, E-trace-028); the map 265 x 265 at (+30, +36) of the frame (seen:
/// the spec's -30 reads as +30); the picture is the terrain with the
/// owners' colours. The borders button centred at (w - 28, 292) and the
/// close cross at (w - 17, 31), as measured in the original (E-trace-030,
/// 2560 x 1440; they supersede the spec's earlier places, Q-1058 a).
pub fn minimap(ctx: &Ctx, borders: bool) -> Panel {
    let origin = [ctx.screen[0] - 326.0, 10.0];
    let mut p = Panel::new(PanelId::Minimap, origin);
    p.sprite(Atlas::Game, [688, 320, 336, 320], [0.0, 0.0]);
    p.text(ctx.t("!Map"), Font::Big, [128.0, 21.0], WHITE);
    p.widgets.push(Widget::Picture {
        picture: crate::ui::widgets::Picture::Minimap,
        at: [30.0, 36.0],
        size: 265.0,
    });
    p.widgets.push(Widget::Button {
        atlas: Atlas::Game,
        rect: [440, 296, 48, 48],
        hover: Hover::Swap([440, 352]),
        pressed: Hover::None,
        // UI-GAME-2.5 (measured, E-trace-030): the round button centred at
        // (w - 28, 292) on the 1440 px screen: its 48 x 48 top-left at
        // (w - 52, 268), so (274, 258) from the frame at (w - 326, 10).
        at: [274.0, 258.0],
        size: None,
        tooltip: Some(ctx.t("!Show/Hide Borders.")),
        action: Action::Hud(20),
        enabled: true,
        selected: borders,
    });
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        // UI-GAME-2.5 (measured, E-trace-030): the close cross centred at
        // (w - 17, 31): its 24 x 24 top-left at (w - 29, 19), (297, 9) from
        // the frame.
        [297.0, 9.0],
        Some(ctx.t("!Close this dialog.")),
        Action::Hud(5),
        true,
    );
    p
}

/// UI-GAME-4.5: a ship's window: the ship in the porthole (a), the jump
/// buttons and the two separators (b), the name and fleet arrows (c, d), the twelve
/// cargo slots (e) and in expedition mode Cancel, Build harbour where one
/// can be founded and the six compass buttons around the porthole (f).
pub fn ship_window(ctx: &Ctx, index: usize) -> Option<Panel> {
    let ship = ctx.world.ships().get(index)?;
    let size = [362.0, 452.0];
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(&mut p, ctx, &ctx.t("!Ship"), size, Action::CloseContext);
    // UI-GAME-4.5: the ship seen through the porthole: the view behind
    // the `uiShipMask` mask, 150 x 150 at (113, 61), under the porthole
    // `uiElements5` (0, 40, 200 x 192) at (88, 40), whose glass lets it
    // through.
    p.widgets.push(Widget::Picture {
        picture: crate::ui::widgets::Picture::ShipView,
        at: SHIP_VIEW_AT,
        size: SHIP_VIEW_SIZE,
    });
    p.sized(
        Atlas::Elements5,
        [0, 40, 200, 192],
        [88.0, 40.0],
        [200.0, 192.0],
    );
    // UI-GAME-4.5: the name: its label over the text input, the arrows to
    // the previous and next ship of the fleet beside it.
    p.text(
        ctx.t("!Name of the ship"),
        Font::Heading,
        [104.0, 392.0],
        HEADING,
    );
    // UI-MENU-5.3 style 2. UI-GAME-4.5 c: what was typed stays in the input
    // until the window refreshes (an arrow click, reopening); the window
    // never reads it back, so the ship keeps its default name.
    let input = crate::ui::widgets::Input::ShipName(index);
    let name = match ctx.ship_names.get(&index) {
        Some(draft) => draft.clone(),
        None => ship_name(ctx, index),
    };
    p.text_input(
        input,
        crate::ui::widgets::INPUT_STYLE_2,
        &name,
        [104.0, 400.0],
        ctx.focus,
    );
    let fleet = crate::world::ships::fleet(ctx.world, ctx.player);
    let place = fleet.iter().position(|&i| i == index);
    for (rect, at, step, right) in [
        ([40, 152], [84.0, 400.0], -1, false),
        ([136, 152], [253.0, 400.0], 1, true),
    ] {
        // UI-GAME-4.5 d: the previous / next ship, wrapping; with one ship
        // either arrow reselects it.
        let other = place.map(|k| {
            let n = fleet.len() as isize;
            fleet[(k as isize + step).rem_euclid(n) as usize]
        });
        p.button(
            Atlas::Elements0,
            [rect[0], rect[1], 24, 24],
            Hover::None,
            at,
            None,
            other.map_or(Action::None, |ship| Action::ShipArrow { ship, right }),
            other.is_some(),
        );
    }
    // UI-GAME-4.5 (b): the two separators, the other windows' separator
    // line cut 350 wide (Q-1059).
    p.sprite(Atlas::Elements3, [0, 432, 350, 8], [2.0, 232.0]);
    p.sprite(Atlas::Elements3, [0, 432, 350, 8], [2.0, 376.0]);
    // UI-GAME-4.5 (e): the label at (16, 248), twelve slots from (16, 255),
    // six per row 56 px apart, the second row 63 px lower, each amount
    // centred at (+24, +53); no click action.
    p.text(
        ctx.t("!Goods on board"),
        Font::Heading,
        [16.0, 248.0],
        HEADING,
    );
    let mut goods: Vec<(u32, u32)> = Vec::new();
    for cargo in &ship.vessel.cargo {
        for &(g, n) in &cargo.goods {
            match goods.iter_mut().find(|x| x.0 == g) {
                Some(x) => x.1 += n,
                None => goods.push((g, n)),
            }
        }
    }
    for &(g, n) in &ship.vessel.aboard {
        match goods.iter_mut().find(|x| x.0 == g) {
            Some(x) => x.1 += n,
            None => goods.push((g, n)),
        }
    }
    for i in 0..12 {
        let at = [16.0 + 56.0 * (i % 6) as f32, 255.0 + 63.0 * (i / 6) as f32];
        let entry = goods.get(i).copied();
        slot(&mut p, ctx, at, entry.map(|(g, _)| g), None);
        if let Some((_, n)) = entry {
            p.centred_text(
                n.to_string(),
                Font::Body,
                [at[0] + 24.0, at[1] + 53.0],
                WHITE,
            );
        }
    }
    let id = index as u32;
    let player = ctx.player;
    // UI-GAME-4.5 (f): in expedition mode, Cancel at (8, 176); Build harbour
    // at (305, 176) exactly while a harbour can be founded where the ship
    // waits (SHIP-7.1; a ship a mission's script made has a last port
    // position from its creation, SHIP-2.7, so it is no exception); while
    // it waits for orders, the six compass buttons around the porthole,
    // each enabled when a landing place lies that way.
    if ship.vessel.expedition {
        p.button(
            Atlas::Elements5,
            [448, 40, 48, 48],
            Hover::Swap([448, 136]),
            [8.0, 176.0],
            Some(ctx.t("!Cancel expedition mode.")),
            Action::Command(Command::ExpeditionStop { player, ship: id }),
            true,
        );
        if ctx.world.can_found_harbour(player, index) {
            p.button(
                Atlas::Elements5,
                [352, 88, 48, 48],
                Hover::Swap([352, 184]),
                [305.0, 176.0],
                Some(ctx.t("!Construct harbor.")),
                Action::Command(Command::ExpeditionBuildHarbor { player, ship: id }),
                true,
            );
        }
        if ship.vessel.step().next.is_none() {
            for (d, (key, x, at)) in COMPASS.iter().enumerate() {
                p.widgets.push(Widget::Button {
                    atlas: Atlas::Elements5,
                    rect: [*x, 0, 40, 40],
                    hover: Hover::Swap([x + 240, 0]),
                    pressed: Hover::None,
                    at: *at,
                    size: None,
                    tooltip: Some(ctx.t(key)),
                    action: Action::Command(Command::ExpeditionGo {
                        player,
                        ship: id,
                        direction: d as u8,
                    }),
                    enabled: ship.vessel.options[d].is_some(),
                    selected: false,
                });
            }
        }
    }
    // UI-GAME-4.5 (b): Jump to port, enabled while the ship has a port, else
    // disabled with `!Ship has no target port.`; Jump to ship.
    let port = ship.vessel.port;
    p.button(
        Atlas::Elements5,
        [400, 40, 48, 48],
        Hover::Swap([400, 136]),
        [8.0, 392.0],
        Some(ctx.t(if port.is_some() {
            "!Jump to port."
        } else {
            "!Ship has no target port."
        })),
        port.map_or(Action::None, Action::Focus),
        port.is_some(),
    );
    p.button(
        Atlas::Elements5,
        [352, 40, 48, 48],
        Hover::Swap([352, 136]),
        [304.0, 392.0],
        Some(ctx.t("!Jump to ship.")),
        Action::Focus(ship.position()),
        true,
    );
    Some(p)
}

/// UI-GAME-4.5 (f): the compass buttons in direction order 0 .. 5: their
/// tooltip key, the x of their picture in `uiElements5` (row 0, 40 x 40;
/// highlighted 240 further right) and their place in the window.
pub const COMPASS: [(&str, u32, [f32; 2]); 6] = [
    ("!East", 120, [278.0, 118.0]),
    ("!SouthEast", 200, [240.0, 192.0]),
    ("!SouthWest", 160, [104.0, 192.0]),
    ("!West", 80, [67.0, 118.0]),
    ("!NorthWest", 0, [104.0, 46.0]),
    ("!NorthEast", 40, [240.0, 46.0]),
];

/// UI-GAME-4.5: where the ship view lies in the ship window, and its size.
pub const SHIP_VIEW_AT: [f32; 2] = [113.0, 61.0];
pub const SHIP_VIEW_SIZE: f32 = 150.0;

/// UI-GAME-4.5 c: a ship's name: the one typed into its window (the
/// game's), else the language text of `<Tribe>ShipName<k>`, the owner's
/// tribe and its ship-name counter at the ship's creation (the shipped
/// files have no text for these keys, so the key itself shows).
pub fn ship_name(ctx: &Ctx, index: usize) -> String {
    ship_name_in(ctx.world, ctx.texts, index)
}

/// [`ship_name`] of a world and language texts.
pub fn ship_name_in(world: &World, texts: &Texts, index: usize) -> String {
    let Some(ship) = world.ships().get(index) else {
        return String::new();
    };
    if let Some(name) = &ship.name {
        return name.clone();
    }
    let tribe = crate::render::models3d::TRIBE_NAMES[usize::from(world.tribe(ship.owner)).min(2)];
    let k = world.ship_name_index(index).unwrap_or(0);
    texts.name(&format!("{tribe}ShipName{k}")).to_owned()
}

/// UI-GAME-9.2: a productivity as the labels and tooltips show it: an
/// integer percentage, at least 0.
pub fn productivity_text(productivity: i32) -> String {
    format!("{}%", productivity.max(0))
}

/// UI-GAME-9.3: a building's tooltip: "<name> (Under construction)" for a
/// site (`under` the translated `!(Under construction)`), "<name> <p>%"
/// with a productivity, else the name.
pub fn building_tooltip(name: &str, under: Option<&str>, productivity: Option<i32>) -> String {
    match (under, productivity) {
        (Some(under), _) => format!("{name} {under}"),
        (None, Some(p)) => format!("{name} {}", productivity_text(p)),
        (None, None) => name.to_owned(),
    }
}

/// UI-GAME-9.3: what is under the pointer on the map, the first match of
/// its order (`view3d::pick_map`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapPick {
    /// An own settler (by id), road carrier (by index), or the builder or
    /// leveller of an own site.
    Settler(usize),
    Carrier(usize),
    Builder,
    Leveller,
    /// An own flag.
    Flag,
    /// A building of any owner.
    Building(Point),
}

/// UI-GAME-9.3: the tooltip of what the pointer is over: a carrier
/// `!Carrier`; a soldier his rank's name and "(health/maximum)"; another
/// settler (worker, builder, specialist, donkey) his profession's name; an
/// own flag `!Flag`; a building by [`building_tooltip`] (with a
/// productivity when it is own, finished and has one, >= 0).
pub fn map_tooltip(ctx: &Ctx, pick: MapPick) -> Option<String> {
    let profession = |good: u32| ctx.texts.name(&ctx.catalog.good_name(good)).to_owned();
    let roles = ctx.world.roles();
    match pick {
        MapPick::Carrier(i) => {
            let c = ctx
                .world
                .settler_view(s2_sim::view::SettlerRef::Carrier(i))?;
            Some(match (c.donkey, roles.donkey) {
                (true, Some(donkey)) => profession(donkey),
                _ => ctx.t("!Carrier"),
            })
        }
        MapPick::Settler(id) => {
            let s = ctx
                .world
                .settler_view(s2_sim::view::SettlerRef::Settler(id))?;
            let name = profession(s.profession);
            let health = s.health;
            Some(match (ctx.world.soldier_rank(s.profession), health) {
                (Some(rank), Some(health)) => {
                    format!("{name} ({health}/{})", ctx.world.soldier_max_health(rank))
                }
                _ => name,
            })
        }
        MapPick::Builder => roles.builder.map(profession),
        MapPick::Leveller => roles.leveller.map(profession),
        MapPick::Flag => Some(ctx.t("!Flag")),
        MapPick::Building(at) => {
            let b = ctx.world.building_at(at)?;
            let name = title_of(ctx, b.building_type);
            let under = b.under_construction.then(|| ctx.t("!(Under construction)"));
            let productivity =
                (b.owner == ctx.player && !b.under_construction && b.productivity >= 0)
                    .then_some(b.productivity);
            Some(building_tooltip(&name, under.as_deref(), productivity))
        }
    }
}

/// UI-GAME-9.3: where the map tooltip's box of `size` goes: 2 % of the
/// screen right of and above the pointer, moved left to stay on the
/// screen, at least 20 px from the top.
pub fn map_tooltip_at(cursor: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let x = (cursor[0] + 0.02 * screen[0])
        .min(screen[0] - size[0])
        .max(0.0);
    let y = (cursor[1] - 0.02 * screen[1] - size[1]).max(20.0);
    [x, y]
}

/// UI-GAME-2.2 (seen, E-trace-030): a control's tooltip box (the bar's
/// buttons, the build menu's cells and tabs, a window's buttons), 32 px
/// high, its top-left corner at the pointer + (25, -45), moved left to end
/// at the screen's right edge (and kept on the screen).
pub const CONTROL_TOOLTIP_HEIGHT: f32 = 32.0;

/// UI-GAME-2.2 (seen, E-trace-030): where a control's tooltip box of `size`
/// goes for the pointer at `cursor`.
pub fn control_tooltip_at(cursor: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let x = (cursor[0] + 25.0).min(screen[0] - size[0]).max(0.0);
    let y = (cursor[1] - 45.0).max(0.0);
    [x, y]
}

/// UI-GAME-9.3: the map tooltip's box height, 32 px of a 768-pixel-high
/// layout, scaled with the screen height.
pub fn map_tooltip_height(screen_height: f32) -> f32 {
    32.0 * screen_height / 768.0
}

/// UI-GAME-9.1: the construction view's floating labels, one per own
/// building whose field is on the screen at `on_screen`: its name in
/// light_system_outlined (1.0, 0.9, 0.78), d px above the point (d = 60 for
/// small buildings and mines, 80 medium, 120 large and harbours); for a
/// finished building with a worker its productivity at d - 20 in grey,
/// `!Production is locked!` at d - 40 while stopped, `!No worker at home.`
/// at d - 56 while its worker is out.
pub fn construction_labels(ctx: &Ctx, on_screen: &[(Point, [f32; 2])]) -> Panel {
    use s2_sim::BuildingSize as S;
    const NAME: [u8; 3] = [255, 230, 199];
    const GREY: [u8; 3] = [230, 230, 230];
    let mut p = Panel::new(PanelId::Labels, [0.0, 0.0]);
    for &(at, [x, y]) in on_screen {
        let Some(b) = ctx.world.building_at(at) else {
            continue;
        };
        if b.owner != ctx.player {
            continue;
        }
        let kind = ctx.world.building_type(b.building_type);
        let d = match kind.map(|k| k.size) {
            Some(S::Medium) => 80.0,
            Some(S::Large | S::Harbor) => 120.0,
            _ => 60.0,
        };
        p.centred_text(
            title_of(ctx, b.building_type),
            Font::Outlined,
            [x, y - d],
            NAME,
        );
        let has_worker = kind.is_some_and(|k| k.worker.is_some());
        if b.under_construction || !has_worker || ctx.world.warehouse(at).is_some() {
            continue;
        }
        p.centred_text(
            productivity_text(b.productivity),
            Font::Outlined,
            [x, y - d + 20.0],
            GREY,
        );
        if !b.production.enabled {
            p.centred_text(
                ctx.t("!Production is locked!"),
                Font::Outlined,
                [x, y - d + 40.0],
                GREY,
            );
        }
        let worker_home = ctx.world.settler_views().iter().any(|v| {
            matches!(v.activity, s2_sim::view::Activity::Worker { building, .. } if building == at)
                && v.hidden
        });
        if !worker_home {
            p.centred_text(
                ctx.t("!No worker at home."),
                Font::Outlined,
                [x, y - d + 56.0],
                GREY,
            );
        }
    }
    p
}

/// UI-GAME-11: the game screen's frame from `uiframe.dds`: the four edge
/// bars 12 px thick along the whole edge, drawn from the strip's first 4
/// texels (11.3), then the four corners at their art size, which cover
/// the bars (11.2).
pub fn screen_frame(screen: [f32; 2]) -> Panel {
    let [w, h] = screen;
    let mut p = Panel::new(PanelId::Frame, [0.0, 0.0]);
    const STRIP: Rect = [0, 83, 4, 12];
    for (at, size) in [
        ([0.0, 0.0], [w, 12.0]),
        ([0.0, h - 12.0], [w, 12.0]),
        ([0.0, 0.0], [12.0, h]),
        ([w - 12.0, 0.0], [12.0, h]),
    ] {
        p.sized(Atlas::Frame, STRIP, at, size);
    }
    p.sprite(Atlas::Frame, [0, 0, 80, 80], [0.0, 0.0]);
    p.sprite(Atlas::Frame, [176, 0, 80, 80], [w - 80.0, 0.0]);
    p.sprite(Atlas::Frame, [0, 112, 120, 144], [0.0, h - 144.0]);
    p.sprite(Atlas::Frame, [136, 112, 120, 144], [w - 120.0, h - 144.0]);
    p
}

/// UI-GAME-1.11: the paused view's layer's alpha, the byte 165 of the
/// colour (1, 1, 1, 0.65) packed by truncation.
pub const PAUSE_ALPHA: f32 = 165.0 / 255.0;

/// UI-GAME-1.11 (T46): the paused view: a layer over the whole screen, the
/// bar included: the image `uiTip` (every texel opaque black) with the
/// colour (1, 1, 1, 0.65), its alpha packed into the byte 165 (0.65 x 255
/// truncated), so every pixel keeps 90 / 255 of its stored value (the
/// interface blends on stored values, GFX-VIEW-2.6 c); then `!Pause` in
/// `light_system` centred on the screen.
pub fn pause_layer(ctx: &Ctx) -> Panel {
    let [w, h] = ctx.screen;
    let mut p = Panel::new(PanelId::Pause, [0.0, 0.0]);
    p.widgets.push(Widget::Fill {
        at: [0.0, 0.0],
        size: [w, h],
        rgba: [0.0, 0.0, 0.0, PAUSE_ALPHA],
    });
    p.widgets.push(Widget::Text {
        text: ctx.t("!Pause"),
        font: Font::Body,
        at: [w / 2.0, h / 2.0],
        colour: WHITE,
        align: crate::ui::widgets::Align::Centre,
    });
    p
}

/// UI-GAME-1.4: a refused action's message, in red, for 5 seconds. The
/// error box's art and place are not in the spec (Q-199 c): a dark box
/// centred above the bar.
pub fn error(ctx: &Ctx, text: &str) -> Panel {
    let bar = bar_origin(ctx.screen);
    let width = 640.0;
    let mut p = Panel::new(
        PanelId::Error,
        [(ctx.screen[0] - width) / 2.0, bar[1] - 44.0],
    );
    p.widgets.push(Widget::Fill {
        at: [0.0, 0.0],
        size: [width, 30.0],
        rgba: [0.0, 0.0, 0.0, 0.75],
    });
    p.centred_text(text, Font::Outlined, [width / 2.0, 15.0], [255, 70, 70]);
    p
}

/// UI-GAME-2.7: above speed 1, a settler running with a clock, 50 x 50 at
/// (screenWidth - 76, 20), or (screenWidth - 382, 20) while the minimap is
/// open: `uiGame` row 640 (clock face 2) or 704 (3), six 64 x 64 frames at
/// x = 64 (t mod 6), t the second clock in whole seconds: real time, so it
/// runs on while paused (TOOL-1.3, TOOL-trace TRACE-5.3; UI-GAME-2.7
/// aligned, Q-231 a answered).
pub fn speed_indicator(ctx: &Ctx, minimap: bool) -> Option<Panel> {
    let row = match ctx.speed {
        2 => 640,
        3 => 704,
        _ => return None,
    };
    let x = if minimap { 382.0 } else { 76.0 };
    let mut p = Panel::new(PanelId::Hud2, [ctx.screen[0] - x, 20.0]);
    let seconds = ctx.real_seconds.max(0.0) as u64;
    let frame = 64 * (seconds % 6) as u32;
    p.sized(Atlas::Game, [frame, row, 64, 64], [0.0, 0.0], [50.0, 50.0]);
    Some(p)
}
