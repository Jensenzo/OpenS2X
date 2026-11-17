use super::*;

/// GFX-VIEW-6.3: the sun quad at the light direction x 100 from the
/// camera, 30 wide: pushed back 4.5 times at 4.5 times the size, so it
/// covers the same angle (30 / 100); the moon only at elevation <= 0.
#[test]
fn sun_and_moon_quads() {
    let light = light_direction(LIGHT_ELEVATION, LIGHT_HEADING);
    let (centre, side) = sky_sprite([10.0, 40.0, -5.0], light, SUN_SIZE);
    let d = [centre[0] - 10.0, centre[1] - 40.0, centre[2] + 5.0];
    let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    assert!((side / dist - SUN_SIZE / SKY_SPRITE_DISTANCE).abs() < 1e-6);
    assert!(dist < FAR);
    for k in 0..3 {
        assert!((d[k] / dist - light[k]).abs() < 1e-5);
    }
    assert!(!moon_shown(LIGHT_ELEVATION));
    assert!(moon_shown(0.0) && moon_shown(-5.0));
}

/// GFX-WATER-2.5: frames advance 16 per 5 s: at t 0 frame 0 (blend
/// from 15, weight 0), at 1.0 s frame 3 with weight 0.2, at 4.99 s
/// frame 15, at 5 s frame 0 again.
#[test]
fn caustics_frames_loop_every_5_s() {
    assert_eq!(caustics_frames(0.0), (0, 15, 0.0));
    let (f, prev, w) = caustics_frames(1.0);
    assert_eq!((f, prev), (3, 2));
    assert!((w - 0.2).abs() < 1e-4);
    assert_eq!(caustics_frames(4.99).0, 15);
    assert_eq!(caustics_frames(5.0).0, 0);
}

/// GFX-WATER-2.5: (x / 16, z) x 0.04 plus the frame's slot: x 0, z 10
/// in frame 2 is (0.125, 0.4); x 400 moves u by one slot (0.0625).
#[test]
fn caustics_texture_coordinates() {
    let [u, v] = caustics_uv(0.0, 10.0, 2);
    assert!((u - 0.125).abs() < 1e-6 && (v - 0.4).abs() < 1e-6);
    let [u2, _] = caustics_uv(25.0, 10.0, 2);
    assert!((u2 - 0.1875).abs() < 1e-6);
    let [_, v2] = caustics_uv(0.0, -10.0, 0);
    assert!((v2 - 0.6).abs() < 1e-5);
}

/// GFX-WATER-2.5: nothing at or above y -0.1; at t 0 (s 0), x = z = 0
/// the wave term is 1, so the scale is 0.7 (1 - min(1, -0.15 y)): 0.595
/// at y -1, 0.35 at y -10 / 3, 0 from y -6.67 down; elsewhere V twice.
// spec: GFX-WATER-2.5
#[test]
fn caustics_brightness() {
    assert_eq!(caustics_scale(3.0, -0.1, 3.0, 1.0), 0.0);
    assert_eq!(caustics_scale(3.0, 2.0, 3.0, 1.0), 0.0);
    assert!((caustics_scale(0.0, -1.0, 0.0, 0.0) - 0.595).abs() < 1e-6);
    assert!((caustics_scale(0.0, -10.0 / 3.0, 0.0, 0.0) - 0.35).abs() < 1e-6);
    assert_eq!(caustics_scale(0.0, -7.0, 0.0, 0.0), 0.0);
    // s = sin(t): the three sines of 0.4 (x, y, z) + s.
    let (x, y, z, t) = (1.0f32, -2.0f32, 3.0f32, 0.5f32);
    let s = t.sin();
    let v = 1.0 - (0.4 * x + s).sin() * (0.4 * y + s).sin() * (0.4 * z + s).sin();
    // V enters twice.
    let want = 0.7 * v * v * (1.0 - 0.3);
    assert!((caustics_scale(x, y, z, t) - want).abs() < 1e-6);
}

/// A flat grid of a given size and height, or a function of the sample.
struct Grid {
    frame: Frame,
    height: fn(u32, u32) -> i32,
}

