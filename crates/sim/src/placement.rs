//! Where buildings and flags may be placed (`specs/ECO-building-placement.md`).

use crate::grid::{self, Point};
use crate::map::state;
use crate::{Building, PlayerId, Rejection, World, NOT_GENERAL};

/// ECO-PLACE-6.2: the military clearance, in fields: no military type
/// within this distance of another military building or site.
pub const MILITARY_CLEARANCE: u32 = 4;

/// Plot size class of a building type (`bp_setSize`, ECO-PLACE rules intro).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildingSize {
    Small,
    Medium,
    Large,
    Mine,
    Harbor,
}

/// What the simulation needs to know about a building type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingType {
    /// The script id (`bp_create`).
    pub id: u32,
    pub size: BuildingSize,
    /// ECO-HQ-4.1: frame materials (`bp_addFrameCost`) and main materials
    /// (`bp_addCost`), as (good, count) in script order.
    pub frame_costs: Vec<(u32, u32)>,
    pub costs: Vec<(u32, u32)>,
    /// ECO-HQ-3.1, 3.3: fields claimed and uncovered; 0 for none.
    pub territory_range: u32,
    pub explore_radius: u32,
    /// MIL-SOL-2.3: `bp_setSoldierLimit`, the most soldiers it asks for; 0
    /// for none.
    pub soldier_limit: u32,
    /// ECO-REQ-1.4: the worker profession (`bp_setWorker`) and how many
    /// (`bp_setWorkerLimit`).
    pub worker: Option<u32>,
    pub worker_limit: u32,
    /// ECO-REQ-3.2: a warehouse once finished (`bp_setDepot`).
    pub warehouse: bool,
    /// ECO-REQ-4.1: construction time 0 or less, finished at once.
    pub instant: bool,
    /// `bp_setGoods`: the product.
    pub product: Option<u32>,
    /// ECO-WORK-3.2, ECO-OUT-3.5: ticks of the work action.
    pub production_ticks: u32,
    /// ECO-REQ-4.2: `bp_setStockLimit`, unset for none.
    pub stock_limit: Option<u32>,
    /// ECO-OUT-1.1: `bp_setWorkingRadius`, default 4.
    pub working_radius: u32,
    pub cycle: crate::WorkCycle,
    /// ECO-PROD-5.1: `bp_setSecondGoods`, the product of slot 1.
    pub second_product: Option<u32>,
    /// `bp_setSwitchableProduction` (ECO-PROD-1.3: only the shipyard).
    pub switchable: bool,
    /// `bp_setCatapult`: orders stone up to the stock limit (ECO-PROD-6.1).
    pub catapult: bool,
    /// `bp_setPlantingGood`: what a forester or farm plants (ECO-OUT-4,
    /// ECO-FOOD-3.3).
    pub planting_good: Option<u32>,
    /// `bp_setSpawn`: a warehouse that makes new helpers (ECO-PROF-3.1).
    pub spawn: bool,
    /// TRN-FREE-5.5a: `bp_setConstructionPosition` offsets per tribe
    /// (Romans, Africans, Chinese) and position index, in height samples
    /// from the building's flag field, already truncated to whole numbers.
    pub construction_positions: [[(i32, i32); 4]; 3],
    /// MIL-UPG-1.1: the next type of the upgrade chain (`mi_addBuildingUpgrade`).
    pub upgrade_to: Option<u32>,
    /// ECO-CAT-2.4, ECO-WORK-3.8a: `bp_setDoorOffsetDirection` per tribe
    /// (Romans, Africans, Chinese), a direction of MAP-GRID-1.2; `None`
    /// for a building whose door field is its own field.
    pub doors: [Option<u8>; 3],
}

impl BuildingType {
    /// A type of this size without costs, territory or exploration.
    pub fn new(id: u32, size: BuildingSize) -> Self {
        Self {
            id,
            size,
            frame_costs: Vec::new(),
            costs: Vec::new(),
            territory_range: 0,
            explore_radius: 0,
            soldier_limit: 0,
            worker: None,
            worker_limit: 1,
            warehouse: false,
            instant: false,
            product: None,
            production_ticks: 0,
            stock_limit: None,
            working_radius: 4,
            cycle: crate::WorkCycle::None,
            second_product: None,
            switchable: false,
            catapult: false,
            planting_good: None,
            spawn: false,
            construction_positions: [[(0, 0); 4]; 3],
            upgrade_to: None,
            doors: [None; 3],
        }
    }
}

/// ECO-PLACE-4.1: the best thing that can be placed on a field. The numbers
/// are those the spec's test cases use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Placement {
    Nothing = 0,
    Flag = 1,
    Small = 2,
    Medium = 3,
    Large = 4,
    Mine = 5,
    Harbor = 6,
}

