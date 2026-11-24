//! Production control, tools and food distribution (AI-CON-10).

use s2_sim::grid::Point;
use s2_sim::{Command, GoodKind, SettlerJob};

use crate::plan::NeedId;
use crate::player::{AiPlayer, Category, Ctx};

impl AiPlayer {
    /// AI-CON-10.1: the stock limit of a limited ware, 40 + 60 S / 9216.
    fn stock_limit(&mut self, ctx: &Ctx) -> u32 {
        40 + (self.statistic(ctx) * 60) / 9216
    }

    /// AI-CON-10.1: the goods production control looks at. Group goods and
    /// deposit or field goods only if they are donkey, corn or fish; the
    /// other wares all but coin, plank and boat.
    pub(crate) fn controlled(ctx: &Ctx, good: u32) -> bool {
        let cfg = ctx.config;
        let n = &cfg.names;
        let kind = cfg.sim.goods.get(&good).map(|g| g.kind);
        let special = [n.donkey, n.corn, n.fish].contains(&good);
        match kind {
            Some(GoodKind::Group) => special,
            Some(GoodKind::Ware) => {
                if cfg.deposits_goods.contains(&good) || cfg.fields_goods.contains(&good) {
                    special
                } else {
                    ![n.coin, n.plank, n.boat].contains(&good)
                }
            }
            _ => false,
        }
    }

    /// The count compared with the limit: `stock + stock2`, for donkeys the
    /// stock of the settler `good_donkey` only.
    fn counted(&self, ctx: &Ctx, good: u32) -> u32 {
        if good == ctx.config.names.donkey {
            self.econ.stock(ctx.config.names.donkey_settler)
        } else {
            self.econ.total(good)
        }
    }

    /// AI-CON-4.1: a limited good must be below its stock limit.
    pub(crate) fn within_stock_limit(&mut self, ctx: &Ctx, good: u32) -> bool {
        !Self::controlled(ctx, good) || self.counted(ctx, good) < self.stock_limit(ctx)
    }

    /// AI-CON-10.1 (T73): the goods in the order production control walks
    /// them, their ids read as signed 32-bit numbers (as AI-CON-4.2).
    pub(crate) fn control_order(goods: &[u32]) -> Vec<u32> {
        let mut order = goods.to_vec();
        order.sort_by_key(|&g| g as i32);
        order
    }

    /// AI-CON-10.1, every 67 ticks: one controlled good per run (cursor).
    pub(crate) fn production_control(&mut self, ctx: &mut Ctx) {
        let goods = &Self::control_order(&ctx.config.goods);
        if goods.is_empty() {
            return;
        }
        let mut chosen = None;
        for _ in 0..goods.len() {
            let i = self.control_cursor % goods.len();
            self.control_cursor = (i + 1) % goods.len();
            let g = goods[i];
            if Self::controlled(ctx, g) {
                chosen = Some(g);
                break;
            }
        }
        let Some(good) = chosen else {
            return;
        };
        let below = self.within_stock_limit(ctx, good);
        for b in ctx.snapshot.of(self.id) {
            if b.under_construction {
                continue;
            }
            let makes = ctx
                .world
                .building_type(b.building_type)
                .is_some_and(|t| t.product == Some(good));
            if makes && b.production.enabled != below {
                ctx.out.push(Command::ProductionToggle {
                    player: self.id,
                    at: b.at,
                });
            }
        }
    }

    /// AI-CON-10.2, every 337 ticks, not during need stage 1: the tool the
    /// locksmith makes next.
    pub(crate) fn locksmith(&mut self, ctx: &mut Ctx) {
        if self.need_stage == 1 {
            return;
        }
        let tools = ctx.config.tool_slots.clone();
        if tools.is_empty() {
            return;
        }
        let names = ctx.config.names.clone();
        let missing: Vec<usize> = (0..tools.len())
            .filter(|&i| self.econ.total(tools[i]) == 0)
            .collect();
        let tools_need: Option<NeedId> = self
            .plan
            .needs
            .iter()
            .find(|n| n.good == names.tools)
            .map(|n| n.id);
        let producer_at = tools_need
            .and_then(|id| self.plan.need(id))
            .and_then(|n| n.production)
            .and_then(|p| self.plan.production(p))
            .and_then(|p| p.building)
            .filter(|&at| {
                ctx.world
                    .building_at(at)
                    .is_some_and(|b| !b.under_construction)
            });
        if missing.is_empty() {
            // Nothing missing: switch the tools building off.
            if let Some(at) = producer_at {
                if ctx
                    .world
                    .building_at(at)
                    .is_some_and(|b| b.production.enabled)
                {
                    ctx.out.push(Command::ProductionToggle {
                        player: self.id,
                        at,
                    });
                }
            }
            return;
        }
        // AI-CON-10.2 "which one is next" (T110): the missing tools in good-id
        // order (the id read as signed); the one after the last handled when
        // that is in the list (cyclic), else the last of the list.
        let mut missing = missing;
        missing.sort_by_key(|&i| tools[i] as i32);
        let chosen = match missing.iter().position(|&i| i == self.locksmith_last) {
            Some(k) => missing[(k + 1) % missing.len()],
            None => missing[missing.len() - 1],
        };
        if tools_need.is_none() {
            // AI-CON-10.2 (T54): the tools need has no consumer building and
            // is not tied to the headquarters.
            let t = ctx.world.game_seconds();
            self.plan.add_need(names.tools, None, t);
            return;
        }
        if let Some(at) = producer_at {
            if ctx
                .world
                .building_at(at)
                .is_some_and(|b| !b.production.enabled)
            {
                ctx.out.push(Command::ProductionToggle {
                    player: self.id,
                    at,
                });
            }
            // The sliders: 1.0 for the chosen tool, the floor for the others
            // (only what differs from the current setting is sent). Only now
            // the chosen tool becomes the last handled (AI-CON-10.2, T110).
            self.locksmith_last = chosen;
            let current = ctx.world.tool_priorities(self.id);
            for slot in 0..tools.len() {
                let wanted: f32 = if slot == chosen { 1.0 } else { 0.001 };
                if (current.weight(slot) - wanted).abs() > 1e-6 {
                    ctx.out.push(Command::ToolWeight {
                        player: self.id,
                        slot: slot as u8,
                        per_mille: if slot == chosen { 1000 } else { 1 },
                    });
                }
            }
        }
    }

