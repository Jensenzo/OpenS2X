//! Roads, waterway buoys, flag-to-door strips and donkey crossings
//! (GFX-OBJ-6).

use super::*;

/// GFX-OBJ-6.2 (play test 10): `width` is the half width; a road is 2.0
/// units wide.
pub const STREET_WIDTH: f32 = 1.0;
pub const STREET_TEX_STRETCH: f32 = 0.25;
pub const CROSSING_RADIUS: f32 = 2.25;
/// GFX-OBJ-6.2a (e), 6.4 (b): a land road lies on the terrain with no
/// lift. This is only a depth-precision stand-in: our strip's cut points
/// are not bit-identical to the terrain's interpolated depth, and without
/// it the depth test would drop scattered pixels.
pub const ROAD_LIFT: f32 = 0.005;

/// A street mesh: positions and normals (Bevy frame), texture coordinates,
/// the tangent along u (Bevy x, z) and indices. Without normals the mesh
/// faces up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Street {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub tangents: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// GFX-OBJ-6.1: `buoyDistance` of `graphics.xml` `<Street>`.
pub const BUOY_DISTANCE: f32 = 1.7;

/// GFX-OBJ-6.2b: the buoys of a waterway through `points` (game x, z, both
/// flags included): the road curve of 6.2a (a) to (c) at the water level
/// `water` with the up normal; for every step its point P and direction D
/// at t = 0.5 and the side S = N x D = (D.z, 0, -D.x); two buoys, at
/// P + S x [`BUOY_DISTANCE`] and P - S x it, in that order (game x, z).
/// Nothing at the flags; the model is not turned.
pub fn buoy_points(points: &[[f32; 2]], water: f32) -> Vec<[f32; 2]> {
    let curve = crate::world::roads::Curve::new(
        points.iter().map(|p| [p[0], water, p[1]]).collect(),
        vec![[0.0, 1.0, 0.0]; points.len()],
    );
    let mut out = Vec::new();
    for i in 0..points.len().saturating_sub(1) {
        let s = curve.sample(i, 0.5);
        let side = [s.dir[2], -s.dir[0]];
        for k in [1.0, -1.0] {
            out.push([
                s.at[0] + side[0] * BUOY_DISTANCE * k,
                s.at[2] + side[1] * BUOY_DISTANCE * k,
            ]);
        }
    }
    out
}

impl Street {
    /// Another mesh's triangles added to this one.
    pub fn append(&mut self, mut other: Street) {
        if self.normals.len() != self.positions.len()
            || other.normals.len() != other.positions.len()
        {
            self.normals.resize(self.positions.len(), [0.0, 1.0, 0.0]);
            other.normals.resize(other.positions.len(), [0.0, 1.0, 0.0]);
        }
        let first = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.uvs.extend(other.uvs);
        self.tangents.extend(other.tangents);
        self.indices
            .extend(other.indices.into_iter().map(|i| first + i));
    }

    /// A draped strip (game frame) in Bevy's frame, [`ROAD_LIFT`] up. F60:
    /// its triangles turned to face up as the crossing's do (the strip's
    /// pieces come from the terrain lattice the other way round, and the
    /// object shader lights a triangle seen from behind with its normal
    /// turned over: the roads were lit from below, by the ambient light
    /// alone).
    fn from_strip(strip: crate::world::roads::Strip) -> Self {
        Self {
            positions: strip
                .positions
                .iter()
                .map(|p| crate::render::world3d::to_bevy([p[0], p[1] + ROAD_LIFT, p[2]]))
                .collect(),
            normals: strip
                .normals
                .iter()
                .map(|n| crate::render::world3d::to_bevy(crate::render::world3d::normalize(*n)))
                .collect(),
            uvs: strip.uvs,
            tangents: strip.tangents.iter().map(|t| [t[0], -t[2]]).collect(),
            indices: strip
                .indices
                .chunks(3)
                .flat_map(|t| [t[0], t[2], t[1]])
                .collect(),
        }
    }
}

