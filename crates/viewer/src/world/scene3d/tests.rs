use super::*;
use s2_sim::map::Terrain;
use std::collections::HashSet;

fn flat_world(width: u32, height: u32) -> World {
    let n = (width * height) as usize;
    let terrain = Terrain {
        width,
        height,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![0; n],
        samples: Vec::new(),
    };
    World::with_map(1, terrain, s2_sim::Rules::empty(), s2_sim::PlayerId(0))
}

/// UI-GAME-13.1: the field icon by build class; class 0 shows the
/// flag on a flag's field, else the crossed building.
#[test]
fn field_icons() {
    let mut world = flat_world(10, 10);
    let player = s2_sim::PlayerId(0);
    // Without building types every free field takes a flag only.
    assert_eq!(world.placement_at((5, 5)), s2_sim::Placement::Flag);
    assert_eq!(field_icon(&world, player, (5, 5)), [448, 0, 46, 32]);
    world.submit(s2_sim::Command::PlaceFlag { player, at: (2, 2) });
    world.step();
    assert_eq!(world.placement_at((2, 2)), s2_sim::Placement::Nothing);
    assert_eq!(field_icon(&world, player, (2, 2)), [448, 0, 46, 32]);
    // Next to the flag nothing fits.
    let next = world.terrain().neighbour((2, 2), 0).unwrap();
    assert_eq!(world.placement_at(next), s2_sim::Placement::Nothing);
    assert_eq!(field_icon(&world, player, next), [448, 66, 46, 32]);
}

// GFX-ANIM-4.3: two carriers meeting on a road both step aside, half
// each, to their own right; walking the same way they do not; the
// offset decays 0.75 a second.
#[test]
fn carriers_evade() {
    let walker = |x: f32, heading: f32| Evader {
        at: [x, 0.0],
        heading,
        moving: true,
        offset: 0.0,
    };
    let pi = std::f32::consts::PI;
    let meeting = [walker(0.0, pi / 2.0), walker(1.2, -pi / 2.0)];
    let o = evade(&meeting, 2.0, 0.5, 1.2, 0.0);
    let full = (1.0 - 1.44 / (2.4 * 2.4)) * 1.2 * 0.5;
    assert!((o[0] - full / 2.0).abs() < 1e-5 && (o[1] - full / 2.0).abs() < 1e-5);
    // Each moves to its own right: opposite world sides.
    let (ra, rb) = (right_of(pi / 2.0), right_of(-pi / 2.0));
    assert!((ra[1] * o[0] + rb[1] * o[1]).abs() < 1e-5);
    let same = [walker(0.0, pi / 2.0), walker(1.2, pi / 2.0)];
    assert_eq!(evade(&same, 2.0, 0.5, 1.2, 0.0), vec![0.0, 0.0]);
    let mut gone = [walker(0.0, 0.0), walker(50.0, pi)];
    gone[0].offset = 1.0;
    let o = evade(&gone, 2.0, 0.5, 1.2, 0.4);
    assert!((o[0] - 0.7).abs() < 1e-5 && o[1] == 0.0);
}

// GFX-ANIM-4.3 (Q-207 g): a walker passing a stander takes the full offset
// whatever the directions, signed by his own lean; the stander none. Two
// walkers toward each other take half each with one sign, from the sum of
// their leans; the offsets add to what each has.
// spec: GFX-ANIM-4.3
#[test]
fn who_takes_the_evading_offset() {
    let pi = std::f32::consts::PI;
    let s = |x: f32, heading: f32, moving: bool, offset: f32| Evader {
        at: [x, 0.0],
        heading,
        moving,
        offset,
    };
    let full = (1.0 - 1.44 / (2.4 * 2.4)) * 1.2 * 0.5;
    // Walking away from a stander: the walker the full f, signed by his
    // lean (here negative), the stander nothing.
    let o = evade(
        &[s(0.0, pi / 2.0, true, -0.1), s(1.2, pi / 2.0, false, 0.4)],
        2.0,
        0.5,
        1.2,
        0.0,
    );
    assert!((o[0] - (-0.1 - full)).abs() < 1e-5, "{o:?}");
    assert!((o[1] - 0.4).abs() < 1e-5, "{o:?}");
    // Toward each other, leans -0.1 and +0.3: the sum positive, both + f/2,
    // added to their leans.
    let o = evade(
        &[s(0.0, pi / 2.0, true, -0.1), s(1.2, -pi / 2.0, true, 0.3)],
        2.0,
        0.5,
        1.2,
        0.0,
    );
    assert!((o[0] - (-0.1 + full / 2.0)).abs() < 1e-5, "{o:?}");
    assert!((o[1] - (0.3 + full / 2.0)).abs() < 1e-5, "{o:?}");
    // Leans summing below 0: both negative.
    let o = evade(
        &[s(0.0, pi / 2.0, true, -0.3), s(1.2, -pi / 2.0, true, 0.1)],
        2.0,
        0.5,
        1.2,
        0.0,
    );
    assert!((o[0] - (-0.3 - full / 2.0)).abs() < 1e-5, "{o:?}");
    assert!((o[1] - (0.1 - full / 2.0)).abs() < 1e-5, "{o:?}");
    // Two standers around one walker: the two pairs add up.
    let o = evade(
        &[
            s(0.0, 0.0, true, 0.0),
            s(1.2, 0.0, false, 0.0),
            s(-1.2, 0.0, false, 0.0),
        ],
        2.0,
        0.5,
        1.2,
        0.0,
    );
    assert!((o[0] - 2.0 * full).abs() < 1e-5, "{o:?}");
}

// TOOL-trace TRACE-5.3: the decay runs once per logic tick, a tick length
// (0.1 s) at a time: 0.075 a tick, 0.3 after 4 ticks; a frame without a
// tick (paused) takes nothing off.
// spec: GFX-ANIM-4.3, TOOL-trace TRACE-5.3
#[test]
fn evade_decays_per_tick() {
    let mut gone = [
        Evader {
            at: [0.0, 0.0],
            heading: 0.0,
            moving: true,
            offset: 1.0,
        },
        Evader {
            at: [50.0, 0.0],
            heading: 0.0,
            moving: true,
            offset: 0.0,
        },
    ];
    for _ in 0..4 {
        let o = evade(&gone, 2.0, 0.5, 1.2, crate::game::TICK_SECONDS);
        gone[0].offset = o[0];
    }
    assert!((gone[0].offset - 0.7).abs() < 1e-5);
}

// GFX-ANIM-4.3: a late game's crowd (thousands of settlers, many close
// together, walking and standing, leaning either way) gets the very same
// offsets from the search by neighbourhood as from the test of every pair:
// to the last bit, each settler's contributions added in the order of its
// partners. A position that is not a number takes every pair.
// spec: GFX-ANIM-4.3
#[test]
fn evade_by_neighbourhood_is_every_pairs_to_the_bit() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 40) as f32 / (1u64 << 24) as f32
    };
    for (count, side) in [(3, 5.0), (200, 40.0), (3000, 300.0), (4000, 120.0)] {
        let crowd: Vec<Evader> = (0..count)
            .map(|i| Evader {
                // Some on one spot, some on cell borders (whole numbers).
                at: if i % 17 == 0 {
                    [(i % 7) as f32 * 2.4, 2.4]
                } else {
                    [next() * side - side / 2.0, next() * side]
                },
                heading: next() * 6.3,
                moving: i % 3 != 0,
                offset: next() - 0.5,
            })
            .collect();
        for (start, radius_scale, scale, dt) in [(2.0, 0.5, 1.2, 0.1), (1.0, 1.0, 2.0, 0.0)] {
            let fast = evade(&crowd, start, radius_scale, scale, dt);
            let every = evade_every_pair(&crowd, start, radius_scale, scale, dt);
            let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&fast), bits(&every), "{count} settlers");
        }
    }
    let mut odd: Vec<Evader> = (0..100)
        .map(|i| Evader {
            at: [i as f32 * 0.5, 0.0],
            heading: 0.0,
            moving: true,
            offset: 0.0,
        })
        .collect();
    odd[50].at[0] = f32::NAN;
    let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(
        bits(&evade(&odd, 2.0, 0.5, 1.2, 0.1)),
        bits(&evade_every_pair(&odd, 2.0, 0.5, 1.2, 0.1))
    );
}

// GFX-OBJ-1.4a: a road removes the doodads on its fields whose type deletes
// by street, for good: they do not come back when the road goes. A sign type
// (the switch off) stays on the road; a flag removes every doodad of its
// field; a field nothing came over keeps its doodads.
// spec: GFX-OBJ-1.4a, LUA-PROP-9, TRN-ROAD-3.1
#[test]
fn a_road_removes_doodads_for_good() {
    use s2_assets::landscape::{ElementType, LandscapeElement};
    let mut world = flat_world(10, 10);
    let player = s2_sim::PlayerId(0);
    for at in [(2, 2), (5, 2)] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.step();
    world.submit(s2_sim::Command::BuildRoad {
        player,
        from: (2, 2),
        to: (5, 2),
    });
    world.step();
    assert!(world.road_at((3, 2)).is_some());
    let mut lookup = Lookup::default();
    lookup.doodads.insert(1, ("sign".into(), false));
    lookup.doodads.insert(2, ("grass".into(), true));
    let element = |name: &str| LandscapeElement {
        name: name.into(),
        mesh: format!("{name}.gr2"),
        shader: String::new(),
        rot: [0.0, 0.0],
        scale_x: [1.0, 1.0],
        scale_y: [1.0, 1.0],
        scale_z: [1.0, 1.0],
        offset: [0.0, 0.0],
        kind: ElementType::Doodad,
        blend_mode: 0,
        align_on_water: false,
        shadow_cast: false,
    };
    let elements = [element("sign"), element("grass")];
    let scenery = Scenery {
        elements: &elements,
        ..Scenery::default()
    };
    let on_road = s2_sim::grid::field_sample((3, 2));
    let on_flag = s2_sim::grid::field_sample((5, 2));
    let off = s2_sim::grid::field_sample((3, 6));
    let doodads = [(2, on_road), (1, on_road), (1, on_flag), (2, off)];
    let frame = Frame::new(10, 10);
    let shown = |world: &World| -> Vec<u32> {
        some_doodads(
            world,
            &lookup,
            &scenery,
            &frame,
            &doodads,
            &[0, 1, 2, 3],
            &mut Poses::default(),
        )
        .iter()
        .map(|p| p.key.2)
        .collect()
    };
    assert_eq!(shown(&world), [1, 3]);
    world.submit(s2_sim::Command::RemoveRoad { player, at: (3, 2) });
    world.step();
    assert!(world.road_at((3, 2)).is_none());
    assert_eq!(shown(&world), [1, 3]);
}

/// UI-GAME-13.2: the road-building icon: a flag on a road where the
/// road may continue and a flag fits, a road piece where no flag fits,
/// the no-entry flag where the road cannot go.
#[test]
fn road_icons() {
    let mut world = flat_world(10, 10);
    let player = s2_sim::PlayerId(0);
    world.submit(s2_sim::Command::PlaceFlag { player, at: (2, 2) });
    world.step();
    assert_eq!(
        road_icon(&world, player, (2, 2), (5, 2), false),
        [8, 456, 40, 32]
    );
    assert_eq!(
        road_icon(&world, player, (2, 2), (3, 2), false),
        [88, 456, 40, 32]
    );
    // A field outside the player's reach: the map's edge row is a
    // border no road uses.
    let blocked = road_icon(&world, player, (2, 2), (9, 9), false);
    assert!(blocked == [48, 456, 40, 32] || blocked == [8, 456, 40, 32]);
    assert_eq!(road_icon(&world, player, (2, 2), (2, 2), false)[0], 48);
}

// GFX-VIEW T8 (placement part): a mark per buildable field, offset by
// (0, 0, -1) and 1.6 wide; none on the cursor field.
// spec: GFX-VIEW-6.1 (GFX-VIEW T8)
#[test]
fn build_help_icons() {
    let world = flat_world(6, 6);
    let frame = Frame::new(6, 6);
    let all = build_icons(&world, &frame, None);
    let buildable: Vec<Point> = world
        .terrain()
        .fields()
        .filter(|&f| icon_column(world.placement_at(f)).is_some())
        .collect();
    assert!(!buildable.is_empty());
    assert_eq!(all.len(), buildable.len());
    let cursor = buildable[0];
    assert_eq!(
        build_icons(&world, &frame, Some(cursor)).len(),
        all.len() - 1
    );
    let [x, z] = frame.field_xz(cursor.0, cursor.1);
    assert_eq!(all[0].0, [x, 0.0, z]);
    // GFX-VIEW T11: a camera looking 3/5 down and 4/5 toward -z: the
    // quad spans ICON_SIZE along its right vector and up its up vector,
    // its bottom edge centred on the field point moved 1 toward it.
    let c = crate::render::world3d::to_bevy(all[0].0);
    let forward = [0.0, -0.6, -0.8];
    let up = [0.0, 0.8, -0.6];
    let (p, uv, idx) = billboards(&all[..1], [1.0, 0.0, 0.0], up, forward);
    assert_eq!((p.len(), idx.len()), (4, 6));
    assert!((p[1][0] - p[0][0] - ICON_SIZE).abs() < 1e-5);
    assert!((uv[1][0] - uv[0][0] - 0.125).abs() < 1e-6);
    let bottom = [0, 1, 2].map(|k| (p[0][k] + p[1][k]) / 2.0);
    let expected = [0, 1, 2].map(|k| c[k] - forward[k] * ICON_OFFSET);
    assert!((0..3).all(|k| (bottom[k] - expected[k]).abs() < 1e-5));
    let height = (0..3).map(|k| (p[3][k] - p[0][k]).powi(2)).sum::<f32>();
    assert!((height.sqrt() - ICON_SIZE).abs() < 1e-5);
    assert_eq!((uv[0][1], uv[3][1]), (1.0, 0.0));
    use s2_sim::Placement as P;
    assert_eq!(icon_column(P::Flag), Some(0.375));
    assert_eq!(icon_column(P::Harbor), Some(0.625));
    assert_eq!(slope_sprite(-1), Some([480, 128]));
    assert_eq!(slope_sprite(4), None);
}

// GFX-OBJ T3b, 2.6b: the Roman woodcutter (H 4.297) at k = 332, main
// progress 0.504545: the house is cut 2.353 above its base.
#[test]
fn rising_house_plane() {
    assert_eq!(rising_plane(3.0, 4.297, 0.0), 3.0);
    assert!((rising_plane(0.0, 4.297, 0.504545) - 2.353).abs() < 1e-3);
}

