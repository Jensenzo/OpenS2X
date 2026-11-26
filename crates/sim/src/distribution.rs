//! Warehouse stop and take-out flags, distribution tables and transport
//! priority ranks (`specs/ECO-distribution.md`).

use std::collections::BTreeMap;

use crate::grid::Point;
use crate::requests::PLAYERS;
use crate::transport::WareLocation;
use crate::{GoodKind, PlayerId, Rejection, Settler, SettlerJob, World};

/// ECO-DIST-1.6: storage trips are re-planned when this many re-checks
/// have counted.
const RECHECK_LIMIT: u32 = 200;
/// ECO-DIST-2.2: the smallest distribution weight.
pub const MIN_WEIGHT: f32 = 0.001;

/// ECO-DIST-1.1: the two flags of one warehouse good.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoodFlags {
    /// "No longer stored here".
    pub stop: bool,
    /// "Take out of this warehouse".
    pub out: bool,
}

impl GoodFlags {
    /// ECO-DIST-1.3 cycle: none -> stop -> out -> none; both -> none.
    fn cycled(self) -> Self {
        match (self.stop, self.out) {
            (false, false) => Self {
                stop: true,
                out: false,
            },
            (true, false) => Self {
                stop: false,
                out: true,
            },
            _ => Self::default(),
        }
    }
}

/// ECO-DIST-2.1: one table's default, from the rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionRules {
    /// The goods whose orders look this table up (food: fish, meat, bread).
    pub goods: Vec<u32>,
    /// (consumer key, default weight in thousandths) in list order. The key is a building
    /// type id, or the construction site type for buildings under
    /// construction.
    pub entries: Vec<(u32, u32)>,
}

/// One entry of a player's table: weight and accumulator, float32 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistEntry {
    pub key: u32,
    weight: u32,
    accumulator: u32,
}

impl DistEntry {
    pub fn weight(&self) -> f32 {
        f32::from_bits(self.weight)
    }

    pub fn accumulator(&self) -> f32 {
        f32::from_bits(self.accumulator)
    }
}

/// A player's table for one scarce good (ECO-DIST-2).
pub type DistTable = Vec<DistEntry>;

fn fresh(rules: &DistributionRules) -> DistTable {
    rules
        .entries
        .iter()
        .map(|&(key, weight)| DistEntry {
            key,
            weight: (weight as f32 / 1000.0).to_bits(),
            accumulator: 0f32.to_bits(),
        })
        .collect()
}

impl World {
    // ---- Warehouse flags (ECO-DIST-1) ----

    /// Whether the warehouse on `at` accepts `good` (ECO-DIST-1.2).
    pub fn warehouse_accepts(&self, at: Point, good: u32) -> bool {
        self.warehouses.get(&at).is_none_or(|w| w.accepts(good))
    }

    /// ECO-DIST-1.3: change the flags of one good of the player's
    /// warehouse on `at`; creates the record.
    fn change_flags(
        &mut self,
        player: PlayerId,
        at: Point,
        good: u32,
        change: impl FnOnce(GoodFlags) -> GoodFlags,
    ) -> Result<(), Rejection> {
        self.owned_warehouse(player, at)?;
        if !self.goods.contains_key(&good) && !self.priorities.contains_key(&good) {
            return Err(Rejection::UnknownGood);
        }
        let record = self
            .warehouses
            .get_mut(&at)
            .expect("a warehouse")
            .flags
            .entry(good as i32)
            .or_default();
        *record = change(*record);
        Ok(())
    }

    fn owned_warehouse(&self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        match self.building_at(at) {
            Some(b) if b.owner == player && self.warehouses.contains_key(&at) => Ok(()),
            _ => Err(Rejection::NoBuilding),
        }
    }

    pub(crate) fn warehouse_cycle(
        &mut self,
        player: PlayerId,
        at: Point,
        good: u32,
    ) -> Result<(), Rejection> {
        self.change_flags(player, at, good, GoodFlags::cycled)
    }

    pub(crate) fn warehouse_set(
        &mut self,
        player: PlayerId,
        at: Point,
        good: u32,
        stop: Option<bool>,
        out: Option<bool>,
    ) -> Result<(), Rejection> {
        self.change_flags(player, at, good, |f| GoodFlags {
            stop: stop.unwrap_or(f.stop),
            out: out.unwrap_or(f.out),
        })
    }

    pub(crate) fn warehouse_toggle(
        &mut self,
        player: PlayerId,
        at: Point,
        good: u32,
        stop: bool,
    ) -> Result<(), Rejection> {
        self.change_flags(player, at, good, |f| GoodFlags {
            stop: f.stop ^ stop,
            out: f.out ^ !stop,
        })
    }

