//! Meshes for what the simulation holds: the placement overlay
//! (`specs/ECO-building-placement.md` ECO-PLACE-4.1), territory borders,
//! the flags, roads and buildings standing on the map, and construction
//! progress (`specs/ECO-warehouse-construction.md`). Plain data, testable
//! without a window.

use s2_sim::grid::Point;
use s2_sim::map::state;
use s2_sim::view::{Activity, SettlerRef, SettlerView, Step};
use s2_sim::{BuildingSize, DepositState, Placement, PlayerId, Requester, ShipState, World};

use crate::render::terrain::{field_position, MeshData};

/// Draw depth of the overlay (above deposits, below start positions) and of
/// the placed objects (above everything else).
pub const OVERLAY_LAYER: f32 = 1.5;
pub const OBJECT_LAYER: f32 = 3.0;
/// Border stones lie under the placed objects, over the territory tint.
pub const BORDER_LAYER: f32 = 2.0;

/// Colour and half size of the overlay mark for a placement class.
pub fn overlay_style(class: Placement) -> Option<([f32; 3], f32)> {
    match class {
        Placement::Nothing => None,
        Placement::Flag => Some(([1.0, 0.92, 0.2], 0.7)),
        Placement::Small => Some(([0.25, 0.85, 0.25], 1.3)),
        Placement::Medium => Some(([0.98, 0.6, 0.1], 1.7)),
        Placement::Large => Some(([0.9, 0.15, 0.15], 2.1)),
        Placement::Mine => Some(([0.65, 0.35, 0.9], 1.7)),
        Placement::Harbor => Some(([0.2, 0.55, 1.0], 2.1)),
    }
}

/// The colour of a player's border marks and warehouses.
pub fn player_color(player: PlayerId) -> [f32; 3] {
    const COLORS: [[f32; 3]; 8] = [
        [0.15, 0.45, 1.0],
        [0.95, 0.2, 0.2],
        [1.0, 0.85, 0.1],
        [0.2, 0.85, 0.3],
        [0.85, 0.35, 0.95],
        [1.0, 0.55, 0.1],
        [0.1, 0.9, 0.9],
        [0.95, 0.95, 0.95],
    ];
    COLORS[usize::from(crate::render::models3d::colour_index(player)) % COLORS.len()]
}

/// TER-7.2: one marker in the owner's colour on every Border field: a stone
/// on land (`buoys` false) or a buoy on water (`buoys` true), at the field's
/// own position. Buoys bob (TER-7.3), so they are drawn with the moving
/// things.
pub fn border_mesh(world: &World, buoys: bool) -> MeshData {
    let mut mesh = MeshData::default();
    let tick = world.tick() as f32;
    for field in world.terrain().fields() {
        let water = world.terrain().has(field, state::WATER);
        if water != buoys || !world.terrain().has(field, state::BORDER) {
            continue;
        }
        let Some(owner) = world.field_owner(field) else {
            continue;
        };
        let color = player_color(owner);
        let mut at = position(field);
        if buoys {
            // 0.05 x sin(0.3 x t + x + z), scaled up to be seen.
            at[1] += 0.9 * (0.03 * tick + at[0] + at[1]).sin();
            push_diamond(&mut mesh, at, 0.0, 1.5, [0.05, 0.05, 0.05]);
            push_diamond(&mut mesh, at, 0.1, 1.1, color);
            push_square(&mut mesh, at, 0.2, 0.35, [1.0, 1.0, 1.0]);
        } else {
            push_square(&mut mesh, at, 0.0, 1.15, [0.1, 0.1, 0.1]);
            push_square(&mut mesh, at, 0.1, 0.85, color);
        }
    }
    mesh
}

/// One mark per field that can take at least a flag.
pub fn overlay_mesh(world: &World) -> MeshData {
    let mut mesh = MeshData::default();
    for field in world.terrain().fields() {
        if let Some((rgb, half)) = overlay_style(world.placement_at(field)) {
            push_square(&mut mesh, position(field), 0.0, half, rgb);
        }
    }
    mesh
}

/// Draw depth of the territory tint: above the terrain markers and the
/// deposits, below the overlay and the objects.
pub const TERRITORY_LAYER: f32 = 1.3;
/// Opacity of the territory tint.
const TERRITORY_ALPHA: f32 = 0.24;
/// World units of a row: the tint rectangles of consecutive rows touch.
const ROW_HEIGHT: f32 = crate::render::terrain::ROW_SPACING;

