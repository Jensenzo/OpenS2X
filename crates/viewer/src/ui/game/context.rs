//! The context dialogs on the map: flag menu, build menu, road menu,
//! deploy-flag menu (UI-GAME-3).

use super::*;

/// UI-GAME-3.5: an icon tab (40 x 27) of `uiElements0`.
pub(crate) fn tab(
    p: &mut Panel,
    id: u32,
    at: [f32; 2],
    tooltip: String,
    action: Action,
    selected: bool,
) {
    tab_enabled(p, id, at, tooltip, action, selected, true);
}

pub(crate) fn tab_enabled(
    p: &mut Panel,
    id: u32,
    at: [f32; 2],
    tooltip: String,
    action: Action,
    selected: bool,
    enabled: bool,
) {
    let x = 8 + 40 * id;
    p.widgets.push(Widget::Button {
        atlas: Atlas::Elements0,
        rect: [x, 416, 40, 27],
        hover: Hover::Swap([x, 448]),
        pressed: Hover::None,
        at,
        size: None,
        tooltip: Some(tooltip),
        action,
        enabled,
        selected: false,
    });
    if selected {
        p.sprite(Atlas::Elements0, [x, 480, 40, 27], at);
    }
}

/// UI-GAME-3.1: the flag menu: five round portrait buttons of
/// `uiElements3`, the picture at (x, 144), highlighted at (x, 192), in a
/// row 52 px apart from (15, 29), on the small panel (1.8, T29). The
/// scholar and explorer buttons call a geologist or scout to the flag
/// (ECO-SPC-1.1).
pub fn flag_menu(ctx: &Ctx, flag: Point) -> Panel {
    let rows = flag_menu_rows(ctx, flag);
    let size = SMALL_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &ctx.t("!Flag Menu"),
        size,
        Action::CloseContext,
    );
    portrait_row(&mut p, ctx, rows);
    p
}

/// A portrait button: its picture's top-left corner, the highlighted
/// picture's, its tooltip key, action and whether it is enabled. The
/// pictures are `uiElements3`'s, but the pave button's (W34).
pub(crate) type Portrait = ([u32; 2], [u32; 2], &'static str, Action, bool);

/// UI-GAME-3.1: the flag menu's buttons, the smart cursor's first.
pub(crate) fn flag_menu_rows(ctx: &Ctx, flag: Point) -> Vec<Portrait> {
    let is_hq_flag = ctx.world.warehouses().any(|(at, _)| {
        flag_of(ctx.world, at) == flag
            && ctx
                .world
                .building_at(at)
                .is_some_and(|b| b.owner == ctx.player)
            && is_headquarters(ctx.world, at)
    });
    vec![
        (
            [0, 144],
            [0, 192],
            "!Create a street.",
            Action::StartRoad(flag),
            true,
        ),
        (
            [48, 144],
            [48, 192],
            "!Create a Waterway.",
            Action::StartWaterway(flag),
            // TRN-ROAD-7.1: only at a flag on a Coast field.
            ctx.world.terrain().has(flag, s2_sim::map::state::COAST),
        ),
        (
            [96, 144],
            [96, 192],
            "!Destroy the flag.",
            Action::Command(Command::RemoveFlag {
                player: ctx.player,
                at: flag,
            }),
            !is_hq_flag,
        ),
        (
            [144, 144],
            [144, 192],
            "!Call a scholar!",
            Action::Command(Command::OrderGeologist {
                player: ctx.player,
                at: flag,
            }),
            true,
        ),
        (
            [192, 144],
            [192, 192],
            "!Call the explorer!",
            Action::Command(Command::OrderScout {
                player: ctx.player,
                at: flag,
            }),
            true,
        ),
    ]
}

/// UI-GAME-3.1: the first portrait's top-left corner and the step.
pub(crate) const PORTRAIT_FIRST: [f32; 2] = [15.0, 29.0];
pub(crate) const PORTRAIT_STEP: f32 = 52.0;

pub(crate) fn portrait_row(p: &mut Panel, ctx: &Ctx, rows: Vec<Portrait>) {
    for (i, (picture, lit, key, action, enabled)) in rows.into_iter().enumerate() {
        // W34: the pave button may show a project's picture.
        let (atlas, picture, hover) = match action {
            Action::Command(Command::PaveRoad { .. }) => {
                let pave = crate::extend::button_pictures().and_then(|b| b.pave.as_ref());
                let (atlas, at, hover) =
                    crate::extend::pave_cell(pave, crate::extend::project_pictures());
                (atlas, at, hover.map_or(Hover::None, Hover::Swap))
            }
            _ => (Atlas::Elements3, picture, Hover::Swap(lit)),
        };
        p.button(
            atlas,
            [picture[0], picture[1], 48, 48],
            hover,
            [
                PORTRAIT_FIRST[0] + PORTRAIT_STEP * i as f32,
                PORTRAIT_FIRST[1],
            ],
            Some(ctx.t(key)),
            action,
            enabled,
        );
    }
}

/// The flag field of a building: its direction-1 neighbour (ECO-PLACE-3).
pub fn flag_of(world: &World, building: Point) -> Point {
    world.terrain().neighbour(building, 1).unwrap_or(building)
}

/// The headquarters: the player's first warehouse with a soldier reserve.
pub(crate) fn is_headquarters(world: &World, at: Point) -> bool {
    world.military_at(at).is_some() && world.warehouse(at).is_some()
}

/// UI-GAME-3.2: which size tabs a site shows.
pub fn build_tabs(class: Placement) -> &'static [u8] {
    match class {
        Placement::Small => &[5],
        Placement::Medium => &[5, 6],
        Placement::Large | Placement::Harbor => &[5, 6, 7],
        _ => &[],
    }
}