impl Heights for Grid {
    fn frame(&self) -> Frame {
        self.frame
    }
    fn raw(&self, c: u32, r: u32) -> i32 {
        (self.height)(c, r)
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

// GFX-VIEW T1: a 100 x 100 map has a 401 x 401 fine grid centred on the
// origin.
// spec: GFX-VIEW-1.1, MAP-GRID-5.2 (GFX-VIEW T1)
#[test]
fn fine_grid_is_centred() {
    let frame = Frame::new(100, 100);
    assert_eq!((frame.columns, frame.rows), (401, 401));
    let [x, z] = frame.sample_xz(0, 0);
    assert!(close(x, -300.0) && close(z, -259.8075));
    let [x, z] = frame.sample_xz(400, 400);
    assert!(close(x, 300.0) && close(z, 259.8075));
    let [x, z] = frame.sample_xz(1, 1);
    assert!(close(x, -297.75) && close(z, -258.5085));
}

// The 2D view's plane position of a field is its offset from sample
// (0, 0) (MAP-GRID-5.3).
#[test]
fn plane_positions_match_the_2d_view() {
    let frame = Frame::new(80, 80);
    let [x, z] = frame.field_xz(45, 39);
    let [u, v] = frame.to_plane(x, z);
    assert!(close(u, 273.0) && close(v, 202.64985));
}

// GFX-VIEW-1.4: on a plane the ground height is that plane, between
// samples too; at a sample it is the sample's height.
#[test]
fn ground_height_follows_the_lattice_planes() {
    let grid = Grid {
        frame: Frame::new(10, 10),
        height: |c, _| c as i32 * 1000,
    };
    // h = 1000 raw per column: world height = (x - x0) / 1.5 minus the
    // half step of odd rows, which is a plane only within a row pair;
    // at samples it is exact.
    let [x, z] = grid.frame.sample_xz(6, 4);
    assert!(close(ground_height(&grid, x, z), 6.0));
    let flat = Grid {
        frame: Frame::new(10, 10),
        height: |_, _| 2500,
    };
    let [x, z] = flat.frame.sample_xz(7, 9);
    assert!(close(ground_height(&flat, x + 0.4, z + 0.3), 2.5));
    // Half way between two samples of one row: their mean.
    let [x, z] = grid.frame.sample_xz(6, 4);
    assert!(close(ground_height(&grid, x + 0.75, z), 6.5));
}

// GFX-VIEW-2.2: flat ground has the normal straight up; a slope rising
// toward +x tilts it toward -x.
#[test]
fn grid_normals() {
    let flat = Grid {
        frame: Frame::new(4, 4),
        height: |_, _| 0,
    };
    assert_eq!(grid_normal(&flat, 5, 5), [0.0, 1.0, 0.0]);
    let slope = Grid {
        frame: Frame::new(4, 4),
        height: |c, _| c as i32 * 1500,
    };
    let n = grid_normal(&slope, 5, 5);
    // Rise 1.5 per 1.5 units: 45 degrees.
    assert!(close(n[0], -std::f32::consts::FRAC_1_SQRT_2));
    assert!(close(n[1], std::f32::consts::FRAC_1_SQRT_2));
    assert!(close(n[2], 0.0));
}

// GFX-VIEW T7: the shipped light.
// spec: GFX-VIEW-2.3 (GFX-VIEW T7)
#[test]
fn shipped_light_direction() {
    let [x, y, z] = light_direction(LIGHT_ELEVATION, LIGHT_HEADING);
    assert!((x - 0.5265).abs() < 1e-4 && (y + 0.7660).abs() < 1e-4);
    assert!((z - 0.3687).abs() < 1e-4);
}

// GFX-VIEW T14: the shipped daytime on flat ground: lit and shadowed
// D, and a mid-grey texel under both.
// spec: GFX-VIEW-2.4, GFX-VIEW-2.5 (GFX-VIEW T14)
#[test]
fn terrain_light_and_shadow() {
    let n_dot_l = LIGHT_ELEVATION.to_radians().sin();
    let shadow = shadow_factor(LIGHT_ELEVATION, SHADOW_INTENSITY);
    let lit = light_d(n_dot_l, 1.0);
    let dark = light_d(n_dot_l, shadow);
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 2e-4);
    assert!(near(lit, [1.3182, 1.1879, 1.1983]), "{lit:?}");
    assert!(near(dark, [0.6718, 0.5688, 0.6598]), "{dark:?}");
    let texel = |d: [f32; 3]| d.map(|d| (d * 0.5).min(1.0));
    assert!(
        near(texel(lit), [0.659, 0.594, 0.599]) || {
            let t = texel(lit);
            t.iter()
                .zip(&[0.659, 0.594, 0.599])
                .all(|(a, b)| (a - b).abs() < 1e-3)
        }
    );
    let t = texel(dark);
    assert!(
        t.iter()
            .zip(&[0.336, 0.284, 0.330])
            .all(|(a, b)| (a - b).abs() < 1e-3),
        "{t:?}"
    );
    // GFX-VIEW-2.3: luminances of the shipped ambient and light.
    assert!((luminance(AMBIENT) - 0.5413).abs() < 1e-4);
    assert!((luminance(LIGHT_COLOR) - 0.8965).abs() < 1e-4);
}

// GFX-VIEW-2.3, GFX-OBJ-3.3: what the shaders are handed: the shadow
// factor 0.1 and the two luminances in the fourth values.
#[test]
fn shader_light_values() {
    let light = shader_light();
    assert!((light.direction[3] - 0.1).abs() < 1e-6);
    assert!((light.color[3] - 0.8965).abs() < 1e-4);
    assert!((light.ambient[3] - 0.5413).abs() < 1e-4);
    assert_eq!(light.ambient[..3], AMBIENT);
    assert!(light.direction[1] < 0.0);
}

// GFX-VIEW T15: the shadow factor by the sun's elevation.
// spec: GFX-VIEW-2.5 (GFX-VIEW T15)
#[test]
fn shadow_factor_by_elevation() {
    let got = [50.0, 40.0, 30.0, 20.0, 10.0].map(|e| shadow_factor(e, 0.9));
    for (g, want) in got.iter().zip([0.1, 0.1, 0.55, 1.0, 1.0]) {
        assert!((g - want).abs() < 1e-6, "{got:?}");
    }
}

// GFX-OBJ T15: a settler on flat open ground, texel green 0.8.
// spec: GFX-OBJ-3.3, GFX-VIEW-2.5 (GFX-OBJ T15)
#[test]
fn settler_light_by_luminance() {
    let n_dot_l = LIGHT_ELEVATION.to_radians().sin();
    let lit = settler_d(n_dot_l, 1.0);
    let dark = settler_d(n_dot_l, shadow_factor(LIGHT_ELEVATION, SHADOW_INTENSITY));
    assert!((lit - 1.228).abs() < 1e-3 && (dark - 0.610).abs() < 1e-3);
    assert!(((0.8 * lit).min(1.0) - 0.982).abs() < 1e-3);
    assert!(((0.8 * dark).min(1.0) - 0.488).abs() < 1e-3);
}

// GFX-VIEW T16, T17: the Roman headquarters on field (10, 10) of a
// 100 x 100 map at raw height 5000: its matrix, its front toward its
// flag, the door bones and a corner of its ground decal.
// spec: GFX-VIEW-4.3 (GFX-VIEW T16, T17)
#[test]
fn headquarters_matrix() {
    let grid = Grid {
        frame: Frame::new(100, 100),
        height: |_, _| 5000,
    };
    let at = building_position(&grid, 10, 10);
    let m = building_matrix(at);
    let want = [
        [0.866025, 0.0, 0.5, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-0.5, 0.0, 0.866025, 0.0],
        [-240.0, 5.0, -207.846, 1.0],
    ];
    for (row, want) in m.iter().zip(&want) {
        for (a, b) in row.iter().zip(want) {
            assert!(close(*a, *b), "{m:?}");
        }
    }
    let p = |q: [f32; 3], want: [f32; 3]| {
        let w = transform_row(&m, q);
        assert!(
            close(w[0], want[0]) && close(w[1], want[1]) && close(w[2], want[2]),
            "{w:?}"
        );
    };
    // Model -z (the front) faces direction 1, toward the flag (10, 9).
    let front = transform_row(&m, [0.0, 0.0, -1.0]);
    assert!(close(front[0] - at[0], 0.5) && close(front[2] - at[2], -0.866));
    let [fx, fz] = grid.frame.field_xz(10, 9);
    assert!(close(fx, -237.0) && close(fz, -213.042));
    assert!(close(fx - at[0], 3.0) && close(fz - at[2], -5.196));
    p([-1.034, -0.044, 2.165], [-241.978, 4.956, -206.488]);
    p([0.873, -0.045, 2.164], [-240.326, 4.955, -205.535]);
    p([-6.619, 0.0, -2.173], [-244.646, 5.0, -213.037]);
}

// GFX-VIEW T18, GFX-OBJ T16: the Roman woodcutter's hut on field
// (11, 11) at raw height 4200: its door bone, base and a decal corner.
// spec: GFX-VIEW-4.3, GFX-OBJ-2.3 (GFX-VIEW T18, GFX-OBJ T16)
#[test]
fn woodcutter_hut_matrix() {
    let grid = Grid {
        frame: Frame::new(100, 100),
        height: |_, _| 4200,
    };
    let m = building_matrix(building_position(&grid, 11, 11));
    assert!(close(m[3][0], -231.0) && close(m[3][1], 4.2) && close(m[3][2], -202.6499));
    let door = transform_row(&m, [0.662, 0.02, -1.581]);
    assert!(close(door[0], -229.636) && close(door[1], 4.22) && close(door[2], -203.688));
    assert!(close(transform_row(&m, [0.0, -0.228, 0.0])[1], 3.972));
    let corner = transform_row(&m, [-3.478, 0.0, -3.38]);
    assert!(
        close(corner[0], -232.322) && close(corner[2], -207.316),
        "{corner:?}"
    );
}

// GFX-VIEW T19: a hut whose field sample lies in a dip keeps the
// sample's height, not its neighbours'.
// spec: GFX-VIEW-4.3 (GFX-VIEW T19)
#[test]
fn building_height_is_the_field_sample() {
    let grid = Grid {
        frame: Frame::new(100, 100),
        // Field (11, 11): sample (46, 44).
        height: |c, r| if (c, r) == (46, 44) { 4200 } else { 4500 },
    };
    assert!(close(building_position(&grid, 11, 11)[1], 4.2));
}

// GFX-VIEW T6 (heading part), 4.1: the heading table, and the step
// direction of two neighbours.
// spec: GFX-VIEW-4.1 (GFX-VIEW T6)
#[test]
fn headings() {
    assert!(close(heading(0), 90f32.to_radians()));
    assert!(close(heading(2), -150f32.to_radians()));
    assert_eq!(step_direction((10, 10), (11, 10)), Some(0));
    assert_eq!(step_direction((10, 11), (11, 12)), Some(5));
    assert_eq!(step_direction((10, 10), (12, 10)), None);
    // Facing +z rotated by the heading of direction 0 faces east (+x),
    // in Bevy's frame too.
    let yaw = bevy_yaw(heading(0));
    let facing = to_bevy([0.0, 0.0, 1.0]);
    let (s, c) = yaw.sin_cos();
    let turned = [
        facing[0] * c + facing[2] * s,
        -facing[0] * s + facing[2] * c,
    ];
    assert!(close(turned[0], 1.0) && close(turned[1], 0.0));
}

// GFX-VIEW T3: flat ground at 0 with water level 0, zoom 2, yaw 0: the
// view direction (0, -0.8, 0.6), 53.13 degrees down; the camera 42 units
// from the target along -normalize(0, -1.12, 0.6): 19.9 behind, 37.0
// above.
// spec: GFX-VIEW-3.3 (GFX-VIEW T3)
#[test]
fn camera_on_flat_ground() {
    let flat = Grid {
        frame: Frame::new(20, 20),
        height: |_, _| 0,
    };
    let mut rig = CameraRig::new(0.0, 0.0);
    let eye = rig.update(&flat);
    assert!(close(rig.target[1], 10.0));
    let d = eye.direction;
    assert!(
        close(d[0], 0.0) && close(d[1], -0.8) && close(d[2], 0.6),
        "{d:?}"
    );
    assert!(close(eye.position[0], 0.0));
    assert!((rig.target[2] - eye.position[2] - 19.85).abs() < 0.05);
    assert!((eye.position[1] - rig.target[1] - 37.02).abs() < 0.05);
    let distance = (0..3)
        .map(|i| (eye.position[i] - rig.target[i]).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!(close(distance, 42.0));
    // The pitches of zoom 0 (17.46 degrees, Q-205 a answered) and 1
    // (36.87 degrees).
    for (zoom, degrees) in [(0, 17.4576), (1, 36.8699)] {
        rig.zoom = zoom;
        let d = rig.direction();
        assert!((d[1].abs().asin().to_degrees() - degrees).abs() < 1e-3);
    }
    // A zoom change eases the view direction by 0.9, then 0.4 a frame.
    rig.zoom = 2;
    rig.update(&flat);
    rig.zoom = 1;
    let eye = rig.update(&flat);
    let want = normalize([0.0, -0.8 + (-0.6 + 0.8) * 0.9, 0.6 + (0.8 - 0.6) * 0.9]);
    assert!(
        (0..3).all(|i| close(eye.direction[i], want[i])),
        "{:?}",
        eye.direction
    );
}

/// Where a point is on the screen, as a share of the half height above
/// the centre (1: the top edge), seen from `eye` (GFX-VIEW-3.3, 3.5).
fn screen_height(eye: &Eye, point: [f32; 3], aspect: f32) -> f32 {
    let d = eye.direction;
    let right = normalize([-d[2], 0.0, d[0]]);
    let up = [
        right[1] * d[2] - right[2] * d[1],
        right[2] * d[0] - right[0] * d[2],
        right[0] * d[1] - right[1] * d[0],
    ];
    let p = [0, 1, 2].map(|i| point[i] - eye.position[i]);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let up = if up[1] < 0.0 { up.map(|v| -v) } else { up };
    dot(p, up) / dot(p, d) / (vertical_fov(aspect) / 2.0).tan()
}

/// F30 (UI-GAME-12.1, GFX-VIEW-3.6): H and HOME (`View3d::jump_home`: the
/// start zoom, then the jump to the headquarters' field) put a camera that
/// was moved, zoomed and turned back where the game started it: the start
/// target, zoom 3, no turn.
// spec: UI-GAME-12.1, GFX-VIEW-3.6
#[test]
fn home_puts_the_camera_back_on_its_start() {
    let flat = Grid {
        frame: Frame::new(40, 40),
        height: |_, _| 5000,
    };
    let [x0, z0] = flat.frame.field_xz(20, 20);
    let mut start = CameraRig::start(Some([x0, z0]));
    let mut moved = CameraRig::start(Some([x0, z0]));
    for _ in 0..30 {
        start.update(&flat);
        moved.update(&flat);
    }
    moved.jump_to(x0 + 30.0, z0 + 20.0);
    moved.zoom = 1;
    moved.yaw_target = 1.0;
    for _ in 0..30 {
        moved.update(&flat);
    }
    assert!(!close(moved.target[0], start.target[0]));
    // H: the start zoom, then the jump home.
    moved.zoom = start.zoom;
    moved.jump_to(x0, z0);
    for _ in 0..30 {
        moved.update(&flat);
    }
    assert_eq!((moved.zoom, moved.yaw, moved.yaw_target), (3, 0.0, 0.0));
    for k in 0..3 {
        assert!(
            close(moved.target[k], start.target[k]),
            "{:?} {:?}",
            moved.target,
            start.target
        );
    }
}

// GFX-VIEW T13, T13a (3.6): a new game starts at zoom 3, yaw 0, the
// target 18 units in front of the headquarters' field point; the HQ
// then stands a little above the centre of the screen.
// spec: GFX-VIEW-3.6 (GFX-VIEW T13, T13a)
#[test]
fn start_camera_on_the_headquarters() {
    let flat = Grid {
        frame: Frame::new(40, 40),
        height: |_, _| 5000,
    };
    let [x0, z0] = flat.frame.field_xz(20, 20);
    let mut rig = CameraRig::start(Some([x0, z0]));
    assert_eq!((rig.zoom, rig.yaw, rig.yaw_target), (3, 0.0, 0.0));
    let mut eye = rig.update(&flat);
    for _ in 0..20 {
        eye = rig.update(&flat);
    }
    assert!(close(rig.target[0], x0) && close(rig.target[1], 10.0));
    assert!(close(rig.target[2], z0 - 18.0), "{:?}", rig.target);
    let p = eye.position;
    assert!(close(p[0], x0) && (p[1] - 65.533).abs() < 0.01, "{p:?}");
    assert!((p[2] - (z0 - 47.752)).abs() < 0.01, "{p:?}");
    let hq = [x0, 5.0, z0];
    // Horizontally in the centre.
    let to = [hq[0] - p[0], hq[2] - p[2]];
    assert!(close(to[0], 0.0));
    for (aspect, want) in [(4.0 / 3.0, 0.070), (16.0 / 9.0, 0.093)] {
        let y = screen_height(&eye, hq, aspect);
        assert!((y - want).abs() < 0.002, "{aspect}: {y}");
    }
    // T13a: zoom 2 and 4 after the start, at 16:9.
    for (zoom, want) in [(2, 0.342), (4, -0.050)] {
        rig.zoom = zoom;
        for _ in 0..30 {
            eye = rig.update(&flat);
        }
        let y = screen_height(&eye, hq, 16.0 / 9.0);
        assert!((y - want).abs() < 0.002, "zoom {zoom}: {y}");
    }
}

// GFX-VIEW T13b (3.6 a): without a headquarters the target stays at
// (0, 10, -10), floor-corrected, zoom 3.
// spec: GFX-VIEW-3.6 (GFX-VIEW T13b)
#[test]
fn start_camera_without_headquarters() {
    let flat = Grid {
        frame: Frame::new(40, 40),
        height: |_, _| 0,
    };
    let mut rig = CameraRig::start(None);
    rig.update(&flat);
    assert_eq!(rig.zoom, 3);
    assert!(close(rig.target[0], 0.0) && close(rig.target[2], -10.0));
    assert!(close(rig.target[1], 10.0));
}

// GFX-VIEW-3.3 (corrected, play test 10): after a jump the camera's x
// and y take their new values at once, its z closes 70 % of the gap a
// frame.
#[test]
fn camera_z_eases_after_a_jump() {
    let flat = Grid {
        frame: Frame::new(60, 60),
        height: |_, _| 0,
    };
    let mut rig = CameraRig::start(Some([0.0, 0.0]));
    for _ in 0..20 {
        rig.update(&flat);
    }
    let before = rig.update(&flat).position;
    rig.jump_to(10.0, 30.0);
    let after = rig.update(&flat).position;
    let settled = {
        let mut r = rig.clone();
        let mut e = r.update(&flat);
        for _ in 0..40 {
            e = r.update(&flat);
        }
        e.position
    };
    assert!(close(after[0], settled[0]) && close(after[1], settled[1]));
    let z = settled[2] + 0.3 * (before[2] - settled[2]);
    assert!(close(after[2], z), "{} vs {z}", after[2]);
}

// GFX-VIEW T10: the horizontal field of view of 50 degrees gives 38.6
// degrees vertically at 4:3 and 29.4 at 16:9.
// spec: GFX-VIEW-3.1, GFX-VIEW-3.5 (GFX-VIEW T10)
#[test]
fn vertical_field_of_view() {
    assert!((vertical_fov(1024.0 / 768.0).to_degrees() - 38.58).abs() < 0.05);
    assert!((vertical_fov(1920.0 / 1080.0).to_degrees() - 29.43).abs() < 0.05);
    assert!((1.0 / (FIELD_OF_VIEW / 2.0).to_radians().tan() - 2.1445).abs() < 1e-3);
}

// GFX-VIEW T4: rotate-left twice from 0, easing 24.5 % per frame.
// spec: GFX-VIEW-3.2 (GFX-VIEW T4)
#[test]
fn camera_rotation() {
    let flat = Grid {
        frame: Frame::new(20, 20),
        height: |_, _| 0,
    };
    let mut rig = CameraRig::new(0.0, 0.0);
    rig.rotate_left();
    assert_eq!(rig.yaw_target, 1.5);
    rig.update(&flat);
    assert!(close(rig.yaw, 1.5 * 0.245));
    rig.rotate_left();
    assert_eq!(rig.yaw_target, 1.5);
    rig.rotate_right();
    assert_eq!(rig.yaw_target, 0.0);
    rig.rotate_right();
    assert_eq!(rig.yaw_target, -1.5);
}

/// A camera settled over a flat map, looking at its middle.
fn settled() -> (Grid, CameraRig, Eye) {
    let flat = Grid {
        frame: Frame::new(40, 40),
        height: |_, _| 0,
    };
    let [x0, z0] = flat.frame.field_xz(20, 20);
    let mut rig = CameraRig::start(Some([x0, z0]));
    let mut eye = rig.update(&flat);
    for _ in 0..40 {
        eye = rig.update(&flat);
    }
    (flat, rig, eye)
}

/// The heading of a direction: 0 north (+z), positive to the right (+x).
fn bearing(d: [f32; 3]) -> f32 {
    d[0].atan2(d[2])
}

// GFX-VIEW T4a: with the right button held, the mouse moved a tenth of the
// screen width to the right turns the view by 1 radian in that frame, with
// no easing, the camera where it was; on release the view eases back to
// the previous direction by 0.4 of the difference a frame, the target
// unchanged.
// spec: GFX-VIEW-3.2a (GFX-VIEW T4a), GFX-VIEW-3.3
#[test]
fn right_button_look_turns_at_once_and_eases_back() {
    let (flat, mut rig, before) = settled();
    let target = rig.target;
    rig.look_start();
    assert!(rig.looking());
    rig.look_turn(0.1, 0.0);
    let looked = rig.update(&flat);
    let turned = bearing(looked.direction) - bearing(before.direction);
    assert!(close(turned, 1.0), "{turned}");
    assert!(close(looked.direction[1], before.direction[1]));
    assert_eq!(looked.position, before.position);
    // The next frame without movement: no easing in the look.
    assert_eq!(rig.update(&flat).direction, looked.direction);
    rig.look_end();
    assert!(!rig.looking());
    let back = rig.update(&flat);
    let d = rig.direction();
    let v = looked.direction;
    let want = normalize([0, 1, 2].map(|i| v[i] + (d[i] - v[i]) * 0.4));
    for i in 0..3 {
        assert!(close(back.direction[i], want[i]), "{back:?} {want:?}");
    }
    assert_eq!(rig.target, target);
}

// GFX-VIEW-3.2a a: the mouse's vertical movement adds 6 dy to the view's
// vertical component, clamped to +-0.9, then the direction is normalised.
// spec: GFX-VIEW-3.2a
#[test]
fn right_button_look_pitch_is_clamped() {
    let (flat, mut rig, before) = settled();
    rig.look_start();
    rig.look_turn(0.0, 0.05);
    let up = rig.update(&flat).direction;
    let [x, y, z] = before.direction;
    let want = normalize([x, y + 0.3, z]);
    assert!(close(up[1], want[1]), "{up:?} {want:?}");
    rig.look_turn(0.0, -1.0);
    let down = rig.update(&flat).direction;
    let h = up[0].hypot(up[2]);
    assert!(close(down[1], -0.9 / (0.81f32 + h * h).sqrt()), "{down:?}");
}

// GFX-VIEW T4b: with the right button held, UP for one frame of 0.04 s at
// speed 0.25 moves camera and target 2 x (5 x 0.25)^2 x 0.04 x 25 = 3.125
// units along the horizontal view direction; after the release the target
// stays there. A move whose camera would leave the map is not taken.
// spec: GFX-VIEW-3.2a (GFX-VIEW T4b)
#[test]
fn right_button_look_flies_with_the_arrows() {
    let (flat, mut rig, before) = settled();
    let target = rig.target;
    rig.look_start();
    rig.look_turn(0.05, 0.0);
    let d = rig.update(&flat).direction;
    let step = 2.0 * look_step(0.04, 0.25);
    assert!(close(step, 3.125));
    rig.look_fly(&flat.frame, 0.0, step);
    let flown = rig.update(&flat);
    let h = d[0].hypot(d[2]);
    let (fx, fz) = (d[0] / h * 3.125, d[2] / h * 3.125);
    assert!(close(flown.position[0] - before.position[0], fx));
    assert!(close(flown.position[2] - before.position[2], fz));
    assert!(close(rig.target[0] - target[0], fx) && close(rig.target[2] - target[2], fz));
    rig.look_fly(&flat.frame, 0.0, 10_000.0);
    assert!(close(rig.update(&flat).position[0], flown.position[0]));
    rig.look_end();
    rig.update(&flat);
    assert!(close(rig.target[0] - target[0], fx) && close(rig.target[2] - target[2], fz));
}

// GFX-VIEW T5: zoom stays within 0 to 4.
// spec: GFX-VIEW-3.1, GFX-VIEW-3.2 (GFX-VIEW T5)
#[test]
fn camera_zoom_limits() {
    let mut rig = CameraRig::new(0.0, 0.0);
    rig.zoom = 4;
    rig.zoom_out();
    assert_eq!(rig.zoom, 4);
    rig.zoom_in();
    assert_eq!(rig.zoom, 3);
    rig.zoom = 0;
    rig.zoom_in();
    assert_eq!(rig.zoom, 0);
}

// GFX-VIEW-3.2: a pan that would leave the grid is not taken.
#[test]
fn pan_stays_over_the_map() {
    let frame = Frame::new(20, 20);
    let mut rig = CameraRig::new(0.0, 0.0);
    rig.pan(&frame, 0.0, 10.0);
    assert!(close(rig.target[2], 10.0));
    rig.pan(&frame, 0.0, 1000.0);
    assert!(close(rig.target[2], 10.0));
    // Right of north is east.
    rig.pan(&frame, 5.0, 0.0);
    assert!(close(rig.target[0], 5.0));
}

// GFX-OBJ T9, 2.3: on a slope every decal vertex lies 0.01 above the
// terrain under it, the decal covers its own triangles, and the texture
// lies as the decal's own vertices laid it.
// spec: GFX-OBJ-2.3 (GFX-OBJ T9)
#[test]
fn decal_drapes_over_a_slope() {
    let slope = Grid {
        frame: Frame::new(10, 10),
        height: |c, r| (c * 150 + r * 400) as i32,
    };
    let [x, z] = slope.frame.field_xz(5, 5);
    let corner = |dx: f32, dz: f32| ([x + dx, 9.0, z + dz], [(dx + 2.0) / 4.0, (dz + 1.5) / 3.0]);
    let (a, b, c, d) = (
        corner(-2.0, -1.5),
        corner(2.0, -1.5),
        corner(2.0, 1.5),
        corner(-2.0, 1.5),
    );
    let drape = drape(&slope, &[[a, b, c], [a, c, d]]);
    assert!(!drape.indices.is_empty());
    for (p, uv) in drape.positions.iter().zip(&drape.uvs) {
        assert!((p[1] - ground_height(&slope, p[0], p[2]) - DECAL_LIFT).abs() < 1e-3);
        assert!(p[0] >= x - 2.0 - 1e-4 && p[0] <= x + 2.0 + 1e-4);
        assert!(p[2] >= z - 1.5 - 1e-4 && p[2] <= z + 1.5 + 1e-4);
        assert!(close(uv[0], (p[0] - x + 2.0) / 4.0));
        assert!(close(uv[1], (p[2] - z + 1.5) / 3.0));
    }
    // The triangles' area in x, z is the decal's: no gap.
    let area: f32 = drape
        .indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| drape.positions[t[k] as usize]);
            ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs() / 2.0
        })
        .sum();
    assert!((area - 12.0).abs() < 1e-2, "{area}");
}

