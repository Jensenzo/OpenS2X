//! Searches over a radius, game animals and ships: the mine's ore field
//! (`specs/ECO-food-mining.md` ECO-FOOD-2), the hunter's animal search
//! (ECO-FOOD-5), the fisherman's spot (ECO-FOOD-4) and the shipyard's ship
//! (ECO-FOOD-7).

use std::collections::{BTreeSet, VecDeque};

use crate::grid::{self, Point};
use crate::map::state;
use crate::movement::{Walker, NOT_WALKABLE};
use crate::requests::Target;
use crate::{PlayerId, World};

/// MAP-GRID-3.2: neighbour order of a flood (ring order).
const FLOOD_ORDER: [u8; 6] = [5, 0, 1, 2, 3, 4];

/// ECO-FOOD-5.7: what `animals.lua` says about an animal kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimalKind {
    pub id: u32,
    /// `an_setRange`: fields from home it roams; -1 when unset.
    pub range: i32,
    /// `an_setMoveDistance` (default 3).
    pub move_distance: u32,
    /// `an_setMovementSpeed` in millionths of a step per tick (default
    /// 0.1).
    pub speed: u32,
    /// `an_setAvoidBuildings` (default true).
    pub avoid_buildings: bool,
    /// `an_setIdleTimeFactor` in thousandths (default 3.0).
    pub idle_thousandths: u32,
}

impl AnimalKind {
    /// The values of an animal the scripts leave unset.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            range: -1,
            move_distance: 3,
            speed: 100_000,
            avoid_buildings: true,
            idle_thousandths: 3000,
        }
    }
}

/// ECO-FOOD-5.7: what a roaming animal is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimalState {
    /// Counting down updates; below 0 it starts choosing (5.8).
    Waiting { left: i32 },
    /// Looking for a place to walk to, with its ticket (5.9).
    Choosing { ticket: i32 },
    /// Walking since the tick clock read `since` (the bits of the float32
    /// clock, ECO-FOOD-5.11b), at `speed` (millionths of a step per tick)
    /// and with the idle time factor `idle_thousandths` of its kind.
    Walking {
        since: u32,
        speed: u32,
        idle_thousandths: u32,
    },
    /// Reserved by a hunter: stands still.
    Hunted,
}

/// A game animal (FMT-S2M-5.4) that roams around its home (ECO-FOOD-5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animal {
    pub kind: u32,
    /// The field it is on (the one it last reached while walking).
    pub at: Point,
    /// ECO-FOOD-5.7: where it was placed.
    pub home: Point,
    pub state: AnimalState,
    pub walker: Walker,
    /// ECO-FOOD-5.2: the hut it is reserved for ("hunted" state).
    pub hunted_by: Option<Target>,
    /// ECO-FOOD-5.2 (T59), 5.10a: when it was registered on its field: 0
    /// from the map, else the tick it was made or its step onto the field
    /// completed, plus 1. Of equal values the one updated first, the lower
    /// id, was registered first.
    pub registered: u64,
}

impl Animal {
    /// A waiting animal standing on its home field.
    pub fn new(kind: u32, at: Point) -> Self {
        Self {
            kind,
            at,
            home: at,
            state: AnimalState::Waiting { left: 0 },
            walker: Walker::new(at),
            hunted_by: None,
            registered: 0,
        }
    }

    /// ECO-FOOD-5.10a: the field it last reached is its registered field;
    /// a new one is registered in tick `tick` (T59).
    fn register(&mut self, tick: u64) {
        if self.walker.at != self.at {
            self.at = self.walker.at;
            self.registered = tick + 1;
        }
    }

    /// ECO-FOOD-5.2 (T50), AI-CON-6.3: the field its current walk ends on,
    /// its own field while it stands: where the hunter's search tests the
    /// player's exploration.
    pub fn walk_goal(&self) -> Point {
        self.walker
            .path
            .back()
            .copied()
            .or(self.walker.next)
            .unwrap_or(self.at)
    }
}

