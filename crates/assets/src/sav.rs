//! Save games of the original (`specs/FMT-savegame.md`): the map part is
//! the map format of `s2m` with the live values (FMT-SAV-2); the game block
//! after it (FMT-SAV-3) is read here, keeping what a game can be rebuilt
//! from and checking the rest only for its layout.

use crate::rc00;
use crate::s2m::{self, FieldPos, Map, Reader, S2mError};

/// FMT-S2M-1.5: the "none" object id.
pub const NO_ID: u64 = u64::MAX;

/// FMT-SAV-3.3: a player of `PlayerSystem`.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedPlayer {
    pub index: u32,
    pub active: bool,
    pub name: String,
    /// FMT-SAV-3.3: the player's two global stocks (good id, amount).
    pub stocks: Vec<Vec<(u32, u32)>>,
}

/// FMT-SAV-3.4a, 3.4b: a building of `VillageSystem`.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedBuilding {
    pub building_type: u32,
    pub owner: u32,
    pub id: u64,
    pub field: FieldPos,
    /// `VillageDepot`: its two stocks. The spec calls the first the wares
    /// held; in both samples the headquarters' wares are in the second and
    /// the first is empty (Q-204).
    pub held: Vec<(u32, u32)>,
    pub stock: Vec<(u32, u32)>,
    pub workers: Vec<u64>,
    /// `VillageConstruction`: the remaining work of the two construction
    /// parts (0, 0 when finished) and the materials delivered.
    pub construction_left: [f32; 2],
    pub delivered: Vec<(u32, u32)>,
    /// `VillageProduction`: the production switch.
    pub production_on: bool,
    /// `Blocking`: the size class (0 small .. 4 harbour).
    pub size_class: u32,
    pub door_flag: u64,
    /// `VillageBulldozing`: the levelling progress (1 = done).
    pub levelled: f32,
    /// `Village Military`: the soldiers inside.
    pub soldiers: Vec<u64>,
    pub harbour: bool,
    /// `VillageUpgrade`: the upgrade running, if one is.
    pub upgrade: Option<SavedUpgrade>,
    /// FMT-SAV-3.4b 1: the worker's reserved outdoor target (a deposit, tree
    /// or animal, by its object id); none while nothing is reserved.
    pub reserved: Option<u64>,
}

/// FMT-SAV-3.4b `VillageUpgrade` while running (Q-259): the target type, its
/// own `VillageConstruction` and `VillageBulldozing`.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedUpgrade {
    pub target: u32,
    pub construction_left: [f32; 2],
    pub delivered: Vec<(u32, u32)>,
    pub levelled: f32,
}

/// FMT-SAV-3.5: the six settler lists, in their order.
pub const SETTLER_LISTS: [&str; 6] = [
    "workers",
    "constructors",
    "carriers",
    "bulldozers",
    "soldiers",
    "specialists",
];

/// FMT-SAV-3.5: a settler: its owner and own id.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedSettler {
    /// The list's `u32`: the owner's player index, for every kind (FMT-SAV-3.5).
    pub owner: u32,
    pub id: u64,
    /// FMT-SAV-3.5a: the profession id, a good id of `Jobs.lua`; a soldier's
    /// rank (3.5c).
    pub profession: u32,
    /// The carried ware of a carrier (FMT-SAV-3.5d).
    pub carried: Option<u32>,
    /// FMT-SAV-3.5c: a soldier's `SettlersLivePoints`.
    pub live_points: Option<LivePoints>,
    /// FMT-SAV-3.5c, 3.7a: a soldier's links and the steps of his fight
    /// objects.
    pub soldier: Option<SavedSoldier>,
    /// FMT-SAV-3.5h: where it stands and its step.
    pub movement: SavedMovement,
    /// FMT-SAV-3.5a (2026-11-09, Q-257): its `SettlersAnimation`.
    pub animation: SavedAnimation,
    /// FMT-SAV-3.5a: the current state number (3 going home, 4 going to
    /// work, 32 a constructor at his site, 33 .. 38 a worker's job states).
    pub state: u32,
    /// FMT-SAV-3.5a: the `ref` after the animation: the package it has
    /// claimed or carries.
    pub package: Option<u64>,
    /// FMT-SAV-3.5b, 3.5e: a worker's building (his first `ref`), a
    /// constructor's or bulldozer's site (its `ref`).
    pub home: Option<u64>,
    /// FMT-SAV-3.5a (e, Q-1033): a constructor's work state: c, the
    /// construction positions ordered so far in the running unit; r, the
    /// ticks left of the 55-tick step; and the step (13 approaching, 10
    /// working).
    pub site_work: Option<[u32; 3]>,
    /// FMT-SAV-3.5a (e, corrected 2026-11-09 c, Q-1037): the work state's
    /// first `u32`: while he approaches the site (step 13) the approach's leg,
    /// 1 on the way out to the far point, 2 on the way back to the flag
    /// field's sample (a working constructor keeps 2).
    pub site_leg: Option<u32>,
    /// FMT-SAV-3.5a (b, 2026-11-09 c), 3.5b: a worker's hunter state (the
    /// last of the twelve): the walk to the animal's step (0 the free walk to
    /// its walk goal, 1 the lattice walk to it), the wait's countdown and the
    /// hunt step (7 waiting inside, 8 walking out, 9 walking to the animal,
    /// 10 working at it).
    pub hunt: Option<SavedHunt>,
    /// FMT-SAV-3.5a (b, 2026-11-11), 3.5b: a worker's harvest state (the
    /// second of the twelve; the farmer's harvest step): the walk out's leg
    /// (1 the door step onto the farm's field, 0 the walk to the flag), the
    /// wait's countdown and the step (7 waiting inside for a target, 8
    /// walking out, 9 walking to the target, 10 working).
    pub harvest: Option<SavedHunt>,
    /// FMT-SAV-3.5a (c), TRN-FREE-6.4 step 4 (Q-1032 c answered): the
    /// going-home record's wander failure count, the `u32` before the
    /// record's own step (the third of the common part's 4 x `u32`; 15 ..
    /// 22 for the beaten player of the 2023 sample, 0 for a settler not
    /// wandering).
    pub home_failures: u32,
}

/// FMT-SAV-3.5c (E-sav-016, E-sav-017): a soldier's four links and the
/// steps of the fight objects it carries: the walk to the target (22 done),
/// the attack object (27, 27 in a duel; 14, 27 walking to a new waiting
/// field; 0, 0 not yet paired), the defend object (27, 27 defending at
/// the door), the walk out and the return (0 in the samples); `defending`
/// is the `bool` after the defended building's `ref`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SavedSoldier {
    pub home: Option<u64>,
    pub attack_target: Option<u64>,
    pub opponent: Option<u64>,
    pub defended: Option<u64>,
    pub defending: bool,
    pub walk_to_target: u32,
    pub attack: [u32; 2],
    pub defend: [u32; 2],
    pub walk_out: [u32; 2],
    pub back: [u32; 2],
}

/// FMT-SAV-3.5a (2026-11-09, Q-257): a settler's animation record: the ticks
/// left of the running request (down by 1 a tick), the total of the last
/// request (kept after it ends) and its code (0 none, 2 put down, 3 pick
/// up, 4 / 5 / 6 work loops 0 / 1 / 2, 7 hit).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SavedAnimation {
    pub left: f32,
    pub total: f32,
    pub code: u32,
}

/// FMT-SAV-3.5a (b, 2026-11-09 c): a worker's hunter state; (2026-11-11)
/// the harvest state has the same fields, its first `u32` the walk out's
/// leg in `walk_step`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedHunt {
    pub walk_step: u32,
    pub countdown: f32,
    pub step: u32,
}

/// FMT-SAV-3.5h (Q-257 a): a settler's place: its current field (the
/// closing field cursor), `Movement Rough`'s four cursors (the step's start
/// and goal fields, the current field, the walk's target), the step's
/// progress (-1 when no step runs, else 0 to 1), the walk's stop-short count
/// and its free path.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavedMovement {
    pub field: Option<FieldPos>,
    pub rough: [Option<FieldPos>; 4],
    pub progress: f32,
    /// FMT-SAV-3.5h (corrected 2026-11-09 c): the `u32` after the
    /// interpolator, the walk's stop-short count k of TRN-FREE-4.2 (1 for a
    /// worker walking out to his object's field, 2 on the hunter's lattice
    /// walk, else 0).
    pub stop_short: u32,
    /// FMT-SAV-3.5h `Path`'s kind: 1 a free path, 2 a street path, 0 none.
    pub path_kind: u32,
    /// FMT-SAV-3.5a (b, 2026-11-09 c): the free path's fields while its
    /// `Movement Path Base` is active, from the walk's target back to its
    /// start (empty otherwise), and the index of the field the running step
    /// goes to (the one after it in the list is where the step started).
    pub free_path: Vec<FieldPos>,
    pub free_index: u32,
    /// FMT-SAV-3.5a (b, 2026-11-09, Q-257): the fine flag: set while the
    /// settler is on the fine lattice (his field then follows the fine
    /// cursor).
    pub fine_on: bool,
    /// The four fine cursors (height samples), as the rough ones: the step's
    /// start, its goal, the current point and the walk's target.
    pub fine: [Option<(u32, u32)>; 4],
    /// `Movement Fine Path`: the walk's points from the target back to the
    /// start, and the index of the point the running step goes to.
    pub fine_path: Vec<(u32, u32)>,
    pub fine_index: u32,
    /// The closing fine cursor: where the settler stands.
    pub fine_at: Option<(u32, u32)>,
    /// The interpolator's progress per tick (0.09 on the fine lattice).
    pub per_tick: f32,
}

/// FMT-SAV-3.5c: `SettlersLivePoints`: health on the rank's own scale, 1 up to
/// its maximum, and the counter that heals a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivePoints {
    pub health: f32,
    pub healing: u32,
}

/// FMT-SAV-3.6: a ware on its way, its fields in save order.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedPackage {
    pub id: u64,
    /// Set exactly when a destination is set (an order or a warehouse
    /// took it).
    pub has_target: bool,
    pub has_next: bool,
    /// The destination building.
    pub destination: u64,
    /// The carrier who claimed it at its flag, or who carries it now.
    pub carrier: u64,
    /// The flag in whose ware list it is: where it lies, or, while carried,
    /// the flag it is carried to (its place there).
    pub flag: u64,
    pub good: u32,
    /// TRN-CARRY-5: its next flag.
    pub next: Option<FieldPos>,
    /// 1 on the ground at its flag, 0 in a carrier's hands.
    pub lying: bool,
    /// ECO-DIST's recheck counter and TRN-CARRY-3.2's wait score.
    pub recheck: u32,
    pub wait_score: u32,
}

