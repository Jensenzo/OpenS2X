use super::*;
use std::collections::HashMap;

/// Writes the stream primitives of FMT-S2M-1 and remembers named offsets.
#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    offsets: HashMap<&'static str, usize>,
}

impl Writer {
    fn mark(&mut self, name: &'static str) {
        self.offsets.insert(name, self.bytes.len());
    }
    fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn string(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.bytes.extend_from_slice(s.as_bytes());
    }
    fn marker(&mut self, section: &str, version: u32) {
        self.u32(version);
        self.u32(rc00::crc32(section.as_bytes()));
        self.u32(section.len() as u32);
    }
    fn field(&mut self, x: u32, y: u32) {
        self.marker("PatternCursor", 0);
        self.u32(x);
        self.u32(y);
    }
    fn fine(&mut self, x: u32, y: u32) {
        self.marker("ElevationCursor", 0);
        self.u32(x);
        self.u32(y);
    }
    fn id(&mut self, id: u64) {
        self.marker("logic UniqueId", 0);
        self.u64(id);
    }
    /// A system marker followed by its flag.
    fn system(&mut self, section: &'static str, version: u32) {
        self.mark(section);
        self.marker(section, version);
        self.u32(1);
    }
    fn size(&mut self) {
        self.u32(W);
        self.u32(H);
    }
}

const W: u32 = 3;
const H: u32 = 2;
const CELLS: u32 = W * H;

/// A complete 3 x 2 map with one or two of everything, padded to 128.
fn tiny_map() -> Writer {
    let mut w = Writer::default();

    w.mark("MapInfo");
    w.marker("MapInfo", 7);
    w.mark("starts");
    w.u32(2);
    w.field(1, 0);
    w.field(2, 1);
    w.string("tiny");
    w.mark("size");
    w.size();
    for player_type in [2, 1, 0, 0, 0, 0, 0, 0] {
        w.i32(player_type);
    }
    for player in 0..8 {
        for value in [0, player, -1, 2] {
            w.i32(value);
        }
    }
    for value in [4, 0, 10] {
        w.i32(value);
    }
    w.system("Core UUID", 0);
    w.bytes.extend(0u8..16);

    w.mark("LogicSystem");
    w.marker("LogicSystem", 7);
    w.u64(100);
    w.mark("logic flag");
    w.u32(1);
    for _ in 0..3 {
        w.f32(0.0);
    }
    w.system("TriggerSystem", 0);
    w.u32(1);
    w.marker("TriggerObject", 1);
    w.u32(1);
    w.id(7);
    w.u32(3);
    w.field(2, 0);
    w.u32(0);
    w.u32(0);
    w.string("Region_K");
    w.u32(2);
    w.f32(0.5);
    w.i32(-1);

    w.system("MapSystem", 0);
    w.mark("MapSystem size");
    w.size();

    w.system("ElevationMap", 1);
    w.i32(-100);
    w.mark("height grid size");
    w.u32(4 * W + 1);
    w.u32(4 * H + 1);
    for i in 0..(4 * W + 1) * (4 * H + 1) {
        w.i32(i as i32 - 50);
    }

    w.system("PatternMap", 0);
    w.mark("pattern count");
    w.u32(CELLS);
    for i in 0..CELLS {
        w.u32(0xBABE_0000 + i);
    }
    w.system("GridStatesMap", 0);
    w.u32(CELLS);
    for i in 0..CELLS {
        w.u32(1 << i);
    }

    w.system("Map Resources", 0);
    w.size();
    for i in 0..CELLS {
        if i == 4 {
            w.u32(7);
            w.u32(0x4012_E5A3);
        } else {
            w.u32(0);
            w.u32(NO_GOOD);
        }
    }
    w.system("Map Territory", 0);
    w.size();
    for _ in 0..CELLS {
        w.i32(-1);
    }
    w.system("Map Exploration", 0);
    w.size();
    for player in 0..8 {
        for _ in 0..CELLS {
            w.i32(player);
        }
    }
    w.system("Map Continents", 1);
    w.size();
    for i in 0..CELLS {
        w.i32(i as i32 % 2);
    }
    w.u32(2);
    for (index, water) in [(0, 1), (1, 0)] {
        w.marker("Map Continent", 3);
        w.u32(3);
        w.u32(water);
        w.u32(index);
        for bound in [0, 0, 2, 1] {
            w.i32(bound);
        }
        w.u32(index);
        for _ in 0..index {
            w.field(1, 1);
        }
        w.u32(1);
        w.i32(1 - index as i32);
    }
    w.u32(2482);

    w.system("resources", 4);
    w.mark("deposit count");
    w.u32(1);
    w.u32(0x5D93_6A9E);
    w.marker("deposit", 1);
    w.id(12);
    w.field(1, 1);
    w.id(NO_OBJECT);
    w.mark("deposit position");
    w.fine(6, 4);
    w.f32(1.0);
    w.i32(0);
    w.i32(-1);

    w.u32(2);
    // One animal that never moved and one walking from (0,1) over (1,1)
    // to (2,1).
    for walking in [false, true] {
        w.u32(0xA111_0000 + u32::from(walking));
        w.marker("Resources Animal", 2);
        w.id(20 + u64::from(walking));
        w.f32(0.0);
        w.field(0, u32::from(!walking));
        w.marker("Navy Movement", 1);
        w.marker("Resources Path", 0);
        w.marker("Movement Path Base", 1);
        w.u32(u32::from(walking));
        if walking {
            w.u32(3);
            w.field(2, 1);
            w.field(1, 1);
            w.field(0, 1);
            w.u32(1);
            w.u32(1);
            w.u32(0);
        }
        w.field(0, 1);
        w.marker("Movement Base", 0);
        w.field(0, 1);
        if walking {
            w.field(2, 1);
            w.field(1, 1);
        } else {
            w.field(0, 1);
            w.mark("step target");
            w.field(u32::MAX, u32::MAX);
        }
        w.marker("Movement Interpolator", 0);
        w.f32(if walking { 0.25 } else { -1.0 });
        w.f32(0.0);
        w.f32(if walking { 0.045 } else { 0.0 });
        w.i32(3);
        w.i32(4);
        w.id(NO_OBJECT);
    }
    w.system("Resources AnimalRespawn", 0);
    w.u32(1);
    w.u32(8);
    w.i32(2);
    w.i32(3);
    w.u32(0);
    w.u32(0);

    w.system("DoodadsSystem", 0);
    for (count, base) in [(2u32, 30u64), (0, 0), (1, 40)] {
        w.u32(count);
        for i in 0..count {
            w.u32(0xD00D_0000 + i);
            w.marker("DoodadsObject", 1);
            w.id(base + u64::from(i));
            w.fine(8 + i, 0);
        }
    }
    w.system("Logic Ambients", 0);
    w.u32(1);
    w.u32(0x5050_5050);
    w.field(2, 1);

    w.mark("end");
    let padded = w.bytes.len().next_multiple_of(PADDING_BLOCK);
    w.bytes.resize(padded, 0);
    w
}

