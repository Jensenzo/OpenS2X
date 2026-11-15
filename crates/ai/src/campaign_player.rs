//! A person playing a mission to its gate (`docs/test-coverage.md`, "Missions"):
//! a wood and stone economy so that planks and stones do not run out, military
//! buildings towards the gate with their soldiers sent home where no enemy is
//! near, attacks on what stands in the way. Or the same beside a computer player
//! that runs the economy and the fighting (`helped`), which is what brings food,
//! mines and new soldiers. Only the commands a player can give.
//!
//! This is a test's player, not a computer player: nothing in the game runs it.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use s2_rules::RuleSet;
use s2_sim::grid::{self, Point};
use s2_sim::map::state;
use s2_sim::{BuildingSize, Command, PlayerId, World};

/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;
/// A new military building keeps this far from the others.
const MIN_SPACING: u32 = 4;
/// Flags tried, nearest first, for the road of a new building.
const ROAD_TRIES: usize = 8;
/// Fields of road a carrier may have to itself before it gets a flag.
const LONGEST_ROAD: usize = 4;
/// How far a woodcutter or stonecutter is taken to reach (a look at the map,
/// not the working radius of its type).
const WORK_REACH: u32 = 6;
/// Ticks between two looks for an attack.
const ATTACK_EVERY: u64 = 100;
/// Ticks a site beside a computer player may take to claim its land.
const STUCK_AFTER: u64 = 8_000;
/// MIL-SOL-3.2: from here on a building counts as near no enemy.
const HOME_FROM_ENEMY: i32 = 18;
/// Military buildings under way at once.
const SITES_AT_ONCE: usize = 2;
/// Roads may not cross these (TRN-ROAD-1.1). Trees and stones are not in the
/// list: they stand there only until they are felled or dug away.
const NOT_LAND: u32 = state::WATER
    | state::BLOCKED
    | state::MOVEMENT_BLOCKED
    | state::DOODAD_BLOCKED
    | state::ELEVATION_BLOCKED;

/// What the player has done, for the report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CampaignLog {
    /// Buildings placed, by type: how many.
    pub placed: BTreeMap<u32, u32>,
    pub roads: u32,
    pub flags: u32,
    pub given_up: u32,
    /// Attacks ordered: the tick, the target and the soldiers sent.
    pub attacks: Vec<(u64, Point, u32)>,
    /// Orders the game refused, by reason.
    pub refused: BTreeMap<String, u32>,
}

/// One kind of building the player uses.
#[derive(Debug, Clone, Copy)]
struct Kind {
    id: u32,
    size: BuildingSize,
    planks: u32,
    stones: u32,
    range: u32,
}

/// The kinds of building and the goods it thinks about.
#[derive(Debug, Clone)]
struct Kinds {
    woodcutter: Kind,
    forester: Kind,
    quarry: Kind,
    sawmill: Kind,
    /// The military buildings, the smallest first.
    military: Vec<Kind>,
    headquarters: u32,
    wood: u32,
    plank: u32,
    stone: u32,
}

impl Kinds {
    fn new(world: &World, rules: &RuleSet) -> Option<Self> {
        let plank = rules.id_named("plank")?;
        let stone = rules.id_named("stone")?;
        let kind = |name: &str| -> Option<Kind> {
            let id = rules.id_named(name)?;
            let building = rules.buildings.iter().find(|b| b.id == id)?;
            let cost = |good: u32| {
                building
                    .frame_costs
                    .iter()
                    .chain(&building.costs)
                    .filter(|a| a.good == good)
                    .map(|a| a.count)
                    .sum()
            };
            let kind = world.building_type(id)?;
            Some(Kind {
                id,
                size: kind.size,
                planks: cost(plank),
                stones: cost(stone),
                range: kind.territory_range,
            })
        };
        Some(Self {
            woodcutter: kind("woodCutter")?,
            forester: kind("forester")?,
            quarry: kind("stonePit")?,
            sawmill: kind("sawMill").or_else(|| kind("sawmill"))?,
            military: ["barracks", "guardHouse", "tower", "fortress"]
                .iter()
                .filter_map(|n| kind(n))
                .collect(),
            headquarters: rules.id_named(s2_rules::HEADQUARTERS)?,
            wood: rules.id_named("wood")?,
            plank,
            stone,
        })
    }
}

