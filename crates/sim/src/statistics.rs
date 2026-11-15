//! The player statistics (`specs/UI-ingame.md` UI-GAME-8.5): eight time
//! series per player, one sample per 601-tick cycle (UI-GAME-8.5a).

use std::collections::VecDeque;

use crate::{GoodKind, PlayerId, World};

/// UI-GAME-8.5a: ticks per sampling cycle: the count runs 0 .. 600, the
/// samples at its counts 0 to 4, the roll-over at 600.
pub const SAMPLE_TICKS: u64 = 601;
/// UI-GAME-8.5: samples kept per series; the oldest is dropped.
pub const MAX_SAMPLES: usize = 960;
/// The number of series.
pub const SERIES: usize = 8;
/// UI-GAME-8.5b, 8.5e: the per-cycle counters: wood, plank, stone, food, water,
/// beer, coal, iron ore, gold, iron, coin, tools, weapons, boat.
pub const PRODUCTION_COUNTERS: usize = 14;
/// UI-GAME-8.5e: the coin counter also adds to the running total of series 5.
pub const COIN_COUNTER: u8 = 10;

/// UI-GAME-8.5: the series by index.
pub mod series {
    /// Owned fields, in hundredths of a percent of all fields (Q-198 d).
    pub const LAND: usize = 0;
    pub const BUILDINGS: usize = 1;
    pub const INHABITANTS: usize = 2;
    pub const GOODS: usize = 3;
    pub const MILITARY: usize = 4;
    /// UI-GAME-8.5a, 8.5e: the coins produced so far, a running total.
    pub const COINS: usize = 5;
    /// Mean productivity of the buildings, in percent.
    pub const PRODUCTIVITY: usize = 6;
    /// UI-GAME-8.5a: the frags so far, a running total of the player's kills.
    pub const FRAGS: usize = 7;
}

/// One player's series, oldest sample first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerStatistics {
    pub series: [VecDeque<u32>; SERIES],
    /// UI-GAME-8.5e: what each counter counted per 600 ticks. Empty until the
    /// first new sample at tick 600.
    pub production: [VecDeque<u32>; PRODUCTION_COUNTERS],
    /// UI-GAME-8.5e: the coins produced so far.
    pub coins: u32,
    /// UI-GAME-8.5a: the soldiers killed so far.
    pub frags: u32,
}

impl PlayerStatistics {
    /// FMT-SAV-3.11: the 8 series and the 14 counters of a save (each at
    /// most [`MAX_SAMPLES`], the newest kept). The running totals go on from
    /// the last samples of their series (UI-GAME-8.5a, 8.5e: series 5 the
    /// coins, 7 the frags).
    pub(crate) fn restored(saved: &[Vec<u32>]) -> Self {
        let mut stats = Self::default();
        let keep = |values: &Vec<u32>| -> VecDeque<u32> {
            values[values.len().saturating_sub(MAX_SAMPLES)..]
                .iter()
                .copied()
                .collect()
        };
        for (mine, values) in stats.series.iter_mut().zip(saved) {
            *mine = keep(values);
        }
        for (mine, values) in stats.production.iter_mut().zip(saved.iter().skip(SERIES)) {
            *mine = keep(values);
        }
        stats.coins = stats.series[series::COINS].back().copied().unwrap_or(0);
        stats.frags = stats.series[series::FRAGS].back().copied().unwrap_or(0);
        stats
    }

    fn push(&mut self, index: usize, value: u32) {
        push_sample(&mut self.series[index], value);
    }

    /// UI-GAME-8.5a: at count 600 of a cycle the per-cycle series start a new
    /// sample and counter 7 is sampled.
    fn roll_over(&mut self) {
        for counter in &mut self.production {
            push_sample(counter, 0);
        }
        self.push(series::FRAGS, self.frags);
    }
}

/// UI-GAME-8.5: a sample; the oldest goes after 960.
fn push_sample(samples: &mut VecDeque<u32>, value: u32) {
    if samples.len() == MAX_SAMPLES {
        samples.pop_front();
    }
    samples.push_back(value);
}

/// UI-GAME-8.5a: the roll-overs a game of format 65 had when `tick` ticks
/// had been updated (its cycle was 600 ticks: ticks 600, 1200, ... of the
/// ones before `tick`), at most 960.
fn roll_overs_before(tick: u64) -> usize {
    let n = tick.saturating_sub(1) / 600;
    usize::try_from(n).map_or(MAX_SAMPLES, |n| n.min(MAX_SAMPLES))
}

impl World {
    /// The statistics of `player`, if it is in the game.
    pub fn statistics(&self, player: PlayerId) -> Option<&PlayerStatistics> {
        self.statistics.get(&player)
    }

