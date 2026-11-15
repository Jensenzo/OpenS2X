//! Buildings, deposits and decorations where the 3D view draws them, with
//! their models and looks (GFX-VIEW-4.3 to 4.5, GFX-OBJ-1, 2).

use super::*;

/// What the view needs from the rule set to pick models: the graphics
/// name of each building type (`bp_setGfxName`) and of each deposit type
/// (`dp_setGfxName`) with its growing start size and speed.
#[derive(Debug, Clone, Default)]
pub struct Lookup {
    pub buildings: HashMap<u32, String>,
    /// GFX-OBJ-8.7 (5): per building type, the graphics name of its remains
    /// (`bp_setRemains`, `rm_setGfxName`), the ruin it leaves.
    pub ruins: HashMap<u32, String>,
    pub deposits: HashMap<u32, DepositLook>,
    /// The names of goods (wares and professions) and the graphics names of
    /// animal kinds (`an_setGfxName`).
    pub goods: HashMap<u32, String>,
    pub animals: HashMap<u32, String>,
    /// Doodad types: their graphics name and whether roads remove them.
    pub doodads: HashMap<u32, (String, bool)>,
    /// GFX-OBJ-7.4: a job's tool by their names: the first good of the
    /// job's cost list that has an `items.xml` entry.
    pub tool_names: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct DepositLook {
    pub gfx_name: String,
    /// `dp_setGrowingStartSize`, `dp_setGrowingSpeed` (size per tick), or
    /// their defaults (ECO-OUT-1.4).
    pub start_size: Option<f32>,
    pub speed: Option<f32>,
}

impl Lookup {
    pub fn new(rules: &s2_rules::RuleSet, tables: &ModelTables) -> Self {
        let name = |g: u32| rules.goods.get(g).and_then(|g| g.name.as_deref());
        let tool_names = rules
            .goods
            .iter()
            .filter_map(|job| {
                let costs = job.costs.iter().filter_map(|a| name(a.good));
                let tool = crate::world::carry::tool_of(costs, |n| tables.item(n).is_some())?;
                Some((job.name.clone()?, tool.to_owned()))
            })
            .collect();
        Self {
            buildings: rules
                .buildings
                .iter()
                .filter_map(|b| Some((b.id, b.gfx_name.clone()?)))
                .collect(),
            ruins: rules
                .buildings
                .iter()
                .filter_map(|b| {
                    let remains = rules.remains.get(b.remains?)?;
                    Some((b.id, remains.gfx_name.clone()?))
                })
                .collect(),
            deposits: rules
                .deposits
                .iter()
                .filter_map(|d| {
                    Some((
                        d.id,
                        DepositLook {
                            gfx_name: d.gfx_name.clone()?,
                            // ECO-OUT-1.4, 6.1: the script's values, else
                            // the defaults (0.1 and 0.0009 for the trees).
                            start_size: Some(
                                d.growing_start_size.unwrap_or(s2_rules::GROWING_START) as f32,
                            ),
                            speed: Some(d.growing_speed.unwrap_or(s2_rules::GROWING_SPEED) as f32),
                        },
                    ))
                })
                .collect(),
            goods: rules
                .goods
                .iter()
                .filter_map(|g| Some((g.id, g.name.clone()?)))
                .collect(),
            animals: rules
                .animals
                .iter()
                .filter_map(|a| Some((a.id, a.gfx_name.clone()?)))
                .collect(),
            doodads: rules
                .doodads
                .iter()
                // GFX-OBJ-1.4a: `dd_setDeleteByStreet` is on unless a script
                // turns it off (the shipped scripts do for the signs).
                .filter_map(|d| {
                    Some((
                        d.id,
                        (d.gfx_name.clone()?, d.delete_by_street.unwrap_or(true)),
                    ))
                })
                .collect(),
            tool_names,
        }
    }
}

/// What a placed model is, for telling one from another between redraws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Building,
    /// GFX-OBJ-8.1, 8.2: a building's soldier flag (serial x 2) and border
    /// flag (serial x 2 + 1).
    MilitaryFlag,
    Flag,
    Border,
    Deposit,
    Doodad,
    Sign,
    /// GFX-OBJ-6.1, 6.2: a waterway's buoy (the road's number in the
    /// world's list x 4096 + the buoy's).
    Buoy,
    /// GFX-OBJ-8.7 (2): a finished building's house collapsing (its
    /// serial).
    Collapse,
    /// GFX-OBJ-8.7 (5): a ruin's model (0) and its ground decal (1).
    Ruin,
}

