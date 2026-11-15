//! The map as the simulation sees it: per-field heights and state bits.

use crate::grid::{self, Point};

/// Field state bits (`FMT-s2m-map.md` FMT-S2M-4.4, MAP-GRID-7.1).
pub mod state {
    pub const BLOCKED: u32 = 0x0001;
    pub const STREET: u32 = 0x0002;
    pub const WATER: u32 = 0x0004;
    pub const COAST: u32 = 0x0008;
    pub const FOR_MINING: u32 = 0x0010;
    pub const BORDER: u32 = 0x0020;
    pub const ELEVATION_BLOCKED: u32 = 0x0040;
    pub const DEPOSIT_BLOCKED: u32 = 0x0080;
    pub const MOVEMENT_BLOCKED: u32 = 0x0100;
    pub const FOR_BUILDING: u32 = 0x0200;
    pub const DOODAD_BLOCKED: u32 = 0x0400;
    pub const LANDING_POSITION: u32 = 0x0800;
    pub const FOR_HARBOR: u32 = 0x1000;
}

/// Per-field data of a map, indexed by `y * width + x`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Terrain {
    pub width: u32,
    pub height: u32,
    /// Height of each field: the value of its own height sample (MAP-GRID-4.2).
    pub heights: Vec<i32>,
    /// Heights below this count as this value in an area mean (MAP-GRID-6.2).
    pub height_floor: i32,
    /// State bits ([`state`]) as stored in the map. No recalculation on load
    /// (ECO-PLACE-1.6).
    pub states: Vec<u32>,
    /// MAP-GRID-4.1: the height samples, `(4 * width + 1)` by
    /// `(4 * height + 1)`, row by row; empty when only field heights are
    /// known (test maps), and then a field's sample is its height and every
    /// other sample counts as level with it.
    pub samples: Vec<i32>,
}

impl Terrain {
    pub fn contains(&self, field: Point) -> bool {
        field.0 < self.width && field.1 < self.height
    }

    /// Index of a field in the per-field layers. The field must be inside.
    pub fn index(&self, field: Point) -> usize {
        debug_assert!(self.contains(field));
        field.1 as usize * self.width as usize + field.0 as usize
    }

    pub fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn height_at(&self, field: Point) -> i32 {
        self.heights[self.index(field)]
    }

    pub fn has(&self, field: Point, bits: u32) -> bool {
        self.states[self.index(field)] & bits != 0
    }

    pub fn neighbour(&self, field: Point, direction: u8) -> Option<Point> {
        let direction = grid::Direction::new(direction)?;
        grid::neighbour(field, direction, self.width, self.height)
    }

    pub fn neighbours(&self, field: Point) -> [Option<Point>; 6] {
        grid::neighbours(field, self.width, self.height)
    }

    /// MAP-GRID-6.2: mean height of the fields within `radius` of `center`
    /// that are inside the map, each raised to at least the floor value.
    /// Integer division, truncating toward zero.
    pub fn mean_height(&self, center: Point, radius: u32) -> i64 {
        let fields = grid::area(center, radius, self.width, self.height);
        let sum: i64 = fields
            .iter()
            .map(|&f| i64::from(self.height_at(f).max(self.height_floor)))
            .sum();
        sum / fields.len().max(1) as i64
    }

    /// MAP-GRID-4.1: columns and rows of the sample lattice.
    pub fn sample_size(&self) -> (u32, u32) {
        (
            grid::SAMPLES_PER_FIELD * self.width + 1,
            grid::SAMPLES_PER_FIELD * self.height + 1,
        )
    }

    /// The height of a sample; without a sample lattice, that of the field
    /// at or before it (MAP-GRID-4.3).
    pub fn sample_height(&self, sample: Point) -> i32 {
        let (columns, rows) = self.sample_size();
        if !self.samples.is_empty() && sample.0 < columns && sample.1 < rows {
            return self.samples[sample.1 as usize * columns as usize + sample.0 as usize];
        }
        let field = grid::sample_field_truncating(sample);
        let field = (field.0.min(self.width - 1), field.1.min(self.height - 1));
        self.height_at(field)
    }

    /// Set a sample's height; a field's own sample sets the field's height
    /// too (MAP-GRID-4.2).
    pub fn set_sample_height(&mut self, sample: Point, height: i32) {
        let (columns, rows) = self.sample_size();
        if !self.samples.is_empty() && sample.0 < columns && sample.1 < rows {
            self.samples[sample.1 as usize * columns as usize + sample.0 as usize] = height;
        }
        let field = grid::sample_field_truncating(sample);
        if self.contains(field) && grid::field_sample(field) == sample {
            let index = self.index(field);
            self.heights[index] = height;
        }
    }

    /// Every field of the map, row by row.
    pub fn fields(&self) -> impl Iterator<Item = Point> {
        let (width, height) = (self.width, self.height);
        (0..height).flat_map(move |y| (0..width).map(move |x| (x, y)))
    }
}
