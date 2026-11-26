//! Turns a map into plain mesh data for the top-down view.
//!
//! No rendering types here: positions, colours and indices are plain arrays so
//! the geometry is testable without a window. Geometry follows
//! `specs/MAP-grid-geometry.md`; the map types come from `specs/FMT-s2m-map.md`.

use std::collections::HashMap;

use s2_assets::s2m::{state, FieldPos, Map, SamplePos};
use s2_sim::grid::{self, Direction};

/// MAP-GRID-5.3: world units between two neighbouring fields.
pub const FIELD_SPACING: f32 = 6.0;
/// MAP-GRID-5.3: world units between two field rows.
pub const ROW_SPACING: f32 = 5.19615;
/// MAP-GRID-5.1: world units per sample step along a row and per sample row.
const SAMPLE_STEP: f32 = 1.5;
const SAMPLE_ROW_STEP: f32 = 1.299_037_5;

/// Draw order of the three meshes, as the depth each is placed at: markers
/// above terrain, start positions above the rest.
pub const TERRAIN_LAYER: f32 = 0.0;
pub const MARKER_LAYER: f32 = 1.0;
pub const START_LAYER: f32 = 2.0;
/// Within the start position mesh the fill lies above its outline.
const START_FILL_Z: f32 = 0.5;

/// Vertex positions, linear RGBA vertex colours and triangle indices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    /// Texture coordinates, one per vertex, or empty for an untextured mesh.
    pub uvs: Vec<[f32; 2]>,
}

impl MeshData {
    /// Add the vertices of `other` to this mesh.
    pub fn append(&mut self, other: MeshData) {
        let base = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.colors.extend(other.colors);
        self.indices
            .extend(other.indices.into_iter().map(|i| i + base));
        self.uvs.extend(other.uvs);
    }
}

/// What a deposit type is, as far as the view distinguishes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepositKind {
    Tree,
    Stone,
    Other,
}

/// Position of field `(x, y)` in the map plane (MAP-GRID-5.3). Odd rows are
/// shifted half a field toward +x, and +y is north (MAP-GRID-1.6), so with
/// the view's y axis pointing up north is at the top.
pub fn field_position(x: u32, y: u32) -> [f32; 2] {
    let shift = if y % 2 == 1 { FIELD_SPACING / 2.0 } else { 0.0 };
    [x as f32 * FIELD_SPACING + shift, y as f32 * ROW_SPACING]
}

/// Position of a height sample in the map plane (MAP-GRID-5.2).
pub fn sample_position(sample: SamplePos) -> [f32; 2] {
    let shift = if sample.y % 2 == 1 { 0.5 } else { 0.0 };
    [
        (sample.x as f32 + shift) * SAMPLE_STEP,
        sample.y as f32 * SAMPLE_ROW_STEP,
    ]
}

/// The extent of the map in the plane: `[width, height]` in world units.
pub fn map_extent(width: u32, height: u32) -> [f32; 2] {
    [
        (width.saturating_sub(1) as f32 + 0.5) * FIELD_SPACING,
        height.saturating_sub(1) as f32 * ROW_SPACING,
    ]
}

/// The field nearest to a point of the map plane, or `None` when the point is
/// more than one field spacing away from every field.
pub fn nearest_field(u: f32, v: f32, width: u32, height: u32) -> Option<FieldPos> {
    let row = (v / ROW_SPACING).round() as i64;
    let mut best: Option<(f32, FieldPos)> = None;
    for y in row - 1..=row + 1 {
        let Ok(y) = u32::try_from(y) else { continue };
        if y >= height {
            continue;
        }
        let shift = field_position(0, y)[0];
        let column = ((u - shift) / FIELD_SPACING).round() as i64;
        for x in column - 1..=column + 1 {
            let Ok(x) = u32::try_from(x) else { continue };
            if x >= width {
                continue;
            }
            let [fu, fv] = field_position(x, y);
            let distance = (fu - u).hypot(fv - v);
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, FieldPos { x, y }));
            }
        }
    }
    best.filter(|&(d, _)| d <= FIELD_SPACING).map(|(_, f)| f)
}

/// The terrain as one triangle mesh with a vertex per field, land coloured by
/// terrain type from `colors` (sRGB 0..1) or, for types not in it, from the
/// generated [`palette_color`]. Vertex
/// `y * width + x` is field `(x, y)`. Neighbouring fields are joined into the
/// lattice's own equilateral triangles (MAP-GRID-1.1), two per field and row
/// step, all counter-clockwise seen with north up.
pub fn terrain_mesh(map: &Map, colors: &HashMap<u32, [f32; 3]>) -> MeshData {
    let (width, height) = (map.info.width, map.info.height);
    let mut mesh = MeshData::default();
    for y in 0..height {
        for x in 0..width {
            let [u, v] = field_position(x, y);
            mesh.positions.push([u, v, 0.0]);
            mesh.colors
                .push(field_color(map, colors, FieldPos { x, y }));
        }
    }
    let vertex = |x: u32, y: u32| y * width + x;
    for y in 0..height.saturating_sub(1) {
        for x in 0..width.saturating_sub(1) {
            let (a, b) = (vertex(x, y), vertex(x + 1, y));
            let (c, d) = (vertex(x, y + 1), vertex(x + 1, y + 1));
            if y % 2 == 0 {
                // The row above is shifted toward +x: `c` sits between `a` and `b`.
                mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
            } else {
                // This row is the shifted one: `d` sits above between `a` and `b`.
                mesh.indices.extend_from_slice(&[a, d, c, a, b, d]);
            }
        }
    }
    mesh
}

/// Image pixels per world unit of the baked terrain (2.5 gives 15 pixels
/// per field; the largest map, 256 fields wide, bakes to about 3,850 pixels).
const PIXELS_PER_UNIT: f32 = 2.5;

/// The terrain painted from the textures of its terrain types
/// (`pa_setGfxName`, resolved through `terrain_static_data.xml`,
/// GFX-PAT; LUA-PROP-7, FMT-S2M-4.3), as an sRGB RGBA image covering the
/// map's extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedTerrain {
    pub width: u32,
    pub height: u32,
    /// Row by row from the top (north), 4 bytes per pixel.
    pub rgba: Vec<u8>,
}

