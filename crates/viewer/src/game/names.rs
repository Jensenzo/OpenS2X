//! Names and lists of goods and buildings for the debug panels.

use bevy::prelude::*;
use s2_assets::s2m::state;
use s2_rules::RuleSet;
use s2_sim::{BuildingSize, Placement};
use std::collections::HashMap;

/// Bits of the field state layer with the names of FMT-S2M-4.4.
pub(crate) const STATE_NAMES: [(u32, &str); 13] = [
    (state::BLOCKED, "Blocked"),
    (state::STREET, "Street"),
    (state::WATER, "Water"),
    (state::COAST, "Coast"),
    (state::FOR_MINING, "ForMining"),
    (state::BORDER, "Border"),
    (state::ELEVATION_BLOCKED, "ElevationBlocked"),
    (state::DEPOSIT_BLOCKED, "DepositBlocked"),
    (state::MOVEMENT_BLOCKED, "MovementBlocked"),
    (state::FOR_BUILDING, "ForBuilding"),
    (state::DOODAD_BLOCKED, "DoodadBlocked"),
    (state::LANDING_POSITION, "LandingPosition"),
    (state::FOR_HARBOR, "ForHarbor"),
];

/// Wares then settler professions, with names, for the warehouse panel.
pub(crate) fn warehouse_goods(rules: Option<&RuleSet>) -> Vec<crate::ui::economy::PanelGood> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    let name = |good: &s2_rules::model::Good| {
        good.name
            .clone()
            .unwrap_or_else(|| format!("{:08X}", good.id))
    };
    rules
        .wares()
        // Q-033: goods without a type are wares.
        .chain(rules.untyped_goods())
        .map(|g| (g.id, name(g), false))
        .chain(rules.jobs().map(|g| (g.id, name(g), true)))
        .collect()
}

/// The name of every building type.
pub(crate) fn building_names(rules: Option<&RuleSet>) -> HashMap<u32, String> {
    let Some(rules) = rules else {
        return HashMap::new();
    };
    rules
        .buildings
        .iter()
        .map(|b| {
            (
                b.id,
                b.name.clone().unwrap_or_else(|| format!("{:08X}", b.id)),
            )
        })
        .collect()
}

/// The name of every good.
pub(crate) fn good_names(rules: Option<&RuleSet>) -> HashMap<u32, String> {
    let Some(rules) = rules else {
        return HashMap::new();
    };
    rules
        .goods
        .iter()
        .map(|good| {
            let name = good
                .name
                .clone()
                .unwrap_or_else(|| format!("{:08X}", good.id));
            (good.id, name)
        })
        .collect()
}

pub(crate) fn good_name(names: &HashMap<u32, String>, good: u32) -> String {
    names
        .get(&good)
        .cloned()
        .unwrap_or_else(|| format!("{good:08X}"))
}

/// "name count, name count, ..." for a list of (good, count).
pub(crate) fn amounts(names: &HashMap<u32, String>, list: &[(u32, u32)]) -> String {
    if list.is_empty() {
        return "none".into();
    }
    list.iter()
        .map(|&(good, n)| format!("{} {n}", good_name(names, good)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Goods that can be dropped as wares for watching transport.
pub(crate) fn ware_list(rules: Option<&RuleSet>) -> Vec<(u32, String)> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    rules
        .goods
        .iter()
        .filter(|good| {
            good.transport_priority.is_some()
                && !matches!(
                    good.good_type,
                    Some(s2_rules::model::GoodType::Group | s2_rules::model::GoodType::Settler)
                )
        })
        .map(|good| {
            let name = good
                .name
                .clone()
                .unwrap_or_else(|| format!("{:08X}", good.id));
            (good.id, name)
        })
        .collect()
}

pub(crate) struct BuildingChoice {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) size: BuildingSize,
}

/// Building types the player can place, ordered as `bp_setSort` orders them.
pub(crate) fn building_list(rules: Option<&RuleSet>) -> Vec<BuildingChoice> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    let mut list: Vec<(i64, BuildingChoice)> = rules
        .building_types()
        .into_iter()
        .map(|t| {
            let building = rules.buildings.get(t.id);
            let name = building
                .and_then(|b| b.name.clone())
                .unwrap_or_else(|| format!("{:08X}", t.id));
            let sort = building.and_then(|b| b.sort).unwrap_or(i64::MAX);
            let choice = BuildingChoice {
                id: t.id,
                name,
                size: t.size,
            };
            (sort, choice)
        })
        .collect();
    list.sort_by_key(|(sort, _)| *sort);
    list.into_iter().map(|(_, choice)| choice).collect()
}

pub(crate) fn size_name(size: BuildingSize) -> &'static str {
    match size {
        BuildingSize::Small => "small",
        BuildingSize::Medium => "medium",
        BuildingSize::Large => "large",
        BuildingSize::Mine => "mine",
        BuildingSize::Harbor => "harbour",
    }
}

pub(crate) fn placement_name(class: Placement) -> &'static str {
    match class {
        Placement::Nothing => "nothing",
        Placement::Flag => "flag only",
        Placement::Small => "small",
        Placement::Medium => "medium",
        Placement::Large => "large",
        Placement::Mine => "mine",
        Placement::Harbor => "harbour",
    }
}
