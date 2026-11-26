//! Geologists and scouts sent to a flag (`specs/ECO-specialists.md`).

use std::collections::VecDeque;

use crate::grid::{self, Point};
use crate::map::state;
use crate::movement::NOT_WALKABLE;
use crate::requests::Requester;
use crate::territory::player_bit;
use crate::{PlayerId, Rejection, Settler, SettlerJob, Tick, World};

/// ECO-SPC-2.2: spots a specialist visits per order.
pub const SPOTS_PER_ORDER: u8 = 10;
/// ECO-SPC-2.2: the search radius around the flag.
const GEOLOGIST_RADIUS: u32 = 5;
const SCOUT_RADIUS: u32 = 10;
/// ECO-SPC-2.3: ticks of work at a spot.
pub const WORK_TICKS: u32 = 80;
/// ECO-SPC-4.1: the scout explores this radius around a spot.
const SCOUT_REVEAL: u32 = 4;
/// ECO-SPC-2.2: a geologist digs only where none of these is set.
const NOT_FOR_DIGGING: u32 = state::BLOCKED | state::DOODAD_BLOCKED | state::DEPOSIT_BLOCKED;

/// ECO-SPC-3.2 (`si_addSignDoodadId`): the sign planted for `good` (none:
/// the "nothing found" sign) when the amount found is at least `threshold`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignRule {
    pub good: Option<u32>,
    pub threshold: u32,
    pub doodad: u32,
    /// `dd_setLifeTime` of the sign's doodad (LUA-PROP-9), ticks.
    pub life_time: Option<u32>,
}

/// What the rules give the specialists (ECO-SPC).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecialistRules {
    /// `good_scholar`, `good_explorer`.
    pub geologist: Option<u32>,
    pub scout: Option<u32>,
    /// ECO-PROD-2.5b: what a geologist reports on buildable ground.
    pub water: Option<u32>,
    pub signs: Vec<SignRule>,
    /// ECO-SPC-3.3: the message for each good found.
    pub found: Vec<(u32, &'static str)>,
}

/// A geologist's sign on a field (ECO-SPC-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sign {
    /// Its doodad type.
    pub doodad: u32,
    /// What was found and how much (none: nothing).
    pub good: Option<u32>,
    pub amount: u32,
    /// LUA-PROP-9: removed in this tick, if its doodad has a lifetime.
    pub gone_at: Option<Tick>,
}

/// A specialist's step at his flag (ECO-SPC-2).
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialistStep {
    /// ECO-SPC-2.1: on the roads to the flag.
    ToFlag,
    /// ECO-SPC-2.2: walking to the spot.
    ToSpot { spot: Point },
    /// ECO-SPC-2.3: working at the spot.
    Work { spot: Point },
    /// ECO-SPC-2.5: walking back to a flag to go home from there.
    Return { flag: Point },
}

impl World {
    /// ECO-SPC-1.1 (`send-geologist`, `send-scout`): a request for a
    /// geologist or scout at the player's flag on `at`.
    pub(crate) fn order_specialist(
        &mut self,
        player: PlayerId,
        at: Point,
        scout: bool,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(at) != Some(player) {
            return Err(Rejection::NotAFlag);
        }
        let rules = &self.specialist_rules;
        let profession = if scout { rules.scout } else { rules.geologist };
        let profession = profession.ok_or(Rejection::UnknownGood)?;
        // ECO-SPC-1.2: each order is its own request.
        let order = self.next_order;
        self.next_order += 1;
        self.request(
            player,
            Requester::Specialist { flag: at, order },
            profession,
        );
        Ok(())
    }

    /// The geologists' signs, by field.
    pub fn signs(&self) -> impl Iterator<Item = (Point, &Sign)> {
        self.signs.iter().map(|(&at, s)| (at, s))
    }

    pub fn sign(&self, field: Point) -> Option<&Sign> {
        self.signs.get(&field)
    }

    /// LUA-PROP-9: signs whose lifetime ran out are removed.
    pub(crate) fn update_signs(&mut self) {
        let now = self.tick;
        let before = self.signs.len();
        self.signs.retain(|_, s| s.gone_at.is_none_or(|t| t > now));
        if self.signs.len() != before {
            self.deposit_version += 1;
        }
    }