/// The land of each player as a translucent fill in its colour: one
/// rectangle for each run of neighbouring fields of one owner in a row.
pub fn territory_mesh(world: &World) -> MeshData {
    let mut mesh = MeshData::default();
    let (width, height) = (world.terrain().width, world.terrain().height);
    let half_w = crate::render::terrain::FIELD_SPACING / 2.0;
    for y in 0..height {
        let mut x = 0;
        while x < width {
            let Some(owner) = world.field_owner((x, y)) else {
                x += 1;
                continue;
            };
            let first = x;
            while x + 1 < width && world.field_owner((x + 1, y)) == Some(owner) {
                x += 1;
            }
            let (left, right) = (position((first, y)), position((x, y)));
            let centre = [(left[0] + right[0]) / 2.0, left[1]];
            let half = (right[0] - left[0]) / 2.0 + half_w;
            push_rect_alpha(
                &mut mesh,
                centre,
                0.0,
                half,
                ROW_HEIGHT / 2.0,
                player_color(owner),
                TERRITORY_ALPHA,
            );
            x += 1;
        }
    }
    mesh
}

/// Half size of a soldier pip and the distance between two.
const PIP_HALF: f32 = 0.6;
const PIP_STEP: f32 = 1.5;

/// Soldier pips above a military building (one per soldier, in the colour
/// of its rank, dim while it is still walking; a dark hollow pip for each
/// wanted soldier that is not there) and coin pips below it.
fn push_garrison(world: &World, mesh: &mut MeshData, at: Point, half: f32) {
    let Some(military) = world.military_at(at) else {
        return;
    };
    let centre = position(at);
    let soldiers = world.garrison(at);
    let headquarters = world.stock(at).is_some();
    let wanted = if headquarters {
        0
    } else {
        world.wanted_soldiers(at) as usize
    };
    let slots = soldiers.len().max(wanted);
    let row = centre[1] + half + 1.4;
    let left = centre[0] - PIP_STEP * (slots.saturating_sub(1)) as f32 / 2.0;
    for slot in 0..slots {
        let x = left + PIP_STEP * slot as f32;
        match soldiers.get(slot) {
            Some(soldier) => {
                let color = crate::ui::military::rank_color(soldier.rank);
                let color = if soldier.present {
                    color
                } else {
                    color.map(|c| c * 0.5)
                };
                push_square(mesh, [x, row], 0.5, PIP_HALF + 0.2, [0.05, 0.05, 0.05]);
                push_square(mesh, [x, row], 0.6, PIP_HALF, color);
            }
            None => push_square(mesh, [x, row], 0.5, PIP_HALF, [0.25, 0.1, 0.1]),
        }
    }
    // Coins: gold diamonds under the building, grey when the supply is off.
    let coins = world.coin_good().map_or(0, |coin| {
        world
            .inputs(at)
            .iter()
            .filter(|&&(g, _)| g == coin)
            .map(|&(_, n)| n)
            .sum::<u32>()
    }) as usize;
    let below = centre[1] - half - 1.4;
    let left = centre[0] - PIP_STEP * (coins.saturating_sub(1)) as f32 / 2.0;
    let gold = if military.coin_switch {
        [1.0, 0.85, 0.1]
    } else {
        [0.5, 0.5, 0.5]
    };
    for i in 0..coins {
        push_diamond(
            mesh,
            [left + PIP_STEP * i as f32, below],
            0.6,
            PIP_HALF + 0.3,
            gold,
        );
    }
}

/// Roads as strips between consecutive fields, flags as
/// diamonds, and buildings as squares sized by plot size (construction sites
/// lighter, warehouses in the owner's colour).
pub fn object_mesh(world: &World) -> MeshData {
    let mut mesh = MeshData::default();
    for road in world.roads() {
        for pair in road.path.windows(2) {
            push_segment(
                &mut mesh,
                position(pair[0]),
                position(pair[1]),
                0.8,
                [0.45, 0.3, 0.15],
            );
        }
    }
    for building in world.buildings() {
        let half = match building.size {
            BuildingSize::Small | BuildingSize::Mine => 2.0,
            BuildingSize::Medium => 2.5,
            BuildingSize::Large | BuildingSize::Harbor => 3.0,
        };
        let fill = if building.under_construction {
            [0.85, 0.75, 0.55]
        } else if world.stock(building.at).is_some() {
            player_color(building.owner)
        } else {
            [0.75, 0.45, 0.3]
        };
        let at = position(building.at);
        push_square(&mut mesh, at, 0.1, half + 0.5, [0.1, 0.08, 0.05]);
        push_square(&mut mesh, at, 0.2, half, fill);
        if !building.under_construction {
            push_garrison(world, &mut mesh, building.at, half);
        }
    }
    for (field, _) in world.flags() {
        let at = position(field);
        push_diamond(&mut mesh, at, 0.3, 1.6, [0.05, 0.05, 0.05]);
        push_diamond(&mut mesh, at, 0.4, 1.1, [0.95, 0.1, 0.1]);
    }
    mesh
}

/// Draw depth of carriers and wares: above the placed objects.
pub const MOVING_LAYER: f32 = 4.0;

