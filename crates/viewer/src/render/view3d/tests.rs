use super::*;
use crate::render::world3d::{CameraRig, Heights};
use bevy::camera::CameraProjection;

/// GFX-ANIM-3.8c, T11a: the lying tree's box height h (element size 1,
/// the last angle drawn just under 1.4235 rad) for a fall along the
/// model's own x and z axes: birch 7.413 / 7.84, broad leaf 9.17 /
/// 9.53, fir 6.96 / 6.55. A fall between the axes gives more (birch
/// 10.2 at 45 degrees), which the spec's ranges leave out (Q-216 b).
/// Skipped without `S2_GAME_DIR`.
#[test]
fn lying_tree_box_heights() {
    let Ok(dir) = std::env::var("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    for (model, along_x, along_z) in [
        ("RS_TreeBirch01", 7.413, 7.841),
        ("RS_TreeBroadLeaf01", 9.165, 9.532),
        ("RS_TreeFir01", 6.958, 6.551),
    ] {
        let rel = format!("data/map/objects/{model}.GR2");
        let path = s2_rules::resolve(std::path::Path::new(&dir), &rel).expect(&rel);
        let data = std::fs::read(&path).expect(&rel);
        let gr2 = s2_assets::gr2::load(&path.to_string_lossy(), &data).expect(&rel);
        let basis = crate::render::models3d::basis_conversion(
            gr2.art_tool.as_ref(),
            crate::render::models3d::DEFAULT_SCALE,
        );
        let mut b = [[f32::MAX; 3], [f32::MIN; 3]];
        for m in &gr2.meshes {
            for p in &m.vertices.positions {
                let q = crate::render::world3d::from_bevy(basis.apply(*p));
                for k in 0..3 {
                    b[0][k] = b[0][k].min(q[k]);
                    b[1][k] = b[1][k].max(q[k]);
                }
            }
        }
        let h = |axis: f32| {
            let placed = crate::world::scene3d::Placed {
                key: (crate::world::scene3d::Kind::Deposit, (0, 0), 0),
                file: rel.clone(),
                units_per_metre: crate::render::models3d::DEFAULT_SCALE,
                at: [0.0; 3],
                heading: 0.0,
                scale: [1.0; 3],
                tilt_axis: axis,
                tilt: 1.4235,
                tint: [1.0; 4],
                family: crate::render::models3d::Family::Plain,
                player: [0.0; 3],
                extra: [0.0; 4],
                shadow: true,
                pieces: None,
                plane_of: None,
                parts: crate::world::scene3d::Parts::All,
                sink: None,
            };
            box_height(b, &placed)
        };
        let right = std::f32::consts::FRAC_PI_2;
        for (axis, want) in [(0.0, along_x), (right, along_z), (2.0 * right, along_x)] {
            let got = h(axis);
            assert!((got - want).abs() < 0.01, "{model} axis {axis}: {got}");
        }
    }
}

/// Hook W17: a project's factor multiplies the profile's map size (at most
/// 8192 texels), and shadows off stay off; no factor is the profile's size.
// spec: none (hook W17, docs/extending.md "The game with extensions"; the size is GFX-VIEW-2.5 d's)
#[test]
fn a_projects_factor_scales_the_profiles_shadow_map() {
    assert_eq!(scaled_shadow_map(Some(2048), None), Some(2048));
    assert_eq!(scaled_shadow_map(Some(1024), Some(2)), Some(2048));
    assert_eq!(scaled_shadow_map(Some(2048), Some(8)), Some(8192));
    assert_eq!(scaled_shadow_map(Some(512), Some(0)), Some(512));
    assert_eq!(scaled_shadow_map(None, Some(2)), None);
}

/// GFX-VIEW-2.5 d (Run D): the map's size by `SHADOW QUALITY`.
#[test]
fn shadow_map_sizes() {
    let got = [0.0, 0.25, 0.5, 0.75, 1.0].map(shadow_map_size);
    assert_eq!(got, [None, Some(256), Some(512), Some(1024), Some(2048)]);
}

// GFX-VIEW-2.5 e (T42): shadows are drawn while the profile's shadows switch
// (SET-4a field 10) is on and the shader detail is 2 or more; the quality
// only sizes the map: the switch off means none whatever the quality, a
// switch on with quality 0 none either, shader detail 1 none.
// spec: GFX-VIEW-2.5, SET-4a
#[test]
fn t42_the_shadows_switch_decides() {
    let scene = |switch: u32, quality: f32, detail: u32| {
        let mut s = s2_assets::cfg::SceneSettings::default();
        s.words[9] = switch;
        s.words[10] = quality.to_bits();
        s.words[15] = detail;
        scene_shadow_map(&s)
    };
    assert_eq!(scene(1, 1.0, 3), Some(2048));
    assert_eq!(scene(1, 0.5, 2), Some(512));
    assert_eq!(scene(0, 1.0, 3), None);
    assert_eq!(scene(1, 0.0, 3), None);
    assert_eq!(scene(1, 1.0, 1), None);
    assert_eq!(
        scene_shadow_map(&s2_assets::cfg::SceneSettings::default()),
        Some(2048)
    );
    // With shadows off the receivers' shadow factor is 1: every shadow test
    // gives lit; with shadows on the day's factor (0.1 at noon) stays.
    let day = crate::render::world3d::DayLight::XML.shader();
    assert_eq!(receivers_light(day, false).direction[3], 1.0);
    assert_eq!(receivers_light(day, true).direction, day.direction);
    assert!(day.direction[3] < 1.0);
}

/// SET-4b (T6): a game started with preset Medium (switch on, `SHADOW
/// QUALITY` 0.5, shader detail high) keeps the map size and the shader
/// level of its start; the shadows switch acts at once. `SHADOW QUALITY`
/// moved to 0 during the game: no shadows from the next frame; moved to 1:
/// shadows again, with the start's 512 map until the next game, which
/// starts with 2048. A shader detail changed during the game waits for the
/// next game too; a game started with quality 0 has no map, so a switch
/// turned on later draws none.
// spec: SET-4b, GFX-VIEW-2.5
#[test]
fn t6_the_shadows_switch_acts_at_once() {
    use s2_assets::cfg::{field, SceneSettings, PRESET_MEDIUM};
    let mut scene = SceneSettings::default();
    scene.apply_preset(PRESET_MEDIUM);
    let game = game_shadow_map(&scene);
    assert_eq!(game, Some(512));
    assert_eq!(frame_shadow_map(&scene, game), Some(512));
    scene.move_slider(field::SHADOW_QUALITY, 0.0);
    assert!(!scene.shadows());
    assert_eq!(frame_shadow_map(&scene, game), None);
    scene.move_slider(field::SHADOW_QUALITY, 1.0);
    assert_eq!(frame_shadow_map(&scene, game), Some(512));
    // Shader detail off during the game: the device level is the start's.
    scene.set_shader_detail(1);
    assert_eq!(frame_shadow_map(&scene, game), Some(512));
    scene.set_shader_detail(3);
    // The next game.
    let game = game_shadow_map(&scene);
    assert_eq!(frame_shadow_map(&scene, game), Some(2048));
    // A game started with quality 0: no map; the switch on later draws none.
    scene.move_slider(field::SHADOW_QUALITY, 0.0);
    let game = game_shadow_map(&scene);
    assert_eq!(game, None);
    scene.words[field::SHADOWS - 1] = 1;
    assert_eq!(frame_shadow_map(&scene, game), None);
    // A game started with the switch off keeps its map for a switch turned
    // on later.
    scene.move_slider(field::SHADOW_QUALITY, 0.75);
    scene.words[field::SHADOWS - 1] = 0;
    let game = game_shadow_map(&scene);
    assert_eq!(frame_shadow_map(&scene, game), None);
    scene.words[field::SHADOWS - 1] = 1;
    assert_eq!(frame_shadow_map(&scene, game), Some(1024));
}

/// Hills of up to 25 world units on a 40 x 40 map.
struct Hills;

impl Heights for Hills {
    fn frame(&self) -> Frame {
        Frame::new(40, 40)
    }
    fn raw(&self, c: u32, r: u32) -> i32 {
        let (c, r) = (c as f32, r as f32);
        (12_500.0 * ((c / 9.0).sin() * (r / 7.0).cos() + 1.0)) as i32
    }
}

/// The view's camera at a rig's eye as Bevy draws it (place_camera,
/// setup_3d): screen point of a game point, and the ray of a screen
/// point, as Camera::world_to_viewport and viewport_to_world compute them.
struct Screen {
    view: Mat4,
    clip: Mat4,
    size: Vec2,
}

impl Screen {
    fn new(eye: world3d::Eye, size: Vec2) -> Self {
        let transform = Transform::from_translation(Vec3::from(world3d::to_bevy(eye.position)))
            .looking_to(Vec3::from(world3d::to_bevy(eye.direction)), Vec3::Y);
        let projection = PerspectiveProjection {
            fov: world3d::vertical_fov(size.x / size.y),
            near: world3d::NEAR,
            far: world3d::FAR,
            aspect_ratio: size.x / size.y,
            ..default()
        };
        Self {
            view: transform.to_matrix(),
            clip: projection.get_clip_from_view(),
            size,
        }
    }