/// One model standing on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// Stays the same while the thing stays: its kind, field and what else
    /// tells it apart (the building's serial and stage, the deposit's type).
    pub key: (Kind, Point, u32),
    /// The game-relative model file and the units per metre it is drawn at
    /// (GFX-VIEW-1.3).
    pub file: String,
    pub units_per_metre: f32,
    /// Position in the game frame and the heading about +y (GFX-VIEW-4.1).
    pub at: [f32; 3],
    pub heading: f32,
    /// Size per axis on top of the units per metre: an element's random
    /// size times a growing tree's growth (GFX-OBJ-1.4).
    pub scale: [f32; 3],
    /// The axis of `tilt` as a horizontal yaw (game frame, from +x toward
    /// +z): a felled tree turns about the line toward its woodcutter
    /// (GFX-ANIM-3.8a).
    pub tilt_axis: f32,
    /// A turn about the horizontal `tilt_axis` after its heading: a felled
    /// tree falling over (radians).
    pub tilt: f32,
    /// A tint, white unless the model has no colouring of its own.
    pub tint: [f32; 4],
    /// The shading family (GFX-OBJ-2), the owner's colour (black: none)
    /// and the family's parameters (a house's colour-map offset).
    pub family: models3d::Family,
    pub player: [f32; 3],
    pub extra: [f32; 4],
    /// GFX-VIEW-2.5a: drawn into the shadow map (false for an element
    /// with `shadowCast="false"`).
    pub shadow: bool,
    /// GFX-OBJ-8.1: only the meshes `0` to `n - 1` (numeric name order)
    /// show; all when `None`.
    pub pieces: Option<u32>,
    /// GFX-OBJ-8.4: the house whose height sets a cut house's plane
    /// (the new type's during an upgrade); its own when `None`.
    pub plane_of: Option<String>,
    /// GFX-OBJ-8.7: which of the model's parts show.
    pub parts: Parts,
    /// GFX-OBJ-8.7 (2): a collapsing house's clock c: it is drawn lowered
    /// by H x c^2, H its model's height (taken where the model is loaded).
    pub sink: Option<f32>,
}

/// GFX-OBJ-8.7: which of a model's parts show: all; all but its ground
/// decal (a collapsing house, whose decal goes at once, (1)); only its
/// ground decal (the copy of the building's decal a ruin lies on, (5)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Parts {
    #[default]
    All,
    NoDecal,
    DecalOnly,
}

pub(crate) const WHITE: [f32; 4] = [1.0; 4];

/// GFX-OBJ-8.7 (2), T36: a collapsing house's base height: the ground's
/// `base` minus H x c^2 (H the house model's height, c its collapse clock,
/// 0 .. 1): slowly at first, its top at the ground when c reaches 1.
pub fn collapse_height(base: f32, h: f32, c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    base - h * c * c
}

/// What the map holds besides the world: the landscape elements, the
/// doodads (type and sample) and the house boxes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scenery<'a> {
    pub elements: &'a [s2_assets::landscape::LandscapeElement],
    pub doodads: &'a [(u32, (u32, u32))],
    /// GFX-OBJ-8.1a: the house boxes by building model file (the `House`
    /// mesh of its `...LOD0` file, else of the file), game frame.
    pub house_boxes: Option<&'a HashMap<String, crate::world::decor::Box3>>,
    /// The elements by name, where they are many and asked for often (F55).
    pub by_name: Option<&'a HashMap<String, usize>>,
    /// GFX-OBJ-1.4 (T32): what the loaded file gave the view, in its order.
    pub load: Load<'a>,
    /// GFX-OBJ-2.2a: the view's slot table; without one the buildings are
    /// numbered as a view built afresh numbers them (2.2a 5).
    pub house_slots: Option<&'a HouseSlots>,
}

/// GFX-OBJ-1.4 (T32): what the file the game came from gave the view, in
/// the file's order: the view learns of it only after the whole file is read.
#[derive(Debug, Clone, Copy, Default)]
pub struct Load<'a> {
    /// The game is a save: its road flags were in the file and come first,
    /// player 0 to 7, each in its list order (FMT-SAV-3.9). A map file holds
    /// none: every flag of a new game is one the game made.
    pub saved: bool,
    /// The file's deposit list in its order, only those of size 0 or more
    /// (a deposit of size -1 makes no instance): field and kind. `None` for a
    /// save of ours, whose list is the world's, field by field.
    pub deposits: Option<&'a [(Point, u32)]>,
    /// The file's doodads, list 1, then 2, then 3, each by ascending id
    /// ([`load_doodads`]); empty: `Scenery::doodads` as they are.
    pub doodads: &'a [(u32, (u32, u32))],
}

/// GFX-OBJ-1.4 (T32): a map's or save's doodads in the order the view learns
/// of them: list 1 by ascending id, then list 2, then list 3, each by id
/// (FMT-S2M-6.1); type and sample.
pub fn load_doodads(lists: &s2_assets::s2m::Doodads) -> Vec<(u32, (u32, u32))> {
    let mut out = Vec::new();
    for list in [&lists.non_blocking, &lists.unknown, &lists.blocking] {
        let mut list: Vec<&s2_assets::s2m::Doodad> = list.iter().collect();
        list.sort_by_key(|d| d.id);
        out.extend(
            list.iter()
                .map(|d| (d.type_id, (d.position.x, d.position.y))),
        );
    }
    out
}

