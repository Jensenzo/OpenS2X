//! Test cases of `specs/GFX-granny-format.md` against the player's own
//! install through `S2_GAME_DIR`. When it is unset each test prints a note
//! and passes without checking anything.
//!

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::game_dir;
use s2_assets::dds::{self, Format};
use s2_assets::gr2::{self, Compression, Gr2};
use s2_assets::rc00;

/// Every file under `data` with the extension `ext`, sorted.
fn files(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.join("data")];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(ext))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn unwrap(path: &Path) -> Vec<u8> {
    try_unwrap(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn try_unwrap(path: &Path) -> Result<Vec<u8>, rc00::Rc00Error> {
    let name = path.to_str().unwrap();
    let raw = std::fs::read(path).unwrap();
    if rc00::is_wrapped(&raw) {
        rc00::decode(name, &raw)
    } else {
        Ok(raw)
    }
}

/// GFX-GR2-1.1 to 1.4, 1.6 over every file: one container variant.
#[test]
fn headers_of_all_files() {
    let Some(dir) = game_dir() else { return };
    let all = files(dir.root(), "gr2");
    assert_eq!(all.len(), 884);
    let mut compression: BTreeMap<String, usize> = BTreeMap::new();
    for path in &all {
        let data = unwrap(path);
        let h = gr2::read_header(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!((h.header_size, h.format_version), (0x1B8, 6));
        assert_eq!(h.type_tag, 0x8000_0013);
        assert_eq!(h.sections.len(), 8);
        assert_eq!(h.total_size as usize, data.len());
        assert_eq!(h.sections[5].compression, Compression::None);
        for s in &h.sections {
            *compression
                .entry(format!("{:?}", s.compression))
                .or_default() += 1;
            // GFX-GR2-1.6: marshalling entries are for byte swapping only;
            // a little-endian reader ignores them.
        }
    }
    let expected = BTreeMap::from([("None".to_owned(), 884), ("Oodle1".to_owned(), 6188)]);
    assert_eq!(compression, expected);
}

/// GFX-GR2-5.4: the DDS files of the data.
#[test]
fn dds_census() {
    let Some(dir) = game_dir() else { return };
    let all = files(dir.root(), "dds");
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    let (mut not_dds, mut cube, mut mips, mut largest) = (0, 0, 0, 0);
    let mut unreadable = Vec::new();
    for path in &all {
        // FMT-RC-2: a non-ASCII base name gives no key yet (Q-021).
        let Ok(data) = try_unwrap(path) else {
            unreadable.push(path.file_name().unwrap().to_string_lossy().into_owned());
            continue;
        };
        let t = match dds::decode(&data) {
            Ok(t) => t,
            Err(dds::DdsError::NotDds) => {
                not_dds += 1;
                continue;
            }
            Err(e) => panic!("{}: {e}", path.display()),
        };
        assert_eq!(t.rgba.len(), (t.width * t.height * 4) as usize);
        let name = match t.format {
            Format::Dxt1 => "DXT1".to_owned(),
            Format::Dxt3 => "DXT3".to_owned(),
            Format::Dxt5 => "DXT5".to_owned(),
            // The spec counts the one 8-bit file (alpha only) as "8-bit".
            Format::Masks { bits: 8, .. } => "8-bit".to_owned(),
            Format::Masks { bits, masks } => {
                let alpha = if masks[3] != 0 { " with alpha" } else { "" };
                format!("{bits}-bit{alpha}")
            }
        };
        *formats.entry(name).or_default() += 1;
        cube += usize::from(t.cube_map);
        mips += usize::from(t.mip_levels > 1);
        largest = largest.max(t.width.max(t.height));
    }
    let expected: BTreeMap<String, usize> = [
        ("DXT1", 372),
        ("DXT5", 68),
        ("DXT3", 33),
        ("32-bit with alpha", 6),
        ("16-bit with alpha", 16),
        ("16-bit", 7),
        ("8-bit", 1),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    assert_eq!(all.len(), 504);
    assert_eq!(unreadable, ["help_gebäude.dds", "help_militär.dds"]);
    // The two files left out are among the spec's 503 DDS files.
    let counted: usize = formats.values().sum();
    assert_eq!(counted + not_dds + unreadable.len(), 504);
    let missing: usize = expected
        .iter()
        .map(|(f, n)| n - formats.get(f).copied().unwrap_or(0))
        .sum();
    assert_eq!(missing, unreadable.len());
    // Both are DXT5.
    assert_eq!(formats["DXT5"], 66);
    assert_eq!((not_dds, cube, largest), (1, 1, 1024));
    assert!((348..=350).contains(&mips), "{mips}");
}

fn read(rel: &str) -> Option<Gr2> {
    let dir = game_dir()?;
    let path = dir.root().join(rel);
    let raw = std::fs::read(&path).unwrap();
    Some(gr2::load(path.to_str().unwrap(), &raw).unwrap_or_else(|e| panic!("{rel}: {e}")))
}

/// GFX-GR2 T1, T8: every section of every file expands; the totals.
// spec: GFX-GR2-1.3, GFX-GR2-1.4, GFX-GR2-2.11 (T1, T8)
#[test]
fn t1_t8_all_files() {
    let Some(dir) = game_dir() else { return };
    let (mut meshes, mut vertices, mut bones, mut animations) = (0, 0, 0, 0);
    for path in files(dir.root(), "gr2") {
        let data = unwrap(&path);
        let raw = gr2::read_raw(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let g = Gr2::from_raw(&raw).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for m in &g.meshes {
            assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.count));
            vertices += m.vertices.count;
        }
        meshes += g.meshes.len();
        bones += g.skeletons.iter().map(|s| s.bones.len()).sum::<usize>();
        animations += g.animations.len();
    }
    assert_eq!(
        (meshes, vertices, bones, animations),
        (3903, 856_538, 1207, 376)
    );
}

/// GFX-GR2 T2
// spec: GFX-GR2-4.2, GFX-GR2-4.3 (T2)
#[test]
fn t2_roman_sawmill() {
    let Some(g) = read("data/buildings/PB_R_Sawmill01.GR2") else {
        return;
    };
    let summary: Vec<(&str, usize, usize, usize)> = g
        .meshes
        .iter()
        .map(|m| {
            (
                m.name.as_str(),
                m.vertices.count,
                m.triangle_count(),
                m.bone_bindings.len(),
            )
        })
        .collect();
    assert_eq!(summary, [("GROUND", 72, 32, 1), ("House", 4293, 2400, 33)]);
    assert_eq!(g.skeletons[0].name, "root");
    assert_eq!(g.skeletons[0].bones.len(), 25);
    let textures: Vec<&str> = g.textures.iter().map(|t| t.base_name()).collect();
    assert_eq!(
        textures,
        [
            "house_decal.dds",
            "PB_R_Sawmill01_L.dds",
            "PB_R_Sawmill01.dds"
        ]
    );
}

/// GFX-GR2 T3
// spec: GFX-GR2-4.4 (T3)
#[test]
fn t3_sawmill_working() {
    let Some(g) = read("data/buildings/PB_R_Sawmill01/working.GR2") else {
        return;
    };
    assert_eq!(g.animations.len(), 1);
    let a = &g.animations[0];
    assert_eq!((a.duration, a.time_step), (60.0, 0.02));
    assert_eq!(a.transform_track_count(), 21);
}

/// GFX-GR2 T4
// spec: GFX-GR2-4.2 (T4)
#[test]
fn t4_forester_mesh() {
    let Some(g) = read("data/character/meshes/asian_forester_mesh.GR2") else {
        return;
    };
    let m = &g.meshes[0];
    let layout: Vec<&str> = m.vertices.layout.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        layout,
        [
            "Position",
            "BoneWeights",
            "BoneIndices",
            "Normal",
            "DiffuseColor0",
            "TextureCoordinates0"
        ]
    );
    assert_eq!(
        (m.vertices.count, m.triangle_count(), m.groups.len()),
        (1222, 1898, 2)
    );
    assert_eq!(m.bone_bindings.len(), 44);
}

/// GFX-GR2 T5
// spec: GFX-GR2-4.4 (T5)
#[test]
fn t5_settler_idle() {
    let Some(g) = read("data/character/animations/settler_idle_01.gr2") else {
        return;
    };
    let a = &g.animations[0];
    assert!((a.duration - 3.333).abs() < 0.001);
    assert!((a.time_step - 1.0 / 24.0).abs() < 1e-6);
    assert_eq!(a.transform_track_count(), 44);
}

/// GFX-GR2 T6
// spec: GFX-GR2-4.2 (T6)
#[test]
fn t6_birch() {
    let Some(g) = read("data/map/objects/RS_TreeBirch01.GR2") else {
        return;
    };
    let m = &g.meshes[0];
    assert_eq!(
        (m.name.as_str(), m.vertices.count, m.triangle_count()),
        ("RS_TreeBirch01", 364, 246)
    );
    let textures: Vec<&str> = g.textures.iter().map(|t| t.base_name()).collect();
    assert_eq!(textures, ["RS_TreeBirch05.dds"]);
}

/// GFX-GR2 T7
// spec: GFX-GR2-4.3 (T7)
#[test]
fn t7_donkey() {
    let Some(g) = read("data/character/donkey/donkey_model.GR2") else {
        return;
    };
    assert_eq!(g.skeletons[0].name, "Bip01");
    assert_eq!(g.skeletons[0].bones.len(), 39);
    assert!(g.meshes.is_empty());
    assert!((g.art_tool.unwrap().units_per_meter - 39.37).abs() < 0.01);
}

/// GFX-GR2 T1 and T9: every section of all 884 files expands to its exact
/// size and the decoder ends 1 to 3 bytes before the stored size
/// (GFX-GR2-2.3, 2.10, 2.11).
#[test]
fn t1_all_sections_decode() {
    let Some(dir) = game_dir() else { return };
    let all = files(dir.root(), "gr2");
    assert_eq!(all.len(), 884);
    let (mut sections, mut short) = (0, [0usize; 4]);
    let mut layouts: BTreeMap<&str, usize> = BTreeMap::new();
    for path in &all {
        let data = unwrap(path);
        let h = gr2::read_header(&data).unwrap();
        for s in h
            .sections
            .iter()
            .filter(|s| s.compression == Compression::Oodle1 && s.stored_size > 0)
        {
            let stored = &data[s.data_offset as usize..][..s.stored_size as usize];
            let (out, used) = gr2::oodle1::decompress(
                stored,
                s.expanded_size as usize,
                s.first16 as usize,
                s.first8 as usize,
            )
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(out.len(), s.expanded_size as usize, "{}", path.display());
            short[s.stored_size as usize - used] += 1;
            let (e0, e1, e2) = (s.first16, s.first8, s.expanded_size);
            let parts = match (e0 > 0, e1 > e0, e2 > e1) {
                (true, false, true) => "0+2",
                (true, false, false) => "0",
                (false, true, false) => "1",
                (false, false, true) => "2",
                (true, true, false) => "0+1",
                _ => "other",
            };
            *layouts.entry(parts).or_default() += 1;
            sections += 1;
        }
    }
    assert_eq!(sections, 3134);
    // GFX-GR2-2.3: 1 byte short in 1156, 2 in 1966, 3 in 12 sections.
    assert_eq!(short, [0, 1156, 1966, 12]);
    // GFX-GR2-2.10: no section has all three parts.
    assert!(!layouts.contains_key("other"), "{layouts:?}");
}

/// (file, section, stored size, expanded size, first-16-bit, first-8-bit,
/// stored SHA-256, expanded SHA-256): GFX-GR2 T9 vectors.
type Vector = (
    &'static str,
    usize,
    usize,
    usize,
    usize,
    usize,
    &'static str,
    &'static str,
);

const T9: &[Vector] = &[
    (
        "data/map/objects/LO_LavaFog01.GR2",
        2,
        49,
        12,
        0,
        12,
        "6ccd588c9c8f2c846683b282b0f19341aae6946fb85c1b45cb462a4b97974ac2",
        "b44387414aea2efef768a8b3ad17f3fd6f8cdb77e82e92ebba0a7b33f1d94d9a",
    ),
    (
        "data/buildings/MilitaryFlag1.GR2",
        2,
        632,
        2208,
        720,
        2208,
        "475587878a5875642b49863280531b89d6fae0daeb1c57ef4f05db21c29f56ee",
        "12587081ab69534993e1699aa946a7594238e51aa2f03f00c106f36a23f1484e",
    ),
    (
        "data/buildings/PB_N_Well01.GR2",
        4,
        4451,
        12936,
        12936,
        12936,
        "947d8800c16cb48e7b64577014aeb97585d5e832344a3ca66e95596d3f543a31",
        "1189490fb7c4f4fba0cfd7e0d1a9b4405f5d0896f106891f5221ded160994715",
    ),
    (
        "data/character/animations/work_well.GR2",
        0,
        223919,
        381588,
        379036,
        379036,
        "ddb91b14ff6810f59944719a63519d9eb453b416b71fac4bbd219f473eafe5b6",
        "520bf8d3a86b5ba1a3351b94d8d3bd9624abe0a93e8deda98f2417d0c68afbce",
    ),
    (
        "data/buildings/PB_R_Sawmill01.GR2",
        0,
        3589,
        12616,
        8528,
        8528,
        "88cc43e947f1f7fb0e58f41b529d2f1cec18177f5a8ab1501fc8f514d6632ee0",
        "59d4964555ae688e3d58c1023d79013f642a6fd0d88cdc5c20421d2ff95fdff1",
    ),
    (
        "data/buildings/PB_R_Sawmill01.GR2",
        1,
        876,
        4320,
        4320,
        4320,
        "ac074ca76a2b8638ed8abf9ffca99f01deadb45b909b9505a58c29188aa6c3ec",
        "c1059cce22b7b6d2d21d1815d438bba9af260c07bea289221757597cc3eec1b3",
    ),
    (
        "data/buildings/PB_R_Sawmill01.GR2",
        2,
        211,
        384,
        384,
        384,
        "e1a361ff33b8feedcdfe82b0220c33de6aa14bac381bf23ed0bdab9451ca1d86",
        "70bae7003762850ea391a77abc656a95146c763c4ff67ac405315b3b77595047",
    ),
    (
        "data/buildings/PB_R_Sawmill01.GR2",
        3,
        105578,
        291936,
        0,
        0,
        "d84ea0e91a0b79695207299cfb467dfd61a9ac6e8a6a5df9b4f8b0d0a0266dd5",
        "5701b1bcb4f068efc330cf8d1d3189ddae1f0a5b7408526d07372bff3b6f1ae6",
    ),
    (
        "data/buildings/PB_R_Sawmill01.GR2",
        4,
        10102,
        28800,
        28800,
        28800,
        "0088655f6c37057e1053941690e692ebec3ba03e6d32a4b93dd0c9df8c2aabbf",
        "cb67b73bae2a7d47559503bb7dd2db34b10078413dff0b9a8a6ac86ffcf29276",
    ),
    (
        "data/buildings/PB_R_Sawmill01.GR2",
        6,
        1359,
        18304,
        18304,
        18304,
        "b0c389ea128e449b223eaf20358d2bd39e298cfd3d80864eabb5ef56ef91b3c8",
        "9accfad41d0bd0178c69de8ed3643e43a80ab95a3bbb5ba9e68e10ee5167a61b",
    ),
];

/// GFX-GR2 T9
// spec: GFX-GR2-1.3, GFX-GR2-2.11 (T9)
#[test]
fn t9_section_hashes() {
    let Some(dir) = game_dir() else { return };
    for &(file, section, stored_size, expanded, e0, e1, stored_hash, expanded_hash) in T9 {
        let data = unwrap(&dir.root().join(file));
        let h = gr2::read_header(&data).unwrap();
        let s = &h.sections[section];
        assert_eq!(
            (s.stored_size as usize, s.expanded_size as usize),
            (stored_size, expanded),
            "{file} s{section}"
        );
        assert_eq!(
            (s.first16 as usize, s.first8 as usize),
            (e0, e1),
            "{file} s{section}"
        );
        let stored = &data[s.data_offset as usize..][..stored_size];
        assert_eq!(
            common::hex(&common::sha256(stored)),
            stored_hash,
            "{file} s{section}"
        );
        let (out, _) = gr2::oodle1::decompress(stored, expanded, e0, e1)
            .unwrap_or_else(|e| panic!("{file} s{section}: {e}"));
        assert_eq!(
            common::hex(&common::sha256(&out)),
            expanded_hash,
            "{file} s{section}"
        );
    }
}
