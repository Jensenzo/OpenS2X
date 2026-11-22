//! Settlers walking from field to field (`specs/TRN-settler-movement.md`).
//!
//! Speeds come from the rules in millionths of a step per tick ([`ONE`] is a
//! whole step, so the shipped speed of 0.045 is 45 000). A walker keeps its
//! step progress and its rate as the 32-bit floats the original keeps
//! (TRN-MOVE-2.1 to 2.3, Q-241), stored as their bits.

use std::collections::VecDeque;

use crate::grid::{self, Point};
use crate::map::{state, Terrain};

/// A whole step of progress.
pub const ONE: u32 = 1_000_000;

/// TRN-MOVE-1.3: slope factor in percent for a step from `a` to `b`.
pub fn slope_factor_percent(terrain: &Terrain, a: Point, b: Point) -> u32 {
    slope_factor_heights(terrain, a, b, terrain.height_at(a), terrain.height_at(b))
}

/// [`slope_factor_percent`] with the two fields' heights given.
fn slope_factor_heights(terrain: &Terrain, a: Point, b: Point, ha: i32, hb: i32) -> u32 {
    if terrain.has(a, state::WATER) || terrain.has(b, state::WATER) {
        return 100;
    }
    let rise = i64::from(hb) - i64::from(ha);
    match grid::height_class(rise) {
        i8::MIN..=0 => 100,
        1 => 80,
        2 => 60,
        _ => 40,
    }
}

/// TRN-MOVE-1.2, Q-241: the float32 progress per tick of a step from `a` to
/// `b` at `base_speed` (millionths of a step per tick): `1 / (distance /
/// speed)`, the speed the base speed times the slope factor (TRN-MOVE-1.3)
/// when `slopes` (settlers; not animals, ECO-FOOD-5.12).
pub fn step_rate(terrain: &Terrain, base_speed: u32, a: Point, b: Point, slopes: bool) -> f32 {
    let distance = grid::distance(a, b).max(1) as f32;
    let mut speed = base_speed as f32 / ONE as f32;
    if slopes {
        speed *= slope_factor_percent(terrain, a, b) as f32 / 100.0;
    }
    1.0 / (distance / speed)
}

/// TRN-MOVE-2.1a: [`step_rate`] for a settler with the heights of `a` and
/// `b` given (those at the walk's order).
fn step_rate_heights(
    terrain: &Terrain,
    base_speed: u32,
    a: Point,
    b: Point,
    ha: i32,
    hb: i32,
) -> f32 {
    let distance = grid::distance(a, b).max(1) as f32;
    let speed =
        base_speed as f32 / ONE as f32 * slope_factor_heights(terrain, a, b, ha, hb) as f32 / 100.0;
    1.0 / (distance / speed)
}

/// A settler's position and walk.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walker {
    /// The field the settler is on, or is leaving while `next` is set.
    pub at: Point,
    /// The field of the step in progress.
    pub next: Option<Point>,
    /// Progress of the step in progress, a float32 from 0 to 1 (its bits).
    pub progress_bits: u32,
    /// Progress per tick of the step in progress, a float32 (its bits).
    pub rate_bits: u32,
    /// Fields still to walk after `next`.
    pub path: VecDeque<Point>,
    /// A step was refused (TRN-CARRY-3.2): the settler stopped and waits
    /// with the rest of its walk.
    pub held: bool,
    /// ECO-WORK-3.8b, 3.8c: the fields, in walking order, where the walk so
    /// far ends as a walk of its own (the door step, the way to the own
    /// flag): arriving there, the excess is dropped and the next step starts
    /// at progress 0, advancing from the next tick.
    pub pauses: Vec<Point>,
    /// TRN-FREE-6.5: the destination of the road walk in this walk (a flag,
    /// or for a carrier a field of its road): at every flag of the player
    /// reached before it the route there is searched again. Still set when
    /// the walk has ended before it (6.5 e).
    pub road_to: Option<Point>,
    /// TRN-MOVE-2.1a: the heights of the field it stands on and of its first
    /// field as they were when the walk was ordered, kept when a height
    /// changes before the first step starts (the step is rated from them).
    pub first_heights: Option<(i32, i32)>,
}

impl Walker {
    pub fn new(at: Point) -> Self {
        Self {
            at,
            next: None,
            progress_bits: 0,
            rate_bits: 0,
            path: VecDeque::new(),
            held: false,
            pauses: Vec::new(),
            road_to: None,
            first_heights: None,
        }
    }

    /// TRN-MOVE-2.1a: a walk ordered and not yet stepping (its first step
    /// still to start) whose first step is between `at` and the field ahead.
    pub fn first_step(&self) -> Option<(Point, Point)> {
        if self.next.is_some() || self.held {
            return None;
        }
        self.path.front().map(|&f| (self.at, f))
    }

    /// The step's progress, 0 to 1.
    pub fn progress(&self) -> f32 {
        f32::from_bits(self.progress_bits)
    }

    /// TRN-MOVE-2.2: the next movement completes the step in progress
    /// (progress plus rate greater than a whole step).
    pub fn completes_step(&self) -> bool {
        self.next.is_some() && self.progress() + f32::from_bits(self.rate_bits) > 1.0
    }

    /// The step's progress in millionths, truncated.
    pub fn progress_millionths(&self) -> u32 {
        (f64::from(self.progress()) * f64::from(ONE)) as u32
    }

    /// The step's progress per tick in millionths, truncated.
    pub fn speed_millionths(&self) -> u32 {
        (f64::from(f32::from_bits(self.rate_bits)) * f64::from(ONE)) as u32
    }

    pub fn is_walking(&self) -> bool {
        self.next.is_some() || !self.path.is_empty()
    }

    /// GFX-ANIM-3.1a: the walk's step progress is 0 or more: a step is in
    /// progress, or a walk was ordered (from the ordering tick on, before
    /// the first step) and has not been stopped at a field.
    pub fn has_walk(&self) -> bool {
        self.next.is_some() || (!self.path.is_empty() && !self.held)
    }

    /// Order a walk along `fields` (neighbours in a row, starting next to
    /// the field last reached, `at`). TRN-MOVE-2.1: it first advances in the
    /// next tick. TRN-FREE-4.3: a new walk replaces the current one at once;
    /// the step in progress is dropped and the walker restarts from `at`.
    pub fn walk(&mut self, fields: impl IntoIterator<Item = Point>) {
        self.next = None;
        self.progress_bits = 0;
        self.held = false;
        self.pauses.clear();
        self.road_to = None;
        self.first_heights = None;
        self.path = fields.into_iter().collect();
    }

    /// TRN-FREE-6.5 e: the road walk ended on a flag before its destination
    /// (a search there found no route); the walker stands there with the
    /// rest of its walk until a route is found again.
    pub fn ended_short(&self) -> bool {
        self.held && self.next.is_none() && self.road_to.is_some_and(|to| to != self.at)
    }

    /// TRN-FREE-6.5 b, c: the road walk's route from the field just reached
    /// is now `fields` (from the next field to the destination, `road_to`):
    /// it replaces the part of the path up to the destination; the rest of
    /// the walk after it stays. `None` (no route): the walk ends here
    /// ([`Walker::ended_short`]).
    pub fn reroute(&mut self, fields: Option<Vec<Point>>) {
        let Some(to) = self.road_to else {
            return;
        };
        let Some(fields) = fields else {
            self.held = true;
            return;
        };
        self.held = false;
        let Some(end) = self.path.iter().position(|&f| f == to) else {
            return;
        };
        if self.path.iter().take(end + 1).eq(fields.iter()) {
            return;
        }
        self.path.drain(..=end);
        for field in fields.into_iter().rev() {
            self.path.push_front(field);
        }
    }

