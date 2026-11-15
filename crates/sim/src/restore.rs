//! A game continued from a saved state (`specs/FMT-savegame.md` FMT-SAV-3,
//! read by the assets crate and handed over as plain data): the buildings,
//! flags, roads, territory and exploration of the save, the workers and
//! the soldiers its buildings name (FMT-SAV-3.4a) and the carriers and
//! donkeys its streets name (FMT-SAV-3.9, with their saved numbers), the
//! wares at flags and in carriers' hands with their destinations and the
//! orders they serve (FMT-SAV-3.6), a named worker's product in his hands
//! and every settler no list names going home (FMT-SAV-3.5a), the ships
//! being built and the finished ships (FMT-SAV-3.8), the players' statistics
//! with their totals and the productivity count (FMT-SAV-3.11) and the
//! random generator (FMT-SAV-3.2). A named worker on his way walks on, one on
//! the land coming back walks to his flag, one producing inside goes on with
//! his work action, one out at his spot goes on with his work action there
//! or with his fine approach to it (Q-257 b); a site's constructor is at it
//! as one who has arrived, goes on with his approach on the fine lattice or
//! with his running unit (Q-257 e, Q-1033). The fights in progress go on
//! where they stood (FMT-SAV-3.7a, T17): `combat.rs` `restore_fights`.

use crate::grid::Point;
use crate::movement::Walker;
use crate::transport::{idle_field, CarrierTask, Entry, WareLocation};
use crate::{Order, PlayerId, Tick, World};

/// FMT-SAV-3.5a (b): a named worker on his way: player, profession, his
/// building, his saved number, his field and his step's goal.
type OnWay = (PlayerId, u32, Point, Option<u32>, Point, Option<Point>);

/// ECO-PLACE-5.1: a building's flag is its south-east neighbour.
const FLAG_DIRECTION: u8 = 1;

/// A saved building (FMT-SAV-3.4a).
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredBuilding {
    pub player: PlayerId,
    pub building_type: u32,
    pub at: Point,
    /// A site's remaining work of its frame and main parts (each 1 at the
    /// start, 0 when done); `None` for a finished building.
    pub remaining: Option<[f32; 2]>,
    /// A site's levelling progress, 0 to 1.
    pub levelled: f32,
    /// The materials lying at a site (FMT-SAV-3.4b: what its
    /// work has used is not among them).
    pub delivered: Vec<(u32, u32)>,
    /// FMT-SAV-3.4a, 3.5c: the soldiers a military building names, in its
    /// order.
    pub soldiers: Vec<RestoredSoldier>,
    pub production_on: bool,
    /// A warehouse's stock.
    pub stock: Vec<(u32, u32)>,
    /// FMT-SAV-3.4a: the save names a worker of the building (its
    /// `VillageWorkers` list holds a record of the workers list).
    pub worker: bool,
    /// FMT-SAV-2.2, SIM-START-4.1: the saved object numbers of the building
    /// and of its worker, kept.
    pub number: Option<u32>,
    pub worker_number: Option<u32>,
}

/// A soldier of a save (FMT-SAV-3.5c): its rank's profession, its health on the
/// rank's own scale and the counter that heals a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoredSoldier {
    /// FMT-SAV-2.2, SIM-START-4.1: its saved object number, kept.
    pub number: Option<u32>,
    pub profession: u32,
    pub health: u32,
    pub healing: u32,
}

/// A saved road (FMT-SAV-3.9): its fields from end flag to end flag, and
/// whether its carrier and its donkey place hold a settler (the street's
/// two carrier refs name records of the carriers list).
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredRoad {
    pub player: PlayerId,
    pub path: Vec<Point>,
    pub carrier: bool,
    pub donkey: bool,
    /// FMT-SAV-2.2, SIM-START-4.1: the saved object numbers of the carrier
    /// and the donkey (their own ids, low 32 bits), kept.
    pub numbers: [Option<u32>; 2],
    /// FMT-SAV-3.5h (Q-257 a): the field each stands on (its current
    /// field). One carrying a ware goes on from there when it lies on the
    /// road; the others stand on the idle field (Q-257 b to e).
    pub standing: [Option<Point>; 2],
}

/// A saved package (FMT-SAV-3.6).
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredWare {
    pub player: PlayerId,
    pub good: u32,
    /// Lying on the ground at this flag (`lying` 1).
    pub lying_at: Option<Point>,
    /// In the hands of the carrier (false) or donkey (true) of the road
    /// numbered so in [`Restore::roads`] (`lying` 0), carrying it to `to`.
    pub carried_by: Option<(usize, bool)>,
    /// While carried: the flag it is carried to and holds its place at (its
    /// flag ref); none for one carried with no flag place.
    pub to: Option<Point>,
    /// Its next flag (TRN-CARRY-5).
    pub next: Option<Point>,
    /// The building it goes to (its field) and whether only to be stored
    /// there (a warehouse pair, ECO-REQ-5.4).
    pub destination: Option<(Point, bool)>,
    /// The order it serves: the tick it was filed (ECO-REQ-5.3f).
    pub order_stamp: Option<u64>,
    /// ECO-DIST-1.6, TRN-CARRY-3.2.
    pub recheck: u32,
    pub wait_score: u32,
}

/// A ship of the save still being built (FMT-SAV-3.8, ECO-FOOD-7.3b): its
/// coast field, its water field and the work left on its frame and main
/// parts (1 down to 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RestoredShip {
    pub player: PlayerId,
    pub coast: Point,
    pub water: Point,
    pub frame_left: f32,
    pub main_left: f32,
}

