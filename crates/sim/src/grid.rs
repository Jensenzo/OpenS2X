//! Field grid geometry (`specs/MAP-grid-geometry.md`).
//!
//! The map is a triangular lattice with hard edges. Odd rows sit half a field
//! toward +x, so every field has up to six neighbours. Heights live on a second
//! lattice of the same shape, four times finer.

/// A lattice point: a field `(x, y)` or a height sample `(column, row)`.
pub type Point = (u32, u32);

/// MAP-GRID-4.1: height samples per field in each direction.
pub const SAMPLES_PER_FIELD: u32 = 4;

/// One of the six neighbour directions, numbered 0 to 5 as in MAP-GRID-1.2:
/// 0 is along +x and the numbers go round the field in one turning sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Direction(u8);

impl Direction {
    /// All six directions in numeric order.
    pub const ALL: [Direction; 6] = [
        Direction(0),
        Direction(1),
        Direction(2),
        Direction(3),
        Direction(4),
        Direction(5),
    ];

    /// `None` unless `index` is 0 to 5.
    pub fn new(index: u8) -> Option<Self> {
        (index < 6).then_some(Self(index))
    }

    pub fn index(self) -> u8 {
        self.0
    }

    /// MAP-GRID-1.2: direction `(d + 3) mod 6`.
    pub fn opposite(self) -> Self {
        Self((self.0 + 3) % 6)
    }
}

/// One step from `(x, y)` in direction `dir` (MAP-GRID-1.2), with no bounds
/// check: the result may lie outside any map (MAP-GRID-1.4).
pub fn step(x: i32, y: i32, dir: Direction) -> (i32, i32) {
    // Odd rows are shifted toward +x, so their diagonal steps reach one column
    // further.
    let odd = y & 1;
    match dir.0 {
        0 => (x + 1, y),
        1 => (x + odd, y - 1),
        2 => (x + odd - 1, y - 1),
        3 => (x - 1, y),
        4 => (x + odd - 1, y + 1),
        _ => (x + odd, y + 1),
    }
}

/// The neighbour of `point` in direction `dir` on a lattice of `width` by
/// `height` points, or `None` where the map ends (MAP-GRID-1.4). Works for
/// fields and, with the height grid's size, for samples (MAP-GRID-4.1).
pub fn neighbour(point: Point, dir: Direction, width: u32, height: u32) -> Option<Point> {
    let (x, y) = step(
        i32::try_from(point.0).ok()?,
        i32::try_from(point.1).ok()?,
        dir,
    );
    let (x, y) = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);
    (x < width && y < height).then_some((x, y))
}

/// All six neighbours of `point`, indexed by direction.
pub fn neighbours(point: Point, width: u32, height: u32) -> [Option<Point>; 6] {
    Direction::ALL.map(|dir| neighbour(point, dir, width, height))
}

/// MAP-GRID-4.2: the height sample that is field `(x, y)`.
pub fn field_sample(field: Point) -> Point {
    let shift = if field.1 % 2 == 1 { 2 } else { 0 };
    (
        SAMPLES_PER_FIELD * field.0 + shift,
        SAMPLES_PER_FIELD * field.1,
    )
}

/// MAP-GRID-4.3: the field at or before `sample`, truncating. For samples of
/// the closing edge (MAP-GRID-4.6) the result lies outside the map.
pub fn sample_field_truncating(sample: Point) -> Point {
    let y = sample.1 / SAMPLES_PER_FIELD;
    let shift = if y % 2 == 1 { 2 } else { 0 };
    // Truncation toward zero: the first two samples of an odd row give x = 0.
    (sample.0.saturating_sub(shift) / SAMPLES_PER_FIELD, y)
}

