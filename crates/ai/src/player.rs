//! One computer player: its planning objects and its per-tick think
//! (`specs/AI-overview.md`, `AI-misc.md`).

use std::collections::BTreeMap;

use s2_sim::grid::Point;
use s2_sim::{BuildingSize, Command, PlayerId, World};

use crate::cells::{cell_of, CellPos, Cells};
use crate::config::{AiConfig, Profile};
use crate::construction::Construction;
use crate::expansion::Expansion;
use crate::maps::{mcell_of, MilitaryMap, TerritoryMap};
use crate::plan::Plan;
use crate::resources::ResourceMap;
use crate::roads::Street;
use crate::schedule::{self, due, Part};
use crate::snapshot::{Snapshot, SLOTS};
use crate::view::{Economy, STOCK_CYCLE};

/// AI-CON-1.8: the statistics count the owned fields every 601 ticks.
const STATISTICS_CYCLE: u64 = 601;

/// AI-CON-1.8: the fields the player owns, water excluded, and (T76) the
/// fields without a continent (index -1: mountains, other blocked ground).
pub(crate) fn owned_land(world: &World, player: PlayerId) -> u32 {
    let terrain = world.terrain();
    terrain
        .fields()
        .filter(|&f| {
            world.field_owner(f) == Some(player)
                && !terrain.has(f, s2_sim::map::state::WATER)
                && world.continent_of(f).is_some_and(|c| c != -1)
        })
        .count() as u32
}

/// AI-CON-1.2: the four protection flags of a building record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Protect {
    /// Soldier limit and territory range above 0.
    pub military: bool,
    /// The type's depot flag: the headquarters, storehouses and harbours.
    pub depot: bool,
    /// No product and a planting good: the forester, not the farm (T46).
    pub planting: bool,
    pub harbour: bool,
}

impl Protect {
    /// AI-CON-1.2: the flags of a building of the type `t`.
    pub fn of(t: &s2_sim::BuildingType) -> Self {
        Self {
            military: t.soldier_limit > 0 && t.territory_range > 0,
            depot: t.warehouse,
            planting: t.product.is_none() && t.planting_good.is_some(),
            harbour: t.size == BuildingSize::Harbor,
        }
    }

    pub fn any(self) -> bool {
        self.military || self.depot || self.planting || self.harbour
    }
}

/// AI-CON-1.1: the list of a cell a building is in; it is in exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// A depot that is not a harbour: a storehouse or the headquarters
    /// (AI-CON-1.1, corrected 2026-10-24).
    Depot,
    /// Everything else (AI-CON-1.1, Q-160): farms and mines included.
    Ordinary,
    /// A forester (a planting building without a product) or a hunter: each
    /// is a single reference of its cell, not an ordinary building.
    Planter,
    /// A building with a soldier limit and a territory range.
    Military,
    Harbour,
}

/// AI-CON-1.2: the AI's record of one of its buildings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingRec {
    pub at: Point,
    pub building_type: u32,
    /// The building's creation number, to tell a new building on the same
    /// field from the old one.
    pub serial: u32,
    pub cell: CellPos,
    pub protect: Protect,
    pub category: Category,
    /// The type is a catapult (`bp_setCatapult`).
    pub catapult: bool,
}

/// One part run, recorded when tracing is on (for the schedule tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub counter: u64,
    pub player: PlayerId,
    pub part: Part,
}

/// What a part of a think reads and writes. Its world and snapshot are
/// those of the think's view (AI-2.1a): the commands that earlier parts and
/// players issued in the tick are in effect in them.
pub struct Ctx<'a> {
    pub world: &'a World,
    pub config: &'a AiConfig,
    pub snapshot: &'a Snapshot,
    pub military: &'a [MilitaryMap],
    pub alive: &'a [bool; SLOTS],
    /// AI-2.7: the simulation's generator, lent for the think.
    pub rng: &'a mut s2_sim::Rng,
    /// TRN-CARRY-5.3a: the players' stored route costs, lent for the think.
    pub routes: &'a mut s2_sim::RouteStore,
    pub out: &'a mut Vec<Command>,
    pub counter: u64,
    pub trace: Option<&'a mut Vec<Run>>,
}

