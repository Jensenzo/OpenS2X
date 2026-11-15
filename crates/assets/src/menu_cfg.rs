//! The start-up script `data/scripts/menu.cfg` (rc00-wrapped Lua): the
//! language and the films' voice tracks per language
//! (`specs/UI-menus.md` UI-MENU-0.3a). Only the two calls the films need
//! are read, line by line; the rest of the script is ignored.

/// UI-MENU-0.3a: the voice track of a language not in the map.
pub const DEFAULT_VOICE_TRACK: u32 = 2;

/// What `menu.cfg` says about the films' sound.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuCfg {
    /// `menu.setLanguage("gb")`: the last one wins.
    pub language: Option<String>,
    /// `menu.setVideoVoiceStream("gb", 2)`, in file order.
    pub voice_streams: Vec<(String, u32)>,
}

impl MenuCfg {
    /// UI-MENU-0.3a: the voice track played with track 0 for the current
    /// language; 2 for a language not in the map.
    pub fn voice_track(&self) -> u32 {
        self.language
            .as_deref()
            .and_then(|lang| {
                self.voice_streams
                    .iter()
                    .rev()
                    .find(|(l, _)| l == lang)
                    .map(|(_, t)| *t)
            })
            .unwrap_or(DEFAULT_VOICE_TRACK)
    }
}

/// The calls' arguments: `name(` ... `)` with its comment cut off.
fn call<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let line = line.split("--").next()?.trim();
    let rest = line.strip_prefix(name)?.trim_start();
    let rest = rest.strip_prefix('(')?;
    Some(rest[..rest.rfind(')')?].trim())
}

fn quoted(arg: &str) -> Option<String> {
    let arg = arg.trim();
    let q = arg.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let inner = &arg[1..];
    Some(inner[..inner.find(q)?].to_owned())
}

/// Read the decoded text of `menu.cfg`.
pub fn parse(text: &str) -> MenuCfg {
    let mut cfg = MenuCfg::default();
    for line in text.lines() {
        if let Some(args) = call(line, "menu.setLanguage") {
            if let Some(lang) = quoted(args) {
                cfg.language = Some(lang);
            }
        } else if let Some(args) = call(line, "menu.setVideoVoiceStream") {
            let Some((lang, track)) = args.split_once(',') else {
                continue;
            };
            if let (Some(lang), Ok(track)) = (quoted(lang), track.trim().parse()) {
                cfg.voice_streams.push((lang, track));
            }
        }
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED: &str = "menu.setLanguage(\"gb\")\t\t-- ISO3166, a2\n\
        menu.setVideoVoiceStream(\"de\", 1)\n\
        menu.setVideoVoiceStream(\"gb\", 2)\n\
        menu.setVideoVoiceStream(\"fr\", 3)\n";

    /// UI-MENU T16a: "gb" plays track 2 with track 0, "de" track 1, a
    /// language not in the map ("it") track 2.
    // spec: FMT-3.10 (UI-MENU T16a)
    #[test]
    fn t16a_voice_track_by_language() {
        let cfg = parse(SHIPPED);
        assert_eq!(cfg.language.as_deref(), Some("gb"));
        assert_eq!(cfg.voice_streams.len(), 3);
        assert_eq!(cfg.voice_track(), 2);
        let de = parse(&SHIPPED.replace("setLanguage(\"gb\")", "setLanguage(\"de\")"));
        assert_eq!(de.voice_track(), 1);
        let fr = parse(&SHIPPED.replace("setLanguage(\"gb\")", "setLanguage('fr')"));
        assert_eq!(fr.voice_track(), 3);
        let it = parse(&SHIPPED.replace("setLanguage(\"gb\")", "setLanguage(\"it\")"));
        assert_eq!(it.voice_track(), 2);
        assert_eq!(parse("").voice_track(), DEFAULT_VOICE_TRACK);
    }

    // spec: FMT-3.10
    #[test]
    fn comments_and_other_lines_are_ignored() {
        let cfg = parse("-- menu.setLanguage(\"de\")\nmenu.other(1)\nmenu.setLanguage(\"fr\")\n");
        assert_eq!(cfg.language.as_deref(), Some("fr"));
        assert!(cfg.voice_streams.is_empty());
    }

    /// The install's own `menu.cfg`. Skipped without `S2_GAME_DIR`.
    // spec: FMT-3.10
    #[test]
    fn install_menu_cfg() {
        let Ok(dir) = std::env::var("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let path = std::path::Path::new(&dir).join("data/scripts/menu.cfg");
        let data = std::fs::read(&path).expect("menu.cfg");
        let plain = crate::rc00::decode(&path.to_string_lossy(), &data).expect("rc00");
        let cfg = parse(&String::from_utf8_lossy(&plain));
        assert_eq!(cfg.language.as_deref(), Some("gb"));
        assert_eq!(cfg.voice_track(), 2);
    }
}
