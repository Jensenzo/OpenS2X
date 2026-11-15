//! The computer opponent (`specs/AI-overview.md`, `AI-construction.md`,
//! `AI-military.md`, `AI-misc.md`).
//!
//! The AI only reads the world. Each tick [`AiSystem::tick`] looks at it and
//! gives back the [`Command`]s the computer players issue, which the caller
//! submits like a human player's. Same seed, same world and same commands
//! give the same AI.

pub mod campaign_player;
pub mod cells;
pub mod config;
mod construction;
mod control;
mod demolition;
pub mod expansion;
pub mod fixture;
pub mod maps;
mod military;
pub mod mission;
mod needs;
pub mod plan;
mod planters;
pub mod player;
pub mod resources;
mod roads;
pub mod run;
pub mod save;
pub mod schedule;
mod ships;
pub mod sites;
mod snap;
pub mod snapshot;
pub mod takeover;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_construction;
#[cfg(test)]
mod tests_military;
#[cfg(test)]
mod tests_ships;
#[cfg(test)]
mod testutil;
pub mod view;

use s2_sim::{Command, Numbers, PlayerId, World};

pub use config::{AiAlternative, AiConfig, Mode, Profile};
pub use player::{AiPlayer, Run};

use maps::MilitaryMap;
use snapshot::{Snapshot, SLOTS};

/// Every player of `world` as a computer player of profile 2 (normal,
/// AI-MISC-2.1), for [`AiSystem::activate_all`]: the start of a headless
/// game of computer players (`s2-app hash`, `soak`, `trace`; `docs/api.md`).
pub fn every_player(world: &World) -> Vec<(PlayerId, u8)> {
    (0..SLOTS as u8)
        .map(PlayerId)
        .filter(|&p| world.is_player(p))
        .map(|p| (p, 2))
        .collect()
}

/// The AI system: one AI object per computer player slot (AI-2.1).
pub struct AiSystem {
    config: AiConfig,
    /// AI-2.1: the AI system's own tick counter `c`.
    counter: u64,
    players: Vec<Option<AiPlayer>>,
    /// AI-MIL-1.1: a military map per player slot.
    military: Vec<MilitaryMap>,
    /// Set to record every part run (for tests of the schedule).
    pub trace: Option<Vec<Run>>,
}

impl AiSystem {
    /// An AI system for `world`'s map. It has no random numbers of its own:
    /// it draws from the simulation's generator (AI-2.7).
    pub fn new(config: AiConfig, world: &World) -> Self {
        let terrain = world.terrain();
        Self {
            config,
            counter: 0,
            players: (0..SLOTS).map(|_| None).collect(),
            military: (0..SLOTS)
                .map(|_| MilitaryMap::new(terrain.width, terrain.height))
                .collect(),
            trace: None,
        }
    }

    pub fn config(&self) -> &AiConfig {
        &self.config
    }

    /// AI-MISC-2.1: switch the player `player` to a computer player with
    /// profile `profile` (1 weak, 2 normal, 3 strong). `false` if the slot
    /// or profile does not exist.
    pub fn activate(&mut self, world: &mut World, player: PlayerId, profile: u8) -> bool {
        world.lend(|world, _, numbers, _| self.activate_with(world, numbers, player, profile))
    }

    /// [`AiSystem::activate`] with the simulation's object counter lent as
    /// `numbers` (SIM-START-4.2: the records the activation makes take
    /// their numbers from it).
    pub fn activate_with(
        &mut self,
        world: &World,
        numbers: &mut Numbers,
        player: PlayerId,
        profile: u8,
    ) -> bool {
        let slot = usize::from(player.0);
        if slot >= SLOTS {
            return false;
        }
        let Some(mut ai) = AiPlayer::new(world, &self.config, player, profile) else {
            return false;
        };
        let snapshot = Snapshot::take(world);
        // The buildings the player already has are announced to it.
        let made = ai.records_made();
        let numbered = ai.update(world, &self.config, &snapshot);
        // SIM-START-4.4 b: the cell of its headquarters takes its number
        // here, after the castle's flag (in the original between the castle
        // and its flag, T6 90181: only hidden numbers change places).
        ai.number_records(made, numbered, numbers);
        ai.resources.refresh_all(world, player, &self.config);
        ai.take_counts(world, &snapshot);
        self.military[slot].refresh_all(world, snapshot.of(player), &self.config);
        self.players[slot] = Some(ai);
        true
    }

    /// Activate the setup's computer slots: `(slot, profile)` pairs, the
    /// profile being the setup's strength 0/1/2 plus one (AI-MISC-2.1);
    /// [`every_player`] gives every player of the world.
    pub fn activate_all(&mut self, world: &mut World, slots: &[(PlayerId, u8)]) {
        for &(player, profile) in slots {
            self.activate(world, player, profile);
        }
    }

