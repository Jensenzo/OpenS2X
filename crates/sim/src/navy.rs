//! Ships, harbours and expeditions (`specs/SHIP-ships-expeditions.md`).
//!
//! A finished ship is a [`Vessel`] with a state machine (SHIP-2); a finished
//! harbour is a [`Harbour`] with docking places (SHIP-1), transfer orders
//! (SHIP-4) and an expedition (SHIP-5). Both are updated in
//! [`World::update_navy`], ships first, then harbours (the order between the
//! two is not traced, Q-154).

use std::collections::VecDeque;

use crate::grid::{self, Point};
use crate::map::{state, Terrain};
use crate::movement::{self, Walker};
use crate::{PlayerId, Rejection, Requester, SettlerJob, World};

/// SHIP-2.4, SHIP-4, SHIP-5.2: counters run 0 to 100, one event every 101
/// updates.
const PERIOD_LAST: u32 = 100;
/// UI-GAME-4.5 c: the default ship names per tribe, keys 0 to 20.
const SHIP_NAMES: usize = 21;
/// SHIP-3.4: radius explored by a moving ship, and on arriving at an
/// expedition target.
const SAILING_EXPLORE: u32 = 3;
const ARRIVAL_EXPLORE: u32 = 6;
/// SHIP-7.1: the ship must be closer than this to the harbour spot.
const BUILD_DISTANCE: u32 = 5;
/// SHIP-6.1: the rings searched around a spot for its landing field.
const LANDING_RINGS: u32 = 3;
/// SHIP-6.1: spots this close to the ship's own harbour are skipped.
const OWN_HARBOUR_DISTANCE: u32 = 3;
/// SHIP-1.2: the ring of docking places.
const DOCK_RING: u32 = 3;
const WAITING_FOR_ORDERS: &str = "Ship waiting for Orders";

/// SHIP-2.2: what a ship is doing (the binary's names in quotes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipState {
    Idle,
    /// "search"
    Search,
    /// "to harbour"
    ToHarbour,
    /// "docked"
    Docked,
    /// "waiting" for an expedition decision
    Waiting,
    /// "sailing" to an expedition position
    Sailing,
    /// "unloading" (transfer of goods)
    Unloading,
    /// "making room" (move to a free position)
    MakingRoom,
    /// "returning" after a cancelled expedition
    Returning,
}

/// SHIP-4.4: goods aboard for a destination harbour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cargo {
    pub destination: Point,
    /// (good, count); soldiers and other settlers are goods of their
    /// profession.
    pub goods: Vec<(u32, u32)>,
}

/// SHIP-6.1: a landing option: the harbour spot and the field of the ship's
/// sea the ship sails to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landing {
    pub spot: Point,
    pub field: Point,
}

/// The state of a finished ship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vessel {
    /// SHIP-2.1: the state machine has started on a water field.
    pub launched: bool,
    pub state: ShipState,
    /// SHIP-2.4: the search counter, 0 to 100.
    pub counter: u32,
    pub walker: Walker,
    /// The harbour it is linked to (reserved or docked there), by field.
    pub harbour: Option<Point>,
    /// The harbour that has called it (SHIP-4.4, 5.3).
    pub called: Option<Point>,
    /// SHIP-2.5, 6.4: the last port position.
    pub port: Option<Point>,
    pub cargo: Vec<Cargo>,
    /// SHIP-6: expedition mode, and what is aboard for it (the harbour's
    /// materials and the builder).
    pub expedition: bool,
    pub aboard: Vec<(u32, u32)>,
    pub target: Option<Landing>,
    /// SHIP-6.1: one option per compass direction.
    pub options: [Option<Landing>; 6],
}

impl Vessel {
    pub fn new(at: Point) -> Self {
        Self {
            launched: false,
            state: ShipState::Idle,
            counter: 0,
            walker: Walker::new(at),
            harbour: None,
            called: None,
            port: None,
            cargo: Vec::new(),
            expedition: false,
            aboard: Vec::new(),
            target: None,
            options: [None; 6],
        }
    }
}

/// SHIP-1.2, 1.3: a docking place and the ships (by index) that have it
/// reserved or are docked at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dock {
    pub field: Point,
    pub reserved: Option<usize>,
    pub docked: Option<usize>,
}

/// SHIP-4.1: a good the harbour's island needs from overseas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    pub good: u32,
    pub count: u32,
    /// The harbour that has been given the transfer order.
    pub supplier: Option<Point>,
    /// SHIP-4.2 (a): the harbours already asked.
    pub asked: Vec<Point>,
}

/// SHIP-4.2: a transfer order: `count` of `good` for the harbour at
/// `destination`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub good: u32,
    pub count: u32,
    pub destination: Point,
}

/// SHIP-5: the harbour side of an expedition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpeditionState {
    Inactive,
    /// SHIP-5.2
    Gathering,
    /// SHIP-5.3
    GettingShip,
    /// SHIP-5.4: the called ship, by index.
    WaitingForShip(usize),
}

/// A finished harbour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Harbour {
    pub at: Point,
    pub docks: Vec<Dock>,
    /// SHIP-4: the pass counter, 0 to 100.
    pub counter: u32,
    pub needs: Vec<Need>,
    pub transfers: Vec<Transfer>,
    /// SHIP-4.4: the ship called to load, by index.
    pub called: Option<usize>,
    pub expedition: ExpeditionState,
    pub expedition_counter: u32,
    /// SHIP-5.2: materials set aside, and whether the builder is.
    pub aside: Vec<(u32, u32)>,
    pub builder_aside: bool,
}

/// One row of a harbour's expedition page (UI-GAME-8.1, SHIP-5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpeditionGood {
    pub good: u32,
    pub required: u32,
    pub aside: u32,
}

/// Labels the connected regions of the map: water fields that are not
/// DoodadBlocked (the seas, SHIP-3.1) and, apart, the other fields (land
/// continents). Label 0 is blocked water. The landmass tests that the specs
/// tie to the map's stored continents read [`World::continent_of`] instead
/// (FMT-S2M-4.9).
pub(crate) fn compute_regions(terrain: &Terrain) -> Vec<u32> {
    let mut label = vec![0u32; terrain.len()];
    let kind = |f: Point| -> Option<bool> {
        let water = terrain.has(f, state::WATER);
        if water && terrain.has(f, state::DOODAD_BLOCKED) {
            None
        } else {
            Some(water)
        }
    };
    let mut next = 0;
    for start in terrain.fields() {
        if label[terrain.index(start)] != 0 {
            continue;
        }
        let Some(water) = kind(start) else { continue };
        next += 1;
        label[terrain.index(start)] = next;
        let mut queue = VecDeque::from([start]);
        while let Some(f) = queue.pop_front() {
            for n in terrain.neighbours(f).into_iter().flatten() {
                let i = terrain.index(n);
                if label[i] == 0 && kind(n) == Some(water) {
                    label[i] = next;
                    queue.push_back(n);
                }
            }
        }
    }
    label
}

