//! The building windows: production, construction site, military,
//! warehouse, and the attack dialog (UI-GAME-4, 5).

use super::*;

/// The button row of the building windows (`uiElements5`, 48 x 48): the
/// `row`-th button from `origin` leftward in 50 px steps (UI-GAME-4.1a d).
#[allow(clippy::too_many_arguments)]
pub(crate) fn column_button(
    p: &mut Panel,
    ctx: &Ctx,
    origin: [f32; 2],
    row: usize,
    sprite: [u32; 2],
    hover: Option<[u32; 2]>,
    key: &str,
    action: Action,
    enabled: bool,
) {
    p.button(
        Atlas::Elements5,
        [sprite[0], sprite[1], 48, 48],
        hover.map_or(Hover::None, Hover::Swap),
        [origin[0] - 50.0 * row as f32, origin[1]],
        Some(ctx.t(key)),
        action,
        enabled,
    );
}

/// A 48 x 48 slot frame with a good's icon and a count under it.
pub(crate) fn slot(p: &mut Panel, ctx: &Ctx, at: [f32; 2], good: Option<u32>, count: Option<u32>) {
    p.sprite(Atlas::Elements3, [440, 216, 48, 48], at);
    if let Some(good) = good {
        good_icon(p, ctx, good, at, 48.0);
    }
    if let Some(n) = count {
        p.text(
            n.to_string(),
            Font::Body,
            [at[0] + 30.0, at[1] + 30.0],
            WHITE,
        );
    }
}

/// A gauge (0..1) of `size`: its art is not named (Q-184): two bars.
pub(crate) fn gauge_box(p: &mut Panel, at: [f32; 2], size: [f32; 2], value: f32) {
    p.widgets.push(Widget::Fill {
        at,
        size,
        rgba: [0.05, 0.12, 0.12, 0.9],
    });
    p.widgets.push(Widget::Fill {
        at: [at[0] + 1.0, at[1] + 1.0],
        size: [(size[0] - 2.0) * value.clamp(0.0, 1.0), size[1] - 2.0],
        rgba: [0.25, 0.85, 0.75, 1.0],
    });
}

/// UI-GAME-4: the window of an own building, by its kind.
pub fn building_window(ctx: &Ctx, at: Point, tab: u8) -> Option<Panel> {
    let b = ctx.world.building_at(at)?;
    if b.under_construction {
        return Some(site_window(ctx, at));
    }
    if ctx.world.warehouse(at).is_some() {
        return Some(depot_window(ctx, at, tab));
    }
    if ctx.world.military_at(at).is_some() {
        return Some(military_window(ctx, at));
    }
    Some(production_window(ctx, at))
}

pub(crate) fn title_of(ctx: &Ctx, building: u32) -> String {
    let name = ctx
        .catalog
        .building_names
        .get(&building)
        .cloned()
        .unwrap_or_default();
    ctx.texts.name(&name).to_owned()
}

/// The portrait frame with the building (its icon stands in for the
/// model, Q-184).
pub(crate) fn portrait(p: &mut Panel, ctx: &Ctx, building: u32) {
    p.sprite(Atlas::Elements3, [344, 216, 88, 88], [22.0, 43.0]);
    building_icon(p, ctx, building, [26.0, 47.0], 80.0);
}