    /// TRN-FREE-6.5 g: the walk ends on the next flag ahead (the field of
    /// the step in progress or one of the path), with no new search there;
    /// with no flag ahead it is kept as it is.
    pub fn end_at_next(&mut self, is_flag: impl Fn(Point) -> bool) {
        if self.next.is_some_and(&is_flag) {
            self.path.clear();
        } else if let Some(i) = self.path.iter().position(|&f| is_flag(f)) {
            self.path.truncate(i + 1);
        } else {
            return;
        }
        self.road_to = None;
        self.held = false;
        let (path, next) = (&self.path, self.next);
        self.pauses.retain(|p| path.contains(p) || Some(*p) == next);
    }

    /// The field the walk ends on.
    pub fn destination(&self) -> Point {
        self.path.back().copied().or(self.next).unwrap_or(self.at)
    }

    /// One tick of movement (TRN-MOVE-2). A step into a field for which
    /// `may_enter` is false is not started: the settler stops where it is and
    /// any carry-over is dropped.
    pub fn advance(
        &mut self,
        terrain: &Terrain,
        base_speed: u32,
        may_enter: impl Fn(Point) -> bool,
    ) {
        self.advance_on(terrain, base_speed, true, may_enter, &mut |_| {});
    }

    /// [`Walker::advance`] calling `arrived` each time a step ends, with the
    /// walker on the field reached and before its next step starts
    /// (TRN-FREE-6.5 c: a road walk searches again at a flag in that tick).
    pub fn advance_with(
        &mut self,
        terrain: &Terrain,
        base_speed: u32,
        may_enter: impl Fn(Point) -> bool,
        arrived: &mut dyn FnMut(&mut Walker),
    ) {
        self.advance_on(terrain, base_speed, true, may_enter, arrived);
    }

    /// [`Walker::advance`] for an animal: slopes do not slow it
    /// (ECO-FOOD-5.12), a step's speed is its kind's speed over the step's
    /// lattice distance.
    pub fn advance_level(&mut self, terrain: &Terrain, base_speed: u32) {
        self.advance_on(terrain, base_speed, false, |_| true, &mut |_| {});
    }

    fn advance_on(
        &mut self,
        terrain: &Terrain,
        base_speed: u32,
        slopes: bool,
        may_enter: impl Fn(Point) -> bool,
        arrived: &mut dyn FnMut(&mut Walker),
    ) {
        if self.ended_short() {
            return;
        }
        if self.next.is_none() {
            // A walk from rest starts at progress 0 (TRN-MOVE-2.1).
            if !self.start_next_step(terrain, base_speed, slopes, &may_enter) {
                self.held = !self.path.is_empty();
                return;
            }
            self.progress_bits = 0;
        }
        let mut progress = self.progress() + f32::from_bits(self.rate_bits);
        // TRN-MOVE-2.2: complete when progress is greater than a whole step
        // (exactly 1.0 goes on).
        while progress > 1.0 {
            progress -= 1.0;
            self.at = self.next.take().expect("a step in progress");
            arrived(self);
            if self.ended_short() {
                // TRN-FREE-6.5 e: it stands here; a walk on starts from rest.
                if self.pauses.first() == Some(&self.at) {
                    self.pauses.remove(0);
                }
                progress = 0.0;
                break;
            }
            // TRN-MOVE-2.3: the next step starts at once with the excess.
            if !self.start_next_step(terrain, base_speed, slopes, &may_enter) {
                progress = 0.0;
                self.held = !self.path.is_empty();
            }
            // ECO-WORK-3.8b, 3.8c: a new walk from here, its first step at 0.
            if self.pauses.first() == Some(&self.at) {
                self.pauses.remove(0);
                progress = 0.0;
                break;
            }
        }
        self.progress_bits = progress.to_bits();
    }

    fn start_next_step(
        &mut self,
        terrain: &Terrain,
        base_speed: u32,
        slopes: bool,
        may_enter: &impl Fn(Point) -> bool,
    ) -> bool {
        match self.path.front() {
            Some(&field) if may_enter(field) => {
                self.path.pop_front();
                self.held = false;
                self.next = Some(field);
                // TRN-MOVE-2.1a: the walk's first step is rated from the
                // heights at the order.
                let rate = match self.first_heights.take() {
                    Some((ha, hb)) if slopes => {
                        step_rate_heights(terrain, base_speed, self.at, field, ha, hb)
                    }
                    _ => step_rate(terrain, base_speed, self.at, field, slopes),
                };
                self.rate_bits = rate.to_bits();
                true
            }
            _ => false,
        }
    }
}

/// ECO-OUT-2.3: field bits that keep a settler off a field when walking
/// over open ground: Blocked, Water, ElevationBlocked, MovementBlocked,
/// DoodadBlocked (DepositBlocked does not).
pub const NOT_WALKABLE: u32 = state::BLOCKED
    | state::WATER
    | state::ELEVATION_BLOCKED
    | state::MOVEMENT_BLOCKED
    | state::DOODAD_BLOCKED;

/// A route of the free walk (TRN-FREE-2, 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeRoute {
    /// TRN-FREE-3.3: the walker's label, the cost of the route.
    pub cost: u32,
    /// The fields after the walker's, ending with the target.
    pub fields: Vec<Point>,
}

/// TRN-FREE-2.5: the cost of the step from `q` to `n` (the search runs from
/// `n` to `q`): by the height class of `height(q) - height(n)`, -1 gives 4,
/// -2 gives 5, -3 gives 7, anything else 3; 2 more if `q` carries
/// DepositBlocked.
fn free_step_cost(terrain: &Terrain, n: Point, q: Point) -> u32 {
    let class =
        grid::height_class(i64::from(terrain.height_at(q)) - i64::from(terrain.height_at(n)));
    let cost = match class {
        -1 => 4,
        -2 => 5,
        -3 => 7,
        _ => 3,
    };
    if terrain.has(q, state::DEPOSIT_BLOCKED) {
        cost + 2
    } else {
        cost
    }
}

/// The working memory of the route search, kept between searches: a search
/// touches few fields of a large map, and allocating and clearing a field
/// array for each one would cost more than the search. A field's entry is
/// valid when its stamp is the current one.
#[derive(Default)]
struct Scratch {
    stamp: u32,
    label: Vec<u32>,
    label_stamp: Vec<u32>,
    /// The number of a field's current open-list entry, plus one.
    entry: Vec<u64>,
    entry_stamp: Vec<u32>,
    open: std::collections::BinaryHeap<std::cmp::Reverse<(u32, u64, u32)>>,
}

impl Scratch {
    fn get(&self, index: usize) -> Option<u32> {
        (self.label_stamp[index] == self.stamp).then(|| self.label[index])
    }

    fn set(&mut self, index: usize, label: u32) {
        self.label[index] = label;
        self.label_stamp[index] = self.stamp;
    }

    /// TRN-FREE-2.3 to 2.7: one search from `to` back towards `from`;
    /// whether it reached `from` before the open list ran empty, with the
    /// labels left in `self`. With `all_passable` every field may be
    /// entered (TRN-FREE-2.8). `extra` of the field stepped from is added
    /// to the cost of a step (ECO-FOOD-5.11a (a)).
    fn search(
        &mut self,
        terrain: &Terrain,
        from: Point,
        to: Point,
        all_passable: bool,
        extra: &dyn Fn(Point) -> u32,
    ) -> bool {
        self.search_with(
            terrain,
            from,
            to,
            &|q| all_passable || !terrain.has(q, NOT_WALKABLE),
            &|n, q| free_step_cost(terrain, n, q) + extra(q),
        )
    }

