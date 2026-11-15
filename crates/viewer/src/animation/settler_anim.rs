//! Which clip a settler plays (`specs/GFX-animations.md` GFX-ANIM-1, 3):
//! the base index from what the settler does, the profession and carried
//! good variants by tribe, and the random idle bag. Plain logic; the view
//! keeps one [`SettlerVisual`] per settler and plays what it says.

use s2_assets::anim_tables::{AnimEntry, RandomSet};

use crate::world::scene3d::SETTLER_BASES;

pub const STAND: usize = 0;
pub const WALK: usize = 1;
pub const SWIM_IDLE: usize = 2;
pub const SWIM_FWD: usize = 3;
pub const WORK_LOOP0: usize = 4;
pub const PICKUP: usize = 7;
pub const PUT_DOWN: usize = 8;
pub const HIT: usize = 9;

/// GFX-ANIM-3.7: world units a dying settler sinks per second.
pub const DYING_SINK: f32 = 0.35;

/// GFX-ANIM-3.1: an action the logic requests, by its number (2 PutDown,
/// 3 Pickup, 4..6 WorkLoop0..2, 7 Hit: the struck soldier of a duel round
/// that scored, GFX-ANIM-3.8, Q-087 answered).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    PutDown,
    Pickup,
    WorkLoop(u8),
    Hit,
}

impl Request {
    pub fn base(self) -> usize {
        match self {
            Self::PutDown => PUT_DOWN,
            Self::Pickup => PICKUP,
            Self::WorkLoop(i) => WORK_LOOP0 + usize::from(i.min(2)),
            Self::Hit => HIT,
        }
    }
}

/// What the logic says about a settler this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlerState {
    pub request: Option<Request>,
    /// The "inside / hidden" flag: a work loop request is ignored.
    pub inside: bool,
    /// Step progress >= 0.
    pub moving: bool,
    /// The model's base is below the terrain height.
    pub under: bool,
    /// The dying state (6): the clip is kept.
    pub dying: bool,
    /// GFX-ANIM-3.4: the base index the kind overrides (a rowing
    /// carrier's), played looping.
    pub override_base: Option<usize>,
    /// The tick the request was set. A new request is a new choice, so its
    /// play-once clip starts again even when the view never saw the Stand
    /// tick between two (GFX-ANIM-3.8f a: a builder's steps).
    pub started: Option<u64>,
}

/// GFX-ANIM-3.1: the base index of a tick, `None` to keep the current clip
/// (dying).
pub fn base_index(s: &SettlerState) -> Option<usize> {
    if s.dying {
        return None;
    }
    if let Some(base) = s.override_base {
        return Some(base);
    }
    let request = s
        .request
        .filter(|r| !(s.inside && matches!(r, Request::WorkLoop(_))));
    Some(match request {
        Some(r) => r.base(),
        None => match (s.moving, s.under) {
            (true, false) => WALK,
            (false, false) => STAND,
            (true, true) => SWIM_FWD,
            (false, true) => SWIM_IDLE,
        },
    })
}

/// GFX-ANIM-1.3: the clip of base `base` for a settler of `tribe` with the
/// profession variant name `profession` and carried good `good`: the
/// tribe's profession list if it has the clip, else the good's list, else
/// the base clip. A list member counts only when its own entry exists (and,
/// for a profession, its tribes mask includes the tribe); a list whose
/// member for `base` is missing falls back to the base clip (GFX-ANIM-1.2),
/// so looking the member up is enough. Repeated names take the first entry.
pub fn resolve<'a>(
    entries: &'a [AnimEntry],
    base: usize,
    good: Option<&str>,
    profession: Option<&str>,
    tribe: u8,
) -> Option<&'a AnimEntry> {
    let find = |name: &str| entries.iter().find(|e| e.name == name);
    let base_name = SETTLER_BASES[base];
    if let Some(j) = profession {
        if let Some(e) = find(&format!("{base_name}{j}")).filter(|e| e.for_tribe(tribe)) {
            return Some(e);
        }
    }
    if let Some(e) = good.and_then(|g| find(&format!("{base_name}{g}"))) {
        return Some(e);
    }
    find(base_name)
}

