//! Geometry of the 3D view (`specs/GFX-world-view.md`) as plain functions:
//! the world frame of the fine height grid, the ground height between
//! samples, grid normals, the light, object headings and the camera rig.
//!
//! Everything here is in the game's own frame (GFX-VIEW-1.1): x along a
//! field row, y up, z across the rows toward north, left-handed. The Bevy
//! side uses a right-handed frame, so [`to_bevy`] negates z: a proper
//! rotation of the picture, nothing mirrored on screen.

use s2_sim::map::Terrain;

/// MAP-GRID-5.1: world units per sample step along a row, per sample row
/// and per raw height unit.
pub const SAMPLE_STEP: f32 = 1.5;
pub const SAMPLE_ROW_STEP: f32 = 1.299_037_5;
pub const HEIGHT_SCALE: f32 = 0.001;
/// MAP-GRID-5.4: the water level, raw height 0 (ships, buoys and the
/// cameras' floor keep to it).
pub const WATER_LEVEL: f32 = 0.0;
/// GFX-WATER-3.3 (corrected 2026-11-09, Q-1018): the drawn water surface and
/// the mirror's plane (2.6a) lie at raw -100, world -0.1 (MAP-GRID-5.4's
/// note).
pub const WATER_SURFACE: f32 = -100.0 * HEIGHT_SCALE;

/// A point of the game frame in Bevy's frame: z negated.
pub fn to_bevy([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y, -z]
}

/// A point of Bevy's frame in the game frame.
pub fn from_bevy([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y, -z]
}

/// The fine grid's place in the world (GFX-VIEW-1.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Columns and rows of the fine grid (MAP-GRID-4.1).
    pub columns: u32,
    pub rows: u32,
}

impl Frame {
    /// The frame of a map of `width` x `height` fields.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            columns: 4 * width + 1,
            rows: 4 * height + 1,
        }
    }

    /// GFX-VIEW-1.1: x and z of sample (0, 0); the grid is centred.
    pub fn origin(&self) -> [f32; 2] {
        [
            -SAMPLE_STEP * (self.columns as f32 - 1.0) / 2.0,
            -SAMPLE_ROW_STEP * (self.rows as f32 - 1.0) / 2.0,
        ]
    }

    /// GFX-VIEW-1.1: x and z of sample `(c, r)`.
    pub fn sample_xz(&self, c: i64, r: i64) -> [f32; 2] {
        let h = if r.rem_euclid(2) == 1 { 0.5 } else { 0.0 };
        let [x0, z0] = self.origin();
        [
            SAMPLE_STEP * (c as f32 + h) + x0,
            SAMPLE_ROW_STEP * r as f32 + z0,
        ]
    }

    /// x and z of field `(x, y)`: its own sample (MAP-GRID-4.2).
    pub fn field_xz(&self, x: u32, y: u32) -> [f32; 2] {
        let s = if y % 2 == 1 { 2 } else { 0 };
        self.sample_xz(i64::from(4 * x + s), i64::from(4 * y))
    }

    /// A world point as the map-plane position the 2D view uses
    /// (MAP-GRID-5.2: relative to sample (0, 0)).
    pub fn to_plane(self, x: f32, z: f32) -> [f32; 2] {
        let [x0, z0] = self.origin();
        [x - x0, z - z0]
    }

    pub fn contains_sample(&self, c: i64, r: i64) -> bool {
        (0..i64::from(self.columns)).contains(&c) && (0..i64::from(self.rows)).contains(&r)
    }

    /// The sample nearest to a world point, ignoring the grid's bounds.
    pub fn nearest_sample(&self, x: f32, z: f32) -> (i64, i64) {
        let [x0, z0] = self.origin();
        let row = ((z - z0) / SAMPLE_ROW_STEP).round() as i64;
        let mut best = (f32::MAX, (0, row));
        for r in row - 1..=row + 1 {
            let h = if r.rem_euclid(2) == 1 { 0.5 } else { 0.0 };
            let c = ((x - x0) / SAMPLE_STEP - h).round() as i64;
            for c in c - 1..=c + 1 {
                let [sx, sz] = self.sample_xz(c, r);
                let d = (sx - x).powi(2) + (sz - z).powi(2);
                if d < best.0 {
                    best = (d, (c, r));
                }
            }
        }
        best.1
    }
}

/// MAP-GRID-1.2: the six neighbour offsets of a sample, directions 0 to 5.
pub fn neighbour_offsets(row: i64) -> [(i64, i64); 6] {
    if row.rem_euclid(2) == 0 {
        [(1, 0), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1)]
    } else {
        [(1, 0), (1, -1), (0, -1), (-1, 0), (0, 1), (1, 1)]
    }
}

/// Raw heights of the fine grid with the frame they lie in.
pub trait Heights {
    fn frame(&self) -> Frame;
    /// The raw height of a sample inside the grid.
    fn raw(&self, c: u32, r: u32) -> i32;

    /// World height of a sample, clamped into the grid.
    fn world(&self, c: i64, r: i64) -> f32 {
        let f = self.frame();
        let c = c.clamp(0, i64::from(f.columns) - 1) as u32;
        let r = r.clamp(0, i64::from(f.rows) - 1) as u32;
        self.raw(c, r) as f32 * HEIGHT_SCALE
    }
}

impl Heights for Terrain {
    fn frame(&self) -> Frame {
        Frame::new(self.width, self.height)
    }

    fn raw(&self, c: u32, r: u32) -> i32 {
        self.sample_height((c, r))
    }
}

