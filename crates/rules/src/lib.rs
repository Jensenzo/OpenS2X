//! Game rule tables, set up by running the original property scripts
//! (`specs/LUA-properties-api.md`) in an embedded Lua.
//!
//! [`load`] runs `data/scripts/properties/data.lua` of the player's install and
//! returns a [`RuleSet`]: goods, professions, buildings, terrain types,
//! deposits, doodads, animals, sounds, AI profiles and global settings, keyed
//! by the scripts' ids.

pub mod mission;
pub mod model;
pub mod scenario;
pub mod script;

use std::path::PathBuf;
use std::rc::Rc;

use s2_assets::{rc00, GameDir};

pub use model::RuleSet;
pub use script::{LoadReport, Overlay, ScriptError, ScriptSource};

use s2_assets::s2m::{self, Map};
use s2_sim::{
    BuildingSize, BuildingType, DepositKind, GameSetup, Ground, MapDeposit, PlayerId, Rules,
    Settings, WorkCycle, World,
};

/// ECO-HQ-2.1: the headquarters' base stock, by script global name, in the
/// spec's order: wares, then settlers.
pub const START_STOCK: [(&str, u32); 38] = [
    ("wood", 12),
    ("plank", 22),
    ("stone", 34),
    ("fish", 2),
    ("flesh", 3),
    ("bread", 4),
    ("beer", 3),
    ("coal", 8),
    ("iron", 8),
    ("axe", 3),
    ("saw", 1),
    ("pickaxe", 1),
    ("hammer", 8),
    ("shovel", 2),
    ("pot", 2),
    ("fishingRod", 3),
    ("scythe", 4),
    ("hatchet", 1),
    ("rollingPin", 1),
    ("spear", 1),
    ("sword", 3),
    ("shield", 3),
    ("boat", 6),
    ("good_helper", 26),
    ("good_constructor", 5),
    ("good_bulldozer", 3),
    ("good_woodcutter", 4),
    ("good_forester", 2),
    ("good_stonecutter", 2),
    ("good_hunter", 1),
    ("good_cabinetmaker", 2),
    ("good_miner", 5),
    ("good_smith", 2),
    ("good_locksmith", 1),
    ("good_scholar", 3),
    ("good_explorer", 1),
    // ECO-HQ-2.1: the settler `good_donkey`, not the ware `donkey`.
    ("good_donkey", 4),
    ("good_soldier0", 23),
];

/// ECO-SPC-3.3: the geologist's message for each good, by script global.
pub const FOUND_MESSAGES: [(&str, &str); 5] = [
    ("gold", "Found Gold!"),
    ("coal", "Found Coal!"),
    ("stone", "Found Stone!"),
    ("ironOre", "Found Iron Ore!"),
    ("water", "Found Water!"),
];

/// ECO-PROD-2.2: the foods a mine eats, in the order it eats them, by script
/// global.
pub const MINE_FOOD: [&str; 3] = ["fish", "bread", "flesh"];

/// ECO-PROD-7: the tool of each of the 12 priority slots, by script global
/// (gripper, axe, saw, pickaxe, hammer, shovel, pot, fishing rod, scythe,
/// hatchet, rolling pin, spear), and the ware group they belong to.
pub const TOOL_SLOTS: [&str; 12] = [
    "gripper",
    "axe",
    "saw",
    "pickaxe",
    "hammer",
    "shovel",
    "pot",
    "fishingRod",
    "scythe",
    "hatchet",
    "rollingPin",
    "spear",
];
pub const TOOLS_GROUP: &str = "tools";
/// UI-GAME-8.5b, 8.5e: the goods of the fourteen production counters, by script
/// global, in the counters' order.
pub const PRODUCTION_COUNTERS: [&str; 14] = [
    "wood", "plank", "stone", "food", "water", "beer", "coal", "ironOre", "gold", "iron", "coin",
    "tools", "weapons", "boat",
];

/// One distribution table: goods and (consumer, weight in thousandths).
pub type DistributionTable = (&'static [&'static str], &'static [(&'static str, u32)]);

/// ECO-DIST-2.1: the six distribution tables with their default weights in
/// thousandths, by script global: the goods whose orders use the table, and
/// the consumer building types (`construction` for sites).
pub const DISTRIBUTION: [DistributionTable; 6] = [
    (
        &["corn"],
        &[
            ("mill", 500),
            ("piggery", 300),
            ("donkeyBreeding", 200),
            ("brewery", 300),
        ],
    ),
    (&["iron"], &[("smithy", 800), ("locksmithery", 400)]),
    (
        &["water"],
        &[
            ("bakery", 600),
            ("brewery", 300),
            ("piggery", 200),
            ("donkeyBreeding", 300),
        ],
    ),
    (
        &["plank"],
        &[
            ("construction", 1000),
            ("locksmithery", 400),
            ("shipYard", 200),
        ],
    ),
    (
        &["fish", "flesh", "bread"],
        &[
            ("stoneMine", 300),
            ("coalMine", 500),
            ("ironMine", 700),
            ("goldMine", 1000),
        ],
    ),
    (
        &["coal"],
        &[("smithy", 800), ("smeltery", 600), ("mint", 1000)],
    ),
];

/// ECO-DIST-2.1: the key of a building under construction.
pub const CONSTRUCTION: &str = "construction";

/// ECO-HQ-1.1: the script global of the headquarters' building type.
pub const HEADQUARTERS: &str = "castle";

/// ECO-CAT-3 column "Cycle" = own: buildings with their own work cycle
/// (`ECO-food-mining.md`), by script global.
pub const OWN_CYCLES: [(&str, WorkCycle); 5] = [
    ("farm", WorkCycle::Farm),
    ("fisher", WorkCycle::Fisher),
    ("hunter", WorkCycle::Hunter),
    ("donkeyBreeding", WorkCycle::DonkeyFarm),
    ("shipYard", WorkCycle::Shipyard),
];

/// Which worker cycle a building type runs, from the data alone (ECO-CAT-3):
/// a mine by its size, an outdoor gatherer by a product the map's deposits
/// supply, a forester by a planting good without a product, every other
/// building with a worker and a product as a workshop.
fn work_cycle(rules: &RuleSet, building: &model::Building) -> WorkCycle {
    if let Some(&(_, cycle)) = OWN_CYCLES
        .iter()
        .find(|(name, _)| rules.id_named(name) == Some(building.id))
    {
        return cycle;
    }
    let product = building.goods.map(|a| a.good);
    let gathered = product
        .and_then(|p| rules.goods.get(p))
        .is_some_and(|g| g.deposits_enabled.is_some_and(|d| d != 0));
    match (building.worker, product, building.planting_good) {
        (None, ..) => WorkCycle::None,
        (Some(_), Some(_), _) if building.size == Some(model::BuildingSize::Mine) => {
            WorkCycle::Mine
        }
        (Some(_), Some(_), None) if gathered => WorkCycle::Gatherer,
        (Some(_), None, Some(_)) => WorkCycle::Forester,
        (Some(_), Some(_), _) => WorkCycle::Workshop,
        (Some(_), None, None) => WorkCycle::None,
    }
}

/// ECO-OUT-4.5: the tree kinds of tree types 0 to 3, by deposit name.
pub const TREE_FAMILIES: [&[&str]; 4] = [
    &["AfricanA", "OliveA", "PalmA", "CypressA"],
    &[
        "BirchA",
        "BirchB",
        "BirchC",
        "BroadLeafA",
        "BroadLeafB",
        "BroadLeafC",
    ],
    &["FirA", "FirB"],
    &["BroadLeafB", "FirB"],
];

/// ECO-OUT-4.5: the kind for any other tree type.
pub const DEFAULT_TREE: &str = "BirchA";

/// ECO-OUT-1.4: growing start size and speed when the script sets none.
pub const GROWING_START: f64 = 0.1;
pub const GROWING_SPEED: f64 = 0.0009;

/// ECO-OUT-6.2: ticks for a deposit to grow from `size` to 1.0 or more,
/// adding `speed` per tick in 32-bit floating point (1001 for a tree from
/// 0.1). Counted here, so that the simulation keeps whole ticks.
pub fn grow_ticks(size: f64, speed: f64) -> u32 {
    let (mut size, speed) = (size as f32, speed as f32);
    if speed <= 0.0 {
        return if size >= 1.0 { 0 } else { u32::MAX };
    }
    let mut ticks = 0;
    while size < 1.0 {
        size += speed;
        ticks += 1;
    }
    ticks
}

