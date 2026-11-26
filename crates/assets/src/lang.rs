//! The language files `data/language/<lang>/controls.txt` and
//! `dialogs.txt` (`specs/UI-ingame.md` UI-GAME-1.3): one entry per line,
//! `!key<TAB>text`; a label starting with `!` is looked up, a key without an
//! entry is shown unchanged.

use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Texts {
    entries: HashMap<String, String>,
}

/// FMT-3.8a: the bytes as Windows-1252 text (the files are not UTF-8; a
/// file that is valid UTF-8 is taken as such). The fonts are keyed by the
/// same bytes ([`cp1252_byte`], `bmfont::Font::glyph`), so every byte of a
/// text draws the glyph of its own number.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| cp1252_char(b)).collect(),
    }
}

/// Windows-1252's characters for the bytes 0x80 to 0x9f; 0 where the code
/// page leaves a byte undefined (it stays the control character of its
/// number, as in Latin-1).
const CP1252_HIGH: [u16; 32] = [
    0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
    0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
];

/// The character of a Windows-1252 byte.
pub fn cp1252_char(b: u8) -> char {
    match b {
        0x80..=0x9f => match CP1252_HIGH[usize::from(b - 0x80)] {
            0 => char::from(b),
            c => char::from_u32(u32::from(c)).unwrap_or(char::from(b)),
        },
        _ => char::from(b),
    }
}

/// The Windows-1252 byte of a character, if the code page has it. The
/// characters U+0000 to U+00FF are their own number, the control
/// characters U+0080 to U+009F included, so a text read byte for byte as
/// Latin-1 keeps its bytes too.
pub fn cp1252_byte(c: char) -> Option<u8> {
    let code = u32::from(c);
    if code < 0x100 {
        return u8::try_from(code).ok();
    }
    CP1252_HIGH
        .iter()
        .position(|&h| u32::from(h) == code)
        .and_then(|i| u8::try_from(0x80 + i).ok())
}

impl Texts {
    pub fn parse(text: &str) -> Self {
        let mut entries = HashMap::new();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            let Some((key, value)) = line.split_once('\t') else {
                continue;
            };
            entries
                .entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
        Self { entries }
    }

    /// Add the entries of another file (the first entry of a key wins).
    pub fn extend(&mut self, other: Texts) {
        for (k, v) in other.entries {
            self.entries.entry(k).or_insert(v);
        }
    }

    /// UI-GAME-1.3: the text of a label: a `!key` with an entry gives the
    /// entry, anything else is shown unchanged.
    pub fn get<'a>(&'a self, label: &'a str) -> &'a str {
        if label.starts_with('!') {
            self.entries.get(label).map_or(label, String::as_str)
        } else {
            label
        }
    }

    /// The language text of a name key: building types, goods and jobs are
    /// keyed by their rules name without the `!` (`WoodcutterHut` gives
    /// "Woodcutter's Hut"); a `!name` entry is taken where there is no plain
    /// one, and the name itself where there is neither.
    pub fn name<'a>(&'a self, name: &'a str) -> &'a str {
        if let Some(text) = self.entries.get(name) {
            return text;
        }
        let bang = format!("!{name}");
        self.entries.get(&bang).map_or(name, String::as_str)
    }

    /// A key's entry, if it has one.
    pub fn entry(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FMT-3.8a (stand-ins list row 12): every byte decodes to a character
    /// whose Windows-1252 byte is the byte itself, so a text draws the
    /// glyph of each byte's number; the five bytes the code page leaves
    /// undefined keep their own number too. A character outside the code
    /// page has no byte.
    // spec: FMT-3.8a
    #[test]
    fn every_byte_comes_back() {
        for b in 0..=255u8 {
            assert_eq!(cp1252_byte(cp1252_char(b)), Some(b), "byte {b:#x}");
        }
        assert_eq!(cp1252_char(0x93), '\u{201c}');
        assert_eq!(cp1252_char(0x81), '\u{81}');
        assert_eq!(cp1252_byte('\u{20ac}'), Some(0x80));
        assert_eq!(cp1252_byte('\u{2013}'), Some(0x96));
        assert_eq!(cp1252_byte('\u{3042}'), None);
    }

    // spec: UI-GAME-1.3, FMT-3.8
    #[test]
    fn keys_and_unknown_labels() {
        let t = Texts::parse("Controls Keys\tStand\r\n!ATTACK\tATTACK\r\n!Africans\tNubians\r\n");
        assert_eq!(t.get("!Africans"), "Nubians");
        assert_eq!(t.get("!Unknown key"), "!Unknown key");
        assert_eq!(t.get("plain"), "plain");
        assert_eq!(decode(&[0x47, 0xe4]), "G\u{e4}");
        assert_eq!(
            decode(&[0x84, 0x47, 0x93, 0x96, 0x80]),
            "\u{201e}G\u{201c}\u{2013}\u{20ac}"
        );
        let names = Texts::parse("WoodcutterHut\tWoodcutter's Hut\n!Sawmill\tSaw Mill\n");
        assert_eq!(names.name("WoodcutterHut"), "Woodcutter's Hut");
        assert_eq!(names.name("Sawmill"), "Saw Mill");
        assert_eq!(names.name("Quarry"), "Quarry");
    }
}
