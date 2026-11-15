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

// Picking over hills: a field's ground point drawn on the screen picks
// that field again, at every zoom and turn of the camera, wherever the
// point is not hidden behind a nearer hill. A flat plane at height 0
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
                    let hit = world3d::ray_ground(&hills, origin, direction, 2000.0)
                        .unwrap_or_else(|| panic!("no ground under ({x}, {y})"));
                    // Hidden behind a nearer hill: the ray meets that one.
                    if dist(hit) < s - 0.05 {
                        continue;
                    }
                    assert_eq!(
                        world3d::ray_field(&hills, origin, direction),
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
