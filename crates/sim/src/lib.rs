//! Deterministic game simulation.
//!
//! All game state lives in [`World`]. It changes only through [`World::step`],
//! which applies queued [`Command`]s and advances one fixed tick. No I/O, no
//! wall-clock time, no floating point in state.

#[cfg(test)]
mod animal_tests;
#[cfg(test)]
mod clearance_tests;
mod combat;
#[cfg(test)]
mod combat_tests;
mod command;
mod conquest;
#[cfg(test)]
mod conquest_tests;
mod construction;
mod deepening;
mod distribution;
#[cfg(test)]
mod distribution_tests;
#[cfg(test)]
mod food_tests;
pub mod grid;
mod hunting;
pub mod invariants;
pub mod ledger;
pub mod map;
mod military;
#[cfg(test)]
mod military_tests;
pub mod mission;
#[cfg(test)]
mod mission_tests;
pub mod movement;
pub mod navy;
#[cfg(test)]
mod navy_tests;
mod paving;
mod placement;
mod production;
#[cfg(test)]
mod prof_tests;
mod requests;
mod restore;
#[cfg(test)]
mod restore_tests;
pub mod rng;
mod roads;
mod route;
#[cfg(test)]
mod route_tests;
mod rules;
mod settlers;
pub mod snap;
#[cfg(test)]
mod specialist_tests;
mod specialists;
#[cfg(test)]
mod start_tests;
pub mod statistics;
mod territory;
pub mod trace;
pub mod transport;
mod upgrade;
#[cfg(test)]
mod upgrade_tests;
pub mod view;
#[cfg(test)]
mod waterway_tests;
mod workshop;

use std::collections::{BTreeMap, BTreeSet};

use grid::Point;
use map::{state, Terrain};

pub use combat::{AttackRecord, AttackStep, DefendStep, Duel, OffsetStep};
pub use command::{Command, CommandResult, Rejection};
pub use conquest::{turn_steps, Catapult, CatapultPhase, Ruin};
pub use construction::{
    remaining_work, Approach, Builder, BuilderPose, ConstructionSite, Leveller,
};
pub use deepening::{Deepening, MineDeepening};
pub use distribution::{DistEntry, DistTable, DistributionRules, GoodFlags, MIN_WEIGHT};
pub use hunting::{Animal, AnimalKind, AnimalState, Respawn, Ship};
pub use military::{MilitaryBuilding, MilitaryRules, MilitarySettings, SoldierView, SOLDIER_RANKS};
pub use mission::{Cheat, LossReason, Mission, MissionEvent, Outcome, Trigger, TriggerKind};
pub use navy::{
    Cargo, Dock, ExpeditionGood, ExpeditionState, Harbour, Landing, Need, ShipState, Transfer,
    Vessel,
};
pub use paving::{PavingRule, PavingWork};
pub use placement::{BuildingSize, BuildingType, Placement, MILITARY_CLEARANCE};
pub use production::{
    Deposit, DepositKind, DepositState, FinePosition, Ground, Work, WorkCycle, FALL_AFTER_WORK,
};
pub use requests::{Message, Order, Request, Requester, Target, Warehouse};
pub use restore::{
    Restore, RestoreReport, RestoredBuilding, RestoredFight, RestoredFineWalk,
    RestoredFinishedShip, RestoredOpponent, RestoredOutdoor, RestoredRoad, RestoredRoughWalk,
    RestoredSettler, RestoredShip, RestoredSoldier, RestoredWaiting, RestoredWare,
};
pub use rng::Rng;
pub use route::Route;
pub use rules::{GoodKind, GoodRules, Recipe, Roles, Rules, Settings, ShipRules};
pub use settlers::{LeavingStep, ReleasedStep, Settler, SettlerJob, SettlerList};
pub use specialists::{Sign, SignRule, SpecialistRules, SpecialistStep};
pub use territory::{start_multiplier, GameSetup, MapAnimal, MapDeposit};
pub use transport::{Carrier, CarrierTask, Entry, RouteStore, Traffic, Ware, WareLocation};
pub use workshop::{
    FieldResource, Production, ProductionRules, ToolPriorities, DEFAULT_TOOL_WEIGHTS,
    MIN_TOOL_WEIGHT, TOOL_SLOTS,
};

/// Simulation tick counter.
pub type Tick = u64;

/// Player slot index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

/// A building or construction site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Building {
    pub owner: PlayerId,
    /// Script id of the building type.
    pub building_type: u32,
    pub size: BuildingSize,
    /// The building's own field `B`.
    pub at: Point,
    /// True until construction is finished (ECO-HQ-4.7).
    pub under_construction: bool,
    /// Its object number (SIM-START-4.2): tells a building from a later one
    /// on the same field, orders warehouses by creation (ECO-REQ-3.2) and
    /// phases its coin check (MIL-PROM-3.1).
    pub serial: u32,
    /// ECO-REQ-1.4: workers sent to it or inside.
    pub workers: u32,
    /// ECO-REQ-1.4, ECO-FOOD-2.2: the 101-tick timer, 0 to 100.
    pub timer: u32,
    /// Its worker has arrived (or it needs none): it works.
    pub active: bool,
    /// ECO-OUT-3.11: productivity in hundredths: 50 from the site's
    /// placement for a type with a product (T26), else -100 until it first
    /// produces.
    pub productivity: i32,
    /// ECO-OUT-3.11 (AI-MIL T24): the same statistics value as a float32
    /// (its bits), each step a float32 operation: what the computer player
    /// and the "No resources found." test read (0.0999998 after falls to
    /// "0.1"); -1.0 without a value.
    pub productivity_bits: u32,
    /// ECO-PROD-1.1: switches and product counter.
    pub production: Production,
}

/// A road between two flags: its fields from one flag to the other. The flag
/// at `path[0]` is its first flag (TRN-CARRY-1.1): the one the road was built
/// from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Road {
    pub owner: PlayerId,
    /// SIM-START-4.2: its object number, taken when it was made (each half
    /// of a split road takes a new one); so a flag's roads in number order
    /// are its link list, new roads appended (TRN-FREE-6.1, Q-280).
    pub number: u32,
    pub path: Vec<Point>,
    /// The carrier working it.
    pub carrier: Option<usize>,
    /// TRN-CARRY-1.2: the second carrier place, filled by a donkey.
    pub donkey: Option<usize>,
    /// TRN-CARRY-5.2: length for ware routing.
    pub length: u32,
    pub traffic: Traffic,
    /// TRN-ROAD-7: a waterway, worked by a boat carrier.
    pub water: bool,
}

/// SIM-START-4.1, 4.2: the world's object counter as it is lent to the
/// computer players ([`World::step_then`], [`World::lend`]): their own
/// records (needs, production records, cells) take their numbers from it.
#[derive(Debug)]
pub struct Numbers(u32);

impl Numbers {
    /// The next object number; the counter rises by one.
    pub fn take(&mut self) -> u32 {
        let number = self.0;
        self.0 = number.wrapping_add(1);
        number
    }

    /// A counter of its own, for tests of the computer player's parts.
    pub fn new(next: u32) -> Self {
        Self(next)
    }
}

