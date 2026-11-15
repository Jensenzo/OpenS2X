//! The rows of a trace (`docs/trace-format.md`): what every settler and
//! every building is doing in a tick, in plain values, for comparing a run
//! with the original's. Also the animation a settler's logic asks for
//! (GFX-ANIM-3.1), which the view plays and the trace records.

use crate::grid::{self, Point};
use crate::{Work, World};

/// GFX-ANIM-3.1: the base animation the logic asks for in a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stand,
    Walk,
    PutDown,
    Pickup,
    /// WorkLoop0 to WorkLoop2.
    WorkLoop(u8),
    /// GFX-ANIM-3.8 (request code 7): the struck soldier of a duel round
    /// whose hit test scored (MIL-ATK-6.3).
    Hit,
    /// The dying state keeps the clip it had.
    Dying,
}

impl Action {
    pub fn name(self) -> String {
        match self {
            Self::Stand => "Stand".into(),
            Self::Walk => "Walk".into(),
            Self::PutDown => "PutDown".into(),
            Self::Pickup => "Pickup".into(),
            Self::WorkLoop(i) => format!("WorkLoop{i}"),
            Self::Hit => "Hit".into(),
            Self::Dying => "Dying".into(),
        }
    }
}

/// GFX-ANIM-3.1 requests from a worker's step, by GFX-ANIM-3.8's table: taking
/// the log or stone picks up, putting the product down at the flag (and the
/// drop half of taking inputs) puts down, the outdoor work action is
/// WorkLoop0 (a farmer's sowing, at a field not yet growing, WorkLoop1),
/// the work inside is WorkLoop0 under the inside flag. The second value is
/// that flag: the settler is inside, and a work loop request is ignored.
pub fn worker_request(world: &World, work: &Work, farmer: bool) -> (Option<Action>, bool) {
    match *work {
        // ECO-OUT-3.7: the drop half plays PutDown, the pickup half Pickup.
        Work::TakeWare { second, .. } => (
            Some(if second {
                Action::Pickup
            } else {
                Action::PutDown
            }),
            false,
        ),
        Work::Take { second } => (
            Some(if second {
                Action::Pickup
            } else {
                Action::PutDown
            }),
            true,
        ),
        // ECO-FOOD-7.4 (T55): at the ship, too; standing then.
        Work::AtShip { phase: 0, .. } => (Some(Action::PutDown), false),
        Work::AtShip { phase: 1, .. } => (Some(Action::Pickup), false),
        // ECO-OUT-3.9: at the flag, too, PutDown then Pickup.
        Work::PutDown { second } => (
            Some(if second {
                Action::Pickup
            } else {
                Action::PutDown
            }),
            false,
        ),
        // GFX-ANIM-3.8: a farmer sowing, the reserved spot's deposit of size
        // 0 (the field placed for the sowing, not yet growing), plays
        // WorkLoop1; harvesting a grown field WorkLoop0.
        Work::AtSpot { spot } => {
            let sowing = farmer
                && world
                    .deposit(spot)
                    .is_none_or(|d| d.state == crate::production::DepositState::Sapling);
            (Some(Action::WorkLoop(u8::from(sowing))), false)
        }
        Work::Working => (Some(Action::WorkLoop(0)), true),
        _ => (None, false),
    }
}

/// One settler's row: carriers, builders and levellers included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlerRow {
    /// `s<n>` a settler, `c<n>` a road carrier, `b<x>_<y>` and `l<x>_<y>`
    /// the builder and the leveller of the site on `(x, y)`.
    pub id: String,
    /// Its profession (a good id).
    pub profession: u32,
    /// The field it is on.
    pub field: Point,
    /// Its exact place in thousandths of a height sample (MAP-GRID-4).
    pub fine: (i64, i64),
    pub action: Action,
    /// The job and its step, as `Worker.AtSpot` or `Carrier.PickUp`.
    pub job: String,
}

/// One building's row: finished buildings and construction sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingRow {
    pub building_type: u32,
    pub field: Point,
    /// `levelling`, `frame`, `main` (a site's parts, ECO-HQ-4) or
    /// `finished`.
    pub state: &'static str,
    /// Thousandths done of the current part: of the levelling, of the
    /// frame part, of the main part (ECO-HQ-4.7); 1000 when finished. The
    /// original keeps each part's remainder, 1 to 0 (TOOL-trace TRACE-3.5).
    pub progress: u32,
}

