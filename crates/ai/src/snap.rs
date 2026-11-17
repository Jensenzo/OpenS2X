//! [`Snap`](s2_sim::snap::Snap) for the computer players' state
//! (`docs/snapshot.md`): every type an [`AiPlayer`](crate::AiPlayer) or a
//! military map holds, each line naming all its fields. The configuration
//! is not written: it comes from the rules when the game is loaded.

use s2_sim::{snap_enum, snap_struct};

snap_struct!(crate::cells::FullMark {
    time,
    count,
    active
});
snap_struct!(crate::cells::Cell {
    pos,
    buildings,
    forester,
    hunter,
    full,
    depot_wanted,
    crowded,
    clearing,
    harbour_wanted,
    position,
    landmass
});
snap_enum!(crate::config::Mode {
    Safe,
    Normal,
    Aggressive
});
snap_struct!(crate::config::Profile {
    mode,
    start_sawmills,
    max_smithies,
    max_mints,
    attack_strength,
    intercepting,
    street_optimizing,
    route_optimizing,
    catapults,
    coin_arrangement
});
snap_enum!(crate::construction::Slot {
    Cell,
    Needs,
    Military,
    Forester,
    Catapult,
    Depot,
    Harbour,
    Ship,
    Hunter
});
snap_enum!(crate::construction::Tag { None, Need { need }, Military { attempts, level, cell, source } });
snap_struct!(crate::construction::Order {
    slot,
    area,
    building,
    weight,
    time,
    tag
});
snap_enum!(crate::construction::Outcome { Placed(f0), NoSite, NoMaterial });
snap_struct!(crate::construction::Construction {
    slots,
    limit,
    count,
    exec
});
snap_enum!(crate::expansion::Kind {
    Position,
    Crowded,
    Anywhere
});
snap_enum!(crate::expansion::Source {
    WaitingNeed,
    Attack,
    Crowded,
    Anywhere
});
snap_struct!(crate::expansion::Target { kind, at, since });
snap_struct!(crate::expansion::Expansion {
    pending,
    current,
    taken,
    remembered
});
snap_struct!(crate::maps::MilCell {
    threat,
    threat_building,
    score
});
snap_struct!(crate::maps::TerritoryCell {
    penalty,
    result,
    catapults,
    attempts
});
snap_struct!(crate::military::Attack { target });
snap_struct!(crate::military::Gate { t0, k });
snap_struct!(crate::military::CatapultScan { cursor, best });
snap_struct!(crate::plan::Need {
    id,
    good,
    consumer,
    cell,
    square,
    production,
    waiting,
    food,
    created,
    tied,
    again,
    celled @ 83 = 0
});
snap_struct!(crate::plan::Production {
    id,
    good,
    cell,
    building,
    need
});
snap_struct!(crate::player::Protect {
    military,
    depot,
    planting,
    harbour
});
snap_enum!(crate::player::Category {
    Depot,
    Ordinary,
    Planter,
    Military,
    Harbour
});
snap_struct!(crate::player::BuildingRec {
    at,
    building_type,
    serial,
    cell,
    protect,
    category,
    catapult
});
snap_struct!(crate::player::AiPlayer {
    id,
    profile_number,
    profile,
    home,
    home_landmass,
    conquered,
    attempts,
    active,
    cells,
    records,
    territory,
    resources,
    plan,
    build,
    expansion,
    street,
    econ,
    stock_updates,
    next_stock,
    next_moving,
    need_stage,
    stock_check,
    statistic,
    forester_index,
    hunter_index,
    pit_cursor,
    destruction_cursor,
    flag_cursor,
    control_cursor,
    locksmith_last,
    military_cooldown,
    settings_countdown,
    attack,
    catapult_scan,
    goods_step,
    weapon_gate,
    coin_gate,
    upgrade_cursor,
    prenumbered
});
snap_struct!(crate::resources::Entry {
    good,
    kind,
    landmass,
    amount
});
snap_struct!(crate::roads::FlagWork {
    flag,
    excluded,
    give_ups,
    target,
    attempts,
    best,
    length,
    length2,
    waypoint,
    cut
});
snap_struct!(crate::roads::RouteWork {
    cursor,
    current,
    k,
    i,
    t,
    w,
    x
});
snap_struct!(crate::roads::Street {
    cursor,
    work,
    optimizer_cursor,
    route
});
snap_enum!(crate::ships::Sail { Build, Go(f0) });
snap_enum!(crate::sites::Area { Cell(f0), Point { at, radius } });
