//! Construction sites: levelling and building (`specs/ECO-warehouse-construction.md`
//! ECO-HQ-4).
//!
//! Leveller and builder walk to the site over the roads (ECO-REQ-3.6). The
//! builder's approach walk and its walks between the four construction
//! positions are timed on the height lattice (TRN-FREE-5.5, ECO-HQ-4.6b);
//! the leveller works in place. Requests and orders follow ECO-REQ-1.2, 1.3
//! and 4.1; the `DebugSupplySite` command can still put materials and
//! workers there.

use std::collections::BTreeSet;

use crate::grid::{self, Point};
use crate::{movement, BuildingSize, BuildingType, PlayerId, Rejection, SettlerJob, Tick, World};

/// ECO-HQ-4.3: levelling progress per step, added to an f32 (TOOL-trace's
/// `levelled`, FMT-SAV's `VillageBulldozing`), so six steps sum to
/// 1.0199999 and the third reads 0.50999999.
const LEVELLING_STEP: f32 = 0.17;
/// ECO-HQ-4.3: levelling is complete at 1.0.
pub(crate) const LEVELLING_DONE: f32 = 1.0;
/// ECO-HQ-4.6: a work unit is one work step at each of the four
/// construction positions.
const STEPS_PER_UNIT: u32 = 4;
/// ECO-HQ-4.6b: lattice steps of the approach walk, north-east.
const APPROACH_STEPS: u32 = 3;
/// FMT-SAV-3.5a (e, Q-1037): a saved constructor's first number at step 13
/// is the approach's leg: this one on the way out, 2 on the way back.
const APPROACH_OUT: u32 = 1;
/// ECO-HQ-4.8: samples within this many lattice steps of the levelled
/// field's sample take the building field's height.
const LEVELLED_SAMPLES: u32 = 2;
/// ECO-HQ-4.8a: a new site's height is the mean of the fields within this
/// distance of it.
const SITE_MEAN_RADIUS: u32 = 2;

/// A leveller at work on a site (ECO-HQ-4.3).
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leveller {
    /// SIM-START-4.2: the settler's object number.
    pub number: u32,
    /// Ticks left of the bulldoze action; 0 when none is running
    /// (ECO-WORK-2.1).
    pub timer: u32,
    /// ECO-HQ-4.3a: his lattice walks from field to field around the site;
    /// he bulldozes standing on the field he walked to.
    pub walker: crate::movement::Walker,
}

/// ECO-HQ-4.3: the site's direction counter starts at direction 2.
const FIRST_LEVEL_DIRECTION: u8 = 2;

/// The builder of a site (ECO-HQ-4.4).
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Builder {
    /// SIM-START-4.2: the settler's object number.
    pub number: u32,
    /// Decrements done in the current work unit (ECO-HQ-4.6a: 221 a unit).
    pub unit_ticks: u32,
    /// True while a work unit is under way: its material has been taken.
    pub in_unit: bool,
    /// Ticks of the current unit so far (TRN-FREE-5.5c).
    pub clock: u32,
    /// The unit's tick at which each of the four work steps starts, after
    /// the walk to its construction position (ECO-HQ-4.6); [`NOT_YET`]
    /// until that walk has ended (ECO-HQ-4.6b step 4, T24).
    pub starts: [u32; 4],
    /// TRN-MOVE-2.5: his walk under way or last walked (an approach leg or
    /// a unit's walk between positions).
    pub walk: movement::LatticeWalk,
    /// The height sample the builder stands on, during a unit the one it
    /// started on; `None` until its first walk, when it stands on the
    /// site's flag (Q-125).
    pub at: Option<Point>,
    /// ECO-HQ-4.6b: where the builder is before a unit.
    pub approach: Approach,
}

/// A unit step whose walk has not ended yet starts at no known tick.
pub const NOT_YET: u32 = u32::MAX;

/// ECO-HQ-4.6b steps 2 and 3: the approach walk and the unit choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approach {
    /// The approach walk is ordered in the next tick.
    Pending,
    /// The out-and-back walk is under way (the builder's `walk`): `clock`
    /// ticks since it was ordered; `back` once the out leg has ended, at
    /// `out_ticks`. The unit choice is made in the tick it ends.
    Walking {
        clock: u32,
        back: bool,
        out_ticks: u32,
    },
    /// The unit choice is made in the next tick (after a unit).
    Choose,
}

/// TRN-FREE-6.4b (T33): a builder's approach walk under way: its start, its
/// far point, the ticks of its two legs and the ticks walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ApproachLeft {
    pub from: Point,
    pub out: Point,
    pub out_ticks: u32,
    pub back_ticks: u32,
    pub clock: u32,
}

impl Builder {
    /// A builder that has just arrived at its site, the settler `number`.
    pub fn arrived(number: u32) -> Self {
        Self {
            number,
            unit_ticks: 0,
            in_unit: false,
            clock: 0,
            starts: [NOT_YET; 4],
            walk: movement::LatticeWalk::standing((0, 0)),
            at: None,
            approach: Approach::Pending,
        }
    }
}

/// Where a builder is during his work unit, for the view (ECO-HQ-4.6,
/// TRN-FREE-5.5): at a construction position doing a step's action, or on
/// a fine walk.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderPose {
    /// At the height sample, hammering (GFX-ANIM-3.8 code 4) since the
    /// action's tick `since` (0 in its first tick).
    Working { at: Point, since: u32 },
    /// On a fine walk (TRN-FREE-5.1, TRN-MOVE-2.5): the lattice step under
    /// way, from the sample last reached to the next (the same sample for a
    /// walk of no step), and the bits of its float32 progress.
    Walking {
        from: Point,
        to: Point,
        progress_bits: u32,
    },
    /// Standing on the sample (between units, waiting for material).
    Standing(Point),
}

impl BuilderPose {
    /// ECO-HQ-4.6b: the sample he last reached; his field is the one it
    /// truncates to (MAP-GRID-4.3).
    pub fn sample(&self) -> Point {
        match *self {
            BuilderPose::Working { at, .. } | BuilderPose::Standing(at) => at,
            BuilderPose::Walking { from, .. } => from,
        }
    }
}

/// ECO-HQ-4.6, 4.6a, 4.7: after how many decrements of remaining work the
/// frame part is done and the building complete, for `frame` and `main`
/// materials and `unit` decrements per work unit (221), each material worth
/// `worth` decrements (220).
///
/// The spec gives the remaining work in 32-bit floating point, and its
/// rounding decides the counts (sawmill 882, not 880 or 881). Only these two
/// whole numbers enter the game state; the f32 values exist here alone, and
/// IEEE subtraction and division give the same result on every platform.
pub fn work_schedule(frame: u32, main: u32, unit: u32, worth: u32) -> (u32, u32) {
    let counted = count_work(frame, main, unit, worth, u32::MAX);
    (counted.frame_end, counted.done)
}

/// ECO-HQ-4.6, 4.7: the remaining work (frame, main), each 1 at the start,
/// after `done` decrements, counted as [`work_schedule`] counts them. For
/// the view only (GFX-OBJ-2.6: the site's pieces and the rising house); the
/// game state keeps the whole decrement count.
pub fn remaining_work(frame: u32, main: u32, unit: u32, worth: u32, done: u32) -> (f32, f32) {
    let counted = count_work(frame, main, unit, worth, done);
    (counted.frame_left, counted.main_left.max(0.0))
}

struct Counted {
    frame_left: f32,
    main_left: f32,
    frame_end: u32,
    done: u32,
}

/// The float32 counting of ECO-HQ-4.6, 4.7, stopped after `limit`
/// decrements or when the building is complete.
fn count_work(frame: u32, main: u32, unit: u32, worth: u32, limit: u32) -> Counted {
    // ECO-HQ-4.6: 1 / n rounded to 32 bits, then divided by the worth and
    // rounded again (T-build-1: one unit in the last place more for 3, 6).
    let amount = |n: u32| (1.0f32 / n as f32) / worth as f32;
    let mut c = Counted {
        frame_left: if frame > 0 { 1.0 } else { 0.0 },
        main_left: 1.0,
        frame_end: 0,
        done: 0,
    };
    loop {
        // The amount is fixed when the unit starts.
        let frame_work = c.frame_left > 0.0;
        if !frame_work && main == 0 {
            return c;
        }
        let step = amount(if frame_work { frame } else { main });
        for _ in 0..unit {
            if c.done >= limit {
                return c;
            }
            c.done += 1;
            if c.frame_left > 0.0 {
                c.frame_left -= step;
                if c.frame_left < 0.0 {
                    c.frame_left = 0.0;
                    c.frame_end = c.done;
                }
            } else {
                c.main_left -= step;
                if c.main_left < 0.0 {
                    return c;
                }
            }
        }
        if c.frame_left <= 0.0 && main == 0 {
            return c;
        }
    }
}

/// ECO-HQ-4.6, 4.6a: a material is worth 4 steps of the construct time (220
/// ticks); a unit does one decrement more (221). Returns the unit and the
/// [`work_schedule`] of the building type's materials.
fn schedule(kind: &BuildingType, construct_ticks: u32) -> (u32, (u32, u32)) {
    let worth = STEPS_PER_UNIT * construct_ticks;
    let unit = worth + 1;
    let counts = (total(&kind.frame_costs), total(&kind.costs));
    (unit, work_schedule(counts.0, counts.1, unit, worth))
}

impl crate::Rules {
    /// ECO-CAT-1.6, 2: the ticks of builder work a building type takes (the
    /// frame done after the first number of decrements, the building after
    /// the second), excluding walking and levelling.
    pub fn construction_work_ticks(&self, building_type: u32) -> Option<(u32, u32)> {
        let kind = self.building_types.iter().find(|t| t.id == building_type)?;
        Some(schedule(kind, self.settings.construct_ticks).1)
    }
}

/// ECO-HQ-4.1: a building under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionSite {
    pub size: BuildingSize,
    /// ECO-HQ-4.2: whether the site had to be levelled.
    pub levelling_needed: bool,
    /// ECO-HQ-4.3: levelling progress, an f32 kept as its bits (0.17 a
    /// step); complete at 1.0. [`ConstructionSite::levelling`] reads it.
    pub levelling_bits: u32,
    /// ECO-HQ-4.3: the site's direction counter, the neighbour the current
    /// levelling step levels: 2, 3, 4, 5, 0, 1.
    pub level_direction: u8,
    pub leveller: Option<Leveller>,
    pub builder: Option<Builder>,
    /// Frame materials and main materials, (good, count).
    pub frame_costs: Vec<(u32, u32)>,
    pub costs: Vec<(u32, u32)>,
    /// Materials delivered so far, used or not.
    pub delivered: Vec<(u32, u32)>,
    /// Materials lying at the site, not used yet.
    pub on_site: Vec<(u32, u32)>,
    /// ECO-HQ-4.6a: decrements of remaining work done so far, one per tick
    /// of work.
    pub work_done: u32,
    /// [`work_schedule`]: the frame is done after `frame_end` decrements,
    /// the building after `work_total`.
    pub frame_end: u32,
    pub work_total: u32,
    /// Decrements of one work unit (221).
    pub unit_ticks: u32,
    /// ECO-REQ-1.2, 1.3: a leveller or builder has been sent and is on its
    /// way.
    pub leveller_on_way: bool,
    pub builder_on_way: bool,
    /// ECO-REQ-4.1: the material orders have been filed.
    pub orders_filed: bool,
}

fn count_of(list: &[(u32, u32)], good: u32) -> u32 {
    list.iter()
        .filter(|&&(g, _)| g == good)
        .map(|&(_, n)| n)
        .sum()
}

pub(crate) fn add(list: &mut Vec<(u32, u32)>, good: u32, count: u32) {
    match list.iter_mut().find(|(g, _)| *g == good) {
        Some((_, n)) => *n += count,
        None => list.push((good, count)),
    }
}

/// Take one of `good` from `list`; false if there is none.
fn take(list: &mut Vec<(u32, u32)>, good: u32) -> bool {
    match list.iter().position(|&(g, n)| g == good && n > 0) {
        Some(i) => {
            list[i].1 -= 1;
            if list[i].1 == 0 {
                list.remove(i);
            }
            true
        }
        None => false,
    }
}

