//! Loader for `.s2m` map files (`specs/FMT-s2m-map.md`).
//!
//! A map is the complete starting state of a game world: header, height grid,
//! per-field layers and the objects placed on the map. On disk it is wrapped in
//! the `rc00` container; [`load`] unwraps and parses, [`parse`] takes the
//! unwrapped contents. Type ids (FMT-S2M-1.7) and object ids are kept as plain
//! integers.

use crate::rc00::{self, Rc00Error};
use s2_sim::grid;

/// FMT-S2M-2.5: number of player indices in the header and exploration layer.
pub const MAX_PLAYERS: usize = 8;
/// FMT-S2M-1.5: the object id that means "none".
pub const NO_OBJECT: u64 = u64::MAX;
/// FMT-S2M-4.5: the good id of a field without a resource.
pub const NO_GOOD: u32 = u32::MAX;
/// FMT-S2M-1.8: the file is padded with zero bytes to a multiple of this.
pub const PADDING_BLOCK: usize = 128;

/// Bits of the field state layer (FMT-S2M-4.4).
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

/// Why a map could not be loaded. Offsets are into the unwrapped contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S2mError {
    /// The `rc00` container around the map did not decode.
    Container(Rc00Error),
    /// The contents end before a value of `needed` bytes at `offset`.
    UnexpectedEnd { offset: usize, needed: u64 },
    /// FMT-S2M-1.2: the marker at `offset` is not the one for `expected`.
    WrongSection {
        offset: usize,
        expected: &'static str,
    },
    /// FMT-S2M-1.6: a layout version this loader does not know.
    UnsupportedVersion { section: &'static str, version: u32 },
    /// FMT-S2M-1.1: a `bool` that is neither 0 nor 1.
    InvalidBool { offset: usize, value: u32 },
    /// A flag that is 1 in every release map is 0; what the section then
    /// contains is not specified. See Q-025.
    FlagNotSet {
        offset: usize,
        section: &'static str,
    },
    /// Two sizes that must agree do not (FMT-S2M-4).
    SizeMismatch {
        offset: usize,
        what: &'static str,
        expected: u64,
        found: u64,
    },
    /// FMT-S2M-1.3, FMT-S2M-1.4: a cursor outside the map or the height grid.
    OutOfRange {
        offset: usize,
        what: &'static str,
        x: u32,
        y: u32,
    },
    /// FMT-S2M-1.8: a non-zero byte in the padding.
    NonZeroPadding { offset: usize },
    /// FMT-S2M-1.8: a whole padding block or more is left after the last object.
    TrailingData { offset: usize, len: usize },
}

impl std::fmt::Display for S2mError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Container(e) => write!(f, "{e}"),
            Self::UnexpectedEnd { offset, needed } => {
                write!(
                    f,
                    "map ends early: {needed} more bytes needed at offset {offset}"
                )
            }
            Self::WrongSection { offset, expected } => {
                write!(f, "expected section `{expected}` at offset {offset}")
            }
            Self::UnsupportedVersion { section, version } => {
                write!(f, "unsupported `{section}` version {version}")
            }
            Self::InvalidBool { offset, value } => {
                write!(f, "invalid bool {value} at offset {offset}")
            }
            Self::FlagNotSet { offset, section } => {
                write!(f, "flag of `{section}` at offset {offset} is not set")
            }
            Self::SizeMismatch {
                offset,
                what,
                expected,
                found,
            } => write!(
                f,
                "{what} at offset {offset} is {found}, expected {expected}"
            ),
            Self::OutOfRange { offset, what, x, y } => {
                write!(f, "{what} ({x}, {y}) at offset {offset} is out of range")
            }
            Self::NonZeroPadding { offset } => {
                write!(f, "non-zero padding byte at offset {offset}")
            }
            Self::TrailingData { offset, len } => {
                write!(
                    f,
                    "{len} unread bytes after the last object at offset {offset}"
                )
            }
        }
    }
}

impl std::error::Error for S2mError {}

impl S2mError {
    /// Where in the contents it happened, if the error says.
    pub fn offset(&self) -> Option<usize> {
        match self {
            Self::Container(_) | Self::UnsupportedVersion { .. } => None,
            Self::UnexpectedEnd { offset, .. }
            | Self::WrongSection { offset, .. }
            | Self::InvalidBool { offset, .. }
            | Self::FlagNotSet { offset, .. }
            | Self::SizeMismatch { offset, .. }
            | Self::OutOfRange { offset, .. }
            | Self::NonZeroPadding { offset }
            | Self::TrailingData { offset, .. } => Some(*offset),
        }
    }
}

