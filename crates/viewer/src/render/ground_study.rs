//! A development switch, not a hook: a study of the ground cut from flat
//! planes (`--ground-study <variant>=<palette.json>`, off by default). Every
//! triangle of the terrain mesh is one flat colour, a project palette's
//! swatch chosen by the terrain kind (the pattern's rules name, [`TABLE`]),
//! lit by its own normal, its lightness stepped a little from its
//! neighbours' by its place on the map; the water is flat facets in the
//! palette's water colours. Roads, borders, objects and everything else are
//! drawn as ever. No rule of the original's is involved: this is a picture
//! for deciding whether a hook is wanted.

use std::collections::HashMap;
use std::sync::OnceLock;

/// How the facets are cut and stepped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Each triangle takes the kind of the field nearest to its middle:
    /// the edge between two kinds runs hard along the triangles' edges; the
    /// lightness step small.
    Hard = 1,
    /// Each triangle takes the field triangle's three kinds blended by its
    /// middle's place in it (still one colour per triangle); the step small.
    Blend = 2,
    /// As `Hard`, with a clear step.
    Clear = 3,
    /// Round 2: each triangle one kind of its field triangle's three, drawn
    /// by their weights at its middle and a hash of its place (two kinds
    /// meet in an irregular line of whole triangles); the step per patch of
    /// three to eight neighbouring triangles. The water one see-through
    /// surface over the ground.
    Ragged = 4,
    /// As `Ragged` without any step.
    RaggedFlat = 5,
    /// Round 2: no facets on the ground: the kinds mixed softly across each
    /// field triangle, lit by the smooth normal, no step, no pattern; the
    /// water as `Ragged`'s.
    Calm = 6,
    /// Round 3: the colour as `Calm` (the kinds mixed softly across each
    /// field triangle, no step, no patches), the light as the faceted
    /// variants (each triangle lit by its own normal); the water deeper and
    /// greener, see-through further out, a thin pale edge at the land.
    Hybrid = 7,
    /// As `Hybrid`, but kinds of different families ([`family_of`]) meet in
    /// a line of whole triangles as in `Ragged`; kinds of one family mix
    /// softly.
    HybridCrisp = 8,
}

impl Variant {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "hard" => Some(Self::Hard),
            "blend" => Some(Self::Blend),
            "clear" => Some(Self::Clear),
            "ragged" => Some(Self::Ragged),
            "ragged-flat" => Some(Self::RaggedFlat),
            "calm" => Some(Self::Calm),
            "hybrid" => Some(Self::Hybrid),
            "hybrid-crisp" => Some(Self::HybridCrisp),
            _ => None,
        }
    }

    /// The lightness step between neighbouring triangles (`Ragged`:
    /// patches): each is one of 1 - s, 1, 1 + s.
    pub fn step(self) -> f32 {
        match self {
            Self::Hard | Self::Blend => 0.035,
            Self::Clear => 0.09,
            Self::Ragged => 0.03,
            Self::RaggedFlat | Self::Calm | Self::Hybrid | Self::HybridCrisp => 0.0,
        }
    }
}

/// The study as switched on: the variant and the palette's swatches by name
/// (display colours, 0 to 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Study {
    pub variant: Variant,
    pub swatches: HashMap<String, [f32; 3]>,
}

static STUDY: OnceLock<Option<Study>> = OnceLock::new();

/// Switch the study on for this process (the first call wins).
pub fn install(study: Option<Study>) {
    let _ = STUDY.set(study);
}

/// The study, when switched on.
pub fn study() -> Option<&'static Study> {
    STUDY.get().and_then(Option::as_ref)
}

/// `--ground-study <variant>=<palette.json>`: the palette file read.
pub fn from_option(text: &str) -> Result<Study, String> {
    let bad = || {
        format!(
            "--ground-study {text}: \
             <hard|blend|clear|ragged|ragged-flat|calm|hybrid|hybrid-crisp>=<palette.json>"
        )
    };
    let (variant, path) = text.split_once('=').ok_or_else(bad)?;
    let variant = Variant::parse(variant).ok_or_else(bad)?;
    let json = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut swatches = swatches(&json);
    if swatches.is_empty() {
        return Err(format!("{path}: no swatches"));
    }
    if variant as u32 >= Variant::Ragged as u32 {
        swatches.extend(own_colours());
    }
    Ok(Study { variant, swatches })
}

/// Round 2's colours of the study's own (`ground_study_colours.txt`: a
/// swatch name and `#RRGGBB` per line, `#` comments), put over the
/// palette's swatches of the same name for the ground only.
pub const OWN_COLOURS: &str = include_str!("ground_study_colours.txt");