pub(crate) fn total(list: &[(u32, u32)]) -> u32 {
    list.iter().map(|&(_, n)| n).sum()
}

impl ConstructionSite {
    /// A material handed over by a carrier.
    pub(crate) fn receive(&mut self, good: u32) {
        add(&mut self.delivered, good, 1);
        add(&mut self.on_site, good, 1);
    }

    /// A site continued from a save (FMT-SAV-3.4b): `frame`
    /// and `main` are the decrements each part has had, `lying` the stock the
    /// file gives, which is what lies there: what the work has used is not in
    /// it, while [`Self::delivered`] counts it too, or the site would order
    /// it again. The material of a unit under way is used (ECO-HQ-4.6).
    /// FMT-SAV-3.5a (e) "A site in mid-unit always has its builder" (Q-204):
    /// with `running` (its constructor in a running unit, who goes on with
    /// it and sets the exact work) the units counted reach that unit's end;
    /// without, no unit is under way whatever the fraction shows: the work
    /// stays as the file has it and only the finished units have used
    /// their material.
    pub(crate) fn restore_work(
        &mut self,
        frame: u32,
        main: u32,
        lying: &[(u32, u32)],
        plank: Option<u32>,
        running: bool,
    ) {
        let unit = self.unit_ticks.max(1);
        if !running {
            let done = if main > 0 {
                self.frame_end + main
            } else {
                frame
            };
            let units = done / unit;
            let frame_units = units.min(total(&self.frame_costs));
            let main_units = (units - frame_units).min(total(&self.costs));
            self.work_done = done.min(self.work_total);
            self.count_used(frame_units, main_units, lying, plank);
            return;
        }
        let main_ticks = self.work_total.saturating_sub(self.frame_end);
        let main_units = main.div_ceil(unit).min(total(&self.costs));
        // The main part starts when the frame part has ended (ECO-HQ-4.7).
        let frame_units = if main > 0 {
            total(&self.frame_costs)
        } else {
            frame.div_ceil(unit).min(total(&self.frame_costs))
        };
        let frame_work = if main > 0 {
            self.frame_end
        } else {
            (frame_units * unit).min(self.frame_end)
        };
        self.work_done = frame_work + (main_units * unit).min(main_ticks);
        self.count_used(frame_units, main_units, lying, plank);
    }

    /// FMT-SAV-3.4b: the stock the file gives lies on the site; the
    /// material of `frame_units` and `main_units` is used and counts as
    /// delivered.
    fn count_used(
        &mut self,
        frame_units: u32,
        main_units: u32,
        lying: &[(u32, u32)],
        plank: Option<u32>,
    ) {
        self.on_site = lying.to_vec();
        let mut used: Vec<(u32, u32)> = Vec::new();
        let mut left = frame_units;
        for &(good, n) in &self.frame_costs {
            let k = left.min(n);
            if k > 0 {
                add(&mut used, good, k);
                left -= k;
            }
        }
        // ECO-HQ-4.5: a plank if one was on site, otherwise a stone; which
        // were used is not in the file, only what is not lying there can be.
        let mut goods: Vec<u32> = self.costs.iter().map(|&(good, _)| good).collect();
        goods.sort_by_key(|&good| Some(good) != plank);
        let mut left = main_units;
        for good in goods {
            let room = self
                .still_needed_beside(good, lying)
                .saturating_sub(count_of(&used, good));
            let k = left.min(room);
            if k > 0 {
                add(&mut used, good, k);
                left -= k;
            }
        }
        self.delivered = lying.to_vec();
        for (good, n) in used {
            add(&mut self.delivered, good, n);
        }
    }

    /// What of `good` the site's costs hold beyond `lying`.
    fn still_needed_beside(&self, good: u32, lying: &[(u32, u32)]) -> u32 {
        let needed = count_of(&self.frame_costs, good) + count_of(&self.costs, good);
        needed.saturating_sub(count_of(lying, good))
    }

    /// ECO-HQ-4.3: the levelling progress, 0 to 1 and a little more.
    pub fn levelling(&self) -> f32 {
        f32::from_bits(self.levelling_bits)
    }

    pub fn levelled(&self) -> bool {
        self.levelling() >= LEVELLING_DONE
    }

    /// Work needed for the frame part and the main part, in ticks.
    /// ECO-HQ-4.7: the frame part is worked off.
    pub fn frame_done(&self) -> bool {
        self.work_done >= self.frame_end
    }

    /// GFX-OBJ-2.6: the frame progress f and the main progress m, 0 to 1:
    /// 1 - the remaining work of each part after the decrements done
    /// (ECO-HQ-4.6a, float32). For the view only.
    pub fn progress(&self) -> (f32, f32) {
        let (frame, main) = self.remaining();
        (1.0 - frame, 1.0 - main)
    }

    /// FMT-SAV-3.4b, 3.5a (e): the decrements a site of an original's save
    /// has had, from the remaining work of its two parts the file holds:
    /// the first count whose float32 counting (ECO-HQ-4.6, 4.7) comes
    /// nearest to them (exactly, for the original's own counting).
    pub(crate) fn decrements_for(&self, frame_left: f32, main_left: f32) -> u32 {
        let worth = self.unit_ticks.saturating_sub(1).max(1);
        let (frame, main) = (total(&self.frame_costs), total(&self.costs));
        let frame_left = if frame > 0 { frame_left.max(0.0) } else { 0.0 };
        let main_left = main_left.max(0.0);
        let mut best = (f32::INFINITY, 0);
        for done in 0..=self.work_total {
            let (f, m) = remaining_work(frame, main, self.unit_ticks, worth, done);
            let off = (f - frame_left).abs() + (m - main_left).abs();
            if off < best.0 {
                best = (off, done);
                if off == 0.0 {
                    break;
                }
            }
        }
        best.1
    }

    /// ECO-HQ-4.6, 4.7: the remaining work of the frame part and the main
    /// part, 1 to 0, after the decrements done (float32).
    pub fn remaining(&self) -> (f32, f32) {
        let worth = self.unit_ticks.saturating_sub(1).max(1);
        remaining_work(
            total(&self.frame_costs),
            total(&self.costs),
            self.unit_ticks,
            worth,
            self.work_done,
        )
    }

    /// How much of `good` the site still needs delivered.
    pub fn still_needed(&self, good: u32) -> u32 {
        let needed = count_of(&self.frame_costs, good) + count_of(&self.costs, good);
        needed.saturating_sub(count_of(&self.delivered, good))
    }

    /// Every material still to be delivered, (good, count), frame first.
    pub fn shortfall(&self) -> Vec<(u32, u32)> {
        let mut goods: Vec<u32> = Vec::new();
        for &(good, _) in self.frame_costs.iter().chain(&self.costs) {
            if !goods.contains(&good) {
                goods.push(good);
            }
        }
        goods
            .into_iter()
            .map(|good| (good, self.still_needed(good)))
            .filter(|&(_, n)| n > 0)
            .collect()
    }

    /// Progress of the whole site in thousandths: levelling, then frame and
    /// main work, each weighted by its share of the time the rules give
    /// (6 steps of bulldoze time per levelling, one tick per decrement).
    pub fn progress_permille(&self, bulldoze_ticks: u32) -> u32 {
        let levelling_total = if self.levelling_needed {
            6 * bulldoze_ticks
        } else {
            0
        };
        let levelling_done = (levelling_total as f32 * self.levelling().min(LEVELLING_DONE)) as u32;
        let all = levelling_total + self.work_total;
        if all == 0 {
            return 1000;
        }
        let done = levelling_done + self.work_done;
        (u64::from(done) * 1000 / u64::from(all)) as u32
    }

    /// The material for the next work unit (ECO-HQ-4.5): during the frame
    /// part a frame material, during the main part a plank if one is on site,
    /// otherwise a stone.
    fn next_material(&self, plank: Option<u32>, stone: Option<u32>) -> Option<u32> {
        let on_site = |good: u32| count_of(&self.on_site, good) > 0;
        if !self.frame_done() {
            return self
                .frame_costs
                .iter()
                .map(|&(good, _)| good)
                .find(|&good| on_site(good));
        }
        [plank, stone].into_iter().flatten().find(|&g| on_site(g))
    }
}

impl World {
    /// ECO-HQ-4.2: levelling is needed for medium, large and harbour
    /// buildings whose field differs in height from one of its neighbours.
    pub(crate) fn needs_levelling(&self, at: Point, size: BuildingSize) -> bool {
        let levelled_size = matches!(
            size,
            BuildingSize::Medium | BuildingSize::Large | BuildingSize::Harbor
        );
        let height = self.terrain.height_at(at);
        levelled_size
            && self
                .terrain
                .neighbours(at)
                .into_iter()
                .flatten()
                .any(|n| self.terrain.height_at(n) != height)
    }

    /// ECO-HQ-4.1: the site of the building just placed on `at`.
    pub(crate) fn start_construction(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building on the site");
        let kind = self
            .building_type(building.building_type)
            .expect("a known type")
            .clone();
        if kind.instant {
            // ECO-REQ-4.1: construction time 0 or less.
            self.complete(at);
            return;
        }
        let site = self.new_site(at, &kind, building.size);
        let levelled = site.levelling_needed;
        self.sites.insert(at, site);
        if !levelled {
            self.clear_decorations(at);
        }
    }

    /// ECO-HQ-4.1: the construction of a building of type `kind` and size
    /// class `size` on `at`.
    pub(crate) fn new_site(
        &self,
        at: Point,
        kind: &BuildingType,
        size: BuildingSize,
    ) -> ConstructionSite {
        let levelling_needed = self.needs_levelling(at, size);
        let (unit, (frame_end, work_total)) = schedule(kind, self.settings.construct_ticks);
        ConstructionSite {
            size,
            levelling_needed,
            // ECO-HQ-4.2a: levelling 0 at creation, whoever places it; the
            // next building update levels a site that needs none.
            levelling_bits: 0.0f32.to_bits(),
            level_direction: FIRST_LEVEL_DIRECTION,
            leveller: None,
            builder: None,
            frame_costs: kind.frame_costs.clone(),
            costs: kind.costs.clone(),
            delivered: Vec::new(),
            on_site: Vec::new(),
            work_done: 0,
            frame_end,
            work_total,
            unit_ticks: unit,
            leveller_on_way: false,
            builder_on_way: false,
            orders_filed: false,
        }
    }

    /// The construction site on `at`.
    pub fn site(&self, at: Point) -> Option<&ConstructionSite> {
        self.sites.get(&at)
    }

    /// ECO-PLACE-5.3 (T16, T17): the fields whose map decorations a site, or
    /// a start castle in the first update, has removed from the world. They
    /// stay removed when the site is torn down before it is finished.
    pub fn decorations_cleared(&self) -> &BTreeSet<Point> {
        &self.decorations_cleared
    }

    /// ECO-PLACE-5.3 (T16): of the site's or building's footprint fields on
    /// `at` and its flag field, those whose map decorations are gone (all
    /// of them once its site, or as a start castle the first update, has
    /// cleared them; none while a levelled site's leveller is still on his
    /// way, before the first update or where nothing stands).
    pub fn decoration_cleared_fields(&self, at: Point) -> Vec<Point> {
        let size = match (self.site(at), self.building_at(at)) {
            (Some(site), _) => site.size,
            (None, Some(b)) => b.size,
            (None, None) => return Vec::new(),
        };
        self.decoration_fields(at, size)
            .into_iter()
            .filter(|f| self.decorations_cleared.contains(f))
            .collect()
    }

    /// ECO-PLACE-5.3: a site's footprint fields and its flag field.
    fn decoration_fields(&self, at: Point, size: BuildingSize) -> Vec<Point> {
        let mut fields = self.footprint_of(at, size);
        if let Some(flag) = self.terrain.neighbour(at, 1) {
            fields.push(flag);
        }
        fields
    }

    /// ECO-PLACE-5.3 (T16): the site on `at` removes the map decorations of
    /// its footprint and flag fields, for good: with its placement when it
    /// needs no levelling, else as its leveller arrives.
    pub(crate) fn clear_decorations(&mut self, at: Point) {
        let Some(size) = self.sites.get(&at).map(|s| s.size) else {
            return;
        };
        let fields = self.decoration_fields(at, size);
        self.decorations_cleared.extend(fields);
    }