/// A terrain type's texture with the pattern's own `texScale` (GFX-PAT-2.3,
/// 3.1): one repeat spans `1 / scale` world units along a row and across.
/// The 3D view also uses the normal map with its `nrmScale` and the render
/// type (GFX-PAT-3.3, GFX-WATER-2.1).
#[derive(Debug, Clone)]
pub struct PatternTexture {
    pub texture: s2_assets::dds::Texture,
    pub scale: [f32; 2],
    pub normal: Option<s2_assets::dds::Texture>,
    /// GFX-VIEW-2.4b: the normal map's lower mip levels as its file stores
    /// them (the 3D view samples the map at its own size with its own chain).
    pub normal_levels: Vec<s2_assets::dds::Level>,
    pub normal_scale: [f32; 2],
    pub kind: u32,
    /// GFX-PAT-5.1, 5.4: the priority and the transition mask set.
    pub priority: u32,
    pub transition: u32,
    /// GFX-VIEW-2.4a: the highlight's intensity and power (type 3 only).
    pub specular: [f32; 2],
}

/// One repeat of a texture reduced to the baked resolution by averaging, in
/// linear light, as `[r, g, b]` per pixel, `width` by `height`.
struct Tile {
    width: usize,
    height: usize,
    pixels: Vec<[f32; 3]>,
}

impl Tile {
    /// The tile's pixel at the texture coordinate `(s, t)`, repeating
    /// (GFX-PAT-3.1).
    fn at(&self, s: f32, t: f32) -> [f32; 3] {
        let x = (s.rem_euclid(1.0) * self.width as f32) as usize % self.width;
        let y = (t.rem_euclid(1.0) * self.height as f32) as usize % self.height;
        self.pixels[y * self.width + x]
    }
}

