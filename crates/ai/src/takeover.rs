//! FMT-SAV-3.10a to 3.10g: a computer player's own records in an original's
//! save, taken over into [`crate::AiPlayer`], so it plays on with its plan
//! rather than starting afresh on the restored world.
//!
//! The activation comes first: it makes what follows from the world (the
//! building records, the resource and military maps). Here the records it
//! cannot know replace its first guesses: needs and productions, cells with
//! their markers, expansion targets and attempt counters, the territory map's
//! counters, the conquered continents, the sources' pending orders, what the
//! periodic parts have reached (3.10h) and the think counter.

use std::collections::BTreeMap;

use s2_assets::s2m::FieldPos;
use s2_assets::sav::{SaveGame, SavedAiMarker, SavedAiOrder, SavedAiTarget, SavedComputer};
use s2_sim::grid::Point;
use s2_sim::{PlayerId, World};

use crate::cells::{Cell, CellPos, Cells, FullMark};
use crate::construction::{Order, Slot, Tag};
use crate::expansion::{Kind, Source, Target};
use crate::plan::{Need, NeedId, Plan, ProdId, Production};
use crate::resources::{SURFACE, UNDERGROUND};
use crate::sites::Area;
use crate::{AiPlayer, AiSystem};

/// FMT-SAV-3.10b: the id of no object.
const NONE: u64 = u64::MAX;

/// What [`AiSystem::take_over`] took over, for the report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TakenOver {
    pub players: usize,
    pub needs: usize,
    pub productions: usize,
    pub cells: usize,
    pub orders: usize,
}

/// A saved field cursor as a point.
fn point(f: FieldPos) -> Point {
    (f.x, f.y)
}

/// A cell pair as a cell; (-1, -1) is none.
fn cell_pos(pair: (i32, i32)) -> Option<CellPos> {
    Some((u32::try_from(pair.0).ok()?, u32::try_from(pair.1).ok()?))
}

/// The objects a player's refs name.
struct Refs {
    /// A building's id: its field, if the world has it.
    buildings: BTreeMap<u64, Point>,
    /// An `Ai Cell`'s id: its cell.
    cells: BTreeMap<u64, CellPos>,
    needs: BTreeMap<u64, NeedId>,
    prods: BTreeMap<u64, ProdId>,
    /// The game's seconds and tick now, for times kept as ticks.
    seconds: f32,
    tick: u64,
}

impl Refs {
    /// The tick a time in game seconds stood for (FMT-SAV-3.10b keeps
    /// seconds; ten ticks a second, ECO-TIME-1.4).
    fn tick_of(&self, seconds: f32) -> u64 {
        let ago = ((self.seconds - seconds).max(0.0) * 10.0) as u64;
        self.tick.saturating_sub(ago)
    }

    fn marker(&self, m: &SavedAiMarker) -> Option<u64> {
        m.set.then(|| self.tick_of(m.time))
    }

    fn building(&self, id: u64) -> Option<Point> {
        self.buildings.get(&id).copied()
    }

    /// FMT-SAV-3.10b: a target; kind 0 is none, whatever it still holds.
    fn target(&self, t: Option<&SavedAiTarget>) -> Option<Target> {
        let t = t?;
        let kind = match t.kind {
            1 => Kind::Position,
            2 => Kind::Crowded,
            3 => Kind::Anywhere,
            _ => return None,
        };
        Some(Target {
            kind,
            at: point(t.at?),
            since: self.tick_of(t.time),
        })
    }

    /// FMT-SAV-3.10b: an order; the source's slot holds one only while it
    /// names a building type (AI-CON-5.1: its failure or success handler
    /// clears the slot, which leaves the rest of the record as it was). The
    /// type's id is a `u32` hash the record writes as `i32`, with -1 none.
    fn order(&self, saved: Option<&SavedAiOrder>, slot: Slot, tag: Tag) -> Option<Order> {
        let o = saved?;
        if o.building == -1 {
            return None;
        }
        let area = match (o.mode, o.point) {
            (1, Some(at)) => Area::Point {
                at: point(at),
                radius: o.radius,
            },
            _ => Area::Cell(cell_pos(o.cell)?),
        };
        Some(Order {
            slot,
            area,
            building: o.building as u32,
            weight: o.weight,
            time: o.time,
            tag,
        })
    }
}