/// A field's point with its height and its terrain normal (GFX-VIEW-2.2),
/// in the game frame.
pub(crate) fn field_point(world: &World, field: Point) -> ([f32; 3], [f32; 3]) {
    let (c, r) = s2_sim::grid::field_sample(field);
    let (c, r) = (i64::from(c), i64::from(r));
    let terrain = world.terrain();
    let [x, z] = terrain.frame().sample_xz(c, r);
    (
        [x, terrain.world(c, r), z],
        crate::render::world3d::grid_normal(terrain, c, r),
    )
}

/// GFX-OBJ-6.2a (d) "When the count is taken" (Q-1048, E-gfx-069, T47):
/// each road's (by its fields) stopping one segment short at its start and
/// at its end, decided when its strip is built: the count of roads at that
/// end's flag at that moment, the road itself included, two or more. A
/// strip is built when its road is made, and nothing else rebuilds it, so it
/// keeps that shape while the road stands: the first road at a flag runs to
/// the flag, every later one stops short, also after the first is gone.
pub type RoadEnds = HashMap<Vec<Point>, [bool; 2]>;

/// [`RoadEnds`] brought up to `world`: the roads that are gone leave it; each
/// road not in it yet is decided, in the world's road order, by the roads at
/// its flags that are in it already and itself (a loaded game's roads are
/// made in the save's road order, medium).
pub fn update_road_ends(ends: &mut RoadEnds, world: &World) {
    let present: std::collections::HashSet<&Vec<Point>> = world.roads().map(|r| &r.path).collect();
    ends.retain(|path, _| present.contains(path));
    let mut at: HashMap<Point, usize> = HashMap::new();
    for path in ends.keys() {
        for end in [path[0], path[path.len() - 1]] {
            *at.entry(end).or_insert(0) += 1;
        }
    }
    for road in world.roads() {
        if ends.contains_key(&road.path) {
            continue;
        }
        let path = &road.path;
        let short = [path[0], path[path.len() - 1]].map(|end| {
            let n = at.entry(end).or_insert(0);
            *n += 1;
            *n >= 2
        });
        ends.insert(path.clone(), short);
    }
}

/// [`RoadEnds`] of `world` seen for the first time.
#[cfg(test)]
pub fn road_ends_of(world: &World) -> RoadEnds {
    let mut ends = RoadEnds::new();
    update_road_ends(&mut ends, world);
    ends
}

/// GFX-OBJ-6.2a (d): how many roads end at each flag (the flag's
/// crossing).
pub(crate) fn roads_at_flags(world: &World) -> HashMap<Point, usize> {
    let mut count = HashMap::new();
    for r in world.roads() {
        for end in [r.path[0], r.path[r.path.len() - 1]] {
            *count.entry(end).or_insert(0) += 1;
        }
    }
    count
}

/// GFX-OBJ-6.2, 6.2a: the strips of the roads: `donkey` the roads upgraded
/// to donkey roads (their donkey requested, TRN-CARRY-6), drawn with
/// texture 1, else the others (texture 0). A land road is the Hermite
/// curve through its field points, 8 samples a step, one segment short at
/// an end flag where another road ends too, draped on the terrain.
/// TRN-ROAD-8.4: the roads being built are drawn as roads too, once they
/// have two fields. Waterways (6.2a f) keep a straight strip on the water.
#[cfg(test)]
pub fn streets(world: &World, donkey: bool) -> Street {
    streets_in(world, donkey, None, &road_ends_of(world))
}

