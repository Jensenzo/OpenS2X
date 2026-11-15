//! The road renderer's geometry (`specs/GFX-objects.md` GFX-OBJ-6.2a,
//! 6.4): a land road as a Hermite curve through its field points, sampled
//! 8 times a step, and the strip along it draped on the terrain. Plain
//! math in the game frame (x east, y up, z the rows); the caller converts
//! to Bevy's frame.

use crate::render::world3d::{neighbour_offsets, Heights};

type V3 = [f32; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}

fn normalise(a: V3) -> V3 {
    let l = length(a);
    if l == 0.0 {
        a
    } else {
        scale(a, 1.0 / l)
    }
}

/// GFX-OBJ-6.1: numSegments.
pub const SEGMENTS: usize = 8;
/// GFX-OBJ-6.2a (c): both end tangents of a step are scaled by 1.3 L.
pub const TANGENT_SCALE: f32 = 1.3;
/// GFX-OBJ-6.2a (b): the arc estimate's chords (t = 0 .. 31/32).
const ARC_CHORDS: usize = 31;

/// One sample of a road: the centre, the direction D(t) and the normal
/// N(t) (linear between the fields', not normalised), and which step and
/// t it was taken at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub at: V3,
    pub dir: V3,
    pub normal: V3,
}

/// GFX-OBJ-6.2a (a), (b): a road's point tangents and step lengths.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub points: Vec<V3>,
    pub normals: Vec<V3>,
    pub tangents: Vec<V3>,
    /// L_i of the second pass; the last point's is 0.
    pub lengths: Vec<f32>,
}

/// GFX-OBJ-6.2a (c): the Hermite basis at t.
fn basis(t: f32) -> [f32; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [
        2.0 * t3 - 3.0 * t2 + 1.0,
        -2.0 * t3 + 3.0 * t2,
        t3 - 2.0 * t2 + t,
        t3 - t2,
    ]
}

/// The basis' derivative at t.
fn basis_derivative(t: f32) -> [f32; 4] {
    let t2 = t * t;
    [
        6.0 * t2 - 6.0 * t,
        -6.0 * t2 + 6.0 * t,
        3.0 * t2 - 4.0 * t + 1.0,
        3.0 * t2 - 2.0 * t,
    ]
}

impl Curve {
    /// GFX-OBJ-6.2a (a), (b): tangents from the neighbours (one-sided at
    /// the ends), then the step lengths in two passes.
    pub fn new(points: Vec<V3>, normals: Vec<V3>) -> Self {
        let n = points.len();
        let mut tangents = Vec::with_capacity(n);
        let mut lengths = Vec::with_capacity(n);
        for i in 0..n {
            let v = if n < 2 {
                [0.0; 3]
            } else if i == 0 {
                sub(points[1], points[0])
            } else if i + 1 == n {
                sub(points[n - 1], points[n - 2])
            } else {
                sub(points[i + 1], points[i - 1])
            };
            tangents.push(normalise(v));
            lengths.push(length(v));
        }
        let mut curve = Self {
            points,
            normals,
            tangents,
            lengths,
        };
        let arcs: Vec<f32> = (0..n.saturating_sub(1))
            .map(|i| {
                let mut sum = 0.0;
                let mut last = curve.point(i, 0.0);
                for k in 1..=ARC_CHORDS {
                    let p = curve.point(i, k as f32 / 32.0);
                    sum += length(sub(p, last));
                    last = p;
                }
                sum
            })
            .collect();
        for (i, arc) in arcs.into_iter().enumerate() {
            curve.lengths[i] = arc;
        }
        if let Some(last) = curve.lengths.last_mut() {
            *last = 0.0;
        }
        curve
    }

    fn ends(&self, i: usize) -> (V3, V3) {
        let s = TANGENT_SCALE * self.lengths[i];
        (scale(self.tangents[i], s), scale(self.tangents[i + 1], s))
    }

    /// GFX-OBJ-6.2a (c): P(t) of step i.
    pub fn point(&self, i: usize, t: f32) -> V3 {
        let h = basis(t);
        let (m0, m1) = self.ends(i);
        let mut p = scale(self.points[i], h[0]);
        p = add(p, scale(self.points[i + 1], h[1]));
        p = add(p, scale(m0, h[2]));
        add(p, scale(m1, h[3]))
    }

