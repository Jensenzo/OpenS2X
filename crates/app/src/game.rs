//! A headless game with computer players on every slot, the way `hash`,
//! `soak` and `trace` start one, the replay file that reproduces it, and
//! the game snapshot that holds it at any tick (`docs/snapshot.md`).
//!
//! A map is a name from the install's map folders (`MP_2P_Storm_Coast`), a
//! path to an `.s2m` file, or `fixture:<name>`: one of [`s2_ai::fixture`]'s
//! synthetic games, which need no install (`fixture:flat2`,
//! `fixture:flat4`, `fixture:town2`, `fixture:town4`).

use std::collections::HashMap;

use s2_ai::run::Runner;
use s2_ai::{AiConfig, AiSystem};
use s2_assets::{s2m, GameDir};
use s2_sim::snap::{Reader, Writer};
use s2_sim::{PlayerId, World};

/// The first bytes of a game snapshot file, then its version.
pub const SNAPSHOT_MAGIC: &[u8; 8] = b"s2-game\0";
/// 4: the computer player's stock cycle (AI-CON-1.7); 5: a need's consumer
/// is optional (AI-CON-2.5); 6: the computer player's records made, which
/// take object numbers (SIM-START-4.2); 7, 8: the computer player's
/// landmasses (AI-CON-3.5, 6.3); 9: the records its sites numbered already
/// (SIM-START-4.4 b); 10: its waiting list, attempt counters and placed
/// expansion target (AI-CON-3.4, 3.5, AI-MIL-3.2).
pub const SNAPSHOT_VERSION: u64 = 14;

/// Everything that decides a headless game: same start, same game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    pub map: String,
    pub seed: u64,
    /// ECO-HQ-2.2: the start resources setting, 0 to 3.
    pub resources: u8,
    /// The computer players, `(slot, profile)`, possibly none (`--ai
    /// none`); `None` is every player of the map with profile 2 (normal).
    pub slots: Option<Vec<(PlayerId, u8)>>,
    /// The closed player slots: no player is created on them, the others
    /// keep their slots and start positions (SIM-START-1.1, 3.1).
    pub closed: Vec<PlayerId>,
}

impl Start {
    pub fn new(map: &str, seed: u64) -> Self {
        Self {
            map: map.to_string(),
            seed,
            resources: 1,
            slots: None,
            closed: Vec::new(),
        }
    }