/// What a save holds that the sim takes over (FMT-SAV-2, 3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Restore {
    pub players: Vec<PlayerId>,
    pub tick: Tick,
    /// The owner of each field (FMT-S2M-4.6), row by row.
    pub owners: Vec<Option<PlayerId>>,
    /// Each field's explored players as bits (FMT-SAV-2.2).
    pub explored: Vec<u8>,
    pub buildings: Vec<RestoredBuilding>,
    pub flags: Vec<(PlayerId, Point)>,
    pub roads: Vec<RestoredRoad>,
    /// FMT-SAV-3.6, 3.9: the packages: lying at flags in each flag's order,
    /// then those in carriers' hands. One without a destination gets one
    /// from ECO-REQ-5 as any ware without.
    pub wares: Vec<RestoredWare>,
    /// FMT-SAV-3.8: the ships being built.
    pub ships: Vec<RestoredShip>,
    /// FMT-SAV-3.2, SIM-RNG-2.3: the generator's two words, state and seed.
    pub random: Option<[u32; 2]>,
    /// FMT-SAV-2.2, SIM-START-4.1: the object counter, `LogicSystem`'s
    /// `next_unique_id` (low 32 bits): the next number a new object takes.
    pub next_object: Option<u32>,
    /// FMT-SAV-3.11: per player slot its 8 series and then its 14 series
    /// (UI-GAME-8.5: our series and production counters in their index
    /// order), oldest sample first.
    pub statistics: Vec<(PlayerId, Vec<Vec<u32>>)>,
    /// FMT-SAV-3.11: per player slot the coins produced and the frags so
    /// far (UI-GAME-8.5a, 8.5e).
    pub totals: Vec<(PlayerId, [u32; 2])>,
    /// FMT-SAV-3.11: the 600-tick productivity decay count (ECO-OUT-3.11) as
    /// saved.
    pub productivity_count: Option<u32>,
    /// FMT-SAV-3.5a (T10): a package in the hands of a building's named worker
    /// taking his product out: the building's field and the good.
    pub products: Vec<(Point, u32)>,
    /// FMT-SAV-3.5a (c, T11): the settlers no building, street or soldier
    /// list names, all going home (state 3).
    pub homeward: Vec<RestoredSettler>,
    /// FMT-SAV-3.8 (T12): the finished ships, in the save's order (before
    /// the ships being built, which follow them in the shipped saves).
    pub finished_ships: Vec<RestoredFinishedShip>,
    /// FMT-SAV-3.5a (e): the constructors a site names, on their way to it
    /// (state 4) or at it (32): the site's field and the saved number.
    pub builders: Vec<(Point, u32)>,
    /// FMT-SAV-3.5a (b): the named workers on their way to their building
    /// (state 4): the building's field, the worker's current field and the
    /// goal of his step in progress (3.5h).
    pub workers_on_way: Vec<(Point, Point, Option<Point>)>,
    /// FMT-SAV-3.5a (e, Q-1033): the constructors at their site in a
    /// running unit (step 10): the site's field, c (construction positions
    /// ordered so far, 1 .. 4: at or walking to position c - 1 of
    /// TRN-FREE-5.5) and r (ticks left of the current 55-tick step).
    pub builder_units: Vec<(Point, u32, u32)>,
    /// FMT-SAV-3.5a (b): the named workers on the land coming back (job
    /// states 36 and 37, not on their building's field): the building's
    /// field and the worker's current field. One holding his product has it
    /// in [`Restore::products`].
    pub workers_back: Vec<(Point, Point)>,
    /// FMT-SAV-3.5a (b, 2026-11-09, Q-257): the named workers producing
    /// inside (job state 33 on their building's field, a work loop running):
    /// the building's field and the ticks left of the work action.
    pub workers_working: Vec<(Point, u32)>,
    /// FMT-SAV-3.5a (b, 2026-11-09, Q-257): the named workers out on the
    /// land in their job's main state (33, 34), working at their spot or
    /// walking their fine approach to it.
    pub workers_outdoors: Vec<RestoredOutdoor>,
    /// FMT-SAV-3.5a (b, 2026-11-11): the named workers inside waiting for a
    /// target in their job's main state (33).
    pub workers_waiting: Vec<RestoredWaiting>,
    /// FMT-SAV-3.5a (e, 2026-11-09, Q-257): the constructors walking on the
    /// fine lattice at their site: approaching it (step 13) or walking to a
    /// construction position (step 10, r = 55); the site's field, the walk
    /// and the work state's first number (at step 13 the approach's leg,
    /// Q-1037: 1 out, 2 back; not used at step 10).
    pub builder_walks: Vec<(Point, RestoredFineWalk, u32)>,
    /// FMT-SAV-3.7a: the fights in progress, the duels (the save's second
    /// list, in its order) first, then the pairings not yet fighting (its
    /// first list).
    pub fights: Vec<RestoredFight>,
}

/// FMT-SAV-3.7a, 3.5c: one fight of the save in progress (a `Military
/// Fighters` record with its soldiers' links). Its soldiers are among their
/// home buildings' soldiers (3.4a), named by their saved numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoredFight {
    /// The attacking soldier, his home building and his field (3.5h).
    pub attacker: u32,
    pub attacker_home: Point,
    pub attacker_at: Point,
    /// His closing fine cursor (3.5h): the lattice sample he stands on
    /// (MIL-ATK-6.1a: off his waiting field's point once he stepped).
    pub attacker_sample: Option<Point>,
    /// The common part's state number (3.5a): 18 attack, 20 wait.
    pub attacker_state: u32,
    /// The building attacked.
    pub target: Point,
    /// MIL-ATK-6.2: the round counter, as saved.
    pub counter: u32,
    /// MIL-ATK-4.3: the attacker's waiting field.
    pub field: Option<Point>,
    /// MIL-ATK-6.3: a hit pending from the last round.
    pub hit: bool,
    /// The save's second list: the two are fighting.
    pub fighting: bool,
    /// The opponent, once paired.
    pub opponent: Option<RestoredOpponent>,
}

/// FMT-SAV-3.7a, 3.5c: a fight's opponent: his saved number, his home
/// building, his field (3.5h) and whether he came out as an interceptor
/// (the save's third list, MIL-ATK-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoredOpponent {
    pub number: u32,
    pub home: Point,
    pub at: Point,
    pub intercept: bool,
}