/// Width of a construction site's progress bar, in world units.
const PROGRESS_BAR_WIDTH: f32 = 6.0;

/// Carriers where they are along their step, the wares they carry, the
/// wares lying at flags (up to six around each flag), and on each
/// construction site a progress bar, its leveller on the neighbour it works
/// on and its builder; a mark over everything with an open settler request
/// (ECO-REQ-1), and the other settlers on their way.
pub fn moving_mesh(world: &World) -> MeshData {
    let mut mesh = MeshData::default();
    for player in 0..8 {
        for request in world.requests(PlayerId(player)) {
            let at = match request.requester {
                Requester::Road(id) => world
                    .road(id)
                    .map(|road| position(s2_sim::transport::idle_field(&road.path))),
                Requester::Site(target)
                | Requester::Building(target)
                | Requester::Military(target)
                | Requester::Harbour(target) => {
                    let p = position(target.at);
                    Some([p[0], p[1] + 4.0])
                }
                Requester::Specialist { flag, .. } => {
                    let p = position(flag);
                    Some([p[0], p[1] + 4.0])
                }
                // ECO-REQ-2.3a: no longer anyone's.
                Requester::Unlinked => None,
            };
            if let Some(at) = at {
                push_square(&mut mesh, at, 0.3, 1.1, REQUEST_COLOR);
            }
        }
    }
    for (at, site) in world.sites() {
        let center = position(at);
        let bar = [center[0], center[1] - 4.2];
        push_rect(
            &mut mesh,
            bar,
            0.0,
            PROGRESS_BAR_WIDTH / 2.0 + 0.3,
            0.8,
            [0.05, 0.05, 0.05],
        );
        let done = site.progress_permille(world.bulldoze_ticks()) as f32 / 1000.0;
        if done > 0.0 {
            let half = PROGRESS_BAR_WIDTH / 2.0 * done;
            let left = bar[0] - PROGRESS_BAR_WIDTH / 2.0;
            let color = if site.levelled() {
                [0.3, 0.9, 0.3]
            } else {
                [0.95, 0.75, 0.2]
            };
            push_rect(&mut mesh, [left + half, bar[1]], 0.1, half, 0.5, color);
        }
        if site.leveller.is_some() {
            let field = world
                .terrain()
                .neighbour(at, site.level_direction)
                .unwrap_or(at);
            push_diamond(&mut mesh, position(field), 0.2, 1.2, [0.95, 0.75, 0.2]);
        }
        if site.builder.is_some() {
            let spot = [center[0] + 2.0, center[1] + 2.0];
            push_diamond(&mut mesh, spot, 0.2, 1.2, [0.3, 0.9, 0.3]);
        }
    }
    let views = world.settler_views();
    for view in views
        .iter()
        .filter(|v| matches!(v.id, SettlerRef::Carrier(_)))
    {
        let at = walker_position(&view.walk);
        push_diamond(&mut mesh, at, 0.0, 1.4, [0.05, 0.05, 0.05]);
        push_diamond(&mut mesh, at, 0.1, 1.0, [0.95, 0.95, 0.85]);
        if let Some(good) = view.carrying {
            let above = [at[0], at[1] + 1.6];
            push_square(
                &mut mesh,
                above,
                0.2,
                0.8,
                crate::render::terrain::palette_color(good),
            );
        }
    }
    settlers_mesh(world, &views, &mut mesh);
    animals_mesh(world, &mut mesh);
    ships_mesh(world, &mut mesh);
    duels_mesh(world, &mut mesh);
    for (field, _) in world.flags() {
        let center = position(field);
        for (slot, &id) in world.wares_at(field).iter().enumerate() {
            let Some(ware) = world.ware(id) else { continue };
            let angle = slot as f32 * std::f32::consts::TAU / 6.0;
            let at = [center[0] + 2.2 * angle.cos(), center[1] + 2.2 * angle.sin()];
            push_square(
                &mut mesh,
                at,
                0.2,
                0.7,
                crate::render::terrain::palette_color(ware.good),
            );
        }
    }
    mesh
}

/// The mark of an open settler request.
pub const REQUEST_COLOR: [f32; 3] = [1.0, 0.2, 0.85];

/// Fill colour of a settler on its way, by what it is doing.
pub fn settler_color(activity: &Activity) -> [f32; 3] {
    match activity {
        Activity::ToSite { .. } => [0.3, 0.9, 0.3],
        Activity::ToBuilding { .. } | Activity::Worker { .. } => [0.35, 0.75, 1.0],
        Activity::ToMilitary { .. } | Activity::Garrison { .. } => [1.0, 0.55, 0.15],
        Activity::Attack { .. } => [0.9, 0.15, 0.15],
        Activity::Defend { .. } => [1.0, 0.85, 0.2],
        Activity::Home => [0.75, 0.75, 0.75],
        Activity::Specialist { .. } => [0.6, 0.4, 0.95],
        Activity::Dying => [0.2, 0.2, 0.2],
        // Carriers, builders and levellers are drawn on their own.
        Activity::Carrier { .. } | Activity::Builder { .. } | Activity::Leveller { .. } => {
            [0.95, 0.95, 0.85]
        }
    }
}