    /// ECO-DIST-1.3 all: mode 0 clears both flags, 1 sets stop, 2 sets out,
    /// for every ware (or every settler) good.
    pub(crate) fn warehouse_all(
        &mut self,
        player: PlayerId,
        at: Point,
        mode: u8,
        settlers: bool,
    ) -> Result<(), Rejection> {
        self.owned_warehouse(player, at)?;
        if mode > 2 {
            return Err(Rejection::NoSuchSetting);
        }
        let kind = if settlers {
            GoodKind::Settler
        } else {
            GoodKind::Ware
        };
        let flags = GoodFlags {
            stop: mode == 1,
            out: mode == 2,
        };
        let goods: Vec<u32> = self
            .goods
            .iter()
            .filter(|(_, g)| g.kind == kind)
            .map(|(&id, _)| id)
            .collect();
        let warehouse = self.warehouses.get_mut(&at).expect("a warehouse");
        for good in goods {
            warehouse.flags.insert(good as i32, flags);
        }
        Ok(())
    }

    /// Whether the warehouse on `at` can provide a helper for a ware
    /// leaving it: it has one or the goods to make one (ECO-REQ-3.3).
    fn can_provide_helper(&self, at: Point) -> bool {
        let Some(helper) = self.roles.helper else {
            return false;
        };
        let warehouse = &self.warehouses[&at];
        if warehouse.count(helper) > 0 {
            return true;
        }
        self.goods.get(&helper).is_some_and(|g| {
            !g.costs.is_empty() && g.costs.iter().all(|&(c, n)| warehouse.count(c) >= n)
        })
    }

    /// ECO-DIST-1.6 (ticks with `t mod 3 = 0`): every ware lying at a flag
    /// with a destination checks that the next flag it keeps is still valid
    /// (it exists, is in its flag's ware network, a road joins the two, the
    /// destination accepts the good) and its counter is below 200, the counter
    /// rises; if not, its next flag is computed again (every 603 ticks at
    /// the latest), which leaves it stranded when there is none (ECO-REQ-5.7 then
    /// takes its destination). A cost change alone (TRN-CARRY-5.3) does not
    /// make it compute again. A ware a carrier has claimed (walking to fetch
    /// it) is not re-checked (ECO-REQ-5.3e).
    pub(crate) fn recheck_wares(&mut self) {
        let ids: Vec<usize> = self
            .wares()
            .filter(|(_, w)| {
                matches!(w.location, WareLocation::Flag(_))
                    && w.destination.is_some()
                    && w.claimed_by.is_none()
            })
            .map(|(id, _)| id)
            .collect();
        for id in ids {
            let valid = self.next_still_valid(id);
            let w = self.wares[id].as_mut().expect("live ware");
            // The test comes before the rise (9d69dd5); computing the next
            // flag starts the counter at 0 (`route_ware`).
            if !valid || w.recheck >= RECHECK_LIMIT {
                self.route_ware(id);
            } else {
                w.recheck += 1;
            }
        }
    }

    /// ECO-DIST-1.7 (ticks with `t mod 7 = 0`): every warehouse with flag
    /// records sends at most one unit of a good marked "out".
    pub(crate) fn take_out(&mut self) {
        // ECO-DIST-1.7e (T25): one list of every player's warehouses
        // together, in the order they were made (object number), not
        // player by player.
        let mut list: Vec<(u32, PlayerId, Point)> = (0..PLAYERS)
            .flat_map(|p| {
                let player = PlayerId(p);
                self.warehouses_of(player)
                    .into_iter()
                    .map(move |at| (player, at))
            })
            .filter_map(|(player, at)| Some((self.building_at(at)?.serial, player, at)))
            .collect();
        list.sort_unstable();
        for (_, player, at) in list {
            // ECO-DIST-1.7c a: a warehouse whose door a settler leaving
            // blocks (its 30 ticks, ECO-REQ-3.7) is skipped, no draw.
            if self.warehouses[&at].cooldown > 0 {
                continue;
            }
            let records: Vec<(i32, GoodFlags)> = self.warehouses[&at]
                .flags
                .iter()
                .map(|(&g, &f)| (g, f))
                .collect();
            if records.is_empty() {
                continue;
            }
            let mut r = (u64::from(self.rng.draw()) % records.len() as u64) as i64;
            for (key, flags) in records {
                if !flags.out {
                    continue;
                }
                r -= 1;
                let good = key as u32;
                if self.warehouses[&at].count(good) > 0 && r < 1 {
                    self.send_out(player, at, good);
                    break;
                }
            }
        }
    }