/// FMT-SAV-3.5a (b, 2026-11-09, Q-257): a named worker out on the land in
/// his job's main state: his building's field, the field his walk out ended
/// on (where his fine approach starts; `Movement Rough`'s current field),
/// the field of what he works at (the rough walk's target), the saved state
/// (33 the main work state, 34 the farm's sowing) and either the ticks left
/// of his work action or his fine walk to the spot.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredOutdoor {
    pub at: Point,
    pub field: Point,
    pub spot: Point,
    pub state: u32,
    pub left: Option<u32>,
    pub walk: Option<RestoredFineWalk>,
    /// FMT-SAV-3.5a (b, 2026-11-09 c): on the rough part of his walk out
    /// (no fine walk, no work action): that walk, `field` the field its
    /// running step started from.
    pub rough: Option<RestoredRoughWalk>,
    /// FMT-SAV-3.5a (b, 2026-11-09 c): a hunter's animal (the building's
    /// reserved target), as its index in the map's animal list.
    pub animal: Option<usize>,
    /// FMT-SAV-3.5a (b, 2026-11-11): a farmer going out to harvest (harvest
    /// step 8): the walk out's leg, 1 the door step from the door field
    /// onto the farm's field, 0 the walk to the farm's flag; `rough` is
    /// that leg's walk (`field` its running step's start) and `spot` the
    /// farm's reserved target, the grain he harvests after it.
    pub leg: Option<u32>,
}

/// FMT-SAV-3.5a (b, 2026-11-11, E-sav-014; 2026-11-09 c), 3.5b: a named
/// worker inside his building in his job's main state (33), waiting for a
/// target: the building's field; the countdown left of the wait (ECO-OUT-
/// 3.2) in the harvest state when its step is 7 (the farmer's harvest step;
/// 2026-11-12, E-sav-016: the woodcutter's and stonemason's too)
/// and in the hunter state when its step is 7 (the hunter's wait for an
/// animal); and the field of the building's reserved deposit (3.4b 1), if
/// it has one. Both states keep their counters between uses: the building's
/// cycle says which of the two is running (the farm's, the woodcutter's and
/// the quarry's the harvest state, the hunter's hut's the hunter state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoredWaiting {
    pub at: Point,
    pub harvest: Option<u32>,
    pub hunt: Option<u32>,
    pub target: Option<Point>,
}

/// FMT-SAV-3.5a (b, 2026-11-09 c), 3.5h: the rough (field by field) part of
/// a walk out, from its free path: the fields from the walk's target back
/// to its start, the index of the field the running step goes to (the one
/// after it is where the step started), the step's progress and progress
/// per tick, and the walk's stop-short count k (TRN-FREE-4.2: it ends k
/// fields before the target).
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredRoughWalk {
    pub path: Vec<Point>,
    pub index: u32,
    pub progress: f32,
    pub rate: f32,
    pub stop_short: u32,
}

/// FMT-SAV-3.5h, 3.5a (2026-11-09): a walk on the fine lattice: its points
/// from the target back to the start (`Movement Fine Path`), the index of
/// the point the running step goes to, the step's progress and its progress
/// per tick (the interpolator).
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredFineWalk {
    pub path: Vec<Point>,
    pub index: u32,
    pub progress: f32,
    pub rate: f32,
}

impl RestoredFineWalk {
    /// The walk's start (the last point of the list) and the steps of
    /// [`Self::line`] done before the running one (which goes from the point
    /// after the goal in the list to the goal): `None` for a list too short
    /// or an index outside it.
    pub(crate) fn start_and_done(&self) -> Option<(Point, u32)> {
        let n = self.path.len();
        let i = self.index as usize;
        if n < 2 || i + 1 >= n {
            return None;
        }
        Some((self.path[n - 1], (n - 2 - i) as u32))
    }

    /// The walk's points after the start, in walking order.
    pub(crate) fn line(&self) -> Vec<Point> {
        self.path.iter().rev().skip(1).copied().collect()
    }
}

/// A finished ship of a save (FMT-SAV-3.8, T12): its player, the coast field
/// it was built from, its water field, its state number (SHIP-2: 0 idle ..
/// 8 returning), search counter, expedition goods store, expedition flag,
/// last port position and the harbours it is docked at or sent to.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredFinishedShip {
    pub player: PlayerId,
    pub coast: Point,
    pub water: Point,
    pub state: u32,
    pub counter: u32,
    pub store: Vec<(u32, u32)>,
    pub expedition: bool,
    pub port: Option<Point>,
    pub harbour: Option<Point>,
    pub called: Option<Point>,
}

/// A settler of a save on its way home (FMT-SAV-3.5a c): its owner,
/// profession, current field and saved number, and its wander failure
/// count (TRN-FREE-6.4 step 4, Q-1032 c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoredSettler {
    pub player: PlayerId,
    pub profession: u32,
    pub field: Point,
    pub number: u32,
    pub failures: u32,
}

/// What [`World::restore`] took over and what it left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreReport {
    pub buildings: usize,
    pub sites: usize,
    pub warehouses: usize,
    pub flags: usize,
    pub roads: usize,
    /// Buildings of a type the rules do not know, of no player, off the
    /// map or on a taken field.
    pub buildings_skipped: usize,
    /// Roads without a flag at both ends, over a taken field, or whose
    /// fields are not a walk of neighbours on the map.
    pub roads_skipped: usize,
    /// Workers seated in their buildings, soldiers present in theirs,
    /// carriers and donkeys on their roads.
    pub workers: usize,
    pub soldiers: usize,
    pub carriers: usize,
    pub donkeys: usize,
    /// Wares put down at their flags; in carriers' hands; with a
    /// destination; serving an order; those whose flag or carrier is not
    /// there (or the flag full).
    pub wares: usize,
    pub wares_carried: usize,
    pub wares_with_destination: usize,
    pub orders_served: usize,
    pub wares_skipped: usize,
    /// Ships being built.
    pub ships: usize,
    /// FMT-SAV-3.5a: products in their workers' hands (T10), settlers sent
    /// home (T11).
    pub products: usize,
    pub homeward: usize,
    /// FMT-SAV-3.8 (T12): finished ships on the water.
    pub finished_ships: usize,
    /// FMT-SAV-3.5a (e): constructors at their sites.
    pub builders: usize,
    /// FMT-SAV-3.5a (b): named workers walking to their buildings.
    pub workers_on_way: usize,
    /// FMT-SAV-3.5a (e): builders going on with their running unit.
    pub builder_units: usize,
    /// FMT-SAV-3.5a (b): workers on the land walking back to their flag.
    pub workers_back: usize,
    /// FMT-SAV-3.5a (b): named workers who go on with their work action.
    pub workers_working: usize,
    /// FMT-SAV-3.5a (b, 2026-11-09): named workers out at their spot going
    /// on with their work action, and those going on with their fine
    /// approach to it.
    pub workers_at_spot: usize,
    pub workers_approaching: usize,
    /// FMT-SAV-3.5a (b, 2026-11-09 c): named workers going on with the rough
    /// part of their walk out.
    pub workers_walking_out: usize,
    /// FMT-SAV-3.5a (e, 2026-11-09): constructors going on with their fine
    /// walk (the approach, or a walk to a construction position).
    pub builder_walks: usize,
    /// FMT-SAV-3.5a (b, 2026-11-11): named workers inside going on with
    /// their wait for a target from the saved countdown.
    pub workers_waiting: usize,
    /// FMT-SAV-3.7a: the fights going on, and those left out (a soldier,
    /// his building or the target not there).
    pub fights: usize,
    pub fights_skipped: usize,
}