impl From<Rc00Error> for S2mError {
    fn from(e: Rc00Error) -> Self {
        Self::Container(e)
    }
}

/// A map field, from a field cursor (FMT-S2M-1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldPos {
    pub x: u32,
    pub y: u32,
}

/// A point of the height grid, from a fine cursor (FMT-S2M-1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplePos {
    pub x: u32,
    pub y: u32,
}

impl FieldPos {
    /// The height sample that is this field (MAP-GRID-4.2).
    pub fn sample(self) -> SamplePos {
        let (x, y) = grid::field_sample((self.x, self.y));
        SamplePos { x, y }
    }
}

/// Map header (FMT-S2M-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapInfo {
    /// FMT-S2M-2.2: entry *i* belongs to player index *i*.
    pub start_positions: Vec<FieldPos>,
    /// FMT-S2M-2.3
    pub name: String,
    /// FMT-S2M-2.4: size in fields.
    pub width: u32,
    pub height: u32,
    /// FMT-S2M-2.5: the player's kind: 0 no player, 1 computer, 2 human.
    pub player_types: [i32; MAX_PLAYERS],
    /// FMT-S2M-2.6: per player index: tribe, colour, team (-1 none), computer
    /// strength.
    pub player_records: [[i32; 4]; MAX_PLAYERS],
    /// FMT-S2M-2.7: meanings unknown (Q-016).
    pub info_values: [i32; 3],
    /// FMT-S2M-2.8: unique id of the map.
    pub uuid: [u8; 16],
    /// FMT-SAV-2.1: what a save game's header (version 9) adds.
    pub save: Option<SaveHeader>,
}

/// FMT-SAV-2.1: the fields a save game's `MapInfo` adds after the UUID:
/// version 8 a flag and the eight player names, version 9 one more value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveHeader {
    pub flag: bool,
    pub player_names: [String; MAX_PLAYERS],
    /// Observed 0; meaning open (Q-100).
    pub value: u32,
}

/// Saved logic state (FMT-S2M-3.1, FMT-S2M-3.4).
#[derive(Debug, Clone, PartialEq)]
pub struct LogicState {
    /// One more than the largest object id used so far.
    pub next_unique_id: u64,
    /// Frame step, time of next tick, elapsed logic time.
    pub clock: [f32; 3],
    /// -1 while no tick has run.
    pub tick: i32,
}

/// A script trigger region (FMT-S2M-3.3; the fields' meaning: Q-187 a).
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub id: u64,
    /// CAM-2.1: 1 explored, 2 territory, 3 victory region (2 or 3 in
    /// release maps).
    pub kind: u32,
    pub position: FieldPos,
    /// Kept, read by no trigger rule (0 in release maps).
    pub reserved: u32,
    /// CAM-2.2: the active flag (false in release maps).
    pub active: bool,
    /// The name map scripts receive in `onTriggerIsActive`.
    pub name: String,
    /// CAM-2.1: the player whose exploration or territory the trigger
    /// tests (2 for campaign05's `RegionVictory__1` to `__3`, else 0).
    pub player: u32,
    /// CAM-2.2: the seconds already gathered toward the next 5-second
    /// check (0.1 to 5.0 in release maps).
    pub timer: f32,
}

/// Height grid (FMT-S2M-4.2). `width = 4 * map width + 1`, likewise `height`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeightGrid {
    /// Heights below this count as this value in area means (MAP-GRID-6.2).
    pub floor: i32,
    pub width: u32,
    pub height: u32,
    /// Row by row; 1000 units are one world unit and water level is 0.
    pub samples: Vec<i32>,
}

impl HeightGrid {
    pub fn get(&self, pos: SamplePos) -> Option<i32> {
        if pos.x >= self.width || pos.y >= self.height {
            return None;
        }
        let index = pos.y as usize * self.width as usize + pos.x as usize;
        self.samples.get(index).copied()
    }
}

/// Underground or fish resource of one field (FMT-S2M-4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    pub amount: u32,
    /// [`NO_GOOD`] when the field is empty.
    pub good: u32,
}

/// A connected region of land or water (FMT-S2M-4.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continent {
    pub field_count: u32,
    pub is_water: bool,
    pub index: u32,
    /// Min x, min y, max x, max y.
    pub bounds: [i32; 4],
    /// Positions on the continent; probably landing or harbor spots.
    pub positions: Vec<FieldPos>,
    /// Indices of other continents; probably neighbours.
    pub links: Vec<i32>,
}

