//! Natural colours for terrain types, chosen by the names the property scripts
//! give them (`pa_setName`, `pa_setGfxName`; `specs/LUA-properties-api.md`
//! LUA-PROP-7). Presentation only.

use std::collections::HashMap;

use s2_rules::RuleSet;

use crate::render::terrain::DepositKind;

/// Keyword rules, first match wins: every keyword of a rule must occur in the
/// lower-case name or texture name. Specific rules come before general ones.
const NATURAL: &[(&[&str], [u8; 3])] = &[
    (&["border"], [24, 24, 28]),
    (&["harbor"], [150, 146, 140]),
    (&["pavement"], [150, 146, 140]),
    (&["swamp", "water"], [70, 92, 72]),
    (&["swamp"], [92, 104, 58]),
    (&["floating lava"], [205, 80, 30]),
    (&["lava", "meadow"], [88, 96, 52]),
    (&["lava", "sand"], [92, 78, 66]),
    (&["lava", "rock"], [58, 50, 50]),
    (&["lava"], [70, 58, 52]),
    (&["snow"], [240, 243, 247]),
    (&["seaground", "rock"], [70, 96, 110]),
    (&["seaground", "sand"], [150, 160, 140]),
    (&["seaground"], [80, 120, 130]),
    (&["water"], [40, 90, 150]),
    (&["desert", "meadow"], [170, 160, 90]),
    (&["desert", "earth"], [176, 138, 90]),
    (&["desert"], [226, 200, 140]),
    (&["sand"], [214, 194, 140]),
    (&["rock", "red"], [150, 104, 88]),
    (&["(res)"], [124, 112, 96]),
    (&["rock"], [132, 132, 128]),
    (&["stone"], [140, 136, 124]),
    (&["acre"], [150, 120, 72]),
    (&["yellow flowers"], [128, 170, 70]),
    (&["red flowers"], [112, 150, 66]),
    (&["bright"], [128, 184, 84]),
    (&["dark"], [70, 124, 54]),
    (&["meadow ground"], [120, 122, 70]),
    (&["med", "meadow"], [138, 160, 88]),
    (&["leaf"], [96, 136, 56]),
    (&["meadow"], [100, 160, 72]),
    (&["earth"], [140, 108, 70]),
    (&["ground"], [128, 104, 72]),
];

/// The natural colour for a terrain type with this name and texture name, as
/// sRGB in 0..1, or `None` if no rule matches.
pub fn natural_color(name: &str, gfx_name: &str) -> Option<[f32; 3]> {
    let text = format!("{name} {gfx_name}").to_ascii_lowercase();
    NATURAL
        .iter()
        .find(|(keywords, _)| keywords.iter().all(|k| text.contains(k)))
        .map(|(_, rgb)| rgb.map(|c| f32::from(c) / 255.0))
}

/// Natural colours for every terrain type of the rule set that has one.
pub fn terrain_colors(rules: &RuleSet) -> HashMap<u32, [f32; 3]> {
    rules
        .patterns
        .iter()
        .filter_map(|pattern| {
            let name = pattern.name.as_deref().unwrap_or("");
            let gfx = pattern.gfx_name.as_deref().unwrap_or("");
            natural_color(name, gfx).map(|rgb| (pattern.id, rgb))
        })
        .collect()
}

/// Deposit types by the good they yield (`dp_setGood`, LUA-PROP-8): wood
/// means a tree, stone a stone pile.
pub fn deposit_kinds(rules: &RuleSet) -> HashMap<u32, DepositKind> {
    rules
        .deposits
        .iter()
        .map(|deposit| {
            let good = deposit
                .good
                .and_then(|id| rules.goods.get(id))
                .and_then(|good| good.name.as_deref())
                .unwrap_or("")
                .to_ascii_lowercase();
            let kind = match good.as_str() {
                "wood" => DepositKind::Tree,
                "stone" => DepositKind::Stone,
                _ => DepositKind::Other,
            };
            (deposit.id, kind)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(name: &str, gfx: &str) -> [u8; 3] {
        natural_color(name, gfx)
            .unwrap_or_else(|| panic!("{name}"))
            .map(|c| (c * 255.0).round() as u8)
    }

    // spec: none (the 2D viewer's colours)
    #[test]
    fn families_get_their_natural_colours() {
        let green = |c: [u8; 3]| c[1] > c[0] && c[1] > c[2];
        let blue = |c: [u8; 3]| c[2] > c[0];
        let grey = |c: [u8; 3]| c[0].abs_diff(c[1]) < 20 && c[1].abs_diff(c[2]) < 20;
        assert!(green(rgb("meadow", "z_meadow_00")));
        assert!(green(rgb("meadow yellow flowers", "z_meadow_01")));
        assert!(green(rgb("!!!MED meadow 00", "m_meadow_00")));
        assert!(blue(rgb("water", "z_water_00")));
        assert!(blue(rgb("seaground plants", "z_seaground_01")));
        assert!(grey(rgb("rock big", "z_rock_01")));
        assert_eq!(rgb("snow", "z_snow_00"), [240, 243, 247]);
        assert_eq!(rgb("§§Desert sand dune", "z_sand_02"), [226, 200, 140]);
        assert_eq!(rgb("swamp land", "z_swamp_00"), [92, 104, 58]);
    }

    // spec: none (the 2D viewer's colours)
    #[test]
    fn specific_rules_win_over_general_ones() {
        assert_ne!(rgb("swamp water", "z_swamp_01"), rgb("water", "z_water_00"));
        assert_ne!(rgb("((00 LAVA rock", "l_rock_00"), rgb("rock", "z_rock_00"));
        assert_ne!(
            rgb("!!!MED rock red", "m_rock_03"),
            rgb("rock", "z_rock_00")
        );
        assert_ne!(
            rgb("seaground", "z_seaground_00"),
            rgb("meadow ground", "z_ground_00")
        );
        // The texture name helps where the name says nothing.
        assert_eq!(rgb("DO NOT USE", "z_acre_00"), [150, 120, 72]);
    }

    // spec: none (the 2D viewer's colours)
    #[test]
    fn unknown_names_have_no_natural_colour() {
        assert_eq!(natural_color("xyzzy", "q_00"), None);
        assert_eq!(natural_color("", ""), None);
    }
}
