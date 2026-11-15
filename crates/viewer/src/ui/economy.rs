//! The production overview and the metalworks tool-priority panel as plain
//! text lines (`specs/ECO-production-chains.md` ECO-PROD), built from the
//! world alone so that they can be tested without a window.

use std::collections::HashMap;

use s2_sim::grid::Point;
use s2_sim::view::{Activity, WorkPhase};
use s2_sim::{Building, PlayerId, ToolPriorities, WorkCycle, World, TOOL_SLOTS};

/// A building type and its display name, in build-menu order.
pub type TypeName = (u32, String);

/// ECO-PROD-6.1: a mine keeps up to this many of each food on order.
const MINE_FOOD_LIMIT: u32 = 2;

fn name_of(names: &HashMap<u32, String>, good: u32) -> String {
    names
        .get(&good)
        .cloned()
        .unwrap_or_else(|| format!("{good:08X}"))
}

/// What the worker of a finished building is doing, in words.
pub(crate) fn worker_state(world: &World, building: &Building) -> String {
    let kind = world.building_type(building.building_type);
    if kind.is_some_and(|k| k.worker.is_none() || k.worker_limit == 0) {
        return "-".into();
    }
    let at = (building.at, building.serial);
    let found = world
        .settler_views()
        .into_iter()
        .find_map(|v| match v.activity {
            Activity::Worker {
                building,
                serial,
                phase,
            } if (building, serial) == at => Some(Some(phase)),
            Activity::ToBuilding { building, serial } if (building, serial) == at => Some(None),
            _ => None,
        });
    match found {
        None => "no worker".into(),
        Some(None) => "worker on the way".into(),
        Some(Some(phase)) => match phase {
            WorkPhase::Begin if !building.production.enabled => "off".into(),
            WorkPhase::Begin => "starting".into(),
            WorkPhase::WaitInputs => "waiting for inputs".into(),
            WorkPhase::Working | WorkPhase::AtSpot => "working".into(),
            WorkPhase::Take | WorkPhase::TakeWare => "taking the product".into(),
            WorkPhase::WaitRoom | WorkPhase::WaitFlag => "flag full".into(),
            WorkPhase::WaitTarget | WorkPhase::WaitSpot => "looking for a target".into(),
            WorkPhase::FishRetry => "no fish found".into(),
            WorkPhase::WaitShip => "waiting for the ship check".into(),
            WorkPhase::Rest => "resting".into(),
            _ => "carrying".into(),
        },
    }
}