/// Complete game state.
#[derive(Clone, PartialEq, Eq)]
pub struct World {
    tick: Tick,
    rng: Rng,
    pending: Vec<Command>,
    terrain: Terrain,
    /// The player whose placement classes are kept (ECO-PLACE-4.1).
    local_player: PlayerId,
    /// ECO-HQ-3.1: owner of each field.
    owners: Vec<Option<PlayerId>>,
    /// ECO-HQ-3.3: bit `p` set when player `p` has explored the field.
    explored: Vec<u8>,
    building_types: BTreeMap<u32, BuildingType>,
    /// Owner of the flag on each field.
    flags: Vec<Option<PlayerId>>,
    /// Building whose own field each field is.
    buildings: Vec<Option<Building>>,
    /// The field indices that hold a building, ascending: the order in which
    /// a map pass would find them, without the pass.
    building_index: Vec<u32>,
    /// Road each inner road field belongs to (TRN-ROAD-3.1).
    streets: Vec<Option<usize>>,
    /// Roads by id; removed roads leave `None`.
    roads: Vec<Option<Road>>,
    /// ECO-PLACE-4.1: best placement per field for `owner`.
    placement: Vec<Placement>,
    /// Results of the commands applied in the last step.
    results: Vec<CommandResult>,
    /// AI-2.1a: the results of the commands of the last tick's late part
    /// (the computer players), kept until the next late part has run.
    late: Vec<CommandResult>,
    settings: Settings,
    priorities: BTreeMap<u32, i64>,
    /// ECO-DIST-4: each player's changed transport ranks; absent is the
    /// default `priorities`.
    ranks: BTreeMap<PlayerId, BTreeMap<u32, i64>>,
    /// ECO-DIST-2: the tables' defaults and each player's changed tables.
    distribution: Vec<DistributionRules>,
    tables: BTreeMap<PlayerId, Vec<DistTable>>,
    /// ECO-DIST-2.1: the key of a building under construction.
    construction: Option<u32>,
    /// Wares by id; delivered wares leave `None`.
    wares: Vec<Option<Ware>>,
    /// Wares lying at each flag, in the order they joined its list
    /// (TRN-CARRY-2.4a: when their place was granted).
    flag_wares: BTreeMap<Point, Vec<usize>>,
    /// TRN-CARRY-2.4a: the key the next ware joining a flag's list takes.
    next_join: u64,
    /// ECO-REQ-5.4a: the place the next ware appended to a free-ware list
    /// takes.
    next_free: u64,
    /// TRN-CARRY-5.3a: the players' stored route costs.
    route_store: transport::RouteStore,
    /// TRN-CARRY-2.4a: the join keys of wares still in a helper's or
    /// worker's hand whose place at the flag was granted, by the settler's
    /// number.
    held_joins: BTreeMap<u32, u64>,
    /// ECO-REQ-4.2 (T25): buildings whose worker arrived in the settlers'
    /// part: their input orders are filed by their own update in the next
    /// tick.
    orders_due: Vec<Target>,
    /// TRN-CARRY-4.3: places at each flag held for wares whose carrier,
    /// worker or helper is cleared to bring them. Rebuilt from the settlers
    /// at the start of every step and kept up to date within it.
    flag_reserved: BTreeMap<Point, u32>,
    /// TRN-CARRY-4.4: each flag's bar (-1 when absent).
    flag_bar: BTreeMap<Point, i64>,
    /// The players of the game: the entries of the setup (UI-SETUP).
    players: Vec<PlayerId>,
    /// Carriers by number.
    carriers: Vec<Carrier>,
    /// ECO-HQ-2, ECO-REQ-3: warehouses by building field.
    warehouses: BTreeMap<Point, Warehouse>,
    /// ECO-HQ-4: construction sites by building field.
    sites: BTreeMap<Point, ConstructionSite>,
    /// MIL-UPG-2: buildings being upgraded, with the type they become. Each
    /// has a site (the target type's construction) beside its finished
    /// state.
    upgrades: BTreeMap<Point, u32>,
    /// MIL-UPG-1.2, 2.4 (T8): a finished construction still linked to its
    /// builder, until the tick of his release (his link dropped in that
    /// tick's settlers' part).
    builder_links: BTreeMap<Point, Tick>,
    headquarters: Option<u32>,
    start_stock: Vec<(u32, u32)>,
    plank: Option<u32>,
    stone: Option<u32>,
    goods: BTreeMap<u32, GoodRules>,
    /// H6: a project's recipes by building type ([`Rules::recipes`]); empty
    /// in the original's game, and then not in the state's `Debug` form, so
    /// its state hashes are as without the hook.
    recipes: BTreeMap<u32, Vec<(u32, u32)>>,
    /// W20: a project's recipe sets by building type
    /// ([`Rules::recipe_sets`]) and the recipe each building works (by its
    /// field; absent is the first). Both empty in the original's game, and
    /// then not in the state's `Debug` form.
    recipe_sets: BTreeMap<u32, Vec<rules::Recipe>>,
    chosen_recipes: BTreeMap<Point, u32>,
    /// W15: a project's mine deepenings by type ([`Rules::deepenings`]),
    /// each field's first resource (kept from its first dig on, only in a
    /// game with deepenings) and each mine's deepenings. All empty in the
    /// original's game, and then not in the state's `Debug` form.
    deepenings: BTreeMap<u32, Deepening>,
    first_resources: deepening::FirstResources,
    mine_deepenings: BTreeMap<Point, MineDeepening>,
    /// W16: a project's recipe groups ([`Rules::recipe_groups`]) and the
    /// stock each player keeps back from them ([`Rules::recipe_keep`]).
    /// Both empty in the original's game, and then not in the state's
    /// `Debug` form.
    recipe_groups: BTreeMap<u32, Vec<u32>>,
    recipe_keep: BTreeMap<u32, u32>,
    /// W19: a project's paving rule ([`Rules::paving`]), the pavements under
    /// way and the paved roads, by road number. None and empty in the
    /// original's game, and then not in the state's `Debug` form.
    paving_rule: Option<PavingRule>,
    pavings: BTreeMap<u32, PavingWork>,
    paved: BTreeSet<u32>,
    roles: Roles,
    /// TRN-FREE-5.5b: each player's tribe; absent is Romans.
    tribes: BTreeMap<PlayerId, u8>,
    /// SIM-START-4.1: the one object counter (its low 32 bits): the number
    /// the next object made takes.
    next_object: u32,
    /// Settlers that are not road carriers, by id; gone ones leave `None`.
    settlers: Vec<Option<Settler>>,
    /// ECO-REQ-2.1 (T39): the one request queue of all players, in filing
    /// order, and its cursor (2.2).
    queue: Vec<Request>,
    queue_cursor: usize,
    /// ECO-REQ-4: open ware orders, in the order they were filed.
    orders: Vec<Order>,
    /// ECO-REQ-5.3f: the orders made so far (the next order's `made`).
    orders_made: u64,
    /// ECO-DIST-3.1a (T30): per player the goods in the order of their
    /// groups in the order list, each placed at its first order and kept.
    order_groups: BTreeMap<PlayerId, Vec<u32>>,
    /// Messages to the players, oldest first.
    messages: Vec<Message>,
    /// ECO-OUT: deposits by field.
    deposits: BTreeMap<Point, Deposit>,
    deposit_kinds: BTreeMap<u32, DepositKind>,
    tree_families: Vec<Vec<u32>>,
    default_tree: Option<u32>,
    /// ECO-PROD-2.2, 5.4, 7: the foods a mine eats in order, the tool of
    /// each priority slot and the tool group.
    mine_food: Vec<u32>,
    tool_slots: Vec<u32>,
    tools_group: Option<u32>,
    /// ECO-PROD-7: each player's tool priorities; absent is the default.
    tool_priorities: BTreeMap<PlayerId, ToolPriorities>,
    /// FMT-S2M-4.5: the resource of each field, or empty for none.
    resources: Vec<Option<FieldResource>>,
    /// ECO-PROD-3.3: the ore field each mine works at the moment.
    ore_fields: BTreeMap<Point, Point>,
    /// ECO-OUT-4.3: per field, empty in test worlds.
    ground: Vec<Ground>,
    /// ECO-OUT-2.6: the deposit each building has reserved.
    targets: BTreeMap<Point, Point>,
    /// GFX-ANIM-3.8 (Run D): trees whose removal starts in this tick unless
    /// they were harvested before (the woodcutter's work arms it).
    removal_timers: BTreeMap<Point, Tick>,
    /// ECO-PLACE-5.3 (T16): the fields whose map decorations a site has
    /// removed from the world, for good.
    decorations_cleared: BTreeSet<Point>,
    /// GFX-OBJ-1.4a: the fields whose map doodads a road, flag or building
    /// has removed for good ([`DOODADS_BY_STREET`], [`DOODADS_ALL`]).
    doodads_removed: BTreeMap<Point, u8>,
    /// TRN-FREE-6.3a: the soldiers, builders and levellers (by number)
    /// going home after a short walk, still linked to the military
    /// building or site that sent for them until their object goes.
    linked_home: BTreeMap<u32, requests::Target>,
    /// AI-CON-8.1e (T95): each field's settler mark, the number of the
    /// settler or carrier that came to stand on it; any settler leaving the
    /// field clears it.
    field_marks: BTreeMap<Point, u32>,
    /// T95: each settler's and carrier's field and walking state as the
    /// last tick left them, by number.
    mark_movers: BTreeMap<u32, (Point, bool)>,
    /// ECO-REQ-4.2: each workshop's input stock.
    inputs: BTreeMap<Point, Vec<(u32, u32)>>,
    deposit_version: u64,
    /// MIL-SOL-2, TER-2.1: the state of every building with a territory
    /// range or an explore radius, by building field.
    military: BTreeMap<Point, MilitaryBuilding>,
    military_rules: MilitaryRules,
    /// MIL-SOL-3 to 5, MIL-PROM-4: the global military counters.
    military_timers: military::Timers,
    /// MIL-SOL-7: each player's settings; absent is the defaults.
    military_settings: BTreeMap<PlayerId, MilitarySettings>,
    /// MIL-CON-4.1: ruins that burn.
    ruins: Vec<Ruin>,
    /// MIL-CON-5: the state of each catapult, by building field.
    catapults: BTreeMap<Point, Catapult>,
    /// MIL-CON-5.5: soldiers each player has killed with catapults.
    kills: BTreeMap<PlayerId, u32>,
    /// MIL-ATK-2.3: one record per soldier sent to attack, in sending order.
    attackers: Vec<AttackRecord>,
    /// UI-SETUP: each player's team; absent or -1 is no team.
    teams: BTreeMap<PlayerId, i8>,
    /// TER-5.4: pairs of players (lower first) whose land has touched.
    contacts: std::collections::BTreeSet<(u8, u8)>,
    /// The fields of all flags in the order they were placed (TER-6.3).
    flag_order: Vec<Point>,
    /// TRN-ROAD-8: the road each player is building.
    routes: BTreeMap<PlayerId, route::Route>,
    /// ECO-FOOD-5: game animals by id; killed ones leave `None`.
    animals: Vec<Option<Animal>>,
    /// ECO-FOOD-5.7: the kinds, by id.
    animal_kinds: BTreeMap<u32, AnimalKind>,
    /// ECO-FOOD-5.9: the highest ticket that has chosen.
    animal_top: i32,
    /// ECO-FOOD-5.15: the respawn record's accumulator, rate and cursor.
    respawn: Respawn,
    /// ECO-FOOD-5.11: the game time in seconds, a 32-bit float as bits.
    game_time: u32,
    /// ECO-FOOD-5.1: the animal each hunter's hut has reserved.
    hunted: BTreeMap<Point, usize>,
    /// ECO-FOOD-5.1, ECO-OUT-2.1: the search counter of each hunter's hut,
    /// gatherer and farm.
    search_timers: BTreeMap<Point, u32>,
    /// ECO-FOOD-4.3: the fish field of each fisherman's hut.
    fish_fields: BTreeMap<Point, Point>,
    /// ECO-FOOD-7: ships under construction and finished.
    ships: Vec<Ship>,
    /// SHIP-1: the finished harbours in creation order.
    harbours: Vec<navy::Harbour>,
    /// SHIP-1.1, 6.1: the map's harbour spots, in map order.
    harbour_spots: Vec<Point>,
    /// SHIP-3.1, 6.1: the sea or continent of each field (0: blocked water).
    regions: Vec<u32>,
    /// FMT-S2M-4.8, 4.9: the map's stored continent index of each field
    /// (-1: none), used as loaded.
    continents: Vec<i32>,
    /// FMT-S2M-4.9: the positions (field cursors) of each land continent
    /// object, by its index.
    land_spots: BTreeMap<i32, Vec<Point>>,
    ship_rules: ShipRules,
    /// ECO-FOOD-4.2
    fishing_distance: u32,
    /// ECO-SPC
    specialist_rules: SpecialistRules,
    /// ECO-SPC-1.2: the number of the next specialist order.
    next_order: u32,
    /// ECO-SPC-3.2: the geologists' signs.
    signs: BTreeMap<Point, Sign>,
    /// UI-GAME-8.5: each player's statistics series.
    statistics: BTreeMap<PlayerId, statistics::PlayerStatistics>,
    /// CAM: the game's mission, if it has one.
    mission: Option<mission::Mission>,
    /// CAM: what happened to the mission in the last step.
    mission_events: Vec<mission::MissionEvent>,
    /// UI-GAME-8.5a: added to the tick for the 601-tick sampling count (0 in
    /// a new game; an original save's load starts that count at 0,
    /// FMT-SAV-3.11).
    sample_phase: u64,
    /// ECO-OUT-3.11, FMT-SAV-3.11: added to the tick for the 600-tick
    /// productivity decay count (0 in a new game; an original save's load
    /// takes the saved count).
    productivity_phase: u64,
    /// GFX-VIEW-5.4 (d): the explorations so far, by any player (a view reads
    /// it to see that one happened; not game state, not in the state hash).
    explorations: u64,
    /// What was made and used up, for the soak checks (not game state).
    ledger: ledger::Ledger,
    /// The shape of each player's road network for the route searches,
    /// derived from `roads` (not game state).
    route_topology: transport::RouteTopology,
    /// GFX-VIEW-5.4 (f): how many height samples roads and levelling have
    /// set so far, for a view to see that the ground moved (not game state,
    /// not saved, not in the state hash).
    height_changes: HeightChanges,
}

