//! Runs the original property scripts against our own implementation of the
//! `properties` API (`specs/LUA-properties-api.md`).
//!
//! Every function of the spec's tables is registered and writes into a
//! [`RuleSet`]. A call to a function the spec does not list is an error that
//! names the function, as is a call with arguments of the wrong shape.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use mlua::chunk::ChunkMode;
use mlua::{FromLua, FromLuaMulti, IntoLuaMulti, Lua, LuaOptions, MultiValue, StdLib, Value};

use crate::model::*;

/// Where scripts come from. `path` is as the scripts give it: relative to the
/// game directory, with backslashes (LUA-PROP-1.3).
pub trait ScriptSource {
    fn read(&self, path: &str) -> Result<Vec<u8>, String>;
}

/// A script run that failed: the Lua error, with the script name and line
/// where it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError(pub String);

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ScriptError {}

impl From<mlua::Error> for ScriptError {
    fn from(e: mlua::Error) -> Self {
        Self(e.to_string())
    }
}

/// What happened while the scripts ran, beyond the rules themselves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// Paths of the scripts run, in order, as they were asked for.
    pub scripts: Vec<String>,
    /// Number of calls per `properties` function.
    pub calls: BTreeMap<&'static str, u32>,
    /// The first `properties` function called.
    pub first_call: Option<&'static str>,
    /// Lines passed to `Default.WriteToLog`.
    pub log: Vec<String>,
}

impl LoadReport {
    fn record(&mut self, name: &'static str) {
        *self.calls.entry(name).or_default() += 1;
        self.first_call.get_or_insert(name);
    }

    pub fn call_count(&self, name: &str) -> u32 {
        self.calls.get(name).copied().unwrap_or(0)
    }
}

/// A script run after the original ones, in the same environment: it sees
/// the globals the originals left (`building_woodcutter`, `good_stone`) and
/// calls the same `properties` functions. Not part of the original game;
/// the hook for projects built on this engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    /// A name for errors and the load report (recorded as `overlay:<name>`).
    pub name: String,
    pub code: Vec<u8>,
}

/// Run `entry` (and every script it calls) and return the rules they set up.
pub fn run(
    source: Rc<dyn ScriptSource>,
    entry: &str,
) -> Result<(RuleSet, LoadReport), ScriptError> {
    run_with_overlays(source, entry, &[])
}

/// [`run`], then each overlay in order in the same environment.
pub fn run_with_overlays(
    source: Rc<dyn ScriptSource>,
    entry: &str,
    overlays: &[Overlay],
) -> Result<(RuleSet, LoadReport), ScriptError> {
    // LUA-PROP-1.1: the scripts call no standard library function, so none is
    // loaded.
    let lua = Lua::new_with(StdLib::NONE, LuaOptions::default())?;
    clear_globals(&lua)?;
    let state = Rc::new(RefCell::new(State::default()));
    install(&lua, &state, source)?;
    run_script(&lua, &state, entry)?;
    if !overlays.is_empty() {
        install_project(&lua, &state)?;
    }
    // W12: the building types the overlays create are a project's.
    let original: Vec<Id> = state
        .borrow()
        .rules
        .buildings
        .iter()
        .map(|b| b.id)
        .collect();
    for overlay in overlays {
        let name = format!("overlay:{}", overlay.name);
        state.borrow_mut().report.scripts.push(name.clone());
        lua.load(overlay.code.as_slice())
            .set_name(format!("@{name}"))
            .set_mode(ChunkMode::Text)
            .exec()?;
    }
    {
        let rules = &mut state.borrow_mut().rules;
        rules.project_buildings = rules
            .buildings
            .iter()
            .map(|b| b.id)
            .filter(|id| !original.contains(id))
            .collect();
    }
    record_names(&lua, &state)?;
    drop(lua);
    let state = Rc::try_unwrap(state)
        .map_err(|_| ScriptError("script state still in use".into()))?
        .into_inner();
    Ok((state.rules, state.report))
}

/// LUA-PROP-1.1: the scripts call no standard library function. The base
/// library is always loaded, so its functions (among them `dofile` and
/// `loadfile`, which read files) are removed here.
fn clear_globals(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    let names: Vec<Value> = globals
        .pairs::<Value, Value>()
        .map(|pair| pair.map(|(name, _)| name))
        .collect::<mlua::Result<_>>()?;
    for name in names {
        globals.raw_set(name, Value::Nil)?;
    }
    Ok(())
}

/// LUA-PROP-1.6: the scripts name their objects with ordinary globals that
/// hold the handles (`good_plank`, `building_castle`). Keep the names of the
/// globals that hold the id of a good or building, so that specs can refer to
/// objects by them.
fn record_names(lua: &Lua, state: &Shared) -> mlua::Result<()> {
    let mut state = state.borrow_mut();
    let rules = &mut state.rules;
    for pair in lua.globals().pairs::<Value, Value>() {
        let (name, value) = pair?;
        let (Value::String(name), Some(id)) = (&name, number(&value)) else {
            continue;
        };
        if id.fract() != 0.0 || !(0.0..=f64::from(u32::MAX)).contains(&id) {
            continue;
        }
        let id = id as Id;
        if rules.goods.contains(id) || rules.buildings.contains(id) {
            rules.names.insert(latin1(&name.as_bytes()), id);
        }
    }
    Ok(())
}

#[derive(Default)]
struct State {
    rules: RuleSet,
    report: LoadReport,
    source: Option<Rc<dyn ScriptSource>>,
}

type Shared = Rc<RefCell<State>>;

/// LUA-PROP-1.3: run a script file in the shared global environment.
fn run_script(lua: &Lua, state: &Shared, path: &str) -> mlua::Result<()> {
    let source = state.borrow().source.clone().expect("source installed");
    let code = source.read(path).map_err(mlua::Error::runtime)?;
    state.borrow_mut().report.scripts.push(path.to_owned());
    lua.load(code)
        .set_name(format!("@{path}"))
        .set_mode(ChunkMode::Text)
        .exec()
}

/// A table whose missing entries are an error naming the entry.
fn strict_table(lua: &Lua, table_name: &'static str) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    let meta = lua.create_table()?;
    meta.raw_set(
        "__index",
        lua.create_function(move |_, (_, key): (Value, Value)| -> mlua::Result<()> {
            let key = match key {
                Value::String(s) => latin1(&s.as_bytes()),
                other => format!("{other:?}"),
            };
            Err(mlua::Error::runtime(format!(
                "unknown function {table_name}.{key}"
            )))
        })?,
    )?;
    table.set_metatable(Some(meta))?;
    Ok(table)
}

fn install(lua: &Lua, state: &Shared, source: Rc<dyn ScriptSource>) -> mlua::Result<()> {
    state.borrow_mut().source = Some(source);

    // LUA-PROP-1.3, LUA-PROP-1.4
    let default = strict_table(lua, "Default")?;
    let shared = state.clone();
    default.raw_set(
        "CallScript",
        lua.create_function(move |lua, Text(path): Text| run_script(lua, &shared, &path))?,
    )?;
    let shared = state.clone();
    default.raw_set(
        "WriteToLog",
        lua.create_function(move |_, Text(line): Text| {
            shared.borrow_mut().report.log.push(line);
            Ok(())
        })?,
    )?;
    lua.globals().raw_set("Default", default)?;

    let properties = strict_table(lua, "properties")?;
    register_all(&Registry {
        lua,
        table: &properties,
        state,
    })?;
    lua.globals().raw_set("properties", properties)
}

struct Registry<'a> {
    lua: &'a Lua,
    table: &'a mlua::Table,
    state: &'a Shared,
}

impl Registry<'_> {
    /// Register `properties.<name>`. Arguments are converted to `A`; more
    /// arguments than `A` has, or ones of the wrong kind, are an error.
    fn def<A, R>(
        &self,
        name: &'static str,
        f: impl Fn(&mut RuleSet, A) -> Result<R, String> + 'static,
    ) -> mlua::Result<()>
    where
        A: Args,
        R: IntoLuaMulti,
    {
        let shared = self.state.clone();
        let function = self.lua.create_function(move |lua, args: MultiValue| {
            let fail = |message: String| mlua::Error::runtime(format!("{name}: {message}"));
            if args.len() > A::COUNT {
                return Err(fail(format!(
                    "{} arguments, expected at most {}",
                    args.len(),
                    A::COUNT
                )));
            }
            let args = A::from_lua_multi(args, lua).map_err(|e| fail(e.to_string()))?;
            let mut state = shared.borrow_mut();
            state.report.record(name);
            f(&mut state.rules, args).map_err(fail)
        })?;
        self.table.raw_set(name, function)
    }
}