/// TRN-FREE-5.5a: the `bp_setConstructionPosition` offsets by tribe and
/// index, truncated toward zero to whole samples; unset ones are (0, 0).
fn construction_positions(building: &model::Building) -> [[(i32, i32); 4]; 3] {
    let mut positions = [[(0, 0); 4]; 3];
    for p in &building.construction_positions {
        if let Some(slot) = positions
            .get_mut(p.tribe as usize)
            .and_then(|t| t.get_mut(p.index as usize))
        {
            *slot = (p.x.trunc() as i32, p.y.trunc() as i32);
        }
    }
    positions
}

impl RuleSet {
    /// W16: every group good a recipe takes (a recipe set's or an H6
    /// recipe's input), with its member goods in good order.
    pub(crate) fn recipe_groups(&self) -> std::collections::BTreeMap<u32, Vec<u32>> {
        let inputs = self.buildings.iter().flat_map(|b| {
            b.inputs.iter().map(|a| a.good).chain(
                b.recipes
                    .iter()
                    .flat_map(|r| r.inputs.iter().map(|a| a.good)),
            )
        });
        let mut groups = std::collections::BTreeMap::new();
        for good in inputs {
            let members: Vec<u32> = self
                .goods
                .iter()
                .filter(|g| g.group == Some(good) && g.id != good)
                .map(|g| g.id)
                .collect();
            if !members.is_empty() {
                let mut sorted = members;
                sorted.sort_unstable();
                groups.insert(good, sorted);
            }
        }
        groups
    }

    /// W16: the kept stock by member good: a group's keep stands for each
    /// of its members; a good's own keep wins over its group's.
    pub(crate) fn recipe_keep_by_member(&self) -> std::collections::BTreeMap<u32, u32> {
        let mut keep = std::collections::BTreeMap::new();
        for (&good, &count) in &self.recipe_keep {
            for g in self
                .goods
                .iter()
                .filter(|g| g.group == Some(good) && g.id != good)
            {
                keep.entry(g.id).or_insert(count);
            }
        }
        for (&good, &count) in &self.recipe_keep {
            if !self
                .goods
                .iter()
                .any(|g| g.group == Some(good) && g.id != good)
            {
                keep.insert(good, count);
            }
        }
        keep
    }
}

impl RuleSet {
    /// Building types with a plot size (`bp_setSize`), for the simulation.
    /// Types without one (the construction-site entry) are left out.
    pub fn building_types(&self) -> Vec<BuildingType> {
        self.buildings
            .iter()
            .filter_map(|building| {
                let size = match building.size? {
                    model::BuildingSize::Small => BuildingSize::Small,
                    model::BuildingSize::Medium => BuildingSize::Medium,
                    model::BuildingSize::Large => BuildingSize::Large,
                    model::BuildingSize::Mine => BuildingSize::Mine,
                    model::BuildingSize::Harbor => BuildingSize::Harbor,
                };
                let amounts = |list: &[model::Amount]| {
                    list.iter().map(|a| (a.good, a.count)).collect::<Vec<_>>()
                };
                Some(BuildingType {
                    id: building.id,
                    size,
                    frame_costs: amounts(&building.frame_costs),
                    costs: amounts(&building.costs),
                    territory_range: building.territory_range.unwrap_or(0),
                    explore_radius: building.explore_radius.unwrap_or(0),
                    soldier_limit: building.soldier_limit.unwrap_or(0),
                    worker: building.worker,
                    // ECO-REQ-1.4: default 1.
                    worker_limit: building.worker_limit.unwrap_or(1),
                    warehouse: building.depot.is_some_and(|d| d != 0),
                    // ECO-REQ-4.1: only the sign matters; default 500.
                    instant: building.construction_time.is_some_and(|t| t <= 0.0),
                    product: building.goods.map(|a| a.good),
                    // ECO-WORK-2.1: a timed action lasts ceil(duration).
                    production_ticks: building
                        .production_time
                        .map_or(0, |t| t.max(0.0).ceil() as u32),
                    stock_limit: building.stock_limit,
                    // ECO-OUT-1.1: default 4.
                    working_radius: building.working_radius.unwrap_or(4),
                    cycle: work_cycle(self, building),
                    second_product: building.second_goods.map(|a| a.good),
                    switchable: building.switchable_production.unwrap_or(false),
                    catapult: building.catapult.unwrap_or(false),
                    planting_good: building.planting_good,
                    spawn: building.spawn.is_some_and(|s| s != 0),
                    construction_positions: construction_positions(building),
                    // ECO-CAT-2.4: the door offset per tribe.
                    doors: [
                        model::Tribe::Romans,
                        model::Tribe::Africans,
                        model::Tribe::Chinese,
                    ]
                    .map(|tribe| {
                        building
                            .door_directions
                            .iter()
                            .find(|&&(t, _)| t == tribe)
                            .map(|&(_, d)| d as u8)
                    }),
                    upgrade_to: self
                        .military
                        .building_upgrades
                        .iter()
                        .find(|&&(from, _)| from == building.id)
                        .map(|&(_, to)| to),
                })
            })
            .collect()
    }
}

