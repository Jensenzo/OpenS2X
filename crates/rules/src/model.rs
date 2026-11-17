//! The rule tables the property scripts fill in (`specs/LUA-properties-api.md`).
//!
//! Plain data. A value is `None` (or a list empty) until a script sets it: the
//! engine's built-in defaults are not specified, so none are invented here.
//! Numbers the spec types as "number" are kept as the scripts give them; the
//! simulation converts them to its own fixed-point types when it uses them.

use std::collections::HashMap;

/// The permanent 32-bit id of a script-created object (LUA-PROP-2.1). Map and
/// save files store these ids.
pub type Id = u32;

/// Number of soldier ranks (LUA-PROP-10: rank 0..4).
pub const SOLDIER_RANKS: usize = 5;

/// An object with an id, creatable from the id alone.
pub trait Record {
    fn new(id: Id) -> Self;
    fn id(&self) -> Id;
}

/// The objects of one family, in the order the scripts created them, with
/// lookup by id.
#[derive(Debug, Clone, PartialEq)]
pub struct Table<T> {
    items: Vec<T>,
    index: HashMap<Id, usize>,
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<T: Record> Table<T> {
    /// Add a new object. `None` if the id is already taken.
    pub(crate) fn create(&mut self, id: Id) -> Option<&mut T> {
        if self.index.contains_key(&id) {
            return None;
        }
        self.index.insert(id, self.items.len());
        self.items.push(T::new(id));
        self.items.last_mut()
    }

    pub(crate) fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.index.get(&id).map(|&i| &mut self.items[i])
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        self.index.get(&id).map(|&i| &self.items[i])
    }

