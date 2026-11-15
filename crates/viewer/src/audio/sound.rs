//! Sound effects, ambient sounds and music (`specs/SND-sound-music.md`) as
//! plain logic: which sound plays, where, how loud, and the music's
//! playlist with the battle loop. The audio output is `audio.rs`.

use std::collections::HashMap;

use s2_rules::model::{SoundType, Tribe};

/// SND-2.3: a positioned sound is heard within this many times its 3D
/// minimum distance; the zoom does not gate it.
pub const AUDIBILITY_FACTOR: f32 = 30.0;
/// SND-8.2: the 3D distances when a definition sets none.
pub const DEFAULT_MIN_DISTANCE: f32 = 1.0;
pub const DEFAULT_MAX_DISTANCE: f32 = 1e9;
/// SND-8.2: the library's roll-off factor, set once at the start.
pub const ROLL_OFF: f32 = 4.0;
/// SND-2.6: the channels of the sound library, one pool.
pub const CHANNELS: usize = 32;
/// SND-2.6a: at most this many ambient loops hold a channel.
pub const MAX_AMBIENT_LOOPS: usize = 4;
/// SND-7.4: an ambient's 2D distance when its definition sets none (all
/// thirteen shipped ambients set 50).
pub const AMBIENT_DISTANCE: f32 = 50.0;
/// SND-2.6a: what the music stream asks a channel with: above every
/// definition's priority (interface sounds reach 200), so it always gets one.
pub const MUSIC_PRIORITY: i64 = 256;
/// SND-8.1: the listener sits 0.8 x the eye's height above the ground
/// along the view.
pub const LISTENER_OFFSET: f32 = 0.8;
/// SND-2.2, GFX-ANIM-3.6: a footstep every 0.5 s divided by the global
/// animation speed (0.278 s) of logic time.
pub const STEP_PERIOD: f32 = 0.5 / crate::animation::anim::GLOBAL_SPEED;
/// GFX-ANIM-3.6: the visibility counter: +5 per frame near the camera (to
/// 100), -3 per frame away; "near" is a squared distance of at most this
/// (7.07 world units; F56: this was taken for the distance itself, so every
/// settler within 50 units stepped).
pub const STEP_NEAR_SQUARED: f32 = 50.0;
/// SND-3.4: the battle loop holds 20 s after the last fight.
pub const BATTLE_HOLD: f64 = 20.0;
/// SND-7.2: no pause between two tracks.
pub const TRACK_GAP: f64 = 0.0;

/// SND-1.1: a sound definition.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundDef {
    pub name: String,
    pub files: Vec<String>,
    pub kind: SoundType,
    pub looping: bool,
    /// 0..1 (the definition's 0..255).
    pub volume: f32,
    /// SND-2.6c: higher is more important.
    pub priority: i64,
    pub min_distance: f32,
    /// SND-7.4: an ambient's 2D distance D (`sn_set2DDistance`; 50 for all
    /// thirteen ambients).
    pub distance_2d: f32,
    pub tribe: Option<Tribe>,
    pub percussion: bool,
}

/// The sounds of the rules, by display name without case (SND-2.1).
#[derive(Debug, Clone, Default)]
pub struct SoundBank {
    by_name: HashMap<String, SoundDef>,
    by_id: HashMap<u32, String>,
}

impl SoundBank {
    pub fn new(rules: &s2_rules::RuleSet) -> Self {
        let mut bank = Self::default();
        for s in rules.sounds.iter() {
            let Some(name) = s.name.clone() else { continue };
            let def = SoundDef {
                name: name.clone(),
                files: s.file_names.clone(),
                kind: s.sound_type.unwrap_or(SoundType::Effect),
                looping: s.looping.unwrap_or(false),
                // SND-1.1: 255 unless given.
                volume: s.volume.unwrap_or(255).clamp(0, 255) as f32 / 255.0,
                priority: s.priority.unwrap_or(0),
                min_distance: s.min_distance_3d.map_or(DEFAULT_MIN_DISTANCE, |d| d as f32),
                distance_2d: s.distance_2d.map_or(AMBIENT_DISTANCE, |d| d as f32),
                tribe: s.music_tribe,
                percussion: s.percussion.unwrap_or(false),
            };
            bank.by_id.insert(s.id, name.to_ascii_lowercase());
            bank.by_name.insert(name.to_ascii_lowercase(), def);
        }
        bank
    }

    pub fn get(&self, name: &str) -> Option<&SoundDef> {
        self.by_name.get(&name.to_ascii_lowercase())
    }

    pub fn by_id(&self, id: u32) -> Option<&SoundDef> {
        self.by_name.get(self.by_id.get(&id)?)
    }

    /// SND-3.2: the music definitions of a tribe: the music and the
    /// percussion sets.
    pub fn tribe_music(&self, tribe: Tribe) -> Vec<&SoundDef> {
        let mut defs: Vec<&SoundDef> = self
            .by_name
            .values()
            .filter(|d| d.kind == SoundType::Music && d.tribe == Some(tribe))
            .collect();
        defs.sort_by_key(|d| d.percussion);
        defs
    }
}

/// SND-2.2 code events by the spec's sound names: the definitions they
/// are, by display name.
pub const STEPS: &str = "steps";
pub const DOOR_OPEN: &str = "opening Door";
pub const DOOR_CLOSE: &str = "closing Door";
pub const TREE_FALL: &str = "falling Tree";
pub const CATAPULT_SHOOT: &str = "catapult shoot";
pub const CATAPULT_IMPACT: &str = "catapult impact";
pub const COLLAPSE: &str = "building collapsing";
/// SND-3.1, 3.4: the menu and battle music.
pub const MUSIC_MAIN: &str = "Intro Music";
pub const MUSIC_BATTLE: &str = "Battle Music";
pub const MUSIC_CREDITS: &str = "Credits Music";
/// SND-2.2 (Q-186 a answered), CAM-3.6: the interface's briefing music
/// (the book dialog) and the end dialog's won and lost jingles.
pub const MUSIC_BRIEFING: &str = "Briefing Music";
pub const MUSIC_WON: &str = "Winning Jingle";
pub const MUSIC_LOST: &str = "Loosing Jingle";

/// SND-2.2: the interface's music slot for what the mission interface shows:
/// the end dialog (and the victory and defeat screens) its won or lost
/// jingle, once; the book dialog of briefings and debriefings the briefing
/// music, looping like the menus' (SND-3.1 names "once" only for the
/// jingles). The end dialog wins over a book.
pub fn interface_music(end: Option<bool>, book: bool) -> Option<(&'static str, bool)> {
    match (end, book) {
        (Some(true), _) => Some((MUSIC_WON, false)),
        (Some(false), _) => Some((MUSIC_LOST, false)),
        (None, true) => Some((MUSIC_BRIEFING, true)),
        (None, false) => None,
    }
}

/// SND-2.3: is a sound at `distance_squared` from the listener heard?
/// Closer than 30 x its 3D minimum distance (squared values compared).
pub fn audible(distance_squared: f32, min_distance: f32) -> bool {
    let reach = AUDIBILITY_FACTOR * min_distance;
    distance_squared < reach * reach
}

/// SND-8.3: the distance factor of a 3D sound: 1 within the minimum
/// distance, else min / (min + 4 x (dist - min)), with the distance capped at
/// the maximum.
pub fn distance_gain(min_distance: f32, distance: f32) -> f32 {
    let min = min_distance.max(f32::MIN_POSITIVE);
    let d = distance.min(DEFAULT_MAX_DISTANCE.max(min));
    if d <= min {
        1.0
    } else {
        min / (min + ROLL_OFF * (d - min))
    }
}

/// SND-8.3: the library holds the factor as a whole level, g x 255 cut to an
/// integer.
pub fn distance_level(min_distance: f32, distance: f32) -> u8 {
    (distance_gain(min_distance, distance) * 255.0) as u8
}

/// SND-8.1: the listener for the camera's eye `eye`, its smoothed unit
/// view direction `view` and the eye's height above the terrain under it:
/// eye + D x (0.8 h / |D|^2) with D = (v.y, v.y, v.z). The original puts
/// the direction's y into the x slot too; reproduced as specified.
pub fn listener(eye: [f32; 3], view: [f32; 3], height: f32) -> [f32; 3] {
    let d = [view[1], view[1], view[2]];
    let length2: f32 = d.iter().map(|c| c * c).sum();
    if length2 == 0.0 {
        return eye;
    }
    let k = LISTENER_OFFSET * height / length2;
    [0, 1, 2].map(|i| eye[i] + d[i] * k)
}