/// GFX-VIEW-1.4: the ground height at a world point: the plane of the
/// lattice triangle around the nearest sample that contains the point.
/// Away from the grid the nearest sample's height.
pub fn ground_height(heights: &impl Heights, x: f32, z: f32) -> f32 {
    let frame = heights.frame();
    let (c, r) = frame.nearest_sample(x, z);
    let (c, r) = (
        c.clamp(0, i64::from(frame.columns) - 1),
        r.clamp(0, i64::from(frame.rows) - 1),
    );
    let p = frame.sample_xz(c, r);
    let hp = heights.world(c, r);
    let offsets = neighbour_offsets(r);
    let cross = |a: [f32; 2], b: [f32; 2], q: [f32; 2]| {
        (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
    };
    for i in 0..6 {
        let (ac, ar) = (c + offsets[i].0, r + offsets[i].1);
        let (bc, br) = (c + offsets[(i + 1) % 6].0, r + offsets[(i + 1) % 6].1);
        if !frame.contains_sample(ac, ar) || !frame.contains_sample(bc, br) {
            continue;
        }
        let (a, b) = (frame.sample_xz(ac, ar), frame.sample_xz(bc, br));
        // The two sign tests: the point lies between the edges P-A and P-B.
        let (s1, s2) = (cross(p, a, [x, z]), cross(p, b, [x, z]));
        let turn = cross(p, a, b);
        if s1 * turn < -1e-6 || s2 * turn > 1e-6 {
            continue;
        }
        let (ha, hb) = (heights.world(ac, ar), heights.world(bc, br));
        return plane_height([p[0], hp, p[1]], [a[0], ha, a[1]], [b[0], hb, b[1]], x, z);
    }
    hp
}

/// GFX-VIEW-1.4, GFX-OBJ-6.2c: the unit normal (game frame, facing up) of
/// the lattice triangle [`ground_height`] takes at a world point; straight
/// up away from the grid.
pub fn ground_normal(heights: &impl Heights, x: f32, z: f32) -> [f32; 3] {
    let frame = heights.frame();
    let (c, r) = frame.nearest_sample(x, z);
    let (c, r) = (
        c.clamp(0, i64::from(frame.columns) - 1),
        r.clamp(0, i64::from(frame.rows) - 1),
    );
    let p = frame.sample_xz(c, r);
    let hp = heights.world(c, r);
    let offsets = neighbour_offsets(r);
    let cross = |a: [f32; 2], b: [f32; 2], q: [f32; 2]| {
        (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
    };
    for i in 0..6 {
        let (ac, ar) = (c + offsets[i].0, r + offsets[i].1);
        let (bc, br) = (c + offsets[(i + 1) % 6].0, r + offsets[(i + 1) % 6].1);
        if !frame.contains_sample(ac, ar) || !frame.contains_sample(bc, br) {
            continue;
        }
        let (a, b) = (frame.sample_xz(ac, ar), frame.sample_xz(bc, br));
        let (s1, s2) = (cross(p, a, [x, z]), cross(p, b, [x, z]));
        let turn = cross(p, a, b);
        if s1 * turn < -1e-6 || s2 * turn > 1e-6 {
            continue;
        }
        let (ha, hb) = (heights.world(ac, ar), heights.world(bc, br));
        let u = [a[0] - p[0], ha - hp, a[1] - p[1]];
        let v = [b[0] - p[0], hb - hp, b[1] - p[1]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let n = if n[1] < 0.0 { n.map(|k| -k) } else { n };
        return normalize(n);
    }
    [0.0, 1.0, 0.0]
}

/// Height of the plane through three points at `(x, z)`.
fn plane_height(p: [f32; 3], a: [f32; 3], b: [f32; 3], x: f32, z: f32) -> f32 {
    let (ux, uy, uz) = (a[0] - p[0], a[1] - p[1], a[2] - p[2]);
    let (vx, vy, vz) = (b[0] - p[0], b[1] - p[1], b[2] - p[2]);
    // Normal = u x v; the plane is n . (q - p) = 0.
    let (nx, ny, nz) = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx);
    if ny.abs() < 1e-9 {
        return p[1];
    }
    p[1] - (nx * (x - p[0]) + nz * (z - p[2])) / ny
}

/// GFX-VIEW-2.2: the grid normal of a sample, from the heights of its row
/// and column neighbours by grid index (clamped at the edge), in world
/// units.
pub fn grid_normal(heights: &impl Heights, c: i64, r: i64) -> [f32; 3] {
    let (sx, sz) = (SAMPLE_STEP, SAMPLE_ROW_STEP);
    let dx = heights.world(c + 1, r) - heights.world(c - 1, r);
    let dz = heights.world(c, r + 1) - heights.world(c, r - 1);
    normalize([-dx * 2.0 * sz, 2.0 * sx * 2.0 * sz, -dz * 2.0 * sx])
}

pub fn normalize([x, y, z]: [f32; 3]) -> [f32; 3] {
    let length = (x * x + y * y + z * z).sqrt();
    if length == 0.0 {
        [0.0, 1.0, 0.0]
    } else {
        [x / length, y / length, z / length]
    }
}

/// GFX-VIEW-2.3: the shipped `daytimes.xml` has every entry the same, so
/// the light is constant: elevation 50 and heading 55 degrees, ambient,
/// light colour and the background colour.
pub const LIGHT_ELEVATION: f32 = 50.0;
pub const LIGHT_HEADING: f32 = 55.0;
pub const AMBIENT: [f32; 3] = [0.6, 0.5, 0.6];
pub const LIGHT_COLOR: [f32; 3] = [0.9375, 0.898, 0.781];
pub const BACKGROUND: [f32; 3] = [0.8, 0.9, 1.0];

/// GFX-VIEW-2.3 (Run A): `shadowIntensity` of the shipped `daytimes.xml`.
pub const SHADOW_INTENSITY: f32 = 0.9;

/// GFX-VIEW-2.3c: the environment clock starts at this hour in the
/// world's first tick.
pub const START_HOUR: f32 = 12.0;

/// GFX-VIEW-2.3c: hours the environment clock advances per second of game
/// time (one table day in about 727 s).
pub const DAY_RATE: f32 = 0.033;

/// GFX-VIEW-2.3c: the environment clock after a frame of `step` game
/// seconds: while the `DayNightCycle` option is on it advances by step x
/// 0.033 hours (float32, not wrapped); off, it is set back to the start
/// hour `start` (12 in a game).
pub fn advance_clock(hour: f32, step: f32, on: bool, start: f32) -> f32 {
    if on {
        hour + step * DAY_RATE
    } else {
        start
    }
}

/// GFX-VIEW-2.3, 2.3b: the light of one hour of the day table in use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayLight {
    /// Degrees.
    pub elevation: f32,
    pub heading: f32,
    pub ambient: [f32; 3],
    pub color: [f32; 3],
    pub background: [f32; 3],
    pub shadow_intensity: f32,
    /// GFX-VIEW-6.3, 6.3a: the skydome entry the cap carries: 0 the
    /// clouds, 1 the stars.
    pub skydome: i32,
    /// GFX-VIEW-6.3 (T44): the next entry's skydome, which the cap mixes in
    /// by [`DayLight::blend`] (the same as `skydome` mixes nothing in).
    pub next_skydome: i32,
    /// GFX-VIEW-2.3c, 6.3: the day-time blend between the two entries.
    pub blend: f32,
}

impl DayLight {
    /// GFX-VIEW-2.3: the shipped `daytimes.xml`, read by the game only when
    /// no binary table exists.
    pub const XML: DayLight = DayLight {
        elevation: LIGHT_ELEVATION,
        heading: LIGHT_HEADING,
        ambient: AMBIENT,
        color: LIGHT_COLOR,
        background: BACKGROUND,
        shadow_intensity: SHADOW_INTENSITY,
        skydome: 0,
        next_skydome: 0,
        blend: 0.0,
    };

    /// GFX-VIEW-2.3: the light of `table` at `hour`: the two entries around
    /// it interpolated linearly (across midnight from the last to the
    /// first), the heading the shorter way; the cap shows the first one's
    /// skydome. None for an empty table.
    pub fn at(table: &[s2_assets::daytimes::DayEntry], hour: f32) -> Option<DayLight> {
        let mut entries: Vec<_> = table.to_vec();
        entries.sort_by_key(|e| e.hour);
        let last = *entries.last()?;
        let (a, b) = match entries.iter().rposition(|e| e.hour as f32 <= hour) {
            Some(i) => (entries[i], *entries.get(i + 1).unwrap_or(&entries[0])),
            None => (last, entries[0]),
        };
        let span = (b.hour - a.hour).rem_euclid(24) as f32;
        let t = if span == 0.0 {
            0.0
        } else {
            ((hour - a.hour as f32).rem_euclid(24.0) / span).clamp(0.0, 1.0)
        };
        let mix = |x: f32, y: f32| x + (y - x) * t;
        let rgb = |x: [f32; 4], y: [f32; 4]| [mix(x[0], y[0]), mix(x[1], y[1]), mix(x[2], y[2])];
        let turn = ((b.heading - a.heading) as f32 + 540.0).rem_euclid(360.0) - 180.0;
        Some(DayLight {
            elevation: mix(a.elevation as f32, b.elevation as f32),
            heading: a.heading as f32 + turn * t,
            ambient: rgb(a.ambient, b.ambient),
            color: rgb(a.light, b.light),
            background: rgb(a.background, b.background),
            shadow_intensity: mix(a.shadow_intensity, b.shadow_intensity),
            skydome: a.skydome,
            next_skydome: b.skydome,
            blend: t,
        })
    }

    /// GFX-VIEW-6.3 (T44): the share of the next entry's sky texture in the
    /// cap: the blend while the next entry names another skydome, else 0
    /// (the cap shows its own texture alone).
    pub fn sky_mix(&self) -> f32 {
        if self.skydome == self.next_skydome {
            0.0
        } else {
            self.blend
        }
    }

    /// GFX-VIEW-2.3c: the light of `table` at the environment clock
    /// `clock`, read at the clock mod 24.
    pub fn at_clock(table: &[s2_assets::daytimes::DayEntry], clock: f32) -> Option<DayLight> {
        Self::at(table, clock.rem_euclid(24.0))
    }

    /// Whether the light differs from `other` enough to be drawn anew: a
    /// tenth of a degree, a 500th of a colour or the skydome entry.
    pub fn differs(&self, other: &DayLight) -> bool {
        let near = |a: f32, b: f32, e: f32| (a - b).abs() <= e;
        let rgb = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| near(a[i], b[i], 0.002));
        self.skydome != other.skydome
            || self.next_skydome != other.next_skydome
            || (self.skydome != self.next_skydome && !near(self.sky_mix(), other.sky_mix(), 0.002))
            || !near(self.elevation, other.elevation, 0.1)
            || !near(self.heading, other.heading, 0.1)
            || !near(self.shadow_intensity, other.shadow_intensity, 0.002)
            || !rgb(self.ambient, other.ambient)
            || !rgb(self.color, other.color)
            || !rgb(self.background, other.background)
    }

    /// The way the light travels (GFX-VIEW-2.3, 2.3d b): from the
    /// elevation's size, so a negative one (night) still lights from above.
    pub fn direction(&self) -> [f32; 3] {
        light_direction(self.elevation.abs(), self.heading)
    }

    /// GFX-VIEW-6.3: the moon shows while the elevation is at most 0.
    pub fn moon_shown(&self) -> bool {
        moon_shown(self.elevation)
    }

    /// GFX-VIEW-2.3, 2.4: the light factor D of a pixel (as [`light_d`]).
    pub fn light_d(&self, n_dot_l: f32, shadow: f32) -> [f32; 3] {
        let lit = n_dot_l.clamp(0.0, 1.0) * shadow;
        [0, 1, 2].map(|i| self.ambient[i] + self.color[i] * lit)
    }

    /// GFX-OBJ-2.4 (GFX-VIEW-2.3d f): a tree's light: the mesh's vertex
    /// colour `c` scales the direct term only, D = ambient + saturate(N .
    /// L) x light x c x s (the object shader's tree branch).
    #[cfg(test)]
    pub fn tree_d(&self, n_dot_l: f32, c: [f32; 3], shadow: f32) -> [f32; 3] {
        let lit = n_dot_l.clamp(0.0, 1.0) * shadow;
        [0, 1, 2].map(|i| self.ambient[i] + self.color[i] * c[i] * lit)
    }

    /// The light as the shaders take it.
    pub fn shader(&self) -> ShaderLight {
        let [x, y, z] = to_bevy(self.direction());
        let [r, g, b] = self.color;
        let [ar, ag, ab] = self.ambient;
        ShaderLight {
            direction: [
                x,
                y,
                z,
                shadow_factor(self.elevation, self.shadow_intensity),
            ],
            color: [r, g, b, luminance(self.color)],
            ambient: [ar, ag, ab, luminance(self.ambient)],
        }
    }
}

