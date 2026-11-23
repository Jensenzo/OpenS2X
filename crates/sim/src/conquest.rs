//! Conquest and the catapult (`specs/MIL-conquest.md`).

use crate::grid::{self, Point};
use crate::map::state;
use crate::movement::{self, Walker};
use crate::{
    construction, BuildingSize, GoodKind, PlayerId, Settler, SettlerJob, Target, Tick, World,
};
use std::collections::BTreeMap;

/// ECO-time-units: `rm_setBurningTime`, ticks a ruin burns.
const BURNING_TICKS: u64 = 333;
/// MIL-CON-5.4: the catapult faces direction 1 at rest.
const REST_FACING: u8 = 1;
/// MIL-CON-5.2: the search step fires on the update where its counter
/// exceeds this.
const SEARCH_AFTER: u32 = 300;

/// A ruin left by a destroyed building (MIL-CON-4.1). What it holds is Q-062.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ruin {
    pub at: Point,
    pub owner: PlayerId,
    /// The tick it has burnt out.
    pub burns_until: Tick,
    /// MIL-CON-4.1a: the size of the building it was, whose field and
    /// footprint it marks while it burns.
    pub size: BuildingSize,
    /// GFX-OBJ-8.7 (5): the type of the building it was, whose remains
    /// (`bp_setRemains`) name its model and whose ground decal it lies on.
    /// `None` for a ruin of a game saved before format 72.
    pub building_type: Option<u32>,
}

/// Where a catapult's operator is in its cycle (MIL-CON-5.2, 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatapultPhase {
    /// Waiting for a stone.
    Idle,
    /// The search step: `counter` updates so far.
    Searching { counter: u32 },
    /// Turning towards the landing field; `left` updates to go.
    Turning {
        aim: Point,
        landing: Point,
        left: u32,
    },
    /// The stone flies.
    Flying {
        aim: Point,
        landing: Point,
        left: u32,
    },
    /// Turning back to the rest direction.
    TurningBack { left: u32 },
}

/// The state of one catapult.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Catapult {
    /// Tells it from a later building on the same field.
    pub serial: u32,
    pub facing: u8,
    pub phase: CatapultPhase,
}

/// MIL-CON-5.4 (2): the number of turn steps from `current` to `n`, "as the
/// original computes it".
pub fn turn_steps(current: u8, n: u8) -> u32 {
    if n > 3 {
        u32::from(6 - n)
    } else {
        (i32::from(n) - i32::from(current)).rem_euclid(6) as u32
    }
}

/// MIL-CON-5.4 (1), 5.6: whether a draw (below `radius / 2 + 1 + radius`)
/// makes a shot at distance `d` miss: it is below `radius / 2 + d`. So
/// `13 - d` of 19 draws hit with radius 12.
pub fn catapult_misses(radius: u32, d: u32, draw: u32) -> bool {
    draw < radius / 2 + d
}

impl World {
    /// The catapult state of the building on `at`.
    pub fn catapult_at(&self, at: Point) -> Option<&Catapult> {
        self.catapults.get(&at)
    }

    /// The ruins that burn.
    pub fn ruins(&self) -> &[Ruin] {
        &self.ruins
    }

    /// MIL-CON-5.5: soldiers `player` has killed with catapults.
    pub fn catapult_kills(&self, player: PlayerId) -> u32 {
        self.kills.get(&player).copied().unwrap_or(0)
    }

    // ---- Taking a building ----

    /// MIL-CON-1.1: whether an attacker of `player` can take the building
    /// `target` now.
    pub(crate) fn can_take(&self, target: Target, player: PlayerId) -> bool {
        let Some(building) = self.target_building(target) else {
            return false;
        };
        if !self.are_enemies(building.owner, player) {
            return true;
        }
        let Some(m) = self.military.get(&target.at) else {
            return false;
        };
        // Only soldiers still on their way in do not count; a present one
        // is an opponent (MIL-ATK-5.1) as well.
        m.soldiers.iter().all(|&id| {
            self.settlers
                .get(id)
                .and_then(|s| s.as_ref())
                .is_some_and(|s| matches!(s.job, SettlerJob::ToMilitary { .. }))
        })
    }