// GFX-OBJ T3b, T3c, T3d, 2.6a: the stage of the site model by frame
// progress, from the float32 counting of the woodcutter's 1 + 1
// planks (N = 16) and the generic site (N = 4, 1 or 2 frame planks).
#[test]
fn site_stages() {
    let f = |frame, k| 1.0 - s2_sim::remaining_work(frame, 1, 221, 220, k).0;
    // T3b: nothing at k = 0, 8 pieces at 25 %, all 16 from the frame's
    // end on.
    assert_eq!(site_stage(16, f(1, 0)), 0);
    assert_eq!(site_stage(16, f(1, 110)), 8);
    assert_eq!(site_stage(16, f(1, 221)), 16);
    assert_eq!(site_stage(16, f(1, 332)), 16);
    // T3c: the decrement at which each stage first shows.
    let first = |n: usize, frame: u32| -> Vec<u32> {
        (1..=n)
            .map(|stage| {
                (0..=2000)
                    .find(|&k| site_stage(n, f(frame, k)) >= stage)
                    .unwrap()
            })
            .collect()
    };
    assert_eq!(
        first(16, 1),
        [7, 20, 33, 46, 59, 72, 85, 98, 111, 123, 136, 149, 162, 175, 188, 201]
    );
    // T3i (Run D): stage 4, the four earth mounds (pieces 0 to 3, two
    // with plank stacks and the shovel) and no posts, from decrement 46
    // to 58; no ware objects are drawn at sites.
    for k in 46..=58 {
        assert_eq!(site_stage(16, f(1, k)), 4, "decrement {k}");
    }
    // T3d.
    assert_eq!(first(4, 1), [23, 67, 111, 155]);
    assert_eq!(first(4, 2), [45, 133, 221, 309]);
}

// GFX-ANIM-4.1a: a logic jump of the target (4.5 or 1.5 units here) is
// drawn over the next tick in six equal parts on a straight line, then
// he walks normally. (Run D: the woodcutter's walk home, for which Run C
// used this example, has no such jump any more; T16, T17 are now the
// sim's `t16_t17_walk_home_table`.)
#[test]
fn logic_jump_slides_over_one_tick() {
    let frame = crate::render::world3d::Frame::new(32, 32);
    let y = 0.0;
    let sample = |c: i64, r: i64| {
        let [x, z] = frame.sample_xz(c, r);
        [x, y, z]
    };
    let field = |x: u32, yy: u32| {
        let [fx, fz] = frame.field_xz(x, yy);
        [fx, y, fz]
    };
    assert_eq!(field(10, 10), sample(40, 40));
    for (stand, home, jump) in [((39, 40), (9, 10), -4.5f32), ((41, 40), (10, 10), -1.5)] {
        let spot = sample(stand.0, stand.1);
        let mut m = Motion::new(spot);
        m.tick(spot, false);
        m.tick(spot, false);
        // Tick T: the walk back, 0.1 of a step on toward (8, 10).
        let step = (field(home.0 - 1, home.1)[0] - field(home.0, home.1)[0]) * 0.1;
        let target = [field(home.0, home.1)[0] + step, y, spot[2]];
        assert!((field(home.0, home.1)[0] - spot[0] - jump).abs() < 1e-4);
        m.tick(target, true);
        for k in 1..=6 {
            let t = k as f32 / 6.0;
            let at = m.at(t);
            assert!((at[0] - (spot[0] + (target[0] - spot[0]) * t)).abs() < 1e-4);
            assert_eq!(at[2], spot[2]);
        }
        // Tick T + 1: normal walking from the target on.
        let next = [target[0] + step, y, spot[2]];
        m.tick(next, true);
        assert_eq!(m.at(0.0), target);
        assert_eq!(m.at(1.0), next);
    }
}

// GFX-ANIM T18, 4.1a: a standing settler's logic field changes: no
// movement until his next moving tick, then a one-tick slide from the
// old drawn spot. The tick he arrives in still takes his position.
#[test]
fn t18_standing_settler_keeps_his_target() {
    let (a, b, c) = ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [5.0, 0.0, 2.0]);
    let mut m = Motion::new(a);
    m.tick(b, false);
    assert_eq!(m.at(1.0), b, "arrival");
    m.tick(c, false);
    assert_eq!((m.at(0.0), m.at(1.0)), (b, b));
    m.tick(c, false);
    assert!(!m.slides());
    m.tick(c, true);
    assert_eq!((m.at(0.0), m.at(1.0)), (b, c));
    let half = m.at(0.5);
    assert_eq!(half, [3.0, 0.0, 1.0]);
}

// GFX-ANIM T11, 3.8a (corrected Run D): the fall starts in W + 30
// (3.0 s of game time into the 55-tick work); after the j-th fall tick
// 0.0043633 (1.2^j - 1) / 0.2: 0.0044 (W + 30), 0.1727 (W + 41),
// 1.1826 (W + 51), 1.4235 (W + 52); in W + 53 it would pass pi / 2, so
// it stops at the last angle drawn. A tick's target is drawn fully at
// the start of the next tick (GFX-ANIM-4.1).
#[test]
fn tree_fall_angle() {
    assert_eq!(fall_angle(None, 100.0), Fall::Standing);
    assert_eq!(fall_start(10), 40);
    assert!((FALL_FIRST_TURN - 0.043633).abs() < 1e-5);
    let angle = |now: f32| match fall_angle(Some(0), now) {
        Fall::Falling(a) => a,
        other => panic!("{other:?} at {now}"),
    };
    assert_eq!(fall_angle(Some(0), 29.99), Fall::Standing);
    assert_eq!(angle(30.0), 0.0);
    for (tick, want) in [(30, 0.0044), (41, 0.1727), (51, 1.1826)] {
        let got = angle(tick as f32 + 1.0);
        assert!((got - want).abs() < 1e-4, "after W + {tick}: {got}");
    }
    // W + 52's target, 1.4235, is approached but never passed: the
    // last frame drawn is just under it.
    let late = angle(52.999);
    assert!(late < 1.4235 && late > 1.4232, "{late}");
    assert_eq!(fall_angle(Some(0), 53.0), Fall::Lying);
    assert_eq!(fall_angle(Some(0), 500.0), Fall::Lying);
}

// GFX-ANIM T11a, 3.8c: a removal that starts in the harvest tick
// W + 55 (the deposit is gone in W + 73): with h = 7.413 and y0 = 10.0
// the drawn height goes 10.0 -> 9.259 over the harvest tick, then the
// targets 10 - 0.05 (k + 2) h: 8.888 (W + 56), 7.405 (W + 60), 5.552
// (W + 65), 2.958 (W + 72).
#[test]
fn removed_deposit_sinks() {
    let y = |now: f32| 10.0 - sink_fraction(55, 73, now) * 7.413;
    assert_eq!(y(54.5), 10.0);
    assert_eq!(y(55.0), 10.0);
    assert!((y(55.5) - (10.0 - 0.05 * 7.413)).abs() < 1e-3);
    for (tick, want) in [
        (55, 9.259),
        (56, 8.888),
        (60, 7.405),
        (65, 5.552),
        (72, 2.958),
    ] {
        let got = y(tick as f32 + 1.0);
        assert!((got - want).abs() < 2e-3, "after W + {tick}: {got}");
    }
    // T11b and ECO-OUT-6.5: the same for every removal; a withered
    // field's counter starts at 1, so it sinks 0.05 h in its first tick.
    assert!((sink_fraction(100, 119, 101.0) - 0.05).abs() < 1e-6);
    assert!((sink_fraction(100, 119, 102.0) - 0.10).abs() < 1e-6);
}

// GFX-ANIM-3.8b: a growing tree is scaled height s, width min(3 s, 1):
// a fresh sapling (0.1) is 0.3 wide and 0.1 high, full width from 1/3.
#[test]
fn growth_scale_by_size() {
    let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 1e-5);
    assert!(close(growth_scale(0.1), [0.3, 0.1, 0.3]));
    assert!(close(growth_scale(1.0 / 3.0), [1.0, 1.0 / 3.0, 1.0]));
    assert!(close(growth_scale(0.5), [1.0, 0.5, 1.0]));
    assert!(close(growth_scale(1.0), [1.0; 3]));
    assert!(close(growth_scale(0.0), [0.0; 3]));
}

// GFX-VIEW-4.5: a sapling has the start size, a felled tree full size;
// a kind without a growing time is always full size.
#[test]
fn deposit_sizes() {
    let world = flat_world(4, 4);
    let look = DepositLook {
        gfx_name: "tree_01".into(),
        start_size: Some(0.2),
        speed: Some(0.0008),
    };
    assert_eq!(deposit_size(&world, 1, DepositState::Sapling, &look), 0.0);
    assert_eq!(
        deposit_size(
            &world,
            1,
            DepositState::Removing {
                since: 0,
                gone_at: 9
            },
            &look
        ),
        1.0
    );
    assert_eq!(
        deposit_size(&world, 1, DepositState::Growing { ripe_at: 50 }, &look),
        1.0
    );
}

// A flag stands at its field on the ground in its owner's colour;
// GFX-VIEW-4.3: at angle 0.
#[test]
fn flags_are_placed_at_their_field() {
    let mut world = flat_world(8, 8);
    let player = s2_sim::PlayerId(0);
    world.submit(s2_sim::Command::PlaceFlag { player, at: (3, 3) });
    world.step();
    assert_eq!(world.flag_at((3, 3)), Some(player));
    let frame = Frame::new(8, 8);
    let placed = objects(
        &world,
        &Lookup::default(),
        &ModelTables::default(),
        &Scenery::default(),
        &frame,
        false,
        &mut Poses::default(),
    );
    let flag = placed
        .iter()
        .find(|p| p.key == (Kind::Flag, (3, 3), 0))
        .expect("the flag");
    let [x, z] = frame.field_xz(3, 3);
    // GFX-OBJ-1.4: moved along z by its field's hash.
    assert_eq!(flag.at, [x, 0.0, z + flag_shift((3, 3), 8)]);
    assert_eq!((flag.file.as_str(), flag.heading), (models3d::FLAG, 0.0));
    assert_eq!(flag.player, models3d::player_colour(player));
    assert_eq!(flag.family, models3d::Family::Flag);
}

/// A game with a Roman headquarters (type 9, `Castle`) on (10, 10) of
/// a 100 x 100 map at raw height `h`, and the tables naming its model.
fn headquarters_game(h: i32) -> (World, Lookup, ModelTables) {
    use s2_sim::{BuildingSize, BuildingType};
    let fields = 100 * 100;
    let terrain = Terrain {
        width: 100,
        height: 100,
        heights: vec![h; fields],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; fields],
        samples: Vec::new(),
    };
    let mut castle = BuildingType::new(9, BuildingSize::Large);
    castle.territory_range = 9;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![castle],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![(s2_sim::PlayerId(0), (10, 10))],
        local_player: s2_sim::PlayerId(0),
        ..Default::default()
    };
    let world = World::new_game(1, terrain, rules, &setup);
    let mut lookup = Lookup::default();
    lookup.buildings.insert(9, "Castle".into());
    let mut tables = ModelTables::default();
    tables.read_buildings(
        r#"<Buildings><Building name="Roman_Castle" mesh="data\buildings\PB_R_Castle01.gr2"/></Buildings>"#,
    );
    (world, lookup, tables)
}

/// GFX-OBJ T27a (6.2, Q-221): road F-G asks for a donkey: both its
/// flags' crossings move to texture 1, F's although its other road is
/// plain; nothing moves back while the flags stand; a flag that is gone
/// leaves the set.
// spec: GFX-OBJ-6.2 (GFX-OBJ T27a)
#[test]
fn t27a_donkey_crossings_stay() {
    let (f, g, h) = ((2, 2), (4, 2), (6, 2));
    let mut moved = std::collections::HashSet::new();
    let stands = |_: Point| true;
    update_donkey_crossings(&mut moved, [], stands);
    assert!(moved.is_empty(), "plain roads: texture 0");
    update_donkey_crossings(&mut moved, [[f, g]], stands);
    assert!(moved.contains(&f) && moved.contains(&g) && !moved.contains(&h));
    // Later frames: the road F-G is still marked (never cleared) or,
    // were it gone, F and G keep texture 1 while they stand.
    update_donkey_crossings(&mut moved, [], stands);
    assert!(moved.contains(&f) && moved.contains(&g));
    update_donkey_crossings(&mut moved, [], |p| p != g);
    assert!(moved.contains(&f) && !moved.contains(&g));
}

/// GFX-OBJ-6.2a (d) "When the count is taken" (T47, Q-1048, F58): roads R1
/// (2, 2)-(6, 2) and then R2 (6, 2)-(10, 2) and R3 (6, 2)-(6, 6) from flag F
/// (6, 2) in turn: R1, the first road at F, runs to F; R2 and R3 stop 1/8 of
/// their first step short of F (0.8722 on a 6-unit step); the far ends run
/// home. With R1 removed R2 and R3 still stop short (a strip keeps its
/// shape).
// spec: GFX-OBJ-6.2a (T47)
#[test]
fn t47_the_first_road_at_a_flag_runs_to_it() {
    let mut world = flat_world(20, 20);
    let player = s2_sim::PlayerId(0);
    let f = (6, 2);
    for at in [(2, 2), f, (10, 2), (6, 6)] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.step();
    let mut ends = RoadEnds::new();
    let build = |world: &mut World, ends: &mut RoadEnds, from, to| {
        world.submit(s2_sim::Command::BuildRoad { player, from, to });
        world.step();
        update_road_ends(ends, world);
    };
    build(&mut world, &mut ends, (2, 2), f);
    build(&mut world, &mut ends, f, (10, 2));
    build(&mut world, &mut ends, f, (6, 6));
    assert_eq!(world.roads().count(), 3);
    let end_at = |ends: &RoadEnds, path: &Vec<Point>, at: Point| -> bool {
        let e = ends[path];
        if path[0] == at {
            e[0]
        } else {
            e[1]
        }
    };
    let roads: Vec<Vec<Point>> = world.roads().map(|r| r.path.clone()).collect();
    let r1 = roads.iter().find(|p| p.contains(&(2, 2))).unwrap().clone();
    let r2 = roads.iter().find(|p| p.contains(&(10, 2))).unwrap().clone();
    let r3 = roads.iter().find(|p| p.contains(&(6, 6))).unwrap().clone();
    assert!(!end_at(&ends, &r1, f), "R1 runs to F");
    assert!(
        end_at(&ends, &r2, f) && end_at(&ends, &r3, f),
        "R2, R3 stop short at F"
    );
    assert!(
        !end_at(&ends, &r1, (2, 2)) && !end_at(&ends, &r2, (10, 2)) && !end_at(&ends, &r3, (6, 6))
    );
    // The strips as drawn: R1 reaches F (a vertex on F's x, within a unit of
    // its row), and none of R2 lies closer to F than 0.8722 on the right.
    let frame = Frame::new(20, 20);
    let [x6, z6] = frame.field_xz(f.0, f.1);
    let street = streets_in(&world, false, None, &ends, Paved::Ignored);
    let near: Vec<[f32; 3]> = street
        .positions
        .iter()
        .map(|p| crate::render::world3d::from_bevy(*p))
        .filter(|p| (p[2] - z6).abs() < 0.5)
        .collect();
    assert!(near.iter().any(|p| (p[0] - x6).abs() < 1e-3));
    assert!(!near.iter().any(|p| p[0] > x6 + 1e-3 && p[0] < x6 + 0.86));
    // R1 removed (its flag (2, 2) taken away): R2 and R3 keep stopping short.
    world.submit(s2_sim::Command::RemoveFlag { player, at: (2, 2) });
    world.step();
    update_road_ends(&mut ends, &world);
    assert_eq!(world.roads().count(), 2);
    assert!(!ends.contains_key(&r1));
    assert!(end_at(&ends, &r2, f) && end_at(&ends, &r3, f));
    // Seen for the first time (a loaded game): in the road order, the
    // first road at F runs to it.
    let fresh = road_ends_of(&world);
    assert_eq!(
        fresh.values().filter(|e| e.contains(&true)).count(),
        1,
        "{fresh:?}"
    );
}

