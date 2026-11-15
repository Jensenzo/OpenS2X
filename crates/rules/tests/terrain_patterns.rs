//! Test cases T1 to T5 of `specs/GFX-terrain-patterns.md` against the
//! player's own install (`S2_GAME_DIR`; skipped when unset).

use s2_assets::terrain_patterns::TerrainPatterns;
use s2_assets::{rc00, GameDir, GameDirError};
use s2_rules::RuleSet;

const LIST: &str = "data/settings/terrain_static_data.xml";

fn install() -> Option<(GameDir, RuleSet, TerrainPatterns)> {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    let (rules, _) = s2_rules::load(&dir).unwrap_or_else(|e| panic!("{e}"));
    let path = s2_rules::resolve(dir.root(), LIST).unwrap();
    let data = std::fs::read(&path).unwrap();
    let plain = if rc00::is_wrapped(&data) {
        rc00::decode(path.to_str().unwrap(), &data).unwrap()
    } else {
        data
    };
    let list = TerrainPatterns::parse(&String::from_utf8_lossy(&plain));
    Some((dir, rules, list))
}

/// The file name (lower case) the texture path of a lookup ends in.
fn file_of(texture: &str) -> String {
    texture
        .rsplit(['\\', '/'])
        .next()
        .unwrap()
        .to_ascii_lowercase()
}

// GFX-PAT T1
// spec: GFX-PAT-2.2, GFX-PAT-2.3 (T1)
#[test]
fn t1_rock_01_borrows_rock_00_at_its_own_scale() {
    let Some((_, _, list)) = install() else {
        return;
    };
    let found = list.resolve("z_rock_01").unwrap();
    assert_eq!(file_of(&found.texture), "z_rock_00.dds");
    assert_eq!(found.tex_scale, [0.062, 0.062]);
}

// GFX-PAT T2
// spec: GFX-PAT-2.2 (T2)
#[test]
fn t2_l_rock_00_uses_l_ground_00() {
    let Some((_, _, list)) = install() else {
        return;
    };
    let found = list.resolve("l_rock_00").unwrap();
    assert_eq!(file_of(&found.texture), "l_ground_00.dds");
    assert_eq!(found.tex_scale, [0.085, 0.085]);
}

// GFX-PAT T3, 2.4
#[test]
fn t3_sand_01_is_an_alias_of_seaground() {
    let Some((_, _, list)) = install() else {
        return;
    };
    let found = list.resolve("z_sand_01").unwrap();
    assert_eq!(file_of(&found.texture), "z_seaground_00.dds");
    assert_eq!(found.tex_scale, [0.06, 0.06]);
}

// GFX-PAT T4
// spec: GFX-PAT-2.3, GFX-PAT-3.2 (T4)
#[test]
fn t4_rock_04_scales_differently_along_and_across() {
    let Some((_, _, list)) = install() else {
        return;
    };
    let found = list.resolve("z_rock_04").unwrap();
    assert_eq!(file_of(&found.texture), "z_rock_00.dds");
    assert_eq!(found.tex_scale, [0.09, 0.16]);
}

// GFX-PAT T5, 1.3: every gfx name of patterns.lua resolves to a file.
#[test]
fn t5_every_gfx_name_resolves_to_an_existing_file() {
    let Some((dir, rules, list)) = install() else {
        return;
    };
    let names: std::collections::BTreeSet<&str> = rules
        .patterns
        .iter()
        .filter_map(|p| p.gfx_name.as_deref())
        .collect();
    assert_eq!(names.len(), 73);
    for name in names {
        let found = list
            .resolve(name)
            .unwrap_or_else(|| panic!("{name}: no entry"));
        let path = found.texture.replace('\\', "/");
        let file = s2_rules::resolve(dir.root(), &path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(file.is_file(), "{name}: {}", file.display());
    }
}
