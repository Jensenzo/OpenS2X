//! Test cases of `specs/LUA-properties-api.md` run against the property
//! scripts of the player's own install through `S2_GAME_DIR`. When it is
//! unset each test prints a note and passes without checking anything.

use s2_assets::s2m::{self, NO_GOOD};
use s2_assets::{GameDir, GameDirError};
use s2_rules::model::GoodType;
use s2_rules::script::function_names;
use s2_rules::{LoadReport, RuleSet};

fn game_dir() -> Option<GameDir> {
    match GameDir::from_env() {
        Ok(dir) => Some(dir),
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn load() -> Option<(RuleSet, LoadReport)> {
    let dir = game_dir()?;
    Some(s2_rules::load(&dir).unwrap_or_else(|e| panic!("{e}")))
}

// T1, LUA-PROP-1.2
#[test]
fn t01_all_scripts_run_in_load_order() {
    let Some((rules, report)) = load() else {
        return;
    };
    let names: Vec<&str> = report
        .scripts
        .iter()
        .map(|path| path.rsplit('\\').next().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "data.lua",
            "doodads.lua",
            "config.lua",
            "patterns.lua",
            "goods.lua",
            "jobs.lua",
            "deposits.lua",
            "remains.lua",
            "buildings.lua",
            "streets.lua",
            "signs.lua",
            "military.lua",
            "ship.lua",
            "animals.lua",
            "sounds.lua",
            "ai.lua"
        ]
    );
    // Every call went to a registered function (others are errors).
    let registered = function_names();
    assert!(report
        .calls
        .keys()
        .all(|name| registered.iter().any(|r| r == name)));
    assert!(
        rules.dangling_references().is_empty(),
        "{:?}",
        rules.dangling_references()
    );
}

// T2
// spec: LUA-PROP-2.1, LUA-PROP-2.6 (T2)
#[test]
fn t02_create_counts() {
    let Some((rules, report)) = load() else {
        return;
    };
    let counts = [
        ("gp_create", rules.goods.len(), 67),
        ("bp_create", rules.buildings.len(), 33),
        ("pa_create", rules.patterns.len(), 73),
        ("dp_create", rules.deposits.len(), 38),
        ("dd_create", rules.doodads.len(), 112),
        ("rm_create", rules.remains.len(), 3),
        ("an_create", rules.animals.len(), 3),
        ("sp_create", rules.streets.len(), 1),
        ("sn_create", rules.sounds.len(), 104),
        ("ai_create", rules.ai_profiles.len(), 3),
    ];
    for (function, created, expected) in counts {
        assert_eq!(report.call_count(function) as usize, expected, "{function}");
        assert_eq!(created, expected, "{function}");
    }
}

// T3: ids are distinct within a family (a repeated id is an error) and fit in
// 32 bits (they are u32); some pattern id has fewer than 8 hex digits.
// spec: LUA-PROP-2.1, LUA-PROP-2.1a (T3)
#[test]
fn t03_short_pattern_ids() {
    let Some((rules, _)) = load() else { return };
    assert!(rules.patterns.iter().any(|p| p.id < 0x1000_0000));
}

// T4
// spec: LUA-PROP-1.5, LUA-PROP-9 (T4)
#[test]
fn t04_doodads_come_first() {
    let Some((rules, report)) = load() else {
        return;
    };
    assert_eq!(report.first_call, Some("dd_create"));
    let skeleton = rules.settings.settler_skeleton_doodad.unwrap();
    assert!(rules.doodads.contains(skeleton));
}

// T5
// spec: LUA-PROP-1.3 (T5)
#[test]
fn t05_file_names_ignore_case() {
    let Some(dir) = game_dir() else { return };
    for (asked, on_disk) in [("goods.lua", "Goods.lua"), ("jobs.lua", "Jobs.lua")] {
        let path = format!(r"data\scripts\properties\{asked}");
        let found = s2_rules::resolve(dir.root(), &path).unwrap();
        assert_eq!(found.file_name().unwrap(), on_disk);
    }
    assert!(s2_rules::resolve(dir.root(), r"data\scripts\properties\nope.lua").is_err());
}

// T6
// spec: LUA-PROP-1.7, LUA-PROP-2.6 (T6)
#[test]
fn t06_never_called_functions() {
    let Some((_, report)) = load() else { return };
    let never: Vec<String> = function_names()
        .into_iter()
        .filter(|name| report.call_count(name) == 0)
        .collect();
    assert_eq!(
        never,
        [
            "an_setIdleTimeFactor",
            "bp_setAlternativeGoods",
            "bp_setHealth",
            "cfg_setGameSpeed",
            "dd_setOnWater",
            "dp_setDoodadAfterDestruction",
            "gp_setValue",
            "rm_setDoodadAfterFinished",
            "sn_set3DMaxDistance",
            "sp_addCost",
            "sp_setHeight",
            "sp_setLength"
        ]
    );
    // Q-030: the spec's tables list 148 functions; the scripts call the rest.
    assert_eq!(report.calls.len(), 136);
}

// T7
// spec: LUA-PROP-2.1, FMT-S2M-1.7 (T7)
#[test]
fn t07_map_ids_resolve() {
    let Some(dir) = game_dir() else { return };
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let rel = "data/maps/campaignMaps/campaign01.s2m";
    let map = s2m::load(rel, &std::fs::read(dir.root().join(rel)).unwrap()).unwrap();

    let missing = |what: &str, ids: Vec<u32>, known: &dyn Fn(u32) -> bool| {
        let mut unknown: Vec<u32> = ids.into_iter().filter(|&id| !known(id)).collect();
        unknown.sort();
        unknown.dedup();
        assert!(
            unknown.is_empty(),
            "{what} not created by a script: {unknown:08X?}"
        );
    };
    missing("patterns", map.patterns.clone(), &|id| {
        rules.patterns.contains(id)
    });
    missing(
        "deposits",
        map.deposits.iter().map(|d| d.type_id).collect(),
        &|id| rules.deposits.contains(id),
    );
    let doodads = &map.doodads;
    let doodad_ids = doodads
        .non_blocking
        .iter()
        .chain(&doodads.unknown)
        .chain(&doodads.blocking)
        .map(|d| d.type_id)
        .collect();
    missing("doodads", doodad_ids, &|id| rules.doodads.contains(id));
    missing(
        "animals",
        map.animals.iter().map(|a| a.type_id).collect(),
        &|id| rules.animals.contains(id),
    );
    let goods = map
        .resources
        .iter()
        .map(|r| r.good)
        .filter(|&good| good != NO_GOOD)
        .collect();
    missing("goods", goods, &|id| rules.goods.contains(id));
    missing(
        "sounds",
        map.ambients.iter().map(|a| a.sound_id).collect(),
        &|id| rules.sounds.contains(id),
    );
}

// T8
// spec: LUA-PROP-9 (T8)
#[test]
fn t08_geologist_signs() {
    let Some((rules, report)) = load() else {
        return;
    };
    assert_eq!(report.call_count("si_addSignDoodadId"), 14);
    assert_eq!(rules.signs.len(), 14);
    assert_eq!(rules.signs.iter().filter(|s| s.good.is_none()).count(), 1);
}

// ECO-SPC: what the simulation takes for geologists and scouts.
// spec: ECO-SPC-3.2, ECO-SPC-3.3, LUA-PROP-9
#[test]
fn specialist_rules_for_the_simulation() {
    let Some((rules, _)) = load() else { return };
    let sim = rules.sim_rules().unwrap().specialists;
    assert!(sim.geologist.is_some() && sim.scout.is_some() && sim.water.is_some());
    assert_eq!(sim.signs.len(), 14);
    assert_eq!(sim.found.len(), 5, "ECO-SPC-3.3: five messages");
    // ECO-CAT-5.2: coal has signs at 1, 3 and 6.
    let coal = rules.good_named("Coal").unwrap().id;
    let mut thresholds: Vec<u32> = sim
        .signs
        .iter()
        .filter(|s| s.good == Some(coal))
        .map(|s| s.threshold)
        .collect();
    thresholds.sort();
    assert_eq!(thresholds, [1, 3, 6]);
}

// T9: every call passes three numbers (a nil colour is an error). Some
// patterns get their colour more than once; the last call wins.
// spec: LUA-PROP-7, LUA-PROP-2.5 (T9)
#[test]
fn t09_minimap_colours() {
    let Some((rules, report)) = load() else {
        return;
    };
    let coloured = rules
        .patterns
        .iter()
        .filter(|p| p.minimap_color.is_some())
        .count();
    assert!(coloured > 0);
    assert!(report.call_count("pa_setMiniMapColor") as usize >= coloured);
}

// ECO-time-units.md: production time is in ticks.
// spec: LUA-PROP-6, ECO-TIME-3.1
#[test]
fn sawmill_production_time_is_333_ticks() {
    let Some((rules, _)) = load() else { return };
    let sawmill = rules.building_named("Sawmill").unwrap();
    assert_eq!(sawmill.production_time, Some(333.0));
    let planks = rules.goods.get(sawmill.goods.unwrap().good).unwrap();
    assert!(planks.name.is_some());
}

// The settings the simulation takes (TRN-MOVE-1.1, TRN-CARRY, TRN-ROAD-5.2).
#[test]
fn settings_for_the_simulation() {
    let Some((rules, _)) = load() else { return };
    let sim = rules.sim_rules().unwrap();
    assert_eq!(sim.settings, s2_sim::Settings::shipped());
    // The transport priority table of TRN-CARRY: coin 0, plank 7, wood 9.
    for (name, priority) in [("Coin", 0), ("Plank", 7), ("Wood", 9)] {
        let good = rules.good_named(name).unwrap();
        assert_eq!(
            sim.transport_priorities.get(&good.id),
            Some(&priority),
            "{name}"
        );
    }
}

// spec: LUA-PROP-2.1a, LUA-PROP-5
#[test]
fn sampled_values() {
    let Some((rules, _)) = load() else { return };
    // Q-029: the one id written with a 0x prefix.
    assert!(rules.sounds.contains(0xCEA2_BCC3));
    // Q-033: no good is set to "ware" explicitly.
    assert_eq!(rules.wares().count(), 0);
    assert_eq!(rules.jobs().count(), 30);
    assert_eq!(rules.ware_groups().count(), 6);
    assert_eq!(rules.untyped_goods().count(), 31);
    assert!(rules
        .goods
        .iter()
        .all(|g| g.good_type != Some(GoodType::Invalid)));
    // The terrain type of FMT-S2M T4 (start position of tutorial01).
    assert!(rules.patterns.contains(0x4545_FAC4));
    assert_eq!(rules.settings.flag_package_limit, Some(6));
}