impl World {
    /// Rebuild a saved game on this world (made from the save's map part,
    /// with no headquarters placed).
    /// What does not fit (a player out of range, a field off the map, a
    /// road that is not a walk of neighbours) is left out, so a damaged
    /// save never stops the game.
    pub fn restore(&mut self, saved: &Restore) -> RestoreReport {
        let mut report = RestoreReport::default();
        // FMT-SAV-3.5a (b): the named workers on their way, walked once the
        // roads are restored.
        let mut walkers: Vec<OnWay> = Vec::new();
        // FMT-SAV-3.4b, 3.5a (e): each site's decrements as the file has
        // them, for a site whose builder goes on with his running unit.
        let mut exact_work: std::collections::BTreeMap<Point, u32> = Default::default();
        let valid = |p: PlayerId| p.0 < crate::requests::PLAYERS;
        // FMT-SAV-2.2, SIM-START-4.1: the counter from the save. The saved
        // buildings, workers, soldiers and carriers keep their numbers; what
        // this rebuild makes besides (roads, flags' entries, network
        // renumberings) takes numbers from the counter on.
        if let Some(n) = saved.next_object {
            self.next_object = n;
        }
        self.players.clear();
        for &p in &saved.players {
            if valid(p) && !self.players.contains(&p) {
                self.players.push(p);
            }
        }
        // The flags first: a building then finds its own and takes one
        // number only (its saved one).
        for &(player, at) in &saved.flags {
            if valid(player) && self.terrain.contains(at) && self.flag_at(at).is_none() {
                self.put_flag(player, at);
                report.flags += 1;
            }
        }
        for b in &saved.buildings {
            let Some(kind) = self.building_types.get(&b.building_type).cloned() else {
                report.buildings_skipped += 1;
                continue;
            };
            // The field and its flag field are on the map and free of other
            // buildings, roads and other players' flags.
            let flag = self.terrain.neighbour(b.at, FLAG_DIRECTION);
            let free = |w: &World, f: Point| w.building_at(f).is_none() && w.road_at(f).is_none();
            let fits = valid(b.player)
                && self.terrain.contains(b.at)
                && free(self, b.at)
                && self.flag_at(b.at).is_none()
                && flag.is_some_and(|f| {
                    free(self, f) && self.flag_at(f).is_none_or(|owner| owner == b.player)
                });
            if !fits {
                report.buildings_skipped += 1;
                continue;
            }
            let has_flag = flag.is_some_and(|f| self.flag_at(f).is_some());
            self.numbered(b.number.filter(|_| has_flag), |w| {
                w.put_building(b.player, kind.id, kind.size, b.at, true)
            });
            match b.remaining {
                None => {
                    if kind.warehouse {
                        self.warehouses.insert(
                            b.at,
                            crate::Warehouse {
                                stock: b.stock.clone(),
                                cooldown: 0,
                                flags: Default::default(),
                            },
                        );
                        report.warehouses += 1;
                    }
                    self.complete(b.at);
                    if Some(kind.id) == self.headquarters {
                        // ECO-HQ-3: the headquarters' land is its own from
                        // the start (the saved territory follows below).
                        self.apply_territory(b.at);
                    }
                    // FMT-SAV-3.4a: the worker the save names, in its
                    // building as one that has arrived (ECO-REQ-4.2); where
                    // it stood is not read (Q-257).
                    let works = kind.worker_limit > 0 && kind.soldier_limit == 0;
                    if let Some(profession) = kind.worker.filter(|_| b.worker && works) {
                        // FMT-SAV-3.5a (b): one on his way walks on to it
                        // over the roads from his field.
                        // over the roads from his field, once the roads are
                        // there (below).
                        let on_way = saved
                            .workers_on_way
                            .iter()
                            .find(|&&(at, ..)| at == b.at)
                            .map(|&(_, field, next)| (field, next));
                        match on_way {
                            Some((field, next)) => walkers.push((
                                b.player,
                                profession,
                                b.at,
                                b.worker_number,
                                field,
                                next,
                            )),
                            None => {
                                self.numbered(b.worker_number, |w| {
                                    w.seat_worker(b.player, profession, b.at)
                                });
                            }
                        }
                        report.workers += 1;
                    }
                    // FMT-SAV-3.4a, 3.5c: the soldiers the military building
                    // names, present with their rank and health (where they
                    // stood is not read, Q-257); a building with one has
                    // claimed its land already (TER-2.1: the saved territory
                    // holds it).
                    if kind.soldier_limit > 0 && !kind.warehouse {
                        let limit = kind.soldier_limit as usize;
                        for &soldier in b.soldiers.iter().take(limit) {
                            let seated = self.numbered(soldier.number, |w| {
                                w.seat_soldier(b.player, soldier, b.at)
                            });
                            if seated {
                                report.soldiers += 1;
                            }
                        }
                        if self.present_soldiers(b.at) > 0 {
                            if let Some(m) = self.military.get_mut(&b.at) {
                                m.applied = true;
                            }
                        }
                    }
                    report.buildings += 1;
                }
                Some([frame, main]) => {
                    self.start_construction(b.at);
                    let plank = self.plank;
                    if let Some(site) = self.sites.get_mut(&b.at) {
                        let done = |left: f32, total: u32| {
                            ((1.0 - left.clamp(0.0, 1.0)) * total as f32) as u32
                        };
                        let frame_done = done(frame, site.frame_end);
                        let main_done = done(main, site.work_total - site.frame_end);
                        let nearest = |left: f32, total: u32| {
                            ((1.0 - left.clamp(0.0, 1.0)) * total as f32).round() as u32
                        };
                        exact_work.insert(
                            b.at,
                            nearest(frame, site.frame_end)
                                + nearest(main, site.work_total - site.frame_end),
                        );
                        if b.levelled >= 1.0 {
                            site.levelling_bits = crate::construction::LEVELLING_DONE.to_bits();
                        }
                        site.restore_work(frame_done, main_done, &b.delivered, plank);
                    }
                    report.sites += 1;
                }
            }
            if let Some(building) = self.buildings[self.terrain.index(b.at)].as_mut() {
                building.production.enabled = b.production_on;
            }
        }
        // The carrier and donkey made for each saved road, by its place in
        // the save's list.
        let mut road_carriers: Vec<[Option<usize>; 2]> = vec![[None; 2]; saved.roads.len()];
        let mut standing = std::collections::BTreeMap::new();
        for (r, road) in saved.roads.iter().enumerate() {
            let (player, path) = (road.player, &road.path);
            let walk = valid(player)
                && path.len() >= 2
                && path.iter().all(|&f| self.terrain.contains(f))
                && path
                    .windows(2)
                    .all(|w| self.terrain.neighbours(w[0]).contains(&Some(w[1])))
                && (1..path.len()).all(|i| !path[..i].contains(&path[i]));
            let fits = walk
                && self.flag_at(path[0]) == Some(player)
                && self.flag_at(path[path.len() - 1]) == Some(player)
                && path[1..path.len() - 1].iter().all(|&f| {
                    self.road_at(f).is_none()
                        && self.flag_at(f).is_none()
                        && self.building_at(f).is_none()
                });
            if !fits {
                report.roads_skipped += 1;
                continue;
            }
            self.add_road(player, path.clone(), false);
            report.roads += 1;
            // FMT-SAV-3.9, TRN-CARRY-1.2, 1.3: the carrier and the donkey the
            // street names, idle on its idle field (where they stood is not
            // read, Q-257), so the road asks for none.
            let id = self.roads.len() - 1;
            for (donkey, named) in [(false, road.carrier), (true, road.donkey)] {
                if named {
                    let walker = Walker::new(idle_field(path));
                    // FMT-SAV-2.2, SIM-START-4.1: its saved number, kept;
                    // else a new one (SIM-START-4.2).
                    let number = match road.numbers[usize::from(donkey)] {
                        Some(n) => n,
                        None => self.new_number(),
                    };
                    let c = self.place_carrier(id, donkey, walker, CarrierTask::Idle, number);
                    road_carriers[r][usize::from(donkey)] = Some(c);
                    if let Some(at) =
                        road.standing[usize::from(donkey)].filter(|f| path.contains(f))
                    {
                        standing.insert(c, at);
                    }
                    if donkey {
                        report.donkeys += 1;
                    } else {
                        report.carriers += 1;
                    }
                }
            }
        }
        if saved.owners.len() == self.owners.len() {
            self.owners = saved
                .owners
                .iter()
                .map(|&o| o.filter(|&p| valid(p)))
                .collect();
        }
        if saved.explored.len() == self.explored.len() {
            for (mine, &bits) in self.explored.iter_mut().zip(&saved.explored) {
                *mine |= bits;
            }
        }
        self.tick = saved.tick;
        // ECO-FOOD-5.11b: the tick clock gains 0.1 a tick as a float32, so
        // the game's seconds at this tick are that sum, rounding included.
        let mut clock = 0.0f32;
        for _ in 0..saved.tick {
            // Once the step is lost to rounding the sum stays: a damaged save's
            // enormous tick must not stop the game here.
            let next = clock + 0.1;
            if next == clock {
                break;
            }
            clock = next;
        }
        self.game_time = clock.to_bits();
        self.restore_wares(saved, &road_carriers, &standing, &mut report);
        // FMT-SAV-3.8 (T12): a finished ship is launched on its water field
        // with its state, counter, expedition goods and flag, last port and
        // harbour links; its number is its place in the list (the save's
        // order).
        for ship in &saved.finished_ships {
            let state = match ship.state {
                0 => crate::navy::ShipState::Idle,
                1 => crate::navy::ShipState::Search,
                2 => crate::navy::ShipState::ToHarbour,
                3 => crate::navy::ShipState::Docked,
                4 => crate::navy::ShipState::Waiting,
                5 => crate::navy::ShipState::Sailing,
                6 => crate::navy::ShipState::Unloading,
                7 => crate::navy::ShipState::MakingRoom,
                8 => crate::navy::ShipState::Returning,
                _ => continue,
            };
            let on_map = self.terrain.contains(ship.water) && self.terrain.contains(ship.coast);
            if !(self.players.contains(&ship.player) && on_map) {
                continue;
            }
            let mut vessel = crate::navy::Vessel::new(ship.water);
            vessel.launched = true;
            vessel.state = state;
            vessel.counter = ship.counter;
            vessel.port = ship.port;
            vessel.harbour = ship.harbour;
            vessel.called = ship.called;
            vessel.expedition = ship.expedition;
            vessel.aboard = ship.store.clone();
            self.ships.push(crate::Ship {
                owner: ship.player,
                at: ship.coast,
                water: Some(ship.water),
                frame_work: 0,
                main_work: 0,
                step_amount: None,
                name: None,
                vessel,
            });
            report.finished_ships += 1;
        }
        // FMT-SAV-3.8, ECO-FOOD-7.3b, 7.4a: a ship being built stands on its
        // coast field over its water field with the work left; its
        // shipwright finds it (ECO-FOOD-7.3).
        for ship in &saved.ships {
            let on_map = self.terrain.contains(ship.coast) && self.terrain.contains(ship.water);
            if valid(ship.player) && on_map {
                self.ships.push(crate::Ship {
                    owner: ship.player,
                    at: ship.coast,
                    water: Some(ship.water),
                    frame_work: ship.frame_left.clamp(0.0, 1.0).to_bits(),
                    main_work: ship.main_left.clamp(0.0, 1.0).to_bits(),
                    step_amount: None,
                    name: None,
                    vessel: crate::navy::Vessel::new(ship.coast),
                });
                report.ships += 1;
            }
        }
        // FMT-SAV-3.5a (b): the named workers on their way walk on; one with
        // no road walk from his field (off the roads) is seated (Q-257 b).
        for (player, profession, at, number, field, next) in walkers {
            let walking = self.numbered(number, |w| {
                w.walk_worker(player, profession, at, field, next)
            });
            match walking {
                Some(_) => report.workers_on_way += 1,
                None => {
                    self.numbered(number, |w| w.seat_worker(player, profession, at));
                }
            }
        }
        // FMT-SAV-3.5a (T10): a named worker taking his product out holds it
        // again, inside with it and waiting for its place at the flag
        // (ECO-WORK-3.4), whether he was at the door or coming back from
        // outside work.
        for &(at, good) in &saved.products {
            let worker = self.settlers.iter_mut().flatten().find(
                |s| matches!(&s.job, crate::SettlerJob::Worker { target, .. } if target.at == at),
            );
            if let Some(worker) = worker {
                if let crate::SettlerJob::Worker { work, .. } = &mut worker.job {
                    *work = crate::Work::WaitRoom { score: 0 };
                    worker.carrying = Some(good);
                    worker.timer = 2;
                    report.products += 1;
                }
            }
        }
        // FMT-SAV-3.5a (b, 2026-11-09, Q-257): a named worker producing
        // inside (33, a work loop with ticks left) goes on with his work
        // action (ECO-WORK-3.2) and ends it after the saved ticks left, then
        // goes on as at its end. One waiting (38: his job could not start)
        // is seated at the start of his cycle, which tries it again in each
        // tick (`seat_worker`, ECO-WORK-3.1a with no ticks to wait).
        for &(at, left) in &saved.workers_working {
            if self.resume_work(at, left) {
                report.workers_working += 1;
            }
        }
        // FMT-SAV-3.5a (b, 2026-11-11, E-sav-014): a named farmer waiting
        // inside in his harvest step (7), and (2026-11-09 c) a hunter waiting
        // for an animal, go on with that wait from the saved countdown
        // (ECO-OUT-3.2, ECO-FOOD-3.2, 5.3), the farm's reserved grain its
        // target.
        for waiting in &saved.workers_waiting {
            if self.resume_waiting(waiting) {
                report.workers_waiting += 1;
            }
        }
        // FMT-SAV-3.5a (b, 2026-11-09, Q-257): a named worker out on the
        // land in his main job state goes on with his work action at his
        // spot, with his fine approach to it (ECO-OUT-3.4, 3.5, 4.7,
        // ECO-FOOD-7.4a), or (2026-11-09 c) with the rough part of his walk
        // out from the saved step, stopping short as saved (ECO-OUT-3.4,
        // 4.6), then as after any walk out.
        for outdoor in &saved.workers_outdoors {
            // An indoor producer standing on his building's door field (one
            // other than its own, ECO-WORK-3.8a) is inside.
            let door = outdoor.field == self.door_field(outdoor.at) && outdoor.walk.is_none();
            if let Some(left) = outdoor.left.filter(|_| door) {
                if self.resume_work(outdoor.at, left) {
                    report.workers_working += 1;
                    continue;
                }
            }
            match self.resume_outdoor(outdoor) {
                Some(true) => report.workers_at_spot += 1,
                Some(false) if outdoor.rough.is_some() => report.workers_walking_out += 1,
                Some(false) => report.workers_approaching += 1,
                None => {}
            }
        }
        // FMT-SAV-3.5a (b, 2026-11-08 b): a named worker on the land coming
        // back (36, 37) walks from his field to his building's flag (ECO-OUT-
        // 3.8), carrying his product if he holds one, then puts it down and
        // goes in as in his cycle.
        for &(at, field) in &saved.workers_back {
            if self.back_worker(at, field) {
                report.workers_back += 1;
            }
        }
        // FMT-SAV-3.5a (e): a constructor his site names is its builder, as
        // one who has arrived (ECO-HQ-4.6b: his next unit starts from the
        // site's flag); one in a running unit goes on with it (below), one
        // approaching (step 13) goes on with his saved walk (below); his
        // walk to the site (state 4) is not taken over (Q-257 e).
        for &(at, number) in &saved.builders {
            if let Some(site) = self.sites.get_mut(&at) {
                if site.builder.is_none() {
                    site.builder = Some(crate::Builder::arrived(number));
                    report.builders += 1;
                }
            }
        }
        // (e, Q-1033 answered 2026-11-08 c): one in a running unit (step 10)
        // goes on with it (ECO-HQ-4.6b step 4) from the saved (c, r).
        for &(at, ordered, left) in &saved.builder_units {
            let Some(number) = self
                .sites
                .get(&at)
                .and_then(|s| s.builder)
                .map(|b| b.number)
            else {
                continue;
            };
            if let Some(builder) = self.resumed_unit(at, number, ordered, left) {
                let site = self.sites.get_mut(&at).expect("a site");
                site.builder = Some(builder);
                // The unit goes on from the decrements made, not from the
                // unit's end that `restore_work` takes without a builder in
                // the middle of one (its material stays used).
                if let Some(&work) = exact_work.get(&at) {
                    site.work_done = work.min(site.work_total);
                }
                report.builder_units += 1;
            }
        }
        // FMT-SAV-3.5a (e, 2026-11-09, Q-257): a constructor on the fine
        // lattice goes on with his walk from the saved point and progress:
        // approaching (step 13) on the way back (leg 2), he chooses his first
        // unit when it ends (ECO-HQ-4.6b step 3); on the way out (leg 1,
        // Q-1037), he first walks back to his flag field's sample; walking
        // to a construction position (step 10, r = 55), he starts his step's
        // action there (step 4).
        for (at, walk, first) in &saved.builder_walks {
            if self.resume_builder_walk(*at, walk, *first) {
                report.builder_walks += 1;
            }
        }
        // FMT-SAV-3.7a (T17): the fights go on where they stood.
        self.restore_fights(&saved.fights, &mut report);
        // FMT-SAV-3.5a (c, T11): every settler no list names is going home:
        // it walks to a warehouse of its network from its field, or, with
        // none in reach (a beaten player's), wanders between its home
        // searches and dies after the 52nd (TRN-FREE-6.4 step 4), going on
        // with its saved failure count (Q-1032 c answered).
        for h in &saved.homeward {
            if self.players.contains(&h.player) && self.terrain.contains(h.field) {
                self.send_home_after(h.player, h.profession, h.field, h.number, h.failures);
                report.homeward += 1;
            }
        }
        // FMT-SAV-3.11: the players' statistics go on from the saved series,
        // the coins and frags from the saved totals.
        for (player, series) in &saved.statistics {
            if self.players.contains(player) {
                self.restore_statistics(*player, series);
            }
        }
        for &(player, [coins, frags]) in &saved.totals {
            if let Some(stats) = self.statistics.get_mut(&player) {
                stats.coins = coins;
                stats.frags = frags;
            }
        }
        // FMT-SAV-3.11, T13: the productivity decay count goes on as saved (it
        // reaches 600, the next decay, in the (600 - count)th tick after the
        // load, tick T + 599 - count); the sampling count (UI-GAME-8.5a) is
        // not saved and starts at 0 in the first tick after the load (tick T).
        let at = saved.tick;
        if let Some(count) = saved.productivity_count {
            let count = u64::from(count % 600);
            self.productivity_phase = (count + 1 + 600 - at % 600) % 600;
        }
        self.sample_phase = (crate::statistics::SAMPLE_TICKS
            - at % crate::statistics::SAMPLE_TICKS)
            % crate::statistics::SAMPLE_TICKS;
        // FMT-SAV-3.2, SIM-RNG-2.3: the generator goes on where the save
        // left it.
        if let Some([state, seed]) = saved.random {
            self.rng.restore(state, seed);
        }
        self.recompute_all_placement();
        report
    }
}