    /// Read the options every game command shares: `--seed S`,
    /// `--resources R`, `--ai 0=2,1=3`, `--closed 1,2`. Unknown arguments
    /// are returned.
    pub fn options(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let mut rest = Vec::new();
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            let mut value = || it.next().ok_or(format!("{arg} wants a value"));
            match arg.as_str() {
                "--seed" => self.seed = number(value()?)?,
                "--resources" => self.resources = number(value()?)? as u8,
                "--ai" => self.slots = Some(slots(value()?)?),
                "--closed" => self.closed = closed(value()?)?,
                _ => rest.push(arg.clone()),
            }
        }
        Ok(rest)
    }

    /// The game at tick 0.
    pub fn begin(&self) -> Result<Runner, String> {
        let chosen = self.slots.iter().flatten();
        if let Some(p) = chosen.clone().find(|(p, _)| self.closed.contains(p)) {
            return Err(format!("slot {} is closed and a computer player", p.0 .0));
        }
        let (mut world, config, names) = if let Some(name) = self.map.strip_prefix("fixture:") {
            if !self.closed.is_empty() {
                return Err("closed slots need a map, not a fixture".into());
            }
            let (world, config) = s2_ai::fixture::named(name, self.seed).ok_or(format!(
                "no fixture {name} (there are {})",
                s2_ai::fixture::NAMES.join(", ")
            ))?;
            (world, config, s2_ai::fixture::labels())
        } else {
            let dir = GameDir::from_env().map_err(|e| e.to_string())?;
            let (rules, _) = s2_rules::load(&dir).map_err(|e| e.to_string())?;
            let file =
                s2_assets::find_map(&dir, &self.map).ok_or(format!("no map {}", self.map))?;
            let name = file.display().to_string();
            let data = std::fs::read(&file).map_err(|e| format!("{name}: {e}"))?;
            let map = s2m::load(&name, &data).map_err(|e| format!("{name}: {e}"))?;
            let mut setup = s2_rules::game_setup(&map, &rules, self.resources);
            // SIM-START-1.1: only the open slots' players are created.
            setup.players.retain(|(p, _)| !self.closed.contains(p));
            let sim = rules.sim_rules()?;
            let world = World::new_game(self.seed, map.terrain(), sim.clone(), &setup);
            let config = AiConfig::new(&rules, &sim);
            let names: HashMap<u32, String> = rules
                .names
                .iter()
                .map(|(name, &id)| (id, name.clone()))
                .collect();
            (world, config, names)
        };
        let slots = match &self.slots {
            Some(slots) => slots.clone(),
            None => s2_ai::every_player(&world),
        };
        let mut ai = AiSystem::new(config, &world);
        ai.activate_all(&mut world, &slots);
        Ok(Runner::new(world, ai, names))
    }

    /// The replay file: the start and the tick to play to. Loading it and
    /// playing to that tick gives the same state as the game it was written
    /// from, since a game of computer players is decided by its start.
    pub fn replay_text(&self, tick: u64) -> String {
        // Written only when a slot is closed, so older replay files read
        // the same.
        let closed = if self.closed.is_empty() {
            String::new()
        } else {
            format!("closed {}\n", self.closed_text())
        };
        format!(
            "s2-replay 1\nmap {}\nseed {}\nresources {}\nai {}\n{closed}tick {tick}\n",
            self.map,
            self.seed,
            self.resources,
            self.ai_text()
        )
    }

    /// The computer players as `--ai` takes them: `0=2,1=3`, `none`, or
    /// empty for every player.
    pub fn ai_text(&self) -> String {
        match &self.slots {
            None => String::new(),
            Some(slots) if slots.is_empty() => "none".into(),
            Some(slots) => {
                let list: Vec<String> = slots
                    .iter()
                    .map(|(p, profile)| format!("{}={profile}", p.0))
                    .collect();
                list.join(",")
            }
        }
    }

    /// The closed slots as `--closed` takes them, empty for none.
    pub fn closed_text(&self) -> String {
        let list: Vec<String> = self.closed.iter().map(|p| p.0.to_string()).collect();
        list.join(",")
    }

    /// Read a replay file: the start and the tick it was written at.
    pub fn parse_replay(text: &str) -> Result<(Start, u64), String> {
        let mut lines = text.lines();
        if lines.next() != Some("s2-replay 1") {
            return Err("not a replay file (s2-replay 1)".into());
        }
        let mut start = Start::new("", 0);
        let mut tick = None;
        for line in lines {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "map" => start.map = value.to_string(),
                "seed" => start.seed = number(value)?,
                "resources" => start.resources = number(value)? as u8,
                "ai" if value.is_empty() => start.slots = None,
                "ai" => start.slots = Some(slots(value)?),
                "closed" => start.closed = closed(value)?,
                "tick" => tick = Some(number(value)?),
                "" => {}
                other => return Err(format!("unknown replay line {other}")),
            }
        }
        if start.map.is_empty() {
            return Err("replay file without map".into());
        }
        Ok((start, tick.ok_or("replay file without tick")?))
    }
}

/// A game snapshot (`docs/snapshot.md`): the start, then the game as it
/// is now. Loading it continues the game as if it had never been saved.
pub fn snapshot(start: &Start, runner: &Runner) -> Vec<u8> {
    let mut w = Writer::new();
    w.raw(SNAPSHOT_MAGIC);
    w.u64(SNAPSHOT_VERSION);
    w.put(&start.map);
    w.put(&start.seed);
    w.put(&start.resources);
    w.put(&start.slots);
    w.put(&start.closed);
    runner.put_state(&mut w);
    w.bytes
}

/// Whether the bytes are a game snapshot (and not a replay file).
pub fn is_snapshot(bytes: &[u8]) -> bool {
    bytes.starts_with(SNAPSHOT_MAGIC)
}