    /// ECO-PLACE-5.3 (T17, Q-295): in the first update the buildings placed
    /// finished at the start (the castles) remove the map decorations of
    /// their footprint and flag fields, as a site does.
    pub(crate) fn clear_start_decorations(&mut self) {
        let start: Vec<(Point, BuildingSize)> = self.buildings().map(|b| (b.at, b.size)).collect();
        for (at, size) in start {
            let fields = self.decoration_fields(at, size);
            self.decorations_cleared.extend(fields);
        }
    }

    /// ECO-HQ-4.6, TRN-FREE-5.5: the pose of the builder of the site on
    /// `at` in this tick. A unit's four steps each work at their own
    /// construction position for `construct_ticks` ticks; between them he
    /// walks from one position to the next (the walk to the first starts
    /// where the previous unit ended, the flag field's sample before the
    /// first). Every walk, the approach's too, goes step by step along the
    /// straight lattice line (TRN-FREE-5.1, ECO-HQ-4.6b).
    pub fn builder_pose(&self, at: Point) -> Option<BuilderPose> {
        let site = self.sites.get(&at)?;
        let builder = site.builder?;
        let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
        let here = builder.at.unwrap_or_else(|| grid::field_sample(flag));
        if !builder.in_unit {
            // ECO-HQ-4.6b step 2 (GFX-OBJ-2.6d): waiting for material he
            // walks out and back; TRN-MOVE-2.5: where his walk is.
            if let Approach::Walking { .. } = builder.approach {
                if let (from, Some(to), progress) = builder.walk.position(&self.terrain) {
                    return Some(BuilderPose::Walking {
                        from,
                        to,
                        progress_bits: progress.to_bits(),
                    });
                }
            }
            return Some(BuilderPose::Standing(here));
        }
        let samples = self.construction_samples(at);
        let action = self.settings.construct_ticks;
        // `clock` counts the unit's ticks already run; this tick's is the
        // one before it.
        let tick = builder.clock.saturating_sub(1);
        // ECO-HQ-4.6b steps 3 to 5: he stands on the sample he chose the
        // unit on in that tick; the walk to position 0 starts from there in
        // the next.
        if tick < 1 {
            return Some(BuilderPose::Standing(here));
        }
        for (i, &start) in builder.starts.iter().enumerate() {
            if tick < start {
                // TRN-MOVE-2.5: on his walk to position i, as walked.
                let (from, to, progress) = builder.walk.position(&self.terrain);
                return Some(BuilderPose::Walking {
                    from,
                    to: to.unwrap_or(from),
                    progress_bits: progress.to_bits(),
                });
            }
            if tick < start.saturating_add(action) {
                return Some(BuilderPose::Working {
                    at: samples[i],
                    since: tick - start,
                });
            }
        }
        Some(BuilderPose::Standing(samples[3]))
    }

    /// All construction sites, by building field.
    pub fn sites(&self) -> impl Iterator<Item = (Point, &ConstructionSite)> {
        self.sites.iter().map(|(&at, site)| (at, site))
    }

    /// Bulldoze time in ticks (ECO-HQ-4.3).
    pub fn bulldoze_ticks(&self) -> u32 {
        self.settings.bulldoze_ticks
    }

    /// ECO-HQ-4.8: a levelling step gives the neighbour field `field` the
    /// height `height` of the building field: every sample within 2 lattice
    /// steps of its sample takes that height, then each sample of the
    /// lattice ring 3 around it becomes the integer average (truncated
    /// toward zero) of the six samples around its direction-1 neighbour, in
    /// ring order, using values already changed. Water, Coast and ElevationBlocked are not
    /// recomputed. Returns the fields whose placement is recomputed: those
    /// within 3 of `field` (through [`World::changed`]'s radius 2).
    /// TRN-MOVE-2.1a (T14): a height near `field` is about to change: every
    /// settler's and carrier's walk ordered and not yet stepping keeps the
    /// heights of its first step as they are now (those at its order: none
    /// changed since, or they were kept then).
    fn keep_first_step_heights(&mut self, field: Point) {
        let terrain = &self.terrain;
        let keep = |w: &mut crate::movement::Walker| {
            if w.first_heights.is_some() {
                return;
            }
            if let Some((a, b)) = w.first_step() {
                if grid::distance(a, field) <= 2 || grid::distance(b, field) <= 2 {
                    w.first_heights = Some((terrain.height_at(a), terrain.height_at(b)));
                }
            }
        };
        for s in self.settlers.iter_mut().flatten() {
            keep(&mut s.walker);
        }
        for c in &mut self.carriers {
            keep(&mut c.walker);
        }
    }

    pub(crate) fn set_field_height(&mut self, field: Point, height: i32) -> Vec<Point> {
        self.keep_first_step_heights(field);
        let center = grid::field_sample(field);
        let (columns, rows) = self.terrain.sample_size();
        for sample in grid::area(center, LEVELLED_SAMPLES, columns, rows) {
            self.set_sample_height(sample, height);
        }
        if !self.terrain.samples.is_empty() {
            for sample in grid::ring(center, LEVELLED_SAMPLES + 1, columns, rows) {
                // Q-271: the six samples around the one a step from P in
                // direction 1, reached from P by one step in each direction
                // 0 .. 5 in turn (the last is P), those inside the map.
                let mut at = (sample.0 as i32, sample.1 as i32);
                let (mut sum, mut count) = (0i64, 0i64);
                for dir in grid::Direction::ALL {
                    at = grid::step(at.0, at.1, dir);
                    if let (Ok(c), Ok(r)) = (u32::try_from(at.0), u32::try_from(at.1)) {
                        if c < columns && r < rows {
                            sum += i64::from(self.terrain.sample_height((c, r)));
                            count += 1;
                        }
                    }
                }
                let mean = sum / count.max(1);
                self.set_sample_height(sample, mean as i32);
            }
        }
        let (width, map_height) = (self.terrain.width, self.terrain.height);
        grid::area(field, 1, width, map_height)
    }

    /// ECO-HQ-4.8b: a flag made on `flag` gives the six height samples one
    /// lattice step around its field's sample that sample's height; its own
    /// sample and the samples farther out are not touched, nor the field
    /// states derived from height. Returns the fields whose placement is
    /// recomputed.
    pub(crate) fn flag_height_ring(&mut self, flag: Point) -> Vec<Point> {
        if self.terrain.samples.is_empty() {
            return Vec::new();
        }
        let center = grid::field_sample(flag);
        let height = self.terrain.sample_height(center);
        let (columns, rows) = self.terrain.sample_size();
        for sample in grid::neighbours(center, columns, rows)
            .into_iter()
            .flatten()
        {
            self.set_sample_height(sample, height);
        }
        let (width, map_height) = (self.terrain.width, self.terrain.height);
        grid::area(flag, 1, width, map_height)
    }

    /// ECO-HQ-4.8a: when a building or a site appears on `at` and its six
    /// neighbours are not all at its height, its field's sample takes the
    /// integer mean of the heights of the 19 fields within 2 of it (a height
    /// below the map's floor counts as the floor; the sum divided by the
    /// number of those fields inside the map, toward zero), spread as a
    /// levelling step spreads a height (4.8). Returns the fields whose
    /// placement is recomputed (none when nothing changed).
    pub(crate) fn level_site_height(&mut self, at: Point) -> Vec<Point> {
        let height = self.terrain.height_at(at);
        let uneven = self
            .terrain
            .neighbours(at)
            .into_iter()
            .flatten()
            .any(|n| self.terrain.height_at(n) != height);
        if !uneven {
            return Vec::new();
        }
        let (width, map_height) = (self.terrain.width, self.terrain.height);
        let fields = grid::area(at, SITE_MEAN_RADIUS, width, map_height);
        let floor = self.terrain.height_floor;
        let sum: i64 = fields
            .iter()
            .map(|&f| i64::from(self.terrain.height_at(f).max(floor)))
            .sum();
        let mean = sum / fields.len().max(1) as i64;
        self.set_field_height(at, mean as i32)
    }

    /// One tick of the construction site on `at`: its requests and orders
    /// (ECO-REQ-1.2, 1.3, 4.1). Its builder and its leveller work in the
    /// settlers' part, in the constructors' and the levellers' lists
    /// (ECO-REQ-0.2, [`World::update_settler_lists`]).
    pub(crate) fn update_site(&mut self, at: Point) {
        let Some(site) = self.sites.get(&at) else {
            return;
        };
        let building = *self.building_at(at).expect("a site's building");
        let target = self.target_of(at).expect("a site's building");
        let requester = crate::Requester::Site(target);
        if !site.levelled() && !site.levelling_needed {
            // ECO-HQ-4.2a: levelled (progress 1) by the first building update,
            // which goes on to the frame part.
            self.sites.get_mut(&at).expect("a site").levelling_bits = LEVELLING_DONE.to_bits();
            // MIL-UPG-2.2a (T9): an upgrade's update does one part: this one
            // marks the levelling; the builder request is the next. The
            // orders' tick (T10): its material orders are filed in this one.
            if self.upgrades.contains_key(&at) {
                self.file_site_orders(at);
                return;
            }
        }
        let site = &self.sites[&at];
        if !site.levelled() {
            // 1.2
            let wanted = site.leveller.is_none()
                && !site.leveller_on_way
                && self.needs_levelling(at, site.size);
            if let (true, Some(leveller)) = (wanted, self.roles.leveller) {
                self.request(building.owner, requester, leveller);
            }
            return;
        }
        // 4.1: every material unit once, as soon as levelling is complete.
        self.file_site_orders(at);
        // 1.3
        let site = &self.sites[&at];
        if site.builder.is_none() && !site.builder_on_way {
            if let Some(builder) = self.roles.builder {
                self.request(building.owner, requester, builder);
            }
        }
    }

    /// ECO-REQ-4.1: the site on `at` orders every material unit once, as
    /// soon as levelling is complete: in the leveller's last step (T29),
    /// else in the site's first update.
    pub(crate) fn file_site_orders(&mut self, at: Point) {
        let Some(site) = self.sites.get(&at) else {
            return;
        };
        if site.orders_filed {
            return;
        }
        let mut needed: Vec<(u32, u32)> = site.shortfall();
        let units: Vec<(u32, u32)> = site
            .frame_costs
            .iter()
            .chain(&site.costs)
            .copied()
            .collect();
        self.sites.get_mut(&at).expect("a site").orders_filed = true;
        for (good, count) in units {
            let left = needed.iter_mut().find(|(g, _)| *g == good);
            let Some((_, left)) = left else { continue };
            let n = count.min(*left);
            *left -= n;
            for _ in 0..n {
                self.order(at, good);
            }
        }
    }

    /// ECO-HQ-4.3a: the leveller's approach has ended on `arrival` (the
    /// site's flag): he walks to the neighbour the site's direction counter
    /// names (direction 2 for a new site) and bulldozes it first; nothing is
    /// drawn.
    pub(crate) fn leveller_arrives(&mut self, at: Point, arrival: Point, number: u32) {
        let first = self.sites[&at].level_direction;
        let target = self.terrain.neighbour(at, first).unwrap_or(arrival);
        let mut walker = crate::movement::Walker::new(arrival);
        walker.walk(self.leveller_path(arrival, target));
        let site = self.sites.get_mut(&at).expect("a site");
        site.leveller = Some(Leveller {
            number,
            timer: 0,
            walker,
        });
        // ECO-PLACE-5.3: before the first levelling step.
        self.clear_decorations(at);
    }

    /// ECO-HQ-4.3a (Q-247): the leveller's walk from one field to the next
    /// is the settlers' free walk: one step to a neighbour, else the free
    /// walk's route with its draws (TRN-FREE-3.2a).
    fn leveller_path(&mut self, from: Point, to: Point) -> Vec<Point> {
        self.free_walk(from, to, 0).unwrap_or_else(|| vec![to])
    }

