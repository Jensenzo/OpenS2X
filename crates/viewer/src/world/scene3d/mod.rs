//! What the 3D view draws from the simulation, as plain data: where each
//! thing stands and how it is turned (`specs/GFX-world-view.md` GFX-VIEW-4)
//! and the build-help marks (GFX-VIEW-6). Positions are in the game frame
//! (`crate::render::world3d`); the Bevy side converts them.

mod movers;
mod objects;
mod slots;
mod streets;

pub(crate) use movers::*;
pub(crate) use objects::*;
pub(crate) use slots::*;
pub(crate) use streets::*;

use std::collections::{HashMap, HashSet};

use s2_assets::model_tables::ModelTables;
use s2_sim::grid::Point;
use s2_sim::map::state;
use s2_sim::{DepositState, World};

use crate::animation::settler_anim::{Request, SettlerState};
use crate::render::models3d;
use crate::render::world3d::{
    ground_height, Frame, Heights, SAMPLE_ROW_STEP, SAMPLE_STEP, WATER_LEVEL,
};

/// Vertex positions (Bevy frame), linear RGBA colours and indices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Marks {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl Marks {
    /// A square ring: four thin squares along the sides of a square.
    fn push_ring(&mut self, world: &World, x: f32, z: f32, half: f32, rgb: [f32; 3]) {
        let w = 0.25;
        for (dx, dz, hx, hz) in [
            (0.0, -half, half, w),
            (0.0, half, half, w),
            (-half, 0.0, w, half),
            (half, 0.0, w, half),
        ] {
            self.push_rect(world, x + dx, z + dz, hx, hz, rgb);
        }
    }

    fn push_rect(&mut self, world: &World, x: f32, z: f32, hx: f32, hz: f32, rgb: [f32; 3]) {
        let first = self.positions.len() as u32;
        // Stored values (the map camera's, GFX-VIEW-2.6 c1).
        let [r, g, b] = rgb;
        for [dx, dz] in [[-hx, -hz], [hx, -hz], [hx, hz], [-hx, hz]] {
            let (px, pz) = (x + dx, z + dz);
            let y = ground_height(world.terrain(), px, pz) + MARK_LIFT;
            self.positions
                .push(crate::render::world3d::to_bevy([px, y, pz]));
            self.colors.push([r, g, b, 1.0]);
        }
        self.indices
            .extend([0, 1, 2, 0, 2, 3].map(|corner| first + corner));
    }
}

/// GFX-VIEW-6.1: build-help icons are 1.6 x 1.6 units (`iconSize`) and
/// moved 1 unit toward the camera in view space (`iconOffset` (0, 0, -1)).
pub const ICON_SIZE: f32 = 1.6;
pub const ICON_OFFSET: f32 = 1.0;
/// Height of the marks over the ground, so they are not hidden in it.
const MARK_LIFT: f32 = 0.15;

/// GFX-OBJ-2.6b: the world height of the plane that cuts a house being
/// built: (H - 0.1) x m x 1.1111112 above its base, H the house model's
/// height and m the main progress.
pub fn rising_plane(base: f32, height: f32, main: f32) -> f32 {
    base + (height - 0.1).max(0.0) * main * 1.111_111_2
}

/// GFX-OBJ-2.6a: how many of a site model's N pieces stand at frame
/// progress f: min(N, floor((N + 1) f + 0.5)), in 32-bit floats.
pub fn site_stage(pieces: usize, frame: f32) -> usize {
    let s = ((pieces as f32 + 1.0) * frame + 0.5).floor();
    (s.max(0.0) as usize).min(pieces)
}

/// GFX-ANIM-4.1, 4.1a: a mover's drawn position: the previous and the
/// current tick's target, blended linearly over the tick. Any jump of the
/// target is drawn as one straight slide lasting one tick. A settler that
/// stands keeps the target of his last moving tick (the tick he arrives in
/// still takes the logic position), so a logic change while he stands is
/// drawn when he moves again, as a one-tick slide from the old spot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub previous: [f32; 3],
    pub current: [f32; 3],
    moving: bool,
}

impl Motion {
    pub fn new(at: [f32; 3]) -> Self {
        Self {
            previous: at,
            current: at,
            moving: true,
        }
    }

    /// A logic tick: `target` from the logic, `moving` whether the mover
    /// moves in it.
    pub fn tick(&mut self, target: [f32; 3], moving: bool) {
        self.previous = self.current;
        if moving || self.moving {
            self.current = target;
        }
        self.moving = moving;
    }