/// [`streets`] of the roads whose first field lies in `chunk` (all without),
/// each stopping short at the ends `ends` says (6.2a d, T47).
pub(crate) fn streets_in(
    world: &World,
    donkey: bool,
    chunk: Option<RoadChunk>,
    ends: &RoadEnds,
) -> Street {
    use crate::world::roads::{Curve, Strip};
    use s2_sim::grid::Point as P;
    let mut street = Street::default();
    let counts = roads_at_flags(world);
    let shared = |p: &P| counts.get(p).copied().unwrap_or(0) >= 2;
    // TRN-ROAD-8.4: a road being built is no road yet: its ends by the
    // roads at its flags now.
    let short = |path: &Vec<P>| {
        ends.get(path)
            .copied()
            .unwrap_or_else(|| [shared(&path[0]), shared(&path[path.len() - 1])])
    };
    let roads = world
        .roads()
        .filter(|r| (r.traffic.donkey_requested && !r.water) == donkey)
        .map(|r| (&r.path, r.water));
    let routes = world
        .routes()
        .filter(|(_, r)| r.fields.len() > 1 && !donkey)
        .map(|(_, r)| (&r.fields, r.water));
    let mut land = Strip::default();
    for (path, water) in roads.chain(routes) {
        if chunk.is_some_and(|c| road_chunk(path[0]) != c) {
            continue;
        }
        // GFX-OBJ-6.2a f: waterways have no strip; their buoys stand
        // with the objects (`objects`, `buoy_points`).
        if water {
            continue;
        }
        let (points, normals) = path.iter().map(|p| field_point(world, *p)).unzip();
        let curve = Curve::new(points, normals);
        let [start, end] = short(path);
        let samples = curve.samples(start, end);
        land.road(world.terrain(), &samples, STREET_WIDTH, STREET_TEX_STRETCH);
    }
    street.append(Street::from_strip(land));
    street
}

/// GFX-OBJ-6.4 (F14): a plain road segment from every building's door flag
/// to the building's field point (P0 the flag's, P1 the building's, with
/// their heights): one piece with d = P1 - P0 normalised (3D) and the side
/// s = n x d with each end's terrain normal, 2 x `width` wide, draped on
/// the terrain (6.4 b). u runs across (0 on the P0 - s side), v along: the
/// fraction of the way x |P1 - P0| x texStretch (1.5 at the building on
/// flat ground). Its far half lies under the house. No crossing disc of
/// its own.
#[cfg(test)]
pub fn door_strips(world: &World, _frame: &Frame) -> Street {
    door_strips_in(world, None)
}

/// [`door_strips`] of the buildings in `chunk` (all without).
fn door_strips_in(world: &World, chunk: Option<RoadChunk>) -> Street {
    use crate::world::roads::{Sample, Strip};
    let mut strip = Strip::default();
    for building in world.buildings() {
        if chunk.is_some_and(|c| road_chunk(building.at) != c) {
            continue;
        }
        let Some(flag) = world.terrain().neighbour(building.at, 1) else {
            continue;
        };
        if world.flag_at(flag).is_none() {
            continue;
        }
        let ((p0, n0), (p1, n1)) = (field_point(world, flag), field_point(world, building.at));
        let d = crate::render::world3d::normalize([p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]]);
        let (a, b) = (
            Sample {
                at: p0,
                dir: d,
                normal: n0,
            },
            Sample {
                at: p1,
                dir: d,
                normal: n1,
            },
        );
        let (s0, s1) = (crate::world::roads::side(&a), crate::world::roads::side(&b));
        strip.piece(
            world.terrain(),
            &a,
            &b,
            s0,
            s1,
            STREET_WIDTH,
            0.0,
            STREET_TEX_STRETCH,
        );
    }
    Street::from_strip(strip)
}

/// GFX-OBJ-6.2 (Q-221): the flags whose crossing disc has moved to
/// crossing texture 1: both end flags of every land road whose donkey has
/// been requested. A flag keeps it for as long as it stands (the mark is
/// never cleared); a flag that is gone leaves the set.
pub fn update_donkey_crossings(
    moved: &mut std::collections::HashSet<Point>,
    donkey_road_ends: impl IntoIterator<Item = [Point; 2]>,
    flag_stands: impl Fn(Point) -> bool,
) {
    for ends in donkey_road_ends {
        moved.extend(ends);
    }
    moved.retain(|p| flag_stands(*p));
}