/// A pattern's texture as one repeat of [`PIXELS_PER_UNIT`] pixels per world
/// unit: `1 / scale` units wide along a row and across (GFX-PAT-3.2).
fn tile_of(pattern: &PatternTexture) -> Tile {
    let texture = &pattern.texture;
    let size = |scale: f32| {
        let units = if scale > 0.0 { 1.0 / scale } else { 1.0 };
        ((units * PIXELS_PER_UNIT).round() as usize).clamp(1, 1024)
    };
    let (tw, th) = (size(pattern.scale[0]), size(pattern.scale[1]));
    let (w, h) = (texture.width as usize, texture.height as usize);
    let mut pixels = vec![[0.0f32; 3]; tw * th];
    if w == 0 || h == 0 || texture.rgba.len() < w * h * 4 {
        return Tile {
            width: tw,
            height: th,
            pixels,
        };
    }
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let mut sum = [0.0f32; 3];
            for y in y0..y1.min(h) {
                for x in x0..x1.min(w) {
                    let at = (y * w + x) * 4;
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += srgb_to_linear(f32::from(texture.rgba[at + c]) / 255.0);
                    }
                }
            }
            let n = ((y1.min(h) - y0) * (x1.min(w) - x0)).max(1) as f32;
            pixels[ty * tw + tx] = sum.map(|c| c / n);
        }
    }
    Tile {
        width: tw,
        height: th,
        pixels,
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// GFX-WATER-5.2: shallow and deep colour of the `Default` water style, the
/// only one that occurs in the release maps (display values).
const SHALLOW_COLOR: [f32; 3] = [0.0, 0.8, 0.3];
const DEEP_COLOR: [f32; 3] = [0.0, 0.0, 0.05];

/// GFX-WATER-2.2: the depth haze of a ground vertex of raw height
/// `raw_height`: the haze colour (display values) and the weight of the lit
/// ground. `None` where the weight is 1 (no haze). At -7.05 and deeper (T9)
/// the vertex colour is 0 in every channel: black, weight 0 (the deep colour
/// is only approached by the vertices above).
pub fn depth_haze(raw_height: i32) -> Option<([f32; 3], f32)> {
    // World height: raw x 0.001.
    let y = raw_height as f32 * 0.001;
    let d = (y + 0.05) / 7.0;
    if d >= 0.0 {
        return None;
    }
    if d <= -1.0 {
        return Some(([0.0; 3], 0.0));
    }
    let k = {
        let mut d = (d * std::f32::consts::FRAC_PI_2).sin() * 0.97;
        if y < -0.1 {
            d = d.clamp(-1.0, -0.07);
        }
        -d
    };
    let colour = [0, 1, 2].map(|c| SHALLOW_COLOR[c] + (DEEP_COLOR[c] - SHALLOW_COLOR[c]) * k);
    Some((colour, 1.0 - k))
}

/// The triangle of the terrain lattice (the one [`terrain_mesh`] makes) that
/// contains the plane point `(u, v)`, as its three fields and barycentric
/// weights. Points outside the lattice give `None`.
fn containing_triangle(
    u: f32,
    v: f32,
    width: u32,
    height: u32,
) -> Option<([FieldPos; 3], [f32; 3])> {
    if width < 2 || height < 2 || v < 0.0 {
        return None;
    }
    let y = ((v / ROW_SPACING).floor() as i64).clamp(0, i64::from(height) - 2) as u32;
    let base = field_position(0, y)[0];
    let first = ((u - base) / FIELD_SPACING).floor() as i64;
    for x in first - 1..=first + 1 {
        let Ok(x) = u32::try_from(x) else { continue };
        if x + 1 >= width {
            continue;
        }
        let f = |dx: u32, dy: u32| FieldPos {
            x: x + dx,
            y: y + dy,
        };
        let (a, b, c, d) = (f(0, 0), f(1, 0), f(0, 1), f(1, 1));
        let triangles = if y.is_multiple_of(2) {
            [[a, b, c], [b, d, c]]
        } else {
            [[a, d, c], [a, b, d]]
        };
        for fields in triangles {
            let [p0, p1, p2] = fields.map(|f| field_position(f.x, f.y));
            let det = (p1[1] - p2[1]) * (p0[0] - p2[0]) + (p2[0] - p1[0]) * (p0[1] - p2[1]);
            let w0 = ((p1[1] - p2[1]) * (u - p2[0]) + (p2[0] - p1[0]) * (v - p2[1])) / det;
            let w1 = ((p2[1] - p0[1]) * (u - p2[0]) + (p0[0] - p2[0]) * (v - p2[1])) / det;
            let w2 = 1.0 - w0 - w1;
            const EDGE: f32 = -1e-4;
            if w0 >= EDGE && w1 >= EDGE && w2 >= EDGE {
                return Some((fields, [w0, w1, w2]));
            }
        }
    }
    None
}

/// Paint the terrain of `map` from `textures` (terrain type id to its
/// texture and scale). Each pixel blends the textures of the three fields of
/// the lattice triangle it lies in by barycentric weight, shaded by height
/// as [`terrain_mesh`] does. The texture coordinate of a pixel is its world
/// position times the type's own `texScale` (GFX-PAT-3.1). A terrain type
/// without a texture is painted with its flat colour from `colors` or the
/// generated palette. Fields below the water line (-100 raw) show their
/// seabed under the depth haze of GFX-WATER-2.2, 2.3: shallow colour to deep
/// colour over 7 world units, the ground fading out with it. The surface's
/// shader (waves, reflection, caustics) waits for the 3D renderer; seen
/// from straight above the surface is nearly clear (GFX-WATER-4.4).
pub fn bake_terrain(
    map: &Map,
    textures: &HashMap<u32, PatternTexture>,
    colors: &HashMap<u32, [f32; 3]>,
) -> BakedTerrain {
    let (w, h) = (map.info.width, map.info.height);
    let [extent_u, extent_v] = map_extent(w, h);
    let width = (extent_u * PIXELS_PER_UNIT).ceil() as u32 + 1;
    let height = (extent_v * PIXELS_PER_UNIT).ceil() as u32 + 1;
    let tiles: HashMap<u32, (Tile, [f32; 2])> = textures
        .iter()
        .map(|(&id, t)| (id, (tile_of(t), t.scale)))
        .collect();
    // Per field: its texture's tile and scale (if any), the colour it falls
    // back to and its brightness, in linear light.
    struct Vertex<'a> {
        tile: Option<&'a (Tile, [f32; 2])>,
        flat: [f32; 3],
        shade: f32,
        /// GFX-WATER-2.2: haze colour (linear) and ground weight.
        haze: Option<([f32; 3], f32)>,
    }
    let vertices: Vec<Vertex> = (0..h)
        .flat_map(|y| (0..w).map(move |x| FieldPos { x, y }))
        .map(|field| {
            let index = map.field_index(field).expect("on the map");
            let pattern = map.patterns[index];
            let height = map.field_height(field).unwrap_or(0);
            let flat = field_color(map, colors, field);
            // GFX-WATER-2.3: the seabed is lit ground blended to the haze;
            // its vertex colour does not shade it.
            let haze = depth_haze(height).map(|(c, weight)| (c.map(srgb_to_linear), weight));
            let shade = if haze.is_some() {
                1.0
            } else {
                shade(map, field, height)
            };
            Vertex {
                tile: tiles.get(&pattern),
                flat: [flat[0], flat[1], flat[2]],
                shade,
                haze,
            }
        })
        .collect();
    let mut rgba = vec![255u8; (width * height * 4) as usize];
    for py in 0..height {
        let v = ((height - 1 - py) as f32) / PIXELS_PER_UNIT;
        for px in 0..width {
            let u = px as f32 / PIXELS_PER_UNIT;
            let (fields, weights) = containing_triangle(u, v, w, h).unwrap_or_else(|| {
                // Outside the lattice: the nearest field alone.
                let field = nearest_field(u, v, w, h).unwrap_or(FieldPos { x: 0, y: 0 });
                ([field; 3], [1.0, 0.0, 0.0])
            });
            let mut rgb = [0.0f32; 3];
            for (field, weight) in fields.into_iter().zip(weights) {
                if weight <= 0.0 {
                    continue;
                }
                let vertex = &vertices[(field.y * w + field.x) as usize];
                // GFX-PAT-3.1: the world position times the vertex's own
                // texScale, repeating.
                let base = vertex.tile.map_or(vertex.flat, |(tile, scale)| {
                    tile.at(u * scale[0], v * scale[1])
                });
                // The flat colour is shaded already.
                let shade = if vertex.tile.is_some() {
                    vertex.shade
                } else {
                    1.0
                };
                for c in 0..3 {
                    let lit = base[c] * shade;
                    let colour = match vertex.haze {
                        Some((haze, keep)) => haze[c] + (lit - haze[c]) * keep,
                        None => lit,
                    };
                    rgb[c] += weight * colour;
                }
            }
            let at = ((py * width + px) * 4) as usize;
            for c in 0..3 {
                rgba[at + c] = (linear_to_srgb(rgb[c]) * 255.0 + 0.5) as u8;
            }
        }
    }
    BakedTerrain {
        width,
        height,
        rgba,
    }
}

/// [`terrain_mesh`] with white vertex colours and texture coordinates into
/// the image [`bake_terrain`] makes of a map of this size, for a textured
/// material.
pub fn textured_terrain_mesh(map: &Map) -> MeshData {
    let (w, h) = (map.info.width, map.info.height);
    let mut mesh = terrain_mesh(map, &HashMap::new());
    let [extent_u, extent_v] = map_extent(w, h);
    let width = (extent_u * PIXELS_PER_UNIT).ceil() + 1.0;
    let height = (extent_v * PIXELS_PER_UNIT).ceil() + 1.0;
    mesh.colors.iter_mut().for_each(|c| *c = [1.0; 4]);
    mesh.uvs = mesh
        .positions
        .iter()
        .map(|&[u, v, _]| {
            [
                (u * PIXELS_PER_UNIT + 0.5) / width,
                (height - 1.0 - v * PIXELS_PER_UNIT + 0.5) / height,
            ]
        })
        .collect();
    mesh
}

