//! Settler requests, ware orders and warehouse dispatch
//! (`specs/ECO-requests-dispatch.md`).

use crate::grid::{self, Point};
use crate::transport::{CarrierTask, Search, WareLocation};
use crate::{PlayerId, Settler, SettlerJob, World};

/// ECO-REQ-2.2 (T39): requests tried per tick over the one queue.
const REQUESTS_PER_TICK: usize = 4;
/// ECO-PROF-3.2: a spawning warehouse's stock of helpers is kept at this.
const HELPER_TARGET: u32 = 100;
/// ECO-REQ-3.6, 3.7: ticks between two sends of one warehouse.
pub(crate) const SEND_COOLDOWN: u32 = 30;
/// ECO-REQ-1.4, ECO-FOOD-2.2: the building timer fires when it has reached
/// this value, then starts again at 0: once every 101 ticks.
const TIMER_TOP: u32 = 100;
/// ECO-REQ-5.1: order matching and storage of unclaimed wares run when the
/// tick number is a multiple of these.
const MATCHING_PERIOD: u64 = 11;
const STORAGE_PERIOD: u64 = 5;
/// ECO-DIST-1.6, 1.7
const RECHECK_PERIOD: u64 = 3;
const TAKE_OUT_PERIOD: u64 = 7;
/// ECO-OUT-3.11: productivity falls by 0.05 (in hundredths) every 600 ticks.
const PRODUCTIVITY_PERIOD: u64 = 600;
const PRODUCTIVITY_DECAY: i32 = 5;
/// Player slots (ECO-REQ-5.2: players 0 to 7).
pub(crate) const PLAYERS: u8 = 8;
/// MAP-GRID-1.6: the flag of a building is its neighbour in this direction.
const FLAG_DIRECTION: u8 = 1;

/// A warehouse (headquarters, depot, harbour) and its stock (ECO-HQ-2,
/// ECO-REQ-3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Warehouse {
    /// (good, count) in the order the goods first arrived.
    pub stock: Vec<(u32, u32)>,
    /// ECO-REQ-3.7: ticks until it may send again.
    pub cooldown: u32,
    /// ECO-DIST-1.1: the stop and take-out flags of the goods a flag
    /// command has touched, by good id as a signed 32-bit number.
    pub flags: std::collections::BTreeMap<i32, crate::GoodFlags>,
}

impl Warehouse {
    /// ECO-DIST-1.2: neither flag is set for `good`.
    pub fn accepts(&self, good: u32) -> bool {
        self.flags
            .get(&(good as i32))
            .is_none_or(|f| !f.stop && !f.out)
    }

    /// The flags of `good` (both off without a record).
    pub fn flags_of(&self, good: u32) -> crate::GoodFlags {
        self.flags.get(&(good as i32)).copied().unwrap_or_default()
    }

    pub fn count(&self, good: u32) -> u32 {
        self.stock
            .iter()
            .filter(|&&(g, _)| g == good)
            .map(|&(_, n)| n)
            .sum()
    }

    pub(crate) fn add(&mut self, good: u32, count: u32) {
        match self.stock.iter_mut().find(|(g, _)| *g == good) {
            Some((_, n)) => *n += count,
            None => self.stock.push((good, count)),
        }
    }

    /// Set the stock of `good` to `count`.
    pub(crate) fn set(&mut self, good: u32, count: u32) {
        match self.stock.iter_mut().find(|(g, _)| *g == good) {
            Some((_, n)) => *n = count,
            None if count > 0 => self.stock.push((good, count)),
            None => {}
        }
    }

    /// Take one of `good`; false if there is none.
    pub(crate) fn take(&mut self, good: u32) -> bool {
        match self.stock.iter_mut().find(|(g, n)| *g == good && *n > 0) {
            Some((_, n)) => {
                *n -= 1;
                true
            }
            None => false,
        }
    }
}

/// A building, by field and serial: tells it from a later building on the
/// same field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub at: Point,
    pub serial: u32,
    /// A ware sent to a warehouse only to be stored (ECO-REQ-5.4, 5.5).
    pub storage: bool,
}

/// Who filed a settler request (ECO-REQ-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requester {
    /// A road, by id: its carrier or donkey.
    Road(usize),
    /// A construction site: its leveller or builder.
    Site(Target),
    /// A finished building: its worker.
    Building(Target),
    /// A military building: one soldier of any rank (MIL-SOL-4.3).
    Military(Target),
    /// A harbour: a settler for its stock (SHIP-4.3, 5.2: the builder of an
    /// expedition).
    Harbour(Target),
    /// ECO-SPC-1.1: a geologist or scout ordered to the flag `flag`; each
    /// order has its own number (ECO-SPC-1.2).
    Specialist { flag: Point, order: u32 },
    /// ECO-REQ-2.3a: a request whose link to its requester was cancelled
    /// (MIL-SOL-4.4 step 1). It stays in the queue and is deleted unserved
    /// when its turn comes.
    Unlinked,
}

/// A request for one settler of `profession` (ECO-REQ-1), of `player`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub player: PlayerId,
    pub requester: Requester,
    pub profession: u32,
    /// SHIP-4.1 (Q-063): the harbour it is attached to (it counts in that
    /// harbour's need of the profession).
    pub harbour: Option<Point>,
}

/// An order for one unit of `good` for the building `target` (ECO-REQ-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub player: PlayerId,
    pub good: u32,
    pub target: Target,
    /// ECO-DIST-3.3b, ECO-REQ-5.3d: the tick it was last chosen; 0 until
    /// then.
    pub stamp: u64,
    /// ECO-REQ-5.3f: its place in the order the orders were made (a served
    /// order keeps it).
    pub made: u64,
    /// SHIP-4.1 (Q-063): the harbour it is attached to (it counts in that
    /// harbour's need of the good) while it is open.
    pub harbour: Option<Point>,
}

/// A message to a player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub tick: u64,
    pub player: PlayerId,
    pub at: Point,
    pub text: &'static str,
    /// ECO-REQ-3.8: shown until this tick.
    pub until: u64,
    /// UI-GAME-11.1: the type of the building on the field when the
    /// message came (its name follows the text: "Building constructed
    /// (Woodcutter's Hut)").
    pub building: Option<u32>,
    /// UI-GAME-11.1, 4.5 c: the ship the message names (its index in the
    /// world's ship list), whose name follows the text ("Ship is awaiting
    /// orders (Aurora)", E-trace-030).
    pub ship: Option<u32>,
}

/// UI-GAME-11.2: a message's timer starts at 600 timer units and each tick
/// loses the last frame's time in seconds; the sim takes the fixed 1/60 s
/// a tick the spec allows, so one unit is 60 ticks.
pub(crate) const TIMER_START: u64 = 600;
pub(crate) const TICKS_PER_TIMER_UNIT: u64 = 60;
/// ECO-REQ-3.8, UI-GAME-11.2: 600 units, 36 000 ticks.
pub(crate) const MESSAGE_TICKS: u64 = TIMER_START * TICKS_PER_TIMER_UNIT;

/// UI-GAME-11.1: the messages not added again while one of the same text
/// is shown for the same field (the found messages start with "Found").
fn kept_single(text: &str) -> bool {
    text == "Lost Territory." || refreshed(text)
}

/// UI-GAME-11.1: the kept-single messages whose timer is reset to 600 when
/// they come again ("Lost Territory." is not).
fn refreshed(text: &str) -> bool {
    text.starts_with("Found ") || matches!(text, "No resources found." | "Building without worker.")
}

