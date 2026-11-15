//! Test cases of `specs/FMT-s2m-map.md` (T1..T29) and the map-based ones of
//! `specs/MAP-grid-geometry.md` (T19, T21, T22, T24..T26), run against the
//! player's own install through `S2_GAME_DIR`. When it is unset each test
//! prints a note and passes without checking anything.

mod common;

use common::{game_dir, hex, read_game_file, sha256};
use s2_assets::rc00;
use s2_assets::s2m::{self, state, FieldPos, Map, S2mError, SamplePos, NO_GOOD, PADDING_BLOCK};
use s2_sim::grid;

const M1: &str = "data/maps/tutorialMaps/tutorial01.s2m";
const M2: &str = "data/maps/campaignMaps/campaign01.s2m";
const M3: &str = "data/maps/freeGameMaps/MP_4P_Fight.s2m";
const M4: &str = "data/maps/freeGameMaps/MP_6P_Paradise_Island.s2m";

/// FMT-S2M T5: the good id of fish.
const FISH: u32 = 0x4012_E5A3;

/// Unwrap and parse a map of the install, checking the unwrapped size and
/// SHA-256 on the way. `None` when `S2_GAME_DIR` is unset.
fn load_vector(rel: &str, size: usize, sha256_hex: &str) -> Option<Map> {
    let wrapped = read_game_file(rel)?;
    let plain = rc00::decode(rel, &wrapped).unwrap_or_else(|e| panic!("{rel}: {e}"));
    assert_eq!(plain.len(), size, "unwrapped size of {rel}");
    assert_eq!(hex(&sha256(&plain)), sha256_hex, "SHA-256 of {rel}");
    Some(s2m::parse(&plain).unwrap_or_else(|e| panic!("{rel}: {e}")))
}

fn m1() -> Option<Map> {
    load_vector(
        M1,
        940544,
        "8cb3678b9a7b8f0fd474e13aa4f1547b9de63722519b28639d402358cf918fc0",
    )
}

fn m2() -> Option<Map> {
    load_vector(
        M2,
        1488768,
        "a5403060eba0b55483d0f38661c68c46534e021684c70055d67ea7ace0349b6b",
    )
}

fn m3() -> Option<Map> {
    load_vector(
        M3,
        839808,
        "1e3e58292d55181aef8a845442519a89d9c471711ee06d93b1e2f1538d8435c8",
    )
}

fn m4() -> Option<Map> {
    load_vector(
        M4,
        9002240,
        "fa5f5de39ba223d5cc8b5e9b425af51b80a8fd7f8002cd4f6989948fdd67b54d",
    )
}

fn field(x: u32, y: u32) -> FieldPos {
    FieldPos { x, y }
}

fn sample(x: u32, y: u32) -> SamplePos {
    SamplePos { x, y }
}

fn fields(points: &[(u32, u32)]) -> Vec<FieldPos> {
    points.iter().map(|&(x, y)| field(x, y)).collect()
}

fn has(map: &Map, at: FieldPos, bit: u32) -> bool {
    map.states[map.field_index(at).unwrap()] & bit != 0
}

fn count_state(map: &Map, bit: u32) -> usize {
    map.states.iter().filter(|&&s| s & bit != 0).count()
}

/// Deposits, animals, doodad lists 1 / 2 / 3, ambients, triggers, continents.
fn object_counts(map: &Map) -> [usize; 8] {
    [
        map.deposits.len(),
        map.animals.len(),
        map.doodads.non_blocking.len(),
        map.doodads.unknown.len(),
        map.doodads.blocking.len(),
        map.ambients.len(),
        map.triggers.len(),
        map.continents.len(),
    ]
}

fn all_fields(map: &Map) -> impl Iterator<Item = FieldPos> {
    let (width, height) = (map.info.width, map.info.height);
    (0..height).flat_map(move |y| (0..width).map(move |x| field(x, y)))
}