/// ECO-FOOD-5.11, 5.11b: the wait after a walk, `trunc(((t_end - t_start) x
/// idle) x 10)` in single precision throughout; the idle factor is at
/// least 2 (E-eco-014).
pub(crate) fn animal_wait(t_start: f32, t_end: f32, idle_thousandths: u32) -> i32 {
    let elapsed = t_end - t_start;
    let idle = idle_thousandths.max(2000) as f32 / 1000.0;
    ((elapsed * idle) * 10.0f32).trunc() as i32
}

/// ECO-FOOD-5.15: the map's respawn record (FMT-S2M-5.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Respawn {
    /// Gains `rate` a tick; a check per 1000.
    pub accumulator: u32,
    pub rate: u32,
    /// The respawn grid's cell last checked.
    pub cursor: (u32, u32),
}

/// ECO-FOOD-5.15: the respawn grid's cells, 16 x 16 fields, and their
/// 4 x 4 sub-cells.
const RESPAWN_CELL: u32 = 16;
const RESPAWN_SUB: u32 = 4;
/// ECO-FOOD-5.15: the flood's reach from a sub-cell's corner.
const RESPAWN_REACH: u32 = 8;

/// A ship under construction or finished (ECO-FOOD-7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ship {
    pub owner: PlayerId,
    /// ECO-FOOD-7.3b: its building field, a Coast field.
    pub at: Point,
    /// ECO-FOOD-7.3b: its water field, the building field's neighbour it
    /// was started over (none for a ship made otherwise).
    pub water: Option<Point>,
    /// ECO-FOOD-7.4a, 7.4b (T60): the work left on the frame part, then
    /// the main part, single-precision values from 1 down to 0 (their
    /// bits).
    pub frame_work: u32,
    pub main_work: u32,
    /// ECO-FOOD-7.4a: the amount of each decrement of the work step under
    /// way (its bits), fixed when the step starts; none between steps.
    pub step_amount: Option<u32>,
    /// SHIP-2: what the finished ship does.
    pub vessel: crate::navy::Vessel,
    /// UI-GAME-4.5 c: the name typed in the ship's window (its `ShipName`);
    /// none: the default name ([`crate::World::ship_name_index`]).
    pub name: Option<String>,
}

impl Ship {
    /// Where the ship is: its building site until it has launched
    /// (SHIP-2.1), then the field it sails on.
    pub fn position(&self) -> Point {
        if self.vessel.launched {
            self.vessel.walker.at
        } else {
            self.at
        }
    }

    /// ECO-FOOD-7.4b (T60): finished once both parts are 0, in the tick
    /// of that decrement.
    pub fn finished(&self) -> bool {
        self.frame_work == 0 && self.main_work == 0
    }

    /// ECO-FOOD-7.4a: one decrement: the frame part while any is left, else
    /// the main part, by `amount`; a residue below it is taken whole, so a
    /// part never goes below 0 (and none carries over).
    pub(crate) fn decrement(&mut self, amount: f32) {
        let part = if self.frame_work != 0 {
            &mut self.frame_work
        } else {
            &mut self.main_work
        };
        let left = f32::from_bits(*part) - amount;
        *part = if left > 0.0 { left.to_bits() } else { 0 };
    }
}

impl World {
    /// ECO-FOOD-2.4, 5.2: every field within `radius` of `at`, in flood
    /// order (breadth first, ring order, `at` first). Terrain does not
    /// matter, so this is the fields of the area in the order a flood
    /// enters them.
    pub(crate) fn flood_within(&self, at: Point, radius: u32) -> Vec<Point> {
        let mut entered = vec![false; self.terrain.len()];
        let mut order = vec![at];
        let mut queue = VecDeque::from([at]);
        entered[self.terrain.index(at)] = true;
        while let Some(a) = queue.pop_front() {
            for direction in FLOOD_ORDER {
                let Some(b) = self.terrain.neighbour(a, direction) else {
                    continue;
                };
                let index = self.terrain.index(b);
                if entered[index] || grid::distance(at, b) > radius {
                    continue;
                }
                entered[index] = true;
                order.push(b);
                queue.push_back(b);
            }
        }
        order
    }