/// UI-GAME-4.1: a production building's window.
pub fn production_window(ctx: &Ctx, at: Point) -> Panel {
    let world = ctx.world;
    let b = world.building_at(at).expect("a building");
    let kind = world.building_type(b.building_type);
    // UI-GAME-4.1a (a): the frame picture `uiMediumDialog`, 480 x 224.
    let size = MEDIUM_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &title_of(ctx, b.building_type),
        size,
        Action::CloseContext,
    );
    // UI-GAME-4.1a (g) (as the original shows it, E-trace-029): the
    // catapult's window has the BUILDING caption and the portrait, both
    // separators, no PRODUCT, TOOL, NEEDED GOODS or PRODUCTIVITY, and one
    // slot, its stone, at 395 (no empty frames).
    let catapult = kind.is_some_and(|k| k.catapult);
    let inputs = if catapult {
        ctx.catalog.stone.into_iter().collect()
    } else {
        ctx.catalog
            .inputs
            .get(&b.building_type)
            .cloned()
            .unwrap_or_default()
    };
    p.centred_text(ctx.t("!BUILDING"), Font::Heading, [64.0, 37.0], HEADING);
    if !inputs.is_empty() && !catapult {
        p.centred_text(
            ctx.t("!NEEDED GOODS "),
            Font::Heading,
            [364.0, 37.0],
            HEADING,
        );
    }
    let tool = ctx.catalog.tools_of.get(&b.building_type).copied();
    if !catapult {
        p.text(
            ctx.t("!PRODUCTIVITY"),
            Font::Heading,
            [136.0, 106.0],
            HEADING,
        );
        // UI-GAME-4.1a (e), (g): the gauge is the site window's (4.2), from
        // (137, 112) to (443, 128), its value `100%` centred in it (the font
        // is not given: the body text's).
        let value = b.productivity as f32 / 100.0;
        gauge_box(&mut p, [137.0, 112.0], [306.0, 16.0], value);
        p.centred_text(
            format!("{}%", b.productivity),
            Font::Body,
            [137.0 + 153.0, 120.0],
            WHITE,
        );
        // UI-GAME-4.1a (f) (E-trace-028): a type whose worker needs no tool
        // shows no TOOL caption, slot or separator after it (the well).
        if tool.is_some() {
            p.centred_text(ctx.t("!TOOL"), Font::Heading, [232.0, 37.0], HEADING);
            slot(&mut p, ctx, [208.0, 43.0], tool, None);
        }
        p.centred_text(ctx.t("!PRODUCT"), Font::Heading, [160.0, 37.0], HEADING);
        // UI-GAME-4.1a (g): the shipyard shows the ship while it builds
        // ships (switched, ECO-PROD-1.3), else its product, the boat.
        let ships = kind.is_some_and(|k| k.switchable) && b.production.switched;
        let product = if ships {
            ctx.catalog.ship
        } else {
            ctx.catalog.products.get(&b.building_type).copied()
        };
        slot(&mut p, ctx, [136.0, 43.0], product, None);
    }
    portrait(&mut p, ctx, b.building_type);
    p.sprite(Atlas::Elements3, [0, 432, 456, 8], [2.0, 143.0]);
    p.sprite(Atlas::Elements3, [160, 0, 8, 56], [192.0, 32.0]);
    if tool.is_some() || catapult {
        p.sprite(Atlas::Elements3, [160, 0, 8, 56], [264.0, 32.0]);
    }
    // Input slots from (395, 43) leftwards; six for the mint.
    // UI-GAME-4.1a (e): the slot frames stand also without inputs (three
    // empty ones; six for the mint); the catapult's only its stone's.
    let slots = if catapult {
        inputs.len()
    } else if inputs.len() > 3 {
        6
    } else {
        3
    };
    // UI-GAME-4.1a (f) (E-trace-028): an input's count is `<stock>/<capacity>`
    // centred under its slot on y 100 (`0/6` for a sawmill's wood); the
    // capacity is what the building orders up to (ECO-PROD-6.1: a mine's
    // foods 2 each, as the original's coal mine shows them, E-trace-029).
    for i in 0..slots {
        let good = inputs.get(i).copied();
        let x = 395.0 - 60.0 * i as f32;
        slot(&mut p, ctx, [x, 43.0], good, None);
        if let Some(good) = good {
            let have: u32 = world
                .inputs(at)
                .iter()
                .filter(|&&(g, _)| g == good)
                .map(|&(_, n)| n)
                .sum();
            let count = match world.input_limit(at, good) {
                Some(c) => format!("{have}/{c}"),
                None => have.to_string(),
            };
            p.centred_text(count, Font::Body, [x + 24.0, 100.0], WHITE);
        }
    }
    // UI-GAME-4.1a (c): two status lines in `light_system`, left edges at
    // x 40, line tops at y 160 and 182; no worker activity. Line 1 while
    // production is off; line 2 while no worker is inside, for a type with
    // a product, the forester and the lookout tower.
    if !b.production.enabled {
        p.text(
            ctx.t(if catapult {
                "!Bombardment stopped."
            } else {
                "!Production stopped."
            }),
            Font::Body,
            [40.0, 160.0],
            WHITE,
        );
    }
    let worker_home = world.settler_views().iter().any(|v| {
        matches!(v.activity, s2_sim::view::Activity::Worker { building, .. } if building == at)
            && v.hidden
    });
    let shows_home = kind.is_some_and(|k| {
        k.worker.is_some()
            && (k.product.is_some()
                || k.cycle == s2_sim::WorkCycle::Forester
                || (k.explore_radius > 0 && k.soldier_limit == 0 && !k.warehouse))
    });
    if shows_home && !worker_home {
        p.text(
            ctx.t("!No worker at home."),
            Font::Body,
            [40.0, 182.0],
            WHITE,
        );
    }
    let column = [407.0, 157.0];
    column_button(
        &mut p,
        ctx,
        column,
        0,
        [448, 88],
        Some([448, 184]),
        "!Destroy the building.",
        Action::Command(Command::RemoveBuilding {
            player: ctx.player,
            at,
        }),
        true,
    );
    column_button(
        &mut p,
        ctx,
        column,
        1,
        [208, 88],
        None,
        if catapult {
            "!Toggle bombardment ON/OFF."
        } else {
            "!Toggle production ON/OFF."
        },
        Action::Command(Command::ProductionToggle {
            player: ctx.player,
            at,
        }),
        kind.is_some_and(|k| k.worker.is_some()),
    );
    if kind.is_some_and(|k| k.switchable) {
        column_button(
            &mut p,
            ctx,
            column,
            2,
            [256, 88],
            None,
            "!Toggle between boat and ship.",
            Action::Command(Command::ProductionSwitch {
                player: ctx.player,
                at,
            }),
            true,
        );
    }
    p
}

