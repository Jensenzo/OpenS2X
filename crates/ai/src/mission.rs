//! A campaign mission or tutorial played headless, started the way the
//! menu starts it (CAM-4.3, UI-SETUP-2.4), with its script running
//! (`specs/CAM-campaign-missions.md`), and a player who gives the commands
//! a person would to carry the land to a field (a mission's victory
//! region, CAM-2.1). The tests of `crates/ai/tests/campaign.rs` and the
//! viewer's end screens play these games; nothing here is part of the
//! rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use s2_assets::{s2m, GameDir};
use s2_rules::mission::{self, MissionScript, ScriptEvent};
use s2_rules::RuleSet;
use s2_sim::grid::{self, Point};
use s2_sim::{BuildingSize, Command, PlayerId, World};

use crate::save::{self, Header, SavedMission};
use crate::{AiConfig, AiSystem};

/// A mission as the program plays it, without the program.
pub struct MissionGame {
    pub world: World,
    pub ai: Option<AiSystem>,
    pub script: Option<MissionScript>,
    /// The computer players, `(slot, profile)`.
    pub slots: Vec<(PlayerId, u8)>,
    pub map_file: PathBuf,
    /// What a save keeps of the mission besides the world.
    pub saved: SavedMission,
    /// What the script asked of the interface in `onLoaded` and
    /// `onStarted` (the debriefing pages are in `saved`).
    pub start_events: Vec<ScriptEvent>,
    /// Script errors and calls it could not carry out, with their tick: a
    /// shipped mission has none.
    pub problems: Vec<String>,
}

impl MissionGame {
    /// CAM-4.3, UI-SETUP-2.4: the mission `name` (`campaign01`,
    /// `tutorial03`) from the install, as the menu starts it.
    pub fn start(dir: &GameDir, rules: &RuleSet, name: &str) -> Result<Self, String> {
        let file = s2_assets::find_map(dir, name).ok_or(format!("no map {name}"))?;
        if mission::map_kind(&file) == 0 {
            return Err(format!("{name} is not a mission"));
        }
        let shown = file.display().to_string();
        let data = std::fs::read(&file).map_err(|e| format!("{shown}: {e}"))?;
        let map = s2m::load(&shown, &data).map_err(|e| format!("{shown}: {e}"))?;
        Self::start_on(&file, &map, rules)
    }

    /// The same with the map read: the order of `load_map` in the viewer:
    /// the world, the computer players, then the mission and its script's
    /// first two callbacks (CAM-1.2).
    pub fn start_on(file: &Path, map: &s2m::Map, rules: &RuleSet) -> Result<Self, String> {
        Self::start_on_with(file, map, rules, None)
    }

    /// A mission with the human's economy and fighting run by a computer
    /// player of `profile`, the way a test's player gets them: it gives only
    /// a player's commands (`docs/test-coverage.md`, "Missions"). No menu
    /// starts this.
    pub fn start_with_computer_help(
        dir: &GameDir,
        rules: &RuleSet,
        name: &str,
        profile: u8,
    ) -> Result<Self, String> {
        let file = s2_assets::find_map(dir, name).ok_or(format!("no map {name}"))?;
        let shown = file.display().to_string();
        let data = std::fs::read(&file).map_err(|e| format!("{shown}: {e}"))?;
        let map = s2m::load(&shown, &data).map_err(|e| format!("{shown}: {e}"))?;
        Self::start_on_with(&file, &map, rules, Some(profile))
    }

    fn start_on_with(
        file: &Path,
        map: &s2m::Map,
        rules: &RuleSet,
        human_profile: Option<u8>,
    ) -> Result<Self, String> {
        let kind = mission::map_kind(file);
        let mut players = mission::players(map, kind);
        if let Some(first) = players.first_mut() {
            first.profile = human_profile;
        }
        let sim = rules.sim_rules()?;
        let setup =
            s2_rules::setup_of_players(map, rules, mission::start_resources(kind), &players);
        let mut world = World::new_game(
            u64::from(s2_sim::rng::SIM_SEED),
            map.terrain(),
            sim.clone(),
            &setup,
        );
        let slots: Vec<(PlayerId, u8)> = players
            .iter()
            .filter_map(|p| Some((p.player, p.profile?)))
            .collect();
        let ai = (!slots.is_empty()).then(|| {
            let mut ai = AiSystem::new(AiConfig::new(rules, &sim), &world);
            ai.activate_all(&mut world, &slots);
            ai
        });
        let mut goal = mission::mission_of(map);
        // CAM-3.1a: the campaign menu's target 4, the tutorial menu's 0.
        if let Some(target) = mission::start_target(kind, 0) {
            goal.target = target;
        }
        let target = goal.target;
        world.set_mission(goal);
        // CAM-3.1b: a campaign game's victory regions start explored.
        if kind == 1 {
            world.explore_victory_regions();
        }
        let script = mission::script_for(file, rules).map_err(|e| e.to_string())?;
        let mut game = Self {
            world,
            ai,
            script: None,
            slots,
            map_file: file.to_path_buf(),
            saved: SavedMission {
                target,
                ..SavedMission::default()
            },
            start_events: Vec::new(),
            problems: Vec::new(),
        };
        if let Some(s) = &script {
            let mut events = s.on_loaded().map_err(|e| e.to_string())?;
            events.extend(s.on_started().map_err(|e| e.to_string())?);
            game.start_events = game.take_events(events);
        }
        game.script = script;
        Ok(game)
    }