/// GFX-OBJ-1.4 (T32): a map's or save's deposit list in the file's order,
/// those of size 0 or more (field and kind).
pub fn load_deposits(deposits: &[s2_assets::s2m::Deposit]) -> Vec<(Point, u32)> {
    deposits
        .iter()
        .filter(|d| d.size >= 0.0)
        .map(|d| ((d.field.x, d.field.y), d.type_id))
        .collect()
}

impl Scenery<'_> {
    pub fn element(&self, name: &str) -> Option<&s2_assets::landscape::LandscapeElement> {
        match self.by_name {
            Some(index) => index.get(name).and_then(|&i| self.elements.get(i)),
            None => self.elements.iter().find(|e| e.name == name),
        }
    }
}

/// GFX-OBJ-1.4: the view's own generator for map objects, the generator of
/// SIM-RNG seeded once when the program starts with 0x5432ACE0 and never
/// reseeded (not by a new game, a load or a map change).
static VIEW_RNG: std::sync::LazyLock<std::sync::Mutex<s2_sim::rng::Rng>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(s2_sim::rng::Rng::new(VIEW_SEED)));

/// GFX-OBJ-1.4: the view generator's seed.
pub const VIEW_SEED: u32 = 0x5432_ACE0;

/// An instance's key in the view (`Placed::key`).
pub type InstanceKey = (Kind, Point, u32);

/// GFX-OBJ-1.4: an instance's yaw (radians) and size per axis.
pub type Pose = (f32, [f32; 3]);

/// GFX-OBJ-1.4: the four draws of one landscape-element instance: yaw =
/// rot.min + u (rot.max - rot.min) in radians, then the x, y and z size, each
/// u a unit draw (next mod 1000001) x 0.000001; made even for empty ranges.
pub fn element_draw(
    element: &s2_assets::landscape::LandscapeElement,
    rng: &mut s2_sim::rng::Rng,
) -> Pose {
    let mut pick = |[lo, hi]: [f32; 2]| lo + (hi - lo) * rng.fraction();
    let yaw = pick(element.rot).to_radians();
    let x = pick(element.scale_x);
    let y = pick(element.scale_y);
    let z = pick(element.scale_z);
    (yaw, [x, y, z])
}

/// GFX-OBJ-1.4: the poses of the view's landscape-element instances, each
/// drawn once when the view first learns of the instance and kept while it
/// stands. The draws come from the program's generator (`VIEW_RNG`), or from
/// a generator of its own (tests).
#[derive(Default)]
pub struct Poses {
    drawn: HashMap<InstanceKey, Pose>,
    own: Option<s2_sim::rng::Rng>,
}

impl std::fmt::Debug for Poses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Poses")
            .field("drawn", &self.drawn.len())
            .finish()
    }
}

impl Poses {
    /// Poses drawn from a generator of their own.
    #[cfg(test)]
    pub fn with_rng(rng: s2_sim::rng::Rng) -> Self {
        Self {
            drawn: HashMap::new(),
            own: Some(rng),
        }
    }

    /// No instance seen yet (the map or save is being loaded).
    pub fn is_empty(&self) -> bool {
        self.drawn.is_empty()
    }

    /// The pose of `key`, drawn now if the instance is new.
    pub fn pose(
        &mut self,
        key: InstanceKey,
        element: &s2_assets::landscape::LandscapeElement,
    ) -> Pose {
        if let Some(&pose) = self.drawn.get(&key) {
            return pose;
        }
        let pose = match self.own.as_mut() {
            Some(rng) => element_draw(element, rng),
            None => {
                let mut rng = VIEW_RNG.lock().unwrap_or_else(|e| e.into_inner());
                element_draw(element, &mut rng)
            }
        };
        self.drawn.insert(key, pose);
        pose
    }

    /// The instance is gone: one appearing under the same key later is a new
    /// instance and draws again.
    pub fn forget(&mut self, key: &InstanceKey) {
        self.drawn.remove(key);
    }
}

/// GFX-OBJ-1.4, T34: a road flag moves along world z by 1.3 x the street
/// width (1.0): -1.3 when the hash of its field's sample (c, r) on the height
/// lattice, i = r x (4 width + 1) + c, times 2.98105e-7 is at most 0.5, else
/// +1.3 (32-bit arithmetic).
pub fn flag_shift(field: Point, width: u32) -> f32 {
    let (c, r) = s2_sim::grid::field_sample(field);
    let i = r
        .wrapping_mul(width.wrapping_mul(4).wrapping_add(1))
        .wrapping_add(c);
    let a = i.wrapping_mul(58);
    let u = a ^ a.wrapping_mul(8192);
    let v = 0x0034_34B6u32.wrapping_sub(
        u.wrapping_mul(u)
            .wrapping_mul(0x4BD9)
            .wrapping_add(0x0002_E48D)
            .wrapping_mul(u),
    ) & 0x004B_746F;
    if v as f32 * 2.981_05e-7 <= 0.5 {
        -FLAG_SHIFT
    } else {
        FLAG_SHIFT
    }
}

/// GFX-OBJ-1.4: 1.3 x the `<Street>` width of `graphics.xml` (1.0).
pub const FLAG_SHIFT: f32 = 1.3;

