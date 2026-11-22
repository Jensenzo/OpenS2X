//! Military buildings, settings, attacks, soldiers, weapons, coins and
//! catapults (`specs/AI-military.md`).

use std::collections::VecDeque;

use s2_sim::grid::Point;
use s2_sim::{Building, Command, PlayerId, World, SOLDIER_RANKS};

use crate::cells::{cell_of, CellPos};
use crate::config::{AiConfig, Mode};
use crate::construction::{Order, Outcome, Slot, Tag};
use crate::expansion::{Kind, Source};
use crate::maps::{mcell_of, morigin, threat_balance, MPos, MilitaryMap};
use crate::player::Category;
use crate::player::{AiPlayer, Ctx};
use crate::resources::{square_of, UNDERGROUND};
use crate::sites::Area;
use crate::view::{dist, flag_of};

/// AI-MIL-2.2: the runs between two changes of the allocation.
const SETTINGS_COUNTDOWN: u32 = 50;
/// AI-MIL-4.1: runs of military building creation after a placement.
const PLACEMENT_COOLDOWN: u32 = 3;
/// AI-MIL-4.1: the order radius by kind.
const RADIUS_POSITION: u32 = 12;
const RADIUS_ANYWHERE: u32 = 20;
/// AI-MIL-4.3: enemy distances.
const NEAR_ENEMY: u32 = 18;
const ODD_DISTANCE: u32 = 24;
/// AI-MIL-4.2: the order in which a cell's 8 neighbours are looked at.
const DIRECTIONS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
/// AI-MIL-1.2 (4.2): the largest threat balance a cell may have.
const MAX_BALANCE: f32 = 40.0;
/// AI-MIL-4.7: coin delivery within this enemy distance.
const COIN_DISTANCE: i32 = 30;
/// AI-MIL-5.2: the lowest target value.
const MIN_TARGET_VALUE: f32 = -100.0;
/// AI-MIL-5.2, 5.3: the radius around the target cell.
const ATTACK_RADIUS: u32 = 12;
/// AI-MIL-5.3: rings of the candidate ranking.
const RANK_RINGS: u32 = 10;
/// AI-MIL-5.4: a warehouse-type target counts as this many defenders.
const WAREHOUSE_DEFENDERS: u32 = 10;
/// AI-MIL-6.2: a mean productivity this high asks for another producer.
const BUSY: f32 = 0.9;
/// AI-MIL-6.2: a producer this idle misses its inputs.
const IDLE: f32 = 0.1;
/// AI-MIL-6.4: the catapult score to beat.
const CATAPULT_SCORE: f32 = 200.0;
/// AI-MIL-6.4: the territory grid has this many cells per cell scanned in a
/// run (at least one is scanned).
const CATAPULT_GRID_DIVISOR: u32 = 600;
const CATAPULT_STONE: u32 = 20;
const CATAPULT_RADIUS: u32 = 8;
const MILITARY_WEIGHT: u32 = 20;

/// AI-MIL-5: what the attack system remembers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attack {
    /// The building being attacked and the territory cell it was chosen in.
    pub target: Option<(Point, MPos)>,
}

/// AI-MIL-6.2: the timing of the weapon (and coin) needs: `t0` in game
/// seconds and the factor `k` (1 when the computer player is activated).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gate {
    pub t0: f32,
    pub k: u32,
}

impl Default for Gate {
    fn default() -> Self {
        Self { t0: 0.0, k: 1 }
    }
}

/// AI-MIL-6.4: the catapult scan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CatapultScan {
    pub cursor: u32,
    pub best: Option<(f32, MPos)>,
}

/// AI-MIL-4.3: the level of a military building at enemy distance `d` before
/// the failed attempts are taken off. Always: +2 if an underground resource
/// (not fish) lies within 4 squares, +1 if stock(stone) > 40. `upgrade`
/// switches to the upgrade check and carries its extra terms: soldiers in
/// stock above 30 and an own finished building within distance < 5 on the
/// same landmass.
pub fn level(
    d: u32,
    underground_near: bool,
    stone_over_40: bool,
    upgrade: Option<(bool, bool)>,
) -> i32 {
    let mut level = if d <= NEAR_ENEMY {
        if upgrade.is_some() {
            2
        } else {
            -1
        }
    } else if d == ODD_DISTANCE {
        1
    } else {
        -2
    };
    level += 2 * i32::from(underground_near) + i32::from(stone_over_40);
    if let Some((soldiers, building)) = upgrade {
        level += i32::from(soldiers) + i32::from(building);
    }
    level
}

/// AI-MIL-4.3: the building type for a result (level minus attempts);
/// a negative result means no building.
pub fn building_for(config: &AiConfig, result: i32) -> Option<u32> {
    let n = &config.names;
    match result {
        r if r < 0 => None,
        0 => Some(n.barracks),
        1 => Some(n.guard_house),
        2 => Some(n.tower),
        _ => Some(n.fortress),
    }
}

/// AI-MIL-2.2: the near and medium allocation after a run, given the current
/// medium allocation `m` and whether soldiers are in stock; `None` for no
/// change.
pub fn allocation(m: f32, soldiers: u32, statistic: u32) -> Option<(f32, f32)> {
    let f = if soldiers == 0 {
        m * 8.0 / 3.0
    } else if soldiers > statistic / 128 {
        (m * 3.3333 * 1.25).min(4.0)
    } else {
        return None;
    };
    Some((f.min(1.0), (0.3 * f).min(1.0)))
}

/// AI-MIL-4.6: the soldiers the headquarters keeps per rank at enemy
/// distance `d`.
pub fn castle_reserve(d: u32) -> u32 {
    if d < 50 {
        (50 - d) * 15 / 50
    } else {
        0
    }
}

/// AI-MIL-5.2: the attack target value of a cell for an enemy player.
pub fn target_value(construction_threat: f32, score: f32, balance: f32, c: f32, a: f32) -> f32 {
    (construction_threat + 2.0 * score - balance) / (c + 1.0 + a)
}