/// Markers for deposits (squares) and animals (diamonds) as one mesh. `kinds`
/// maps deposit type ids to what they are; unknown ids are drawn as
/// [`DepositKind::Other`]. Without `deposits` only the animals (a game
/// draws its own deposits); `animals` is false in a game, whose animals
/// roam and are drawn from the simulation.
pub fn marker_mesh(
    map: &Map,
    kinds: &HashMap<u32, DepositKind>,
    deposits: bool,
    animals: bool,
) -> MeshData {
    let mut mesh = MeshData::default();
    for deposit in map.deposits.iter().filter(|_| deposits) {
        let kind = kinds
            .get(&deposit.type_id)
            .copied()
            .unwrap_or(DepositKind::Other);
        let (color, size) = match kind {
            DepositKind::Tree => ([0.03, 0.30, 0.08], 1.5),
            DepositKind::Stone => ([0.82, 0.82, 0.80], 1.3),
            DepositKind::Other => ([0.85, 0.20, 0.75], 1.3),
        };
        // Deposits carry their exact position on the height grid.
        push_square(&mut mesh, sample_position(deposit.position), size, color);
    }
    for animal in map.animals.iter().filter(|_| animals) {
        let at = field_position(animal.position.x, animal.position.y);
        push_diamond(&mut mesh, at, 0.0, 1.6, [1.0, 0.55, 0.0]);
    }
    mesh
}

/// Player start positions as outlined diamonds, one colour per player index
/// (FMT-S2M-2.2: entry *i* belongs to player *i*).
pub fn start_mesh(map: &Map) -> MeshData {
    let mut mesh = MeshData::default();
    for (player, start) in map.info.start_positions.iter().enumerate() {
        let at = field_position(start.x, start.y);
        push_diamond(&mut mesh, at, 0.0, 9.0, [0.0, 0.0, 0.0]);
        push_diamond(&mut mesh, at, START_FILL_Z, 6.5, player_color(player));
    }
    mesh
}

/// One colour per player index.
pub fn player_color(player: usize) -> [f32; 3] {
    const COLORS: [[f32; 3]; 8] = [
        [0.90, 0.10, 0.10],
        [0.15, 0.35, 0.95],
        [0.95, 0.85, 0.10],
        [0.10, 0.75, 0.25],
        [0.95, 0.50, 0.05],
        [0.65, 0.20, 0.85],
        [0.10, 0.80, 0.85],
        [0.95, 0.95, 0.95],
    ];
    COLORS[player % COLORS.len()]
}

/// A stable colour for a terrain type id: the same id always gets the same
/// colour, on every run and every map.
pub fn palette_color(id: u32) -> [f32; 3] {
    let mut h = id.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    let hue = (h & 0xFFFF) as f32 / 65536.0 * 360.0;
    let saturation = 0.35 + ((h >> 16) & 0xFF) as f32 / 255.0 * 0.30;
    let value = 0.62 + ((h >> 24) & 0xFF) as f32 / 255.0 * 0.23;
    hsv_to_rgb(hue, saturation, value)
}

/// Colour of one field: terrain type palette shaded by height, with water and
/// coast taken from the field state bits (MAP-GRID-7.1).
fn field_color(map: &Map, colors: &HashMap<u32, [f32; 3]>, field: FieldPos) -> [f32; 4] {
    let Some(index) = map.field_index(field) else {
        return [0.0, 0.0, 0.0, 1.0];
    };
    let height = map.field_height(field).unwrap_or(0);
    let state = map.states[index];

    let rgb = if state & state::WATER != 0 {
        // Water level is height 0; deeper water is darker.
        let depth = (-height as f32 / 10_000.0).clamp(0.0, 1.0);
        mix([0.22, 0.52, 0.74], [0.03, 0.12, 0.36], depth)
    } else {
        let pattern = map.patterns[index];
        let mut rgb = colors
            .get(&pattern)
            .copied()
            .unwrap_or_else(|| palette_color(pattern));
        if state & state::COAST != 0 {
            rgb = mix(rgb, [0.90, 0.82, 0.55], 0.45);
        }
        let shade = shade(map, field, height);
        rgb.map(|c| (c * shade).clamp(0.0, 1.0))
    };
    let [r, g, b] = rgb.map(srgb_to_linear);
    [r, g, b, 1.0]
}

/// Brightness from height: higher ground is lighter, and slopes facing the
/// light from the north-west are lighter than those facing away.
fn shade(map: &Map, field: FieldPos, height: i32) -> f32 {
    let neighbour_height = |direction: u8| {
        Direction::new(direction)
            .and_then(|d| grid::neighbour((field.x, field.y), d, map.info.width, map.info.height))
            .and_then(|(x, y)| map.field_height(FieldPos { x, y }))
            .unwrap_or(height)
    };
    // MAP-GRID-1.6: direction 1 is south-east, 4 is north-west.
    let rise_to_south_east = (neighbour_height(1) - neighbour_height(4)) as f32;
    let elevation = 0.82 + 0.33 * (height as f32 / 15_000.0).clamp(0.0, 1.0);
    elevation + (rise_to_south_east / 6_000.0).clamp(-0.25, 0.25)
}

fn push_square(mesh: &mut MeshData, at: [f32; 2], half: f32, rgb: [f32; 3]) {
    let corners = [[-half, -half], [half, -half], [half, half], [-half, half]];
    push_quad(mesh, at, 0.0, corners, rgb);
}

fn push_diamond(mesh: &mut MeshData, at: [f32; 2], z: f32, half: f32, rgb: [f32; 3]) {
    let corners = [[0.0, -half], [half, 0.0], [0.0, half], [-half, 0.0]];
    push_quad(mesh, at, z, corners, rgb);
}