/// SND-8.1, SND-8.3a 1: the listener's right R = u x f for its smoothed unit
/// view direction `view` (game frame): the forward f = D / |D| with D =
/// (v.y, v.y, v.z) (8.1's slip), the up u = Y - (f . Y) f made unit, so R
/// is the screen's right in the game's left-handed frame. None when D is 0
/// or f is straight up or down (the game takes another fixed axis there,
/// which no game camera reaches; the sound then plays in the middle).
pub fn listener_right(view: [f32; 3]) -> Option<[f32; 3]> {
    let d = [view[1], view[1], view[2]];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if length == 0.0 {
        return None;
    }
    let f = d.map(|c| c / length);
    let u = [-f[1] * f[0], 1.0 - f[1] * f[1], -f[1] * f[2]];
    let u_length = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    if u_length < 1e-6 {
        return None;
    }
    let u = u.map(|c| c / u_length);
    Some([
        u[1] * f[2] - u[2] * f[1],
        u[2] * f[0] - u[0] * f[2],
        u[0] * f[1] - u[1] * f[0],
    ])
}

/// SND-8.3a 2: a 3D sound's pan, 0 (full left) to 255 (full right), 128 the
/// centre: 128 + trunc(128 n . R), n the unit vector from the listener
/// point `ear` to the sound (n = 0 for a sound at the listener).
pub fn pan(ear: [f32; 3], right: [f32; 3], at: [f32; 3]) -> u8 {
    let d = [at[0] - ear[0], at[1] - ear[1], at[2] - ear[2]];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let along = if length < 1e-6 {
        0.0
    } else {
        (d[0] * right[0] + d[1] * right[1] + d[2] * right[2]) / length
    };
    (128 + (128.0 * along) as i32).clamp(0, 255) as u8
}

/// SND-8.3a 3: the library's pan table T(p) = min(255, floor(16 sqrt p)).
pub fn pan_table(p: u8) -> u32 {
    ((16.0 * f64::from(p).sqrt()).floor() as u32).min(255)
}

/// SND-8.3a 3: the left and right levels of a sound at `pan` with the level
/// v (0 .. 255, 8.3): trunc(T(255 - pan) v / 255) and trunc(T(pan) v / 255),
/// an equal-power law (a centred sound at 180 and 181 of 255).
pub fn pan_levels(pan: u8, v: u8) -> [u8; 2] {
    [255 - pan, pan].map(|p| (pan_table(p) * u32::from(v) / 255) as u8)
}

/// SND-8.3a 3: the left and right gains of a sound at `pan`, the table's
/// T(255 - pan) / 255 and T(pan) / 255, applied to the channel's volume.
pub fn pan_gains(pan: u8) -> [f32; 2] {
    [255 - pan, pan].map(|p| pan_table(p) as f32 / 255.0)
}

/// SND-2.6b, SND-8.3a 4: a panned channel's level, the mean of its left and
/// right levels cut to an integer.
pub fn panned_level(pan: u8, v: u8) -> u8 {
    let [l, r] = pan_levels(pan, v);
    ((u16::from(l) + u16::from(r)) / 2) as u8
}

/// SND-2.4, SND-8.3: a playing sound's two gains, which the game sets every
/// audio update and the output reads as it plays.
#[derive(Debug)]
pub struct PanGains([std::sync::atomic::AtomicU32; 2]);

impl PanGains {
    pub fn new(gains: [f32; 2]) -> Self {
        Self(gains.map(|g| std::sync::atomic::AtomicU32::new(g.to_bits())))
    }

    pub fn set(&self, gains: [f32; 2]) {
        for (cell, g) in self.0.iter().zip(gains) {
            cell.store(g.to_bits(), std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// What the output reads (Windows only: elsewhere nothing plays).
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    pub fn get(&self) -> [f32; 2] {
        [0, 1].map(|k| f32::from_bits(self.0[k].load(std::sync::atomic::Ordering::Relaxed)))
    }
}

/// The volume settings (SND-5.1): master, music, UI, sound; ambient 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub ui: f32,
    pub sound: f32,
}

impl Volumes {
    pub fn from_settings(v: [f32; 4]) -> Self {
        Self {
            master: v[0],
            music: v[1],
            ui: v[2],
            sound: v[3],
        }
    }
}

/// What decides a playing sound's volume (SND-7.1, SND-8.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Voice {
    kind: SoundType,
    volume: f32,
    min_distance: f32,
}

impl Voice {
    /// SND-7.1: int(definition volume (0..255) x its type's setting).
    fn units(&self, volumes: Volumes) -> f32 {
        let kind = match self.kind {
            SoundType::Music => volumes.music,
            SoundType::Gui => volumes.ui,
            SoundType::Ambient => 1.0,
            SoundType::Effect => volumes.sound,
        };
        (self.volume * 255.0 * kind).floor()
    }

    /// SND-8.3: the whole level of the distance, 255 for a sound without a
    /// place.
    fn fade(&self, distance: Option<f32>) -> f32 {
        distance.map_or(255.0, |d| f32::from(distance_level(self.min_distance, d)))
    }

    /// SND-7.1, SND-8.3: the channel's volume: the product of SND-7.1, with
    /// the master applied on top by the library. A positioned sound
    /// `distance` from the listener is scaled by its whole level / 255.
    pub fn volume(&self, volumes: Volumes, distance: Option<f32>) -> f32 {
        self.units(volumes) / 255.0 * volumes.master * self.fade(distance) / 255.0
    }

    /// SND-8.3: the channel's level v after distance, 0 .. 255, under the
    /// master.
    pub fn level(&self, volumes: Volumes, distance: Option<f32>) -> u8 {
        (self.units(volumes) * self.fade(distance) / 255.0) as u8
    }

    /// SND-2.6b, SND-8.3a 4: the channel's present level, 0 .. 255, which
    /// decides among equal priorities: a panned sound's the mean of its left
    /// and right levels; an unpanned sound's (2D) v.
    pub fn present_level(&self, volumes: Volumes, distance: Option<f32>, pan: Option<u8>) -> u8 {
        let v = self.level(volumes, distance);
        pan.map_or(v, |pan| panned_level(pan, v))
    }
}

impl SoundDef {
    pub fn voice(&self) -> Voice {
        Voice {
            kind: self.kind,
            volume: self.volume,
            min_distance: self.min_distance,
        }
    }
}

/// [`Voice::volume`] of a definition.
#[cfg(test)]
pub fn channel_volume(def: &SoundDef, volumes: Volumes, distance: Option<f32>) -> f32 {
    def.voice().volume(volumes, distance)
}

/// A sound holding a channel (SND-2.6a).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held<H> {
    /// What the output plays it as.
    pub handle: H,
    pub priority: i64,
    pub level: u8,
    /// The music and the ambient loops: nothing can take these channels.
    pub set_aside: bool,
}

/// SND-2.6b: where a new sound goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Start<H> {
    /// This channel is free.
    Free(usize),
    /// This channel's sound is cut at once and the new one takes it.
    Cut(usize, H),
    /// The new sound is lost, with no retry and no queue.
    Refused,
}

/// SND-2.6: the library's 32 channels.
#[derive(Debug, Clone)]
pub struct Channels<H> {
    slots: [Option<Held<H>>; CHANNELS],
    /// The last channel handed out: the ring begins after it.
    last: usize,
}

impl<H> Default for Channels<H> {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            last: CHANNELS - 1,
        }
    }
}

impl<H: Clone> Channels<H> {
    /// SND-2.6b: the channels in the ring's order, beginning after the last
    /// one handed out.
    fn ring(&self) -> impl Iterator<Item = usize> {
        let first = self.last + 1;
        (0..CHANNELS).map(move |i| (first + i) % CHANNELS)
    }

    /// SND-2.6b: the channel a sound of this priority would take: the first
    /// free one in the ring; if none, the lowest priority playing, the
    /// quietest among equals, the first in the ring among equal levels, if
    /// that priority is at most the new sound's. The channels set aside are
    /// skipped.
    pub fn choose(&self, priority: i64) -> Start<H> {
        let mut candidate: Option<(usize, &Held<H>)> = None;
        for i in self.ring() {
            let Some(held) = &self.slots[i] else {
                return Start::Free(i);
            };
            if held.set_aside {
                continue;
            }
            let better =
                candidate.is_none_or(|(_, c)| (held.priority, held.level) < (c.priority, c.level));
            if better {
                candidate = Some((i, held));
            }
        }
        match candidate {
            Some((i, c)) if c.priority <= priority => Start::Cut(i, c.handle.clone()),
            _ => Start::Refused,
        }
    }

    /// The new sound takes the channel [`Self::choose`] gave; the ring moves
    /// on (only when a channel is handed out, SND-2.6b).
    pub fn hold(&mut self, channel: usize, held: Held<H>) {
        self.slots[channel] = Some(held);
        self.last = channel;
    }

