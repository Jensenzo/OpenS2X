//! Read-only views of the world that several parts of the AI share: the
//! player's stock, its road network, distances.

use std::collections::{BTreeMap, BTreeSet};

use s2_sim::grid::{self, Point};
use s2_sim::{PlayerId, SettlerJob, WareLocation, Work, World};

/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
pub const FLAG_DIRECTION: u8 = 1;

/// The flag field of the building on `at`.
pub fn flag_of(world: &World, at: Point) -> Option<Point> {
    world.terrain().neighbour(at, FLAG_DIRECTION)
}

/// `stock(g)` and `stock2(g)` of the specs (AI-CON-1.7): the goods in the
/// stores of the player's warehouses, and the wares of the player outside
/// buildings (lying at flags or carried). Both lag the game: `stock` is counted at
/// cycle tick 0 of a 22-tick cycle, `stock2` at cycle tick 2.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Economy {
    stock: BTreeMap<u32, u32>,
    moving: BTreeMap<u32, u32>,
}

/// AI-CON-1.7: the cycle of the stock counts in ticks.
pub const STOCK_CYCLE: u64 = 22;

impl Economy {
    /// AI-CON-1.7 (corrected, AI-MIL T18): the stores of the player's
    /// warehouses (headquarters, storehouses, harbours); not the inputs of
    /// workshops nor the materials at construction sites.
    pub fn count_stock(
        world: &World,
        player: PlayerId,
        own: &[s2_sim::Building],
    ) -> BTreeMap<u32, u32> {
        let mut stock: BTreeMap<u32, u32> = BTreeMap::new();
        for b in own {
            for &(good, count) in world.stock(b.at).unwrap_or(&[]) {
                *stock.entry(good).or_default() += count;
            }
        }
        let _ = player;
        stock
    }

    /// AI-CON-1.7 (corrected, T91, T92): every ware object of the player,
    /// whoever holds it: lying at a flag, carried by a road carrier or
    /// donkey, by a warehouse's helper between its door and its flag, a
    /// worker's product from the 4th tick of the put-down that ends his work
    /// (not while he puts it down), the plank a shipwright carries to the
    /// ship until the 4th tick of putting it down there (he then holds none).
    pub fn count_moving(world: &World, player: PlayerId) -> BTreeMap<u32, u32> {
        let mut moving: BTreeMap<u32, u32> = BTreeMap::new();
        for (_, ware) in world.wares() {
            let owner = match ware.location {
                WareLocation::Flag(at) => world.flag_at(at),
                WareLocation::Carried(carrier) => world
                    .carriers()
                    .get(carrier)
                    .and_then(|c| world.road(c.road))
                    .map(|r| r.owner),
            };
            if owner == Some(player) {
                *moving.entry(ware.good).or_default() += 1;
            }
        }
        for (_, settler) in world.settlers() {
            let Some(good) = settler.carrying else {
                continue;
            };
            if settler.player != player {
                continue;
            }
            // Not yet a ware object while the put-down that ends his work
            // runs (the drop-time action before the take).
            let putting_down = matches!(
                settler.job,
                SettlerJob::Worker {
                    work: Work::Take { second: false } | Work::TakeWare { second: false, .. },
                    ..
                }
            );
            if !putting_down {
                *moving.entry(good).or_default() += 1;
            }
        }
        moving
    }

    pub fn set_stock(&mut self, stock: BTreeMap<u32, u32>) {
        self.stock = stock;
    }

    pub fn set_moving(&mut self, moving: BTreeMap<u32, u32>) {
        self.moving = moving;
    }

    /// An economy with the given counts (for tests).
    pub fn from_counts(stock: &[(u32, u32)], moving: &[(u32, u32)]) -> Self {
        Self {
            stock: stock.iter().copied().collect(),
            moving: moving.iter().copied().collect(),
        }
    }

    /// `stock(g)`: the count of the good in the player's warehouses.
    pub fn stock(&self, good: u32) -> u32 {
        self.stock.get(&good).copied().unwrap_or(0)
    }

    /// `stock2(g)`: wares of the good on the roads.
    pub fn stock2(&self, good: u32) -> u32 {
        self.moving.get(&good).copied().unwrap_or(0)
    }

    /// `stock(g) + stock2(g)`.
    pub fn total(&self, good: u32) -> u32 {
        self.stock(good) + self.stock2(good)
    }
}

/// The player's flags grouped by the road network they belong to.
pub struct Network {
    component: BTreeMap<Point, usize>,
    /// Components that hold a warehouse's flag.
    warehouse: BTreeSet<usize>,
    /// Roads at each flag.
    roads_at: BTreeMap<Point, u32>,
    /// The flag graph: neighbours and road lengths in steps.
    adjacency: BTreeMap<Point, Vec<(Point, u32)>>,
}