// spec: GFX-VIEW-1.5
#[test]
fn ray_meets_the_ground() {
    let flat = Grid {
        frame: Frame::new(20, 20),
        height: |_, _| 2000,
    };
    let hit = ray_ground(&flat, [0.0, 30.0, 0.0], normalize([0.0, -1.0, 1.0]), 200.0)
        .expect("the ray hits");
    assert!(close(hit[1], 2.0) && close(hit[2], 28.0));
}

fn entry(hour: i32, elevation: i32, heading: i32, skydome: i32) -> s2_assets::daytimes::DayEntry {
    s2_assets::daytimes::DayEntry {
        hour,
        background: [0.8, 0.9, 1.0, 1.0],
        skydome,
        elevation,
        heading,
        ambient: [0.6, 0.5, 0.6, 1.0],
        light: [0.9375, 0.898, 0.781, 1.0],
        shadow_intensity: 0.9,
        fog_near: 100.0,
        fog_far: 200.0,
    }
}

// GFX-VIEW-2.3, 2.3b: the light of a table at an hour: its own entry at
// that hour; between two entries linear, the heading the shorter way, across
// midnight from the last entry to the first; the cap shows the first one's
// skydome. The shipped default at the start hour 12: elevation 50, heading
// 50 (not daytimes.xml's 55).
// spec: GFX-VIEW-2.3, GFX-VIEW-2.3b, GFX-VIEW-6.3a
#[test]
fn the_day_table_at_an_hour() {
    let table = [
        entry(0, -28, 3, 1),
        entry(3, 50, 50, 0),
        entry(12, 50, 50, 0),
        entry(20, 50, 50, 0),
        entry(22, 15, 24, 0),
        entry(23, -28, 3, 1),
    ];
    let noon = DayLight::at(&table, START_HOUR).unwrap();
    assert_eq!(
        (noon.elevation, noon.heading, noon.skydome),
        (50.0, 50.0, 0)
    );
    let late = DayLight::at(&table, 22.5).unwrap();
    assert_eq!((late.elevation, late.heading), (-6.5, 13.5));
    let midnight = DayLight::at(&table, 23.5).unwrap();
    assert_eq!(
        (midnight.elevation, midnight.heading, midnight.skydome),
        (-28.0, 3.0, 1)
    );
    // Across 360 degrees the shorter way: 350 to 10 passes 0.
    let wrap = [entry(0, 10, 350, 0), entry(12, 10, 10, 0)];
    assert_eq!(DayLight::at(&wrap, 6.0).unwrap().heading, 360.0);
    assert!(DayLight::at(&[], 12.0).is_none());
    // A night table (the same entry at every hour): the moon shows.
    let night = [entry(0, -36, 47, 1), entry(12, -36, 47, 1)];
    let light = DayLight::at(&night, START_HOUR).unwrap();
    assert!(light.moon_shown() && light.skydome == 1);
    assert!(!noon.moon_shown());
}

