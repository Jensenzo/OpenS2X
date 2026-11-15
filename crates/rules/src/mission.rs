//! Mission scripts (`specs/CAM-campaign-missions.md` CAM-1): the
//! `<map>.lua` of a campaign or tutorial map, run in its own Lua state with
//! the `menu` and `triggers` tables, called back by name (`onLoaded`,
//! `onStarted`, `onLogicTick`, `onTriggerIsActive`).
//!
//! The script never changes the world itself: what it asks for comes back
//! as [`ScriptEvent`]s, game changes as [`Command`]s to submit and the rest
//! for the interface (book dialog, letters, colours).

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, LuaOptions, StdLib, Value};
use s2_assets::s2m::Map;
use s2_sim::mission::{Mission, Trigger, TriggerKind};
use s2_sim::{Command, PlayerId, World};

use crate::model::RuleSet;
use crate::script::ScriptError;
use crate::SetupPlayer;

/// What a script call asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptEvent {
    /// A change of the game, to submit for the next tick.
    Command(Command),
    /// `menu.openBookDialog(key)`: the book dialog with page 1 = `key`.
    OpenBook(String),
    /// `menu.addPage(key)`
    AddPage(String),
    /// `menu.addPicture(name)`: the current page's picture,
    /// `data\gfx\<name>.dds`.
    AddPicture(String),
    /// `menu.addDebriefing(key)`: a page of the debriefing.
    AddDebriefing(String),
    /// `menu.addExtro(key)`
    AddExtro(String),
    /// `menu.createLetter(key)`: a "You have Mail!" message.
    Letter(String),
    /// `menu.setPlayerColor(player, r, g, b)`, channels 0 to 1.
    PlayerColor { player: PlayerId, rgb: [f32; 3] },
    /// A call the program could not carry out (an unknown name passed as
    /// nil, a value out of range), for the log.
    Warning(String),
}

/// ECO-HQ-2.2, CAM-4.3, UI-SETUP-2.4: the start resources setting of a map of
/// kind `kind` (CAM-1.1): written at each start, so nothing carries over from an
/// earlier game: 1 (x 2) for a campaign mission, 2 (x 4) for a tutorial lesson.
pub fn start_resources(kind: u8) -> u8 {
    match kind {
        2 => 2,
        _ => 1,
    }
}

/// CAM-3.1a: the target a new game of a map of kind `kind` plays with: the
/// start settings' target replaces the header's for the kinds 0, 1, 2, 100
/// and 102. The campaign menu sets 4 (reach the portal) for every mission,
/// the tutorial menu 0, the free game's setup its chosen entry `chosen`
/// (0 to 3, UI-SETUP-1.8). `None` for the other kinds: the header's.
pub fn start_target(kind: u8, chosen: u8) -> Option<u8> {
    match kind {
        1 => Some(s2_sim::mission::target::PORTAL),
        2 => Some(0),
        0 | 100 | 102 => Some(chosen.min(3)),
        _ => None,
    }
}

/// CAM-4.3a, FMT-S2M-2.5, 2.6, UI-SETUP-2.4: who plays a map of kind `kind`.
/// A campaign map sets its players by its own records: kind, tribe, colour, team
/// and strength. A tutorial makes entry 0 the human and the others computer
/// players; it writes only the kinds, so the rest are the set-up's reset
/// values (UI-SETUP-2.3, 2.4): tribe 0, no team, colour its number and
/// strength 0, the Weak profile.
pub fn players(map: &Map, kind: u8) -> Vec<SetupPlayer> {
    let starts = map.info.start_positions.len();
    if kind == 1 {
        return campaign_players(&map.info.player_types, &map.info.player_records, starts);
    }
    (0..starts.min(6))
        .map(|i| SetupPlayer {
            player: PlayerId(i as u8),
            start: i,
            tribe: 0,
            colour: i as u8,
            team: -1,
            profile: (i > 0).then_some(1),
        })
        .collect()
}

