//! Settlers, animals and ships as skinned 3D models, kept in step with the world, with their clips.

use super::*;

/// One mover's entity and its last two tick positions and headings, its
/// parts, the pose they are drawn in and its animation state.
pub(crate) struct MoverState {
    pub(crate) entity: Entity,
    pub(crate) file: String,
    pub(crate) carried: Option<(String, Entity)>,
    /// GFX-OBJ-7.6: a rowing carrier's boat; his ware then lies in it.
    pub(crate) boat: Option<(String, Entity)>,
    /// GFX-ANIM-4.1, 4.1a: the last two tick targets.
    pub(crate) motion: crate::world::scene3d::Motion,
    pub(crate) heading_previous: f32,
    pub(crate) heading_current: f32,
    pub(crate) model: Option<crate::render::models3d::LoadedModel>,
    pub(crate) look: crate::render::models3d::Look,
    pub(crate) parts: Vec<Entity>,
    pub(crate) pose: Option<crate::animation::anim::PoseKey>,
    /// What the logic last said (GFX-ANIM-3).
    pub(crate) anim: crate::world::scene3d::MoverAnim,
    pub(crate) visual: crate::animation::settler_anim::SettlerVisual,
    /// A random idle clip playing (its file).
    pub(crate) idle: Option<String>,
    pub(crate) rng: crate::animation::settler_anim::Rng,
    /// The bone the carried ware is held at.
    pub(crate) link: Option<usize>,
    /// The ware lies in the boat (GFX-OBJ-7.6).
    pub(crate) in_boat: bool,
    /// A ship's bobbing (GFX-ANIM-7.2).
    pub(crate) bob: bool,
    pub(crate) ship: Option<usize>,
    /// GFX-ANIM-2.3: the pose's clock when its events were last checked.
    pub(crate) events_at: Option<(crate::animation::anim::PoseKey, f32)>,
    /// GFX-ANIM-4.3: the sideways offset of evading (to its right).
    pub(crate) evade: f32,
    /// GFX-WATER-2.6b: its render layers (a settler's from MIRROR DETAILS
    /// 3 on), which what it carries shares.
    pub(crate) layers: RenderLayers,
    /// A settler (not an animal, donkey or ship): it has a seen mark
    /// (GFX-VIEW-5.4).
    pub(crate) settler: bool,
    /// The tick's target position (game frame), for its field.
    pub(crate) at: [f32; 3],
    /// GFX-OBJ-3.2 (3a): a settler figure's atlas block, given back when
    /// the figure goes.
    pub(crate) block: Option<crate::render::settler_colors::FigureBlock>,
    /// Hook W30d: a project's figure: its look and walking phase.
    pub(crate) figure: Option<FigureState>,
}

/// Hook W30d: what the view keeps of a figure between frames.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FigureState {
    pub(crate) look: &'static crate::extend::FigureLook,
    /// The walking cycle's phase (0 .. 1), moved on by the ground covered.
    pub(crate) phase: f32,
    /// Where it was drawn in the last frame (game frame).
    pub(crate) last: Option<[f32; 3]>,
    /// The figure clip of the motion and whether it loops.
    pub(crate) clip: Option<(&'static str, bool)>,
}

/// Hook W30d: a step of more than this between two frames is a jump (a new
/// place), not ground covered.
const FIGURE_JUMP: f32 = 4.0;

/// Hook W30d: the figure clip of the tick's motion (`figure::motion`, from
/// the base index the game's choice gave), the file's own or its fallback
/// (`figure::choose`); a missing clip is logged once by file and name.
pub(crate) fn figure_motion(
    visual: &crate::animation::settler_anim::SettlerVisual,
    anim: &crate::world::scene3d::MoverAnim,
    carrying: bool,
    available: &[String],
    logged: &mut std::collections::HashSet<(String, String)>,
    model: &str,
) -> Option<(&'static str, bool)> {
    use crate::render::models3d::figure;
    let base = visual.choice.as_ref()?.base;
    let moving =
        matches!(anim, crate::world::scene3d::MoverAnim::Settler { state, .. } if state.moving);
    let (chosen, missing) = figure::choose(figure::motion(base, carrying), moving, available);
    if let Some(name) = missing {
        if logged.insert((model.to_owned(), name.to_owned())) {
            eprintln!(
                "figure {model}: no clip {name}, drawn with {} (hook W30d)",
                chosen.0
            );
        }
    }
    Some(chosen)
}

/// Hook W30d: the clip file a figure shows now: its motion's clip, a
/// walking one at the phase it has walked to.
pub(crate) fn figure_clip_file(figure: &FigureState) -> Option<(String, bool)> {
    use crate::render::models3d::figure;
    let (clip, looping) = figure.clip?;
    let phase = figure::walks(clip).then(|| figure::phase_index(figure.phase));
    Some((figure::clip_file(&figure.look.model, clip, phase), looping))
}

impl MoverState {
    /// The clip on screen: a random idle clip, else the resolved one.
    fn clip(&self) -> Option<(String, bool)> {
        if let Some(idle) = &self.idle {
            return Some((idle.clone(), false));
        }
        match &self.anim {
            crate::world::scene3d::MoverAnim::Clip(c) => c.clone(),
            crate::world::scene3d::MoverAnim::Unique(f) => Some((f.clone(), false)),
            crate::world::scene3d::MoverAnim::Settler { .. } => self.visual.clip.clone(),
        }
    }
}

/// A mover with a settler's animation that is not a donkey: a settler,
/// carrier, soldier, builder or leveller (GFX-VIEW-5.4, GFX-WATER-2.6b).
pub(crate) fn is_settler(anim: &crate::world::scene3d::MoverAnim) -> bool {
    matches!(
        anim,
        crate::world::scene3d::MoverAnim::Settler { donkey: false, .. }
    )
}

/// GFX-VIEW-5.4: every settler's seen mark, kept from frame to frame.
#[derive(Debug, Clone, Default)]
pub struct MarkState {
    /// (a): one mark per settler, kept while it changes jobs.
    pub(crate) marks: std::collections::HashMap<crate::world::scene3d::SeenKey, bool>,
    /// Whose mark each mover shows, as of the last frame.
    movers:
        std::collections::HashMap<crate::world::scene3d::MoverId, crate::world::scene3d::SeenKey>,
    /// (d): the explorations seen in the last frame.
    explorations: Option<u64>,
    /// The next frame refreshes every settler in the drawn area.
    forced_next: bool,
    /// Frames made so far: the first is the settler renderer's first.
    frames: u64,
}