/// What a think lends each of its parts besides the view.
pub struct Lent<'a> {
    pub config: &'a AiConfig,
    pub military: &'a [MilitaryMap],
    pub alive: &'a [bool; SLOTS],
    pub rng: &'a mut s2_sim::Rng,
    pub routes: &'a mut s2_sim::RouteStore,
    /// The tick's commands so far, every player's.
    pub out: &'a mut Vec<Command>,
    pub counter: u64,
    pub trace: Option<&'a mut Vec<Run>>,
}

impl Lent<'_> {
    /// The context of one part, reading the world through `view`.
    pub fn ctx<'b>(&'b mut self, view: &'b crate::view::ThinkView) -> Ctx<'b> {
        Ctx {
            world: view.world(),
            config: self.config,
            snapshot: view.snapshot(),
            military: self.military,
            alive: self.alive,
            rng: &mut *self.rng,
            routes: &mut *self.routes,
            out: &mut *self.out,
            counter: self.counter,
            trace: self.trace.as_deref_mut(),
        }
    }
}

/// A computer player (AI-MISC-2).
#[derive(Debug, Clone)]
pub struct AiPlayer {
    pub id: PlayerId,
    /// 1 to 3: weak, normal, strong.
    pub profile_number: u8,
    pub profile: Profile,
    /// AI-MISC-2.1: where the player started, and the landmass of that
    /// field (AI-CON-1.1b (c): its stored continent index at the
    /// activation, never updated).
    pub home: Point,
    pub home_landmass: i32,
    /// AI-CON-3.5: the conquered continents, the landmasses of the
    /// player's cells as of the last run of the 1323-tick part (AI-2.3).
    pub conquered: Vec<i32>,
    /// AI-CON-3.5, AI-MIL-3.2: the attempt counter of each 16 x 16 cell
    /// coordinate (0 when absent): halved when a waiting-need target there
    /// is finished, raised by 1 when one is cancelled.
    pub attempts: BTreeMap<CellPos, u32>,
    /// False once re-initialised (AI-MISC-2.3).
    pub active: bool,
    pub cells: Cells,
    pub records: BTreeMap<Point, BuildingRec>,
    pub territory: TerritoryMap,
    pub resources: ResourceMap,
    pub plan: Plan,
    pub build: Construction,
    pub expansion: Expansion,
    pub street: Street,
    /// `stock` and `stock2` as of the start of this tick's think.
    pub econ: Economy,
    /// AI-CON-1.7: the player's updates so far, the stock counts' cycle.
    pub stock_updates: u64,
    /// AI-CON-1.7 (T92): the counts taken at the end of the last think for
    /// this update's refresh: the state the previous tick left.
    pub next_stock: Option<BTreeMap<u32, u32>>,
    pub next_moving: Option<BTreeMap<u32, u32>>,
    /// AI-CON-2: need creation stage, and the doubling stock-check counter.
    pub need_stage: u8,
    pub stock_check: u32,
    /// The statistics value S and the tick it was taken (Q-073).
    pub statistic: (u64, u32),
    /// Cursors of the periodic parts.
    /// AI-CON-7.1, 7.2: the list indices of the forester and hunter runs.
    pub forester_index: usize,
    pub hunter_index: usize,
    /// AI-CON-7.3: the grid position of the stone pit and woodcutter run.
    pub pit_cursor: u32,
    pub destruction_cursor: u32,
    pub flag_cursor: u32,
    pub control_cursor: usize,
    pub locksmith_last: usize,
    /// AI-MIL-4.1: runs of military building creation to wait.
    pub military_cooldown: u32,
    /// AI-MIL-2.2: runs of military settings before the allocation changes.
    pub settings_countdown: u32,
    pub attack: crate::military::Attack,
    pub catapult_scan: crate::military::CatapultScan,
    /// AI-CON-10.4: the step of the goods arrangement cycle (0 to 9).
    pub goods_step: u32,
    /// AI-MIL-6.2, 6.3: the weapon and coin schemes' (t0, k).
    pub weapon_gate: crate::military::Gate,
    pub coin_gate: crate::military::Gate,
    /// AI-MIL-4.5a: the entry of the world's military list the next upgrade
    /// run looks at.
    pub upgrade_cursor: u32,
    /// SIM-START-4.4 b (190c1e1): the sites placed in this tick and how
    /// many records each one's announce makes; their placement numbered
    /// them already, so the announce in the next update takes none.
    pub prenumbered: BTreeMap<Point, u32>,
}

