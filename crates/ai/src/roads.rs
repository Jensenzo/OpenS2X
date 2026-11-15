//! Roads (AI-CON-8): joining building flags to other road networks, the
//! street optimizer and the route optimizer.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use s2_sim::grid::Point;
use s2_sim::map::state;
use s2_sim::movement::{free_walk_without_draws, NOT_WALKABLE};
use s2_sim::{Carrier, CarrierTask, Command, PlayerId, Rng, World};

use crate::player::{AiPlayer, Category, Ctx};
use crate::view::{dist, flag_of, Network};

/// AI-CON-8.4: breadth-first radius of the flag search.
const SEARCH_RADIUS: u32 = 10;
/// AI-CON-8.1: attempts per target (1 to 5 measure, 6 executes) and
/// give-ups per flag.
const EXECUTE_ATTEMPT: u32 = 6;
const MAX_GIVE_UPS: u32 = 5;
/// AI-CON-8.1a: the start value of the best lengths.
const NO_LENGTH: u32 = 1000;
/// AI-CON-8.3: tries per route flag, and the route/distance factor.
const MAX_TRIES: u32 = 2;
const ROUTE_FACTOR: u32 = 4;

/// What the street construction is doing with one flag (AI-CON-8.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagWork {
    pub flag: Point,
    /// AI-CON-8.1c: flags given up for this flag, and how often.
    pub excluded: Vec<Point>,
    pub give_ups: u32,
    /// The remembered target and the attempt counter (0 once a new T is
    /// remembered).
    pub target: Option<Point>,
    pub attempts: u32,
    /// The best strategy so far (0 none) and its lengths (8.1a).
    pub best: u8,
    pub length: u32,
    pub length2: u32,
    /// The flag position of strategies 2 and 5, the road cut (a field of
    /// it) and its far end P of strategies 3 to 5.
    pub waypoint: Option<Point>,
    pub cut: Option<(Point, Point)>,
}

impl FlagWork {
    fn new(flag: Point) -> Self {
        Self {
            flag,
            excluded: Vec::new(),
            give_ups: 0,
            target: None,
            attempts: 0,
            best: 0,
            length: NO_LENGTH,
            length2: NO_LENGTH,
            waypoint: None,
            cut: None,
        }
    }

    fn reset_attempts(&mut self) {
        self.target = None;
        self.attempts = 0;
        self.best = 0;
        self.length = NO_LENGTH;
        self.length2 = NO_LENGTH;
        self.waypoint = None;
        self.cut = None;
    }
}

/// AI-CON-8.3: the route optimizer's state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteWork {
    /// The building cursor (a creation number) and the current building.
    pub cursor: u32,
    pub current: Option<Point>,
    pub k: u32,
    pub i: usize,
    pub t: u32,
    pub w: usize,
    pub x: Vec<Point>,
}

impl RouteWork {
    fn clear_partner(&mut self) {
        self.i = 0;
        self.t = 0;
        self.w = 0;
        self.x.clear();
    }

    fn next_partner(&mut self) {
        self.k += 1;
        self.clear_partner();
    }
}

/// AI-CON-8.1 / 8.3 / 8.2 state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Street {
    pub cursor: usize,
    pub work: Option<FlagWork>,
    pub optimizer_cursor: usize,
    pub route: RouteWork,
}

/// A walk path a -> b "counts" when it exists and its length in steps is at
/// most 2 x distance(a, b) (AI-CON-8.1, 8.1e); its length in steps.
fn walk(
    world: &World,
    player: PlayerId,
    a: Point,
    b: Point,
    excluded: &[Point],
    road: &[Point],
) -> Option<u32> {
    let steps = walk_path(world, player, a, b, excluded, road)?;
    (steps <= 2 * dist(a, b)).then_some(steps)
}

/// AI-CON-8.1e: the walk path a -> b, the computer player's labelled search
/// of AI-2.7 b with the hit note off. It floods from `b` (open list by the
/// label plus h, h the distance to `a`, first in first out among equals,
/// relabelling) with the summed step cost and stops the moment `a` is
/// labelled. A field is not entered when it is farther than 4 x distance(a,
/// b) from `b` or is not the player's; nor, unless it is `a`, when it is in
/// `excluded`, holds a flag, is an inner road field not in `road` (the road
/// R a second leg may walk over), is not walkable, holds a tree or a stone
/// pile (T68: DepositBlocked, as a flag does), or a settler stands on it
/// (T61, T64: not in a step, [`standing_fields`]). Its length is the
/// number of steps from `a` down the labels to `b`, each to the neighbour
/// with the lowest label below the current one (the lowest direction among
/// equal labels); neighbours are 1 step without a search. No draws.
pub(crate) fn walk_path(
    world: &World,
    player: PlayerId,
    a: Point,
    b: Point,
    excluded: &[Point],
    road: &[Point],
) -> Option<u32> {
    walk_fields(world, player, a, b, excluded, road).map(|p| p.len() as u32 - 1)
}