/// GFX-ANIM-1.4: the integer weight of each probability:
/// trunc(p x q + 0.5) with q = 1 / p_min as a 32-bit float (truncated
/// toward zero; Q-088 answered).
pub fn weights(probabilities: &[f32]) -> Vec<u32> {
    let min = probabilities
        .iter()
        .copied()
        .filter(|&p| p > 0.0)
        .fold(f32::INFINITY, f32::min);
    let q = 1.0 / min;
    probabilities
        .iter()
        .map(|&p| {
            if min.is_finite() && p > 0.0 {
                (p * q + 0.5).trunc() as u32
            } else {
                0
            }
        })
        .collect()
}

/// A small generator for the view's random choices (not game state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0.max(1);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform in 0..1.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// GFX-ANIM-1.4, 3.3: a settler's random idle set: the bag of clip
/// indices, the place in it and the wait before the next clip.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomIdle {
    pub set: usize,
    bag: Vec<usize>,
    next: usize,
    /// Real seconds until the next clip starts.
    pub wait: f32,
    /// The clip playing (index into the set's clips), if any.
    pub playing: Option<usize>,
}

impl RandomIdle {
    /// A new bag: clip i weight_i times, shuffled by sum(weights) random
    /// swaps; the first wait.
    pub fn new(set_index: usize, set: &RandomSet, rng: &mut Rng) -> Self {
        let probabilities: Vec<f32> = set.clips.iter().map(|(_, p)| *p).collect();
        let mut bag = Vec::new();
        for (i, w) in weights(&probabilities).into_iter().enumerate() {
            bag.extend(std::iter::repeat_n(i, w as usize));
        }
        let mut idle = Self {
            set: set_index,
            bag,
            next: 0,
            wait: 0.0,
            playing: None,
        };
        idle.shuffle(rng);
        idle.wait = delay(set, rng);
        idle
    }

    fn shuffle(&mut self, rng: &mut Rng) {
        let n = self.bag.len();
        for _ in 0..n {
            let (a, b) = (rng.below(n), rng.below(n));
            self.bag.swap(a, b);
        }
        self.next = 0;
    }

    #[cfg(test)]
    pub fn bag_len(&self) -> usize {
        self.bag.len()
    }

    /// The bag's count of clip `i`.
    #[cfg(test)]
    pub fn count(&self, i: usize) -> usize {
        self.bag.iter().filter(|&&c| c == i).count()
    }

    /// Advance by `dt` real seconds; the clip to start now, if the wait
    /// ran out. `last_duration` is the length in real seconds of the clip
    /// chosen last, added to the next wait.
    pub fn update(&mut self, dt: f32, set: &RandomSet, rng: &mut Rng) -> Option<usize> {
        if self.bag.is_empty() {
            return None;
        }
        self.wait -= dt;
        if self.wait > 0.0 {
            return None;
        }
        if self.next >= self.bag.len() {
            self.shuffle(rng);
        }
        let clip = self.bag[self.next];
        self.next += 1;
        self.playing = Some(clip);
        self.wait = delay(set, rng);
        Some(clip)
    }

    /// GFX-ANIM-3.3: the next wait also covers the clip just chosen.
    pub fn add_clip_time(&mut self, seconds: f32) {
        self.wait += seconds;
    }
}

/// minDelay + r x (maxDelay - minDelay).
fn delay(set: &RandomSet, rng: &mut Rng) -> f32 {
    set.min_delay + rng.unit() * (set.max_delay - set.min_delay)
}

/// GFX-ANIM-1.4, 3.3: the random set of a settler carrying `good`: the
/// good's own set if there is one, else the one without an item.
pub fn random_set(sets: &[RandomSet], good: Option<&str>) -> Option<usize> {
    good.and_then(|g| sets.iter().position(|s| s.item.as_deref() == Some(g)))
        .or_else(|| sets.iter().position(|s| s.item.is_none()))
}

/// GFX-ANIM-3.2: what decides a clip change: the base index, the loop
/// flag, good, profession and tribe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipChoice {
    pub base: usize,
    pub looping: bool,
    pub good: Option<String>,
    pub profession: Option<String>,
    pub tribe: u8,
    /// [`SettlerState::started`].
    pub started: Option<u64>,
}

/// The view's per-settler animation state.
#[derive(Debug, Clone, PartialEq)]
pub struct SettlerVisual {
    pub choice: Option<ClipChoice>,
    /// The resolved clip file, and whether it loops.
    pub clip: Option<(String, bool)>,
    /// The name of the resolved entry, whose Sound events play (SND-2.1):
    /// entries sharing a file carry their own events (the bulldozer's and
    /// the forester's `work_shovel`).
    pub entry: Option<String>,
    pub random: Option<RandomIdle>,
    /// GFX-ANIM-3.7: metres sunk so far while dying.
    pub sunk: f32,
}