/// Argument tuples, with how many arguments they take at most.
trait Args: FromLuaMulti {
    const COUNT: usize;
}

macro_rules! args {
    ($count:literal: $($name:ident),+) => {
        impl<$($name: FromLua),+> Args for ($($name,)+) {
            const COUNT: usize = $count;
        }
    };
}
args!(1: A);
args!(2: A, B);
args!(3: A, B, C);
args!(4: A, B, C, D);
args!(5: A, B, C, D, E);
args!(6: A, B, C, D, E, F);

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(i) => Some(*i as f64),
        Value::Number(n) => Some(*n),
        _ => None,
    }
}

fn kind_error(value: &Value, expected: &str) -> mlua::Error {
    mlua::Error::runtime(format!("expected {expected}, got {}", value.type_name()))
}

fn integer(value: &Value) -> mlua::Result<i64> {
    match number(value) {
        Some(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Ok(n as i64),
        Some(n) => Err(mlua::Error::runtime(format!(
            "expected an integer, got {n}"
        ))),
        None => Err(kind_error(value, "an integer")),
    }
}

/// A Lua string, Latin-1 decoded (LUA-PROP-1.1).
struct Text(String);

/// A number.
struct Num(f64);

/// An integer.
struct Int(i64);

/// A non-negative integer.
struct Count(u32);

/// A Lua boolean.
struct Flag(bool);

/// The handle of an object (LUA-PROP-2.2). Handles are the objects' ids.
#[derive(Clone, Copy)]
struct Handle(Id);

/// A handle or -1 for "no object" (LUA-PROP-2.2).
struct Ref(Option<Id>);

impl FromLua for Text {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match &value {
            Value::String(s) => Ok(Self(latin1(&s.as_bytes()))),
            _ => Err(kind_error(&value, "a string")),
        }
    }
}

impl FromLua for Num {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        number(&value)
            .map(Self)
            .ok_or_else(|| kind_error(&value, "a number"))
    }
}

impl FromLua for Int {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        integer(&value).map(Self)
    }
}

impl FromLua for Count {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        let n = integer(&value)?;
        u32::try_from(n)
            .map(Self)
            .map_err(|_| mlua::Error::runtime(format!("expected a count, got {n}")))
    }
}

impl FromLua for Flag {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::Boolean(b) => Ok(Self(b)),
            other => Err(kind_error(&other, "a boolean")),
        }
    }
}

impl FromLua for Handle {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        let n = integer(&value).map_err(|_| kind_error(&value, "a handle"))?;
        u32::try_from(n)
            .map(Self)
            .map_err(|_| mlua::Error::runtime(format!("{n} is not a handle")))
    }
}

impl FromLua for Ref {
    fn from_lua(value: Value, lua: &Lua) -> mlua::Result<Self> {
        if number(&value) == Some(-1.0) {
            return Ok(Self(None));
        }
        Handle::from_lua(value, lua).map(|Handle(id)| Self(Some(id)))
    }
}

/// LUA-PROP-2.1: 1 to 8 hexadecimal digits.
fn hex_id(text: &str) -> Result<Id, String> {
    // Q-029: one shipped create passes its id with a `0x` prefix, which the
    // spec does not allow for. It is read as hex notation until the
    // investigation side says how the engine reads it.
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    let valid = (1..=8).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_hexdigit());
    if !valid {
        return Err(format!("\"{text}\" is not an id of 1 to 8 hex digits"));
    }
    Id::from_str_radix(digits, 16).map_err(|e| e.to_string())
}

/// LUA-PROP-2.3: AI profiles are numbered in decimal.
fn decimal_id(text: &str) -> Result<Id, String> {
    text.parse()
        .map_err(|_| format!("\"{text}\" is not a profile number"))
}

/// Create an object; its handle is its id.
fn create<T: Record>(table: &mut Table<T>, id: Id) -> Result<f64, String> {
    match table.create(id) {
        Some(_) => Ok(f64::from(id)),
        None => Err(format!("id {id:08X} already exists")),
    }
}

fn object<T: Record>(table: &mut Table<T>, Handle(id): Handle) -> Result<&mut T, String> {
    table
        .get_mut(id)
        .ok_or_else(|| format!("unknown handle {id}"))
}

fn script_enum<T>(value: i64, from: fn(i64) -> Option<T>, what: &str) -> Result<T, String> {
    from(value).ok_or_else(|| format!("{value} is not a {what}"))
}

fn rank(value: i64) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|&r| r < SOLDIER_RANKS)
        .ok_or_else(|| format!("{value} is not a soldier rank (0..4)"))
}

fn color_channel(value: i64) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| format!("{value} is not a colour value (0..255)"))
}

/// `name(handle, value)` setting one field of an object. The value type's
/// conversion decides what the field holds.
macro_rules! setter {
    ($r:ident, $name:literal, $table:ident . $field:ident : Text) => {
        $r.def($name, |rules, (h, Text(v)): (Handle, Text)| {
            object(&mut rules.$table, h)?.$field = Some(v);
            Ok(())
        })?
    };
    ($r:ident, $name:literal, $table:ident . $field:ident : Num) => {
        $r.def($name, |rules, (h, Num(v)): (Handle, Num)| {
            object(&mut rules.$table, h)?.$field = Some(v);
            Ok(())
        })?
    };
    ($r:ident, $name:literal, $table:ident . $field:ident : Int) => {
        $r.def($name, |rules, (h, Int(v)): (Handle, Int)| {
            object(&mut rules.$table, h)?.$field = Some(v);
            Ok(())
        })?
    };
    ($r:ident, $name:literal, $table:ident . $field:ident : Count) => {
        $r.def($name, |rules, (h, Count(v)): (Handle, Count)| {
            object(&mut rules.$table, h)?.$field = Some(v);
            Ok(())
        })?
    };
    ($r:ident, $name:literal, $table:ident . $field:ident : Flag) => {
        $r.def($name, |rules, (h, Flag(v)): (Handle, Flag)| {
            object(&mut rules.$table, h)?.$field = Some(v);
            Ok(())
        })?
    };
    ($r:ident, $name:literal, $table:ident . $field:ident : Ref) => {
        $r.def($name, |rules, (h, Ref(v)): (Handle, Ref)| {
            object(&mut rules.$table, h)?.$field = v;
            Ok(())
        })?
    };
    ($r:ident, $name:literal, $table:ident . $field:ident : enum $type:ident) => {
        $r.def($name, |rules, (h, Int(v)): (Handle, Int)| {
            let value = script_enum(v, $type::from_number, stringify!($type))?;
            object(&mut rules.$table, h)?.$field = Some(value);
            Ok(())
        })?
    };
}

/// `name(value)` setting one field of a global record.
macro_rules! global {
    ($r:ident, $name:literal, $record:ident . $field:ident : $kind:ident) => {
        $r.def($name, |rules, ($kind(v),): ($kind,)| {
            rules.$record.$field = Some(v);
            Ok(())
        })?
    };
}

/// `name(handle, good, count)` adding to a list of amounts.
macro_rules! add_amount {
    ($r:ident, $name:literal, $table:ident . $field:ident) => {
        $r.def(
            $name,
            |rules, (h, Handle(good), Count(count)): (Handle, Handle, Count)| {
                object(&mut rules.$table, h)?
                    .$field
                    .push(Amount { good, count });
                Ok(())
            },
        )?
    };
}

/// `name(handle, good, count)` setting one amount.
macro_rules! set_amount {
    ($r:ident, $name:literal, $table:ident . $field:ident) => {
        $r.def(
            $name,
            |rules, (h, Handle(good), Count(count)): (Handle, Handle, Count)| {
                object(&mut rules.$table, h)?.$field = Some(Amount { good, count });
                Ok(())
            },
        )?
    };
}

/// `xx_create("<hex>")`.
macro_rules! creator {
    ($r:ident, $name:literal, $table:ident) => {
        $r.def($name, |rules, (Text(id),): (Text,)| {
            create(&mut rules.$table, hex_id(&id)?)
        })?
    };
}

