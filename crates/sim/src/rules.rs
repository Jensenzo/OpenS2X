//! What the simulation takes from the rule scripts: [`Rules`] and its parts,
//! in the simulation's own units.

use std::collections::BTreeMap;

use crate::{
    AnimalKind, BuildingType, DepositKind, DistributionRules, MilitaryRules, ProductionRules,
    SpecialistRules,
};

/// Global settings the simulation uses, in its own units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// TRN-MOVE-1.1: walking progress per tick, in millionths of a step.
    pub settler_speed: u32,
    /// Ticks of the pickup-time and drop-time actions (TRN-CARRY intro:
    /// timed actions last `ceil(duration)` ticks).
    pub pickup_ticks: u32,
    pub drop_ticks: u32,
    /// TRN-ROAD-5.2: wares one flag holds.
    pub flag_package_limit: u32,
    /// ECO-HQ-4.3, 4.6: ticks of the bulldoze-time and construct-time
    /// actions.
    pub bulldoze_ticks: u32,
    pub construct_ticks: u32,
    /// ECO-FOOD-4.2: `cfg_setFishingDistance`, fields around a fishing spot
    /// that may hold the fish.
    pub fishing_distance: u32,
    /// MIL-CON-5.4: ticks per turn step of a catapult (plus one), and per
    /// field of a stone's flight (`cfg_setCatapultRotationTimeFactor`,
    /// `cfg_setCatapultFlightTimeFactor`).
    pub catapult_turn_ticks: u32,
    pub catapult_flight_ticks: u32,
}

impl Settings {
    /// The values of the shipped scripts as the specs give them: speed 0.045
    /// (TRN-MOVE-1.1), pickup and drop time 3.3, so 4 ticks (TRN-CARRY), flag
    /// limit 6 (TRN-ROAD-5.2), bulldoze and construct time 55 (ECO-HQ-4.3,
    /// 4.6).
    pub fn shipped() -> Self {
        Self {
            settler_speed: 45_000,
            pickup_ticks: 4,
            drop_ticks: 4,
            flag_package_limit: 6,
            bulldoze_ticks: 55,
            construct_ticks: 55,
            fishing_distance: 1,
            catapult_turn_ticks: 11,
            catapult_flight_ticks: 11,
        }
    }
}

/// What the simulation takes from the rule scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub building_types: Vec<BuildingType>,
    /// Transport priority per good id (`gp_setTransportPriority`); smaller
    /// goes first.
    pub transport_priorities: BTreeMap<u32, i64>,
    pub settings: Settings,
    /// ECO-HQ-1.1: the building type every player starts with (`castle`).
    pub headquarters: Option<u32>,
    /// ECO-HQ-2.1: the headquarters' base stock, (good, count) in order.
    pub start_stock: Vec<(u32, u32)>,
    /// ECO-HQ-4.5: the goods plank and stone, which the main part of a
    /// construction site takes in this order.
    pub plank: Option<u32>,
    pub stone: Option<u32>,
    /// What the scripts say about each good, by id.
    pub goods: BTreeMap<u32, GoodRules>,
    pub roles: Roles,
    /// Deposit types (`deposits.lua`).
    pub deposit_kinds: Vec<DepositKind>,
    /// ECO-OUT-4.5: the tree kinds of tree types 0 to 3, and the kind for
    /// any other type.
    pub tree_families: Vec<Vec<u32>>,
    pub default_tree: Option<u32>,
    /// ECO-PROD: goods by role in the production rules.
    pub production: ProductionRules,
    /// ECO-FOOD-7.4: the ship record of the scripts.
    pub ship: ShipRules,
    /// ECO-DIST-2.1: the six distribution tables with their defaults.
    pub distribution: Vec<DistributionRules>,
    /// ECO-DIST-2.1: the key of every building under construction
    /// (`construction`).
    pub construction: Option<u32>,
    /// MIL-SOL-1.1: the five soldier ranks.
    pub military: MilitaryRules,
    /// ECO-FOOD-5.7: the animal kinds.
    pub animals: Vec<AnimalKind>,
    /// ECO-SPC: geologists and scouts.
    pub specialists: SpecialistRules,
    /// A project's recipe for a building type (hook H6, `docs/extending.md`
    /// "New building types"): what one product of the type consumes, (good,
    /// count) in order, in place of its product's cost list (ECO-PROD-2.1).
    /// Empty in the original's rules.
    pub recipes: BTreeMap<u32, Vec<(u32, u32)>>,
    /// A project's recipes a player chooses between per building (hook W20,
    /// `docs/extending.md` "Recipes a player chooses"): by building type, in
    /// the order the overlay added them. Empty in the original's rules.
    pub recipe_sets: BTreeMap<u32, Vec<Recipe>>,
    /// A project's deepening of a mine type (hook W15, `docs/extending.md`
    /// "Deepening a mine"), by building type. Empty in the original's rules.
    pub deepenings: BTreeMap<u32, crate::Deepening>,
    /// A project's group goods in recipes (hook W16, `docs/extending.md`
    /// "A group good in a recipe"): each group a recipe takes, with its
    /// member goods in good order. An order of such a group is served by
    /// any member a warehouse holds. Empty in the original's rules.
    pub recipe_groups: BTreeMap<u32, Vec<u32>>,
    /// Hook W16: how many of each good a player keeps back from the orders
    /// of a recipe group (by member good). Empty in the original's rules.
    pub recipe_keep: BTreeMap<u32, u32>,
    /// A project's paving rule (hook W19, `docs/extending.md` "Paved
    /// roads"). None in the original's rules.
    pub paving: Option<crate::PavingRule>,
}

