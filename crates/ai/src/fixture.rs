//! A synthetic game that needs no original data: our own small rule set
//! (castle, woodcutter, sawmill, stone pit, the military chain, ...) on flat
//! ground. The unit tests build on it, and `s2-app hash`, `soak` and `trace`
//! run it as `fixture:flat2` and `fixture:flat4`, which is how CI checks
//! determinism without a game install.

use std::collections::BTreeMap;

use s2_sim::grid::Point;
use s2_sim::map::{state, Terrain};
use s2_sim::{
    BuildingSize, BuildingType, DepositKind, FieldResource, GameSetup, GoodKind, GoodRules,
    MapDeposit, PlayerId, Rules, World,
};

use crate::config::{AiConfig, Names, Profile, DEFAULT_PROFILE};

pub const CASTLE: u32 = 100;
pub const WOODCUTTER: u32 = 101;
pub const FORESTER: u32 = 102;
pub const STONE_PIT: u32 = 103;
pub const FISHER: u32 = 104;
pub const SAWMILL: u32 = 105;
pub const BAKERY: u32 = 106;
pub const HUNTER: u32 = 107;
pub const DEPOT: u32 = 108;
pub const LOCKSMITH: u32 = 109;
pub const BARRACKS: u32 = 110;
pub const GUARD_HOUSE: u32 = 111;
pub const TOWER: u32 = 112;
pub const FORTRESS: u32 = 113;
pub const SMITHY: u32 = 114;
pub const BREWERY: u32 = 115;
pub const MINT: u32 = 116;
pub const CATAPULT: u32 = 117;
pub const HARBOR: u32 = 118;
pub const SHIPYARD: u32 = 119;

pub const PLANK: u32 = 7;
pub const STONE: u32 = 8;
pub const WOOD: u32 = 9;
pub const FISH: u32 = 10;
pub const BREAD: u32 = 11;
pub const COAL: u32 = 12;
pub const TOOLS: u32 = 13;
pub const SAW: u32 = 14;
pub const AXE: u32 = 15;
pub const IRON: u32 = 16;
pub const SWORD: u32 = 17;
pub const SHIELD: u32 = 18;
pub const BEER: u32 = 19;
pub const COIN: u32 = 20;
pub const GOLD: u32 = 21;
pub const BOAT: u32 = 22;
pub const SHIP: u32 = 23;
/// The soldier professions, ranks 0 to 4.
pub const SOLDIER: [u32; 5] = [30, 31, 32, 33, 34];

pub const TREE: u32 = 50;
pub const ROCK: u32 = 51;

pub fn flat(width: u32, height: u32) -> Terrain {
    let fields = (width * height) as usize;
    Terrain {
        width,
        height,
        heights: vec![0; fields],
        height_floor: -100,
        states: vec![state::FOR_BUILDING; fields],
        samples: Vec::new(),
    }
}

fn building(
    id: u32,
    size: BuildingSize,
    product: Option<u32>,
    radius: u32,
    costs: &[(u32, u32)],
) -> BuildingType {
    let mut t = BuildingType::new(id, size);
    t.product = product;
    t.working_radius = radius;
    t.costs = costs.to_vec();
    t
}