/// A tree or stone pile (FMT-S2M-5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Deposit {
    pub type_id: u32,
    pub id: u64,
    /// The field it stands on.
    pub field: FieldPos,
    /// [`NO_OBJECT`] in release maps.
    pub linked_id: u64,
    /// Exact position on the height grid.
    pub position: SamplePos,
    /// 1.0 = fully grown.
    pub size: f32,
    /// 0 in release maps.
    pub unknown_a: i32,
    /// -1 in release maps.
    pub unknown_b: i32,
}

/// Way points of an animal saved in mid-walk (FMT-S2M-5.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimalPath {
    /// Destination first: entry 0 is the destination, entry `index` the field
    /// of the step in progress and entry `index + 1` the field the animal is on.
    pub fields: Vec<FieldPos>,
    /// True in release maps.
    pub unknown_a: bool,
    pub index: u32,
    pub unknown_b: bool,
}

/// An animal and its movement state (FMT-S2M-5.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Animal {
    pub type_id: u32,
    pub id: u64,
    /// FMT-S2M-5.4, ECO-FOOD-5.14a: the clock value when its current or
    /// last walk started; 0.0 in release maps.
    pub walk_start: f32,
    /// The field the animal was placed on; it roams around it.
    pub home: FieldPos,
    /// `None` unless the animal was saved in mid-walk.
    pub path: Option<AnimalPath>,
    /// The field the animal is on; equal to `current` in release maps.
    pub position: FieldPos,
    /// Cursor A: current field.
    pub current: FieldPos,
    /// Cursor B: destination of the walk, equal to `current` when not walking.
    pub destination: FieldPos,
    /// Cursor C: target of the step in progress or of the last step taken.
    /// `None` (the none cursor, FMT-S2M-1.3) until the animal first steps.
    pub step_target: Option<FieldPos>,
    /// Progress of the step; -1.0 when not walking.
    pub progress: f32,
    /// Carry-over into the next step.
    pub carry_over: f32,
    /// Progress per tick of the current or last step; 0.0 if it never took one.
    pub progress_per_tick: f32,
    pub counters: [i32; 2],
    /// [`NO_OBJECT`] in release maps.
    pub linked_id: u64,
}

/// Animal respawn state (FMT-S2M-5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimalRespawn {
    /// A counter.
    pub counter: u32,
    /// Grows with map area.
    pub target_population: u32,
    /// The nested timer; two counters.
    pub timer: [i32; 2],
}

/// A decoration object (FMT-S2M-6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doodad {
    pub type_id: u32,
    pub id: u64,
    pub position: SamplePos,
}

/// The three doodad lists (FMT-S2M-6.1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Doodads {
    /// List 1: decoration that does not block its field.
    pub non_blocking: Vec<Doodad>,
    /// List 2: empty in every release map; purpose unknown.
    pub unknown: Vec<Doodad>,
    /// List 3: each makes its field `DoodadBlocked`.
    pub blocking: Vec<Doodad>,
}

/// An ambient sound emitter (FMT-S2M-6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ambient {
    pub sound_id: u32,
    pub field: FieldPos,
}

/// A parsed map. Per-field layers are indexed by `y * width + x`
/// ([`Map::field_index`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    pub info: MapInfo,
    pub logic: LogicState,
    pub triggers: Vec<Trigger>,
    pub heights: HeightGrid,
    /// FMT-S2M-4.3: terrain pattern type id per field.
    pub patterns: Vec<u32>,
    /// FMT-S2M-4.4: [`state`] bits per field.
    pub states: Vec<u32>,
    pub resources: Vec<Resource>,
    /// FMT-S2M-4.6: owner player index per field, -1 for none.
    pub territory: Vec<i32>,
    /// FMT-S2M-4.7: one layer per player index.
    pub exploration: Vec<Vec<i32>>,
    /// FMT-S2M-4.8: continent index per field, -1 for none.
    pub continent_of: Vec<i32>,
    pub continents: Vec<Continent>,
    /// FMT-S2M-4.8: the value after the continent list; meaning unknown.
    pub continents_trailer: u32,
    pub deposits: Vec<Deposit>,
    pub animals: Vec<Animal>,
    pub animal_respawn: AnimalRespawn,
    /// FMT-S2M-5.6: two values that close the `resources` section; both 0 in
    /// release maps, meaning unknown.
    pub resources_trailer: [u32; 2],
    pub doodads: Doodads,
    pub ambients: Vec<Ambient>,
    /// FMT-S2M-1.8: number of zero bytes after the last object; 0 for a
    /// save game.
    pub padding: usize,
    /// FMT-SAV-2.2, 3.1: in a save game, the offset where the game block
    /// (`Game File Logic`) starts, right after the map part.
    pub game_block: Option<usize>,
}

