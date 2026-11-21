//! Settlers other than road carriers: on their way from a warehouse to a
//! site or building, at work in a building, and on their way home
//! (`specs/ECO-requests-dispatch.md` ECO-REQ-3.6, `specs/TRN-carriers-transport.md`
//! TRN-CARRY-1.4, 7.1).

use crate::grid::{self, Point};
use crate::movement::{self, Walker};
use crate::{Building, ConstructionSite, PlayerId, Target, World};

/// TRN-CARRY-7.1: a settler that cannot reach a warehouse dies after this
/// many ticks (the settler dying time).
const DYING_TICKS: u32 = 28;
/// TRN-CARRY-7.1, MIL-ATK-6.6 (T31, E-mil-025): a dying settler's timer:
/// the dying's ticks and one more update, then he is removed (present in
/// the ticks t .. t + 28 of a dying from t).
pub(crate) const DYING_UPDATES: u32 = DYING_TICKS + 1;
/// TRN-CARRY-7.1 (T31a, E-mil-027): the skeleton doodad comes in the dying
/// settler's update in which this many ticks of the dying have passed.
const SKELETON_AFTER: u32 = 14;
/// T47 (TRN-FREE-6.4b): the start tick of a removed site's builder's
/// leaving step while the approach's back leg he walks on has not arrived:
/// the step waits for its timer's end, the arrival tick, and counts from it.
const BACK_LEG_HOLD: u64 = u64::MAX;
/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;
/// TRN-FREE-6.4 step 3: the warehouse search's field distance.
const HOME_WAREHOUSE_RANGE: u32 = 1000;
/// TRN-FREE-6.4 step 3: the farthest network flag a settler free-walks to.
const HOME_FLAG_RANGE: u32 = 30;
/// TRN-FREE-6.4 step 4: the failure count above which a homeless settler
/// gives up at the end of its wander step (52 searches and 52 steps).
pub(crate) const HOME_FAILURES: u8 = 50;

/// What a settler is doing.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlerJob {
    /// Walking to a construction site as its builder or leveller.
    ToSite { target: Target, builder: bool },
    /// Walking to a finished building as its worker.
    ToBuilding { target: Target },
    /// The worker of its building, in its work cycle.
    Worker { target: Target, work: crate::Work },
    /// Walking home to the warehouse `target` (TRN-CARRY-7.1).
    Home { target: Target },
    /// MIL-SOL-4.7: walking to its military building, linked to it from the
    /// start.
    ToMilitary { target: Target, health: u32 },
    /// MIL-SOL-2.4: inside the building on its door field; heals
    /// (MIL-SOL-1.2).
    Garrison {
        target: Target,
        health: u32,
        heal: u32,
    },
    /// MIL-ATK-2.3: sent out to attack; unlinked from its home building.
    Attack {
        home: Target,
        health: u32,
        step: crate::combat::AttackStep,
    },
    /// MIL-ATK-3, 5: a soldier of `home` sent out against an attacker; it
    /// stays linked to `home`.
    Defend {
        home: Target,
        health: u32,
        step: crate::combat::DefendStep,
        /// The attacker (settler id) it fights.
        opponent: Option<usize>,
        /// MIL-ATK-3: sent before the attackers left (an interceptor).
        intercept: bool,
        /// MIL-ATK-3.4: the building an interceptor defends (the attack's
        /// target); he waits outside while it has attackers. None for a
        /// defender of the target (and an older game's interceptor).
        defends: Option<Target>,
    },
    /// ECO-SPC-2: a geologist or scout sent to the flag `flag`, with the
    /// spots he still has to visit.
    Specialist {
        flag: Point,
        spots_left: u8,
        step: crate::SpecialistStep,
    },
    /// ECO-REQ-5.3 (b), 5.3c: a warehouse's helper carrying one ware out to
    /// the warehouse's flag, for an order or taken out without one
    /// (ECO-DIST-1.7d), then back in.
    CarryOut {
        warehouse: Target,
        /// Where the ware goes and for whom; none for a ware taken out.
        destination: Option<Point>,
        target: Option<Target>,
        /// ECO-REQ-5.3f: the order its ware serves.
        order: Option<crate::Order>,
        step: CarryOutStep,
    },
    /// TRN-CARRY-7.1: no warehouse within reach; MIL-ATK-6.6 (T31): killed
    /// in a duel. He stands where he is: on the height sample `stand` when
    /// that is not his field's point (an attacker on his offset step).
    Dying { stand: Option<Point> },
    /// TRN-FREE-6.4a: sent home, before its home search: the step and the
    /// tick it began in (each acts in a later tick), and the height sample
    /// it stands on or walks back from when that is not its field's point
    /// (a builder on his construction position, T24).
    Leaving {
        step: LeavingStep,
        at: crate::Tick,
        stand: Option<Point>,
        /// MIL-SOL-1.2, TRN-FREE-6.4 step 4 (Q-1034): a soldier's health,
        /// which he keeps while going home (a homeless one loses 1 at each
        /// 52nd failure); none for other settlers, and for a soldier of a
        /// game saved before format 70 (then his rank's maximum).
        health: Option<u32>,
    },
    /// TRN-CARRY-7.1a, 7.1b: the carrier (or donkey) `carrier` of a
    /// removed road, from the removal until it walks home over the roads.
    Released { carrier: usize, step: ReleasedStep },
}

/// TRN-FREE-6.4a: the steps of going home before the home search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeavingStep {
    /// (a) Waiting for the step in progress to end.
    Wait,
    /// (b) Finishing the step (the fine walk back to its field's point from
    /// the stand, its remaining ticks in the timer; none on the point).
    Finish,
    /// TRN-FREE-6.4b (T33): a removed site's builder on his approach walk
    /// (ECO-HQ-4.6b step 2) walks on: the rest of its out leg from `from` to
    /// the stand, its remaining ticks in the timer; then (b), back to his
    /// field's point.
    Out { from: Point },
    /// (c) The home search of TRN-FREE-6.4 is due.
    Search,
    /// MIL-SOL-4.5 (T19): a released soldier's step from the building's
    /// field to its flag; on arriving it goes through (a) to (c) from that
    /// tick.
    StepOut,
    /// TRN-FREE-6.4b (T34, Q-286): a removed site's builder in a work
    /// walk (ECO-HQ-4.6b step 4) walks the rest of it, `walk`, on to the
    /// construction position it goes to (each step's rate set when it
    /// starts, TRN-MOVE-2.5, the excess carried within it, TRN-FREE-5.3).
    /// In the tick he reaches it, without a standing tick, `walk` becomes
    /// the straight line from there to his field's point (TRN-FREE-5.1),
    /// ordered at progress 0, the excess dropped (`home`); then (c).
    Through {
        walk: movement::LatticeWalk,
        home: bool,
    },
    /// TRN-FREE-6.4 steps 2, 3: off the roads, the free walk to the home
    /// warehouse's network flag nearest to it.
    ToFlag,
    /// TRN-FREE-6.4 step 4 (2026-11-08 b): no warehouse in reach after
    /// `tries` failed home searches: a single step to a neighbouring field.
    Wander { tries: u8 },
    /// TRN-FREE-6.4 step 4: the home search again (as [`LeavingStep::Search`])
    /// after `tries` failed ones.
    Retry { tries: u8 },
}

/// TRN-CARRY-7.1a: where a released carrier is before it turns home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleasedStep {
    /// Its road is gone and it has not noticed; `walking`: a step was in
    /// progress then; `to_road`: it was on its way to that road over other
    /// roads (TRN-CARRY-7.1a, T45: a step ending in the notice tick counts
    /// as standing).
    Unnoticed { walking: bool, to_road: bool },
    /// Finishing the step it was walking.
    Finishing,
    /// Standing on the step's end field until its timer runs out.
    Standing,
    /// TRN-CARRY-7.1b: off the roads, free-walking to a network flag of a
    /// warehouse (TRN-FREE-6.4 steps 2, 3).
    ToFlag,
}

/// TRN-CARRY-7.1a: a released carrier stands in the tick its step ends
/// (A) and in A + 1; its way home is ordered in A + 2.
const RELEASED_STAND_TICKS: u32 = 2;

