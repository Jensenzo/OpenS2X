//! Mesh data of the 3D terrain (`specs/GFX-world-view.md` GFX-VIEW-2, 5)
//! and the water (`specs/GFX-water.md`), as plain arrays in Bevy's frame
//! (see [`crate::render::world3d::to_bevy`]). No rendering types here.

use crate::render::terrain::depth_haze;
use crate::render::world3d::{grid_normal, to_bevy, Frame, Heights, HEIGHT_SCALE, WATER_SURFACE};

/// GFX-VIEW-2.1: fields per terrain cell along each axis.
pub const CELL_FIELDS: u32 = 4;
/// Samples per cell along each axis.
const CELL_SAMPLES: u32 = 4 * CELL_FIELDS;

/// One terrain cell: its samples with position, normal, tangent and depth
/// haze, and the lattice triangles between them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CellMesh {
    pub positions: Vec<[f32; 3]>,
    /// GFX-VIEW-2.1a: the decoded normal bytes, not renormalised.
    pub normals: Vec<[f32; 3]>,
    /// GFX-VIEW-2.1a (b): the decoded tangent bytes (w 1, Bevy's layout).
    pub tangents: Vec<[f32; 4]>,
    /// GFX-WATER-2.2: haze colour (display values) and the weight of the lit
    /// ground; white and 1 above the water.
    pub haze: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

/// GFX-VIEW-2.1: the number of terrain cells, ceil(width / 4) by
/// ceil(height / 4).
pub fn cell_count(width: u32, height: u32) -> (u32, u32) {
    (width.div_ceil(CELL_FIELDS), height.div_ceil(CELL_FIELDS))
}

/// The samples of cell `(cx, cy)`: columns and rows from its first to its
/// last, inclusive; the last cell of a row or column ends at the grid's
/// edge, so the cells cover the whole grid (MAP-GRID-4.6).
pub fn cell_samples(frame: &Frame, cx: u32, cy: u32) -> ([u32; 2], [u32; 2]) {
    let columns = [
        cx * CELL_SAMPLES,
        ((cx + 1) * CELL_SAMPLES).min(frame.columns - 1),
    ];
    let rows = [
        cy * CELL_SAMPLES,
        ((cy + 1) * CELL_SAMPLES).min(frame.rows - 1),
    ];
    (columns, rows)
}

/// GFX-VIEW-2.2: a unit vector stored as three unsigned bytes
/// trunc((v + 1) x 127.5).
pub fn normal_bytes(v: [f32; 3]) -> [u8; 3] {
    v.map(|x| ((x + 1.0) * 127.5).clamp(0.0, 255.0) as u8)
}

/// GFX-VIEW-2.2, 2.1a: stored bytes read back as b / 127.5 - 1, not
/// renormalised.
pub fn from_normal_bytes(b: [u8; 3]) -> [f32; 3] {
    b.map(|x| f32::from(x) / 127.5 - 1.0)
}

/// GFX-VIEW-2.1a: a terrain vertex's normal and tangent (game frame) from
/// its grid normal: (a) the grid normal's bytes decoded; (b) the game's +x
/// made perpendicular to that decoded normal n, normalize(n x (x x n)) =
/// normalize(x |n|^2 - (n . x) n), stored as bytes and decoded the same way.
/// The bitangent is not stored: (c) B = T x N in the game frame.
pub fn vertex_frame(grid: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let n = from_normal_bytes(normal_bytes(grid));
    let [nx, ny, nz] = n;
    let t = crate::render::world3d::normalize([ny * ny + nz * nz, -nx * ny, -nx * nz]);
    (n, from_normal_bytes(normal_bytes(t)))
}

/// GFX-VIEW-2.1b: the heights a terrain vertex is drawn from. Its position
/// follows the current heights; its normal and tangent (2.1a) are those of
/// the current heights only for samples with column <= the map's width in
/// fields and row <= its height in fields (the corner the original's refresh
/// after a height change covers, handed the field size for the sample
/// size); every other sample keeps the normal of the heights when the map
/// view was set up (`setup`, the game's start or load). Without `setup`
/// every normal is the current heights'.
pub struct TerrainHeights<'a, H: Heights, S: Heights> {
    pub current: &'a H,
    pub setup: Option<&'a S>,
}

#[cfg(test)]
impl<'a, H: Heights> TerrainHeights<'a, H, H> {
    /// Every normal from the current heights (no set-up heights kept).
    pub fn current(current: &'a H) -> Self {
        Self {
            current,
            setup: None,
        }
    }
}

impl<H: Heights, S: Heights> TerrainHeights<'_, H, S> {
    /// GFX-VIEW-2.1b: the grid normal sample `(c, r)`'s terrain normal is
    /// made from.
    pub fn grid_normal(&self, c: u32, r: u32) -> [f32; 3] {
        let frame = self.current.frame();
        let (width, height) = ((frame.columns - 1) / 4, (frame.rows - 1) / 4);
        let (c64, r64) = (i64::from(c), i64::from(r));
        match self.setup {
            Some(setup) if c > width || r > height => grid_normal(setup, c64, r64),
            _ => grid_normal(self.current, c64, r64),
        }
    }
}

