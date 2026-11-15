//! Orders driven by the cells (AI-CON-7): foresters, hunters, stone pits and
//! woodcutters, depots, harbours, crowded cells.

use s2_sim::Command;

use crate::cells::{origin, CellPos, CELL};
use crate::construction::{Order, Slot, Tag};
use crate::expansion::{Kind, Source};
use crate::player::{AiPlayer, Ctx};
use crate::sites::Area;

/// AI-CON-7.1 forester order weight; the others are in AI-CON-5.1.
const FORESTER_WEIGHT: u32 = 50;
const HUNTER_WEIGHT: u32 = 10;
const CELL_WEIGHT: u32 = 10;
const DEPOT_WEIGHT: u32 = 15;
const HARBOUR_WEIGHT: u32 = 16;

impl AiPlayer {
    /// AI-CON-7.1, 7.3: trees (or rocks) of `good` in the cell on the
    /// landmass of its current position (its stored continent index,
    /// AI-CON-1.1b; every cell has a position, AI-CON-1.1a), as the
    /// cell-coordinate resource map holds them (AI-CON-1.1c, T52: it lags
    /// the deposits by up to a round of its refresh).
    fn deposits_in_cell(&self, ctx: &Ctx, cell: CellPos, good: u32) -> u32 {
        let world = ctx.world;
        let landmass = self
            .cells
            .get(cell)
            .and_then(|c| c.position)
            .and_then(|p| world.continent_of(p))
            .unwrap_or(-1);
        self.resources
            .coordinate(cell)
            .iter()
            .filter(|e| {
                e.good == good && e.kind == crate::resources::SURFACE && e.landmass == landmass
            })
            .map(|e| e.amount)
            .sum()
    }

    /// The next cell after `cursor`, in cell order, cycling.
    /// AI-CON-7.1, 7.2: the cell at the list index (creation order), the
    /// index advanced after it and back to 0 when it is at or past the end.
    fn next_listed_cell(&self, index: &mut usize) -> Option<CellPos> {
        let cells = self.cells.in_order();
        if *index >= cells.len() {
            *index = 0;
        }
        let cell = cells.get(*index).copied();
        *index += 1;
        cell
    }

    /// AI-CON-7.1 (every 3 ticks, one cell per run).
    pub(crate) fn forester_construction(&mut self, ctx: &mut Ctx) {
        let n = ctx.config.names.clone();
        // Nothing while its own order is pending.
        if self.build.busy(Slot::Forester) {
            return;
        }
        let mut index = self.forester_index;
        let cell = self.next_listed_cell(&mut index);
        self.forester_index = index;
        let Some(cell) = cell else {
            return;
        };
        let limit = if self.econ.stock(n.plank) + self.econ.stock(n.wood) >= 40 {
            10
        } else {
            40
        };
        let trees = self.deposits_in_cell(ctx, cell, n.wood);
        let forester = self.cells.get(cell).and_then(|c| c.forester);
        match forester {
            Some(at) => {
                let building = ctx.world.building_at(at);
                if let Some(b) = building.filter(|b| !b.under_construction) {
                    let want_on = trees <= limit;
                    if b.production.enabled != want_on {
                        ctx.out.push(Command::ProductionToggle {
                            player: self.id,
                            at,
                        });
                    }
                }
            }
            None => {
                // T66: a plank producer of the cell is a placed one.
                if trees < limit && self.plan.placed_producers_in(cell, n.plank) > 0 {
                    let order = Order {
                        slot: Slot::Forester,
                        area: Area::Cell(cell),
                        building: n.forester,
                        weight: FORESTER_WEIGHT,
                        time: ctx.world.game_seconds(),
                        tag: Tag::None,
                    };
                    self.build.issue(order);
                }
            }
        }
    }

    /// AI-CON-7.2 (every 3 ticks, one cell per run).
    pub(crate) fn hunter_construction(&mut self, ctx: &mut Ctx) {
        let n = ctx.config.names.clone();
        // Nothing while its own order is pending.
        if self.build.busy(Slot::Hunter) {
            return;
        }
        let mut index = self.hunter_index;
        let cell = self.next_listed_cell(&mut index);
        self.hunter_index = index;
        let Some(cell) = cell else {
            return;
        };
        if self.cells.get(cell).is_some_and(|c| c.hunter.is_some()) {
            return;
        }
        if self.animals_in_cell(ctx, cell) > 0 {
            let order = Order {
                slot: Slot::Hunter,
                area: Area::Cell(cell),
                building: n.hunter,
                weight: HUNTER_WEIGHT,
                time: ctx.world.game_seconds(),
                tag: Tag::None,
            };
            self.build.issue(order);
        }
    }

    /// AI-CON-7.2 (Q-363, T42): the respawn record's count of the cell
    /// (ECO-FOOD-5.15): the living animals whose home field lies in it,
    /// wherever they stand, explored or not.
    fn animals_in_cell(&self, ctx: &Ctx, cell: CellPos) -> u32 {
        let (x0, y0) = origin(cell);
        ctx.world
            .animals()
            .filter(|(_, a)| {
                a.home.0 >= x0 && a.home.0 < x0 + CELL && a.home.1 >= y0 && a.home.1 < y0 + CELL
            })
            .count() as u32
    }