impl Default for DayLight {
    fn default() -> Self {
        Self::XML
    }
}

/// GFX-VIEW-2.3: the factor a shadowed pixel keeps of the direct light:
/// 1 - k x intensity, k fading in as the sun climbs from 20 to 40 degrees,
/// clamped to 0..1. The shipped table gives 0.1.
pub fn shadow_factor(elevation_deg: f32, intensity: f32) -> f32 {
    let k = ((elevation_deg.abs() - 20.0) / 20.0).clamp(0.0, 1.0);
    (1.0 - k * intensity).clamp(0.0, 1.0)
}

/// GFX-VIEW-2.4a: the pattern render type that draws a highlight.
pub const SPECULAR_TYPE: u32 = 3;

/// GFX-VIEW-2.4a: a type-3 pattern's highlight, added to every channel:
/// a_n x I x saturate(R . V) ^ power with R = normalize(2 x D x N - L) taken
/// per channel as written, all in tangent space: `d` the light factor of
/// 2.4 (ambient included, shadow applied), `n` the normal-map normal, `l`
/// toward the light, `v` toward the eye, `a_n` the normal-map texel's alpha
/// (the terrain shader's `shade`).
#[cfg(test)]
pub fn specular_add(
    d: [f32; 3],
    n: [f32; 3],
    l: [f32; 3],
    v: [f32; 3],
    a_n: f32,
    [intensity, power]: [f32; 2],
) -> f32 {
    let r: [f32; 3] = std::array::from_fn(|i| 2.0 * d[i] * n[i] - l[i]);
    let len = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-6);
    let r_dot_v = (r[0] * v[0] + r[1] * v[1] + r[2] * v[2]) / len;
    a_n * intensity * r_dot_v.clamp(0.0, 1.0).powf(power)
}

/// GFX-VIEW-2.3: luminance of a colour (0.299, 0.587, 0.114), carried as
/// the fourth value of the ambient and the light colour.
pub fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// GFX-VIEW-2.3, 2.4: the light factor D of a terrain or object pixel:
/// ambient + light x N.L x shadow, not scaled down above 1 (the product
/// with the texel is clamped only at the output). The shaders do this per
/// pixel (and `colour_probe` checks them).
#[cfg(test)]
pub fn light_d(n_dot_l: f32, shadow: f32) -> [f32; 3] {
    let lit = n_dot_l.clamp(0.0, 1.0) * shadow;
    [0, 1, 2].map(|i| AMBIENT[i] + LIGHT_COLOR[i] * lit)
}

/// GFX-VIEW-6.3: the sun and moon quads of `graphics.xml`: their sizes,
/// their distance from the camera, and their textures.
pub const SUN_SIZE: f32 = 30.0;
pub const MOON_SIZE: f32 = 9.0;
pub const SKY_SPRITE_DISTANCE: f32 = 100.0;
pub const SUN_FILE: &str = "data/enviroment/sun.tga";
pub const MOON_FILE: &str = "moon.dds";
/// The quads are drawn first with the depth test off, so everything drawn
/// after covers them. Moved this many times farther away at this many
/// times their size they look the same and lie behind the scene (the far
/// plane is at 500).
pub const SKY_SPRITE_PUSH: f32 = 4.5;

/// GFX-VIEW-6.3: the centre (game frame) and side of a sun or moon quad
/// of `size` for a camera at `camera`: at the light direction (the way the
/// light travels, GFX-VIEW-2.3: (0, 0, 1) turned by the heading and the
/// absolute elevation, so the sun lies below the horizon and is hidden;
/// Q-229 d) x 100 from the camera, pushed back by [`SKY_SPRITE_PUSH`].
/// Whether `size` is the side or half of it is not given; the side
/// (Q-256 f).
pub fn sky_sprite(camera: [f32; 3], light: [f32; 3], size: f32) -> ([f32; 3], f32) {
    let len = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2])
        .sqrt()
        .max(1e-6);
    let d = SKY_SPRITE_DISTANCE * SKY_SPRITE_PUSH / len;
    (
        [
            camera[0] + light[0] * d,
            camera[1] + light[1] * d,
            camera[2] + light[2] * d,
        ],
        size * SKY_SPRITE_PUSH,
    )
}

/// GFX-VIEW-6.3: the moon is drawn while the light's elevation is at most
/// 0 (GFX-VIEW-6.3a: at night in the tables used); the sun always.
pub fn moon_shown(elevation_deg: f32) -> bool {
    elevation_deg <= 0.0
}

/// GFX-VIEW-6.3: the vector a sun or moon quad is placed along: (0, 0, 1)
/// turned by the heading and the absolute elevation (the sun below the
/// horizon by day), the moon's with the height mirrored.
pub fn sky_sprite_direction(light: &DayLight, moon: bool) -> [f32; 3] {
    let [x, y, z] = light_direction(light.elevation.abs(), light.heading);
    [x, if moon { -y } else { y }, z]
}

/// GFX-WATER-2.6: the mirror texture's size for the mirror quality
/// `quality` (the 14th scene field, 0..1) on a back buffer `width` pixels
/// wide: 128 x 2^q texels wide with q = trunc(4 x quality) limited to
/// 0..4, halved until it is no wider than the back buffer, and half as
/// high; none at q = 0.
pub fn mirror_texture_size(quality: f32, width: u32) -> Option<(u32, u32)> {
    // A NaN gives 0 too.
    let level = (4.0 * quality).trunc().clamp(0.0, 4.0) as u32;
    if level == 0 {
        return None;
    }
    let mut w = 128u32 << level;
    while w > width.max(2) {
        w /= 2;
    }
    Some((w, w / 2))
}