impl World {
    /// FMT-SAV-3.6: the saved packages. One lying at a flag lies there
    /// again, in the flag's order; one in a carrier's hands is in the hands
    /// of the carrier the save names, which carries it to its flag (its
    /// place there; with none, to its next flag or its road's end at the
    /// destination); each keeps its destination, a warehouse pair's as a
    /// ware to be stored, and the order it serves with the tick it was
    /// filed. The levelled sites then order their material (ECO-REQ-4.1),
    /// less what is on its way to them.
    fn restore_wares(
        &mut self,
        saved: &Restore,
        road_carriers: &[[Option<usize>; 2]],
        standing: &std::collections::BTreeMap<usize, Point>,
        report: &mut RestoreReport,
    ) {
        let mut served: Vec<(Point, u32)> = Vec::new();
        for w in &saved.wares {
            let target = w.destination.and_then(|(at, storage)| {
                let mut t = self.target_of(at)?;
                t.storage = storage;
                Some(t)
            });
            let flag_of =
                |world: &World, t: &crate::Target| world.terrain.neighbour(t.at, FLAG_DIRECTION);
            let destination = target.as_ref().and_then(|t| flag_of(self, t));
            let target = target.filter(|_| destination.is_some());
            let order = target
                .filter(|t| !t.storage)
                .and(w.order_stamp)
                .map(|stamp| {
                    let made = self.orders_made;
                    self.orders_made += 1;
                    Order {
                        player: w.player,
                        good: w.good,
                        target: target.expect("checked"),
                        stamp,
                        made,
                    }
                });
            let carrier = w
                .carried_by
                .and_then(|(r, donkey)| road_carriers.get(r)?[usize::from(donkey)])
                .filter(|&c| self.carriers[c].ware.is_none());
            let id = match (w.lying_at, carrier) {
                (Some(flag), _) => {
                    if self.flag_at(flag) != Some(w.player) || !self.flag_has_room_for(flag, None) {
                        report.wares_skipped += 1;
                        continue;
                    }
                    let id = self.new_ware_at(w.good, flag, destination, target);
                    report.wares += 1;
                    id
                }
                (None, Some(c)) => {
                    match self.hand_ware(c, w, standing.get(&c), destination, target) {
                        Some(id) => {
                            report.wares_carried += 1;
                            id
                        }
                        None => {
                            report.wares_skipped += 1;
                            continue;
                        }
                    }
                }
                (None, None) => {
                    report.wares_skipped += 1;
                    continue;
                }
            };
            let ware = self.wares[id].as_mut().expect("live ware");
            // TRN-CARRY-5.4: lying, its saved next flag (else the one just
            // computed); carried, it is computed again at the flag.
            if let (Some(next), Some(_), Some(_)) = (w.next, destination, w.lying_at) {
                ware.next = Some(next);
            }
            ware.recheck = w.recheck;
            ware.wait_score = i64::from(w.wait_score);
            if destination.is_some() {
                report.wares_with_destination += 1;
            }
            if let Some(order) = order {
                ware.order = Some(order);
                served.push((order.target.at, order.good));
                report.orders_served += 1;
            }
        }
        // ECO-REQ-4.1: a levelled site orders every unit once now. The wares
        // on their way serve orders already: each takes the place of one
        // open order its building filed (a site's now, a workshop's when its
        // worker was seated, ECO-PROD-6.1); a building that files later counts
        // them (ECO-PROD-6.4).
        let levelled: Vec<Point> = self
            .sites
            .iter()
            .filter(|(_, site)| site.levelled() && !site.orders_filed)
            .map(|(&at, _)| at)
            .collect();
        for at in levelled {
            self.file_site_orders(at);
        }
        for (at, good) in served {
            self.cancel_orders(at, good, 1);
        }
    }

