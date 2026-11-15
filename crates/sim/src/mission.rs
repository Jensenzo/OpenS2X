//! Missions (`specs/CAM-campaign-missions.md`): the map's triggers (CAM-2),
//! the mission target with victory and defeat (CAM-3), and what a mission
//! script changes in the game (CAM-1.3): stock, building locks, buildings
//! and ships placed finished.
//!
//! A world has no mission until [`World::set_mission`] gives it one; test
//! worlds and the AI's games without one never win or lose.

use std::collections::{BTreeMap, BTreeSet};

use crate::grid::Point;
use crate::movement::Walker;
use crate::{PlayerId, Rejection, Settler, SettlerJob, World};

/// CAM-2.2: a trigger is evaluated once per 5 s of game time (50 ticks).
pub const TRIGGER_PERIOD: u32 = 50;

/// CAM-3.1b: the radius explored around a victory region at a campaign
/// game's set-up.
const VICTORY_EXPLORE_RADIUS: u32 = 2;

/// CAM-3.1: the mission targets of the map header's first closing value.
pub mod target {
    pub const ENDLESS: u8 = 0;
    pub const DESTROY_ENEMIES: u8 = 1;
    pub const CONQUER: u8 = 2;
    pub const COINS: u8 = 3;
    pub const PORTAL: u8 = 4;
}

/// CAM-2.1: what a trigger tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    /// 1: the player has explored the field.
    Explored,
    /// 2: the player's territory covers the field.
    Territory,
    /// 3: like 2, and reaching it completes the mission.
    Victory,
}

impl TriggerKind {
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::Explored),
            2 => Some(Self::Territory),
            3 => Some(Self::Victory),
            _ => None,
        }
    }
}

/// CAM-2.1: a trigger of the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    pub kind: TriggerKind,
    pub field: Point,
    pub player: PlayerId,
    /// The name `onTriggerIsActive` receives.
    pub name: String,
    pub active: bool,
    /// Ticks since the last evaluation (CAM-2.2).
    pub timer: u32,
}

/// CAM-3.2, 3.4: why a player has lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossReason {
    /// "Castle destroyed".
    CastleDestroyed,
    /// "Mission not completed": another team won.
    MissionNotCompleted,
}

/// CAM-3.2, 3.4: a player's result, with the tick it came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won { tick: u64 },
    Lost { tick: u64, reason: LossReason },
}

/// What happened to the mission in the last step, for the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionEvent {
    Lost {
        player: PlayerId,
        reason: LossReason,
    },
    Won {
        player: PlayerId,
    },
    /// CAM-3.5: the map was marked completed with this value (1 to 4).
    Completed {
        value: u8,
    },
}

/// CAM-3.7: the cheats typed in the game that change it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cheat {
    /// `INeedSomeHelp`: the map is completed with the current target.
    INeedSomeHelp,
    /// `ShowMeTheWorld`: the player has explored every field.
    ShowMeTheWorld,
    /// `GiveMeSomeMore`: 10 of every good into the headquarters.
    GiveMeSomeMore,
}

/// The mission of a game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mission {
    /// CAM-3.1
    pub target: u8,
    /// CAM-2, in map order.
    pub triggers: Vec<Trigger>,
    pub outcomes: BTreeMap<PlayerId, Outcome>,
    /// CAM-1.3 `lockBuilding`: (player, building type).
    pub locks: BTreeSet<(PlayerId, u32)>,
    /// CAM-3.5: the value the map was marked completed with.
    pub completed: Option<u8>,
}

impl Mission {
    pub fn new(target: u8, triggers: Vec<Trigger>) -> Self {
        Self {
            target,
            triggers,
            ..Self::default()
        }
    }
}

impl World {
    /// Give the game its mission (CAM-3.1 target, CAM-2 triggers).
    pub fn set_mission(&mut self, mission: Mission) {
        self.mission = Some(mission);
    }

    pub fn mission(&self) -> Option<&Mission> {
        self.mission.as_ref()
    }

    /// What happened to the mission in the last step.
    pub fn mission_events(&self) -> &[MissionEvent] {
        &self.mission_events
    }

