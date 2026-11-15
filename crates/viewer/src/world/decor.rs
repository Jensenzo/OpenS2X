//! Building decorations of the 3D view (`specs/GFX-objects.md` GFX-OBJ-8):
//! the soldier and border flags, the status icons and the upgrade cut, as
//! plain functions in the game frame.

/// A model's bounding box in the game frame: lowest and highest corner.
pub type Box3 = [[f32; 3]; 2];

/// GFX-OBJ-8.1: the soldier flag's model, six meshes `0` to `5`.
pub const SOLDIER_FLAG: &str = "data/buildings/MilitaryFlag1.GR2";
/// GFX-OBJ-8.2: the border-distance flag's model.
pub const BORDER_FLAG: &str = "data/buildings/MilitaryFlag2.GR2";

/// GFX-OBJ-8.1a: the flag model is turned by 90 degrees in the building's
/// frame (its banners, along the pole model's +x, fly toward the
/// building's front, -z) before the building's heading.
pub const FLAG_TURN: f32 = 90.0;

fn centre_and_size(b: &Box3) -> ([f32; 3], [f32; 3]) {
    let centre = [0, 1, 2].map(|k| (b[0][k] + b[1][k]) / 2.0);
    let size = [0, 1, 2].map(|k| b[1][k] - b[0][k]);
    (centre, size)
}

/// GFX-OBJ-8.1a: where the soldier flag stands in the building's own
/// frame: the front right corner of the house's box, at its base.
pub fn soldier_flag_local(house: &Box3) -> [f32; 3] {
    let (c, s) = centre_and_size(house);
    [c[0] + 0.33 * s[0], 0.0, c[2] - 0.55 * s[2]]
}

/// GFX-OBJ-8.2: the border flag at the front left corner.
pub fn border_flag_local(house: &Box3) -> [f32; 3] {
    let (c, s) = centre_and_size(house);
    [c[0] - 0.33 * s[0], 0.0, c[2] - 0.55 * s[2]]
}

/// A point of a building's frame in the world: turned by the building's
/// heading (degrees, GFX-VIEW-4.1's sense: +z goes to (sin a, 0, cos a))
/// and moved to its position.
pub fn building_to_world(heading_deg: f32, at: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    let (s, c) = heading_deg.to_radians().sin_cos();
    [
        at[0] + p[0] * c + p[2] * s,
        at[1] + p[1],
        at[2] - p[0] * s + p[2] * c,
    ]
}

/// GFX-OBJ-8.1: how many of the soldier flag's meshes show (`0` to `n`):
/// n = the soldiers inside, at most 5; the headquarters always 5.
pub fn soldier_flag_meshes(inside: usize, headquarters: bool) -> usize {
    if headquarters {
        6
    } else {
        inside.min(5) + 1
    }
}

/// GFX-OBJ-8.2: the border flag's texture row i (shifted down by i / 3)
/// by the enemy distance n: 0 far (n > 24), 1 medium (19 to 24), 2 near
/// (n <= 18, also while n is -1).
pub fn border_flag_row(enemy_distance: i32) -> u32 {
    match enemy_distance {
        n if n > 24 => 0,
        19..=24 => 1,
        _ => 2,
    }
}

/// GFX-OBJ-8.3: the status effects over a building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusIcon {
    Sleep,
    ProductionOff,
    NoSoldier,
    NoCoin,
    NoSoldierAndCoin,
}

impl StatusIcon {
    /// The effect's script name (`data/scripts/effects/<name>.pfx`).
    pub fn effect(self) -> &'static str {
        match self {
            Self::Sleep => "Sleep",
            Self::ProductionOff => "Sleep_ProductionOff",
            Self::NoSoldier => "Sleep_NoSoldier",
            Self::NoCoin => "Sleep_NoCoin",
            Self::NoSoldierAndCoin => "Sleep_NoSoldierAndCoin",
        }
    }

    /// Its icon in `particles_texture.tga`: u0, v0, u1, v1 (GFX-OBJ-8.3's
    /// table; the view takes it from the `.pfx`, 8.3a).
    #[cfg(test)]
    pub fn rect(self) -> [f32; 4] {
        match self {
            Self::Sleep => [0.0, 0.3125, 0.0625, 0.375],
            Self::ProductionOff => [0.0625, 0.3125, 0.125, 0.375],
            Self::NoSoldier => [0.125, 0.3125, 0.1875, 0.375],
            Self::NoCoin => [0.1875, 0.3125, 0.25, 0.375],
            // 8.3a: the file stores v1 0.4125.
            Self::NoSoldierAndCoin => [0.125, 0.3125, 0.25, 0.4125],
        }
    }
}