impl MarkState {
    /// One frame of GFX-VIEW-5.4 (b): each settler's mark (a: clear when it
    /// first shows) is refreshed by [`refresh_seen`] from `explored` at its
    /// position when its field lies in the frame's drawn field rectangle
    /// `area` or the frame is the renderer's first, and it walks or the frame
    /// is forced; one inside a building keeps its mark. (d) An exploration by
    /// any player since the last frame (`explorations`, `World::explorations`)
    /// forces the next. (f) Without a rectangle (no terrain cell drawn) the
    /// settlers are not gone through at all: nothing is refreshed and the
    /// frame does not count.
    pub fn frame(
        &mut self,
        explorations: u64,
        points: &[crate::world::scene3d::SeenPoint],
        area: Option<FieldRect>,
        explored: impl Fn([f32; 3]) -> bool,
    ) {
        let Some(area) = area else {
            return;
        };
        let first = self.frames == 0;
        self.frames += 1;
        let forced = self.forced_next;
        self.forced_next = self.explorations.is_some_and(|e| e != explorations);
        self.explorations = Some(explorations);
        let present: std::collections::HashSet<_> = points.iter().map(|p| p.key).collect();
        self.marks.retain(|key, _| present.contains(key));
        self.movers = points.iter().map(|p| (p.id, p.key)).collect();
        for p in points {
            let mark = self.marks.entry(p.key).or_insert(false);
            if !p.shown {
                continue;
            }
            let in_area = area.contains(p.field);
            *mark = refresh_seen(*mark, explored(p.at), in_area, first, p.walks, forced);
        }
    }

    /// A settler's mark (none: not a settler with a mark).
    pub fn mark(&self, id: crate::world::scene3d::MoverId) -> Option<bool> {
        self.marks.get(self.movers.get(&id)?).copied()
    }
}

/// GFX-VIEW-5.4 (g): the marks of a game a tool plays ahead (`--run`,
/// `--until-built`, a scene, the look comparison): the camera stands on the
/// game's start view for every tick played (GFX-VIEW-3.6 a: the local
/// player's headquarters at zoom 3, with the window's aspect), one frame
/// before the first tick and one after each, as the original's recording
/// ran; the view then takes them over, its first frame being the jump.
#[derive(Debug, Clone)]
pub struct PlayAhead {
    pub marks: MarkState,
    /// GFX-OBJ-2.2a: the house slots of the ticks played, one view update
    /// a tick (a tick's frame time each), as the original's view ran.
    pub slots: crate::world::scene3d::HouseSlots,
    /// GFX-ANIM-3.8a, 3.8c: the fall axes of the trees felled in the ticks
    /// played: the woodcutter's view object ticks for every settler, seen
    /// or not, so a tree felled while the tool played falls toward him.
    pub fall_axes: std::collections::HashMap<s2_sim::grid::Point, f32>,
    frame: Frame,
    pub(crate) clip_from_world: Mat4,
    /// Every field counts as explored (fog of war off or not drawn).
    all_explored: bool,
    /// The drawn field rectangle and the height changes it was taken at
    /// (`World::height_changes`): the camera stands still, so it changes
    /// only when the ground does.
    drawn: Option<(u64, Option<FieldRect>)>,
}

impl PlayAhead {
    /// The start view of `world` in a window of `aspect`.
    pub fn new(world: &s2_sim::World, aspect: f32, all_explored: bool) -> Self {
        let frame = Frame::new(world.terrain().width, world.terrain().height);
        let headquarters =
            super::headquarters_of(world, crate::game::PLAYER).map(|at| frame.field_xz(at.0, at.1));
        let mut rig = world3d::CameraRig::start(headquarters);
        let eye = rig.update(world.terrain());
        Self {
            marks: MarkState::default(),
            slots: crate::world::scene3d::HouseSlots::default(),
            fall_axes: Default::default(),
            frame,
            clip_from_world: clip_from_world(eye, aspect),
            all_explored,
            drawn: None,
        }
    }

    /// The drawn field rectangle of the last frame.
    #[cfg(test)]
    pub fn drawn_rect(&self) -> Option<FieldRect> {
        self.drawn.and_then(|(_, area)| area)
    }

    /// A frame with the world as it stands.
    pub fn frame(&mut self, world: &s2_sim::World) {
        let houses = crate::world::scene3d::houses(world);
        self.slots
            .update(Some((world.tick(), &houses)), crate::game::TICK_SECONDS);
        crate::world::scene3d::update_fall_axes(world, &self.frame, &mut self.fall_axes);
        let points = crate::world::scene3d::seen_points(world, &self.frame);
        let (frame, clip, all) = (self.frame, self.clip_from_world, self.all_explored);
        let (width, height) = (world.terrain().width, world.terrain().height);
        let changes = world.height_changes();
        let area = match self.drawn {
            Some((at, area)) if at == changes => area,
            _ => {
                let area = drawn_field_rect(world.terrain(), &frame, clip);
                self.drawn = Some((changes, area));
                area
            }
        };
        self.marks
            .frame(world.explorations(), &points, area, |[x, _, z]| {
                all || {
                    let [u, v] = frame.to_plane(x, z);
                    crate::render::terrain::nearest_field(u, v, width, height)
                        .is_some_and(|f| world.is_explored(crate::game::PLAYER, (f.x, f.y)))
                }
            });
    }
}

/// The 3D camera's matrix from the game frame to clip space for `eye`
/// (GFX-VIEW-3.5: the horizontal 50 degrees, near plane 1), as the map
/// camera's projection makes it.
pub fn clip_from_world(eye: world3d::Eye, aspect: f32) -> Mat4 {
    let position = Vec3::from(world3d::to_bevy(eye.position));
    let direction = Vec3::from(world3d::to_bevy(eye.direction));
    let camera = Transform::from_translation(position).looking_to(direction, Vec3::Y);
    let projection = PerspectiveProjection {
        fov: world3d::vertical_fov(aspect),
        aspect_ratio: aspect,
        near: world3d::NEAR,
        far: world3d::FAR,
        ..default()
    };
    bevy::camera::CameraProjection::get_clip_from_view(&projection) * camera.to_matrix().inverse()
}

/// GFX-VIEW-5.4 (b), (f) (Q-1031 answered): a frame's drawn field
/// rectangle, columns and rows inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldRect {
    pub columns: [u32; 2],
    pub rows: [u32; 2],
}

