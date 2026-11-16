//! Where to build: the site value of a building type at a field
//! (AI-CON-6) and the incremental site search (AI-CON-5.4).

use s2_sim::grid::{self, Point};
use s2_sim::map::state;
use s2_sim::movement::NOT_WALKABLE;
use s2_sim::{Building, BuildingSize, BuildingType, DepositState, Placement, PlayerId, World};

use crate::cells::{centre, origin, CellPos, CELL};
use crate::config::AiConfig;
use crate::view::{area, dist};

/// AI-CON-5.4: the fields one step of the search tests at most.
pub const FIELDS_PER_TICK: usize = 10;

/// What the site value looks at.
pub struct SiteEnv<'a> {
    pub world: &'a World,
    pub config: &'a AiConfig,
    pub player: PlayerId,
    /// The player's buildings.
    pub own: &'a [Building],
}

impl SiteEnv<'_> {
    fn finished_of_type(
        &self,
        f: impl Fn(&BuildingType) -> bool,
    ) -> impl Iterator<Item = &Building> {
        self.own.iter().filter(move |b| {
            !b.under_construction && self.world.building_type(b.building_type).is_some_and(&f)
        })
    }

    /// The distance from `at` to the nearest of `fields`.
    fn nearest(&self, at: Point, fields: impl Iterator<Item = Point>) -> Option<u32> {
        fields.map(|f| dist(at, f)).min()
    }

    /// AI-CON-6.2 (T81): the number of trees or rocks of `good` standing
    /// within `radius` of `at`, counted over a flood from `at`: it steps to
    /// a neighbour within the radius that holds a deposit or is walkable, so
    /// a deposit no such chain reaches (beyond water) does not count.
    pub fn deposits_of(&self, at: Point, radius: u32, good: u32) -> u32 {
        let world = self.world;
        let terrain = world.terrain();
        let counts = |f: Point| {
            world.deposit(f).is_some_and(|d| {
                counts_for_site(&d.state)
                    && world.deposit_kind(d.kind).and_then(|k| k.good) == Some(good)
            })
        };
        let mut seen = std::collections::BTreeSet::from([at]);
        let mut open = vec![at];
        let mut count = 0;
        while let Some(f) = open.pop() {
            count += u32::from(counts(f));
            for d in 0..6 {
                let Some(q) = terrain.neighbour(f, d) else {
                    continue;
                };
                let enters = dist(at, q) <= radius
                    && (world.deposit(q).is_some() || !terrain.has(q, NOT_WALKABLE));
                if enters && seen.insert(q) {
                    open.push(q);
                }
            }
        }
        count
    }

    /// AI-CON-6.2 (T72): the sum of the resource amounts of `good` on the
    /// fields within `radius` of `at`, explored or not.
    pub fn resource_fields(&self, at: Point, radius: u32, good: u32) -> u32 {
        let world = self.world;
        area(world, at, radius)
            .into_iter()
            .filter_map(|f| world.resource_at(f))
            .filter(|r| r.good == good)
            .map(|r| u32::from(r.amount))
            .sum()
    }

    /// Whether game animals live within `radius` of `at`.
    pub fn animals_near(&self, at: Point, radius: u32) -> bool {
        // AI-CON-6.3 (T41): the hunter's own search's animals (ECO-FOOD-5.2):
        // one no hut has reserved, explored at its walk's end (T50).
        self.world.animals().any(|(_, a)| {
            a.hunted_by.is_none()
                && dist(at, a.at) <= radius
                && self.world.is_explored(self.player, a.walk_goal())
        })
    }

    /// AI-CON-6.2 (94d1a13) supplier value of good `g` seen from `at`:
    /// max(1, 15 - d) with d the distance to the nearest own building, in
    /// any state (sites included), of a type that produces it (a group good:
    /// any of its members), 0.1 if none within 1000; only when no type
    /// produces it, the nearest own finished warehouse on `at`'s landmass.
    fn supplier_value(&self, at: Point, good: u32) -> f32 {
        let world = self.world;
        let members = self.config.group_members(good);
        let produces =
            |t: &BuildingType| !t.warehouse && t.product.is_some_and(|p| members.contains(&p));
        let d = if world.building_types().any(produces) {
            self.nearest(
                at,
                self.own
                    .iter()
                    .filter(|b| world.building_type(b.building_type).is_some_and(produces))
                    .map(|b| b.at),
            )
        } else {
            let landmass = world.continent_of(at);
            self.nearest(
                at,
                self.finished_of_type(|o| o.warehouse)
                    .filter(|b| world.continent_of(b.at) == landmass)
                    .map(|b| b.at),
            )
        };
        match d {
            Some(d) if d < 1000 => (15 - d as i64).max(1) as f32,
            _ => 0.1,
        }
    }

    /// AI-CON-6: the value of a building of type `t` on `at`; 0 means
    /// unusable.
    pub fn value(&self, t: &BuildingType, at: Point) -> f32 {
        let world = self.world;
        let cfg = self.config;
        let names = &cfg.names;
        let terrain = world.terrain();
        let mut v = 1.0f32;
        let large = t.size == BuildingSize::Large;
        let mine = t.size == BuildingSize::Mine;
        let harbour = t.size == BuildingSize::Harbor;
        let military = t.territory_range > 0 && t.soldier_limit > 0;
        // AI-CON-6.1 (Q-075): the first harbour-pavement field in the rings
        // outward from p needs an own building near it.
        if !harbour {
            let r = if large { 2 } else { 1 };
            let reach = if large { 4 } else { 3 };
            let pavement = area(world, at, r)
                .into_iter()
                .find(|&f| terrain.has(f, state::FOR_HARBOR));
            if let Some(f) = pavement {
                if !self.own.iter().any(|b| dist(b.at, f) <= reach) {
                    return 0.0;
                }
            }
        }
        // AI-CON-6.2 base value.
        let special = t.id == names.shipyard || t.id == names.hunter;
        match t.product {
            None => {
                if let Some(planted) = t.planting_good {
                    // The supplier value of the planted good (a woodcutter
                    // in any state for a forester's wood; 94d1a13).
                    v = self.supplier_value(at, planted);
                }
            }
            Some(_) if special => {}
            Some(g) => {
                // H6: a type with a project's recipe makes its product from
                // that, not from the map: rated by its inputs' suppliers.
                let own =
                    cfg.sim.recipes.contains_key(&t.id) || cfg.sim.recipe_sets.contains_key(&t.id);
                if !own && cfg.deposits_goods.contains(&g) && !mine {
                    v = self.deposits_of(at, t.working_radius, g) as f32;
                } else if !own && cfg.fields_goods.contains(&g) {
                    v = self.resource_fields(at, t.working_radius, g) as f32;
                } else {
                    let inputs = cfg.type_inputs(t.id, g);
                    if !inputs.is_empty() {
                        v = inputs
                            .iter()
                            .map(|&(i, _)| self.supplier_value(at, i))
                            .sum();
                    }
                }
            }
        }
        // AI-CON-6.3 special types.
        if t.id == names.shipyard {
            let d = self.nearest(
                at,
                self.own
                    .iter()
                    .filter(|b| {
                        world
                            .building_type(b.building_type)
                            .is_some_and(|o| o.size == BuildingSize::Harbor)
                    })
                    .map(|b| b.at),
            );
            v = match d {
                Some(d) => (100 / d.max(1)).max(1) as f32,
                None => 1.0,
            };
        }
        if t.id == names.hunter {
            v = if self.animals_near(at, t.working_radius) {
                1.0
            } else {
                0.0
            };
        }
        if t.id == names.farm {
            // The 12 fields of the ring at distance exactly 2 that pass the
            // grain planting test (ECO-OUT-4.3; T38).
            let plantable = grid::ring(at, 2, terrain.width, terrain.height)
                .into_iter()
                .filter(|&f| world.grain_plantable(f))
                .count();
            v += plantable as f32;
        }
        if military {
            let own_military = || {
                self.own
                    .iter()
                    .filter(|b| {
                        world
                            .building_type(b.building_type)
                            .is_some_and(|o| o.territory_range > 0)
                    })
                    .map(|b| b.at)
            };
            let nearest = self.nearest(at, own_military());
            if nearest.is_some_and(|d| d < 5) {
                return 0.0;
            }
            // The landmass is the map's stored continent index, compared
            // for equality (FMT-S2M-4.9).
            let continent = world.continent_of(at);
            let free = area(world, at, t.territory_range)
                .into_iter()
                .filter(|&f| {
                    world.continent_of(f) == continent && world.field_owner(f) != Some(self.player)
                })
                .count();
            if free == 0 {
                return 0.0;
            }
            v = free as f32 + nearest.map_or(100, |d| d.min(100)) as f32;
        }
        if harbour {
            let near = self.nearest(
                at,
                self.own
                    .iter()
                    .filter(|b| {
                        world
                            .building_type(b.building_type)
                            .is_some_and(|o| o.size == BuildingSize::Harbor)
                    })
                    .map(|b| b.at),
            );
            v = if near.is_some_and(|d| d < 5) {
                0.0
            } else {
                1.0
            };
        }
        if t.warehouse && !harbour {
            let near = self.nearest(
                at,
                self.own
                    .iter()
                    .filter(|b| {
                        !b.under_construction
                            && world
                                .building_type(b.building_type)
                                .is_some_and(|o| o.warehouse)
                    })
                    .map(|b| b.at),
            );
            v = if near.is_some_and(|d| d < 11) {
                0.0
            } else {
                1.0
            };
        }
        // AI-CON-6.4 penalties: an own farm in any state (its site
        // included, 94d1a13).
        if !mine && !harbour {
            let reach = if t.id == names.farm { 3 } else { 4 };
            let farm_near = self
                .own
                .iter()
                .filter(|b| b.building_type == names.farm)
                .any(|b| dist(at, b.at) <= reach);
            if farm_near {
                if military {
                    v *= 0.5;
                } else {
                    v = 0.0;
                }
            }
        }
        // A Border field on the ring at distance exactly 2 (its 12 fields,
        // those outside the map skipped); p, its neighbours and farther
        // fields are not looked at.
        if !military && !mine && !harbour {
            let border = grid::ring(at, 2, terrain.width, terrain.height)
                .into_iter()
                .any(|f| terrain.has(f, state::BORDER));
            if border {
                v *= 0.4;
            }
        }
        if v > 0.0 {
            let class = world.compute_placement(self.player, at);
            let small = t.size == BuildingSize::Small;
            let medium = t.size == BuildingSize::Medium;
            if (small && class == Placement::Medium)
                || ((small || medium) && class == Placement::Large)
            {
                v *= 0.5;
            }
        }
        v
    }
}

