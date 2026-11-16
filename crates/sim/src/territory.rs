//! Game start, territory and exploration (`specs/ECO-warehouse-construction.md`
//! ECO-HQ-1 to ECO-HQ-3).

use crate::grid::{self, Point};
use crate::map::{state, Terrain};
use crate::{PlayerId, Rules, World};

/// How a new game is set up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameSetup {
    /// Each player in the game and its start position (FMT-S2M-2.2).
    pub players: Vec<(PlayerId, Point)>,
    /// ECO-HQ-2.2: the start resources setting, 0 to 3.
    pub start_resources: u8,
    /// The player whose placement classes are kept.
    pub local_player: PlayerId,
    /// The map's deposits.
    pub deposits: Vec<MapDeposit>,
    /// ECO-OUT-4.3, 4.5: per field (row by row), or empty.
    pub ground: Vec<crate::Ground>,
    /// FMT-S2M-4.5: the resource of each field (row by row), or empty.
    pub resources: Vec<Option<crate::FieldResource>>,
    /// TRN-FREE-5.5b: each player's tribe (0 Romans, 1 Africans, 2 Chinese);
    /// players not listed are Romans (Q-051). A setup entry may carry -1
    /// (unset, UI-SETUP-1.2): it is treated as 0 (UI-SETUP-2.6, Q-053).
    pub tribes: Vec<(PlayerId, i8)>,
    /// FMT-S2M-5.4: the game animals standing on the map.
    pub animals: Vec<MapAnimal>,
    /// MIL-SOL-3.1: each player's team (UI-SETUP); players not listed, and
    /// -1, are in no team.
    pub teams: Vec<(PlayerId, i8)>,
    /// SHIP-1.1: the harbour spots the map marks on its land continents.
    pub harbour_spots: Vec<Point>,
    /// FMT-S2M-4.8, 4.9: the map's continent index per field (row by row,
    /// -1 none), or empty for a world without a map file.
    pub continents: Vec<i32>,
    /// FMT-S2M-4.9: each land continent object's index and positions.
    pub land_spots: Vec<(i32, Vec<Point>)>,
    /// SIM-START-3.2: the set-up option that shuffles the start positions:
    /// each player in slot order draws one of the positions of `players`
    /// not yet taken. Off: each player gets its own (3.1).
    pub shuffle_starts: bool,
    /// ECO-HQ-3.6, UI-SETUP-1.8: the free game's fog of war set to Off:
    /// each player gets the whole map explored when it is made.
    pub fog_of_war_off: bool,
    /// ECO-FOOD-5.15, FMT-S2M-5.5: the map's respawn record.
    pub respawn: crate::Respawn,
    /// SIM-START-4.5: the object counter's value when the start sequence
    /// begins (its low 32 bits): the map file's next id (FMT-S2M-3.1
    /// `next_unique_id`), the first castle's number; the map's own objects
    /// keep their ids (T8: 90 175 on MP_2P_Storm_Coast).
    pub next_object: u32,
}

/// A game animal of the map, as its record holds it (FMT-S2M-5.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAnimal {
    /// Its animal type (`animals.lua`).
    pub kind: u32,
    /// The field it stands on.
    pub field: Point,
    /// ECO-FOOD-5.7: where it was placed (it roams around it).
    pub home: Point,
    /// ECO-FOOD-5.13: its state (0 waiting, 1 choosing, 2 walking) and
    /// counter (the wait left, or the ticket).
    pub state: i32,
    pub counter: i32,
    /// An animal saved in mid-walk: the field of the step in progress, its
    /// progress (the bits of the record's float32) and the fields after
    /// it, in walking order.
    pub walk: Option<(Point, u32, Vec<Point>)>,
}

impl MapAnimal {
    /// A waiting animal on its home field that chooses in the first tick
    /// (a map record with counter 0).
    pub fn new(kind: u32, field: Point) -> Self {
        Self {
            kind,
            field,
            home: field,
            state: 0,
            counter: 0,
            walk: None,
        }
    }
}

/// A deposit of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapDeposit {
    pub field: Point,
    pub kind: u32,
    /// Its stored sample position (TRN-FREE-5.1).
    pub position: Point,
    /// Ticks until it is ripe: 0 for a fully grown one (ECO-OUT-6.1).
    pub ripe_in: u32,
}

