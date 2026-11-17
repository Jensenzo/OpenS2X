//! Production: deposits and tree growth, the woodcutter, stonemason and
//! forester (`specs/ECO-outdoor-workers.md`), the workshop cycle
//! (`specs/ECO-worker-cycle.md`) and its input orders
//! (`specs/ECO-requests-dispatch.md` ECO-REQ-4.2, 4.3).
//!
//! Walking over open ground is the free walk of `specs/TRN-free-walk.md`
//! ([`World::free_walk`]), with the fine approach of TRN-FREE-5.
//!
//! About 1500 lines: the worker cycle (ECO-WORK) and the outdoor workers
//! (ECO-OUT) share one step function whose arms read the same settler and
//! deposit state; split, each part would reach into the other's helpers.
//! The tests are in `tests.rs`.

use std::collections::VecDeque;

use crate::grid::{self, Point};
use crate::map::state;
use crate::movement::{self, NOT_WALKABLE};
use crate::{PlayerId, Settler, SettlerJob, Target, Tick, World};

/// ECO-OUT-3.2, 4.2: the countdown before a target check or spot search.
const CHECK_COUNTDOWN: u32 = 50;
/// ECO-WORK-3.1a: the arrival tick and the first waiting tick, before a
/// worker's first check in a building that works inside.
const ARRIVAL_STAND: u32 = 2;
/// ECO-OUT-2.5, 4.4: a flood stops at a field farther than radius + 4.
const SEARCH_MARGIN: u32 = 4;
/// ECO-OUT-4.4: the forester collects this many candidates.
const FORESTER_CANDIDATES: usize = 11;
/// ECO-OUT-6.4: a removed deposit is gone this many ticks after the harvest
/// (counter 1 to 20; settlers are updated before resources, so it reaches 2
/// in the harvest tick).
const REMOVAL_TICKS: Tick = 18;
/// GFX-ANIM-3.8 (corrected Run D): the woodcutter's work arms a removal
/// timer on his tree this many ticks after the work's end (60 for the
/// 55-tick work); ECO-OUT-3.5 reads it as the fall's start.
pub const FALL_AFTER_WORK: Tick = 5;
/// ECO-OUT-3.11, in hundredths: a building with no value yet (a type
/// without a product, the forester's hut) gets 0.5 from its first product;
/// each product adds 0.25, up to 1.0. A type with a product has 0.5 from its
/// site's placement (T26).
pub(crate) const PRODUCTIVITY_FIRST: i32 = 50;
const PRODUCTIVITY_STEP: i32 = 25;
const PRODUCTIVITY_MAX: i32 = 100;
/// ECO-OUT-3.2: "No resources found." below 0.5 (the float32 value, T24).
const PRODUCTIVITY_LOW: f32 = 0.5;
/// ECO-OUT-3.11 (AI-MIL T24): the float32 statistics value: 0.5 to start,
/// +0.25 a product up to 1.0, -0.05 a fall down to 0.
pub(crate) const STAT_FIRST: f32 = 0.5;
const STAT_STEP: f32 = 0.25;
const STAT_MAX: f32 = 1.0;
pub(crate) const STAT_FALL: f32 = 0.05;
/// ECO-FOOD-5.1, ECO-OUT-2.1: the building's search counter reaches this
/// and is set back to 0.
const SEARCH_TOP: u32 = 100;
/// ECO-FOOD-1.3: the fisherman retries in ticks with `(tick + n) mod 100`
/// equal to 0.
const FISH_GRID: u64 = 100;
/// ECO-FOOD-7.3: the shipwright's counter runs from 100 to 0 (101 ticks).
const SHIP_COUNTDOWN: u32 = 100;
/// ECO-OUT-6.5: a withered grain field is removed as a harvested one is,
/// counted from the tick its life ran out (the counter starts at 1 there,
/// without a second step in the same tick; Q-131).
const WITHER_REMOVAL_TICKS: Tick = 19;
/// ECO-OUT-2.3: flood neighbour order (MAP-GRID-3.2 ring order).
const FLOOD_ORDER: [u8; 6] = [5, 0, 1, 2, 3, 4];
/// ECO-OUT-8.1: the hunter's free walk stops this many fields short of the
/// animal's field ...
const HUNTER_STOP_SHORT: usize = 1;
/// ... and his lattice walk on towards its point this many lattice steps
/// short of it.
const HUNTER_LATTICE_STOP: u32 = 2;
/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;

/// Which work cycle a building's worker runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorkCycle {
    /// None modelled: the worker stays inside.
    #[default]
    None,
    /// ECO-WORK-3, ECO-PROD: inputs to product inside the building.
    Workshop,
    /// ECO-PROD-2.2, 3.3: a workshop cycle whose worker eats food and takes
    /// ore from the ground.
    Mine,
    /// ECO-OUT-3: woodcutter and stonemason.
    Gatherer,
    /// ECO-OUT-4
    Forester,
    /// ECO-FOOD-3: harvest a ripe grain field or sow one.
    Farm,
    /// ECO-FOOD-4
    Fisher,
    /// ECO-FOOD-5
    Hunter,
    /// ECO-FOOD-6: the workshop cycle, then a donkey settler at the door.
    DonkeyFarm,
    /// ECO-FOOD-7: boats (workshop cycle) or ship work trips.
    Shipyard,
}

impl WorkCycle {
    /// ECO-WORK-3.1a: the cycles whose worker works inside the building
    /// (workshop, mine, donkey farm, shipyard).
    pub fn works_inside(self) -> bool {
        matches!(
            self,
            WorkCycle::Workshop | WorkCycle::Mine | WorkCycle::DonkeyFarm | WorkCycle::Shipyard
        )
    }
}

/// A deposit type (`deposits.lua`, ECO-OUT-1.3, 1.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositKind {
    pub id: u32,
    /// The good harvested from it.
    pub good: Option<u32>,
    /// `dp_nextDeposit`: what a stone pile becomes after one stone.
    pub next: Option<u32>,
    /// `dp_setElevationSize` (default 1): lattice steps the fine approach
    /// stops short (TRN-FREE-5.2).
    pub elevation: u32,
    /// ECO-OUT-6.2: ticks from planting until it is ripe (1001 for trees).
    pub grow_ticks: u32,
    /// `dp_setBlocking`: also sets Blocked on its field (ECO-OUT-1.5).
    pub blocking: bool,
    /// `dp_setLifeTime` (ECO-OUT-6.5): ticks it lasts unreserved, if any.
    pub life_time: Option<u32>,
    /// `dp_setNeedForPlantablePattern` (ECO-FOOD-3.3).
    pub needs_plantable: bool,
    /// TRN-FREE-5.4: `dp_setRandomFinePosOffsetState` (default on): a new
    /// deposit stands one lattice step off its field's sample.
    pub random_offset: bool,
}

/// Where a deposit is in its life (ECO-OUT-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepositState {
    /// ECO-OUT-4.5: created by a forester, not growing yet.
    Sapling,
    /// Growing, ripe from tick `ripe_at` on (ECO-OUT-6.2).
    Growing { ripe_at: Tick },
    /// ECO-OUT-6.4: being removed since tick `since` (its counter is 20
    /// - (gone_at - t) at the end of tick t); gone in tick `gone_at`.
    Removing { since: Tick, gone_at: Tick },
}

/// A worker's place on the height lattice (TRN-FREE-5), see
/// [`World::fine_position`].
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinePosition {
    /// The sample last reached.
    pub at: Point,
    /// The sample walked toward, and the bits of the step's float32
    /// progress (TRN-MOVE-2.1, 2.5).
    pub next: Option<Point>,
    pub progress_bits: u32,
    /// The lattice direction faced: of the step, else toward the object.
    pub facing: Option<u8>,
}

impl FinePosition {
    /// The step's progress, 0 to 1.
    pub fn progress(&self) -> f32 {
        f32::from_bits(self.progress_bits)
    }

    /// The step's progress in millionths.
    pub fn progress_millionths(&self) -> u32 {
        (f64::from(self.progress()) * f64::from(crate::movement::ONE)) as u32
    }
}

/// A tree, stone pile or other deposit on a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deposit {
    pub kind: u32,
    /// Its exact position, a sample of the height lattice (TRN-FREE-5.1).
    pub position: Point,
    pub state: DepositState,
    /// ECO-OUT-2.6: the building it is reserved for.
    pub reserved_by: Option<Target>,
    /// ECO-OUT-6.5: ticks of life left while it is not reserved.
    pub life_left: Option<u32>,
}

impl Deposit {
    /// ECO-OUT-1.3: fully grown and not being removed.
    pub fn ripe(&self, now: Tick) -> bool {
        matches!(self.state, DepositState::Growing { ripe_at } if now >= ripe_at)
    }
}

/// ECO-OUT-4.3: the field bits that keep a tree or grain off a field.
const NOT_FOR_TREES: u32 = state::BLOCKED
    | state::STREET
    | state::WATER
    | state::COAST
    | state::ELEVATION_BLOCKED
    | state::DEPOSIT_BLOCKED
    | state::MOVEMENT_BLOCKED
    | state::DOODAD_BLOCKED;

/// What the forester needs to know about a field's terrain (ECO-OUT-4.3,
/// 4.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ground {
    /// Neither mining ground nor harbour ground.
    pub plantable: bool,
    /// `pa_setForPlanting`: where grain may be sown (ECO-OUT-4.3).
    pub for_planting: bool,
    /// `pa_setTreeType`: 0 to 3, or none.
    pub tree_type: Option<u8>,
}

/// What a spot search came to (ECO-OUT-4.2 to 4.4).
enum PlantSpot {
    Found(Point),
    /// The queue ran empty first: the search did not finish.
    Unfinished,
    /// It finished without a candidate.
    None,
}

/// A worker's step in its cycle.
///
/// Internal state of the simulation: views read [`crate::view`] instead
/// (`docs/api.md`); this type may change in any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Starting the cycle (ECO-OUT-3.1, 4.1; ECO-WORK-3.10).
    Begin,
    /// ECO-WORK-3.1
    WaitInputs,
    /// ECO-WORK-3.1b: production switched off: standing inside in a state
    /// of his own.
    Off,
    /// ECO-WORK-3.2: the work action.
    Working,
    /// ECO-WORK-3.3: drop time, then pickup time.
    Take { second: bool },
    /// ECO-WORK-3.4: inside with the product, waiting for its place at the
    /// flag; `score` is its wait score (TRN-CARRY-4.4).
    WaitRoom { score: i64 },
    /// ECO-OUT-3.2: inside, counting down to the next check for a reserved
    /// target (woodcutter, stonemason, a farm's harvest, the hunter).
    WaitTarget { countdown: u32 },
    /// ECO-OUT-4.2: inside, counting down to the next spot search
    /// (forester, a farm's sowing).
    WaitSpot { countdown: u32 },
    /// ECO-FOOD-1.3: the fisherman could not start a trip and tries again
    /// in the ticks of his own 100-tick grid.
    FishRetry,
    /// ECO-FOOD-7.3: the shipwright's counter to the ship check.
    WaitShip { countdown: u32 },
    /// ECO-FOOD-7.4 (T55): at the ship on `spot`, putting the plank down:
    /// phase 0 the drop, 1 the pick-up motion, 2 standing; then the work
    /// step (`AtSpot`).
    AtShip { spot: Point, phase: u8 },
    /// ECO-FOOD-6.2: the tick after a donkey was bred.
    Donkey,
    /// ECO-FOOD-4.1a (T51): the fisher's trip has started, `spot` chosen;
    /// he stands, and his walk out is ordered in the next tick.
    OutNext { spot: Point },
    /// Walking out to the flag, on to `spot` if there is one.
    Out { spot: Option<Point> },
    /// ECO-OUT-3.4, 4.6: walking to the last field before `spot`.
    ToSpot { spot: Point },
    /// TRN-FREE-5: the fine approach to the object on `spot`.
    Approach { spot: Point },
    /// ECO-OUT-3.5, 4.7: the work or planting action at `spot`.
    AtSpot { spot: Point },
    /// ECO-OUT-3.7: taking the log or stone; `stand` is the height sample
    /// the worker took it at, the end of his fine approach (TRN-FREE-5),
    /// if he made one.
    TakeWare { second: bool, stand: Option<Point> },
    /// ECO-OUT-3.8 (c1070f8): standing after the take when his walk to the
    /// target ended on the flag (the target next to it), before the walk
    /// back; 3.8a: until his ware is admitted at the flag. `score` is the
    /// ware's wait score for the flag (TRN-CARRY-4.4); its place is granted
    /// once he holds a join for it (TRN-CARRY-2.4a).
    StandBack { stand: Option<Point>, score: i64 },
    /// TRN-FREE-5.6: the fine walk back along the lattice from `stand` to
    /// the point of his field (where his walk out stopped).
    Return { stand: Point, score: i64 },
    /// ECO-OUT-3.8a: on his field, the walk of one step to the flag waits
    /// until his ware is admitted there.
    WaitBack { score: i64 },
    /// ECO-OUT-3.8, 4.8: walking back to the flag; carrying a ware not yet
    /// admitted he asks in every tick and does not step onto the flag
    /// (3.8a).
    Back { score: i64 },
    /// ECO-WORK-3.6: one tick after arriving at the flag.
    AtFlag,
    /// Standing at the flag with the product, waiting for a place there.
    WaitFlag,
    /// ECO-WORK-3.6: drop time (then the ware lies there), pickup time.
    PutDown { second: bool },
    /// Walking from the flag into the building.
    In,
    /// ECO-WORK-3.8
    StepIn,
    /// ECO-WORK-3.9
    Rest,
}