    fn project(&self, game: [f32; 3]) -> Option<Vec2> {
        let p = Vec3::from(world3d::to_bevy(game));
        let ndc = (self.clip * self.view.inverse()).project_point3(p);
        let inside = ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0 && ndc.z > 0.0;
        inside.then(|| Vec2::new((ndc.x + 1.0) / 2.0, (1.0 - ndc.y) / 2.0) * self.size)
    }

    fn ray(&self, cursor: Vec2) -> ([f32; 3], [f32; 3]) {
        let ndc = Vec2::new(
            cursor.x / self.size.x * 2.0 - 1.0,
            1.0 - cursor.y / self.size.y * 2.0,
        );
        let world_from_clip = self.view * self.clip.inverse();
        let near = world_from_clip.project_point3(ndc.extend(1.0));
        let far = world_from_clip.project_point3(ndc.extend(f32::EPSILON));
        let direction = (far - near).normalize();
        (
            world3d::from_bevy(near.to_array()),
            world3d::from_bevy(direction.to_array()),
        )
    }
}

// Picking over hills (GFX-VIEW-1.5, Q-202 a, Q-1067): a field's point drawn
// on the screen picks that field again, at every zoom and turn of the
// camera, over the sectors in the walk's order from the eye, wherever the
// ray meets the field-point surface only there (a ray that meets it twice
// picks by the list's order, tested in world3d). A flat plane at height 0
// would pick other fields for many of them.
// spec: GFX-VIEW-1.5
#[test]
fn screen_points_over_hills_pick_their_field() {
    let hills = Hills;
    let frame = hills.frame();
    let size = Vec2::new(1280.0, 800.0);
    let mut checked = 0;
    let mut flat_misses = 0;
    for zoom in [0, 1, 2, 4] {
        for turns in 0..2 {
            let mut rig = CameraRig::new(10.0, -20.0);
            rig.zoom = zoom;
            for _ in 0..turns {
                rig.rotate_left();
            }
            let mut eye = rig.update(&hills);
            for _ in 0..60 {
                eye = rig.update(&hills);
            }
            let screen = Screen::new(eye, size);
            let mut tree = world3d::sectors::SectorTree::new(&hills);
            let p = [0, 1, 2].map(|i| eye.position[i] + 0.7 * (rig.target[i] - eye.position[i]));
            let list = tree.walk(p, 1.0e6, &|_| world3d::sectors::Seen::Straddles);
            assert_eq!(list.len(), 100);
            // The edge fields' points lie on the mesh's border, where a
            // ray can pass under the mesh.
            for y in 1..39 {
                for x in 1..39 {
                    let [fx, fz] = frame.field_xz(x, y);
                    let ground = [fx, world3d::ground_height(&hills, fx, fz), fz];
                    let Some(cursor) = screen.project(ground) else {
                        continue;
                    };
                    let (origin, direction) = screen.ray(cursor);
                    let dist = |p: [f32; 3]| {
                        (0..3)
                            .map(|i| (p[i] - origin[i]).powi(2))
                            .sum::<f32>()
                            .sqrt()
                    };
                    // A ray that only touches the ground at the point (a
                    // crest seen edge on) meets it or not by rounding.
                    let above = |s: f32| {
                        let p = [0, 1, 2].map(|i| origin[i] + direction[i] * s);
                        p[1] > world3d::ground_height(&hills, p[0], p[2])
                    };
                    let s = dist(ground);
                    if above(s - 0.3) && above(s + 0.3) {
                        continue;
                    }
                    // Every place the ray meets the field-point surface.
                    if surface_hits(&hills, origin, direction) != 1 {
                        continue;
                    }
                    assert_eq!(
                        tree.pick(&hills, &list, origin, direction),
                        Some((x, y)),
                        "zoom {zoom}, turn {turns}, cursor {cursor}"
                    );
                    checked += 1;
                    // The same ray against the plane y = 0.
                    let t = -origin[1] / direction[1];
                    let [u, v] =
                        frame.to_plane(origin[0] + direction[0] * t, origin[2] + direction[2] * t);
                    let flat =
                        crate::render::terrain::nearest_field(u, v, 40, 40).map(|f| (f.x, f.y));
                    flat_misses += usize::from(flat != Some((x, y)));
                }
            }
        }
    }
    assert!(checked > 500, "{checked} fields checked");
    assert!(flat_misses > checked / 2, "{flat_misses} of {checked}");
}

/// The camera hook (docs/extending.md): a project's point moves the rig's
/// target each frame at the zoom and turn it has (Bevy's z is the game's
/// -z); the player's own camera input since the last frame (here a pan and
/// following a settler, UI-GAME-12.1 O) ends it with `cleared`. Without a
/// point the rig is the player's alone (GFX-VIEW-3.2).
// spec: GFX-VIEW-3.2 (the player's input wins over the camera hook)
#[test]
fn a_project_follow_ends_with_the_players_input() {
    use crate::extend::CameraFollow;
    let mut view = View3d::new(true, None);
    view.rig.target = [10.0, 0.0, 20.0];
    let mut follow = CameraFollow::default();
    follow_target(&mut view, &mut follow);
    assert_eq!(view.rig.target, [10.0, 0.0, 20.0]);
    assert_eq!(follow, CameraFollow::default());

    follow.target = Some(Vec3::new(30.0, 5.0, -40.0));
    let zoom = view.rig.zoom;
    for step in 0..3 {
        follow.target = Some(Vec3::new(30.0 + step as f32, 5.0, -40.0));
        follow_target(&mut view, &mut follow);
        view.placed = Some(rig_state(&view.rig));
        assert_eq!(view.rig.target, [30.0 + step as f32, 0.0, 40.0]);
        assert_eq!(view.rig.zoom, zoom);
        assert!(!follow.cleared);
    }
    // The player pans: the follow ends and says so; the rig keeps the pan.
    view.rig.target[0] += 1.5;
    follow_target(&mut view, &mut follow);
    assert_eq!(follow.target, None);
    assert!(follow.cleared);
    assert_eq!(view.rig.target, [33.5, 0.0, 40.0]);
    // A new point, followed; then the player follows a settler (O): O wins.
    follow.cleared = false;
    follow.target = Some(Vec3::ZERO);
    follow_target(&mut view, &mut follow);
    view.placed = Some(rig_state(&view.rig));
    view.follow = Some(s2_sim::view::SettlerRef::Settler(1));
    follow_target(&mut view, &mut follow);
    assert!(follow.cleared && follow.target.is_none());
    assert_eq!(view.follow, Some(s2_sim::view::SettlerRef::Settler(1)));
}

/// W10: a project's point set while O follows a settler (GFX-VIEW-3.7) is
/// not dropped: O's follow ends, the normal camera goes on from O's eye,
/// and the project's follow takes the camera, without `cleared`. While the
/// player looks around (GFX-VIEW-3.2a) the point is dropped as before.
// spec: GFX-VIEW-3.7 (W10: the original's follow hands the camera to a project's point)
#[test]
fn a_project_point_set_while_o_follows_takes_the_camera() {
    use crate::extend::CameraFollow;
    let mut view = View3d::new(true, None);
    view.rig.target = [10.0, 0.0, 20.0];
    view.follow = Some(s2_sim::view::SettlerRef::Settler(7));
    view.follow_eye = view.rig.last_eye();
    let mut follow = CameraFollow {
        target: Some(Vec3::new(30.0, 5.0, -40.0)),
        cleared: false,
    };
    follow_target(&mut view, &mut follow);
    assert_eq!(view.follow, None);
    assert_eq!(view.follow_eye, None);
    assert_eq!(follow.target, Some(Vec3::new(30.0, 5.0, -40.0)));
    assert!(!follow.cleared);
    assert_eq!([view.rig.target[0], view.rig.target[2]], [30.0, 40.0]);
    // Without a project's point, O's follow is the player's alone.
    let mut none = CameraFollow::default();
    view.follow = Some(s2_sim::view::SettlerRef::Settler(7));
    view.placed = None;
    follow_target(&mut view, &mut none);
    assert_eq!(view.follow, Some(s2_sim::view::SettlerRef::Settler(7)));
    assert_eq!(none, CameraFollow::default());
}

/// F55: a round of placing removes no doodad by leaving it out; every other
/// kind goes when it is not wanted.
// spec: GFX-OBJ-1.3, ECO-PLACE-5.3
#[test]
fn doodads_left_out_of_a_round_stay() {
    use crate::world::scene3d::Kind;
    use std::collections::HashSet;
    let (standing, kept) = (
        (Kind::Doodad, (1u32, 1u32), 0u32),
        (Kind::Doodad, (3, 3), 2),
    );
    let felled = (Kind::Deposit, (4, 4), 7);
    let placed = [standing, kept, felled];
    let keep_kinds: HashSet<_> = HashSet::new();
    assert_eq!(objects::gone_objects(placed.iter(), &keep_kinds), [felled]);
    let keep: HashSet<_> = [felled].into_iter().collect();
    assert!(objects::gone_objects(placed.iter(), &keep).is_empty());
}

