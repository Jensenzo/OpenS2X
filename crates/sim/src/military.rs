//! Soldiers and military buildings (`specs/MIL-soldiers.md`,
//! `specs/MIL-coins-promotion.md`). Fights are in `combat`.

use crate::grid::{self, Point};
use crate::movement::Walker;
use crate::{
    Building, PlayerId, Rejection, Request, Requester, Settler, SettlerJob, Target, World,
};

/// MIL-SOL-1.1: soldier ranks 0 to 4.
pub const SOLDIER_RANKS: usize = 5;
/// MIL-SOL-3.1: the enemy distance when there is no enemy building.
const NO_ENEMY: i32 = 1000;
/// MIL-SOL-3.2: distances above / from which the far and medium factors
/// apply.
const FAR_ABOVE: i32 = 24;
const MEDIUM_FROM: i32 = 19;
/// MIL-SOL-4.4: ticks a building waits between two releases.
pub(crate) const RELEASE_COOLDOWN: u32 = 40;
/// MIL-SOL-4.7a: the ticks a soldier sent for a request stands before its
/// walk starts (its creation tick), kept in its action timer.
pub(crate) const SOLDIER_FIRST_STAND: u32 = 1;
/// MIL-SOL-1.2: the heal counter passes this, then health rises.
const HEAL_PASSES: u32 = 600;
/// MIL-SOL-1.4: radius a walking soldier explores.
const SOLDIER_EXPLORE: u32 = 2;
/// MIL-PROM-3.1: coin check period.
const COIN_PERIOD: u32 = 33;
/// Counters of the global military update (MIL-SOL-3.1, 4.1, 5.1,
/// MIL-PROM-4.1): they count from these values down to 0, then run.
const DISTANCE_TOP: u32 = 10;
const ALLOCATION_TOP: u32 = 20;
const RECRUIT_TOP: u32 = 100;
const PROMOTION_TOP: u32 = 300;
/// MIL-PROM-4.1: the counter after a run.
const PROMOTION_PERIOD_TOP: u32 = 300;
/// MIL-SOL-7.1: the highest slider position the commands accept (Q-060).
const MAX_FACTOR: u32 = 1000;

/// MIL-SOL-1.1: the soldier goods by rank and their health, from the
/// scripts (`mi_setSoldier`, `mi_setSoldierMaxHealth`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MilitaryRules {
    pub soldiers: [Option<u32>; SOLDIER_RANKS],
    pub max_health: [u32; SOLDIER_RANKS],
    /// `mi_setFightingTime` rounded up: ticks of one duel round.
    pub fighting_ticks: u32,
}

/// MIL-SOL-7.1: one player's military settings. Factors are 32-bit floats
/// kept as bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MilitarySettings {
    allocation: [u32; 3],
    attack: u32,
    defenders: u32,
    /// Defense type 1, "prefer strong soldiers".
    pub prefer_strong: bool,
    /// MIL-SOL-5.1: privates one warehouse makes per run.
    pub recruit_count: u32,
}

impl Default for MilitarySettings {
    fn default() -> Self {
        Self {
            allocation: [1.0f32.to_bits(), 0.3f32.to_bits(), 0.1f32.to_bits()],
            attack: 1.0f32.to_bits(),
            defenders: 1.0f32.to_bits(),
            prefer_strong: true,
            recruit_count: 10,
        }
    }
}

impl MilitarySettings {
    /// The allocation factor of class 0 (near), 1 (medium) or 2 (far).
    pub fn allocation(&self, class: usize) -> f32 {
        f32::from_bits(self.allocation[class])
    }

    pub fn attack_strength(&self) -> f32 {
        f32::from_bits(self.attack)
    }

    pub fn defenders(&self) -> f32 {
        f32::from_bits(self.defenders)
    }
}

/// The global counters of the military update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timers {
    distance: u32,
    allocation: u32,
    recruit: u32,
    promotion: u32,
}

impl Default for Timers {
    fn default() -> Self {
        Self {
            distance: DISTANCE_TOP,
            allocation: ALLOCATION_TOP,
            recruit: RECRUIT_TOP,
            promotion: PROMOTION_TOP,
        }
    }
}

/// A counter that is lowered while above 0 and runs when it is 0.
fn due(counter: &mut u32, top: u32) -> bool {
    if *counter > 0 {
        *counter -= 1;
        false
    } else {
        *counter = top;
        true
    }
}