    /// The drawn position at the fraction `t` (0..1) of the tick.
    pub fn at(&self, t: f32) -> [f32; 3] {
        [0, 1, 2].map(|i| self.previous[i] + (self.current[i] - self.previous[i]) * t)
    }

    /// The drawn position changes in this tick.
    pub fn slides(&self) -> bool {
        self.current != self.previous
    }
}

/// GFX-ANIM-4.3: a settler for the evading rule: position (game x, z), the
/// heading (GFX-VIEW-4.1), whether it walks, and its sideways offset so far
/// (signed, world units, to its right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evader {
    pub at: [f32; 2],
    pub heading: f32,
    pub moving: bool,
    pub offset: f32,
}

/// GFX-ANIM-4.3 Evading: the offsets after `dt` seconds. Each offset
/// decays by 0.75 units a second toward 0; two settlers heading toward
/// each other (dot product of their directions <= 0) within R = evadeStart
/// x scale get (1 - d^2 / R^2) x scale x evadeRadiusScale, half each when
/// both walk, all to the one that walks when the other stands; the side is
/// the one it already leans to (its right when it does not lean yet, so
/// both pass the same way, OBS-5). An offset already larger is kept.
pub fn evade(evaders: &[Evader], start: f32, radius_scale: f32, scale: f32, dt: f32) -> Vec<f32> {
    let decay = 0.75 * dt;
    let mut out: Vec<f32> = evaders
        .iter()
        .map(|e| e.offset.signum() * (e.offset.abs() - decay).max(0.0))
        .collect();
    let r = start * scale;
    if r <= 0.0 {
        return out;
    }
    let dir = |h: f32| [h.sin(), h.cos()];
    for i in 0..evaders.len() {
        for j in i + 1..evaders.len() {
            let (a, b) = (&evaders[i], &evaders[j]);
            if !a.moving && !b.moving {
                continue;
            }
            let (dx, dz) = (b.at[0] - a.at[0], b.at[1] - a.at[1]);
            let d2 = dx * dx + dz * dz;
            if d2 >= r * r {
                continue;
            }
            let (da, db) = (dir(a.heading), dir(b.heading));
            if da[0] * db[0] + da[1] * db[1] > 0.0 {
                continue;
            }
            let full = (1.0 - d2 / (r * r)) * scale * radius_scale;
            let share = |me: &Evader, other: &Evader| match (me.moving, other.moving) {
                (true, true) => full / 2.0,
                (true, false) => full,
                _ => 0.0,
            };
            for (k, me, other) in [(i, a, b), (j, b, a)] {
                let want = share(me, other);
                if want > out[k].abs() {
                    let side = if out[k] < 0.0 || me.offset < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    out[k] = side * want;
                }
            }
        }
    }
    out
}

/// GFX-ANIM-4.3: a settler's right side in the game frame (x, z) for its
/// heading.
pub fn right_of(heading: f32) -> [f32; 2] {
    [heading.cos(), -heading.sin()]
}

/// ECO-PLACE-5.3: the field a decoration on the sample `sample` stands on:
/// the field whose sample point is nearest to it; exactly halfway between
/// two fields of a row, the one with the higher x (T17); else the first of
/// equals, row by row. The distances are taken from whole half-sample
/// differences, so equal ones compare equal.
pub fn decoration_field(sample: (u32, u32), width: u32, height: u32) -> Point {
    // GFX-VIEW-1.1: odd sample rows lie half a sample to the right.
    let half_column = |c: u32, r: u32| i64::from(2 * c + r % 2);
    let (sc, sr) = (half_column(sample.0, sample.1), i64::from(sample.1));
    let row = sample.1 / 4;
    let mut best = (f32::MAX, (0, 0));
    for y in row.saturating_sub(1)..=(row + 1).min(height.saturating_sub(1)) {
        let column = sample.0 / 4;
        for x in column.saturating_sub(1)..=(column + 1).min(width.saturating_sub(1)) {
            // MAP-GRID-4.2: a field's own sample.
            let (c, r) = (4 * x + 2 * (y % 2), 4 * y);
            let dx = SAMPLE_STEP / 2.0 * (half_column(c, r) - sc) as f32;
            let dz = SAMPLE_ROW_STEP * (i64::from(r) - sr) as f32;
            let d = dx * dx + dz * dz;
            if d < best.0 || (d == best.0 && y == best.1 .1) {
                best = (d, (x, y));
            }
        }
    }
    best.1
}