/// GFX-VIEW-2.2, 2.4: a cell's mesh is rebuilt when a sample its vertex
/// normals read changes, also one just outside it (a road evening the next
/// cell's edge), not when one further away does.
// spec: GFX-VIEW-2.2, GFX-VIEW-2.4
#[test]
fn a_cell_is_rebuilt_when_a_normal_it_draws_changes() {
    let (width, height) = (20, 20);
    let frame = world3d::Frame::new(width, height);
    let samples = (frame.columns * frame.rows) as usize;
    let mut terrain = s2_sim::map::Terrain {
        width,
        height,
        heights: vec![5000; (width * height) as usize],
        height_floor: -100,
        states: vec![0; (width * height) as usize],
        samples: vec![5000; samples],
    };
    let ([_, c1], _) = crate::render::terrain3d::cell_samples(&frame, 0, 0);
    let before = build::cell_heights(&terrain, &frame, 0, 0);
    let mesh_before = crate::render::terrain3d::cell_mesh(
        &crate::render::terrain3d::TerrainHeights::current(&terrain),
        0,
        0,
    );
    // A sample one column past the cell's last: the edge vertex's normal
    // reads it.
    let at = |c: u32, r: u32| (r * frame.columns + c) as usize;
    terrain.samples[at(c1 + 1, 5)] = 5600;
    assert_ne!(build::cell_heights(&terrain, &frame, 0, 0), before);
    let mesh_after = crate::render::terrain3d::cell_mesh(
        &crate::render::terrain3d::TerrainHeights::current(&terrain),
        0,
        0,
    );
    assert_ne!(mesh_after.normals, mesh_before.normals);
    // Two columns past it: nothing of this cell changes.
    terrain.samples[at(c1 + 1, 5)] = 5000;
    terrain.samples[at(c1 + 2, 5)] = 5600;
    assert_eq!(build::cell_heights(&terrain, &frame, 0, 0), before);
    assert_eq!(
        crate::render::terrain3d::cell_mesh(
            &crate::render::terrain3d::TerrainHeights::current(&terrain),
            0,
            0
        )
        .normals,
        mesh_before.normals
    );
}

/// F55: only the doodads something happened to are looked at again: on a
/// field whose removals grew (GFX-OBJ-1.4a), on a field a site cleared
/// (ECO-PLACE-5.3), and on or next to a terrain cell whose heights changed.
// spec: GFX-OBJ-1.3, GFX-OBJ-1.4a, ECO-PLACE-5.3
#[test]
fn only_the_doodads_that_changed_are_placed_again() {
    use std::collections::{BTreeMap, BTreeSet};
    // Numbers 0 to 3: on samples (0, 0), (40, 0), (0, 32), (40, 32): cells
    // (0, 0), (2, 0), (0, 2), (2, 2) of 16 samples each.
    let doodads = [(1, (0, 0)), (1, (40, 0)), (1, (0, 32)), (1, (40, 32))];
    let mut track = objects::DoodadTrack::new(&doodads, 20, 20);
    let none = BTreeMap::new();
    let mut removed = BTreeMap::new();
    let one = crate::world::scene3d::decoration_field((40, 0), 20, 20);
    removed.insert(one, s2_sim::DOODADS_BY_STREET);
    // The first time nothing is "changed": all are placed anyway.
    assert!(track.changed(&removed, &BTreeSet::new(), &[]).is_empty());
    // Nothing happened.
    assert!(track.changed(&removed, &BTreeSet::new(), &[]).is_empty());
    // A flag comes on doodad 1's field too: its removals grow.
    removed.insert(one, s2_sim::DOODADS_BY_STREET | s2_sim::DOODADS_ALL);
    assert_eq!(track.changed(&removed, &BTreeSet::new(), &[]), [1]);
    assert!(track.changed(&removed, &BTreeSet::new(), &[]).is_empty());
    // A site clears the field doodad 2 stands on, once.
    let field = crate::world::scene3d::decoration_field((0, 32), 20, 20);
    let cleared = BTreeSet::from([field]);
    assert_eq!(track.changed(&removed, &cleared, &[]), [2]);
    assert!(track.changed(&removed, &cleared, &[]).is_empty());
    // The ground moves in cell (2, 1): cells next to it hold doodads 1 and 3;
    // two reasons at once give one number.
    assert_eq!(track.changed(&removed, &cleared, &[(2, 1)]), [1, 3]);
    let mut fresh = objects::DoodadTrack::new(&doodads, 20, 20);
    assert!(fresh.changed(&none, &BTreeSet::new(), &[]).is_empty());
    assert_eq!(fresh.changed(&removed, &BTreeSet::new(), &[(2, 1)]), [1, 3]);
}

/// GFX-VIEW-5.4 (a) to (d), T36: a new settler starts unseen; its mark is
/// refreshed from the local player's exploration layer only in a frame that
/// draws its field (or the renderer's first) while it walks or the frame is
/// forced by an exploration; otherwise it keeps its value. So a settler that
/// walked out of view and stands still where the camera jumps stays unseen
/// (constructor 90640) until it steps or something is explored; with fog of
/// war off every field counts as explored.
// spec: GFX-VIEW-5.4
#[test]
fn a_settler_is_seen_only_after_a_refresh_in_view() {
    // (a) new: clear. Walking out of view: no refresh.
    let mut mark = false;
    mark = refresh_seen(mark, true, false, false, true, false);
    assert!(!mark, "walked only outside the drawn area");
    // The camera jumps to it while it stands: still unseen.
    mark = refresh_seen(mark, true, true, false, false, false);
    assert!(!mark, "T36: standing where the camera jumped");
    // An exploration forces the next frame: seen.
    assert!(refresh_seen(mark, true, true, false, false, true));
    // Or it takes a step in view.
    mark = refresh_seen(mark, true, true, false, true, false);
    assert!(mark);
    // Once seen it keeps the mark while it stands, in view or not.
    assert!(refresh_seen(mark, false, true, false, false, false));
    assert!(refresh_seen(mark, false, false, false, true, false));
    // Walking in view on unexplored land clears it.
    assert!(!refresh_seen(mark, false, true, false, true, false));
    // The renderer's first frame refreshes walking settlers anywhere.
    assert!(refresh_seen(false, true, false, true, true, false));
    assert!(!refresh_seen(false, true, false, true, false, false));
}

/// GFX-VIEW-5.4 (a): the mark is the settler's, made once: a constructor
/// seen while he works at his site in view (the builder's mover) is still
/// seen when he leaves it standing (a settler's mover again), and one inside
/// a building keeps his mark. A settler new to the game starts unseen.
// spec: GFX-VIEW-5.4
#[test]
fn a_settlers_mark_survives_his_job_changes() {
    use crate::world::scene3d::{MoverId, SeenKey, SeenPoint};
    let point = |id, walks, shown| SeenPoint {
        id,
        key: SeenKey::Object(90640),
        at: [0.0; 3],
        field: (10, 10),
        walks,
        shown,
    };
    let all = Some(FieldRect {
        columns: [0, 39],
        rows: [0, 39],
    });
    let mut marks = MarkState::default();
    let site = MoverId::Builder((10, 10));
    // The renderer's first frame, nobody walks; then he walks in view.
    marks.frame(0, &[point(site, false, true)], all, |_| true);
    assert_eq!(marks.mark(site), Some(false), "new: unseen");
    marks.frame(0, &[point(site, true, true)], all, |_| true);
    assert_eq!(marks.mark(site), Some(true));
    // Done: he stands by the door as a settler of his own.
    let home = MoverId::Settler(7);
    marks.frame(0, &[point(home, false, true)], all, |_| true);
    assert_eq!(marks.mark(home), Some(true), "the same settler's mark");
    // Inside a building: kept, not refreshed (even walking, unexplored).
    marks.frame(0, &[point(home, true, false)], all, |_| false);
    assert_eq!(marks.mark(home), Some(true));
    // Another settler starts unseen.
    let other = SeenPoint {
        key: SeenKey::Object(90641),
        ..point(MoverId::Settler(8), false, true)
    };
    marks.frame(0, &[other], all, |_| true);
    assert_eq!(marks.mark(MoverId::Settler(8)), Some(false));
}

/// GFX-VIEW-5.4 (b): the drawn area of a play-ahead's camera is its
/// picture: the point looked at is in it, a point behind the camera and one
/// far to the side are not.
// spec: GFX-VIEW-5.4, GFX-VIEW-3.5
#[test]
fn the_play_aheads_camera_sees_its_target() {
    let eye = world3d::Eye {
        position: [0.0, 40.0, -40.0],
        direction: world3d::normalize([0.0, -0.6, 0.8]),
    };
    let clip = clip_from_world(eye, 16.0 / 9.0);
    assert!(in_picture(clip, [0.0, 0.0, 13.3]));
    assert!(!in_picture(clip, [0.0, 40.0, -60.0]), "behind");
    assert!(!in_picture(clip, [200.0, 0.0, 13.3]), "far to the side");
}

/// GFX-VIEW-5.4 (b), (f) (Q-1031 answered): the drawn field rectangle is
/// the union of the 4 x 4-field cells the camera sees, their ranges shared
/// at the borders and inclusive (the last cell ends at width - 1): around
/// the field looked at, not the whole map, on cell borders; a field just
/// outside is not in it. A camera looking up at the sky draws no cell.
// spec: GFX-VIEW-5.4
#[test]
fn the_drawn_rectangle_is_the_union_of_the_cells_seen() {
    let (width, height) = (170, 170);
    let fields = (width * height) as usize;
    let terrain = s2_sim::map::Terrain {
        width,
        height,
        heights: vec![5000; fields],
        height_floor: -100,
        states: vec![0; fields],
        samples: Vec::new(),
    };
    let frame = Frame::new(width, height);
    let [x, z] = frame.field_xz(85, 85);
    let mut rig = world3d::CameraRig::start(Some([x, z]));
    let eye = rig.update(&terrain);
    let clip = clip_from_world(eye, 16.0 / 9.0);
    let rect = drawn_field_rect(&terrain, &frame, clip).expect("cells drawn");
    assert!(rect.contains((85, 85)), "{rect:?}");
    assert!(rect.columns[1] - rect.columns[0] < 100, "{rect:?}");
    for edge in [rect.columns[0], rect.columns[1], rect.rows[0], rect.rows[1]] {
        assert!(edge % 4 == 0 || edge == width - 1, "{rect:?}");
    }
    assert!(!rect.contains((rect.columns[1] + 1, rect.rows[0])));
    assert!(
        rect.contains((rect.columns[1], rect.rows[1])),
        "borders included"
    );
    let sky = world3d::Eye {
        position: [x, 40.0, z],
        direction: [0.0, 1.0, 0.0001],
    };
    assert_eq!(
        drawn_field_rect(&terrain, &frame, clip_from_world(sky, 16.0 / 9.0)),
        None
    );
}