/// ECO-PLACE-4.1: the order in which sizes are tried.
const CLASS_ORDER: [(BuildingSize, Placement); 5] = [
    (BuildingSize::Harbor, Placement::Harbor),
    (BuildingSize::Large, Placement::Large),
    (BuildingSize::Medium, Placement::Medium),
    (BuildingSize::Small, Placement::Small),
    (BuildingSize::Mine, Placement::Mine),
];

/// MAP-GRID-1.6 direction numbers used here.
const SOUTH_EAST: u8 = 1;
const LARGE_FOOTPRINT: [u8; 3] = [3, 4, 5];

/// ECO-PLACE-3.4, 3.10: the largest allowed height class to the mean.
const MAX_SLOPE_CLASS: i8 = 3;

/// ECO-PLACE-3.8
const HARBOR_WATER_DISTANCE: u32 = 3;
const HARBOR_WATER_FIELDS: usize = 3;

/// ECO-PLACE-3.9, 3.10: radius 2 has 1 + 6 + 12 fields.
const RADIUS_2_FIELDS: usize = 19;

/// ECO-PLACE-4.2: a change on a field can alter the class of fields up to this
/// far away. A building candidate looks at most two fields from its own
/// field (its flag's neighbours, its footprint's fields, radius 2 for slope
/// and other buildings), and the water ring of a harbour does not change
/// with objects.
const RECOMPUTE_RADIUS: u32 = 2;

impl World {
    /// ECO-PLACE-2.1
    pub(crate) fn is_general_site(&self, player: PlayerId, field: Point) -> bool {
        self.general_site_in(player, field, false)
    }

    /// `owns`, or, when `free_ok`, also unowned (SHIP-6.1: a harbour site
    /// "for nobody or for the player").
    fn owns_in(&self, player: PlayerId, field: Point, free_ok: bool) -> bool {
        match self.field_owner(field) {
            Some(owner) => owner == player,
            None => free_ok,
        }
    }

    fn general_site_in(&self, player: PlayerId, field: Point, free_ok: bool) -> bool {
        self.owns_in(player, field, free_ok) && !self.terrain.has(field, NOT_GENERAL)
    }

    /// ECO-PLACE-2.2
    pub fn check_flag(&self, player: PlayerId, field: Point) -> Result<(), Rejection> {
        self.check_flag_in(player, field, false)
    }

    fn check_flag_in(
        &self,
        player: PlayerId,
        field: Point,
        free_ok: bool,
    ) -> Result<(), Rejection> {
        if !self.terrain.contains(field) {
            return Err(Rejection::OutsideMap);
        }
        if !self.owns_in(player, field, free_ok) {
            return Err(Rejection::NotOwned);
        }
        if !self.general_site_in(player, field, free_ok) {
            return Err(Rejection::NotGeneralSite);
        }
        let near = std::iter::once(Some(field)).chain(self.terrain.neighbours(field));
        if near.flatten().any(|f| self.flag_at(f).is_some()) {
            return Err(Rejection::FlagTooClose);
        }
        Ok(())
    }

    /// ECO-PLACE-3.6: the building's own field, and for large buildings and
    /// harbours its west, north-west and north-east neighbours. `None` for a
    /// footprint field outside the map.
    fn footprint(&self, at: Point, size: BuildingSize) -> Vec<Option<Point>> {
        let mut fields = vec![Some(at)];
        if matches!(size, BuildingSize::Large | BuildingSize::Harbor) {
            fields.extend(LARGE_FOOTPRINT.map(|d| self.terrain.neighbour(at, d)));
        }
        fields
    }

    /// The footprint fields of a building of `size` on `at` that are inside
    /// the map.
    pub(crate) fn footprint_of(&self, at: Point, size: BuildingSize) -> Vec<Point> {
        self.footprint(at, size).into_iter().flatten().collect()
    }

    /// ECO-PLACE-6.1: a building type counts as military for the clearance
    /// when its territory range is above 0 (the headquarters, barracks,
    /// guard house, watchtower and stronghold; never the lookout tower, the
    /// catapult, the storehouse, the harbour or a workshop).
    pub fn counts_as_military(&self, building_type: u32) -> bool {
        self.building_types
            .get(&building_type)
            .is_some_and(|t| t.territory_range > 0)
    }

    /// ECO-PLACE-6.2: a building or construction site of any player whose
    /// type counts as military (6.1) stands within distance
    /// [`MILITARY_CLEARANCE`] of `at` (rings 0 to 4, `at` included; each
    /// building on its own field only; fields outside the map skipped).
    /// A military type may not be placed on such a field (6.3).
    pub fn military_near(&self, at: Point) -> bool {
        grid::area(
            at,
            MILITARY_CLEARANCE,
            self.terrain.width,
            self.terrain.height,
        )
        .into_iter()
        .any(|f| {
            self.building_at(f)
                .is_some_and(|b| self.counts_as_military(b.building_type))
        })
    }