/// GFX-OBJ-8.3: seconds a production building idles at 25 % productivity
/// or less before it shows `Sleep`.
pub const IDLE_SECONDS: f32 = 10.0;

/// GFX-OBJ-8.3: the idle timer of one building: the game step while its
/// productivity is at most 0.25 (-1 before its first product), reset
/// otherwise.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct IdleTimer {
    pub seconds: f32,
}

impl IdleTimer {
    /// Advance by a frame's game step `dt` (0 while paused, TRACE-5.3); true once it has idled
    /// [`IDLE_SECONDS`].
    pub fn update(&mut self, productivity: f32, dt: f32) -> bool {
        if productivity <= 0.25 {
            self.seconds += dt;
        } else {
            self.seconds = 0.0;
        }
        self.seconds >= IDLE_SECONDS
    }
}

/// What a building shows a status icon for (GFX-OBJ-8.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusOf {
    /// A production building: production on, and whether it has idled
    /// long enough.
    Production { enabled: bool, idle: bool },
    /// A military building: "move soldiers out" on, coins on (the switch
    /// on and the accumulator set).
    Military { move_out: bool, coins: bool },
}

/// GFX-OBJ-8.3: the icon of a finished building of the local player that
/// is not sinking, if it has a reason to idle.
pub fn status_icon(state: StatusOf) -> Option<StatusIcon> {
    match state {
        StatusOf::Production { enabled: false, .. } => Some(StatusIcon::ProductionOff),
        StatusOf::Production { idle: true, .. } => Some(StatusIcon::Sleep),
        StatusOf::Production { .. } => None,
        StatusOf::Military {
            move_out: true,
            coins: true,
        } => Some(StatusIcon::NoSoldier),
        StatusOf::Military {
            move_out: false,
            coins: false,
        } => Some(StatusIcon::NoCoin),
        StatusOf::Military {
            move_out: true,
            coins: false,
        } => Some(StatusIcon::NoSoldierAndCoin),
        StatusOf::Military { .. } => None,
    }
}

/// GFX-OBJ-8.3a: one live particle of an effect, relative to the effect's
/// start point (game frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub at: [f32; 3],
    pub size: f32,
    pub alpha: f32,
    /// The colour's r, g, b (its start value x curve 2).
    pub colour: [f32; 3],
}