/// UI-GAME-4.2: a construction site's window.
pub fn site_window(ctx: &Ctx, at: Point) -> Panel {
    let world = ctx.world;
    let b = world.building_at(at).expect("a building");
    // UI-GAME-1.8 (F59): the site window's frame picture `uiMediumDialog`,
    // 480 x 224.
    let size = MEDIUM_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &title_of(ctx, b.building_type),
        size,
        Action::CloseContext,
    );
    p.centred_text(ctx.t("!BUILDING"), Font::Heading, [64.0, 37.0], HEADING);
    p.centred_text(
        ctx.t("!NEEDED GOODS "),
        Font::Heading,
        [364.0, 37.0],
        HEADING,
    );
    portrait(&mut p, ctx, b.building_type);
    if let Some(site) = world.site(at) {
        // UI-GAME-4.2 (E-trace-028): the completion gauge from (137, 112) to
        // (443, 128) with its percent centred in it; status line 1 reads
        // `!Construction in progress...`.
        let done = site.progress_permille(world.bulldoze_ticks()) as f32 / 1000.0;
        gauge_box(&mut p, [137.0, 112.0], [306.0, 16.0], done);
        p.centred_text(
            format!("{}%", (done * 100.0).floor() as u32),
            Font::Body,
            [137.0 + 153.0, 120.0],
            WHITE,
        );
        p.text(
            ctx.t("!Construction in progress..."),
            Font::Body,
            [40.0, 160.0],
            WHITE,
        );
        let mut goods: Vec<(u32, u32, u32)> = Vec::new();
        for &(good, n) in site.frame_costs.iter().chain(&site.costs) {
            match goods.iter_mut().find(|g| g.0 == good) {
                Some(g) => g.1 += n,
                None => goods.push((good, n, 0)),
            }
        }
        for g in goods.iter_mut() {
            g.2 = site
                .delivered
                .iter()
                .filter(|(d, _)| *d == g.0)
                .map(|(_, n)| n)
                .sum();
        }
        // UI-GAME-4.2 (E-trace-028): one slot per good still to come, the
        // first at (380, 42), the number still needed as `x<n>` centred on
        // (447, 73) right of it; further goods leftwards, 90 px apart (ours:
        // only one good was seen).
        let to_come: Vec<(u32, u32)> = goods
            .iter()
            .filter(|(_, need, have)| need > have)
            .map(|&(good, need, have)| (good, need - have))
            .collect();
        for (i, (good, left)) in to_come.iter().take(3).enumerate() {
            let x = 380.0 - 90.0 * i as f32;
            slot(&mut p, ctx, [x, 42.0], Some(*good), None);
            p.centred_text(format!("x{left}"), Font::Body, [x + 67.0, 73.0], WHITE);
        }
    }
    column_button(
        &mut p,
        ctx,
        [407.0, 157.0],
        0,
        [448, 88],
        Some([448, 184]),
        "!Destroy the Construction.",
        Action::Command(Command::RemoveBuilding {
            player: ctx.player,
            at,
        }),
        true,
    );
    p
}