fn has_neighbour_with(map: &Map, at: FieldPos, bit: u32) -> bool {
    grid::neighbours((at.x, at.y), map.info.width, map.info.height)
        .into_iter()
        .flatten()
        .any(|(x, y)| has(map, field(x, y), bit))
}

/// MAP-GRID-7.2: the non-Water fields with a Water neighbour.
fn is_computed_coast(map: &Map, at: FieldPos) -> bool {
    !has(map, at, state::WATER) && has_neighbour_with(map, at, state::WATER)
}

/// Every `.s2m` under `data/maps`, as paths relative to the install.
fn release_maps() -> Option<Vec<String>> {
    let dir = game_dir()?;
    let mut found = Vec::new();
    let mut pending = vec![dir.root().join("data/maps")];
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                pending.push(entry);
            } else if entry.extension().is_some_and(|ext| ext == "s2m") {
                let rel = entry.strip_prefix(dir.root()).unwrap();
                found.push(rel.to_str().unwrap().to_owned());
            }
        }
    }
    found.sort();
    Some(found)
}

// T1
// spec: FMT-S2M-2.1, FMT-S2M-2.2, FMT-S2M-2.3, FMT-S2M-2.4, FMT-S2M-2.5, FMT-S2M-2.7, FMT-S2M-3.1, FMT-S2M-3.4 (T1)
#[test]
fn t01_tutorial_header() {
    let Some(map) = m1() else { return };
    assert_eq!(map.info.name, "tutorial01");
    assert_eq!((map.info.width, map.info.height), (80, 80));
    assert_eq!(map.info.start_positions, fields(&[(45, 38)]));
    assert_eq!(map.info.player_types, [0; 8]);
    assert_eq!(map.info.info_values[2], 10);
    assert_eq!(map.logic.next_unique_id, 3340);
    assert_eq!(map.logic.tick, -1);
}

// T2
// spec: FMT-S2M-4.2 (T2)
#[test]
fn t02_tutorial_height_grid() {
    let Some(map) = m1() else { return };
    let heights = &map.heights;
    assert_eq!((heights.width, heights.height), (321, 321));
    assert_eq!(heights.samples.first(), Some(&-8598));
    assert_eq!(heights.samples.last(), Some(&-480));
    assert_eq!(heights.get(sample(180, 152)), Some(1872));
    assert_eq!(heights.samples.iter().min(), Some(&-22890));
    assert_eq!(heights.samples.iter().max(), Some(&24015));
    assert_eq!(heights.floor, -100);
}

// T3, T4
// spec: FMT-S2M-4.3, FMT-S2M-4.4 (T3, T4)
#[test]
fn t03_t04_tutorial_fields() {
    let Some(map) = m1() else { return };
    let sea = map.field_index(field(0, 0)).unwrap();
    assert_eq!(map.patterns[sea], 0xBABE_B00B);
    assert_eq!(map.states[sea], 0x0004);

    let start = map.field_index(field(45, 38)).unwrap();
    assert_eq!(map.patterns[start], 0x4545_FAC4);
    assert_eq!(map.states[start], 0x0200);
    assert_eq!(map.resources[start].amount, 0);
    assert_eq!(map.resources[start].good, NO_GOOD);
    assert_eq!(map.territory[start], -1);
    assert_eq!(map.continent_of[start], 1);
}

// T5
// spec: FMT-S2M-4.5 (T5)
#[test]
fn t05_tutorial_first_resource() {
    let Some(map) = m1() else { return };
    let first = all_fields(&map)
        .find(|&at| map.resources[map.field_index(at).unwrap()].good != NO_GOOD)
        .unwrap();
    assert_eq!(first, field(25, 4));
    let resource = map.resources[map.field_index(first).unwrap()];
    assert_eq!((resource.amount, resource.good), (7, FISH));
}

// T6
// spec: FMT-S2M-1.8, FMT-S2M-5.1, FMT-S2M-5.3, FMT-S2M-6.1, FMT-S2M-6.3 (T6)
#[test]
fn t06_tutorial_object_counts() {
    let Some(map) = m1() else { return };
    assert_eq!(object_counts(&map), [771, 6, 1416, 0, 40, 10, 0, 51]);
    assert_eq!(map.padding, 106);
}