/// The start a game snapshot names, and the game it holds: the map and
/// rules are loaded as for a new game (the AI's configuration and the
/// report's names come from them), then the state is replaced by the
/// snapshot's.
pub fn load_snapshot(bytes: &[u8]) -> Result<(Start, Runner), String> {
    let start = read_start(&mut Reader::new(bytes))?;
    let mut runner = start.begin()?;
    restore(&mut runner, bytes)?;
    Ok((start, runner))
}

/// Load a game snapshot into a runner already set up for its start (the
/// same map, seed, set-up), without loading the rules again.
pub fn restore(runner: &mut Runner, bytes: &[u8]) -> Result<Start, String> {
    let mut r = Reader::new(bytes);
    let start = read_start(&mut r)?;
    runner.get_state(&mut r).map_err(|e| e.to_string())?;
    if !r.at_end() {
        return Err("bytes after the game in the snapshot".into());
    }
    Ok(start)
}

fn read_start(r: &mut Reader) -> Result<Start, String> {
    let error = |e: s2_sim::snap::SnapError| e.to_string();
    if r.raw(SNAPSHOT_MAGIC.len()).map_err(error)? != SNAPSHOT_MAGIC {
        return Err("not a game snapshot (s2-game)".into());
    }
    let version = r.u64().map_err(error)?;
    if version != SNAPSHOT_VERSION {
        return Err(format!(
            "game snapshot version {version}, this build reads {SNAPSHOT_VERSION}"
        ));
    }
    Ok(Start {
        map: r.get().map_err(error)?,
        seed: r.get().map_err(error)?,
        resources: r.get().map_err(error)?,
        slots: r.get().map_err(error)?,
        closed: r.get().map_err(error)?,
    })
}

/// A save of the game program (`s2_ai::save`, `docs/snapshot.md`) as a game
/// to run headless: a fixture of `tests/saves`, or a player's save of a map of
/// the install (`S2_GAME_DIR`, for the rules of its computer players).
pub fn load_save(file: &str) -> Result<(Start, Runner), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let mut r = Reader::new(&bytes);
    let header = s2_ai::save::read_header(&mut r).map_err(|e| format!("{file}: {e}"))?;
    // A fixture is loaded as the program that wrote it; a player's save as
    // s2-reborn, which refuses a project's (W11 b).
    let mark = if header.map.starts_with("fixture:") {
        header.mark.clone()
    } else {
        s2_ai::save::SaveMark::default()
    };
    let (config, names) = match header.map.strip_prefix("fixture:") {
        Some(name) => {
            let (_, config) =
                s2_ai::fixture::named(name, 1).ok_or(format!("{file}: no fixture {name}"))?;
            (config, s2_ai::fixture::labels())
        }
        None => {
            let dir = GameDir::from_env().map_err(|e| e.to_string())?;
            let (rules, _) = s2_rules::load(&dir).map_err(|e| e.to_string())?;
            let sim = rules.sim_rules()?;
            let names: HashMap<u32, String> = rules
                .names
                .iter()
                .map(|(name, &id)| (id, name.clone()))
                .collect();
            (AiConfig::new(&rules, &sim), names)
        }
    };
    let loaded = s2_ai::save::read_as(&bytes, &mark, &config.sim, |world| {
        Some(AiSystem::new(config.clone(), world))
    })
    .map_err(|e| format!("{file}: {e}"))?;
    if loaded.current_format() {
        let same = loaded.world.state_hash() == loaded.header.world_hash
            && loaded.ai.as_ref().map(AiSystem::state_hash) == loaded.header.ai_hash;
        if !same {
            return Err(format!("{file}: the game loaded is not the game saved"));
        }
    }
    let ai = match loaded.ai {
        Some(ai) => ai,
        None => AiSystem::new(config, &loaded.world),
    };
    // The map's name without folders and extension, whichever separators
    // the path was written with.
    let map = loaded.header.map.rsplit(['/', '\\']).next().unwrap_or("");
    let map = map
        .rsplit_once('.')
        .map_or(map, |(stem, _)| stem)
        .to_string();
    let start = Start {
        map: map.clone(),
        slots: Some(loaded.header.slots.clone()),
        ..Start::new(&map, 1)
    };
    Ok((start, Runner::new(loaded.world, ai, names)))
}