/// UI-GAME-3.2: the flag button, the build menu's first cell: the slot
/// `uiElements3` (0, 0, 80 x 80) with the flag icon `uiBuildings0` (0, 0,
/// 80 x 80) on it, enabled while a flag fits on the field (tooltip
/// `!Create flag at position.`), else greyed with `!Flag too close to
/// another flag.`.
pub(crate) fn flag_button(p: &mut Panel, ctx: &Ctx, field: Point, at: [f32; 2]) {
    let fits = ctx.world.check_flag(ctx.player, field).is_ok();
    p.widgets.push(Widget::Button {
        atlas: Atlas::Elements3,
        rect: BUILD_SLOT,
        hover: Hover::None,
        pressed: Hover::None,
        at,
        size: None,
        tooltip: Some(ctx.t(if fits {
            "!Create flag at position."
        } else {
            "!Flag too close to another flag."
        })),
        action: Action::Command(Command::PlaceFlag {
            player: ctx.player,
            at: field,
        }),
        enabled: fits,
        selected: false,
    });
    p.icon(Atlas::Buildings(0), FLAG_ICON, at, [80.0, 80.0], !fits);
}

/// UI-GAME-3.2: the cell slot of the build menu, `uiElements3` (0, 0, 80 x 80).
pub(crate) const BUILD_SLOT: Rect = [0, 0, 80, 80];

/// UI-GAME-3.2: the flag icon, `uiBuildings0` (0, 0, 80 x 80).
pub(crate) const FLAG_ICON: Rect = [0, 0, 80, 80];

/// UI-GAME-3.2: the free field menu: size tabs, the grid of building types
/// of the chosen size (four 80 x 80 cells a row) in the order of their sort
/// index, military types among the others and never the headquarters, the
/// flag button first. A field that takes only a flag opens the road menu
/// instead (3.3).
/// UI-GAME-3.2 (seen, E-trace-030): a build menu cell's tooltip.
pub(crate) fn build_cell_tooltip(name: &str, count: usize) -> String {
    format!("{name} ({count})")
}