fn patched(offset_name: &str, delta: usize, value: u32) -> Result<Map, S2mError> {
    let mut w = tiny_map();
    let at = w.offsets[offset_name] + delta;
    w.bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    parse(&w.bytes)
}

// FMT-S2M-1.2
#[test]
fn marker_hash_is_crc_of_section_name() {
    assert_eq!(rc00::crc32(b"MapInfo"), 0xC835_B682);
}

// spec: FMT-S2M-1.2, FMT-S2M-2.1, FMT-S2M-3.1, FMT-S2M-4.1, FMT-S2M-5.1, FMT-S2M-6.1
#[test]
fn parses_every_section() {
    let w = tiny_map();
    let map = parse(&w.bytes).unwrap();

    // FMT-S2M-2
    assert_eq!(map.info.name, "tiny");
    assert_eq!((map.info.width, map.info.height), (W, H));
    assert_eq!(
        map.info.start_positions,
        [FieldPos { x: 1, y: 0 }, FieldPos { x: 2, y: 1 }]
    );
    assert_eq!(map.info.player_types, [2, 1, 0, 0, 0, 0, 0, 0]);
    assert_eq!(map.info.player_records[5], [0, 5, -1, 2]);
    assert_eq!(map.info.info_values, [4, 0, 10]);
    assert_eq!(map.info.uuid[15], 15);

    // FMT-S2M-3
    assert_eq!(map.logic.next_unique_id, 100);
    assert_eq!(map.logic.clock, [0.0; 3]);
    assert_eq!(map.logic.tick, -1);
    assert_eq!(
        map.triggers,
        [Trigger {
            id: 7,
            kind: 3,
            position: FieldPos { x: 2, y: 0 },
            reserved: 0,
            active: false,
            name: "Region_K".into(),
            player: 2,
            timer: 0.5,
        }]
    );

    // FMT-S2M-4
    assert_eq!(map.heights.floor, -100);
    assert_eq!((map.heights.width, map.heights.height), (13, 9));
    assert_eq!(map.heights.samples.len(), 13 * 9);
    assert_eq!(map.heights.get(SamplePos { x: 0, y: 0 }), Some(-50));
    assert_eq!(map.heights.get(SamplePos { x: 12, y: 8 }), Some(66));
    assert_eq!(map.heights.get(SamplePos { x: 13, y: 0 }), None);
    assert_eq!(map.patterns[5], 0xBABE_0005);
    assert_eq!(map.states[2], state::WATER);
    assert_eq!(map.states[4], state::FOR_MINING);
    assert_eq!(
        map.resources[4],
        Resource {
            amount: 7,
            good: 0x4012_E5A3
        }
    );
    assert_eq!(
        map.resources[0],
        Resource {
            amount: 0,
            good: NO_GOOD
        }
    );
    assert_eq!(map.territory, [-1; 6]);
    assert_eq!(map.exploration.len(), MAX_PLAYERS);
    assert_eq!(map.exploration[7], [7; 6]);
    assert_eq!(map.continent_of, [0, 1, 0, 1, 0, 1]);
    assert_eq!(map.continents.len(), 2);
    assert!(map.continents[0].is_water);
    assert_eq!(map.continents[0].positions, []);
    assert_eq!(map.continents[1].bounds, [0, 0, 2, 1]);
    assert_eq!(map.continents[1].positions, [FieldPos { x: 1, y: 1 }]);
    assert_eq!(map.continents[1].links, [0]);
    assert_eq!(map.continents_trailer, 2482);

    // FMT-S2M-5
    assert_eq!(
        map.deposits,
        [Deposit {
            type_id: 0x5D93_6A9E,
            id: 12,
            field: FieldPos { x: 1, y: 1 },
            linked_id: NO_OBJECT,
            position: SamplePos { x: 6, y: 4 },
            size: 1.0,
            unknown_a: 0,
            unknown_b: -1,
        }]
    );
    assert_eq!(map.animals.len(), 2);
    let idle = &map.animals[0];
    assert_eq!(idle.path, None);
    assert_eq!(idle.home, FieldPos { x: 0, y: 1 });
    assert_eq!((idle.position, idle.current), (idle.home, idle.home));
    assert_eq!(idle.destination, idle.home);
    assert_eq!(idle.step_target, None);
    assert_eq!(
        (idle.progress, idle.carry_over, idle.progress_per_tick),
        (-1.0, 0.0, 0.0)
    );
    assert_eq!(idle.counters, [3, 4]);
    assert_eq!(idle.linked_id, NO_OBJECT);
    let walking = &map.animals[1];
    let path = walking.path.as_ref().unwrap();
    assert_eq!(path.fields.len(), 3);
    assert_eq!(
        (path.unknown_a, path.index, path.unknown_b),
        (true, 1, false)
    );
    assert_eq!(walking.id, 21);
    assert_eq!(walking.home, FieldPos { x: 0, y: 0 });
    assert_eq!(walking.current, FieldPos { x: 0, y: 1 });
    assert_eq!(walking.destination, path.fields[0]);
    assert_eq!(walking.step_target, Some(path.fields[1]));
    assert_eq!(walking.current, path.fields[2]);
    assert_eq!(walking.progress_per_tick, 0.045);
    assert_eq!(
        map.animal_respawn,
        AnimalRespawn {
            counter: 1,
            target_population: 8,
            timer: [2, 3]
        }
    );
    assert_eq!(map.resources_trailer, [0, 0]);

    // FMT-S2M-6
    assert_eq!(map.doodads.non_blocking.len(), 2);
    assert_eq!(map.doodads.unknown, []);
    assert_eq!(
        map.doodads.blocking,
        [Doodad {
            type_id: 0xD00D_0000,
            id: 40,
            position: SamplePos { x: 8, y: 0 }
        }]
    );
    assert_eq!(
        map.ambients,
        [Ambient {
            sound_id: 0x5050_5050,
            field: FieldPos { x: 2, y: 1 }
        }]
    );

    // FMT-S2M-1.8
    assert_eq!(map.padding, w.bytes.len() - w.offsets["end"]);
    assert!(map.padding < PADDING_BLOCK);
}

