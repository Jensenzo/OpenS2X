//! What the computer opponent takes from the rule scripts: the three
//! profiles of `ai.lua`, the score, threat and priority weights, and the
//! script names of the goods and buildings it reasons about
//! (`specs/AI-overview.md` AI-1).

use std::collections::{BTreeMap, BTreeSet};

use s2_rules::model::{MilitaryMode, RuleSet};
use s2_sim::{BuildingSize, BuildingType, Rules};

/// An id that matches no good and no building.
pub const NONE: u32 = u32::MAX;

/// AI-1.2 military mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Safe,
    Normal,
    Aggressive,
}

/// AI-1.2: one row of the profile table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Profile {
    pub mode: Mode,
    pub start_sawmills: u32,
    pub max_smithies: u32,
    pub max_mints: u32,
    pub attack_strength: f32,
    pub intercepting: f32,
    pub street_optimizing: bool,
    pub route_optimizing: bool,
    pub catapults: bool,
    pub coin_arrangement: bool,
}

/// AI-1.3: `bp_setAiScore` and `bp_setAiThreat` of a building type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Weights {
    pub score: i64,
    pub threat: i64,
}

/// The script globals the AI reasons about, resolved once. A name the scripts
/// do not define resolves to [`NONE`].
#[derive(Debug, Clone)]
pub struct Names {
    pub wood: u32,
    pub plank: u32,
    pub stone: u32,
    pub fish: u32,
    pub flesh: u32,
    pub bread: u32,
    pub beer: u32,
    pub coal: u32,
    pub iron: u32,
    pub iron_ore: u32,
    pub gold: u32,
    pub sword: u32,
    pub shield: u32,
    pub coin: u32,
    pub boat: u32,
    pub ship: u32,
    pub corn: u32,
    /// AI-CON-1.1c: the underground resource the resource map leaves out.
    pub water: u32,
    /// The ware `donkey` and the settler `good_donkey`.
    pub donkey: u32,
    pub donkey_settler: u32,
    pub tools: u32,
    pub food: u32,
    pub castle: u32,
    pub sawmill: u32,
    pub stone_pit: u32,
    pub woodcutter: u32,
    pub forester: u32,
    pub hunter: u32,
    pub fisher: u32,
    pub slaughterhouse: u32,
    pub depot: u32,
    pub harbor: u32,
    pub shipyard: u32,
    pub smithy: u32,
    pub brewery: u32,
    pub mint: u32,
    pub farm: u32,
    pub locksmith: u32,
    pub catapult: u32,
    pub barracks: u32,
    pub guard_house: u32,
    pub tower: u32,
    pub fortress: u32,
}

impl Default for Names {
    /// Every name unresolved.
    fn default() -> Self {
        Self {
            wood: NONE,
            plank: NONE,
            stone: NONE,
            fish: NONE,
            flesh: NONE,
            bread: NONE,
            beer: NONE,
            coal: NONE,
            iron: NONE,
            iron_ore: NONE,
            gold: NONE,
            sword: NONE,
            shield: NONE,
            coin: NONE,
            boat: NONE,
            ship: NONE,
            corn: NONE,
            water: NONE,
            donkey: NONE,
            donkey_settler: NONE,
            tools: NONE,
            food: NONE,
            castle: NONE,
            sawmill: NONE,
            stone_pit: NONE,
            woodcutter: NONE,
            forester: NONE,
            hunter: NONE,
            fisher: NONE,
            slaughterhouse: NONE,
            depot: NONE,
            harbor: NONE,
            shipyard: NONE,
            smithy: NONE,
            brewery: NONE,
            mint: NONE,
            farm: NONE,
            locksmith: NONE,
            catapult: NONE,
            barracks: NONE,
            guard_house: NONE,
            tower: NONE,
            fortress: NONE,
        }
    }
}

/// Hook W12 (`docs/extending.md`, "The computer players and a project's
/// type"): a project's building type the computer players build for a
/// square good none of their cells offers a square for, read from the
/// overlay's `reborn.bp_setAiAlternative`. Not part of the original game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiAlternative {
    pub building: u32,
    /// Wanted only while the player's stock of the good (on the roads too)
    /// is below this.
    pub stock_below: u32,
    /// The input good and the stock of it that must be in the warehouses.
    pub input: u32,
    pub input_at_least: u32,
    /// The most of the type one player has placed or planned.
    pub most: u32,
}

/// AI-MIL-1.1 (T25): the military-map score of a building type without
/// `bp_setAiScore`.
pub const DEFAULT_AI_SCORE: i64 = 20;