    /// The job of a geologist or scout once sent (ECO-SPC-2); false when he
    /// leaves the world.
    pub(crate) fn specialist_job(
        &mut self,
        settler: &mut Settler,
        flag: Point,
        spots_left: u8,
        step: SpecialistStep,
    ) -> bool {
        let arrived = settler.walker.next.is_none() && !settler.walker.is_walking();
        let player = settler.player;
        let flag_stands = self.flag_at(flag) == Some(player);
        let job = |step| SettlerJob::Specialist {
            flag,
            spots_left,
            step,
        };
        match step {
            SpecialistStep::ToFlag => {
                if !arrived {
                    return true;
                }
                // ECO-SPC-2.1 (corrected 2026-11-24): off the order's flag
                // he goes home at once from where he stands, no walk back
                // to the flag (2.1a: a walk cut short ends here too).
                if settler.walker.at != flag || !flag_stands {
                    self.specialist_goes_home(settler, flag);
                } else {
                    self.next_spot(settler, flag, spots_left);
                }
            }
            // ECO-SPC-2.4: the flag is gone: he finishes at once.
            SpecialistStep::ToSpot { .. } | SpecialistStep::Work { .. } if !flag_stands => {
                self.specialist_finishes(settler, flag);
            }
            SpecialistStep::ToSpot { spot } => {
                if arrived {
                    settler.timer = WORK_TICKS;
                    settler.job = job(SpecialistStep::Work { spot });
                }
            }
            SpecialistStep::Work { spot } => {
                if settler.timer > 0 {
                    return true;
                }
                if self.is_scout(settler.profession) {
                    self.scout_explores(player, spot);
                } else {
                    self.geologist_reports(player, spot);
                }
                let left = spots_left.saturating_sub(1);
                if left == 0 {
                    self.specialist_finishes(settler, flag);
                } else {
                    self.next_spot(settler, flag, left);
                }
            }
            SpecialistStep::Return { flag: to } => {
                if arrived {
                    if settler.walker.at == to && self.flag_at(to).is_some() {
                        self.plan_home_excluding(settler, None);
                    } else {
                        self.specialist_finishes(settler, flag);
                    }
                }
            }
        }
        true
    }

    fn is_scout(&self, profession: u32) -> bool {
        Some(profession) == self.specialist_rules.scout
    }

    /// ECO-SPC-2.2: the next spot; 2.2b: with none, or a spot whose walk
    /// cannot start, he goes home at once from where he stands.
    fn next_spot(&mut self, settler: &mut Settler, flag: Point, spots_left: u8) {
        let candidates = self.specialist_candidates(settler.player, flag, settler.profession);
        if candidates.is_empty() {
            self.specialist_goes_home(settler, flag);
            return;
        }
        // ECO-SPC-2.2: the random draw, over the one candidate.
        let spot = candidates[(u64::from(self.rng.draw()) % candidates.len() as u64) as usize];
        match self.free_walk(settler.walker.at, spot, 0) {
            Some(walk) => {
                settler.walker.walk(walk);
                settler.job = SettlerJob::Specialist {
                    flag,
                    spots_left,
                    step: SpecialistStep::ToSpot { spot },
                };
            }
            // ECO-SPC-2.2b: no walk to the spot: home at once.
            None => self.specialist_goes_home(settler, flag),
        }
    }

    /// ECO-SPC-2.2a: the spots marked now: the current spot of every
    /// geologist and scout (of any player) walking to it or working at it.
    /// The one choosing is out of the list while his job runs, so his own
    /// spot, just done, is not among them.
    fn marked_spots(&self) -> std::collections::BTreeSet<Point> {
        self.settlers
            .iter()
            .flatten()
            .filter_map(|s| match s.job {
                SettlerJob::Specialist {
                    step: SpecialistStep::ToSpot { spot } | SpecialistStep::Work { spot },
                    ..
                } => Some(spot),
                _ => None,
            })
            .collect()
    }

    /// ECO-SPC-2.2: the candidate spots around `flag`: a six-neighbour
    /// flood from the flag field over walkable fields that stops at the
    /// first candidate, or at the first field beyond the radius with none
    /// found. So at most one candidate, the nearest in frontier order. A
    /// geologist's candidate is diggable, a scout's unexplored; neither
    /// holds a layer-4 object: a sign, or another specialist's current spot
    /// (2.2a).
    pub(crate) fn specialist_candidates(
        &self,
        player: PlayerId,
        flag: Point,
        profession: u32,
    ) -> Vec<Point> {
        let scout = self.is_scout(profession);
        let marked = self.marked_spots();
        let radius = if scout {
            SCOUT_RADIUS
        } else {
            GEOLOGIST_RADIUS
        };
        let candidate = |f: Point| {
            if self.signs.contains_key(&f) || marked.contains(&f) {
                false
            } else if scout {
                !self.explored(player, f)
            } else {
                self.terrain.has(f, state::FOR_MINING)
                    && !self.terrain.has(f, NOT_FOR_DIGGING)
                    && self.flag_at(f).is_none()
                    && self.building_at(f).is_none()
                    && !self.deposits.contains_key(&f)
            }
        };
        let mut entered = vec![false; self.terrain.len()];
        let mut queue = VecDeque::from([flag]);
        entered[self.terrain.index(flag)] = true;
        if candidate(flag) {
            return vec![flag];
        }
        while let Some(a) = queue.pop_front() {
            for b in self.terrain.neighbours(a).into_iter().flatten() {
                let index = self.terrain.index(b);
                if entered[index] || self.terrain.has(b, NOT_WALKABLE) {
                    continue;
                }
                entered[index] = true;
                if grid::distance(flag, b) > radius {
                    return Vec::new();
                }
                if candidate(b) {
                    return vec![b];
                }
                queue.push_back(b);
            }
        }
        Vec::new()
    }