impl SettlerVisual {
    pub fn new() -> Self {
        Self {
            choice: None,
            clip: None,
            entry: None,
            random: None,
            sunk: 0.0,
        }
    }

    /// GFX-ANIM-3.1, 3.2: the tick's choice; returns true when the clip
    /// changed. A changed choice ends a random idle (GFX-ANIM-3.3).
    pub fn tick(
        &mut self,
        state: &SettlerState,
        entries: &[AnimEntry],
        good: Option<&str>,
        profession: Option<&str>,
        tribe: u8,
    ) -> bool {
        let Some(base) = base_index(state) else {
            return false;
        };
        let choice = ClipChoice {
            base,
            looping: state.override_base.is_some() || crate::world::scene3d::base_loops(base),
            good: good.map(str::to_owned),
            profession: profession.map(str::to_owned),
            tribe,
            started: state.started,
        };
        if self.choice.as_ref() == Some(&choice) {
            return false;
        }
        let entry = resolve(entries, base, good, profession, tribe);
        self.clip = entry.map(|e| (e.file.clone(), choice.looping));
        self.entry = entry.map(|e| e.name.clone());
        self.choice = Some(choice);
        self.random = None;
        true
    }

    pub fn standing(&self) -> bool {
        self.choice.as_ref().is_some_and(|c| c.base == STAND)
    }
}

impl Default for SettlerVisual {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_assets::anim_tables::SettlerAnimations;