/// [`World::height_changes`]: a count a view reads, written as nothing in a
/// snapshot and read back as 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeightChanges(pub u64);

/// The `Debug` form [`World::state_hash`] hashes: every field but the
/// ledger, which measures the game and is no part of it, and the route
/// topology, which is derived from the roads. The pattern names
/// every field, so a new one cannot be left out by mistake.
impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            tick,
            rng,
            pending,
            terrain,
            local_player,
            owners,
            explored,
            building_types,
            flags,
            buildings,
            building_index,
            streets,
            roads,
            placement,
            results,
            late,
            settings,
            priorities,
            ranks,
            distribution,
            tables,
            construction,
            wares,
            flag_wares,
            next_join,
            next_free,
            route_store,
            held_joins,
            orders_due,
            flag_reserved,
            flag_bar,
            players,
            carriers,
            warehouses,
            sites,
            upgrades,
            builder_links,
            headquarters,
            start_stock,
            plank,
            stone,
            goods,
            recipes,
            recipe_sets,
            chosen_recipes,
            deepenings,
            first_resources,
            mine_deepenings,
            recipe_groups,
            recipe_keep,
            paving_rule,
            pavings,
            paved,
            roles,
            tribes,
            next_object,
            settlers,
            queue,
            queue_cursor,
            orders,
            orders_made,
            order_groups,
            messages,
            deposits,
            deposit_kinds,
            tree_families,
            default_tree,
            mine_food,
            tool_slots,
            tools_group,
            tool_priorities,
            resources,
            ore_fields,
            ground,
            targets,
            removal_timers,
            decorations_cleared,
            doodads_removed,
            linked_home,
            field_marks,
            mark_movers,
            inputs,
            deposit_version,
            military,
            military_rules,
            military_timers,
            military_settings,
            ruins,
            catapults,
            kills,
            attackers,
            teams,
            contacts,
            flag_order,
            routes,
            animals,
            animal_kinds,
            animal_top,
            respawn,
            game_time,
            hunted,
            search_timers,
            fish_fields,
            ships,
            harbours,
            harbour_spots,
            regions,
            continents,
            land_spots,
            ship_rules,
            fishing_distance,
            specialist_rules,
            next_order,
            signs,
            statistics,
            mission,
            mission_events,
            sample_phase,
            productivity_phase,
            explorations: _,
            ledger: _,
            route_topology: _,
            height_changes: _,
        } = self;
        let mut d = f.debug_struct("World");
        d.field("tick", tick)
            .field("rng", rng)
            .field("pending", pending)
            .field("terrain", terrain)
            .field("local_player", local_player)
            .field("owners", owners)
            .field("explored", explored)
            .field("building_types", building_types)
            .field("flags", flags)
            .field("buildings", buildings)
            .field("building_index", building_index)
            .field("streets", streets)
            .field("roads", roads)
            .field("placement", placement)
            .field("results", results)
            .field("late", late)
            .field("settings", settings)
            .field("priorities", priorities)
            .field("ranks", ranks)
            .field("distribution", distribution)
            .field("tables", tables)
            .field("construction", construction)
            .field("wares", wares)
            .field("flag_wares", flag_wares)
            .field("next_join", next_join)
            .field("next_free", next_free)
            .field("route_store", route_store)
            .field("held_joins", held_joins)
            .field("orders_due", orders_due)
            .field("flag_reserved", flag_reserved)
            .field("flag_bar", flag_bar)
            .field("players", players)
            .field("carriers", carriers)
            .field("warehouses", warehouses)
            .field("sites", sites)
            .field("upgrades", upgrades)
            .field("builder_links", builder_links)
            .field("headquarters", headquarters)
            .field("start_stock", start_stock)
            .field("plank", plank)
            .field("stone", stone)
            .field("goods", goods)
            .field("roles", roles)
            .field("tribes", tribes)
            .field("next_object", next_object)
            .field("settlers", settlers)
            .field("queue", queue)
            .field("queue_cursor", queue_cursor)
            .field("orders", orders)
            .field("orders_made", orders_made)
            .field("order_groups", order_groups)
            .field("messages", messages)
            .field("deposits", deposits)
            .field("deposit_kinds", deposit_kinds)
            .field("tree_families", tree_families)
            .field("default_tree", default_tree)
            .field("mine_food", mine_food)
            .field("tool_slots", tool_slots)
            .field("tools_group", tools_group)
            .field("tool_priorities", tool_priorities)
            .field("resources", resources)
            .field("ore_fields", ore_fields)
            .field("ground", ground)
            .field("targets", targets)
            .field("removal_timers", removal_timers)
            .field("decorations_cleared", decorations_cleared)
            .field("doodads_removed", doodads_removed)
            .field("linked_home", linked_home)
            .field("field_marks", field_marks)
            .field("mark_movers", mark_movers)
            .field("inputs", inputs)
            .field("deposit_version", deposit_version)
            .field("military", military)
            .field("military_rules", military_rules)
            .field("military_timers", military_timers)
            .field("military_settings", military_settings)
            .field("ruins", ruins)
            .field("catapults", catapults)
            .field("kills", kills)
            .field("attackers", attackers)
            .field("teams", teams)
            .field("contacts", contacts)
            .field("flag_order", flag_order)
            .field("routes", routes)
            .field("animals", animals)
            .field("animal_kinds", animal_kinds)
            .field("animal_top", animal_top)
            .field("respawn", respawn)
            .field("game_time", game_time)
            .field("hunted", hunted)
            .field("search_timers", search_timers)
            .field("fish_fields", fish_fields)
            .field("ships", ships)
            .field("harbours", harbours)
            .field("harbour_spots", harbour_spots)
            .field("regions", regions)
            .field("continents", continents)
            .field("land_spots", land_spots)
            .field("ship_rules", ship_rules)
            .field("fishing_distance", fishing_distance)
            .field("specialist_rules", specialist_rules)
            .field("next_order", next_order)
            .field("signs", signs)
            .field("statistics", statistics)
            .field("mission", mission)
            .field("mission_events", mission_events);
        // H6: only a project's game has recipes.
        if !recipes.is_empty() {
            d.field("recipes", recipes);
        }
        // W20: only a project's game has recipe sets and choices.
        if !recipe_sets.is_empty() {
            d.field("recipe_sets", recipe_sets);
        }
        if !chosen_recipes.is_empty() {
            d.field("chosen_recipes", chosen_recipes);
        }
        // W15: only a project's game has deepenings.
        if !deepenings.is_empty() {
            d.field("deepenings", deepenings);
        }
        if !first_resources.is_empty() {
            d.field("first_resources", first_resources);
        }
        if !mine_deepenings.is_empty() {
            d.field("mine_deepenings", mine_deepenings);
        }
        // W16: only a project's game has recipe groups.
        if !recipe_groups.is_empty() {
            d.field("recipe_groups", recipe_groups);
        }
        if !recipe_keep.is_empty() {
            d.field("recipe_keep", recipe_keep);
        }
        // W19: only a project's game paves.
        if paving_rule.is_some() {
            d.field("paving_rule", paving_rule);
        }
        if !pavings.is_empty() {
            d.field("pavings", pavings);
        }
        if !paved.is_empty() {
            d.field("paved", paved);
        }
        // FMT-SAV-3.11: only a loaded original save has phases.
        if *sample_phase != 0 || *productivity_phase != 0 {
            d.field("sample_phase", sample_phase)
                .field("productivity_phase", productivity_phase);
        }
        d.finish()
    }
}

