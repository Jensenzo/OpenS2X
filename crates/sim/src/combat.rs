//! Attacks and combat (`specs/MIL-attack-combat.md`): the attack order and
//! the soldiers it sends, interceptors, defenders coming out of the target,
//! and the duel. What happens when the target falls is `conquest.rs`.

use std::collections::VecDeque;

use crate::grid::{self, Point};
use crate::military::RELEASE_COOLDOWN;
use crate::movement::{self, NOT_WALKABLE};
use crate::trace::Action;
use crate::{PlayerId, Rejection, Settler, SettlerJob, Target, World};

/// MIL-ATK-1.1: sending buildings and interceptors are closer than this.
const REACH: u32 = 24;
/// MIL-ATK-3.3: the meeting search stays closer than this to the building.
const MEETING_REACH: u32 = 20;
/// MIL-ATK-6.1, UI-GAME-11.1: the newest attack message is refreshed, not
/// repeated, while its timer still has at least 540 of its 600 units
/// (at most 60 units, 3600 ticks, old; Q-222).
const MESSAGE_REFRESH_TIMER: u64 = 540;
/// MIL-ATK-4.1: a soldier switched to an errand in tick c leaves in tick
/// c + 1; its own timer runs down once in tick c already.
const LEAVE_DELAY: u32 = 2;
/// MIL-ATK-1.4: the fractions of the dialog's four count buttons.
pub const COUNT_FRACTIONS: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

pub(crate) const UNDER_ATTACK: &str = "You are under attack!";

/// Where an attacker is in its errand.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackStep {
    /// Still on its building's door field, waiting for the release
    /// cooldown (MIL-ATK-4.1).
    Leaving,
    /// Walking to the meeting field, or, without one, to the target's door
    /// (MIL-ATK-4.2).
    Walking,
    /// Standing on its field, checking each tick (MIL-ATK-4.4).
    Waiting,
    /// Walking back to the building it came from.
    Home,
}

/// Where a soldier sent against an attacker is.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefendStep {
    Leaving,
    /// Walking to the attacker's field.
    Out,
    /// Standing on it.
    Waiting,
    /// Walking back to its building's door.
    Back,
}

/// MIL-ATK-2.3: the attacker record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackRecord {
    /// The attacking soldier (settler id).
    pub soldier: usize,
    pub target: Target,
    /// MIL-ATK-6.2: the round counter; even, the attacker strikes.
    pub counter: u32,
    /// The meeting field (MIL-ATK-3.3) or waiting field (MIL-ATK-4.3).
    pub field: Option<Point>,
    /// The defender or interceptor it is paired with (settler id).
    pub opponent: Option<usize>,
    /// MIL-ATK-6.3: the last round scored a hit.
    pub hit: bool,
    /// MIL-ATK-6.1a: his step off the waiting field's lattice point.
    pub offset: Option<OffsetStep>,
}

/// MIL-ATK-6.1a: an attacker's step from his waiting field's lattice point
/// one lattice step in a drawn direction, or back to the point before a new
/// one.
///
/// Internal state of the simulation (`docs/api.md`); this type may change
/// in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetStep {
    /// The lattice walk, samples `from` to `to`.
    pub walk: movement::LatticeWalk,
    /// Walking back to the field's point (his opponent gone, Q-1050 b).
    pub back: bool,
}

/// MIL-ATK-1.4: the number the dialog puts into the order for the fraction
/// `s` of `available` attackers: `trunc(s x (A - 1) + 1.5)`.
pub fn dialog_count(available: u32, fraction: f32) -> u32 {
    (fraction * (available as f32 - 1.0) + 1.5) as u32
}

fn health_of(settler: &Settler) -> Option<u32> {
    match settler.job {
        SettlerJob::Attack { health, .. }
        | SettlerJob::Defend { health, .. }
        | SettlerJob::Garrison { health, .. }
        | SettlerJob::ToMilitary { health, .. } => Some(health),
        _ => None,
    }
}

fn health_mut(settler: &mut Settler) -> Option<&mut u32> {
    match &mut settler.job {
        SettlerJob::Attack { health, .. } | SettlerJob::Defend { health, .. } => Some(health),
        _ => None,
    }
}

/// A duel in progress: its field and the rank and health of both soldiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duel {
    pub field: Point,
    pub attacker: (usize, u32),
    pub defender: (usize, u32),
}

impl World {
    /// The duels in progress: pairs standing on their field (MIL-ATK-6.1).
    pub fn duels(&self) -> Vec<Duel> {
        let fighter = |id: usize| {
            let s = self.settlers.get(id)?.as_ref()?;
            let health = health_of(s)?;
            Some((self.soldier_rank(s.profession)?, health, s.walker.at))
        };
        self.attackers
            .iter()
            .filter_map(|r| {
                let (a, d) = (fighter(r.soldier)?, fighter(r.opponent?)?);
                (r.field == Some(a.2)
                    && a.2 == d.2
                    && self.offset_arrived(r)
                    && !self.settlers_walking(r.soldier, r.opponent?))
                .then_some(Duel {
                    field: a.2,
                    attacker: (a.0, a.1),
                    defender: (d.0, d.1),
                })
            })
            .collect()
    }

    fn settlers_walking(&self, a: usize, b: usize) -> bool {
        [a, b].iter().any(|&id| {
            self.settlers
                .get(id)
                .and_then(|s| s.as_ref())
                .is_none_or(|s| s.walker.is_walking())
        })
    }

    /// MIL-ATK-6.1a: the attacker of `record` has made his step off the
    /// waiting field's point for his opponent and it has ended.
    fn offset_arrived(&self, record: &AttackRecord) -> bool {
        record
            .offset
            .is_some_and(|o| !o.back && o.walk.is_done(&self.terrain))
    }