/// GFX-WATER-2.6a 2: the mirror view's field of view is 1.03 times the main
/// camera's horizontal one, the aspect the main one's: the vertical field of
/// view (radians) for the main one's `main_vertical` at `aspect`.
pub fn mirror_vertical_fov(main_vertical: f32, aspect: f32) -> f32 {
    let aspect = aspect.max(0.01);
    let horizontal = 2.0 * ((main_vertical / 2.0).tan() * aspect).atan();
    2.0 * ((MIRROR_FOV_FACTOR * horizontal / 2.0).tan() / aspect).atan()
}

/// GFX-WATER-2.6a 2: the mirror view's field of view over the main one's.
pub const MIRROR_FOV_FACTOR: f32 = 1.03;

/// GFX-WATER-2.6a 1: a point reflected in the water plane, y -> 2 level - y
/// with the surface's level -0.1 (3.3; the same in Bevy's frame as in the
/// game's).
pub fn reflect_in_water(p: [f32; 3]) -> [f32; 3] {
    [p[0], 2.0 * WATER_SURFACE - p[1], p[2]]
}

/// GFX-WATER-2.6b: what the mirror pass draws besides the sky (the cloud
/// cap over the background, 2.6c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorContent {
    pub land: bool,
    /// The settlers with what they carry (the scene's draw bit 8).
    pub settlers: bool,
    pub objects: bool,
}

/// GFX-WATER-2.6b: the mirror's content for the main camera's forward
/// vector's y `forward_y` and the `MIRROR DETAILS` setting `details`: the
/// sky alone when the camera looks down more steeply than 30 degrees
/// (forward y below -0.5); else by n = round(4 details): n = 0 the sky
/// alone, n >= 1 sky and land, n >= 3 also the settlers with what they
/// carry (the main view's settlers of that frame), n = 4 everything but the
/// water.
pub fn mirror_content(forward_y: f32, details: f32) -> MirrorContent {
    let n = (4.0 * details).round().clamp(0.0, 4.0) as u32;
    let flat = forward_y >= -0.5;
    MirrorContent {
        land: flat && n >= 1,
        settlers: flat && n >= 3,
        objects: flat && n == 4,
    }
}

/// GFX-WATER-2.5: the caustics strip `map/textures/caustics.tga`, 16 frames
/// of 128 x 128 side by side, looping every 5 s. The functions below are
/// the terrain shader's `caustics_at`, for its tests.
pub const CAUSTICS_FILE: &str = "data/map/textures/caustics.tga";
#[cfg(test)]
pub const CAUSTICS_FRAMES: f32 = 16.0;

/// GFX-WATER-2.5: at time `t` (seconds) the frame f = floor(frac(0.2 t) x
/// 16), the frame before it, and the weight w of f in their blend:
/// r(f) w + r(f - 1) (1 - w), w the fraction of 16 frac(0.2 t).
#[cfg(test)]
pub fn caustics_frames(t: f32) -> (u32, u32, f32) {
    let phase = (0.2 * t).rem_euclid(1.0) * CAUSTICS_FRAMES;
    let f = (phase.floor() as u32).min(15);
    (f, (f + 15) % 16, phase - phase.floor())
}

/// GFX-WATER-2.5: the strip coordinates of frame `frame` at the ground's
/// texture coordinate, its position (x, z) (GFX-PAT-3.1): (x / 16, z) x
/// 0.04, the frame's slot added to u literally; the strip wraps as a whole.
#[cfg(test)]
pub fn caustics_uv(x: f32, z: f32, frame: u32) -> [f32; 2] {
    [
        (x / 16.0 * 0.04 + frame as f32 / CAUSTICS_FRAMES).rem_euclid(1.0),
        (z * 0.04).rem_euclid(1.0),
    ]
}

/// GFX-WATER-2.5: what the red channel of the blended sample is multiplied
/// by, besides the light colour: 0.7 x V x V x (1 - min(1, -0.15 y)) with
/// V = 1 - sin(0.4 x + s) sin(0.4 y + s) sin(0.4 z + s) and s = sin(t) (V
/// enters twice); 0 at or above y = -0.1.
#[cfg(test)]
pub fn caustics_scale(x: f32, y: f32, z: f32, t: f32) -> f32 {
    if y >= -0.1 {
        return 0.0;
    }
    let s = t.sin();
    let wave = 1.0 - (0.4 * x + s).sin() * (0.4 * y + s).sin() * (0.4 * z + s).sin();
    0.7 * wave * wave * (1.0 - (-0.15 * y).min(1.0))
}

#[cfg(test)]
/// GFX-OBJ-3.3 (Run A): a settler's scalar D: N.L x lum(light) x shadow +
/// lum(ambient).
pub fn settler_d(n_dot_l: f32, shadow: f32) -> f32 {
    n_dot_l.clamp(0.0, 1.0) * luminance(LIGHT_COLOR) * shadow + luminance(AMBIENT)
}

#[cfg(test)]
/// GFX-OBJ-3.3 (T41): a settler's highlight: R = normalize(2 D N + L) (N
/// the unit normal, L the light's way down, D the scalar of
/// [`settler_d`]), S = saturate(R . V)^16 with V toward the camera; the
/// object shader's settler pixel does the same.
pub fn settler_highlight(n: [f32; 3], l: [f32; 3], v: [f32; 3], d: f32) -> ([f32; 3], f32) {
    let r = normalize([0, 1, 2].map(|k| 2.0 * d * n[k] + l[k]));
    let s = (r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
        .clamp(0.0, 1.0)
        .powi(16);
    (r, s)
}

/// The light as the shaders take it (GFX-VIEW-2.3): the direction it
/// travels (Bevy frame) with the shadow factor as its fourth value, the
/// light colour and the ambient with their luminances as theirs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaderLight {
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub ambient: [f32; 4],
}

/// GFX-VIEW-2.3: the light of the shipped `daytimes.xml` for the shaders.
#[cfg(test)]
pub fn shader_light() -> ShaderLight {
    DayLight::XML.shader()
}

#[cfg(test)]
/// GFX-VIEW-4.3a (Q-083): every building, site, decal and military flag
/// is turned by the loader's default heading, -30 degrees, so its door
/// (model -z) faces its flag (direction 1).
pub const BUILDING_HEADING: f32 = s2_assets::model_tables::DEFAULT_BUILDING_HEADING as f32;

/// GFX-VIEW-4.3c: where a building stands: its field's world position at
/// the height of the field's own sample (not the footprint's).
pub fn building_position(heights: &impl Heights, x: u32, y: u32) -> [f32; 3] {
    let frame = heights.frame();
    let s = if y % 2 == 1 { 2 } else { 0 };
    let (c, r) = (i64::from(4 * x + s), i64::from(4 * y));
    let [px, pz] = frame.sample_xz(c, r);
    [px, heights.world(c, r), pz]
}

#[cfg(test)]
/// GFX-VIEW-4.3b: a building's world matrix R x T in the row-vector
/// convention (rows: images of +x, +y, +z, then the translation).
pub fn building_matrix(at: [f32; 3]) -> [[f32; 4]; 4] {
    let a = BUILDING_HEADING.to_radians();
    let (s, c) = a.sin_cos();
    [
        [c, 0.0, -s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [s, 0.0, c, 0.0],
        [at[0], at[1], at[2], 1.0],
    ]
}

#[cfg(test)]
/// A point times a row-vector matrix.
pub fn transform_row(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|j| p[0] * m[0][j] + p[1] * m[1][j] + p[2] * m[2][j] + m[3][j])
}

/// GFX-VIEW-2.3: the direction the light travels, from the sun down.
pub fn light_direction(elevation_deg: f32, heading_deg: f32) -> [f32; 3] {
    let (e, h) = (elevation_deg.to_radians(), heading_deg.to_radians());
    [h.sin() * e.cos(), -e.sin(), h.cos() * e.cos()]
}