/// CAM-4.3a: the players of a campaign map: each index of kind 1 (computer) or
/// 2 (human, FMT-S2M-2.5) that has a start position; strength 0, 1, 2 starts AI
/// profile 1, 2, 3, ignored on the human (FMT-S2M-2.6).
fn campaign_players(kinds: &[i32], records: &[[i32; 4]], starts: usize) -> Vec<SetupPlayer> {
    kinds
        .iter()
        .zip(records)
        .enumerate()
        // FMT-S2M-2.2: the original keeps at most the first 6.
        .take(starts.min(6))
        .filter(|(_, (&kind, _))| kind != 0)
        .map(|(i, (&kind, record))| {
            let [tribe, colour, team, strength] = *record;
            SetupPlayer {
                player: PlayerId(i as u8),
                start: i,
                // UI-SETUP-2.6: an unset tribe is treated as 0.
                tribe: tribe.clamp(0, 2) as u8,
                colour: colour.clamp(0, 7) as u8,
                team: team.clamp(-1, i32::from(i8::MAX)) as i8,
                profile: (kind == 1).then_some(strength.clamp(0, 2) as u8 + 1),
            }
        })
        .collect()
}

/// GFX-OBJ-2.11, FMT-S2M-2.6: the colour of each player index, its own unless
/// the players say otherwise.
pub fn colours(players: &[SetupPlayer]) -> [u8; 8] {
    let mut colours = std::array::from_fn(|i| i as u8);
    for p in players {
        if let Some(c) = colours.get_mut(usize::from(p.player.0)) {
            *c = p.colour;
        }
    }
    colours
}

/// CAM-2.1, 3.1: the mission of a map: its target and its triggers.
/// Which trigger fields hold the player and the timer is Q-187.
pub fn mission_of(map: &Map) -> Mission {
    let triggers = map
        .triggers
        .iter()
        .filter_map(|t| {
            Some(Trigger {
                kind: TriggerKind::from_code(t.kind)?,
                field: (t.position.x, t.position.y),
                // CAM-2.1, FMT-S2M-3.3 (Q-187 a): the player is the u32
                // after the name, the flag the bool, the timer the f32.
                player: PlayerId(u8::try_from(t.player).ok()?),
                name: t.name.clone(),
                active: t.active,
                timer: (t.timer.clamp(0.0, 5.0) * 10.0).round() as u32,
            })
        })
        .collect();
    let target = u8::try_from(map.info.info_values[0]).unwrap_or(0);
    Mission::new(target.min(4), triggers)
}

/// CAM-1.1: the kind of a map by its folder: 1 campaign, 2 tutorial, 0 a
/// free game.
pub fn map_kind(map: &Path) -> u8 {
    let folder = map
        .parent()
        .and_then(Path::file_name)
        .map(|f| f.to_string_lossy().to_ascii_lowercase());
    match folder.as_deref() {
        Some("campaignmaps") => 1,
        Some("tutorialmaps") => 2,
        _ => 0,
    }
}

/// CAM-1.1: the script of a campaign or tutorial map, `<folder>\<map
/// name>.lua` next to it; none for a free game or a map without one.
pub fn script_for(map: &Path, rules: &RuleSet) -> Result<Option<MissionScript>, ScriptError> {
    if map_kind(map) == 0 {
        return Ok(None);
    }
    let (Some(folder), Some(stem)) = (map.parent(), map.file_stem()) else {
        return Ok(None);
    };
    let name = format!("{}.lua", stem.to_string_lossy());
    let Ok(path) = crate::resolve(folder, &name) else {
        return Ok(None);
    };
    let source = crate::InstallSource {
        root: folder.to_path_buf(),
        overlays: Vec::new(),
    };
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(name);
    let code = crate::script::ScriptSource::read(&source, &file).map_err(ScriptError)?;
    MissionScript::load(&code, &file, rules).map(Some)
}

/// Submit the commands among `events` to `world`; the others are returned
/// for the interface.
pub fn submit(world: &mut World, events: Vec<ScriptEvent>) -> Vec<ScriptEvent> {
    events
        .into_iter()
        .filter_map(|e| match e {
            ScriptEvent::Command(c) => {
                world.submit(c);
                None
            }
            other => Some(other),
        })
        .collect()
}

/// A mission script loaded into its own Lua state.
pub struct MissionScript {
    lua: Lua,
    out: Rc<RefCell<Vec<ScriptEvent>>>,
    /// The globals before the script ran (the property names, `menu`,
    /// `triggers`): not the script's variables.
    builtins: std::collections::HashSet<String>,
}

impl std::fmt::Debug for MissionScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionScript")
    }
}

/// A number argument as a u32 id or count; nil and others are `None`.
fn as_u32(value: &Value) -> Option<u32> {
    let n = match value {
        Value::Integer(i) => *i as f64,
        Value::Number(n) => *n,
        _ => return None,
    };
    (n >= 0.0 && n <= f64::from(u32::MAX) && n.fract() == 0.0).then_some(n as u32)
}

