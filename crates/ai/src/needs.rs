//! Needs: creation, arrangement and construction (AI-CON-2 to AI-CON-4).

use s2_sim::grid;
use s2_sim::map::state;

use crate::cells::{centre, origin, CellPos, CELL};
use crate::config::AiConfig;
use crate::construction::{Order, Outcome, Slot, Tag};
use crate::expansion::{Kind, Source};
use crate::maps::{mcell_of, threat_balance};
use crate::plan::NeedId;
use crate::player::{AiPlayer, Ctx};
use crate::resources::{square_of, square_origin, Square, SQUARE, SURFACE, UNDERGROUND};
use crate::sites::Area;

/// AI-CON-3.3: a cell takes this many productions.
const CELL_CAPACITY: usize = 20;
/// AI-CON-3.3: the cost of a cell without a supplier or consumer.
const NO_DISTANCE: u32 = 100_000;

/// AI-CON-3.3 (corrected 2026-10-27 b): the supplier part S (the cell's
/// producers and the supplier distances) and the consumer distance C make
/// S + C when both are below 100 000, S when only S is, else C.
pub(crate) fn combined(supplier: u32, consumer: u32) -> u32 {
    match (supplier < NO_DISTANCE, consumer < NO_DISTANCE) {
        (true, true) => supplier + consumer,
        (true, false) => supplier,
        _ => consumer,
    }
}
/// AI-CON-3.3: the cost of one producer already in the cell.
const PRODUCER_COST: u32 = 50;
/// AI-CON-3.2: an input with this stock or less and no producer gets a need.
const INPUT_STOCK_LIMIT: u32 = 10;
/// AI-CON-5.1: weight of the orders of the needs construction.
const NEEDS_WEIGHT: u32 = 10;

/// AI-CON-4.2: score(g) = (D - A) x aiPriority x max(0.001, (100 - max(1,
/// stock + stock2)) / 100) / max(0.2, A) x max(0.1, avgProd).
pub fn needs_score(demand: f32, supply: f32, priority: f32, stock: u32, productivity: f32) -> f32 {
    let stock_factor = ((100.0 - stock.max(1) as f32) / 100.0).max(0.001);
    (demand - supply) * priority * stock_factor / supply.max(0.2) * productivity.max(0.1)
}

impl AiPlayer {
    /// AI-CON-1.4/3.4: the goods whose producer sits on a resource square.
    fn is_square_good(&self, ctx: &Ctx, good: u32) -> bool {
        let n = &ctx.config.names;
        [n.wood, n.stone, n.fish, n.coal, n.iron_ore, n.gold].contains(&good)
    }

    /// The statistics value S of the specs (AI-CON-1.8): the fields the
    /// player owned when the statistics last counted.
    pub(crate) fn statistic(&mut self, _ctx: &Ctx) -> u32 {
        self.statistic.1
    }

    /// AI-2.3 need creation (every 47 ticks), AI-CON-2.
    pub(crate) fn need_creation(&mut self, ctx: &mut Ctx) {
        let t = ctx.world.game_seconds();
        let n = ctx.config.names.clone();
        let consumer = self.home;
        match self.need_stage {
            0 => {
                // AI-CON-2.1, SIM-START-4.4 g: a record per need.
                self.plan.add_need(n.stone, Some(consumer), t);
                for _ in 0..self.profile.start_sawmills.max(1) {
                    self.plan.add_need(n.plank, Some(consumer), t);
                }
                self.need_stage = 1;
            }
            1 if t >= 300.0 => {
                // AI-CON-2.2
                let has = |g: u32| self.plan.placed_producers(g) > 0;
                if has(n.stone) && has(n.wood) && has(n.plank) {
                    if self.profile.max_smithies > 0 {
                        self.plan.add_need(n.sword, Some(consumer), t);
                        self.plan.add_need(n.beer, Some(consumer), t);
                    }
                    self.need_stage = 2;
                }
            }
            2 if t >= 800.0 => {
                self.plan.add_need(n.donkey, Some(consumer), t);
                self.need_stage = 3;
            }
            3 if t >= 1200.0 => {
                if self.profile.max_mints > 0 {
                    self.plan.add_need(n.coin, Some(consumer), t);
                }
                self.need_stage = 4;
            }
            _ => {}
        }
        // AI-CON-2.4 stock checks: the counter k doubles each time it fires.
        if t > 600.0 * self.stock_check as f32 {
            self.stock_check *= 2;
            let s = self.statistic(ctx);
            // T86: divided by the needs of the good standing in cells (the
            // count of AI-CON-3.3), not by its producers.
            let in_cells = |good: u32| {
                self.plan
                    .needs
                    .iter()
                    .filter(|n| {
                        n.cell.is_some()
                            && (n.good == good || ctx.config.group_of(n.good) == Some(good))
                    })
                    .count()
                    .max(1) as u32
            };
            let (planks, stones) = (in_cells(n.plank), in_cells(n.stone));
            // SIM-START-4.4 g: each a record of its own (measured: plank need
            // 91252 of E-trace-011 beside the castle's start need).
            if self.econ.stock(n.plank) < s / 32 / planks {
                self.plan.add_need(n.plank, Some(consumer), t);
            }
            if self.econ.stock(n.stone) < s / 16 / stones {
                self.plan.add_need(n.stone, Some(consumer), t);
            }
        }
    }