    /// ECO-HQ-4.3, 4.3a, ECO-WORK-2.2: the leveller walks to his field (a
    /// lattice walk, the slope factor applies), bulldozes there for the
    /// bulldoze time, then the site levels the neighbour of its direction
    /// counter and the counter moves on; six steps, then he goes home.
    /// ECO-REQ-0.2 (ECO-HQ T25): in the levellers' list, after the
    /// constructors' and the carriers', so a constructor or carrier starting
    /// a step in the tick of a levelling step sees the heights before it.
    /// The site sees the completed levelling, and files its builder request
    /// and orders, in its next update (ECO-REQ-1.3).
    pub(crate) fn level(&mut self, at: Point) {
        let bulldoze = self.settings.bulldoze_ticks;
        let speed = self.settings.settler_speed;
        let Some(mut leveller) = self.sites.get_mut(&at).and_then(|s| s.leveller.take()) else {
            return;
        };
        if leveller.walker.is_walking() {
            leveller.walker.advance(&self.terrain, speed, |_| true);
            if !leveller.walker.is_walking() {
                leveller.timer = bulldoze;
            }
            self.sites.get_mut(&at).expect("a site").leveller = Some(leveller);
            return;
        }
        if leveller.timer == 0 {
            // On his field without a walk: the action starts.
            leveller.timer = bulldoze;
            self.sites.get_mut(&at).expect("a site").leveller = Some(leveller);
            return;
        }
        leveller.timer -= 1;
        if leveller.timer > 0 {
            self.sites.get_mut(&at).expect("a site").leveller = Some(leveller);
            return;
        }
        // The action is over: the counter's neighbour takes the site's
        // height and is no longer marked; the counter moves on.
        let site = self.sites.get_mut(&at).expect("a site");
        let direction = site.level_direction;
        site.levelling_bits = (site.levelling() + LEVELLING_STEP).to_bits();
        site.level_direction = (direction + 1) % 6;
        let next = site.level_direction;
        let done = site.levelled();
        if let Some(neighbour) = self.terrain.neighbour(at, direction) {
            let height = self.terrain.height_at(at);
            let touched = self.set_field_height(neighbour, height);
            self.changed(&touched);
        }
        if done {
            // The leveller goes home (ECO-HQ-4.3) after the going-home steps
            // (TRN-FREE-6.4a: standing in this tick and the next two).
            let owner = self.building_at(at).expect("a site's building").owner;
            self.send_home_after_steps(
                owner,
                self.roles.leveller,
                leveller.walker.at,
                None,
                leveller.number,
            );
            // ECO-REQ-4.1 (T29): the site files its material orders in this
            // last step, so the matching of this tick sees them; the builder
            // request comes in its next update (ECO-REQ-1.3).
            self.file_site_orders(at);
            return;
        }
        // He walks to the counter's neighbour, the walk timed by the heights
        // after the change.
        let here = leveller.walker.at;
        let target = self.terrain.neighbour(at, next).unwrap_or(here);
        leveller.walker.walk(self.leveller_path(here, target));
        self.sites.get_mut(&at).expect("a site").leveller = Some(leveller);
    }

    /// ECO-HQ-4.5 to 4.7, 4.6b: one tick of the builder. Arriving in tick
    /// `S`, he orders the approach walk (3 lattice steps north-east and
    /// back to the flag field's sample) in `S + 1`; in the tick it ends the
    /// unit is chosen: its material is taken and the walk to position 0 is
    /// ordered in the next tick. In each of the 221 work ticks of a unit
    /// the remaining work falls once (ECO-HQ-4.6a); the next unit is chosen
    /// in the tick after the previous one ended. ECO-REQ-0.2: in the
    /// constructors' list of the settlers' part.
    pub(crate) fn build(&mut self, at: Point) {
        let (plank, stone) = (self.plank, self.stone);
        let Some(snapshot) = self.sites.get(&at).and_then(|s| s.builder) else {
            return;
        };
        if !self.sites[&at].levelled() {
            return;
        }
        let mut builder = snapshot;
        let speed = self.settings.settler_speed;
        if !builder.in_unit {
            // Step 2: the approach, out and back; TRN-MOVE-2.5 (T14): each
            // step's rate from the heights when it starts.
            match builder.approach {
                Approach::Pending => {
                    let (here, out, flag_sample) = self.approach_points(at, builder.at);
                    let walk = movement::LatticeWalk::new(&self.terrain, speed, here, out);
                    let back = walk.is_done(&self.terrain);
                    builder.walk = if back {
                        movement::LatticeWalk::new(&self.terrain, speed, out, flag_sample)
                    } else {
                        walk
                    };
                    builder.approach = Approach::Walking {
                        clock: 0,
                        back,
                        out_ticks: 0,
                    };
                    self.sites.get_mut(&at).expect("a site").builder = Some(builder);
                    return;
                }
                Approach::Walking {
                    clock,
                    back,
                    out_ticks,
                } => {
                    let clock = clock + 1;
                    builder.walk.advance(&self.terrain, speed);
                    let (mut back, mut out_ticks) = (back, out_ticks);
                    if !back && builder.walk.is_done(&self.terrain) {
                        // The out leg has ended: the walk back is ordered,
                        // from where it ended (its far point; a save's walk
                        // out, Q-1037: its target) to the flag field's
                        // sample.
                        let (_, _, flag_sample) = self.approach_points(at, builder.at);
                        let out = builder.walk.to;
                        builder.walk =
                            movement::LatticeWalk::new(&self.terrain, speed, out, flag_sample);
                        (back, out_ticks) = (true, clock);
                    }
                    if !(back && builder.walk.is_done(&self.terrain)) {
                        builder.approach = Approach::Walking {
                            clock,
                            back,
                            out_ticks,
                        };
                        self.sites.get_mut(&at).expect("a site").builder = Some(builder);
                        return;
                    }
                    // Done: he stands at the walk's end, the flag field's
                    // sample (a save's approach, FMT-SAV-3.5a e: its target).
                    builder.at = Some(builder.walk.to);
                    builder.approach = Approach::Choose;
                }
                Approach::Choose => {}
            }
            // Step 3: the unit choice.
            let site = self.sites.get_mut(&at).expect("a site");
            if site.work_done >= site.work_total {
                self.finish_building(at);
                return;
            }
            // ECO-HQ-4.5: without a suitable material the approach starts
            // again in the next tick.
            let Some(good) = site.next_material(plank, stone) else {
                builder.approach = Approach::Pending;
                site.builder = Some(builder);
                return;
            };
            take(&mut site.on_site, good);
            builder.in_unit = true;
            // The material is built in (ECO-HQ-4.5).
            self.used(good, 1);
            builder.clock = 0;
            builder.starts = [NOT_YET; 4];
            builder.approach = Approach::Choose;
        }
        // Step 4, ECO-HQ-4.6a: the remaining work falls in each tick of a
        // step's action and once more in the tick after the fourth.
        let samples = self.construction_samples(at);
        let action = self.settings.construct_ticks;
        let tick = builder.clock;
        builder.clock += 1;
        // TRN-FREE-5.5, ECO-HQ-4.6b step 4 (T24): the walk to position i is
        // ordered in the tick after the unit was chosen, each later one in
        // the tick after the action before; its steps' rates are those of
        // the heights when each starts (TRN-MOVE-2.5); the action starts in
        // the tick the walk ends, one tick after the order for a walk with
        // no step (Q-125).
        if let Some(i) = builder.starts.iter().position(|&s| s == NOT_YET) {
            let ordered = if i == 0 {
                1
            } else {
                builder.starts[i - 1] + action
            };
            if tick == ordered {
                let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
                let from = match i {
                    0 => builder.at.unwrap_or_else(|| grid::field_sample(flag)),
                    _ => samples[i - 1],
                };
                builder.walk = movement::LatticeWalk::new(&self.terrain, speed, from, samples[i]);
                if builder.walk.is_done(&self.terrain) {
                    builder.starts[i] = tick + 1;
                }
            } else if tick > ordered {
                builder.walk.advance(&self.terrain, speed);
                if builder.walk.is_done(&self.terrain) {
                    builder.starts[i] = tick;
                }
            }
        }
        let site = self.sites.get_mut(&at).expect("a site");
        let last = builder.starts[3].saturating_add(action);
        let works = tick == last
            || builder
                .starts
                .iter()
                .any(|&s| tick >= s && tick < s.saturating_add(action));
        if works {
            site.work_done += 1;
        }
        if tick >= last {
            // The unit ends on position 3.
            builder.in_unit = false;
            builder.at = Some(samples[3]);
        }
        site.builder = Some(builder);
        // ECO-HQ-4.7: complete at once, in this tick; the builder keeps
        // working to the end of his step's action (the extra decrement after
        // the fourth step has none).
        if site.work_done >= site.work_total {
            let ends = builder
                .starts
                .iter()
                .find(|&&s| tick >= s && tick < s.saturating_add(action))
                .map_or(tick, |&s| s + action);
            self.finish_building_after(at, Tick::from(ends - tick));
        }
    }

    /// FMT-SAV-3.5a (e, Q-1033 answered 2026-11-08 c): the builder of a
    /// save's site in a running unit (`step` 10, ECO-HQ-4.6b step 4) with
    /// `ordered` (c) construction positions ordered so far and `left` (r)
    /// ticks left of the current step's action: he is at, or walking to,
    /// position c - 1. r = the action's length: walking there; 0 < r below
    /// it: working there, r decrements left (the first in the first tick
    /// after the load), then the walk to position c (after position 3 the
    /// extra decrement and the unit's end, 4.6a); r = 0: the step is over,
    /// the next update orders that walk or ends the unit. The unit's material
    /// is taken. The earlier steps' start ticks are made up (as if each walk
    /// between positions took one tick), and a walk to a position starts from
    /// the one before (its progress is not read, Q-257 e). `None` for c = 0
    /// (no walk ordered yet) or past 4, or r above the action's length.
    pub(crate) fn resumed_unit(
        &self,
        at: Point,
        number: u32,
        ordered: u32,
        left: u32,
    ) -> Option<Builder> {
        let action = self.settings.construct_ticks;
        if !(1..=4).contains(&ordered) || left > action || !self.sites.get(&at)?.levelled() {
            return None;
        }
        let p = ordered as usize - 1;
        // r = 0 counts as the action fully done: the next update orders the
        // walk on (or makes the extra decrement after position 3).
        let done = action - left;
        let samples = self.construction_samples(at);
        let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
        let flag_sample = grid::field_sample(flag);
        let mut builder = Builder::arrived(number);
        builder.in_unit = true;
        builder.approach = Approach::Choose;
        builder.at = Some(flag_sample);
        let start = |i: usize| 2 + i as u32 * (action + 1);
        if left < action {
            for i in 0..=p {
                builder.starts[i] = start(i);
            }
            builder.clock = start(p) + done;
            builder.walk = movement::LatticeWalk::standing(samples[p]);
        } else {
            for i in 0..p {
                builder.starts[i] = start(i);
            }
            let order_tick = if p == 0 { 1 } else { start(p - 1) + action };
            builder.clock = order_tick + 1;
            let from = if p == 0 { flag_sample } else { samples[p - 1] };
            builder.walk = movement::LatticeWalk::new(
                &self.terrain,
                self.settings.settler_speed,
                from,
                samples[p],
            );
        }
        Some(builder)
    }