// GFX-VIEW-2.3c (T28): the environment clock advances by the frame's game
// time x 0.033 hours (float32, from 12; with 0.025 s frames 15.3005 at trace
// tick 1001), the table is read at the clock mod 24, the skydome is the
// earlier entry's; with the DayNightCycle option off the clock stays at 12.
// spec: GFX-VIEW-2.3c (T28)
#[test]
fn t28_the_day_clock_runs_with_the_game_time() {
    let mut table = Vec::new();
    for (hour, elevation, heading) in [
        (0, -28, 3),
        (1, 20, 20),
        (2, 40, 40),
        (3, 50, 50),
        (12, 50, 50),
        (20, 50, 50),
        (21, 40, 40),
        (22, 15, 24),
        (23, -28, 3),
    ] {
        let night = i32::from(hour == 23 || hour == 0);
        table.push(entry(hour, elevation, heading, night));
    }
    let mut hour = START_HOUR;
    for _ in 0..4000 {
        hour = advance_clock(hour, 0.025, true, START_HOUR);
    }
    assert!((hour - 15.3005).abs() < 0.003, "{hour}");
    assert_eq!(advance_clock(hour, 0.025, false, START_HOUR), START_HOUR);
    let at = |clock: f32| DayLight::at_clock(&table, clock).unwrap();
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    let evening = at(21.9084);
    assert!(
        near(evening.elevation, 17.29) && near(evening.heading, 25.46),
        "{evening:?}"
    );
    assert_eq!(at(23.0019).skydome, 1);
    let night = at(23.8906);
    assert_eq!((night.elevation, night.heading), (-28.0, 3.0));
    assert_eq!(at(25.0014).skydome, 0);
    assert!(near(at(25.8735).elevation, 37.47));
    let noon = at(31.8141);
    assert_eq!(
        (noon.elevation, noon.heading, noon.skydome),
        (50.0, 50.0, 0)
    );
    assert!(!noon.differs(&at(START_HOUR)) && evening.differs(&noon));
}