/// The name of an enum value's variant from its `Debug` form.
fn variant(value: &dyn std::fmt::Debug) -> String {
    let text = format!("{value:?}");
    text.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}

/// A sample in thousandths of a sample.
fn milli(sample: Point) -> (i64, i64) {
    (i64::from(sample.0) * 1000, i64::from(sample.1) * 1000)
}

/// Between two samples, `progress` millionths of the way.
fn between(a: Point, b: Point, progress: u32) -> (i64, i64) {
    let (a, b) = (milli(a), milli(b));
    let p = i64::from(progress);
    (
        a.0 + (b.0 - a.0) * p / 1_000_000,
        a.1 + (b.1 - a.1) * p / 1_000_000,
    )
}

/// Between two samples by the float32 progress the game keeps (Q-241): the
/// position in double precision, in thousandths, rounded toward zero.
fn between_float(a: Point, b: Point, progress: f32) -> (i64, i64) {
    let p = f64::from(progress);
    let along = |a: u32, b: u32| {
        let (a, b) = (f64::from(a), f64::from(b));
        ((a + (b - a) * p) * 1000.0) as i64
    };
    (along(a.0, b.0), along(a.1, b.1))
}

fn walker_fine(walker: &crate::view::Step) -> (i64, i64) {
    let at = grid::field_sample(walker.at);
    match (walker.next, walker.progress_bits) {
        (Some(next), Some(bits)) => {
            between_float(at, grid::field_sample(next), f32::from_bits(bits))
        }
        (Some(next), None) => between(at, grid::field_sample(next), walker.done),
        (None, _) => milli(at),
    }
}

impl World {
    /// Every settler's trace row, in id order: settlers, then carriers,
    /// then builders and levellers by site: the view model
    /// ([`World::settler_views`]) in the trace's terms.
    pub fn trace_settlers(&self) -> Vec<SettlerRow> {
        use crate::view::{Activity as A, SettlerRef as R};
        self.settler_views()
            .into_iter()
            // TRN-CARRY-7.1a: a released carrier goes on as a settler, one
            // row throughout.
            .filter(|v| !matches!(v.activity, A::Carrier { retired: true, .. }))
            .map(|v| {
                // SIM-START-4.2: the settler's object number, as the
                // original's trace names its settlers.
                let id = v.object.to_string();
                // A fine step by its float32 progress where the game keeps
                // one (TRN-MOVE-2.1, 2.5).
                let step_fine = |s: &crate::view::Step| match (s.next, s.progress_bits) {
                    (Some(next), Some(bits)) => between_float(s.at, next, f32::from_bits(bits)),
                    (Some(next), None) => between(
                        s.at,
                        next,
                        (u64::from(s.done) * 1_000_000 / u64::from(s.total)) as u32,
                    ),
                    (None, _) => milli(s.at),
                };
                let (fine, mut action) = match (v.id, &v.fine) {
                    (R::Builder(_), Some(f)) => (step_fine(&f.step), Action::Stand),
                    // His walks between the fields (ECO-HQ-4.3a); on his
                    // field he stands on its sample.
                    (R::Leveller(_), _) => (
                        walker_fine(&v.walk),
                        if v.has_walk {
                            Action::Walk
                        } else {
                            Action::Stand
                        },
                    ),
                    (_, Some(f)) => (
                        step_fine(&f.step),
                        if f.step.next.is_some() || v.has_walk {
                            Action::Walk
                        } else {
                            Action::Stand
                        },
                    ),
                    (_, None) => (
                        walker_fine(&v.walk),
                        if v.has_walk {
                            Action::Walk
                        } else {
                            Action::Stand
                        },
                    ),
                };
                match (v.id, v.request) {
                    // A builder walks while his step moves, else stands or
                    // hammers.
                    (R::Builder(_), r) => {
                        action = match r {
                            Some(r) => r,
                            None if v.has_walk => Action::Walk,
                            None => Action::Stand,
                        }
                    }
                    (R::Leveller(_), r) => action = r.unwrap_or(action),
                    (_, Some(r)) => action = r,
                    (_, None) => {}
                }
                if v.activity == A::Dying {
                    action = Action::Dying;
                }
                let detail = v.detail.as_deref().map(|d| {
                    d.split(|c: char| !c.is_alphanumeric())
                        .next()
                        .unwrap_or("")
                        .to_string()
                });
                let job = match (&v.activity, detail) {
                    (A::Carrier { .. }, Some(d)) => format!("Carrier.{d}"),
                    (A::Builder { .. }, Some(d)) => format!("Builder.{d}"),
                    (A::Leveller { .. }, _) => "Leveller".into(),
                    (
                        a @ (A::Worker { .. }
                        | A::Attack { .. }
                        | A::Defend { .. }
                        | A::Specialist { .. }),
                        Some(d),
                    ) => format!("{}.{d}", variant(a)),
                    (a, _) => variant(a),
                };
                SettlerRow {
                    id,
                    profession: v.profession,
                    field: v.walk.at,
                    fine,
                    action,
                    job,
                }
            })
            .collect()
    }

