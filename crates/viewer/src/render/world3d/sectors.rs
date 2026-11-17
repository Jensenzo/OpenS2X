//! GFX-VIEW-1.5 (Q-202 a, Q-1067): the terrain sectors, the list of them a
//! drawn frame makes (a quadtree walked nearest child first) and the pick of
//! the field under a ray over the field-point surface, sector by sector in
//! that list's order, the first triangle hit ending the search.

use super::{ray_triangle, Heights};

/// GFX-VIEW-2.1: a sector is a terrain cell of 4 x 4 fields.
pub const SECTOR: u32 = 4;

/// GFX-VIEW-1.5: the ray's length.
pub const REACH: f32 = 1000.0;

/// The sectors across and down a map of `width` x `height` fields.
pub fn sector_count(width: u32, height: u32) -> (u32, u32) {
    (width.div_ceil(SECTOR), height.div_ceil(SECTOR))
}

/// GFX-VIEW-1.5 (Q-1067): sector `(sx, sy)`'s field range, both ends
/// included: x from 4 sx to min(4 sx + 4, width - 1), y the same way, so
/// neighbouring sectors share a column and a row of fields.
pub fn sector_fields(sx: u32, sy: u32, width: u32, height: u32) -> ([u32; 2], [u32; 2]) {
    let x0 = SECTOR * sx;
    let y0 = SECTOR * sy;
    (
        [x0, (x0 + SECTOR).min(width.saturating_sub(1))],
        [y0, (y0 + SECTOR).min(height.saturating_sub(1))],
    )
}

/// The world position of field `(x, y)`'s point (game frame): its own
/// sample (column 4x on even rows, 4x + 2 on odd ones, row 4y) at its drawn
/// height (Q-202 a). A point one past the map's last column or row (a pick
/// cell on the map's edge, Q-1069, medium) is read from the fine samples
/// without a wrap: a column past the row's end runs on into the next
/// sample row (on an odd row x = width gives sample (1, 4y + 1)); past the
/// last sample row, the last one.
pub fn field_point(heights: &impl Heights, x: u32, y: u32) -> [f32; 3] {
    let frame = heights.frame();
    let (columns, rows) = (i64::from(frame.columns), i64::from(frame.rows));
    let mut c = i64::from(4 * x + if y % 2 == 1 { 2 } else { 0 });
    let mut r = i64::from(4 * y);
    if c >= columns {
        c -= columns;
        r += 1;
    }
    r = r.min(rows - 1);
    let [px, pz] = frame.sample_xz(c, r);
    [px, heights.world(c, r), pz]
}

/// An axis-aligned box in the game frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    const EMPTY: Self = Self {
        min: [f32::MAX; 3],
        max: [f32::MIN; 3],
    };

    fn add(&mut self, p: [f32; 3]) {
        for (i, v) in p.into_iter().enumerate() {
            self.min[i] = self.min[i].min(v);
            self.max[i] = self.max[i].max(v);
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            min: [0, 1, 2].map(|i| self.min[i].min(other.min[i])),
            max: [0, 1, 2].map(|i| self.max[i].max(other.max[i])),
        }
    }

    /// (min + max) / 2.
    pub fn centre(&self) -> [f32; 3] {
        [0, 1, 2].map(|i| (self.min[i] + self.max[i]) / 2.0)
    }

    fn longest(&self) -> f32 {
        (0..3)
            .map(|i| self.max[i] - self.min[i])
            .fold(0.0, f32::max)
    }

    fn contains(&self, p: [f32; 3]) -> bool {
        (0..3).all(|i| p[i] >= self.min[i] && p[i] <= self.max[i])
    }

    /// Whether a ray from `origin` along `direction` meets the box within
    /// `reach`.
    pub fn meets(&self, origin: [f32; 3], direction: [f32; 3], reach: f32) -> bool {
        let (mut near, mut far) = (0.0f32, reach);
        for i in 0..3 {
            let (o, d) = (origin[i], direction[i]);
            if d.abs() < 1e-12 {
                if o < self.min[i] || o > self.max[i] {
                    return false;
                }
                continue;
            }
            let (a, b) = ((self.min[i] - o) / d, (self.max[i] - o) / d);
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return false;
            }
        }
        true
    }
}