/// A value in 0..1 from the effect's seed, the particle, its life and the
/// value's number (the viewer's own draw; the original's random source is
/// not specified and is not game state).
fn unit(seed: u64, particle: u64, life: u64, n: u64) -> f32 {
    let mut h = seed ^ 0x9e37_79b9_7f4a_7c15;
    for v in [particle, life, n] {
        h = (h ^ v).wrapping_mul(0x0100_0000_01b3);
        h ^= h >> 29;
    }
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// GFX-OBJ-8.3a: the pool of an effect `elapsed` seconds (real time,
/// GFX-OBJ-2.9's clock) after it started: particle i is born (i + 1) x
/// spawn after the start and reborn every ttl with new values within the
/// variances; at age a (t = a / ttl) it is at pos + vel x curve 0 x a, of
/// size x curve 1, colour x curve 2, alpha x curve 3 (alpha clamped to 0
/// .. 1). `None` for one not yet born or done. The life ttl +- ttlVar is
/// drawn once per particle (GFX-OBJ-8.7 (3): the dust's 5 +- 3 s; an
/// effect of one round, as the dust, has one life per particle).
pub fn particles(e: &s2_assets::pfx::Effect, seed: u64, elapsed: f32) -> Vec<Option<Particle>> {
    let count = usize::try_from(e.count).unwrap_or(0);
    let spread = |base: f32, var: f32, r: f32| base + var * (2.0 * r - 1.0);
    (0..count)
        .map(|i| {
            let r = |life: u64, n: u64| unit(seed, i as u64, life, n);
            let spawn = spread(e.spawn, e.spawn_var, r(0, 0)).max(0.0);
            let since = elapsed - (i + 1) as f32 * spawn;
            let ttl = if e.ttl_var == 0.0 {
                e.ttl
            } else {
                spread(e.ttl, e.ttl_var, r(0, 8))
            };
            if since < 0.0 || ttl <= 0.0 {
                return None;
            }
            let life = (since / ttl).floor();
            if e.max_loops > 0 && life >= e.max_loops as f32 {
                return None;
            }
            let age = since - life * ttl;
            let life = life as u64 + 1;
            let t = (age / ttl).clamp(0.0, 1.0);
            let vel = [0, 1, 2].map(|k| spread(e.vel[k], e.vel_var[k], r(life, 1 + k as u64)));
            let pos = [0, 1, 2].map(|k| spread(e.pos[k], e.pos_var[k], r(life, 4 + k as u64)));
            let speed = e.curves[0].at(t);
            Some(Particle {
                at: [0, 1, 2].map(|k| pos[k] + vel[k] * speed * age),
                size: (spread(e.size, e.size_var, r(life, 7)) * e.curves[1].at(t)).max(0.0),
                alpha: (spread(e.color[3], e.color_var[3], r(life, 12)) * e.curves[3].at(t))
                    .clamp(0.0, 1.0),
                colour: [0, 1, 2].map(|k| {
                    (spread(e.color[k], e.color_var[k], r(life, 9 + k as u64)) * e.curves[2].at(t))
                        .max(0.0)
                }),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close3(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 2e-3)
    }

    const HEADING: f32 = -30.0;

    /// GFX-OBJ-8.3a: the Sleep emitter's values.
    fn sleep(var: bool) -> s2_assets::pfx::Effect {
        use s2_assets::pfx::{Curve, Effect};
        let v = if var { 1.0 } else { 0.0 };
        Effect {
            version: "0002".into(),
            emitter: "Default".into(),
            max_loops: 0,
            pos: [0.0; 3],
            pos_var: [0.0; 3],
            vel: [0.5, 2.5, 0.0],
            vel_var: [0.3 * v, 0.25 * v, 0.1 * v],
            size: 1.0,
            size_var: 0.2 * v,
            ttl: 6.0,
            ttl_var: 0.0,
            spawn: 0.8,
            spawn_var: 0.1 * v,
            color: [1.0; 4],
            color_var: [0.0; 4],
            particle: "Decal".into(),
            rect: [0.0, 0.3125, 0.0625, 0.375],
            count: 4,
            curve_set: "Default".into(),
            curves: [
                Curve::Linear(1.0, 0.6),
                Curve::Cubic([0.0, -2.0, 2.0, 1.0]),
                Curve::Constant(1.0),
                Curve::Cubic([1.0, 0.0, 0.0, -2.0]),
            ],
        }
    }

    // GFX-OBJ-8.3a (Run D): particle i is born (i + 1) x spawn after the
    // effect starts and reborn every ttl; it starts at the box centre at
    // size 0 and is at start + vel x (1 - 0.4 t) x age, size 4 t - 3 t^2 +
    // t^3, alpha 1 - t^3: over its 6 s it rises 9 units and drifts 1.8.
    #[test]
    fn sleep_icon_particles() {
        let e = sleep(false);
        let at = |elapsed: f32| particles(&e, 7, elapsed);
        assert!(at(0.79).iter().all(Option::is_none));
        let first = at(0.8)[0].expect("born");
        assert!(close3(first.at, [0.0; 3]) && first.size.abs() < 1e-6);
        // Age 3 s (t = 0.5).
        let p = at(3.8)[0].unwrap();
        assert!(
            close3(p.at, [0.5 * 0.8 * 3.0, 2.5 * 0.8 * 3.0, 0.0]),
            "{p:?}"
        );
        assert!((p.size - 1.375).abs() < 1e-4 && (p.alpha - 0.875).abs() < 1e-4);
        // Near the end of its life: 9 up, 1.8 aside, size 2, alpha 0.
        let p = at(6.7999)[0].unwrap();
        assert!(close3(p.at, [1.8, 9.0, 0.0]), "{p:?}");
        assert!((p.size - 2.0).abs() < 1e-3 && p.alpha < 1e-3);
        // Reborn at 6.8 at the centre; the others born 0.8 s apart.
        assert!(close3(at(6.8)[0].unwrap().at, [0.0; 3]));
        let born: Vec<bool> = at(2.5).iter().map(Option::is_some).collect();
        assert_eq!(born, [true, true, true, false]);
        // With the variances every value stays within its range.
        let e = sleep(true);
        for k in 0..200 {
            let elapsed = 0.05 * k as f32;
            for p in particles(&e, 3, elapsed).into_iter().flatten() {
                assert!(p.size >= 0.0 && p.size <= 2.0 * 1.2 + 1e-4);
                assert!((0.0..=1.0).contains(&p.alpha));
                assert!(p.at[1] >= 0.0 && p.at[1] <= 6.0 * 2.75 + 1e-3);
            }
        }
    }

    // GFX-OBJ-8.7 (3), T36: `DemolitionDust` as 8.7 gives it: 50 particles
    // of one round, born within the first 0.1 s (spawn 0.001 +- 0.001 each),
    // +-5 units around the start in x and z, none reborn; each lives 5 +- 3
    // s; colour (0.6, 0.3, 0.0) x 1, alpha 0.3 +- 0.15 x (0.5 + 3 t - 8 t^2
    // + 4.5 t^3): half at birth, back to 0 at the end; size 7 +- 3 x (0.1 +
    // 0.5 t + 2.8 t^2 - 1.4 t^3): a tenth at birth.
    // spec: GFX-OBJ-8.7, GFX-OBJ-8.3a
    #[test]
    fn t36_the_collapse_dust() {
        use s2_assets::pfx::{Curve, Effect};
        let e = Effect {
            version: "0002".into(),
            emitter: "Default".into(),
            max_loops: 1,
            pos: [0.0; 3],
            pos_var: [5.0, 0.0, 5.0],
            vel: [0.0, 0.8, 0.0],
            vel_var: [1.0, 0.5, 1.0],
            size: 7.0,
            size_var: 3.0,
            ttl: 5.0,
            ttl_var: 3.0,
            spawn: 0.001,
            spawn_var: 0.001,
            color: [0.6, 0.3, 0.0, 0.3],
            color_var: [0.0, 0.0, 0.0, 0.15],
            particle: "Decal".into(),
            rect: [0.25, 0.0, 0.5, 0.25],
            count: 50,
            curve_set: "Default".into(),
            curves: [
                Curve::Linear(1.0, 0.4),
                Curve::Cubic([0.1, 1.5, 2.0, 2.0]),
                Curve::Constant(1.0),
                Curve::Cubic([0.5, -3.0, 0.0, 1.0]),
            ],
        };
        assert!((e.curves[1].at(1.0) - 2.0).abs() < 1e-5);
        assert!(e.curves[3].at(1.0).abs() < 1e-5);
        assert_eq!(particles(&e, 9, 0.1).iter().flatten().count(), 50);
        for p in particles(&e, 9, 0.1).into_iter().flatten() {
            assert!(p.at[0].abs() <= 5.0 + 0.1 && p.at[2].abs() <= 5.0 + 0.1);
            assert!(p.at[1] >= 0.0 && p.at[1] < 0.2);
            assert!(p.size >= 0.4 - 1e-4 && p.size <= 1.0 + 0.1);
            assert!(p.alpha >= 0.15 * 0.5 - 1e-4 && p.alpha <= 0.45 * 0.7);
            assert_eq!(p.colour, [0.6, 0.3, 0.0]);
        }
        // Lives of 2 to 8 s, one round: all there at 1.9 s, some gone by 6
        // s, all by 8.2 s.
        assert!(particles(&e, 9, 1.9).iter().flatten().count() == 50);
        let at_6 = particles(&e, 9, 6.0).iter().flatten().count();
        assert!(at_6 > 0 && at_6 < 50, "{at_6}");
        assert!(particles(&e, 9, 8.2).iter().all(Option::is_none));
        assert!(particles(&e, 9, 30.0).iter().all(Option::is_none));
    }

    // GFX-OBJ T17: the Roman guard house on field (11, 11) at raw height
    // 4200: soldier flag, border flag, and the way the banners fly.
    // spec: GFX-OBJ-8.1
    #[test]
    fn guard_house_flags() {
        let house = [[-2.929, 0.0, -3.268], [2.929, 4.0, 3.416]];
        let at = [-231.0, 4.2, -202.6499];
        let soldier = soldier_flag_local(&house);
        assert!(close3(soldier, [1.9331, 0.0, -3.6022]), "{soldier:?}");
        assert!(close3(
            building_to_world(HEADING, at, soldier),
            [-227.525, 4.2, -204.803]
        ));
        let border = border_flag_local(&house);
        assert!(close3(border, [-1.9331, 0.0, -3.6022]));
        assert!(close3(
            building_to_world(HEADING, at, border),
            [-230.873, 4.2, -206.736]
        ));
        // The pole model's +x, turned by 90 and then by the heading.
        let fly = building_to_world(FLAG_TURN + HEADING, [0.0; 3], [1.0, 0.0, 0.0]);
        assert!(close3(fly, [0.5, 0.0, -0.866]), "{fly:?}");
    }

    // GFX-OBJ T18: meshes shown by the soldiers inside.
    // spec: GFX-OBJ-8.1
    #[test]
    fn soldier_flag_mesh_counts() {
        let shown: Vec<usize> = [0, 1, 3, 7]
            .into_iter()
            .map(|n| soldier_flag_meshes(n, false))
            .collect();
        assert_eq!(shown, [1, 2, 4, 6]);
        assert_eq!(soldier_flag_meshes(0, true), 6);
    }

    // GFX-OBJ T19: the headquarters' flag.
    // spec: GFX-OBJ-8.1
    #[test]
    fn headquarters_flag() {
        let house = [[-6.432, 0.0, -1.548], [6.501, 9.0, 8.119]];
        let local = soldier_flag_local(&house);
        assert!(close3(local, [4.3024, 0.0, -2.0314]), "{local:?}");
        let world = building_to_world(HEADING, [-240.0, 5.0, -207.846], local);
        assert!(close3(world, [-235.258, 5.0, -207.454]), "{world:?}");
    }

    // GFX-OBJ T20: the border flag's row by the enemy distance.
    // spec: GFX-OBJ-8.2
    #[test]
    fn border_flag_rows() {
        let rows = [30, 20, 10, -1].map(border_flag_row);
        assert_eq!(rows, [0, 1, 2, 2]);
        assert_eq!([25, 24, 19, 18].map(border_flag_row), [0, 1, 1, 2]);
    }

    // GFX-OBJ T21: a barracks with its coins switched off.
    // spec: GFX-OBJ-8.3
    #[test]
    fn coin_and_soldier_icons() {
        let icon = |move_out, coins| status_icon(StatusOf::Military { move_out, coins });
        assert_eq!(icon(false, false), Some(StatusIcon::NoCoin));
        assert_eq!(icon(false, true), None);
        assert_eq!(icon(true, false), Some(StatusIcon::NoSoldierAndCoin));
        assert_eq!(icon(true, true), Some(StatusIcon::NoSoldier));
        assert_eq!(StatusIcon::NoCoin.effect(), "Sleep_NoCoin");
        assert_eq!(StatusIcon::NoCoin.rect(), [0.1875, 0.3125, 0.25, 0.375]);
    }

    // GFX-OBJ T22: a woodcutter's hut at 20 % productivity shows "Z" after
    // 10 s, not at 9.9 s; switched off it shows the stop gear.
    // spec: GFX-OBJ-8.3 (T22, T28, T29)
    #[test]
    fn sleep_after_ten_seconds() {
        let mut timer = IdleTimer::default();
        let mut idle = false;
        for _ in 0..99 {
            idle = timer.update(0.2, 0.1);
        }
        assert!(!idle && (timer.seconds - 9.9).abs() < 1e-3);
        let icon = |enabled, idle| status_icon(StatusOf::Production { enabled, idle });
        assert_eq!(icon(true, idle), None);
        idle = timer.update(0.2, 0.1);
        assert!(idle);
        assert_eq!(icon(true, idle), Some(StatusIcon::Sleep));
        assert!(!timer.update(0.3, 0.1) && timer.seconds == 0.0);
        assert_eq!(icon(false, false), Some(StatusIcon::ProductionOff));
        // Play test 11 (T28): at 0.25 exactly and before the first product
        // (-1) it idles too; the first product (0.5) ends it.
        for productivity in [0.25, -1.0] {
            let mut timer = IdleTimer::default();
            for _ in 0..99 {
                assert!(!timer.update(productivity, 0.1));
            }
            assert!(timer.update(productivity, 0.1));
            assert!(!timer.update(0.5, 0.1));
        }
    }

    // GFX-OBJ T23: a guard house upgrading to a watch tower at u = 0.45:
    // the plane at base + (H - 0.1) x 0.5, complete in height at 0.9
    // (2.6b's rising plane).
    // spec: GFX-OBJ-8.4
    #[test]
    fn upgrade_cut() {
        let plane = crate::world::scene3d::rising_plane;
        let h = 7.0;
        assert!((plane(2.0, h, 0.45) - (2.0 + (h - 0.1) * 0.5)).abs() < 1e-5);
        assert!((plane(2.0, h, 0.9) - (2.0 + h - 0.1)).abs() < 1e-5);
    }
}