    pub fn contains(&self, id: Id) -> bool {
        self.index.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// In creation order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }
}

impl<'a, T> IntoIterator for &'a Table<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

macro_rules! record {
    ($type:ident) => {
        impl Record for $type {
            fn new(id: Id) -> Self {
                Self {
                    id,
                    ..Default::default()
                }
            }

            fn id(&self) -> Id {
                self.id
            }
        }
    };
}

/// The enumerations of LUA-PROP-3. The scripts pass these as numbers.
macro_rules! script_enum {
    ($(#[$meta:meta])* $type:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $type {
            $($variant = $value),+
        }

        impl $type {
            /// `None` for a number the spec does not list.
            pub fn from_number(value: i64) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

script_enum!(
    /// Good type.
    GoodType { Invalid = -1, Ware = 0, Settler = 1, Group = 2 }
);
script_enum!(
    /// Plot size class of a building.
    BuildingSize { Small = 0, Medium = 1, Large = 2, Mine = 3, Harbor = 4 }
);
script_enum!(
    /// Side of the door; the numbers are grid directions (MAP-GRID-1.6).
    DoorDirection { West = 3, NorthWest = 4, NorthEast = 5 }
);
script_enum!(Tribe { Romans = 0, Africans = 1, Chinese = 2 });
script_enum!(
    /// Tree family a forester plants on a terrain type.
    TreeSet { Caribbean = 0, Coniferous = 1, BlackForest = 2, Lava = 3 }
);
script_enum!(DoodadType { Landscape = 0, Sign = 1 });
script_enum!(SoundType { Ambient = 0, Music = 1, Gui = 2, Effect = 3 });
script_enum!(MilitaryMode { Safe = 0, Normal = 1, Aggressive = 2 });

/// A good and how many of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount {
    pub good: Id,
    pub count: u32,
}

/// A ware, a settler profession or a ware group (LUA-PROP-5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Good {
    pub id: Id,
    pub name: Option<String>,
    pub short_name: Option<String>,
    /// Column and row of the icon in the icon atlas.
    pub icon: Option<(i64, i64)>,
    pub good_type: Option<GoodType>,
    pub sort: Option<i64>,
    pub transport_priority: Option<i64>,
    pub ai_priority: Option<i64>,
    /// The ware group this good is a member of.
    pub group: Option<Id>,
    /// What is consumed to make one unit; for a profession, the base settler
    /// and the tool.
    pub costs: Vec<Amount>,
    /// Ticks a worker of this profession pauses between two work cycles.
    pub rest_time: Option<f64>,
    pub plantable: Option<bool>,
    /// Gathered from deposits standing on the map.
    pub deposits_enabled: Option<i64>,
    /// Exists as a per-field amount in the map's resource layer.
    pub fields_enabled: Option<i64>,
    pub value: Option<f64>,
}
record!(Good);

/// Where a builder stands while working on a building (`bp_setConstructionPosition`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstructionPosition {
    pub tribe: Tribe,
    /// 0-based spot number.
    pub index: u32,
    /// Offset from the building in field units.
    pub x: f64,
    pub y: f64,
}

/// A building type (LUA-PROP-6).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Building {
    pub id: Id,
    pub name: Option<String>,
    pub gfx_name: Option<String>,
    pub icon: Option<(i64, i64)>,
    pub sort: Option<i64>,
    pub size: Option<BuildingSize>,
    /// Wares for the first building phase.
    pub frame_costs: Vec<Amount>,
    /// Wares for the second building phase.
    pub costs: Vec<Amount>,
    /// Ticks; -1 = none.
    pub construction_time: Option<f64>,
    pub construction_positions: Vec<ConstructionPosition>,
    pub door_directions: Vec<(Tribe, DoorDirection)>,
    /// Profession that operates the building.
    pub worker: Option<Id>,
    pub worker_limit: Option<u32>,
    /// Product and amount per cycle.
    pub goods: Option<Amount>,
    /// Alternative product.
    pub second_goods: Option<Amount>,
    pub switchable_production: Option<bool>,
    /// Ticks of one production cycle.
    pub production_time: Option<f64>,
    pub stock_limit: Option<u32>,
    /// Fields.
    pub working_radius: Option<u32>,
    pub planting_good: Option<Id>,
    pub depot: Option<i64>,
    pub spawn: Option<i64>,
    pub harbor: Option<bool>,
    pub catapult: Option<bool>,
    pub soldier_limit: Option<u32>,
    /// Fields.
    pub territory_range: Option<u32>,
    /// Fields.
    pub explore_radius: Option<u32>,
    pub remains: Option<Id>,
    pub ai_score: Option<i64>,
    pub ai_threat: Option<i64>,
    pub health: Option<f64>,
    /// Hook H6, a project's only (`reborn.bp_addInput` in an overlay): what
    /// one product of this type consumes, in place of its product's cost
    /// list. Empty in the original's scripts.
    pub inputs: Vec<Amount>,
    /// Hook W12, a project's only (`reborn.bp_setAiAlternative` in an
    /// overlay): the computer players may build this type for a square
    /// good none of their cells offers a square for. None in the original's
    /// scripts.
    pub ai_alternative: Option<AiAlternative>,
    /// Hook W20, a project's only (`reborn.bp_addRecipe` and
    /// `reborn.bp_addRecipeInput` in an overlay): the recipes a player
    /// chooses between per building of this type, in order. Empty in the
    /// original's scripts.
    pub recipes: Vec<RecipeChoice>,
    /// Hook W15, a project's only (`reborn.bp_setDeepening` and its
    /// companions in an overlay): how an exhausted mine of this type is
    /// deepened. None in the original's scripts.
    pub deepening: Option<DeepeningRule>,
}
record!(Building);

/// Hook W15: a mine type's deepening. Not part of the original game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeepeningRule {
    /// Of each field's first amount, the share that comes back, per mille.
    pub share_per_mille: u32,
    /// The most deepenings of one mine.
    pub most: u32,
    /// What one deepening costs.
    pub costs: Vec<Amount>,
    /// The food one product eats after the first deepening, the second, ...
    pub food: Vec<u32>,
}

/// Hook W20: one recipe of a type's set: the product one cycle makes and
/// what it consumes. Not part of the original game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeChoice {
    pub product: Id,
    pub inputs: Vec<Amount>,
}

