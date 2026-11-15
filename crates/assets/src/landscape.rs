//! The `LandscapeElements` table of `settings/terrain_static_data.xml`
//! (`specs/GFX-objects.md` GFX-OBJ-1.2 to 1.5): the model, shader and
//! random rotation and size of trees, stones, fields, doodads and the road
//! flag, by name.

use crate::xml;

/// GFX-OBJ-1.2: the element's `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Flag,
    Tree,
    Stone,
    WheatField,
    Gate,
    Doodad,
}

/// GFX-OBJ-1.2: one `LandscapeElement`.
#[derive(Debug, Clone, PartialEq)]
pub struct LandscapeElement {
    pub name: String,
    pub mesh: String,
    pub shader: String,
    /// Yaw range in degrees.
    pub rot: [f32; 2],
    pub scale_x: [f32; 2],
    pub scale_y: [f32; 2],
    pub scale_z: [f32; 2],
    pub offset: [f32; 2],
    pub kind: ElementType,
    /// 0 alpha test, 1 alpha test and blending, 2 additive.
    pub blend_mode: u32,
    pub align_on_water: bool,
    /// GFX-VIEW-2.5a: `shadowCast`, true unless "false" (grass and small
    /// plants cast none).
    pub shadow_cast: bool,
}

fn pair(e: &xml::Element, key: &str, default: [f32; 2]) -> [f32; 2] {
    let Some(text) = e.get(key) else {
        return default;
    };
    let v: Vec<f32> = text
        .split_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    match v[..] {
        [a, b, ..] => [a, b],
        [a] => [a, a],
        [] => default,
    }
}

/// GFX-OBJ-1.2: the elements in file order; a repeated name is ignored.
pub fn read(text: &str) -> Vec<LandscapeElement> {
    let mut out: Vec<LandscapeElement> = Vec::new();
    for e in xml::elements(text) {
        if e.tag != "LandscapeElement" {
            continue;
        }
        let (Some(name), Some(mesh)) = (e.get("name"), e.get("mesh")) else {
            continue;
        };
        if out.iter().any(|o| o.name == name) {
            continue;
        }
        let kind = match e.get("type").map(str::trim) {
            Some("flag") => ElementType::Flag,
            Some("tree") => ElementType::Tree,
            Some("stone") => ElementType::Stone,
            Some("wheatField") => ElementType::WheatField,
            Some("gate") => ElementType::Gate,
            _ => ElementType::Doodad,
        };
        out.push(LandscapeElement {
            name: name.to_owned(),
            mesh: mesh.to_owned(),
            shader: e.get("shader").unwrap_or_default().trim().to_owned(),
            rot: pair(&e, "rot", [0.0, 0.0]),
            scale_x: pair(&e, "scaleX", [1.0, 1.0]),
            scale_y: pair(&e, "scaleY", [1.0, 1.0]),
            scale_z: pair(&e, "scaleZ", [1.0, 1.0]),
            offset: pair(&e, "offset", [0.0, 0.0]),
            kind,
            blend_mode: e
                .get("blendMode")
                .and_then(|b| b.trim().parse().ok())
                .unwrap_or(0),
            align_on_water: e
                .get("alignOnWater")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true")),
            shadow_cast: !e
                .get("shadowCast")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("false")),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r#"<terrain_static_data><Patterns><Pattern name="x"/></Patterns>
    <LandscapeElements>
        <LandscapeElement name = "z_flag_00" mesh = "data\map\objects\flag.gr2"
            shader = "flag_shader.fx" rot = "90 90" type = "flag"/>
        <LandscapeElement name = "tree_14" mesh = "data\map\objects\RS_TreeCherryBlossom01.GR2"
            shader = "tree_shader.fx" rot = "0 360" scaleX = "0.6 0.8" scaleZ = "0.6 0.8"
            scaleY = "0.6 0.8" type = "tree"/>
        <LandscapeElement name = "Fern01" mesh = "f.GR2" shader = "doodads_shader.fx" blendMode="1"
            shadowCast="false"/>
        <LandscapeElement name = "tree_14" mesh = "other.GR2"/>
    </LandscapeElements></terrain_static_data>"#;

    /// GFX-OBJ T1: tree_14 is the cherry blossom with the tree shader,
    /// any yaw and 0.6..0.8; the first of a name wins; defaults.
    // spec: GFX-OBJ-1.2
    #[test]
    fn elements() {
        let list = read(TEXT);
        assert_eq!(list.len(), 3);
        let t = &list[1];
        assert!(t.mesh.ends_with("RS_TreeCherryBlossom01.GR2"));
        assert_eq!(
            (t.shader.as_str(), t.kind),
            ("tree_shader.fx", ElementType::Tree)
        );
        assert_eq!((t.rot, t.scale_y), ([0.0, 360.0], [0.6, 0.8]));
        assert_eq!(list[0].rot, [90.0, 90.0]);
        let fern = &list[2];
        assert_eq!((fern.kind, fern.blend_mode), (ElementType::Doodad, 1));
        assert_eq!((fern.scale_x, fern.offset), ([1.0, 1.0], [0.0, 0.0]));
        // GFX-VIEW-2.5a: casts a shadow unless `shadowCast="false"`.
        assert!(t.shadow_cast && !fern.shadow_cast);
    }
}