    /// ECO-PLACE-3: whether a building of `size` fits on `at`, checked in the
    /// spec's order; the first failed rule is the reason.
    pub fn check_building(
        &self,
        player: PlayerId,
        at: Point,
        size: BuildingSize,
    ) -> Result<(), Rejection> {
        self.check_building_in(player, at, size, false, None)
    }

    /// SHIP-6.1: whether a harbour could be built on `at` by `player` if
    /// the land around it were unowned or his.
    pub fn harbour_site_for(&self, player: PlayerId, at: Point) -> bool {
        self.check_building_in(player, at, BuildingSize::Harbor, true, None)
            .is_ok()
    }

    /// MIL-UPG-1.2: whether a building of `size` would fit on `at` with the
    /// building standing there ignored.
    pub(crate) fn check_building_replacing(
        &self,
        player: PlayerId,
        at: Point,
        size: BuildingSize,
    ) -> Result<(), Rejection> {
        self.check_building_in(player, at, size, false, Some(at))
    }

    /// `ignore`: a building's own field, whose Blocked bit, building and
    /// General-site bits are not held against it (MIL-UPG-1.2).
    fn check_building_in(
        &self,
        player: PlayerId,
        at: Point,
        size: BuildingSize,
        free_ok: bool,
        ignore: Option<Point>,
    ) -> Result<(), Rejection> {
        let terrain = &self.terrain;
        if !terrain.contains(at) {
            return Err(Rejection::OutsideMap);
        }
        // 3.1
        if !self.owns_in(player, at, free_ok) {
            return Err(Rejection::NotOwned);
        }
        // 3.2
        let flag = terrain
            .neighbour(at, SOUTH_EAST)
            .ok_or(Rejection::FlagFieldUnusable)?;
        let ground = if size == BuildingSize::Mine {
            state::FOR_MINING
        } else {
            state::FOR_BUILDING
        };
        if !terrain.has(flag, ground) {
            return Err(Rejection::FlagFieldUnusable);
        }
        match self.flag_at(flag) {
            Some(owner) if owner != player => return Err(Rejection::FlagFieldUnusable),
            Some(_) => {}
            None => self
                .check_flag_in(player, flag, free_ok)
                .map_err(|_| Rejection::FlagFieldUnusable)?,
        }
        // 3.3: fields outside the map are ignored.
        let near = std::iter::once(Some(at)).chain(terrain.neighbours(at));
        if near
            .flatten()
            .any(|f| Some(f) != ignore && terrain.has(f, state::BLOCKED))
        {
            return Err(Rejection::TooCloseToBuilding);
        }
        // 3.4
        if matches!(size, BuildingSize::Small | BuildingSize::Mine) {
            let radius = if size == BuildingSize::Small { 2 } else { 1 };
            let mean = terrain.mean_height(at, radius);
            let steep = terrain
                .neighbours(at)
                .into_iter()
                .flatten()
                .any(|f| !terrain.has(f, state::WATER) && !self.gentle(f, mean));
            if steep {
                return Err(Rejection::TooSteep);
            }
        }
        // 3.5
        if size == BuildingSize::Mine {
            let mining = self.general_site_in(player, at, free_ok)
                && !terrain.has(at, state::STREET)
                && terrain.has(at, state::FOR_MINING);
            return if mining {
                Ok(())
            } else {
                Err(Rejection::NotMiningGround)
            };
        }
        // 3.6
        for field in self.footprint(at, size) {
            let usable = field.is_some_and(|f| {
                self.flag_at(f).is_none()
                    && self.owns_in(player, f, free_ok)
                    && !terrain.has(
                        f,
                        if Some(f) == ignore {
                            NOT_GENERAL & !(state::BLOCKED | state::MOVEMENT_BLOCKED)
                        } else {
                            NOT_GENERAL
                        } | state::STREET,
                    )
                    && terrain.has(f, state::FOR_BUILDING)
                    && (size != BuildingSize::Harbor || terrain.has(f, state::FOR_HARBOR))
            });
            if !usable {
                return Err(Rejection::FootprintUnusable);
            }
        }
        // 3.7
        if size == BuildingSize::Small {
            return Ok(());
        }
        // 3.8
        if size == BuildingSize::Harbor {
            let open_water = grid::ring(at, HARBOR_WATER_DISTANCE, terrain.width, terrain.height)
                .into_iter()
                .filter(|&f| {
                    terrain.has(f, state::WATER)
                        && !terrain.has(f, state::DOODAD_BLOCKED | state::LANDING_POSITION)
                })
                .count();
            if open_water < HARBOR_WATER_FIELDS {
                return Err(Rejection::NoWaterForHarbor);
            }
        }
        let radius_2 = grid::area(at, 2, terrain.width, terrain.height);
        // 3.9
        if matches!(size, BuildingSize::Large | BuildingSize::Harbor) {
            let room = radius_2.len() == RADIUS_2_FIELDS
                && radius_2
                    .iter()
                    .all(|&f| Some(f) == ignore || self.building_at(f).is_none());
            if !room {
                return Err(Rejection::NoRoomForLargeBuilding);
            }
        }
        // 3.10
        if radius_2.len() != RADIUS_2_FIELDS {
            return Err(Rejection::TooCloseToEdge);
        }
        let mean = terrain.mean_height(at, 2);
        let steep = radius_2[1..].iter().any(|&f| {
            let skipped = size == BuildingSize::Harbor && terrain.has(f, state::WATER);
            !skipped && !self.gentle(f, mean)
        });
        if steep {
            return Err(Rejection::TooSteep);
        }
        Ok(())
    }

