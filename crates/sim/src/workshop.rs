//! Production state and rules of workshops and mines
//! (`specs/ECO-production-chains.md`, ECO-PROD): the on/off and product
//! switches, input orders, mine food and ore, product choice and the
//! metalworks' tool priorities.

use crate::grid::Point;
use crate::{PlayerId, Rejection, World};

use crate::production::WorkCycle;

/// ECO-PROD-6.1: a mine keeps up to this many of each food on order.
const MINE_FOOD_ORDER: u32 = 2;
/// ECO-PROD-3.3: the amount of a resource field never goes above this.
pub(crate) const ORE_CAP: u8 = 7;

/// ECO-PROD-1.1: the production state of a building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Production {
    /// ECO-PROD-1.2: production on/off.
    pub enabled: bool,
    /// ECO-PROD-1.3: the product switch.
    pub switched: bool,
    /// ECO-PROD-5.1: products taken so far.
    pub count: u32,
}

impl Production {
    pub const fn new() -> Self {
        Self {
            enabled: true,
            switched: false,
            count: 0,
        }
    }
}

impl Default for Production {
    fn default() -> Self {
        Self::new()
    }
}

/// What the rules name for the production rules: goods by role (ECO-PROD-2.2,
/// ECO-PROD-7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProductionRules {
    /// ECO-PROD-2.2: the foods a mine eats, in the order it eats them
    /// (fish, bread, ham).
    pub mine_food: Vec<u32>,
    /// ECO-PROD-7: the tool of each of the 12 priority slots.
    pub tool_slots: Vec<u32>,
    /// ECO-PROD-5.4: the ware group `tools`.
    pub tools_group: Option<u32>,
}

/// The underground or fish resource of a field (FMT-S2M-4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldResource {
    pub good: u32,
    pub amount: u8,
}

/// ECO-PROD-7: number of tool slots.
pub const TOOL_SLOTS: usize = 12;
/// ECO-PROD-7: the default weight of each slot, in slot order.
pub const DEFAULT_TOOL_WEIGHTS: [f32; TOOL_SLOTS] =
    [0.1, 0.4, 0.2, 0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.2, 0.3];
/// ECO-PROD-7.3: a weight is never below this.
pub const MIN_TOOL_WEIGHT: f32 = 0.001;

/// ECO-PROD-7.1: one player's tool priorities. Weights and accumulators are
/// 32-bit floats whose float32 sums decide the choice (ECO-PROD-7.2), kept
/// as their bit patterns so that the state stays comparable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolPriorities {
    weights: [u32; TOOL_SLOTS],
    accumulators: [u32; TOOL_SLOTS],
    /// The exclusive slot, if one is set.
    pub exclusive: Option<u8>,
}

impl Default for ToolPriorities {
    fn default() -> Self {
        Self {
            weights: DEFAULT_TOOL_WEIGHTS.map(f32::to_bits),
            accumulators: [0f32.to_bits(); TOOL_SLOTS],
            exclusive: None,
        }
    }
}

impl ToolPriorities {
    pub fn weight(&self, slot: usize) -> f32 {
        f32::from_bits(self.weights[slot])
    }

    pub fn accumulator(&self, slot: usize) -> f32 {
        f32::from_bits(self.accumulators[slot])
    }

    /// ECO-PROD-7.2: the slot of the next tool.
    pub fn choose(&mut self) -> Option<usize> {
        if let Some(slot) = self.exclusive {
            return Some(usize::from(slot));
        }
        let mut best: Option<(usize, f32)> = None;
        for slot in 0..TOOL_SLOTS {
            let value = self.accumulator(slot) + self.weight(slot);
            self.accumulators[slot] = value.to_bits();
            if value > best.map_or(0.0, |(_, v)| v) {
                best = Some((slot, value));
            }
        }
        let (slot, _) = best?;
        self.accumulators[slot] = 0f32.to_bits();
        Some(slot)
    }

    /// ECO-PROD-7.3: set a weight, to the minimum if it is not above it.
    pub fn set_weight(&mut self, slot: usize, value: f32) {
        let value = if value > MIN_TOOL_WEIGHT {
            value
        } else {
            MIN_TOOL_WEIGHT
        };
        self.weights[slot] = value.to_bits();
    }

    /// ECO-PROD-7.3: default weights, no exclusive slot, accumulators kept.
    pub fn reset(&mut self) {
        self.weights = Self::default().weights;
        self.exclusive = None;
    }
}

fn count(list: &[(u32, u32)], good: u32) -> u32 {
    list.iter()
        .filter(|&&(g, _)| g == good)
        .map(|&(_, n)| n)
        .sum()
}

impl World {
    /// The production state of the finished building on `at`.
    pub fn production_at(&self, at: Point) -> Option<Production> {
        self.building_at(at)
            .filter(|b| !b.under_construction)
            .map(|b| b.production)
    }

    /// ECO-PROD-7: the player's tool priorities.
    pub fn tool_priorities(&self, player: PlayerId) -> ToolPriorities {
        self.tool_priorities
            .get(&player)
            .copied()
            .unwrap_or_default()
    }

    /// FMT-S2M-4.5: the resource of a field.
    pub fn resource_at(&self, field: Point) -> Option<FieldResource> {
        if !self.terrain.contains(field) {
            return None;
        }
        self.resources
            .get(self.terrain.index(field))
            .copied()
            .flatten()
    }

    /// The field a mine is working at the moment (ECO-PROD-3.3).
    pub fn ore_field(&self, mine: Point) -> Option<Point> {
        self.ore_fields.get(&mine).copied()
    }

    /// A finished building of `player`, or the rejection.
    fn own_finished(&self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        match self.building_at(at) {
            Some(b) if b.owner == player && !b.under_construction => Ok(()),
            _ => Err(Rejection::NoBuilding),
        }
    }

    fn production_mut(&mut self, at: Point) -> &mut Production {
        let index = self.terrain.index(at);
        &mut self.buildings[index]
            .as_mut()
            .expect("a building")
            .production
    }