    /// FMT-SAV-3.11: `player`'s statistics from a save's 8 + 14 series.
    pub(crate) fn restore_statistics(&mut self, player: PlayerId, saved: &[Vec<u32>]) {
        self.statistics
            .insert(player, PlayerStatistics::restored(saved));
    }

    /// `docs/snapshot.md`: a world read from a format before 66 is given what
    /// the production statistics (UI-GAME-8.5a, 8.5e) need from the state:
    /// each counter a zero sample for every roll-over the game has had, so its
    /// current sample counts from the load on, as in a game of this build; and
    /// series 7, which held zeros sampled at tick 4 of each cycle, the frags'
    /// samples of the roll-overs, zero (the kills before the load are not in
    /// the older game). Coins and frags count from the load.
    pub(crate) fn statistics_from_format_65(&mut self) {
        let n = roll_overs_before(self.tick);
        for stats in self.statistics.values_mut() {
            for counter in &mut stats.production {
                *counter = std::iter::repeat_n(0, n).collect();
            }
            stats.series[series::FRAGS] = std::iter::repeat_n(stats.frags, n).collect();
        }
    }

    /// `docs/snapshot.md`: what an older format's world lacks of the rules it
    /// was made with, taken from `rules`, this build's rules of the same game.
    /// Before format 66 the goods had no production counter (UI-GAME-8.5e),
    /// and only the scripts' names give it, so every good would count nothing
    /// for the rest of the game: each good of `rules` with a counter gives it
    /// to the world's good of the same id (one the older world did not list
    /// is listed with its counter alone, which no other rule reads). A world
    /// of this build's format is left as it is.
    pub fn take_later_rules(&mut self, format: u32, rules: &crate::Rules) {
        if format >= 66 {
            return;
        }
        for (&id, good) in &rules.goods {
            if let Some(counter) = good.production_counter {
                self.goods.entry(id).or_default().production_counter = Some(counter);
            }
        }
    }

    /// UI-GAME-8.5e: a worker of `player` finished a work cycle whose building
    /// makes `good`: +1 in its counter's current sample, dropped while the
    /// series is empty (the first 600 ticks); a coin also on the running total.
    pub(crate) fn count_production(&mut self, player: PlayerId, good: u32) {
        let Some(counter) = self.goods.get(&good).and_then(|g| g.production_counter) else {
            return;
        };
        let stats = self.statistics.entry(player).or_default();
        if let Some(sample) = stats.production[usize::from(counter)].back_mut() {
            *sample += 1;
        }
        if counter == COIN_COUNTER {
            stats.coins += 1;
        }
    }

    /// UI-GAME-8.5a: `player`'s soldier or catapult killed a soldier.
    pub(crate) fn count_frag(&mut self, player: PlayerId) {
        self.statistics.entry(player).or_default().frags += 1;
    }

    /// UI-GAME-8.5a: at counts 0 to 4 of each 601-tick cycle (the tick plus
    /// the sampling phase, FMT-SAV-3.11), in this order: land; goods and the
    /// coins; buildings; inhabitants and military; productivity. The
    /// per-cycle series and the frags roll over at count 600.
    pub(crate) fn update_statistics(&mut self) {
        let step = (self.tick + self.sample_phase) % SAMPLE_TICKS;
        if step == SAMPLE_TICKS - 1 {
            for player in self.players.clone() {
                self.statistics.entry(player).or_default().roll_over();
            }
            return;
        }
        if step > 4 {
            return;
        }
        for player in self.players.clone() {
            let values: Vec<(usize, u32)> = match step {
                0 => vec![(series::LAND, self.land_share(player))],
                1 => vec![
                    (series::GOODS, self.goods_count(player)),
                    (
                        series::COINS,
                        self.statistics.get(&player).map_or(0, |s| s.coins),
                    ),
                ],
                2 => vec![(series::BUILDINGS, self.building_count(player))],
                3 => vec![
                    (series::INHABITANTS, self.inhabitants(player)),
                    (series::MILITARY, self.military_strength(player)),
                ],
                _ => vec![(series::PRODUCTIVITY, self.mean_productivity(player))],
            };
            let stats = self.statistics.entry(player).or_default();
            for (index, value) in values {
                stats.push(index, value);
            }
        }
    }

    /// Owned fields as a share of all fields, in hundredths of a percent.
    fn land_share(&self, player: PlayerId) -> u32 {
        let owned = self.owners.iter().filter(|&&o| o == Some(player)).count() as u64;
        let all = self.owners.len().max(1) as u64;
        (owned * 10_000 / all) as u32
    }