/// ECO-HQ-2.2: setting 0, 1, 2, 3 multiplies the start stock by 1, 2, 4, 8;
/// any other setting by 0.
pub fn start_multiplier(setting: u8) -> u32 {
    match setting {
        0..=3 => 1 << setting,
        _ => 0,
    }
}

/// ECO-HQ-3.5: who owns `field` among `claimants` (field, player, range):
/// the first within distance 2, else the greatest strength `r / d^2` (`r`
/// doubled for `d < 5`, and only for `d <= r`), equal strength to the lower
/// player number; nobody if none reaches it.
fn owner_by_strength(claimants: &[(Point, PlayerId, u32)], field: Point) -> Option<PlayerId> {
    if let Some(&(_, player, _)) = claimants
        .iter()
        .find(|(c, _, _)| grid::distance(*c, field) <= 2)
    {
        return Some(player);
    }
    // (player, r, d^2) of each player's strongest building; strengths are
    // compared as fractions.
    let mut best: Vec<(PlayerId, u64, u64)> = Vec::new();
    for &(c, player, range) in claimants {
        let d = u64::from(grid::distance(c, field));
        let r = if d < 5 {
            2 * u64::from(range)
        } else {
            u64::from(range)
        };
        if d > r {
            continue;
        }
        let (num, den) = (r, d * d);
        match best.iter_mut().find(|(p, _, _)| *p == player) {
            Some(entry) if num * entry.2 > entry.1 * den => *entry = (player, num, den),
            Some(_) => {}
            None => best.push((player, num, den)),
        }
    }
    best.into_iter()
        .reduce(|a, b| {
            let (left, right) = (a.1 * b.2, b.1 * a.2);
            if right > left || (right == left && b.0 < a.0) {
                b
            } else {
                a
            }
        })
        .map(|(player, _, _)| player)
}

/// UI-SETUP-2.6: a setup entry's tribe as the index the simulation uses; the
/// unset value -1 is tribe 0.
fn tribe_of_setup(tribe: i8) -> u8 {
    u8::try_from(tribe).unwrap_or(0)
}

/// The bit of `player` in the per-field explored mask.
pub(crate) fn player_bit(player: PlayerId) -> u8 {
    assert!(player.0 < 8, "player {} out of range", player.0);
    1 << player.0
}

/// TER-2.4: what a landing harbour site claims and explores.
const LANDING_CLAIM: u32 = 4;
const LANDING_EXPLORE: u32 = 6;

impl World {
    /// TRN-FREE-5.5b: set a player's tribe (0 Romans, 1 Africans, 2 Chinese),
    /// a per-player game setup value, before building starts. An unset
    /// tribe (-1) is 0 (UI-SETUP-2.6, Q-053): it is never used as an index.
    pub fn set_tribe(&mut self, player: PlayerId, tribe: i8) {
        self.tribes.insert(player, tribe_of_setup(tribe));
    }

    /// TRN-FREE-5.5b: the tribe of `player`; Romans (0) unless set.
    pub fn tribe(&self, player: PlayerId) -> u8 {
        self.tribes.get(&player).copied().unwrap_or(0)
    }