    /// The search of [`Scratch::search`] with its own field test
    /// (`passable`, `from` always) and step cost (`cost(n, q)`, the walker
    /// stepping from `q` to `n`): SHIP-3.1's ship route.
    fn search_with(
        &mut self,
        terrain: &Terrain,
        from: Point,
        to: Point,
        passable: &dyn Fn(Point) -> bool,
        cost: &dyn Fn(Point, Point) -> u32,
    ) -> bool {
        use std::cmp::Reverse;
        let len = terrain.len();
        if self.label.len() < len {
            self.label.resize(len, 0);
            self.label_stamp.resize(len, 0);
            self.entry.resize(len, 0);
            self.entry_stamp.resize(len, 0);
        }
        if self.stamp == u32::MAX {
            self.label_stamp.fill(0);
            self.entry_stamp.fill(0);
            self.stamp = 0;
        }
        self.stamp += 1;
        self.open.clear();
        // The open list sorted by f, first in first out among equal f (the
        // entry number). A field relabelled while it is listed gets a new
        // entry; older ones are skipped when they come up.
        let mut seq = 0u64;
        let start = terrain.index(to);
        self.set(start, 0);
        self.put(&mut seq, start, grid::distance(to, from));
        let width = terrain.width;
        while let Some(Reverse((_, number, index))) = self.open.pop() {
            let index = index as usize;
            if self.entry_stamp[index] != self.stamp || self.entry[index] != number + 1 {
                continue;
            }
            self.entry_stamp[index] = 0;
            let n = (index as u32 % width, index as u32 / width);
            let label_n = self.get(index).expect("labelled");
            for q in terrain.neighbours(n).into_iter().flatten() {
                if q != from && !passable(q) {
                    continue;
                }
                // The walker steps from `q` to `n`; ECO-FOOD-5.11a (a): the
                // extra belongs to `q`, the field stepped from.
                let g = label_n + cost(n, q);
                let index = terrain.index(q);
                if self.get(index).is_some_and(|l| l <= g) {
                    continue;
                }
                self.set(index, g);
                if q == from {
                    return true;
                }
                self.put(&mut seq, index, g + grid::distance(q, from));
            }
        }
        false
    }

    fn put(&mut self, seq: &mut u64, index: usize, f: u32) {
        self.entry[index] = *seq + 1;
        self.entry_stamp[index] = self.stamp;
        self.open.push(std::cmp::Reverse((f, *seq, index as u32)));
        *seq += 1;
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

/// TRN-FREE-2 and 3: the free walk from `from` to `to` (`from != to`) by
/// the search, without the shortcuts of TRN-FREE-2.1 and 2.2 (see
/// [`crate::World`]'s use). The route goes downhill in the labels from the
/// walker, keeping its direction while that is allowed and otherwise
/// drawing one of the allowed directions; the first direction is drawn
/// too. The draws come from `rng`, the simulation's generator (SIM-RNG-2.4:
/// settlers' movement), recorded as the caller's.
/// `None` if neither try reaches the walker.
#[track_caller]
pub fn free_walk(
    terrain: &Terrain,
    rng: &mut crate::Rng,
    from: Point,
    to: Point,
) -> Option<FreeRoute> {
    let caller = std::panic::Location::caller();
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        let found = scratch.search(terrain, from, to, false, &|_| 0)
            || scratch.search(terrain, from, to, true, &|_| 0);
        found
            .then(|| route_from_labels(terrain, rng, caller, &scratch, from, to))
            .flatten()
    })
}

/// ECO-FOOD-5.11a: an animal's walk: the free walk's search and read-out
/// with `extra` of the field stepped from added to the cost of each step
/// (5.11a (a)), and no second try with every field passable. The draws are
/// the free walk's (5.14).
#[track_caller]
pub fn animal_walk(
    terrain: &Terrain,
    rng: &mut crate::Rng,
    from: Point,
    to: Point,
    extra: &dyn Fn(Point) -> u32,
) -> Option<FreeRoute> {
    let caller = std::panic::Location::caller();
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        scratch
            .search(terrain, from, to, false, extra)
            .then(|| route_from_labels(terrain, rng, caller, &scratch, from, to))
            .flatten()
    })
}

/// SHIP-3.1 (T13): a ship's route from `from` to `to` (`from != to`): the
/// free walk's search and drawing read-out (TRN-FREE-2.3 to 3.2a, no try
/// with every field passable) with the ship's field test, Water and not
/// DoodadBlocked (its own field always), and step cost: 1 when each of the
/// six neighbours of the field stepped from lies inside the map, off its
/// border rows and columns, and is Water, not DoodadBlocked and not
/// LandingPosition; else 3. The draws are recorded as the caller's.
#[track_caller]
pub fn ship_walk(
    terrain: &Terrain,
    rng: &mut crate::Rng,
    from: Point,
    to: Point,
) -> Option<FreeRoute> {
    let caller = std::panic::Location::caller();
    let open_water =
        |f: Point| terrain.has(f, state::WATER) && !terrain.has(f, state::DOODAD_BLOCKED);
    let inner =
        |f: Point| f.0 > 0 && f.1 > 0 && f.0 + 1 < terrain.width && f.1 + 1 < terrain.height;
    let cost = |_: Point, q: Point| {
        let calm = terrain.neighbours(q).iter().all(|n| {
            n.is_some_and(|n| inner(n) && open_water(n) && !terrain.has(n, state::LANDING_POSITION))
        });
        if calm {
            1
        } else {
            3
        }
    };
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        scratch
            .search_with(terrain, from, to, &open_water, &cost)
            .then(|| route_from_labels(terrain, rng, caller, &scratch, from, to))
            .flatten()
    })
}

/// AI-CON-8.3 c: the free walk's path from `from` to `to` by the first try
/// alone (TRN-FREE-2.3 to 2.5, no try with every field passable), read out
/// without draws: the first direction is 0, and where the kept direction is
/// not allowed the lowest allowed one is taken (TRN-FREE-3.1, 3.2). `None`
/// if the first try does not reach `from`.
pub fn free_walk_without_draws(terrain: &Terrain, from: Point, to: Point) -> Option<FreeRoute> {
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        if !scratch.search(terrain, from, to, false, &|_| 0) {
            return None;
        }
        read_labels(terrain, &mut |_| 0, true, &scratch, from, to)
    })
}

/// AI-CON-5.4b: the free walk's path from `from` to `to` by the first try
/// alone, as [`free_walk_without_draws`], read out taking at every field
/// the lowest direction among the allowed ones (no kept direction, no
/// draws): the computer player's site search counts its steps. `None` if
/// the first try does not reach `from`.
pub fn free_walk_lowest_directions(terrain: &Terrain, from: Point, to: Point) -> Option<FreeRoute> {
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        if !scratch.search(terrain, from, to, false, &|_| 0) {
            return None;
        }
        read_labels(terrain, &mut |_| 0, false, &scratch, from, to)
    })
}

/// TRN-FREE-3: the route down the labels.
fn route_from_labels(
    terrain: &Terrain,
    rng: &mut crate::Rng,
    caller: &'static std::panic::Location<'static>,
    labels: &Scratch,
    from: Point,
    to: Point,
) -> Option<FreeRoute> {
    read_labels(
        terrain,
        &mut |n| rng.draw_for(caller) % n,
        true,
        labels,
        from,
        to,
    )
}