/// `name(handle, int x, int y)` setting an icon cell.
macro_rules! icon {
    ($r:ident, $name:literal, $table:ident) => {
        $r.def(
            $name,
            |rules, (h, Int(column), Int(row)): (Handle, Int, Int)| {
                object(&mut rules.$table, h)?.icon = Some((column, row));
                Ok(())
            },
        )?
    };
}

fn register_all(r: &Registry) -> mlua::Result<()> {
    // LUA-PROP-4
    global!(r, "cfg_setGameSpeed", settings.game_speed: Num);
    global!(r, "cfg_setSettlerSpeed", settings.settler_speed: Num);
    global!(r, "cfg_setPickupTime", settings.pickup_time: Num);
    global!(r, "cfg_setDropTime", settings.drop_time: Num);
    global!(r, "cfg_setBulldozeTime", settings.bulldoze_time: Num);
    global!(r, "cfg_setConstructTime", settings.construct_time: Num);
    global!(r, "cfg_setFlagPackageLimit", settings.flag_package_limit: Count);
    global!(r, "cfg_setFishingDistance", settings.fishing_distance: Count);
    global!(r, "cfg_setCatapultRotationTimeFactor", settings.catapult_rotation_time_factor: Num);
    global!(r, "cfg_setCatapultFlightTimeFactor", settings.catapult_flight_time_factor: Num);
    global!(r, "cfg_setSettlerDyingTime", settings.settler_dying_time: Int);
    r.def(
        "cfg_setSettlerSkeletonDoodad",
        |rules, (Ref(v),): (Ref,)| {
            rules.settings.settler_skeleton_doodad = v;
            Ok(())
        },
    )?;
    r.def("cfg_setBorderPattern", |rules, (Ref(v),): (Ref,)| {
        rules.settings.border_pattern = v;
        Ok(())
    })?;

    // LUA-PROP-5
    creator!(r, "gp_create", goods);
    setter!(r, "gp_setName", goods.name: Text);
    setter!(r, "gp_setShortName", goods.short_name: Text);
    icon!(r, "gp_setIconXY", goods);
    setter!(r, "gp_setType", goods.good_type: enum GoodType);
    setter!(r, "gp_setSort", goods.sort: Int);
    setter!(r, "gp_setTransportPriority", goods.transport_priority: Int);
    setter!(r, "gp_setAiPriority", goods.ai_priority: Int);
    setter!(r, "gp_setGroupGoodId", goods.group: Ref);
    add_amount!(r, "gp_addCost", goods.costs);
    setter!(r, "gp_setRestTime", goods.rest_time: Num);
    setter!(r, "gp_setPlantable", goods.plantable: Flag);
    setter!(r, "gp_enableDeposits", goods.deposits_enabled: Int);
    setter!(r, "gp_enableFields", goods.fields_enabled: Int);
    setter!(r, "gp_setValue", goods.value: Num);

    // LUA-PROP-6
    creator!(r, "bp_create", buildings);
    setter!(r, "bp_setName", buildings.name: Text);
    setter!(r, "bp_setGfxName", buildings.gfx_name: Text);
    icon!(r, "bp_setIconXY", buildings);
    setter!(r, "bp_setSort", buildings.sort: Int);
    setter!(r, "bp_setSize", buildings.size: enum BuildingSize);
    add_amount!(r, "bp_addFrameCost", buildings.frame_costs);
    add_amount!(r, "bp_addCost", buildings.costs);
    setter!(r, "bp_setConstructionTime", buildings.construction_time: Num);
    r.def(
        "bp_setConstructionPosition",
        |rules, (h, Int(tribe), Count(index), Num(x), Num(y)): (Handle, Int, Count, Num, Num)| {
            let tribe = script_enum(tribe, Tribe::from_number, "Tribe")?;
            let positions = &mut object(&mut rules.buildings, h)?.construction_positions;
            // LUA-PROP-2.5: the last call for a spot wins.
            positions.retain(|p| (p.tribe, p.index) != (tribe, index));
            positions.push(ConstructionPosition { tribe, index, x, y });
            Ok(())
        },
    )?;
    r.def(
        "bp_setDoorOffsetDirection",
        |rules, (h, Int(tribe), Int(direction)): (Handle, Int, Int)| {
            let tribe = script_enum(tribe, Tribe::from_number, "Tribe")?;
            let direction = script_enum(direction, DoorDirection::from_number, "DoorDirection")?;
            let doors = &mut object(&mut rules.buildings, h)?.door_directions;
            doors.retain(|&(t, _)| t != tribe);
            doors.push((tribe, direction));
            Ok(())
        },
    )?;
    setter!(r, "bp_setWorker", buildings.worker: Ref);
    setter!(r, "bp_setWorkerLimit", buildings.worker_limit: Count);
    set_amount!(r, "bp_setGoods", buildings.goods);
    set_amount!(r, "bp_setSecondGoods", buildings.second_goods);
    // LUA-PROP-6: the same function as bp_setSecondGoods under another name.
    set_amount!(r, "bp_setAlternativeGoods", buildings.second_goods);
    setter!(r, "bp_setSwitchableProduction", buildings.switchable_production: Flag);
    setter!(r, "bp_setProductionTime", buildings.production_time: Num);
    setter!(r, "bp_setStockLimit", buildings.stock_limit: Count);
    setter!(r, "bp_setWorkingRadius", buildings.working_radius: Count);
    setter!(r, "bp_setPlantingGood", buildings.planting_good: Ref);
    setter!(r, "bp_setDepot", buildings.depot: Int);
    setter!(r, "bp_setSpawn", buildings.spawn: Int);
    setter!(r, "bp_setHarbor", buildings.harbor: Flag);
    setter!(r, "bp_setCatapult", buildings.catapult: Flag);
    setter!(r, "bp_setSoldierLimit", buildings.soldier_limit: Count);
    setter!(r, "bp_setTerritoryRange", buildings.territory_range: Count);
    setter!(r, "bp_setExploreRadius", buildings.explore_radius: Count);
    setter!(r, "bp_setRemains", buildings.remains: Ref);
    setter!(r, "bp_setAiScore", buildings.ai_score: Int);
    setter!(r, "bp_setAiThreat", buildings.ai_threat: Int);
    setter!(r, "bp_setHealth", buildings.health: Num);

    // LUA-PROP-7
    creator!(r, "pa_create", patterns);
    setter!(r, "pa_setName", patterns.name: Text);
    setter!(r, "pa_setGfxName", patterns.gfx_name: Text);
    setter!(r, "pa_setBlocked", patterns.blocked: Flag);
    setter!(r, "pa_setForBuilding", patterns.for_building: Flag);
    setter!(r, "pa_setForPlanting", patterns.for_planting: Flag);
    setter!(r, "pa_setForMining", patterns.for_mining: Flag);
    setter!(r, "pa_setForHarbor", patterns.for_harbor: Flag);
    setter!(r, "pa_setTreeType", patterns.tree_type: enum TreeSet);
    r.def(
        "pa_setMiniMapColor",
        |rules, (h, Int(red), Int(green), Int(blue)): (Handle, Int, Int, Int)| {
            let color = [
                color_channel(red)?,
                color_channel(green)?,
                color_channel(blue)?,
            ];
            object(&mut rules.patterns, h)?.minimap_color = Some(color);
            Ok(())
        },
    )?;

    // LUA-PROP-8
    creator!(r, "dp_create", deposits);
    setter!(r, "dp_setName", deposits.name: Text);
    r.def(
        "dp_setGfxName",
        |rules, (h, Text(name), variant): (Handle, Text, Option<Int>)| {
            let deposit = object(&mut rules.deposits, h)?;
            deposit.gfx_name = Some(name);
            deposit.gfx_variant = variant.map(|Int(v)| v);
            Ok(())
        },
    )?;
    setter!(r, "dp_setGood", deposits.good: Ref);
    setter!(r, "dp_setBlocking", deposits.blocking: Flag);
    setter!(r, "dp_nextDeposit", deposits.next_deposit: Ref);
    setter!(r, "dp_setElevationSize", deposits.elevation_size: Int);
    r.def(
        "dp_addUndergroundPattern",
        |rules, (h, Handle(pattern)): (Handle, Handle)| {
            object(&mut rules.deposits, h)?
                .underground_patterns
                .push(pattern);
            Ok(())
        },
    )?;
    setter!(r, "dp_setRandomFinePosOffsetState", deposits.random_fine_pos_offset: Flag);
    setter!(r, "dp_setNeedForPlantablePattern", deposits.needs_plantable_pattern: Flag);
    setter!(r, "dp_setGrowingSpeed", deposits.growing_speed: Num);
    setter!(r, "dp_setGrowingStartSize", deposits.growing_start_size: Num);
    setter!(r, "dp_setLifeTime", deposits.life_time: Int);
    setter!(r, "dp_setDoodadAfterDestruction", deposits.doodad_after_destruction: Ref);

    // LUA-PROP-9
    creator!(r, "dd_create", doodads);
    setter!(r, "dd_setName", doodads.name: Text);
    setter!(r, "dd_setGfxName", doodads.gfx_name: Text);
    setter!(r, "dd_setBlocking", doodads.blocking: Flag);
    setter!(r, "dd_setType", doodads.doodad_type: enum DoodadType);
    setter!(r, "dd_setLifeTime", doodads.life_time: Int);
    setter!(r, "dd_setDeleteByStreet", doodads.delete_by_street: Flag);
    setter!(r, "dd_setOnWater", doodads.on_water: Flag);
    creator!(r, "rm_create", remains);
    setter!(r, "rm_setName", remains.name: Text);
    setter!(r, "rm_setGfxName", remains.gfx_name: Text);
    setter!(r, "rm_setBurningTime", remains.burning_time: Int);
    setter!(r, "rm_setDoodadAfterFinished", remains.doodad_after_finished: Ref);
    r.def(
        "si_addSignDoodadId",
        |rules, (Ref(good), Count(threshold), Handle(doodad)): (Ref, Count, Handle)| {
            rules.signs.push(Sign {
                good,
                threshold,
                doodad,
            });
            Ok(())
        },
    )?;

    // LUA-PROP-10
    global!(r, "mi_setFightingTime", military.fighting_time: Num);
    r.def(
        "mi_setSoldierId",
        |rules, (Int(rank_number), Ref(good)): (Int, Ref)| {
            rules.military.soldiers[rank(rank_number)?] = good;
            Ok(())
        },
    )?;
    r.def(
        "mi_setSoldierMaxHealth",
        |rules, (Int(rank_number), Int(health)): (Int, Int)| {
            rules.military.soldier_max_health[rank(rank_number)?] = Some(health);
            Ok(())
        },
    )?;
    r.def(
        "mi_addBuildingUpgrade",
        |rules, (Handle(from), Handle(to)): (Handle, Handle)| {
            rules.military.building_upgrades.push((from, to));
            Ok(())
        },
    )?;
    global!(r, "sh_setSpeed", ship.speed: Num);
    r.def(
        "sh_addFrameCost",
        |rules, (Handle(good), Count(count)): (Handle, Count)| {
            rules.ship.frame_costs.push(Amount { good, count });
            Ok(())
        },
    )?;
    r.def(
        "sh_addCost",
        |rules, (Handle(good), Count(count)): (Handle, Count)| {
            rules.ship.costs.push(Amount { good, count });
            Ok(())
        },
    )?;
    global!(r, "sh_setConstructTime", ship.construct_time: Num);
    global!(r, "sh_setSize", ship.size: Int);
    global!(r, "sh_setStockLimit", ship.stock_limit: Count);

    // LUA-PROP-11
    creator!(r, "an_create", animals);
    setter!(r, "an_setName", animals.name: Text);
    setter!(r, "an_setGfxName", animals.gfx_name: Text);
    setter!(r, "an_setMovementSpeed", animals.movement_speed: Num);
    setter!(r, "an_setMoveDistance", animals.move_distance: Count);
    setter!(r, "an_setRange", animals.range: Count);
    setter!(r, "an_setAvoidBuildings", animals.avoid_buildings: Flag);
    setter!(r, "an_setIdleTimeFactor", animals.idle_time_factor: Num);
    creator!(r, "sp_create", streets);
    setter!(r, "sp_setName", streets.name: Text);
    setter!(r, "sp_setShortName", streets.short_name: Text);
    setter!(r, "sp_setMesh", streets.mesh: Text);
    setter!(r, "sp_setHeight", streets.height: Num);
    setter!(r, "sp_setLength", streets.length: Num);
    add_amount!(r, "sp_addCost", streets.costs);
    creator!(r, "sn_create", sounds);
    setter!(r, "sn_setName", sounds.name: Text);
    r.def(
        "sn_addFileName",
        |rules, (h, Text(path)): (Handle, Text)| {
            object(&mut rules.sounds, h)?.file_names.push(path);
            Ok(())
        },
    )?;
    setter!(r, "sn_setType", sounds.sound_type: enum SoundType);
    setter!(r, "sn_setLooping", sounds.looping: Flag);
    setter!(r, "sn_setVolume", sounds.volume: Int);
    setter!(r, "sn_setPriority", sounds.priority: Int);
    setter!(r, "sn_set3DSound", sounds.positioned: Flag);
    setter!(r, "sn_set3DMinDistance", sounds.min_distance_3d: Num);
    setter!(r, "sn_set3DMaxDistance", sounds.max_distance_3d: Num);
    setter!(r, "sn_set2DDistance", sounds.distance_2d: Num);
    setter!(r, "sn_setMusicTribe", sounds.music_tribe: enum Tribe);
    setter!(r, "sn_setPercussion", sounds.percussion: Flag);
    r.def("ai_create", |rules, (Text(number),): (Text,)| {
        create(&mut rules.ai_profiles, decimal_id(&number)?)
    })?;
    setter!(r, "ai_setName", ai_profiles.name: Text);
    setter!(r, "ai_setMilitaryMode", ai_profiles.military_mode: enum MilitaryMode);
    setter!(r, "ai_setAttackStrength", ai_profiles.attack_strength: Num);
    setter!(r, "ai_setInterceptingFactor", ai_profiles.intercepting_factor: Num);
    setter!(r, "ai_setStartSawmillCount", ai_profiles.start_sawmill_count: Count);
    setter!(r, "ai_setMaximumSmithies", ai_profiles.maximum_smithies: Count);
    setter!(r, "ai_setMaximumMints", ai_profiles.maximum_mints: Count);
    setter!(r, "ai_enableStreetOptimizing", ai_profiles.street_optimizing: Flag);
    setter!(r, "ai_enableRouteOptimizing", ai_profiles.route_optimizing: Flag);
    setter!(r, "ai_enableCatapults", ai_profiles.catapults: Flag);
    setter!(r, "ai_enableCoinArrangement", ai_profiles.coin_arrangement: Flag);
    Ok(())
}