    /// FMT-SAV-3.6: a package in the hands of carrier `c`, which carries it
    /// on from where it stands (its saved field `at` on its road, FMT-SAV-3.5h,
    /// else its idle field; TRN-CARRY-3.5a: its road's walk) to the flag it
    /// holds its place at; with none, to its next flag, else to its road's
    /// end at the destination. The ware's index, or none when no such flag
    /// is reached.
    fn hand_ware(
        &mut self,
        c: usize,
        w: &RestoredWare,
        at: Option<&Point>,
        destination: Option<Point>,
        target: Option<crate::Target>,
    ) -> Option<usize> {
        let road = self.roads[self.carriers[c].road].as_ref()?;
        let ends = [road.path[0], *road.path.last().expect("road ends")];
        let goal =
            w.to.or(w.next)
                .or_else(|| destination.filter(|d| ends.contains(d)))?;
        if self.flag_at(goal) != Some(w.player) {
            return None;
        }
        // Q-257 a: from its saved field, unless it stands at the goal
        // already (it is handed in there by the flag's own rules instead).
        if let Some(&at) = at.filter(|&&at| at != goal) {
            self.carriers[c].walker = Walker::new(at);
        }
        let walk = self.carry_walk_again(c, w.player, goal)?;
        let id = self.new_ware_at(w.good, goal, destination, target);
        if let Some(list) = self.flag_wares.get_mut(&goal) {
            list.retain(|&x| x != id);
        }
        let ware = self.wares[id].as_mut().expect("live ware");
        ware.location = WareLocation::Carried(c);
        ware.claimed_by = Some(c);
        ware.free_place = None;
        let carrier = &mut self.carriers[c];
        carrier.ware = Some(id);
        carrier.walker.walk(walk);
        carrier.task = CarrierTask::Carry {
            to: goal,
            entry: Entry::Open { stop: false },
        };
        Some(id)
    }
}