/// The vertex of sample `(c, r)`: position, normal, tangent and haze.
fn sample_vertex<H: Heights, S: Heights>(
    heights: &TerrainHeights<'_, H, S>,
    c: u32,
    r: u32,
) -> ([f32; 3], [f32; 3], [f32; 4], [f32; 4]) {
    let frame = heights.current.frame();
    let [x, z] = frame.sample_xz(i64::from(c), i64::from(r));
    let raw = heights.current.raw(c, r);
    let y = raw as f32 * HEIGHT_SCALE;
    let (normal, tangent) = vertex_frame(heights.grid_normal(c, r));
    let [tx, ty, tz] = to_bevy(tangent);
    let haze = match depth_haze(raw) {
        Some(([red, green, blue], weight)) => [red, green, blue, weight],
        None => [1.0; 4],
    };
    (to_bevy([x, y, z]), to_bevy(normal), [tx, ty, tz, 1.0], haze)
}

/// GFX-VIEW-2.1, MAP-GRID-4.5: the mesh of one cell. Between sample rows
/// `r` and `r + 1`, for each column `c`: on even `r` the triangles
/// (c, r), (c+1, r), (c, r+1) and (c+1, r), (c+1, r+1), (c, r+1); on odd `r`
/// (c, r), (c+1, r+1), (c, r+1) and (c, r), (c+1, r), (c+1, r+1). Each is
/// wound counter-clockwise seen from above in Bevy's frame.
pub fn cell_mesh<H: Heights, S: Heights>(
    heights: &TerrainHeights<'_, H, S>,
    cx: u32,
    cy: u32,
) -> CellMesh {
    let frame = heights.current.frame();
    let ([c0, c1], [r0, r1]) = cell_samples(&frame, cx, cy);
    let mut mesh = CellMesh::default();
    for r in r0..=r1 {
        for c in c0..=c1 {
            let (position, normal, tangent, haze) = sample_vertex(heights, c, r);
            mesh.positions.push(position);
            mesh.normals.push(normal);
            mesh.tangents.push(tangent);
            mesh.haze.push(haze);
        }
    }
    let width = c1 - c0 + 1;
    let index = |c: u32, r: u32| (r - r0) * width + (c - c0);
    for r in r0..r1 {
        for c in c0..c1 {
            let (a, b) = (index(c, r), index(c + 1, r));
            let (d, e) = (index(c, r + 1), index(c + 1, r + 1));
            if r % 2 == 0 {
                mesh.indices.extend_from_slice(&[a, b, d, b, e, d]);
            } else {
                mesh.indices.extend_from_slice(&[a, e, d, a, b, e]);
            }
        }
    }
    mesh
}

/// GFX-VIEW-6.3 (corrected 2026-11-12 night, T44): the cap's cells along
/// each side.
pub const CAP_CELLS: u32 = 64;
/// GFX-VIEW-6.3: the cap's bend: y = -(x^2 + z^2) / 5010 below its centre.
const CAP_BEND: f32 = 5010.0;
/// GFX-VIEW-6.3: the radius of the sphere the cap is cut from.
const CAP_RADIUS: f32 = 5000.0;
/// GFX-VIEW-6.3: the cap's side, 2 sqrt(5010^2 - 5000^2) = 632.77.
pub fn cap_side() -> f32 {
    2.0 * (CAP_BEND * CAP_BEND - CAP_RADIUS * CAP_RADIUS).sqrt()
}
/// GFX-VIEW-6.3: the cap's centre over the camera's eye.
pub const CAP_LIFT: f32 = 15.0;
/// GFX-VIEW-6.3: the cloud texture's tiles along each side of the cap.
const CAP_TILES: f32 = 7.0;

/// GFX-VIEW-6.3: the `Skydome` entry's rotation (radians).
pub const SKY_ROTATION: f32 = std::f32::consts::FRAC_PI_2;
/// GFX-VIEW-6.3a: the `Stars` entry, the second skydome: its texture and
/// rotation (-45 degrees).
pub const STARS_FILE: &str = "data/enviroment/starfield.tga";
pub const STARS_ROTATION: f32 = -std::f32::consts::FRAC_PI_4;

/// GFX-VIEW-6.3: texture coordinates turned by `a`: u' = u cos a - v sin a,
/// v' = u sin a + v cos a (the sky shader, `sky.wesl`, turns them so).
#[cfg(test)]
pub fn sky_turn(u: f32, v: f32, a: f32) -> [f32; 2] {
    let (sin, cos) = a.sin_cos();
    [u * cos - v * sin, u * sin + v * cos]
}

/// GFX-VIEW-6.3 (corrected 2026-11-12 night, T44): the cloud cap around
/// the camera (Bevy frame, relative to the point 15 units above the eye): a
/// grid of 64 x 64 cells, vertex (i, j) at x = S i / 64 - S / 2, z = S j /
/// 64 - S / 2 (S the side of [`cap_side`]), bent down as y = -(x^2 + z^2) /
/// 5010; vertex colour white with alpha 1 - ((32.5 - i)^2 + (32.5 - j)^2) /
/// 65^2 (its peak half a cell toward +x and +z); texture coordinates u = 7 i
/// / 64 and v = 7 (1 - j / 64), not yet turned by a skydome entry's
/// rotation (the sky shader turns them, [`sky_turn`]). Positions, uvs,
/// colours, indices.
#[allow(clippy::type_complexity)]
pub fn cloud_cap() -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[f32; 4]>, Vec<u32>) {
    let n = CAP_CELLS;
    let side = cap_side();
    let (mut positions, mut uvs, mut colors, mut indices) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for j in 0..=n {
        for i in 0..=n {
            let x = side * i as f32 / n as f32 - side / 2.0;
            let z = side * j as f32 / n as f32 - side / 2.0;
            let y = -(x * x + z * z) / CAP_BEND;
            positions.push(to_bevy([x, y, z]));
            uvs.push([
                CAP_TILES * i as f32 / n as f32,
                CAP_TILES * (1.0 - j as f32 / n as f32),
            ]);
            let (di, dj) = (32.5 - i as f32, 32.5 - j as f32);
            let alpha = 1.0 - (di * di + dj * dj) / (65.0 * 65.0);
            colors.push([1.0, 1.0, 1.0, alpha.max(0.0)]);
        }
    }
    let row = n + 1;
    for j in 0..n {
        for i in 0..n {
            let a = j * row + i;
            indices.extend_from_slice(&[a, a + row, a + 1, a + 1, a + row, a + row + 1]);
        }
    }
    (positions, uvs, colors, indices)
}