impl AiPlayer {
    /// AI-MISC-2.1: activation. The caller announces the buildings the
    /// player has by running [`AiPlayer::update`].
    pub fn new(world: &World, config: &AiConfig, id: PlayerId, profile_number: u8) -> Option<Self> {
        let profile = *config.profile(profile_number)?;
        let terrain = world.terrain();
        let home = world
            .buildings()
            .find(|b| b.owner == id && b.building_type == config.names.castle)
            .or_else(|| world.buildings().find(|b| b.owner == id))
            .map_or((0, 0), |b| b.at);
        Some(Self {
            id,
            profile_number,
            profile,
            home,
            home_landmass: world.continent_of(home).unwrap_or(-1),
            conquered: Vec::new(),
            attempts: BTreeMap::new(),
            active: true,
            cells: Cells::new(terrain.width, terrain.height),
            records: BTreeMap::new(),
            territory: TerritoryMap::new(terrain.width, terrain.height),
            resources: ResourceMap::new(terrain.width, terrain.height),
            plan: Plan::default(),
            build: Construction::default(),
            expansion: Expansion::default(),
            street: Street::default(),
            econ: Economy::default(),
            stock_updates: 0,
            next_stock: None,
            next_moving: None,
            need_stage: 0,
            stock_check: 1,
            statistic: (0, 0),
            forester_index: 0,
            hunter_index: 0,
            pit_cursor: 0,
            destruction_cursor: 0,
            flag_cursor: 0,
            control_cursor: 0,
            locksmith_last: usize::MAX,
            military_cooldown: 0,
            settings_countdown: 0,
            attack: Default::default(),
            catapult_scan: Default::default(),
            goods_step: 0,
            weapon_gate: Default::default(),
            coin_gate: Default::default(),
            upgrade_cursor: 0,
            prenumbered: BTreeMap::new(),
        })
    }

    /// AI-CON-1.7, 1.8: the lagging counts, at the start of every update.
    /// The cycle counter starts at 0 and is advanced before the phase test:
    /// `stock` is summed in the player's 22nd update and every 22 after,
    /// `stock2` in the 24th and every 22 after; until then both read 0. S
    /// every 601 ticks.
    /// AI-CON-1.7 (T92): both counts read the state the previous tick left
    /// (the original's player update is the tick's first step), taken at the
    /// end of the previous think ([`Self::prepare_counts`]).
    pub(crate) fn refresh_counts(&mut self, world: &World, _snapshot: &Snapshot) {
        let tick = world.tick();
        self.stock_updates += 1;
        let n = self.stock_updates;
        if n.is_multiple_of(STOCK_CYCLE) {
            self.econ
                .set_stock(self.next_stock.take().unwrap_or_default());
        }
        if n % STOCK_CYCLE == 2 && n > STOCK_CYCLE {
            self.econ
                .set_moving(self.next_moving.take().unwrap_or_default());
        }
        if tick.is_multiple_of(STATISTICS_CYCLE) {
            self.statistic = (tick, owned_land(world, self.id));
        }
    }

    /// Activation: the statistic of the moment; the stock counts stay 0
    /// until the cycle sums them (AI-CON-1.7).
    pub(crate) fn take_counts(&mut self, world: &World, _snapshot: &Snapshot) {
        self.statistic = (world.tick(), owned_land(world, self.id));
    }

    /// AI-MISC-4.1 and AI-MISC-2.2: the per-tick update. Buildings that are
    /// new to the player are announced, records of buildings that are gone
    /// are dropped. No decision is taken here. Returns how many of the
    /// records made were numbered by their site's placement already.
    pub fn update(&mut self, world: &World, config: &AiConfig, snapshot: &Snapshot) -> u32 {
        let now = world.tick();
        let mine = snapshot.of(self.id);
        // Gone, or replaced by a later building on the same field.
        let gone: Vec<Point> = self
            .records
            .values()
            .filter(|r| !mine.iter().any(|b| b.at == r.at && b.serial == r.serial))
            .map(|r| r.at)
            .collect();
        for at in gone {
            self.remove_record(at, now);
        }
        let mut numbered = 0;
        for b in mine {
            if !self.records.contains_key(&b.at) {
                self.announce(world, config, b, now);
                numbered += self.prenumbered.remove(&b.at).unwrap_or(0);
            }
        }
        // A site whose placement was refused took no numbers.
        self.prenumbered.clear();
        numbered
    }