/// GFX-OBJ-1.4: an element's height at (x, z): the ground there, or the
/// water level where `alignOnWater` is set and the ground is lower.
fn element_height(
    world: &World,
    e: &s2_assets::landscape::LandscapeElement,
    x: f32,
    z: f32,
) -> f32 {
    let ground = ground_height(world.terrain(), x, z);
    if e.align_on_water {
        ground.max(WATER_LEVEL)
    } else {
        ground
    }
}

/// GFX-VIEW-4.5: a growing deposit's size, from its growing start size to
/// 1 at the growing speed per tick since it began to grow; a sapling has
/// the start size, anything else full size. Without a speed the size grows
/// evenly over the kind's growing time.
pub fn deposit_size(world: &World, kind: u32, state: DepositState, look: &DepositLook) -> f32 {
    let start = look.start_size.unwrap_or(1.0).clamp(0.0, 1.0);
    match state {
        // ECO-OUT-4.5: a tree being planted has no size yet; it shows when
        // the planting ends (OBS-7: he digs, then the sapling is there).
        DepositState::Sapling => 0.0,
        DepositState::Removing { .. } => 1.0,
        DepositState::Growing { ripe_at } => {
            let grow = world.deposit_kind(kind).map_or(0, |k| k.grow_ticks);
            // Ripe (the map's trees from the start): full size.
            if grow == 0 || world.tick() >= ripe_at {
                return 1.0;
            }
            let since = world
                .tick()
                .saturating_sub(ripe_at.saturating_sub(u64::from(grow)));
            let size = match look.speed {
                Some(speed) if speed > 0.0 => start + speed * since as f32,
                _ => start + (1.0 - start) * since as f32 / grow as f32,
            };
            size.clamp(start, 1.0)
        }
    }
}