    /// AI-CON-10.3 (T87): some need of `ore`, in need-map order, has a
    /// consumer that is ready (AI-MIL-6.2), takes the ore as an input,
    /// has its worker in it and holds none of it (an input never delivered
    /// counts as none).
    pub(crate) fn ore_starved(&self, ctx: &Ctx, ore: u32) -> bool {
        let mine = ctx.snapshot.of(self.id);
        self.plan
            .needs
            .iter()
            .filter(|n| n.good == ore)
            .filter_map(|n| mine.iter().find(|b| Some(b.at) == n.consumer))
            .any(|b| {
                let takes = ctx
                    .config
                    .building(b.building_type)
                    .and_then(|t| t.product)
                    .is_some_and(|p| ctx.config.inputs(p).iter().any(|&(g, _)| g == ore));
                let holds = ctx
                    .world
                    .inputs(b.at)
                    .iter()
                    .any(|&(g, k)| g == ore && k > 0);
                // T98: "its worker in it": one of its workers stands, not
                // walking, on the building's own field at this moment.
                let worker_in = || {
                    ctx.world.settlers().any(|(_, s)| {
                        matches!(s.job, SettlerJob::Worker { target, .. }
                            if target.at == b.at && target.serial == b.serial)
                            && s.walker.at == b.at
                            && !s.walker.is_walking()
                    })
                };
                takes && !holds && self.consumer_ready(b) && worker_in()
            })
    }

    /// AI-CON-10.3 (T87), every 643 ticks: the food table's weights for the
    /// mines.
    pub(crate) fn food_arrangement(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let n = ctx.config.names.clone();
        let Some(table) = world
            .distribution_rules()
            .iter()
            .position(|t| t.goods.contains(&n.fish))
        else {
            return;
        };
        let rules = &world.distribution_rules()[table];
        let current = &world.distribution(self.id)[table];
        let mine_type = |name_good: u32| ctx.config.mine_producer.get(&name_good).copied();
        let ore_weight = |ore: u32| {
            if self.econ.stock(ore) > 4 {
                300
            } else if self.ore_starved(ctx, ore) {
                1000
            } else {
                500
            }
        };
        let settings = [
            (
                mine_type(n.stone),
                if self.econ.stock(n.stone) < 5 {
                    1000
                } else {
                    500
                },
            ),
            (mine_type(n.gold), ore_weight(n.gold)),
            (mine_type(n.coal), ore_weight(n.coal)),
            (mine_type(n.iron_ore), ore_weight(n.iron_ore)),
        ];
        for (key, thousandths) in settings {
            let Some(key) = key else { continue };
            if let Some(entry) = rules.entries.iter().position(|e| e.0 == key) {
                let now = (current[entry].weight() * 1000.0).round() as u32;
                if now != thousandths {
                    ctx.out.push(Command::DistributionWeight {
                        player: self.id,
                        table: table as u8,
                        entry: entry as u8,
                        thousandths,
                    });
                }
            }
        }
    }

    /// Set (`on`) or clear the stop (`stop`) or out flag of `good` in the
    /// warehouse on `at`, if it is not so already or the warehouse has no
    /// record of the good yet.
    pub(crate) fn set_warehouse_flag(
        &self,
        ctx: &mut Ctx,
        at: Point,
        good: u32,
        stop: bool,
        on: bool,
    ) {
        let warehouse = ctx.world.warehouse(at);
        let flags = warehouse.map(|w| w.flags_of(good)).unwrap_or_default();
        let now = if stop { flags.stop } else { flags.out };
        // ECO-DIST-1.7c b: the record is found or created, so a missing one
        // is made even when the flag stays as it is (its flags clear).
        let missing = warehouse.is_some_and(|w| !w.flags.contains_key(&(good as i32)));
        if now != on || missing {
            ctx.out.push(Command::WarehouseSet {
                player: self.id,
                at,
                good,
                stop,
                on,
            });
        }
    }