    /// ECO-FOOD-2.4: the first field in flood order within `radius` whose
    /// resource is `good` (amount at least 1).
    pub(crate) fn find_ore_field(&self, at: Point, good: u32, radius: u32) -> Option<Point> {
        self.flood_within(at, radius).into_iter().find(|&f| {
            self.resource_at(f)
                .is_some_and(|r| r.good == good && r.amount > 0)
        })
    }

    /// ECO-FOOD-5.2: the first animal in flood order, by the field it is on,
    /// whose walk's end the hut's owner has explored (T50: not its own
    /// field, [`Animal::walk_goal`]) and no other hut has reserved.
    pub(crate) fn find_animal(&self, at: Point, owner: PlayerId, radius: u32) -> Option<usize> {
        let near: Vec<(usize, Point, u64)> = self
            .animals
            .iter()
            .enumerate()
            .filter_map(|(id, a)| {
                let a = a.as_ref()?;
                (a.hunted_by.is_none()
                    && grid::distance(at, a.at) <= radius
                    && self.explored(owner, a.walk_goal()))
                .then_some((id, a.at, a.registered))
            })
            .collect();
        if near.is_empty() {
            return None;
        }
        // T59: of several on the first field, the one registered there
        // first.
        self.flood_within(at, radius).into_iter().find_map(|f| {
            near.iter()
                .filter(|&&(_, p, _)| p == f)
                .min_by_key(|&&(id, _, registered)| (registered, id))
                .map(|&(id, _, _)| id)
        })
    }

    /// The animal with this id.
    pub fn animal(&self, id: usize) -> Option<&Animal> {
        self.animals.get(id)?.as_ref()
    }

    /// All animals, with their ids.
    pub fn animals(&self) -> impl Iterator<Item = (usize, &Animal)> {
        self.animals
            .iter()
            .enumerate()
            .filter_map(|(id, a)| a.as_ref().map(|a| (id, a)))
    }

    /// ECO-FOOD-5.2: the animal the hunter's hut on `at` has reserved.
    pub fn hunted_animal(&self, at: Point) -> Option<usize> {
        self.hunted.get(&at).copied()
    }

    /// ECO-FOOD-4.3: the fish field of the fisherman's hut on `at`.
    pub fn fish_field(&self, at: Point) -> Option<Point> {
        self.fish_fields.get(&at).copied()
    }

    /// ECO-FOOD-7: all ships.
    pub fn ships(&self) -> &[Ship] {
        &self.ships
    }

    /// ECO-FOOD-5.13: an animal as the map's record holds it: waiting with
    /// the wait left, choosing with its ticket, or walking where it was
    /// saved (its walk counted from the game's start, Q-233).
    pub(crate) fn map_animal(&self, a: &crate::MapAnimal) -> Animal {
        let mut animal = Animal::new(a.kind, a.field);
        animal.home = a.home;
        let kind = self.animal_kinds.get(&a.kind).copied();
        animal.state = match (a.state, &a.walk, kind) {
            (1, _, _) => AnimalState::Choosing { ticket: a.counter },
            (2, Some((step, progress, rest)), Some(kind)) => {
                animal
                    .walker
                    .walk(std::iter::once(*step).chain(rest.iter().copied()));
                animal.walker.next = animal.walker.path.pop_front();
                animal.walker.progress_bits = *progress;
                animal.walker.rate_bits =
                    crate::movement::step_rate(&self.terrain, kind.speed, a.field, *step, false)
                        .to_bits();
                AnimalState::Walking {
                    since: 0,
                    speed: kind.speed,
                    idle_thousandths: kind.idle_thousandths,
                }
            }
            _ => AnimalState::Waiting { left: a.counter },
        };
        animal
    }

    /// Put an animal on the map (tests).
    pub fn add_animal(&mut self, kind: u32, at: Point) -> usize {
        self.animals.push(Some(Animal::new(kind, at)));
        self.animals.len() - 1
    }

    // ---- Roaming (ECO-FOOD-5.7 to 5.11) ----