impl World {
    /// FMT-SAV-2.2, SIM-START-4.1: `make`, which takes one object number,
    /// with `number` (a saved object's own) as that number; the counter
    /// goes on as before. Without a saved number, the counter's next.
    /// FMT-SAV-3.5a (b): the worker of the building on `at`, on his way to it
    /// from `field` (stepping to `next`): over the roads to its flag, from
    /// his field or else from his step's goal, then in, the door step a walk
    /// of its own (ECO-WORK-3.8, 3.8c), as the economy sends one (ECO-REQ-3).
    /// `None` (nothing made) when no road walk leads from either to the flag.
    fn walk_worker(
        &mut self,
        player: PlayerId,
        profession: u32,
        at: Point,
        field: Point,
        next: Option<Point>,
    ) -> Option<usize> {
        let target = self.target_of(at)?;
        let flag = self.flag_of(at);
        let from = |start: Point| -> Option<Vec<Point>> {
            if start == flag {
                Some(Vec::new())
            } else {
                self.road_walk_from(player, start, flag)
            }
        };
        let mut walk = match from(field) {
            Some(walk) => walk,
            None => {
                let next = next.filter(|&n| n != field)?;
                let mut walk = vec![next];
                walk.extend(from(next)?);
                walk
            }
        };
        walk.push(at);
        let door = self.door_field(at);
        if door != at {
            walk.push(door);
        }
        let mut walker = Walker::new(field);
        walker.walk(walk);
        walker.road_to = Some(flag);
        walker.pauses.push(flag);
        if door != at {
            walker.pauses.push(at);
        }
        let index = self.terrain.index(at);
        self.buildings[index].as_mut()?.workers += 1;
        // SIM-START-4.2: a settler made.
        let number = self.new_number();
        Some(self.add_settler(crate::Settler {
            number,
            player,
            profession,
            list: crate::SettlerList::Workers,
            walker,
            job: crate::SettlerJob::ToBuilding { target },
            timer: 0,
            carrying: None,
        }))
    }