/// [`walk_path`]'s fields, from `a` to `b` (both included).
pub(crate) fn walk_fields(
    world: &World,
    player: PlayerId,
    a: Point,
    b: Point,
    excluded: &[Point],
    road: &[Point],
) -> Option<Vec<Point>> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    match dist(a, b) {
        0 => return Some(vec![a]),
        1 => return Some(vec![a, b]),
        _ => {}
    }
    let terrain = world.terrain();
    let road_fields: BTreeSet<Point> = world
        .roads()
        .filter(|r| r.owner == player)
        .flat_map(|r| r.path[1..r.path.len() - 1].iter().copied())
        .collect();
    let standing = standing_fields(world);
    let reach = 4 * dist(a, b);
    let mut labels: BTreeMap<Point, u32> = BTreeMap::new();
    let mut entry: BTreeMap<Point, u64> = BTreeMap::new();
    let mut open = BinaryHeap::new();
    let mut seq = 0u64;
    labels.insert(b, 0);
    entry.insert(b, seq);
    open.push(Reverse((dist(b, a), seq, b)));
    seq += 1;
    let mut found = false;
    'search: while let Some(Reverse((_, number, n))) = open.pop() {
        if entry.get(&n) != Some(&number) {
            continue;
        }
        entry.remove(&n);
        let label_n = labels[&n];
        for q in terrain.neighbours(n).into_iter().flatten() {
            // AI-CON-8.1e (T93): only fields usable by the player, the goal
            // too.
            if dist(b, q) > reach || !usable(world, player, q) {
                continue;
            }
            if q != a
                && (excluded.contains(&q)
                    || world.flag_at(q).is_some()
                    || (road_fields.contains(&q) && !road.contains(&q))
                    || standing.contains(&q))
            {
                continue;
            }
            let g = label_n + summed_cost(world, n, q);
            if labels.get(&q).is_some_and(|&l| l <= g) {
                continue;
            }
            labels.insert(q, g);
            if q == a {
                found = true;
                break 'search;
            }
            entry.insert(q, seq);
            open.push(Reverse((g + dist(q, a), seq, q)));
            seq += 1;
        }
    }
    if !found {
        return None;
    }
    let mut fields = vec![a];
    let mut at = a;
    while at != b {
        let here = labels[&at];
        let neighbours = terrain.neighbours(at);
        let (_, next) = (0..6)
            .filter_map(|d| {
                let q = neighbours[d]?;
                let l = *labels.get(&q)?;
                (l < here).then_some((l, q))
            })
            .min_by_key(|&(l, _)| l)?;
        at = next;
        fields.push(at);
    }
    Some(fields)
}

/// The road planner accepts a road when its field count - 1 is at most
/// 2 x the straight distance of its ends (AI-CON-8.1b).
fn plannable(world: &World, player: PlayerId, from: Point, to: Point) -> bool {
    world
        .find_route(player, from, to)
        .is_ok_and(|(path, _)| path.len() as u32 - 1 <= 2 * dist(from, to))
}

/// AI-CON-8.4 flag search: from `start` breadth-first with no limit on the
/// steps over the fields within distance 10 of it that are walkable and do
/// not carry DepositBlocked (trees, stones), the start tested first, then
/// each field's neighbours in the ring order 5, 0, 1, 2, 3, 4
/// (MAP-GRID-3.2), each tested when it is reached.
/// The first own flag that is not in `excluded` and (given a network and a
/// flag) not in the same road network as that flag.
pub(crate) fn flag_search(
    world: &World,
    player: PlayerId,
    start: Point,
    excluded: &[Point],
    network: Option<(&Network, Point)>,
) -> Option<Point> {
    /// MAP-GRID-3.2: ring 1 visited as the neighbours in directions 5, 0,
    /// 1, 2, 3, 4.
    const RING_ORDER: [u8; 6] = [5, 0, 1, 2, 3, 4];
    let terrain = world.terrain();
    let target = |here: Point| {
        world.flag_at(here) == Some(player)
            && !excluded.contains(&here)
            && network.is_none_or(|(n, own)| !n.connected(here, own))
    };
    if target(start) {
        return Some(start);
    }
    let mut seen: BTreeSet<Point> = BTreeSet::new();
    let mut queue = VecDeque::new();
    seen.insert(start);
    queue.push_back(start);
    while let Some(here) = queue.pop_front() {
        for next in RING_ORDER
            .into_iter()
            .filter_map(|dir| terrain.neighbour(here, dir))
        {
            if seen.contains(&next)
                || terrain.has(next, NOT_WALKABLE | state::DEPOSIT_BLOCKED)
                || dist(start, next) > SEARCH_RADIUS
            {
                continue;
            }
            seen.insert(next);
            if target(next) {
                return Some(next);
            }
            queue.push_back(next);
        }
    }
    None
}

/// The building whose flag is `flag`.
fn building_of_flag(world: &World, player: PlayerId, flag: Point) -> Option<Point> {
    let b = world.terrain().neighbour(flag, 4)?;
    world
        .building_at(b)
        .filter(|x| x.owner == player && flag_of(world, b) == Some(flag))
        .map(|_| b)
}

