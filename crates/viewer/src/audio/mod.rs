//! Playing the sounds and music of `sound.rs` (`specs/SND-sound-music.md`):
//! the requests of the interface and the 3D view, the code events read off
//! the world (footsteps, falling trees, collapses, catapults), ambient
//! emitters near the camera, and the music. The output is Bevy's audio,
//! built for Windows (the game's platform); elsewhere the requests are
//! dropped.

pub(crate) mod sound;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use s2_sim::grid::Point;

use self::sound::{
    Channels, Held, Music, MusicCommand, MusicSource, SoundBank, SoundRequest, Start, Voice,
    Volumes,
};
use crate::render::world3d::Heights;
use crate::world::scene3d::MoverId;

/// Sounds asked for this frame.
#[derive(Resource, Default)]
pub struct SoundQueue(pub Vec<SoundRequest>);

/// `--bench` (F56): the sounds started and the most playing at once, by the
/// program's own playing of them.
#[derive(Resource, Default)]
pub(crate) struct SoundStats {
    /// Sounds started since the timing began, by definition name.
    pub(crate) started: std::collections::BTreeMap<String, u32>,
    /// The most one-shot sounds playing in one frame.
    pub(crate) most_at_once: usize,
    /// SND-2.6b: sounds lost for want of a channel.
    pub(crate) refused: u32,
    /// SND-2.6b: sounds cut for a new one.
    pub(crate) cut: u32,
}

/// What holds a channel (SND-2.6a), as the output plays it.
#[derive(Clone)]
struct Playing {
    entity: Entity,
    /// Where a positioned sound is (SND-2.4: where it was fired, or where
    /// its moving source was last seen) and what decides its volume; the
    /// music and the loops set their own.
    fired: Option<(Option<[f32; 3]>, Voice)>,
    /// SND-2.4: the moving object a sound follows.
    source: Option<MoverId>,
    /// SND-8.3: the stereo gains a positioned sound plays with.
    pan: Option<Arc<sound::PanGains>>,
    /// The volume last given to the output.
    sent: f32,
}

/// SND-2.4: where a mover is drawn now, in the game's frame.
fn mover_at(
    movers: Option<&crate::render::view3d::Movers>,
    transforms: &Query<&Transform>,
    id: MoverId,
) -> Option<[f32; 3]> {
    let state = movers?.states.get(&id)?;
    let t = transforms.get(state.entity).ok()?;
    Some(crate::render::world3d::from_bevy(t.translation.to_array()))
}

/// SND-8.1, SND-8.3a: the listener point and its right the pan is measured
/// from (the right from the camera's view direction); none in the 2D view,
/// where sounds play in the middle.
fn pan_view(
    view: &crate::render::view3d::View3d,
    camera: Option<&Transform>,
) -> Option<([f32; 3], [f32; 3])> {
    let camera = camera.filter(|_| view.active)?;
    let ear = view.ear?;
    let forward = crate::render::world3d::from_bevy(camera.forward().to_array());
    Some((ear, sound::listener_right(forward)?))
}

/// What the world looked like last frame, for its sound events.
#[derive(Default)]
struct Seen {
    felling: HashSet<Point>,
    buildings: HashSet<(Point, u32)>,
    flying: HashMap<Point, Point>,
    loads: u64,
}

#[derive(Resource, Default)]
pub struct AudioState {
    bank: Option<Arc<SoundBank>>,
    root: Option<PathBuf>,
    files: HashMap<String, Option<Arc<[u8]>>>,
    /// SND-2.6: the library's 32 channels.
    channels: Channels<Playing>,
    music: Music,
    music_entity: Option<Entity>,
    /// SND-7.4: the ambient player and the loop playing for each sound.
    ambient_player: sound::Ambients,
    ambients: HashMap<u32, Entity>,
    steps: HashMap<u64, sound::StepClock>,
    /// The tick the footsteps were last counted for.
    step_tick: Option<u64>,
    /// The walking settlers and carriers as of the tick and map load the
    /// key names: id, field, whether moving, the field's point (F51: read
    /// once a tick, not every frame).
    walkers: Vec<(u64, Point, bool, [f32; 3], MoverId)>,
    walkers_for: Option<(u64, u64)>,
    seen: Seen,
    random: u64,
}

