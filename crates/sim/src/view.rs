//! The view model: what a view (the renderer, the interface, sound, a tool,
//! a project built on these crates) reads of the people of the world, as plain values
//! (`docs/api.md`). The simulation's own state machines (a settler's job,
//! a worker's step, a carrier's task, a builder's approach) stay inside
//! it; [`World::settler_views`] answers in their place, one
//! [`SettlerView`] per settler, road carrier, builder and leveller.
//!
//! Every value here is read from the state, never computed differently
//! from it: a view that draws from a [`SettlerView`] draws the same as one
//! that read the state machines.

use crate::combat::{AttackStep, DefendStep};
use crate::grid::{self, Point};
use crate::movement::{Walker, ONE};
use crate::trace::{worker_request, Action};
use crate::{
    BuilderPose, CarrierTask, PlayerId, ReleasedStep, SettlerJob, SpecialistStep, Work, WorkCycle,
    World,
};

/// One step of a walk on a lattice (fields, or height samples for a fine
/// walk): from `at` toward `next` (none: standing on `at`), `done` of
/// `total` of the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub at: Point,
    pub next: Option<Point>,
    pub done: u32,
    pub total: u32,
    /// Progress per tick in millionths of a step (TRN-MOVE-1.1); 0 for a
    /// step that is not walked at the settlers' speed.
    pub speed: u32,
    /// The bits of the float32 progress the game keeps for a field step
    /// (TRN-MOVE-2.1, Q-241); `None` when `done` is the exact measure.
    pub progress_bits: Option<u32>,
}

impl Step {
    /// How far along the step, 0 to 1.
    pub fn fraction(&self) -> f32 {
        self.done as f32 / self.total as f32
    }

    /// A step that stands on `at`.
    pub fn standing(at: Point) -> Self {
        Self {
            at,
            next: None,
            done: 0,
            total: ONE,
            speed: 0,
            progress_bits: None,
        }
    }
}

impl From<&Walker> for Step {
    fn from(w: &Walker) -> Self {
        Self {
            at: w.at,
            next: w.next,
            done: w.progress_millionths(),
            total: ONE,
            speed: w.speed_millionths(),
            progress_bits: Some(w.progress_bits),
        }
    }
}

/// A fine walk on the height samples (TRN-FREE-5): the step and the
/// lattice direction faced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FineStep {
    pub step: Step,
    pub facing: Option<u8>,
}

/// Who a [`SettlerView`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SettlerRef {
    /// A settler by its number ([`World::settlers`]).
    Settler(usize),
    /// A road carrier, donkey or boat carrier by its number.
    Carrier(usize),
    /// The builder of the site on this field.
    Builder(Point),
    /// The leveller of the site on this field.
    Leveller(Point),
}

/// What a settler is doing, in the terms of the specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// On the way to a construction site as its builder or leveller.
    ToSite { site: Point, builder: bool },
    /// On the way to the building it will work in (by field and serial,
    /// which tells it from a later building on that field).
    ToBuilding { building: Point, serial: u32 },
    /// The worker of a building, in it or out on its work.
    Worker {
        building: Point,
        serial: u32,
        phase: WorkPhase,
    },
    /// Going home to a warehouse (TRN-FREE-6.4).
    Home,
    /// A soldier on the way to a military building.
    ToMilitary { building: Point },
    /// A soldier of a military building.
    Garrison { building: Point },
    /// A soldier sent to attack (MIL-ATK-4); `waiting` while it stands on
    /// its field checking each tick.
    Attack { waiting: bool },
    /// A soldier out to meet an attacker (MIL-ATK-5).
    Defend { waiting: bool },
    /// A geologist or scout ordered to a flag (ECO-SPC).
    Specialist { flag: Point },
    /// Dying (TRN-CARRY-7.1).
    Dying,
    /// A road carrier, donkey or boat carrier; `retired` when its road is
    /// gone.
    Carrier { road: usize, retired: bool },
    /// The builder at a site.
    Builder { site: Point },
    /// The leveller at a site.
    Leveller { site: Point },
}