/// The end flags of the land roads whose donkey has been requested.
pub fn donkey_road_ends(world: &World) -> Vec<[Point; 2]> {
    world
        .roads()
        .filter(|r| r.traffic.donkey_requested && !r.water)
        .map(|r| [r.path[0], r.path[r.path.len() - 1]])
        .collect()
}

/// GFX-OBJ-6.2, 6.2a (d), 6.2c (T40): one crossing under every flag with a
/// road (a road stops a segment short where the flag holds two or more);
/// `donkey`: the crossings moved to crossing texture 1, else the others.
/// The crossing is a hexagon of six triangles: the centre at the flag's
/// sample point, normal up; six corners at crossingRadius in the
/// directions (sin a, 0, cos a), a = 60, 120, .. 360 degrees, each on the
/// plane of the terrain triangle under it with that triangle's normal;
/// triangles (centre, corner k + 1, corner k); the texture laid flat in x
/// and z, u = (dx / 2.25 + 1) / 2, v = (dz / 2.25 + 1) / 2. A corner whose
/// direction holds a waterway keeps the flag's height ([`crossing_corners`],
/// T44). No lift but [`ROAD_LIFT`], the roads' depth-precision stand-in.
#[cfg(test)]
pub fn crossings(
    world: &World,
    frame: &Frame,
    moved: &std::collections::HashSet<Point>,
    donkey: bool,
) -> Street {
    crossings_in(world, frame, moved, donkey, None)
}

/// [`crossings`] of the flags in `chunk` (all without).
fn crossings_in(
    world: &World,
    frame: &Frame,
    moved: &std::collections::HashSet<Point>,
    donkey: bool,
    chunk: Option<RoadChunk>,
) -> Street {
    let mut street = Street::default();
    let mut flags: Vec<Point> = world
        .roads()
        .flat_map(|r| [r.path[0], r.path[r.path.len() - 1]])
        .filter(|p| moved.contains(p) == donkey)
        .filter(|p| chunk.is_none_or(|c| road_chunk(*p) == c))
        .collect();
    flags.sort_unstable();
    flags.dedup();
    let terrain = world.terrain();
    // GFX-OBJ-6.2c (T44): the directions (MAP-GRID 0 .. 5) of each flag's
    // waterways.
    let mut water: std::collections::HashMap<Point, [bool; 6]> = std::collections::HashMap::new();
    for road in world.roads().filter(|r| r.water && r.path.len() >= 2) {
        let n = road.path.len();
        for (flag, next) in [
            (road.path[0], road.path[1]),
            (road.path[n - 1], road.path[n - 2]),
        ] {
            if let Some(d) = (0..6u8).find(|d| terrain.neighbour(flag, *d) == Some(next)) {
                water.entry(flag).or_default()[usize::from(d)] = true;
            }
        }
    }
    for flag in flags {
        let [cx, cz] = frame.field_xz(flag.0, flag.1);
        let first = street.positions.len() as u32;
        let mut vertex = |dx: f32, dz: f32, y: f32, normal: [f32; 3]| {
            let (x, z) = (cx + dx, cz + dz);
            let y = y + ROAD_LIFT;
            street
                .positions
                .push(crate::render::world3d::to_bevy([x, y, z]));
            street.normals.push(crate::render::world3d::to_bevy(normal));
            street.uvs.push([
                (dx / CROSSING_RADIUS + 1.0) / 2.0,
                (dz / CROSSING_RADIUS + 1.0) / 2.0,
            ]);
            street.tangents.push([1.0, 0.0]);
        };
        let cy = ground_height(terrain, cx, cz);
        vertex(0.0, 0.0, cy, [0.0, 1.0, 0.0]);
        let corners = crossing_corners(
            [cx, cy, cz],
            water.get(&flag).copied().unwrap_or_default(),
            |x, z| {
                (
                    ground_height(terrain, x, z),
                    crate::render::world3d::ground_normal(terrain, x, z),
                )
            },
        );
        for (at, normal) in corners {
            vertex(at[0] - cx, at[2] - cz, at[1], normal);
        }
        for k in 1..=6u32 {
            street.indices.extend([first, first + k % 6 + 1, first + k]);
        }
    }
    street
}