    /// A new game: every player's headquarters on its start position with
    /// the start stock, its territory and the land it has explored
    /// (ECO-HQ-1 to 3). Players whose start position is outside the map, and
    /// all players when the rules name no headquarters type, get nothing.
    pub fn new_game(seed: u64, terrain: Terrain, rules: Rules, setup: &GameSetup) -> Self {
        let mut world = Self::empty(seed, terrain, rules, setup.local_player);
        world.tribes = setup
            .tribes
            .iter()
            .map(|&(player, tribe)| (player, tribe_of_setup(tribe)))
            .collect();
        world.teams = setup.teams.iter().copied().collect();
        world.players = setup.players.iter().map(|&(player, _)| player).collect();
        world.set_harbour_spots(setup.harbour_spots.clone());
        world.set_continents(setup.continents.clone());
        world.land_spots = setup.land_spots.iter().cloned().collect();
        if setup.ground.len() == world.terrain.len() {
            world.ground = setup.ground.clone();
        }
        if setup.resources.len() == world.terrain.len() {
            world.resources = setup.resources.clone();
        }
        world.respawn = setup.respawn;
        // SIM-START-4.5: the map's deposits, doodads and animals keep their
        // own numbers and take none.
        world.next_object = setup.next_object;
        // ECO-FOOD-5.13: the map's animals in record order, each in the
        // state and with the counter its record holds; nothing is drawn.
        for a in &setup.animals {
            if world.terrain.contains(a.field) && world.terrain.contains(a.home) {
                let animal = world.map_animal(a);
                world.animals.push(Some(animal));
            }
        }
        for d in &setup.deposits {
            if world.terrain.contains(d.field) && world.deposit_kinds.contains_key(&d.kind) {
                let deposit = crate::Deposit {
                    kind: d.kind,
                    position: d.position,
                    state: crate::DepositState::Growing {
                        ripe_at: u64::from(d.ripe_in),
                    },
                    reserved_by: None,
                    life_left: None,
                };
                world.add_deposit(d.field, deposit);
            }
        }
        // SIM-START-1.1, SIM-RNG-2.2: the generator is reset before the
        // players are made, so every new game's first draw is the same and
        // a shuffled start (3.2) is the same in every game of a set-up.
        world.rng.reset();
        let multiplier = start_multiplier(setup.start_resources);
        let mut free: Vec<Point> = setup.players.iter().map(|&(_, at)| at).collect();
        // SIM-START-1.1, 3.1: the players in slot order.
        let mut players = setup.players.clone();
        players.sort_by_key(|&(player, _)| player);
        let (width, height) = (world.terrain.width, world.terrain.height);
        for (player, own) in players {
            // ECO-HQ-3.6: fog of war Off: right after the player is made, the
            // exploration of ECO-HQ-3.3 around the map's centre with radius
            // 2 x max(width, height) (the outermost rows and columns stay
            // unexplored).
            if setup.fog_of_war_off {
                world.explore(player, (width / 2, height / 2), 2 * width.max(height));
            }
            let at = if setup.shuffle_starts {
                // SIM-START-3.2: `draw mod (positions left)`.
                let k = world.rng.draw() as usize % free.len();
                free.remove(k)
            } else {
                own
            };
            if world.terrain.contains(at) {
                world.place_headquarters(player, at, multiplier);
            }
        }
        world.recompute_all_placement();
        world
    }

    /// ECO-HQ-1, 2, 3 for one player. Placement classes are not updated.
    fn place_headquarters(&mut self, player: PlayerId, at: Point, multiplier: u32) {
        let Some(building_type) = self
            .headquarters
            .and_then(|id| self.building_types.get(&id))
            .cloned()
        else {
            return;
        };
        // ECO-HQ-1.1, 3.4: object bits and flag as ECO-PLACE-5.1, finished.
        let new_flag = self.new_flag_of(at);
        self.put_building(player, building_type.id, building_type.size, at, false);
        // ECO-HQ-4.8a: the castle's own field first takes the mean height
        // around it; ECO-HQ-1.2: then the ground is levelled at once to it.
        self.level_site_height(at);
        if self.needs_levelling(at, building_type.size) {
            let height = self.terrain.height_at(at);
            for neighbour in self.terrain.neighbours(at).into_iter().flatten() {
                self.set_field_height(neighbour, height);
            }
        }
        // ECO-HQ-4.8b: the flag's height ring.
        if let Some(flag) = new_flag {
            self.flag_height_ring(flag);
        }
        // ECO-HQ-2.1, 2.2
        let stock = self
            .start_stock
            .iter()
            .map(|&(good, count)| (good, count * multiplier))
            .collect();
        self.warehouses.insert(
            at,
            crate::Warehouse {
                stock,
                cooldown: 0,
                flags: Default::default(),
            },
        );
        self.complete(at);
        // ECO-HQ-3, TER-2.2
        self.apply_territory(at);
        self.territory_changed(at, building_type.territory_range, true);
    }

    /// TER-2.1, 2.2: a building becomes active: the one-time flag is set,
    /// the land within its range is claimed and the radius explored. No
    /// message and no placement update here.
    pub(crate) fn apply_territory(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let (range, radius) = (kind.territory_range, kind.explore_radius);
        self.military.get_mut(&at).expect("military").applied = true;
        if range > 0 {
            self.claim(at, range);
        }
        self.explore(building.owner, at, radius);
    }

