//! Hook W19, a project's (not the original's): paved roads (`docs/extending.md`,
//! "Paved roads"). An overlay gives the rule (`reborn.cfg_setPaving`); a
//! player orders a road paved with `Command::PaveRoad`. The material leaves
//! the player's nearest stocked warehouse ware by ware and is carried over
//! the roads to the road's first flag, where it is laid; once all of it has
//! arrived the road is paved, and its own carriers and donkeys walk its steps
//! faster.

use std::collections::BTreeMap;

use crate::grid::Point;
use crate::{PlayerId, Rejection, World};

/// W19: the paving rule of a project.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PavingRule {
    /// The good a pavement takes.
    pub good: u32,
    /// How many of it per field of the road's path.
    pub per_field: u32,
    /// The speed of a carrier or donkey on its paved road, in percent of
    /// its speed there unpaved (130: 30 % faster).
    pub speed_percent: u32,
}

/// W19: a pavement under way: the wares it takes, sent and laid so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PavingWork {
    pub needed: u32,
    pub sent: u32,
    pub laid: u32,
}

impl World {
    /// W19: the paving rule of the game; none in the original's game.
    pub fn paving_rule(&self) -> Option<PavingRule> {
        self.paving_rule
    }

    /// W19: whether the road with object number `number` is paved.
    pub fn is_paved(&self, number: u32) -> bool {
        self.paved.contains(&number)
    }

    /// DEBUG ONLY, not a game rule: the road with object number `number`
    /// is paved at once (no paving rule, no wares), for tests of other
    /// crates (the view of a paved road, hook W21).
    pub fn debug_pave_road(&mut self, number: u32) {
        self.paved.insert(number);
    }

    /// W19: the pavement under way of the road with object number `number`.
    pub fn paving_work(&self, number: u32) -> Option<PavingWork> {
        self.pavings.get(&number).copied()
    }

    /// W19: whether `Command::PaveRoad` would be accepted for the road
    /// through the inner field `at` now.
    pub fn can_pave(&self, player: PlayerId, at: Point) -> bool {
        self.pave_check(player, at).is_ok()
    }

    fn pave_check(&self, player: PlayerId, at: Point) -> Result<(u32, u32), Rejection> {
        let rule = self.paving_rule.ok_or(Rejection::NotPavable)?;
        let road = self
            .road_at(at)
            .filter(|r| r.owner == player)
            .ok_or(Rejection::NoRoad)?;
        if road.water
            || self.paved.contains(&road.number)
            || self.pavings.contains_key(&road.number)
        {
            return Err(Rejection::NotPavable);
        }
        Ok((road.number, rule.per_field * road.path.len() as u32))
    }

    /// W19 (`Command::PaveRoad`): the pavement of the road through `at`
    /// begins: `per_field` x the path's fields of the good are to be sent.
    pub(crate) fn pave_road(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        let (number, needed) = self.pave_check(player, at)?;
        self.pavings.insert(
            number,
            PavingWork {
                needed,
                ..PavingWork::default()
            },
        );
        Ok(())
    }