/// When the player attacks an enemy's military building it can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attacks {
    /// Never: a computer player beside it does the fighting.
    Never,
    /// With more soldiers than stand in the building.
    Outnumbering,
    /// With whatever is at hand: rank counts more than number (MIL-ATK).
    Always,
    /// With whatever is at hand, first where it outnumbers the building, then
    /// at the weakest; among those the nearest the gate.
    Weakest,
}

/// The player of a mission: see the module's text.
#[derive(Debug, Clone)]
pub struct CampaignPlayer {
    pub player: PlayerId,
    /// The fields whose ownership wins the mission (CAM-2.1).
    pub goals: Vec<Point>,
    pub log: CampaignLog,
    kinds: Kinds,
    /// Sites without a road: not tried again.
    tried: BTreeSet<Point>,
    /// A site's flag: the roads refused for it so far.
    road_tries: BTreeMap<Point, usize>,
    /// Commands were given last tick: read their outcome first.
    waiting: bool,
    /// The military sliders are set.
    set_up: bool,
    next_attack: u64,
    /// A computer player runs the economy and the fighting: this player only
    /// carries the land towards the gate, with the sites in `own_sites`.
    helped: bool,
    own_sites: BTreeSet<Point>,
    pub attacks: Attacks,
    /// Long roads get flags in the middle.
    pub flags_on_roads: bool,
    /// Beside a computer player: a stuck site is given up, and soldiers in any
    /// warehouse count for a new site; else only the headquarters' count, and
    /// it waits for a site as long as it takes.
    pub restless: bool,
    /// Beside a computer player: when each of this player's sites was placed,
    /// to give up one that gets no wares or no soldier.
    placed_at: BTreeMap<Point, u64>,
}

impl CampaignPlayer {
    pub fn new(
        world: &World,
        rules: &RuleSet,
        player: PlayerId,
        goals: Vec<Point>,
    ) -> Option<Self> {
        Some(Self {
            player,
            goals,
            log: CampaignLog::default(),
            kinds: Kinds::new(world, rules)?,
            tried: BTreeSet::new(),
            road_tries: BTreeMap::new(),
            waiting: false,
            set_up: false,
            next_attack: 0,
            helped: false,
            own_sites: BTreeSet::new(),
            attacks: Attacks::Outnumbering,
            flags_on_roads: true,
            restless: true,
            placed_at: BTreeMap::new(),
        })
    }

    /// The same beside a computer player that runs the human's economy and
    /// fighting (`MissionGame::start_with_computer_help`).
    pub fn helped(
        world: &World,
        rules: &RuleSet,
        player: PlayerId,
        goals: Vec<Point>,
    ) -> Option<Self> {
        Some(Self {
            helped: true,
            ..Self::new(world, rules, player, goals)?
        })
    }

    fn headquarters(&self, world: &World) -> Option<Point> {
        world
            .buildings()
            .find(|b| b.owner == self.player && b.building_type == self.kinds.headquarters)
            .map(|b| b.at)
    }

    /// What the headquarters holds of `good`.
    fn stock(&self, world: &World, good: u32) -> u32 {
        world
            .warehouses()
            .filter(|(at, _)| {
                world
                    .building_at(*at)
                    .is_some_and(|b| b.owner == self.player)
            })
            .flat_map(|(_, s)| s.iter())
            .filter(|(g, _)| *g == good)
            .map(|(_, n)| *n)
            .sum()
    }