/// What a building with a territory range or explore radius keeps
/// (TER-2.1, MIL-SOL-2, MIL-PROM-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilitaryBuilding {
    /// TER-2.1: the one-time territory flag.
    pub applied: bool,
    /// MIL-SOL-2.2: settler ids of its soldiers, in the order they were
    /// linked.
    pub soldiers: Vec<usize>,
    /// MIL-SOL-4.4: ticks until the next soldier may leave.
    pub cooldown: u32,
    /// MIL-SOL-3.1: the enemy distance; -1 before the first update.
    pub enemy_distance: i32,
    /// MIL-PROM-3.3: the coin accumulator, a 32-bit float as bits.
    pub(crate) accumulator: u32,
    /// MIL-PROM-3.2: coin supply switch, on at creation.
    pub coin_switch: bool,
    /// MIL-SOL-4.6: "move soldiers out".
    pub move_out: bool,
    /// MIL-SOL-6.1: soldiers of each rank the headquarters keeps outside
    /// its stock.
    pub castle: [u32; SOLDIER_RANKS],
}

impl MilitaryBuilding {
    fn new(headquarters: bool) -> Self {
        Self {
            applied: false,
            soldiers: Vec::new(),
            cooldown: 0,
            enemy_distance: -1,
            accumulator: (-1.0f32).to_bits(),
            coin_switch: true,
            move_out: false,
            castle: [u32::from(headquarters); SOLDIER_RANKS],
        }
    }

    pub fn accumulator(&self) -> f32 {
        f32::from_bits(self.accumulator)
    }
}

/// One soldier of a building, for the views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoldierView {
    pub id: usize,
    pub rank: usize,
    pub health: u32,
    pub present: bool,
}

/// MIL-SOL-1.3: of the 7 x 7 pairs of numbers `(r1, r2)` a round draws
/// (each `draw mod 7`), those that make an attacker of rank `attacker` hit
/// a defender of rank `defender`: `r1 <= a` and `r2 > b`, with
/// `a = max(2, attacker + 1)`, `h = a / 2` and `b = defender + 1 - h`,
/// raised to 2 when below 3 (private against private: 12 of 49).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn duel_hit_outcomes(attacker: usize, defender: usize) -> u32 {
    let a = (attacker as i32 + 1).max(2);
    let h = a / 2;
    let mut b = defender as i32 + 1 - h;
    if b < 3 {
        b = 2;
    }
    let mut hits = 0;
    for r1 in 0..7 {
        for r2 in 0..7 {
            if r1 <= a && r2 > b {
                hits += 1;
            }
        }
    }
    hits
}

impl World {
    pub(crate) fn military_settings_mut(&mut self, player: PlayerId) -> &mut MilitarySettings {
        self.military_settings.entry(player).or_default()
    }

    /// MIL-SOL-7.1: the settings of `player`.
    pub fn military_settings(&self, player: PlayerId) -> MilitarySettings {
        self.military_settings
            .get(&player)
            .copied()
            .unwrap_or_default()
    }

    /// The good coins are (`coin`), for the views.
    pub fn coin_good(&self) -> Option<u32> {
        self.roles.coin
    }

    /// The military state of the building on `at`.
    pub fn military_at(&self, at: Point) -> Option<&MilitaryBuilding> {
        self.military.get(&at)
    }

    /// All military buildings by field.
    pub fn military_buildings(&self) -> impl Iterator<Item = (Point, &MilitaryBuilding)> {
        self.military.iter().map(|(&at, m)| (at, m))
    }

    /// TER-5.4: whether the land of the two players has touched.
    pub fn have_met(&self, a: PlayerId, b: PlayerId) -> bool {
        self.contacts.contains(&(a.0.min(b.0), a.0.max(b.0)))
    }

    /// MIL-SOL-1.1: the rank of a soldier profession.
    pub fn soldier_rank(&self, profession: u32) -> Option<usize> {
        self.military_rules
            .soldiers
            .iter()
            .position(|&g| g == Some(profession))
    }

    /// MIL-SOL-1.1: the maximum health of a rank.
    pub fn soldier_max_health(&self, rank: usize) -> u32 {
        self.military_rules
            .max_health
            .get(rank)
            .copied()
            .unwrap_or(0)
    }

    /// The soldiers of the building on `at` with their health.
    pub fn garrison(&self, at: Point) -> Vec<SoldierView> {
        let Some(m) = self.military.get(&at) else {
            return Vec::new();
        };
        m.soldiers
            .iter()
            .filter_map(|&id| {
                let s = self.settlers.get(id)?.as_ref()?;
                let rank = self.soldier_rank(s.profession)?;
                let (health, present) = match s.job {
                    SettlerJob::Garrison { health, .. } => (health, true),
                    SettlerJob::ToMilitary { health, .. } | SettlerJob::Defend { health, .. } => {
                        (health, false)
                    }
                    _ => return None,
                };
                Some(SoldierView {
                    id,
                    rank,
                    health,
                    present,
                })
            })
            .collect()
    }

    /// The soldiers of each rank in the warehouse stock of `at`.
    pub fn stock_soldiers(&self, at: Point) -> [u32; SOLDIER_RANKS] {
        let mut counts = [0; SOLDIER_RANKS];
        if let Some(w) = self.warehouses.get(&at) {
            for (rank, good) in self.military_rules.soldiers.iter().enumerate() {
                counts[rank] = good.map_or(0, |g| w.count(g));
            }
        }
        counts
    }