/// FMT-SAV-3.6: `Transport BuildingNeedGood`, a building's order: the
/// building, the good, the package serving it and the tick it was filed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedOrder {
    pub building: u64,
    pub good: u32,
    pub package: u64,
    pub stamp: u32,
}

/// FMT-SAV-3.8: a ship of `Navy System`: its owner, its construction
/// (remaining main and frame work, the shipwright, the coast field), its
/// water field (the first `Movement Base` cursor) and its state.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedShip {
    pub owner: u32,
    pub id: u64,
    pub main_left: f32,
    pub frame_left: f32,
    pub shipwright: u64,
    pub coast: Option<FieldPos>,
    pub water: Option<FieldPos>,
    pub state: u32,
    /// FMT-SAV-3.8 (T12): the expedition goods store (the `Stock` after the
    /// movement), the harbour it is docked at and the one it is sent to, the
    /// search counter, the expedition flag, the last port position and the
    /// ship's number (its name index).
    pub store: Vec<(u32, u32)>,
    pub harbour: Option<u64>,
    pub sent_to: Option<u64>,
    pub counter: u32,
    pub expedition: bool,
    pub port: Option<FieldPos>,
    pub number: u32,
}

/// FMT-SAV-3.9: a flag of the net.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedFlag {
    pub owner: u32,
    pub id: u64,
    pub field: FieldPos,
    pub links: usize,
    /// FMT-SAV-3.6, 3.9: the flag's `Package Container`, the ids of the
    /// packages (`Transport Package`) lying at it.
    pub packages: Vec<u64>,
}

/// FMT-SAV-3.9: a street of the net: its fields from end to end, its end
/// flags and its carriers (the last two of its four refs).
#[derive(Debug, Clone, PartialEq)]
pub struct SavedStreet {
    pub owner: u32,
    pub street_type: u32,
    pub id: u64,
    pub fields: Vec<FieldPos>,
    pub flags: [u64; 2],
    pub carriers: [u64; 2],
}

/// FMT-SAV-3.7a: `Military Fighters`, one attacker's pairing (MIL-ATK-2.3 ..
/// 6): the link's attacker, opponent (none until paired) and target
/// building; the round counter (MIL-ATK-6.2); whether the attacker and the
/// opponent have arrived; the attacker's waiting field (MIL-ATK-4.3); the
/// hit pending from the last round (MIL-ATK-6.3 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedFighters {
    pub attacker: Option<u64>,
    pub opponent: Option<u64>,
    pub target: Option<u64>,
    pub counter: u32,
    pub attacker_arrived: bool,
    pub opponent_arrived: bool,
    pub waiting: Option<FieldPos>,
    pub hit_pending: bool,
}

/// FMT-SAV-3.7a: `Military Defender`, one interceptor (MIL-ATK-3): the
/// link's attacker he is paired with (none until paired), the interceptor
/// himself and the building he defends; 1 when that is his own home (never
/// read by the fight); his meeting field (MIL-ATK-3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedDefender {
    pub attacker: Option<u64>,
    pub interceptor: Option<u64>,
    pub target: Option<u64>,
    pub home_attacked: u32,
    pub meeting: Option<FieldPos>,
}

/// A save game read as far as it goes: the map part, then the game block's
/// sections in order; `error` names the section the reading stopped in.
#[derive(Debug, Clone, Default)]
pub struct SaveGame {
    pub map: Option<Map>,
    pub random: Option<[u8; 8]>,
    pub players: Vec<SavedPlayer>,
    pub buildings: Vec<SavedBuilding>,
    pub remains: usize,
    pub orders: usize,
    pub settlers: [Vec<SavedSettler>; 6],
    pub packages: Vec<SavedPackage>,
    pub package_needs: usize,
    /// FMT-SAV-3.6: `Transport PackageNeedTarget`: a package and the
    /// warehouse it is taken to for storage (none while none is found).
    pub package_targets: Vec<(u64, u64)>,
    pub building_needs: usize,
    /// FMT-SAV-3.6: each `Transport BuildingNeedGood`'s building and good id.
    pub building_need_goods: Vec<(u64, u32)>,
    /// FMT-SAV-3.6: the same orders with their serving package and stamp.
    pub building_orders: Vec<SavedOrder>,
    pub military_recruiting: Option<u32>,
    /// FMT-SAV-3.7, 3.7a: the fight records: the attackers' pairings not
    /// yet fighting (list 1), the fighting pairs in their duel order (list
    /// 2) and the interceptors (list 3).
    pub fight_pairings: Vec<SavedFighters>,
    pub fight_duels: Vec<SavedFighters>,
    pub fight_defenders: Vec<SavedDefender>,
    pub ships: usize,
    pub saved_ships: Vec<SavedShip>,
    pub flags: Vec<SavedFlag>,
    pub streets: Vec<SavedStreet>,
    /// FMT-SAV-3.9: each player's three routing tables' node counts.
    pub routing: Vec<[usize; 3]>,
    pub ai_think: Option<u32>,
    pub ai_enabled: Option<bool>,
    /// FMT-SAV-3.10, 3.10a: the active computer players: slot, profile
    /// (3 for "Hard") and home field.
    pub computer_players: Vec<SavedComputer>,
    /// FMT-SAV-3.11: the samples of each player's first series.
    pub statistic_samples: Vec<usize>,
    /// FMT-SAV-3.11: each player's 8 series, then 14 series, oldest
    /// sample first.
    pub statistics: Vec<Vec<Vec<u32>>>,
    /// FMT-SAV-3.11: the count of the 600-tick productivity decay
    /// (ECO-OUT-3.11), not the statistics' sampling.
    pub statistics_cycle: Option<u32>,
    /// FMT-SAV-3.11: each player's coins produced so far, frags so far and
    /// owned field count.
    pub statistic_totals: Vec<[u32; 3]>,
    pub script: Option<String>,
    pub padding: Option<usize>,
    /// The sections read completely, in order.
    pub sections: Vec<&'static str>,
    /// The parts that did not read, in file order. Reading goes on at the
    /// next section found after each, so a save loads what it can.
    pub problems: Vec<SaveProblem>,
}

impl SaveGame {
    /// The whole file was read.
    pub fn complete(&self) -> bool {
        self.problems.is_empty() && self.padding.is_some()
    }
}

/// A part of a save that did not read: its section, the record in it if a
/// list's record failed ("building 6 of 21"), the error and its offset in
/// the decoded file, and the section reading went on with (`None`: it
/// stopped there).
#[derive(Debug, Clone, PartialEq)]
pub struct SaveProblem {
    pub section: &'static str,
    pub record: Option<String>,
    pub offset: usize,
    pub error: S2mError,
    pub resumed: Option<&'static str>,
}

impl std::fmt::Display for SaveProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.section)?;
        if let Some(record) = &self.record {
            write!(f, ", {record}")?;
        }
        write!(f, " at offset {}: {}", self.offset, self.error)?;
        match self.resumed {
            Some(next) => write!(f, "; read on from {next}"),
            None => write!(f, "; nothing after it read"),
        }
    }
}

/// A section that did not read: the error, and the record of a list that
/// failed.
#[derive(Debug)]
struct Failed {
    record: Option<String>,
    error: S2mError,
}

impl From<S2mError> for Failed {
    fn from(error: S2mError) -> Self {
        Self {
            record: None,
            error,
        }
    }
}

/// FMT-SAV-1.2: a save file as stored (the rc00 container keyed by its
/// file name), read as far as it goes.
pub fn load(name: &str, file: &[u8]) -> Result<SaveGame, S2mError> {
    let data = rc00::decode(name, file)?;
    Ok(parse(&data))
}

/// FMT-SAV-2, 3: the decoded contents of a save, read as far as it goes.
/// A part that does not read is kept in [`SaveGame::problems`] and reading
/// goes on at the next section; without its map part nothing is read.
pub fn parse(data: &[u8]) -> SaveGame {
    let mut save = SaveGame::default();
    let stop = |error: S2mError| SaveProblem {
        section: "map part",
        record: None,
        offset: error.offset().unwrap_or(0),
        error,
        resumed: None,
    };
    let map = match s2m::parse(data) {
        Ok(map) => map,
        Err(e) => {
            save.problems.push(stop(e));
            return save;
        }
    };
    let Some(start) = map.game_block else {
        save.problems.push(stop(S2mError::WrongSection {
            offset: 0,
            expected: "MapInfo version 9",
        }));
        return save;
    };
    let fields = (map.info.width, map.info.height);
    save.map = Some(map);
    save.sections.push("map part");
    let mut r = Reader {
        data,
        pos: start,
        fields: Some(fields),
    };
    game_block(&mut r, &mut save);
    save
}

/// A section's reader.
type Section = fn(&mut Reader, &mut SaveGame) -> Result<(), Failed>;

/// FMT-SAV-3.1: the sections of `Game File Logic`, in order (`Logic Random`
/// from its version 1).
const SECTIONS: [(&str, Section); 11] = [
    ("Logic Random", |r, save| {
        save.random = Some(random(r)?);
        Ok(())
    }),
    ("PlayerSystem", |r, save| {
        save.players = players(r)?;
        Ok(())
    }),
    ("VillageSystem", villages),
    ("SettlersSystem", settlers),
    ("Transport System", transport),
    ("MilitarySystem", military),
    ("Navy System", navy),
    ("Net System", net),
    ("AI System", ai),
    ("LogicStatistics", statistics),
    ("GameScript", |r, save| {
        save.script = Some(script(r)?);
        Ok(())
    }),
];

/// FMT-SAV-3.1: `Game File Logic` and its sections in order, then the zero
/// padding (FMT-S2M-1.8). A section that does not read is a problem, and
/// reading goes on at the first later section whose marker comes after
/// the place it stopped (a marker is its class name's CRC and length,
/// FMT-S2M-1.2), so a record the reader does not know costs the rest of its
/// section only.
fn game_block(r: &mut Reader, save: &mut SaveGame) {
    let mut next = 1;
    match r.marker_of("Game File Logic", &[2]) {
        Ok(version) if version >= 1 => next = 0,
        Ok(_) => {}
        Err(error) => {
            let failed = Failed::from(error);
            match resume(r, save, "Game File Logic", failed, 0) {
                Some(i) => next = i,
                None => return,
            }
        }
    }
    while let Some(&(name, read)) = SECTIONS.get(next) {
        match read(r, save) {
            Ok(()) => {
                save.sections.push(name);
                next += 1;
            }
            Err(failed) => match resume(r, save, name, failed, next + 1) {
                Some(i) => next = i,
                None => return,
            },
        }
    }
    match r.padding() {
        Ok(n) => save.padding = Some(n),
        Err(error) => save.problems.push(SaveProblem {
            section: "padding",
            record: None,
            offset: error.offset().unwrap_or(r.pos),
            error,
            resumed: None,
        }),
    }
}