    /// The military sites this player is waiting for, for the report: the
    /// field, still being built, and how much of it there is.
    pub fn waiting_for(&self, world: &World) -> Vec<String> {
        let hq_flag = self
            .headquarters(world)
            .and_then(|hq| world.terrain().neighbour(hq, FLAG_DIRECTION));
        let network = hq_flag.map(|f| crate::mission::flags_joined_to(world, f));
        world
            .buildings()
            .filter(|b| b.owner == self.player && self.own_sites.contains(&b.at))
            .filter(|b| b.under_construction || world.military_at(b.at).is_some_and(|m| !m.applied))
            .map(|b| {
                let joined = world
                    .terrain()
                    .neighbour(b.at, FLAG_DIRECTION)
                    .is_some_and(|f| network.as_ref().is_some_and(|n| n.contains(&f)));
                format!(
                    "{:?}: building {} joined {joined} progress {:?} soldiers {}",
                    b.at,
                    b.under_construction,
                    world.site(b.at).map(|s| s.progress()),
                    world.present_soldiers(b.at),
                )
            })
            .collect()
    }

    /// The player's commands before the next tick, given to `world`.
    pub fn act(&mut self, world: &mut World) {
        if self.waiting {
            self.waiting = false;
            self.read_results(world);
        }
        let Some(hq) = self.headquarters(world) else {
            return;
        };
        let Some(hq_flag) = world.terrain().neighbour(hq, FLAG_DIRECTION) else {
            return;
        };
        if self
            .goals
            .iter()
            .any(|&g| world.field_owner(g) == Some(self.player))
        {
            return;
        }
        if !self.set_up {
            self.set_up = true;
            // MIL-SOL-7: everything that can go goes.
            world.submit(Command::MilitaryAttackStrength {
                player: self.player,
                thousandths: 1000,
            });
        }
        let helped = self.helped;
        let command = self
            .join_a_site(world, hq_flag)
            .or_else(|| {
                self.flags_on_roads
                    .then(|| self.flag_a_long_road(world))
                    .flatten()
            })
            .or_else(|| self.attack(world))
            .or_else(|| {
                (helped && self.restless)
                    .then(|| self.give_up_a_stuck_site(world))
                    .flatten()
            })
            .or_else(|| (!helped).then(|| self.send_soldiers_home(world)).flatten())
            .or_else(|| (!helped).then(|| self.economy(world, hq)).flatten())
            .or_else(|| self.next_military(world));
        if let Some(command) = command {
            match &command {
                Command::PlaceBuilding { building, at, .. } => {
                    *self.log.placed.entry(*building).or_default() += 1;
                    self.own_sites.insert(*at);
                    self.placed_at.insert(*at, world.tick());
                }
                Command::BuildRoad { .. } => self.log.roads += 1,
                Command::PlaceFlag { .. } => self.log.flags += 1,
                Command::Attack { target, count, .. } => {
                    self.log.attacks.push((world.tick(), *target, *count));
                }
                _ => {}
            }
            world.submit(command);
            self.waiting = true;
        }
    }

    fn read_results(&mut self, world: &World) {
        for result in world.last_results() {
            let Err(reason) = result.outcome else {
                continue;
            };
            let name = match result.command {
                Command::BuildRoad { player, from, .. } if player == self.player => {
                    *self.road_tries.entry(from).or_default() += 1;
                    "road"
                }
                Command::PlaceBuilding { player, at, .. } if player == self.player => {
                    self.tried.insert(at);
                    "building"
                }
                Command::PlaceFlag { player, .. } if player == self.player => "flag",
                Command::Attack { player, .. } if player == self.player => "attack",
                _ => continue,
            };
            *self
                .log
                .refused
                .entry(format!("{name}: {reason:?}"))
                .or_default() += 1;
        }
    }