/// GFX-VIEW-6.3 (T44): the cap's colour is lerp(T0, T1, b): T0 the
/// skydome of the entry the clock is in, T1 the next entry's, mixed by the
/// day-time blend; the next entry's share is 0 while both name the same
/// skydome. The default table: clouds alone at 12, half and half at 22.5,
/// 0.725 stars + 0.275 clouds at 0.275, stars alone at 23.5.
// spec: GFX-VIEW-6.3 (T44)
#[test]
fn t44_the_sky_mixes_two_skydomes() {
    let table: Vec<_> = [(0, 1), (1, 0), (3, 0), (12, 0), (20, 0), (22, 0), (23, 1)]
        .into_iter()
        .map(|(hour, sky)| entry(hour, 50, 50, sky))
        .collect();
    let sky = |hour: f32| {
        let l = DayLight::at(&table, hour).unwrap();
        (l.skydome, l.next_skydome, l.sky_mix())
    };
    let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
    let (first, _, mix) = sky(12.0);
    assert!(first == 0 && mix == 0.0);
    let (first, next, mix) = sky(22.5);
    assert!(first == 0 && next == 1 && near(mix, 0.5));
    let (first, next, mix) = sky(0.275);
    assert!(first == 1 && next == 0 && near(mix, 0.275));
    let (first, _, mix) = sky(23.5);
    assert!(first == 1 && mix == 0.0);
    // The mix moving is a new light to draw.
    let (a, b) = (
        DayLight::at(&table, 22.4).unwrap(),
        DayLight::at(&table, 22.45).unwrap(),
    );
    assert!(a.differs(&b));
}

// GFX-VIEW-2.3b: without a binary table the light of daytimes.xml.
// spec: GFX-VIEW-2.3b
#[test]
fn without_a_table_the_xml_light() {
    let light = crate::render::view3d::day_light(None, "MP_2P_Storm_Coast");
    assert_eq!(light, DayLight::XML);
    assert_eq!(light.heading, 55.0);
}

// GFX-VIEW T30, T31 (2.3d): the shipped table at hour 0.28 (the Weak game
// near trace tick 11 000) and at hour 0 exactly. Every value blended, the
// direction from |elevation| (the light from above, not below), the shadow
// factor 1 below |e| 20 and 0.8 at |e| 28 with intensity 0.5.
// spec: GFX-VIEW-2.3d (GFX-VIEW T30, T31)
#[test]
fn t30_t31_the_night_light_comes_from_above() {
    let Ok(root) = std::env::var("S2_GAME_DIR") else {
        return;
    };
    let table =
        crate::render::view3d::day_table(Some(std::path::Path::new(&root)), "MP_2P_Storm_Coast");
    let near = |a: f32, b: f32, e: f32| (a - b).abs() < e;
    let light = DayLight::at_clock(&table, 48.28).unwrap();
    assert!(near(light.elevation, -14.56, 0.01), "{light:?}");
    assert!(near(light.heading, 7.76, 0.01), "{light:?}");
    assert!(near(light.shadow_intensity, 0.612, 0.001), "{light:?}");
    let d = light.direction();
    for (got, want) in d.iter().zip([0.1307, -0.2514, 0.9590]) {
        assert!(near(*got, want, 1e-3), "{d:?}");
    }
    let s = shadow_factor(light.elevation, light.shadow_intensity);
    assert_eq!(s, 1.0);
    let flat = light.light_d(-d[1], 1.0);
    for (got, want) in flat.iter().zip([0.2983, 0.3964, 0.6361]) {
        assert!(near(*got, want, 2e-3), "{flat:?}");
    }
    assert_eq!(light.light_d(-d[1], s), flat);
    // The shaders get the same direction (Bevy's frame).
    let shader = light.shader();
    assert!(shader.direction[1] < 0.0 && shader.direction[3] == 1.0);
    // T31: hour 0 exactly.
    let midnight = DayLight::at_clock(&table, 24.0).unwrap();
    let s = shadow_factor(midnight.elevation, midnight.shadow_intensity);
    assert!(near(s, 0.8, 1e-4), "{s}");
    let n_dot_l = -midnight.direction()[1];
    assert!(near(n_dot_l, 0.4695, 1e-4), "{n_dot_l}");
    let lit = midnight.light_d(n_dot_l, 1.0);
    let dark = midnight.light_d(n_dot_l, s);
    for (got, want) in lit.iter().zip([0.3135, 0.4558, 0.8699]) {
        assert!(near(*got, want, 2e-3), "{lit:?}");
    }
    for (got, want) in dark.iter().zip([0.2947, 0.4344, 0.8135]) {
        assert!(near(*got, want, 2e-3), "{dark:?}");
    }
}