    /// Every building's trace row, by field.
    pub fn trace_buildings(&self) -> Vec<BuildingRow> {
        self.buildings()
            .map(|b| {
                // TRACE-3.5a: an upgrading building keeps its own type and
                // its finished state; the upgrade's construction is not shown.
                let site = self
                    .sites
                    .get(&b.at)
                    .filter(|_| !self.upgrades.contains_key(&b.at));
                let (state, progress) = match site {
                    // TOOL-trace: the f32 progress x 1000, rounded down.
                    Some(site) if !site.levelled() => (
                        "levelling",
                        ((f64::from(site.levelling()) * 1000.0) as u32).min(1000),
                    ),
                    Some(site) => {
                        // 1 - the part's remaining work (float32), x 1000,
                        // rounded down (docs/trace-format.md).
                        let (frame, main) = site.remaining();
                        let (left, state) = if site.frame_done() {
                            (main, "main")
                        } else {
                            (frame, "frame")
                        };
                        (state, thousandths_done(left))
                    }
                    None => ("finished", 1000),
                };
                BuildingRow {
                    building_type: b.building_type,
                    field: b.at,
                    state,
                    progress,
                }
            })
            .collect()
    }
}

/// A construction part's thousandths done from its remaining work: 1 - the
/// float32 remainder, x 1000, rounded down (docs/trace-format.md).
fn thousandths_done(left: f32) -> u32 {
    (((1.0 - f64::from(left)) * 1000.0) as u32).min(1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ECO-HQ T-build-1: the farm's frame remainders 0.015152434 and
    // 9.171199e-07 and the main remainder 0.9969697 of E-trace-002 (trace
    // ticks 8260, 8270, 8280) are 984, 999 and 3 thousandths done.
    // spec: ECO-HQ-4.6, ECO-HQ-4.6a (T-build-1)
    #[test]
    fn a_parts_progress_is_one_minus_its_float_remainder() {
        assert_eq!(thousandths_done(0.015152434), 984);
        assert_eq!(thousandths_done(9.171199e-07), 999);
        assert_eq!(thousandths_done(0.9969697), 3);
        assert_eq!((thousandths_done(1.0), thousandths_done(0.0)), (0, 1000));
    }

    // spec: none (trace positions, docs/trace-format.md)
    #[test]
    fn positions_between_samples_are_in_thousandths() {
        assert_eq!(between((4, 8), (6, 8), 500_000), (5000, 8000));
        assert_eq!(between((4, 8), (2, 12), 250_000), (3500, 9000));
        assert_eq!(milli((3, 7)), (3000, 7000));
    }

    // Q-241: a field step's position from the float32 progress, in double
    // precision: the computer player's first constructor leaving the castle
    // door (139, 52) toward (139, 51), samples (556, 208) -> (558, 204), at
    // trace ticks 178 to 180 of E-trace-002.
    // spec: TRN-MOVE-2.1, TRN-MOVE-2.2 (Q-241)
    #[test]
    fn field_steps_follow_the_float_progress() {
        let rate = 0.045f32;
        let mut progress = 0.0f32;
        let mut rows = Vec::new();
        for _ in 0..3 {
            progress += rate;
            rows.push(between_float((556, 208), (558, 204), progress));
        }
        assert_eq!(rows, [(556090, 207819), (556180, 207639), (556270, 207459)]);
    }

    // spec: none (the trace's job column)
    #[test]
    fn variants_are_named_by_their_debug_form() {
        assert_eq!(variant(&crate::CarrierTask::Idle), "Idle");
        assert_eq!(variant(&Work::AtSpot { spot: (2, 2) }), "AtSpot");
    }
}