/// GFX-WATER-3.1: the water cells, ceil((w - 1) / 4) by ceil((h - 1) / 4).
pub fn water_cell_count(width: u32, height: u32) -> (u32, u32) {
    (
        width.saturating_sub(1).div_ceil(CELL_FIELDS),
        height.saturating_sub(1).div_ceil(CELL_FIELDS),
    )
}

/// A water surface rectangle at the water level: x and z from `min` to
/// `max` in the game frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// GFX-WATER-3.6 (rewritten 2026-11-09, Q-1018): the height the water
/// surface is drawn at, the level of 3.3 (raw -100, world -0.1). Ground above
/// it (land, beaches at 0, shore fields between -100 and 0) hides it by the
/// depth test alone.
pub fn water_surface_y() -> f32 {
    WATER_SURFACE
}

/// GFX-WATER-3.1 to 3.3: one quad over every water cell whose lowest sample
/// lies below -100 raw. Cell `(cx, cy)` spans fields 4cx .. 4cx + 4 and
/// 4cy .. 4cy + 4 (clipped to the last field). Its quad (3.3, corrected
/// 2026-11-09, Q-1036) runs in x from the first field column's even-row
/// position to the end column's, and in z from the first field row to the
/// end row, so neighbouring quads abut and never overlap; a cell clipped at
/// the last field column reaches 1.5 units (a sample step) further for each
/// column cut off, one clipped at the last row is not extended.
pub fn water_quads(heights: &impl Heights, width: u32, height: u32) -> Vec<WaterQuad> {
    let frame = heights.frame();
    let (cells_x, cells_y) = water_cell_count(width, height);
    let field_sample = |x: u32, y: u32| (4 * x + if y % 2 == 1 { 2 } else { 0 }, 4 * y);
    let mut quads = Vec::new();
    for cy in 0..cells_y {
        for cx in 0..cells_x {
            let end_x = CELL_FIELDS * (cx + 1);
            let end_y = CELL_FIELDS * (cy + 1);
            let last_x = end_x.min(width - 1);
            let last_y = end_y.min(height - 1);
            let first = field_sample(CELL_FIELDS * cx, CELL_FIELDS * cy);
            let last = field_sample(last_x, last_y);
            let mut lowest = i32::MAX;
            for r in first.1..=last.1 {
                for c in first.0..=last.0 {
                    lowest = lowest.min(heights.raw(c, r));
                }
            }
            if lowest < WATER_QUAD_BELOW {
                quads.push(cell_box(&frame, cx, cy, width, height));
            }
        }
    }
    quads
}

/// GFX-WATER-3.1, 3.3: cell `(cx, cy)`'s box in x and z: from the first
/// field column's even-row position to the end column's (1.5 units further
/// for each column cut off at the last field), from the first field row to
/// the end row.
fn cell_box(frame: &Frame, cx: u32, cy: u32, width: u32, height: u32) -> WaterQuad {
    let end_x = CELL_FIELDS * (cx + 1);
    let last_x = end_x.min(width - 1);
    let last_y = (CELL_FIELDS * (cy + 1)).min(height - 1);
    let [x0, z0] = frame.field_xz(CELL_FIELDS * cx, CELL_FIELDS * cy);
    let [x1, _] = frame.field_xz(last_x, 0);
    let [_, z1] = frame.field_xz(0, last_y);
    let cut = end_x - last_x;
    let x1 = x1 + crate::render::world3d::SAMPLE_STEP * cut as f32;
    WaterQuad {
        min: [x0.min(x1), z0.min(z1)],
        max: [x0.max(x1), z0.max(z1)],
    }
}

/// GFX-WATER-3.2: a cell gets water when its lowest sample is below this.
const WATER_QUAD_BELOW: i32 = -100;

/// Two triangles of a rectangle at height `y`, as Bevy positions and
/// indices, facing up.
pub fn quad_mesh(quads: &[WaterQuad], y: f32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut positions = Vec::with_capacity(quads.len() * 4);
    let mut indices = Vec::with_capacity(quads.len() * 6);
    for quad in quads {
        let first = positions.len() as u32;
        let [x0, z0] = quad.min;
        let [x1, z1] = quad.max;
        for [x, z] in [[x0, z0], [x1, z0], [x1, z1], [x0, z1]] {
            positions.push(to_bevy([x, y, z]));
        }
        // Counter-clockwise from above in Bevy's frame, where z is flipped.
        indices.extend([0, 1, 2, 0, 2, 3].map(|i| first + i));
    }
    (positions, indices)
}

/// GFX-WATER-6.1: the map-edge skirt, 200 world units wide.
pub const SKIRT_WIDTH: f32 = 200.0;

/// GFX-WATER-6.1 (E-gfx-051): the unit the skirt's pieces reach into the
/// map, 2 x the sample step.
pub const SKIRT_UNIT: f32 = 3.0;