    /// A channel given back: a loop stopped by the game, or SND-2.6: a
    /// finished sound, at the next audio update.
    pub fn retain(&mut self, mut keep: impl FnMut(&Held<H>) -> bool) {
        for slot in &mut self.slots {
            if slot.as_ref().is_some_and(|held| !keep(held)) {
                *slot = None;
            }
        }
    }

    /// The sounds holding a channel.
    pub fn held_mut(&mut self) -> impl Iterator<Item = &mut Held<H>> {
        self.slots.iter_mut().flatten()
    }

    /// The effects and interface sounds playing: all but those set aside.
    #[cfg(test)]
    pub fn sounds_playing(&self) -> usize {
        self.slots.iter().flatten().filter(|h| !h.set_aside).count()
    }
}

/// SND-7.4: an ambient emitter of the map: its sound, where it is (game
/// frame, its field's ground height) and its definition's volume V (0 to
/// 255) and 2D distance D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emitter {
    pub sound: u32,
    pub at: [f32; 3],
    pub volume: f32,
    pub distance: f32,
}

/// SND-7.4: an ambient loop playing: its sound, the emitter its level
/// follows, and its current and wanted level (0 to 255; negative is
/// silent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientLoop {
    pub sound: u32,
    pub emitter: usize,
    pub current: f32,
    pub wanted: f32,
}

/// SND-7.4 1: the horizontal distance from the listener.
fn horizontal(l: [f32; 3], e: [f32; 3]) -> f32 {
    ((l[0] - e[0]).powi(2) + (l[2] - e[2]).powi(2)).sqrt()
}

/// SND-7.4 1: the four nearest emitters with different sounds, by their
/// horizontal distance from the listener: per sound only its nearest counts,
/// ties keep the earlier, and there is no distance cut. Their indexes,
/// nearest first.
pub fn nearest_ambients(listener: [f32; 3], emitters: &[Emitter]) -> Vec<usize> {
    let mut best: Vec<(f32, usize)> = Vec::new();
    for (i, e) in emitters.iter().enumerate() {
        let r = horizontal(listener, e.at);
        match best.iter_mut().find(|(_, j)| emitters[*j].sound == e.sound) {
            Some(b) if r < b.0 => *b = (r, i),
            Some(_) => {}
            None => best.push((r, i)),
        }
    }
    best.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    best.truncate(MAX_AMBIENT_LOOPS);
    best.into_iter().map(|(_, i)| i).collect()
}

/// SND-7.4 5: the wanted level of an emitter heard at `listener`:
/// int(V x min(1, 2 - r / D) x max(0, 1 - 0.0125 |Ly - Ey|)).
/// Worked in double precision: in single precision 2 - 60 / 50 is just
/// under 0.8 and int(100 x it) 79, where 7.T1 has 80.
pub fn ambient_level(listener: [f32; 3], e: &Emitter) -> f32 {
    let r = f64::from(horizontal(listener, e.at));
    let near = (2.0 - r / f64::from(e.distance)).min(1.0);
    let height = (1.0 - 0.0125 * f64::from((listener[1] - e.at[1]).abs())).max(0.0);
    (f64::from(e.volume) * near * height).trunc() as f32
}

/// What an ambient update asks of the output.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AmbientChanges {
    /// Sounds to start, looping, at level 0.
    pub start: Vec<u32>,
    /// Sounds whose channel is freed.
    pub stop: Vec<u32>,
}

/// SND-7.4: the ambient player: up to four loops, each the nearest emitter
/// of its sound.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ambients {
    pub playing: Vec<AmbientLoop>,
}

impl Ambients {
    /// One audio update, `t` the last frame's real time in seconds. A loop
    /// started here is in `playing` at level 0; when the output refuses it,
    /// [`Self::refused`] takes it out to be tried again next update.
    pub fn update(&mut self, listener: [f32; 3], emitters: &[Emitter], t: f32) -> AmbientChanges {
        let mut changes = AmbientChanges::default();
        let chosen = nearest_ambients(listener, emitters);
        let chosen_of = |sound: u32| chosen.iter().copied().find(|&i| emitters[i].sound == sound);
        // 2. Keep or hand over; 3. fade out the rest.
        for p in &mut self.playing {
            match chosen_of(p.sound) {
                Some(i) => p.emitter = i,
                None => p.wanted = 0.0,
            }
        }
        // 4. Start the chosen sounds not playing while fewer than four play.
        for &i in &chosen {
            let sound = emitters[i].sound;
            if self.playing.iter().any(|p| p.sound == sound) {
                continue;
            }
            if self.playing.len() >= MAX_AMBIENT_LOOPS {
                break;
            }
            self.playing.push(AmbientLoop {
                sound,
                emitter: i,
                current: 0.0,
                wanted: 0.0,
            });
            changes.start.push(sound);
        }
        // 5. Levels: a kept loop follows its emitter; the current level
        // moves toward the wanted one by at most 255 t.
        let step = 255.0 * t;
        for p in &mut self.playing {
            if chosen_of(p.sound).is_some() {
                p.wanted = ambient_level(listener, &emitters[p.emitter]);
            }
            let gap = p.wanted - p.current;
            p.current = if gap.abs() <= step {
                p.wanted
            } else {
                p.current + step.copysign(gap)
            };
        }
        // 3. A dropped loop stops once both levels are below 1.
        self.playing.retain(|p| {
            let done = chosen_of(p.sound).is_none() && p.wanted < 1.0 && p.current < 1.0;
            if done {
                changes.stop.push(p.sound);
            }
            !done
        });
        changes
    }

    /// SND-7.4 4: the output refused to start `sound`: it is tried again in
    /// the next update.
    pub fn refused(&mut self, sound: u32) {
        self.playing.retain(|p| p.sound != sound);
    }

    /// SND-7.4 5: a loop's channel volume, 0 to 1: int(int(current) x the
    /// ambient type volume (1)) of 255, a negative value as silence.
    pub fn volume(p: &AmbientLoop) -> f32 {
        p.current.trunc().max(0.0) / 255.0
    }
}

/// SND-1.2: one of a definition's files, uniformly.
pub fn pick_file(def: &SoundDef, random: u64) -> Option<&str> {
    if def.files.is_empty() {
        return None;
    }
    Some(&def.files[(random % def.files.len() as u64) as usize])
}

/// SND-3.2: the in-game playlist: for each present player slot its
/// tribe's music and percussion sets, without repeats.
pub fn playlist(bank: &SoundBank, tribes: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for &t in tribes {
        let tribe = match t {
            1 => Tribe::Africans,
            2 => Tribe::Chinese,
            _ => Tribe::Romans,
        };
        for def in bank.tribe_music(tribe) {
            for f in &def.files {
                if !out.contains(f) {
                    out.push(f.clone());
                }
            }
        }
    }
    out
}

/// What the music stream should do now.
#[derive(Debug, Clone, PartialEq)]
pub enum MusicCommand {
    Play { file: String, looping: bool },
    Stop,
}

/// Where a music command comes from: a menu's music, the credits', the
/// game's playlist and battle loop (SND-3), the interface's briefing music
/// and jingles (SND-2.2) by their definition's name
/// ([`interface_music`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicSource {
    Menu,
    Credits,
    Game,
    Interface(&'static str),
}

/// SND-3.2: the tribe whose music set holds `file` (0 Roman, 1 African,
/// 2 Chinese).
pub fn tribe_of(bank: &SoundBank, file: &str) -> Option<u8> {
    [
        (0, Tribe::Romans),
        (1, Tribe::Africans),
        (2, Tribe::Chinese),
    ]
    .into_iter()
    .find(|(_, tribe)| {
        bank.tribe_music(*tribe)
            .iter()
            .any(|d| d.files.iter().any(|f| f == file))
    })
    .map(|(n, _)| n)
}

/// What a music command switches to, for a project (`docs/extending.md`,
/// the music hook): a menu's or the credits' music; in a game the battle
/// loop starting (the only looping track there, SND-3.4) or stopping, or a
/// playlist track of the tribe `tribe` gives (SND-3.2); W13: the book's
/// briefing music and the end dialog's jingles (SND-2.2), by the
/// definition's name.
pub fn music_slot(
    source: MusicSource,
    command: &MusicCommand,
    tribe: impl Fn(&str) -> Option<u8>,
) -> Option<crate::extend::MusicSlot> {
    use crate::extend::MusicSlot;
    match (source, command) {
        (MusicSource::Menu, MusicCommand::Play { .. }) => Some(MusicSlot::Menu),
        (MusicSource::Credits, MusicCommand::Play { .. }) => Some(MusicSlot::Credits),
        (MusicSource::Game, MusicCommand::Play { looping: true, .. }) => {
            Some(MusicSlot::Battle(true))
        }
        (MusicSource::Game, MusicCommand::Play { file, .. }) => tribe(file).map(MusicSlot::Tribe),
        (MusicSource::Game, MusicCommand::Stop) => Some(MusicSlot::Battle(false)),
        (MusicSource::Interface(name), MusicCommand::Play { .. }) => match name {
            MUSIC_BRIEFING => Some(MusicSlot::Briefing),
            MUSIC_WON => Some(MusicSlot::Jingle(true)),
            MUSIC_LOST => Some(MusicSlot::Jingle(false)),
            _ => None,
        },
        _ => None,
    }
}