/// Argument `i`, nil when missing.
fn arg(a: &[Value], i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Nil)
}

fn as_player(value: &Value) -> Option<PlayerId> {
    as_u32(value)
        .and_then(|p| u8::try_from(p).ok())
        .map(PlayerId)
}

fn as_point(x: &Value, y: &Value) -> Option<(u32, u32)> {
    Some((as_u32(x)?, as_u32(y)?))
}

impl MissionScript {
    /// CAM-1.1, 1.5: load `code` (named `name` in errors) with the property
    /// scripts' good and building names as globals.
    pub fn load(code: &[u8], name: &str, rules: &RuleSet) -> Result<Self, ScriptError> {
        let lua = Lua::new_with(StdLib::NONE, LuaOptions::default())?;
        let out = Rc::new(RefCell::new(Vec::new()));
        {
            let globals = lua.globals();
            for (key, &id) in &rules.names {
                globals.raw_set(key.as_str(), f64::from(id))?;
            }
            install(&lua, &out)?;
        }
        let builtins = global_names(&lua);
        lua.load(code)
            .set_name(format!("@{name}"))
            .set_mode(ChunkMode::Text)
            .exec()?;
        Ok(Self { lua, out, builtins })
    }

    /// FMT-SAV-3.12, CAM-1.1: the script's variables a save keeps, as
    /// (name, `u32` value) pairs in name order. Which globals the original
    /// keeps is Q-100 b; here every global the script set to a whole
    /// number from 0 to 2^32 - 1 (other values, tables and functions are
    /// not kept).
    pub fn variables(&self) -> Vec<(String, u32)> {
        let mut out: Vec<(String, u32)> = self
            .lua
            .globals()
            .pairs::<Value, Value>()
            .flatten()
            .filter_map(|(key, value)| {
                let Value::String(key) = key else { return None };
                let key = key.to_str().ok()?.to_string();
                if self.builtins.contains(&key) {
                    return None;
                }
                let n = match value {
                    Value::Integer(i) => i as f64,
                    Value::Number(n) => n,
                    _ => return None,
                };
                (n.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&n))
                    .then_some((key, n as u32))
            })
            .collect();
        out.sort();
        out
    }

    /// FMT-SAV-3.12, CAM-1.1: a save's variables set again after the
    /// script was loaded.
    pub fn set_variables(&self, variables: &[(String, u32)]) -> Result<(), ScriptError> {
        let globals = self.lua.globals();
        for (name, value) in variables {
            globals.set(name.as_str(), f64::from(*value))?;
        }
        Ok(())
    }

    /// Call the global function `name` if the script defines it.
    fn call(
        &self,
        name: &str,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<Vec<ScriptEvent>, ScriptError> {
        if let Ok(f) = self.lua.globals().get::<Function>(name) {
            f.call::<()>(args)?;
        }
        Ok(std::mem::take(&mut *self.out.borrow_mut()))
    }

    /// CAM-1.2 `onLoaded()`
    pub fn on_loaded(&self) -> Result<Vec<ScriptEvent>, ScriptError> {
        self.call("onLoaded", ())
    }

    /// CAM-1.2 `onStarted()`
    pub fn on_started(&self) -> Result<Vec<ScriptEvent>, ScriptError> {
        self.call("onStarted", ())
    }

    /// CAM-1.2 `onLogicTick(seconds)`
    pub fn on_logic_tick(&self, seconds: u32) -> Result<Vec<ScriptEvent>, ScriptError> {
        self.call("onLogicTick", seconds)
    }

    /// CAM-1.2, 2.3 `onTriggerIsActive(name)`
    pub fn on_trigger(&self, name: &str) -> Result<Vec<ScriptEvent>, ScriptError> {
        self.call("onTriggerIsActive", name)
    }

    /// The calls of one logic tick, after `world` has stepped: CAM-1.2
    /// `onLogicTick` with the whole seconds of the tick just run, then
    /// CAM-2.3 `onTriggerIsActive` for the first set trigger.
    pub fn after_step(&self, world: &World) -> Result<Vec<ScriptEvent>, ScriptError> {
        let seconds = (world.tick().saturating_sub(1) / 10) as u32;
        let mut events = self.on_logic_tick(seconds)?;
        if let Some(name) = world.active_trigger() {
            events.extend(self.on_trigger(name)?);
        }
        Ok(events)
    }
}

/// The names of the state's globals.
fn global_names(lua: &Lua) -> std::collections::HashSet<String> {
    lua.globals()
        .pairs::<Value, Value>()
        .flatten()
        .filter_map(|(key, _)| match key {
            Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
            _ => None,
        })
        .collect()
}

