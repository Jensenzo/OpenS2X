//! Every value of `specs/ECO-buildings-catalog.md` (ECO-CAT-2 to 6) checked
//! against what the property scripts of the player's install load, and the
//! cycle the simulation derives for each building (ECO-CAT-3 "Cycle").
//! Skipped (passes) when `S2_GAME_DIR` is not set.

use s2_assets::{GameDir, GameDirError};
use s2_rules::model::{BuildingSize as Size, GoodType};
use s2_rules::RuleSet;
use s2_sim::WorkCycle as C;

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

type List = &'static [(&'static str, u32)];

/// One row of ECO-CAT-2 and ECO-CAT-3.
struct Row {
    global: &'static str,
    id: u32,
    size: Size,
    frame: List,
    main: List,
    /// ECO-CAT-2 "Work ticks" (the building complete); None = not built.
    work: Option<u32>,
    worker: Option<&'static str>,
    product: Option<&'static str>,
    second: Option<&'static str>,
    stock: Option<u32>,
    prod_time: Option<u32>,
    radius: Option<u32>,
    cycle: C,
}

const fn row(
    global: &'static str,
    id: u32,
    size: Size,
    frame: List,
    main: List,
    work: Option<u32>,
) -> Row {
    Row {
        global,
        id,
        size,
        frame,
        main,
        work,
        worker: None,
        product: None,
        second: None,
        stock: None,
        prod_time: None,
        radius: None,
        cycle: C::None,
    }
}

impl Row {
    const fn works(
        mut self,
        worker: &'static str,
        product: Option<&'static str>,
        prod_time: u32,
        cycle: C,
    ) -> Self {
        self.worker = Some(worker);
        self.product = product;
        self.prod_time = Some(prod_time);
        self.cycle = cycle;
        self
    }
    const fn worker(mut self, worker: &'static str) -> Self {
        self.worker = Some(worker);
        self
    }
    const fn stock(mut self, n: u32) -> Self {
        self.stock = Some(n);
        self
    }
    const fn radius(mut self, n: u32) -> Self {
        self.radius = Some(n);
        self
    }
    const fn second(mut self, g: &'static str) -> Self {
        self.second = Some(g);
        self
    }
}

const P: &str = "plank";
const S: &str = "stone";