    /// Number of soldiers present in the building on `at` (MIL-SOL-2.4).
    pub fn present_soldiers(&self, at: Point) -> usize {
        self.present_ranks(at).len()
    }

    /// (settler id, rank) of the present soldiers of `at`, in list order.
    pub(crate) fn present_ranks(&self, at: Point) -> Vec<(usize, usize)> {
        let Some(m) = self.military.get(&at) else {
            return Vec::new();
        };
        m.soldiers
            .iter()
            .filter_map(|&id| {
                let s = self.settlers.get(id)?.as_ref()?;
                matches!(s.job, SettlerJob::Garrison { .. })
                    .then(|| self.soldier_rank(s.profession).map(|r| (id, r)))
                    .flatten()
            })
            .collect()
    }

    /// MIL-SOL-2.1: the building types that count as military.
    fn is_headquarters_type(&self, building: &Building) -> bool {
        self.building_types[&building.building_type].warehouse
    }

    /// Creation of the military state of a finished building: called by
    /// `complete`.
    pub(crate) fn register_military(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        if kind.territory_range == 0 && kind.explore_radius == 0 {
            return;
        }
        let headquarters = kind.warehouse;
        self.military
            .entry(at)
            .or_insert_with(|| MilitaryBuilding::new(headquarters));
    }