    /// TER-2.2: a finished building claims and explores at once: the
    /// one-time flag, the land, Border and the placement classes. No
    /// message of its own.
    pub(crate) fn claim_now(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building");
        let range = self.building_types[&building.building_type].territory_range;
        self.apply_territory(at);
        if range > 0 {
            self.territory_changed(at, range, true);
            self.recompute_placement_area(at, range);
        }
    }

    /// TER-2.4: a harbour site placed by a landing expedition claims range
    /// 4 and explores radius 6 at once.
    pub(crate) fn claim_landing(&mut self, at: Point) {
        let owner = self.building_at(at).expect("a harbour site").owner;
        self.claim(at, LANDING_CLAIM);
        self.explore(owner, at, LANDING_EXPLORE);
        self.territory_changed(at, LANDING_CLAIM, true);
        self.recompute_placement_area(at, LANDING_CLAIM);
    }

    /// TER-2.1: the first building update with a soldier present (or, for
    /// a type with a worker and no soldier limit, its worker) claims once.
    pub(crate) fn check_occupation(&mut self, at: Point) {
        let Some(m) = self.military.get(&at) else {
            return;
        };
        if m.applied {
            return;
        }
        let building = *self.building_at(at).expect("a building");
        let kind = &self.building_types[&building.building_type];
        let active = if kind.soldier_limit > 0 {
            self.present_soldiers(at) > 0
        } else {
            kind.worker.is_some() && kind.worker_limit > 0 && building.active
        };
        if !active {
            return;
        }
        let (range, headquarters) = (kind.territory_range, kind.warehouse);
        if range == 0 {
            self.message(building.owner, at, "Watchtower occupied!");
        } else if !headquarters {
            self.message(building.owner, at, "Military building occupied!");
        }
        self.apply_territory(at);
        if range > 0 {
            self.territory_changed(at, range, true);
            self.recompute_placement_area(at, range);
        }
    }

    /// TER-2.3: the type of the building on `at` changes; an applied
    /// building claims and explores again with the new type at once.
    pub(crate) fn change_building_type(&mut self, at: Point, to: u32) {
        let index = self.terrain.index(at);
        let Some(building) = self.buildings[index].as_mut() else {
            return;
        };
        building.building_type = to;
        // W15: a deepening's construction completes as an upgrade does.
        self.deepening_done(at);
        let applied = self.military.get(&at).is_some_and(|m| m.applied);
        if applied {
            self.apply_territory(at);
            let range = self.building_types[&to].territory_range;
            if range > 0 {
                self.territory_changed(at, range, true);
                self.recompute_placement_area(at, range);
            }
        }
    }

    /// TER-3.1: the claimants for a claim or release around `at` with
    /// range `range`: the claiming building first (when `first` is given),
    /// then every other building with a set flag within 3 x range, in
    /// creation order.
    fn claimants(
        &self,
        at: Point,
        range: u32,
        first: Option<PlayerId>,
    ) -> Vec<(Point, PlayerId, u32)> {
        let mut found: Vec<(Point, PlayerId, u32)> = Vec::new();
        if let Some(owner) = first {
            found.push((at, owner, range));
        }
        let mut others: Vec<(u32, Point, PlayerId, u32)> = self
            .military
            .iter()
            .filter(|&(&c, m)| m.applied && c != at && grid::distance(c, at) <= 3 * range)
            .filter_map(|(&c, _)| {
                let b = self.building_at(c)?;
                let r = self.building_types[&b.building_type].territory_range;
                (r > 0).then_some((b.serial, c, b.owner, r))
            })
            .collect();
        others.sort_unstable_by_key(|&(serial, ..)| serial);
        found.extend(others.into_iter().map(|(_, c, p, r)| (c, p, r)));
        found
    }

    /// TER-3: the building on `at` claims the fields within `range`;
    /// previous owners that lose a field get "Lost Territory.".
    fn claim(&mut self, at: Point, range: u32) {
        let building = *self.building_at(at).expect("a claiming building");
        let claimants = self.claimants(at, range, Some(building.owner));
        let (width, height) = (self.terrain.width, self.terrain.height);
        let mut lost: Vec<PlayerId> = Vec::new();
        for field in grid::area(at, range, width, height) {
            let owner = owner_by_strength(&claimants, field);
            let index = self.terrain.index(field);
            if let Some(before) = self.owners[index] {
                if owner != Some(before) && !lost.contains(&before) {
                    lost.push(before);
                }
            }
            self.owners[index] = owner;
        }
        for player in lost {
            self.message(player, at, "Lost Territory.");
        }
    }