    /// SIM-START-4.4 b (190c1e1, T13): how many records the announce of a
    /// site of type `building_type` on `at` makes, as the player stands at
    /// the end of its part: its cell's record if the cell is new, the
    /// production record (a type with a product, not a depot or the
    /// hunter's hut) and a need record for each of its needs that finds no
    /// need without a consumer to take (AI-CON-2.5).
    pub fn site_records(
        &self,
        world: &World,
        config: &AiConfig,
        building_type: u32,
        at: Point,
    ) -> u32 {
        let Some(t) = world.building_type(building_type) else {
            return 0;
        };
        let mut records = u32::from(self.cells.get(cell_of(at)).is_none());
        if t.product.is_some() && !t.warehouse && building_type != config.names.hunter {
            records += 1;
        }
        for good in Self::new_building_needs(config, t) {
            if !self
                .plan
                .needs
                .iter()
                .any(|n| n.good == good && n.consumer.is_none())
            {
                records += 1;
            }
        }
        records
    }

    /// AI-MISC-2.2: a new building of the player.
    fn announce(&mut self, world: &World, config: &AiConfig, b: &s2_sim::Building, now: u64) {
        let Some(t) = world.building_type(b.building_type) else {
            return;
        };
        let protect = Protect::of(t);
        let cell = cell_of(b.at);
        let names = &config.names;
        // AI-CON-1.1 (corrected 2026-10-24): military needs the type's
        // soldier limit and territory range; the headquarters has no
        // soldier limit and is a depot.
        let category = if t.size == BuildingSize::Harbor {
            Category::Harbour
        } else if protect.military {
            Category::Military
        } else if t.warehouse {
            Category::Depot
        } else if protect.planting || b.building_type == names.hunter {
            Category::Planter
        } else {
            Category::Ordinary
        };
        self.records.insert(
            b.at,
            BuildingRec {
                at: b.at,
                building_type: b.building_type,
                serial: b.serial,
                cell,
                protect,
                category,
                catapult: t.catapult,
            },
        );
        let has_harbour = category == Category::Harbour || self.counts(cell).3 > 0;
        let entry = self.cells.ensure(cell, now);
        entry.buildings.push(b.at);
        // AI-CON-1.1a (corrected, T49): only a building of the four lists
        // (military, harbour, depot, ordinary) moves the cell's position;
        // the forester and the hunter are single references.
        if category != Category::Planter {
            entry.building_added(world, b.at, now, has_harbour);
        }
        // AI-CON-1.6: a depot ends the wish for one, a harbour likewise.
        match category {
            Category::Depot => entry.depot_wanted = None,
            Category::Harbour => entry.harbour_wanted = None,
            _ => {}
        }
        if b.building_type == names.forester {
            entry.forester = Some(b.at);
        }
        if b.building_type == names.hunter {
            entry.hunter = Some(b.at);
        }
        if t.catapult {
            if let Some(c) = self.territory.get_mut(mcell_of(b.at)) {
                c.catapults += 1;
            }
        }
        // A producer: adopt the planned production of its good in the cell,
        // else it is a new one (a woodcutter ordered by the cells).
        if let (Some(good), false) = (t.product, t.warehouse) {
            // SIM-START-4.4 b (Q-263 a): every building whose type has a
            // product makes its production record with it, whether or not a
            // planned production waits for it, except the hunter's hut
            // (sawmill 90212, T6; hunter's hut 90716 none, T12).
            // AI-CON-2.5a: the hunter's hut has no production at all.
            let hunter = b.building_type == names.hunter;
            if !hunter {
                self.plan.building_record_made();
            }
            if !hunter && !self.plan.prods.iter().any(|p| p.building == Some(b.at)) {
                let planned = self
                    .plan
                    .prods
                    .iter_mut()
                    .find(|p| p.good == good && p.cell == cell && p.building.is_none());
                let production = match planned {
                    Some(p) => {
                        p.building = Some(b.at);
                        p.id
                    }
                    None => {
                        let id = self.plan.add_production(good, cell, None);
                        if let Some(p) = self.plan.prods.iter_mut().find(|p| p.id == id) {
                            p.building = Some(b.at);
                        }
                        id
                    }
                };
                // AI-CON-2.5b (T102): a production given a cell, whatever
                // order placed its building, is tied to the first need in
                // the cell's need list whose good it supplies and that has
                // no production yet.
                let untied = self
                    .plan
                    .production(production)
                    .is_some_and(|p| p.need.is_none());
                let first = self.plan.first_need_in(cell, |g| config.supplies(good, g));
                if let (true, Some(need)) = (untied, first) {
                    if let Some(p) = self.plan.prods.iter_mut().find(|p| p.id == production) {
                        p.need = Some(need);
                    }
                    if let Some(n) = self.plan.need_mut(need) {
                        n.production = Some(production);
                    }
                }
            }
        }
        if matches!(
            category,
            Category::Military | Category::Harbour | Category::Ordinary
        ) {
            self.update_crowded(cell, now);
        }
        // AI-CON-2.5: the needs of the new building, tied to it. A site the
        // player places itself is announced in its next update, before any
        // part that reads the needs.
        let t_now = world.game_seconds();
        for good in Self::new_building_needs(config, t) {
            self.plan.tie_need(good, b.at, t_now);
        }
    }

