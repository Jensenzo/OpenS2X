//! The profession table of `specs/ECO-professions.md` (ECO-PROF-1.1) checked
//! against the property scripts of the player's install: the cost list
//! (helper and tool), the rest time and the spawn flag of the warehouses
//! (ECO-PROF-3.1). Skipped (passes) when `S2_GAME_DIR` is not set.

use s2_assets::{GameDir, GameDirError};
use s2_rules::RuleSet;

fn rules() -> Option<RuleSet> {
    match GameDir::from_env() {
        Ok(dir) => Some(s2_rules::load(&dir).unwrap_or_else(|e| panic!("{e}")).0),
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

const HELPER: u32 = 0x7BFF_15CE;

/// (id, name, cost list as ids and counts by name, rest time).
type Row = (
    u32,
    &'static str,
    &'static [(&'static str, u32)],
    Option<u32>,
);

/// ECO-PROF-1.1, in the table's order. A cost entry is a good by script
/// global, or `#` and a hexadecimal profession id.
const TABLE: &[Row] = &[
    (
        0xDD2D_AEBE,
        "builder",
        &[("#7BFF15CE", 1), ("hammer", 1)],
        None,
    ),
    (
        0x2740_000E,
        "leveller",
        &[("#7BFF15CE", 1), ("shovel", 1)],
        None,
    ),
    (
        0x2C50_533E,
        "woodcutter",
        &[("#7BFF15CE", 1), ("axe", 1)],
        Some(222),
    ),
    (
        0x6EBA_970E,
        "forester",
        &[("#7BFF15CE", 1), ("shovel", 1)],
        Some(222),
    ),
    (
        0x8A3A_A1EE,
        "stonemason",
        &[("#7BFF15CE", 1), ("pickaxe", 1)],
        Some(222),
    ),
    (
        0xAB43_100E,
        "hunter",
        &[("#7BFF15CE", 1), ("spear", 1)],
        Some(222),
    ),
    (
        0x4AC5_89CE,
        "fisherman",
        &[("#7BFF15CE", 1), ("fishingRod", 1)],
        Some(1111),
    ),
    (
        0x8D0B_A75E,
        "farmer",
        &[("#7BFF15CE", 1), ("scythe", 1)],
        Some(55),
    ),
    (
        0x2195_BC53,
        "carpenter",
        &[("#7BFF15CE", 1), ("saw", 1)],
        Some(55),
    ),
    (
        0x8627_97A3,
        "baker",
        &[("#7BFF15CE", 1), ("rollingPin", 1)],
        Some(55),
    ),
    (0x290E_6B9E, "miller", &[("#7BFF15CE", 1)], Some(55)),
    (0x0EAF_FECE, "brewer", &[("#7BFF15CE", 1)], Some(55)),
    (0x360B_16E3, "pig farmer", &[("#7BFF15CE", 1)], Some(222)),
    (
        0x35A8_2813,
        "donkey breeder",
        &[("#7BFF15CE", 1)],
        Some(222),
    ),
    (
        0xF062_2D7E,
        "butcher",
        &[("#7BFF15CE", 1), ("hatchet", 1)],
        Some(222),
    ),
    (
        0x59B7_633E,
        "miner",
        &[("#7BFF15CE", 1), ("pickaxe", 1)],
        Some(55),
    ),
    (
        0x9C0A_E313,
        "smelter",
        &[("#7BFF15CE", 1), ("pot", 1)],
        Some(55),
    ),
    (
        0xE59A_65CE,
        "locksmith",
        &[("#7BFF15CE", 1), ("gripper", 1)],
        Some(55),
    ),
    (
        0x03A2_B9F3,
        "smith",
        &[("#7BFF15CE", 1), ("hammer", 1)],
        Some(55),
    ),
    (
        0xFDF2_1163,
        "coiner",
        &[("#7BFF15CE", 1), ("pot", 1)],
        Some(55),
    ),
    (
        0xAE71_FBF3,
        "shipwright",
        &[("#7BFF15CE", 1), ("hammer", 1)],
        Some(222),
    ),
    (
        0x3355_C25E,
        "geologist",
        &[("#7BFF15CE", 1), ("hammer", 1)],
        None,
    ),
    (
        0xA07B_F6FE,
        "scout",
        &[("#7BFF15CE", 1), ("spear", 1)],
        None,
    ),
    (
        0x1821_18D3,
        "boat carrier",
        &[("#7BFF15CE", 1), ("boat", 1)],
        None,
    ),
    (
        0x15E8_6843,
        "private",
        &[("#7BFF15CE", 1), ("sword", 1), ("shield", 1), ("beer", 1)],
        None,
    ),
    (
        0xBA0E_7F83,
        "private first class",
        &[("#15E86843", 1), ("coin", 1)],
        None,
    ),
    (
        0x466C_6C23,
        "sergeant",
        &[("#BA0E7F83", 1), ("coin", 1)],
        None,
    ),
    (
        0x9060_14F3,
        "officer",
        &[("#466C6C23", 1), ("coin", 1)],
        None,
    ),
    (
        0x559D_DEA3,
        "general",
        &[("#906014F3", 1), ("coin", 1)],
        None,
    ),
    (HELPER, "helper", &[("#7BFF15CE", 1)], Some(55)),
    (0xD415_7C5F, "donkey", &[], None),
];