/// Hook W12: when the computer players build a project's type in place of
/// the mine of a square good (`docs/extending.md`, "The computer players and
/// a project's type"). Not part of the original game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiAlternative {
    /// The good the type stands in for (coal for a charcoal burner).
    pub good: Id,
    /// Wanted only while the player's stock of `good` is below this.
    pub stock_below: u32,
    /// The input good that must be in stock, and how much of it at least.
    pub input: Id,
    pub input_at_least: u32,
    /// The most of the type one player has placed or planned.
    pub most: u32,
}

/// A terrain type (LUA-PROP-7). Maps store its id per field (FMT-S2M-4.3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pattern {
    pub id: Id,
    pub name: Option<String>,
    /// Texture name.
    pub gfx_name: Option<String>,
    pub blocked: Option<bool>,
    pub for_building: Option<bool>,
    pub for_planting: Option<bool>,
    pub for_mining: Option<bool>,
    pub for_harbor: Option<bool>,
    pub tree_type: Option<TreeSet>,
    /// Red, green, blue, 0..255 each.
    pub minimap_color: Option<[u8; 3]>,
}
record!(Pattern);

/// Something standing on a field that yields a good (LUA-PROP-8).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Deposit {
    pub id: Id,
    pub name: Option<String>,
    pub gfx_name: Option<String>,
    pub gfx_variant: Option<i64>,
    pub good: Option<Id>,
    pub blocking: Option<bool>,
    /// The deposit type it turns into after one unit was taken.
    pub next_deposit: Option<Id>,
    pub elevation_size: Option<i64>,
    pub underground_patterns: Vec<Id>,
    pub random_fine_pos_offset: Option<bool>,
    pub needs_plantable_pattern: Option<bool>,
    /// Size gained per tick.
    pub growing_speed: Option<f64>,
    pub growing_start_size: Option<f64>,
    /// Ticks.
    pub life_time: Option<i64>,
    pub doodad_after_destruction: Option<Id>,
}
record!(Deposit);

/// Decoration or a geologist sign (LUA-PROP-9).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Doodad {
    pub id: Id,
    pub name: Option<String>,
    pub gfx_name: Option<String>,
    pub blocking: Option<bool>,
    pub doodad_type: Option<DoodadType>,
    /// Ticks.
    pub life_time: Option<i64>,
    pub delete_by_street: Option<bool>,
    pub on_water: Option<bool>,
}
record!(Doodad);

/// The ruin a destroyed building leaves (LUA-PROP-9).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Remains {
    pub id: Id,
    pub name: Option<String>,
    pub gfx_name: Option<String>,
    /// Ticks.
    pub burning_time: Option<i64>,
    pub doodad_after_finished: Option<Id>,
}
record!(Remains);

/// An animal type (LUA-PROP-11).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Animal {
    pub id: Id,
    pub name: Option<String>,
    pub gfx_name: Option<String>,
    /// Walking progress per tick.
    pub movement_speed: Option<f64>,
    /// Fields.
    pub move_distance: Option<u32>,
    /// Fields.
    pub range: Option<u32>,
    pub avoid_buildings: Option<bool>,
    pub idle_time_factor: Option<f64>,
}
record!(Animal);

/// A street type (LUA-PROP-11).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Street {
    pub id: Id,
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub mesh: Option<String>,
    pub height: Option<f64>,
    pub length: Option<f64>,
    pub costs: Vec<Amount>,
}
record!(Street);

/// A sound (LUA-PROP-11). Maps refer to ambient sounds by id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sound {
    pub id: Id,
    pub name: Option<String>,
    /// Several files are random variants.
    pub file_names: Vec<String>,
    pub sound_type: Option<SoundType>,
    pub looping: Option<bool>,
    /// 0..255.
    pub volume: Option<i64>,
    pub priority: Option<i64>,
    pub positioned: Option<bool>,
    pub min_distance_3d: Option<f64>,
    pub max_distance_3d: Option<f64>,
    pub distance_2d: Option<f64>,
    pub music_tribe: Option<Tribe>,
    pub percussion: Option<bool>,
}
record!(Sound);

