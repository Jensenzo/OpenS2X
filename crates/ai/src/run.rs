//! Running a game with computer players, and reporting on it: used by the
//! headless executable, the viewer and the tests.
//!
//! The report keeps names in a `HashMap` (lookups only) and measures its
//! own run time with `Instant`; neither reaches a decision or the world.
#![allow(clippy::disallowed_types)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use s2_sim::grid::Point;
use s2_sim::{Command, PlayerId, SettlerJob, World};

use crate::AiSystem;

/// What a run has seen so far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// Commands the AI issued, by kind.
    pub issued: BTreeMap<String, usize>,
    /// Commands the world rejected, by kind and reason.
    pub rejected: BTreeMap<String, usize>,
    /// Attack orders: tick, player, target field, soldiers ordered.
    pub attacks: Vec<(u64, PlayerId, Point, u32)>,
    /// A building that changed owner while standing: tick, field, from, to.
    pub conquests: Vec<(u64, Point, PlayerId, PlayerId)>,
    /// Ticks in which at least one duel was being fought.
    pub duel_ticks: u64,
    /// The tick in which the land of two players first touched (TER-5.4).
    pub first_contact: Option<u64>,
    /// Wall-clock time spent in the AI and in the world (a measurement of the
    /// run, not part of any game state).
    pub ai_time: std::time::Duration,
    pub sim_time: std::time::Duration,
}

/// A game with an AI system, stepped one tick at a time.
pub struct Runner {
    pub world: World,
    pub ai: AiSystem,
    pub stats: Stats,
    /// Building type and good names by id, for the report.
    names: HashMap<u32, String>,
    /// Owner and creation number of the building on each field last tick.
    owners: BTreeMap<Point, (PlayerId, u32)>,
}

fn kind_of(command: &Command) -> String {
    let text = format!("{command:?}");
    text.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}

impl Runner {
    pub fn new(world: World, ai: AiSystem, names: HashMap<u32, String>) -> Self {
        let owners = world
            .buildings()
            .map(|b| (b.at, (b.owner, b.serial)))
            .collect();
        Self {
            world,
            ai,
            stats: Stats::default(),
            names,
            owners,
        }
    }

    /// One tick: the world's update, then the AI thinks as its last part
    /// (AI-2.1a) and its commands take effect inside the tick.
    pub fn step(&mut self) {
        let tick = self.world.tick();
        let started = std::time::Instant::now();
        let ai = &mut self.ai;
        let mut ai_time = std::time::Duration::ZERO;
        let mut issued = Vec::new();
        self.world.step_then(|world, rng, numbers, routes| {
            let thinking = std::time::Instant::now();
            let commands = ai.tick(world, rng, numbers, routes);
            ai_time = thinking.elapsed();
            issued = commands.clone();
            commands
        });
        self.stats.ai_time += ai_time;
        self.stats.sim_time += started.elapsed().saturating_sub(ai_time);
        for command in issued {
            *self.stats.issued.entry(kind_of(&command)).or_default() += 1;
            if let Command::Attack {
                player,
                target,
                count,
                ..
            } = command
            {
                self.stats.attacks.push((tick, player, target, count));
            }
        }
        for result in self.world.last_results() {
            if let Err(reason) = result.outcome {
                *self
                    .stats
                    .rejected
                    .entry(format!("{}: {reason:?}", kind_of(&result.command)))
                    .or_default() += 1;
            }
        }
        if !self.world.duels().is_empty() {
            self.stats.duel_ticks += 1;
        }
        if self.stats.first_contact.is_none() {
            let slots = 0..crate::SLOTS as u8;
            let met = slots.clone().any(|a| {
                slots
                    .clone()
                    .any(|b| a < b && self.world.have_met(PlayerId(a), PlayerId(b)))
            });
            if met {
                self.stats.first_contact = Some(self.world.tick());
            }
        }
        // A building that stands on the same field but has a new owner.
        let mut now: BTreeMap<Point, (PlayerId, u32)> = BTreeMap::new();
        for b in self.world.buildings() {
            if let Some(&(old_owner, old_serial)) = self.owners.get(&b.at) {
                if old_owner != b.owner && old_serial != b.serial {
                    self.stats
                        .conquests
                        .push((self.world.tick(), b.at, old_owner, b.owner));
                }
            }
            now.insert(b.at, (b.owner, b.serial));
        }
        self.owners = now;
    }