/// MAP-GRID-4.4: the nearest field of `sample` on a map of `width` by
/// `height` fields: among the 5 x 5 fields around the truncated one
/// (MAP-GRID-4.3), inside the map, the one whose own sample is the fewest
/// sample steps away, at most 4. TRN-CARRY-7.1 (T62, E-trn-045) gives the
/// order on ties: rows from two after the truncated field to two before,
/// in each row columns from two after to two before, the first at the
/// smallest distance. With none at 4 or less, the truncated field clamped
/// into the map.
pub fn sample_field_nearest(sample: Point, width: u32, height: u32) -> Point {
    let (tx, ty) = sample_field_truncating(sample);
    let mut best: Option<(u32, Point)> = None;
    for dy in (-2i64..=2).rev() {
        for dx in (-2i64..=2).rev() {
            let (x, y) = (i64::from(tx) + dx, i64::from(ty) + dy);
            if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
                continue;
            }
            let field = (x as u32, y as u32);
            let d = distance(field_sample(field), sample);
            if d <= 4 && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, field));
            }
        }
    }
    best.map_or(
        (
            tx.min(width.saturating_sub(1)),
            ty.min(height.saturating_sub(1)),
        ),
        |(_, f)| f,
    )
}

/// MAP-GRID-2.1: number of neighbour steps from `a` to `b`, ignoring
/// obstacles.
pub fn distance(a: Point, b: Point) -> u32 {
    let dx = i64::from(b.0) - i64::from(a.0);
    let dy = (i64::from(b.1) - i64::from(a.1)).unsigned_abs();
    let mut ax = dx.unsigned_abs();
    if dy % 2 == 1 {
        let b_odd = b.1 % 2 == 1;
        if (!b_odd && dx >= 1) || (b_odd && dx <= -1) {
            ax -= 1;
        }
    }
    let steps = if ax > dy / 2 { dy - dy / 2 + ax } else { dy };
    steps as u32
}

/// MAP-GRID-3.2: the fields of ring `k` around `center` in visiting order,
/// leaving out positions outside a `width` x `height` map (MAP-GRID-3.3).
/// Ring 0 is the centre itself.
pub fn ring(center: Point, k: u32, width: u32, height: u32) -> Vec<Point> {
    let inside = |(x, y): (i32, i32)| {
        let (x, y) = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);
        (x < width && y < height).then_some((x, y))
    };
    let start = (center.0 as i32, center.1 as i32);
    if k == 0 {
        return inside(start).into_iter().collect();
    }
    let toward_4 = Direction(4);
    let mut at = start;
    for _ in 0..k {
        at = step(at.0, at.1, toward_4);
    }
    let mut fields = Vec::with_capacity(6 * k as usize);
    for dir in Direction::ALL {
        for _ in 0..k {
            at = step(at.0, at.1, dir);
            fields.extend(inside(at));
        }
    }
    fields
}

/// MAP-GRID-3.1: the centre, then rings 1 to `radius`, inside the map.
pub fn area(center: Point, radius: u32, width: u32, height: u32) -> Vec<Point> {
    (0..=radius)
        .flat_map(|k| ring(center, k, width, height))
        .collect()
}