/// A computer player profile (LUA-PROP-11). Its id is the profile number.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiProfile {
    pub id: Id,
    pub name: Option<String>,
    pub military_mode: Option<MilitaryMode>,
    /// 0..1.
    pub attack_strength: Option<f64>,
    pub intercepting_factor: Option<f64>,
    pub start_sawmill_count: Option<u32>,
    pub maximum_smithies: Option<u32>,
    pub maximum_mints: Option<u32>,
    pub street_optimizing: Option<bool>,
    pub route_optimizing: Option<bool>,
    pub catapults: Option<bool>,
    pub coin_arrangement: Option<bool>,
}
record!(AiProfile);

/// Global settings (LUA-PROP-4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    /// Seconds per logic tick.
    pub game_speed: Option<f64>,
    /// Walking progress per tick along one field step.
    pub settler_speed: Option<f64>,
    /// Ticks.
    pub pickup_time: Option<f64>,
    /// Ticks.
    pub drop_time: Option<f64>,
    /// Ticks.
    pub bulldoze_time: Option<f64>,
    /// Ticks of one construction work step.
    pub construct_time: Option<f64>,
    pub flag_package_limit: Option<u32>,
    /// Fields.
    pub fishing_distance: Option<u32>,
    pub catapult_rotation_time_factor: Option<f64>,
    pub catapult_flight_time_factor: Option<f64>,
    /// Ticks.
    pub settler_dying_time: Option<i64>,
    pub settler_skeleton_doodad: Option<Id>,
    pub border_pattern: Option<Id>,
}

/// Military settings (LUA-PROP-10).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Military {
    /// Ticks of one round of a duel.
    pub fighting_time: Option<f64>,
    /// Profession per rank.
    pub soldiers: [Option<Id>; SOLDIER_RANKS],
    /// Hit points per rank.
    pub soldier_max_health: [Option<i64>; SOLDIER_RANKS],
    /// (from, to) pairs of building types.
    pub building_upgrades: Vec<(Id, Id)>,
}

/// Ship settings (LUA-PROP-10).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ship {
    /// Sailing progress per tick.
    pub speed: Option<f64>,
    pub frame_costs: Vec<Amount>,
    pub costs: Vec<Amount>,
    /// Ticks of one ship building work step.
    pub construct_time: Option<f64>,
    pub size: Option<i64>,
    pub stock_limit: Option<u32>,
}

/// A geologist sign rule (`si_addSignDoodadId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sign {
    /// `None` is the "nothing found" sign.
    pub good: Option<Id>,
    /// Planted when the amount found is at least this.
    pub threshold: u32,
    pub doodad: Id,
}

/// Everything the property scripts define.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleSet {
    pub settings: Settings,
    /// Wares, professions and ware groups; see [`RuleSet::wares`] and
    /// [`RuleSet::jobs`].
    pub goods: Table<Good>,
    pub buildings: Table<Building>,
    /// Terrain types.
    pub patterns: Table<Pattern>,
    pub deposits: Table<Deposit>,
    pub doodads: Table<Doodad>,
    pub remains: Table<Remains>,
    pub animals: Table<Animal>,
    pub streets: Table<Street>,
    pub sounds: Table<Sound>,
    pub ai_profiles: Table<AiProfile>,
    pub military: Military,
    pub ship: Ship,
    pub signs: Vec<Sign>,
    /// Script globals that hold the id of a good or building, by name
    /// (`good_plank`, LUA-PROP-1.6).
    pub names: std::collections::BTreeMap<String, Id>,
    /// Hook W12: the building types a project's overlay created (an id the
    /// original's scripts do not have). Empty without overlays.
    pub project_buildings: std::collections::BTreeSet<Id>,
    /// Hook W16 (`reborn.cfg_setRecipeKeep` in an overlay): how many of a
    /// good, or of each member of a group, a player keeps back from the
    /// orders of a recipe group. Empty without overlays.
    pub recipe_keep: std::collections::BTreeMap<Id, u32>,
    /// Hook W19 (`reborn.cfg_setPaving` in an overlay): the good a road's
    /// pavement takes per field and the speed on it in percent. None
    /// without overlays.
    pub paving: Option<(Id, u32, u32)>,
    /// Hook W20's companion (`reborn.dist_addConsumer` in an overlay): a
    /// building type added to the distribution table of a good (ECO-DIST-2.1),
    /// with its default weight in thousandths: (good, building, weight), in
    /// call order. Empty without overlays.
    pub dist_consumers: Vec<(Id, Id, u32)>,
    /// Hook W25 (`reborn.ai_set*` in an overlay): when the computer players
    /// use the project's rules. Empty without overlays: they play the
    /// original's game.
    pub ai_project: AiProjectRules,
}