    /// MIL-ATK-6.1a: a new step for the attacker of `record` on his waiting
    /// field `field`: one draw `d = draw mod 6`, one lattice step from the
    /// field's lattice point in lattice direction `d` (TRN-FREE-5.4's
    /// offset; off the map: no step). True when he has arrived at once.
    fn offset_start(&mut self, record: usize, field: Point) -> bool {
        let speed = self.settings.settler_speed;
        let sample = grid::field_sample(field);
        let d = (self.rng.draw() % 6) as usize;
        let (columns, rows) = self.terrain.sample_size();
        let to = grid::neighbour(sample, grid::Direction::ALL[d], columns, rows).unwrap_or(sample);
        let walk = movement::LatticeWalk::new(&self.terrain, speed, sample, to);
        let arrived = walk.is_done(&self.terrain);
        self.attackers[record].offset = Some(OffsetStep { walk, back: false });
        arrived
    }

    /// MIL-ATK-6.1a (Q-1050 b): the attacker of `record` leaves his offset
    /// point: the walk back to the field's point (12 ticks on level
    /// ground). False when he stands on the point (no walk).
    fn offset_back(&mut self, record: usize, field: Point) -> bool {
        let speed = self.settings.settler_speed;
        let sample = grid::field_sample(field);
        let Some(o) = self.attackers[record].offset else {
            return false;
        };
        let (at, ..) = o.walk.position(&self.terrain);
        if at == sample {
            self.attackers[record].offset = None;
            return false;
        }
        self.attackers[record].offset = Some(OffsetStep {
            walk: movement::LatticeWalk::new(&self.terrain, speed, at, sample),
            back: true,
        });
        true
    }

    /// MIL-ATK-6.1a: one tick of a walk on the offset lattice step; true in
    /// the tick it ends.
    fn offset_advance(&mut self, record: usize) -> bool {
        let speed = self.settings.settler_speed;
        let Some(mut o) = self.attackers[record].offset else {
            return false;
        };
        if o.walk.is_done(&self.terrain) {
            return false;
        }
        o.walk.advance(&self.terrain, speed);
        self.attackers[record].offset = Some(o);
        o.walk.is_done(&self.terrain)
    }

    /// The attacker records, in sending order.
    pub fn attack_records(&self) -> &[AttackRecord] {
        &self.attackers
    }

    /// MIL-SOL-2.4: (settler id, rank, health) of the idle present
    /// soldiers of the building on `at`, in list order.
    fn idle_soldiers(&self, at: Point) -> Vec<(usize, usize, u32)> {
        let Some(m) = self.military.get(&at) else {
            return Vec::new();
        };
        m.soldiers
            .iter()
            .filter_map(|&id| {
                let s = self.settlers.get(id)?.as_ref()?;
                let SettlerJob::Garrison { health, .. } = s.job else {
                    return None;
                };
                Some((id, self.soldier_rank(s.profession)?, health))
            })
            .collect()
    }

    /// MIL-ATK-3: soldiers of the building on `at` out as interceptors.
    pub(crate) fn interceptors_out(&self, at: Point) -> usize {
        self.settlers()
            .filter(|(_, s)| {
                matches!(s.job, SettlerJob::Defend { home, intercept: true, .. } if home.at == at)
            })
            .count()
    }

    // ---- Availability (MIL-ATK-1) ----

    /// MIL-ATK-1.1: the soldiers the building on `from` can send against the
    /// target on `target`.
    fn sendable(&self, player: PlayerId, from: Point, target: Point) -> u32 {
        let Some(building) = self.building_at(from) else {
            return 0;
        };
        let kind = &self.building_types[&building.building_type];
        if building.owner != player
            || building.under_construction
            || kind.territory_range == 0
            || kind.warehouse
            || !self.military.contains_key(&from)
        {
            return 0;
        }
        let d = grid::distance(from, target);
        if d >= REACH {
            return 0;
        }
        let n = self.idle_soldiers(from).len();
        if n == 0 {
            return 0;
        }
        let k = ((n as f32 - 1.0) * self.military_settings(player).attack_strength()) as u32;
        if k == 0 || self.interceptors_out(from) > 0 {
            return 0;
        }
        // Conditions 4 and 6: a free path from the target to the building,
        // shorter than 2 x d (which also puts them on one land region).
        match movement::free_walk_steps(&self.terrain, target, from) {
            Some(steps) if (steps as u32) < 2 * d => k,
            _ => 0,
        }
    }

    /// MIL-ATK-1.2: the attackers `player` can send against the building on
    /// `target`.
    pub fn attack_available(&self, player: PlayerId, target: Point) -> u32 {
        let Some(building) = self.building_at(target) else {
            return 0;
        };
        if building.under_construction || !self.military.contains_key(&target) {
            return 0;
        }
        let range = self.building_types[&building.building_type].territory_range;
        if range == 0 {
            return 0;
        }
        let (width, height) = (self.terrain.width, self.terrain.height);
        let ring = grid::ring(target, range + 1, width, height);
        if !ring.iter().any(|&f| self.field_owner(f) == Some(player)) {
            return 0;
        }
        self.military_order()
            .into_iter()
            .map(|at| self.sendable(player, at, target))
            .sum()
    }

    /// MIL-ATK-1.4: the four counts of the dialog's buttons.
    pub fn attack_counts(&self, player: PlayerId, target: Point) -> [u32; 4] {
        let available = self.attack_available(player, target);
        COUNT_FRACTIONS.map(|s| dialog_count(available, s))
    }