/// AI-MIL-5.2 (T31): whether a cell is looked at for an attack: one of
/// its eight neighbouring cells (the cell itself not; cells off the grid
/// count nothing) holds a nonzero threat in the attacking player's own map.
pub fn next_to_own_threat(own: &MilitaryMap, cell: MPos) -> bool {
    (-1i64..=1).any(|dx| {
        (-1i64..=1).any(|dy| {
            let (x, y) = (i64::from(cell.0) + dx, i64::from(cell.1) + dy);
            (dx, dy) != (0, 0) && x >= 0 && y >= 0 && own.get((x as u32, y as u32)).threat != 0
        })
    })
}

/// AI-MIL-5.2: the target cell and player. The cells next to the player's
/// own military presence ([`next_to_own_threat`]) are visited column by
/// column (x from 0, and for each x every y from 0), and for each the alive
/// enemy players 0 to 7 in order whose score in the cell is not 0; value =
/// (their threat under construction + 2 x their score - the balance) /
/// (C + 1 + A). The highest value above -100 wins, a later one only when
/// strictly greater. `counters` gives the cell's C and A.
pub fn attack_target(
    maps: &[MilitaryMap],
    alive: &[bool],
    player: PlayerId,
    enemy: impl Fn(PlayerId) -> bool,
    balance: impl Fn(MPos) -> f32,
    counters: impl Fn(MPos) -> (f32, f32),
) -> Option<(f32, MPos, PlayerId)> {
    let own = maps.get(usize::from(player.0))?;
    let mut best: Option<(f32, MPos, PlayerId)> = None;
    for x in 0..own.cols {
        for y in 0..own.rows {
            let cell = (x, y);
            if !next_to_own_threat(own, cell) {
                continue;
            }
            let balance = balance(cell);
            let (c, a) = counters(cell);
            for (slot, m) in maps.iter().enumerate() {
                let p = PlayerId(slot as u8);
                if !alive.get(slot).copied().unwrap_or(false) || p == player || !enemy(p) {
                    continue;
                }
                let them = m.get(cell);
                if them.score == 0 {
                    continue;
                }
                let value = target_value(
                    them.threat_building as f32,
                    them.score as f32,
                    balance,
                    c,
                    a,
                );
                if value > MIN_TARGET_VALUE && best.is_none_or(|(v, ..)| value > v) {
                    best = Some((value, cell, p));
                }
            }
        }
    }
    best
}

/// AI-MIL-5.4, 5.5: what a run does with its candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackDecision {
    /// The first candidate attacked, with how many soldiers.
    Attack { at: Point, count: u32 },
    /// None attacked; the last candidate that reached the need test, if any
    /// did (the position of 5.5's expansion target).
    NotAttacked { last: Option<Point> },
}

/// AI-MIL-5.4, 5.5 (T30): the candidates in 5.3's order, each with the
/// soldiers wanted against it and those the player can send against it.
/// In mode Safe one whose want is above the soldiers in stock is skipped
/// and is no last candidate; every other reaches the need test (need =
/// max(1, want) + C) and is attacked when [`attack_count`] allows; the
/// first attack ends the run.
pub fn decide_attack(
    mode: Mode,
    candidates: &[(Point, u32, u32)],
    in_stock: u32,
    c: f32,
) -> AttackDecision {
    let mut last = None;
    for &(at, want, available) in candidates {
        if mode == Mode::Safe && want > in_stock {
            continue;
        }
        let need = want.max(1) as f32 + c;
        if let Some(count) = attack_count(mode, need, available) {
            return AttackDecision::Attack { at, count };
        }
        last = Some(at);
    }
    AttackDecision::NotAttacked { last }
}

/// AI-MIL-5.5 (T30): C and A are whole numbers; halving drops the
/// remainder (3 becomes 1, 1 becomes 0).
fn halved(counter: f32) -> f32 {
    (counter / 2.0).floor()
}

/// AI-MIL-5.3: whether an attackable enemy building can be a candidate: it
/// is in the player's explored map, or its enemy distance is at most 12,
/// a signed test that -1 passes: an enemy headquarters always holds -1
/// (MIL-SOL-3.1 never updates it), as does a military building made since
/// the last update (T31).
pub fn candidate_seen(explored: bool, enemy_distance: Option<i32>) -> bool {
    explored || enemy_distance.is_some_and(|d| d <= 12)
}

/// AI-MIL-5.4: the soldiers wanted against a building with `defenders`
/// defenders for the strength ratio `ratio` and the owner's defend-outside
/// factor.
pub fn soldiers_wanted(ratio: f32, defenders: u32, defend_outside: f32) -> u32 {
    (ratio.min(5.0) * defenders as f32 * (defend_outside * 0.5 + 1.0) * 1.5) as u32
}

/// AI-MIL-5.4: how many soldiers to send, by military mode (`None`: do not
/// attack). `need` includes the cell's result counter.
pub fn attack_count(mode: Mode, need: f32, available: u32) -> Option<u32> {
    if available == 0 {
        return None;
    }
    let need_whole = need as u32;
    match mode {
        Mode::Aggressive => Some(available),
        _ if need > available as f32 => None,
        Mode::Safe => Some(need_whole.min(available)),
        Mode::Normal => Some((2 * need_whole).min(available)),
    }
}

/// AI-MIL-5.3: candidates with the highest surrounding score first; a
/// candidate whose score equals that of an earlier one is dropped.
pub fn order_candidates<T>(mut list: Vec<(f32, T)>, key: impl Fn(&T) -> Point) -> Vec<(f32, T)> {
    list.sort_by(|a, b| b.0.total_cmp(&a.0).then(key(&a.1).cmp(&key(&b.1))));
    list.dedup_by(|b, a| a.0 == b.0);
    list
}

/// Enemy distance of a field: the distance to the nearest occupied
/// military building of an enemy of `player`, 1000 if none (AI-MIL intro).
pub fn enemy_distance(world: &World, player: PlayerId, at: Point) -> u32 {
    world
        .military_buildings()
        .filter(|(_, m)| m.applied)
        .filter_map(|(p, _)| {
            let b = world.building_at(p)?;
            world.are_enemies(player, b.owner).then(|| dist(at, p))
        })
        .min()
        .unwrap_or(1000)
}