impl FieldRect {
    /// A field inside, borders included.
    pub fn contains(&self, (x, y): (u32, u32)) -> bool {
        (self.columns[0]..=self.columns[1]).contains(&x)
            && (self.rows[0]..=self.rows[1]).contains(&y)
    }
}

/// GFX-VIEW-5.4 (f): a terrain cell of 4 x 4 fields: its world box
/// (heights included) and the fields it covers, both ends included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellBox {
    x: [f32; 2],
    y: [f32; 2],
    z: [f32; 2],
    fields: FieldRect,
}

/// GFX-VIEW-5.4 (f) (Q-1031 answered): the terrain's cells of 4 x 4 fields:
/// cell (i, j) covers the fields of columns 4i to min(4i + 4, width - 1)
/// and rows 4j to min(4j + 4, height - 1), both ends included, and its box
/// spans its samples and their heights.
pub fn cell_boxes(terrain: &s2_sim::map::Terrain, frame: &Frame) -> Vec<CellBox> {
    use crate::render::terrain3d;
    let (cells_x, cells_y) = terrain3d::cell_count(terrain.width, terrain.height);
    let mut out = Vec::with_capacity((cells_x * cells_y) as usize);
    for cy in 0..cells_y {
        for cx in 0..cells_x {
            let ([c0, c1], [r0, r1]) = terrain3d::cell_samples(frame, cx, cy);
            let (mut low, mut high) = (i32::MAX, i32::MIN);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    let h = terrain.raw(c, r);
                    low = low.min(h);
                    high = high.max(h);
                }
            }
            let corner = |c: u32, r: u32| frame.sample_xz(i64::from(c), i64::from(r));
            let xs = [
                corner(c0, r0),
                corner(c1, r0),
                corner(c0, r1),
                corner(c1, r1),
            ];
            let range = |k: usize| {
                [
                    xs.iter().map(|p| p[k]).fold(f32::INFINITY, f32::min),
                    xs.iter().map(|p| p[k]).fold(f32::NEG_INFINITY, f32::max),
                ]
            };
            out.push(CellBox {
                x: range(0),
                y: [
                    low as f32 * world3d::HEIGHT_SCALE,
                    high as f32 * world3d::HEIGHT_SCALE,
                ],
                z: range(1),
                fields: FieldRect {
                    columns: [cx * 4, (cx * 4 + 4).min(terrain.width - 1)],
                    rows: [cy * 4, (cy * 4 + 4).min(terrain.height - 1)],
                },
            });
        }
    }
    out
}

/// GFX-VIEW-5.4 (b), (f) (Q-1031 answered): the drawn field rectangle of a
/// frame seen through `clip_from_world`: the smallest first and the largest
/// last column and row of the cells ([`cell_boxes`]) whose box the camera
/// sees. None when it sees no cell.
pub fn drawn_rect_of(cells: &[CellBox], clip_from_world: Mat4) -> Option<FieldRect> {
    cells
        .iter()
        .filter(|c| box_seen(clip_from_world, c.x, c.y, c.z))
        .map(|c| c.fields)
        .reduce(|a, b| FieldRect {
            columns: [
                a.columns[0].min(b.columns[0]),
                a.columns[1].max(b.columns[1]),
            ],
            rows: [a.rows[0].min(b.rows[0]), a.rows[1].max(b.rows[1])],
        })
}

/// [`drawn_rect_of`] the terrain's cells as they are.
pub fn drawn_field_rect(
    terrain: &s2_sim::map::Terrain,
    frame: &Frame,
    clip_from_world: Mat4,
) -> Option<FieldRect> {
    drawn_rect_of(&cell_boxes(terrain, frame), clip_from_world)
}

/// A box of the game frame seen through `clip_from_world`: not wholly
/// beyond one plane of the view volume (the screen's four edges and the
/// near plane; the far plane is at infinity).
fn box_seen(clip_from_world: Mat4, x: [f32; 2], y: [f32; 2], z: [f32; 2]) -> bool {
    let corners: Vec<Vec4> = (0..8)
        .map(|i| {
            let at = [x[i & 1], y[(i >> 1) & 1], z[(i >> 2) & 1]];
            clip_from_world * Vec3::from(world3d::to_bevy(at)).extend(1.0)
        })
        .collect();
    let outside = |test: &dyn Fn(&Vec4) -> bool| corners.iter().all(test);
    !(outside(&|p| p.x < -p.w)
        || outside(&|p| p.x > p.w)
        || outside(&|p| p.y < -p.w)
        || outside(&|p| p.y > p.w)
        || outside(&|p| p.z > p.w))
}

/// A point of the game frame inside the picture of `clip_from_world` (in
/// front of the camera, within the screen's edges).
#[cfg(test)]
pub fn in_picture(clip_from_world: Mat4, at: [f32; 3]) -> bool {
    let p = clip_from_world * Vec3::from(world3d::to_bevy(at)).extend(1.0);
    if p.w <= 0.0 {
        return false;
    }
    let ndc = p.truncate() / p.w;
    ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0 && ndc.z >= 0.0
}

/// GFX-VIEW-5.4: every settler's seen mark. A settler is drawn only while
/// its mark is set.
#[derive(Resource, Default)]
pub struct SeenMarks {
    state: MarkState,
    /// The game (`Viewer::loads`) the marks belong to.
    game: Option<u64>,
    /// The settlers' points of the tick `points_tick`: they change only
    /// with the tick, so a frame between two ticks reuses them.
    points: Vec<crate::world::scene3d::SeenPoint>,
    points_tick: Option<u64>,
    /// The terrain cells' boxes (5.4 f) and the height changes they were
    /// taken at (`World::height_changes`).
    cells: Vec<CellBox>,
    cells_at: Option<u64>,
}

/// GFX-VIEW-5.4 (b): in a frame, a settler's mark becomes "the local
/// player's exploration layer is set at its field" when the frame draws its
/// field (`in_area`) or is the settler renderer's first in the game
/// (`first`), and it walks or the frame is forced; else it keeps its value.
/// (a) A new settler starts unseen (`mark` false).
pub fn refresh_seen(
    mark: bool,
    explored: bool,
    in_area: bool,
    first: bool,
    walking: bool,
    forced: bool,
) -> bool {
    if (in_area || first) && (walking || forced) {
        explored
    } else {
        mark
    }
}