// T7, MAP-GRID T21
// spec: FMT-S2M-5.2, MAP-GRID-4.2 (T7, MAP-GRID T21)
#[test]
fn t07_tutorial_first_deposit() {
    let Some(map) = m1() else { return };
    let deposit = &map.deposits[0];
    assert_eq!(deposit.type_id, 0x5D93_6A9E);
    assert_eq!(deposit.id, 12);
    assert_eq!(deposit.field, field(38, 61));
    assert_eq!(deposit.position, sample(154, 244));
    assert_eq!(deposit.size, 1.0);
    assert_eq!(deposit.position, deposit.field.sample());
}

// T8
// spec: FMT-S2M-4.4 (T8)
#[test]
fn t08_tutorial_state_layer() {
    let Some(map) = m1() else { return };
    assert_eq!(count_state(&map, state::WATER), 2925);
    assert_eq!(count_state(&map, state::FOR_BUILDING), 2415);
    assert_eq!(count_state(&map, state::DEPOSIT_BLOCKED), 771);
}

// T9
// spec: FMT-S2M-2.2, FMT-S2M-2.4, FMT-S2M-2.5, FMT-S2M-2.7, FMT-S2M-3.1 (T9)
#[test]
fn t09_campaign_header() {
    let Some(map) = m2() else { return };
    assert_eq!(map.info.name, "campaign01");
    assert_eq!((map.info.width, map.info.height), (100, 100));
    assert_eq!(map.info.start_positions, fields(&[(33, 33)]));
    assert_eq!(map.info.player_types, [2, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(map.info.info_values, [4, 0, 1]);
    assert_eq!(map.logic.next_unique_id, 56747);
}

// T10
// spec: FMT-S2M-3.2, FMT-S2M-3.3 (T10)
#[test]
fn t10_campaign_triggers() {
    let Some(map) = m2() else { return };
    let triggers: Vec<_> = map
        .triggers
        .iter()
        .map(|t| (t.name.as_str(), t.kind, t.position))
        .collect();
    assert_eq!(
        triggers,
        [
            ("Region_K", 2, field(55, 51)),
            ("RegionVictory__0", 3, field(64, 70))
        ]
    );
}

// T11
// spec: FMT-S2M-4.2 (T11)
#[test]
fn t11_campaign_height_grid() {
    let Some(map) = m2() else { return };
    let heights = &map.heights;
    assert_eq!((heights.width, heights.height), (401, 401));
    assert_eq!(heights.samples.first(), Some(&-9081));
    assert_eq!(heights.samples.last(), Some(&-6976));
    assert_eq!(heights.get(sample(132, 132)), Some(503));
}

// T12
// spec: FMT-S2M-4.8, FMT-S2M-4.9, FMT-S2M-5.1, FMT-S2M-6.1 (T12)
#[test]
fn t12_campaign_object_counts() {
    let Some(map) = m2() else { return };
    assert_eq!(object_counts(&map), [1122, 68, 2427, 0, 130, 14, 2, 67]);
    let sea = &map.continents[0];
    assert!(sea.is_water);
    assert_eq!(sea.field_count, 4385);
    assert_eq!(sea.bounds, [0, 0, 99, 99]);
}

// T13
// spec: FMT-S2M-2.2, FMT-S2M-2.3, FMT-S2M-3.1 (T13)
#[test]
fn t13_fight_header() {
    let Some(map) = m3() else { return };
    assert_eq!(map.info.name, "MP_4P_Fight");
    assert_eq!((map.info.width, map.info.height), (80, 80));
    assert_eq!(
        map.info.start_positions,
        fields(&[(27, 57), (52, 56), (52, 24), (26, 24)])
    );
    assert_eq!(map.logic.next_unique_id, 14153);
}

// T14
// spec: FMT-S2M-4.2 (T14)
#[test]
fn t14_fight_height_grid() {
    let Some(map) = m3() else { return };
    assert_eq!(map.heights.samples.first(), Some(&-10000));
    assert_eq!(map.heights.samples.last(), Some(&-10000));
    assert_eq!(map.heights.get(sample(108, 228)), Some(347));
}

// T15
// spec: FMT-S2M-1.8, FMT-S2M-4.8 (T15)
#[test]
fn t15_fight_object_counts() {
    let Some(map) = m3() else { return };
    assert_eq!(object_counts(&map)[..6], [180, 56, 546, 0, 56, 14]);
    assert_eq!(map.continents.len(), 19);
    assert_eq!(map.padding, 57);
}

// T16
// spec: FMT-S2M-2.2, FMT-S2M-2.4 (T16)
#[test]
fn t16_paradise_island_header() {
    let Some(map) = m4() else { return };
    assert_eq!(map.info.name, "MP_6P_Paradise_Island");
    assert_eq!((map.info.width, map.info.height), (240, 240));
    assert_eq!(
        map.info.start_positions,
        fields(&[
            (54, 160),
            (116, 202),
            (185, 160),
            (185, 86),
            (115, 43),
            (53, 85)
        ])
    );
}

// T17
// spec: FMT-S2M-4.2 (T17)
#[test]
fn t17_paradise_island_height_grid() {
    let Some(map) = m4() else { return };
    let heights = &map.heights;
    assert_eq!((heights.width, heights.height), (961, 961));
    assert_eq!(heights.samples.first(), Some(&-6980));
    assert_eq!(heights.samples.last(), Some(&-7149));
    assert_eq!(heights.samples.iter().min(), Some(&-10188));
    assert_eq!(heights.samples.iter().max(), Some(&29537));
}

// T18
// spec: FMT-S2M-1.8, FMT-S2M-4.8 (T18)
#[test]
fn t18_paradise_island_object_counts() {
    let Some(map) = m4() else { return };
    assert_eq!(object_counts(&map)[..6], [2563, 809, 26426, 0, 1906, 418]);
    assert_eq!(map.continents.len(), 196);
    assert_eq!(map.padding, 7);
}

// T20
// spec: FMT-S2M-1.6 (T20)
#[test]
fn t20_demo_maps_are_rejected() {
    for rel in ["data/map/Presentation.s2m", "data/map/X_MediterranDemo.s2m"] {
        let Some(file) = read_game_file(rel) else {
            return;
        };
        assert_eq!(
            s2m::load(rel, &file),
            Err(S2mError::UnsupportedVersion {
                section: "MapInfo",
                version: 1
            }),
            "{rel}"
        );
    }
}

// T23
// spec: FMT-S2M-1.6, FMT-S2M-3.1a (T23)
#[test]
fn t23_uncorrected_versions_are_rejected() {
    let Some(wrapped) = read_game_file(M1) else {
        return;
    };
    let plain = rc00::decode(M1, &wrapped).unwrap();
    for section in ["LogicSystem", "resources"] {
        // A marker is version, CRC-32 of the name, length of the name.
        let mut tail = rc00::crc32(section.as_bytes()).to_le_bytes().to_vec();
        tail.extend_from_slice(&(section.len() as u32).to_le_bytes());
        let found: Vec<usize> = (4..plain.len() - 8)
            .filter(|&i| plain[i..i + 8] == tail[..])
            .collect();
        assert_eq!(found.len(), 1, "markers of {section}");
        let mut changed = plain.clone();
        changed[found[0] - 4..found[0]].copy_from_slice(&[0; 4]);
        assert_eq!(
            s2m::parse(&changed),
            Err(S2mError::UnsupportedVersion {
                section,
                version: 0
            })
        );
    }
}

// T24
// spec: FMT-S2M-5.4 (T24)
#[test]
fn t24_tutorial_animals() {
    let Some(map) = m1() else { return };
    assert_eq!(map.animals.len(), 6);
    let (walking, stopped): (Vec<_>, Vec<_>) = map.animals.iter().partition(|a| a.path.is_some());
    assert_eq!((walking.len(), stopped.len()), (2, 4));
    for animal in stopped {
        assert_eq!(animal.current, animal.destination);
        assert_eq!(animal.step_target, Some(animal.current));
    }
    assert!(map.animals.iter().all(|a| a.step_target.is_some()));
}

// T25
// spec: FMT-S2M-5.4 (T25)
#[test]
fn t25_animals_that_never_moved() {
    for (map, count) in [(m2(), 68), (m3(), 56), (m4(), 809)] {
        let Some(map) = map else { return };
        assert_eq!(map.animals.len(), count);
        for animal in &map.animals {
            let name = &map.info.name;
            assert_eq!(animal.path, None, "{name}: animal {}", animal.id);
            assert_eq!(animal.step_target, None, "{name}: animal {}", animal.id);
            assert_eq!(
                [animal.position, animal.current, animal.destination],
                [animal.home; 3],
                "{name}: animal {}",
                animal.id
            );
            assert_eq!(
                (animal.progress, animal.carry_over, animal.progress_per_tick),
                (-1.0, 0.0, 0.0),
                "{name}: animal {}",
                animal.id
            );
        }
    }
}

// MAP-GRID T19
// spec: MAP-GRID-4.1, MAP-GRID-4.2 (MAP-GRID T19)
#[test]
fn grid_t19_tutorial_field_heights() {
    let Some(map) = m1() else { return };
    for ((x, y), height) in [
        ((45, 38), 1872),
        ((45, 39), 1872),
        ((0, 0), -8598),
        ((79, 79), -7513),
        ((10, 11), -4343),
    ] {
        assert_eq!(map.field_height(field(x, y)), Some(height), "({x}, {y})");
    }
}

// MAP-GRID T24
// spec: MAP-GRID-7.1, MAP-GRID-7.2 (MAP-GRID T24)
#[test]
fn grid_t24_tutorial_coast() {
    let Some(map) = m1() else { return };
    assert_eq!(count_state(&map, state::COAST), 493);
    for at in all_fields(&map) {
        assert_eq!(
            has(&map, at, state::COAST),
            is_computed_coast(&map, at),
            "{at:?}"
        );
    }
}

// T19, T21, T22, T26, T27, T29 and MAP-GRID T22, T25, T26 over every release
// map. The loader rejects every marker version other than those of
// FMT-S2M-1.6 and every out-of-range cursor other than a none step target, so
// loading all maps is most of T22 and T26; the counts are checked here.
#[test]
fn t21_all_release_maps() {
    let Some(maps) = release_maps() else { return };
    assert_eq!(maps.len(), 34);

    // MAP-GRID T22: a deposit stands on its field's sample or on one of that
    // sample's six neighbours (field samples are on even sample rows).
    const DEPOSIT_OFFSETS: [(i64, i64); 7] =
        [(0, 0), (1, 0), (-1, 0), (0, -1), (-1, -1), (0, 1), (-1, 1)];
    let (mut coast_fields, mut fish_fields) = (0, 0);
    let (mut continents, mut deposits, mut animals, mut doodads) = (0, 0, 0, 0);
    let (mut none_cursors, mut walking, mut tight_next_id) = (0, 0, 0);

    for rel in &maps {
        let file = read_game_file(rel).unwrap();
        let map = s2m::load(rel, &file).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let (width, height) = (map.info.width, map.info.height);
        let cells = width as usize * height as usize;

        // T21
        assert!(map.padding < PADDING_BLOCK, "{rel}");

        // FMT-S2M-2.3, FMT-S2M-2.4, FMT-S2M-3.4
        let stem = rel.rsplit('/').next().unwrap().trim_end_matches(".s2m");
        assert_eq!(map.info.name, stem);
        assert_eq!(width, height, "{rel}");
        assert!((80..=240).contains(&width), "{rel}");
        assert_eq!(map.logic.tick, -1, "{rel}");
        assert_eq!(map.patterns.len(), cells, "{rel}");
        assert_eq!(map.states.len(), cells, "{rel}");
        assert_eq!(map.resources.len(), cells, "{rel}");

        // T22
        continents += map.continents.len();
        deposits += map.deposits.len();
        animals += map.animals.len();
        let lists = &map.doodads;
        doodads += lists.non_blocking.len() + lists.unknown.len() + lists.blocking.len();

        // T26, T27
        for animal in &map.animals {
            none_cursors += usize::from(animal.step_target.is_none());
            let Some(path) = &animal.path else { continue };
            walking += 1;
            let index = path.index as usize;
            let step_target = animal.step_target.unwrap();
            assert_eq!(path.fields[0], animal.destination, "{rel}: {}", animal.id);
            assert_eq!(path.fields[index], step_target, "{rel}: {}", animal.id);
            assert_eq!(
                path.fields[index + 1],
                animal.current,
                "{rel}: {}",
                animal.id
            );
            let around = grid::neighbours((animal.current.x, animal.current.y), width, height);
            assert!(
                around.contains(&Some((step_target.x, step_target.y))),
                "{rel}: animal {} steps to a field that is not a neighbour",
                animal.id
            );
        }

        // T29
        let largest_id = (map.triggers.iter().map(|t| t.id))
            .chain(map.deposits.iter().map(|d| d.id))
            .chain(map.animals.iter().map(|a| a.id))
            .chain(lists.non_blocking.iter().map(|d| d.id))
            .chain(lists.unknown.iter().map(|d| d.id))
            .chain(lists.blocking.iter().map(|d| d.id))
            .max()
            .unwrap();
        assert!(map.logic.next_unique_id > largest_id, "{rel}");
        tight_next_id += usize::from(map.logic.next_unique_id == largest_id + 1);

        // T19
        for &start in &map.info.start_positions {
            assert!(has(&map, start, state::FOR_BUILDING), "{rel}: {start:?}");
        }
        for deposit in &map.deposits {
            assert!(
                has(&map, deposit.field, state::DEPOSIT_BLOCKED),
                "{rel}: deposit {}",
                deposit.id
            );
            let own = deposit.field.sample();
            let offset = (
                i64::from(deposit.position.x) - i64::from(own.x),
                i64::from(deposit.position.y) - i64::from(own.y),
            );
            assert!(
                DEPOSIT_OFFSETS.contains(&offset),
                "{rel}: deposit {} is {offset:?} from its field",
                deposit.id
            );
        }
        for doodad in &map.doodads.blocking {
            let at = field(doodad.position.x / 4, doodad.position.y / 4);
            assert!(
                has(&map, at, state::DOODAD_BLOCKED),
                "{rel}: doodad {}",
                doodad.id
            );
        }
        let continent_fields: u64 = map
            .continents
            .iter()
            .map(|c| u64::from(c.field_count))
            .sum();
        assert!(continent_fields <= cells as u64, "{rel}");

        // MAP-GRID T25, T26
        for at in all_fields(&map) {
            let coast = has(&map, at, state::COAST);
            assert_eq!(coast, is_computed_coast(&map, at), "{rel}: {at:?}");
            assert!(!(coast && has(&map, at, state::WATER)), "{rel}: {at:?}");
            coast_fields += usize::from(coast);

            if map.resources[map.field_index(at).unwrap()].good == FISH {
                assert!(has(&map, at, state::WATER), "{rel}: fish at {at:?}");
                assert!(
                    has_neighbour_with(&map, at, state::COAST),
                    "{rel}: fish at {at:?}"
                );
                fish_fields += 1;
            }
        }
    }

    assert_eq!(
        (continents, deposits, animals, doodads),
        (5161, 45961, 4284, 215838)
    );
    assert_eq!(none_cursors, 3927);
    assert_eq!(walking, 90);
    assert_eq!(tight_next_id, 25);
    assert_eq!(coast_fields, 40287);
    assert_eq!(fish_fields, 41471);
}