impl AudioState {
    fn next_random(&mut self) -> u64 {
        // xorshift64*: which file of a set plays (SND-1.2).
        let mut x = self.random.max(1);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn bytes(&mut self, file: &str) -> Option<Arc<[u8]>> {
        if let Some(found) = self.files.get(file) {
            return found.clone();
        }
        let data = self.root.as_ref().and_then(|root| {
            let path = crate::extend::find(root, file).ok()?;
            // IMA ADPCM files become PCM, which the decoder reads.
            crate::files::read(&path)
                .ok()
                .map(|d| Arc::from(s2_assets::wav::playable(&d)))
        });
        if data.is_none() {
            eprintln!("sound file {file} not found");
        }
        self.files.insert(file.to_owned(), data.clone());
        data
    }
}

/// GFX-OBJ-8.7 (4), SND-2.2: the fields whose finished building (field,
/// serial) went between two ticks: a collapse each, in field order. Not a
/// capture, whose new building stands finished on the same field at once
/// (2.2a (4)); an upgrade keeps its building.
pub(crate) fn collapses(before: &HashSet<(Point, u32)>, now: &HashSet<(Point, u32)>) -> Vec<Point> {
    let mut gone: Vec<Point> = before
        .difference(now)
        .map(|&(f, _)| f)
        .filter(|f| !now.iter().any(|(at, _)| at == f))
        .collect();
    gone.sort_unstable();
    gone.dedup();
    gone
}

/// SND-2.2: the code events read off the world each frame: a footstep of
/// every walking settler and carrier every 0.5 s, a tree that starts to
/// fall, a building that goes, a catapult's shot and impact. SND-3.4:
/// whether the player fights.
pub fn world_events(
    viewer: Res<crate::game::Viewer>,
    view: Res<crate::render::view3d::View3d>,
    camera: Single<&Transform, With<crate::render::view3d::MapCamera3d>>,
    mut state: ResMut<AudioState>,
    mut queue: ResMut<SoundQueue>,
    mut ui: ResMut<crate::ui::render::UiEvents>,
) {
    for name in ui.sounds.drain(..) {
        queue.0.push(SoundRequest::Gui(name.to_owned()));
    }
    let Some(world) = &viewer.world else { return };
    if viewer.paused {
        return;
    }
    let frame = world.terrain().frame();
    let at = |p: Point| -> [f32; 3] {
        let [x, z] = frame.field_xz(p.0, p.1);
        [
            x,
            crate::render::world3d::ground_height(world.terrain(), x, z),
            z,
        ]
    };
    // GFX-ANIM-3.6: footsteps of every visible settler and carrier: the
    // visibility counter per frame, the step accumulator per logic tick.
    // SND-8.6: the gate measures from the camera's eye; while it follows a
    // settler that is the follow camera's eye (GFX-VIEW-3.7), which trails
    // him.
    let eye = crate::render::world3d::from_bevy(camera.translation.to_array());
    let near = |p: [f32; 3]| !view.active || sound::step_near(eye, p);
    let ticks = match state.step_tick {
        Some(t) if world.tick() >= t => world.tick() - t,
        _ => 0,
    };
    state.step_tick = Some(world.tick());
    // The world changes once a tick: the walkers and the seen things are
    // read then, the visibility counters every frame (F51).
    let key = (world.tick(), viewer.loads);
    let new_tick = state.walkers_for != Some(key);
    if new_tick {
        state.walkers_for = Some(key);
        state.walkers.clear();
        for v in world.settler_views() {
            let (id, moving, mover) = match v.id {
                s2_sim::view::SettlerRef::Settler(id) if !v.hidden => {
                    let moving = match &v.fine {
                        Some(fine) => fine.step.next.is_some(),
                        None => v.walk.next.is_some(),
                    };
                    (id as u64, moving, MoverId::Settler(id))
                }
                s2_sim::view::SettlerRef::Carrier(i) => (
                    (1 << 32) | i as u64,
                    v.walk.next.is_some(),
                    MoverId::Carrier(i),
                ),
                _ => continue,
            };
            state
                .walkers
                .push((id, v.walk.at, moving, at(v.walk.at), mover));
        }
    }
    let AudioState { walkers, steps, .. } = &mut *state;
    for &(key, field, moving, point, mover) in walkers.iter() {
        let clock = steps.entry(key).or_default();
        clock.frame(near(point));
        let mut sounds = 0;
        for _ in 0..ticks.min(10) {
            sounds += u32::from(clock.tick(moving));
        }
        if sounds > 0 {
            // SND-2.4, SND-8.4: at the walking settler, following him.
            queue.0.push(SoundRequest::At {
                name: sound::STEPS.into(),
                at: at(field),
                source: Some(mover),
            });
        }
    }
    if !new_tick {
        return;
    }
    let present: HashSet<u64> = state.walkers.iter().map(|w| w.0).collect();
    state.steps.retain(|k, _| present.contains(k));
    // A new map starts with nothing seen.
    let fresh = state.seen.loads != viewer.loads;
    state.seen.loads = viewer.loads;
    // SND-2.2, GFX-ANIM-3.8a: the tree fall sounds when the tree starts to
    // fall (its woodcutter's work clip at 3.0 s).
    let felling: HashSet<Point> = world
        .deposits()
        .filter(|&(f, _)| {
            world
                .felling_began(f)
                .is_some_and(|b| world.tick() >= crate::world::scene3d::fall_start(b))
        })
        .map(|(f, _)| f)
        .collect();
    let buildings: HashSet<(Point, u32)> = world
        .buildings()
        .filter(|b| !b.under_construction)
        .map(|b| (b.at, b.serial))
        .collect();
    let mut flying = HashMap::new();
    for b in world.buildings() {
        if let Some(c) = world.catapult_at(b.at) {
            if let s2_sim::CatapultPhase::Flying { landing, .. } = c.phase {
                flying.insert(b.at, landing);
            }
        }
    }
    if !fresh {
        for f in felling.difference(&state.seen.felling) {
            queue.0.push(SoundRequest::At {
                name: sound::TREE_FALL.into(),
                at: at(*f),
                source: None,
            });
        }
        for f in collapses(&state.seen.buildings, &buildings) {
            queue.0.push(SoundRequest::At {
                name: sound::COLLAPSE.into(),
                at: at(f),
                source: None,
            });
        }
        for (c, landing) in &flying {
            if !state.seen.flying.contains_key(c) {
                queue.0.push(SoundRequest::At {
                    name: sound::CATAPULT_SHOOT.into(),
                    at: at(*c),
                    source: None,
                });
            }
            let _ = landing;
        }
        for (c, landing) in &state.seen.flying {
            if !flying.contains_key(c) {
                queue.0.push(SoundRequest::At {
                    name: sound::CATAPULT_IMPACT.into(),
                    at: at(*landing),
                    source: None,
                });
            }
        }
    }
    state.seen.felling = felling;
    state.seen.buildings = buildings;
    state.seen.flying = flying;
}

/// SND-3.4: the player is in a fight: one of its soldiers is in a duel.
fn fighting(world: &s2_sim::World) -> bool {
    let mine = |id: usize| {
        world
            .settlers()
            .any(|(i, s)| i == id && s.player == crate::game::PLAYER)
    };
    world
        .duels()
        .iter()
        .any(|d| mine(d.attacker.0) || mine(d.defender.0))
}

/// SND-8.1: the listener, set by the 3D camera each frame; the 2D view
/// listens at its centre.
fn listener(view: &crate::render::view3d::View3d) -> [f32; 3] {
    match view.ear.filter(|_| view.active) {
        Some(ear) => ear,
        None => [view.rig.target[0], view.rig.target[1], view.rig.target[2]],
    }
}

/// SND-2.6a: a music stream or an ambient loop takes a channel by the rule of
/// 2.6b and holds it; nothing takes it from there.
fn start_set_aside(
    channels: &mut Channels<Playing>,
    out: &mut output::Out,
    priority: i64,
    play: impl FnOnce(&mut output::Out) -> Option<Entity>,
) -> Option<Entity> {
    let slot = match channels.choose(priority) {
        Start::Free(slot) => slot,
        Start::Cut(slot, cut) => {
            out.stop(cut.entity);
            slot
        }
        Start::Refused => return None,
    };
    let entity = play(out)?;
    channels.hold(
        slot,
        Held {
            handle: Playing {
                entity,
                fired: None,
                source: None,
                pan: None,
                sent: 0.0,
            },
            priority,
            level: 255,
            set_aside: true,
        },
    );
    Some(entity)
}

/// The distance from the listener to a place.
fn distance_to(ear: [f32; 3], at: [f32; 3]) -> f32 {
    (0..3).map(|k| (at[k] - ear[k]).powi(2)).sum::<f32>().sqrt()
}

/// Play the requested sounds, the music and the ambient emitters.
#[allow(clippy::too_many_arguments)]
pub fn play(
    time: Res<Time>,
    viewer: Res<crate::game::Viewer>,
    view: Res<crate::render::view3d::View3d>,
    menu: Res<crate::menus::menu::MenuState>,
    mission: Res<crate::menus::campaign::MissionUi>,
    mut state: ResMut<AudioState>,
    mut queue: ResMut<SoundQueue>,
    mut slots: MessageWriter<crate::extend::MusicSlot>,
    mut stats: Option<ResMut<SoundStats>>,
    movers: Option<Res<crate::render::view3d::Movers>>,
    cameras: Query<&Transform, With<crate::render::view3d::MapCamera3d>>,
    transforms: Query<&Transform>,
    mut out: output::Out,
) {
    if state.bank.is_none() {
        if let Some(rules) = &viewer.rules {
            state.bank = Some(Arc::new(SoundBank::new(rules)));
            state.root = viewer.game_root.clone();
        }
    }
    let requests = std::mem::take(&mut queue.0);
    let Some(bank) = state.bank.clone() else {
        return;
    };
    let volumes = Volumes::from_settings(menu.settings.volumes);
    let ear = listener(&view);
    let panning = pan_view(&view, cameras.single().ok());
    // SND-8.3a: a positioned sound's pan (the middle, 128, in the 2D view).
    let pan_at = |at: [f32; 3]| panning.map_or(128, |(ear, right)| sound::pan(ear, right, at));
    // SND-2.6: finished channels are released at the next audio update.
    state.channels.retain(|held| out.alive(held.handle.entity));
    // SND-2.4: a sound fired for a moving object is where the object is
    // now. SND-8.3, SND-7.1: a playing sound's level and pan follow the
    // listener, its place and the sliders.
    for held in state.channels.held_mut() {
        let Some((at, voice)) = held.handle.fired else {
            continue;
        };
        let at = match (at, held.handle.source) {
            (Some(last), Some(id)) => {
                let now = sound::position_now(last, mover_at(movers.as_deref(), &transforms, id));
                held.handle.fired = Some((Some(now), voice));
                Some(now)
            }
            _ => at,
        };
        let pan_now = at.filter(|_| held.handle.pan.is_some()).map(pan_at);
        if let (Some(p), Some(gains)) = (pan_now, &held.handle.pan) {
            gains.set(sound::pan_gains(p));
        }
        let distance = at.map(|at| distance_to(ear, at));
        held.level = voice.present_level(volumes, distance, pan_now);
        let volume = voice.volume(volumes, distance);
        if volume != held.handle.sent {
            held.handle.sent = volume;
            out.set_volume(held.handle.entity, volume);
        }
    }
    for r in requests {
        let (def, at, source) = match &r {
            SoundRequest::Gui(name) => match bank.get(name) {
                Some(d) => (d, None, None),
                None => continue,
            },
            SoundRequest::At { name, at, source } => {
                let Some(d) = bank.get(name) else { continue };
                let d2: f32 = (0..3).map(|k| (at[k] - ear[k]).powi(2)).sum();
                if !sound::audible(d2, d.min_distance) {
                    continue;
                }
                (d, Some(*at), *source)
            }
        };
        let distance = at.map(|at| distance_to(ear, at));
        // SND-2.6b: a free channel, or the one this sound may cut, or none:
        // then it is lost, with no retry.
        let start = state.channels.choose(def.priority);
        if matches!(start, Start::Refused) {
            if let Some(stats) = stats.as_mut() {
                stats.refused += 1;
            }
            continue;
        }
        let random = state.next_random();
        let Some(file) = sound::pick_file(def, random).map(str::to_owned) else {
            continue;
        };
        let Some(bytes) = state.bytes(&file) else {
            continue;
        };
        let slot = match start {
            Start::Free(slot) => slot,
            Start::Cut(slot, cut) => {
                out.stop(cut.entity);
                if let Some(stats) = stats.as_mut() {
                    stats.cut += 1;
                }
                slot
            }
            Start::Refused => continue,
        };
        let volume = def.voice().volume(volumes, distance);
        // SND-8.3a: a positioned sound pans; a GUI sound plays in the middle.
        let pan_now = at.map(pan_at);
        let pan = pan_now.map(|p| Arc::new(sound::PanGains::new(sound::pan_gains(p))));
        if let Some(entity) = out.play(&file, bytes, volume, false, pan.clone()) {
            state.channels.hold(
                slot,
                Held {
                    handle: Playing {
                        entity,
                        fired: Some((at, def.voice())),
                        source,
                        pan,
                        sent: volume,
                    },
                    priority: def.priority,
                    level: def.voice().present_level(volumes, distance, pan_now),
                    set_aside: false,
                },
            );
            if let Some(stats) = stats.as_mut() {
                *stats.started.entry(def.name.clone()).or_default() += 1;
            }
        }
    }
    if let Some(stats) = stats.as_mut() {
        let loops = state.ambients.len() + usize::from(state.music_entity.is_some());
        stats.most_at_once = stats.most_at_once.max(out.playing().saturating_sub(loops));
    }
    // SND-3: the music.
    let ended = state.music_entity.is_some_and(|e| !out.alive(e));
    // SND-2.2, CAM-3.6: the end dialog's jingle and the book's briefing
    // music, over the game's playlist.
    let interface = sound::interface_music(
        mission
            .end
            .map(|e| matches!(e, crate::menus::campaign::EndScreen::Victory(_))),
        mission.book.is_some(),
    );
    let (command, source) = match menu.screen {
        // UI-MENU-0.1: the intro brings its own sound.
        crate::menus::menu::Screen::Intro => (None, MusicSource::Menu),
        crate::menus::menu::Screen::Credits => (
            bank.get(sound::MUSIC_CREDITS)
                .and_then(|d| d.files.first().cloned())
                .and_then(|f| state.music.menu(&f)),
            MusicSource::Credits,
        ),
        crate::menus::menu::Screen::None | crate::menus::menu::Screen::GameMenu
            if interface.is_some() =>
        {
            let (name, looping) = interface.expect("checked");
            (
                bank.get(name)
                    .and_then(|d| d.files.first().cloned())
                    .and_then(|f| state.music.slot(&f, looping)),
                MusicSource::Interface(name),
            )
        }
        crate::menus::menu::Screen::None | crate::menus::menu::Screen::GameMenu
            if viewer.world.is_some() && menu.in_game =>
        {
            let world = viewer.world.as_ref().expect("checked");
            let tribes: Vec<u8> = (0..8u8)
                .map(s2_sim::PlayerId)
                .filter(|&p| world.is_player(p))
                .map(|p| world.tribe(p))
                .collect();
            let list = sound::playlist(&bank, &tribes);
            let battle = bank
                .get(sound::MUSIC_BATTLE)
                .and_then(|d| d.files.first().cloned());
            let command = state.music.game(
                time.elapsed_secs_f64(),
                fighting(world),
                ended,
                &list,
                battle.as_deref(),
            );
            (command, MusicSource::Game)
        }
        _ => (
            bank.get(sound::MUSIC_MAIN)
                .and_then(|d| d.files.first().cloned())
                .and_then(|f| state.music.menu(&f)),
            MusicSource::Menu,
        ),
    };
    // A project's music (`docs/extending.md`): what the player switched to.
    if let Some(slot) = command
        .as_ref()
        .and_then(|c| sound::music_slot(source, c, |f| sound::tribe_of(&bank, f)))
    {
        slots.write(slot);
    }
    // SND-7.1: the music stream plays at the music volume; the master is
    // the library's effects master. Silent when a project plays the music.
    let music_volume = sound::music_volume(crate::extend::music_external(), volumes.music);
    match command {
        Some(MusicCommand::Play { file, looping }) => {
            if let Some(e) = state.music_entity.take() {
                state.channels.retain(|held| held.handle.entity != e);
                out.stop(e);
            }
            if let Some(bytes) = state.bytes(&file) {
                state.music_entity = start_set_aside(
                    &mut state.channels,
                    &mut out,
                    sound::MUSIC_PRIORITY,
                    |out| out.play(&file, bytes, music_volume, looping, None),
                );
            }
        }
        Some(MusicCommand::Stop) => {
            if let Some(e) = state.music_entity.take() {
                state.channels.retain(|held| held.handle.entity != e);
                out.stop(e);
            }
        }
        None => {
            if let Some(e) = state.music_entity {
                out.set_volume(e, music_volume);
            }
        }
    }
    // SND-7.4: the four nearest emitters of different sounds play, looping,
    // their levels by distance and height moving at most 255 a second of
    // real time; a dropped one fades out and stops.
    let mut emitters = Vec::new();
    if menu.screen == crate::menus::menu::Screen::None {
        if let (Some(map), Some(world)) = (&viewer.map, &viewer.world) {
            let frame = world.terrain().frame();
            for a in &map.ambients {
                let Some(def) = bank.by_id(a.sound_id) else {
                    continue;
                };
                let [x, z] = frame.field_xz(a.field.x, a.field.y);
                let y = crate::render::world3d::ground_height(world.terrain(), x, z);
                emitters.push(sound::Emitter {
                    sound: a.sound_id,
                    at: [x, y, z],
                    volume: (def.volume * 255.0).round(),
                    distance: def.distance_2d,
                });
            }
        }
    }
    let changes = state
        .ambient_player
        .update(ear, &emitters, time.delta_secs());
    for id in changes.stop {
        if let Some(e) = state.ambients.remove(&id) {
            state.channels.retain(|held| held.handle.entity != e);
            out.stop(e);
        }
    }
    for id in changes.start {
        let file = bank
            .by_id(id)
            .and_then(|def| Some((def.priority, def.files.first()?.clone())));
        let started = file.and_then(|(priority, file)| {
            let bytes = state.bytes(&file)?;
            start_set_aside(&mut state.channels, &mut out, priority, |out| {
                out.play(&file, bytes, 0.0, true, None)
            })
        });
        match started {
            Some(e) => {
                state.ambients.insert(id, e);
            }
            // SND-7.4 4: tried again in the next update.
            None => state.ambient_player.refused(id),
        }
    }
    let master = volumes.master;
    for p in &state.ambient_player.playing {
        if let Some(&e) = state.ambients.get(&p.sound) {
            out.set_volume(e, sound::Ambients::volume(p) * master);
        }
    }
}

#[cfg(windows)]
pub(crate) mod output {
    use std::collections::HashMap;
    use std::sync::Arc;