pub fn rules() -> Rules {
    let mut castle = BuildingType::new(CASTLE, BuildingSize::Large);
    castle.territory_range = 9;
    castle.explore_radius = 12;
    castle.warehouse = true;
    let mut forester = building(FORESTER, BuildingSize::Small, None, 6, &[(PLANK, 1)]);
    forester.planting_good = Some(WOOD);
    let mut depot = building(DEPOT, BuildingSize::Medium, None, 4, &[(STONE, 3)]);
    depot.warehouse = true;
    let military = |id: u32, size: BuildingSize, range: u32, soldiers: u32| {
        let mut t = building(id, size, None, 4, &[(STONE, 2)]);
        t.territory_range = range;
        t.explore_radius = range + 3;
        t.soldier_limit = soldiers;
        t
    };
    let mut catapult = building(CATAPULT, BuildingSize::Medium, None, 12, &[(STONE, 2)]);
    catapult.catapult = true;
    let mut harbor = building(HARBOR, BuildingSize::Harbor, None, 4, &[(STONE, 2)]);
    harbor.warehouse = true;
    let mut shipyard = building(SHIPYARD, BuildingSize::Medium, Some(BOAT), 8, &[(STONE, 2)]);
    shipyard.second_product = Some(BOAT);
    shipyard.switchable = true;
    shipyard.cycle = s2_sim::WorkCycle::Shipyard;
    let mut building_types = vec![
        castle,
        harbor,
        shipyard,
        military(BARRACKS, BuildingSize::Small, 8, 2),
        military(GUARD_HOUSE, BuildingSize::Small, 9, 3),
        military(TOWER, BuildingSize::Medium, 10, 6),
        military(FORTRESS, BuildingSize::Large, 11, 9),
        building(SMITHY, BuildingSize::Medium, Some(SWORD), 4, &[(STONE, 2)]),
        building(BREWERY, BuildingSize::Medium, Some(BEER), 4, &[(STONE, 2)]),
        building(MINT, BuildingSize::Medium, Some(COIN), 4, &[(STONE, 2)]),
        catapult,
        building(
            WOODCUTTER,
            BuildingSize::Small,
            Some(WOOD),
            6,
            &[(PLANK, 1)],
        ),
        forester,
        building(
            STONE_PIT,
            BuildingSize::Small,
            Some(STONE),
            8,
            &[(PLANK, 1)],
        ),
        building(FISHER, BuildingSize::Small, Some(FISH), 8, &[(PLANK, 1)]),
        building(SAWMILL, BuildingSize::Medium, Some(PLANK), 4, &[(STONE, 2)]),
        building(BAKERY, BuildingSize::Medium, Some(BREAD), 4, &[(STONE, 2)]),
        building(
            HUNTER,
            BuildingSize::Small,
            Some(FISH + 100),
            8,
            &[(PLANK, 1)],
        ),
        depot,
        building(
            LOCKSMITH,
            BuildingSize::Medium,
            Some(TOOLS),
            4,
            &[(STONE, 2)],
        ),
    ];
    // MIL-UPG-1.1: the upgrade chain.
    for t in &mut building_types {
        t.upgrade_to = match t.id {
            BARRACKS => Some(GUARD_HOUSE),
            GUARD_HOUSE => Some(TOWER),
            TOWER => Some(FORTRESS),
            _ => None,
        };
    }
    let good = |kind: GoodKind, costs: &[(u32, u32)]| GoodRules {
        kind,
        costs: costs.to_vec(),
        ..GoodRules::default()
    };
    let mut goods = BTreeMap::new();
    goods.insert(PLANK, good(GoodKind::Ware, &[(WOOD, 1)]));
    goods.insert(STONE, good(GoodKind::Ware, &[]));
    goods.insert(WOOD, good(GoodKind::Ware, &[]));
    goods.insert(FISH, good(GoodKind::Ware, &[]));
    goods.insert(BREAD, good(GoodKind::Ware, &[]));
    goods.insert(COAL, good(GoodKind::Ware, &[]));
    goods.insert(IRON, good(GoodKind::Ware, &[]));
    goods.insert(TOOLS, good(GoodKind::Group, &[(PLANK, 1), (IRON, 1)]));
    goods.insert(SAW, good(GoodKind::Ware, &[]));
    goods.insert(AXE, good(GoodKind::Ware, &[]));
    goods.insert(SWORD, good(GoodKind::Ware, &[(IRON, 1), (COAL, 1)]));
    goods.insert(SHIELD, good(GoodKind::Ware, &[(IRON, 1), (COAL, 1)]));
    goods.insert(BEER, good(GoodKind::Ware, &[]));
    goods.insert(COIN, good(GoodKind::Ware, &[(GOLD, 1), (COAL, 1)]));
    goods.insert(GOLD, good(GoodKind::Ware, &[]));
    goods.insert(BOAT, good(GoodKind::Ware, &[(PLANK, 1)]));
    goods.insert(SHIP, good(GoodKind::Ware, &[]));
    for g in SOLDIER {
        goods.insert(g, good(GoodKind::Settler, &[]));
    }
    let kind = |id: u32, good: u32| DepositKind {
        id,
        good: Some(good),
        next: None,
        elevation: 1,
        grow_ticks: 1001,
        blocking: false,
        life_time: None,
        needs_plantable: false,
        random_offset: true,
    };
    Rules {
        building_types,
        transport_priorities: goods.keys().map(|&g| (g, i64::from(g))).collect(),
        headquarters: Some(CASTLE),
        start_stock: vec![(PLANK, 20), (STONE, 20), (WOOD, 10), (SOLDIER[0], 5)],
        military: s2_sim::MilitaryRules {
            soldiers: SOLDIER.map(Some),
            max_health: [3, 4, 5, 6, 7],
            fighting_ticks: 5,
        },
        plank: Some(PLANK),
        stone: Some(STONE),
        goods,
        deposit_kinds: vec![kind(TREE, WOOD), kind(ROCK, STONE)],
        production: s2_sim::ProductionRules {
            tool_slots: vec![SAW, AXE],
            tools_group: Some(TOOLS),
            ..Default::default()
        },
        ..Rules::empty()
    }
}