impl World {
    /// The deposit on `field`.
    pub fn deposit(&self, field: Point) -> Option<&Deposit> {
        self.deposits.get(&field)
    }

    /// ECO-OUT-3.5, 6.4: a felled tree's fall: the tick it starts to fall
    /// ([`FALL_AFTER_WORK`] after the harvest) and the tick it is gone.
    /// `None` for anything else: trees are the deposits that neither block
    /// nor have a lifetime (ECO-OUT-1.5, 6.5); a stone pile's last stone
    /// and a withered field do not fall.
    pub fn fall_of(&self, field: Point) -> Option<(Tick, Tick)> {
        let deposit = self.deposits.get(&field)?;
        let DepositState::Removing { gone_at, .. } = deposit.state else {
            return None;
        };
        let kind = self.deposit_kinds.get(&deposit.kind)?;
        if kind.blocking || kind.life_time.is_some() {
            return None;
        }
        let harvest = gone_at.checked_sub(REMOVAL_TICKS)?;
        Some((harvest + FALL_AFTER_WORK, gone_at))
    }

    /// GFX-ANIM-3.8 (Run D): a woodcutter starting his work at a tree
    /// arms its removal for work time + 5 ticks from now, so the tree goes
    /// even if he is interrupted. Trees are the deposits that neither block
    /// nor have a lifetime (ECO-OUT-1.5, 6.5).
    fn arm_removal(&mut self, cycle: WorkCycle, work_ticks: u32, spot: Point) {
        self.arm_removal_from(self.tick, cycle, work_ticks, spot);
    }

    /// [`Self::arm_removal`] for work that began in tick `began`.
    fn arm_removal_from(&mut self, began: Tick, cycle: WorkCycle, work_ticks: u32, spot: Point) {
        let tree = self
            .deposits
            .get(&spot)
            .and_then(|d| self.deposit_kinds.get(&d.kind))
            .is_some_and(|k| !k.blocking && k.life_time.is_none());
        if cycle == WorkCycle::Gatherer && tree {
            let at = began + Tick::from(work_ticks) + FALL_AFTER_WORK;
            self.removal_timers.insert(spot, at);
        }
    }

    /// GFX-ANIM-3.8a: the tick the woodcutter's work at the tree on `field`
    /// began (the work clip's start): from the worker at the tree, or, once
    /// it is felled, the harvest tick less his building's production time.
    /// `None` for anything but a tree being worked at or felled.
    pub fn felling_began(&self, field: Point) -> Option<Tick> {
        let deposit = self.deposits.get(&field)?;
        let kind = self.deposit_kinds.get(&deposit.kind)?;
        if kind.blocking || kind.life_time.is_some() {
            return None;
        }
        // Only a gatherer whose product is the tree's good fells it; a
        // forester at his sapling (ECO-OUT-4.7) is planting, not felling.
        let production = |target: &Target| {
            let building = self.building_at(target.at)?;
            let kind_of = self.building_types.get(&building.building_type)?;
            (kind_of.cycle == WorkCycle::Gatherer && kind_of.product == kind.good)
                .then_some(kind_of.production_ticks)
        };
        if let Some((start, _)) = self.fall_of(field) {
            let harvest = start - FALL_AFTER_WORK;
            // The gatherer whose product is the tree's good.
            let ticks = self
                .building_types
                .values()
                .find(|t| t.cycle == WorkCycle::Gatherer && t.product == kind.good)?
                .production_ticks;
            return harvest.checked_sub(Tick::from(ticks));
        }
        self.settlers.iter().flatten().find_map(|s| match &s.job {
            SettlerJob::Worker {
                target,
                work: Work::AtSpot { spot },
            } if *spot == field => {
                let done = production(target)?.saturating_sub(s.timer);
                self.tick.checked_sub(Tick::from(done))
            }
            _ => None,
        })
    }

    /// All deposits by field.
    pub fn deposits(&self) -> impl Iterator<Item = (Point, &Deposit)> {
        self.deposits.iter().map(|(&f, d)| (f, d))
    }

    pub fn deposit_kind(&self, id: u32) -> Option<&DepositKind> {
        self.deposit_kinds.get(&id)
    }

    /// Goes up whenever a deposit appears, changes kind or disappears, and
    /// whenever a sign (ECO-SPC-3.2) appears or goes.
    pub fn deposit_version(&self) -> u64 {
        self.deposit_version
    }

    /// The input stock of a workshop (ECO-REQ-4.2).
    pub fn inputs(&self, at: Point) -> &[(u32, u32)] {
        self.inputs.get(&at).map(Vec::as_slice).unwrap_or_default()
    }

    /// ECO-OUT-2.6: the deposit the building on `at` has reserved.
    pub fn reserved_target(&self, at: Point) -> Option<Point> {
        self.targets.get(&at).copied()
    }

    /// Put a deposit on a field (ECO-PLACE-1.5: DepositBlocked).
    pub(crate) fn add_deposit(&mut self, field: Point, deposit: Deposit) {
        let blocking = self
            .deposit_kinds
            .get(&deposit.kind)
            .is_some_and(|k| k.blocking);
        self.deposits.insert(field, deposit);
        self.set_bits(field, state::DEPOSIT_BLOCKED, true);
        if blocking {
            // ECO-OUT-1.5
            self.set_bits(field, state::BLOCKED, true);
        }
        self.deposit_version += 1;
    }

    /// ECO-OUT-6.4, 6.5: one tick of the deposits: lifetimes count down
    /// while unreserved, removed ones disappear and their field is free
    /// again.
    pub(crate) fn update_deposits(&mut self) {
        let now = self.tick;
        let mut withered = false;
        for deposit in self.deposits.values_mut() {
            if let (Some(left), None, DepositState::Growing { .. }) =
                (deposit.life_left, deposit.reserved_by, deposit.state)
            {
                // ECO-OUT-6.5
                if left <= 1 {
                    deposit.life_left = None;
                    deposit.state = DepositState::Removing {
                        since: now,
                        gone_at: now + WITHER_REMOVAL_TICKS,
                    };
                    withered = true;
                } else {
                    deposit.life_left = Some(left - 1);
                }
            }
        }
        // GFX-ANIM-3.8: an armed tree not harvested by its time starts
        // being removed (the counter starts at 1, as for a withered field).
        let due: Vec<Point> = self
            .removal_timers
            .iter()
            .filter(|(_, &at)| now >= at)
            .map(|(&f, _)| f)
            .collect();
        for field in due {
            self.removal_timers.remove(&field);
            if let Some(deposit) = self.deposits.get_mut(&field) {
                if !matches!(deposit.state, DepositState::Removing { .. }) {
                    deposit.state = DepositState::Removing {
                        since: now,
                        gone_at: now + WITHER_REMOVAL_TICKS,
                    };
                    deposit.reserved_by = None;
                    withered = true;
                }
            }
        }
        if withered {
            self.deposit_version += 1;
        }
        let gone: Vec<Point> = self
            .deposits
            .iter()
            .filter(
                |(_, d)| matches!(d.state, DepositState::Removing { gone_at, .. } if now >= gone_at),
            )
            .map(|(&f, _)| f)
            .collect();
        for field in gone {
            let blocking = self
                .deposits
                .remove(&field)
                .and_then(|d| self.deposit_kinds.get(&d.kind))
                .is_some_and(|k| k.blocking);
            self.set_bits(field, state::DEPOSIT_BLOCKED, false);
            if blocking {
                self.set_bits(field, state::BLOCKED, false);
            }
            self.changed(&[field]);
            self.deposit_version += 1;
        }
    }

    // ---- Inputs (ECO-REQ-4.2, ECO-PROD-6): see `workshop.rs` ----

    /// Open orders of `good` for the building on `at`.
    pub(crate) fn open_orders(&self, at: Point, good: u32) -> u32 {
        self.orders
            .iter()
            .filter(|o| o.target.at == at && o.good == good)
            .count() as u32
    }

    /// A ware for a workshop's stock.
    pub(crate) fn receive_input(&mut self, at: Point, good: u32) {
        add(self.inputs.entry(at).or_default(), good, 1);
    }

    // ---- The building's target search (ECO-OUT-2) ----

    /// ECO-FOOD-2.2: the 101-tick timer of a finished building fired: a
    /// mine's ore search.
    pub(crate) fn timer_fired(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        if self.building_types[&building.building_type].cycle == WorkCycle::Mine {
            self.mine_search(at);
        }
    }

    /// ECO-OUT-2.2: the target search of a gatherer or a farm, when its
    /// search counter fires (ECO-OUT-2.1).
    fn target_search(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        if self.targets.contains_key(&at) {
            return;
        }
        let Some(product) = kind.product else { return };
        if !self.goods.get(&product).is_some_and(|g| g.gathered) {
            return;
        }
        let radius = kind.working_radius;
        if let Some(found) = self.search_target(at, product, radius) {
            let target = self.target_of(at).expect("a building");
            self.deposits
                .get_mut(&found)
                .expect("a deposit")
                .reserved_by = Some(target);
            self.targets.insert(at, found);
        }
    }

    /// ECO-FOOD-2.2 to 2.6: the mine's search for an ore field, every 101
    /// ticks while its worker is there and it has no current field.
    fn mine_search(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let (Some(product), radius) = (kind.product, kind.working_radius) else {
            return;
        };
        if !building.active
            || self.ore_fields.contains_key(&at)
            || !self.goods.get(&product).is_some_and(|g| g.resource)
        {
            return;
        }
        match self.find_ore_field(at, product, radius) {
            Some(field) => {
                self.ore_fields.insert(at, field);
            }
            None => {
                // ECO-FOOD-2.6
                if building.production.enabled
                    && f32::from_bits(building.productivity_bits) < PRODUCTIVITY_LOW
                {
                    self.message(building.owner, at, "No resources found.");
                }
            }
        }
    }

    /// ECO-OUT-2.1, ECO-FOOD-5.1: the building's search counter (1 per
    /// tick and 1 more in every tick it is not 0): when it has reached 100
    /// a hunter's hut searches for an animal and a gatherer or a farm for a
    /// target (ECO-OUT-2.2), once every 51 ticks (ECO-OUT T22; it was the
    /// 101-tick timer for the gatherers and farms).
    pub(crate) fn update_search(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        if !matches!(
            kind.cycle,
            WorkCycle::Hunter | WorkCycle::Gatherer | WorkCycle::Farm
        ) {
            return;
        }
        let counter = self.search_timers.get(&at).copied().unwrap_or(0);
        let value = counter + 1 + u32::from(counter != 0);
        if value < SEARCH_TOP {
            self.search_timers.insert(at, value);
            return;
        }
        self.search_timers.insert(at, 0);
        if kind.cycle == WorkCycle::Hunter {
            self.hunter_search(at);
        } else {
            self.target_search(at);
        }
    }