// GFX-VIEW T32 (2.3d f, GFX-OBJ-2.4): a tree vertex facing up at hour 12,
// texel (40, 80, 30): with vertex colour 1 lit (53, 95, 36), in a shadow
// (27, 46, 20); with 0.25 lit (31, 54, 22), in a shadow (25, 41, 18). The
// whole D times 0.25 would give (13, 24, 9) lit.
// spec: GFX-OBJ-2.4, GFX-VIEW-2.3d (GFX-VIEW T32)
#[test]
fn t32_a_tree_vertex_colour_scales_the_direct_light_only() {
    let light = DayLight::XML;
    let n_dot_l = 0.766;
    let s = shadow_factor(light.elevation, light.shadow_intensity);
    let pixel = |c: f32, shadow: f32| {
        let d = light.tree_d(n_dot_l, [c; 3], shadow);
        [40.0, 80.0, 30.0]
            .iter()
            .zip(d)
            .map(|(t, d)| (t * d).round() as u8)
            .collect::<Vec<_>>()
    };
    assert_eq!(pixel(1.0, 1.0), [53, 95, 36]);
    assert_eq!(pixel(1.0, s), [27, 46, 20]);
    assert_eq!(pixel(0.25, 1.0), [31, 54, 22]);
    assert_eq!(pixel(0.25, s), [25, 41, 18]);
}

// GFX-VIEW T33 (2.4a): z_pavement_00 (intensity 0.6, power 5), a normal-map
// texel straight up with alpha 1, flat ground lit at hour 12, N = L = (0, 0,
// 1) in tangent space: R = (0, 0, 1); seen from straight above S = 1 and
// +0.6 on every channel, from V = (0, 0.6, 0.8) S = 0.8^5 and +0.1966. T34:
// a type-3 pattern of intensity 0 adds nothing.
// spec: GFX-VIEW-2.4a (GFX-VIEW T33, T34)
#[test]
fn t33_t34_the_type_3_highlight() {
    let d = DayLight::XML.light_d(1.0, 1.0);
    let up = [0.0, 0.0, 1.0];
    let pavement = [0.6, 5.0];
    assert!((specular_add(d, up, up, up, 1.0, pavement) - 0.6).abs() < 1e-5);
    let side = specular_add(d, up, up, [0.0, 0.6, 0.8], 1.0, pavement);
    assert!((side - 0.1966).abs() < 1e-4, "{side}");
    assert!((specular_add(d, up, up, up, 0.5, pavement) - 0.3).abs() < 1e-5);
    assert_eq!(specular_add(d, up, up, up, 1.0, [0.0, 1.0]), 0.0);
    // Per channel as written: a tilted normal gives R off the mirror ray.
    let n = [0.6, 0.0, 0.8];
    let r = specular_add([1.0; 3], n, up, up, 1.0, [1.0, 1.0]);
    let want = 0.6 / (1.2f32 * 1.2 + 0.6 * 0.6).sqrt();
    assert!((r - want).abs() < 1e-5, "{r} {want}");
}

// GFX-VIEW-3.6 e (CAM T14): a scripted move to a field ends at zoom 3 where
// the jump of 3.6 d puts the camera; the eye goes 12.5 % of the way each
// frame with the view direction kept, so from 50 units away it takes about
// 47 frames; then the normal camera goes on from there.
// spec: GFX-VIEW-3.6, CAM-5.3b (T14)
#[test]
fn a_scripted_move_eases_to_the_jump_at_zoom_3() {
    let flat = Grid {
        frame: Frame::new(80, 80),
        height: |_, _| 0,
    };
    let mut rig = CameraRig::start(Some([0.0, 0.0]));
    rig.zoom = 1;
    for _ in 0..40 {
        rig.update(&flat);
    }
    let start = rig.update(&flat);
    // Where the jump puts the camera at zoom 3.
    let wanted = {
        let mut r = rig.clone();
        r.zoom = 3;
        r.jump_to(30.0, 40.0);
        let mut e = r.update(&flat);
        for _ in 0..60 {
            e = r.update(&flat);
        }
        e.position
    };
    rig.move_to(30.0, 40.0);
    assert!(rig.moving());
    let gap = |p: [f32; 3]| {
        (0..3)
            .map(|i| (wanted[i] - p[i]).powi(2))
            .sum::<f32>()
            .sqrt()
    };
    let first = rig.update(&flat);
    assert_eq!(rig.zoom, 3);
    assert!(close(gap(first.position), 0.875 * gap(start.position)));
    assert_eq!(first.direction, start.direction, "the view is kept");
    let mut frames = 1;
    while rig.moving() {
        rig.update(&flat);
        frames += 1;
        assert!(frames < 200);
    }
    let expected = (0.1 / gap(start.position)).ln() / 0.875f32.ln();
    assert!(
        (frames as f32 - expected).abs() <= 1.0,
        "{frames} vs {expected}"
    );
    // About 50 units: about 47 frames.
    let mut far = CameraRig::start(Some([0.0, 0.0]));
    for _ in 0..40 {
        far.update(&flat);
    }
    far.update(&flat);
    far.move_to(0.0, 50.0);
    let mut n = 0;
    while far.moving() {
        far.update(&flat);
        n += 1;
    }
    assert!((45..=48).contains(&n), "{n}");
    // The normal camera goes on from the jump's place.
    let after = rig.update(&flat).position;
    assert!(gap(after) < 0.2, "{}", gap(after));
}

// GFX-VIEW-3.7 (T37): the follow camera: behind 4 back, 1 to the side, 2 up,
// looking at his head height; ahead 5 on, looking at his face; the eye and
// the view direction each go 5 % of the way a frame; the eye stays at least
// max(ground + 1, 3) outside the building pose; the zoom plays no part.
// spec: GFX-VIEW-3.7
#[test]
fn the_follow_camera_trails_him() {
    let flat = Grid {
        frame: Frame::new(60, 60),
        height: |_, _| 0,
    };
    let p = [10.0, 0.0, 20.0];
    let (f, s) = ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let (eye, look) = follow_target(FollowPose::Behind, p, f, s, None);
    assert_eq!(eye, [11.0, 2.0, 16.0]);
    assert_eq!(look, [10.0, 2.0, 20.0]);
    let (eye, look) = follow_target(FollowPose::Ahead, p, f, s, None);
    assert_eq!(eye, [11.0, 2.0, 25.0]);
    assert_eq!(look, [10.0, 1.25, 20.0]);
    // 5 % of the way a frame; settled at the wanted eye.
    let (wanted, look) = follow_target(FollowPose::Behind, p, f, s, None);
    let start = Eye {
        position: [11.0, 22.0, 16.0],
        direction: [0.0, -1.0, 0.0],
    };
    let next = follow_frame(start, wanted, look, false, &flat);
    assert!(close(next.position[1], 22.0 - 0.05 * 20.0));
    let mut e = next;
    for _ in 0..400 {
        e = follow_frame(e, wanted, look, false, &flat);
    }
    // Settled at the wanted eye, raised to the floor 3 (the water level + 3;
    // the wanted 2 above level ground at 0 is below it).
    assert!(close(e.position[0], wanted[0]) && close(e.position[2], wanted[2]));
    assert!(close(e.position[1], 3.0), "{:?}", e.position);
    // The floor: never below 3 over level ground at height 0.
    let low = Eye {
        position: [11.0, 0.0, 16.0],
        direction: [0.0, 0.0, 1.0],
    };
    assert!(close(
        follow_frame(low, [11.0, 0.0, 16.0], look, false, &flat).position[1],
        3.0
    ));
    // The building pose has none.
    let (eye, look) = follow_target(FollowPose::Behind, p, f, s, Some([10.0, 0.0, 22.0]));
    assert!(close(look[1], 1.1));
    assert!(follow_frame(low, eye, look, true, &flat).position[1] < 3.0);
}