/// UI-GAME-4.1 to 4.3: the headings' x is their centre (BUILDING 64 over
/// the portrait at 22 + 44, PRODUCT 160 and TOOL 232 over their slots,
/// COINS 392 over the coin slot; play-test 4: drawn from the left they ran
/// into each other, Q-208).
///
/// UI-GAME-4.3: a military building's window. The upgrade button's
/// refusal tooltips need the map's allowed types and the larger site's
/// test, which the simulation answers only when the command runs: the
/// button is enabled while the type can be upgraded at all.
pub fn military_window(ctx: &Ctx, at: Point) -> Panel {
    let world = ctx.world;
    let b = world.building_at(at).expect("a building");
    let kind = world.building_type(b.building_type);
    let m = world.military_at(at).expect("military");
    // UI-GAME-4.3 (F59, E-ui-033): the frame picture `uiMedium2Dialog`,
    // 480 x 272.
    let size = MEDIUM2_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &title_of(ctx, b.building_type),
        size,
        Action::CloseContext,
    );
    p.centred_text(ctx.t("!COINS"), Font::Heading, [392.0, 37.0], HEADING);
    p.centred_text(
        ctx.t("!SOLDIERS INSIDE"),
        Font::Heading,
        [210.0, 37.0],
        HEADING,
    );
    p.centred_text(ctx.t("!BUILDING"), Font::Heading, [64.0, 37.0], HEADING);
    portrait(&mut p, ctx, b.building_type);
    // The two separators, 8 x 176.
    for x in [123.0, 300.0] {
        p.sized(Atlas::Elements3, [160, 0, 8, 56], [x, 32.0], [8.0, 176.0]);
    }
    let coins = ctx.catalog.coin.map_or(0, |coin| {
        world
            .inputs(at)
            .iter()
            .filter(|&&(g, _)| g == coin)
            .map(|&(_, n)| n)
            .sum()
    });
    slot(&mut p, ctx, [369.0, 45.0], ctx.catalog.coin, None);
    // UI-GAME-4.3 (F59, E-ui-033): the coin stock, a `light_system` number
    // centred on x 392, its line top at y 98.
    p.centred_text(coins.to_string(), Font::Body, [392.0, 98.0], WHITE);
    let places = kind.map_or(0, |k| k.soldier_limit) as usize;
    for i in 0..places {
        let at_slot = [137.0 + 56.0 * (i % 3) as f32, 45.0 + 56.0 * (i / 3) as f32];
        p.sprite(Atlas::Elements3, [440, 216, 48, 48], at_slot);
        if let Some(&soldier) = m.soldiers.get(i) {
            if let Some((_, s)) = world.settlers().find(|(id, _)| *id == soldier) {
                good_icon(&mut p, ctx, s.profession, at_slot, 48.0);
            }
        }
    }
    // The buttons in a row at y 212, 50 px apart from x 407 leftward.
    let upgrading = world.upgrading(at).is_some();
    let upgrade = kind.and_then(|k| k.upgrade_to);
    let buttons = [
        (
            [448, 88],
            Some([448, 184]),
            "!Destroy the building.",
            Command::RemoveBuilding {
                player: ctx.player,
                at,
            },
            true,
        ),
        (
            [304, 88],
            None,
            "!Toggle coin supply ON/OFF.",
            Command::ToggleCoinSupply {
                player: ctx.player,
                at,
            },
            true,
        ),
        (
            [208, 232],
            None,
            "!Toggle Moving soldiers out.",
            Command::ToggleMoveOut {
                player: ctx.player,
                at,
            },
            true,
        ),
        (
            [256, 232],
            None,
            if upgrading {
                "!Upgrade Running"
            } else if upgrade.is_some() {
                "!Upgrade Building"
            } else {
                "!Upgrade not possible"
            },
            Command::UpgradeBuilding {
                player: ctx.player,
                at,
            },
            upgrade.is_some() && !upgrading,
        ),
    ];
    for (i, (sprite, hover, key, command, enabled)) in buttons.into_iter().enumerate() {
        p.button(
            Atlas::Elements5,
            [sprite[0], sprite[1], 48, 48],
            hover.map_or(Hover::None, Hover::Swap),
            [407.0 - 50.0 * i as f32, 212.0],
            Some(ctx.t(key)),
            Action::Command(command),
            enabled,
        );
    }
    // The status lines: coins, move out, upgrade.
    let lines = [
        (!m.coin_switch, "!Coin supply stopped."),
        (m.move_out, "!Soldiers moving out."),
        (upgrading, "!Upgrade Running"),
    ];
    for (i, (on, key)) in lines.into_iter().enumerate() {
        if on {
            p.text(
                ctx.t(key),
                Font::Outlined,
                [20.0, 220.0 + 16.0 * i as f32],
                WHITE,
            );
        }
    }
    p
}