impl World {
    /// ECO-REQ-3.8, UI-GAME-11.1: a message; for the kinds kept single,
    /// not while the player has one with the same text for the same field
    /// shown still: that one's timer is reset to 600 units instead (not
    /// for "Lost Territory.", which stays as it is; T36b).
    pub(crate) fn message(&mut self, player: PlayerId, at: Point, text: &'static str) {
        let now = self.tick;
        let building = self.building_at(at).map(|b| b.building_type);
        let shown = self.messages.iter_mut().find(|m| {
            kept_single(text) && m.player == player && m.at == at && m.text == text && m.until > now
        });
        match shown {
            Some(message) => {
                if refreshed(text) {
                    message.until = now + MESSAGE_TICKS;
                }
            }
            None => self.messages.push(Message {
                tick: now,
                player,
                at,
                text,
                until: now + MESSAGE_TICKS,
                building,
                ship: None,
            }),
        }
    }

    /// UI-GAME-11.1, 4.5 c: a message naming ship `ship` (its name follows
    /// the text in the row).
    pub(crate) fn ship_message(
        &mut self,
        player: PlayerId,
        at: Point,
        text: &'static str,
        ship: usize,
    ) {
        let now = self.tick;
        self.messages.push(Message {
            tick: now,
            player,
            at,
            text,
            until: now + MESSAGE_TICKS,
            building: self.building_at(at).map(|b| b.building_type),
            ship: u32::try_from(ship).ok(),
        });
    }

    /// The building `target` names, if it still stands.
    pub(crate) fn target_building(&self, target: Target) -> Option<&crate::Building> {
        self.building_at(target.at)
            .filter(|b| b.serial == target.serial)
    }

    /// The flag of the building `target`, if it still stands.
    pub(crate) fn target_flag(&self, target: Target) -> Option<Point> {
        self.target_building(target)?;
        self.terrain.neighbour(target.at, FLAG_DIRECTION)
    }

    /// The target of the building on `at`.
    pub(crate) fn target_of(&self, at: Point) -> Option<Target> {
        self.building_at(at).map(|b| Target {
            at,
            serial: b.serial,
            storage: false,
        })
    }

    /// A ware arrives in the building `target` (TRN-CARRY-3.3), served by
    /// the order `served` filed with it.
    pub(crate) fn hand_over(&mut self, target: Target, good: u32, served: Option<Order>) {
        if self.target_building(target).is_none() {
            // Q-230: the building went while the carrier walked in; the
            // ware is gone (not specified; ECO-REQ-5.5 sends on only wares
            // lying at a flag).
            self.used(good, 1);
            return;
        }
        // W16: a member delivered for a recipe group's order counts as the
        // group in the building's stock.
        let good = match served {
            Some(o)
                if o.good != good
                    && self
                        .recipe_groups
                        .get(&o.good)
                        .is_some_and(|m| m.contains(&good)) =>
            {
                self.used(good, 1);
                self.made(Some(o.good), 1);
                o.good
            }
            _ => good,
        };
        // ECO-REQ-5.3f (T20): the served order is open again, last among
        // the open orders with the stamp it was chosen with; then the
        // building drops its open order of the good that was made first
        // (the served one when it is the oldest).
        if let Some(order) = served.filter(|o| o.target.at == target.at && o.good == good) {
            // SHIP-4.1: open again, attached to no harbour.
            self.orders.push(Order {
                harbour: None,
                ..order
            });
            let first = self
                .orders
                .iter()
                .enumerate()
                .filter(|(_, o)| o.target.at == target.at && o.good == good)
                .min_by_key(|(_, o)| o.made)
                .map(|(i, _)| i);
            if let Some(i) = first {
                let dropped = self.orders.remove(i);
                self.detach_orders(&[dropped]);
            }
        }
        // MIL-UPG-2.3: an upgrading building takes what its construction
        // still needs for the site and everything else (coins) as usual.
        if let Some(site) = self
            .sites
            .get_mut(&target.at)
            .filter(|s| !self.upgrades.contains_key(&target.at) || s.still_needed(good) > 0)
        {
            site.receive(good);
        } else if let Some(warehouse) = self.warehouses.get_mut(&target.at) {
            warehouse.add(good, 1);
        } else {
            self.receive_input(target.at, good);
        }
    }

    /// W16: the member of the recipe group `group` the warehouse on `at`
    /// hands out to its order: the first in good order it holds of which
    /// the player has more than it keeps back across its warehouses.
    pub(crate) fn group_member_at(&self, player: PlayerId, at: Point, group: u32) -> Option<u32> {
        let members = self.recipe_groups.get(&group)?;
        let warehouse = self.warehouses.get(&at)?;
        members.iter().copied().find(|&m| {
            warehouse.count(m) > 0 && {
                let total: u32 = self
                    .warehouses_of(player)
                    .iter()
                    .filter_map(|w| self.warehouses.get(w))
                    .map(|w| w.count(m))
                    .sum();
                total > self.recipe_keep.get(&m).copied().unwrap_or(0)
            }
        })
    }