    /// TER-4.1: the applied building that stood on `at` with range `range`
    /// is gone: every field within its range is recomputed over the others.
    pub(crate) fn release_territory(&mut self, at: Point, range: u32) {
        let claimants = self.claimants(at, range, None);
        let (width, height) = (self.terrain.width, self.terrain.height);
        for field in grid::area(at, range, width, height) {
            let owner = owner_by_strength(&claimants, field);
            let index = self.terrain.index(field);
            self.owners[index] = owner;
        }
        self.territory_changed(at, range, false);
        self.recompute_placement_area(at, range);
    }

    /// TER-5 (in this order, without the placement classes, which the
    /// caller recomputes at the end): Border in the square r + 2, roads on
    /// Border or foreign fields within r + 4, and, for a claim, contact.
    fn territory_changed(&mut self, at: Point, range: u32, claim: bool) {
        let (width, height) = (self.terrain.width, self.terrain.height);
        // 5.1
        let reach = i64::from(range) + 2;
        let (cx, cy) = (i64::from(at.0), i64::from(at.1));
        for y in (cy - reach).max(0)..=(cy + reach).min(i64::from(height) - 1) {
            for x in (cx - reach).max(0)..=(cx + reach).min(i64::from(width) - 1) {
                let field = (x as u32, y as u32);
                let border = self.is_border(field);
                self.set_bits(field, state::BORDER, border);
            }
        }
        // 5.3
        let mut doomed: Vec<usize> = Vec::new();
        for field in grid::area(at, range + 4, width, height) {
            let index = self.terrain.index(field);
            let Some(id) = self.streets[index] else {
                continue;
            };
            let Some(road) = self.roads[id].as_ref() else {
                continue;
            };
            let foreign = self.owners[index] != Some(road.owner);
            if (foreign || self.terrain.has(field, state::BORDER)) && !doomed.contains(&id) {
                doomed.push(id);
            }
        }
        for id in doomed {
            self.remove_road(id);
        }
        // 5.4
        if claim {
            let mine = self.building_at(at).map(|b| b.owner);
            for field in grid::area(at, range + 1, width, height) {
                if let (Some(owner), Some(mine)) = (self.field_owner(field), mine) {
                    if owner != mine {
                        self.contacts
                            .insert((owner.0.min(mine.0), owner.0.max(mine.0)));
                    }
                }
            }
        }
    }

    /// TER-5.2: the placement classes within `range + 4` of `at`.
    pub(crate) fn recompute_placement_area(&mut self, at: Point, range: u32) {
        let (width, height) = (self.terrain.width, self.terrain.height);
        for field in grid::area(at, range + 4, width, height) {
            let class = self.compute_placement(self.local_player, field);
            let index = self.terrain.index(field);
            self.placement[index] = class;
        }
    }

    /// TER-6.3: in the roads update, every flag whose field is not its
    /// owner's inner land is removed with its roads.
    pub(crate) fn remove_lost_flags(&mut self) {
        let mut i = 0;
        while i < self.flag_order.len() {
            let field = self.flag_order[i];
            let lost = self
                .flag_at(field)
                .is_some_and(|owner| !self.is_inner_land(owner, field));
            if lost {
                self.remove_flag(field);
            } else {
                i += 1;
            }
        }
    }

