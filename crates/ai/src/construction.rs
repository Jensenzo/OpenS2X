//! The construction system (AI-CON-5): order slots, the budget, and the
//! execution of one order at a time with a site search.

use s2_sim::grid::Point;
use s2_sim::Command;

use crate::cells::CellPos;
use crate::expansion::Source;
use crate::maps::MPos;
use crate::plan::NeedId;
use crate::player::{AiPlayer, Ctx};
use crate::sites::{class_fits, military_near, Area, Scan, SiteEnv};

/// A restart of the site search after a placement fell through.
const MAX_RESTARTS: u32 = 3;

/// AI-CON-5.2: the order in which ties of the pick are kept, the earlier
/// source winning.
const PICK_ORDER: [Slot; 9] = [
    Slot::Needs,
    Slot::Cell,
    Slot::Military,
    Slot::Forester,
    Slot::Catapult,
    Slot::Depot,
    Slot::Harbour,
    Slot::Ship,
    Slot::Hunter,
];

/// AI-CON-5.1: the nine order sources in tie-break order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    Cell = 0,
    Needs,
    Military,
    Forester,
    Catapult,
    Depot,
    Harbour,
    Ship,
    Hunter,
}

/// What a source remembers about its order for the handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    None,
    /// AI-CON-4.3, 7.1: the need. A needs order makes no production: the
    /// production record comes with the placed site.
    Need {
        need: NeedId,
    },
    /// AI-MIL-4: a military building for an expansion target: the failed
    /// attempts so far, the level before they are taken off (AI-MIL-4.3),
    /// the military-map cell and the source of the target.
    Military {
        attempts: u32,
        level: i32,
        cell: MPos,
        source: Source,
    },
}

/// AI-CON-5.1: an order is a cell (or a point), a building type, a weight
/// and the time it was issued, on the game-seconds clock (ECO-TIME).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub slot: Slot,
    pub area: Area,
    pub building: u32,
    pub weight: u32,
    pub time: f32,
    pub tag: Tag,
}

/// How an order ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Placed(Point),
    NoSite,
    NoMaterial,
}

/// AI-CON-5.3a: the states of an execution, one advanced per tick.
#[derive(Debug, Clone)]
enum Phase {
    /// Taken by the budget step; the checks run in the next tick.
    Taken,
    /// The site search, one step per tick.
    Scanning(Scan),
    /// The search's result, placed in the next tick.
    Found(Point),
    /// The order has ended in this tick; the system is idle from the next.
    Ended,
}

#[derive(Debug, Clone)]
pub struct Exec {
    pub order: Order,
    phase: Phase,
    restarts: u32,
}

/// The construction system's state.
#[derive(Debug, Clone, Default)]
pub struct Construction {
    pub slots: [Option<Order>; 9],
    /// AI-CON-5.2: limit L and count C.
    pub limit: u32,
    pub count: u32,
    pub exec: Option<Exec>,
}

impl Construction {
    /// The execution's state (AI-CON-5.3a): "taken", "scanning", "found",
    /// "ended", with the fields tested while scanning.
    pub fn stage(&self) -> Option<(&'static str, usize)> {
        self.exec.as_ref().map(|e| match &e.phase {
            Phase::Taken => ("taken", 0),
            Phase::Scanning(scan) => ("scanning", scan.tested()),
            Phase::Found(_) => ("found", 0),
            Phase::Ended => ("ended", 0),
        })
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// AI-2.4a: an order that ended in an earlier tick is done with; the
    /// system is idle again.
    pub(crate) fn forget_ended(&mut self) {
        if self
            .exec
            .as_ref()
            .is_some_and(|e| matches!(e.phase, Phase::Ended))
        {
            self.exec = None;
        }
    }

    pub fn slot(&self, slot: Slot) -> Option<&Order> {
        self.slots[slot as usize].as_ref()
    }

    /// Whether the source has an order pending or executing.
    pub fn busy(&self, slot: Slot) -> bool {
        self.slots[slot as usize].is_some()
            || self
                .exec
                .as_ref()
                .is_some_and(|e| e.order.slot == slot && !matches!(e.phase, Phase::Ended))
    }

    /// File an order (one per source at a time).
    pub fn issue(&mut self, order: Order) -> bool {
        if self.busy(order.slot) {
            return false;
        }
        self.slots[order.slot as usize] = Some(order);
        true
    }
}

impl AiPlayer {
    /// AI-2.4 budget reset: both counters to 0.
    pub(crate) fn budget_reset(&mut self) {
        self.build.limit = 0;
        self.build.count = 0;
    }