    /// GFX-OBJ-6.2a (c): the sample of step i at t; at t = 0 and 1 the
    /// field's own point, direction and normal.
    pub fn sample(&self, i: usize, t: f32) -> Sample {
        if t <= 0.0 {
            return self.field(i);
        }
        if t >= 1.0 {
            return self.field(i + 1);
        }
        let h = basis_derivative(t);
        let (m0, m1) = self.ends(i);
        let mut d = scale(self.points[i], h[0]);
        d = add(d, scale(self.points[i + 1], h[1]));
        d = add(d, scale(m0, h[2]));
        d = add(d, scale(m1, h[3]));
        let (n0, n1) = (self.normals[i], self.normals[i + 1]);
        Sample {
            at: self.point(i, t),
            dir: normalise(d),
            normal: add(n0, scale(sub(n1, n0), t)),
        }
    }

    fn field(&self, i: usize) -> Sample {
        Sample {
            at: self.points[i],
            dir: self.tangents[i],
            normal: self.normals[i],
        }
    }

    /// GFX-OBJ-6.2a (d): t = k / 8 of every step after t = 0 of step 0;
    /// one segment short at an end flag shared with another road.
    pub fn samples(&self, short_start: bool, short_end: bool) -> Vec<Sample> {
        let steps = self.points.len().saturating_sub(1);
        if steps == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(steps * SEGMENTS + 1);
        for i in 0..steps {
            let first = if i == 0 { 0 } else { 1 };
            for k in first..=SEGMENTS {
                if (i == 0 && k == 0 && short_start)
                    || (i + 1 == steps && k == SEGMENTS && short_end)
                {
                    continue;
                }
                out.push(self.sample(i, k as f32 / SEGMENTS as f32));
            }
        }
        out
    }
}

/// A draped strip in the game frame: positions, normals, texture
/// coordinates, the tangent along u, triangle indices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Strip {
    pub positions: Vec<V3>,
    pub normals: Vec<V3>,
    pub uvs: Vec<[f32; 2]>,
    pub tangents: Vec<V3>,
    pub indices: Vec<u32>,
}

/// GFX-OBJ-6.2a (e): the side vector N x D (not normalised again).
pub fn side(s: &Sample) -> V3 {
    cross(s.normal, s.dir)
}

/// A plane n . p = d with the inside where n . p - d >= 0.
#[derive(Debug, Clone, Copy)]
struct Plane {
    n: V3,
    d: f32,
}

impl Plane {
    fn through(p: V3, n: V3, inside: V3) -> Self {
        let n = normalise(n);
        let plane = Self { n, d: dot(n, p) };
        if plane.distance(inside) < 0.0 {
            Self {
                n: scale(n, -1.0),
                d: -plane.d,
            }
        } else {
            plane
        }
    }

    fn distance(&self, p: V3) -> f32 {
        dot(self.n, p) - self.d
    }
}

/// Sutherland-Hodgman against one plane (in 3D; cut points on the
/// polygon's edges).
fn clip(polygon: Vec<V3>, plane: Plane) -> Vec<V3> {
    let mut out = Vec::with_capacity(polygon.len() + 2);
    for i in 0..polygon.len() {
        let (p, q) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        let (dp, dq) = (plane.distance(p), plane.distance(q));
        if dp >= 0.0 {
            out.push(p);
        }
        if (dp >= 0.0) != (dq >= 0.0) {
            let t = dp / (dp - dq);
            out.push(add(p, scale(sub(q, p), t)));
        }
    }
    out
}