impl RuleSet {
    /// What the simulation takes from the scripts, in its own units. Fails
    /// naming the first setting the scripts left unset.
    pub fn sim_rules(&self) -> Result<Rules, String> {
        let settings = &self.settings;
        let missing = |name: &str| format!("the scripts do not set {name}");
        // TRN-MOVE-1.1: progress per tick in millionths of a step.
        let speed = settings
            .settler_speed
            .ok_or_else(|| missing("cfg_setSettlerSpeed"))?;
        // Timed actions last ceil(duration) ticks (ECO-WORK-2.1).
        let ticks = |value: Option<f64>, name: &str| {
            value
                .map(|v| v.ceil().max(0.0) as u32)
                .ok_or_else(|| missing(name))
        };
        let settings = Settings {
            settler_speed: (speed * 1_000_000.0).round() as u32,
            pickup_ticks: ticks(settings.pickup_time, "cfg_setPickupTime")?,
            drop_ticks: ticks(settings.drop_time, "cfg_setDropTime")?,
            flag_package_limit: settings
                .flag_package_limit
                .ok_or_else(|| missing("cfg_setFlagPackageLimit"))?,
            bulldoze_ticks: ticks(settings.bulldoze_time, "cfg_setBulldozeTime")?,
            construct_ticks: ticks(settings.construct_time, "cfg_setConstructTime")?,
            fishing_distance: settings
                .fishing_distance
                .ok_or_else(|| missing("cfg_setFishingDistance"))?,
            // MIL-CON-5.4: whole ticks per unit; 11 in the shipped game.
            catapult_turn_ticks: settings
                .catapult_rotation_time_factor
                .map_or(11, |f| f.round().max(0.0) as u32),
            catapult_flight_ticks: settings
                .catapult_flight_time_factor
                .map_or(11, |f| f.round().max(0.0) as u32),
            // TRN-CARRY-7.1 (T31a): a doodad the scripts define.
            skeleton_doodad: settings
                .settler_skeleton_doodad
                .filter(|&d| self.doodads.contains(d)),
        };
        let transport_priorities = self
            .goods
            .iter()
            .filter_map(|good| Some((good.id, good.transport_priority?)))
            .collect();
        let start_stock = START_STOCK
            .iter()
            .map(|&(name, count)| {
                let good = self
                    .id_named(name)
                    .filter(|&id| self.goods.contains(id))
                    .ok_or_else(|| format!("the scripts define no good {name}"))?;
                Ok((good, count))
            })
            .collect::<Result<_, String>>()?;
        let good = |name: &str| self.id_named(name).filter(|&id| self.goods.contains(id));
        let goods = self
            .goods
            .iter()
            .map(|g| {
                let rules = s2_sim::GoodRules {
                    costs: g.costs.iter().map(|a| (a.good, a.count)).collect(),
                    // ECO-WORK-3.9: over when the counter is greater than
                    // the rest time.
                    rest_ticks: g.rest_time.map_or(0, |r| r.max(0.0).floor() as u32 + 1),
                    gathered: g.deposits_enabled.is_some_and(|d| d != 0),
                    resource: g.fields_enabled.is_some_and(|d| d != 0),
                    kind: match g.good_type {
                        // Q-033: a good without gp_setType is a ware.
                        Some(model::GoodType::Ware) | None => s2_sim::GoodKind::Ware,
                        Some(model::GoodType::Settler) => s2_sim::GoodKind::Settler,
                        Some(model::GoodType::Group) => s2_sim::GoodKind::Group,
                        Some(model::GoodType::Invalid) => s2_sim::GoodKind::Other,
                    },
                    // UI-GAME-8.5e: a good with a group counts as its group.
                    production_counter: PRODUCTION_COUNTERS
                        .iter()
                        .position(|&name| {
                            let counter = self.id_named(name);
                            counter.is_some() && counter == Some(g.group.unwrap_or(g.id))
                        })
                        .map(|counter| counter as u8),
                };
                (g.id, rules)
            })
            .collect();
        let roles = s2_sim::Roles {
            helper: good("good_helper"),
            builder: good("good_constructor"),
            leveller: good("good_bulldozer"),
            donkey: good("good_donkey"),
            coin: good("coin"),
            boat_carrier: good("good_shipHelper"),
        };
        Ok(Rules {
            // H6: a project's recipes (`reborn.bp_addInput`); none in the
            // original's scripts.
            recipes: self
                .buildings
                .iter()
                .filter(|b| !b.inputs.is_empty())
                .map(|b| (b.id, b.inputs.iter().map(|a| (a.good, a.count)).collect()))
                .collect(),
            // W20: a project's recipe sets (`reborn.bp_addRecipe`); none in
            // the original's scripts.
            // W16: the groups a recipe takes, with their members in good
            // order, and the stock kept back from them (a group's keep for
            // each member); none in the original's scripts.
            recipe_groups: self.recipe_groups(),
            recipe_keep: self.recipe_keep_by_member(),
            // W19: a project's paving rule (`reborn.cfg_setPaving`).
            paving: self
                .paving
                .map(|(good, per_field, speed_percent)| s2_sim::PavingRule {
                    good,
                    per_field,
                    speed_percent,
                }),
            // W15: a project's mine deepenings (`reborn.bp_setDeepening`).
            deepenings: self
                .buildings
                .iter()
                .filter_map(|b| b.deepening.as_ref().map(|d| (b.id, d)))
                .map(|(id, d)| {
                    let deepening = s2_sim::Deepening {
                        share_per_mille: d.share_per_mille,
                        most: d.most,
                        costs: d.costs.iter().map(|a| (a.good, a.count)).collect(),
                        food: d.food.clone(),
                    };
                    (id, deepening)
                })
                .collect(),
            recipe_sets: self
                .buildings
                .iter()
                .filter(|b| !b.recipes.is_empty())
                .map(|b| {
                    let set = b.recipes.iter().map(|r| s2_sim::Recipe {
                        product: r.product,
                        inputs: r.inputs.iter().map(|a| (a.good, a.count)).collect(),
                    });
                    (b.id, set.collect())
                })
                .collect(),
            building_types: self.building_types(),
            transport_priorities,
            settings,
            headquarters: self
                .id_named(HEADQUARTERS)
                .filter(|&id| self.buildings.contains(id)),
            start_stock,
            plank: good("plank"),
            stone: good("stone"),
            goods,
            roles,
            deposit_kinds: self
                .deposits
                .iter()
                .map(|d| DepositKind {
                    id: d.id,
                    good: d.good,
                    next: d.next_deposit,
                    // TRN-FREE-5.2: 1 for trees and small stones.
                    elevation: d
                        .elevation_size
                        .map_or(1, |e| u32::try_from(e).unwrap_or(1)),
                    grow_ticks: grow_ticks(
                        d.growing_start_size.unwrap_or(GROWING_START),
                        d.growing_speed.unwrap_or(GROWING_SPEED),
                    ),
                    blocking: d.blocking.unwrap_or(false),
                    // TRN-FREE-5.4: default on.
                    random_offset: d.random_fine_pos_offset.unwrap_or(true),
                    life_time: d
                        .life_time
                        .filter(|&t| t > 0)
                        .map(|t| u32::try_from(t).unwrap_or(u32::MAX)),
                    needs_plantable: d.needs_plantable_pattern.unwrap_or(false),
                })
                .collect(),
            tree_families: TREE_FAMILIES
                .iter()
                .map(|names| names.iter().filter_map(|n| self.deposit_named(n)).collect())
                .collect(),
            default_tree: self.deposit_named(DEFAULT_TREE),
            production: s2_sim::ProductionRules {
                mine_food: MINE_FOOD.iter().filter_map(|n| good(n)).collect(),
                tool_slots: TOOL_SLOTS.iter().filter_map(|n| good(n)).collect(),
                tools_group: good(TOOLS_GROUP),
            },
            ship: self.ship_rules(&settings),
            distribution: DISTRIBUTION
                .iter()
                .map(|&(goods, entries)| {
                    let goods: Vec<u32> = goods.iter().filter_map(|n| good(n)).collect();
                    let mut entries: Vec<(u32, u32)> = entries
                        .iter()
                        .filter_map(|&(n, w)| Some((self.id_named(n)?, w)))
                        .collect();
                    // A project's consumers of the table's goods
                    // (`reborn.dist_addConsumer`), after the original's, once
                    // each; none in the original's scripts.
                    for &(g, building, weight) in &self.dist_consumers {
                        if goods.contains(&g) && !entries.iter().any(|&(k, _)| k == building) {
                            entries.push((building, weight));
                        }
                    }
                    s2_sim::DistributionRules { goods, entries }
                })
                .collect(),
            construction: self.id_named(CONSTRUCTION),
            specialists: self.specialist_rules(),
            animals: self
                .animals
                .iter()
                .map(|a| {
                    let mut kind = s2_sim::AnimalKind::new(a.id);
                    // -1 when `an_setRange` is not called.
                    kind.range = a.range.map_or(-1, |r| r as i32);
                    kind.move_distance = a.move_distance.unwrap_or(kind.move_distance);
                    if let Some(speed) = a.movement_speed {
                        kind.speed = (speed * 1_000_000.0).round() as u32;
                    }
                    kind.avoid_buildings = a.avoid_buildings.unwrap_or(true);
                    if let Some(f) = a.idle_time_factor {
                        kind.idle_thousandths = (f * 1000.0).round().max(0.0) as u32;
                    }
                    kind
                })
                .collect(),
            military: s2_sim::MilitaryRules {
                soldiers: self.military.soldiers,
                max_health: self
                    .military
                    .soldier_max_health
                    .map(|h| h.map_or(0, |h| u32::try_from(h).unwrap_or(0))),
                fighting_ticks: self
                    .military
                    .fighting_time
                    .map_or(0, |t| t.max(0.0).ceil() as u32),
            },
        })
    }

    /// ECO-SPC: the geologist's and scout's professions, the sign table
    /// (`si_addSignDoodadId`) with each sign doodad's lifetime, and the
    /// message for each good a geologist reports (ECO-SPC-3.3).
    fn specialist_rules(&self) -> s2_sim::SpecialistRules {
        let good = |name: &str| self.id_named(name).filter(|&id| self.goods.contains(id));
        s2_sim::SpecialistRules {
            geologist: good("good_scholar"),
            scout: good("good_explorer"),
            water: good("water"),
            signs: self
                .signs
                .iter()
                .map(|s| s2_sim::SignRule {
                    good: s.good,
                    threshold: s.threshold,
                    doodad: s.doodad,
                    life_time: self
                        .doodads
                        .get(s.doodad)
                        .and_then(|d| d.life_time)
                        .filter(|&t| t > 0)
                        .map(|t| u32::try_from(t).unwrap_or(u32::MAX)),
                })
                .collect(),
            found: FOUND_MESSAGES
                .iter()
                .filter_map(|&(name, text)| Some((good(name)?, text)))
                .collect(),
        }
    }

    /// ECO-FOOD-7.4: planks of the ship's frame and main part and the ticks
    /// of one work step.
    fn ship_rules(&self, settings: &Settings) -> s2_sim::ShipRules {
        let plank = self.id_named("plank");
        let planks = |list: &[model::Amount]| {
            list.iter()
                .filter(|a| Some(a.good) == plank)
                .map(|a| a.count)
                .sum()
        };
        s2_sim::ShipRules {
            // SHIP-3.1: progress per tick, in millionths of a step.
            speed: self
                .ship
                .speed
                .map_or(0, |v| (v.max(0.0) * 1_000_000.0).round() as u32),
            stock_limit: self.ship.stock_limit.unwrap_or(0),
            frame_planks: planks(&self.ship.frame_costs),
            main_planks: planks(&self.ship.costs),
            // ECO-FOOD-7.4: the ship's own construct time, else the
            // construction time.
            construct_ticks: self
                .ship
                .construct_time
                .map_or(settings.construct_ticks, |t| t.ceil().max(0.0) as u32),
        }
    }