/// GFX-ANIM-3.8a (corrected Run D): the tree starts to fall in the first
/// tick in which 3.0 s of game time (30 ticks) have passed since the
/// woodcutter's work action started; its turn rate starts at (pi / 2) /
/// (20 x globalSpeed) per second and grows 1.2 times per tick; each tick
/// turns it by a tenth of the rate (ticks of 0.1 s).
pub const FALL_START_TICKS: u64 = 30;
pub const FALL_FIRST_TURN: f32 =
    std::f32::consts::FRAC_PI_2 / (20.0 * crate::animation::anim::GLOBAL_SPEED);
pub const FALL_GROWTH: f32 = 1.2;
const TICKS_PER_SECOND: f32 = 10.0;

/// GFX-ANIM-3.8a: the tick a tree whose felling work began in tick
/// `began` starts to fall (the first of its fall ticks).
pub fn fall_start(began: u64) -> u64 {
    began + FALL_START_TICKS
}

/// GFX-ANIM-3.8a: a tree's state in its fall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fall {
    Standing,
    /// Turned by this angle (radians), interpolated within the tick.
    Falling(f32),
    /// The tick whose angle would pass pi / 2 (the 24th) stopped it: it
    /// keeps the matrix of the last frame drawn while falling (no clamp).
    Lying,
}

/// GFX-ANIM-3.8a, T11: how far a tree has turned at `now` (ticks with
/// their fraction): after the j-th fall tick FALL_FIRST_TURN / 10 x
/// (1.2^j - 1) / 0.2, drawn fully at the start of the next tick and
/// interpolated toward it during the tick (GFX-ANIM-4.1); lying from the
/// tick whose angle would pass pi / 2.
pub fn fall_angle(began: Option<u64>, now: f32) -> Fall {
    let Some(began) = began else {
        return Fall::Standing;
    };
    let k = now - fall_start(began) as f32;
    if k < 0.0 {
        return Fall::Standing;
    }
    let turned = |n: f32| {
        FALL_FIRST_TURN / TICKS_PER_SECOND * (FALL_GROWTH.powf(n) - 1.0) / (FALL_GROWTH - 1.0)
    };
    let (n, f) = (k.floor(), k.fract());
    if turned(n + 1.0) > std::f32::consts::FRAC_PI_2 {
        return Fall::Lying;
    }
    Fall::Falling(turned(n) + (turned(n + 1.0) - turned(n)) * f)
}

/// GFX-ANIM-3.8c, T11a, T11b: how far a deposit being removed (since tick
/// `since`, gone in `gone_at`) has sunk at `now`, as a share of its box
/// height h: each tick's target is 0.05 x its removal counter (20 -
/// (gone_at - t) at the end of tick t, ECO-OUT-6.4), drawn fully at the
/// start of the next tick and interpolated toward it during the tick, from
/// 0 in the tick the removal starts.
pub fn sink_fraction(since: u64, gone_at: u64, now: f32) -> f32 {
    if now < since as f32 {
        return 0.0;
    }
    let t = now.floor() as u64;
    let target = |t: u64| 0.05 * (20.0 - gone_at.saturating_sub(t) as f32).max(0.0);
    let previous = if t == since { 0.0 } else { target(t - 1) };
    previous + (target(t) - previous) * now.fract()
}

/// GFX-ANIM-3.8b: the scale of a growing deposit of size s on top of its
/// element's: height s, width min(3 s, 1).
pub fn growth_scale(s: f32) -> [f32; 3] {
    let w = (3.0 * s).min(1.0);
    [w, s, w]
}