impl Map {
    /// Index of a field in the per-field layers, or `None` outside the map.
    pub fn field_index(&self, field: FieldPos) -> Option<usize> {
        (field.x < self.info.width && field.y < self.info.height)
            .then(|| field.y as usize * self.info.width as usize + field.x as usize)
    }

    /// MAP-GRID-4.2: the height of a field is the value of its sample.
    pub fn field_height(&self, field: FieldPos) -> Option<i32> {
        self.field_index(field)?;
        self.heights.get(field.sample())
    }

    /// The per-field data the simulation works on: field heights, the height
    /// floor and the stored state bits (no recalculation, ECO-PLACE-1.6).
    pub fn terrain(&self) -> s2_sim::map::Terrain {
        let (width, height) = (self.info.width, self.info.height);
        let heights = (0..height)
            .flat_map(|y| (0..width).map(move |x| FieldPos { x, y }))
            .map(|field| self.field_height(field).unwrap_or(self.heights.floor))
            .collect();
        s2_sim::map::Terrain {
            width,
            height,
            heights,
            height_floor: self.heights.floor,
            states: self.states.clone(),
            samples: self.heights.samples.clone(),
        }
    }
}

/// Load a map file as found on disk. `path` is the path or name the file is
/// opened under and `file` its complete contents. A file without the `rc00`
/// magic is taken as already unwrapped (FMT-RC-1.2).
pub fn load(path: &str, file: &[u8]) -> Result<Map, S2mError> {
    if rc00::is_wrapped(file) {
        parse(&rc00::decode(path, file)?)
    } else {
        parse(file)
    }
}

/// Parse the unwrapped contents of a map file.
/// FMT-S2M-2, FMT-SAV-2.1: only the header of a map or save game file
/// (decoded), as the game's header cache holds it (FMT-SAV-1.4).
pub fn parse_info(data: &[u8]) -> Result<MapInfo, S2mError> {
    let mut r = Reader {
        data,
        pos: 0,
        fields: None,
    };
    read_map_info(&mut r)
}