/// Note the problem of `section` and move the reader to the first section
/// from `SECTIONS[from]` on whose marker follows; its index, or `None` when
/// none does.
fn resume(
    r: &mut Reader,
    save: &mut SaveGame,
    section: &'static str,
    failed: Failed,
    from: usize,
) -> Option<usize> {
    let offset = failed.error.offset().unwrap_or(r.pos);
    let found = SECTIONS
        .iter()
        .enumerate()
        .skip(from)
        .find_map(|(i, (name, _))| find_marker(r.data, r.pos, name).map(|at| (i, at)));
    save.problems.push(SaveProblem {
        section,
        record: failed.record,
        offset,
        error: failed.error,
        resumed: found.map(|(i, _)| SECTIONS[i].0),
    });
    let (i, at) = found?;
    r.pos = at;
    Some(i)
}

/// The offset of the first marker of class `name` at or after `from`
/// (FMT-S2M-1.2: version, the name's CRC-32, its length).
fn find_marker(data: &[u8], from: usize, name: &str) -> Option<usize> {
    let mut key = rc00::crc32(name.as_bytes()).to_le_bytes().to_vec();
    key.extend((name.len() as u32).to_le_bytes());
    let start = from.checked_add(4)?;
    data.get(start..)?
        .windows(key.len())
        .position(|w| w == key)
        .map(|p| start + p - 4)
}

