//! A film's subtitles (`data/video/<film>.xml`, `specs/UI-menus.md`
//! UI-MENU-0.3): `<Subtitle frame="f" text="key"/>` cues in frame order;
//! a cue's text key shows from its frame until the next cue, and an empty
//! key clears the line.

use crate::xml;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Subtitles {
    /// (frame, text key), in frame order.
    pub cues: Vec<(u32, String)>,
}

impl Subtitles {
    pub fn parse(text: &str) -> Self {
        let mut cues: Vec<(u32, String)> = xml::elements(text)
            .into_iter()
            .filter(|e| e.tag == "Subtitle")
            .filter_map(|e| {
                let frame = e.get("frame")?.trim().parse().ok()?;
                Some((frame, e.get("text").unwrap_or("").to_owned()))
            })
            .collect();
        cues.sort_by_key(|c| c.0);
        Self { cues }
    }

    /// The text key shown at `frame`, if any.
    pub fn at(&self, frame: u32) -> Option<&str> {
        let i = self.cues.partition_point(|c| c.0 <= frame);
        let key = self.cues[..i].last()?.1.as_str();
        (!key.is_empty()).then_some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // UI-MENU-0.3, T14: Intro_01 from frame 33 to 403, then nothing until
    // Intro_02 at 415; the last line stays to the end.
    #[test]
    fn intro_cues() {
        let s = Subtitles::parse(
            r#"<?xml version="1.0" ?>
            <Subtitles>
                <Subtitle frame="33" text="Intro_01"/>
                <Subtitle frame="403" text=""/>
                <Subtitle frame="415" text="Intro_02"/>
                <Subtitle frame="2821" text="Intro_13"/>
            </Subtitles>"#,
        );
        assert_eq!(s.at(0), None);
        assert_eq!(s.at(32), None);
        assert_eq!(s.at(33), Some("Intro_01"));
        assert_eq!(s.at(402), Some("Intro_01"));
        assert_eq!(s.at(403), None);
        assert_eq!(s.at(415), Some("Intro_02"));
        assert_eq!(s.at(3365), Some("Intro_13"));
    }
}
