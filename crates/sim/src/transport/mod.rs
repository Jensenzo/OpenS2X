//! Carriers and ware transport (`specs/TRN-carriers-transport.md`).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::sync::{Arc, Mutex, PoisonError};

use crate::grid::Point;
use crate::movement::{slope_factor_percent, Walker};
use crate::{PlayerId, Rejection, Road, Target, Tick, World};

/// TRN-CARRY-2.1: an idle carrier on its idle field looks for work every this
/// many ticks, phased by its number.
const IDLE_LOOK_PERIOD: u64 = 20;
/// TRN-CARRY-3.2: a carrier waiting for room checks again every this many ticks.
const ROOM_RETRY_PERIOD: u64 = 10;
/// TRN-CARRY-3.2, 3.6: and in those checks tries a road swap every this many.
const ROAD_SWAP_PERIOD: u64 = 200;
/// TRN-CARRY-2.5: the priority that stands for "no ware".
const NO_WARE_PRIORITY: i64 = 100;
/// TRN-CARRY-6.1: a road's traffic period ends when its tick count passes this.
const TRAFFIC_PERIOD: u32 = 6000;
/// TRN-CARRY-6.2: a road asks for a donkey when its average traffic exceeds this.
const DONKEY_THRESHOLD: f32 = 130.0;
/// TRN-CARRY-5.2: road length is kept in twelfths so that `1 / f` is whole for
/// every slope factor (1.0, 0.8, 0.6, 0.4 give 12, 15, 20, 30).
const LENGTH_SCALE: u32 = 12;

/// A ware in the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ware {
    pub good: u32,
    /// The flag it is going to; `None` until ECO-REQ-5 gives it one.
    pub destination: Option<Point>,
    /// The building at `destination` that takes it (ECO-REQ-5). `None` for a
    /// debug ware sent to a bare flag.
    pub target: Option<Target>,
    pub location: WareLocation,
    /// The carrier that claimed it (TRN-CARRY-2.6).
    pub claimed_by: Option<usize>,
    /// ECO-DIST-1.6: re-checks counted since its route was last computed.
    pub recheck: u32,
    /// TRN-CARRY-4.4: grows by `15 - priority` with each refused entry check.
    pub wait_score: i64,
    /// TRN-CARRY-5.5, ECO-REQ-5.7: lying at a flag, it found no next flag
    /// toward its destination when its route was last computed (on
    /// arriving at the flag, or by the re-check of ECO-DIST-1.6).
    pub stranded: bool,
    /// TRN-CARRY-5.4, ECO-DIST-1.6: lying at a flag, its next flag as last
    /// computed (on arriving there, on getting a destination, by a re-check
    /// that found it no longer valid); kept while the costs change.
    pub next: Option<Point>,
    /// TRN-CARRY-2.4a: its key in the list of the flag it lies at or is
    /// admitted to: wares join in the order their places are granted.
    pub joined: u64,
    /// ECO-REQ-5.3f: the order it serves, filed with it until it is
    /// delivered.
    pub order: Option<crate::Order>,
    /// TRN-CARRY-5.4 (T39): the last answer for the flag it takes after a
    /// flag G other than its own next flag: (G, the network number then,
    /// the answer).
    pub next_after: Option<(Point, u32, Option<Point>)>,
    /// ECO-REQ-5.4a: its place in its player's free-ware list of its good,
    /// while it is on it (appended when put down at a flag without a
    /// destination, or when the order holding it is dropped while it lies
    /// at a flag; off when 5.3 (a) gives it an order). Places only grow, so
    /// the list's order is the order of the places.
    pub free_place: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WareLocation {
    /// Lying at the flag on this field.
    Flag(Point),
    /// Held by this carrier.
    Carried(usize),
}

/// Which roads a route search goes over and what a road costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Search {
    /// A settler's walk (TRN-FREE-6.1): road lengths, land roads only.
    Walk,
    /// A walk to a field of a road (TRN-FREE-6.1, Q-378) and a carrier's
    /// carry walk (TRN-CARRY-3.5a, T58): as `Walk`, but the road the walker
    /// is assigned to (this id) costs 1.
    OwnWalk(usize),
    /// A settler request's warehouse (ECO-REQ-3.2): every road, each its
    /// length L (TRN-CARRY-5.3: the 3L and 4L only in the ware network;
    /// ECO-REQ T18).
    Settler,
    /// A ware's route (TRN-CARRY-5.1, 5.1a): ware costs, only the roads
    /// with a carrier or donkey assigned (the ware networks).
    Ware,
    /// The computer player's list of route flags (AI-CON-8.3 c): as `Ware`
    /// without the 3L of a road whose carrier is busy (TRN-CARRY-5.3).
    WareList,
}

impl Search {
    fn uses(self, road: &Road) -> bool {
        match self {
            Search::Walk | Search::OwnWalk(_) => !road.water,
            Search::Settler => true,
            Search::Ware | Search::WareList => road.carrier.is_some() || road.donkey.is_some(),
        }
    }
}

/// How far the entry check of TRN-CARRY-3.2 has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// Not settled: the answer was no (or has not been asked). With `stop`
    /// the carrier stops at the next field it reaches and checks only every
    /// 10th tick from there.
    Open { stop: bool },
    /// Settled: the ware goes into the building at the flag.
    Building,
    /// Settled: exchange with this ware at the flag, claimed for the carrier.
    Exchange(usize),
    /// Settled: a place at the flag is reserved for the ware.
    Reserved,
}

/// What a carrier is doing.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierTask {
    /// No work: on the idle field or walking to it.
    Idle,
    /// Walking to the flag of a claimed ware.
    Fetch { ware: usize, flag: Point },
    /// Picking a ware up at a flag (TRN-CARRY-3.1): the drop-time action, then
    /// the pickup-time action, each ending in tick `ends`.
    PickUp {
        ware: usize,
        ends: Tick,
        second: bool,
    },
    /// Carrying its ware to the flag `to`, making the entry check for it
    /// (TRN-CARRY-3.2).
    Carry { to: Point, entry: Entry },
    /// Putting its ware down at `at` (TRN-CARRY-3.3): drop time, then pickup
    /// time. With `exchange` it takes that ware in the same two actions
    /// (TRN-CARRY-3.7). `bid`: its ware's place at the flag was granted
    /// (reserved, 3.2 step 4), its exchange ware, if any, claimed only when
    /// the put-down began.
    PutDown {
        at: Point,
        ends: Tick,
        second: bool,
        exchange: Option<usize>,
        bid: bool,
    },
    /// TRN-CARRY-1.4: out of its warehouse with `stand` (it stands one
    /// tick on the flag it comes out to, ECO-WORK-3.8d), then on its way
    /// over the roads to its road's flag, or with the walk on (T24) to its
    /// idle field, `flag`; then idle.
    Arrive { flag: Point, stand: bool },
    /// TRN-CARRY-3.3: walking into the building at the flag `flag` to hand
    /// its ware over, then (`back`) out to the flag again.
    Deliver { flag: Point, back: bool },
    /// TRN-CARRY-3.3, 3.3a: in the building at the flag `flag`, standing
    /// with its ware: in the tick its walk in ended and, with `waits`, in
    /// the next; then the building receives the ware and the walk out is
    /// ordered.
    Inside { flag: Point, waits: bool },
    /// Its road was removed (TRN-CARRY-7.1): it walks home as a settler and
    /// this record stays only to keep the carrier numbers.
    Retired,
    /// ECO-WORK-3.8d, TRN-CARRY-3.3a: standing one tick on the flag `flag`
    /// (out of the building it delivered to, or after a put-down without an
    /// exchange ware); then it acts as after a put-down.
    Out { flag: Point },
    /// TRN-CARRY-3.3a: the carrying walk ended at `at`: it stands this tick,
    /// then puts its ware down (taking `exchange` in the same actions).
    Stop { at: Point, exchange: Option<usize> },
}

/// TRN-CARRY-5.3a: the stored route costs of the ware network (kind 1),
/// per player, by flag pair in either order; "no route" is stored too. A
/// pair asked before gives its stored value; otherwise the cheapest ware
/// route is searched as the roads are now (TRN-CARRY-5.3) and stored. A
/// player's store is emptied only by a renumbering of its ware networks
/// (SIM-START-4.3 b, c). Users: the ware order matching (ECO-REQ-5.3) and
/// the computer player's route optimizer (AI-CON-8.3 b, c), to which the
/// world lends it ([`World::step_then`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteStore {
    ware: BTreeMap<PlayerId, BTreeMap<(Point, Point), Option<u32>>>,
    /// The player's ware renumberings so far (SIM-START-4.3 b, c).
    renumberings: BTreeMap<PlayerId, u32>,
}

impl RouteStore {
    /// The stored ware route cost from the flag `from` to the flag `to` of
    /// `player`, searched in `world` and stored when not asked before; a
    /// flag to itself costs 0.
    pub fn ware_cost(
        &mut self,
        world: &World,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Option<u32> {
        if from == to {
            return Some(0);
        }
        let key = (from.min(to), from.max(to));
        *self
            .ware
            .entry(player)
            .or_default()
            .entry(key)
            .or_insert_with(|| world.route_cost(player, from, to))
    }

    /// SIM-START-4.3 b, c: a ware renumbering empties the player's store.
    pub(crate) fn empty_ware(&mut self, player: PlayerId) {
        self.ware.remove(&player);
        *self.renumberings.entry(player).or_default() += 1;
    }

    /// The stored ware route cost between the flags `a` and `b` of
    /// `player` if it was asked since the store was last emptied, without
    /// asking it (`None`: not stored).
    pub fn stored(&self, player: PlayerId, a: Point, b: Point) -> Option<Option<u32>> {
        let key = (a.min(b), a.max(b));
        self.ware.get(&player)?.get(&key).copied()
    }

    /// The player's ware renumberings so far.
    pub(crate) fn renumberings(&self, player: PlayerId) -> u32 {
        self.renumberings.get(&player).copied().unwrap_or(0)
    }
}

/// The settler working a road.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carrier {
    /// TRN-CARRY-2.1: the number that phases its checks, the low 32 bits of
    /// the settler's object number (SIM-START-4.2).
    pub number: u32,
    pub road: usize,
    /// TRN-CARRY-1.2: the road's second place.
    pub donkey: bool,
    pub walker: Walker,
    pub ware: Option<usize>,
    pub task: CarrierTask,
    /// ECO-TIME-1.5: sent out by the buildings' part of this tick; the
    /// original sends road carriers in the transport part, after the
    /// settlers' movement, so it first moves in the next tick (Q-246).
    pub fresh: bool,
    /// TRN-ROAD-6.3 (T32, Q-294): released, the exchange ware it stays
    /// bound to while it walks home.
    pub bound: Option<usize>,
}

/// TRN-CARRY-6: a road's traffic count and donkey request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Traffic {
    pub ticks: u32,
    /// Wares delivered or put down this period (TRN-CARRY-3.4).
    pub transport_count: u32,
    /// TRN-CARRY-6.1 (T52): the average traffic, a float32 (its bits).
    pub average_bits: u32,
    pub donkey_requested: bool,
}

impl Traffic {
    /// TRN-CARRY-6.1 (T52): the average traffic, not rounded.
    pub fn average(&self) -> f32 {
        f32::from_bits(self.average_bits)
    }

    /// TRN-CARRY-6.1, 6.2: one tick of the road's traffic clock.
    pub fn tick(&mut self, length: u32) {
        self.ticks += 1;
        if self.ticks > TRAFFIC_PERIOD {
            // T52: ((L x count) as a float32 + average) x 0.5, kept as a
            // float32, so halves accumulate.
            let traffic = (length * self.transport_count) as f32;
            let average = (traffic + self.average()) * 0.5;
            self.average_bits = average.to_bits();
            self.ticks = 0;
            self.transport_count = 0;
            if average > DONKEY_THRESHOLD {
                self.donkey_requested = true;
            }
        }
    }
}

/// A player's flags with, for each road at them, the flag at its other end
/// and the road's id: the shape of the network the route searches walk
/// (TRN-CARRY-5.1, TRN-FREE-6.1). The cost of a road is read when a search
/// reaches it.
#[derive(Debug, Default, PartialEq, Eq)]
struct FlagRoads {
    /// The flags with a road, ascending: their indices order as their
    /// fields do.
    flags: Vec<Point>,
    /// For each flag of `flags`, its roads: the other end's index and the
    /// road's id.
    roads: Vec<Vec<(usize, usize)>>,
}

impl FlagRoads {
    fn index(&self, flag: Point) -> Option<usize> {
        self.flags.binary_search(&flag).ok()
    }
}

/// Each player's [`FlagRoads`], made when a search first needs them and
/// dropped when a road is added, removed or split. Derived from the roads,
/// so no part of the game state: not in the state hash, written as nothing
/// in a snapshot, equal in every world, and a clone starts empty.
#[derive(Default)]
pub(crate) struct RouteTopology(Mutex<BTreeMap<PlayerId, Arc<FlagRoads>>>);