/// The class name whose marker is next, if it is one of `names`, without
/// reading it.
fn peek(r: &Reader, names: &[&'static str]) -> Option<&'static str> {
    let at = r.pos;
    let word = |k: usize| {
        r.data
            .get(at + 4 * k..at + 4 * k + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let (crc, len) = (word(1)?, word(2)?);
    names
        .iter()
        .copied()
        .find(|n| rc00::crc32(n.as_bytes()) == crc && n.len() as u32 == len)
}

/// `u32` count, then that many records called `what` into `out`. The
/// records read stay in `out` when one fails, and the failure names it.
fn records<T>(
    r: &mut Reader,
    what: &str,
    out: &mut Vec<T>,
    mut item: impl FnMut(&mut Reader) -> Result<T, S2mError>,
) -> Result<(), Failed> {
    out.clear();
    let count = r.u32()?;
    r.byte_len(u64::from(count), 4)?;
    for n in 1..=count {
        match item(r) {
            Ok(v) => out.push(v),
            Err(error) => {
                return Err(Failed {
                    record: Some(format!("{what} {n} of {count}")),
                    error,
                })
            }
        }
    }
    Ok(())
}

/// `u32` count, then that many items.
fn count_of<T>(
    r: &mut Reader,
    mut item: impl FnMut(&mut Reader) -> Result<T, S2mError>,
) -> Result<Vec<T>, S2mError> {
    r.list(4, |r| item(r))
}

/// FMT-S2M-1.5 (FMT-SAV-3.5a `ref`).
fn id(r: &mut Reader) -> Result<u64, S2mError> {
    r.object_id()
}

/// FMT-SAV-3.5a: a `ref` that may be none (all ones).
fn some_id(r: &mut Reader) -> Result<Option<u64>, S2mError> {
    let id = r.object_id()?;
    Ok((id != crate::s2m::NO_OBJECT).then_some(id))
}

/// FMT-SAV-3.3a `Stock` (v0): `bool`, `i32`, n x (good id, amount).
fn stock(r: &mut Reader) -> Result<Vec<(u32, u32)>, S2mError> {
    r.marker("Stock", 0)?;
    r.bool()?;
    r.i32()?;
    count_of(r, |r| Ok((r.u32()?, r.u32()?)))
}

/// FMT-SAV-3.5a `good`: (`u32` valid flag, `u32` good id).
fn good(r: &mut Reader) -> Result<Option<u32>, S2mError> {
    let valid = r.u32()?;
    let good = r.u32()?;
    Ok((valid != 0).then_some(good))
}

/// A field cursor where none is allowed (FMT-S2M-1.3, in a save).
fn cursor(r: &mut Reader) -> Result<Option<FieldPos>, S2mError> {
    r.optional_field_cursor()
}

/// A fine cursor where none is allowed (FMT-S2M-1.4, in a save).
fn fine(r: &mut Reader) -> Result<Option<(u32, u32)>, S2mError> {
    r.marker("ElevationCursor", 0)?;
    let x = r.u32()?;
    let y = r.u32()?;
    Ok((x != u32::MAX && y != u32::MAX).then_some((x, y)))
}

/// `Settlers Container` (v0): a list of object ids.
fn settlers_container(r: &mut Reader) -> Result<Vec<u64>, S2mError> {
    r.marker("Settlers Container", 0)?;
    count_of(r, id)
}

/// FMT-SAV-3.2
fn random(r: &mut Reader) -> Result<[u8; 8], S2mError> {
    r.marker("Logic Random", 0)?;
    r.bool()?;
    r.array::<8>()
}

/// FMT-SAV-3.3: `PlayerSystem` with its 8 slots.
fn players(r: &mut Reader) -> Result<Vec<SavedPlayer>, S2mError> {
    r.marker("PlayerSystem", 0)?;
    r.bool()?;
    let mut out = Vec::new();
    for _ in 0..8 {
        if r.i32()? == -1 {
            continue;
        }
        out.push(player(r)?);
    }
    Ok(out)
}

/// FMT-SAV-3.3 `PlayerObject` (v5).
fn player(r: &mut Reader) -> Result<SavedPlayer, S2mError> {
    r.marker("PlayerObject", 5)?;
    let active = r.bool()?;
    let name = r.string()?;
    let index = r.u32()?;
    for _ in 0..3 {
        r.u32()?;
    }
    r.marker("PlayerLocksmith", 1)?;
    for _ in 0..12 {
        r.f32()?;
        r.f32()?;
    }
    r.u32()?;
    r.marker("Good Priorities", 0)?;
    if r.bool()? {
        count_of(r, |r| {
            r.marker("Good Priority", 0)?;
            Ok((r.u32()?, r.u32()?))
        })?;
    }
    r.marker("Good Arrangement", 0)?;
    r.bool()?;
    count_of(r, |r| {
        r.marker("Player GoodArrangementGroup", 0)?;
        r.bool()?;
        r.marker("Player ArrangementBase", 0)?;
        count_of(r, |r| {
            r.marker("Player ArrangementObject", 0)?;
            Ok((r.f32()?, r.f32()?, r.u32()?))
        })?;
        r.u32()
    })?;
    r.marker("Player Military", 0)?;
    for _ in 0..5 {
        r.f32()?;
    }
    r.u32()?;
    r.f32()?;
    r.marker("Messages", 0)?;
    count_of(r, |r| {
        r.marker("Message", 2)?;
        r.f32()?;
        cursor(r)?;
        r.string()?;
        r.string()?;
        r.u32()?;
        r.u32()?;
        r.marker("uniqueId", 0)?;
        r.u64()?;
        r.string()
    })?;
    let mut stocks = Vec::new();
    if active {
        stocks.push(stock(r)?);
        r.u32()?;
        r.marker("Player ShipNames", 0)?;
        r.u32()?;
        r.u32()?;
        for _ in 0..16 {
            r.bool()?;
        }
        stocks.push(stock(r)?);
    }
    Ok(SavedPlayer {
        index,
        active,
        name,
        stocks,
    })
}

/// FMT-SAV-3.4: `VillageSystem`.
fn villages(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("VillageSystem", 0)?;
    r.bool()?;
    records(r, "building", &mut save.buildings, |r| {
        let building_type = r.u32()?;
        let owner = r.u32()?;
        building(r, building_type, owner)
    })?;
    let mut remains = Vec::new();
    records(r, "remains record", &mut remains, |r| {
        r.u32()?;
        r.u32()?;
        // FMT-SAV-3.4a (Q-260 a): its own id, the building's field, a
        // `u32`, a `Blocking`, and from version 1 on a `u32`.
        r.marker("VillageRemains", 1)?;
        id(r)?;
        cursor(r)?;
        r.u32()?;
        blocking(r)?;
        r.u32()
    })?;
    save.remains = remains.len();
    r.marker("Order System", 2)?;
    r.bool()?;
    let mut orders = Vec::new();
    records(r, "order", &mut orders, village_order)?;
    save.orders = orders.len();
    r.u32()?;
    Ok(())
}

/// FMT-SAV-3.4a `Blocking` (v0): active, the field, the size class (0 small).
fn blocking(r: &mut Reader) -> Result<(), S2mError> {
    r.marker("Blocking", 0)?;
    r.bool()?;
    cursor(r)?;
    r.u32()?;
    Ok(())
}

/// FMT-SAV-3.4b `Village Order` (v3).
fn village_order(r: &mut Reader) -> Result<(), S2mError> {
    r.marker("Village Order", 3)?;
    id(r)?;
    r.u32()?;
    r.bool()?;
    for _ in 0..4 {
        id(r)?;
    }
    Ok(())
}

/// `Order Container` (v0): the open orders as a list of object ids (the
/// spec says `Village Order` objects; both samples hold ids, Q-204).
fn order_container(r: &mut Reader) -> Result<usize, S2mError> {
    r.marker("Order Container", 0)?;
    Ok(count_of(r, id)?.len())
}

/// The remaining work of the two construction parts and the materials
/// delivered.
type Construction = ([f32; 2], Vec<(u32, u32)>);

/// FMT-SAV-3.4b 3: `VillageConstruction` (v2): the remaining work of the two
/// parts, the builder at work, the materials delivered, the type id.
fn construction(r: &mut Reader) -> Result<Construction, S2mError> {
    r.marker("VillageConstruction", 2)?;
    let left = [r.f32()?, r.f32()?];
    id(r)?;
    let delivered = stock(r)?;
    r.u32()?;
    Ok((left, delivered))
}

/// FMT-SAV-3.4b 10: `VillageBulldozing` (v1): the levelling progress, the
/// leveller, 2, the type id.
fn bulldozing(r: &mut Reader) -> Result<f32, S2mError> {
    r.marker("VillageBulldozing", 1)?;
    let levelled = r.f32()?;
    id(r)?;
    r.u32()?;
    r.u32()?;
    Ok(levelled)
}

/// FMT-SAV-3.4a, 3.4b: `VillageBuilding` (v6).
fn building(r: &mut Reader, building_type: u32, owner: u32) -> Result<SavedBuilding, S2mError> {
    r.marker("VillageBuilding", 6)?;
    let own = id(r)?;
    let field_at = r.pos;
    let field = cursor(r)?.ok_or(S2mError::OutOfRange {
        offset: field_at,
        what: "building field",
        x: u32::MAX,
        y: u32::MAX,
    })?;
    r.marker("ticker", 0)?;
    r.u32()?;
    r.u32()?;
    r.marker("VillageDepot", 0)?;
    let held = stock(r)?;
    let second = stock(r)?;
    r.marker("Need Good System", 0)?;
    count_of(r, |r| {
        r.u32()?;
        id(r)
    })?;
    r.marker("Returning Good System", 0)?;
    count_of(r, id)?;
    r.marker("VillageWorkers", 0)?;
    let workers = count_of(r, id)?;
    // 3.4b 1, 2: the reserved outdoor target and a second reservation.
    let reserved = some_id(r)?;
    id(r)?;
    // 3.4b 3
    let (construction_left, delivered) = construction(r)?;
    // 3.4b 4
    r.marker("VillageProduction", 1)?;
    let production_on = r.bool()?;
    r.u32()?;
    r.bool()?;
    // 3.4b 5, 6
    r.marker("Blocking", 0)?;
    r.bool()?;
    cursor(r)?;
    let size_class = r.u32()?;
    r.u32()?;
    r.u32()?;
    // 3.4b 7, 8, 9
    let door_flag = id(r)?;
    r.marker("Village SettlerSpawn", 0)?;
    r.u32()?;
    cursor(r)?;
    r.marker("Village Territory Updater", 0)?;
    r.bool()?;
    // 3.4b 10
    let levelled = bulldozing(r)?;
    // 3.4b 11
    order_container(r)?;
    r.bool()?;
    // 3.4b 12: military types only.
    let mut soldiers = Vec::new();
    if peek(r, &["Village Military"]).is_some() {
        r.marker("Village Military", 2)?;
        r.marker("VillageSoldiers", 0)?;
        soldiers = settlers_container(r)?;
        r.marker("Village Attackers", 0)?;
        settlers_container(r)?;
        r.u32()?;
        r.f32()?;
        r.bool()?;
        r.u32()?;
        // One counter per rank (MIL: 5 ranks).
        for _ in 0..5 {
            r.u32()?;
        }
        r.marker("Village Interceptors", 0)?;
        settlers_container(r)?;
        r.bool()?;
    }
    // 3.4b 13
    r.marker("Village CarrierRefresh", 0)?;
    r.u32()?;
    r.marker("VillageGoodFlags", 0)?;
    count_of(r, |r| Ok((r.u32()?, r.bool()?, r.bool()?)))?;
    r.u32()?;
    r.u32()?;
    // 3.4b 14
    if peek(r, &["Village Catapult"]).is_some() {
        r.marker("Village Catapult", 0)?;
        cursor(r)?;
        cursor(r)?;
        r.f32()?;
        r.f32()?;
        r.u32()?;
        r.u32()?;
    }
    let harbour = peek(r, &["Village Harbor"]).is_some();
    if harbour {
        r.marker("Village Harbor", 6)?;
        r.bool()?;
        count_of(r, |r| {
            r.marker("Village Landing Position", 0)?;
            id(r)?;
            id(r)?;
            cursor(r)
        })?;
        count_of(r, id)?;
        r.marker("Village Expedition", 0)?;
        r.u32()?;
        stock(r)?;
        id(r)?;
        r.u32()?;
        order_container(r)?;
        count_of(r, id)?;
        id(r)?;
        settlers_container(r)?;
        r.u32()?;
        for _ in 0..3 {
            r.marker("Village HarborNeedsTransfer", 5)?;
        }
    }
    // The target type, then the upgrade's own construction and levelling,
    // not two refs (Q-259).
    r.marker("VillageUpgrade", 0)?;
    let upgrade = if r.bool()? {
        let target = r.u32()?;
        let (construction_left, delivered) = construction(r)?;
        Some(SavedUpgrade {
            target,
            construction_left,
            delivered,
            levelled: bulldozing(r)?,
        })
    } else {
        None
    };
    Ok(SavedBuilding {
        building_type,
        owner,
        id: own,
        field,
        held,
        stock: second,
        workers,
        construction_left,
        delivered,
        production_on,
        size_class,
        door_flag,
        levelled,
        soldiers,
        harbour,
        upgrade,
        reserved,
    })
}

/// FMT-SAV-3.5: `SettlersSystem` and its six lists.
fn settlers(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("SettlersSystem", 0)?;
    r.bool()?;
    for (list, what) in save.settlers.iter_mut().zip(SETTLER_RECORDS) {
        records(r, what, list, |r| {
            let owner = r.u32()?;
            settler(r, owner)
        })?;
    }
    Ok(())
}

/// A record of each settler list, as a problem names it.
const SETTLER_RECORDS: [&str; 6] = [
    "worker",
    "constructor",
    "carrier",
    "bulldozer",
    "soldier",
    "specialist",
];

/// The class of the settler object, by its marker (FMT-SAV-3.5).
const SETTLER_CLASSES: [&str; 6] = [
    "SettlersWorker",
    "SettlersConstructor",
    "SettlersCarrier",
    "SettlersBulldozer",
    "SettlersSoldier",
    "SettlersSpecialist",
];

/// FMT-SAV-3.5b to 3.5f, then the common part 3.5a.
fn settler(r: &mut Reader, owner: u32) -> Result<SavedSettler, S2mError> {
    let offset = r.pos;
    let class = peek(r, &SETTLER_CLASSES).ok_or(S2mError::WrongSection {
        offset,
        expected: "a settler class",
    })?;
    let step = |r: &mut Reader| r.u32().map(|_| ());
    let mut carried = None;
    let mut live_points = None;
    let mut soldier = None;
    let mut home = None;
    let mut site_work = None;
    let mut site_leg = None;
    let mut hunt = None;
    let mut harvest = None;
    match class {
        "SettlersWorker" => {
            r.marker("SettlersWorker", 1)?;
            home = some_id(r)?;
            id(r)?;
            // The twelve states.
            r.u32()?;
            step(r)?;
            // The harvest state (3.5a b, 2026-11-11): the walk out's leg, a
            // `u32`, the countdown, the harvest step.
            let leg = r.u32()?;
            r.u32()?;
            let countdown = r.f32()?;
            harvest = Some(SavedHunt {
                walk_step: leg,
                countdown,
                step: r.u32()?,
            });
            cursor(r)?;
            r.u32()?;
            step(r)?;
            r.u32()?;
            r.u32()?;
            step(r)?;
            step(r)?;
            r.u32()?;
            step(r)?;
            good(r)?;
            step(r)?;
            step(r)?;
            r.u32()?;
            r.u32()?;
            step(r)?;
            r.u32()?;
            r.u32()?;
            step(r)?;
            good(r)?;
            step(r)?;
            step(r)?;
            r.u32()?;
            r.u32()?;
            step(r)?;
            step(r)?;
            r.f32()?;
            cursor(r)?;
            r.u32()?;
            r.u32()?;
            step(r)?;
            r.bool()?;
            r.u32()?;
            step(r)?;
            for _ in 0..4 {
                r.u32()?;
            }
            r.f32()?;
            step(r)?;
            // The hunter's state (3.5a b): the walk out's own step, the walk
            // to the animal's step, the countdown, the hunt step.
            r.u32()?;
            let walk_step = r.u32()?;
            let countdown = r.f32()?;
            let hunt_step = r.u32()?;
            hunt = Some(SavedHunt {
                walk_step,
                countdown,
                step: hunt_step,
            });
        }
        "SettlersConstructor" => {
            r.marker("SettlersConstructor", 0)?;
            step(r)?;
            site_leg = Some(r.u32()?);
            let ordered = r.u32()?;
            let left = r.u32()?;
            r.f32()?;
            let work_step = r.u32()?;
            site_work = Some([ordered, left, work_step]);
            home = some_id(r)?;
        }
        "SettlersCarrier" => {
            r.marker("SettlersCarrier", 0)?;
            r.u32()?;
            r.u32()?;
            r.u32()?;
            step(r)?;
            r.u32()?;
            step(r)?;
            step(r)?;
            carried = good(r)?;
            r.bool()?;
            id(r)?;
        }
        "SettlersBulldozer" => {
            r.marker("SettlersBulldozer", 0)?;
            step(r)?;
            step(r)?;
            home = some_id(r)?;
        }
        "SettlersSoldier" => {
            r.marker("SettlersSoldier", 3)?;
            r.u32()?;
            step(r)?;
            let home = some_id(r)?;
            let attack_target = some_id(r)?;
            let opponent = some_id(r)?;
            r.u32()?;
            let walk_to_target = r.u32()?;
            let attack = [r.u32()?, r.u32()?];
            let defend = [r.u32()?, r.u32()?];
            let walk_out = [r.u32()?, r.u32()?];
            let back = [r.u32()?, r.u32()?];
            let defended = some_id(r)?;
            let defending = r.bool()?;
            soldier = Some(SavedSoldier {
                home,
                attack_target,
                opponent,
                defended,
                defending,
                walk_to_target,
                attack,
                defend,
                walk_out,
                back,
            });
            r.marker("SettlersLivePoints", 0)?;
            live_points = Some(LivePoints {
                health: r.f32()?,
                healing: r.u32()?,
            });
        }
        _ => {
            r.marker("SettlersSpecialist", 0)?;
            cursor(r)?;
            r.u32()?;
            id(r)?;
        }
    }
    // 3.5a: `Settlers Settler` (v0).
    r.marker("Settlers Settler", 0)?;
    let own = id(r)?;
    let movement = movement(r)?;
    r.marker("SettlersAnimation", 1)?;
    let animation = SavedAnimation {
        left: r.f32()?,
        total: r.f32()?,
        code: r.u32()?,
    };
    let package = some_id(r)?;
    let profession = r.u32()?;
    let state = r.u32()?;
    // (c, Q-1032 c): the going-home record's tail: its wander step's
    // counter, the wander failure count, the record's own step.
    r.u32()?;
    r.u32()?;
    let home_failures = r.u32()?;
    r.u32()?;
    r.u32()?;
    id(r)?;
    Ok(SavedSettler {
        home_failures,
        owner,
        id: own,
        profession,
        carried,
        live_points,
        soldier,
        movement,
        animation,
        state,
        package,
        home,
        site_work,
        site_leg,
        hunt,
        harvest,
    })
}

/// FMT-SAV-3.5h `Movement Path Base` (v1): while active, its fields (from
/// the target back to the start) and the index of the running step's goal.
fn path_base(r: &mut Reader) -> Result<Option<(Vec<FieldPos>, u32)>, S2mError> {
    r.marker("Movement Path Base", 1)?;
    if !r.bool()? {
        return Ok(None);
    }
    let fields = count_of(r, cursor)?.into_iter().flatten().collect();
    r.bool()?;
    let index = r.u32()?;
    r.bool()?;
    Ok(Some((fields, index)))
}

/// FMT-SAV-3.5h `MovementMovement` (v1).
fn movement(r: &mut Reader) -> Result<SavedMovement, S2mError> {
    let mut m = SavedMovement {
        progress: -1.0,
        ..Default::default()
    };
    r.marker("MovementMovement", 1)?;
    r.marker("Movement Rough", 0)?;
    r.bool()?;
    for rough in &mut m.rough {
        *rough = cursor(r)?;
    }
    r.marker("Path", 0)?;
    m.path_kind = r.u32()?;
    cursor(r)?;
    r.marker("Movement Free Path", 0)?;
    r.bool()?;
    if let Some((fields, index)) = path_base(r)? {
        (m.free_path, m.free_index) = (fields, index);
    }
    r.marker("Movement Street Path", 0)?;
    r.bool()?;
    r.bool()?;
    r.bool()?;
    r.u32()?;
    cursor(r)?;
    count_of(r, cursor)?;
    id(r)?;
    r.bool()?;
    r.bool()?;
    // Without marker.
    m.fine_on = r.bool()?;
    for fine_cursor in &mut m.fine {
        *fine_cursor = fine(r)?;
    }
    r.marker("Movement Fine Path", 0)?;
    r.bool()?;
    m.fine_path = count_of(r, fine)?.into_iter().flatten().collect();
    r.bool()?;
    m.fine_index = r.u32()?;
    if peek(r, &["Movement Interpolator"]).is_some() {
        r.marker("Movement Interpolator", 0)?;
        m.progress = r.f32()?;
        r.f32()?;
        m.per_tick = r.f32()?;
    }
    m.stop_short = r.u32()?;
    m.fine_at = fine(r)?;
    r.bool()?;
    m.field = cursor(r)?;
    Ok(m)
}

/// FMT-SAV-3.6: `Transport System`.
fn transport(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("Transport System", 0)?;
    r.bool()?;
    r.marker("Package System", 0)?;
    r.bool()?;
    records(r, "package", &mut save.packages, |r| {
        r.u32()?;
        r.marker("Transport Package", 1)?;
        let own = id(r)?;
        let has_target = r.bool()?;
        let has_next = r.bool()?;
        let destination = id(r)?;
        let carrier = id(r)?;
        let flag = id(r)?;
        let good = r.u32()?;
        let next = cursor(r)?;
        let lying = r.bool()?;
        let recheck = r.u32()?;
        let wait_score = r.u32()?;
        // Two cursors and a ref, reset whenever the destination or the
        // next flag changes.
        cursor(r)?;
        cursor(r)?;
        id(r)?;
        Ok(SavedPackage {
            id: own,
            has_target,
            has_next,
            destination,
            carrier,
            flag,
            good,
            next,
            lying,
            recheck,
            wait_score,
        })
    })?;
    r.marker("Transport PackageNeedSystem", 0)?;
    r.bool()?;
    save.package_targets = count_of(r, |r| {
        r.marker("Transport PackageNeedTarget", 0)?;
        Ok((id(r)?, id(r)?))
    })?;
    save.package_needs = save.package_targets.len();
    r.marker("Transport BuildingNeedSystem", 0)?;
    r.bool()?;
    save.building_orders = count_of(r, |r| {
        r.marker("Transport BuildingNeedGood", 1)?;
        let building = id(r)?;
        let good = r.u32()?;
        let package = id(r)?;
        let stamp = r.u32()?;
        id(r)?;
        Ok(SavedOrder {
            building,
            good,
            package,
            stamp,
        })
    })?;
    save.building_need_goods = save
        .building_orders
        .iter()
        .map(|o| (o.building, o.good))
        .collect();
    save.building_needs = save.building_need_goods.len();
    Ok(())
}

/// FMT-SAV-3.7a: `Military FightConnections`, the link of a fight record:
/// in use, then attacker, opponent and target.
fn fight_link(r: &mut Reader) -> Result<[Option<u64>; 3], S2mError> {
    r.marker("Military FightConnections", 0)?;
    r.bool()?;
    Ok([some_id(r)?, some_id(r)?, some_id(r)?])
}

/// FMT-SAV-3.7a: a `Military Fighters` record: v0 has the three `ref`s
/// bare where v1 and v2 have the link record; v0 and v1 lack the always-0
/// `bool` after the field cursor (ignored on reading).
fn fighters(r: &mut Reader) -> Result<SavedFighters, S2mError> {
    let version = r.marker_of("Military Fighters", &[0, 1, 2])?;
    r.bool()?;
    let [attacker, opponent, target] = if version == 0 {
        [some_id(r)?, some_id(r)?, some_id(r)?]
    } else {
        fight_link(r)?
    };
    let counter = r.u32()?;
    let attacker_arrived = r.bool()?;
    let opponent_arrived = r.bool()?;
    let waiting = cursor(r)?;
    if version >= 2 {
        r.bool()?;
    }
    Ok(SavedFighters {
        attacker,
        opponent,
        target,
        counter,
        attacker_arrived,
        opponent_arrived,
        waiting,
        hit_pending: r.bool()?,
    })
}

/// FMT-SAV-3.7a: a `Military Defender` record.
fn defender(r: &mut Reader) -> Result<SavedDefender, S2mError> {
    r.marker("Military Defender", 0)?;
    r.bool()?;
    let [attacker, interceptor, target] = fight_link(r)?;
    Ok(SavedDefender {
        attacker,
        interceptor,
        target,
        home_attacked: r.u32()?,
        meeting: cursor(r)?,
    })
}

/// FMT-SAV-3.7: `MilitarySystem`; (3.7a, 2026-11-12) the fight records of
/// a fight in progress are read into the save (the third list from
/// `Military Fight` v1 on), and reading goes on to `Military Attack` and
/// `Military Training` (was: a save with any fight record lost the rest of
/// its military section).
fn military(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("MilitarySystem", 0)?;
    r.bool()?;
    r.marker("Military Recruting", 0)?;
    save.military_recruiting = Some(r.u32()?);
    r.marker("Military Distances", 0)?;
    r.u32()?;
    r.marker("Military Allocation", 0)?;
    r.u32()?;
    let version = r.marker_of("Military Fight", &[0, 1])?;
    save.fight_pairings = count_of(r, fighters)?;
    save.fight_duels = count_of(r, fighters)?;
    if version >= 1 {
        save.fight_defenders = count_of(r, defender)?;
    }
    r.marker("Military Attack", 0)?;
    r.marker("Military Training", 0)?;
    r.u32()?;
    Ok(())
}

/// FMT-SAV-3.8: `Navy System` and its ships.
fn navy(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("Navy System", 2)?;
    r.bool()?;
    save.saved_ships = count_of(r, |r| {
        let owner = r.u32()?;
        r.marker("Navy Ship", 3)?;
        let own = id(r)?;
        r.marker("NavyConstruction", 1)?;
        let main_left = r.f32()?;
        let frame_left = r.f32()?;
        let shipwright = id(r)?;
        let coast = cursor(r)?;
        r.marker("Navy Movement", 1)?;
        r.marker("Navy Path", 0)?;
        path_base(r)?;
        r.bool()?;
        r.marker("Movement Base", 0)?;
        let water = cursor(r)?;
        for _ in 0..2 {
            cursor(r)?;
        }
        r.marker("Movement Interpolator", 0)?;
        r.f32()?;
        r.f32()?;
        r.f32()?;
        let store = stock(r)?;
        let state = r.u32()?;
        let harbour = some_id(r)?;
        let sent_to = some_id(r)?;
        let counter = r.u32()?;
        r.marker("Navy Expedition", 0)?;
        id(r)?;
        cursor(r)?;
        let expedition = r.bool()?;
        for _ in 0..6 {
            cursor(r)?;
        }
        id(r)?;
        count_of(r, id)?;
        let port = cursor(r)?;
        let number = r.u32()?;
        Ok(SavedShip {
            owner,
            id: own,
            main_left,
            frame_left,
            shipwright,
            coast,
            water,
            state,
            store,
            harbour,
            sent_to,
            counter,
            expedition,
            port,
            number,
        })
    })?;
    save.ships = save.saved_ships.len();
    Ok(())
}

/// FMT-SAV-3.9: `Net System`: flags, streets, routing tables.
fn net(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("Net System", 1)?;
    r.bool()?;
    records(r, "flag", &mut save.flags, |r| {
        let owner = r.u32()?;
        r.marker("Flag", 1)?;
        let own = id(r)?;
        id(r)?;
        let offset = r.pos;
        let field = cursor(r)?.ok_or(S2mError::OutOfRange {
            offset,
            what: "flag field",
            x: u32::MAX,
            y: u32::MAX,
        })?;
        let links = count_of(r, |r| {
            r.marker("Net FlagLink", 0)?;
            id(r)?;
            id(r)
        })?
        .len();
        for _ in 0..3 {
            id(r)?;
        }
        // FMT-SAV-3.6: the flag's packages, as object ids (their class is
        // open in the spec, Q-100; the autosave holds ids, Q-204).
        r.marker("Package Container", 0)?;
        r.bool()?;
        let packages = count_of(r, id)?;
        r.marker("Net Specialist", 0)?;
        settlers_container(r)?;
        order_container(r)?;
        r.i32()?;
        Ok(SavedFlag {
            owner,
            id: own,
            field,
            links,
            packages,
        })
    })?;
    records(r, "street", &mut save.streets, |r| {
        let owner = r.u32()?;
        let street_type = r.u32()?;
        r.marker("NetStreet", 1)?;
        let own = id(r)?;
        let fields = count_of(r, cursor)?.into_iter().flatten().collect();
        r.u32()?;
        let flags = [id(r)?, id(r)?];
        let carriers = [id(r)?, id(r)?];
        r.marker("Net Street Map Updater", 0)?;
        order_container(r)?;
        r.u32()?;
        r.u32()?;
        r.f32()?;
        for _ in 0..5 {
            r.bool()?;
        }
        Ok(SavedStreet {
            owner,
            street_type,
            id: own,
            fields,
            flags,
            carriers,
        })
    })?;
    for _ in 0..8 {
        let mut tables = [0; 3];
        for table in tables.iter_mut() {
            *table = count_of(r, |r| {
                id(r)?;
                count_of(r, |r| {
                    id(r)?;
                    r.u32()
                })
            })?
            .len();
        }
        save.routing.push(tables);
    }
    Ok(())
}

/// FMT-SAV-3.10b `Ai ConstructionOrder`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedAiOrder {
    pub mode: u32,
    /// The 16 x 16 cell, (-1, -1) none.
    pub cell: (i32, i32),
    pub point: Option<FieldPos>,
    pub radius: u32,
    /// The building type's id, -1 none.
    pub building: i32,
    pub weight: u32,
    /// Game seconds when it was made.
    pub time: f32,
}

/// FMT-SAV-3.10b `Ai ExpansionTarget`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedAiTarget {
    /// 0: none.
    pub kind: u32,
    pub at: Option<FieldPos>,
    pub time: f32,
}

/// FMT-SAV-3.10b: a cell marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedAiMarker {
    pub set: bool,
    pub time: f32,
    pub weight: f32,
}

/// FMT-SAV-3.10d `Ai Building`: its own id is its building's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedAiBuilding {
    pub id: u64,
    pub building: u64,
    pub cell: u64,
    pub needs: Vec<u64>,
    pub productions: Vec<u64>,
}