/// GFX-VIEW-4.1: the rotation about +y of an object facing direction `d`
/// (MAP-GRID-1.2), in radians.
pub fn heading(direction: u8) -> f32 {
    const DEGREES: [f32; 6] = [90.0, 150.0, -150.0, -90.0, -30.0, 30.0];
    DEGREES[usize::from(direction % 6)].to_radians()
}

/// GFX-VIEW-4.1: a heading angle in the game frame as a rotation about
/// Bevy's +y: the z flip of [`to_bevy`] turns the sense around.
pub fn bevy_yaw(heading: f32) -> f32 {
    -heading
}

/// The direction of the step from `from` to `to`, two neighbouring fields;
/// `None` when they are not neighbours.
pub fn step_direction(from: (u32, u32), to: (u32, u32)) -> Option<u8> {
    let offsets = if from.1.is_multiple_of(2) {
        [(1, 0), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1)]
    } else {
        [(1, 0), (1, -1), (0, -1), (-1, 0), (0, 1), (1, 1)]
    };
    let d = (
        i64::from(to.0) - i64::from(from.0),
        i64::from(to.1) - i64::from(from.1),
    );
    offsets.iter().position(|&o| o == d).map(|i| i as u8)
}

/// GFX-VIEW-3.1, 3.5: the horizontal field of view in degrees, the near
/// and far planes, and the zoom steps.
pub const FIELD_OF_VIEW: f32 = 50.0;
pub const NEAR: f32 = 1.0;
pub const FAR: f32 = 500.0;
pub const MAX_ZOOM: u8 = 4;
/// GFX-VIEW-3.1: the three yaw targets.
pub const YAW_LEFT: f32 = 1.5;
pub const YAW_RIGHT: f32 = -1.5;
/// GFX-VIEW-3.3: smoothing per frame of the yaw; the share of the old
/// camera z kept each frame (corrected, play test 10: z eases, x and y do
/// not).
const YAW_EASE: f32 = 0.245;
const Z_KEEP: f32 = 0.3;
/// GFX-VIEW-3.3, 3.6 (play test 10): a jump to a field puts the target
/// this far in front of it (game z), which brings the field to the middle
/// of the screen.
pub const JUMP_Z_OFFSET: f32 = 18.0;
/// GFX-VIEW-3.6 a: the target of a game without headquarters.
pub const NO_HQ_TARGET: [f32; 3] = [0.0, 10.0, -10.0];
/// GFX-VIEW-3.3: world units of camera distance per zoom step: 0.3 x 70.
pub const DISTANCE_PER_ZOOM: f32 = 0.3 * 70.0;
/// GFX-VIEW-3.1: a game starts at this zoom step.
pub const START_ZOOM: u8 = 3;
/// GFX-VIEW-3.3: the pitch component at zoom 0, 17.46 degrees down (Q-205 a
/// answered: the normal camera's zoom 0 is -0.3; the sky in the photo of
/// OBS-1 comes from another camera mode).
pub const ZOOM0_PITCH: f32 = -0.3;
/// GFX-VIEW-3.6 e: the share of the way to the wanted eye a scripted move
/// goes each frame, and how near the wanted eye it ends.
pub const SCRIPTED_EASE: f32 = 0.125;
pub const SCRIPTED_END: f32 = 0.1;
/// GFX-VIEW-3.3: the view direction eases toward the rig's direction by
/// these shares a frame, the first when the zoom changed in it.
const DIRECTION_EASE_ZOOMED: f32 = 0.9;
const DIRECTION_EASE: f32 = 0.4;

/// GFX-VIEW-3.5: the vertical field of view (radians) of the horizontal 50
/// degrees on a screen of `aspect` (width / height).
pub fn vertical_fov(aspect: f32) -> f32 {
    2.0 * ((FIELD_OF_VIEW / 2.0).to_radians().tan() / aspect.max(0.01)).atan()
}

/// The camera of GFX-VIEW-3: a target on the ground, a zoom step and a
/// yaw that eases toward one of three targets.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraRig {
    /// The point looked at, in the game frame; y is set by [`Self::update`].
    pub target: [f32; 3],
    pub zoom: u8,
    pub yaw: f32,
    pub yaw_target: f32,
    /// GFX-VIEW-3.3: the base horizontal view direction (x, z). A jump to a
    /// field sets the direction (0, -1, 1) / sqrt 2, so it is north.
    pub base: [f32; 2],
    /// The smoothed ground value and the camera's z, once set.
    ground: Option<f32>,
    eye_z: Option<f32>,
    /// The smoothed view direction and the zoom of the last update.
    view: Option<[f32; 3]>,
    last_zoom: Option<u8>,
    /// The camera's position of the last update.
    last_eye: Option<[f32; 3]>,
    /// GFX-VIEW-3.2a: the right-button look while it lasts.
    look: Option<Eye>,
    /// GFX-VIEW-3.6 e: a scripted move (camera mode 5) while it runs.
    scripted: bool,
}

/// Where the camera is and where it looks, in the game frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eye {
    pub position: [f32; 3],
    pub direction: [f32; 3],
}

impl CameraRig {
    /// GFX-VIEW-3.1: start zoom 2, yaw 0, looking at `target` (x, z).
    pub fn new(x: f32, z: f32) -> Self {
        Self {
            target: [x, 0.0, z],
            zoom: 2,
            yaw: 0.0,
            yaw_target: 0.0,
            base: [0.0, 1.0],
            ground: None,
            eye_z: None,
            view: None,
            last_zoom: None,
            last_eye: None,
            look: None,
            scripted: false,
        }
    }

    /// GFX-VIEW-3.6 a: the camera of a game start: zoom 3, yaw 0, then a
    /// jump to the local player's headquarters (its field point, x and z)
    /// when there is one, else the target (0, 10, -10).
    pub fn start(headquarters: Option<[f32; 2]>) -> Self {
        let mut rig = Self::new(NO_HQ_TARGET[0], NO_HQ_TARGET[2]);
        rig.target[1] = NO_HQ_TARGET[1];
        rig.zoom = START_ZOOM;
        if let Some([x, z]) = headquarters {
            rig.jump_to(x, z);
        }
        rig
    }

    /// GFX-VIEW-3.3 (corrected, play test 10): a jump to the field at
    /// (x, z): the target at x and z - 18 (its y follows the floor), yaw
    /// and yaw target 0, the base direction north. The zoom is the
    /// caller's (a game start and H set 3, a scripted move 2).
    pub fn jump_to(&mut self, x: f32, z: f32) {
        self.target[0] = x;
        self.target[2] = z - JUMP_Z_OFFSET;
        self.yaw = 0.0;
        self.yaw_target = 0.0;
        self.base = [0.0, 1.0];
    }

    /// GFX-VIEW-3.6 e: a scripted move to the field at (x, z) (the tutorial,
    /// CAM-5.3b): the target and turn of the jump of 3.6 d at zoom 3; the
    /// eye then goes 12.5 % of the way to where that jump puts it each
    /// frame, the view direction kept, until it is within 0.1 of it. The
    /// player's camera input is not read meanwhile ([`Self::moving`]).
    pub fn move_to(&mut self, x: f32, z: f32) {
        self.jump_to(x, z);
        self.zoom = START_ZOOM;
        self.look = None;
        self.scripted = true;
    }

    /// GFX-VIEW-3.6 e: whether a scripted move runs.
    pub fn moving(&self) -> bool {
        self.scripted
    }

    /// GFX-VIEW-3.2: rotate-left and rotate-right.
    pub fn rotate_left(&mut self) {
        self.yaw_target = if self.yaw_target == YAW_RIGHT {
            0.0
        } else {
            YAW_LEFT
        };
    }

    pub fn rotate_right(&mut self) {
        self.yaw_target = if self.yaw_target == YAW_LEFT {
            0.0
        } else {
            YAW_RIGHT
        };
    }