    fn numbered<R>(&mut self, number: Option<u32>, make: impl FnOnce(&mut World) -> R) -> R {
        let Some(n) = number else {
            return make(self);
        };
        let counter = self.next_object;
        self.next_object = n;
        let made = make(self);
        debug_assert!(self.next_object <= n.wrapping_add(1), "one number taken");
        self.next_object = counter;
        made
    }
}

impl World {
    /// FMT-SAV-3.5a (b, 2026-11-09): the worker seated in the building on
    /// `at`, which works inside, is in his work action (ECO-WORK-3.2) with
    /// `left` ticks of it to go. False without such a worker or building.
    fn resume_work(&mut self, at: Point, left: u32) -> bool {
        let inside = self
            .building_at(at)
            .and_then(|b| self.building_types.get(&b.building_type))
            .is_some_and(|kind| kind.cycle.works_inside());
        if !inside || left == 0 {
            return false;
        }
        let worker = self.settlers.iter_mut().flatten().find(|s| {
            matches!(&s.job, crate::SettlerJob::Worker { target, work: crate::Work::Begin }
                if target.at == at)
        });
        let Some(worker) = worker else {
            return false;
        };
        if let crate::SettlerJob::Worker { work, .. } = &mut worker.job {
            *work = crate::Work::Working;
            worker.timer = left;
        }
        true
    }

    /// FMT-SAV-3.5a (b): the worker seated in his building on `at` stands on
    /// `field` and walks back to the building's flag (ECO-OUT-3.8), with the
    /// product he holds. False without such a worker, or on the building's
    /// own field.
    fn back_worker(&mut self, at: Point, field: Point) -> bool {
        if field == at || !self.terrain.contains(field) {
            return false;
        }
        let Some(index) = self.settlers.iter().position(|s| {
            s.as_ref().is_some_and(
                |s| matches!(&s.job, crate::SettlerJob::Worker { target, .. } if target.at == at),
            )
        }) else {
            return false;
        };
        let mut settler = self.settlers[index].take().expect("a settler");
        let crate::SettlerJob::Worker { target, .. } = settler.job else {
            unreachable!("found as a worker");
        };
        settler.walker = Walker::new(field);
        settler.timer = 0;
        let flag = self.flag_of(at);
        let work = self.walk_back(&mut settler, flag, 0, true);
        settler.job = crate::SettlerJob::Worker { target, work };
        self.settlers[index] = Some(settler);
        true
    }
}