    /// ECO-PLACE-3.4, 3.10: the field's height class to the mean is within
    /// -3 .. 3.
    fn gentle(&self, field: Point, mean: i64) -> bool {
        let difference = i64::from(self.terrain.height_at(field)) - mean;
        grid::height_class(difference).abs() <= MAX_SLOPE_CLASS
    }

    /// ECO-PLACE-4.1
    pub fn compute_placement(&self, player: PlayerId, field: Point) -> Placement {
        for (size, class) in CLASS_ORDER {
            if self.check_building(player, field, size).is_ok() {
                return class;
            }
        }
        if self.check_flag(player, field).is_ok() {
            Placement::Flag
        } else {
            Placement::Nothing
        }
    }

    pub(crate) fn recompute_all_placement(&mut self) {
        let classes: Vec<Placement> = self
            .terrain
            .fields()
            .map(|f| self.compute_placement(self.local_player, f))
            .collect();
        self.placement = classes;
    }

    /// ECO-PLACE-4.2: recompute the classes around changed fields.
    pub(crate) fn changed(&mut self, fields: &[Point]) {
        let (width, height) = (self.terrain.width, self.terrain.height);
        let mut around: Vec<Point> = fields
            .iter()
            .flat_map(|&f| grid::area(f, RECOMPUTE_RADIUS, width, height))
            .collect();
        around.sort_unstable();
        around.dedup();
        for field in around {
            let class = self.compute_placement(self.local_player, field);
            let index = self.terrain.index(field);
            self.placement[index] = class;
        }
    }

    /// Put a flag on a field already checked; on a road it splits the road
    /// (TRN-ROAD-4.2).
    pub(crate) fn put_flag(&mut self, player: PlayerId, field: Point) {
        let index = self.terrain.index(field);
        // SIM-START-4.4 j: on a road, the road is removed first.
        let split = self.streets[index].and_then(|id| self.take_split_road(id));
        // SIM-START-4.2: the flag's own number, then one for each of its
        // three network entries (carrier, donkey, boat; SIM-START-4.3).
        for _ in 0..4 {
            self.new_number();
        }
        self.flags[index] = Some(player);
        self.flag_order.push(field);
        // GFX-OBJ-1.4a (iv): a flag on the field removes its doodads.
        self.remove_doodads(field, crate::DOODADS_ALL);
        if let Some((id, road)) = split {
            self.split_road(id, road, field);
        }
    }

    pub(crate) fn place_flag(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        self.check_flag(player, at)?;
        self.put_flag(player, at);
        // ECO-HQ-4.8b: the flag's height ring.
        let touched = self.flag_height_ring(at);
        self.changed(&touched);
        self.changed(&[at]);
        Ok(())
    }

    /// The field of the flag a building on `at` would make (ECO-PLACE-5.1),
    /// when there is none yet.
    pub(crate) fn new_flag_of(&self, at: Point) -> Option<Point> {
        self.terrain
            .neighbour(at, SOUTH_EAST)
            .filter(|&f| self.flag_at(f).is_none())
    }

    /// ECO-PLACE-5.1; `records` numbers follow the building's own
    /// (SIM-START-4.4 b: a computer player's records of its site).
    pub(crate) fn place_building(
        &mut self,
        player: PlayerId,
        building_type: u32,
        at: Point,
        records: u32,
    ) -> Result<(), Rejection> {
        let size = self
            .building_type(building_type)
            .ok_or(Rejection::UnknownBuildingType)?
            .size;
        // ECO-PLACE-6.3: the military clearance first, before every other
        // placement rule (the build menu greys the cell the same way).
        if self.counts_as_military(building_type) && self.military_near(at) {
            return Err(Rejection::TooCloseToMilitary);
        }
        // CAM-1.3: not a type the mission has locked for the player.
        if self.is_locked(player, building_type) {
            return Err(Rejection::BuildingLocked);
        }
        self.check_building(player, at, size)?;
        let new_flag = self.new_flag_of(at);
        let mut touched =
            self.put_building_numbered(player, building_type, size, at, true, records);
        // ECO-HQ-4.8a: the site's field takes the mean height around it;
        // 4.8b: then a new flag's height ring.
        touched.extend(self.level_site_height(at));
        if let Some(flag) = new_flag {
            touched.extend(self.flag_height_ring(flag));
        }
        // ECO-HQ-4.1: a placed building starts as a construction site.
        self.start_construction(at);
        self.changed(&touched);
        Ok(())
    }