    /// Military buildings in the world's list order (creation order).
    pub(crate) fn military_order(&self) -> Vec<Point> {
        let mut found: Vec<(u32, Point)> = self
            .military
            .keys()
            .filter_map(|&at| Some((self.building_at(at)?.serial, at)))
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, at)| at).collect()
    }

    /// MIL-PROM-2.1: whether the building can use coins.
    fn can_use_coins(&self, at: Point) -> bool {
        let Some(building) = self.building_at(at) else {
            return false;
        };
        let kind = &self.building_types[&building.building_type];
        kind.territory_range > 0
            && !kind.warehouse
            && self
                .present_ranks(at)
                .iter()
                .any(|&(_, rank)| rank < SOLDIER_RANKS - 1)
    }

    fn coins_in_store(&self, at: Point) -> u32 {
        let Some(coin) = self.roles.coin else {
            return 0;
        };
        self.inputs.get(&at).map_or(0, |l| {
            l.iter().filter(|&&(g, _)| g == coin).map(|&(_, n)| n).sum()
        })
    }

    // ---- The per-building part (ECO-REQ-0.2, E-mil-002) ----

    /// The military part of one building update of a finished building:
    /// release cooldown, headquarters soldiers or coin check, and the
    /// territory check (TER-2.1).
    pub(crate) fn update_military_building(&mut self, at: Point) {
        let Some(m) = self.military.get_mut(&at) else {
            return;
        };
        m.cooldown = m.cooldown.saturating_sub(1);
        let building = *self.building_at(at).expect("a building");
        if self.is_headquarters_type(&building) {
            self.update_headquarters_soldiers(at);
        } else {
            self.coin_check(at, building.serial);
        }
        self.check_occupation(at);
    }

    /// MIL-PROM-3.1, 3.2
    fn coin_check(&mut self, at: Point, serial: u32) {
        let Some(coin) = self.roles.coin else { return };
        let period_phase = (self.tick as u32).wrapping_add(serial) % COIN_PERIOD;
        if period_phase != 0 {
            return;
        }
        let m = &self.military[&at];
        let present = !self.present_ranks(at).is_empty();
        let usable = present && m.coin_switch && self.can_use_coins(at);
        if !usable {
            let open = self
                .orders
                .iter()
                .filter(|o| o.target.at == at && o.good == coin);
            let open = open.count() as u32;
            if open > 0 {
                self.cancel_orders(at, coin, open);
            }
            return;
        }
        let open = self
            .orders
            .iter()
            .any(|o| o.target.at == at && o.good == coin);
        if open {
            return;
        }
        let building = self.building_at(at).expect("a building");
        let Some(limit) = self.building_types[&building.building_type].stock_limit else {
            return;
        };
        let want = limit.saturating_sub(self.coins_in_store(at));
        for _ in 0..want {
            self.order(at, coin);
        }
    }

    /// MIL-SOL-6.2: every tick the headquarters brings its soldiers to the
    /// wanted number per rank.
    fn update_headquarters_soldiers(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        if self.has_attackers(at) {
            // MIL-SOL-6.3: while attackers are reported and no soldier is
            // present, one of each rank in stock comes out, general first.
            if self.present_soldiers(at) == 0 {
                for rank in (0..SOLDIER_RANKS).rev() {
                    self.bring_out_soldier(at, rank);
                }
            }
            return;
        }
        for rank in 0..SOLDIER_RANKS {
            let Some(good) = self.military_rules.soldiers[rank] else {
                continue;
            };
            let wanted = self.military[&at].castle[rank] as usize;
            let present: Vec<usize> = self
                .present_ranks(at)
                .into_iter()
                .filter(|&(_, r)| r == rank)
                .map(|(id, _)| id)
                .collect();
            if present.len() > wanted {
                // One goes back into the stock.
                let id = present[0];
                self.settlers[id] = None;
                self.military
                    .get_mut(&at)
                    .expect("military")
                    .soldiers
                    .retain(|&s| s != id);
                if let Some(w) = self.warehouses.get_mut(&at) {
                    w.add(good, 1);
                }
            } else if present.len() < wanted {
                self.bring_out_soldier(at, rank);
            }
        }
        let _ = building;
    }

    /// MIL-SOL-6.2: one soldier of `rank` leaves the stock at the door and
    /// is linked to the headquarters, only while the warehouse's door is
    /// free (its send cooldown, ECO-REQ-3.7, at 0); coming out starts the
    /// 30-tick cooldown (T20).
    fn bring_out_soldier(&mut self, at: Point, rank: usize) {
        let Some(good) = self.military_rules.soldiers[rank] else {
            return;
        };
        let owner = self.building_at(at).expect("a building").owner;
        let took = self
            .warehouses
            .get_mut(&at)
            .is_some_and(|w| w.cooldown == 0 && w.take(good));
        if !took {
            return;
        }
        if let Some(w) = self.warehouses.get_mut(&at) {
            w.cooldown = crate::requests::SEND_COOLDOWN;
        }
        let target = self.target_of(at).expect("a building");
        // SIM-START-4.2: a settler made.
        let number = self.new_number();
        let id = self.add_settler(Settler {
            number,
            player: owner,
            profession: good,
            list: crate::SettlerList::Soldiers,
            // MIL-SOL-6.5: it stands on the door field.
            walker: Walker::new(self.door_field(at)),
            job: SettlerJob::Garrison {
                target,
                health: self.military_rules.max_health[rank],
                heal: 0,
            },
            timer: 0,
            carrying: None,
        });
        self.military
            .get_mut(&at)
            .expect("military")
            .soldiers
            .push(id);
    }

    // ---- The soldier's own update (settlers) ----

    /// FMT-SAV-3.4a, 3.5c: a soldier a save names, present in the military
    /// building on `at` with its rank, health (kept within the rank's scale,
    /// 1 to its maximum) and healing counter; false when the profession is no
    /// soldier rank or the building no military one.
    pub(crate) fn seat_soldier(
        &mut self,
        player: PlayerId,
        soldier: crate::restore::RestoredSoldier,
        at: Point,
    ) -> bool {
        let profession = soldier.profession;
        let (Some(rank), Some(target)) = (self.soldier_rank(profession), self.target_of(at)) else {
            return false;
        };
        if !self.military.contains_key(&at) {
            return false;
        }
        let health = soldier
            .health
            .clamp(1, self.soldier_max_health(rank).max(1));
        // SIM-START-4.2: a settler made.
        let number = self.new_number();
        let id = self.add_settler(Settler {
            number,
            player,
            profession,
            list: crate::SettlerList::Soldiers,
            walker: crate::movement::Walker::new(at),
            job: SettlerJob::Garrison {
                target,
                health,
                heal: soldier.healing,
            },
            timer: 0,
            carrying: None,
        });
        if let Some(m) = self.military.get_mut(&at) {
            m.soldiers.push(id);
        }
        true
    }

    /// A soldier walking to its building arrives (MIL-SOL-2.4); false if
    /// the building no longer wants it and it goes home instead.
    pub(crate) fn soldier_arrived(
        &mut self,
        settler: &mut Settler,
        id: usize,
        target: Target,
        health: u32,
    ) {
        let linked = self.target_building(target).is_some()
            && self
                .military
                .get(&target.at)
                .is_some_and(|m| m.soldiers.contains(&id));
        // TRN-FREE-6.3: he enters only standing on the building's field (a
        // road walk that ended short counts as done where he stands); else
        // he goes home. 6.3a: still the building's soldier (counted as
        // coming) until his object goes.
        if linked && settler.walker.at != target.at {
            self.linked_home.insert(settler.number, target);
            self.plan_home_excluding(settler, None);
            return;
        }
        if linked {
            settler.job = SettlerJob::Garrison {
                target,
                health,
                heal: 0,
            };
        } else {
            self.plan_home_excluding(settler, None);
        }
    }

    /// MIL-SOL-1.2: one tick of a present soldier.
    pub(crate) fn heal_soldier(&self, settler: &mut Settler) {
        let Some(rank) = self.soldier_rank(settler.profession) else {
            return;
        };
        let max = self.military_rules.max_health[rank];
        if let SettlerJob::Garrison { health, heal, .. } = &mut settler.job {
            if *heal > HEAL_PASSES {
                *health = (*health + 1).min(max);
                *heal = 0;
            } else {
                *heal += 1;
            }
        }
    }

    /// MIL-SOL-1.4: a walking soldier uncovers the fields around it.
    pub(crate) fn soldier_explores(&mut self, player: PlayerId, at: Point) {
        if !self.explored(player, at) {
            self.explore(player, at, SOLDIER_EXPLORE);
        }
    }

    /// Release a soldier from its building (MIL-SOL-4.5): it is unlinked
    /// and goes home; a headquarters' soldier goes back into the stock.
    /// TRN-FREE-6.3a: the settler `id`, going home after a short walk, is
    /// let go by the military building or site still holding him: it
    /// counts him no more (a site asks for another in its next update).
    pub(crate) fn unlink_home(&mut self, id: usize) {
        let Some(settler) = self.settlers[id].as_ref() else {
            return;
        };
        let leveller = Some(settler.profession) == self.roles.leveller;
        let Some(target) = self.linked_home.remove(&settler.number) else {
            return;
        };
        self.unlink_target(target, id, leveller);
    }

    /// TRN-FREE-6.3a: what [`World::unlink_home`] does to the building or
    /// site, for a settler whose object is already taken out.
    pub(crate) fn unlink_target(&mut self, target: crate::Target, id: usize, leveller: bool) {
        if self.target_building(target).is_none() {
            return;
        }
        if let Some(m) = self.military.get_mut(&target.at) {
            m.soldiers.retain(|&s| s != id);
        } else if let Some(site) = self.sites.get_mut(&target.at) {
            if leveller {
                site.leveller_on_way = false;
            } else {
                site.builder_on_way = false;
            }
        } else {
            // 6.3a Worker (Q-1056): the production building counts him no
            // more; its timer's next firing asks for a worker (ECO-REQ-1.4).
            let index = self.terrain.index(target.at);
            if let Some(b) = self.buildings[index].as_mut() {
                b.workers = b.workers.saturating_sub(1);
            }
        }
    }

    fn release_soldier(&mut self, at: Point, id: usize) {
        if let Some(m) = self.military.get_mut(&at) {
            m.soldiers.retain(|&s| s != id);
        }
        let Some(mut settler) = self.settlers[id].take() else {
            return;
        };
        // MIL-SOL-4.5 (T19): it first takes the one step from the
        // building's field to its flag and is sent home on arriving there
        // (TRN-FREE-6.4a from that tick, the walk home searched from the
        // flag).
        let flag = self.flag_of(at);
        let inside = matches!(settler.job, SettlerJob::Garrison { .. }) && settler.walker.at == at;
        if inside && grid::distance(at, flag) == 1 {
            settler.walker.walk([flag]);
            settler.job = SettlerJob::Leaving {
                step: crate::LeavingStep::StepOut,
                at: self.tick,
                stand: None,
                health: crate::settlers::carried_health(&settler.job),
            };
        } else {
            self.plan_home_excluding(&mut settler, None);
        }
        self.settlers[id] = Some(settler);
    }

    // ---- Serving a soldier request (MIL-SOL-4.7) ----

    /// Serve the soldier request of the building `target`; true if a
    /// soldier was sent.
    pub(crate) fn serve_soldier(&mut self, player: PlayerId, target: Target) -> bool {
        let destination = self.flag_of(target.at);
        let ranks = self.military_rules.soldiers;
        let has_any = |world: &World, at: &Point| {
            ranks
                .iter()
                .flatten()
                .any(|&g| world.warehouses[at].count(g) > 0)
        };
        let candidates: Vec<Point> = self
            .warehouses_of(player)
            .into_iter()
            .filter(|at| has_any(self, at))
            .collect();
        let search = crate::transport::Search::Settler;
        let Some((from, _)) = self.nearest_warehouse(player, candidates, destination, search)
        else {
            return false;
        };
        if self.warehouses[&from].cooldown > 0 {
            return false;
        }
        let strong = self.military_settings(player).prefer_strong;
        let in_stock = |rank: usize| ranks[rank].filter(|&g| self.warehouses[&from].count(g) > 0);
        let rank = if strong {
            (0..SOLDIER_RANKS).rev().find(|&r| in_stock(r).is_some())
        } else {
            (0..SOLDIER_RANKS).find(|&r| in_stock(r).is_some())
        };
        let Some(rank) = rank else { return false };
        let good = ranks[rank].expect("a soldier good");
        let warehouse = self.warehouses.get_mut(&from).expect("a warehouse");
        warehouse.take(good);
        warehouse.cooldown = crate::requests::SEND_COOLDOWN;
        let out = self.flag_of(from);
        let mut walk = vec![out];
        walk.extend(
            self.road_walk(player, out, destination)
                .expect("a warehouse of the destination's network"),
        );
        walk.push(target.at);
        // ECO-WORK-3.8a: out of the warehouse's door field.
        let mut walker = self.walker_leaving(from, walk);
        // TRN-FREE-6.5: a road walk to the building's flag.
        walker.road_to = Some(destination);
        // MIL-SOL-4.7a, ECO-WORK-3.8c: the road walk ends exactly on the
        // building's flag and the step inside starts there at progress 0.
        walker.pauses.push(destination);
        // SIM-START-4.2: a settler made.
        let number = self.new_number();
        let id = self.add_settler(Settler {
            number,
            player,
            profession: good,
            list: crate::SettlerList::Soldiers,
            walker,
            job: SettlerJob::ToMilitary {
                target,
                health: self.military_rules.max_health[rank],
            },
            // MIL-SOL-4.7a: it stands in its creation tick.
            timer: SOLDIER_FIRST_STAND,
            carrying: None,
        });
        self.military
            .get_mut(&target.at)
            .expect("a military building")
            .soldiers
            .push(id);
        true
    }

    /// MIL-SOL-4.2: whether a request is linked to the building `target`:
    /// its soldier requests and, while it is upgraded, the upgrade's leveller
    /// and builder requests (any settler request filed for it).
    fn linked_to(request: &Request, target: Target) -> bool {
        match request.requester {
            Requester::Military(t) | Requester::Site(t) | Requester::Building(t) => t == target,
            _ => false,
        }
    }

    /// MIL-SOL-4.2: R, the open requests of the building, whatever their
    /// profession.
    pub(crate) fn linked_requests(&self, player: PlayerId, target: Target) -> usize {
        self.queue
            .iter()
            .filter(|r| r.player == player && Self::linked_to(r, target))
            .count()
    }

    /// MIL-SOL-4.4 step 1: the building's oldest open request (the queue
    /// keeps filing order) loses its link at once; the queue deletes it on
    /// its turn (ECO-REQ-2.3a), so the cursor stays.
    pub(crate) fn cancel_linked_request(&mut self, player: PlayerId, target: Target) {
        if let Some(request) = self
            .queue
            .iter_mut()
            .find(|r| r.player == player && Self::linked_to(r, target))
        {
            request.requester = Requester::Unlinked;
        }
    }

    // ---- The global military update (ECO-REQ-0.2) ----

    /// MIL-SOL-3 to 5, MIL-PROM-4: recruiting, enemy distances, allocation
    /// and promotion, each on its own counter.
    pub(crate) fn update_military(&mut self) {
        let mut timers = self.military_timers;
        if due(&mut timers.recruit, RECRUIT_TOP) {
            self.recruit();
        }
        if due(&mut timers.distance, DISTANCE_TOP) {
            self.update_enemy_distances();
        }
        if due(&mut timers.allocation, ALLOCATION_TOP) {
            self.allocate_soldiers();
        }
        if due(&mut timers.promotion, PROMOTION_PERIOD_TOP) {
            self.promote();
        }
        self.military_timers = timers;
        // MIL-ATK-6.3: the duels, after the settlers' updates.
        self.fight_rounds();
        self.burn_ruins();
    }

    /// MIL-SOL-5.1: every warehouse makes privates from helper, sword,
    /// shield and beer.
    fn recruit(&mut self) {
        let Some(private) = self.military_rules.soldiers[0] else {
            return;
        };
        let costs = self
            .goods
            .get(&private)
            .map(|g| g.costs.clone())
            .unwrap_or_default();
        if costs.is_empty() {
            return;
        }
        let mut warehouses: Vec<(u32, Point)> = self
            .warehouses
            .keys()
            .filter_map(|&at| Some((self.building_at(at)?.serial, at)))
            .collect();
        warehouses.sort_unstable();
        for (_, at) in warehouses {
            let owner = self.building_at(at).expect("a building").owner;
            let count = self.military_settings(owner).recruit_count;
            let warehouse = self.warehouses.get_mut(&at).expect("a warehouse");
            let mut taken = Vec::new();
            for _ in 0..count {
                if !costs.iter().all(|&(g, n)| warehouse.count(g) >= n) {
                    break;
                }
                for &(g, n) in &costs {
                    for _ in 0..n {
                        if warehouse.take(g) {
                            taken.push(g);
                        }
                    }
                }
                warehouse.add(private, 1);
            }
            // The weapons are used up in the new soldiers.
            for g in taken {
                self.used(g, 1);
            }
        }
    }

    /// UI-SETUP: whether two players are enemies (MIL-SOL-3.1).
    pub fn are_enemies(&self, a: PlayerId, b: PlayerId) -> bool {
        if a == b {
            return false;
        }
        let team = |p: PlayerId| self.teams.get(&p).copied().unwrap_or(-1);
        !(team(a) != -1 && team(a) == team(b))
    }

    /// MIL-SOL-3.1
    fn update_enemy_distances(&mut self) {
        let order = self.military_order();
        // The occupied military buildings: (field, owner).
        let occupied: Vec<(Point, PlayerId)> = order
            .iter()
            .filter(|&&at| self.military[&at].applied)
            .filter(|&&at| {
                let b = self.building_at(at).expect("a building");
                self.building_types[&b.building_type].territory_range > 0
            })
            .map(|&at| (at, self.building_at(at).expect("a building").owner))
            .collect();
        for at in order {
            let building = *self.building_at(at).expect("a building");
            if self.is_headquarters_type(&building) {
                continue;
            }
            let distance = occupied
                .iter()
                .filter(|&&(_, owner)| self.are_enemies(building.owner, owner))
                .map(|&(field, _)| grid::distance(at, field) as i32)
                .min()
                .unwrap_or(NO_ENEMY);
            let m = self.military.get_mut(&at).expect("military");
            if m.enemy_distance < 0 {
                // MIL-PROM-3.3: the first update starts the accumulator.
                m.accumulator = 0.0f32.to_bits();
            }
            m.enemy_distance = distance;
        }
    }

    /// MIL-SOL-3.2: soldiers a building wants.
    pub fn wanted_soldiers(&self, at: Point) -> u32 {
        let (Some(building), Some(m)) = (self.building_at(at), self.military.get(&at)) else {
            return 0;
        };
        let kind = &self.building_types[&building.building_type];
        if m.move_out {
            return u32::from(!m.applied);
        }
        let settings = self.military_settings(building.owner);
        let n = m.enemy_distance;
        let class = if n > FAR_ABOVE {
            2
        } else if n >= MEDIUM_FROM {
            1
        } else {
            0
        };
        let f = settings.allocation(class);
        let wanted = (f * kind.soldier_limit as f32 + 0.5).trunc() as i64;
        wanted.max(1) as u32
    }

    /// MIL-SOL-4: every building asks for one more soldier or lets one go.
    fn allocate_soldiers(&mut self) {
        let Some(placeholder) = self.military_rules.soldiers[0] else {
            return;
        };
        for at in self.military_order() {
            let building = *self.building_at(at).expect("a building");
            if building.under_construction || self.is_headquarters_type(&building) {
                continue;
            }
            let kind = &self.building_types[&building.building_type];
            if kind.soldier_limit == 0 {
                continue;
            }
            let target = self.target_of(at).expect("a building");
            let player = building.owner;
            let wanted = i64::from(self.wanted_soldiers(at));
            let soldiers = self.military[&at].soldiers.len() as i64;
            let mut open = self.linked_requests(player, target) as i64;
            let mut d = wanted - soldiers - open;
            if d >= 1 {
                // 4.3: one request, whatever d is. A building can have several
                // open at once (R of 4.2), so the one-per-requester rule of
                // `request` does not apply.
                self.queue.push(Request {
                    player,
                    requester: Requester::Military(target),
                    profession: placeholder,
                });
                // SIM-START-4.2: a request filed takes a number.
                self.new_number();
                continue;
            }
            // 4.4
            while d < 0 && open > 0 {
                self.cancel_linked_request(player, target);
                open -= 1;
                d += 1;
            }
            if d < 0 {
                // Not present first: walking to it, or going home after a
                // short walk still linked (TRN-FREE-6.3a), in list order.
                let walking: Vec<(usize, bool)> = {
                    let m = &self.military[&at];
                    // Soldiers out fighting stay (MIL-ATK).
                    m.soldiers
                        .iter()
                        .copied()
                        .filter_map(|id| {
                            let s = self.settlers[id].as_ref()?;
                            let home = self.linked_home.get(&s.number) == Some(&target);
                            (home || matches!(s.job, SettlerJob::ToMilitary { .. }))
                                .then_some((id, home))
                        })
                        .collect()
                };
                for (id, home) in walking {
                    if d >= 0 {
                        break;
                    }
                    if home {
                        // 6.3a: unlinked; he walks on home.
                        self.unlink_home(id);
                    } else {
                        self.release_soldier(at, id);
                    }
                    d += 1;
                }
            }
            if d < 0 && self.military[&at].cooldown == 0 {
                let present = self.present_ranks(at);
                let strong = self.military_settings(player).prefer_strong;
                // 4.5: with "prefer strong" the lowest rank leaves, first in
                // list order on equal rank; else the highest.
                let pick = if strong {
                    present.iter().min_by_key(|&&(_, r)| r)
                } else {
                    let best = present.iter().map(|&(_, r)| r).max();
                    present.iter().find(|&&(_, r)| Some(r) == best)
                };
                if let Some(&(id, _)) = pick {
                    self.release_soldier(at, id);
                    self.military.get_mut(&at).expect("military").cooldown = RELEASE_COOLDOWN;
                }
            }
        }
    }

    /// MIL-PROM-3.3: which coin order of `group` (order indices of one
    /// network, in list order) is served.
    pub(crate) fn pick_coin_order(&mut self, group: &[usize]) -> Option<usize> {
        let mut candidates: Vec<(Point, usize)> = Vec::new();
        for &i in group {
            let at = self.orders[i].target.at;
            if !self.can_use_coins(at) {
                continue;
            }
            if !candidates.iter().any(|&(a, _)| a == at) {
                candidates.push((at, i));
            }
        }
        let mut best: Option<(f32, Point, usize)> = None;
        for (at, first) in candidates {
            let Some(m) = self.military.get_mut(&at) else {
                continue;
            };
            let mut value = m.accumulator();
            if m.enemy_distance > 0 {
                // ECO-TIME-5.1: every step is rounded to float32, the
                // reciprocal first and then the sum (3 x 1/30 > 1/10).
                let share = 1.0f32 / m.enemy_distance as f32;
                value += share;
            }
            m.accumulator = value.to_bits();
            if best.is_none_or(|(v, _, _)| value > v) {
                best = Some((value, at, first));
            }
        }
        let (_, at, index) = best?;
        self.military.get_mut(&at).expect("military").accumulator = 0.0f32.to_bits();
        Some(index)
    }

    /// MIL-PROM-4.2: the promotion run.
    fn promote(&mut self) {
        let Some(coin) = self.roles.coin else { return };
        for at in self.military_order() {
            if !self.can_use_coins(at) || self.coins_in_store(at) == 0 {
                continue;
            }
            let mut promoted = [false; SOLDIER_RANKS];
            for (id, rank) in self.present_ranks(at) {
                if rank + 1 >= SOLDIER_RANKS || promoted[rank] {
                    continue;
                }
                let Some(next) = self.military_rules.soldiers[rank + 1] else {
                    continue;
                };
                promoted[rank] = true;
                // MIL-PROM-4.3. Health is kept (Q-060).
                self.settlers[id].as_mut().expect("a soldier").profession = next;
            }
            if let Some(list) = self.inputs.get_mut(&at) {
                if let Some(entry) = list.iter_mut().find(|(g, n)| *g == coin && *n > 0) {
                    entry.1 -= 1;
                    self.used(coin, 1);
                }
            }
        }
    }

    // ---- Commands (MIL-SOL-7) ----

    pub(crate) fn set_allocation(
        &mut self,
        player: PlayerId,
        class: u8,
        thousandths: u32,
    ) -> Result<(), Rejection> {
        if class > 2 || thousandths > MAX_FACTOR {
            return Err(Rejection::NoSuchSetting);
        }
        self.military_settings_mut(player).allocation[usize::from(class)] =
            (thousandths as f32 / 1000.0).to_bits();
        Ok(())
    }

    pub(crate) fn set_attack_strength(
        &mut self,
        player: PlayerId,
        thousandths: u32,
    ) -> Result<(), Rejection> {
        if thousandths > MAX_FACTOR {
            return Err(Rejection::NoSuchSetting);
        }
        self.military_settings_mut(player).attack = (thousandths as f32 / 1000.0).to_bits();
        Ok(())
    }

    pub(crate) fn set_defenders(
        &mut self,
        player: PlayerId,
        thousandths: u32,
    ) -> Result<(), Rejection> {
        if thousandths > MAX_FACTOR {
            return Err(Rejection::NoSuchSetting);
        }
        self.military_settings_mut(player).defenders = (thousandths as f32 / 1000.0).to_bits();
        Ok(())
    }

    fn own_military(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<&mut MilitaryBuilding, Rejection> {
        let owned = self.building_at(at).is_some_and(|b| b.owner == player);
        if !owned {
            return Err(Rejection::NoBuilding);
        }
        self.military.get_mut(&at).ok_or(Rejection::NotMilitary)
    }

    pub(crate) fn toggle_coin_supply(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<(), Rejection> {
        let m = self.own_military(player, at)?;
        m.coin_switch = !m.coin_switch;
        Ok(())
    }

    pub(crate) fn toggle_move_out(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        let m = self.own_military(player, at)?;
        m.move_out = !m.move_out;
        Ok(())
    }

    pub(crate) fn set_castle_soldiers(
        &mut self,
        player: PlayerId,
        at: Point,
        rank: u8,
        amount: u32,
    ) -> Result<(), Rejection> {
        if usize::from(rank) >= SOLDIER_RANKS {
            return Err(Rejection::NoSuchSetting);
        }
        let headquarters = self
            .building_at(at)
            .is_some_and(|b| self.is_headquarters_type(b));
        let m = self.own_military(player, at)?;
        if !headquarters {
            return Err(Rejection::NotMilitary);
        }
        m.castle[usize::from(rank)] = amount;
        Ok(())
    }

    /// A soldier request names a building: wanted while it stands
    /// (ECO-REQ-2.3).
    pub(crate) fn soldier_request_wanted(&self, request: &Request) -> bool {
        match request.requester {
            Requester::Military(target) => self
                .target_building(target)
                .is_some_and(|b| !b.under_construction && self.military.contains_key(&target.at)),
            _ => false,
        }
    }
}
// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
crate::snap_struct!(Timers {
    distance,
    allocation,
    recruit,
    promotion
});

crate::snap_struct!(MilitarySettings {
    allocation,
    attack,
    defenders,
    prefer_strong,
    recruit_count
});