pub fn parse(data: &[u8]) -> Result<Map, S2mError> {
    let mut r = Reader {
        data,
        pos: 0,
        fields: None,
    };

    let info = read_map_info(&mut r)?;
    let save = info.save.is_some();
    let (width, height) = (info.width, info.height);
    let cells = u64::from(width) * u64::from(height);

    // FMT-S2M-3
    r.marker("LogicSystem", 7)?;
    let next_unique_id = r.u64()?;
    r.set_flag("LogicSystem")?;
    let clock = [r.f32()?, r.f32()?, r.f32()?];
    r.marker("TriggerSystem", 0)?;
    r.set_flag("TriggerSystem")?;
    let triggers = r.list(TRIGGER_MIN_SIZE, read_trigger)?;
    let tick = r.i32()?;

    // FMT-S2M-4.1
    r.marker("MapSystem", 0)?;
    r.set_flag("MapSystem")?;
    r.layer_size("MapSystem", width, height)?;

    // FMT-S2M-4.2
    r.marker("ElevationMap", 1)?;
    r.set_flag("ElevationMap")?;
    let floor = r.i32()?;
    let heights_at = r.pos;
    let (ew, eh) = (r.u32()?, r.u32()?);
    for (what, found, fields) in [
        ("height grid width", ew, width),
        ("height grid height", eh, height),
    ] {
        r.expect_size(
            heights_at,
            what,
            u64::from(grid::SAMPLES_PER_FIELD) * u64::from(fields) + 1,
            u64::from(found),
        )?;
    }
    let heights = HeightGrid {
        floor,
        width: ew,
        height: eh,
        samples: r.i32_array(u64::from(ew) * u64::from(eh))?,
    };

    // FMT-S2M-4.3, FMT-S2M-4.4
    r.marker("PatternMap", 0)?;
    r.set_flag("PatternMap")?;
    r.layer_count("pattern count", cells)?;
    let patterns = r.u32_array(cells)?;
    r.marker("GridStatesMap", 0)?;
    r.set_flag("GridStatesMap")?;
    r.layer_count("field state count", cells)?;
    let states = r.u32_array(cells)?;

    // FMT-S2M-4.5
    r.marker("Map Resources", 0)?;
    r.set_flag("Map Resources")?;
    r.layer_size("Map Resources", width, height)?;
    let resources = r
        .u32_array(cells.saturating_mul(2))?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Resource {
            amount: pair[0],
            good: pair[1],
        })
        .collect();

    // FMT-S2M-4.6
    r.marker("Map Territory", 0)?;
    r.set_flag("Map Territory")?;
    r.layer_size("Map Territory", width, height)?;
    let territory = r.i32_array(cells)?;

    // FMT-S2M-4.7; FMT-SAV-2.2: a save writes version 1, whose cells count
    // by their low byte.
    let exploration_version = r.marker_of("Map Exploration", &[0, 1])?;
    r.set_flag("Map Exploration")?;
    r.layer_size("Map Exploration", width, height)?;
    let mut exploration = Vec::with_capacity(MAX_PLAYERS);
    for _ in 0..MAX_PLAYERS {
        let mut cells = r.i32_array(cells)?;
        if exploration_version == 1 {
            cells.iter_mut().for_each(|c| *c &= 0xFF);
        }
        exploration.push(cells);
    }

    // FMT-S2M-4.8
    r.marker("Map Continents", 1)?;
    r.set_flag("Map Continents")?;
    r.layer_size("Map Continents", width, height)?;
    let continent_of = r.i32_array(cells)?;
    let continents = r.list(CONTINENT_MIN_SIZE, read_continent)?;
    let continents_trailer = r.u32()?;

    // FMT-S2M-5
    r.marker("resources", 4)?;
    r.set_flag("resources")?;
    let deposits = r.list(DEPOSIT_SIZE, read_deposit)?;
    let animals = r.list(ANIMAL_MIN_SIZE, read_animal)?;
    r.marker("Resources AnimalRespawn", 0)?;
    r.set_flag("Resources AnimalRespawn")?;
    let animal_respawn = AnimalRespawn {
        counter: r.u32()?,
        target_population: r.u32()?,
        timer: [r.i32()?, r.i32()?],
    };
    // FMT-S2M-5.6
    let resources_trailer = [r.u32()?, r.u32()?];

    // FMT-S2M-6
    r.marker("DoodadsSystem", 0)?;
    r.set_flag("DoodadsSystem")?;
    let doodads = Doodads {
        non_blocking: r.list(DOODAD_SIZE, read_doodad)?,
        unknown: r.list(DOODAD_SIZE, read_doodad)?,
        blocking: r.list(DOODAD_SIZE, read_doodad)?,
    };
    r.marker("Logic Ambients", 0)?;
    r.set_flag("Logic Ambients")?;
    let ambients = r.list(AMBIENT_SIZE, |r| {
        Ok(Ambient {
            sound_id: r.u32()?,
            field: r.field_cursor()?,
        })
    })?;

    Ok(Map {
        info,
        logic: LogicState {
            next_unique_id,
            clock,
            tick,
        },
        triggers,
        heights,
        patterns,
        states,
        resources,
        territory,
        exploration,
        continent_of,
        continents,
        continents_trailer,
        deposits,
        animals,
        animal_respawn,
        resources_trailer,
        doodads,
        ambients,
        padding: if save { 0 } else { r.padding()? },
        game_block: save.then_some(r.pos),
    })
}

const MARKER_SIZE: u64 = 12;
/// Marker plus two `u32`.
const CURSOR_SIZE: u64 = MARKER_SIZE + 8;
/// Marker plus `u64`.
const OBJECT_ID_SIZE: u64 = MARKER_SIZE + 8;

// Smallest encodings of list elements. A count is checked against the bytes
// left before anything is allocated for it.
const TRIGGER_MIN_SIZE: u64 =
    MARKER_SIZE + 4 + OBJECT_ID_SIZE + 4 + CURSOR_SIZE + 4 + 4 + 4 + 4 + 4;
const CONTINENT_MIN_SIZE: u64 = MARKER_SIZE + 4 + 4 + 4 + 16 + 4 + 4;
const DEPOSIT_SIZE: u64 =
    4 + MARKER_SIZE + OBJECT_ID_SIZE + CURSOR_SIZE + OBJECT_ID_SIZE + CURSOR_SIZE + 12;
const ANIMAL_MIN_SIZE: u64 = 4
    + MARKER_SIZE
    + OBJECT_ID_SIZE
    + 4
    + CURSOR_SIZE
    + 2 * MARKER_SIZE
    + (MARKER_SIZE + 4)
    + CURSOR_SIZE
    + MARKER_SIZE
    + 3 * CURSOR_SIZE
    + (MARKER_SIZE + 12)
    + 8
    + OBJECT_ID_SIZE;