/// GFX-VIEW-5.4: the settlers' seen marks for this frame. (d) The layer is
/// the local player's exploration layer, every field explored with fog of
/// war off; any player's exploration (`World::explorations` changed) marks
/// the next frame, which refreshes every settler in the drawn area: the
/// field rectangle of the terrain cells the map camera sees
/// ([`drawn_field_rect`], 5.4 f, Q-1031 answered). (g) A game
/// a tool played ahead brings the marks of its play ([`PlayAhead`]); the
/// renderer's first frame was then before the first tick played.
pub fn seen_marks(
    mut viewer: ResMut<crate::game::Viewer>,
    view: Res<View3d>,
    movers: Res<Movers>,
    mut seen: ResMut<SeenMarks>,
    cameras: Query<(&Camera, &GlobalTransform), With<MapCamera3d>>,
    mut visibility: Query<&mut Visibility, With<Object3d>>,
) {
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    if seen.game != Some(viewer.loads) {
        *seen = SeenMarks {
            state: viewer
                .ahead_marks
                .take()
                .map(|ahead| ahead.marks)
                .unwrap_or_default(),
            game: Some(viewer.loads),
            ..SeenMarks::default()
        };
    }
    let viewer = &*viewer;
    let Some(world) = &viewer.world else { return };
    let camera = cameras.iter().find(|(c, _)| c.is_active);
    let (width, height) = (world.terrain().width, world.terrain().height);
    let frame = view.frame;
    if seen.points_tick != Some(world.tick()) {
        seen.points = crate::world::scene3d::seen_points(world, &frame);
        seen.points_tick = Some(world.tick());
    }
    if seen.cells_at != Some(world.height_changes()) {
        seen.cells = cell_boxes(world.terrain(), &frame);
        seen.cells_at = Some(world.height_changes());
    }
    let seen = &mut *seen;
    let area = camera.and_then(|(c, t)| {
        drawn_rect_of(&seen.cells, c.clip_from_view() * t.to_matrix().inverse())
    });
    seen.state
        .frame(world.explorations(), &seen.points, area, |[x, _, z]| {
            let [u, v] = frame.to_plane(x, z);
            crate::render::terrain::nearest_field(u, v, width, height)
                .is_some_and(|f| !viewer.fogged(world, (f.x, f.y)))
        });
    for (id, state) in &movers.states {
        if !state.settler {
            continue;
        }
        let Some(mark) = seen.state.mark(*id) else {
            continue;
        };
        if let Ok(mut shown) = visibility.get_mut(state.entity) {
            let want = if mark {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            shown.set_if_neq(want);
        }
    }
}

/// The pose a mover's clip asks for: a unique clip on a pose of
/// its own, the others by the pool's rule (GFX-ANIM-2.4: looping clips
/// shared, play-once clips private).
pub(crate) fn wanted_pose(
    model: &crate::render::models3d::LoadedModel,
    clip: Option<(String, bool)>,
    unique_clip: bool,
) -> Option<crate::animation::anim::PoseWant> {
    let want = pose_key(model, clip.as_ref())?;
    Some(if unique_clip { want.private() } else { want })
}

/// The movers on the map and the tick they were taken at.
#[derive(Resource, Default)]
pub struct Movers {
    pub(crate) states: std::collections::HashMap<crate::world::scene3d::MoverId, MoverState>,
    pub(crate) tick: Option<u64>,
    pub(crate) ship_phase: f32,
    /// Hook W30d: the figures' missing clips already logged (file, clip).
    pub(crate) figure_missing: std::collections::HashSet<(String, String)>,
}

/// An entity with one child per part.
pub(crate) fn spawn_parts(
    commands: &mut Commands,
    transform: Transform,
    parts: crate::animation::animate3d::PoseParts,
    layers: &RenderLayers,
) -> (Entity, Vec<Entity>) {
    let mut children = Vec::new();
    let entity = commands
        .spawn((Object3d, transform, Visibility::default()))
        .with_children(|c| {
            for (mesh, material) in parts {
                children.push(
                    c.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        // GFX-WATER-2.6b: in the mirror image of a flat
                        // view with every detail (settlers from n = 3).
                        layers.clone(),
                    ))
                    .id(),
                );
            }
        })
        .id();
    (entity, children)
}

/// A clip on a model, asked of the pool of its skeleton's model file
/// (GFX-ANIM-2.4). `None` without a skeleton or clip.
pub(crate) fn pose_key(
    model: &crate::render::models3d::LoadedModel,
    clip: Option<&(String, bool)>,
) -> Option<crate::animation::anim::PoseWant> {
    model.skeleton.as_ref()?;
    let (clip, looping) = clip?;
    let key = |f: &str| f.replace('\\', "/").to_ascii_lowercase();
    Some(crate::animation::anim::PoseWant::new(
        &key(&model.skeleton_file),
        &key(clip),
        *looping,
    ))
}

pub(crate) type ObjectAssets<'a> = (
    &'a mut Assets<Image>,
    &'a mut Assets<crate::render::models3d::ObjectMaterial>,
);

/// Move a mover's parts to the pose of the clip it should show now
/// (GFX-ANIM-3.2: detach from the old pose, attach to or create the new).
/// `restart`: a new request of the same clip (GFX-ANIM-3.8f a) leaves the
/// pose and takes one again, as the Stand tick between two requests does:
/// a play-once clip then starts at 0 in a pose of its own (2.4 b, T19b).
pub(crate) fn repose(
    state: &mut MoverState,
    restart: bool,
    animator: &mut crate::animation::animate3d::Animator,
    library: &mut crate::render::models3d::ModelLibrary,
    part_materials: &mut Query<&mut MeshMaterial3d<crate::render::models3d::ObjectMaterial>>,
    assets: ObjectAssets,
) {
    let Some(model) = state.model.as_ref() else {
        return;
    };
    let unique_clip =
        state.idle.is_none() && matches!(state.anim, crate::world::scene3d::MoverAnim::Unique(_));
    let wanted = wanted_pose(model, state.clip(), unique_clip);
    let same = match (&state.pose, &wanted) {
        (Some(pose), Some(want)) => pose.holds(want),
        (None, None) => true,
        _ => false,
    };
    if same && !restart {
        return;
    }
    // The model copied only for a change (most frames of most settlers
    // keep their pose).
    let model = model.clone();
    // GFX-ANIM-2.4 (c): leave the old pose before taking the new (T19c).
    if let Some(old) = state.pose.take() {
        animator.detach(&old, &state.look);
    }
    // Hook W30d: a figure's shared poses are held for good.
    if let (Some(_), Some(want)) = (&state.figure, &wanted) {
        let (images, materials) = (&mut *assets.0, &mut *assets.1);
        animator.pin(want, &model, &state.look, library, images, materials);
    }
    let (pose, parts) = parts_in(
        animator,
        library,
        &model,
        wanted.as_ref(),
        &state.look,
        assets,
    );
    for (&child, (_, material)) in state.parts.iter().zip(parts) {
        if let Ok(mut m) = part_materials.get_mut(child) {
            m.0 = material;
        }
    }
    state.pose = pose;
}