    /// The id of the deposit type with this name.
    pub fn deposit_named(&self, name: &str) -> Option<u32> {
        self.deposits
            .iter()
            .find(|d| d.name.as_deref() == Some(name))
            .map(|d| d.id)
    }

    /// ECO-OUT-4.3, 4.5: the forester's view of each field's terrain.
    pub fn ground(&self, map: &Map) -> Vec<Ground> {
        map.patterns
            .iter()
            .map(|&id| match self.patterns.get(id) {
                Some(p) => Ground {
                    plantable: !p.for_mining.unwrap_or(false) && !p.for_harbor.unwrap_or(false),
                    for_planting: p.for_planting.unwrap_or(false),
                    tree_type: p.tree_type.map(|t| t as u8),
                },
                None => Ground::default(),
            })
            .collect()
    }

    /// The map's deposits with the ticks each still needs to be ripe
    /// (ECO-OUT-6.1: 1.0 is fully grown).
    pub fn map_deposits(&self, map: &Map) -> Vec<MapDeposit> {
        map.deposits
            .iter()
            .map(|d| {
                let speed = self
                    .deposits
                    .get(d.type_id)
                    .and_then(|k| k.growing_speed)
                    .unwrap_or(GROWING_SPEED);
                let ripe_in = if d.size >= 1.0 {
                    0
                } else {
                    grow_ticks(f64::from(d.size), speed)
                };
                MapDeposit {
                    field: (d.field.x, d.field.y),
                    kind: d.type_id,
                    position: (d.position.x, d.position.y),
                    ripe_in,
                }
            })
            .collect()
    }
}

/// A new game on `map` (ECO-HQ): one player per start position, player `i`
/// on start position `i`, with the start resources setting
/// `start_resources` (0 to 3). Player 0 is the local player.
pub fn new_game(
    seed: u64,
    map: &Map,
    rules: &RuleSet,
    start_resources: u8,
) -> Result<World, String> {
    Ok(World::new_game(
        seed,
        map.terrain(),
        rules.sim_rules()?,
        &game_setup(map, rules, start_resources),
    ))
}

/// A player of a game as the start gives it: its entry, the start position
/// it plays from (UI-SETUP-2.1), tribe, colour and team, and its AI profile if
/// a computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupPlayer {
    pub player: PlayerId,
    pub start: usize,
    pub tribe: u8,
    /// FMT-S2M-2.6: 0..7, its own index unless a map says otherwise; two
    /// players may share one.
    pub colour: u8,
    pub team: i8,
    pub profile: Option<u8>,
}

/// UI-SETUP-2.1: the setup of a game with these players, each on its entry's
/// start position, with its tribe and team.
pub fn setup_of_players(
    map: &Map,
    rules: &RuleSet,
    start_resources: u8,
    players: &[SetupPlayer],
) -> GameSetup {
    let mut setup = game_setup(map, rules, start_resources);
    let positions = std::mem::take(&mut setup.players);
    setup.players = players
        .iter()
        .filter_map(|s| positions.get(s.start).map(|&(_, at)| (s.player, at)))
        .collect();
    setup.tribes = players.iter().map(|s| (s.player, s.tribe as i8)).collect();
    setup.teams = players.iter().map(|s| (s.player, s.team)).collect();
    setup
}

/// The setup of [`new_game`]: players on the map's start positions, its
/// deposits, ground types and resources.
pub fn game_setup(map: &Map, rules: &RuleSet, start_resources: u8) -> GameSetup {
    let players = map
        .info
        .start_positions
        .iter()
        // FMT-S2M-2.2: the original keeps at most the first 6.
        .take(6)
        .enumerate()
        .map(|(i, p)| (PlayerId(i as u8), (p.x, p.y)))
        .collect();
    GameSetup {
        players,
        start_resources,
        local_player: PlayerId(0),
        deposits: rules.map_deposits(map),
        ground: rules.ground(map),
        resources: map
            .resources
            .iter()
            .map(|r| {
                (r.good != s2m::NO_GOOD && r.amount > 0).then(|| s2_sim::FieldResource {
                    good: r.good,
                    amount: u8::try_from(r.amount).unwrap_or(u8::MAX),
                })
            })
            .collect(),
        // TRN-FREE-5.5b, Q-051: every player is a Roman until the setup
        // says otherwise.
        tribes: Vec::new(),
        // MIL-SOL-3.1: no teams, everyone is an enemy.
        teams: Vec::new(),
        // SHIP-1.1: the positions the map lists on its land continents.
        // SIM-START-3.1: each player on its own start position.
        shuffle_starts: false,
        // ECO-HQ-3.6: fog of war On, as a new game opens.
        fog_of_war_off: false,
        // SIM-START-4.5: the counter is set to the map file's next id
        // (FMT-S2M-3.1), its low 32 bits.
        next_object: map.logic.next_unique_id as u32,
        // FMT-S2M-5.5: accumulator, rate, then the cell cursor's x and y.
        respawn: s2_sim::Respawn {
            accumulator: map.animal_respawn.counter,
            rate: map.animal_respawn.target_population,
            cursor: (
                u32::try_from(map.animal_respawn.timer[0]).unwrap_or(0),
                u32::try_from(map.animal_respawn.timer[1]).unwrap_or(0),
            ),
        },
        // FMT-S2M-4.9: the stored continent layer, as loaded, and the
        // positions of the land continents.
        continents: map.continent_of.clone(),
        land_spots: map
            .continents
            .iter()
            .filter(|c| !c.is_water)
            .map(|c| {
                (
                    c.index as i32,
                    c.positions.iter().map(|p| (p.x, p.y)).collect(),
                )
            })
            .collect(),
        harbour_spots: map
            .continents
            .iter()
            .filter(|c| !c.is_water)
            .flat_map(|c| c.positions.iter().map(|p| (p.x, p.y)))
            .collect(),
        // FMT-S2M-5.4: each animal on the field it is on.
        animals: map
            .animals
            .iter()
            .map(|a| {
                let field = |p: &s2m::FieldPos| (p.x, p.y);
                // ECO-FOOD-5.13: the record's state and counter; a walking
                // animal's path is stored destination first, entry `index`
                // the step in progress (FMT-S2M-5.4).
                let walk = a
                    .path
                    .as_ref()
                    .filter(|_| a.counters[0] == 2)
                    .and_then(|p| {
                        let index = p.index as usize;
                        let step = field(p.fields.get(index)?);
                        let rest = p.fields[..index].iter().rev().map(field).collect();
                        let progress = a.progress.max(0.0).to_bits();
                        Some((step, progress, rest))
                    });
                s2_sim::MapAnimal {
                    kind: a.type_id,
                    field: field(&a.position),
                    home: field(&a.home),
                    state: a.counters[0],
                    counter: a.counters[1],
                    walk,
                }
            })
            .collect(),
    }
}