/// What a synthetic map holds besides ground.
#[derive(Default)]
pub struct Extras {
    pub trees: Vec<Point>,
    pub rocks: Vec<Point>,
    pub fish: Vec<Point>,
}

/// A flat 96 x 96 map with a headquarters for every player in `players` at
/// well separated start positions.
pub fn world_with(players: &[u8], extras: &Extras) -> World {
    world_seeded(1, players, extras)
}

/// [`world_with`] with the game's seed.
pub fn world_seeded(seed: u64, players: &[u8], extras: &Extras) -> World {
    world_on(seed, players, extras, rules())
}

/// [`world_seeded`] with other rules.
pub fn world_on(seed: u64, players: &[u8], extras: &Extras, rules: Rules) -> World {
    world_on_terrain(seed, players, extras, rules, flat(96, 96))
}

/// [`world_on`] on `terrain` (96 x 96).
pub fn world_on_terrain(
    seed: u64,
    players: &[u8],
    extras: &Extras,
    rules: Rules,
    terrain: Terrain,
) -> World {
    let deposit = |kind: u32, f: &Point| MapDeposit {
        field: *f,
        kind,
        position: s2_sim::grid::field_sample(*f),
        ripe_in: 0,
    };
    let mut resources = vec![None; 96 * 96];
    for &(x, y) in &extras.fish {
        resources[(y * 96 + x) as usize] = Some(FieldResource {
            good: FISH,
            amount: 5,
        });
    }
    let setup = GameSetup {
        players: players
            .iter()
            .map(|&p| {
                (
                    PlayerId(p),
                    (20 + 40 * u32::from(p % 2), 20 + 40 * u32::from(p / 2)),
                )
            })
            .collect(),
        start_resources: 0,
        local_player: PlayerId(0),
        deposits: extras
            .trees
            .iter()
            .map(|f| deposit(TREE, f))
            .chain(extras.rocks.iter().map(|f| deposit(ROCK, f)))
            .collect(),
        resources,
        ..GameSetup::default()
    };
    World::new_game(seed, terrain, rules, &setup)
}

pub fn world(players: &[u8]) -> World {
    world_with(players, &Extras::default())
}