/// ECO-FOOD-5.7: the animals where they are on their roam; a reserved one
/// has a red mark.
fn animals_mesh(world: &World, mesh: &mut MeshData) {
    for (_, animal) in world.animals() {
        let at = walker_position(&animal.step());
        push_diamond(mesh, at, 0.0, 1.9, [0.05, 0.05, 0.05]);
        push_diamond(mesh, at, 0.1, 1.5, [1.0, 0.55, 0.0]);
        if animal.hunted_by.is_some() {
            push_diamond(mesh, at, 0.2, 0.6, [0.9, 0.1, 0.1]);
        }
    }
}

/// Fill colour of a ship by what it is doing (SHIP-2.2).
pub fn ship_color(state: ShipState, expedition: bool) -> [f32; 3] {
    match state {
        _ if expedition => [0.2, 0.85, 0.95],
        ShipState::Idle | ShipState::Docked => [0.75, 0.75, 0.8],
        ShipState::Search | ShipState::MakingRoom => [0.55, 0.6, 0.9],
        ShipState::ToHarbour | ShipState::Sailing | ShipState::Returning => [0.3, 0.8, 0.4],
        ShipState::Unloading => [0.95, 0.75, 0.2],
        ShipState::Waiting => [0.95, 0.35, 0.8],
    }
}

/// Colour of the landing option in compass direction `d` (SHIP-6.1).
const OPTION_COLORS: [[f32; 3]; 6] = [
    [0.95, 0.3, 0.3],
    [0.95, 0.65, 0.2],
    [0.9, 0.9, 0.25],
    [0.35, 0.85, 0.35],
    [0.3, 0.7, 0.95],
    [0.7, 0.4, 0.95],
];

/// SHIP: docking places of the harbours (teal, yellow while a ship is
/// docked, orange while one is reserved), ships under construction (brown)
/// and finished ships where they sail, with a pip for each cargo entry; a
/// ship waiting for orders shows its landing options as lines to the
/// landing fields, the target brighter.
fn ships_mesh(world: &World, mesh: &mut MeshData) {
    for harbour in world.harbours() {
        for dock in &harbour.docks {
            let color = match (dock.docked, dock.reserved) {
                (Some(_), _) => [0.95, 0.85, 0.2],
                (None, Some(_)) => [1.0, 0.55, 0.15],
                _ => [0.1, 0.65, 0.65],
            };
            push_diamond(mesh, position(dock.field), 0.0, 0.7, color);
        }
    }
    for ship in world.ships() {
        if !ship.vessel.launched {
            if !ship.finished() {
                push_square(mesh, position(ship.at), 0.1, 1.2, [0.45, 0.3, 0.15]);
            }
            continue;
        }
        let v = &ship.vessel;
        let at = walker_position(&v.step());
        for (d, option) in v.options.iter().enumerate() {
            let Some(landing) = option else { continue };
            let chosen = v.target == Some(*landing);
            let half = if chosen { 0.3 } else { 0.12 };
            push_segment(mesh, at, position(landing.field), half, OPTION_COLORS[d]);
            push_diamond(mesh, position(landing.field), 0.2, 1.1, OPTION_COLORS[d]);
        }
        push_diamond(mesh, at, 0.3, 2.9, [0.05, 0.05, 0.05]);
        push_diamond(mesh, at, 0.4, 2.4, ship_color(v.state, v.expedition));
        push_rect(mesh, [at[0], at[1] + 0.2], 0.5, 0.25, 1.6, [0.9, 0.9, 0.9]);
        for i in 0..v.cargo.len() {
            let p = [at[0] - 1.8 + 0.7 * i as f32, at[1] - 3.1];
            push_square(mesh, p, 0.6, 0.3, [0.95, 0.75, 0.2]);
        }
    }
}

/// How far two soldiers of a duel stand apart on their field.
const DUEL_SPREAD: f32 = 1.3;

/// MIL-ATK-6: a duel is a pair of soldiers on one field with crossed
/// blades between them, red while a round runs; every soldier on an errand
/// shows its health as pips above it.
fn duels_mesh(world: &World, mesh: &mut MeshData) {
    for duel in world.duels() {
        let (field, at) = (duel.field, position(duel.field));
        let round = world
            .settler_views()
            .iter()
            .any(|v| v.fighting && v.walk.at == field);
        let blade = if round {
            [1.0, 0.15, 0.1]
        } else {
            [0.95, 0.95, 0.95]
        };
        let (a, b) = ([at[0] - 0.8, at[1] + 0.8], [at[0] + 0.8, at[1] - 0.8]);
        push_segment(mesh, a, b, 0.25, blade);
        push_segment(mesh, [a[0], b[1]], [b[0], a[1]], 0.25, blade);
    }
}