    /// AI-CON-2.5: the goods a new building of type `t` needs: every input
    /// of its product (not for a hunter), food for a mine whose product is
    /// stone, the planting good of a planting building without a product.
    pub fn new_building_needs(config: &AiConfig, t: &s2_sim::BuildingType) -> Vec<u32> {
        let names = &config.names;
        match (t.product, t.planting_good) {
            (Some(product), _) => {
                let mut goods: Vec<u32> = if t.id == names.hunter {
                    Vec::new()
                } else {
                    config
                        .type_inputs(t.id, product)
                        .iter()
                        .map(|&(g, _)| g)
                        .collect()
                };
                if t.size == BuildingSize::Mine && product == names.stone {
                    goods.push(names.food);
                }
                goods
            }
            (None, Some(planting)) => vec![planting],
            (None, None) => Vec::new(),
        }
    }

    /// AI-CON-9.3 (T84, T111): the destruction run's step before its
    /// demolish command: every need linked to one of the building's
    /// productions and holding a square loses its cell and its square (the
    /// square's slot is free again), so the next arrangement places it again
    /// as a new need. The production stays linked until the building's
    /// record goes.
    pub(crate) fn tear_down_needs(&mut self, at: Point) {
        let prods: Vec<_> = self
            .plan
            .prods
            .iter()
            .filter(|p| p.building == Some(at))
            .map(|p| p.id)
            .collect();
        for n in &mut self.plan.needs {
            if n.square.is_some() && n.production.is_some_and(|p| prods.contains(&p)) {
                n.cell = None;
                n.square = None;
                n.again = true;
            }
        }
    }

    /// AI-CON-1.1: one step of the validity scan; AI-CON-1.1d (T112): a
    /// need whose cell the step removed loses it, so the next arrangement
    /// places it again as a new one; a cell made again later for the same
    /// place starts with no needs.
    pub(crate) fn cell_scan(&mut self, world: &World) {
        self.cells.scan(world, self.id, world.tick());
        let cells = &self.cells;
        for n in &mut self.plan.needs {
            if n.cell.is_some_and(|c| cells.get(c).is_none()) {
                n.cell = None;
            }
        }
    }