// MAP-GRID-4.2
#[test]
fn field_height_uses_the_fields_own_sample() {
    let map = parse(&tiny_map().bytes).unwrap();
    assert_eq!(map.field_height(FieldPos { x: 0, y: 0 }), Some(-50));
    // Odd row: sample (4 * 1 + 2, 4), index 4 * 13 + 6.
    assert_eq!(FieldPos { x: 1, y: 1 }.sample(), SamplePos { x: 6, y: 4 });
    assert_eq!(map.field_height(FieldPos { x: 1, y: 1 }), Some(8));
    assert_eq!(map.field_height(FieldPos { x: 3, y: 0 }), None);
    assert_eq!(map.field_index(FieldPos { x: 2, y: 1 }), Some(5));
}

// T20 in miniature, FMT-S2M-1.6
#[test]
fn rejects_the_demo_map_header_version() {
    assert_eq!(
        patched("MapInfo", 0, 1),
        Err(S2mError::UnsupportedVersion {
            section: "MapInfo",
            version: 1
        })
    );
}

// FMT-S2M-1.6, FMT-SAV-4.1 (2026-11-11): `MapInfo` is read as written, 7
// (maps) or 9 (the game's saves); 8 is neither and is unsupported, as is 10.
// spec: FMT-S2M-1.6, FMT-SAV-4.1
#[test]
fn rejects_map_info_versions_nobody_writes() {
    for version in [8, 10] {
        assert_eq!(
            patched("MapInfo", 0, version),
            Err(S2mError::UnsupportedVersion {
                section: "MapInfo",
                version
            })
        );
    }
}