impl World {
    /// A world without a map.
    pub fn new(seed: u64) -> Self {
        Self::with_map(seed, Terrain::default(), Rules::empty(), PlayerId(0))
    }

    /// A test world on `terrain`: `owner` owns and has explored every field
    /// and no Border bits are computed, as the test cases of ECO-PLACE and
    /// TRN-ROAD assume. A real game starts with [`World::new_game`].
    pub fn with_map(seed: u64, terrain: Terrain, rules: Rules, owner: PlayerId) -> Self {
        let mut world = Self::empty(seed, terrain, rules, owner);
        world.owners.fill(Some(owner));
        world.explored.fill(territory::player_bit(owner));
        world.recompute_all_placement();
        world
    }

    /// A world on `terrain` in which no one owns or has explored anything.
    fn empty(seed: u64, terrain: Terrain, rules: Rules, local_player: PlayerId) -> Self {
        let fields = terrain.len();
        let regions = navy::compute_regions(&terrain);
        let continents = navy::continents_from_regions(&regions);
        Self {
            tick: 0,
            // SIM-RNG-1.2: the low 32 bits of `seed` seed the generator
            // (the game passes `rng::SIM_SEED`, SIM-RNG-2.1).
            rng: Rng::new(seed as u32),
            pending: Vec::new(),
            terrain,
            local_player,
            owners: vec![None; fields],
            explored: vec![0; fields],
            building_types: rules
                .building_types
                .into_iter()
                .map(|t| (t.id, t))
                .collect(),
            flags: vec![None; fields],
            buildings: vec![None; fields],
            building_index: Vec::new(),
            streets: vec![None; fields],
            roads: Vec::new(),
            placement: vec![Placement::Nothing; fields],
            results: Vec::new(),
            late: Vec::new(),
            settings: rules.settings,
            priorities: rules.transport_priorities,
            ranks: BTreeMap::new(),
            distribution: rules.distribution,
            tables: BTreeMap::new(),
            construction: rules.construction,
            wares: Vec::new(),
            flag_wares: BTreeMap::new(),
            next_join: 0,
            next_free: 0,
            route_store: Default::default(),
            held_joins: BTreeMap::new(),
            orders_due: Vec::new(),
            flag_reserved: BTreeMap::new(),
            flag_bar: BTreeMap::new(),
            players: Vec::new(),
            carriers: Vec::new(),
            warehouses: BTreeMap::new(),
            sites: BTreeMap::new(),
            upgrades: BTreeMap::new(),
            builder_links: BTreeMap::new(),
            headquarters: rules.headquarters,
            start_stock: rules.start_stock,
            plank: rules.plank,
            stone: rules.stone,
            goods: rules.goods,
            recipes: rules.recipes,
            recipe_sets: rules.recipe_sets,
            chosen_recipes: BTreeMap::new(),
            deepenings: rules.deepenings,
            first_resources: BTreeMap::new(),
            mine_deepenings: BTreeMap::new(),
            recipe_groups: rules.recipe_groups,
            recipe_keep: rules.recipe_keep,
            paving_rule: rules.paving,
            pavings: BTreeMap::new(),
            paved: BTreeSet::new(),
            roles: rules.roles,
            tribes: BTreeMap::new(),
            next_object: 0,
            settlers: Vec::new(),
            queue: Vec::new(),
            queue_cursor: 0,
            orders: Vec::new(),
            orders_made: 0,
            order_groups: BTreeMap::new(),
            messages: Vec::new(),
            deposits: BTreeMap::new(),
            deposit_kinds: rules.deposit_kinds.into_iter().map(|k| (k.id, k)).collect(),
            tree_families: rules.tree_families,
            default_tree: rules.default_tree,
            mine_food: rules.production.mine_food,
            tool_slots: rules.production.tool_slots,
            tools_group: rules.production.tools_group,
            tool_priorities: BTreeMap::new(),
            resources: Vec::new(),
            ore_fields: BTreeMap::new(),
            ground: Vec::new(),
            targets: BTreeMap::new(),
            removal_timers: BTreeMap::new(),
            decorations_cleared: BTreeSet::new(),
            doodads_removed: BTreeMap::new(),
            linked_home: BTreeMap::new(),
            field_marks: BTreeMap::new(),
            mark_movers: BTreeMap::new(),
            inputs: BTreeMap::new(),
            deposit_version: 0,
            military: BTreeMap::new(),
            military_rules: rules.military,
            military_timers: military::Timers::default(),
            military_settings: BTreeMap::new(),
            attackers: Vec::new(),
            ruins: Vec::new(),
            catapults: BTreeMap::new(),
            kills: BTreeMap::new(),
            teams: BTreeMap::new(),
            contacts: Default::default(),
            flag_order: Vec::new(),
            routes: BTreeMap::new(),
            animals: Vec::new(),
            animal_kinds: rules.animals.into_iter().map(|k| (k.id, k)).collect(),
            animal_top: 0,
            respawn: Respawn::default(),
            game_time: 0,
            hunted: BTreeMap::new(),
            search_timers: BTreeMap::new(),
            fish_fields: BTreeMap::new(),
            ships: Vec::new(),
            harbours: Vec::new(),
            harbour_spots: Vec::new(),
            regions,
            continents,
            land_spots: BTreeMap::new(),
            ship_rules: rules.ship,
            fishing_distance: rules.settings.fishing_distance,
            specialist_rules: rules.specialists,
            next_order: 0,
            signs: BTreeMap::new(),
            statistics: BTreeMap::new(),
            mission: None,
            mission_events: Vec::new(),
            sample_phase: 0,
            productivity_phase: 0,
            explorations: 0,
            ledger: ledger::Ledger::default(),
            route_topology: transport::RouteTopology::default(),
            height_changes: HeightChanges::default(),
        }
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// SIM-START-4.1: the number the next object made will take.
    pub fn next_object(&self) -> u32 {
        self.next_object
    }

    /// AI-2.7, SIM-START-4.2, TRN-CARRY-5.3a: `f` reads the world with the
    /// simulation's generator, object counter and stored route costs lent
    /// to it, outside a tick too (a computer player's activation,
    /// SIM-START-1.1).
    pub fn lend<R>(
        &mut self,
        f: impl FnOnce(&World, &mut Rng, &mut Numbers, &mut RouteStore) -> R,
    ) -> R {
        let mut rng = std::mem::replace(&mut self.rng, Rng::new(0));
        let mut numbers = Numbers(self.next_object);
        let mut routes = std::mem::take(&mut self.route_store);
        let out = f(self, &mut rng, &mut numbers, &mut routes);
        self.rng = rng;
        self.next_object = numbers.0;
        self.route_store = routes;
        out
    }

    /// SIM-START-4.1, 4.2: a new object's number, the counter's value, which
    /// then rises by one.
    pub(crate) fn new_number(&mut self) -> u32 {
        let number = self.next_object;
        self.next_object = number.wrapping_add(1);
        number
    }

    /// The simulation's generator (SIM-RNG-2): its state, seed and draw
    /// count, for traces (TOOL-trace TRACE-4.6).
    pub fn rng(&self) -> &Rng {
        &self.rng
    }

    /// Record where each draw of the simulation's generator is asked for,
    /// from now on (`docs/rng-diff.md`); not game state.
    pub fn record_draws(&mut self) {
        self.rng.record_draws();
    }

    /// The places that drew since the last call, oldest first, as
    /// `file:line` of the simulation's source.
    pub fn take_draw_record(&mut self) -> Vec<String> {
        self.rng
            .take_record()
            .into_iter()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .collect()
    }

    /// A 64-bit FNV-1a hash of the complete game state, for comparing runs
    /// (determinism tests, replays). It is taken over the state's `Debug`
    /// form, in which every collection is ordered.
    pub fn state_hash(&self) -> u64 {
        use std::fmt::Write;
        struct Fnv(u64);
        impl Write for Fnv {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                for &byte in s.as_bytes() {
                    self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                }
                Ok(())
            }
        }
        let mut hash = Fnv(0xcbf2_9ce4_8422_2325);
        write!(hash, "{self:?}").expect("hashing does not fail");
        hash.0
    }

