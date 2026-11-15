//! The AI's resource map (AI-CON-1.1): one record per 4 x 4 square with, per
//! good, kind (1 underground, 2 surface) and landmass (the stored continent
//! index of the fields holding it, AI-CON-3.5, 1.1b), the amount. Every field
//! counts, explored or not (AI-CON-1.1c). Three squares are refreshed per
//! tick, after the think; and the cell-coordinate map (16 x 16 fields) keeps
//! per coordinate the sums of its squares as they were at its own refresh,
//! one coordinate every 16 counts (AI-CON-1.1c, T52).

use s2_sim::grid::Point;
use s2_sim::{DepositState, PlayerId, World};

use crate::config::AiConfig;

/// AI-CON-1.1: square size in fields.
pub const SQUARE: u32 = 4;
/// AI-CON-1.1c (T52): squares refreshed per tick, the map's own count (3 in
/// every recording and save read).
const REFRESH_PER_TICK: u32 = 3;
/// AI-CON-1.1c (T52): one cell coordinate is refreshed every 16 counts.
const COORDINATE_PERIOD: u64 = 16;

/// A square by its coordinates (field coordinates / 4).
pub type Square = (u32, u32);

pub fn square_of(at: Point) -> Square {
    (at.0 / SQUARE, at.1 / SQUARE)
}

/// The squares of a cell in row order.
pub fn squares_of(cell: crate::cells::CellPos) -> impl Iterator<Item = Square> {
    let (x0, y0) = crate::cells::origin(cell);
    let per_side = crate::cells::CELL / SQUARE;
    (0..per_side)
        .flat_map(move |sy| (0..per_side).map(move |sx| (x0 / SQUARE + sx, y0 / SQUARE + sy)))
}

/// The top-left field of a square.
pub fn square_origin(square: Square) -> Point {
    (square.0 * SQUARE, square.1 * SQUARE)
}

/// Kind of a resource: underground ore, stone; or on the surface (trees,
/// rocks, fish).
pub const UNDERGROUND: u8 = 1;
pub const SURFACE: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub good: u32,
    pub kind: u8,
    /// AI-CON-3.5: the stored continent index of the fields it counts.
    pub landmass: i32,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceMap {
    pub cols: u32,
    pub rows: u32,
    squares: Vec<Vec<Entry>>,
    cursor: u32,
    /// AI-CON-1.1c (T52): the cell-coordinate map, per coordinate (x
    /// fastest) the entries of its squares summed when it was refreshed.
    coordinates: Vec<Vec<Entry>>,
    coordinate_cursor: u32,
}

impl ResourceMap {
    pub fn new(width: u32, height: u32) -> Self {
        let (cols, rows) = (width.div_ceil(SQUARE), height.div_ceil(SQUARE));
        let (ccols, crows) = (
            width.div_ceil(crate::cells::CELL),
            height.div_ceil(crate::cells::CELL),
        );
        Self {
            cols,
            rows,
            squares: vec![Vec::new(); (cols * rows) as usize],
            cursor: 0,
            coordinates: vec![Vec::new(); (ccols * crows) as usize],
            coordinate_cursor: 0,
        }
    }

    pub fn clear(&mut self) {
        for s in &mut self.squares {
            s.clear();
        }
        self.cursor = 0;
        for c in &mut self.coordinates {
            c.clear();
        }
        self.coordinate_cursor = 0;
    }

    /// The cell coordinates per row of the map.
    fn coordinate_cols(&self) -> u32 {
        (self.cols * SQUARE).div_ceil(crate::cells::CELL)
    }

    /// AI-CON-1.1c (T52): the entries of the cell coordinate `cell` as its
    /// last refresh summed them (per good, kind and stored landmass).
    pub fn coordinate(&self, cell: crate::cells::CellPos) -> &[Entry] {
        let ccols = self.coordinate_cols();
        let index = (cell.1 * ccols + cell.0) as usize;
        if cell.0 >= ccols {
            return &[];
        }
        self.coordinates.get(index).map_or(&[], Vec::as_slice)
    }

