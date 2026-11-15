//! The 16 x 16 cell system (AI-CON-1.1, 1.6): the AI's planning grid.

use std::collections::BTreeMap;

use s2_sim::grid::{self, Point};
use s2_sim::{BuildingSize, PlayerId, World};

/// AI-CON-1.1: cell size in fields.
pub const CELL: u32 = 16;

/// A cell by cell coordinates (field coordinates / 16).
pub type CellPos = (u32, u32);

/// The cell a field is in.
pub fn cell_of(at: Point) -> CellPos {
    (at.0 / CELL, at.1 / CELL)
}

/// AI-CON-1.1: the cell's position is its top-left field.
pub fn origin(cell: CellPos) -> Point {
    (cell.0 * CELL, cell.1 * CELL)
}

/// The field in the middle of a cell, used as the start of site searches and
/// for the distances between cells (AI-CON-3.3, 5.4).
pub fn centre(cell: CellPos) -> Point {
    (cell.0 * CELL + CELL / 2, cell.1 * CELL + CELL / 2)
}

/// AI-CON-1.6, AI-MISC-1.1 (T53): a "full" mark on a good: no site was
/// found for a producer of it. It keeps the game time it was set and how
/// often it has expired; the first lasts until the next check, the second
/// 600 s, then 1200 s, ...
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FullMark {
    /// Game seconds at which it was set.
    pub time: f32,
    /// How often it has expired (never reset).
    pub count: u32,
    /// In force; cleared by the 37-tick check once time + count x 600 has
    /// passed.
    pub active: bool,
}

/// One cell of the AI player.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub pos: CellPos,
    /// The player's buildings in the cell, in the order they were announced.
    pub buildings: Vec<Point>,
    /// AI-CON-7.1, 7.2: the forester and the hunter of the cell.
    pub forester: Option<Point>,
    pub hunter: Option<Point>,
    /// AI-CON-1.6 "full" marks by good.
    pub full: BTreeMap<u32, FullMark>,
    /// AI-CON-1.6 timed markers (the tick they were set).
    pub depot_wanted: Option<u64>,
    /// The tick and the weight (100 if the cell had a harbour and no
    /// military building when the marker was set, else 1).
    pub crowded: Option<(u64, u32)>,
    /// Set by the military part (`AI-military.md`): the tick and the size
    /// of building to tear down (AI-CON-9.4).
    pub clearing: Option<(u64, BuildingSize)>,
    pub harbour_wanted: Option<u64>,
    /// AI-CON-1.1a: the cell's position, the start of the site search's
    /// routes; set when the validity scan makes the cell or by its first
    /// building.
    pub position: Option<Point>,
    /// AI-CON-1.1b (b): the cell's landmass, the stored continent index at
    /// the first position it gets; -1 while unset, and set again by a new
    /// position while it is -1.
    pub landmass: i32,
}

impl Cell {
    fn new(pos: CellPos, now: u64) -> Self {
        Self {
            pos,
            buildings: Vec::new(),
            forester: None,
            hunter: None,
            full: BTreeMap::new(),
            // AI-CON-1.6: set when a cell is created.
            depot_wanted: Some(now),
            crowded: None,
            clearing: None,
            harbour_wanted: None,
            position: None,
            landmass: -1,
        }
    }

    /// AI-CON-1.1a: the cell gets the position `at`; AI-CON-1.1b (b): its
    /// landmass follows only while it is unset (-1). AI-CON-1.6: when the
    /// landmass is set, a position of that land continent object (a harbour
    /// spot) inside the cell sets the harbour-wanted marker in tick `now`,
    /// unless the cell has a harbour or the marker is already set.
    fn set_position(&mut self, world: &World, at: Point, now: u64, has_harbour: bool) {
        self.position = Some(at);
        if self.landmass != -1 {
            return;
        }
        self.landmass = world.continent_of(at).unwrap_or(-1);
        let spot_here = world
            .land_spots(self.landmass)
            .iter()
            .any(|&spot| cell_of(spot) == self.pos);
        if spot_here && !has_harbour && self.harbour_wanted.is_none() {
            self.harbour_wanted = Some(now);
        }
    }

    /// AI-CON-1.1a: a building added to the cell sets the position to its
    /// field when that is strictly nearer to the cell's centre than the
    /// position (an unset position counts as far); `has_harbour`: the cell
    /// has a harbour, this building included.
    pub fn building_added(&mut self, world: &World, at: Point, now: u64, has_harbour: bool) {
        let c = centre(self.pos);
        if self
            .position
            .is_none_or(|p| grid::distance(at, c) < grid::distance(p, c))
        {
            self.set_position(world, at, now, has_harbour);
        }
    }

    /// AI-CON-1.1a: the start of the site search's routes: the position, or
    /// the centre while it is unset.
    pub fn route_start(&self) -> Point {
        self.position.unwrap_or_else(|| centre(self.pos))
    }