/// FMT-SAV-2, 3: a game continued from a save read by
/// [`s2_assets::sav`]: the world of its map part with no headquarters
/// placed, then the saved buildings, flags, roads, territory and
/// exploration, the workers the buildings name, the carriers and donkeys
/// the streets name with their numbers, the packages with their
/// destinations and orders, the ships being built and the statistics
/// (`World::restore`). Other settlers are not taken over (Q-204, Q-257).
pub fn restore_game(
    save: &s2_assets::sav::SaveGame,
    rules: &RuleSet,
) -> Result<(World, s2_sim::RestoreReport), String> {
    let map = save
        .map
        .as_ref()
        .ok_or("the save's map part did not read")?;
    let mut setup = game_setup(map, rules, 0);
    setup.players.clear();
    let sim = rules.sim_rules()?;
    let mut world = World::new_game(s2_sim::rng::SIM_SEED as u64, map.terrain(), sim, &setup);
    let point = |f: s2m::FieldPos| (f.x, f.y);
    let player = |p: u32| PlayerId(u8::try_from(p).unwrap_or(u8::MAX));
    // FMT-SAV-3.5: the ids of the workers and carriers lists, which the
    // buildings' and streets' refs name (FMT-SAV-3.4a, 3.9).
    let ids = |list: usize| -> std::collections::HashSet<u64> {
        save.settlers[list].iter().map(|s| s.id).collect()
    };
    let (workers, carriers) = (ids(0), ids(2));
    // FMT-SAV-3.5a (b, c): a named worker in state 3 is going home, not his
    // building's worker any more.
    let home_bound: std::collections::HashSet<u64> = save.settlers[0]
        .iter()
        .filter(|s| s.state == 3)
        .map(|s| s.id)
        .collect();
    let workers: std::collections::HashSet<u64> = workers
        .into_iter()
        .filter(|w| !home_bound.contains(w))
        .collect();
    // FMT-SAV-3.5c: a soldier's rank is its profession id (3.5a), its health and
    // healing counter its `SettlersLivePoints`.
    let soldiers: std::collections::HashMap<u64, s2_sim::RestoredSoldier> = save.settlers[4]
        .iter()
        .filter_map(|s| {
            let points = s.live_points?;
            Some((
                s.id,
                s2_sim::RestoredSoldier {
                    number: Some(s.id as u32),
                    profession: s.profession,
                    health: points.health.round().max(0.0) as u32,
                    healing: points.healing,
                },
            ))
        })
        .collect();
    let cells = map.territory.len();
    let mut explored = vec![0u8; cells];
    for (index, layer) in map.exploration.iter().enumerate().take(8) {
        for (bits, &cell) in explored.iter_mut().zip(layer) {
            if cell > 0 {
                *bits |= 1 << index;
            }
        }
    }
    let restore = s2_sim::Restore {
        players: save
            .players
            .iter()
            .filter(|p| p.active)
            .map(|p| player(p.index))
            .collect(),
        tick: u64::try_from(map.logic.tick).unwrap_or(0),
        owners: map
            .territory
            .iter()
            .map(|&o| u8::try_from(o).ok().map(PlayerId))
            .collect(),
        explored,
        buildings: save
            .buildings
            .iter()
            .map(|b| s2_sim::RestoredBuilding {
                player: player(b.owner),
                building_type: b.building_type,
                at: point(b.field),
                // FMT-SAV-3.4b: the file has the main part first:
                // wherever the first is below 1, the second is 0.
                remaining: (b.construction_left != [0.0, 0.0])
                    .then_some([b.construction_left[1], b.construction_left[0]]),
                levelled: b.levelled,
                delivered: b.delivered.clone(),
                production_on: b.production_on,
                stock: b.stock.clone(),
                worker: b.workers.iter().any(|w| workers.contains(w)),
                // FMT-SAV-2.2, SIM-START-4.1: their own ids, kept.
                number: Some(b.id as u32),
                worker_number: b
                    .workers
                    .iter()
                    .find(|w| workers.contains(w))
                    .map(|&w| w as u32),
                soldiers: b
                    .soldiers
                    .iter()
                    .filter_map(|id| soldiers.get(id).copied())
                    .collect(),
            })
            .collect(),
        flags: save
            .flags
            .iter()
            .map(|f| (player(f.owner), point(f.field)))
            .collect(),
        roads: save
            .streets
            .iter()
            .map(|s| s2_sim::RestoredRoad {
                player: player(s.owner),
                path: s.fields.iter().copied().map(point).collect(),
                carrier: carriers.contains(&s.carriers[0]),
                donkey: carriers.contains(&s.carriers[1]),
                // FMT-SAV-2.2, SIM-START-4.1: their own ids, kept.
                numbers: s
                    .carriers
                    .map(|id| carriers.contains(&id).then_some(id as u32)),
                // FMT-SAV-3.5h (Q-257 a): where each stands.
                standing: s.carriers.map(|id| {
                    save.settlers[2]
                        .iter()
                        .find(|c| c.id == id)
                        .and_then(|c| c.movement.field)
                        .map(point)
                }),
            })
            .collect(),
        wares: restored_wares(save),
        ships: save
            .saved_ships
            .iter()
            .filter(|s| s.main_left > 0.0 || s.frame_left > 0.0)
            .filter_map(|s| {
                Some(s2_sim::RestoredShip {
                    player: player(s.owner),
                    coast: point(s.coast?),
                    water: point(s.water?),
                    frame_left: s.frame_left,
                    main_left: s.main_left,
                })
            })
            .collect(),
        random: save.random.map(random_words),
        // FMT-SAV-2.2: `LogicSystem`'s counter.
        next_object: Some(map.logic.next_unique_id as u32),
        // FMT-SAV-3.11: the players' statistics, slot by slot.
        statistics: save
            .statistics
            .iter()
            .enumerate()
            .map(|(slot, series)| (player(slot as u32), series.clone()))
            .collect(),
        // FMT-SAV-3.11: the coins produced and the frags so far.
        totals: save
            .statistic_totals
            .iter()
            .enumerate()
            .map(|(slot, t)| (player(slot as u32), [t[0], t[1]]))
            .collect(),
        // FMT-SAV-3.11: the productivity decay count.
        productivity_count: save.statistics_cycle,
        products: saved_products(save),
        homeward: saved_homeward(save),
        builders: saved_builders(save),
        workers_on_way: saved_workers_on_way(save),
        // FMT-SAV-3.5a (e, Q-1033 answered): the running units go on.
        builder_units: saved_builder_units(save),
        workers_back: saved_workers_back(save),
        workers_working: saved_workers_working(save),
        workers_outdoors: saved_workers_outdoors(save),
        workers_waiting: saved_workers_waiting(save),
        builder_walks: saved_builder_walks(save),
        // FMT-SAV-3.7a (T17): the fights in progress.
        fights: saved_fights(save),
        // FMT-SAV-3.8 (T12): the finished ships (no work left on either
        // part), on their water field.
        finished_ships: save
            .saved_ships
            .iter()
            .filter(|s| s.main_left <= 0.0 && s.frame_left <= 0.0)
            .filter_map(|s| {
                let water = point(s.water?);
                let building = |id: Option<u64>| {
                    let id = id?;
                    save.buildings
                        .iter()
                        .find(|b| b.id == id)
                        .map(|b| point(b.field))
                };
                Some(s2_sim::RestoredFinishedShip {
                    player: player(s.owner),
                    coast: s.coast.map(point).unwrap_or(water),
                    water,
                    state: s.state,
                    counter: s.counter,
                    store: s.store.clone(),
                    expedition: s.expedition,
                    port: s.port.map(point),
                    harbour: building(s.harbour),
                    called: building(s.sent_to),
                })
            })
            .collect(),
    };
    let report = world.restore(&restore);
    Ok((world, report))
}

/// FMT-SAV-3.7a, 3.5c (T17): the fights in progress: the duels (the second
/// list, in its order), then the pairings not yet fighting (the first); each
/// record's attacker and opponent by their saved numbers, with their home
/// buildings' fields (their `home` links), their fields (3.5h) and the
/// attacker's state number; an opponent named in the third list came out
/// as an interceptor. A record whose soldier, home or target is not in the
/// save is left out.
pub fn saved_fights(save: &s2_assets::sav::SaveGame) -> Vec<s2_sim::RestoredFight> {
    let point = |f: s2m::FieldPos| (f.x, f.y);
    let building = |id: Option<u64>| {
        let id = id?;
        save.buildings
            .iter()
            .find(|b| b.id == id)
            .map(|b| point(b.field))
    };
    let soldier = |id: Option<u64>| {
        let id = id?;
        save.settlers[4].iter().find(|s| s.id == id)
    };
    let duels = save.fight_duels.iter().map(|f| (f, true));
    let pairings = save.fight_pairings.iter().map(|f| (f, false));
    duels
        .chain(pairings)
        .filter_map(|(f, fighting)| {
            let a = soldier(f.attacker)?;
            let opponent = match f.opponent {
                Some(_) => {
                    let o = soldier(f.opponent)?;
                    Some(s2_sim::RestoredOpponent {
                        number: o.id as u32,
                        home: building(o.soldier?.home)?,
                        at: point(o.movement.field?),
                        intercept: save
                            .fight_defenders
                            .iter()
                            .any(|d| d.interceptor == Some(o.id)),
                    })
                }
                None => None,
            };
            Some(s2_sim::RestoredFight {
                attacker: a.id as u32,
                attacker_home: building(a.soldier?.home)?,
                attacker_at: point(a.movement.field?),
                attacker_sample: a.movement.fine_at,
                attacker_state: a.state,
                target: building(f.target)?,
                counter: f.counter,
                field: f.waiting.map(point),
                hit: f.hit_pending,
                fighting,
                opponent,
            })
        })
        .collect()
}