    /// [`World::state_hash`] split by part of the state, to see which part
    /// two runs first differ in: `(name, hash)` for the clock and random
    /// numbers, settlers, wares, buildings, flags, roads, players and land.
    /// Together the parts cover the dynamic state; the static rules are only
    /// in the full hash.
    pub fn state_hashes(&self) -> [(&'static str, u64); 8] {
        fn fnv(value: &dyn std::fmt::Debug) -> u64 {
            let text = format!("{value:?}");
            text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
            })
        }
        [
            (
                "rng",
                fnv(&(
                    self.tick,
                    &self.rng,
                    &self.pending,
                    self.game_time,
                    self.next_object,
                    self.next_order,
                    self.animal_top,
                    self.deposit_version,
                    &self.respawn,
                )),
            ),
            (
                "settlers",
                fnv(&(
                    &self.settlers,
                    &self.carriers,
                    &self.animals,
                    &self.hunted,
                    &self.search_timers,
                    (&self.field_marks, &self.mark_movers),
                )),
            ),
            (
                "wares",
                fnv(&(
                    &self.wares,
                    &self.flag_wares,
                    (self.next_join, &self.held_joins, &self.route_store),
                    self.next_free,
                    &self.orders_due,
                    &self.flag_reserved,
                    (&self.orders, self.orders_made, &self.order_groups),
                    &self.inputs,
                    &self.warehouses,
                )),
            ),
            (
                "buildings",
                fnv(&(
                    (
                        &self.buildings,
                        &self.building_index,
                        &self.sites,
                        &self.upgrades,
                        &self.builder_links,
                        &self.military,
                        &self.catapults,
                        &self.ruins,
                    ),
                    (
                        &self.attackers,
                        &self.ships,
                        &self.harbours,
                        &self.targets,
                        &self.ore_fields,
                        &self.fish_fields,
                    ),
                )),
            ),
            (
                "flags",
                fnv(&(&self.flags, &self.flag_bar, &self.flag_order)),
            ),
            ("roads", fnv(&(&self.roads, &self.streets, &self.routes))),
            (
                "players",
                fnv(&(
                    (
                        &self.players,
                        &self.owners,
                        &self.explored,
                        &self.placement,
                        (&self.queue, &self.queue_cursor),
                        &self.tables,
                        &self.ranks,
                    ),
                    (
                        &self.tool_priorities,
                        &self.military_settings,
                        &self.military_timers,
                        &self.kills,
                        &self.teams,
                        &self.contacts,
                        &self.tribes,
                    ),
                    (
                        &self.messages,
                        &self.statistics,
                        &self.signs,
                        &self.mission,
                        &self.mission_events,
                    ),
                )),
            ),
            (
                "land",
                fnv(&(
                    &self.terrain,
                    &self.deposits,
                    &self.resources,
                    &self.ground,
                    &self.removal_timers,
                    &self.decorations_cleared,
                    &self.regions,
                    &self.continents,
                    &self.land_spots,
                )),
            ),
        ]
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    /// The player whose placement classes [`World::placement_at`] gives.
    /// Whether the slot holds a player (a setup entry); nothing clears it.
    pub fn is_player(&self, player: PlayerId) -> bool {
        self.players.contains(&player)
    }

    /// The players of the game, in player order (CAM-3.6a: the end
    /// screen's rows).
    pub fn players(&self) -> &[PlayerId] {
        &self.players
    }

    /// The team `player` plays in (UI-SETUP, CAM-4.3a: its index, 0 the
    /// first), if it has one.
    pub fn team_of(&self, player: PlayerId) -> Option<i8> {
        self.teams.get(&player).copied().filter(|&t| t >= 0)
    }

