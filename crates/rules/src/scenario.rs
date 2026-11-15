//! A scripted test scenario, not an AI: every player puts military
//! buildings on its land as near to the nearest enemy headquarters as it
//! can, one at a time, and joins each to its road network. It only submits
//! the commands a player can give, so a run is replayable from its seed and
//! the command log (`specs/TER-territory.md`, `MIL-soldiers.md` show up
//! once the two sides' land meets).

use s2_sim::grid::{self, Point};
use s2_sim::{BuildingSize, Command, PlayerId, World};

use crate::model::RuleSet;

/// Ticks between two decisions of the players.
const DECISION_PERIOD: u64 = 25;
/// A new building keeps this far from the military buildings it has.
const MIN_SPACING: u32 = 4;
/// Flags tried, nearest first, for the road of a new building.
const ROAD_TRIES: usize = 6;
/// The names the scripts give the military building types, in the order the
/// scenario builds them: a guard house, a tower after every third one.
const BUILDINGS: [&str; 2] = ["GuardHouse", "Tower"];
/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;

/// What a run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expansion {
    /// Each military building placed, with its owner, in order.
    pub placed: Vec<(PlayerId, Point)>,
    /// Buildings given up for want of a road.
    pub without_road: u32,
    /// The fields they stood on: not tried again.
    pub unreachable: Vec<Point>,
}

/// Run `world` until tick `until`, the players expanding towards each other.
pub fn expand_towards_each_other(world: &mut World, rules: &RuleSet, until: u64) -> Expansion {
    let mut report = Expansion::default();
    let types: Vec<(u32, BuildingSize)> = BUILDINGS
        .iter()
        .filter_map(|name| {
            let building = rules
                .buildings
                .iter()
                .find(|b| b.name.as_deref() == Some(name))?;
            Some((building.id, world.building_type(building.id)?.size))
        })
        .collect();
    let Some(headquarters) = rules.id_named(crate::HEADQUARTERS) else {
        return report;
    };
    let hqs: Vec<(PlayerId, Point)> = world
        .buildings()
        .filter(|b| b.building_type == headquarters)
        .map(|b| (b.owner, b.at))
        .collect();
    let mut built = vec![0usize; hqs.len()];
    while world.tick() < until {
        if world.tick().is_multiple_of(DECISION_PERIOD) {
            for (i, &(player, hq)) in hqs.iter().enumerate() {
                let Some(&(_, enemy)) = hqs
                    .iter()
                    .filter(|&&(p, _)| p != player)
                    .min_by_key(|&&(p, at)| (grid::distance(hq, at), p.0))
                else {
                    continue;
                };
                let kind = if built[i] % 3 == 2 { 1 } else { 0 };
                let Some(&(id, size)) = types.get(kind).or(types.first()) else {
                    continue;
                };
                if let Some(at) = expand_once(world, player, hq, enemy, id, size, &mut report) {
                    built[i] += 1;
                    report.placed.push((player, at));
                }
            }
            if world.tick() >= until {
                break;
            }
        }
        world.step();
    }
    report
}