/// Everything that stands still on the map: buildings (GFX-VIEW-4.3, 4.4:
/// the construction site's `frame` model until finished, then the `mesh`;
/// every building turned by its type's heading, -30 degrees, at its
/// field's own sample height), flags, the border markers of TER-7 (a stone
/// on land at the ground, a buoy on water at the surface; in the owner's
/// colour) and the deposits at their sample (TRN-FREE-5.1) with their
/// growth (GFX-ANIM-3.8b: height s, width min(3 s, 1)). The view
/// re-places growing trees every few ticks, not every frame.
#[allow(clippy::too_many_arguments)]
pub fn objects(
    world: &World,
    lookup: &Lookup,
    tables: &ModelTables,
    scenery: &Scenery,
    frame: &Frame,
    borders: bool,
    poses: &mut Poses,
) -> Vec<Placed> {
    let mut out = Vec::new();
    if poses.is_empty() {
        load_poses(world, lookup, scenery, poses);
    }
    let on_ground = |x: f32, z: f32| [x, ground_height(world.terrain(), x, z), z];
    let afresh;
    let slots = match scenery.house_slots {
        Some(slots) => slots,
        None => {
            afresh = HouseSlots::of_world(world);
            &afresh
        }
    };
    let plain = |key, file: String, at, heading, scale, family, player| Placed {
        key,
        file,
        units_per_metre: models3d::DEFAULT_SCALE,
        at,
        heading,
        scale,
        tilt: 0.0,
        tilt_axis: 0.0,
        tint: WHITE,
        family,
        player,
        extra: [0.0; 4],
        shadow: true,
        pieces: None,
        plane_of: None,
        parts: Parts::All,
        sink: None,
    };
    // GFX-OBJ-8.7 (1), (2): a finished building that left the world
    // collapses: its house stays, in its slot's column, without its ground
    // decal, status icon and flags, and sinks by H x c^2 (the base the
    // ground at its field now, read again each time; H where the model is
    // loaded) until its clock c reaches 1 and its slot is freed.
    for (house, offset, c) in slots.collapsing() {
        let Some(gfx) = lookup.buildings.get(&house.building_type) else {
            continue;
        };
        let tribe = world.tribe(house.owner);
        let Some(entry) = tables.building(&models3d::building_entry(tribe, gfx)) else {
            continue;
        };
        let Some(mesh) = entry.mesh.as_ref() else {
            continue;
        };
        let at = crate::render::world3d::building_position(world.terrain(), house.at.0, house.at.1);
        let mut placed = plain(
            (Kind::Collapse, house.at, house.serial),
            mesh.clone(),
            at,
            (entry.heading as f32).to_radians(),
            [1.0; 3],
            models3d::Family::House,
            models3d::player_colour(house.owner),
        );
        placed.extra[0] = offset;
        placed.parts = Parts::NoDecal;
        placed.sink = Some(c);
        out.push(placed);
    }
    // GFX-OBJ-8.7 (5): a ruin while its remains burn: the model of its
    // remains for the tribe of the building it was (`<tribe>_<remains>`),
    // at its own entry's heading (the loader's -30 degrees, as every
    // building), on that building's field, lying on a copy of the type's
    // ground decal draped as the building's (2.3). It is drawn in the site
    // frame's pass (2.6a: per-vertex light x texel, no colour map, no light
    // map, opaque; every piece: `extra[3]` 2 = 1 + a whole frame). No
    // flames. A ruin of a game saved before it knew its building (snapshot
    // 72) is not drawn.
    for ruin in world.ruins() {
        let Some(building_type) = ruin.building_type else {
            continue;
        };
        let tribe = world.tribe(ruin.owner);
        let Some(entry) = lookup
            .buildings
            .get(&building_type)
            .and_then(|gfx| tables.building(&models3d::building_entry(tribe, gfx)))
        else {
            continue;
        };
        let at = crate::render::world3d::building_position(world.terrain(), ruin.at.0, ruin.at.1);
        let model = lookup
            .ruins
            .get(&building_type)
            .and_then(|name| tables.building(&models3d::building_entry(tribe, name)));
        if let Some((model, file)) = model.and_then(|m| Some((m, m.mesh.as_ref()?))) {
            let mut placed = plain(
                (Kind::Ruin, ruin.at, 0),
                file.clone(),
                at,
                (model.heading as f32).to_radians(),
                [1.0; 3],
                models3d::Family::House,
                [0.0; 3],
            );
            placed.extra[3] = 2.0;
            out.push(placed);
        }
        let heading = (entry.heading as f32).to_radians();
        if let Some(mesh) = entry.mesh.as_ref() {
            let mut decal = plain(
                (Kind::Ruin, ruin.at, 1),
                mesh.clone(),
                at,
                heading,
                [1.0; 3],
                models3d::Family::House,
                [0.0; 3],
            );
            decal.parts = Parts::DecalOnly;
            out.push(decal);
        }
    }
    for building in world.buildings() {
        let Some(gfx) = lookup.buildings.get(&building.building_type) else {
            continue;
        };
        let tribe = world.tribe(building.owner);
        let Some(entry) = tables.building(&models3d::building_entry(tribe, gfx)) else {
            continue;
        };
        let file = if building.under_construction {
            entry.frame.as_ref().or(entry.mesh.as_ref())
        } else {
            entry.mesh.as_ref()
        };
        let Some(file) = file else { continue };
        // GFX-VIEW-4.3a, c: turned so the door faces the flag, at the
        // height of the field's own sample.
        let at = crate::render::world3d::building_position(
            world.terrain(),
            building.at.0,
            building.at.1,
        );
        let heading = (entry.heading as f32).to_radians();
        // GFX-OBJ-2.6b: beside the site model the house itself, its ground
        // decal (2.3) from the start, its walls cut by a plane that rises
        // with the main progress m (`extra[1]`; the height is set where the
        // model is loaded), not drawn while m is 0. m is the logic value of
        // the tick (not interpolated between ticks).
        let site = world
            .site(building.at)
            .filter(|_| building.under_construction);
        if building.under_construction && entry.frame.is_some() {
            if let (Some(mesh), Some(site)) = (entry.mesh.as_ref(), site) {
                let (_, main) = site.progress();
                let mut house = plain(
                    (Kind::Building, building.at, building.serial * 2),
                    mesh.clone(),
                    at,
                    heading,
                    [1.0; 3],
                    models3d::Family::House,
                    models3d::player_colour(building.owner),
                );
                // GFX-OBJ-2.2a (6): the rising house takes the building's
                // column.
                house.extra = [slots.offset(building.serial), main, 1.0, 0.0];
                out.push(house);
            }
        }
        let mut placed = plain(
            (
                Kind::Building,
                building.at,
                building.serial * 2 + u32::from(building.under_construction),
            ),
            file.clone(),
            at,
            heading,
            [1.0; 3],
            models3d::Family::House,
            models3d::player_colour(building.owner),
        );
        // GFX-OBJ-2.2, 2.2a: the column of the colour map, the building's
        // slot mod 8. A site's frame model (drawn with the house's family,
        // its frame pass) reads no column (2.2a (6), 2.6a: the shader's
        // frame pass samples no colour map), so it has none here.
        placed.extra[0] = if building.under_construction && entry.frame.is_some() {
            0.0
        } else {
            slots.offset(building.serial)
        };
        // GFX-OBJ-2.6a: the site model's pieces go up with the frame
        // progress f: `extra[3]` = 1 + f; the stage needs the model's piece
        // count and is taken where the model is loaded.
        if let (Some(site), Some(_)) = (site, entry.frame.as_ref()) {
            placed.extra[3] = 1.0 + site.progress().0;
        }
        // GFX-OBJ-8.4: during an upgrade the old house shows only above
        // the plane, the new house only below it, the new type's site
        // model around both; u is the upgrade site's main progress.
        let upgrade = world
            .upgrading(building.at)
            .filter(|_| !building.under_construction)
            .and_then(|next| Some((lookup.buildings.get(&next)?, world.site(building.at)?)))
            .and_then(|(gfx, site)| {
                let to = tables.building(&models3d::building_entry(tribe, gfx))?;
                Some((to, site.progress().1))
            });
        if let Some((to, u)) = upgrade {
            placed.extra[1] = u;
            placed.extra[2] = 2.0;
            placed.plane_of = to.mesh.clone();
            if let Some(mesh) = to.mesh.as_ref() {
                let mut house = plain(
                    (Kind::Building, building.at, building.serial * 2 + 1),
                    mesh.clone(),
                    at,
                    heading,
                    [1.0; 3],
                    models3d::Family::House,
                    models3d::player_colour(building.owner),
                );
                house.extra = [placed.extra[0], u, 1.0, 0.0];
                out.push(house);
            }
            if let Some(frame_file) = to.frame.as_ref() {
                out.push(plain(
                    (Kind::Building, building.at, u32::MAX - building.serial),
                    frame_file.clone(),
                    at,
                    heading,
                    [1.0; 3],
                    models3d::Family::House,
                    models3d::player_colour(building.owner),
                ));
            }
        }
        out.push(placed);
        // GFX-OBJ-8.1, 8.2: the soldier flag of a finished military
        // building or headquarters at the house's front right corner,
        // the border flag of an occupied military building at its front
        // left; both turned by 90 degrees in the building's frame.
        let military = world
            .military_at(building.at)
            .filter(|_| !building.under_construction);
        let kind = world.building_type(building.building_type);
        let headquarters = kind.is_some_and(|k| k.warehouse);
        let holds_soldiers = kind.is_some_and(|k| k.soldier_limit >= 1);
        let house_box = entry
            .mesh
            .as_ref()
            .and_then(|m| scenery.house_boxes?.get(m));
        if let (Some(m), Some(house_box)) = (military, house_box) {
            if headquarters || holds_soldiers {
                let flag_at =
                    |local| crate::world::decor::building_to_world(entry.heading as f32, at, local);
                let flag_heading =
                    (crate::world::decor::FLAG_TURN + entry.heading as f32).to_radians();
                let shown = crate::world::decor::soldier_flag_meshes(
                    world.present_soldiers(building.at),
                    headquarters,
                );
                let mut soldier = plain(
                    (Kind::MilitaryFlag, building.at, building.serial * 2),
                    crate::world::decor::SOLDIER_FLAG.to_owned(),
                    flag_at(crate::world::decor::soldier_flag_local(house_box)),
                    flag_heading,
                    [1.0; 3],
                    models3d::Family::Flag,
                    models3d::player_colour(building.owner),
                );
                soldier.pieces = Some(shown as u32);
                out.push(soldier);
                if !headquarters && m.applied {
                    let mut border = plain(
                        (Kind::MilitaryFlag, building.at, building.serial * 2 + 1),
                        crate::world::decor::BORDER_FLAG.to_owned(),
                        flag_at(crate::world::decor::border_flag_local(house_box)),
                        flag_heading,
                        [1.0; 3],
                        models3d::Family::Flag,
                        models3d::player_colour(building.owner),
                    );
                    border.extra[1] =
                        crate::world::decor::border_flag_row(m.enemy_distance) as f32 / 3.0;
                    out.push(border);
                }
            }
        }
    }
    // GFX-OBJ-1.5: the road flag is the element z_flag_00.
    let flag = scenery.element("z_flag_00");
    for (field, player) in world.flags() {
        let key = (Kind::Flag, field, 0);
        let [x, z] = frame.field_xz(field.0, field.1);
        // GFX-OBJ-1.4: the element's four draws, its offset, and the
        // flag's shift along z by its field.
        let z = z + flag_shift(field, world.terrain().width);
        let (file, (heading, scale), at) = match flag {
            Some(e) => {
                let (x, z) = (x + e.offset[0], z + e.offset[1]);
                (
                    e.mesh.clone(),
                    poses.pose(key, e),
                    [x, element_height(world, e, x, z), z],
                )
            }
            None => (models3d::FLAG.to_owned(), (0.0, [1.0; 3]), on_ground(x, z)),
        };
        out.push(plain(
            key,
            file,
            at,
            heading,
            scale,
            models3d::Family::Flag,
            models3d::player_colour(player),
        ));
    }
    // GFX-OBJ-6.2b: a waterway is drawn as buoys only (`street_buoy.GR2`,
    // a lit rigid mesh, not turned).
    for (n, road) in world.roads().filter(|r| r.water).enumerate() {
        let points: Vec<[f32; 2]> = road.path.iter().map(|p| frame.field_xz(p.0, p.1)).collect();
        for (i, p) in buoy_points(&points, WATER_LEVEL).into_iter().enumerate() {
            let mut buoy = plain(
                (Kind::Buoy, road.path[0], (n as u32) << 12 | i as u32),
                models3d::STREET_BUOY.into(),
                [p[0], WATER_LEVEL, p[1]],
                0.0,
                [1.0; 3],
                models3d::Family::Plain,
                [1.0; 3],
            );
            buoy.shadow = false;
            out.push(buoy);
        }
    }
    if borders {
        for field in world.terrain().fields() {
            if !world.terrain().has(field, state::BORDER) {
                continue;
            }
            let Some(owner) = world.field_owner(field) else {
                continue;
            };
            let water = world.terrain().has(field, state::WATER);
            let [x, z] = frame.field_xz(field.0, field.1);
            let (file, at) = if water {
                (models3d::BORDER_BUOY, [x, WATER_LEVEL, z])
            } else {
                (models3d::BORDER_STONE, on_ground(x, z))
            };
            out.push(plain(
                (Kind::Border, field, u32::from(water)),
                file.into(),
                at,
                0.0,
                [1.0; 3],
                models3d::Family::Border,
                models3d::player_colour(owner),
            ));
        }
    }
    // GFX-OBJ-1.3: deposits by their element; GFX-ANIM-3.8b growth times
    // the element's random size.
    for (field, deposit) in world.deposits() {
        let Some(look) = lookup.deposits.get(&deposit.kind) else {
            continue;
        };
        let key = (Kind::Deposit, field, deposit.kind);
        let element = scenery.element(&look.gfx_name);
        let (file, family, heading, scale) = match element {
            Some(e) => {
                let (yaw, scale) = poses.pose(key, e);
                (
                    e.mesh.clone(),
                    models3d::Family::of_shader(&e.shader),
                    yaw,
                    scale,
                )
            }
            None => match models3d::deposit_model(&look.gfx_name) {
                Some(file) => (file, models3d::Family::Plain, 0.0, [1.0; 3]),
                None => continue,
            },
        };
        // GFX-ANIM-3.8b: size 0 (being planted) draws at scale 0.
        let size = deposit_size(world, deposit.kind, deposit.state, look);
        if size <= 0.0 {
            continue;
        }
        let growth = growth_scale(size);
        // GFX-OBJ-1.4, T33: its fine position (TRN-FREE-5.4; a map's or
        // save's as stored) plus the element's offset, nothing at random.
        let [x, z] = frame.sample_xz(i64::from(deposit.position.0), i64::from(deposit.position.1));
        let at = match element {
            Some(e) => {
                let (x, z) = (x + e.offset[0], z + e.offset[1]);
                [x, element_height(world, e, x, z), z]
            }
            None => on_ground(x, z),
        };
        let mut placed = plain(
            key,
            file,
            at,
            heading,
            [0, 1, 2].map(|k| scale[k] * growth[k]),
            family,
            [0.0; 3],
        );
        placed.shadow = scenery
            .element(&look.gfx_name)
            .is_none_or(|e| e.shadow_cast);
        out.push(placed);
    }
    // The map's doodads: those in `scenery.doodads` (the view asks for the few
    // that changed by `some_doodads`).
    if !scenery.doodads.is_empty() {
        out.extend(
            scenery
                .doodads
                .iter()
                .enumerate()
                .filter_map(|(i, &d)| doodad(world, lookup, scenery, frame, i as u32, d, poses)),
        );
    }
    // ECO-SPC-3.2: the geologists' signs, as doodads on their field's sample.
    for (field, sign) in world.signs() {
        let Some(e) = lookup
            .doodads
            .get(&sign.doodad)
            .and_then(|gfx| scenery.element(&gfx.0))
        else {
            continue;
        };
        let sample = s2_sim::grid::field_sample(field);
        let key = (Kind::Sign, sample, sign.doodad);
        let (yaw, scale) = poses.pose(key, e);
        let [x, z] = frame.sample_xz(i64::from(sample.0), i64::from(sample.1));
        let (x, z) = (x + e.offset[0], z + e.offset[1]);
        let at = [x, element_height(world, e, x, z), z];
        out.push(plain(
            key,
            e.mesh.clone(),
            at,
            yaw,
            scale,
            models3d::Family::of_shader(&e.shader),
            [0.0; 3],
        ));
    }
    out
}