pub fn own_colours() -> HashMap<String, [f32; 3]> {
    OWN_COLOURS
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let name = words.next()?;
            let hex = words.next()?.strip_prefix('#')?;
            let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
            Some((
                name.to_owned(),
                [byte(0)?, byte(2)?, byte(4)?].map(|c| f32::from(c) / 255.0),
            ))
        })
        .collect()
}

/// The swatches of a palette file (`"name": "...", ... "base_colour":
/// "#RRGGBB"` in each entry of its `swatches` list), read without a JSON
/// library: each `"name"` takes the next `"base_colour"`.
pub fn swatches(json: &str) -> HashMap<String, [f32; 3]> {
    let value_after = |text: &str, key: &str| -> Option<(String, usize)> {
        let at = text.find(key)? + key.len();
        let rest = &text[at..];
        let open = rest.find('"')? + 1;
        let close = open + rest[open..].find('"')?;
        Some((rest[open..close].to_owned(), at + close + 1))
    };
    let mut out = HashMap::new();
    let mut text = json;
    while let Some((name, end)) = value_after(text, "\"name\":") {
        text = &text[end..];
        let Some((colour, end)) = value_after(text, "\"base_colour\":") else {
            break;
        };
        // The next entry's name must not come before this colour.
        if text.find("\"name\":").is_some_and(|n| n < end) {
            continue;
        }
        text = &text[end..];
        let hex = colour.trim_start_matches('#');
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        if let (Some(r), Some(g), Some(b)) = (byte(0), byte(2), byte(4)) {
            out.insert(name, [r, g, b].map(|c| f32::from(c) / 255.0));
        }
    }
    out
}

/// The terrain kinds by the words of their rules names (`pt_setName`), the
/// first rule that matches wins, each with the swatches it is drawn with
/// (more than one: their plain mean, where the palette has nothing
/// fitting).
pub const TABLE: &[(&str, &[&str])] = &[
    ("LAVA rock floating", &["ember", "soot"]),
    ("LAVA Sand", &["sand", "soot"]),
    ("LAVA Meadow", &["leaf_olive", "soot"]),
    ("LAVA rock", &["charcoal_glint"]),
    ("LAVA 0", &["ember"]),
    ("LAVA", &["soot"]),
    ("seaground plants", &["sand", "leaf_olive"]),
    ("seaground sand", &["sand"]),
    ("seaground", &["stone_dark", "sand"]),
    ("swamp water", &["water_pale", "leaf_olive"]),
    ("swamp meadow", &["leaf_olive"]),
    ("swamp", &["earth", "leaf_olive"]),
    ("water", &["seabed"]),
    ("snow", &["snow"]),
    ("rocky plants", &["rock", "turf"]),
    ("rocky earth dark", &["rock_dark", "earth_dark"]),
    ("rocky earth", &["rock", "earth"]),
    ("rock red", &["terracotta_dark"]),
    ("stone ground", &["rock_light"]),
    ("rock", &["rock"]),
    ("Desert meadow", &["turf_dry"]),
    ("Desert earth", &["earth", "sand"]),
    ("Desert", &["sand"]),
    ("sand", &["sand"]),
    ("meadow ground", &["turf_dry"]),
    ("meadow yellow", &["turf", "grain"]),
    ("meadow red", &["turf"]),
    ("meadow dark", &["leaf"]),
    ("meadow leaf", &["leaf_light"]),
    ("meadow bright", &["leaf_light", "turf"]),
    ("MED meadow", &["turf_dry"]),
    ("meadow", &["turf"]),
    ("leaf", &["leaf"]),
    ("MED ground 01", &["earth_dark"]),
    ("ground", &["earth"]),
    ("earth", &["earth"]),
    ("DO NOT USE", &["earth"]),
    ("HARBOR", &["rock_light"]),
    ("border", &["stone_dark"]),
];

/// Round 3 (`HybridCrisp`): the families of terrain kinds. Kinds of one
/// family mix softly; kinds of two families meet in a line of whole
/// triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Meadows of every kind, the swamp, the desert's and the lava's meadow.
    Meadow = 0,
    /// Sand, the desert, the sea ground and the ground under water.
    Sand = 1,
    /// Rock, stone ground, the rocky kinds, the harbour ground, the border.
    Rock = 2,
    Snow = 3,
    /// Earth and plain ground (the desert's earth too).
    Earth = 4,
    /// The lava and its rock.
    Lava = 5,
    /// A kind no row names.
    Other = 6,
}