    use bevy::audio::{
        AddAudioSource, AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, ChannelCount,
        Decodable, PlaybackSettings, SampleRate, Source, Volume,
    };
    use bevy::ecs::system::SystemParam;
    use bevy::prelude::*;

    use super::sound::PanGains;

    /// SND-8.3: a positioned sound with its own stereo gains, which the game
    /// changes while it plays (one asset per sound playing, gone with it).
    #[derive(Asset, TypePath, Clone)]
    pub struct PannedSound {
        sound: AudioSource,
        gains: Arc<PanGains>,
    }

    /// The file's samples, two channels out, each times its gain.
    pub struct PannedDecoder {
        inner: <AudioSource as Decodable>::Decoder,
        gains: Arc<PanGains>,
        now: [f32; 2],
        /// A mono file's right sample, waiting.
        right: Option<f32>,
        mono: bool,
        /// The next sample's channel of a two-channel file.
        channel: usize,
        /// Samples until the gains are read again.
        until_read: u32,
    }

    impl Iterator for PannedDecoder {
        type Item = bevy::audio::Sample;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(r) = self.right.take() {
                return Some(r);
            }
            if self.until_read == 0 {
                self.now = self.gains.get();
                self.until_read = 256;
            }
            self.until_read -= 1;
            let sample = self.inner.next()?;
            if self.mono {
                self.right = Some(sample * self.now[1]);
                return Some(sample * self.now[0]);
            }
            let out = sample * self.now[self.channel.min(1)];
            self.channel = (self.channel + 1) % usize::from(self.inner.channels().get().max(1));
            Some(out)
        }
    }