/// GFX-ANIM-3.8a, 3.8c (stand-ins list row 14): a tool that plays a game
/// ahead keeps the fall axes of the trees felled in its ticks, as the view
/// does tick by tick (the original's woodcutter view object ticks whether
/// seen or not): every tree felled while the woodcutter of Storm Coast's
/// first hut works has the axis toward him, the same one the view takes
/// when it runs through those ticks, and none falls the fixed +x way.
/// Skipped without `S2_GAME_DIR`.
// spec: GFX-ANIM-3.8a, GFX-ANIM-3.8c
#[test]
fn a_played_ahead_tree_falls_toward_its_woodcutter() {
    let Some(root) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
    let (rules, _) = s2_rules::load(&dir).expect("the property scripts");
    let path = crate::game::find_maps(&dir.root().join("data/maps"))
        .into_iter()
        .find(|p| crate::game::map_name(p).eq_ignore_ascii_case("MP_2P_Storm_Coast"))
        .expect("the map");
    let map = s2_assets::s2m::parse(&crate::game::read_game_file(&path).unwrap()).unwrap();
    let mut world = s2_rules::new_game(1, &map, &rules, 0).expect("a game");
    let hut = rules
        .buildings
        .iter()
        .find(|b| b.name.as_deref() == Some("WoodcutterHut"))
        .map(|b| b.id)
        .expect("WoodcutterHut");
    crate::tools::scenes::building(&mut world, s2_sim::PlayerId(0), hut).expect("a hut");
    let frame = Frame::new(world.terrain().width, world.terrain().height);
    let mut ahead = PlayAhead::new(&world, 16.0 / 9.0, true);
    let mut live = std::collections::HashMap::new();
    let mut felled = std::collections::BTreeSet::new();
    for _ in 0..30_000 {
        world.step();
        ahead.frame(&world);
        crate::world::scene3d::update_fall_axes(&world, &frame, &mut live);
        felled.extend(
            world
                .deposits()
                .filter(|(f, _)| world.fall_of(*f).is_some())
                .map(|(f, _)| f),
        );
        if felled.len() >= 2 {
            break;
        }
    }
    assert!(felled.len() >= 2, "only {} trees felled", felled.len());
    // Each lying tree's axis came from its woodcutter while the play ran:
    // the same as the view's, tick by tick.
    let mut checked = 0;
    for _ in 0..30_000 {
        world.step();
        ahead.frame(&world);
        crate::world::scene3d::update_fall_axes(&world, &frame, &mut live);
        for (field, _) in world
            .deposits()
            .filter(|(f, _)| world.fall_of(*f).is_some())
        {
            assert_eq!(
                ahead.fall_axes.get(&field),
                live.get(&field),
                "tree {field:?}"
            );
            assert!(
                ahead.fall_axes.contains_key(&field),
                "tree {field:?} has no axis"
            );
            checked += 1;
        }
        if checked > 100 {
            break;
        }
    }
    assert!(checked > 100, "only {checked} lying-tree ticks");
    // GFX-ANIM-3.8c1: a view starting after the felling, the woodcutter
    // gone from the tree (a load), gives it no axis at all, no fixed one.
    let mut late = std::collections::HashMap::new();
    crate::world::scene3d::update_fall_axes(&world, &frame, &mut late);
    for (field, _) in world.deposits() {
        if world.fall_of(field).is_some() {
            assert!(!late.contains_key(&field), "tree {field:?}");
        }
    }
}

/// GFX ANIM T11a (Q-1087): one woodcutter's work at a tree begun in tick W,
/// the game loaded at W + 20, at W + 40 and 5 ticks after the harvest: at
/// W + 20 the tree falls from W + 30 as in play; at W + 40 it stands at the
/// load and falls from the first tick after it (0.0044 rad after that tick,
/// the 3.8a sequence), about the line to where he stands; loaded after the
/// harvest it stands upright, never falls, never sinks, and is gone at its
/// removal's end. A removal that starts after the load sinks as in play.
/// Skipped without `S2_GAME_DIR`.
// spec: GFX-ANIM-3.8c1, GFX-ANIM-3.8a, GFX-ANIM-3.8c
#[test]
fn t11a_a_load_restores_no_fall_and_no_sink() {
    use crate::world::scene3d::{fall_angle_from, seen_fall_start, sink_seen, Fall};
    let Some(root) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
    let (rules, _) = s2_rules::load(&dir).expect("the property scripts");
    let path = crate::game::find_maps(&dir.root().join("data/maps"))
        .into_iter()
        .find(|p| crate::game::map_name(p).eq_ignore_ascii_case("MP_2P_Storm_Coast"))
        .expect("the map");
    let map = s2_assets::s2m::parse(&crate::game::read_game_file(&path).unwrap()).unwrap();
    let mut world = s2_rules::new_game(1, &map, &rules, 0).expect("a game");
    let hut = rules
        .buildings
        .iter()
        .find(|b| b.name.as_deref() == Some("WoodcutterHut"))
        .map(|b| b.id)
        .expect("WoodcutterHut");
    crate::tools::scenes::building(&mut world, s2_sim::PlayerId(0), hut).expect("a hut");
    let frame = Frame::new(world.terrain().width, world.terrain().height);
    // To the tick his work action at a tree began.
    let (tree, began) = (0..30_000)
        .find_map(|_| {
            world.step();
            world.deposits().find_map(|(f, d)| {
                (world.fall_of(f).is_none()
                    && !matches!(d.state, s2_sim::DepositState::Removing { .. }))
                .then(|| world.felling_began(f))
                .flatten()
                .map(|w| (f, w))
            })
        })
        .expect("a tree felled");
    let mut saves = std::collections::BTreeMap::new();
    while world.deposit(tree).is_some() {
        let t = world.tick();
        if t == began + 20 || t == began + 40 {
            saves.insert(t - began, world.clone());
        }
        if matches!(world.deposit(tree).map(|d| d.state), Some(s2_sim::DepositState::Removing { since, .. }) if t == since + 5)
        {
            saves.insert(100, world.clone());
        }
        world.step();
    }
    assert_eq!(saves.len(), 3, "the three saves");
    // Saved at W + 20: as in play.
    let w20 = &saves[&20];
    let start = seen_fall_start(w20, tree, Some(began + 20));
    assert_eq!(start, Some(began + 30));
    assert_eq!(
        fall_angle_from(start, (began + 29) as f32 + 0.9),
        Fall::Standing
    );
    // Saved at W + 40: upright at the load, 0.0044 rad one tick after it.
    let w40 = &saves[&40];
    let loaded = Some(began + 40);
    let start = seen_fall_start(w40, tree, loaded);
    assert_eq!(start, Some(began + 40));
    assert_eq!(
        fall_angle_from(start, (began + 40) as f32),
        Fall::Falling(0.0)
    );
    let Fall::Falling(a) = fall_angle_from(start, (began + 41) as f32) else {
        panic!("falling one tick after the load");
    };
    assert!((a - 0.0043633).abs() < 1e-5, "{a}");
    // About the line to where he stands: his work action restored, the
    // view takes the axis from him, not a fixed one.
    let mut axes = std::collections::HashMap::new();
    crate::world::scene3d::update_fall_axes(w40, &frame, &mut axes);
    assert!(axes.contains_key(&tree), "an axis from the woodcutter");
    // Saved 5 ticks after the harvest: upright, no fall, no sink, gone at
    // its removal's end.
    let mut after = saves.remove(&100).unwrap();
    let loaded = Some(after.tick());
    let Some(s2_sim::DepositState::Removing { since, gone_at }) =
        after.deposit(tree).map(|d| d.state)
    else {
        panic!("removing");
    };
    assert!(!sink_seen(since, loaded));
    assert!(sink_seen(since, None), "seen in play it sinks");
    let mut axes = std::collections::HashMap::new();
    while after.deposit(tree).is_some() {
        assert_eq!(seen_fall_start(&after, tree, loaded), None);
        crate::world::scene3d::update_fall_axes(&after, &frame, &mut axes);
        assert!(!axes.contains_key(&tree), "no fall axis");
        after.step();
    }
    assert_eq!(
        after.tick(),
        gone_at + 1,
        "gone when its counter reaches 20"
    );
    // A removal the view saw start sinks.
    assert!(sink_seen(since, Some(since)));
}