    /// MIL-CON-2: the attacker (settler `number`, record `record`) stands at
    /// the door of a building it can take.
    pub(crate) fn take_building(
        &mut self,
        settler: &mut Settler,
        number: usize,
        record: usize,
        home: Target,
        health: u32,
    ) {
        let target = self.attackers[record].target;
        let Some(old) = self.target_building(target).copied() else {
            self.attacker_goes_home(settler, number, home, health);
            return;
        };
        let at = target.at;
        if !self.are_enemies(old.owner, settler.player) {
            // Already taken: it just enters (MIL-CON-1.1, 3.1).
            self.join_taken(settler, number, record, home, health);
            return;
        }
        // 2.1: a headquarters is destroyed, nothing is taken over.
        if self.building_types[&old.building_type].warehouse {
            self.remove_with_remains(at);
            self.message(settler.player, at, "Attacked Castle destroyed.");
            self.attacker_goes_home(settler, number, home, health);
            return;
        }
        // 2.2 step 1 (corrected, T19, E-mil-038): nothing is done to the
        // target's interceptors at the take-over. A pair holds (the duel, or
        // the walk to it, goes on); an interceptor finds the building he
        // defends gone in defense wait and walks home (MIL-ATK-3.4).
        // 2
        self.message(old.owner, at, "Lost Military Building to enemy.");
        // 3: removed without remains, together with its flag, which stays
        // the old owner's otherwise.
        let flag = self.target_flag(target);
        self.delete_building(at);
        if let Some(flag) = flag {
            if self.flag_at(flag).is_some() {
                self.remove_flag(flag);
            }
        }
        // 4: a finished building of the same type for the attacker, which
        // claims at once (TER-2.2).
        let touched = self.put_building(settler.player, old.building_type, old.size, at, false);
        self.complete(at);
        self.claim_now(at);
        self.changed(&touched);
        // 5
        self.message(settler.player, at, "Military Building conquered.");
        // 6: the other attackers have the new building as their target.
        let new = self.target_of(at).expect("the new building");
        for r in &mut self.attackers {
            if r.target == target {
                r.target = new;
            }
        }
        // 2.3
        self.join_taken(settler, number, record, home, health);
    }

    /// MIL-CON-3.1: an attacker reaches a building of its own player.
    fn join_taken(
        &mut self,
        settler: &mut Settler,
        number: usize,
        record: usize,
        home: Target,
        health: u32,
    ) {
        let target = self.attackers[record].target;
        let player = settler.player;
        let own = self
            .target_building(target)
            .is_some_and(|b| b.owner == player && !b.under_construction)
            && self.military.contains_key(&target.at)
            && !self.building_types[&self
                .building_at(target.at)
                .expect("a building")
                .building_type]
                .warehouse;
        if !own {
            self.attacker_goes_home(settler, number, home, health);
            return;
        }
        let wanted = i64::from(self.wanted_soldiers(target.at));
        let soldiers = self.military[&target.at].soldiers.len() as i64;
        // MIL-SOL-4.2's R and 4.4's cancelling (ECO-REQ-2.3a).
        let mut open = self.linked_requests(player, target) as i64;
        let mut room = wanted - soldiers - open - 1;
        while room < 0 && open > 0 {
            self.cancel_linked_request(player, target);
            open -= 1;
            room += 1;
        }
        if room < 0 {
            self.attacker_goes_home(settler, number, home, health);
            return;
        }
        self.attackers.retain(|r| r.soldier != number);
        // MIL-CON-2.3 (Q-1052, E-mil-013): he stands on the flag, the door
        // (MIL-ATK-4.2), and walks in as every settler enters a building:
        // the walk to the flag is over at once (so no opponent can be paired
        // with him before it ends), then the walk onto the building's field
        // at the usual walking time; he is inside when it ends
        // (`soldier_arrived`). Ours ends the walk on the building's field
        // (no separate door step: TRN-FREE-6.3, he enters standing there).
        settler.job = SettlerJob::ToMilitary { target, health };
        settler.timer = 0;
        if settler.walker.at != target.at {
            settler.walker.walk([target.at]);
            // MIL-CON-2.3 (T18, E-mil-028): the walk to the flag takes this
            // update (he stands on it); the step in starts at progress 0 in
            // his next update and moves from the one after (he stands while
            // the timer runs, MIL-SOL-4.7a's hold).
            settler.timer = 1;
        }
        // MIL-ATK-2.3 (Q-1070): he leaves the list of the building that sent
        // him for this one's.
        self.leave_home_list(home, number);
        self.military
            .get_mut(&target.at)
            .expect("military")
            .soldiers
            .push(number);
    }