/// GFX-WATER-6.1 (read again 2026-11-11, E-gfx-051): the skirt around the
/// map, and the height of its ground, the lowest of the edge samples. With
/// each cell's box x0 .. x1, z0 .. z1 (the water cells of 3.1, 3.3: every
/// cell, water or not) and u = [`SKIRT_UNIT`]: each cell of the first cell
/// row x0 .. x1, z0 - 200 .. z0 + u (bit 1); of the last row x0 .. x1,
/// z1 - u .. z1 + 200 (bit 4); the first cell of each row x0 - 200 .. x0 +
/// 3u, z0 .. z1 (bit 8); the last cell of each row x1 - 4u .. x1 + 200,
/// z0 .. z1 (bit 2); and the corners: first cell x0 - 200 .. x0 + u,
/// z0 - 200 .. z0 + u (9); last of the first row x1 - u .. x1 + 200,
/// z0 - 200 .. z0 + u (3); first of the last row x0 - 200 .. x0 + u,
/// z1 - u .. z1 + 200 (12); last cell x1 - u .. x1 + 200, z1 - u .. z1 +
/// 200 + u (6). The pieces overlap the edge cells to close the seams.
pub fn skirt(heights: &impl Heights, width: u32, height: u32) -> (Vec<WaterQuad>, f32) {
    let frame = heights.frame();
    let (columns, rows) = (frame.columns, frame.rows);
    let mut lowest = i32::MAX;
    for c in 0..columns {
        lowest = lowest.min(heights.raw(c, 0)).min(heights.raw(c, rows - 1));
    }
    for r in 0..rows {
        lowest = lowest
            .min(heights.raw(0, r))
            .min(heights.raw(columns - 1, r));
    }
    let (w, u) = (SKIRT_WIDTH, SKIRT_UNIT);
    let (cells_x, cells_y) = water_cell_count(width, height);
    let piece = |x0: f32, x1: f32, z0: f32, z1: f32| WaterQuad {
        min: [x0, z0],
        max: [x1, z1],
    };
    let mut quads = Vec::new();
    if cells_x == 0 || cells_y == 0 {
        return (quads, lowest as f32 * HEIGHT_SCALE);
    }
    let cell = |cx: u32, cy: u32| cell_box(&frame, cx, cy, width, height);
    for cx in 0..cells_x {
        let top = cell(cx, 0);
        quads.push(piece(
            top.min[0],
            top.max[0],
            top.min[1] - w,
            top.min[1] + u,
        ));
        let bottom = cell(cx, cells_y - 1);
        quads.push(piece(
            bottom.min[0],
            bottom.max[0],
            bottom.max[1] - u,
            bottom.max[1] + w,
        ));
    }
    for cy in 0..cells_y {
        let first = cell(0, cy);
        quads.push(piece(
            first.min[0] - w,
            first.min[0] + 3.0 * u,
            first.min[1],
            first.max[1],
        ));
        let last = cell(cells_x - 1, cy);
        quads.push(piece(
            last.max[0] - 4.0 * u,
            last.max[0] + w,
            last.min[1],
            last.max[1],
        ));
    }
    let (first, last_top) = (cell(0, 0), cell(cells_x - 1, 0));
    let (first_bottom, last) = (cell(0, cells_y - 1), cell(cells_x - 1, cells_y - 1));
    quads.push(piece(
        first.min[0] - w,
        first.min[0] + u,
        first.min[1] - w,
        first.min[1] + u,
    ));
    quads.push(piece(
        last_top.max[0] - u,
        last_top.max[0] + w,
        last_top.min[1] - w,
        last_top.min[1] + u,
    ));
    quads.push(piece(
        first_bottom.min[0] - w,
        first_bottom.min[0] + u,
        first_bottom.max[1] - u,
        first_bottom.max[1] + w,
    ));
    quads.push(piece(
        last.max[0] - u,
        last.max[0] + w,
        last.max[1] - u,
        last.max[1] + w + u,
    ));
    (quads, lowest as f32 * HEIGHT_SCALE)
}

/// GFX-VIEW-5.1: the world texture: two texels per field across a row
/// (2W + 1 wide) and one per row, red = the field's visibility (255
/// explored, 0 not), green = the water depth term of GFX-WATER-4.4 (255
/// below the water level, 0 above).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTexture {
    pub width: u32,
    pub height: u32,
    /// Row by row from field row 0, two bytes per texel.
    pub texels: Vec<u8>,
}

/// GFX-VIEW-5.1: the world rectangle the world texture covers, x0, z0, x1,
/// z1 in the game frame: texel `2x + odd` of row `y` is centred on field
/// `(x, y)`, the texel after it half way to the next field.
pub fn world_texture_rect(frame: &Frame, width: u32, height: u32) -> [f32; 4] {
    let [fx, fz] = frame.field_xz(0, 0);
    let row = 4.0 * crate::render::world3d::SAMPLE_ROW_STEP;
    let x0 = fx - 1.5;
    let z0 = fz - row / 2.0;
    [
        x0,
        z0,
        x0 + 3.0 * (2 * width + 1) as f32,
        z0 + row * height as f32,
    ]
}

/// Build the world texture from the visibility and the depth byte of each
/// field. For the visibility a texel between two fields takes the brighter
/// of them; the depth byte (GFX-WATER-4.4) fills the field's texel pair, the
/// texel on the field and the one after it (odd rows shifted by one texel,
/// the extra texel closing a row taking its last field's byte, GFX-VIEW-5.1).
pub fn world_texture(
    width: u32,
    height: u32,
    explored: impl Fn(u32, u32) -> bool,
    depth: impl Fn(u32, u32) -> u8,
) -> WorldTexture {
    let texture_width = 2 * width + 1;
    let mut texels = vec![0u8; (texture_width * height * 2) as usize];
    for y in 0..height {
        let odd = y % 2;
        for t in 0..texture_width {
            let fields: Vec<u32> = if t < odd {
                vec![0]
            } else if (t - odd) % 2 == 0 {
                vec![((t - odd) / 2).min(width - 1)]
            } else {
                let left = (t - odd) / 2;
                vec![left.min(width - 1), (left + 1).min(width - 1)]
            };
            let at = ((y * texture_width + t) * 2) as usize;
            if fields.iter().any(|&x| explored(x, y)) {
                texels[at] = 255;
            }
            texels[at + 1] = depth((t.saturating_sub(odd) / 2).min(width - 1), y);
        }
    }
    WorldTexture {
        width: texture_width,
        height,
        texels,
    }
}