pub fn names() -> Names {
    Names {
        castle: CASTLE,
        plank: PLANK,
        stone: STONE,
        wood: WOOD,
        fish: FISH,
        bread: BREAD,
        coal: COAL,
        tools: TOOLS,
        woodcutter: WOODCUTTER,
        forester: FORESTER,
        stone_pit: STONE_PIT,
        fisher: FISHER,
        sawmill: SAWMILL,
        hunter: HUNTER,
        depot: DEPOT,
        harbor: HARBOR,
        shipyard: SHIPYARD,
        ship: SHIP,
        boat: BOAT,
        locksmith: LOCKSMITH,
        barracks: BARRACKS,
        guard_house: GUARD_HOUSE,
        tower: TOWER,
        fortress: FORTRESS,
        smithy: SMITHY,
        brewery: BREWERY,
        mint: MINT,
        catapult: CATAPULT,
        sword: SWORD,
        shield: SHIELD,
        beer: BEER,
        coin: COIN,
        gold: GOLD,
        iron: IRON,
        ..Names::default()
    }
}

pub fn config() -> AiConfig {
    config_for(&rules())
}

/// [`config`] for other rules.
pub fn config_for(rules: &Rules) -> AiConfig {
    let profiles: [Profile; 3] = [DEFAULT_PROFILE; 3];
    let mut priority = BTreeMap::new();
    priority.insert(BREAD, 70);
    priority.insert(COAL, 100);
    AiConfig::assemble(
        profiles,
        names(),
        BTreeMap::new(),
        priority,
        vec![
            PLANK, STONE, WOOD, FISH, BREAD, COAL, TOOLS, SAW, AXE, IRON, SWORD, SHIELD, BEER,
            COIN, GOLD,
        ],
        rules,
    )
}

/// The settlers of [`town_rules`]: helpers, builders, levellers and the
/// workers of the wood, stone and fish huts and the sawmill.
pub const HELPER: u32 = 40;
pub const BUILDER: u32 = 41;
pub const LEVELLER: u32 = 42;
pub const WOODCUTTER_JOB: u32 = 43;
pub const FORESTER_JOB: u32 = 44;
pub const STONEMASON: u32 = 45;
pub const FISHERMAN: u32 = 46;
pub const SAWYER: u32 = 47;
/// Their tools besides the saw and the axe.
pub const HAMMER: u32 = 24;
pub const SHOVEL: u32 = 25;
pub const PICK: u32 = 26;
pub const ROD: u32 = 27;

/// [`rules`] with people: the headquarters spawns helpers, builders and
/// levellers come out of its stock or are made from a helper and a tool,
/// and the woodcutter, forester, stone pit, fisher and sawmill have workers.
/// A small economy that runs on its own, for the soak and hash runs.
pub fn town_rules() -> Rules {
    let mut rules = rules();
    rules.roles.helper = Some(HELPER);
    rules.roles.builder = Some(BUILDER);
    rules.roles.leveller = Some(LEVELLER);
    let workers = [
        (WOODCUTTER, WOODCUTTER_JOB),
        (FORESTER, FORESTER_JOB),
        (STONE_PIT, STONEMASON),
        (FISHER, FISHERMAN),
        (SAWMILL, SAWYER),
    ];
    for t in &mut rules.building_types {
        if t.id == CASTLE {
            t.spawn = true;
        }
        t.worker = workers.iter().find(|&&(b, _)| b == t.id).map(|&(_, w)| w);
    }
    let settler = |costs: &[(u32, u32)]| GoodRules {
        kind: GoodKind::Settler,
        costs: costs.to_vec(),
        ..GoodRules::default()
    };
    let ware = || GoodRules {
        kind: GoodKind::Ware,
        ..GoodRules::default()
    };
    rules.goods.insert(HELPER, settler(&[(HELPER, 1)]));
    for (job, tool) in [
        (BUILDER, HAMMER),
        (LEVELLER, SHOVEL),
        (WOODCUTTER_JOB, AXE),
        (FORESTER_JOB, SHOVEL),
        (STONEMASON, PICK),
        (FISHERMAN, ROD),
        (SAWYER, SAW),
    ] {
        rules.goods.insert(job, settler(&[(tool, 1), (HELPER, 1)]));
    }
    for tool in [HAMMER, SHOVEL, PICK, ROD] {
        rules.goods.insert(tool, ware());
    }
    rules.transport_priorities = rules.goods.keys().map(|&g| (g, i64::from(g))).collect();
    rules.start_stock = vec![
        (PLANK, 30),
        (STONE, 25),
        (WOOD, 10),
        (FISH, 10),
        (SOLDIER[0], 5),
        (HELPER, 30),
        (BUILDER, 4),
        (LEVELLER, 2),
        (WOODCUTTER_JOB, 1),
        (STONEMASON, 1),
        (AXE, 3),
        (SAW, 2),
        (HAMMER, 3),
        (SHOVEL, 3),
        (PICK, 2),
        (ROD, 2),
    ];
    rules
}