    /// TRN-ROAD-6.2: a flag is removed. In this order: every road that ends
    /// at it; the building behind it, with remains; then the flag with all
    /// wares lying on it, destroyed (TRN-ROAD-6.4), the wares of the
    /// building's remains (MIL-CON-4.4) among them: made on this flag, each
    /// with its number, and destroyed with it.
    pub(crate) fn remove_flag(&mut self, field: Point) {
        let ends: Vec<usize> = self
            .roads
            .iter()
            .enumerate()
            .filter_map(|(id, r)| {
                let r = r.as_ref()?;
                (r.path[0] == field || r.path.last() == Some(&field)).then_some(id)
            })
            .collect();
        for id in ends {
            self.remove_road(id);
        }
        let building = self
            .terrain
            .neighbours(field)
            .into_iter()
            .flatten()
            .find(|&n| self.building_at(n).is_some() && self.flag_of(n) == field);
        if let Some(at) = building {
            self.remove_with_remains(at);
        }
        for ware in self.wares_at(field).to_vec() {
            self.destroy_ware(ware);
        }
        self.flag_wares.remove(&field);
        self.flag_bar.remove(&field);
        let index = self.terrain.index(field);
        self.flags[index] = None;
        self.flag_order.retain(|&f| f != field);
        self.changed(&[field]);
    }

    /// TER-6.1: the field is inner land of `player`.
    pub(crate) fn is_inner_land(&self, player: PlayerId, field: Point) -> bool {
        self.terrain.contains(field)
            && self.field_owner(field) == Some(player)
            && !self.terrain.has(
                field,
                state::BORDER | state::WATER | state::ELEVATION_BLOCKED | state::DOODAD_BLOCKED,
            )
    }

    /// ECO-PLACE-1.4: the field belongs to a player and a neighbour is
    /// outside the map or belongs to someone else or no one.
    fn is_border(&self, field: Point) -> bool {
        let Some(owner) = self.field_owner(field) else {
            return false;
        };
        self.terrain
            .neighbours(field)
            .iter()
            .any(|n| n.is_none_or(|n| self.field_owner(n) != Some(owner)))
    }