    impl Source for PannedDecoder {
        fn current_span_len(&self) -> Option<usize> {
            let n = self.inner.current_span_len()?;
            Some(if self.mono { 2 * n } else { n })
        }
        fn channels(&self) -> ChannelCount {
            if self.mono {
                ChannelCount::new(2).expect("two")
            } else {
                self.inner.channels()
            }
        }
        fn sample_rate(&self) -> SampleRate {
            self.inner.sample_rate()
        }
        fn total_duration(&self) -> Option<std::time::Duration> {
            self.inner.total_duration()
        }
    }

    impl Decodable for PannedSound {
        type Decoder = PannedDecoder;

        fn decoder(&self) -> PannedDecoder {
            let inner = self.sound.decoder();
            let mono = inner.channels().get() == 1;
            PannedDecoder {
                inner,
                gains: self.gains.clone(),
                now: self.gains.get(),
                right: None,
                mono,
                channel: 0,
                until_read: 256,
            }
        }
    }

    /// The output's sources, to the app.
    pub fn register(app: &mut App) {
        app.add_audio_source::<PannedSound>();
    }

    #[derive(Resource, Default)]
    pub struct Sources(HashMap<String, Handle<AudioSource>>);

    /// An entity playing a file, panned or not.
    type Playable = Or<(With<AudioPlayer>, With<AudioPlayer<PannedSound>>)>;