    /// Whether `player` is a computer player that is still switched on.
    pub fn is_active(&self, player: PlayerId) -> bool {
        self.player(player).is_some_and(|p| p.active)
    }

    pub fn player(&self, player: PlayerId) -> Option<&AiPlayer> {
        self.players.get(usize::from(player.0))?.as_ref()
    }

    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// A 64-bit FNV-1a hash of the AI's whole changing state (its tick
    /// counter, players and military maps), taken over the
    /// `Debug` form like [`World::state_hash`]: every collection in it is
    /// ordered.
    pub fn state_hash(&self) -> u64 {
        let text = format!("{:?}", (self.counter, &self.players, &self.military));
        text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// Write the AI's changing state (the part [`AiSystem::state_hash`]
    /// covers) to a snapshot (`docs/snapshot.md`). The configuration comes
    /// from the rules and is not written.
    pub fn put_state(&self, w: &mut s2_sim::snap::Writer) {
        w.put(&self.counter);
        w.put(&self.players);
        w.put(&self.military);
    }

    /// What [`AiSystem::put_state`] writes, for the format's description.
    pub fn state_shape(s: &mut s2_sim::snap::Shape) -> String {
        s.define("s2_ai::AiSystem", |s| {
            vec![
                s.field("counter", |a: &Self| &a.counter),
                s.field("players", |a: &Self| &a.players),
                s.field("military", |a: &Self| &a.military),
            ]
        })
    }

    /// Read back what [`AiSystem::put_state`] wrote, in place of this
    /// system's state; the reader is in the format of the file.
    pub fn get_state(
        &mut self,
        r: &mut s2_sim::snap::Reader,
    ) -> Result<(), s2_sim::snap::SnapError> {
        self.counter = r.get()?;
        self.players = r.get()?;
        self.military = r.get()?;
        Ok(())
    }

    /// The player's AI object, for tests that set up its state.
    #[cfg(test)]
    pub(crate) fn player_mut(&mut self, player: PlayerId) -> &mut AiPlayer {
        self.players[usize::from(player.0)]
            .as_mut()
            .expect("an active AI")
    }

    /// Announce the buildings of `snapshot` to the player (tests).
    #[cfg(test)]
    pub(crate) fn update_with(&mut self, world: &World, player: PlayerId, snapshot: &Snapshot) {
        let ai = self.players[usize::from(player.0)]
            .as_mut()
            .expect("an active AI");
        ai.update(world, &self.config, snapshot);
    }

    /// Like [`AiSystem::with_ctx`] with a snapshot the test made.
    #[cfg(test)]
    pub(crate) fn with_snapshot<R>(
        &mut self,
        world: &World,
        player: PlayerId,
        snapshot: &Snapshot,
        f: impl FnOnce(&mut AiPlayer, &mut player::Ctx) -> R,
    ) -> (R, Vec<Command>) {
        let mut alive = [false; SLOTS];
        for (slot, flag) in alive.iter_mut().enumerate() {
            *flag = snapshot.alive(PlayerId(slot as u8));
        }
        let mut out = Vec::new();
        // Tests of single parts draw from a generator of their own and keep
        // a store of route costs of their own.
        let mut rng = s2_sim::Rng::new(1);
        let mut routes = s2_sim::RouteStore::default();
        let ai = self.players[usize::from(player.0)]
            .as_mut()
            .expect("an active AI");
        let mut ctx = player::Ctx {
            world,
            config: &self.config,
            snapshot,
            military: &self.military,
            alive: &alive,
            rng: &mut rng,
            routes: &mut routes,
            out: &mut out,
            counter: self.counter,
            trace: None,
        };
        let result = f(ai, &mut ctx);
        (result, out)
    }

    /// Run `f` on a player with a think's view of `world` and what a think
    /// lends its parts (AI-2.1a), for tests of parts in a row. Gives back
    /// the commands.
    #[cfg(test)]
    pub(crate) fn with_view<R>(
        &mut self,
        world: &World,
        player: PlayerId,
        f: impl FnOnce(&mut AiPlayer, &mut view::ThinkView, &mut player::Lent) -> R,
    ) -> (R, Vec<Command>) {
        let mut view = view::ThinkView::new(world);
        let mut alive = [false; SLOTS];
        for (slot, flag) in alive.iter_mut().enumerate() {
            *flag = view.snapshot().alive(PlayerId(slot as u8));
        }
        let mut out = Vec::new();
        let mut rng = s2_sim::Rng::new(1);
        let mut routes = s2_sim::RouteStore::default();
        let ai = self.players[usize::from(player.0)]
            .as_mut()
            .expect("an active AI");
        let mut lent = player::Lent {
            config: &self.config,
            military: &self.military,
            alive: &alive,
            rng: &mut rng,
            routes: &mut routes,
            out: &mut out,
            counter: self.counter,
            trace: None,
        };
        let result = f(ai, &mut view, &mut lent);
        (result, out)
    }

    /// Run `f` on a player with the context of the current tick (counter as
    /// it is now), for tests of single parts. Gives back the commands.
    #[cfg(test)]
    pub(crate) fn with_ctx<R>(
        &mut self,
        world: &World,
        player: PlayerId,
        f: impl FnOnce(&mut AiPlayer, &mut player::Ctx) -> R,
    ) -> (R, Vec<Command>) {
        let snapshot = Snapshot::take(world);
        let mut alive = [false; SLOTS];
        for (slot, flag) in alive.iter_mut().enumerate() {
            *flag = snapshot.alive(PlayerId(slot as u8));
        }
        let mut out = Vec::new();
        // Tests of single parts draw from a generator of their own and keep
        // a store of route costs of their own.
        let mut rng = s2_sim::Rng::new(1);
        let mut routes = s2_sim::RouteStore::default();
        let ai = self.players[usize::from(player.0)]
            .as_mut()
            .expect("an active AI");
        let mut ctx = player::Ctx {
            world,
            config: &self.config,
            snapshot: &snapshot,
            military: &self.military,
            alive: &alive,
            rng: &mut rng,
            routes: &mut routes,
            out: &mut out,
            counter: self.counter,
            trace: None,
        };
        let result = f(ai, &mut ctx);
        (result, out)
    }

    /// AI-2.1, 2.1a: one tick of the AI system, the last part of the logic
    /// tick: it reads the world as the update left it, draws from the
    /// simulation's generator `rng` (AI-2.7), asks the players' stored
    /// route costs `routes` (TRN-CARRY-5.3a), and returns the commands that
    /// take effect inside the same tick. Run it through
    /// [`World::step_then`], or [`AiSystem::step`] does that.
    pub fn tick(
        &mut self,
        world: &World,
        rng: &mut s2_sim::Rng,
        numbers: &mut Numbers,
        routes: &mut s2_sim::RouteStore,
    ) -> Vec<Command> {
        self.counter += 1;
        // AI-2.1a: the players read the world through a view that takes
        // each command as it is issued.
        let mut view = view::ThinkView::new(world);
        let mut alive = [false; SLOTS];
        for (slot, flag) in alive.iter_mut().enumerate() {
            *flag = view.snapshot().alive(PlayerId(slot as u8));
        }
        let mut out = Vec::new();
        for slot in 0..SLOTS {
            let id = PlayerId(slot as u8);
            if alive[slot] {
                self.military[slot].refresh_next(
                    view.world(),
                    view.snapshot().of(id),
                    &self.config,
                );
            }
            let Some(ai) = self.players[slot].as_mut().filter(|p| p.active) else {
                continue;
            };
            let made = ai.records_made();
            let numbered = ai.update(view.world(), &self.config, view.snapshot());
            // AI-MISC-2.3: no cells left, reset and switch off.
            if ai.cells.is_empty() {
                ai.reinitialise();
                ai.number_records(made, numbered, numbers);
                continue;
            }
            let issued = out.len();
            let mut lent = player::Lent {
                config: &self.config,
                military: &self.military,
                alive: &alive,
                rng: &mut *rng,
                routes: &mut *routes,
                out: &mut out,
                counter: self.counter,
                trace: self.trace.as_mut(),
            };
            ai.think(&mut view, &mut lent);
            // SIM-START-4.4 b (190c1e1, T13): a site the player placed
            // numbers the records its announce will make right after its
            // own number, as the player stands now.
            for command in &mut out[issued..] {
                if let Command::PlanBuilding {
                    building,
                    at,
                    records,
                    ..
                } = command
                {
                    *records = ai.site_records(world, &self.config, *building, *at);
                    ai.prenumbered.insert(*at, *records);
                }
            }
            // SIM-START-4.4 c: each road the computer player plans takes its
            // stub's number when its `PlanRoad` is applied.
            ai.number_records(made, numbered, numbers);
        }
        out
    }

    /// One logic tick of `world` with the computer players as its last part
    /// (AI-2.1a); the results of their commands are among
    /// [`World::last_results`].
    pub fn step(&mut self, world: &mut World) {
        world.step_then(|w, rng, numbers, routes| self.tick(w, rng, numbers, routes));
    }
}