/// The `menu` and `triggers` tables (CAM-1.3, 1.4).
fn install(lua: &Lua, out: &Rc<RefCell<Vec<ScriptEvent>>>) -> mlua::Result<()> {
    let menu = lua.create_table()?;
    let text = |event: fn(String) -> ScriptEvent| {
        let out = out.clone();
        lua.create_function(move |_, key: String| {
            out.borrow_mut().push(event(key));
            Ok(())
        })
    };
    menu.set("openBookDialog", text(ScriptEvent::OpenBook)?)?;
    menu.set("addPage", text(ScriptEvent::AddPage)?)?;
    menu.set("addPicture", text(ScriptEvent::AddPicture)?)?;
    menu.set("addDebriefing", text(ScriptEvent::AddDebriefing)?)?;
    menu.set("addExtro", text(ScriptEvent::AddExtro)?)?;
    menu.set("createLetter", text(ScriptEvent::Letter)?)?;
    // A command from the arguments, or a warning naming the call.
    let command = |name: &'static str, make: fn(&[Value]) -> Option<Command>| {
        let out = out.clone();
        lua.create_function(move |_, args: mlua::Variadic<Value>| {
            let event = match make(&args) {
                Some(c) => ScriptEvent::Command(c),
                None => ScriptEvent::Warning(format!("menu.{name}: arguments {args:?}")),
            };
            out.borrow_mut().push(event);
            Ok(())
        })
    };
    menu.set(
        "addResource",
        command("addResource", |a| {
            Some(Command::AddGoods {
                player: as_player(&arg(a, 0))?,
                good: as_u32(&arg(a, 1))?,
                amount: as_u32(&arg(a, 2))?,
            })
        })?,
    )?;
    menu.set(
        "clearResources",
        command("clearResources", |a| {
            Some(Command::ClearGoods {
                player: as_player(&arg(a, 0))?,
            })
        })?,
    )?;
    menu.set(
        "lockBuilding",
        command("lockBuilding", |a| {
            Some(Command::LockBuilding {
                player: as_player(&arg(a, 0))?,
                building: as_u32(&arg(a, 1))?,
            })
        })?,
    )?;
    menu.set(
        "createBuilding",
        command("createBuilding", |a| {
            Some(Command::CreateBuilding {
                player: as_player(&arg(a, 0))?,
                building: as_u32(&arg(a, 1))?,
                at: as_point(&arg(a, 2), &arg(a, 3))?,
                settler: None,
            })
        })?,
    )?;
    menu.set(
        "createBuildingWithSettler",
        command("createBuildingWithSettler", |a| {
            Some(Command::CreateBuilding {
                player: as_player(&arg(a, 0))?,
                building: as_u32(&arg(a, 1))?,
                at: as_point(&arg(a, 2), &arg(a, 3))?,
                settler: Some(as_u32(&arg(a, 4))?),
            })
        })?,
    )?;
    menu.set(
        "createShip",
        command("createShip", |a| {
            Some(Command::CreateShip {
                player: as_player(&arg(a, 0))?,
                at: as_point(&arg(a, 1), &arg(a, 2))?,
            })
        })?,
    )?;
    menu.set(
        "createStreet",
        command("createStreet", |a| {
            Some(Command::CreateStreet {
                player: as_player(&arg(a, 0))?,
                from: as_point(&arg(a, 1), &arg(a, 2))?,
                to: as_point(&arg(a, 3), &arg(a, 4))?,
            })
        })?,
    )?;
    let colour_out = out.clone();
    menu.set(
        "setPlayerColor",
        lua.create_function(move |_, (p, r, g, b): (Value, f32, f32, f32)| {
            let event = match as_player(&p) {
                Some(player) => ScriptEvent::PlayerColor {
                    player,
                    rgb: [r, g, b].map(|c| c.clamp(0.0, 1.0)),
                },
                None => ScriptEvent::Warning(format!("menu.setPlayerColor: player {p:?}")),
            };
            colour_out.borrow_mut().push(event);
            Ok(())
        })?,
    )?;
    // CAM-1.3: not used by the shipped scripts; accepted and ignored.
    for name in ["setLanguage", "setVideoVoiceStream"] {
        menu.set(
            name,
            lua.create_function(|_, _: mlua::Variadic<Value>| Ok(()))?,
        )?;
    }
    lua.globals().set("menu", menu)?;

    let triggers = lua.create_table()?;
    let trigger_out = out.clone();
    triggers.set(
        "removeTrigger",
        lua.create_function(move |_, name: String| {
            trigger_out
                .borrow_mut()
                .push(ScriptEvent::Command(Command::RemoveTrigger { name }));
            Ok(())
        })?,
    )?;
    lua.globals().set("triggers", triggers)
}