// ECO-PROF-1.1: every profession's cost list and rest time.
#[test]
fn profession_table_matches_the_scripts() {
    let Some(rules) = rules() else { return };
    let resolve = |entry: &str| -> u32 {
        match entry.strip_prefix('#') {
            Some(hex) => u32::from_str_radix(hex, 16).unwrap(),
            None => rules
                .id_named(entry)
                .filter(|&id| rules.goods.contains(id))
                .unwrap_or_else(|| panic!("no good {entry}")),
        }
    };
    for &(id, name, costs, rest) in TABLE {
        let good = rules
            .goods
            .get(id)
            .unwrap_or_else(|| panic!("{name}: no good {id:08X}"));
        let mut got: Vec<(u32, u32)> = good.costs.iter().map(|a| (a.good, a.count)).collect();
        let mut expected: Vec<(u32, u32)> = costs.iter().map(|&(g, n)| (resolve(g), n)).collect();
        got.sort_unstable();
        expected.sort_unstable();
        assert_eq!(got, expected, "{name} {id:08X}: cost list");
        assert_eq!(
            good.rest_time.map(|t| t as u32),
            rest,
            "{name} {id:08X}: rest time"
        );
    }
}

// ECO-PROF-2.2: only the donkey has no cost list, so no warehouse can make
// one; only the helper's list names the helper itself.
#[test]
fn donkey_has_no_cost_list_and_helper_names_itself() {
    let Some(rules) = rules() else { return };
    let donkey = rules.goods.get(0xD415_7C5F).unwrap();
    assert!(donkey.costs.is_empty());
    let own: Vec<_> = rules
        .goods
        .iter()
        .filter(|g| g.costs.iter().any(|a| a.good == g.id))
        .map(|g| g.id)
        .collect();
    assert_eq!(own, [HELPER]);
}

// ECO-PROF-3.1, ECO-CAT-4: the headquarters, the storehouse and the harbour
// are the buildings with the spawn flag.
#[test]
fn spawn_flag_marks_the_three_warehouses() {
    let Some(rules) = rules() else { return };
    let mut spawning: Vec<String> = rules
        .buildings
        .iter()
        .filter(|b| b.spawn.is_some_and(|s| s != 0))
        .map(|b| {
            rules
                .names
                .iter()
                .find(|(_, &id)| id == b.id)
                .map(|(n, _)| n.clone())
                .unwrap_or_else(|| format!("{:08X}", b.id))
        })
        .collect();
    spawning.sort();
    assert_eq!(spawning.len(), 3, "{spawning:?}");
    for b in rules
        .buildings
        .iter()
        .filter(|b| b.spawn.is_some_and(|s| s != 0))
    {
        assert!(b.depot.is_some_and(|d| d != 0), "{:08X}: not a depot", b.id);
    }
    // The simulation takes the flag.
    let types = rules.building_types();
    assert_eq!(types.iter().filter(|t| t.spawn).count(), 3);
}