    /// The game time in seconds, summed in 32-bit floats (0.1 per tick).
    pub fn game_seconds(&self) -> f32 {
        f32::from_bits(self.game_time)
    }

    /// ECO-FOOD-5.7: every animal's update, in list order, once per tick in
    /// the resource update; the tick clock gains 0.1 after it (5.11b), so
    /// the animals read the sum of the ticks before this one.
    pub(crate) fn update_animals(&mut self) {
        self.update_animal_list();
        self.game_time = (self.game_seconds() + 0.1f32).to_bits();
    }

    fn update_animal_list(&mut self) {
        // 5.9: the start count of this pass.
        let mut started = 0;
        let now = self.game_seconds();
        for i in 0..self.animals.len() {
            // 5.7: a reserved animal never waits or chooses again; a walk it
            // is on goes on to its end, moved as in walking, and from there
            // it stands until killed, the walk's end setting no wait.
            // Reserved while waiting it keeps its wait counter as it is
            // (T47): released, it counts down from there.
            if let Some(a) = self.animals[i].as_mut().filter(|a| a.hunted_by.is_some()) {
                if matches!(a.state, AnimalState::Waiting { .. }) {
                    continue;
                }
                if let AnimalState::Walking { speed, .. } = a.state {
                    if a.walker.is_walking() {
                        a.walker.advance_level(&self.terrain, speed);
                        a.register(self.tick);
                        if !a.walker.is_walking() {
                            a.state = AnimalState::Hunted;
                        }
                        continue;
                    }
                }
                a.state = AnimalState::Hunted;
                continue;
            }
            // The two common cases in place: counting down a wait and
            // walking.
            if let Some(a) = self.animals[i].as_mut().filter(|a| a.hunted_by.is_none()) {
                match &mut a.state {
                    AnimalState::Waiting { left } if *left >= 1 => {
                        *left -= 1;
                        continue;
                    }
                    // 5.9 rules 1 and 4: the ticket only rises.
                    AnimalState::Choosing { ticket }
                        if started >= 2 || self.animal_top > *ticket =>
                    {
                        *ticket += 1;
                        continue;
                    }
                    AnimalState::Walking {
                        since,
                        speed,
                        idle_thousandths,
                    } => {
                        let (since, speed, idle) = (*since, *speed, *idle_thousandths);
                        // 5.12: slopes do not slow animals.
                        a.walker.advance_level(&self.terrain, speed);
                        a.register(self.tick);
                        if !a.walker.is_walking() {
                            // 5.11, 5.11b: waits `trunc(((t_end - t_start) x
                            // idle) x 10)`, the float32 clock values taken
                            // in double precision; the idle factor is at
                            // least 2 (E-eco-014).
                            a.state = AnimalState::Waiting {
                                left: animal_wait(f32::from_bits(since), now, idle),
                            };
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            let Some(mut animal) = self.animals[i].take() else {
                continue;
            };
            self.animal_step(&mut animal, &mut started);
            self.animals[i] = Some(animal);
        }
    }

    fn animal_step(&mut self, animal: &mut Animal, started: &mut u32) {
        let Some(kind) = self.animal_kinds.get(&animal.kind).copied() else {
            return;
        };
        if animal.hunted_by.is_some() {
            // 5.7: handled in `update_animal_list`.
            return;
        }
        match animal.state {
            AnimalState::Hunted => animal.state = AnimalState::Waiting { left: 0 },
            AnimalState::Waiting { left } => {
                // 5.8
                animal.state = if left - 1 < 0 {
                    AnimalState::Choosing { ticket: -1 }
                } else {
                    AnimalState::Waiting { left: left - 1 }
                };
            }
            AnimalState::Choosing { ticket } => {
                // 5.9
                let mut ticket = ticket;
                let choose = if *started >= 2 {
                    ticket += 1;
                    false
                } else if self.animal_top < ticket {
                    self.animal_top = ticket;
                    true
                } else if self.animal_top == ticket {
                    self.animal_top = 0;
                    *started += 1;
                    true
                } else {
                    ticket += 1;
                    false
                };
                animal.state = AnimalState::Choosing { ticket };
                if choose {
                    // 5.10, 5.11: with no target, or no walk to it, the
                    // animal stays choosing with ticket 100.
                    let walk = self
                        .animal_target(animal, &kind)
                        .and_then(|target| self.animal_walk(animal.at, target, &kind));
                    animal.state = match walk {
                        Some(walk) => {
                            animal.walker.walk(walk);
                            AnimalState::Walking {
                                since: self.game_time,
                                speed: kind.speed,
                                idle_thousandths: kind.idle_thousandths,
                            }
                        }
                        None => AnimalState::Choosing { ticket: 100 },
                    };
                }
            }
            // Walking is advanced in `update_animals`.
            AnimalState::Walking { .. } => {}
        }
    }

    /// ECO-FOOD-5.10: the field the animal walks to.
    fn animal_target(&mut self, animal: &Animal, kind: &AnimalKind) -> Option<Point> {
        if kind.range < 0 {
            return None;
        }
        let range = kind.range as u32;
        // The current field, or home when the animal is farther than its
        // range from home.
        let center = if grid::distance(animal.home, animal.at) > range {
            animal.home
        } else {
            animal.at
        };
        let md = u64::from(kind.move_distance);
        let n = u64::from(self.rng.draw()) % (3 * md * (md + 1)).max(1) + 1;
        let (w, h) = (self.terrain.width, self.terrain.height);
        // 5.10a: the fields animals are registered on, its own included (it
        // is out of the list while it chooses).
        let occupied: BTreeSet<Point> = self
            .animals
            .iter()
            .flatten()
            .chain([animal])
            .map(|a| a.at)
            .collect();
        // A building or site counts on its own field only, and the field
        // itself is within 2.
        let near_building = |f: Point| {
            grid::area(f, 2, w, h)
                .into_iter()
                .any(|g| self.building_at(g).is_some())
        };
        let bad = state::STREET | state::FOR_MINING | state::BORDER | state::FOR_HARBOR;
        let mut seen = BTreeSet::from([center]);
        let mut queue = VecDeque::from([center]);
        let (mut count, mut last) = (0u64, None);
        while let Some(f) = queue.pop_front() {
            // T48: no flag on the field itself, for every kind.
            if !self.terrain.has(f, bad)
                && !occupied.contains(&f)
                && self.flag_at(f).is_none()
                && !(kind.avoid_buildings && near_building(f))
            {
                count += 1;
                last = Some(f);
                if count == n {
                    return last;
                }
            }
            // Neighbours within range of home that are walkable; the
            // centre itself is not tested.
            for direction in FLOOD_ORDER {
                let Some(q) = self.terrain.neighbour(f, direction) else {
                    continue;
                };
                if grid::distance(animal.home, q) <= range
                    && !self.terrain.has(q, NOT_WALKABLE)
                    && seen.insert(q)
                {
                    queue.push_back(q);
                }
            }
        }
        last
    }

    /// ECO-FOOD-5.11a: the animal's walk to `target`: one step to a
    /// neighbour without a search (5.14: no draw), else the free walk's
    /// search with 10 more per building or site among the field stepped
    /// from and its six neighbours (5.11a (a), for a kind that avoids
    /// buildings), and no second try.
    fn animal_walk(&mut self, from: Point, to: Point, kind: &AnimalKind) -> Option<Vec<Point>> {
        if grid::distance(from, to) == 1 {
            return Some(vec![to]);
        }
        let buildings = &self.buildings;
        let terrain = &self.terrain;
        let holds = |f: Point| buildings[terrain.index(f)].is_some();
        // The field the animal steps from (5.11a (a)).
        let extra = |q: Point| {
            if !kind.avoid_buildings {
                return 0;
            }
            let around = terrain.neighbours(q).into_iter().flatten();
            10 * (std::iter::once(q)
                .chain(around)
                .filter(|&f| holds(f))
                .count() as u32)
        };
        crate::movement::animal_walk(terrain, &mut self.rng, from, to, &extra).map(|r| r.fields)
    }

    /// ECO-FOOD-5.15, after the animals' update: the accumulator gains the
    /// rate; for every 1000 the cursor moves to the next cell (row by row)
    /// and that cell is checked.
    pub(crate) fn update_respawn(&mut self) {
        self.respawn.accumulator += self.respawn.rate;
        let columns = self.terrain.width.div_ceil(RESPAWN_CELL);
        let rows = self.terrain.height.div_ceil(RESPAWN_CELL);
        while self.respawn.accumulator >= 1000 {
            let (mut x, mut y) = self.respawn.cursor;
            x += 1;
            if x >= columns {
                x = 0;
                y += 1;
                if y >= rows {
                    y = 0;
                }
            }
            self.respawn.cursor = (x, y);
            self.respawn.accumulator -= 1000;
            self.respawn_check((x, y));
        }
    }

    /// Whether a tree (a deposit yielding the trees' good, wood) stands on
    /// `field`.
    pub(crate) fn tree_on(&self, field: Point) -> bool {
        let wood = self
            .default_tree
            .and_then(|k| self.deposit_kinds.get(&k))
            .and_then(|k| k.good);
        wood.is_some()
            && self
                .deposits
                .get(&field)
                .and_then(|d| self.deposit_kinds.get(&d.kind))
                .is_some_and(|k| k.good == wood)
    }

    /// The trees on the fields of the square of `side` fields from `corner`.
    fn trees_in(&self, corner: Point, side: u32) -> u32 {
        let (w, h) = (self.terrain.width, self.terrain.height);
        (corner.1..(corner.1 + side).min(h))
            .flat_map(|y| (corner.0..(corner.0 + side).min(w)).map(move |x| (x, y)))
            .filter(|&f| self.tree_on(f))
            .count() as u32
    }

    /// ECO-FOOD-5.15: one respawn check on a cell of the grid.
    fn respawn_check(&mut self, cell: (u32, u32)) {
        let corner = (cell.0 * RESPAWN_CELL, cell.1 * RESPAWN_CELL);
        let in_cell = |f: Point| f.0 / RESPAWN_CELL == cell.0 && f.1 / RESPAWN_CELL == cell.1;
        // 1. The living animals whose home lies in the cell.
        let homes = self
            .animals
            .iter()
            .flatten()
            .filter(|a| in_cell(a.home))
            .count();
        if homes > 4 {
            return;
        }
        // 2. The cell's wood (one per tree, medium in the spec).
        if self.trees_in(corner, RESPAWN_CELL) <= 30 {
            return;
        }
        // 3. Sub-cells, i (x) outer, j (y) inner.
        let mut found = None;
        'cells: for i in 0..RESPAWN_CELL / RESPAWN_SUB {
            for j in 0..RESPAWN_CELL / RESPAWN_SUB {
                let start = (corner.0 + RESPAWN_SUB * i, corner.1 + RESPAWN_SUB * j);
                if !self.terrain.contains(start) || self.trees_in(start, RESPAWN_SUB) <= 12 {
                    continue;
                }
                if let Some(field) = self.respawn_flood(start) {
                    found = Some(field);
                    break 'cells;
                }
            }
        }
        let Some(field) = found else {
            return;
        };
        // 4. The kind, the kinds in ascending id order, then its first
        // wait: two draws.
        let count = self.animal_kinds.len() as u32;
        if count == 0 {
            return;
        }
        let pick = (self.rng.draw() % count) as usize;
        let Some(kind) = self.animal_kinds.values().nth(pick).copied() else {
            return;
        };
        let id = kind.id;
        let speed = f64::from(kind.speed) / 1_000_000.0;
        let idle = f64::from(kind.idle_thousandths.max(2000)) / 1000.0;
        let span = (f64::from(kind.move_distance) / speed * idle).trunc() as u32;
        let wait = self.rng.draw() % span.max(1);
        let mut animal = Animal::new(id, field);
        animal.state = AnimalState::Waiting { left: wait as i32 };
        animal.registered = self.tick + 1;
        // SIM-START-4.2: an animal made takes the next number (ECO-FOOD-5.9:
        // so it updates after every older one).
        self.new_number();
        self.animals.push(Some(animal));
    }

    /// ECO-FOOD-5.15: the flood from a sub-cell's corner over walkable
    /// fields within 8 of it, in the ring order of 5.10 (the start without
    /// a walkable test): the first field that, with each of its six
    /// neighbours, holds a tree.
    fn respawn_flood(&self, start: Point) -> Option<Point> {
        let mut seen = BTreeSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(f) = queue.pop_front() {
            let neighbours = self.terrain.neighbours(f);
            if self.tree_on(f)
                && neighbours
                    .iter()
                    .all(|n| n.is_some_and(|n| self.tree_on(n)))
            {
                return Some(f);
            }
            for direction in FLOOD_ORDER {
                let Some(q) = neighbours[usize::from(direction)] else {
                    continue;
                };
                if grid::distance(start, q) <= RESPAWN_REACH
                    && !self.terrain.has(q, NOT_WALKABLE)
                    && seen.insert(q)
                {
                    queue.push_back(q);
                }
            }
        }
        None
    }

    /// Remove an animal (killed, or gone by other means).
    pub fn remove_animal(&mut self, id: usize) {
        if let Some(animal) = self.animals.get_mut(id) {
            *animal = None;
        }
        self.hunted.retain(|_, &mut a| a != id);
    }

    /// ECO-FOOD-4.2: the fishing spots of the hut on `at`: a flood from its
    /// flag field over walkable fields within the working radius of it; a
    /// field is a candidate if it is a Coast field and the resource layer
    /// holds `fish` on it or within the fishing distance. The flood runs
    /// until the queue is empty.
    pub fn fishing_spots(&self, flag: Point, fish: u32, radius: u32) -> Vec<Point> {
        let holds = |f: Point| {
            self.resource_at(f)
                .is_some_and(|r| r.good == fish && r.amount > 0)
        };
        let candidate = |f: Point| {
            self.terrain.has(f, state::COAST)
                && grid::area(
                    f,
                    self.fishing_distance,
                    self.terrain.width,
                    self.terrain.height,
                )
                .into_iter()
                .any(holds)
        };
        let mut found = Vec::new();
        let mut entered = vec![false; self.terrain.len()];
        let mut queue = VecDeque::from([flag]);
        entered[self.terrain.index(flag)] = true;
        if candidate(flag) {
            found.push(flag);
        }
        while let Some(a) = queue.pop_front() {
            for direction in FLOOD_ORDER {
                let Some(b) = self.terrain.neighbour(a, direction) else {
                    continue;
                };
                let index = self.terrain.index(b);
                if entered[index]
                    || self.terrain.has(b, NOT_WALKABLE)
                    || grid::distance(flag, b) > radius
                {
                    continue;
                }
                entered[index] = true;
                if candidate(b) {
                    found.push(b);
                }
                queue.push_back(b);
            }
        }
        found
    }

    /// ECO-FOOD-4.3, 4.5: the first field holding `fish` in a spiral
    /// around `center` (the field, then its rings up to the fishing
    /// distance).
    pub(crate) fn first_fish_around(&self, center: Point, fish: u32) -> Option<Point> {
        grid::area(
            center,
            self.fishing_distance,
            self.terrain.width,
            self.terrain.height,
        )
        .into_iter()
        .find(|&f| {
            self.resource_at(f)
                .is_some_and(|r| r.good == fish && r.amount > 0)
        })
    }

    /// ECO-PROD-3.3, 4.5: take one unit from the resource on `field`; a
    /// field that reaches 0 loses its good.
    pub(crate) fn take_unit(&mut self, field: Point) {
        let index = self.terrain.index(field);
        let Some(resource) = self.resources.get(index).copied().flatten() else {
            return;
        };
        let left = (resource.amount - 1).min(crate::workshop::ORE_CAP);
        self.resources[index] = (left > 0).then_some(crate::FieldResource {
            amount: left,
            ..resource
        });
    }
}