/// FMT-SAV-3.6, 3.9: the wares lying at the flags: each flag's `Package
/// Container` names packages of the `Package System`, whose good id is read;
/// in the flag's order: those lying there (`lying` 1); a flag's list also
/// names the packages carried to it (FMT-SAV-3.6).
pub fn saved_wares(save: &s2_assets::sav::SaveGame) -> Vec<(PlayerId, (u32, u32), u32)> {
    let lying: std::collections::BTreeMap<u64, u32> = save
        .packages
        .iter()
        .filter(|p| p.lying)
        .map(|p| (p.id, p.good))
        .collect();
    let player = |p: u32| PlayerId(u8::try_from(p).unwrap_or(u8::MAX));
    save.flags
        .iter()
        .flat_map(|f| {
            f.packages
                .iter()
                .filter_map(|id| lying.get(id))
                .map(move |&good| (player(f.owner), (f.field.x, f.field.y), good))
        })
        .collect()
}

/// FMT-SAV-3.6 (Q-1022, Q-1023): the save's packages for `World::restore`:
/// each flag's list in its order (lying = 1 there, lying = 0 in the hands
/// of the carrier its carrier ref names, carried to that flag), then the
/// packages in carriers' hands with no flag place. Each keeps its
/// destination: a warehouse pair's (`PackageNeedTarget`) to be stored, else
/// its destination building with the order (`BuildingNeedGood`) it serves
/// and that order's stamp. The owner is the flag's, else the carrier's
/// street's, else the destination's.
/// FMT-SAV-3.5a (T10): the packages in hands (`lying` 0) whose carrier is a
/// worker a building names: that building's field and the good.
pub fn saved_products(save: &s2_assets::sav::SaveGame) -> Vec<((u32, u32), u32)> {
    let named: std::collections::HashMap<u64, (u32, u32)> = save
        .buildings
        .iter()
        .flat_map(|b| b.workers.iter().map(move |&w| (w, (b.field.x, b.field.y))))
        .collect();
    let workers: std::collections::HashSet<u64> = save.settlers[0].iter().map(|s| s.id).collect();
    save.packages
        .iter()
        .filter(|p| !p.lying && workers.contains(&p.carrier))
        .filter_map(|p| Some((*named.get(&p.carrier)?, p.good)))
        .collect()
}

/// FMT-SAV-3.5a (e): the constructors that name a site of the save, on
/// their way to it (state 4) or at it (32): the site's field and their saved
/// number.
pub fn saved_builders(save: &s2_assets::sav::SaveGame) -> Vec<((u32, u32), u32)> {
    let sites: std::collections::HashMap<u64, (u32, u32)> = save
        .buildings
        .iter()
        .filter(|b| b.construction_left != [0.0, 0.0])
        .map(|b| (b.id, (b.field.x, b.field.y)))
        .collect();
    save.settlers[1]
        .iter()
        .filter(|s| matches!(s.state, 4 | 32))
        .filter_map(|s| Some((*sites.get(&s.home?)?, s.id as u32)))
        .collect()
}

/// FMT-SAV-3.5a (e, Q-1033 answered 2026-11-08 c): the constructors at a
/// site of the save (state 32) in a running unit (work step 10): the site's
/// field, c (construction positions ordered so far: at or walking to
/// position c - 1 of TRN-FREE-5.5) and r (ticks left of the 55-tick step).
pub fn saved_builder_units(save: &s2_assets::sav::SaveGame) -> Vec<((u32, u32), u32, u32)> {
    let sites: std::collections::HashMap<u64, (u32, u32)> = save
        .buildings
        .iter()
        .filter(|b| b.construction_left != [0.0, 0.0])
        .map(|b| (b.id, (b.field.x, b.field.y)))
        .collect();
    save.settlers[1]
        .iter()
        .filter(|s| s.state == 32)
        .filter_map(|s| {
            let [ordered, left, step] = s.site_work?;
            (step == 10).then_some((*sites.get(&s.home?)?, ordered, left))
        })
        .collect()
}

/// FMT-SAV-3.5a (b, 2026-11-08 b): the named workers on the land coming back
/// to their building (job states 36 and 37, off its field): the building's
/// field and the worker's current field.
pub fn saved_workers_back(save: &s2_assets::sav::SaveGame) -> Vec<((u32, u32), (u32, u32))> {
    let workers: std::collections::HashMap<u64, &s2_assets::sav::SavedSettler> =
        save.settlers[0].iter().map(|s| (s.id, s)).collect();
    let point = |f: s2m::FieldPos| (f.x, f.y);
    save.buildings
        .iter()
        .filter_map(|b| {
            let worker = b.workers.iter().find_map(|w| workers.get(w))?;
            let field = point(worker.movement.field.or(worker.movement.rough[2])?);
            let at = point(b.field);
            (matches!(worker.state, 36 | 37) && field != at).then_some((at, field))
        })
        .collect()
}

/// FMT-SAV-3.5a (b, 2026-11-09, Q-257): the named workers producing inside
/// (job state 33 on their building's field, a work loop running: animation
/// code 4 or 5 with ticks left): the building's field and the ticks left,
/// after which the action ends (the Fight autosave's 5, at 323, 312, 291, 112
/// and 103 of 333). One standing on his building's door field comes with the
/// workers out on the land ([`saved_workers_outdoors`]) and is found inside
/// there.
pub fn saved_workers_working(save: &s2_assets::sav::SaveGame) -> Vec<((u32, u32), u32)> {
    let workers: std::collections::HashMap<u64, &s2_assets::sav::SavedSettler> =
        save.settlers[0].iter().map(|s| (s.id, s)).collect();
    let point = |f: s2m::FieldPos| (f.x, f.y);
    save.buildings
        .iter()
        .filter_map(|b| {
            let worker = b.workers.iter().find_map(|w| workers.get(w))?;
            let field = point(worker.movement.field.or(worker.movement.rough[2])?);
            let at = point(b.field);
            let animation = worker.animation;
            let left = animation.left.round();
            (worker.state == 33 && field == at && matches!(animation.code, 4 | 5) && left >= 1.0)
                .then_some((at, left as u32))
        })
        .collect()
}

/// FMT-SAV-3.5a (b, 2026-11-11, E-sav-014; 2026-11-09 c), 3.5b: the named
/// workers in their job's main state (33) whose harvest state (the second
/// of the twelve) or hunter state (the last) is at step 7, waiting inside
/// for a target: the building's field, each state's countdown left
/// (ECO-OUT-3.2) where its step is 7, and the field of the building's
/// reserved deposit (3.4b 1). Both states keep their counters between uses;
/// the simulation takes the one the building's cycle runs. The 2023 Coastal
/// Storm sample's farmers 110877, 203134, 211736, 216019 and 256071 wait
/// with 27, 34, 4, 45 and 35 left, its woodcutters 104042, 228464, 253582
/// and 271583 (2026-11-12, E-sav-016: the same state) with 24, 28, 9, 35.
pub fn saved_workers_waiting(save: &s2_assets::sav::SaveGame) -> Vec<s2_sim::RestoredWaiting> {
    let workers: std::collections::HashMap<u64, &s2_assets::sav::SavedSettler> =
        save.settlers[0].iter().map(|s| (s.id, s)).collect();
    let point = |f: s2m::FieldPos| (f.x, f.y);
    let deposits: std::collections::HashMap<u64, (u32, u32)> = save
        .map
        .iter()
        .flat_map(|m| m.deposits.iter().map(|d| (d.id, point(d.field))))
        .collect();
    let left = |h: &s2_assets::sav::SavedHunt| {
        (h.step == 7 && h.countdown >= 0.0).then(|| h.countdown.round() as u32)
    };
    save.buildings
        .iter()
        .filter_map(|b| {
            let worker = b.workers.iter().find_map(|w| workers.get(w))?;
            if worker.state != 33 {
                return None;
            }
            let harvest = worker.harvest.as_ref().and_then(left);
            let hunt = worker.hunt.as_ref().and_then(left);
            (harvest.is_some() || hunt.is_some()).then(|| s2_sim::RestoredWaiting {
                at: point(b.field),
                harvest,
                hunt,
                target: b.reserved.and_then(|r| deposits.get(&r).copied()),
            })
        })
        .collect()
}

/// FMT-SAV-3.5h, 3.5a (2026-11-09, Q-257): a settler's walk on the fine
/// lattice: the fine flag set and a fine path (from the target back to the
/// start, the index naming the running step's goal) with a step under way.
fn saved_fine_walk(s: &s2_assets::sav::SavedSettler) -> Option<s2_sim::RestoredFineWalk> {
    let m = &s.movement;
    (m.fine_on && m.fine_path.len() >= 2 && m.progress >= 0.0).then(|| s2_sim::RestoredFineWalk {
        path: m.fine_path.clone(),
        index: m.fine_index,
        progress: m.progress,
        rate: m.per_tick,
    })
}