    /// AI-CON-1.1c: the coordinate `index` (x fastest) summed from its
    /// squares as they are now.
    fn refresh_coordinate(&mut self, index: u32) {
        let ccols = self.coordinate_cols();
        let cell = (index % ccols, index / ccols);
        let mut entries: Vec<Entry> = Vec::new();
        for square in squares_of(cell) {
            for e in self.entries(square) {
                match entries
                    .iter_mut()
                    .find(|x| (x.good, x.kind, x.landmass) == (e.good, e.kind, e.landmass))
                {
                    Some(x) => x.amount += e.amount,
                    None => entries.push(*e),
                }
            }
        }
        entries.sort_by_key(|e| (e.good, e.kind, e.landmass));
        self.coordinates[index as usize] = entries;
    }

    /// AI-CON-1.1c (T52): after the think of the AI counter `c`, the next
    /// three squares and, when `c` is a multiple of 16, the next cell
    /// coordinate from its squares as they are then.
    pub fn refresh_after_think(
        &mut self,
        world: &World,
        player: PlayerId,
        config: &AiConfig,
        c: u64,
    ) {
        self.refresh_next(world, player, config);
        if c.is_multiple_of(COORDINATE_PERIOD) && !self.coordinates.is_empty() {
            let index = self.coordinate_cursor;
            self.coordinate_cursor = (index + 1) % self.coordinates.len() as u32;
            self.refresh_coordinate(index);
        }
    }

    pub fn entries(&self, square: Square) -> &[Entry] {
        if square.0 >= self.cols || square.1 >= self.rows {
            return &[];
        }
        &self.squares[(square.1 * self.cols + square.0) as usize]
    }

    /// The amount of `good` of `kind` in the square, on every landmass.
    pub fn amount(&self, square: Square, good: u32, kind: u8) -> u32 {
        self.entries(square)
            .iter()
            .filter(|e| e.good == good && e.kind == kind)
            .map(|e| e.amount)
            .sum()
    }

    /// Set the entries of a square (tests), and its cell coordinate's from
    /// the squares as they are then, as a refresh would.
    #[cfg(test)]
    pub(crate) fn set_entries(&mut self, square: Square, entries: Vec<Entry>) {
        self.squares[(square.1 * self.cols + square.0) as usize] = entries;
        let (cx, cy) = (
            square.0 * SQUARE / crate::cells::CELL,
            square.1 * SQUARE / crate::cells::CELL,
        );
        self.refresh_coordinate(cy * self.coordinate_cols() + cx);
    }

    /// All squares in square order (row by row).
    pub fn all_squares(&self) -> impl Iterator<Item = Square> {
        let cols = self.cols;
        (0..self.cols * self.rows).map(move |i| (i % cols, i / cols))
    }

    /// AI-2.3 resource system: refresh the next few squares, cycling.
    pub fn refresh_next(&mut self, world: &World, player: PlayerId, config: &AiConfig) {
        let total = self.cols * self.rows;
        for _ in 0..REFRESH_PER_TICK.min(total) {
            let index = self.cursor;
            self.cursor = (self.cursor + 1) % total;
            self.refresh(
                (index % self.cols, index / self.cols),
                world,
                player,
                config,
            );
        }
    }

    pub fn refresh_all(&mut self, world: &World, player: PlayerId, config: &AiConfig) {
        for index in 0..self.cols * self.rows {
            self.refresh(
                (index % self.cols, index / self.cols),
                world,
                player,
                config,
            );
        }
        for index in 0..self.coordinates.len() as u32 {
            self.refresh_coordinate(index);
        }
    }