    /// GFX-VIEW-3.2: zoom in and out by one step within 0 to 4.
    pub fn zoom_in(&mut self) {
        self.zoom = self.zoom.saturating_sub(1);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom + 1).min(MAX_ZOOM);
    }

    /// GFX-VIEW-3.3: pitch by zoom: the truncated zoom picks -0.3 / -0.6 /
    /// -0.8 in the normal camera (Q-205 a answered).
    pub fn pitch(&self) -> f32 {
        match self.zoom {
            0 => ZOOM0_PITCH,
            1 => -0.6,
            _ => -0.8,
        }
    }

    /// GFX-VIEW-3.3: the view direction for the current yaw: the base
    /// direction is renormalised as a whole every frame, so at its fixed
    /// point the direction is a unit vector whose y is the pitch component
    /// p and whose horizontal part is sqrt(1 - p^2) long (asin |p| below
    /// the horizontal: 17.46, 36.87, 53.13 degrees).
    pub fn direction(&self) -> [f32; 3] {
        let [bx, bz] = self.base;
        let base = bx.hypot(bz).max(1e-6);
        let (bx, bz) = (bx / base, bz / base);
        let (s, c) = self.yaw.sin_cos();
        let p = self.pitch();
        let h = (1.0 - p * p).sqrt();
        [h * (c * bx - s * bz), p, h * (c * bz + s * bx)]
    }

    /// GFX-VIEW-3.2: the horizontal view direction and its right side.
    pub fn axes(&self) -> ([f32; 2], [f32; 2]) {
        let [x, _, z] = self.direction();
        let length = x.hypot(z).max(1e-6);
        let forward = [x / length, z / length];
        (forward, [forward[1], -forward[0]])
    }

    /// GFX-VIEW-3.2: move the target by `(right, forward)` world units along
    /// the view axes; the move is taken only when the new target lies over
    /// a sample inside the map.
    pub fn pan(&mut self, frame: &Frame, right: f32, forward: f32) {
        let ([fx, fz], [rx, rz]) = self.axes();
        let x = self.target[0] + fx * forward + rx * right;
        let z = self.target[2] + fz * forward + rz * right;
        let (c, r) = frame.nearest_sample(x, z);
        if frame.contains_sample(c, r) {
            self.target[0] = x;
            self.target[2] = z;
        }
    }

    /// GFX-VIEW-3.2a: the look mode starts: the current view direction is
    /// the one that turns, and the camera stays where it is. The base
    /// direction and the yaw are left as they are, so the release restores
    /// them. Nothing happens before the camera's first update.
    pub fn look_start(&mut self) {
        if let Some(position) = self.last_eye {
            let direction = self.view.unwrap_or_else(|| self.direction());
            self.look = Some(Eye {
                position,
                direction,
            });
        }
    }

    /// GFX-VIEW-3.2a: the look is dropped; the next updates ease the view
    /// back to the yaw-turned base direction and place the camera from the
    /// target again (3.3).
    pub fn look_end(&mut self) {
        self.look = None;
    }

    pub fn looking(&self) -> bool {
        self.look.is_some()
    }

    /// GFX-VIEW-3.2a a: the mouse's movement (a fraction of the window a
    /// frame) turns the view at once: yaw -10 dx radians about the vertical
    /// axis (to the right for dx > 0), then 6 dy added to the vertical
    /// component, clamped to +-0.9, and the direction normalised.
    pub fn look_turn(&mut self, dx: f32, dy: f32) {
        let Some(look) = &mut self.look else { return };
        let [x, y, z] = look.direction;
        let (s, c) = (-10.0 * dx).sin_cos();
        let (x, z) = (c * x - s * z, c * z + s * x);
        let y = (y + 6.0 * dy).clamp(-0.9, 0.9);
        look.direction = normalize([x, y, z]);
    }

    /// GFX-VIEW-3.2a b: the arrows fly the camera and the target by
    /// `(right, forward)` world units along the look's horizontal direction;
    /// a move is taken only when the camera stays over the map.
    pub fn look_fly(&mut self, frame: &Frame, right: f32, forward: f32) {
        let Some(look) = &mut self.look else { return };
        let [x, _, z] = look.direction;
        let length = x.hypot(z).max(1e-6);
        let (fx, fz) = (x / length, z / length);
        let (dx, dz) = (fx * forward + fz * right, fz * forward - fx * right);
        let (c, r) = frame.nearest_sample(look.position[0] + dx, look.position[2] + dz);
        if frame.contains_sample(c, r) {
            look.position[0] += dx;
            look.position[2] += dz;
            self.target[0] += dx;
            self.target[2] += dz;
        }
    }

    /// GFX-VIEW-3.3: one frame of the view update: the target's height
    /// from the ground around it, the yaw easing and the camera position.
    pub fn update(&mut self, heights: &impl Heights) -> Eye {
        let [tx, _, tz] = self.target;
        let around = [
            (0.0, 0.0),
            (10.0, 0.0),
            (-10.0, 0.0),
            (0.0, 10.0),
            (0.0, -10.0),
        ];
        let g = around
            .iter()
            .map(|&(dx, dz)| ground_height(heights, tx + dx, tz + dz))
            .sum::<f32>()
            / around.len() as f32;
        let floor = g.max(WATER_LEVEL + 5.0) + 5.0;
        let ground = match self.ground {
            Some(smoothed) if smoothed >= 0.0 && (smoothed - floor).abs() < 10.0 => smoothed,
            _ => floor,
        };
        self.ground = Some(ground);
        self.target[1] = ground;

        // GFX-VIEW-3.2a c: in the look the camera stays and looks along the
        // look's direction, never below the ground under it or 5 above the
        // water; no yaw easing.
        if let Some(look) = &mut self.look {
            let [x, _, z] = look.position;
            let lowest = ground_height(heights, x, z).max(WATER_LEVEL + 5.0);
            look.position[1] = look.position[1].max(lowest);
            self.view = Some(look.direction);
            self.eye_z = Some(z);
            self.last_eye = Some(look.position);
            return *look;
        }

        if self.scripted {
            return self.scripted_frame(heights);
        }

        self.yaw += (self.yaw_target - self.yaw) * YAW_EASE;
        let d = self.direction();
        let ease = if self.last_zoom.is_some_and(|z| z != self.zoom) {
            DIRECTION_EASE_ZOOMED
        } else {
            DIRECTION_EASE
        };
        self.last_zoom = Some(self.zoom);
        let view = match self.view {
            Some(v) => normalize([0, 1, 2].map(|i| v[i] + (d[i] - v[i]) * ease)),
            None => d,
        };
        self.view = Some(view);
        let back = normalize([d[0], 1.4 * d[1], d[2]]);
        let distance = DISTANCE_PER_ZOOM * f32::from(self.zoom);
        let mut eye = [0, 1, 2].map(|i| self.target[i] - back[i] * distance);
        if let Some(old) = self.eye_z {
            eye[2] += Z_KEEP * (old - eye[2]);
        }
        self.eye_z = Some(eye[2]);
        let lowest = (ground_height(heights, eye[0], eye[2]) + 10.0).max(WATER_LEVEL + 5.0);
        eye[1] = eye[1].max(lowest);
        self.last_eye = Some(eye);
        Eye {
            position: eye,
            direction: view,
        }
    }
}

impl CameraRig {
    /// GFX-VIEW-3.6 e: a frame of the scripted move: zoom 3, the wanted eye
    /// where the normal camera puts it for the target (no easing), the real
    /// eye 12.5 % of the way there, the view direction kept; the move ends
    /// within 0.1 of the wanted eye.
    fn scripted_frame(&mut self, heights: &impl Heights) -> Eye {
        self.zoom = START_ZOOM;
        let d = self.direction();
        let back = normalize([d[0], 1.4 * d[1], d[2]]);
        let distance = DISTANCE_PER_ZOOM * f32::from(self.zoom);
        let mut wanted = [0, 1, 2].map(|i| self.target[i] - back[i] * distance);
        let lowest = (ground_height(heights, wanted[0], wanted[2]) + 10.0).max(WATER_LEVEL + 5.0);
        wanted[1] = wanted[1].max(lowest);
        let real = self.last_eye.unwrap_or(wanted);
        let eye = [0, 1, 2].map(|i| real[i] + (wanted[i] - real[i]) * SCRIPTED_EASE);
        let gap = [0, 1, 2]
            .map(|i| wanted[i] - eye[i])
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt();
        if gap < SCRIPTED_END {
            self.scripted = false;
        }
        self.yaw = self.yaw_target;
        self.last_zoom = Some(self.zoom);
        self.eye_z = Some(eye[2]);
        self.last_eye = Some(eye);
        let view = *self.view.get_or_insert(d);
        Eye {
            position: eye,
            direction: view,
        }
    }
}