/// One building of `player` towards `enemy`, if none of its military
/// buildings is still waiting for its first soldier or construction.
fn expand_once(
    world: &mut World,
    player: PlayerId,
    hq: Point,
    enemy: Point,
    id: u32,
    size: BuildingSize,
    report: &mut Expansion,
) -> Option<Point> {
    let military = |world: &World, kind: u32| {
        world
            .building_type(kind)
            .is_some_and(|t| t.soldier_limit > 0)
    };
    let own: Vec<(Point, bool)> = world
        .buildings()
        .filter(|b| b.owner == player && military(world, b.building_type))
        .map(|b| {
            let waiting =
                b.under_construction || world.military_at(b.at).is_some_and(|m| !m.applied);
            (b.at, waiting)
        })
        .collect();
    if own.iter().any(|&(_, waiting)| waiting) {
        return None;
    }
    let best = world
        .terrain()
        .fields()
        .filter(|&f| world.field_owner(f) == Some(player))
        .filter(|&f| {
            own.iter()
                .all(|&(at, _)| grid::distance(at, f) >= MIN_SPACING)
        })
        .filter(|f| !report.unreachable.contains(f))
        .filter(|&f| world.check_building(player, f, size).is_ok())
        .min_by_key(|&f| (grid::distance(f, enemy), f))?;
    let outcome = step_with(
        world,
        Command::PlaceBuilding {
            player,
            building: id,
            at: best,
        },
    );
    outcome.ok()?;
    let flag = world.terrain().neighbour(best, FLAG_DIRECTION)?;
    // Only flags that are on the headquarters' network: a flag left behind
    // by a removed building is not.
    let hq_flag = world.terrain().neighbour(hq, FLAG_DIRECTION)?;
    let mut flags: Vec<Point> = world
        .flags()
        .filter(|&(f, owner)| owner == player && f != flag)
        .map(|(f, _)| f)
        .filter(|&f| world.route_cost(player, hq_flag, f).is_some())
        .collect();
    flags.sort_by_key(|&f| (grid::distance(flag, f), f));
    for other in flags.into_iter().take(ROAD_TRIES) {
        let road = Command::BuildRoad {
            player,
            from: flag,
            to: other,
        };
        if step_with(world, road).is_ok() {
            return Some(best);
        }
    }
    // No road: the building would never be built.
    report.without_road += 1;
    report.unreachable.push(best);
    let _ = step_with(world, Command::RemoveBuilding { player, at: best });
    None
}

/// Submit `command`, step once and give its outcome.
fn step_with(world: &mut World, command: Command) -> Result<(), s2_sim::Rejection> {
    world.submit(command);
    world.step();
    world
        .last_results()
        .last()
        .map_or(Ok(()), |result| result.outcome)
}

/// What an attack run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttackReport {
    /// The attack orders given: tick, target, soldiers ordered.
    pub orders: Vec<(u64, Point, u32)>,
    /// The first building taken: its field and the tick.
    pub conquered: Option<(Point, u64)>,
    /// The tick of the first duel, if one was fought.
    pub first_fight: Option<u64>,
}

/// When an attack run ends before its limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackStop {
    /// At the first duel in progress.
    Fight,
    /// When a building has been taken.
    Conquest,
}

/// Player `attacker` orders attacks until the run reaches tick `until` or
/// `stop` happens: every decision period, when none of its soldiers is out on
/// an attack, it attacks the enemy military building it can send the most
/// soldiers against (the nearest of those), with the dialog's highest count,
/// strong units first. Only commands a player can give.
pub fn attack_until(
    world: &mut World,
    attacker: PlayerId,
    until: u64,
    stop: AttackStop,
) -> AttackReport {
    let mut report = AttackReport::default();
    let mut target: Option<Point> = None;
    while world.tick() < until {
        if world.tick().is_multiple_of(DECISION_PERIOD) {
            let out = world.attack_records().iter().any(|r| {
                world
                    .settlers()
                    .any(|(id, s)| id == r.soldier && s.player == attacker)
            });
            if !out {
                let best = world
                    .buildings()
                    .filter(|b| world.are_enemies(attacker, b.owner))
                    .filter_map(|b| {
                        let available = world.attack_available(attacker, b.at);
                        (available > 0).then_some((available, b.at))
                    })
                    .max_by_key(|&(available, at)| (available, std::cmp::Reverse(at)));
                if let Some((_, at)) = best {
                    let count = world.attack_counts(attacker, at)[3];
                    target = Some(at);
                    report.orders.push((world.tick(), at, count));
                    world.submit(Command::Attack {
                        player: attacker,
                        target: at,
                        count,
                        strong_first: true,
                    });
                }
            }
        }
        world.step();
        if report.first_fight.is_none() && !world.duels().is_empty() {
            report.first_fight = Some(world.tick());
            if stop == AttackStop::Fight {
                break;
            }
        }
        // A building of the enemy that is now ours.
        if let Some(at) = target {
            if world.building_at(at).is_some_and(|b| b.owner == attacker) {
                report.conquered = Some((at, world.tick()));
                if stop == AttackStop::Conquest {
                    break;
                }
            }
        }
    }
    report
}

