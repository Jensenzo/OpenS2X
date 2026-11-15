//! The debug mode's keyboard editing: placing, removing and ordering with
//! keys (`--debug`).

use crate::game::{size_name, Panel, Viewer, PLAYER};
use crate::input::{Hover, CLICK_SLOP, PAGE_SIZE};
use bevy::prelude::*;
use s2_sim::Command;

/// Keys and clicks that change the world. Each action is one command,
/// applied in its own simulation step; its outcome goes to the status line.
#[allow(clippy::too_many_arguments)]
pub(crate) fn edit_world(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut viewer: ResMut<Viewer>,
    window: Single<&Window>,
    hover: Res<Hover>,
    mut press: Local<Option<Vec2>>,
) {
    // The original interface takes the clicks and keys (`ui`).
    if viewer.map.is_none() || !viewer.debug {
        return;
    }
    let hovered = hover.0.map(|f| (f.x, f.y));

    if keys.just_pressed(KeyCode::KeyB) {
        viewer.overlay = !viewer.overlay;
        viewer.world_changed = true;
    }

    // Building list: Q / E page, 1-9 choose, 0 clears.
    let pages = viewer.building_list.len().div_ceil(PAGE_SIZE).max(1);
    if keys.just_pressed(KeyCode::KeyE) {
        viewer.page = (viewer.page + 1) % pages;
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        viewer.page = (viewer.page + pages - 1) % pages;
    }
    const DIGITS: [KeyCode; 9] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    for (key, offset) in DIGITS.iter().zip(0..) {
        let index = viewer.page * PAGE_SIZE + offset;
        // 1 to 4 are the dialog's buttons while it is open.
        if viewer.attack.is_some() {
            break;
        }
        if keys.just_pressed(*key) && index < viewer.building_list.len() {
            viewer.selected = Some(index);
            viewer.road_start = None;
            viewer.status = format!("click to place a {}", viewer.building_list[index].name);
        }
    }
    let right_click = buttons.just_released(MouseButton::Right);
    if (keys.just_pressed(KeyCode::Digit0) || right_click) && viewer.attack.take().is_some() {
        viewer.status.clear();
    } else if keys.just_pressed(KeyCode::Digit0) || right_click {
        viewer.selected = None;
        viewer.road_start = None;
        viewer.status.clear();
    }

    let mut command = None;

    // MIL-ATK-1: A over an enemy military building opens the attack dialog,
    // 1 to 4 give the order with that many soldiers, F9 chooses weak or
    // strong units first.
    {
        let world = viewer.world.as_ref().expect("world of the loaded map");
        let over = hovered.filter(|&at| crate::ui::military::attackable(world, PLAYER, at));
        viewer.attack_hover = over.is_some();
        if keys.just_pressed(KeyCode::KeyA) {
            if let Some(at) = over {
                viewer.attack = Some(at);
                viewer.selected = None;
                viewer.road_start = None;
            }
        }
    }
    if keys.just_pressed(KeyCode::F9) {
        viewer.attack_strong = !viewer.attack_strong;
    }
    if keys.just_pressed(KeyCode::F8) {
        viewer.borders = !viewer.borders;
        viewer.world_changed = true;
        viewer.moved = true;
    }
    if keys.just_pressed(KeyCode::F10) {
        viewer.show_messages = !viewer.show_messages;
    }
    // SHIP-5 to 7: Home prepares an expedition at the harbour under the
    // cursor, Insert chooses a ship, Numpad 1-6 send it in a compass
    // direction, End founds a harbour, Delete cancels the expedition.
    if keys.just_pressed(KeyCode::Insert) {
        viewer.ship_choice += 1;
    }
    if let Some(world) = viewer.world.as_ref() {
        let ship = crate::world::ships::chosen(world, PLAYER, viewer.ship_choice).map(|i| i as u32);
        if keys.just_pressed(KeyCode::Home) {
            match crate::world::ships::start_command(world, PLAYER, hovered) {
                Some(c) => command = Some(c),
                None => {
                    let note = "Home: point at one of your harbours";
                    viewer.status = note.into();
                }
            }
        }
        let go = [
            KeyCode::Numpad1,
            KeyCode::Numpad2,
            KeyCode::Numpad3,
            KeyCode::Numpad4,
            KeyCode::Numpad5,
            KeyCode::Numpad6,
        ];
        for (direction, key) in go.into_iter().enumerate() {
            if let (true, Some(ship)) = (keys.just_pressed(key), ship) {
                command = Some(Command::ExpeditionGo {
                    player: PLAYER,
                    ship,
                    direction: direction as u8,
                });
            }
        }
        if let Some(ship) = ship {
            if keys.just_pressed(KeyCode::End) {
                command = Some(Command::ExpeditionBuildHarbor {
                    player: PLAYER,
                    ship,
                });
            }
            if keys.just_pressed(KeyCode::Delete) {
                command = Some(Command::ExpeditionStop {
                    player: PLAYER,
                    ship,
                });
            }
        }
    }
    if let Some(target) = viewer.attack {
        let world = viewer.world.as_ref().expect("world of the loaded map");
        for (button, key) in [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
        ]
        .into_iter()
        .enumerate()
        {
            if keys.just_pressed(key) {
                command = crate::ui::military::attack_command(
                    world,
                    PLAYER,
                    target,
                    button,
                    viewer.attack_strong,
                );
            }
        }
        if command.is_some() {
            viewer.attack = None;
        }
    }

    // Debug ware: G / H choose the good; T over a flag picks where it starts,
    // T over another flag drops it there with that flag as destination.
    let wares = viewer.ware_list.len();
    if wares > 0 && keys.just_pressed(KeyCode::KeyG) {
        viewer.ware_choice = (viewer.ware_choice + 1) % wares;
    }
    if wares > 0 && keys.just_pressed(KeyCode::KeyH) {
        viewer.ware_choice = (viewer.ware_choice + wares - 1) % wares;
    }
    if keys.just_pressed(KeyCode::KeyT) && wares > 0 {
        let world = viewer.world.as_ref().expect("world of the loaded map");
        match hovered.filter(|&f| world.flag_at(f) == Some(PLAYER)) {
            None => viewer.status = "T: point at one of your flags".into(),
            Some(at) => match viewer.drop_from.take() {
                None => {
                    viewer.drop_from = Some(at);
                    viewer.status = format!(
                        "{} at ({}, {}): press T over the destination flag",
                        viewer.ware_list[viewer.ware_choice].1, at.0, at.1
                    );
                }
                Some(from) => {
                    command = Some(Command::DropWare {
                        player: PLAYER,
                        good: viewer.ware_list[viewer.ware_choice].0,
                        at: from,
                        destination: at,
                    });
                }
            },
        }
    }

    if keys.just_pressed(KeyCode::KeyX) {
        command = hovered.map(|at| Command::RemoveBuilding { player: PLAYER, at });
    }

    // MIL-UPG-1.3: U upgrades the military building under the cursor to the
    // next type of its chain (while the tool panel is shut; there U is a
    // weight key).
    if !viewer.show_tools && keys.just_pressed(KeyCode::KeyU) {
        command = hovered.map(|at| Command::UpgradeBuilding { player: PLAYER, at });
    }

    if keys.just_pressed(KeyCode::KeyI) {
        viewer.show_stock = !viewer.show_stock;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        viewer.show_production = !viewer.show_production;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        viewer.show_tools = !viewer.show_tools;
    }

    // ECO-PROD-1.2, 1.3: O switches production of the building under the
    // cursor on or off, K toggles its product switch.
    for (key, switch) in [(KeyCode::KeyO, false), (KeyCode::KeyK, true)] {
        if !keys.just_pressed(key) {
            continue;
        }
        let world = viewer.world.as_ref().expect("world of the loaded map");
        match hovered.and_then(|at| crate::ui::economy::production_target(world, PLAYER, at)) {
            None => viewer.status = "O / K: point at one of your finished buildings".into(),
            Some(at) if switch => command = Some(Command::ProductionSwitch { player: PLAYER, at }),
            Some(at) => command = Some(Command::ProductionToggle { player: PLAYER, at }),
        }
    }

    // ECO-PROD-7.3: [ ] choose a tool slot, U / J raise / lower its weight
    // by 0.1, V makes it the exclusive slot (or clears that), Y resets.
    if viewer.show_tools {
        let slots = s2_sim::TOOL_SLOTS;
        if keys.just_pressed(KeyCode::BracketRight) {
            viewer.tool_slot = (viewer.tool_slot + 1) % slots;
        }
        if keys.just_pressed(KeyCode::BracketLeft) {
            viewer.tool_slot = (viewer.tool_slot + slots - 1) % slots;
        }
        let slot = viewer.tool_slot as u8;
        let priorities = viewer
            .world
            .as_ref()
            .expect("world of the loaded map")
            .tool_priorities(PLAYER);
        let per_mille = (priorities.weight(viewer.tool_slot) * 1000.0).round() as i64;
        for (key, step) in [(KeyCode::KeyU, 100), (KeyCode::KeyJ, -100)] {
            if keys.just_pressed(key) {
                command = Some(Command::ToolWeight {
                    player: PLAYER,
                    slot,
                    per_mille: (per_mille + step).clamp(0, 1000) as u32,
                });
            }
        }
        if keys.just_pressed(KeyCode::KeyV) {
            command = Some(if priorities.exclusive == Some(slot) {
                Command::ToolClearExclusive { player: PLAYER }
            } else {
                Command::ToolExclusive {
                    player: PLAYER,
                    slot,
                }
            });
        }
        if keys.just_pressed(KeyCode::KeyY) {
            command = Some(Command::ToolReset { player: PLAYER });
        }
    }

    // ECO-DIST: F warehouse flags, R distribution tables, Z transport
    // priority. One panel is open at a time.
    for (key, panel) in [
        (KeyCode::KeyF, Panel::Warehouse),
        (KeyCode::KeyR, Panel::Distribution),
        (KeyCode::KeyZ, Panel::Priority),
        (KeyCode::F1, Panel::Military),
    ] {
        if keys.just_pressed(key) {
            viewer.panel = if viewer.panel == panel {
                Panel::None
            } else {
                panel
            };
        }
    }
    if viewer.panel != Panel::None {
        // The metalworks panel uses the same keys.
        viewer.show_tools = false;
        let v = &mut *viewer;
        let next = keys.just_pressed(KeyCode::BracketRight);
        let previous = keys.just_pressed(KeyCode::BracketLeft);
        let world = v.world.as_ref().expect("world of the loaded map");
        let step = |index: usize, len: usize| -> usize {
            if len == 0 {
                0
            } else if next {
                (index + 1) % len
            } else if previous {
                (index + len - 1) % len
            } else {
                index
            }
        };
        match v.panel {
            Panel::Warehouse => {
                v.wh_good = step(v.wh_good, v.warehouse_goods.len());
                let at = crate::ui::economy::warehouse_target(world, PLAYER, hovered);
                let good = v.warehouse_goods.get(v.wh_good).map(|g| g.0);
                if let (Some(at), Some(good)) = (at, good) {
                    if keys.just_pressed(KeyCode::Enter) {
                        command = Some(Command::WarehouseCycle {
                            player: PLAYER,
                            at,
                            good,
                        });
                    }
                    for (key, stop) in [(KeyCode::Semicolon, true), (KeyCode::Quote, false)] {
                        if keys.just_pressed(key) {
                            command = Some(Command::WarehouseToggle {
                                player: PLAYER,
                                at,
                                good,
                                stop,
                            });
                        }
                    }
                }
                let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
                for (key, mode) in [(KeyCode::F5, 0), (KeyCode::F6, 1), (KeyCode::F7, 2)] {
                    if let (true, Some(at)) = (keys.just_pressed(key), at) {
                        command = Some(Command::WarehouseAll {
                            player: PLAYER,
                            at,
                            mode,
                            settlers: shift,
                        });
                    }
                }
            }
            Panel::Distribution => {
                let tables = world.distribution_rules().len();
                if keys.just_pressed(KeyCode::Tab) && tables > 0 {
                    v.dist_table = (v.dist_table + 1) % tables;
                    v.dist_entry = 0;
                }
                let entries = world
                    .distribution_rules()
                    .get(v.dist_table)
                    .map_or(0, |t| t.entries.len());
                v.dist_entry = step(v.dist_entry, entries);
                let current = world.distribution(PLAYER);
                let weight = current
                    .get(v.dist_table)
                    .and_then(|t| t.get(v.dist_entry))
                    .map_or(0, |e| (e.weight() * 1000.0).round() as i64);
                for (key, delta) in [(KeyCode::KeyU, 100), (KeyCode::KeyJ, -100)] {
                    if keys.just_pressed(key) && entries > 0 {
                        command = Some(Command::DistributionWeight {
                            player: PLAYER,
                            table: v.dist_table as u8,
                            entry: v.dist_entry as u8,
                            thousandths: (weight + delta).clamp(0, 10_000) as u32,
                        });
                    }
                }
                if keys.just_pressed(KeyCode::KeyY) && tables > 0 {
                    command = Some(Command::DistributionReset {
                        player: PLAYER,
                        table: v.dist_table as u8,
                    });
                }
            }
            Panel::Priority => {
                let len = crate::ui::economy::rank_order(world, PLAYER).len();
                v.rank_sel = step(v.rank_sel, len);
                for (key, up) in [(KeyCode::KeyU, true), (KeyCode::KeyJ, false)] {
                    if keys.just_pressed(key) {
                        if let Some(c) =
                            crate::ui::economy::priority_move(world, PLAYER, v.rank_sel, up)
                        {
                            command = Some(c);
                            // The good stays selected: it moved one place.
                            v.rank_sel = if up {
                                v.rank_sel.saturating_sub(1)
                            } else {
                                (v.rank_sel + 1).min(len.saturating_sub(1))
                            };
                        }
                    }
                }
                if keys.just_pressed(KeyCode::KeyY) {
                    command = Some(Command::PriorityReset { player: PLAYER });
                }
            }
            Panel::Military => {
                v.mil_sel = step(v.mil_sel, crate::ui::military::SETTINGS);
                let steps = if keys.just_pressed(KeyCode::KeyU) {
                    1
                } else if keys.just_pressed(KeyCode::KeyJ) {
                    -1
                } else if keys.just_pressed(KeyCode::Enter) && v.mil_sel == 5 {
                    1
                } else {
                    0
                };
                if steps != 0 {
                    command = crate::ui::military::setting_command(
                        world, PLAYER, v.mil_sel, steps, hovered,
                    );
                }
                if keys.just_pressed(KeyCode::KeyY) {
                    command = Some(Command::MilitaryReset { player: PLAYER });
                }
            }
            Panel::None => {}
        }
    }

    // MIL-PROM-3.2, MIL-SOL-4.6: F2 toggles the coin supply and F3 the
    // moving out of soldiers of the military building under the cursor.
    for (key, coin) in [(KeyCode::F2, true), (KeyCode::F3, false)] {
        if !keys.just_pressed(key) {
            continue;
        }
        let world = viewer.world.as_ref().expect("world of the loaded map");
        let target = hovered.filter(|&at| {
            world
                .building_at(at)
                .is_some_and(|b| b.owner == PLAYER && !b.under_construction)
                && world.military_at(at).is_some()
        });
        match target {
            None => viewer.status = "F2 / F3: point at one of your military buildings".into(),
            Some(at) if coin => command = Some(Command::ToggleCoinSupply { player: PLAYER, at }),
            Some(at) => command = Some(Command::ToggleMoveOut { player: PLAYER, at }),
        }
    }
    if keys.just_pressed(KeyCode::F4) {
        viewer.territory = !viewer.territory;
        viewer.world_changed = true;
    }

    // Debug only, a shortcut past requests and orders: C hands a site what
    // it still needs, plus its workers.
    if keys.just_pressed(KeyCode::KeyC) {
        let world = viewer.world.as_ref().expect("world of the loaded map");
        match hovered.and_then(|at| world.site(at).map(|site| (at, site))) {
            None => viewer.status = "C: point at one of your construction sites".into(),
            Some((at, site)) => {
                command = Some(Command::DebugSupplySite {
                    player: PLAYER,
                    at,
                    wares: site.shortfall(),
                    workers: true,
                });
            }
        }
    }

    // A left click is a press and release without dragging.
    if buttons.just_pressed(MouseButton::Left) {
        *press = window.cursor_position();
    }
    let clicked = buttons.just_released(MouseButton::Left)
        && press
            .take()
            .zip(window.cursor_position())
            .is_some_and(|(a, b)| a.distance(b) < CLICK_SLOP);
    if clicked {
        let Some(at) = hovered else { return };
        let world = viewer.world.as_ref().expect("world of the loaded map");
        command = Some(if let Some(index) = viewer.selected {
            Command::PlaceBuilding {
                player: PLAYER,
                building: viewer.building_list[index].id,
                at,
            }
        } else if world.flag_at(at) == Some(PLAYER) {
            match viewer.road_start {
                Some(from) if from != at => {
                    viewer.road_start = None;
                    Command::BuildRoad {
                        player: PLAYER,
                        from,
                        to: at,
                    }
                }
                _ => {
                    viewer.road_start = Some(at);
                    viewer.status = format!("road from ({}, {}): click the other flag", at.0, at.1);
                    return;
                }
            }
        } else {
            viewer.road_start = None;
            Command::PlaceFlag { player: PLAYER, at }
        });
    }

    let Some(command) = command else { return };
    // Applied at the next tick boundary (see `advance_world`).
    viewer
        .world
        .as_mut()
        .expect("world of the loaded map")
        .submit(command);
}

/// The current page of the building list.
pub(crate) fn building_menu(viewer: &Viewer) -> Vec<String> {
    let list = &viewer.building_list;
    if list.is_empty() {
        return Vec::new();
    }
    let pages = list.len().div_ceil(PAGE_SIZE);
    let first = viewer.page * PAGE_SIZE;
    let entries: Vec<String> = list[first..(first + PAGE_SIZE).min(list.len())]
        .iter()
        .enumerate()
        .map(|(i, choice)| {
            let mark = if viewer.selected == Some(first + i) {
                "*"
            } else {
                ""
            };
            format!(
                "{mark}{} {} ({})",
                i + 1,
                choice.name,
                size_name(choice.size)
            )
        })
        .collect();
    let mut lines = vec![format!("buildings, page {} of {pages}:", viewer.page + 1)];
    for row in entries.chunks(3) {
        lines.push(format!("  {}", row.join("   ")));
    }
    lines
}