    /// ECO-FOOD-5.1: the hunter's hut's search for an animal.
    fn hunter_search(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let radius = self.building_types[&building.building_type].working_radius;
        if self.hunted.contains_key(&at) {
            return;
        }
        if let Some(id) = self.find_animal(at, building.owner, radius) {
            let target = self.target_of(at).expect("a building");
            if let Some(Some(animal)) = self.animals.get_mut(id) {
                animal.hunted_by = Some(target);
            }
            self.hunted.insert(at, id);
        }
    }

    /// ECO-OUT-2.3 to 2.5: the first ripe, unreserved deposit of `product`
    /// within `radius` in flood order.
    pub(crate) fn search_target(&self, at: Point, product: u32, radius: u32) -> Option<Point> {
        let walkable = |f: Point| !self.terrain.has(f, NOT_WALKABLE);
        let goal = |f: Point| {
            grid::distance(at, f) <= radius
                && self.deposits.get(&f).is_some_and(|d| {
                    d.reserved_by.is_none()
                        && d.ripe(self.tick)
                        && self.deposit_kinds.get(&d.kind).and_then(|k| k.good) == Some(product)
                })
        };
        let mut entered = vec![false; self.terrain.len()];
        let mut queue = VecDeque::from([at]);
        entered[self.terrain.index(at)] = true;
        if goal(at) {
            return Some(at);
        }
        while let Some(a) = queue.pop_front() {
            for direction in FLOOD_ORDER {
                let Some(b) = self.terrain.neighbour(a, direction) else {
                    continue;
                };
                let index = self.terrain.index(b);
                if entered[index] {
                    continue;
                }
                let by_deposit = self.deposits.contains_key(&b)
                    && !self.terrain.has(b, state::ELEVATION_BLOCKED)
                    && (walkable(a) || a == at);
                if !(walkable(b) || by_deposit) {
                    continue;
                }
                entered[index] = true;
                if grid::distance(at, b) > radius + SEARCH_MARGIN {
                    return None;
                }
                if goal(b) {
                    return Some(b);
                }
                queue.push_back(b);
            }
        }
        None
    }

    /// ECO-OUT-4.3 with the grain flag, without the radius and the
    /// Blocked-neighbour conditions (the farm's site value, AI-CON-6.3): no
    /// flag, none of the eight blocking bits, terrain neither for mining nor
    /// harbour ground, and for planting.
    pub fn grain_plantable(&self, field: Point) -> bool {
        self.terrain.contains(field)
            && self.flag_at(field).is_none()
            && !self.terrain.has(field, NOT_FOR_TREES)
            && self.ground_at(field).plantable
            && self.ground_at(field).for_planting
    }

    /// ECO-OUT-4.3, 4.4: the forester's candidates, in flood order; `None`
    /// if the search did not finish (the queue ran empty first).
    pub(crate) fn forester_candidates(
        &self,
        at: Point,
        radius: u32,
        needs_planting: bool,
    ) -> Option<Vec<Point>> {
        let terrain = &self.terrain;
        let candidate = |f: Point| {
            grid::distance(at, f) <= radius
                && self.flag_at(f).is_none()
                && !terrain.has(f, NOT_FOR_TREES)
                && self.ground_at(f).plantable
                && (!needs_planting || self.ground_at(f).for_planting)
                && terrain
                    .neighbours(f)
                    .into_iter()
                    .flatten()
                    .all(|n| !terrain.has(n, state::BLOCKED))
        };
        let mut found = Vec::new();
        let mut entered = vec![false; terrain.len()];
        let mut queue = VecDeque::from([at]);
        entered[terrain.index(at)] = true;
        if candidate(at) {
            found.push(at);
        }
        while let Some(a) = queue.pop_front() {
            for direction in FLOOD_ORDER {
                let Some(b) = terrain.neighbour(a, direction) else {
                    continue;
                };
                let index = terrain.index(b);
                if entered[index] || terrain.has(b, NOT_WALKABLE) {
                    continue;
                }
                entered[index] = true;
                if grid::distance(at, b) > radius + SEARCH_MARGIN {
                    return Some(found);
                }
                if candidate(b) {
                    found.push(b);
                    if found.len() == FORESTER_CANDIDATES {
                        return Some(found);
                    }
                }
                queue.push_back(b);
            }
        }
        None
    }

    /// ECO-OUT-4.3: what the terrain of a field allows.
    pub fn ground_at(&self, field: Point) -> Ground {
        self.ground
            .get(self.terrain.index(field))
            .copied()
            .unwrap_or(Ground {
                plantable: true,
                for_planting: true,
                tree_type: None,
            })
    }