    /// AI-CON-7.3 (every 53 ticks, one grid position per run): nothing while
    /// its own order is pending; the grid cursor (x fastest, wrapping, from
    /// (0, 0)) is advanced before the run, and the player's cell at that
    /// position, if any, is tested.
    pub(crate) fn pit_and_woodcutter(&mut self, ctx: &mut Ctx) {
        let n = ctx.config.names.clone();
        if self.build.busy(Slot::Cell) {
            return;
        }
        let (cols, rows) = (self.cells.cols, self.cells.rows);
        if cols * rows == 0 {
            return;
        }
        self.pit_cursor = (self.pit_cursor + 1) % (cols * rows);
        let cell = (self.pit_cursor % cols, self.pit_cursor / cols);
        if self.cells.get(cell).is_none() {
            return;
        }
        // The cell's productions of the good or of a good in its group
        // (corrected 2026-10-26 night).
        let producers = |good: u32| {
            self.plan
                .prods
                .iter()
                .filter(|p| p.cell == cell && ctx.config.supplies(p.good, good))
                .count() as u32
        };
        let stone_producers = producers(n.stone);
        let wood_producers = producers(n.wood);
        let building = if stone_producers < 2
            && 5 * stone_producers < self.deposits_in_cell(ctx, cell, n.stone)
        {
            Some(n.stone_pit)
        } else if wood_producers < 2
            && 5 * wood_producers + 5 < self.deposits_in_cell(ctx, cell, n.wood)
        {
            Some(n.woodcutter)
        } else {
            None
        };
        if let Some(building) = building {
            let order = self.cell_order(ctx, Slot::Cell, cell, building, CELL_WEIGHT);
            self.build.issue(order);
        }
    }

    /// AI-CON-7.4 (every 1423 ticks): the oldest depot-wanted marker.
    pub(crate) fn depot_orders(&mut self, ctx: &mut Ctx) {
        let n = ctx.config.names.clone();
        if self.build.busy(Slot::Depot) {
            return;
        }
        let oldest = self
            .cells
            .iter()
            .filter(|c| c.clearing.is_none())
            .filter_map(|c| c.depot_wanted.map(|t| (t, c.pos)))
            .min();
        if let Some((_, cell)) = oldest {
            let now = ctx.world.tick();
            let order = self.cell_order(ctx, Slot::Depot, cell, n.depot, DEPOT_WEIGHT);
            if self.build.issue(order) {
                if let Some(c) = self.cells.get_mut(cell) {
                    c.depot_wanted = Some(now);
                }
            }
        }
    }

    /// AI-CON-7.5 (every 997 ticks): the cell with the oldest *harbour
    /// wanted* marker (AI-CON-1.6, set with the cell's landmass) gets a
    /// harbour order.
    pub(crate) fn harbour_orders(&mut self, ctx: &mut Ctx) {
        let n = ctx.config.names.clone();
        let world = ctx.world;
        let now = world.tick();
        // AI-CON-7.5: harbours are always enabled (Q-169).
        if self.build.busy(Slot::Harbour) {
            return;
        }
        let oldest = self
            .cells
            .iter()
            .filter_map(|c| c.harbour_wanted.map(|t| (t, c.pos)))
            .min();
        if let Some((_, cell)) = oldest {
            let order = self.cell_order(ctx, Slot::Harbour, cell, n.harbor, HARBOUR_WEIGHT);
            if self.build.issue(order) {
                if let Some(c) = self.cells.get_mut(cell) {
                    c.harbour_wanted = Some(now);
                }
            }
        }
    }

    /// AI-CON-7.6 (every 19 ticks): while the crowded source's own slot is
    /// empty, the crowded cell with the largest (now - marker time) x
    /// weight, above 0, sets one; of equal values the first in the cell
    /// list (creation order, T96).
    pub(crate) fn crowded_cells(&mut self, ctx: &mut Ctx) {
        if self.expansion.is_pending(Source::Crowded) {
            return;
        }
        let now = ctx.world.tick();
        let mut best: Option<(u64, CellPos)> = None;
        for &pos in self.cells.in_order() {
            let Some((t, w)) = self.cells.get(pos).and_then(|c| c.crowded) else {
                continue;
            };
            let value = now.saturating_sub(t) * u64::from(w);
            if value > best.map_or(0, |(v, _)| v) {
                best = Some((value, pos));
            }
        }
        if let Some((_, cell)) = best {
            self.expansion
                .set(Source::Crowded, Kind::Crowded, origin(cell), now);
            if let Some(c) = self.cells.get_mut(cell) {
                if let Some((_, w)) = c.crowded {
                    c.crowded = Some((now, w));
                }
            }
        }
    }
}