// FMT-S2M-1.6: other sections are version-checked too.
#[test]
fn rejects_unknown_section_versions() {
    assert_eq!(
        patched("ElevationMap", 0, 0),
        Err(S2mError::UnsupportedVersion {
            section: "ElevationMap",
            version: 0
        })
    );
}

// T23 in miniature
// spec: FMT-S2M-1.6, FMT-S2M-3.1a
#[test]
fn rejects_the_uncorrected_logic_and_resources_versions() {
    for section in ["LogicSystem", "resources"] {
        assert_eq!(
            patched(section, 0, 0),
            Err(S2mError::UnsupportedVersion {
                section,
                version: 0
            })
        );
    }
}

// T28, FMT-S2M-1.3: the none cursor is accepted only as an animal's step
// target, and no other out-of-range value is accepted even there.
#[test]
fn none_cursor_is_rejected_outside_the_step_target() {
    // First start position: count, marker, then x and y.
    let mut w = tiny_map();
    let at = w.offsets["starts"] + 4 + 12;
    w.bytes[at..at + 8].copy_from_slice(&[0xFF; 8]);
    assert!(matches!(
        parse(&w.bytes),
        Err(S2mError::OutOfRange {
            what: "start position",
            x: u32::MAX,
            y: u32::MAX,
            ..
        })
    ));

    // The deposit's field cursor sits before its linked id and position.
    let mut w = tiny_map();
    let at = w.offsets["deposit position"] - 20 - 8;
    w.bytes[at..at + 8].copy_from_slice(&[0xFF; 8]);
    assert!(matches!(
        parse(&w.bytes),
        Err(S2mError::OutOfRange {
            what: "field cursor",
            x: u32::MAX,
            y: u32::MAX,
            ..
        })
    ));

    assert!(matches!(
        patched("step target", 12, W),
        Err(S2mError::OutOfRange {
            what: "field cursor",
            x: 3,
            ..
        })
    ));
    assert!(matches!(
        patched("step target", 12, u32::MAX - 1),
        Err(S2mError::OutOfRange { .. })
    ));
}

// FMT-S2M-1.2
#[test]
fn rejects_wrong_marker_hash_and_length() {
    let offset = tiny_map().offsets["MapSystem"];
    for delta in [4, 8] {
        assert_eq!(
            patched("MapSystem", delta, 0x1234_5678),
            Err(S2mError::WrongSection {
                offset,
                expected: "MapSystem"
            })
        );
    }
    assert!(matches!(
        parse(b"-- this is a Lua script, not a map --"),
        Err(S2mError::WrongSection {
            offset: 0,
            expected: "MapInfo"
        })
    ));
}

// FMT-S2M-1.1
#[test]
fn rejects_invalid_bool() {
    let offset = tiny_map().offsets["PatternMap"] + 12;
    assert_eq!(
        patched("PatternMap", 12, 2),
        Err(S2mError::InvalidBool { offset, value: 2 })
    );
}

// Q-025
// spec: FMT-S2M-1.2
#[test]
fn rejects_unset_section_flags() {
    let offset = tiny_map().offsets["Map Territory"] + 12;
    assert_eq!(
        patched("Map Territory", 12, 0),
        Err(S2mError::FlagNotSet {
            offset,
            section: "Map Territory"
        })
    );
    assert!(matches!(
        patched("logic flag", 0, 0),
        Err(S2mError::FlagNotSet {
            section: "LogicSystem",
            ..
        })
    ));
}

