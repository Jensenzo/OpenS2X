//! GFX-OBJ-2.2a: the house renderer's slot table. Every building of the map
//! (every player, every type) takes a slot when it is created; its colour
//! column is the slot mod 8 (GFX-OBJ-2.2). The table is the view's: it is
//! not saved, and a loaded game numbers its buildings afresh.

use std::collections::{HashMap, HashSet};

use s2_sim::grid::Point;

/// GFX-OBJ-2.2: the colour map's columns (`numVariations` in the shipped
/// `buildings.xml`).
pub const COLOUR_COLUMNS: u32 = 8;

/// GFX-OBJ-2.2a (3 b), 8.7 (2): a finished building's collapse clock rises
/// by this much per second of the view's frame time; its slot is held, and
/// its house drawn sinking, until the clock reaches 1 (2.857 s).
pub const COLLAPSE_RATE: f32 = 0.35;

/// A building as the slot table sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct House {
    /// Its object number: creation order (SIM-START-4).
    pub serial: u32,
    pub at: Point,
    /// No frame or main work left (a construction site is not).
    pub finished: bool,
    /// GFX-OBJ-8.7: its type and owner, to draw its collapse.
    pub building_type: u32,
    pub owner: s2_sim::PlayerId,
}

/// GFX-OBJ-2.2a: the slots of the buildings the view knows, the stack of
/// free slots and the count of slots ever made.
#[derive(Debug, Clone, Default)]
pub struct HouseSlots {
    known: HashMap<u32, (u32, House)>,
    /// (2): the slot freed last is on top.
    free: Vec<u32>,
    made: u32,
    /// (3 b): the slots held by a finished building's collapse, with its
    /// clock and the building, in the order the collapses began.
    collapsing: Vec<(u32, f32, House)>,
    /// The game tick the buildings were last taken from.
    tick: Option<u64>,
}

impl HouseSlots {
    /// (5): a view built afresh on `world` (a load, or a view started on a
    /// game under way): its buildings take slots 1, 2, 3, ... in creation
    /// order, the free stack empty.
    pub fn of_world(world: &s2_sim::World) -> Self {
        let mut slots = Self::default();
        slots.update(Some((world.tick(), &houses(world))), 0.0);
        slots
    }

    /// One view update, `seconds` of frame time after the last; `now` the
    /// game's tick and its buildings, when they may have changed. (3 b) A
    /// held slot is freed at the first update after its collapse clock
    /// reached 1, before this update's new buildings take theirs. A building
    /// gone since the last update frees its slot: (3 a) at once if it was a
    /// construction site; (4) at once to the finished building made on its
    /// field in the same update (a conquest; an upgrade keeps the building
    /// and so its slot); (3 b) else after its collapse. Then (1, 2) each new
    /// building, in creation order, takes the slot on top of the free stack,
    /// or a new one. Buildings gone or made between two updates are taken in
    /// creation order, those gone first.
    pub fn update(&mut self, now: Option<(u64, &[House])>, seconds: f32) {
        let mut held = Vec::with_capacity(self.collapsing.len());
        for (slot, clock, house) in self.collapsing.drain(..) {
            if clock >= 1.0 {
                self.free.push(slot);
            } else {
                held.push((slot, (clock + COLLAPSE_RATE * seconds).min(1.0), house));
            }
        }
        self.collapsing = held;
        let Some((tick, houses)) = now else { return };
        if self.tick == Some(tick) {
            return;
        }
        self.tick = Some(tick);
        let present: HashSet<u32> = houses.iter().map(|h| h.serial).collect();
        let mut new: Vec<&House> = houses
            .iter()
            .filter(|h| !self.known.contains_key(&h.serial))
            .collect();
        new.sort_by_key(|h| h.serial);
        let finished_new: HashSet<Point> =
            new.iter().filter(|h| h.finished).map(|h| h.at).collect();
        let mut gone: Vec<(u32, u32, House)> = self
            .known
            .iter()
            .filter(|(serial, _)| !present.contains(serial))
            .map(|(&serial, &(slot, house))| (serial, slot, house))
            .collect();
        gone.sort_by_key(|g| g.0);
        let mut handed: HashMap<Point, u32> = HashMap::new();
        for (serial, slot, house) in gone {
            self.known.remove(&serial);
            if !house.finished {
                self.free.push(slot);
            } else if finished_new.contains(&house.at) {
                handed.insert(house.at, slot);
            } else {
                self.collapsing.push((slot, 0.0, house));
            }
        }
        for house in new {
            let slot = handed
                .remove(&house.at)
                .or_else(|| self.free.pop())
                .unwrap_or_else(|| {
                    self.made += 1;
                    self.made
                });
            self.known.insert(house.serial, (slot, *house));
        }
        for house in houses {
            if let Some(entry) = self.known.get_mut(&house.serial) {
                entry.1 = *house;
            }
        }
    }

    /// The slot of the building with object number `serial`.
    pub fn slot(&self, serial: u32) -> Option<u32> {
        self.known.get(&serial).map(|k| k.0)
    }

    /// GFX-OBJ-2.2: the offset of the house's third texture coordinate,
    /// (slot mod 8) / 8; 0 for a building the table does not know.
    pub fn offset(&self, serial: u32) -> f32 {
        self.slot(serial)
            .map_or(0.0, |s| (s % COLOUR_COLUMNS) as f32 / COLOUR_COLUMNS as f32)
    }

    /// The tick the buildings were last taken from.
    pub fn tick(&self) -> Option<u64> {
        self.tick
    }