/// What a human player builds, in order, by script name: wood, planks and
/// stone first, then land and a few medium buildings; after the list the
/// last [`HUMAN_REPEAT`] entries repeat.
const HUMAN_ORDER: [&str; 30] = [
    "WoodcutterHut",
    "WoodcutterHut",
    "ForesterHouse",
    "Sawmill",
    "Stone Pit",
    "GuardHouse",
    "WoodcutterHut",
    "Stone Pit",
    "GuardHouse",
    "ForesterHouse",
    "Sawmill",
    "Mill",
    "Depot",
    "Tower",
    "FisherHut",
    "Farm",
    "Brewery",
    "Spring",
    "Slaughter House",
    "Bakery",
    "Barracks",
    "Piggerey",
    "Smeltery",
    "Depot",
    "GuardHouse",
    "WoodcutterHut",
    "Tower",
    "ForesterHouse",
    "Stone Pit",
    "GuardHouse",
];
const HUMAN_REPEAT: usize = 6;
/// Ticks between two decisions of the human.
const HUMAN_PERIOD: u64 = 120;
/// Sites the human has open at most before placing the next building.
const HUMAN_SITES: usize = 4;
/// Roads tried for one building before it is given up.
const HUMAN_ROAD_TRIES: u32 = 4;

/// A scripted player that builds like a person at the screen would: the
/// [`HUMAN_ORDER`] near the headquarters (military buildings at the edge of
/// its land), each joined by a road from its flag to a flag of the network
/// or into the middle of a road (which splits it), and flags set on every
/// other field of each new road. Only commands a player can give, through
/// the interface's route building (TRN-ROAD-8); not an AI.
pub struct Human {
    player: PlayerId,
    hq: Point,
    order: Vec<(u32, BuildingSize, bool)>,
    next: usize,
    rng: u64,
    /// Building flags whose road is not there yet, with the tries so far.
    unjoined: Vec<(Point, Point, u32)>,
    /// Building flags whose new road gets its flags next.
    to_flag: Vec<Point>,
    /// Fields given up: not tried again.
    refused: Vec<Point>,
}

impl Human {
    /// The human playing `player`, if it has a headquarters.
    pub fn new(world: &World, rules: &RuleSet, player: PlayerId, seed: u64) -> Option<Self> {
        let headquarters = rules.id_named(crate::HEADQUARTERS)?;
        let hq = world
            .buildings()
            .find(|b| b.owner == player && b.building_type == headquarters)?
            .at;
        let order = HUMAN_ORDER
            .iter()
            .filter_map(|name| {
                let building = rules
                    .buildings
                    .iter()
                    .find(|b| b.name.as_deref() == Some(name))?;
                let kind = world.building_type(building.id)?;
                Some((building.id, kind.size, kind.soldier_limit > 0))
            })
            .collect();
        Some(Self {
            player,
            hq,
            order,
            next: 0,
            rng: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            unjoined: Vec::new(),
            to_flag: Vec::new(),
            refused: Vec::new(),
        })
    }