    /// Submit the script's commands; keep what a save keeps; the rest for
    /// the interface.
    fn take_events(&mut self, events: Vec<ScriptEvent>) -> Vec<ScriptEvent> {
        let rest = mission::submit(&mut self.world, events);
        for event in &rest {
            match event {
                ScriptEvent::AddDebriefing(key) => self.saved.debriefing.push(key.clone()),
                ScriptEvent::AddExtro(key) => self.saved.extro.push(key.clone()),
                ScriptEvent::Letter(key) => self.saved.letters.push(key.clone()),
                ScriptEvent::Warning(w) => {
                    self.problems
                        .push(format!("tick {}: {w}", self.world.tick()));
                }
                _ => {}
            }
        }
        rest
    }

    /// One logic tick: the world and the computer players (AI-2.1a), then
    /// the script's calls (CAM-1.2, 2.3). Gives what the script asked of the
    /// interface.
    pub fn step(&mut self) -> Vec<ScriptEvent> {
        match self.ai.as_mut() {
            Some(ai) => ai.step(&mut self.world),
            None => self.world.step(),
        }
        // A command of the script's that was refused is a fault of the
        // mission's start, as a script error is. The player's own are his.
        for result in self.world.last_results() {
            if let (Err(reason), true) = (result.outcome, is_script_command(&result.command)) {
                self.problems.push(format!(
                    "tick {}: {:?} refused: {reason:?}",
                    self.world.tick(),
                    result.command
                ));
            }
        }
        let Some(script) = self.script.as_ref() else {
            return Vec::new();
        };
        let events = match script.after_step(&self.world) {
            Ok(events) => events,
            Err(e) => {
                self.problems
                    .push(format!("tick {}: {e}", self.world.tick()));
                Vec::new()
            }
        };
        self.saved.script_variables = script.variables();
        self.take_events(events)
    }

    /// The bytes of a save of this game, as the program writes them
    /// (`s2_ai::save`, FMT-SAV-3.12).
    pub fn save(&self) -> Vec<u8> {
        let header = Header {
            map: self.map_file.display().to_string(),
            tick: self.world.tick(),
            world_hash: self.world.state_hash(),
            ai_hash: self.ai.as_ref().map(AiSystem::state_hash),
            slots: self.slots.clone(),
            mission: self.saved.clone(),
            // s2-reborn's headless missions: the empty mark.
            mark: save::SaveMark::default(),
        };
        save::write(&header, &self.world, self.ai.as_ref())
    }

    /// A save read back, as the program loads one: the world and computer
    /// players from it, the map's script loaded again without its start
    /// callbacks, and its variables set (CAM-1.1, FMT-SAV-3.12).
    pub fn load(bytes: &[u8], rules: &RuleSet) -> Result<Self, String> {
        let sim = rules.sim_rules()?;
        let loaded = save::read(bytes, &sim, |world| {
            Some(AiSystem::new(AiConfig::new(rules, &sim), world))
        })
        .map_err(|e| e.to_string())?;
        let map_file = PathBuf::from(&loaded.header.map);
        let script = mission::script_for(&map_file, rules).map_err(|e| e.to_string())?;
        if let Some(s) = &script {
            s.set_variables(&loaded.header.mission.script_variables)
                .map_err(|e| e.to_string())?;
        }
        if loaded.current_format() {
            let same = loaded.world.state_hash() == loaded.header.world_hash
                && loaded.ai.as_ref().map(AiSystem::state_hash) == loaded.header.ai_hash;
            if !same {
                return Err("the game loaded is not the game saved".into());
            }
        }
        Ok(Self {
            world: loaded.world,
            ai: loaded.ai,
            script,
            slots: loaded.header.slots,
            map_file,
            saved: loaded.header.mission,
            start_events: Vec::new(),
            problems: Vec::new(),
        })
    }
}

/// The commands only a mission script gives (CAM-1.3, 1.4).
fn is_script_command(command: &Command) -> bool {
    matches!(
        command,
        Command::AddGoods { .. }
            | Command::ClearGoods { .. }
            | Command::LockBuilding { .. }
            | Command::CreateBuilding { .. }
            | Command::CreateShip { .. }
            | Command::CreateStreet { .. }
            | Command::RemoveTrigger { .. }
    )
}