    // ---- The order (MIL-ATK-1.5, 2) ----

    pub(crate) fn attack(
        &mut self,
        player: PlayerId,
        target_at: Point,
        count: u32,
        strong_first: bool,
    ) -> Result<(), Rejection> {
        let building = *self.building_at(target_at).ok_or(Rejection::NoBuilding)?;
        if !self.are_enemies(player, building.owner) {
            return Err(Rejection::NotAnEnemy);
        }
        let available = self.attack_available(player, target_at);
        if available == 0 {
            return Ok(());
        }
        let target = self.target_of(target_at).expect("a building");
        let mut left = count.min(available);
        // 2.1: by distance, within one distance in list order.
        let mut levels: Vec<(u32, Point)> = self
            .military_order()
            .into_iter()
            .filter(|&at| self.building_at(at).is_some_and(|b| b.owner == player))
            .map(|at| (grid::distance(at, target_at), at))
            .filter(|&(d, _)| d < REACH)
            .collect();
        levels.sort_by_key(|&(d, _)| d);
        for (_, at) in levels {
            if left == 0 {
                break;
            }
            let share = self.sendable(player, at, target_at).min(left);
            if share > 0 {
                left -= self.send_attackers(at, target, share as usize, strong_first);
            }
        }
        self.intercept(target);
        Ok(())
    }

    /// MIL-ATK-2.2, 2.3: `share` soldiers of the building on `at` go.
    fn send_attackers(&mut self, at: Point, target: Target, share: usize, strong: bool) -> u32 {
        let mut idle = self.idle_soldiers(at);
        let ranks: Vec<usize> = if strong {
            (0..crate::SOLDIER_RANKS).rev().collect()
        } else {
            (0..crate::SOLDIER_RANKS).collect()
        };
        let mut chosen: Vec<usize> = Vec::new();
        'ranks: for rank in ranks {
            while chosen.len() < share {
                // The highest health of this rank, the first on a tie.
                let mut best: Option<usize> = None;
                for (i, &(_, r, health)) in idle.iter().enumerate() {
                    if r == rank && best.is_none_or(|b| health > idle[b].2) {
                        best = Some(i);
                    }
                }
                let Some(i) = best else { continue 'ranks };
                chosen.push(idle.remove(i).0);
            }
        }
        let home = self.target_of(at).expect("a building");
        for &id in &chosen {
            self.military
                .get_mut(&at)
                .expect("military")
                .soldiers
                .retain(|&s| s != id);
            let settler = self.settlers[id].as_mut().expect("a soldier");
            let health = health_of(settler).expect("a garrison soldier");
            settler.job = SettlerJob::Attack {
                home,
                health,
                step: AttackStep::Leaving,
            };
            settler.timer = LEAVE_DELAY;
            let counter = (u64::from(self.rng.draw()) % 2) as u32;
            self.attackers.push(AttackRecord {
                soldier: id,
                target,
                counter,
                field: None,
                opponent: None,
                hit: false,
                offset: None,
            });
        }
        chosen.len() as u32
    }

    // ---- Interceptors (MIL-ATK-3) ----

    /// MIL-ATK-5.2: the idle soldier of the building on `at` that comes
    /// out: with `strong` the highest rank, else the lowest, on equal rank
    /// the higher health, then the first in the list.
    fn pick_defender(&self, at: Point, strong: bool) -> Option<usize> {
        let mut best: Option<(usize, usize, u32)> = None;
        for c in self.idle_soldiers(at) {
            let better = match best {
                None => true,
                Some(b) if strong => (c.1, c.2) > (b.1, b.2),
                Some(b) => c.1 < b.1 || (c.1 == b.1 && c.2 > b.2),
            };
            if better {
                best = Some(c);
            }
        }
        best.map(|(id, ..)| id)
    }

    /// MIL-ATK-3.1, 3.2
    fn intercept(&mut self, target: Target) {
        let Some(owner) = self.target_building(target).map(|b| b.owner) else {
            return;
        };
        let outside = self.military_settings(owner).defenders();
        let strong = self.military_settings(owner).prefer_strong;
        for at in self.military_order() {
            let Some(b) = self.building_at(at) else {
                continue;
            };
            let kind = &self.building_types[&b.building_type];
            if b.owner != owner
                || b.under_construction
                || kind.warehouse
                || kind.territory_range == 0
            {
                continue;
            }
            let e = grid::distance(at, target.at);
            if e >= REACH {
                continue;
            }
            if at != target.at {
                match movement::free_walk_steps(&self.terrain, target.at, at) {
                    Some(steps) if (steps as u32) < 2 * e => {}
                    _ => continue,
                }
            }
            let present = self.idle_soldiers(at).len();
            let j = ((present as f32 - 1.0) * outside).max(0.0) as usize;
            let out = self.interceptors_out(at);
            for _ in out..j {
                // 3.2: the first attacker still at home without a field.
                let Some(record) = self.attackers.iter().position(|r| {
                    r.target == target
                        && r.field.is_none()
                        && self.settlers[r.soldier].as_ref().is_some_and(|s| {
                            matches!(
                                s.job,
                                SettlerJob::Attack {
                                    step: AttackStep::Leaving,
                                    ..
                                }
                            )
                        })
                }) else {
                    return;
                };
                let Some(id) = self.pick_defender(at, strong) else {
                    break;
                };
                let Some(field) = self.meeting_field(at, target.at) else {
                    break;
                };
                let home = self.target_of(at).expect("a building");
                self.pair(record, id, home, true, Some(field));
            }
        }
    }