impl Network {
    pub fn compute(world: &World, player: PlayerId) -> Self {
        let flags: Vec<Point> = world
            .flags()
            .filter(|&(_, owner)| owner == player)
            .map(|(f, _)| f)
            .collect();
        let index: BTreeMap<Point, usize> =
            flags.iter().enumerate().map(|(i, &f)| (f, i)).collect();
        let mut parent: Vec<usize> = (0..flags.len()).collect();
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        let mut roads_at: BTreeMap<Point, u32> = BTreeMap::new();
        let mut adjacency: BTreeMap<Point, Vec<(Point, u32)>> = BTreeMap::new();
        // TRN-FREE-6.1, AI-CON-8.1 (T62): each flag's roads in the order they
        // were attached, by object number (a split's halves last).
        let mut roads: Vec<&s2_sim::Road> = world.roads().filter(|r| r.owner == player).collect();
        roads.sort_by_key(|r| r.number);
        for road in roads {
            let (Some(&first), Some(&last)) = (road.path.first(), road.path.last()) else {
                continue;
            };
            *roads_at.entry(first).or_default() += 1;
            *roads_at.entry(last).or_default() += 1;
            let steps = road.path.len() as u32 - 1;
            adjacency.entry(first).or_default().push((last, steps));
            adjacency.entry(last).or_default().push((first, steps));
            if let (Some(&a), Some(&b)) = (index.get(&first), index.get(&last)) {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
        let mut component = BTreeMap::new();
        for (i, &f) in flags.iter().enumerate() {
            component.insert(f, find(&mut parent, i));
        }
        let mut warehouse = BTreeSet::new();
        for (at, _) in world.warehouses() {
            if world.building_at(at).is_some_and(|b| b.owner == player) {
                if let Some(c) = flag_of(world, at).and_then(|f| component.get(&f)) {
                    warehouse.insert(*c);
                }
            }
        }
        Self {
            component,
            warehouse,
            roads_at,
            adjacency,
        }
    }

    /// The shortest route between two flags over the roads, in steps, with
    /// the flags on it; `extra` adds a road that does not exist yet.
    pub fn route(
        &self,
        from: Point,
        to: Point,
        extra: Option<(Point, Point, u32)>,
    ) -> Option<(u32, Vec<Point>)> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let mut best: BTreeMap<Point, (u32, Point)> = BTreeMap::new();
        let mut queue = BinaryHeap::new();
        best.insert(from, (0, from));
        queue.push(Reverse((0u32, from)));
        while let Some(Reverse((cost, here))) = queue.pop() {
            if here == to {
                let mut flags = vec![to];
                let mut at = to;
                while at != from {
                    at = best[&at].1;
                    flags.push(at);
                }
                flags.reverse();
                return Some((cost, flags));
            }
            if best.get(&here).is_some_and(|&(c, _)| c < cost) {
                continue;
            }
            let mut next: Vec<(Point, u32)> =
                self.adjacency.get(&here).cloned().unwrap_or_default();
            if let Some((a, b, len)) = extra {
                if here == a {
                    next.push((b, len));
                } else if here == b {
                    next.push((a, len));
                }
            }
            for (flag, len) in next {
                let total = cost + len;
                if best.get(&flag).is_none_or(|&(c, _)| total < c) {
                    best.insert(flag, (total, here));
                    queue.push(Reverse((total, flag)));
                }
            }
        }
        None
    }

    /// The flags in the network, in field order.
    pub fn flags(&self) -> impl Iterator<Item = Point> + '_ {
        self.component.keys().copied()
    }

    /// The flag belongs to a road network that reaches a warehouse.
    pub fn on_warehouse_network(&self, flag: Point) -> bool {
        self.component
            .get(&flag)
            .is_some_and(|c| self.warehouse.contains(c))
    }

    /// Both flags are in one network.
    pub fn connected(&self, a: Point, b: Point) -> bool {
        match (self.component.get(&a), self.component.get(&b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// Number of roads that end at the flag.
    pub fn roads_at(&self, flag: Point) -> u32 {
        self.roads_at.get(&flag).copied().unwrap_or(0)
    }
}

/// Hex distance, in the type the specs use for "fields".
pub fn dist(a: Point, b: Point) -> u32 {
    grid::distance(a, b)
}

/// Fields of the area of `radius` around `at` inside the map.
pub fn area(world: &World, at: Point, radius: u32) -> Vec<Point> {
    let t = world.terrain();
    grid::area(at, radius, t.width, t.height)
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_struct!(Economy { stock, moving });

/// AI-2.1a: the world as the computer players read it while they think.
/// Each command takes effect when its part issues it in the original, so a
/// later part of the same think, and the players who think after it in the
/// tick, already read the changed state. The computer players never change
/// the world: this view lays the tick's commands over it, each applied at
/// once by the simulation's own rules to a private copy made at the tick's
/// first command (`World::apply_at_once`), and the world itself takes them
/// after the think ([`World::step_then`]).
pub struct ThinkView<'w> {
    base: &'w World,
    /// The world with the commands issued so far applied, from the first.
    layer: Option<World>,
    /// The buildings of every player as the view has them.
    snapshot: crate::snapshot::Snapshot,
    /// How many of the tick's commands the layer holds.
    applied: usize,
}

impl<'w> ThinkView<'w> {
    /// The world as the update left it, before any command of the tick.
    pub fn new(base: &'w World) -> Self {
        Self {
            base,
            layer: None,
            snapshot: crate::snapshot::Snapshot::take(base),
            applied: 0,
        }
    }

    /// The world with the commands issued so far in effect.
    pub fn world(&self) -> &World {
        self.layer.as_ref().unwrap_or(self.base)
    }

    /// Every player's buildings as [`ThinkView::world`] has them.
    pub fn snapshot(&self) -> &crate::snapshot::Snapshot {
        &self.snapshot
    }

    /// Lay the commands of `issued` (the tick's commands, in the order they
    /// were issued) that the view does not hold yet over it. AI-2.1b
    /// (Q-289): every command is in effect at once: sites with their flags
    /// and records, roads with their end flags, upgrades, tear-downs, flag
    /// removals and the settings. (A site is placed by the construction
    /// system's last step, after its street parts, AI-2.4a.)
    pub fn issue(&mut self, issued: &[s2_sim::Command]) {
        let Some(new) = issued.get(self.applied..).filter(|n| !n.is_empty()) else {
            return;
        };
        self.applied = issued.len();
        let base = self.base;
        let layer = self.layer.get_or_insert_with(|| base.clone());
        for command in new {
            // A refused command changes nothing, here as in the world.
            let _ = layer.apply_at_once(command);
        }
        self.snapshot = crate::snapshot::Snapshot::take(layer);
    }
}
