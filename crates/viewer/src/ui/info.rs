//! The debug text: what the hovered field holds, the economy, the speed.

use crate::game::{
    ai_text, amounts, good_name, placement_name, BuildingChoice, InfoText, Panel, SpeedText,
    Viewer, PLAYER, STATE_NAMES,
};
use crate::input::{building_menu, Hover};
use crate::tools::model::ViewMode;
use bevy::prelude::*;
use s2_assets::s2m::{FieldPos, Map, NO_GOOD};
use s2_rules::RuleSet;
use s2_sim::grid::Point;
use s2_sim::view::Activity;
use s2_sim::World;
use std::collections::HashMap;

/// The speed text: "Speed xN", and "PAUSED" on a line of its own while the
/// game is paused.
pub(crate) fn speed_label(paused: bool, speed: u32) -> String {
    if paused {
        format!("Speed x{speed}\nPAUSED")
    } else {
        format!("Speed x{speed}")
    }
}

/// Show the game speed large, in the corner, while a game runs.
pub(crate) fn speed_hud(
    viewer: Res<Viewer>,
    mode: Res<State<ViewMode>>,
    mut hud: Single<(&mut Text, &mut TextColor, &mut Visibility), With<SpeedText>>,
) {
    let (text, color, visibility) = &mut *hud;
    // The original interface has its own speed button (UI-GAME-2.2).
    if viewer.world.is_none() || *mode.get() == ViewMode::Model || !viewer.debug {
        **visibility = Visibility::Hidden;
        return;
    }
    **visibility = Visibility::Visible;
    let label = speed_label(viewer.paused, viewer.speed);
    if text.0 != label {
        text.0 = label;
    }
    color.0 = if viewer.paused {
        Color::srgb(1.0, 0.35, 0.3)
    } else {
        Color::WHITE
    };
}