    /// ECO-DIST-1.7 step 4: one unit leaves the warehouse without a
    /// destination.
    fn send_out(&mut self, player: PlayerId, at: Point, good: u32) {
        let settler = self
            .goods
            .get(&good)
            .is_some_and(|g| g.kind == GoodKind::Settler);
        if settler {
            let warehouse = self.warehouses.get_mut(&at).expect("a warehouse");
            warehouse.take(good);
            // ECO-DIST-1.7a: sent like a settler for a request, so the door
            // waits 30 ticks (ECO-REQ-3.6).
            warehouse.cooldown = crate::requests::SEND_COOLDOWN;
            // SIM-START-4.2: a settler made.
            let mut who = Settler {
                number: self.new_number(),
                player,
                profession: good,
                // ECO-REQ-0.2a (2): by the profession.
                list: self.door_list_of(good),
                walker: crate::movement::Walker::new(at),
                job: SettlerJob::Dying { stand: None },
                timer: 0,
                carrying: None,
            };
            self.plan_home_excluding(&mut who, Some(at));
            self.add_settler(who);
            return;
        }
        let out = self.flag_of(at);
        let limit = self.settings.flag_package_limit as usize;
        if self.flag_load(out) >= limit || !self.can_provide_helper(at) {
            return;
        }
        // ECO-DIST-1.7d (Q-265): the warehouse's helper sending, as for an
        // order's ware (ECO-REQ-5.3 b): the door waits 30 ticks, the helper
        // is made at the door with the ware in its hands and puts it down at
        // the flag (5.3c), where it has no destination.
        let helper = self.roles.helper.expect("a helper to provide");
        let made_from = self
            .goods
            .get(&helper)
            .map(|g| g.costs.clone())
            .unwrap_or_default();
        let warehouse = self.warehouses.get_mut(&at).expect("a warehouse");
        warehouse.cooldown = crate::requests::SEND_COOLDOWN;
        if warehouse.count(helper) == 0 {
            // ECO-REQ-3.3: made from its cost list.
            for &(good, n) in &made_from {
                for _ in 0..n {
                    warehouse.take(good);
                }
            }
            warehouse.add(helper, 1);
            for &(good, n) in &made_from {
                self.used(good, n);
            }
        }
        let warehouse = self.warehouses.get_mut(&at).expect("a warehouse");
        warehouse.take(helper);
        warehouse.take(good);
        let door = self.door_field(at);
        let target = self.target_of(at).expect("a warehouse");
        // SIM-START-4.4 f: the helper is made at the door, then the ware.
        let number = self.new_number();
        self.new_number();
        self.add_settler(Settler {
            number,
            player,
            profession: helper,
            list: crate::SettlerList::Carriers,
            walker: crate::movement::Walker::new(door),
            job: SettlerJob::CarryOut {
                warehouse: target,
                destination: None,
                target: None,
                order: None,
                step: crate::settlers::CarryOutStep::Door { score: 0 },
            },
            timer: crate::settlers::HELPER_DOOR_TICKS,
            carrying: Some(good),
        });
    }

    // ---- Distribution tables (ECO-DIST-2) ----

    /// The tables of the rules, one per scarce good, in list order.
    pub fn distribution_rules(&self) -> &[DistributionRules] {
        &self.distribution
    }

    /// The player's tables (ECO-DIST-2.1): the defaults until a command
    /// or a pick changes them.
    pub fn distribution(&self, player: PlayerId) -> Vec<DistTable> {
        self.tables
            .get(&player)
            .cloned()
            .unwrap_or_else(|| self.distribution.iter().map(fresh).collect())
    }

    /// The table that serves orders for `good`.
    pub(crate) fn table_of(&self, good: u32) -> Option<usize> {
        self.distribution
            .iter()
            .position(|t| t.goods.contains(&good))
    }

    /// ECO-DIST-2.2 set weight; the accumulator is kept.
    pub(crate) fn set_weight(
        &mut self,
        player: PlayerId,
        table: u8,
        entry: u8,
        thousandths: u32,
    ) -> Result<(), Rejection> {
        let mut tables = self.distribution(player);
        let e = tables
            .get_mut(usize::from(table))
            .and_then(|t| t.get_mut(usize::from(entry)))
            .ok_or(Rejection::NoSuchSetting)?;
        let value = thousandths as f32 / 1000.0;
        e.weight = if value > MIN_WEIGHT {
            value
        } else {
            MIN_WEIGHT
        }
        .to_bits();
        self.tables.insert(player, tables);
        Ok(())
    }