    /// AI-2.4 / AI-CON-5.2, every 50 ticks while no order is executing: raise
    /// the count while it is below the limit, else take the pending order with
    /// the largest (now - order time) x weight, which must be above 0. Taking
    /// it is all that happens in this tick (AI-CON-5.3a).
    pub(crate) fn budget_step(&mut self, ctx: &mut Ctx) {
        if self.build.exec.is_some() {
            return;
        }
        if self.build.count < self.build.limit {
            self.build.count += 1;
            return;
        }
        let now = ctx.world.game_seconds();
        let mut best: Option<(f32, Slot)> = None;
        for slot in PICK_ORDER {
            if let Some(o) = &self.build.slots[slot as usize] {
                let value = (now - o.time) * o.weight as f32;
                if value > best.map_or(0.0, |(v, _)| v) {
                    best = Some((value, slot));
                }
            }
        }
        let Some((_, slot)) = best else {
            return;
        };
        let order = self.build.slots[slot as usize]
            .take()
            .expect("a pending order");
        self.build.exec = Some(Exec {
            order,
            phase: Phase::Taken,
            restarts: 0,
        });
    }

    /// AI-CON-5.3 (1), in the tick after the take: check material (twice
    /// the type's construction cost, the frame part and the main part each
    /// counted twice, AI-MIL T18) and cell, then set up the site search, no
    /// field tested yet.
    fn begin(&mut self, exec: Exec, ctx: &mut Ctx) {
        let order = exec.order;
        let Some(t) = ctx.world.building_type(order.building) else {
            self.end(exec, Outcome::NoSite, ctx);
            return;
        };
        let affordable = t
            .frame_costs
            .iter()
            .chain(&t.costs)
            .fold(
                std::collections::BTreeMap::<u32, u32>::new(),
                |mut m, &(g, n)| {
                    *m.entry(g).or_default() += n;
                    m
                },
            )
            .into_iter()
            .all(|(g, n)| self.econ.stock(g) >= 2 * n);
        if !affordable {
            self.end(exec, Outcome::NoMaterial, ctx);
            return;
        }
        if let Area::Cell(cell) = order.area {
            if self.cells.get(cell).is_none() {
                self.end(exec, Outcome::NoSite, ctx);
                return;
            }
        }
        // AI-CON-1.1a: a cell's routes start at its position.
        let start = match order.area {
            Area::Cell(cell) => self.cells.get(cell).map(|c| c.route_start()),
            Area::Point { .. } => None,
        };
        let scan = Scan::new(ctx.world, order.area, order.building, start);
        self.build.exec = Some(Exec {
            phase: Phase::Scanning(scan),
            ..exec
        });
    }