impl AiSystem {
    /// FMT-SAV-3.10: the records of `save`'s computer players into those
    /// [`AiSystem::activate_all`] made on the world restored from it, and
    /// the think counter (AI-2.1).
    pub fn take_over(&mut self, world: &World, save: &SaveGame) -> TakenOver {
        let mut report = TakenOver::default();
        if let Some(think) = save.ai_think {
            self.counter = u64::from(think);
        }
        for saved in &save.computer_players {
            let slot = usize::try_from(saved.slot).unwrap_or(usize::MAX);
            let Some(player) = self.players.get_mut(slot).and_then(Option::as_mut) else {
                continue;
            };
            let owner = PlayerId(saved.slot as u8);
            let buildings = save
                .buildings
                .iter()
                .map(|b| (b.id, point(b.field)))
                .filter(|&(_, at)| world.building_at(at).is_some_and(|b| b.owner == owner))
                .collect();
            take_over_player(player, &self.config, world, saved, buildings, &mut report);
            report.players += 1;
        }
        report
    }
}

fn take_over_player(
    ai: &mut AiPlayer,
    config: &crate::AiConfig,
    world: &World,
    saved: &SavedComputer,
    buildings: BTreeMap<u64, Point>,
    report: &mut TakenOver,
) {
    // Ids of ours for both lists, in the order the file names them.
    let mut all_needs: Vec<_> = saved.needs.iter().chain(&saved.food_needs).collect();
    all_needs.sort_by_key(|n| n.id);
    let needs: BTreeMap<u64, NeedId> = all_needs
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, i as NeedId))
        .collect();
    let prods: BTreeMap<u64, ProdId> = saved
        .productions
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, (all_needs.len() + i) as ProdId))
        .collect();
    let refs = Refs {
        buildings,
        cells: saved
            .cells
            .iter()
            .filter_map(|c| Some((c.id, cell_pos(c.pos)?)))
            .collect(),
        needs,
        prods,
        seconds: world.game_seconds(),
        tick: world.tick(),
    };

    // FMT-SAV-3.10d, AI-CON-8.3 b: a consumer's needs in its list's order.
    let mut ties: BTreeMap<u64, u32> = BTreeMap::new();
    let mut tied = 0;
    for b in &saved.buildings {
        for &need in &b.needs {
            tied += 1;
            ties.insert(need, tied);
        }
    }
    let plan_needs: Vec<Need> = all_needs
        .iter()
        .map(|n| {
            let square = cell_pos(n.square).map(|sq| {
                // AI-CON-3.4: the record names the square; its kind is what
                // the square holds of the good (AI-CON-4.3).
                let kind = if ai.resources.amount(sq, n.good, UNDERGROUND) > 0 {
                    UNDERGROUND
                } else {
                    SURFACE
                };
                (sq, kind)
            });
            Need {
                id: refs.needs[&n.id],
                good: n.good,
                consumer: refs.building(n.consumer),
                cell: refs.cells.get(&n.cell).copied(),
                square,
                production: refs.prods.get(&n.production).copied(),
                waiting: n.waiting,
                food: refs.needs.get(&n.food).copied(),
                created: refs.seconds,
                tied: ties.get(&n.id).copied().unwrap_or(0),
                again: false,
            }
        })
        .collect();
    let plan_prods: Vec<Production> = saved
        .productions
        .iter()
        .filter_map(|p| {
            Some(Production {
                id: refs.prods[&p.id],
                good: p.good,
                cell: refs.cells.get(&p.cell).copied()?,
                building: refs.building(p.building),
                need: refs.needs.get(&p.need).copied(),
            })
        })
        .collect();
    let waiting = saved
        .waiting
        .iter()
        .filter_map(|id| refs.needs.get(id).copied())
        .collect();
    report.needs += plan_needs.len();
    report.productions += plan_prods.len();
    ai.plan = Plan::restored(plan_needs, plan_prods, waiting, tied);

    // FMT-SAV-3.10e: the cells in creation order, each with its markers.
    let cells: Vec<Cell> = saved
        .cells
        .iter()
        .filter_map(|c| {
            let pos = cell_pos(c.pos)?;
            let mut ids: Vec<u64> = [&c.depots, &c.ordinary, &c.military, &c.harbours]
                .into_iter()
                .flatten()
                .copied()
                .chain([c.forester, c.hunter])
                .filter(|&id| id != NONE)
                .collect();
            // A building's id is its place in the order it was made.
            ids.sort_unstable();
            Some(Cell {
                pos,
                buildings: ids.iter().filter_map(|&id| refs.building(id)).collect(),
                forester: refs.building(c.forester),
                hunter: refs.building(c.hunter),
                full: c
                    .full
                    .iter()
                    .map(|&(good, marker, count)| {
                        (
                            good,
                            FullMark {
                                time: marker.time,
                                count,
                                active: marker.set,
                            },
                        )
                    })
                    .collect(),
                depot_wanted: refs.marker(&c.depot_wanted),
                crowded: refs
                    .marker(&c.crowded)
                    .map(|tick| (tick, c.crowded.weight as u32)),
                // FMT-SAV-3.10h, AI-CON-9.4: nothing ever sets the clearing marker
                // or its size.
                clearing: None,
                harbour_wanted: refs.marker(&c.harbour_wanted),
                position: c.centre.map(point),
                landmass: c.landmass as i32,
            })
        })
        .collect();
    report.cells += cells.len();
    let cols = ai.cells.cols;
    let cursor = cell_pos(saved.cell_scan).map_or(0, |(x, y)| y * cols + x);
    ai.cells = Cells::new(world.terrain().width, world.terrain().height).restored(cells, cursor);
    // A building the records of the save leave out of its cell (none in the
    // samples) is still in the cell of its own record.
    let records: Vec<(Point, CellPos)> = ai.records.values().map(|r| (r.at, r.cell)).collect();
    for (at, cell) in records {
        let entry = ai.cells.ensure(cell, refs.tick);
        if !entry.buildings.contains(&at) {
            entry.buildings.push(at);
        }
    }

    // FMT-SAV-3.10a: the expansion attempts, one per 16 x 16 cell, row by row.
    ai.attempts = saved
        .attempts
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n != 0)
        .map(|(i, &n)| ((i as u32 % cols, i as u32 / cols), n))
        .collect();
    // FMT-SAV-3.10g.
    ai.conquered = saved.conquered.iter().map(|&l| l as i32).collect();
    // FMT-SAV-3.10e, AI-MIL-1.3.
    let (w, _, territory) = &saved.territory;
    if *w == ai.territory.cols {
        for (i, e) in territory.iter().enumerate() {
            let pos = (i as u32 % w, i as u32 / w);
            if let Some(t) = ai.territory.get_mut(pos) {
                t.penalty = e.penalty as f32;
                t.result = e.results as f32;
                t.catapults = e.catapults;
                t.attempts = e.attempts as f32;
            }
        }
    }

    // FMT-SAV-3.10a, c, e, f, g, AI-MIL-3.1: each source's slot keeps its
    // target until it is finished or cancelled.
    let sources = [
        (Source::WaitingNeed, saved.waiting_target.as_ref()),
        (Source::Attack, saved.attack_target.as_ref()),
        (Source::Crowded, saved.cell_target.as_ref()),
        (Source::Anywhere, saved.needs_target.as_ref()),
    ];
    for (source, target) in sources {
        ai.expansion.pending[source as usize] = refs.target(target);
    }
    // AI-MIL-3.1: the current target is the copy of its source's; its source
    // is the slot holding the same target.
    let source_of = |t: Target| {
        sources
            .iter()
            .map(|&(source, _)| source)
            .find(|&s| ai.expansion.pending[s as usize] == Some(t))
    };
    let p = &saved.progress;
    // FMT-SAV-3.10h: the source of the target in progress, where the file names
    // one (0: none).
    let named = match p.expansion_source {
        1 => Some(Source::WaitingNeed),
        2 => Some(Source::Crowded),
        3 => Some(Source::Anywhere),
        4 => Some(Source::Attack),
        _ => None,
    };
    let current = refs
        .target(saved.current_target.as_ref())
        .and_then(|t| Some((named.or_else(|| source_of(t))?, t)));
    let taken = refs
        .target(saved.other_target.as_ref())
        .and_then(|t| Some((source_of(t)?, t)));
    // AI-MIL-3.2: a military run remembered the current target.
    let remembered =
        current.is_some_and(|(_, t)| refs.target(saved.remembered_target.as_ref()) == Some(t));
    ai.expansion.current = current;
    ai.expansion.taken = taken;
    ai.expansion.remembered = remembered;

    // FMT-SAV-3.10a, c, e, f: the pending orders.
    let o = &saved.orders;
    // FMT-SAV-3.10h: a pending military order's attempts are the record's, its
    // cell and source are in the order and the target; its level is worked out
    // as when an order is made (AI-MIL-4.3). Without a target in progress there
    // is nothing for it to be for.
    let military_tag = ai.expansion.current.and_then(|(source, target)| {
        let order = o.military.as_ref()?;
        let site = match (order.mode, order.point) {
            (1, Some(at)) => point(at),
            _ => crate::cells::origin(cell_pos(order.cell)?),
        };
        let cell = match target.kind {
            Kind::Crowded => crate::maps::mcell_of(target.at),
            _ => crate::maps::mcell_of(site),
        };
        let level = crate::military::level(
            crate::military::enemy_distance(world, ai.id, site),
            ai.underground_near(site, config.names.fish),
            ai.econ.stock(config.names.stone) > 40,
            None,
        );
        Some(Tag::Military {
            attempts: p.military_attempts,
            level,
            cell,
            source,
        })
    });
    let need_tag = refs
        .needs
        .get(&saved.order_need)
        .map(|&need| Tag::Need { need });
    ai.build.slots = Default::default();
    ai.build.exec = None;
    let orders = [
        need_tag.and_then(|tag| refs.order(o.needs.as_ref(), Slot::Needs, tag)),
        refs.order(o.clearing.as_ref(), Slot::Cell, Tag::None),
        military_tag.and_then(|tag| refs.order(o.military.as_ref(), Slot::Military, tag)),
        refs.order(o.forester.as_ref(), Slot::Forester, Tag::None),
        refs.order(o.catapult.as_ref(), Slot::Catapult, Tag::None),
        refs.order(o.depot.as_ref(), Slot::Depot, Tag::None),
        refs.order(o.harbour.as_ref(), Slot::Harbour, Tag::None),
        refs.order(o.ship.as_ref(), Slot::Ship, Tag::None),
        refs.order(o.hunter.as_ref(), Slot::Hunter, Tag::None),
    ];
    for order in orders.into_iter().flatten() {
        report.orders += 1;
        ai.build.slots[order.slot as usize] = Some(order);
    }
    // FMT-SAV-3.10c, AI-CON-7.2.
    ai.hunter_index = saved.hunter_cursor as usize;
    // FMT-SAV-3.10h: what the periodic parts have reached.
    ai.need_stage = u8::try_from(p.need_stage).unwrap_or(u8::MAX);
    ai.stock_check = p.stock_check;
    ai.forester_index = p.forester_index as usize;
    ai.flag_cursor = p.destruction_flag;
    // AI-CON-9.2: the file names the last building; our cursor is the serial to
    // look at next.
    ai.destruction_cursor = refs
        .buildings
        .get(&p.destruction_building)
        .and_then(|at| ai.records.get(at))
        .map_or(0, |r| r.serial.wrapping_add(1));
    // AI-CON-10.1: the next good after the entry the file names, which may be a
    // profession, in the order the goods are walked.
    let goods = AiPlayer::control_order(&config.goods);
    let last = p.control_entry as i32;
    ai.control_cursor = goods.iter().position(|&g| g as i32 > last).unwrap_or(0);
    // AI-CON-10.2: the last tool handled, by its slot.
    ai.locksmith_last = config
        .tool_slots
        .iter()
        .position(|&tool| tool == p.last_tool)
        .unwrap_or(usize::MAX);
    ai.goods_step = p.goods_step;
    ai.military_cooldown = p.military_cooldown;
    ai.settings_countdown = p.settings_countdown;
    let gate = |(k, t0): (u32, f32)| crate::military::Gate { t0, k };
    ai.weapon_gate = gate(p.weapon_gate);
    ai.coin_gate = gate(p.coin_gate);
    ai.upgrade_cursor = p.upgrade_cursor;
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_assets::sav::{
        SavedAiBuilding, SavedAiCell, SavedAiNeed, SavedAiOrders, SavedAiProduction,
        SavedAiProgress, SavedAiTerritory, SavedBuilding,
    };

    const NOT_SET: SavedAiMarker = SavedAiMarker {
        set: false,
        time: 0.0,
        weight: 0.0,
    };

    fn at(p: Point) -> FieldPos {
        FieldPos { x: p.0, y: p.1 }
    }

    fn building(id: u64, b: &s2_sim::Building) -> SavedBuilding {
        SavedBuilding {
            building_type: b.building_type,
            owner: u32::from(b.owner.0),
            id,
            field: at(b.at),
            held: Vec::new(),
            stock: Vec::new(),
            workers: Vec::new(),
            construction_left: [0.0; 2],
            delivered: Vec::new(),
            production_on: true,
            size_class: 0,
            door_flag: 0,
            levelled: 1.0,
            soldiers: Vec::new(),
            harbour: false,
            upgrade: None,
            reserved: None,
        }
    }

    fn order(mode: u32, cell: (i32, i32), point: Option<Point>, building: i32) -> SavedAiOrder {
        SavedAiOrder {
            mode,
            cell,
            point: point.map(at),
            radius: 12,
            building,
            weight: 10,
            time: 95.5,
        }
    }

    /// A game of two computer players after 2000 ticks, and a save that says
    /// something else of player 0's records than its own play did.
    fn game() -> (World, AiSystem, SaveGame) {
        let (mut world, config) = crate::fixture::named("town2", 7).expect("the fixture");
        let mut ai = AiSystem::new(config, &world);
        let slots = crate::every_player(&world);
        ai.activate_all(&mut world, &slots);
        for _ in 0..2000 {
            ai.step(&mut world);
        }
        let mine: Vec<s2_sim::Building> = world
            .buildings()
            .filter(|b| b.owner == PlayerId(0))
            .copied()
            .collect();
        let castle = mine[0];
        let cell = crate::cells::cell_of(castle.at);
        let cell_ref = (cell.0 as i32, cell.1 as i32);
        let good = crate::fixture::PLANK;
        let computer = SavedComputer {
            slot: 0,
            profile: 2,
            home: Some(at(castle.at)),
            orders: SavedAiOrders {
                // A slot whose handler cleared it keeps the rest.
                forester: Some(order(0, cell_ref, None, -1)),
                needs: Some(order(0, cell_ref, None, crate::fixture::SAWMILL as i32)),
                depot: Some(order(
                    1,
                    (-1, -1),
                    Some(castle.at),
                    crate::fixture::DEPOT as i32,
                )),
                ..SavedAiOrders::default()
            },
            order_need: 21,
            attempts: {
                let mut a = vec![
                    0;
                    (world.terrain().width.div_ceil(16) * world.terrain().height.div_ceil(16))
                        as usize
                ];
                a[world.terrain().width.div_ceil(16) as usize + 1] = 3;
                a
            },
            buildings: vec![SavedAiBuilding {
                id: 100,
                building: 100,
                cell: 200,
                needs: vec![22, 21],
                productions: vec![],
            }],
            needs: vec![
                SavedAiNeed {
                    id: 21,
                    good,
                    cell: 200,
                    production: 31,
                    consumer: 100,
                    square: (1, 2),
                    waiting: false,
                    food: NONE,
                    need: NONE,
                },
                SavedAiNeed {
                    id: 23,
                    good: crate::fixture::WOOD,
                    cell: NONE,
                    production: NONE,
                    consumer: NONE,
                    square: (-1, -1),
                    waiting: true,
                    food: NONE,
                    need: 22,
                },
            ],
            food_needs: vec![SavedAiNeed {
                id: 22,
                good: crate::fixture::BREAD,
                cell: NONE,
                production: NONE,
                consumer: 100,
                square: (-1, -1),
                waiting: false,
                food: 23,
                need: NONE,
            }],
            productions: vec![SavedAiProduction {
                id: 31,
                good,
                cell: 200,
                building: 100,
                need: 21,
            }],
            territory: (
                world.terrain().width.div_ceil(8),
                world.terrain().height.div_ceil(8),
                vec![
                    SavedAiTerritory {
                        building: NONE,
                        penalty: 19,
                        results: 2,
                        catapults: 1,
                        attempts: 5,
                    };
                    2
                ],
            ),
            cells: vec![SavedAiCell {
                id: 200,
                pos: cell_ref,
                centre: Some(at(castle.at)),
                depots: vec![100],
                ordinary: vec![],
                military: vec![],
                productions: vec![31],
                needs: vec![21],
                crowded: SavedAiMarker {
                    set: true,
                    time: world.game_seconds() - 10.0,
                    weight: 100.0,
                },
                depot_wanted: NOT_SET,
                full: vec![(
                    good,
                    SavedAiMarker {
                        set: true,
                        time: 50.0,
                        weight: 1.0,
                    },
                    2,
                )],
                clearing: NOT_SET,
                clear_size: 0,
                forester: NONE,
                harbours: vec![],
                landmass: 7,
                harbour_wanted: NOT_SET,
                hunter: NONE,
            }],
            cell_target: Some(SavedAiTarget {
                kind: 2,
                at: Some(at(castle.at)),
                time: 90.0,
            }),
            needs_target: Some(SavedAiTarget {
                kind: 0,
                at: Some(at(castle.at)),
                time: 80.0,
            }),
            current_target: Some(SavedAiTarget {
                kind: 2,
                at: Some(at(castle.at)),
                time: 90.0,
            }),
            other_target: None,
            remembered_target: Some(SavedAiTarget {
                kind: 2,
                at: Some(at(castle.at)),
                time: 90.0,
            }),
            waiting: vec![23],
            waiting_target: None,
            attack_target: None,
            hunter_cursor: 4,
            cell_scan: (1, 1),
            conquered: vec![7],
            progress: SavedAiProgress::default(),
        };
        let save = SaveGame {
            buildings: vec![building(100, &castle)],
            computer_players: vec![computer],
            ai_think: Some(2345),
            ..SaveGame::default()
        };
        (world, ai, save)
    }

    // FMT-SAV-3.10h (Q-1000): the values of the periodic parts are taken over; a
    // pending military order keeps its attempts, with its cell and source from the
    // order and the target; AI-CON-9.4's clearing stays off.
    // spec: FMT-SAV-3.10h, AI-CON-9.4, AI-MIL-4.1, AI-MIL-4.4, AI-MIL-2.2, AI-MIL-6.2, AI-MIL-6.3
    #[test]
    fn a_saved_computer_players_progress_is_taken_over() {
        let (world, mut ai, mut save) = game();
        let tool = ai.config.tool_slots.get(1).copied();
        let goods = AiPlayer::control_order(&ai.config.goods);
        let saved = &mut save.computer_players[0];
        let target = saved.remembered_target;
        saved.current_target = target;
        saved.waiting_target = target;
        let castle_at = saved.home.expect("home");
        saved.orders.military = Some(order(1, (0, 0), Some(point(castle_at)), 7));
        saved.progress = SavedAiProgress {
            expansion_source: 1,
            destruction_flag: 71,
            destruction_building: 100,
            need_stage: 4,
            stock_check: 8,
            last_tool: tool.unwrap_or(0),
            goods_step: 6,
            control_entry: goods.first().copied().unwrap_or(0),
            forester_index: 3,
            military_cooldown: 2,
            military_attempts: 2,
            settings_countdown: 31,
            weapon_gate: (4, 186.4),
            coin_gate: (2, 90.5),
            upgrade_cursor: 9,
        };
        ai.take_over(&world, &save);
        let p = ai.player(PlayerId(0)).expect("player 0");
        assert_eq!((p.need_stage, p.stock_check), (4, 8));
        assert_eq!(p.forester_index, 3);
        assert_eq!(p.flag_cursor, 71);
        let castle = point(castle_at);
        assert_eq!(
            p.destruction_cursor,
            p.records[&castle].serial.wrapping_add(1)
        );
        assert_eq!(p.control_cursor, usize::from(goods.len() > 1));
        assert_eq!(
            p.locksmith_last,
            if tool.is_some() { 1 } else { usize::MAX }
        );
        assert_eq!(p.goods_step, 6);
        assert_eq!(p.military_cooldown, 2);
        assert_eq!(p.settings_countdown, 31);
        assert_eq!((p.weapon_gate.k, p.weapon_gate.t0), (4, 186.4));
        assert_eq!((p.coin_gate.k, p.coin_gate.t0), (2, 90.5));
        assert_eq!(p.upgrade_cursor, 9);
        let (source, _) = p.expansion.current.expect("the target in progress");
        assert_eq!(source, Source::WaitingNeed);
        let military = p.build.slots[Slot::Military as usize].expect("the military order");
        assert_eq!(military.building, 7);
        let Tag::Military {
            attempts,
            source,
            cell,
            ..
        } = military.tag
        else {
            panic!("{:?}", military.tag);
        };
        assert_eq!((attempts, source), (2, Source::WaitingNeed));
        assert_eq!(cell, crate::maps::mcell_of(castle));
    }

    // spec: FMT-SAV-3.10, FMT-SAV-3.10a, FMT-SAV-3.10b, FMT-SAV-3.10d, FMT-SAV-3.10e, FMT-SAV-3.10g, AI-MIL-3.1, AI-MIL-3.2
    #[test]
    fn a_saved_computer_players_records_replace_its_first_guesses() {
        let (world, mut ai, save) = game();
        let report = ai.take_over(&world, &save);
        assert_eq!(
            report,
            TakenOver {
                players: 1,
                needs: 3,
                productions: 1,
                cells: 1,
                orders: 2,
            }
        );
        assert_eq!(ai.counter(), 2345);
        let castle_at = save.computer_players[0].home.map(point).expect("home");
        let cell = crate::cells::cell_of(castle_at);
        let p = ai.player(PlayerId(0)).expect("player 0");
        // Needs in the order of their ids, with the refs turned into places.
        let ids: Vec<u32> = p.plan.needs.iter().map(|n| n.good).collect();
        assert_eq!(
            ids,
            [
                crate::fixture::PLANK,
                crate::fixture::BREAD,
                crate::fixture::WOOD
            ]
        );
        let (planks, bread, wood) = (&p.plan.needs[0], &p.plan.needs[1], &p.plan.needs[2]);
        assert_eq!(planks.consumer, Some(castle_at));
        assert_eq!(planks.cell, Some(cell));
        assert_eq!(planks.square.map(|s| s.0), Some((1, 2)));
        assert_eq!(planks.production, Some(p.plan.prods[0].id));
        // AI-CON-8.3 b: the consumer's list says bread before planks.
        assert!(bread.tied < planks.tied && bread.tied > 0);
        assert_eq!(wood.tied, 0);
        // The food need's link, the waiting list.
        assert_eq!(bread.food, Some(wood.id));
        assert!(wood.waiting);
        assert_eq!(p.plan.waiting, [wood.id]);
        assert_eq!(p.plan.prods[0].building, Some(castle_at));
        assert_eq!(p.plan.prods[0].need, Some(planks.id));
        // The saved cell, markers and all; the other buildings of the player
        // are in the cells of their records.
        assert_eq!(p.cells.in_order()[0], cell);
        let c = p.cells.get(cell).expect("the cell");
        assert_eq!(c.position, Some(castle_at));
        assert_eq!(c.landmass, 7);
        assert_eq!(c.depot_wanted, None);
        let (crowded, weight) = c.crowded.expect("crowded");
        assert_eq!(weight, 100);
        assert!(world.tick() - crowded <= 101, "10 s ago: {crowded}");
        let full = c.full[&crate::fixture::PLANK];
        assert_eq!((full.time, full.count, full.active), (50.0, 2, true));
        for rec in p.records.values() {
            assert!(
                p.cells
                    .get(rec.cell)
                    .is_some_and(|c| c.buildings.contains(&rec.at)),
                "{:?}",
                rec.at
            );
        }
        // Counters.
        assert_eq!(p.attempts.get(&(1, 1)), Some(&3));
        assert_eq!(p.attempts.len(), 1);
        assert_eq!(p.conquered, [7]);
        let t = p.territory.get((1, 0));
        assert_eq!(
            (t.penalty, t.result, t.catapults, t.attempts),
            (19.0, 2.0, 1, 5.0)
        );
        assert_eq!(p.hunter_index, 4);
        // Kind 0 is no target, whatever the record still holds; the current
        // target comes with its source, a military run remembered it.
        use crate::expansion::{Kind, Source};
        assert!(p.expansion.pending[Source::Anywhere as usize].is_none());
        let crowded = p.expansion.pending[Source::Crowded as usize].expect("crowded target");
        assert_eq!((crowded.kind, crowded.at), (Kind::Crowded, castle_at));
        assert_eq!(p.expansion.current, Some((Source::Crowded, crowded)));
        assert!(p.expansion.remembered);
        // Orders: a slot only while it names a type.
        let needs = p.build.slots[Slot::Needs as usize].as_ref().expect("needs");
        assert_eq!(needs.building, crate::fixture::SAWMILL);
        assert_eq!(needs.time, 95.5);
        assert!(matches!(needs.area, Area::Cell(c) if c == cell));
        assert!(matches!(needs.tag, Tag::Need { need } if need == planks.id));
        let depot = p.build.slots[Slot::Depot as usize].as_ref().expect("depot");
        assert!(matches!(depot.area, Area::Point { at, radius: 12 } if at == castle_at));
        assert!(p.build.slots[Slot::Forester as usize].is_none());
        // The other computer player is untouched.
        assert!(!ai
            .player(PlayerId(1))
            .expect("player 1")
            .plan
            .needs
            .is_empty());
    }

    // spec: FMT-SAV-3.10a, AI-2.1
    #[test]
    fn taken_over_it_plays_on_and_a_save_of_it_loads_the_same() {
        let (mut world, mut ai, save) = game();
        ai.take_over(&world, &save);
        let mut checker = s2_sim::invariants::Checker::new(&world);
        for _ in 0..1500 {
            ai.step(&mut world);
            let broken = checker.check(&world);
            assert!(broken.is_empty(), "{broken:?}");
        }
        assert_eq!(ai.counter(), 2345 + 1500);
        // Our own save of the game goes on the same.
        let mut w = s2_sim::snap::Writer::new();
        ai.put_state(&mut w);
        let mut again = AiSystem::new(ai.config().clone(), &world);
        again
            .get_state(&mut s2_sim::snap::Reader::new(&w.bytes))
            .expect("the state");
        assert_eq!(again.state_hash(), ai.state_hash());
    }
}
