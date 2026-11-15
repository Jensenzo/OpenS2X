//! Bitmap fonts in the AngelCode BMFont text format, as the game ships them
//! (`data/language/<lang>/fonts/*.fnt` with `_00.tga` pages,
//! `specs/UI-ingame.md` UI-GAME-1.7). The standard format; nothing game
//! specific.

use std::collections::HashMap;

/// One glyph: its rectangle on the page, its drawing offset and advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Glyph {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub x_advance: i32,
    pub page: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Font {
    /// `info size`.
    pub size: i32,
    pub line_height: i32,
    pub base: i32,
    /// Page file names, by page number.
    pub pages: Vec<String>,
    pub glyphs: HashMap<u32, Glyph>,
    /// Kerning amounts by (first, second) character.
    pub kernings: HashMap<(u32, u32), i32>,
}

/// `key=value` pairs of a line; values may be quoted.
fn pairs(line: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut rest = line;
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq]
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_owned();
        let after = &rest[eq + 1..];
        let (value, next) = if let Some(quoted) = after.strip_prefix('"') {
            let end = quoted.find('"').unwrap_or(quoted.len());
            (&quoted[..end], &quoted[(end + 1).min(quoted.len())..])
        } else {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            (&after[..end], &after[end..])
        };
        out.insert(key, value.to_owned());
        rest = next;
    }
    out
}

impl Font {
    pub fn parse(text: &str) -> Self {
        let mut font = Font::default();
        for line in text.lines() {
            let tag = line.split_whitespace().next().unwrap_or("");
            let p = pairs(line);
            let int = |k: &str| {
                p.get(k)
                    .and_then(|v| v.trim().parse::<i32>().ok())
                    .unwrap_or(0)
            };
            match tag {
                "info" => font.size = int("size"),
                "common" => {
                    font.line_height = int("lineHeight");
                    font.base = int("base");
                }
                "page" => {
                    let id = int("id").max(0) as usize;
                    if font.pages.len() <= id {
                        font.pages.resize(id + 1, String::new());
                    }
                    font.pages[id] = p.get("file").cloned().unwrap_or_default();
                }
                "char" => {
                    font.glyphs.insert(
                        int("id").max(0) as u32,
                        Glyph {
                            x: int("x").max(0) as u32,
                            y: int("y").max(0) as u32,
                            width: int("width").max(0) as u32,
                            height: int("height").max(0) as u32,
                            x_offset: int("xoffset"),
                            y_offset: int("yoffset"),
                            x_advance: int("xadvance"),
                            page: int("page").max(0) as u32,
                        },
                    );
                }
                "kerning" => {
                    font.kernings.insert(
                        (int("first").max(0) as u32, int("second").max(0) as u32),
                        int("amount"),
                    );
                }
                _ => {}
            }
        }
        font
    }

    /// The width in pixels of `text` on one line: the sum of its
    /// characters' advances. The game keeps no kerning pairs (UI-MENU-1.8a),
    /// so `kernings` is read but not applied.
    pub fn width(&self, text: &str) -> i32 {
        text.chars()
            .map(|c| self.glyphs.get(&(c as u32)).map_or(0, |g| g.x_advance))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "info face=\"Garamond BookCondensed\" size=16 bold=0 padding=3,3,3,3 spacing=1,1
common lineHeight=16 base=13 scaleW=256 scaleH=256 pages=1
page id=0 file=\"light_system_00.tga\"
chars count=2
char id=32   x=0     y=0     width=1     height=0     xoffset=0     yoffset=16    xadvance=3     page=0
char id=65   x=145   y=127   width=8     height=16    xoffset=-2    yoffset=0     xadvance=7     page=0
kerning first=65 second=65 amount=-1
";

    // spec: UI-GAME-1.7, UI-MENU-1.8
    #[test]
    fn parses_a_font() {
        let f = Font::parse(TEXT);
        assert_eq!((f.size, f.line_height, f.base), (16, 16, 13));
        assert_eq!(f.pages, ["light_system_00.tga"]);
        let a = f.glyphs[&65];
        assert_eq!(
            (a.x, a.y, a.width, a.x_offset, a.x_advance),
            (145, 127, 8, -2, 7)
        );
        assert_eq!(f.width("A A"), 7 + 3 + 7);
        // UI-MENU-1.8a: the pen advances by xadvance only, no kerning.
        assert_eq!(f.width("AA"), 14);
    }
}