    /// FMT-SAV-3.5a (e, 2026-11-09, Q-257): the site's builder on a walk on
    /// the fine lattice goes on with it from the saved point and progress,
    /// the running step at its saved rate (TRN-MOVE-2.5). Before a unit
    /// (step 13, the approach) `first` is the approach's leg (Q-1037): on
    /// the way back (2) the walk ends at its target, the flag field's sample,
    /// where he then chooses his first unit in the same tick (ECO-HQ-4.6b
    /// step 3); on the way out (1) its end orders the walk back to the flag
    /// field's sample (step 2), and he chooses when that ends; in a
    /// unit, walking to a construction position (step 10, r = 55), it is the
    /// walk [`Self::resumed_unit`] ordered, to the same position from the
    /// same start (to position 0: from where he stood before the unit, on
    /// position 3 after an earlier one). False when the saved walk is not
    /// the straight lattice line from its start to its target (TRN-FREE-5.1),
    /// or not that unit's walk.
    pub(crate) fn resume_builder_walk(
        &mut self,
        at: Point,
        saved: &crate::RestoredFineWalk,
        first: u32,
    ) -> bool {
        let Some(mut builder) = self.sites.get(&at).and_then(|s| s.builder) else {
            return false;
        };
        let Some((start, done)) = saved.start_and_done() else {
            return false;
        };
        let mut walk = movement::LatticeWalk::standing(start);
        walk.to = saved.path[0];
        let steps = walk.steps(&self.terrain);
        if steps != saved.line() {
            return false;
        }
        walk.done = done;
        walk.progress_bits = saved.progress.clamp(0.0, 1.0).to_bits();
        let rate = if saved.rate > 0.0 && saved.rate.is_finite() {
            saved.rate
        } else {
            let from = if done == 0 {
                start
            } else {
                steps[done as usize - 1]
            };
            movement::lattice_rate(
                &self.terrain,
                self.settings.settler_speed,
                from,
                steps[done as usize],
            )
        };
        walk.rate_bits = rate.to_bits();
        if builder.in_unit {
            // ECO-HQ-4.6b steps 4, 5: the walk to position 0 starts where he
            // stood before the unit (after a unit, on position 3).
            let running = builder.walk;
            let first = builder.starts[0] == NOT_YET;
            let same = running.to == walk.to && (running.from == start || first);
            if !same || running.is_done(&self.terrain) {
                return false;
            }
            if first {
                builder.at = Some(start);
            }
        } else {
            builder.approach = Approach::Walking {
                clock: 0,
                back: first != APPROACH_OUT,
                out_ticks: 0,
            };
        }
        builder.walk = walk;
        self.sites.get_mut(&at).expect("a site").builder = Some(builder);
        true
    }

    /// ECO-HQ-4.6b step 2: ticks of the approach walk from the sample the
    /// builder stands on (the flag field's sample before his first walk) to
    /// the point 3 lattice steps in direction 5 from the sample of the field
    /// he stands on, and back to the flag field's sample, as the heights
    /// are now.
    #[cfg(test)]
    fn builder_approach_ticks(&self, at: Point, from: Option<Point>) -> u32 {
        let legs = self.builder_approach(at, from);
        legs[0].2 + legs[1].2
    }

    /// TRN-FREE-6.4b (T34, Q-286): the site's builder's work walk from one
    /// construction position to the next, if one is under way (ECO-HQ-4.6b
    /// step 4): the walk with the steps it has walked, its progress and the
    /// rate of the step in progress.
    pub(crate) fn work_walk_left(&self, at: Point) -> Option<movement::LatticeWalk> {
        let builder = self.sites.get(&at)?.builder?;
        if !builder.in_unit {
            return None;
        }
        match self.builder_pose(at)? {
            BuilderPose::Walking { from, to, .. } if from != to => Some(builder.walk),
            _ => None,
        }
    }

    /// TRN-FREE-6.4b (T33): the site's builder's approach walk, if one is
    /// under way (ECO-HQ-4.6b step 2).
    pub(crate) fn approach_left(&self, at: Point) -> Option<ApproachLeft> {
        let builder = self.sites.get(&at)?.builder?;
        if builder.in_unit {
            return None;
        }
        let Approach::Walking {
            clock,
            back,
            out_ticks,
        } = builder.approach
        else {
            return None;
        };
        // TRN-MOVE-2.5: the ticks left as the heights are now.
        let speed = self.settings.settler_speed;
        let left = builder.walk.ticks_left(&self.terrain, speed);
        let (from, out, flag_sample) = self.approach_points(at, builder.at);
        let (out_ticks, back_ticks) = if back {
            (out_ticks, clock - out_ticks + left)
        } else {
            let steps = movement::fine_approach(&self.terrain, out, flag_sample, 0);
            let back_ticks = movement::lattice_walk_ticks(&self.terrain, speed, out, &steps);
            (clock + left, back_ticks)
        };
        (clock < out_ticks + back_ticks).then_some(ApproachLeft {
            from,
            out,
            out_ticks,
            back_ticks,
            clock,
        })
    }

    /// ECO-HQ-4.6b step 2: the approach walk's start (the sample the
    /// builder stands on, the flag field's sample before his first walk),
    /// its far point 3 lattice steps in direction 5 from the sample of the
    /// field he stands on, and its end, the flag field's sample.
    fn approach_points(&self, at: Point, from: Option<Point>) -> (Point, Point, Point) {
        let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
        let flag_sample = grid::field_sample(flag);
        let here = from.unwrap_or(flag_sample);
        let (columns, rows) = self.terrain.sample_size();
        let mut out = grid::field_sample(grid::sample_field_truncating(here));
        let north_east = grid::Direction::new(5).expect("direction 5");
        for _ in 0..APPROACH_STEPS {
            out = grid::neighbour(out, north_east, columns, rows).unwrap_or(out);
        }
        (here, out, flag_sample)
    }

    /// ECO-HQ-4.6b step 2: the approach walk's two legs, (from, to, ticks):
    /// out to the point 3 lattice steps in direction 5 and back to the flag
    /// field's sample.
    #[cfg(test)]
    fn builder_approach(&self, at: Point, from: Option<Point>) -> [(Point, Point, u32); 2] {
        let (here, out, flag_sample) = self.approach_points(at, from);
        let speed = self.settings.settler_speed;
        let walk = |a: Point, b: Point| {
            let steps = movement::fine_approach(&self.terrain, a, b, 0);
            movement::lattice_walk_ticks(&self.terrain, speed, a, &steps)
        };
        [
            (here, out, walk(here, out)),
            (out, flag_sample, walk(out, flag_sample)),
        ]
    }

    /// TRN-FREE-5.5a, 5.5b: the four construction positions (height
    /// samples) of the building on `at`: the sample of its flag field plus
    /// the owner's tribe's offset for each index, no odd-row correction.
    pub fn construction_samples(&self, at: Point) -> [Point; 4] {
        let building = self.building_at(at).expect("a building");
        let tribe = self.tribe(building.owner);
        // MIL-UPG-2.3a (T7): an upgrade's builder works at the positions of
        // the type the building keeps until completion, not the target's.
        let offsets = self
            .building_types
            .get(&building.building_type)
            .map_or([(0, 0); 4], |t| {
                t.construction_positions[usize::from(tribe.min(2))]
            });
        let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
        let (c, r) = grid::field_sample(flag);
        let (columns, rows) = self.terrain.sample_size();
        offsets.map(|(x, y)| {
            (
                (i64::from(c) + i64::from(x)).clamp(0, i64::from(columns) - 1) as u32,
                (i64::from(r) + i64::from(y)).clamp(0, i64::from(rows) - 1) as u32,
            )
        })
    }

    /// ECO-HQ-4.7: the builder is released and goes home, and the building
    /// is finished.
    fn finish_building(&mut self, at: Point) {
        self.finish_building_after(at, 0);
    }

    /// ECO-HQ-4.7: the building is finished; the builder goes home as from
    /// the tick `delay` ticks after this one (the tick after his step's
    /// action, which he works to its end).
    fn finish_building_after(&mut self, at: Point, delay: Tick) {
        // TRN-FREE-6.4a (T24): from the sample he stands on, back to the
        // point of the site's flag, then home over the roads.
        let stand = self.builder_pose(at).map(|pose| pose.sample());
        let site = self.sites.remove(&at).expect("a site");
        let owner = self.building_at(at).expect("a site's building").owner;
        if let Some(builder) = site.builder {
            let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
            let builder_role = self.roles.builder;
            let id = self.send_home_after_steps(owner, builder_role, flag, stand, builder.number);
            if let Some(Some(s)) = id.map(|id| &mut self.settlers[id]) {
                if let SettlerJob::Leaving { at: begun, .. } = &mut s.job {
                    *begun += delay;
                }
            }
            // MIL-UPG-1.2, 2.4 (T8): linked to the construction until his
            // release.
            self.builder_links.insert(at, self.tick + delay);
        }
        if self.upgrades.contains_key(&at) {
            // MIL-UPG-2.4 (T8): the type changes in the building's update
            // once no builder is linked (`update_buildings`), at once when
            // none is.
            if !self.builder_links.contains_key(&at) {
                let to = self.upgrades.remove(&at).expect("an upgrade");
                self.change_building_type(at, to);
            }
        } else {
            self.complete(at);
            // UI-GAME-11.1: "Building completed (<name>)", misc.
            self.message(owner, at, "Building constructed");
        }
    }

    /// A building becomes finished: a warehouse type starts its stock
    /// (ECO-REQ-3.2), and a building without a worker works at once.
    pub(crate) fn complete(&mut self, at: Point) {
        let index = self.terrain.index(at);
        let building = self.buildings[index].as_mut().expect("a building");
        building.under_construction = false;
        building.timer = 0;
        let kind = &self.building_types[&building.building_type];
        if kind.worker.is_none() || kind.worker_limit == 0 {
            building.active = true;
        }
        let harbour = kind.size == crate::BuildingSize::Harbor;
        if kind.warehouse {
            self.warehouses.entry(at).or_default();
        }
        self.register_military(at);
        if harbour {
            // SHIP-1.2
            self.make_harbour(at);
        }
    }