/// The rules the computer opponent runs on.
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Profiles 1 to 3 (index 0 to 2).
    pub profiles: [Profile; 3],
    pub names: Names,
    /// AI-1.3, by building type id.
    pub weights: BTreeMap<u32, Weights>,
    /// `gp_setAiPriority`, by good id.
    pub priority: BTreeMap<u32, i64>,
    /// All goods in id order (the "good-id order" of the specs).
    pub goods: Vec<u32>,
    /// The simulation's rules (building types in script order, goods).
    pub sim: Rules,
    /// AI-CON-1.4: the producer building type of a good, normal table and
    /// mine table (Q-074).
    pub producer: BTreeMap<u32, u32>,
    pub mine_producer: BTreeMap<u32, u32>,
    /// AI-CON-1.5 flags of a good.
    pub deposits_goods: Vec<u32>,
    pub fields_goods: Vec<u32>,
    /// ECO-PROD-7: the tool of each of the 12 slots.
    pub tool_slots: Vec<u32>,
    /// AI-CON-3.3 (T77, Q-371): the settler goods (the professions and
    /// soldiers; goods of type Settler), never a consumer good. The ware
    /// groups (food, tools, weapons, ship, donkey) are consumers.
    pub settler_goods: Vec<u32>,
    /// Hook W12: the building types a project's overlay created; they stay
    /// out of the producer tables. Empty for the original's rules.
    pub project_types: BTreeSet<u32>,
    /// Hook W12: the alternative producers, by the good they stand in for.
    /// Empty for the original's rules and every overlay without one.
    pub alternatives: BTreeMap<u32, AiAlternative>,
    /// Hook W25: when the computer players use the project's rules
    /// (deepening, paving, recipes). Empty for the original's rules.
    pub project_rules: s2_rules::model::AiProjectRules,
    /// Index of a building type id in `sim.building_types`.
    type_index: BTreeMap<u32, usize>,
}

impl AiConfig {
    /// Read the weights and profiles of `rules`; `sim` is
    /// `rules.sim_rules()`.
    pub fn new(rules: &RuleSet, sim: &Rules) -> Self {
        let id = |name: &str| rules.id_named(name).unwrap_or(NONE);
        let names = Names {
            wood: id("wood"),
            plank: id("plank"),
            stone: id("stone"),
            fish: id("fish"),
            flesh: id("flesh"),
            bread: id("bread"),
            beer: id("beer"),
            coal: id("coal"),
            iron: id("iron"),
            iron_ore: id("ironOre"),
            gold: id("gold"),
            sword: id("sword"),
            shield: id("shield"),
            coin: id("coin"),
            boat: id("boat"),
            ship: id("ship"),
            corn: id("corn"),
            water: id("water"),
            donkey: id("donkey"),
            donkey_settler: id("good_donkey"),
            tools: id("tools"),
            food: id("food"),
            castle: id("castle"),
            sawmill: id("sawMill"),
            stone_pit: id("stonePit"),
            woodcutter: id("woodCutter"),
            forester: id("forester"),
            hunter: id("hunter"),
            fisher: id("fisher"),
            slaughterhouse: id("slaughterHouse"),
            depot: id("depot"),
            harbor: id("harbor"),
            shipyard: id("shipYard"),
            smithy: id("smithy"),
            brewery: id("brewery"),
            mint: id("mint"),
            farm: id("farm"),
            locksmith: id("locksmithery"),
            catapult: id("catapult"),
            barracks: id("barracks"),
            guard_house: id("guardHouse"),
            tower: id("tower"),
            fortress: id("fortress"),
        };
        let mut profiles = [DEFAULT_PROFILE; 3];
        for (slot, profile) in profiles.iter_mut().enumerate() {
            if let Some(p) = rules.ai_profiles.get(slot as u32 + 1) {
                *profile = Profile {
                    mode: match p.military_mode {
                        Some(MilitaryMode::Safe) | None => Mode::Safe,
                        Some(MilitaryMode::Normal) => Mode::Normal,
                        Some(MilitaryMode::Aggressive) => Mode::Aggressive,
                    },
                    start_sawmills: p.start_sawmill_count.unwrap_or(1),
                    max_smithies: p.maximum_smithies.unwrap_or(0),
                    max_mints: p.maximum_mints.unwrap_or(0),
                    attack_strength: p.attack_strength.unwrap_or(1.0) as f32,
                    intercepting: p.intercepting_factor.unwrap_or(1.0) as f32,
                    street_optimizing: p.street_optimizing.unwrap_or(false),
                    route_optimizing: p.route_optimizing.unwrap_or(false),
                    catapults: p.catapults.unwrap_or(false),
                    coin_arrangement: p.coin_arrangement.unwrap_or(false),
                };
            }
        }
        let weights = rules
            .buildings
            .iter()
            .map(|b| {
                (
                    b.id,
                    Weights {
                        // AI-MIL-1.1 (T25): a type without bp_setAiScore
                        // scores 20, the type's default; the threat's is 0.
                        score: b.ai_score.unwrap_or(DEFAULT_AI_SCORE),
                        threat: b.ai_threat.unwrap_or(0),
                    },
                )
            })
            .collect();
        let priority = rules
            .goods
            .iter()
            .filter_map(|g| g.ai_priority.map(|p| (g.id, p)))
            .collect();
        let mut goods: Vec<u32> = rules.goods.iter().map(|g| g.id).collect();
        goods.sort_unstable();
        let mut config = Self::assemble(profiles, names, weights, priority, goods, sim);
        // W12: a project's types and alternatives (none in the original's
        // rules, so the tables stay as assembled). Two types for one good:
        // the lower type id's.
        let mut alternatives = BTreeMap::new();
        let mut declaring: Vec<_> = rules
            .buildings
            .iter()
            .filter_map(|b| b.ai_alternative.map(|a| (b.id, a)))
            .collect();
        declaring.sort_by_key(|&(id, _)| id);
        for (building, a) in declaring {
            alternatives.entry(a.good).or_insert(AiAlternative {
                building,
                stock_below: a.stock_below,
                input: a.input,
                input_at_least: a.input_at_least,
                most: a.most,
            });
        }
        if !rules.project_buildings.is_empty() || !alternatives.is_empty() {
            config.set_project(rules.project_buildings.clone(), alternatives);
        }
        // W25: none in the original's rules.
        config.project_rules = rules.ai_project.clone();
        config
    }