/// The family of a terrain kind by its rules name, by the same rows as
/// [`TABLE`] (the first that matches).
pub fn family_of(name: &str) -> Family {
    let Some((words, _)) = row_of(name) else {
        return Family::Other;
    };
    match *words {
        "LAVA Sand" => Family::Sand,
        "LAVA Meadow" => Family::Meadow,
        w if w.starts_with("LAVA") => Family::Lava,
        w if w.starts_with("seaground") || w == "water" => Family::Sand,
        w if w.starts_with("swamp") => Family::Meadow,
        "snow" => Family::Snow,
        "rocky plants" | "rocky earth dark" | "rocky earth" | "rock red" | "stone ground"
        | "rock" | "HARBOR" | "border" => Family::Rock,
        "Desert meadow" => Family::Meadow,
        "Desert earth" => Family::Earth,
        "Desert" | "sand" => Family::Sand,
        w if w.contains("meadow") || w == "leaf" => Family::Meadow,
        _ => Family::Earth,
    }
}

/// Round 3's water (`Hybrid`, `HybridCrisp`): the middle and the deep a
/// darker, greener teal than the palette's (whose deep reads like the sky);
/// the shallow the palette's.
pub const HYBRID_WATER: [[f32; 3]; 2] = [[0.200, 0.400, 0.380], [0.090, 0.235, 0.225]];

/// The water's colours for `variant`: [`water_colours`], with round 3's
/// middle and deep for the hybrids.
pub fn water_colours_for(variant: Variant, swatches: &HashMap<String, [f32; 3]>) -> [[f32; 3]; 3] {
    let mut out = water_colours(swatches);
    if matches!(variant, Variant::Hybrid | Variant::HybridCrisp) {
        out[1] = HYBRID_WATER[0];
        out[2] = HYBRID_WATER[1];
    }
    out
}

/// The table's row for a kind's rules name (none: [`FALLBACK`]).
pub fn row_of(name: &str) -> Option<&'static (&'static str, &'static [&'static str])> {
    TABLE.iter().find(|(words, _)| name.contains(words))
}

/// The swatch of a kind not in the table, or of a swatch missing from the
/// palette: a plain mid grey.
pub const FALLBACK: [f32; 3] = [0.5, 0.5, 0.5];

/// The colour of a terrain kind by its rules name: its row's swatches'
/// mean (those the palette has).
pub fn colour_of(name: &str, swatches: &HashMap<String, [f32; 3]>) -> [f32; 3] {
    let Some((_, names)) = row_of(name) else {
        return FALLBACK;
    };
    let found: Vec<[f32; 3]> = names
        .iter()
        .filter_map(|n| swatches.get(*n))
        .copied()
        .collect();
    if found.is_empty() {
        return FALLBACK;
    }
    let n = found.len() as f32;
    [0, 1, 2].map(|k| found.iter().map(|c| c[k]).sum::<f32>() / n)
}