    /// The `DebugSupplySite` command (debug only, see [`crate::Command`]).
    pub(crate) fn debug_supply_site(
        &mut self,
        player: PlayerId,
        at: Point,
        wares: &[(u32, u32)],
        workers: bool,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        self.building_at(at)
            .filter(|b| b.owner == player)
            .ok_or(Rejection::NoBuilding)?;
        let site = self
            .sites
            .get_mut(&at)
            .ok_or(Rejection::NotAConstructionSite)?;
        let mut asked: Vec<(u32, u32)> = Vec::new();
        for &(good, count) in wares {
            add(&mut asked, good, count);
        }
        if asked
            .iter()
            .any(|&(good, count)| count > site.still_needed(good))
        {
            return Err(Rejection::WareNotNeeded);
        }
        for &(good, count) in &asked {
            add(&mut site.delivered, good, count);
            add(&mut site.on_site, good, count);
        }
        // What came this way is no longer ordered.
        for &(good, count) in &asked {
            self.cancel_orders(at, good, count);
            self.made(Some(good), count);
        }
        let site = self.sites.get_mut(&at).expect("a site");
        if workers {
            if site.levelling_needed && !site.levelled() && site.leveller.is_none() {
                let flag = self.terrain.neighbour(at, 1).unwrap_or(at);
                // SIM-START-4.2: a settler made.
                let number = self.new_number();
                self.leveller_arrives(at, flag, number);
            }
            if self.sites[&at].builder.is_none() {
                let number = self.new_number();
                let site = self.sites.get_mut(&at).expect("a site");
                site.builder = Some(Builder::arrived(number));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{BuildingType, Command, Rules, Tick};

    const SAWMILL: u32 = 20;
    const HUT: u32 = 21;
    const HUT_2: u32 = 22;
    const PLANK: u32 = 7;
    const STONE: u32 = 8;

    fn rules() -> Rules {
        let mut sawmill = BuildingType::new(SAWMILL, BuildingSize::Medium);
        sawmill.frame_costs = vec![(PLANK, 2)];
        sawmill.costs = vec![(STONE, 2)];
        let mut hut = BuildingType::new(HUT, BuildingSize::Small);
        hut.costs = vec![(PLANK, 2)];
        let mut hut_2 = BuildingType::new(HUT_2, BuildingSize::Small);
        hut_2.frame_costs = vec![(PLANK, 1)];
        hut_2.costs = vec![(PLANK, 1)];
        Rules {
            building_types: vec![sawmill, hut, hut_2],
            plank: Some(PLANK),
            stone: Some(STONE),
            ..Rules::empty()
        }
    }

    fn site_world(terrain: crate::map::Terrain) -> World {
        World::with_map(5, terrain, rules(), P0)
    }

    fn supply(world: &mut World, at: Point, wares: Vec<(u32, u32)>, workers: bool) -> Tick {
        let tick = world.tick();
        let outcome = apply(
            world,
            Command::DebugSupplySite {
                player: P0,
                at,
                wares,
                workers,
            },
        );
        assert_eq!(outcome, Ok(()));
        tick
    }

    /// Step until `done` holds; the tick in which it first did.
    fn run_until(world: &mut World, done: impl Fn(&World) -> bool) -> Tick {
        for _ in 0..10_000 {
            if done(world) {
                return world.tick() - 1;
            }
            world.step();
        }
        panic!("never done");
    }

    /// A sawmill site on a field raised to 300 in flat land: placed, the
    /// site's field takes the mean of the 19 fields within 2, 300 / 19 = 15
    /// (ECO-HQ-4.8a), and its neighbours still differ.
    fn uneven_sawmill_site() -> (World, Point) {
        let at = (7, 7);
        let mut terrain = flat(16, 16);
        set_height(&mut terrain, at, 300);
        let mut world = site_world(terrain);
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        assert_eq!(world.terrain().height_at(at), SITE_HEIGHT);
        (world, at)
    }

    /// The height of [`uneven_sawmill_site`]'s field once placed.
    const SITE_HEIGHT: i32 = 300 / 19;

    // ECO-PLACE T16 (ECO-PLACE-5.3): a site that needs no levelling clears
    // the decorations of its footprint and flag field with its placement; a
    // levelled one only once its leveller has arrived; a building keeps
    // them cleared.
    // spec: ECO-PLACE-5.3 (ECO-PLACE T16)
    #[test]
    fn t16_sites_clear_their_decorations() {
        let at = (7, 7);
        let flag = flat(16, 16).neighbour(at, 1).unwrap();
        let mut world = site_world(flat(16, 16));
        assert_eq!(world.decoration_cleared_fields(at), Vec::<Point>::new());
        assert_eq!(building(&mut world, HUT, at), Ok(()));
        assert!(!world.site(at).unwrap().levelling_needed);
        assert_eq!(world.decoration_cleared_fields(at), [at, flag]);
        supply(&mut world, at, vec![(PLANK, 2)], true);
        run_until(&mut world, |w| {
            w.building_at(at).is_some_and(|b| !b.under_construction)
        });
        assert_eq!(world.decoration_cleared_fields(at), [at, flag]);

        let (mut world, at) = uneven_sawmill_site();
        assert!(world.site(at).unwrap().levelling_needed);
        assert_eq!(world.decoration_cleared_fields(at), Vec::<Point>::new());
        assert!(world.decorations_cleared().is_empty());
        supply(&mut world, at, vec![], true);
        run_until(&mut world, |w| w.site(at).unwrap().leveller.is_some());
        let flag = world.terrain().neighbour(at, 1).unwrap();
        let cleared = world.decoration_cleared_fields(at);
        assert_eq!(cleared, [at, flag]);
        assert_eq!(
            world
                .decorations_cleared()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            {
                let mut sorted = cleared.clone();
                sorted.sort();
                sorted
            }
        );
    }

    // ECO-PLACE T16 (ECO-PLACE-5.3): the decorations are removed from the
    // world, so a site torn down before it is finished does not bring them
    // back (not measured: the spec's "removed from the world").
    // spec: ECO-PLACE-5.3 (ECO-PLACE T16)
    #[test]
    fn t16_a_removed_site_leaves_its_decorations_gone() {
        let at = (7, 7);
        let flag = flat(16, 16).neighbour(at, 1).unwrap();
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, HUT, at), Ok(()));
        assert!(world.site(at).is_some());
        assert_eq!(world.remove_building(P0, at), Ok(()));
        assert!(world.site(at).is_none() && world.building_at(at).is_none());
        assert_eq!(
            world
                .decorations_cleared()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            {
                let mut both = vec![at, flag];
                both.sort();
                both
            }
        );
        // A snapshot keeps them.
        let copy = World::from_snapshot(&world.snapshot()).unwrap();
        assert_eq!(copy.decorations_cleared(), world.decorations_cleared());
    }

    // UI-GAME-11.1, T33: a finished building gives its owner the misc
    // message "Building constructed" at its field, naming its type.
    #[test]
    fn finished_building_message() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, HUT, at), Ok(()));
        supply(&mut world, at, vec![(PLANK, 2)], true);
        run_until(&mut world, |w| {
            w.building_at(at).is_some_and(|b| !b.under_construction)
        });
        let done: Vec<_> = world
            .messages()
            .iter()
            .filter(|m| m.text == "Building constructed")
            .collect();
        assert_eq!(done.len(), 1);
        assert_eq!((done[0].player, done[0].at), (P0, at));
        assert_eq!(done[0].building, Some(HUT));
    }

    // ECO-HQ T6: 6 steps of 55 ticks of work, with a walk of one field (23
    // ticks on the level) before each (ECO-HQ-4.3a: the first from the flag
    // to the direction-2 neighbour); then every neighbour has the site's
    // height. The supply command comes before the buildings' update, so the
    // first walk advances in the command's tick already.
    // spec: ECO-HQ-4.2, ECO-HQ-4.3, ECO-HQ-4.3a (T6), ECO-HQ-4.8a
    #[test]
    fn t6_levelling() {
        let (mut world, at) = uneven_sawmill_site();
        let site = world.site(at).unwrap();
        assert!(!site.levelled());
        let start = supply(&mut world, at, vec![], true);
        assert!(world.site(at).unwrap().leveller.is_some());
        let mut heights_set = 0;
        let levelled = run_until(&mut world, |w| {
            let site = w.site(at).unwrap();
            site.levelled()
        });
        assert_eq!(levelled - start, 6 * (55 + 23) - 1);
        for n in world.terrain().neighbours(at).into_iter().flatten() {
            assert_eq!(world.terrain().height_at(n), SITE_HEIGHT, "{n:?}");
            heights_set += 1;
        }
        assert_eq!(heights_set, 6);
        assert!(world.site(at).unwrap().leveller.is_none());
        // ECO-PLACE-4.2: the kept classes follow the height changes.
        let mut full = world.clone();
        full.recompute_all_placement();
        assert_eq!(world.placement, full.placement);
    }

    // ECO-HQ-4.3, 4.3a (T21, T22, T-level-2 to T-level-4): arriving on the
    // site's flag the leveller walks to the site's direction-2 neighbour
    // (the flag field's west neighbour) whatever the generator's state, and
    // nothing is drawn, then or later; the site levels its neighbours in the
    // order 2, 3, 4, 5, 0, 1; between the steps he walks one field (23 ticks
    // on the level) and bulldozes 55.
    // spec: ECO-HQ-4.3, ECO-HQ-4.3a (T21, T22, T-level-2, T-level-3, T-level-4)
    #[test]
    fn levelling_draws_nothing_and_follows_the_counter() {
        for burnt in [0, 1, 2, 3, 4, 5] {
            let (mut world, at) = uneven_sawmill_site();
            for _ in 0..burnt {
                world.rng.draw();
            }
            let neighbours = world.terrain().neighbours(at).map(|n| n.unwrap());
            let draws = world.rng.draws();
            supply(&mut world, at, vec![], true);
            assert_eq!(world.rng.draws(), draws, "T-level-4");
            let leveller = world.site(at).unwrap().leveller.clone().unwrap();
            assert_eq!(leveller.walker.at, neighbours[1], "on the flag");
            assert_eq!(leveller.walker.destination(), neighbours[2], "T-level-3");
            // The flag field's west neighbour.
            assert_eq!(
                world.terrain().neighbour(neighbours[1], 3),
                Some(neighbours[2])
            );
            let mut levelled = Vec::new();
            let mut ticks = Vec::new();
            while levelled.len() < 6 {
                world.step();
                for (d, n) in neighbours.iter().enumerate() {
                    if world.terrain().height_at(*n) == SITE_HEIGHT && !levelled.contains(&d) {
                        levelled.push(d);
                        ticks.push(world.tick());
                    }
                }
                assert!(world.tick() < 2000);
            }
            assert_eq!(levelled, [2, 3, 4, 5, 0, 1]);
            assert!(world.site(at).unwrap().levelled());
            assert!(world.site(at).unwrap().leveller.is_none());
            // Neighbour to neighbour: a one-field walk and the bulldoze time.
            for pair in ticks.windows(2) {
                assert_eq!(pair[1] - pair[0], 23 + 55, "{ticks:?}");
            }
            // No draw in the whole levelling (T22).
            assert_eq!(world.rng.draws(), draws, "T22");
        }
    }

    // ECO-HQ-4.3 (T21): each step adds 0.17 to the site's levelling
    // progress, an f32 (TOOL-trace's `levelled`, FMT-SAV's
    // `VillageBulldozing`): the third step reads 0.50999999, just below
    // 0.51, so the trace row is 509 (the original's rows of trace ticks 732
    // to 809), and the sixth, 1.0199999, completes it.
    // spec: ECO-HQ-4.3 (T21, T-level-2)
    #[test]
    fn levelling_progress_is_a_float32_sum() {
        let (mut world, at) = uneven_sawmill_site();
        supply(&mut world, at, vec![], true);
        let mut rows = Vec::new();
        let mut progress = vec![world.site(at).unwrap().levelling()];
        while !world.site(at).unwrap().levelled() {
            world.step();
            let p = world.site(at).unwrap().levelling();
            if p != *progress.last().unwrap() {
                progress.push(p);
                let row = world.trace_buildings().into_iter().find(|r| r.field == at);
                rows.push(row.map(|r| (r.state, r.progress)).unwrap());
            }
            assert!(world.tick() < 2000);
        }
        let mut sum = 0.0f32;
        let sums: Vec<f32> = std::iter::once(0.0)
            .chain((0..6).map(|_| {
                sum += 0.17f32;
                sum
            }))
            .collect();
        assert_eq!(progress, sums);
        assert!(f64::from(sums[3]) < 0.51, "below 0.51");
        let levelling = |p| ("levelling", p);
        assert_eq!(rows[..5], [170, 340, 509, 680, 850].map(levelling), "T21");
        assert_eq!(rows[5].0, "frame");
    }

    // ECO-HQ T7: a small building needs no levelling.
    // spec: ECO-HQ-4.2 (T7)
    #[test]
    fn t7_small_buildings_need_no_levelling() {
        let at = (7, 7);
        let mut terrain = flat(16, 16);
        set_height(&mut terrain, at, 300);
        let mut world = site_world(terrain);
        assert_eq!(building(&mut world, HUT, at), Ok(()));
        assert!(world.site(at).unwrap().levelled());
        supply(&mut world, at, vec![], true);
        assert!(world.site(at).unwrap().leveller.is_none());
    }

    // T-level-1: a site is created with levelling 0, whoever places it; the
    // building update of the tick a command placed it in levels it (a
    // human's command runs before the buildings: frame at that tick's end).
    // spec: ECO-HQ-4.2a (T-level-1)
    #[test]
    fn level_t1_levelled_by_the_next_building_update() {
        let world = site_world(flat(16, 16));
        let kind = world.building_types[&HUT].clone();
        let site = world.new_site((7, 7), &kind, BuildingSize::Small);
        assert_eq!(site.levelling(), 0.0);
        assert!(!site.levelled());
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, HUT, (7, 7)), Ok(()));
        assert!(world.site((7, 7)).unwrap().levelled());
    }

    // ECO-HQ-4.2: a medium building on even ground needs none either.
    #[test]
    fn even_ground_needs_no_levelling() {
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, (7, 7)), Ok(()));
        assert!(world.site((7, 7)).unwrap().levelled());
    }

    // ECO-HQ T8: frame 2 x 220, then main 2 x 220; finished after 882
    // decrements, with the approach walk and the one-tick steps of
    // ECO-HQ-4.6b.
    #[test]
    fn t8_building_time() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        let start = supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], true);
        // ECO-HQ-4.6a, 4.7: the frame ends at unit 2's 220th decrement, the
        // building at unit 4's 219th: 882 decrements.
        let site = world.site(at).unwrap();
        assert_eq!((site.frame_end, site.work_total), (441, 882));
        // ECO-HQ-4.6b on flat ground with all four positions on the flag's
        // sample: the approach (34 + 34) ends in start + 68, where unit 1 is
        // chosen; a unit is 225 ticks (walks of 0 ticks cost one tick), the
        // next is chosen one tick after the previous ended (a unit every 226).
        // The frame's last decrement is the 220th of unit 2 (the fourth
        // step's last tick): start + 68 + 226 + 224.
        let frame = run_until(&mut world, |w| w.site(at).unwrap().frame_done());
        assert_eq!(frame - start, 68 + 226 + 224);
        // The 882nd decrement is unit 4's 219th: its third step, 53 ticks in.
        let done = run_until(&mut world, |w| w.site(at).is_none());
        assert_eq!(done - start, 68 + 3 * 226 + 170 + 53);
        assert!(!world.building_at(at).unwrap().under_construction);
    }

    // ECO-HQ-4.6: in a unit the builder works four actions of 55 ticks,
    // each at its construction position and starting at tick 0 of its
    // action, walking in between.
    #[test]
    fn builder_pose_follows_the_unit() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        supply(&mut world, at, vec![(PLANK, 2)], true);
        run_until(&mut world, |w| w.site(at).unwrap().builder.unwrap().in_unit);
        let (mut working, mut starts, mut walking) = (0, 0, 0);
        while world.site(at).unwrap().builder.unwrap().in_unit {
            match world.builder_pose(at).unwrap() {
                BuilderPose::Working { since, .. } => {
                    working += 1;
                    starts += u32::from(since == 0);
                }
                BuilderPose::Walking { progress_bits, .. } => {
                    assert!(f32::from_bits(progress_bits) <= 1.0);
                    walking += 1;
                }
                BuilderPose::Standing(_) => {}
            }
            world.step();
        }
        assert_eq!((working, starts), (4 * 55, 4));
        assert!(walking >= 4);
        assert!(matches!(
            world.builder_pose(at),
            Some(BuilderPose::Standing(_))
        ));
    }

    // ECO-HQ-4.6b steps 3 and 4 (E-trace-005, builder 90202 at the hunter's
    // site, trace ticks 842 and 843): in the tick the first unit is chosen
    // he stands on the flag field's sample; in the next the walk to position
    // 0 starts there at progress 0. The next unit starts from position 3.
    // spec: ECO-HQ-4.6b, TRN-FREE-5.5
    #[test]
    fn the_first_units_walk_starts_on_the_flag_sample() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        let kind = world.building_types.get_mut(&SAWMILL).unwrap();
        kind.construction_positions = [[(-1, 2), (0, 0), (1, 1), (-1, 3)]; 3];
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        supply(&mut world, at, vec![(PLANK, 2)], true);
        run_until(&mut world, |w| w.site(at).unwrap().builder.unwrap().in_unit);
        let flag = grid::field_sample(world.terrain.neighbour(at, 1).unwrap());
        let samples = world.construction_samples(at);
        assert_ne!(flag, samples[3]);
        assert_eq!(world.builder_pose(at), Some(BuilderPose::Standing(flag)));
        world.step();
        match world.builder_pose(at).unwrap() {
            BuilderPose::Walking {
                from,
                progress_bits,
                ..
            } => assert_eq!((from, f32::from_bits(progress_bits)), (flag, 0.0)),
            pose => panic!("{pose:?}"),
        }
        // The second unit is chosen on position 3, standing there.
        run_until(&mut world, |w| {
            !w.site(at).unwrap().builder.unwrap().in_unit
        });
        run_until(&mut world, |w| w.site(at).unwrap().builder.unwrap().in_unit);
        assert_eq!(
            world.builder_pose(at),
            Some(BuilderPose::Standing(samples[3]))
        );
    }

    // ECO-HQ T24 (ECO-HQ-4.6b step 4, TRN-MOVE-2.5): a unit's walks take
    // their steps' rates from the heights when each starts, not when the
    // unit starts: a rise put under position 2 during the action at
    // position 1 slows the walk there.
    // spec: ECO-HQ-4.6b (ECO-HQ T24), TRN-MOVE-2.5
    #[test]
    fn t24_a_units_walk_takes_the_heights_when_it_starts() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        let kind = world.building_types.get_mut(&SAWMILL).unwrap();
        kind.construction_positions = [[(-1, 2), (0, 0), (4, 4), (-1, 3)]; 3];
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        supply(&mut world, at, vec![(PLANK, 2)], true);
        let builder = |w: &World| w.site(at).unwrap().builder.unwrap();
        run_until(&mut world, |w| builder(w).in_unit);
        run_until(&mut world, |w| builder(w).starts[1] != NOT_YET);
        let samples = world.construction_samples(at);
        let (speed, action) = (world.settings.settler_speed, world.settings.construct_ticks);
        let steps = movement::fine_approach(&world.terrain, samples[1], samples[2], 0);
        let level = movement::lattice_walk_ticks(&world.terrain, speed, samples[1], &steps);
        // The rise, during the action at position 1.
        set_height(
            &mut world.terrain,
            grid::sample_field_truncating(samples[2]),
            900,
        );
        let risen = movement::lattice_walk_ticks(&world.terrain, speed, samples[1], &steps);
        assert!(risen > level, "{risen} {level}");
        run_until(&mut world, |w| builder(w).starts[2] != NOT_YET);
        let b = builder(&world);
        assert_eq!(b.starts[2] - (b.starts[1] + action), risen);
    }

    // GFX-OBJ T3b: the frame and main progress of the Roman woodcutter
    // (1 + 1 planks) after k decrements, float32: just below one half at
    // 110, the frame done at 221, main 0.504545 at 332.
    // spec: ECO-HQ-4.7, GFX-OBJ-2.6 (GFX-OBJ T3b)
    #[test]
    fn t3b_progress() {
        let left = |k| remaining_work(1, 1, 221, 220, k);
        assert_eq!(left(0), (1.0, 1.0));
        let (frame, main) = left(110);
        assert!(frame > 0.5 && frame < 0.5 + 1e-6, "{frame}");
        assert_eq!(main, 1.0);
        assert_eq!(left(221), (0.0, 1.0));
        let (frame, main) = left(332);
        assert_eq!(frame, 0.0);
        assert!((1.0 - main - 0.504545).abs() < 1e-5, "{main}");
        assert_eq!(left(442), (0.0, 0.0));
    }

    // GFX-OBJ-2.6d, ECO-HQ-4.6b step 2: a builder waiting for material is
    // drawn on his walk out (3 lattice steps north-east) and back to the
    // flag field's sample, not standing; the walk goes one lattice step at a
    // time (TRN-FREE-5.1), each step's float32 progress growing by 0.09 a
    // tick on the level with the excess carried (TRN-MOVE-2.5: 12 ticks to
    // the first sample, 0.08 into the second), and his field stays the flag
    // field (E-trace-002 trace ticks 291 to 359).
    // spec: ECO-HQ-4.6b, TRN-FREE-5.1, TRN-MOVE-2.5
    #[test]
    fn waiting_builder_walks_out_and_back() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, HUT_2, at), Ok(()));
        supply(&mut world, at, vec![], true);
        let flag_field = world.terrain().neighbour(at, 1).unwrap();
        let flag = grid::field_sample(flag_field);
        let (columns, rows) = world.terrain().sample_size();
        let north_east = grid::Direction::new(5).unwrap();
        let mut line = vec![flag];
        for _ in 0..3 {
            let last = *line.last().unwrap();
            line.push(grid::neighbour(last, north_east, columns, rows).unwrap());
        }
        let rate = 2.0 * 0.045f32;
        let mut steps = Vec::new();
        let mut walking = Vec::new();
        for _ in 0..140 {
            let pose = world.builder_pose(at).unwrap();
            let view = world
                .settler_views()
                .into_iter()
                .find(|v| v.id == crate::view::SettlerRef::Builder(at))
                .unwrap();
            assert_eq!(view.walk.at, flag_field, "{pose:?}");
            if let BuilderPose::Walking {
                from,
                to,
                progress_bits,
            } = pose
            {
                assert!(f32::from_bits(progress_bits) <= 1.0);
                if steps.last() != Some(&(from, to)) {
                    steps.push((from, to));
                }
                walking.push((from, to, progress_bits));
            }
            world.step();
        }
        let out: Vec<_> = line.windows(2).map(|w| (w[0], w[1])).collect();
        let back: Vec<_> = line.windows(2).rev().map(|w| (w[1], w[0])).collect();
        assert_eq!(steps[..3], out[..], "{steps:?}");
        assert_eq!(steps[3..6], back[..], "{steps:?}");
        // The walk's first tick at 0, then 0.09 a tick in float32.
        let mut progress = 0.0f32;
        for (k, &(from, to, bits)) in walking[..12].iter().enumerate() {
            assert_eq!((from, to), out[0], "tick {k}");
            assert_eq!(bits, progress.to_bits(), "tick {k}");
            progress += rate;
        }
        assert_eq!(walking[12].2, (progress - 1.0).to_bits());
        assert_eq!((walking[12].0, walking[12].1), out[1]);
        // Out in 34 ticks (TRN-FREE-5.3), the walk back from its end at 0.
        assert_eq!((walking[33].0, walking[33].1), out[2]);
        assert_eq!(walking[34], (line[3], line[2], 0));
    }

    // ECO-HQ T9: with only the planks the frame is built, then the builder
    // waits until a stone comes.
    // spec: ECO-HQ-4.5, ECO-HQ-4.7 (T9)
    #[test]
    fn t9_waits_for_material() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        let start = supply(&mut world, at, vec![(PLANK, 2)], true);
        for _ in 0..2000 {
            world.step();
        }
        // Two whole units of 221: the frame done, one decrement into the
        // main part, then waiting for a stone.
        let site = world.site(at).unwrap();
        assert_eq!(site.work_done, 442);
        assert!(site.frame_done());
        assert!(!site.builder.unwrap().in_unit);
        let more = supply(&mut world, at, vec![(STONE, 2)], false);
        // ECO-HQ-4.6b: without material the unit choice fails in start + 68
        // + 226 + 225 + 1, then every 69 ticks (the approach is ordered one
        // tick after a failed choice and takes 68). A unit starts in the
        // first choice tick from `more` on and, as unit 3 of the sawmill's
        // 882 decrements, ends them 449 ticks after it was chosen.
        let first_choice = start + 68 + 226 + 226;
        let wait = (69 - (more - first_choice) % 69) % 69;
        let done = run_until(&mut world, |w| w.site(at).is_none());
        assert_eq!(done - more, wait + 449);
        assert!(more - start > 880);
    }

    /// The tick (`world.tick() - 1`) in which `seen` first holds, stepping
    /// from the current state; panics after `limit` ticks.
    fn first_tick(world: &mut World, limit: u32, seen: impl Fn(&World) -> bool) -> Tick {
        for _ in 0..limit {
            world.step();
            if seen(world) {
                return world.tick() - 1;
            }
        }
        panic!("not reached within {limit} ticks");
    }

    // ECO-HQ T16, T17: the builder's approach walk, unit 1, walk to
    // position 0 of 0 ticks.
    // spec: ECO-HQ-4.6a, ECO-HQ-4.6b, TRN-FREE-5.5 (T16, T17)
    #[test]
    fn t16_t17_approach_unit_and_first_step() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        // The builder's road walk ended in start - 1; the approach is
        // ordered in `start` (ECO-HQ-4.6b step 2).
        let start = supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], true);
        // 3 lattice steps are 34 ticks (TRN-FREE-5.3), out and back.
        assert_eq!(world.builder_approach_ticks(at, None), 34 + 34);
        let on_site = |w: &World| w.site(at).unwrap().on_site.clone();
        assert_eq!(on_site(&world), vec![(PLANK, 2), (STONE, 2)]);
        // Unit 1 is chosen 68 ticks after the approach was ordered.
        let chosen = first_tick(&mut world, 200, |w| {
            w.site(at).unwrap().on_site == vec![(PLANK, 1), (STONE, 2)]
        });
        assert_eq!(chosen, start + 68);
        assert_eq!(world.site(at).unwrap().work_done, 0);
        // The walk to position 0 is ordered one tick later and, being 0
        // ticks long, lets the first decrement fall one tick after that.
        let first = first_tick(&mut world, 50, |w| w.site(at).unwrap().work_done == 1);
        assert_eq!(first, chosen + 2);
    }

    // ECO-HQ T17: a walk of n ticks to position 0 lets the first decrement
    // fall n ticks after it was ordered.
    // spec: ECO-HQ-4.6a, TRN-FREE-5.3 (T17)
    #[test]
    fn t17_walk_to_position_zero() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        // Position 0 two lattice steps from the flag's sample (23 ticks,
        // TRN-FREE-5.3); the others on the flag's sample.
        let flag = world.terrain.neighbour(at, 1).unwrap();
        let sample = grid::field_sample(flag);
        let target = (sample.0 - 1, sample.1 + 2);
        assert_eq!(grid::distance(sample, target), 2);
        let kind = world.building_types.get_mut(&SAWMILL).unwrap();
        kind.construction_positions = [[(-1, 2), (0, 0), (0, 0), (0, 0)]; 3];
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        let start = supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], true);
        let chosen = first_tick(&mut world, 200, |w| {
            w.site(at).unwrap().on_site == vec![(PLANK, 1), (STONE, 2)]
        });
        assert_eq!(chosen, start + 68);
        // Ordered in chosen + 1, 23 ticks long.
        let first = first_tick(&mut world, 100, |w| w.site(at).unwrap().work_done == 1);
        assert_eq!(first, chosen + 1 + 23);
    }

    // ECO-HQ T18: after a unit that ended in E, the next material is taken
    // in E + 1 and the walk to position 0 is ordered in E + 2.
    // spec: ECO-HQ-4.6b (T18)
    #[test]
    fn t18_next_unit_follows_the_end_of_the_previous() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        let _ = supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], true);
        let first = first_tick(&mut world, 200, |w| {
            w.site(at).unwrap().on_site == vec![(PLANK, 1), (STONE, 2)]
        });
        // The unit ends with its extra decrement: the 221st.
        let end = first_tick(&mut world, 400, |w| w.site(at).unwrap().work_done == 221);
        assert_eq!(end, first + 225);
        let second = first_tick(&mut world, 10, |w| {
            w.site(at).unwrap().on_site == vec![(STONE, 2)]
        });
        assert_eq!(second, end + 1);
        let decrement = first_tick(&mut world, 10, |w| w.site(at).unwrap().work_done == 222);
        assert_eq!(decrement, second + 2);
    }

    // ECO-HQ T19: without material the approach repeats, in the tick after
    // each failed choice; a unit starts at the first choice from the arrival
    // of the material on.
    // spec: ECO-HQ-4.6b, ECO-HQ-4.5 (T19)
    #[test]
    fn t19_approach_repeats_until_material_is_there() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        let start = supply(&mut world, at, vec![], true);
        // Choices fail in start + 68, start + 137, start + 206, ...
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(world.tick() - 1, start + 100);
        let _ = supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], false);
        let chosen = first_tick(&mut world, 200, |w| {
            w.site(at).unwrap().on_site == vec![(PLANK, 1), (STONE, 2)]
        });
        assert_eq!(chosen, start + 137);
    }

    // ECO-HQ-4.5: the main part takes a plank first if one is on site.
    #[test]
    fn main_part_prefers_planks() {
        let mut site = ConstructionSite {
            size: BuildingSize::Small,
            levelling_needed: false,
            levelling_bits: LEVELLING_DONE.to_bits(),
            level_direction: 0,
            leveller: None,
            builder: None,
            frame_costs: vec![],
            costs: vec![(STONE, 1), (PLANK, 1)],
            delivered: vec![],
            on_site: vec![(STONE, 1), (PLANK, 1)],
            work_done: 0,
            frame_end: 0,
            work_total: 441,
            unit_ticks: 221,
            leveller_on_way: false,
            builder_on_way: false,
            orders_filed: false,
        };
        assert_eq!(site.next_material(Some(PLANK), Some(STONE)), Some(PLANK));
        site.on_site = vec![(STONE, 1)];
        assert_eq!(site.next_material(Some(PLANK), Some(STONE)), Some(STONE));
    }

    // Levelling first, then building: T6 and T8 together.
    // spec: ECO-HQ-4.3, ECO-HQ-4.6b (T6, T8), ECO-REQ-1.3
    #[test]
    fn levelling_then_building() {
        let (mut world, at) = uneven_sawmill_site();
        supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], true);
        let levelled = run_until(&mut world, |w| w.site(at).unwrap().levelled());
        let done = run_until(&mut world, |w| w.site(at).is_none());
        // The site sees the levelling its leveller's last step completed in
        // its next update, where the approach is ordered.
        assert_eq!(done - levelled, 970);
    }

    // ECO-HQ-4.7 examples: sawmill (2 + 2), woodcutter hut (1 + 1), depot
    // (4 + 3), as (frame done, building complete) in decrements.
    #[test]
    fn work_schedules() {
        assert_eq!(work_schedule(2, 2, 221, 220), (441, 882));
        assert_eq!(work_schedule(1, 1, 221, 220), (221, 442));
        assert_eq!(
            work_schedule(4, 3, 221, 220),
            (3 * 221 + 218, 6 * 221 + 216)
        );
        assert_eq!(work_schedule(0, 2, 221, 220).0, 0);
    }

    // ECO-HQ T-build-1: a farm (frame 3, main 3), every decrement in float32
    // with the amount f32(f32(1 / 3) / 220): the frame reads 0.015152434,
    // 0.0015160687 and 9.171199e-07 after 650, 659 and 660 decrements, the
    // 661st ends it; the main part counts down from 1 by 0.0015151516 and
    // the building completes at decrement 1322 (unit 6's 217th).
    // spec: ECO-HQ-4.6 (T-build-1)
    #[test]
    fn t_build_1_the_amount_is_rounded_twice() {
        let left = |k| remaining_work(3, 3, 221, 220, k).0;
        assert_eq!(left(650), 0.015152434);
        assert_eq!(left(659), 0.0015160687);
        assert_eq!(left(660), 9.171199e-7);
        assert_eq!(left(661), 0.0);
        assert_eq!(remaining_work(3, 3, 221, 220, 662).1, 1.0 - 0.0015151516);
        assert_eq!(work_schedule(3, 3, 221, 220), (661, 5 * 221 + 217));
    }

    // ECO-HQ T11: a hut with 2 planks: unit 1 takes a plank when it starts,
    // the frame completes in unit 1's last tick, the building in unit 2's
    // last tick (442 work ticks).
    // spec: ECO-HQ-4.5, ECO-HQ-4.6, ECO-HQ-4.7 (T11)
    #[test]
    fn t11_hut_work() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, HUT_2, at), Ok(()));
        let start = supply(&mut world, at, vec![(PLANK, 2)], true);
        // ECO-HQ-4.6b: the plank is taken when unit 1 is chosen, after the
        // approach walk.
        assert_eq!(world.site(at).unwrap().on_site, vec![(PLANK, 2)]);
        let taken = run_until(&mut world, |w| {
            w.site(at).unwrap().on_site == vec![(PLANK, 1)]
        });
        assert_eq!(taken - start, 68);
        // Frame: unit 1's 221st decrement, in the tick after its fourth
        // step; the building: unit 2's, 226 ticks later.
        let frame = run_until(&mut world, |w| w.site(at).unwrap().frame_done());
        assert_eq!(frame - start, 68 + 225);
        let done = run_until(&mut world, |w| w.site(at).is_none());
        assert_eq!(done - start, 68 + 226 + 225);
    }

    // ECO-HQ T12: a levelling step sets the 19 samples within 2 lattice
    // steps of the neighbour's sample to the site's height; the ring at 3
    // is averaged; ElevationBlocked and Water stay as they were.
    // spec: ECO-HQ-4.8 (T12)
    #[test]
    fn t12_levelling_heights() {
        let at = (7, 7);
        let mut terrain = flat(16, 16);
        let (columns, rows) = terrain.sample_size();
        terrain.samples = vec![0; (columns * rows) as usize];
        let site_sample = grid::field_sample(at);
        terrain.samples[(site_sample.1 * columns + site_sample.0) as usize] = 300;
        set_height(&mut terrain, at, 300);
        let n = terrain.neighbour(at, 0).unwrap();
        crate::test_support::set(
            &mut terrain,
            n,
            crate::map::state::FOR_BUILDING | crate::map::state::ELEVATION_BLOCKED,
        );
        let mut world = site_world(terrain);
        let touched = world.set_field_height(n, 300);
        let center = grid::field_sample(n);
        for s in grid::area(center, 2, columns, rows) {
            assert_eq!(world.terrain().sample_height(s), 300, "{s:?}");
        }
        for s in grid::ring(center, 3, columns, rows) {
            let h = world.terrain().sample_height(s);
            assert!((0..300).contains(&h), "{s:?} {h}");
        }
        assert_eq!(world.terrain().height_at(n), 300);
        assert!(world.terrain().has(n, crate::map::state::ELEVATION_BLOCKED));
        assert_eq!(touched.len(), 7);
    }

    // TRN-MOVE T14: a walk is ordered from (5, 8) to (5, 7), then the
    // field ahead is levelled up in the same tick (a class-2 climb now):
    // the first step is still rated level (0.045, the heights at the
    // order); the walk's next step and a walk ordered after the change see
    // the new heights.
    // spec: TRN-MOVE-2.1a (T14), TRN-MOVE-1.3
    #[test]
    fn t14_the_first_step_is_rated_at_the_order() {
        let mut world = site_world(flat(16, 16));
        let (from, ahead) = ((5, 8), (5, 7));
        let settler = world.dying_settler(P0, 0, from, 7);
        let id = world.add_settler(settler);
        world.settlers[id].as_mut().unwrap().walker.walk([ahead]);
        world.set_field_height(ahead, 1136);
        let rated = |w: &crate::movement::Walker| w.speed_millionths();
        let terrain = world.terrain.clone();
        let speed = world.settings.settler_speed;
        let walker = &mut world.settlers[id].as_mut().unwrap().walker;
        assert_eq!(walker.first_heights, Some((0, 0)));
        walker.advance(&terrain, speed, |_| true);
        assert_eq!(walker.next, Some(ahead));
        assert_eq!(rated(walker), 45_000);
        assert_eq!(walker.first_heights, None);
        // Ordered after the change: rated from the heights now.
        let mut later = crate::movement::Walker::new(from);
        later.walk([ahead]);
        later.advance(&terrain, speed, |_| true);
        assert!(rated(&later) < 45_000, "{}", rated(&later));
        // A new walk drops what was kept.
        walker.first_heights = Some((0, 0));
        walker.walk([from]);
        assert_eq!(walker.first_heights, None);
    }

    // spec: none (the debug command that supplies a site, not a game rule)
    #[test]
    fn debug_supply_is_checked() {
        let at = (7, 7);
        let mut world = site_world(flat(16, 16));
        assert_eq!(building(&mut world, SAWMILL, at), Ok(()));
        let supply = |wares: Vec<(u32, u32)>| Command::DebugSupplySite {
            player: P0,
            at,
            wares,
            workers: false,
        };
        assert_eq!(
            apply(&mut world, supply(vec![(PLANK, 3)])),
            Err(Rejection::WareNotNeeded)
        );
        assert_eq!(
            apply(&mut world, supply(vec![(PLANK, 1), (PLANK, 2)])),
            Err(Rejection::WareNotNeeded)
        );
        assert_eq!(apply(&mut world, supply(vec![(PLANK, 1)])), Ok(()));
        assert_eq!(
            world.site(at).unwrap().shortfall(),
            vec![(PLANK, 1), (STONE, 2)]
        );
        assert_eq!(
            apply(
                &mut world,
                Command::DebugSupplySite {
                    player: P0,
                    at: (3, 3),
                    wares: vec![],
                    workers: true
                }
            ),
            Err(Rejection::NoBuilding)
        );
    }

    // spec: ECO-HQ-4.3, ECO-HQ-4.7
    #[test]
    fn progress_counts_up_to_a_thousand() {
        let (mut world, at) = uneven_sawmill_site();
        let bulldoze = world.bulldoze_ticks();
        assert_eq!(world.site(at).unwrap().progress_permille(bulldoze), 0);
        supply(&mut world, at, vec![(PLANK, 2), (STONE, 2)], true);
        let mut last = 0;
        while let Some(site) = world.site(at) {
            let now = site.progress_permille(bulldoze);
            assert!(now >= last && now <= 1000);
            last = now;
            world.step();
        }
        assert!(last > 990);
    }

    // Same commands, same world, with sites at work.
    // spec: none (determinism of construction: same commands, same world)
    #[test]
    fn construction_is_deterministic() {
        let run = || {
            let (mut world, at) = uneven_sawmill_site();
            supply(&mut world, at, vec![(PLANK, 2), (STONE, 1)], true);
            for _ in 0..900 {
                world.step();
            }
            world
        };
        assert_eq!(run(), run());
    }
}