/// Input stock of one building: every good it holds or has on order, as
/// `name have/limit (+open orders)`.
fn stock_text(world: &World, building: &Building, names: &HashMap<u32, String>) -> String {
    let kind = world.building_type(building.building_type);
    let limit = match kind {
        Some(k) if k.cycle == WorkCycle::Mine => Some(MINE_FOOD_LIMIT),
        Some(k) => k.stock_limit,
        None => None,
    };
    let mut goods: Vec<u32> = world.inputs(building.at).iter().map(|&(g, _)| g).collect();
    for order in world.orders() {
        if order.target.at == building.at && !goods.contains(&order.good) {
            goods.push(order.good);
        }
    }
    goods.sort_unstable();
    goods
        .into_iter()
        .map(|good| {
            let have: u32 = world
                .inputs(building.at)
                .iter()
                .filter(|&&(g, _)| g == good)
                .map(|&(_, n)| n)
                .sum();
            let open = world
                .orders()
                .iter()
                .filter(|o| o.target.at == building.at && o.good == good)
                .count();
            let mut text = format!("{} {have}", name_of(names, good));
            if let Some(limit) = limit {
                text.push_str(&format!("/{limit}"));
            }
            if open > 0 {
                text.push_str(&format!(" (+{open})"));
            }
            text
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every building type with the player's buildings of it, their production
/// state and their stock (ECO-PROD-1, 6). Types without a building are
/// listed on one line at the end.
pub fn production_overview(
    world: &World,
    player: PlayerId,
    types: &[TypeName],
    names: &HashMap<u32, String>,
) -> Vec<String> {
    let mut buildings: Vec<&Building> = world.buildings().filter(|b| b.owner == player).collect();
    buildings.sort_by_key(|b| (b.building_type, b.at.1, b.at.0));
    let mut lines = Vec::new();
    let mut unbuilt = Vec::new();
    for (id, name) in types {
        let mine: Vec<&&Building> = buildings
            .iter()
            .filter(|b| b.building_type == *id)
            .collect();
        if mine.is_empty() {
            unbuilt.push(name.as_str());
            continue;
        }
        lines.push(format!("{name} x{}", mine.len()));
        for b in mine {
            let at = format!("({}, {})", b.at.0, b.at.1);
            if b.under_construction {
                lines.push(format!("  {at} construction site"));
                continue;
            }
            let kind = world.building_type(b.building_type);
            let runs = kind.is_some_and(|k| {
                matches!(
                    k.cycle,
                    WorkCycle::Workshop
                        | WorkCycle::Mine
                        | WorkCycle::Farm
                        | WorkCycle::Fisher
                        | WorkCycle::Hunter
                        | WorkCycle::DonkeyFarm
                        | WorkCycle::Shipyard
                )
            });
            let mut line = format!("  {at} {}", worker_state(world, b));
            if runs {
                let p = b.production;
                line.push_str(&format!(
                    "  {}{}  made {}",
                    if p.enabled { "on" } else { "OFF" },
                    if p.switched { " switched" } else { "" },
                    p.count
                ));
            }
            let stock = stock_text(world, b, names);
            if !stock.is_empty() {
                line.push_str(&format!("  {stock}"));
            }
            if let Some(field) = world.ore_field(b.at) {
                if let Some(r) = world.resource_at(field) {
                    line.push_str(&format!("  ore {} at ({}, {})", r.amount, field.0, field.1));
                }
            }
            lines.push(line);
        }
    }
    if !unbuilt.is_empty() {
        lines.push(format!("not built: {}", unbuilt.join(", ")));
    }
    lines
}

/// ECO-PROD-7: the player's 12 tool slots with weight, share of the weights,
/// accumulator and the exclusive mark; `selected` is the slot the keys act
/// on.
pub fn tool_lines(
    priorities: &ToolPriorities,
    tools: &[u32],
    names: &HashMap<u32, String>,
    selected: usize,
) -> Vec<String> {
    let total: f32 = (0..TOOL_SLOTS).map(|s| priorities.weight(s)).sum();
    let mut lines = vec![match priorities.exclusive {
        Some(slot) => format!(
            "metalworks tools: only {} (exclusive)",
            tools
                .get(usize::from(slot))
                .map_or_else(|| format!("slot {slot}"), |&g| name_of(names, g))
        ),
        None => "metalworks tools: by weight".into(),
    }];
    for slot in 0..TOOL_SLOTS {
        let name = tools
            .get(slot)
            .map_or_else(|| format!("slot {slot}"), |&g| name_of(names, g));
        let weight = priorities.weight(slot);
        let share = if total > 0.0 {
            weight / total * 100.0
        } else {
            0.0
        };
        lines.push(format!(
            "{}{:>2} {:<14} weight {:.3}  {:>4.1}%  acc {:.2}{}",
            if slot == selected { ">" } else { " " },
            slot,
            name,
            weight,
            share,
            priorities.accumulator(slot),
            if priorities.exclusive == Some(slot as u8) {
                "  EXCLUSIVE"
            } else {
                ""
            }
        ));
    }
    lines
}

/// One good of the warehouse panel: id, name, and whether it is a settler.
pub type PanelGood = (u32, String, bool);

/// The player's warehouse the panel acts on: the one under the cursor, else
/// the first of the player's.
pub fn warehouse_target(world: &World, player: PlayerId, hovered: Option<Point>) -> Option<Point> {
    let own = |at: Point| {
        world.warehouse(at).is_some() && world.building_at(at).is_some_and(|b| b.owner == player)
    };
    hovered
        .filter(|&at| own(at))
        .or_else(|| world.warehouses().map(|(at, _)| at).find(|&at| own(at)))
}

/// ECO-DIST-1: every good of the warehouse on `at` with its stock and flag
/// (`.` accepts, `STOP` no longer stored, `OUT` taken out); `selected`
/// indexes `goods`.
pub fn warehouse_lines(
    world: &World,
    at: Point,
    goods: &[PanelGood],
    selected: usize,
) -> Vec<String> {
    let Some(warehouse) = world.warehouse(at) else {
        return vec!["warehouse flags: no warehouse".into()];
    };
    let mut lines = vec![format!(
        "warehouse ({}, {}): flags  Enter click  ; stop  ' out  F5/F6/F7 all clear/stop/out (Shift: settlers)",
        at.0, at.1
    )];
    for (i, (good, name, settler)) in goods.iter().enumerate() {
        let f = warehouse.flags_of(*good);
        let mark = match (f.stop, f.out) {
            (false, false) => ".   ",
            (true, false) => "STOP",
            (false, true) => "OUT ",
            (true, true) => "BOTH",
        };
        lines.push(format!(
            "{}{} {:<16} {:>4}{}",
            if i == selected { ">" } else { " " },
            mark,
            name,
            warehouse.count(*good),
            if *settler { "  settler" } else { "" }
        ));
    }
    lines
}

/// ECO-DIST-2: table `table` of the player with weight, accumulator and the
/// number of open orders of each consumer. `rules_goods` names the table's
/// goods; `keys` names the consumer types.
pub fn distribution_lines(
    world: &World,
    player: PlayerId,
    table: usize,
    selected: usize,
    names: &HashMap<u32, String>,
    keys: &HashMap<u32, String>,
) -> Vec<String> {
    let rules = world.distribution_rules();
    let Some(rule) = rules.get(table) else {
        return vec!["distribution: no tables".into()];
    };
    let tables = world.distribution(player);
    let goods = rule
        .goods
        .iter()
        .map(|&g| name_of(names, g))
        .collect::<Vec<_>>()
        .join("/");
    let mut lines = vec![format!(
        "distribution {} of {}: {goods}  Tab table  [ ] entry  U/J weight +/- 0.1  Y reset",
        table + 1,
        rules.len()
    )];
    for (i, entry) in tables[table].iter().enumerate() {
        let waiting = world
            .orders()
            .iter()
            .filter(|o| o.player == player && rule.goods.contains(&o.good))
            .filter(|o| world.order_key(o) == Some(entry.key))
            .count();
        lines.push(format!(
            "{}{:<18} weight {:.3}  acc {:.2}  waiting {waiting}",
            if i == selected { ">" } else { " " },
            name_of(keys, entry.key),
            entry.weight(),
            entry.accumulator()
        ));
    }
    lines
}

/// The goods of the transport priority list in rank order (equal ranks by
/// id).
pub fn rank_order(world: &World, player: PlayerId) -> Vec<u32> {
    let mut goods: Vec<(i64, u32)> = world
        .ranks(player)
        .into_iter()
        .map(|(good, rank)| (rank, good))
        .collect();
    goods.sort_unstable();
    goods.into_iter().map(|(_, g)| g).collect()
}

/// ECO-DIST-4: the player's transport priority list, first served on top;
/// `selected` indexes [`rank_order`].
pub fn priority_lines(
    world: &World,
    player: PlayerId,
    names: &HashMap<u32, String>,
    selected: usize,
) -> Vec<String> {
    let mut lines = vec![
        "transport priority (first served on top)  [ ] good  U/J move up/down  Y reset".into(),
    ];
    for (i, good) in rank_order(world, player).into_iter().enumerate() {
        lines.push(format!(
            "{}{:>3} {}",
            if i == selected { ">" } else { " " },
            world.rank(player, good),
            name_of(names, good)
        ));
    }
    lines
}

/// ECO-DIST-4.2: the command that moves the good at list position
/// `selected` past its neighbour (`up`: towards the first served), or one
/// rank when the neighbour has the same rank; `None` at the ends.
pub fn priority_move(
    world: &World,
    player: PlayerId,
    selected: usize,
    up: bool,
) -> Option<s2_sim::Command> {
    let order = rank_order(world, player);
    let rank = world.rank(player, *order.get(selected)?);
    let neighbour = if up {
        order.get(selected.checked_sub(1)?)?
    } else {
        order.get(selected + 1)?
    };
    let other = world.rank(player, *neighbour);
    let to = match (up, other == rank) {
        (true, true) => rank - 1,
        (false, true) => rank + 1,
        _ => other,
    };
    Some(s2_sim::Command::PriorityMove {
        player,
        from: rank,
        to,
    })
}

/// The finished building of `player` on `at`, if any (the target of the
/// production commands).
pub fn production_target(world: &World, player: PlayerId, at: Point) -> Option<Point> {
    world
        .building_at(at)
        .filter(|b| b.owner == player && !b.under_construction)
        .map(|b| b.at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_sim::{BuildingSize, BuildingType, Command, Rules};

    const P0: PlayerId = PlayerId(0);
    const SAWMILL: u32 = 7;
    const PLANK: u32 = 1;
    const WOOD: u32 = 2;

    fn world() -> World {
        let terrain = s2_sim::map::Terrain {
            width: 20,
            height: 20,
            heights: vec![0; 400],
            height_floor: -100,
            states: vec![s2_sim::map::state::FOR_BUILDING; 400],
            samples: Vec::new(),
        };
        let mut sawmill = BuildingType::new(SAWMILL, BuildingSize::Small);
        sawmill.instant = true;
        sawmill.product = Some(PLANK);
        sawmill.worker = Some(50);
        sawmill.cycle = WorkCycle::Workshop;
        sawmill.stock_limit = Some(6);
        let mut goods = std::collections::BTreeMap::new();
        goods.insert(
            PLANK,
            s2_sim::GoodRules {
                costs: vec![(WOOD, 1)],
                ..Default::default()
            },
        );
        let rules = Rules {
            building_types: vec![sawmill],
            goods,
            ..Rules::empty()
        };
        let mut world = World::with_map(1, terrain, rules, P0);
        world.submit(Command::PlaceBuilding {
            player: P0,
            building: SAWMILL,
            at: (10, 10),
        });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()));
        world
    }

    fn names() -> HashMap<u32, String> {
        HashMap::from([(PLANK, "Plank".to_string()), (WOOD, "Wood".to_string())])
    }

    fn types() -> Vec<TypeName> {
        vec![
            (SAWMILL, "Sawmill".to_string()),
            (8, "Mill".to_string()),
            (9, "Bakery".to_string()),
        ]
    }

    // spec: none (the viewer's debug economy panel)
    #[test]
    fn overview_lists_built_and_unbuilt_types() {
        let world = world();
        let lines = production_overview(&world, P0, &types(), &names());
        assert_eq!(lines[0], "Sawmill x1");
        assert!(
            lines[1].starts_with("  (10, 10) no worker  on  made 0"),
            "{lines:?}"
        );
        assert_eq!(lines.last().unwrap(), "not built: Mill, Bakery");
    }

    // spec: none (the viewer's debug economy panel)
    #[test]
    fn overview_shows_off_and_orders() {
        let mut world = world();
        world.submit(Command::ProductionToggle {
            player: P0,
            at: (10, 10),
        });
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()));
        let lines = production_overview(&world, P0, &types(), &names());
        assert!(lines[1].contains("OFF"), "{lines:?}");
        world.submit(Command::ProductionToggle {
            player: P0,
            at: (10, 10),
        });
        world.step();
        let lines = production_overview(&world, P0, &types(), &names());
        assert!(lines[1].contains("on"), "{lines:?}");
        assert!(lines[1].contains("Wood 0/6 (+6)"), "{lines:?}");
    }

    // spec: ECO-PROD-7.1, ECO-PROD-7.5
    #[test]
    fn tool_panel_shows_weights_shares_and_the_exclusive_slot() {
        let mut p = ToolPriorities::default();
        let tools: Vec<u32> = (100..112).collect();
        let names: HashMap<u32, String> = tools
            .iter()
            .map(|&g| (g, format!("tool{}", g - 100)))
            .collect();
        let lines = tool_lines(&p, &tools, &names, 9);
        assert_eq!(lines.len(), 13);
        assert_eq!(lines[0], "metalworks tools: by weight");
        assert!(lines[10].starts_with("> 9 tool9"), "{}", lines[10]);
        assert!(lines[10].contains("weight 0.900"), "{}", lines[10]);
        assert!(lines[10].contains("17.0%"), "{}", lines[10]);
        p.exclusive = Some(1);
        let lines = tool_lines(&p, &tools, &names, 0);
        assert_eq!(lines[0], "metalworks tools: only tool1 (exclusive)");
        assert!(lines[2].ends_with("EXCLUSIVE"));
    }

    // spec: ECO-DIST-1.1
    #[test]
    fn warehouse_panel_marks_flags() {
        let mut world = world();
        world.submit(Command::PlaceBuilding {
            player: P0,
            building: SAWMILL,
            at: (4, 4),
        });
        world.step();
        let goods = vec![
            (PLANK, "Plank".to_string(), false),
            (WOOD, "Wood".to_string(), false),
        ];
        // No warehouse in this world.
        assert_eq!(warehouse_target(&world, P0, None), None);
        let lines = warehouse_lines(&world, (10, 10), &goods, 0);
        assert_eq!(lines, ["warehouse flags: no warehouse"]);
    }

    // spec: ECO-DIST-4.1, ECO-DIST-4.2
    #[test]
    fn priority_list_is_in_rank_order_and_moves() {
        let terrain = s2_sim::map::Terrain {
            width: 8,
            height: 8,
            heights: vec![0; 64],
            height_floor: -100,
            states: vec![s2_sim::map::state::FOR_BUILDING; 64],
            samples: Vec::new(),
        };
        let rules = Rules {
            transport_priorities: [(PLANK, 7), (WOOD, 9), (3, 0)].into_iter().collect(),
            ..Rules::empty()
        };
        let mut world = World::with_map(1, terrain, rules, P0);
        let names = HashMap::from([
            (PLANK, "Plank".into()),
            (WOOD, "Wood".into()),
            (3, "Coin".into()),
        ]);
        let lines = priority_lines(&world, P0, &names, 2);
        assert!(lines[1].ends_with("Coin"));
        assert!(
            lines[3].starts_with(">") && lines[3].ends_with("Wood"),
            "{lines:?}"
        );
        assert_eq!(priority_move(&world, P0, 0, true), None, "already first");
        let cmd = priority_move(&world, P0, 2, true).unwrap();
        world.submit(cmd);
        world.step();
        let order = rank_order(&world, P0);
        assert_eq!(
            order,
            [3, WOOD, PLANK],
            "wood took plank's rank, plank moved down"
        );
    }
}