    /// ECO-OUT-4.5: a random kind of the tree family of the field's terrain.
    fn sapling_kind(&mut self, field: Point) -> Option<u32> {
        let family = match self.ground_at(field).tree_type {
            Some(t) => self.tree_families.get(usize::from(t)).cloned(),
            None => None,
        };
        let family = family
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| self.default_tree.into_iter().collect());
        if family.is_empty() {
            return None;
        }
        let pick = (u64::from(self.rng.draw()) % family.len() as u64) as usize;
        Some(family[pick])
    }

    // ---- The worker cycles ----

    /// The worker's next step (ECO-WORK-3, ECO-OUT-3, 4, ECO-FOOD); after
    /// timer and movement (ECO-WORK-2.2). `number` is the settler's own
    /// number (ECO-FOOD-1.3).
    pub(crate) fn work(
        &mut self,
        settler: &mut Settler,
        target: Target,
        work: Work,
        number: usize,
    ) -> Work {
        let at = target.at;
        let Some(building) = self.target_building(target).copied() else {
            return work;
        };
        let kind = self.building_types[&building.building_type].clone();
        let flag = self
            .terrain
            .neighbour(at, FLAG_DIRECTION)
            .expect("a building has a flag");
        let arrived = settler.walker.next.is_none() && !settler.walker.is_walking();
        let timer_done = settler.timer == 0;
        let rest = self
            .goods
            .get(&settler.profession)
            .map_or(1, |g| g.rest_ticks.max(1));
        match work {
            // ECO-WORK-3.1a: the ticks after arriving, before the first
            // check.
            Work::Begin if kind.cycle.works_inside() && !timer_done => Work::Begin,
            Work::Begin => match kind.cycle {
                // ECO-FOOD-7.3 (T54): in ship mode the step starts only while
                // production is on; off, he stays in the mode check and
                // tries again every tick.
                WorkCycle::Shipyard if building.production.switched => {
                    if building.production.enabled {
                        // ECO-FOOD-7.1: ship mode.
                        Work::WaitShip {
                            countdown: SHIP_COUNTDOWN,
                        }
                    } else {
                        Work::Begin
                    }
                }
                WorkCycle::Workshop
                | WorkCycle::Mine
                | WorkCycle::DonkeyFarm
                | WorkCycle::Shipyard => {
                    // ECO-PROD-3.1: a cycle begins only while production is
                    // on, and a mine only with an ore field.
                    if !building.production.enabled {
                        // ECO-WORK-3.1b: switched off, a state of his own.
                        Work::Off
                    } else if kind.cycle == WorkCycle::Mine && !self.take_ore(at) {
                        Work::Begin
                    } else if self.can_produce(at) {
                        settler.timer = kind.production_ticks;
                        Work::Working
                    } else {
                        Work::WaitInputs
                    }
                }
                // ECO-OUT-3.1, 4.1, ECO-FOOD-3.1, 5.3: only while enabled.
                WorkCycle::Gatherer | WorkCycle::Hunter if !building.production.enabled => {
                    Work::Begin
                }
                WorkCycle::Forester | WorkCycle::Farm if !building.production.enabled => {
                    Work::Begin
                }
                WorkCycle::Gatherer | WorkCycle::Hunter => Work::WaitTarget {
                    countdown: CHECK_COUNTDOWN,
                },
                WorkCycle::Forester => Work::WaitSpot {
                    countdown: CHECK_COUNTDOWN,
                },
                // ECO-FOOD-3.2a: the decision in the tick the rest ends.
                WorkCycle::Farm => self.farm_decide(settler, at, flag),
                WorkCycle::Fisher => self.start_fishing(at, flag).unwrap_or(Work::FishRetry),
                WorkCycle::None => Work::Begin,
            },
            Work::OutNext { spot } => {
                self.walk_out(settler, at, flag);
                Work::Out { spot: Some(spot) }
            }
            Work::FishRetry => {
                // ECO-FOOD-1.3: once per 100 ticks, in the ticks with
                // `(tick + n) mod 100 = 0`.
                if (self.tick + number as u64).is_multiple_of(FISH_GRID) {
                    self.start_fishing(at, flag).unwrap_or(Work::FishRetry)
                } else {
                    work
                }
            }
            // ECO-WORK-3.1b: switched off, he stands in a state of his own;
            // in the tick after it is on again he only returns to waiting,
            // his first check (3.1) in the next.
            Work::WaitInputs | Work::Off if !building.production.enabled => Work::Off,
            Work::Off => Work::Begin,
            Work::WaitInputs => {
                if self.can_produce(at) {
                    settler.timer = kind.production_ticks;
                    Work::Working
                } else {
                    work
                }
            }
            Work::Working if timer_done && kind.cycle == WorkCycle::DonkeyFarm => {
                // ECO-FOOD-6.2: no ware; the donkey is made next tick.
                self.consume_inputs(at);
                self.produced(at);
                Work::Donkey
            }
            Work::Donkey => {
                // ECO-FOOD-6.2: a settler `good_donkey` at the door (T-door-5);
                // no rest, but in this tick the breeder only goes back to
                // waiting: his first can-produce check is in the next tick
                // (T58, corrected 2026-10-31: a cycle of 335 ticks).
                // SIM-START-4.2: the donkey is a settler made.
                let donkey = self.new_number();
                self.send_home_by_door(building.owner, self.roles.donkey, at, donkey);
                Work::WaitInputs
            }
            Work::Working if timer_done => {
                // ECO-WORK-3.2: the inputs are used, the product exists.
                self.consume_inputs(at);
                self.produced(at);
                // ECO-PROD-5.1: the product is decided as the taking starts.
                settler.carrying = self.take_product(at);
                self.product_made(settler.carrying);
                settler.timer = self.settings.drop_ticks;
                Work::Take { second: false }
            }
            Work::Take { second: false } if timer_done => {
                settler.timer = self.settings.pickup_ticks;
                self.take_in_hand(settler);
                Work::Take { second: true }
            }
            // ECO-PROD-5.5: no ware was made, nothing is carried out.
            Work::Take { second: true } if timer_done && settler.carrying.is_none() => {
                settler.timer = rest;
                Work::Rest
            }
            Work::Take { second: true } if timer_done => {
                // ECO-WORK-3.4 (77ed505): he stands in the two ticks after the
                // take; with room the place and the walk out come in the
                // third.
                settler.timer = 2;
                Work::WaitRoom { score: 0 }
            }
            Work::WaitRoom { mut score } if !timer_done => {
                // ECO-WORK-3.4 (T-room-1, T-room-2), TRN-CARRY-4.4: the second
                // standing tick makes a whole entry check: it raises the
                // flag's bar (or, at a full flag, adds to the ware's score),
                // and when the bar already holds the ware's score (another
                // ware's bid set it) it admits the ware and he walks out.
                let good = settler.carrying.expect("a product to bring out");
                if settler.timer == 1 && self.flag_bid(flag, good, None, &mut score) {
                    self.hold_join(settler.number);
                    self.walk_out(settler, at, flag);
                    Work::Out { spot: None }
                } else {
                    Work::WaitRoom { score }
                }
            }
            Work::WaitRoom { mut score } => {
                // TRN-CARRY-4.3, 4.4: admitted by the entry check, a place at
                // the flag is reserved for the product (Q-177).
                let good = settler.carrying.expect("a product to bring out");
                if self.flag_bid(flag, good, None, &mut score) {
                    // TRN-CARRY-2.4a: admitted, the product joins the
                    // flag's list.
                    self.hold_join(settler.number);
                    self.walk_out(settler, at, flag);
                    Work::Out { spot: None }
                } else {
                    Work::WaitRoom { score }
                }
            }
            Work::WaitTarget { countdown } if countdown > 0 => Work::WaitTarget {
                countdown: countdown - 1,
            },
            Work::WaitTarget { .. } => {
                let spot = match kind.cycle {
                    // ECO-FOOD-5.3
                    WorkCycle::Hunter => self
                        .hunted
                        .get(&at)
                        .and_then(|&id| self.animal(id))
                        .map(|a| a.at),
                    // ECO-OUT-3.2, ECO-FOOD-3.2
                    _ => self.targets.get(&at).copied(),
                };
                match spot {
                    Some(spot) => {
                        self.walk_out(settler, at, flag);
                        Work::Out { spot: Some(spot) }
                    }
                    None => {
                        if building.production.enabled
                            && f32::from_bits(building.productivity_bits) < PRODUCTIVITY_LOW
                        {
                            self.message(building.owner, at, "No resources found.");
                        }
                        Work::WaitTarget {
                            countdown: CHECK_COUNTDOWN,
                        }
                    }
                }
            }
            Work::WaitSpot { countdown } if countdown > 0 => Work::WaitSpot {
                countdown: countdown - 1,
            },
            Work::WaitSpot { .. } => match self.plant_spot(at, &kind) {
                // ECO-OUT-4.2 to 4.5
                PlantSpot::Found(spot) => {
                    self.walk_out(settler, at, flag);
                    Work::Out { spot: Some(spot) }
                }
                PlantSpot::Unfinished => Work::WaitSpot {
                    countdown: CHECK_COUNTDOWN,
                },
                // ECO-FOOD-3.2a (T49, corrected 2026-10-27 b): a finished
                // search with no candidate ends the sowing step in its own
                // tick; the farmer decides again in the next (Begin).
                PlantSpot::None if kind.cycle == WorkCycle::Farm => Work::Begin,
                // ECO-OUT-4.4: the forester starts again without resting.
                PlantSpot::None => Work::WaitSpot {
                    countdown: CHECK_COUNTDOWN,
                },
            },
            Work::WaitShip { countdown } if countdown > 0 => Work::WaitShip {
                countdown: countdown - 1,
            },
            Work::WaitShip { .. } => {
                // ECO-FOOD-7.3
                if self.can_produce(at) {
                    if let Some(site) = self.ship_for(at, building.owner, kind.working_radius) {
                        // ECO-FOOD-7.4: one plank is consumed and carried;
                        // SIM-START-4.2 (T56): a ware object, after the ship.
                        self.consume_inputs(at);
                        self.new_number();
                        settler.carrying = self.plank;
                        self.made(self.plank, 1);
                        self.walk_out(settler, at, flag);
                        return Work::Out { spot: Some(site) };
                    }
                }
                // T54: no ship: back to the mode check for this tick; the
                // next step starts in the next (checks 102 ticks apart).
                Work::Begin
            }
            Work::Out { spot } if arrived => match spot {
                Some(spot) => {
                    // ECO-OUT-8.1 (T25, T27): the hunter aims at the
                    // animal's walk goal as the walk starts.
                    let spot = match kind.cycle {
                        WorkCycle::Hunter => self.hunted_goal(at, spot),
                        _ => spot,
                    };
                    self.walk_to_spot(settler, spot, kind.cycle)
                }
                None => Work::AtFlag,
            },
            Work::ToSpot { spot } if arrived => match kind.cycle {
                WorkCycle::Gatherer | WorkCycle::Forester | WorkCycle::Farm => {
                    // TRN-FREE-5: on the lattice to the object, stopping
                    // short by its elevation size.
                    let ticks = self.approach_ticks(settler.walker.at, spot, false);
                    if ticks > 0 {
                        settler.timer = ticks;
                        Work::Approach { spot }
                    } else {
                        settler.timer = kind.production_ticks;
                        self.arm_removal(kind.cycle, kind.production_ticks, spot);
                        Work::AtSpot { spot }
                    }
                }
                WorkCycle::Hunter => {
                    // ECO-OUT-8.1 (T18): in the tick the free walk ends, the
                    // lattice walk towards the animal's point; T25, T27: at
                    // its position now (the start field of a step).
                    let spot = self.hunted_field(at, spot);
                    let ticks = self.approach_ticks(settler.walker.at, spot, true);
                    if ticks > 0 {
                        settler.timer = ticks;
                        Work::Approach { spot }
                    } else {
                        self.hunter_at_animal(settler, at, spot, &kind)
                    }
                }
                WorkCycle::Shipyard => {
                    // ECO-FOOD-7.4 (T55): first the plank goes down, as at
                    // a flag: the drop.
                    settler.timer = self.settings.drop_ticks;
                    Work::AtShip { spot, phase: 0 }
                }
                _ => {
                    settler.timer = kind.production_ticks;
                    Work::AtSpot { spot }
                }
            },
            // ECO-FOOD-7.4a (T60): the work step starts in his last standing
            // tick: its amount fixed, the first of its 56 decrements (one a
            // tick, to the tick before the walk back).
            Work::AtShip { spot, phase: 2 } if settler.timer == 1 => {
                self.ship_step_start(building.owner, spot);
                work
            }
            Work::AtShip { .. } if !timer_done => work,
            Work::AtShip { spot, phase: 0 } => {
                // T55, T56: the drop ends (A + 4): the plank is gone; the
                // pick-up motion.
                if let Some(plank) = settler.carrying.take() {
                    self.used(plank, 1);
                }
                settler.timer = self.settings.pickup_ticks;
                Work::AtShip { spot, phase: 1 }
            }
            Work::AtShip { spot, phase: 1 } => {
                // T55: he stands from A + 8.
                settler.timer = 2;
                Work::AtShip { spot, phase: 2 }
            }
            Work::AtShip { spot, .. } => {
                // T55: the work step's 56 decrements, then the walk back:
                // the trip's stay at the ship is the construct time + 10
                // (the measured build; the spec's A + 65 for the walk back
                // counts one tick less in our tick numbering).
                settler.timer = self.ship_rules.construct_ticks;
                self.ship_decrement(building.owner, spot);
                Work::AtSpot { spot }
            }
            // ECO-FOOD-7.4a (T60): a decrement in every tick of the step; in
            // the last (his timer at 1), the 56th, and the step ends.
            Work::AtSpot { spot } if kind.cycle == WorkCycle::Shipyard && !timer_done => {
                self.ship_decrement(building.owner, spot);
                if settler.timer == 1 {
                    self.ship_step_end(building.owner, spot);
                }
                work
            }
            Work::Approach { spot } if timer_done && kind.cycle == WorkCycle::Hunter => {
                self.hunter_at_animal(settler, at, spot, &kind)
            }
            Work::Approach { spot } if timer_done => {
                // ECO-OUT-3.5, 4.7: the production time.
                settler.timer = kind.production_ticks;
                self.arm_removal(kind.cycle, kind.production_ticks, spot);
                Work::AtSpot { spot }
            }
            Work::AtSpot { spot } if timer_done => match kind.cycle {
                WorkCycle::Forester => {
                    let stand = self.approach_end(settler.walker.at, spot, false);
                    // ECO-OUT-4.7b: a planting that ends with its spot
                    // raises the hut's productivity as a product would.
                    if self.plant(spot) {
                        self.produced(at);
                    }
                    self.walk_home(settler, stand, flag, 0)
                }
                WorkCycle::Farm
                    if self
                        .deposits
                        .get(&spot)
                        .is_some_and(|d| d.state == DepositState::Sapling) =>
                {
                    // ECO-FOOD-3.3, 1.2: sowing ends; the field is sown.
                    let stand = self.approach_end(settler.walker.at, spot, false);
                    // ECO-OUT-9.1 (T19): the planting step makes the field's
                    // deposit once more, its lattice direction drawn
                    // (TRN-FREE-5.4): one draw, no number, the field the
                    // same.
                    self.rng.draw();
                    self.plant(spot);
                    // ECO-FOOD-1.2 (T26): the sowing does not raise the
                    // farm's productivity; the harvest does.
                    // ECO-OUT-9.1, ECO-FOOD-3.2a: back to the flag, in, rest
                    // (TRN-FREE-5.6: first back on the lattice).
                    self.walk_home(settler, stand, flag, 0)
                }
                WorkCycle::Hunter => {
                    // ECO-FOOD-5.3: the animal is killed.
                    if let Some(id) = self.hunted.remove(&at) {
                        self.remove_animal(id);
                    }
                    self.produced(at);
                    settler.carrying = kind.product;
                    self.product_made(kind.product);
                    settler.timer = self.settings.drop_ticks;
                    // TRN-FREE-5.6: home first by the fine walk back.
                    let stand = self.approach_end(settler.walker.at, spot, true);
                    Work::TakeWare {
                        second: false,
                        stand,
                    }
                }
                WorkCycle::Fisher => {
                    // ECO-FOOD-4.5: a unit from the fish field, or from the
                    // first one around him; 4.6: the fish is made in every
                    // case.
                    self.catch_fish(at, settler.walker.at);
                    self.produced(at);
                    settler.carrying = kind.product;
                    self.product_made(kind.product);
                    settler.timer = self.settings.drop_ticks;
                    Work::TakeWare {
                        second: false,
                        stand: None,
                    }
                }
                WorkCycle::Shipyard => {
                    // ECO-FOOD-7.4: the plank is down (AtShip), one step is
                    // worked.
                    self.produced(at);
                    self.walk_back(settler, flag, 0, false)
                }
                _ => {
                    let stand = self.approach_end(settler.walker.at, spot, false);
                    self.harvest(at, spot);
                    self.produced(at);
                    settler.carrying = kind.product;
                    self.product_made(kind.product);
                    settler.timer = self.settings.drop_ticks;
                    Work::TakeWare {
                        second: false,
                        stand,
                    }
                }
            },
            Work::TakeWare {
                second: false,
                stand,
            } if timer_done => {
                settler.timer = self.settings.pickup_ticks;
                self.take_in_hand(settler);
                Work::TakeWare {
                    second: true,
                    stand,
                }
            }
            Work::TakeWare {
                second: true,
                stand,
            } if timer_done => {
                // ECO-OUT-3.8a (1): one check as his walk back starts.
                let score = self.ask_flag(settler, flag, 0);
                // ECO-OUT-3.8: his walk out had no field: a standing tick
                // first; ECO-FOOD-4.6a (T52): the fisherman stands one tick
                // on the spot too, his walk back from the next.
                if settler.walker.at == flag || kind.cycle == WorkCycle::Fisher {
                    Work::StandBack { stand, score }
                } else {
                    self.walk_home(settler, stand, flag, score)
                }
            }
            Work::StandBack { stand, score } => {
                // ECO-OUT-3.8a (1): the standing tick is the wait for his
                // ware's admission, a check in each.
                let score = self.ask_flag(settler, flag, score);
                if !self.admitted(settler) {
                    return Work::StandBack { stand, score };
                }
                // TRN-FREE-5.6 step 3 (afcf1a8): standing exactly on the
                // point of the field his sample truncates to, he walks to
                // the flag from that field at once.
                let on_point = stand
                    .map(grid::sample_field_truncating)
                    .filter(|&f| stand == Some(grid::field_sample(f)) && self.terrain.contains(f));
                match on_point {
                    Some(field) => {
                        settler.walker = movement::Walker::new(field);
                        self.walk_back(settler, flag, score, false)
                    }
                    None => self.walk_home(settler, stand, flag, score),
                }
            }
            Work::Return { score, .. } if timer_done => self.walk_back(settler, flag, score, true),
            Work::WaitBack { score } => self.walk_back(settler, flag, score, true),
            Work::Back { score }
                if !settler.walker.is_walking() && settler.walker.next.is_none() =>
            {
                if settler.walker.at != flag {
                    // ECO-OUT-3.8: no route; try again.
                    return self.walk_back(settler, flag, score, false);
                }
                if settler.carrying.is_some() && self.admitted(settler) {
                    // ECO-OUT-3.8a: admitted on the way, the put-down at once.
                    return Work::AtFlag;
                }
                if let Some(good) = settler.carrying {
                    if self.flag_reserve_direct(flag, good, &mut 0) {
                        self.hold_join(settler.number);
                        Work::AtFlag
                    } else {
                        Work::WaitFlag
                    }
                } else {
                    self.walk_in(settler, at);
                    Work::In
                }
            }
            // ECO-OUT-3.8a (3): walking, a check in every tick.
            Work::Back { score } => Work::Back {
                score: self.ask_flag(settler, flag, score),
            },
            Work::WaitFlag => {
                let good = settler.carrying.expect("a product to put down");
                if self.flag_reserve_direct(flag, good, &mut 0) {
                    self.hold_join(settler.number);
                    Work::AtFlag
                } else {
                    work
                }
            }
            // The place at the flag is reserved (TRN-CARRY-4.3).
            Work::AtFlag => {
                settler.timer = self.settings.drop_ticks;
                Work::PutDown { second: false }
            }
            Work::PutDown { second: false } if timer_done => {
                let good = settler.carrying.take().expect("a ware to put down");
                self.unreserve(flag);
                let joined = self.drop_held_join(settler.number);
                self.new_ware_joined(good, flag, None, None, joined);
                settler.timer = self.settings.pickup_ticks;
                Work::PutDown { second: true }
            }
            Work::PutDown { second: true } if timer_done => {
                self.walk_in(settler, at);
                Work::In
            }
            Work::In if arrived => Work::StepIn,
            Work::StepIn => {
                settler.timer = rest;
                Work::Rest
            }
            Work::Rest if timer_done => match kind.cycle {
                // ECO-OUT-3.10: the next cycle starts in the tick the rest
                // is over, which is already the first tick of the wait; the
                // farmer decides then too (ECO-FOOD-3.2a, ECO-OUT T20), the
                // forester (ECO-OUT-5.2, medium) and the fisherman's trip
                // starts (ECO-FOOD-4.6, T53).
                WorkCycle::Gatherer
                | WorkCycle::Hunter
                | WorkCycle::Farm
                | WorkCycle::Forester
                | WorkCycle::Fisher => self.work(settler, target, Work::Begin, number),
                _ => Work::Begin,
            },
            _ => work,
        }
    }

    /// ECO-FOOD-4.1 to 4.3: the fisherman's trip starts if the hut is on
    /// and the spot search finds a candidate; the spot is chosen at random
    /// and the hut's fish field set. Otherwise the message and no trip.
    fn start_fishing(&mut self, at: Point, flag: Point) -> Option<Work> {
        let building = *self.building_at(at).expect("a building");
        if !building.production.enabled {
            return None;
        }
        let kind = &self.building_types[&building.building_type];
        let (fish, radius) = (kind.product?, kind.working_radius);
        let spots = self.fishing_spots(flag, fish, radius);
        if spots.is_empty() {
            self.message(building.owner, at, "No resources found.");
            return None;
        }
        // ECO-OUT-4.9: one draw, `x mod n`.
        let pick = (u64::from(self.rng.draw()) % spots.len() as u64) as usize;
        let spot = spots[pick];
        match self.first_fish_around(spot, fish) {
            Some(field) => {
                self.fish_fields.insert(at, field);
            }
            None => {
                self.fish_fields.remove(&at);
            }
        }
        // ECO-FOOD-4.1a (T51): the walk out comes in the next tick.
        Some(Work::OutNext { spot })
    }

    /// ECO-OUT-8.1 (T25, T27): the position of the animal the hut on `at`
    /// hunts (its registered field, the start field of a step in
    /// progress), the lattice walk's aim; else `spot`.
    pub(crate) fn hunted_field(&self, at: Point, spot: Point) -> Point {
        self.hunted
            .get(&at)
            .and_then(|&id| self.animal(id))
            .map_or(spot, |a| a.at)
    }

    /// ECO-OUT-8.1 (T27): the walk goal of the animal the hut on `at` hunts,
    /// the free walk's aim: the last field of its current walk, its own
    /// field while it stands; else `spot`.
    pub(crate) fn hunted_goal(&self, at: Point, spot: Point) -> Point {
        self.hunted
            .get(&at)
            .and_then(|&id| self.animal(id))
            .map_or(spot, |a| a.walk_goal())
    }

    /// ECO-FOOD-4.5: the catch. The hut's fish field gives one unit while
    /// it holds fish; otherwise the field is forgotten and the first field
    /// holding fish within the fishing distance of `stand` loses one, if
    /// any.
    fn catch_fish(&mut self, at: Point, stand: Point) {
        let Some(fish) = self
            .building_at(at)
            .and_then(|b| self.building_types[&b.building_type].product)
        else {
            return;
        };
        let own = self.fish_fields.remove(&at).filter(|&f| {
            self.resource_at(f)
                .is_some_and(|r| r.good == fish && r.amount > 0)
        });
        match own {
            Some(field) => {
                self.take_unit(field);
                if self.resource_at(field).is_some() {
                    self.fish_fields.insert(at, field);
                }
            }
            None => {
                if let Some(field) = self.first_fish_around(stand, fish) {
                    self.take_unit(field);
                }
            }
        }
    }

    /// ECO-FOOD-3.1, 3.2a: the farmer's decision: with a reserved ripe
    /// field the target step, the woodcutter's 51-tick target wait (ECO-OUT
    /// T21: the harvest trip starts when it succeeds); otherwise the sowing
    /// step, which waits 50 ticks inside before its spot search.
    fn farm_decide(&mut self, _settler: &mut Settler, at: Point, _flag: Point) -> Work {
        match self.targets.get(&at) {
            Some(_) => Work::WaitTarget {
                countdown: CHECK_COUNTDOWN,
            },
            None => Work::WaitSpot {
                countdown: CHECK_COUNTDOWN,
            },
        }
    }

    /// ECO-FOOD-7.3: the player's ship under construction within `radius`
    /// of the shipyard, or a new one on a site; its field.
    fn ship_for(&mut self, at: Point, owner: crate::PlayerId, radius: u32) -> Option<Point> {
        if let Some(ship) = self
            .ships
            .iter()
            .find(|s| s.owner == owner && !s.finished() && grid::distance(at, s.at) <= radius)
        {
            return Some(ship.at);
        }
        let (site, water) = self.ship_site(at, radius)?;
        // SIM-START-4.2: a ship takes a number when it is made.
        self.new_number();
        // SHIP-2.7: a ship the shipyard builds starts with the coast field
        // the shipwright works from as its last port position.
        let mut vessel = crate::navy::Vessel::new(site);
        vessel.port = Some(site);
        self.ships.push(crate::Ship {
            owner,
            at: site,
            water: Some(water),
            frame_work: 1.0f32.to_bits(),
            main_work: 1.0f32.to_bits(),
            step_amount: None,
            name: None,
            vessel,
        });
        Some(site)
    }

    /// ECO-FOOD-7.3b (T57): where a new ship is started: a flood from the
    /// shipyard's field (always entered) over walkable fields within the
    /// radius, neighbours in the ring order of MAP-GRID-3.2; the first
    /// entered field that is a Coast field on the shipyard's continent with
    /// a neighbour in the map that is Water, not DoodadBlocked and no
    /// unfinished ship's water field (T61); those tested in MAP-GRID directions 3, 4, 5, 0, 1, 2
    /// (the original's 4, 5, 0, 1, 2, 3), the last that passes the water
    /// field. (The building field, the water field.)
    pub(crate) fn ship_site(&self, at: Point, radius: u32) -> Option<(Point, Point)> {
        let continent = self.continent_of(at);
        let water_of = |f: Point| {
            [3, 4, 5, 0, 1, 2]
                .into_iter()
                .filter_map(|d| self.terrain.neighbour(f, d))
                .filter(|&n| {
                    // T61: a launched (finished) ship no longer holds its
                    // water field.
                    self.terrain.has(n, state::WATER)
                        && !self.terrain.has(n, state::DOODAD_BLOCKED)
                        && !self
                            .ships
                            .iter()
                            .any(|s| s.water == Some(n) && !s.finished())
                })
                .last()
        };
        let site = |f: Point| {
            (self.terrain.has(f, state::COAST) && self.continent_of(f) == continent)
                .then(|| water_of(f))
                .flatten()
        };
        let mut entered = vec![false; self.terrain.len()];
        let mut queue = std::collections::VecDeque::from([at]);
        entered[self.terrain.index(at)] = true;
        while let Some(a) = queue.pop_front() {
            if let Some(water) = site(a) {
                return Some((a, water));
            }
            for direction in FLOOD_ORDER {
                let Some(b) = self.terrain.neighbour(a, direction) else {
                    continue;
                };
                let index = self.terrain.index(b);
                if entered[index]
                    || grid::distance(at, b) > radius
                    || self.terrain.has(b, NOT_WALKABLE)
                {
                    continue;
                }
                entered[index] = true;
                queue.push_back(b);
            }
        }
        None
    }

    /// ECO-FOOD-7.4a: the ship at `spot` whose work step is under way.
    fn worked_ship(&mut self, owner: crate::PlayerId, spot: Point) -> Option<&mut crate::Ship> {
        self.ships
            .iter_mut()
            .find(|s| s.at == spot && s.owner == owner && s.step_amount.is_some())
    }

    /// ECO-FOOD-7.4a (T60): a work step starts on the ship at `spot`: the
    /// amount of each decrement is fixed, `1 / (construct time x frame
    /// planks)` while the frame has work left, else `1 / (construct time x
    /// main planks)`, and the first decrement is made.
    fn ship_step_start(&mut self, owner: crate::PlayerId, spot: Point) {
        let rules = self.ship_rules;
        let Some(ship) = self
            .ships
            .iter_mut()
            .find(|s| s.at == spot && s.owner == owner && !s.finished())
        else {
            return;
        };
        let planks = if ship.frame_work != 0 {
            rules.frame_planks
        } else {
            rules.main_planks
        };
        let amount = 1.0f32 / (rules.construct_ticks * planks) as f32;
        ship.step_amount = Some(amount.to_bits());
        ship.decrement(amount);
    }

    /// ECO-FOOD-7.4a: one decrement of the step under way on the ship at
    /// `spot`. 7.4b (T60): the decrement that empties the main part
    /// finishes the ship in that tick (the navy starts it, SHIP-2.1).
    fn ship_decrement(&mut self, owner: crate::PlayerId, spot: Point) {
        if let Some(ship) = self.worked_ship(owner, spot) {
            let amount = f32::from_bits(ship.step_amount.expect("a step"));
            ship.decrement(amount);
        }
    }

    /// ECO-FOOD-7.4a, 7.4b: the step's end, after its 56th decrement; with
    /// both parts 0 the "Ship constructed" message.
    fn ship_step_end(&mut self, owner: crate::PlayerId, spot: Point) {
        let Some(ship) = self.worked_ship(owner, spot) else {
            return;
        };
        ship.step_amount = None;
        if ship.finished() {
            // ECO-FOOD-7.4b (Q-130): "Ship constructed", message type 4, at
            // the ship's water field.
            let at = ship.water.unwrap_or(spot);
            self.message(owner, at, "Ship constructed");
        }
    }

    /// TRN-FREE: the fields of a free walk from `from` to `to`, stopping
    /// `stop_short` fields before the end (TRN-FREE-4.2). A neighbour, and a
    /// building and its flag, are one step without a search (TRN-FREE-2.1,
    /// 2.2). `None` if no walk starts (TRN-FREE-2.8). Its draws are recorded
    /// as the caller's (SIM-RNG-2.4).
    #[track_caller]
    pub(crate) fn free_walk(
        &mut self,
        from: Point,
        to: Point,
        stop_short: usize,
    ) -> Option<Vec<Point>> {
        if from == to {
            return Some(Vec::new());
        }
        let flag_of = |b: Point| {
            self.building_at(b)
                .and_then(|_| self.terrain.neighbour(b, FLAG_DIRECTION))
        };
        let mut fields = if grid::distance(from, to) == 1
            || flag_of(from) == Some(to)
            || flag_of(to) == Some(from)
        {
            vec![to]
        } else {
            movement::free_walk(&self.terrain, &mut self.rng, from, to)?.fields
        };
        fields.truncate(fields.len().saturating_sub(stop_short));
        Some(fields)
    }

    /// TRN-FREE-5.1, 5.2: the lattice steps of the fine approach from
    /// `field` to the object on `spot`, if there is one.
    fn approach_steps(&self, field: Point, spot: Point, hunter: bool) -> Option<Vec<Point>> {
        if hunter {
            // ECO-OUT-8.1 (T18): towards the point of the animal's field.
            return Some(movement::fine_approach(
                &self.terrain,
                grid::field_sample(field),
                grid::field_sample(spot),
                HUNTER_LATTICE_STOP,
            ));
        }
        let deposit = self.deposits.get(&spot)?;
        let elevation = self
            .deposit_kinds
            .get(&deposit.kind)
            .map_or(1, |k| k.elevation);
        let from = grid::field_sample(field);
        Some(movement::fine_approach(
            &self.terrain,
            from,
            deposit.position,
            elevation,
        ))
    }

    /// TRN-FREE-5.3: ticks of the fine approach from `field` to the object
    /// on `spot`.
    fn approach_ticks(&self, field: Point, spot: Point, hunter: bool) -> u32 {
        let Some(steps) = self.approach_steps(field, spot, hunter) else {
            return 0;
        };
        let from = grid::field_sample(field);
        movement::lattice_walk_ticks(&self.terrain, self.settings.settler_speed, from, &steps)
    }

    /// The sample the fine approach from `field` to the object on `spot`
    /// ends at, if there is an object.
    fn approach_end(&self, field: Point, spot: Point, hunter: bool) -> Option<Point> {
        let steps = self.approach_steps(field, spot, hunter)?;
        Some(
            steps
                .last()
                .copied()
                .unwrap_or_else(|| grid::field_sample(field)),
        )
    }

    /// TRN-FREE-5: where an outdoor worker stands on the height lattice
    /// while he walks his fine approach, works at the object and takes the
    /// ware: the sample he last reached, the next one and the progress
    /// toward it (of [`movement::ONE`]), and the lattice direction he faces.
    /// `None` in his other steps, and where he made no fine approach (no
    /// object at the spot); then he stands on his field.
    pub fn fine_position(&self, settler: &Settler) -> Option<FinePosition> {
        let work = match &settler.job {
            SettlerJob::Worker { work, .. } => work,
            // TRN-FREE-6.4a, T24: a builder sent home from his position.
            SettlerJob::Leaving { .. } => return self.leaving_fine_position(settler),
            // MIL-ATK-6.1a: an attacker's step off his waiting field's point.
            SettlerJob::Attack { .. } => return self.attacker_fine_position(settler),
            _ => return None,
        };
        let field = settler.walker.at;
        let hunter = match &settler.job {
            SettlerJob::Worker { target, .. } => self
                .target_building(*target)
                .and_then(|b| self.building_types.get(&b.building_type))
                .is_some_and(|k| k.cycle == WorkCycle::Hunter),
            _ => false,
        };
        let facing = |at: Point, spot: Point| {
            self.deposits
                .get(&spot)
                .filter(|d| d.position != at)
                .map(|d| movement::direction_toward(at, d.position))
        };
        match *work {
            Work::Approach { spot } => {
                let steps = self.approach_steps(field, spot, hunter)?;
                let from = grid::field_sample(field);
                let total = movement::lattice_walk_ticks(
                    &self.terrain,
                    self.settings.settler_speed,
                    from,
                    &steps,
                );
                let elapsed = total.saturating_sub(settler.timer);
                let (at, next, progress) = movement::lattice_walk_at(
                    &self.terrain,
                    self.settings.settler_speed,
                    from,
                    &steps,
                    elapsed,
                );
                let facing = match next {
                    Some(n) => Some(movement::direction_toward(at, n)),
                    None => facing(at, spot),
                };
                Some(FinePosition {
                    at,
                    next,
                    progress_bits: progress.to_bits(),
                    facing,
                })
            }
            Work::AtSpot { spot } => {
                let at = self.approach_end(field, spot, hunter)?;
                Some(FinePosition {
                    at,
                    next: None,
                    progress_bits: 0,
                    facing: facing(at, spot),
                })
            }
            Work::TakeWare {
                stand: Some(at), ..
            }
            | Work::StandBack {
                stand: Some(at), ..
            } => Some(FinePosition {
                at,
                next: None,
                progress_bits: 0,
                facing: None,
            }),
            Work::Return { stand, .. } => {
                let steps = self.return_steps(stand, field);
                let speed = self.settings.settler_speed;
                let total = movement::lattice_walk_ticks(&self.terrain, speed, stand, &steps);
                let elapsed = total.saturating_sub(settler.timer);
                // GFX-ANIM-3.8e (W + 63): in the tick it is ordered it is
                // already walking, at progress 0.
                let (at, next, progress) =
                    movement::lattice_walk_at(&self.terrain, speed, stand, &steps, elapsed);
                Some(FinePosition {
                    at,
                    next,
                    progress_bits: progress.to_bits(),
                    facing: next.map(|n| movement::direction_toward(at, n)),
                })
            }
            _ => None,
        }
    }

    /// ECO-OUT-3.4, 4.6: the free walk with stop short 1 (TRN-FREE-4.2);
    /// waits at the flag while no walk starts. The hunter stops two fields
    /// short of the animal (ECO-OUT-8.1); the fisherman stands on the Coast
    /// spot itself (ECO-FOOD-4.4). The shipwright walks to the field itself
    /// (ECO-FOOD-7.4: the stop point is not traced).
    fn walk_to_spot(&mut self, settler: &mut Settler, spot: Point, cycle: WorkCycle) -> Work {
        let (goal, stop_short) = match cycle {
            WorkCycle::Fisher | WorkCycle::Shipyard => (spot, 0),
            WorkCycle::Hunter => (spot, HUNTER_STOP_SHORT),
            _ => (spot, 1),
        };
        match self.free_walk(settler.walker.at, goal, stop_short) {
            Some(walk) => {
                settler.walker.walk(walk);
                Work::ToSpot { spot }
            }
            None => Work::Out { spot: Some(spot) },
        }
    }

    /// ECO-OUT-8.1, ECO-FOOD-5.3, 5.4: the hunter's walk to the animal on
    /// `spot` has ended (the lattice walk included): the action runs only if
    /// the animal is still reserved for the hut on `at` (and where it was);
    /// else no action, no kill, a ham all the same.
    fn hunter_at_animal(
        &mut self,
        settler: &mut Settler,
        at: Point,
        spot: Point,
        kind: &crate::BuildingType,
    ) -> Work {
        let there = self
            .hunted
            .get(&at)
            .and_then(|&id| self.animal(id))
            .is_some_and(|a| a.at == spot);
        if there {
            settler.timer = kind.production_ticks;
            return Work::AtSpot { spot };
        }
        self.hunted.remove(&at);
        self.produced(at);
        settler.carrying = kind.product;
        self.product_made(kind.product);
        settler.timer = self.settings.drop_ticks;
        let stand = self.approach_end(settler.walker.at, spot, true);
        Work::TakeWare {
            second: false,
            stand,
        }
    }

    /// TRN-FREE-5.6: the lattice line from `stand` back to the point of
    /// `field` (5.1's line, not stopping short).
    fn return_steps(&self, stand: Point, field: Point) -> Vec<Point> {
        movement::fine_approach(&self.terrain, stand, grid::field_sample(field), 0)
    }

    /// TRN-FREE-5.6, ECO-OUT-3.8: the walk home of a worker standing on the
    /// lattice at `stand` after a fine approach: first the fine walk back
    /// to the point of his field (the one his walk out stopped on), then
    /// the free walk to the flag from there; without a stand, or already on
    /// the point, the free walk at once.
    fn walk_home(
        &mut self,
        settler: &mut Settler,
        stand: Option<Point>,
        flag: Point,
        score: i64,
    ) -> Work {
        let Some(stand) = stand else {
            return self.walk_back(settler, flag, score, false);
        };
        let steps = self.return_steps(stand, settler.walker.at);
        if steps.is_empty() {
            return self.walk_back(settler, flag, score, false);
        }
        settler.timer =
            movement::lattice_walk_ticks(&self.terrain, self.settings.settler_speed, stand, &steps);
        Work::Return { stand, score }
    }

    /// ECO-OUT-3.8, 4.8: the free walk to the flag itself. 3.8a (2), (3):
    /// carrying a ware not yet admitted, a walk of one step is ordered only
    /// once a check (`check`: in this tick too) admits it, the wait standing
    /// on his field; a longer walk at once, without a check.
    pub(crate) fn walk_back(
        &mut self,
        settler: &mut Settler,
        flag: Point,
        score: i64,
        check: bool,
    ) -> Work {
        let Some(walk) = self.free_walk(settler.walker.at, flag, 0) else {
            return Work::Back { score };
        };
        let mut score = score;
        if walk.len() == 1 && !self.admitted(settler) {
            if check {
                score = self.ask_flag(settler, flag, score);
            }
            if !self.admitted(settler) {
                return Work::WaitBack { score };
            }
        }
        settler.walker.walk(walk);
        Work::Back { score }
    }

    /// ECO-OUT-3.8a: the entry check of TRN-CARRY-4.4 for the flag of a
    /// worker carrying a ware back (no destination) until it is admitted;
    /// admitted, its place is his (TRN-CARRY-2.4a, 4.3). The wait score
    /// after the check.
    fn ask_flag(&mut self, settler: &Settler, flag: Point, mut score: i64) -> i64 {
        let Some(good) = settler.carrying else {
            return score;
        };
        if self.admitted(settler) || self.flag_at(flag).is_none() {
            return score;
        }
        if self.flag_bid(flag, good, None, &mut score) {
            self.hold_join(settler.number);
        }
        score
    }

    /// ECO-OUT-3.8a: the worker's ware has its place at the flag (or he
    /// carries none).
    pub(crate) fn admitted(&self, settler: &Settler) -> bool {
        settler.carrying.is_none() || self.held_joins.contains_key(&settler.number)
    }

    /// ECO-OUT-4.2 to 4.5, ECO-FOOD-3.3: a spot from a finished search,
    /// with its sapling or new grain field created and reserved.
    fn plant_spot(&mut self, at: Point, kind: &crate::BuildingType) -> PlantSpot {
        let grain = if kind.cycle == WorkCycle::Farm {
            // ECO-FOOD-3.3: the planting good's deposit.
            let planting = kind.planting_good;
            match self
                .deposit_kinds
                .values()
                .find(|k| planting.is_some() && k.good == planting)
            {
                Some(k) => Some(k.clone()),
                None => return PlantSpot::None,
            }
        } else {
            None
        };
        let needs_planting = grain.as_ref().is_some_and(|k| k.needs_plantable);
        let Some(candidates) = self.forester_candidates(at, kind.working_radius, needs_planting)
        else {
            return PlantSpot::Unfinished;
        };
        if candidates.is_empty() {
            return PlantSpot::None;
        }
        let pick = (u64::from(self.rng.draw()) % candidates.len() as u64) as usize;
        let spot = candidates[pick];
        let (deposit_kind, life_left) = match &grain {
            Some(k) => (k.id, k.life_time),
            None => match self.sapling_kind(spot) {
                Some(k) => (k, None),
                None => return PlantSpot::None,
            },
        };
        let reserved_by = self.target_of(at);
        // TRN-FREE-5.4, ECO-OUT-4.5a: every deposit made in a game draws its
        // lattice direction `draw mod 6`, after the spot and kind draws; a
        // kind with the random offset stands one lattice step that way.
        let d = (self.rng.draw() % 6) as u8;
        let sample = grid::field_sample(spot);
        let (columns, rows) = self.terrain.sample_size();
        let offset = self
            .deposit_kinds
            .get(&deposit_kind)
            .is_some_and(|k| k.random_offset);
        let position = offset
            .then(|| grid::neighbour(sample, grid::Direction::ALL[usize::from(d)], columns, rows))
            .flatten()
            .unwrap_or(sample);
        // SIM-START-4.2: a deposit made takes a number.
        self.new_number();
        self.add_deposit(
            spot,
            Deposit {
                kind: deposit_kind,
                position,
                state: DepositState::Sapling,
                reserved_by,
                life_left,
            },
        );
        self.changed(&[spot]);
        PlantSpot::Found(spot)
    }

    /// ECO-OUT-4.7: planting ends; the tree starts growing. False when the
    /// spot's deposit is gone.
    fn plant(&mut self, spot: Point) -> bool {
        let now = self.tick;
        let Some(deposit) = self.deposits.get_mut(&spot) else {
            return false;
        };
        let grow = self
            .deposit_kinds
            .get(&deposit.kind)
            .map_or(0, |k| k.grow_ticks);
        deposit.state = DepositState::Growing {
            ripe_at: now + Tick::from(grow),
        };
        deposit.reserved_by = None;
        self.deposit_version += 1;
        true
    }

    /// ECO-OUT-3.6: the reservation ends; a tree is felled, a stone pile
    /// becomes the next smaller one or, the last, is removed.
    fn harvest(&mut self, building: Point, spot: Point) {
        self.targets.remove(&building);
        // GFX-ANIM-3.8: harvested in time, the armed removal is not needed.
        self.removal_timers.remove(&spot);
        let now = self.tick;
        let Some(deposit) = self.deposits.get_mut(&spot) else {
            return;
        };
        deposit.reserved_by = None;
        let next = self.deposit_kinds.get(&deposit.kind).and_then(|k| k.next);
        match next {
            Some(next) => {
                // ECO-OUT-3.6 (8d46644): the smaller pile is a new deposit on
                // the same field and position (no draw), taking the next
                // object number now, before the stone's at the pick-up.
                deposit.kind = next;
                self.new_number();
            }
            None => {
                deposit.state = DepositState::Removing {
                    since: now,
                    gone_at: now + REMOVAL_TICKS,
                }
            }
        }
        self.deposit_version += 1;
    }

    /// ECO-OUT-3.11: the building on `at` produced. UI-GAME-8.5e: the counter of
    /// its type's first good gets +1.
    fn produced(&mut self, at: Point) {
        let index = self.terrain.index(at);
        let building = self.buildings[index].as_mut().expect("a building");
        let made = self
            .building_types
            .get(&building.building_type)
            .and_then(|kind| kind.product)
            .map(|good| (building.owner, good));
        building.productivity = if building.productivity < 0 {
            PRODUCTIVITY_FIRST
        } else {
            (building.productivity + PRODUCTIVITY_STEP).min(PRODUCTIVITY_MAX)
        };
        // T24: the float32 value, the same step in float32.
        let value = f32::from_bits(building.productivity_bits);
        let value = if value < 0.0 {
            STAT_FIRST
        } else {
            (value + STAT_STEP).min(STAT_MAX)
        };
        building.productivity_bits = value.to_bits();
        if let Some((owner, good)) = made {
            self.count_production(owner, good);
        }
    }

    /// ECO-OUT-2.6: a removed building's reservation ends.
    pub(crate) fn release_target(&mut self, building: Point) {
        if let Some(found) = self.targets.remove(&building) {
            if let Some(deposit) = self.deposits.get_mut(&found) {
                deposit.reserved_by = None;
            }
        }
        if let Some(id) = self.hunted.remove(&building) {
            if let Some(Some(animal)) = self.animals.get_mut(id) {
                animal.hunted_by = None;
                // ECO-FOOD-5.7 (T62): released while standing at its hunted
                // walk's end, it is waiting again in the removal's tick with
                // the wait its walk left, 0; it spends no update hunted.
                if animal.state == crate::AnimalState::Hunted {
                    animal.state = crate::AnimalState::Waiting { left: 0 };
                }
            }
        }
        self.search_timers.remove(&building);
        self.fish_fields.remove(&building);
        self.inputs.remove(&building);
        self.ore_fields.remove(&building);
    }

    /// A worker's product exists (ECO-WORK-3.2, ECO-OUT, ECO-FOOD): it is
    /// counted as made. Its number comes with the pick-up (`take_in_hand`).
    fn product_made(&mut self, good: Option<u32>) {
        self.made(good, 1);
    }

    /// SIM-START-4.4 f' (Q-263 d): a produced ware takes its number in the
    /// first tick of its worker's pick-up action after the work, as he takes
    /// it in hand (plank 90520 at 3505).
    fn take_in_hand(&mut self, settler: &Settler) {
        if settler.carrying.is_some() {
            self.new_number();
        }
    }

    /// ECO-REQ-4.2: a worker of `profession` put into the finished building
    /// on `at` as one that has just arrived (a mission's settler, a restored
    /// game's worker); its id.
    pub(crate) fn seat_worker(&mut self, player: PlayerId, profession: u32, at: Point) -> usize {
        let target = self.target_of(at).expect("a building");
        let index = self.terrain.index(at);
        let b = self.buildings[index].as_mut().expect("a building");
        b.workers += 1;
        b.active = true;
        // SIM-START-4.2: a settler made.
        let number = self.new_number();
        let id = self.add_settler(Settler {
            number,
            player,
            profession,
            list: crate::SettlerList::Workers,
            walker: crate::movement::Walker::new(at),
            job: SettlerJob::Worker {
                target,
                work: Work::Begin,
            },
            timer: 0,
            carrying: None,
        });
        self.file_input_orders(at);
        id
    }

    /// A worker arrived in its building (ECO-REQ-4.2).
    /// ECO-WORK-3.8, 3.8a (T-door-4): the worker of the building on `at`
    /// goes out to its flag: from a door field other than the building's
    /// own, a step of its own back onto the building's field first, then
    /// the walk to the flag from progress 0.
    fn walk_out(&self, settler: &mut Settler, at: Point, flag: Point) {
        let door = self.door_field(at);
        if door != at && settler.walker.at == door {
            settler.walker.walk([at, flag]);
            settler.walker.pauses.push(at);
        } else {
            settler.walker.walk([flag]);
        }
    }

    /// ECO-WORK-3.8, 3.8b (T-door-4): the worker comes back into the
    /// building on `at`: the walk ends on the building's field and a step
    /// of its own from progress 0 takes it to a door field other than the
    /// building's own, where it waits.
    fn walk_in(&self, settler: &mut Settler, at: Point) {
        let door = self.door_field(at);
        if door != at {
            settler.walker.walk([at, door]);
            settler.walker.pauses.push(at);
        } else {
            settler.walker.walk([at]);
        }
    }

    pub(crate) fn worker_arrived(&mut self, settler: &mut Settler, target: Target) {
        settler.job = SettlerJob::Worker {
            target,
            work: Work::Begin,
        };
        // ECO-WORK-3.1a: in a building that works inside, the worker stands
        // in the arrival tick and one more; its first check comes after.
        let inside = self
            .target_building(target)
            .is_some_and(|b| self.building_types[&b.building_type].cycle.works_inside());
        if inside {
            settler.timer = ARRIVAL_STAND;
        }
        // ECO-REQ-4.2 (T25): the building files its input orders in its own
        // update, in the next tick's buildings' part.
        self.orders_due.push(target);
    }
}