fn table() -> Vec<Row> {
    use Size::*;
    vec![
        row("castle", 0xF6E26CB3, Large, &[], &[], None),
        row(
            "woodCutter",
            0x5A926FA3,
            Small,
            &[(P, 1)],
            &[(P, 1)],
            Some(442),
        )
        .works("good_woodcutter", Some("wood"), 55, C::Gatherer)
        .radius(6),
        row(
            "forester",
            0x3FF43D23,
            Small,
            &[(P, 1)],
            &[(P, 1)],
            Some(442),
        )
        .works("good_forester", None, 55, C::Forester)
        .radius(6),
        row(
            "stonePit",
            0x043185F3,
            Small,
            &[(P, 1)],
            &[(P, 1)],
            Some(442),
        )
        .works("good_stonecutter", Some("stone"), 55, C::Gatherer)
        .radius(8),
        row("fisher", 0x3EF3BC43, Small, &[(P, 1)], &[(P, 1)], Some(442))
            .works("good_fisher", Some("fish"), 333, C::Fisher)
            .radius(8),
        row("hunter", 0x3C10E223, Small, &[(P, 1)], &[(P, 1)], Some(442))
            .works("good_hunter", Some("flesh"), 55, C::Hunter)
            .radius(8),
        row("spring", 0xC2C7C303, Small, &[(P, 1)], &[(P, 1)], Some(442))
            .works("good_helper", Some("water"), 333, C::Workshop)
            .stock(6),
        row(
            "watchTower",
            0x281F0783,
            Small,
            &[(P, 2)],
            &[(P, 2)],
            Some(882),
        )
        .worker("good_explorer"),
        row(
            "barracks",
            0xA7BBC573,
            Small,
            &[(P, 1)],
            &[(P, 1)],
            Some(442),
        )
        .stock(1),
        row(
            "guardHouse",
            0x12E67603,
            Small,
            &[(P, 2)],
            &[(S, 3)],
            Some(1101),
        )
        .stock(2),
        row(
            "tower",
            0x7D00B493,
            Medium,
            &[(P, 3)],
            &[(S, 5)],
            Some(1760),
        )
        .stock(4),
        row(
            "fortress",
            0x8C137E93,
            Large,
            &[(P, 4)],
            &[(S, 7)],
            Some(2419),
        )
        .stock(6),
        row(
            "ironMine",
            0x18282BD3,
            Mine,
            &[(P, 2)],
            &[(P, 2)],
            Some(882),
        )
        .works("good_miner", Some("ironOre"), 333, C::Mine)
        .stock(6)
        .radius(3),
        row(
            "goldMine",
            0x154C1DAE,
            Mine,
            &[(P, 2)],
            &[(P, 2)],
            Some(882),
        )
        .works("good_miner", Some("gold"), 333, C::Mine)
        .stock(6)
        .radius(3),
        row(
            "coalMine",
            0x9B027DAE,
            Mine,
            &[(P, 2)],
            &[(P, 2)],
            Some(882),
        )
        .works("good_miner", Some("coal"), 333, C::Mine)
        .stock(6)
        .radius(3),
        row(
            "stoneMine",
            0x6222A09E,
            Mine,
            &[(P, 2)],
            &[(P, 2)],
            Some(882),
        )
        .works("good_miner", Some("stone"), 333, C::Mine)
        .stock(6)
        .radius(3),
        row(
            "sawMill",
            0x918FF373,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_cabinetmaker", Some("plank"), 333, C::Workshop)
        .stock(6),
        row("mill", 0xF7E2ED93, Medium, &[(P, 2)], &[(S, 2)], Some(882))
            .works("good_miller", Some("flour"), 333, C::Workshop)
            .stock(6),
        row(
            "bakery",
            0x0AF7BB13,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_baker", Some("bread"), 333, C::Workshop)
        .stock(6),
        row(
            "slaughterHouse",
            0x5C5E9743,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_slaughter", Some("flesh"), 333, C::Workshop)
        .stock(6),
        row(
            "smeltery",
            0x154551B3,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_smelter", Some("iron"), 333, C::Workshop)
        .stock(6),
        row(
            "locksmithery",
            0xFAADA31E,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_locksmith", Some("tools"), 333, C::Workshop)
        .stock(6),
        row(
            "smithy",
            0xB779AB83,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_smith", Some("sword"), 333, C::Workshop)
        .second("shield")
        .stock(6),
        row("mint", 0xBC203663, Medium, &[(P, 2)], &[(S, 2)], Some(882))
            .works("good_coiner", Some("coin"), 333, C::Workshop)
            .stock(6),
        row(
            "brewery",
            0xFBCA3A8E,
            Medium,
            &[(P, 2)],
            &[(S, 2)],
            Some(882),
        )
        .works("good_brewer", Some("beer"), 333, C::Workshop)
        .stock(6),
        row(
            "shipYard",
            0x78F8184E,
            Medium,
            &[(P, 2)],
            &[(S, 3)],
            Some(1101),
        )
        .works("good_shipBuilder", Some("boat"), 333, C::Shipyard)
        .second("boat")
        .stock(6)
        .radius(8),
        row(
            "depot",
            0xB3965083,
            Medium,
            &[(P, 4)],
            &[(S, 3)],
            Some(1542),
        ),
        row(
            "catapult",
            0xA1445EF3,
            Medium,
            &[(P, 3)],
            &[(P, 1), (S, 2)],
            Some(1322),
        )
        .worker("good_helper")
        .stock(4)
        .radius(12),
        row("farm", 0x07F63873, Large, &[(P, 3)], &[(S, 3)], Some(1322))
            .works("good_farmer", Some("corn"), 83, C::Farm)
            .radius(2),
        row(
            "piggery",
            0xA6C72EDE,
            Large,
            &[(P, 3)],
            &[(S, 3)],
            Some(1322),
        )
        .works("good_pigfarmer", Some("pig"), 333, C::Workshop)
        .stock(6),
        row(
            "donkeyBreeding",
            0x1F0FCD1E,
            Large,
            &[(P, 3)],
            &[(S, 3)],
            Some(1322),
        )
        .works("good_donkeyBreeder", Some("donkey"), 333, C::DonkeyFarm)
        .stock(6),
        row(
            "harbor",
            0xCF526FF3,
            Harbor,
            &[(P, 4)],
            &[(S, 6)],
            Some(2200),
        ),
    ]
}

fn building<'a>(rules: &'a RuleSet, global: &str) -> &'a s2_rules::model::Building {
    rules
        .buildings
        .get(rules.id_named(global).unwrap())
        .unwrap()
}

fn good(rules: &RuleSet, global: &str) -> u32 {
    rules
        .id_named(global)
        .filter(|&id| rules.goods.contains(id))
        .unwrap_or_else(|| panic!("no good {global}"))
}

fn amounts(rules: &RuleSet, list: List) -> Vec<(u32, u32)> {
    list.iter().map(|&(g, n)| (good(rules, g), n)).collect()
}

// ECO-CAT T1: 33 building types, 32 with a size.
// spec: ECO-CAT-1.1, ECO-CAT-2.1 (T1)
#[test]
fn t01_building_types() {
    let Some(rules) = rules() else { return };
    assert_eq!(rules.buildings.len(), 33);
    assert_eq!(
        rules.buildings.iter().filter(|b| b.size.is_some()).count(),
        32
    );
    assert_eq!(table().len(), 32);
    let site = rules.id_named("construction").unwrap();
    assert_eq!(site, 0x4A2CE4DE);
    assert_eq!(rules.buildings.get(site).unwrap().size, None);
    // The simulation's types skip the construction site (no size).
    assert_eq!(rules.building_types().len(), 32);
}

// ECO-CAT-2, 3: every building row.
#[test]
fn catalog_rows_match_the_scripts_and_the_derived_cycle() {
    let Some(rules) = rules() else { return };
    let sim = rules.sim_rules().unwrap();
    for r in table() {
        let name = r.global;
        assert_eq!(rules.id_named(name), Some(r.id), "{name}: id");
        let b = rules
            .buildings
            .get(r.id)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(b.size, Some(r.size), "{name}: size");
        let amt =
            |l: &[s2_rules::model::Amount]| l.iter().map(|a| (a.good, a.count)).collect::<Vec<_>>();
        assert_eq!(
            amt(&b.frame_costs),
            amounts(&rules, r.frame),
            "{name}: frame"
        );
        assert_eq!(amt(&b.costs), amounts(&rules, r.main), "{name}: main");
        assert_eq!(
            b.worker,
            r.worker.map(|w| rules.id_named(w).unwrap()),
            "{name}: worker"
        );
        assert_eq!(
            b.goods.map(|a| (a.good, a.count)),
            r.product.map(|p| (good(&rules, p), 1)),
            "{name}: product (ECO-CAT-1.7: amount 1)"
        );
        assert_eq!(
            b.second_goods.map(|a| a.good),
            r.second.map(|p| good(&rules, p)),
            "{name}: second product"
        );
        assert_eq!(b.stock_limit, r.stock, "{name}: stock");
        assert_eq!(
            b.production_time,
            r.prod_time.map(f64::from),
            "{name}: production time"
        );
        assert_eq!(b.working_radius, r.radius, "{name}: radius");
        // ECO-CAT-1.6: the work ticks the simulation counts.
        match r.work {
            Some(work) => {
                let (frame, total) = sim.construction_work_ticks(r.id).unwrap();
                assert_eq!(total, work, "{name}: work ticks");
                assert!(frame < total);
            }
            None => assert_eq!(b.construction_time, Some(-1.0), "{name}: not built"),
        }
        // ECO-CAT-3: the cycle derived from the data.
        let kind = sim.building_types.iter().find(|t| t.id == r.id).unwrap();
        assert_eq!(kind.cycle, r.cycle, "{name}: cycle");
        assert_eq!(kind.stock_limit, r.stock, "{name}");
        // ECO-CAT-1.5: radius default 4, worker limit default 1.
        assert_eq!(kind.working_radius, r.radius.unwrap_or(4), "{name}");
    }
}

// ECO-CAT T6, T7: the work ticks of the three examples and the others.
// spec: ECO-CAT-1.6 (T6, T7)
#[test]
fn t06_t07_work_ticks() {
    let Some(rules) = rules() else { return };
    let sim = rules.sim_rules().unwrap();
    let ticks = |g: &str| {
        sim.construction_work_ticks(rules.id_named(g).unwrap())
            .unwrap()
    };
    // Stronghold: frame done at unit 4 (decrement 4 x 221 - 4... see ECO-HQ-4.7).
    assert_eq!(ticks("fortress").1, 2419);
    assert_eq!(ticks("guardHouse").1, 1101);
    assert_eq!(ticks("harbor").1, 2200);
    assert_eq!(ticks("catapult").1, 1322);
    assert_eq!(ticks("depot").1, 1542);
}

// ECO-CAT-2.4, T10: door offsets of tribe 1.
#[test]
fn t10_door_offsets_of_the_africans() {
    let Some(rules) = rules() else { return };
    let with: Vec<&str> = table()
        .iter()
        .map(|r| r.global)
        .filter(|g| {
            let b = building(&rules, g);
            b.door_directions
                .iter()
                .any(|&(t, d)| t as u32 == 1 && d == s2_rules::model::DoorDirection::NorthWest)
        })
        .collect();
    assert_eq!(with, ["castle", "farm", "piggery", "donkeyBreeding"]);
}

// ECO-CAT-3.x / T2 / T3 / T4 / T5 / T8 and the inputs of every building
// (ECO-CAT-1.3, ECO-PROD-2.1).
#[test]
fn inputs_are_the_cost_lists_of_the_products() {
    let Some(rules) = rules() else { return };
    let cost = |g: &str| -> Vec<(String, u32)> {
        let good = rules.goods.get(good(&rules, g)).unwrap();
        good.costs
            .iter()
            .map(|a| {
                let name = rules
                    .names
                    .iter()
                    .find(|(_, &id)| id == a.good)
                    .map(|(n, _)| n.clone())
                    .unwrap();
                (name, a.count)
            })
            .collect()
    };
    let list = |v: &[(&str, u32)]| {
        v.iter()
            .map(|&(n, c)| (n.to_string(), c))
            .collect::<Vec<_>>()
    };
    // ECO-CAT-3 "Inputs" column, by product.
    assert_eq!(cost("plank"), list(&[("wood", 1)]));
    assert_eq!(cost("flour"), list(&[("corn", 1)]));
    assert_eq!(cost("bread"), list(&[("water", 1), ("flour", 1)]));
    assert_eq!(cost("beer"), list(&[("water", 1), ("corn", 1)]));
    assert_eq!(cost("pig"), list(&[("corn", 1), ("water", 1)]));
    assert_eq!(cost("flesh"), list(&[("pig", 1)]));
    assert_eq!(cost("donkey"), list(&[("corn", 1), ("water", 1)]));
    assert_eq!(cost("iron"), list(&[("ironOre", 1), ("coal", 1)]));
    assert_eq!(cost("coin"), list(&[("gold", 1), ("coal", 1)]));
    assert_eq!(cost("tools"), list(&[("plank", 1), ("iron", 1)]));
    assert_eq!(cost("sword"), list(&[("iron", 1), ("coal", 1)]));
    assert_eq!(cost("shield"), cost("sword"));
    assert_eq!(cost("boat"), list(&[("plank", 1)]));
    assert!(cost("water").is_empty());
    // ECO-CAT-1.4: the tool of a profession; four need none.
    for (job, tool) in [
        ("good_woodcutter", Some("axe")),
        ("good_forester", Some("shovel")),
        ("good_stonecutter", Some("pickaxe")),
        ("good_fisher", Some("fishingRod")),
        ("good_hunter", Some("spear")),
        ("good_cabinetmaker", Some("saw")),
        ("good_farmer", Some("scythe")),
        ("good_miller", None),
        ("good_baker", Some("rollingPin")),
        ("good_slaughter", Some("hatchet")),
        ("good_brewer", None),
        ("good_miner", Some("pickaxe")),
        ("good_smelter", Some("pot")),
        ("good_smith", Some("hammer")),
        ("good_coiner", Some("pot")),
        ("good_locksmith", Some("gripper")),
        ("good_pigfarmer", None),
        ("good_donkeyBreeder", None),
        ("good_shipBuilder", Some("hammer")),
        ("good_constructor", Some("hammer")),
        ("good_bulldozer", Some("shovel")),
        ("good_scholar", Some("hammer")),
        ("good_explorer", Some("spear")),
    ] {
        let mut expected = vec![(job_helper(), 1)];
        if let Some(t) = tool {
            expected.push((t.to_string(), 1));
        }
        let mut got = cost(job);
        got.sort();
        expected.sort();
        assert_eq!(got, expected, "{job}");
    }
}

fn job_helper() -> String {
    "good_helper".to_string()
}

// ECO-CAT-5: groups, priorities and the sort order.
#[test]
fn goods_match_the_catalog() {
    let Some(rules) = rules() else { return };
    let group_of = |g: &str| {
        let id = good(&rules, g);
        rules.goods.get(id).unwrap().group
    };
    let named = |n: &str| Some(good(&rules, n));
    for f in ["fish", "flesh", "bread"] {
        assert_eq!(group_of(f), named("food"), "{f}");
    }
    for t in [
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
    ] {
        assert_eq!(group_of(t), named("tools"), "{t}");
    }
    for w in ["sword", "shield"] {
        assert_eq!(group_of(w), named("weapons"), "{w}");
    }
    // ECO-CAT-5 "Priority" column (ECO-CAT T9: weapons 1, tools 12).
    for (g, p) in [
        ("wood", 9),
        ("plank", 7),
        ("stone", 8),
        ("pig", 11),
        ("corn", 11),
        ("flour", 11),
        ("fish", 11),
        ("flesh", 11),
        ("bread", 11),
        ("water", 10),
        ("beer", 2),
        ("coal", 6),
        ("ironOre", 5),
        ("iron", 3),
        ("gold", 4),
        ("coin", 0),
        ("gripper", 12),
        ("hammer", 12),
        ("spear", 12),
        ("sword", 1),
        ("shield", 1),
        ("boat", 13),
        ("donkey", 14),
        ("food", 11),
        ("tools", 12),
        ("weapons", 1),
        ("ship", 11),
    ] {
        let id = good(&rules, g);
        assert_eq!(
            rules.goods.get(id).unwrap().transport_priority,
            Some(p),
            "{g}"
        );
    }
    // ECO-CAT-5.1: the 31 ware sort values in the listed order, 0 to 30.
    let order = [
        "wood",
        "plank",
        "stone",
        "pig",
        "corn",
        "flour",
        "fish",
        "flesh",
        "bread",
        "water",
        "beer",
        "coal",
        "ironOre",
        "gold",
        "iron",
        "coin",
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
        "sword",
        "shield",
        "boat",
    ];
    for (i, g) in order.iter().enumerate() {
        let id = good(&rules, g);
        assert_eq!(rules.goods.get(id).unwrap().sort, Some(i as i64), "{g}");
    }
    // Groups: 6 group goods; the boat carrier is typed a group (ECO-CAT-6).
    let groups = rules
        .goods
        .iter()
        .filter(|g| g.good_type == Some(GoodType::Group))
        .count();
    assert_eq!(groups, 6);
    // Rest times (ECO-CAT-6).
    for (job, rest) in [
        ("good_helper", Some(55.0)),
        ("good_woodcutter", Some(222.0)),
        ("good_fisher", Some(1111.0)),
        ("good_miller", Some(55.0)),
        ("good_slaughter", Some(222.0)),
        ("good_pigfarmer", Some(222.0)),
        ("good_shipBuilder", Some(222.0)),
        ("good_constructor", None),
    ] {
        assert_eq!(
            rules.goods.get(good(&rules, job)).unwrap().rest_time,
            rest,
            "{job}"
        );
    }
}

// ECO-CAT-4: military and storage buildings.
#[test]
fn military_and_storage_values() {
    let Some(rules) = rules() else { return };
    // (global, soldiers, territory, explore, coin stock)
    for (g, soldiers, territory, explore, coins) in [
        ("barracks", Some(2), Some(8), Some(11), Some(1)),
        ("guardHouse", Some(3), Some(9), Some(12), Some(2)),
        ("tower", Some(6), Some(10), Some(13), Some(4)),
        ("fortress", Some(9), Some(11), Some(14), Some(6)),
        ("castle", None, Some(9), Some(12), None),
        ("watchTower", None, None, Some(20), None),
    ] {
        let b = building(&rules, g);
        assert_eq!(b.soldier_limit, soldiers, "{g}");
        assert_eq!(b.territory_range, territory, "{g}");
        assert_eq!(b.explore_radius, explore, "{g}");
        assert_eq!(b.stock_limit, coins, "{g}");
    }
    for g in ["castle", "depot", "harbor"] {
        let b = building(&rules, g);
        assert_eq!(b.depot, Some(1), "{g}");
        assert_eq!(b.worker_limit, Some(0), "{g}");
    }
    // ECO-CAT-1.2: the woodcutter's radius is the second call's.
    assert_eq!(building(&rules, "woodCutter").working_radius, Some(6));
}

// ECO-PROD-2.2, 5.4, 7: the goods the production rules need are found.
#[test]
fn production_roles_resolve() {
    let Some(rules) = rules() else { return };
    let sim = rules.sim_rules().unwrap();
    let id = |g: &str| good(&rules, g);
    assert_eq!(
        sim.production.mine_food,
        [id("fish"), id("bread"), id("flesh")]
    );
    assert_eq!(sim.production.tool_slots.len(), 12);
    assert_eq!(sim.production.tool_slots[0], id("gripper"));
    assert_eq!(sim.production.tool_slots[9], id("hatchet"));
    assert_eq!(sim.production.tools_group, Some(id("tools")));
    // The catapult is flagged and takes stone.
    let catapult = sim.building_types.iter().find(|t| t.catapult).unwrap();
    assert_eq!(catapult.id, 0xA1445EF3);
    assert_eq!(sim.stone, Some(id("stone")));
    // Only the shipyard is switchable.
    let switchable: Vec<u32> = sim
        .building_types
        .iter()
        .filter(|t| t.switchable)
        .map(|t| t.id)
        .collect();
    assert_eq!(switchable, [0x78F8184E]);
}

// ECO-DIST-2.1: every consumer and good of the six distribution tables
// resolves in the scripts, and the table keys are building types.
#[test]
fn distribution_tables_resolve() {
    let Some(rules) = rules() else { return };
    let sim = rules.sim_rules().expect("sim rules");
    assert_eq!(sim.distribution.len(), s2_rules::DISTRIBUTION.len());
    for (table, (goods, entries)) in sim.distribution.iter().zip(s2_rules::DISTRIBUTION) {
        assert_eq!(table.goods.len(), goods.len(), "{goods:?}");
        assert_eq!(table.entries.len(), entries.len(), "{goods:?}");
    }
    assert!(sim.construction.is_some());
    // The food table serves fish, meat and bread; the plank table starts
    // with construction.
    assert_eq!(sim.distribution[4].goods.len(), 3);
    assert_eq!(sim.distribution[3].entries[0].0, sim.construction.unwrap());
}