// FMT-S2M-4.1 to 4.3
#[test]
fn rejects_inconsistent_sizes() {
    assert!(matches!(
        patched("MapSystem size", 0, W + 1),
        Err(S2mError::SizeMismatch {
            what: "MapSystem",
            expected: 3,
            found: 4,
            ..
        })
    ));
    assert!(matches!(
        patched("height grid size", 4, 4 * H),
        Err(S2mError::SizeMismatch {
            what: "height grid height",
            expected: 9,
            found: 8,
            ..
        })
    ));
    assert!(matches!(
        patched("pattern count", 0, CELLS - 1),
        Err(S2mError::SizeMismatch {
            what: "pattern count",
            ..
        })
    ));
}

// FMT-S2M-1.3, FMT-S2M-1.4
#[test]
fn rejects_cursors_outside_the_map() {
    // Second start position: count, one cursor, marker, then x.
    assert!(matches!(
        patched("starts", 4 + 20 + 12, W),
        Err(S2mError::OutOfRange {
            what: "start position",
            x: 3,
            y: 1,
            ..
        })
    ));
    // The closing edge of the height grid is still a valid fine position.
    assert!(patched("deposit position", 12, 4 * W).is_ok());
    assert!(matches!(
        patched("deposit position", 12, 4 * W + 1),
        Err(S2mError::OutOfRange {
            what: "fine cursor",
            ..
        })
    ));
}

// FMT-S2M-1.8
#[test]
fn checks_padding() {
    let w = tiny_map();
    let end = w.offsets["end"];
    assert!(!end.is_multiple_of(PADDING_BLOCK));

    // No padding at all is within 0..127.
    assert_eq!(parse(&w.bytes[..end]).unwrap().padding, 0);

    let mut bytes = w.bytes.clone();
    *bytes.last_mut().unwrap() = 1;
    assert_eq!(
        parse(&bytes),
        Err(S2mError::NonZeroPadding {
            offset: bytes.len() - 1
        })
    );

    let mut bytes = w.bytes[..end].to_vec();
    bytes.resize(end + PADDING_BLOCK, 0);
    assert_eq!(
        parse(&bytes),
        Err(S2mError::TrailingData {
            offset: end,
            len: PADDING_BLOCK
        })
    );
}

// Every truncation is an error, never a panic.
// spec: FMT-S2M-1.2
#[test]
fn truncated_input_is_an_error() {
    let w = tiny_map();
    for len in 0..w.offsets["end"] {
        assert!(parse(&w.bytes[..len]).is_err(), "prefix of {len} bytes");
    }
}

// Counts are checked against the bytes left before anything is allocated.
// spec: none (a damaged count reserves no memory)
#[test]
fn huge_counts_do_not_allocate() {
    assert!(matches!(
        patched("deposit count", 0, u32::MAX),
        Err(S2mError::UnexpectedEnd { .. })
    ));
    assert!(matches!(
        patched("starts", 0, u32::MAX),
        Err(S2mError::UnexpectedEnd { .. })
    ));
    // A map size that needs more cells than the file holds.
    let mut w = tiny_map();
    for name in ["size", "MapSystem size"] {
        let at = w.offsets[name];
        w.bytes[at..at + 8].copy_from_slice(&[0xFF; 8]);
    }
    assert!(parse(&w.bytes).is_err());
}

// No byte pattern may panic the parser.
// spec: none (damaged input does not panic)
#[test]
fn corrupted_input_does_not_panic() {
    let w = tiny_map();
    let mut state = 0x2545_F491u32;
    for _ in 0..4000 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let mut bytes = w.bytes.clone();
        let at = (state >> 8) as usize % bytes.len();
        bytes[at] ^= 1 << (state & 7);
        let _ = parse(&bytes);
    }
}

// FMT-RC-1.2: a map that is not wrapped is parsed as is.
#[test]
fn load_accepts_unwrapped_contents() {
    let w = tiny_map();
    assert_eq!(load("tiny.s2m", &w.bytes).unwrap().info.name, "tiny");
}

// spec: FMT-RC-1.2, FMT-RC-1.5
#[test]
fn load_reports_container_errors() {
    let mut file = rc00::MAGIC.to_vec();
    file.extend_from_slice(b"rc01");
    file.resize(64, 0);
    assert_eq!(
        load("tiny.s2m", &file),
        Err(S2mError::Container(Rc00Error::TagMismatch {
            found: *b"rc01"
        }))
    );
}