/// GFX-VIEW-5.4 (e), (g), T36a: our Weak game (the recording's set-up,
/// fog of war off) played to trace tick 6000 with the camera on player 0's
/// headquarters, then the farm scene's camera of the shots run (paused): the
/// walking leveller 90750 is seen; constructor 90640 and worker 90480, in
/// the picture but standing, walked only on player 1's land and are not.
// spec: GFX-VIEW-5.4 (T36a)
#[test]
fn t36a_the_farm_picture_after_playing_ahead_on_the_castle() {
    let Ok(dir) = s2_assets::GameDir::from_env() else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let file = s2_assets::find_map(&dir, "MP_2P_Storm_Coast").expect("the map");
    let data = std::fs::read(&file).expect("map file");
    let map = s2_assets::s2m::load(&file.display().to_string(), &data).expect("map");
    let setup = s2_rules::game_setup(&map, &rules, 1);
    let sim = rules.sim_rules().expect("sim rules");
    let mut world = s2_sim::World::new_game(
        u64::from(s2_sim::rng::SIM_SEED),
        map.terrain(),
        sim.clone(),
        &setup,
    );
    let mut ai = s2_ai::AiSystem::new(s2_ai::AiConfig::new(&rules, &sim), &world);
    ai.activate_all(&mut world, &[(s2_sim::PlayerId(1), 1)]);
    // The shots run's window, 2560 x 1440; fog of war off.
    let aspect = 2560.0 / 1440.0;
    let mut ahead = PlayAhead::new(&world, aspect, true);
    ahead.frame(&world);
    let start = Frame::new(world.terrain().width, world.terrain().height);
    let mut changes = world.height_changes();
    while world.tick() < 6000 {
        ai.step(&mut world);
        ahead.frame(&world);
        // The drawn rectangle kept while the ground stands still is the one
        // taken anew (checked whenever the ground moved, and every 500).
        if world.height_changes() != changes || world.tick().is_multiple_of(500) {
            changes = world.height_changes();
            assert_eq!(
                ahead.drawn_rect(),
                drawn_field_rect(world.terrain(), &start, ahead.clip_from_world),
                "tick {}",
                world.tick()
            );
        }
    }
    // The paused jump: one frame with the farm scene's camera.
    let eye = world3d::Eye {
        position: [307.625, 36.113274, -172.04248],
        direction: [0.0, -0.6, 0.8],
    };
    let clip = clip_from_world(eye, aspect);
    let frame = Frame::new(world.terrain().width, world.terrain().height);
    let points = crate::world::scene3d::seen_points(&world, &frame);
    let mut marks = ahead.marks;
    let area = drawn_field_rect(world.terrain(), &frame, clip);
    marks.frame(world.explorations(), &points, area, |_| true);
    let seen = |object: u32| {
        let view = world
            .settler_views()
            .into_iter()
            .find(|v| v.object == object)
            .unwrap_or_else(|| panic!("no settler {object}"));
        let id = match view.id {
            s2_sim::view::SettlerRef::Settler(n) => crate::world::scene3d::MoverId::Settler(n),
            s2_sim::view::SettlerRef::Carrier(n) => crate::world::scene3d::MoverId::Carrier(n),
            s2_sim::view::SettlerRef::Builder(at) => crate::world::scene3d::MoverId::Builder(at),
            s2_sim::view::SettlerRef::Leveller(at) => crate::world::scene3d::MoverId::Leveller(at),
        };
        let point = points.iter().find(|p| p.id == id).expect("a seen point");
        (
            in_picture(clip, point.at),
            area.is_some_and(|a| a.contains(point.field)),
            marks.mark(id) == Some(true),
        )
    };
    // (in the picture, in the drawn rectangle, seen). Carrier 90554 walks
    // below the picture's lower edge in ours at this tick (the shots run's
    // is in it and drawn; our game is not that run's to the tick, Q-1030),
    // but on a terrain cell the camera sees (5.4 f): walking, he is seen.
    let got = [90750, 90554, 90640, 90480].map(|o| (o, seen(o)));
    assert_eq!(
        got,
        [
            (90750, (true, true, true)),
            (90554, (false, true, true)),
            (90640, (true, true, false)),
            (90480, (true, true, false)),
        ]
    );
}

/// GFX-WATER-2.6b (Q-1028): at MIRROR DETAILS n = 3 the mirror draws the
/// land and the settlers with what they carry, not the other objects; at
/// n = 4 everything; from a default camera the sky alone.
// spec: GFX-WATER-2.6b
#[test]
fn mirror_draws_the_settlers_from_details_three() {
    let sees = |camera: &RenderLayers, thing: &RenderLayers| camera.intersects(thing);
    let flat = -0.3;
    let three = mirror_camera_layers(world3d::mirror_content(flat, 0.75));
    assert!(sees(&three, &mirrored_settler_layers()));
    assert!(sees(&three, &mirrored_layers()));
    assert!(!sees(&three, &mirrored_object_layers()));
    let two = mirror_camera_layers(world3d::mirror_content(flat, 0.5));
    assert!(!sees(&two, &mirrored_settler_layers()));
    let four = mirror_camera_layers(world3d::mirror_content(flat, 1.0));
    assert!(sees(&four, &mirrored_settler_layers()) && sees(&four, &mirrored_object_layers()));
    let steep = mirror_camera_layers(world3d::mirror_content(-0.7, 1.0));
    assert!(!sees(&steep, &mirrored_settler_layers()) && !sees(&steep, &mirrored_layers()));
    assert!(sees(&steep, &mirrored_sky_layers()));
}

/// GFX-WATER-2.6a 1, 3: the mirror camera sees a world point where the main
/// camera sees it reflected in the water plane (the picture lines up with the
/// screen, not turned left to right), and its transform holds the reflection
/// (a negative determinant: the culling is reversed with it).
// spec: GFX-WATER-2.6a
#[test]
fn mirror_camera_is_the_main_view_of_the_reflected_world() {
    let main =
        Transform::from_xyz(30.0, 40.0, -12.0).looking_to(Vec3::new(0.3, -0.7, -0.6), Vec3::Y);
    let mirror = super::mirror_transform(&main);
    let main_view = main.to_matrix().inverse();
    let mirror_view = mirror.to_matrix().inverse();
    for p in [
        Vec3::new(31.0, 2.0, -40.0),
        Vec3::new(10.0, 9.5, -25.0),
        Vec3::new(55.0, 0.5, -30.0),
    ] {
        let reflected = Vec3::from(world3d::reflect_in_water(p.to_array()));
        let a = mirror_view.transform_point3(p);
        let b = main_view.transform_point3(reflected);
        assert!((a - b).length() < 1e-3, "{a} vs {b}");
    }
    assert!(mirror.to_matrix().determinant() < 0.0);
    assert!(main.to_matrix().determinant() > 0.0);
}

/// Mean saturate(N . L) over a normal map for the light (tangent space)
/// `l`, N = 2 x texel - 1 filtered bilinearly (wrapping) at `count` fixed
/// pseudo-random points, optionally normalised.
fn mean_lambert(rgba: &[u8], size: u32, l: [f32; 3], normalised: bool, count: u32) -> f32 {
    let texel = |x: i64, y: i64| {
        let (x, y) = (x.rem_euclid(size as i64), y.rem_euclid(size as i64));
        let at = ((y * size as i64 + x) * 4) as usize;
        [0, 1, 2].map(|c| f32::from(rgba[at + c]) / 255.0)
    };
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 40) as f32 / (1u64 << 24) as f32
    };
    let mut sum = 0.0f64;
    for _ in 0..count {
        let (u, v) = (next() * size as f32 - 0.5, next() * size as f32 - 0.5);
        let (x0, y0) = (u.floor(), v.floor());
        let (fx, fy) = (u - x0, v - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let mut t = [0.0f32; 3];
        for (dx, dy, w) in [
            (0, 0, (1.0 - fx) * (1.0 - fy)),
            (1, 0, fx * (1.0 - fy)),
            (0, 1, (1.0 - fx) * fy),
            (1, 1, fx * fy),
        ] {
            let c = texel(x0 + dx, y0 + dy);
            for k in 0..3 {
                t[k] += w * c[k];
            }
        }
        let mut n = t.map(|c| c * 2.0 - 1.0);
        if normalised {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            n = n.map(|c| c / len);
        }
        sum += f64::from((n[0] * l[0] + n[1] * l[1] + n[2] * l[2]).clamp(0.0, 1.0));
    }
    (sum / f64::from(count)) as f32
}

