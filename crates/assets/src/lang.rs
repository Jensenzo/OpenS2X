//! The language files `data/language/<lang>/controls.txt` and
//! `dialogs.txt` (`specs/UI-ingame.md` UI-GAME-1.3): one entry per line,
//! `!key<TAB>text`; a label starting with `!` is looked up, a key without an
//! entry is shown unchanged.

use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Texts {
    entries: HashMap<String, String>,
}

/// The bytes as Windows-1252 text (the files are not UTF-8): the bytes
/// 0x80 to 0x9f that code page maps to other characters are taken as
/// Latin-1 (a stand-in; the game's own font lookup is by byte).
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
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

    // spec: UI-GAME-1.3, FMT-3.8
    #[test]
    fn keys_and_unknown_labels() {
        let t = Texts::parse("Controls Keys\tStand\r\n!ATTACK\tATTACK\r\n!Africans\tNubians\r\n");
        assert_eq!(t.get("!Africans"), "Nubians");
        assert_eq!(t.get("!Unknown key"), "!Unknown key");
        assert_eq!(t.get("plain"), "plain");
        assert_eq!(decode(&[0x47, 0xe4]), "G\u{e4}");
        let names = Texts::parse("WoodcutterHut\tWoodcutter's Hut\n!Sawmill\tSaw Mill\n");
        assert_eq!(names.name("WoodcutterHut"), "Woodcutter's Hut");
        assert_eq!(names.name("Sawmill"), "Saw Mill");
        assert_eq!(names.name("Quarry"), "Quarry");
    }
}