/// MAP-GRID-6.1: steepness class of a height difference, -4 to 4.
pub fn height_class(difference: i64) -> i8 {
    const LIMITS: [(i64, i8); 4] = [(2400, 4), (1200, 3), (600, 2), (200, 1)];
    for (limit, class) in LIMITS {
        if difference > limit {
            return class;
        }
        if difference < -limit {
            return -class;
        }
    }
    0
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
crate::snap_struct!(Direction, (0));

#[cfg(test)]
mod tests {
    use super::*;

    fn around(x: u32, y: u32) -> [Option<Point>; 6] {
        neighbours((x, y), 80, 80)
    }

    // TRN-CARRY T62 (TRN-CARRY-7.1 "its own field", E-trn-045): the
    // war's sample (129, 85) truncates to (31, 21) but belongs to (32, 21),
    // whose sample (130, 84) is one step away; (87, 80) belongs to (22, 20),
    // not to (21, 20). A field's own sample is its own field.
    // spec: MAP-GRID-4.4, TRN-CARRY-7.1 (TRN-CARRY T62)
    #[test]
    fn a_sample_belongs_to_its_nearest_field() {
        assert_eq!(sample_field_truncating((129, 85)), (31, 21));
        assert_eq!(distance(field_sample((32, 21)), (129, 85)), 1);
        assert_eq!(sample_field_nearest((129, 85), 64, 64), (32, 21));
        assert_eq!(sample_field_truncating((87, 80)), (21, 20));
        assert_eq!(sample_field_nearest((87, 80), 64, 64), (22, 20));
        for f in [(0, 0), (5, 7), (63, 63), (10, 10)] {
            assert_eq!(sample_field_nearest(field_sample(f), 64, 64), f);
        }
        // The closing edge's samples: clamped into the map at worst.
        let edge = sample_field_nearest((256, 256), 64, 64);
        assert!(edge.0 < 64 && edge.1 < 64);
    }

    // T1
    // spec: MAP-GRID-1.2 (T1)
    #[test]
    fn neighbours_on_an_even_row() {
        assert_eq!(
            around(10, 10),
            [
                Some((11, 10)),
                Some((10, 9)),
                Some((9, 9)),
                Some((9, 10)),
                Some((9, 11)),
                Some((10, 11))
            ]
        );
    }

    // T2
    // spec: MAP-GRID-1.2 (T2)
    #[test]
    fn neighbours_on_an_odd_row() {
        assert_eq!(
            around(10, 11),
            [
                Some((11, 11)),
                Some((11, 10)),
                Some((10, 10)),
                Some((9, 11)),
                Some((10, 12)),
                Some((11, 12))
            ]
        );
    }

    // T3..T8, MAP-GRID-1.4
    #[test]
    fn edges_do_not_wrap() {
        assert_eq!(
            around(0, 0),
            [Some((1, 0)), None, None, None, None, Some((0, 1))]
        );
        assert_eq!(
            around(0, 1),
            [
                Some((1, 1)),
                Some((1, 0)),
                Some((0, 0)),
                None,
                Some((0, 2)),
                Some((1, 2))
            ]
        );
        assert_eq!(
            around(79, 0),
            [
                None,
                None,
                None,
                Some((78, 0)),
                Some((78, 1)),
                Some((79, 1))
            ]
        );
        assert_eq!(
            around(79, 78),
            [
                None,
                Some((79, 77)),
                Some((78, 77)),
                Some((78, 78)),
                Some((78, 79)),
                Some((79, 79))
            ]
        );
        assert_eq!(
            around(79, 79),
            [None, None, Some((79, 78)), Some((78, 79)), None, None]
        );
        assert_eq!(
            around(0, 79),
            [
                Some((1, 79)),
                Some((1, 78)),
                Some((0, 78)),
                None,
                None,
                None
            ]
        );
    }

    // T9
    // spec: MAP-GRID-1.3 (T9)
    #[test]
    fn opposite_direction_steps_back() {
        for start in [(10, 10), (10, 11)] {
            for dir in Direction::ALL {
                let (x, y) = step(start.0, start.1, dir);
                assert_eq!(step(x, y, dir.opposite()), start, "{dir:?} from {start:?}");
            }
        }
    }

    // MAP-GRID-1.3: each step uses the parity of the row reached so far.
    #[test]
    fn repeated_steps_follow_row_parity() {
        let dir = Direction::new(5).unwrap();
        let mut at = (10, 10);
        let mut seen = Vec::new();
        for _ in 0..4 {
            at = step(at.0, at.1, dir);
            seen.push(at);
        }
        assert_eq!(seen, [(10, 11), (11, 12), (11, 13), (12, 14)]);
    }

    // spec: MAP-GRID-1.2
    #[test]
    fn direction_index_range() {
        assert_eq!(Direction::new(5).map(Direction::index), Some(5));
        assert_eq!(Direction::new(6), None);
        assert_eq!(Direction::ALL.map(Direction::index), [0, 1, 2, 3, 4, 5]);
    }

    // T17
    // spec: MAP-GRID-4.2 (T17)
    #[test]
    fn field_to_sample() {
        assert_eq!(field_sample((45, 38)), (180, 152));
        assert_eq!(field_sample((45, 39)), (182, 156));
        assert_eq!(field_sample((79, 79)), (318, 316));
    }

    // T18
    // spec: MAP-GRID-4.3 (T18)
    #[test]
    fn sample_to_field_truncates() {
        assert_eq!(sample_field_truncating((182, 156)), (45, 39));
        assert_eq!(sample_field_truncating((181, 156)), (44, 39));
        assert_eq!(sample_field_truncating((183, 159)), (45, 39));
        // Before the first field of an odd row.
        assert_eq!(sample_field_truncating((1, 4)), (0, 1));
    }

    // MAP-GRID-4.2: one field step is four sample steps in the same direction.
    #[test]
    fn field_step_is_four_sample_steps() {
        for field in [(10u32, 10u32), (10, 11)] {
            for dir in Direction::ALL {
                let (c, r) = field_sample(field);
                let mut at = (c as i32, r as i32);
                for _ in 0..SAMPLES_PER_FIELD {
                    at = step(at.0, at.1, dir);
                }
                let (x, y) = step(field.0 as i32, field.1 as i32, dir);
                let expected = field_sample((x as u32, y as u32));
                assert_eq!(at, (expected.0 as i32, expected.1 as i32));
            }
        }
    }

    // MAP-GRID T10, T11
    // spec: MAP-GRID-2.1 (T10, T11)
    #[test]
    fn distances() {
        assert_eq!(distance((10, 10), (10, 10)), 0);
        assert_eq!(distance((10, 10), (13, 10)), 3);
        assert_eq!(distance((10, 10), (10, 14)), 4);
        assert_eq!(distance((10, 10), (12, 13)), 4);
        assert_eq!(distance((10, 11), (8, 14)), 4);
        assert_eq!(distance((0, 0), (79, 79)), 119);
        assert_eq!(distance((45, 38), (25, 4)), 37);
    }

    // MAP-GRID T12: the formula equals the breadth-first step count.
    // spec: MAP-GRID-2.1 (T12)
    #[test]
    fn distance_equals_breadth_first_search() {
        let (width, height) = (60, 60);
        for start in [(20, 20), (20, 21)] {
            let mut steps = vec![u32::MAX; (width * height) as usize];
            let index = |(x, y): Point| (y * width + x) as usize;
            steps[index(start)] = 0;
            let mut queue = std::collections::VecDeque::from([start]);
            while let Some(at) = queue.pop_front() {
                for next in neighbours(at, width, height).into_iter().flatten() {
                    if steps[index(next)] == u32::MAX {
                        steps[index(next)] = steps[index(at)] + 1;
                        queue.push_back(next);
                    }
                }
            }
            for y in 0..height {
                for x in 0..width {
                    let bfs = steps[index((x, y))];
                    if bfs <= 9 {
                        assert_eq!(distance(start, (x, y)), bfs, "{start:?} to ({x}, {y})");
                        assert_eq!(distance((x, y), start), bfs, "({x}, {y}) to {start:?}");
                    }
                }
            }
        }
    }

    // MAP-GRID T14, T15, T16
    // spec: MAP-GRID-3.1, MAP-GRID-3.2, MAP-GRID-3.3 (T14, T15, T16)
    #[test]
    fn ring_order() {
        assert_eq!(
            ring((10, 10), 1, 80, 80),
            [(10, 11), (11, 10), (10, 9), (9, 9), (9, 10), (9, 11)]
        );
        assert_eq!(
            ring((10, 10), 2, 80, 80),
            [
                (10, 12),
                (11, 12),
                (11, 11),
                (12, 10),
                (11, 9),
                (11, 8),
                (10, 8),
                (9, 8),
                (8, 9),
                (8, 10),
                (8, 11),
                (9, 12)
            ]
        );
        assert_eq!(
            ring((10, 11), 1, 80, 80),
            [(11, 12), (11, 11), (11, 10), (10, 10), (9, 11), (10, 12)]
        );
    }

    // MAP-GRID-3.1, 3.3
    #[test]
    fn ring_sizes_and_edges() {
        for k in 0..5 {
            let fields = ring((20, 21), k, 80, 80);
            assert_eq!(fields.len(), if k == 0 { 1 } else { 6 * k as usize });
            assert!(fields.iter().all(|&f| distance((20, 21), f) == k));
        }
        assert_eq!(area((20, 20), 2, 80, 80).len(), 1 + 3 * 2 * 3);
        // A corner keeps only what is inside.
        assert_eq!(ring((0, 0), 1, 80, 80), [(0, 1), (1, 0)]);
        assert_eq!(area((0, 0), 1, 80, 80).len(), 3);
    }

    // MAP-GRID T23
    // spec: MAP-GRID-6.1 (T23)
    #[test]
    fn height_classes() {
        let cases = [
            (200, 0),
            (201, 1),
            (600, 1),
            (601, 2),
            (2400, 3),
            (2401, 4),
            (-200, 0),
            (-201, -1),
            (-2401, -4),
        ];
        for (difference, class) in cases {
            assert_eq!(height_class(difference), class, "{difference}");
        }
    }
}