    /// AI-MISC-2.2: a building of the player is gone.
    fn remove_record(&mut self, at: Point, now: u64) {
        let Some(rec) = self.records.remove(&at) else {
            return;
        };
        // Its productions are deleted (AI-CON-9.3); a need linked to one
        // loses only the production and keeps its cell and its square
        // ("which removals take the cell away", T111): the needs order of
        // that cell places a new producer there. Only the computer player's
        // own tear-down takes the cell away ([`Self::tear_down_needs`]).
        let gone: Vec<_> = self
            .plan
            .prods
            .iter()
            .filter(|p| p.building == Some(at))
            .map(|p| p.id)
            .collect();
        for id in gone {
            self.plan.remove_production(id);
        }
        // AI-CON-9.3 the building's own needs (T109): the needs whose
        // consumer it is are deleted with it (off the need and waiting
        // lists; a production linked to one loses it).
        let own: Vec<_> = self
            .plan
            .needs
            .iter()
            .filter(|n| n.consumer == Some(at))
            .map(|n| n.id)
            .collect();
        for id in own {
            self.plan.remove_need(id);
        }
        if let Some(cell) = self.cells.get_mut(rec.cell) {
            cell.buildings.retain(|&p| p != at);
            if cell.forester == Some(at) {
                cell.forester = None;
            }
            if cell.hunter == Some(at) {
                cell.hunter = None;
            }
        }
        // AI-CON-1.6: depot wanted when the depot list becomes empty,
        // harbour wanted when the harbour list does.
        let (depots, _, _, harbours) = self.counts(rec.cell);
        if let Some(cell) = self.cells.get_mut(rec.cell) {
            if rec.category == Category::Depot && depots == 0 {
                cell.depot_wanted = Some(now);
            }
            if rec.category == Category::Harbour && harbours == 0 {
                cell.harbour_wanted = Some(now);
            }
        }
        // AI-CON-1.6 (T96): crowded is evaluated again when a military or
        // ordinary building goes, not when a depot or harbour does.
        if matches!(rec.category, Category::Military | Category::Ordinary) {
            self.update_crowded(rec.cell, now);
        }
        // A catapult: the territory cell's count is halved.
        if rec.catapult {
            if let Some(c) = self.territory.get_mut(mcell_of(at)) {
                c.catapults /= 2;
            }
        }
    }

    /// The numbers of (depots, ordinary buildings, military buildings,
    /// harbours) in a cell (AI-CON-1.1).
    /// AI-2.3, AI-CON-3.5: the conquered continents are cleared and
    /// rebuilt as the set of the landmasses of the player's cells
    /// (AI-CON-1.1b (b)).
    pub(crate) fn conquered_continents(&mut self) {
        let mut set: Vec<i32> = self.cells.iter().map(|c| c.landmass).collect();
        set.sort_unstable();
        set.dedup();
        self.conquered = set;
    }

    pub(crate) fn counts(&self, cell: CellPos) -> (u32, u32, u32, u32) {
        let mut n = (0, 0, 0, 0);
        if let Some(entry) = self.cells.get(cell) {
            for at in &entry.buildings {
                match self.records.get(at).map(|r| r.category) {
                    Some(Category::Depot) => n.0 += 1,
                    Some(Category::Ordinary) => n.1 += 1,
                    Some(Category::Military) => n.2 += 1,
                    Some(Category::Harbour) => n.3 += 1,
                    Some(Category::Planter) | None => {}
                }
            }
        }
        n
    }

    /// AI-CON-1.6 *crowded*, evaluated when a military, harbour or ordinary
    /// building is added and when a military or ordinary one is removed: if
    /// 5 x military buildings < ordinary buildings + (1 if the cell has a
    /// harbour) the marker is set, otherwise cleared. A set marker keeps its
    /// time and weight (T96); a clear one set gets the time `now` and the
    /// weight 100 if the cell has a harbour and no military building, else
    /// 1.
    fn update_crowded(&mut self, cell: CellPos, now: u64) {
        let (_, ordinary, military, harbours) = self.counts(cell);
        let crowded = 5 * military < ordinary + u32::from(harbours > 0);
        let weight = if harbours > 0 && military == 0 {
            100
        } else {
            1
        };
        if let Some(entry) = self.cells.get_mut(cell) {
            entry.crowded = match (crowded, entry.crowded) {
                (true, Some(set)) => Some(set),
                (true, None) => Some((now, weight)),
                (false, _) => None,
            };
        }
    }

    /// SIM-START-4.2: the records made so far (needs, production records,
    /// cells), each an object of the simulation.
    pub fn records_made(&self) -> u32 {
        self.plan.made().wrapping_add(self.cells.made())
    }

