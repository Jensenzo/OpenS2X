//! The terrain pattern list `data/settings/terrain_static_data.xml`
//! (`specs/GFX-terrain-patterns.md`, GFX-PAT): which texture file and which
//! texture scale a terrain type's graphics name (`pa_setGfxName`) draws with.

/// One `Pattern` entry (GFX-PAT-1.1).
#[derive(Debug, Clone, PartialEq)]
pub struct PatternEntry {
    pub name: String,
    /// The `texture` attribute, a game-relative path with backslashes.
    pub texture: Option<String>,
    /// The `alias` attribute: the entry whose texture this one borrows.
    pub alias: Option<String>,
    /// `texScale`: the texture coordinate per world unit along a row and
    /// across the rows; `1 1` unless given (GFX-PAT-1.1).
    pub tex_scale: [f32; 2],
    /// The `normal` attribute: the normal map, a game-relative path.
    pub normal: Option<String>,
    /// `nrmScale`: the normal map's coordinate per world unit; `1 1` unless
    /// given (GFX-PAT-1.1).
    pub nrm_scale: [f32; 2],
    /// The render `type` (GFX-WATER-2.1): 0 unless given, 1 underwater
    /// ground.
    pub kind: u32,
    /// GFX-PAT-5.4: the `transition` value, the mask set it draws its
    /// transitions with; 0 unless given.
    pub transition: u32,
    /// GFX-PAT-1.1, 1.4: `specularIntensity` (0 unless given),
    /// `specularPower` (1 unless given) and `specular` (`1 1 1` unless
    /// given).
    pub specular_intensity: f32,
    pub specular_power: f32,
    pub specular: [f32; 3],
}

/// What a graphics name draws with (GFX-PAT-2).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPattern {
    /// The game-relative texture path, backslashes as in the file.
    pub texture: String,
    /// The pattern's **own** `texScale`, also when the texture is borrowed
    /// through an alias (GFX-PAT-2.3).
    pub tex_scale: [f32; 2],
    /// The normal map, borrowed with the texture through an alias
    /// (GFX-PAT-2.2), and the pattern's own `nrmScale` (GFX-PAT-2.3, 3.3).
    pub normal: Option<String>,
    pub nrm_scale: [f32; 2],
    /// The pattern's own render `type` (GFX-PAT-2.3, GFX-WATER-2.1).
    pub kind: u32,
    /// GFX-PAT-5.1: the entry's position in the list, its priority.
    pub priority: usize,
    /// GFX-PAT-5.4: the entry's own `transition` value.
    pub transition: u32,
    /// GFX-VIEW-2.4a: the entry's own highlight, intensity and power: I =
    /// `specularIntensity` x the first value of `specular`, clamped to 0..1
    /// (it travels in a colour's alpha), and `specularPower`. Only a type-3
    /// pattern draws it.
    pub specular: [f32; 2],
}

impl ResolvedPattern {
    /// GFX-PAT-3.1: the texture coordinate of the point `(u, v)` of the map
    /// plane (MAP-GRID-5.2), repeating.
    pub fn coordinate(&self, u: f32, v: f32) -> [f32; 2] {
        [u * self.tex_scale[0], v * self.tex_scale[1]]
    }

    /// GFX-PAT-3.2: world units one copy of the texture spans along a row
    /// and across the rows.
    pub fn repeat_units(&self) -> [f32; 2] {
        self.tex_scale.map(|s| 1.0 / s)
    }
}

/// The pattern list in file order, duplicates dropped (GFX-PAT-1.2).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerrainPatterns {
    pub entries: Vec<PatternEntry>,
}