/// GFX-VIEW-3.2: the key pan step of a frame: min(1, dt x 25) x (5 s)^2,
/// moved twice that along an axis.
pub fn key_step(dt: f32, speed: f32) -> f32 {
    (dt * 25.0).min(1.0) * (5.0 * speed).powi(2)
}

/// GFX-VIEW-3.2a b: the look's arrow step of a frame, the key step without
/// its min(1, ...) clamp: dt x 25 x (5 s)^2, moved twice that.
pub fn look_step(dt: f32, speed: f32) -> f32 {
    dt * 25.0 * (5.0 * speed).powi(2)
}

/// GFX-VIEW-3.2: the middle-button drag factor 40 x (10 s)^2 per unit of
/// mouse movement.
pub fn drag_factor(speed: f32) -> f32 {
    40.0 * (10.0 * speed).powi(2)
}

/// The point where a ray from `origin` along `direction` (game frame) first
/// meets the ground: the lattice triangles of GFX-VIEW-1.4, the ones the
/// terrain mesh is drawn with, tested exactly. The ray is walked in short
/// pieces; each piece lies inside the fan of six triangles around the
/// sample nearest its start (the fan reaches 1.3 units from the sample, a
/// sample's own area plus a piece less than 1.12; at the map's edge the
/// fans at its middle and end are tested too), so a ray grazing a crest is
/// not stepped over. Off the map there is no ground.
#[cfg(test)]
pub fn ray_ground(
    heights: &impl Heights,
    origin: [f32; 3],
    direction: [f32; 3],
    reach: f32,
) -> Option<[f32; 3]> {
    ray_hit(heights, origin, direction, reach).map(|(hit, _)| hit)
}

/// As [`ray_ground`], with the vertex of the hit triangle nearest the hit
/// point (GFX-VIEW-1.5).
pub fn ray_hit(
    heights: &impl Heights,
    origin: [f32; 3],
    direction: [f32; 3],
    reach: f32,
) -> Option<([f32; 3], (i64, i64))> {
    const STEP: f32 = 0.25;
    let frame = heights.frame();
    let at = |t: f32| [0, 1, 2].map(|i| origin[i] + direction[i] * t);
    let vertex = |c: i64, r: i64| {
        let [x, z] = frame.sample_xz(c, r);
        [x, heights.world(c, r), z]
    };
    let mut t = 0.0;
    while t < reach {
        let mut best: Option<(f32, [(i64, i64); 3])> = None;
        let mut fans: Vec<(i64, i64)> = Vec::with_capacity(3);
        for s in [t, t + STEP / 2.0, t + STEP] {
            let p = at(s);
            let (c, r) = frame.nearest_sample(p[0], p[2]);
            // Near the edge the sample inside the grid; its fan holds the
            // grid's part of the piece.
            let (c, r) = (
                c.clamp(0, i64::from(frame.columns) - 1),
                r.clamp(0, i64::from(frame.rows) - 1),
            );
            if !fans.contains(&(c, r)) {
                fans.push((c, r));
            }
        }
        for (c, r) in fans {
            let centre = vertex(c, r);
            let offsets = neighbour_offsets(r);
            for i in 0..6 {
                let (ac, ar) = (c + offsets[i].0, r + offsets[i].1);
                let (bc, br) = (c + offsets[(i + 1) % 6].0, r + offsets[(i + 1) % 6].1);
                if !frame.contains_sample(ac, ar) || !frame.contains_sample(bc, br) {
                    continue;
                }
                let hit = ray_triangle(origin, direction, centre, vertex(ac, ar), vertex(bc, br));
                if let Some(s) = hit.filter(|&s| s >= t - 1e-4 && s <= t + STEP) {
                    if best.is_none_or(|(b, _)| s < b) {
                        best = Some((s, [(c, r), (ac, ar), (bc, br)]));
                    }
                }
            }
        }
        if let Some((s, corners)) = best {
            let hit = at(s.max(0.0));
            let d = |&(c, r): &(i64, i64)| {
                let v = vertex(c, r);
                (0..3).map(|i| (v[i] - hit[i]).powi(2)).sum::<f32>()
            };
            let nearest = corners
                .into_iter()
                .min_by(|a, b| d(a).total_cmp(&d(b)))
                .expect("three corners");
            return Some((hit, nearest));
        }
        t += STEP;
    }
    None
}

/// GFX-OBJ-2.3: how far a ground decal lies above the terrain.
pub const DECAL_LIFT: f32 = 0.01;

/// A mesh in the game frame: positions, normals, uvs and triangle indices.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Drape {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// A decal triangle: three corners (game frame) with their uvs.
pub type DecalTriangle = [([f32; 3], [f32; 2]); 3];

/// GFX-OBJ-2.3: a ground decal laid on the terrain: the lattice triangles
/// under each of the decal's own triangles, clipped to it in x and z and
/// lifted [`DECAL_LIFT`] above the ground, so the decal follows every
/// slope. A point's uv is the decal triangle's at that place in x and z
/// (its barycentric weights), so the texture lies as the decal mesh lays
/// it (Q-202 d).
pub fn drape(heights: &impl Heights, decal: &[DecalTriangle]) -> Drape {
    let mut out = Drape::default();
    let frame = heights.frame();
    let vertex = |c: i64, r: i64| {
        let [x, z] = frame.sample_xz(c, r);
        [x, heights.world(c, r), z]
    };
    for corners in decal {
        let xz = corners.map(|(p, _)| [p[0], p[2]]);
        let area = cross2(xz[0], xz[1], xz[2]);
        if area.abs() < 1e-6 {
            continue;
        }
        let uv_of = |x: f32, z: f32| {
            let w1 = cross2(xz[2], xz[0], [x, z]) / area;
            let w2 = cross2(xz[0], xz[1], [x, z]) / area;
            let w0 = 1.0 - w1 - w2;
            [0, 1].map(|k| corners[0].1[k] * w0 + corners[1].1[k] * w1 + corners[2].1[k] * w2)
        };
        let (lo, hi) = xz
            .iter()
            .fold(([f32::MAX; 2], [f32::MIN; 2]), |(lo, hi), p| {
                (
                    [lo[0].min(p[0]), lo[1].min(p[1])],
                    [hi[0].max(p[0]), hi[1].max(p[1])],
                )
            });
        let (c0, r0) = frame.nearest_sample(lo[0], lo[1]);
        let (c1, r1) = frame.nearest_sample(hi[0], hi[1]);
        for r in r0 - 1..=r1 + 1 {
            for c in c0 - 2..=c1 + 2 {
                let offsets = neighbour_offsets(r);
                // The two triangles between this row and the next with this
                // sample as their first corner: (P, n4, n5) and (P, n5, n0).
                for (i, j) in [(4, 5), (5, 0)] {
                    let (a, b) = (offsets[i], offsets[j]);
                    let lattice = [(c, r), (c + a.0, r + a.1), (c + b.0, r + b.1)];
                    if !lattice.iter().all(|&(c, r)| frame.contains_sample(c, r)) {
                        continue;
                    }
                    let triangle = lattice.map(|(c, r)| vertex(c, r));
                    let polygon = clip_to_triangle(triangle.to_vec(), xz);
                    if polygon.len() < 3 {
                        continue;
                    }
                    let normal = triangle_normal(triangle);
                    let first = out.positions.len() as u32;
                    for p in &polygon {
                        out.positions.push([p[0], p[1] + DECAL_LIFT, p[2]]);
                        out.normals.push(normal);
                        out.uvs.push(uv_of(p[0], p[2]));
                    }
                    for k in 1..polygon.len() as u32 - 1 {
                        out.indices.extend([first, first + k, first + k + 1]);
                    }
                }
            }
        }
    }
    out
}