/// The stand-in continent layer of a world made without a map file (tests,
/// fixtures): each region of [`compute_regions`] as its own index, blocked
/// water -1. A game from a map loads the file's layer (FMT-S2M-4.9).
pub(crate) fn continents_from_regions(regions: &[u32]) -> Vec<i32> {
    regions
        .iter()
        .map(|&r| if r == 0 { -1 } else { r as i32 })
        .collect()
}

impl World {
    /// FMT-S2M-4.8, 4.9: the map's continent layer, used as loaded; a layer
    /// of another size than the map is ignored.
    pub fn set_continents(&mut self, continents: Vec<i32>) {
        if continents.len() == self.terrain.len() {
            self.continents = continents;
        }
    }

    /// FMT-S2M-4.8, 4.9: the map's stored continent index of `field`, -1
    /// for none (mountains and other blocked ground, doodad-blocked water);
    /// `None` outside the map. Landmass tests compare indices for equality,
    /// so two -1 fields are on the same landmass.
    pub fn continent_of(&self, field: Point) -> Option<i32> {
        self.terrain
            .contains(field)
            .then(|| self.continents[self.terrain.index(field)])
    }

    /// FMT-S2M-4.9: the positions of the land continent object of `index`
    /// (its field cursors, the harbour spots); none for a water continent
    /// or an index without an object.
    pub fn land_spots(&self, index: i32) -> &[Point] {
        self.land_spots.get(&index).map_or(&[], Vec::as_slice)
    }

    /// SHIP-1.1, 6.1: the harbour spots the map marks on its land
    /// continents, in map order (row by row).
    pub fn set_harbour_spots(&mut self, mut spots: Vec<Point>) {
        spots.retain(|&s| self.terrain.contains(s));
        spots.sort_by_key(|&s| self.terrain.index(s));
        spots.dedup();
        self.harbour_spots = spots;
    }

    pub fn harbour_spots(&self) -> &[Point] {
        &self.harbour_spots
    }

    pub fn harbours(&self) -> &[Harbour] {
        &self.harbours
    }

    /// DEBUG ONLY, not a game rule: a finished ship of `owner` on the water
    /// field `at`, for scenarios and tests of other crates. Its index in
    /// [`World::ships`].
    pub fn debug_add_ship(&mut self, owner: PlayerId, at: Point) -> usize {
        self.ships.push(crate::Ship {
            owner,
            at,
            water: None,
            frame_work: 0,
            main_work: 0,
            step_amount: None,
            name: None,
            vessel: Vessel::new(at),
        });
        self.ships.len() - 1
    }

    /// DEBUG ONLY, not a game rule: ship `index` is launched where it
    /// stands (SHIP-2.1), for tests of other crates.
    pub fn debug_launch_ship(&mut self, index: usize) {
        if let Some(s) = self.ships.get_mut(index) {
            s.vessel.launched = true;
        }
    }

    pub fn harbour_at(&self, at: Point) -> Option<&Harbour> {
        self.harbours.iter().find(|h| h.at == at)
    }

    /// The region (sea or continent) a field is in; 0 for blocked water.
    pub fn region_of(&self, field: Point) -> u32 {
        self.regions[self.terrain.index(field)]
    }

    /// SHIP-4.1: the harbour's island needs `count` of `good` from
    /// overseas. How needs arise is not traced (Q-063); a game that fills
    /// them calls this.
    pub fn add_harbour_need(&mut self, harbour: Point, good: u32, count: u32) {
        let Some(h) = self.harbours.iter_mut().find(|h| h.at == harbour) else {
            return;
        };
        match h.needs.iter_mut().find(|n| n.good == good) {
            Some(n) => n.count += count,
            None => h.needs.push(Need {
                good,
                count,
                supplier: None,
                asked: Vec::new(),
            }),
        }
    }

    // ---- Harbours (SHIP-1) ----

    /// SHIP-1.2, 1.4: a harbour is finished: it is a warehouse and makes
    /// its docking places once.
    pub(crate) fn make_harbour(&mut self, at: Point) {
        if self.harbours.iter().any(|h| h.at == at) {
            return;
        }
        self.warehouses.entry(at).or_default();
        let (width, height) = (self.terrain.width, self.terrain.height);
        let mut docks = Vec::new();
        for field in grid::ring(at, DOCK_RING, width, height) {
            let usable = self.terrain.has(field, state::WATER)
                && !self
                    .terrain
                    .has(field, state::DOODAD_BLOCKED | state::LANDING_POSITION)
                && !self
                    .terrain
                    .neighbours(field)
                    .into_iter()
                    .flatten()
                    .any(|n| self.terrain.has(n, state::LANDING_POSITION));
            if usable {
                self.set_bits(field, state::LANDING_POSITION, true);
                docks.push(Dock {
                    field,
                    reserved: None,
                    docked: None,
                });
            }
        }
        self.harbours.push(Harbour {
            at,
            docks,
            counter: 0,
            needs: Vec::new(),
            transfers: Vec::new(),
            called: None,
            expedition: ExpeditionState::Inactive,
            expedition_counter: 0,
            aside: Vec::new(),
            builder_aside: false,
        });
    }

    /// The building on `at` is gone: its harbour record ends and ships
    /// linked to it lose the link (Q-066: nothing else is traced).
    pub(crate) fn harbour_removed(&mut self, at: Point) {
        let Some(index) = self.harbours.iter().position(|h| h.at == at) else {
            return;
        };
        let gone = self.harbours.remove(index);
        for ship in &mut self.ships {
            let v = &mut ship.vessel;
            if v.harbour == Some(at) {
                v.harbour = None;
            }
            if v.called == Some(at) {
                v.called = None;
            }
            v.cargo.retain(|c| c.destination != at);
        }
        for h in &mut self.harbours {
            h.transfers.retain(|t| t.destination != at);
            for need in &mut h.needs {
                if need.supplier == Some(at) {
                    need.supplier = None;
                }
            }
        }
        let _ = gone;
    }

    /// SHIP-1.3: the harbour has a docking place that is neither reserved
    /// nor occupied and lies on the same sea as `field`.
    fn reachable_by_sea(&self, harbour: Point, field: Point) -> bool {
        let sea = self.region_of(field);
        sea != 0
            && self.harbour_at(harbour).is_some_and(|h| {
                h.docks.iter().any(|d| {
                    d.reserved.is_none() && d.docked.is_none() && self.region_of(d.field) == sea
                })
            })
    }