/// GFX-WATER-2.6, T16: the mirror texture is 128 x 2^trunc(4 q) wide,
/// halved until it fits the back buffer, half as high: 2048 x 1024 on a
/// 2560-wide back buffer at quality 1.0, 1024 x 512 on a 1024-wide one;
/// none at quality below 0.25.
#[test]
fn mirror_texture_size_from_quality_and_back_buffer() {
    assert_eq!(mirror_texture_size(1.0, 2560), Some((2048, 1024)));
    assert_eq!(mirror_texture_size(1.0, 1024), Some((1024, 512)));
    assert_eq!(mirror_texture_size(1.0, 1000), Some((512, 256)));
    assert_eq!(mirror_texture_size(0.5, 2560), Some((512, 256)));
    assert_eq!(mirror_texture_size(0.74, 2560), Some((512, 256)));
    assert_eq!(mirror_texture_size(0.25, 2560), Some((256, 128)));
    assert_eq!(mirror_texture_size(2.0, 4096), Some((2048, 1024)));
    assert_eq!(mirror_texture_size(0.24, 2560), None);
    assert_eq!(mirror_texture_size(0.0, 2560), None);
    assert_eq!(mirror_texture_size(f32::NAN, 2560), None);
}

/// GFX-WATER-2.6a 2: the mirror's field of view is 1.03 x the main
/// camera's horizontal one at the same aspect: at the main 50 degrees
/// horizontal and 4 / 3 the mirror's is 51.5 degrees horizontal.
// spec: GFX-WATER-2.6a
#[test]
fn mirror_field_of_view_is_wider_by_three_percent() {
    let aspect = 4.0 / 3.0;
    let v = mirror_vertical_fov(vertical_fov(aspect), aspect);
    let horizontal = 2.0 * ((v / 2.0).tan() * aspect).atan();
    assert!(
        (horizontal.to_degrees() - 51.5).abs() < 1e-3,
        "{}",
        horizontal.to_degrees()
    );
    assert!(v > vertical_fov(aspect));
    // 2.6a 1: reflected in the surface's plane at -0.1 (3.3, 2026-11-09).
    assert_eq!(
        reflect_in_water([1.0, 5.0, -2.0]),
        [1.0, 2.0 * WATER_SURFACE - 5.0, -2.0]
    );
    assert!((reflect_in_water([0.0, 5.0, 0.0])[1] + 5.2).abs() < 1e-6);
}

/// GFX-WATER-2.6b, T16a: the sky alone from a camera looking down more
/// than 30 degrees (the default 45), whatever the details; flatter views
/// by n = round(4 x details): 0 sky, 1 and 2 sky and land, 3 also the
/// settlers, 4 everything.
// spec: GFX-WATER-2.6b
#[test]
fn mirror_content_by_pitch_and_details() {
    let sky = MirrorContent {
        land: false,
        settlers: false,
        objects: false,
    };
    let land = MirrorContent {
        land: true,
        settlers: false,
        objects: false,
    };
    let settlers = MirrorContent {
        land: true,
        settlers: true,
        objects: false,
    };
    let all = MirrorContent {
        land: true,
        settlers: true,
        objects: true,
    };
    let down45 = -(45f32.to_radians().sin());
    assert_eq!(mirror_content(down45, 1.0), sky);
    assert_eq!(mirror_content(-0.51, 1.0), sky);
    assert_eq!(mirror_content(-0.5, 1.0), all);
    assert_eq!(mirror_content(-0.3, 0.1), sky);
    assert_eq!(mirror_content(-0.3, 0.125), land);
    assert_eq!(mirror_content(-0.3, 0.624), land);
    // n = 3: the settlers too (Q-1028).
    assert_eq!(mirror_content(-0.3, 0.625), settlers);
    assert_eq!(mirror_content(-0.3, 0.874), settlers);
    assert_eq!(mirror_content(-0.3, 0.875), all);
}

/// GFX-OBJ-3.3 (T41): a settler pixel on flat ground at noon seen from
/// straight above: D 1.2280, R = normalize(2 D N + L) = (0.2723, 0.9347,
/// 0.2285), S 0.339 (a mirror reflection of the light would give 0.014); in
/// a shadow D 0.6100, R (0.6257, 0.5769, 0.5251), S 0.0002. The shader
/// takes the same terms.
// spec: GFX-OBJ-3.3 (T41)
#[test]
fn t41_the_settler_highlight() {
    let (n, l, v) = ([0.0, 1.0, 0.0], [0.4924, -0.7660, 0.4132], [0.0, 1.0, 0.0]);
    let near = |a: f32, b: f32, e: f32| (a - b).abs() < e;
    let d = 0.7660 * 0.8965 + 0.5413;
    assert!(near(d, 1.2280, 1e-3));
    let (r, s) = settler_highlight(n, l, v, d);
    assert!(near(r[0], 0.2723, 1e-3) && near(r[1], 0.9347, 1e-3) && near(r[2], 0.2285, 1e-3));
    assert!(near(s, 0.339, 2e-3), "{s}");
    let shade = 0.7660 * 0.8965 * 0.1 + 0.5413;
    let (r, s) = settler_highlight(n, l, v, shade);
    assert!(near(shade, 0.6100, 1e-3));
    assert!(near(r[0], 0.6257, 1e-3) && near(r[1], 0.5769, 1e-3) && near(r[2], 0.5251, 1e-3));
    assert!(s < 0.0003);
    let shader = include_str!("../shaders/object.wesl");
    assert!(shader.contains("let r = normalize(2.0 * d * n + light.light_dir.xyz);"));
    assert!(shader.contains("let d = saturate(dot(raw_n, -light.light_dir.xyz))"));
}

/// A flat map of `w` x `h` fields at height 0.
fn flat(w: u32, h: u32) -> Grid {
    Grid {
        frame: Frame::new(w, h),
        height: |_, _| 0,
    }
}

/// The point above sector `(sx, sy)`'s middle field, 10 up.
fn above_sector(g: &Grid, sx: u32, sy: u32) -> [f32; 3] {
    let [x, z] = g.frame().field_xz(4 * sx + 2, 4 * sy + 2);
    [x, 10.0, z]
}

// GFX-VIEW-1.5 (Q-1067) test (a): a 50 x 38 map has 13 x 10 sectors;
// sector (0, 0) covers fields x 0..4, y 0..4; (11, 0) x 44..48; (12, 9)
// x 48..49, y 36..37 (neighbours share a column and a row).
// spec: GFX-VIEW-1.5
#[test]
fn sectors_and_their_field_ranges() {
    use sectors::{sector_count, sector_fields};
    assert_eq!(sector_count(50, 38), (13, 10));
    assert_eq!(sector_fields(0, 0, 50, 38), ([0, 4], [0, 4]));
    assert_eq!(sector_fields(11, 0, 50, 38).0, [44, 48]);
    assert_eq!(sector_fields(12, 9, 50, 38), ([48, 49], [36, 37]));
    assert_eq!(sector_fields(1, 0, 50, 38).0, [4, 8]);
}

// GFX-VIEW-1.5 (Q-1067) test (b): a 2 x 2-sector map (8 x 8 fields), flat,
// the root across a plane, the four sectors in view, P above sector (1, 0):
// the list (1, 0), (0, 0), (1, 1), (0, 1). A sector outside a plane, or too
// far, is left out.
// spec: GFX-VIEW-1.5
#[test]
fn the_list_starts_at_the_nearest_child() {
    use sectors::{Bounds, SectorTree, Seen};
    let g = flat(8, 8);
    let mut tree = SectorTree::new(&g);
    let p = above_sector(&g, 1, 0);
    // Only the root (the whole map's box) is across a plane.
    let whole = |b: &Bounds| {
        // The root is 45 wide (fields 0..7 and the odd rows' half field),
        // a leaf 27 at most.
        if b.max[0] - b.min[0] > 35.0 {
            Seen::Straddles
        } else {
            Seen::Inside
        }
    };
    assert_eq!(
        tree.walk(p, 500.0, &whole),
        [(1, 0), (0, 0), (1, 1), (0, 1)]
    );
    // P above (0, 1): nearest 2, so 2, 0, 3, 1.
    assert_eq!(
        tree.walk(above_sector(&g, 0, 1), 500.0, &whole),
        [(0, 1), (0, 0), (1, 1), (1, 0)]
    );
    // P above (1, 1): 3, 1, 2, 0.
    assert_eq!(
        tree.walk(above_sector(&g, 1, 1), 500.0, &whole),
        [(1, 1), (1, 0), (0, 1), (0, 0)]
    );
    // Sector (0, 0) wholly outside a plane: left out.
    let (x00, z00) = (g.frame().field_xz(2, 2)[0], g.frame().field_xz(2, 2)[1]);
    let left_out = |b: &Bounds| {
        let small = b.max[0] - b.min[0] < 35.0;
        if small && b.min[0] <= x00 && b.max[0] >= x00 && b.min[2] <= z00 && b.max[2] >= z00 {
            Seen::Outside
        } else {
            whole(b)
        }
    };
    assert_eq!(tree.walk(p, 500.0, &left_out), [(1, 0), (1, 1), (0, 1)]);
    // Too far: P outside every box, the far distance 0.
    let high = [p[0], 1000.0, p[2]];
    assert_eq!(tree.walk(high, 0.0, &whole), []);
}