/// TRN-FREE-3.2, 3.2a: the route down the labels, `pick(n)` choosing among
/// `n` directions: the first direction (of 6), and among the allowed ones
/// where the kept direction is not allowed. Without `keep`, the lowest
/// allowed direction at every field (AI-CON-5.4b).
fn read_labels(
    terrain: &Terrain,
    pick: &mut dyn FnMut(u32) -> u32,
    keep: bool,
    labels: &Scratch,
    from: Point,
    to: Point,
) -> Option<FreeRoute> {
    let label = |f: Point| labels.get(terrain.index(f));
    let cost = label(from)?;
    let mut direction = pick(6) as u8;
    let mut at = from;
    let mut fields = Vec::new();
    while at != to {
        let here = label(at).expect("on the route");
        let neighbours = terrain.neighbours(at);
        let lower: Vec<(u8, u32)> = (0u8..6)
            .filter_map(|d| {
                let l = label(neighbours[usize::from(d)]?)?;
                (l < here).then_some((d, l))
            })
            .collect();
        let least = lower.iter().map(|&(_, l)| l).min()?;
        let allowed: Vec<u8> = lower
            .iter()
            .filter(|&&(_, l)| l == least)
            .map(|&(d, _)| d)
            .collect();
        if !keep {
            direction = allowed[0];
        } else if !allowed.contains(&direction) {
            direction = allowed[pick(allowed.len() as u32) as usize];
        }
        at = neighbours[usize::from(direction)].expect("an allowed neighbour");
        fields.push(at);
    }
    Some(FreeRoute { cost, fields })
}

/// The number of steps of a free walk from `from` to `to` over walkable
/// fields only (the first try of TRN-FREE-2.8), or `None` if there is none.
/// It draws from a private generator, so the game's own is untouched: the
/// route may differ in length from the one a real walk takes (TRN-FREE-3.3).
pub fn free_walk_steps(terrain: &Terrain, from: Point, to: Point) -> Option<usize> {
    if from == to {
        return Some(0);
    }
    if grid::distance(from, to) == 1 {
        return Some(1);
    }
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        if !scratch.search(terrain, from, to, false, &|_| 0) {
            return None;
        }
        let mut rng = crate::Rng::new(1);
        Some(
            route_from_labels(
                terrain,
                &mut rng,
                std::panic::Location::caller(),
                &scratch,
                from,
                to,
            )?
            .fields
            .len(),
        )
    })
}

/// Ticks a walk along `fields` from `from` takes (TRN-MOVE-2, TRN-FREE-4.1):
/// from the order until the walker stands on the last field.
pub fn walk_ticks(terrain: &Terrain, base_speed: u32, from: Point, fields: &[Point]) -> u32 {
    let mut walker = Walker::new(from);
    walker.walk(fields.iter().copied());
    let mut ticks = 0;
    while walker.is_walking() {
        ticks += 1;
        walker.advance(terrain, base_speed, |_| true);
    }
    ticks
}

/// MAP-GRID-2.2 boundaries of the 60 degree sectors, as unit vectors scaled
/// by 1e9: -150, -90, -30, 31, 91, 151 degrees (angles cut toward zero, so
/// the positive boundaries are one degree further).
const SECTOR_BOUNDS: [(i64, i64); 6] = [
    (-866_025_404, -500_000_000),
    (0, -1_000_000_000),
    (866_025_404, -500_000_000),
    (857_167_301, 515_038_075),
    (-17_452_406, 999_847_695),
    (-874_619_707, 484_809_620),
];

/// MAP-GRID-2.2: the direction from lattice point `a` toward `b` (fields or
/// samples), by the 60 degree sector of the angle. Integer only: the
/// offset is taken in half steps and compared with the sector boundaries by
/// cross products; no offset of whole and half steps lies exactly on an
/// irrational boundary, and the two exact ones (straight up or down the
/// rows) are decided as the spec's truncation does.
pub fn direction_toward(a: Point, b: Point) -> u8 {
    let x = 2 * (i64::from(b.0) - i64::from(a.0)) + i64::from(b.1 % 2) - i64::from(a.1 % 2);
    let y = 2 * (i64::from(b.1) - i64::from(a.1));
    if y == 0 {
        return if x > 0 { 0 } else { 3 };
    }
    let cross =
        |(bx, by): (i64, i64)| i128::from(bx) * i128::from(y) - i128::from(by) * i128::from(x);
    let [m150, m90, m30, p31, p91, p151] = SECTOR_BOUNDS.map(cross);
    if m150 > 0 && m90 <= 0 {
        2
    } else if m90 > 0 && m30 <= 0 {
        1
    } else if m30 > 0 && p31 < 0 {
        0
    } else if p31 >= 0 && p91 < 0 {
        5
    } else if p91 >= 0 && p151 < 0 {
        4
    } else {
        3
    }
}

/// TRN-FREE-5.1, 5.2: the lattice steps of the fine approach from the
/// sample `walker` to an object at the sample `object`: the straight line
/// from the object towards the walker, walked in reverse, stopping short by
/// `elevation` steps.
pub fn fine_approach(
    terrain: &Terrain,
    walker: Point,
    object: Point,
    elevation: u32,
) -> Vec<Point> {
    let (columns, rows) = terrain.sample_size();
    let mut line = vec![object];
    let mut at = object;
    while at != walker && line.len() <= 64 {
        let direction = grid::Direction::new(direction_toward(at, walker)).expect("0 to 5");
        let Some(next) = grid::neighbour(at, direction, columns, rows) else {
            break;
        };
        at = next;
        line.push(at);
    }
    // From the walker's end towards the object, without the walker's own
    // sample and the last `elevation` steps.
    line.reverse();
    let steps = line
        .len()
        .saturating_sub(1)
        .saturating_sub(elevation as usize);
    line[1..=steps].to_vec()
}

/// TRN-MOVE-2.1, 2.5: the float32 progress per tick of a step on the sample
/// lattice from `a` to `b`: `2 x base speed x factor`, the factor by the
/// height class of the rise between the samples (1 gives 0.8, 2 gives 0.6,
/// 3 and 4 give 0.4, others 1.0).
pub(crate) fn lattice_rate(terrain: &Terrain, base_speed: u32, a: Point, b: Point) -> f32 {
    let rise = i64::from(terrain.sample_height(b)) - i64::from(terrain.sample_height(a));
    let percent: u32 = match grid::height_class(rise) {
        1 => 80,
        2 => 60,
        3 | 4 => 40,
        _ => 100,
    };
    2.0 * (base_speed as f32 / ONE as f32) * (percent as f32 / 100.0)
}

/// TRN-MOVE-2.1 to 2.3, 2.5: a walk on the sample lattice from `from`
/// along `samples`, `stop` ticks into it (or to its end): the ticks walked,
/// the sample last reached, the next one and the float32 progress toward
/// it. Each tick adds the step's rate; the step is done when the progress
/// passes 1.0 (exactly 1.0 goes on) and the next carries the excess.
fn lattice_walk(
    terrain: &Terrain,
    base_speed: u32,
    from: Point,
    samples: &[Point],
    stop: u32,
) -> (u32, Point, Option<Point>, f32) {
    let mut ticks = 0;
    let mut progress = 0.0f32;
    let mut at = from;
    for &next in samples {
        let rate = lattice_rate(terrain, base_speed, at, next);
        while progress <= 1.0 {
            if ticks == stop {
                return (ticks, at, Some(next), progress);
            }
            progress += rate;
            ticks += 1;
        }
        progress -= 1.0;
        at = next;
    }
    (ticks, at, None, 0.0)
}