    /// ECO-PLACE-5.1: the building's object bits, its record and its flag
    /// (south-east, when that field is inside the map and has none). Returns
    /// the fields whose state changed.
    pub(crate) fn put_building(
        &mut self,
        player: PlayerId,
        building_type: u32,
        size: BuildingSize,
        at: Point,
        under_construction: bool,
    ) -> Vec<Point> {
        self.put_building_numbered(player, building_type, size, at, under_construction, 0)
    }

    /// [`World::put_building`] with `records` object numbers taken right
    /// after the building's own (SIM-START-4.4 b).
    fn put_building_numbered(
        &mut self,
        player: PlayerId,
        building_type: u32,
        size: BuildingSize,
        at: Point,
        under_construction: bool,
        records: u32,
    ) -> Vec<Point> {
        let footprint: Vec<Point> = self.footprint(at, size).into_iter().flatten().collect();
        self.set_bits(at, state::BLOCKED, true);
        // GFX-OBJ-1.4a (iv): the building on its field removes its doodads.
        self.remove_doodads(at, crate::DOODADS_ALL);
        for &field in &footprint {
            self.set_bits(field, state::MOVEMENT_BLOCKED, true);
        }
        let index = self.terrain.index(at);
        // SIM-START-4.2: a building takes the next number; 4.4 b: a
        // computer player's records of it the numbers after it.
        let serial = self.new_number();
        for _ in 0..records {
            self.new_number();
        }
        if let Err(i) = self.building_index.binary_search(&(index as u32)) {
            self.building_index.insert(i, index as u32);
        }
        // ECO-OUT-3.11 (T26): a type with a product has its value, 0.5,
        // from its site's placement on; any other none (-1) yet.
        let productivity = match self.building_types.get(&building_type) {
            Some(t) if t.product.is_some() => crate::production::PRODUCTIVITY_FIRST,
            _ => -100,
        };
        self.buildings[index] = Some(Building {
            owner: player,
            building_type,
            size,
            at,
            under_construction,
            serial,
            workers: 0,
            timer: 0,
            active: false,
            productivity,
            productivity_bits: if productivity < 0 {
                (-1.0f32).to_bits()
            } else {
                crate::production::STAT_FIRST.to_bits()
            },
            production: crate::Production::new(),
        });
        // ECO-FOOD-5.16 (3ede553): the animals registered on the footprint
        // (the building's field, then for a large building or a harbour its
        // west, north-west and north-east neighbours) are removed, without a
        // draw; the flag's field is not part of it.
        for &field in &footprint {
            let under: Vec<usize> = self
                .animals()
                .filter(|(_, a)| a.at == field)
                .map(|(id, _)| id)
                .collect();
            for id in under {
                self.remove_animal(id);
            }
        }
        let mut touched = footprint;
        if let Some(flag) = self.terrain.neighbour(at, SOUTH_EAST) {
            if self.flag_at(flag).is_none() {
                self.put_flag(player, flag);
            }
            touched.push(flag);
        }
        touched
    }

    /// ECO-PLACE-5.1: removing a building clears Blocked and MovementBlocked
    /// again. Blocked returns to the terrain's value, which was clear: a
    /// building can only be placed on a field that is not Blocked
    /// (ECO-PLACE-3.3). The flag stays. With remains (MIL-CON-4.1b: the
    /// player's demolish too) a finished building's ruin marks its field
    /// while it burns (MIL-CON-4.1a).
    pub(crate) fn remove_building(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        self.building_at(at)
            .filter(|b| b.owner == player)
            .ok_or(Rejection::NoBuilding)?;
        // MIL-CON-4.1b (Q-150, T17a): the player's demolish removes with
        // remains, as the computer player's tear-down: material on the
        // flag (4.4), a ruin for a finished building; the flag stays.
        self.remove_with_remains(at);
        Ok(())
    }

    /// AI-CON-9.3, MIL-CON-4.1 (T17): the computer player's demolish
    /// removes its building with remains.
    pub(crate) fn tear_down(&mut self, player: PlayerId, at: Point) -> Result<(), Rejection> {
        self.building_at(at)
            .filter(|b| b.owner == player)
            .ok_or(Rejection::NoBuilding)?;
        self.remove_with_remains(at);
        Ok(())
    }