#[cfg(test)]
mod start_tests {
    use super::*;

    // spec: CAM-3.1a (T12)
    #[test]
    fn the_start_settings_give_the_target() {
        // A campaign mission 4, a lesson 0, a free game its chosen entry,
        // whatever the header says.
        assert_eq!(start_target(1, 2), Some(4));
        assert_eq!(start_target(2, 2), Some(0));
        assert_eq!(start_target(0, 2), Some(2));
        assert_eq!(start_target(100, 3), Some(3));
        assert_eq!(start_target(102, 0), Some(0));
        assert_eq!(start_target(3, 2), None);
    }

    /// CAM-4.3a: mission 5's row: the human on team 1 with its ally P2, strength
    /// picking the profile, ignored on the human; a slot of kind 0 is no player
    /// and a player without a start position neither; a colour may be shared.
    // spec: CAM-4.3a, FMT-S2M-2.5, FMT-S2M-2.6, UI-SETUP-1.5
    #[test]
    fn a_campaign_maps_records_set_its_players() {
        let kinds = [2, 1, 1, 1, 0, 0, 1, 0];
        let records = [
            [0, 0, 1, 2],
            [2, 1, 0, 2],
            [1, 5, 1, 1],
            [2, 1, 0, 0],
            [0, 4, -1, 0],
            [0, 5, -1, 0],
            [0, 6, -1, 0],
            [0, 7, -1, 0],
        ];
        let players = campaign_players(&kinds, &records, 6);
        let row = |p: &SetupPlayer| (p.player.0, p.tribe, p.colour, p.team, p.profile);
        assert_eq!(
            players.iter().map(row).collect::<Vec<_>>(),
            [
                (0, 0, 0, 1, None),
                (1, 2, 1, 0, Some(3)),
                (2, 1, 5, 1, Some(2)),
                (3, 2, 1, 0, Some(1)),
            ]
        );
        assert!(players.iter().all(|p| p.start == usize::from(p.player.0)));
        assert_eq!(colours(&players), [0, 1, 5, 1, 4, 5, 6, 7]);
        // Only as many as the map has start positions.
        assert_eq!(campaign_players(&kinds, &records, 2).len(), 2);
    }