/// The named fixtures with their AI configuration: `flat2` (players 0
/// and 1) and `flat4` (players 0 to 3) on [`rules`], `town2` and `town4`
/// the same on [`town_rules`]; each headquarters with a wood of 30 trees,
/// 8 rocks and a fish pond around it at fixed offsets.
pub fn named(name: &str, seed: u64) -> Option<(World, AiConfig)> {
    let (players, rules): (&[u8], Rules) = match name {
        "flat2" => (&[0, 1], rules()),
        "flat4" => (&[0, 1, 2, 3], rules()),
        "town2" => (&[0, 1], town_rules()),
        "town4" => (&[0, 1, 2, 3], town_rules()),
        _ => return None,
    };
    Some(game_on(players, seed, rules))
}

/// [`named`]'s game of `players` on other rules (the same wood, rocks and
/// pond around each headquarters).
pub fn game_on(players: &[u8], seed: u64, rules: Rules) -> (World, AiConfig) {
    let mut extras = Extras::default();
    for &p in players {
        let (hx, hy) = (20 + 40 * u32::from(p % 2), 20 + 40 * u32::from(p / 2));
        for i in 0..30u32 {
            extras.trees.push((hx + 5 + i % 6, hy + 2 + i / 6));
        }
        for i in 0..8u32 {
            extras.rocks.push((hx - 7 + i % 4, hy + 4 + i / 4));
        }
        for i in 0..6u32 {
            extras.fish.push((hx - 2 + i % 3, hy - 8 + i / 3));
        }
    }
    let config = config_for(&rules);
    (world_on(seed, players, &extras, rules), config)
}

/// The names of [`named`]'s fixtures.
pub const NAMES: [&str; 4] = ["flat2", "flat4", "town2", "town4"];

/// Building type and good names by id, for reports ([`crate::run::Runner`]
/// takes them as a map for lookups).
#[allow(clippy::disallowed_types)]
pub fn labels() -> std::collections::HashMap<u32, String> {
    [
        (CASTLE, "castle"),
        (WOODCUTTER, "woodcutter"),
        (FORESTER, "forester"),
        (STONE_PIT, "stonePit"),
        (FISHER, "fisher"),
        (SAWMILL, "sawmill"),
        (BAKERY, "bakery"),
        (HUNTER, "hunter"),
        (DEPOT, "depot"),
        (LOCKSMITH, "locksmith"),
        (BARRACKS, "barracks"),
        (GUARD_HOUSE, "guardHouse"),
        (TOWER, "tower"),
        (FORTRESS, "fortress"),
        (SMITHY, "smithy"),
        (BREWERY, "brewery"),
        (MINT, "mint"),
        (CATAPULT, "catapult"),
        (HARBOR, "harbor"),
        (SHIPYARD, "shipyard"),
        (PLANK, "plank"),
        (STONE, "stone"),
        (WOOD, "wood"),
        (FISH, "fish"),
    ]
    .into_iter()
    .map(|(id, n)| (id, n.to_string()))
    .collect()
}