/// Hook W25: the computer players' use of a project's rules, each none by
/// default (they never deepen, pave or choose a recipe).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiProjectRules {
    /// `reborn.ai_setDeepen(food_at_least)`: a computer player deepens an
    /// exhausted mine of a type with a deepening while its stock of food
    /// (fish, meat and bread together) is at least this, one at a time.
    pub deepen_food: Option<u32>,
    /// W25b: `reborn.ai_setDeepen(food_at_least, false)`: an exhausted mine
    /// not deepened in the destruction run is torn down as the original's
    /// rule does; without (`true` or no second value) it is kept for a later
    /// run, as since W25. A mine being deepened is kept either way.
    pub deepen_tears_down: bool,
    /// `reborn.ai_setPaving(traffic_at_least, reserve)`: a computer player
    /// paves its road of the highest traffic average, when that is at least
    /// the first value, while it holds more of the paving good than the
    /// reserve plus the road's cost, one at a time.
    pub paving: Option<(u32, u32)>,
    /// `reborn.ai_setRecipeRule(building, index, good, above)`: a computer
    /// player's building of the type works recipe `index` while the player
    /// holds more than `above` of the good (a group good: its members
    /// together, each counting only its stock above its kept stock,
    /// `reborn.cfg_setRecipeKeep`, W25b), else its first; in call order, the
    /// last for a type wins.
    pub recipes: Vec<AiRecipeRule>,
}

/// Hook W25: one `reborn.ai_setRecipeRule`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AiRecipeRule {
    pub building: Id,
    pub index: u32,
    pub good: Id,
    pub above: u32,
}

impl AiProjectRules {
    /// Whether any rule is set.
    pub fn any(&self) -> bool {
        self.deepen_food.is_some() || self.paving.is_some() || !self.recipes.is_empty()
    }
}

impl RuleSet {
    fn goods_of_type(&self, good_type: GoodType) -> impl Iterator<Item = &Good> {
        self.goods
            .iter()
            .filter(move |good| good.good_type == Some(good_type))
    }

    /// Goods of type ware.
    pub fn wares(&self) -> impl Iterator<Item = &Good> {
        self.goods_of_type(GoodType::Ware)
    }

    /// Goods whose type the scripts never set. The engine's default type is
    /// not specified (Q-033).
    pub fn untyped_goods(&self) -> impl Iterator<Item = &Good> {
        self.goods.iter().filter(|good| good.good_type.is_none())
    }

    /// Goods of type settler: the professions.
    pub fn jobs(&self) -> impl Iterator<Item = &Good> {
        self.goods_of_type(GoodType::Settler)
    }

    /// Goods of type ware group.
    pub fn ware_groups(&self) -> impl Iterator<Item = &Good> {
        self.goods_of_type(GoodType::Group)
    }

    /// The first building with this name.
    pub fn building_named(&self, name: &str) -> Option<&Building> {
        self.buildings
            .iter()
            .find(|building| building.name.as_deref() == Some(name))
    }

    /// The id a script global holds (`good_plank`), if it is a good or
    /// building.
    pub fn id_named(&self, global: &str) -> Option<Id> {
        self.names.get(global).copied()
    }