/// ECO-REQ-5.3c: where a warehouse's helper is in its round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarryOutStep {
    /// Standing on the door field, bidding for the ware's place at the flag
    /// (TRN-CARRY-4.4, its wait score): the first check raises the flag's
    /// bar, a later one admits it (ECO-REQ-5.3c, T19).
    Door { score: i64 },
    /// Standing on the door field with the ware's place granted, until the
    /// door ticks are over; the walk ordered in the third tick at the
    /// earliest.
    Admitted,
    /// The chain of walks to the flag (ECO-WORK-3.8b, 3.8c).
    Out,
    /// The put-down action at the flag; the ware lies there when it ends.
    PutDown,
    /// A pick-up action with nothing to pick up.
    Pickup,
    /// Standing in the going-home state before the walk back.
    Stand,
    /// Walking back in.
    Back,
    /// On the door field; gone in the next tick.
    In,
}

/// ECO-REQ-5.3c: the ticks a warehouse helper stands after its pick-up.
const HELPER_STAND_TICKS: u32 = 4;
/// ECO-REQ-5.3c: a new helper stands on the door field in its first two
/// ticks; its walk is ordered in the third. It is made by the transport's
/// part (ECO-TIME-1.5), after the settlers' update of its first tick, so
/// its timer counts down from the next.
pub(crate) const HELPER_DOOR_TICKS: u32 = 2;

/// ECO-REQ-0.2, FMT-SAV-3.5: the six lists of settlers, in the order the
/// settlers' part of a tick updates them. A settler is in the list of what
/// it was made for and stays in it while it changes jobs: a worker or a
/// carrier going home is still a worker or a carrier (a helper can be
/// either: the well's helper is in the workers' list, E-trace-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SettlerList {
    Workers,
    Constructors,
    /// Road carriers and warehouse helpers, with donkeys and boat carriers.
    Carriers,
    Levellers,
    Soldiers,
    Specialists,
}

/// ECO-REQ-0.2: a member of the settlers' lists as this program keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Member {
    /// A settler of [`World::settlers`], by id.
    Settler(usize),
    /// A road carrier, by its index.
    Carrier(usize),
    /// The builder at work on the site on this field.
    Builder(Point),
    /// The leveller at work on the site on this field.
    Leveller(Point),
}

/// A settler other than a road carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settler {
    /// SIM-START-4.2: its object number, taken when it was made (a settler
    /// keeps it while it changes jobs: a carrier, builder or leveller going
    /// home is the same settler).
    pub number: u32,
    pub player: PlayerId,
    /// Its profession (a good of type settler).
    pub profession: u32,
    /// ECO-REQ-0.2: the list it was made in.
    pub list: SettlerList,
    pub walker: Walker,
    pub job: SettlerJob,
    /// ECO-WORK-2.1: the action timer; 0 when no action runs.
    pub timer: u32,
    /// The ware it carries.
    pub carrying: Option<u32>,
}

/// MIL-SOL-1.2: the health a soldier's job carries (a soldier going home
/// keeps his, Q-1034); none for other jobs.
pub(crate) fn carried_health(job: &SettlerJob) -> Option<u32> {
    match *job {
        SettlerJob::Leaving { health, .. } => health,
        SettlerJob::Attack { health, .. }
        | SettlerJob::Defend { health, .. }
        | SettlerJob::Garrison { health, .. }
        | SettlerJob::ToMilitary { health, .. } => Some(health),
        _ => None,
    }
}

