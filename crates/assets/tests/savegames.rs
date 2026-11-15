//! Test cases of `specs/FMT-savegame.md` for the header (T1, T5), run
//! against the player's own saves in `S2_SAVES_DIR` (`<Documents>\S2\saves`).
//! When it is unset each test prints a note and passes without checking
//! anything.

use s2_assets::rc00;
use s2_assets::s2m;

/// The decoded body of a save of `S2_SAVES_DIR`, or `None` when it is unset.
fn decoded(name: &str) -> Option<Vec<u8>> {
    let Some(dir) = std::env::var_os("S2_SAVES_DIR") else {
        eprintln!("skipped: S2_SAVES_DIR is not set");
        return None;
    };
    let path = std::path::Path::new(&dir).join(name);
    let wrapped = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    // FMT-SAV-1.2: the rc00 container keyed by the file name, as a map.
    Some(rc00::decode(name, &wrapped).unwrap_or_else(|e| panic!("{name}: {e}")))
}

/// FMT-SAV-2.1: `MapInfo` version 9 with the player names, the map the game
/// was started from; FMT-SAV-2.2: the map part.
fn check(name: &str, size: Option<usize>) {
    let Some(data) = decoded(name) else { return };
    if let Some(size) = size {
        assert_eq!(data.len(), size, "decoded size of {name}");
    }
    let info = s2m::parse_info(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
    let save = info.save.as_ref().expect("a save header");
    assert_eq!(save.player_names[0], "default");
    assert!(save.player_names[1..].iter().all(|n| n == "AI"), "{save:?}");
    assert_eq!(save.value, 0);
    assert_eq!(info.name, "MP_2P_Storm_Coast");
    assert_eq!((info.width, info.height), (170, 170));
    // FMT-SAV-2.2: the map part reads with `Map Exploration` version 1, up
    // to the game block (T2).
    let map = s2m::parse(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
    let start = map.game_block.expect("a save has a game block");
    assert!(map.logic.tick > 0);
    assert!(map.exploration[0].iter().all(|&c| c == 0 || c == 1));
    assert!(map.exploration[0].contains(&1));
    let marker = &data[start..start + 12];
    assert_eq!(
        marker[4..8],
        s2_assets::rc00::crc32(b"Game File Logic").to_le_bytes()
    );
}

// FMT-SAV T5: the 2023 save, 0x6b6180 bytes decoded.
// spec: FMT-SAV-2.1 (T1)
#[test]
fn coastal_storm_header() {
    check("17-03-2023 16-40-11 Coastal Storm.sav", Some(0x6b6180));
}

// FMT-SAV T1: the autosave (rewritten whenever the game autosaves, so its
// size is not fixed).
// spec: FMT-SAV-2.1 (T1)
#[test]
fn autosave_header() {
    check("Autosave.sav", None);
}

// FMT-SAV T7a: the autosave the original wrote on 2026-10-20 at 20:49
// (E-sav-004, 1 835 993 bytes) has an upgrade running in its sixth
// building, to the watchtower, whose record holds a `VillageConstruction`
// and a `VillageBulldozing` after the target type; its one remains record
// holds a `Blocking` (Q-260). Checked only while `Autosave.sav` is that
// file (the game rewrites it).
// spec: FMT-SAV-3.4a, FMT-SAV-3.4b (T7a)
#[test]
fn autosave_of_2049_with_a_running_upgrade() {
    let Some(dir) = std::env::var_os("S2_SAVES_DIR") else {
        eprintln!("skipped: S2_SAVES_DIR is not set");
        return;
    };
    let file =
        std::fs::read(std::path::Path::new(&dir).join("Autosave.sav")).expect("Autosave.sav");
    if file.len() != 1_835_993 {
        eprintln!("skipped: Autosave.sav is not the one of 2026-10-20 20:49");
        return;
    }
    let save = s2_assets::sav::load("Autosave.sav", &file).expect("rc00");
    assert_eq!(save.buildings.len(), 21, "{:?}", save.problems);
    assert_eq!(save.remains, 1, "{:?}", save.problems);
    let running: Vec<usize> = (0..21)
        .filter(|&i| save.buildings[i].upgrade.is_some())
        .collect();
    assert_eq!(running, [5]);
    let guard = &save.buildings[5];
    assert_eq!((guard.building_type, guard.owner), (0x12e6_7603, 1));
    assert_eq!((guard.field.x, guard.field.y), (138, 57));
    assert_eq!(guard.id & 0xffff_ffff, 90300);
    let upgrade = guard.upgrade.as_ref().unwrap();
    assert_eq!(upgrade.target, 0x7d00_b493);
    assert_eq!(upgrade.construction_left, [1.0, 1.0]);
}