/// FMT-SAV-3.5a (b, 2026-11-09 c), 3.5h: a settler's rough walk on a free
/// path (`Path` kind 1, its `Movement Path Base` active) with a step under
/// way: the fields from the target back, the running step's index, its
/// progress and rate, and the stop-short count.
fn saved_rough_walk(m: &s2_assets::sav::SavedMovement) -> Option<s2_sim::RestoredRoughWalk> {
    let usable = m.path_kind == 1 && m.free_path.len() >= 2 && m.progress >= 0.0;
    usable.then(|| s2_sim::RestoredRoughWalk {
        path: m.free_path.iter().map(|f| (f.x, f.y)).collect(),
        index: m.free_index,
        progress: m.progress,
        rate: m.per_tick,
        stop_short: m.stop_short,
    })
}

/// FMT-SAV-3.5a (b, 2026-11-09, Q-257): the named workers out on the land in
/// their job's main state (33, 34; off their building's field): working at
/// their spot (a work loop, code 4 or 5, with ticks left) or walking their
/// fine approach to it (no animation, a fine walk). Their field is
/// `Movement Rough`'s current field (where the walk out ended and the fine
/// approach starts; the closing field follows the fine cursor), their spot
/// the rough walk's target (3.5h). The Fight autosave's: a forester planting
/// (2 ticks left), a woodcutter cutting (52), a shipwright at his ship (43);
/// a forester and a woodcutter on their approach.
pub fn saved_workers_outdoors(save: &s2_assets::sav::SaveGame) -> Vec<s2_sim::RestoredOutdoor> {
    let workers: std::collections::HashMap<u64, &s2_assets::sav::SavedSettler> =
        save.settlers[0].iter().map(|s| (s.id, s)).collect();
    // (2026-11-09 c): a hunter's animal, the building's reserved target
    // (3.4b 1), by its object id in the map part's animal list (FMT-S2M-5.4).
    let animals: std::collections::HashMap<u64, usize> = save
        .map
        .iter()
        .flat_map(|m| m.animals.iter().enumerate().map(|(i, a)| (a.id, i)))
        .collect();
    let point = |f: s2m::FieldPos| (f.x, f.y);
    // (2026-11-11): a building's reserved target among the map part's
    // deposits (trees, stone piles, grain; FMT-S2M-5.2), by its object id.
    let deposits: std::collections::HashMap<u64, (u32, u32)> = save
        .map
        .iter()
        .flat_map(|m| m.deposits.iter().map(|d| (d.id, point(d.field))))
        .collect();
    save.buildings
        .iter()
        .filter_map(|b| {
            let worker = b.workers.iter().find_map(|w| workers.get(w))?;
            let m = &worker.movement;
            let at = point(b.field);
            // (2026-11-11, E-sav-014): a farmer going out to harvest (harvest
            // step 8): on the door step onto the farm's field (leg 1) or the
            // walk to its flag (leg 0), the farm's reserved target the grain
            // he goes to after it (3.4b 1).
            if let Some(out) = worker.harvest.filter(|h| worker.state == 33 && h.step == 8) {
                let rough = (!m.fine_on).then(|| saved_rough_walk(m)).flatten()?;
                let field = *rough.path.get(rough.index as usize + 1)?;
                return Some(s2_sim::RestoredOutdoor {
                    at,
                    field,
                    spot: *deposits.get(&b.reserved?)?,
                    state: worker.state,
                    left: None,
                    walk: None,
                    rough: Some(rough),
                    animal: None,
                    leg: Some(out.walk_step),
                });
            }
            let here = point(m.field.or(m.rough[2])?);
            if !matches!(worker.state, 33 | 34) || here == at {
                return None;
            }
            let animation = worker.animation;
            let left = animation.left.round();
            let working = matches!(animation.code, 4 | 5) && left >= 1.0;
            let walk = (animation.code == 0)
                .then(|| saved_fine_walk(worker))
                .flatten();
            // (2026-11-09 c): no fine flag and an active free path: the rough
            // part of his walk out.
            let rough = (!working && walk.is_none() && !m.fine_on)
                .then(|| saved_rough_walk(m))
                .flatten();
            if !working && walk.is_none() && rough.is_none() {
                return None;
            }
            // (2026-11-09 c, from code): a hunter is taken in hunt step 10
            // (working at his animal) and 9 (walking to it: walk step 0 the
            // free walk, 1 the lattice walk); waiting inside (7) or on his
            // way to the flag (8) he is seated.
            let animal = b.reserved.and_then(|r| animals.get(&r).copied());
            if animal.is_some() {
                let hunt = worker.hunt?;
                let fits = match hunt.step {
                    10 => working,
                    9 => match hunt.walk_step {
                        0 => rough.is_some(),
                        1 => walk.is_some(),
                        _ => false,
                    },
                    _ => false,
                };
                fits.then_some(())?;
            }
            Some(s2_sim::RestoredOutdoor {
                at,
                field: point(m.rough[2]?),
                spot: point(m.rough[3]?),
                state: worker.state,
                left: working.then_some(left as u32),
                walk: if working { None } else { walk },
                rough,
                animal,
                leg: None,
            })
        })
        .collect()
}

/// FMT-SAV-3.5a (e, 2026-11-09, Q-257): the constructors at a site (state
/// 32) walking on the fine lattice: approaching it (step 13, no unit
/// started) or walking to a construction position in a running unit (step
/// 10 with r = 55): the site's field, the walk and the work state's first
/// number (Q-1037: at step 13 the approach's leg, 1 out, 2 back).
pub fn saved_builder_walks(
    save: &s2_assets::sav::SaveGame,
) -> Vec<((u32, u32), s2_sim::RestoredFineWalk, u32)> {
    let sites: std::collections::HashMap<u64, (u32, u32)> = save
        .buildings
        .iter()
        .filter(|b| b.construction_left != [0.0, 0.0])
        .map(|b| (b.id, (b.field.x, b.field.y)))
        .collect();
    save.settlers[1]
        .iter()
        .filter(|s| s.state == 32)
        .filter_map(|s| {
            let [_, left, step] = s.site_work?;
            let walking = step == 13 || (step == 10 && left == 55);
            walking.then_some(())?;
            Some((*sites.get(&s.home?)?, saved_fine_walk(s)?, s.site_leg?))
        })
        .collect()
}

/// FMT-SAV-3.5a (b): the named workers on their way to their building
/// (state 4): the building's field, the worker's current field and the goal
/// of his step in progress (3.5h, `Movement Rough`'s second cursor).
#[allow(clippy::type_complexity)]
pub fn saved_workers_on_way(
    save: &s2_assets::sav::SaveGame,
) -> Vec<((u32, u32), (u32, u32), Option<(u32, u32)>)> {
    let workers: std::collections::HashMap<u64, &s2_assets::sav::SavedSettler> =
        save.settlers[0].iter().map(|s| (s.id, s)).collect();
    let point = |f: s2m::FieldPos| (f.x, f.y);
    save.buildings
        .iter()
        .filter_map(|b| {
            let worker = b.workers.iter().find_map(|w| workers.get(w))?;
            let field = worker.movement.field.or(worker.movement.rough[2])?;
            let next = worker.movement.rough[1].filter(|_| worker.movement.progress >= 0.0);
            (worker.state == 4).then_some((point(b.field), point(field), next.map(point)))
        })
        .collect()
}

/// FMT-SAV-3.5a (c, T11): the settlers of every list that no building's
/// worker or soldier list and no street's carrier refs name, in state 3
/// (going home): owner, profession, current field and saved number.
pub fn saved_homeward(save: &s2_assets::sav::SaveGame) -> Vec<s2_sim::RestoredSettler> {
    // (b): a named worker in state 3 goes home too.
    let home_bound: std::collections::HashSet<u64> = save.settlers[0]
        .iter()
        .filter(|s| s.state == 3)
        .map(|s| s.id)
        .collect();
    let mut named: std::collections::HashSet<u64> = save
        .buildings
        .iter()
        .flat_map(|b| b.workers.iter().chain(&b.soldiers).copied())
        .filter(|id| !home_bound.contains(id))
        .collect();
    named.extend(save.streets.iter().flat_map(|s| s.carriers));
    save.settlers
        .iter()
        .flatten()
        .filter(|s| s.state == 3 && !named.contains(&s.id))
        .filter_map(|s| {
            let field = s.movement.field.or(s.movement.rough[2])?;
            Some(s2_sim::RestoredSettler {
                player: PlayerId(u8::try_from(s.owner).ok()?),
                profession: s.profession,
                field: (field.x, field.y),
                number: s.id as u32,
                failures: s.home_failures,
            })
        })
        .collect()
}