/// GFX-VIEW-1.5 walk step 1: where a box lies against the view's six planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    /// Wholly outside one plane.
    Outside,
    /// Wholly inside all six.
    Inside,
    /// Across a plane.
    Straddles,
}

/// GFX-VIEW-1.5 walk step 4: the order an inner node visits its children
/// in, by its nearest child.
const ORDER: [[usize; 4]; 4] = [[0, 1, 2, 3], [1, 0, 3, 2], [2, 0, 3, 1], [3, 1, 2, 0]];

/// A quadtree node over sector indices `x` and `y`, both ends included.
#[derive(Debug, Clone)]
struct Node {
    x: [u32; 2],
    y: [u32; 2],
    children: [Option<usize>; 4],
    bounds: Bounds,
    /// Walk step 3's choice, kept for whole-subtree appends (child 0 in a
    /// new tree).
    kept: usize,
}

/// GFX-VIEW-1.5 (Q-1067): the quadtree over a map's sector grid, with each
/// node's kept nearest child.
#[derive(Debug, Clone)]
pub struct SectorTree {
    width: u32,
    height: u32,
    nodes: Vec<Node>,
}

impl SectorTree {
    /// The tree over a map of `width` x `height` fields: node (x0..x1,
    /// y0..y1) splits at mx = floor((x0 + x1) / 2), my = floor((y0 + y1) /
    /// 2) into child 0 (x0..mx, y0..my), 1 (mx + 1..x1, y0..my), 2 (x0..mx,
    /// my + 1..y1) and 3 (mx + 1..x1, my + 1..y1); a range one sector wide
    /// has only 0 and 2, one sector tall only 0 and 1; a single sector is a
    /// leaf. Its boxes from `heights` (see [`SectorTree::refresh`]).
    pub fn new(heights: &impl Heights) -> Self {
        let frame = heights.frame();
        let (width, height) = ((frame.columns - 1) / 4, (frame.rows - 1) / 4);
        let (nx, ny) = sector_count(width, height);
        let mut tree = Self {
            width,
            height,
            nodes: Vec::new(),
        };
        if nx > 0 && ny > 0 {
            tree.build([0, nx - 1], [0, ny - 1]);
            tree.refresh(heights);
        }
        tree
    }

    fn build(&mut self, x: [u32; 2], y: [u32; 2]) -> usize {
        let at = self.nodes.len();
        self.nodes.push(Node {
            x,
            y,
            children: [None; 4],
            bounds: Bounds::EMPTY,
            kept: 0,
        });
        if x[0] == x[1] && y[0] == y[1] {
            return at;
        }
        let mx = (x[0] + x[1]) / 2;
        let my = (y[0] + y[1]) / 2;
        let mut children = [None; 4];
        if x[0] == x[1] {
            children[0] = Some(self.build(x, [y[0], my]));
            children[2] = Some(self.build(x, [my + 1, y[1]]));
        } else if y[0] == y[1] {
            children[0] = Some(self.build([x[0], mx], y));
            children[1] = Some(self.build([mx + 1, x[1]], y));
        } else {
            children[0] = Some(self.build([x[0], mx], [y[0], my]));
            children[1] = Some(self.build([mx + 1, x[1]], [y[0], my]));
            children[2] = Some(self.build([x[0], mx], [my + 1, y[1]]));
            children[3] = Some(self.build([mx + 1, x[1]], [my + 1, y[1]]));
        }
        self.nodes[at].children = children;
        at
    }

    /// The boxes again after a height change: a leaf's is the box around
    /// its sector's field points (ours: the surface the pick tests; the
    /// spec's "box around the sector's terrain" is medium), an inner node's
    /// the union of its children's. The kept choices stay.
    pub fn refresh(&mut self, heights: &impl Heights) {
        for i in (0..self.nodes.len()).rev() {
            let node = &self.nodes[i];
            let bounds = if node.children.iter().all(Option::is_none) {
                let (xs, ys) = sector_fields(node.x[0], node.y[0], self.width, self.height);
                let mut b = Bounds::EMPTY;
                for y in ys[0]..=ys[1] {
                    for x in xs[0]..=xs[1] {
                        b.add(field_point(heights, x, y));
                    }
                }
                b
            } else {
                node.children
                    .iter()
                    .flatten()
                    .fold(Bounds::EMPTY, |b, &c| b.union(self.nodes[c].bounds))
            };
            self.nodes[i].bounds = bounds;
        }
    }