/// UI-GAME-4.4, 8.1: the headquarters' and storehouses' window: the goods
/// and settlers pages (a slot per good: x = 23 + (slot mod 6) x 54,
/// y = 60 + (slot div 6) x 62, its count, the stop and out markers; a click
/// cycles the good's flag; "For all wares." cycles every good's) and the
/// headquarters' soldier reserve. The storehouse's destroy button is not
/// placed by the spec (Q-184): under the tabs' row on the right.
pub fn depot_window(ctx: &Ctx, at: Point, tab: u8) -> Panel {
    let world = ctx.world;
    let b = world.building_at(at).expect("a building");
    // UI-GAME-4.4 (F59, E-trace-026): the tabbed `uiDepot` frame, 376 x
    // 464.
    let size = DEPOT_DIALOG;
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &title_of(ctx, b.building_type),
        size,
        Action::CloseContext,
    );
    let hq = is_headquarters(world, at);
    let mut tabs: Vec<(u8, &str)> = vec![
        (9, "!List of all goods in depot."),
        (10, "!List of all settlers in depot."),
    ];
    if hq {
        tabs.push((11, "!Military management."));
    }
    if world.harbour_at(at).is_some() {
        tabs.push((12, "!Expedition management."));
    }
    let tab = if tabs.iter().any(|t| t.0 == tab) {
        tab
    } else {
        9
    };
    for (i, &(id, key)) in tabs.iter().enumerate() {
        tab_button(&mut p, ctx, id, i, key, Action::BuildingTab(id), id == tab);
    }
    if !hq {
        p.button(
            Atlas::Elements5,
            [448, 88, 48, 48],
            Hover::Swap([448, 184]),
            [300.0, 400.0],
            Some(ctx.t("!Destroy the building.")),
            Action::Command(Command::RemoveBuilding {
                player: ctx.player,
                at,
            }),
            true,
        );
    }
    let warehouse = world.warehouse(at);
    match tab {
        12 => expedition_page(&mut p, ctx, at),
        11 => {
            // UI-GAME-4.4 (as the original shows it, E-trace-028, E-trace-029;
            // Q-1053 answered): five rows 65 px apart, Private to General:
            // the rank's settler name left at (50, 90 + 65 k), a slider of the
            // economy dialogs' style from x 50 to 250 under it (the rank's
            // reserve, 0 to 15), `n/m` right-aligned on x 250 on the name's
            // line (m the reserve, n the soldiers of the rank held for it
            // now: those in the headquarters, at most m) and the rank's icon
            // in a framed 48 x 48 box at (260, 82 + 65 k).
            if let Some(m) = world.military_at(at) {
                let stock = world.stock_soldiers(at);
                for (rank, in_stock) in stock.iter().enumerate() {
                    let y = 90.0 + 65.0 * rank as f32;
                    let amount = m.castle[rank];
                    let good = ctx.catalog.soldiers[rank];
                    let name = ctx.texts.name(RANK_NAMES[rank]).to_owned();
                    p.text(name, Font::Body, [50.0, y], WHITE);
                    p.right_text(
                        format!("{}/{amount}", (*in_stock).min(amount)),
                        Font::Body,
                        [250.0, y],
                        WHITE,
                    );
                    p.widgets.push(Widget::Slider {
                        // The bar's middle on about 108 + 65 k.
                        at: [50.0, y + 18.0 - 4.0],
                        width: 200.0,
                        value: amount as f32 / crate::ui::CASTLE_MOST as f32,
                        kind: SliderKind::Castle(rank as u8),
                        enabled: true,
                    });
                    let box_at = [260.0, 82.0 + 65.0 * rank as f32];
                    p.sprite(Atlas::Elements3, [440, 216, 48, 48], box_at);
                    if let Some(good) = good {
                        good_icon(&mut p, ctx, good, box_at, 48.0);
                    }
                }
            }
        }
        _ => {
            let settlers = tab == 10;
            let goods = if settlers {
                &ctx.catalog.settlers
            } else {
                &ctx.catalog.wares
            };
            // UI-GAME-8.1: by the goods' slot numbers.
            for (slot, good) in ctx.catalog.slots(goods) {
                let cell = super::globals::goods_slot(slot);
                let flags = warehouse.map(|w| w.flags_of(good));
                p.button(
                    Atlas::Elements3,
                    [0, 80, 48, 56],
                    Hover::None,
                    cell,
                    Some(ctx.good_name(good)),
                    Action::Command(Command::WarehouseCycle {
                        player: ctx.player,
                        at,
                        good,
                    }),
                    true,
                );
                good_icon(&mut p, ctx, good, cell, 48.0);
                let count = warehouse.map_or(0, |w| w.count(good));
                p.text(
                    count.to_string(),
                    Font::Body,
                    [cell[0] + 46.0 - 12.0, cell[1] + 49.0 - 6.0],
                    WHITE,
                );
                if flags.is_some_and(|f| f.stop) {
                    p.sized(Atlas::Elements3, [312, 72, 16, 16], cell, [16.0, 16.0]);
                }
                if flags.is_some_and(|f| f.out) {
                    p.sized(
                        Atlas::Elements3,
                        [336, 72, 16, 16],
                        [cell[0], cell[1] + 33.0],
                        [16.0, 16.0],
                    );
                }
            }
            p.button(
                Atlas::Elements5,
                [304, 40, 48, 48],
                Hover::None,
                [240.0, 400.0],
                Some(ctx.t("!For all wares.")),
                Action::DepotAll { at, settlers },
                true,
            );
        }
    }
    p
}