    /// Every building of the player, construction sites included (Q-198 d).
    fn building_count(&self, player: PlayerId) -> u32 {
        self.buildings().filter(|b| b.owner == player).count() as u32
    }

    /// The warehouses of `player` with their stock.
    fn stocks_of(&self, player: PlayerId) -> impl Iterator<Item = &[(u32, u32)]> {
        self.warehouses
            .iter()
            .filter(move |(&at, _)| self.building_at(at).is_some_and(|b| b.owner == player))
            .map(|(_, w)| w.stock.as_slice())
    }

    /// The sum of all building stocks: the wares in the warehouses and the
    /// inputs stored in the other buildings (Q-198 d).
    fn goods_count(&self, player: PlayerId) -> u32 {
        let ware = |good: u32| {
            self.goods
                .get(&good)
                .is_some_and(|g| g.kind == GoodKind::Ware)
        };
        let stored: u32 = self
            .stocks_of(player)
            .flatten()
            .filter(|&&(good, _)| ware(good))
            .map(|&(_, n)| n)
            .sum();
        let inputs: u32 = self
            .inputs
            .iter()
            .filter(|(&at, _)| self.building_at(at).is_some_and(|b| b.owner == player))
            .flat_map(|(_, list)| list.iter().map(|&(_, n)| n))
            .sum();
        stored + inputs
    }

    /// Settlers outside the warehouses except carriers, and the stocked
    /// settlers of every kind except helpers and donkeys.
    fn inhabitants(&self, player: PlayerId) -> u32 {
        let outside = self.settlers().filter(|(_, s)| s.player == player).count() as u32;
        let stocked: u32 = self
            .stocks_of(player)
            .flatten()
            .filter(|&&(good, _)| {
                self.goods
                    .get(&good)
                    .is_some_and(|g| g.kind == GoodKind::Settler)
                    && Some(good) != self.roles.helper
                    && Some(good) != self.roles.donkey
            })
            .map(|&(_, n)| n)
            .sum();
        outside + stocked
    }

    /// The sum over the soldiers of rank index + 1, stocked ones included.
    fn military_strength(&self, player: PlayerId) -> u32 {
        let weight = |profession: u32| self.soldier_rank(profession).map_or(0, |r| r as u32 + 1);
        let outside: u32 = self
            .settlers()
            .filter(|(_, s)| s.player == player)
            .map(|(_, s)| weight(s.profession))
            .sum();
        let stocked: u32 = self
            .stocks_of(player)
            .flatten()
            .map(|&(good, n)| weight(good) * n)
            .sum();
        outside + stocked
    }