/// TRN-MOVE-2.5 (T14), TRN-FREE-5.1: a walk on the sample lattice under
/// way, along the straight line from `from` to `to`. Each step's rate is
/// set from the heights when the step starts (the first when the walk is
/// ordered, a later one in the tick the step before ends), so a height
/// change on the way changes only the steps started after it; the progress
/// passing 1.0 carries its excess into the next step, as in
/// [`lattice_walk_ticks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatticeWalk {
    pub from: Point,
    pub to: Point,
    /// Steps of the line already walked.
    pub done: u32,
    pub progress_bits: u32,
    pub rate_bits: u32,
}

impl LatticeWalk {
    /// A walk ordered now: the first step's rate from the heights now.
    pub fn new(terrain: &Terrain, base_speed: u32, from: Point, to: Point) -> Self {
        let mut walk = Self {
            from,
            to,
            done: 0,
            progress_bits: 0,
            rate_bits: 0,
        };
        if let Some(&next) = walk.steps(terrain).first() {
            walk.rate_bits = lattice_rate(terrain, base_speed, from, next).to_bits();
        }
        walk
    }

    /// No walk: standing on `at`.
    pub fn standing(at: Point) -> Self {
        Self {
            from: at,
            to: at,
            done: 0,
            progress_bits: 0,
            rate_bits: 0,
        }
    }

    /// The samples of the line after `from` (TRN-FREE-5.1).
    pub fn steps(&self, terrain: &Terrain) -> Vec<Point> {
        fine_approach(terrain, self.from, self.to, 0)
    }

    pub fn is_done(&self, terrain: &Terrain) -> bool {
        self.done as usize >= self.steps(terrain).len()
    }

    /// One tick of the walk.
    pub fn advance(&mut self, terrain: &Terrain, base_speed: u32) {
        let steps = self.steps(terrain);
        self.advance_along(terrain, base_speed, &steps);
    }

    /// The sample last reached, the next one and the float32 progress
    /// toward it (as [`lattice_walk_at`]).
    pub fn position(&self, terrain: &Terrain) -> (Point, Option<Point>, f32) {
        self.position_along(&self.steps(terrain))
    }

    /// The ticks left, if the heights stay as they are.
    pub fn ticks_left(&self, terrain: &Terrain, base_speed: u32) -> u32 {
        self.ticks_left_along(terrain, base_speed, &self.steps(terrain))
    }

    /// TRN-FREE-5.3 (T48): a walk from `from` along the given samples (an
    /// outdoor worker's fine approach stopping short of its object, or his
    /// fine walk back), ordered now: the first step's rate from the heights
    /// now. Driven by the `_along` methods with the same samples.
    pub fn along(terrain: &Terrain, base_speed: u32, from: Point, steps: &[Point]) -> Self {
        let mut walk = Self {
            from,
            to: steps.last().copied().unwrap_or(from),
            done: 0,
            progress_bits: 0,
            rate_bits: 0,
        };
        if let Some(&next) = steps.first() {
            walk.rate_bits = lattice_rate(terrain, base_speed, from, next).to_bits();
        }
        walk
    }

    /// One tick of the walk along `steps`: a step's rate is set from the
    /// heights in the tick it starts, the excess carried.
    pub fn advance_along(&mut self, terrain: &Terrain, base_speed: u32, steps: &[Point]) {
        let Some(&next) = steps.get(self.done as usize) else {
            return;
        };
        let mut progress = f32::from_bits(self.progress_bits) + f32::from_bits(self.rate_bits);
        if progress > 1.0 {
            progress -= 1.0;
            self.done += 1;
            match steps.get(self.done as usize) {
                Some(&after) => {
                    self.rate_bits = lattice_rate(terrain, base_speed, next, after).to_bits();
                }
                None => progress = 0.0,
            }
        }
        self.progress_bits = progress.to_bits();
    }

    /// [`LatticeWalk::position`] along `steps`.
    pub fn position_along(&self, steps: &[Point]) -> (Point, Option<Point>, f32) {
        let done = self.done as usize;
        let at = if done == 0 {
            self.from
        } else {
            steps[(done - 1).min(steps.len().saturating_sub(1))]
        };
        match steps.get(done) {
            Some(&next) => (at, Some(next), f32::from_bits(self.progress_bits)),
            None => (at, None, 0.0),
        }
    }

    /// [`LatticeWalk::ticks_left`] along `steps`: the step under way at its
    /// own rate, the later ones at the heights now.
    pub fn ticks_left_along(&self, terrain: &Terrain, base_speed: u32, steps: &[Point]) -> u32 {
        let mut walk = *self;
        let mut ticks = 0;
        while (walk.done as usize) < steps.len() {
            walk.advance_along(terrain, base_speed, steps);
            ticks += 1;
        }
        ticks
    }
}

/// TRN-MOVE-2.5, TRN-FREE-5.3: ticks of a walk on the sample lattice from
/// `from` along `samples`, the float32 progress carried over between the
/// steps.
pub fn lattice_walk_ticks(
    terrain: &Terrain,
    base_speed: u32,
    from: Point,
    samples: &[Point],
) -> u32 {
    lattice_walk(terrain, base_speed, from, samples, u32::MAX).0
}