/// Names of every registered `properties` function.
pub fn function_names() -> Vec<String> {
    let lua = Lua::new_with(StdLib::NONE, LuaOptions::default()).expect("Lua state");
    let state = Rc::new(RefCell::new(State::default()));
    let table = lua.create_table().expect("table");
    register_all(&Registry {
        lua: &lua,
        table: &table,
        state: &state,
    })
    .expect("registration");
    let mut names: Vec<String> = table
        .pairs::<String, Value>()
        .map(|pair| pair.expect("name").0)
        .collect();
    names.sort();
    names
}

/// Hook H6: the functions only a project's overlays have, in the table
/// `reborn`, made after the original scripts ran (they never see it, and
/// `properties` stays the original's). `reborn.bp_addInput(building, good,
/// count)` appends to the building type's own recipe (`Building::inputs`).
/// W12: `reborn.bp_setAiAlternative(building, good, stock_below, input,
/// input_at_least, most)` sets `Building::ai_alternative`. W20:
/// `reborn.bp_addRecipe(building, product)` starts a recipe of the type's
/// set (`Building::recipes`), `reborn.bp_addRecipeInput(building, good,
/// count)` appends to its last one. W15: `reborn.bp_setDeepening(building,
/// share_per_mille, most)` gives a mine type its deepening,
/// `reborn.bp_addDeepeningCost(building, good, count)` and
/// `reborn.bp_addDeepeningFood(building, food_per_product)` add to it. W16:
/// `reborn.cfg_setRecipeKeep(good, count)` sets `RuleSet::recipe_keep`. W19:
/// `reborn.cfg_setPaving(good, per_field, speed_percent)` sets
/// `RuleSet::paving`. `reborn.dist_addConsumer(good, building, weight)` adds
/// to `RuleSet::dist_consumers`. W25: `reborn.ai_setDeepen(food_at_least)`
/// (W25b: `, keep_waiting`, `false` tearing down a mine not deepened now),
/// `reborn.ai_setPaving(traffic_at_least, reserve)` and
/// `reborn.ai_setRecipeRule(building, index, good, above)` fill
/// `RuleSet::ai_project`.
fn install_project(lua: &Lua, state: &Shared) -> mlua::Result<()> {
    let reborn = strict_table(lua, "reborn")?;
    let r = Registry {
        lua,
        table: &reborn,
        state,
    };
    add_amount!(r, "bp_addInput", buildings.inputs);
    r.def(
        "bp_setAiAlternative",
        |rules,
         (
            h,
            Handle(good),
            Count(stock_below),
            Handle(input),
            Count(input_at_least),
            Count(most),
        ): (Handle, Handle, Count, Handle, Count, Count)| {
            object(&mut rules.buildings, h)?.ai_alternative = Some(AiAlternative {
                good,
                stock_below,
                input,
                input_at_least,
                most,
            });
            Ok(())
        },
    )?;
    r.def(
        "bp_addRecipe",
        |rules, (h, Handle(product)): (Handle, Handle)| {
            object(&mut rules.buildings, h)?.recipes.push(RecipeChoice {
                product,
                inputs: Vec::new(),
            });
            Ok(())
        },
    )?;
    r.def(
        "dist_addConsumer",
        |rules, (Handle(good), h, Count(weight)): (Handle, Handle, Count)| {
            if !rules.goods.contains(good) {
                return Err(format!("unknown good {good}"));
            }
            let Handle(building) = h;
            object(&mut rules.buildings, h)?;
            rules.dist_consumers.push((good, building, weight));
            Ok(())
        },
    )?;
    r.def(
        "cfg_setPaving",
        |rules, (Handle(good), Count(per_field), Count(speed_percent)): (Handle, Count, Count)| {
            if !rules.goods.contains(good) {
                return Err(format!("unknown good {good}"));
            }
            if speed_percent == 0 {
                return Err("the speed must be at least 1 percent".to_string());
            }
            rules.paving = Some((good, per_field, speed_percent));
            Ok(())
        },
    )?;
    r.def(
        "cfg_setRecipeKeep",
        |rules, (Handle(good), Count(count)): (Handle, Count)| {
            if !rules.goods.contains(good) {
                return Err(format!("unknown good {good}"));
            }
            rules.recipe_keep.insert(good, count);
            Ok(())
        },
    )?;
    r.def(
        "bp_setDeepening",
        |rules, (h, Count(share_per_mille), Count(most)): (Handle, Count, Count)| {
            object(&mut rules.buildings, h)?.deepening = Some(DeepeningRule {
                share_per_mille,
                most,
                ..DeepeningRule::default()
            });
            Ok(())
        },
    )?;
    r.def(
        "bp_addDeepeningCost",
        |rules, (h, Handle(good), Count(count)): (Handle, Handle, Count)| {
            object(&mut rules.buildings, h)?
                .deepening
                .as_mut()
                .ok_or_else(|| "no deepening: call reborn.bp_setDeepening first".to_string())?
                .costs
                .push(Amount { good, count });
            Ok(())
        },
    )?;
    r.def(
        "bp_addDeepeningFood",
        |rules, (h, Count(food)): (Handle, Count)| {
            object(&mut rules.buildings, h)?
                .deepening
                .as_mut()
                .ok_or_else(|| "no deepening: call reborn.bp_setDeepening first".to_string())?
                .food
                .push(food);
            Ok(())
        },
    )?;
    r.def(
        "bp_addRecipeInput",
        |rules, (h, Handle(good), Count(count)): (Handle, Handle, Count)| {
            object(&mut rules.buildings, h)?
                .recipes
                .last_mut()
                .ok_or_else(|| "no recipe: call reborn.bp_addRecipe first".to_string())?
                .inputs
                .push(Amount { good, count });
            Ok(())
        },
    )?;
    // W25: the computer players' use of the project's rules.
    r.def(
        "ai_setDeepen",
        |rules, (Count(food), keep_waiting): (Count, Option<Flag>)| {
            rules.ai_project.deepen_food = Some(food);
            // W25b: `false` tears down a mine not deepened now.
            rules.ai_project.deepen_tears_down = matches!(keep_waiting, Some(Flag(false)));
            Ok(())
        },
    )?;
    r.def(
        "ai_setPaving",
        |rules, (Count(traffic), Count(reserve)): (Count, Count)| {
            rules.ai_project.paving = Some((traffic, reserve));
            Ok(())
        },
    )?;
    r.def(
        "ai_setRecipeRule",
        |rules, (h, Count(index), Handle(good), Count(above)): (Handle, Count, Handle, Count)| {
            if !rules.goods.contains(good) {
                return Err(format!("unknown good {good}"));
            }
            let Handle(building) = h;
            object(&mut rules.buildings, h)?;
            rules.ai_project.recipes.push(crate::model::AiRecipeRule {
                building,
                index,
                good,
                above,
            });
            Ok(())
        },
    )?;
    lua.globals().raw_set("reborn", reborn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Scripts held in memory, by exact path.
    struct Memory(HashMap<&'static str, &'static str>);

    impl ScriptSource for Memory {
        fn read(&self, path: &str) -> Result<Vec<u8>, String> {
            self.0
                .get(path)
                .map(|code| code.as_bytes().to_vec())
                .ok_or_else(|| format!("no script {path}"))
        }
    }

    fn run_scripts(
        scripts: &[(&'static str, &'static str)],
    ) -> Result<(RuleSet, LoadReport), ScriptError> {
        let source = Memory(scripts.iter().copied().collect());
        run(Rc::new(source), scripts[0].0)
    }

    fn run_one(code: &'static str) -> Result<(RuleSet, LoadReport), ScriptError> {
        run_scripts(&[("main.lua", code)])
    }

    fn error_of(code: &'static str) -> String {
        run_one(code).unwrap_err().0
    }

    /// An overlay runs after the originals, sees their globals and changes
    /// the rules through the same functions.
    // spec: none (rule overlays of our own)
    #[test]
    fn overlay_changes_a_building_cost() {
        let source = Memory(
            [(
                "main.lua",
                "good_stone = properties.gp_create(\"00000002\")
                 building_hut = properties.bp_create(\"00000003\")
                 properties.bp_addCost(building_hut, good_stone, 2)",
            )]
            .into_iter()
            .collect(),
        );
        let overlay = Overlay {
            name: "cheaper".into(),
            code: b"properties.bp_addCost(building_hut, good_stone, 1)".to_vec(),
        };
        let (plain, _) = run(Rc::new(Memory(source.0.clone())), "main.lua").unwrap();
        let (rules, report) =
            run_with_overlays(Rc::new(source), "main.lua", std::slice::from_ref(&overlay)).unwrap();
        let count = |r: &RuleSet| -> u32 {
            r.buildings
                .iter()
                .flat_map(|b| &b.costs)
                .map(|a| a.count)
                .sum()
        };
        assert_eq!(count(&plain) + 1, count(&rules));
        assert_eq!(report.scripts, ["main.lua", "overlay:cheaper"]);
        // An overlay error names the overlay.
        let bad = Overlay {
            name: "bad".into(),
            code: b"properties.no_such_function()".to_vec(),
        };
        let err =
            run_with_overlays(Rc::new(Memory(source_clone())), "main.lua", &[bad]).unwrap_err();
        assert!(err.0.contains("no_such_function"), "{}", err.0);
    }

    /// Hook H6: an overlay creates a building type of its own with
    /// `properties.bp_create` and gives it a recipe with
    /// `reborn.bp_addInput`, without touching its product's cost list; the
    /// original scripts never see `reborn`, and without overlays there is no
    /// such table.
    // spec: none (hook H6, docs/extending.md "New building types")
    #[test]
    fn an_overlay_adds_a_building_type_with_its_own_recipe() {
        let main = "good_wood = properties.gp_create(\"00000006\")
                    good_stone = properties.gp_create(\"00000002\")
                    good_coal = properties.gp_create(\"00000007\")
                    properties.gp_addCost(good_coal, good_stone, 1)
                    building_hut = properties.bp_create(\"00000003\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let burner = Overlay {
            name: "charcoal".into(),
            code: b"charcoalBurner = properties.bp_create(\"00000040\")
                    properties.bp_setGoods(charcoalBurner, good_coal, 1)
                    reborn.bp_addInput(charcoalBurner, good_wood, 2)"
                .to_vec(),
        };
        let (rules, report) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&burner)).unwrap();
        let b = rules.buildings.get(0x40).expect("the new type");
        assert_eq!(rules.names.get("charcoalBurner"), Some(&0x40));
        assert_eq!(b.inputs, [Amount { good: 6, count: 2 }]);
        assert_eq!(report.call_count("bp_addInput"), 1);
        // Coal's own cost list is the original's.
        assert_eq!(
            rules.goods.get(7).unwrap().costs,
            [Amount { good: 2, count: 1 }]
        );
        assert!(rules.dangling_references().is_empty());
        // The originals never see `reborn`.
        let original = Memory(
            [("main.lua", "reborn.bp_addInput(1, 2, 3)")]
                .into_iter()
                .collect(),
        );
        let err = run_with_overlays(Rc::new(original), "main.lua", std::slice::from_ref(&burner))
            .unwrap_err();
        assert!(err.0.contains("reborn"), "{}", err.0);
        // Without overlays, no type has a recipe of its own.
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert!(plain.buildings.iter().all(|b| b.inputs.is_empty()));
    }

    /// Hook W20: an overlay gives a type a set of recipes, each a product
    /// and its inputs, with `reborn.bp_addRecipe` and
    /// `reborn.bp_addRecipeInput`; an input before any recipe is an error;
    /// without overlays no type has a set.
    // spec: none (hook W20, docs/extending.md "Recipes a player chooses")
    #[test]
    fn an_overlay_gives_a_type_recipes_to_choose_from() {
        let main = "good_wood = properties.gp_create(\"00000006\")
                    good_stone = properties.gp_create(\"00000002\")
                    good_coal = properties.gp_create(\"00000007\")
                    good_fish = properties.gp_create(\"00000008\")
                    building_hut = properties.bp_create(\"00000003\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let post = Overlay {
            name: "trading-post".into(),
            code: b"tradingPost = properties.bp_create(\"00000041\")
                    properties.bp_setGoods(tradingPost, good_coal, 1)
                    reborn.bp_addRecipe(tradingPost, good_coal)
                    reborn.bp_addRecipeInput(tradingPost, good_fish, 2)
                    reborn.bp_addRecipeInput(tradingPost, good_wood, 1)
                    reborn.bp_addRecipe(tradingPost, good_stone)
                    reborn.bp_addRecipeInput(tradingPost, good_wood, 4)"
                .to_vec(),
        };
        let (rules, report) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&post)).unwrap();
        let b = rules.buildings.get(0x41).expect("the new type");
        assert_eq!(
            b.recipes,
            [
                RecipeChoice {
                    product: 7,
                    inputs: vec![Amount { good: 8, count: 2 }, Amount { good: 6, count: 1 }],
                },
                RecipeChoice {
                    product: 2,
                    inputs: vec![Amount { good: 6, count: 4 }],
                },
            ]
        );
        assert_eq!(report.call_count("bp_addRecipe"), 2);
        assert!(rules.dangling_references().is_empty());
        let early = Overlay {
            name: "early".into(),
            code: b"reborn.bp_addRecipeInput(building_hut, good_wood, 1)".to_vec(),
        };
        let err = run_with_overlays(source(), "main.lua", &[early]).unwrap_err();
        assert!(err.0.contains("bp_addRecipe first"), "{}", err.0);
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert!(plain.buildings.iter().all(|b| b.recipes.is_empty()));
    }

    /// Hook W15: an overlay gives a mine type a deepening with
    /// `reborn.bp_setDeepening`, its costs and foods with the companions;
    /// a cost before the deepening is an error; without overlays no type
    /// has one.
    // spec: none (hook W15, docs/extending.md "Deepening a mine")
    #[test]
    fn an_overlay_gives_a_mine_a_deepening() {
        let main = "good_wood = properties.gp_create(\"00000006\")
                    good_stone = properties.gp_create(\"00000002\")
                    good_plank = properties.gp_create(\"00000001\")
                    building_coalmine = properties.bp_create(\"00000003\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let deep = Overlay {
            name: "deep-mines".into(),
            code: b"reborn.bp_setDeepening(building_coalmine, 250, 2)
                    reborn.bp_addDeepeningCost(building_coalmine, good_plank, 4)
                    reborn.bp_addDeepeningCost(building_coalmine, good_stone, 2)
                    reborn.bp_addDeepeningFood(building_coalmine, 2)
                    reborn.bp_addDeepeningFood(building_coalmine, 3)"
                .to_vec(),
        };
        let (rules, report) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&deep)).unwrap();
        assert_eq!(
            rules.buildings.get(3).unwrap().deepening,
            Some(DeepeningRule {
                share_per_mille: 250,
                most: 2,
                costs: vec![Amount { good: 1, count: 4 }, Amount { good: 2, count: 2 }],
                food: vec![2, 3],
            })
        );
        assert_eq!(report.call_count("bp_setDeepening"), 1);
        assert!(rules.dangling_references().is_empty());
        let early = Overlay {
            name: "early".into(),
            code: b"reborn.bp_addDeepeningCost(building_coalmine, good_wood, 1)".to_vec(),
        };
        let err = run_with_overlays(source(), "main.lua", &[early]).unwrap_err();
        assert!(err.0.contains("bp_setDeepening first"), "{}", err.0);
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert!(plain.buildings.iter().all(|b| b.deepening.is_none()));
    }

    /// Hook W16: a recipe that takes a group good makes the group a recipe
    /// group with its members in good order; `reborn.cfg_setRecipeKeep` on
    /// the group keeps that many of each member, on a good that many of it.
    // spec: none (hook W16, docs/extending.md "A group good in a recipe")
    #[test]
    fn a_recipe_group_and_its_kept_stock() {
        let main = "good_tools = properties.gp_create(\"00000032\")
                    good_saw = properties.gp_create(\"0000003E\")
                    properties.gp_setGroupGoodId(good_saw, good_tools)
                    good_axe = properties.gp_create(\"0000003D\")
                    properties.gp_setGroupGoodId(good_axe, good_tools)
                    good_coal = properties.gp_create(\"00000007\")
                    good_iron = properties.gp_create(\"00000009\")
                    building_smelter = properties.bp_create(\"00000003\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let melt = Overlay {
            name: "tool-recycling".into(),
            code: b"reborn.bp_addRecipe(building_smelter, good_iron)
                    reborn.bp_addRecipeInput(building_smelter, good_tools, 2)
                    reborn.bp_addRecipeInput(building_smelter, good_coal, 1)
                    reborn.cfg_setRecipeKeep(good_tools, 2)
                    reborn.cfg_setRecipeKeep(good_saw, 1)"
                .to_vec(),
        };
        let (rules, report) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&melt)).unwrap();
        assert_eq!(report.call_count("cfg_setRecipeKeep"), 2);
        assert_eq!(rules.recipe_keep.get(&0x32), Some(&2));
        let groups = rules.recipe_groups();
        assert_eq!(groups.get(&0x32), Some(&vec![0x3D, 0x3E]));
        assert_eq!(groups.len(), 1);
        let keep = rules.recipe_keep_by_member();
        assert_eq!(keep.get(&0x3D), Some(&2));
        assert_eq!(keep.get(&0x3E), Some(&1));
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert!(plain.recipe_groups().is_empty() && plain.recipe_keep_by_member().is_empty());
    }

    /// Hook W19: `reborn.cfg_setPaving` sets the paving rule; a speed of
    /// 0 percent is an error; without overlays there is none.
    // spec: none (hook W19, docs/extending.md "Paved roads")
    #[test]
    fn an_overlay_sets_the_paving_rule() {
        let main = "good_stone = properties.gp_create(\"00000002\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let paving = Overlay {
            name: "paved-roads".into(),
            code: b"reborn.cfg_setPaving(good_stone, 1, 130)".to_vec(),
        };
        let (rules, _) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&paving)).unwrap();
        assert_eq!(rules.paving, Some((2, 1, 130)));
        let still = Overlay {
            name: "still".into(),
            code: b"reborn.cfg_setPaving(good_stone, 1, 0)".to_vec(),
        };
        let err = run_with_overlays(source(), "main.lua", &[still]).unwrap_err();
        assert!(err.0.contains("percent"), "{}", err.0);
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert_eq!(plain.paving, None);
    }

    /// Hook W25: the three `reborn.ai_set*` calls fill the computer
    /// players' rules; an unknown good is an error; without overlays there
    /// are none. W25b: `ai_setDeepen`'s second value.
    // spec: none (hooks W25 and W25b, docs/extending.md "The computer players and a project's rules")
    #[test]
    fn an_overlay_sets_the_computer_players_rules() {
        let main = "good_stone = properties.gp_create(\"00000002\")
                    good_tools = properties.gp_create(\"00000005\")
                    building_smelter = properties.bp_create(\"00000009\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let ai = Overlay {
            name: "ai".into(),
            code: b"reborn.ai_setDeepen(6)
                    reborn.ai_setPaving(3, 10)
                    reborn.ai_setRecipeRule(building_smelter, 1, good_tools, 40)"
                .to_vec(),
        };
        let (rules, report) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&ai)).unwrap();
        assert_eq!(rules.ai_project.deepen_food, Some(6));
        assert!(
            !rules.ai_project.deepen_tears_down,
            "kept for later, as W25"
        );
        assert_eq!(rules.ai_project.paving, Some((3, 10)));
        assert_eq!(
            rules.ai_project.recipes,
            [crate::model::AiRecipeRule {
                building: 9,
                index: 1,
                good: 5,
                above: 40
            }]
        );
        assert!(rules.ai_project.any());
        assert_eq!(report.call_count("ai_setPaving"), 1);
        let wrong = Overlay {
            name: "wrong".into(),
            code: b"reborn.ai_setRecipeRule(building_smelter, 1, 99, 40)".to_vec(),
        };
        let err = run_with_overlays(source(), "main.lua", &[wrong]).unwrap_err();
        assert!(err.0.contains("unknown good"), "{}", err.0);
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert!(!plain.ai_project.any());
        // W25b: a second value `false` tears down a mine not deepened now.
        for (code, tears_down) in [
            (&b"reborn.ai_setDeepen(6, false)"[..], true),
            (&b"reborn.ai_setDeepen(6, true)"[..], false),
        ] {
            let deepen = Overlay {
                name: "deepen".into(),
                code: code.to_vec(),
            };
            let (rules, _) = run_with_overlays(source(), "main.lua", &[deepen]).unwrap();
            assert_eq!(rules.ai_project.deepen_food, Some(6));
            assert_eq!(rules.ai_project.deepen_tears_down, tears_down);
        }
    }

    /// Hook W12: the types an overlay creates are the project's, an
    /// original type it changes is not; `reborn.bp_setAiAlternative` gives a
    /// type its alternative; without overlays there are none of either.
    // spec: none (hook W12, docs/extending.md "The computer players and a project's type")
    #[test]
    fn an_overlay_type_is_a_projects_and_may_stand_in_for_a_good() {
        let main = "good_wood = properties.gp_create(\"00000006\")
                    good_coal = properties.gp_create(\"00000007\")
                    building_hut = properties.bp_create(\"00000003\")";
        let source = || Rc::new(Memory([("main.lua", main)].into_iter().collect()));
        let burner = Overlay {
            name: "charcoal".into(),
            code: b"charcoalBurner = properties.bp_create(\"00000040\")
                    properties.bp_setGoods(charcoalBurner, good_coal, 1)
                    properties.bp_setSort(building_hut, 2)
                    reborn.bp_setAiAlternative(charcoalBurner, good_coal, 4, good_wood, 8, 2)"
                .to_vec(),
        };
        let (rules, report) =
            run_with_overlays(source(), "main.lua", std::slice::from_ref(&burner)).unwrap();
        assert_eq!(
            rules.project_buildings.iter().copied().collect::<Vec<_>>(),
            [0x40]
        );
        assert_eq!(
            rules.buildings.get(0x40).unwrap().ai_alternative,
            Some(AiAlternative {
                good: 7,
                stock_below: 4,
                input: 6,
                input_at_least: 8,
                most: 2,
            })
        );
        assert_eq!(rules.buildings.get(3).unwrap().ai_alternative, None);
        assert_eq!(report.call_count("bp_setAiAlternative"), 1);
        assert!(rules.dangling_references().is_empty());
        let (plain, _) = run(source(), "main.lua").unwrap();
        assert!(plain.project_buildings.is_empty());
        assert!(plain.buildings.iter().all(|b| b.ai_alternative.is_none()));
    }

    fn source_clone() -> HashMap<&'static str, &'static str> {
        [("main.lua", "x = 1")].into_iter().collect()
    }

    // spec: LUA-PROP-4, LUA-PROP-5, LUA-PROP-6, LUA-PROP-7, LUA-PROP-8, LUA-PROP-9, LUA-PROP-10, LUA-PROP-11
    #[test]
    fn spec_tables_are_all_registered() {
        let names = function_names();
        // The rows of the spec's tables (Q-030).
        assert_eq!(names.len(), 148);
        for name in [
            "cfg_setGameSpeed",
            "bp_setAlternativeGoods",
            "si_addSignDoodadId",
            "ai_enableCoinArrangement",
        ] {
            assert!(names.iter().any(|n| n == name), "{name}");
        }
    }

    // LUA-PROP-2.1, LUA-PROP-2.2: a create returns the id as the handle.
    #[test]
    fn create_and_set() {
        let (rules, report) = run_one(
            "local g = properties.gp_create(\"2C50533E\")
             properties.gp_setName(g, \"Woodcutter\")
             properties.gp_setType(g, 1)
             properties.gp_setRestTime(g, 12.5)
             local b = properties.bp_create(\"5a926fa3\")
             properties.bp_setName(b, \"WoodcutterHut\")
             properties.bp_setWorker(b, g)
             properties.bp_setProductionTime(b, 55)
             properties.bp_addCost(b, g, 2)
             properties.bp_addCost(b, g, 3)",
        )
        .unwrap();
        let good = rules.goods.get(0x2C50_533E).unwrap();
        assert_eq!(good.name.as_deref(), Some("Woodcutter"));
        assert_eq!(good.good_type, Some(GoodType::Settler));
        assert_eq!(good.rest_time, Some(12.5));
        let building = rules.building_named("WoodcutterHut").unwrap();
        assert_eq!(building.id, 0x5A92_6FA3);
        assert_eq!(building.worker, Some(0x2C50_533E));
        assert_eq!(building.production_time, Some(55.0));
        // LUA-PROP-2.5: add* appends.
        assert_eq!(building.costs.len(), 2);
        assert_eq!(report.first_call, Some("gp_create"));
        assert_eq!(report.call_count("bp_addCost"), 2);
        assert!(rules.dangling_references().is_empty());
    }

    // LUA-PROP-2.1: ids may have fewer than 8 digits.
    #[test]
    fn short_and_prefixed_ids() {
        let (rules, _) = run_one(
            "properties.pa_create(\"13374e4\")
             properties.sn_create(\"0xcea2bcc3\")",
        )
        .unwrap();
        assert!(rules.patterns.contains(0x0133_74E4));
        // Q-029
        assert!(rules.sounds.contains(0xCEA2_BCC3));
        for bad in ["\"\"", "\"123456789\"", "\"0x\"", "\"xyz\"", "12"] {
            let code: &'static str =
                Box::leak(format!("properties.pa_create({bad})").into_boxed_str());
            assert!(error_of(code).contains("pa_create"), "{bad}");
        }
    }

    // LUA-PROP-2.3
    #[test]
    fn ai_profiles_are_numbered_in_decimal() {
        let (rules, _) =
            run_one("local a = properties.ai_create(\"12\") properties.ai_setName(a, \"strong\")")
                .unwrap();
        assert_eq!(
            rules.ai_profiles.get(12).unwrap().name.as_deref(),
            Some("strong")
        );
    }

    // LUA-PROP-2.5: the last call wins.
    #[test]
    fn last_setter_call_wins() {
        let (rules, _) = run_one(
            "local b = properties.bp_create(\"1\")
             properties.bp_setGoods(b, 7, 1)
             properties.bp_setGoods(b, 8, 2)
             properties.bp_setConstructionPosition(b, 0, 0, 1.0, 2.0)
             properties.bp_setConstructionPosition(b, 0, 0, 3.0, 4.0)
             properties.bp_setConstructionPosition(b, 1, 0, 5.0, 6.0)
             properties.bp_setDoorOffsetDirection(b, 2, 3)
             properties.bp_setDoorOffsetDirection(b, 2, 5)",
        )
        .unwrap();
        let building = rules.buildings.get(1).unwrap();
        assert_eq!(building.goods, Some(Amount { good: 8, count: 2 }));
        assert_eq!(building.construction_positions.len(), 2);
        // Replacing a spot moves it to the end; tribe 1's spot came later.
        assert_eq!(building.construction_positions[0].x, 3.0);
        assert_eq!(building.construction_positions[1].x, 5.0);
        assert_eq!(
            building.door_directions,
            [(Tribe::Chinese, DoorDirection::NorthEast)]
        );
    }

    // LUA-PROP-2.2: -1 means "no object".
    #[test]
    fn minus_one_is_no_object() {
        let (rules, _) = run_one(
            "local b = properties.bp_create(\"1\")
             properties.bp_setWorker(b, 5)
             properties.bp_setWorker(b, -1)
             local d = properties.dd_create(\"2\")
             properties.si_addSignDoodadId(-1, 0, d)",
        )
        .unwrap();
        assert_eq!(rules.buildings.get(1).unwrap().worker, None);
        assert_eq!(
            rules.signs,
            [Sign {
                good: None,
                threshold: 0,
                doodad: 2
            }]
        );
    }

    // LUA-PROP-6: bp_setAlternativeGoods is bp_setSecondGoods under another name.
    #[test]
    fn alternative_goods_are_second_goods() {
        let (rules, _) = run_one(
            "local b = properties.bp_create(\"1\")
             properties.bp_setAlternativeGoods(b, 9, 1)",
        )
        .unwrap();
        assert_eq!(
            rules.buildings.get(1).unwrap().second_goods,
            Some(Amount { good: 9, count: 1 })
        );
    }

    // spec: LUA-PROP-1.6, LUA-PROP-2.2
    #[test]
    fn global_records() {
        let (rules, _) = run_one(
            "local d = properties.dd_create(\"5\")
             properties.cfg_setSettlerSpeed(0.045)
             properties.cfg_setFlagPackageLimit(6)
             properties.cfg_setSettlerSkeletonDoodad(d)
             properties.mi_setSoldierMaxHealth(4, 100)
             properties.sh_addCost(3, 2)",
        )
        .unwrap();
        assert_eq!(rules.settings.settler_speed, Some(0.045));
        assert_eq!(rules.settings.flag_package_limit, Some(6));
        assert_eq!(rules.settings.settler_skeleton_doodad, Some(5));
        assert_eq!(rules.military.soldier_max_health[4], Some(100));
        assert_eq!(rules.ship.costs, [Amount { good: 3, count: 2 }]);
    }

    // spec: LUA-PROP-1.1
    #[test]
    fn unknown_functions_are_errors_naming_them() {
        assert!(error_of("properties.xx_doSomething(1)").contains("properties.xx_doSomething"));
        assert!(error_of("Default.Explode()").contains("Default.Explode"));
        // Even when only looked up, not called.
        assert!(error_of("local f = properties.bp_setColour").contains("properties.bp_setColour"));
    }

    // spec: LUA-PROP-2.4
    #[test]
    fn wrong_arguments_are_errors_naming_the_function() {
        for (code, needle) in [
            ("properties.gp_setName(1, \"x\")", "unknown handle"),
            (
                "local g = properties.gp_create(\"1\") properties.gp_setName(g)",
                "gp_setName",
            ),
            (
                "local g = properties.gp_create(\"1\") properties.gp_setName(g, 5)",
                "gp_setName",
            ),
            (
                "local g = properties.gp_create(\"1\") properties.gp_setName(g, \"a\", \"b\")",
                "at most 2",
            ),
            (
                "local g = properties.gp_create(\"1\") properties.gp_setType(g, 7)",
                "GoodType",
            ),
            (
                "local g = properties.gp_create(\"1\") properties.gp_setPlantable(g, 1)",
                "boolean",
            ),
            (
                "local b = properties.bp_create(\"1\") properties.bp_setStockLimit(b, -2)",
                "count",
            ),
            (
                "local b = properties.bp_create(\"1\") properties.bp_setStockLimit(b, 1.5)",
                "integer",
            ),
            (
                "properties.gp_create(\"1\") properties.gp_create(\"01\")",
                "already exists",
            ),
            ("properties.mi_setSoldierMaxHealth(5, 1)", "soldier rank"),
            (
                "local p = properties.pa_create(\"1\") properties.pa_setMiniMapColor(p, 1, 2, 300)",
                "colour",
            ),
            (
                "local p = properties.pa_create(\"1\") properties.pa_setMiniMapColor(p, 1, 2, nil)",
                "pa_setMiniMapColor",
            ),
        ] {
            let leaked: &'static str = Box::leak(code.to_owned().into_boxed_str());
            let error = error_of(leaked);
            assert!(error.contains(needle), "{code}: {error}");
        }
    }

    // LUA-PROP-1.1: no standard library.
    #[test]
    fn no_standard_library() {
        assert!(run_one("print(1)").is_err());
        assert!(run_one("local x = string.len(\"a\")").is_err());
        assert!(run_one("dofile(\"x.lua\")").is_err());
        assert!(run_one("local x = _G").is_ok());
        assert!(run_one("local t = {} t.x = 1 if t.x ~= 1 then error() end").is_ok());
    }

    // LUA-PROP-1.3, LUA-PROP-1.5: scripts share one global environment, in order.
    #[test]
    fn call_script_shares_globals_in_order() {
        let (rules, report) = run_scripts(&[
            (
                "data.lua",
                "Default.CallScript(\"a.lua\") Default.CallScript(\"b.lua\")",
            ),
            ("a.lua", "WOOD = properties.gp_create(\"10\")"),
            (
                "b.lua",
                "properties.gp_setName(WOOD, \"Wood\") Default.WriteToLog(\"done\")",
            ),
        ])
        .unwrap();
        assert_eq!(rules.good_named("Wood").map(|g| g.id), Some(0x10));
        assert_eq!(report.scripts, ["data.lua", "a.lua", "b.lua"]);
        assert_eq!(report.log, ["done"]);
    }

    // spec: LUA-PROP-1.3
    #[test]
    fn errors_name_the_script() {
        let error = run_scripts(&[
            ("data.lua", "Default.CallScript(\"bad.lua\")"),
            ("bad.lua", "\n\nproperties.nope()"),
        ])
        .unwrap_err()
        .0;
        assert!(error.contains("bad.lua:3"), "{error}");
        assert!(
            run_scripts(&[("data.lua", "Default.CallScript(\"missing.lua\")")])
                .unwrap_err()
                .0
                .contains("missing.lua")
        );
    }

    // LUA-PROP-1.1: Latin-1 bytes pass through as characters.
    #[test]
    fn latin1_strings() {
        struct Bytes;
        impl ScriptSource for Bytes {
            fn read(&self, _: &str) -> Result<Vec<u8>, String> {
                let mut code = b"-- caf\xE9\nlocal g = properties.gp_create(\"1\") properties.gp_setName(g, \"\xC4pfel\")".to_vec();
                code.push(b'\n');
                Ok(code)
            }
        }
        let (rules, _) = run(Rc::new(Bytes), "x.lua").unwrap();
        assert_eq!(rules.goods.get(1).unwrap().name.as_deref(), Some("Äpfel"));
    }

    // spec: LUA-PROP-2.2
    #[test]
    fn dangling_references_are_reported() {
        let (rules, _) = run_one(
            "local b = properties.bp_create(\"1\")
             properties.bp_setWorker(b, 99)",
        )
        .unwrap();
        let dangling = rules.dangling_references();
        assert_eq!(dangling.len(), 1);
        assert!(dangling[0].contains("00000063"), "{dangling:?}");
    }
}