/// The steps of a worker's cycle (ECO-WORK-3, ECO-OUT-3, 4), without their
/// details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkPhase {
    Begin,
    WaitInputs,
    Working,
    Take,
    WaitRoom,
    WaitTarget,
    WaitSpot,
    FishRetry,
    WaitShip,
    Donkey,
    Out,
    ToSpot,
    Approach,
    AtSpot,
    TakeWare,
    Return,
    Back,
    AtFlag,
    WaitFlag,
    PutDown,
    In,
    StepIn,
    Rest,
}

impl From<&Work> for WorkPhase {
    fn from(w: &Work) -> Self {
        match w {
            Work::Begin => Self::Begin,
            Work::WaitInputs | Work::Off => Self::WaitInputs,
            Work::Working => Self::Working,
            Work::Take { .. } => Self::Take,
            Work::WaitRoom { .. } => Self::WaitRoom,
            Work::WaitTarget { .. } => Self::WaitTarget,
            Work::WaitSpot { .. } => Self::WaitSpot,
            Work::FishRetry => Self::FishRetry,
            Work::WaitShip { .. } => Self::WaitShip,
            Work::Donkey => Self::Donkey,
            // ECO-FOOD-4.1a: standing, the trip already shown.
            Work::OutNext { .. } | Work::Out { .. } => Self::Out,
            Work::ToSpot { .. } => Self::ToSpot,
            Work::Approach { .. } => Self::Approach,
            Work::AtSpot { .. } => Self::AtSpot,
            Work::TakeWare { .. } | Work::StandBack { .. } => Self::TakeWare,
            Work::Return { .. } => Self::Return,
            Work::Back { .. } => Self::Back,
            Work::WaitBack { .. } => Self::Return,
            Work::AtFlag => Self::AtFlag,
            Work::WaitFlag => Self::WaitFlag,
            // ECO-FOOD-7.4 (T55): the plank put down at the ship, then he
            // stands before the work step.
            Work::PutDown { .. } | Work::AtShip { phase: 0 | 1, .. } => Self::PutDown,
            Work::AtShip { .. } => Self::AtSpot,
            Work::In => Self::In,
            Work::StepIn => Self::StepIn,
            Work::Rest => Self::Rest,
        }
    }
}

/// A settler, carrier, builder or leveller as a view sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct SettlerView {
    pub id: SettlerRef,
    /// Its own number: a settler's index, a carrier's creation number, 0
    /// for builders and levellers.
    pub number: u32,
    /// SIM-START-4.2: its object number (the original's trace ids).
    pub object: u32,
    pub player: PlayerId,
    /// Its profession (a good); a carrier's is the helper, donkey or boat
    /// carrier role, a builder's the builder role, a leveller's the
    /// leveller role (0 where the rules have none).
    pub profession: u32,
    /// Its walk on the fields (TRN-MOVE). A builder stands on his site's
    /// field here and walks on the samples ([`SettlerView::fine`]); a
    /// leveller stands on the neighbour he levels.
    pub walk: Step,
    /// GFX-ANIM-3.1a: a walk is ordered or a step runs.
    pub has_walk: bool,
    /// A step runs or fields of its walk are left (held or not).
    pub walking: bool,
    /// Its walk on the height samples while it has one (TRN-FREE-5,
    /// ECO-HQ-4.6b).
    pub fine: Option<FineStep>,
    /// GFX-ANIM-3.1: the action the logic asks for in this tick.
    pub request: Option<Action>,
    /// Ticks since a builder's or leveller's work action began (0 in its
    /// first tick), while it runs.
    pub request_age: Option<u64>,
    /// GFX-ANIM-3.1: inside its building (a work loop asked for there is
    /// not shown).
    pub inside: bool,
    /// Standing on its own building's field (a worker or soldier in it):
    /// not drawn.
    pub hidden: bool,
    pub activity: Activity,
    /// The good it carries (a carrier: the ware's good).
    pub carrying: Option<u32>,
    /// A soldier's health (MIL-SOL-1.2).
    pub health: Option<u32>,
    /// An attacker whose fight action runs (MIL-ATK-6.4).
    pub fighting: bool,
    /// The field a worker works at (ECO-OUT-3.5) or a specialist examines
    /// (ECO-SPC-2.3).
    pub work_spot: Option<Point>,
    /// The sample a worker stands on while he takes the ware he made
    /// outside (ECO-OUT-3.7).
    pub stand: Option<Point>,
    /// A carrier on a waterway (GFX-OBJ-7.6).
    pub on_water: bool,
    /// A carrier that is a donkey (TRN-CARRY-1.2).
    pub donkey: bool,
    /// The current step in the simulation's own words (a worker's, a
    /// soldier's on an errand, a specialist's, a carrier's task, a
    /// builder's pose), for debug text and the trace only: its form may
    /// change with any release.
    pub detail: Option<String>,
}