    /// ECO-DIST-2.2 reset: defaults, accumulators 0.
    pub(crate) fn reset_table(&mut self, player: PlayerId, table: u8) -> Result<(), Rejection> {
        let mut tables = self.distribution(player);
        let rules = self
            .distribution
            .get(usize::from(table))
            .ok_or(Rejection::NoSuchSetting)?;
        tables[usize::from(table)] = fresh(rules);
        self.tables.insert(player, tables);
        Ok(())
    }

    /// The consumer key of the building an order is for (ECO-DIST-2.1): the
    /// construction site type while it is under construction.
    pub fn order_key(&self, order: &crate::Order) -> Option<u32> {
        let b = self.target_building(order.target)?;
        let site_good = self
            .sites
            .get(&order.target.at)
            .is_some_and(|s| s.still_needed(order.good) > 0);
        if b.under_construction || (site_good && self.upgrades.contains_key(&order.target.at)) {
            self.construction
        } else {
            Some(b.building_type)
        }
    }

    /// ECO-DIST-2.3: choose among the open orders `group` (indices into
    /// the order list, all for `good`, one network) by the weighted round
    /// robin of the good's table. The chosen order's stamp becomes `now`.
    pub(crate) fn pick_by_table(
        &mut self,
        player: PlayerId,
        table: usize,
        group: &[usize],
    ) -> Option<usize> {
        let keys: Vec<Option<u32>> = group
            .iter()
            .map(|&i| self.order_key(&self.orders[i]))
            .collect();
        let mut tables = self.distribution(player);
        let entries = &mut tables[table];
        let present: Vec<bool> = entries
            .iter()
            .map(|e| keys.contains(&Some(e.key)))
            .collect();
        // ECO-DIST-2.3a (T26): with no entry present the pick is tried
        // again, once per entry of the table in all; each try adds every
        // weight and zeroes the first entry, and finds no order.
        let mut picked = None;
        for _ in 0..entries.len().max(1) {
            for e in entries.iter_mut() {
                e.accumulator = (e.accumulator() + e.weight()).to_bits();
            }
            let mut best: Option<usize> = None;
            for (i, e) in entries.iter().enumerate() {
                if present[i] && best.is_none_or(|b| e.accumulator() > entries[b].accumulator()) {
                    best = Some(i);
                }
            }
            if let Some(b) = best {
                entries[b].accumulator = 0f32.to_bits();
                picked = Some(entries[b].key);
                break;
            }
            if let Some(first) = entries.first_mut() {
                first.accumulator = 0f32.to_bits();
            }
        }
        self.tables.insert(player, tables);
        let key = picked?;
        // The order that has waited longest; ties go to the first filed.
        let chosen = group
            .iter()
            .zip(&keys)
            .filter(|(_, &k)| k == Some(key))
            .map(|(&i, _)| i)
            .min_by_key(|&i| (self.orders[i].stamp, i))?;
        self.orders[chosen].stamp = self.tick;
        Some(chosen)
    }

    // ---- Transport priority (ECO-DIST-4) ----

    /// The player's transport rank of `good`; smaller is served first. A
    /// good without a rank counts as 0.
    pub fn rank(&self, player: PlayerId, good: u32) -> i64 {
        self.ranks
            .get(&player)
            .and_then(|r| r.get(&good))
            .or_else(|| self.priorities.get(&good))
            .copied()
            .unwrap_or(0)
    }

    /// The player's ranks of every good that has one.
    pub fn ranks(&self, player: PlayerId) -> BTreeMap<u32, i64> {
        self.ranks
            .get(&player)
            .cloned()
            .unwrap_or_else(|| self.priorities.clone())
    }

    /// ECO-DIST-4.2 move: every good of rank `from` gets rank `to`, those
    /// between shift by one the other way.
    pub(crate) fn move_rank(&mut self, player: PlayerId, from: i64, to: i64) {
        let mut ranks = self.ranks(player);
        for rank in ranks.values_mut() {
            *rank = if *rank == from {
                to
            } else if from < to && (from + 1..=to).contains(rank) {
                *rank - 1
            } else if from > to && (to..from).contains(rank) {
                *rank + 1
            } else {
                *rank
            };
        }
        self.ranks.insert(player, ranks);
    }

    pub(crate) fn reset_ranks(&mut self, player: PlayerId) {
        self.ranks.remove(&player);
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
crate::snap_struct!(DistEntry {
    key,
    weight,
    accumulator
});