    /// AI-CON-1.6: whether a "full" mark on `good` is in force.
    pub fn is_full(&self, good: u32) -> bool {
        self.full.get(&good).is_some_and(|m| m.active)
    }

    /// AI-CON-1.6 (T53): mark `good` full at game time `seconds`; a mark
    /// in force is left as it is (its time kept), one not in force is set
    /// from now with its count kept.
    pub fn mark_full(&mut self, good: u32, seconds: f32) {
        let mark = self.full.entry(good).or_default();
        if !mark.active {
            mark.active = true;
            mark.time = seconds;
        }
    }

    /// AI-CON-1.6 (T53): the 37-tick check clears a mark in force once its
    /// time + count x 600 s lies before now (strictly), and counts it.
    fn expire_full(&mut self, seconds: f32) {
        for mark in self.full.values_mut() {
            if mark.active && mark.time + 600.0 * (mark.count as f32) < seconds {
                mark.active = false;
                mark.count += 1;
            }
        }
    }
}

/// The player's cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Cells {
    pub cols: u32,
    pub rows: u32,
    cells: BTreeMap<CellPos, Cell>,
    /// AI-CON-7.1, 7.2: the cells in creation order (the player's list).
    order: Vec<CellPos>,
    /// AI-CON-1.1: the grid position the last scan looked at.
    cursor: u32,
    /// SIM-START-4.4 b, h: the cells made so far, each an object that
    /// takes a number from the simulation's counter: for a building, or by
    /// the scan (T10: (8, 2) at counter 149, (9, 2) at 154).
    made: u32,
}

impl Cells {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            cols: width.div_ceil(CELL),
            rows: height.div_ceil(CELL),
            cells: BTreeMap::new(),
            order: Vec::new(),
            cursor: 0,
            made: 0,
        }
    }

    /// FMT-SAV-3.10e: the cells of a save in their creation order, and the
    /// grid position the scan looked at last.
    pub(crate) fn restored(mut self, cells: Vec<Cell>, cursor: u32) -> Self {
        self.made = cells.len() as u32;
        self.order = cells.iter().map(|c| c.pos).collect();
        self.cells = cells.into_iter().map(|c| (c.pos, c)).collect();
        self.cursor = cursor;
        self
    }

    /// SIM-START-4.2: how many cells were made so far.
    pub fn made(&self) -> u32 {
        self.made
    }

    /// AI-CON-7.1, 7.2: the cells in creation order.
    pub fn in_order(&self) -> &[CellPos] {
        &self.order
    }

    fn insert(&mut self, pos: CellPos, now: u64) {
        if let std::collections::btree_map::Entry::Vacant(e) = self.cells.entry(pos) {
            e.insert(Cell::new(pos, now));
            self.order.push(pos);
            self.made = self.made.wrapping_add(1);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn get(&self, pos: CellPos) -> Option<&Cell> {
        self.cells.get(&pos)
    }

    pub fn get_mut(&mut self, pos: CellPos) -> Option<&mut Cell> {
        self.cells.get_mut(&pos)
    }

    /// The cells in cell order (row by row, as a grid pass finds them).
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        let mut cells: Vec<&Cell> = self.cells.values().collect();
        cells.sort_by_key(|c| (c.pos.1, c.pos.0));
        cells.into_iter()
    }

    /// Positions in cell order.
    pub fn positions(&self) -> Vec<CellPos> {
        self.iter().map(|c| c.pos).collect()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        self.cells.values_mut()
    }

    /// The cell `pos`, created (with its depot-wanted marker) if needed.
    pub fn ensure(&mut self, pos: CellPos, now: u64) -> &mut Cell {
        self.insert(pos, now);
        self.cells.get_mut(&pos).expect("just made")
    }

    pub fn remove(&mut self, pos: CellPos) {
        self.cells.remove(&pos);
        self.order.retain(|&p| p != pos);
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.order.clear();
        self.cursor = 0;
    }

    /// AI-CON-1.6: every cell's check of its "full" marks.
    pub fn expire_full(&mut self, seconds: f32) {
        for cell in self.cells.values_mut() {
            cell.expire_full(seconds);
        }
    }

    /// AI-CON-1.1: one grid position per call, the cursor advanced before
    /// the check (from (0, 0), so the first check is (1, 0)), cycling: the
    /// cell exists as soon as one of the four corner fields of its square is
    /// the player's, and goes when it has no buildings, depots or forester
    /// and no corner is owned.
    pub fn scan(&mut self, world: &World, player: PlayerId, now: u64) {
        let total = self.cols * self.rows;
        if total == 0 {
            return;
        }
        self.cursor = (self.cursor + 1) % total;
        let pos = (self.cursor % self.cols, self.cursor / self.cols);
        let owned = corner_owned(world, player, pos);
        match self.cells.get(&pos) {
            // AI-CON-1.1a: no cell when the border scan finds no field.
            None if owned => {
                if let Some(at) = self.border_position(world, player, pos) {
                    self.insert(pos, now);
                    if let Some(cell) = self.cells.get_mut(&pos) {
                        cell.set_position(world, at, now, false);
                    }
                }
            }
            Some(cell) if !owned && cell.buildings.is_empty() && cell.forester.is_none() => {
                self.remove(pos);
            }
            _ => {}
        }
    }
}