/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;
/// A new military building keeps this far from the others.
const MIN_SPACING: u32 = 4;
/// Flags tried, nearest first, for the road of a new building.
const ROAD_TRIES: usize = 8;
/// Fields of road a carrier may have to itself before it gets a flag.
const LONGEST_ROAD: usize = 4;

/// More steps than any way over land takes.
const NO_WAY_OVER_LAND: u32 = 100_000;

/// What the player has done, for the report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerLog {
    /// Military buildings placed: the tick and the field.
    pub placed: Vec<(u64, Point)>,
    /// Roads ordered, and flags set on them.
    pub roads: u32,
    pub flags: u32,
    /// Sites given up for want of a road.
    pub given_up: u32,
    /// Orders the game refused, by reason.
    pub refused: Vec<String>,
}

/// A person carrying the land towards `goal`: one military building at a
/// time, on the buildable field of the land nearest the goal, joined to the
/// headquarters by road, with flags on the road so that it has carriers
/// enough. Only the commands a player can give; everything else it reads
/// from the world, so it can go on from a loaded save.
#[derive(Debug, Clone)]
pub struct TerritoryPlayer {
    pub player: PlayerId,
    pub goal: Point,
    /// The military building it builds and its plot size.
    pub building: u32,
    pub size: BuildingSize,
    pub log: PlayerLog,
    /// The player's headquarters type (CAM-3.2).
    headquarters: u32,
    /// Sites without a road: not tried again.
    tried: BTreeSet<Point>,
    /// A site's flag: the roads refused for it so far.
    road_tries: BTreeMap<Point, usize>,
    /// Commands were given last tick: read their outcome first.
    waiting: bool,
}

impl TerritoryPlayer {
    pub fn new(
        world: &World,
        rules: &RuleSet,
        player: PlayerId,
        goal: Point,
        building: u32,
    ) -> Option<Self> {
        Some(Self {
            player,
            goal,
            building,
            size: world.building_type(building)?.size,
            log: PlayerLog::default(),
            headquarters: rules.id_named(s2_rules::HEADQUARTERS)?,
            tried: BTreeSet::new(),
            road_tries: BTreeMap::new(),
            waiting: false,
        })
    }

    fn hq_flag(&self, world: &World) -> Option<Point> {
        let hq = world
            .buildings()
            .find(|b| b.owner == self.player && b.building_type == self.headquarters)?
            .at;
        world.terrain().neighbour(hq, FLAG_DIRECTION)
    }

    /// The player's commands before the next tick, given to `world`.
    pub fn act(&mut self, world: &mut World) {
        if self.waiting {
            self.waiting = false;
            for result in world.last_results() {
                let Err(reason) = result.outcome else {
                    continue;
                };
                match result.command {
                    Command::BuildRoad { player, from, .. } if player == self.player => {
                        *self.road_tries.entry(from).or_default() += 1;
                        self.log
                            .refused
                            .push(format!("{:?}: {reason:?}", result.command));
                    }
                    Command::PlaceBuilding { player, .. }
                    | Command::PlaceFlag { player, .. }
                    | Command::RemoveBuilding { player, .. }
                        if player == self.player =>
                    {
                        self.log
                            .refused
                            .push(format!("{:?}: {reason:?}", result.command));
                    }
                    _ => {}
                }
            }
        }
        let Some(hq_flag) = self.hq_flag(world) else {
            return;
        };
        // The goal is reached; the mission sees it at its next evaluation.
        if world.field_owner(self.goal) == Some(self.player) {
            return;
        }
        let command = self
            .join_a_site(world, hq_flag)
            .or_else(|| self.flag_a_long_road(world))
            .or_else(|| self.next_building(world));
        if let Some(command) = command {
            match &command {
                Command::PlaceBuilding { at, .. } => self.log.placed.push((world.tick(), *at)),
                Command::BuildRoad { .. } => self.log.roads += 1,
                Command::PlaceFlag { .. } => self.log.flags += 1,
                _ => {}
            }
            world.submit(command);
            self.waiting = true;
        }
    }