/// FMT-SAV-3.10d `Ai Need`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedAiNeed {
    pub id: u64,
    pub good: u32,
    pub cell: u64,
    pub production: u64,
    pub consumer: u64,
    /// The resource square, (-1, -1) none.
    pub square: (i32, i32),
    pub waiting: bool,
    pub food: u64,
    pub need: u64,
}

/// FMT-SAV-3.10d `Ai Production`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedAiProduction {
    pub id: u64,
    pub good: u32,
    pub cell: u64,
    pub building: u64,
    pub need: u64,
}

/// FMT-SAV-3.10e `Ai Cell`.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedAiCell {
    pub id: u64,
    pub pos: (i32, i32),
    /// The centre the record names (AI-CON-1.1a's position).
    pub centre: Option<FieldPos>,
    pub depots: Vec<u64>,
    pub ordinary: Vec<u64>,
    pub military: Vec<u64>,
    pub productions: Vec<u64>,
    pub needs: Vec<u64>,
    pub crowded: SavedAiMarker,
    pub depot_wanted: SavedAiMarker,
    /// `AI CellFull`: good, its marker, its count.
    pub full: Vec<(u32, SavedAiMarker, u32)>,
    pub clearing: SavedAiMarker,
    pub clear_size: u32,
    pub forester: u64,
    pub harbours: Vec<u64>,
    pub landmass: u32,
    pub harbour_wanted: SavedAiMarker,
    pub hunter: u64,
}