    pub fn local_player(&self) -> PlayerId {
        self.local_player
    }

    /// Queue a command to be applied at the start of the next tick.
    pub fn submit(&mut self, cmd: Command) {
        self.pending.push(cmd);
    }

    /// AI-2.1a: apply one command at once, outside a tick, as the tick would
    /// apply it. Only for a private copy of the world: the view a computer
    /// player reads while it thinks, which lays the commands the players
    /// issued earlier in the tick over the world (`s2_ai::view::ThinkView`).
    /// The game itself takes commands at tick boundaries only.
    pub fn apply_at_once(&mut self, cmd: &Command) -> Result<(), Rejection> {
        self.apply(cmd)
    }

    /// Apply pending commands and advance one tick.
    pub fn step(&mut self) {
        self.step_then(|_, _, _, _| Vec::new());
    }

    /// AI-2.1a, AI-2.7: one tick whose last part is `last`, the computer
    /// players. It runs after the whole update, reads the world as the
    /// update left it and may draw from the simulation's generator and take
    /// numbers from its object counter (SIM-START-4.2: the computer
    /// player's records; the only things it may change); the commands it
    /// returns take effect at once, inside the same tick, and their results
    /// follow those of the tick's submitted commands in
    /// [`World::last_results`].
    pub fn step_then(
        &mut self,
        last: impl FnOnce(&World, &mut Rng, &mut Numbers, &mut RouteStore) -> Vec<Command>,
    ) {
        self.update();
        let commands = self.lend(last);
        self.late.clear();
        for cmd in commands {
            let outcome = self.apply(&cmd);
            self.late.push(CommandResult {
                command: cmd,
                outcome,
            });
        }
        self.results.extend(self.late.iter().cloned());
        self.tick += 1;
    }

    /// The update of one tick: the submitted commands, then every system
    /// (ECO-REQ-0.2). The tick counter is raised by the caller.
    fn update(&mut self) {
        self.results.clear();
        // Before the commands: CAM-3.7's cheat completes the map by a command.
        self.mission_events.clear();
        // ECO-PLACE-5.3 (T17): the start castles' first update, before any
        // command places a site.
        if self.tick == 0 {
            self.clear_start_decorations();
        }
        for cmd in std::mem::take(&mut self.pending) {
            let outcome = self.apply(&cmd);
            self.results.push(CommandResult {
                command: cmd,
                outcome,
            });
        }
        // TRN-ROAD-8.3: the routes being built, once a tick.
        self.update_routes();
        self.rebuild_reservations();
        // ECO-REQ-4.2 (T25): the input orders of buildings whose worker
        // arrived last tick, in the buildings' part.
        for target in std::mem::take(&mut self.orders_due) {
            if self.target_building(target).is_some() {
                self.file_input_orders(target.at);
            }
        }
        // ECO-REQ-0.2: buildings with the request queue, the roads, the
        // settlers' six lists (carriers, builders and levellers included),
        // the transport jobs, the resources.
        self.update_players();
        self.update_roads();
        self.update_settler_lists();
        // MIL-UPG-1.2 (Q-367): a released builder drops his link to the
        // construction in the settlers' part of his release tick.
        let now = self.tick;
        self.builder_links.retain(|_, release| *release > now);
        self.transport_jobs();
        self.update_deposits();
        // ECO-FOOD-5.7: the animals' update is part of the resource update,
        // then the respawn (5.15).
        self.update_animals();
        self.update_respawn();
        // LUA-PROP-9: signs with a lifetime (their place in the tick is not
        // traced, Q-197 d).
        self.update_signs();
        // ECO-REQ-0.2: the military update after the resources.
        self.update_military();
        // SHIP-2: the navy after the rest (Q-154).
        self.update_navy();
        // CAM-2.2, 3.2, 3.3: once per logic tick.
        self.update_mission();
        // UI-GAME-8.5: the statistics samples (their place in the tick is
        // not traced, Q-198 d).
        self.update_statistics();
        // AI-CON-8.1e (T95): the fields' settler marks, once per tick.
        self.update_field_marks();
    }

    /// AI-CON-8.1e (T95): a settler that comes to stand on a field marks it;
    /// any settler leaving a field clears its mark, whoever set it. Taken
    /// once per tick from every settler's and carrier's field and walking
    /// state: a new field clears the old one's mark, then marks the new one
    /// when it stands there (clears it when it walks on); standing still and
    /// starting to walk clears the field's mark.
    pub(crate) fn update_field_marks(&mut self) {
        let movers: Vec<(u32, Point, bool)> = self
            .carriers
            .iter()
            .filter(|c| !matches!(c.task, transport::CarrierTask::Retired))
            .map(|c| (c.number, c.walker.at, c.walker.is_walking()))
            .chain(
                self.settlers()
                    .map(|(_, s)| (s.number, s.walker.at, s.walker.is_walking())),
            )
            .collect();
        let mut now = BTreeMap::new();
        for (number, at, walking) in movers {
            match self.mark_movers.get(&number) {
                Some(&(before, _)) if before != at => {
                    self.field_marks.remove(&before);
                    self.set_mark(at, number, walking);
                }
                Some(&(_, was_walking)) => {
                    if walking && !was_walking {
                        self.field_marks.remove(&at);
                    }
                }
                None => self.set_mark(at, number, walking),
            }
            now.insert(number, (at, walking));
        }
        self.mark_movers = now;
    }

    fn set_mark(&mut self, at: Point, number: u32, walking: bool) {
        if walking {
            self.field_marks.remove(&at);
        } else {
            self.field_marks.insert(at, number);
        }
    }

    /// AI-CON-8.1e (T95): the number of the settler or carrier holding the
    /// settler mark of `field`, if any.
    pub fn field_mark(&self, field: Point) -> Option<u32> {
        self.field_marks.get(&field).copied()
    }

    /// What became of the commands of the last step, in order.
    pub fn last_results(&self) -> &[CommandResult] {
        &self.results
    }

    /// What became of the commands of the last tick's late part (the
    /// computer players, [`World::step_then`]): during a late part, those of
    /// the tick before, as the next think needs them.
    pub fn late_results(&self) -> &[CommandResult] {
        &self.late
    }