/// Read a game file of either kind: a snapshot (the game at its tick) or a
/// replay file (played from the start to its tick).
pub fn load_game(file: &str) -> Result<(Start, Runner), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    if is_snapshot(&bytes) {
        return load_snapshot(&bytes).map_err(|e| format!("{file}: {e}"));
    }
    let text = String::from_utf8(bytes).map_err(|_| format!("{file}: not a game file"))?;
    let (start, tick) = Start::parse_replay(&text)?;
    let mut runner = start.begin()?;
    while runner.world.tick() < tick {
        runner.step();
    }
    Ok((start, runner))
}

fn number(text: &str) -> Result<u64, String> {
    text.parse().map_err(|_| format!("not a number: {text}"))
}

/// `0=2,1=3`: computer players by slot and profile (1 weak, 2 normal, 3
/// strong); `none`: no computer player.
fn slots(text: &str) -> Result<Vec<(PlayerId, u8)>, String> {
    if text == "none" {
        return Ok(Vec::new());
    }
    text.split(',')
        .map(|pair| {
            let (s, p) = pair
                .split_once('=')
                .ok_or(format!("{pair}: want slot=profile"))?;
            let (s, p) = (number(s)?, number(p)?);
            if !(1..=3).contains(&p) || s > 7 {
                return Err(format!("{pair}: slot 0..7, profile 1..3"));
            }
            Ok((PlayerId(s as u8), p as u8))
        })
        .collect()
}

/// `1,2`: the closed player slots.
fn closed(text: &str) -> Result<Vec<PlayerId>, String> {
    text.split(',')
        .map(|s| match number(s)? {
            s @ 0..=7 => Ok(PlayerId(s as u8)),
            _ => Err(format!("{s}: slot 0..7")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the replay file of the safety net)
    #[test]
    fn replay_text_round_trips() {
        let mut start = Start::new("fixture:flat2", 9);
        start.resources = 3;
        start.slots = Some(vec![(PlayerId(0), 1), (PlayerId(1), 3)]);
        let text = start.replay_text(1234);
        assert_eq!(Start::parse_replay(&text), Ok((start.clone(), 1234)));
        start.slots = Some(Vec::new());
        let text = start.replay_text(5);
        assert!(text.contains("ai none\n"), "{text}");
        assert_eq!(Start::parse_replay(&text), Ok((start.clone(), 5)));
        start.slots = None;
        let text = start.replay_text(5);
        assert_eq!(Start::parse_replay(&text), Ok((start.clone(), 5)));
        start.closed = vec![PlayerId(1), PlayerId(3)];
        let text = start.replay_text(5);
        assert!(text.contains("closed 1,3\n"), "{text}");
        assert_eq!(Start::parse_replay(&text), Ok((start, 5)));
    }

    // A closed slot has no player; the open ones keep their slots and start
    // fields, and a closed slot cannot be a computer player.
    // spec: SIM-START-1.1, SIM-START-3.1 (TRC-14)
    #[test]
    fn a_closed_slot_has_no_player() {
        if std::env::var_os("S2_GAME_DIR").is_none() {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        }
        let mut start = Start::new("MP_2P_Storm_Coast", u64::from(s2_sim::rng::SIM_SEED));
        start.closed = vec![PlayerId(1)];
        let runner = start.begin().expect("the game");
        assert!(runner.world.is_player(PlayerId(0)));
        assert!(!runner.world.is_player(PlayerId(1)));
        let castles: Vec<_> = runner.world.buildings().map(|b| (b.owner, b.at)).collect();
        assert_eq!(castles, [(PlayerId(0), (29, 51))]);
        start.slots = Some(vec![(PlayerId(1), 2)]);
        assert!(start.begin().is_err());
    }

    // spec: none (the replay file of the safety net)
    #[test]
    fn bad_replay_files_are_refused() {
        for text in [
            "",
            "s2-replay 2\n",
            "s2-replay 1\nmap x\n",
            "s2-replay 1\ntick 3\n",
        ] {
            assert!(Start::parse_replay(text).is_err(), "{text:?}");
        }
        assert!(Start::parse_replay("s2-replay 1\nmap x\nai 9=2\ntick 1\n").is_err());
    }
}