    /// MIL-ATK-3.3: the meeting field for an interceptor of the building on
    /// `from` and the target on `target`.
    fn meeting_field(&self, from: Point, target: Point) -> Option<Point> {
        let mut seen = vec![false; self.terrain.len()];
        seen[self.terrain.index(from)] = true;
        let mut queue = VecDeque::from([from]);
        let mut best: Option<(u32, Point)> = None;
        while let Some(f) = queue.pop_front() {
            let d = grid::distance(f, target);
            if d > 0 && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, f));
            }
            for q in self.terrain.neighbours(f).into_iter().flatten() {
                let index = self.terrain.index(q);
                if !seen[index]
                    && grid::distance(from, q) < MEETING_REACH
                    && !self.terrain.has(q, NOT_WALKABLE)
                {
                    seen[index] = true;
                    queue.push_back(q);
                }
            }
        }
        self.free_field(best?.1, None)
    }

    /// MIL-ATK-4.3: the first free field of a breadth-first search from
    /// `start` over walkable fields, the start itself first. `own` is the
    /// soldier looking, whose own field does not count as taken.
    pub(crate) fn free_field(&self, start: Point, own: Option<usize>) -> Option<Point> {
        let mut seen = vec![false; self.terrain.len()];
        seen[self.terrain.index(start)] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(f) = queue.pop_front() {
            let index = self.terrain.index(f);
            let taken = self
                .attackers
                .iter()
                .any(|r| r.field == Some(f) && Some(r.soldier) != own);
            if !taken && self.streets[index].is_none() && self.flags[index].is_none() {
                return Some(f);
            }
            for q in self.terrain.neighbours(f).into_iter().flatten() {
                let i = self.terrain.index(q);
                if !seen[i] && !self.terrain.has(q, NOT_WALKABLE) {
                    seen[i] = true;
                    queue.push_back(q);
                }
            }
        }
        None
    }

    /// Pair the attacker `record` with the soldier `defender`, who leaves
    /// the building `home` to meet it on `field` (or on the attacker's field
    /// when none is given).
    fn pair(
        &mut self,
        record: usize,
        defender: usize,
        home: Target,
        intercept: bool,
        field: Option<Point>,
    ) {
        let attacker = self.attackers[record].soldier;
        let defends = intercept.then_some(self.attackers[record].target);
        if field.is_some() {
            self.attackers[record].field = field;
        }
        self.attackers[record].opponent = Some(defender);
        let settler = self.settlers[defender].as_mut().expect("a defender");
        let health = health_of(settler).expect("a soldier");
        settler.job = SettlerJob::Defend {
            home,
            health,
            step: DefendStep::Leaving,
            opponent: Some(attacker),
            intercept,
            defends,
        };
        settler.timer = LEAVE_DELAY;
    }

    /// MIL-ATK-5.1, 3.4: the first unpaired interceptor of the building
    /// `target` itself, waiting outside (defense wait), in settler order.
    fn waiting_interceptor(&self, target: Target) -> Option<usize> {
        self.settlers()
            .find(|(_, s)| {
                matches!(
                    s.job,
                    SettlerJob::Defend {
                        home,
                        intercept: true,
                        opponent: None,
                        step: DefendStep::Waiting,
                        ..
                    } if home == target
                ) && !s.walker.is_walking()
            })
            .map(|(id, _)| id)
    }

    /// MIL-ATK-3.4, 5.1, 6.1a: the waiting interceptor `id` is paired with
    /// the attacker of `record`: he walks from where he stands to that
    /// attacker's waiting field and fights there.
    fn pair_waiting_interceptor(&mut self, record: usize, id: usize) {
        let attacker = self.attackers[record].soldier;
        self.attackers[record].opponent = Some(id);
        let Some(field) = self.attackers[record].field else {
            return;
        };
        let mut settler = self.settlers[id].take().expect("an interceptor");
        if let SettlerJob::Defend { step, opponent, .. } = &mut settler.job {
            *step = DefendStep::Out;
            *opponent = Some(attacker);
        }
        if settler.walker.at != field {
            self.walk_to(&mut settler, field);
        }
        self.settlers[id] = Some(settler);
    }

    // ---- The soldiers' own updates ----

    /// Whether the soldier `defender` is paired with the attacker `attacker`.
    fn defender_pairs_with(&self, defender: usize, attacker: usize) -> bool {
        self.settlers.get(defender).and_then(|s| s.as_ref()).is_some_and(|s| {
            matches!(s.job, SettlerJob::Defend { opponent, .. } if opponent == Some(attacker))
        })
    }

    /// MIL-ATK-4.1: the home building lets one soldier out per 40 ticks.
    fn leave_building(&mut self, home: Target) -> bool {
        if self.target_building(home).is_none() {
            return true;
        }
        match self.military.get_mut(&home.at) {
            Some(m) if m.cooldown > 0 => false,
            Some(m) => {
                m.cooldown = RELEASE_COOLDOWN;
                true
            }
            None => true,
        }
    }

    /// MIL-ATK-6.1, UI-GAME-11.1 (T36a): "You are under attack!" at the
    /// target, unless the newest one within one field still has a timer
    /// of at least 540 units: that one is reset to 600 instead.
    pub(crate) fn under_attack(&mut self, target: Target) {
        use crate::requests::{MESSAGE_TICKS, TICKS_PER_TIMER_UNIT};
        let Some(owner) = self.target_building(target).map(|b| b.owner) else {
            return;
        };
        let now = self.tick;
        let newest = self.messages.iter_mut().rev().find(|m| {
            m.player == owner
                && m.text == UNDER_ATTACK
                && crate::grid::distance(m.at, target.at) <= 1
        });
        let young = newest.filter(|m| {
            m.until.saturating_sub(now) >= MESSAGE_REFRESH_TIMER * TICKS_PER_TIMER_UNIT
        });
        match young {
            Some(message) => message.until = now + MESSAGE_TICKS,
            None => self.messages.push(crate::Message {
                tick: now,
                player: owner,
                at: target.at,
                text: UNDER_ATTACK,
                until: now + MESSAGE_TICKS,
                building: self.building_at(target.at).map(|b| b.building_type),
                ship: None,
            }),
        }
    }

    /// The soldier `settler` walks to `to`; false if no walk starts.
    fn walk_to(&mut self, settler: &mut Settler, to: Point) -> bool {
        match self.free_walk(settler.walker.at, to, 0) {
            Some(walk) => {
                settler.walker.walk(walk);
                true
            }
            None => false,
        }
    }

    /// A soldier is back at the door of the building it came from: it
    /// joins it again, or goes home to a warehouse when there is none.
    fn rejoin(&mut self, settler: &mut Settler, number: usize, home: Target, health: u32) {
        let own = self
            .target_building(home)
            .is_some_and(|b| b.owner == settler.player && !b.under_construction)
            && self.military.contains_key(&home.at);
        if own {
            settler.job = SettlerJob::Garrison {
                target: home,
                health,
                heal: 0,
            };
            let m = self.military.get_mut(&home.at).expect("military");
            if !m.soldiers.contains(&number) {
                m.soldiers.push(number);
            }
        } else {
            self.plan_home_excluding(settler, None);
        }
    }

    /// MIL-ATK-4.4: the attacker ends its errand and walks home.
    pub(crate) fn attacker_goes_home(
        &mut self,
        settler: &mut Settler,
        number: usize,
        home: Target,
        health: u32,
    ) {
        self.attackers.retain(|r| r.soldier != number);
        let at_door = settler.walker.at == home.at && !settler.walker.is_walking();
        if at_door {
            self.rejoin(settler, number, home, health);
            return;
        }
        if self.target_building(home).is_some() && self.walk_to(settler, home.at) {
            settler.job = SettlerJob::Attack {
                home,
                health,
                step: AttackStep::Home,
            };
        } else {
            self.plan_home_excluding(settler, None);
        }
    }

    /// The attacker's job logic; false when it leaves the world.
    pub(crate) fn attack_job(
        &mut self,
        settler: &mut Settler,
        number: usize,
        home: Target,
        health: u32,
        step: AttackStep,
    ) -> bool {
        // MIL-ATK-6.6: a soldier at 0 dies in its own update.
        if health == 0 {
            self.attackers.retain(|r| r.soldier != number);
            return false;
        }
        let Some(record) = self.attackers.iter().position(|r| r.soldier == number) else {
            // Its errand is over (the record is gone): back home.
            if step != AttackStep::Home || !settler.walker.is_walking() {
                self.attacker_goes_home(settler, number, home, health);
            }
            return true;
        };
        // A defender that fell or left the pair is no longer its opponent.
        if let Some(o) = self.attackers[record].opponent {
            if !self.defender_pairs_with(o, number) {
                self.attackers[record].opponent = None;
            }
        }
        let target = self.attackers[record].target;
        let arrived = !settler.walker.is_walking();
        let set = |step| SettlerJob::Attack { home, health, step };
        match step {
            AttackStep::Leaving => {
                if settler.timer > 0 || !self.leave_building(home) {
                    return true;
                }
                if self.target_building(target).is_none() {
                    self.attacker_goes_home(settler, number, home, health);
                    return true;
                }
                // MIL-ATK-4.2 (Q-1051): to his meeting field if an
                // interception gave him one, else to the target's door, its
                // flag.
                let to = self.attackers[record]
                    .field
                    .unwrap_or_else(|| self.attack_door(target));
                if self.walk_to(settler, to) {
                    settler.job = set(AttackStep::Walking);
                }
            }
            AttackStep::Walking => {
                if !arrived {
                    return true;
                }
                if self.target_building(target).is_none() {
                    self.attacker_goes_home(settler, number, home, health);
                    return true;
                }
                match self.attackers[record].field {
                    Some(f) if settler.walker.at == f => {
                        // MIL-ATK-6.1a: paired, he begins his wait with his
                        // offset step and arrives (6.1) when it ends;
                        // unpaired he stands on the point (Q-1050 a).
                        settler.job = set(AttackStep::Waiting);
                        if self.attackers[record].opponent.is_some() && self.offset_start(record, f)
                        {
                            self.under_attack(target);
                        }
                    }
                    Some(f) => {
                        self.walk_to(settler, f);
                    }
                    None => self.attacker_at_door(settler, number, record, home, health),
                }
            }
            AttackStep::Waiting => {
                let field = self.attackers[record].field;
                // MIL-ATK-6.1a (Q-1050 b): off the point without an opponent
                // (or with his target gone) he walks back to it first and
                // chooses nothing until that step has ended.
                if let (Some(f), Some(o)) = (field, self.attackers[record].offset) {
                    let leaving = self.attackers[record].opponent.is_none()
                        || self.target_building(target).is_none();
                    if o.back {
                        if !self.offset_advance(record) {
                            return true;
                        }
                        self.attackers[record].offset = None;
                    } else if leaving && self.offset_back(record, f) {
                        return true;
                    }
                }
                // MIL-ATK-4.4, in this order.
                if self.target_building(target).is_none() {
                    self.attackers[record].offset = None;
                    self.attacker_goes_home(settler, number, home, health);
                    return true;
                } else if self.attackers[record].opponent.is_none() {
                    if self.can_take(target, settler.player) {
                        self.attackers[record].field = None;
                        self.attackers[record].offset = None;
                        let door = self.attack_door(target);
                        if self.walk_to(settler, door) {
                            settler.job = set(AttackStep::Walking);
                        }
                    } else {
                        // Paired here, his wait job starts: the draw and
                        // the step in its next update (Q-1050 a).
                        self.find_opponent(record, settler.player);
                    }
                    return true;
                }
                // MIL-ATK-6.1a: paired, his step off the point, or its walk.
                if let Some(f) = field {
                    let arrived = match self.attackers[record].offset {
                        None => self.offset_start(record, f),
                        Some(_) => self.offset_advance(record),
                    };
                    if arrived {
                        self.under_attack(target);
                    }
                }
            }
            AttackStep::Home => {
                if arrived {
                    self.attackers.retain(|r| r.soldier != number);
                    self.rejoin(settler, number, home, health);
                }
            }
        }
        true
    }

    /// MIL-ATK-4.2 (Q-1051): the door an attacker walks to: the field of
    /// the target's flag (its building's field when it has none).
    fn attack_door(&self, target: Target) -> Point {
        self.target_flag(target).unwrap_or(target.at)
    }

    /// MIL-ATK-4.2: the attacker has reached the target's door.
    fn attacker_at_door(
        &mut self,
        settler: &mut Settler,
        number: usize,
        record: usize,
        home: Target,
        health: u32,
    ) {
        let target = self.attackers[record].target;
        if self.can_take(target, settler.player) {
            self.take_building(settler, number, record, home, health);
            return;
        }
        let Some(field) = self.free_field(settler.walker.at, Some(number)) else {
            return;
        };
        self.attackers[record].field = Some(field);
        self.attackers[record].offset = None;
        self.find_opponent(record, settler.player);
        self.walk_to(settler, field);
        settler.job = SettlerJob::Attack {
            home,
            health,
            step: AttackStep::Walking,
        };
    }

    /// MIL-ATK-5.1: pair the attacker (a soldier of `attacking`) with the
    /// first unpaired interceptor of the target itself waiting outside
    /// (MIL-ATK-3.4); otherwise, if the target's owner is an enemy, with one
    /// of the target's idle soldiers.
    fn find_opponent(&mut self, record: usize, attacking: PlayerId) {
        let target = self.attackers[record].target;
        let Some(owner) = self.target_building(target).map(|b| b.owner) else {
            return;
        };
        if let Some(id) = self.waiting_interceptor(target) {
            self.pair_waiting_interceptor(record, id);
            return;
        }
        if !self.are_enemies(owner, attacking) {
            return;
        }
        let strong = self.military_settings(owner).prefer_strong;
        if let Some(defender) = self.pick_defender(target.at, strong) {
            self.pair(record, defender, target, false, None);
        }
    }

    /// MIL-ATK-6.6, 3.4: the job logic of a soldier sent against an
    /// attacker; false when it leaves the world.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn defend_job(
        &mut self,
        settler: &mut Settler,
        number: usize,
        home: Target,
        health: u32,
        step: DefendStep,
        opponent: Option<usize>,
        intercept: bool,
        defends: Option<Target>,
    ) -> bool {
        if health == 0 {
            if let Some(m) = self.military.get_mut(&home.at) {
                m.soldiers.retain(|&s| s != number);
            }
            return false;
        }
        let home_stands =
            self.target_building(home).is_some() && self.military.contains_key(&home.at);
        if !home_stands {
            self.plan_home_excluding(settler, None);
            return true;
        }
        // The pair holds while the attacker's record points back.
        let record = opponent.and_then(|a| {
            self.attackers
                .iter()
                .position(|r| r.soldier == a && r.opponent == Some(number) && r.field.is_some())
        });
        let set = |step, opponent| SettlerJob::Defend {
            home,
            health,
            step,
            opponent,
            intercept,
            defends,
        };
        let arrived = !settler.walker.is_walking();
        if record.is_none() && step != DefendStep::Back {
            // 3.4: an interceptor out without an opponent waits where he
            // stands (defense wait) while the building he defends has
            // attackers (MIL-ATK-6.6, 6.1a: after his duel too).
            let wait = intercept
                && step != DefendStep::Leaving
                && defends
                    .is_some_and(|t| self.target_building(t).is_some() && self.has_attackers(t.at));
            if wait {
                if step != DefendStep::Waiting || opponent.is_some() {
                    settler.job = set(DefendStep::Waiting, None);
                }
                return true;
            }
            // 3.4: no opponent (any more): back to its building.
            if step == DefendStep::Leaving {
                self.rejoin(settler, number, home, health);
            } else if self.walk_to(settler, home.at) {
                settler.job = set(DefendStep::Back, None);
            } else {
                self.plan_home_excluding(settler, None);
            }
            return true;
        }
        match step {
            DefendStep::Leaving => {
                if settler.timer > 0 || !self.leave_building(home) {
                    return true;
                }
                let field = self.attackers[record.expect("paired")]
                    .field
                    .expect("a field");
                if self.walk_to(settler, field) {
                    settler.job = set(DefendStep::Out, opponent);
                }
            }
            DefendStep::Out => {
                if !arrived {
                    return true;
                }
                let r = &self.attackers[record.expect("paired")];
                let (field, target) = (r.field.expect("a field"), r.target);
                if settler.walker.at == field {
                    settler.job = set(DefendStep::Waiting, opponent);
                    self.under_attack(target);
                } else {
                    self.walk_to(settler, field);
                }
            }
            DefendStep::Waiting => {}
            DefendStep::Back => {
                if !arrived {
                    return true;
                }
                if settler.walker.at == home.at {
                    self.rejoin(settler, number, home, health);
                } else if !self.walk_to(settler, home.at) {
                    self.plan_home_excluding(settler, None);
                }
            }
        }
        true
    }

    // ---- A save's fights (FMT-SAV-3.7a) ----

    /// FMT-SAV-3.7a (T17): the fights of a save go on where they stood, in
    /// the next military update: each attacker, seated by the restore among
    /// his home's soldiers (the save lists him there, 3.4a), leaves that
    /// list and is out on his errand again with his record (target, saved
    /// round counter, waiting field, pending hit); a fighting pair stands on
    /// the record's field, the defender on its point and the attacker on his
    /// saved sample, one lattice step off it (MIL-ATK-6.1a), its opponent out
    /// of his own building as a defender, still among its soldiers; an attacker waiting (state 20) stands at his field, one
    /// walking on to a new waiting field (state 18 without an opponent)
    /// walks there from his field. The records keep the save's order, duels
    /// first.
    pub(crate) fn restore_fights(
        &mut self,
        fights: &[crate::RestoredFight],
        report: &mut crate::RestoreReport,
    ) {
        for fight in fights {
            if self.restore_fight(fight) {
                report.fights += 1;
            } else {
                report.fights_skipped += 1;
            }
        }
    }

    /// The garrison soldier with saved number `number` of the building on
    /// `home`, and his health.
    fn seated(&self, number: u32, home: Point) -> Option<(usize, u32)> {
        self.military.get(&home)?.soldiers.iter().find_map(|&id| {
            let s = self.settlers.get(id)?.as_ref()?;
            match s.job {
                SettlerJob::Garrison { health, .. } if s.number == number => Some((id, health)),
                _ => None,
            }
        })
    }

    fn restore_fight(&mut self, fight: &crate::RestoredFight) -> bool {
        // Every field of the record on the map (a damaged save).
        let on_map = [
            Some(fight.attacker_at),
            fight.field,
            fight.opponent.map(|o| o.at),
        ]
        .into_iter()
        .flatten()
        .all(|f| self.terrain.contains(f));
        if !on_map {
            return false;
        }
        let (Some(target), Some(home)) = (
            self.target_of(fight.target),
            self.target_of(fight.attacker_home),
        ) else {
            return false;
        };
        if self.target_building(target).is_none() {
            return false;
        }
        let Some((attacker, health)) = self.seated(fight.attacker, fight.attacker_home) else {
            return false;
        };
        let opponent = match fight.opponent {
            Some(o) => {
                let (Some(found), Some(their_home)) =
                    (self.seated(o.number, o.home), self.target_of(o.home))
                else {
                    return false;
                };
                Some((found, their_home, o))
            }
            None => None,
        };
        let field = fight.field;
        let paired = opponent.is_some();
        // The attacker leaves his home's list (MIL-ATK-2.3).
        if let Some(m) = self.military.get_mut(&fight.attacker_home) {
            m.soldiers.retain(|&s| s != attacker);
        }
        let duel_field = field.filter(|_| fight.fighting && paired);
        let stands = duel_field
            .or(field.filter(|_| fight.attacker_state == 20))
            .unwrap_or(fight.attacker_at);
        let mut settler = self.settlers[attacker].take().expect("seated");
        settler.walker = crate::movement::Walker::new(stands);
        settler.timer = 0;
        let step = match field {
            Some(f) if f == stands => AttackStep::Waiting,
            Some(f) => {
                self.walk_to(&mut settler, f);
                AttackStep::Walking
            }
            None => AttackStep::Walking,
        };
        settler.job = SettlerJob::Attack { home, health, step };
        self.settlers[attacker] = Some(settler);
        let mut paired_with = None;
        if let Some(((id, health), their_home, o)) = opponent {
            // In a duel he stands on its field; paired but not fighting yet
            // he walks to the attacker's field from his own.
            let mut settler = self.settlers[id].take().expect("seated");
            settler.walker = crate::movement::Walker::new(duel_field.unwrap_or(o.at));
            settler.timer = 0;
            let step = match (duel_field, field) {
                (Some(_), _) => DefendStep::Waiting,
                (None, Some(f)) if f == o.at => DefendStep::Waiting,
                (None, Some(f)) => {
                    self.walk_to(&mut settler, f);
                    DefendStep::Out
                }
                (None, None) => DefendStep::Out,
            };
            settler.job = SettlerJob::Defend {
                home: their_home,
                health,
                step,
                opponent: Some(attacker),
                intercept: o.intercept,
                defends: o.intercept.then_some(target),
            };
            self.settlers[id] = Some(settler);
            paired_with = Some(id);
        }
        // FMT-SAV-3.7a, MIL-ATK-6.1a: an attacker on his field but off its
        // lattice point stands on his saved sample: paired, his step made;
        // without an opponent, he walks back to the point (Q-1050 b).
        let speed = self.settings.settler_speed;
        let offset = match (field, fight.attacker_sample) {
            (Some(f), Some(at)) if f == stands && at != grid::field_sample(f) => {
                let point = grid::field_sample(f);
                Some(if paired {
                    let mut walk = movement::LatticeWalk::new(&self.terrain, speed, point, at);
                    walk.done = walk.steps(&self.terrain).len() as u32;
                    walk.progress_bits = 0;
                    OffsetStep { walk, back: false }
                } else {
                    OffsetStep {
                        walk: movement::LatticeWalk::new(&self.terrain, speed, at, point),
                        back: true,
                    }
                })
            }
            _ => None,
        };
        self.attackers.push(AttackRecord {
            soldier: attacker,
            target,
            counter: fight.counter,
            field,
            opponent: paired_with,
            hit: fight.hit,
            offset,
        });
        true
    }

    /// MIL-ATK-6.1a: where an attacker stands on the height lattice while
    /// he makes or has made his step off his waiting field's point.
    pub(crate) fn attacker_fine_position(&self, settler: &Settler) -> Option<crate::FinePosition> {
        if !matches!(
            settler.job,
            SettlerJob::Attack {
                step: AttackStep::Waiting,
                ..
            }
        ) {
            return None;
        }
        let record = self.attackers.iter().find(|r| {
            self.settlers
                .get(r.soldier)
                .and_then(|s| s.as_ref())
                .is_some_and(|s| s.number == settler.number)
        })?;
        let offset = record.offset?;
        let (at, next, progress) = offset.walk.position(&self.terrain);
        Some(crate::FinePosition {
            at,
            next,
            progress_bits: progress.to_bits(),
            facing: next.map(|n| movement::direction_toward(at, n)),
        })
    }

    // ---- The duel (MIL-ATK-6) ----

    /// MIL-ATK-6.3: one round for every pair that stands on its field and
    /// is not in a fight animation, in record order.
    pub(crate) fn fight_rounds(&mut self) {
        for i in 0..self.attackers.len() {
            let record = self.attackers[i].clone();
            let (Some(defender), Some(field)) = (record.opponent, record.field) else {
                continue;
            };
            let attacker = record.soldier;
            let Some(mut a) = self.settlers[attacker].take() else {
                continue;
            };
            let Some(mut d) = self.settlers[defender].take() else {
                self.settlers[attacker] = Some(a);
                continue;
            };
            let ready =
                |s: &Settler| !s.walker.is_walking() && s.walker.at == field && s.timer == 0;
            // MIL-ATK-6.1a: the attacker has arrived when his step has ended.
            let stepped = self.offset_arrived(&record);
            let waiting = matches!(
                a.job,
                SettlerJob::Attack {
                    step: AttackStep::Waiting,
                    ..
                }
            ) && matches!(
                d.job,
                SettlerJob::Defend {
                    step: DefendStep::Waiting,
                    ..
                }
            );
            if waiting
                && stepped
                && ready(&a)
                && ready(&d)
                && health_of(&a).is_some_and(|h| h > 0)
                && health_of(&d).is_some_and(|h| h > 0)
            {
                self.round(i, &mut a, &mut d);
            }
            self.settlers[attacker] = Some(a);
            self.settlers[defender] = Some(d);
        }
    }

    fn round(&mut self, i: usize, attacker: &mut Settler, defender: &mut Settler) {
        let striker_attacks = self.attackers[i].counter.is_multiple_of(2);
        let (striker, other) = if striker_attacks {
            (attacker, defender)
        } else {
            (defender, attacker)
        };
        // 1: the previous round's hit lands on the soldier who strikes now.
        if self.attackers[i].hit {
            self.attackers[i].hit = false;
            let health = health_mut(striker).expect("a fighting soldier");
            *health -= 1;
            if *health == 0 {
                // UI-GAME-8.5a: a frag for the player whose blow it was.
                self.count_frag(other.player);
                return;
            }
        }
        // 2
        let ticks = self.military_rules.fighting_ticks;
        striker.timer = ticks;
        other.timer = ticks;
        // 3, 4
        let r1 = (u64::from(self.rng.draw()) % 7) as i32;
        let r2 = (u64::from(self.rng.draw()) % 7) as i32;
        let rank_s = self.soldier_rank(striker.profession).unwrap_or(0) as i32;
        let rank_o = self.soldier_rank(other.profession).unwrap_or(0) as i32;
        let a = (rank_s + 1).max(2);
        let h = a / 2;
        let b = (rank_o + 1 - h).max(2);
        self.attackers[i].hit = r1 <= a && r2 > b;
        // 5
        self.attackers[i].counter += 1;
    }

    /// GFX-ANIM-3.4, 3.8 (Q-087 answered), MIL-ATK-6.3: the clip request of
    /// soldier `n` while a duel round's fight animation runs (its timer, on
    /// its pair's field, both waiting): the one who struck this round
    /// WorkLoop0 (with the lookup good Sword, the attack); the other Hit
    /// when the round's hit test scored, else WorkLoop1; both for the
    /// fighting time.
    pub(crate) fn fight_request(&self, n: usize, settler: &Settler) -> Option<Action> {
        if settler.timer == 0 || settler.walker.is_walking() {
            return None;
        }
        let waiting = matches!(
            settler.job,
            SettlerJob::Attack {
                step: AttackStep::Waiting,
                ..
            } | SettlerJob::Defend {
                step: DefendStep::Waiting,
                ..
            }
        );
        if !waiting {
            return None;
        }
        let record = self
            .attackers
            .iter()
            .find(|r| r.soldier == n || r.opponent == Some(n))?;
        record.opponent?;
        if record.field? != settler.walker.at {
            return None;
        }
        // MIL-ATK-6.2, 6.3 step 5: the counter rose after the round, so an
        // odd counter means the attacker struck in it.
        let attacker_struck = record.counter % 2 == 1;
        Some(if (n == record.soldier) == attacker_struck {
            Action::WorkLoop(0)
        } else if record.hit {
            Action::Hit
        } else {
            Action::WorkLoop(1)
        })
    }

    // ---- Headquarters (MIL-SOL-6.3, MIL-ATK-5.4) ----

    /// MIL-SOL-6.2: attackers are reported at the building on `at`.
    pub(crate) fn has_attackers(&self, at: Point) -> bool {
        self.attackers.iter().any(|r| r.target.at == at)
    }
}