    /// AI-CON-1.1c: per field of the square, explored or not, a deposit
    /// gives a surface entry of its good; a field without one its
    /// underground resource (water excepted).
    fn refresh(&mut self, square: Square, world: &World, _player: PlayerId, config: &AiConfig) {
        let terrain = world.terrain();
        let (x0, y0) = square_origin(square);
        let mut entries: Vec<Entry> = Vec::new();
        let mut add = |good: u32, kind: u8, landmass: i32, amount: u32| match entries
            .iter_mut()
            .find(|e| e.good == good && e.kind == kind && e.landmass == landmass)
        {
            Some(e) => e.amount += amount,
            None => entries.push(Entry {
                good,
                kind,
                landmass,
                amount,
            }),
        };
        for y in y0..(y0 + SQUARE).min(terrain.height) {
            for x in x0..(x0 + SQUARE).min(terrain.width) {
                let f = (x, y);
                let landmass = world.continent_of(f).unwrap_or(-1);
                if let Some(d) = world.deposit(f) {
                    if matches!(d.state, DepositState::Growing { .. }) {
                        if let Some(kind) = world.deposit_kind(d.kind) {
                            if let Some(good) = kind.good {
                                add(good, SURFACE, landmass, 1 + next_links(world, kind.next));
                            }
                        }
                    }
                } else if let Some(r) = world.resource_at(f) {
                    if r.amount > 0 && r.good != config.names.water {
                        // AI-CON-1.4: fish lies in the resource map as an
                        // underground good.
                        add(r.good, UNDERGROUND, landmass, u32::from(r.amount));
                    }
                }
            }
        }
        entries.sort_by_key(|e| (e.good, e.kind, e.landmass));
        self.squares[(square.1 * self.cols + square.0) as usize] = entries;
    }
}

/// AI-CON-1.1c (T57): the next-deposit links (`dp_nextDeposit`) from a
/// deposit type followed to the end: a stone pile of six has five, a tree
/// none.
fn next_links(world: &World, mut next: Option<u32>) -> u32 {
    let mut links = 0;
    while let Some(id) = next {
        links += 1;
        // A chain that came back to itself (no rules do that) ends here.
        if links > 64 {
            break;
        }
        next = world.deposit_kind(id).and_then(|k| k.next);
    }
    links
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_struct!(ResourceMap {
    cols,
    rows,
    squares,
    cursor,
    coordinates,
    coordinate_cursor
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;

    const P0: PlayerId = PlayerId(0);

    // AI-CON-1.1c (AI-CON T52): three squares are refreshed after each
    // think; one cell coordinate every 16 counts, by its own cursor (x
    // fastest), summed from its squares as they are then.
    // spec: AI-CON-1.1c (AI-CON T52)
    #[test]
    fn t52_three_squares_a_tick_and_a_coordinate_every_16_counts() {
        let world = world(&[0]);
        let config = config();
        let mut map = ResourceMap::new(96, 96);
        for c in 1..16 {
            map.refresh_after_think(&world, P0, &config, c);
        }
        assert_eq!((map.cursor, map.coordinate_cursor), (45, 0));
        map.refresh_after_think(&world, P0, &config, 16);
        assert_eq!((map.cursor, map.coordinate_cursor), (48, 1));
        let mut sum: Vec<Entry> = Vec::new();
        for square in squares_of((0, 0)) {
            for e in map.entries(square) {
                match sum
                    .iter_mut()
                    .find(|x| (x.good, x.kind, x.landmass) == (e.good, e.kind, e.landmass))
                {
                    Some(x) => x.amount += e.amount,
                    None => sum.push(*e),
                }
            }
        }
        sum.sort_by_key(|e| (e.good, e.kind, e.landmass));
        assert_eq!(map.coordinate((0, 0)), sum.as_slice());
        // Until its next turn a coordinate keeps what it held.
        assert!(map.coordinate((1, 0)).is_empty());
        for c in 17..32 {
            map.refresh_after_think(&world, P0, &config, c);
        }
        assert_eq!(map.coordinate_cursor, 1);
    }
}