    /// The first good with this name.
    pub fn good_named(&self, name: &str) -> Option<&Good> {
        self.goods
            .iter()
            .find(|good| good.name.as_deref() == Some(name))
    }

    /// References to objects no script created, as readable descriptions.
    /// Empty for a consistent rule set.
    pub fn dangling_references(&self) -> Vec<String> {
        let mut found = Vec::new();
        let mut check = |ok: bool, owner: &str, id: Id, what: &str, target: Id| {
            if !ok {
                found.push(format!(
                    "{owner} {id:08X}: {what} {target:08X} does not exist"
                ));
            }
        };
        let good = |id: Id| self.goods.contains(id);

        for g in &self.goods {
            if let Some(group) = g.group {
                check(good(group), "good", g.id, "group", group);
            }
            for cost in &g.costs {
                check(good(cost.good), "good", g.id, "cost good", cost.good);
            }
        }
        for b in &self.buildings {
            let amounts = b.frame_costs.iter().chain(&b.costs).chain(&b.inputs);
            for amount in amounts.chain(&b.goods).chain(&b.second_goods) {
                check(good(amount.good), "building", b.id, "good", amount.good);
            }
            let alternative = b.ai_alternative.iter().flat_map(|a| [a.good, a.input]);
            let recipes = b
                .recipes
                .iter()
                .flat_map(|r| std::iter::once(r.product).chain(r.inputs.iter().map(|a| a.good)));
            let deepening = b
                .deepening
                .iter()
                .flat_map(|d| d.costs.iter().map(|a| a.good));
            let alternative = alternative.chain(recipes).chain(deepening);
            for target in b
                .worker
                .iter()
                .chain(&b.planting_good)
                .copied()
                .chain(alternative)
            {
                check(good(target), "building", b.id, "good", target);
            }
            if let Some(remains) = b.remains {
                let ok = self.remains.contains(remains);
                check(ok, "building", b.id, "remains", remains);
            }
        }
        for d in &self.deposits {
            if let Some(target) = d.good {
                check(good(target), "deposit", d.id, "good", target);
            }
            if let Some(next) = d.next_deposit {
                let ok = self.deposits.contains(next);
                check(ok, "deposit", d.id, "next deposit", next);
            }
            for &pattern in &d.underground_patterns {
                let ok = self.patterns.contains(pattern);
                check(ok, "deposit", d.id, "pattern", pattern);
            }
            if let Some(doodad) = d.doodad_after_destruction {
                let ok = self.doodads.contains(doodad);
                check(ok, "deposit", d.id, "doodad", doodad);
            }
        }
        for r in &self.remains {
            if let Some(doodad) = r.doodad_after_finished {
                let ok = self.doodads.contains(doodad);
                check(ok, "remains", r.id, "doodad", doodad);
            }
        }
        for s in &self.streets {
            for cost in &s.costs {
                check(good(cost.good), "street", s.id, "cost good", cost.good);
            }
        }
        if let Some(doodad) = self.settings.settler_skeleton_doodad {
            let ok = self.doodads.contains(doodad);
            check(ok, "settings", 0, "skeleton doodad", doodad);
        }
        if let Some(pattern) = self.settings.border_pattern {
            let ok = self.patterns.contains(pattern);
            check(ok, "settings", 0, "border pattern", pattern);
        }
        for soldier in self.military.soldiers.iter().flatten() {
            check(good(*soldier), "military", 0, "soldier good", *soldier);
        }
        for &(from, to) in &self.military.building_upgrades {
            for building in [from, to] {
                let ok = self.buildings.contains(building);
                check(ok, "military", 0, "upgrade building", building);
            }
        }
        for amount in self.ship.frame_costs.iter().chain(&self.ship.costs) {
            check(good(amount.good), "ship", 0, "cost good", amount.good);
        }
        for sign in &self.signs {
            if let Some(target) = sign.good {
                check(good(target), "sign", sign.doodad, "good", target);
            }
            let ok = self.doodads.contains(sign.doodad);
            check(ok, "sign", sign.doodad, "doodad", sign.doodad);
        }
        found
    }
}