/// UI-GAME-4.4 (Q-1053 answered, E-trace-029): the military page's row
/// names, the settler names of the five ranks.
pub(crate) const RANK_NAMES: [&str; 5] = [
    "dlg_PrivateSettler",
    "dlg_PrivateFirstClassSettler",
    "dlg_SergeantSettler",
    "dlg_OfficerSettler",
    "dlg_GeneralSettler",
];

/// UI-GAME-4.4, 8.1a: a harbour's expedition page. (a) Four slots at
/// (23 + 54 i, 60), no +/- and no click: the builder n/1, stones n/6, planks
/// n/4 (n set aside in the harbour's expedition store, SHIP-5.2) and the ship
/// 1/1 while one is called for the expedition (SHIP-5.4), else 0/1; each with
/// its good's name as tooltip. (b) `!Exploration.` at (298, 386) starts a
/// preparation (SHIP-5.1); while one runs it is disabled, with the tooltip
/// `!Exploration is running!`; nothing stops it (SHIP-5.1a). (c) `!Jump to
/// next expedition` at (248, 386): the player's ship in expedition mode
/// nearest this harbour ([`nearest_expedition`]); `!No expedition started`
/// and disabled while he has none.
fn expedition_page(p: &mut Panel, ctx: &Ctx, at: Point) {
    let world = ctx.world;
    let goods = world.expedition_goods(at);
    let builder = world.roles().builder;
    let row = |good: Option<u32>| goods.iter().find(|r| Some(r.good) == good);
    let stones = goods
        .iter()
        .find(|r| r.required == 6 && Some(r.good) != builder);
    let planks = goods
        .iter()
        .find(|r| r.required == 4 && Some(r.good) != builder);
    let harbour = world.harbour_at(at);
    let called =
        harbour.is_some_and(|h| matches!(h.expedition, s2_sim::ExpeditionState::WaitingForShip(_)));
    let slots: [(Option<u32>, u32, u32); 4] = [
        (builder, row(builder).map_or(0, |r| r.aside), 1),
        (stones.map(|r| r.good), stones.map_or(0, |r| r.aside), 6),
        (planks.map(|r| r.good), planks.map_or(0, |r| r.aside), 4),
        (ctx.catalog.ship, u32::from(called), 1),
    ];
    for (i, (good, n, of)) in slots.into_iter().enumerate() {
        let cell = [23.0 + 54.0 * i as f32, 60.0];
        // A slot of 8.1 with its good's name as tooltip, no click action.
        p.button(
            Atlas::Elements3,
            [0, 80, 48, 56],
            Hover::None,
            cell,
            good.map(|g| ctx.good_name(g)),
            Action::None,
            true,
        );
        if let Some(good) = good {
            good_icon(p, ctx, good, cell, 48.0);
        }
        p.text(
            format!("{n}/{of}"),
            Font::Body,
            [cell[0] + 46.0 - 18.0, cell[1] + 49.0 - 6.0],
            WHITE,
        );
    }
    let preparing = harbour.is_some_and(|h| h.expedition != s2_sim::ExpeditionState::Inactive);
    // UI-GAME-8.1a (d) (corrected tonight, E-trace-028): the label at (50,
    // 400) reads `!Exploration is running!` while the harbour prepares an
    // expedition, white, left edge x 45, centred on y 400 (its text in the
    // other states was not read: none).
    if preparing {
        p.text(
            ctx.t("!Exploration is running!"),
            Font::Body,
            [45.0, 400.0],
            WHITE,
        );
    }
    p.widgets.push(Widget::Button {
        atlas: Atlas::Elements5,
        rect: [256, 88, 48, 48],
        hover: Hover::Swap([256, 184]),
        pressed: Hover::Swap([256, 184]),
        at: [298.0, 386.0],
        size: None,
        tooltip: Some(ctx.t(if preparing {
            "!Exploration is running!"
        } else {
            "!Exploration."
        })),
        action: Action::Command(Command::ExpeditionStart {
            player: ctx.player,
            at,
        }),
        enabled: !preparing,
        selected: false,
    });
    let nearest = nearest_expedition(world, ctx.player, at);
    p.button(
        Atlas::Elements5,
        [352, 40, 48, 48],
        Hover::Swap([352, 136]),
        [248.0, 386.0],
        Some(ctx.t(if nearest.is_some() {
            "!Jump to next expedition"
        } else {
            "!No expedition started"
        })),
        nearest.map_or(Action::None, Action::JumpToShip),
        nearest.is_some(),
    );
}