/// The lattice triangles (game frame corners) whose x, z bounds meet
/// `lo` .. `hi`.
fn lattice_triangles(heights: &impl Heights, lo: [f32; 2], hi: [f32; 2]) -> Vec<[V3; 3]> {
    let frame = heights.frame();
    let vertex = |c: i64, r: i64| {
        let [x, z] = frame.sample_xz(c, r);
        [x, heights.world(c, r), z]
    };
    let (c0, r0) = frame.nearest_sample(lo[0], lo[1]);
    let (c1, r1) = frame.nearest_sample(hi[0], hi[1]);
    let mut out = Vec::new();
    for r in r0 - 1..=r1 + 1 {
        for c in c0 - 2..=c1 + 2 {
            let offsets = neighbour_offsets(r);
            // The two triangles between this row and the next with this
            // sample as their first corner (as `world3d::drape`).
            for (i, j) in [(4, 5), (5, 0)] {
                let (a, b) = (offsets[i], offsets[j]);
                let lattice = [(c, r), (c + a.0, r + a.1), (c + b.0, r + b.1)];
                if !lattice.iter().all(|&(c, r)| frame.contains_sample(c, r)) {
                    continue;
                }
                let t = lattice.map(|(c, r)| vertex(c, r));
                let (tlo, thi) = t.iter().fold(([f32::MAX; 2], [f32::MIN; 2]), |(l, h), p| {
                    (
                        [l[0].min(p[0]), l[1].min(p[2])],
                        [h[0].max(p[0]), h[1].max(p[2])],
                    )
                });
                if thi[0] < lo[0] || tlo[0] > hi[0] || thi[1] < lo[1] || tlo[1] > hi[1] {
                    continue;
                }
                out.push(t);
            }
        }
    }
    out
}

impl Strip {
    /// GFX-OBJ-6.2a (e), 6.4 (b), (c): the piece between centre samples A
    /// and B: the terrain triangles under the quad A -/+ S_A w, B -/+ S_B w,
    /// clipped to the four planes through its edges that contain the end
    /// normals, with no lift. u = clamp(((p - A) / w . (S_A + S_B) / 2 + 1)
    /// / 2, 0, 1); v = `v_a` + f x |B - A| x `stretch`, f the way from A's
    /// end plane to B's; the normal N_A + f (N_B - N_A); the tangent
    /// (S_A + S_B) / 2.
    #[allow(clippy::too_many_arguments)]
    pub fn piece(
        &mut self,
        heights: &impl Heights,
        a: &Sample,
        b: &Sample,
        sa: V3,
        sb: V3,
        width: f32,
        v_a: f32,
        stretch: f32,
    ) {
        let corners = [
            sub(a.at, scale(sa, width)),
            sub(b.at, scale(sb, width)),
            add(b.at, scale(sb, width)),
            add(a.at, scale(sa, width)),
        ];
        let centre = scale(add(a.at, b.at), 0.5);
        let mean_normal = scale(add(a.normal, b.normal), 0.5);
        let end_a = Plane::through(a.at, cross(sa, a.normal), centre);
        let end_b = Plane::through(b.at, cross(sb, b.normal), centre);
        let planes = [
            end_a,
            end_b,
            Plane::through(
                corners[0],
                cross(sub(corners[1], corners[0]), mean_normal),
                centre,
            ),
            Plane::through(
                corners[3],
                cross(sub(corners[2], corners[3]), mean_normal),
                centre,
            ),
        ];
        let (lo, hi) = corners
            .iter()
            .fold(([f32::MAX; 2], [f32::MIN; 2]), |(l, h), p| {
                (
                    [l[0].min(p[0]), l[1].min(p[2])],
                    [h[0].max(p[0]), h[1].max(p[2])],
                )
            });
        let across = scale(add(sa, sb), 0.5);
        let step = length(sub(b.at, a.at));
        for triangle in lattice_triangles(heights, lo, hi) {
            let mut polygon = triangle.to_vec();
            for plane in planes {
                polygon = clip(polygon, plane);
                if polygon.is_empty() {
                    break;
                }
            }
            if polygon.len() < 3 {
                continue;
            }
            let first = self.positions.len() as u32;
            for p in &polygon {
                let (da, db) = (end_a.distance(*p), end_b.distance(*p));
                let f = if da + db > 1e-9 { da / (da + db) } else { 0.0 };
                let u =
                    ((dot(scale(sub(*p, a.at), 1.0 / width), across) + 1.0) / 2.0).clamp(0.0, 1.0);
                self.positions.push(*p);
                self.normals
                    .push(add(a.normal, scale(sub(b.normal, a.normal), f)));
                self.uvs.push([u, v_a + f * step * stretch]);
                self.tangents.push(across);
            }
            for k in 1..polygon.len() as u32 - 1 {
                self.indices.extend([first, first + k, first + k + 1]);
            }
        }
    }