    // ---- AI-CON-3 needs arrangement ----

    /// AI-2.3 needs arrangement (every 13 ticks): pass 1 gives food needs
    /// their concrete needs, pass 2 gives the first need without a cell its
    /// inputs or its cell. Each pass stops at the first need it changes.
    pub(crate) fn needs_arrangement(&mut self, ctx: &mut Ctx) {
        self.link_food_needs(ctx);
        self.arrange_first(ctx);
    }

    /// AI-CON-3.1 (Q-178): pass 1, over the needs of the food group good
    /// only (a list of their own), in need-id order: a food need without a
    /// linked concrete need gets one, drawn (no consumer); one whose linked
    /// fish need has no cell and waits for expansion gets a bread or flesh
    /// need instead.
    fn link_food_needs(&mut self, ctx: &mut Ctx) -> bool {
        let n = ctx.config.names.clone();
        let t = ctx.world.game_seconds();
        for i in 0..self.plan.needs.len() {
            let need = self.plan.needs[i].clone();
            if need.good != n.food {
                continue;
            }
            match need.food.and_then(|f| self.plan.need(f).cloned()) {
                None => {
                    // AI-2.7 a: d = draw mod 10.
                    let r = ctx.rng.draw() % 10;
                    let good = if r < 7 {
                        n.fish
                    } else if r < 9 {
                        n.bread
                    } else {
                        n.flesh
                    };
                    let food = self.plan.add_need(good, None, t);
                    self.plan.needs[i].food = Some(food);
                    return true;
                }
                Some(food) if food.good == n.fish && food.cell.is_none() && food.waiting => {
                    // Replace a fish need that found no fishing square.
                    self.plan.remove_need(food.id);
                    // AI-2.7 a: draw mod 6 equal to 5 asks for flesh.
                    let good = if ctx.rng.draw() % 6 < 5 {
                        n.bread
                    } else {
                        n.flesh
                    };
                    let replacement = self.plan.add_need(good, None, t);
                    if let Some(slot) = self.plan.needs.iter_mut().find(|x| x.id == need.id) {
                        slot.food = Some(replacement);
                    }
                    return true;
                }
                Some(_) => {}
            }
        }
        false
    }

    /// [`AiPlayer::link_food_needs`] for the tests of other modules.
    #[cfg(test)]
    pub(crate) fn link_food_needs_for_test(&mut self, ctx: &mut Ctx) -> bool {
        self.link_food_needs(ctx)
    }

    /// Whether the cell accepts a need for `good` (AI-CON-3.3).
    pub(crate) fn cell_accepts(&self, ctx: &Ctx, cell: CellPos, good: u32) -> bool {
        let n = &ctx.config.names;
        let unlimited = [n.coal, n.stone, n.iron_ore, n.gold].contains(&good);
        // T71: fewer than 20 needs in the cell, not productions.
        (unlimited || self.plan.needs_in(cell) < CELL_CAPACITY)
            && !self.cells.get(cell).is_some_and(|c| c.is_full(good))
    }

    /// AI-CON-3.3 (T70): distance from `cell` to the nearest cell holding a
    /// production of `good` (or of a member of a group good), halved when
    /// that production is free ([`Plan::is_free`]: no need, or a need
    /// without a consumer building).
    /// AI-CON-3.3 (T59): a cell's point for the arrangement's distances,
    /// its position (AI-CON-1.1a; the centre field while it has none).
    fn cell_point(&self, cell: CellPos) -> grid::Point {
        self.cells
            .get(cell)
            .map_or_else(|| centre(cell), |c| c.route_start())
    }