impl AiPlayer {
    /// Soldiers of all ranks in the player's warehouses.
    pub(crate) fn soldiers_in_stock(&self, ctx: &Ctx) -> u32 {
        ctx.snapshot
            .of(self.id)
            .iter()
            .filter(|b| {
                !b.under_construction && self.records.get(&b.at).is_some_and(|r| r.protect.depot)
            })
            .map(|b| ctx.world.stock_soldiers(b.at).iter().sum::<u32>())
            .sum()
    }

    /// The strength of a player: its soldiers in buildings and in the field
    /// weighted by rank + 1, plus its stock (AI-military intro).
    fn strength(world: &World, snapshot: &crate::snapshot::Snapshot, player: PlayerId) -> u32 {
        let mut total = 0;
        for (_, s) in world.settlers() {
            if s.player == player {
                if let Some(rank) = world.soldier_rank(s.profession) {
                    total += rank as u32 + 1;
                }
            }
        }
        for b in snapshot.of(player) {
            if world.warehouse(b.at).is_some() {
                for (rank, n) in world.stock_soldiers(b.at).iter().enumerate() {
                    total += n * (rank as u32 + 1);
                }
            }
        }
        total
    }

    // ---- AI-MIL-2 settings ----

    /// AI-MIL-2, every 53 ticks.
    pub(crate) fn military_settings(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let current = world.military_settings(self.id);
        // 2.1: attack strength and intercepting from the profile, every run.
        let strength = (self.profile.attack_strength * 1000.0).round() as u32;
        if (current.attack_strength() * 1000.0).round() as u32 != strength {
            ctx.out.push(Command::MilitaryAttackStrength {
                player: self.id,
                thousandths: strength,
            });
        }
        let defenders = (self.profile.intercepting * 1000.0).round() as u32;
        if (current.defenders() * 1000.0).round() as u32 != defenders {
            ctx.out.push(Command::MilitaryDefenders {
                player: self.id,
                thousandths: defenders,
            });
        }
        // 2.3: Safe prefers weak soldiers (type 0), Aggressive strong ones
        // (type 1), Normal leaves it.
        let wanted = match self.profile.mode {
            Mode::Safe => Some(false),
            Mode::Aggressive => Some(true),
            Mode::Normal => None,
        };
        if let Some(strong) = wanted {
            if current.prefer_strong != strong {
                ctx.out.push(Command::MilitaryDefenseType {
                    player: self.id,
                    strong,
                });
            }
        }
        // 2.2: near and medium allocation, a countdown of 50 runs gates it.
        // T29: every run first lowers it by 1 and tests only when it is 0 or
        // below, so two changes are 50 runs apart; a run without a change
        // leaves it at 0 or below and the next run tests again. Signed, as a
        // saved value below 0 reads as a u32 (FMT-SAV-3.10h).
        let left = (self.settings_countdown as i32).saturating_sub(1);
        self.settings_countdown = left.max(0) as u32;
        if left > 0 {
            return;
        }
        let soldiers = self.soldiers_in_stock(ctx);
        let s = self.statistic(ctx);
        if let Some((near, medium)) = allocation(current.allocation(1), soldiers, s) {
            ctx.out.push(Command::MilitaryAllocation {
                player: self.id,
                class: 0,
                thousandths: (near * 1000.0).round() as u32,
            });
            ctx.out.push(Command::MilitaryAllocation {
                player: self.id,
                class: 1,
                thousandths: (medium * 1000.0).round() as u32,
            });
            self.settings_countdown = SETTINGS_COUNTDOWN;
        }
    }

    // ---- AI-MIL-3 expansion targets ----

    /// AI-MIL-3.1, every 29 ticks: while no target is in progress, the
    /// oldest pending target is taken (strictly older wins, the sources in
    /// the order waiting need, crowded cell, anywhere, attack) and, one run
    /// later, becomes the current target. The source's slot keeps it until
    /// it is finished or cancelled (3.2).
    pub(crate) fn expansion_targets(&mut self, _ctx: &mut Ctx) {
        if let Some((source, target)) = self.expansion.taken.take() {
            self.expansion.current = Some((source, target));
            return;
        }
        if self.expansion.current.is_some() {
            return;
        }
        let order = [
            Source::WaitingNeed,
            Source::Crowded,
            Source::Anywhere,
            Source::Attack,
        ];
        let mut oldest: Option<(Source, crate::expansion::Target)> = None;
        for source in order {
            if let Some(t) = self.expansion.pending[source as usize] {
                if oldest.is_none_or(|(_, o)| t.since < o.since) {
                    oldest = Some((source, t));
                }
            }
        }
        if let Some(taken) = oldest {
            self.expansion.taken = Some(taken);
        }
    }

    /// AI-MIL-3.2: the current target is finished (`success`) or cancelled:
    /// its source is told (a waiting-need target's cell coordinate's
    /// attempt counter halved or raised by 1), and the source's slot and
    /// the current target are cleared.
    fn end_target(&mut self, success: bool) {
        if let Some((source, target)) = self.expansion.current.take() {
            if source == Source::WaitingNeed {
                let a = self.attempts.entry(cell_of(target.at)).or_insert(0);
                *a = if success { *a / 2 } else { *a + 1 };
            }
            self.expansion.pending[source as usize] = None;
        }
        self.expansion.remembered = false;
    }

    // ---- AI-MIL-4 military buildings ----