/// AI-CON-5.4a: the distance within which a building with a territory
/// range keeps a new military site away.
pub const MILITARY_CLEARANCE: u32 = s2_sim::MILITARY_CLEARANCE;

/// AI-CON-5.4a, the military clause of the size test: a building or
/// construction site of any player whose type has a territory range (the
/// headquarters included) stands within distance 4 of `at` (rings 0 to 4,
/// each building counted on its own field); the placement's own test,
/// ECO-PLACE-6.2.
pub fn military_near(world: &World, at: Point) -> bool {
    world.military_near(at)
}

/// AI-CON-5.4a: what makes a field unfit for the flag test.
const FLAG_BLOCKING: u32 = state::BORDER
    | state::WATER
    | state::ELEVATION_BLOCKED
    | state::DOODAD_BLOCKED
    | state::BLOCKED
    | state::DEPOSIT_BLOCKED
    | state::MOVEMENT_BLOCKED;

/// AI-CON-5.4a, the flag test: one copy of `at` stepped through the
/// directions 0, 1, 2, 3, 4 in turn (the east neighbour, three fields at
/// distance 2, the direction-2 neighbour); one of them must be in the map,
/// owned by `player` and free of the blocking states (a flag on it does not
/// matter).
pub fn flag_room(world: &World, player: PlayerId, at: Point) -> bool {
    let terrain = world.terrain();
    let mut p = (at.0 as i32, at.1 as i32);
    (0..5).any(|d| {
        p = grid::step(p.0, p.1, grid::Direction::new(d).expect("a direction"));
        let (Ok(x), Ok(y)) = (u32::try_from(p.0), u32::try_from(p.1)) else {
            return false;
        };
        let f = (x, y);
        terrain.contains(f)
            && world.field_owner(f) == Some(player)
            && !terrain.has(f, FLAG_BLOCKING)
    })
}