    /// GFX-VIEW-1.5 (Q-1067): one drawn frame's list of sectors, from the
    /// root, with P = eye + 0.7 (target - eye), the view's far distance and
    /// where a box lies against the view's planes. A node outside a plane is
    /// skipped (1), so is one too far away: P outside its box and the
    /// squared distance from P to its centre above (far + its longest
    /// side)^2 (2). An inner node then takes its nearest child (the smallest
    /// squared distance from P to the child's box centre, a later child
    /// only when strictly smaller) and keeps it (3). A leaf is appended; an
    /// inner node wholly in view appends its whole subtree, each node below
    /// in the order of the child it kept (5); one across a plane visits its
    /// children (4), both in the order [`ORDER`] of the nearest child.
    pub fn walk(
        &mut self,
        p: [f32; 3],
        far: f32,
        seen: &dyn Fn(&Bounds) -> Seen,
    ) -> Vec<(u32, u32)> {
        let mut list = Vec::new();
        if !self.nodes.is_empty() {
            self.visit(0, p, far, seen, &mut list);
        }
        list
    }

    fn visit(
        &mut self,
        i: usize,
        p: [f32; 3],
        far: f32,
        seen: &dyn Fn(&Bounds) -> Seen,
        list: &mut Vec<(u32, u32)>,
    ) {
        let bounds = self.nodes[i].bounds;
        let side = seen(&bounds);
        if side == Seen::Outside {
            return;
        }
        let reach = far + bounds.longest();
        if !bounds.contains(p) && distance2(p, bounds.centre()) > reach * reach {
            return;
        }
        let children = self.nodes[i].children;
        if children.iter().all(Option::is_none) {
            list.push((self.nodes[i].x[0], self.nodes[i].y[0]));
            return;
        }
        let mut nearest: Option<(usize, f32)> = None;
        for (k, c) in children.iter().enumerate() {
            let Some(c) = *c else { continue };
            let d = distance2(p, self.nodes[c].bounds.centre());
            if nearest.is_none_or(|(_, best)| d < best) {
                nearest = Some((k, d));
            }
        }
        let n = nearest.map_or(0, |(k, _)| k);
        self.nodes[i].kept = n;
        for k in ORDER[n] {
            let Some(c) = children[k] else { continue };
            if side == Seen::Inside {
                self.append(c, list);
            } else {
                self.visit(c, p, far, seen, list);
            }
        }
    }

    fn append(&self, i: usize, list: &mut Vec<(u32, u32)>) {
        let node = &self.nodes[i];
        if node.children.iter().all(Option::is_none) {
            list.push((node.x[0], node.y[0]));
            return;
        }
        for k in ORDER[node.kept] {
            if let Some(c) = node.children[k] {
                self.append(c, list);
            }
        }
    }

    /// The box of sector `(sx, sy)`'s leaf.
    fn leaf_bounds(&self, sx: u32, sy: u32) -> Option<Bounds> {
        let mut i = 0;
        loop {
            let node = self.nodes.get(i)?;
            if node.children.iter().all(Option::is_none) {
                return (node.x[0] == sx && node.y[0] == sy).then_some(node.bounds);
            }
            i = node.children.iter().flatten().copied().find(|&c| {
                let n = &self.nodes[c];
                (n.x[0]..=n.x[1]).contains(&sx) && (n.y[0]..=n.y[1]).contains(&sy)
            })?;
        }
    }

