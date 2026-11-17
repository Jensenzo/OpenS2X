//! [`Snap`](super::Snap) for every type of the game state. Each line names
//! all fields of its type (the macros do not compile otherwise). The few
//! types with private fields have theirs in their own module (`Rng`,
//! `Direction`, `military::Timers`).

use crate::{snap_enum, snap_struct};

snap_struct!(crate::ledger::Ledger { made, used });
snap_struct!(crate::World {
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
    recipes @ 67 = std::collections::BTreeMap::new(),
    recipe_sets @ 76 = std::collections::BTreeMap::new(),
    chosen_recipes @ 76 = std::collections::BTreeMap::new(),
    deepenings @ 77 = std::collections::BTreeMap::new(),
    first_resources @ 77 = std::collections::BTreeMap::new(),
    mine_deepenings @ 77 = std::collections::BTreeMap::new(),
    recipe_groups @ 78 = std::collections::BTreeMap::new(),
    recipe_keep @ 78 = std::collections::BTreeMap::new(),
    paving_rule @ 79 = None,
    pavings @ 79 = std::collections::BTreeMap::new(),
    paved @ 79 = std::collections::BTreeSet::new(),
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
    doodads_removed @ 71 = std::collections::BTreeMap::new(),
    linked_home @ 73 = std::collections::BTreeMap::new(),
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
    sample_phase @ 68 = 0,
    productivity_phase @ 68 = 0,
    explorations @ 68 = 0,
    ledger,
    route_topology,
    height_changes @ 75 = crate::HeightChanges::default()
});
/// Derived from the roads: written as nothing, read back empty.
/// Not game state: written as nothing, read back as 0.
impl crate::snap::Snap for crate::HeightChanges {
    fn put(&self, _: &mut crate::snap::Writer) {}
    fn get(_: &mut crate::snap::Reader) -> Result<Self, crate::snap::SnapError> {
        Ok(Self::default())
    }
    fn shape(_: &mut crate::snap::Shape) -> String {
        "()".to_string()
    }
}
impl crate::snap::Snap for crate::transport::RouteTopology {
    fn put(&self, _: &mut crate::snap::Writer) {}
    fn get(_: &mut crate::snap::Reader) -> Result<Self, crate::snap::SnapError> {
        Ok(Self::default())
    }
    fn shape(_: &mut crate::snap::Shape) -> String {
        "()".to_string()
    }
}
snap_enum!(crate::combat::AttackStep {
    Leaving,
    Walking,
    Waiting,
    Home,
    Meeting,
    Arrived
});
snap_enum!(crate::combat::DefendStep {
    Leaving,
    Out,
    Waiting,
    Back
});
snap_struct!(crate::combat::AttackRecord {
    soldier,
    target,
    counter,
    field,
    opponent,
    hit,
    offset @ 74 = None
});
snap_struct!(crate::combat::OffsetStep { walk, back });
snap_struct!(crate::combat::Duel {
    field,
    attacker,
    defender
});
snap_enum!(crate::command::Command { Noop { player }, PlaceFlag { player, at }, BuildRoad { player, from, to }, PlaceBuilding { player, building, at }, BuildWaterway { player, from, to }, StartRoute { player, at, waterway }, ExtendRoute { player, to }, CancelRoute { player }, RemoveBuilding { player, at }, UpgradeBuilding { player, at }, RemoveRoad { player, at }, RemoveFlag { player, at }, ExpeditionStart { player, at }, ExpeditionStop { player, ship }, ExpeditionBuildHarbor { player, ship }, ExpeditionGo { player, ship, direction }, DropWare { player, good, at, destination }, ProductionToggle { player, at }, ProductionSwitch { player, at }, ToolWeight { player, slot, per_mille }, ToolExclusive { player, slot }, ToolClearExclusive { player }, ToolReset { player }, WarehouseCycle { player, at, good }, WarehouseToggle { player, at, good, stop }, WarehouseSet { player, at, good, stop, on }, WarehouseAll { player, at, mode, settlers }, DistributionWeight { player, table, entry, thousandths }, DistributionReset { player, table }, PriorityMove { player, from, to }, PriorityReset { player }, MilitaryAllocation { player, class, thousandths }, MilitaryAttackStrength { player, thousandths }, MilitaryDefenders { player, thousandths }, MilitaryDefenseType { player, strong }, MilitaryReset { player }, ToggleCoinSupply { player, at }, ToggleMoveOut { player, at }, SetCastleSoldiers { player, at, rank, amount }, Attack { player, target, count, strong_first }, AddGoods { player, good, amount }, ClearGoods { player }, LockBuilding { player, building }, CreateBuilding { player, building, at, settler }, CreateStreet { player, from, to }, CreateShip { player, at }, NameShip { player, ship, name }, RemoveTrigger { name }, Cheat { player, cheat }, OrderGeologist { player, at }, OrderScout { player, at }, DebugSupplySite { player, at, wares, workers }, PlanRoad { player, from, to }, PlanBuilding { player, building, at, records }, TearDown { player, at }, ChooseRecipe { player, at, index }, DeepenMine { player, at }, PaveRoad { player, at } });
snap_enum!(crate::command::Rejection {
    OutsideMap,
    NotOwned,
    NotGeneralSite,
    FlagTooClose,
    FlagFieldUnusable,
    TooCloseToBuilding,
    TooSteep,
    NotMiningGround,
    FootprintUnusable,
    NoWaterForHarbor,
    NoRoomForLargeBuilding,
    TooCloseToEdge,
    UnknownBuildingType,
    NoFlagAtStart,
    NoFlagAtTarget,
    SameFlag,
    TargetNotGeneralSite,
    NoRoute,
    NoBuilding,
    NotAFlag,
    NoRoad,
    NotCoast,
    WaterwayTooLong,
    RouteExists,
    NotAHarbour,
    NoSuchShip,
    NotOnExpedition,
    NoSuchDirection,
    NoHarbourSpot,
    UnknownGood,
    FlagFull,
    NotAConstructionSite,
    WareNotNeeded,
    NoSuchToolSlot,
    NoSuchSetting,
    NotMilitary,
    NotAnEnemy,
    NotUpgradable,
    NoSpaceToUpgrade,
    BuildingLocked,
    NoSuchTrigger,
    PlannedRoadTooLong,
    NotARecipe,
    NotDeepenable,
    NotPavable,
    TooCloseToMilitary
});
snap_struct!(crate::command::CommandResult { command, outcome });
snap_struct!(crate::conquest::Ruin {
    at,
    owner,
    burns_until,
    size,
    building_type @ 72 = None
});
snap_enum!(crate::conquest::CatapultPhase { Idle, Searching { counter }, Turning { aim, landing, left }, Flying { aim, landing, left }, TurningBack { left } });
snap_struct!(crate::conquest::Catapult {
    serial,
    facing,
    phase
});
snap_struct!(crate::construction::Leveller {
    number,
    timer,
    walker
});
snap_struct!(crate::construction::Builder {
    number,
    unit_ticks,
    in_unit,
    clock,
    starts,
    walk,
    at,
    approach
});
snap_enum!(crate::construction::Approach { Pending, Walking { clock, back, out_ticks }, Choose });
snap_enum!(crate::construction::BuilderPose { Working { at, since }, Walking { from, to, progress_bits }, Standing(f0) });
snap_struct!(crate::construction::ConstructionSite {
    size,
    levelling_needed,
    levelling_bits,
    level_direction,
    leveller,
    builder,
    frame_costs,
    costs,
    delivered,
    on_site,
    work_done,
    frame_end,
    work_total,
    unit_ticks,
    leveller_on_way,
    builder_on_way,
    orders_filed
});
snap_struct!(crate::distribution::GoodFlags { stop, out });
snap_struct!(crate::distribution::DistributionRules { goods, entries });
snap_struct!(crate::hunting::AnimalKind {
    id,
    range,
    move_distance,
    speed,
    avoid_buildings,
    idle_thousandths
});
snap_enum!(crate::hunting::AnimalState { Waiting { left }, Choosing { ticket }, Walking { since, speed, idle_thousandths }, Hunted });
snap_struct!(crate::hunting::Animal {
    kind,
    at,
    home,
    state,
    walker,
    hunted_by,
    registered
});
snap_struct!(crate::hunting::Respawn {
    accumulator,
    rate,
    cursor
});
snap_struct!(crate::hunting::Ship {
    owner,
    at,
    water,
    frame_work,
    main_work,
    step_amount,
    vessel,
    name
});
snap_struct!(crate::PlayerId, (0));
snap_struct!(crate::Building {
    owner,
    building_type,
    size,
    at,
    under_construction,
    serial,
    workers,
    timer,
    active,
    productivity,
    productivity_bits,
    production
});
snap_struct!(crate::Road {
    owner,
    number,
    path,
    carrier,
    donkey,
    length,
    traffic,
    water
});
snap_struct!(crate::map::Terrain {
    width,
    height,
    heights,
    height_floor,
    states,
    samples
});
snap_struct!(crate::military::MilitaryRules {
    soldiers,
    max_health,
    fighting_ticks
});
snap_struct!(crate::military::MilitaryBuilding {
    applied,
    soldiers,
    cooldown,
    enemy_distance,
    accumulator,
    coin_switch,
    move_out,
    castle
});
snap_enum!(crate::mission::TriggerKind {
    Explored,
    Territory,
    Victory
});
snap_struct!(crate::mission::Trigger {
    kind,
    field,
    player,
    name,
    active,
    timer
});
snap_enum!(crate::mission::LossReason {
    CastleDestroyed,
    MissionNotCompleted
});
snap_enum!(crate::mission::Outcome { Won { tick }, Lost { tick, reason } });
snap_enum!(crate::mission::MissionEvent { Lost { player, reason }, Won { player }, Completed { value } });
snap_enum!(crate::mission::Cheat {
    INeedSomeHelp,
    ShowMeTheWorld,
    GiveMeSomeMore
});
snap_struct!(crate::mission::Mission {
    target,
    triggers,
    outcomes,
    locks,
    completed
});
snap_struct!(crate::movement::Walker {
    at,
    next,
    progress_bits,
    rate_bits,
    path,
    held,
    pauses,
    road_to
});
snap_struct!(crate::movement::FreeRoute { cost, fields });
snap_struct!(crate::movement::LatticeWalk {
    from,
    to,
    done,
    progress_bits,
    rate_bits
});
snap_enum!(crate::navy::ShipState {
    Idle,
    Search,
    ToHarbour,
    Docked,
    Waiting,
    Sailing,
    Unloading,
    MakingRoom,
    Returning
});
snap_struct!(crate::navy::Cargo { destination, goods });
snap_struct!(crate::navy::Landing { spot, field });
snap_struct!(crate::navy::Vessel {
    launched,
    state,
    counter,
    walker,
    harbour,
    called,
    port,
    cargo,
    expedition,
    aboard,
    target,
    options
});
snap_struct!(crate::navy::Dock {
    field,
    reserved,
    docked
});
snap_struct!(crate::navy::Need {
    good,
    count,
    supplier,
    asked
});
snap_struct!(crate::navy::Transfer {
    good,
    count,
    destination
});
snap_enum!(crate::navy::ExpeditionState { Inactive, Gathering, GettingShip, WaitingForShip(f0) });
snap_struct!(crate::navy::Harbour {
    at,
    docks,
    counter,
    needs,
    transfers,
    called,
    expedition,
    expedition_counter,
    aside,
    builder_aside
});
snap_enum!(crate::placement::BuildingSize {
    Small,
    Medium,
    Large,
    Mine,
    Harbor
});
snap_struct!(crate::placement::BuildingType {
    id,
    size,
    frame_costs,
    costs,
    territory_range,
    explore_radius,
    soldier_limit,
    worker,
    worker_limit,
    warehouse,
    instant,
    product,
    production_ticks,
    stock_limit,
    working_radius,
    cycle,
    second_product,
    switchable,
    catapult,
    planting_good,
    spawn,
    construction_positions,
    upgrade_to,
    doors
});
snap_enum!(crate::placement::Placement {
    Nothing,
    Flag,
    Small,
    Medium,
    Large,
    Mine,
    Harbor
});
snap_enum!(crate::production::WorkCycle {
    None,
    Workshop,
    Mine,
    Gatherer,
    Forester,
    Farm,
    Fisher,
    Hunter,
    DonkeyFarm,
    Shipyard
});
snap_struct!(crate::production::DepositKind {
    id,
    good,
    next,
    elevation,
    grow_ticks,
    blocking,
    life_time,
    needs_plantable,
    random_offset
});
snap_enum!(crate::production::DepositState { Sapling, Growing { ripe_at }, Removing { since, gone_at } });
snap_struct!(crate::production::FinePosition {
    at,
    next,
    progress_bits,
    facing
});
snap_struct!(crate::production::Deposit {
    kind,
    position,
    state,
    reserved_by,
    life_left
});
snap_struct!(crate::production::Ground {
    plantable,
    for_planting,
    tree_type
});
snap_enum!(crate::production::Work { Begin, WaitInputs, Off, Working, Take { second }, WaitRoom { score }, WaitTarget { countdown }, WaitSpot { countdown }, FishRetry, WaitShip { countdown }, Donkey, OutNext { spot }, Out { spot }, ToSpot { spot }, Approach { spot }, AtSpot { spot }, TakeWare { second, stand }, StandBack { stand, score }, Return { stand, score }, WaitBack { score }, Back { score }, AtFlag, WaitFlag, PutDown { second }, In, StepIn, Rest, AtShip { spot, phase } });
snap_struct!(crate::requests::Warehouse {
    stock,
    cooldown,
    flags
});
snap_struct!(crate::requests::Target {
    at,
    serial,
    storage
});
snap_enum!(crate::requests::Requester { Road(f0), Site(f0), Building(f0), Military(f0), Harbour(f0), Specialist { flag, order }, Unlinked });
snap_struct!(crate::requests::Request {
    player,
    requester,
    profession,
    harbour @ 82 = None
});
snap_struct!(crate::requests::Order {
    player,
    good,
    target,
    stamp,
    made,
    harbour @ 82 = None
});
snap_struct!(crate::requests::Message {
    tick,
    player,
    at,
    text,
    until,
    building,
    ship @ 81 = None
});
snap_struct!(crate::rules::Settings {
    settler_speed,
    pickup_ticks,
    drop_ticks,
    flag_package_limit,
    bulldoze_ticks,
    construct_ticks,
    fishing_distance,
    catapult_turn_ticks,
    catapult_flight_ticks
});
snap_struct!(crate::rules::Rules {
    building_types,
    transport_priorities,
    settings,
    headquarters,
    start_stock,
    plank,
    stone,
    goods,
    roles,
    deposit_kinds,
    tree_families,
    default_tree,
    production,
    ship,
    distribution,
    construction,
    military,
    animals,
    specialists,
    recipes @ 67 = std::collections::BTreeMap::new(),
    recipe_sets @ 76 = std::collections::BTreeMap::new(),
    deepenings @ 77 = std::collections::BTreeMap::new(),
    recipe_groups @ 78 = std::collections::BTreeMap::new(),
    recipe_keep @ 78 = std::collections::BTreeMap::new(),
    paving @ 79 = None
});
snap_struct!(crate::paving::PavingRule {
    good,
    per_field,
    speed_percent
});
snap_struct!(crate::paving::PavingWork { needed, sent, laid });
snap_struct!(crate::deepening::Deepening {
    share_per_mille,
    most,
    costs,
    food
});
snap_struct!(crate::deepening::MineDeepening { done, under_way });
snap_struct!(crate::rules::Recipe { product, inputs });
snap_struct!(crate::rules::ShipRules {
    speed,
    stock_limit,
    frame_planks,
    main_planks,
    construct_ticks
});
snap_struct!(crate::rules::GoodRules {
    costs,
    rest_ticks,
    gathered,
    resource,
    kind,
    production_counter @ 66 = None
});
snap_enum!(crate::rules::GoodKind {
    Other,
    Ware,
    Settler,
    Group
});
snap_struct!(crate::rules::Roles {
    helper,
    builder,
    leveller,
    donkey,
    coin,
    boat_carrier
});
snap_enum!(crate::settlers::SettlerJob { ToSite { target, builder }, ToBuilding { target }, Worker { target, work }, Home { target }, ToMilitary { target, health }, Garrison { target, health, heal }, Attack { home, health, step }, Defend { home, health, step, opponent, intercept, defends @ 75 = None }, Specialist { flag, spots_left, step }, CarryOut { warehouse, destination, target, order, step }, Dying, Released { carrier, step }, Leaving { step, at, stand, health @ 70 = None } });
snap_enum!(crate::settlers::LeavingStep {
    Wait,
    Finish,
    Search,
    ToFlag,
    Out { from },
    StepOut,
    Through { walk, home },
    Wander { tries },
    Retry { tries }
});
snap_enum!(crate::settlers::CarryOutStep {
    Door { score },
    Admitted,
    Out,
    PutDown,
    Pickup,
    Stand,
    Back,
    In
});
snap_enum!(crate::settlers::ReleasedStep {
    Unnoticed { walking, to_road },
    Finishing,
    Standing,
    ToFlag
});
snap_enum!(crate::settlers::SettlerList {
    Workers,
    Constructors,
    Carriers,
    Levellers,
    Soldiers,
    Specialists
});
snap_struct!(crate::settlers::Settler {
    number,
    player,
    profession,
    list,
    walker,
    job,
    timer,
    carrying
});
snap_struct!(crate::specialists::SignRule {
    good,
    threshold,
    doodad,
    life_time
});
snap_struct!(crate::specialists::SpecialistRules {
    geologist,
    scout,
    water,
    signs,
    found
});
snap_struct!(crate::specialists::Sign {
    doodad,
    good,
    amount,
    gone_at
});
snap_enum!(crate::specialists::SpecialistStep { ToFlag, ToSpot { spot }, Work { spot }, Return { flag } });
snap_struct!(crate::statistics::PlayerStatistics {
    series,
    production @ 66 = Default::default(),
    coins @ 66 = 0,
    frags @ 66 = 0
});
snap_struct!(crate::transport::Ware {
    good,
    destination,
    target,
    location,
    claimed_by,
    recheck,
    wait_score,
    stranded,
    next,
    joined,
    order,
    next_after,
    free_place
});
snap_enum!(crate::transport::WareLocation { Flag(f0), Carried(f0) });
snap_enum!(crate::transport::Entry { Open { stop }, Building, Exchange(f0), Reserved });
snap_enum!(crate::transport::CarrierTask { Idle, Fetch { ware, flag }, PickUp { ware, ends, second }, Carry { to, entry }, PutDown { at, ends, second, exchange, bid }, Arrive { flag, stand }, Deliver { flag, back }, Inside { flag, waits }, Retired, Out { flag }, Stop { at, exchange } });
snap_struct!(crate::transport::Carrier {
    number,
    road,
    donkey,
    walker,
    ware,
    task,
    fresh,
    bound
});
snap_struct!(crate::transport::Traffic {
    ticks,
    transport_count,
    average_bits,
    donkey_requested
});
snap_struct!(crate::workshop::Production {
    enabled,
    switched,
    count
});
snap_struct!(crate::workshop::ProductionRules {
    mine_food,
    tool_slots,
    tools_group
});
snap_struct!(crate::workshop::FieldResource { good, amount });