/// GFX-ANIM-3.8a: the horizontal axis a tree falls about, from the tree
/// toward the woodcutter working at it, as a yaw in the game frame (the
/// angle of the axis from +x toward +z); kept per tree while it is worked
/// at, felled and lying.
pub fn update_fall_axes(world: &World, frame: &Frame, axes: &mut HashMap<Point, f32>) {
    axes.retain(|&field, _| world.felling_began(field).is_some());
    let views = world.settler_views();
    let worker_in = |v: &s2_sim::view::SettlerView, p: s2_sim::view::WorkPhase| matches!(v.activity, s2_sim::view::Activity::Worker { phase, .. } if phase == p);
    for view in &views {
        let (true, Some(spot)) = (
            worker_in(view, s2_sim::view::WorkPhase::AtSpot),
            view.work_spot,
        ) else {
            continue;
        };
        if axes.contains_key(&spot) || world.felling_began(spot).is_none() {
            continue;
        }
        let (Some(deposit), Some(fine)) = (world.deposit(spot), view.fine) else {
            continue;
        };
        let [tx, tz] =
            frame.sample_xz(i64::from(deposit.position.0), i64::from(deposit.position.1));
        let (at, _) = fine_point(world, frame, &fine);
        axes.insert(spot, (at[2] - tz).atan2(at[0] - tx));
    }
    // A tree felled while the view did not see its woodcutter at it (a
    // game played ahead, a loaded save): while he takes the log he still
    // stands at his stand, a few lattice steps from the tree (the nearest
    // felled one within 3), so the axis is the same line.
    for view in &views {
        let (true, Some(stand)) = (
            worker_in(view, s2_sim::view::WorkPhase::TakeWare),
            view.stand,
        ) else {
            continue;
        };
        let tree = world
            .deposits()
            .filter(|(f, d)| {
                !axes.contains_key(f)
                    && world.fall_of(*f).is_some()
                    && s2_sim::grid::distance(stand, d.position) <= 3
            })
            .min_by_key(|(_, d)| s2_sim::grid::distance(stand, d.position))
            .map(|(f, d)| (f, d.position));
        if let (Some((field, position)), Some(fine)) = (tree, view.fine) {
            let [tx, tz] = frame.sample_xz(i64::from(position.0), i64::from(position.1));
            let (at, _) = fine_point(world, frame, &fine);
            axes.insert(field, (at[2] - tz).atan2(at[0] - tx));
        }
    }
    // Any other: toward +x (stand-in).
    for (field, _) in world.deposits() {
        if !axes.contains_key(&field) && world.fall_of(field).is_some() {
            axes.insert(field, 0.0);
        }
    }
}

/// The first flag of a road being built: a yellow ring (a viewer aid,
/// not the game's).
pub fn marks(world: &World, frame: &Frame, road_start: Option<Point>) -> Marks {
    let mut marks = Marks::default();
    if let Some(field) = road_start {
        let [x, z] = frame.field_xz(field.0, field.1);
        marks.push_ring(world, x, z, 1.6, [1.0, 0.85, 0.1]);
    }
    marks
}

/// GFX-VIEW-6.1: the icon atlas's u of a build class (eight icons a row).
pub fn icon_column(class: s2_sim::Placement) -> Option<f32> {
    use s2_sim::Placement as P;
    match class {
        P::Nothing => None,
        P::Flag => Some(0.375),
        P::Small => Some(0.25),
        P::Medium => Some(0.125),
        P::Large => Some(0.0),
        P::Mine => Some(0.5),
        P::Harbor => Some(0.625),
    }
}

/// GFX-VIEW-6.1: the build-help icons: for every field whose build class
/// is not 0, except the cursor field, the icon's anchor (game frame, the
/// field's world point) and its atlas column.
pub fn build_icons(world: &World, frame: &Frame, cursor: Option<Point>) -> Vec<([f32; 3], f32)> {
    let mut out = Vec::new();
    for field in world.terrain().fields() {
        if Some(field) == cursor {
            continue;
        }
        let Some(u) = icon_column(world.placement_at(field)) else {
            continue;
        };
        let [x, z] = frame.field_xz(field.0, field.1);
        let y = ground_height(world.terrain(), x, z);
        out.push(([x, y, z], u));
    }
    out
}