    /// CAM-3.1b (Q-1003 answered 2026-11-09): at a campaign game's set-up,
    /// in the step of CAM-3.1a, the surroundings of every victory region
    /// (trigger kind 3) start explored for its player: the fields within
    /// distance 2 of its field (19, the explore of ECO-HQ-3.3). Kind 1 and 2
    /// triggers are not explored. The game draws no symbol for the gate; the
    /// minimap shows this patch. Called for a new campaign game only (a
    /// loaded one has its exploration from the save).
    pub fn explore_victory_regions(&mut self) {
        let Some(mission) = self.mission.as_ref() else {
            return;
        };
        let regions: Vec<(PlayerId, Point)> = mission
            .triggers
            .iter()
            .filter(|t| t.kind == TriggerKind::Victory)
            .map(|t| (t.player, t.field))
            .collect();
        for (player, field) in regions {
            if self.terrain.contains(field) {
                self.explore(player, field, VICTORY_EXPLORE_RADIUS);
            }
        }
    }

    /// CAM-2.3: the first trigger (list order) whose flag is set.
    pub fn active_trigger(&self) -> Option<&str> {
        self.mission
            .as_ref()?
            .triggers
            .iter()
            .find(|t| t.active)
            .map(|t| t.name.as_str())
    }

    /// CAM-1.3 `lockBuilding`: whether `player` may not choose the type.
    pub fn is_locked(&self, player: PlayerId, building: u32) -> bool {
        self.mission
            .as_ref()
            .is_some_and(|m| m.locks.contains(&(player, building)))
    }

    pub fn outcome(&self, player: PlayerId) -> Option<Outcome> {
        self.mission.as_ref()?.outcomes.get(&player).copied()
    }

    /// One tick of the mission, at the end of the step: the triggers
    /// (CAM-2.2), defeat (CAM-3.2), then victory (CAM-3.3).
    pub(crate) fn update_mission(&mut self) {
        let Some(mut mission) = self.mission.take() else {
            return;
        };
        self.update_triggers(&mut mission);
        self.update_defeat(&mut mission);
        self.update_victory(&mut mission);
        self.mission = Some(mission);
    }

    /// CAM-2.2: each trigger's timer runs; at 5 s its condition is tested.
    fn update_triggers(&mut self, mission: &mut Mission) {
        for i in 0..mission.triggers.len() {
            let t = &mut mission.triggers[i];
            t.timer += 1;
            if t.timer < TRIGGER_PERIOD {
                continue;
            }
            t.timer = 0;
            let (kind, field, player) = (t.kind, t.field, t.player);
            if !self.terrain.contains(field) {
                continue;
            }
            let met = match kind {
                TriggerKind::Explored => self.is_explored(player, field),
                TriggerKind::Territory | TriggerKind::Victory => {
                    self.field_owner(field) == Some(player)
                }
            };
            if !met {
                continue;
            }
            if kind == TriggerKind::Victory {
                self.mark_completed(mission, target::PORTAL);
            }
            mission.triggers[i].active = true;
        }
    }

    /// CAM-3.5: the map is completed with `value`.
    fn mark_completed(&mut self, mission: &mut Mission, value: u8) {
        if mission.completed.is_none() {
            mission.completed = Some(value);
            self.mission_events.push(MissionEvent::Completed { value });
        }
    }

    /// CAM-3.2: a player without a headquarters has lost.
    fn update_defeat(&mut self, mission: &mut Mission) {
        let Some(hq) = self.headquarters else {
            return;
        };
        for &player in &self.players {
            if mission.outcomes.contains_key(&player) {
                continue;
            }
            let has_hq = self
                .buildings()
                .any(|b| b.owner == player && b.building_type == hq);
            if !has_hq {
                let reason = LossReason::CastleDestroyed;
                mission.outcomes.insert(
                    player,
                    Outcome::Lost {
                        tick: self.tick,
                        reason,
                    },
                );
                self.mission_events
                    .push(MissionEvent::Lost { player, reason });
            }
        }
    }

    /// The team of `player`: itself and its allies, in player order.
    fn team(&self, player: PlayerId) -> Vec<PlayerId> {
        self.players
            .iter()
            .copied()
            .filter(|&p| !self.are_enemies(player, p))
            .collect()
    }