    /// The player's finished warehouses in order of creation.
    pub(crate) fn warehouses_of(&self, player: PlayerId) -> Vec<Point> {
        let mut found: Vec<(u32, Point)> = self
            .warehouses
            .keys()
            .filter_map(|&at| {
                let b = self.building_at(at)?;
                (b.owner == player && !b.under_construction).then_some((b.serial, at))
            })
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, at)| at).collect()
    }

    pub(crate) fn flag_of(&self, at: Point) -> Point {
        self.terrain
            .neighbour(at, FLAG_DIRECTION)
            .expect("a building has a flag")
    }

    /// ECO-REQ-3.2: among `candidates` (in order of creation), the warehouse
    /// connected to `destination` with the lowest route cost from its flag;
    /// ties go to the first.
    pub(crate) fn nearest_warehouse(
        &self,
        player: PlayerId,
        candidates: impl IntoIterator<Item = Point>,
        destination: Point,
        search: Search,
    ) -> Option<(Point, u32)> {
        let mut best: Option<(Point, u32)> = None;
        // TRN-FREE-6.1: a settler walks land roads only, so with waterways
        // in the network a warehouse must also be reachable over land.
        let waterways = self
            .roads
            .iter()
            .flatten()
            .any(|r| r.owner == player && r.water);
        let costs = self.route_costs_to(player, destination, search);
        for at in candidates {
            let Some(cost) = costs.from(self.flag_of(at)) else {
                continue;
            };
            if waterways
                && self
                    .road_walk(player, self.flag_of(at), destination)
                    .is_none()
            {
                continue;
            }
            if best.is_none_or(|(_, c)| cost < c) {
                best = Some((at, cost));
            }
        }
        best
    }

    /// ECO-REQ-5.3: H, the warehouse in D's ware network (`network`, the
    /// costs towards D's flag `destination`) with `good` in stock and the
    /// lowest stored ware route cost from D's flag to its flag
    /// (TRN-CARRY-5.3a), the first in the player's list on a tie; with
    /// waterways it must also be reachable over land (as
    /// [`World::nearest_warehouse`]).
    pub(crate) fn stocked_warehouse(
        &mut self,
        player: PlayerId,
        candidates: Vec<Point>,
        good: u32,
        destination: Point,
        network: &crate::transport::CostsTo,
    ) -> Option<(Point, u32)> {
        let waterways = self
            .roads
            .iter()
            .flatten()
            .any(|r| r.owner == player && r.water);
        let mut best: Option<(Point, u32)> = None;
        for at in candidates {
            let warehouse_flag = self.flag_of(at);
            // W16: a recipe group's order is served by a member the
            // warehouse holds above the player's kept stock.
            let has = if self.recipe_groups.contains_key(&good) {
                self.group_member_at(player, at, good).is_some()
            } else {
                self.spare_for_others(at, good) > 0
            };
            if !has || network.from(warehouse_flag).is_none() {
                continue;
            }
            let Some(cost) = self.stored_ware_cost(player, destination, warehouse_flag) else {
                continue;
            };
            if waterways
                && self
                    .road_walk(player, warehouse_flag, destination)
                    .is_none()
            {
                continue;
            }
            if best.is_none_or(|(_, c)| cost < c) {
                best = Some((at, cost));
            }
        }
        best
    }

    // ---- The per-tick parts (ECO-REQ-0.1) ----

    /// One tick of the buildings of all players in one pass, then the one
    /// request queue (ECO-REQ-0.1, T39).
    pub(crate) fn update_players(&mut self) {
        // The set of buildings does not change while they are updated (only
        // commands add or remove them, and removals are skipped below). The
        // game's one building list is in creation order: by object number.
        let mut buildings: Vec<(u32, Point)> = self
            .building_index
            .iter()
            .filter_map(|&index| self.buildings[index as usize].as_ref())
            .map(|b| (b.serial, b.at))
            .collect();
        buildings.sort_unstable();
        self.update_buildings(buildings.into_iter().map(|(_, at)| at).collect());
        self.process_requests();
    }

    fn update_buildings(&mut self, fields: Vec<Point>) {
        for at in fields {
            // An earlier building's update may have removed this one.
            let Some(owner) = self.building_at(at).map(|b| b.owner) else {
                continue;
            };
            // MIL-UPG-2.4 (T8): an upgrade whose construction is done
            // changes the type once no builder is linked to it.
            if !self.sites.contains_key(&at) && !self.builder_links.contains_key(&at) {
                if let Some(to) = self.upgrades.remove(&at) {
                    self.change_building_type(at, to);
                }
            }
            if self.sites.contains_key(&at) {
                self.update_site(at);
                // MIL-UPG-3.1: an upgrading building goes on as it was.
                if self.upgrades.contains_key(&at) && self.building_at(at).is_some() {
                    self.update_finished(at);
                }
            } else {
                self.update_finished(at);
                // SHIP-4.3 (T28): a harbour's own update follows.
                self.update_harbour_at(at);
            }
            // TER-6.2: right after its own update a building not on its
            // owner's inner land is removed.
            if self.building_at(at).is_some() && !self.is_inner_land(owner, at) {
                // TRN-FREE-6.4a (T49): before the settlers' part, so its
                // workers go home from this tick.
                self.remove_with_remains_as(at, true);
            }
        }
    }

    /// A finished building: the warehouse cooldown (ECO-REQ-3.7) and the
    /// 101-tick timer (ECO-REQ-1.4).
    fn update_finished(&mut self, at: Point) {
        if let Some(warehouse) = self.warehouses.get_mut(&at) {
            warehouse.cooldown = warehouse.cooldown.saturating_sub(1);
        }
        // ECO-FOOD-5.1, ECO-OUT-2.1: the 51-tick search counter.
        self.update_search(at);
        // MIL-SOL, MIL-PROM, TER-2.1
        self.update_military_building(at);
        // MIL-CON-5.2
        self.update_catapult(at);
        let index = self.terrain.index(at);
        let building = self.buildings[index].as_mut().expect("a building");
        // ECO-OUT-3.11: every 600 ticks productivity falls by 0.05; the count
        // is the tick plus a loaded original save's phase (FMT-SAV-3.11).
        if (self.tick + self.productivity_phase).is_multiple_of(PRODUCTIVITY_PERIOD)
            && building.productivity >= 0
        {
            building.productivity = (building.productivity - PRODUCTIVITY_DECAY).max(0);
            // AI-MIL T24: the float32 value, the same fall in float32.
            let value = f32::from_bits(building.productivity_bits);
            building.productivity_bits = (value - crate::production::STAT_FALL).max(0.0).to_bits();
        }
        if building.timer < TIMER_TOP {
            building.timer += 1;
            return;
        }
        building.timer = 0;
        self.request_worker(at);
        self.timer_fired(at);
        self.spawn_helper(at);
    }

    /// ECO-PROF-3.1, 3.2: the 101-tick check of a spawning warehouse (the
    /// headquarters, a depot, a harbour): below 100 helpers in its stock one
    /// is added, above 100 the stock is cut to 100.
    fn spawn_helper(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let spawns = self.building_types[&building.building_type].spawn;
        let Some(helper) = self.roles.helper.filter(|_| spawns) else {
            return;
        };
        let Some(warehouse) = self.warehouses.get_mut(&at) else {
            return;
        };
        let have = warehouse.count(helper);
        if have != HELPER_TARGET {
            warehouse.set(
                helper,
                if have < HELPER_TARGET {
                    have + 1
                } else {
                    HELPER_TARGET
                },
            );
        }
    }

    /// ECO-REQ-1.4
    fn request_worker(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let Some(worker) = kind.worker else { return };
        if building.workers == kind.worker_limit {
            return;
        }
        let requester = Requester::Building(self.target_of(at).expect("a building"));
        self.request(building.owner, requester, worker);
    }

    /// File a request unless the requester has one open for `profession`.
    pub(crate) fn request(&mut self, player: PlayerId, requester: Requester, profession: u32) {
        let open = self
            .queue
            .iter()
            .any(|r| r.player == player && r.requester == requester && r.profession == profession);
        if !open {
            self.queue.push(Request {
                player,
                requester,
                profession,
                harbour: None,
            });
            // SIM-START-4.2: a request filed takes a number.
            self.new_number();
        }
    }

    /// SHIP-4.3 (T31): file a request even when the requester has one open
    /// for `profession` (one per missing settler).
    pub(crate) fn file_request(&mut self, player: PlayerId, requester: Requester, profession: u32) {
        self.queue.push(Request {
            player,
            requester,
            profession,
            harbour: None,
        });
        // SIM-START-4.2: a request filed takes a number.
        self.new_number();
    }

    /// SHIP-4.3 (T31): the open requests of `requester` whose profession
    /// `job` accepts.
    pub(crate) fn open_requests_of(
        &self,
        player: PlayerId,
        requester: Requester,
        job: impl Fn(u32) -> bool,
    ) -> u32 {
        self.queue
            .iter()
            .filter(|r| r.player == player && r.requester == requester && job(r.profession))
            .count() as u32
    }

    /// SHIP-5.2a: the first open request of `requester` for `profession`
    /// (in the order filed) is cancelled: it leaves the queue (and its
    /// harbour's need), the queue's turn staying on the request it was on.
    pub(crate) fn cancel_first_request(
        &mut self,
        player: PlayerId,
        requester: Requester,
        profession: u32,
    ) {
        let Some(index) = self.queue.iter().position(|r| {
            r.player == player && r.requester == requester && r.profession == profession
        }) else {
            return;
        };
        let request = self.queue.remove(index);
        if index < self.queue_cursor {
            self.queue_cursor -= 1;
        }
        self.detach_request(&request);
    }

    /// ECO-REQ-2.3a (T38): the open requests of a requester that is gone
    /// lose their link where they stand in the queue, which deletes them
    /// unserved on their turn.
    pub(crate) fn unlink_requests(&mut self, player: PlayerId, requester: Requester) {
        let mut detached = Vec::new();
        for request in self
            .queue
            .iter_mut()
            .filter(|r| r.player == player && r.requester == requester)
        {
            request.requester = Requester::Unlinked;
            // SHIP-4.1: a request whose requester is gone leaves its
            // harbour's need at once.
            if let Some(h) = request.harbour.take() {
                detached.push((h, request.profession));
            }
        }
        for (h, profession) in detached {
            self.detach_need(h, profession);
        }
    }

    pub(crate) fn has_open_request(&self, player: PlayerId, requester: Requester) -> bool {
        self.queue
            .iter()
            .any(|r| r.player == player && r.requester == requester)
    }

    /// ECO-REQ-1.1: every road without a carrier, or busy without a donkey,
    /// files a request; and the road's traffic clock (TRN-CARRY-6).
    pub(crate) fn update_roads(&mut self) {
        self.remove_lost_flags();
        self.update_traffic();
        // W19: a project's pavements under way send their material.
        self.update_pavings();
        for id in 0..self.roads.len() {
            let Some(road) = self.roads[id].as_ref() else {
                continue;
            };
            let player = road.owner;
            let requester = Requester::Road(id);
            if self.has_open_request(player, requester) {
                continue;
            }
            // ECO-REQ-1.1: a waterway asks for a boat carrier and never for a
            // donkey (TRN-CARRY-6.2).
            let profession = if road.water {
                road.carrier
                    .is_none()
                    .then_some(self.roles.boat_carrier)
                    .flatten()
            } else if road.carrier.is_none() {
                self.roles.helper
            } else if road.traffic.donkey_requested && road.donkey.is_none() {
                self.roles.donkey
            } else {
                None
            };
            if let Some(profession) = profession {
                self.request(player, requester, profession);
            }
        }
    }

    // ---- The queue (ECO-REQ-2) ----

    /// ECO-REQ-2.2 to 2.5: the one queue of all players (T39), 4 tries a
    /// tick whatever players the requests belong to.
    fn process_requests(&mut self) {
        let tries = self.queue.len().min(REQUESTS_PER_TICK);
        let mut last_failed: Option<Request> = None;
        for _ in 0..tries {
            if self.queue.is_empty() {
                break;
            }
            if self.queue_cursor >= self.queue.len() {
                self.queue_cursor = 0;
            }
            let index = self.queue_cursor;
            let request = self.queue[index];
            // 2.3
            if !self.request_wanted(&request) {
                self.queue.remove(index);
                self.detach_request(&request);
                continue;
            }
            // 2.5: the try before it, of any player.
            if last_failed.is_some_and(|failed| self.same_request_key(&failed, &request)) {
                self.queue_cursor += 1;
                continue;
            }
            // 2.4
            if self.serve(request.player, request, index) {
                let served = self.queue.remove(index);
                self.detach_request(&served);
                // T36: a success does not clear the failed key.
                continue;
            }
            last_failed = Some(request);
            self.queue_cursor += 1;
            // 3.8
            if let Requester::Building(target) = request.requester {
                self.message(request.player, target.at, "Building without worker.");
            }
        }
    }

    /// ECO-REQ-2.5 (T36): whether `request` has the key of the request that
    /// failed before it in the tick: the same profession and its requester's
    /// flag in the same network, the land road network (the boat network for
    /// a boat carrier); a military building's request by the building.
    fn same_request_key(&self, failed: &Request, request: &Request) -> bool {
        if failed.profession != request.profession {
            return false;
        }
        match (failed.requester, request.requester) {
            (Requester::Military(_), _) | (_, Requester::Military(_)) => {
                failed.requester == request.requester
            }
            (Requester::Unlinked, _) | (_, Requester::Unlinked) => false,
            (a, b) => {
                let kind = if Some(request.profession) == self.roles.boat_carrier {
                    crate::roads::Network::Boat
                } else {
                    crate::roads::Network::Carrier
                };
                self.same_network(
                    self.request_destination(a),
                    self.request_destination(b),
                    kind,
                )
            }
        }
    }

    /// ECO-REQ-2.3: whether the requester still exists and still needs the
    /// settler. A request whose need was met otherwise (a debug supply) is
    /// dropped as well.
    fn request_wanted(&self, request: &Request) -> bool {
        match request.requester {
            Requester::Road(id) => self
                .roads
                .get(id)
                .and_then(Option::as_ref)
                .is_some_and(|r| {
                    if Some(request.profession) == self.roles.helper {
                        r.carrier.is_none()
                    } else {
                        r.donkey.is_none()
                    }
                }),
            Requester::Site(target) => {
                self.target_building(target).is_some()
                    && self.sites.get(&target.at).is_some_and(|site| {
                        if Some(request.profession) == self.roles.leveller {
                            !site.levelled() && site.leveller.is_none() && !site.leveller_on_way
                        } else {
                            site.builder.is_none() && !site.builder_on_way
                        }
                    })
            }
            Requester::Building(target) => self.target_building(target).is_some_and(|b| {
                let limit = self.building_types[&b.building_type].worker_limit;
                !b.under_construction && b.workers < limit
            }),
            Requester::Military(_) => self.soldier_request_wanted(request),
            Requester::Harbour(target) => self.target_building(target).is_some(),
            // ECO-SPC-1.1: while the flag stands (Q-184 h).
            Requester::Specialist { flag, .. } => self.flag_at(flag).is_some(),
            // 2.3a
            Requester::Unlinked => false,
        }
    }

    /// ECO-REQ-3.1: where the settler goes. For a road, its first flag.
    fn request_destination(&self, requester: Requester) -> Point {
        match requester {
            Requester::Road(id) => self.roads[id].as_ref().expect("live road").path[0],
            Requester::Site(target)
            | Requester::Building(target)
            | Requester::Military(target)
            | Requester::Harbour(target) => self.flag_of(target.at),
            Requester::Specialist { flag, .. } => flag,
            Requester::Unlinked => unreachable!("deleted on its turn (ECO-REQ-2.3a)"),
        }
    }

    /// SHIP-4.1 (Q-063): a settler request leaving the queue, or served
    /// over land, leaves its harbour's need.
    fn detach_request(&mut self, request: &Request) {
        if let Some(h) = request.harbour {
            self.detach_need(h, request.profession);
        }
    }

    /// SHIP-4.1 (Q-063): the settler request at `index` of the queue found
    /// no warehouse able to send the settler: it is attached as a ware
    /// order is ([`World::harbour_for`] from the requester's flag), if it
    /// is not attached yet. SHIP-4.1 (T27, Q-1079): "reachable over the
    /// roads" is the ware network (the roads with a carrier or donkey
    /// assigned) and its cost, for a settler request too.
    fn settler_harbour_step(&mut self, index: usize) {
        let request = self.queue[index];
        if request.harbour.is_some() || !self.has_harbour(request.player) {
            return;
        }
        let flag = self.request_destination(request.requester);
        let costs = self.route_costs_to(request.player, flag, Search::Ware);
        if let Some(h) = self.harbour_for(request.player, request.profession, &costs) {
            self.queue[index].harbour = Some(h);
            self.attach_need(h, request.profession);
        }
    }

    /// ECO-REQ-3: serve a request if a warehouse can; true if served. The
    /// request is the queue's entry `index`.
    fn serve(&mut self, player: PlayerId, request: Request, index: usize) -> bool {
        // MIL-SOL-4.7: a warehouse never makes a soldier for a request.
        if let Requester::Military(target) = request.requester {
            return self.serve_soldier(player, target);
        }
        let profession = request.profession;
        let destination = self.request_destination(request.requester);
        let warehouses = self.warehouses_of(player);
        let costs = self
            .goods
            .get(&profession)
            .map(|g| g.costs.clone())
            .unwrap_or_default();
        // 3.2: A has one in stock.
        let a = self.nearest_warehouse(
            player,
            warehouses
                .iter()
                .copied()
                .filter(|at| self.warehouses[at].count(profession) > 0),
            destination,
            Search::Settler,
        );
        // 3.3: B can make one.
        let can_make = |at: &Point| {
            !costs.is_empty()
                && costs
                    .iter()
                    .all(|&(good, n)| self.warehouses[at].count(good) >= n)
        };
        let b = self.nearest_warehouse(
            player,
            warehouses.iter().copied().filter(can_make),
            destination,
            Search::Settler,
        );
        // 3.4 (T37): B only if strictly nearer by field distance, from the
        // warehouses' own fields (not their flags) to the destination.
        let chosen = match (a, b) {
            (Some((a, _)), Some((b, _))) if a != b => {
                let distance = |at: Point| grid::distance(at, destination);
                if distance(b) < distance(a) {
                    b
                } else {
                    a
                }
            }
            (Some((a, _)), _) => a,
            (None, Some((b, _))) => b,
            (None, None) => {
                self.settler_harbour_step(index);
                return false;
            }
        };
        // 3.5
        let warehouse = &self.warehouses[&chosen];
        let in_stock = warehouse.count(profession) > 0;
        let sendable = warehouse.cooldown == 0 && (in_stock || can_make(&chosen));
        // SHIP-4.1 (Q-063): a request that finds a warehouse over land is
        // detached before it is served.
        if let Some(h) = self.queue[index].harbour.take() {
            self.detach_need(h, profession);
        }
        if !sendable {
            return false;
        }
        // 3.6
        let warehouse = self.warehouses.get_mut(&chosen).expect("a warehouse");
        let mut taken = Vec::new();
        if !in_stock {
            for &(good, n) in &costs {
                for _ in 0..n {
                    if warehouse.take(good) {
                        taken.push(good);
                    }
                }
            }
            warehouse.add(profession, 1);
        }
        warehouse.take(profession);
        warehouse.cooldown = SEND_COOLDOWN;
        // ECO-REQ-3.3: the tool is used up in the new profession.
        for good in taken {
            self.used(good, 1);
        }
        self.send_settler(player, chosen, request, destination);
        true
    }

    /// ECO-REQ-3.6: the settler appears at the warehouse's door (its door
    /// field, ECO-WORK-3.8a), walks out to its flag and over the roads to
    /// its requester (TRN-CARRY-1.4), a road walk to the requester's flag
    /// (TRN-FREE-6.5).
    fn send_settler(&mut self, player: PlayerId, from: Point, request: Request, flag: Point) {
        // SIM-START-4.2: the settler sent is made now and takes its number.
        let number = self.new_number();
        let out = self.flag_of(from);
        let mut walk = vec![out];
        walk.extend(
            self.road_walk(player, out, flag)
                .expect("a warehouse of the destination's network"),
        );
        match request.requester {
            Requester::Road(id) => {
                // TRN-CARRY-1.3, 1.4: the first of its road's flags it
                // reaches, the road's last flag too.
                let path = &self.roads[id].as_ref().expect("live road").path;
                let ends = [path[0], path[path.len() - 1]];
                let arrival = walk
                    .iter()
                    .position(|f| ends.contains(f))
                    .expect("the way ends at the road's first flag");
                let flag = walk[arrival];
                // ECO-WORK-3.8d, TRN-CARRY-1.4: out to the warehouse's flag,
                // where it stands one tick whether or not that flag is on its
                // road; its walk on is ordered there (`CarrierTask::Arrive`).
                let walker = self.walker_leaving(from, vec![out]);
                // A waterway's boat carrier takes the carrier's place.
                let donkey = Some(request.profession) != self.roles.helper
                    && Some(request.profession) != self.roles.boat_carrier;
                // SIM-START-4.3 b: a road getting its carrier joins the
                // donkey networks of its two ends.
                if !donkey {
                    self.renumber_carried(id);
                }
                let carrier = self.place_carrier(
                    id,
                    donkey,
                    walker,
                    CarrierTask::Arrive { flag, stand: true },
                    number,
                );
                self.carriers[carrier].fresh = true;
            }
            Requester::Site(target) => {
                let builder = Some(request.profession) != self.roles.leveller;
                // The walk ends on the site's flag (ECO-WORK-3.8c; the
                // leveller's ECO-HQ-4.3a): the builder's approach starts
                // there after a standing tick.
                let mut walker = self.walker_leaving(from, walk);
                walker.road_to = Some(flag);
                let site = self.sites.get_mut(&target.at).expect("a site");
                if builder {
                    site.builder_on_way = true;
                } else {
                    site.leveller_on_way = true;
                }
                self.add_settler(Settler {
                    number,
                    player,
                    profession: request.profession,
                    list: if builder {
                        crate::SettlerList::Constructors
                    } else {
                        crate::SettlerList::Levellers
                    },
                    walker,
                    job: SettlerJob::ToSite { target, builder },
                    timer: 0,
                    carrying: None,
                });
            }
            Requester::Military(_) => unreachable!("served by serve_soldier"),
            Requester::Unlinked => unreachable!("deleted on its turn (ECO-REQ-2.3a)"),
            Requester::Harbour(target) if Some(request.profession) == self.roles.builder => {
                // SHIP-5.2 (T23): the expedition's builder leaves as a
                // builder going to his site, out of the door to the serving
                // warehouse's flag only; there his walk to the harbour (no
                // site) is refused and he goes home, his home the harbour.
                let walker = self.walker_leaving(from, vec![out]);
                self.add_settler(Settler {
                    number,
                    player,
                    profession: request.profession,
                    list: crate::SettlerList::Constructors,
                    walker,
                    job: SettlerJob::ToSite {
                        target,
                        builder: true,
                    },
                    timer: 0,
                    carrying: None,
                });
            }
            Requester::Harbour(target) => {
                // It walks into the harbour and joins its stock.
                walk.push(target.at);
                let mut walker = self.walker_leaving(from, walk);
                walker.road_to = Some(flag);
                self.add_settler(Settler {
                    number,
                    player,
                    profession: request.profession,
                    list: self.list_of(request.profession),
                    walker,
                    job: SettlerJob::Home { target },
                    timer: 0,
                    carrying: None,
                });
            }
            Requester::Specialist { flag, .. } => {
                // ECO-SPC-2.1: over the roads to the flag.
                let mut walker = self.walker_leaving(from, walk);
                walker.road_to = Some(flag);
                self.add_settler(Settler {
                    number,
                    player,
                    profession: request.profession,
                    list: crate::SettlerList::Specialists,
                    walker,
                    job: SettlerJob::Specialist {
                        flag,
                        spots_left: crate::specialists::SPOTS_PER_ORDER,
                        step: crate::SpecialistStep::ToFlag,
                    },
                    timer: 0,
                    carrying: None,
                });
            }
            Requester::Building(target) => {
                walk.push(target.at);
                // ECO-WORK-3.8 (T-door-4): a door field other than the
                // building's own is reached by a step of its own from the
                // building's field, from progress 0.
                let door = self.door_field(target.at);
                if door != target.at {
                    walk.push(door);
                }
                let mut walker = self.walker_leaving(from, walk);
                walker.road_to = Some(flag);
                // ECO-WORK-3.8c (T-door-3): the road walk ends exactly on
                // the building's flag and the walk to its field starts there
                // at progress 0 in the arrival tick.
                walker.pauses.push(flag);
                if door != target.at {
                    walker.pauses.push(target.at);
                }
                let index = self.terrain.index(target.at);
                self.buildings[index].as_mut().expect("a building").workers += 1;
                self.add_settler(Settler {
                    number,
                    player,
                    profession: request.profession,
                    list: crate::SettlerList::Workers,
                    walker,
                    job: SettlerJob::ToBuilding { target },
                    timer: 0,
                    carrying: None,
                });
            }
        }
    }

    // ---- Ware orders (ECO-REQ-4) and destinations (ECO-REQ-5) ----

    /// File one order of `good` for the building on `at`.
    pub(crate) fn order(&mut self, at: Point, good: u32) {
        let building = self.building_at(at).expect("a building");
        let player = building.owner;
        let target = self.target_of(at).expect("a building");
        // ECO-DIST-3.3b: a new order's stamp is 0, so it counts as having
        // waited longer than every order chosen before.
        let made = self.orders_made;
        self.orders_made += 1;
        // ECO-DIST-3.1a (T30): the good's group gets its place in the
        // player's order list at its first order, and keeps it.
        let groups = self.order_groups.entry(player).or_default();
        if !groups.contains(&good) {
            groups.push(good);
        }
        self.orders.push(Order {
            player,
            good,
            target,
            stamp: 0,
            made,
            harbour: None,
        });
        // SIM-START-4.4 d: material files no request and takes no number;
        // its helper and ware do when it leaves the warehouse (f).
    }

    /// Drop up to `count` open orders of `good` for the building on `at`.
    pub(crate) fn cancel_orders(&mut self, at: Point, good: u32, count: u32) {
        let mut left = count;
        let mut removed = Vec::new();
        self.orders.retain(|o| {
            if left > 0 && o.target.at == at && o.good == good {
                left -= 1;
                removed.push(*o);
                false
            } else {
                true
            }
        });
        self.detach_orders(&removed);
    }

    /// ECO-REQ-5.1
    pub(crate) fn transport_jobs(&mut self) {
        // ECO-REQ-5.5 (T42, E-eco-049): first, in every tick, a ware whose
        // building is gone is free.
        self.free_wares_of_gone_buildings();
        // ECO-REQ-5.1: in this order when several are due.
        if self.tick.is_multiple_of(MATCHING_PERIOD) {
            self.drop_stranded_orders();
            self.match_orders();
        }
        if self.tick.is_multiple_of(RECHECK_PERIOD) {
            self.recheck_wares();
        }
        if self.tick.is_multiple_of(STORAGE_PERIOD) {
            self.store_wares();
        }
        if self.tick.is_multiple_of(TAKE_OUT_PERIOD) {
            self.take_out();
        }
    }

    /// ECO-REQ-5.5 (T42, E-eco-049, as corrected by the investigation's
    /// f0f1ecb7): at the start of every tick's transport step, before the
    /// matching and any storage run, a ware lying at a flag for a building
    /// (not a storage trip) whose flag is gone loses its destination: free,
    /// on the free-ware list (5.4a), so the next matching can give it to
    /// another order; a carrier's claim on it stays.
    fn free_wares_of_gone_buildings(&mut self) {
        let ids: Vec<usize> = self
            .wares()
            .filter(|(_, w)| {
                matches!(w.location, WareLocation::Flag(_))
                    && w.target
                        .is_some_and(|t| !t.storage && self.target_flag(t).is_none())
            })
            .map(|(id, _)| id)
            .collect();
        for id in ids {
            self.join_free_list(id);
            let w = self.wares[id].as_mut().expect("live ware");
            w.destination = None;
            w.target = None;
            w.order = None;
            w.stranded = false;
            w.next = None;
            w.recheck = 0;
        }
    }

    /// ECO-REQ-5.7 (a): before the matching, a ware lying at a flag for an
    /// order that found no next flag loses the order: it lies there free,
    /// without destination, and its building orders it again. (b) has no
    /// case here: a ware reaching its building's flag is handed in by its
    /// carrier, and a free ware at that flag is never chosen (5.3 a).
    fn drop_stranded_orders(&mut self) {
        let ids: Vec<usize> = self
            .wares()
            .filter(|(_, w)| {
                matches!(w.location, WareLocation::Flag(_))
                    && w.stranded
                    && w.target.is_some_and(|t| !t.storage)
            })
            .map(|(id, _)| id)
            .collect();
        // ECO-REQ-5.4a (T31, T32): the lost wares are collected in the order
        // of their orders and dropped last collected first: they join the
        // free-ware lists in that order (player by player, the last made
        // order's ware first).
        let mut joining: Vec<(PlayerId, u64, usize)> = ids
            .iter()
            .map(|&id| {
                let w = self.wares[id].as_ref().expect("live ware");
                let (player, made) = w.order.map_or((PlayerId(0), 0), |o| (o.player, o.made));
                (player, made, id)
            })
            .collect();
        joining.sort_by_key(|&(player, made, id)| (player, std::cmp::Reverse((made, id))));
        for &(_, _, id) in &joining {
            self.join_free_list(id);
        }
        for id in ids {
            let w = self.wares[id].as_mut().expect("live ware");
            let target = w.target.take().expect("an order's ware");
            w.order = None;
            w.destination = None;
            w.stranded = false;
            w.next = None;
            w.recheck = 0;
            let good = w.good;
            if self.target_building(target).is_some() {
                self.order(target.at, good);
            }
        }
    }

    /// ECO-REQ-5.2, ECO-DIST-3: player by player; within a player by good
    /// in transport rank order, equal ranks in the order of their groups'
    /// places (ECO-DIST-3.1a, T30); each good once per road network,
    /// serving one order.
    fn match_orders(&mut self) {
        let mut served = vec![false; self.orders.len()];
        for p in 0..PLAYERS {
            let player = PlayerId(p);
            let mut open_goods: Vec<u32> = self
                .orders
                .iter()
                .filter(|o| o.player == player)
                .map(|o| o.good)
                .collect();
            open_goods.sort_unstable();
            open_goods.dedup();
            // ECO-DIST-3.1a (T30): the goods in their groups' places (a good
            // without one, from an older snapshot, gets one now, by id),
            // then a stable sort by rank.
            let groups = self.order_groups.entry(player).or_default();
            for &g in &open_goods {
                if !groups.contains(&g) {
                    groups.push(g);
                }
            }
            let mut goods: Vec<u32> = groups
                .iter()
                .copied()
                .filter(|g| open_goods.binary_search(g).is_ok())
                .collect();
            goods.sort_by_key(|&g| self.rank(player, g));
            // ECO-DIST-3.1, 3.3a (T23): list 0, the orders of finished
            // warehouse-type buildings, before list 1, all others; each good
            // by good (all goods' list 0 first, as measured).
            let lists = [true, false]
                .into_iter()
                .flat_map(|zero| goods.iter().map(move |&g| (zero, g)));
            for (zero, good) in lists.collect::<Vec<_>>() {
                let mut open: Vec<usize> = Vec::new();
                for (i, o) in self.orders.iter().enumerate() {
                    if o.player == player
                        && o.good == good
                        && !served[i]
                        && self.in_list_zero(o.target.at) == zero
                    {
                        open.push(i);
                    }
                }
                // An order whose building is gone is dropped.
                open.retain(|&i| {
                    let gone = self.target_flag(self.orders[i].target).is_none();
                    served[i] = gone;
                    !gone
                });
                while let Some(&first) = open.first() {
                    let flag = self.target_flag(self.orders[first].target).expect("a flag");
                    let network = self.route_costs_from(player, flag);
                    let (group, rest): (Vec<usize>, Vec<usize>) = open.iter().partition(|&&i| {
                        let f = self.target_flag(self.orders[i].target).expect("a flag");
                        f == flag || network.contains_key(&f)
                    });
                    open = rest;
                    if let Some(i) = self.match_in_network(player, good, &group, flag, &network) {
                        served[i] = true;
                    }
                }
            }
        }
        // SHIP-4.1 (Q-063 end (1), (2)): an order that took its ware, or
        // whose building is gone, leaves its harbour's need at once.
        for i in (0..self.orders.len()).filter(|&i| served[i]) {
            if let Some(h) = self.orders[i].harbour.take() {
                self.detach_need(h, self.orders[i].good);
            }
        }
        let mut i = 0;
        self.orders.retain(|_| {
            i += 1;
            !served[i - 1]
        });
    }

    /// SHIP-4.1 (Q-063): the harbour step of the order `i` the pass chose
    /// with no warehouse over land to serve it. Unattached: it is attached
    /// to the harbour [`World::harbour_for`] gives, if any, adding one to
    /// that harbour's need of the good. Attached: it stays while its
    /// harbour still accepts the good and is reachable over the roads from
    /// the order's flag, else it is detached (and may attach again at a
    /// later pass). SHIP-4.1 (T24): the choice asks the stored costs from
    /// `representative`, the flag of the matching's representative order.
    fn harbour_step(&mut self, i: usize, representative: Point) {
        let order = self.orders[i];
        if order.harbour.is_none() && !self.has_harbour(order.player) {
            return;
        }
        let Some(flag) = self.target_flag(order.target) else {
            return;
        };
        let costs = self.route_costs_to(order.player, flag, Search::Ware);
        match order.harbour {
            Some(h) => {
                let keeps = self.harbour_at(h).is_some()
                    && self.warehouse_accepts(h, order.good)
                    && costs.from(self.flag_of(h)).is_some();
                if !keeps {
                    self.orders[i].harbour = None;
                    self.detach_need(h, order.good);
                }
            }
            None => {
                if let Some(h) =
                    self.harbour_for_stored(order.player, order.good, &costs, representative)
                {
                    self.orders[i].harbour = Some(h);
                    self.attach_need(h, order.good);
                }
            }
        }
    }

    /// SHIP-4.1 (Q-063 end (2)): orders leaving the open orders without a
    /// ware leave their harbours' needs.
    pub(crate) fn detach_orders(&mut self, removed: &[Order]) {
        for o in removed {
            if let Some(h) = o.harbour {
                self.detach_need(h, o.good);
            }
        }
    }

    /// ECO-DIST-3.3a: an order of the building on `at` is in list 0: a
    /// finished warehouse-type building (headquarters, warehouse, harbour).
    fn in_list_zero(&self, at: Point) -> bool {
        self.warehouses.contains_key(&at)
            && self.building_at(at).is_some_and(|b| !b.under_construction)
    }

    /// ECO-REQ-5.3d: the turn of `good` in the network of the flag `flag`
    /// of its representative order (`network`, the costs from it), the
    /// open orders there being `group`. An order is chosen only at the
    /// points of 5.3d, and each choice marks it whether or not it is then
    /// served. Returns the order served.
    fn match_in_network(
        &mut self,
        player: PlayerId,
        good: u32,
        group: &[usize],
        flag: Point,
        network: &std::collections::BTreeMap<Point, u32>,
    ) -> Option<usize> {
        // 1. A free ware of the good lies at a flag of the network (the
        // representative's own included): choose D1.
        let in_network = |at: Point| at == flag || network.contains_key(&at);
        if Some(good) != self.roles.coin
            && self.free_wares(player, good).any(|(_, at)| in_network(at))
        {
            let d1 = self.choose_order(player, good, group)?;
            // SHIP-4.1: a served order leaves its harbour (match_orders);
            // the copy filed with its ware is attached to none.
            let order = Order {
                harbour: None,
                ..self.orders[d1]
            };
            let d1_flag = self.target_flag(order.target).expect("a flag");
            // (a) when a free ware for D1 lies off its flag; either way the
            // good is done in this network.
            if let Some(served) = self.match_free_ware(order, d1_flag) {
                return served.then_some(d1);
            }
            // None for D1: on with 2, D1 staying marked.
        }
        // 2. H for the representative's flag.
        let costs = self.route_costs_to(player, flag, Search::Ware);
        let warehouses = self.warehouses_of(player);
        match self.stocked_warehouse(player, warehouses.clone(), good, flag, &costs) {
            None => {
                // No H: an order is chosen (and marked); nothing is sent.
                // SHIP-4.1 (Q-063): then its harbour step.
                if let Some(i) = self.choose_order(player, good, group) {
                    self.harbour_step(i, flag);
                }
                None
            }
            Some((h, _)) if self.can_send(h) => {
                let d2 = self.choose_order(player, good, group)?;
                let order = Order {
                    harbour: None,
                    ..self.orders[d2]
                };
                let d2_flag = self.target_flag(order.target).expect("a flag");
                // T34: the warehouse nearest to D2's flag, other than D2's
                // own building, only has to pass the three tests; H sends.
                let costs = self.route_costs_to(player, d2_flag, Search::Ware);
                let others: Vec<Point> = warehouses
                    .into_iter()
                    .filter(|&w| w != order.target.at)
                    .collect();
                let (h2, _) = self.stocked_warehouse(player, others, good, d2_flag, &costs)?;
                self.can_send(h2)
                    .then(|| self.send_from(h, order, d2_flag))?;
                Some(d2)
            }
            // Cooldown running, no helper or the flag full: no choice.
            Some(_) => None,
        }
    }

    /// ECO-REQ-5.3 (a), 5.4, 5.4a: the free wares of `good` (on the
    /// player's free-ware list of the good, lying at one of the player's
    /// flags, without an order) and their flags, in the list's order.
    fn free_wares(&self, player: PlayerId, good: u32) -> impl Iterator<Item = (usize, Point)> + '_ {
        let mut free: Vec<(u64, usize, Point)> = self
            .wares()
            .filter_map(move |(id, w)| {
                let WareLocation::Flag(at) = w.location else {
                    return None;
                };
                let free = w.target.is_none_or(|t| t.storage);
                let listed = (w.good == good && free && self.flag_at(at) == Some(player))
                    .then_some(w.free_place)??;
                Some((listed, id, at))
            })
            .collect();
        free.sort_unstable();
        free.into_iter().map(|(_, id, at)| (id, at))
    }

    /// ECO-REQ-5.3 (b): whether the warehouse on `h` can send a ware now:
    /// its cooldown is over, it has a helper and fewer than the flag limit
    /// of wares are counted at its flag (TRN-CARRY-4.3, no direction test,
    /// 4.2a).
    fn can_send(&self, h: Point) -> bool {
        let warehouse = &self.warehouses[&h];
        let helper = self.roles.helper.is_some_and(|g| warehouse.count(g) > 0);
        warehouse.cooldown == 0
            && helper
            && self.flag_load(self.flag_of(h)) < self.settings.flag_package_limit as usize
    }

    /// ECO-DIST-3.3: which of the open orders `group` of `good` in one
    /// network is served; ECO-REQ-5.3d, ECO-DIST-3.3b: the chosen order is
    /// marked with the current tick.
    fn choose_order(&mut self, player: PlayerId, good: u32, group: &[usize]) -> Option<usize> {
        let chosen = self.pick_order(player, good, group)?;
        self.orders[chosen].stamp = self.tick;
        Some(chosen)
    }

    /// ECO-DIST-3.3: the pick itself.
    pub(crate) fn pick_order(
        &mut self,
        player: PlayerId,
        good: u32,
        group: &[usize],
    ) -> Option<usize> {
        // ECO-DIST-1.8: a warehouse's own order only while it accepts.
        let group: Vec<usize> = group
            .iter()
            .copied()
            .filter(|&i| self.warehouse_accepts(self.orders[i].target.at, good))
            .collect();
        if group.is_empty() {
            return None;
        }
        let plain = |w: &Self| {
            group
                .iter()
                .copied()
                .min_by_key(|&i| (w.orders[i].stamp, i))
        };
        // ECO-DIST-3.3a (T24): list 0 (the groups are of one list) by the
        // plain rule, also for a good with a table and for coins.
        if self.in_list_zero(self.orders[group[0]].target.at) {
            return plain(self);
        }
        // MIL-PROM-3.3: coins go by the enemy-distance accumulators.
        if Some(good) == self.roles.coin {
            return self.pick_coin_order(&group);
        }
        match self.table_of(good) {
            Some(table) => self.pick_by_table(player, table, &group),
            None => plain(self),
        }
    }

    /// ECO-REQ-5.3 (a) for the order whose building's flag is `flag`: `None`
    /// when no free ware for it lies off that flag in its network; else
    /// whether it was served, by that ware or, when H's stored cost is
    /// lower, by H as in (b).
    fn match_free_ware(&mut self, order: Order, flag: Point) -> Option<bool> {
        let player = order.player;
        let good = order.good;
        // The flags in D's ware network (a search that stores nothing).
        let network = self.route_costs_to(player, flag, Search::Ware);
        // T28: the nearest free ware by field distance from the building's
        // field (not its flag), not at the building's own flag; the first
        // in the list wins a tie.
        let (ware, at) = self
            .free_wares(player, good)
            .filter(|&(_, at)| at != flag && network.from(at).is_some())
            .min_by_key(|&(_, at)| grid::distance(order.target.at, at))?;
        let warehouses = self.warehouses_of(player);
        let h = self.stocked_warehouse(player, warehouses, good, flag, &network);
        // TRN-CARRY-5.3a: the stored cost from D's flag to Wf's flag against
        // H's stored cost. ECO-REQ-5.3 (a) (T44): without an H neither cost
        // is asked, so nothing is stored for the pair then.
        let from_h = h.and_then(|(h, hc)| {
            let cost = self.stored_ware_cost(player, flag, at).unwrap_or(u32::MAX);
            (cost > hc).then_some(h)
        });
        match from_h {
            Some(h) => {
                let sent = self.can_send(h);
                if sent {
                    self.send_from(h, order, flag);
                }
                Some(sent)
            }
            _ => {
                let w = self.wares[ware].as_mut().expect("live ware");
                // ECO-REQ-5.4a: given an order, it leaves the free-ware list.
                w.free_place = None;
                w.destination = Some(flag);
                w.target = Some(order.target);
                // ECO-REQ-5.3f: the served order is filed with its ware.
                w.order = Some(order);
                // ECO-REQ-5.3e (Q-282): its next flag is cleared, not
                // computed, and a carrier's claim on it is released at once:
                // the next re-check (ECO-DIST-1.6) computes the next flag,
                // and any carrier may claim it then.
                w.next = None;
                w.claimed_by = None;
                w.stranded = false;
                Some(true)
            }
        }
    }

    /// ECO-REQ-5.3 (b): the warehouse on `h`, able to send (`can_send`),
    /// sends one of the order's good to its building, whose flag is `flag`.
    fn send_from(&mut self, h: Point, order: Order, flag: Point) {
        // W16: for a recipe group's order H hands out the member chosen by
        // `group_member_at`; the order keeps the group.
        let good = if self.recipe_groups.contains_key(&order.good) {
            self.group_member_at(order.player, h, order.good)
                .expect("checked by stocked_warehouse")
        } else {
            order.good
        };
        let helper = self.roles.helper.expect("checked by can_send");
        let warehouse = self.warehouses.get_mut(&h).expect("a warehouse");
        warehouse.take(good);
        warehouse.take(helper);
        warehouse.cooldown = SEND_COOLDOWN;
        // ECO-REQ-5.3 (b), 5.3c: a helper appears at H's door with the ware
        // and carries it out to H's flag.
        let door = self.door_field(h);
        let warehouse = self.target_of(h).expect("a warehouse");
        // SIM-START-4.4 f: the helper is made at the door, then the ware.
        let number = self.new_number();
        self.new_number();
        self.add_settler(Settler {
            number,
            player: order.player,
            profession: helper,
            list: crate::SettlerList::Carriers,
            walker: crate::movement::Walker::new(door),
            job: SettlerJob::CarryOut {
                warehouse,
                destination: Some(flag),
                target: Some(order.target),
                order: Some(order),
                step: crate::settlers::CarryOutStep::Door { score: 0 },
            },
            timer: crate::settlers::HELPER_DOOR_TICKS,
            carrying: Some(good),
        });
    }

    /// ECO-REQ-5.5: every ware lying at a flag without a destination goes to
    /// the warehouse of its network with the lowest route cost; straight in
    /// if it lies at that warehouse's flag, also when a carrier has claimed
    /// it (T27). A ware whose building is gone loses its destination first.
    fn store_wares(&mut self) {
        let ids: Vec<usize> = self.wares().map(|(id, _)| id).collect();
        for id in ids {
            let w = self.wares[id].as_ref().expect("live ware");
            let WareLocation::Flag(at) = w.location else {
                continue;
            };
            if let Some(target) = w.target {
                // ECO-REQ-5.7 (c): a storage trip without a next flag is
                // chosen again, in the next run (T41).
                let lost = target.storage && w.stranded;
                let gone = self.target_flag(target).is_none();
                if !lost && !gone {
                    continue;
                }
                // ECO-REQ-5.4a: an order dropped while its ware lies at a
                // flag puts the ware on the free-ware list (a lost storage
                // trip keeps its place).
                if gone && !target.storage {
                    self.join_free_list(id);
                }
                let w = self.wares[id].as_mut().expect("live ware");
                w.destination = None;
                w.target = None;
                w.order = None;
                w.stranded = false;
                w.next = None;
                w.recheck = 0;
                // T41: the lost trip is passed over in this run; a ware whose
                // building is gone is placed again at once (as before; the
                // run's pass-over is measured for a lost trip only).
                if lost {
                    continue;
                }
            }
            if self.wares[id]
                .as_ref()
                .expect("live ware")
                .destination
                .is_some()
            {
                continue;
            }
            let Some(player) = self.flag_at(at) else {
                continue;
            };
            // The flags in the ware's network (a search that stores nothing).
            let network = self.route_costs_from(player, at);
            let mut best: Option<(Point, u32)> = None;
            for warehouse in self.warehouses_of(player) {
                // ECO-DIST-1.5: only warehouses that accept the good.
                let good = self.wares[id].as_ref().expect("live ware").good;
                let warehouse_flag = self.flag_of(warehouse);
                if !self.warehouse_accepts(warehouse, good)
                    || !network.contains_key(&warehouse_flag)
                {
                    continue;
                }
                // TRN-CARRY-5.3a (8d46644, T26): the stored cost from the
                // ware's flag to the warehouse's.
                let Some(cost) = self.stored_ware_cost(player, at, warehouse_flag) else {
                    continue;
                };
                if best.is_none_or(|(_, c)| cost < c) {
                    best = Some((warehouse, cost));
                }
            }
            let Some((warehouse, _)) = best else { continue };
            let flag = self.flag_of(warehouse);
            if flag == at {
                // T27: claimed or not; a carrier on his way to fetch it
                // finds no ware at the flag (ECO-REQ-5.3e).
                let good = self.wares[id].as_ref().expect("live ware").good;
                self.remove_ware(id);
                self.warehouses
                    .get_mut(&warehouse)
                    .expect("a warehouse")
                    .add(good, 1);
                continue;
            }
            let mut target = self.target_of(warehouse).expect("a warehouse");
            target.storage = true;
            let w = self.wares[id].as_mut().expect("live ware");
            w.destination = Some(flag);
            w.target = Some(target);
            self.route_ware(id);
        }
    }

    // ---- Views ----

    /// The player's open settler requests, in queue order.
    pub fn requests(&self, player: PlayerId) -> Vec<Request> {
        self.queue
            .iter()
            .filter(|r| r.player == player)
            .copied()
            .collect()
    }

    /// ECO-REQ-2.1, 2.2 (T39): the one queue of all players and where it
    /// goes on next.
    pub fn request_queue(&self) -> (&[Request], usize) {
        (&self.queue, self.queue_cursor)
    }

    /// Open ware orders, in filing order.
    pub fn orders(&self) -> &[Order] {
        &self.orders
    }

    /// Messages to the players, oldest first.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// The warehouse on `at`.
    pub fn warehouse(&self, at: Point) -> Option<&Warehouse> {
        self.warehouses.get(&at)
    }
}

#[cfg(test)]
pub(crate) mod tests;