    fn apply(&mut self, cmd: &Command) -> Result<(), Rejection> {
        match *cmd {
            Command::Noop { .. } => Ok(()),
            Command::PlaceFlag { player, at } => self.place_flag(player, at),
            Command::BuildRoad { player, from, to } => self.build_road(player, from, to),
            Command::PlanRoad { player, from, to } => self.plan_road(player, from, to),
            Command::BuildWaterway { player, from, to } => self.build_waterway(player, from, to),
            Command::StartRoute {
                player,
                at,
                waterway,
            } => self.start_route(player, at, waterway),
            Command::ExtendRoute { player, to } => self.extend_route(player, to),
            Command::CancelRoute { player } => self.cancel_route(player),
            Command::PlaceBuilding {
                player,
                building,
                at,
            } => self.place_building(player, building, at, 0),
            Command::PlanBuilding {
                player,
                building,
                at,
                records,
            } => self.place_building(player, building, at, records),
            Command::RemoveBuilding { player, at } => self.remove_building(player, at),
            Command::TearDown { player, at } => self.tear_down(player, at),
            Command::UpgradeBuilding { player, at } => self.upgrade_building(player, at),
            Command::RemoveRoad { player, at } => self.remove_road_command(player, at),
            Command::RemoveFlag { player, at } => self.remove_flag_command(player, at),
            Command::ExpeditionStart { player, at } => self.expedition_start(player, at),
            Command::ExpeditionStop { player, ship } => self.expedition_stop(player, ship),
            Command::ExpeditionBuildHarbor { player, ship } => {
                self.expedition_build_harbour(player, ship)
            }
            Command::ExpeditionGo {
                player,
                ship,
                direction,
            } => self.expedition_go(player, ship, direction),
            Command::ProductionToggle { player, at } => self.toggle_production(player, at),
            Command::OrderGeologist { player, at } => self.order_specialist(player, at, false),
            Command::OrderScout { player, at } => self.order_specialist(player, at, true),
            Command::ProductionSwitch { player, at } => self.toggle_switch(player, at),
            Command::ChooseRecipe { player, at, index } => self.choose_recipe(player, at, index),
            Command::DeepenMine { player, at } => self.deepen_mine(player, at),
            Command::PaveRoad { player, at } => self.pave_road(player, at),
            Command::ToolWeight {
                player,
                slot,
                per_mille,
            } => self.tool_command(player, |p| {
                if usize::from(slot) >= TOOL_SLOTS {
                    return Err(Rejection::NoSuchToolSlot);
                }
                p.set_weight(usize::from(slot), per_mille as f32 / 1000.0);
                Ok(())
            }),
            Command::ToolExclusive { player, slot } => self.tool_command(player, |p| {
                if usize::from(slot) >= TOOL_SLOTS {
                    return Err(Rejection::NoSuchToolSlot);
                }
                p.exclusive = Some(slot);
                Ok(())
            }),
            Command::ToolClearExclusive { player } => self.tool_command(player, |p| {
                p.exclusive = None;
                Ok(())
            }),
            Command::ToolReset { player } => self.tool_command(player, |p| {
                p.reset();
                Ok(())
            }),
            Command::WarehouseCycle { player, at, good } => self.warehouse_cycle(player, at, good),
            Command::WarehouseToggle {
                player,
                at,
                good,
                stop,
            } => self.warehouse_toggle(player, at, good, stop),
            Command::WarehouseSet {
                player,
                at,
                good,
                stop,
                on,
            } => self.warehouse_set(player, at, good, stop.then_some(on), (!stop).then_some(on)),
            Command::WarehouseAll {
                player,
                at,
                mode,
                settlers,
            } => self.warehouse_all(player, at, mode, settlers),
            Command::DistributionWeight {
                player,
                table,
                entry,
                thousandths,
            } => self.set_weight(player, table, entry, thousandths),
            Command::DistributionReset { player, table } => self.reset_table(player, table),
            Command::PriorityMove { player, from, to } => {
                self.move_rank(player, from, to);
                Ok(())
            }
            Command::PriorityReset { player } => {
                self.reset_ranks(player);
                Ok(())
            }
            Command::MilitaryAllocation {
                player,
                class,
                thousandths,
            } => self.set_allocation(player, class, thousandths),
            Command::MilitaryAttackStrength {
                player,
                thousandths,
            } => self.set_attack_strength(player, thousandths),
            Command::MilitaryDefenders {
                player,
                thousandths,
            } => self.set_defenders(player, thousandths),
            Command::MilitaryDefenseType { player, strong } => {
                self.military_settings_mut(player).prefer_strong = strong;
                Ok(())
            }
            Command::MilitaryReset { player } => {
                self.military_settings.remove(&player);
                Ok(())
            }
            Command::ToggleCoinSupply { player, at } => self.toggle_coin_supply(player, at),
            Command::ToggleMoveOut { player, at } => self.toggle_move_out(player, at),
            Command::SetCastleSoldiers {
                player,
                at,
                rank,
                amount,
            } => self.set_castle_soldiers(player, at, rank, amount),
            Command::Attack {
                player,
                target,
                count,
                strong_first,
            } => self.attack(player, target, count, strong_first),
            Command::DropWare {
                player,
                good,
                at,
                destination,
            } => self.drop_ware(player, good, at, destination),
            Command::DebugSupplySite {
                player,
                at,
                ref wares,
                workers,
            } => self.debug_supply_site(player, at, wares, workers),
            Command::AddGoods {
                player,
                good,
                amount,
            } => self.add_goods(player, good, amount),
            Command::ClearGoods { player } => self.clear_goods(player),
            Command::LockBuilding { player, building } => self.lock_building(player, building),
            Command::CreateBuilding {
                player,
                building,
                at,
                settler,
            } => self.create_building(player, building, at, settler),
            Command::CreateShip { player, at } => self.create_ship(player, at),
            Command::NameShip {
                player,
                ship,
                ref name,
            } => self.name_ship(player, ship, name.clone()),
            Command::CreateStreet { player, from, to } => self.create_street(player, from, to),
            Command::RemoveTrigger { ref name } => self.remove_trigger(name),
            Command::Cheat { player, cheat } => self.cheat(player, cheat),
        }
    }

    /// The goods the rules give a fixed part: carriers, builder, leveller,
    /// donkey (ECO-REQ-1, 5.3).
    pub fn roles(&self) -> Roles {
        self.roles
    }

    /// Owner of the flag on `field`, if any.
    pub fn flag_at(&self, field: Point) -> Option<PlayerId> {
        self.terrain
            .contains(field)
            .then(|| self.flags[self.terrain.index(field)])
            .flatten()
    }

    /// The building whose own field is `field`.
    pub fn building_at(&self, field: Point) -> Option<&Building> {
        if !self.terrain.contains(field) {
            return None;
        }
        self.buildings[self.terrain.index(field)].as_ref()
    }

    /// All buildings, row by row of their fields (from the maintained index
    /// of building fields, not a pass over the map).
    pub fn buildings(&self) -> impl Iterator<Item = &Building> {
        self.building_index
            .iter()
            .filter_map(|&i| self.buildings[i as usize].as_ref())
    }

    /// Every building type of the rules, by id.
    pub fn building_types(&self) -> impl Iterator<Item = &BuildingType> {
        self.building_types.values()
    }