/// The water's three colours, shallow to deep: the palette's `water_pale`,
/// `water` and `water_deep`, or a plain blue of ours where one is missing.
pub fn water_colours(swatches: &HashMap<String, [f32; 3]>) -> [[f32; 3]; 3] {
    let plain = [[0.55, 0.72, 0.80], [0.30, 0.52, 0.66], [0.18, 0.36, 0.50]];
    let mut out = plain;
    for (k, name) in ["water_pale", "water", "water_deep"].iter().enumerate() {
        if let Some(c) = swatches.get(*name) {
            out[k] = *c;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: &str = r##"{ "version": 2, "swatches": [
      { "name": "turf", "column": 3, "base_colour": "#72803F", "roughness": 0.9 },
      { "name": "rock", "base_colour": "#8C8573" },
      { "name": "earth", "base_colour": "#6B5540" },
      { "name": "water", "base_colour": "#4F86A6" } ], "free_cells": [] }"##;

    // spec: none (the ground study: a development switch, not a hook; the palette file read by name)
    #[test]
    fn reads_the_swatches() {
        let s = swatches(PALETTE);
        assert_eq!(s.len(), 4);
        assert_eq!(s["turf"], [0x72, 0x80, 0x3F].map(|c| c as f32 / 255.0));
        assert_eq!(s["water"][2], 0xA6 as f32 / 255.0);
    }

    // spec: none (the ground study: each terrain kind its swatch, a mean where the palette has nothing fitting)
    #[test]
    fn terrain_kinds_take_their_swatches() {
        let s = swatches(PALETTE);
        assert_eq!(colour_of("meadow", &s), s["turf"]);
        assert_eq!(colour_of("!!!MED rock big", &s), s["rock"]);
        let mix = colour_of("(RES rocky earth)", &s);
        for k in 0..3 {
            assert!((mix[k] - (s["rock"][k] + s["earth"][k]) / 2.0).abs() < 1e-6);
        }
        // No such swatch in this palette, no row: the grey.
        assert_eq!(colour_of("snow", &s), FALLBACK);
        assert_eq!(colour_of("something new", &s), FALLBACK);
        // The rows' order: the lava sand is sand and soot, not lava.
        assert_eq!(
            row_of("((00 LAVA Sand 00").expect("a row").1,
            &["sand", "soot"]
        );
        assert_eq!(
            row_of("seaground rock").expect("a row").1,
            &["stone_dark", "sand"]
        );
        let water = water_colours(&s);
        assert_eq!(water[1], s["water"]);
        assert_ne!(water[0], s["water"]);
        assert_eq!(Variant::parse("blend"), Some(Variant::Blend));
        assert!(Variant::Clear.step() > Variant::Hard.step());
    }

    // spec: none (the ground study, round 2: the study's own colours over the palette's, the new variants)
    #[test]
    fn round_two_colours_and_variants() {
        let own = own_colours();
        let hex = |r: u8, g: u8, b: u8| [r, g, b].map(|c| f32::from(c) / 255.0);
        assert_eq!(own["rock"], hex(0x8C, 0x85, 0x73));
        assert_eq!(own["snow"], hex(0x9A, 0xA8, 0xB4));
        assert_eq!(own.len(), 9);
        // The snow's lit planes stay below white under the day's light
        // (sun 0.9375 0.898 0.781 full on, ambient 0.6 0.5 0.6).
        let sun = [0.9375, 0.898, 0.781];
        let ambient = [0.6, 0.5, 0.6];
        for k in 0..3 {
            assert!(own["snow"][k] * (sun[k] + ambient[k]) < 1.0);
        }
        // The meadow under the crowns' leaf_light (#6F9A48) in lightness.
        let light = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(light(own["turf"]) < light(hex(0x6F, 0x9A, 0x48)));
        for (name, variant, step) in [
            ("ragged", Variant::Ragged, 0.03),
            ("ragged-flat", Variant::RaggedFlat, 0.0),
            ("calm", Variant::Calm, 0.0),
        ] {
            assert_eq!(Variant::parse(name), Some(variant));
            assert_eq!(variant.step(), step);
        }
        let s = swatches(PALETTE);
        assert_eq!(colour_of("water", &s), FALLBACK);
        assert_eq!(colour_of("water", &own), own["seabed"]);
    }

    // spec: none (the ground study, round 3: the hybrids, the families, the water)
    #[test]
    fn round_three_families_and_water() {
        assert_eq!(Variant::parse("hybrid"), Some(Variant::Hybrid));
        assert_eq!(Variant::parse("hybrid-crisp"), Some(Variant::HybridCrisp));
        assert_eq!(Variant::Hybrid.step(), 0.0);
        for (name, family) in [
            ("meadow 01", Family::Meadow),
            ("meadow dark", Family::Meadow),
            ("(MED meadow 02", Family::Meadow),
            ("swamp meadow", Family::Meadow),
            ("Desert meadow", Family::Meadow),
            ("sand", Family::Sand),
            ("seaground rock", Family::Sand),
            ("water", Family::Sand),
            ("((00 LAVA Sand 00", Family::Sand),
            ("!!!MED rock big", Family::Rock),
            ("(RES rocky earth)", Family::Rock),
            ("stone ground", Family::Rock),
            ("HARBOR", Family::Rock),
            ("snow", Family::Snow),
            ("earth", Family::Earth),
            ("Desert earth", Family::Earth),
            ("MED ground 01", Family::Earth),
            ("LAVA 0", Family::Lava),
            ("something new", Family::Other),
        ] {
            assert_eq!(family_of(name), family, "{name}");
        }
        // The deep water darker and greener than the plain deep blue.
        let plain = water_colours(&HashMap::new());
        let hybrid = water_colours_for(Variant::Hybrid, &HashMap::new());
        let light = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(light(hybrid[2]) < light(plain[2]));
        assert!(hybrid[2][1] > hybrid[2][2], "greener than blue");
        assert_eq!(hybrid[0], plain[0]);
        assert_eq!(water_colours_for(Variant::Calm, &HashMap::new()), plain);
    }
}