    /// AI-CON-10.4, every 643 ticks: a ten-step cycle, one step per run,
    /// that sets the stop and out flags of the finished storehouses and
    /// harbours (the headquarters is never touched).
    pub(crate) fn goods_arrangement(&mut self, ctx: &mut Ctx) {
        let step = self.goods_step;
        self.goods_step = (step + 1) % 10;
        let world = ctx.world;
        let n = ctx.config.names.clone();
        let finished = |at: Point| {
            ctx.snapshot
                .building(self.id, at)
                .is_some_and(|b| !b.under_construction)
        };
        // T100: the depots in the player's cell list order (creation
        // order), each cell's in the order they were added; unfinished ones
        // skipped.
        let mut storehouses: Vec<Point> = Vec::new();
        for &pos in self.cells.in_order() {
            let Some(cell) = self.cells.get(pos) else {
                continue;
            };
            for &at in &cell.buildings {
                let depot = self
                    .records
                    .get(&at)
                    .is_some_and(|r| r.category == Category::Depot);
                if depot && finished(at) {
                    storehouses.push(at);
                }
            }
        }
        let harbours: Vec<Point> = self
            .records
            .values()
            .filter(|r| r.category == Category::Harbour && finished(r.at))
            .map(|r| r.at)
            .collect();
        let all: Vec<Point> = storehouses.iter().chain(&harbours).copied().collect();
        let soldiers: Vec<u32> = ctx
            .config
            .sim
            .military
            .soldiers
            .iter()
            .flatten()
            .copied()
            .collect();
        match step {
            0 => {
                for good in [n.plank, n.stone, n.coin] {
                    let counts: Vec<(u32, Point)> = storehouses
                        .iter()
                        .map(|&at| (world.warehouse(at).map_or(0, |w| w.count(good)), at))
                        .collect();
                    // AI-CON-10.4 step 0 (T94): each the first found.
                    let Some(((smallest, emptiest), (largest, fullest))) = first_extremes(&counts)
                    else {
                        continue;
                    };
                    if smallest + 40 < largest {
                        // Stop on every finished depot and harbour, then out
                        // on the fullest and stop cleared on the emptiest.
                        // T100: what the run leaves is sent, warehouse by
                        // warehouse (each command is compared with the world
                        // as it was before the run).
                        for &at in &all {
                            self.set_warehouse_flag(ctx, at, good, true, at != emptiest);
                        }
                        self.set_warehouse_flag(ctx, fullest, good, false, true);
                    }
                }
            }
            1 => {
                for good in [n.plank, n.stone, n.coin] {
                    for &at in &all {
                        self.set_warehouse_flag(ctx, at, good, true, false);
                        self.set_warehouse_flag(ctx, at, good, false, false);
                    }
                }
            }
            2 | 7 => {
                // At the front: the cell holds a finished military building
                // with an enemy distance below 36.
                let at_front = |at: Point| {
                    let Some(rec) = self.records.get(&at) else {
                        return false;
                    };
                    self.cells.get(rec.cell).is_some_and(|c| {
                        c.buildings.iter().any(|b| {
                            self.records
                                .get(b)
                                .is_some_and(|r| r.category == Category::Military)
                                && finished(*b)
                                && world
                                    .military_at(*b)
                                    .is_some_and(|m| m.enemy_distance >= 0 && m.enemy_distance < 36)
                        })
                    })
                };
                if all.iter().any(|&at| at_front(at)) {
                    for &at in &all {
                        let front = at_front(at);
                        for &good in &soldiers {
                            self.set_warehouse_flag(ctx, at, good, false, !front);
                        }
                    }
                }
            }
            3 | 8 => {
                for &at in &all {
                    for &good in &soldiers {
                        self.set_warehouse_flag(ctx, at, good, false, false);
                    }
                }
            }
            _ => {}
        }
    }
}

/// AI-CON-10.4 step 0 (T94): the emptiest and the fullest storehouse, each
/// the first found: a strictly smaller or larger count replaces.
pub(crate) fn first_extremes(counts: &[(u32, Point)]) -> Option<((u32, Point), (u32, Point))> {
    let mut emptiest: Option<(u32, Point)> = None;
    let mut fullest: Option<(u32, Point)> = None;
    for &(count, at) in counts {
        if emptiest.is_none_or(|(c, _)| count < c) {
            emptiest = Some((count, at));
        }
        if fullest.is_none_or(|(c, _)| count > c) {
            fullest = Some((count, at));
        }
    }
    Some((emptiest?, fullest?))
}