/// GFX-VIEW-6.1: the build-help quads (Bevy frame), facing the camera
/// whose right, up and forward (into the screen) vectors are given: each
/// quad's bottom edge is centred on its anchor moved [`ICON_OFFSET`]
/// toward the camera along the view axis, reaching [`ICON_SIZE`] up and
/// half of it to each side; with their atlas coordinates: positions, uvs
/// and indices.
pub fn billboards(
    icons: &[([f32; 3], f32)],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
    let (mut positions, mut uvs, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    let h = ICON_SIZE / 2.0;
    for (at, u) in icons {
        let anchor = crate::render::world3d::to_bevy(*at);
        let c = [0, 1, 2].map(|k| anchor[k] - forward[k] * ICON_OFFSET);
        let first = positions.len() as u32;
        for (sx, sy, uu, vv) in [
            (-1.0, 0.0, *u, 1.0),
            (1.0, 0.0, u + 0.125, 1.0),
            (1.0, 2.0, u + 0.125, 0.0),
            (-1.0, 2.0, *u, 0.0),
        ] {
            positions.push([0, 1, 2].map(|k| c[k] + (right[k] * sx + up[k] * sy) * h));
            uvs.push([uu, vv]);
        }
        indices.extend([0, 1, 2, 0, 2, 3].map(|k| first + k));
    }
    (positions, uvs, indices)
}

/// GFX-VIEW-6.2: a cursor sprite's top-left corner in `uiElements3`.
pub const CURSOR_SPRITE: [u32; 2] = [224, 96];
pub const NO_ROAD_SPRITE: [u32; 2] = [192, 96];
pub const ROAD_ON_SPRITE: [u32; 2] = [288, 128];

/// GFX-VIEW-6.2: the slope arrow of a height class.
pub fn slope_sprite(class: i8) -> Option<[u32; 2]> {
    let x = match class {
        0 => 288,
        1 => 320,
        2 => 352,
        3 => 384,
        -3 => 416,
        -2 => 448,
        -1 => 480,
        _ => return None,
    };
    Some([x, 128])
}

/// UI-GAME-13.1: the cursor icon of the hovered field outside road
/// building, a 46 x 32 rectangle of `uiElements3` by the field's build
/// class; class 0 shows the flag when a flag stands there, else the
/// building with the red cross.
pub fn field_icon(world: &World, player: s2_sim::PlayerId, field: Point) -> [u32; 4] {
    use s2_sim::Placement as P;
    let (x, y) = match world.placement_at(field) {
        P::Flag => (448, 0),
        P::Small => (256, 0),
        P::Medium => (304, 0),
        P::Large => (352, 0),
        P::Mine => (400, 0),
        P::Harbor => (312, 32),
        P::Nothing if world.flag_at(field) == Some(player) => (448, 0),
        P::Nothing => (448, 66),
    };
    [x, y, 46, 32]
}

/// UI-GAME-13.2: the cursor icon while a road or waterway is built from
/// `from`: 40 x 32 at y 456: x 8 when the road may continue to the field
/// and a flag fits there, 88 when it may continue but no flag fits, 48 when
/// the road cannot use the field. "May continue" is taken as the route
/// search finding a way from the road's start (an own flag at the field is
/// a way too).
pub fn road_icon(
    world: &World,
    player: s2_sim::PlayerId,
    from: Point,
    field: Point,
    waterway: bool,
) -> [u32; 4] {
    let reachable = field != from
        && if waterway {
            world.find_waterway(from, field).is_ok()
        } else {
            world.find_route(player, from, field).is_ok()
        };
    let flag = world.flag_at(field) == Some(player) || world.check_flag(player, field).is_ok();
    let x = match (reachable, flag) {
        (true, true) => 8,
        (true, false) => 88,
        (false, _) => 48,
    };
    [x, 456, 40, 32]
}

/// GFX-VIEW-6.2 (road building only): the ring on the cursor field and,
/// for each neighbour, for a land road the "no road" sprite where the road
/// cannot continue to it, else the slope arrow by the height class between
/// the cursor and the neighbour; for a waterway the "can continue" sprite
/// where the route may go on and nothing otherwise. Whether a road can go
/// through a neighbour is taken as the road search finding the one-step
/// route (Q-183).
pub fn cursor_sprites(
    world: &World,
    player: s2_sim::PlayerId,
    cursor: Point,
    waterway: bool,
) -> Vec<(Point, [u32; 2])> {
    let mut out = vec![(cursor, CURSOR_SPRITE)];
    let terrain = world.terrain();
    for d in 0..6 {
        let Some(n) = terrain.neighbour(cursor, d) else {
            continue;
        };
        let open = if waterway {
            world
                .find_waterway(cursor, n)
                .is_ok_and(|(path, _)| path.len() == 2)
        } else {
            world
                .find_route(player, cursor, n)
                .is_ok_and(|(path, _)| path.len() == 2)
        };
        let sprite = if waterway {
            if !open {
                continue;
            }
            ROAD_ON_SPRITE
        } else if !open {
            NO_ROAD_SPRITE
        } else {
            let class = s2_sim::grid::height_class(
                i64::from(terrain.height_at(n)) - i64::from(terrain.height_at(cursor)),
            );
            slope_sprite(class).unwrap_or(NO_ROAD_SPRITE)
        };
        out.push((n, sprite));
    }
    out
}

#[cfg(test)]
mod tests;