/// What the road-stop search of AI-CON-8.1 strategy 3 found.
pub(crate) struct RoadStop {
    /// The walk path F -> T: the fields after F, ending with T.
    pub(crate) path: Vec<Point>,
    /// The first field reached that carries a road R attached to T.
    pub(crate) hit: Point,
    /// P, the other end of R.
    pub(crate) p: Point,
    /// R's fields, which the attempt's second leg may walk over (8.1e).
    pub(crate) road: Vec<Point>,
}

/// AI-CON-8.1e, AI-2.7 b (T93): a field the computer player's searches may
/// enter: the player's inner land (TER-6.1: owned, not Border, not Water,
/// ElevationBlocked or DoodadBlocked) and not Blocked, DepositBlocked or
/// MovementBlocked. A flag or road field always passes.
pub(crate) fn usable(world: &World, player: PlayerId, q: Point) -> bool {
    world.field_owner(q) == Some(player)
        && !world
            .terrain()
            .has(q, state::BORDER | NOT_WALKABLE | state::DEPOSIT_BLOCKED)
}

/// AI-2.7 b, Q-239: the cost of a step between neighbours `n` and `q` for
/// the computer player's searches: the free walk's cost of the step each
/// way, by the height class (3 level or down, 4, 5, 7 for one, two, three
/// classes up), doubled when `q` is a harbour field. (The free walk's 2 more
/// each way for a deposit-blocked field never applies: such a field is not
/// usable, T93.)
fn summed_cost(world: &World, n: Point, q: Point) -> u32 {
    let terrain = world.terrain();
    let one_way = |from: Point, to: Point| {
        let rise = i64::from(terrain.height_at(to)) - i64::from(terrain.height_at(from));
        match s2_sim::grid::height_class(rise) {
            1 => 4,
            2 => 5,
            3 => 7,
            _ => 3,
        }
    };
    let mut cost = one_way(n, q) + one_way(q, n);
    if terrain.has(q, state::FOR_HARBOR) {
        cost *= 2;
    }
    cost
}

/// AI-CON-8.1 strategies 3 to 5, the **road-stop search** (AI-2.7 b,
/// Q-239): the free walk's labelled search from `f` (open list by f = g +
/// h, h the distance to `t`, first in first out among equal f, relabelling,
/// the stop as soon as `t` is labelled) with the computer player's summed
/// step cost and one try. A field is not entered when it is farther than 4
/// x distance(`f`, `t`) from `f`, is in `excluded`, is not the player's, holds
/// a flag (but `t`), or is a road field that is neither `t` nor on the hit
/// road. While no hit is noted, the first road field evaluated whose road
/// has `t` as an end is the hit, and its road can then be entered. The
/// path is read from `t` down the labels to `f` with the free walk's rule
/// and draws (TRN-FREE-3.2a): a first direction, none for a single step,
/// one more where the kept direction is not allowed. It counts when its
/// field count - 1 is at most 2 x distance(`f`, `t`) (Q-168).
pub(crate) fn road_stop_search(
    world: &World,
    rng: &mut Rng,
    player: PlayerId,
    f: Point,
    t: Point,
    excluded: &[Point],
) -> Option<RoadStop> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let terrain = world.terrain();
    // Each inner road field's road: its far end from `t`, when `t` is an end.
    let mut road_of: BTreeMap<Point, usize> = BTreeMap::new();
    let mut far_end: BTreeMap<usize, (Point, &[Point])> = BTreeMap::new();
    for (i, road) in world.roads().filter(|r| r.owner == player).enumerate() {
        let (first, last) = (road.path[0], road.path[road.path.len() - 1]);
        if first == t {
            far_end.insert(i, (last, &road.path));
        } else if last == t {
            far_end.insert(i, (first, &road.path));
        }
        for &field in &road.path[1..road.path.len() - 1] {
            road_of.insert(field, i);
        }
    }
    let reach = 4 * dist(f, t);
    let mut hit: Option<(Point, usize)> = None;
    let mut labels: BTreeMap<Point, u32> = BTreeMap::new();
    let mut entry: BTreeMap<Point, u64> = BTreeMap::new();
    let mut open = BinaryHeap::new();
    let mut seq = 0u64;
    labels.insert(f, 0);
    entry.insert(f, seq);
    open.push(Reverse((dist(f, t), seq, f)));
    seq += 1;
    let mut found = false;
    'search: while let Some(Reverse((_, number, n))) = open.pop() {
        if entry.get(&n) != Some(&number) {
            continue;
        }
        entry.remove(&n);
        let label_n = labels[&n];
        for q in terrain.neighbours(n).into_iter().flatten() {
            // AI-CON-8.1e (T93): only fields usable by the player.
            if dist(f, q) > reach || excluded.contains(&q) || !usable(world, player, q) {
                continue;
            }
            // Q-245: the hit is noted after the bound, the excluded fields
            // and the usability test, before the flag and road tests.
            let road = road_of.get(&q).copied().filter(|_| q != t);
            if let Some(road) = road {
                if hit.is_none() && far_end.contains_key(&road) {
                    hit = Some((q, road));
                }
            }
            if q != t && world.flag_at(q).is_some() {
                continue;
            }
            if road.is_some_and(|r| hit.map(|(_, h)| h) != Some(r)) {
                continue;
            }
            let g = label_n + summed_cost(world, n, q);
            if labels.get(&q).is_some_and(|&l| l <= g) {
                continue;
            }
            labels.insert(q, g);
            if q == t {
                found = true;
                break 'search;
            }
            entry.insert(q, seq);
            open.push(Reverse((g + dist(q, t), seq, q)));
            seq += 1;
        }
    }
    // Without a hit there is no read-out and no draw (Q-245: the original
    // draws nothing in the hunter flag's attempts 3 to 5 before T has a
    // road, E-trace-002 counters 155, 162, 169).
    let (hit, road) = hit?;
    if !found {
        return None;
    }
    // TRN-FREE-3.2a: from `t` down the labels to `f`.
    let mut path = vec![t];
    if dist(f, t) > 1 {
        let mut direction = (rng.draw() % 6) as u8;
        let mut at = t;
        while at != f {
            let here = labels[&at];
            let neighbours = terrain.neighbours(at);
            let lower: Vec<(u8, u32)> = (0u8..6)
                .filter_map(|d| {
                    let l = *labels.get(&neighbours[usize::from(d)]?)?;
                    (l < here).then_some((d, l))
                })
                .collect();
            let least = lower.iter().map(|&(_, l)| l).min()?;
            let allowed: Vec<u8> = lower
                .iter()
                .filter(|&&(_, l)| l == least)
                .map(|&(d, _)| d)
                .collect();
            if !allowed.contains(&direction) {
                direction = allowed[(rng.draw() % allowed.len() as u32) as usize];
            }
            at = neighbours[usize::from(direction)]?;
            path.push(at);
        }
    } else {
        path.push(f);
    }
    // The fields after `f`, ending with `t`.
    path.pop();
    path.reverse();
    if path.len() as u32 > 2 * dist(f, t) {
        return None;
    }
    let (p, road) = far_end[&road];
    Some(RoadStop {
        p,
        road: road.to_vec(),
        path,
        hit,
    })
}