/// The doodad number `i` of the map, placed: by its element (GFX-OBJ-1.3), on
/// its sample, shifted by the element's offset. Those on the fields a site
/// cleared (ECO-PLACE-5.3) are gone for good, and so (GFX-OBJ-1.4a) are those
/// whose field a road, flag or building removed them from
/// (`World::doodads_removed`): a field that had the Street state removes the
/// doodads whose type deletes by street (`dd_setDeleteByStreet`), one that was
/// movement-blocked or carried a flag or building every doodad. A doodad
/// stands on the field of its sample (ECO-PLACE-5.3).
fn doodad(
    world: &World,
    lookup: &Lookup,
    scenery: &Scenery,
    frame: &Frame,
    i: u32,
    (type_id, sample): (u32, (u32, u32)),
    poses: &mut Poses,
) -> Option<Placed> {
    let gfx = lookup.doodads.get(&type_id)?;
    let e = scenery.element(&gfx.0)?;
    let cleared = world.decorations_cleared();
    let removed = world.doodads_removed();
    if !cleared.is_empty() || !removed.is_empty() {
        let (width, height) = (world.terrain().width, world.terrain().height);
        let field = decoration_field(sample, width, height);
        if cleared.contains(&field) {
            return None;
        }
        let which = removed.get(&field).copied().unwrap_or(0);
        if which & s2_sim::DOODADS_ALL != 0 || (gfx.1 && which & s2_sim::DOODADS_BY_STREET != 0) {
            return None;
        }
    }
    let key = (Kind::Doodad, sample, i);
    let (yaw, scale) = poses.pose(key, e);
    // GFX-OBJ-1.4: its sample plus the element's offset.
    let [x, z] = frame.sample_xz(i64::from(sample.0), i64::from(sample.1));
    let (x, z) = (x + e.offset[0], z + e.offset[1]);
    let at = [x, element_height(world, e, x, z), z];
    Some(Placed {
        key,
        file: e.mesh.clone(),
        units_per_metre: models3d::DEFAULT_SCALE,
        at,
        heading: yaw,
        scale,
        tilt: 0.0,
        tilt_axis: 0.0,
        tint: WHITE,
        family: models3d::Family::of_shader(&e.shader),
        player: [0.0; 3],
        extra: [0.0; 4],
        shadow: e.shadow_cast,
        pieces: None,
        plane_of: None,
        parts: Parts::All,
        sink: None,
    })
}