    /// Reserve the first free docking place of `harbour` on the ship's
    /// sea (which of several is not traced, Q-154); its field.
    fn reserve_dock(&mut self, harbour: Point, ship: usize) -> Option<Point> {
        let sea = self.region_of(self.ships[ship].vessel.walker.at);
        let regions = &self.regions;
        let terrain = &self.terrain;
        let h = self.harbours.iter_mut().find(|h| h.at == harbour)?;
        let dock = h.docks.iter_mut().find(|d| {
            d.reserved.is_none() && d.docked.is_none() && regions[terrain.index(d.field)] == sea
        })?;
        dock.reserved = Some(ship);
        Some(dock.field)
    }

    /// The ship drops its harbour link and the reservation or berth it
    /// holds there.
    fn release_links(&mut self, ship: usize) {
        if let Some(at) = self.ships[ship].vessel.harbour.take() {
            if let Some(h) = self.harbours.iter_mut().find(|h| h.at == at) {
                for d in &mut h.docks {
                    if d.reserved == Some(ship) {
                        d.reserved = None;
                    }
                    if d.docked == Some(ship) {
                        d.docked = None;
                    }
                }
            }
        }
    }

    /// The harbour the ship is docked at.
    fn docked_at(&self, ship: usize) -> Option<Point> {
        let at = self.ships[ship].vessel.harbour?;
        self.harbour_at(at)?
            .docks
            .iter()
            .any(|d| d.docked == Some(ship))
            .then_some(at)
    }

    // ---- Ships (SHIP-2, 3) ----

    /// SHIP-3.1 (T13): the fields from `from` (not included) to `to`: the
    /// ship's route search, its read-out drawing from the generator; none
    /// to another sea.
    fn sea_route(&mut self, from: Point, to: Point) -> Option<Vec<Point>> {
        if from == to {
            return Some(Vec::new());
        }
        let sea = self.region_of(from);
        if sea == 0 || self.region_of(to) != sea {
            return None;
        }
        movement::ship_walk(&self.terrain, &mut self.rng, from, to).map(|r| r.fields)
    }

    /// Start the ship sailing to `to`; false if there is no way.
    /// SHIP-3.1: the ship starts a route to `to`. SHIP-2.7: every route
    /// start clears its last port position (one to its own field too).
    fn sail(&mut self, ship: usize, to: Point) -> bool {
        self.ships[ship].vessel.port = None;
        let from = self.ships[ship].vessel.walker.at;
        match self.sea_route(from, to) {
            Some(path) => {
                self.ships[ship].vessel.walker.walk(path);
                true
            }
            None => false,
        }
    }

    /// SHIP-2.1: a finished ship starts at a water field next to where it
    /// was built (the field it was built on is a Coast field of the land,
    /// Q-130, Q-154).
    fn launch_field(&self, at: Point) -> Point {
        let (width, height) = (self.terrain.width, self.terrain.height);
        (0..=LANDING_RINGS)
            .flat_map(|k| grid::ring(at, k, width, height))
            .find(|&f| self.region_of(f) != 0 && self.terrain.has(f, state::WATER))
            .unwrap_or(at)
    }

    /// SHIP-4.4: a ship that can be called: finished, not on an expedition,
    /// not moving, without assignment, empty.
    fn ship_is_free(&self, i: usize) -> bool {
        let s = &self.ships[i];
        let v = &s.vessel;
        s.finished()
            && v.launched
            && !v.expedition
            && v.aboard.is_empty()
            && !v.walker.is_walking()
            && v.called.is_none()
            && v.cargo.is_empty()
            && matches!(
                v.state,
                ShipState::Idle | ShipState::Search | ShipState::Docked
            )
    }