    /// Hook W12: a project's building types (left out of the producer
    /// tables, which are built again) and its alternative producers.
    pub fn set_project(
        &mut self,
        types: BTreeSet<u32>,
        alternatives: BTreeMap<u32, AiAlternative>,
    ) {
        self.project_types = types;
        self.alternatives = alternatives;
        self.producer.clear();
        self.mine_producer.clear();
        self.build_tables();
    }

    /// An AI configuration from its parts, for the scripts' values read by
    /// [`AiConfig::new`] and for tests that build their own small rules.
    pub fn assemble(
        profiles: [Profile; 3],
        names: Names,
        weights: BTreeMap<u32, Weights>,
        priority: BTreeMap<u32, i64>,
        goods: Vec<u32>,
        sim: &Rules,
    ) -> Self {
        let mut config = Self {
            profiles,
            names,
            weights,
            priority,
            goods,
            sim: sim.clone(),
            producer: BTreeMap::new(),
            mine_producer: BTreeMap::new(),
            deposits_goods: Vec::new(),
            fields_goods: Vec::new(),
            tool_slots: sim.production.tool_slots.clone(),
            settler_goods: sim
                .goods
                .iter()
                .filter(|(_, g)| g.kind == s2_sim::GoodKind::Settler)
                .map(|(&id, _)| id)
                .collect(),
            project_types: BTreeSet::new(),
            alternatives: BTreeMap::new(),
            project_rules: Default::default(),
            type_index: sim
                .building_types
                .iter()
                .enumerate()
                .map(|(i, t)| (t.id, i))
                .collect(),
        };
        config.build_tables();
        config
    }

    /// AI-CON-1.4 (Q-074): the two producer tables, from every good in
    /// good-id order. The mine table holds the first building type, in type-id
    /// order, among the mines and the fisher whose products include the
    /// good; the normal table the first among all other types. Flesh has the
    /// slaughterhouse in the normal table and no mine entry. AI-CON-1.5:
    /// goods flags. Hook W12: a project's types are in neither table.
    fn build_tables(&mut self) {
        let n = self.names.clone();
        let mut types: Vec<&BuildingType> = self
            .sim
            .building_types
            .iter()
            .filter(|t| !self.project_types.contains(&t.id))
            .collect();
        types.sort_by_key(|t| t.id);
        let makes = |t: &BuildingType, g: u32| t.product == Some(g) || t.second_product == Some(g);
        let is_mine = |t: &BuildingType| t.size == BuildingSize::Mine || t.id == n.fisher;
        for &g in &self.goods {
            if g == n.flesh {
                if n.slaughterhouse != NONE {
                    self.producer.insert(g, n.slaughterhouse);
                }
                continue;
            }
            if let Some(t) = types.iter().find(|t| is_mine(t) && makes(t, g)) {
                self.mine_producer.insert(g, t.id);
            }
            if let Some(t) = types.iter().find(|t| !is_mine(t) && makes(t, g)) {
                self.producer.insert(g, t.id);
            }
        }
        self.deposits_goods = vec![n.wood, n.stone, n.corn];
        self.fields_goods = vec![n.stone, n.fish, n.coal, n.iron_ore, n.gold];
    }