/// The pose taken and the materials of a model's parts in it (the rest
/// pose without one).
pub(crate) fn parts_in(
    animator: &mut crate::animation::animate3d::Animator,
    library: &mut crate::render::models3d::ModelLibrary,
    model: &crate::render::models3d::LoadedModel,
    want: Option<&crate::animation::anim::PoseWant>,
    look: &crate::render::models3d::Look,
    assets: ObjectAssets,
) -> (
    Option<crate::animation::anim::PoseKey>,
    crate::animation::animate3d::PoseParts,
) {
    let (images, materials) = assets;
    match want {
        Some(want) => {
            let (pose, parts) = animator.attach(want, model, look, library, images, materials);
            (Some(pose), parts)
        }
        None => (
            None,
            (0..model.parts.len())
                .filter_map(|i| {
                    let m = library.rest_material(model, i, look, images, materials)?;
                    Some((model.parts[i].mesh.clone(), m))
                })
                .collect(),
        ),
    }
}

/// Settlers, carriers, animals and ships: at each new tick their targets
/// move on (the old target kept), headings turn by at most 0.4398 rad a
/// tick and the clip is chosen (GFX-ANIM-3.1, 3.2, 5.1); every frame they
/// are drawn between the two by the share of the tick that has passed
/// (GFX-ANIM-4.1, 4.2), random idle clips run (GFX-ANIM-3.3), a dying
/// settler sinks (3.7) and the carried ware follows its bone (3.5). The
/// idle waits, the sinking and the evading run once per logic tick, the
/// ships' bobbing on game time (TOOL-trace TRACE-5.3).
#[allow(clippy::too_many_arguments)]
pub fn sync_movers(
    mut commands: Commands,
    clock: Res<crate::game::GameClock>,
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    mut movers: ResMut<Movers>,
    placements: Res<Placements>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut animator: ResMut<crate::animation::animate3d::Animator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::render::models3d::ObjectMaterial>>,
    mut transforms: Query<&mut Transform, With<Object3d>>,
    mut part_materials: Query<&mut MeshMaterial3d<crate::render::models3d::ObjectMaterial>>,
    mut sounds: ResMut<crate::audio::SoundQueue>,
) {
    use crate::world::scene3d::{ease_heading, lerp_heading, MoverAnim, TURN_PER_TICK};
    if !view.active || view.built != Some(viewer.loads) {
        return;
    }
    let Some(world) = &viewer.world else { return };
    let library = &mut *library;
    let animator = &mut *animator;
    let tick = world.tick();
    if movers.tick != Some(tick) {
        let ticks = movers.tick.map_or(1, |t| tick.saturating_sub(t).max(1));
        movers.tick = Some(tick);
        let now = crate::world::scene3d::movers(
            world,
            &placements.lookup,
            &library.tables,
            &library.animations,
            &view.frame,
            // GFX-OBJ-7.5 (c): the tool is hidden under the local player's
            // fog of war.
            &|field| viewer.fogged(world, field),
        );
        let mut keep = std::collections::HashSet::new();
        let mut figure_missing = std::mem::take(&mut movers.figure_missing);
        for mover in now {
            keep.insert(mover.id);
            let reuse = movers
                .states
                .get(&mover.id)
                .is_some_and(|s| s.file == mover.file);
            if !reuse {
                if let Some(old) = movers.states.remove(&mover.id) {
                    if let Some(pose) = &old.pose {
                        animator.detach(pose, &old.look);
                    }
                    if let Some(block) = &old.block {
                        library.release_figure(block);
                    }
                    commands.entity(old.entity).despawn();
                }
                let heading = mover.heading.unwrap_or(0.0);
                let transform = Transform::from_translation(Vec3::from(world3d::to_bevy(mover.at)))
                    .with_rotation(Quat::from_rotation_y(world3d::bevy_yaw(heading)));
                // Hook W30d: a project's figure, with its far version.
                let model = match mover.figure {
                    Some(look) => library.figure_model(look, &mut meshes, &mut images),
                    None => library.skinned_model(
                        &mover.file,
                        mover.skeleton.as_deref(),
                        mover.units_per_metre,
                        &mut meshes,
                        &mut images,
                    ),
                };
                let Some(model) = model else {
                    continue;
                };
                let mut block = None;
                let look = match &mover.paint {
                    Some(paint) => {
                        let (look, used) = library.settler_look(paint, &mut images);
                        block = used;
                        look
                    }
                    None => {
                        let mut look =
                            crate::render::models3d::Look::family(mover.family, mover.player);
                        look.tint = mover.tint;
                        // GFX-OBJ-2.8a: a ship's parts are drawn with their
                        // part number as the player colour weight (hull 0,
                        // sails 1; `Look::part_extra`); the frame is one part.
                        look.extra[0] = 1.0;
                        look
                    }
                };
                // The first clip is chosen before the parts are spawned
                // (their materials are those of its pose).
                let mut visual = crate::animation::settler_anim::SettlerVisual::new();
                choose_clip(
                    &mut visual,
                    &mover.anim,
                    library,
                    Some((&model, mover.at[1])),
                );
                let carrying = mover.carried.is_some() && mover.boat.is_none();
                let figure = mover.figure.map(|look| {
                    let clip = figure_motion(
                        &visual,
                        &mover.anim,
                        carrying,
                        library.figure_clips(&look.model),
                        &mut figure_missing,
                        &look.model,
                    );
                    FigureState {
                        look,
                        phase: 0.0,
                        last: None,
                        clip,
                    }
                });
                if let Some(f) = &figure {
                    visual.clip = figure_clip_file(f);
                }
                let first = match &mover.anim {
                    MoverAnim::Clip(c) => c.clone(),
                    MoverAnim::Unique(f) => Some((f.clone(), false)),
                    MoverAnim::Settler { .. } => visual.clip.clone(),
                };
                let want = wanted_pose(&model, first, matches!(mover.anim, MoverAnim::Unique(_)));
                if let (Some(_), Some(w)) = (&figure, &want) {
                    animator.pin(w, &model, &look, library, &mut images, &mut materials);
                }
                let (pose, parts) = parts_in(
                    animator,
                    library,
                    &model,
                    want.as_ref(),
                    &look,
                    (&mut images, &mut materials),
                );
                // GFX-WATER-2.6b: a settler and what it carries are in the
                // mirror from MIRROR DETAILS 3 on, the rest at 4.
                let settler = is_settler(&mover.anim);
                let layers = if settler {
                    mirrored_settler_layers()
                } else {
                    mirrored_object_layers()
                };
                let (entity, children) = spawn_parts(&mut commands, transform, parts, &layers);
                // Hook W30d: a figure shows its model or its far version by
                // its height on the screen, as a project's tree does
                // (`tree_versions`); the model until it is decided.
                if let Some(look) = mover.figure {
                    let fars: Vec<bool> = model
                        .parts
                        .iter()
                        .map(|p| p.facet.as_ref().is_some_and(|f| f.far))
                        .collect();
                    if fars.iter().any(|&f| f) {
                        for (&child, &far) in children.iter().zip(&fars) {
                            let shown = if far {
                                Visibility::Hidden
                            } else {
                                Visibility::Inherited
                            };
                            commands.entity(child).insert((TreeVersion { far }, shown));
                        }
                        commands.entity(entity).insert(ProjectTree {
                            extent: [0.0, model.extent[1], 0.0],
                            far_below: look.far_below,
                            far: None,
                        });
                    }
                }
                let link = mover
                    .link
                    .as_deref()
                    .and_then(|bone| model.skeleton.as_ref().and_then(|s| s.find(bone)));
                let seed = match mover.id {
                    crate::world::scene3d::MoverId::Settler(i)
                    | crate::world::scene3d::MoverId::Carrier(i)
                    | crate::world::scene3d::MoverId::Animal(i)
                    | crate::world::scene3d::MoverId::Ship(i) => i as u64,
                    crate::world::scene3d::MoverId::Builder(p)
                    | crate::world::scene3d::MoverId::Leveller(p) => {
                        u64::from(p.0) << 16 | u64::from(p.1)
                    }
                };
                movers.states.insert(
                    mover.id,
                    MoverState {
                        entity,
                        file: mover.file.clone(),
                        carried: None,
                        boat: None,
                        motion: crate::world::scene3d::Motion::new(mover.at),
                        heading_previous: heading,
                        heading_current: heading,
                        model: Some(model),
                        look,
                        parts: children,
                        pose,
                        anim: mover.anim.clone(),
                        visual,
                        idle: None,
                        rng: crate::animation::settler_anim::Rng(
                            seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
                        ),
                        link,
                        in_boat: false,
                        events_at: None,
                        bob: mover.bob,
                        evade: 0.0,
                        layers,
                        settler,
                        at: mover.at,
                        block,
                        figure,
                        ship: match mover.id {
                            crate::world::scene3d::MoverId::Ship(i) => Some(i),
                            _ => None,
                        },
                    },
                );
            }
            let fresh = !reuse;
            let Some(state) = movers.states.get_mut(&mover.id) else {
                continue;
            };
            // GFX-ANIM-4.1a: a settler standing for a second tick keeps
            // his last target; the site's builder and leveller (drawn from
            // the site, not walked) and animals and ships always follow.
            let held = matches!(
                mover.id,
                crate::world::scene3d::MoverId::Settler(_)
                    | crate::world::scene3d::MoverId::Carrier(_)
            ) && matches!(&mover.anim, MoverAnim::Settler { state: s, .. } if !s.moving);
            state.motion.tick(mover.at, !held);
            state.heading_previous = state.heading_current;
            if let Some(target) = mover.heading {
                state.heading_current = ease_heading(
                    state.heading_current,
                    target,
                    TURN_PER_TICK * ticks.min(8) as f32,
                );
            }
            // GFX-ANIM-3.1, 3.2: the tick's clip choice.
            state.anim = mover.anim.clone();
            state.at = mover.at;
            let swim = state.model.as_ref().map(|m| (m, mover.at[1]));
            let mut changed = choose_clip(&mut state.visual, &mover.anim, library, swim);
            // Hook W30d: a figure's own clip for the motion.
            if let Some(figure) = state.figure.as_mut() {
                let carrying = mover.carried.is_some() && mover.boat.is_none();
                let clip = figure_motion(
                    &state.visual,
                    &mover.anim,
                    carrying,
                    library.figure_clips(&figure.look.model),
                    &mut figure_missing,
                    &figure.look.model,
                );
                // A motion of another clip (carrying or not) is a change,
                // as the original's good-dependent clips are.
                changed |= clip != figure.clip;
                figure.clip = clip;
                state.visual.clip = figure_clip_file(figure);
            }
            if changed {
                state.idle = None;
            }
            state.bob = mover.bob;
            // A mover spawned this tick already wears its first pose (its
            // parts are not in the world yet to be changed).
            if !fresh {
                repose(
                    state,
                    changed,
                    animator,
                    library,
                    &mut part_materials,
                    (&mut images, &mut materials),
                );
            }
            // The held ware or tool and a rowing carrier's boat, children of
            // the mover (GFX-OBJ-7), items at their own size.
            let in_boat = mover.boat.is_some();
            let settings = &library.tables.character_settings;
            // GFX-OBJ-7.6 in the game frame, carried into the meshes' frame
            // (Bevy's z flip on both sides).
            let flip = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
            let ware_in_boat =
                flip * crate::world::carry::boat_ware_matrix(
                    settings.good_offset,
                    settings.good_rotation,
                ) * flip;
            let boat_at = flip * crate::world::carry::boat_matrix(settings.boat_offset) * flip;
            for (slot, wanted, at) in [
                (
                    &mut state.carried,
                    &mover.carried,
                    in_boat.then_some(ware_in_boat),
                ),
                (&mut state.boat, &mover.boat, Some(boat_at)),
            ] {
                let have = slot.as_ref().map(|(f, _)| f.clone());
                if have != *wanted {
                    if let Some((_, entity)) = slot.take() {
                        commands.entity(entity).despawn();
                    }
                    if let Some(file) = wanted {
                        if let Some(model) = library.model(
                            file,
                            crate::render::models3d::DEFAULT_SCALE,
                            &mut meshes,
                            &mut images,
                        ) {
                            let (_, parts) = parts_in(
                                animator,
                                library,
                                &model,
                                None,
                                &crate::render::models3d::Look::plain([1.0; 4]),
                                (&mut images, &mut materials),
                            );
                            let (entity, _) = spawn_parts(
                                &mut commands,
                                Transform::IDENTITY,
                                parts,
                                &state.layers,
                            );
                            commands.entity(state.entity).add_child(entity);
                            *slot = Some((file.clone(), entity));
                        }
                    }
                }
                // Fixed in the boat: set once per tick.
                if let (Some((_, entity)), Some(m)) = (slot.as_ref(), at) {
                    commands.entity(*entity).insert(Transform::from_matrix(m));
                }
            }
            state.in_boat = in_boat;
        }
        movers.figure_missing = figure_missing;
        let gone: Vec<_> = movers
            .states
            .keys()
            .filter(|id| !keep.contains(*id))
            .copied()
            .collect();
        for id in gone {
            if let Some(state) = movers.states.remove(&id) {
                if let Some(pose) = &state.pose {
                    animator.detach(pose, &state.look);
                }
                if let Some(block) = &state.block {
                    library.release_figure(block);
                }
                commands.entity(state.entity).despawn();
            }
        }
        // GFX-OBJ-3.2 (3): the blocks of the figures made this tick.
        library.flush_atlas(&mut images);
    }
    // TRACE-5.3: the per-tick visuals run once for each tick of this frame,
    // a tick length each.
    let ticks = clock.ticks;
    let tick = crate::game::TICK_SECONDS;
    let t = viewer.tick_fraction.clamp(0.0, 1.0);
    let sets = library.animations.random.clone();
    // GFX-ANIM-7.2: the ships' phase, on game time (TRACE-5.3); 7.2a: a
    // frame draws with the phase before its own advance, the last frame's.
    let ship_phase = movers.ship_phase;
    movers.ship_phase = crate::animation::object_anim::ship_phase(clock.world_seconds);
    // GFX-ANIM-4.3: settlers meeting step aside, once per tick.
    for _ in 0..ticks {
        let walkers: Vec<_> = movers
            .states
            .iter()
            .filter(|(_, s)| matches!(s.anim, MoverAnim::Settler { .. }))
            .map(|(id, s)| {
                let at = s.motion.at(t);
                let at = [at[0], at[2]];
                let evader = crate::world::scene3d::Evader {
                    at,
                    heading: lerp_heading(s.heading_previous, s.heading_current, t),
                    moving: s.motion.slides(),
                    offset: s.evade,
                };
                (*id, evader)
            })
            .collect();
        // The pairs' offsets add up (Q-207 g): in one order every run, so
        // the sums are the same to the last bit.
        let mut walkers = walkers;
        walkers.sort_by_key(|(id, _)| *id);
        let settings = &library.tables.character_settings;
        let evaders: Vec<_> = walkers.iter().map(|(_, e)| *e).collect();
        let offsets = crate::world::scene3d::evade(
            &evaders,
            settings.evade_start,
            settings.evade_radius_scale,
            library.tables.character_scale,
            tick,
        );
        for ((id, _), offset) in walkers.iter().zip(offsets) {
            if let Some(s) = movers.states.get_mut(id) {
                s.evade = offset;
            }
        }
    }
    if viewer.log_builders {
        log_still_builders(&movers, animator, world.tick());
    }
    for (&mover, state) in movers.states.iter_mut() {
        // Hook W30d: a walking figure's phase follows the ground it covers
        // (a cycle a stride: its feet do not slide, on a slope and on the
        // slow free walk too); it shows that phase's pose.
        if let Some(figure) = state.figure.as_mut() {
            let at = state.motion.at(t);
            let moved = figure
                .last
                .map_or(0.0, |l| Vec3::from(l).distance(Vec3::from(at)));
            if moved < FIGURE_JUMP {
                figure.phase = crate::render::models3d::figure::advance(figure.phase, moved);
            }
            figure.last = Some(at);
            let want = figure_clip_file(figure);
            if want != state.visual.clip {
                state.visual.clip = want;
                repose(
                    state,
                    false,
                    animator,
                    library,
                    &mut part_materials,
                    (&mut images, &mut materials),
                );
            }
        }
        // GFX-ANIM-3.3: random idle clips of a standing settler (not a
        // figure's: its own clips only, hook W30d).
        if let MoverAnim::Settler {
            good,
            donkey: false,
            ..
        } = &state.anim
        {
            let good = good.clone();
            if state.idle.is_some() {
                if state.pose.as_ref().is_none_or(|p| animator.finished(p)) {
                    state.idle = None;
                }
            } else if state.figure.is_none() && state.visual.standing() {
                if state.visual.random.is_none() {
                    if let Some(i) =
                        crate::animation::settler_anim::random_set(&sets, good.as_deref())
                    {
                        state.visual.random =
                            Some(crate::animation::settler_anim::RandomIdle::new(
                                i,
                                &sets[i],
                                &mut state.rng,
                            ));
                    }
                }
                if let Some(random) = state.visual.random.as_mut() {
                    let set = &sets[random.set];
                    // TRACE-5.3: the wait counts down once per tick.
                    for _ in 0..ticks {
                        if let Some(clip) = random.update(tick, set, &mut state.rng) {
                            state.idle = Some(set.clips[clip].0.clone());
                        }
                    }
                }
            }
            let before = state.pose.clone();
            repose(
                state,
                false,
                animator,
                library,
                &mut part_materials,
                (&mut images, &mut materials),
            );
            // The wait after a random clip covers the clip (GFX-ANIM-3.3).
            if state.idle.is_some() && state.pose != before {
                let seconds = state
                    .pose
                    .as_ref()
                    .and_then(|p| animator.duration(p))
                    .map_or(0.0, |d| d / crate::animation::anim::GLOBAL_SPEED);
                if let Some(random) = state.visual.random.as_mut() {
                    random.add_clip_time(seconds);
                }
            }
            // GFX-ANIM-3.7: dying, once the clip has ended, sinks.
            let dying = matches!(&state.anim, MoverAnim::Settler { state: s, .. } if s.dying);
            if dying && state.pose.as_ref().is_none_or(|p| animator.finished(p)) {
                // TRACE-5.3: per tick.
                state.visual.sunk +=
                    crate::animation::settler_anim::DYING_SINK * tick * ticks as f32;
            }
        }
        let Ok(mut transform) = transforms.get_mut(state.entity) else {
            continue;
        };
        let mut at = state.motion.at(t);
        at[1] -= state.visual.sunk;
        let heading = lerp_heading(state.heading_previous, state.heading_current, t);
        if state.evade != 0.0 {
            let [rx, rz] = crate::world::scene3d::right_of(heading);
            at[0] += rx * state.evade;
            at[2] += rz * state.evade;
        }
        let mut rotation = Quat::from_rotation_y(world3d::bevy_yaw(heading));
        if let (true, Some(index)) = (state.bob, state.ship) {
            // GFX-ANIM-7.2, 7.2a: the bow dips (the second value, about the
            // beam), then the ship heels to starboard (the first, about its
            // length), then the yaw of its heading + 90 degrees, which turns
            // its bow where a settler of that heading faces; lifted over the
            // water. Moving: a step in progress; the phase is the one before
            // this frame's advance.
            let p = ship_phase + crate::animation::object_anim::SHIP_PHASE_OFFSET * index as f32;
            let moving = state.motion.slides();
            let (pitch, roll, lift) = crate::animation::object_anim::bobbing(p, moving);
            at[1] += lift;
            rotation = crate::animation::object_anim::ship_rotation(heading, pitch, roll);
        }
        transform.translation = Vec3::from(world3d::to_bevy(at));
        transform.rotation = rotation;
        // GFX-ANIM-2.3, SND-2.1: the clip's Sound events at the mover.
        if let Some(pose) = &state.pose {
            if let (Some(now), Some(duration)) =
                (animator.pool.elapsed(pose), animator.duration(pose))
            {
                // A clip joined now fires the events at or after its
                // start offset (GFX-ANIM-2.3).
                let last = match &state.events_at {
                    Some((p, t)) if p == pose => *t,
                    _ => now - 1e-4,
                };
                // A settler's resolved entry carries its own events; a
                // random idle and the other movers go by the clip file.
                // Hook W30d: a figure's clips carry none (the entries' are
                // timed to the original's clips).
                let entry = match (&state.anim, &state.idle) {
                    (crate::world::scene3d::MoverAnim::Settler { .. }, None) => {
                        state.visual.entry.as_deref()
                    }
                    _ => None,
                };
                let events = match entry {
                    _ if state.figure.is_some() => &[][..],
                    Some(name) => library.sounds_of_entry(name),
                    None => library.sounds_of(&pose.clip),
                };
                for name in
                    crate::audio::sound::events_crossed(events, last, now, duration, pose.looping)
                {
                    // SND-2.4: it follows the mover while it plays.
                    sounds.0.push(crate::audio::sound::SoundRequest::At {
                        name: name.to_owned(),
                        at,
                        source: Some(mover),
                    });
                }
                state.events_at = Some((pose.clone(), now));
            }
        }
        // GFX-OBJ-7.2: the ware or tool at the attach bone, B(t) every
        // frame, nothing added (not while it lies in a boat, 7.6).
        if let (Some((_, ware)), Some(bone), Some(pose), Some(model), false) = (
            &state.carried,
            state.link,
            &state.pose,
            &state.model,
            state.in_boat,
        ) {
            if let Some(world) = animator.bone_world(pose, bone) {
                let m = match state.figure {
                    // Hook W30d: the point's frame turned with the figure,
                    // the item in its file's axes.
                    Some(_) => crate::render::models3d::figure::ware_matrix(world),
                    None => {
                        crate::world::carry::attach_matrix(model.basis, model.basis_inverse, world)
                    }
                };
                if let Ok(mut t) = transforms.get_mut(*ware) {
                    *t = Transform::from_matrix(m);
                }
            }
        }
    }
}