    #[derive(SystemParam)]
    pub struct Out<'w, 's> {
        commands: Commands<'w, 's>,
        assets: ResMut<'w, Assets<AudioSource>>,
        panned: ResMut<'w, Assets<PannedSound>>,
        sources: Local<'s, Sources>,
        sinks: Query<'w, 's, &'static mut AudioSink>,
        alive: Query<'w, 's, (), Playable>,
    }

    impl Out<'_, '_> {
        /// Play a file; with `pan`, positioned, at the gains it holds.
        pub fn play(
            &mut self,
            file: &str,
            bytes: Arc<[u8]>,
            volume: f32,
            looping: bool,
            pan: Option<Arc<PanGains>>,
        ) -> Option<Entity> {
            let settings = if looping {
                PlaybackSettings::LOOP
            } else {
                PlaybackSettings::DESPAWN
            }
            .with_volume(Volume::Linear(volume.max(0.0)));
            if let Some(gains) = pan {
                let handle = self.panned.add(PannedSound {
                    sound: AudioSource { bytes },
                    gains,
                });
                return Some(self.commands.spawn((AudioPlayer(handle), settings)).id());
            }
            let handle = self
                .sources
                .0
                .entry(file.to_owned())
                .or_insert_with(|| self.assets.add(AudioSource { bytes }))
                .clone();
            Some(
                self.commands
                    .spawn((AudioPlayer::new(handle), settings))
                    .id(),
            )
        }

