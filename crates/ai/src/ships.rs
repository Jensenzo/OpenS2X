//! Shipyard, ships and expeditions (`specs/AI-misc.md` AI-MISC-1).

use s2_sim::grid::Point;
use s2_sim::{Command, ExpeditionState, ShipState};

use crate::construction::{Order, Slot, Tag};
use crate::player::{AiPlayer, Category, Ctx};
use crate::sites::Area;
use crate::view::dist;

/// AI-MISC-1.1: the shipyard needs this many planks in stock to work.
const MIN_PLANKS: u32 = 12;
/// AI-MISC-1.1: ships wanted per harbour.
const SHIPS_PER_HARBOUR: u32 = 2;
/// AI-MISC-1.1: the shipyard order's weight.
const SHIPYARD_WEIGHT: u32 = 10;
/// AI-MISC-1.3: the ship builds a harbour when the spot is closer than this
/// (SHIP-7.1).
const BUILD_DISTANCE: u32 = 5;

/// What the expedition ship is told to do (AI-MISC-1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sail {
    /// Found the harbour on the landing spot.
    Build,
    /// Sail to the landing option in this compass direction.
    Go(u8),
}

/// AI-MISC-1.3: the decision for an expedition ship that stands still with an
/// active expedition. `spot_valid` says whether the known landing spot can
/// still take a harbour; `pick` draws a number below its argument.
pub fn sail_decision(
    vessel: &s2_sim::Vessel,
    position: Point,
    spot_valid: impl Fn(Point) -> bool,
    mut pick: impl FnMut(u32) -> u32,
) -> Option<Sail> {
    if vessel.walker.is_walking() || vessel.state != ShipState::Waiting || !vessel.expedition {
        return None;
    }
    if let Some(target) = vessel.target {
        if dist(position, target.spot) < BUILD_DISTANCE && spot_valid(target.spot) {
            return Some(Sail::Build);
        }
    }
    let directions: Vec<u8> = (0..6u8)
        .filter(|&d| vessel.options[usize::from(d)].is_some())
        .collect();
    if directions.is_empty() {
        return None;
    }
    Some(Sail::Go(directions[pick(directions.len() as u32) as usize]))
}

impl AiPlayer {
    /// AI-MISC-1.1, every 673 ticks: order a shipyard in a cell with a
    /// finished harbour, or run the one it has.
    pub(crate) fn ship_construction(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let n = ctx.config.names.clone();
        let shipyard = ctx
            .snapshot
            .of(self.id)
            .iter()
            .find(|b| b.building_type == n.shipyard)
            .copied();
        let Some(yard) = shipyard else {
            if self.build.busy(Slot::Ship) {
                return;
            }
            let cell = self
                .cells
                .iter()
                .find(|c| {
                    !c.is_full(n.ship)
                        && c.buildings.iter().any(|a| {
                            self.records
                                .get(a)
                                .is_some_and(|r| r.category == Category::Harbour)
                                && ctx
                                    .snapshot
                                    .building(self.id, *a)
                                    .is_some_and(|b| !b.under_construction)
                        })
                })
                .map(|c| c.pos);
            if let Some(cell) = cell {
                self.build.issue(Order {
                    slot: Slot::Ship,
                    area: Area::Cell(cell),
                    building: n.shipyard,
                    weight: SHIPYARD_WEIGHT,
                    time: world.game_seconds(),
                    tag: Tag::None,
                });
            }
            return;
        };
        if yard.under_construction {
            return;
        }
        // Ship mode (the switch is on) rather than boat mode.
        if !yard.production.switched {
            ctx.out.push(Command::ProductionSwitch {
                player: self.id,
                at: yard.at,
            });
        }
        let harbours = self
            .records
            .values()
            .filter(|r| r.category == Category::Harbour)
            .count() as u32;
        let ships = world
            .ships()
            .iter()
            .filter(|s| s.owner == self.id && s.finished())
            .count() as u32;
        let on = self.econ.stock(n.plank) >= MIN_PLANKS && ships < SHIPS_PER_HARBOUR * harbours;
        if yard.production.enabled != on {
            ctx.out.push(Command::ProductionToggle {
                player: self.id,
                at: yard.at,
            });
        }
    }

    /// Whether the harbour's construction costs are in stock.
    fn harbour_costs_in_stock(&self, ctx: &Ctx) -> bool {
        let Some(t) = ctx.config.building(ctx.config.names.harbor) else {
            return false;
        };
        let mut need: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        for &(g, c) in t.frame_costs.iter().chain(&t.costs) {
            *need.entry(g).or_default() += c;
        }
        need.into_iter().all(|(g, c)| self.econ.stock(g) >= c)
    }

    /// AI-MISC-1.2 and 1.3, every 2111 ticks: start an expedition, or move or
    /// settle the expedition ship.
    pub(crate) fn expedition(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let ships = world.ships();
        let expedition = ships
            .iter()
            .enumerate()
            .find(|(_, s)| s.owner == self.id && s.finished() && s.vessel.expedition);
        if let Some((index, ship)) = expedition {
            // 1.3: sailing.
            let decision = sail_decision(
                &ship.vessel,
                ship.position(),
                |spot| world.harbour_site_for(self.id, spot),
                // AI-2.7 c: draw mod (the directions counted).
                |n| ctx.rng.draw() % n.max(1),
            );
            match decision {
                Some(Sail::Build) => ctx.out.push(Command::ExpeditionBuildHarbor {
                    player: self.id,
                    ship: index as u32,
                }),
                Some(Sail::Go(direction)) => ctx.out.push(Command::ExpeditionGo {
                    player: self.id,
                    ship: index as u32,
                    direction,
                }),
                None => {}
            }
            return;
        }
        // 1.2: preparing an expedition. The harbour costs must be in stock.
        if !self.harbour_costs_in_stock(ctx) {
            return;
        }
        // AI-CON-1.1b (b), (c): the harbours of the cells on the home
        // landmass.
        let harbours: Vec<Point> = self
            .cells
            .iter()
            .filter(|c| c.landmass == self.home_landmass)
            .flat_map(|c| c.buildings.iter().copied())
            .filter(|a| {
                self.records
                    .get(a)
                    .is_some_and(|r| r.category == Category::Harbour)
                    && ctx
                        .snapshot
                        .building(self.id, *a)
                        .is_some_and(|b| !b.under_construction)
            })
            .collect();
        for at in harbours {
            let Some(h) = world.harbour_at(at) else {
                continue;
            };
            if h.expedition != ExpeditionState::Inactive {
                // One is being prepared: nothing happens.
                return;
            }
            let idle = ships.iter().any(|s| {
                s.owner == self.id
                    && s.finished()
                    && s.vessel.launched
                    && s.vessel.harbour == Some(at)
                    && !s.vessel.expedition
                    && s.vessel.target.is_none()
                    && s.vessel.cargo.is_empty()
                    && !s.vessel.walker.is_walking()
            });
            if idle {
                ctx.out.push(Command::ExpeditionStart {
                    player: self.id,
                    at,
                });
                return;
            }
        }
    }
}