    /// GFX-ANIM-3.8: the builder's hammering request (code 4) resolves to a
    /// clip of the install's table. Skipped without `S2_GAME_DIR`.
    #[test]
    fn install_builder_hammers() {
        let Ok(dir) = std::env::var("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let t = crate::render::models3d::read_animations(std::path::Path::new(&dir));
        let work =
            resolve(&t.entries, WORK_LOOP0, None, Some("Constructor"), 0).expect("a work clip");
        let stand = resolve(&t.entries, STAND, None, Some("Constructor"), 0).expect("stand");
        eprintln!(
            "builder: work {} ({}), stand {} ({})",
            work.name, work.file, stand.name, stand.file
        );
        assert_ne!(work.file, stand.file);
    }

    /// SND-2.1: two entries on the same file (the bulldozer's and the
    /// forester's `work_shovel`) keep their own names, so each plays its
    /// own Sound events.
    #[test]
    fn shared_file_keeps_the_entry() {
        let t = SettlerAnimations::read(
            r#"<a><Animations>
            <Animation name="WorkLoop0Bulldozer" filename="shovel"/>
            <Animation name="WorkLoop0Forester" filename="shovel"/>
            </Animations></a>"#,
        );
        let mut v = SettlerVisual::new();
        let state = SettlerState {
            request: Some(Request::WorkLoop(0)),
            inside: false,
            moving: false,
            under: false,
            dying: false,
            override_base: None,
            started: None,
        };
        assert!(v.tick(&state, &t.entries, None, Some("Forester"), 0));
        assert_eq!(v.entry.as_deref(), Some("WorkLoop0Forester"));
        assert_eq!(v.clip.as_ref().map(|c| c.0.as_str()), Some("shovel"));
    }

    /// GFX-ANIM-3.8f a, T19a: each of a builder's work steps is a request
    /// of its own. When frames span several ticks and the one Stand tick
    /// between two steps is not seen, the next step is still a new choice,
    /// so the play-once hammer clip starts again instead of holding its
    /// last frame for 55 ticks (play test 10, F8).
    #[test]
    fn next_work_step_restarts_without_the_stand_tick() {
        let t = SettlerAnimations::read(
            r#"<a><Animations>
            <Animation name="WorkLoop0Constructor" filename="constructor_hammer"/>
            </Animations></a>"#,
        );
        let step = |started| SettlerState {
            request: Some(Request::WorkLoop(0)),
            inside: false,
            moving: false,
            under: false,
            dying: false,
            override_base: None,
            started: Some(started),
        };
        let mut v = SettlerVisual::new();
        assert!(v.tick(&step(500), &t.entries, None, Some("Constructor"), 0));
        // Later ticks of the same step keep the clip running.
        assert!(!v.tick(&step(500), &t.entries, None, Some("Constructor"), 0));
        // The step starting in tick 556 (555 Stand unseen) restarts it.
        assert!(v.tick(&step(556), &t.entries, None, Some("Constructor"), 0));
        assert_eq!(
            v.clip.as_ref().map(|c| c.0.as_str()),
            Some("constructor_hammer")
        );
    }

    fn table() -> SettlerAnimations {
        SettlerAnimations::read(
            r#"<a><Animations>
            <Animation name="Stand" filename="stand"/>
            <Animation name="Walk" filename="walk"/>
            <Animation name="WorkLoop0" filename="work0"/>
            <Animation name="Hit" filename="hit"/>
            <Animation name="StandWood" filename="stand_log"/>
            <Animation name="WalkWood" filename="walk_log"/>
            <Animation name="WalkBread" filename="walk_bread"/>
            <Animation name="WorkLoop0Smith" tribes="5" filename="smith"/>
            <Animation name="StandAxe" filename="stand_pickaxe"/>
            <Animation name="WorkLoop0Axe" filename="chop"/>
            </Animations>
            <RandomAnimations><Random minDelay="5.0" maxDelay="8">
            <Animation src="i01" probability="0.01"/><Animation src="i02" probability="0.2"/>
            <Animation src="i03" probability="0.1"/><Animation src="i04" probability="0.01"/>
            <Animation src="i05" probability="0.01"/><Animation src="i06" probability="0.1"/>
            <Animation src="i07" probability="0.1"/><Animation src="i08" probability="0.1"/>
            <Animation src="i09" probability="0.2"/><Animation src="i10" probability="0.2"/>
            <Animation src="i12" probability="0.2"/>
            </Random></RandomAnimations></a>"#,
        )
    }

    fn file(e: Option<&AnimEntry>) -> &str {
        e.map_or("-", |e| e.file.as_str())
    }

    /// GFX-ANIM T1: the Roman smith works with WorkLoop0Smith (mask 5); the
    /// African smith's mask excludes it and falls to WorkLoop0.
    // spec: GFX-ANIM-1.2, GFX-ANIM-1.3 (T1)
    #[test]
    fn t1_profession_by_tribe() {
        let t = table();
        let e = &t.entries;
        assert_eq!(
            file(resolve(e, WORK_LOOP0, None, Some("Smith"), 0)),
            "smith"
        );
        assert_eq!(
            file(resolve(e, WORK_LOOP0, None, Some("Smith"), 1)),
            "work0"
        );
        assert_eq!(
            file(resolve(e, WORK_LOOP0, None, Some("Smith"), 2)),
            "smith"
        );
    }

    /// GFX-ANIM T2: a carrier with Wood walks WalkWood, stands StandWood,
    /// hits with Hit (no HitWood). GFX-ANIM-1.2 (corrected): Bread has no
    /// Stand entry, but WalkBread makes a list, so it walks WalkBread and
    /// stands with the base clip.
    #[test]
    fn t2_carried_good() {
        let t = table();
        let e = &t.entries;
        assert_eq!(file(resolve(e, WALK, Some("Wood"), None, 0)), "walk_log");
        assert_eq!(file(resolve(e, STAND, Some("Wood"), None, 0)), "stand_log");
        assert_eq!(file(resolve(e, 9, Some("Wood"), None, 0)), "hit");
        assert_eq!(file(resolve(e, WALK, Some("Bread"), None, 0)), "walk_bread");
        assert_eq!(file(resolve(e, STAND, Some("Bread"), None, 0)), "stand");
        // GFX-ANIM-1.3: the woodcutter has no clips of his own; his tool,
        // the Axe, is his carried good while he carries no ware.
        assert_eq!(
            file(resolve(e, WORK_LOOP0, Some("Axe"), Some("Woodcutter"), 1)),
            "chop"
        );
        assert_eq!(
            file(resolve(e, WALK, Some("Wood"), Some("Woodcutter"), 1)),
            "walk_log"
        );
    }

    /// GFX-ANIM-3.4, T15: a rowing carrier's override plays WorkLoop1Boat
    /// moving and WorkLoop0Boat standing, looping, whatever the request.
    #[test]
    fn t15_boat_override_loops() {
        let t = SettlerAnimations::read(
            r#"<a><Animations>
            <Animation name="Walk" filename="walk"/>
            <Animation name="WorkLoop0Boat" filename="boat_stand"/>
            <Animation name="WorkLoop1Boat" filename="boat_walk"/>
            </Animations></a>"#,
        );
        let mut state = SettlerState {
            request: Some(Request::Pickup),
            inside: false,
            moving: true,
            under: false,
            dying: false,
            override_base: Some(WORK_LOOP0 + 1),
            started: None,
        };
        let mut v = SettlerVisual::new();
        assert!(v.tick(&state, &t.entries, Some("Boat"), Some("Carrier"), 0));
        assert_eq!(v.clip, Some(("boat_walk".to_owned(), true)));
        state.override_base = Some(WORK_LOOP0);
        assert!(v.tick(&state, &t.entries, Some("Boat"), Some("Carrier"), 0));
        assert_eq!(v.clip, Some(("boat_stand".to_owned(), true)));
    }

    /// GFX-ANIM-3.1: requests, the hidden flag, moving, swimming, dying.
    #[test]
    fn base_index_of_states() {
        let s = |request, inside, moving, under, dying| SettlerState {
            request,
            inside,
            moving,
            under,
            dying,
            override_base: None,
            started: None,
        };
        assert_eq!(base_index(&s(None, false, true, false, false)), Some(WALK));
        assert_eq!(
            base_index(&s(None, false, false, false, false)),
            Some(STAND)
        );
        assert_eq!(
            base_index(&s(None, false, true, true, false)),
            Some(SWIM_FWD)
        );
        assert_eq!(
            base_index(&s(None, false, false, true, false)),
            Some(SWIM_IDLE)
        );
        let work = Some(Request::WorkLoop(1));
        assert_eq!(
            base_index(&s(work, false, false, false, false)),
            Some(WORK_LOOP0 + 1)
        );
        assert_eq!(base_index(&s(work, true, false, false, false)), Some(STAND));
        let pick = Some(Request::Pickup);
        assert_eq!(
            base_index(&s(pick, true, false, false, false)),
            Some(PICKUP)
        );
        assert_eq!(
            base_index(&s(Some(Request::PutDown), false, true, false, false)),
            Some(PUT_DOWN)
        );
        assert_eq!(base_index(&s(None, false, true, false, true)), None);
    }

    /// GFX-ANIM-1.4 (Q-088 answered), T5's bag: the shipped set's weights
    /// 1, 20, 10 (0.01, 0.2, 0.1 with q = 100: 0.2 x 100 + 0.5 truncates
    /// to 20), a bag of 3 x 1 + 4 x 20 + 4 x 10 = 123 with clip 2 (index
    /// 1) 20 times, reshuffled after 123 picks.
    // spec: GFX-ANIM-1.4, GFX-ANIM-3.3 (T5)
    #[test]
    fn t5_random_bag() {
        let t = table();
        let set = &t.random[0];
        let w = weights(&set.clips.iter().map(|c| c.1).collect::<Vec<_>>());
        assert_eq!(w[..3], [1, 20, 10]);
        assert_eq!(w.iter().sum::<u32>(), 123);
        let mut rng = Rng(7);
        let mut idle = RandomIdle::new(0, set, &mut rng);
        assert_eq!(idle.bag_len(), 123);
        assert_eq!(idle.count(1), 20);
        assert!((5.0..=8.0).contains(&idle.wait));
        let mut picks = [0; 11];
        for _ in 0..123 {
            idle.wait = 0.0;
            picks[idle.update(0.0, set, &mut rng).unwrap()] += 1;
        }
        assert_eq!(picks[1], 20);
        assert_eq!(picks[0], 1);
        // The 124th pick comes from a fresh shuffle.
        idle.wait = 0.0;
        assert!(idle.update(0.0, set, &mut rng).is_some());
        assert_eq!(random_set(&t.random, Some("Wood")), Some(0));
    }

    /// GFX-ANIM-3.2: the clip changes only with the choice.
    #[test]
    fn clip_changes_with_the_choice() {
        let t = table();
        let mut v = SettlerVisual::new();
        let walking = SettlerState {
            request: None,
            inside: false,
            moving: true,
            under: false,
            dying: false,
            override_base: None,
            started: None,
        };
        assert!(v.tick(&walking, &t.entries, None, None, 0));
        assert!(!v.tick(&walking, &t.entries, None, None, 0));
        assert_eq!(v.clip, Some(("walk".into(), true)));
        assert!(v.tick(&walking, &t.entries, Some("Wood"), None, 0));
        assert_eq!(v.clip, Some(("walk_log".into(), true)));
        let dying = SettlerState {
            dying: true,
            override_base: None,
            started: None,
            ..walking
        };
        assert!(!v.tick(&dying, &t.entries, Some("Wood"), None, 0));
        assert_eq!(v.clip, Some(("walk_log".into(), true)));
    }
}