impl RouteTopology {
    /// The roads changed: every player's shape is made again when needed.
    pub(crate) fn clear(&mut self) {
        self.0
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

impl Clone for RouteTopology {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for RouteTopology {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for RouteTopology {}

/// How the cheapest route from the start reaches a flag: its cost (the
/// route itself is read out back from the end, TRN-CARRY-5.4).
#[derive(Debug, Clone, Copy)]
struct RouteStep {
    cost: u32,
}

/// [`World::route_costs_to`]: the cost from each flag to one flag.
pub(crate) struct CostsTo {
    to: Point,
    graph: Arc<FlagRoads>,
    /// By flag index of `graph`.
    cost: Vec<Option<u32>>,
}

impl CostsTo {
    /// The cost from the flag `from`; `None` when it is not connected.
    pub(crate) fn from(&self, from: Point) -> Option<u32> {
        if from == self.to {
            return Some(0);
        }
        self.cost.get(self.graph.index(from)?).copied().flatten()
    }
}

/// The flags a route search settled, with how it reached each.
struct Routes {
    /// The start, when it has no road (then it is the only flag settled).
    alone: Option<(Point, RouteStep)>,
    graph: Arc<FlagRoads>,
    /// By flag index of `graph`.
    best: Vec<Option<RouteStep>>,
}

impl Routes {
    fn get(&self, flag: Point) -> Option<RouteStep> {
        match self.alone {
            Some((at, step)) => (at == flag).then_some(step),
            None => self.best[self.graph.index(flag)?],
        }
    }

    /// The settled flags in field order.
    fn iter(&self) -> impl Iterator<Item = (Point, RouteStep)> + '_ {
        let settled = self
            .graph
            .flags
            .iter()
            .zip(&self.best)
            .filter_map(|(&flag, step)| step.map(|step| (flag, step)));
        self.alone.into_iter().chain(settled)
    }
}

/// TRN-CARRY-1.3: a road's idle field is entry `floor(n / 2)` of its fields.
pub fn idle_field(path: &[Point]) -> Point {
    path[path.len() / 2]
}

/// The fields after `from` up to and including `to`, along `path`.
fn along(path: &[Point], from: Point, to: Point) -> Vec<Point> {
    let i = path
        .iter()
        .position(|&f| f == from)
        .expect("field on the road");
    let j = path
        .iter()
        .position(|&f| f == to)
        .expect("field on the road");
    if i <= j {
        path[i + 1..=j].to_vec()
    } else {
        path[j..i].iter().rev().copied().collect()
    }
}

impl World {
    /// TRN-CARRY-5.2: road length, the sum over its steps of `1/f(A->B) +
    /// 1/f(B->A)` cut to a whole number.
    pub fn road_length(&self, path: &[Point]) -> u32 {
        let twelfths: u32 = path
            .windows(2)
            .map(|pair| {
                let forth = slope_factor_percent(&self.terrain, pair[0], pair[1]);
                let back = slope_factor_percent(&self.terrain, pair[1], pair[0]);
                LENGTH_SCALE * 100 / forth + LENGTH_SCALE * 100 / back
            })
            .sum();
        twelfths / LENGTH_SCALE
    }

    /// A new carrier (or, with `donkey`, the road's donkey) standing on
    /// `at` with `task`, in the road's place for it.
    /// `number` is the settler's object number (SIM-START-4.2).
    pub(crate) fn place_carrier(
        &mut self,
        road: usize,
        donkey: bool,
        walker: Walker,
        task: CarrierTask,
        number: u32,
    ) -> usize {
        let index = self.carriers.len();
        self.carriers.push(Carrier {
            number,
            road,
            donkey,
            walker,
            ware: None,
            task,
            fresh: false,
            bound: None,
        });
        let r = self.roads[road].as_mut().expect("live road");
        if donkey {
            r.donkey = Some(index);
        } else {
            r.carrier = Some(index);
        }
        index
    }

    /// Test worlds have no warehouse: a carrier on the road's idle field.
    #[cfg(test)]
    pub(crate) fn spawn_carrier(&mut self, road: usize) {
        let path = &self.roads[road].as_ref().expect("live road").path;
        let walker = Walker::new(idle_field(path));
        // Numbered by its place among the carriers, so tests can set the
        // phase of TRN-CARRY-2.1 directly.
        let number = self.carriers.len() as u32;
        self.place_carrier(road, false, walker, CarrierTask::Idle, number);
    }

    /// Wares lying at the flag on `field`, in the order they were put down.
    pub fn wares_at(&self, field: Point) -> &[usize] {
        self.flag_wares
            .get(&field)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn ware(&self, id: usize) -> Option<&Ware> {
        self.wares.get(id)?.as_ref()
    }

    /// All wares with their ids.
    pub fn wares(&self) -> impl Iterator<Item = (usize, &Ware)> {
        self.wares
            .iter()
            .enumerate()
            .filter_map(|(id, ware)| ware.as_ref().map(|w| (id, w)))
    }

    pub fn carriers(&self) -> &[Carrier] {
        &self.carriers
    }

    /// TRN-CARRY-5.3: cost of using `road` towards its end `toward`. A place
    /// is free when it is assigned and its settler carries nothing, also
    /// while it is still on its way to the road.
    fn road_cost(&self, road: usize, toward: Point) -> u32 {
        self.road_cost_with(road, toward, true)
    }

    /// TRN-CARRY-5.3 (ECO-REQ T18): what road `road` costs a search of the
    /// kind `search` towards its flag `toward`: in the ware network 4L and
    /// 3L as 5.3 says (a list of route flags without the 3L), elsewhere its
    /// length L.
    fn search_cost(&self, search: Search, road: usize, toward: Point) -> u32 {
        match search {
            Search::OwnWalk(own) if own == road => 1,
            Search::Walk | Search::OwnWalk(_) | Search::Settler => {
                self.roads[road].as_ref().expect("live road").length
            }
            Search::WareList => self.road_cost_with(road, toward, false),
            Search::Ware => self.road_cost(road, toward),
        }
    }

    /// TRN-CARRY-5.3, with or without the 3L of a road whose places are not
    /// free (`busy`).
    fn road_cost_with(&self, road: usize, toward: Point, busy: bool) -> u32 {
        let r = self.roads[road].as_ref().expect("live road");
        if self.flag_load(toward) as u32 >= self.settings.flag_package_limit {
            return 4 * r.length;
        }
        let free = |place: Option<usize>| place.is_some_and(|c| self.carriers[c].ware.is_none());
        if !busy || free(r.carrier) || free(r.donkey) {
            r.length
        } else {
            3 * r.length
        }
    }

    /// The player's flags with the roads at them (all of the player's
    /// roads, waterways included), as kept in the route topology.
    fn flag_roads(&self, player: PlayerId) -> Arc<FlagRoads> {
        let mut cache = self
            .route_topology
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(roads) = cache.get(&player) {
            debug_assert!(
                **roads == self.make_flag_roads(player),
                "the route topology missed a change of the roads"
            );
            return Arc::clone(roads);
        }
        let roads = Arc::new(self.make_flag_roads(player));
        cache.insert(player, Arc::clone(&roads));
        roads
    }

    fn make_flag_roads(&self, player: PlayerId) -> FlagRoads {
        let mut edges: BTreeMap<Point, Vec<(Point, usize)>> = BTreeMap::new();
        for (id, road) in self.roads.iter().enumerate() {
            let Some(road) = road.as_ref().filter(|r| r.owner == player) else {
                continue;
            };
            let (a, b) = (road.path[0], *road.path.last().expect("road ends"));
            edges.entry(a).or_default().push((b, id));
            edges.entry(b).or_default().push((a, id));
        }
        let flags: Vec<Point> = edges.keys().copied().collect();
        let index = |flag: Point| flags.binary_search(&flag).expect("a flag with a road");
        let roads = edges
            .values()
            .map(|at| at.iter().map(|&(other, id)| (index(other), id)).collect())
            .collect();
        FlagRoads { flags, roads }
    }

    /// Dijkstra over the player's flag network from `from`, stopping once
    /// `to` is settled; a road costs the ware cost of TRN-CARRY-5.3 towards
    /// the end it leads to, or for a walk its length, and a walk leaves
    /// waterways out (TRN-FREE-6.1: settlers walk land roads only; wares go
    /// over both, Q-198 a). A ware's search uses only the roads with a
    /// carrier or donkey assigned (TRN-CARRY-5.1a). Equal costs are decided
    /// by field order, then by the first flag. The costs are read from the
    /// world as it is during the search, which changes nothing.
    fn cheapest(&self, player: PlayerId, search: Search, from: Point, to: Option<Point>) -> Routes {
        let graph = self.flag_roads(player);
        let start = RouteStep { cost: 0 };
        let Some(origin) = graph.index(from) else {
            return Routes {
                alone: Some((from, start)),
                graph,
                best: Vec::new(),
            };
        };
        let target = to.and_then(|to| graph.index(to));
        let mut best: Vec<Option<RouteStep>> = vec![None; graph.flags.len()];
        // Flag indices order as their fields, so the queue's ties fall as
        // they would on the fields.
        let mut queue = BinaryHeap::new();
        queue.push(Reverse((0u32, origin, origin, usize::MAX)));
        // The first flag after the start and the road's id last: ties as
        // before.
        while let Some(Reverse((cost, at, first, _road))) = queue.pop() {
            if best[at].is_some() {
                continue;
            }
            best[at] = Some(RouteStep { cost });
            if Some(at) == target {
                break;
            }
            for &(next, id) in &graph.roads[at] {
                if best[next].is_some() {
                    continue;
                }
                let r = self.roads[id].as_ref().expect("live road");
                if !search.uses(r) {
                    continue;
                }
                let step = self.search_cost(search, id, graph.flags[next]);
                let first = if at == origin { next } else { first };
                queue.push(Reverse((cost + step, next, first, id)));
            }
        }
        Routes {
            alone: None,
            graph,
            best,
        }
    }

    /// TRN-CARRY-5.1, 5.4: the first flag after `from` on the cheapest route
    /// through the player's road network to `destination`. `None` at the
    /// destination or when there is no route (TRN-CARRY-5.5). Equal costs are
    /// decided by field order.
    pub fn next_flag(&self, player: PlayerId, from: Point, destination: Point) -> Option<Point> {
        if from == destination {
            return None;
        }
        // T30: the route read out back from the destination; the next flag
        // is the last one reached before the ware's own.
        let steps = self.cheapest(player, Search::Ware, from, Some(destination));
        let legs = self.route_read_out(&steps, Search::Ware, from, destination)?;
        legs.first().map(|&(_, next, _)| next)
    }

    /// TRN-FREE-6.1 (T31), TRN-CARRY-5.4 (T30): the route from `from` to `to`
    /// of the search `steps` (of the kind `search`; only its costs count),
    /// read out back from `to`: at each flag the first of its roads in the
    /// order they were attached to it whose other flag lies on a cheapest
    /// route (its cost plus the road's cost towards this flag is this
    /// flag's). A new road is appended to its flags' lists, a split road's
    /// halves too (Q-280): the roads' creation order, their numbers. The
    /// legs (flag, next flag, road) from `from` on; `None` without a route.
    fn route_read_out(
        &self,
        steps: &Routes,
        search: Search,
        from: Point,
        to: Point,
    ) -> Option<Vec<(Point, Point, usize)>> {
        let cost = |at: Point| steps.get(at).map(|s| s.cost);
        cost(to)?;
        let graph = &steps.graph;
        let mut legs = Vec::new();
        let mut at = to;
        while at != from {
            let here = cost(at)?;
            let mut roads = graph.roads[graph.index(at)?].clone();
            roads.sort_by_key(|&(_, id)| self.roads[id].as_ref().expect("live road").number);
            let (previous, id) = roads
                .into_iter()
                .map(|(other, id)| (graph.flags[other], id))
                .find(|&(other, id)| {
                    let r = self.roads[id].as_ref().expect("live road");
                    search.uses(r)
                        && cost(other).is_some_and(|c| c + self.search_cost(search, id, at) == here)
                })?;
            legs.push((previous, at, id));
            at = previous;
        }
        legs.reverse();
        Some(legs)
    }

    /// TRN-CARRY-5.1a: the flags of `player` in one ware network with the
    /// flag `flag` (over the roads with a carrier or donkey assigned), `flag`
    /// included.
    pub fn ware_network_of(&self, player: PlayerId, flag: Point) -> BTreeSet<Point> {
        let mut seen = BTreeSet::from([flag]);
        let mut open = vec![flag];
        let ware_roads: Vec<&Road> = self
            .roads
            .iter()
            .flatten()
            .filter(|r| r.owner == player && Search::Ware.uses(r))
            .collect();
        while let Some(at) = open.pop() {
            for r in &ware_roads {
                let (a, b) = (r.path[0], *r.path.last().expect("road ends"));
                let other = if a == at {
                    b
                } else if b == at {
                    a
                } else {
                    continue;
                };
                if seen.insert(other) {
                    open.push(other);
                }
            }
        }
        seen
    }

    /// ECO-REQ-3.2: the cost of the cheapest route from the flag `from` to
    /// the flag `to` as ware routing counts it; `None` if they are not
    /// connected.
    pub fn route_cost(&self, player: PlayerId, from: Point, to: Point) -> Option<u32> {
        if from == to {
            return Some(0);
        }
        self.cheapest(player, Search::Ware, from, Some(to))
            .get(to)
            .map(|step| step.cost)
    }

    /// ECO-REQ-3.2: the costs of the cheapest routes as ware routing counts
    /// them from every flag connected to the flag `to`, found by one search
    /// back from `to`, a road costing what it costs on the way forward
    /// (towards its end nearer `to`). The cheapest cost does not depend on
    /// which of the cheapest routes has it, so each equals
    /// [`World::route_cost`] from that flag.
    pub(crate) fn route_costs_to(&self, player: PlayerId, to: Point, search: Search) -> CostsTo {
        let graph = self.flag_roads(player);
        let Some(target) = graph.index(to) else {
            return CostsTo {
                to,
                graph,
                cost: Vec::new(),
            };
        };
        let mut cost: Vec<Option<u32>> = vec![None; graph.flags.len()];
        let mut queue = BinaryHeap::new();
        queue.push(Reverse((0u32, target)));
        while let Some(Reverse((c, at))) = queue.pop() {
            if cost[at].is_some() {
                continue;
            }
            cost[at] = Some(c);
            for &(previous, id) in &graph.roads[at] {
                let r = self.roads[id].as_ref().expect("live road");
                if cost[previous].is_none() && search.uses(r) {
                    let step = self.search_cost(search, id, graph.flags[at]);
                    queue.push(Reverse((c + step, previous)));
                }
            }
        }
        CostsTo { to, graph, cost }
    }

    /// AI-CON-8.3 c: the flags of the cheapest route from the flag `from`
    /// to the flag `to` over the ware networks (TRN-CARRY-5.1a), both
    /// included, a road costing 4L towards a full flag and L otherwise (the
    /// 3L of a busy road is not counted, TRN-CARRY-5.3); `None` if they are
    /// not in one ware network. Of equally cheap routes the one read out
    /// back from `to` by each flag's road order, as a ware's next flag
    /// (AI-CON-8.3 c, T55; TRN-CARRY-5.4), made afresh at every call.
    pub fn ware_route_flags(&self, player: PlayerId, from: Point, to: Point) -> Option<Vec<Point>> {
        let routes = self.cheapest(player, Search::WareList, from, Some(to));
        let legs = self.route_read_out(&routes, Search::WareList, from, to)?;
        let mut flags = vec![from];
        flags.extend(legs.iter().map(|&(_, at, _)| at));
        Some(flags)
    }

    /// Route costs from the flag `from` to every flag it is connected to.
    pub(crate) fn route_costs_from(&self, player: PlayerId, from: Point) -> BTreeMap<Point, u32> {
        self.cheapest(player, Search::Ware, from, None)
            .iter()
            .map(|(flag, step)| (flag, step.cost))
            .collect()
    }

    /// The fields a settler walks over the roads from the flag `from` to the
    /// flag `to`, `to` included and `from` not. TRN-FREE-6.1: the cheapest
    /// route over the flag network without the penalties for full flags and
    /// busy roads, so by road length (TRN-CARRY-5.2).
    pub(crate) fn road_walk(&self, player: PlayerId, from: Point, to: Point) -> Option<Vec<Point>> {
        self.road_walk_by(player, Search::Walk, from, to)
    }

    /// [`World::road_walk`] by the search `search` (a walk's: `Walk`, or
    /// `OwnWalk` with the walker's road at 1).
    fn road_walk_by(
        &self,
        player: PlayerId,
        search: Search,
        from: Point,
        to: Point,
    ) -> Option<Vec<Point>> {
        if from == to {
            return Some(Vec::new());
        }
        let steps = self.cheapest(player, search, from, Some(to));
        // T31: only the costs count; the route is read out back from `to`.
        let legs = self.route_read_out(&steps, search, from, to)?;
        Some(self.leg_fields(&legs))
    }

    /// TRN-FREE-6.1 (T43): a road walk from `field`, an inner field of the
    /// road `path`, to the flag `to`. The search starts at the road's first
    /// flag A: if the route's next flag is the road's other flag B, he walks
    /// there along the road directly, else along the road to A and on from
    /// there. Which end is nearer does not count.
    pub(crate) fn road_walk_from_road_field(
        &self,
        player: PlayerId,
        field: Point,
        path: &[Point],
        to: Point,
    ) -> Option<Vec<Point>> {
        self.road_walk_from_road_field_by(player, Search::Walk, field, path, to)
    }

    /// [`World::road_walk_from_road_field`] by the search `search`.
    fn road_walk_from_road_field_by(
        &self,
        player: PlayerId,
        search: Search,
        field: Point,
        path: &[Point],
        to: Point,
    ) -> Option<Vec<Point>> {
        let (first, other) = (path[0], *path.last()?);
        let legs = if first == to {
            Vec::new()
        } else {
            let steps = self.cheapest(player, search, first, Some(to));
            self.route_read_out(&steps, search, first, to)?
        };
        let mut walk;
        if legs.first().is_some_and(|&(_, next, _)| next == other) {
            walk = along(path, field, other);
            walk.extend(self.leg_fields(&legs[1..]));
        } else {
            walk = along(path, field, first);
            walk.extend(self.leg_fields(&legs));
        }
        Some(walk)
    }

    /// The fields of the legs (flag, next flag, road) of a route, each next
    /// flag included and the first flag not.
    fn leg_fields(&self, legs: &[(Point, Point, usize)]) -> Vec<Point> {
        let mut fields = Vec::new();
        for &(a, b, id) in legs {
            let path = &self.roads[id].as_ref().expect("live road").path;
            fields.extend(along(path, a, b));
        }
        fields
    }

    /// TRN-CARRY-4.1: the most wares a flag holds.
    pub fn flag_package_limit(&self) -> usize {
        self.settings.flag_package_limit as usize
    }

    /// TRN-CARRY-4.1, 4.3: wares lying at the flag plus the places reserved
    /// for wares on their way.
    pub fn flag_load(&self, flag: Point) -> usize {
        self.wares_at(flag).len() + self.flag_reserved.get(&flag).copied().unwrap_or(0) as usize
    }

    /// TRN-CARRY-4: whether the flag on `flag` can take ware `ware`.
    pub fn flag_has_room(&self, flag: Point, ware: usize) -> bool {
        let destination = self.wares[ware].as_ref().expect("live ware").destination;
        self.flag_has_room_for(flag, destination)
    }

    /// TRN-CARRY-4: whether the flag on `flag` can take a ware going to
    /// `destination`.
    pub fn flag_has_room_for(&self, flag: Point, destination: Option<Point>) -> bool {
        let next = self
            .flag_at(flag)
            .and_then(|owner| destination.and_then(|d| self.next_flag(owner, flag, d)));
        self.flag_has_room_next(flag, next)
    }

    /// TRN-CARRY-4: whether the flag on `flag` can take a ware that goes on
    /// to `next` from there (none: no destination or no route).
    fn flag_has_room_next(&self, flag: Point, next: Option<Point>) -> bool {
        if self.flag_at(flag).is_none() {
            return false;
        }
        let load = self.flag_load(flag);
        let limit = self.settings.flag_package_limit as usize;
        if load >= limit {
            return false;
        }
        if load + 1 < limit {
            return true;
        }
        // 4.2: the last place. A ware without a destination or a route only
        // takes it; any other needs one of the wares there to leave in
        // another direction. A road carrier's reserved ware always does;
        // 4.2a: one a worker or helper brings out of its building never
        // does (no next flag until it is put down).
        let Some(next) = next else {
            return true;
        };
        // The wares there by the next flags they keep (TRN-CARRY-5.4).
        self.carrier_reserved(flag)
            || self.wares_at(flag).iter().any(|&other| {
                let other = self.wares[other].as_ref().expect("live ware");
                other.destination.is_some() && other.next.is_some_and(|n| n != next)
            })
    }

    /// TRN-CARRY-5.4 (T39): the flag the ware takes after the flag `g`, for
    /// the last-place test of 4.2 and the road swap of 3.6: the last answer
    /// while asked from the same `g` with its ware network numbered as then,
    /// else searched with the costs of now and remembered. (Stand-in for
    /// `g`'s ware-network number: the count of the player's ware
    /// renumberings, which changes with it, and with any other.)
    fn next_flag_after(&mut self, ware: usize, g: Point) -> Option<Point> {
        let w = self.wares[ware].as_ref()?;
        let destination = w.destination?;
        let owner = self.flag_at(g)?;
        let number = self.route_store.renumberings(owner);
        if let Some((at, n, answer)) = w.next_after {
            if (at, n) == (g, number) {
                return answer;
            }
        }
        let answer = self.next_flag(owner, g, destination);
        self.wares[ware].as_mut()?.next_after = Some((g, number, answer));
        answer
    }

    /// TRN-CARRY-4.2, 4.3: whether a road carrier holds a reserved place at
    /// `flag` (cleared to bring its ware there, or putting it down), as
    /// counted by `rebuild_reservations`.
    fn carrier_reserved(&self, flag: Point) -> bool {
        self.carriers.iter().any(|c| match c.task {
            CarrierTask::Carry {
                to,
                entry: Entry::Reserved,
            } => to == flag,
            // T53 (Q-377): also when it claimed an exchange ware as its
            // put-down began, its place having been reserved (`bid`).
            CarrierTask::PutDown {
                at,
                second: false,
                exchange,
                bid,
                ..
            } => at == flag && (exchange.is_none() || bid),
            // T40: in its arrival tick, standing on the flag, too.
            CarrierTask::Stop { at, exchange: None } => at == flag,
            // T60: standing with the ware a building did not take.
            CarrierTask::Out { flag: at } => at == flag && c.ware.is_some(),
            _ => false,
        })
    }

    /// TRN-CARRY-4.4, steps 3 and 4 of the entry check: whether the ware
    /// of `good` bound for `destination` is admitted to the flag now, with
    /// its wait score in `score`. Admitting reserves the place.
    pub(crate) fn flag_bid(
        &mut self,
        flag: Point,
        good: u32,
        destination: Option<Point>,
        score: &mut i64,
    ) -> bool {
        let next = self
            .flag_at(flag)
            .and_then(|owner| destination.and_then(|d| self.next_flag(owner, flag, d)));
        self.flag_bid_next(flag, good, next, score)
    }

    /// [`World::flag_bid`] for a ware that goes on to `next` from `flag`.
    fn flag_bid_next(
        &mut self,
        flag: Point,
        good: u32,
        next: Option<Point>,
        score: &mut i64,
    ) -> bool {
        if !self.flag_has_room_next(flag, next) {
            let owner = self.flag_at(flag).expect("a flag");
            *score += (15 - self.rank(owner, good)).max(0);
            return false;
        }
        let bar = self.flag_bar.get(&flag).copied().unwrap_or(-1);
        if *score > bar {
            self.flag_bar.insert(flag, *score);
            false
        } else if *score < bar {
            false
        } else {
            self.flag_bar.remove(&flag);
            *self.flag_reserved.entry(flag).or_default() += 1;
            *score = 0;
            true
        }
    }

    /// ECO-WORK-3.4 / TRN-CARRY-4.3: a worker or helper's place at the
    /// flag, taken at once when nobody is bidding for the flag (no bar).
    /// Otherwise as `flag_bid`.
    pub(crate) fn flag_reserve_direct(&mut self, flag: Point, good: u32, score: &mut i64) -> bool {
        if !self.flag_bar.contains_key(&flag) && self.flag_has_room_for(flag, None) {
            *self.flag_reserved.entry(flag).or_default() += 1;
            return true;
        }
        self.flag_bid(flag, good, None, score)
    }

    /// A reserved place at the flag is used or given up.
    pub(crate) fn unreserve(&mut self, flag: Point) {
        if let Some(n) = self.flag_reserved.get_mut(&flag) {
            *n = n.saturating_sub(1);
        }
    }

    /// TRN-CARRY-4.4: a ware left the flag (picked up or destroyed).
    fn ware_left(&mut self, flag: Point) {
        self.flag_bar.remove(&flag);
    }

    /// TRN-CARRY-4.3: the reserved places of this tick: carriers cleared
    /// to put a ware down and workers cleared to bring a product out.
    pub(crate) fn rebuild_reservations(&mut self) {
        let mut reserved: BTreeMap<Point, u32> = BTreeMap::new();
        for c in &self.carriers {
            match c.task {
                CarrierTask::Carry {
                    to,
                    entry: Entry::Reserved,
                } => *reserved.entry(to).or_default() += 1,
                // TRN-CARRY-4.3, 3.7 (T53, Q-377): a reserved place
                // counts through the drop-time action, also when the
                // carrier claimed an exchange ware as it began (that ware
                // still lies at the flag, so the flag counts both).
                CarrierTask::PutDown {
                    at,
                    second: false,
                    exchange,
                    bid,
                    ..
                } if exchange.is_none() || bid => *reserved.entry(at).or_default() += 1,
                // TRN-CARRY-4.3 (T40): in its arrival tick, standing on the
                // flag before the put-down, its place still counts.
                CarrierTask::Stop { at, exchange: None } => *reserved.entry(at).or_default() += 1,
                // T60: standing with the ware a building did not take, the
                // place taken at its arrival.
                CarrierTask::Out { flag } if c.ware.is_some() => {
                    *reserved.entry(flag).or_default() += 1
                }
                _ => {}
            }
        }
        for flag in self.worker_reservations() {
            *reserved.entry(flag).or_default() += 1;
        }
        self.flag_reserved = reserved;
    }

    /// TRN-CARRY-2.3, 2.4: the best ware at `flag` ready for `road`, and its
    /// priority.
    fn best_ware(&self, road: usize, flag: Point) -> Option<(usize, i64)> {
        let r = self.roads[road].as_ref().expect("live road");
        let other = if r.path[0] == flag {
            *r.path.last().expect("road ends")
        } else {
            r.path[0]
        };
        let mut best: Option<(usize, i64)> = None;
        for &id in self.wares_at(flag) {
            let ware = self.wares[id].as_ref().expect("live ware");
            // TRN-CARRY-2.4, ECO-DIST-4.3: the owner's transport rank.
            let priority = self.rank(r.owner, ware.good);
            // Strictly smaller: the first listed wins among equals. A ware
            // that cannot win is not routed.
            if best.is_some_and(|(_, p)| priority >= p) {
                continue;
            }
            // TRN-CARRY-5.4: by the next flag it keeps.
            let ready =
                ware.claimed_by.is_none() && ware.destination.is_some() && ware.next == Some(other);
            if ready {
                best = Some((id, priority));
            }
        }
        best
    }

    /// TRN-CARRY-2.5: the ware a carrier should fetch, with its flag.
    fn choose_work(&self, carrier: usize) -> Option<(usize, Point)> {
        let c = &self.carriers[carrier];
        let road = self.roads[c.road].as_ref().expect("live road");
        let (first, second) = (road.path[0], *road.path.last().expect("road ends"));
        // 1. Standing on one of its flags: the best ware there.
        if c.walker.next.is_none() && (c.walker.at == first || c.walker.at == second) {
            if let Some((ware, _)) = self.best_ware(c.road, c.walker.at) {
                return Some((ware, c.walker.at));
            }
        }
        // 2. The better end; ties go to the second flag.
        let at_first = self.best_ware(c.road, first);
        let at_second = self.best_ware(c.road, second);
        let p1 = at_first.map_or(NO_WARE_PRIORITY, |(_, p)| p);
        let p2 = at_second.map_or(NO_WARE_PRIORITY, |(_, p)| p);
        if p1 == NO_WARE_PRIORITY && p2 == NO_WARE_PRIORITY {
            return None;
        }
        if p2 <= p1 {
            at_second.map(|(w, _)| (w, second))
        } else {
            at_first.map(|(w, _)| (w, first))
        }
    }

    /// Walk a carrier along its road to `to` from the field it last reached;
    /// a step in progress is dropped (TRN-CARRY-2.2, TRN-FREE-4.3).
    /// TRN-CARRY-1.4, ECO-WORK-3.8d (T24): the fields from `at` over the
    /// player's roads to the carrier's road, through the end with the
    /// shorter walk, and along its road to its idle field (TRN-CARRY-1.3).
    fn walk_to_idle_field(&self, carrier: usize, at: Point) -> Option<Vec<Point>> {
        let own = self.carriers[carrier].road;
        let road = self.roads[own].as_ref()?;
        let idle = idle_field(&road.path);
        if self.flag_at(at).is_some() {
            return self.road_walk_onto(road.owner, own, at, &road.path, idle);
        }
        // TRN-FREE-6.1 (T43), TRN-CARRY-3.6a: from a field of another road (a
        // swap has just given him this road): the search starts at that road's
        // first flag; if its next flag is the road's other flag he goes there
        // along the road directly, else to the first flag and on.
        if let Some(on) = self.road_at(at) {
            let (first, other) = (on.path[0], *on.path.last()?);
            let goal = road.path[0];
            let search = Search::OwnWalk(own);
            let next = (first != goal)
                .then(|| {
                    let steps = self.cheapest(road.owner, search, first, Some(goal));
                    self.route_read_out(&steps, search, first, goal)
                })
                .flatten()
                .and_then(|legs| legs.first().map(|&(_, next, _)| next));
            let by = if next == Some(other) { other } else { first };
            let mut walk = along(&on.path, at, by);
            walk.extend(self.road_walk_onto(road.owner, own, by, &road.path, idle)?);
            return Some(walk);
        }
        // Off the flags (a building's field): the end with the shorter walk
        // (not measured).
        let ends = [road.path[0], *road.path.last()?];
        ends.into_iter()
            .filter_map(|end| {
                let mut walk = self.road_walk_from(road.owner, at, end)?;
                walk.extend(along(&road.path, end, idle));
                Some(walk)
            })
            .min_by_key(Vec::len)
    }

    /// TRN-FREE-6.5 a, b, g: a road walk from the flag `at` to the field
    /// `to`, fixed when the walk was ordered: a flag, or a field of a road,
    /// reached over one of that road's ends. `own`: the road the walker is
    /// assigned to.
    fn road_walk_to_field(
        &self,
        player: PlayerId,
        own: usize,
        at: Point,
        to: Point,
    ) -> Option<Vec<Point>> {
        if self.flag_at(to).is_some() {
            return self.road_walk(player, at, to);
        }
        let path = &self.road_at(to)?.path;
        self.road_walk_onto(player, own, at, path, to)
    }

    /// TRN-FREE-6.1 (Q-378, T42): a road walk from the flag `at` to the
    /// field `to` of the road `path`. Its goal is the road's first flag A,
    /// by the walk search with the walker's road `own` at cost 1; if that
    /// route comes into A from the road's other flag B, the walk ends at B.
    /// From there it walks along the road to `to`.
    fn road_walk_onto(
        &self,
        player: PlayerId,
        own: usize,
        at: Point,
        path: &[Point],
        to: Point,
    ) -> Option<Vec<Point>> {
        let (first, other) = (path[0], *path.last()?);
        let search = Search::OwnWalk(own);
        let mut legs = if at == first {
            Vec::new()
        } else {
            let steps = self.cheapest(player, search, at, Some(first));
            // T41: on equal costs the road into A attached there first.
            self.route_read_out(&steps, search, at, first)?
        };
        // T42: into A from B, the walk ends at B.
        if legs
            .last()
            .is_some_and(|&(previous, _, _)| previous == other)
        {
            legs.pop();
        }
        let end = legs.last().map_or(at, |&(_, next, _)| next);
        let mut walk = self.leg_fields(&legs);
        walk.extend(along(path, end, to));
        Some(walk)
    }

    fn route_carrier(&mut self, carrier: usize, to: Point) {
        let road = self.carriers[carrier].road;
        let (path, owner) = {
            let r = self.roads[road].as_ref().expect("live road");
            (r.path.clone(), r.owner)
        };
        // TRN-CARRY-3.5a: off its road (after carrying along another road)
        // it goes by a road walk.
        let walker = &self.carriers[carrier].walker;
        if !path.contains(&walker.at) && !walker.next.is_some_and(|n| path.contains(&n)) {
            let at = walker.at;
            if let Some(walk) = self.road_walk_to_field(owner, road, at, to) {
                self.carriers[carrier].walker.walk(walk);
            }
            return;
        }
        let walker = &mut self.carriers[carrier].walker;
        // TRN-ROAD-4.2: given the half its step goes to while the field it
        // left is on the other, it finishes that step first.
        if let Some(next) = walker
            .next
            .filter(|n| !path.contains(&walker.at) && path.contains(n))
        {
            walker.path = along(&path, next, to).into();
            walker.pauses.clear();
            walker.road_to = None;
            walker.held = false;
            return;
        }
        let from = walker.at;
        walker.walk(along(&path, from, to));
    }

    /// TRN-CARRY-3.5a (T46, T47): leaving the flag `at` with its ware, the
    /// carrier carries it towards the next flag the ware holds: normally its
    /// road's far end `far`; else by a road walk there, which ends at the
    /// first flag reached, where it acts as at the end of its own road.
    fn start_carry(&mut self, carrier: usize, at: Point, far: Point) {
        let ware = self.carriers[carrier].ware.expect("a carried ware");
        let next = self.wares[ware].as_ref().and_then(|w| w.next);
        let own = self.carriers[carrier].road;
        let owner = self.roads[own].as_ref().expect("live road").owner;
        // T58: the walk's search counts his own road as 1.
        let walk = next
            .filter(|&n| n != far)
            .and_then(|n| self.road_walk_by(owner, Search::OwnWalk(own), at, n));
        let to = match walk {
            Some(walk) => {
                let flags = &self.flags;
                let terrain = &self.terrain;
                let walker = &mut self.carriers[carrier].walker;
                walker.walk(walk);
                walker.end_at_next(|f| flags[terrain.index(f)].is_some());
                walker.destination()
            }
            None => {
                self.route_carrier(carrier, far);
                far
            }
        };
        self.carriers[carrier].task = CarrierTask::Carry {
            to,
            entry: Entry::Open { stop: false },
        };
    }

    fn start_fetch(&mut self, carrier: usize, ware: usize, flag: Point, tick: Tick) {
        self.wares[ware].as_mut().expect("live ware").claimed_by = Some(carrier);
        // TRN-CARRY-2.2: if the field it last reached is the ware's flag, it
        // is there at once.
        if self.carriers[carrier].walker.at == flag {
            self.carriers[carrier].walker.walk([]);
            self.carriers[carrier].task = CarrierTask::PickUp {
                ware,
                ends: tick + Tick::from(self.settings.drop_ticks),
                second: false,
            };
        } else {
            self.carriers[carrier].task = CarrierTask::Fetch { ware, flag };
            self.route_carrier(carrier, flag);
        }
    }

    /// One tick of a carrier: movement, then job logic (TRN-MOVE-2.1), in
    /// the carriers' list (ECO-REQ-0.2).
    pub(crate) fn update_carrier(&mut self, carrier: usize, tick: Tick) {
        if std::mem::take(&mut self.carriers[carrier].fresh) {
            return;
        }
        // TRN-CARRY-3.2 (T42): on its step onto the flag it carries to and
        // refused there (its exchange undone on the way, 3.7), it cancels
        // the step in the next tick: it stands on the last field before the
        // flag, back on its point, and makes no check in that tick; the
        // checks of the 10-tick grid follow. Unless that tick's movement
        // completes the step (T51): it then waits on the flag.
        if let CarrierTask::Carry {
            to,
            entry: Entry::Open { stop: true },
        } = self.carriers[carrier].task
        {
            let walker = &mut self.carriers[carrier].walker;
            if walker.next == Some(to) && !walker.completes_step() {
                walker.next = None;
                walker.path.push_front(to);
                walker.progress_bits = 0;
                walker.held = true;
                return;
            }
        }
        let walking = self.carriers[carrier].walker.is_walking();
        let stepping = self.carriers[carrier].walker.next.is_some();
        self.move_carrier(carrier);
        // TRN-CARRY-2.2 (T33): the tick its walk to the idle field ends
        // is no look tick, even on its grid: it only becomes a standing
        // carrier there.
        if walking && self.stands_idle_on_idle_field(carrier) {
            return;
        }
        let walker = &self.carriers[carrier].walker;
        let arrived = walking && !walker.is_walking();
        // A step ended and the walk goes no further: stopped short.
        let came_to_stop = stepping && walker.next.is_none() && walker.is_walking();
        self.carrier_job(carrier, tick, arrived, came_to_stop);
    }

    /// TRN-CARRY-2.2: an idle carrier standing on its road's idle field.
    fn stands_idle_on_idle_field(&self, carrier: usize) -> bool {
        let c = &self.carriers[carrier];
        c.task == CarrierTask::Idle
            && !c.walker.is_walking()
            && self.roads[c.road]
                .as_ref()
                .is_some_and(|r| c.walker.at == idle_field(&r.path))
    }

    fn move_carrier(&mut self, carrier: usize) {
        let c = &self.carriers[carrier];
        if matches!(c.task, CarrierTask::Retired) {
            return;
        }
        // TRN-CARRY-3.2 (T35): a carrier that may not enter walks on over
        // the road's inner fields and stops on the last one before its
        // target flag; it steps onto the flag only when settled.
        let gate = match c.task {
            CarrierTask::Carry { to, entry } => Some((to, entry)),
            _ => None,
        };
        // W19: on its own paved road a carrier or donkey walks faster.
        let base = self.paved_speed(
            c.road,
            c.walker.at,
            c.walker.path.front().copied(),
            self.settings.settler_speed,
        );
        let may_enter = |field| match gate {
            Some((to, entry)) => field != to || !matches!(entry, Entry::Open { .. }),
            None => true,
        };
        if c.walker.road_to.is_none() {
            let terrain = &self.terrain;
            self.carriers[carrier]
                .walker
                .advance(terrain, base, may_enter);
            return;
        }
        // TRN-FREE-6.5 g: its walk to its road searches again at every flag.
        let (own, at) = (c.road, c.walker.at);
        let owner = self.roads[own].as_ref().map(|r| r.owner);
        let mut walker = std::mem::replace(
            &mut self.carriers[carrier].walker,
            crate::movement::Walker::new(at),
        );
        walker.advance_with(&self.terrain, base, may_enter, &mut |walker| {
            if let Some(owner) = owner {
                let route = |at, to| self.road_walk_to_field(owner, own, at, to);
                self.road_walk_arrived(owner, walker, &route);
            }
        });
        self.carriers[carrier].walker = walker;
    }

    /// One tick of the carrier's job; `arrived`: its walk ended in this
    /// tick's movement; `came_to_stop`: this tick's movement ended a step
    /// and its walk stopped short there.
    fn carrier_job(&mut self, carrier: usize, tick: Tick, arrived: bool, came_to_stop: bool) {
        if matches!(self.carriers[carrier].task, CarrierTask::Retired) {
            return;
        }
        let walked_in = arrived;
        let number = u64::from(self.carriers[carrier].number);
        let task = self.carriers[carrier].task;
        let standing = self.carriers[carrier].walker.next.is_none();
        let at = self.carriers[carrier].walker.at;
        let road_path = self.roads[self.carriers[carrier].road]
            .as_ref()
            .expect("live road")
            .path
            .clone();
        let idle = idle_field(&road_path);
        let other_end = |flag: Point| {
            if road_path[0] == flag {
                *road_path.last().expect("road ends")
            } else {
                road_path[0]
            }
        };

        // TRN-FREE-6.3, TRN-ROAD-4.2: standing off its road with nothing to
        // carry (given the half of a split road it was not on, while in a
        // building), its idle step orders the walk to the idle field, a road
        // walk from where it stands, as on its way to its road.
        let off_road =
            standing && !self.carriers[carrier].walker.is_walking() && !road_path.contains(&at);
        let empty_handed = self.carriers[carrier].ware.is_none();
        if off_road && empty_handed && matches!(task, CarrierTask::Idle | CarrierTask::Out { .. }) {
            if let Some(walk) = self.walk_to_idle_field(carrier, at) {
                let walker = &mut self.carriers[carrier].walker;
                walker.walk(walk);
                walker.road_to = Some(idle);
            }
            self.carriers[carrier].task = CarrierTask::Arrive {
                flag: idle,
                stand: false,
            };
            return;
        }
        match task {
            CarrierTask::Retired => {}
            CarrierTask::Arrive { flag, stand } => {
                let walker = &self.carriers[carrier].walker;
                if walker.ended_short() {
                    // TRN-FREE-6.3 (Q-272): its idle step orders the walk to
                    // the idle field again, a new road walk from where it
                    // stands, in every check; with none possible it stays
                    // (what the job then does is not traced).
                    let to = walker.road_to.expect("a road walk");
                    let own = self.carriers[carrier].road;
                    let owner = self.roads[own].as_ref().map(|r| r.owner);
                    let route = owner.and_then(|owner| self.road_walk_to_field(owner, own, at, to));
                    if route.is_some() {
                        self.carriers[carrier].walker.reroute(route);
                    }
                } else if standing && !walker.is_walking() {
                    if stand {
                        // ECO-WORK-3.8d, TRN-CARRY-1.3, 1.4: one tick on the
                        // flag it came out to, its road's or not.
                        self.carriers[carrier].task = CarrierTask::Arrive { flag, stand: false };
                        // TRN-ROAD-4.2 (T31): a fetch claim kept over a split
                        // goes in the arrival tick.
                        for w in self.wares.iter_mut().flatten() {
                            if w.claimed_by == Some(carrier) {
                                w.claimed_by = None;
                            }
                        }
                    } else if at == flag {
                        // On its road: to the idle field, looking for work
                        // on the way (TRN-CARRY-2.2). T37: out of a
                        // warehouse onto one of its road's two flags it looks
                        // once there first and takes a ready ware at once.
                        self.carriers[carrier].task = CarrierTask::Idle;
                        let on_end =
                            road_path.first() == Some(&at) || road_path.last() == Some(&at);
                        if let Some((ware, from)) =
                            on_end.then(|| self.choose_work(carrier)).flatten()
                        {
                            self.start_fetch(carrier, ware, from, tick);
                            return;
                        }
                        self.route_carrier(carrier, idle);
                    } else if let Some(walk) = self.walk_to_idle_field(carrier, at) {
                        // T24: over the roads to its road and on to the idle
                        // field as one walk (the excess carries on at its
                        // road's flag), ordered here at progress 0.
                        self.carriers[carrier].walker.walk(walk);
                        self.carriers[carrier].walker.road_to = Some(idle);
                        self.carriers[carrier].task = CarrierTask::Arrive {
                            flag: idle,
                            stand: false,
                        };
                    }
                }
            }
            CarrierTask::Deliver { flag, back } => {
                let walker = &self.carriers[carrier].walker;
                if !standing || walker.is_walking() {
                    return;
                }
                if !back {
                    // TRN-CARRY-3.3a: the tick the walk in ends is spent
                    // standing, and the next too but in a building with a
                    // door offset (on its door field; 9d3c62b, T27).
                    // TRN-CARRY-3.3 (T60): with the building gone meanwhile
                    // the next tick is the hand-over tick, the walk out
                    // ordered in it: one standing tick.
                    let door = self
                        .terrain
                        .neighbour(flag, 4)
                        .is_some_and(|b| self.door_field(b) != b);
                    let gone = !self.carriers[carrier]
                        .ware
                        .and_then(|w| self.wares[w].as_ref())
                        .and_then(|w| w.target)
                        .is_some_and(|t| self.target_building(t).is_some());
                    self.carriers[carrier].task = CarrierTask::Inside {
                        flag,
                        waits: !door && !gone,
                    };
                } else if self.carriers[carrier].ware.is_some() {
                    // T60 (Q-230 c): back with the ware the removed building
                    // did not take (its place taken now, as a granted one).
                    // TRN-CARRY-3.3a: he stands in A as after any delivery;
                    // from A + 1 he puts it down as at the end of a carrying
                    // walk (`Out` with a ware in hand, then `Stop`).
                    *self.flag_reserved.entry(flag).or_default() += 1;
                    let key = self.take_join();
                    if let Some(ware) = self.carriers[carrier].ware {
                        self.wares[ware].as_mut().expect("live ware").joined = key;
                    }
                    self.carriers[carrier].task = CarrierTask::Out { flag };
                } else {
                    // ECO-WORK-3.8d: back on the flag it stands this tick.
                    self.carriers[carrier].task = CarrierTask::Out { flag };
                }
            }
            CarrierTask::Inside { flag, waits: true } => {
                self.carriers[carrier].task = CarrierTask::Inside { flag, waits: false };
            }
            CarrierTask::Inside { flag, waits: false } => {
                // TRN-CARRY-3.3, 3.3a: the building receives the ware and the
                // walk out to the flag starts at progress 0 (from a door
                // field over the building's field).
                let ware = self.carriers[carrier].ware.expect("a carried ware");
                let road = self.carriers[carrier].road;
                if let Some(r) = self.roads[road].as_mut() {
                    r.traffic.transport_count += 1;
                }
                let target = self.wares[ware].as_ref().and_then(|w| w.target);
                if target.is_some_and(|t| self.target_building(t).is_some()) {
                    self.carriers[carrier].ware = None;
                    let w = self.wares[ware].take().expect("live ware");
                    self.hand_over(target.expect("a target"), w.good, w.order);
                } else if let Some(w) = self.wares[ware].as_mut() {
                    // T60 (Q-230 c): the building went while he walked in:
                    // the hand-over finds no order, nothing is delivered and
                    // the ware stays in his hands, bound for nothing; he
                    // brings it back out and puts it down at the flag.
                    w.target = None;
                    w.order = None;
                    w.destination = None;
                }
                self.carriers[carrier].task = CarrierTask::Deliver { flag, back: true };
                let building = self.terrain.neighbour(flag, 4).unwrap_or(flag);
                let walker = &mut self.carriers[carrier].walker;
                if walker.at == building {
                    walker.walk([flag]);
                } else {
                    // ECO-WORK-3.8b: the door step is a walk of its own.
                    walker.walk([building, flag]);
                    walker.pauses = vec![building];
                }
            }
            CarrierTask::Out { flag } if self.carriers[carrier].ware.is_some() => {
                // TRN-CARRY-3.3a (T60): the ware a building did not take,
                // from A + 1 put down as at the end of a carrying walk: it
                // stands this tick, the drop-time action from the next.
                self.carriers[carrier].task = CarrierTask::Stop {
                    at: flag,
                    exchange: None,
                };
            }
            CarrierTask::Out { .. } => {
                // Then as after putting down (TRN-CARRY-3.5).
                self.carriers[carrier].task = CarrierTask::Idle;
                if let Some((ware, flag)) = self.choose_work(carrier) {
                    self.start_fetch(carrier, ware, flag, tick);
                } else {
                    self.route_carrier(carrier, idle);
                }
            }
            CarrierTask::Idle => {
                // TRN-CARRY-2.2 (corrected, 03543ac): during its walk to the
                // idle field it does not look for work.
                if self.carriers[carrier].walker.is_walking() {
                    return;
                }
                let on_idle_field = standing && at == idle;
                // TRN-CARRY-2.1 on the idle field; off it, once, then the
                // walk to the idle field.
                let look = !on_idle_field || (number + tick).is_multiple_of(IDLE_LOOK_PERIOD);
                if look {
                    if let Some((ware, flag)) = self.choose_work(carrier) {
                        self.start_fetch(carrier, ware, flag, tick);
                        return;
                    }
                }
                if !on_idle_field {
                    self.route_carrier(carrier, idle);
                }
            }
            CarrierTask::Fetch { ware, flag } => {
                if standing && at == flag && !self.carriers[carrier].walker.is_walking() {
                    // ECO-REQ-5.3e (Q-282): a ware redirected while it was
                    // fetched lost this carrier's claim: finding no ware
                    // claimed by it at the flag, the carrier stands in the
                    // arrival tick and chooses work again in the next.
                    let mine = self.wares[ware].as_ref().is_some_and(|w| {
                        w.claimed_by == Some(carrier) && w.location == WareLocation::Flag(flag)
                    });
                    if !mine {
                        self.carriers[carrier].task = CarrierTask::Idle;
                        return;
                    }
                    self.carriers[carrier].task = CarrierTask::PickUp {
                        ware,
                        ends: tick + Tick::from(self.settings.drop_ticks),
                        second: false,
                    };
                } else if standing && !self.carriers[carrier].walker.is_walking() && !arrived {
                    // TRN-ROAD-4.2 (T27): his walk ended short of the ware's
                    // flag (on a split's new flag): he stood there in the
                    // arrival tick; now the walk on, at progress 0.
                    self.route_carrier(carrier, flag);
                }
            }
            CarrierTask::PickUp { ware, ends, second } => {
                if tick < ends {
                    return;
                }
                if !second {
                    // ECO-REQ-5.3e (Q-285, 498fa91): he takes the ware only
                    // while it is still linked to him; a claim released by
                    // the matching's redirect leaves him without it, and he
                    // plays the pick-up half all the same.
                    let mine = self.wares[ware].as_ref().is_some_and(|w| {
                        w.claimed_by == Some(carrier) && w.location == WareLocation::Flag(at)
                    });
                    if !mine {
                        self.carriers[carrier].task = CarrierTask::PickUp {
                            ware,
                            ends: tick + Tick::from(self.settings.pickup_ticks),
                            second: true,
                        };
                        return;
                    }
                    // TRN-CARRY-3.1: the ware leaves the flag when the drop
                    // action ends and is held by the carrier.
                    if let Some(list) = self.flag_wares.get_mut(&at) {
                        list.retain(|&w| w != ware);
                    }
                    self.ware_left(at);
                    let w = self.wares[ware].as_mut().expect("live ware");
                    w.location = WareLocation::Carried(carrier);
                    w.wait_score = 0;
                    self.carriers[carrier].ware = Some(ware);
                    self.carriers[carrier].task = CarrierTask::PickUp {
                        ware,
                        ends: tick + Tick::from(self.settings.pickup_ticks),
                        second: true,
                    };
                    return;
                }
                if self.carriers[carrier].ware != Some(ware) {
                    // ECO-REQ-5.3e (Q-285): without a ware he goes on as a
                    // carrier without one (TRN-CARRY-2.2).
                    self.carriers[carrier].task = CarrierTask::Idle;
                    return;
                }
                // TRN-CARRY-3.2 (T36): the first entry check comes in the
                // tick after the walk is ordered.
                self.start_carry(carrier, at, other_end(at));
            }
            CarrierTask::Carry { to, entry } => {
                let arrived = standing && at == to && !self.carriers[carrier].walker.is_walking();
                // TRN-CARRY-3.7 (T38): an exchange ware whose claim went on
                // the way (ECO-REQ-5.3e) undoes the exchange at once; still
                // walking, the carrier checks again as one not settled.
                // TRN-CARRY-3.3 (T59): the building case is settled only
                // while the building takes the ware, tested every tick.
                let ware = self.carriers[carrier].ware;
                let entry = match entry {
                    Entry::Exchange(other) if !arrived && !self.claimed_at(other, carrier, to) => {
                        let open = Entry::Open { stop: false };
                        self.carriers[carrier].task = CarrierTask::Carry { to, entry: open };
                        open
                    }
                    Entry::Building
                        if !arrived
                            && ware.is_some_and(|w| self.delivers_here(w, to).is_none()) =>
                    {
                        let open = Entry::Open { stop: false };
                        self.carriers[carrier].task = CarrierTask::Carry { to, entry: open };
                        open
                    }
                    entry => entry,
                };
                let walker = &self.carriers[carrier].walker;
                if arrived {
                    match entry {
                        Entry::Open { stop: true } => {
                            // TRN-CARRY-3.2 (T51): refused on its step onto
                            // the flag, which the movement then completed: it
                            // waits on the flag like one waiting before it,
                            // with no check in the tick it came, then checks
                            // on the 10-tick grid and road swaps on the
                            // 200-tick grid; admitted, the next tick is its
                            // arrival tick (3.3a).
                            if !walked_in && (number + tick).is_multiple_of(ROOM_RETRY_PERIOD) {
                                let settled = self.entry_check(carrier, to);
                                if !settled && (number + tick).is_multiple_of(ROAD_SWAP_PERIOD) {
                                    self.road_swap(carrier, to);
                                }
                            }
                        }
                        Entry::Open { stop: false } => {
                            // No answer yet at the flag (an exchange that
                            // fell through): ask every tick (TRN-CARRY-3.3).
                            self.entry_check(carrier, to);
                        }
                        Entry::Building => {
                            let ware = self.carriers[carrier].ware.expect("a carried ware");
                            if let Some(building) = self.delivers_here(ware, to) {
                                self.carriers[carrier].task = CarrierTask::Deliver {
                                    flag: to,
                                    back: false,
                                };
                                // TRN-CARRY-3.3a (9d3c62b, T27): into a
                                // building with a door offset, sites
                                // included, on to its door field.
                                let door = self.door_field(building);
                                if door != building {
                                    // ECO-WORK-3.8b: the door step is a
                                    // walk of its own.
                                    let walker = &mut self.carriers[carrier].walker;
                                    walker.walk([building, door]);
                                    walker.pauses = vec![building];
                                } else {
                                    self.carriers[carrier].walker.walk([building]);
                                }
                            } else {
                                self.carriers[carrier].task = CarrierTask::Carry {
                                    to,
                                    entry: Entry::Open { stop: false },
                                };
                            }
                        }
                        Entry::Exchange(other) => {
                            let here = self.wares[other].as_ref().is_some_and(|w| {
                                w.location == WareLocation::Flag(to)
                                    && w.claimed_by == Some(carrier)
                            });
                            if here {
                                // TRN-CARRY-3.3a: it stands in the arrival tick.
                                self.carriers[carrier].task = CarrierTask::Stop {
                                    at: to,
                                    exchange: Some(other),
                                };
                            } else {
                                // TRN-CARRY-3.7: the exchange is cancelled.
                                self.carriers[carrier].task = CarrierTask::Carry {
                                    to,
                                    entry: Entry::Open { stop: false },
                                };
                            }
                        }
                        Entry::Reserved => {
                            // TRN-CARRY-3.3a: it stands in the arrival tick.
                            self.carriers[carrier].task = CarrierTask::Stop {
                                at: to,
                                exchange: None,
                            };
                        }
                    }
                } else if matches!(entry, Entry::Open { .. }) {
                    // Walking: check in every tick. Stopped short: check
                    // only in the ticks of the 10-tick grid, and try a road
                    // swap in those of the 200-tick grid (TRN-CARRY-3.2);
                    // not in the tick it came to a stop, even on the grid
                    // (T56).
                    let stopped = standing && walker.is_walking();
                    let on_grid = (number + tick).is_multiple_of(ROOM_RETRY_PERIOD);
                    if !stopped || (on_grid && !came_to_stop) {
                        let settled = self.entry_check(carrier, to);
                        if stopped && settled {
                            // TRN-CARRY-3.2a: admitted, its walk is ordered
                            // again in this tick (its first step advances in
                            // the next).
                            self.carriers[carrier].walker.held = false;
                        }
                        if stopped && !settled && (number + tick).is_multiple_of(ROAD_SWAP_PERIOD) {
                            self.road_swap(carrier, to);
                        }
                    }
                }
            }
            CarrierTask::Stop { at: flag, exchange } => {
                // TRN-CARRY-3.7 (e987c42): with its place reserved it looks
                // again as its put-down begins: a ware ready for the way back
                // is claimed now and taken in the same two actions.
                let bid = exchange.is_none();
                // T57 (Q-381): an exchange ware whose claim went in the
                // arrival tick counts as none: it looks again the same way.
                let exchange = exchange.filter(|&x| self.claimed_at(x, carrier, flag));
                let road = self.carriers[carrier].road;
                let exchange = exchange.or_else(|| {
                    let (other, _) = self.best_ware(road, flag)?;
                    self.wares[other].as_mut()?.claimed_by = Some(carrier);
                    Some(other)
                });
                self.carriers[carrier].task = if exchange.is_none() && !bid {
                    // None ready either: as after a cancelled exchange
                    // (3.3), its entry is asked again (not measured).
                    CarrierTask::Carry {
                        to: flag,
                        entry: Entry::Open { stop: false },
                    }
                } else {
                    // TRN-CARRY-3.3a: the drop-time action from the tick
                    // after the arrival.
                    CarrierTask::PutDown {
                        at: flag,
                        ends: tick + Tick::from(self.settings.drop_ticks),
                        second: false,
                        exchange,
                        bid,
                    }
                };
            }
            CarrierTask::PutDown {
                at: flag,
                ends,
                second,
                exchange,
                bid,
            } => {
                if tick < ends {
                    return;
                }
                if !second {
                    self.put_ware_down(carrier, flag, exchange, bid);
                    self.carriers[carrier].task = CarrierTask::PutDown {
                        at: flag,
                        ends: tick + Tick::from(self.settings.pickup_ticks),
                        second: true,
                        exchange,
                        bid,
                    };
                    return;
                }
                // TRN-CARRY-3.4 (T50, Q-374): the put-down counts on the road
                // now that both its actions are over, after the roads'
                // update of this tick (in the next period if this tick
                // ended one, TRN-CARRY-6.1).
                let road = self.carriers[carrier].road;
                if let Some(r) = self.roads[road].as_mut() {
                    r.traffic.transport_count += 1;
                }
                // TRN-CARRY-3.5: an exchange ware goes back at once; else
                // take a ware for the way back, or go to the idle field.
                // ECO-REQ-5.3e (T24): an exchange ware redirected during the
                // put-down was not taken: he goes on as without one.
                if self.carriers[carrier].ware.is_some() {
                    // TRN-CARRY-3.2 (T36): the first check in the next tick.
                    self.start_carry(carrier, flag, other_end(flag));
                    return;
                }
                // TRN-CARRY-3.3a: without one it stands once more, then
                // acts (`CarrierTask::Out`).
                self.carriers[carrier].task = CarrierTask::Out { flag };
            }
        }
    }

    /// TRN-CARRY-3.2: the entry check of the carrier for the flag `to`, in
    /// its order: the building, a ware to exchange, a full flag, the bid.
    /// Records the outcome in the carrier's task; true when settled.
    fn entry_check(&mut self, carrier: usize, to: Point) -> bool {
        let ware = self.carriers[carrier].ware.expect("a carried ware");
        let road = self.carriers[carrier].road;
        // 1. into the building at the flag.
        let entry = if self.delivers_here(ware, to).is_some() {
            Some(Entry::Building)
        } else if let Some((other, _)) = self.best_ware(road, to) {
            // 2. a ware ready for the way back: claimed, entry regardless of room.
            self.wares[other].as_mut().expect("live ware").claimed_by = Some(carrier);
            Some(Entry::Exchange(other))
        } else {
            // 3., 4.
            let w = self.wares[ware].as_ref().expect("live ware");
            let (good, mut score) = (w.good, w.wait_score);
            // TRN-CARRY-5.4 (T39, T43): the flag after `to`, from the ware's
            // memory, asked only by the last-place test (the ware would take
            // `to`'s last place); at a full flag or with more places free it
            // asks and stores nothing.
            let limit = self.settings.flag_package_limit as usize;
            let next = if self.flag_load(to) + 1 == limit {
                self.next_flag_after(ware, to)
            } else {
                None
            };
            let admitted = self.flag_bid_next(to, good, next, &mut score);
            self.wares[ware].as_mut().expect("live ware").wait_score = score;
            // TRN-CARRY-2.4a: admitted, it joins the flag's list now.
            if admitted {
                let key = self.take_join();
                self.wares[ware].as_mut().expect("live ware").joined = key;
            }
            admitted.then_some(Entry::Reserved)
        };
        let Some(entry) = entry else {
            self.carriers[carrier].task = CarrierTask::Carry {
                to,
                entry: Entry::Open { stop: true },
            };
            return false;
        };
        self.carriers[carrier].task = CarrierTask::Carry { to, entry };
        true
    }

    /// TRN-CARRY-3.6: a carrier waiting before the full flag `full` swaps
    /// roads with the carrier of the road its ware takes next, if that one
    /// is (a) idle on its idle field or (b) waiting at `full` with a ware
    /// whose next flag after `full` is the far end of this carrier's road
    /// (Q-121): the other carries towards `full` and each needs the other's
    /// road out of it. Wares are not exchanged; each goes on over its new
    /// road past the full flag.
    fn road_swap(&mut self, carrier: usize, full: Point) {
        let c = &self.carriers[carrier];
        let (r1, donkey) = (c.road, c.donkey);
        let ware = c.ware.expect("a carried ware");
        let road = self.roads[r1].as_ref().expect("live road");
        let player = road.owner;
        let r1_path = road.path.clone();
        let water = road.water;
        let back_end = if r1_path[0] == full {
            *r1_path.last().expect("road ends")
        } else {
            r1_path[0]
        };
        // TRN-CARRY-5.4 (T39): the flag after `full`, from the ware's
        // memory (none without a destination).
        let Some(onward) = self.next_flag_after(ware, full) else {
            return;
        };
        // R2: the lowest-numbered road between `full` and `onward`, of the
        // same kind (land or waterway).
        let Some(r2) = (0..self.roads.len()).find(|&id| {
            id != r1
                && self.roads[id].as_ref().is_some_and(|r| {
                    let ends = (r.path[0], *r.path.last().expect("road ends"));
                    r.water == water && (ends == (full, onward) || ends == (onward, full))
                })
        }) else {
            return;
        };
        let r2_road = self.roads[r2].as_ref().expect("live road");
        let r2_path = r2_road.path.clone();
        let place = if donkey {
            r2_road.donkey
        } else {
            r2_road.carrier
        };
        let Some(other) = place else { return };
        let o = &self.carriers[other];
        let o_standing = o.walker.next.is_none();
        // (a)
        let idle = o.ware.is_none()
            && o.task == CarrierTask::Idle
            && o_standing
            && !o.walker.is_walking()
            && o.walker.at == idle_field(&r2_path)
            && self.flag_at(o.walker.at).is_none();
        // (b) (TRN-CARRY-3.6a, Q-1012): the other holds a ware with a
        // destination and waits at a flag G for room (its own entry check
        // said no; one whose entry is settled only stands between two
        // steps); its ware's next flag NF after G (searched now) and G must
        // be the two ends of this carrier's road. Nothing else is compared.
        let r1_ends = (r1_path[0], *r1_path.last().expect("road ends"));
        let blocked_leg = match (o.task, o.ware) {
            (
                CarrierTask::Carry {
                    to: goal,
                    entry: Entry::Open { stop: true },
                },
                Some(theirs),
            ) if o_standing && o.walker.is_walking() => self.wares[theirs]
                .as_ref()
                .and_then(|w| w.destination)
                .and_then(|d| self.next_flag(player, goal, d))
                .filter(|&nf| (goal, nf) == r1_ends || (nf, goal) == r1_ends)
                .map(|nf| (goal, nf)),
            _ => None,
        };
        let blocked = blocked_leg.is_some();
        if !idle && !blocked {
            return;
        }
        // TRN-CARRY-3.6a: nothing above asks whether `full` is on this carrier's
        // own road. A carrier carrying along a neighbouring road (3.5a) swaps at
        // a flag off it too; see the walks below.
        let on_own_road = r1_path.contains(&full);
        // Exchange the roads.
        let set = |road: &mut Road, who: usize| {
            if donkey {
                road.donkey = Some(who);
            } else {
                road.carrier = Some(who);
            }
        };
        if donkey {
            set(self.roads[r1].as_mut().expect("live road"), other);
            set(self.roads[r2].as_mut().expect("live road"), carrier);
        } else {
            // SIM-START-4.3 (d) (T15): R1, then R2 lose their carriers (a
            // road without a donkey has the ware network of both ends
            // renumbered, two numbers); then R2 gets the waiting carrier and
            // R1 the other, each with a number when its ends' ware ids
            // differ then (4.3 b).
            for r in [r1, r2] {
                let road = self.roads[r].as_mut().expect("live road");
                road.carrier = None;
                if road.donkey.is_none() {
                    self.new_number();
                    self.new_number();
                    // TRN-CARRY-5.3a: the ware renumberings empty the store.
                    self.route_store.empty_ware(player);
                }
            }
            self.renumber_carried(r2);
            set(self.roads[r2].as_mut().expect("live road"), carrier);
            self.renumber_carried(r1);
            set(self.roads[r1].as_mut().expect("live road"), other);
        }
        self.carriers[carrier].road = r2;
        self.carriers[other].road = r1;
        // This one goes on with its ware past `full` along R2.
        // TRN-CARRY-3.6a: off its old road it starts its carry walk again from
        // where it stands, R2 now its own road (3.5a).
        let mut walk = vec![full];
        walk.extend(along(&r2_path, full, onward));
        if !on_own_road {
            walk = self
                .carry_walk_again(carrier, player, onward)
                .unwrap_or(walk);
        }
        self.carriers[carrier].walker.walk(walk);
        self.carriers[carrier].task = CarrierTask::Carry {
            to: onward,
            entry: Entry::Open { stop: false },
        };
        // TRN-CARRY-4.4: a swap resets the flag's bar.
        self.flag_bar.remove(&full);
        let from = self.carriers[other].walker.at;
        if !on_own_road {
            // TRN-CARRY-3.6a: the other only has its road changed. (a) In its own
            // next update (timing as in 3.6) it finds itself off its new road and
            // starts the road walk to its idle field (TRN-FREE-6.1, the Idle step
            // of `update_carrier`). (b) It starts its carry walk again too, to its
            // ware's next flag after `full`.
            // Off this carrier's road, G is R2's far end and R1 begins
            // there: it carries on from G along R1 to NF.
            if let Some((_, nf)) = blocked_leg {
                let walk = self.carry_walk_again(other, player, nf);
                self.carriers[other].walker.walk(walk.unwrap_or_default());
                self.carriers[other].task = CarrierTask::Carry {
                    to: nf,
                    entry: Entry::Open { stop: false },
                };
            }
        } else if idle {
            // (a) to its new road's idle field, by way of `full`: one walk,
            // the step's excess carried over `full` (T44). Ordered now at
            // progress 0, it takes it up in its own next update (T54): this
            // tick's when it comes after this carrier in the list, else the
            // next tick's, so it advances a tick later.
            let idle1 = idle_field(&r1_path);
            let mut walk = along(&r2_path, from, full);
            walk.extend(along(&r1_path, full, idle1));
            let o = &mut self.carriers[other];
            o.walker.walk(walk);
            o.walker.road_to = Some(idle1);
            o.task = CarrierTask::Arrive {
                flag: idle1,
                stand: false,
            };
            o.fresh = true;
        } else if let Some((goal, nf)) = blocked_leg {
            // (b) on with its own ware from G along R1 to NF: past `full`
            // in the normal case (G = `full`, NF the far end).
            let walk = if goal == full {
                let mut walk = vec![full];
                walk.extend(along(&r1_path, full, back_end));
                Some(walk)
            } else {
                self.carry_walk_again(other, player, nf)
            };
            self.carriers[other].walker.walk(walk.unwrap_or_default());
            self.carriers[other].task = CarrierTask::Carry {
                to: nf,
                entry: Entry::Open { stop: false },
            };
        }
    }

    /// TRN-CARRY-3.5a, 3.6a: a carry walk to the flag `to` started again from
    /// where the carrier stands, with the road it now holds at cost 1
    /// (TRN-FREE-6.1): from a flag, or from a road's field by T43.
    pub(crate) fn carry_walk_again(
        &self,
        carrier: usize,
        player: PlayerId,
        to: Point,
    ) -> Option<Vec<Point>> {
        let c = &self.carriers[carrier];
        let search = Search::OwnWalk(c.road);
        let at = c.walker.at;
        if self.flag_at(at).is_some() {
            return self.road_walk_by(player, search, at, to);
        }
        let path = &self.road_at(at)?.path;
        self.road_walk_from_road_field_by(player, search, at, path, to)
    }

    /// The field of the building at `flag` that takes `ware`, if the ware
    /// is for it and the building still stands (TRN-CARRY-3.3).
    fn delivers_here(&self, ware: usize, flag: Point) -> Option<Point> {
        let w = self.wares[ware].as_ref().expect("live ware");
        let target = w.target?;
        // TRN-CARRY-3.3 (T59): and the building takes the good (a warehouse
        // without stop or out on it, ECO-DIST-1.1).
        (w.destination == Some(flag)
            && self.target_flag(target) == Some(flag)
            && self.warehouse_accepts(target.at, w.good))
        .then_some(target.at)
    }

    /// TRN-CARRY-3.3, 3.7: the ware lies at the flag, its reserved place
    /// used; with `exchange` the carrier takes that ware off the flag in
    /// the same moment. A debug ware put down at its destination flag (no
    /// building) is delivered and leaves the world (Q-103). One put down at
    /// its building's flag keeps its destination (T59; the storage run or
    /// the matching takes it from there, ECO-REQ-5.7). It counts on the
    /// road when the put-down is over (TRN-CARRY-3.4, T50).
    fn put_ware_down(&mut self, carrier: usize, flag: Point, exchange: Option<usize>, bid: bool) {
        let ware = self.carriers[carrier].ware.take().expect("a carried ware");
        // ECO-REQ-5.3e (T24): the pick-up half takes the exchange ware only
        // while it is still claimed by him and lies at the flag.
        let take = exchange.filter(|&other| self.claimed_at(other, carrier, flag));
        // TRN-CARRY-4.3 (T53): a reserved place is used now, the exchange
        // claimed at the put-down's start too.
        if exchange.is_none() || bid {
            self.unreserve(flag);
        }
        match exchange {
            None => {}
            Some(_) if take.is_none() => {}
            Some(other) => {
                if let Some(list) = self.flag_wares.get_mut(&flag) {
                    list.retain(|&w| w != other);
                }
                self.ware_left(flag);
                let x = self.wares[other].as_mut().expect("live ware");
                x.location = WareLocation::Carried(carrier);
                x.wait_score = 0;
            }
        }
        // TRN-CARRY-3.7 (T31): in an exchange the carrier holds the other
        // ware only after the put-down ware's next flag is computed, so in
        // that search his road counts him as free (L, not 3L).
        let w = self.wares[ware].as_mut().expect("live ware");
        w.wait_score = 0;
        // T59: put down at the flag of the building it is bound for (that
        // building no longer taking it), it keeps that destination, with
        // no next flag (ECO-REQ-5.7 a, c).
        if w.destination == Some(flag) && w.target.is_none() {
            // A ware sent to a bare flag (`DropWare`) ends there (W19: laid
            // on a pavement that starts there).
            let good = w.good;
            self.wares[ware] = None;
            self.paving_ware_arrived(flag, good);
            self.used(good, 1);
            self.carriers[carrier].ware = take;
            return;
        }
        w.location = WareLocation::Flag(flag);
        w.claimed_by = None;
        // TRN-CARRY-2.4a: an exchange ware entered without a bid and joins
        // the list now; an admitted one joined when its place was granted.
        if exchange.is_some() && !bid {
            let key = self.take_join();
            self.wares[ware].as_mut().expect("live ware").joined = key;
        }
        // ECO-REQ-5.4a: put down without a destination, it is appended to
        // its free-ware list unless it is on it.
        if self.wares[ware]
            .as_ref()
            .is_some_and(|w| w.destination.is_none())
        {
            self.join_free_list(ware);
        }
        self.lay_at_flag(flag, ware);
        // TRN-CARRY-5.4, TRN-ROAD-6.6: arriving at the flag, its next flag
        // is computed over the ware networks.
        self.route_ware(ware);
        self.carriers[carrier].ware = take;
    }

    /// The ware `ware` lies at `flag`, claimed by `carrier`.
    fn claimed_at(&self, ware: usize, carrier: usize, flag: Point) -> bool {
        self.wares[ware].as_ref().is_some_and(|w| {
            w.claimed_by == Some(carrier) && w.location == WareLocation::Flag(flag)
        })
    }

    /// ECO-REQ-5.4a: the ware is appended to its player's free-ware list of
    /// its good, unless it is on it.
    pub(crate) fn join_free_list(&mut self, ware: usize) {
        let place = self.next_free;
        if let Some(w) = self.wares[ware].as_mut().filter(|w| w.free_place.is_none()) {
            w.free_place = Some(place);
            self.next_free += 1;
        }
    }

    /// TRN-CARRY-2.4a: the key of a ware joining a flag's list now.
    pub(crate) fn take_join(&mut self) -> u64 {
        let key = self.next_join;
        self.next_join += 1;
        key
    }

    /// TRN-CARRY-2.4a: the place of the ware in the hand of the settler
    /// `number` is granted: it joins the flag's list now.
    pub(crate) fn hold_join(&mut self, number: u32) {
        let key = self.take_join();
        self.held_joins.insert(number, key);
    }

    /// The settler `number` no longer holds a ware with a granted place.
    pub(crate) fn drop_held_join(&mut self, number: u32) -> Option<u64> {
        self.held_joins.remove(&number)
    }

    /// TRN-CARRY-2.4a: the ware lies at `flag`, in the flag's list after
    /// every ware that joined before it.
    fn lay_at_flag(&mut self, flag: Point, ware: usize) {
        let key = self.wares[ware].as_ref().expect("live ware").joined;
        let wares = &self.wares;
        let list = self.flag_wares.entry(flag).or_default();
        let at = list.partition_point(|&w| wares[w].as_ref().is_some_and(|w| w.joined <= key));
        list.insert(at, ware);
    }

    /// A new ware of `good` lying at `flag`, going to `destination` for
    /// `target`, joining the flag's list now. The caller has checked the
    /// room.
    pub(crate) fn new_ware_at(
        &mut self,
        good: u32,
        flag: Point,
        destination: Option<Point>,
        target: Option<Target>,
    ) -> usize {
        self.new_ware_joined(good, flag, destination, target, None)
    }

    /// [`World::new_ware_at`] for a ware whose place was granted earlier
    /// with the key `joined` (TRN-CARRY-2.4a); none joins now.
    pub(crate) fn new_ware_joined(
        &mut self,
        good: u32,
        flag: Point,
        destination: Option<Point>,
        target: Option<Target>,
        joined: Option<u64>,
    ) -> usize {
        let joined = joined.unwrap_or_else(|| self.take_join());
        let id = self.wares.len();
        self.wares.push(Some(Ware {
            good,
            destination,
            target,
            location: WareLocation::Flag(flag),
            claimed_by: None,
            recheck: 0,
            wait_score: 0,
            stranded: false,
            next: None,
            joined,
            order: None,
            next_after: None,
            free_place: None,
        }));
        // ECO-REQ-5.4a: a ware made at a flag without a destination is
        // appended to its free-ware list.
        if destination.is_none() {
            self.join_free_list(id);
        }
        self.lay_at_flag(flag, id);
        // TRN-CARRY-5.4: its route is computed as it comes to the flag.
        self.route_ware(id);
        id
    }

    /// TRN-CARRY-5.4, 5.5: the next flag of a ware lying at a flag is
    /// computed (again) over the ware networks; without a way (5.5,
    /// ECO-DIST-1.6) it has none and is stranded. At its destination it
    /// needs none.
    pub(crate) fn route_ware(&mut self, ware: usize) {
        let (next, way) = self.ware_route(ware);
        if let Some(w) = self.wares[ware].as_mut() {
            w.next = next;
            w.stranded = !way;
            // TRN-CARRY-5.4 (T39): the fresh search clears the memory.
            w.next_after = None;
            // ECO-DIST-1.6 (9d69dd5): every computation of the next flag
            // starts the re-check count at 0.
            w.recheck = 0;
        }
    }

    /// ECO-DIST-1.6: whether the next flag a ware lying at a flag keeps is
    /// still valid: it exists, is in the ware network of the destination's
    /// flag (TRN-CARRY-5.1a; T22: not of the ware's own flag), a road joins
    /// the ware's flag and the next flag, and the destination still accepts
    /// the good (ECO-DIST-1.2).
    pub(crate) fn next_still_valid(&self, ware: usize) -> bool {
        let Some(w) = self.wares[ware].as_ref() else {
            return false;
        };
        let (WareLocation::Flag(at), Some(next)) = (w.location, w.next) else {
            return false;
        };
        let Some(player) = self.flag_at(at) else {
            return false;
        };
        let joined = self.roads.iter().flatten().any(|r| {
            let ends = (r.path[0], r.path[r.path.len() - 1]);
            ends == (at, next) || ends == (next, at)
        });
        let destination = w.destination.unwrap_or(next);
        self.flag_at(next).is_some()
            && joined
            && self.route_cost(player, next, destination).is_some()
            && self.ware_accepted(w)
    }

    /// ECO-DIST-1.2: the ware's destination building still accepts its good
    /// (a building without flag records accepts all).
    fn ware_accepted(&self, w: &Ware) -> bool {
        w.target.is_none_or(|t| {
            !self.warehouses.contains_key(&t.at) || self.warehouse_accepts(t.at, w.good)
        })
    }

    /// TRN-CARRY-5.4, 5.5, ECO-DIST-1.6: the next flag of a ware lying at a
    /// flag toward its destination, and whether it has a way there: the
    /// destination is in the ware network of its flag (TRN-CARRY-5.1a) and
    /// still accepts the good (ECO-DIST-1.2). A ware without a destination,
    /// or lying at it, needs none.
    fn ware_route(&self, ware: usize) -> (Option<Point>, bool) {
        let Some(w) = self.wares[ware].as_ref() else {
            return (None, true);
        };
        let (WareLocation::Flag(at), Some(destination)) = (w.location, w.destination) else {
            return (None, true);
        };
        // T59, ECO-REQ T41: one lying at the flag of the building it is
        // bound for (put down there, refused) has no next flag and no way.
        if at == destination {
            return (None, w.target.is_none());
        }
        if !self.ware_accepted(w) {
            return (None, false);
        }
        let next = self
            .flag_at(at)
            .and_then(|player| self.next_flag(player, at, destination));
        (next, next.is_some())
    }

    /// Take a ware lying at a flag out of the world.
    pub(crate) fn remove_ware(&mut self, ware: usize) {
        if let Some(WareLocation::Flag(flag)) = self.wares[ware].as_ref().map(|w| w.location) {
            if let Some(list) = self.flag_wares.get_mut(&flag) {
                list.retain(|&w| w != ware);
            }
            self.ware_left(flag);
        }
        self.wares[ware] = None;
    }

    /// The `DropWare` command: put a new ware at a flag, sent to another flag.
    pub(crate) fn drop_ware(
        &mut self,
        player: PlayerId,
        good: u32,
        at: Point,
        destination: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) || !self.terrain.contains(destination) {
            return Err(Rejection::OutsideMap);
        }
        if self.flag_at(at) != Some(player) || self.flag_at(destination) != Some(player) {
            return Err(Rejection::NotAFlag);
        }
        if !self.priorities.contains_key(&good) {
            return Err(Rejection::UnknownGood);
        }
        if !self.flag_has_room_for(at, Some(destination)) {
            return Err(Rejection::FlagFull);
        }
        // SIM-START-4.2: a ware made.
        self.new_number();
        self.new_ware_at(good, at, Some(destination), None);
        self.made(Some(good), 1);
        Ok(())
    }

    /// TRN-CARRY-6: one tick of every road's traffic clock.
    pub(crate) fn update_traffic(&mut self) {
        for road in self.roads.iter_mut().flatten() {
            road.traffic.tick(road.length);
        }
    }

    /// TRN-ROAD-6.4: a ware is destroyed, wherever it is. If it was on its
    /// way to fill an order, the order is open again.
    pub(crate) fn destroy_ware(&mut self, ware: usize) {
        let Some(w) = self.wares[ware].clone() else {
            return;
        };
        self.used(w.good, 1);
        match w.location {
            WareLocation::Flag(_) => self.remove_ware(ware),
            WareLocation::Carried(carrier) => {
                if let Some(c) = self.carriers.get_mut(carrier) {
                    if c.ware == Some(ware) {
                        c.ware = None;
                    }
                }
                self.wares[ware] = None;
            }
        }
        self.reopen_order(w.order, w.target, w.good);
    }

    /// ECO-REQ-5.3h (T43): the order a destroyed ware served is open again
    /// as it was: last among the open orders, with its stamp and its place
    /// in the order the orders were made (as 5.3f), no order dropped. Not a
    /// new order (its stamp 0 would put it before every order chosen
    /// before). A ware with no order filed (none of ours) files one anew.
    pub(crate) fn reopen_order(
        &mut self,
        order: Option<crate::Order>,
        target: Option<crate::Target>,
        good: u32,
    ) {
        let Some(target) = target.filter(|t| !t.storage) else {
            return;
        };
        if self.target_building(target).is_none() {
            return;
        }
        match order.filter(|o| o.target.at == target.at) {
            // SHIP-4.1: open again, attached to no harbour.
            Some(o) => self.orders.push(crate::Order { harbour: None, ..o }),
            None => self.order(target.at, good),
        }
    }

    /// TRN-CARRY-7.1, 7.1a: the carrier (or donkey) of a removed road
    /// leaves its post: the same settler, with its number and the
    /// step it walks, goes on as a settler that notices the release in its
    /// own next update (`World::released_carrier`). Its ware and its claims
    /// stay until then (TRN-ROAD-6.3).
    pub(crate) fn retire_carrier(
        &mut self,
        carrier: usize,
        donkey: bool,
        player: PlayerId,
        road_path: &[Point],
    ) {
        // TRN-CARRY-7.1a (T45): still on its way to its road over other
        // roads, its walk ends on the next flag it reaches.
        let c = &self.carriers[carrier];
        let to_road = matches!(c.task, CarrierTask::Arrive { .. })
            && c.walker.next.is_some()
            && !road_path.contains(&c.walker.at);
        if to_road {
            let flags = &self.flags;
            let terrain = &self.terrain;
            let walker = &mut self.carriers[carrier].walker;
            walker.end_at_next(|f| flags[terrain.index(f)].is_some());
            walker.road_to = None;
        }
        let c = &mut self.carriers[carrier];
        // TRN-ROAD-6.3 (T32, Q-294): the exchange ware it was bound to stays
        // bound over the walk home.
        c.bound = match c.task {
            CarrierTask::Carry {
                entry: Entry::Exchange(other),
                ..
            } => Some(other),
            CarrierTask::Stop { exchange, .. } | CarrierTask::PutDown { exchange, .. } => exchange,
            _ => None,
        };
        c.task = CarrierTask::Retired;
        let (number, walker) = (c.number, c.walker.clone());
        let profession = if donkey {
            self.roles.donkey
        } else {
            self.roles.helper
        };
        let Some(profession) = profession else {
            // No such profession in the rules: nobody goes home.
            self.let_go(carrier);
            self.release_claims_of(number);
            return;
        };
        let walking = walker.next.is_some();
        self.add_settler(crate::Settler {
            number,
            player,
            profession,
            list: crate::SettlerList::Carriers,
            walker,
            job: crate::SettlerJob::Released {
                carrier,
                step: crate::ReleasedStep::Unnoticed { walking, to_road },
            },
            timer: 0,
            carrying: None,
        });
    }

    /// TRN-CARRY-5.3a: [`RouteStore::ware_cost`] on the world's own store.
    pub(crate) fn stored_ware_cost(
        &mut self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Option<u32> {
        let mut store = std::mem::take(&mut self.route_store);
        let cost = store.ware_cost(self, player, from, to);
        self.route_store = store;
        cost
    }

    /// TRN-ROAD-6.3: a released carrier lets go as its walk home starts:
    /// the ware in its hands is destroyed, a ware it had claimed at a flag
    /// to fetch is free again (Q-294); the exchange ware it was bound to
    /// stays claimed by it while it walks home (T32), until it is taken in
    /// at a warehouse or dies ([`World::release_claims_of`]).
    pub(crate) fn let_go(&mut self, carrier: usize) {
        if let Some(ware) = self.carriers[carrier].ware {
            self.destroy_ware(ware);
        }
        self.carriers[carrier].ware = None;
        let bound = self.carriers[carrier].bound;
        for (i, w) in self.wares.iter_mut().enumerate() {
            if let Some(w) = w {
                if w.claimed_by == Some(carrier) && bound != Some(i) {
                    w.claimed_by = None;
                }
            }
        }
    }

    /// TRN-ROAD-6.3 (T32, Q-294): the bindings of the released carrier
    /// whose settler has the object number `number` end with it (taken in,
    /// dead, or no one to walk home): those wares are free.
    pub(crate) fn release_claims_of(&mut self, number: u32) {
        let Some(carrier) = self
            .carriers
            .iter()
            .position(|c| c.number == number && c.task == CarrierTask::Retired)
        else {
            return;
        };
        for w in self.wares.iter_mut().flatten() {
            if w.claimed_by == Some(carrier) {
                w.claimed_by = None;
            }
        }
    }

    /// TRN-ROAD-4.2 for carriers: the old road's carrier (and donkey) is
    /// attached by one field: the field it stands on, or the field its step
    /// goes to. That field on the first half (the old road's first flag to
    /// the split, the split included) or the field of the building at the
    /// first flag: the first half; any other field, also off the old road
    /// on its way to it: the second. A half left without a carrier requests
    /// one (ECO-REQ-1.1). A carrier carrying a ware to an end of its half
    /// goes on unchanged; any other walk or claim that no longer fits is cut
    /// back to the new road; a carrier on its way to its road ends that walk
    /// on the next flag it reaches (TRN-FREE-6.5 g).
    pub(crate) fn split_carriers(&mut self, old: usize, new: usize, split: Point, road: &Road) {
        let old_path = &road.path;
        let places = [(road.carrier, false), (road.donkey, true)];
        let split_index = old_path
            .iter()
            .position(|&p| p == split)
            .expect("on the road");
        let first_half = &old_path[..=split_index];
        let first_building = self
            .terrain
            .neighbour(old_path[0], 4)
            .filter(|&b| self.building_at(b).is_some());
        for (place, donkey) in places {
            let Some(carrier) = place else { continue };
            let c = &self.carriers[carrier];
            let field = c.walker.next.unwrap_or(c.walker.at);
            let mine = if first_half.contains(&field) || Some(field) == first_building {
                old
            } else {
                new
            };
            self.carriers[carrier].road = mine;
            // SIM-START-4.4 j: the carrier's half joins the donkey networks
            // of its two flags (4.3 b).
            if !donkey {
                self.renumber_carried(mine);
            }
            let r = self.roads[mine].as_mut().expect("live road");
            if donkey {
                r.donkey = Some(carrier);
            } else {
                r.carrier = Some(carrier);
            }
            self.fit_task_to_road(carrier, mine, split);
        }
    }

    /// TRN-ROAD-4.2 (T28, T29): the new flag `split` lies ahead on the
    /// carrier's walk (his step goes to it, or it is still to be walked).
    fn split_ahead(&self, carrier: usize, split: Point) -> bool {
        let walker = &self.carriers[carrier].walker;
        walker.next == Some(split) || walker.path.contains(&split)
    }

    /// After a split: cut a carrier's walk or claim back to its road `road`.
    fn fit_task_to_road(&mut self, carrier: usize, road: usize, split: Point) {
        let path = self.roads[road].as_ref().expect("live road").path.clone();
        let ends = (path[0], *path.last().expect("road ends"));
        let is_end = |f: Point| f == ends.0 || f == ends.1;
        match self.carriers[carrier].task {
            // T33: on his way to the idle field with the new flag ahead, as
            // T31: his walk ends on the new flag, he stands there in the
            // arrival tick and walks to his half's idle field from the next.
            CarrierTask::Idle if self.split_ahead(carrier, split) => {
                self.carriers[carrier].walker.end_at_next(|f| f == split);
                self.carriers[carrier].task = CarrierTask::Arrive {
                    flag: idle_field(&path),
                    stand: true,
                };
            }
            CarrierTask::Idle => {
                let walker = &mut self.carriers[carrier].walker;
                let goal = walker.is_walking().then(|| walker.destination());
                if walker.next.is_some() && !path.contains(&walker.at) {
                    // Its step goes onto its half: it finishes that step.
                    walker.path.clear();
                    walker.pauses.clear();
                    walker.road_to = None;
                    walker.held = false;
                } else if let Some(goal) = goal.filter(|g| path.contains(g)) {
                    // T34: the new flag beyond the walk's goal: the walk is
                    // kept to the old idle field, where he stands in the
                    // arrival tick; his walk to the half's idle field is
                    // ordered in the next. Is that his half's idle field
                    // too, the walk just ends there.
                    if goal != idle_field(&path) {
                        self.carriers[carrier].task = CarrierTask::Arrive {
                            flag: goal,
                            stand: true,
                        };
                    }
                } else {
                    walker.walk([]);
                }
            }
            // TRN-FREE-6.5 g (T25): on its way, the walk ends on the next
            // flag it reaches, without a new search there; it stands there
            // in the arrival tick and its walk on to the new road's idle
            // field is ordered in the next (as out of its warehouse,
            // TRN-CARRY-1.4).
            CarrierTask::Arrive { stand, .. } => {
                let walking = self.carriers[carrier].walker.is_walking();
                if walking {
                    let flags = &self.flags;
                    let terrain = &self.terrain;
                    let walker = &mut self.carriers[carrier].walker;
                    walker.end_at_next(|f| flags[terrain.index(f)].is_some());
                }
                self.carriers[carrier].task = CarrierTask::Arrive {
                    flag: idle_field(&path),
                    stand: stand || walking,
                };
            }
            CarrierTask::Fetch { flag, .. } if is_end(flag) => {
                // T27: with the new flag ahead on his walk, the walk ends
                // there (TRN-FREE-6.5 g); on from there in the Fetch step.
                // T30: with it behind, his walk to the ware is unchanged.
                if self.split_ahead(carrier, split) {
                    self.carriers[carrier].walker.end_at_next(|f| f == split);
                }
            }
            // T31: the ware on the other half and the new flag ahead: his
            // walk ends on the new flag and he keeps his claim until he
            // arrives; then he stands a tick and walks to his half's idle
            // field (the Arrive step releases the claim).
            CarrierTask::Fetch { .. } if self.split_ahead(carrier, split) => {
                self.carriers[carrier].walker.end_at_next(|f| f == split);
                self.carriers[carrier].task = CarrierTask::Arrive {
                    flag: idle_field(&path),
                    stand: true,
                };
            }
            CarrierTask::Fetch { ware, .. } => {
                // ECO-REQ-5.3e: its claim may be gone already.
                if let Some(w) = self.wares[ware].as_mut() {
                    if w.claimed_by == Some(carrier) {
                        w.claimed_by = None;
                    }
                }
                self.carriers[carrier].task = CarrierTask::Idle;
                self.carriers[carrier].walker.walk([]);
            }
            // TRN-ROAD-4.2 (82d505f, T25): carrying to an end of its half
            // with the new flag behind, nothing changes: it finishes its
            // step and walks on along the same fields, its entry check
            // going on where it was.
            CarrierTask::Carry { to, .. } if is_end(to) && !self.split_ahead(carrier, split) => {}
            CarrierTask::Carry { entry, .. } => {
                if let Entry::Exchange(other) = entry {
                    if let Some(w) = self.wares[other].as_mut() {
                        w.claimed_by = None;
                    }
                }
                // T28, T29: with the new flag ahead, whichever half its flag
                // lies on, the walk is cut there, the step kept; he puts the
                // ware down there, its next flag unchanged. In his step onto
                // it he arrives with a place, without an entry check (T29).
                let ahead = self.split_ahead(carrier, split);
                let entry = if self.carriers[carrier].walker.next == Some(split) {
                    *self.flag_reserved.entry(split).or_default() += 1;
                    let key = self.take_join();
                    if let Some(ware) = self.carriers[carrier].ware {
                        self.wares[ware].as_mut().expect("live ware").joined = key;
                    }
                    Entry::Reserved
                } else {
                    Entry::Open { stop: false }
                };
                self.carriers[carrier].task = CarrierTask::Carry { to: split, entry };
                if ahead {
                    self.carriers[carrier].walker.end_at_next(|f| f == split);
                } else {
                    self.route_carrier(carrier, split);
                }
            }
            CarrierTask::PickUp { .. }
            | CarrierTask::PutDown { .. }
            | CarrierTask::Deliver { .. }
            | CarrierTask::Inside { .. }
            | CarrierTask::Out { .. }
            | CarrierTask::Stop { .. }
            | CarrierTask::Retired => {}
        }
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
crate::snap_struct!(RouteStore { ware, renumberings });

#[cfg(test)]
mod tests;