    /// W19, each tick (with the roads' part): pavings and pavements of roads
    /// that are gone are dropped; each pavement under way sends one more ware
    /// of the good, if a warehouse of its network holds one and its flag has
    /// room: from the warehouse nearest to the road's first flag (ECO-REQ-5.3's
    /// choice), put at its flag with the road's first flag as destination.
    pub(crate) fn update_pavings(&mut self) {
        let Some(rule) = self.paving_rule else {
            return;
        };
        if self.pavings.is_empty() && self.paved.is_empty() {
            return;
        }
        let roads: BTreeMap<u32, (PlayerId, Point)> = self
            .roads
            .iter()
            .flatten()
            .map(|r| (r.number, (r.owner, r.path[0])))
            .collect();
        self.pavings.retain(|n, _| roads.contains_key(n));
        self.paved.retain(|n| roads.contains_key(n));
        // A ware lost on its way (destroyed with a road it was on, ...) is
        // sent again: at each first flag the pavements' wares not yet laid
        // are at most the wares of the good still bound for that flag.
        let mut travelling: BTreeMap<Point, u32> = BTreeMap::new();
        for (_, w) in self.wares() {
            if w.good == rule.good && w.target.is_none() {
                if let Some(to) = w.destination {
                    *travelling.entry(to).or_default() += 1;
                }
            }
        }
        let numbers: Vec<u32> = self.pavings.keys().rev().copied().collect();
        for number in numbers {
            let flag = roads[&number].1;
            let left = travelling.entry(flag).or_default();
            let work = self.pavings.get_mut(&number).expect("kept");
            let out = work.sent - work.laid;
            let found = out.min(*left);
            *left -= found;
            work.sent = work.laid + found;
        }
        let due: Vec<u32> = self
            .pavings
            .iter()
            .filter(|(_, w)| w.sent < w.needed)
            .map(|(&n, _)| n)
            .collect();
        for number in due {
            let (player, flag) = roads[&number];
            let costs = self.route_costs_to(player, flag, crate::transport::Search::Ware);
            let warehouses = self.warehouses_of(player);
            let Some((h, _)) = self.stocked_warehouse(player, warehouses, rule.good, flag, &costs)
            else {
                continue;
            };
            let from = self.flag_of(h);
            if from == flag {
                // The road starts at the warehouse's own flag: laid at once.
                self.warehouses
                    .get_mut(&h)
                    .expect("a warehouse")
                    .take(rule.good);
                self.pavings.get_mut(&number).expect("due").sent += 1;
                self.paving_ware_arrived(flag, rule.good);
                self.used(rule.good, 1);
                continue;
            }
            if !self.flag_has_room_for(from, Some(flag)) {
                continue;
            }
            self.warehouses
                .get_mut(&h)
                .expect("a warehouse")
                .take(rule.good);
            // SIM-START-4.2: a ware made (out of the stock, as a helper's).
            self.new_number();
            self.new_ware_at(rule.good, from, Some(flag), None);
            self.pavings.get_mut(&number).expect("due").sent += 1;
        }
    }

    /// W19: a ware of the paving good that ends at the bare flag `flag` (its
    /// destination, TRN-CARRY-3.3's `DropWare` case) is laid on the first
    /// pavement under way that starts there and still waits for one; the
    /// last one makes the road paved.
    pub(crate) fn paving_ware_arrived(&mut self, flag: Point, good: u32) {
        if self.paving_rule.is_none_or(|r| r.good != good) || self.pavings.is_empty() {
            return;
        }
        let firsts: BTreeMap<u32, Point> = self
            .roads
            .iter()
            .flatten()
            .map(|r| (r.number, r.path[0]))
            .collect();
        let Some((&number, work)) = self
            .pavings
            .iter_mut()
            .find(|(n, w)| firsts.get(n) == Some(&flag) && w.laid < w.sent)
        else {
            return;
        };
        work.laid += 1;
        if work.laid >= work.needed {
            self.pavings.remove(&number);
            self.paved.insert(number);
        }
    }

    /// W19: a road split by a new flag: both halves keep a pavement (a
    /// pavement under way is dropped, what was laid is lost).
    pub(crate) fn paving_split(&mut self, old: u32, first: u32, second: u32) {
        self.pavings.remove(&old);
        if self.paved.remove(&old) {
            self.paved.insert(first);
            self.paved.insert(second);
        }
    }

    /// W19: the base speed a carrier or donkey of the road `road` walks its
    /// next step with: the paving factor on top when its road is paved and
    /// the step's two fields both lie on it.
    pub(crate) fn paved_speed(
        &self,
        road: usize,
        at: Point,
        next: Option<Point>,
        base: u32,
    ) -> u32 {
        let (Some(rule), Some(next)) = (self.paving_rule, next) else {
            return base;
        };
        let Some(r) = self.roads.get(road).and_then(Option::as_ref) else {
            return base;
        };
        if !self.paved.contains(&r.number) || !r.path.contains(&at) || !r.path.contains(&next) {
            return base;
        }
        base * rule.speed_percent / 100
    }
}