/// Whether a field of placement class `class` takes a building of `size`
/// (ECO-PLACE-4, AI-CON-5.3): small needs class 2 to 4, medium 3 to 4,
/// large 4, mine 5, harbour 6.
pub fn class_fits(size: BuildingSize, class: Placement) -> bool {
    match size {
        BuildingSize::Small => matches!(
            class,
            Placement::Small | Placement::Medium | Placement::Large
        ),
        BuildingSize::Medium => matches!(class, Placement::Medium | Placement::Large),
        BuildingSize::Large => class == Placement::Large,
        BuildingSize::Mine => class == Placement::Mine,
        BuildingSize::Harbor => class == Placement::Harbor,
    }
}

/// Where a site search looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    /// AI-CON-5.4 cell mode: the 16 x 16 fields of a cell, routes up to 16.
    Cell(CellPos),
    /// Point mode: a spiral of radius R, routes up to 2 R.
    Point { at: Point, radius: u32 },
}

impl Area {
    /// The field the route length is measured from.
    pub fn start(self) -> Point {
        match self {
            Area::Cell(cell) => centre(cell),
            Area::Point { at, .. } => at,
        }
    }

    /// The route length limit.
    pub fn limit(self) -> u32 {
        match self {
            Area::Cell(_) => CELL,
            Area::Point { radius, .. } => 2 * radius,
        }
    }

    /// The candidate fields in scan order.
    pub fn fields(self, world: &World) -> Vec<Point> {
        let terrain = world.terrain();
        match self {
            // AI-CON-5.4: all 256 fields, row by row with x fastest; those
            // outside the map fail the in-map test.
            Area::Cell(cell) => {
                let (x0, y0) = origin(cell);
                (y0..y0 + CELL)
                    .flat_map(|y| (x0..x0 + CELL).map(move |x| (x, y)))
                    .collect()
            }
            Area::Point { at, radius } => grid::area(at, radius, terrain.width, terrain.height),
        }
    }
}