        pub fn set_volume(&mut self, e: Entity, volume: f32) {
            if let Ok(mut sink) = self.sinks.get_mut(e) {
                sink.set_volume(Volume::Linear(volume.max(0.0)));
            }
        }

        pub fn stop(&mut self, e: Entity) {
            if let Ok(mut c) = self.commands.get_entity(e) {
                c.despawn();
            }
        }

        pub fn alive(&self, e: Entity) -> bool {
            self.alive.get(e).is_ok()
        }

        /// Everything playing now.
        pub fn playing(&self) -> usize {
            self.alive.iter().count()
        }
    }
}

#[cfg(not(windows))]
pub(crate) mod output {
    use std::sync::Arc;

    use bevy::ecs::system::SystemParam;
    use bevy::prelude::*;

    /// No audio output on this platform: requests are dropped.
    #[derive(SystemParam)]
    pub struct Out<'w> {
        _time: Res<'w, Time>,
    }

    /// No sources to add.
    pub fn register(_: &mut App) {}

    impl Out<'_> {
        pub fn play(
            &mut self,
            _: &str,
            _: Arc<[u8]>,
            _: f32,
            _: bool,
            _: Option<Arc<super::sound::PanGains>>,
        ) -> Option<Entity> {
            None
        }
        pub fn set_volume(&mut self, _: Entity, _: f32) {}
        pub fn stop(&mut self, _: Entity) {}
        pub fn alive(&self, _: Entity) -> bool {
            true
        }
        pub fn playing(&self) -> usize {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(world: &s2_sim::World) -> HashSet<(Point, u32)> {
        world
            .buildings()
            .filter(|b| !b.under_construction)
            .map(|b| (b.at, b.serial))
            .collect()
    }

    // Play test 50 (GFX-OBJ-8.7 (4), SND-2.2): a finished building the
    // player tears down gives the collapse sound once, at its field, over
    // the whole collapse and the ruin's 333 ticks; another building
    // standing on gives none.
    // spec: GFX-OBJ-8.7, SND-2.2
    #[test]
    fn a_tear_down_plays_the_collapse_sound_once() {
        let n = 30 * 30;
        let terrain = s2_sim::map::Terrain {
            width: 30,
            height: 30,
            heights: vec![0; n],
            height_floor: -100,
            states: vec![s2_sim::map::state::FOR_BUILDING; n],
            samples: Vec::new(),
        };
        let rules = s2_sim::Rules {
            building_types: vec![s2_sim::BuildingType::new(7, s2_sim::BuildingSize::Small)],
            ..s2_sim::Rules::empty()
        };
        let p0 = s2_sim::PlayerId(0);
        let mut world = s2_sim::World::with_map(1, terrain, rules, p0);
        for at in [(10, 10), (16, 10)] {
            world.submit(s2_sim::Command::CreateBuilding {
                player: p0,
                building: 7,
                at,
                settler: None,
            });
        }
        world.step();
        assert_eq!(finished(&world).len(), 2);
        world.submit(s2_sim::Command::RemoveBuilding {
            player: p0,
            at: (10, 10),
        });
        let mut seen = finished(&world);
        let mut heard = Vec::new();
        for _ in 0..600 {
            world.step();
            let now = finished(&world);
            heard.extend(collapses(&seen, &now));
            seen = now;
        }
        assert_eq!(heard, [(10, 10)]);
    }

    // GFX-OBJ-8.7 (4), 2.2a (4): a captured building (another serial on the
    // same field at once) and a building that stays are no collapse.
    // spec: GFX-OBJ-8.7
    #[test]
    fn a_capture_is_no_collapse() {
        let before: HashSet<(Point, u32)> = [((3, 3), 1), ((5, 5), 2), ((7, 7), 3)].into();
        let now: HashSet<(Point, u32)> = [((3, 3), 9), ((5, 5), 2)].into();
        assert_eq!(collapses(&before, &now), [(7, 7)]);
        assert!(collapses(&now, &now).is_empty());
    }
}
