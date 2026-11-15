//! The animation tables (`specs/GFX-animations.md` GFX-ANIM-1):
//! `Animation` entries with a name, a file, a tribes mask and Sound /
//! Particle events, and the random idle sets of
//! `character/settler_animations.xml`. The same `Animation` entries appear
//! in the donkey's and the animals' tables.

use crate::xml;

/// GFX-ANIM-1.1: a Sound or Particle child of an animation, `start` in
/// animation seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimEvent {
    pub sound: bool,
    pub name: String,
    pub start: f32,
    /// A Particle's `slot` (a bone of a building's model), if given.
    pub slot: Option<String>,
}

/// GFX-ANIM-1.1: an `Animation` entry. `tribes` is the mask (bit 0 Roman,
/// bit 1 African, bit 2 Chinese); 0 means all tribes.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimEntry {
    pub name: String,
    pub file: String,
    pub tribes: u32,
    pub events: Vec<AnimEvent>,
}

impl AnimEntry {
    /// The entry applies to `tribe` (0..2).
    pub fn for_tribe(&self, tribe: u8) -> bool {
        self.tribes == 0 || self.tribes & (1 << tribe.min(31)) != 0
    }
}

/// GFX-ANIM-1.4: a `RandomAnimations/Random` set: for settlers carrying
/// `item` (or all others), its delays in seconds and its clips with their
/// probabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomSet {
    pub item: Option<String>,
    pub min_delay: f32,
    pub max_delay: f32,
    pub clips: Vec<(String, f32)>,
}

/// The settler animation table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettlerAnimations {
    pub entries: Vec<AnimEntry>,
    pub random: Vec<RandomSet>,
}

fn number(e: &xml::Element, key: &str) -> Option<f32> {
    e.get(key)?.trim().parse().ok()
}

/// A `Sound` or `Particle` element as an event.
pub fn event(e: &xml::Element) -> Option<AnimEvent> {
    let sound = match e.tag.as_str() {
        "Sound" => true,
        "Particle" => false,
        _ => return None,
    };
    Some(AnimEvent {
        sound,
        name: e.get("name").unwrap_or_default().to_owned(),
        start: number(e, "start").unwrap_or(0.0),
        slot: e.get("slot").map(str::to_owned),
    })
}

/// The `Animation` entries (with `name` and `filename`) of a run of
/// elements, each with the events that follow it.
pub fn entries(elements: &[xml::Element]) -> Vec<AnimEntry> {
    let mut out: Vec<AnimEntry> = Vec::new();
    for e in elements {
        if e.tag == "Animation" {
            if let (Some(name), Some(file)) = (e.get("name"), e.get("filename")) {
                out.push(AnimEntry {
                    name: name.to_owned(),
                    file: file.to_owned(),
                    tribes: e
                        .get("tribes")
                        .and_then(|t| t.trim().parse().ok())
                        .unwrap_or(0),
                    events: Vec::new(),
                });
            }
        } else if let (Some(ev), Some(last)) = (event(e), out.last_mut()) {
            last.events.push(ev);
        }
    }
    out
}

impl SettlerAnimations {
    /// `settler_animations.xml`: the `Animations` entries and the
    /// `RandomAnimations` sets.
    pub fn read(text: &str) -> Self {
        let elements = xml::elements(text);
        let split = elements
            .iter()
            .position(|e| e.tag == "RandomAnimations")
            .unwrap_or(elements.len());
        let mut random: Vec<RandomSet> = Vec::new();
        for e in &elements[split..] {
            match e.tag.as_str() {
                "Random" => random.push(RandomSet {
                    item: e.get("item").map(str::to_owned),
                    min_delay: number(e, "minDelay").unwrap_or(0.0),
                    max_delay: number(e, "maxDelay").unwrap_or(0.0),
                    clips: Vec::new(),
                }),
                "Animation" => {
                    if let (Some(src), Some(set)) = (e.get("src"), random.last_mut()) {
                        set.clips
                            .push((src.to_owned(), number(e, "probability").unwrap_or(1.0)));
                    }
                }
                _ => {}
            }
        }
        Self {
            entries: entries(&elements[..split]),
            random,
        }
    }

    /// The first entry of a name (later repeats are ignored).
    pub fn get(&self, name: &str) -> Option<&AnimEntry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r#"<settler_animations><Animations>
        <Animation name="Stand" filename = "a\stand.gr2"/>
        <Animation name="WorkLoop2" filename = "a\fall.gr2">
            <Sound start="0.0" name="die"/>
            <Sound start="0.3" name="fall down"/>
        </Animation>
        <Animation name="WorkLoop0Smith" tribes="5" filename = "a\smith.GR2"/>
        <Animation name="Stand" filename = "a\second.gr2"/>
    </Animations>
    <RandomAnimations>
        <Random minDelay="5.0" maxDelay="8" >
            <Animation src="a\idle_01.gr2" probability="0.01"/>
            <Animation src="a\idle_02.gr2"/>
        </Random>
    </RandomAnimations></settler_animations>"#;

    // spec: GFX-ANIM-1.2, GFX-ANIM-1.4
    #[test]
    fn entries_events_and_random_sets() {
        let t = SettlerAnimations::read(TEXT);
        assert_eq!(t.entries.len(), 4);
        assert_eq!(t.get("Stand").unwrap().file, r"a\stand.gr2");
        let fall = t.get("WorkLoop2").unwrap();
        assert_eq!(fall.events.len(), 2);
        assert!(fall.events[1].sound && fall.events[1].start == 0.3);
        // GFX-ANIM T1: mask 5 is Roman and Chinese.
        let smith = t.get("WorkLoop0Smith").unwrap();
        assert!(smith.for_tribe(0) && !smith.for_tribe(1) && smith.for_tribe(2));
        assert!(t.get("Stand").unwrap().for_tribe(1));
        assert_eq!(t.random.len(), 1);
        let set = &t.random[0];
        assert_eq!(
            (set.min_delay, set.max_delay, set.item.clone()),
            (5.0, 8.0, None)
        );
        assert_eq!(set.clips[1], (r"a\idle_02.gr2".into(), 1.0));
    }
}