impl World {
    /// TRN-CARRY-4.3: the flags at which workers hold a reserved place for the
    /// product they carry out or are about to put down; a warehouse helper
    /// holds one for its ware (ECO-REQ-5.3 b).
    pub(crate) fn worker_reservations(&self) -> Vec<Point> {
        self.settlers
            .iter()
            .flatten()
            .filter(|s| s.carrying.is_some())
            .filter_map(|s| match s.job {
                // ECO-REQ-5.3c (T19): a helper's place counts from its
                // ware's admission.
                SettlerJob::CarryOut {
                    step: CarryOutStep::Door { .. },
                    ..
                } => None,
                SettlerJob::CarryOut { warehouse, .. } => Some(self.flag_of(warehouse.at)),
                SettlerJob::Worker {
                    target,
                    work:
                        crate::Work::Out { spot: None }
                        | crate::Work::AtFlag
                        | crate::Work::PutDown { second: false },
                } => Some(self.flag_of(target.at)),
                // ECO-OUT-3.8a: walking back, from his ware's admission.
                SettlerJob::Worker {
                    target,
                    work:
                        crate::Work::StandBack { .. }
                        | crate::Work::Return { .. }
                        | crate::Work::WaitBack { .. }
                        | crate::Work::Back { .. },
                } if self.held_joins.contains_key(&s.number) => Some(self.flag_of(target.at)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn add_settler(&mut self, settler: Settler) -> usize {
        let id = self.settlers.len();
        self.settlers.push(Some(settler));
        id
    }

    /// ECO-REQ-0.2a (2): the list of a settler made at a warehouse's door
    /// without a request, by its `profession` (ECO-DIST-1.7a: moved to
    /// another warehouse or a harbour; MIL-CON: out of a burning
    /// warehouse): builders constructors, levellers levellers, helpers and
    /// donkeys carriers, scouts and geologists specialists, the soldier ranks
    /// soldiers, every other profession (a boat carrier too) workers.
    pub(crate) fn door_list_of(&self, profession: u32) -> SettlerList {
        if Some(profession) == self.roles.boat_carrier {
            SettlerList::Workers
        } else {
            self.list_of(profession)
        }
    }

    /// ECO-REQ-0.2, 0.2a: the list of a settler by its `profession`: of a
    /// builder or leveller sent home from his site (the same settler, made
    /// in that list), of a released road carrier (made a carrier), of one
    /// sent for a harbour's request (SHIP-4.3) and of a donkey of a donkey
    /// farm (made a carrier). A settler sent for a building's, road's or
    /// site's request is made in the request kind's list where it is sent.
    pub(crate) fn list_of(&self, profession: u32) -> SettlerList {
        let p = Some(profession);
        if p == self.roles.builder {
            SettlerList::Constructors
        } else if p == self.roles.leveller {
            SettlerList::Levellers
        } else if [
            self.roles.helper,
            self.roles.donkey,
            self.roles.boat_carrier,
        ]
        .contains(&p)
        {
            SettlerList::Carriers
        } else if self.soldier_rank(profession).is_some() {
            SettlerList::Soldiers
        } else if [self.specialist_rules.geologist, self.specialist_rules.scout].contains(&p) {
            SettlerList::Specialists
        } else {
            SettlerList::Workers
        }
    }

    /// ECO-REQ-0.2: the members of the settlers' lists in the order of the
    /// settlers' part: list by list, each in the order its settlers were
    /// made (their object numbers, SIM-START-4.2: the original's lists hold
    /// their settlers in ascending numbers in every tick of E-trace-002,
    /// 011 and 012). A released road carrier is the settler it became.
    pub(crate) fn settler_order(&self) -> Vec<(SettlerList, u32, Member)> {
        let mut members: Vec<(SettlerList, u32, Member)> = self
            .settlers()
            .map(|(id, s)| (s.list, s.number, Member::Settler(id)))
            .collect();
        members.extend(
            self.carriers
                .iter()
                .enumerate()
                .filter(|(_, c)| !matches!(c.task, crate::CarrierTask::Retired))
                .map(|(i, c)| (SettlerList::Carriers, c.number, Member::Carrier(i))),
        );
        for (&at, site) in &self.sites {
            if let Some(builder) = site.builder {
                members.push((
                    SettlerList::Constructors,
                    builder.number,
                    Member::Builder(at),
                ));
            }
            if let Some(leveller) = &site.leveller {
                members.push((
                    SettlerList::Levellers,
                    leveller.number,
                    Member::Leveller(at),
                ));
            }
        }
        members.sort_by_key(|&(list, number, _)| (list, number));
        members
    }

    /// AI-CON-9.2 (T41): the buildings one of whose settlers (its worker,
    /// or one walking to it as its worker) stands, not walking, on the
    /// building's field.
    pub fn workers_at_home(&self) -> std::collections::BTreeSet<Point> {
        self.settlers()
            .filter_map(|(_, s)| match s.job {
                SettlerJob::Worker { target, .. } | SettlerJob::ToBuilding { target }
                    if s.walker.at == target.at && !s.walker.has_walk() =>
                {
                    Some(target.at)
                }
                _ => None,
            })
            .collect()
    }

    /// All settlers other than road carriers, with their ids.
    pub fn settlers(&self) -> impl Iterator<Item = (usize, &Settler)> {
        self.settlers
            .iter()
            .enumerate()
            .filter_map(|(id, s)| s.as_ref().map(|s| (id, s)))
    }

    /// The fields from `field` to the flag `to` over the roads: from a flag,
    /// from a field of a road (first to one of its ends, TRN-FREE-6.1 T43),
    /// or from a building's field (first out to its flag).
    pub(crate) fn road_walk_from(
        &self,
        player: PlayerId,
        field: Point,
        to: Point,
    ) -> Option<Vec<Point>> {
        if self.flag_at(field).is_some() {
            return self.road_walk(player, field, to);
        }
        if let Some(road) = self.road_at(field) {
            // TRN-FREE-6.1 (T43): from the road's first flag's search.
            let path = road.path.clone();
            return self.road_walk_from_road_field(player, field, &path, to);
        }
        let flag = self.terrain.neighbour(field, FLAG_DIRECTION)?;
        self.flag_at(flag)?;
        let mut walk = vec![flag];
        walk.extend(self.road_walk(player, flag, to)?);
        Some(walk)
    }

    /// TRN-CARRY-7.1, ECO-HQ-5.1: a released settler of `profession` at
    /// `field` goes home: to the warehouse of its network with the lowest
    /// route cost (which warehouse is not specified; the storage rule of
    /// ECO-REQ-5.5 is used), or dies if none can be reached. Without a
    /// profession (test worlds) it simply leaves.
    pub(crate) fn send_home(
        &mut self,
        player: PlayerId,
        profession: Option<u32>,
        field: Point,
        number: u32,
    ) {
        let Some(profession) = profession else {
            return;
        };
        let mut settler = self.dying_settler(player, profession, field, number);
        self.plan_home(&mut settler);
        self.add_settler(settler);
    }

    /// FMT-SAV-3.5a (c), TRN-FREE-6.4 step 4 (Q-1032 c): a saved settler
    /// going home with `failures` failed home searches behind it: as
    /// [`World::send_home`], a failed search wandering on with that count.
    pub(crate) fn send_home_after(
        &mut self,
        player: PlayerId,
        profession: u32,
        field: Point,
        number: u32,
        failures: u32,
    ) {
        let mut settler = self.dying_settler(player, profession, field, number);
        settler.job = SettlerJob::Leaving {
            step: LeavingStep::Retry {
                tries: u8::try_from(failures).unwrap_or(u8::MAX),
            },
            at: self.tick,
            stand: None,
            health: None,
        };
        self.plan_home(&mut settler);
        self.add_settler(settler);
    }

    /// ECO-FOOD-6.2, ECO-WORK-3.8a, 3.8b (T-door-5), TRN-FREE-6.4a (T40): a
    /// settler made in the building on `at` (the donkey breeding's donkey)
    /// appears on the building's door field and walks out, the door step
    /// onto the building's field a walk of its own, then the step to the
    /// building's flag. It is sent home on arriving there, as a released
    /// soldier is (MIL-SOL-4.5): TRN-FREE-6.4a from that tick, the walk home
    /// searched from the flag.
    pub(crate) fn send_home_by_door(
        &mut self,
        player: PlayerId,
        profession: Option<u32>,
        at: Point,
        number: u32,
    ) {
        let Some(profession) = profession else {
            return;
        };
        let mut settler = self.dying_settler(player, profession, at, number);
        settler.walker = self.walker_leaving(at, vec![self.flag_of(at)]);
        settler.job = SettlerJob::Leaving {
            step: LeavingStep::StepOut,
            at: self.tick,
            stand: None,
            health: None,
        };
        settler.timer = 0;
        self.add_settler(settler);
    }

    /// A new settler on `field` with nowhere to go yet: dying unless a home
    /// is planned for it.
    fn dying_settler(
        &self,
        player: PlayerId,
        profession: u32,
        field: Point,
        number: u32,
    ) -> Settler {
        Settler {
            number,
            player,
            profession,
            list: self.list_of(profession),
            walker: Walker::new(field),
            job: SettlerJob::Dying { stand: None },
            timer: DYING_UPDATES,
            carrying: None,
        }
    }

    /// TRN-FREE-6.4a: `settler` is sent home in this tick: its walk ends
    /// with the step in progress, then one going-home step a tick before its
    /// home search.
    pub(crate) fn start_leaving(&self, settler: &mut Settler) {
        settler.walker.path.clear();
        settler.walker.pauses.clear();
        settler.walker.road_to = None;
        settler.job = SettlerJob::Leaving {
            step: LeavingStep::Wait,
            at: self.tick,
            stand: None,
            health: carried_health(&settler.job),
        };
    }

    /// TRN-FREE-6.4a: a released settler of `profession` on `field` goes
    /// home: a leveller after his last step standing on its point, a builder
    /// whose building is complete on the sample `stand` (T24). Its id.
    pub(crate) fn send_home_after_steps(
        &mut self,
        player: PlayerId,
        profession: Option<u32>,
        field: Point,
        stand: Option<Point>,
        number: u32,
    ) -> Option<usize> {
        let profession = profession?;
        let mut settler = Settler {
            number,
            player,
            profession,
            list: self.list_of(profession),
            walker: Walker::new(field),
            job: SettlerJob::Dying { stand: None },
            timer: 0,
            carrying: None,
        };
        self.start_leaving(&mut settler);
        if let SettlerJob::Leaving { stand: on, .. } = &mut settler.job {
            // Only off its field's point.
            *on = stand.filter(|&s| s != grid::field_sample(field));
        }
        Some(self.add_settler(settler))
    }

    /// TRN-FREE-6.4a (a), T24: the lattice steps of the fine walk from
    /// `stand` back to the point of the settler's field (TRN-FREE-5.6).
    fn leaving_steps(&self, settler: &Settler, stand: Point) -> Vec<Point> {
        movement::fine_approach(
            &self.terrain,
            stand,
            grid::field_sample(settler.walker.at),
            0,
        )
    }

    /// TRN-FREE-6.4a, T24: where a settler sent home from a sample other
    /// than its field's point stands on the lattice, or walks back from it
    /// (the walk at progress 0 in the tick it is ordered); `None` once it
    /// is on its field's point.
    pub(crate) fn leaving_fine_position(
        &self,
        settler: &Settler,
    ) -> Option<crate::production::FinePosition> {
        let SettlerJob::Leaving {
            step,
            stand: Some(stand),
            ..
        } = settler.job
        else {
            return None;
        };
        let (at, next, progress) = match step {
            LeavingStep::Finish => {
                let steps = self.leaving_steps(settler, stand);
                let speed = self.settings.settler_speed;
                let total = movement::lattice_walk_ticks(&self.terrain, speed, stand, &steps);
                let elapsed = total.saturating_sub(settler.timer);
                movement::lattice_walk_at(&self.terrain, speed, stand, &steps, elapsed)
            }
            // T34: the work walk under way, then the line to the point.
            LeavingStep::Through { walk, .. } => walk.position(&self.terrain),
            // T33: on the approach's out leg from `from` to the stand.
            LeavingStep::Out { from } => {
                let steps = movement::fine_approach(&self.terrain, from, stand, 0);
                let speed = self.settings.settler_speed;
                let total = movement::lattice_walk_ticks(&self.terrain, speed, from, &steps);
                let elapsed = total.saturating_sub(settler.timer);
                movement::lattice_walk_at(&self.terrain, speed, from, &steps, elapsed)
            }
            _ => (stand, None, 0.0),
        };
        Some(crate::production::FinePosition {
            at,
            next,
            progress_bits: progress.to_bits(),
            facing: next.map(|n| movement::direction_toward(at, n)),
        })
    }

    /// Point `settler` at the nearest reachable warehouse, or let it die.
    fn plan_home(&mut self, settler: &mut Settler) {
        self.plan_home_excluding(settler, None);
    }

    /// As [`World::plan_home`], leaving out the warehouse on `exclude`. A
    /// warehouse that accepts the profession (ECO-DIST-1.2) is preferred;
    /// when none does, any warehouse takes it.
    pub(crate) fn plan_home_excluding(&mut self, settler: &mut Settler, exclude: Option<Point>) {
        self.plan_home_via(settler, exclude);
    }

    /// TRN-FREE-6.4 step 3: for a settler off the roads, the nearest
    /// finished warehouse of its player on its landmass within field
    /// distance 1000 that accepts its profession, then the flag of that
    /// warehouse's network nearest to the settler, if at most 30 fields
    /// away: the flag it free-walks to before going home.
    pub(crate) fn home_network_flag(&self, settler: &Settler) -> Option<Point> {
        let at = settler.walker.at;
        // Same landmass: equal stored continent indices (FMT-S2M-4.9).
        let landmass = |f: Point| self.continent_of(f);
        let warehouse = self
            .warehouses
            .keys()
            .filter_map(|&w| self.building_at(w))
            .filter(|b| {
                b.owner == settler.player
                    && !b.under_construction
                    && self.warehouse_accepts(b.at, settler.profession)
                    && landmass(b.at) == landmass(at)
                    && grid::distance(at, b.at) <= HOME_WAREHOUSE_RANGE
            })
            .min_by_key(|b| (grid::distance(at, b.at), b.serial))?;
        self.network_flag_near(settler, warehouse.at)
    }

    /// TRN-FREE-6.4 steps 2, 3: the flag of the network of the warehouse on
    /// `warehouse` nearest to the settler (field distance), if at most 30
    /// fields away.
    fn network_flag_near(&self, settler: &Settler, warehouse: Point) -> Option<Point> {
        let at = settler.walker.at;
        let home_flag = self.terrain.neighbour(warehouse, FLAG_DIRECTION)?;
        self.flag_order
            .iter()
            .copied()
            .filter(|&f| self.flag_at(f) == Some(settler.player))
            .filter(|&f| self.road_walk(settler.player, f, home_flag).is_some())
            .min_by_key(|&f| grid::distance(at, f))
            .filter(|&f| grid::distance(at, f) <= HOME_FLAG_RANGE)
    }

    fn plan_home_via(&mut self, settler: &mut Settler, exclude: Option<Point>) {
        if !self.home_found(settler, exclude) {
            self.wander(settler);
        }
    }

    /// TRN-FREE-6.4 step 1: points the settler home over the roads if a
    /// warehouse is in reach; false (the settler untouched) if none is.
    fn home_found(&self, settler: &mut Settler, exclude: Option<Point>) -> bool {
        let accepting = self.warehouses.keys().any(|&w| {
            Some(w) != exclude
                && self.building_at(w).is_some_and(|b| {
                    b.owner == settler.player
                        && !b.under_construction
                        && self.warehouse_accepts(w, settler.profession)
                })
        });
        let at = settler.walker.at;
        let mut best: Option<(u32, Target, Vec<Point>)> = None;
        let mut warehouses: Vec<&Building> = self
            .warehouses
            .keys()
            .filter_map(|&w| self.building_at(w))
            .filter(|b| b.owner == settler.player && !b.under_construction)
            .filter(|b| Some(b.at) != exclude)
            .filter(|b| !accepting || self.warehouse_accepts(b.at, settler.profession))
            .collect();
        warehouses.sort_by_key(|b| b.serial);
        for building in warehouses {
            let flag = self
                .terrain
                .neighbour(building.at, FLAG_DIRECTION)
                .expect("a building has a flag");
            let Some(mut walk) = self.road_walk_from(settler.player, at, flag) else {
                continue;
            };
            // T35: the route's cost is its sum of road lengths L (as 6.1),
            // from the flag the road walk starts at (the settler's own, or
            // the end or building's flag it first walks to); T36: off a
            // flag, plus that first leg, its L counted as a road's (the leg's
            // weight is not measured: medium).
            let first = std::iter::once(at)
                .chain(walk.iter().copied())
                .position(|f| self.flag_at(f) == Some(settler.player));
            let (start, leg) = match first {
                Some(0) | None => (at, 0),
                Some(i) => {
                    let fields: Vec<Point> = std::iter::once(at)
                        .chain(walk[..i].iter().copied())
                        .collect();
                    (walk[i - 1], self.road_length(&fields))
                }
            };
            let Some(cost) = self
                .route_costs_to(settler.player, flag, crate::transport::Search::Walk)
                .from(start)
            else {
                continue;
            };
            let cost = cost + leg;
            walk.push(building.at);
            if best.as_ref().is_none_or(|&(c, _, _)| cost < c) {
                let target = Target {
                    at: building.at,
                    serial: building.serial,
                    storage: false,
                };
                best = Some((cost, target, walk));
            }
        }
        match best {
            Some((_, target, mut walk)) => {
                // ECO-WORK-3.8b: entering, the walk to the warehouse's flag
                // and on to its field end there, and a one-step walk of its
                // own takes it to the door field (each from progress 0).
                let flag = self.flag_of(target.at);
                let door = self.door_field(target.at);
                let mut pauses: Vec<Point> = (settler.walker.at != flag)
                    .then_some(flag)
                    .into_iter()
                    .collect();
                if door != target.at {
                    walk.push(door);
                    pauses.push(target.at);
                }
                // TRN-FREE-6.4c (1): standing on the warehouse's own flag
                // the road walk has no step: it ends in the next tick, which
                // orders the step in (held until then).
                let on_flag = settler.walker.at == flag;
                settler.walker.walk(walk);
                settler.walker.pauses = pauses;
                // TRN-FREE-6.5: a road walk to the warehouse's flag.
                settler.walker.road_to = Some(flag);
                settler.walker.held = on_flag;
                settler.timer = u32::from(on_flag);
                settler.job = SettlerJob::Home { target };
                true
            }
            None => false,
        }
    }

    /// TRN-FREE-6.4 step 4 (2026-11-08 b, E-gfx-041): a home search found no
    /// warehouse: one wander step. A draw r = draw mod 6; the neighbours in
    /// ring order (directions 4, 5, 0, 1, 2, 3) that are inside the map,
    /// walkable for the settler (water for a boat carrier) and explored by
    /// its player count r down; the step goes to the one at which r drops
    /// below 0, else to the last that qualified (none: it stays). The
    /// failure count (0 when sent home) goes on in the leaving steps; what
    /// follows the step is [`World::after_wander`].
    fn wander(&mut self, settler: &mut Settler) {
        let tries = match settler.job {
            SettlerJob::Leaving {
                step: LeavingStep::Retry { tries },
                ..
            } => tries,
            _ => 0,
        };
        let at = settler.walker.at;
        let boat = self.roles.boat_carrier == Some(settler.profession);
        let mut r = i64::from(self.rng.draw() % 6);
        let mut step = None;
        for d in [4u8, 5, 0, 1, 2, 3] {
            let Some(n) = self.terrain.neighbour(at, d) else {
                continue;
            };
            let walkable = if boat {
                self.terrain.has(n, crate::map::state::WATER)
            } else {
                !self.terrain.has(n, movement::NOT_WALKABLE)
            };
            if !walkable || !self.is_explored(settler.player, n) {
                continue;
            }
            step = Some(n);
            r -= 1;
            if r < 0 {
                break;
            }
        }
        settler.walker.walk(step);
        settler.job = SettlerJob::Leaving {
            step: LeavingStep::Wander { tries },
            at: self.tick,
            stand: None,
            health: carried_health(&settler.job),
        };
        settler.timer = 0;
    }

    /// TRN-FREE-6.4 step 4: at the end of a wander step, with the failure
    /// count at most [`HOME_FAILURES`] the count goes up by one and the home
    /// search runs again in the next tick; above it (the 52nd failure) the
    /// settler dies (TRN-CARRY-7.1). A soldier instead loses 1 health
    /// (MIL-SOL-1.2; Q-1034: there is no search for a military building, no
    /// draw, no healing meanwhile): at 0 or below he dies as the others do,
    /// else his count starts over at 0 and he searches again in the next
    /// tick.
    fn after_wander(&self, settler: &mut Settler, tries: u8) {
        let mut health = carried_health(&settler.job);
        let next = if tries <= HOME_FAILURES {
            tries + 1
        } else if settler.list == SettlerList::Soldiers {
            // A soldier of an older save kept no health: his rank's maximum.
            let full = || {
                self.soldier_rank(settler.profession)
                    .map(|rank| self.soldier_max_health(rank))
                    .unwrap_or(0)
            };
            let left = health.unwrap_or_else(full).saturating_sub(1);
            health = Some(left);
            if left == 0 {
                self.die_homeless(settler);
                return;
            }
            0
        } else {
            self.die_homeless(settler);
            return;
        };
        settler.job = SettlerJob::Leaving {
            step: LeavingStep::Retry { tries: next },
            at: self.tick,
            stand: None,
            health,
        };
    }

    /// TRN-CARRY-7.1 (T31a, E-mil-027): 14 ticks into his dying the doodad
    /// the config names is put on the dying settler's sample (`stand`, or
    /// his field's point) and takes the next object number, in his place in
    /// the settlers' part; nothing when the config names none or one already
    /// lies there. (The map's own doodads are not in the simulation.)
    fn lay_skeleton(&mut self, settler: &Settler, stand: Option<Point>) {
        let Some(doodad) = self.settings.skeleton_doodad else {
            return;
        };
        let sample = stand.unwrap_or_else(|| grid::field_sample(settler.walker.at));
        if self.skeletons.contains_key(&sample) {
            return;
        }
        let field = stand.map_or(settler.walker.at, grid::sample_field_truncating);
        let number = self.new_number();
        self.skeletons.insert(
            sample,
            crate::Skeleton {
                doodad,
                number,
                field,
            },
        );
    }

    /// TRN-CARRY-7.1: a homeless settler gives up and dies.
    fn die_homeless(&self, settler: &mut Settler) {
        settler.walker.walk([]);
        settler.job = SettlerJob::Dying { stand: None };
        settler.timer = DYING_UPDATES;
    }

    /// A removed building's leveller, builder and workers go home. A site's
    /// builder stood on `builder_stand`, his work action with `builder_work`
    /// ticks left (0 when he was not hammering).
    pub(crate) fn evict(
        &mut self,
        building: &Building,
        site: Option<&ConstructionSite>,
        builder_stand: Option<Point>,
        builder_work: u32,
        builder_approach: Option<crate::construction::ApproachLeft>,
        builder_walk: Option<movement::LatticeWalk>,
    ) {
        if let Some(site) = site {
            if let Some(leveller) = &site.leveller {
                self.send_home(
                    building.owner,
                    self.roles.leveller,
                    building.at,
                    leveller.number,
                );
            }
            if let Some(builder) = site.builder {
                // TRN-FREE-6.4b: as after completion (6.4a, T24), from his
                // sample back to the point of the site's flag, T being the
                // tick after the removal; never on the site's field.
                let flag = self
                    .terrain
                    .neighbour(building.at, 1)
                    .unwrap_or(building.at);
                let id = self.send_home_after_steps(
                    building.owner,
                    self.roles.builder,
                    flag,
                    builder_stand,
                    builder.number,
                );
                if let Some(Some(s)) = id.map(|id| &mut self.settlers[id]) {
                    match builder_approach {
                        // T34 (Q-286): in a work walk he walks it on to its
                        // position, then the line to the flag's point, from
                        // the removal on.
                        None if builder_walk.is_some() => {
                            let walk = builder_walk.expect("a work walk");
                            if let SettlerJob::Leaving { at, health, .. } = s.job {
                                s.job = SettlerJob::Leaving {
                                    step: LeavingStep::Through { walk, home: false },
                                    at,
                                    stand: Some(walk.from),
                                    health,
                                };
                            }
                        }
                        // T33: on his approach walk he walks it to its end,
                        // out and back to the flag's point, from T on.
                        Some(left) => {
                            let back_leg = left.clock >= left.out_ticks;
                            let (step, timer) = if back_leg {
                                (
                                    LeavingStep::Finish,
                                    left.out_ticks + left.back_ticks - left.clock,
                                )
                            } else {
                                (
                                    LeavingStep::Out { from: left.from },
                                    left.out_ticks - left.clock,
                                )
                            };
                            if let SettlerJob::Leaving { at, health, .. } = s.job {
                                // T47 (TRN-FREE-6.4b, the back leg): that
                                // walk already ends on the flag's point, so
                                // going home has nothing left to walk: he
                                // stands in his arrival tick A and in A + 1
                                // (the step after A, as 6.4a's (b) after an
                                // empty (a)), and walks home in A + 2. The
                                // walk's end step is held until A, the tick
                                // the timer's last count falls in, one tick
                                // later than after the out leg; A is taken
                                // from the timer, so a removal before the
                                // settlers' part of its tick (a command) and
                                // one after it (a computer player's, AI-2.1a)
                                // are alike.
                                let at = if back_leg { BACK_LEG_HOLD } else { at };
                                s.job = SettlerJob::Leaving {
                                    step,
                                    at,
                                    stand: Some(left.out),
                                    health,
                                };
                            }
                            s.timer = timer;
                        }
                        None => {
                            if let SettlerJob::Leaving { at, .. } = &mut s.job {
                                *at += 1;
                            }
                            // Q-366 a: his work action runs on in T, counted
                            // down.
                            s.timer = builder_work;
                        }
                    }
                }
            }
        }
        for id in 0..self.settlers.len() {
            let Some(settler) = self.settlers[id].as_ref() else {
                continue;
            };
            // MIL-SOL-4.4 (Q-1071 c): a soldier out as a defender loses only
            // his home link and goes on (`World::defend_job`).
            if let SettlerJob::Worker { target, .. }
            | SettlerJob::ToMilitary { target, .. }
            | SettlerJob::Garrison { target, .. } = settler.job
            {
                if target.at == building.at && target.serial == building.serial {
                    // TRN-FREE-6.4a: home after the going-home steps, T
                    // being the tick after the removal (T28).
                    let mut settler = self.settlers[id].take().expect("a settler");
                    self.start_leaving(&mut settler);
                    if let SettlerJob::Leaving { at, .. } = &mut settler.job {
                        *at += 1;
                    }
                    // TRN-CARRY-7.2a (Q-230 a, b): sent home, he loses the
                    // ware in his hands at once (a worker's product from its
                    // pickup on): into no warehouse, not put down.
                    if let Some(good) = settler.carrying.take() {
                        self.used(good, 1);
                        self.drop_held_join(settler.number);
                    }
                    self.settlers[id] = Some(settler);
                }
            }
        }
    }

    /// TRN-FREE-6.5 c, d: a road walk of `player` that has just reached one
    /// of the player's flags before its destination searches its route
    /// there again (`route` from that flag to the destination, 6.1), over
    /// the roads as they are now; without a route it ends there (e). On the
    /// destination the road walk is done.
    pub(crate) fn road_walk_arrived(
        &self,
        player: PlayerId,
        walker: &mut Walker,
        route: &dyn Fn(Point, Point) -> Option<Vec<Point>>,
    ) {
        let Some(to) = walker.road_to else {
            return;
        };
        if walker.at == to {
            walker.road_to = None;
        } else if self.flag_at(walker.at) == Some(player) {
            walker.reroute(route(walker.at, to));
        }
    }

    /// ECO-REQ-0.2: the settlers' part of the tick: the six lists one after
    /// the other, workers, constructors, carriers and helpers (with donkeys
    /// and boat carriers), levellers, soldiers, specialists, each in its
    /// order ([`World::settler_order`]). A settler that leaves the world is
    /// taken out where it stands; one made during the pass (or becoming
    /// another member, a builder arriving at his site) is first updated in
    /// the next tick.
    pub(crate) fn update_settler_lists(&mut self) {
        let tick = self.tick;
        for (_, number, member) in self.settler_order() {
            match member {
                Member::Settler(id) => self.update_settler(id),
                Member::Carrier(carrier) => self.update_carrier(carrier, tick),
                Member::Builder(at) => {
                    let site = self.sites.get(&at);
                    if site
                        .and_then(|s| s.builder)
                        .is_some_and(|b| b.number == number)
                    {
                        self.build(at);
                    }
                }
                Member::Leveller(at) => {
                    let site = self.sites.get(&at);
                    if site
                        .and_then(|s| s.leveller.as_ref())
                        .is_some_and(|l| l.number == number)
                    {
                        self.level(at);
                    }
                }
            }
        }
    }

    /// One tick of a settler: timer, movement, job (ECO-WORK-2.2).
    fn update_settler(&mut self, id: usize) {
        let speed = self.settings.settler_speed;
        let Some(mut settler) = self.settlers[id].take() else {
            return;
        };
        // MIL-SOL-4.7a: a soldier sent for a request stands while its
        // timer runs (its creation tick); its walk starts after it.
        // TRN-FREE-6.4c (1): a walk home held for its no-step tick is
        // ordered then and moves from the next.
        let mut stands = matches!(settler.job, SettlerJob::ToMilitary { .. }) && settler.timer > 0;
        if matches!(settler.job, SettlerJob::Home { .. })
            && settler.timer > 0
            && settler.walker.held
        {
            settler.walker.held = false;
            stands = true;
        }
        settler.timer = settler.timer.saturating_sub(1);
        let player = settler.player;
        let route = |at: Point, to: Point| self.road_walk(player, at, to);
        // ECO-OUT-3.8a (3): a worker walking back with his ware not yet
        // admitted does not step onto the flag.
        let barred = match settler.job {
            SettlerJob::Worker {
                target,
                work: crate::Work::Back { .. },
            } if !self.admitted(&settler) => Some(self.flag_of(target.at)),
            _ => None,
        };
        if !stands {
            settler.walker.advance_with(
                &self.terrain,
                speed,
                |f| Some(f) != barred,
                &mut |walker| self.road_walk_arrived(player, walker, &route),
            );
        }
        match settler.job {
            SettlerJob::ToMilitary { .. }
            | SettlerJob::Attack { .. }
            | SettlerJob::Defend { .. } => {
                self.soldier_explores(settler.player, settler.walker.at);
            }
            SettlerJob::Garrison { .. } => self.heal_soldier(&mut settler),
            _ => {}
        }
        let keep = self.settler_job(&mut settler, id);
        if !keep {
            // TRN-FREE-6.3a: his object goes (taken into a stock, or dead):
            // the building or site still holding him lets him go.
            if let Some(target) = self.linked_home.remove(&settler.number) {
                let leveller = Some(settler.profession) == self.roles.leveller;
                self.unlink_target(target, id, leveller);
            }
        }
        if keep {
            self.settlers[id] = Some(settler);
        } else if let Some(good) = settler.carrying {
            // Q-230: a settler that leaves the world (steps into a
            // warehouse, dies) with a ware in hand takes it along, and
            // it is gone. Not specified for a released worker; a
            // released carrier's ware is destroyed (TRN-CARRY-7.2).
            self.used(good, 1);
            self.drop_held_join(settler.number);
        }
    }

    /// ECO-REQ-5.3c: one tick of a warehouse helper's round (the action
    /// timer counts down before the job logic). False when it has gone in.
    fn carry_out(
        &mut self,
        settler: &mut Settler,
        warehouse: Target,
        destination: Option<Point>,
        target: Option<Target>,
        order: Option<crate::Order>,
        step: CarryOutStep,
    ) -> bool {
        let flag = self.flag_of(warehouse.at);
        // ECO-REQ-5.3g (Q-284): he checks only his warehouse: at the door
        // its flag, on arriving at the flag field and in each tick of the
        // put-down and the pick-up a building on its field; else he goes
        // home at once (no put-down or pick-up actions).
        let gone = match step {
            CarryOutStep::Door { .. } => self.flag_at(flag).is_none(),
            CarryOutStep::Out if !settler.walker.is_walking() => {
                settler.carrying.is_none() || self.building_at(warehouse.at).is_none()
            }
            CarryOutStep::PutDown | CarryOutStep::Pickup => {
                self.building_at(warehouse.at).is_none()
            }
            _ => false,
        };
        if gone {
            self.helper_goes_home(settler, target);
            return true;
        }
        let next = match step {
            CarryOutStep::Door { mut score } => {
                // ECO-REQ-5.3c (T19), TRN-CARRY-4.4: the walk waits for the
                // ware's entry check, one per tick from the tick after the
                // helper was made: the first raises the flag's bar (to the
                // score 0), the next admits the ware; a ware leaving the
                // flag between resets the bar and delays it a tick.
                let admitted = settler
                    .carrying
                    .is_none_or(|good| self.flag_bid(flag, good, None, &mut score));
                if !admitted {
                    CarryOutStep::Door { score }
                } else {
                    // TRN-CARRY-2.4a: the admission grants the ware's place.
                    if settler.carrying.is_some() {
                        self.hold_join(settler.number);
                    }
                    // T35: his walk out starts in the tick the ware is
                    // admitted; there is no fixed wait at the door (usually
                    // t + 2: the bar raised at t + 1, admitted at t + 2).
                    settler.walker = self.walker_leaving(warehouse.at, vec![flag]);
                    CarryOutStep::Out
                }
            }
            CarryOutStep::Admitted if settler.timer > 0 => step,
            CarryOutStep::Admitted => {
                settler.walker = self.walker_leaving(warehouse.at, vec![flag]);
                CarryOutStep::Out
            }
            CarryOutStep::Out if settler.walker.is_walking() => step,
            CarryOutStep::Out => {
                settler.timer = self.settings.drop_ticks;
                CarryOutStep::PutDown
            }
            CarryOutStep::PutDown if settler.timer > 0 => step,
            CarryOutStep::PutDown => {
                // The ware lies at the flag from the end of the put-down.
                let joined = self.drop_held_join(settler.number);
                if let Some(good) = settler.carrying.take() {
                    let target = target.filter(|&t| self.target_building(t).is_some());
                    let destination = target.and(destination);
                    let id = self.new_ware_joined(good, flag, destination, target, joined);
                    // ECO-REQ-5.3f: the served order is filed with its ware.
                    if target.is_some() {
                        self.wares[id].as_mut().expect("live ware").order = order;
                    }
                    // TRN-CARRY-4.3 (T41): counted once, as lying from now.
                    self.unreserve(flag);
                }
                settler.timer = self.settings.pickup_ticks;
                CarryOutStep::Pickup
            }
            CarryOutStep::Pickup if settler.timer > 0 => step,
            CarryOutStep::Pickup => {
                settler.timer = HELPER_STAND_TICKS;
                CarryOutStep::Stand
            }
            CarryOutStep::Stand if settler.timer > 0 => step,
            CarryOutStep::Stand => {
                // Back in: onto the building's field, then a walk of its own
                // to the door field (ECO-WORK-3.8b).
                let door = self.door_field(warehouse.at);
                let mut walker = crate::movement::Walker::new(flag);
                if door == warehouse.at {
                    walker.walk([warehouse.at]);
                } else {
                    walker.walk([warehouse.at, door]);
                    walker.pauses = vec![warehouse.at];
                }
                settler.walker = walker;
                CarryOutStep::Back
            }
            CarryOutStep::Back if settler.walker.is_walking() => step,
            CarryOutStep::Back => {
                // ECO-REQ-5.3c (T23), TRN-FREE-6.4c: into a warehouse whose
                // door is its own field (a harbour) the step from the field
                // to the door still takes a tick: on the field in the
                // arrival tick, at the door in the next, gone in the one
                // after.
                if self.door_field(warehouse.at) == warehouse.at {
                    settler.timer = 2;
                }
                CarryOutStep::In
            }
            CarryOutStep::In if settler.timer > 0 => step,
            CarryOutStep::In => {
                if self.target_building(warehouse).is_some() {
                    if let Some(w) = self.warehouses.get_mut(&warehouse.at) {
                        w.add(settler.profession, 1);
                        return false;
                    }
                }
                self.plan_home(settler);
                return true;
            }
        };
        settler.job = SettlerJob::CarryOut {
            warehouse,
            destination,
            target,
            order,
            step: next,
        };
        true
    }

    /// ECO-REQ-5.3g, TRN-ROAD-6.3, 6.4: a helper going home destroys the
    /// ware in his hands (its order is open again, as for a destroyed ware)
    /// and walks home as any settler whose building is gone (TRN-FREE-6).
    fn helper_goes_home(&mut self, settler: &mut Settler, target: Option<Target>) {
        self.drop_held_join(settler.number);
        if let Some(good) = settler.carrying.take() {
            self.used(good, 1);
            if let Some(t) = target.filter(|t| !t.storage) {
                if self.target_building(t).is_some() {
                    self.order(t.at, good);
                }
            }
        }
        self.plan_home(settler);
    }

    /// The settler's job logic; false when it leaves the world.
    fn settler_job(&mut self, settler: &mut Settler, number: usize) -> bool {
        if settler.walker.ended_short() {
            // TRN-FREE-6.5 e: its road walk ended on a flag.
            match settler.job {
                // Going home it chooses again from there (6.4 step 4).
                SettlerJob::Home { .. } => {
                    self.plan_home(settler);
                    return true;
                }
                // TRN-FREE-6.3 (Q-272): the walk is not retried; it counts
                // as done where the settler stands, and the job checks the
                // place below (a worker's routine too, corrected 2026-11-15:
                // he goes home unless he stands on his building's field).
                SettlerJob::ToSite { .. }
                | SettlerJob::ToMilitary { .. }
                | SettlerJob::ToBuilding { .. } => {
                    let walker = &mut settler.walker;
                    walker.path.clear();
                    walker.pauses.clear();
                    walker.held = false;
                    walker.road_to = None;
                }
                // Any other walker (a specialist): stand-in (Q-272), it
                // waits on the flag and searches again each tick.
                _ => {
                    let to = settler.walker.road_to.expect("a road walk");
                    let route = self.road_walk(settler.player, settler.walker.at, to);
                    if route.is_some() {
                        settler.walker.reroute(route);
                    }
                    return true;
                }
            }
        }
        let arrived = settler.walker.next.is_none() && !settler.walker.is_walking();
        match settler.job.clone() {
            SettlerJob::ToSite { target, builder } => {
                let alive = self.target_building(target).is_some();
                if !arrived {
                    // TRN-FREE-6.4b (T37, T38): his site removed while he
                    // walks to it, he goes home as any settler still moving:
                    // his walk ends on the next flag he reaches, then the
                    // steps of 6.4a.
                    if !alive {
                        let mut walk = settler.walker.clone();
                        walk.end_at_next(|f| self.flag_at(f).is_some());
                        self.start_leaving(settler);
                        settler.walker.path = walk.path;
                    }
                    return true;
                }
                let on_flag = settler.walker.at == self.flag_of(target.at);
                let Some(site) = self.sites.get_mut(&target.at).filter(|_| alive) else {
                    self.plan_home(settler);
                    return true;
                };
                if !on_flag {
                    // TRN-FREE-6.3: not on the site's flag, he goes home at
                    // once. 6.3a: the site keeps him as its builder (or
                    // leveller) on the way until his object goes.
                    self.linked_home.insert(settler.number, target);
                    self.plan_home(settler);
                    return true;
                }
                if builder {
                    site.builder_on_way = false;
                    if site.builder.is_none() {
                        site.builder = Some(crate::Builder::arrived(settler.number));
                        return false;
                    }
                } else {
                    site.leveller_on_way = false;
                    if site.leveller.is_none() && !site.levelled() {
                        // ECO-HQ-4.3a: on the site's flag, in this tick.
                        let arrival = settler.walker.at;
                        self.leveller_arrives(target.at, arrival, settler.number);
                        return false;
                    }
                }
                // Not needed there any more.
                self.plan_home(settler);
                true
            }
            SettlerJob::ToBuilding { target } => {
                if !arrived {
                    return true;
                }
                if self.target_building(target).is_none() {
                    self.plan_home(settler);
                    return true;
                }
                // TRN-FREE-6.3 (corrected 2026-11-15, Q-272): every work
                // routine checks, when the walk to its building has ended,
                // that the worker stands on the building's field (entered,
                // its door field included); otherwise he goes home at once
                // from where he stands (6.4). 6.3a Worker (Q-1056): the
                // building keeps him as its worker on the way until his
                // object goes, then its 101-tick timer asks again.
                let at = settler.walker.at;
                if at != target.at && at != self.door_field(target.at) {
                    self.linked_home.insert(settler.number, target);
                    self.plan_home(settler);
                    return true;
                }
                let index = self.terrain.index(target.at);
                self.buildings[index].as_mut().expect("a building").active = true;
                self.worker_arrived(settler, target);
                true
            }
            SettlerJob::Worker { target, work } => {
                let work = self.work(settler, target, work, number);
                settler.job = SettlerJob::Worker { target, work };
                true
            }
            SettlerJob::Home { target } => {
                if !arrived {
                    return true;
                }
                // TRN-FREE-6.4b (T47b): his walk home cut by a removed road
                // ended short of the warehouse, which still stands: he
                // chooses his way home again in this, his arrival tick.
                let door = self.door_field(target.at);
                let short = settler.walker.at != door && settler.walker.at != target.at;
                if settler.timer == 0 && short && self.target_building(target).is_some() {
                    self.home_again_now(settler);
                    return true;
                }
                // ECO-REQ-5.3c, ECO-WORK-3.8b: it stands on the door field
                // in its arrival tick and is gone in the next (the timer,
                // set to 2 on arrival, reads 1 after the next tick's
                // countdown). TRN-FREE-6.4c (2): where the door is the
                // building's own field the one-step walk to it has no step
                // and takes a tick: gone a tick later (3).
                if settler.timer == 0 {
                    let no_door = self.door_field(target.at) == target.at;
                    settler.timer = if no_door { 3 } else { 2 };
                    return true;
                }
                if settler.timer > 1 {
                    return true;
                }
                if self.target_building(target).is_some() {
                    if let Some(warehouse) = self.warehouses.get_mut(&target.at) {
                        warehouse.add(settler.profession, 1);
                        // TRN-ROAD-6.3 (T32): a released carrier's claims
                        // end as it is taken in.
                        self.release_claims_of(settler.number);
                        return false;
                    }
                }
                self.plan_home(settler);
                true
            }
            SettlerJob::ToMilitary { target, health } => {
                if arrived {
                    self.soldier_arrived(settler, number, target, health);
                }
                true
            }
            SettlerJob::Garrison { .. } => true,
            SettlerJob::CarryOut {
                warehouse,
                destination,
                target,
                order,
                step,
            } => self.carry_out(settler, warehouse, destination, target, order, step),
            SettlerJob::Attack { home, health, step } => {
                self.attack_job(settler, number, home, health, step)
            }
            SettlerJob::Defend {
                home,
                health,
                step,
                opponent,
                intercept,
                defends,
            } => self.defend_job(
                settler, number, home, health, step, opponent, intercept, defends,
            ),
            SettlerJob::Specialist {
                flag,
                spots_left,
                step,
            } => self.specialist_job(settler, flag, spots_left, step),
            SettlerJob::Dying { stand } => {
                if settler.timer == DYING_UPDATES - SKELETON_AFTER {
                    self.lay_skeleton(settler, stand);
                }
                if settler.timer > 0 {
                    return true;
                }
                // TRN-ROAD-6.3 (Q-294): a released carrier's bindings end
                // with it.
                self.release_claims_of(settler.number);
                false
            }
            SettlerJob::Leaving {
                step,
                at,
                stand,
                health,
            } => {
                // TRN-FREE-6.4b (Q-366 a): a removed site's builder's work
                // action runs on, its time counted down as in any tick (the
                // settler's timer, before the job), until the walk replaces
                // it.
                // T34 (Q-286): the work walk goes on every tick, from the
                // removal on, to its position; in the tick he reaches it the
                // line to his field's point is ordered at progress 0 (the
                // walk's end drops the excess); the home search in the tick
                // after the line's arrival.
                if let LeavingStep::Through { mut walk, mut home } = step {
                    let speed = self.settings.settler_speed;
                    walk.advance(&self.terrain, speed);
                    if !home && walk.is_done(&self.terrain) {
                        let point = grid::field_sample(settler.walker.at);
                        walk = movement::LatticeWalk::new(&self.terrain, speed, walk.to, point);
                        home = true;
                    }
                    settler.job = if home && walk.is_done(&self.terrain) {
                        SettlerJob::Leaving {
                            step: LeavingStep::Search,
                            at: self.tick,
                            stand: None,
                            health,
                        }
                    } else {
                        SettlerJob::Leaving {
                            step: LeavingStep::Through { walk, home },
                            at,
                            stand,
                            health,
                        }
                    };
                    return true;
                }
                // T33: the approach walk's out leg goes on to its end, then
                // the walk back (the approach's back leg) from there.
                if let LeavingStep::Out { .. } = step {
                    if settler.timer == 0 {
                        if let Some(s) = stand {
                            let steps = self.leaving_steps(settler, s);
                            settler.timer = movement::lattice_walk_ticks(
                                &self.terrain,
                                self.settings.settler_speed,
                                s,
                                &steps,
                            );
                        }
                        settler.job = SettlerJob::Leaving {
                            step: LeavingStep::Finish,
                            at,
                            stand,
                            health,
                        };
                    }
                    return true;
                }
                // T47 (TRN-FREE-6.4b, the back leg): held until the walk's
                // arrival tick A, which then counts as the step's start.
                if matches!(step, LeavingStep::Finish) && at == BACK_LEG_HOLD {
                    if settler.timer == 0 {
                        settler.job = SettlerJob::Leaving {
                            step,
                            at: self.tick,
                            stand,
                            health,
                        };
                    }
                    return true;
                }
                // TRN-FREE-6.4a: one step a tick, after the tick it began.
                if self.tick <= at {
                    return true;
                }
                let tick = self.tick;
                match step {
                    LeavingStep::StepOut if settler.walker.is_walking() => {}
                    LeavingStep::StepOut => {
                        // On the flag's point: TRN-FREE-6.4a counts from now.
                        settler.job = SettlerJob::Leaving {
                            step: LeavingStep::Wait,
                            at: tick,
                            stand: None,
                            health,
                        };
                    }
                    LeavingStep::Wait if settler.walker.next.is_some() => {}
                    LeavingStep::Wait => {
                        // (a): the fine walk back to its field's point (T24),
                        // nothing to walk on the point.
                        let walk = stand.map(|s| (s, self.leaving_steps(settler, s)));
                        let stand = match walk {
                            Some((s, steps)) if !steps.is_empty() => {
                                settler.timer = movement::lattice_walk_ticks(
                                    &self.terrain,
                                    self.settings.settler_speed,
                                    s,
                                    &steps,
                                );
                                Some(s)
                            }
                            _ => None,
                        };
                        settler.job = SettlerJob::Leaving {
                            step: LeavingStep::Finish,
                            at: tick,
                            stand,
                            health,
                        };
                    }
                    // (b): until the fine walk's arrival tick.
                    LeavingStep::Finish if stand.is_some() && settler.timer > 0 => {}
                    LeavingStep::Finish => {
                        settler.job = SettlerJob::Leaving {
                            step: LeavingStep::Search,
                            at: tick,
                            stand: None,
                            health,
                        };
                    }
                    // TRN-FREE-6.4 step 4: the wander step, then the search
                    // again in the next tick, or the end.
                    LeavingStep::Wander { .. } if settler.walker.is_walking() => {}
                    LeavingStep::Wander { tries } => self.after_wander(settler, tries),
                    LeavingStep::Search | LeavingStep::Retry { .. } => {
                        // TRN-FREE-6.4: 1 from a flag or road on its field;
                        // on a building's field no road walk starts (6.5 a,
                        // T28), so 2 (the warehouse chosen from the field
                        // south-east), else 3, reached by a free walk to its
                        // network flag nearest to the settler.
                        let at = settler.walker.at;
                        let on_roads = self.flag_at(at).is_some() || self.road_at(at).is_some();
                        let flag = (!on_roads)
                            .then(|| {
                                self.home_flag_south_east(settler)
                                    .or_else(|| self.home_network_flag(settler))
                            })
                            .flatten();
                        match flag.and_then(|to| self.free_walk(at, to, 0)) {
                            Some(walk) => {
                                settler.walker.walk(walk);
                                settler.job = SettlerJob::Leaving {
                                    step: LeavingStep::ToFlag,
                                    at: tick,
                                    stand: None,
                                    health,
                                };
                            }
                            None => self.plan_home(settler),
                        }
                    }
                    LeavingStep::Out { .. } | LeavingStep::Through { .. } => {}
                    LeavingStep::ToFlag if settler.walker.is_walking() => {}
                    // At the walk's end it chooses again from the flag, in
                    // this tick; a way into the warehouse there is ordered
                    // in the arrival tick and moves from the next, with no
                    // standing tick (T32: 6.4c (1)'s belongs to a home
                    // search started on the flag).
                    LeavingStep::ToFlag => {
                        self.plan_home(settler);
                        if matches!(settler.job, SettlerJob::Home { .. }) {
                            settler.walker.held = false;
                            settler.timer = 0;
                        }
                    }
                }
                true
            }
            SettlerJob::Released { carrier, step } => {
                self.released_carrier(settler, carrier, step);
                true
            }
        }
    }

    /// TRN-CARRY-7.1a: one update of a released carrier, its movement
    /// already advanced in this tick.
    fn released_carrier(&mut self, settler: &mut Settler, carrier: usize, step: ReleasedStep) {
        let next = match step {
            ReleasedStep::Unnoticed { walking, to_road } => {
                // TRN-ROAD-6.3: noticing, a ware in its hands is destroyed
                // and one it had claimed is let go.
                self.let_go(carrier);
                // T34 (corrected 2026-10-27 b), T49 (2026-10-29): the walk
                // it is on is completed at its rate, every step of it, not
                // only the step in progress, but it ends on the first field
                // it reaches that is a flag or no longer a road field (the
                // roads as they are now).
                let walker = &mut settler.walker;
                walker.pauses.clear();
                walker.held = false;
                walker.end_at_next(|f| self.flag_at(f).is_some() || self.road_at(f).is_none());
                if walker.next.is_some() {
                    ReleasedStep::Finishing
                } else {
                    walker.path.clear();
                    // Its step ended in this update (A), or it stood: then
                    // the next tick takes A's place. T45: on its way to its
                    // road, a step ending now counts as standing too.
                    settler.timer = RELEASED_STAND_TICKS + u32::from(!walking) + u32::from(to_road);
                    ReleasedStep::Standing
                }
            }
            ReleasedStep::Finishing if settler.walker.is_walking() => step,
            ReleasedStep::Finishing => {
                // T48: a walk finished after the notice tick is the walking
                // case, whatever the road (A, A + 1).
                settler.timer = RELEASED_STAND_TICKS;
                ReleasedStep::Standing
            }
            ReleasedStep::Standing if settler.timer > 0 => step,
            ReleasedStep::Standing => {
                // TRN-FREE-6.4, TRN-CARRY-7.1b: the three tries, all in this
                // tick. 1: from a flag or road on its field, over the roads
                // as they now are; inside a building no road walk starts
                // (6.5 a, Q-361), so 2 decides.
                let at = settler.walker.at;
                let on_roads = self.flag_at(at).is_some() || self.road_at(at).is_some();
                if on_roads {
                    self.plan_home_via(settler, None);
                    return;
                }
                // 2: the warehouse chosen from the field south-east, else 3:
                // the nearest on the landmass; reached by a free walk to its
                // network flag nearest to the settler (the removed road's
                // fields are not walked as a road).
                let flag = self
                    .home_flag_south_east(settler)
                    .or_else(|| self.home_network_flag(settler));
                match flag.and_then(|to| self.free_walk(at, to, 0)) {
                    Some(walk) => {
                        settler.walker.walk(walk);
                        ReleasedStep::ToFlag
                    }
                    // Every try failed (the retries of TRN-FREE-6.4 step 4
                    // are not timed): it dies.
                    None => {
                        self.plan_home_via(settler, None);
                        return;
                    }
                }
            }
            ReleasedStep::ToFlag if settler.walker.is_walking() => step,
            ReleasedStep::ToFlag => {
                // TRN-FREE-6.4: at the walk's end, the warehouse north-west
                // of the flag takes it in, else it chooses again from the
                // flag, in this tick. TRN-CARRY-7.1b (T55): the way in is
                // ordered in the arrival tick, with no standing tick on the
                // flag (as LeavingStep::ToFlag).
                self.plan_home_via(settler, None);
                if matches!(settler.job, SettlerJob::Home { .. }) {
                    settler.walker.held = false;
                    settler.timer = 0;
                }
                return;
            }
        };
        settler.job = SettlerJob::Released {
            carrier,
            step: next,
        };
    }

    /// TRN-FREE-6.4b (T47b, E-trn-040): a settler whose walk home a removed
    /// road cut chooses his way home again at once, by TRN-FREE-6.4's tries
    /// 1 to 3, as at the end of any walk home: on a flag or road, over the
    /// roads; off them, a free walk to the warehouse's network flag nearest
    /// to him (the warehouse chosen from the field south-east, else the
    /// nearest), ordered at progress 0 now, and on that flag, in its arrival
    /// tick, the road walk on (`LeavingStep::ToFlag`).
    fn home_again_now(&mut self, settler: &mut Settler) {
        let at = settler.walker.at;
        let on_roads = self.flag_at(at).is_some() || self.road_at(at).is_some();
        if !on_roads {
            let flag = self
                .home_flag_south_east(settler)
                .or_else(|| self.home_network_flag(settler));
            if let Some(walk) = flag.and_then(|to| self.free_walk(at, to, 0)) {
                settler.walker.walk(walk);
                settler.job = SettlerJob::Leaving {
                    step: LeavingStep::ToFlag,
                    at: self.tick,
                    stand: None,
                    health: carried_health(&settler.job),
                };
                return;
            }
        }
        self.plan_home(settler);
        if matches!(settler.job, SettlerJob::Home { .. }) {
            settler.walker.held = false;
            settler.timer = 0;
        }
    }

    /// TRN-FREE-6.4 step 2 (corrected 2026-10-24, Q-266): when a flag, a
    /// building or a road lies south-east of the settler, the warehouse
    /// chosen from there as in step 1, and that warehouse's network flag
    /// nearest to the settler, within 30 fields.
    fn home_flag_south_east(&self, settler: &Settler) -> Option<Point> {
        let field = self.terrain.neighbour(settler.walker.at, FLAG_DIRECTION)?;
        let something = self.flag_at(field).is_some()
            || self.road_at(field).is_some()
            || self.building_at(field).is_some();
        if !something {
            return None;
        }
        let mut probe = settler.clone();
        probe.walker = Walker::new(field);
        self.home_found(&mut probe, None);
        let SettlerJob::Home { target } = probe.job else {
            return None;
        };
        self.network_flag_near(settler, target.at)
    }
}