/// Hook W20: one recipe of a type's set: the product one cycle makes and
/// what it consumes, (good, count) in order (in place of the product's
/// cost list, ECO-PROD-2.1). Not part of the original game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recipe {
    pub product: u32,
    pub inputs: Vec<(u32, u32)>,
}

/// ECO-FOOD-7.4: what the ship record (`ship.lua`) gives the shipyard's
/// work: one trip per plank of a part.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShipRules {
    /// `sh_setSpeed` in millionths of a step per tick (SHIP-3.1), 180 000
    /// for 0.18.
    pub speed: u32,
    /// `sh_setStockLimit`: cargo entries a ship holds (SHIP-4.4), 12.
    pub stock_limit: u32,
    /// Planks of the frame part (`sh_addFrameCost`), 4 in the shipped game.
    pub frame_planks: u32,
    /// Planks of the main part (`sh_addCost`), 8.
    pub main_planks: u32,
    /// `sh_setConstructTime`: ticks of one work step, 55.
    pub construct_ticks: u32,
}

/// What the simulation takes from a good's script entry (`gp_*`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoodRules {
    /// `gp_addCost`: for a profession the helper and tool one is made of
    /// (ECO-REQ-3.3), for a ware the inputs of one unit (ECO-REQ-4.2).
    pub costs: Vec<(u32, u32)>,
    /// ECO-WORK-3.9: ticks a rest of this profession lasts (rest time + 1).
    pub rest_ticks: u32,
    /// `gp_enableDeposits`: gathered from deposits (ECO-OUT-2.2).
    pub gathered: bool,
    /// `gp_enableFields`: a per-field resource of the map (ECO-FOOD-2.3).
    pub resource: bool,
    /// The good's type (LUA-PROP-5).
    pub kind: GoodKind,
    /// UI-GAME-8.5e: the production counter a building making this good adds
    /// to, by the good or its group; none for corn, flour, pigs and donkeys.
    pub production_counter: Option<u8>,
}

/// A good's type: ware, settler profession, ware group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GoodKind {
    #[default]
    Other,
    Ware,
    Settler,
    Group,
}

/// Goods the rules give a fixed part (ECO-REQ-1, 5.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Roles {
    /// `good_helper`: road carriers.
    pub helper: Option<u32>,
    /// `good_constructor`
    pub builder: Option<u32>,
    /// `good_bulldozer`
    pub leveller: Option<u32>,
    /// `good_donkey`: the second carrier of a busy road.
    pub donkey: Option<u32>,
    /// `coin`: never taken as a free ware (ECO-REQ-5.3).
    pub coin: Option<u32>,
    /// `good_shipHelper`: the carrier of a waterway (ECO-REQ-1.1).
    pub boat_carrier: Option<u32>,
}

impl Rules {
    /// No building types or goods, shipped settings.
    pub fn empty() -> Self {
        Self {
            building_types: Vec::new(),
            transport_priorities: BTreeMap::new(),
            settings: Settings::shipped(),
            headquarters: None,
            start_stock: Vec::new(),
            plank: None,
            stone: None,
            goods: BTreeMap::new(),
            roles: Roles::default(),
            deposit_kinds: Vec::new(),
            tree_families: Vec::new(),
            default_tree: None,
            production: ProductionRules::default(),
            ship: ShipRules {
                speed: 180_000,
                stock_limit: 12,
                ..ShipRules::default()
            },
            distribution: Vec::new(),
            construction: None,
            military: MilitaryRules::default(),
            animals: Vec::new(),
            specialists: SpecialistRules::default(),
            recipes: BTreeMap::new(),
            recipe_sets: BTreeMap::new(),
            deepenings: BTreeMap::new(),
            recipe_groups: BTreeMap::new(),
            recipe_keep: BTreeMap::new(),
            paving: None,
        }
    }
}