    /// CAM-3.3, 3.4
    fn update_victory(&mut self, mission: &mut Mission) {
        let total = self.terrain.len() as u64;
        for &player in &self.players.clone() {
            if mission.outcomes.contains_key(&player) {
                continue;
            }
            let team = self.team(player);
            let won = match mission.target {
                target::DESTROY_ENEMIES => {
                    let enemies: Vec<PlayerId> = self
                        .players
                        .iter()
                        .copied()
                        .filter(|&p| self.are_enemies(player, p))
                        .collect();
                    let destroyed = enemies
                        .iter()
                        .filter(|p| matches!(mission.outcomes.get(p), Some(Outcome::Lost { .. })))
                        .count();
                    destroyed == enemies.len()
                }
                target::CONQUER => {
                    // The sum of the team's percentages above 74 (Q-187).
                    let owned = self
                        .owners
                        .iter()
                        .filter(|o| o.is_some_and(|o| team.contains(&o)))
                        .count() as u64;
                    total > 0 && owned * 100 > 74 * total
                }
                target::COINS => {
                    let Some(coin) = self.roles.coin else {
                        continue;
                    };
                    let coins: u32 = self
                        .warehouses
                        .iter()
                        .filter(|(at, _)| {
                            self.building_at(**at)
                                .is_some_and(|b| team.contains(&b.owner))
                        })
                        .map(|(_, w)| w.count(coin))
                        .sum();
                    coins > 99
                }
                _ => false,
            };
            if !won {
                continue;
            }
            for &p in &self.players.clone() {
                if mission.outcomes.contains_key(&p) {
                    continue;
                }
                let outcome = if team.contains(&p) {
                    self.mission_events.push(MissionEvent::Won { player: p });
                    Outcome::Won { tick: self.tick }
                } else {
                    let reason = LossReason::MissionNotCompleted;
                    self.mission_events
                        .push(MissionEvent::Lost { player: p, reason });
                    Outcome::Lost {
                        tick: self.tick,
                        reason,
                    }
                };
                mission.outcomes.insert(p, outcome);
            }
            if team.contains(&self.local_player) {
                self.mark_completed(mission, mission.target);
            }
            break;
        }
    }

    // ---- The script's commands (CAM-1.3, 1.4) ----

    fn mission_mut(&mut self) -> &mut Mission {
        self.mission.get_or_insert_with(Mission::default)
    }

    /// The warehouse of the player's first headquarters, in map order.
    fn headquarters_of(&self, player: PlayerId) -> Option<Point> {
        let hq = self.headquarters?;
        self.buildings()
            .find(|b| b.owner == player && b.building_type == hq)
            .map(|b| b.at)
            .filter(|at| self.warehouses.contains_key(at))
    }

    /// `menu.addResource`
    pub(crate) fn add_goods(
        &mut self,
        player: PlayerId,
        good: u32,
        amount: u32,
    ) -> Result<(), Rejection> {
        if !self.goods.contains_key(&good) {
            return Err(Rejection::UnknownGood);
        }
        let at = self.headquarters_of(player).ok_or(Rejection::NoBuilding)?;
        self.warehouses
            .get_mut(&at)
            .expect("a warehouse")
            .add(good, amount);
        self.made(Some(good), amount);
        Ok(())
    }

    /// `menu.clearResources`
    pub(crate) fn clear_goods(&mut self, player: PlayerId) -> Result<(), Rejection> {
        let at = self.headquarters_of(player).ok_or(Rejection::NoBuilding)?;
        let stock = std::mem::take(&mut self.warehouses.get_mut(&at).expect("a warehouse").stock);
        for (good, n) in stock {
            self.used(good, n);
        }
        Ok(())
    }

    /// `menu.lockBuilding`
    pub(crate) fn lock_building(
        &mut self,
        player: PlayerId,
        building: u32,
    ) -> Result<(), Rejection> {
        if self.building_type(building).is_none() {
            return Err(Rejection::UnknownBuildingType);
        }
        self.mission_mut().locks.insert((player, building));
        Ok(())
    }