    /// ECO-PROD-1.2: toggle production; off cancels the open input orders
    /// (6.4), on files them (6.1).
    pub(crate) fn toggle_production(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<(), Rejection> {
        self.own_finished(player, at)?;
        let production = self.production_mut(at);
        production.enabled = !production.enabled;
        if production.enabled {
            self.file_input_orders(at);
        } else {
            self.orders.retain(|o| o.target.at != at);
        }
        Ok(())
    }

    /// W20: the recipe of its type's set the building on `at` works.
    fn chosen_recipe_of(&self, at: Point) -> Option<&crate::rules::Recipe> {
        let building = self.building_at(at)?;
        let set = self.recipe_sets.get(&building.building_type)?;
        set.get(self.chosen_recipes.get(&at).copied().unwrap_or(0) as usize)
    }

    /// W20 (`Command::ChooseRecipe`): the building on `at` works recipe
    /// `index` of its type's set from now on. The open orders of goods the
    /// new recipe does not take are withdrawn (wares already on their way
    /// are still delivered, as when production is switched off, ECO-PROD-1.2),
    /// and its inputs are ordered up to the limits (ECO-PROD-6.1).
    pub(crate) fn choose_recipe(
        &mut self,
        player: PlayerId,
        at: Point,
        index: u32,
    ) -> Result<(), Rejection> {
        self.own_finished(player, at)?;
        let building_type = self.building_at(at).expect("finished").building_type;
        let inputs: Vec<u32> = match self.recipe_sets.get(&building_type) {
            Some(set) if (index as usize) < set.len() => {
                set[index as usize].inputs.iter().map(|&(g, _)| g).collect()
            }
            _ => return Err(Rejection::NotARecipe),
        };
        if index == 0 {
            self.chosen_recipes.remove(&at);
        } else {
            self.chosen_recipes.insert(at, index);
        }
        self.orders
            .retain(|o| o.target.at != at || inputs.contains(&o.good));
        self.file_input_orders(at);
        Ok(())
    }

    /// ECO-PROD-1.3: toggle the product switch. The command does not look at
    /// `switchable`.
    pub(crate) fn toggle_switch(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        self.own_finished(player, at)?;
        let production = self.production_mut(at);
        production.switched = !production.switched;
        Ok(())
    }

    /// ECO-PROD-7.3: the commands on a player's tool priorities.
    pub(crate) fn tool_command(
        &mut self,
        player: PlayerId,
        change: impl FnOnce(&mut ToolPriorities) -> Result<(), Rejection>,
    ) -> Result<(), Rejection> {
        let mut priorities = self.tool_priorities(player);
        change(&mut priorities)?;
        self.tool_priorities.insert(player, priorities);
        Ok(())
    }

    // ---- Inputs (ECO-PROD-2, 6) ----

    /// ECO-PROD-2.1: what one product consumes (a workshop's cost list of
    /// product slot 0); a mine eats food instead (2.2), outdoor workers
    /// consume nothing (2.3).
    pub(crate) fn input_list(&self, at: Point) -> Vec<(u32, u32)> {
        let building = self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        // ECO-PROD-2.1; the donkey farm and the shipyard use the cost list of
        // their product too (ECO-FOOD-6.1, 7.2).
        if !matches!(
            kind.cycle,
            WorkCycle::Workshop | WorkCycle::DonkeyFarm | WorkCycle::Shipyard
        ) {
            return Vec::new();
        }
        // W20: the recipe the building works of its type's set comes first.
        if let Some(recipe) = self.chosen_recipe_of(at) {
            return recipe.inputs.clone();
        }
        // H6: a project's recipe for the type comes next.
        if let Some(recipe) = self.recipes.get(&building.building_type) {
            return recipe.clone();
        }
        kind.product
            .and_then(|p| self.goods.get(&p))
            .map(|g| g.costs.clone())
            .unwrap_or_default()
    }

    /// ECO-PROD-6.1: file orders up to the limits, if production is on.
    pub(crate) fn file_input_orders(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        if !building.production.enabled {
            return;
        }
        for (good, limit) in self.input_limits(at) {
            let have = count(self.inputs(at), good);
            // ECO-PROD-6.1, 6.4 (T21): the orders counted include those
            // whose ware is already on its way.
            let open = self.open_orders(at, good) + self.orders_on_their_way(at, good);
            for _ in 0..limit.saturating_sub(have + open) {
                self.order(at, good);
            }
        }
    }

    /// ECO-PROD-6.1, UI-GAME-4.1a (f): the most of `good` the building on
    /// `at` orders (the capacity its window shows under the input): a
    /// mine's foods 2 each, the catapult's stone and any other input the
    /// type's stock limit; `None` for a good it does not order.
    pub fn input_limit(&self, at: Point, good: u32) -> Option<u32> {
        self.building_at(at)?;
        self.input_limits(at)
            .into_iter()
            .find(|&(g, _)| g == good)
            .map(|(_, limit)| limit)
    }

    /// ECO-PROD-6.1: the goods the building on `at` orders, in filing order,
    /// each with the amount it orders up to.
    fn input_limits(&self, at: Point) -> Vec<(u32, u32)> {
        let building = self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        if kind.cycle == WorkCycle::Mine {
            // ECO-REQ-4.2 (T30): filed fish, meat, bread (the eating order
            // of `mine_food` is fish, bread, meat, ECO-PROD-2.2).
            let mut filed = self.mine_food.clone();
            if filed.len() == 3 {
                filed.swap(1, 2);
            }
            // W15: a deepened mine orders its extra food per product too.
            let extra = self.mine_food_per_product(at) - 1;
            filed
                .iter()
                .map(|&g| (g, MINE_FOOD_ORDER + extra))
                .collect()
        } else if kind.catapult {
            match (self.stone, kind.stock_limit) {
                (Some(stone), Some(limit)) => vec![(stone, limit)],
                _ => Vec::new(),
            }
        } else if let Some(limit) = kind.stock_limit {
            self.input_list(at)
                .iter()
                .map(|&(g, _)| (g, limit))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// ECO-PROD-6.1, 6.4 (T21): the orders of `good` for the building on
    /// `at` whose ware is on its way: lying or carried towards it, or in the
    /// hands of a warehouse's helper carrying it out for it.
    pub(crate) fn orders_on_their_way(&self, at: Point, good: u32) -> u32 {
        // W16: a member on its way for a recipe group's order counts for
        // the group (its order's good).
        let wares = self
            .wares()
            .filter(|(_, w)| {
                (w.good == good || w.order.is_some_and(|o| o.good == good))
                    && w.target.is_some_and(|t| t.at == at)
            })
            .count();
        let helpers = self
            .settlers()
            .filter(|(_, s)| match &s.job {
                crate::SettlerJob::CarryOut {
                    target: Some(t),
                    order,
                    ..
                } if t.at == at => {
                    s.carrying == Some(good) || order.is_some_and(|o| o.good == good)
                }
                _ => false,
            })
            .count();
        (wares + helpers) as u32
    }

    /// ECO-PROD-3.1 (a) to (c): enabled, a worker, the inputs in stock.
    pub(crate) fn can_produce(&self, at: Point) -> bool {
        let building = self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        if !building.production.enabled || building.workers < 1 {
            return false;
        }
        let stock = self.inputs(at);
        match kind.cycle {
            // W15: a deepened mine begins only with all its food in stock.
            WorkCycle::Mine => {
                let food: u32 = self.mine_food.iter().map(|&g| count(stock, g)).sum();
                food >= self.mine_food_per_product(at)
            }
            _ => self
                .input_list(at)
                .iter()
                .all(|&(good, n)| count(stock, good) >= n),
        }
    }

    /// ECO-PROD-4.1, 4.2: the work action ended: consume the inputs and, if
    /// production is on, order them again, one order per unit.
    pub(crate) fn consume_inputs(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let enabled = building.production.enabled;
        if kind.cycle == WorkCycle::Mine {
            // ECO-PROD-2.2: the first food of fish, bread, ham in stock (W15:
            // a deepened mine eats that many times, each the first).
            for _ in 0..self.mine_food_per_product(at) {
                let eaten = self
                    .mine_food
                    .iter()
                    .copied()
                    .find(|&g| count(self.inputs(at), g) >= 1);
                if let Some(good) = eaten {
                    self.take_input(at, good);
                    if enabled {
                        self.order(at, good);
                    }
                }
            }
            return;
        }
        for (good, n) in self.input_list(at) {
            for _ in 0..n {
                self.take_input(at, good);
            }
            if enabled {
                for _ in 0..n {
                    self.order(at, good);
                }
            }
        }
    }

    fn take_input(&mut self, at: Point, good: u32) {
        if let Some((_, c)) = self
            .inputs
            .entry(at)
            .or_default()
            .iter_mut()
            .find(|(g, c)| *g == good && *c > 0)
        {
            *c -= 1;
            self.used(good, 1);
        }
    }

    // ---- Product (ECO-PROD-5) ----

    /// ECO-PROD-5.1 to 5.5: the good the worker takes out, or `None` for
    /// "none" (no ware is made).
    pub(crate) fn take_product(&mut self, at: Point) -> Option<u32> {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let production = building.production;
        let second =
            production.switched || (kind.second_product.is_some() && production.count % 2 == 1);
        // W20: a type with a recipe set makes its chosen recipe's product.
        let decided = if let Some(recipe) = self.chosen_recipe_of(at) {
            Some(recipe.product)
        } else if second {
            kind.second_product
        } else {
            kind.product
        };
        let production = self.production_mut(at);
        production.count = production.count.wrapping_add(1);
        match decided {
            Some(good) if Some(good) == self.tools_group => {
                let owner = building.owner;
                let mut priorities = self.tool_priorities(owner);
                let slot = priorities.choose();
                self.tool_priorities.insert(owner, priorities);
                slot.and_then(|s| self.tool_slots.get(s).copied())
            }
            other => other,
        }
    }

    // ---- Ore (ECO-PROD-3.3) ----

    /// ECO-PROD-3.3: the miner begins a cycle: one unit is taken from the
    /// mine's current ore field. False if the mine has none. A field that
    /// no longer holds the mine's good (another mine took the last unit,
    /// ECO-FOOD-2.5) is dropped; the next search (ECO-FOOD-2.2) finds a new
    /// one.
    pub(crate) fn take_ore(&mut self, at: Point) -> bool {
        let building = *self.building_at(at).expect("a building");
        let Some(product) = self.building_types[&building.building_type].product else {
            return false;
        };
        let Some(field) = self.ore_fields.get(&at).copied() else {
            return false;
        };
        let holds = self
            .resource_at(field)
            .is_some_and(|r| r.good == product && r.amount > 0);
        if !holds {
            self.ore_fields.remove(&at);
            return false;
        }
        self.take_unit(field);
        // ECO-FOOD-2.7: the unit that empties the field clears it.
        if self.resource_at(field).is_none() {
            self.ore_fields.remove(&at);
        }
        true
    }

    /// Put a resource on a field (map loading and tests).
    pub fn set_resource(&mut self, field: Point, resource: Option<FieldResource>) {
        if self.resources.len() != self.terrain.len() {
            self.resources = vec![None; self.terrain.len()];
        }
        let index = self.terrain.index(field);
        self.resources[index] = resource;
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
crate::snap_struct!(ToolPriorities {
    weights,
    accumulators,
    exclusive
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Walker;
    use crate::requests::tests as req;
    use crate::test_support::*;
    use crate::{
        BuildingSize, BuildingType, Command, GoodRules, Rules, Settler, SettlerJob, Tick, Work,
        World,
    };

    const WOOD: u32 = 6;
    const PLANK: u32 = req::PLANK;
    const STONE: u32 = req::STONE;
    const IRON: u32 = 30;
    const COAL: u32 = 31;
    const IRON_ORE: u32 = 32;
    const SWORD: u32 = 33;
    const SHIELD: u32 = 34;
    const FISH: u32 = 40;
    const BREAD: u32 = 41;
    const HAM: u32 = 42;
    const PIG: u32 = 43;
    const WATER: u32 = 44;
    const TOOLS: u32 = 50;
    /// Tool goods 60..72 in slot order: gripper, axe, saw, pickaxe, hammer,
    /// shovel, pot, fishing rod, scythe, hatchet, rolling pin, spear.
    const TOOL: [u32; 12] = [60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71];
    const SMITHY: u32 = 300;
    const METAL: u32 = 301;
    const COAL_MINE: u32 = 302;
    const IRON_MINE: u32 = 303;
    const STONE_MINE: u32 = 304;
    const WELL: u32 = 305;
    const HUNTER: u32 = 306;
    const CATAPULT: u32 = 307;
    const SAWMILL: u32 = 308;
    const JOB: u32 = 20;

    const GRIPPER: usize = 0;
    const AXE: usize = 1;
    const SAW: usize = 2;
    const PICKAXE: usize = 3;
    const HAMMER: usize = 4;
    const SHOVEL: usize = 5;
    const POT: usize = 6;
    const ROD: usize = 7;
    const SCYTHE: usize = 8;
    const HATCHET: usize = 9;
    const PIN: usize = 10;
    const SPEAR: usize = 11;

    fn rules() -> Rules {
        let mut rules = req::rules(&req::full_stock());
        let kind = |id, product, ticks, cycle, stock| {
            let mut t = BuildingType::new(id, BuildingSize::Small);
            t.worker = Some(JOB);
            t.product = product;
            t.production_ticks = ticks;
            t.cycle = cycle;
            t.stock_limit = stock;
            t
        };
        let mut smithy = kind(SMITHY, Some(SWORD), 333, WorkCycle::Workshop, Some(6));
        smithy.second_product = Some(SHIELD);
        let mut catapult = kind(CATAPULT, None, 0, WorkCycle::None, Some(4));
        catapult.catapult = true;
        rules.building_types.extend([
            smithy,
            kind(METAL, Some(TOOLS), 333, WorkCycle::Workshop, Some(6)),
            kind(COAL_MINE, Some(COAL), 333, WorkCycle::Mine, Some(6)),
            kind(IRON_MINE, Some(IRON_ORE), 333, WorkCycle::Mine, Some(6)),
            kind(STONE_MINE, Some(STONE), 333, WorkCycle::Mine, Some(6)),
            kind(WELL, Some(WATER), 333, WorkCycle::Workshop, Some(6)),
            kind(HUNTER, Some(HAM), 55, WorkCycle::Hunter, None),
            catapult,
            kind(SAWMILL, Some(PLANK), 333, WorkCycle::Workshop, Some(6)),
        ]);
        let costs = |list: &[(u32, u32)]| GoodRules {
            costs: list.to_vec(),
            ..GoodRules::default()
        };
        rules.goods.insert(
            JOB,
            GoodRules {
                rest_ticks: 56,
                ..GoodRules::default()
            },
        );
        rules.goods.insert(PLANK, costs(&[(WOOD, 1)]));
        rules.goods.insert(SWORD, costs(&[(IRON, 1), (COAL, 1)]));
        rules.goods.insert(SHIELD, costs(&[(IRON, 1), (COAL, 1)]));
        rules.goods.insert(TOOLS, costs(&[(PLANK, 1), (IRON, 1)]));
        rules.goods.insert(COAL, costs(&[(TOOLS + 100, 1)]));
        rules.goods.insert(HAM, costs(&[(PIG, 1)]));
        // ECO-FOOD-2.3: the per-field resources.
        for good in [COAL, IRON_ORE, STONE, FISH] {
            rules.goods.entry(good).or_default().resource = true;
        }
        rules.production = ProductionRules {
            mine_food: vec![FISH, BREAD, HAM],
            tool_slots: TOOL.to_vec(),
            tools_group: Some(TOOLS),
        };
        rules
    }

    fn world() -> World {
        World::with_map(5, flat(30, 30), rules(), P0)
    }

    fn finished(world: &mut World, kind: u32, at: Point) {
        assert_eq!(building(world, kind, at), Ok(()));
        world.sites.remove(&at);
        world.complete(at);
    }

    /// A mine on `at` with its worker, working the ore field `field` (found
    /// by the search of ECO-FOOD-2, which the ECO-PROD tests do not look at).
    fn mine_on(w: &mut World, kind: u32, at: Point, field: Point) -> usize {
        let id = working(w, kind, at);
        w.ore_fields.insert(at, field);
        id
    }

    /// A building of `kind` on `at` with its worker inside, which has
    /// begun its cycle (the orders of ECO-PROD-6.1 are filed).
    fn working(world: &mut World, kind: u32, at: Point) -> usize {
        finished(world, kind, at);
        let target = world.target_of(at).unwrap();
        let index = world.terrain.index(at);
        let b = world.buildings[index].as_mut().unwrap();
        b.workers = 1;
        b.active = true;
        let id = world.add_settler(Settler {
            number: 0,
            player: P0,
            profession: JOB,
            list: crate::SettlerList::Workers,
            walker: Walker::new(at),
            job: SettlerJob::Worker {
                target,
                work: Work::Begin,
            },
            timer: 0,
            carrying: None,
        });
        world.file_input_orders(at);
        id
    }

    fn work_of(world: &World, id: usize) -> Work {
        match world.settlers[id].as_ref().unwrap().job {
            SettlerJob::Worker { work, .. } => work,
            ref other => panic!("not working: {other:?}"),
        }
    }

    fn carrying(world: &World, id: usize) -> Option<u32> {
        world.settlers[id].as_ref().unwrap().carrying
    }

    fn stock(world: &World, at: Point, good: u32) -> u32 {
        count(world.inputs(at), good)
    }

    fn open(world: &World, at: Point, good: u32) -> u32 {
        world.open_orders(at, good)
    }

    /// Step until the worker starts taking a product (ECO-PROD-5.1) and
    /// give the good it carries. Wares at flags are removed so that the
    /// flag never fills.
    fn next_product(world: &mut World, id: usize, within: u32) -> Option<u32> {
        let mut was_taking = matches!(work_of(world, id), Work::Take { .. });
        for _ in 0..within {
            world.step();
            let taking = matches!(work_of(world, id), Work::Take { .. });
            if taking && !was_taking {
                return carrying(world, id);
            }
            was_taking = taking;
            let lying: Vec<usize> = world.wares().map(|(i, _)| i).collect();
            for ware in lying {
                world.remove_ware(ware);
            }
        }
        None
    }

    /// The tool slot a good stands for.
    fn slot_of(good: Option<u32>) -> usize {
        TOOL.iter().position(|&t| Some(t) == good).expect("a tool")
    }

    /// Take `n` products from the building on `at` as ECO-PROD-5 does.
    fn take(world: &mut World, at: Point, n: usize) -> Vec<Option<u32>> {
        (0..n).map(|_| world.take_product(at)).collect()
    }

    // ECO-PROD T1
    // spec: ECO-PROD-5.2 (T1)
    #[test]
    fn smithy_alternates_sword_and_shield() {
        let mut w = world();
        let at = (10, 10);
        finished(&mut w, SMITHY, at);
        let got = take(&mut w, at, 5);
        assert_eq!(
            got,
            [SWORD, SHIELD, SWORD, SHIELD, SWORD].map(Some).to_vec()
        );
    }

    // ECO-PROD T2
    // spec: ECO-PROD-1.3, ECO-PROD-5.1 (T2)
    #[test]
    fn switch_gives_the_second_product_and_resumes_by_parity() {
        let mut w = world();
        let at = (10, 10);
        finished(&mut w, SMITHY, at);
        assert_eq!(take(&mut w, at, 3).len(), 3);
        assert_eq!(w.production_at(at).unwrap().count, 3);
        assert_eq!(w.toggle_switch(P0, at), Ok(()));
        assert_eq!(take(&mut w, at, 2), [Some(SHIELD), Some(SHIELD)]);
        assert_eq!(w.toggle_switch(P0, at), Ok(()));
        // count is 5, 6: shield, sword.
        assert_eq!(take(&mut w, at, 2), [Some(SHIELD), Some(SWORD)]);
    }

    fn metalworks() -> (World, Point) {
        let mut w = world();
        let at = (10, 10);
        finished(&mut w, METAL, at);
        (w, at)
    }

    // ECO-PROD T3
    // spec: ECO-PROD-5.4, ECO-PROD-7.2 (T3)
    #[test]
    fn first_twelve_tools_with_default_weights() {
        let (mut w, at) = metalworks();
        let got: Vec<usize> = take(&mut w, at, 12).into_iter().map(slot_of).collect();
        assert_eq!(
            got,
            [
                HATCHET, SCYTHE, ROD, HATCHET, POT, SCYTHE, SHOVEL, HATCHET, ROD, AXE, HAMMER,
                SCYTHE
            ]
        );
    }

    // ECO-PROD T4
    // spec: ECO-PROD-7.5 (T4)
    #[test]
    fn long_run_shares_follow_the_weights() {
        let (mut w, at) = metalworks();
        let mut tally = [0u32; 12];
        for good in take(&mut w, at, 510) {
            tally[slot_of(good)] += 1;
        }
        let mut expected = [0u32; 12];
        for (slot, n) in [
            (HATCHET, 80),
            (SCYTHE, 79),
            (ROD, 67),
            (POT, 57),
            (SHOVEL, 49),
            (AXE, 39),
            (HAMMER, 39),
            (SPEAR, 30),
            (SAW, 20),
            (PICKAXE, 20),
            (PIN, 20),
            (GRIPPER, 10),
        ] {
            expected[slot] = n;
        }
        assert_eq!(tally, expected);
    }

    // ECO-PROD T5
    // spec: ECO-PROD-7.2, ECO-PROD-7.3 (T5)
    #[test]
    fn exclusive_slot_does_not_move_the_accumulators() {
        let (mut w, at) = metalworks();
        take(&mut w, at, 2);
        w.submit(Command::ToolExclusive {
            player: P0,
            slot: 1,
        });
        w.step();
        assert_eq!(w.last_results()[0].outcome, Ok(()));
        let got: Vec<usize> = take(&mut w, at, 3).into_iter().map(slot_of).collect();
        assert_eq!(got, [AXE, AXE, AXE]);
        w.submit(Command::ToolClearExclusive { player: P0 });
        w.step();
        assert_eq!(slot_of(w.take_product(at)), ROD);
    }

    // ECO-PROD T6, ECO-PROD-7.3
    #[test]
    fn minimum_weight_and_reset_keeps_accumulators() {
        let (mut w, at) = metalworks();
        w.submit(Command::ToolWeight {
            player: P0,
            slot: 9,
            per_mille: 0,
        });
        w.step();
        assert_eq!(w.tool_priorities(P0).weight(HATCHET), MIN_TOOL_WEIGHT);
        let got: Vec<usize> = take(&mut w, at, 3).into_iter().map(slot_of).collect();
        assert_eq!(got, [SCYTHE, ROD, POT]);
        // Reset: default weights, no exclusive slot, accumulators kept.
        let before = w.tool_priorities(P0);
        w.submit(Command::ToolExclusive {
            player: P0,
            slot: 2,
        });
        w.submit(Command::ToolReset { player: P0 });
        w.step();
        let after = w.tool_priorities(P0);
        assert_eq!(after.exclusive, None);
        assert_eq!(after.weight(HATCHET), 0.9);
        for slot in 0..TOOL_SLOTS {
            assert_eq!(after.accumulator(slot), before.accumulator(slot));
        }
        w.submit(Command::ToolWeight {
            player: P0,
            slot: 12,
            per_mille: 500,
        });
        w.step();
        assert_eq!(w.last_results()[0].outcome, Err(Rejection::NoSuchToolSlot));
    }

    // ECO-PROD T7: the cost list of the group `tools`, consumed and ordered
    // again.
    // spec: ECO-PROD-2.1, ECO-PROD-4.1, ECO-PROD-4.2 (T7)
    #[test]
    fn metalworks_consumes_plank_and_iron() {
        let mut w = world();
        let at2 = (14, 10);
        let id = working(&mut w, METAL, at2);
        w.inputs.insert(at2, vec![(PLANK, 1), (IRON, 1)]);
        let (plank, iron) = (open(&w, at2, PLANK), open(&w, at2, IRON));
        assert_eq!((plank, iron), (6, 6), "ordered up to the stock limit");
        let tool = next_product(&mut w, id, 400);
        assert!(TOOL.contains(&tool.unwrap()));
        assert_eq!(stock(&w, at2, PLANK), 0);
        assert_eq!(stock(&w, at2, IRON), 0);
        assert_eq!(open(&w, at2, PLANK), plank + 1);
        assert_eq!(open(&w, at2, IRON), iron + 1);
    }

    fn ore(w: &mut World, field: Point, good: u32, amount: u8) {
        w.set_resource(field, Some(FieldResource { good, amount }));
    }

    /// Hook W15 (`docs/extending.md`, "Deepening a mine"): a coal mine whose
    /// type has a deepening (250 per mille, at most 2, 4 planks and 2 stone,
    /// food 2 then 3) is refused a deepening while ore is left, accepted
    /// once exhausted; its site orders the 4 planks and 2 stone, a builder
    /// works them as an upgrade's, and at its end the field dug from 7 holds
    /// 1 coal (7 x 0.25 rounded down), the mine eats 2 food per product
    /// (it begins only with 2 in stock) and orders 2 more of each food; a
    /// third deepening is refused; the snapshot keeps it all. The original's
    /// game keeps no first resources and has no trace of the hook.
    // spec: none (hook W15; the search is ECO-FOOD-2's, the food ECO-PROD-2.2's, the work MIL-UPG-2.3's)
    #[test]
    fn a_deepened_mine_digs_again_at_a_higher_food_cost() {
        let mut plain = world();
        ore(&mut plain, (10, 11), COAL, 7);
        plain.take_unit((10, 11));
        let debug = format!("{plain:?}");
        assert!(!debug.contains("first_resources") && !debug.contains("deepenings"));
        let mut rules = rules();
        rules.building_types.iter_mut().for_each(|t| {
            if t.id == COAL_MINE {
                t.working_radius = 2;
            }
        });
        rules.deepenings.insert(
            COAL_MINE,
            crate::Deepening {
                share_per_mille: 250,
                most: 2,
                costs: vec![(PLANK, 4), (STONE, 2)],
                food: vec![2, 3],
            },
        );
        let mut w = World::with_map(5, flat(30, 30), rules, P0);
        let (at, field) = ((10, 10), (10, 11));
        ore(&mut w, field, COAL, 7);
        let id = mine_on(&mut w, COAL_MINE, at, field);
        let deepen = Command::DeepenMine { player: P0, at };
        assert!(!w.can_deepen(P0, at));
        assert_eq!(apply(&mut w, deepen.clone()), Err(Rejection::NotDeepenable));
        // Dug out: 7 units, the last clears the field.
        for _ in 0..7 {
            w.take_unit(field);
        }
        w.ore_fields.remove(&at);
        assert_eq!(w.resource_at(field), None);
        assert!(w.can_deepen(P0, at));
        assert!(!w.can_deepen(PlayerId(1), at));
        assert_eq!(apply(&mut w, deepen.clone()), Ok(()));
        assert_eq!(
            w.mine_deepening(at),
            Some(crate::MineDeepening {
                done: 0,
                under_way: true
            })
        );
        assert!(!w.can_deepen(P0, at));
        // Its site orders the material as any site's.
        for _ in 0..3 {
            w.step();
        }
        assert_eq!(w.open_orders(at, PLANK), 4);
        assert_eq!(w.open_orders(at, STONE), 2);
        // Delivered (the debug supply stands in for the carriers) and worked.
        let supply = Command::DebugSupplySite {
            player: P0,
            at,
            wares: vec![(PLANK, 4), (STONE, 2)],
            workers: true,
        };
        assert_eq!(apply(&mut w, supply), Ok(()));
        for _ in 0..5000 {
            if w.mine_deepening(at).is_some_and(|d| d.done == 1) {
                break;
            }
            w.step();
        }
        assert_eq!(
            w.mine_deepening(at),
            Some(crate::MineDeepening {
                done: 1,
                under_way: false
            })
        );
        assert!(w.site(at).is_none());
        assert_eq!(w.building_at(at).unwrap().building_type, COAL_MINE);
        assert_eq!(
            w.resource_at(field),
            Some(FieldResource {
                good: COAL,
                amount: 1
            })
        );
        // Two food per coal now: ordered 2 + 1 of each, begun only with 2.
        assert_eq!(w.mine_food_per_product(at), 2);
        assert_eq!(w.input_limit(at, FISH), Some(3));
        w.inputs.insert(at, vec![(FISH, 1)]);
        assert!(!w.can_produce(at));
        w.inputs.insert(at, vec![(FISH, 1), (BREAD, 1)]);
        assert!(w.can_produce(at));
        w.consume_inputs(at);
        assert_eq!(stock(&w, at, FISH) + stock(&w, at, BREAD), 0);
        // The search finds the coal again and the miner digs it.
        w.inputs.insert(at, vec![(FISH, 4)]);
        assert_eq!(next_product(&mut w, id, 1500), Some(COAL));
        // The snapshot keeps the deepening and the first amounts.
        let copy = World::from_snapshot(&w.snapshot()).unwrap();
        assert!(copy == w);
        // A second deepening once exhausted, then no third.
        w.ore_fields.remove(&at);
        w.set_resource(field, None);
        w.mine_deepenings.get_mut(&at).unwrap().done = 2;
        assert!(!w.can_deepen(P0, at));
        assert_eq!(apply(&mut w, deepen), Err(Rejection::NotDeepenable));
        assert_eq!(w.mine_food_per_product(at), 3);
    }

    // ECO-PROD T8: fish is eaten before ham; the one eaten is re-ordered.
    // spec: ECO-PROD-2.2, ECO-PROD-4.2 (T8)
    #[test]
    fn mine_eats_fish_first_and_orders_it_again() {
        let mut w = world();
        let at = (10, 10);
        ore(&mut w, (10, 11), COAL, 7);
        let id = mine_on(&mut w, COAL_MINE, at, (10, 11));
        w.inputs.insert(at, vec![(FISH, 1), (HAM, 2)]);
        let before = (open(&w, at, FISH), open(&w, at, BREAD), open(&w, at, HAM));
        assert_eq!(next_product(&mut w, id, 400), Some(COAL));
        assert_eq!(stock(&w, at, FISH), 0);
        assert_eq!(stock(&w, at, HAM), 2);
        assert_eq!(open(&w, at, FISH), before.0 + 1);
        assert_eq!(open(&w, at, BREAD), before.1);
        assert_eq!(open(&w, at, HAM), before.2);
    }

    // ECO-REQ T30 (ECO-REQ-4.2, ECO-DIST-3.1a): a mine files its food orders
    // fish, fish, meat, meat, bread, bread (not its eating order fish,
    // bread, meat), and each good's group takes its place in the player's
    // order list at its first order and keeps it after the orders are gone.
    // spec: ECO-REQ-4.2 (ECO-REQ T30), ECO-DIST-3.1a
    #[test]
    fn t30_a_mine_files_fish_meat_bread_and_the_groups_keep_their_places() {
        let mut w = world();
        let at = (10, 10);
        ore(&mut w, (10, 11), COAL, 7);
        let _ = mine_on(&mut w, COAL_MINE, at, (10, 11));
        w.orders.clear();
        w.order_groups.clear();
        w.file_input_orders(at);
        let filed: Vec<u32> = w.orders.iter().map(|o| o.good).collect();
        assert_eq!(filed, [FISH, FISH, HAM, HAM, BREAD, BREAD]);
        let player = w.building_at(at).unwrap().owner;
        assert_eq!(w.order_groups[&player], [FISH, HAM, BREAD]);
        w.orders.clear();
        w.order(at, BREAD);
        w.order(at, FISH);
        assert_eq!(w.order_groups[&player], [FISH, HAM, BREAD], "places kept");
    }

    // ECO-PROD T9: the ore unit goes at the begin, before the food arrives.
    // spec: ECO-PROD-3.3, ECO-PROD-2.2 (T9)
    #[test]
    fn mine_takes_ore_at_once_and_waits_for_food() {
        let mut w = world();
        let at = (10, 10);
        ore(&mut w, (10, 11), COAL, 7);
        let id = mine_on(&mut w, COAL_MINE, at, (10, 11));
        w.step();
        assert_eq!(w.resource_at((10, 11)).unwrap().amount, 6);
        assert_eq!(work_of(&w, id), Work::WaitInputs);
        for _ in 0..50 {
            w.step();
            assert_eq!(work_of(&w, id), Work::WaitInputs);
        }
        w.receive_input(at, BREAD);
        w.step();
        assert_eq!(work_of(&w, id), Work::Working);
        assert_eq!(w.resource_at((10, 11)).unwrap().amount, 6);
    }

    // ECO-PROD T10
    // spec: ECO-PROD-3.3, ECO-PROD-8.4 (T10)
    #[test]
    fn last_unit_clears_the_field_and_the_next_cycle_waits_for_ore() {
        let mut w = world();
        let at = (10, 10);
        ore(&mut w, (10, 11), IRON_ORE, 1);
        let id = mine_on(&mut w, IRON_MINE, at, (10, 11));
        w.inputs.insert(at, vec![(FISH, 5)]);
        w.step();
        assert_eq!(w.resource_at((10, 11)), None);
        assert_eq!(w.ore_field(at), None);
        assert_eq!(next_product(&mut w, id, 400), Some(IRON_ORE));
        // The cycle goes on to its rest and then waits at Begin.
        for _ in 0..300 {
            w.step();
        }
        assert_eq!(work_of(&w, id), Work::Begin);
        // ECO-FOOD-2.7: the next search is at most 101 ticks away.
        ore(&mut w, (11, 11), IRON_ORE, 3);
        let mut found = None;
        for _ in 0..101 {
            w.step();
            if w.ore_field(at).is_some() {
                found = Some(w.tick());
                break;
            }
        }
        assert!(found.is_some());
        w.step();
        assert_eq!(work_of(&w, id), Work::Working);
        assert_eq!(w.resource_at((11, 11)).unwrap().amount, 2);
    }

    // ECO-PROD-3.3: the amount is capped at 7.
    #[test]
    fn ore_amount_is_capped_at_seven() {
        let mut w = world();
        let at = (10, 10);
        ore(&mut w, (10, 11), COAL, 200);
        let _ = mine_on(&mut w, COAL_MINE, at, (10, 11));
        w.step();
        assert_eq!(w.resource_at((10, 11)).unwrap().amount, 7);
    }

    // ECO-PROD T11
    // spec: ECO-PROD-2.4 (T11)
    #[test]
    fn stone_mine_eats_food() {
        let mut w = world();
        let at = (10, 10);
        ore(&mut w, (10, 11), STONE, 7);
        let id = mine_on(&mut w, STONE_MINE, at, (10, 11));
        w.inputs.insert(at, vec![(HAM, 1)]);
        assert_eq!(next_product(&mut w, id, 400), Some(STONE));
        assert_eq!(stock(&w, at, HAM), 0);
    }

    // ECO-PROD T12
    // spec: ECO-PROD-2.5 (T12)
    #[test]
    fn well_works_continuously_and_orders_nothing() {
        let mut w = world();
        let at = (10, 10);
        let id = working(&mut w, WELL, at);
        assert!(w.orders().is_empty());
        let mut starts: Vec<Tick> = Vec::new();
        let mut was = false;
        for _ in 0..1000 {
            w.step();
            let now = work_of(&w, id) == Work::Working;
            if now && !was {
                starts.push(w.tick());
            }
            was = now;
            let lying: Vec<usize> = w.wares().map(|(i, _)| i).collect();
            for ware in lying {
                w.remove_ware(ware);
            }
        }
        assert!(starts.len() >= 2);
        // The gap of ECO-WORK-4.2 with the two standing ticks of 3.4.
        assert_eq!(starts[1] - starts[0], 333 + 123);
        assert!(w.orders().is_empty());
    }

    // ECO-PROD T20, ECO-PROD-2.5a (Q-052): a well is placed by the rules for
    // small buildings alone and works at its normal rate whatever the
    // resource layer holds under it.
    #[test]
    fn well_checks_nothing() {
        let mut w = world();
        let at = (10, 10);
        // A layer with other goods and no water anywhere.
        ore(&mut w, (10, 11), COAL, 7);
        ore(&mut w, at, IRON_ORE, 3);
        assert_eq!(building(&mut w, WELL, at), Ok(()));
        let id = {
            w.sites.remove(&at);
            w.complete(at);
            let target = w.target_of(at).unwrap();
            let index = w.terrain.index(at);
            let b = w.buildings[index].as_mut().unwrap();
            b.workers = 1;
            b.active = true;
            w.add_settler(Settler {
                number: 0,
                player: P0,
                profession: JOB,
                list: crate::SettlerList::Workers,
                walker: Walker::new(at),
                job: SettlerJob::Worker {
                    target,
                    work: Work::Begin,
                },
                timer: 0,
                carrying: None,
            })
        };
        let mut starts: Vec<Tick> = Vec::new();
        let mut was = false;
        for _ in 0..1000 {
            w.step();
            let now = work_of(&w, id) == Work::Working;
            if now && !was {
                starts.push(w.tick());
            }
            was = now;
            let lying: Vec<usize> = w.wares().map(|(i, _)| i).collect();
            for ware in lying {
                w.remove_ware(ware);
            }
        }
        assert!(starts.len() >= 2);
        // The gap of ECO-WORK-4.2 with the two standing ticks of 3.4.
        assert_eq!(starts[1] - starts[0], 333 + 123);
        assert!(w.orders().is_empty());
    }

    // ECO-PROD T13
    // spec: ECO-PROD-2.3 (T13)
    #[test]
    fn hunter_needs_no_pig() {
        let mut w = world();
        let at = (10, 10);
        w.add_animal(1, (13, 10));
        let id = working(&mut w, HUNTER, at);
        assert!(w.orders().is_empty());
        // ECO-FOOD-5.4: the ham is taken after the kill.
        let mut taken = None;
        for _ in 0..800 {
            w.step();
            if matches!(work_of(&w, id), Work::TakeWare { .. }) {
                taken = carrying(&w, id);
                break;
            }
        }
        assert_eq!(taken, Some(HAM));
        assert!(w.orders().is_empty());
        assert_eq!(stock(&w, at, PIG), 0);
    }

    // ECO-PROD T14, T15
    // spec: ECO-PROD-6.3, ECO-PROD-6.4, ECO-PROD-6.1 (T14, T15)
    #[test]
    fn switching_off_cancels_unassigned_orders_and_on_orders_up_to_the_limit() {
        let mut w = world();
        let at = (10, 10);
        finished(&mut w, SAWMILL, at);
        w.inputs.insert(at, vec![(WOOD, 3)]);
        let id = {
            let target = w.target_of(at).unwrap();
            let index = w.terrain.index(at);
            let b = w.buildings[index].as_mut().unwrap();
            b.workers = 1;
            b.active = true;
            let id = w.add_settler(Settler {
                number: 0,
                player: P0,
                profession: JOB,
                list: crate::SettlerList::Workers,
                walker: Walker::new(at),
                job: SettlerJob::Worker {
                    target,
                    work: Work::Begin,
                },
                timer: 0,
                carrying: None,
            });
            w.file_input_orders(at);
            id
        };
        assert_eq!(open(&w, at, WOOD), 3);
        // One of the three has a ware on its way: no longer an open order.
        let pos = w.orders.iter().rposition(|o| o.target.at == at).unwrap();
        w.orders.remove(pos);
        w.step();
        assert_eq!(work_of(&w, id), Work::Working);
        w.submit(Command::ProductionToggle { player: P0, at });
        w.step();
        assert_eq!(open(&w, at, WOOD), 0, "the 2 open orders are removed");
        // The work action ends, nothing is re-ordered, the plank comes out.
        assert_eq!(next_product(&mut w, id, 400), Some(PLANK));
        assert_eq!(stock(&w, at, WOOD), 2);
        assert_eq!(open(&w, at, WOOD), 0);
        // The assigned log is delivered and stored.
        w.receive_input(at, WOOD);
        assert_eq!(stock(&w, at, WOOD), 3);
        // No next work action while off (ECO-WORK-3.1b: a state of his own).
        for _ in 0..300 {
            w.step();
        }
        assert_eq!(work_of(&w, id), Work::Off);
        // T15
        w.submit(Command::ProductionToggle { player: P0, at });
        w.step();
        assert_eq!(open(&w, at, WOOD), 3);
        // ECO-WORK-3.1b: back to waiting, then the first check.
        assert_eq!(work_of(&w, id), Work::Begin);
        w.step();
        assert_eq!(work_of(&w, id), Work::Working);
    }

    // ECO-PROD T16, T17
    // spec: ECO-PROD-6.1, ECO-PROD-2.2 (T16, T17), UI-GAME-4.1a
    #[test]
    fn mine_orders_two_of_each_food_and_catapult_stones() {
        let mut w = world();
        let _ = working(&mut w, COAL_MINE, (10, 10));
        for food in [FISH, HAM, BREAD] {
            assert_eq!(open(&w, (10, 10), food), 2);
        }
        assert_eq!(w.orders().len(), 6);
        let _ = working(&mut w, CATAPULT, (16, 10));
        assert_eq!(open(&w, (16, 10), STONE), 4);
        // UI-GAME-4.1a (f): the window's capacity is the same limit.
        for food in [FISH, HAM, BREAD] {
            assert_eq!(w.input_limit((10, 10), food), Some(2));
        }
        assert_eq!(w.input_limit((16, 10), STONE), Some(4));
        assert_eq!(w.input_limit((10, 10), STONE), None);
        assert_eq!(w.input_limit((30, 30), FISH), None, "no building");
    }

    // ECO-PROD T21 (1.2, 6.4, 6.1): switching production on files orders up
    // to the limit counting the orders whose ware is already on its way (a
    // ware sent before production was switched off is still delivered).
    // spec: ECO-PROD-6.4, ECO-PROD-6.1, ECO-PROD-1.2 (ECO-PROD T21)
    #[test]
    fn t21_switching_on_counts_the_wares_on_their_way() {
        let mut w = world();
        let at = (10, 10);
        let _ = working(&mut w, SAWMILL, at);
        let limit = open(&w, at, WOOD);
        assert!(limit >= 2);
        // One order's ware is on its way: sent, lying at the sawmill's flag.
        let i = w.orders.iter().position(|o| o.target.at == at).unwrap();
        let order = w.orders.remove(i);
        let flag = w.flag_of(at);
        w.new_ware_at(WOOD, flag, Some(flag), Some(order.target));
        assert_eq!(w.toggle_production(P0, at), Ok(()));
        assert_eq!(open(&w, at, WOOD), 0, "off cancels the open orders");
        assert_eq!(w.toggle_production(P0, at), Ok(()));
        assert_eq!(open(&w, at, WOOD), limit - 1, "the ware on its way counts");
    }

    // ECO-PROD-1.1, 1.4: a new building is on, and a command for a field
    // without a building changes nothing.
    #[test]
    fn production_state_starts_on() {
        let mut w = world();
        finished(&mut w, SAWMILL, (10, 10));
        assert_eq!(w.production_at((10, 10)), Some(Production::new()));
        assert_eq!(w.toggle_production(P0, (3, 3)), Err(Rejection::NoBuilding));
        assert_eq!(
            w.toggle_production(PlayerId(1), (10, 10)),
            Err(Rejection::NoBuilding)
        );
    }
}