    fn random(&mut self, below: u32) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % u64::from(below.max(1))) as u32
    }

    /// The commands of this tick.
    pub fn decide(&mut self, world: &World) -> Vec<Command> {
        let player = self.player;
        if !world.tick().is_multiple_of(HUMAN_PERIOD) {
            return Vec::new();
        }
        if world.building_at(self.hq).is_none_or(|b| b.owner != player) {
            return Vec::new();
        }
        let mut out = Vec::new();
        if world.route(player).is_some() {
            out.push(Command::CancelRoute { player });
        }
        let has_road = |flag: Point| {
            world
                .roads()
                .any(|r| r.owner == player && (r.path[0] == flag || r.path.last() == Some(&flag)))
        };
        // Flags on the new roads, every other field.
        for flag in std::mem::take(&mut self.to_flag) {
            let Some(road) = world
                .roads()
                .find(|r| r.owner == player && (r.path[0] == flag || r.path.last() == Some(&flag)))
            else {
                continue;
            };
            for (i, &field) in road.path.iter().enumerate() {
                let inner = i > 0 && i + 1 < road.path.len();
                if inner && i % 2 == 0 && world.check_flag(player, field).is_ok() {
                    out.push(Command::PlaceFlag { player, at: field });
                }
            }
        }
        // Roads for the buildings placed before.
        let mut still = Vec::new();
        for (at, flag, tries) in std::mem::take(&mut self.unjoined) {
            if world.building_at(at).is_none() {
                continue;
            }
            if has_road(flag) {
                self.to_flag.push(flag);
                continue;
            }
            if tries >= HUMAN_ROAD_TRIES {
                self.refused.push(at);
                out.push(Command::RemoveBuilding { player, at });
                continue;
            }
            if out.iter().all(|c| !matches!(c, Command::StartRoute { .. })) {
                out.extend(self.road_from(world, flag, tries));
            }
            still.push((at, flag, tries + 1));
        }
        self.unjoined = still;
        let open = world
            .buildings()
            .filter(|b| b.owner == player && b.under_construction)
            .count();
        if open >= HUMAN_SITES || !self.unjoined.is_empty() || self.order.is_empty() {
            return out;
        }
        let i = if self.next < self.order.len() {
            self.next
        } else {
            let n = self.order.len();
            n - HUMAN_REPEAT + (self.next - n) % HUMAN_REPEAT
        };
        let (id, size, military) = self.order[i];
        let jitter: Vec<u32> = (0..4).map(|_| self.random(4)).collect();
        let hq = self.hq;
        let best = world
            .terrain()
            .fields()
            .filter(|&f| world.field_owner(f) == Some(player))
            .filter(|f| !self.refused.contains(f))
            .filter(|&f| world.check_building(player, f, size).is_ok())
            .min_by_key(|&f| {
                let d = grid::distance(f, hq);
                let wobble = jitter[(f.0 + f.1) as usize % jitter.len()];
                // Military buildings at the edge, the rest near the middle.
                let key = if military { 100 - d.min(100) } else { d };
                (key + wobble, f)
            });
        let Some(at) = best else {
            self.next += 1;
            return out;
        };
        self.next += 1;
        out.push(Command::PlaceBuilding {
            player,
            building: id,
            at,
        });
        if let Some(flag) = world.terrain().neighbour(at, FLAG_DIRECTION) {
            if out.iter().all(|c| !matches!(c, Command::StartRoute { .. })) {
                out.extend(self.road_from(world, flag, 0));
                self.unjoined.push((at, flag, 1));
            } else {
                self.unjoined.push((at, flag, 0));
            }
        }
        out
    }

    /// A road from the building flag `flag` as the route mode builds it:
    /// to the nearest flag of the network, or (every other try, and a third
    /// of first tries) into the nearest road field where a flag fits.
    fn road_from(&mut self, world: &World, flag: Point, tries: u32) -> Vec<Command> {
        let player = self.player;
        let Some(hq_flag) = world.terrain().neighbour(self.hq, FLAG_DIRECTION) else {
            return Vec::new();
        };
        let into_road = if tries == 0 {
            self.random(3) == 0
        } else {
            tries % 2 == 1
        };
        let mut targets: Vec<Point> = if into_road {
            world
                .roads()
                .filter(|r| r.owner == player && !r.water && r.path.len() > 3)
                .filter(|r| world.route_cost(player, hq_flag, r.path[0]).is_some())
                .flat_map(|r| r.path[1..r.path.len() - 1].iter().copied())
                .filter(|&f| world.check_flag(player, f).is_ok())
                .collect()
        } else {
            world
                .flags()
                .filter(|&(f, owner)| owner == player && f != flag)
                .map(|(f, _)| f)
                .filter(|&f| world.route_cost(player, hq_flag, f).is_some())
                .collect()
        };
        targets.sort_by_key(|&f| (grid::distance(flag, f), f));
        let skip = (tries / 2) as usize;
        let Some(&to) = targets.get(skip).or(targets.first()) else {
            return Vec::new();
        };
        vec![
            Command::StartRoute {
                player,
                at: flag,
                waterway: false,
            },
            Command::ExtendRoute { player, to },
        ]
    }
}