    /// SIM-START-4.4: the records made since the count `before`, but the
    /// `numbered` ones its sites' placement numbered already (4.4 b,
    /// `Command::PlanBuilding`), take their numbers from the lent counter,
    /// at the end of the player's part, before its commands take effect.
    /// Within the tick only hidden numbers change places by it (records
    /// made in the think come before the commands' objects): no settler's
    /// number differs.
    pub fn number_records(&self, before: u32, numbered: u32, numbers: &mut s2_sim::Numbers) {
        let made = self.records_made().wrapping_sub(before);
        for _ in 0..made.saturating_sub(numbered) {
            numbers.take();
        }
    }

    /// AI-MISC-2.3: delete everything and switch the player off.
    pub fn reinitialise(&mut self) {
        self.cells.clear();
        self.records.clear();
        self.territory.clear();
        self.resources.clear();
        self.plan.clear();
        self.build.clear();
        self.expansion.clear();
        self.street = Street::default();
        self.attack = Default::default();
        self.catapult_scan = Default::default();
        self.prenumbered.clear();
        self.active = false;
    }

    /// AI-2.1, AI-2.3 to 2.6: the think of one tick.
    /// AI-2.2, 2.3: the parts due in this tick, in their order. AI-2.1a:
    /// each part reads the world through `view`, which then takes the
    /// commands the part issued, so the later parts read them in effect.
    pub fn think(&mut self, view: &mut crate::view::ThinkView, lent: &mut Lent) {
        let slot = self.id.0;
        let c = lent.counter;
        self.refresh_counts(view.world(), view.snapshot());
        for (part, period) in schedule::THINK {
            if part == Part::ConstructionSystem {
                self.construction_run(view, lent);
                continue;
            }
            if due(c, slot, period) {
                self.run_viewed(part, view, lent);
            }
            match part {
                // AI-2.4b: the resource system's inner periods.
                Part::ResourceSystem => self.run_group(&schedule::RESOURCE, view, lent),
                Part::ExpansionSystem => self.run_group(&schedule::EXPANSION, view, lent),
                Part::MilitarySystem => self.run_group(&schedule::MILITARY, view, lent),
                Part::DestructionSystem => self.run_group(&schedule::DESTRUCTION, view, lent),
                _ => {}
            }
        }
        // Hook W25: a project's paving and recipe rules (none in the
        // original's rules: nothing runs).
        let project = &lent.config.project_rules;
        if (project.paving.is_some() || !project.recipes.is_empty())
            && due(c, slot, crate::project::PROJECT_PERIOD)
        {
            self.project_rules(&mut lent.ctx(view));
            view.issue(lent.out);
        }
        // AI-CON-1.1c (T52): the resource map's squares, and every 16 counts
        // a cell coordinate, after all computer players have thought (each
        // player reads only its own copy, so refreshing it after its own
        // think is the same).
        self.resources
            .refresh_after_think(view.world(), self.id, lent.config, lent.counter);
        self.prepare_counts(view.world(), view.snapshot());
    }

    /// AI-CON-1.7 (T92): at the end of a think, the counts the next update's
    /// refresh reads, when it is due: the state as this tick leaves it.
    pub(crate) fn prepare_counts(&mut self, world: &World, snapshot: &Snapshot) {
        let n = self.stock_updates + 1;
        if n.is_multiple_of(STOCK_CYCLE) {
            self.next_stock = Some(Economy::count_stock(world, self.id, snapshot.of(self.id)));
        }
        if n % STOCK_CYCLE == 2 && n > STOCK_CYCLE {
            self.next_moving = Some(Economy::count_moving(world, self.id));
        }
    }

    /// AI-2.4, 2.4a: the construction system's run in every tick: its inner
    /// parts first, then one last step: the 50-tick step when no order is
    /// executing, else the executing order's step (AI-CON-5.3a) with its
    /// end handler. An order that ended in the tick before no longer counts
    /// as executing.
    fn construction_run(&mut self, view: &mut crate::view::ThinkView, lent: &mut Lent) {
        self.build.forget_ended();
        self.run_viewed(Part::ConstructionSystem, view, lent);
        self.run_group(&schedule::CONSTRUCTION, view, lent);
        if self.build.exec.is_some() {
            self.run_viewed(Part::ExecutingStep, view, lent);
        } else {
            self.run_group(&[schedule::BUDGET_STEP], view, lent);
        }
    }