/// FMT-SAV-3.10e: an element of `Ai TerritoryMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedAiTerritory {
    pub building: u64,
    pub penalty: u32,
    pub results: u32,
    pub catapults: u32,
    pub attempts: u32,
}

/// The construction orders of FMT-SAV-3.10a, c, e, f: each source's order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavedAiOrders {
    /// `Ai ConstructionSystem`: the order being carried out.
    pub current: Option<SavedAiOrder>,
    pub needs: Option<SavedAiOrder>,
    pub forester: Option<SavedAiOrder>,
    pub ship: Option<SavedAiOrder>,
    pub hunter: Option<SavedAiOrder>,
    pub depot: Option<SavedAiOrder>,
    pub clearing: Option<SavedAiOrder>,
    pub harbour: Option<SavedAiOrder>,
    pub military: Option<SavedAiOrder>,
    pub catapult: Option<SavedAiOrder>,
}

/// FMT-SAV-3.10a: an active computer player (`AI Player` with its body), its
/// records as they stand in the file: ids and refs, FMT-SAV-3.10b.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavedComputer {
    pub slot: u32,
    pub profile: i32,
    pub home: Option<FieldPos>,
    pub orders: SavedAiOrders,
    /// `Ai LowResResourceMapAdds`: the expansion attempts per 16 x 16 cell.
    pub attempts: Vec<u32>,
    pub buildings: Vec<SavedAiBuilding>,
    pub needs: Vec<SavedAiNeed>,
    pub food_needs: Vec<SavedAiNeed>,
    pub productions: Vec<SavedAiProduction>,
    /// `Ai TerritoryMap`: its width, its height, its cells.
    pub territory: (u32, u32, Vec<SavedAiTerritory>),
    pub cells: Vec<SavedAiCell>,
    /// `Ai CellExpansion`, `Ai NeedsArrangement`: their targets.
    pub cell_target: Option<SavedAiTarget>,
    pub needs_target: Option<SavedAiTarget>,
    /// `Ai ExpansionSystem`: the current target, and the other it holds.
    pub current_target: Option<SavedAiTarget>,
    pub other_target: Option<SavedAiTarget>,
    /// `Ai MilitaryBuildingCreation`: the remembered target.
    pub remembered_target: Option<SavedAiTarget>,
    /// `Ai ResourceSystem`: the waiting list of needs without a cell, and
    /// the waiting need's target.
    pub waiting: Vec<u64>,
    pub waiting_target: Option<SavedAiTarget>,
    /// `Ai AttackExecution`: the target of a failed attack.
    pub attack_target: Option<SavedAiTarget>,
    /// `AI NeedsConstruction`: the need its order is for.
    pub order_need: u64,
    /// `AI ConstructionHunter`: the cell cursor of the hunter search.
    pub hunter_cursor: u32,
    /// `AI CellValidator`: the scan cursor.
    pub cell_scan: (i32, i32),
    /// `Ai ConqueredContinents`.
    pub conquered: Vec<u32>,
    /// FMT-SAV-3.10h: the values of the periodic parts.
    pub progress: SavedAiProgress,
}

/// FMT-SAV-3.10h: what a computer player's periodic parts have reached.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SavedAiProgress {
    /// `Ai ExpansionSystem`: the source of the target in progress: 0 none,
    /// 1 waiting need, 2 crowded cell, 3 anywhere, 4 attack.
    pub expansion_source: u32,
    /// `Ai DestructionSystem`: the flag cursor and the last building's id.
    pub destruction_flag: u32,
    pub destruction_building: u64,
    /// `AI NeedCreation`: the stage and n of the doubling stock check.
    pub need_stage: u32,
    pub stock_check: u32,
    /// `AI LockSmith`: the last tool handled.
    pub last_tool: u32,
    /// `AI GoodsArrangement`: the step.
    pub goods_step: u32,
    /// `AI ProductionControl`: the last entry visited, a key of the property
    /// table of goods and professions.
    pub control_entry: u32,
    /// `AI ForesterConstruction`: the run's list index.
    pub forester_index: u32,
    /// `Ai MilitaryBuildingCreation`: the 3-run cooldown and the attempt count.
    pub military_cooldown: u32,
    pub military_attempts: u32,
    /// `AI MilitarySettings`: the countdown.
    pub settings_countdown: u32,
    /// `AI WeaponProduction`, `AI CoinProduction`: (k, t0).
    pub weapon_gate: (u32, f32),
    pub coin_gate: (u32, f32),
    /// `AI MilitaryUpgrade`: the cursor.
    pub upgrade_cursor: u32,
}

/// FMT-SAV-3.10: `AI System`; an active computer player's body read record
/// by record (FMT-SAV-3.10a to 3.10g), with the records whose role the spec
/// names kept.
fn ai(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("AI System", 2)?;
    r.bool()?;
    let exists: Vec<bool> = {
        let mut slots = vec![false; 8];
        for p in &save.players {
            if let Some(s) = slots.get_mut(p.index as usize) {
                *s = true;
            }
        }
        slots
    };
    for (slot, player_exists) in exists.into_iter().enumerate() {
        r.marker("AI Player", 6)?;
        let active = r.bool()?;
        let profile = r.i32()?;
        if active {
            let body = ai_body(r).map_err(|e| Failed {
                record: Some(format!("the computer player of slot {slot}")),
                error: e,
            })?;
            save.computer_players.push(SavedComputer {
                slot: slot as u32,
                profile,
                ..body
            });
        }
        if player_exists {
            r.marker("Ai Military Map", 0)?;
            r.bool()?;
            let (w, h) = (r.u32()?, r.u32()?);
            for _ in 0..u64::from(w) * u64::from(h) {
                r.marker("AI Military Map Element", 0)?;
                r.u32()?;
                r.u32()?;
                r.u32()?;
            }
            r.u32()?;
            r.u32()?;
        }
    }
    save.ai_think = Some(r.u32()?);
    resource_map(r, "Ai ResourceMap", 1, 5)?;
    resource_map(r, "Ai LowResResourceMap", 0, 2)?;
    save.ai_enabled = Some(r.bool()?);
    Ok(())
}

/// FMT-SAV-3.10b: a pair, two `u32` without a marker; (-1, -1) is none.
fn pair(r: &mut Reader) -> Result<(i32, i32), S2mError> {
    Ok((r.u32()? as i32, r.u32()? as i32))
}

/// FMT-SAV-3.10b: `Ai ConstructionOrder` (v0).
fn ai_order(r: &mut Reader) -> Result<SavedAiOrder, S2mError> {
    r.marker("Ai ConstructionOrder", 0)?;
    Ok(SavedAiOrder {
        mode: r.u32()?,
        cell: pair(r)?,
        point: cursor(r)?,
        radius: r.u32()?,
        building: r.i32()?,
        weight: r.u32()?,
        time: r.f32()?,
    })
}

/// FMT-SAV-3.10b: `Ai ExpansionTarget` (v0).
fn ai_target(r: &mut Reader) -> Result<SavedAiTarget, S2mError> {
    r.marker("Ai ExpansionTarget", 0)?;
    Ok(SavedAiTarget {
        kind: r.u32()?,
        at: cursor(r)?,
        time: r.f32()?,
    })
}

/// FMT-SAV-3.10b: a cell marker, written as `Ai Cell` version 1.
fn ai_marker(r: &mut Reader) -> Result<SavedAiMarker, S2mError> {
    r.marker("Ai Cell", 1)?;
    Ok(SavedAiMarker {
        set: r.bool()?,
        time: r.f32()?,
        weight: r.f32()?,
    })
}

/// FMT-SAV-3.10b: `Ai ReferncesNeed`, `Ai ReferncesProduction`, `Ai
/// ReferncesBuilding` (v0, names as written): a list of refs.
fn ai_refs(r: &mut Reader, class: &'static str) -> Result<Vec<u64>, S2mError> {
    r.marker(class, 0)?;
    count_of(r, id)
}