impl TerrainPatterns {
    /// Read the text of the file. Comments are skipped; an entry without a
    /// `name`, or whose name was read before, is left out (GFX-PAT-1.2).
    pub fn parse(text: &str) -> Self {
        let text = crate::xml::strip_comments(text);
        let mut entries: Vec<PatternEntry> = Vec::new();
        let mut rest = text.as_str();
        while let Some(at) = rest.find("<Pattern") {
            rest = &rest[at + "<Pattern".len()..];
            // `<Patterns>` is the list, not an entry.
            if !rest.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let end = rest.find('>').unwrap_or(rest.len());
            let attributes = crate::xml::attributes(&rest[..end]);
            rest = &rest[end..];
            let get = |key: &str| {
                attributes
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
            };
            let Some(name) = get("name") else { continue };
            if entries.iter().any(|e| e.name == name) {
                continue;
            }
            entries.push(PatternEntry {
                name,
                texture: get("texture"),
                alias: get("alias"),
                tex_scale: get("texScale")
                    .and_then(|v| two_numbers(&v))
                    .unwrap_or([1.0, 1.0]),
                normal: get("normal"),
                nrm_scale: get("nrmScale")
                    .and_then(|v| two_numbers(&v))
                    .unwrap_or([1.0, 1.0]),
                kind: get("type").and_then(|v| v.trim().parse().ok()).unwrap_or(0),
                transition: get("transition")
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0),
                specular_intensity: get("specularIntensity")
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0.0),
                specular_power: get("specularPower")
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(1.0),
                specular: get("specular")
                    .and_then(|v| three_numbers(&v))
                    .unwrap_or([1.0, 1.0, 1.0]),
            });
        }
        Self { entries }
    }

    /// GFX-PAT-1.3, 2: the texture and scale of the entry named `gfx_name`.
    /// An alias takes the texture of the entry it names, which must have its
    /// own texture, and keeps its own scale. `None` for a name without an
    /// entry, or an alias without a texture to borrow.
    pub fn resolve(&self, gfx_name: &str) -> Option<ResolvedPattern> {
        let priority = self.entries.iter().position(|e| e.name == gfx_name)?;
        let entry = &self.entries[priority];
        let (texture, normal) = match (&entry.texture, &entry.alias) {
            (Some(texture), _) => (texture.clone(), entry.normal.clone()),
            (None, Some(alias)) => {
                let target = self.entries.iter().find(|e| &e.name == alias)?;
                (target.texture.clone()?, target.normal.clone())
            }
            (None, None) => return None,
        };
        Some(ResolvedPattern {
            texture,
            tex_scale: entry.tex_scale,
            normal,
            nrm_scale: entry.nrm_scale,
            kind: entry.kind,
            priority,
            transition: entry.transition,
            specular: [
                (entry.specular_intensity * entry.specular[0]).clamp(0.0, 1.0),
                entry.specular_power,
            ],
        })
    }
}

fn three_numbers(value: &str) -> Option<[f32; 3]> {
    let mut parts = value.split_whitespace().map(str::parse::<f32>);
    Some([
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ])
}