/// GFX-VIEW-2.4b, T41: `z_seaground_00_n` (512 x 512) goes to the 512
/// array with the file's own mip levels, and with N as filtered (not
/// normalised) its mean saturate(N . L) under the cell63 noon light is
/// 0.567 (the original); normalised 0.629; resampled to 256 and normalised
/// (the program before 2.4b) 0.682. Under the hour-23 light: 0.232 / 0.258
/// / 0.231. Flat ground: T = (1, 0, 0), N = (0, 1, 0), B = T x N, so the
/// light toward the sun in (T, B, N) is (-d.x, -d.z, -d.y).
/// Skipped without `S2_GAME_DIR`.
// spec: GFX-VIEW-2.4b (T41)
#[test]
fn t41_the_sea_grounds_normal_map_at_its_own_size() {
    let Ok(dir) = std::env::var("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let rel = "data/map/textures/z_seaground_00_n.dds";
    let path = s2_rules::resolve(std::path::Path::new(&dir), rel).expect(rel);
    let data = std::fs::read(&path).expect(rel);
    let (map, levels) =
        s2_assets::dds::load_with_levels(&path.to_string_lossy(), &data).expect(rel);
    assert_eq!((map.width, map.height), (512, 512));
    let class = normal_class(map.width, map.height);
    assert_eq!(NORMAL_SIZES[class], 512);
    let chain = crate::render::textures::chain_at(&map, &levels, 512);
    // The top level is the file's own; below it the file's own levels.
    assert_eq!(&chain[..512 * 512 * 4], &map.rgba[..]);
    assert_eq!(&chain[512 * 512 * 4..][..256 * 256 * 4], &levels[0].2[..]);
    let toward = |d: [f32; 3]| {
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        [-d[0] / len, -d[2] / len, -d[1] / len]
    };
    let noon = toward([0.492404, -0.766044, 0.413176]);
    let night = toward([0.129134, -0.255417, 0.958168]);
    let n = 200_000;
    let top = &chain[..512 * 512 * 4];
    let small = crate::render::textures::resample(&map, 256);
    for (l, want) in [
        (noon, [0.567, 0.629, 0.682]),
        (night, [0.232, 0.258, 0.231]),
    ] {
        let got = [
            mean_lambert(top, 512, l, false, n),
            mean_lambert(top, 512, l, true, n),
            mean_lambert(&small, 256, l, true, n),
        ];
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 0.004, "{got:?} against {want:?}");
        }
    }
}

/// GFX-VIEW-2.4b: the shader samples a layer's normal map from the array of
/// its size and uses N = 2 x texel - 1 as filtered, not normalised; a map
/// goes to the array of its size (the smallest it fits, the largest when
/// none), and one without the file's whole chain gets a box-filtered one.
// spec: GFX-VIEW-2.4b
#[test]
fn normal_maps_lie_at_their_own_size() {
    let src = include_str!("../shaders/terrain.wesl");
    assert!(src.contains("let n_ts = bump_texel.rgb * 2.0 - 1.0;"));
    assert!(!src.contains("normalize(bump"));
    assert!(!src.contains("let n_ts = normalize("));
    for (binding, name) in [
        (12, "normals_256"),
        (13, "normals_512"),
        (14, "normals_1024"),
    ] {
        assert!(src.contains(&format!(
            "@group(3) @binding({binding}) var {name}: texture_2d_array<f32>;"
        )));
    }
    assert_eq!(normal_class(128, 128), 0);
    assert_eq!(normal_class(256, 256), 1);
    assert_eq!(normal_class(256, 512), 2);
    assert_eq!(normal_class(1024, 1024), 3);
    assert_eq!(normal_class(2048, 2048), 3);
    assert_eq!(normal_class(100, 100), 0);
    // A 4 x 4 map whose file stores no lower levels: box-filtered.
    let flat = s2_assets::dds::Texture {
        width: 4,
        height: 4,
        format: s2_assets::dds::Format::Dxt1,
        mip_levels: 1,
        cube_map: false,
        rgba: [10, 20, 30, 40].repeat(16),
    };
    let chain = crate::render::textures::chain_at(&flat, &[], 4);
    assert_eq!(chain, [10, 20, 30, 40].repeat(16 + 4 + 1));
    // With its own levels (here deliberately different): kept as stored.
    let levels = vec![(2, 2, [1, 2, 3, 4].repeat(4)), (1, 1, vec![5, 6, 7, 8])];
    let chain = crate::render::textures::chain_at(&flat, &levels, 4);
    assert_eq!(&chain[64..80], &[1, 2, 3, 4].repeat(4)[..]);
    assert_eq!(&chain[80..], &[5, 6, 7, 8]);
}

/// GFX-VIEW-2.4b: the normal arrays are sampled trilinearly with wrap
/// addressing and no anisotropy, whatever the colour textures use.
// spec: GFX-VIEW-2.4b
#[test]
fn normal_maps_are_trilinear() {
    let bevy::image::ImageSampler::Descriptor(d) = normal_sampler() else {
        panic!("a sampler descriptor");
    };
    use bevy::image::{ImageAddressMode, ImageFilterMode};
    assert_eq!(
        (d.address_mode_u, d.address_mode_v),
        (ImageAddressMode::Repeat, ImageAddressMode::Repeat)
    );
    assert_eq!(
        (d.mag_filter, d.min_filter, d.mipmap_filter),
        (
            ImageFilterMode::Linear,
            ImageFilterMode::Linear,
            ImageFilterMode::Linear
        )
    );
    assert_eq!(d.anisotropy_clamp, 1);
}

/// GFX-VIEW-6.3 (T44): the cap's textures at hour 0.275 of the default
/// table: T0 the stars turned by -45 degrees, T1 the clouds turned by 90,
/// 0.275 of the clouds; at noon the clouds alone.
// spec: GFX-VIEW-6.3 (T44)
#[test]
fn t44_the_caps_material_mixes_the_entries_textures() {
    let mut images = Assets::<Image>::default();
    let clouds = images.add(Image::default());
    let stars = images.add(Image::default());
    let textures = SkyTextures {
        clouds: clouds.clone(),
        stars: stars.clone(),
    };
    let night = world3d::DayLight {
        skydome: 1,
        next_skydome: 0,
        blend: 0.275,
        ..world3d::DayLight::XML
    };
    let (first, second, extra) = textures.mix(&night);
    assert_eq!((first.id(), second.id()), (stars.id(), clouds.id()));
    assert_eq!(extra[0], terrain3d::STARS_ROTATION);
    assert_eq!(extra[1], terrain3d::SKY_ROTATION);
    assert!((extra[2] - 0.275).abs() < 1e-6);
    let (first, second, extra) = textures.mix(&world3d::DayLight {
        blend: 0.4,
        ..world3d::DayLight::XML
    });
    assert_eq!((first.id(), second.id()), (clouds.id(), clouds.id()));
    assert_eq!(extra[2], 0.0);
    // The object shader's sky family blends.
    let shader = include_str!("../shaders/object.wesl");
    assert!(shader.contains("return in.color * mix(t0, t1, params.extra.z);"));
}

/// GFX-WATER-2.6c (f) 6: the mirror texture is read bilinearly with wrap
/// addressing.
// spec: GFX-WATER-2.6c
#[test]
fn the_mirror_texture_wraps() {
    let bevy::image::ImageSampler::Descriptor(d) = mirror::mirror_target(4, 2).sampler else {
        panic!("a sampler descriptor");
    };
    use bevy::image::{ImageAddressMode, ImageFilterMode};
    assert_eq!(
        (d.address_mode_u, d.address_mode_v),
        (ImageAddressMode::Repeat, ImageAddressMode::Repeat)
    );
    assert_eq!(
        (d.mag_filter, d.min_filter),
        (ImageFilterMode::Linear, ImageFilterMode::Linear)
    );
}

/// GFX-VIEW-2.5 c, f (T43): the shadow test takes the pixel's own
/// light-space depth with a constant bias and no offset along the normal.
// spec: GFX-VIEW-2.5 (T43)
#[test]
fn t43_the_shadow_lookup_has_no_normal_offset() {
    let light = build::shadow_light();
    assert!(light.shadow_maps_enabled);
    assert_eq!(light.shadow_normal_bias, 0.0);
    assert_eq!(light.shadow_depth_bias, build::SHADOW_DEPTH_BIAS);
}

/// GFX-VIEW-3.2 (Q-172): the camera speed is the profile's scroll speed
/// (SET-2 field 7), 0.25 without a file.
// spec: GFX-VIEW-3.2, SET-2
#[test]
fn the_camera_speed_is_the_profiles() {
    let mut menu = crate::menus::menu::MenuState::default();
    assert_eq!(camera::camera_speed(&menu), 0.25);
    menu.settings.cfg.user.scroll_speed = 0.5;
    assert_eq!(camera::camera_speed(&menu), 0.5);
}

/// GFX-OBJ-8.3c: a clip's particle entry starts once per play: at 0.5 s of
/// the leveller's shovel play, not again in the play's later loops (its
/// clock runs on), and at 0 s on the play's first frame (the forester).
// spec: GFX-OBJ-8.3c
#[test]
fn a_clip_particle_starts_once_per_play() {
    // The play's frames: 0, 0.3, 0.6, ... (clip seconds); the clip is
    // 2 s long, so it loops three times in 6 s.
    let frames: Vec<f32> = (0..20).map(|i| i as f32 * 0.3).collect();
    let starts = |start: f32| {
        let mut last = frames[0] - 1e-4;
        let mut n = 0;
        for &now in &frames {
            if clip_particle_starts(start, last, now) {
                n += 1;
            }
            last = now;
        }
        n
    };
    assert_eq!(starts(0.5), 1);
    assert_eq!(starts(0.0), 1);
    // A settler that joins a play after the entry's time starts none.
    assert!(!clip_particle_starts(0.5, 2.9999, 3.0));
}