    /// A site not yet on the road network gets its road, from the nearest
    /// flag on the network; one with none is given up.
    fn join_a_site(&mut self, world: &World, hq_flag: Point) -> Option<Command> {
        let player = self.player;
        let network = flags_joined_to(world, hq_flag);
        let cut_off = world
            .buildings()
            .filter(|b| b.owner == player)
            .map(|b| b.at)
            .find(|&at| {
                world
                    .terrain()
                    .neighbour(at, FLAG_DIRECTION)
                    .is_some_and(|f| !network.contains(&f))
            })?;
        let flag = world.terrain().neighbour(cut_off, FLAG_DIRECTION)?;
        let mut flags: Vec<Point> = world
            .flags()
            .filter(|&(f, owner)| owner == player && network.contains(&f))
            .map(|(f, _)| f)
            .collect();
        flags.sort_by_key(|&f| (grid::distance(flag, f), f));
        let tries = flags.len().min(ROAD_TRIES);
        let done = self.road_tries.get(&flag).copied().unwrap_or(0);
        if done >= tries {
            self.log.given_up += 1;
            self.tried.insert(cut_off);
            return Some(Command::RemoveBuilding {
                player,
                at: cut_off,
            });
        }
        Some(Command::BuildRoad {
            player,
            from: flag,
            to: flags[done],
        })
    }

    /// A road with a long stretch gets a flag in the middle of it.
    fn flag_a_long_road(&self, world: &World) -> Option<Command> {
        let at = world
            .roads()
            .filter(|r| r.owner == self.player && r.path.len() > LONGEST_ROAD)
            .find_map(|r| {
                let middle = r.path.len() / 2;
                [middle, middle + 1, middle - 1]
                    .into_iter()
                    .map(|i| r.path[i])
                    .find(|&f| world.check_flag(self.player, f).is_ok())
            })?;
        Some(Command::PlaceFlag {
            player: self.player,
            at,
        })
    }

    /// When every military building is built and manned, the next one on
    /// the buildable field of the land nearest the goal.
    fn next_building(&self, world: &World) -> Option<Command> {
        let player = self.player;
        let military = |kind: u32| {
            world
                .building_type(kind)
                .is_some_and(|t| t.soldier_limit > 0)
        };
        let own: Vec<_> = world
            .buildings()
            .filter(|b| b.owner == player && military(b.building_type))
            .collect();
        let waiting = own
            .iter()
            .any(|b| b.under_construction || world.military_at(b.at).is_some_and(|m| !m.applied));
        if waiting {
            return None;
        }
        let steps = steps_to(world, self.goal);
        let terrain = world.terrain();
        let at = terrain
            .fields()
            .filter(|&f| world.field_owner(f) == Some(player))
            .filter(|f| !self.tried.contains(f))
            .filter(|&f| own.iter().all(|b| grid::distance(b.at, f) >= MIN_SPACING))
            .filter(|&f| world.check_building(player, f, self.size).is_ok())
            // ECO-PLACE-6.3: the build menu greys a military cell within 4
            // fields of any military building or site; a player picks
            // another field.
            .filter(|&f| !(world.counts_as_military(self.building) && world.military_near(f)))
            // With no way over land (a coast between), the straight line.
            .map(|f| {
                let straight = NO_WAY_OVER_LAND + grid::distance(f, self.goal);
                (steps[terrain.index(f)].unwrap_or(straight), f)
            })
            .min()?
            .1;
        Some(Command::PlaceBuilding {
            player,
            building: self.building,
            at,
        })
    }
}

/// Per field, the steps over land a road could take to `goal`: what a person
/// sees looking at the map, rather than the straight line across a bay.
fn steps_to(world: &World, goal: Point) -> Vec<Option<u32>> {
    use s2_sim::map::state;
    const NOT_ROAD: u32 = state::WATER
        | state::BLOCKED
        | state::MOVEMENT_BLOCKED
        | state::DEPOSIT_BLOCKED
        | state::DOODAD_BLOCKED
        | state::ELEVATION_BLOCKED;
    let terrain = world.terrain();
    let mut steps = vec![None; terrain.len()];
    steps[terrain.index(goal)] = Some(0);
    let mut open = std::collections::VecDeque::from([goal]);
    while let Some(field) = open.pop_front() {
        let next_steps = steps[terrain.index(field)].map(|s: u32| s + 1);
        for next in terrain.neighbours(field).into_iter().flatten() {
            let i = terrain.index(next);
            if steps[i].is_none() && !terrain.has(next, NOT_ROAD) {
                steps[i] = next_steps;
                open.push_back(next);
            }
        }
    }
    steps
}

/// The flags joined to `start` by road: the ends of the roads, followed.
pub(crate) fn flags_joined_to(world: &World, start: Point) -> BTreeSet<Point> {
    let mut links: BTreeMap<Point, Vec<Point>> = BTreeMap::new();
    for road in world.roads() {
        if let (Some(&a), Some(&b)) = (road.path.first(), road.path.last()) {
            links.entry(a).or_default().push(b);
            links.entry(b).or_default().push(a);
        }
    }
    let mut joined = BTreeSet::from([start]);
    let mut open = vec![start];
    while let Some(flag) = open.pop() {
        for &next in links.get(&flag).into_iter().flatten() {
            if joined.insert(next) {
                open.push(next);
            }
        }
    }
    joined
}
