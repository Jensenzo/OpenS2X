//! Hook W15, a project's (not the original's): deepening an exhausted mine
//! (`docs/extending.md`, "Deepening a mine"). An overlay gives a mine type a
//! deepening (`reborn.bp_setDeepening`); its player then orders one with
//! `Command::DeepenMine` once the mine is exhausted. The material comes and
//! a builder works it as an upgrade's construction does (MIL-UPG-2.3) while
//! the mine stays as it is; at its end part of each field's first amount
//! comes back and the mine eats more food per product.

use std::collections::BTreeMap;

use crate::grid::Point;
use crate::production::WorkCycle;
use crate::workshop::FieldResource;
use crate::{PlayerId, Rejection, World};

/// W15: the deepening of a mine type, from an overlay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Deepening {
    /// Of each field's first amount, the share that comes back, per mille
    /// (rounded down, at least 1).
    pub share_per_mille: u32,
    /// The most deepenings of one mine.
    pub most: u32,
    /// What one deepening costs, (good, count) in order, delivered as a
    /// site's main material.
    pub costs: Vec<(u32, u32)>,
    /// The food one product eats after the first deepening, the second, ...
    /// (the last one given for any later one; 1 before the first).
    pub food: Vec<u32>,
}

/// W15: a mine's deepenings so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MineDeepening {
    /// Deepenings completed.
    pub done: u32,
    /// One is being paid and worked now.
    pub under_way: bool,
}

impl World {
    /// W15: the deepening of building type `id` (`Rules::deepenings`); none
    /// in the original's game.
    pub fn deepening(&self, id: u32) -> Option<&Deepening> {
        self.deepenings.get(&id)
    }

    /// W15: the deepenings of the mine on `at` so far; none for a mine never
    /// deepened.
    pub fn mine_deepening(&self, at: Point) -> Option<MineDeepening> {
        self.mine_deepenings.get(&at).copied()
    }

    /// W15: whether `Command::DeepenMine` would be accepted for the building
    /// on `at` now.
    pub fn can_deepen(&self, player: PlayerId, at: Point) -> bool {
        self.deepen_check(player, at).is_ok()
    }

    fn deepen_check(&self, player: PlayerId, at: Point) -> Result<u32, Rejection> {
        let building = self
            .building_at(at)
            .filter(|b| b.owner == player && !b.under_construction)
            .ok_or(Rejection::NoBuilding)?;
        let kind = &self.building_types[&building.building_type];
        let deepening = self
            .deepenings
            .get(&building.building_type)
            .filter(|_| kind.cycle == WorkCycle::Mine)
            .ok_or(Rejection::NotDeepenable)?;
        let so_far = self.mine_deepenings.get(&at).copied().unwrap_or_default();
        let product = kind.product.ok_or(Rejection::NotDeepenable)?;
        // ECO-FOOD-2.7's own test: no field within its radius holds ore.
        let exhausted = !self.ore_fields.contains_key(&at)
            && self
                .find_ore_field(at, product, kind.working_radius)
                .is_none();
        if so_far.under_way
            || so_far.done >= deepening.most
            || self.sites.contains_key(&at)
            || !exhausted
        {
            return Err(Rejection::NotDeepenable);
        }
        Ok(building.building_type)
    }

    /// W15 (`Command::DeepenMine`): the construction of the deepening
    /// begins: a site of the mine's own type on its field with the
    /// deepening's costs as its main material, no frame and no levelling,
    /// worked as an upgrade's (MIL-UPG-2.3); the mine goes on meanwhile.
    pub(crate) fn deepen_mine(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        let building_type = self.deepen_check(player, at)?;
        let mut kind = self.building_types[&building_type].clone();
        kind.frame_costs = Vec::new();
        kind.costs = self.deepenings[&building_type].costs.clone();
        let mut site = self.new_site(at, &kind, kind.size);
        site.levelling_needed = false;
        self.upgrades.insert(at, building_type);
        self.sites.insert(at, site);
        self.mine_deepenings.entry(at).or_default().under_way = true;
        Ok(())
    }

    /// W15: the deepening's construction is done (its upgrade completes):
    /// each field within the mine's working radius whose first resource was
    /// the mine's product gets back `share_per_mille` of its first amount,
    /// rounded down but at least 1 (a field holding another good now is left
    /// as it is, one holding more keeps it), and the mine counts one more.
    pub(crate) fn deepening_done(&mut self, at: Point) {
        let Some(so_far) = self.mine_deepenings.get_mut(&at).filter(|d| d.under_way) else {
            return;
        };
        so_far.under_way = false;
        so_far.done += 1;
        let Some(building) = self.building_at(at).copied() else {
            return;
        };
        let kind = &self.building_types[&building.building_type];
        let (Some(product), radius) = (kind.product, kind.working_radius) else {
            return;
        };
        let share = self
            .deepenings
            .get(&building.building_type)
            .map_or(0, |d| d.share_per_mille);
        for field in self.flood_within(at, radius) {
            let Some(first) = self.first_resources.get(&field).copied() else {
                continue;
            };
            if first.good != product {
                continue;
            }
            let back = ((u32::from(first.amount) * share) / 1000).max(1);
            let back = back.min(u32::from(crate::workshop::ORE_CAP)) as u8;
            match self.resource_at(field) {
                Some(r) if r.good != product || r.amount >= back => {}
                _ => self.set_resource(
                    field,
                    Some(FieldResource {
                        good: product,
                        amount: back,
                    }),
                ),
            }
        }
    }

    /// W15: the food one product of the mine on `at` eats (ECO-PROD-2.2 is
    /// 1): after its d-th deepening the deepening's d-th food value.
    pub(crate) fn mine_food_per_product(&self, at: Point) -> u32 {
        let Some(done) = self
            .mine_deepenings
            .get(&at)
            .map(|d| d.done)
            .filter(|&d| d > 0)
        else {
            return 1;
        };
        let Some(building) = self.building_at(at) else {
            return 1;
        };
        self.deepenings
            .get(&building.building_type)
            .and_then(|d| d.food.get(done as usize - 1).or(d.food.last()))
            .copied()
            .unwrap_or(1)
            .max(1)
    }

    /// W15: before a unit is first taken from `field`, a game whose rules
    /// have deepenings keeps the field's resource as its first.
    pub(crate) fn keep_first_resource(&mut self, field: Point) {
        if self.deepenings.is_empty() || self.first_resources.contains_key(&field) {
            return;
        }
        if let Some(resource) = self.resource_at(field) {
            self.first_resources.insert(field, resource);
        }
    }
}

/// W15: the empty maps of a game without deepenings.
pub(crate) type FirstResources = BTreeMap<Point, FieldResource>;