    /// GFX-OBJ-8.7 (2): the finished buildings collapsing, each with its
    /// slot's colour offset (as [`Self::offset`]) and its collapse clock c,
    /// 0 .. 1.
    pub fn collapsing(&self) -> impl Iterator<Item = (&House, f32, f32)> {
        self.collapsing.iter().map(|(slot, clock, house)| {
            let offset = (slot % COLOUR_COLUMNS) as f32 / COLOUR_COLUMNS as f32;
            (house, offset, *clock)
        })
    }
}

/// The world's buildings as the slot table sees them.
pub fn houses(world: &s2_sim::World) -> Vec<House> {
    world
        .buildings()
        .map(|b| House {
            serial: b.serial,
            at: b.at,
            finished: !b.under_construction,
            building_type: b.building_type,
            owner: b.owner,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T35's player at the table: its buildings as they stand.
    struct Play {
        slots: HouseSlots,
        houses: Vec<House>,
        tick: u64,
        serial: u32,
    }

    impl Play {
        fn new() -> Self {
            Self {
                slots: HouseSlots::default(),
                houses: Vec::new(),
                tick: 0,
                serial: 90_000,
            }
        }
        /// One view update a tenth of a second after the last.
        fn frame(&mut self) {
            self.tick += 1;
            self.slots.update(Some((self.tick, &self.houses)), 0.1);
        }
        fn place(&mut self, at: u32, finished: bool) -> u32 {
            self.serial += 3;
            self.houses.push(House {
                serial: self.serial,
                at: (at, at),
                finished,
                building_type: 1,
                owner: s2_sim::PlayerId(0),
            });
            self.frame();
            self.serial
        }
        fn remove(&mut self, serial: u32) {
            self.houses.retain(|h| h.serial != serial);
            self.frame();
        }
        fn finish(&mut self, serial: u32) {
            for h in &mut self.houses {
                if h.serial == serial {
                    h.finished = true;
                }
            }
            self.frame();
        }
        fn slot(&self, serial: u32) -> u32 {
            self.slots.slot(serial).expect("a slot")
        }
    }

    // GFX-OBJ-2.2a (1) to (5), T35: slots by creation, the free stack, the
    // collapse's hold, a capture keeping its slot, and a load numbering
    // afresh in creation order.
    // spec: GFX-OBJ-2.2a, GFX-OBJ-2.2
    #[test]
    fn t35_slots_by_creation_the_free_stack_and_the_collapse() {
        let mut p = Play::new();
        // (1) HQ 1, A 2, B 3, C 4: columns 1 to 4.
        let hq = p.place(10, true);
        let a = p.place(11, false);
        let b = p.place(12, false);
        let c = p.place(13, false);
        assert_eq!([hq, a, b, c].map(|s| p.slot(s)), [1, 2, 3, 4]);
        assert_eq!(p.slots.offset(hq), 1.0 / 8.0);
        // (2) site B torn down, D takes its slot.
        p.remove(b);
        let d = p.place(14, false);
        assert_eq!(p.slot(d), 3);
        // (3) E a new one.
        let e = p.place(15, false);
        assert_eq!(p.slot(e), 5);
        // (4) C, then A: the stack 4, 2 (2 on top).
        p.remove(c);
        p.remove(a);
        let f = p.place(16, false);
        let g = p.place(17, false);
        let h = p.place(18, false);
        assert_eq!([f, g, h].map(|s| p.slot(s)), [2, 4, 6]);
        // (7) a load after (4): 1 to 6 in creation order HQ, D, E, F, G, H.
        let mut loaded = HouseSlots::default();
        loaded.update(Some((0, &p.houses)), 0.0);
        let order = [hq, d, e, f, g, h].map(|s| loaded.slot(s).expect("a slot"));
        assert_eq!(order, [1, 2, 3, 4, 5, 6]);
        assert_eq!([d, e, f, g].map(|s| p.slot(s)), [3, 5, 2, 4]);
        // (5) E finished, then demolished: J at once takes a new slot, its
        // 5 still held; K 3.0 s later takes 5.
        p.finish(e);
        p.remove(e);
        let j = p.place(19, false);
        assert_eq!(p.slot(j), 7);
        for _ in 0..28 {
            p.frame();
        }
        let k = p.place(20, false);
        assert_eq!(p.slot(k), 5);
        // A site created during the collapse cannot get the held slot.
        assert_eq!(p.slots.made, 7);
        // (3 b) the clock: 1 / 0.35 s of frame time, then the next update.
        let mut q = Play::new();
        let one = q.place(30, true);
        q.remove(one);
        for _ in 0..29 {
            q.frame();
        }
        assert!(q.slots.free.is_empty());
        q.frame();
        assert_eq!(q.slots.free, [1]);
    }

    // GFX-OBJ-2.2a (4): a captured building (the old one removed, one of
    // the same type made finished on its field in the same update) keeps
    // the slot; a finished building that goes with no successor holds it.
    // spec: GFX-OBJ-2.2a
    #[test]
    fn a_captured_building_keeps_its_slot() {
        let mut p = Play::new();
        let hq = p.place(10, true);
        let tower = p.place(11, true);
        let site = p.place(12, false);
        p.houses.retain(|h| h.serial != tower && h.serial != site);
        p.serial += 3;
        let taken = p.serial;
        p.houses.push(House {
            serial: taken,
            at: (11, 11),
            finished: true,
            building_type: 1,
            owner: s2_sim::PlayerId(1),
        });
        p.frame();
        assert_eq!(p.slot(taken), 2);
        // The site's slot 3 went onto the stack at once.
        let next = p.place(13, false);
        assert_eq!(p.slot(next), 3);
        assert_eq!(p.slot(hq), 1);
    }
}