    pub(crate) fn supplier_distance(&self, ctx: &Ctx, cell: CellPos, good: u32) -> u32 {
        let members = ctx.config.group_members(good);
        self.plan
            .prods
            .iter()
            .filter(|p| members.contains(&p.good))
            .map(|p| {
                let d = grid::distance(self.cell_point(cell), self.cell_point(p.cell));
                if self.plan.is_free(p) {
                    d / 2
                } else {
                    d
                }
            })
            .min()
            .unwrap_or(NO_DISTANCE)
    }

    /// AI-CON-3.3 (T70, T77, T85): distance to the cells that hold a
    /// production of a good h with `good` in its cost list (a member of the
    /// food group is looked up as food; h no settler good, a ware group
    /// counts), halved when that production is free, times the consumer
    /// cell's needs of the looked-up good or of a good in its group, plus 1.
    pub(crate) fn consumer_distance(&self, ctx: &Ctx, cell: CellPos, good: u32) -> u32 {
        let food = ctx.config.names.food;
        let group = ctx.config.group_of(good);
        let key = if group == Some(food) { food } else { good };
        // T85: the consumer cell's needs whose good is the key or whose
        // good's group is (for a fish need: food, fish, bread and meat).
        let counted = |cell: CellPos| {
            self.plan
                .needs
                .iter()
                .filter(|n| {
                    n.cell == Some(cell)
                        && (n.good == key || ctx.config.group_of(n.good) == Some(key))
                })
                .count() as u32
        };
        let mut best = NO_DISTANCE;
        for &h in &ctx.config.goods {
            if ctx.config.settler_goods.contains(&h) {
                continue;
            }
            if !ctx.config.inputs(h).iter().any(|&(i, _)| i == key) {
                continue;
            }
            for p in self.plan.prods.iter().filter(|p| p.good == h) {
                let d = grid::distance(self.cell_point(cell), self.cell_point(p.cell));
                let d = if self.plan.is_free(p) { d / 2 } else { d };
                let own = counted(p.cell) + 1;
                best = best.min(d.saturating_mul(own));
            }
        }
        best
    }

    /// AI-CON-3.2 to 3.4 for the first need that can be changed, in need-id
    /// order; food needs never get a cell (pass 2 does not see their list).
    fn arrange_first(&mut self, ctx: &mut Ctx) -> bool {
        let t = ctx.world.game_seconds();
        let food = ctx.config.names.food;
        for i in 0..self.plan.needs.len() {
            let need = self.plan.needs[i].clone();
            // A waiting need stays on the waiting list until the cell system
            // takes it (AI-CON-3.4, Q-161b).
            if need.good == food || need.cell.is_some() || need.waiting {
                continue;
            }
            // AI-CON-3.2 (Q-178, Q-365): an input is missing when no cell
            // has a production with a placed building of that good or of a
            // good in its group (bread, fish and meat for food) and the
            // input's own stock is at most 10.
            let cfg = ctx.config;
            // W12: a need the alternative takes checks the type's recipe.
            let alternative = self.alternative_for(ctx, &need);
            let inputs = match alternative {
                Some(building) => cfg.type_inputs(building, need.good),
                None => cfg.inputs(need.good),
            };
            let missing: Vec<u32> = inputs
                .iter()
                .map(|&(g, _)| g)
                .filter(|&g| {
                    !self
                        .plan
                        .prods
                        .iter()
                        .any(|p| p.building.is_some() && cfg.supplies(p.good, g))
                        && self.econ.stock(g) <= INPUT_STOCK_LIMIT
                })
                .collect();
            if !missing.is_empty() {
                // The first missing input without a need that has no
                // consumer gets one (AI-CON-2.5, T26: no consumer); with
                // none the need is passed over. It gets no cell.
                let without = missing.into_iter().find(|&g| {
                    !self
                        .plan
                        .needs
                        .iter()
                        .any(|x| x.good == g && x.consumer.is_none())
                });
                if let Some(g) = without {
                    self.plan.add_need(g, None, t);
                    return true;
                }
                continue;
            }
            let changed = if let Some(building) = alternative {
                self.arrange_alternative(ctx, need.id, building)
            } else if self.is_square_good(ctx, need.good) {
                self.arrange_resource(ctx, need.id)
            } else {
                self.arrange_plain(ctx, need.id)
            };
            if changed {
                return true;
            }
        }
        false
    }