/// GFX-WATER-4.4 (corrected 2026-11-09, Q-1018): the depth byte of a field
/// of raw height `raw`: with t = clamp((level - h) / (level - ref), 0, 1),
/// level -100 and ref -3000, trunc(255 (1 - t)). 255 on all land and at the
/// waterline, 0 from -3000 down; the factor clamp(1.2 - byte / 255, 0, 1)
/// is 0.2 there and 1 from about -2420 on.
pub fn water_depth_byte(raw: i32) -> u8 {
    const LEVEL: f32 = -100.0;
    const REFERENCE: f32 = -3000.0;
    let t = ((raw as f32 - LEVEL) / (REFERENCE - LEVEL)).clamp(0.0, 1.0);
    (255.0 * (1.0 - t)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GFX-VIEW-6.3 (corrected), T44: the cap with the camera's eye at (0,
    /// 50, 0): positions (game frame, the eye's height added), alphas and
    /// texture coordinates before the turn.
    // spec: GFX-VIEW-6.3 (T44)
    #[test]
    fn t44_cloud_cap_shape() {
        let (positions, uvs, colors, indices) = cloud_cap();
        let side = CAP_CELLS as usize + 1;
        assert_eq!(side, 65);
        assert!((cap_side() - 632.77).abs() < 0.01, "{}", cap_side());
        assert_eq!(positions.len(), side * side);
        assert_eq!(uvs.len(), positions.len());
        assert_eq!(indices.len(), 6 * (side - 1) * (side - 1));
        let eye_y = 50.0 + CAP_LIFT;
        // Game frame: Bevy's z negated.
        let game = |k: usize| {
            let p = positions[k];
            [p[0], p[1] + eye_y, -p[2]]
        };
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|c| (a[c] - b[c]).abs() < 0.02);
        let at = |i: usize, j: usize| j * side + i;
        assert!(
            near(game(at(0, 0)), [-316.39, 25.04, -316.39]),
            "{:?}",
            game(0)
        );
        assert!((colors[at(0, 0)][3] - 0.500).abs() < 0.001);
        assert_eq!(uvs[at(0, 0)], [0.0, 7.0]);
        assert!(near(game(at(32, 32)), [0.0, 65.0, 0.0]));
        assert!((colors[at(32, 32)][3] - 0.99988).abs() < 0.0001);
        assert_eq!(uvs[at(32, 32)], [3.5, 3.5]);
        assert!(
            near(game(at(64, 32)), [316.39, 45.02, 0.0]),
            "{:?}",
            game(at(64, 32))
        );
        assert!((colors[at(64, 32)][3] - 0.765).abs() < 0.001);
        assert!((colors[at(32, 0)][3] - 0.750).abs() < 0.001);
        // At 90 degrees (u, v) -> (-v, u).
        let [u, v] = sky_turn(2.0, 3.0, SKY_ROTATION);
        assert!((u + 3.0).abs() < 1e-5 && (v - 2.0).abs() < 1e-5);
    }

    struct Grid {
        frame: Frame,
        height: Box<dyn Fn(u32, u32) -> i32>,
    }

    impl Heights for Grid {
        fn frame(&self) -> Frame {
            self.frame
        }
        fn raw(&self, c: u32, r: u32) -> i32 {
            (self.height)(c, r)
        }
    }

    fn flat(width: u32, height: u32, raw: i32) -> Grid {
        Grid {
            frame: Frame::new(width, height),
            height: Box::new(move |_, _| raw),
        }
    }

    // GFX-VIEW-2.1: 4 x 4 field cells; the last row and column of cells end
    // at the grid's edge, and together the cells cover every sample.
    #[test]
    fn cells_cover_the_grid() {
        assert_eq!(cell_count(80, 80), (20, 20));
        assert_eq!(cell_count(81, 6), (21, 2));
        let frame = Frame::new(6, 6);
        assert_eq!(cell_samples(&frame, 0, 0), ([0, 16], [0, 16]));
        assert_eq!(cell_samples(&frame, 1, 1), ([16, 24], [16, 24]));
        let mesh = cell_mesh(&TerrainHeights::current(&flat(6, 6, 0)), 1, 0);
        assert_eq!(mesh.positions.len(), 9 * 17);
        assert_eq!(mesh.indices.len(), 8 * 16 * 6);
    }

    // MAP-GRID-4.5: every triangle faces up in Bevy's frame.
    #[test]
    fn triangles_face_up() {
        let grid = Grid {
            frame: Frame::new(4, 4),
            height: Box::new(|c, r| ((c * 7 + r * 3) % 11) as i32 * 300),
        };
        let mesh = cell_mesh(&TerrainHeights::current(&grid), 0, 0);
        for t in mesh.indices.chunks(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            let u = [b[0] - a[0], b[2] - a[2]];
            let v = [c[0] - a[0], c[2] - a[2]];
            // y of (b - a) x (c - a).
            assert!(u[1] * v[0] - u[0] * v[1] > 0.0);
        }
    }

    // GFX-WATER T6, T7: haze per vertex; none on dry land.
    // spec: GFX-WATER-2.2
    #[test]
    fn vertex_haze() {
        let dry = cell_mesh(&TerrainHeights::current(&flat(4, 4, 500)), 0, 0);
        assert_eq!(dry.haze[0], [1.0; 4]);
        let wet = cell_mesh(&TerrainHeights::current(&flat(4, 4, -200)), 0, 0);
        let [r, g, b, w] = wet.haze[0];
        assert!((w - 0.93).abs() < 1e-4);
        assert!(r.abs() < 1e-6 && (g - 0.744).abs() < 1e-3 && (b - 0.2825).abs() < 1e-3);
    }

    // GFX-WATER T10: a cell whose lowest sample is exactly -100 gets no
    // quad; one below gets a quad from its first field to its end field
    // (3.3, corrected 2026-11-09): of a 9 x 9 map the first cell's from field
    // (0, 0) to field (4, 4)'s even-row position; the second column's cells
    // end on the last field column, none cut off.
    // spec: GFX-WATER-3.1, GFX-WATER-3.2, GFX-WATER-3.3
    #[test]
    fn water_quads_by_lowest_sample() {
        assert_eq!(water_cell_count(80, 80), (20, 20));
        assert!(water_quads(&flat(9, 9, -100), 9, 9).is_empty());
        let quads = water_quads(&flat(9, 9, -101), 9, 9);
        assert_eq!(quads.len(), 4);
        let frame = Frame::new(9, 9);
        let [x0, z0] = frame.field_xz(0, 0);
        assert_eq!(quads[0].min, [x0, z0]);
        let [x1, z1] = frame.field_xz(4, 4);
        assert_eq!(quads[0].max, [x1, z1]);
        let [x8, _] = frame.field_xz(8, 0);
        assert_eq!(quads[1].min[0], x1);
        assert_eq!(quads[1].max[0], x8);
    }

    // GFX-WATER T17 (3.3, Q-1036): Storm Coast's cells (6, 3) and (7, 3)
    // (170 x 170 fields): quads u -366 .. -342 and -342 .. -318, v
    // -379.31895 .. -358.53435: they share the edge u = -342 and do not
    // overlap. A cell clipped at the last field column (42: fields 168 ..
    // 172, three cut off) reaches 3 x 1.5 past field 169.
    // spec: GFX-WATER-3.3 (T17)
    #[test]
    fn t17_water_quads_abut() {
        let quads = water_quads(&flat(170, 170, -500), 170, 170);
        assert_eq!(quads.len(), 43 * 43);
        let quad = |cx: usize, cy: usize| quads[cy * 43 + cx];
        let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
        let (a, b) = (quad(6, 3), quad(7, 3));
        assert!(close(a.min[0], -366.0) && close(a.max[0], -342.0), "{a:?}");
        assert!(close(b.min[0], -342.0) && close(b.max[0], -318.0), "{b:?}");
        assert_eq!(a.max[0], b.min[0]);
        assert!(
            close(a.min[1], -379.31895) && close(a.max[1], -358.53435),
            "{a:?}"
        );
        assert_eq!((a.min[1], a.max[1]), (b.min[1], b.max[1]));
        assert_eq!(quad(6, 4).min[1], a.max[1]);
        let frame = Frame::new(170, 170);
        let [x169, _] = frame.field_xz(169, 0);
        assert!(close(quad(42, 0).max[0], x169 + 4.5));
        assert_eq!(quad(42, 42).max[1], frame.field_xz(0, 169)[1]);
    }

    // GFX-WATER T18 (4.4, Q-1018): the depth byte per field and its factor
    // 1.2 - byte / 255 (clamped): 0.2 on land and at the waterline, rising
    // with the depth, 1 from about -2420 on.
    // spec: GFX-WATER-4.4 (T18)
    #[test]
    fn t18_the_depth_byte() {
        let factor = |b: u8| (1.2f32 - f32::from(b) / 255.0).clamp(0.0, 1.0);
        let cases = [
            (0, 255, 0.2),
            (-100, 255, 0.2),
            (-101, 254, 0.204),
            (-500, 219, 0.341),
            (-1200, 158, 0.580),
            (-2200, 70, 0.925),
            (-3000, 0, 1.0),
            (-6000, 0, 1.0),
        ];
        for (h, byte, f) in cases {
            assert_eq!(water_depth_byte(h), byte, "{h}");
            assert!((factor(byte) - f).abs() < 1e-3, "{h}");
        }
        assert!((50..=51).contains(&water_depth_byte(-2420)));
        assert_eq!(factor(water_depth_byte(-2420)), 1.0);
    }

    // GFX-WATER T11, T12, T13 (3.6, 4.4, rewritten 2026-11-09): the depth
    // byte fills each field's texel pair in the world texture's green
    // channel (odd rows one texel on, the closing texel the row's last
    // field's): land at 0, -40 and a site levelled to -23 store 255 (factor
    // 0.2; the ground hides the surface there, T11, T12); between a field at
    // 0 and one at -1200 the linear filter gives about 0.39 (T13).
    // spec: GFX-WATER-3.6 (T11, T12, T13), GFX-WATER-4.4
    #[test]
    fn the_depth_bytes_in_the_world_texture() {
        let heights = [0, -40, -23, -1200, 5];
        let t = world_texture(
            5,
            2,
            |_, _| true,
            |x, _| water_depth_byte(heights[x as usize]),
        );
        let green: Vec<u8> = t.texels.chunks(2).map(|t| t[1]).collect();
        assert_eq!(
            &green[..11],
            &[255, 255, 255, 255, 255, 255, 158, 158, 255, 255, 255]
        );
        // Odd row: one texel on; texel 0 takes field 0's byte.
        assert_eq!(
            &green[11..],
            &[255, 255, 255, 255, 255, 255, 255, 158, 158, 255, 255]
        );
        let factor = |b: f32| (1.2f32 - b / 255.0).clamp(0.0, 1.0);
        assert!((factor((255.0 + 158.0) / 2.0) - 0.39).abs() < 0.01);
    }

    // GFX-WATER T11, T12, T14 (3.6, rewritten 2026-11-09): the surface lies
    // at -0.1 (raw -100): ground at 0 (a flat beach), at -40 and a site
    // levelled to -23 are above it and hide it; ground at -101 is below.
    // spec: GFX-WATER-3.3, GFX-WATER-3.6 (T11, T12, T14)
    #[test]
    fn the_surface_lies_at_the_water_rules_bound() {
        let y = water_surface_y();
        assert!((y + 0.1).abs() < 1e-6);
        for land in [0, -40, -23] {
            assert!(land as f32 * HEIGHT_SCALE > y);
        }
        assert!(-101.0 * HEIGHT_SCALE < y);
    }

    // GFX-WATER T2: tutorial01 has 400 water cells, 259 with a quad, (0, 0),
    // (1, 0) and (2, 0) among them. Skipped without S2_GAME_DIR.
    // spec: GFX-WATER-3.2
    #[test]
    fn tutorial01_water_cells() {
        let Ok(dir) = s2_assets::GameDir::from_env() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let path = dir.root().join("data/maps/tutorialMaps/tutorial01.s2m");
        let data = std::fs::read(&path).expect("tutorial01.s2m");
        let data = s2_assets::rc00::decode(&path.to_string_lossy(), &data).unwrap_or(data);
        let map = s2_assets::s2m::parse(&data).expect("a map");
        let terrain = map.terrain();
        let quads = water_quads(&terrain, terrain.width, terrain.height);
        assert_eq!(water_cell_count(terrain.width, terrain.height), (20, 20));
        assert_eq!(quads.len(), 259);
        let frame = terrain.frame();
        let [x, z] = frame.sample_xz(0, 0);
        assert_eq!(quads[0].min, [x, z]);
        let [x, _] = frame.field_xz(8, 0);
        assert_eq!(quads[2].min[0], x);
    }

    // GFX-WATER-6.1 (E-gfx-051): one piece per edge cell of each side and
    // one per corner, at the lowest edge height; each reaches u = 3.0 into
    // the map (the first column's 3u, the last column's 4u) and 200 out.
    // spec: GFX-WATER-6.1
    #[test]
    fn skirt_pieces() {
        let grid = Grid {
            frame: Frame::new(9, 9),
            height: Box::new(|c, _| -(c as i32) * 100),
        };
        let (pieces, y) = skirt(&grid, 9, 9);
        // 2 x 2 cells: 2 top, 2 bottom, 2 first-column, 2 last-column, 4
        // corners.
        assert_eq!(water_cell_count(9, 9), (2, 2));
        assert_eq!(pieces.len(), 12);
        let frame = &grid.frame;
        let first = cell_box(frame, 0, 0, 9, 9);
        let last = cell_box(frame, 1, 1, 9, 9);
        let close = |a: &WaterQuad, b: [f32; 4]| {
            [a.min[0], a.max[0], a.min[1], a.max[1]]
                .iter()
                .zip(b)
                .all(|(x, y)| (x - y).abs() < 1e-3)
        };
        let (w, u) = (SKIRT_WIDTH, SKIRT_UNIT);
        let [x0, z0] = [first.min[0], first.min[1]];
        let [x1, z1] = [last.max[0], last.max[1]];
        let want = [
            // The first cell row's first piece (bit 1).
            [x0, first.max[0], z0 - w, z0 + u],
            // The first cell of row 0 (bit 8), the last (bit 2).
            [x0 - w, x0 + 3.0 * u, z0, first.max[1]],
            [x1 - 4.0 * u, x1 + w, z0, first.max[1]],
            // The corners 9, 3, 12, 6.
            [x0 - w, x0 + u, z0 - w, z0 + u],
            [x1 - u, x1 + w, z0 - w, z0 + u],
            [x0 - w, x0 + u, z1 - u, z1 + w],
            [x1 - u, x1 + w, z1 - u, z1 + w + u],
        ];
        for b in want {
            assert!(pieces.iter().any(|p| close(p, b)), "{b:?} in {pieces:?}");
        }
        // The lowest edge sample: the last column's.
        let lowest = -100 * (grid.frame.columns as i32 - 1);
        assert!((y - lowest as f32 * HEIGHT_SCALE).abs() < 1e-4);
    }

    // GFX-VIEW-5.1, T9: explored fields are lit, unexplored black; the
    // texel between two fields is lit when either is.
    #[test]
    fn world_texture_texels() {
        let deep = |_, y| if y == 1 { 255 } else { 0 };
        let texture = world_texture(3, 2, |x, y| x == 1 && y == 0, deep);
        assert_eq!((texture.width, texture.height), (7, 2));
        let red: Vec<u8> = texture.texels.chunks(2).map(|t| t[0]).collect();
        // Row 0 (even): texels 0 (field 0), 1 (between 0 and 1), 2 (field 1),
        // 3 (between 1 and 2), 4 (field 2).
        assert_eq!(&red[..7], &[0, 255, 255, 255, 0, 0, 0]);
        // Row 1's depth bytes, unexplored.
        let green: Vec<u8> = texture.texels.chunks(2).map(|t| t[1]).collect();
        assert_eq!(&green[7..], &[255; 7]);
        assert!(red[7..].iter().all(|&r| r == 0));
        let frame = Frame::new(3, 2);
        let [x0, _, x1, _] = world_texture_rect(&frame, 3, 2);
        let [fx, _] = frame.field_xz(1, 0);
        // Field 1 of an even row is texel 2: centre at x0 + 7.5.
        assert!((fx - (x0 + 7.5)).abs() < 1e-4);
        assert!((x1 - x0 - 21.0).abs() < 1e-4);
    }

    fn close(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() <= eps)
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    // GFX-VIEW T38: the stored bytes of a grid normal, the decoded normal
    // (not renormalised) and the tangent made from it, stored the same way.
    // spec: GFX-VIEW-2.2, GFX-VIEW-2.1a
    #[test]
    fn t38_normal_and_tangent_bytes() {
        // The tangent before its bytes: +x made perpendicular to n.
        let tangent = |[nx, ny, nz]: [f32; 3]| {
            crate::render::world3d::normalize([ny * ny + nz * nz, -nx * ny, -nx * nz])
        };
        assert_eq!(normal_bytes([0.0, 1.0, 0.0]), [127, 255, 127]);
        let (n, t) = vertex_frame([0.0, 1.0, 0.0]);
        assert!(close(n, [-0.0039, 1.0, -0.0039], 1e-4));
        assert!(close(tangent(n), [1.0, 0.004, 0.0], 1e-4));
        let tb = normal_bytes(tangent(n));
        assert!(
            tb[0] >= 254 && (127..=128).contains(&tb[1]) && tb[2] == 127,
            "{tb:?}"
        );
        assert_eq!(t, from_normal_bytes(tb));

        assert_eq!(normal_bytes([0.6, 0.8, 0.0]), [204, 229, 127]);
        let (n, t) = vertex_frame([0.6, 0.8, 0.0]);
        assert!(close(n, [0.6, 0.7961, -0.0039], 1e-4));
        assert!(close(tangent(n), [0.7986, -0.6019, 0.0030], 1e-4));
        assert_eq!(normal_bytes(tangent(n)), [229, 50, 127]);
        assert_eq!(t, from_normal_bytes([229, 50, 127]));
    }

    // GFX-VIEW-2.1a (c): the bitangent is T x N in the game frame. The
    // terrain shader takes cross(n, t) of the vertex's Bevy-frame vectors
    // (z negated, a reflection), which is the game frame's T x N carried
    // into Bevy's: not N x T, so the normal map's second axis is not
    // flipped.
    // spec: GFX-VIEW-2.1a
    #[test]
    fn the_shaders_bitangent_is_t_cross_n() {
        for grid in [
            [0.0, 1.0, 0.0],
            [0.6, 0.8, 0.0],
            [0.2, 0.9, 0.39],
            [-0.3, 0.8, -0.52],
        ] {
            let (n, t) = vertex_frame(crate::render::world3d::normalize(grid));
            let game = cross(t, n);
            let shader = cross(to_bevy(n), to_bevy(t));
            assert!(close(shader, to_bevy(game), 1e-6), "{grid:?}");
        }
        // Flat ground: B is the game's +z (Bevy -z).
        let (n, t) = vertex_frame([0.0, 1.0, 0.0]);
        assert!(close(cross(t, n), [0.0, 0.0, 1.0], 0.01));
    }

    // GFX-VIEW T40, 2.1b: on a map of 170 x 170 fields (681 x 681 samples)
    // a height change in the refreshed corner (columns and rows 0 .. 170)
    // turns the terrain's normal; one at (540, 230) moves only the
    // position, the normal and tangent stay the set-up heights'.
    // spec: GFX-VIEW-2.1b
    #[test]
    fn t40_only_the_corner_takes_the_new_normals() {
        let frame = Frame::new(170, 170);
        assert_eq!((frame.columns, frame.rows), (681, 681));
        let setup = Grid {
            frame,
            height: Box::new(|_, _| 5000),
        };
        let bump = |c0: u32, r0: u32| Grid {
            frame,
            height: Box::new(move |c, r| if c == c0 + 1 && r == r0 { 5600 } else { 5000 }),
        };
        let flat = vertex_frame([0.0, 1.0, 0.0]);
        for (at, follows) in [((100, 100), true), ((540, 230), false), ((170, 170), true)] {
            let current = bump(at.0, at.1);
            let heights = TerrainHeights {
                current: &current,
                setup: Some(&setup),
            };
            let normal = vertex_frame(heights.grid_normal(at.0, at.1));
            assert_eq!(normal != flat, follows, "{at:?}");
            // The position always follows: the changed sample is drawn high.
            let (c, r) = (at.0 + 1, at.1);
            let (cx, cy) = (c / CELL_SAMPLES, r / CELL_SAMPLES);
            let mesh = cell_mesh(&heights, cx, cy);
            let ([c0, c1], [r0, _]) = cell_samples(&frame, cx, cy);
            let i = ((r - r0) * (c1 - c0 + 1) + (c - c0)) as usize;
            assert!((mesh.positions[i][1] - 5600.0 * HEIGHT_SCALE).abs() < 1e-4);
        }
        // A change at column 171: the normal there keeps the set-up heights';
        // column 170's, which reads it, takes it.
        let current = bump(170, 10);
        let heights = TerrainHeights {
            current: &current,
            setup: Some(&setup),
        };
        assert_eq!(vertex_frame(heights.grid_normal(171, 10)), flat);
        assert_ne!(vertex_frame(heights.grid_normal(170, 10)), flat);
    }
}