/// Four corners given counter-clockwise, as offsets from `at`.
fn push_quad(mesh: &mut MeshData, at: [f32; 2], z: f32, corners: [[f32; 2]; 4], rgb: [f32; 3]) {
    let first = mesh.positions.len() as u32;
    let [r, g, b] = rgb.map(srgb_to_linear);
    for [du, dv] in corners {
        mesh.positions.push([at[0] + du, at[1] + dv, z]);
        mesh.colors.push([r, g, b, 1.0]);
    }
    mesh.indices
        .extend([0, 1, 2, 0, 2, 3].map(|corner| first + corner));
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let sector = (hue / 60.0).rem_euclid(6.0);
    let chroma = value * saturation;
    let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;
    [r + m, g + m, b + m]
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_assets::s2m::{
        Animal, AnimalRespawn, Deposit, Doodads, HeightGrid, LogicState, MapInfo, Resource,
        MAX_PLAYERS, NO_GOOD, NO_OBJECT,
    };

    /// A map of `width` x `height` land fields at height 0 with nothing on it.
    fn flat_map(width: u32, height: u32) -> Map {
        let cells = (width * height) as usize;
        let (ew, eh) = (4 * width + 1, 4 * height + 1);
        Map {
            info: MapInfo {
                start_positions: Vec::new(),
                name: "flat".into(),
                width,
                height,
                player_types: [0; MAX_PLAYERS],
                player_records: [[0; 4]; MAX_PLAYERS],
                info_values: [0; 3],
                uuid: [0; 16],
                save: None,
            },
            logic: LogicState {
                next_unique_id: 1,
                clock: [0.0; 3],
                tick: -1,
            },
            triggers: Vec::new(),
            heights: HeightGrid {
                floor: -100,
                width: ew,
                height: eh,
                samples: vec![0; (ew * eh) as usize],
            },
            patterns: vec![0x4545_FAC4; cells],
            states: vec![state::FOR_BUILDING; cells],
            resources: vec![
                Resource {
                    amount: 0,
                    good: NO_GOOD
                };
                cells
            ],
            territory: vec![-1; cells],
            exploration: vec![vec![0; cells]; MAX_PLAYERS],
            continent_of: vec![0; cells],
            continents: Vec::new(),
            continents_trailer: 0,
            deposits: Vec::new(),
            animals: Vec::new(),
            animal_respawn: AnimalRespawn {
                counter: 0,
                target_population: 0,
                timer: [0; 2],
            },
            resources_trailer: [0; 2],
            doodads: Doodads::default(),
            ambients: Vec::new(),
            padding: 0,
            game_block: None,
        }
    }

    fn set_height(map: &mut Map, x: u32, y: u32, height: i32) {
        let sample = FieldPos { x, y }.sample();
        let index = (sample.y * map.heights.width + sample.x) as usize;
        map.heights.samples[index] = height;
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        (a[0] - b[0]).hypot(a[1] - b[1])
    }

    // MAP-GRID-5.3 (and T20: field (45,39) is at u 273.0, v 202.64985).
    #[test]
    fn field_positions() {
        assert_eq!(field_position(0, 0), [0.0, 0.0]);
        assert_eq!(field_position(3, 0), [18.0, 0.0]);
        assert_eq!(field_position(45, 38), [270.0, 38.0 * ROW_SPACING]);
        let [u, v] = field_position(45, 39);
        assert_eq!(u, 273.0);
        assert!((v - 202.64985).abs() < 1e-3);
    }

    // MAP-GRID-1.1: odd rows are half a field toward +x, even rows are not.
    #[test]
    fn odd_rows_are_offset_and_even_rows_are_not() {
        for y in 0..6 {
            let shift = field_position(0, y)[0];
            assert_eq!(shift, if y % 2 == 1 { 3.0 } else { 0.0 }, "row {y}");
            assert_eq!(field_position(1, y)[0] - shift, FIELD_SPACING);
        }
    }

    // MAP-GRID-1.6: +y is north, so rows go up the screen.
    #[test]
    fn rows_run_north() {
        assert!(field_position(0, 1)[1] > field_position(0, 0)[1]);
        assert!((field_position(7, 2)[1] - 2.0 * ROW_SPACING).abs() < 1e-6);
    }

    // MAP-GRID-4.2, MAP-GRID-5.2: a field and its own sample are the same point.
    #[test]
    fn sample_positions_agree_with_field_positions() {
        for (x, y) in [(0, 0), (10, 10), (10, 11), (45, 39)] {
            let [fu, fv] = field_position(x, y);
            let [su, sv] = sample_position(FieldPos { x, y }.sample());
            assert!(
                (fu - su).abs() < 1e-3 && (fv - sv).abs() < 1e-3,
                "({x}, {y})"
            );
        }
        // Odd sample rows are half a sample step toward +x.
        assert_eq!(sample_position(SamplePos { x: 0, y: 1 })[0], 0.75);
    }

    // spec: MAP-GRID-1.1, MAP-GRID-4.5
    #[test]
    fn terrain_vertex_and_triangle_counts() {
        for (width, height) in [(2, 2), (5, 4), (80, 80)] {
            let mesh = terrain_mesh(&flat_map(width, height), &HashMap::new());
            let vertices = (width * height) as usize;
            assert_eq!(mesh.positions.len(), vertices);
            assert_eq!(mesh.colors.len(), vertices);
            let triangles = 2 * (width - 1) * (height - 1);
            assert_eq!(mesh.indices.len(), 3 * triangles as usize);
            assert!(mesh.indices.iter().all(|&i| (i as usize) < vertices));
        }
    }

    // spec: MAP-GRID-1.1
    #[test]
    fn terrain_vertices_are_the_fields() {
        let map = flat_map(5, 4);
        let mesh = terrain_mesh(&map, &HashMap::new());
        for (x, y) in [(0, 0), (4, 0), (0, 1), (2, 3), (4, 3)] {
            let [u, v] = field_position(x, y);
            assert_eq!(mesh.positions[(y * 5 + x) as usize], [u, v, 0.0]);
        }
        // Row 1 is the shifted one.
        assert_eq!(mesh.positions[5][0], 3.0);
        assert_eq!(mesh.positions[10][0], 0.0);
    }

    // MAP-GRID-1.1: every triangle joins three mutual neighbours, so all its
    // sides are one field spacing long, and all turn the same way.
    #[test]
    fn terrain_triangles_are_equilateral_and_counter_clockwise() {
        let mesh = terrain_mesh(&flat_map(6, 5), &HashMap::new());
        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.positions[triangle[i] as usize]);
            for side in [distance(a, b), distance(b, c), distance(c, a)] {
                assert!((side - FIELD_SPACING).abs() < 1e-3, "side {side}");
            }
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(cross > 0.0);
        }
    }

    // MAP-GRID-1.2: the triangles use exactly the neighbour pairs of the grid.
    #[test]
    fn terrain_edges_are_grid_neighbours() {
        let (width, height) = (6, 5);
        let mesh = terrain_mesh(&flat_map(width, height), &HashMap::new());
        for triangle in mesh.indices.chunks(3) {
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                let (a, b) = (triangle[i], triangle[j]);
                let from = (a % width, a / width);
                let to = (b % width, b / width);
                assert!(
                    grid::neighbours(from, width, height).contains(&Some(to)),
                    "{from:?} - {to:?}"
                );
            }
        }
    }

    // spec: none (the 2D viewer's own colour palette for terrain ids)
    #[test]
    fn palette_is_stable_and_distinguishes_ids() {
        assert_eq!(palette_color(0xBABE_B00B), palette_color(0xBABE_B00B));
        assert_ne!(palette_color(0xBABE_B00B), palette_color(0x4545_FAC4));
        for id in [0, 1, 0x0133_74E4, u32::MAX] {
            assert!(palette_color(id).iter().all(|c| (0.0..=1.0).contains(c)));
        }
    }

    // MAP-GRID-7.1: water and coast come from the state bits.
    #[test]
    fn water_and_coast_are_coloured_from_state_bits() {
        let mut map = flat_map(4, 4);
        let land = terrain_mesh(&map, &HashMap::new()).colors[5];
        map.states[5] = state::WATER;
        let water = terrain_mesh(&map, &HashMap::new()).colors[5];
        assert!(water[2] > water[0] && water[2] > water[1], "water is blue");
        assert_ne!(water, land);
        map.states[5] = state::COAST;
        assert_ne!(terrain_mesh(&map, &HashMap::new()).colors[5], land);
    }

    // spec: none (the 2D viewer's own terrain colours)
    #[test]
    fn terrain_colours_come_from_the_given_table() {
        let map = flat_map(4, 4);
        let generated = terrain_mesh(&map, &HashMap::new()).colors[5];
        let natural = HashMap::from([(0x4545_FAC4, [0.2, 0.6, 0.2])]);
        let coloured = terrain_mesh(&map, &natural).colors[5];
        assert_ne!(coloured, generated);
        assert!(coloured[1] > coloured[0] && coloured[1] > coloured[2]);
        // Other ids still use the generated palette.
        let other = HashMap::from([(1, [1.0, 0.0, 0.0])]);
        assert_eq!(terrain_mesh(&map, &other).colors[5], generated);
    }

    // spec: none (the 2D viewer's own depth shading)
    #[test]
    fn deeper_water_is_darker() {
        let mut map = flat_map(4, 4);
        map.states[5] = state::WATER;
        set_height(&mut map, 1, 1, -500);
        let shallow = terrain_mesh(&map, &HashMap::new()).colors[5];
        set_height(&mut map, 1, 1, -9000);
        let deep = terrain_mesh(&map, &HashMap::new()).colors[5];
        assert!(deep[2] < shallow[2]);
    }

    // spec: none (the 2D viewer's own height shading)
    #[test]
    fn higher_ground_is_lighter() {
        let mut map = flat_map(5, 5);
        let low = terrain_mesh(&map, &HashMap::new()).colors[12];
        set_height(&mut map, 2, 2, 9000);
        let high = terrain_mesh(&map, &HashMap::new()).colors[12];
        assert!(high[1] > low[1]);
    }

    // spec: MAP-GRID-1.1
    #[test]
    fn nearest_field_finds_fields_on_both_row_kinds() {
        for (x, y) in [(0, 0), (10, 10), (10, 11), (79, 79)] {
            let [u, v] = field_position(x, y);
            assert_eq!(
                nearest_field(u + 1.0, v - 1.0, 80, 80),
                Some(FieldPos { x, y })
            );
        }
        // Just right of the midpoint between (10,11) and (11,11).
        let [u, v] = field_position(10, 11);
        assert_eq!(
            nearest_field(u + 3.2, v, 80, 80),
            Some(FieldPos { x: 11, y: 11 })
        );
    }

    // spec: MAP-GRID-1.4
    #[test]
    fn nearest_field_is_none_off_the_map() {
        assert_eq!(nearest_field(-20.0, 0.0, 80, 80), None);
        assert_eq!(nearest_field(0.0, 80.0 * ROW_SPACING + 10.0, 80, 80), None);
        assert_eq!(nearest_field(0.0, 0.0, 0, 0), None);
        // Close to the edge still counts.
        assert_eq!(
            nearest_field(-2.0, -2.0, 80, 80),
            Some(FieldPos { x: 0, y: 0 })
        );
    }

    // spec: MAP-GRID-1.1
    #[test]
    fn extent_covers_the_shifted_rows() {
        let [width, height] = map_extent(80, 80);
        assert_eq!(width, field_position(79, 1)[0]);
        assert_eq!(height, field_position(0, 79)[1]);
    }

    // spec: none (the 2D viewer's markers for map objects)
    #[test]
    fn markers_for_deposits_animals_and_starts() {
        let mut map = flat_map(8, 8);
        let field = FieldPos { x: 3, y: 5 };
        for (type_id, id) in [(0x7EE, 1), (0x570, 2), (0x999, 3)] {
            map.deposits.push(Deposit {
                type_id,
                id,
                field,
                linked_id: NO_OBJECT,
                position: field.sample(),
                size: 1.0,
                unknown_a: 0,
                unknown_b: -1,
            });
        }
        map.animals.push(Animal {
            type_id: 1,
            id: 4,
            walk_start: 0.0,
            home: field,
            path: None,
            position: FieldPos { x: 6, y: 2 },
            current: FieldPos { x: 6, y: 2 },
            destination: FieldPos { x: 6, y: 2 },
            step_target: None,
            progress: -1.0,
            carry_over: 0.0,
            progress_per_tick: 0.0,
            counters: [0; 2],
            linked_id: NO_OBJECT,
        });
        map.info.start_positions = vec![FieldPos { x: 1, y: 1 }, FieldPos { x: 6, y: 6 }];
        let kinds = HashMap::from([(0x7EE, DepositKind::Tree), (0x570, DepositKind::Stone)]);
        let center = |mesh: &MeshData, quad: usize| {
            let corners = &mesh.positions[4 * quad..4 * quad + 4];
            [0, 1].map(|axis| corners.iter().map(|c| c[axis]).sum::<f32>() / 4.0)
        };
        let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-3;

        // Three deposits and one animal.
        let markers = marker_mesh(&map, &kinds, true, true);
        assert_eq!(markers.positions.len(), 4 * 4);
        assert_eq!(markers.indices.len(), 6 * 4);
        assert!(close(center(&markers, 0), field_position(3, 5)));
        assert!(close(center(&markers, 3), field_position(6, 2)));
        // Tree, stone and unknown deposits get different colours.
        let color = |quad: usize| markers.colors[4 * quad];
        assert_ne!(color(0), color(1));
        assert_ne!(color(1), color(2));
        assert_ne!(color(0), color(2));

        // Two start positions, each an outline with a fill above it.
        let starts = start_mesh(&map);
        assert_eq!(starts.positions.len(), 4 * 4);
        assert_eq!(starts.indices.len(), 6 * 4);
        assert!(close(center(&starts, 1), field_position(1, 1)));
        assert!(close(center(&starts, 3), field_position(6, 6)));
        assert!(starts.positions[4][2] > starts.positions[0][2]);
        assert_ne!(starts.colors[4], starts.colors[12]);
        assert_eq!(starts.colors[4][..3], player_color(0).map(srgb_to_linear));

        // Markers lie above the terrain and start positions above markers.
        const { assert!(TERRAIN_LAYER < MARKER_LAYER && MARKER_LAYER < START_LAYER) };
    }
    /// A solid-colour 8x8 texture.
    fn solid(rgb: [u8; 3]) -> s2_assets::dds::Texture {
        s2_assets::dds::Texture {
            width: 8,
            height: 8,
            format: s2_assets::dds::Format::Dxt1,
            mip_levels: 1,
            cube_map: false,
            rgba: (0..64)
                .flat_map(|_| [rgb[0], rgb[1], rgb[2], 255])
                .collect(),
        }
    }

    // The containing triangle's weights reproduce the point from its fields
    // for points all over the lattice.
    // spec: MAP-GRID-4.5
    #[test]
    fn containing_triangle_weights_reproduce_the_point() {
        let (w, h) = (6, 5);
        let [eu, ev] = map_extent(w, h);
        for iu in 0..40 {
            for iv in 0..30 {
                let (u, v) = (eu * iu as f32 / 39.0, ev * iv as f32 / 29.0);
                // Odd rows are shifted toward +x, so the lattice does not
                // cover the strips left of its first odd-row field.
                let Some((fields, weights)) = containing_triangle(u, v, w, h) else {
                    continue;
                };
                assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-4);
                let mut at = [0.0f32; 2];
                for (field, weight) in fields.into_iter().zip(weights) {
                    let p = field_position(field.x, field.y);
                    at[0] += weight * p[0];
                    at[1] += weight * p[1];
                }
                assert!(
                    (at[0] - u).abs() < 1e-2 && (at[1] - v).abs() < 1e-2,
                    "({u}, {v})"
                );
            }
        }
    }

    // A map of one terrain type with a solid texture bakes to that colour
    // (times the flat map's brightness), and untextured types keep their
    // flat colour.
    // spec: GFX-PAT-3.1, GFX-PAT-3.2
    #[test]
    fn baked_terrain_takes_the_texture_colour() {
        let map = flat_map(6, 6);
        let textures = HashMap::from([(
            0x4545_FAC4,
            PatternTexture {
                texture: solid([200, 40, 40]),
                scale: [0.065, 0.065],
                normal: None,
                normal_levels: Vec::new(),
                normal_scale: [1.0, 1.0],
                kind: 0,
                priority: 0,
                specular: [0.0, 1.0],
                transition: 0,
            },
        )]);
        let baked = bake_terrain(&map, &textures, &HashMap::new());
        let [eu, ev] = map_extent(6, 6);
        assert_eq!(baked.width, (eu * PIXELS_PER_UNIT).ceil() as u32 + 1);
        assert_eq!(baked.height, (ev * PIXELS_PER_UNIT).ceil() as u32 + 1);
        assert_eq!(baked.rgba.len(), (baked.width * baked.height * 4) as usize);
        let at = ((baked.height / 2 * baked.width + baked.width / 2) * 4) as usize;
        let [r, g, b, a] = [0, 1, 2, 3].map(|c| baked.rgba[at + c]);
        assert!(r > g && r > b && a == 255, "{r} {g} {b}");
        // No textures: the palette colour of the type, not red-dominant.
        let flat = bake_terrain(&map, &HashMap::new(), &HashMap::new());
        assert_ne!(flat.rgba, baked.rgba);
    }

    /// A 2 x 1 texture: left pixel `left`, right pixel `right`.
    fn halves(left: [u8; 3], right: [u8; 3]) -> s2_assets::dds::Texture {
        s2_assets::dds::Texture {
            width: 2,
            height: 1,
            format: s2_assets::dds::Format::Dxt1,
            mip_levels: 1,
            cube_map: false,
            rgba: vec![
                left[0], left[1], left[2], 255, right[0], right[1], right[2], 255,
            ],
        }
    }

    /// The baked colour at the plane point `(u, v)`.
    fn pixel(baked: &BakedTerrain, u: f32, v: f32) -> [u8; 3] {
        let x = (u * PIXELS_PER_UNIT).round() as u32;
        let y = baked.height - 1 - (v * PIXELS_PER_UNIT).round() as u32;
        let at = ((y * baked.width + x) * 4) as usize;
        [0, 1, 2].map(|c| baked.rgba[at + c])
    }

    // GFX-PAT-3.1, 3.2: the texture repeats every 1 / texScale world units,
    // by each pattern's own scale. With scale 0.1 a repeat is 10 units: the
    // left half of the texture covers 0 to 5, the right half 5 to 10, and
    // it starts again at 10.
    #[test]
    fn texture_repeat_follows_the_pattern_scale() {
        let map = flat_map(12, 6);
        let textures = HashMap::from([(
            0x4545_FAC4,
            PatternTexture {
                texture: halves([250, 0, 0], [0, 0, 250]),
                scale: [0.1, 0.1],
                normal: None,
                normal_levels: Vec::new(),
                normal_scale: [1.0, 1.0],
                kind: 0,
                priority: 0,
                specular: [0.0, 1.0],
                transition: 0,
            },
        )]);
        let baked = bake_terrain(&map, &textures, &HashMap::new());
        let red = |p: [u8; 3]| p[0] > 100 && p[2] < 60;
        let blue = |p: [u8; 3]| p[2] > 100 && p[0] < 60;
        let v = 2.0 * ROW_SPACING;
        // The coordinate is the world position times the scale: u = 14.5
        // is 1.45 repeats (left half), 19.5 is 1.95 (right half), 24.5 is
        // 2.45 (left half again).
        assert!(red(pixel(&baked, 14.5, v)), "{:?}", pixel(&baked, 14.5, v));
        assert!(blue(pixel(&baked, 19.5, v)));
        assert!(red(pixel(&baked, 24.5, v)));
        // A different scale changes the repeat: 0.2 gives 5 units.
        let textures = HashMap::from([(
            0x4545_FAC4,
            PatternTexture {
                texture: halves([250, 0, 0], [0, 0, 250]),
                scale: [0.2, 0.2],
                normal: None,
                normal_levels: Vec::new(),
                normal_scale: [1.0, 1.0],
                kind: 0,
                priority: 0,
                specular: [0.0, 1.0],
                transition: 0,
            },
        )]);
        let baked = bake_terrain(&map, &textures, &HashMap::new());
        assert!(
            red(pixel(&baked, 16.25, v)),
            "{:?}",
            pixel(&baked, 16.25, v)
        );
        assert!(blue(pixel(&baked, 18.75, v)));
        assert!(red(pixel(&baked, 21.25, v)));
    }

    // GFX-WATER T6 to T9: the haze at a few depths (Default style).
    // spec: GFX-WATER-2.2 (GFX-WATER T6 to T9)
    #[test]
    fn depth_haze_matches_the_spec_values() {
        let close = |a: f32, b: f32| (a - b).abs() < 5e-4;
        // T6: at and just below the surface there is (almost) no haze.
        assert_eq!(depth_haze(0), None);
        assert_eq!(depth_haze(-50), None);
        let (_, w) = depth_haze(-80).unwrap();
        assert!(close(w, 0.9935), "{w}");
        // T7: -0.2 is limited to 7% haze.
        let (c, w) = depth_haze(-200).unwrap();
        assert!(close(w, 0.93), "{w}");
        assert!(
            close(c[0], 0.0) && close(c[1], 0.744) && close(c[2], 0.2825),
            "{c:?}"
        );
        // T8: -3.55.
        let (c, w) = depth_haze(-3550).unwrap();
        assert!(close(w, 0.3141), "{w}");
        assert!(close(c[1], 0.2513) && close(c[2], 0.1285), "{c:?}");
        // T9: -7.05 and deeper is black, the ground invisible; -7.0 is near
        // the deep colour.
        for h in [-7050, -9000, -20000] {
            assert_eq!(depth_haze(h), Some(([0.0; 3], 0.0)));
        }
        let (c, w) = depth_haze(-7000).unwrap();
        assert!(close(w, 0.0301), "{w}");
        assert!(
            close(c[0], 0.0) && close(c[1], 0.0240) && close(c[2], 0.0575),
            "{c:?}"
        );
    }

    // Deep water shows the deep colour whatever the seabed texture, shallow
    // water the seabed through a green haze; land is untouched.
    // spec: GFX-WATER-2.2, GFX-WATER-2.3
    #[test]
    fn water_is_the_depth_haze_over_the_seabed() {
        let textures = HashMap::from([(
            0x4545_FAC4,
            PatternTexture {
                texture: solid([200, 40, 40]),
                scale: [0.1, 0.1],
                normal: None,
                normal_levels: Vec::new(),
                normal_scale: [1.0, 1.0],
                kind: 0,
                priority: 0,
                specular: [0.0, 1.0],
                transition: 0,
            },
        )]);
        let centre = |height: i32| {
            let mut map = flat_map(8, 8);
            map.heights.samples.iter_mut().for_each(|h| *h = height);
            map.states.iter_mut().for_each(|s| *s |= state::WATER);
            let baked = bake_terrain(&map, &textures, &HashMap::new());
            let at = ((baked.height / 2 * baked.width + baked.width / 2) * 4) as usize;
            [0, 1, 2].map(|c| baked.rgba[at + c])
        };
        let deep = centre(-8000);
        // GFX-WATER T9: below -7.05 the ground is black.
        assert!(deep.iter().all(|&c| c < 3), "{deep:?}");
        let shallow = centre(-2000);
        let land = centre(500);
        assert!(
            land[0] > 150 && land[1] < 120,
            "land keeps its texture {land:?}"
        );
        // The red seabed fades and the green shallow colour comes through.
        assert!(
            shallow[0] < land[0] && shallow[1] > land[1],
            "{shallow:?} {land:?}"
        );
    }

    // The texture coordinates of the textured mesh land inside the image and
    // the mesh has the same lattice as the flat one.
    // spec: GFX-PAT-3.1
    #[test]
    fn textured_mesh_has_uvs_in_range() {
        let map = flat_map(7, 5);
        let flat = terrain_mesh(&map, &HashMap::new());
        let mesh = textured_terrain_mesh(&map);
        assert_eq!(mesh.indices, flat.indices);
        assert_eq!(mesh.uvs.len(), mesh.positions.len());
        assert!(mesh
            .uvs
            .iter()
            .all(|&[u, v]| (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)));
        // North (larger v) is the top of the image: smaller image v.
        let south = mesh.uvs[0][1];
        let north = mesh.uvs[(4 * 7) as usize][1];
        assert!(north < south);
    }
}