/// `--log-builders` (play test 10, F8): every frame a builder is in a work
/// step but drawn still: his play-once clip has run out (GFX-ANIM-3.8f a
/// allows only the last 0.7 tick of a step, T19a) or he shows another clip.
pub(crate) fn log_still_builders(
    movers: &Movers,
    animator: &crate::animation::animate3d::Animator,
    tick: u64,
) {
    for (id, state) in &movers.states {
        let crate::world::scene3d::MoverAnim::Settler { state: s, .. } = &state.anim else {
            continue;
        };
        if !matches!(id, crate::world::scene3d::MoverId::Builder(_)) || s.request.is_none() {
            continue;
        }
        let Some(pose) = &state.pose else {
            info!("builder {id:?} tick {tick}: working, no pose");
            continue;
        };
        let duration = animator.duration(pose).unwrap_or(0.0);
        let elapsed = animator.pool.elapsed(pose).unwrap_or(0.0);
        let hammer = state
            .visual
            .clip
            .as_ref()
            .is_some_and(|(c, _)| c.replace('\\', "/").eq_ignore_ascii_case(&pose.clip));
        if animator.finished(pose) || !hammer {
            info!(
                "builder {id:?} tick {tick}: drawn still in a work step started at tick {:?}: \
                 clip {} duration {duration:.3} s looping {} at {elapsed:.3} s, {} on the pose",
                s.started,
                pose.clip,
                pose.looping,
                animator.pool.users(pose),
            );
        }
    }
}