/// GFX-OBJ-8.3b: the shipped table's two settler clips with a particle:
/// the leveller's shovel (`StoneDust` at 0.5 s) and the forester's
/// planting (`StoneDust` at 0 s); the builder's and the woodcutter's clips
/// have none. Skipped without `S2_GAME_DIR`.
// spec: GFX-OBJ-8.3b
#[test]
fn the_settler_clips_with_particles() {
    let Ok(dir) = std::env::var("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let mut library = crate::render::models3d::ModelLibrary::new(
        Some(std::path::Path::new(&dir)),
        Default::default(),
    );
    library.index_clip_sounds();
    assert_eq!(
        library.particles_of_entry("WorkLoop0Bulldozer"),
        [("StoneDust".to_owned(), 0.5)]
    );
    assert_eq!(
        library.particles_of_entry("WorkLoop0Forester"),
        [("StoneDust".to_owned(), 0.0)]
    );
    assert!(library
        .particles_of_entry("WorkLoop0Constructor")
        .is_empty());
    assert!(library.particles_of_entry("WorkLoop0Axe").is_empty());
    let mut names: Vec<&String> = library.entry_particles.keys().collect();
    names.sort();
    assert_eq!(names, ["WorkLoop0Bulldozer", "WorkLoop0Forester"]);
}