pub fn restored_wares(save: &s2_assets::sav::SaveGame) -> Vec<s2_sim::RestoredWare> {
    use s2_assets::sav::NO_ID;
    use std::collections::HashMap;
    let player = |p: u32| PlayerId(u8::try_from(p).unwrap_or(u8::MAX));
    let point = |f: s2m::FieldPos| (f.x, f.y);
    let buildings: HashMap<u64, ((u32, u32), u32)> = save
        .buildings
        .iter()
        .map(|b| (b.id, (point(b.field), b.owner)))
        .collect();
    let flags: HashMap<u64, ((u32, u32), u32)> = save
        .flags
        .iter()
        .map(|f| (f.id, (point(f.field), f.owner)))
        .collect();
    let carriers: std::collections::HashSet<u64> = save.settlers[2].iter().map(|s| s.id).collect();
    let streets: HashMap<u64, (usize, bool, u32)> = save
        .streets
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            [(s.carriers[0], false), (s.carriers[1], true)]
                .into_iter()
                .filter(|(id, _)| *id != NO_ID && carriers.contains(id))
                .map(move |(id, donkey)| (id, (i, donkey, s.owner)))
        })
        .collect();
    let stamps: HashMap<u64, u32> = save
        .building_orders
        .iter()
        .filter(|o| o.package != NO_ID)
        .map(|o| (o.package, o.stamp))
        .collect();
    let stored: HashMap<u64, u64> = save
        .package_targets
        .iter()
        .filter(|(_, warehouse)| *warehouse != NO_ID)
        .copied()
        .collect();
    // The flag whose list names it (else its flag ref).
    let ware = |p: &s2_assets::sav::SavedPackage,
                listed_at: Option<((u32, u32), u32)>|
     -> s2_sim::RestoredWare {
        let flag = listed_at.or_else(|| {
            (p.flag != NO_ID)
                .then(|| flags.get(&p.flag))
                .flatten()
                .copied()
        });
        let carried = (!p.lying)
            .then(|| streets.get(&p.carrier).copied())
            .flatten();
        let warehouse = stored.get(&p.id).and_then(|w| buildings.get(w)).copied();
        let building = (p.destination != NO_ID)
            .then(|| buildings.get(&p.destination))
            .flatten()
            .copied();
        let destination = match (warehouse, building) {
            (Some(w), _) => Some((w, true)),
            (None, Some(b)) if p.has_target => Some((b, false)),
            _ => None,
        };
        let owner = flag
            .map(|f| f.1)
            .or(carried.map(|c| c.2))
            .or(destination.map(|d| d.0 .1))
            .unwrap_or(u32::MAX);
        s2_sim::RestoredWare {
            player: player(owner),
            good: p.good,
            lying_at: flag.filter(|_| p.lying).map(|f| f.0),
            carried_by: carried.map(|c| (c.0, c.1)),
            to: flag.filter(|_| !p.lying).map(|f| f.0),
            next: p.next.filter(|_| p.has_next).map(point),
            destination: destination.map(|((at, _), storage)| (at, storage)),
            order_stamp: destination
                .filter(|(_, storage)| !storage)
                .and(stamps.get(&p.id))
                .map(|&s| u64::from(s)),
            recheck: p.recheck,
            wait_score: p.wait_score,
        }
    };
    // FMT-SAV-3.5a (T10): one in a named worker's hands is his product
    // (`saved_products`), not a ware yet, whether a flag's list names it or
    // not.
    let workers: std::collections::HashSet<u64> = save
        .buildings
        .iter()
        .flat_map(|b| b.workers.iter().copied())
        .collect();
    let packages: HashMap<u64, &s2_assets::sav::SavedPackage> = save
        .packages
        .iter()
        .filter(|p| p.lying || !workers.contains(&p.carrier))
        .map(|p| (p.id, p))
        .collect();
    let mut listed = std::collections::HashSet::new();
    let mut out = Vec::new();
    for f in &save.flags {
        for id in &f.packages {
            if let Some(p) = packages.get(id) {
                if listed.insert(*id) {
                    out.push(ware(p, Some((point(f.field), f.owner))));
                }
            }
        }
    }
    out.extend(
        save.packages
            .iter()
            .filter(|p| !listed.contains(&p.id) && packages.contains_key(&p.id))
            .map(|p| ware(p, None)),
    );
    out
}

/// FMT-SAV-3.2, SIM-RNG-2.3: the generator's 8 bytes as its two words, the
/// state first, then the seed (little-endian; the order read from the
/// samples, whose second word is the seed 0x5432ACE0 of SIM-RNG-2.1).
pub fn random_words(bytes: [u8; 8]) -> [u32; 2] {
    let word = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    [word(0), word(4)]
}

/// FMT-SAV-3.10a: the save's active computer players as `(slot, profile)`
/// for the AI (`s2_ai::AiSystem::activate_all`): the saved profile is the
/// AI's profile number (3 for "Hard", UI-SETUP-2.5); one outside 1 to 3 is
/// left out. Their records follow with `s2_ai::AiSystem::take_over`.
pub fn computer_players(save: &s2_assets::sav::SaveGame) -> Vec<(PlayerId, u8)> {
    save.computer_players
        .iter()
        .filter_map(|c| {
            let slot = u8::try_from(c.slot).ok()?;
            let profile = u8::try_from(c.profile)
                .ok()
                .filter(|p| (1..=3).contains(p))?;
            Some((PlayerId(slot), profile))
        })
        .collect()
}

/// A test world on `map` with the rules of `rules`, in which `owner` owns
/// and has explored every field, as the ECO-PLACE and TRN-ROAD test cases
/// assume.
pub fn new_world(seed: u64, map: &Map, rules: &RuleSet, owner: PlayerId) -> Result<World, String> {
    Ok(World::with_map(
        seed,
        map.terrain(),
        rules.sim_rules()?,
        owner,
    ))
}

/// LUA-PROP-1.2: the entry point, as a game-relative path.
pub const ENTRY_SCRIPT: &str = r"data\scripts\properties\data.lua";

/// Run the property scripts of an install.
pub fn load(game_dir: &GameDir) -> Result<(RuleSet, LoadReport), ScriptError> {
    let source = InstallSource::of(game_dir);
    script::run(Rc::new(source), ENTRY_SCRIPT)
}

/// Run the property scripts of an install, then `overlays` in the same
/// environment (see [`script::Overlay`]).
pub fn load_with_overlays(
    game_dir: &GameDir,
    overlays: &[script::Overlay],
) -> Result<(RuleSet, LoadReport), ScriptError> {
    let source = InstallSource::of(game_dir);
    script::run_with_overlays(Rc::new(source), ENTRY_SCRIPT, overlays)
}

/// Scripts read from an install: file names matched without regard to case
/// (LUA-PROP-1.3) and unwrapped from the rc00 container (FMT-RC). A script
/// in one of `overlays` (the [`GameDir`]'s, `docs/extending.md`) takes the
/// place of the install's script of the same path.
pub struct InstallSource {
    pub root: PathBuf,
    pub overlays: Vec<PathBuf>,
}

impl InstallSource {
    /// The scripts of `game_dir`, with its overlay folders.
    pub fn of(game_dir: &GameDir) -> Self {
        Self {
            root: game_dir.root().to_path_buf(),
            overlays: game_dir.overlays().to_vec(),
        }
    }
}

impl ScriptSource for InstallSource {
    fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let file = s2_assets::find_in(&self.overlays, &self.root, path)?;
        let data = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        if rc00::is_wrapped(&data) {
            // The key depends on the file's own name (FMT-RC-2), so decode
            // with the name found on disk.
            let name = file.to_str().ok_or("path is not valid UTF-8")?;
            rc00::decode(name, &data).map_err(|e| format!("{path}: {e}"))
        } else {
            Ok(data)
        }
    }
}

/// The case-insensitive file lookup, now in `s2_assets` (kept here for
/// the crates that call it by this path).
pub use s2_assets::resolve;