    /// The mean productivity in percent of the player's finished buildings
    /// that have a worker; one that never produced counts as 0 (Q-198 d).
    fn mean_productivity(&self, player: PlayerId) -> u32 {
        let values: Vec<u32> = self
            .buildings()
            .filter(|b| b.owner == player && !b.under_construction)
            .filter(|b| {
                self.building_types
                    .get(&b.building_type)
                    .is_some_and(|t| t.worker.is_some())
            })
            .map(|b| b.productivity.max(0) as u32)
            .collect();
        if values.is_empty() {
            0
        } else {
            values.iter().sum::<u32>() / values.len() as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requests::tests::{self as req, HQ};
    use crate::test_support::*;

    // UI-GAME-8.5a: one sample per 601-tick cycle per series, each taken at
    // its count of the cycle; the roll-over at count 600.
    // spec: UI-GAME-8.5a
    #[test]
    fn samples_every_601_ticks() {
        let mut rules = req::rules(&req::full_stock());
        for (good, kind) in [
            (req::PLANK, GoodKind::Ware),
            (req::STONE, GoodKind::Ware),
            (req::AXE, GoodKind::Ware),
            (req::HAMMER, GoodKind::Ware),
            (req::SHOVEL, GoodKind::Ware),
            (req::HELPER, GoodKind::Settler),
            (req::BUILDER, GoodKind::Settler),
            (req::LEVELLER, GoodKind::Settler),
            (req::WOODCUTTER, GoodKind::Settler),
        ] {
            rules.goods.entry(good).or_default().kind = kind;
        }
        let setup = crate::GameSetup {
            players: vec![(P0, HQ)],
            start_resources: 0,
            local_player: P0,
            ..Default::default()
        };
        let mut world = World::new_game(9, flat(40, 40), rules, &setup);
        let mut lengths = Vec::new();
        for _ in 0..1205 {
            world.step();
            let s = world.statistics(P0).unwrap();
            lengths.push(s.series.iter().map(VecDeque::len).collect::<Vec<_>>());
        }
        // Count 0: land only; 1: goods, the coins; ... 4: productivity. The
        // frags roll over at count 600 (tick 600), the next land sample is
        // count 0 of the second cycle (tick 601).
        assert_eq!(lengths[0], [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(lengths[1], [1, 0, 0, 1, 0, 1, 0, 0]);
        assert_eq!(lengths[2], [1, 1, 0, 1, 0, 1, 0, 0]);
        assert_eq!(lengths[3], [1, 1, 1, 1, 1, 1, 0, 0]);
        assert_eq!(lengths[4], [1, 1, 1, 1, 1, 1, 1, 0]);
        assert_eq!(lengths[599], [1, 1, 1, 1, 1, 1, 1, 0]);
        assert_eq!(lengths[600], [1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(lengths[601], [2, 1, 1, 1, 1, 1, 1, 1]);
        // Tick 1201 is count 600 of the second cycle, 1204 count 2 of the third.
        assert_eq!(lengths[1201], [2, 2, 2, 2, 2, 2, 2, 2]);
        assert_eq!(lengths[1204], [3, 3, 2, 3, 2, 3, 2, 2]);
        let s = world.statistics(P0).unwrap();
        // The headquarters is the one building; its land is part of the map.
        assert_eq!(s.series[series::BUILDINGS][0], 1);
        assert!(s.series[series::LAND][0] > 0 && s.series[series::LAND][0] < 10_000);
        // Goods: the wares of the start stock (planks, stones, tools).
        let wares = 22 + 34 + 3 + 8 + 2;
        assert_eq!(s.series[series::GOODS][0], wares);
        // Inhabitants: the stocked builders, levellers and woodcutters.
        assert_eq!(s.series[series::INHABITANTS][0], 5 + 3 + 4);
        assert_eq!(s.series[series::MILITARY][0], 0);
        assert!(world.warehouse(HQ).is_some());
    }

    // UI-GAME-8.5e, 8.5a: a count on an empty series is dropped, so nothing made in
    // the first 600 ticks is recorded, while series 5's total counts it. From tick
    // 600 each cycle has its sample; a good without a counter counts nothing; a
    // group's members add to its counter; the totals are sampled.
    // spec: UI-GAME-8.5e, UI-GAME-8.5a
    #[test]
    fn production_counters() {
        const COIN: u32 = 500;
        const BREAD: u32 = 501;
        const CORN: u32 = 502;
        let mut rules = req::rules(&req::full_stock());
        rules.goods.entry(COIN).or_default().production_counter = Some(COIN_COUNTER);
        rules.goods.entry(BREAD).or_default().production_counter = Some(3);
        rules.goods.entry(CORN).or_default();
        let setup = crate::GameSetup {
            players: vec![(P0, HQ)],
            start_resources: 0,
            local_player: P0,
            ..Default::default()
        };
        let mut world = World::new_game(9, flat(40, 40), rules, &setup);
        let run_to = |world: &mut World, tick: u64| {
            while world.tick() < tick {
                world.step();
            }
        };
        run_to(&mut world, 10);
        world.count_production(P0, COIN);
        world.count_production(P0, BREAD);
        world.count_production(P0, CORN);
        let stats = |world: &World| world.statistics(P0).unwrap().clone();
        assert!(stats(&world).production.iter().all(VecDeque::is_empty));
        assert_eq!(stats(&world).coins, 1);
        run_to(&mut world, 610);
        assert!(stats(&world).production.iter().all(|c| c.len() == 1));
        assert_eq!(stats(&world).series[series::COINS].back(), Some(&1));
        world.count_production(P0, BREAD);
        world.count_production(P0, BREAD);
        world.count_production(P0, COIN);
        world.count_frag(P0);
        run_to(&mut world, 1210);
        world.count_production(P0, BREAD);
        let s = stats(&world);
        assert_eq!(s.production[3], [2, 1]);
        assert_eq!(s.production[usize::from(COIN_COUNTER)], [1, 0]);
        assert_eq!(s.production[0], [0, 0]);
        assert_eq!(s.coins, 2);
        assert_eq!(s.series[series::COINS].back(), Some(&2));
        assert_eq!(s.series[series::FRAGS], [0, 1]);
    }

    // UI-GAME-8.5: at most 960 samples; the oldest goes.
    #[test]
    fn keeps_960_samples() {
        let mut stats = PlayerStatistics::default();
        for i in 0..1000 {
            stats.push(0, i);
        }
        assert_eq!(stats.series[0].len(), MAX_SAMPLES);
        assert_eq!(stats.series[0][0], 40);
    }
}