    /// A building not yet on the road network gets its road, from the nearest
    /// flag on the network; a site with none is given up.
    fn join_a_site(&mut self, world: &World, hq_flag: Point) -> Option<Command> {
        let player = self.player;
        let network = crate::mission::flags_joined_to(world, hq_flag);
        let tried = &self.tried;
        let (helped, own_sites) = (self.helped, &self.own_sites);
        let cut_off = world.buildings().filter(|b| b.owner == player).find(|b| {
            !tried.contains(&b.at)
                && (!helped || own_sites.contains(&b.at))
                && world
                    .terrain()
                    .neighbour(b.at, FLAG_DIRECTION)
                    .is_some_and(|f| !network.contains(&f))
        })?;
        let flag = world.terrain().neighbour(cut_off.at, FLAG_DIRECTION)?;
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
            let at = cut_off.at;
            self.tried.insert(at);
            // A conquered building stands by itself; a site without a road
            // would only keep its plot.
            return cut_off
                .under_construction
                .then_some(Command::RemoveBuilding { player, at });
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

    /// The enemy's military building nearest the goal that can be taken with
    /// the soldiers at hand (MIL-ATK-1.2).
    fn attack(&mut self, world: &World) -> Option<Command> {
        if self.attacks == Attacks::Never || world.tick() < self.next_attack {
            return None;
        }
        self.next_attack = world.tick() + ATTACK_EVERY;
        let player = self.player;
        let goal = *self.goals.first()?;
        let weakest_first = self.attacks == Attacks::Weakest;
        let mut targets: Vec<(u32, u32, Point, u32)> = world
            .military_buildings()
            .filter_map(|(at, _)| {
                let building = world.building_at(at)?;
                if !world.are_enemies(player, building.owner) || building.under_construction {
                    return None;
                }
                let available = world.attack_available(player, at);
                let defenders = world.present_soldiers(at) as u32;
                let enough = match self.attacks {
                    Attacks::Always | Attacks::Weakest => available > 0,
                    _ => available > defenders + 1,
                };
                let short = if weakest_first {
                    (defenders + 1).saturating_sub(available)
                } else {
                    0
                };
                enough.then_some((short, grid::distance(at, goal), at, available))
            })
            .collect();
        targets.sort();
        let &(_, _, target, count) = targets.first()?;
        Some(Command::Attack {
            player,
            target,
            count,
            strong_first: true,
        })
    }

    /// A site of this player's that has claimed no land long after it was placed
    /// is taken down, and its field not tried again: the next goes elsewhere.
    fn give_up_a_stuck_site(&mut self, world: &World) -> Option<Command> {
        let now = world.tick();
        let at = world
            .buildings()
            .filter(|b| b.owner == self.player)
            .filter(|b| b.under_construction || world.military_at(b.at).is_some_and(|m| !m.applied))
            .map(|b| b.at)
            .find(|at| {
                self.placed_at
                    .get(at)
                    .is_some_and(|&placed| now >= placed + STUCK_AFTER)
            })?;
        self.placed_at.remove(&at);
        self.own_sites.remove(&at);
        self.tried.insert(at);
        self.log.given_up += 1;
        Some(Command::RemoveBuilding {
            player: self.player,
            at,
        })
    }

    /// MIL-SOL-3.2, 4.6: a building far from any enemy that has claimed its land
    /// keeps it with nobody in it: its soldiers go back for the next one. So a
    /// few soldiers carry the land a long way, and the ones at the front stay.
    fn send_soldiers_home(&self, world: &World) -> Option<Command> {
        let at = world
            .military_buildings()
            .filter(|(at, _)| {
                world
                    .building_at(*at)
                    .is_some_and(|b| b.owner == self.player && !b.under_construction)
            })
            .filter(|(at, _)| Some(*at) != self.headquarters(world))
            .find(|(_, m)| m.applied && !m.move_out && m.enemy_distance > HOME_FROM_ENEMY)?
            .0;
        Some(Command::ToggleMoveOut {
            player: self.player,
            at,
        })
    }

    /// How many of a building type stand or are being built.
    fn count(&self, world: &World, kind: Kind) -> u32 {
        world
            .buildings()
            .filter(|b| b.owner == self.player && b.building_type == kind.id)
            .count() as u32
    }

    /// The next building of the economy: woodcutters where the trees are, a
    /// sawmill by the headquarters, a quarry at the stones, foresters by the
    /// woodcutters.
    fn economy(&self, world: &World, hq: Point) -> Option<Command> {
        let k = &self.kinds;
        let wanted = [
            (k.woodcutter, 1),
            (k.sawmill, 1),
            (k.quarry, 1),
            (k.woodcutter, 2),
            (k.forester, 1),
            (k.quarry, 2),
            (k.woodcutter, 3),
            (k.forester, 2),
        ];
        for (kind, n) in wanted {
            if self.count(world, kind) >= n {
                continue;
            }
            if self.stock(world, k.plank) < kind.planks {
                return None;
            }
            let at = self.economy_site(world, hq, kind);
            if let Some(at) = at {
                return Some(Command::PlaceBuilding {
                    player: self.player,
                    building: kind.id,
                    at,
                });
            }
        }
        None
    }

    fn economy_site(&self, world: &World, hq: Point, kind: Kind) -> Option<Point> {
        let k = &self.kinds;
        let free = world
            .terrain()
            .fields()
            .filter(|&f| world.field_owner(f) == Some(self.player))
            .filter(|f| !self.tried.contains(f))
            .filter(|&f| world.check_building(self.player, f, kind.size).is_ok());
        // The buildings that harvest stand where the most is in reach; the
        // others as near as they can to what they work with.
        let score: Box<dyn Fn(Point) -> Option<i64>> = if kind.id == k.sawmill.id {
            Box::new(|_| Some(0))
        } else if kind.id == k.woodcutter.id || kind.id == k.quarry.id {
            let (good, least) = if kind.id == k.woodcutter.id {
                (k.wood, 5)
            } else {
                (k.stone, 2)
            };
            let things = deposits_of(world, good);
            Box::new(move |f| {
                let n = things
                    .iter()
                    .filter(|&&t| grid::distance(f, t) <= WORK_REACH)
                    .count() as i64;
                (n >= least).then_some(-n)
            })
        } else {
            let cutters: Vec<Point> = world
                .buildings()
                .filter(|b| b.owner == self.player && b.building_type == k.woodcutter.id)
                .map(|b| b.at)
                .collect();
            Box::new(move |f| {
                cutters
                    .iter()
                    .map(|&c| i64::from(grid::distance(f, c)))
                    .min()
            })
        };
        free.filter_map(|f| Some((score(f)?, grid::distance(f, hq), f)))
            .min()
            .map(|s| s.2)
    }

    /// The next military building: one that takes the gate at once if there
    /// is any, else the cheapest kind on the field of the land nearest the
    /// gate.
    fn next_military(&self, world: &World) -> Option<Command> {
        let player = self.player;
        let k = &self.kinds;
        let military = |kind: u32| {
            world
                .building_type(kind)
                .is_some_and(|t| t.soldier_limit > 0)
        };
        let own: Vec<_> = world
            .buildings()
            .filter(|b| b.owner == player && military(b.building_type))
            .collect();
        let under_way = own
            .iter()
            .filter(|b| !self.helped || self.own_sites.contains(&b.at))
            .filter(|b| b.under_construction || world.military_at(b.at).is_some_and(|m| !m.applied))
            .count();
        if under_way >= if self.helped { 1 } else { SITES_AT_ONCE } {
            return None;
        }
        // MIL-SOL: a building nobody can man takes no land. A computer player
        // beside it also keeps soldiers in other warehouses.
        let hq = self.headquarters(world);
        let soldiers: u32 = world
            .warehouses()
            .filter(|(at, _)| world.building_at(*at).is_some_and(|b| b.owner == player))
            .filter(|(at, _)| self.restless || Some(*at) == hq)
            .map(|(at, _)| world.stock_soldiers(at).iter().sum::<u32>())
            .sum();
        if soldiers == 0 {
            return None;
        }
        // A computer player beside it needs its wares too.
        let spare = if self.helped { 8 } else { 0 };
        let planks = self.stock(world, k.plank).saturating_sub(spare);
        let stones = self.stock(world, k.stone).saturating_sub(spare);
        let locked = |id: u32| {
            world
                .mission()
                .is_some_and(|m| m.locks.contains(&(player, id)))
        };
        let usable: Vec<Kind> = k
            .military
            .iter()
            .copied()
            .filter(|kind| !locked(kind.id))
            .filter(|kind| kind.planks <= planks && kind.stones <= stones)
            .collect();
        if usable.is_empty() {
            return None;
        }
        let terrain = world.terrain();
        let goal = *self.goals.first()?;
        let spaced = |f: Point| own.iter().all(|b| grid::distance(b.at, f) >= MIN_SPACING);
        let open = |f: Point, kind: Kind| world.check_building(player, f, kind.size).is_ok();
        let mine: Vec<Point> = terrain
            .fields()
            .filter(|&f| world.field_owner(f) == Some(player))
            .filter(|f| !self.tried.contains(f))
            .filter(|&f| spaced(f))
            .collect();
        // A field from which a building's land reaches the gate.
        for &kind in &usable {
            let at = mine
                .iter()
                .copied()
                .filter(|&f| grid::distance(f, goal) <= kind.range && open(f, kind))
                .min_by_key(|&f| (grid::distance(f, goal), f));
            if let Some(at) = at {
                return Some(Command::PlaceBuilding {
                    player,
                    building: kind.id,
                    at,
                });
            }
        }
        // Else the cheapest on the way there.
        let kind = usable[0];
        let steps = land_steps_to(world, goal, kind.range);
        let at = mine
            .iter()
            .copied()
            .filter(|&f| open(f, kind))
            .map(|f| {
                let far = steps[terrain.index(f)].unwrap_or(u32::MAX / 2);
                (far, grid::distance(f, goal), f)
            })
            .min()?
            .2;
        Some(Command::PlaceBuilding {
            player,
            building: kind.id,
            at,
        })
    }
}

/// The deposits of the map that give `good`: trees, stone piles.
fn deposits_of(world: &World, good: u32) -> Vec<Point> {
    world
        .deposits()
        .filter(|(_, d)| {
            world
                .deposit_kind(d.kind)
                .is_some_and(|k| k.good == Some(good))
        })
        .map(|(f, _)| f)
        .collect()
}

/// Per field, the steps over land to where a military building's `range`
/// reaches `goal`: what a person sees looking at the map. Trees and stones do
/// not count as walls, water and steep ground do.
pub fn land_steps_to(world: &World, goal: Point, range: u32) -> Vec<Option<u32>> {
    let terrain = world.terrain();
    let mut steps = vec![None; terrain.len()];
    let mut open: BinaryHeap<std::cmp::Reverse<(u32, Point)>> = BinaryHeap::new();
    for field in grid::area(goal, range, terrain.width, terrain.height) {
        if !terrain.has(field, NOT_LAND) {
            open.push(std::cmp::Reverse((0, field)));
            steps[terrain.index(field)] = Some(0);
        }
    }
    while let Some(std::cmp::Reverse((cost, field))) = open.pop() {
        if steps[terrain.index(field)] != Some(cost) {
            continue;
        }
        for next in terrain.neighbours(field).into_iter().flatten() {
            let i = terrain.index(next);
            if terrain.has(next, NOT_LAND) {
                continue;
            }
            if steps[i].is_none_or(|s| s > cost + 1) {
                steps[i] = Some(cost + 1);
                open.push(std::cmp::Reverse((cost + 1, next)));
            }
        }
    }
    steps
}