/// F55: the doodads numbered in `which` placed as [`objects`] places them, for
/// the view, which places the few that changed: `doodads` is the map's whole
/// list. Those not drawn now give nothing.
pub fn some_doodads(
    world: &World,
    lookup: &Lookup,
    scenery: &Scenery,
    frame: &Frame,
    doodads: &[(u32, (u32, u32))],
    which: &[u32],
    poses: &mut Poses,
) -> Vec<Placed> {
    which
        .iter()
        .filter_map(|&i| doodad(world, lookup, scenery, frame, i, doodads[i as usize], poses))
        .collect()
}

/// GFX-OBJ-1.4 (T32): the instances of a map or save as it is loaded, each
/// drawing its four numbers in the order the view learns of them: a save's
/// road flags (player 0 to 7, each in its list order); the file's deposits in
/// its list order; the doodads, list 1, 2, 3, each by id (a doodad whose type
/// names no element makes none); then what the game made, in the order it
/// made it: the flags in the order they were placed (a new game's
/// headquarters flags), then the deposits that were not in the file (field
/// by field, a stand-in for their making order) and the geologists' signs.
/// Instances that are not drawn now (a doodad under a road, a map tree
/// already felled when the view starts) draw too. Returns the instances in
/// that order.
pub(crate) fn load_poses(
    world: &World,
    lookup: &Lookup,
    scenery: &Scenery,
    poses: &mut Poses,
) -> Vec<InstanceKey> {
    let load = scenery.load;
    let mut order: Vec<InstanceKey> = Vec::new();
    let mut pose = |key: InstanceKey, e: &s2_assets::landscape::LandscapeElement| {
        poses.pose(key, e);
        order.push(key);
    };
    let flag = scenery.element("z_flag_00");
    let mut flags_done: HashSet<Point> = HashSet::new();
    if load.saved {
        if let Some(e) = flag {
            for player in 0..8 {
                for field in world.flags_in_order(s2_sim::PlayerId(player)) {
                    if flags_done.insert(field) {
                        pose((Kind::Flag, field, 0), e);
                    }
                }
            }
        }
    }
    let deposit_element = |kind: u32| {
        lookup
            .deposits
            .get(&kind)
            .and_then(|look| scenery.element(&look.gfx_name))
    };
    let mut deposits_done: HashSet<InstanceKey> = HashSet::new();
    let from_world: Vec<(Point, u32)>;
    let file_deposits = match load.deposits {
        Some(list) => list,
        None => {
            from_world = world.deposits().map(|(f, d)| (f, d.kind)).collect();
            &from_world
        }
    };
    for &(field, kind) in file_deposits {
        let key = (Kind::Deposit, field, kind);
        if let Some(e) = deposit_element(kind) {
            if deposits_done.insert(key) {
                pose(key, e);
            }
        }
    }
    let doodads = if load.doodads.is_empty() {
        scenery.doodads
    } else {
        load.doodads
    };
    for (i, &(type_id, sample)) in doodads.iter().enumerate() {
        if let Some(e) = lookup
            .doodads
            .get(&type_id)
            .and_then(|gfx| scenery.element(&gfx.0))
        {
            pose((Kind::Doodad, sample, i as u32), e);
        }
    }
    // What the game made after the load.
    if let Some(e) = flag {
        let placed = world.flag_order().iter().copied();
        for field in placed.chain(world.flags().map(|(f, _)| f)) {
            if world.flag_at(field).is_some() && flags_done.insert(field) {
                pose((Kind::Flag, field, 0), e);
            }
        }
    }
    for (field, deposit) in world.deposits() {
        let key = (Kind::Deposit, field, deposit.kind);
        if let Some(e) = deposit_element(deposit.kind) {
            if deposits_done.insert(key) {
                pose(key, e);
            }
        }
    }
    for (field, sign) in world.signs() {
        if let Some(e) = lookup
            .doodads
            .get(&sign.doodad)
            .and_then(|gfx| scenery.element(&gfx.0))
        {
            let sample = s2_sim::grid::field_sample(field);
            pose((Kind::Sign, sample, sign.doodad), e);
        }
    }
    order
}
