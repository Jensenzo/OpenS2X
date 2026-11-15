//! The two coarse maps of 8 x 8-field cells (`AI-military.md` AI-MIL-1): the
//! per-player military map and the AI player's own territory map.

use s2_sim::grid::Point;
use s2_sim::{Building, PlayerId, World};

use crate::config::AiConfig;

/// AI-MIL-1.1: military map cell size in fields.
pub const MCELL: u32 = 8;

/// A cell of the 8 x 8 grids.
pub type MPos = (u32, u32);

pub fn mcell_of(at: Point) -> MPos {
    (at.0 / MCELL, at.1 / MCELL)
}

/// The top-left field of an 8 x 8 cell.
pub fn morigin(cell: MPos) -> Point {
    (cell.0 * MCELL, cell.1 * MCELL)
}

/// AI-MIL-1.1: what one player has in one cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MilCell {
    /// Sum of `bp_setAiThreat` of every building of the player in the cell,
    /// sites and unoccupied ones included (Q-159).
    pub threat: i64,
    /// The same for the buildings whose construction has progress: sites
    /// and buildings being upgraded.
    pub threat_building: i64,
    /// Sum of `bp_setAiScore` of the buildings.
    pub score: i64,
}

/// AI-MIL-1.1: one player's military map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilitaryMap {
    pub cols: u32,
    pub rows: u32,
    cells: Vec<MilCell>,
    /// The cell index refreshed next.
    cursor: u32,
}

impl MilitaryMap {
    pub fn new(width: u32, height: u32) -> Self {
        let (cols, rows) = (width.div_ceil(MCELL), height.div_ceil(MCELL));
        Self {
            cols,
            rows,
            cells: vec![MilCell::default(); (cols * rows) as usize],
            cursor: 0,
        }
    }

    pub fn get(&self, pos: MPos) -> MilCell {
        if pos.0 >= self.cols || pos.1 >= self.rows {
            return MilCell::default();
        }
        self.cells[(pos.1 * self.cols + pos.0) as usize]
    }

    /// AI-MIL-1.1: refresh one cell (cycling over the grid) from the
    /// player's buildings.
    pub fn refresh_next(&mut self, world: &World, buildings: &[Building], config: &AiConfig) {
        let total = self.cols * self.rows;
        if total == 0 {
            return;
        }
        let index = self.cursor;
        self.cursor = (self.cursor + 1) % total;
        let pos = (index % self.cols, index / self.cols);
        let mut cell = MilCell::default();
        for b in buildings.iter().filter(|b| mcell_of(b.at) == pos) {
            let w = config.weights_of(b.building_type);
            cell.score += w.score;
            cell.threat += w.threat;
            if b.under_construction || world.upgrading(b.at).is_some() {
                cell.threat_building += w.threat;
            }
        }
        self.cells[index as usize] = cell;
    }

    /// Refresh every cell at once (used when an AI is activated).
    pub fn refresh_all(&mut self, world: &World, buildings: &[Building], config: &AiConfig) {
        for _ in 0..self.cols * self.rows {
            self.refresh_next(world, buildings, config);
        }
    }

    /// Whether any building is recorded in the cell.
    pub fn has_threat(&self, pos: MPos) -> bool {
        self.get(pos).threat > 0 || self.get(pos).threat_building > 0
    }
}

/// AI-MIL-1.2: the threat balance of a cell: over the 3 x 3 neighbourhood
/// (weight 1 on (x + 1, y + 1), the centre and the seven others 0.5) and all
/// alive players, plus the threat of enemies and minus the threat of `player`
/// and its allies.
pub fn threat_balance(
    world: &World,
    maps: &[MilitaryMap],
    alive: &[bool],
    player: PlayerId,
    pos: MPos,
) -> f32 {
    let mut total = 0.0f32;
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            let (x, y) = (i64::from(pos.0) + dx, i64::from(pos.1) + dy);
            if x < 0 || y < 0 {
                continue;
            }
            // AI-MIL-1.2 (T26): the cell at (x + 1, y + 1) weighs 1, the
            // centre and the seven others 0.5.
            let weight = if dx == 1 && dy == 1 { 1.0 } else { 0.5 };
            for (slot, map) in maps.iter().enumerate() {
                if !alive.get(slot).copied().unwrap_or(false) {
                    continue;
                }
                let other = PlayerId(slot as u8);
                let threat = map.get((x as u32, y as u32)).threat as f32;
                if other == player || !world.are_enemies(player, other) {
                    total -= weight * threat;
                } else {
                    total += weight * threat;
                }
            }
        }
    }
    total
}

/// AI-MIL-1.3: one cell of the player's territory map.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TerritoryCell {
    /// Military-site penalty P.
    pub penalty: f32,
    /// Attack result counter C.
    pub result: f32,
    /// Catapults placed K.
    pub catapults: u32,
    /// Attack attempt counter A.
    pub attempts: f32,
}

/// AI-MIL-1.3: the AI player's own 8 x 8 territory map.
#[derive(Debug, Clone, PartialEq)]
pub struct TerritoryMap {
    pub cols: u32,
    pub rows: u32,
    cells: Vec<TerritoryCell>,
}

impl TerritoryMap {
    pub fn new(width: u32, height: u32) -> Self {
        let (cols, rows) = (width.div_ceil(MCELL), height.div_ceil(MCELL));
        Self {
            cols,
            rows,
            cells: vec![TerritoryCell::default(); (cols * rows) as usize],
        }
    }

    pub fn get(&self, pos: MPos) -> TerritoryCell {
        self.index(pos)
            .map_or_else(Default::default, |i| self.cells[i])
    }

    pub fn get_mut(&mut self, pos: MPos) -> Option<&mut TerritoryCell> {
        let i = self.index(pos)?;
        Some(&mut self.cells[i])
    }

    fn index(&self, pos: MPos) -> Option<usize> {
        (pos.0 < self.cols && pos.1 < self.rows).then(|| (pos.1 * self.cols + pos.0) as usize)
    }

    pub fn clear(&mut self) {
        self.cells.fill(TerritoryCell::default());
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_struct!(MilitaryMap {
    cols,
    rows,
    cells,
    cursor
});

s2_sim::snap_struct!(TerritoryMap { cols, rows, cells });

#[cfg(test)]
mod tests {
    use super::*;

    // spec: AI-MIL-1.1
    #[test]
    fn eight_field_cells() {
        assert_eq!(mcell_of((7, 7)), (0, 0));
        assert_eq!(mcell_of((8, 15)), (1, 1));
        assert_eq!(morigin((3, 2)), (24, 16));
        let map = MilitaryMap::new(20, 9);
        assert_eq!((map.cols, map.rows), (3, 2));
    }
}