    /// ECO-HQ-2.2, CAM-4.3, UI-SETUP-2.4: setting 1 for a mission, 2 for a lesson.
    // spec: ECO-HQ-2.2, CAM-4.3, UI-SETUP-2.4
    #[test]
    fn missions_and_lessons_have_their_start_stock() {
        assert_eq!(start_resources(1), 1);
        assert_eq!(start_resources(2), 2);
        assert_eq!(s2_sim::start_multiplier(start_resources(1)), 2);
        assert_eq!(s2_sim::start_multiplier(start_resources(2)), 4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAMPAIGN: &str = r#"
player0 = 0 -- user
player1 = 1

function onLoaded()
	menu.addDebriefing("b01_C_00")
	menu.lockBuilding(player0,guardHouse)
	menu.lockBuilding(player0,unknownThing)
end

function onStarted()
	menu.addResource(player0,good_soldier0,40)
end

function onLogicTick(seconds)
    if seconds == 5 * 5 then
		menu.openBookDialog("b01_A_00")
		menu.addPage("b01_A_01")
		menu.addPicture("ui_briefing_screen_1")
    end
end

function onTriggerIsActive(name)
    if name == "Region_K" then
	    menu.openBookDialog("b01_B_00")
        triggers.removeTrigger(name)
    end
end
"#;

    fn rules() -> RuleSet {
        let mut rules = RuleSet::default();
        rules.names.insert("guardHouse".into(), 0x12e6_7603);
        rules.names.insert("good_soldier0".into(), 0x15e8_6843);
        rules
    }

    // spec: CAM-1.2, CAM-1.3
    #[test]
    fn callbacks_turn_into_events() {
        // CAM T1 (script side)
        let script = MissionScript::load(CAMPAIGN.as_bytes(), "campaign01.lua", &rules()).unwrap();
        let loaded = script.on_loaded().unwrap();
        assert_eq!(loaded[0], ScriptEvent::AddDebriefing("b01_C_00".into()));
        assert_eq!(
            loaded[1],
            ScriptEvent::Command(Command::LockBuilding {
                player: PlayerId(0),
                building: 0x12e6_7603
            })
        );
        assert!(matches!(loaded[2], ScriptEvent::Warning(_)));
        assert_eq!(
            script.on_started().unwrap(),
            vec![ScriptEvent::Command(Command::AddGoods {
                player: PlayerId(0),
                good: 0x15e8_6843,
                amount: 40
            })]
        );
        assert!(script.on_logic_tick(24).unwrap().is_empty());
        assert_eq!(
            script.on_logic_tick(25).unwrap(),
            vec![
                ScriptEvent::OpenBook("b01_A_00".into()),
                ScriptEvent::AddPage("b01_A_01".into()),
                ScriptEvent::AddPicture("ui_briefing_screen_1".into()),
            ]
        );
        // T2 (script side)
        assert_eq!(
            script.on_trigger("Region_K").unwrap(),
            vec![
                ScriptEvent::OpenBook("b01_B_00".into()),
                ScriptEvent::Command(Command::RemoveTrigger {
                    name: "Region_K".into()
                }),
            ]
        );
    }

    // spec: CAM-1.2
    #[test]
    fn missing_callbacks_and_errors() {
        let script = MissionScript::load(b"x = 1", "empty.lua", &rules()).unwrap();
        assert!(script.on_loaded().unwrap().is_empty());
        assert!(script.on_trigger("a").unwrap().is_empty());
        let broken =
            MissionScript::load(b"function onStarted() nothere() end", "b.lua", &rules()).unwrap();
        let e = broken.on_started().unwrap_err();
        assert!(e.0.contains("b.lua"), "{e}");
        // No file access from a mission script.
        let io =
            MissionScript::load(b"function onLoaded() dofile('x') end", "c.lua", &rules()).unwrap();
        assert!(io.on_loaded().is_err());
    }

    // spec: CAM-5.2
    #[test]
    fn tutorial_calls() {
        let code = r#"
function onStarted()
   menu.clearResources(player0)
   menu.createBuildingWithSettler(player0,guardHouse,51,36,good_soldier0)
   menu.createStreet(player0,51,37,45,37)
   menu.createShip(player0,101,63)
   menu.setPlayerColor(1, 0.5, 2, 0)
end
player0 = 0
"#;
        let script = MissionScript::load(code.as_bytes(), "t.lua", &rules()).unwrap();
        let events = script.on_started().unwrap();
        assert_eq!(
            events,
            vec![
                ScriptEvent::Command(Command::ClearGoods {
                    player: PlayerId(0)
                }),
                ScriptEvent::Command(Command::CreateBuilding {
                    player: PlayerId(0),
                    building: 0x12e6_7603,
                    at: (51, 36),
                    settler: Some(0x15e8_6843)
                }),
                ScriptEvent::Command(Command::CreateStreet {
                    player: PlayerId(0),
                    from: (51, 37),
                    to: (45, 37)
                }),
                ScriptEvent::Command(Command::CreateShip {
                    player: PlayerId(0),
                    at: (101, 63)
                }),
                ScriptEvent::PlayerColor {
                    player: PlayerId(1),
                    rgb: [0.5, 1.0, 0.0]
                },
            ]
        );
    }

    // FMT-SAV-3.12, CAM-1.1: the script's numbers are its variables a save
    // keeps; set again on a script loaded anew, they are as saved.
    // spec: FMT-SAV-3.12, CAM-1.1
    #[test]
    fn script_variables_are_kept() {
        let code = b"step = 3 big = 7.5 done = false label = 'x' t = {} \
            function onLogicTick(s) step = step + 1 end";
        let script = MissionScript::load(code, "v.lua", &rules()).unwrap();
        assert_eq!(script.variables(), [("step".to_owned(), 3)]);
        script.on_logic_tick(1).unwrap();
        script.on_logic_tick(2).unwrap();
        let saved = script.variables();
        assert_eq!(saved, [("step".to_owned(), 5)]);
        // The property names are not variables, even when a number.
        assert!(!saved.iter().any(|(n, _)| n == "guardHouse"));
        let again = MissionScript::load(code, "v.lua", &rules()).unwrap();
        again.set_variables(&saved).unwrap();
        assert_eq!(again.variables(), saved);
        again.on_logic_tick(3).unwrap();
        assert_eq!(again.variables(), [("step".to_owned(), 6)]);
    }
}