// GFX-OBJ-6.4 (a) to (c), F14: a strip of road from the headquarters'
// flag to its field point, 6.0 long on flat ground and 2.0 wide (the
// street width is the half width, 6.2); v runs from 0 at the flag to
// 1.5 at the building, u from 0 to 1 across.
#[test]
fn door_strip_from_flag_to_building() {
    let (world, _, _) = headquarters_game(0);
    let frame = Frame::new(100, 100);
    let hq = (10, 10);
    let flag = world.terrain().neighbour(hq, 1).expect("a neighbour");
    assert!(world.flag_at(flag).is_some());
    let strip = door_strips(&world, &frame);
    assert!(!strip.positions.is_empty());
    let p0 = frame.field_xz(flag.0, flag.1);
    let p1 = frame.field_xz(hq.0, hq.1);
    let length = (p1[0] - p0[0]).hypot(p1[1] - p0[1]);
    assert!((length - 6.0).abs() < 1e-3);
    let dir = [(p1[0] - p0[0]) / length, (p1[1] - p0[1]) / length];
    let mut v_range = (f32::MAX, f32::MIN);
    let mut sides = std::collections::HashSet::new();
    for (p, uv) in strip.positions.iter().zip(&strip.uvs) {
        let [x, _, z] = crate::render::world3d::from_bevy(*p);
        let (dx, dz) = (x - p0[0], z - p0[1]);
        let along = dx * dir[0] + dz * dir[1];
        let across = dx * -dir[1] + dz * dir[0];
        assert!((-1e-3..=6.0 + 1e-3).contains(&along), "{along}");
        assert!(across.abs() <= 1.0 + 1e-3, "{across}");
        assert!((uv[1] - along * 0.25).abs() < 1e-3, "v {uv:?} at {along}");
        // u = 0 on one side, 1 on the other, 0.5 on the centre line.
        assert!(((uv[0] - 0.5).abs() - across.abs() / 2.0).abs() < 1e-3);
        if across.abs() > 0.1 {
            sides.insert((uv[0] > 0.5) == (across > 0.0));
        }
        v_range = (v_range.0.min(uv[1]), v_range.1.max(uv[1]));
    }
    assert_eq!(sides.len(), 1, "u grows to one side");
    assert!(v_range.0.abs() < 1e-3 && (v_range.1 - 1.5).abs() < 1e-3);
    let across: Vec<f32> = strip
        .positions
        .iter()
        .map(|p| {
            let [x, _, z] = crate::render::world3d::from_bevy(*p);
            (x - p0[0]) * -dir[1] + (z - p0[1]) * dir[0]
        })
        .collect();
    let widest = across.iter().cloned().fold(f32::MIN, f32::max)
        - across.iter().cloned().fold(f32::MAX, f32::min);
    assert!((widest - 2.0).abs() < 1e-3, "{widest}");
}

// GFX-VIEW-4.3a to c, T16: the headquarters is drawn turned by -30
// degrees at its field's own sample height.
#[test]
fn buildings_face_their_flag() {
    let (world, lookup, tables) = headquarters_game(5000);
    let frame = Frame::new(100, 100);
    let placed = objects(
        &world,
        &lookup,
        &tables,
        &Scenery::default(),
        &frame,
        false,
        &mut Poses::default(),
    );
    let hq = placed
        .iter()
        .find(|p| p.key.0 == Kind::Building && p.key.1 == (10, 10))
        .expect("the headquarters");
    assert!(
        (hq.heading - (-30f32).to_radians()).abs() < 1e-6,
        "{}",
        hq.heading
    );
    let [x, z] = frame.field_xz(10, 10);
    assert_eq!(hq.at, [x, 5.0, z]);
    assert_eq!(
        hq.at,
        crate::render::world3d::building_position(world.terrain(), 10, 10)
    );
}

/// GFX-OBJ-2.2, 2.2a: each house draws with its own slot's colour column:
/// the headquarters, created first, slot 1 (column 1, offset 1/8), or the
/// slot the view's table gave it.
// spec: GFX-OBJ-2.2, GFX-OBJ-2.2a
#[test]
fn every_house_takes_its_slots_colour_column() {
    let (mut world, lookup, tables) = headquarters_game(5000);
    world.submit(s2_sim::Command::PlaceBuilding {
        player: s2_sim::PlayerId(0),
        building: 9,
        at: (14, 14),
    });
    world.step();
    let place = |scenery: &Scenery| {
        objects(
            &world,
            &lookup,
            &tables,
            scenery,
            &Frame::new(100, 100),
            false,
            &mut Poses::default(),
        )
    };
    let house_of = |placed: &[Placed], at: Point| {
        placed
            .iter()
            .find(|p| p.key.0 == Kind::Building && p.key.1 == at)
            .map(|p| p.extra[0])
    };
    let hq = world
        .buildings()
        .find(|b| !b.under_construction)
        .expect("the headquarters");
    let site = world
        .buildings()
        .find(|b| b.under_construction)
        .expect("the site");
    assert!(hq.serial < site.serial);
    let afresh = place(&Scenery::default());
    assert_eq!(house_of(&afresh, hq.at), Some(1.0 / 8.0));
    // The test's tables give the site no frame model: its house shows, in
    // slot 2's column.
    assert_eq!(house_of(&afresh, site.at), Some(2.0 / 8.0));
    // A table whose seven earlier sites were torn down: the free stack 1 to
    // 7, 7 on top.
    let mut table = HouseSlots::default();
    let earlier: Vec<House> = (0..7)
        .map(|i| House {
            serial: i,
            at: (90, i),
            finished: false,
            building_type: 1,
            owner: s2_sim::PlayerId(0),
        })
        .collect();
    table.update(Some((0, &earlier)), 0.0);
    table.update(Some((1, &[])), 0.0);
    table.update(Some((2, &houses(&world))), 0.0);
    // The headquarters (created first) 7, the site 6.
    assert_eq!(table.slot(hq.serial), Some(7));
    assert_eq!(table.slot(site.serial), Some(6));
    let scenery = Scenery {
        house_slots: Some(&table),
        ..Scenery::default()
    };
    let placed = place(&scenery);
    assert_eq!(house_of(&placed, hq.at), Some(7.0 / 8.0));
    assert_eq!(house_of(&placed, site.at), Some(6.0 / 8.0));
}

// GFX-OBJ-8.1, T19: the headquarters carries its soldier flag with
// all six meshes at the front right corner of its house box, turned
// so its banners fly toward the flag; no border flag (8.2).
#[test]
fn headquarters_soldier_flag() {
    let (world, lookup, tables) = headquarters_game(5000);
    let frame = Frame::new(100, 100);
    let mut boxes = HashMap::new();
    boxes.insert(
        r"data\buildings\PB_R_Castle01.gr2".to_owned(),
        [[-6.432, 0.0, -1.548], [6.501, 9.0, 8.119]],
    );
    let scenery = Scenery {
        house_boxes: Some(&boxes),
        ..Scenery::default()
    };
    let placed = objects(
        &world,
        &lookup,
        &tables,
        &scenery,
        &frame,
        false,
        &mut Poses::default(),
    );
    let flags: Vec<&Placed> = placed
        .iter()
        .filter(|p| p.key.0 == Kind::MilitaryFlag)
        .collect();
    assert_eq!(flags.len(), 1, "{flags:?}");
    let flag = flags[0];
    assert_eq!(flag.file, crate::world::decor::SOLDIER_FLAG);
    assert_eq!(flag.pieces, Some(6));
    let want = [-235.258, 5.0, -207.454];
    assert!(
        flag.at.iter().zip(&want).all(|(a, b)| (a - b).abs() < 2e-3),
        "{:?}",
        flag.at
    );
    assert!((flag.heading - 60f32.to_radians()).abs() < 1e-5);
    assert_eq!(flag.family, models3d::Family::Flag);
    // Without the house's box no flag is placed.
    let bare = objects(
        &world,
        &lookup,
        &tables,
        &Scenery::default(),
        &frame,
        false,
        &mut Poses::default(),
    );
    assert!(bare.iter().all(|p| p.key.0 != Kind::MilitaryFlag));
}

/// F55: the few doodads the view places again come out as the whole list gives
/// them; under a building every doodad is gone (GFX-OBJ-1.4a iv), the switch
/// on or off.
// spec: GFX-OBJ-1.3, GFX-OBJ-1.4a
#[test]
fn some_doodads_are_placed_as_the_whole_list_places_them() {
    use s2_assets::landscape::{ElementType, LandscapeElement};
    let (world, mut lookup, tables) = headquarters_game(5000);
    let frame = Frame::new(100, 100);
    lookup.doodads.insert(1, ("rock".into(), false));
    lookup.doodads.insert(2, ("grass".into(), true));
    let element = |name: &str| LandscapeElement {
        name: name.into(),
        mesh: format!("{name}.gr2"),
        shader: String::new(),
        rot: [0.0, 360.0],
        scale_x: [1.0, 2.0],
        scale_y: [1.0, 2.0],
        scale_z: [1.0, 2.0],
        offset: [0.5, -0.5],
        kind: ElementType::Doodad,
        blend_mode: 0,
        align_on_water: false,
        shadow_cast: false,
    };
    let elements = [element("rock"), element("grass")];
    let under = s2_sim::grid::field_sample((10, 10));
    let doodads = [(1, under), (2, under), (2, (120, 120)), (9, (121, 120))];
    let whole = Scenery {
        elements: &elements,
        doodads: &doodads,
        ..Scenery::default()
    };
    let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
    let from_all: Vec<Placed> =
        objects(&world, &lookup, &tables, &whole, &frame, false, &mut poses)
            .into_iter()
            .filter(|p| p.key.0 == Kind::Doodad)
            .collect();
    let numbers: Vec<u32> = from_all.iter().map(|p| p.key.2).collect();
    assert_eq!(numbers, [2], "under a building, of no known type");
    let few = some_doodads(
        &world,
        &lookup,
        &whole,
        &frame,
        &doodads,
        &[0, 1, 2, 3],
        &mut poses,
    );
    assert_eq!(few, from_all);
}

/// F55: the road meshes are looked at again when the roads, flags or buildings
/// changed, not when only time passed; ground that moved counts where something
/// of theirs stands, not elsewhere on the map; and a new thing changes the number
/// of its own square's mesh only.
// spec: GFX-OBJ-6.2, GFX-OBJ-6.4, TRN-ROAD-8.4
#[test]
fn road_meshes_wait_for_what_they_show() {
    let (mut world, _, _) = headquarters_game(5000);
    let moved = HashSet::new();
    let before = road_content(&world);
    let parts = street_parts(&world, &moved, &road_ends_of(&world));
    world.step();
    assert_eq!(road_content(&world), before, "nothing was built");
    assert_eq!(street_parts(&world, &moved, &road_ends_of(&world)), parts);
    // The building's cell and those next to it, not a far one.
    let (c, r) = s2_sim::grid::field_sample((10, 10));
    let home = (c / 16, r / 16);
    let square = road_chunk((10, 10));
    assert!(road_chunks_on_cells(&world, &[home]).contains(&square));
    assert!(road_chunks_on_cells(&world, &[(home.0 + 1, home.1)]).contains(&square));
    assert!(road_chunks_on_cells(&world, &[(home.0 + 9, home.1 + 9)]).is_empty());
    assert!(road_chunks_on_cells(&world, &[]).is_empty());
    // A flag: the roads' number changes.
    world.submit(s2_sim::Command::PlaceFlag {
        player: s2_sim::PlayerId(0),
        at: (14, 10),
    });
    world.step();
    assert!(
        world.flag_at((14, 10)).is_some(),
        "{:?}",
        world.last_results()
    );
    assert_ne!(road_content(&world), before);
}

/// F55: the squares' meshes hold together what the whole map's strips hold, each
/// road in the square of its first field, each crossing in that of its flag.
// spec: GFX-OBJ-6.2, GFX-OBJ-6.4
#[test]
fn street_parts_add_up_to_the_whole_map() {
    let mut world = flat_world(60, 10);
    let player = s2_sim::PlayerId(0);
    // A road over the edge of two squares.
    let (start, far) = ((22, 2), (26, 2));
    for at in [start, far] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.step();
    world.submit(s2_sim::Command::BuildRoad {
        player,
        from: start,
        to: far,
    });
    world.step();
    assert_eq!(world.roads().count(), 1, "{:?}", world.last_results());
    let frame = Frame::new(60, 10);
    let moved = HashSet::new();
    let parts = street_parts(&world, &moved, &road_ends_of(&world));
    let vertices = |part: u8| -> usize {
        parts
            .keys()
            .filter(|k| k.0 == part)
            .map(|k| {
                street_part(
                    &world,
                    &frame,
                    &moved,
                    &road_ends_of(&world),
                    part,
                    k.1,
                    false,
                )
                .positions
                .len()
            })
            .sum()
    };
    assert_eq!(vertices(0), streets(&world, true).positions.len());
    assert_eq!(
        vertices(1),
        streets(&world, false).positions.len() + door_strips(&world, &frame).positions.len()
    );
    assert_eq!(
        vertices(2),
        crossings(&world, &frame, &moved, false).positions.len()
    );
    assert_eq!(
        vertices(3),
        crossings(&world, &frame, &moved, true).positions.len()
    );
    assert!(vertices(1) > 0 && vertices(2) > 0);
    // The road in the square of its first field, its flags' discs in their own.
    let squares =
        |part: u8| -> Vec<_> { parts.keys().filter(|k| k.0 == part).map(|k| k.1).collect() };
    assert_eq!(squares(1), [road_chunk(start)]);
    assert_eq!(squares(2), [road_chunk(start), road_chunk(far)]);
    // A flag on the moved list changes its crossing's part.
    let moved = HashSet::from([far]);
    assert!(street_parts(&world, &moved, &road_ends_of(&world)).contains_key(&(3, road_chunk(far))));
    // Squares of 24 fields.
    assert_eq!(road_chunk((23, 24)), (0, 1));
    assert_eq!(road_chunk((48, 23)), (2, 0));
}