    /// The executing order's step (AI-CON-5.3a): one state of the
    /// execution, the construction system's last step in the tick, after
    /// its inner parts (AI-2.4a).
    pub(crate) fn construction_tick(&mut self, ctx: &mut Ctx) {
        let Some(mut exec) = self.build.exec.take() else {
            return;
        };
        match &mut exec.phase {
            Phase::Ended => {}
            Phase::Taken => self.begin(exec, ctx),
            Phase::Scanning(scan) => {
                // A step that finds the search finished takes the result.
                if scan.done() {
                    match scan.best() {
                        Some(at) => {
                            exec.phase = Phase::Found(at);
                            self.build.exec = Some(exec);
                        }
                        None => self.end(exec, Outcome::NoSite, ctx),
                    }
                    return;
                }
                let env = SiteEnv {
                    world: ctx.world,
                    config: ctx.config,
                    player: self.id,
                    own: ctx.snapshot.of(self.id),
                };
                scan.step(&env);
                self.build.exec = Some(exec);
            }
            Phase::Found(at) => {
                let at = *at;
                if !self.final_check(ctx, exec.order.building, at) {
                    self.restart_or_fail(exec, ctx);
                    return;
                }
                // The site is placed inside this tick (AI-2.1a); how many
                // records it numbers is filled in at the end of the
                // player's part (SIM-START-4.4 b).
                ctx.out.push(Command::PlanBuilding {
                    player: self.id,
                    building: exec.order.building,
                    at,
                    records: 0,
                });
                // AI-CON-5.2: a placed building raises the limit.
                self.build.limit = 2 * self.build.limit + 2;
                self.end(exec, Outcome::Placed(at), ctx);
            }
        }
    }

    /// The order's end: its source's handler runs now, the system stays busy
    /// to the end of the tick (AI-CON-5.3a).
    fn end(&mut self, exec: Exec, outcome: Outcome, ctx: &mut Ctx) {
        self.build.exec = Some(Exec {
            phase: Phase::Ended,
            ..exec
        });
        self.finish_order(exec.order, outcome, ctx);
    }

    /// AI-CON-5.3 (3): the field's class still fits the size and, for a
    /// type with a territory range, the military clause of AI-CON-5.4a
    /// holds (no building or site with a territory range within 4).
    fn final_check(&self, ctx: &Ctx, building: u32, at: Point) -> bool {
        let world = ctx.world;
        let Some(t) = world.building_type(building) else {
            return false;
        };
        let class = world.compute_placement(self.id, at);
        if !class_fits(t.size, class) {
            return false;
        }
        !(t.territory_range > 0 && military_near(world, at))
    }

    /// AI-CON-5.3 (3): a field that no longer fits restarts the order at (1)
    /// in the next tick.
    fn restart_or_fail(&mut self, exec: Exec, ctx: &mut Ctx) {
        if exec.restarts >= MAX_RESTARTS {
            self.end(exec, Outcome::NoSite, ctx);
            return;
        }
        self.build.exec = Some(Exec {
            order: exec.order,
            phase: Phase::Taken,
            restarts: exec.restarts + 1,
        });
    }

    /// The failure and success handlers of each source.
    fn finish_order(&mut self, order: Order, outcome: Outcome, ctx: &mut Ctx) {
        match order.slot {
            Slot::Needs => self.needs_order_done(order, outcome, ctx),
            Slot::Military | Slot::Catapult => self.military_order_done(order, outcome, ctx),
            Slot::Ship => {
                if let (Outcome::NoSite, Area::Cell(cell)) = (outcome, order.area) {
                    // AI-MISC-1.1: a failed order marks the cell full for `ship`.
                    let seconds = ctx.world.game_seconds();
                    let ship = ctx.config.names.ship;
                    if let Some(c) = self.cells.get_mut(cell) {
                        c.mark_full(ship, seconds);
                    }
                }
            }
            Slot::Cell | Slot::Forester | Slot::Depot | Slot::Harbour | Slot::Hunter => {}
        }
    }

    /// The cell order of a building type for a cell (helper for sources).
    pub(crate) fn cell_order(
        &self,
        ctx: &Ctx,
        slot: Slot,
        cell: CellPos,
        building: u32,
        weight: u32,
    ) -> Order {
        Order {
            slot,
            area: Area::Cell(cell),
            building,
            weight,
            time: ctx.world.game_seconds(),
            tag: Tag::None,
        }
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_enum!(Phase {
    Taken,
    Scanning(f0),
    Found(f0),
    Ended
});

s2_sim::snap_struct!(Exec {
    order,
    phase,
    restarts
});