/// FMT-SAV-3.10d: `Ai Need` (v0).
fn ai_need(r: &mut Reader) -> Result<SavedAiNeed, S2mError> {
    r.marker("Ai Need", 0)?;
    r.bool()?;
    Ok(SavedAiNeed {
        id: id(r)?,
        good: r.u32()?,
        cell: id(r)?,
        production: id(r)?,
        consumer: id(r)?,
        square: pair(r)?,
        waiting: r.bool()?,
        food: id(r)?,
        need: id(r)?,
    })
}

/// FMT-SAV-3.10a: an active computer player's body; slot and profile are
/// the caller's.
fn ai_body(r: &mut Reader) -> Result<SavedComputer, S2mError> {
    let mut c = SavedComputer {
        home: cursor(r)?,
        ..SavedComputer::default()
    };
    ai_construction(r, &mut c)?;
    r.marker("AI NeedsArrangement", 0)?;
    r.bool()?;
    c.needs_target = Some(ai_target(r)?);
    r.marker("AI NeedsConstruction", 0)?;
    r.bool()?;
    c.orders.needs = Some(ai_order(r)?);
    c.order_need = id(r)?;
    r.marker("Ai ResourceMapAdds", 0)?;
    r.bool()?;
    count_of(r, |r| {
        r.marker("Ai ResourceMapAddsElement", 0)?;
        id(r)?;
        r.bool()
    })?;
    r.marker("Ai LowResResourceMapAdds", 0)?;
    r.bool()?;
    c.attempts = count_of(r, |r| {
        r.marker("Ai LowResResourceMapAddsElement", 0)?;
        r.u32()
    })?;
    // FMT-SAV-3.10d: buildings, needs, food needs, productions.
    c.buildings = count_of(r, |r| {
        r.marker("Ai Building", 1)?;
        r.bool()?;
        Ok(SavedAiBuilding {
            id: id(r)?,
            building: id(r)?,
            cell: id(r)?,
            needs: ai_refs(r, "Ai ReferncesNeed")?,
            productions: ai_refs(r, "Ai ReferncesProduction")?,
        })
    })?;
    c.needs = count_of(r, ai_need)?;
    c.food_needs = count_of(r, ai_need)?;
    c.productions = count_of(r, |r| {
        r.marker("Ai Production", 0)?;
        r.bool()?;
        Ok(SavedAiProduction {
            id: id(r)?,
            good: r.u32()?,
            cell: id(r)?,
            building: id(r)?,
            need: id(r)?,
        })
    })?;
    ai_territory(r, &mut c)?;
    ai_cells(r, &mut c)?;
    // FMT-SAV-3.10g, 3.10f: the other subsystems in their order.
    r.marker("Ai ExpansionSystem", 1)?;
    r.bool()?;
    c.current_target = Some(ai_target(r)?);
    c.other_target = Some(ai_target(r)?);
    c.progress.expansion_source = r.u32()?;
    r.marker("AI Expedition", 0)?;
    r.bool()?;
    ai_military(r, &mut c)?;
    r.marker("Ai ResourceSystem", 0)?;
    r.bool()?;
    c.waiting = ai_refs(r, "Ai ReferncesNeed")?;
    c.waiting_target = Some(ai_target(r)?);
    r.u32()?;
    r.marker("Ai DestructionSystem", 2)?;
    r.bool()?;
    c.progress.destruction_flag = r.u32()?;
    c.progress.destruction_building = r.u64()?;
    r.marker("AI NeedCreation", 0)?;
    r.bool()?;
    c.progress.need_stage = r.u32()?;
    c.progress.stock_check = r.u32()?;
    r.marker("AI LockSmith", 1)?;
    r.bool()?;
    id(r)?;
    c.progress.last_tool = r.u32()?;
    r.marker("AI FoodArrangement", 0)?;
    r.bool()?;
    r.marker("AI GoodsArrangement", 0)?;
    r.bool()?;
    c.progress.goods_step = r.u32()?;
    r.marker("AI ProductionControl", 0)?;
    r.bool()?;
    c.progress.control_entry = r.u32()?;
    r.marker("Ai ConqueredContinents", 0)?;
    r.bool()?;
    c.conquered = count_of(r, |r| r.u32())?;
    Ok(c)
}

/// FMT-SAV-3.10c: `Ai ConstructionSystem` (v3) and its seven parts.
fn ai_construction(r: &mut Reader, c: &mut SavedComputer) -> Result<(), S2mError> {
    r.marker("Ai ConstructionSystem", 3)?;
    r.bool()?;
    c.orders.current = Some(ai_order(r)?);
    r.u32()?;
    r.marker("AI BuildingConstruction", 0)?;
    r.bool()?;
    r.u32()?;
    ai_order(r)?;
    r.marker("AI SearchConstructionPlace", 0)?;
    r.i32()?;
    r.bool()?;
    r.marker("AI CellPositionItertator", 0)?;
    pair(r)?;
    cursor(r)?;
    r.u32()?;
    r.u32()?;
    for _ in 0..4 {
        r.u32()?;
    }
    for _ in 0..3 {
        cursor(r)?;
    }
    r.u32()?;
    r.f32()?;
    r.u32()?;
    cursor(r)?;
    r.marker("AI StreetConstruction", 2)?;
    r.bool()?;
    r.u32()?;
    r.u64()?;
    r.u32()?;
    count_of(r, cursor)?;
    // The m records after their `u32` are not named (FMT-SAV-3.10c);
    // only an empty list reads.
    let offset = r.pos;
    if r.u32()? != 0 {
        return Err(S2mError::WrongSection {
            offset,
            expected: "no street construction records (their class is not specified)",
        });
    }
    cursor(r)?;
    for _ in 0..4 {
        r.u32()?;
    }
    cursor(r)?;
    r.marker("AI ForesterConstruction", 1)?;
    r.bool()?;
    c.orders.forester = Some(ai_order(r)?);
    c.progress.forester_index = r.u32()?;
    r.marker("AI StreetOptimizer", 1)?;
    r.bool()?;
    r.u32()?;
    r.marker("AI StreetRouteOptimizer", 1)?;
    r.bool()?;
    r.u64()?;
    count_of(r, cursor)?;
    for _ in 0..4 {
        r.u32()?;
    }
    r.marker("AI ShipConstruction", 0)?;
    r.bool()?;
    c.orders.ship = Some(ai_order(r)?);
    id(r)?;
    r.marker("AI ConstructionHunter", 0)?;
    r.bool()?;
    c.orders.hunter = Some(ai_order(r)?);
    c.hunter_cursor = r.u32()?;
    Ok(())
}

/// FMT-SAV-3.10e: `Ai TerritoryMap` (v0).
fn ai_territory(r: &mut Reader, c: &mut SavedComputer) -> Result<(), S2mError> {
    r.marker("Ai TerritoryMap", 0)?;
    r.bool()?;
    let (w, h) = (r.u32()?, r.u32()?);
    let mut cells = Vec::new();
    for _ in 0..u64::from(w) * u64::from(h) {
        r.marker("AI TerritoryMap Element", 2)?;
        cells.push(SavedAiTerritory {
            building: id(r)?,
            penalty: r.u32()?,
            results: r.u32()?,
            catapults: r.u32()?,
            attempts: r.u32()?,
        });
    }
    c.territory = (w, h, cells);
    Ok(())
}

/// FMT-SAV-3.10e: `Ai CellSystem` (v3) and its cells.
fn ai_cells(r: &mut Reader, c: &mut SavedComputer) -> Result<(), S2mError> {
    r.marker("Ai CellSystem", 3)?;
    r.bool()?;
    r.marker("Ai CellDepotCreation", 0)?;
    r.bool()?;
    c.orders.depot = Some(ai_order(r)?);
    r.marker("Ai CellExpansion", 0)?;
    r.bool()?;
    c.cell_target = Some(ai_target(r)?);
    c.cells = count_of(r, |r| {
        r.marker("Ai Cell", 5)?;
        r.bool()?;
        Ok(SavedAiCell {
            id: id(r)?,
            pos: pair(r)?,
            centre: cursor(r)?,
            depots: ai_refs(r, "Ai ReferncesBuilding")?,
            ordinary: ai_refs(r, "Ai ReferncesBuilding")?,
            military: ai_refs(r, "Ai ReferncesBuilding")?,
            productions: ai_refs(r, "Ai ReferncesProduction")?,
            needs: ai_refs(r, "Ai ReferncesNeed")?,
            crowded: ai_marker(r)?,
            depot_wanted: ai_marker(r)?,
            full: {
                r.marker("AI CellFull", 0)?;
                r.bool()?;
                count_of(r, |r| {
                    let good = r.u32()?;
                    r.marker("AI CellFullElement", 0)?;
                    Ok((good, ai_marker(r)?, r.u32()?))
                })?
            },
            clearing: ai_marker(r)?,
            clear_size: r.u32()?,
            forester: id(r)?,
            harbours: ai_refs(r, "Ai ReferncesBuilding")?,
            landmass: r.u32()?,
            harbour_wanted: ai_marker(r)?,
            hunter: id(r)?,
        })
    })?;
    r.marker("AI CellValidator", 0)?;
    r.bool()?;
    c.cell_scan = pair(r)?;
    pair(r)?;
    r.marker("AI CellClearing", 0)?;
    r.bool()?;
    pair(r)?;
    pair(r)?;
    c.orders.clearing = Some(ai_order(r)?);
    r.marker("Ai HarborCreation", 1)?;
    r.bool()?;
    c.orders.harbour = Some(ai_order(r)?);
    r.bool()?;
    Ok(())
}

/// FMT-SAV-3.10f: `Ai MilitarySystem` (v8) and its ten parts.
fn ai_military(r: &mut Reader, c: &mut SavedComputer) -> Result<(), S2mError> {
    r.marker("Ai MilitarySystem", 8)?;
    r.bool()?;
    r.marker("Ai MilitaryBuildingCreation", 0)?;
    r.bool()?;
    c.orders.military = Some(ai_order(r)?);
    c.remembered_target = Some(ai_target(r)?);
    c.progress.military_cooldown = r.u32()?;
    c.progress.military_attempts = r.u32()?;
    r.marker("AI MilitarySettings", 0)?;
    r.bool()?;
    c.progress.settings_countdown = r.u32()?;
    r.marker("Ai AttackSystem", 1)?;
    r.bool()?;
    r.marker("Ai AttackTargetSelection", 0)?;
    r.bool()?;
    r.marker("Ai AttackTarget", 0)?;
    pair(r)?;
    r.u32()?;
    r.f32()?;
    r.u32()?;
    r.marker("Ai AttackExecution", 1)?;
    r.bool()?;
    cursor(r)?;
    c.attack_target = Some(ai_target(r)?);
    r.u32()?;
    r.marker("AI SoldierCreation", 0)?;
    r.bool()?;
    r.marker("AI CatapultConstruction", 0)?;
    r.bool()?;
    pair(r)?;
    r.f32()?;
    pair(r)?;
    r.u32()?;
    c.orders.catapult = Some(ai_order(r)?);
    r.marker("AI CastleSettings", 0)?;
    r.bool()?;
    r.marker("AI WeaponProduction", 0)?;
    r.bool()?;
    c.progress.weapon_gate = (r.u32()?, r.f32()?);
    r.marker("AI CoinProduction", 0)?;
    r.bool()?;
    c.progress.coin_gate = (r.u32()?, r.f32()?);
    r.marker("AI CoinArrangement", 0)?;
    r.bool()?;
    r.marker("AI MilitaryUpgrade", 1)?;
    r.bool()?;
    c.progress.upgrade_cursor = r.u32()?;
    Ok(())
}