/// Health pips of a soldier on an errand.
fn push_health(mesh: &mut MeshData, at: [f32; 2], rank: usize, health: u32) {
    let left = at[0] - 0.5 * (health.saturating_sub(1)) as f32 * 0.9;
    for i in 0..health {
        let p = [left + 0.9 * i as f32, at[1] + 2.3];
        push_square(mesh, p, 0.3, 0.42, [0.05, 0.05, 0.05]);
        push_square(mesh, p, 0.4, 0.3, crate::ui::military::rank_color(rank));
    }
}

/// Settlers drawn: all but workers standing inside their building.
fn settlers_mesh(world: &World, views: &[SettlerView], mesh: &mut MeshData) {
    for settler in views {
        if !matches!(settler.id, SettlerRef::Settler(_)) || settler.hidden {
            continue;
        }
        let mut at = walker_position(&settler.walk);
        // A soldier on an errand: its owner's ring, and health pips; the two
        // of a duel stand side by side.
        let health = settler.health.unwrap_or(0);
        let errand = match settler.activity {
            Activity::Attack { waiting } => Some((health, -1.0, waiting)),
            Activity::Defend { waiting } => Some((health, 1.0, waiting)),
            _ => None,
        };
        if let Some((health, side, standing)) = errand {
            if standing {
                at[0] += side * DUEL_SPREAD;
            }
            push_diamond(mesh, at, 0.0, 1.9, player_color(settler.player));
            if let Some(rank) = world.soldier_rank(settler.profession) {
                push_health(mesh, at, rank, health);
            }
        }
        push_diamond(mesh, at, 0.0, 1.4, [0.05, 0.05, 0.05]);
        push_diamond(mesh, at, 0.1, 1.0, settler_color(&settler.activity));
        if let Some(good) = settler.carrying {
            let above = [at[0], at[1] + 1.6];
            push_square(
                mesh,
                above,
                0.2,
                0.8,
                crate::render::terrain::palette_color(good),
            );
        }
    }
}

/// Draw depth of trees and stone piles: above the terrain markers, below
/// the overlay.
pub const DEPOSIT_LAYER: f32 = 1.2;

/// Trees as green squares growing with their age (ECO-OUT-6), saplings as
/// small light-green ones, felled trees brown; stone piles grey, smaller as
/// they shrink.
pub fn deposit_mesh(world: &World) -> MeshData {
    let mut mesh = MeshData::default();
    let now = world.tick();
    for (field, deposit) in world.deposits() {
        let kind = world.deposit_kind(deposit.kind);
        let stone = kind.is_some_and(|k| k.grow_ticks == 0);
        let (color, half) = match deposit.state {
            DepositState::Sapling => ([0.55, 0.85, 0.45], 0.5),
            DepositState::Removing { .. } => ([0.45, 0.3, 0.15], 1.0),
            DepositState::Growing { .. } if stone => {
                let chain = chain_length(world, deposit.kind) as f32;
                ([0.82, 0.82, 0.80], 0.7 + 0.15 * chain)
            }
            DepositState::Growing { ripe_at } => {
                let grow = kind.map_or(1, |k| k.grow_ticks.max(1));
                let left = ripe_at.saturating_sub(now) as f32 / grow as f32;
                let grown = (1.0 - left).clamp(0.0, 1.0);
                ([0.03, 0.30 + 0.25 * (1.0 - grown), 0.08], 0.6 + 0.9 * grown)
            }
        };
        push_square(&mut mesh, position(field), 0.0, half, color);
    }
    mesh
}

/// Stones left in a pile: its kind and every next kind (ECO-OUT-3.6).
fn chain_length(world: &World, mut kind: u32) -> u32 {
    let mut n = 1;
    while let Some(next) = world.deposit_kind(kind).and_then(|k| k.next) {
        kind = next;
        n += 1;
        if n > 32 {
            break;
        }
    }
    n
}

/// Where a walker is drawn: along its step by its progress.
pub fn walker_position(walker: &Step) -> [f32; 2] {
    let from = position(walker.at);
    let Some(next) = walker.next else {
        return from;
    };
    let to = position(next);
    let t = walker.fraction();
    [
        from[0] + (to[0] - from[0]) * t,
        from[1] + (to[1] - from[1]) * t,
    ]
}

fn position(field: Point) -> [f32; 2] {
    field_position(field.0, field.1)
}

fn push_square(mesh: &mut MeshData, at: [f32; 2], z: f32, half: f32, rgb: [f32; 3]) {
    let corners = [[-half, -half], [half, -half], [half, half], [-half, half]];
    push_quad(mesh, corners.map(|[u, v]| [at[0] + u, at[1] + v]), z, rgb);
}