impl World {
    /// FMT-SAV-3.5a (b, 2026-11-09, Q-257): the worker seated in the
    /// building `o.at` (as one who has just arrived) is out on the land in
    /// his job's main state again: at his spot in his work action with the
    /// saved ticks left (`Some(true)`), or on his fine approach to it at the
    /// saved point and progress (`Some(false)`). He stands on `o.field`, the
    /// field his walk out ended on (ECO-OUT-3.4). What he works at keeps its
    /// reservation for the building (ECO-OUT-2.6): a tree or stone pile he
    /// harvests (and the farm's grain) is the building's target, a forester's
    /// (or the farm's sowing) spot holds the deposit made for it, not growing
    /// yet (ECO-OUT-4.5); a woodcutter's tree has its removal armed as at the
    /// work's start, so it falls 5 ticks after the action's end even if he is
    /// interrupted (ECO-OUT-3.5). A shipwright works the ship on his field
    /// (ECO-FOOD-7.4a), a fisherman fishes on his coast spot (ECO-FOOD-4.5).
    /// `None`, and he stays seated, for any other job (the hunter), a
    /// spot without its deposit or ship, or a walk that is not the straight
    /// lattice line our approach walks (TRN-FREE-5.1).
    pub(crate) fn resume_outdoor(&mut self, o: &crate::RestoredOutdoor) -> Option<bool> {
        if !(self.terrain.contains(o.field) && self.terrain.contains(o.spot)) {
            return None;
        }
        let building = *self.building_at(o.at)?;
        let kind = self.building_types.get(&building.building_type)?.clone();
        let target = self.target_of(o.at)?;
        let index = self.settlers.iter().position(|s| {
            s.as_ref().is_some_and(|s| {
                matches!(&s.job, SettlerJob::Worker { target, work: Work::Begin }
                    if target.at == o.at)
            })
        })?;
        let at_spot = |w: &mut World, left: u32| {
            let settler = w.settlers[index].as_mut().expect("found");
            settler.walker = movement::Walker::new(o.field);
            settler.timer = left;
            settler.job = SettlerJob::Worker {
                target,
                work: Work::AtSpot { spot: o.spot },
            };
            Some(true)
        };
        let working = o.left.filter(|&l| l > 0 && o.walk.is_none());
        // FMT-SAV-3.5a (b, 2026-11-11): a farmer going out to harvest.
        if let Some(leg) = o.leg {
            return (matches!(kind.cycle, WorkCycle::Farm | WorkCycle::Gatherer) && o.state == 33)
                .then(|| self.resume_harvest_out(o, leg, index, target))
                .flatten();
        }
        // ECO-OUT-4, ECO-FOOD-3.3: the forester plants; the farm sows in its
        // planting state (34) and harvests in its main one (33).
        let planting = match kind.cycle {
            WorkCycle::Forester => true,
            WorkCycle::Farm => o.state == 34,
            WorkCycle::Gatherer => false,
            WorkCycle::Shipyard => {
                self.resume_ship_step(building.owner, o.spot)?;
                return at_spot(self, working?);
            }
            // ECO-FOOD-4.4, 4.5: the fisherman stands on his coast spot
            // itself; the hut's fish field is the first fish around it, as
            // at his trip's start (4.1).
            // FMT-SAV-3.5a (b, 2026-11-09 c), ECO-OUT-8.1: a hunter at work.
            WorkCycle::Hunter => return self.resume_hunter(o, index, target),
            WorkCycle::Fisher if o.field == o.spot => {
                let left = working?;
                match self.first_fish_around(o.spot, kind.product?) {
                    Some(field) => self.fish_fields.insert(o.at, field),
                    None => self.fish_fields.remove(&o.at),
                };
                return at_spot(self, left);
            }
            // (2026-11-09 c): on the rough walk out to his coast spot (k = 0,
            // ECO-FOOD-4.4), the fish field taken as at the trip's start.
            WorkCycle::Fisher if o.rough.is_some() && o.left.is_none() && o.walk.is_none() => {
                let rough = o.rough.as_ref().expect("checked");
                let walker = self.resumed_rough_walk(o.field, o.spot, rough)?;
                match self.first_fish_around(o.spot, kind.product?) {
                    Some(field) => self.fish_fields.insert(o.at, field),
                    None => self.fish_fields.remove(&o.at),
                };
                let settler = self.settlers[index].as_mut().expect("found");
                settler.walker = walker;
                settler.timer = 0;
                settler.job = SettlerJob::Worker {
                    target,
                    work: Work::ToSpot { spot: o.spot },
                };
                return Some(false);
            }
            _ => return None,
        };
        let deposit = self.deposits.get(&o.spot)?;
        if deposit.reserved_by.is_some_and(|t| t.at != o.at) {
            return None;
        }
        let mut walker = movement::Walker::new(o.field);
        let (work, timer) = match (o.left, &o.walk, &o.rough) {
            (Some(left), None, None) if left > 0 => (Work::AtSpot { spot: o.spot }, left),
            (None, Some(walk), None) => {
                let left = self.approach_left_from(o.field, o.spot, walk, false)?;
                (Work::Approach { spot: o.spot }, left)
            }
            // FMT-SAV-3.5a (b, 2026-11-09 c), ECO-OUT-3.4, 4.6: on the rough
            // part of the walk out, walking to the last field before the
            // object; then his fine approach as after any walk out.
            (None, None, Some(rough)) => {
                walker = self.resumed_rough_walk(o.field, o.spot, rough)?;
                (Work::ToSpot { spot: o.spot }, 0)
            }
            _ => return None,
        };
        let deposit = self.deposits.get_mut(&o.spot).expect("checked");
        deposit.reserved_by = Some(target);
        if planting {
            deposit.state = DepositState::Sapling;
        } else {
            self.targets.insert(o.at, o.spot);
        }
        self.deposit_version += 1;
        if let (Work::AtSpot { .. }, false) = (work, planting) {
            // ECO-OUT-3.5: the action's last tick is the (timer)th after the
            // load, tick T + timer - 1; it began production time before, and
            // the removal was armed then for work time + 5 after it.
            let began = (self.tick + Tick::from(timer))
                .saturating_sub(1 + Tick::from(kind.production_ticks));
            self.arm_removal_from(began, kind.cycle, kind.production_ticks, o.spot);
        }
        let settler = self.settlers[index].as_mut().expect("found");
        settler.walker = walker;
        settler.timer = timer;
        settler.job = SettlerJob::Worker { target, work };
        Some(matches!(work, Work::AtSpot { .. }))
    }