/// GFX-ANIM-3.8e, 4.1a, T16, T17 (F7, corrected Run D): a woodcutter's
/// drawn position from 14 ticks before to 14 after he picks up the log,
/// logged at quarter ticks with the view's tick rule (`Motion`, as
/// `sync_movers` feeds it). He stands at the fine point through the
/// work, drop and pickup, then walks home along the lattice back to his
/// field and on: no slide and no jump; no quarter tick moves him more
/// than a quarter of a free walk's tick (0.27 world units on level
/// ground, at most about 0.3). Skipped without `S2_GAME_DIR`;
/// `--nocapture` prints the log.
#[test]
fn woodcutter_walks_home_without_a_jump() {
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
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let animations = s2_assets::anim_tables::SettlerAnimations::default();
    let frame = crate::render::world3d::Heights::frame(world.terrain());
    let hut = rules
        .buildings
        .iter()
        .find(|b| b.name.as_deref() == Some("WoodcutterHut"))
        .map(|b| b.id)
        .expect("WoodcutterHut");
    let player = s2_sim::PlayerId(0);
    let at = crate::tools::scenes::building(&mut world, player, hut).expect("a hut");
    // The hut's worker: the settler whose job is at the hut.
    let worker = |world: &World| {
        world.settlers().find_map(|(id, s)| match &s.job {
            s2_sim::SettlerJob::Worker { target, work } if target.at == at => Some((id, *work)),
            _ => None,
        })
    };
    // Per tick: the work step, the mover's target and moving flag, and
    // the view's motion after the tick.
    let mut log: Vec<(u64, s2_sim::Work, Motion)> = Vec::new();
    // GFX-ANIM-3.1a, T19e: per tick, whether he plays a walk clip and
    // whether a step is in progress.
    let mut clips: Vec<(u64, s2_sim::Work, bool, bool)> = Vec::new();
    let mut motion: Option<Motion> = None;
    let mut pickup = None;
    for _ in 0..40_000 {
        world.step();
        let Some((id, work)) = worker(&world) else {
            motion = None;
            continue;
        };
        let movers = movers(&world, &lookup, &tables, &animations, &frame, &|_| false);
        let Some(mover) = movers.iter().find(|m| m.id == MoverId::Settler(id)) else {
            motion = None;
            continue;
        };
        let moving = matches!(&mover.anim, MoverAnim::Settler { state, .. } if state.moving);
        let stepping = world
            .settlers()
            .any(|(i, s)| i == id && s.walker.next.is_some());
        clips.push((world.tick(), work, moving, stepping));
        let m = motion.get_or_insert_with(|| Motion::new(mover.at));
        m.tick(mover.at, moving);
        log.push((world.tick(), work, *m));
        if pickup.is_none() && matches!(work, s2_sim::Work::TakeWare { second: true, .. }) {
            pickup = Some(world.tick());
        }
        // On until 14 ticks after the pickup and 2 into the free walk
        // home (T19e).
        let back = clips
            .iter()
            .filter(|c| matches!(c.1, s2_sim::Work::Back { .. }))
            .count();
        if pickup.is_some_and(|p| world.tick() >= p + 14) && back >= 3 {
            break;
        }
    }
    let pickup = pickup.expect("the woodcutter picked up a log");
    let window: Vec<_> = log
        .iter()
        .filter(|(t, ..)| *t + 14 >= pickup && *t <= pickup + 14)
        .collect();
    let mut samples = Vec::new();
    for (tick, work, m) in &window {
        for q in 0..4 {
            let f = q as f32 / 4.0;
            let p = m.at(f);
            samples.push((*tick as f32 + f, *work, p));
            println!(
                "tick {:>6.2} {:<40} drawn ({:9.3}, {:7.3}, {:9.3})",
                *tick as f32 + f - pickup as f32,
                format!("{work:?}"),
                p[0],
                p[1],
                p[2]
            );
        }
    }
    for pair in samples.windows(2) {
        let (a, b) = (pair[0].2, pair[1].2);
        let d = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        assert!(d <= 0.3 / 4.0, "a jump of {d} at {}", pair[1].0);
    }
    // Standing still through the drop and the pickup.
    let home = window
        .iter()
        .find(|(_, w, _)| matches!(w, s2_sim::Work::Return { .. }))
        .map(|(t, ..)| *t)
        .expect("the walk home");
    for (tick, work, m) in &window {
        if *tick <= home {
            assert!(
                !m.slides(),
                "moves before the walk home: {work:?} at {tick}"
            );
        }
    }
    // T19e: WalkWood (drawn in place) in the tick the fine walk back
    // arrives and the free walk to the flag is ordered, and never one
    // Stand tick between two walking ticks.
    let ordered = clips
        .iter()
        .find(|(t, w, ..)| *t >= pickup && matches!(w, s2_sim::Work::Back { .. }))
        .expect("the free walk home ordered");
    assert!(ordered.2, "Stand in the ordering tick {}", ordered.0);
    assert!(!ordered.3, "already stepping in the ordering tick");
    for w in clips.windows(3) {
        let consecutive = w[0].0 + 1 == w[1].0 && w[1].0 + 1 == w[2].0;
        assert!(
            !(consecutive && w[0].2 && !w[1].2 && w[2].2),
            "a Stand tick between two walks at {} ({:?})",
            w[1].0,
            w[1].1
        );
    }
}

/// GFX-ANIM T10a (stand-ins list row 10, Q-181 b): the hunted animal's
/// Dying starts in the tick the hunter's approach ends and his 55-tick
/// action starts (the animal still on the map): the tick before, it walks
/// or stands; from then on Dying, once, on its own instance, until the
/// logic removes it at the end of the action. A hunting lodge by the Storm
/// Coast headquarters on the real rules. Skipped without `S2_GAME_DIR`.
// spec: GFX-ANIM-5.2 (GFX-ANIM T10a), ECO-FOOD-5.3
#[test]
fn t10a_the_hunted_animal_dies_when_the_action_starts() {
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
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let animations = s2_assets::anim_tables::SettlerAnimations::default();
    let frame = crate::render::world3d::Heights::frame(world.terrain());
    let lodge = rules
        .buildings
        .iter()
        .find(|b| b.name.as_deref() == Some("HuntingLodge"))
        .map(|b| b.id)
        .expect("HuntingLodge");
    let player = s2_sim::PlayerId(0);
    let at = crate::tools::scenes::building(&mut world, player, lodge).expect("a lodge");
    let at_spot = |world: &World| {
        world.settlers().any(|(_, s)| {
            matches!(&s.job, s2_sim::SettlerJob::Worker { target, work }
                if target.at == at && matches!(work, s2_sim::Work::AtSpot { .. }))
        })
    };
    // Per tick while an animal is hunted: whether the hunter is in his
    // action and whether the hunted animal's mover plays Dying.
    let mut log: Vec<(u64, bool, Option<bool>)> = Vec::new();
    let mut prey = None;
    for _ in 0..60_000 {
        world.step();
        let hunted = world
            .animals()
            .find(|(_, a)| a.hunted_by.is_some_and(|t| t.at == at))
            .map(|(id, _)| id);
        prey = prey.or(hunted);
        let Some(id) = prey else {
            continue;
        };
        let movers = movers(&world, &lookup, &tables, &animations, &frame, &|_| false);
        let dying = movers
            .iter()
            .find(|m| m.id == MoverId::Animal(id))
            .map(|m| matches!(&m.anim, MoverAnim::Unique(_)));
        log.push((world.tick(), at_spot(&world), dying));
        if dying.is_none() {
            break;
        }
    }
    let a = log
        .iter()
        .position(|(_, action, _)| *action)
        .expect("the hunter's action started");
    assert!(a > 0, "the animal was hunted before the action");
    assert_eq!(log[a - 1].2, Some(false), "a - 1: not Dying");
    let removed = log
        .iter()
        .position(|(.., d)| d.is_none())
        .expect("the animal removed");
    assert!(
        log[a..removed].iter().all(|(.., d)| *d == Some(true)),
        "Dying from a until it is removed: {:?}",
        &log[a.saturating_sub(1)..=removed]
    );
    let ticks = log[removed].0 - log[a].0;
    assert!((55..=56).contains(&ticks), "removed {ticks} ticks after a");
}

/// ECO-HQ-4.3a, GFX-VIEW-4.2 (stand-ins list row 9): the leveller walks
/// from field to field around his site and is drawn where his step is, so
/// he never jumps a field in one tick; ECO-HQ-4.6b, TRN-MOVE-2.5 (row 8):
/// the builder is drawn on the lattice step under way, between two
/// neighbouring height samples, also without a jump. Three large sites by
/// the Storm Coast headquarters, built on the real rules. Skipped without
/// `S2_GAME_DIR`.
// spec: ECO-HQ-4.3a, ECO-HQ-4.6b, TRN-MOVE-2.5, GFX-VIEW-4.2
#[test]
fn builder_and_leveller_walk_without_a_jump() {
    use crate::render::world3d::{SAMPLE_ROW_STEP, SAMPLE_STEP};
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
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let animations = s2_assets::anim_tables::SettlerAnimations::default();
    let frame = crate::render::world3d::Heights::frame(world.terrain());
    let player = s2_sim::PlayerId(0);
    let large: Vec<u32> = rules
        .buildings
        .iter()
        .filter(|b| {
            b.size == Some(s2_rules::model::BuildingSize::Large)
                && b.costs.len() + b.frame_costs.len() > 0
        })
        .map(|b| b.id)
        .collect();
    let sites: Vec<Point> = large
        .iter()
        .filter_map(|&b| crate::tools::scenes::building(&mut world, player, b))
        .take(3)
        .collect();
    assert!(!sites.is_empty(), "no large site placed");
    let xz = |s: Point| frame.sample_xz(i64::from(s.0), i64::from(s.1));
    let flat = |a: [f32; 3], b: [f32; 3]| ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
    let mut last: std::collections::HashMap<MoverId, [f32; 3]> = Default::default();
    let (mut leveller_between, mut builder_between) = (0, 0);
    for _ in 0..12_000 {
        world.step();
        let movers = movers(&world, &lookup, &tables, &animations, &frame, &|_| false);
        let seen = seen_points(&world, &frame);
        let mut now = std::collections::HashMap::new();
        for view in world.settler_views() {
            let id = match view.id {
                s2_sim::view::SettlerRef::Leveller(at) => MoverId::Leveller(at),
                s2_sim::view::SettlerRef::Builder(at) => MoverId::Builder(at),
                _ => continue,
            };
            let Some(mover) = movers.iter().find(|m| m.id == id) else {
                continue;
            };
            let point = seen.iter().find(|p| p.id == id).expect("a seen point");
            assert_eq!(
                point.at, mover.at,
                "{id:?}: the seen mark where he is drawn"
            );
            let moving = matches!(&mover.anim, MoverAnim::Settler { state, .. } if state.moving);
            match id {
                MoverId::Leveller(_) => {
                    let step = view.walk;
                    if let Some(next) = step.next {
                        let (a, b) = (
                            frame.field_xz(step.at.0, step.at.1),
                            frame.field_xz(next.0, next.1),
                        );
                        let f = step.fraction();
                        let want = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                        assert!(
                            (mover.at[0] - want[0]).abs() < 1e-3
                                && (mover.at[2] - want[1]).abs() < 1e-3,
                            "leveller {id:?} at {:?}, his step at {want:?}",
                            mover.at
                        );
                        assert!(
                            moving && mover.heading.is_some(),
                            "leveller {id:?} stepping but not walking"
                        );
                        if f > 0.0 {
                            leveller_between += 1;
                        }
                    }
                }
                _ => {
                    let fine = view.fine.expect("a builder's lattice walk");
                    if let Some(to) = fine.step.next.filter(|&to| to != fine.step.at) {
                        let (a, b) = (xz(fine.step.at), xz(to));
                        let span = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                        assert!(
                            span <= SAMPLE_STEP.max(SAMPLE_ROW_STEP) * 1.01,
                            "builder {id:?}: a step of {span} is not one lattice step"
                        );
                        let p = [mover.at[0], mover.at[2]];
                        let via = ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt()
                            + ((p[0] - b[0]).powi(2) + (p[1] - b[1]).powi(2)).sqrt();
                        assert!((via - span).abs() < 1e-3, "builder {id:?} off his step");
                        assert!(moving, "builder {id:?} stepping but not walking");
                        builder_between += 1;
                    }
                }
            }
            // No jump: a walk step is 0.045 of a field (6 units) a tick or
            // less, a lattice step a quarter of that; 0.5 leaves room.
            if let Some(before) = last.get(&id) {
                let d = flat(*before, mover.at);
                assert!(d < 0.5, "{id:?} jumps {d} at tick {}", world.tick());
            }
            now.insert(id, mover.at);
        }
        last = now;
    }
    assert!(
        leveller_between > 20,
        "only {leveller_between} leveller ticks between fields"
    );
    assert!(
        builder_between > 20,
        "only {builder_between} builder ticks between samples"
    );
}

/// GFX-ANIM-3.8d, T20 (play test 11): a forester planting on the real
/// rules plays his own work entry, `WorkLoop0Forester`, whose Sound events
/// are the four shovel digs and "plant grows" (no chopping, no tree fall:
/// the planted tree is not being felled); the sapling is not drawn while he
/// digs (size 0) and appears when the planting ends at the start size 0.1,
/// 0.3 wide and 0.1 high, then grows (GFX-ANIM-3.8b). Skipped without
/// `S2_GAME_DIR`.
// spec: GFX-ANIM-3.8d (T20), GFX-ANIM-3.8b, ECO-OUT-6.1
#[test]
fn t20_forester_plants_with_his_own_sounds_and_a_sapling() {
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
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let animations = crate::render::models3d::read_animations(dir.root());
    let frame = crate::render::world3d::Heights::frame(world.terrain());
    let hut = rules
        .buildings
        .iter()
        .find(|b| b.name.as_deref() == Some("ForesterHouse"))
        .map(|b| b.id)
        .expect("ForesterHouse");
    let player = s2_sim::PlayerId(0);
    let at = crate::tools::scenes::building(&mut world, player, hut).expect("a hut");
    let planting = |world: &World| {
        world.settlers().find_map(|(id, s)| match &s.job {
            s2_sim::SettlerJob::Worker {
                target,
                work: s2_sim::Work::AtSpot { spot },
            } if target.at == at => Some((id, *spot)),
            _ => None,
        })
    };
    let mut seen = None;
    for _ in 0..40_000 {
        world.step();
        if let Some(found) = planting(&world) {
            seen = Some(found);
            break;
        }
    }
    let (id, spot) = seen.expect("the forester plants");
    // His clip: the profession's own entry, with its own sounds.
    let movers = movers(&world, &lookup, &tables, &animations, &frame, &|_| false);
    let mover = movers
        .iter()
        .find(|m| m.id == MoverId::Settler(id))
        .expect("the forester is drawn");
    let MoverAnim::Settler {
        state,
        good,
        profession,
        tribe,
        ..
    } = &mover.anim
    else {
        panic!("a settler's clip");
    };
    let mut visual = crate::animation::settler_anim::SettlerVisual::new();
    visual.tick(
        state,
        &animations.entries,
        good.as_deref(),
        profession.as_deref(),
        *tribe,
    );
    let entry = visual.entry.clone().expect("a work entry");
    assert_eq!(entry, "WorkLoop0Forester");
    let sounds: Vec<(&str, f32)> = animations
        .entries
        .iter()
        .find(|e| e.name == entry)
        .unwrap()
        .events
        .iter()
        .filter(|e| e.sound)
        .map(|e| (e.name.as_str(), e.start))
        .collect();
    assert_eq!(
        sounds,
        [
            ("shovel", 0.3),
            ("shovel", 2.35),
            ("shovel", 5.28),
            ("shovel", 8.73),
            ("plant grows", 7.3)
        ]
    );
    // Not a felling: no tree fall and its sound.
    assert_eq!(world.felling_began(spot), None);
    // The sapling: invisible while he digs, at the start size when the
    // planting ends.
    let size = |world: &World| {
        let d = world.deposit(spot).expect("the sapling");
        deposit_size(world, d.kind, d.state, &lookup.deposits[&d.kind])
    };
    assert_eq!(size(&world), 0.0);
    while planting(&world).is_some() {
        assert_eq!(size(&world), 0.0, "tick {}", world.tick());
        world.step();
    }
    let first = size(&world);
    assert!((first - 0.1).abs() < 0.001, "{first}");
    let scale = growth_scale(first);
    assert!((scale[0] - 0.3).abs() < 0.003 && (scale[1] - 0.1).abs() < 0.001);
    for _ in 0..500 {
        world.step();
    }
    let later = size(&world);
    assert!(later > 0.5 && later < 0.6, "{later}");
}