    /// Write the game (world, computer players and the report's counts)
    /// to a snapshot (`docs/snapshot.md`). The run times are not written.
    pub fn put_state(&self, w: &mut s2_sim::snap::Writer) {
        w.raw(&self.world.snapshot());
        self.ai.put_state(w);
        let s = &self.stats;
        w.put(&s.issued);
        w.put(&s.rejected);
        w.put(&s.attacks);
        w.put(&s.conquests);
        w.put(&s.duel_ticks);
        w.put(&s.first_contact);
    }

    /// Continue the game [`Runner::put_state`] wrote. The runner must have
    /// been set up for the same map and rules (its AI configuration and
    /// names are kept).
    pub fn get_state(
        &mut self,
        r: &mut s2_sim::snap::Reader,
    ) -> Result<(), s2_sim::snap::SnapError> {
        let world = World::read_snapshot(r)?;
        self.ai.get_state(r)?;
        self.stats = Stats {
            issued: r.get()?,
            rejected: r.get()?,
            attacks: r.get()?,
            conquests: r.get()?,
            duel_ticks: r.get()?,
            first_contact: r.get()?,
            ..Stats::default()
        };
        // The owners seen in the last tick are those of the world now.
        self.owners = world
            .buildings()
            .map(|b| (b.at, (b.owner, b.serial)))
            .collect();
        self.world = world;
        Ok(())
    }

    /// Run `ticks` ticks, calling `report` every `every` ticks and at the end.
    pub fn run(&mut self, ticks: u64, every: u64, mut report: impl FnMut(&Self)) {
        for done in 1..=ticks {
            self.step();
            if done % every.max(1) == 0 || done == ticks {
                report(self);
            }
        }
    }