    /// FMT-SAV-3.5a (b, 2026-11-11, E-sav-014): a farmer of a save going out
    /// to harvest (harvest step 8) finishes the saved step of his walk out
    /// (ECO-OUT-3.3): on the door step (`leg` 1, from the door field onto
    /// the farm's field) and then the walk to the flag as a walk of its own
    /// from progress 0 (ECO-WORK-3.8b), or on the walk to the flag (`leg`
    /// 0, from the farm's field). From the flag on he walks to the farm's
    /// reserved grain on `o.spot` as after any walk out (ECO-OUT-3.4), which
    /// keeps its reservation for the farm (ECO-OUT-2.6). (2026-11-12,
    /// E-sav-016) A woodcutter or stonemason in step 8 the same way: their
    /// routine runs the same harvest state; starting on the building's own
    /// field, theirs is leg 0. `None`, and he stays seated, for a walk that
    /// is not its leg's, or no object on the spot or one another building
    /// holds.
    fn resume_harvest_out(
        &mut self,
        o: &crate::RestoredOutdoor,
        leg: u32,
        index: usize,
        target: Target,
    ) -> Option<bool> {
        let (door, flag) = (self.door_field(o.at), self.flag_of(o.at));
        let (from, to) = match leg {
            1 if door != o.at => (door, o.at),
            0 => (o.at, flag),
            _ => return None,
        };
        let rough = o.rough.as_ref().filter(|r| r.stop_short == 0)?;
        if o.field != from {
            return None;
        }
        let mut walker = self.resumed_rough_walk(from, to, rough)?;
        if to == o.at {
            walker.path.push_back(flag);
            walker.pauses.push(o.at);
        }
        let deposit = self.deposits.get_mut(&o.spot)?;
        if deposit.reserved_by.is_some_and(|t| t.at != o.at) {
            return None;
        }
        deposit.reserved_by = Some(target);
        self.targets.insert(o.at, o.spot);
        self.deposit_version += 1;
        let settler = self.settlers[index].as_mut().expect("found");
        settler.walker = walker;
        settler.timer = 0;
        settler.job = SettlerJob::Worker {
            target,
            work: Work::Out { spot: Some(o.spot) },
        };
        Some(false)
    }