/// GFX-ANIM-3.8e (F7, Run D): the woodcutter's drawn position every
/// frame (6 a tick, 60 frames a second at speed 1) from his last chop
/// (W + 25) until he has reached the second field of his walk home,
/// with the tree's fall angle and sink, written to `$S2_F7_LOG` (or
/// printed). Compare with the table of 3.8e. Ignored: run with
/// `--ignored`; needs `S2_GAME_DIR`.
#[test]
#[ignore]
fn woodcutter_frames_from_last_chop() {
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
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let animations = s2_assets::anim_tables::SettlerAnimations::default();
    let frame = crate::render::world3d::Heights::frame(world.terrain());
    let hut = rules
        .buildings
        .iter()
        .find(|b| b.name.as_deref() == Some("WoodcutterHut"))
        .map(|b| b.id)
        .expect("WoodcutterHut");
    let player = s2_sim::PlayerId(0);
    let at = crate::tools::scenes::building(&mut world, player, hut).expect("a hut");
    let worker = |world: &World| {
        world.settlers().find_map(|(id, s)| match &s.job {
            s2_sim::SettlerJob::Worker { target, work } if target.at == at => {
                Some((id, *work, s.walker.at))
            }
            _ => None,
        })
    };
    let mut lines = Vec::new();
    let mut motion: Option<Motion> = None;
    // W: the tick the work action ran first (`world.tick()` after a
    // step is one past it).
    let (mut w, mut tree, mut home_fields) = (None, None, Vec::new());
    for _ in 0..40_000 {
        world.step();
        let Some((id, work, field)) = worker(&world) else {
            motion = None;
            continue;
        };
        let ran = world.tick() - 1;
        if let (None, s2_sim::Work::AtSpot { spot }) = (w, work) {
            w = Some(ran);
            tree = Some(spot);
        }
        let movers = movers(&world, &lookup, &tables, &animations, &frame, &|_| false);
        let Some(mover) = movers.iter().find(|m| m.id == MoverId::Settler(id)) else {
            motion = None;
            continue;
        };
        let moving = matches!(&mover.anim, MoverAnim::Settler { state, .. } if state.moving);
        let m = motion.get_or_insert_with(|| Motion::new(mover.at));
        m.tick(mover.at, moving);
        let Some(w) = w else { continue };
        if matches!(work, s2_sim::Work::Back { .. }) && home_fields.last() != Some(&field) {
            home_fields.push(field);
        }
        if ran < w + 25 {
            continue;
        }
        let tree = tree.expect("the tree");
        for k in 0..6 {
            let f = k as f32 / 6.0;
            let p = m.at(f);
            // As `sync_objects` passes them.
            let now = world.tick() as f32 + f;
            let fall = fall_angle(world.felling_began(tree), now);
            let sink = match world.deposit(tree).map(|d| d.state) {
                Some(DepositState::Removing { since, gone_at }) => {
                    format!("{:.2}", sink_fraction(since, gone_at, now - 1.0))
                }
                Some(_) => "-".to_owned(),
                None => "gone".to_owned(),
            };
            lines.push(format!(
                "W+{:<5.2} {:<44} field {:?} drawn ({:9.3}, {:7.3}, {:9.3}) moving {} tree {:?} sink {}",
                ran as f32 - w as f32 + f,
                format!("{work:?}"),
                field,
                p[0],
                p[1],
                p[2],
                moving,
                fall,
                sink
            ));
        }
        // His start field, then two more.
        if home_fields.len() >= 3 {
            break;
        }
    }
    let text = lines.join("\n");
    match std::env::var_os("S2_F7_LOG") {
        Some(out) => std::fs::write(out, text + "\n").expect("the log"),
        None => println!("{text}"),
    }
    assert!(home_fields.len() >= 3, "the walk home did not get far");
}

/// GFX-OBJ-1.4, T1, T2: an instance's yaw and size come from the
/// element's ranges; fixed ranges give fixed values.
// spec: GFX-OBJ-1.4
#[test]
fn element_draws_stay_in_range() {
    let list = s2_assets::landscape::read(
        r#"<x><LandscapeElement name="tree_14" mesh="m" shader="tree_shader.fx"
        rot="0 360" scaleX="0.6 0.8" scaleY="0.6 0.8" scaleZ="0.6 0.8" type="tree"/>
        <LandscapeElement name="medstoneressource_03" mesh="s" shader="stone_shader.fx"
        type="stone"/></x>"#,
    );
    let mut rng = s2_sim::rng::Rng::new(VIEW_SEED);
    for _ in 0..50 {
        let (yaw, scale) = element_draw(&list[0], &mut rng);
        assert!((0.0..=std::f32::consts::TAU).contains(&yaw));
        assert!(scale.iter().all(|s| (0.6..=0.8).contains(s)));
    }
    assert_eq!(element_draw(&list[1], &mut rng), (0.0, [1.0; 3]));
    assert_eq!(
        models3d::Family::of_shader(&list[0].shader),
        models3d::Family::Tree
    );
    assert_eq!(
        models3d::Family::of_shader(&list[1].shader),
        models3d::Family::Stone
    );
}

/// GFX-OBJ-6.2b: the spec's test: a straight waterway of 3 points along
/// +x at the water level, (0, 0), (1.5, 0), (3, 0): buoys at
/// (0.75, -1.7), (0.75, 1.7), (2.25, -1.7), (2.25, 1.7) (x, z). A bend's
/// pair stands across the curve's midpoint, 1.7 to each side.
// spec: GFX-OBJ-6.2b
#[test]
fn buoys_a_pair_per_step() {
    let straight = buoy_points(&[[0.0, 0.0], [1.5, 0.0], [3.0, 0.0]], -0.5);
    let want = [[0.75, -1.7], [0.75, 1.7], [2.25, -1.7], [2.25, 1.7]];
    assert_eq!(straight.len(), 4);
    for (p, w) in straight.iter().zip(want) {
        assert!(
            (p[0] - w[0]).abs() < 1e-5 && (p[1] - w[1]).abs() < 1e-5,
            "{straight:?}"
        );
    }
    let bent = buoy_points(&[[0.0, 0.0], [2.0, 0.0], [2.0, 4.0]], -0.5);
    assert_eq!(bent.len(), 4);
    for pair in bent.chunks(2) {
        let d = [pair[0][0] - pair[1][0], pair[0][1] - pair[1][1]];
        assert!((d[0].hypot(d[1]) - 3.4).abs() < 1e-4, "{bent:?}");
    }
    assert!(buoy_points(&[[0.0, 0.0]], -0.5).is_empty());
}

/// GFX-OBJ-6.2a f: a waterway has no strip; its buoys stand at the
/// water level as `street_buoy.GR2`.
// spec: GFX-OBJ-6.2a, GFX-OBJ-6.2b
#[test]
fn waterway_buoys_not_strip() {
    // A lake over x 13 to 15, its shores Coast (as the sim's waterway
    // tests), flags on both shores and a waterway between them.
    use s2_sim::map::state;
    let (w, h) = (30u32, 20u32);
    let mut terrain = flat_world(w, h).terrain().clone();
    let water = |f: (u32, u32)| (13..=15).contains(&f.0) && (4..=14).contains(&f.1);
    for f in terrain.fields() {
        let i = terrain.index(f);
        if water(f) {
            terrain.states[i] = state::WATER;
        }
    }
    for f in terrain.fields() {
        let i = terrain.index(f);
        if !water(f) && terrain.neighbours(f).into_iter().flatten().any(water) {
            terrain.states[i] |= state::COAST;
        }
    }
    let player = s2_sim::PlayerId(0);
    let mut world = World::with_map(1, terrain, s2_sim::Rules::empty(), player);
    let frame = Frame::new(w, h);
    for at in [(12, 9), (16, 9)] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.submit(s2_sim::Command::BuildWaterway {
        player,
        from: (12, 9),
        to: (16, 9),
    });
    world.step();
    let road = world.roads().find(|r| r.water).expect("the waterway");
    let ends = (road.path[0], road.path[road.path.len() - 1]);
    assert!(streets(&world, false).positions.is_empty());
    let placed = objects(
        &world,
        &Lookup::default(),
        &ModelTables::default(),
        &Scenery::default(),
        &frame,
        false,
        &mut Poses::default(),
    );
    let buoys: Vec<&Placed> = placed.iter().filter(|p| p.key.0 == Kind::Buoy).collect();
    let points: Vec<[f32; 2]> = world
        .roads()
        .find(|r| r.water)
        .unwrap()
        .path
        .iter()
        .map(|p| frame.field_xz(p.0, p.1))
        .collect();
    assert_eq!(ends, ((12, 9), (16, 9)));
    // GFX-OBJ-6.2b: two per step, not turned.
    assert_eq!(buoys.len(), 2 * (points.len() - 1));
    assert!(buoys
        .iter()
        .all(|p| p.file == models3d::STREET_BUOY && p.at[1] == WATER_LEVEL && p.heading == 0.0));
}

// GFX-VIEW T6: a step from A to its direction-0 neighbour B at progress
// 0.25 is drawn at A + 0.25 (B - A), heading 90 degrees.
// spec: GFX-VIEW-4.2 (GFX-VIEW T6)
#[test]
fn walker_between_fields() {
    let world = flat_world(8, 8);
    let frame = Frame::new(8, 8);
    let mut walker = s2_sim::view::Step::standing((3, 3));
    walker.next = Some((4, 3));
    walker.done = walker.total / 4;
    let (at, heading) = walker_point(&world, &frame, &walker, None);
    let a = frame.field_xz(3, 3);
    let b = frame.field_xz(4, 3);
    assert!((at[0] - (a[0] + 0.25 * (b[0] - a[0]))).abs() < 1e-4);
    assert!((at[2] - a[1]).abs() < 1e-4);
    assert!((heading.unwrap() - 90f32.to_radians()).abs() < 1e-6);
    // Standing: at its field, no heading.
    let standing = s2_sim::view::Step::standing((3, 3));
    assert_eq!(walker_point(&world, &frame, &standing, None).1, None);
    // A ship floats at the given level.
    assert_eq!(walker_point(&world, &frame, &walker, Some(-1.0)).0[1], -1.0);
}

// GFX-ANIM T4: from 0 toward 90 degrees the heading turns 25.2 degrees a
// tick: 25.2, 50.4, 75.6, 90; and the shorter way round.
// spec: GFX-ANIM-4.2 (GFX-ANIM T4)
#[test]
fn heading_eases_per_tick() {
    let target = 90f32.to_radians();
    let mut h = 0.0;
    let mut seen = Vec::new();
    for _ in 0..4 {
        h = ease_heading(h, target, TURN_PER_TICK);
        seen.push((h.to_degrees() * 10.0).round() / 10.0);
    }
    assert_eq!(seen, [25.2, 50.4, 75.6, 90.0]);
    // From 170 to -170 degrees the short way is +20 through 180.
    let h = ease_heading(170f32.to_radians(), -170f32.to_radians(), TURN_PER_TICK);
    assert!((h.to_degrees() - (-170.0)).abs() < 1e-3);
    let half = lerp_heading(170f32.to_radians(), -170f32.to_radians(), 0.5);
    assert!((half.to_degrees().abs() - 180.0).abs() < 1e-3);
}

// spec: GFX-OBJ-1.7
#[test]
fn character_names() {
    assert_eq!(character_name(0, "Woodcutter"), "Roman_Woodcutter");
    assert_eq!(character_name(1, &soldier_kind(0)), "African_Soldier");
    assert_eq!(soldier_kind(3), "Soldier3");
}

// ECO-PLACE T16 (ECO-PLACE-5.3): a decoration stands on the field whose
// sample point is nearest: E-trace-014's decorations by the farm site on
// (136, 58): fine (544, 234) on (136, 58), (540, 234) on (135, 58), (543, 236)
// on (135, 59); the kept (549, 233) on (137, 58).
// spec: ECO-PLACE-5.3 (ECO-PLACE T16)
#[test]
fn t16_a_decoration_stands_on_the_nearest_field() {
    let field = |s| decoration_field(s, 170, 170);
    assert_eq!(field((544, 234)), (136, 58));
    assert_eq!(field((540, 234)), (135, 58));
    assert_eq!(field((543, 236)), (135, 59));
    assert_eq!(field((549, 233)), (137, 58));
    assert_eq!(field((0, 0)), (0, 0));
}

// ECO-PLACE T17 (ECO-PLACE-5.3, Q-295): E-trace-006's decorations by the
// start castles of MP_2P_Storm_Coast (160 x 64 fields): (114, 208), halfway
// between (28, 52) and (29, 52), and (115, 208) stand on (29, 52); (561, 200)
// on the flag field (140, 50); (561, 208) on (140, 52); (562, 208), halfway,
// on (141, 52), the higher x.
// spec: ECO-PLACE-5.3 (ECO-PLACE T17)
#[test]
fn t17_a_decoration_halfway_stands_on_the_higher_x() {
    let field = |s| decoration_field(s, 160, 64);
    assert_eq!(field((114, 208)), (29, 52));
    assert_eq!(field((115, 208)), (29, 52));
    assert_eq!(field((561, 200)), (140, 50));
    assert_eq!(field((561, 208)), (140, 52));
    assert_eq!(field((562, 208)), (141, 52));
    // On an odd row the fields' samples lie at 4x + 2: halfway is 4x + 4.
    assert_eq!(field((44, 4)), (11, 1));
}

/// GFX-OBJ-1.4: the view's generator is SIM-RNG's seeded 0x5432ACE0; each
/// instance makes four unit draws (yaw, then x, y, z size), also for empty
/// ranges, in the order the view learns of the instances; an instance keeps
/// its pose, and one that went draws afresh when it comes back.
// spec: GFX-OBJ-1.4
#[test]
fn poses_draw_four_numbers_per_instance_in_order() {
    let list = s2_assets::landscape::read(
        r#"<x><LandscapeElement name="tree_14" mesh="m" shader="tree_shader.fx"
        rot="0 360" scaleX="0.6 0.8" scaleY="0.5 1.0" scaleZ="0.6 0.8" type="tree"/>
        <LandscapeElement name="medstoneressource_03" mesh="s" shader="stone_shader.fx"
        type="stone"/></x>"#,
    );
    let mut reference = s2_sim::rng::Rng::new(0x5432_ACE0);
    let mut unit = || (reference.draw() % 1_000_001) as f32 * 0.000_001;
    let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
    // An empty range moves the stream by four as well.
    let stone = poses.pose((Kind::Deposit, (1, 1), 2), &list[1]);
    assert_eq!(stone, (0.0, [1.0; 3]));
    for _ in 0..4 {
        unit();
    }
    let tree = poses.pose((Kind::Deposit, (2, 1), 1), &list[0]);
    let want = (
        (360.0 * unit()).to_radians(),
        [0.6 + 0.2 * unit(), 0.5 + 0.5 * unit(), 0.6 + 0.2 * unit()],
    );
    assert_eq!(tree, want);
    // Asked again: the same pose, no draws.
    assert_eq!(poses.pose((Kind::Deposit, (2, 1), 1), &list[0]), tree);
    let next = poses.pose((Kind::Deposit, (3, 1), 1), &list[0]);
    assert_eq!(next.0, (360.0 * unit()).to_radians());
    for _ in 0..3 {
        unit();
    }
    // Gone and back: a new instance, the next four draws.
    poses.forget(&(Kind::Deposit, (2, 1), 1));
    let again = poses.pose((Kind::Deposit, (2, 1), 1), &list[0]);
    assert_eq!(again.0, (360.0 * unit()).to_radians());
}