/// SND-7.1: the music stream's volume, the music volume; silent when a
/// project plays the music itself (`Extensions::music_external`).
pub fn music_volume(external: bool, music: f32) -> f32 {
    if external {
        0.0
    } else {
        music
    }
}

/// SND-3: the music: a fixed track (menus) or the playlist, interrupted by
/// the battle loop while the player fights.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Music {
    /// The file playing and whether it is the battle loop.
    pub current: Option<(String, bool)>,
    next: usize,
    battle_until: f64,
    /// When the next playlist track may start (after the gap).
    resume_at: Option<f64>,
}

impl Music {
    /// SND-3.1: a menu's music, looping; replaces whatever plays.
    pub fn menu(&mut self, file: &str) -> Option<MusicCommand> {
        self.slot(file, true)
    }

    /// SND-2.2: a slot's music: nothing while that file is the one set
    /// (a jingle played to its end stays set, so it plays once), else it
    /// replaces whatever plays.
    pub fn slot(&mut self, file: &str, looping: bool) -> Option<MusicCommand> {
        if self.current.as_ref().is_some_and(|(f, _)| f == file) {
            return None;
        }
        self.current = Some((file.to_owned(), false));
        Some(MusicCommand::Play {
            file: file.to_owned(),
            looping,
        })
    }

    /// SND-3.3, 3.4: the in-game music at `now`: `fighting` holds the
    /// battle loop 20 s; otherwise the playlist goes on when the track has
    /// `ended`, after the gap.
    pub fn game(
        &mut self,
        now: f64,
        fighting: bool,
        ended: bool,
        playlist: &[String],
        battle: Option<&str>,
    ) -> Option<MusicCommand> {
        if fighting {
            self.battle_until = now + BATTLE_HOLD;
            if let Some(b) = battle {
                if !self
                    .current
                    .as_ref()
                    .is_some_and(|(_, is_battle)| *is_battle)
                {
                    self.current = Some((b.to_owned(), true));
                    return Some(MusicCommand::Play {
                        file: b.to_owned(),
                        looping: true,
                    });
                }
            }
            return None;
        }
        if self.current.as_ref().is_some_and(|(_, b)| *b) {
            if now < self.battle_until {
                return None;
            }
            self.current = None;
            self.resume_at = Some(now);
            return Some(MusicCommand::Stop);
        }
        let playing_playlist = self
            .current
            .as_ref()
            .is_some_and(|(f, _)| playlist.contains(f));
        if playing_playlist && !ended {
            return None;
        }
        if playlist.is_empty() {
            return None;
        }
        match self.resume_at {
            None => {
                self.resume_at = Some(now + if playing_playlist { TRACK_GAP } else { 0.0 });
                if playing_playlist {
                    return None;
                }
            }
            Some(at) if now < at => return None,
            _ => {}
        }
        self.resume_at = None;
        let file = playlist[self.next % playlist.len()].clone();
        self.next += 1;
        self.current = Some((file.clone(), false));
        Some(MusicCommand::Play {
            file,
            looping: false,
        })
    }
}

/// GFX-ANIM-3.6: a settler's footsteps: the step accumulator (in step
/// periods) and the visibility counter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepClock {
    accumulator: f32,
    visible: i32,
}

impl Default for StepClock {
    /// Standing: the accumulator held at 1, so the first moving tick steps.
    fn default() -> Self {
        Self {
            accumulator: 1.0,
            visible: 0,
        }
    }
}

/// GFX-ANIM-3.6: whether a settler at `at` counts as near the camera at `eye`
/// (squared distance exactly 50 is near).
pub fn step_near(eye: [f32; 3], at: [f32; 3]) -> bool {
    (0..3).map(|i| (at[i] - eye[i]).powi(2)).sum::<f32>() <= STEP_NEAR_SQUARED
}

impl StepClock {
    /// One frame of the visibility counter: +5 near the camera up to 100,
    /// -3 otherwise, not below 0.
    pub fn frame(&mut self, near: bool) {
        self.visible = if near {
            (self.visible + 5).min(100)
        } else {
            (self.visible - 3).max(0)
        };
    }

    /// One logic tick (0.1 s): standing holds the accumulator at 1; moving,
    /// a step sounds when it has reached 1, and it grows by a tick's share
    /// of the period. True when a step sounds (and the counter is
    /// positive).
    pub fn tick(&mut self, moving: bool) -> bool {
        if !moving {
            self.accumulator = 1.0;
            return false;
        }
        let due = self.accumulator >= 1.0;
        if due {
            self.accumulator -= 1.0;
        }
        self.accumulator += 0.1 / STEP_PERIOD;
        due && self.visible > 0
    }
}

/// GFX-ANIM-2.3: the events whose `start` the clip's clock passed going
/// from `last` to `now` (animation seconds): every loop of a looping clip,
/// once for a clip played once.
pub fn events_crossed(
    events: &[(String, f32)],
    last: f32,
    now: f32,
    duration: f32,
    looping: bool,
) -> Vec<&str> {
    if now <= last || duration <= 0.0 {
        return Vec::new();
    }
    let hit = |s: f32| -> bool {
        if looping {
            if now - last >= duration {
                return true;
            }
            let (l, n) = (last.rem_euclid(duration), now.rem_euclid(duration));
            if n >= l {
                l < s && s <= n
            } else {
                s > l || s <= n
            }
        } else {
            last < s && s <= now.min(duration)
        }
    };
    events
        .iter()
        .filter(|(_, s)| hit(*s))
        .map(|(n, _)| n.as_str())
        .collect()
}

/// A sound the game asks for: a definition by name at a place (game
/// frame), or a GUI file. SND-2.4: a sound fired for a moving object names
/// it in `source` and follows it while it plays; one fired for a place
/// (`None`) keeps `at`.
#[derive(Debug, Clone, PartialEq)]
pub enum SoundRequest {
    At {
        name: String,
        at: [f32; 3],
        source: Option<crate::world::scene3d::MoverId>,
    },
    Gui(String),
}