/// Play test F15 (GFX-ANIM-6.1a b, MIL-SOL-6.2): the soldier the
/// headquarters sends to a new barracks holds the headquarters' door open
/// while he steps from its field out to its flag (the door's pairing is the
/// building and its own flag, the neighbour the building faces, GFX-OBJ-2.1):
/// a soldier is among the walkers the doors read.
// spec: GFX-ANIM-6.1a, MIL-SOL-6.2
#[test]
fn a_soldier_walking_out_holds_the_door_open() {
    use crate::animation::object_anim::{door_triggers, Stepper, ENTER_DISTANCE, LEAVE_DISTANCE};
    use s2_ai::fixture::{world, BARRACKS, SOLDIER};
    let p0 = s2_sim::PlayerId(0);
    let hq = (20, 20);
    let mut world = world(&[0]);
    let barracks = (25, 22);
    world.submit(s2_sim::Command::PlaceBuilding {
        player: p0,
        building: BARRACKS,
        at: barracks,
    });
    world.step();
    // Its flag joined to the headquarters' by a road (the soldier's way).
    let flag = world.terrain().neighbour(hq, 1).expect("the flag");
    let site_flag = world.terrain().neighbour(barracks, 1).expect("its flag");
    world.submit(s2_sim::Command::BuildRoad {
        player: p0,
        from: flag,
        to: site_flag,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    let shortfall = world.site(barracks).expect("a site").shortfall();
    world.submit(s2_sim::Command::DebugSupplySite {
        player: p0,
        at: barracks,
        wares: shortfall,
        workers: true,
    });
    let doors: std::collections::HashMap<_, _> = [(hq, flag)].into();
    let mut held_by_soldier = false;
    for _ in 0..6000 {
        world.step();
        let steppers: Vec<Stepper> = world
            .settler_views()
            .iter()
            .filter(|v| SOLDIER.contains(&v.profession))
            .filter_map(|v| match v.id {
                s2_sim::view::SettlerRef::Settler(n) => Some(Stepper {
                    id: n as u64,
                    at: v.walk.at,
                    next: v.walk.next,
                    progress: v.walk.fraction(),
                }),
                _ => None,
            })
            .collect();
        if door_triggers(&doors, &steppers, ENTER_DISTANCE, LEAVE_DISTANCE).contains(&hq) {
            held_by_soldier = true;
            break;
        }
    }
    let soldiers: Vec<_> = world
        .settler_views()
        .iter()
        .filter(|v| SOLDIER.contains(&v.profession))
        .map(|v| (v.walk.at, v.walk.next))
        .collect();
    assert!(
        held_by_soldier,
        "no soldier stepped out of the headquarters' door: barracks {:?}, soldiers {soldiers:?}, tick {}",
        world.building_at(barracks).map(|b| b.under_construction),
        world.tick()
    );
}

/// Hook W33 (with GFX-OBJ-8.7 (3), 8.3c): an effect's particle is drawn in
/// its own colour from the effect file and its alpha, whichever effect it
/// is (a project's, the collapse dust, a clip's: one function draws all
/// three); a quad without a particle shows nothing.
// spec: GFX-OBJ-8.7, GFX-OBJ-8.3c, GFX-VIEW-2.6 (hook W33)
#[test]
fn an_effects_particles_take_the_files_colour() {
    let mut materials = Assets::<StandardMaterial>::default();
    let pool: Vec<(Entity, Handle<StandardMaterial>)> = (0..2u32)
        .map(|n| {
            (
                Entity::from_raw_u32(n + 1).unwrap(),
                materials.add(StandardMaterial::default()),
            )
        })
        .collect();
    let wood = crate::world::decor::Particle {
        at: [1.0, 2.0, 3.0],
        size: 0.25,
        alpha: 0.5,
        colour: [0.6, 0.3, 0.0],
    };
    let drawn = draw_particles(
        &pool,
        vec![Some(wood), None],
        [10.0, 20.0, 0.0],
        Quat::IDENTITY,
        &mut materials,
    );
    let colour = materials.get(&pool[0].1).unwrap().base_color;
    assert_eq!(colour, Color::linear_rgba(0.6, 0.3, 0.0, 0.5));
    assert_ne!(colour, Color::srgba(1.0, 1.0, 1.0, 0.5));
    assert_eq!(drawn[0].0, pool[0].0);
    assert_eq!(drawn[0].1.scale, Vec3::splat(0.25));
    assert_eq!(
        drawn[0].1.translation,
        Vec3::from(world3d::to_bevy([11.0, 22.0, 3.0]))
    );
    assert_eq!(drawn[1].1.scale, Vec3::ZERO);
    assert_eq!(
        materials.get(&pool[1].1).unwrap().base_color,
        StandardMaterial::default().base_color
    );
}

/// How many places (0.05 apart) a ray of length 1000 meets the field-point
/// surface of `heights` (GFX-VIEW-1.5's triangles, every cell).
fn surface_hits(heights: &impl world3d::Heights, origin: [f32; 3], direction: [f32; 3]) -> usize {
    use world3d::sectors::field_point;
    let frame = heights.frame();
    let (w, h) = ((frame.columns - 1) / 4, (frame.rows - 1) / 4);
    let mut ts: Vec<f32> = Vec::new();
    for y in 0..h - 1 {
        for x in 0..w - 1 {
            let tris = if y % 2 == 0 {
                [
                    [(x, y), (x + 1, y), (x, y + 1)],
                    [(x + 1, y), (x + 1, y + 1), (x, y + 1)],
                ]
            } else {
                [
                    [(x, y), (x + 1, y + 1), (x, y + 1)],
                    [(x, y), (x + 1, y), (x + 1, y + 1)],
                ]
            };
            for tri in tris {
                let [a, b, c] = tri.map(|(x, y)| field_point(heights, x, y));
                if let Some(t) = world3d::ray_triangle_for_tests(origin, direction, a, b, c) {
                    if (0.0..=1000.0).contains(&t) && ts.iter().all(|u| (u - t).abs() > 0.05) {
                        ts.push(t);
                    }
                }
            }
        }
    }
    ts.len()
}

// GFX-VIEW-1.5 (Q-1067) walk step 1: a box against the view's six planes
// (the camera's frustum, Bevy frame; boxes in the game frame, z turned):
// in front and small, wholly inside; behind the eye, outside; around the
// eye, across.
// spec: GFX-VIEW-1.5
#[test]
fn a_box_against_the_views_planes() {
    use bevy::camera::CameraProjection;
    use world3d::sectors::{Bounds, Seen};
    let projection = PerspectiveProjection {
        fov: 0.8,
        aspect_ratio: 1.6,
        near: 1.0,
        far: 500.0,
        ..default()
    };
    // At the origin looking along the game's +z (Bevy's -z).
    let eye = Transform::from_translation(Vec3::ZERO).looking_to(Vec3::NEG_Z, Vec3::Y);
    let frustum = projection.compute_frustum(&GlobalTransform::from(eye));
    let cube = |z: f32, half: f32| Bounds {
        min: [-half, -half, z - half],
        max: [half, half, z + half],
    };
    assert_eq!(seen_by(&frustum, &cube(50.0, 2.0)), Seen::Inside);
    assert_eq!(seen_by(&frustum, &cube(-50.0, 2.0)), Seen::Outside);
    assert_eq!(seen_by(&frustum, &cube(0.0, 5.0)), Seen::Straddles);
    // Beyond the far plane.
    assert_eq!(seen_by(&frustum, &cube(600.0, 2.0)), Seen::Outside);
}

// Hook W17b: off (the default) the map camera gets nothing more, and every
// pixel is the original's: the shaders read the occlusion only under Bevy's
// SCREEN_SPACE_AMBIENT_OCCLUSION definition, which only a view with the
// component sets (1 otherwise), and only multiply the ambient term by it.
// On: Bevy's occlusion with multisampling off; the terrain and the objects
// draw the depth and normal prepass it needs (their own prepass shaders).
// spec: none (hook W17b: no ambient occlusion by default, a project's on the ambient term only)
#[test]
fn ambient_occlusion_is_a_projects_option() {
    assert!(!crate::extend::RenderOptions::default().ambient_occlusion);
    assert!(ambient_occlusion(false).is_none());
    let (ssao, msaa) = ambient_occlusion(true).expect("the parts");
    assert_eq!(
        ssao.quality_level,
        bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::High
    );
    assert_eq!(msaa, bevy::render::view::Msaa::Off);
    // The flat surface's value divided out: 1 - 1/6 at High quality, the
    // same number in both shaders.
    assert!((AO_FLAT - 0.8333333).abs() < 1e-6);
    for (name, src) in [
        ("terrain", include_str!("../shaders/terrain.wesl")),
        ("object", include_str!("../shaders/object.wesl")),
    ] {
        assert!(
            src.contains(
                "@if(SCREEN_SPACE_AMBIENT_OCCLUSION) {\n        let p = vec2<i32>(frag.xy - view.viewport.xy);\n        ao = min(textureLoad(screen_space_ambient_occlusion_texture, p, 0i).r / AO_FLAT, 1.0);"
            ),
            "{name}"
        );
        assert!(src.contains("var ao = 1.0;"), "{name}");
        assert!(src.contains("const AO_FLAT: f32 = 0.8333333;"), "{name}");
    }
    let terrain = include_str!("../shaders/terrain.wesl");
    assert!(terrain.contains("+ params.ambient.rgb * occlusion;"));
    let object = include_str!("../shaders/object.wesl");
    assert!(object.contains("let ambient = light.ambient.rgb * occlusion;"));
    assert!(object.contains("+ light.ambient.w * occlusion;"));
    // Every prepass writes the surface's own normal (screen derivatives of
    // the world position, toward the camera) in Bevy's encoding.
    for src in [
        include_str!("../shaders/terrain_prepass.wesl"),
        include_str!("../shaders/object_shadow.wesl"),
    ] {
        assert!(src.contains("@if(NORMAL_PREPASS)"));
        assert!(src.contains("normalize(cross(dpdx(p), dpdy(p)))"));
        assert!(src.contains("if direction_world_to_view(n).z < 0.0 {"));
        assert!(src.contains("* 0.5 + vec3<f32>(0.5), 1.0)"));
    }
}

// GFX-ANIM-3.8a (Q-202 c, E-gfx-077): a felled tree's top moves toward the
// woodcutter's left as he faces it, (z, 0, -x) for the unit vector (x, 0, z)
// from the tree to him: woodcutter at (0, 0, -3), the top toward -x; at
// (3, 0, 0), toward -z (game frame). The axis is the scene's, the yaw of
// the line to him.
// spec: GFX-ANIM-3.8a
#[test]
fn a_felled_tree_falls_to_the_woodcutters_left() {
    let top = |woodcutter: [f32; 3]| {
        let axis = woodcutter[2].atan2(woodcutter[0]);
        let placed = crate::world::scene3d::Placed {
            key: (crate::world::scene3d::Kind::Deposit, (0, 0), 0),
            file: String::new(),
            units_per_metre: crate::render::models3d::DEFAULT_SCALE,
            at: [0.0; 3],
            heading: 0.7,
            scale: [1.0; 3],
            tilt_axis: axis,
            tilt: 0.5,
            tint: [1.0; 4],
            family: crate::render::models3d::Family::Plain,
            player: [0.0; 3],
            extra: [0.0; 4],
            shadow: true,
            pieces: None,
            plane_of: None,
            parts: crate::world::scene3d::Parts::All,
            sink: None,
        };
        let t = placed_transform(&placed);
        world3d::from_bevy((t.rotation * Vec3::Y).to_array())
    };
    let s = 0.5f32.sin();
    let a = top([0.0, 0.0, -3.0]);
    assert!((a[0] + s).abs() < 1e-5 && a[2].abs() < 1e-5, "{a:?}");
    let b = top([3.0, 0.0, 0.0]);
    assert!((b[2] + s).abs() < 1e-5 && b[0].abs() < 1e-5, "{b:?}");
}

// spec: none (hook W30: a project building's smoke rises from its start for as long as the building works)
#[test]
fn a_project_buildings_effect_runs_while_it_works() {
    // Not working: none.
    assert!(!building_effect_on(None, 0.0, 5.0));
    // From the work's start (a start of 0 at once).
    assert!(building_effect_on(Some(5.0), 0.0, 5.0));
    // A later start in clip seconds, 1.8 of them a game second.
    assert!(!building_effect_on(Some(5.0), 0.9, 5.4));
    assert!(building_effect_on(Some(5.0), 0.9, 5.5));
    // And on, as long as the work goes on.
    assert!(building_effect_on(Some(5.0), 0.9, 500.0));
}

/// Hook W30b: a tree shows its far version while its size on the screen is
/// below the share: a 9 m tree in a view of span 0.5246 (29.4 degrees) is
/// 0.1 of the view's height at depth 171.6; behind the camera never.
// spec: none (hook W30b: the far version by the size on the screen)
#[test]
fn a_tree_far_away_shows_its_far_version() {
    let span = 2.0 * (29.4f32.to_radians() * 0.5).tan();
    assert!((span - 0.5246).abs() < 1e-3);
    let at = 9.0 / (0.1 * span);
    assert!(!far_version(9.0, at - 1.0, span, 0.1));
    assert!(far_version(9.0, at + 1.0, span, 0.1));
    assert!(!far_version(9.0, -500.0, span, 0.1));
    // A share of 0: never far.
    assert!(!far_version(9.0, 1.0e6, span, 0.0));
}

/// Hook W30b: the default switch in the game's camera (GFX-VIEW-3.3, 3.5):
/// in a 16:9 view a grown 10 m tree at the farthest zoom is far from the
/// view's top to its bottom, at the default zoom full but along its far
/// edge, at zoom 2 full everywhere; a sapling a tenth of the size is far.
// spec: GFX-VIEW-3.3, GFX-VIEW-3.5 (the depths of the view's top, middle
// and bottom by zoom); none (hook W30b: the default switch)
#[test]
fn the_default_switch_by_zoom() {
    use crate::render::world3d::{vertical_fov, DISTANCE_PER_ZOOM};
    let fov = vertical_fov(16.0 / 9.0);
    let span = 2.0 * (fov * 0.5).tan();
    // The eye 0.8815 x the distance above the ground (the back direction
    // of pitch -0.8 with y x 1.4), the view 53.13 degrees down.
    let depths = |zoom: u8| {
        let h = 0.8815 * DISTANCE_PER_ZOOM * f32::from(zoom);
        let pitch = 0.8f32.asin();
        let half = fov * 0.5;
        [half, 0.0, -half].map(|off: f32| h / (pitch - off).sin() * off.cos())
    };
    let far = |size: f32, zoom: u8| {
        depths(zoom).map(|d| far_version(size, d, span, crate::extend::TREE_FAR_BELOW))
    };
    assert_eq!(far(10.0, 4), [true, true, true]);
    assert_eq!(far(10.0, 3), [true, false, false]);
    assert_eq!(far(9.0, 3), [true, true, false]);
    assert_eq!(far(10.0, 2), [false, false, false]);
    assert_eq!(far(1.0, 2), [true, true, true]);
}

/// Hook W30b: `tree_versions` shows each tree's one version by the main
/// camera (not the mirror's), touching only trees whose version changes.
// spec: none (hook W30b: each tree one version, by the main camera)
#[test]
fn tree_versions_follow_the_camera() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    let projection = Projection::Perspective(PerspectiveProjection {
        fov: 29.4f32.to_radians(),
        ..default()
    });
    let camera = world
        .spawn((
            MapCamera3d,
            Transform::from_xyz(0.0, 0.0, 0.0).looking_to(-Vec3::Z, Vec3::Y),
            projection.clone(),
        ))
        .id();
    // The mirror's camera far below: not asked.
    world.spawn((
        MapCamera3d,
        MirrorCamera3d,
        Transform::from_xyz(0.0, -1.0e4, 0.0).looking_to(-Vec3::Z, Vec3::Y),
        projection,
    ));
    let tree = |world: &mut World, z: f32| {
        let near = world
            .spawn((TreeVersion { far: false }, Visibility::Inherited))
            .id();
        let far = world
            .spawn((TreeVersion { far: true }, Visibility::Hidden))
            .id();
        let parent = world
            .spawn((
                Transform::from_xyz(0.0, -4.5, z),
                ProjectTree {
                    extent: [6.0, 9.0, 6.0],
                    far_below: 0.1,
                    far: None,
                },
            ))
            .add_children(&[near, far])
            .id();
        (parent, near, far)
    };
    let (close, close_near, close_far) = tree(&mut world, -50.0);
    let (distant, distant_near, distant_far) = tree(&mut world, -500.0);
    world.run_system_once(tree_versions).expect("runs");
    let shown = |world: &World, e: Entity| *world.get::<Visibility>(e).expect("visibility");
    assert_eq!(shown(&world, close_near), Visibility::Inherited);
    assert_eq!(shown(&world, close_far), Visibility::Hidden);
    assert_eq!(shown(&world, distant_near), Visibility::Hidden);
    assert_eq!(shown(&world, distant_far), Visibility::Inherited);
    assert_eq!(world.get::<ProjectTree>(close).unwrap().far, Some(false));
    assert_eq!(world.get::<ProjectTree>(distant).unwrap().far, Some(true));
    // The camera moves close to the distant one: it turns to its model.
    world
        .get_mut::<Transform>(camera)
        .expect("camera")
        .translation
        .z = -460.0;
    world.run_system_once(tree_versions).expect("runs");
    assert_eq!(shown(&world, distant_near), Visibility::Inherited);
    assert_eq!(shown(&world, distant_far), Visibility::Hidden);
    // Grown to a tenth of its size it is small again: far.
    world.get_mut::<Transform>(distant).expect("tree").scale = Vec3::splat(0.1);
    world.run_system_once(tree_versions).expect("runs");
    assert_eq!(shown(&world, distant_far), Visibility::Inherited);
}