/// GFX-ANIM-3.1, 3.2: a settler's (or donkey's) clip choice of the tick;
/// true when the clip changed. With its model and its base point's height
/// (`swim`), the in-water test is GFX-ANIM-3.1's (Q-180 g): the middle of
/// the body at or below the water level.
pub(crate) fn choose_clip(
    visual: &mut crate::animation::settler_anim::SettlerVisual,
    anim: &crate::world::scene3d::MoverAnim,
    library: &crate::render::models3d::ModelLibrary,
    swim: Option<(&crate::render::models3d::LoadedModel, f32)>,
) -> bool {
    let crate::world::scene3d::MoverAnim::Settler {
        state,
        good,
        profession,
        tribe,
        donkey,
    } = anim
    else {
        return false;
    };
    let entries = if *donkey {
        library
            .tables
            .donkey
            .as_ref()
            .map(|d| d.animations.as_slice())
            .unwrap_or_default()
    } else {
        library.animations.entries.as_slice()
    };
    let mut state = state.clone();
    if let Some((model, foot)) = swim {
        let height = model.bounds[1][1] - model.bounds[0][1];
        state.under = crate::world::scene3d::swims(foot, height);
    }
    visual.tick(
        &state,
        entries,
        good.as_deref(),
        profession.as_deref(),
        *tribe,
    )
}

/// A bone's world matrix (file axes) in the frame of the model's converted
/// meshes, without the conversion's scale (the ware keeps its own size).
pub(crate) fn anim_frame(model: &crate::render::models3d::LoadedModel, world: Mat4) -> Mat4 {
    crate::animation::anim::conjugate(model.basis, model.basis_inverse, world)
}