impl World {
    /// Every settler, then every road carrier, then the builder and the
    /// leveller of each site (by field): what each does, where and how.
    pub fn settler_views(&self) -> Vec<SettlerView> {
        let mut out = Vec::new();
        for (n, _) in self.settlers() {
            out.extend(self.settler_view(SettlerRef::Settler(n)));
        }
        for n in 0..self.carriers.len() {
            out.extend(self.settler_view(SettlerRef::Carrier(n)));
        }
        let sites: Vec<Point> = self.sites.keys().copied().collect();
        for at in sites {
            out.extend(self.settler_view(SettlerRef::Builder(at)));
            out.extend(self.settler_view(SettlerRef::Leveller(at)));
        }
        out
    }

    /// One [`SettlerView`], if that settler, carrier, builder or leveller
    /// exists.
    pub fn settler_view(&self, id: SettlerRef) -> Option<SettlerView> {
        match id {
            SettlerRef::Settler(n) => self.view_of_settler(n),
            SettlerRef::Carrier(n) => self.view_of_carrier(n),
            SettlerRef::Builder(at) => self.view_of_builder(at),
            SettlerRef::Leveller(at) => self.view_of_leveller(at),
        }
    }

    fn view_of_settler(&self, n: usize) -> Option<SettlerView> {
        let s = self.settlers.get(n)?.as_ref()?;
        let mut view = SettlerView {
            id: SettlerRef::Settler(n),
            number: n as u32,
            object: s.number,
            player: s.player,
            profession: s.profession,
            walk: Step::from(&s.walker),
            has_walk: s.walker.has_walk(),
            walking: s.walker.is_walking(),
            fine: self.fine_position(s).map(|f| FineStep {
                step: Step {
                    at: f.at,
                    next: f.next,
                    done: f.progress_millionths(),
                    total: ONE,
                    speed: 0,
                    progress_bits: Some(f.progress_bits),
                },
                facing: f.facing,
            }),
            request: None,
            request_age: None,
            inside: false,
            hidden: false,
            activity: Activity::Home,
            carrying: s.carrying,
            health: None,
            fighting: false,
            work_spot: None,
            stand: None,
            on_water: false,
            donkey: false,
            detail: None,
        };
        // TRN-FREE-6.4a, T24: sent home off its field's point, it is on the
        // field its sample truncates to, as a builder is (ECO-HQ-4.6b);
        // TRN-FREE-5.1, MAP-GRID-4.3: a worker on the lattice too.
        // MIL-ATK-6.6 (T31): a soldier dying on his offset sample too.
        let on_lattice = matches!(
            s.job,
            SettlerJob::Leaving { stand: Some(_), .. }
                | SettlerJob::Worker { .. }
                | SettlerJob::Dying { stand: Some(_) }
        );
        if let (true, Some(f)) = (on_lattice, &view.fine) {
            view.walk = Step::standing(grid::sample_field_truncating(f.step.at));
        }
        // MIL-ATK-6.1a (T29, E-mil-025; T37, E-mil-030): during either
        // step (off the waiting field's point, or back to it) an attacker's
        // field is the field the sample the step started from truncates to
        // (the waiting field for the step off); from the update the step
        // ends, the field of the sample he stands on.
        if let (SettlerJob::Attack { .. }, Some(f)) = (&s.job, &view.fine) {
            view.walk = Step::standing(grid::sample_field_truncating(f.step.at));
        }
        let standing_on = |at: Point| s.walker.at == at && s.walker.next.is_none();
        view.activity = match &s.job {
            SettlerJob::ToSite { target, builder } => Activity::ToSite {
                site: target.at,
                builder: *builder,
            },
            SettlerJob::ToBuilding { target } => Activity::ToBuilding {
                building: target.at,
                serial: target.serial,
            },
            SettlerJob::Worker { target, work } => {
                let farmer = self.target_building(*target).is_some_and(|b| {
                    self.building_types[&b.building_type].cycle == WorkCycle::Farm
                });
                (view.request, view.inside) = worker_request(self, work, farmer);
                // GFX-ANIM-3.1a (c1070f8): a walk to a target next to the
                // flag, with no field to walk, counts as moving.
                if matches!(work, Work::ToSpot { .. }) {
                    view.has_walk = true;
                }
                // ECO-WORK-3.8 (T-door-4): inside, it waits on the door field.
                view.hidden = standing_on(target.at) || standing_on(self.door_field(target.at));
                view.detail = Some(format!("{work:?}"));
                match *work {
                    Work::AtSpot { spot } => view.work_spot = Some(spot),
                    Work::TakeWare { stand, .. } => view.stand = stand,
                    _ => {}
                }
                Activity::Worker {
                    building: target.at,
                    serial: target.serial,
                    phase: WorkPhase::from(work),
                }
            }
            // ECO-HQ-4.7: a builder whose building completed during his
            // step's action works on until it ends.
            SettlerJob::Leaving {
                at, stand: Some(_), ..
            } if self.tick <= *at => {
                view.request = Some(Action::WorkLoop(0));
                Activity::Home
            }
            // TRN-FREE-6.4b (Q-366 a): a removed site's builder shows his
            // work action in T while its time runs.
            SettlerJob::Leaving {
                step: crate::settlers::LeavingStep::Wait,
                stand: Some(_),
                ..
            } if s.timer > 0 => {
                view.request = Some(Action::WorkLoop(0));
                Activity::Home
            }
            SettlerJob::Home { .. } | SettlerJob::Leaving { .. } => Activity::Home,
            // ECO-REQ-5.3c: the helper's put-down and pick-up actions.
            SettlerJob::CarryOut { step, .. } => {
                view.request = match step {
                    crate::settlers::CarryOutStep::PutDown => Some(Action::PutDown),
                    crate::settlers::CarryOutStep::Pickup => Some(Action::Pickup),
                    _ => None,
                };
                view.detail = Some(format!("{step:?}"));
                Activity::Home
            }
            SettlerJob::ToMilitary { target, health } => {
                view.health = Some(*health);
                Activity::ToMilitary {
                    building: target.at,
                }
            }
            SettlerJob::Garrison { target, health, .. } => {
                view.health = Some(*health);
                view.hidden = standing_on(target.at);
                Activity::Garrison {
                    building: target.at,
                }
            }
            SettlerJob::Attack { health, step, .. } => {
                view.health = Some(*health);
                view.fighting = s.timer > 0;
                view.request = self.fight_request(n, s);
                view.detail = Some(format!("{step:?}"));
                Activity::Attack {
                    // T26: on the meeting field before his wait job too.
                    waiting: matches!(step, AttackStep::Waiting | AttackStep::Arrived),
                }
            }
            SettlerJob::Defend {
                health, step, home, ..
            } => {
                view.health = Some(*health);
                // MIL-ATK-5.3 (T34): held inside, as a present soldier.
                if *step == DefendStep::Held {
                    view.hidden = standing_on(home.at);
                }
                view.request = self.fight_request(n, s);
                view.detail = Some(format!("{step:?}"));
                Activity::Defend {
                    waiting: *step == DefendStep::Waiting,
                }
            }
            SettlerJob::Specialist { flag, step, .. } => {
                view.detail = Some(format!("{step:?}"));
                if let SpecialistStep::Work { spot } = *step {
                    // GFX-ANIM-3.8: the geologist at a spot without a
                    // resource entry that is not buildable ground hammers
                    // (WorkLoop1), else digs; the scout plays WorkLoop0.
                    let hammer = Some(s.profession) == self.specialist_rules.geologist
                        && self.resource_at(spot).is_none()
                        && !self.terrain.has(spot, crate::map::state::FOR_BUILDING);
                    view.request = Some(Action::WorkLoop(u8::from(hammer)));
                    view.work_spot = Some(spot);
                }
                Activity::Specialist { flag: *flag }
            }
            SettlerJob::Dying { .. } => Activity::Dying,
            // TRN-CARRY-7.1a: until it notices, it still holds its ware.
            SettlerJob::Released { carrier, step, .. } => {
                if let ReleasedStep::Unnoticed { .. } = step {
                    let ware = self.carriers.get(*carrier).and_then(|c| c.ware);
                    view.carrying = ware.and_then(|w| self.ware(w)).map(|w| w.good);
                }
                view.detail = Some(format!("Released.{step:?}"));
                Activity::Home
            }
        };
        Some(view)
    }