    /// AI-MIL-4.2: the best cell of the military map on the home landmass
    /// for an expansion towards `target`.
    fn best_cell(&self, ctx: &Ctx, target: Option<Point>) -> Option<MPos> {
        let world = ctx.world;
        let map = &ctx.military[usize::from(self.id.0)];
        let home = mcell_of(self.home);
        // The cell's top-left field decides its landmass (AI-MIL-4.2): its
        // stored continent index against the home field's (AI-CON-1.1b).
        let home_landmass = world.continent_of(self.home);
        let landmass_of = |c: MPos| world.continent_of(morigin(c));
        let mut seen = vec![false; (map.cols * map.rows) as usize];
        let mut queue = VecDeque::new();
        seen[(home.1 * map.cols + home.0) as usize] = true;
        queue.push_back(home);
        let mut best: Option<(f32, MPos)> = None;
        while let Some(c) = queue.pop_front() {
            let balance = threat_balance(world, ctx.military, ctx.alive, self.id, c);
            let own = map.get(c);
            // The neighbour tests read the player's own map (Q-166a).
            let neighbours: Vec<MPos> = DIRECTIONS
                .iter()
                .filter_map(|&(dx, dy)| {
                    let (nx, ny) = (c.0 as i32 + dx, c.1 as i32 + dy);
                    (nx >= 0 && ny >= 0 && nx < map.cols as i32 && ny < map.rows as i32)
                        .then_some((nx as u32, ny as u32))
                })
                .collect();
            let neighbour_threat = neighbours.iter().any(|&n| map.get(n).threat > 0);
            let neighbour_empty = neighbours.iter().any(|&n| map.get(n).score == 0);
            let qualifies = balance < MAX_BALANCE
                && (neighbour_threat
                    || (own.score > 0 && own.threat_building == 0 && neighbour_empty));
            if qualifies {
                let d = target.map_or(1, |t| dist(morigin(c), t));
                let penalty = self.territory.get(c).penalty;
                let value = 100.0 / (4.0 * d as f32 + own.threat.max(1) as f32) / (penalty + 1.0);
                if best.is_none_or(|(v, _)| value > v) {
                    best = Some((value, c));
                }
            }
            for n in neighbours {
                let index = (n.1 * map.cols + n.0) as usize;
                if !seen[index] && landmass_of(n).is_some() && landmass_of(n) == home_landmass {
                    seen[index] = true;
                    queue.push_back(n);
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// Whether an underground resource lies within 4 squares of `at`.
    pub(crate) fn underground_near(&self, at: Point, fish: u32) -> bool {
        let (sx, sy) = square_of(at);
        for y in sy.saturating_sub(4)..=(sy + 4).min(self.resources.rows.saturating_sub(1)) {
            for x in sx.saturating_sub(4)..=(sx + 4).min(self.resources.cols.saturating_sub(1)) {
                if self
                    .resources
                    .entries((x, y))
                    .iter()
                    .any(|e| e.kind == UNDERGROUND && e.amount > 0 && e.good != fish)
                {
                    return true;
                }
            }
        }
        false
    }

    /// AI-MIL-4.1, every 311 ticks.
    pub(crate) fn military_building(&mut self, ctx: &mut Ctx) {
        if self.military_cooldown > 0 {
            self.military_cooldown -= 1;
            return;
        }
        if self.build.busy(Slot::Military) {
            return;
        }
        // AI-MIL-3.2, 4.1: the first run past both finishes the target it
        // remembers, and makes no order.
        if self.expansion.remembered {
            self.end_target(true);
            return;
        }
        let Some((source, target)) = self.expansion.current else {
            return;
        };
        let (area, mcell) = match target.kind {
            Kind::Position => {
                let Some(c) = self.best_cell(ctx, Some(target.at)) else {
                    self.drop_target(source, MPos::default(), false);
                    return;
                };
                (
                    Area::Point {
                        at: morigin(c),
                        radius: RADIUS_POSITION,
                    },
                    c,
                )
            }
            Kind::Crowded => (Area::Cell(cell_of(target.at)), mcell_of(target.at)),
            Kind::Anywhere => {
                let Some(c) = self.best_cell(ctx, None) else {
                    self.drop_target(source, MPos::default(), false);
                    return;
                };
                (
                    Area::Point {
                        at: morigin(c),
                        radius: RADIUS_ANYWHERE,
                    },
                    c,
                )
            }
        };
        let lv = self.creation_level(ctx, area);
        let Some(building) = building_for(ctx.config, lv.max(0)) else {
            self.drop_target(source, mcell, true);
            return;
        };
        let order = Order {
            slot: Slot::Military,
            area,
            building,
            weight: MILITARY_WEIGHT,
            time: ctx.world.game_seconds(),
            tag: Tag::Military {
                attempts: 0,
                level: lv,
                cell: mcell,
                source,
            },
        };
        self.build.issue(order);
        // AI-MIL-4.1: the run that makes an order remembers the target.
        self.expansion.remembered = true;
    }

    /// AI-MIL-4.3 (creation): the level at an order's place, the point in
    /// point mode and the cell's origin in cell mode, as things stand now.
    fn creation_level(&self, ctx: &Ctx, area: Area) -> i32 {
        let site = match area {
            Area::Point { at, .. } => at,
            Area::Cell(c) => crate::cells::origin(c),
        };
        level(
            enemy_distance(ctx.world, self.id, site),
            self.underground_near(site, ctx.config.names.fish),
            self.econ.stock(ctx.config.names.stone) > 40,
            None,
        )
    }

    /// AI-MIL-4.4: the target is given up; the penalty of the cell rises.
    fn drop_target(&mut self, _source: Source, cell: MPos, penalise: bool) {
        // AI-MIL-3.2: cancelled at once.
        self.end_target(false);
        if penalise {
            if let Some(c) = self.territory.get_mut(cell) {
                c.penalty += 1.0;
            }
        }
    }

    /// AI-MIL-4.3 (upgrade check): the level an upgrade of the building on
    /// `at` aims for.
    pub(crate) fn upgrade_level(&self, ctx: &Ctx, at: Point) -> i32 {
        let world = ctx.world;
        // The nearest own finished warehouse on the site's landmass
        // (AI-CON-1.1b) within distance < 5.
        let landmass = world.continent_of(at);
        let warehouse_near = ctx.snapshot.of(self.id).iter().any(|b| {
            !b.under_construction
                && world
                    .building_type(b.building_type)
                    .is_some_and(|t| t.warehouse)
                && dist(b.at, at) < 5
                && world.continent_of(b.at) == landmass
        });
        level(
            enemy_distance(world, self.id, at),
            self.underground_near(at, ctx.config.names.fish),
            self.econ.stock(ctx.config.names.stone) > 40,
            Some((self.soldiers_in_stock(ctx) > 30, warehouse_near)),
        )
        .max(0)
    }

    /// AI-MIL-4.5: the command that upgrades the finished military building
    /// `b` if it can be upgraded, its level (barracks 0 to fortress 3) is
    /// below the upgrade level at its position, and twice the target type's
    /// full construction costs are in stock (T19: the list of frame and
    /// building goods merged with itself, as for AI-CON-5.3 (1)).
    fn upgrade_command(&self, ctx: &Ctx, b: &Building) -> Option<Command> {
        let rec = self.records.get(&b.at)?;
        if b.under_construction || rec.protect.depot || !rec.protect.military {
            return None;
        }
        let n = &ctx.config.names;
        let level = [n.barracks, n.guard_house, n.tower, n.fortress]
            .iter()
            .position(|&t| t == b.building_type)? as i32;
        let next = ctx.world.upgrade_target(self.id, b.at).ok()?;
        if level >= self.upgrade_level(ctx, b.at) {
            return None;
        }
        self.costs_in_stock(ctx, next, 2)
            .then_some(Command::UpgradeBuilding {
                player: self.id,
                at: b.at,
            })
    }

    /// AI-MIL-4.5, every 37 ticks: one military building per run.
    /// AI-MIL-4.5a: the run takes the entry at the player's own cursor of
    /// the world's military list (every player's buildings whose type has a
    /// territory range, sites included, in creation order), the cursor
    /// starting again at 0 when it has reached the list's end; only the
    /// player's own building that is not a headquarters is tested.
    pub(crate) fn military_upgrade(&mut self, ctx: &mut Ctx) {
        // With the headquarters types (the warehouses among them).
        let mut list: Vec<(Building, bool)> = ctx
            .world
            .buildings()
            .filter_map(|b| {
                let kind = ctx.world.building_type(b.building_type)?;
                (kind.territory_range > 0).then_some((*b, kind.warehouse))
            })
            .collect();
        list.sort_by_key(|(b, _)| b.serial);
        if self.upgrade_cursor as usize >= list.len() {
            self.upgrade_cursor = 0;
        }
        let Some(&(b, headquarters)) = list.get(self.upgrade_cursor as usize) else {
            return;
        };
        self.upgrade_cursor += 1;
        if b.owner != self.id || headquarters {
            return;
        }
        if let Some(command) = self.upgrade_command(ctx, &b) {
            ctx.out.push(command);
        }
    }

    /// AI-MIL-4.4, 4.4a (T32): upgrade instead, in the order's area: the
    /// cell's 16 x 16 fields row by row from its first row (cell mode), or
    /// the fields around the point in ring order up to the order's own
    /// radius (point mode), fields off the map skipped; the first field
    /// whose building is the player's own, has a territory range, can be
    /// upgraded (MIL-UPG-1.2) and whose next type's costs are in stock
    /// **once** (MIL-UPG-1.4) is upgraded. No level test, unlike the 37-tick
    /// run (4.5). True if one was.
    fn upgrade_in(&self, ctx: &mut Ctx, area: Area) -> bool {
        let world = ctx.world;
        let (width, height) = (world.terrain().width, world.terrain().height);
        let fields: Vec<Point> = match area {
            Area::Cell(c) => {
                let (x0, y0) = crate::cells::origin(c);
                (y0..y0 + crate::cells::CELL)
                    .flat_map(|y| (x0..x0 + crate::cells::CELL).map(move |x| (x, y)))
                    .filter(|&(x, y)| x < width && y < height)
                    .collect()
            }
            Area::Point { at, radius } => s2_sim::grid::area(at, radius, width, height),
        };
        let found = fields.into_iter().find(|&f| {
            let Some(b) = world
                .building_at(f)
                .filter(|b| b.at == f && b.owner == self.id)
            else {
                return false;
            };
            let ranged = world
                .building_type(b.building_type)
                .is_some_and(|t| t.territory_range > 0);
            let Ok(next) = world.upgrade_target(self.id, f) else {
                return false;
            };
            ranged && self.costs_in_stock(ctx, next, 1)
        });
        match found {
            Some(at) => {
                ctx.out.push(Command::UpgradeBuilding {
                    player: self.id,
                    at,
                });
                true
            }
            None => false,
        }
    }

    /// MIL-UPG-1.4: whether `times` the full construction costs of the
    /// type `kind` (frame and building goods, one list) are in stock
    /// (AI-CON-1.7).
    fn costs_in_stock(&self, ctx: &Ctx, kind: u32, times: u32) -> bool {
        let Some(kind) = ctx.world.building_type(kind) else {
            return false;
        };
        let mut needed: Vec<(u32, u32)> = Vec::new();
        for &(good, count) in kind.frame_costs.iter().chain(&kind.costs) {
            match needed.iter_mut().find(|(g, _)| *g == good) {
                Some((_, c)) => *c += count,
                None => needed.push((good, count)),
            }
        }
        needed
            .iter()
            .all(|&(good, count)| self.econ.stock(good) >= times * count)
    }

    /// AI-MIL-4.4: the handlers of military and catapult orders.
    pub(crate) fn military_order_done(&mut self, order: Order, outcome: Outcome, ctx: &mut Ctx) {
        let Tag::Military {
            attempts,
            cell,
            source,
            ..
        } = order.tag
        else {
            return;
        };
        match outcome {
            Outcome::Placed(_) => {
                if let Some(c) = self.territory.get_mut(cell) {
                    c.penalty /= 2.0;
                }
                self.military_cooldown = PLACEMENT_COOLDOWN;
                // AI-MIL-3.2: the placement leaves the target remembered; a
                // later run finishes it.
                self.expansion.remembered = true;
            }
            // AI-MIL-4.4 (corrected, T18): no material re-orders like no
            // site.
            Outcome::NoSite | Outcome::NoMaterial => {
                let attempts = attempts + 1;
                // AI-MIL-4.4 (T28): the level is worked out anew at each
                // failure, at the order's place as things stand now (enemy
                // distance, underground resource, stone), not kept from the
                // order's creation; the new attempt count is taken off it.
                let lv = self.creation_level(ctx, order.area);
                let result = lv.max(0) - attempts as i32;
                match building_for(ctx.config, result) {
                    Some(building) => {
                        let next = Order {
                            building,
                            time: ctx.world.game_seconds(),
                            tag: Tag::Military {
                                attempts,
                                level: lv,
                                cell,
                                source,
                            },
                            ..order
                        };
                        self.build.issue(next);
                    }
                    // AI-MIL-4.4: below 0 after no material nothing happens:
                    // the target stays current and remembered, and the next
                    // run past the cooldown finishes it.
                    None if outcome == Outcome::NoMaterial => {}
                    None => {
                        // AI-MIL-4.4: below 0 the AI tries to upgrade an own
                        // military building in the order's cell (within the
                        // order's radius in point mode, 4.4a). If one is upgraded the
                        // target counts as done and P is halved, else the
                        // target is cancelled and P rises by 1.
                        if self.upgrade_in(ctx, order.area) {
                            if let Some(c) = self.territory.get_mut(cell) {
                                c.penalty /= 2.0;
                            }
                            // AI-MIL-3.2: finished at once.
                            self.end_target(true);
                        } else {
                            self.drop_target(source, cell, true);
                        }
                    }
                }
            }
        }
    }

    /// AI-MIL-4.6, every 613 ticks: the headquarters keeps soldiers back
    /// where enemies are near.
    pub(crate) fn castle_settings(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let reserve = castle_reserve(enemy_distance(world, self.id, self.home));
        let Some(m) = world.military_at(self.home) else {
            return;
        };
        for rank in 0..SOLDIER_RANKS {
            if m.castle[rank] != reserve {
                ctx.out.push(Command::SetCastleSoldiers {
                    player: self.id,
                    at: self.home,
                    rank: rank as u8,
                    amount: reserve,
                });
            }
        }
    }

    /// AI-MIL-4.7, every 653 ticks (profile coin arrangement): coins and the
    /// "send soldiers out" flag by enemy distance.
    pub(crate) fn coin_arrangement(&mut self, ctx: &mut Ctx) {
        if !self.profile.coin_arrangement {
            return;
        }
        let world = ctx.world;
        for b in ctx.snapshot.of(self.id) {
            let Some(rec) = self.records.get(&b.at) else {
                continue;
            };
            if b.under_construction || rec.protect.depot || !rec.protect.military {
                continue;
            }
            let Some(m) = world.military_at(b.at) else {
                continue;
            };
            if m.enemy_distance < 0 {
                continue;
            }
            let near = m.enemy_distance < COIN_DISTANCE;
            if m.coin_switch != near {
                ctx.out.push(Command::ToggleCoinSupply {
                    player: self.id,
                    at: b.at,
                });
            }
            if m.move_out == near {
                ctx.out.push(Command::ToggleMoveOut {
                    player: self.id,
                    at: b.at,
                });
            }
        }
    }

    // ---- AI-MIL-5 attacks ----

    /// AI-MIL-5, every 641 ticks.
    pub(crate) fn attack_system(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        // 5.5: the results of the last attack.
        if let Some((at, cell)) = self.attack.target {
            let building = world.building_at(at);
            let ours = building.is_some_and(|b| b.owner == self.id);
            let on_the_way = world.attack_records().iter().any(|r| {
                r.target.at == at
                    && world
                        .settlers()
                        .any(|(id, s)| id == r.soldier && s.player == self.id)
            });
            if building.is_some() && !ours && on_the_way {
                return;
            }
            if let Some(c) = self.territory.get_mut(cell) {
                if building.is_none() || ours {
                    c.result = halved(c.result);
                    c.attempts = halved(c.attempts);
                } else {
                    c.result += 1.0;
                }
            }
            self.attack.target = None;
            return;
        }
        // 5.2 (T31): a target cell and player, next to the player's own
        // military presence, column by column.
        let best = attack_target(
            ctx.military,
            ctx.alive,
            self.id,
            |p| world.are_enemies(self.id, p),
            |cell| threat_balance(world, ctx.military, ctx.alive, self.id, cell),
            |cell| {
                let t = self.territory.get(cell);
                (t.result, t.attempts)
            },
        );
        let Some((_, cell, owner)) = best else {
            return;
        };
        // 5.3: candidates.
        let origin = morigin(cell);
        let mut candidates: Vec<(f32, Building)> = Vec::new();
        for b in ctx.snapshot.of(owner) {
            if b.under_construction || dist(b.at, origin) > ATTACK_RADIUS {
                continue;
            }
            let Some(t) = world.building_type(b.building_type) else {
                continue;
            };
            let attackable = b.building_type == ctx.config.names.castle
                || (t.soldier_limit > 0 && world.military_at(b.at).is_some_and(|m| m.applied));
            if !attackable {
                continue;
            }
            // 5.3 (T31): the signed enemy distance test.
            let seen = candidate_seen(
                world.is_explored(self.id, b.at),
                world.military_at(b.at).map(|m| m.enemy_distance),
            );
            if !seen {
                continue;
            }
            // The score around the building, lowest first.
            let around: f32 = ctx
                .snapshot
                .of(owner)
                .iter()
                .filter_map(|o| {
                    let ring = dist(b.at, o.at);
                    (ring <= RANK_RINGS).then(|| {
                        ctx.config.weights_of(o.building_type).score as f32
                            / (ring.saturating_sub(3).max(1)) as f32
                    })
                })
                .sum();
            candidates.push((around, *b));
        }
        let candidates = order_candidates(candidates, |b| b.at);
        // 5.4: the decision per candidate.
        let ours = Self::strength(world, ctx.snapshot, self.id).max(1);
        let theirs = Self::strength(world, ctx.snapshot, owner);
        let ratio = (theirs as f32 / ours as f32).min(5.0);
        let defend_outside = world.military_settings(owner).defenders();
        let c_value = self.territory.get(cell).result;
        let in_stock = self.soldiers_in_stock(ctx);
        let tested: Vec<(Point, u32, u32)> = candidates
            .iter()
            .map(|(_, b)| {
                let warehouse = world.warehouse(b.at).is_some();
                let defenders = if warehouse {
                    WAREHOUSE_DEFENDERS
                } else {
                    // T33 (Q-1076): the building's list, not only those
                    // inside.
                    world.listed_soldiers(b.at) as u32
                };
                let want = soldiers_wanted(ratio, defenders, defend_outside);
                (b.at, want, world.attack_available(self.id, b.at))
            })
            .collect();
        let last = match decide_attack(self.profile.mode, &tested, in_stock, c_value) {
            AttackDecision::Attack { at, count } => {
                ctx.out.push(Command::Attack {
                    player: self.id,
                    target: at,
                    count,
                    strong_first: true,
                });
                self.attack.target = Some((at, cell));
                return;
            }
            AttackDecision::NotAttacked { last } => last,
        };
        // No candidate attacked: A + 1, and an expansion target at the last
        // candidate that reached the need test (T30), if any did.
        if let Some(c) = self.territory.get_mut(cell) {
            c.attempts += 1.0;
        }
        if !self.expansion.is_pending(Source::Attack) {
            if let Some(at) = last {
                let now = world.tick();
                self.expansion.set(Source::Attack, Kind::Position, at, now);
            }
        }
    }

    // ---- AI-MIL-6 soldiers, weapons, coins, catapults ----

    /// AI-MIL-6.1, every 409 ticks: among the finished depots on the home
    /// landmass (storehouses and the headquarters, AI-CON-1.1) the one with
    /// the highest 2 x enemy distance - (S + B) recruits, ties keeping the
    /// earlier; S and B the lowest stored ware costs from its flag to a
    /// recorded smithy and brewery (T22). Then, cell by cell, every
    /// finished harbour and every other finished depot (any landmass) gets
    /// "out" set for the soldier's wares (sword, shield, beer), and last the
    /// chosen one gets it cleared; each record is found or created
    /// (ECO-DIST-1.7c b; Q-166c, Q-167, Q-264).
    pub(crate) fn soldier_creation(&mut self, ctx: &mut Ctx) {
        let world = ctx.world;
        let n = ctx.config.names.clone();
        let finished = |at: Point| {
            ctx.snapshot
                .building(self.id, at)
                .is_some_and(|b| !b.under_construction)
        };
        // Cell by cell in the cell system's order, each cell's buildings in
        // the order they were announced.
        let mut harbours: Vec<(CellPos, Point)> = Vec::new();
        let mut depots: Vec<(CellPos, Point)> = Vec::new();
        for &cell in self.cells.in_order() {
            let Some(entry) = self.cells.get(cell) else {
                continue;
            };
            for at in &entry.buildings {
                let Some(r) = self.records.get(at).filter(|r| finished(r.at)) else {
                    continue;
                };
                if r.category == Category::Harbour {
                    harbours.push((cell, r.at));
                } else if r.category == Category::Depot {
                    depots.push((cell, r.at));
                }
            }
        }
        // T22: every smithy and brewery the player has recorded, finished
        // or a construction site, cell by cell, each cell's in its order.
        let mut producers: Vec<(u32, Point)> = Vec::new();
        for &cell in self.cells.in_order() {
            let Some(entry) = self.cells.get(cell) else {
                continue;
            };
            for at in &entry.buildings {
                let kind = world.building_at(*at).map(|b| b.building_type);
                if let Some(kind) = kind.filter(|&k| k == n.smithy || k == n.brewery) {
                    producers.push((kind, *at));
                }
            }
        }
        let mut best: Option<(i64, Point)> = None;
        for &(cell, at) in &depots {
            // AI-CON-1.1b (b), (c): the depots of the cells on the home
            // landmass.
            if self.cells.get(cell).map(|c| c.landmass) != Some(self.home_landmass) {
                continue;
            }
            // AI-MIL-6.1 (T22): S and B, the lowest stored ware costs
            // (TRN-CARRY-5.3a) asked from the depot's flag to those in its
            // flag's ware network, each from 100 (a dearer one counts 100).
            let (mut smithy, mut brewery) = (100i64, 100i64);
            if let Some(depot_flag) = flag_of(world, at) {
                let network = world.ware_network_of(self.id, depot_flag);
                for &(kind, b) in &producers {
                    let Some(flag) = flag_of(world, b).filter(|f| network.contains(f)) else {
                        continue;
                    };
                    let Some(cost) = ctx.routes.ware_cost(world, self.id, depot_flag, flag) else {
                        continue;
                    };
                    let least = if kind == n.smithy {
                        &mut smithy
                    } else {
                        &mut brewery
                    };
                    *least = (*least).min(i64::from(cost));
                }
            }
            let score = 2 * i64::from(enemy_distance(world, self.id, at)) - (smithy + brewery);
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, at));
            }
        }
        let Some((_, chosen)) = best else { return };
        let wares = [n.sword, n.shield, n.beer];
        for &cell in self.cells.in_order() {
            let of_cell = |list: &[(CellPos, Point)]| -> Vec<Point> {
                list.iter()
                    .filter(|(c, at)| *c == cell && *at != chosen)
                    .map(|&(_, at)| at)
                    .collect()
            };
            for at in of_cell(&harbours).into_iter().chain(of_cell(&depots)) {
                for good in wares {
                    self.set_warehouse_flag(ctx, at, good, false, true);
                }
            }
        }
        for good in wares {
            self.set_warehouse_flag(ctx, chosen, good, false, false);
        }
    }

    /// AI-MIL-6.2, 6.3 (Q-283): every building of `kind` the player has, in
    /// any state, in its building list's order, as (field, productivity,
    /// ready): ready when every need tied to it has a production and it is
    /// finished with its worker in. A production exists only from its
    /// site's placement (T27): one only ordered does not count.
    fn readiness_of(&self, ctx: &Ctx, kind: u32) -> Vec<(Point, f32, bool)> {
        ctx.snapshot
            .of(self.id)
            .iter()
            .filter(|b| b.building_type == kind)
            .map(|b| {
                let ready = self.consumer_ready(b);
                // AI-MIL T24: the float32 statistics value.
                let value = f32::from_bits(b.productivity_bits).max(0.0);
                (b.at, value, ready)
            })
            .collect()
    }

    /// AI-MIL-6.2 (T27), AI-CON-10.3 (T87): the consumer building `b` is
    /// ready: every need tied to it has a production that is placed, and it
    /// is finished with its worker in.
    pub(crate) fn consumer_ready(&self, b: &Building) -> bool {
        let placed = |id| {
            self.plan
                .prods
                .iter()
                .any(|p| p.id == id && p.building.is_some())
        };
        self.plan
            .needs
            .iter()
            .filter(|n| n.consumer == Some(b.at))
            .all(|n| n.production.is_some_and(placed))
            && !b.under_construction
            && b.active
    }

    /// AI-MIL-6.2 and 6.3 shared scheme for a weapon or coin building type.
    /// `gate` is the scheme's (t0, k): a pass is due once `now - t0 > 600 s
    /// x k`; it first sets `t0 = now - 600 k` and raises k, and when the
    /// mean productivity is high (or soldiers are few) raises k again and
    /// creates the need, so the next pass comes 600 s after a pass without a
    /// need and 1200 s after one with a need (Q-166d).
    fn producer_needs(
        &mut self,
        ctx: &mut Ctx,
        building: u32,
        product: u32,
        limit: u32,
        by_soldiers: bool,
        mut gate: Gate,
    ) -> Gate {
        let t = ctx.world.game_seconds();
        // AI-MIL-6.2, 6.3 (Q-283): one not ready, or a ready one doing
        // little, ends the check before the gate; the last ready one doing
        // little gets needs for its inputs out of stock.
        let have = self.readiness_of(ctx, building);
        let (mut stop, mut idle, mut sum) = (false, None, 0.0f32);
        for &(at, productivity, ready) in &have {
            if !ready {
                stop = true;
                continue;
            }
            sum += productivity;
            if productivity < IDLE {
                stop = true;
                idle = Some(at);
            }
        }
        if stop {
            if let Some(at) = idle {
                let inputs: Vec<u32> = ctx
                    .config
                    .type_inputs(building, product)
                    .iter()
                    .map(|&(g, _)| g)
                    .collect();
                // AI-MIL T24: inputs of which the building's own input store
                // holds none (not the player's stock).
                let store = ctx.world.inputs(at);
                for input in inputs {
                    if !store.iter().any(|&(g, n)| g == input && n > 0) {
                        self.plan.tie_need(input, at, t);
                    }
                }
            }
            return gate;
        }
        if have.len() as u32 >= limit || t - gate.t0 <= 600.0 * gate.k as f32 {
            return gate;
        }
        gate.t0 = t - 600.0 * gate.k as f32;
        gate.k += 1;
        let mean = if have.is_empty() {
            0.0
        } else {
            sum / have.len() as f32
        };
        let s = self.statistic(ctx);
        let few_soldiers = by_soldiers
            && self.soldiers_in_stock(ctx)
                < (s as f32 * 0.020_833_3) as u32 / self.plan.producers(product).max(1) as u32;
        if (!have.is_empty() && mean >= BUSY) || few_soldiers {
            gate.k += 1;
            self.plan.add_need(product, Some(self.home), t);
        }
        gate
    }

    /// AI-MIL-6.2, every 631 ticks, not during need stage 1.
    pub(crate) fn weapon_production(&mut self, ctx: &mut Ctx) {
        if self.need_stage == 1 {
            return;
        }
        let n = &ctx.config.names;
        let (smithy, sword) = (n.smithy, n.sword);
        self.weapon_gate = self.producer_needs(
            ctx,
            smithy,
            sword,
            self.profile.max_smithies,
            true,
            self.weapon_gate,
        );
    }

    /// AI-MIL-6.3, every 647 ticks, in need stage 4.
    pub(crate) fn coin_production(&mut self, ctx: &mut Ctx) {
        if self.need_stage < 4 {
            return;
        }
        let n = &ctx.config.names;
        let (mint, coin) = (n.mint, n.coin);
        self.coin_gate = self.producer_needs(
            ctx,
            mint,
            coin,
            self.profile.max_mints,
            false,
            self.coin_gate,
        );
    }

    /// AI-MIL-6.4, every 5 ticks (profile catapults).
    pub(crate) fn catapult_construction(&mut self, ctx: &mut Ctx) {
        if !self.profile.catapults {
            return;
        }
        // AI-MIL-6.4 (T25): no scan while a catapult order exists, pending
        // or executing; the cursor stands still.
        if self.build.busy(crate::construction::Slot::Catapult) {
            return;
        }
        let world = ctx.world;
        let n = ctx.config.names.clone();
        let map = &ctx.military[usize::from(self.id.0)];
        let total = map.cols * map.rows;
        if total == 0 {
            return;
        }
        let stone = self.econ.stock(n.stone);
        let near = world.military_settings(self.id).allocation(0);
        let stone_producer = self.plan.producers(n.stone) > 0;
        // max(1, columns x rows / 600) cells per run.
        for _ in 0..(total / CATAPULT_GRID_DIVISOR).max(1) {
            let index = self.catapult_scan.cursor;
            let cell = (index % map.cols, index / map.cols);
            let own = map.get(cell);
            if own.score > 0 && stone >= CATAPULT_STONE && stone_producer {
                let balance = threat_balance(world, ctx.military, ctx.alive, self.id, cell);
                let k = self.territory.get(cell).catapults as f32 + 1.0;
                let score = (2.0 * balance + own.score as f32) / near.max(0.01) / (k * k);
                if self.catapult_scan.best.is_none_or(|(s, _)| score > s) {
                    self.catapult_scan.best = Some((score, cell));
                }
            }
            self.catapult_scan.cursor = (index + 1) % total;
            if self.catapult_scan.cursor == 0 {
                // A full pass: the best cell above 200 gets an order, and the
                // catapults are switched by the stone stock.
                if let Some((score, cell)) = self.catapult_scan.best.take() {
                    if score > CATAPULT_SCORE && !self.build.busy(Slot::Catapult) {
                        let order = Order {
                            slot: Slot::Catapult,
                            area: Area::Point {
                                at: morigin(cell),
                                radius: CATAPULT_RADIUS,
                            },
                            building: n.catapult,
                            weight: MILITARY_WEIGHT,
                            time: world.game_seconds(),
                            tag: Tag::None,
                        };
                        self.build.issue(order);
                    }
                }
                let on = stone > CATAPULT_STONE;
                for b in ctx.snapshot.of(self.id) {
                    if b.building_type == n.catapult
                        && !b.under_construction
                        && b.production.enabled != on
                    {
                        ctx.out.push(Command::ProductionToggle {
                            player: self.id,
                            at: b.at,
                        });
                    }
                }
            }
        }
    }
}