    /// FMT-SAV-3.5a (b, 2026-11-11, E-sav-014): a farmer of a save waiting
    /// inside in his harvest step (7) goes on with the wait for a target
    /// (ECO-OUT-3.2, ECO-FOOD-3.2) from the saved countdown, not from the
    /// start of his cycle: his first check comes `countdown + 1` ticks after
    /// the load, as the countdown falls to 0 and is checked in the tick
    /// after. The farm's reserved grain (3.4b 1) stays its target, reserved
    /// for it (ECO-OUT-2.6), unless another building holds it. (2026-11-12,
    /// E-sav-016) A woodcutter or stonemason waiting in step 7 the same way,
    /// with his building's reserved tree or stone pile. (2026-11-09
    /// c, from code) A hunter waiting inside for an animal (hunt step 7)
    /// goes on with his wait the same way (ECO-FOOD-5.3). False, and he
    /// stays seated, for another cycle (which of the worker's two saved
    /// states runs is the building's), a countdown above the wait's start
    /// or a worker not seated in the building.
    pub(crate) fn resume_waiting(&mut self, w: &crate::RestoredWaiting) -> bool {
        let Some(building) = self.building_at(w.at).copied() else {
            return false;
        };
        let Some(cycle) = self
            .building_types
            .get(&building.building_type)
            .map(|k| k.cycle)
        else {
            return false;
        };
        let countdown = match cycle {
            WorkCycle::Farm | WorkCycle::Gatherer => w.harvest,
            WorkCycle::Hunter => w.hunt,
            _ => None,
        };
        let Some(countdown) = countdown.filter(|&c| c <= CHECK_COUNTDOWN) else {
            return false;
        };
        let Some(target) = self.target_of(w.at) else {
            return false;
        };
        let Some(index) = self.settlers.iter().position(|s| {
            s.as_ref().is_some_and(|s| {
                matches!(&s.job, SettlerJob::Worker { target, work: Work::Begin }
                    if target.at == w.at)
            })
        }) else {
            return false;
        };
        if let (WorkCycle::Farm | WorkCycle::Gatherer, Some(spot)) = (cycle, w.target) {
            if let Some(deposit) = self.deposits.get_mut(&spot) {
                if deposit.reserved_by.is_none_or(|t| t.at == w.at) {
                    deposit.reserved_by = Some(target);
                    self.targets.insert(w.at, spot);
                    self.deposit_version += 1;
                }
            }
        }
        let settler = self.settlers[index].as_mut().expect("found");
        settler.timer = 0;
        settler.job = SettlerJob::Worker {
            target,
            work: Work::WaitTarget { countdown },
        };
        true
    }

