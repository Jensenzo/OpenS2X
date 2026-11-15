//! The ware ledger: how many of each ware were made and used up since the
//! game began, for the conservation check of the soak runs
//! (`docs/test-coverage.md`): at every tick the wares held anywhere equal
//! those held at the start plus made minus used. The ledger measures the
//! game; no rule reads it and [`World::state_hash`] leaves it out.

use std::collections::BTreeMap;

use crate::{GoodKind, World};

/// Wares made and used up, by good.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    pub made: BTreeMap<u32, u64>,
    pub used: BTreeMap<u32, u64>,
}

impl World {
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// `count` of `good` came into being (a product, a mission's gift, a
    /// debug command). Goods that are not wares are not counted.
    pub(crate) fn made(&mut self, good: Option<u32>, count: u32) {
        if let Some(good) = good.filter(|&g| self.is_ware(g)) {
            *self.ledger.made.entry(good).or_default() += u64::from(count);
        }
    }

    /// `count` of `good` were used up or destroyed.
    pub(crate) fn used(&mut self, good: u32, count: u32) {
        if self.is_ware(good) {
            *self.ledger.used.entry(good).or_default() += u64::from(count);
        }
    }

    fn is_ware(&self, good: u32) -> bool {
        self.goods
            .get(&good)
            .is_some_and(|g| g.kind == GoodKind::Ware)
    }

    /// Every ware the world holds, by good: in warehouse stocks, in the
    /// buildings' input stocks, on construction sites, lying at flags or
    /// carried, in a settler's hands, set aside in a harbour, and aboard or
    /// in the cargo of a ship.
    pub fn ware_holdings(&self) -> BTreeMap<u32, u64> {
        let mut held: BTreeMap<u32, u64> = BTreeMap::new();
        for (_, good, n) in self.ware_places() {
            *held.entry(good).or_default() += u64::from(n);
        }
        held
    }

    /// [`World::ware_holdings`] by place: `(place, good, count)`, the place
    /// named like `warehouse (12, 30)`, `flag (4, 5)`, `carrier 3`.
    pub fn ware_places(&self) -> Vec<(String, u32, u32)> {
        let mut out = Vec::new();
        let mut add = |place: String, list: &[(u32, u32)]| {
            for &(g, n) in list {
                if n > 0 && self.is_ware(g) {
                    out.push((place.clone(), g, n));
                }
            }
        };
        for (at, w) in &self.warehouses {
            add(format!("warehouse {at:?}"), &w.stock);
        }
        for (at, list) in &self.inputs {
            add(format!("inputs {at:?}"), list);
        }
        for (at, site) in &self.sites {
            add(format!("site {at:?}"), &site.on_site);
        }
        for (id, w) in self.wares.iter().enumerate() {
            let Some(w) = w else { continue };
            let place = match w.location {
                crate::WareLocation::Flag(f) => format!("flag {f:?} (ware {id})"),
                crate::WareLocation::Carried(c) => format!("carrier {c} (ware {id})"),
            };
            add(place, &[(w.good, 1)]);
        }
        for (id, s) in self.settlers.iter().enumerate() {
            if let Some(g) = s.as_ref().and_then(|s| s.carrying) {
                add(format!("settler {id}"), &[(g, 1)]);
            }
        }
        for h in &self.harbours {
            add(format!("harbour {:?} aside", h.at), &h.aside);
        }
        for (i, ship) in self.ships.iter().enumerate() {
            add(format!("ship {i} aboard"), &ship.vessel.aboard);
            for cargo in &ship.vessel.cargo {
                add(format!("ship {i} cargo"), &cargo.goods);
            }
        }
        out
    }
}