/// Twice the signed area of the triangle a, b, q in x and z.
fn cross2(a: [f32; 2], b: [f32; 2], q: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
}

/// A polygon clipped to the triangle `clip` in x and z (Sutherland-Hodgman;
/// the height follows the polygon's plane).
fn clip_to_triangle(mut polygon: Vec<[f32; 3]>, clip: [[f32; 2]; 3]) -> Vec<[f32; 3]> {
    let sign = cross2(clip[0], clip[1], clip[2]).signum();
    for e in 0..3 {
        let (a, b) = (clip[e], clip[(e + 1) % 3]);
        let side = |p: &[f32; 3]| sign * cross2(a, b, [p[0], p[2]]);
        let mut next = Vec::with_capacity(polygon.len() + 2);
        for i in 0..polygon.len() {
            let (p, q) = (polygon[i], polygon[(i + 1) % polygon.len()]);
            let (sp, sq) = (side(&p), side(&q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                next.push([0, 1, 2].map(|k| p[k] + (q[k] - p[k]) * t));
            }
        }
        polygon = next;
        if polygon.is_empty() {
            break;
        }
    }
    polygon
}

/// The upward unit normal of a triangle.
fn triangle_normal([a, b, c]: [[f32; 3]; 3]) -> [f32; 3] {
    let (u, v) = (
        [0, 1, 2].map(|k| b[k] - a[k]),
        [0, 1, 2].map(|k| c[k] - a[k]),
    );
    let mut n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    if n[1] < 0.0 {
        n = n.map(|v| -v);
    }
    let length = n.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-9);
    n.map(|v| v / length)
}

/// The distance along the ray to where it meets the triangle (either
/// side), if it does.
fn ray_triangle(
    origin: [f32; 3],
    direction: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> Option<f32> {
    let sub = |p: [f32; 3], q: [f32; 3]| [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
    let cross = |p: [f32; 3], q: [f32; 3]| {
        [
            p[1] * q[2] - p[2] * q[1],
            p[2] * q[0] - p[0] * q[2],
            p[0] * q[1] - p[1] * q[0],
        ]
    };
    let dot = |p: [f32; 3], q: [f32; 3]| p[0] * q[0] + p[1] * q[1] + p[2] * q[2];
    let (e1, e2) = (sub(b, a), sub(c, a));
    let h = cross(direction, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-9 {
        return None;
    }
    let s = sub(origin, a);
    let u = dot(s, h) / det;
    let q = cross(s, e1);
    let v = dot(direction, q) / det;
    const EDGE: f32 = 1e-5;
    if u < -EDGE || v < -EDGE || u + v > 1.0 + EDGE {
        return None;
    }
    Some(dot(e2, q) / det)
}

/// GFX-VIEW-1.5: the field under a ray (game frame) of length 1000: the
/// first of the terrain's lattice triangles it meets (the ones the terrain
/// mesh is drawn with), then the vertex of that triangle nearest the hit
/// point. The drawn lattice is the fine one; a vertex that is not a field's
/// own sample gives the field nearest it (Q-202). No hit picks nothing.
pub fn ray_field(
    heights: &impl Heights,
    origin: [f32; 3],
    direction: [f32; 3],
) -> Option<(u32, u32)> {
    const REACH: f32 = 1000.0;
    let (_, (c, r)) = ray_hit(heights, origin, direction, REACH)?;
    let frame = heights.frame();
    let [x, z] = frame.sample_xz(c, r);
    let [u, v] = frame.to_plane(x, z);
    let (width, height) = ((frame.columns - 1) / 4, (frame.rows - 1) / 4);
    crate::render::terrain::nearest_field(u, v, width, height).map(|f| (f.x, f.y))
}

#[cfg(test)]
mod tests;

/// GFX-VIEW-3.7: the follow camera's pose while it follows a settler
/// (camera mode 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FollowPose {
    /// The starting pose: 4 behind him, 1 to his side, 2 up, looking at
    /// his head height.
    #[default]
    Behind,
    /// 5 ahead, 1 to his side, 2 up, looking back at his face.
    Ahead,
}

/// GFX-VIEW-3.7: the eye and view direction each move this share of the
/// way to the wanted ones every frame (no time step).
pub const FOLLOW_EASE: f32 = 0.05;

/// GFX-VIEW-3.7: the wanted eye and the point looked at for a settler at
/// `p` whose world matrix has the forward axis `f` and the side axis `s`
/// (game frame, unit length). `door`: when he stands where the game finds a
/// building, the point one step from the building's field in direction 1
/// (the eye's start of that pose, 1.15 above the ground there).
pub fn follow_target(
    pose: FollowPose,
    p: [f32; 3],
    f: [f32; 3],
    s: [f32; 3],
    door: Option<[f32; 3]>,
) -> ([f32; 3], [f32; 3]) {
    let add = |a: [f32; 3], b: [f32; 3], k: f32| [0, 1, 2].map(|i| a[i] + b[i] * k);
    if let Some(start) = door {
        // At a building: the eye 3 units back from L along the line from
        // its start to L, then to the side.
        let look = add(p, [0.0, 1.1, 0.0], 1.0);
        let start = add(start, [0.0, 1.15, 0.0], 1.0);
        let toward = normalize([0, 1, 2].map(|i| look[i] - start[i]));
        let eye = add(add(start, toward, -3.0), s, 1.0);
        return (eye, look);
    }
    let up2 = [0.0, 2.0, 0.0];
    match pose {
        FollowPose::Behind => (
            add(add(add(p, f, -4.0), s, 1.0), up2, 1.0),
            add(p, up2, 1.0),
        ),
        FollowPose::Ahead => (
            add(add(add(p, f, 5.0), s, 1.0), up2, 1.0),
            add(p, [0.0, 1.25, 0.0], 1.0),
        ),
    }
}

/// GFX-VIEW-3.7: one frame of the follow camera: the real eye and view
/// direction 5 % of the way to the wanted eye and to the direction from it to
/// `look`; then, unless `floorless` (the building pose), the eye is raised to
/// at least max(ground + 1, water level + 3) where it stands.
pub fn follow_frame(
    real: Eye,
    wanted: [f32; 3],
    look: [f32; 3],
    floorless: bool,
    heights: &impl Heights,
) -> Eye {
    let wanted_direction = normalize([0, 1, 2].map(|i| look[i] - wanted[i]));
    let mut position =
        [0, 1, 2].map(|i| real.position[i] + (wanted[i] - real.position[i]) * FOLLOW_EASE);
    let direction = normalize(
        [0, 1, 2]
            .map(|i| real.direction[i] + (wanted_direction[i] - real.direction[i]) * FOLLOW_EASE),
    );
    if !floorless {
        let ground = ground_height(heights, position[0], position[2]);
        position[1] = position[1].max((ground + 1.0).max(WATER_LEVEL + 3.0));
    }
    Eye {
        position,
        direction,
    }
}

impl CameraRig {
    /// GFX-VIEW-3.7: the follow ends: the normal camera (mode 0) goes on
    /// with the base direction north and the smoothed ground reset, from
    /// the eye the follow left (the target is not moved back).
    pub fn resume_from(&mut self, eye: Eye) {
        self.base = [0.0, 1.0];
        self.ground = None;
        self.view = Some(eye.direction);
        self.eye_z = Some(eye.position[2]);
        self.last_eye = Some(eye.position);
    }

    /// The eye of the last update, if there was one.
    pub fn last_eye(&self) -> Option<Eye> {
        Some(Eye {
            position: self.last_eye?,
            direction: self.view?,
        })
    }
}