    /// The building on `at` is gone (removed by its player, TER-6.2, or
    /// destroyed): its field and footprint are cleared, its workers and
    /// soldiers go home, and an applied military building releases its
    /// land (TER-4.1).
    pub(crate) fn delete_building(&mut self, at: Point) {
        let building = *self.building_at(at).expect("a building to delete");
        // What lies in it is lost with it: its stock, its inputs, the
        // materials on its site.
        let lost: Vec<(u32, u32)> = [
            self.warehouses.get(&at).map(|w| w.stock.as_slice()),
            self.inputs.get(&at).map(Vec::as_slice),
            self.sites.get(&at).map(|s| s.on_site.as_slice()),
        ]
        .into_iter()
        .flatten()
        .flatten()
        .copied()
        .collect();
        for (good, n) in lost {
            self.used(good, n);
        }
        let footprint: Vec<Point> = self
            .footprint(at, building.size)
            .into_iter()
            .flatten()
            .collect();
        self.set_bits(at, state::BLOCKED, false);
        for &field in &footprint {
            self.set_bits(field, state::MOVEMENT_BLOCKED, false);
        }
        let index = self.terrain.index(at);
        // TRN-FREE-6.4b: where a removed site's builder stands, and the
        // ticks his work action has left (it runs on into T).
        let builder_pose = self.builder_pose(at);
        let builder_stand = builder_pose.map(|pose| pose.sample());
        // T33: an approach walk under way is walked to its end.
        let builder_approach = self.approach_left(at);
        // T34 (Q-286): a work walk under way is walked on to its position.
        let builder_walk = self.work_walk_left(at);
        let builder_work = match builder_pose {
            Some(crate::construction::BuilderPose::Working { since, .. }) => {
                self.settings.construct_ticks.saturating_sub(since)
            }
            _ => 0,
        };
        self.buildings[index] = None;
        if let Ok(i) = self.building_index.binary_search(&(index as u32)) {
            self.building_index.remove(i);
        }
        let site = self.sites.remove(&at);
        self.upgrades.remove(&at);
        self.builder_links.remove(&at);
        self.warehouses.remove(&at);
        self.harbour_removed(at);
        // Its leveller, builder, workers and soldiers go home (ECO-HQ-5.1);
        // its reservation ends (ECO-OUT-2.6).
        self.evict(
            &building,
            site.as_ref(),
            builder_stand,
            builder_work,
            builder_approach,
            builder_walk,
        );
        self.release_target(at);
        // W20: a later building on the field starts with the first recipe.
        self.chosen_recipes.remove(&at);
        // W15: a later mine on the field starts undeepened.
        self.mine_deepenings.remove(&at);
        if let Some(military) = self.military.remove(&at) {
            // Coins in its store are gone with it (MIL-PROM-3).
            self.inputs.remove(&at);
            let range = self.building_types[&building.building_type].territory_range;
            if military.applied && range > 0 {
                self.release_territory(at, range);
            }
        }
        self.changed(&footprint);
    }
}

impl World {
    /// ECO-WORK-3.8a, MIL-SOL-6.5: the door field of the building on `at`:
    /// its field stepped once in its door offset direction (ECO-CAT-2.4,
    /// by the owner's tribe), or its own field when it has none.
    pub(crate) fn door_field(&self, at: Point) -> Point {
        let Some(building) = self.building_at(at) else {
            return at;
        };
        let tribe = self.tribe(building.owner);
        self.building_types
            .get(&building.building_type)
            .and_then(|t| t.doors.get(usize::from(tribe)).copied().flatten())
            .and_then(|d| self.terrain.neighbour(at, d))
            .unwrap_or(at)
    }