    fn view_of_carrier(&self, n: usize) -> Option<SettlerView> {
        let c = self.carriers.get(n)?;
        let road = self.roads.get(c.road).and_then(Option::as_ref);
        let water = road.is_some_and(|r| r.water);
        let profession = if c.donkey {
            self.roles.donkey
        } else if water {
            self.roles.boat_carrier
        } else {
            self.roles.helper
        };
        Some(SettlerView {
            id: SettlerRef::Carrier(n),
            number: n as u32,
            object: c.number,
            player: road
                .and_then(|r| self.flag_at(r.path[0]))
                .unwrap_or(PlayerId(0)),
            profession: profession.unwrap_or(0),
            walk: Step::from(&c.walker),
            has_walk: c.walker.has_walk(),
            walking: c.walker.is_walking(),
            fine: None,
            // GFX-ANIM-3.1 with TRN-CARRY-3.1, 3.3: a pick-up or put-down is a
            // drop-time action, which requests PutDown, then a pickup-time
            // action, which requests Pickup (E-trace-005: carrier 90 204
            // fetching at (140, 50), PutDown 325 .. 328, Pickup 329 .. 332;
            // the helper's put-down of ECO-REQ T16 likewise).
            request: match c.task {
                CarrierTask::PickUp { second, .. } | CarrierTask::PutDown { second, .. } => {
                    Some(if second {
                        Action::Pickup
                    } else {
                        Action::PutDown
                    })
                }
                _ => None,
            },
            request_age: None,
            inside: false,
            hidden: false,
            activity: Activity::Carrier {
                road: c.road,
                retired: c.task == CarrierTask::Retired,
            },
            carrying: c.ware.and_then(|w| self.ware(w)).map(|w| w.good),
            health: None,
            fighting: false,
            work_spot: None,
            stand: None,
            on_water: water,
            donkey: c.donkey,
            detail: Some(format!("{:?}", c.task)),
        })
    }