    /// AI-CON-3.3 plain goods and corn.
    fn arrange_plain(&mut self, ctx: &mut Ctx, id: NeedId) -> bool {
        let Some(need) = self.plan.need(id).cloned() else {
            return false;
        };
        let inputs: Vec<u32> = ctx
            .config
            .inputs(need.good)
            .iter()
            .map(|&(g, _)| g)
            .collect();
        match self.plain_cell(ctx, need.good, &inputs) {
            Some(cell) => {
                self.assign_cell(ctx.config, id, cell, None);
                true
            }
            None => {
                // Q-366 night rules: only while this source's own slot is
                // empty (AI-CON-3.3, AI-MIL-3.1).
                if !self.expansion.is_pending(Source::Anywhere) {
                    let now = ctx.world.tick();
                    self.expansion
                        .set(Source::Anywhere, Kind::Anywhere, self.home, now);
                }
                false
            }
        }
    }

    /// Hook W12: a square good's need arranged for the project's alternative
    /// type `building`: AI-CON-3.3's cell choice with the type's recipe as
    /// its inputs and no square. With no cell that accepts it, the need goes
    /// on the waiting list as AI-CON-3.4 puts it there.
    fn arrange_alternative(&mut self, ctx: &mut Ctx, id: NeedId, building: u32) -> bool {
        let Some(good) = self.plan.need(id).map(|n| n.good) else {
            return false;
        };
        let inputs: Vec<u32> = ctx
            .config
            .type_inputs(building, good)
            .iter()
            .map(|&(g, _)| g)
            .collect();
        match self.plain_cell(ctx, good, &inputs) {
            Some(cell) => {
                self.assign_cell(ctx.config, id, cell, None);
                true
            }
            None => {
                self.plan.put_waiting(id);
                false
            }
        }
    }

    /// Hook W12: the alternative producer type for `need` if it takes the
    /// need now: its good is a square good with an alternative, no cell
    /// offers a square for it (AI-CON-3.4), the player's stock of the good
    /// is below the alternative's limit, its stock of the input at least the
    /// alternative's, and fewer than its most of the type are placed or
    /// planned. Never for the original's rules (no alternatives).
    fn alternative_for(&self, ctx: &Ctx, need: &crate::plan::Need) -> Option<u32> {
        let alt = ctx.config.alternatives.get(&need.good)?;
        if !self.is_square_good(ctx, need.good)
            || self.econ.total(need.good) >= alt.stock_below
            || self.econ.stock(alt.input) < alt.input_at_least
            || self.alternatives_of(ctx, alt.building, need.good) >= alt.most
        {
            return None;
        }
        let kinds = Self::square_kinds(ctx, need.good);
        let offered = self.cells.in_order().iter().any(|&cell| {
            self.cell_accepts(ctx, cell, need.good)
                && self.best_square(ctx, cell, need.good, kinds).is_some()
        });
        (!offered).then_some(alt.building)
    }

    /// Hook W12: the player's buildings and sites of type `building`, and
    /// its needs of `good` arranged for it (a cell, no square) whose
    /// building is not placed yet.
    fn alternatives_of(&self, ctx: &Ctx, building: u32, good: u32) -> u32 {
        let placed = ctx
            .world
            .buildings()
            .filter(|b| b.owner == self.id && b.building_type == building)
            .count();
        let planned = self
            .plan
            .needs
            .iter()
            .filter(|n| n.good == good && n.cell.is_some() && n.square.is_none())
            .filter(|n| {
                n.production.is_none_or(|p| {
                    self.plan
                        .prods
                        .iter()
                        .find(|x| x.id == p)
                        .is_none_or(|x| x.building.is_none())
                })
            })
            .count();
        (placed + planned) as u32
    }

    /// AI-CON-3.3: the cell with the lowest cost for a need of `good` whose
    /// producer takes `inputs`, the first in cell order on ties.
    fn plain_cell(&self, ctx: &Ctx, good: u32, inputs: &[u32]) -> Option<CellPos> {
        let mut best: Option<(u32, CellPos)> = None;
        for cell in self.cells.in_order().to_vec() {
            if !self.cell_accepts(ctx, cell, good) {
                continue;
            }
            // T71: the needs of the good already in the cell.
            let own = self.plan.needs_of_in(cell, &[good]) as u32;
            let mut supplier = PRODUCER_COST * own;
            for &input in inputs {
                supplier = supplier.saturating_add(self.supplier_distance(ctx, cell, input));
            }
            let cost = combined(supplier, self.consumer_distance(ctx, cell, good));
            if best.is_none_or(|(c, _)| cost < c) {
                best = Some((cost, cell));
            }
        }
        best.map(|(_, cell)| cell)
    }