/// TRN-MOVE-2.5, TRN-FREE-5.3: where a lattice walk of
/// [`lattice_walk_ticks`] is after `elapsed` of its ticks: the sample last
/// reached, the next one and the float32 progress toward it. At or past the
/// end, the last sample with no next.
pub fn lattice_walk_at(
    terrain: &Terrain,
    base_speed: u32,
    from: Point,
    samples: &[Point],
    elapsed: u32,
) -> (Point, Option<Point>, f32) {
    let (_, at, next, progress) = lattice_walk(terrain, base_speed, from, samples, elapsed);
    (at, next, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{flat, set, set_height};

    const SPEED: u32 = 45_000;

    /// Ticks from the order until the walk along `path` ends: the order is
    /// given in tick 0 and movement first advances in tick 1.
    fn ticks_to_walk(terrain: &Terrain, start: Point, path: &[Point]) -> u32 {
        let mut walker = Walker::new(start);
        walker.walk(path.iter().copied());
        let mut tick = 0;
        while walker.is_walking() {
            tick += 1;
            walker.advance(terrain, SPEED, |_| true);
            assert!(tick < 10_000);
        }
        assert_eq!(walker.at, *path.last().unwrap());
        tick
    }

    fn straight(from: Point, steps: u32) -> Vec<Point> {
        (1..=steps).map(|i| (from.0 + i, from.1)).collect()
    }

    // TRN-MOVE T1, T2, T3, and TRN-MOVE-2.4
    // spec: TRN-MOVE-2.1, TRN-MOVE-2.2, TRN-MOVE-2.4 (T1 to T3)
    #[test]
    fn level_walks() {
        let terrain = flat(20, 4);
        assert_eq!(ticks_to_walk(&terrain, (0, 1), &straight((0, 1), 1)), 23);
        assert_eq!(ticks_to_walk(&terrain, (0, 1), &straight((0, 1), 2)), 45);
        assert_eq!(ticks_to_walk(&terrain, (0, 1), &straight((0, 1), 3)), 67);
        assert_eq!(ticks_to_walk(&terrain, (0, 1), &straight((0, 1), 6)), 134);
        // 9 steps: 200 x 0.045 is exactly 9, but the float32 0.045 is a
        // hair above it, so the ninth step ends in tick 200 (Q-241, Q-243).
        assert_eq!(ticks_to_walk(&terrain, (0, 1), &straight((0, 1), 9)), 200);
    }

    // TRN-MOVE T2: the first step ends in tick 23 with 0.035 carried over.
    // spec: TRN-MOVE-2.1, TRN-MOVE-2.2, TRN-MOVE-2.3 (T2)
    #[test]
    fn carry_over() {
        let terrain = flat(10, 4);
        let mut walker = Walker::new((0, 1));
        walker.walk(straight((0, 1), 2));
        for _ in 0..23 {
            walker.advance(&terrain, SPEED, |_| true);
        }
        assert_eq!(walker.at, (1, 1));
        assert_eq!(walker.next, Some((2, 1)));
        assert_eq!(walker.progress_millionths(), 35_000);
    }

    /// Ticks for one step from `a` to `b` from rest.
    fn one_step(terrain: &Terrain, a: Point, b: Point) -> u32 {
        ticks_to_walk(terrain, a, &[b])
    }

    fn two_fields(ha: i32, hb: i32) -> Terrain {
        let mut terrain = flat(6, 4);
        set_height(&mut terrain, (1, 1), ha);
        set_height(&mut terrain, (2, 1), hb);
        terrain
    }

    // TRN-MOVE T4 to T6
    // spec: TRN-MOVE-1.3 (T4 to T6)
    #[test]
    fn climbing_is_slower() {
        let (a, b) = ((1, 1), (2, 1));
        assert_eq!(slope_factor_percent(&two_fields(1000, 1200), a, b), 100);
        let terrain = two_fields(1000, 1201);
        assert_eq!(slope_factor_percent(&terrain, a, b), 80);
        assert_eq!(one_step(&terrain, a, b), 28);
        assert_eq!(slope_factor_percent(&terrain, b, a), 100);
        assert_eq!(one_step(&terrain, b, a), 23);
        for (hb, percent, ticks) in [(1601, 60, 38), (2201, 40, 56), (3401, 40, 56)] {
            let terrain = two_fields(1000, hb);
            assert_eq!(slope_factor_percent(&terrain, a, b), percent, "{hb}");
            assert_eq!(one_step(&terrain, a, b), ticks, "{hb}");
        }
    }

    // TRN-MOVE T7
    // spec: TRN-MOVE-1.3 (T7)
    #[test]
    fn water_steps_are_never_slowed() {
        for water in [(1, 1), (2, 1)] {
            let mut terrain = two_fields(-500, 3000);
            set(&mut terrain, water, state::WATER);
            assert_eq!(slope_factor_percent(&terrain, (1, 1), (2, 1)), 100);
        }
    }

    // TRN-MOVE T9, T10
    // spec: TRN-MOVE-1.3, TRN-MOVE-1.4 (T9, T10)
    #[test]
    fn two_step_slopes() {
        let mut climb = flat(8, 4);
        set_height(&mut climb, (2, 1), 400);
        set_height(&mut climb, (3, 1), 800);
        assert_eq!(ticks_to_walk(&climb, (1, 1), &[(2, 1), (3, 1)]), 56);
        assert_eq!(ticks_to_walk(&climb, (3, 1), &[(2, 1), (1, 1)]), 45);

        let mut ridge = flat(8, 4);
        set_height(&mut ridge, (2, 1), 400);
        let there = ticks_to_walk(&ridge, (1, 1), &[(2, 1), (3, 1)]);
        let back = ticks_to_walk(&ridge, (3, 1), &[(2, 1), (1, 1)]);
        for ticks in [there, back] {
            assert!((50..=51).contains(&ticks), "{ticks}");
        }
        let mut dip = flat(8, 4);
        set_height(&mut dip, (2, 1), -400);
        let ticks = ticks_to_walk(&dip, (1, 1), &[(2, 1), (3, 1)]);
        assert!((50..=51).contains(&ticks), "{ticks}");
    }

    fn route(terrain: &Terrain, seed: u64, from: Point, to: Point) -> FreeRoute {
        free_walk(terrain, &mut crate::Rng::new(seed as u32), from, to).unwrap()
    }

    // TRN-FREE T9: 6 fields in a straight line on flat open ground: label
    // 18, 6 steps, 134 ticks, whatever the draws.
    // spec: TRN-FREE-2.3, TRN-FREE-3.1, TRN-FREE-3.3 (TRN-FREE T9)
    #[test]
    fn t9_straight_free_walk() {
        let terrain = flat(20, 8);
        for seed in 1..20 {
            let r = route(&terrain, seed, (2, 3), (8, 3));
            assert_eq!(r.cost, 18);
            assert_eq!(r.fields.len(), 6);
            assert_eq!(*r.fields.last().unwrap(), (8, 3));
            assert_eq!(walk_ticks(&terrain, SPEED, (2, 3), &r.fields), 134);
        }
    }

    // TRN-FREE-6.5 b, c, e: a new route from the flag reached replaces the
    // walk up to the road walk's destination and keeps the rest after it;
    // none ends the walk there, the walker standing with the rest.
    // spec: TRN-FREE-6.5
    #[test]
    fn a_new_route_replaces_the_walk_up_to_the_destination() {
        let mut walker = Walker::new((5, 5));
        walker.walk([(6, 5), (7, 5), (8, 5), (9, 5)]);
        walker.road_to = Some((8, 5));
        walker.reroute(Some(vec![(6, 5), (7, 5), (8, 5)]));
        assert_eq!(
            Vec::from(walker.path.clone()),
            [(6, 5), (7, 5), (8, 5), (9, 5)]
        );
        walker.reroute(Some(vec![(5, 4), (6, 4), (7, 4), (8, 5)]));
        assert_eq!(
            Vec::from(walker.path.clone()),
            [(5, 4), (6, 4), (7, 4), (8, 5), (9, 5)]
        );
        assert!(!walker.ended_short());
        walker.reroute(None);
        assert!(walker.ended_short() && walker.is_walking() && !walker.has_walk());
        walker.reroute(Some(vec![(6, 5), (7, 5), (8, 5)]));
        assert!(!walker.ended_short() && walker.has_walk());
    }

    // AI-CON-8.3 c: the computer player's free walk path without draws
    // starts with direction 0 and takes the lowest allowed direction where
    // the kept one is not allowed: to a field 3 steps in direction 1 and 3
    // in direction 2 away it goes 1, 1, 1, then 2, 2, 2.
    // spec: AI-CON-8.3, TRN-FREE-3.2
    #[test]
    fn the_free_walk_without_draws_keeps_the_lowest_allowed_direction() {
        let terrain = flat(30, 30);
        let from = (10, 10);
        let mut expected = Vec::new();
        let mut to = from;
        for d in [1, 1, 1, 2, 2, 2] {
            to = terrain.neighbour(to, d).unwrap();
            expected.push(to);
        }
        let route = free_walk_without_draws(&terrain, from, to).unwrap();
        assert_eq!(route.fields, expected);
        assert_eq!(route.cost, 18);
        // A neighbour: one step.
        let next = terrain.neighbour(from, 4).unwrap();
        assert_eq!(
            free_walk_without_draws(&terrain, from, next)
                .unwrap()
                .fields,
            [next]
        );
    }

    // AI-CON-5.4b, T56 (b): the site search's read-out takes the lowest
    // allowed direction at every field, with no kept direction. From s the
    // labels lead down to f (direction 5), whose lowest lower neighbours a
    // (direction 0) and b (direction 5) tie at 7: b lies low, its one step
    // up to p costs 7 (class -3); a lies high, its way goes over x, 3 + 4.
    // Keeping direction 5 goes s, f, b, p (3 steps); the lowest direction
    // goes s, f, a, x, p (4).
    // spec: AI-CON-5.4b, AI-CON-8.3, TRN-FREE-3.2
    #[test]
    fn t56_the_site_route_takes_the_lowest_direction_at_every_field() {
        let mut terrain = flat(30, 30);
        let (s, f, a, b, x, p) = ((9, 9), (10, 10), (11, 10), (10, 11), (11, 11), (11, 12));
        for (field, h) in [(p, 1300), (x, 1000), (a, 900)] {
            set_height(&mut terrain, field, h);
        }
        assert_eq!(terrain.neighbour(s, 5), Some(f));
        assert_eq!(terrain.neighbour(f, 0), Some(a));
        assert_eq!(terrain.neighbour(f, 5), Some(b));
        let kept = free_walk_without_draws(&terrain, s, p).unwrap();
        assert_eq!(kept.fields, [f, b, p]);
        let lowest = free_walk_lowest_directions(&terrain, s, p).unwrap();
        assert_eq!(lowest.fields, [f, a, x, p]);
        assert_eq!((kept.cost, lowest.cost), (13, 13));
    }

    // TRN-MOVE T14 (TRN-MOVE-2.5): a lattice walk under way sets each
    // step's rate when the step starts. With the heights unchanged it is the
    // walk of lattice_walk_ticks, tick for tick; a rise put on the second
    // step after the first started slows that step only, and changes
    // nothing already walked.
    // spec: TRN-MOVE-2.5 (TRN-MOVE T14), TRN-FREE-5.1
    #[test]
    fn t14_a_lattice_step_takes_its_rate_when_it_starts() {
        let mut terrain = flat(20, 20);
        let (from, to) = ((8, 8), (8, 20));
        let steps = fine_approach(&terrain, from, to, 0);
        assert!(steps.len() >= 3);
        let mut walk = LatticeWalk::new(&terrain, SPEED, from, to);
        let total = lattice_walk_ticks(&terrain, SPEED, from, &steps);
        for elapsed in 0..=total {
            let (at, next, progress) = lattice_walk_at(&terrain, SPEED, from, &steps, elapsed);
            assert_eq!(walk.position(&terrain), (at, next, progress), "{elapsed}");
            walk.advance(&terrain, SPEED);
        }
        assert!(walk.is_done(&terrain));
        // A rise on the second step, put there while the first is walked.
        let mut walk = LatticeWalk::new(&terrain, SPEED, from, to);
        walk.advance(&terrain, SPEED);
        let before = walk.position(&terrain);
        let second = crate::grid::sample_field_truncating(steps[1]);
        set_height(&mut terrain, second, 400);
        assert_eq!(walk.position(&terrain), before, "nothing walked changes");
        let mut ticks = 1;
        while walk.done == 0 {
            walk.advance(&terrain, SPEED);
            ticks += 1;
        }
        let flat_first = lattice_walk_ticks(&flat(20, 20), SPEED, from, &steps[..1]);
        assert_eq!(ticks, flat_first, "the first step keeps its rate");
        assert!(
            walk.ticks_left(&terrain, SPEED) > total - ticks,
            "the next is slower"
        );
    }

    // TRN-FREE T14 (TRN-FREE-3.2a): the current direction is the starting
    // draw, then the direction of the last step. A number is drawn only
    // when it is not among the allowed directions. The target lies 3 steps
    // in direction 1 and then 3 in direction 2 from the walker, so on flat
    // ground both directions are allowed until one is used up.
    #[test]
    fn t14_direction_is_kept_and_drawn_only_when_not_allowed() {
        let terrain = flat(30, 30);
        let from = (10, 10);
        let mut to = from;
        for d in [1, 1, 1, 2, 2, 2] {
            to = terrain.neighbour(to, d).unwrap();
        }
        for seed in 1..60 {
            let start = crate::Rng::new(seed).draw() % 6;
            let mut rng = crate::Rng::new(seed);
            let r = free_walk(&terrain, &mut rng, from, to).unwrap();
            assert_eq!(r.fields.len(), 6);
            let mut dirs = Vec::new();
            let mut at = from;
            for &f in &r.fields {
                dirs.push(direction_toward(at, f));
                at = f;
            }
            // Draws: the start, plus one at each step whose direction is not
            // allowed, which here is only the turn from the used-up
            // direction, and, for a start draw outside {1, 2}, the first
            // step (which then picks 1 or 2).
            let mut expected = crate::Rng::new(seed);
            let turns = dirs.windows(2).filter(|w| w[0] != w[1]).count();
            let first_draw = u64::from(start != 1 && start != 2);
            for _ in 0..(1 + first_draw + turns as u64) {
                expected.draw();
            }
            if start == 1 || start == 2 {
                assert_eq!(dirs[0], start as u8, "seed {seed}: the start is kept");
            }
            assert_eq!(rng, expected, "seed {seed}: draws only when not allowed");
        }
    }

    // TRN-FREE T10: leaving a tree field costs 2 more, so a tree-free detour
    // of the same number of fields wins.
    // spec: TRN-FREE-2.5 (TRN-FREE T10)
    #[test]
    fn t10_trees_cost_more() {
        let mut terrain = flat(20, 8);
        for x in 3..8 {
            set(
                &mut terrain,
                (x, 3),
                state::FOR_BUILDING | state::DEPOSIT_BLOCKED,
            );
        }
        for seed in 1..20 {
            let r = route(&terrain, seed, (2, 3), (8, 3));
            assert!(
                r.fields.iter().all(|&f| f.1 != 3 || f == (8, 3)),
                "{:?}",
                r.fields
            );
        }
    }

    // TRN-FREE T11: a target with water all round: the first try fails, the
    // second crosses the water.
    // spec: TRN-FREE-2.6, TRN-FREE-2.8 (TRN-FREE T11)
    #[test]
    fn t11_second_try_crosses_water() {
        let mut terrain = flat(20, 8);
        let target = (10, 4);
        for f in crate::grid::area(target, 2, 20, 8) {
            if f != target {
                set(&mut terrain, f, state::WATER);
            }
        }
        let r = route(&terrain, 3, (2, 4), target);
        assert_eq!(*r.fields.last().unwrap(), target);
        assert!(r.fields.iter().any(|&f| terrain.has(f, state::WATER)));
    }

    // TRN-FREE-2.4, 2.5: uphill costs 4, 5, 7; downhill and level 3.
    #[test]
    fn free_step_costs() {
        let terrain = two_fields(1000, 1201);
        // The walker steps from (1,1) up to (2,1): the search goes from
        // (2,1) to (1,1), class of 1000 - 1201 is -1.
        assert_eq!(free_step_cost(&terrain, (2, 1), (1, 1)), 4);
        assert_eq!(free_step_cost(&terrain, (1, 1), (2, 1)), 3);
        for (h, cost) in [(1601, 5), (2201, 7), (3401, 3)] {
            let terrain = two_fields(1000, h);
            assert_eq!(free_step_cost(&terrain, (2, 1), (1, 1)), cost, "{h}");
        }
    }

    // TRN-FREE T12, TRN-FREE-4.3: halfway through a step from A to B a new
    // walk to C starts from A; the half step is lost.
    #[test]
    fn t12_new_walk_restarts_from_the_last_field() {
        let terrain = flat(20, 8);
        let mut walker = Walker::new((2, 3));
        walker.walk([(3, 3)]);
        for _ in 0..12 {
            walker.advance(&terrain, SPEED, |_| true);
        }
        assert_eq!((walker.at, walker.next), ((2, 3), Some((3, 3))));
        walker.walk([(2, 4)]);
        assert_eq!(
            (walker.at, walker.next, walker.progress_bits),
            ((2, 3), None, 0)
        );
        let mut ticks = 0;
        while walker.is_walking() {
            walker.advance(&terrain, SPEED, |_| true);
            ticks += 1;
        }
        assert_eq!((walker.at, ticks), ((2, 4), 23));
    }

    // TRN-FREE T13: the object 3 lattice steps from the walker, elevation
    // size 1, level: 2 lattice steps, 23 ticks; 4 steps to a field centre
    // give 3 steps, 34 ticks (ECO-OUT-5.1); TRN-FREE-5.3 timings.
    #[test]
    fn t13_fine_approach() {
        let terrain = flat(20, 8);
        let walker = crate::grid::field_sample((5, 4));
        let object = (walker.0 + 3, walker.1);
        let steps = fine_approach(&terrain, walker, object, 1);
        assert_eq!(steps.len(), 2);
        assert_eq!(lattice_walk_ticks(&terrain, SPEED, walker, &steps), 23);
        let centre = crate::grid::field_sample((6, 4));
        let steps = fine_approach(&terrain, walker, centre, 1);
        assert_eq!(steps.len(), 3);
        assert_eq!(lattice_walk_ticks(&terrain, SPEED, walker, &steps), 34);
        let line: Vec<Point> = (1..=4).map(|i| (walker.0 + i, walker.1)).collect();
        let ticks: Vec<u32> = (1..=4)
            .map(|n| lattice_walk_ticks(&terrain, SPEED, walker, &line[..n]))
            .collect();
        assert_eq!(ticks, [12, 23, 34, 45]);
        // The position along the walk: on the first step for its 12 ticks,
        // at the end after 45.
        assert_eq!(
            lattice_walk_at(&terrain, SPEED, walker, &line, 0),
            (walker, Some(line[0]), 0.0)
        );
        let (at, next, progress) = lattice_walk_at(&terrain, SPEED, walker, &line, 6);
        assert_eq!((at, next), (walker, Some(line[0])));
        assert!(progress > 1.0 / 3.0 && progress < 2.0 / 3.0, "{progress}");
        assert_eq!(
            lattice_walk_at(&terrain, SPEED, walker, &line, 12).0,
            line[0]
        );
        assert_eq!(
            lattice_walk_at(&terrain, SPEED, walker, &line, 45),
            (line[3], None, 0.0)
        );
    }

    // MAP-GRID-2.2, T13 of the grid spec: the direction toward each
    // neighbour is that neighbour's direction.
    #[test]
    fn directions_toward_neighbours() {
        for at in [(10, 10), (10, 11)] {
            for (d, n) in crate::grid::neighbours(at, 80, 80).into_iter().enumerate() {
                assert_eq!(direction_toward(at, n.unwrap()), d as u8, "{at:?} {d}");
            }
        }
        assert_eq!(direction_toward((10, 10), (10, 20)), 5);
        assert_eq!(direction_toward((10, 10), (10, 0)), 2);
    }

    // GFX-ANIM-3.1a, T19e: a walk counts from the tick it is ordered,
    // before its first step; a stop at a refused field and the arrival end
    // it; a new walk ordered on arrival counts at once.
    #[test]
    fn a_walk_counts_from_its_ordering_tick() {
        let terrain = flat(10, 4);
        let mut walker = Walker::new((0, 1));
        assert!(!walker.has_walk());
        walker.walk(straight((0, 1), 3));
        assert!(walker.has_walk() && walker.next.is_none());
        walker.advance(&terrain, SPEED, |_| true);
        assert!(walker.has_walk() && walker.next.is_some());
        for _ in 0..50 {
            walker.advance(&terrain, SPEED, |f| f != (2, 1));
        }
        assert_eq!(walker.at, (1, 1));
        assert!(walker.held && !walker.has_walk());
        walker.walk(straight((1, 1), 1));
        assert!(walker.has_walk());
        for _ in 0..50 {
            walker.advance(&terrain, SPEED, |_| true);
        }
        assert_eq!(walker.at, (2, 1));
        assert!(!walker.has_walk());
    }

    // spec: TRN-MOVE-2.2, TRN-FREE-4.3
    #[test]
    fn a_refused_step_stops_the_walk_and_drops_the_carry_over() {
        let terrain = flat(10, 4);
        let mut walker = Walker::new((0, 1));
        walker.walk(straight((0, 1), 3));
        for _ in 0..50 {
            walker.advance(&terrain, SPEED, |f| f != (2, 1));
        }
        assert_eq!(walker.at, (1, 1));
        assert_eq!(walker.next, None);
        assert_eq!(walker.progress_bits, 0);
        assert_eq!(walker.path.len(), 2);
    }

    // ECO-FOOD T44 (5.11a (a)): flat ground, a building on (10, 10), an
    // animal that avoids buildings walks from (8, 9) to (8, 11). The extra
    // belongs to the field stepped from: (9, 10), a neighbour of the
    // building, is labelled 13, (8, 10) 3 and the walker's field 6, so the
    // only route is over (8, 10), whatever the starting draw.
    // spec: ECO-FOOD-5.11a (T44)
    #[test]
    fn t44_the_extra_belongs_to_the_field_stepped_from() {
        let terrain = flat(20, 20);
        let building = (10, 10);
        let extra = |q: Point| {
            let near = std::iter::once(q)
                .chain(terrain.neighbours(q).into_iter().flatten())
                .filter(|&f| f == building)
                .count();
            10 * near as u32
        };
        let (from, to) = ((8, 9), (8, 11));
        let mut scratch = Scratch::default();
        assert!(scratch.search(&terrain, from, to, false, &extra));
        let label = |f: Point| scratch.get(terrain.index(f));
        assert_eq!(
            [label((9, 10)), label((8, 10)), label(from)],
            [Some(13), Some(3), Some(6)]
        );
        for seed in 1..40 {
            let mut rng = crate::Rng::new(seed);
            let r = animal_walk(&terrain, &mut rng, from, to, &extra).unwrap();
            assert_eq!(r.fields, [(8, 10), (8, 11)], "seed {seed}");
        }
    }

    // SHIP T13 (SHIP-3.1): a ship's route is the free walk's search with the
    // ship's field test and step cost: in open water a step costs 1; one
    // from a field beside land, a landing field or the map's edge 3; land
    // is not entered; the read-out draws from the generator.
    // spec: SHIP-3.1 (SHIP T13)
    #[test]
    fn ship_t13_the_ships_route_and_its_cost() {
        let mut terrain = flat(20, 12);
        for y in 0..12 {
            for x in 0..15 {
                set(&mut terrain, (x, y), state::WATER);
            }
        }
        let mut rng = crate::Rng::new(5);
        // Open water: four steps of 1, and draws.
        let route = ship_walk(&terrain, &mut rng, (4, 5), (8, 5)).unwrap();
        assert_eq!((route.cost, route.fields.len()), (4, 4));
        assert!(rng.draws() > 0, "the read-out draws");
        // From beside the land (x 15 on) and from the map's edge: 3.
        let route = ship_walk(&terrain, &mut rng, (14, 5), (13, 5)).unwrap();
        assert_eq!(route.cost, 3);
        let route = ship_walk(&terrain, &mut rng, (0, 5), (1, 5)).unwrap();
        assert_eq!(route.cost, 3);
        // From beside a landing field: 3.
        let mut landing = terrain.clone();
        set(&mut landing, (6, 8), state::WATER | state::LANDING_POSITION);
        let q = landing.neighbours((6, 8))[0].unwrap();
        let n = landing.neighbours(q)[0].unwrap();
        assert_ne!(n, (6, 8));
        assert_eq!(ship_walk(&landing, &mut rng, q, n).unwrap().cost, 3);
        // An island in the way is gone around.
        let mut island = terrain.clone();
        for y in 2..10 {
            set(&mut island, (8, y), 0);
        }
        let route = ship_walk(&island, &mut rng, (5, 5), (11, 5)).unwrap();
        assert!(route.fields.iter().all(|&f| island.has(f, state::WATER)));
    }
}