/// AI-CON-8.1e (T79): a worker on his fine walk back from a work spot to
/// his own field (TRN-FREE-5.6) is in a step until he stands on it, also in
/// a tick his walker shows him standing.
pub(crate) fn in_a_step(job: &s2_sim::SettlerJob) -> bool {
    matches!(
        job,
        s2_sim::SettlerJob::Worker {
            work: s2_sim::Work::Return { .. },
            ..
        }
    )
}

/// AI-CON-8.1e (T80): the fields on which a road's donkey stands idle.
pub(crate) fn idle_donkey_fields(carriers: &[Carrier]) -> BTreeSet<Point> {
    carriers
        .iter()
        .filter(|c| c.donkey && matches!(c.task, CarrierTask::Idle) && !c.walker.is_walking())
        .map(|c| c.walker.at)
        .collect()
}

/// AI-CON-8.1e (T61, T64): the fields a settler stands on, not in a step
/// (a carrier waiting on his idle field, a worker in a timed action on his
/// work spot; one walking through does not block a walk path). A retired
/// road carrier is the settler he became. Builders and levellers at work on
/// their sites are not counted (not observed). A field on which a road's
/// donkey stands idle is not counted, even with the road's carrier standing
/// there too (T80).
pub(crate) fn standing_fields(world: &World) -> BTreeSet<Point> {
    let donkeys = idle_donkey_fields(world.carriers());
    let carriers = world
        .carriers()
        .iter()
        .filter(|c| !matches!(c.task, CarrierTask::Retired))
        .map(|c| (c.number, &c.walker));
    let settlers = world
        .settlers()
        .filter(|(_, s)| !in_a_step(&s.job))
        .map(|(_, s)| (s.number, &s.walker));
    // AI-CON-8.1e (T95): standing there means holding the field's settler
    // mark; another settler that walked over the field since cleared it.
    carriers
        .chain(settlers)
        .filter(|(number, w)| !w.is_walking() && world.field_mark(w.at) == Some(*number))
        .map(|(_, w)| w.at)
        .filter(|at| !donkeys.contains(at))
        .collect()
}