/// GFX-OBJ-1.4, T34: a road flag moves by -1.3 or +1.3 along z by the
/// 32-bit hash of its field's sample (c, r) on the height lattice, i = r x
/// (4 width + 1) + c, not of the field index: i = 0 gives v = 0x3434B6 and
/// 0x4B746F = 0x3426 -> -1.3; on Storm Coast (width 170) (10, 11) is
/// sample (42, 44), i = 30006, -1.3, and (130, 40) is (520, 160), i =
/// 109480, v = 4211814, +1.3 (the field index 6930 would give -1.3).
// spec: GFX-OBJ-1.4
#[test]
fn road_flags_shift_by_the_sample_hash() {
    let v = |i: u32| {
        let a = i.wrapping_mul(58);
        let u = a ^ a.wrapping_mul(8192);
        let t = u.wrapping_mul(u).wrapping_mul(0x4BD9).wrapping_add(0x2E48D);
        0x3434B6u32.wrapping_sub(t.wrapping_mul(u)) & 0x4B746F
    };
    let shift = |i: u32| {
        if v(i) as f32 * 2.981_05e-7 <= 0.5 {
            -1.3
        } else {
            1.3
        }
    };
    assert_eq!(flag_shift((0, 0), 64), -1.3);
    // T34.
    assert_eq!(v(109_480), 4_211_814);
    assert_eq!(flag_shift((10, 11), 170), -1.3);
    assert_eq!(flag_shift((130, 40), 170), 1.3);
    assert_eq!(shift(40 * 170 + 130), -1.3, "the field index differs");
    let mut seen = [false; 2];
    for y in 0..20u32 {
        for x in 0..20u32 {
            let c = 4 * x + if y % 2 == 1 { 2 } else { 0 };
            let s = flag_shift((x, y), 64);
            assert_eq!(s, shift(4 * y * 257 + c));
            seen[usize::from(s > 0.0)] = true;
        }
    }
    assert_eq!(seen, [true, true]);
}

/// GFX-OBJ-1.4, T32: a load gives the view a save's flags first (player 0
/// to 7, each in its list order), then the file's deposits in list order
/// (none for size -1), then the doodads, list 1, 2, 3, each by id (none for
/// a type with no element), then what the game made: the flags in the order
/// they were placed, the deposits not in the file, the signs. Four draws each.
// spec: GFX-OBJ-1.4
#[test]
fn a_load_gives_the_view_flags_deposits_then_doodads() {
    use s2_assets::landscape::{ElementType, LandscapeElement};
    use s2_assets::s2m::{Doodad, Doodads, FieldPos, SamplePos};
    let (world, mut lookup, _) = headquarters_game(5000);
    let element = |name: &str| LandscapeElement {
        name: name.into(),
        mesh: format!("{name}.gr2"),
        shader: String::new(),
        rot: [0.0, 360.0],
        scale_x: [1.0, 2.0],
        scale_y: [1.0, 2.0],
        scale_z: [1.0, 2.0],
        offset: [0.0, 0.0],
        kind: ElementType::Doodad,
        blend_mode: 0,
        align_on_water: false,
        shadow_cast: false,
    };
    let elements = [element("rock"), element("z_flag_00"), element("tree")];
    lookup.doodads.insert(1, ("rock".into(), false));
    let kind = 77;
    lookup.deposits.insert(
        kind,
        DepositLook {
            gfx_name: "tree".into(),
            start_size: None,
            speed: None,
        },
    );
    // The doodad lists out of id order, one of no known type.
    let doodad = |type_id, id, x| Doodad {
        type_id,
        id,
        position: SamplePos { x, y: 8 },
    };
    let lists = Doodads {
        non_blocking: vec![doodad(1, 30, 4), doodad(1, 10, 8), doodad(9, 20, 12)],
        unknown: vec![doodad(1, 5, 16)],
        blocking: vec![doodad(1, 7, 20)],
    };
    let doodads = load_doodads(&lists);
    assert_eq!(
        doodads,
        [
            (1, (8, 8)),
            (9, (12, 8)),
            (1, (4, 8)),
            (1, (16, 8)),
            (1, (20, 8))
        ]
    );
    // A file deposit list: one grown, one not started (size -1).
    let deposit = |id, x: u32, size| s2_assets::s2m::Deposit {
        type_id: kind,
        id,
        field: FieldPos { x, y: 30 },
        linked_id: 0,
        position: SamplePos { x: 4 * x, y: 120 },
        size,
        unknown_a: 0,
        unknown_b: -1,
    };
    let file = load_deposits(&[
        deposit(12, 31, 1.0),
        deposit(11, 30, -1.0),
        deposit(13, 29, 0.5),
    ]);
    assert_eq!(file, [((31, 30), kind), ((29, 30), kind)]);
    // A new game: no flags in the file; the headquarters' flag the game made.
    let scenery = Scenery {
        elements: &elements,
        load: Load {
            saved: false,
            deposits: Some(&file),
            doodads: &doodads,
        },
        ..Scenery::default()
    };
    let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
    let order = load_poses(&world, &lookup, &scenery, &mut poses);
    let hq_flags = world.flag_order().to_vec();
    assert!(!hq_flags.is_empty());
    let mut want: Vec<InstanceKey> = vec![
        (Kind::Deposit, (31, 30), kind),
        (Kind::Deposit, (29, 30), kind),
        (Kind::Doodad, (8, 8), 0),
        (Kind::Doodad, (4, 8), 2),
        (Kind::Doodad, (16, 8), 3),
        (Kind::Doodad, (20, 8), 4),
    ];
    want.extend(hq_flags.iter().map(|&f| (Kind::Flag, f, 0)));
    assert_eq!(&order[..want.len()], &want[..]);
    // The first instance's draws are the generator's first four.
    let mut rng = s2_sim::rng::Rng::new(VIEW_SEED);
    let first = element_draw(&elements[2], &mut rng);
    assert_eq!(poses.pose(want[0], &elements[2]), first);
    // A save: its flags come first.
    let saved = Scenery {
        load: Load {
            saved: true,
            ..scenery.load
        },
        ..scenery
    };
    let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
    let order = load_poses(&world, &lookup, &saved, &mut poses);
    assert_eq!(order[0], (Kind::Flag, hq_flags[0], 0));
    assert_eq!(order[hq_flags.len()], want[0]);
}

/// GFX-OBJ-1.4, T32, T33: the first free game of a program run on Storm
/// Coast: the first instance is the file's first deposit (id 11, field
/// (65, 103)) with the unit draws 0.491558, 0.170283, 0.4644, 0.828969, the
/// second deposit 12 on (66, 104); all 3212 deposits make instances, so the
/// first doodad's draws start at draw 12849. Deposit 2093 on (136, 52)
/// stands at its stored fine position (544, 207), not at its field's sample
/// (544, 208). Skipped without `S2_GAME_DIR`.
// spec: GFX-OBJ-1.4, TRN-FREE-5.4
#[test]
fn storm_coast_loads_its_deposits_first() {
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
    let world = s2_rules::new_game(1, &map, &rules, 0).expect("a game");
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let elements = crate::render::models3d::read_landscape(dir.root());
    let deposits = load_deposits(&map.deposits);
    let doodads = load_doodads(&map.doodads);
    assert_eq!(deposits.len(), 3212);
    let scenery = Scenery {
        elements: &elements,
        load: Load {
            saved: false,
            deposits: Some(&deposits),
            doodads: &doodads,
        },
        ..Scenery::default()
    };
    let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
    let order = load_poses(&world, &lookup, &scenery, &mut poses);
    assert_eq!(order[0].0, Kind::Deposit);
    assert_eq!(order[0].1, (65, 103));
    assert_eq!(order[1].1, (66, 104));
    assert!(order[..3212].iter().all(|k| k.0 == Kind::Deposit));
    assert_eq!(order[3212].0, Kind::Doodad);
    let mut rng = s2_sim::rng::Rng::new(VIEW_SEED);
    let units: Vec<f32> = (0..4).map(|_| rng.fraction()).collect();
    let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
    for (u, want) in units.iter().zip([0.491558, 0.170283, 0.4644, 0.828969]) {
        assert!(close(*u, want), "{u} against {want}");
    }
    // The first doodad draws from draw 12849 on (1-based).
    let mut rng = s2_sim::rng::Rng::new(VIEW_SEED);
    for _ in 0..12848 {
        rng.fraction();
    }
    let first_doodad = order[3212];
    let e = lookup
        .doodads
        .get(&doodads[first_doodad.2 as usize].0)
        .and_then(|gfx| scenery.element(&gfx.0))
        .expect("its element");
    assert_eq!(
        poses.pose(first_doodad, e),
        element_draw(e, &mut rng),
        "the first doodad's draws"
    );
    // T33: the map's deposit keeps its stored fine position.
    let tree = world
        .deposits()
        .find(|(f, _)| *f == (136, 52))
        .map(|(_, d)| d.position);
    assert_eq!(tree, Some((544, 207)));
}

/// GFX-OBJ-1.3, GFX-OBJ-2.7, GFX-GR2-5.1 (Q-1029): the grain field's
/// deposit is the landscape element `Wheatfield`, drawn as its mesh
/// `LO_Wheat.GR2` with the weed shader (the wheat family), never
/// `RS_Wheatfield01.GR2`, which the original does not read. Skipped without
/// `S2_GAME_DIR`.
// spec: GFX-OBJ-1.3, GFX-OBJ-2.7, GFX-GR2-5.1
#[test]
fn the_wheat_field_is_lo_wheat_with_the_weed_shader() {
    let Some(root) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
    let (rules, _) = s2_rules::load(&dir).expect("the property scripts");
    let tables = crate::render::models3d::read_tables(dir.root());
    let lookup = Lookup::new(&rules, &tables);
    let elements = crate::render::models3d::read_landscape(dir.root());
    let scenery = Scenery {
        elements: &elements,
        ..Scenery::default()
    };
    let wheat: Vec<&DepositLook> = lookup
        .deposits
        .values()
        .filter(|look| look.gfx_name.to_ascii_lowercase().contains("wheat"))
        .collect();
    assert!(!wheat.is_empty(), "a grain field deposit");
    for look in wheat {
        let e = scenery
            .element(&look.gfx_name)
            .unwrap_or_else(|| panic!("no element {:?}", look.gfx_name));
        assert!(
            e.mesh.to_ascii_lowercase().ends_with("lo_wheat.gr2"),
            "{}",
            e.mesh
        );
        assert_eq!(
            models3d::Family::of_shader(&e.shader),
            models3d::Family::Wheat
        );
    }
}

/// GFX-OBJ-8.7 (T36): a finished hut torn down with remains (MIL-CON-4.1):
/// in the same update its house collapses, in its slot's column, without
/// its ground decal, sinking by H x c^2 as the clock rises 0.35 a second,
/// and is gone at the first update after c reaches 1 (2.857 s); its ruin,
/// `Roman_RuinSmall01` by the hut's remains, stands on its field from the
/// same tick at the ruin entry's heading, on a copy of the hut's decal
/// alone. A ruin knows the building it was; the headquarters' slot and the
/// standing castle are untouched.
// spec: GFX-OBJ-8.7, GFX-OBJ-2.2a
#[test]
fn t36_a_finished_building_collapses_onto_its_ruin() {
    use s2_sim::{BuildingSize, BuildingType};
    let p0 = s2_sim::PlayerId(0);
    let fields = 100 * 100;
    let terrain = Terrain {
        width: 100,
        height: 100,
        heights: vec![5000; fields],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; fields],
        samples: Vec::new(),
    };
    let mut castle = BuildingType::new(9, BuildingSize::Large);
    castle.territory_range = 9;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let mut hut = BuildingType::new(5, BuildingSize::Small);
    hut.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![castle, hut],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        local_player: p0,
        ..Default::default()
    };
    let mut world = World::new_game(1, terrain, rules, &setup);
    let building = |building_type, at| s2_sim::RestoredBuilding {
        player: p0,
        building_type,
        at,
        remaining: None,
        levelled: 1.0,
        delivered: Vec::new(),
        production_on: true,
        stock: Vec::new(),
        worker: false,
        number: None,
        worker_number: None,
        soldiers: Vec::new(),
    };
    let (castle_at, hut_at) = ((10, 10), (20, 20));
    let report = world.restore(&s2_sim::Restore {
        players: vec![p0],
        owners: vec![Some(p0); fields],
        explored: vec![1; fields],
        buildings: vec![building(9, castle_at), building(5, hut_at)],
        ..Default::default()
    });
    assert_eq!(report.buildings, 2);
    let mut lookup = Lookup::default();
    lookup.buildings.insert(9, "Castle".into());
    lookup.buildings.insert(5, "Woodcutter".into());
    lookup.ruins.insert(5, "RuinSmall01".into());
    let mut tables = ModelTables::default();
    tables.read_buildings(
        r#"<Buildings>
            <Building name="Roman_Castle" mesh="data\buildings\PB_R_Castle01.gr2"/>
            <Building name="Roman_Woodcutter" mesh="data\buildings\PB_R_Woodcutter01.gr2"/>
            <Building name="Roman_RuinSmall01" mesh="data\buildings\PB_R_RuinSmall01.GR2"/>
        </Buildings>"#,
    );
    let mut slots = HouseSlots::of_world(&world);
    let hut_slot = slots
        .slot(world.building_at(hut_at).expect("the hut").serial)
        .expect("its slot");
    world.submit(s2_sim::Command::TearDown {
        player: p0,
        at: hut_at,
    });
    world.step();
    assert!(world.building_at(hut_at).is_none());
    let ruin = world.ruins().first().copied().expect("the hut's ruin");
    assert_eq!((ruin.at, ruin.building_type), (hut_at, Some(5)));
    let place = |slots: &HouseSlots| {
        objects(
            &world,
            &lookup,
            &tables,
            &Scenery {
                house_slots: Some(slots),
                ..Scenery::default()
            },
            &Frame::new(100, 100),
            false,
            &mut Poses::default(),
        )
    };
    // The removal's frame: the collapse starts at c = 0.
    slots.update(Some((world.tick(), &houses(&world))), 0.04);
    let placed = place(&slots);
    let collapse: Vec<&Placed> = placed
        .iter()
        .filter(|p| p.key.0 == Kind::Collapse)
        .collect();
    assert_eq!(collapse.len(), 1);
    let house = collapse[0];
    assert_eq!(house.key.1, hut_at);
    assert!(house.file.ends_with("PB_R_Woodcutter01.gr2"));
    assert_eq!(house.parts, Parts::NoDecal);
    assert_eq!(house.sink, Some(0.0));
    assert_eq!(house.extra[0], (hut_slot % 8) as f32 / 8.0);
    let ruins: Vec<&Placed> = placed.iter().filter(|p| p.key.0 == Kind::Ruin).collect();
    assert_eq!(ruins.len(), 2);
    let model = ruins.iter().find(|p| p.key.2 == 0).expect("the ruin model");
    assert!(model.file.ends_with("PB_R_RuinSmall01.GR2"));
    assert_eq!(model.parts, Parts::All);
    assert!((model.heading - (-30.0f32).to_radians()).abs() < 1e-6);
    // The site frame's pass (2.6a), every piece.
    assert_eq!(model.extra[3], 2.0);
    let decal = ruins
        .iter()
        .find(|p| p.key.2 == 1)
        .expect("the ruin's decal");
    assert!(decal.file.ends_with("PB_R_Woodcutter01.gr2"));
    assert_eq!(decal.parts, Parts::DecalOnly);
    assert_eq!(model.at, decal.at);
    // No standing house of the hut, the castle as before.
    assert!(!placed
        .iter()
        .any(|p| p.key.0 == Kind::Building && p.key.1 == hut_at));
    assert!(placed
        .iter()
        .any(|p| p.key.0 == Kind::Building && p.key.1 == castle_at));
    // The clock: 0.35 a second of frame time; sunk by H x c^2.
    let c = |slots: &HouseSlots| slots.collapsing().next().map(|(_, _, c)| c);
    let mut seconds = 0.0;
    while seconds < 1.0 - 1e-4 {
        slots.update(None, 0.1);
        seconds += 0.1;
    }
    assert!((c(&slots).expect("collapsing") - 0.35).abs() < 1e-4);
    assert!((collapse_height(2.0, 4.297, 0.35) - (2.0 - 4.297 * 0.1225)).abs() < 1e-4);
    assert!((collapse_height(2.0, 4.297, 1.0) - (2.0 - 4.297)).abs() < 1e-4);
    // c reaches 1 after 2.857 s; the house goes at the next update.
    for _ in 0..19 {
        slots.update(None, 0.1);
    }
    assert_eq!(c(&slots), Some(1.0));
    slots.update(None, 0.1);
    assert_eq!(c(&slots), None);
    assert!(!place(&slots).iter().any(|p| p.key.0 == Kind::Collapse));
}