pub fn build_menu(ctx: &Ctx, field: Point, tab: u8) -> Panel {
    let class = ctx.world.placement_at(field);
    let tabs = build_tabs(class);
    let tab = if tabs.contains(&tab) {
        tab
    } else {
        tabs.first().copied().unwrap_or(0)
    };
    let listed = build_page(ctx.catalog, class, tab);
    // The big panel at its full size, as the photo build-menu-flag.jpg
    // shows it (UI-GAME-1.8; room for five rows of four).
    let size = BIG_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &ctx.t("!Build Building or Flag"),
        size,
        Action::CloseContext,
    );
    let tips = [
        (5, "!Show small buildings"),
        (6, "!Show medium buildings"),
        (7, "!Show large buildings"),
    ];
    for (i, &t) in tabs.iter().enumerate() {
        let tip = tips.iter().find(|(id, _)| *id == t).map_or("", |(_, k)| k);
        tab_button(&mut p, ctx, t, i, tip, Action::BuildTab(t), t == tab);
    }
    let cell = |i: usize| [23.0 + 80.0 * (i % 4) as f32, 60.0 + 80.0 * (i / 4) as f32];
    flag_button(&mut p, ctx, field, cell(0));
    for (i, b) in listed.iter().enumerate() {
        let at = cell(i + 1);
        // UI-GAME-3.2, ECO-PLACE-6.3: a military type (one with a
        // territory range) within 4 fields of a military building or site
        // is greyed first, then a type the map forbids (CAM-1.3).
        let refused = if ctx.world.counts_as_military(b.id) && ctx.world.military_near(field) {
            Some("!Too close to other military buildings.")
        } else if ctx.world.is_locked(ctx.player, b.id) {
            Some("!Building not allowed")
        } else {
            None
        };
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements3,
            rect: BUILD_SLOT,
            hover: Hover::None,
            pressed: Hover::None,
            at,
            size: None,
            // UI-GAME-3.2 (seen, E-trace-030): the type's name and, in
            // brackets, how many the player has of it ("Woodcutter's Hut
            // (0)"; the count's meaning taken, medium: sites counted too);
            // a greyed cell's reason in its place (E-trace-031).
            tooltip: Some(match refused {
                Some(key) => ctx.t(key),
                None => build_cell_tooltip(
                    ctx.texts.name(&b.name),
                    ctx.world
                        .buildings()
                        .filter(|x| x.owner == ctx.player && x.building_type == b.id)
                        .count(),
                ),
            }),
            action: Action::Command(Command::PlaceBuilding {
                player: ctx.player,
                building: b.id,
                at: field,
            }),
            enabled: refused.is_none(),
            selected: false,
        });
        match b.icon {
            Some((u, v)) => p.icon(
                Atlas::Buildings(ctx.tribe()),
                [80 * u, 80 * v, 80, 80],
                at,
                [80.0, 80.0],
                refused.is_some(),
            ),
            None => p.sized(
                Atlas::Elements3,
                [440, 216, 48, 48],
                [at[0] + 2.0, at[1] + 2.0],
                [76.0, 76.0],
            ),
        }
    }
    p
}

/// UI-GAME-3.2: the building types of the build menu's page `tab` on a
/// field of `class`, after the flag: the types of the page's size (the
/// harbour too on a harbour site's large page; the mine page on a mine
/// site), in the catalog's order, never the headquarters.
pub(crate) fn build_page(catalog: &Catalog, class: Placement, tab: u8) -> Vec<&BuildingEntry> {
    let size_of_tab = |t: u8| match t {
        5 => BuildingSize::Small,
        6 => BuildingSize::Medium,
        _ => BuildingSize::Large,
    };
    catalog
        .buildings
        .iter()
        .filter(|b| !b.headquarters)
        .filter(|b| match class {
            Placement::Mine => b.size == BuildingSize::Mine,
            Placement::Harbor if tab == 7 => {
                b.size == BuildingSize::Large || b.size == BuildingSize::Harbor
            }
            Placement::Nothing | Placement::Flag => false,
            _ => b.size == size_of_tab(tab),
        })
        .collect()
}