/// GFX-OBJ-6.2c (T44): a crossing's six corners (game frame, with their
/// normals) around a flag at `centre`, its sample point: corner k (k = 0 ..
/// 5) at 2.25 in the direction (sin a, 0, cos a), a = 60 (k + 1) degrees,
/// on the terrain (`ground`: the height and the normal of the terrain
/// triangle there), except a corner whose map direction k (MAP-GRID: 0
/// east, 1 south-east, .. 5 north-east) holds a waterway (`water`): it
/// keeps the flag's height and the normal the corner before it got ((0, 1,
/// 0) for corner 0). The road in direction k leaves through the side from
/// corner k to corner k + 1.
pub fn crossing_corners(
    centre: [f32; 3],
    water: [bool; 6],
    ground: impl Fn(f32, f32) -> (f32, [f32; 3]),
) -> [([f32; 3], [f32; 3]); 6] {
    let mut normal = [0.0, 1.0, 0.0];
    std::array::from_fn(|k| {
        let a = (60.0 * (k + 1) as f32).to_radians();
        let (x, z) = (
            centre[0] + CROSSING_RADIUS * a.sin(),
            centre[2] + CROSSING_RADIUS * a.cos(),
        );
        if water[k] {
            return ([x, centre[1], z], normal);
        }
        let (y, n) = ground(x, z);
        normal = n;
        ([x, y, z], n)
    })
}

/// What the road meshes are made from, but for the ground (F55): the roads with
/// their donkeys, the roads being built (TRN-ROAD-8.4), the flags and the
/// buildings (their door strips, GFX-OBJ-6.4), as one number. The same number
/// means the same roads.
pub fn road_content(world: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x0100_0000_01b3);
    let field = |p: &Point| u64::from(p.0) << 32 | u64::from(p.1);
    for road in world.roads() {
        mix(u64::from(road.traffic.donkey_requested) | u64::from(road.water) << 1);
        for p in &road.path {
            mix(field(p));
        }
    }
    for (_, route) in world.routes() {
        mix(u64::from(route.water) | 2);
        for p in &route.fields {
            mix(field(p));
        }
    }
    for (p, _) in world.flags() {
        mix(field(&p));
    }
    for building in world.buildings() {
        mix(field(&building.at));
    }
    h
}

/// F55: the road meshes are cut into squares of this many fields, so a road built
/// makes one small mesh again and not all the map's roads (a late map's are
/// 300 000 vertices, 15 MB to the graphics card, each time).
pub const ROAD_CHUNK_FIELDS: u32 = 24;

/// A square of the map's road meshes.
pub type RoadChunk = (u32, u32);

/// The four parts of the street pass (GFX-OBJ-6.2), in the order they are drawn,
/// with their textures: the donkey roads, the other roads with the flag-to-door
/// strips (6.4), and the crossings of each texture.
pub const STREET_PARTS: [(&str, &str); 4] = [
    ("street.dds", "street_n.dds"),
    ("simple_street.dds", "simple_street_n.dds"),
    ("simple_crossing.dds", "simple_crossing_n.dds"),
    ("crossing.dds", "crossing_n.dds"),
];

/// The square a field's road, flag or building is drawn with.
pub fn road_chunk(field: Point) -> RoadChunk {
    (field.0 / ROAD_CHUNK_FIELDS, field.1 / ROAD_CHUNK_FIELDS)
}

/// One mesh of the street pass: the part of [`STREET_PARTS`] in a square.
pub fn street_part(
    world: &World,
    frame: &Frame,
    moved: &std::collections::HashSet<Point>,
    ends: &RoadEnds,
    part: u8,
    chunk: RoadChunk,
) -> Street {
    match part {
        0 => streets_in(world, true, Some(chunk), ends),
        1 => {
            let mut plain = streets_in(world, false, Some(chunk), ends);
            plain.append(door_strips_in(world, Some(chunk)));
            plain
        }
        _ => crossings_in(world, frame, moved, part == 3, Some(chunk)),
    }
}