/// AI-CON-5.4, 5.4b: the length in steps of the route from `start` to
/// `to`, a free-walk path (TRN-FREE-2, 3): 0 for the start itself, 1 for a
/// neighbour (no search), else the first try's search of the direct variant
/// of AI-CON-8.3 c, its steps counted from the start down the labels taking
/// the lowest allowed direction at every field (5.4b: no kept direction,
/// no draws); `None` when the first try does not reach it.
pub fn route_length(world: &World, start: Point, to: Point) -> Option<u32> {
    if start == to {
        return Some(0);
    }
    if grid::distance(start, to) == 1 {
        return Some(1);
    }
    s2_sim::movement::free_walk_lowest_directions(world.terrain(), start, to)
        .map(|route| route.fields.len() as u32)
}

/// AI-CON-5.4b: a candidate's score, ((limit - len) / limit) x value in
/// single precision, divided first: with a limit that is no power of two
/// (2R in point mode) the order can decide a near tie.
pub fn score(value: f32, len: u32, limit: u32) -> f32 {
    (limit - len) as f32 / limit as f32 * value
}

/// A site search in progress.
#[derive(Debug, Clone)]
pub struct Scan {
    pub area: Area,
    pub building: u32,
    fields: Vec<Point>,
    next: usize,
    /// The field the routes are measured from.
    start: Point,
    best: Option<(f32, Point)>,
}

impl Scan {
    /// A search over `area`, its routes from `start` (the cell's position,
    /// AI-CON-1.1a) or else the area's own start.
    pub fn new(world: &World, area: Area, building: u32, start: Option<Point>) -> Self {
        Self {
            fields: area.fields(world),
            start: start.unwrap_or(area.start()),
            area,
            building,
            next: 0,
            best: None,
        }
    }

    pub fn done(&self) -> bool {
        self.next >= self.fields.len()
    }

    /// The fields tested so far.
    pub fn tested(&self) -> usize {
        self.next
    }

    /// The best field so far.
    pub fn best(&self) -> Option<Point> {
        self.best.map(|(_, at)| at)
    }

    /// One step of the search (AI-CON-5.4): up to `FIELDS_PER_TICK` fields,
    /// ending early after the first that passes the cheap tests (in the map,
    /// size with the military clause, the flag test of AI-CON-5.4a), for
    /// which the site value and the route are then worked out.
    pub fn step(&mut self, env: &SiteEnv) {
        let Some(t) = env.world.building_type(self.building) else {
            self.next = self.fields.len();
            return;
        };
        let limit = self.area.limit();
        let terrain = env.world.terrain();
        for _ in 0..FIELDS_PER_TICK {
            let Some(&f) = self.fields.get(self.next) else {
                break;
            };
            self.next += 1;
            if f.0 >= terrain.width || f.1 >= terrain.height {
                continue;
            }
            if env.world.field_owner(f) != Some(env.player) {
                continue;
            }
            let class = env.world.compute_placement(env.player, f);
            if !class_fits(t.size, class) {
                continue;
            }
            if t.territory_range > 0 && military_near(env.world, f) {
                continue;
            }
            if !flag_room(env.world, env.player, f) {
                continue;
            }
            self.consider(env, t, f, limit);
            break;
        }
    }

    /// AI-CON-5.4: a field whose value is above the best score so far and
    /// whose route is shorter than the limit scores ((limit - len) / limit)
    /// x value (5.4b), and the best score wins (strictly greater).
    fn consider(&mut self, env: &SiteEnv, t: &BuildingType, f: Point, limit: u32) {
        let best = self.best.map_or(0.0, |(s, _)| s);
        let value = env.value(t, f);
        if value <= best {
            return;
        }
        let Some(len) = route_length(env.world, self.start, f) else {
            return;
        };
        if len >= limit {
            return;
        }
        let score = score(value, len, limit);
        if score > best {
            self.best = Some((score, f));
        }
    }
}

// The snapshot (docs/snapshot.md) of the types whose fields are private
// to this module.
s2_sim::snap_struct!(Scan {
    area,
    building,
    fields,
    next,
    start,
    best
});

/// AI-CON-6.2 (T90): whether a deposit counts in a site value's flood: the
/// test looks only at its good and its removal mark, never at its growth,
/// so a sapling counts from the moment the forester makes it (ECO-OUT-4.5);
/// a deposit being removed does not.
pub(crate) fn counts_for_site(state: &DepositState) -> bool {
    !matches!(state, DepositState::Removing { .. })
}