/// GFX-OBJ-6.2c (T40): a flag on flat ground at height 0: the crossing is
/// the hexagon of six triangles around the flag's point, corners at 2.25
/// in the directions (sin a, 0, cos a), a = 60 .. 360, the texture flat in x
/// and z.
// spec: GFX-OBJ-6.2c (T40)
#[test]
fn t40_the_crossing_is_a_hexagon() {
    let mut world = flat_world(60, 10);
    let player = s2_sim::PlayerId(0);
    let (start, far) = ((22, 2), (26, 2));
    for at in [start, far] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.step();
    world.submit(s2_sim::Command::BuildRoad {
        player,
        from: start,
        to: far,
    });
    world.step();
    assert_eq!(world.roads().count(), 1);
    let frame = Frame::new(60, 10);
    let street = crossings(&world, &frame, &HashSet::new(), false);
    // Two flags, seven vertices and six triangles each.
    assert_eq!(street.positions.len(), 14);
    assert_eq!(street.indices.len(), 2 * 18);
    let [cx, cz] = frame.field_xz(start.0, start.1);
    let game = |k: usize| crate::render::world3d::from_bevy(street.positions[k]);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    let want = [
        ([0.0, 0.0], [0.5, 0.5]),
        ([1.9486, 1.125], [0.933, 0.75]),
        ([1.9486, -1.125], [0.933, 0.25]),
        ([0.0, -2.25], [0.5, 0.0]),
        ([-1.9486, -1.125], [0.067, 0.25]),
        ([-1.9486, 1.125], [0.067, 0.75]),
        ([0.0, 2.25], [0.5, 1.0]),
    ];
    // The first flag's crossing comes first (flags in field order).
    for (k, (d, uv)) in want.iter().enumerate() {
        let p = game(k);
        assert!(near(p[0] - cx, d[0]) && near(p[2] - cz, d[1]), "{k}: {p:?}");
        assert!(near(p[1], ROAD_LIFT), "{k}: {p:?}");
        let got = street.uvs[k];
        assert!(
            (got[0] - uv[0]).abs() < 1e-3 && (got[1] - uv[1]).abs() < 1e-3,
            "{k}: {got:?}"
        );
        assert!(near(street.normals[k][1], 1.0));
    }
    assert_eq!(
        street.indices[..18],
        [0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 5, 4, 0, 6, 5, 0, 1, 6]
    );
}

/// GFX-OBJ-6.2c (T44): a flag at x 0, z 0 with sample height 0.5, a
/// waterway in direction 1 (south-east), land roads in 0 and 3, the terrain
/// under the corners giving heights 0.6, 0.7, 0.4, 0.3, 0.45, 0.55 with
/// normals n0 .. n5: corner 1 keeps the flag's height and takes corner 0's
/// normal; the others lie on the terrain. With the waterway in direction 0
/// instead, corner 0 keeps the flag's height with the normal (0, 1, 0).
// spec: GFX-OBJ-6.2c (T44)
#[test]
fn t44_the_crossing_corner_beside_a_waterway() {
    let heights = [0.6, 0.7, 0.4, 0.3, 0.45, 0.55];
    let normal = |k: usize| {
        let v = [0.1 * k as f32, 1.0, -0.05 * k as f32];
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / l, v[1] / l, v[2] / l]
    };
    // The terrain under corner k (found by its angle).
    let ground = |x: f32, z: f32| {
        let a = x.atan2(z).to_degrees().rem_euclid(360.0);
        let k = ((a / 60.0).round() as usize + 5) % 6;
        (heights[k], normal(k))
    };
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
    let mut water = [false; 6];
    water[1] = true;
    let corners = crossing_corners([0.0, 0.5, 0.0], water, ground);
    assert!(near(corners[0].0, [1.9486, 0.6, 1.125]));
    assert!(near(corners[0].1, normal(0)));
    assert!(
        near(corners[1].0, [1.9486, 0.5, -1.125]),
        "{:?}",
        corners[1]
    );
    assert!(near(corners[1].1, normal(0)));
    assert!(near(corners[2].0, [0.0, 0.4, -2.25]));
    assert!(near(corners[2].1, normal(2)));
    for k in 3..6 {
        assert!((corners[k].0[1] - heights[k]).abs() < 1e-6);
        assert!(near(corners[k].1, normal(k)));
    }
    let mut water = [false; 6];
    water[0] = true;
    let corners = crossing_corners([0.0, 0.5, 0.0], water, ground);
    assert!(near(corners[0].0, [1.9486, 0.5, 1.125]));
    assert!(near(corners[0].1, [0.0, 1.0, 0.0]));
    assert!(near(corners[1].0, [1.9486, 0.7, -1.125]));
    assert!(near(corners[1].1, normal(1)));
    // Land roads only: every corner on the terrain.
    let corners = crossing_corners([0.0, 0.5, 0.0], [false; 6], ground);
    for (k, c) in corners.iter().enumerate() {
        assert!((c.0[1] - heights[k]).abs() < 1e-6 && near(c.1, normal(k)));
    }
}

/// F60 (GFX-OBJ-6.3, 6.2d b): the street pass is lit by its normals from
/// above, so every triangle of the roads, the flag-to-door strips and the
/// crossings turns the same way, toward the camera above (Bevy's front
/// face): the object shader turns a triangle's normal over when it is seen
/// from behind, which had lit our cobbled and dirt roads by the ambient
/// light alone, darker than their crossings (in the original road and
/// crossing stones are the same light colour).
// spec: GFX-OBJ-6.3, GFX-OBJ-6.2d (T46)
#[test]
fn f60_street_meshes_face_up() {
    // A road between two flags (as T40), and the headquarters' door strip.
    let mut world = flat_world(60, 10);
    let player = s2_sim::PlayerId(0);
    for at in [(22, 2), (26, 2)] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.step();
    world.submit(s2_sim::Command::BuildRoad {
        player,
        from: (22, 2),
        to: (26, 2),
    });
    world.step();
    assert_eq!(world.roads().count(), 1);
    let frame = Frame::new(60, 10);
    let (hq_world, _, _) = headquarters_game(0);
    // In Bevy's frame (y up), a triangle facing up has a positive y in
    // (b - a) x (c - a).
    let faces = |s: &Street| -> (usize, usize) {
        let mut out = (0, 0);
        for t in s.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|k| s.positions[t[k] as usize]);
            let (u, v) = ([b[0] - a[0], b[2] - a[2]], [c[0] - a[0], c[2] - a[2]]);
            let y = u[1] * v[0] - u[0] * v[1];
            if y > 0.0 {
                out.0 += 1;
            } else if y < 0.0 {
                out.1 += 1;
            }
        }
        out
    };
    let roads = faces(&streets(&world, false));
    let doors = faces(&door_strips(&hq_world, &Frame::new(100, 100)));
    let crossings = faces(&crossings(&world, &frame, &HashSet::new(), false));
    assert!(roads.0 > 0 && doors.0 > 0 && crossings.0 > 0);
    assert_eq!((roads.1, doors.1, crossings.1), (0, 0, 0));
}

/// Hook W21: without a paved look the street pass is the original's, paved
/// roads or not; with one, a paved land road leaves its part (0 or 1) for
/// the paved part, a mesh of its own, so becoming paved changes the numbers
/// of both parts; a road not paved stays where it was.
// spec: none (hook W21, docs/extending.md "The look of a paved road")
#[test]
fn a_paved_look_draws_paved_roads_in_a_part_of_their_own() {
    let mut world = flat_world(20, 20);
    let player = s2_sim::PlayerId(0);
    for at in [(2, 2), (6, 2), (10, 2)] {
        world.submit(s2_sim::Command::PlaceFlag { player, at });
    }
    world.step();
    for (from, to) in [((2, 2), (6, 2)), ((6, 2), (10, 2))] {
        world.submit(s2_sim::Command::BuildRoad { player, from, to });
        world.step();
    }
    let moved = HashSet::new();
    let ends = road_ends_of(&world);
    let number = world
        .roads()
        .find(|r| r.path[0] == (2, 2))
        .expect("the first road")
        .number;
    let plain = street_parts(&world, &moved, &ends);
    assert_eq!(street_parts_with(&world, &moved, &ends, true), plain);
    world.debug_pave_road(number);
    assert!(world.is_paved(number));
    // No look: the original's pass, the paved road drawn as a dirt road.
    assert_eq!(street_parts(&world, &moved, &ends), plain);
    let with = street_parts_with(&world, &moved, &ends, true);
    let square = road_chunk((2, 2));
    assert!(with.contains_key(&(PAVED_PART, square)));
    assert!(!plain.contains_key(&(PAVED_PART, square)));
    assert_ne!(with.get(&(1, square)), plain.get(&(1, square)));
    assert_eq!(road_part(false, true, true), PAVED_PART);
    assert_eq!(
        (road_part(true, true, false), road_part(false, true, false)),
        (0, 1)
    );
    // The strips: the paved road's in the paved part only, the other road's
    // in part 1 as before.
    let count = |paved| {
        streets_in(&world, false, None, &ends, paved)
            .positions
            .len()
    };
    let (all, left, only) = (
        count(Paved::Ignored),
        count(Paved::Left),
        count(Paved::Only),
    );
    assert!(only > 0 && left > 0);
    assert_eq!(left + only, all);
}

// GFX-ANIM-3.1 (Q-180 g): the swim test is against the water level: the
// middle of the body (foot + half the model's height) at or below it swims;
// a settler on dry ground above the water level never does, one whose field
// lies under the water plane deep enough does.
// spec: GFX-ANIM-3.1
#[test]
fn the_swim_test_is_the_water_level() {
    let level = crate::render::world3d::WATER_LEVEL;
    assert!(swims(level - 1.0, 2.0), "the middle exactly at the surface");
    assert!(!swims(level - 0.9, 2.0));
    assert!(!swims(level + 3.0, 2.0), "dry ground above the water");
    assert!(swims(level - 5.0, 2.0));
}

/// A world of 8 x 8 flat fields with two trees of kind 7 (1001 ticks to
/// ripe): a ripe one on (2, 2) and a young one on (5, 5) (`ripe_in` 667),
/// and the lookup giving kind 7 the element `tree_01` (start size 0.1,
/// 0.0009 a tick).
fn two_trees() -> (World, Lookup, s2_assets::landscape::LandscapeElement) {
    use s2_assets::landscape::{ElementType, LandscapeElement};
    let n = 64;
    let terrain = Terrain {
        width: 8,
        height: 8,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![0; n],
        samples: Vec::new(),
    };
    let kind = s2_sim::DepositKind {
        id: 7,
        good: None,
        next: None,
        elevation: 1,
        grow_ticks: 1001,
        blocking: false,
        life_time: None,
        needs_plantable: false,
        random_offset: false,
    };
    let rules = s2_sim::Rules {
        deposit_kinds: vec![kind],
        ..s2_sim::Rules::empty()
    };
    let deposit = |field: (u32, u32), ripe_in| s2_sim::MapDeposit {
        field,
        kind: 7,
        position: s2_sim::grid::field_sample(field),
        ripe_in,
    };
    let setup = s2_sim::GameSetup {
        deposits: vec![deposit((2, 2), 0), deposit((5, 5), 667)],
        ..Default::default()
    };
    let world = World::new_game(1, terrain, rules, &setup);
    let mut lookup = Lookup::default();
    lookup.deposits.insert(
        7,
        DepositLook {
            gfx_name: "tree_01".into(),
            start_size: Some(0.1),
            speed: Some(0.0009),
        },
    );
    let element = LandscapeElement {
        name: "tree_01".into(),
        mesh: "data/x/tree01.gr2".into(),
        shader: "tree_shader.fx".into(),
        rot: [0.0, 360.0],
        scale_x: [0.8, 1.0],
        scale_y: [0.8, 1.0],
        scale_z: [0.8, 1.0],
        offset: [0.0, 0.0],
        kind: ElementType::Tree,
        blend_mode: 0,
        align_on_water: false,
        shadow_cast: true,
    };
    (world, lookup, element)
}

