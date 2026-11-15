//! Whole-file vectors T1..T10 and the file-based step vectors S8..S11 from
//! `specs/FMT-rc00-container.md`.
//!
//! These read the player's own install through `S2_GAME_DIR`. When it is unset
//! each test prints a note and passes without checking anything.

mod common;

use common::{hex, read_game_file, sha256};
use s2_assets::rc00::{self, Rc00Error};

const T1: &str = "data/scripts/properties/streets.lua";
const T10: &str = "data/maps/campaignMaps/campaign01.s2m";

fn check_vector(rel: &str, wrapped_size: usize, plain_size: usize, sha256_hex: &str) {
    let Some(wrapped) = read_game_file(rel) else {
        return;
    };
    assert_eq!(wrapped.len(), wrapped_size, "wrapped size of {rel}");
    assert!(rc00::is_wrapped(&wrapped));
    // `decode` also checks FMT-RC-1.4 to 1.6.
    let plain = rc00::decode(rel, &wrapped).unwrap_or_else(|e| panic!("{rel}: {e}"));
    assert_eq!(plain.len(), plain_size, "plain size of {rel}");
    assert_eq!(hex(&sha256(&plain)), sha256_hex, "SHA-256 of {rel}");
}

macro_rules! whole_file_vectors {
    ($($name:ident: $rel:expr, $wrapped:expr, $plain:expr, $sha:expr;)*) => {
        $(
            // spec: FMT-RC-1.3, FMT-RC-1.6, FMT-RC-6.1 (T1 to T7: whole files)
            #[test]
            fn $name() {
                check_vector($rel, $wrapped, $plain, $sha);
            }
        )*
    };
}

whole_file_vectors! {
    t01_streets_lua: T1, 136, 229,
        "a55af21cb9a9e7c0808c1bd7af0dd2a81060dcb5424f725f4a8c4b14a0a12e24";
    t02_config_lua: "data/scripts/properties/config.lua", 466, 898,
        "58a03b99e76a88d79f5fad1e0d37f4f211b6b43400a2469b5a48c92f860ad8f5";
    t03_buildings_lua: "data/scripts/properties/buildings.lua", 16007, 54737,
        "56148a7d6edf478709deb2a3b9a5973b66526a7403fd9f7f5a64077c6f68f25b";
    t04_decals_xml: "data/settings/decals.xml", 124, 133,
        "25d28a0bfd052158171d0a0cb0cdea7722ef98c2ddcf07ca5ab4bdf97e4e2466";
    t05_menu_cfg: "data/scripts/menu.cfg", 231, 458,
        "6dab3c747896c6d28ad1d16ffc9648589a4d03f45209192d72de7e5f9d6c370e";
    t06_info_txt: "data/language/gb/info.txt", 11031, 24404,
        "4a6acb05d2fc2fe1f0f038744d18fcd3906843025c6044a577d2f135e6016909";
    t07_shared_inc: "data/gfx/fx_20/shared.inc", 7847, 23192,
        "48bce141f85037671509b6ff60d9bd85359e81133c58e4ca7e2afcd93a929314";
    t08_snow_dds: "data/map/textures/z_snow_00.dds", 100, 184,
        "be89d064a18e4c3b8c3da29d4c853f2052152f1ee13938daba146d0dee73bf13";
    t09_eye_table_tga: "data/character/colormaps/eye_table.tga", 104, 92,
        "898ac4591172ddbb1615819c561e8479a5ce4a25661c08c60f4dee67e08e0801";
    t10_campaign01_s2m: T10, 610653, 1488768,
        "a5403060eba0b55483d0f38661c68c46534e021684c70055d67ea7ace0349b6b";
}

// S8
// spec: FMT-RC-1.1 (S8)
#[test]
fn s08_streets_header() {
    let Some(wrapped) = read_game_file(T1) else {
        return;
    };
    let header = rc00::parse_header(&wrapped).unwrap();
    assert_eq!(header.data_crc, 0x2F0F_DB76);
    assert_eq!(header.key_crc, 0xF6F4_A1B8);
    assert_eq!(header.plain_size, 229);
}

// S9
// spec: FMT-RC-1.5, FMT-RC-2.4 (S9)
#[test]
fn s09_renamed_script_fails_key_check() {
    let Some(wrapped) = read_game_file(T1) else {
        return;
    };
    assert!(matches!(
        rc00::decode("roads.lua", &wrapped),
        Err(Rc00Error::KeyCrcMismatch {
            expected: 0xF6F4_A1B8,
            ..
        })
    ));
}

// S10
// spec: FMT-RC-2.3, FMT-RC-2.5 (S10)
#[test]
fn s10_renamed_map_still_decodes() {
    let Some(wrapped) = read_game_file(T10) else {
        return;
    };
    let plain = rc00::decode("anything.s2m", &wrapped).unwrap();
    assert_eq!(
        hex(&sha256(&plain)),
        "a5403060eba0b55483d0f38661c68c46534e021684c70055d67ea7ace0349b6b"
    );
}

// S11
// spec: FMT-RC-1.3, FMT-RC-4.2 (S11)
#[test]
fn s11_streets_first_bytes() {
    let Some(wrapped) = read_game_file(T1) else {
        return;
    };
    let plain = rc00::decode(T1, &wrapped).unwrap();
    assert!(plain.starts_with(b"\r\n-------------- Streets"));
}

// spec: none (the tests' own SHA-256)
#[test]
fn sha256_helper_matches_known_digests() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // Two blocks after padding.
    assert_eq!(
        hex(&sha256(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}