/// UI-GAME-8.1a (c): the player's ship in expedition mode nearest to the
/// harbour's field `at` (MAP-GRID-2.1 distance; on a tie the earlier in the
/// world's ship list), by its index. The same ship on every click.
pub(crate) fn nearest_expedition(world: &World, player: PlayerId, at: Point) -> Option<usize> {
    world
        .ships()
        .iter()
        .enumerate()
        .filter(|(_, s)| s.owner == player && s.vessel.expedition)
        .min_by_key(|(i, s)| (s2_sim::grid::distance(s.position(), at), *i))
        .map(|(i, _)| i)
}

/// UI-GAME-5.1: the attack dialog. The number sent is the share of the
/// available soldiers, rounded (the label's rule is Q-184); the presets set
/// the share to their counts.
pub fn attack_dialog(ctx: &Ctx, target: Point, share: f32, strong: bool) -> Panel {
    let size = [466.0, 220.0];
    let mut p = Panel::new(PanelId::Context, context_origin(ctx, size));
    frame(
        &mut p,
        ctx,
        &ctx.t("!Attack Menu"),
        size,
        Action::CloseContext,
    );
    let available = ctx.world.attack_available(ctx.player, target);
    let count = attack_count(available, share);
    p.widgets.push(Widget::Button {
        atlas: Atlas::Elements5,
        rect: [8, 456, 48, 48],
        hover: Hover::None,
        pressed: Hover::None,
        at: [178.0, 49.0],
        size: None,
        tooltip: Some(ctx.t("!Use weak units first.")),
        action: Action::AttackStrong(false),
        enabled: strong,
        selected: !strong,
    });
    p.widgets.push(Widget::Button {
        atlas: Atlas::Elements5,
        rect: [64, 456, 48, 48],
        hover: Hover::None,
        pressed: Hover::None,
        at: [254.0, 49.0],
        size: None,
        tooltip: Some(ctx.t("!Use strong units first.")),
        action: Action::AttackStrong(true),
        enabled: !strong,
        selected: strong,
    });
    p.sprite(Atlas::Elements3, [0, 432, 400, 8], [37.0, 103.0]);
    p.text(ctx.t("!Soldiers"), Font::Heading, [243.0, 124.0], HEADING);
    let presets: [(Rect, &str, [f32; 2]); 4] = [
        ([8, 288, 48, 48], "!Low number of soldiers.", [36.0, 117.0]),
        (
            [64, 288, 48, 48],
            "!Medium number of soldiers.",
            [92.0, 117.0],
        ),
        (
            [8, 344, 48, 48],
            "!High number of soldiers.",
            [344.0, 117.0],
        ),
        (
            [64, 344, 48, 48],
            "!Highest number of soldiers.",
            [400.0, 117.0],
        ),
    ];
    for (i, (rect, key, at)) in presets.into_iter().enumerate() {
        p.button(
            Atlas::Elements1,
            rect,
            Hover::None,
            at,
            Some(ctx.t(key)),
            Action::AttackPreset(i),
            available > 0,
        );
    }
    p.widgets.push(Widget::Slider {
        at: [164.0, 132.0],
        width: 160.0,
        value: share,
        kind: SliderKind::AttackShare,
        enabled: available > 0,
    });
    p.text(count.to_string(), Font::Body, [245.0, 160.0], WHITE);
    p.sprite(Atlas::Elements3, [0, 432, 400, 8], [37.0, 170.0]);
    p.widgets.push(Widget::TextButton {
        text: ctx.t("!ATTACK"),
        font: Font::Big,
        at: [140.0, 180.0],
        colour: WHITE,
        action: Action::AttackGo,
        enabled: count > 0,
    });
    p
}

/// UI-GAME-5.1, T12: the soldiers a share of the available ones sends.
pub fn attack_count(available: u32, share: f32) -> u32 {
    ((available as f32) * share.clamp(0.0, 1.0)).round() as u32
}