impl AiPlayer {
    /// AI-CON-8.1, every 7 ticks: one flag at a time through its attempts.
    pub(crate) fn street_construction(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let flags = world.flags_in_order(self.id);
        if flags.is_empty() {
            return;
        }
        // AI-CON-8.1f (T60): the cursor is a list index; the work goes on
        // only while its flag is still the one at the cursor (a flag removed
        // earlier in the list closes it up: the work is dropped).
        let flag = match &self.street.work {
            Some(w) if flags.get(self.street.cursor) == Some(&w.flag) => w.flag,
            // T97: another flag at the cursor while work is held: the
            // cursor moves on at once and that flag is handled in this run
            // as a new F. One with a lower object number than the worked
            // flag (earlier in the list, which is in creation order) makes
            // the run do nothing, the work kept (code, not observed).
            Some(w) if self.street.cursor < flags.len() => {
                if flags
                    .iter()
                    .position(|&f| f == w.flag)
                    .is_some_and(|i| i > self.street.cursor)
                {
                    return;
                }
                self.street.work = None;
                self.street.cursor += 1;
                flags[self.street.cursor - 1]
            }
            _ => {
                self.street.work = None;
                // AI-CON-8.1d: the restart at the list's end happens at the
                // start of a run, which handles the first flag.
                if self.street.cursor >= flags.len() {
                    self.street.cursor = 0;
                }
                flags[self.street.cursor]
            }
        };
        // A flag without a building is skipped.
        if building_of_flag(world, self.id, flag).is_none() {
            self.next_flag(&flags, flag);
            return;
        }
        let mut work = self
            .street
            .work
            .take()
            .unwrap_or_else(|| FlagWork::new(flag));
        let network = Network::compute(world, self.id);
        let Some(target) =
            flag_search(world, self.id, flag, &work.excluded, Some((&network, flag)))
        else {
            self.next_flag(&flags, flag);
            return;
        };
        // A flag with roads is not joined to a lone flag.
        if network.roads_at(flag) > 0 && network.roads_at(target) == 0 {
            self.give_up(work, target, &flags);
            return;
        }
        if work.target != Some(target) {
            // A new target is remembered and the run ends.
            work.reset_attempts();
            work.target = Some(target);
            self.street.work = Some(work);
            return;
        }
        work.attempts += 1;
        if work.attempts > EXECUTE_ATTEMPT {
            // A failed execution leaves the counter at 6: given up now.
            self.give_up(work, target, &flags);
            return;
        }
        if work.attempts < EXECUTE_ATTEMPT {
            self.measure(ctx, &mut work, target);
            self.street.work = Some(work);
            return;
        }
        if self.execute(ctx, &work, target) {
            self.next_flag(&flags, flag);
        } else {
            self.street.work = Some(work);
        }
    }

    /// Take the flag after `current` in the list.
    /// AI-CON-8.1d: the cursor moves past the flag; it is not wrapped here,
    /// so a flag added before the next run is the next one handled.
    fn next_flag(&mut self, flags: &[Point], current: Point) {
        let index = flags.iter().position(|&f| f == current).unwrap_or(0);
        self.street.cursor = index + 1;
        self.street.work = None;
    }

    /// AI-CON-8.1c.
    fn give_up(&mut self, mut work: FlagWork, target: Point, flags: &[Point]) {
        work.excluded.push(target);
        work.give_ups += 1;
        if work.give_ups > MAX_GIVE_UPS {
            self.next_flag(flags, work.flag);
        } else {
            work.reset_attempts();
            self.street.work = Some(work);
        }
    }