    /// ECO-SPC-4.1: the rectangle of radius 4 around `spot` (clamped to the
    /// map) becomes explored for the scout's player and every player that
    /// is not his enemy. The objects of the hexagonal radius become known
    /// to whoever sees them; the program draws every object on an explored
    /// field, so that part has nothing to do.
    fn scout_explores(&mut self, player: PlayerId, spot: Point) {
        let (width, height) = (self.terrain.width, self.terrain.height);
        let r = SCOUT_REVEAL;
        let bits = self
            .players
            .iter()
            .filter(|&&p| !self.are_enemies(player, p))
            .fold(player_bit(player), |bits, &p| bits | player_bit(p));
        // GFX-VIEW-5.4 (d): an exploration.
        self.explorations += 1;
        let xs = spot.0.saturating_sub(r)..=(spot.0 + r).min(width - 1);
        for y in spot.1.saturating_sub(r)..=(spot.1 + r).min(height - 1) {
            for x in xs.clone() {
                let index = self.terrain.index((x, y));
                self.explored[index] |= bits;
            }
        }
    }

    /// ECO-SPC-3, ECO-PROD-2.5b: what the geologist finds on `spot`, its
    /// sign and the message.
    pub(crate) fn geologist_reports(&mut self, player: PlayerId, spot: Point) {
        let found = if self.terrain.has(spot, state::FOR_BUILDING) {
            self.specialist_rules.water.map(|w| (w, 1))
        } else {
            self.resource_at(spot)
                .map(|r| (r.good, u32::from(r.amount)))
        };
        // ECO-SPC-3.2: a spot with a sign is never chosen (2.2).
        if !self.signs.contains_key(&spot) {
            // ECO-SPC-3.1, 3.2: no entry plants the empty sign; a good gets
            // the rule with the largest threshold not above the amount, and
            // no sign without one.
            let (good, amount) = found.map_or((None, 0), |(g, n)| (Some(g), n));
            let rule = self
                .specialist_rules
                .signs
                .iter()
                .filter(|s| s.good == good && s.threshold <= amount)
                .max_by_key(|s| s.threshold)
                .copied();
            if let Some(rule) = rule {
                let now = self.tick;
                // SIM-START-4.2: a sign is a doodad made.
                self.new_number();
                self.signs.insert(
                    spot,
                    Sign {
                        doodad: rule.doodad,
                        good,
                        amount,
                        gone_at: rule.life_time.map(|t| now + Tick::from(t)),
                    },
                );
                self.deposit_version += 1;
            }
        }
        // ECO-SPC-3.3
        if let Some((good, _)) = found {
            let text = self
                .specialist_rules
                .found
                .iter()
                .find(|(g, _)| *g == good)
                .map(|&(_, t)| t);
            if let Some(text) = text {
                self.message(player, spot, text);
            }
        }
    }

    /// ECO-SPC-2.5: he goes home: over the terrain back to the order's flag
    /// field, then home over the roads; with the flag gone the walk home
    /// starts where he stands (off the roads: TRN-FREE-6.4 step 3, first to
    /// the nearest flag of his home's network).
    fn specialist_finishes(&mut self, settler: &mut Settler, flag: Point) {
        self.specialist_home(settler, flag, true);
    }

    /// ECO-SPC-2.2b: home at once from where he stands (TRN-FREE-6.4), no
    /// walk back to the order's flag.
    fn specialist_goes_home(&mut self, settler: &mut Settler, flag: Point) {
        self.specialist_home(settler, flag, false);
    }

    /// ECO-SPC-2.5: home, by way of the order's flag field when `by_flag`
    /// and the flag stands, else from where he stands.
    fn specialist_home(&mut self, settler: &mut Settler, flag: Point, by_flag: bool) {
        let at = settler.walker.at;
        let on_roads = self.flag_at(at).is_some() || self.road_at(at).is_some();
        let to = if by_flag && self.flag_at(flag) == Some(settler.player) {
            (at != flag).then_some(flag)
        } else if on_roads {
            None
        } else {
            self.home_network_flag(settler)
        };
        let Some(to) = to else {
            self.plan_home_excluding(settler, None);
            return;
        };
        match self.free_walk(at, to, 0) {
            Some(walk) => {
                settler.walker.walk(walk);
                settler.job = SettlerJob::Specialist {
                    flag,
                    spots_left: 0,
                    step: SpecialistStep::Return { flag: to },
                };
            }
            // No walk back (not specified): home from where he stands.
            None => self.plan_home_excluding(settler, None),
        }
    }
}