// GFX-VIEW-1.5 (Q-1067) test (c): a 3 x 1-sector map, flat, a new tree, all
// wholly in view, P above sector 2: the root appends child 1 (sector 2,
// nearest) first, then child 0's subtree with its kept choice child 0: 2,
// 0, 1. After a frame in which the root was across a plane (child 0 then
// chose sector 1), the same view gives 2, 1, 0.
// spec: GFX-VIEW-1.5
#[test]
fn a_wholly_seen_subtree_keeps_its_last_choice() {
    use sectors::{Bounds, SectorTree, Seen};
    let g = flat(12, 4);
    let mut tree = SectorTree::new(&g);
    let p = above_sector(&g, 2, 0);
    let all = |_: &Bounds| Seen::Inside;
    assert_eq!(tree.walk(p, 500.0, &all), [(2, 0), (0, 0), (1, 0)]);
    // The root across a plane, its children in view.
    let root_across = |b: &Bounds| {
        if b.max[0] - b.min[0] > 10.0 * 3.0 {
            Seen::Straddles
        } else {
            Seen::Inside
        }
    };
    assert_eq!(tree.walk(p, 500.0, &root_across), [(2, 0), (1, 0), (0, 0)]);
    assert_eq!(tree.walk(p, 500.0, &all), [(2, 0), (1, 0), (0, 0)]);
}

// GFX-VIEW-1.5 (Q-202 a): the pick over the field points. A ray straight
// down onto a field point picks that field; onto a point inside an even
// row's first triangle (x, y) (x + 1, y) (x, y + 1) near (x + 1, y), that
// field; the triangles of an odd row run the other way. A ray that meets
// the surface in two sectors picks in the list's first sector, not the
// nearest hit. Nothing is picked off the map or with no sector listed.
// spec: GFX-VIEW-1.5
#[test]
fn the_pick_takes_the_first_hit_in_the_list() {
    use sectors::SectorTree;
    let g = flat(12, 12);
    let tree = SectorTree::new(&g);
    let (nx, ny) = sectors::sector_count(12, 12);
    let all: Vec<(u32, u32)> = (0..ny).flat_map(|y| (0..nx).map(move |x| (x, y))).collect();
    let down = [0.0, -1.0, 0.0];
    let at = |x: f32, z: f32| [x, 50.0, z];
    let [fx, fz] = g.frame().field_xz(5, 6);
    assert_eq!(tree.pick(&g, &all, at(fx, fz), down), Some((5, 6)));
    // Even row 6, cell (5, 6): a point a quarter of the way from (6, 6)
    // toward (5, 6), slightly toward (5, 7): (6, 6).
    let [ax, az] = g.frame().field_xz(5, 6);
    let [bx, bz] = g.frame().field_xz(6, 6);
    let [cx, cz] = g.frame().field_xz(5, 7);
    let p = [
        0.75 * bx + 0.2 * ax + 0.05 * cx,
        0.75 * bz + 0.2 * az + 0.05 * cz,
    ];
    assert_eq!(tree.pick(&g, &all, at(p[0], p[1]), down), Some((6, 6)));
    // Odd row 7: its cell (5, 7) has (5, 7) (6, 8) (5, 8): the point near
    // (6, 8) on that triangle picks (6, 8).
    let [ax, az] = g.frame().field_xz(5, 7);
    let [bx, bz] = g.frame().field_xz(6, 8);
    let [cx, cz] = g.frame().field_xz(5, 8);
    let p = [
        0.1 * ax + 0.8 * bx + 0.1 * cx,
        0.1 * az + 0.8 * bz + 0.1 * cz,
    ];
    assert_eq!(tree.pick(&g, &all, at(p[0], p[1]), down), Some((6, 8)));
    // Off the map, and no list.
    assert_eq!(tree.pick(&g, &all, at(-500.0, 0.0), down), None);
    assert_eq!(tree.pick(&g, &[], at(fx, fz), down), None);
    // A ridge along field column 6 (15 high) on flat ground: a ray from 20
    // above field (2, 6) down to the ground at field (10, 6), between rows 6
    // and 7, meets the ridge in sector (1, 1) and the ground behind it in
    // sector (2, 1). The list's order decides: sector (2, 1) listed first
    // gives the far field, (1, 1) first the ridge's; sector (0, 1) has no
    // hit (the ray stays above it).
    let ridge = Grid {
        frame: Frame::new(12, 12),
        height: |c, _| if (23..=27).contains(&c) { 15_000 } else { 0 },
    };
    let tree = SectorTree::new(&ridge);
    let [x2, z6] = ridge.frame().field_xz(2, 6);
    let [x10, _] = ridge.frame().field_xz(10, 6);
    let [_, z7] = ridge.frame().field_xz(2, 7);
    let z = z6 + 0.3 * (z7 - z6);
    let origin = [x2, 20.0, z];
    let dir = normalize([x10 - x2, -20.0, 0.0]);
    let far_first = tree.pick(&ridge, &[(0, 1), (2, 1), (1, 1)], origin, dir);
    let near_first = tree.pick(&ridge, &[(0, 1), (1, 1), (2, 1)], origin, dir);
    assert!(
        far_first.is_some_and(|(x, y)| (9..=11).contains(&x) && (6..=7).contains(&y)),
        "{far_first:?}"
    );
    assert!(
        near_first.is_some_and(|(x, y)| (5..=6).contains(&x) && (6..=7).contains(&y)),
        "{near_first:?}"
    );
    assert_eq!(tree.pick(&ridge, &[(0, 1)], origin, dir), None);
}

// GFX-VIEW-1.5 (Q-1069): a sector tests every cell whose top-left point lies
// in its inclusive range, so the cell starting on the column it shares with
// the next sector is found in its own turn; a cell on the map's last column
// reads its far corners past the map (an even row's x = width is sample
// column 4w), and a hit nearest such a corner picks nothing.
// spec: GFX-VIEW-1.5
#[test]
fn a_sector_tests_the_cells_of_its_shared_column() {
    use sectors::SectorTree;
    let g = flat(12, 12);
    let tree = SectorTree::new(&g);
    let down = [0.0, -1.0, 0.0];
    // Inside cell (4, 6)'s first triangle, near (4, 6): sector (0, 1)
    // alone finds it.
    let [ax, az] = g.frame().field_xz(4, 6);
    let [bx, bz] = g.frame().field_xz(5, 6);
    let [cx, cz] = g.frame().field_xz(4, 7);
    let p = [
        0.8 * ax + 0.1 * bx + 0.1 * cx,
        0.8 * az + 0.1 * bz + 0.1 * cz,
    ];
    let at = [p[0], 50.0, p[1]];
    assert_eq!(tree.pick(&g, &[(0, 1)], at, down), Some((4, 6)));
    assert_eq!(tree.pick(&g, &[(1, 1)], at, down), Some((4, 6)));
    // The last column: field (11, 6) and the point past it, sample column 48.
    assert_eq!(
        sectors::field_point(&g, 12, 6)[0],
        g.frame().sample_xz(48, 24)[0]
    );
    // An odd row's point past the map runs on into the next sample row.
    assert_eq!(sectors::field_point(&g, 12, 7), {
        let [x, z] = g.frame().sample_xz(1, 29);
        [x, 0.0, z]
    });
    let [ex, ez] = g.frame().field_xz(11, 6);
    let near_edge = [ex + 1.0, 50.0, ez + 0.5];
    let past = [ex + 5.0, 50.0, ez + 0.5];
    assert_eq!(tree.pick(&g, &[(2, 1)], near_edge, down), Some((11, 6)));
    assert_eq!(tree.pick(&g, &[(2, 1)], past, down), None);
}