/// Describe the field under the cursor.
pub(crate) fn hover_info(
    viewer: Res<Viewer>,
    view: Res<crate::render::view3d::View3d>,
    hover: Res<Hover>,
    info: Single<(&mut Text, &mut Visibility), With<InfoText>>,
) {
    let (mut text, mut visibility) = info.into_inner();
    let shown = if viewer.debug {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != shown {
        *visibility = shown;
    }
    if !viewer.debug {
        return;
    }
    let mut lines = Vec::new();
    match (&viewer.map, &viewer.error) {
        (Some(map), _) => {
            lines.push(format!(
                "{}  {} x {}  (map {} of {})",
                map.info.name,
                map.info.width,
                map.info.height,
                viewer.current + 1,
                viewer.maps.len()
            ));
            if viewer.compact {
                if let Some(world) = &viewer.world {
                    lines.push(format!(
                        "tick {}{}",
                        world.tick(),
                        ai_text(viewer.ai.as_ref())
                    ));
                    if let Some(target) = viewer.attack {
                        lines.extend(crate::ui::military::attack_lines(
                            world,
                            PLAYER,
                            target,
                            viewer.attack_strong,
                        ));
                    }
                    lines.extend(crate::ui::military::message_lines(world, 8));
                }
                if !viewer.status.is_empty() {
                    lines.push(format!("> {}", viewer.status));
                }
                let new_text = lines.join("\n");
                if text.0 != new_text {
                    text.0 = new_text;
                }
                return;
            }
            let field = hover.0;
            match field {
                Some(field) => {
                    lines.extend(describe_field(map, viewer.rules.as_ref(), field));
                    if let Some(world) = &viewer.world {
                        lines.extend(describe_world(
                            world,
                            &viewer.building_list,
                            &viewer.good_names,
                            (field.x, field.y),
                        ));
                    }
                }
                None => lines.push("field: -".into()),
            }
            if let Some(ai) = viewer.ai.as_ref() {
                lines.push(ai_text(Some(ai)).trim_start().to_string());
            }
            if let Some(world) = &viewer.world {
                lines.extend(economy_lines(world, &viewer.good_names));
            }
            if viewer.show_stock {
                if let Some(world) = &viewer.world {
                    let hovered = field.map(|f| (f.x, f.y));
                    lines.extend(stock_lines(world, &viewer.good_names, hovered));
                }
            }
            if let Some(world) = &viewer.world {
                if viewer.show_production {
                    let types: Vec<crate::ui::economy::TypeName> = viewer
                        .building_list
                        .iter()
                        .map(|c| (c.id, c.name.clone()))
                        .collect();
                    lines.push("production:".into());
                    lines.extend(crate::ui::economy::production_overview(
                        world,
                        PLAYER,
                        &types,
                        &viewer.good_names,
                    ));
                }
                match viewer.panel {
                    Panel::Warehouse => {
                        let at = crate::ui::economy::warehouse_target(
                            world,
                            PLAYER,
                            field.map(|f| (f.x, f.y)),
                        );
                        match at {
                            Some(at) => lines.extend(crate::ui::economy::warehouse_lines(
                                world,
                                at,
                                &viewer.warehouse_goods,
                                viewer.wh_good,
                            )),
                            None => lines.push("warehouse flags: no warehouse".into()),
                        }
                    }
                    Panel::Distribution => lines.extend(crate::ui::economy::distribution_lines(
                        world,
                        PLAYER,
                        viewer.dist_table,
                        viewer.dist_entry,
                        &viewer.good_names,
                        &viewer.building_names,
                    )),
                    Panel::Priority => lines.extend(crate::ui::economy::priority_lines(
                        world,
                        PLAYER,
                        &viewer.good_names,
                        viewer.rank_sel,
                    )),
                    Panel::Military => lines.extend(crate::ui::military::settings_lines(
                        world,
                        PLAYER,
                        viewer.mil_sel,
                        field.map(|f| (f.x, f.y)),
                    )),
                    Panel::None => {}
                }
                if viewer.show_tools {
                    lines.extend(crate::ui::economy::tool_lines(
                        &world.tool_priorities(PLAYER),
                        &viewer.tool_goods,
                        &viewer.good_names,
                        viewer.tool_slot,
                    ));
                }
            }
            if let (Some(world), Some(target)) = (&viewer.world, viewer.attack) {
                lines.extend(crate::ui::military::attack_lines(
                    world,
                    PLAYER,
                    target,
                    viewer.attack_strong,
                ));
            }
            if let (Some(world), true) = (&viewer.world, viewer.show_messages) {
                let messages = crate::ui::military::message_lines(world, 8);
                if !messages.is_empty() {
                    lines.push("messages (F10):".into());
                    lines.extend(messages);
                }
            }
            if let Some(world) = viewer.world.as_ref() {
                lines.extend(crate::world::ships::lines(
                    world,
                    PLAYER,
                    viewer.ship_choice,
                ));
            }
            lines.extend(building_menu(&viewer));
            if let Some((_, name)) = viewer.ware_list.get(viewer.ware_choice) {
                let world = viewer.world.as_ref();
                let tick = world.map_or(0, |w| w.tick());
                let carriers = world.map_or(0, |w| w.carriers().len());
                let moving = world.map_or(0, |w| w.wares().count());
                let state = if viewer.paused { "paused" } else { "running" };
                lines.push(format!(
                    "ware: {name}   tick {tick} ({state}, x{})   carriers {carriers}   wares {moving}",
                    viewer.speed
                ));
            }
            if !viewer.status.is_empty() {
                lines.push(format!("> {}", viewer.status));
            }
        }
        (None, Some(error)) => lines.push(error.clone()),
        (None, None) => {}
    }
    if view.active {
        lines.push(
            "3D: WASD/arrows/middle drag pan  wheel zoom  PgUp/PgDn rotate  F11 2D view  F12 screenshot"
                .into(),
        );
    } else {
        lines.push("N/P map  WASD/arrows/drag pan  wheel zoom  F11 3D view  F12 screenshot".into());
    }
    lines.push(
        "B overlay  click: flag / two flags: road  Q/E 1-9: building  0: none  X: remove".into(),
    );
    lines.push(
        "G/H ware  T at a flag, T at the destination flag: drop it  Space pause  +/- speed".into(),
    );
    lines.push("I stock  C (debug) supply the construction site under the cursor".into());
    lines.push(
        "L production list  O on/off and K switch of the building under the cursor  M tools:"
            .into(),
    );
    lines.push("  [ ] slot  U/J weight +/- 0.1  V exclusive  Y reset".into());
    lines.push(
        "F warehouse flags (Enter ; ' F5-F7)  R distribution (Tab [ ] U/J Y)  Z transport priority (U/J Y)"
            .into(),
    );
    lines.push(
        "F1 military settings ([ ] U/J Enter Y)  F2 coin supply / F3 move soldiers out (under cursor)  F4 territory tint"
            .into(),
    );
    lines.push(
        "A over an enemy military building: attack (1-4 count, F9 weak/strong first)  F8 borders  F10 messages"
            .into(),
    );
    lines.push(
        "magenta: open settler request  green: builder/leveller  blue: worker  grey: going home"
            .into(),
    );
    if !viewer.real_game && viewer.map.is_some() {
        lines.push("no rules: no headquarters, every field is yours".into());
    }
    if viewer.overlay {
        lines.push(
            "overlay: yellow flag, green small, orange medium, red large, purple mine, blue harbour"
                .into(),
        );
    }

    let new_text = lines.join("\n");
    if text.0 != new_text {
        text.0 = new_text;
    }
}

pub(crate) fn describe_field(map: &Map, rules: Option<&RuleSet>, field: FieldPos) -> Vec<String> {
    let Some(index) = map.field_index(field) else {
        return Vec::new();
    };
    let state = map.states[index];
    let names: Vec<&str> = STATE_NAMES
        .iter()
        .filter(|(bit, _)| state & bit != 0)
        .map(|&(_, name)| name)
        .collect();
    let resource = map.resources[index];
    let resource = if resource.good == NO_GOOD {
        "none".to_owned()
    } else {
        format!("{} x good {:08X}", resource.amount, resource.good)
    };
    let start = map
        .info
        .start_positions
        .iter()
        .position(|&start| start == field)
        .map(|player| format!("  (start of player {player})"))
        .unwrap_or_default();
    let pattern = map.patterns[index];
    let terrain_name = rules
        .and_then(|rules| rules.patterns.get(pattern))
        .and_then(|p| p.name.as_deref())
        .map(|name| format!(" {name}"))
        .unwrap_or_default();
    let deposit_names: Vec<String> = map
        .deposits
        .iter()
        .filter(|deposit| deposit.field == field)
        .map(|deposit| {
            rules
                .and_then(|rules| rules.deposits.get(deposit.type_id))
                .and_then(|d| d.name.clone())
                .unwrap_or_else(|| format!("{:08X}", deposit.type_id))
        })
        .collect();
    let deposits = if deposit_names.is_empty() {
        "none".to_owned()
    } else {
        deposit_names.join(", ")
    };
    vec![
        format!("field: ({}, {}){start}", field.x, field.y),
        format!("height: {}", map.field_height(field).unwrap_or(0)),
        format!("terrain: {pattern:08X}{terrain_name}"),
        format!("deposit: {deposits}"),
        format!("state: {state:#06X} {}", names.join(" ")),
        format!("resource: {resource}"),
        format!("continent: {}", map.continent_of[index]),
    ]
}

/// ECO-REQ: the player's waiting settler requests by profession, open
/// orders, settlers on their way and the latest message.
pub(crate) fn economy_lines(world: &World, names: &HashMap<u32, String>) -> Vec<String> {
    let mut waiting: Vec<(u32, u32)> = Vec::new();
    for request in world.requests(PLAYER) {
        match waiting.iter_mut().find(|(g, _)| *g == request.profession) {
            Some((_, n)) => *n += 1,
            None => waiting.push((request.profession, 1)),
        }
    }
    let orders = world.orders().iter().filter(|o| o.player == PLAYER).count();
    let out = world
        .settler_views()
        .iter()
        .filter(|v| {
            matches!(v.id, s2_sim::view::SettlerRef::Settler(_)) && v.player == PLAYER && v.walking
        })
        .count();
    let mut lines = vec![format!(
        "requests waiting: {} ({})   ware orders: {orders}   settlers walking: {out}",
        world.requests(PLAYER).len(),
        amounts(names, &waiting)
    )];
    if let Some(message) = world.messages().iter().rev().find(|m| m.player == PLAYER) {
        lines.push(format!(
            "message (tick {}): {} ({}, {})",
            message.tick, message.text, message.at.0, message.at.1
        ));
    }
    lines
}

/// The stock of the warehouse under the cursor, or else of the player's
/// first warehouse (ECO-HQ-2), wrapped into lines of a few goods each.
pub(crate) fn stock_lines(
    world: &World,
    names: &HashMap<u32, String>,
    hovered: Option<Point>,
) -> Vec<String> {
    let under_cursor = hovered.and_then(|at| world.stock(at).map(|stock| (at, stock)));
    let own = || {
        world
            .warehouses()
            .find(|&(at, _)| world.building_at(at).is_some_and(|b| b.owner == PLAYER))
    };
    let Some((at, stock)) = under_cursor.or_else(own) else {
        return vec!["stock: no warehouse".into()];
    };
    let owner = world.building_at(at).map_or(0, |b| b.owner.0);
    let mut lines = vec![format!(
        "stock of the warehouse at ({}, {}), player {owner}:",
        at.0, at.1
    )];
    for row in stock.chunks(8) {
        lines.push(format!("  {}", amounts(names, row)));
    }
    lines
}

/// What the simulation holds on a field and what may be placed there.
pub(crate) fn describe_world(
    world: &World,
    list: &[BuildingChoice],
    names: &HashMap<u32, String>,
    field: Point,
) -> Vec<String> {
    let owner = match world.field_owner(field) {
        Some(owner) => format!("player {}", owner.0),
        None => "no one".into(),
    };
    let explored = if world.is_explored(PLAYER, field) {
        "explored"
    } else {
        "unexplored"
    };
    let mut lines = vec![
        format!("owner: {owner}, {explored}"),
        format!("can place: {}", placement_name(world.placement_at(field))),
    ];
    if world.flag_at(field).is_some() {
        lines.push("flag".into());
    }
    if let Some(building) = world.building_at(field) {
        let name = list
            .iter()
            .find(|choice| choice.id == building.building_type)
            .map_or_else(
                || format!("{:08X}", building.building_type),
                |c| c.name.clone(),
            );
        let state = if building.under_construction {
            "construction site"
        } else {
            "building"
        };
        lines.push(format!("{state}: {name}"));
    }
    if let Some(site) = world.site(field) {
        lines.extend(describe_site(world, site, names));
    }
    if let Some(building) = world.building_at(field).filter(|b| !b.under_construction) {
        lines.extend(describe_work(world, building, names));
        lines.extend(crate::ui::military::building_lines(
            world,
            field,
            world.coin_good(),
        ));
    }
    if let Some(deposit) = world.deposit(field) {
        let state = match deposit.state {
            s2_sim::DepositState::Sapling => "sapling".to_owned(),
            s2_sim::DepositState::Removing { .. } => "felled".to_owned(),
            s2_sim::DepositState::Growing { ripe_at } if ripe_at <= world.tick() => {
                "ripe".to_owned()
            }
            s2_sim::DepositState::Growing { ripe_at } => {
                format!("growing, ripe in {} ticks", ripe_at - world.tick())
            }
        };
        let reserved = if deposit.reserved_by.is_some() {
            ", reserved"
        } else {
            ""
        };
        lines.push(format!("deposit {:08X}: {state}{reserved}", deposit.kind));
    }
    if let Some(road) = world.road_at(field) {
        lines.push(format!("road, {} steps", road.path.len() - 1));
    }
    lines
}

/// A finished building's worker, its step, its input stock and its target.
pub(crate) fn describe_work(
    world: &World,
    building: &s2_sim::Building,
    names: &HashMap<u32, String>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let at = (building.at, building.serial);
    let worker = world
        .settler_views()
        .into_iter()
        .find(|v| match v.activity {
            Activity::Worker {
                building, serial, ..
            }
            | Activity::ToBuilding { building, serial } => (building, serial) == at,
            _ => false,
        });
    match worker {
        Some(s) => {
            let step = match s.activity {
                Activity::Worker { .. } => s.detail.clone().unwrap_or_default(),
                _ => "on the way".to_owned(),
            };
            lines.push(format!(
                "  worker {}: {step}",
                good_name(names, s.profession)
            ));
        }
        None if world.stock(building.at).is_none() => lines.push("  no worker".into()),
        None => {}
    }
    let inputs = world.inputs(building.at);
    if !inputs.is_empty() {
        lines.push(format!("  inputs: {}", amounts(names, inputs)));
    }
    if let Some(target) = world.reserved_target(building.at) {
        lines.push(format!("  target: ({}, {})", target.0, target.1));
    }
    lines
}

/// ECO-HQ-4: levelling, frame and main work, materials and workers.
pub(crate) fn describe_site(
    world: &World,
    site: &s2_sim::ConstructionSite,
    names: &HashMap<u32, String>,
) -> Vec<String> {
    let levelling = if !site.levelling_needed {
        "not needed".to_owned()
    } else if site.levelled() {
        "done".to_owned()
    } else {
        format!("{} / 100", (site.levelling() * 100.0).round() as u32)
    };
    let workers = match (site.leveller.is_some(), site.builder.is_some()) {
        (false, false) => "none",
        (true, false) => "leveller",
        (false, true) => "builder",
        (true, true) => "leveller, builder",
    };
    vec![
        format!(
            "  progress {}%   levelling {levelling}   workers: {workers}",
            site.progress_permille(world.bulldoze_ticks()) / 10
        ),
        format!(
            "  work {} / {} ticks (frame done at {})",
            site.work_done, site.work_total, site.frame_end
        ),
        format!("  on site: {}", amounts(names, &site.on_site)),
        format!("  still needed: {}", amounts(names, &site.shortfall())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // The speed line is always there; PAUSED is added while paused.
    // spec: UI-GAME-2.7
    #[test]
    fn speed_label_shows_speed_and_pause() {
        assert_eq!(speed_label(false, 1), "Speed x1");
        assert_eq!(speed_label(false, 16), "Speed x16");
        assert_eq!(speed_label(true, 2), "Speed x2\nPAUSED");
    }
}
