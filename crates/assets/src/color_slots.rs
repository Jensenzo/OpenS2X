//! `character/color_slots.xml` (`specs/GFX-objects.md` GFX-OBJ-3.1): the
//! colour slots, the 16-step colour ramp of each slot and the colour maps
//! that give each of the 16 material indices a slot.

use crate::xml;

/// GFX-OBJ-3.1: how a slot's ramp is given.
#[derive(Debug, Clone, PartialEq)]
pub enum Ramp {
    /// One colour, all 16 steps alike.
    Solid([f32; 3]),
    /// A gradient from `dark` to `bright`.
    Gradient { bright: [f32; 3], dark: [f32; 3] },
    /// An image whose rows of 16 texels are alternative ramps; `rows` says
    /// how many (default: the image height).
    Texture { file: String, rows: Option<u32> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorRamp {
    pub slot: String,
    pub ramp: Ramp,
}

/// GFX-OBJ-3.1: a colour map: the slot of material index 1..16 (entry 0
/// is index 1).
#[derive(Debug, Clone, PartialEq)]
pub struct ColorMap {
    pub name: String,
    pub materials: [Option<String>; 16],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorSlots {
    pub ramps: Vec<ColorRamp>,
    pub maps: Vec<ColorMap>,
}

fn rgb(text: &str) -> Option<[f32; 3]> {
    let mut it = text.split_whitespace().map(|v| v.parse::<f32>().ok());
    Some([it.next()??, it.next()??, it.next()??])
}

impl ColorSlots {
    pub fn read(text: &str) -> Self {
        let mut out = Self::default();
        for e in xml::elements(text) {
            match e.tag.as_str() {
                "ColorRamp" => {
                    let Some(slot) = e.get("slot") else { continue };
                    let ramp = if let Some(file) = e.get("texture") {
                        Ramp::Texture {
                            file: file.to_owned(),
                            rows: e.get("rows").and_then(|r| r.trim().parse().ok()),
                        }
                    } else if let Some(c) = e.get("solid").and_then(rgb) {
                        Ramp::Solid(c)
                    } else if let (Some(bright), Some(dark)) =
                        (e.get("bright").and_then(rgb), e.get("dark").and_then(rgb))
                    {
                        Ramp::Gradient { bright, dark }
                    } else {
                        continue;
                    };
                    out.ramps.push(ColorRamp {
                        slot: slot.to_owned(),
                        ramp,
                    });
                }
                "ColorMap" => out.maps.push(ColorMap {
                    name: e.get("name").unwrap_or_default().to_owned(),
                    materials: Default::default(),
                }),
                "Material" => {
                    let (Some(map), Some(index), Some(slot)) = (
                        out.maps.last_mut(),
                        e.get("index").and_then(|i| i.trim().parse::<usize>().ok()),
                        e.get("slot"),
                    ) else {
                        continue;
                    };
                    if (1..=16).contains(&index) {
                        map.materials[index - 1] = Some(slot.to_owned());
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn ramp(&self, slot: &str) -> Option<&Ramp> {
        self.ramps.iter().find(|r| r.slot == slot).map(|r| &r.ramp)
    }

    pub fn map(&self, name: &str) -> Option<&ColorMap> {
        self.maps.iter().find(|m| m.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: GFX-OBJ-3.1
    #[test]
    fn ramps_and_maps() {
        let s = ColorSlots::read(
            r#"<color_slots><ColorRamps>
            <ColorRamp slot="_empty_" solid="0.0 0.0 0.0"/>
            <ColorRamp slot="Hair" texture="data\c\hair_table.tga" rows="16"/>
            <ColorRamp slot="Skin" texture="data\c\skin_table.tga"/>
            </ColorRamps><ColorMaps><ColorMap name="default">
            <Material index="1" slot="Hair"/><Material index="16" slot="_empty_"/>
            </ColorMap></ColorMaps></color_slots>"#,
        );
        assert_eq!(s.ramp("_empty_"), Some(&Ramp::Solid([0.0; 3])));
        assert_eq!(
            s.ramp("Hair"),
            Some(&Ramp::Texture {
                file: r"data\c\hair_table.tga".into(),
                rows: Some(16)
            })
        );
        let m = s.map("default").unwrap();
        assert_eq!(m.materials[0].as_deref(), Some("Hair"));
        assert_eq!(m.materials[15].as_deref(), Some("_empty_"));
        assert_eq!(m.materials[1], None);
    }
}