/// Every mesh the street pass has now, with a number for what it shows but for
/// the ground: the same number means the same strips on the same ground.
pub fn street_parts(
    world: &World,
    moved: &std::collections::HashSet<Point>,
    road_ends: &RoadEnds,
) -> std::collections::BTreeMap<(u8, RoadChunk), u64> {
    let mut out = std::collections::BTreeMap::new();
    let mut add = |part: u8, at: Point, values: &mut dyn Iterator<Item = u64>| {
        let h: &mut u64 = out
            .entry((part, road_chunk(at)))
            .or_insert(0xcbf2_9ce4_8422_2325);
        for v in values {
            *h = (*h ^ v).wrapping_mul(0x0100_0000_01b3);
        }
    };
    let field = |p: &Point| u64::from(p.0) << 32 | u64::from(p.1);
    let counts = roads_at_flags(world);
    let shared = |p: &Point| u64::from(counts.get(p).copied().unwrap_or(0) >= 2);
    for road in world.roads().filter(|r| !r.water) {
        let part = if road.traffic.donkey_requested { 0 } else { 1 };
        let ends = road_ends.get(&road.path).map_or(
            [
                shared(&road.path[0]),
                shared(&road.path[road.path.len() - 1]),
            ],
            |e| e.map(u64::from),
        );
        add(
            part,
            road.path[0],
            &mut road.path.iter().map(field).chain(ends),
        );
    }
    for (_, route) in world
        .routes()
        .filter(|(_, r)| r.fields.len() > 1 && !r.water)
    {
        let ends = [
            shared(&route.fields[0]),
            shared(&route.fields[route.fields.len() - 1]),
        ];
        add(
            1,
            route.fields[0],
            &mut route.fields.iter().map(field).chain(ends),
        );
    }
    for building in world.buildings() {
        let Some(flag) = world.terrain().neighbour(building.at, 1) else {
            continue;
        };
        if world.flag_at(flag).is_some() {
            add(1, building.at, &mut [field(&building.at)].into_iter());
        }
    }
    let mut flags: Vec<Point> = world
        .roads()
        .flat_map(|r| [r.path[0], r.path[r.path.len() - 1]])
        .collect();
    flags.sort_unstable();
    flags.dedup();
    for flag in flags {
        let part = if moved.contains(&flag) { 3 } else { 2 };
        add(part, flag, &mut [field(&flag)].into_iter());
    }
    out
}

/// The squares with a road, a flag or a building's door strip on or next to one
/// of the terrain cells `(column, row)` (F55): ground that moved elsewhere leaves
/// their meshes as they are.
pub fn road_chunks_on_cells(
    world: &World,
    cells: &[(u32, u32)],
) -> std::collections::BTreeSet<RoadChunk> {
    let mut out = std::collections::BTreeSet::new();
    if cells.is_empty() {
        return out;
    }
    let cell_samples = 4 * crate::render::terrain3d::CELL_FIELDS;
    let near = |p: Point| {
        let (c, r) = s2_sim::grid::field_sample(p);
        let (cx, cy) = (c / cell_samples, r / cell_samples);
        cells
            .iter()
            .any(|&(x, y)| cx.abs_diff(x) <= 1 && cy.abs_diff(y) <= 1)
    };
    for road in world.roads() {
        if road.path.iter().any(|&p| near(p)) {
            out.insert(road_chunk(road.path[0]));
        }
    }
    for (_, route) in world.routes() {
        if route.fields.iter().any(|&p| near(p)) {
            out.insert(road_chunk(route.fields[0]));
        }
    }
    for (flag, _) in world.flags() {
        if near(flag) {
            out.insert(road_chunk(flag));
        }
    }
    for building in world.buildings() {
        if near(building.at) {
            out.insert(road_chunk(building.at));
        }
    }
    out
}