    /// `menu.createBuilding` and `createBuildingWithSettler`: the building
    /// finished on `at` (with its flag), then the settler in it: a soldier
    /// rank occupies a military building, any other profession is its
    /// worker. The site rules of ECO-PLACE are not asked (Q-187).
    pub(crate) fn create_building(
        &mut self,
        player: PlayerId,
        building: u32,
        at: Point,
        settler: Option<u32>,
    ) -> Result<(), Rejection> {
        let size = self
            .building_type(building)
            .ok_or(Rejection::UnknownBuildingType)?
            .size;
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        if self.building_at(at).is_some() || self.flag_at(at).is_some() {
            return Err(Rejection::TooCloseToBuilding);
        }
        let new_flag = self.new_flag_of(at);
        let mut touched = self.put_building(player, building, size, at, false);
        // ECO-HQ-4.8b: a new flag's height ring.
        if let Some(flag) = new_flag {
            touched.extend(self.flag_height_ring(flag));
        }
        self.complete(at);
        self.changed(&touched);
        let Some(profession) = settler else {
            return Ok(());
        };
        if !self.goods.contains_key(&profession) {
            return Err(Rejection::UnknownGood);
        }
        let target = self.target_of(at).expect("a building");
        let soldier = self.soldier_rank(profession).is_some()
            && self.military.contains_key(&at)
            && self.building_types[&building].soldier_limit > 0;
        if soldier {
            // MIL-SOL-1.2: a new soldier has full health.
            let rank = self.soldier_rank(profession).expect("a soldier");
            let health = self.military_rules.max_health[rank];
            // SIM-START-4.2: a settler made.
            let number = self.new_number();
            let id = self.add_settler(Settler {
                number,
                player,
                profession,
                list: crate::SettlerList::Soldiers,
                walker: Walker::new(at),
                job: SettlerJob::Garrison {
                    target,
                    health,
                    heal: 0,
                },
                timer: 0,
                carrying: None,
            });
            self.military
                .get_mut(&at)
                .expect("military")
                .soldiers
                .push(id);
        } else {
            // ECO-REQ-4.2: as a worker that has arrived.
            self.seat_worker(player, profession, at);
        }
        Ok(())
    }

    /// `menu.createStreet` (CAM-1.3b): nothing unless the start holds the
    /// player's flag (1); the route is the road-building mode's search
    /// (TRN-ROAD-2; none found: nothing built, medium). The end gets a flag
    /// only when it lies on another road and a flag fits there
    /// (ECO-PLACE-2.2), which splits that road (2, TRN-ROAD-4.2); then the
    /// road is built, as the player's, when the end holds a flag (3).
    /// An end on a free field gets neither flag nor road.
    pub(crate) fn create_street(
        &mut self,
        player: PlayerId,
        from: Point,
        to: Point,
    ) -> Result<(), Rejection> {
        if self.flag_at(from) != Some(player) {
            return Err(Rejection::NoFlagAtStart);
        }
        self.find_route(player, from, to)?;
        if self.flag_at(to).is_none() {
            if self.road_at(to).is_none() {
                return Err(Rejection::NoFlagAtTarget);
            }
            self.place_flag(player, to)?;
        }
        self.build_road(player, from, to)
    }

    /// `menu.createShip`: a finished ship on `at`, loaded with an
    /// expedition's builder, 4 wood and 6 stone (CAM-1.3, Q-187).
    pub(crate) fn create_ship(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        if !self.terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        let ship = self.debug_add_ship(player, at);
        let v = &mut self.ships[ship].vessel;
        v.aboard = SHIP_LOAD.to_vec();
        v.expedition = true;
        for (good, n) in SHIP_LOAD {
            self.made(Some(good), n);
        }
        Ok(())
    }

    /// `triggers.removeTrigger`
    pub(crate) fn remove_trigger(&mut self, name: &str) -> Result<(), Rejection> {
        let mission = self.mission.as_mut().ok_or(Rejection::NoSuchTrigger)?;
        let i = mission
            .triggers
            .iter()
            .position(|t| t.name == name)
            .ok_or(Rejection::NoSuchTrigger)?;
        mission.triggers.remove(i);
        Ok(())
    }

    /// CAM-3.7
    pub(crate) fn cheat(&mut self, player: PlayerId, cheat: Cheat) -> Result<(), Rejection> {
        match cheat {
            Cheat::INeedSomeHelp => {
                let mut mission = self.mission.take().unwrap_or_default();
                let value = mission.target;
                self.mark_completed(&mut mission, value);
                self.mission = Some(mission);
                Ok(())
            }
            Cheat::ShowMeTheWorld => {
                let bit = crate::territory::player_bit(player);
                for e in &mut self.explored {
                    *e |= bit;
                }
                Ok(())
            }
            Cheat::GiveMeSomeMore => {
                let at = self.headquarters_of(player).ok_or(Rejection::NoBuilding)?;
                let goods: Vec<u32> = self.goods.keys().copied().collect();
                let w = self.warehouses.get_mut(&at).expect("a warehouse");
                for &good in &goods {
                    w.add(good, 10);
                }
                for good in goods {
                    self.made(Some(good), 10);
                }
                Ok(())
            }
        }
    }
}

/// CAM-1.3 `createShip`: the load, by good id (builder, wood, stone).
const SHIP_LOAD: [(u32, u32); 3] = [(0xdd2d_aebe, 1), (0xcd6e_c133, 4), (0x55e9_52d3, 6)];