    /// Give the need its cell and square (it joins the end of the cell's
    /// need list). AI-CON-2.5b (T102), 3.3 (corrected): it is tied to the
    /// first production in the cell's production list (the order they were
    /// added) whose good is the need's good or has it as its group, and that
    /// has no need yet, placed or not (a need placed again after its
    /// producer's tear-down, AI-CON-9.3 T84, is one case).
    pub(crate) fn assign_cell(
        &mut self,
        config: &AiConfig,
        id: NeedId,
        cell: CellPos,
        square: Option<(Square, u8)>,
    ) {
        let Some(good) = self.plan.need(id).map(|n| n.good) else {
            return;
        };
        let free = self
            .plan
            .prods
            .iter()
            .find(|p| p.cell == cell && config.supplies(p.good, good) && p.need.is_none())
            .map(|p| p.id);
        self.plan.give_cell(id, cell);
        if let Some(n) = self.plan.need_mut(id) {
            n.square = square;
            n.production = free;
            n.again = false;
        }
        self.plan.unwait(id);
        if let Some(p) = free {
            if let Some(prod) = self.plan.prods.iter_mut().find(|x| x.id == p) {
                prod.need = Some(id);
            }
        }
    }

    /// The kinds of resource a need for `good` looks for, in order.
    fn square_kinds(ctx: &Ctx, good: u32) -> &'static [u8] {
        let n = &ctx.config.names;
        if good == n.coal || good == n.iron_ore || good == n.gold || good == n.fish {
            &[UNDERGROUND]
        } else if good == n.stone {
            &[SURFACE, UNDERGROUND]
        } else {
            &[SURFACE]
        }
    }

    /// AI-CON-3.4 (T65, T89): the square a cell offers for `good` with the
    /// resource kinds `kinds`, in one scan of its 4 x 4 squares, sx outer,
    /// sy inner, each only with the good in it and its slot free
    /// ([`AiPlayer::square_taken`]). A square's corners are (4 sx, 4 sy) and
    /// the fields 4 further right and down, clamped into the map; one counts
    /// when the player owns it and it is not Border. A surface entry with
    /// one counting corner ends the scan with its square; an underground
    /// entry with at least two only updates the best underground square
    /// (the largest amount), which the cell offers when the scan finds no
    /// surface square.
    pub(crate) fn best_square(
        &self,
        ctx: &Ctx,
        cell: CellPos,
        good: u32,
        kinds: &[u8],
    ) -> Option<(Square, u8)> {
        let world = ctx.world;
        let terrain = world.terrain();
        let (x0, y0) = origin(cell);
        let (right, bottom) = (terrain.width - 1, terrain.height - 1);
        let mut best: Option<(u32, Square)> = None;
        for sx in 0..CELL / SQUARE {
            for sy in 0..CELL / SQUARE {
                let sq = (x0 / SQUARE + sx, y0 / SQUARE + sy);
                if self.square_taken(sq) {
                    continue;
                }
                let (qx, qy) = square_origin(sq);
                let (fx, fy) = ((qx + SQUARE).min(right), (qy + SQUARE).min(bottom));
                let corners = [
                    (qx.min(right), qy.min(bottom)),
                    (fx, qy.min(bottom)),
                    (qx.min(right), fy),
                    (fx, fy),
                ];
                let usable = corners
                    .iter()
                    .filter(|&&f| {
                        world.field_owner(f) == Some(self.id) && !terrain.has(f, state::BORDER)
                    })
                    .count();
                for &kind in kinds {
                    let amount = self.resources.amount(sq, good, kind);
                    if amount == 0 {
                        continue;
                    }
                    if kind != UNDERGROUND {
                        if usable >= 1 {
                            return Some((sq, kind));
                        }
                        continue;
                    }
                    if usable >= 2 && best.is_none_or(|(a, _)| amount > a) {
                        best = Some((amount, sq));
                    }
                }
            }
        }
        best.map(|(_, sq)| (sq, UNDERGROUND))
    }

    /// AI-CON-3.4 resource goods.
    fn arrange_resource(&mut self, ctx: &mut Ctx, id: NeedId) -> bool {
        let Some(need) = self.plan.need(id).cloned() else {
            return false;
        };
        let first_input = ctx.config.inputs(need.good).first().map(|&(g, _)| g);
        // AI-CON-3.4 (T89): each cell offers one square, of either kind
        // (stone), and the cells compete on the cost alone.
        let kinds = Self::square_kinds(ctx, need.good);
        let mut best: Option<(u32, CellPos, Square, u8)> = None;
        for cell in self.cells.in_order().to_vec() {
            if !self.cell_accepts(ctx, cell, need.good) {
                continue;
            }
            let Some((sq, kind)) = self.best_square(ctx, cell, need.good, kinds) else {
                continue;
            };
            let supplier = first_input.map_or(0, |g| self.supplier_distance(ctx, cell, g));
            let cost = combined(supplier, self.consumer_distance(ctx, cell, need.good));
            if best.is_none_or(|(c, ..)| cost < c) {
                best = Some((cost, cell, sq, kind));
            }
        }
        match best {
            Some((_, cell, sq, kind)) => {
                self.assign_cell(ctx.config, id, cell, Some((sq, kind)));
                true
            }
            None => {
                // No square in any cell: put it on the end of the waiting
                // list; that is no change, the pass goes on (2026-10-26
                // night, AI-MIL T17).
                self.plan.put_waiting(id);
                false
            }
        }
    }

    /// AI-CON-3.5, every 23 ticks (the cell system): the first waiting need
    /// is taken and un-waited (3.4, Q-161b), and the best cell coordinate
    /// for its good, among the resource entries on the player's conquered
    /// continents, gets an expansion target of kind 1 at the top-left field
    /// of the first square holding the good (Q-161c).
    pub(crate) fn expansion_for_waiting(&mut self, ctx: &mut Ctx) {
        // Only while the cell system's target slot is empty (Q-365): a
        // target made stays until the expansion system takes it.
        if self.expansion.is_pending(Source::WaitingNeed) {
            return;
        }
        // The first entry of the waiting list (AI-CON-3.4: the need put on
        // it longest ago).
        let Some(&id) = self.plan.waiting.first() else {
            return;
        };
        self.plan.unwait(id);
        let Some(good) = self.plan.need(id).map(|n| n.good) else {
            return;
        };
        let world = ctx.world;
        let kinds = Self::square_kinds(ctx, good);
        let own_cells = self.cells.in_order().to_vec();
        let mut best: Option<(f32, CellPos)> = None;
        // The cell coordinates x outer, y inner (Q-278).
        for cx in 0..world.terrain().width.div_ceil(CELL) {
            for cy in 0..world.terrain().height.div_ceil(CELL) {
                let cell = (cx, cy);
                let v = self.waiting_value(ctx, cell, good, &own_cells);
                // The coordinate's attempt counter (AI-MIL-3.2), not a
                // production count.
                let prods = self.attempts.get(&cell).copied().unwrap_or(0) as f32;
                // AI-CON-1.1c (Q-278, T52): the cell-coordinate resource
                // map, per kind and stored landmass the sum over the
                // coordinate's fields as its last refresh made it; every
                // entry of the good on one of the conquered continents is
                // scored with its own amount.
                let entries: Vec<(u8, i32, u32)> = self
                    .resources
                    .coordinate(cell)
                    .iter()
                    .filter(|e| e.good == good && kinds.contains(&e.kind))
                    .map(|e| (e.kind, e.landmass, e.amount))
                    .collect();
                for (_, landmass, amount) in entries {
                    if amount <= 2 || !self.conquered.contains(&landmass) {
                        continue;
                    }
                    let score = (amount as f32 + v) / (prods + 1.0);
                    if best.is_none_or(|(s, _)| score > s) {
                        best = Some((score, cell));
                    }
                }
            }
        }
        let Some((_, cell)) = best else {
            return;
        };
        // The squares from 4 cx to 4 cx + 4 and 4 cy to 4 cy + 4, x outer,
        // y inner: the first whose entries of the good (all kinds) sum
        // above 0 (Q-365).
        let per_side = CELL / SQUARE;
        let (sx0, sy0) = (cell.0 * per_side, cell.1 * per_side);
        let first = (sx0..=sx0 + per_side)
            .flat_map(|sx| (sy0..=sy0 + per_side).map(move |sy| (sx, sy)))
            .find(|&sq| {
                self.resources
                    .entries(sq)
                    .iter()
                    .filter(|e| e.good == good)
                    .map(|e| e.amount)
                    .sum::<u32>()
                    > 0
            });
        let Some(sq) = first else {
            return;
        };
        let now = world.tick();
        self.expansion
            .set(Source::WaitingNeed, Kind::Position, square_origin(sq), now);
    }

    /// AI-CON-3.5: V for the cell coordinate `cell`: with C the player's
    /// nearest existing cell and S its best square for `good`, `P = max(1,
    /// 6 x threat balance at S + 30 x hexdist(C's centre, S))`, doubled plus
    /// 720 when the player already has a cell at S's own coordinate, and
    /// `V = 1200 - P`; -1000 without a cell or a square.
    fn waiting_value(&self, ctx: &Ctx, cell: CellPos, good: u32, own: &[CellPos]) -> f32 {
        // C: the player's cell whose top-left field is nearest to the
        // coordinate's, the first in list order on ties (Q-278); S is then
        // measured from the square of C's position (AI-CON-1.1a).
        let top_left = origin(cell);
        let Some(c) = own
            .iter()
            .copied()
            .min_by_key(|&c| grid::distance(origin(c), top_left))
        else {
            return -1000.0;
        };
        let Some(c_at) = self.cells.get(c).and_then(|c| c.position) else {
            return -1000.0;
        };
        let from = square_origin(square_of(c_at));
        // S: the coordinate's own square, x outer, y inner, holding the good
        // and with its slot free (T65), nearest to the square of C's
        // position (Q-278).
        let (x0, y0) = (top_left.0 / SQUARE, top_left.1 / SQUARE);
        let per_side = CELL / SQUARE;
        let s = (x0..x0 + per_side)
            .flat_map(|sx| (y0..y0 + per_side).map(move |sy| (sx, sy)))
            .filter(|&sq| {
                self.resources
                    .entries(sq)
                    .iter()
                    .any(|e| e.good == good && e.amount > 0)
                    && !self.square_taken(sq)
            })
            .min_by_key(|&sq| grid::distance(square_origin(sq), from));
        let Some(s) = s else {
            return -1000.0;
        };
        let at = square_origin(s);
        let balance = threat_balance(ctx.world, ctx.military, ctx.alive, self.id, mcell_of(at));
        let mut p = (6.0 * balance + 30.0 * grid::distance(at, from) as f32).max(1.0);
        if self.cells.get(cell).is_some() {
            p = 2.0 * p + 720.0;
        }
        1200.0 - p
    }

    /// AI-CON-3.4, 3.5 (T65): whether a square's slot is taken: a need was
    /// given the square at its arrangement and keeps it, whether or not it
    /// has its production yet.
    fn square_taken(&self, sq: Square) -> bool {
        self.plan
            .needs
            .iter()
            .any(|n| n.square.is_some_and(|(s, _)| s == sq))
    }

    // ---- AI-CON-4 needs construction ----

    /// AI-CON-4.1: a worker for the producer type is available.
    pub(crate) fn worker_available(&self, ctx: &Ctx, building: u32) -> bool {
        let Some(t) = ctx.config.building(building) else {
            return false;
        };
        let Some(worker) = t.worker else {
            return true;
        };
        if self.econ.stock(worker) > 0 {
            return true;
        }
        let tools: Vec<u32> = ctx
            .config
            .inputs(worker)
            .iter()
            .map(|&(g, _)| g)
            .filter(|g| ctx.config.tool_slots.contains(g))
            .collect();
        tools.is_empty() || tools.iter().any(|&g| self.econ.stock(g) > 0)
    }

    /// AI-2.3 needs construction (every 17 ticks): the good with the best
    /// score gets an order for its producer.
    pub(crate) fn needs_construction(&mut self, ctx: &mut Ctx) {
        if self.build.busy(Slot::Needs) {
            return;
        }
        let mut best: Option<(f32, u32)> = None;
        // AI-CON-4.2: ties go to the first good in id order, the id read
        // as a signed 32-bit number.
        let mut goods = ctx.config.goods.clone();
        goods.sort_by_key(|&g| g as i32);
        for &g in &goods {
            let demand = self
                .plan
                .needs
                .iter()
                .filter(|n| n.good == g && n.cell.is_some())
                .count() as f32;
            if demand == 0.0 {
                continue;
            }
            let supply = self.plan.producers(g) as f32;
            if demand - supply <= 0.0 {
                continue;
            }
            let Some(need) = self.first_unserved(g) else {
                continue;
            };
            if self.producer_type(ctx, &need).is_none() {
                continue;
            }
            // AI-CON-4.1: the worker check looks in the normal table first,
            // then the mine table. W12: a need arranged for a project's
            // alternative checks the alternative's worker.
            let checked = self.alternative_type(ctx, &need).or_else(|| {
                ctx.config
                    .producer
                    .get(&g)
                    .or_else(|| ctx.config.mine_producer.get(&g))
                    .copied()
            });
            let Some(checked) = checked else {
                continue;
            };
            if !self.within_stock_limit(ctx, g) || !self.worker_available(ctx, checked) {
                continue;
            }
            let score = needs_score(
                demand,
                supply,
                ctx.config.priority_of(g) as f32,
                self.econ.total(g),
                self.average_productivity(ctx, g),
            );
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, g));
            }
        }
        let Some((_, good)) = best else {
            return;
        };
        let Some(need) = self.first_unserved(good) else {
            return;
        };
        let (Some(cell), Some(building)) = (need.cell, self.producer_type(ctx, &need)) else {
            return;
        };
        // AI-CON-7.1 (T88): the order remembers only the need; the
        // production record comes with the placed site.
        let order = Order {
            slot: Slot::Needs,
            area: Area::Cell(cell),
            building,
            weight: NEEDS_WEIGHT,
            time: ctx.world.game_seconds(),
            tag: Tag::Need { need: need.id },
        };
        self.build.issue(order);
    }

    /// The first need of `good` that has a cell and no production.
    fn first_unserved(&self, good: u32) -> Option<crate::plan::Need> {
        self.plan
            .needs
            .iter()
            .find(|n| n.good == good && n.cell.is_some() && n.production.is_none())
            .cloned()
    }

    /// AI-CON-4.3: the producer type of a need: from the mine table if its
    /// square holds the good as an underground resource, else from the
    /// normal table; a fields-only good without a square is refused.
    fn producer_type(&self, ctx: &Ctx, need: &crate::plan::Need) -> Option<u32> {
        if let Some(t) = self.alternative_type(ctx, need) {
            return Some(t);
        }
        let cfg = ctx.config;
        if let Some((_, kind)) = need.square {
            if kind == UNDERGROUND {
                return cfg.mine_producer.get(&need.good).copied();
            }
        }
        if let Some(&t) = cfg.producer.get(&need.good) {
            return Some(t);
        }
        None
    }

    /// Hook W12: the alternative's type for a need of a square good with a
    /// cell and no square (only the alternative arranges such a need).
    fn alternative_type(&self, ctx: &Ctx, need: &crate::plan::Need) -> Option<u32> {
        if need.cell.is_none() || need.square.is_some() || !self.is_square_good(ctx, need.good) {
            return None;
        }
        ctx.config.alternatives.get(&need.good).map(|a| a.building)
    }

    pub(crate) fn average_productivity(&self, ctx: &Ctx, good: u32) -> f32 {
        let mut sum = 0.0f32;
        let mut count = 0u32;
        // AI-CON-4.2 (T107, E-ai-075): over exactly the production records
        // A(g) counts, in their order, each with its own building's value; a
        // building of the good with no record (the hunter's hut, AI-CON-2.5a)
        // does not count. T78: sites count, with the 0.5 their placement
        // gave them (ECO-OUT-3.11).
        for p in self.plan.prods.iter().filter(|p| p.good == good) {
            let Some(b) = p.building.and_then(|at| ctx.snapshot.building(self.id, at)) else {
                continue;
            };
            // AI-MIL T24: the float32 statistics value.
            sum += f32::from_bits(b.productivity_bits).max(0.0);
            count += 1;
        }
        if count == 0 {
            1.0
        } else {
            sum / count as f32
        }
    }

    /// AI-CON-4.3 handlers.
    pub(crate) fn needs_order_done(&mut self, order: Order, outcome: Outcome, ctx: &mut Ctx) {
        let Tag::Need { need } = order.tag else {
            return;
        };
        match outcome {
            Outcome::Placed(at) => {
                // AI-CON-7.1, SIM-START-4.4 b (T88): the placed site's
                // production record, for the need the order remembered.
                let (Some(good), Area::Cell(cell)) =
                    (self.plan.need(need).map(|n| n.good), order.area)
                else {
                    return;
                };
                let production = self.plan.add_production(good, cell, Some(need));
                if let Some(p) = self.plan.prods.iter_mut().find(|p| p.id == production) {
                    p.building = Some(at);
                }
                if let Some(n) = self.plan.need_mut(need) {
                    n.production = Some(production);
                }
            }
            Outcome::NoSite => {
                let good = self.plan.need(need).map(|n| n.good);
                if let (Some(good), Area::Cell(cell)) = (good, order.area) {
                    let seconds = ctx.world.game_seconds();
                    if let Some(c) = self.cells.get_mut(cell) {
                        c.mark_full(good, seconds);
                    }
                }
                if let Some(n) = self.plan.need_mut(need) {
                    n.cell = None;
                    n.square = None;
                }
            }
            Outcome::NoMaterial => {}
        }
    }
}