    fn view_of_builder(&self, at: Point) -> Option<SettlerView> {
        let pose = self.builder_pose(at)?;
        let owner = self.building_at(at)?.owner;
        // ECO-HQ-4.6: he hammers at each construction position for a step's
        // action and walks between them.
        let (step, request_age) = match pose {
            BuilderPose::Working { at: s, since } => (
                Step::standing(s),
                Some(self.tick().saturating_sub(u64::from(since))),
            ),
            BuilderPose::Standing(s) => (Step::standing(s), None),
            BuilderPose::Walking {
                from,
                to,
                progress_bits,
            } => (
                Step {
                    at: from,
                    next: (from != to).then_some(to),
                    done: (f64::from(f32::from_bits(progress_bits)) * f64::from(ONE)) as u32,
                    total: ONE,
                    speed: 0,
                    progress_bits: Some(progress_bits),
                },
                None,
            ),
        };
        let walking = matches!(pose, BuilderPose::Walking { .. });
        Some(SettlerView {
            id: SettlerRef::Builder(at),
            number: 0,
            object: self.sites[&at].builder.map_or(0, |b| b.number),
            player: owner,
            profession: self.roles.builder.unwrap_or(0),
            // ECO-HQ-4.6b: on the field his last sample truncates to, the
            // flag field when he arrives.
            walk: Step::standing(grid::sample_field_truncating(pose.sample())),
            has_walk: walking,
            walking,
            fine: Some(FineStep { step, facing: None }),
            request: request_age.map(|_| Action::WorkLoop(0)),
            request_age,
            inside: false,
            hidden: false,
            activity: Activity::Builder { site: at },
            carrying: None,
            health: None,
            fighting: false,
            work_spot: None,
            stand: None,
            on_water: false,
            donkey: false,
            detail: Some(format!("{pose:?}")),
        })
    }