impl Cells {
    /// AI-CON-1.1a: the position of a cell made by the validity scan: the
    /// first field of its square's border (the top row left to right, the
    /// bottom row, the left column top to bottom, the right column) that the
    /// player owns and whose stored continent index equals the index at a
    /// neighbouring cell's position (AI-CON-1.1b; cells cx - 1 .. cx + 1
    /// outer, cy - 1 .. cy + 1 inner, existing ones with cx, cy > 0).
    fn border_position(&self, world: &World, player: PlayerId, pos: CellPos) -> Option<Point> {
        let (x0, y0) = origin(pos);
        let last = CELL - 1;
        let border: Vec<Point> = (0..CELL)
            .map(|i| (x0 + i, y0))
            .chain((0..CELL).map(|i| (x0 + i, y0 + last)))
            .chain((0..CELL).map(|i| (x0, y0 + i)))
            .chain((0..CELL).map(|i| (x0 + last, y0 + i)))
            .collect();
        let mut regions = Vec::new();
        for cx in pos.0.saturating_sub(1)..=pos.0 + 1 {
            for cy in pos.1.saturating_sub(1)..=pos.1 + 1 {
                if (cx, cy) == pos || cx == 0 || cy == 0 {
                    continue;
                }
                if let Some(p) = self.cells.get(&(cx, cy)).and_then(|c| c.position) {
                    regions.push(world.continent_of(p));
                }
            }
        }
        regions.iter().find_map(|&region| {
            border
                .iter()
                .copied()
                .find(|&f| world.field_owner(f) == Some(player) && world.continent_of(f) == region)
        })
    }
}

/// AI-CON-1.1 (T82): one of the cell's four corner fields is owned by
/// `player`: its origin and the origins of the next cells to the right and
/// below, (16 cx + 16, 16 cy), (16 cx, 16 cy + 16), (16 cx + 16, 16 cy + 16),
/// not its square's own last row and column (corner fields outside the map
/// are not).
pub fn corner_owned(world: &World, player: PlayerId, pos: CellPos) -> bool {
    let (x, y) = origin(pos);
    [(x, y), (x + CELL, y), (x, y + CELL), (x + CELL, y + CELL)]
        .into_iter()
        .any(|f| world.field_owner(f) == Some(player))
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_struct!(Cells {
    cols,
    rows,
    cells,
    order,
    cursor,
    made
});

#[cfg(test)]
mod tests {
    use super::*;

    // spec: AI-CON-1.1
    #[test]
    fn cells_are_16_fields_wide() {
        assert_eq!(cell_of((0, 0)), (0, 0));
        assert_eq!(cell_of((15, 15)), (0, 0));
        assert_eq!(cell_of((16, 31)), (1, 1));
        assert_eq!(origin((2, 3)), (32, 48));
        assert_eq!(Cells::new(100, 33).cols, 7);
        assert_eq!(Cells::new(100, 33).rows, 3);
    }

    // AI-CON-1.6, AI-MISC-1.1 (AI-CON T53): the first mark lasts until the
    // next check, the second 600 s, the third 1200 s; setting a mark in
    // force changes nothing; the count is never reset.
    // spec: AI-CON-1.6 (AI-CON T53), AI-MISC-1.1
    #[test]
    fn full_marks_last_until_the_check_then_600_then_1200_seconds() {
        let mut cell = Cell::new((0, 0), 0);
        assert!(!cell.is_full(5));
        cell.mark_full(5, 677.18);
        cell.expire_full(677.18);
        assert!(cell.is_full(5), "not before now");
        cell.expire_full(680.88);
        assert!(!cell.is_full(5), "the first until the next check");
        cell.mark_full(5, 902.13);
        cell.mark_full(5, 950.0);
        cell.expire_full(1502.13);
        assert!(cell.is_full(5), "time kept: 902.13 + 600 is not before");
        cell.expire_full(1502.2);
        assert!(!cell.is_full(5));
        cell.mark_full(5, 2000.0);
        cell.expire_full(3199.9);
        assert!(cell.is_full(5));
        cell.expire_full(3200.1);
        assert!(!cell.is_full(5));
        assert_eq!(cell.full[&5].count, 3);
    }
}