    /// AI-CON-8.1a: measure strategy `work.attempts`.
    fn measure(&self, ctx: &mut Ctx, work: &mut FlagWork, t: Point) {
        let world = ctx.world;
        let (id, f) = (self.id, work.flag);
        let excluded = work.excluded.clone();
        let walk = |a: Point, b: Point, road: &[Point]| walk(world, id, a, b, &excluded, road);
        match work.attempts {
            1 => {
                if let Some(len) = walk(f, t, &[]) {
                    if len < work.length {
                        work.length = len;
                        work.best = 1;
                    }
                }
            }
            2 => {
                for w in self.road_waypoints(world, t) {
                    if let Some(len) = walk(f, w, &[]) {
                        if len < work.length {
                            work.length = len;
                            work.best = 2;
                            work.waypoint = Some(w);
                        }
                    }
                }
            }
            k @ 3..=5 => {
                let Some(stop) = road_stop_search(world, ctx.rng, id, f, t, &work.excluded) else {
                    return;
                };
                // The road-stop path's length is its field count with F and
                // T, the walk paths' in steps: 3 to 5 replace 1 or 2 only
                // with a path one step shorter (T30). Equal lengths among
                // 3 to 5 are allowed.
                let first = stop.path.len() as u32 + 1;
                if first > work.length {
                    return;
                }
                let p = stop.p;
                // The second legs may walk over R's fields (8.1e); T74: the
                // fields of this attempt's road-stop path are excluded
                // positions for them (their goal a exempt).
                let mut excluded = work.excluded.clone();
                excluded.extend(stop.path.iter().copied());
                let second =
                    |a: Point, b: Point| crate::roads::walk(world, id, a, b, &excluded, &stop.road);
                // T75: strategy 5's goals lie on the road-stop path, so its
                // walks P -> w never count: only 3 and 4 measure.
                if let 3 | 4 = k {
                    let leg = if k == 3 { t } else { f };
                    if let Some(second) = second(leg, p) {
                        if second < work.length2 {
                            work.length = first;
                            work.length2 = second;
                            work.best = k as u8;
                            work.cut = Some((stop.hit, p));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// The inner fields of the roads at `t` where a flag can be set.
    fn road_waypoints(&self, world: &World, t: Point) -> Vec<Point> {
        let mut out = Vec::new();
        // AI-CON-8.1 a (T62): the roads at `t` in the flag's road list, the
        // order they were attached, by object number.
        let mut roads: Vec<&s2_sim::Road> = world.roads().filter(|r| r.owner == self.id).collect();
        roads.sort_by_key(|r| r.number);
        for road in roads {
            if road.path.first() == Some(&t) || road.path.last() == Some(&t) {
                for &w in &road.path[1..road.path.len() - 1] {
                    if world.check_flag(self.id, w).is_ok() {
                        out.push(w);
                    }
                }
            }
        }
        out
    }

    /// AI-CON-8.1b, attempt 6: build with the best strategy. True if it
    /// worked (the planner accepted every road it could be asked).
    fn execute(&mut self, ctx: &mut Ctx, work: &FlagWork, t: Point) -> bool {
        let world = ctx.world;
        let (id, f) = (self.id, work.flag);
        // The road builder (`PlanRoad`): its stub takes a number and the
        // path found evens the ground, built or refused (SIM-START-4.4 c,
        // TRN-ROAD-3.3a); the planner's verdict is known here beforehand.
        let road = |ctx: &mut Ctx, from: Point, to: Point| {
            ctx.out.push(Command::PlanRoad {
                player: id,
                from,
                to,
            });
        };
        match work.best {
            1 => {
                road(ctx, f, t);
                plannable(world, id, f, t)
            }
            2 => {
                let Some(w) = work.waypoint else {
                    return false;
                };
                if !plannable(world, id, f, w) {
                    road(ctx, f, w);
                    return false;
                }
                ctx.out.push(Command::PlaceFlag { player: id, at: w });
                road(ctx, f, w);
                true
            }
            3..=5 => {
                // The search of attempt 3 is repeated first; nothing is done
                // if it fails.
                let Some(stop) = road_stop_search(world, ctx.rng, id, f, t, &work.excluded) else {
                    return false;
                };
                // R is removed; every variant then rebuilds P's connection
                // (the network is only torn if the planner refuses it).
                ctx.out.push(Command::RemoveRoad {
                    player: id,
                    at: stop.hit,
                });
                road(ctx, f, t);
                match work.best {
                    3 => road(ctx, t, stop.p),
                    4 => road(ctx, f, stop.p),
                    _ => {
                        if let Some(w) = work.waypoint {
                            ctx.out.push(Command::PlaceFlag { player: id, at: w });
                            road(ctx, stop.p, w);
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// AI-CON-8.2 street optimizer, every 67 ticks (profile switch): one road
    /// per run gets a flag on its first waypoint, from the road's second
    /// field on, where a flag fits and no object stands.
    pub(crate) fn street_optimizer(&mut self, ctx: &mut Ctx) {
        if !self.profile.street_optimizing {
            return;
        }
        let world = ctx.world;
        // T62: the player's roads in creation order, by object number (a
        // new road may take a removed one's storage slot).
        let mut roads: Vec<&s2_sim::Road> = world.roads().filter(|r| r.owner == self.id).collect();
        roads.sort_by_key(|r| r.number);
        if roads.is_empty() {
            return;
        }
        // T58: the cursor is compared with the list's length only when it
        // is read; after the run it is one more, not wrapped, so roads made
        // since come next.
        let cursor = self.street.optimizer_cursor;
        let i = if cursor >= roads.len() { 0 } else { cursor };
        self.street.optimizer_cursor = i + 1;
        let road = roads[i];
        let n = road.path.len();
        for &w in road.path.iter().take(n.saturating_sub(1)).skip(1) {
            if world.check_flag(self.id, w).is_ok()
                && world.building_at(w).is_none()
                && world.deposit(w).is_none()
            {
                ctx.out.push(Command::PlaceFlag {
                    player: self.id,
                    at: w,
                });
                return;
            }
        }
    }

    // ---- AI-CON-8.3 route optimizer ----

    /// The buildings that give B its partners with k >= 2: the producer
    /// buildings of B's needs, then the consumer buildings of B's productions.
    /// B's needs in the order of its own list, the order they were tied to
    /// it (T51), not by need id. A production counts only through a need
    /// with a consumer building, so the locksmithery's tools production,
    /// whose need has none (AI-CON-10.2), adds no partner (T54).
    pub(crate) fn related_buildings(&self, b: Point) -> Vec<Point> {
        let mut list = Vec::new();
        let mut needs: Vec<&crate::plan::Need> = self
            .plan
            .needs
            .iter()
            .filter(|n| n.consumer == Some(b))
            .collect();
        needs.sort_by_key(|n| n.tied);
        for need in needs {
            if let Some(at) = need
                .production
                .and_then(|p| self.plan.production(p))
                .and_then(|p| p.building)
            {
                list.push(at);
            }
        }
        for prod in self.plan.prods.iter().filter(|p| p.building == Some(b)) {
            if let Some(need) = prod.need.and_then(|n| self.plan.need(n)) {
                if let Some(consumer) = need.consumer.filter(|c| self.records.contains_key(c)) {
                    list.push(consumer);
                }
            }
        }
        list
    }

    /// AI-CON-8.3b: the partner of building `b` for the index `k`; `None`
    /// when `b` is done. The searches of k = 0 and 1 pick among the
    /// player's finished warehouses (headquarters, storehouses, harbours) in
    /// the order they were made, the first winning a tie; k = 0 by the cost
    /// of the cheapest ware route from B's flag to theirs (one ware network,
    /// TRN-CARRY-5.3), k = 1 by distance.
    pub(crate) fn partner(&mut self, ctx: &mut Ctx, b: Point) -> Option<Point> {
        let world = ctx.world;
        let rec = self.records.get(&b)?.clone();
        let finished = |at: Point| world.building_at(at).is_some_and(|x| !x.under_construction);
        let fb = flag_of(world, b)?;
        let k = self.street.route.k;
        let mut warehouses: Vec<(u32, Point)> = self
            .records
            .values()
            .filter(|r| r.at != b && world.warehouse(r.at).is_some() && finished(r.at))
            .map(|r| (r.serial, r.at))
            .collect();
        warehouses.sort();
        // The first of the cheapest by stored ware route cost from B's flag
        // (TRN-CARRY-5.3a), among those in its ware network.
        let id = self.id;
        let network = world.ware_network_of(id, fb);
        let mut cheapest = |keep: &dyn Fn(Point) -> bool| {
            let mut best: Option<(u32, Point)> = None;
            for &(_, at) in &warehouses {
                if !keep(at) {
                    continue;
                }
                let Some(f) = flag_of(world, at).filter(|f| network.contains(f)) else {
                    continue;
                };
                let Some(cost) = ctx.routes.ware_cost(world, id, fb, f) else {
                    continue;
                };
                if best.is_none_or(|(c, _)| cost < c) {
                    best = Some((cost, at));
                }
            }
            best.map(|(_, at)| at)
        };
        // B being upgraded counts as unfinished while its upgrade's
        // construction has work left (MIL-UPG-3; T44).
        // (the upgrade's construction with work left: its site still there,
        // MIL-UPG-2.4)
        // AI-2.1a (T69): so does an upgrade of B the military system ordered
        // earlier in this think: the think's view has it in effect.
        if !finished(b) || (world.upgrading(b).is_some() && world.site(b).is_some()) {
            // The nearest finished warehouse holding wood, else stone.
            if k != 0 {
                return None;
            }
            let n = &ctx.config.names;
            return [n.wood, n.stone].into_iter().find_map(|good| {
                cheapest(&|at| world.warehouse(at).is_some_and(|w| w.count(good) > 0))
            });
        }
        // AI-CON-1.2 (Q-279, T45): B is a depot by its type's depot flag, a
        // harbour too, though it is filed in its cell's harbour list.
        if rec.protect.depot {
            // The 3 x 3 neighbouring cells numbered 0, 1, 2, 4, 5, 6, 8, 9, 10
            // (columns outer, rows inner); S from a cell's depot list only.
            let (cx, cy) = rec.cell;
            for number in k..=10u32 {
                let (dx, dy) = (number / 4, number % 4);
                if dy > 2 {
                    continue;
                }
                let x = i64::from(cx) + i64::from(dx) - 1;
                let y = i64::from(cy) + i64::from(dy) - 1;
                if x < 0 || y < 0 {
                    continue;
                }
                let pos = (x as u32, y as u32);
                if pos == rec.cell {
                    continue;
                }
                let depot = self.cells.get(pos).and_then(|c| {
                    c.buildings.iter().copied().find(|a| {
                        self.records
                            .get(a)
                            .is_some_and(|r| r.category == Category::Depot)
                    })
                });
                if let Some(at) = depot {
                    self.street.route.k = number;
                    return Some(at);
                }
            }
            return None;
        }
        let mut k = k;
        if k == 0 {
            if let Some(at) = cheapest(&|_| true) {
                return Some(at);
            }
            k = 1;
            self.street.route.k = 1;
        }
        if k == 1 {
            // AI-CON-1.1b: on B's field's landmass.
            let landmass = world.continent_of(b);
            let mut best: Option<(u32, Point)> = None;
            for &(_, at) in warehouses
                .iter()
                .filter(|&&(_, at)| world.continent_of(at) == landmass)
            {
                if best.is_none_or(|(d, _)| dist(b, at) < d) {
                    best = Some((dist(b, at), at));
                }
            }
            if let Some((_, at)) = best {
                return Some(at);
            }
            k = 2;
            self.street.route.k = 2;
        }
        self.related_buildings(b).get((k - 2) as usize).copied()
    }

    /// AI-CON-8.3, every 37 ticks (profile switch).
    pub(crate) fn route_optimizer(&mut self, ctx: &mut Ctx) {
        if !self.profile.route_optimizing {
            return;
        }
        let world = ctx.world;
        // a. The building record with the smallest id >= cursor.
        let mut records: Vec<_> = self.records.values().map(|r| (r.serial, r.at)).collect();
        records.sort();
        let cursor = self.street.route.cursor;
        let Some(&(serial, b)) = records.iter().find(|&&(s, _)| s >= cursor) else {
            self.street.route = RouteWork::default();
            return;
        };
        if self.street.route.current != Some(b) {
            if self.records.get(&b).is_some_and(|r| r.protect.planting) {
                self.street.route = RouteWork {
                    cursor: serial + 1,
                    ..RouteWork::default()
                };
                return;
            }
            if serial > cursor {
                // Only moves the cursor to it; handled next run.
                self.street.route.cursor = serial;
                return;
            }
            self.street.route.current = Some(b);
            self.street.route.k = 0;
            self.street.route.clear_partner();
        }
        let Some(s) = self.partner(ctx, b) else {
            // Done with this building.
            self.street.route = RouteWork {
                cursor: serial + 1,
                ..RouteWork::default()
            };
            return;
        };
        // c. On the buildings' flags, over the ware networks with the ware
        // costs (corrected 2026-10-24 night).
        let (Some(fb), Some(fs)) = (flag_of(world, b), flag_of(world, s)) else {
            self.street.route.next_partner();
            return;
        };
        // TRN-CARRY-5.3a: every cost of c is a stored cost.
        if !world.ware_network_of(self.id, fb).contains(&fs) {
            self.street.route.next_partner();
            return;
        }
        let Some(r) = ctx.routes.ware_cost(world, self.id, fb, fs) else {
            self.street.route.next_partner();
            return;
        };
        if r > ROUTE_FACTOR * dist(fb, fs) {
            self.direct_variant(ctx, fb, fs);
        } else {
            self.route_variant(ctx, fb, fs, r);
        }
    }

    /// AI-CON-8.3c, route cost above 4 x distance: one field of the path
    /// from B's flag (field 0) to S's per run.
    pub(crate) fn direct_variant(&mut self, ctx: &mut Ctx, fb: Point, fs: Point) {
        let world = ctx.world;
        let id = self.id;
        // The free walk's path by its first try, read out without draws.
        let Some(path) = free_walk_without_draws(world.terrain(), fb, fs) else {
            self.street.route.next_partner();
            return;
        };
        let w = self.street.route.w;
        let field = if w == 0 {
            fb
        } else {
            let Some(&field) = path.fields.get(w - 1) else {
                self.street.route.next_partner();
                return;
            };
            field
        };
        self.street.route.w += 1;
        let Some(near) = flag_search(world, id, field, &[], None) else {
            return;
        };
        match self.street.route.x.last().copied() {
            Some(last) if last != near => {
                if world.flag_at(last) == Some(id) {
                    // T99: G and Fn without a ware route are not too long
                    // (the stored cost reports none as -2; the test is
                    // signed).
                    let too_long = ctx
                        .routes
                        .ware_cost(world, id, last, near)
                        .is_some_and(|cost| cost > ROUTE_FACTOR * dist(near, last));
                    if too_long {
                        // The road builder Fn -> G (the road if the planner
                        // accepts it); S is finished either way (Q-273).
                        ctx.out.push(Command::PlanRoad {
                            player: id,
                            from: near,
                            to: last,
                        });
                        self.street.route.next_partner();
                        return;
                    }
                }
                // Only the last Fn is kept.
                self.street.route.x = vec![near];
            }
            Some(_) => {}
            None => self.street.route.x = vec![near],
        }
    }

    /// AI-CON-8.3c, route cost not above 4 x distance: shortcuts between
    /// the flags of the ware route, listed from B's flag (entry 0) to S's.
    fn route_variant(&mut self, ctx: &mut Ctx, fb: Point, fs: Point, r: u32) {
        let world = ctx.world;
        let id = self.id;
        // The list's search runs from S's flag towards B's (a road costs 4L
        // when the flag it leads to away from S's is full; 79e4711), read
        // from B's flag (entry 0) to S's.
        let Some(route) = world.ware_route_flags(id, fs, fb).map(|mut r| {
            r.reverse();
            r
        }) else {
            self.street.route.next_partner();
            return;
        };
        let i = self.street.route.i;
        if i + 2 > route.len() {
            self.street.route.next_partner();
            return;
        }
        let f0 = route[i];
        let mut excluded: Vec<Point> = route.iter().take(i + 2).copied().collect();
        excluded.extend(self.street.route.x.iter().copied());
        let Some(near) = flag_search(world, id, f0, &excluded, None) else {
            self.street.route.t = 0;
            self.street.route.x.clear();
            self.street.route.i += 1;
            return;
        };
        self.street.route.x.push(near);
        // An Fn outside S's ware network ends the run without a try.
        if !world.ware_network_of(id, fs).contains(&near) {
            return;
        }
        // The walk path counts by its steps; its length in the sum is its
        // road length L (TRN-CARRY-5.2, both directions; T36).
        let path = walk_fields(world, id, near, f0, &[], &[])
            .filter(|p| p.len() as u32 - 1 <= 2 * dist(near, f0));
        if let Some(path) = path {
            // T63: the two stored costs are asked only when the walk path
            // counts, cost(Fn -> S) first.
            let Some(a) = ctx.routes.ware_cost(world, id, near, fs) else {
                return;
            };
            let len = world.road_length(&path);
            // cost(Fn -> S) + the new road + cost(F0 -> B) < r.
            if let Some(c) = ctx.routes.ware_cost(world, id, f0, fb) {
                if a + len + c < r {
                    // The road builder Fn -> F0 (the road if the planner
                    // accepts it); S is finished either way (Q-273).
                    ctx.out.push(Command::PlanRoad {
                        player: id,
                        from: near,
                        to: f0,
                    });
                    self.street.route.next_partner();
                    return;
                }
            }
        }
        self.street.route.t += 1;
        if self.street.route.t > MAX_TRIES {
            self.street.route.t = 0;
            self.street.route.x.clear();
            self.street.route.i += 1;
        }
    }
}