    /// ECO-WORK-3.8a to 3.8c: a walker leaving the building on `at` along
    /// `walk` (which starts with the building's flag): it appears on the
    /// door field and, when that is not the building's field, first steps
    /// back onto the building's field as a walk of its own; from there to
    /// the flag is a walk, and the road walk starts on the flag; each starts
    /// at progress 0.
    pub(crate) fn walker_leaving(&self, at: Point, walk: Vec<Point>) -> crate::movement::Walker {
        let door = self.door_field(at);
        let mut walker = crate::movement::Walker::new(door);
        let start = (door != at).then_some(at);
        let flag = walk.first().copied().filter(|_| walk.len() > 1);
        walker.walk(start.into_iter().chain(walk));
        walker.pauses = start.into_iter().chain(flag).collect();
        walker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{Command, Rejection};

    fn class(world: &World, field: Point) -> Placement {
        world.placement_at(field)
    }

    // spec: ECO-PLACE-3.9
    #[test]
    fn flat_building_ground_fits_large_away_from_edges() {
        let world = world(flat(12, 12));
        assert_eq!(class(&world, (5, 5)), Placement::Large);
        // ECO-PLACE-3.9 / 3.10: radius 2 must lie inside the map.
        assert_eq!(class(&world, (1, 5)), Placement::Small);
        // ECO-PLACE-3.2: no south-east neighbour on the last row... row 0 is
        // the south edge, so the flag of a field on row 0 would be outside.
        assert_eq!(class(&world, (5, 0)), Placement::Flag);
        assert_eq!(
            world.check_building(P0, (5, 0), BuildingSize::Small),
            Err(Rejection::FlagFieldUnusable)
        );
    }

    // ECO-PLACE-2.3: a flag does not need building ground.
    #[test]
    fn flags_need_no_building_ground() {
        let mut terrain = flat(8, 8);
        set(&mut terrain, (3, 3), 0);
        let mut world = world(terrain);
        assert_eq!(class(&world, (3, 3)), Placement::Flag);
        assert_eq!(flag(&mut world, (3, 3)), Ok(()));
    }

    // ECO-PLACE-2.1
    #[test]
    fn general_site_bits_stop_flags() {
        for bit in [
            state::BORDER,
            state::WATER,
            state::ELEVATION_BLOCKED,
            state::DOODAD_BLOCKED,
            state::BLOCKED,
            state::DEPOSIT_BLOCKED,
            state::MOVEMENT_BLOCKED,
        ] {
            let mut terrain = flat(8, 8);
            set(&mut terrain, (3, 3), state::FOR_BUILDING | bit);
            let mut world = world(terrain);
            assert_eq!(
                flag(&mut world, (3, 3)),
                Err(Rejection::NotGeneralSite),
                "{bit:#x}"
            );
            assert_eq!(class(&world, (3, 3)), Placement::Nothing, "{bit:#x}");
        }
    }

    // ECO T15, ECO-PLACE-2.2
    #[test]
    fn flags_keep_one_field_apart() {
        let mut world = world(flat(10, 10));
        assert_eq!(flag(&mut world, (4, 4)), Ok(()));
        for neighbour in world.terrain.neighbours((4, 4)).into_iter().flatten() {
            assert_eq!(
                world.check_flag(P0, neighbour),
                Err(Rejection::FlagTooClose),
                "{neighbour:?}"
            );
        }
        assert_eq!(flag(&mut world, (4, 4)), Err(Rejection::FlagTooClose));
        assert_eq!(flag(&mut world, (6, 4)), Ok(()));
    }

    // ECO T14, ECO-PLACE-3.3
    #[test]
    fn buildings_do_not_touch() {
        let mut world = world(flat(14, 14));
        assert_eq!(building(&mut world, SMALL, (6, 6)), Ok(()));
        for neighbour in world.terrain.neighbours((6, 6)).into_iter().flatten() {
            for size in [
                BuildingSize::Small,
                BuildingSize::Medium,
                BuildingSize::Mine,
            ] {
                assert!(
                    world.check_building(P0, neighbour, size).is_err(),
                    "{neighbour:?}"
                );
            }
        }
        // A flag or road next to it is still possible.
        let beside = (5, 6);
        assert_eq!(world.check_flag(P0, beside), Ok(()));
    }

    // ECO-PLACE-5.1
    #[test]
    fn placing_and_removing_a_large_building() {
        let mut world = world(flat(14, 14));
        let at = (6, 6);
        assert_eq!(building(&mut world, LARGE, at), Ok(()));
        let flag_field = world.terrain.neighbour(at, 1).unwrap();
        assert_eq!(world.flag_at(flag_field), Some(P0));
        assert!(world.terrain.has(at, state::BLOCKED));
        let footprint: Vec<Point> = world
            .footprint(at, BuildingSize::Large)
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(footprint.len(), 4);
        for &f in &footprint {
            assert!(world.terrain.has(f, state::MOVEMENT_BLOCKED), "{f:?}");
            assert_eq!(class(&world, f), Placement::Nothing);
        }
        assert_eq!(
            apply(
                &mut world,
                Command::RemoveBuilding {
                    player: P0,
                    at: (1, 1)
                }
            ),
            Err(Rejection::NoBuilding)
        );
        assert_eq!(
            apply(&mut world, Command::RemoveBuilding { player: P0, at }),
            Ok(())
        );
        assert!(!world.terrain.has(at, state::BLOCKED));
        assert!(footprint
            .iter()
            .all(|&f| !world.terrain.has(f, state::MOVEMENT_BLOCKED)));
        assert!(world.building_at(at).is_none());
        // The flag stays.
        assert_eq!(world.flag_at(flag_field), Some(P0));
    }

    // ECO-PLACE-3.2: the flag field needs ForMining for a mine.
    #[test]
    fn mines_need_mining_ground_and_a_mining_flag_field() {
        let mut terrain = flat(10, 10);
        for field in terrain.fields().collect::<Vec<_>>() {
            set(&mut terrain, field, state::FOR_MINING);
        }
        let mut world = world(terrain.clone());
        assert_eq!(class(&world, (4, 4)), Placement::Mine);
        assert_eq!(building(&mut world, MINE, (4, 4)), Ok(()));

        // Building ground under the flag field: no mine.
        let flag_field = terrain.neighbour((4, 4), 1).unwrap();
        set(&mut terrain, flag_field, state::FOR_BUILDING);
        let world = world_with(terrain);
        assert_eq!(
            world.check_building(P0, (4, 4), BuildingSize::Mine),
            Err(Rejection::FlagFieldUnusable)
        );
    }

    fn world_with(terrain: crate::map::Terrain) -> World {
        crate::test_support::world(terrain)
    }

    // ECO-PLACE-3.4: a small building compares its neighbours with the
    // integer mean of radius 2; a difference above 2400 is too steep. One
    // raised field among 19 at height 0 raises the mean by height / 19.
    #[test]
    fn slope_limits() {
        let building_at = (5, 6);
        let raised = (6, 6);
        for (height, expected) in [(2533, Ok(())), (2534, Err(Rejection::TooSteep))] {
            let mut terrain = flat(12, 12);
            set_height(&mut terrain, raised, height);
            let world = world(terrain);
            // 2533 - 2533 / 19 = 2400; 2534 - 2534 / 19 = 2401.
            assert_eq!(
                world.check_building(P0, building_at, BuildingSize::Small),
                expected,
                "{height}"
            );
        }
    }

    // ECO-PLACE-3.8: a harbour needs harbour ground and water at distance 3.
    #[test]
    fn harbor_needs_water_at_distance_three() {
        let mut terrain = flat(16, 16);
        let at = (7, 8);
        for field in terrain.fields().collect::<Vec<_>>() {
            if crate::grid::distance(at, field) <= 1 {
                set(&mut terrain, field, state::FOR_BUILDING | state::FOR_HARBOR);
            }
        }
        assert_eq!(
            world(terrain.clone()).check_building(P0, at, BuildingSize::Harbor),
            Err(Rejection::NoWaterForHarbor)
        );
        for field in crate::grid::ring(at, 3, 16, 16).into_iter().take(3) {
            set(&mut terrain, field, state::WATER);
            set_height(&mut terrain, field, -500);
        }
        let world = world(terrain);
        assert_eq!(world.check_building(P0, at, BuildingSize::Harbor), Ok(()));
        assert_eq!(class(&world, at), Placement::Harbor);
    }

    /// The incrementally kept classes always equal a full recomputation
    /// (ECO-PLACE-4.2), whatever commands were applied, and no command panics.
    #[test]
    fn incremental_classes_equal_full_recomputation() {
        let mut terrain = flat(20, 18);
        for (i, field) in terrain.fields().collect::<Vec<_>>().into_iter().enumerate() {
            set_height(&mut terrain, field, ((i * 7919) % 2900) as i32 - 400);
            if i % 23 == 0 {
                set(&mut terrain, field, state::FOR_MINING);
            }
            if i % 31 == 0 {
                set(
                    &mut terrain,
                    field,
                    state::FOR_BUILDING | state::DEPOSIT_BLOCKED,
                );
            }
        }
        let mut world = world(terrain);
        let mut rng = crate::Rng::new(99);
        let mut accepted = 0;
        for _ in 0..600 {
            // Two draws make the 62 bits the fields below take apart.
            let r = (u64::from(rng.draw()) << 31) | u64::from(rng.draw());
            let field = |shift: u32| (((r >> shift) % 22) as u32, ((r >> (shift + 8)) % 20) as u32);
            let command = match r % 5 {
                0 | 1 => Command::PlaceFlag {
                    player: P0,
                    at: field(8),
                },
                2 => Command::BuildRoad {
                    player: P0,
                    from: field(8),
                    to: field(24),
                },
                3 => Command::PlaceBuilding {
                    player: P0,
                    building: (r >> 40) as u32 % 7,
                    at: field(8),
                },
                _ => Command::RemoveBuilding {
                    player: P0,
                    at: field(8),
                },
            };
            world.submit(command);
            world.step();
            accepted += usize::from(world.last_results()[0].outcome.is_ok());
            let mut full = world.clone();
            full.recompute_all_placement();
            assert_eq!(world.placement, full.placement);
        }
        assert!(accepted > 50, "only {accepted} commands were accepted");
    }

    // spec: ECO-PLACE-3.1
    #[test]
    fn other_players_and_unknown_types_are_rejected() {
        let mut world = world(flat(10, 10));
        let other = crate::PlayerId(1);
        assert_eq!(
            apply(
                &mut world,
                Command::PlaceFlag {
                    player: other,
                    at: (4, 4)
                }
            ),
            Err(Rejection::NotOwned)
        );
        assert_eq!(
            building(&mut world, 99, (4, 4)),
            Err(Rejection::UnknownBuildingType)
        );
        assert_eq!(flag(&mut world, (40, 4)), Err(Rejection::OutsideMap));
        assert_eq!(
            building(&mut world, SMALL, (4, 40)),
            Err(Rejection::OutsideMap)
        );
    }
}
