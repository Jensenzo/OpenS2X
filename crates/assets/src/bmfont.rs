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
            .map(|c| self.glyph(c).map_or(0, |g| g.x_advance))
            .sum()
    }

    /// FMT-3.8a: the glyph of a character. The shipped fonts are
    /// `charset="ANSI"`: their ids are Windows-1252 bytes, so a character
    /// is looked up by its byte (the texts' bytes, `lang::decode`); one the
    /// code page lacks has no glyph.
    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        let byte = crate::lang::cp1252_byte(c)?;
        self.glyphs.get(&u32::from(byte))
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

    /// FMT-3.8a (stand-ins list row 12): an `ANSI` font's ids are
    /// Windows-1252 bytes: the left double quote (byte 0x93) and the euro
    /// (0x80) find glyphs 147 and 128, typed or decoded from a text file; a
    /// character outside the code page finds none.
    // spec: FMT-3.8a
    #[test]
    fn glyphs_by_their_windows_1252_byte() {
        let f = Font::parse(
            "common lineHeight=16 base=13 pages=1
char id=128 x=0 y=0 width=6 height=9 xoffset=0 yoffset=3 xadvance=8 page=0
char id=147 x=8 y=0 width=5 height=4 xoffset=0 yoffset=2 xadvance=6 page=0
char id=150 x=16 y=0 width=7 height=1 xoffset=0 yoffset=8 xadvance=9 page=0
",
        );
        assert_eq!(f.glyph('\u{201c}').map(|g| g.x), Some(8));
        assert_eq!(f.glyph('\u{20ac}').map(|g| g.x), Some(0));
        let text = crate::lang::decode(&[0x93, 0x96, 0x80]);
        assert_eq!(f.width(&text), 6 + 9 + 8);
        assert_eq!(f.glyph('\u{3042}'), None);
        // A text read byte for byte as Latin-1 (its control character
        // U+0093) still finds the byte's glyph.
        assert_eq!(f.glyph('\u{93}').map(|g| g.x), Some(8));
    }

    /// FMT-3.8b T7 (Q-1088 a, E-ui-044; stand-ins list row 12): the gb
    /// `credits.txt` holds the byte 0x96 twice (in the shipped file on two
    /// lines, `cr170` and `cr501`, not on one as T7 reads); each draws glyph
    /// 150 in a font that has it (`light_system`), adding its advance (the
    /// fonts' extra spacing is 0, UI-MENU-1.8); in `mini` (glyphs up to 126)
    /// it draws nothing and adds no width. Skipped without `S2_GAME_DIR`.
    // spec: FMT-3.8b (FMT T7), FMT-3.8a
    #[test]
    fn t7_a_byte_without_a_glyph_vanishes() {
        let Ok(dir) = std::env::var("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let gb = std::path::Path::new(&dir).join("data/language/gb");
        let name = "data/language/gb/credits.txt";
        let raw = std::fs::read(gb.join("credits.txt")).expect("credits.txt");
        let plain = crate::rc00::decode(name, &raw).expect("the file unwrapped");
        let text = crate::lang::decode(&plain);
        let dash = crate::lang::cp1252_char(0x96);
        let lines: Vec<&str> = text.lines().filter(|l| l.contains(dash)).collect();
        assert_eq!(text.matches(dash).count(), 2, "the byte 0x96 twice");
        let font = |name: &str| {
            let fnt = std::fs::read(gb.join("fonts").join(name)).expect("the font");
            Font::parse(&String::from_utf8_lossy(&fnt))
        };
        let light = font("light_system.fnt");
        let glyph = light.glyphs.get(&150).expect("glyph 150");
        let mini = font("mini.fnt");
        assert!(mini.glyphs.keys().all(|&id| id <= 126));
        assert_eq!(light.glyph(dash), Some(glyph));
        assert_eq!(mini.glyph(dash), None);
        for line in lines {
            let n = line.matches(dash).count() as i32;
            let without: String = line.chars().filter(|&c| c != dash).collect();
            assert_eq!(
                light.width(line),
                light.width(&without) + n * glyph.x_advance
            );
            assert_eq!(mini.width(line), mini.width(&without));
        }
    }
}