    fn run_group(
        &mut self,
        group: &[(Part, u64)],
        view: &mut crate::view::ThinkView,
        lent: &mut Lent,
    ) {
        for &(part, period) in group {
            if due(lent.counter, self.id.0, period) {
                self.run_viewed(part, view, lent);
            }
        }
    }

    /// AI-2.1a: one part through the view, then its commands into the view.
    fn run_viewed(&mut self, part: Part, view: &mut crate::view::ThinkView, lent: &mut Lent) {
        self.run(part, &mut lent.ctx(view));
        view.issue(lent.out);
    }

    /// Run one part: record it when tracing, then do its work.
    fn run(&mut self, part: Part, ctx: &mut Ctx) {
        let run = Run {
            counter: ctx.counter,
            player: self.id,
            part,
        };
        if let Some(trace) = ctx.trace.as_mut() {
            trace.push(run);
        }
        let v = ctx.counter + schedule::SLOT_SHIFT * u64::from(self.id.0);
        match part {
            Part::NeedsArrangement => self.needs_arrangement(ctx),
            Part::NeedsConstruction => self.needs_construction(ctx),
            // AI-CON-1.1c (T52): the resource map is refreshed after the
            // think (`think`), not in this part.
            Part::ResourceSystem => {}
            // AI-CON-1.6: marks that ran out are cleared in this check.
            Part::FullMarkExpiry => self.cells.expire_full(ctx.world.game_seconds()),
            // AI-CON-1.1: one cell index of the grid per run.
            Part::CellScan => self.cell_scan(ctx.world),
            Part::CellSystem => self.expansion_for_waiting(ctx),
            // AI-2.4a: the run's parts and its last step are run by
            // `construction_run`.
            Part::ConstructionSystem => {}
            Part::ExecutingStep => self.construction_tick(ctx),
            Part::DestructionSystem => {
                // AI-2.6: a useless flag in every tick that is not a
                // building or clearing tick.
                if !v.is_multiple_of(41) && !v.is_multiple_of(59) {
                    self.flag_destruction(ctx);
                }
            }
            Part::NeedCreation => self.need_creation(ctx),
            Part::Locksmith => self.locksmith(ctx),
            Part::Arrangements => {
                self.food_arrangement(ctx);
                self.goods_arrangement(ctx);
            }
            Part::ProductionControl => self.production_control(ctx),
            Part::StreetConstruction => self.street_construction(ctx),
            Part::StreetOptimizer => self.street_optimizer(ctx),
            Part::RouteOptimizer => self.route_optimizer(ctx),
            Part::ForesterConstruction => self.forester_construction(ctx),
            Part::HunterConstruction => self.hunter_construction(ctx),
            Part::BudgetReset => self.budget_reset(),
            Part::BudgetStep => self.budget_step(ctx),
            Part::PitAndWoodcutter => self.pit_and_woodcutter(ctx),
            Part::DepotOrders => self.depot_orders(ctx),
            Part::HarbourOrders => self.harbour_orders(ctx),
            Part::CrowdedCells => self.crowded_cells(ctx),
            Part::BuildingDestruction => self.building_destruction(ctx),
            Part::CellClearing => self.cell_clearing(ctx),
            Part::ShipConstruction => self.ship_construction(ctx),
            Part::Expedition => self.expedition(ctx),
            Part::ExpansionTargets => self.expansion_targets(ctx),
            Part::MilitaryBuilding => self.military_building(ctx),
            Part::MilitarySettings => self.military_settings(ctx),
            Part::AttackSystem => self.attack_system(ctx),
            Part::SoldierCreation => self.soldier_creation(ctx),
            Part::CatapultConstruction => self.catapult_construction(ctx),
            Part::CastleSettings => self.castle_settings(ctx),
            Part::WeaponProduction => self.weapon_production(ctx),
            Part::CoinProduction => self.coin_production(ctx),
            Part::CoinArrangement => self.coin_arrangement(ctx),
            Part::MilitaryUpgrade => self.military_upgrade(ctx),
            Part::ConqueredContinents => self.conquered_continents(),
            _ => {}
        }
    }
}