    /// The report of the moment for the given computer players.
    pub fn describe(&self, players: &[PlayerId]) -> String {
        let mut out = String::new();
        let world = &self.world;
        writeln!(out, "== tick {} ==", world.tick()).ok();
        for &p in players {
            let mut by_type: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
            for b in world.buildings().filter(|b| b.owner == p) {
                let name = self.names.get(&b.building_type).map_or("?", String::as_str);
                let entry = by_type.entry(name).or_default();
                if b.under_construction {
                    entry.1 += 1;
                } else {
                    entry.0 += 1;
                }
            }
            let buildings: Vec<String> = by_type
                .iter()
                .map(|(name, (done, building))| {
                    if *building > 0 {
                        format!("{name} {done}+{building}")
                    } else {
                        format!("{name} {done}")
                    }
                })
                .collect();
            let (mut inside, mut out_in_field) = (0u32, 0u32);
            let mut ranks = [0u32; s2_sim::SOLDIER_RANKS];
            for (_, s) in world.settlers().filter(|(_, s)| s.player == p) {
                let Some(rank) = world.soldier_rank(s.profession) else {
                    continue;
                };
                ranks[rank] += 1;
                match s.job {
                    SettlerJob::Garrison { .. } | SettlerJob::ToMilitary { .. } => inside += 1,
                    _ => out_in_field += 1,
                }
            }
            let mut stock_ranks = [0u32; s2_sim::SOLDIER_RANKS];
            for (at, _) in world
                .warehouses()
                .filter(|(at, _)| world.building_at(*at).is_some_and(|b| b.owner == p))
            {
                for (total, n) in stock_ranks.iter_mut().zip(world.stock_soldiers(at)) {
                    *total += n;
                }
            }
            let stock: u32 = stock_ranks.iter().sum();
            let roads = world.roads().filter(|r| r.owner == p).count();
            // How much of its landmass the player owns.
            let landmass = self.ai.player(p).map(|a| {
                let t = world.terrain();
                (0..t.height)
                    .flat_map(|y| (0..t.width).map(move |x| (x, y)))
                    .filter(|&f| world.continent_of(f) == Some(a.home_landmass))
                    .count()
            });
            let active = self.ai.is_active(p);
            let cells = self.ai.player(p).map_or(0, |a| a.cells.len());
            writeln!(
                out,
                "player {}: {} fields (landmass {}), {} roads, soldiers {} in buildings + {} out + \
                 {} in stock, AI {}, {} cells",
                p.0,
                world.owned_field_count(p),
                landmass.map_or("?".to_string(), |n| n.to_string()),
                roads,
                inside,
                out_in_field,
                stock,
                if active { "on" } else { "off" },
                cells
            )
            .ok();
            writeln!(out, "  buildings: {}", buildings.join(", ")).ok();
            let military: Vec<String> = world
                .buildings()
                .filter(|b| b.owner == p && !b.under_construction)
                .filter_map(|b| {
                    let name = self
                        .names
                        .get(&world.military_at(b.at).map(|_| b.building_type)?)?;
                    Some(format!("{name}@{},{}", b.at.0, b.at.1))
                })
                .collect();
            writeln!(out, "  military: {}", military.join(" ")).ok();
            let orders = self.stats.attacks.iter().filter(|a| a.1 == p);
            writeln!(
                out,
                "  ranks: out of stock {ranks:?}, in stock {stock_ranks:?}; attack orders {}, \
                 soldiers ordered {}, buildings taken {}, lost {}",
                orders.clone().count(),
                orders.map(|a| a.3).sum::<u32>(),
                self.stats.conquests.iter().filter(|c| c.3 == p).count(),
                self.stats.conquests.iter().filter(|c| c.2 == p).count(),
            )
            .ok();
            let stock: Vec<String> = [
                "wood", "plank", "stone", "fish", "bread", "flesh", "coal", "iron", "ironOre",
                "gold", "beer", "sword", "shield", "coin",
            ]
            .iter()
            .filter_map(|name| {
                let id = self
                    .names
                    .iter()
                    .find(|(_, n)| n == name)
                    .map(|(&id, _)| id)?;
                let total: u32 = world
                    .warehouses()
                    .filter(|(at, _)| world.building_at(*at).is_some_and(|b| b.owner == p))
                    .map(|(_, st)| {
                        st.iter()
                            .filter(|(g, _)| *g == id)
                            .map(|(_, c)| *c)
                            .sum::<u32>()
                    })
                    .sum();
                Some(format!("{name} {total}"))
            })
            .collect();
            writeln!(out, "  stock: {}", stock.join(", ")).ok();
            // Jams: wares lying at flags, flags at the limit, carriers
            // stopped before a full flag (TRN-CARRY-3.2).
            let limit = world.flag_package_limit();
            let flags = world.flags_in_order(p);
            let lying: usize = flags.iter().map(|&f| world.wares_at(f).len()).sum();
            let full = flags
                .iter()
                .filter(|&&f| world.flag_load(f) >= limit)
                .count();
            let waiting = world
                .carriers()
                .iter()
                .enumerate()
                .filter(|&(i, c)| {
                    world
                        .roads()
                        .any(|r| r.owner == p && (r.carrier == Some(i) || r.donkey == Some(i)))
                        && matches!(
                            c.task,
                            s2_sim::transport::CarrierTask::Carry {
                                entry: s2_sim::transport::Entry::Open { stop: true },
                                ..
                            }
                        )
                        && c.walker.next.is_none()
                })
                .count();
            writeln!(
                out,
                "  transport: {} wares at {} flags, {} flags full, {} carriers waiting",
                lying,
                flags.len(),
                full,
                waiting
            )
            .ok();
        }
        out
    }

    /// The tail of the report: commands, rejections, attacks, conquests.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        writeln!(
            out,
            "time: AI {:.1} s, simulation {:.1} s",
            self.stats.ai_time.as_secs_f64(),
            self.stats.sim_time.as_secs_f64()
        )
        .ok();
        writeln!(out, "commands issued: {:?}", self.stats.issued).ok();
        writeln!(out, "commands rejected: {:?}", self.stats.rejected).ok();
        writeln!(
            out,
            "lands first touched: {}",
            self.stats
                .first_contact
                .map_or("never".to_string(), |t| format!("tick {t}"))
        )
        .ok();
        writeln!(
            out,
            "attack orders: {} (first {:?}); ticks with a duel: {}; buildings conquered: {}",
            self.stats.attacks.len(),
            self.stats.attacks.first(),
            self.stats.duel_ticks,
            self.stats.conquests.len()
        )
        .ok();
        for (tick, at, from, to) in self.stats.conquests.iter().take(10) {
            writeln!(
                out,
                "  tick {tick}: {at:?} from player {} to {}",
                from.0, to.0
            )
            .ok();
        }
        out
    }
}