    fn view_of_leveller(&self, at: Point) -> Option<SettlerView> {
        let site = self.sites.get(&at)?;
        let leveller = site.leveller.as_ref()?;
        let owner = self.building_at(at)?.owner;
        let step = Step::from(&leveller.walker);
        let working = leveller.timer > 0;
        Some(SettlerView {
            id: SettlerRef::Leveller(at),
            number: 0,
            object: leveller.number,
            player: owner,
            profession: self.roles.leveller.unwrap_or(0),
            walk: step,
            has_walk: leveller.walker.has_walk(),
            walking: leveller.walker.is_walking(),
            fine: None,
            request: working.then_some(Action::WorkLoop(0)),
            request_age: working.then_some(0),
            inside: false,
            hidden: false,
            activity: Activity::Leveller { site: at },
            carrying: None,
            health: None,
            fighting: false,
            work_spot: None,
            stand: None,
            on_water: false,
            donkey: false,
            detail: None,
        })
    }
}

impl crate::Animal {
    /// Where the animal walks (ECO-FOOD-5.11).
    pub fn step(&self) -> Step {
        Step::from(&self.walker)
    }
}

impl crate::navy::Vessel {
    /// Where the ship sails (SHIP-3.1).
    pub fn step(&self) -> Step {
        Step::from(&self.walker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the view model reads the state as it is)
    #[test]
    fn a_walker_step_is_the_walker() {
        let mut w = Walker::new((3, 4));
        w.next = Some((4, 4));
        w.progress_bits = 0.25f32.to_bits();
        w.rate_bits = 0.045f32.to_bits();
        let s = Step::from(&w);
        assert_eq!((s.at, s.next, s.speed), ((3, 4), Some((4, 4)), 45_000));
        assert_eq!(s.fraction(), 0.25);
        assert_eq!(Step::standing((1, 1)).fraction(), 0.0);
    }

    // spec: none (WorkPhase names the variants of the worker's step)
    #[test]
    fn work_phases_name_the_steps() {
        assert_eq!(
            WorkPhase::from(&Work::AtSpot { spot: (1, 1) }),
            WorkPhase::AtSpot
        );
        assert_eq!(WorkPhase::from(&Work::Rest), WorkPhase::Rest);
    }
}