fn two_numbers(value: &str) -> Option<[f32; 2]> {
    let mut parts = value.split_whitespace().map(str::parse::<f32>);
    let first = parts.next()?.ok()?;
    let second = parts.next()?.ok()?;
    Some([first, second])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" ?>
<terrain_static_data>
  <GroundCover><Cover name = "GrassBlades" density = "0.3" /></GroundCover>
  <Patterns>
    <Pattern name = "z_rock_00" texture = "data\map\textures\z_rock_00.dds" texScale = "0.09 0.09"
        normal = "data\map\textures\rocks_normal.dds" nrmScale = "0.7 0.7" type = "3"
        specularIntensity = "0.1" specularPower = "4" />
    <Pattern name = "z_pavement_00" texture = "data\map\textures\pavement.dds" type = "3"
        specularIntensity = "0.6" specularPower = "5" specular = "1 0.5 0.5" ambient = "1.0 1.2 1.3" />
    <!-- <Pattern name = "z_gone" texture = "x.dds" /> -->
    <Pattern
        name   = "z_rock_01"
        alias  = "z_rock_00"
        texScale = "0.062 0.062"
    />
    <Pattern name = "z_rock_04" alias = "z_rock_00" texScale = "0.09 0.16" />
    <Pattern name = "z_rock_00" texture = "data\map\textures\other.dds" />
    <Pattern name = "z_plain" texture = "data\map\textures\plain.dds" />
    <Pattern name = "z_lost" alias = "z_missing" />
  </Patterns>
</terrain_static_data>"#;

    // GFX-PAT-1.1, 1.2: entries in file order, the first of a name wins,
    // comments are not entries, scale defaults to 1 1.
    #[test]
    fn parses_entries_in_order() {
        let list = TerrainPatterns::parse(SAMPLE);
        let names: Vec<&str> = list.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "z_rock_00",
                "z_pavement_00",
                "z_rock_01",
                "z_rock_04",
                "z_plain",
                "z_lost"
            ]
        );
        assert_eq!(
            list.entries[0].texture.as_deref(),
            Some(r"data\map\textures\z_rock_00.dds")
        );
        assert_eq!(list.entries[4].tex_scale, [1.0, 1.0]);
        // GFX-PAT-5.1: the list position is the priority.
        assert_eq!(list.resolve("z_plain").unwrap().priority, 4);
        assert_eq!(list.resolve("z_plain").unwrap().transition, 0);
    }

    // GFX-PAT T1, T4, 2.3: the alias borrows the texture and keeps its scale.
    #[test]
    fn alias_keeps_its_own_scale() {
        let list = TerrainPatterns::parse(SAMPLE);
        let rock_01 = list.resolve("z_rock_01").unwrap();
        assert_eq!(rock_01.texture, r"data\map\textures\z_rock_00.dds");
        assert_eq!(rock_01.tex_scale, [0.062, 0.062]);
        let rock_04 = list.resolve("z_rock_04").unwrap();
        assert_eq!(rock_04.tex_scale, [0.09, 0.16]);
        assert_eq!(list.resolve("z_rock_00").unwrap().tex_scale, [0.09, 0.09]);
    }

    // GFX-PAT-1.1, 2.2, 2.3: the normal map is borrowed with the texture;
    // nrmScale and type are the alias's own, with defaults 1 1 and 0.
    #[test]
    fn normal_map_scale_and_type() {
        let list = TerrainPatterns::parse(SAMPLE);
        let rock_00 = list.resolve("z_rock_00").unwrap();
        assert_eq!(
            rock_00.normal.as_deref(),
            Some(r"data\map\textures\rocks_normal.dds")
        );
        assert_eq!((rock_00.nrm_scale, rock_00.kind), ([0.7, 0.7], 3));
        let rock_01 = list.resolve("z_rock_01").unwrap();
        assert_eq!(rock_01.normal, rock_00.normal);
        assert_eq!((rock_01.nrm_scale, rock_01.kind), ([1.0, 1.0], 0));
        assert_eq!(list.resolve("z_plain").unwrap().normal, None);
    }

    // GFX-PAT-1.1, 1.4, GFX-VIEW-2.4a (T34): the highlight's intensity is
    // specularIntensity x the first value of `specular`, the power
    // specularPower; without them 0 and 1. An alias keeps its own (none).
    // spec: GFX-PAT-1.4, GFX-VIEW-2.4a (GFX-VIEW T34)
    #[test]
    fn specular_values() {
        let list = TerrainPatterns::parse(SAMPLE);
        assert_eq!(list.resolve("z_rock_00").unwrap().specular, [0.1, 4.0]);
        assert_eq!(list.resolve("z_pavement_00").unwrap().specular, [0.6, 5.0]);
        assert_eq!(list.entries[1].specular, [1.0, 0.5, 0.5]);
        assert_eq!(list.resolve("z_rock_01").unwrap().specular, [0.0, 1.0]);
        assert_eq!(list.resolve("z_plain").unwrap().specular, [0.0, 1.0]);
    }

    // GFX-PAT-1.3, 2.2: no entry, or an alias with nothing to borrow.
    #[test]
    fn unresolvable_names() {
        let list = TerrainPatterns::parse(SAMPLE);
        assert_eq!(list.resolve("z_nothing"), None);
        assert_eq!(list.resolve("z_lost"), None);
        assert_eq!(list.resolve("z_gone"), None);
    }

    // GFX-PAT T6, 3.1, 3.2: 6 world units along a row move the coordinate by
    // 0.39 on meadow; one copy of the texture spans 15.4 units.
    #[test]
    fn coordinates_follow_the_scale() {
        let pattern = ResolvedPattern {
            texture: String::new(),
            tex_scale: [0.065, 0.065],
            normal: None,
            nrm_scale: [1.0, 1.0],
            kind: 0,
            priority: 0,
            transition: 0,
            specular: [0.0, 1.0],
        };
        let a = pattern.coordinate(0.0, 0.0);
        let b = pattern.coordinate(6.0, 0.0);
        assert!((b[0] - a[0] - 0.39).abs() < 1e-6);
        let [along, across] = pattern.repeat_units();
        assert!((along - 15.38).abs() < 0.01 && (across - 15.38).abs() < 0.01);
        // 2.56 fields along a row (6 units each).
        assert!((along / 6.0 - 2.56).abs() < 0.01);
    }
}