    /// ECO-HQ-3.3: `player` has explored every field within `radius` of
    /// `center`, except the outermost rows and columns of the map.
    pub(crate) fn explore(&mut self, player: PlayerId, center: Point, radius: u32) {
        let (width, height) = (self.terrain.width, self.terrain.height);
        let bit = player_bit(player);
        // GFX-VIEW-5.4 (d): every exploration counts, whatever it sets.
        self.explorations += 1;
        for (x, y) in grid::area(center, radius, width, height) {
            if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                continue;
            }
            let index = self.terrain.index((x, y));
            self.explored[index] |= bit;
        }
    }

    /// ECO-HQ-2: the stock of the warehouse on `at`, (good, count) in order.
    pub fn stock(&self, at: Point) -> Option<&[(u32, u32)]> {
        self.warehouses.get(&at).map(|w| w.stock.as_slice())
    }

    /// All warehouses with their stock, by building field.
    pub fn warehouses(&self) -> impl Iterator<Item = (Point, &[(u32, u32)])> {
        self.warehouses
            .iter()
            .map(|(&at, w)| (at, w.stock.as_slice()))
    }

    /// Number of fields `player` owns.
    pub fn owned_field_count(&self, player: PlayerId) -> usize {
        self.owners.iter().filter(|&&o| o == Some(player)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{BuildingSize, BuildingType, Placement};

    const CASTLE: u32 = 100;
    const PLANK: u32 = 7;
    const STONE: u32 = 8;

    fn rules() -> Rules {
        let mut castle = BuildingType::new(CASTLE, BuildingSize::Large);
        castle.territory_range = 9;
        castle.explore_radius = 12;
        Rules {
            building_types: vec![castle],
            headquarters: Some(CASTLE),
            start_stock: vec![(PLANK, 22), (STONE, 34)],
            ..Rules::empty()
        }
    }

    fn game(terrain: crate::map::Terrain, at: Point, start_resources: u8) -> World {
        let setup = GameSetup {
            players: vec![(P0, at)],
            start_resources,
            local_player: P0,
            ..Default::default()
        };
        World::new_game(3, terrain, rules(), &setup)
    }

    // UI-SETUP T8, UI-SETUP-2.6: an unset tribe (-1) is treated as 0.
    #[test]
    fn unset_tribe_is_romans() {
        let setup = GameSetup {
            players: vec![(P0, (20, 20))],
            local_player: P0,
            tribes: vec![(P0, -1), (crate::PlayerId(1), 2)],
            ..Default::default()
        };
        let mut world = World::new_game(3, flat(40, 40), rules(), &setup);
        assert_eq!(world.tribe(P0), 0);
        assert_eq!(world.tribe(crate::PlayerId(1)), 2);
        assert_eq!(world.tribe(crate::PlayerId(2)), 0, "players not listed");
        world.set_tribe(crate::PlayerId(3), -1);
        assert_eq!(world.tribe(crate::PlayerId(3)), 0);
        world.set_tribe(crate::PlayerId(3), 1);
        assert_eq!(world.tribe(crate::PlayerId(3)), 1);
    }

    // ECO-HQ-2.2
    #[test]
    fn start_resources_multiply() {
        assert_eq!([0, 1, 2, 3, 4].map(start_multiplier), [1, 2, 4, 8, 0]);
    }

    // ECO-HQ T1, T2 on a synthetic rule set
    // spec: ECO-HQ-2.1, ECO-HQ-2.2 (T1, T2)
    #[test]
    fn headquarters_stock() {
        let at = (20, 20);
        let world = game(flat(40, 40), at, 0);
        assert_eq!(world.stock(at), Some(&[(PLANK, 22), (STONE, 34)][..]));
        let world = game(flat(40, 40), at, 2);
        assert_eq!(world.stock(at), Some(&[(PLANK, 88), (STONE, 136)][..]));
        let hq = world.building_at(at).unwrap();
        assert_eq!(hq.building_type, CASTLE);
        assert!(!hq.under_construction);
        assert_eq!(world.flag_at((20, 19)), Some(P0));
    }

    // ECO-HQ T3, T5: exactly the fields within 9; 1 + 3 * 9 * 10 = 271.
    // spec: ECO-HQ-3.1 (T3, T5)
    #[test]
    fn territory_within_range() {
        let at = (27, 21);
        let world = game(flat(60, 50), at, 0);
        for field in world.terrain().fields() {
            let inside = grid::distance(at, field) <= 9;
            assert_eq!(world.field_owner(field) == Some(P0), inside, "{field:?}");
        }
        assert_eq!(world.owned_field_count(P0), 271);
        assert_eq!(world.field_owner((36, 21)), Some(P0));
        assert_eq!(world.field_owner((37, 21)), None);
    }

    // ECO-HQ-3.2, ECO-PLACE-1.4: the outermost ring of the territory is
    // Border; no flag can go there, nor outside.
    #[test]
    fn border_and_placement_follow_territory() {
        let at = (27, 21);
        let world = game(flat(60, 50), at, 0);
        for field in world.terrain().fields() {
            let border = grid::distance(at, field) == 9;
            assert_eq!(
                world.terrain().has(field, state::BORDER),
                border,
                "{field:?}"
            );
        }
        assert_eq!(world.placement_at((36, 21)), Placement::Nothing);
        assert_eq!(world.placement_at((37, 21)), Placement::Nothing);
        assert_eq!(world.placement_at((35, 21)), Placement::Flag);
        assert_eq!(world.placement_at((30, 21)), Placement::Large);
    }

    // ECO-HQ T4: within 12, but never the outermost rows and columns.
    // spec: ECO-HQ-3.3 (T4)
    #[test]
    fn exploration_skips_the_map_edge() {
        let at = (8, 8);
        let world = game(flat(30, 30), at, 0);
        for field in world.terrain().fields() {
            let edge = field.0 == 0 || field.1 == 0 || field.0 == 29 || field.1 == 29;
            let expected = grid::distance(at, field) <= 12 && !edge;
            assert_eq!(world.is_explored(P0, field), expected, "{field:?}");
        }
        assert!(!world.is_explored(PlayerId(1), at));
    }

    // ECO-HQ T27: a free game on an 80 x 80 map, two players, fog of war
    // Off: at the first tick every field with 0 < x < 79 and 0 < y < 79 is
    // explored for both players, the outermost rows and columns are not;
    // with fog of war On only the fields within 12 of each headquarters.
    // spec: ECO-HQ-3.6 (T27), ECO-HQ-3.3
    #[test]
    fn fog_of_war_off_explores_the_map_at_the_start() {
        let (p0, p1) = ((20, 20), (60, 60));
        let setup = |off: bool| GameSetup {
            players: vec![(P0, p0), (crate::PlayerId(1), p1)],
            local_player: P0,
            fog_of_war_off: off,
            ..Default::default()
        };
        let off = World::new_game(3, flat(80, 80), rules(), &setup(true));
        let on = World::new_game(3, flat(80, 80), rules(), &setup(false));
        for field in off.terrain().fields() {
            let edge = field.0 == 0 || field.1 == 0 || field.0 == 79 || field.1 == 79;
            for player in [P0, crate::PlayerId(1)] {
                assert_eq!(off.is_explored(player, field), !edge, "{field:?}");
            }
            let near = |at: Point| grid::distance(at, field) <= 12 && !edge;
            assert_eq!(on.is_explored(P0, field), near(p0), "{field:?}");
            assert_eq!(on.is_explored(crate::PlayerId(1), field), near(p1));
        }
    }

    // ECO-HQ T10: the headquarters' ground is levelled at once, to the
    // castle field's new height: the integer mean of the 19 fields within 2
    // (ECO-HQ-4.8a): (500 + 0 + 100 + .. + 500 + 12 x 0) / 19 = 105.
    // spec: ECO-HQ-1.2 (T10), ECO-HQ-4.8a
    #[test]
    fn headquarters_ground_is_levelled_at_once() {
        let at = (20, 20);
        let mut terrain = flat(40, 40);
        set_height(&mut terrain, at, 500);
        for (i, n) in terrain.neighbours(at).into_iter().flatten().enumerate() {
            set_height(&mut terrain, n, 100 * i as i32);
        }
        let world = game(terrain, at, 0);
        assert_eq!(world.terrain().height_at(at), 105);
        for n in world.terrain().neighbours(at).into_iter().flatten() {
            assert_eq!(world.terrain().height_at(n), 105, "{n:?}");
        }
        assert!(world.sites().next().is_none());
    }

    // Only owned fields take buildings and flags.
    // spec: ECO-PLACE-2.1, ECO-PLACE-3.1
    #[test]
    fn placing_outside_the_territory_is_refused() {
        let mut world = game(flat(60, 50), (27, 21), 0);
        assert_eq!(flag(&mut world, (45, 21)), Err(crate::Rejection::NotOwned));
        assert_eq!(flag(&mut world, (33, 21)), Ok(()));
    }

    // ECO-HQ T13: distance 3 from player 0's (range 8) against 5 from player
    // 1's: 16/9 against 8/25, player 0. T14: within 2 of both, the first
    // looked at.
    // spec: ECO-HQ-3.5, TER-3.1 (T13, T14)
    #[test]
    fn t13_t14_owner_by_strength() {
        let p1 = PlayerId(1);
        let a = (20, 20);
        let b = (28, 20);
        let field = (23, 20);
        assert_eq!(grid::distance(a, field), 3);
        assert_eq!(grid::distance(b, field), 5);
        let claimants = [(a, P0, 8), (b, p1, 8)];
        assert_eq!(owner_by_strength(&claimants, field), Some(P0));
        let near = [((26, 20), p1, 8), ((22, 20), P0, 8)];
        assert_eq!(owner_by_strength(&near, (24, 20)), Some(p1));
        // Equal strength: the lower player number.
        let equal = [((14, 20), p1, 8), ((26, 20), P0, 8)];
        assert_eq!(owner_by_strength(&equal, (20, 20)), Some(P0));
        // Nobody reaches it.
        assert_eq!(owner_by_strength(&[(a, P0, 3)], (40, 20)), None);
    }

    // ECO-HQ-3.5 at game start: two headquarters close together share the
    // land between them by strength; the second claim takes fields from the
    // first and says "Lost Territory.".
    #[test]
    fn overlapping_headquarters() {
        let setup = GameSetup {
            players: vec![(P0, (20, 20)), (PlayerId(1), (32, 20))],
            start_resources: 0,
            local_player: P0,
            ..Default::default()
        };
        let world = World::new_game(3, flat(60, 50), rules(), &setup);
        // Halfway (26, 20) is 6 from both: equal, player 0.
        assert_eq!(world.field_owner((26, 20)), Some(P0));
        assert_eq!(world.field_owner((27, 20)), Some(PlayerId(1)));
        assert_eq!(world.field_owner((25, 20)), Some(P0));
        let texts: Vec<_> = world
            .messages()
            .iter()
            .map(|m| (m.player, m.text))
            .collect();
        assert_eq!(texts, [(P0, "Lost Territory.")]);
    }
}