    /// FMT-SAV-3.5a (b, 2026-11-09 c, from code, no sample), ECO-OUT-8.1: a
    /// hunter of a save out at work, his animal (the hut's reserved target)
    /// held for the hut as it stands (hunted, or shot: ours keeps it until
    /// the work ends): hunt step 10, working with the saved ticks left, at
    /// the animal's field; step 9 on the lattice walk to it (walk step 1),
    /// from the saved step and progress; step 9 on the free walk to its walk
    /// goal (walk step 0), the rough walk on from the saved step. `None` (he
    /// is seated inside) without his animal or when the walk does not fit.
    fn resume_hunter(
        &mut self,
        o: &crate::RestoredOutdoor,
        index: usize,
        target: Target,
    ) -> Option<bool> {
        let id = o.animal?;
        let animal = self.animals.get(id)?.as_ref()?;
        if animal.hunted_by.is_some_and(|t| t.at != o.at) || self.hunted.contains_key(&o.at) {
            return None;
        }
        let field = animal.at;
        let (work, timer, walker) = match (o.left, &o.walk, &o.rough) {
            (Some(left), None, None) if left > 0 => (
                Work::AtSpot { spot: field },
                left,
                movement::Walker::new(o.field),
            ),
            (None, Some(walk), None) => (
                Work::Approach { spot: field },
                self.approach_left_from(o.field, field, walk, true)?,
                movement::Walker::new(o.field),
            ),
            (None, None, Some(rough)) => (
                Work::ToSpot { spot: o.spot },
                0,
                self.resumed_rough_walk(o.field, o.spot, rough)?,
            ),
            _ => return None,
        };
        let animal = self.animals[id].as_mut().expect("checked");
        animal.hunted_by = Some(target);
        animal.state = crate::AnimalState::Hunted;
        self.hunted.insert(o.at, id);
        let settler = self.settlers[index].as_mut().expect("found");
        settler.walker = walker;
        settler.timer = timer;
        settler.job = SettlerJob::Worker { target, work };
        Some(matches!(work, Work::AtSpot { .. }))
    }

    /// FMT-SAV-3.5a (b, 2026-11-09 c), 3.5h: the walker of a saved rough walk
    /// out: on the running step from `field` (the list's point after its
    /// index) to the index's point at the saved progress and rate, then the
    /// points before the index down to the stop-short count's (TRN-FREE-4.2:
    /// it ends k fields before the target, which must be `spot`). `None` for
    /// a list that does not hold together (a step between fields that are
    /// not neighbours, the step's start not `field`, no step under way).
    fn resumed_rough_walk(
        &self,
        field: Point,
        spot: Point,
        rough: &crate::RestoredRoughWalk,
    ) -> Option<movement::Walker> {
        let (path, i, k) = (&rough.path, rough.index as usize, rough.stop_short as usize);
        if i + 1 >= path.len() || k > i || path[i + 1] != field || path.first() != Some(&spot) {
            return None;
        }
        if !(0.0..=1.0).contains(&rough.progress) {
            return None;
        }
        let adjacent = |a: Point, b: Point| (0..6).any(|d| self.terrain.neighbour(a, d) == Some(b));
        if !path[..=i + 1].windows(2).all(|w| adjacent(w[0], w[1])) {
            return None;
        }
        let goal = path[i];
        let rate = if rough.rate > 0.0 && rough.rate.is_finite() {
            rough.rate
        } else {
            movement::step_rate(
                &self.terrain,
                self.settings.settler_speed,
                field,
                goal,
                true,
            )
        };
        let mut walker = movement::Walker::new(field);
        walker.walk(path[k..i].iter().rev().copied());
        walker.next = Some(goal);
        walker.progress_bits = rough.progress.to_bits();
        walker.rate_bits = rate.to_bits();
        Some(walker)
    }

    /// TRN-FREE-5.1 to 5.3, FMT-SAV-3.5a (b): the ticks left of the fine
    /// approach from `field` to the object on `spot` for a worker at the
    /// saved `walk`'s point and progress: the walk must be our approach's
    /// line (from the field's sample to the object's position, ending
    /// `elevation` steps short of it; for a hunter to the point of the
    /// animal's field, ending 2 short, ECO-OUT-8.1), and the ticks walked are
    /// those whose step and progress are the saved ones (the nearest
    /// progress).
    fn approach_left_from(
        &self,
        field: Point,
        spot: Point,
        walk: &crate::RestoredFineWalk,
        hunter: bool,
    ) -> Option<u32> {
        let point = if hunter {
            grid::field_sample(spot)
        } else {
            self.deposits.get(&spot)?.position
        };
        let (start, done) = walk.start_and_done()?;
        let from = grid::field_sample(field);
        if start != from || walk.path.first() != Some(&point) {
            return None;
        }
        if movement::fine_approach(&self.terrain, from, point, 0) != walk.line() {
            return None;
        }
        let steps = self.approach_steps(field, spot, hunter)?;
        let done = done as usize;
        let (at, goal) = (
            if done == 0 {
                from
            } else {
                steps.get(done - 1).copied()?
            },
            *steps.get(done)?,
        );
        let speed = self.settings.settler_speed;
        let total = movement::lattice_walk_ticks(&self.terrain, speed, from, &steps);
        let elapsed = (0..total)
            .filter_map(|e| {
                let (a, next, p) = movement::lattice_walk_at(&self.terrain, speed, from, &steps, e);
                (a == at && next == Some(goal)).then_some((e, (p - walk.progress).abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))?
            .0;
        Some(total - elapsed)
    }

    /// ECO-FOOD-7.4a, FMT-SAV-3.5a (b): the work step under way on the
    /// player's ship on `spot` (being built): its amount as fixed at the
    /// step's start, `1 / (construct time x frame planks)` while the frame
    /// has work left, else with the main planks. `None` without that ship.
    fn resume_ship_step(&mut self, owner: crate::PlayerId, spot: Point) -> Option<()> {
        let rules = self.ship_rules;
        let ship = self
            .ships
            .iter_mut()
            .find(|s| s.at == spot && s.owner == owner && !s.finished())?;
        let planks = if ship.frame_work != 0 {
            rules.frame_planks
        } else {
            rules.main_planks
        };
        let amount = 1.0f32 / (rules.construct_ticks * planks) as f32;
        ship.step_amount = Some(amount.to_bits());
        Some(())
    }
}

fn add(list: &mut Vec<(u32, u32)>, good: u32, n: u32) {
    match list.iter_mut().find(|(g, _)| *g == good) {
        Some((_, c)) => *c += n,
        None => list.push((good, n)),
    }
}

#[cfg(test)]
mod tests;