    /// All flags with their fields, row by row.
    pub fn flags(&self) -> impl Iterator<Item = (Point, PlayerId)> + '_ {
        self.terrain
            .fields()
            .zip(&self.flags)
            .filter_map(|(field, owner)| owner.map(|o| (field, o)))
    }

    /// Every flag in the order it was placed (TER-6.3), all players'
    /// together; a flag that went is no longer in it.
    pub fn flag_order(&self) -> &[Point] {
        &self.flag_order
    }

    /// The flags of `player` in the order they were placed (TER-6.3), the
    /// "list order" the computer player visits them in.
    pub fn flags_in_order(&self, player: PlayerId) -> Vec<Point> {
        self.flag_order
            .iter()
            .copied()
            .filter(|&f| self.flag_at(f) == Some(player))
            .collect()
    }

    /// All roads.
    pub fn roads(&self) -> impl Iterator<Item = &Road> {
        self.roads.iter().flatten()
    }

    /// The road with id `id`.
    pub fn road(&self, id: usize) -> Option<&Road> {
        self.roads.get(id)?.as_ref()
    }

    /// The road an inner road field belongs to.
    pub fn road_at(&self, field: Point) -> Option<&Road> {
        if !self.terrain.contains(field) {
            return None;
        }
        let id = self.streets[self.terrain.index(field)]?;
        self.roads[id].as_ref()
    }

    /// ECO-PLACE-4.1: the best thing the owner can place on `field`.
    pub fn placement_at(&self, field: Point) -> Placement {
        if !self.terrain.contains(field) {
            return Placement::Nothing;
        }
        self.placement[self.terrain.index(field)]
    }

    pub fn building_type(&self, id: u32) -> Option<&BuildingType> {
        self.building_types.get(&id)
    }

    /// ECO-HQ: the rules' headquarters type, if they name one.
    pub fn headquarters_type(&self) -> Option<u32> {
        self.headquarters
    }

    /// H6: a project's recipe for building type `id` (`Rules::recipes`):
    /// what one product consumes in place of its product's cost list. None
    /// in the original's game.
    pub fn recipe(&self, id: u32) -> Option<&[(u32, u32)]> {
        self.recipes.get(&id).map(Vec::as_slice)
    }

    /// W20: a project's recipes building type `id` chooses between
    /// (`Rules::recipe_sets`), in order. None in the original's game.
    pub fn recipe_set(&self, id: u32) -> Option<&[rules::Recipe]> {
        self.recipe_sets.get(&id).map(Vec::as_slice)
    }

    /// W20: the index of the recipe the building on `at` works, if its type
    /// has a recipe set (0 until a `Command::ChooseRecipe`).
    pub fn chosen_recipe(&self, at: Point) -> Option<u32> {
        let building = self.building_at(at)?;
        self.recipe_sets
            .contains_key(&building.building_type)
            .then(|| self.chosen_recipes.get(&at).copied().unwrap_or(0))
    }

    /// TRN-ROAD-1.3, ECO-HQ-3.3
    fn explored(&self, player: PlayerId, field: Point) -> bool {
        self.terrain.contains(field)
            && self.explored[self.terrain.index(field)] & territory::player_bit(player) != 0
    }

    /// ECO-HQ-3.1: the player who owns `field`.
    pub fn field_owner(&self, field: Point) -> Option<PlayerId> {
        if !self.terrain.contains(field) {
            return None;
        }
        self.owners[self.terrain.index(field)]
    }

    /// ECO-HQ-3.3: whether `player` has explored `field`.
    pub fn is_explored(&self, player: PlayerId, field: Point) -> bool {
        self.explored(player, field)
    }

    /// GFX-VIEW-5.4 (d): how many explorations any player has made so far
    /// (a soldier walking by unexplored fields, a military building occupied,
    /// a territory change, a ship's move or landing, a specialist, the start).
    /// A view sees that one happened when it changes.
    pub fn explorations(&self) -> u64 {
        self.explorations
    }

    /// GFX-VIEW-5.4 (f): how many height samples have been set since the
    /// world was made or loaded (roads, levelling): a view that built
    /// something from the heights sees by it whether the ground moved.
    pub fn height_changes(&self) -> u64 {
        self.height_changes.0
    }

    /// Set a height sample and count it ([`World::height_changes`]).
    pub(crate) fn set_sample_height(&mut self, sample: Point, height: i32) {
        self.terrain.set_sample_height(sample, height);
        self.height_changes.0 += 1;
    }

    fn set_bits(&mut self, field: Point, bits: u32, on: bool) {
        let index = self.terrain.index(field);
        if on {
            self.terrain.states[index] |= bits;
            // GFX-OBJ-1.4a (i), (ii): the doodads of a field that becomes a
            // street or movement-blocked are rechecked and removed.
            if bits & state::STREET != 0 {
                self.remove_doodads(field, DOODADS_BY_STREET);
            }
            if bits & state::MOVEMENT_BLOCKED != 0 {
                self.remove_doodads(field, DOODADS_ALL);
            }
        } else {
            self.terrain.states[index] &= !bits;
        }
    }

    /// GFX-OBJ-1.4a: the doodads on `field` that `which` removes are gone
    /// for good.
    pub(crate) fn remove_doodads(&mut self, field: Point, which: u8) {
        *self.doodads_removed.entry(field).or_insert(0) |= which;
    }

    /// GFX-OBJ-1.4a: the fields whose map doodads are gone for good, and
    /// which: [`DOODADS_BY_STREET`] (the field had the Street state: the
    /// doodads whose type deletes by street, `dd_setDeleteByStreet`, on
    /// unless a script turns it off) and [`DOODADS_ALL`] (it was
    /// movement-blocked, or a flag or building stood on it: every doodad).
    /// A doodad stands on the field of its sample (ECO-PLACE-5.3). They stay
    /// removed when the road, flag or building goes.
    pub fn doodads_removed(&self) -> &BTreeMap<Point, u8> {
        &self.doodads_removed
    }

    /// GFX-OBJ-1.4a: a game saved before the record (snapshot format 70)
    /// has the doodads under its roads, flags and buildings as they stand
    /// removed; those under one that went before the save show again.
    pub(crate) fn doodads_removed_from_state(&mut self) {
        let terrain = &self.terrain;
        let mut removed = BTreeMap::new();
        for y in 0..terrain.height {
            for x in 0..terrain.width {
                let index = terrain.index((x, y));
                let mut which = 0;
                if terrain.has((x, y), state::STREET) {
                    which |= DOODADS_BY_STREET;
                }
                if terrain.has((x, y), state::MOVEMENT_BLOCKED)
                    || self.flags[index].is_some()
                    || self.buildings[index].is_some()
                {
                    which |= DOODADS_ALL;
                }
                if which != 0 {
                    removed.insert((x, y), which);
                }
            }
        }
        self.doodads_removed = removed;
    }
}

/// GFX-OBJ-1.4a (i): a field's doodads whose type deletes by street are gone.
pub const DOODADS_BY_STREET: u8 = 1;
/// GFX-OBJ-1.4a (ii), (iv): every doodad of the field is gone.
pub const DOODADS_ALL: u8 = 2;

/// Bits that keep a field from being a general site (ECO-PLACE-2.1).
const NOT_GENERAL: u32 = state::BORDER
    | state::WATER
    | state::ELEVATION_BLOCKED
    | state::DOODAD_BLOCKED
    | state::BLOCKED
    | state::DEPOSIT_BLOCKED
    | state::MOVEMENT_BLOCKED;

/// Small synthetic maps for the unit tests.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    pub const P0: PlayerId = PlayerId(0);
    pub const SMALL: u32 = 1;
    pub const MEDIUM: u32 = 2;
    pub const LARGE: u32 = 3;
    pub const MINE: u32 = 4;
    pub const HARBOR: u32 = 5;

    /// A flat `width` x `height` map of building ground at height 0.
    pub fn flat(width: u32, height: u32) -> Terrain {
        let fields = (width * height) as usize;
        Terrain {
            width,
            height,
            heights: vec![0; fields],
            height_floor: -100,
            states: vec![state::FOR_BUILDING; fields],
            samples: Vec::new(),
        }
    }

    pub fn world(terrain: Terrain) -> World {
        let building_types = [
            (SMALL, BuildingSize::Small),
            (MEDIUM, BuildingSize::Medium),
            (LARGE, BuildingSize::Large),
            (MINE, BuildingSize::Mine),
            (HARBOR, BuildingSize::Harbor),
        ]
        .map(|(id, size)| BuildingType::new(id, size))
        .to_vec();
        let rules = Rules {
            building_types,
            ..Rules::empty()
        };
        World::with_map(7, terrain, rules, P0)
    }

    pub fn set(terrain: &mut Terrain, field: Point, bits: u32) {
        let index = terrain.index(field);
        terrain.states[index] = bits;
    }

    pub fn set_height(terrain: &mut Terrain, field: Point, height: i32) {
        let index = terrain.index(field);
        terrain.heights[index] = height;
    }

    /// Apply one command in its own step and return the outcome.
    pub fn apply(world: &mut World, command: Command) -> Result<(), Rejection> {
        world.submit(command);
        world.step();
        world.last_results()[0].outcome
    }

    pub fn flag(world: &mut World, at: Point) -> Result<(), Rejection> {
        apply(world, Command::PlaceFlag { player: P0, at })
    }

    pub fn building(world: &mut World, building: u32, at: Point) -> Result<(), Rejection> {
        apply(
            world,
            Command::PlaceBuilding {
                player: P0,
                building,
                at,
            },
        )
    }

    /// Build a road and, as test worlds have no warehouse, put a carrier on
    /// its idle field at once.
    pub fn road(world: &mut World, from: Point, to: Point) -> Result<(), Rejection> {
        let outcome = apply(
            world,
            Command::BuildRoad {
                player: P0,
                from,
                to,
            },
        );
        if outcome.is_ok() {
            world.spawn_carrier(world.roads.len() - 1);
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(seed: u64, ticks: u64) -> World {
        let mut w = World::new(seed);
        for t in 0..ticks {
            if t % 7 == 0 {
                w.submit(Command::Noop {
                    player: PlayerId(0),
                });
            }
            w.step();
        }
        w
    }

    // spec: none (the step counter of the simulation)
    #[test]
    fn advances_ticks() {
        assert_eq!(run(1, 10).tick(), 10);
    }

    // spec: none (determinism: same seed and commands, same world)
    #[test]
    fn deterministic_for_same_seed_and_commands() {
        assert_eq!(run(42, 1000), run(42, 1000));
    }

    // spec: none (the seed is part of the state)
    #[test]
    fn different_seeds_diverge() {
        assert_ne!(run(1, 10), run(2, 10));
    }

    // spec: none (command results of the last step)
    #[test]
    fn results_of_the_last_step() {
        let mut w = World::new(1);
        w.submit(Command::PlaceFlag {
            player: PlayerId(0),
            at: (0, 0),
        });
        w.step();
        assert_eq!(w.last_results()[0].outcome, Err(Rejection::OutsideMap));
        w.step();
        assert!(w.last_results().is_empty());
    }
}