    // ---- Removal (MIL-CON-4) ----

    /// MIL-CON-4.1, 4.3: the building on `at` is destroyed and leaves a
    /// ruin (a headquarters never does); a finished warehouse sends the
    /// settlers in its stock out.
    pub(crate) fn remove_with_remains(&mut self, at: Point) {
        self.remove_with_remains_as(at, false);
    }

    /// [`World::remove_with_remains`]; `lost`: for land lost (TER-6.2,
    /// 6.3), before the settlers' part of the tick (TRN-FREE-6.4a, T49).
    pub(crate) fn remove_with_remains_as(&mut self, at: Point, lost: bool) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let headquarters = self.headquarters == Some(building.building_type);
        let evacuees: Vec<u32> = if kind.warehouse && !building.under_construction {
            self.warehouses.get(&at).map_or(Vec::new(), |w| {
                w.stock
                    .iter()
                    .filter(|&&(good, _)| {
                        self.roles.helper != Some(good)
                            && self
                                .goods
                                .get(&good)
                                .is_some_and(|g| g.kind == GoodKind::Settler)
                    })
                    .flat_map(|&(good, n)| std::iter::repeat_n(good, n as usize))
                    .collect()
            })
        } else {
            Vec::new()
        };
        let under_construction = building.under_construction;
        if !headquarters {
            self.put_remains(at);
        }
        self.delete_building_as(at, lost);
        if !headquarters && !under_construction {
            // SIM-START-4.2: the building's remains take a number.
            self.new_number();
            self.ruins.push(Ruin {
                at,
                owner: building.owner,
                burns_until: self.tick + BURNING_TICKS,
                size: building.size,
                building_type: Some(building.building_type),
            });
            // MIL-CON-4.1a: while it burns its field is marked like a
            // building's (ECO-PLACE-5.1).
            self.mark_ruin(at, building.size, true);
        }
        for profession in evacuees {
            // SIM-START-4.2: each comes out of the stock as a settler made.
            let mut settler = Settler {
                number: self.new_number(),
                player: building.owner,
                profession,
                // ECO-REQ-0.2a (2): by the profession.
                list: self.door_list_of(profession),
                walker: Walker::new(at),
                job: SettlerJob::Dying { stand: None },
                timer: 0,
                carrying: None,
            };
            self.plan_home_excluding(&mut settler, None);
            self.add_settler(settler);
        }
    }

    /// MIL-CON-4.4 (T14, T17): part of the material of the building on
    /// `at` goes on its flag as wares, each taking its number (SIM-START-4.2)
    /// before the building leaves the lists: of the frame material
    /// round(f x F) of the first good, of the main material M = round(m x T)
    /// handed out over the list in order, each good's sum halved rounded
    /// up, a site's unused material in full; one ware of each good in
    /// ascending good order per round while the flag holds fewer than its
    /// limit, the others lost.
    fn put_remains(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let site = self.sites.get(&at).filter(|_| building.under_construction);
        let (frame_costs, costs) = match site {
            Some(s) => (&s.frame_costs, &s.costs),
            None => (&kind.frame_costs, &kind.costs),
        };
        let (frame_total, main_total) =
            (construction::total(frame_costs), construction::total(costs));
        // f and m: the shares of frame and main work done (ECO-HQ-4.7).
        let (f, m) = match site {
            Some(s) => {
                let (frame_left, main_left) = construction::remaining_work(
                    frame_total,
                    main_total,
                    s.unit_ticks,
                    s.unit_ticks - 1,
                    s.work_done,
                );
                (1.0 - frame_left, 1.0 - main_left)
            }
            None => (1.0f32, 1.0f32),
        };
        // round(x) = trunc(x + 0.5).
        let round = |share: f32, n: u32| (share * n as f32 + 0.5) as u32;
        let mut amounts: BTreeMap<u32, u32> = BTreeMap::new();
        if let Some(&(good, count)) = frame_costs.first() {
            *amounts.entry(good).or_default() += round(f, count);
        }
        let mut main = round(m, main_total);
        for &(good, count) in costs {
            let n = main.min(count);
            *amounts.entry(good).or_default() += n;
            main -= n;
        }
        for n in amounts.values_mut() {
            *n = n.div_ceil(2);
        }
        if let Some(s) = site {
            for &(good, n) in &s.on_site {
                *amounts.entry(good).or_default() += n;
            }
        }
        let flag = self.flag_of(at);
        let limit = self.settings.flag_package_limit as usize;
        while amounts.values().any(|&n| n > 0) {
            for (&good, n) in amounts.iter_mut().filter(|(_, n)| **n > 0) {
                *n -= 1;
                if self.wares_at(flag).len() < limit {
                    self.new_number();
                    self.made(Some(good), 1);
                    self.new_ware_at(good, flag, None, None);
                }
            }
        }
    }

    /// The ruins that have burnt out disappear, and the marks of their
    /// fields are cleared (MIL-CON-4.1a).
    pub(crate) fn burn_ruins(&mut self) {
        let now = self.tick;
        let out: Vec<(Point, BuildingSize)> = self
            .ruins
            .iter()
            .filter(|r| r.burns_until <= now)
            .map(|r| (r.at, r.size))
            .collect();
        self.ruins.retain(|r| r.burns_until > now);
        for (at, size) in out {
            self.mark_ruin(at, size, false);
        }
    }

    /// MIL-CON-4.1a, ECO-PLACE-5.1: a burning ruin's field is Blocked and
    /// its footprint MovementBlocked, as a building's; burnt out, cleared.
    fn mark_ruin(&mut self, at: Point, size: BuildingSize, on: bool) {
        let footprint = self.footprint_of(at, size);
        self.set_bits(at, state::BLOCKED, on);
        for &field in &footprint {
            self.set_bits(field, state::MOVEMENT_BLOCKED, on);
        }
        self.changed(&footprint);
    }

    // ---- The catapult (MIL-CON-5) ----

    /// MIL-CON-5.3: the nearest occupied enemy military building within the
    /// catapult's radius that is no headquarters.
    pub(crate) fn catapult_target(&self, at: Point, owner: PlayerId, radius: u32) -> Option<Point> {
        let mut best: Option<(u32, Point)> = None;
        for candidate in self.military_order() {
            let Some(b) = self.building_at(candidate) else {
                continue;
            };
            let kind = &self.building_types[&b.building_type];
            if !self.are_enemies(owner, b.owner)
                || kind.warehouse
                || kind.territory_range == 0
                || !self.military[&candidate].applied
            {
                continue;
            }
            // Interceptors are out and fewer than 2 are at home: spared.
            if self.interceptors_out(candidate) > 0 && self.present_soldiers(candidate) < 2 {
                continue;
            }
            let d = grid::distance(at, candidate);
            if d <= radius && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, candidate));
            }
        }
        best.map(|(_, field)| field)
    }

    pub(crate) fn catapult_stones(&self, at: Point) -> u32 {
        let Some(stone) = self.stone else { return 0 };
        self.inputs(at)
            .iter()
            .filter(|&&(g, _)| g == stone)
            .map(|&(_, n)| n)
            .sum()
    }

    /// One update of a finished catapult's operator (MIL-CON-5.2).
    pub(crate) fn update_catapult(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        if !kind.catapult || building.under_construction || !building.active {
            return;
        }
        let radius = kind.working_radius;
        let state = self.catapults.entry(at).or_insert(Catapult {
            serial: building.serial,
            facing: REST_FACING,
            phase: CatapultPhase::Idle,
        });
        if state.serial != building.serial {
            *state = Catapult {
                serial: building.serial,
                facing: REST_FACING,
                phase: CatapultPhase::Idle,
            };
        }
        let state = *state;
        let ready = building.workers >= 1 && building.production.enabled;
        let next = match state.phase {
            CatapultPhase::Idle => {
                if ready && self.catapult_stones(at) >= 1 {
                    CatapultPhase::Searching { counter: 0 }
                } else {
                    CatapultPhase::Idle
                }
            }
            CatapultPhase::Searching { counter } => {
                if counter + 1 > SEARCH_AFTER {
                    self.catapult_search(at, building.owner, radius, state.facing)
                } else {
                    CatapultPhase::Searching {
                        counter: counter + 1,
                    }
                }
            }
            CatapultPhase::Turning { aim, landing, left } => {
                if left > 1 {
                    CatapultPhase::Turning {
                        aim,
                        landing,
                        left: left - 1,
                    }
                } else {
                    // 5.4 (3): the stone is taken and flies.
                    self.catapult_facing(at, movement::direction_toward(at, landing));
                    self.take_stone(at);
                    let left = grid::distance(at, landing) * self.settings.catapult_flight_ticks;
                    CatapultPhase::Flying { aim, landing, left }
                }
            }
            CatapultPhase::Flying { aim, landing, left } => {
                if left > 1 {
                    CatapultPhase::Flying {
                        aim,
                        landing,
                        left: left - 1,
                    }
                } else {
                    // 5.4 (4): impact, then turning back (5).
                    if landing == aim && self.building_at(landing).is_some() {
                        self.catapult_hit(at, landing);
                    }
                    let steps = turn_steps(self.catapults[&at].facing, REST_FACING);
                    CatapultPhase::TurningBack {
                        left: (steps + 1) * self.settings.catapult_turn_ticks,
                    }
                }
            }
            CatapultPhase::TurningBack { left } => {
                if left > 1 {
                    CatapultPhase::TurningBack { left: left - 1 }
                } else {
                    self.catapult_facing(at, REST_FACING);
                    CatapultPhase::Idle
                }
            }
        };
        self.catapults.get_mut(&at).expect("a catapult").phase = next;
    }

    fn catapult_facing(&mut self, at: Point, facing: u8) {
        self.catapults.get_mut(&at).expect("a catapult").facing = facing;
    }

    fn take_stone(&mut self, at: Point) {
        let Some(stone) = self.stone else { return };
        if let Some(list) = self.inputs.get_mut(&at) {
            if let Some(entry) = list.iter_mut().find(|(g, n)| *g == stone && *n > 0) {
                entry.1 -= 1;
                self.used(stone, 1);
            }
        }
        // A new one is ordered.
        self.file_input_orders(at);
    }

    /// The search step has run its 301 updates: look for a target and
    /// start the shot (MIL-CON-5.3, 5.4).
    fn catapult_search(
        &mut self,
        at: Point,
        owner: PlayerId,
        radius: u32,
        facing: u8,
    ) -> CatapultPhase {
        if self.catapult_stones(at) == 0 {
            return CatapultPhase::Idle;
        }
        let Some(aim) = self.catapult_target(at, owner, radius) else {
            return CatapultPhase::Searching { counter: 0 };
        };
        // 1: aim. 19 is radius / 2 + 1 + radius.
        let d = grid::distance(at, aim);
        let draw = (u64::from(self.rng.draw()) % u64::from(radius / 2 + 1 + radius)) as u32;
        let mut landing = aim;
        if catapult_misses(radius, d, draw) {
            let direction = (u64::from(self.rng.draw()) % 6) as usize;
            landing = self.terrain.neighbours(aim)[direction].unwrap_or(aim);
        }
        // 2: turn.
        let n = movement::direction_toward(at, landing);
        let steps = turn_steps(facing, n);
        CatapultPhase::Turning {
            aim,
            landing,
            left: (steps + 1) * self.settings.catapult_turn_ticks,
        }
    }

    /// MIL-CON-5.5: a stone hits the building on `target`.
    pub(crate) fn catapult_hit(&mut self, catapult: Point, target: Point) {
        let owner = self.building_at(catapult).expect("the catapult").owner;
        let victim = self.building_at(target).expect("a building").owner;
        let present = self.present_ranks(target);
        // The lowest rank, the first in the list on equal rank.
        if let Some(&(id, _)) = present.iter().min_by_key(|&&(_, r)| r) {
            self.settlers[id] = None;
            self.military
                .get_mut(&target)
                .expect("military")
                .soldiers
                .retain(|&s| s != id);
            self.message(victim, catapult, "Soldier killed by Catapult");
            *self.kills.entry(owner).or_insert(0) += 1;
            // UI-GAME-8.5a: the player's catapult shot killing a soldier.
            self.count_frag(owner);
        }
        if self.present_ranks(target).is_empty() {
            self.remove_with_remains(target);
        }
    }
}