/// Hook W30b: a tree kind with a project's look is drawn with its model in
/// the project tree family, with the same turn, sizes, place and shadow as
/// the original's (the element's draws, in the same order); a growing one
/// by the look's rule. Without a look everything is as before.
// spec: GFX-OBJ-1.4, GFX-ANIM-3.8b
#[test]
fn a_project_tree_takes_the_originals_place() {
    let (world, mut lookup, element) = two_trees();
    let frame = Frame::new(8, 8);
    let elements = [element];
    let scenery = Scenery {
        elements: &elements,
        ..Scenery::default()
    };
    let place = |lookup: &Lookup| {
        let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
        let mut out = objects(
            &world,
            lookup,
            &ModelTables::default(),
            &scenery,
            &frame,
            false,
            &mut poses,
        );
        out.retain(|p| p.key.0 == Kind::Deposit);
        out.sort_by_key(|p| p.key);
        out
    };
    let original = place(&lookup);
    assert_eq!(original.len(), 2);
    assert!(original.iter().all(|p| p.file == "data/x/tree01.gr2"));
    assert!(original
        .iter()
        .all(|p| p.family == models3d::Family::Tree && p.shadow));
    // No look of the kind's name: the same.
    lookup.tree_looks = tree_looks_by_kind(
        [(7, Some("BirchA"))],
        &[crate::extend::TreeLook::new("FirA", "fir.glb")],
    );
    assert!(lookup.tree_looks.is_empty());
    assert_eq!(place(&lookup), original);
    // A look: its model and family, the rest as the original's.
    let mut look = crate::extend::TreeLook::new("BirchA", "data/trees/birch.glb");
    lookup.tree_looks = tree_looks_by_kind([(7, Some("BirchA")), (8, None)], &[look.clone()]);
    let project = place(&lookup);
    for (p, o) in project.iter().zip(&original) {
        assert_eq!(p.file, "data/trees/birch.glb");
        assert_eq!(p.family, models3d::Family::FacetTree);
        assert_eq!(
            (p.key, p.at, p.heading, p.scale, p.shadow),
            (o.key, o.at, o.heading, o.scale, o.shadow)
        );
    }
    // The young tree: the original's growing; with RootWidth its width by
    // the square root of its size, as high.
    let young = |placed: &[Placed]| placed[1].scale;
    let full = place(&Lookup {
        tree_looks: HashMap::new(),
        ..lookup.clone()
    });
    let s = deposit_size(
        &world,
        7,
        world.deposit((5, 5)).expect("the young tree").state,
        &lookup.deposits[&7],
    );
    assert!(s > 0.0 && s < 1.0 / 3.0, "{s}");
    let element_scale = [0, 1, 2].map(|k| full[1].scale[k] / growth_scale(s)[k]);
    look.growth = crate::extend::TreeGrowth::RootWidth;
    lookup.tree_looks = tree_looks_by_kind([(7, Some("BirchA"))], &[look]);
    let root = place(&lookup);
    let want = [0, 1, 2].map(|k| element_scale[k] * root_width_growth(s)[k]);
    for (got, want) in young(&root).iter().zip(want) {
        assert!((got - want).abs() < 1e-4, "{:?}", young(&root));
    }
    assert_ne!(young(&root)[0], young(&full)[0]);
    assert!((young(&root)[1] - young(&full)[1]).abs() < 1e-6);
    // The ripe one is full grown either way.
    assert_eq!(root[0].scale, full[0].scale);
}

/// Hook W30b, `TreeGrowth::RootWidth`: height s, width the square root of
/// s; a third-grown tree 0.577 wide, both rules full at 1.
// spec: GFX-ANIM-3.8b
#[test]
fn root_width_growth_by_size() {
    let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 1e-5);
    assert!(close(root_width_growth(0.1), [0.316_228, 0.1, 0.316_228]));
    assert!(close(
        root_width_growth(1.0 / 3.0),
        [0.577_350, 1.0 / 3.0, 0.577_350]
    ));
    assert!(close(root_width_growth(0.25), [0.5, 0.25, 0.5]));
    assert!(close(root_width_growth(1.0), growth_scale(1.0)));
    assert!(close(root_width_growth(0.0), [0.0; 3]));
}

/// Hook W30b: looks find their kind by its rules name; a later look of a
/// name wins; none: no entry.
// spec: none (hook W30b: a look names its kind by the rules name)
#[test]
fn tree_looks_find_their_kinds() {
    use crate::extend::TreeLook;
    let kinds = [(1, Some("BirchA")), (2, Some("FirA")), (3, None)];
    assert!(tree_looks_by_kind(kinds, &[]).is_empty());
    let looks = tree_looks_by_kind(
        kinds,
        &[
            TreeLook::new("BirchA", "a.glb"),
            TreeLook::new("BirchA", "b.glb"),
            TreeLook::new("PalmB", "p.glb"),
        ],
    );
    assert_eq!(looks.len(), 1);
    assert_eq!(looks[&1].model, "b.glb");
}

/// The world, lookup, elements and map doodads of [`lying_things`].
type LyingThings = (
    World,
    Lookup,
    Vec<s2_assets::landscape::LandscapeElement>,
    Vec<(u32, (u32, u32))>,
);

/// An element of the landscape table for the hook W30c tests.
fn element_of(
    name: &str,
    shader: &str,
    kind: s2_assets::landscape::ElementType,
    rot: [f32; 2],
    scale: [f32; 2],
    shadow_cast: bool,
) -> s2_assets::landscape::LandscapeElement {
    s2_assets::landscape::LandscapeElement {
        name: name.into(),
        mesh: format!("data/x/{name}.gr2"),
        shader: shader.into(),
        rot,
        scale_x: scale,
        scale_y: scale,
        scale_z: scale,
        offset: [0.0, 0.0],
        kind,
        blend_mode: 0,
        align_on_water: false,
        shadow_cast,
    }
}

/// A world of 8 x 8 flat fields for hook W30c: a stone pile of three
/// stones (kind 20, `StoneRessourceA04`) on (2, 2) and one of two (kind 21,
/// `StoneRessourceA05`, the state after it) on (5, 5), a young grain field
/// (kind 30, `Field01`) on (3, 5), a tree (kind 7, `BirchA`) on (6, 2); the
/// map's doodads: two bushes (type 40, `bush01`) and a rock (type 41,
/// `rock 1`).
fn lying_things() -> LyingThings {
    use s2_assets::landscape::ElementType;
    let n = 64;
    let terrain = Terrain {
        width: 8,
        height: 8,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![0; n],
        samples: Vec::new(),
    };
    let kind = |id, next, grow_ticks, blocking| s2_sim::DepositKind {
        id,
        good: None,
        next,
        elevation: 1,
        grow_ticks,
        blocking,
        life_time: None,
        needs_plantable: false,
        random_offset: false,
    };
    let rules = s2_sim::Rules {
        deposit_kinds: vec![
            kind(7, None, 1001, false),
            kind(20, Some(21), 0, true),
            kind(21, None, 0, true),
            kind(30, None, 1067, true),
        ],
        ..s2_sim::Rules::empty()
    };
    let deposit = |field: (u32, u32), kind, ripe_in| s2_sim::MapDeposit {
        field,
        kind,
        position: s2_sim::grid::field_sample(field),
        ripe_in,
    };
    let setup = s2_sim::GameSetup {
        deposits: vec![
            deposit((2, 2), 20, 0),
            deposit((5, 5), 21, 0),
            deposit((3, 5), 30, 700),
            deposit((6, 2), 7, 0),
        ],
        ..Default::default()
    };
    let world = World::new_game(1, terrain, rules, &setup);
    let mut lookup = Lookup::default();
    for (id, gfx, start, speed) in [
        (7, "tree_01", 0.1, 0.0009),
        (20, "stoneressource_03", 0.1, 0.0009),
        (21, "stoneressource_02", 0.1, 0.0009),
        (30, "Wheatfield", 0.2, 0.00075),
    ] {
        lookup.deposits.insert(
            id,
            DepositLook {
                gfx_name: gfx.into(),
                start_size: Some(start),
                speed: Some(speed),
            },
        );
    }
    lookup.doodads.insert(40, ("Bush01".into(), true));
    lookup.doodads.insert(41, ("rock1".into(), true));
    let elements = vec![
        element_of(
            "tree_01",
            "tree_shader.fx",
            ElementType::Tree,
            [0.0, 360.0],
            [0.8, 1.0],
            true,
        ),
        element_of(
            "stoneressource_03",
            "stone_shader.fx",
            ElementType::Stone,
            [0.0, 0.0],
            [1.0, 1.0],
            true,
        ),
        element_of(
            "stoneressource_02",
            "stone_shader.fx",
            ElementType::Stone,
            [0.0, 0.0],
            [1.0, 1.0],
            true,
        ),
        element_of(
            "Wheatfield",
            "weed_shader.fx",
            ElementType::WheatField,
            [0.0, 360.0],
            [1.0, 1.0],
            false,
        ),
        element_of(
            "Bush01",
            "doodads_shader.fx",
            ElementType::Doodad,
            [0.0, 360.0],
            [0.8, 1.2],
            true,
        ),
        element_of(
            "rock1",
            "rock_shader.fx",
            ElementType::Doodad,
            [0.0, 360.0],
            [0.8, 1.2],
            true,
        ),
    ];
    let doodads = vec![
        (40, s2_sim::grid::field_sample((1, 6))),
        (40, s2_sim::grid::field_sample((6, 6))),
        (41, s2_sim::grid::field_sample((1, 1))),
    ];
    (world, lookup, elements, doodads)
}

/// Hook W30c: a stone pile's state, a grain field and a decoration with a
/// project's look are drawn with its model in the project object family,
/// at the same place, with the same turn, sizes and shadow as the
/// original's (the element's draws, in the same order), a growing field
/// grown as the original's; each state of a pile its own model, the
/// instance's key holding its kind, so a pile that loses a stone is a new
/// instance with the next state's model. A look naming a tree kind leaves
/// the tree as it is; a kind without a look is drawn as before.
// spec: GFX-OBJ-1.3, GFX-OBJ-1.4, GFX-ANIM-3.8b, ECO-OUT-3.4
#[test]
fn a_project_object_takes_the_originals_place() {
    use crate::extend::ObjectLook;
    let (world, mut lookup, elements, doodads) = lying_things();
    let frame = Frame::new(8, 8);
    let scenery = Scenery {
        elements: &elements,
        doodads: &doodads,
        ..Scenery::default()
    };
    let place = |lookup: &Lookup| {
        let mut poses = Poses::with_rng(s2_sim::rng::Rng::new(VIEW_SEED));
        let mut out = objects(
            &world,
            lookup,
            &ModelTables::default(),
            &scenery,
            &frame,
            false,
            &mut poses,
        );
        out.retain(|p| matches!(p.key.0, Kind::Deposit | Kind::Doodad));
        out.sort_by_key(|p| p.key);
        out
    };
    let original = place(&lookup);
    assert_eq!(original.len(), 7);
    // No looks, or looks of other names: the same.
    let deposit_names = [
        (7, Some("BirchA")),
        (20, Some("StoneRessourceA04")),
        (21, Some("StoneRessourceA05")),
        (30, Some("Field01")),
    ];
    let doodad_names = [(40, Some("bush01")), (41, Some("rock 1"))];
    assert!(object_looks_by_kind(deposit_names, &[]).is_empty());
    let others = [ObjectLook::new("shell", "s.glb")];
    lookup.object_deposits = object_looks_by_kind(deposit_names, &others);
    lookup.object_doodads = object_looks_by_kind(doodad_names, &others);
    assert!(lookup.object_deposits.is_empty() && lookup.object_doodads.is_empty());
    assert_eq!(place(&lookup), original);
    // The looks.
    let looks = [
        ObjectLook::new("StoneRessourceA04", "data/objects/stones_3.glb"),
        ObjectLook::new("StoneRessourceA05", "data/objects/stones_2.glb"),
        ObjectLook::new("Field01", "data/objects/grain.glb"),
        ObjectLook::new("bush01", "data/objects/bush.glb"),
        ObjectLook::new("BirchA", "data/objects/not_a_tree.glb"),
    ];
    lookup.object_deposits = object_looks_by_kind(deposit_names, &looks);
    lookup.object_doodads = object_looks_by_kind(doodad_names, &looks);
    let project = place(&lookup);
    assert_eq!(project.len(), original.len());
    let want = |key: InstanceKey| match key {
        (Kind::Deposit, _, 20) => Some("data/objects/stones_3.glb"),
        (Kind::Deposit, _, 21) => Some("data/objects/stones_2.glb"),
        (Kind::Deposit, _, 30) => Some("data/objects/grain.glb"),
        (Kind::Doodad, _, 0 | 1) => Some("data/objects/bush.glb"),
        _ => None,
    };
    for (p, o) in project.iter().zip(&original) {
        assert_eq!(
            (p.key, p.at, p.heading, p.scale, p.shadow, p.tilt),
            (o.key, o.at, o.heading, o.scale, o.shadow, o.tilt)
        );
        match want(p.key) {
            Some(file) => {
                assert_eq!(p.file, file, "{:?}", p.key);
                assert_eq!(p.family, models3d::Family::FacetObject, "{:?}", p.key);
            }
            None => assert_eq!((&p.file, p.family), (&o.file, o.family), "{:?}", p.key),
        }
    }
    // The tree stays the original's tree; the rock its own.
    let tree = project.iter().find(|p| p.key.2 == 7).expect("the tree");
    assert_eq!(tree.family, models3d::Family::Tree);
    // The bushes keep their own random turns and sizes (not one pose for
    // the kind), the field is young and grows as the original's.
    let bushes: Vec<&Placed> = project
        .iter()
        .filter(|p| p.key.0 == Kind::Doodad && p.key.2 < 2)
        .collect();
    assert_ne!(bushes[0].heading, bushes[1].heading);
    let field = project.iter().find(|p| p.key.2 == 30).expect("the field");
    let s = deposit_size(
        &world,
        30,
        world.deposit((3, 5)).expect("the field").state,
        &lookup.deposits[&30],
    );
    assert!(s > 0.0 && s < 1.0, "{s}");
    for k in 0..3 {
        assert!(
            (field.scale[k] - growth_scale(s)[k]).abs() < 1e-5,
            "{:?}",
            field.scale
        );
    }
    // Each state of a pile its model; the key holds the kind.
    let pile = |kind| {
        project
            .iter()
            .find(|p| p.key.0 == Kind::Deposit && p.key.2 == kind)
    };
    let (three, two) = (pile(20).expect("3"), pile(21).expect("2"));
    assert_ne!(three.key, two.key);
    assert_ne!(three.file, two.file);
}

/// Hook W30c: a sign or a decoration takes its type's project look; a type
/// without one keeps its element's model and family.
// spec: none (hook W30c: a doodad type's look by its type)
#[test]
fn a_sign_takes_its_types_look() {
    use s2_assets::landscape::ElementType;
    let sign = element_of(
        "Sign_Coal_S",
        "signs_shader.fx",
        ElementType::Doodad,
        [0.0, 360.0],
        [1.0, 1.0],
        true,
    );
    let mut lookup = Lookup::default();
    assert_eq!(
        object_look(&lookup, 7, &sign),
        (sign.mesh.clone(), models3d::Family::Haze)
    );
    lookup.object_doodads = object_looks_by_kind(
        [(7, Some("Coal (few)")), (8, Some("Coal (much)"))],
        &[crate::extend::ObjectLook::new(
            "Coal (few)",
            "sign_coal_s.glb",
        )],
    );
    assert_eq!(
        object_look(&lookup, 7, &sign),
        ("sign_coal_s.glb".to_owned(), models3d::Family::FacetObject)
    );
    assert_eq!(object_look(&lookup, 8, &sign).1, models3d::Family::Haze);
}

/// Hook W30c: looks find their types by the rules name; a later look of a
/// name wins; two types of one name (the rules have two `Gate01`) both take
/// it; none: no entry.
// spec: none (hook W30c: a look names its kind by the rules name)
#[test]
fn object_looks_find_their_kinds() {
    use crate::extend::ObjectLook;
    let kinds = [
        (1, Some("Gate01")),
        (2, Some("Gate01")),
        (3, Some("bush01")),
        (4, None),
    ];
    assert!(object_looks_by_kind(kinds, &[]).is_empty());
    let looks = object_looks_by_kind(
        kinds,
        &[
            ObjectLook::new("Gate01", "a.glb"),
            ObjectLook::new("Gate01", "b.glb"),
            ObjectLook::new("Bush01", "graphics name, not a rules name.glb"),
        ],
    );
    assert_eq!(looks.len(), 2);
    assert_eq!(looks[&1].model, "b.glb");
    assert_eq!(looks[&2].model, "b.glb");
}