/// FMT-SAV-3.10: a resource map: `bool`, w, h, w x h cell lists, then
/// `tail` plain `u32`.
fn resource_map(
    r: &mut Reader,
    class: &'static str,
    version: u32,
    tail: usize,
) -> Result<(), S2mError> {
    r.marker(class, version)?;
    r.bool()?;
    let (w, h) = (r.u32()?, r.u32()?);
    for _ in 0..u64::from(w) * u64::from(h) {
        r.marker("AI ResourceMapElements", 0)?;
        count_of(r, |r| {
            r.marker("AI ResourceMapElement", 0)?;
            r.u32()?;
            r.u32()?;
            r.u32()?;
            r.i32()
        })?;
    }
    for _ in 0..tail {
        r.u32()?;
    }
    Ok(())
}

/// FMT-SAV-3.11: `LogicStatistics`.
fn statistics(r: &mut Reader, save: &mut SaveGame) -> Result<(), Failed> {
    r.marker("LogicStatistics", 0)?;
    save.statistics_cycle = Some(r.u32()?);
    count_of(r, |r| {
        id(r)?;
        r.u32()?;
        r.f32()?;
        r.u32()
    })?;
    for _ in 0..8 {
        r.marker("LogicPlayerStatistics", 2)?;
        let mut series = Vec::with_capacity(8 + 14);
        for _ in 0..8 + 14 {
            series.push(count_of(r, |r| r.u32())?);
        }
        save.statistic_samples
            .push(series.first().map_or(0, Vec::len));
        save.statistics.push(series);
        save.statistic_totals.push([r.u32()?, r.u32()?, r.u32()?]);
    }
    Ok(())
}

/// FMT-SAV-3.12: `GameScript`: the script path; the variables are read
/// and dropped.
fn script(r: &mut Reader) -> Result<String, S2mError> {
    r.marker("GameScript", 0)?;
    r.bool()?;
    let path = r.string()?;
    count_of(r, |r| {
        r.string()?;
        r.u32()
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A class marker (FMT-S2M-1.2): version, the name's CRC-32, its length.
    fn marker(out: &mut Vec<u8>, name: &str, version: u32) {
        out.extend(version.to_le_bytes());
        out.extend(rc00::crc32(name.as_bytes()).to_le_bytes());
        out.extend((name.len() as u32).to_le_bytes());
    }

    fn words(out: &mut Vec<u8>, values: &[u32]) {
        for v in values {
            out.extend(v.to_le_bytes());
        }
    }

    // A section that does not read is a problem with its record and offset,
    // and reading goes on at the next section whose marker follows: here a
    // PlayerSystem of an unknown version, a VillageSystem whose first
    // building is not one, a SettlersSystem that reads, and a
    // Transport System after which no section follows.
    // spec: FMT-SAV-3.1, FMT-S2M-1.2 (the sections and their markers)
    #[test]
    fn a_section_that_does_not_read_is_skipped_to_the_next() {
        let mut data = Vec::new();
        marker(&mut data, "Game File Logic", 2);
        marker(&mut data, "Logic Random", 0);
        words(&mut data, &[1]);
        data.extend([1, 2, 3, 4, 5, 6, 7, 8]);
        let players_at = data.len();
        marker(&mut data, "PlayerSystem", 9);
        data.extend([0xee; 13]);
        marker(&mut data, "VillageSystem", 0);
        words(&mut data, &[0, 2, 0x1234, 0]);
        let building_at = data.len();
        marker(&mut data, "Stock", 0);
        data.extend([0xee; 7]);
        marker(&mut data, "SettlersSystem", 0);
        words(&mut data, &[1, 0, 0, 0, 0, 0, 0]);
        let transport_at = data.len();
        marker(&mut data, "Transport System", 0);
        words(&mut data, &[5]);
        let mut save = SaveGame::default();
        let mut r = Reader {
            data: &data,
            pos: 0,
            fields: Some((4, 4)),
        };
        game_block(&mut r, &mut save);
        assert_eq!(save.random, Some([1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(save.sections, ["Logic Random", "SettlersSystem"]);
        let found: Vec<_> = save
            .problems
            .iter()
            .map(|p| (p.section, p.record.as_deref(), p.offset, p.resumed))
            .collect();
        assert_eq!(
            found,
            [
                // An unknown version has no offset of its own: the place
                // after the marker.
                ("PlayerSystem", None, players_at + 12, Some("VillageSystem")),
                (
                    "VillageSystem",
                    Some("building 1 of 2"),
                    building_at,
                    Some("SettlersSystem")
                ),
                ("Transport System", None, transport_at + 12, None),
            ]
        );
        assert!(save.buildings.is_empty());
        assert!(!save.complete());
        assert_eq!(
            save.problems[1].to_string(),
            format!(
                "VillageSystem, building 1 of 2 at offset {building_at}: expected section \
                 `VillageBuilding` at offset {building_at}; read on from SettlersSystem"
            )
        );
    }

    /// A `logic UniqueId` (FMT-S2M-1.5).
    fn object(out: &mut Vec<u8>, id: u64) {
        marker(out, "logic UniqueId", 0);
        out.extend(id.to_le_bytes());
    }

    /// A field cursor (FMT-S2M-1.3); none is all ones.
    fn field(out: &mut Vec<u8>, at: Option<(u32, u32)>) {
        marker(out, "PatternCursor", 0);
        let (x, y) = at.unwrap_or((u32::MAX, u32::MAX));
        words(out, &[x, y]);
    }

    // FMT-SAV-3.7a (T16): a `Military Fight` block (v1) with an older
    // pairing on its way (a v0 record, its refs bare) in list 1, one
    // fighting pair in list 2 (both arrived, counter 3, hit pending) and
    // one interceptor in list 3 is read, record by record, and reading
    // goes on to `Military Attack` and `Military Training`.
    // spec: FMT-SAV-3.7, FMT-SAV-3.7a
    #[test]
    fn t16_a_fight_in_progress_is_read() {
        let none = u64::MAX;
        let mut data = Vec::new();
        marker(&mut data, "MilitarySystem", 0);
        words(&mut data, &[1]);
        marker(&mut data, "Military Recruting", 0);
        words(&mut data, &[10]);
        marker(&mut data, "Military Distances", 0);
        words(&mut data, &[0]);
        marker(&mut data, "Military Allocation", 0);
        words(&mut data, &[0]);
        marker(&mut data, "Military Fight", 1);
        // List 1: a v0 record.
        words(&mut data, &[1]);
        marker(&mut data, "Military Fighters", 0);
        words(&mut data, &[1]);
        for id in [501, none, 900] {
            object(&mut data, id);
        }
        words(&mut data, &[0, 0, 0]);
        field(&mut data, None);
        words(&mut data, &[0]);
        // List 2: a fighting pair.
        words(&mut data, &[1]);
        marker(&mut data, "Military Fighters", 2);
        words(&mut data, &[1]);
        marker(&mut data, "Military FightConnections", 0);
        words(&mut data, &[1]);
        for id in [502, 601, 900] {
            object(&mut data, id);
        }
        words(&mut data, &[3, 1, 1]);
        field(&mut data, Some((2, 3)));
        words(&mut data, &[0, 1]);
        // List 3: an interceptor.
        words(&mut data, &[1]);
        marker(&mut data, "Military Defender", 0);
        words(&mut data, &[1]);
        marker(&mut data, "Military FightConnections", 0);
        words(&mut data, &[1]);
        for id in [501, 602, 900] {
            object(&mut data, id);
        }
        words(&mut data, &[1]);
        field(&mut data, Some((1, 1)));
        marker(&mut data, "Military Attack", 0);
        marker(&mut data, "Military Training", 0);
        words(&mut data, &[7]);
        let mut save = SaveGame::default();
        let mut r = Reader {
            data: &data,
            pos: 0,
            fields: Some((4, 4)),
        };
        military(&mut r, &mut save).expect("the block reads");
        assert_eq!(r.pos, data.len());
        assert_eq!(save.military_recruiting, Some(10));
        let at = |x, y| Some(FieldPos { x, y });
        assert_eq!(
            save.fight_pairings,
            [SavedFighters {
                attacker: Some(501),
                opponent: None,
                target: Some(900),
                counter: 0,
                attacker_arrived: false,
                opponent_arrived: false,
                waiting: None,
                hit_pending: false,
            }]
        );
        assert_eq!(
            save.fight_duels,
            [SavedFighters {
                attacker: Some(502),
                opponent: Some(601),
                target: Some(900),
                counter: 3,
                attacker_arrived: true,
                opponent_arrived: true,
                waiting: at(2, 3),
                hit_pending: true,
            }]
        );
        assert_eq!(
            save.fight_defenders,
            [SavedDefender {
                attacker: Some(501),
                interceptor: Some(602),
                target: Some(900),
                home_attacked: 1,
                meeting: at(1, 1),
            }]
        );
    }

    // A list keeps the records read before the one that fails.
    // spec: FMT-SAV-3.4a (the building list)
    #[test]
    fn records_before_a_failing_one_are_kept() {
        let mut data = Vec::new();
        words(&mut data, &[3, 7, 8, 0xbad]);
        let mut r = Reader {
            data: &data,
            pos: 0,
            fields: None,
        };
        let mut out = Vec::new();
        let failed = records(&mut r, "value", &mut out, |r| match r.u32()? {
            0xbad => Err(S2mError::InvalidBool {
                offset: r.pos - 4,
                value: 0xbad,
            }),
            v => Ok(v),
        })
        .unwrap_err();
        assert_eq!(out, [7, 8]);
        assert_eq!(failed.record.as_deref(), Some("value 3 of 3"));
        assert_eq!(failed.error.offset(), Some(12));
    }
}