    /// The nearest free ship of `owner` accepted by `fits` (straight-line
    /// distance to `to`, the first in the ship list on ties).
    fn nearest_free_ship(
        &self,
        owner: PlayerId,
        to: Point,
        fits: impl Fn(&World, usize) -> bool,
    ) -> Option<usize> {
        let mut best: Option<(u32, usize)> = None;
        for i in 0..self.ships.len() {
            if self.ships[i].owner != owner || !self.ship_is_free(i) || !fits(self, i) {
                continue;
            }
            let d = grid::distance(self.ships[i].vessel.walker.at, to);
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, i));
            }
        }
        best.map(|(_, i)| i)
    }

    /// A ship is called to a harbour: it leaves what it was doing.
    fn call_ship(&mut self, ship: usize, harbour: Point) {
        let v = &mut self.ships[ship].vessel;
        v.called = Some(harbour);
        v.state = ShipState::Idle;
    }

    pub(crate) fn update_navy(&mut self) {
        for i in 0..self.ships.len() {
            if self.ships[i].finished() {
                self.update_ship(i);
            }
        }
        for h in 0..self.harbours.len() {
            self.update_harbour(h);
        }
    }

    fn update_ship(&mut self, i: usize) {
        if !self.ships[i].vessel.launched {
            // SHIP-2.1
            let field = self.launch_field(self.ships[i].at);
            let v = &mut self.ships[i].vessel;
            v.walker = Walker::new(field);
            v.launched = true;
            v.state = ShipState::Idle;
        }
        // Movement first, so that the update in which a ship arrives sees
        // it stopped.
        if self.ships[i].vessel.walker.is_walking() {
            let speed = self.ship_rules.speed;
            self.ships[i]
                .vessel
                .walker
                .advance(&self.terrain, speed, |_| true);
            let at = self.ships[i].vessel.walker.at;
            let owner = self.ships[i].owner;
            // SHIP-3.4
            if !self.explored(owner, at) {
                self.explore(owner, at, SAILING_EXPLORE);
            }
        }
        match self.ships[i].vessel.state {
            ShipState::Idle => self.ship_idle(i),
            ShipState::Search => self.ship_search(i),
            ShipState::ToHarbour => self.ship_to_harbour(i),
            ShipState::Docked => {
                // SHIP-2.6
                let v = &mut self.ships[i].vessel;
                if v.harbour.is_some() {
                    v.state = ShipState::Idle;
                } else {
                    v.state = ShipState::Search;
                    v.counter = 0;
                }
            }
            ShipState::Waiting => {
                // SHIP-6.3
                let v = &mut self.ships[i].vessel;
                if let Some(target) = v.target {
                    v.state = ShipState::Sailing;
                    self.sail(i, target.field);
                } else if !v.expedition {
                    v.state = ShipState::Returning;
                }
            }
            ShipState::Sailing => self.ship_sailing(i),
            ShipState::Unloading => self.ship_unloading(i),
            ShipState::MakingRoom => {
                if !self.ships[i].vessel.walker.is_walking() {
                    let v = &mut self.ships[i].vessel;
                    v.state = ShipState::Search;
                    v.counter = 0;
                }
            }
            ShipState::Returning => {
                // SHIP-6.5: when stopped, search at once.
                if !self.ships[i].vessel.walker.is_walking() {
                    let v = &mut self.ships[i].vessel;
                    v.state = ShipState::Search;
                    v.counter = PERIOD_LAST;
                }
            }
        }
    }

    /// SHIP-2.3
    fn ship_idle(&mut self, i: usize) {
        // 1: cargo aboard
        if !self.ships[i].vessel.cargo.is_empty() {
            self.ships[i].vessel.state = ShipState::Unloading;
            return;
        }
        // 2: an expedition without a target
        if self.ships[i].vessel.expedition && self.ships[i].vessel.target.is_none() {
            self.start_waiting(i);
            return;
        }
        // 3: called to a harbour
        if let Some(harbour) = self.ships[i].vessel.called {
            if self.docked_at(i) == Some(harbour) {
                // Waiting for the harbour to load it (SHIP-4.4).
                return;
            }
            self.release_links(i);
            let at = self.ships[i].vessel.walker.at;
            if self.reachable_by_sea(harbour, at) {
                if let Some(dock) = self.reserve_dock(harbour, i) {
                    if self.sail(i, dock) {
                        let v = &mut self.ships[i].vessel;
                        v.harbour = Some(harbour);
                        v.state = ShipState::ToHarbour;
                        return;
                    }
                    self.release_dock_reservation(harbour, i);
                }
            }
            // The harbour cannot be reached: the call is void.
            self.ships[i].vessel.called = None;
            if let Some(h) = self.harbours.iter_mut().find(|h| h.at == harbour) {
                if h.called == Some(i) {
                    h.called = None;
                }
                if h.expedition == ExpeditionState::WaitingForShip(i) {
                    h.expedition = ExpeditionState::GettingShip;
                    h.expedition_counter = 0;
                }
            }
        }
        // 4: expedition goods of a cancelled expedition, docked
        if !self.ships[i].vessel.aboard.is_empty() {
            if let Some(harbour) = self.docked_at(i) {
                let goods = std::mem::take(&mut self.ships[i].vessel.aboard);
                if let Some(w) = self.warehouses.get_mut(&harbour) {
                    for (good, count) in goods {
                        w.add(good, count);
                    }
                }
            }
        }
        // 5
        let v = &mut self.ships[i].vessel;
        v.state = ShipState::Search;
        v.counter = 0;
    }

    fn release_dock_reservation(&mut self, harbour: Point, ship: usize) {
        if let Some(h) = self.harbours.iter_mut().find(|h| h.at == harbour) {
            for d in &mut h.docks {
                if d.reserved == Some(ship) {
                    d.reserved = None;
                }
            }
        }
    }

    /// SHIP-2.4
    fn ship_search(&mut self, i: usize) {
        if self.ships[i].vessel.counter < PERIOD_LAST {
            self.ships[i].vessel.counter += 1;
            return;
        }
        self.ships[i].vessel.counter = 0;
        self.release_links(i);
        let owner = self.ships[i].owner;
        let at = self.ships[i].vessel.walker.at;
        let mut best: Option<(u32, Point)> = None;
        for h in &self.harbours {
            let mine = self.building_at(h.at).is_some_and(|b| b.owner == owner);
            if !mine || !self.reachable_by_sea(h.at, at) {
                continue;
            }
            let d = grid::distance(at, h.at);
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, h.at));
            }
        }
        if let Some((_, harbour)) = best {
            if let Some(dock) = self.reserve_dock(harbour, i) {
                if self.sail(i, dock) {
                    let v = &mut self.ships[i].vessel;
                    v.harbour = Some(harbour);
                    v.state = ShipState::ToHarbour;
                } else {
                    self.release_dock_reservation(harbour, i);
                }
                return;
            }
        }
        // SHIP-3.3: making room
        if self.terrain.has(at, state::LANDING_POSITION) {
            let (width, height) = (self.terrain.width, self.terrain.height);
            let free: Vec<Point> = grid::ring(at, 1, width, height)
                .into_iter()
                .filter(|&f| {
                    self.terrain.has(f, state::WATER)
                        && !self.terrain.has(f, state::LANDING_POSITION)
                        && self.region_of(f) == self.region_of(at)
                })
                .collect();
            if !free.is_empty() {
                let k = (u64::from(self.rng.draw()) % 6) as usize;
                let to = free[(k + 1).min(free.len()) - 1];
                if to != at && self.sail(i, to) {
                    self.ships[i].vessel.state = ShipState::MakingRoom;
                }
            }
        }
    }

    /// SHIP-2.5
    fn ship_to_harbour(&mut self, i: usize) {
        if self.ships[i].vessel.walker.is_walking() {
            return;
        }
        let Some(at) = self.ships[i].vessel.harbour else {
            self.ships[i].vessel.state = ShipState::Idle;
            return;
        };
        let field = self.ships[i].vessel.walker.at;
        let docked = self
            .harbours
            .iter_mut()
            .find(|h| h.at == at)
            .and_then(|h| {
                h.docks
                    .iter_mut()
                    .find(|d| d.reserved == Some(i) && d.field == field)
            })
            .map(|d| {
                d.reserved = None;
                d.docked = Some(i);
            })
            .is_some();
        let v = &mut self.ships[i].vessel;
        if docked {
            // SHIP-2.5, 2.7: the harbour building's own field, not the
            // docking place's water field.
            v.port = Some(at);
            v.state = ShipState::Docked;
        } else {
            v.harbour = None;
            v.state = ShipState::Idle;
        }
    }

    /// SHIP-4.6
    fn ship_unloading(&mut self, i: usize) {
        if self.ships[i].vessel.walker.is_walking() {
            return;
        }
        let Some(first) = self.ships[i].vessel.cargo.first().cloned() else {
            self.ships[i].vessel.state = ShipState::Idle;
            return;
        };
        let destination = first.destination;
        if self.docked_at(i) == Some(destination) {
            self.ships[i].vessel.cargo.remove(0);
            if let Some(w) = self.warehouses.get_mut(&destination) {
                for &(good, count) in &first.goods {
                    w.add(good, count);
                }
            }
            self.needs_met(destination, &first.goods);
            return;
        }
        self.release_links(i);
        let at = self.ships[i].vessel.walker.at;
        if self.harbour_at(destination).is_some() && self.reachable_by_sea(destination, at) {
            if let Some(dock) = self.reserve_dock(destination, i) {
                if self.sail(i, dock) {
                    let v = &mut self.ships[i].vessel;
                    v.harbour = Some(destination);
                    v.state = ShipState::ToHarbour;
                    return;
                }
                self.release_dock_reservation(destination, i);
            }
        }
        // The destination cannot be reached: the entry is lost (Q-066).
        self.ships[i].vessel.cargo.remove(0);
    }

    /// SHIP-4.6: the requests waiting for the goods are told.
    fn needs_met(&mut self, harbour: Point, goods: &[(u32, u32)]) {
        let Some(h) = self.harbours.iter_mut().find(|h| h.at == harbour) else {
            return;
        };
        for &(good, count) in goods {
            if let Some(n) = h.needs.iter_mut().find(|n| n.good == good) {
                n.count = n.count.saturating_sub(count);
            }
        }
        h.needs.retain(|n| n.count > 0);
    }

    /// SHIP-6.1, 2.3 case 2: the ship starts waiting for orders.
    fn start_waiting(&mut self, i: usize) {
        let owner = self.ships[i].owner;
        let at = self.ships[i].vessel.walker.at;
        self.ships[i].vessel.options = self.landing_options(i);
        self.ships[i].vessel.state = ShipState::Waiting;
        // UI-GAME-11.1, 4.5 c (E-trace-030): the row names the ship.
        self.ship_message(owner, at, WAITING_FOR_ORDERS, i);
    }

    /// SHIP-6.1
    pub(crate) fn landing_options(&self, ship: usize) -> [Option<Landing>; 6] {
        let v = &self.ships[ship].vessel;
        let owner = self.ships[ship].owner;
        let at = v.walker.at;
        let sea = self.region_of(at);
        let (width, height) = (self.terrain.width, self.terrain.height);
        let mut options: [Option<(u32, Landing)>; 6] = [None; 6];
        // The spots of the last port position's land continent (SHIP-2.7);
        // none without one.
        let own = v.port.map(|p| self.continent_spots(p)).unwrap_or_default();
        for &spot in &self.harbour_spots {
            if own.contains(&spot) && grid::distance(at, spot) <= OWN_HARBOUR_DISTANCE {
                continue;
            }
            if !self.harbour_site_for(owner, spot) {
                continue;
            }
            let Some(field) = (1..=LANDING_RINGS)
                .flat_map(|k| grid::ring(spot, k, width, height))
                .find(|&f| sea != 0 && self.region_of(f) == sea)
            else {
                continue;
            };
            let direction = usize::from(movement::direction_toward(at, spot));
            let distance = grid::distance(at, spot);
            // The nearest wins; on equal distance the later spot.
            if options[direction].is_none_or(|(d, _)| distance <= d) {
                options[direction] = Some((distance, Landing { spot, field }));
            }
        }
        options.map(|o| o.map(|(_, landing)| landing))
    }

    /// SHIP-6.4
    fn ship_sailing(&mut self, i: usize) {
        if self.ships[i].vessel.walker.is_walking() {
            return;
        }
        let owner = self.ships[i].owner;
        let at = self.ships[i].vessel.walker.at;
        // SHIP-2.7: the first land neighbour of the stop field; none when
        // no neighbour qualifies (ring 1 only).
        let port = self.first_land_neighbour(at);
        let v = &mut self.ships[i].vessel;
        v.port = port;
        v.target = None;
        self.ships[i].vessel.options = self.landing_options(i);
        self.explore(owner, at, ARRIVAL_EXPLORE);
        self.ships[i].vessel.state = ShipState::Waiting;
        // UI-GAME-11.1, 4.5 c (E-trace-030): the row names the ship.
        self.ship_message(owner, at, WAITING_FOR_ORDERS, i);
    }

    // ---- Harbours (SHIP-4, 5) ----

    fn update_harbour(&mut self, h: usize) {
        self.expedition_step(h);
        if self.harbours[h].counter < PERIOD_LAST {
            self.harbours[h].counter += 1;
            return;
        }
        self.harbours[h].counter = 0;
        self.harbour_pass(h);
    }

    fn harbour_owner(&self, at: Point) -> Option<PlayerId> {
        self.building_at(at).map(|b| b.owner)
    }

    /// SHIP-5.2 (T14, Q-297): the harbour's served orders of `good` not yet
    /// delivered: the wares on their way to it for an order (not storage
    /// trips, which are no order of it), in a warehouse helper's hands too.
    fn on_the_way(&self, at: Point, good: u32) -> u32 {
        let ordered = |t: &Option<crate::Target>| t.is_some_and(|t| t.at == at && !t.storage);
        let wares = self
            .wares()
            .filter(|(_, w)| w.good == good && ordered(&w.target))
            .count();
        let helpers = self
            .settlers()
            .filter(|(_, s)| {
                s.carrying == Some(good)
                    && matches!(&s.job, SettlerJob::CarryOut { target, .. } if ordered(target))
            })
            .count();
        (wares + helpers) as u32
    }

    /// Wares on their way to the building on `at`.
    fn wares_inbound(&self, at: Point, good: u32) -> u32 {
        self.wares()
            .filter(|(_, w)| w.good == good && w.target.is_some_and(|t| t.at == at))
            .count() as u32
    }

    /// SHIP-4.2 (c): the harbour can supply `good`.
    fn can_supply(&self, supplier: Point, good: u32) -> bool {
        let Some(w) = self.warehouses.get(&supplier) else {
            return false;
        };
        if !w.accepts(good) {
            return false;
        }
        if w.count(good) > 0 {
            return true;
        }
        let Some(owner) = self.harbour_owner(supplier) else {
            return false;
        };
        let network = self.route_costs_from(owner, self.flag_of(supplier));
        self.warehouses_of(owner).into_iter().any(|at| {
            self.warehouses[&at].count(good) > 0
                && (at == supplier || network.contains_key(&self.flag_of(at)))
        })
    }

    /// SHIP-4: one pass of a harbour, in the order 4.2, 4.3, 4.4.
    fn harbour_pass(&mut self, h: usize) {
        self.ask_for_needs(h);
        self.gather_transfers(h);
        self.load_ships(h);
    }

    /// SHIP-4.2
    fn ask_for_needs(&mut self, h: usize) {
        let at = self.harbours[h].at;
        let Some(owner) = self.harbour_owner(at) else {
            return;
        };
        // A need whose supplier can no longer supply it is cancelled.
        for n in 0..self.harbours[h].needs.len() {
            let need = self.harbours[h].needs[n].clone();
            if let Some(supplier) = need.supplier {
                // Goods the supplier has ordered into itself for the
                // transfer count as supply (Q-154).
                let gathering = self.open_orders(supplier, need.good)
                    + self.wares_inbound(supplier, need.good)
                    > 0;
                if !gathering && !self.can_supply(supplier, need.good) {
                    if let Some(s) = self.harbours.iter_mut().find(|x| x.at == supplier) {
                        s.transfers
                            .retain(|t| !(t.destination == at && t.good == need.good));
                    }
                    self.harbours[h].needs[n].supplier = None;
                }
            }
        }
        for n in 0..self.harbours[h].needs.len() {
            let need = self.harbours[h].needs[n].clone();
            if need.supplier.is_some() {
                continue;
            }
            let chosen = (0..self.harbours.len()).find(|&o| {
                let other = &self.harbours[o];
                other.at != at
                    && self.harbour_owner(other.at) == Some(owner)
                    && !need.asked.contains(&other.at)
                    && !other.needs.iter().any(|x| x.good == need.good)
                    && self.can_supply(other.at, need.good)
            });
            if let Some(o) = chosen {
                let supplier = self.harbours[o].at;
                self.harbours[o].transfers.push(Transfer {
                    good: need.good,
                    count: need.count,
                    destination: at,
                });
                let entry = &mut self.harbours[h].needs[n];
                entry.asked.push(supplier);
                entry.supplier = Some(supplier);
            }
        }
    }

    /// SHIP-4.3: order the shortfall into the supplying harbour.
    fn gather_transfers(&mut self, h: usize) {
        let at = self.harbours[h].at;
        let Some(owner) = self.harbour_owner(at) else {
            return;
        };
        let mut wanted: Vec<(u32, u32)> = Vec::new();
        for t in &self.harbours[h].transfers {
            match wanted.iter_mut().find(|(g, _)| *g == t.good) {
                Some((_, n)) => *n += t.count,
                None => wanted.push((t.good, t.count)),
            }
        }
        for (good, count) in wanted {
            let stock = self.warehouses.get(&at).map_or(0, |w| w.count(good));
            let target = self.target_of(at).expect("a harbour");
            let settler = self
                .goods
                .get(&good)
                .is_some_and(|g| g.kind == crate::GoodKind::Settler);
            let coming = if settler {
                u32::from(self.has_open_request(owner, Requester::Harbour(target)))
            } else {
                self.open_orders(at, good) + self.wares_inbound(at, good)
            };
            let missing = count.saturating_sub(stock + coming);
            if settler {
                if missing > 0 {
                    self.request(owner, Requester::Harbour(target), good);
                }
            } else {
                for _ in 0..missing {
                    self.order(at, good);
                }
            }
        }
    }

    /// SHIP-4.4, 4.5
    fn load_ships(&mut self, h: usize) {
        let at = self.harbours[h].at;
        let Some(owner) = self.harbour_owner(at) else {
            return;
        };
        let limit = self.ship_rules.stock_limit as usize;
        let mut loaded = false;
        let mut served = false;
        for n in 0..self.harbours[h].transfers.len() {
            let Some(t) = self.harbours[h].transfers.get(n).cloned() else {
                break;
            };
            let ready = self
                .warehouses
                .get(&at)
                .is_some_and(|w| w.count(t.good) >= t.count);
            if self.harbours[h].called.is_none() && ready {
                let destination = t.destination;
                let ship = self.nearest_free_ship(owner, at, |w, i| {
                    let field = w.ships[i].vessel.walker.at;
                    w.reachable_by_sea(at, field) && w.reachable_by_sea(destination, field)
                });
                if let Some(ship) = ship {
                    self.call_ship(ship, at);
                    self.harbours[h].called = Some(ship);
                }
            }
            if let Some(ship) = self.harbours[h].called {
                if self.docked_at(ship) == Some(at)
                    && self.ships[ship].vessel.cargo.len() < limit
                    && ready
                {
                    let w = self.warehouses.get_mut(&at).expect("a warehouse");
                    for _ in 0..t.count {
                        w.take(t.good);
                    }
                    self.ships[ship].vessel.cargo.push(Cargo {
                        destination: t.destination,
                        goods: vec![(t.good, t.count)],
                    });
                    served = true;
                    loaded = true;
                    // Done: the order goes (its slot is reused below).
                    self.harbours[h].transfers[n].count = 0;
                }
            }
        }
        self.harbours[h].transfers.retain(|t| t.count > 0);
        // SHIP-4.5: the ship is released to sail when it is here and
        // something was loaded or nothing could be served.
        if let Some(ship) = self.harbours[h].called {
            if self.docked_at(ship) == Some(at) && (loaded || !served) {
                self.harbours[h].called = None;
                let v = &mut self.ships[ship].vessel;
                v.called = None;
                v.state = ShipState::Idle;
            }
        }
    }

    // ---- Expeditions (SHIP-5) ----

    /// SHIP-5.1: the harbour on `at` starts preparing an expedition.
    pub(crate) fn expedition_start(
        &mut self,
        player: PlayerId,
        at: Point,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        if self.harbour_owner(at) != Some(player) {
            return Err(Rejection::NotAHarbour);
        }
        let h = self
            .harbours
            .iter_mut()
            .find(|h| h.at == at)
            .ok_or(Rejection::NotAHarbour)?;
        if h.expedition == ExpeditionState::Inactive {
            h.expedition = ExpeditionState::Gathering;
            h.expedition_counter = 0;
        }
        Ok(())
    }

    fn expedition_step(&mut self, h: usize) {
        match self.harbours[h].expedition {
            ExpeditionState::Inactive => {}
            ExpeditionState::Gathering | ExpeditionState::GettingShip => {
                if self.harbours[h].expedition_counter < PERIOD_LAST {
                    self.harbours[h].expedition_counter += 1;
                    return;
                }
                self.harbours[h].expedition_counter = 0;
                if self.harbours[h].expedition == ExpeditionState::Gathering {
                    self.gather_expedition(h);
                } else {
                    self.get_ship(h);
                }
            }
            ExpeditionState::WaitingForShip(ship) => {
                // SHIP-5.4
                if self.docked_at(ship) == Some(self.harbours[h].at) {
                    self.transfer_to_ship(h, ship);
                }
            }
        }
    }

    /// SHIP-5.2
    /// SHIP-5.2: the materials an expedition at the harbour on `at` takes: its
    /// own building cost, frame list then main list.
    fn expedition_materials(&self, at: Point) -> Vec<(u32, u32)> {
        let mut required: Vec<(u32, u32)> = Vec::new();
        let Some(building) = self.building_at(at) else {
            return required;
        };
        let kind = &self.building_types[&building.building_type];
        for &(good, count) in kind.frame_costs.iter().chain(&kind.costs) {
            match required.iter_mut().find(|(g, _)| *g == good) {
                Some((_, n)) => *n += count,
                None => required.push((good, count)),
            }
        }
        required
    }

    /// UI-GAME-8.1, SHIP-5.2: what the harbour's expedition page lists: the
    /// materials and the builder an expedition takes, with what is set aside so
    /// far. Empty where no harbour is.
    pub fn expedition_goods(&self, at: Point) -> Vec<ExpeditionGood> {
        let Some(harbour) = self.harbour_at(at) else {
            return Vec::new();
        };
        let mut goods: Vec<ExpeditionGood> = self
            .expedition_materials(at)
            .into_iter()
            .map(|(good, required)| ExpeditionGood {
                good,
                required,
                aside: harbour
                    .aside
                    .iter()
                    .find(|(g, _)| *g == good)
                    .map_or(0, |&(_, n)| n),
            })
            .collect();
        if let Some(builder) = self.roles.builder {
            goods.push(ExpeditionGood {
                good: builder,
                required: 1,
                aside: u32::from(harbour.builder_aside),
            });
        }
        goods
    }

    pub(crate) fn gather_expedition(&mut self, h: usize) {
        let at = self.harbours[h].at;
        if self.building_at(at).is_none() {
            return;
        }
        let required = self.expedition_materials(at);
        let mut nothing_missing = true;
        for (good, count) in required {
            let have = self.harbours[h]
                .aside
                .iter()
                .find(|(g, _)| *g == good)
                .map_or(0, |&(_, n)| n);
            let mut missing = count.saturating_sub(have);
            if missing == 0 {
                continue;
            }
            let stock = self.warehouses.get(&at).map_or(0, |w| w.count(good));
            let take = missing.min(stock);
            if take > 0 {
                let w = self.warehouses.get_mut(&at).expect("a warehouse");
                for _ in 0..take {
                    w.take(good);
                }
                let aside = &mut self.harbours[h].aside;
                match aside.iter_mut().find(|(g, _)| *g == good) {
                    Some((_, n)) => *n += take,
                    None => aside.push((good, take)),
                }
                missing -= take;
            }
            // The orders of the good are made to equal what is still
            // missing. T14: all its orders count, the served ones not yet
            // delivered too (a helper's ware included); only open ones are
            // cancelled.
            let open = self.open_orders(at, good);
            let orders = open + self.on_the_way(at, good);
            if orders < missing {
                for _ in orders..missing {
                    self.order(at, good);
                }
            } else if orders > missing {
                self.cancel_orders(at, good, (orders - missing).min(open));
            }
            if missing > 0 {
                nothing_missing = false;
            }
        }
        // The builder.
        if !self.harbours[h].builder_aside {
            if let (Some(builder), Some(owner)) = (self.roles.builder, self.harbour_owner(at)) {
                let in_stock = self
                    .warehouses
                    .get(&at)
                    .is_some_and(|w| w.count(builder) > 0);
                if in_stock {
                    self.warehouses
                        .get_mut(&at)
                        .expect("a warehouse")
                        .take(builder);
                    self.harbours[h].builder_aside = true;
                } else {
                    let target = self.target_of(at).expect("a harbour");
                    self.request(owner, Requester::Harbour(target), builder);
                }
            }
        }
        if nothing_missing && self.harbours[h].builder_aside {
            self.harbours[h].expedition = ExpeditionState::GettingShip;
            self.harbours[h].expedition_counter = 0;
        }
    }

    /// SHIP-5.3
    fn get_ship(&mut self, h: usize) {
        let at = self.harbours[h].at;
        let Some(owner) = self.harbour_owner(at) else {
            return;
        };
        let docks: Vec<Point> = self.harbours[h].docks.iter().map(|d| d.field).collect();
        for dock in docks {
            let sea = self.region_of(dock);
            let ship = self.nearest_free_ship(owner, at, |w, i| {
                w.region_of(w.ships[i].vessel.walker.at) == sea
            });
            if let Some(ship) = ship {
                self.call_ship(ship, at);
                self.harbours[h].expedition = ExpeditionState::WaitingForShip(ship);
                return;
            }
        }
    }

    /// SHIP-5.5
    fn transfer_to_ship(&mut self, h: usize, ship: usize) {
        let mut goods = std::mem::take(&mut self.harbours[h].aside);
        if self.harbours[h].builder_aside {
            if let Some(builder) = self.roles.builder {
                goods.push((builder, 1));
            }
        }
        self.harbours[h].builder_aside = false;
        self.harbours[h].expedition = ExpeditionState::Inactive;
        self.harbours[h].expedition_counter = 0;
        self.release_links(ship);
        let v = &mut self.ships[ship].vessel;
        v.aboard = goods;
        v.expedition = true;
        v.called = None;
        v.target = None;
        v.state = ShipState::Idle;
    }

    // ---- Names (UI-GAME-4.5 c) ----

    /// UI-GAME-4.5 c: the owner's ship-name counter at the creation of ship
    /// `i`: 0 for a player's first ship, 1 more for each later one, back to
    /// 0 after 20 (ships stay in the list, so it is the number of the
    /// owner's earlier ships modulo 21). The default name is the language
    /// text of `<Tribe>ShipName<k>`.
    pub fn ship_name_index(&self, i: usize) -> Option<u32> {
        let owner = self.ships.get(i)?.owner;
        let earlier = self.ships[..i].iter().filter(|s| s.owner == owner).count();
        Some((earlier % SHIP_NAMES) as u32)
    }

    /// UI-GAME-4.5 c: the player names its ship `ship`.
    pub(crate) fn name_ship(
        &mut self,
        player: PlayerId,
        ship: u32,
        name: String,
    ) -> Result<(), Rejection> {
        let s = self
            .ships
            .get_mut(ship as usize)
            .filter(|s| s.owner == player)
            .ok_or(Rejection::NoSuchShip)?;
        s.name = Some(name);
        Ok(())
    }

    // ---- The voyage (SHIP-6, 7) ----

    fn expedition_ship(&self, player: PlayerId, ship: u32) -> Result<usize, Rejection> {
        let i = ship as usize;
        let s = self.ships.get(i).ok_or(Rejection::NoSuchShip)?;
        if s.owner != player || !s.finished() {
            return Err(Rejection::NoSuchShip);
        }
        Ok(i)
    }

    /// SHIP-6.2: command "go" in direction `direction`.
    pub(crate) fn expedition_go(
        &mut self,
        player: PlayerId,
        ship: u32,
        direction: u8,
    ) -> Result<(), Rejection> {
        let i = self.expedition_ship(player, ship)?;
        let v = &mut self.ships[i].vessel;
        if !v.expedition || v.target.is_some() {
            return Err(Rejection::NotOnExpedition);
        }
        let option = v
            .options
            .get(usize::from(direction))
            .ok_or(Rejection::NoSuchDirection)?;
        v.target = *option;
        Ok(())
    }

    /// SHIP-6.5: cancel the expedition.
    pub(crate) fn expedition_stop(&mut self, player: PlayerId, ship: u32) -> Result<(), Rejection> {
        let i = self.expedition_ship(player, ship)?;
        let v = &mut self.ships[i].vessel;
        if !v.expedition {
            return Err(Rejection::NotOnExpedition);
        }
        v.expedition = false;
        v.target = None;
        v.options = [None; 6];
        if matches!(v.state, ShipState::Waiting | ShipState::Sailing) {
            v.state = ShipState::Returning;
        }
        Ok(())
    }

    /// SHIP-7.1: among the harbour spots of the land continent of the
    /// ship's last port position (SHIP-2.7), the one nearest to the ship's
    /// own field (a strictly smaller distance wins, so the first in the
    /// continent's list on equal distance), if a harbour can be built
    /// there for the player.
    fn founding_spot(&self, i: usize) -> Option<Point> {
        let v = &self.ships[i].vessel;
        let port = v.port?;
        let ship_at = v.walker.at;
        let spot = self
            .continent_spots(port)
            .into_iter()
            .min_by_key(|&s| grid::distance(ship_at, s))?;
        self.harbour_site_for(self.ships[i].owner, spot)
            .then_some(spot)
    }

    /// SHIP-7.1: the harbour spots of the land continent `field` is on, in
    /// the continent's own order (FMT-S2M-4.9: its object's positions, by
    /// the stored continent index). A world made without a map file has no
    /// continent objects: there the spots of `field`'s computed region
    /// stand in, in map order.
    fn continent_spots(&self, field: Point) -> Vec<Point> {
        if self.land_spots.is_empty() {
            let region = self.region_of(field);
            return self
                .harbour_spots
                .iter()
                .copied()
                .filter(|&s| self.region_of(s) == region)
                .collect();
        }
        self.continent_of(field)
            .map(|c| self.land_spots(c).to_vec())
            .unwrap_or_default()
    }

    /// SHIP-2.7: whether `field` belongs to a land continent: its stored
    /// continent index has a land continent object (FMT-S2M-4.9). In a
    /// world made without a map file, its computed region is land.
    pub(crate) fn on_land_continent(&self, field: Point) -> bool {
        if self.land_spots.is_empty() {
            return self.region_of(field) != 0 && !self.terrain.has(field, state::WATER);
        }
        self.continent_of(field)
            .is_some_and(|c| self.land_spots.contains_key(&c))
    }

    /// SHIP-2.7: the first land neighbour of `field`: its six neighbours
    /// in the order of directions 4, 5, 0, 1, 2, 3 (ring 1 from its start
    /// field), the first inside the map that is not Water and lies on a
    /// land continent; none when none of the six does.
    pub(crate) fn first_land_neighbour(&self, field: Point) -> Option<Point> {
        [4u8, 5, 0, 1, 2, 3]
            .into_iter()
            .filter_map(|d| self.terrain.neighbour(field, d))
            .find(|&n| !self.terrain.has(n, state::WATER) && self.on_land_continent(n))
    }

    /// SHIP-7, UI-GAME-4.5 (f): whether `Command::ExpeditionBuildHarbor`
    /// would be accepted for the player's ship `ship` now (its window then
    /// shows the button).
    pub fn can_found_harbour(&self, player: PlayerId, ship: usize) -> bool {
        let Ok(i) = self.expedition_ship(player, ship as u32) else {
            return false;
        };
        let v = &self.ships[i].vessel;
        if !v.expedition || v.walker.is_walking() {
            return false;
        }
        self.founding_spot(i)
            .is_some_and(|spot| grid::distance(v.walker.at, spot) < BUILD_DISTANCE)
    }

    /// SHIP-7: found a harbour.
    pub(crate) fn expedition_build_harbour(
        &mut self,
        player: PlayerId,
        ship: u32,
    ) -> Result<(), Rejection> {
        let i = self.expedition_ship(player, ship)?;
        let v = &self.ships[i].vessel;
        if !v.expedition || v.walker.is_walking() {
            return Err(Rejection::NotOnExpedition);
        }
        let ship_at = v.walker.at;
        let spot = self.founding_spot(i).ok_or(Rejection::NoHarbourSpot)?;
        if grid::distance(ship_at, spot) >= BUILD_DISTANCE {
            return Err(Rejection::NoHarbourSpot);
        }
        let kind = self
            .building_types
            .values()
            .find(|t| t.size == crate::BuildingSize::Harbor)
            .map(|t| (t.id, t.size))
            .ok_or(Rejection::UnknownBuildingType)?;
        let aboard = std::mem::take(&mut self.ships[i].vessel.aboard);
        // The site first, then its claim (TER-2.4).
        let new_flag = self.new_flag_of(spot);
        let mut touched = self.put_building(player, kind.0, kind.1, spot, true);
        // ECO-HQ-4.8a: the site's field takes the mean height around it;
        // 4.8b: then a new flag's height ring.
        touched.extend(self.level_site_height(spot));
        if let Some(flag) = new_flag {
            touched.extend(self.flag_height_ring(flag));
        }
        self.start_construction(spot);
        self.claim_landing(spot);
        // SHIP-7.2: the materials aboard become the site's delivered
        // material, so its orders are cancelled; the builder is on the site.
        let builder = self.roles.builder;
        // SIM-START-4.2, 4.4 (f'): settlers travel as counts of the ship's
        // store; the builder aboard is made at the founding, after the
        // harbour site's own numbers, and takes one settler number.
        let number = aboard
            .iter()
            .any(|&(good, _)| Some(good) == builder)
            .then(|| self.new_number());
        let mut delivered: Vec<(u32, u32)> = Vec::new();
        if let Some(site) = self.sites.get_mut(&spot) {
            for &(good, count) in &aboard {
                if Some(good) == builder {
                    if site.builder.is_none() {
                        site.builder = number.map(crate::Builder::arrived);
                    }
                    continue;
                }
                let n = count.min(site.still_needed(good));
                if n > 0 {
                    crate::construction::add(&mut site.delivered, good, n);
                    crate::construction::add(&mut site.on_site, good, n);
                    delivered.push((good, n));
                }
            }
        }
        for (good, n) in delivered {
            self.cancel_orders(spot, good, n);
        }
        self.changed(&touched);
        let v = &mut self.ships[i].vessel;
        v.expedition = false;
        v.target = None;
        v.options = [None; 6];
        v.state = ShipState::Idle;
        Ok(())
    }
}