    /// Profile `number` (1 to 3).
    pub fn profile(&self, number: u8) -> Option<&Profile> {
        (1..=3)
            .contains(&number)
            .then(|| &self.profiles[usize::from(number) - 1])
    }

    /// AI-1.3 score and threat of a building type.
    pub fn weights_of(&self, building_type: u32) -> Weights {
        // AI-MIL-1.1 (T25): a type's defaults, score 20 and threat 0.
        self.weights
            .get(&building_type)
            .copied()
            .unwrap_or(Weights {
                score: DEFAULT_AI_SCORE,
                threat: 0,
            })
    }

    /// `gp_setAiPriority` of a good, 0 if unset.
    pub fn priority_of(&self, good: u32) -> i64 {
        self.priority.get(&good).copied().unwrap_or(0)
    }

    /// The sim type of a building type id.
    pub fn building(&self, id: u32) -> Option<&BuildingType> {
        self.type_index
            .get(&id)
            .map(|&i| &self.sim.building_types[i])
    }

    /// The goods a (group) good stands for: food is fish, flesh and bread,
    /// tools the twelve tools; any other good is its own only member.
    pub fn group_members(&self, good: u32) -> Vec<u32> {
        let n = &self.names;
        if good == n.food {
            vec![n.fish, n.flesh, n.bread]
        } else if good == n.tools {
            self.tool_slots.clone()
        } else {
            vec![good]
        }
    }

    /// AI-CON-1.4, 3.2, 7.3: whether a production of `product` supplies
    /// `good`: its good is `good` or has `good` as its group good (bread,
    /// fish and meat supply food).
    pub fn supplies(&self, product: u32, good: u32) -> bool {
        product == good || self.group_of(product) == Some(good)
    }

    /// The ware group a good belongs to (food or tools), if any.
    pub fn group_of(&self, good: u32) -> Option<u32> {
        let n = &self.names;
        if [n.fish, n.flesh, n.bread].contains(&good) {
            Some(n.food)
        } else if self.tool_slots.contains(&good) {
            Some(n.tools)
        } else {
            None
        }
    }

    /// AI-CON-1.5: "resource goods": deposits or fields goods except corn.
    pub fn is_resource_good(&self, good: u32) -> bool {
        good != self.names.corn
            && (self.deposits_goods.contains(&good) || self.fields_goods.contains(&good))
    }

    /// The inputs of one unit of `good` (`gp_addCost`).
    pub fn inputs(&self, good: u32) -> &[(u32, u32)] {
        self.sim
            .goods
            .get(&good)
            .map_or(&[], |g| g.costs.as_slice())
    }

    /// The inputs of one product `product` of building type `building`: the
    /// first recipe of a project's recipe set for the type (hook W20,
    /// `s2_sim::Rules::recipe_sets`: the one a building works until a
    /// player chooses another, which the computer players never do), else
    /// its recipe (hook H6, `s2_sim::Rules::recipes`), else the product's
    /// [`inputs`](Self::inputs). The original's rules have neither.
    pub fn type_inputs(&self, building: u32, product: u32) -> &[(u32, u32)] {
        if let Some(first) = self.sim.recipe_sets.get(&building).and_then(|s| s.first()) {
            return &first.inputs;
        }
        self.sim
            .recipes
            .get(&building)
            .map_or_else(|| self.inputs(product), Vec::as_slice)
    }
}

/// Used when the scripts define no profile at all.
pub const DEFAULT_PROFILE: Profile = Profile {
    mode: Mode::Safe,
    start_sawmills: 1,
    max_smithies: 0,
    max_mints: 0,
    attack_strength: 0.5,
    intercepting: 0.5,
    street_optimizing: false,
    route_optimizing: false,
    catapults: false,
    coin_arrangement: false,
};