pub(crate) fn tab_button(
    p: &mut Panel,
    ctx: &Ctx,
    id: u8,
    i: usize,
    key: &str,
    action: Action,
    on: bool,
) {
    tab(
        p,
        u32::from(id),
        [15.0 + 40.0 * i as f32, 25.0],
        ctx.t(key),
        action,
        on,
    );
}

/// UI-GAME-3.3: the road menu (`!Street Menu`), the flag menu's layout
/// with up to two portraits: a flag where one fits, and the road destroyed
/// (on a road). A free field that takes only a flag opens it too, with the
/// flag button alone (T23).
pub fn road_menu(ctx: &Ctx, field: Point) -> Panel {
    let rows = road_menu_rows(ctx, field);
    // UI-GAME-1.8: the small panel, whatever the button count.
    let size = SMALL_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &ctx.t("!Street Menu"),
        size,
        Action::CloseContext,
    );
    portrait_row(&mut p, ctx, rows);
    p
}

/// UI-GAME-3.6: the deploy-flag menu (`!Deploy Flag Menu`), opened by a
/// click on the end of the road being built: `!Create Flag.` where a flag
/// fits (`place-flag`; the logic finishes the road the next tick), then
/// `!Cancel street construction.` (`cancel-road`), on the
/// small panel. Outside road building (CAM-5.3c's step 0x32 opens it) only
/// `!Create Flag.`.
pub fn deploy_flag_menu(ctx: &Ctx, field: Point) -> Panel {
    let mut rows = Vec::new();
    if ctx.world.check_flag(ctx.player, field).is_ok() {
        rows.push((
            [368, 160],
            [416, 160],
            "!Create Flag.",
            Action::Command(Command::PlaceFlag {
                player: ctx.player,
                at: field,
            }),
            true,
        ));
    }
    if ctx.world.route(ctx.player).is_some() {
        rows.push((
            [240, 144],
            [240, 192],
            "!Cancel street construction.",
            Action::Command(Command::CancelRoute { player: ctx.player }),
            true,
        ));
    }
    let size = SMALL_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &ctx.t("!Deploy Flag Menu"),
        size,
        Action::CloseContext,
    );
    portrait_row(&mut p, ctx, rows);
    p
}

pub(crate) fn road_menu_rows(ctx: &Ctx, field: Point) -> Vec<Portrait> {
    let mut rows = Vec::new();
    if ctx.world.check_flag(ctx.player, field).is_ok() {
        rows.push((
            [368, 160],
            [416, 160],
            "!Deploy flag.",
            Action::Command(Command::PlaceFlag {
                player: ctx.player,
                at: field,
            }),
            true,
        ));
    }
    if ctx.world.road_at(field).is_some() {
        rows.push((
            [240, 144],
            [240, 192],
            "!Destroy the street.",
            Action::Command(Command::RemoveRoad {
                player: ctx.player,
                at: field,
            }),
            true,
        ));
    }
    // Hook W19 (a project's, docs/extending.md "Paved roads"): with a paving
    // rule, the road menu has a pave button (the flag-with-hammer picture,
    // or a project's, W34: `portrait_row`), usable while the command would
    // be accepted.
    if ctx.world.paving_rule().is_some()
        && ctx
            .world
            .road_at(field)
            .is_some_and(|r| r.owner == ctx.player && !r.water)
    {
        rows.push((
            [368, 160],
            [416, 160],
            "Pave the road.",
            Action::Command(Command::PaveRoad {
                player: ctx.player,
                at: field,
            }),
            ctx.world.can_pave(ctx.player, field),
        ));
    }
    rows
}