/// SND-2.4: where a playing sound is now: at its source's drawn place while
/// the source is on the map, else where it was last (a place's sound keeps
/// its own).
pub fn position_now(last: [f32; 3], source_now: Option<[f32; 3]>) -> [f32; 3] {
    source_now.unwrap_or(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The music hook (docs/extending.md): a menu's and the credits'
    /// tracks, the battle loop's start and stop and each playlist track by
    /// its tribe, in the order the player switches (SND-3.1, 3.2, 3.4).
    // spec: SND-3.1, SND-3.2, SND-3.4
    #[test]
    fn music_slots_follow_the_player() {
        use crate::extend::MusicSlot;
        let play = |file: &str, looping| MusicCommand::Play {
            file: file.into(),
            looping,
        };
        let tribe = |f: &str| match f {
            "Rome1.mp3" => Some(0),
            "Nubia1.mp3" => Some(1),
            _ => None,
        };
        let slot = |source, c: &MusicCommand| music_slot(source, c, tribe);
        let menu = Music::default().menu("main.mp3").expect("plays");
        assert_eq!(slot(MusicSource::Menu, &menu), Some(MusicSlot::Menu));
        assert_eq!(
            slot(MusicSource::Credits, &play("credits.mp3", true)),
            Some(MusicSlot::Credits)
        );
        let list = ["Rome1.mp3".to_owned(), "Nubia1.mp3".to_owned()];
        let mut m = Music::default();
        let mut slots = Vec::new();
        let mut at = |m: &mut Music, now: f64, fighting: bool, ended: bool| {
            if let Some(c) = m.game(now, fighting, ended, &list, Some("battle.mp3")) {
                slots.push(slot(MusicSource::Game, &c));
            }
        };
        at(&mut m, 0.0, false, false);
        // SND-7.2: the next track at the first update after the end.
        at(&mut m, 1.0, false, true);
        at(&mut m, 1.1, false, true);
        at(&mut m, 2.0, true, false);
        at(&mut m, 10.0, false, false);
        at(&mut m, 22.5, false, false);
        at(&mut m, 22.6, false, false);
        assert_eq!(
            slots,
            [
                Some(MusicSlot::Tribe(0)),
                Some(MusicSlot::Tribe(1)),
                Some(MusicSlot::Battle(true)),
                Some(MusicSlot::Battle(false)),
                Some(MusicSlot::Tribe(0)),
            ]
        );
    }

    /// SND-2.2, CAM-3.6 (Q-186 a answered): the end dialog plays the won or
    /// lost jingle once, the book dialog the briefing music; the game's
    /// playlist comes back at once when the book closes.
    // spec: SND-2.2, CAM-3.6, SND-3.1 (and hook W13)
    #[test]
    fn the_end_dialog_and_the_book_have_their_music() {
        assert_eq!(interface_music(Some(true), false), Some((MUSIC_WON, false)));
        assert_eq!(
            interface_music(Some(false), true),
            Some((MUSIC_LOST, false))
        );
        assert_eq!(interface_music(None, true), Some((MUSIC_BRIEFING, true)));
        assert_eq!(interface_music(None, false), None);
        // W13: a project hears of them as slots, by the definition's name.
        use crate::extend::MusicSlot;
        let slot = |end, book| {
            let (name, looping) = interface_music(end, book).expect("music");
            let command = Music::default().slot("any.mp3", looping).expect("plays");
            music_slot(MusicSource::Interface(name), &command, |_| None)
        };
        assert_eq!(slot(None, true), Some(MusicSlot::Briefing));
        assert_eq!(slot(Some(true), true), Some(MusicSlot::Jingle(true)));
        assert_eq!(slot(Some(false), false), Some(MusicSlot::Jingle(false)));
        assert_eq!(
            music_slot(MusicSource::Interface("other"), &MusicCommand::Stop, |_| {
                None
            }),
            None
        );
        let mut m = Music::default();
        let list = ["Rome1.mp3".to_owned()];
        assert!(m.game(0.0, false, false, &list, None).is_some());
        assert_eq!(
            m.slot("brief.mp3", true),
            Some(MusicCommand::Play {
                file: "brief.mp3".into(),
                looping: true
            })
        );
        assert_eq!(m.slot("brief.mp3", true), None, "already playing");
        assert_eq!(
            m.game(5.0, false, false, &list, None),
            Some(MusicCommand::Play {
                file: "Rome1.mp3".into(),
                looping: false
            }),
            "the playlist when the book closes"
        );
        // A jingle plays once: still set after its end, it does not restart.
        assert!(m.slot("won.mp3", false).is_some());
        assert_eq!(m.slot("won.mp3", false), None);
        // W13: the project hears of it by the definition's name, whatever
        // the file.
        assert_eq!(
            music_slot(
                MusicSource::Interface(MUSIC_WON),
                &MusicCommand::Play {
                    file: "won.mp3".into(),
                    looping: false
                },
                |_| None
            ),
            Some(crate::extend::MusicSlot::Jingle(true)),
        );
    }

    /// SND-7.1: the music plays at the music volume; with the music hook's
    /// switch on, the viewer's player is silent.
    // spec: SND-7.1
    #[test]
    fn external_music_silences_the_player() {
        assert_eq!(music_volume(false, 0.6), 0.6);
        assert_eq!(music_volume(true, 0.6), 0.0);
    }

    fn def(min: f32) -> SoundDef {
        SoundDef {
            name: "chopping Wood".into(),
            files: vec!["a.wav".into(), "b.wav".into()],
            kind: SoundType::Effect,
            looping: false,
            volume: 1.0,
            priority: 11,
            min_distance: min,
            distance_2d: AMBIENT_DISTANCE,
            tribe: None,
            percussion: false,
        }
    }

    /// SND T1, T2, T3: 30 x 20 = 600 reaches 500 (g = 20 / 1940), not 700;
    /// the zoom gates nothing (the test takes no zoom).
    // spec: SND-2.3
    #[test]
    fn audibility() {
        assert!(audible(500.0 * 500.0, 20.0));
        assert!((distance_gain(20.0, 500.0) - 0.0103).abs() < 1e-4);
        assert!(!audible(700.0 * 700.0, 20.0));
        assert!(!audible(600.0 * 600.0, 20.0));
    }

    /// SND 8.T1: the listener at zoom 0..4 on flat ground at height 0,
    /// camera at yaw 0 on target (0, 5, 0); the eyes from the table, the
    /// view directions those of GFX-VIEW-3.3 (pitch components -0.3, -0.6,
    /// -0.8, -0.8, -0.8).
    #[test]
    fn listener_8t1() {
        let eyes = [
            [0.0, 10.0, 0.0],
            [0.0, 20.21, -14.48],
            [0.0, 42.02, -19.83],
            [0.0, 60.53, -29.75],
            [0.0, 79.04, -39.67],
        ];
        let expected = [
            [-2.20, 7.80, 7.00],
            [-7.13, 13.08, -4.97],
            [-16.40, 25.62, -7.53],
            [-23.62, 36.91, -12.03],
            [-30.85, 48.20, -16.53],
        ];
        for (zoom, (eye, want)) in eyes.iter().zip(expected).enumerate() {
            let p: f32 = [-0.3, -0.6, -0.8, -0.8, -0.8][zoom];
            let view = [0.0, p, (1.0 - p * p).sqrt()];
            let got = listener(*eye, view, eye[1]);
            for k in 0..3 {
                assert!((got[k] - want[k]).abs() < 0.02, "zoom {zoom}: {got:?}");
            }
        }
        assert_eq!(listener([1.0, 2.0, 3.0], [0.0; 3], 2.0), [1.0, 2.0, 3.0]);
    }

    /// SND T9 (SND-8.3a): a sound at full level (v = 255) at the listener's
    /// right plays on the right only; straight ahead (n . R = 0) at 180 and
    /// 181, level 180; at n . R = 0.5 pan 192, 126 and 221, level 173.
    // spec: SND-8.3a, SND-2.6b (T9)
    #[test]
    fn t9_pan_law() {
        assert_eq!(pan_table(0), 0);
        assert_eq!(pan_table(127), 180);
        assert_eq!(pan_table(128), 181);
        assert_eq!(pan_table(255), 255);
        let ear = [0.0, 10.0, 0.0];
        let right = [1.0, 0.0, 0.0];
        // (a) at the right.
        let a = pan(ear, right, [5.0, 10.0, 0.0]);
        assert_eq!(a, 255);
        assert_eq!(pan_levels(a, 255), [0, 255]);
        // (b) straight ahead, below, or at the listener.
        for at in [[0.0, 10.0, 7.0], [0.0, 0.0, 0.0], ear] {
            let b = pan(ear, right, at);
            assert_eq!(b, 128);
            assert_eq!(pan_levels(b, 255), [180, 181]);
            assert_eq!(panned_level(b, 255), 180);
        }
        // (c) n . R = 0.5.
        let c = pan(ear, right, [1.0, 10.0, 3f32.sqrt()]);
        assert_eq!(c, 192);
        assert_eq!(pan_levels(c, 255), [126, 221]);
        assert_eq!(panned_level(c, 255), 173);
        // The left: 0; the gains are the table over 255.
        assert_eq!(pan(ear, right, [-5.0, 10.0, 0.0]), 0);
        assert_eq!(pan_gains(128), [180.0 / 255.0, 181.0 / 255.0]);
        assert_eq!(pan_gains(255), [0.0, 1.0]);
        let gains = PanGains::new([0.25, 1.0]);
        assert_eq!(gains.get(), [0.25, 1.0]);
        gains.set([1.0, 0.5]);
        assert_eq!(gains.get(), [1.0, 0.5]);
    }

    /// SND-8.1, SND-8.3a 1: the listener's right is u x f with f from D =
    /// (v.y, v.y, v.z) and u = Y - (f . Y) f made unit: a unit vector at
    /// right angles to f and to u, horizontal; for a view without the slip
    /// (v.y = 0, looking along +z) it is +x. The default camera's slip
    /// turns it: a sound under 8.T1's zoom-0 target is off the centre.
    // spec: SND-8.1, SND-8.3a
    #[test]
    fn the_listeners_right_from_its_up_and_forward() {
        let r = listener_right([0.0, 0.0, 1.0]).unwrap();
        assert_eq!(r, [1.0, 0.0, 0.0]);
        let view = [0.0, -0.3, (1.0f32 - 0.09).sqrt()];
        let r = listener_right(view).unwrap();
        let d = [view[1], view[1], view[2]];
        let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let f = d.map(|c| c / length);
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        assert!((dot(r, r) - 1.0).abs() < 1e-5);
        assert!(dot(r, f).abs() < 1e-5);
        assert!(r[1].abs() < 1e-5, "horizontal: {r:?}");
        assert!(r[0] > 0.9, "toward +x: {r:?}");
        assert_eq!(listener_right([0.0, 0.0, 0.0]), None);
        assert_eq!(listener_right([0.0, 1.0, 0.0]).map(|_| ()), Some(()));
        // 8.T1 zoom 0: a sound below the eye is in the middle (R is
        // horizontal and at right angles to the slipped D); one 30 units
        // ahead along the view, in the middle of the screen, is heard to
        // the right (n . R = 0.377: pan 176).
        let eye = [0.0, 10.0, 0.0];
        let ear = listener(eye, view, 10.0);
        assert_eq!(pan(ear, r, [0.0, 0.0, 0.0]), 128);
        let ahead = [0, 1, 2].map(|k| eye[k] + 30.0 * view[k]);
        assert_eq!(pan(ear, r, ahead), 176);
    }

    // SND-2.4: a sound fired for a walking settler is where he is now each
    // audio update, so its level and pan follow him; with him gone from the
    // map it stays where he was last; a sound of a place keeps its place.
    // From 8.T1's zoom 0 listener and eye: a step (min 3) fired under the
    // target, the settler walking 20 units to the right.
    // spec: SND-2.4, SND-8.3, SND-8.3a
    #[test]
    fn a_sound_follows_its_moving_source() {
        let ear = [-2.20, 7.80, 7.00];
        let eye = [0.0, 10.0, 0.0];
        let step = def(3.0).voice();
        let volumes = Volumes::from_settings([1.0, 0.3, 1.0, 1.0]);
        let level_at = |at: [f32; 3]| {
            let d = (0..3).map(|k| (at[k] - ear[k]).powi(2)).sum::<f32>().sqrt();
            step.level(volumes, Some(d))
        };
        let fired = [0.0, 0.0, 0.0];
        let mut at = fired;
        let mut seen = Vec::new();
        for walked in [0.0, 5.0, 10.0, 20.0] {
            at = position_now(at, Some([walked, 0.0, 0.0]));
            seen.push((level_at(at), pan(eye, [1.0, 0.0, 0.0], at)));
        }
        assert_eq!(seen[0].1, 128);
        for w in seen.windows(2) {
            assert!(w[1].0 <= w[0].0, "quieter as he walks off: {seen:?}");
            assert!(w[1].1 > w[0].1, "further right: {seen:?}");
        }
        assert!(seen[3].0 < seen[0].0);
        // Gone into a building: where he was last.
        assert_eq!(position_now(at, None), [20.0, 0.0, 0.0]);
        // A place's sound has no source and keeps its place.
        assert_eq!(position_now(fired, None), fired);
    }

    // spec: SND-8.3
    #[test]
    fn falloff_8t2_8t3() {
        let listeners = [
            [-2.20, 7.80, 7.00],
            [-7.13, 13.08, -4.97],
            [-16.40, 25.62, -7.53],
            [-23.62, 36.91, -12.03],
            [-30.85, 48.20, -16.53],
        ];
        let chop = [1.000, 1.000, 0.306, 0.164, 0.112];
        let steps = [0.089, 0.056, 0.026, 0.017, 0.013];
        for (zoom, l) in listeners.iter().enumerate() {
            let d2: f32 = l.iter().map(|c| c * c).sum();
            let d = d2.sqrt();
            assert!(audible(d2, 20.0) && audible(d2, 3.0), "zoom {zoom}");
            assert!(
                (distance_gain(20.0, d) - chop[zoom]).abs() < 2e-3,
                "zoom {zoom}"
            );
            assert!(
                (distance_gain(3.0, d) - steps[zoom]).abs() < 2e-3,
                "zoom {zoom}"
            );
        }
        // The channel volume is the channel's times the whole level / 255.
        let v = Volumes::from_settings([1.0, 0.3, 1.0, 1.0]);
        let d = 31.34;
        let level = distance_level(20.0, d);
        assert_eq!(level, 78);
        assert!((channel_volume(&def(20.0), v, Some(d)) - 78.0 / 255.0).abs() < 1e-6);
    }

    /// SND-8.3: a sound at twice its minimum distance plays at 1/5, the
    /// level is cut to a whole number, and at the cut-off of SND-2.3 g is
    /// always 1/117, level 2, whatever the minimum distance.
    // spec: SND-8.2, SND-8.3
    #[test]
    fn roll_off_four_and_whole_levels() {
        assert_eq!(distance_gain(20.0, 20.0), 1.0);
        assert_eq!(distance_gain(20.0, 5.0), 1.0);
        assert!((distance_gain(20.0, 40.0) - 0.2).abs() < 1e-6);
        // 0.2 x 255 = 51 exactly, whatever the rounding of the float.
        assert!(matches!(distance_level(20.0, 40.0), 50 | 51));
        assert_eq!(distance_level(20.0, 10.0), 255);
        for (min, reach) in [(20.0, 600.0), (3.0, 90.0)] {
            assert!((distance_gain(min, reach) - 1.0 / 117.0).abs() < 1e-5);
            assert_eq!(distance_level(min, reach), 2);
        }
        // Below one 255th the channel is silent, not rounded up.
        assert_eq!(distance_level(1.0, 1000.0), 0);
        let v = Volumes::from_settings([1.0, 0.3, 1.0, 1.0]);
        assert_eq!(channel_volume(&def(1.0), v, Some(1000.0)), 0.0);
        assert_eq!(
            def(20.0).voice().level(v, Some(40.0)),
            distance_level(20.0, 40.0)
        );
        // The type's volume is in the level, the master is the library's.
        let half = Volumes::from_settings([0.5, 0.3, 1.0, 0.5]);
        assert_eq!(def(20.0).voice().level(half, None), 127);
        assert!((channel_volume(&def(20.0), half, None) - 0.5 * 127.0 / 255.0).abs() < 1e-6);
    }

    /// SND 2.T1's start: the music and 4 ambient loops set aside, the other
    /// 27 channels holding effects of these priorities and levels.
    fn busy(effects: &[(i64, u8)]) -> Channels<u32> {
        let mut c = Channels::default();
        let mut start = |handle: u32, priority: i64, level: u8, set_aside: bool| {
            let Start::Free(slot) = c.choose(priority) else {
                panic!("not free");
            };
            c.hold(
                slot,
                Held {
                    handle,
                    priority,
                    level,
                    set_aside,
                },
            );
        };
        for handle in 0..5 {
            start(1000 + handle, 100, 64, true);
        }
        for (i, &(priority, level)) in effects.iter().enumerate() {
            start(i as u32, priority, level, false);
        }
        c
    }

    /// SND 2.T1: 27 effects playing, music and 4 ambients on, every effect
    /// priority 11: a step (0) is refused and nothing is cut.
    // spec: SND-2.6a, SND-2.6b, SND-2.6c (2.T1)
    #[test]
    fn a_step_is_refused_when_nothing_lower_plays() {
        let c = busy(&[(11, 200); 27]);
        assert_eq!(c.sounds_playing(), 27);
        assert_eq!(c.choose(0), Start::Refused);
    }

    /// SND 2.T2: one effect of priority 8 at any level, the rest 11: a hammer
    /// (11) cuts the priority 8 one, however close (loud) it is.
    // spec: SND-2.6b, SND-2.6c (2.T2)
    #[test]
    fn priority_goes_before_distance() {
        let mut effects = [(11, 3); 27];
        effects[9] = (8, 255);
        assert_eq!(busy(&effects).choose(11), Start::Cut(14, 9));
    }

    /// SND 2.T3: all priority 11, levels 40, 12, 12 and the rest higher: the
    /// first level 12 channel in the ring is cut; the ring begins after the
    /// last channel handed out and moves only when one was handed out.
    // spec: SND-2.6b (2.T3)
    #[test]
    fn the_quietest_goes_first_in_the_ring() {
        let mut effects = [(11, 100); 27];
        effects[0] = (11, 40);
        effects[1] = (11, 12);
        effects[2] = (11, 12);
        let mut c = busy(&effects);
        assert_eq!(c.choose(11), Start::Cut(6, 1));
        // A refusal leaves the ring where it was.
        assert_eq!(c.choose(5), Start::Refused);
        assert_eq!(c.choose(11), Start::Cut(6, 1));
        // Handed out: that channel is now the end of the ring, so of the two
        // quietest the one after it comes first.
        c.hold(
            6,
            Held {
                handle: 77,
                priority: 11,
                level: 12,
                set_aside: false,
            },
        );
        assert_eq!(c.choose(11), Start::Cut(7, 2));
    }

    /// SND-2.6a, 2.6c: an interface sound takes any effect's channel, but no
    /// sound takes the music's or a loop's; a sound of the highest priority
    /// is refused when only those are left.
    // spec: SND-2.6a, SND-2.6c
    #[test]
    fn set_aside_channels_are_never_cut() {
        let c = busy(&[(11, 200); 27]);
        assert_eq!(c.choose(200), Start::Cut(5, 0));
        let mut all: Channels<u32> = Channels::default();
        for handle in 0..CHANNELS as u32 {
            let Start::Free(slot) = all.choose(0) else {
                panic!("not free");
            };
            all.hold(
                slot,
                Held {
                    handle,
                    priority: 0,
                    level: 0,
                    set_aside: true,
                },
            );
        }
        assert_eq!(all.choose(i64::MAX), Start::Refused);
    }

    /// SND-2.6a: of six ambient sounds in reach four play, the loudest.
    /// SND 7.T1: listener (0, 20, 0); forest1 at r = 10, 30, 70, water1 at
    /// 60, meadow1 at 120, beach at 99, all at height 20, V 100, D 50: the
    /// four nearest different sounds start at level 0; wanted 100, 80, 2 and
    /// negative; after one update of t = 0.02 the levels are 5.1, 5.1, 2 and
    /// -5.1; forest1 reaches 100 after 20 updates.
    // spec: SND-7.4, SND-2.6a (7.T1)
    #[test]
    fn t7_1_the_four_nearest_ambients() {
        let (forest, water, meadow, beach) = (1, 2, 3, 4);
        let e = |sound, x: f32| Emitter {
            sound,
            at: [x, 20.0, 0.0],
            volume: 100.0,
            distance: 50.0,
        };
        let emitters = [
            e(forest, 30.0),
            e(forest, 10.0),
            e(forest, 70.0),
            e(water, 60.0),
            e(meadow, 120.0),
            e(beach, 99.0),
        ];
        let l = [0.0, 20.0, 0.0];
        assert_eq!(nearest_ambients(l, &emitters), [1, 3, 5, 4]);
        let mut player = Ambients::default();
        let changes = player.update(l, &emitters, 0.02);
        assert_eq!(changes.start, [forest, water, beach, meadow]);
        let level = |p: &Ambients, s| p.playing.iter().find(|x| x.sound == s).unwrap().current;
        let wanted = |p: &Ambients, s| p.playing.iter().find(|x| x.sound == s).unwrap().wanted;
        assert_eq!(
            [forest, water, beach].map(|s| wanted(&player, s)),
            [100.0, 80.0, 2.0]
        );
        assert!(wanted(&player, meadow) < 0.0);
        for (s, v) in [(forest, 5.1), (water, 5.1), (beach, 2.0), (meadow, -5.1)] {
            assert!(
                (level(&player, s) - v).abs() < 1e-4,
                "{s}: {}",
                level(&player, s)
            );
        }
        assert_eq!(
            Ambients::volume(&player.playing[3]),
            0.0,
            "negative is silent"
        );
        for _ in 1..19 {
            player.update(l, &emitters, 0.02);
        }
        assert!(level(&player, forest) < 100.0);
        player.update(l, &emitters, 0.02);
        assert_eq!(level(&player, forest), 100.0);
        // Height: silent 80 units above or below.
        let high = Emitter {
            at: [0.0, 100.0, 0.0],
            ..e(forest, 0.0)
        };
        assert_eq!(ambient_level(l, &high), 0.0);
    }

    /// SND 7.T2: the listener moves so that a fifth sound (lava) is among the
    /// four nearest: the farthest (meadow1) leaves, fades to 0 and stops below
    /// 1; lava starts once a channel is free, never more than four at once. A
    /// nearer emitter of a sound that plays takes its channel over.
    // spec: SND-7.4, SND-2.6a (7.T2)
    #[test]
    fn t7_2_a_fifth_sound_waits_for_a_channel() {
        let (forest, water, meadow, beach, lava) = (1, 2, 3, 4, 5);
        let e = |sound, x: f32| Emitter {
            sound,
            at: [x, 20.0, 0.0],
            volume: 100.0,
            distance: 50.0,
        };
        let mut emitters = vec![
            e(forest, 10.0),
            e(water, 60.0),
            e(meadow, 40.0),
            e(beach, 99.0),
            e(lava, 300.0),
            e(forest, 200.0),
        ];
        let mut player = Ambients::default();
        let at = [0.0, 20.0, 0.0];
        for _ in 0..30 {
            player.update(at, &emitters, 0.02);
        }
        assert_eq!(player.playing.len(), 4);
        // Lava comes near, the meadow is now the farthest of five.
        emitters[4] = e(lava, 20.0);
        emitters[2] = e(meadow, 150.0);
        let first = player.update(at, &emitters, 0.02);
        assert!(first.start.is_empty(), "four still play");
        assert!(player.playing.len() <= 4);
        let mut started = false;
        for _ in 0..40 {
            let c = player.update(at, &emitters, 0.02);
            assert!(player.playing.len() <= 4);
            if c.stop.contains(&meadow) {
                assert!(!player.playing.iter().any(|p| p.sound == meadow));
            }
            started |= c.start.contains(&lava);
        }
        assert!(started);
        assert!(!player.playing.iter().any(|p| p.sound == meadow));
        // The far forest emitter now nearer than the first: the loop is
        // handed over, not restarted.
        emitters[5] = e(forest, 1.0);
        let c = player.update(at, &emitters, 0.02);
        assert!(c.start.is_empty() && c.stop.is_empty());
        let loop_ = player.playing.iter().find(|p| p.sound == forest).unwrap();
        assert_eq!(loop_.emitter, 5);
    }

    /// SND-2.6: finished channels are free again, and the first free one in
    /// the ring comes before any cut.
    // spec: SND-2.6, SND-2.6b
    #[test]
    fn finished_channels_are_free_again() {
        let mut c = busy(&[(11, 200); 27]);
        c.retain(|h| h.handle != 20);
        assert_eq!(c.sounds_playing(), 26);
        assert_eq!(c.choose(0), Start::Free(25));
    }

    /// SND 2.T4: one woodcutter, nothing else playing: the same sound fired
    /// three times faster than it ends: all three overlap, each on a channel
    /// of its own; no limit per sound or per file.
    // spec: SND-2.6d (2.T4)
    #[test]
    fn one_sound_overlaps_itself() {
        let mut c: Channels<u32> = Channels::default();
        for handle in 0..3 {
            let Start::Free(slot) = c.choose(11) else {
                panic!("not free");
            };
            assert_eq!(slot, handle as usize);
            c.hold(
                slot,
                Held {
                    handle,
                    priority: 11,
                    level: 255,
                    set_aside: false,
                },
            );
        }
        assert_eq!(c.sounds_playing(), 3);
    }

    /// SND 2.T5: sound events run on logic time, so a real second at speed 3
    /// (30 ticks) has three times the events of one at speed 1 (10 ticks).
    /// That a sample keeps its length: the output never sets a speed.
    // spec: SND-2.6e (2.T5)
    #[test]
    fn events_come_with_the_ticks() {
        let steps = |ticks: u32| {
            let mut clock = StepClock::default();
            clock.frame(true);
            (0..ticks).filter(|_| clock.tick(true)).count()
        };
        // The first step comes on the first tick, so up to 3 apart.
        assert!(steps(216 * 3).abs_diff(3 * steps(216)) <= 3);
        let events = vec![("chop".to_owned(), 1.0)];
        let per_second = |speed: f32| {
            let mut n = 0;
            for frame in 0..600 {
                let (last, now) = (
                    frame as f32 * speed / 60.0,
                    (frame + 1) as f32 * speed / 60.0,
                );
                n += events_crossed(&events, last, now, 2.0, true).len();
            }
            n
        };
        assert_eq!(per_second(3.0), 3 * per_second(1.0));
    }

    /// SND-2.2, GFX-ANIM-3.6: steps every 0.278 s of logic time, the first
    /// on the first moving tick; none while the visibility counter is 0;
    /// each a random file of the set (T4).
    #[test]
    fn footsteps() {
        let mut clock = StepClock::default();
        clock.frame(true);
        let ticks: Vec<bool> = (0..20).map(|_| clock.tick(true)).collect();
        assert!(ticks[0]);
        // 2 s of walking: 1 + floor(1.9 / 0.2778) steps.
        assert_eq!(ticks.iter().filter(|&&s| s).count(), 7);
        // Standing resets: the next step comes at once.
        assert!(!clock.tick(false));
        assert!(clock.tick(true));
        let mut far = StepClock::default();
        for _ in 0..3 {
            far.frame(false);
        }
        assert!(!far.tick(true));
        assert!((STEP_PERIOD - 0.27778).abs() < 1e-4);
        assert_eq!(pick_file(&def(3.0), 3), Some("b.wav"));
    }

    /// SND 8.T4: flat ground, a settler walking under the camera at zoom 0
    /// to 4: none steps, since the eye stays 10 or more above the ground
    /// (GFX-VIEW-3.3) and the gate asks for 7.07 at most. 8.T5: following him
    /// the gate measures from the follow camera's eye (GFX-VIEW-3.7), settled
    /// sqrt 21 = 4.58 behind him or sqrt 30 = 5.48 ahead: a step every 0.278 s
    /// of logic time, from him; one standing next to him steps too.
    // spec: SND-8.5, SND-8.6, GFX-ANIM-3.6, GFX-VIEW-3.3, GFX-VIEW-3.7 (8.T4, 8.T5)
    #[test]
    fn steps_belong_to_the_follow_camera() {
        use crate::render::world3d::{CameraRig, Frame, Heights};
        struct Flat(Frame);
        impl Heights for Flat {
            fn frame(&self) -> Frame {
                self.0
            }
            fn raw(&self, _: u32, _: u32) -> i32 {
                0
            }
        }
        let flat = Flat(Frame::new(100, 100));
        let [x, z] = flat.frame().field_xz(50, 50);
        for zoom in 0..=4 {
            let mut rig = CameraRig::new(x, z);
            rig.zoom = zoom;
            let mut eye = rig.update(&flat);
            for _ in 0..200 {
                eye = rig.update(&flat);
            }
            assert!(eye.position[1] >= 10.0, "zoom {zoom}: {:?}", eye.position);
            for (dx, dz) in [(0.0, 0.0), (0.0, -14.0), (0.0, -30.0), (7.0, 7.0)] {
                let under = [eye.position[0] + dx, 0.0, eye.position[2] + dz];
                assert!(!step_near(eye.position, under), "zoom {zoom}");
            }
        }
        // Following: the follow camera settled behind him, walking along +z.
        use crate::render::world3d::{follow_target, FollowPose};
        let him = [x, 0.0, z];
        let forward = [0.0, 1.0];
        let (eye, _) = follow_target(
            FollowPose::Behind,
            him,
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            None,
        );
        let gap = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>();
        assert!((gap(eye, him) - 21.0).abs() < 1e-4);
        let (ahead, _) = follow_target(
            FollowPose::Ahead,
            him,
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            None,
        );
        assert!((gap(ahead, him) - 30.0).abs() < 1e-4);
        assert!(step_near(ahead, him));
        // A step from the first moving tick, one each 0.278 s.
        assert!(step_near(eye, him));
        let mut clock = StepClock::default();
        clock.frame(step_near(eye, him));
        let ticks: Vec<u32> = (0..28).filter(|_| clock.tick(true)).collect();
        // 28 ticks are 2.8 s: 1 + floor(2.7 / 0.2778) steps.
        assert_eq!(ticks.len(), 10);
        // One next to him (one field along the view, 2 units to the side).
        let next = [x + 2.0, 0.0, z + forward[1]];
        assert!(step_near(eye, next));
        // One 8 units on is out of reach.
        let far = [x + 8.0 * forward[0], 0.0, z + 8.0 * forward[1]];
        assert!(!step_near(eye, far));
    }

    /// GFX-ANIM-3.6 (F56): near is a squared camera distance of at most 50,
    /// 7.07 world units, not 50 units; exactly 50 is near.
    // spec: GFX-ANIM-3.6
    #[test]
    fn footsteps_are_near_within_seven_units() {
        let eye = [10.0, 20.0, 30.0];
        assert!(step_near(eye, [15.0, 20.0, 35.0]));
        assert!(step_near(eye, [10.0, 20.0, 37.0]));
        assert!(step_near(eye, [14.0, 15.0, 30.0]));
        assert!(!step_near(eye, [10.0, 20.0, 37.1]));
        assert!(!step_near(eye, [10.0, 20.0, 80.0]));
        // SND-8.1 T1: the eye stands 15 units from the ground under the
        // target at zoom 1: a settler there does not step.
        assert!(!step_near([0.0, 20.21, -14.48], [0.0, 0.0, 0.0]));
    }

    /// SND T7, 2.5: the default volumes and the product.
    #[test]
    fn volumes() {
        let v = Volumes::from_settings([1.0, 0.3, 1.0, 1.0]);
        assert_eq!(v.music, 0.3);
        assert!((channel_volume(&def(20.0), v, Some(40.0)) - 51.0 / 255.0).abs() < 0.005);
        // SND-7.1: int(255 x 0.3) = 76 for a music-type channel.
        let music = SoundDef {
            kind: SoundType::Music,
            ..def(0.0)
        };
        assert!((channel_volume(&music, v, None) - 76.0 / 255.0).abs() < 1e-6);
        assert_eq!(channel_volume(&def(20.0), v, Some(10.0)), 1.0);
    }

    /// SND T5: a Roman and a Chinese player: Rome1..4, RomePer2..4,
    /// China1..3, ChinaPer1, 1a, 1b, 2, 3. Skipped without `S2_GAME_DIR`.
    // spec: SND-3.2, SND-7.2 (T5)
    #[test]
    fn t5_playlist() {
        let Ok(dir) = s2_assets::GameDir::from_env() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let (rules, _) = s2_rules::load(&dir).expect("rules");
        let bank = SoundBank::new(&rules);
        let names: Vec<String> = playlist(&bank, &[0, 2])
            .iter()
            .map(|f| {
                f.rsplit('\\')
                    .next()
                    .unwrap()
                    .trim_end_matches(".mp3")
                    .to_owned()
            })
            .collect();
        assert_eq!(
            names,
            [
                "Rome1",
                "Rome2",
                "Rome3",
                "Rome4",
                "RomePer2",
                "RomePer3",
                "RomePer4",
                "China1",
                "China2",
                "China3",
                "ChinaPer1",
                "ChinaPer1a",
                "ChinaPer1b",
                "ChinaPer2",
                "ChinaPer3"
            ]
        );
        assert!(bank.get(STEPS).is_some() && bank.get(MUSIC_BATTLE).is_some());
        // SND-2.2: the briefing music and the two jingles, by name.
        for name in [MUSIC_BRIEFING, MUSIC_WON, MUSIC_LOST] {
            assert!(
                bank.get(name).is_some_and(|d| !d.files.is_empty()),
                "{name}"
            );
        }
        // The music hook's tribe of a track: the set it comes from.
        for (tribe, first) in [(0, "Rome1"), (1, "Nubia1"), (2, "China1")] {
            let list = playlist(&bank, &[tribe]);
            assert!(list.iter().any(|f| f.contains(first)), "{list:?}");
            assert!(list.iter().all(|f| tribe_of(&bank, f) == Some(tribe)));
        }
        let main = &bank.get(MUSIC_MAIN).expect("the main theme").files[0];
        assert_eq!(tribe_of(&bank, main), None);
    }

    /// GFX-ANIM-2.3: events fire as the clock passes them, each loop.
    #[test]
    fn clip_events() {
        let ev = vec![("chop".to_owned(), 1.24), ("chop2".to_owned(), 2.88)];
        assert_eq!(events_crossed(&ev, 1.0, 1.3, 5.0, true), ["chop"]);
        assert!(events_crossed(&ev, 1.3, 2.0, 5.0, true).is_empty());
        assert_eq!(events_crossed(&ev, 4.9, 6.3, 5.0, true), ["chop"]);
        assert_eq!(events_crossed(&ev, 2.0, 9.0, 5.0, false), ["chop2"]);
        assert!(events_crossed(&ev, 5.0, 9.0, 5.0, false).is_empty());
    }

    /// SND T6: the battle loop starts at the fight and stops 20 s after
    /// the last blow; the playlist then goes on (SND-7.2: in order, no
    /// gap).
    #[test]
    fn battle_music() {
        let list = vec!["Rome1.mp3".to_owned(), "Rome2.mp3".to_owned()];
        let mut m = Music::default();
        assert_eq!(
            m.game(0.0, false, false, &list, Some("Battle.wav")),
            Some(MusicCommand::Play {
                file: "Rome1.mp3".into(),
                looping: false
            })
        );
        assert_eq!(m.game(1.0, false, false, &list, Some("Battle.wav")), None);
        assert_eq!(
            m.game(5.0, true, false, &list, Some("Battle.wav")),
            Some(MusicCommand::Play {
                file: "Battle.wav".into(),
                looping: true
            })
        );
        assert_eq!(m.game(20.0, false, false, &list, Some("Battle.wav")), None);
        assert_eq!(
            m.game(25.0, false, false, &list, Some("Battle.wav")),
            Some(MusicCommand::Stop)
        );
        assert_eq!(
            m.game(25.1, false, false, &list, Some("Battle.wav")),
            Some(MusicCommand::Play {
                file: "Rome2.mp3".into(),
                looping: false
            })
        );
    }
}