    /// GFX-VIEW-1.5 (Q-202 a): the field under a ray (game frame, unit
    /// direction, length 1000) over the sectors of `list` in turn: a sector
    /// only when the ray meets its box; in it the cells row by row (y from
    /// top to bottom, x from left to right; ours: the cells whose corners
    /// lie in its field range), the first triangle of a cell before the
    /// second. An even row's cell (x, y) has the triangles (x, y) (x + 1, y)
    /// (x, y + 1) and (x + 1, y) (x + 1, y + 1) (x, y + 1); an odd row's (x,
    /// y) (x + 1, y + 1) (x, y + 1) and (x, y) (x + 1, y) (x + 1, y + 1),
    /// over the field points. The cells are those whose top-left point lies
    /// in the sector's inclusive range and on the map (Q-1069), so a cell
    /// starting on a shared column or row is tested in both sectors, and a
    /// cell on the map's last column or row has corners past the map
    /// ([`field_point`]). The first triangle hit ends the search; the
    /// field is its corner nearest the hit point (the first when strictly
    /// nearer than both others, else the second when strictly nearer than
    /// both others, else the third); a corner past the map picks nothing
    /// (ours: the spec names no field there). None when nothing is hit.
    pub fn pick(
        &self,
        heights: &impl Heights,
        list: &[(u32, u32)],
        origin: [f32; 3],
        direction: [f32; 3],
    ) -> Option<(u32, u32)> {
        for &(sx, sy) in list {
            let Some(bounds) = self.leaf_bounds(sx, sy) else {
                continue;
            };
            if !bounds.meets(origin, direction, REACH) {
                continue;
            }
            let (xs, ys) = sector_fields(sx, sy, self.width, self.height);
            for y in ys[0]..=ys[1] {
                for x in xs[0]..=xs[1] {
                    let triangles = if y % 2 == 0 {
                        [
                            [(x, y), (x + 1, y), (x, y + 1)],
                            [(x + 1, y), (x + 1, y + 1), (x, y + 1)],
                        ]
                    } else {
                        [
                            [(x, y), (x + 1, y + 1), (x, y + 1)],
                            [(x, y), (x + 1, y), (x + 1, y + 1)],
                        ]
                    };
                    for corners in triangles {
                        let [a, b, c] = corners.map(|(x, y)| field_point(heights, x, y));
                        let Some(t) = ray_triangle(origin, direction, a, b, c) else {
                            continue;
                        };
                        if !(0.0..=REACH).contains(&t) {
                            continue;
                        }
                        let hit = [0, 1, 2].map(|i| origin[i] + direction[i] * t);
                        let [da, db, dc] = [a, b, c].map(|v| distance2(hit, v));
                        let k = if da < db && da < dc {
                            0
                        } else if db < da && db < dc {
                            1
                        } else {
                            2
                        };
                        let (fx, fy) = corners[k];
                        return (fx < self.width && fy < self.height).then_some((fx, fy));
                    }
                }
            }
        }
        None
    }
}

fn distance2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

/// The sector list of the frame last drawn, with the tree it came from and
/// what the tree's boxes were made for (`Viewer::loads`,
/// `World::height_changes`).
#[derive(Debug, Clone, Default)]
pub struct FrameSectors {
    pub tree: Option<SectorTree>,
    pub list: Vec<(u32, u32)>,
    pub made_for: Option<(u64, u64)>,
}

impl FrameSectors {
    /// The tree for `heights`, made new for another map (`loads`) and its
    /// boxes refreshed after a height change.
    pub fn tree_for(
        &mut self,
        heights: &impl Heights,
        loads: u64,
        changes: u64,
    ) -> &mut SectorTree {
        match self.made_for {
            Some((l, c)) if l == loads => {
                if c != changes {
                    if let Some(tree) = self.tree.as_mut() {
                        tree.refresh(heights);
                    }
                }
            }
            _ => {
                self.tree = None;
                self.list.clear();
            }
        }
        self.made_for = Some((loads, changes));
        self.tree.get_or_insert_with(|| SectorTree::new(heights))
    }

    /// GFX-VIEW-1.5: the field under a ray, over the last drawn frame's
    /// list; nothing before a frame was drawn.
    pub fn pick(
        &self,
        heights: &impl Heights,
        origin: [f32; 3],
        direction: [f32; 3],
    ) -> Option<(u32, u32)> {
        self.tree
            .as_ref()?
            .pick(heights, &self.list, origin, direction)
    }
}