/// A rectangle of half width `half_w` and half height `half_h`.
fn push_rect(mesh: &mut MeshData, at: [f32; 2], z: f32, half_w: f32, half_h: f32, rgb: [f32; 3]) {
    let corners = [
        [-half_w, -half_h],
        [half_w, -half_h],
        [half_w, half_h],
        [-half_w, half_h],
    ];
    push_quad(mesh, corners.map(|[u, v]| [at[0] + u, at[1] + v]), z, rgb);
}

fn push_diamond(mesh: &mut MeshData, at: [f32; 2], z: f32, half: f32, rgb: [f32; 3]) {
    let corners = [[0.0, -half], [half, 0.0], [0.0, half], [-half, 0.0]];
    push_quad(mesh, corners.map(|[u, v]| [at[0] + u, at[1] + v]), z, rgb);
}

/// A strip of half width `half` from `a` to `b`.
fn push_segment(mesh: &mut MeshData, a: [f32; 2], b: [f32; 2], half: f32, rgb: [f32; 3]) {
    let (du, dv) = (b[0] - a[0], b[1] - a[1]);
    let length = du.hypot(dv).max(f32::EPSILON);
    // Perpendicular to the left of a -> b.
    let (nu, nv) = (-dv / length * half, du / length * half);
    let corners = [
        [a[0] - nu, a[1] - nv],
        [b[0] - nu, b[1] - nv],
        [b[0] + nu, b[1] + nv],
        [a[0] + nu, a[1] + nv],
    ];
    push_quad(mesh, corners, 0.0, rgb);
}

/// A translucent rectangle.
fn push_rect_alpha(
    mesh: &mut MeshData,
    at: [f32; 2],
    z: f32,
    half_w: f32,
    half_h: f32,
    rgb: [f32; 3],
    alpha: f32,
) {
    let corners = [
        [-half_w, -half_h],
        [half_w, -half_h],
        [half_w, half_h],
        [-half_w, half_h],
    ];
    push_quad_alpha(
        mesh,
        corners.map(|[u, v]| [at[0] + u, at[1] + v]),
        z,
        rgb,
        alpha,
    );
}

/// Four corners, counter-clockwise.
fn push_quad(mesh: &mut MeshData, corners: [[f32; 2]; 4], z: f32, rgb: [f32; 3]) {
    push_quad_alpha(mesh, corners, z, rgb, 1.0);
}