const DOODAD_SIZE: u64 = 4 + MARKER_SIZE + OBJECT_ID_SIZE + CURSOR_SIZE;
const AMBIENT_SIZE: u64 = 4 + CURSOR_SIZE;

/// FMT-S2M-2
fn read_map_info(r: &mut Reader) -> Result<MapInfo, S2mError> {
    // Maps are version 7, save games version 9 (FMT-SAV-2.1); FMT-S2M-1.6,
    // FMT-SAV-4.1: no other version is written.
    let version = r.marker_of("MapInfo", &[7, 9])?;
    // The size is stored after the start positions, so these are range-checked
    // once it is known.
    let starts_at = r.pos;
    let start_positions = r.list(CURSOR_SIZE, Reader::field_cursor)?;
    let name = r.string()?;
    let (width, height) = (r.u32()?, r.u32()?);
    if let Some(p) = start_positions
        .iter()
        .find(|p| p.x >= width || p.y >= height)
    {
        return Err(S2mError::OutOfRange {
            offset: starts_at,
            what: "start position",
            x: p.x,
            y: p.y,
        });
    }
    r.fields = Some((width, height));

    let mut player_types = [0; MAX_PLAYERS];
    for value in &mut player_types {
        *value = r.i32()?;
    }
    let mut player_records = [[0; 4]; MAX_PLAYERS];
    for value in player_records.iter_mut().flatten() {
        *value = r.i32()?;
    }
    let info_values = [r.i32()?, r.i32()?, r.i32()?];

    // FMT-S2M-2.8
    r.marker("Core UUID", 0)?;
    r.set_flag("Core UUID")?;
    let mut uuid = [0; 16];
    uuid.copy_from_slice(r.take(16)?);
    // FMT-SAV-2.1
    let save = if version >= 8 {
        let flag = r.bool()?;
        let mut player_names: [String; MAX_PLAYERS] = Default::default();
        for name in &mut player_names {
            *name = r.string()?;
        }
        let value = if version >= 9 { r.u32()? } else { 0 };
        Some(SaveHeader {
            flag,
            player_names,
            value,
        })
    } else {
        None
    };

    Ok(MapInfo {
        start_positions,
        name,
        width,
        height,
        player_types,
        player_records,
        info_values,
        uuid,
        save,
    })
}

/// FMT-S2M-3.3
fn read_trigger(r: &mut Reader) -> Result<Trigger, S2mError> {
    r.marker("TriggerObject", 1)?;
    r.set_flag("TriggerObject")?;
    Ok(Trigger {
        id: r.object_id()?,
        kind: r.u32()?,
        position: r.field_cursor()?,
        reserved: r.u32()?,
        active: r.bool()?,
        name: r.string()?,
        player: r.u32()?,
        timer: r.f32()?,
    })
}

/// FMT-S2M-4.9
fn read_continent(r: &mut Reader) -> Result<Continent, S2mError> {
    r.marker("Map Continent", 3)?;
    Ok(Continent {
        field_count: r.u32()?,
        is_water: r.bool()?,
        index: r.u32()?,
        bounds: [r.i32()?, r.i32()?, r.i32()?, r.i32()?],
        positions: r.list(CURSOR_SIZE, Reader::field_cursor)?,
        links: r.list(4, Reader::i32)?,
    })
}

/// FMT-S2M-5.2
fn read_deposit(r: &mut Reader) -> Result<Deposit, S2mError> {
    let type_id = r.u32()?;
    r.marker("deposit", 1)?;
    Ok(Deposit {
        type_id,
        id: r.object_id()?,
        field: r.field_cursor()?,
        linked_id: r.object_id()?,
        position: r.fine_cursor()?,
        size: r.f32()?,
        unknown_a: r.i32()?,
        unknown_b: r.i32()?,
    })
}

/// FMT-S2M-5.4
fn read_animal(r: &mut Reader) -> Result<Animal, S2mError> {
    let type_id = r.u32()?;
    r.marker("Resources Animal", 2)?;
    let id = r.object_id()?;
    let walk_start = r.f32()?;
    let home = r.field_cursor()?;

    r.marker("Navy Movement", 1)?;
    r.marker("Resources Path", 0)?;
    r.marker("Movement Path Base", 1)?;
    let path = if r.bool()? {
        Some(AnimalPath {
            fields: r.list(CURSOR_SIZE, Reader::field_cursor)?,
            unknown_a: r.bool()?,
            index: r.u32()?,
            unknown_b: r.bool()?,
        })
    } else {
        None
    };
    let position = r.field_cursor()?;

    r.marker("Movement Base", 0)?;
    let (current, destination) = (r.field_cursor()?, r.field_cursor()?);
    // The only place where the none cursor is allowed (FMT-S2M-1.3).
    let step_target = r.optional_field_cursor()?;
    r.marker("Movement Interpolator", 0)?;
    Ok(Animal {
        type_id,
        id,
        walk_start,
        home,
        path,
        position,
        current,
        destination,
        step_target,
        progress: r.f32()?,
        carry_over: r.f32()?,
        progress_per_tick: r.f32()?,
        counters: [r.i32()?, r.i32()?],
        linked_id: r.object_id()?,
    })
}