    /// GFX-OBJ-6.2a (e): the strip along a road's samples, v running on
    /// from 0 at the first sample.
    pub fn road(&mut self, heights: &impl Heights, samples: &[Sample], width: f32, stretch: f32) {
        let mut v = 0.0;
        for pair in samples.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            self.piece(heights, a, b, side(a), side(b), width, v, stretch);
            v += length(sub(b.at, a.at)) * stretch;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::world3d::Frame;

    const UP: V3 = [0.0, 1.0, 0.0];

    fn close(a: f32, b: f32, e: f32) -> bool {
        (a - b).abs() <= e
    }

    fn close3(a: V3, b: V3, e: f32) -> bool {
        (0..3).all(|k| close(a[k], b[k], e))
    }

    fn flat_curve(points: &[V3]) -> Curve {
        Curve::new(points.to_vec(), vec![UP; points.len()])
    }

    /// GFX-OBJ T27b: a straight flat road of two steps.
    // spec: GFX-OBJ-6.2a (T27b)
    #[test]
    fn t27b_straight_road() {
        let c = flat_curve(&[[0.0; 3], [6.0, 0.0, 0.0], [12.0, 0.0, 0.0]]);
        for t in &c.tangents {
            assert!(close3(*t, [1.0, 0.0, 0.0], 1e-6));
        }
        assert!(close(c.lengths[0], 5.7614, 1e-3), "{:?}", c.lengths);
        assert!(close(c.lengths[1], 5.5400, 1e-3), "{:?}", c.lengths);
        assert_eq!(c.lengths[2], 0.0);
        let s = c.samples(false, false);
        let xs = [
            0.0, 0.8722, 1.6397, 2.3373, 3.0, 3.6627, 4.3603, 5.1278, 6.0, 6.8486, 7.6127, 8.3204,
            9.0, 9.6796, 10.3873, 11.1514, 12.0,
        ];
        assert_eq!(s.len(), 17);
        for (sample, x) in s.iter().zip(xs) {
            assert!(
                close3(sample.at, [x, 0.0, 0.0], 1e-3),
                "{:?} {x}",
                sample.at
            );
            assert!(close3(side(sample), [0.0, 0.0, -1.0], 1e-5));
        }
        // v: 0, 0.2181, 0.4099, ..., 1.5 at x 6, 3.0 at x 12.
        let mut v = vec![0.0];
        for pair in s.windows(2) {
            v.push(v.last().unwrap() + length(sub(pair[1].at, pair[0].at)) * 0.25);
        }
        assert!(close(v[1], 0.2181, 1e-3) && close(v[2], 0.4099, 1e-3));
        assert!(close(v[8], 1.5, 1e-4) && close(v[16], 3.0, 1e-4));
        // Two roads at both flags: 15 samples, 0.8722 to 11.1514.
        let short = c.samples(true, true);
        assert_eq!(short.len(), 15);
        assert!(close(short[0].at[0], 0.8722, 1e-3));
        assert!(close(short[14].at[0], 11.1514, 1e-3));
        let along = short[14].at[0] - short[0].at[0];
        assert!(close(along * 0.25, 2.5698, 1e-3));
    }

    /// GFX-OBJ T27c: a 60 degree bend.
    // spec: GFX-OBJ-6.2a (T27c)
    #[test]
    fn t27c_bend() {
        let c = flat_curve(&[[0.0; 3], [6.0, 0.0, 0.0], [9.0, 0.0, -5.196]]);
        assert!(close3(c.tangents[0], [1.0, 0.0, 0.0], 1e-4));
        assert!(close3(c.tangents[1], [0.866, 0.0, -0.5], 1e-3));
        assert!(close3(c.tangents[2], [0.5, 0.0, -0.866], 1e-3));
        assert!(close(c.lengths[0], 5.9212, 1e-3), "{:?}", c.lengths);
        assert!(close(c.lengths[1], 6.0328, 1e-3), "{:?}", c.lengths);
        let at = |i, t| c.sample(i, t).at;
        assert!(
            close3(at(0, 0.125), [0.9033, 0.0, 0.0526], 1e-3),
            "{:?}",
            at(0, 0.125)
        );
        assert!(
            close3(at(0, 0.5), [3.1289, 0.0, 0.4811], 1e-3),
            "{:?}",
            at(0, 0.5)
        );
        assert!(close3(at(0, 0.75), [4.4859, 0.0, 0.5412], 1e-3));
        assert!(close3(at(0, 1.0), [6.0, 0.0, 0.0], 1e-6));
        assert!(close3(at(1, 0.5), [7.8588, 0.0, -2.2392], 1e-3));
        assert!(close3(
            side(&c.sample(0, 0.5)),
            [0.1751, 0.0, -0.9845],
            1e-3
        ));
        assert!(close3(side(&c.sample(1, 0.0)), [-0.5, 0.0, -0.866], 1e-3));
        let s = c.samples(false, false);
        let v: f32 = s
            .windows(2)
            .map(|p| length(sub(p[1].at, p[0].at)) * 0.25)
            .sum();
        let v1: f32 = s[..9]
            .windows(2)
            .map(|p| length(sub(p[1].at, p[0].at)) * 0.25)
            .sum();
        assert!(close(v1, 1.5376, 1e-3), "{v1}");
        assert!(close(v, 3.0764, 1e-3), "{v}");
        // The swing out to +z before the bend, up to about 0.56.
        let swing = s.iter().map(|p| p.at[2]).fold(f32::MIN, f32::max);
        assert!(close(swing, 0.56, 0.03), "{swing}");
    }

    struct Slope;

    impl Heights for Slope {
        fn frame(&self) -> Frame {
            Frame::new(20, 20)
        }
        fn raw(&self, c: u32, r: u32) -> i32 {
            (c * 37 + r * 91 % 13) as i32 * 20
        }
    }

    struct Level;

    impl Heights for Level {
        fn frame(&self) -> Frame {
            Frame::new(20, 20)
        }
        fn raw(&self, _: u32, _: u32) -> i32 {
            0
        }
    }

    /// GFX-OBJ-6.2a (e), 6.4 (b): the strip lies exactly on the terrain
    /// (every vertex on the ground, no lift), inside the quad, with u and v
    /// as the formulas give.
    #[test]
    fn strip_is_draped_without_lift() {
        let ground = |h: &dyn Fn(f32, f32) -> f32, p: V3| close(p[1], h(p[0], p[2]), 1e-4);
        let slope = |x: f32, z: f32| crate::render::world3d::ground_height(&Slope, x, z);
        let frame = Slope.frame();
        let pts: Vec<V3> = [(5, 5), (6, 5), (7, 6)]
            .iter()
            .map(|&(x, y)| {
                let [px, pz] = frame.field_xz(x, y);
                [px, slope(px, pz), pz]
            })
            .collect();
        let c = flat_curve(&pts);
        let samples = c.samples(false, false);
        let mut strip = Strip::default();
        strip.road(&Slope, &samples, 1.0, 0.25);
        assert!(!strip.indices.is_empty());
        for p in &strip.positions {
            assert!(ground(&slope, *p), "{p:?}");
        }
        // Level ground: one straight step of 6 units, u 0 .. 1 across, v
        // 0 .. 1.5 along, the area 2 x 6.
        let level = Level.frame();
        let (a, b) = (level.field_xz(5, 6), level.field_xz(6, 6));
        let c = flat_curve(&[[a[0], 0.0, a[1]], [b[0], 0.0, b[1]]]);
        let mut strip = Strip::default();
        strip.road(&Level, &c.samples(false, false), 1.0, 0.25);
        let mut area = 0.0;
        for t in strip.indices.chunks(3) {
            let [p, q, r] = [0, 1, 2].map(|k| strip.positions[t[k] as usize]);
            area += length(cross(sub(q, p), sub(r, p))) / 2.0;
        }
        assert!(close(area, 12.0, 1e-3), "{area}");
        for (p, uv) in strip.positions.iter().zip(&strip.uvs) {
            assert!(close(uv[1], (p[0] - a[0]) * 0.25, 1e-4), "{p:?} {uv:?}");
            // S = (0, 0, -1): u = 0 on the +z side (A - S w).
            assert!(
                close(uv[0], (1.0 - (p[2] - a[1])) / 2.0, 1e-4),
                "{p:?} {uv:?}"
            );
        }
        for t in &strip.tangents {
            assert!(close3(*t, [0.0, 0.0, -1.0], 1e-5));
        }
    }
}