fn push_quad_alpha(mesh: &mut MeshData, corners: [[f32; 2]; 4], z: f32, rgb: [f32; 3], alpha: f32) {
    let first = mesh.positions.len() as u32;
    let [r, g, b] = rgb.map(crate::render::terrain::srgb_to_linear);
    for [u, v] in corners {
        mesh.positions.push([u, v, z]);
        mesh.colors.push([r, g, b, alpha]);
    }
    mesh.indices
        .extend([0, 1, 2, 0, 2, 3].map(|corner| first + corner));
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_sim::map::{state, Terrain};
    use s2_sim::{BuildingType, Command, PlayerId};

    const P0: PlayerId = PlayerId(0);

    fn flat_world(width: u32, height: u32) -> World {
        let fields = (width * height) as usize;
        let terrain = Terrain {
            width,
            height,
            heights: vec![0; fields],
            height_floor: -100,
            states: vec![state::FOR_BUILDING; fields],
            samples: Vec::new(),
        };
        let rules = s2_sim::Rules {
            building_types: vec![{
                let mut sawmill = BuildingType::new(7, BuildingSize::Medium);
                sawmill.frame_costs = vec![(1, 2)];
                sawmill.costs = vec![(2, 2)];
                sawmill
            }],
            ..s2_sim::Rules::empty()
        };
        World::with_map(1, terrain, rules, P0)
    }

    // The tint is one translucent rectangle per run of one owner in a row.
    // spec: none (the 2D view's territory drawing)
    #[test]
    fn territory_is_one_rectangle_per_row_run() {
        let world = flat_world(6, 4);
        let mesh = territory_mesh(&world);
        assert_eq!(quads(&mesh), 4);
        assert!(mesh.colors.iter().all(|c| c[3] > 0.0 && c[3] < 1.0));
    }

    fn quads(mesh: &MeshData) -> usize {
        assert_eq!(mesh.positions.len() % 4, 0);
        assert_eq!(mesh.indices.len(), mesh.positions.len() / 4 * 6);
        mesh.positions.len() / 4
    }

    fn center(mesh: &MeshData, quad: usize) -> [f32; 2] {
        let corners = &mesh.positions[4 * quad..4 * quad + 4];
        [0, 1].map(|axis| corners.iter().map(|c| c[axis]).sum::<f32>() / 4.0)
    }

    // spec: ECO-PLACE-4.1
    #[test]
    fn overlay_marks_every_field_that_takes_something() {
        let world = flat_world(10, 10);
        let marked = world
            .terrain()
            .fields()
            .filter(|&f| world.placement_at(f) != Placement::Nothing)
            .count();
        let mesh = overlay_mesh(&world);
        assert_eq!(quads(&mesh), marked);
        assert!(marked > 0);
        // The first mark is on the first marked field.
        let first = world
            .terrain()
            .fields()
            .find(|&f| world.placement_at(f) != Placement::Nothing)
            .unwrap();
        let [u, v] = center(&mesh, 0);
        let [fu, fv] = position(first);
        assert!((u - fu).abs() < 1e-4 && (v - fv).abs() < 1e-4);
    }

    // spec: ECO-PLACE-4.1
    #[test]
    fn classes_have_distinct_marks() {
        let classes = [
            Placement::Flag,
            Placement::Small,
            Placement::Medium,
            Placement::Large,
            Placement::Mine,
            Placement::Harbor,
        ];
        let colors: Vec<_> = classes.map(|c| overlay_style(c).unwrap().0).to_vec();
        for (i, a) in colors.iter().enumerate() {
            for b in &colors[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert_eq!(overlay_style(Placement::Nothing), None);
    }

    // spec: none (the 2D view's object drawing)
    #[test]
    fn objects_flags_roads_and_buildings() {
        let mut world = flat_world(14, 12);
        for command in [
            Command::PlaceFlag {
                player: P0,
                at: (2, 4),
            },
            Command::PlaceFlag {
                player: P0,
                at: (8, 4),
            },
            Command::BuildRoad {
                player: P0,
                from: (2, 4),
                to: (8, 4),
            },
            Command::PlaceBuilding {
                player: P0,
                building: 7,
                at: (5, 9),
            },
        ] {
            world.submit(command);
        }
        world.step();
        assert!(world.last_results().iter().all(|r| r.outcome.is_ok()));
        let mesh = object_mesh(&world);
        // A road of 6 steps, one building (outline and fill), three flags
        // (the building's own included), each with an outline.
        assert_eq!(quads(&mesh), 6 + 2 + 3 * 2);
        // The road strip starts at the first flag.
        let [u, v] = center(&mesh, 0);
        let [a, b] = [position((2, 4)), position((3, 4))];
        assert!((u - (a[0] + b[0]) / 2.0).abs() < 1e-4);
        assert!((v - (a[1] + b[1]) / 2.0).abs() < 1e-4);
    }

    #[test]
    fn carriers_and_wares() {
        // A game: the HQ sends the road's carrier (ECO-REQ-1.1).
        let fields = 30 * 30;
        let terrain = Terrain {
            width: 30,
            height: 30,
            heights: vec![0; fields],
            height_floor: -100,
            states: vec![state::FOR_BUILDING; fields],
            samples: Vec::new(),
        };
        let mut castle = BuildingType::new(9, BuildingSize::Large);
        castle.territory_range = 9;
        castle.explore_radius = 12;
        castle.warehouse = true;
        castle.worker_limit = 0;
        let mut rules = s2_sim::Rules {
            building_types: vec![castle],
            headquarters: Some(9),
            start_stock: vec![(20, 5)],
            ..s2_sim::Rules::empty()
        };
        rules.roles.helper = Some(20);
        rules.transport_priorities.insert(5, 7);
        let setup = s2_sim::GameSetup {
            players: vec![(P0, (10, 10))],
            start_resources: 0,
            local_player: P0,
            ..Default::default()
        };
        let mut world = World::new_game(1, terrain, rules, &setup);
        let (hq_flag, end) = ((10, 9), (14, 9));
        world.submit(Command::PlaceFlag {
            player: P0,
            at: end,
        });
        world.step();
        world.submit(Command::BuildRoad {
            player: P0,
            from: hq_flag,
            to: end,
        });
        world.step();
        // The open request is marked over the road's middle field.
        assert_eq!(quads(&moving_mesh(&world)), 1);
        // The carrier walks out of the HQ and onto its idle field (12, 9).
        for _ in 0..200 {
            world.step();
        }
        assert_eq!(world.carriers()[0].walker.at, (12, 9));
        for _ in 0..2 {
            world.submit(Command::DropWare {
                player: P0,
                good: 5,
                at: end,
                destination: hq_flag,
            });
        }
        world.step();
        assert!(world.last_results().iter().all(|r| r.outcome.is_ok()));
        // One carrier (outline and fill) and two wares at the flag.
        let mesh = moving_mesh(&world);
        assert_eq!(quads(&mesh), 2 + 2);
        let [u, v] = center(&mesh, 0);
        let [fu, fv] = position((12, 9));
        assert!((u - fu).abs() < 1e-4 && (v - fv).abs() < 1e-4);
    }

    // A site with a builder: a progress bar (outline only at first) and the
    // builder; after some work the bar has a fill.
    // spec: ECO-HQ-4.7
    #[test]
    fn construction_progress() {
        let mut world = flat_world(14, 12);
        for command in [
            Command::PlaceBuilding {
                player: P0,
                building: 7,
                at: (5, 6),
            },
            Command::DebugSupplySite {
                player: P0,
                at: (5, 6),
                wares: vec![(1, 2), (2, 2)],
                workers: true,
            },
        ] {
            world.submit(command);
        }
        world.step();
        assert!(world.last_results().iter().all(|r| r.outcome.is_ok()));
        // ECO-HQ-4.6b: the builder starts with his approach walk, so no work
        // is done yet: the outline only, and the builder.
        assert_eq!(quads(&moving_mesh(&world)), 2);
        // After the approach (68 ticks) and the first steps: a fill.
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(quads(&moving_mesh(&world)), 3);
    }

    // A real game: border marks in the owner's colour, the headquarters in it
    // too.
    // spec: TER-5.1
    #[test]
    fn borders_and_headquarters() {
        let fields = 40 * 40;
        let terrain = Terrain {
            width: 40,
            height: 40,
            heights: vec![0; fields],
            height_floor: -100,
            states: vec![state::FOR_BUILDING; fields],
            samples: Vec::new(),
        };
        let mut castle = BuildingType::new(9, BuildingSize::Large);
        castle.territory_range = 3;
        let rules = s2_sim::Rules {
            building_types: vec![castle],
            headquarters: Some(9),
            ..s2_sim::Rules::empty()
        };
        let setup = s2_sim::GameSetup {
            players: vec![(PlayerId(1), (20, 20))],
            start_resources: 0,
            local_player: PlayerId(1),
            ..Default::default()
        };
        let world = World::new_game(1, terrain, rules, &setup);
        let mesh = object_mesh(&world);
        // The headquarters (outline and fill) and its flag (outline and fill).
        assert_eq!(quads(&mesh), 2 + 2);
        // TER-7.2: ring 3 has 18 Border fields, each with one stone (a dark
        // outline and the owner's colour) and no buoy on land.
        let stones = border_mesh(&world, false);
        assert_eq!(quads(&stones), 18 * 2);
        assert_eq!(quads(&border_mesh(&world, true)), 0);
        let blue_or_red = crate::render::terrain::srgb_to_linear(player_color(PlayerId(1))[0]);
        assert_eq!(stones.colors[4][0], blue_or_red);
    }

    // TER-7.2: a Border field with Water gets a buoy, not a stone.
    #[test]
    fn water_borders_carry_buoys() {
        let fields = 40 * 40;
        let mut terrain = Terrain {
            width: 40,
            height: 40,
            heights: vec![0; fields],
            height_floor: -100,
            states: vec![state::FOR_BUILDING; fields],
            samples: Vec::new(),
        };
        for f in s2_sim::grid::ring((20, 20), 3, 40, 40) {
            let index = terrain.index(f);
            terrain.states[index] = state::WATER;
        }
        let mut castle = BuildingType::new(9, BuildingSize::Large);
        castle.territory_range = 3;
        let rules = s2_sim::Rules {
            building_types: vec![castle],
            headquarters: Some(9),
            ..s2_sim::Rules::empty()
        };
        let setup = s2_sim::GameSetup {
            players: vec![(PlayerId(1), (20, 20))],
            start_resources: 0,
            local_player: PlayerId(1),
            ..Default::default()
        };
        let world = World::new_game(1, terrain, rules, &setup);
        assert_eq!(quads(&border_mesh(&world, false)), 0);
        assert_eq!(quads(&border_mesh(&world, true)), 18 * 3);
    }

    // spec: GFX-VIEW-4.2
    #[test]
    fn walkers_are_drawn_along_their_step() {
        let mut walker = Step::standing((2, 4));
        assert_eq!(walker_position(&walker), position((2, 4)));
        walker.next = Some((3, 4));
        walker.done = walker.total / 2;
        let [u, v] = walker_position(&walker);
        let ([a, b], [c, d]) = (position((2, 4)), position((3, 4)));
        assert!((u - (a + c) / 2.0).abs() < 1e-4 && (v - (b + d) / 2.0).abs() < 1e-4);
    }

    // spec: none (the 2D view's road segments)
    #[test]
    fn segments_turn_counter_clockwise() {
        let mut mesh = MeshData::default();
        push_segment(&mut mesh, [0.0, 0.0], [3.0, 5.0], 1.0, [1.0, 1.0, 1.0]);
        let p = &mesh.positions;
        let cross =
            (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
        assert!(cross > 0.0);
    }
}