/// FMT-S2M-6.2. Types with a limited life time carry one more value; the
/// loader cannot tell them apart and none occurs in a map.
fn read_doodad(r: &mut Reader) -> Result<Doodad, S2mError> {
    let type_id = r.u32()?;
    r.marker("DoodadsObject", 1)?;
    Ok(Doodad {
        type_id,
        id: r.object_id()?,
        position: r.fine_cursor()?,
    })
}

/// Cursor over the unwrapped contents (FMT-S2M-1).
pub(crate) struct Reader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) pos: usize,
    /// Map size in fields, once the header has given it.
    pub(crate) fields: Option<(u32, u32)>,
}

impl<'a> Reader<'a> {
    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn end(&self, needed: u64) -> S2mError {
        S2mError::UnexpectedEnd {
            offset: self.pos,
            needed,
        }
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], S2mError> {
        if n > self.remaining() {
            return Err(self.end(n as u64));
        }
        let bytes = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(bytes)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], S2mError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    // FMT-S2M-1.1
    pub(crate) fn u32(&mut self) -> Result<u32, S2mError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, S2mError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, S2mError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(crate) fn f32(&mut self) -> Result<f32, S2mError> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    pub(crate) fn bool(&mut self) -> Result<bool, S2mError> {
        let offset = self.pos;
        match self.u32()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(S2mError::InvalidBool { offset, value }),
        }
    }

    /// A `bool` the spec only ever shows as 1, without saying what 0 means.
    pub(crate) fn set_flag(&mut self, section: &'static str) -> Result<(), S2mError> {
        let offset = self.pos;
        if self.bool()? {
            Ok(())
        } else {
            Err(S2mError::FlagNotSet { offset, section })
        }
    }

    /// Latin-1 bytes with a length prefix.
    pub(crate) fn string(&mut self) -> Result<String, S2mError> {
        let len = self.u32()?;
        let bytes = self.take(self.byte_len(u64::from(len), 1)?)?;
        Ok(bytes.iter().map(|&b| char::from(b)).collect())
    }

    /// FMT-S2M-1.2, FMT-S2M-1.6: a marker of `section` in one of the
    /// `supported` versions; its version.
    pub(crate) fn marker_of(
        &mut self,
        section: &'static str,
        supported: &[u32],
    ) -> Result<u32, S2mError> {
        let offset = self.pos;
        let version = self.u32()?;
        let (crc, len) = (self.u32()?, self.u32()?);
        if crc != rc00::crc32(section.as_bytes()) || len as usize != section.len() {
            return Err(S2mError::WrongSection {
                offset,
                expected: section,
            });
        }
        if !supported.contains(&version) {
            return Err(S2mError::UnsupportedVersion { section, version });
        }
        Ok(version)
    }

    /// FMT-S2M-1.2, FMT-S2M-1.6
    pub(crate) fn marker(&mut self, section: &'static str, supported: u32) -> Result<(), S2mError> {
        let offset = self.pos;
        let version = self.u32()?;
        let (crc, len) = (self.u32()?, self.u32()?);
        if crc != rc00::crc32(section.as_bytes()) || len as usize != section.len() {
            return Err(S2mError::WrongSection {
                offset,
                expected: section,
            });
        }
        if version != supported {
            return Err(S2mError::UnsupportedVersion { section, version });
        }
        Ok(())
    }

    pub(crate) fn cursor(&mut self, section: &'static str) -> Result<(usize, u32, u32), S2mError> {
        let offset = self.pos;
        self.marker(section, 0)?;
        Ok((offset, self.u32()?, self.u32()?))
    }

    /// FMT-S2M-1.3
    pub(crate) fn field_cursor(&mut self) -> Result<FieldPos, S2mError> {
        let (offset, x, y) = self.cursor("PatternCursor")?;
        self.in_map(offset, x, y)
    }

    /// FMT-S2M-1.3: a field cursor where the none cursor is allowed.
    pub(crate) fn optional_field_cursor(&mut self) -> Result<Option<FieldPos>, S2mError> {
        let (offset, x, y) = self.cursor("PatternCursor")?;
        if (x, y) == (u32::MAX, u32::MAX) {
            Ok(None)
        } else {
            self.in_map(offset, x, y).map(Some)
        }
    }

    pub(crate) fn in_map(&self, offset: usize, x: u32, y: u32) -> Result<FieldPos, S2mError> {
        match self.fields {
            Some((width, height)) if x >= width || y >= height => Err(S2mError::OutOfRange {
                offset,
                what: "field cursor",
                x,
                y,
            }),
            _ => Ok(FieldPos { x, y }),
        }
    }

    /// FMT-S2M-1.4
    pub(crate) fn fine_cursor(&mut self) -> Result<SamplePos, S2mError> {
        let (offset, x, y) = self.cursor("ElevationCursor")?;
        let steps = u64::from(grid::SAMPLES_PER_FIELD);
        match self.fields {
            Some((width, height))
                if u64::from(x) > steps * u64::from(width)
                    || u64::from(y) > steps * u64::from(height) =>
            {
                Err(S2mError::OutOfRange {
                    offset,
                    what: "fine cursor",
                    x,
                    y,
                })
            }
            _ => Ok(SamplePos { x, y }),
        }
    }

    /// FMT-S2M-1.5
    pub(crate) fn object_id(&mut self) -> Result<u64, S2mError> {
        self.marker("logic UniqueId", 0)?;
        self.u64()
    }

    /// Bytes taken by `count` items of at least `size` bytes, or an error if
    /// the contents cannot hold them.
    pub(crate) fn byte_len(&self, count: u64, size: u64) -> Result<usize, S2mError> {
        match count.checked_mul(size) {
            Some(bytes) if bytes <= self.remaining() as u64 => Ok(bytes as usize),
            Some(bytes) => Err(self.end(bytes)),
            None => Err(self.end(u64::MAX)),
        }
    }

    /// A `u32` count followed by that many items of at least `min_size` bytes.
    pub(crate) fn list<T>(
        &mut self,
        min_size: u64,
        mut item: impl FnMut(&mut Self) -> Result<T, S2mError>,
    ) -> Result<Vec<T>, S2mError> {
        let count = self.u32()?;
        self.byte_len(u64::from(count), min_size)?;
        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            items.push(item(self)?);
        }
        Ok(items)
    }

    pub(crate) fn u32_array(&mut self, count: u64) -> Result<Vec<u32>, S2mError> {
        let bytes = self.take(self.byte_len(count, 4)?)?;
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect())
    }

    pub(crate) fn i32_array(&mut self, count: u64) -> Result<Vec<i32>, S2mError> {
        let bytes = self.take(self.byte_len(count, 4)?)?;
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| i32::from_le_bytes(*b))
            .collect())
    }

    pub(crate) fn expect_size(
        &self,
        offset: usize,
        what: &'static str,
        expected: u64,
        found: u64,
    ) -> Result<(), S2mError> {
        if expected == found {
            Ok(())
        } else {
            Err(S2mError::SizeMismatch {
                offset,
                what,
                expected,
                found,
            })
        }
    }

    /// The width and height a layer repeats; they must match the header.
    pub(crate) fn layer_size(
        &mut self,
        what: &'static str,
        width: u32,
        height: u32,
    ) -> Result<(), S2mError> {
        let offset = self.pos;
        let (w, h) = (self.u32()?, self.u32()?);
        self.expect_size(offset, what, u64::from(width), u64::from(w))?;
        self.expect_size(offset, what, u64::from(height), u64::from(h))
    }

    /// The field count a layer stores; it must be width * height.
    pub(crate) fn layer_count(&mut self, what: &'static str, cells: u64) -> Result<(), S2mError> {
        let offset = self.pos;
        let count = self.u32()?;
        self.expect_size(offset, what, cells, u64::from(count))
    }

    /// FMT-S2M-1.8: what is left must be fewer than 128 zero bytes.
    pub(crate) fn padding(&self) -> Result<usize, S2mError> {
        let rest = &self.data[self.pos..];
        if rest.len() >= PADDING_BLOCK {
            return Err(S2mError::TrailingData {
                offset: self.pos,
                len: rest.len(),
            });
        }
        match rest.iter().position(|&b| b != 0) {
            Some(i) => Err(S2mError::NonZeroPadding {
                offset: self.pos + i,
            }),
            None => Ok(rest.len()),
        }
    }
}

#[cfg(test)]
mod tests;
