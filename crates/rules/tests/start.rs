//! Real-map test cases of `specs/ECO-warehouse-construction.md` (game start,
//! territory, exploration), run on the player's own install through
//! `S2_GAME_DIR`. When it is unset each test prints a note and passes without
//! checking anything.

use s2_assets::s2m;
use s2_assets::{GameDir, GameDirError};
use s2_rules::RuleSet;
use s2_sim::grid::{self, Point};
use s2_sim::{PlayerId, World};

const T01: &str = "data/maps/tutorialMaps/tutorial01.s2m";
const FIGHT: &str = "data/maps/freeGameMaps/MP_4P_Fight.s2m";

fn game_dir() -> Option<GameDir> {
    match GameDir::from_env() {
        Ok(dir) => Some(dir),
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

/// A new game on a map of the install, with its rules.
fn game(rel: &str, start_resources: u8) -> Option<(World, RuleSet, s2m::Map)> {
    let dir = game_dir()?;
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let data = std::fs::read(dir.root().join(rel)).unwrap();
    let map = s2m::load(rel, &data).unwrap_or_else(|e| panic!("{rel}: {e}"));
    let world = s2_rules::new_game(1, &map, &rules, start_resources).unwrap();
    Some((world, rules, map))
}

fn player_at(map: &s2m::Map, at: Point) -> PlayerId {
    let i = map
        .info
        .start_positions
        .iter()
        .position(|p| (p.x, p.y) == at)
        .unwrap_or_else(|| panic!("no start position {at:?}"));
    PlayerId(i as u8)
}

fn count(world: &World, rules: &RuleSet, at: Point, name: &str) -> u32 {
    let good = rules.id_named(name).unwrap();
    world
        .stock(at)
        .unwrap()
        .iter()
        .filter(|&&(g, _)| g == good)
        .map(|&(_, n)| n)
        .sum()
}

// ECO-HQ T1
// spec: ECO-HQ-2.1 (T1)
#[test]
fn t1_start_stock() {
    let Some((world, rules, _)) = game(T01, 0) else {
        return;
    };
    let at = (45, 38);
    let hq = world.building_at(at).expect("a headquarters");
    assert_eq!(Some(hq.building_type), rules.id_named("castle"));
    assert!(!hq.under_construction);
    let stock = world.stock(at).unwrap();
    assert_eq!(stock.len(), s2_rules::START_STOCK.len());
    for (&(good, n), &(name, base)) in stock.iter().zip(&s2_rules::START_STOCK) {
        assert_eq!((good, n), (rules.id_named(name).unwrap(), base), "{name}");
    }
    assert_eq!(count(&world, &rules, at, "plank"), 22);
    assert_eq!(count(&world, &rules, at, "stone"), 34);
    assert_eq!(count(&world, &rules, at, "good_helper"), 26);
    assert_eq!(count(&world, &rules, at, "good_constructor"), 5);
    assert_eq!(count(&world, &rules, at, "good_soldier0"), 23);
}

// ECO-HQ T15: the donkeys are settlers.
// spec: ECO-HQ-2.1, ECO-CAT-3.4 (T15)
#[test]
fn t15_donkeys_are_settlers() {
    let Some((world, rules, _)) = game(T01, 0) else {
        return;
    };
    assert_eq!(count(&world, &rules, (45, 38), "good_donkey"), 4);
    assert_eq!(count(&world, &rules, (45, 38), "donkey"), 0);
}

// ECO-HQ T2
// spec: ECO-HQ-2.2 (T2)
#[test]
fn t2_start_resources_setting_2() {
    let Some((world, rules, _)) = game(T01, 2) else {
        return;
    };
    let at = (45, 38);
    assert_eq!(count(&world, &rules, at, "plank"), 88);
    assert_eq!(count(&world, &rules, at, "stone"), 136);
    assert_eq!(count(&world, &rules, at, "good_helper"), 104);
}

// ECO-HQ T3, T4
// spec: ECO-HQ-3.1, ECO-HQ-3.3 (T3, T4)
#[test]
fn t3_t4_territory_and_exploration_on_mp_4p_fight() {
    let Some((world, _, map)) = game(FIGHT, 0) else {
        return;
    };
    let at = (27, 57);
    let player = player_at(&map, at);
    assert!(world.building_at(at).is_some());
    let (width, height) = (map.info.width, map.info.height);
    for field in world.terrain().fields() {
        let near = grid::distance(at, field);
        assert_eq!(
            world.field_owner(field) == Some(player),
            near <= 9,
            "owned {field:?}"
        );
        let edge = field.0 == 0 || field.1 == 0 || field.0 == width - 1 || field.1 == height - 1;
        assert_eq!(
            world.is_explored(player, field),
            near <= 12 && !edge,
            "explored {field:?}"
        );
    }
    assert_eq!(world.field_owner((36, 57)), Some(player));
    assert_eq!(world.field_owner((37, 57)), None);
}

// ECO-HQ T5
// spec: ECO-HQ-3.1 (T5)
#[test]
fn t5_owned_field_count_on_tutorial01() {
    let Some((world, _, map)) = game(T01, 0) else {
        return;
    };
    let player = player_at(&map, (45, 38));
    assert_eq!(world.owned_field_count(player), 271);
}

/// ECO-REQ on a real map: a woodcutter hut next to the headquarters of
/// `tutorial01`, connected by a road, is built with the HQ's builder and
/// planks, and its woodcutter moves in.
// spec: ECO-REQ-1.3, ECO-REQ-1.4, ECO-HQ-4.4
#[test]
fn woodcutter_hut_from_a_fresh_game() {
    use s2_sim::{Command, Placement, SettlerJob};
    let Some((mut world, rules, map)) = game(T01, 0) else {
        return;
    };
    let hq = (45, 38);
    let player = player_at(&map, hq);
    let hut_type = rules.id_named("woodCutter").unwrap();
    let hq_flag = world.terrain().neighbour(hq, 1).unwrap();
    // The first field 3 to 5 steps from the HQ that takes a small building.
    let hut = (3..=5)
        .flat_map(|k| grid::ring(hq, k, map.info.width, map.info.height))
        .find(|&f| {
            world.placement_at(f) >= Placement::Small && world.placement_at(f) != Placement::Mine
        })
        .unwrap();
    let hut_flag = world.terrain().neighbour(hut, 1).unwrap();
    for command in [
        Command::PlaceBuilding {
            player,
            building: hut_type,
            at: hut,
        },
        Command::BuildRoad {
            player,
            from: hq_flag,
            to: hut_flag,
        },
    ] {
        world.submit(command);
        world.step();
        assert_eq!(world.last_results()[0].outcome, Ok(()), "{hut:?}");
    }
    let planks = count(&world, &rules, hq, "plank");
    let mut moved_in = false;
    for _ in 0..6000 {
        world.step();
        moved_in = world
            .settlers()
            .any(|(_, s)| matches!(s.job, SettlerJob::Worker { .. }));
        if moved_in {
            break;
        }
    }
    assert!(moved_in, "tick {}", world.tick());
    assert!(!world.building_at(hut).unwrap().under_construction);
    let frame_and_main = 2;
    assert_eq!(count(&world, &rules, hq, "plank"), planks - frame_and_main);
    assert_eq!(count(&world, &rules, hq, "good_woodcutter"), 3);
}

// ECO-OUT-6.2: from 0.1 at 0.0009 per tick in 32-bit floats: 1001 ticks.
#[test]
fn trees_grow_in_1001_ticks() {
    assert_eq!(s2_rules::grow_ticks(0.1, 0.0009), 1001);
    let Some((world, rules, _)) = game(T01, 0) else {
        return;
    };
    let birch = rules.deposit_named("BirchA").unwrap();
    assert_eq!(world.deposit_kind(birch).unwrap().grow_ticks, 1001);
    // The map's trees are ripe from the start (ECO-OUT-6.1).
    let ripe = world.deposits().filter(|(_, d)| d.ripe(0)).count();
    assert!(ripe > 100, "{ripe}");
}

/// The first production chain on `tutorial01` from a fresh game: a
/// woodcutter hut next to trees and a sawmill, both connected to the
/// headquarters, give planks.
// spec: ECO-CAT-7.1, ECO-OUT-3.6, ECO-WORK-3.2
#[test]
fn planks_from_a_fresh_game() {
    use s2_sim::{Command, Placement, Rejection};
    let Some((mut world, rules, map)) = game(T01, 0) else {
        return;
    };
    let hq = (45, 38);
    let player = player_at(&map, hq);
    let (width, height) = (map.info.width, map.info.height);
    let hq_flag = world.terrain().neighbour(hq, 1).unwrap();
    let wood = rules.id_named("wood").unwrap();
    let plank = rules.id_named("plank").unwrap();
    let apply = |world: &mut World, command: Command| -> Result<(), Rejection> {
        world.submit(command);
        world.step();
        world.last_results()[0].outcome
    };
    // Place `kind` on the first field 3 to 7 from the HQ that takes it,
    // passes `good`, and can be connected to the HQ flag.
    let place =
        |world: &mut World, kind: &str, min: Placement, good: &dyn Fn(&World, Point) -> bool| {
            let id = rules.id_named(kind).unwrap();
            for f in (3..=7).flat_map(|k| grid::ring(hq, k, width, height)) {
                let class = world.placement_at(f);
                if class < min || class == Placement::Mine || !good(world, f) {
                    continue;
                }
                let mut trial = world.clone();
                let flag = trial.terrain().neighbour(f, 1).unwrap();
                let placed = apply(
                    &mut trial,
                    Command::PlaceBuilding {
                        player,
                        building: id,
                        at: f,
                    },
                );
                let road = apply(
                    &mut trial,
                    Command::BuildRoad {
                        player,
                        from: hq_flag,
                        to: flag,
                    },
                );
                if placed.is_ok() && road.is_ok() {
                    *world = trial;
                    return f;
                }
            }
            panic!("no place for {kind}");
        };
    let near_trees = |world: &World, f: Point| {
        world
            .deposits()
            .filter(|(d, k)| grid::distance(*d, f) <= 4 && k.ripe(0))
            .filter(|(_, k)| world.deposit_kind(k.kind).and_then(|k| k.good) == Some(wood))
            .count()
            >= 3
    };
    let hut = place(&mut world, "woodCutter", Placement::Small, &near_trees);
    let sawmill = place(&mut world, "sawMill", Placement::Medium, &|_, _| true);
    let sawmill_flag = world.terrain().neighbour(sawmill, 1).unwrap();
    let mut first_plank = None;
    for _ in 0..20_000 {
        world.step();
        let made = world.wares().any(|(_, w)| {
            w.good == plank && w.location == s2_sim::WareLocation::Flag(sawmill_flag)
        });
        if made && first_plank.is_none() {
            first_plank = Some(world.tick());
            break;
        }
    }
    let tick = first_plank.unwrap_or_else(|| {
        panic!(
            "no plank: hut {hut:?} done {}, sawmill {sawmill:?} done {}, inputs {:?}",
            world.site(hut).is_none(),
            world.site(sawmill).is_none(),
            world.inputs(sawmill)
        )
    });
    eprintln!(
        "first plank at the sawmill's flag in tick {tick} ({} s)",
        tick / 10
    );
}

/// ECO-HQ T10a (ECO-HQ-1.2a, Q-118; corrected by ECO-HQ-4.8a): on
/// `MP_4P_Fight` the headquarters at (27,57), height 492, first takes the
/// integer mean of the 19 fields within 2 of it, then levels its six
/// neighbours, flag included, to that value before the first tick; fields
/// two away are unchanged.
// spec: ECO-HQ-1.2, ECO-HQ-4.8a (T10a)
#[test]
fn t10a_headquarters_levelled_at_creation() {
    let Some((world, _, map)) = game(FIGHT, 0) else {
        return;
    };
    let at = (27, 57);
    let original = map.terrain().heights;
    let terrain = world.terrain();
    let index = |f: Point| terrain.index(f);
    assert_eq!(original[index(at)], 492);
    let near = grid::area(at, 2, terrain.width, terrain.height);
    let sum: i64 = near
        .iter()
        .map(|&f| i64::from(original[index(f)].max(map.terrain().height_floor)))
        .sum();
    let mean = (sum / near.len() as i64) as i32;
    assert_eq!(terrain.height_at(at), mean);
    let before = [
        ((28, 57), 888),
        ((28, 56), 1257),
        ((27, 56), 589),
        ((26, 57), 300),
        ((27, 58), 229),
        ((28, 58), 295),
    ];
    for (field, was) in before {
        assert_eq!(original[index(field)], was, "{field:?} before");
        assert_eq!(terrain.height_at(field), mean, "{field:?} after");
    }
    for field in terrain.fields() {
        if grid::distance(at, field) == 2 {
            assert_eq!(
                terrain.height_at(field),
                original[index(field)],
                "{field:?}"
            );
        }
    }
}

/// ECO-HQ T-level-6 (ECO-HQ-4.8a at the start): on MP_2P_Storm_Coast the
/// castles on (29, 51) and (139, 51) (7682 each in the map) and all six
/// neighbours of each are at 7205, the integer mean of the 19 fields within
/// 2 of each castle in the map (7205.6): the map's (139, 50) 7306 -> 7205,
/// (140, 50) 6997 -> 7205; 169 samples per castle changed, up to 7 lattice
/// steps from its sample, none beyond.
// spec: ECO-HQ-4.8a (T-level-6), ECO-HQ-1.2
#[test]
fn t_level_6_castles_at_their_mean() {
    let Some(dir) = game_dir() else {
        return;
    };
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let data = std::fs::read(dir.root().join(rel)).unwrap();
    let map = s2m::load(rel, &data).unwrap();
    let world = s2_rules::new_game(u64::from(s2_sim::rng::SIM_SEED), &map, &rules, 1).unwrap();
    let before = map.terrain();
    let terrain = world.terrain();
    assert_eq!(before.height_at((139, 50)), 7306);
    assert_eq!(before.height_at((140, 50)), 6997);
    let castles = [(29, 51), (139, 51)];
    for castle in castles {
        assert_eq!(before.height_at(castle), 7682);
        assert_eq!(terrain.height_at(castle), 7205, "{castle:?}");
        for n in terrain.neighbours(castle).into_iter().flatten() {
            assert_eq!(terrain.height_at(n), 7205, "{n:?}");
        }
    }
    // The samples changed: within 7 lattice steps of a castle's sample.
    let (columns, rows) = terrain.sample_size();
    let centres: Vec<Point> = castles.iter().map(|&c| grid::field_sample(c)).collect();
    let mut changed = [0usize; 2];
    for y in 0..rows {
        for x in 0..columns {
            let i = (y * columns + x) as usize;
            if terrain.samples[i] == before.samples[i] {
                continue;
            }
            let near: Vec<u32> = centres.iter().map(|&c| grid::distance(c, (x, y))).collect();
            let k = if near[0] <= near[1] { 0 } else { 1 };
            assert!(
                near[k] <= 7,
                "sample {:?} changed, {} from a castle",
                (x, y),
                near[k]
            );
            changed[k] += 1;
        }
    }
    assert_eq!(changed, [169, 169]);
}

/// TRN-FREE T1 to T6a on `MP_4P_Fight` with the headquarters at (27,57) as
/// at game start (ground levelled, ECO-HQ-1.2a), the walker on its flag
/// (28,56). Ties are drawn at random (TRN-FREE-3.2), so every case is run
/// with many seeds.
#[test]
fn trn_free_routes_on_mp_4p_fight() {
    use s2_sim::movement::{free_walk, walk_ticks};
    use s2_sim::Rng;
    let Some((world, _, map)) = game(FIGHT, 0) else {
        return;
    };
    let from = (28, 56);
    let speed = 45_000;
    let walks = |terrain: &s2_sim::map::Terrain, to: Point| {
        (1..40u64)
            .map(|seed| {
                let r = free_walk(terrain, &mut Rng::new(seed as u32), from, to).unwrap();
                let ticks = walk_ticks(terrain, speed, from, &r.fields);
                (r.cost, r.fields, ticks)
            })
            .collect::<Vec<_>>()
    };
    let terrain = world.terrain();
    // T1: one route.
    for (cost, fields, ticks) in walks(terrain, (33, 60)) {
        assert_eq!(cost, 26);
        assert_eq!(
            fields,
            [
                (29, 56),
                (29, 57),
                (30, 58),
                (31, 58),
                (32, 58),
                (32, 59),
                (33, 60)
            ]
        );
        assert_eq!(ticks, 195);
    }
    // T2
    for (cost, fields, ticks) in walks(terrain, (24, 62)) {
        assert_eq!((cost, fields.len(), ticks), (30, 8, 219));
        assert_eq!(
            fields[..7],
            [
                (27, 56),
                (26, 56),
                (25, 57),
                (25, 58),
                (24, 59),
                (24, 60),
                (23, 61)
            ]
        );
    }
    // T3
    for (cost, fields, ticks) in walks(terrain, (36, 58)) {
        assert_eq!((cost, fields.len(), ticks), (39, 9, 260));
    }
    // T4 (TRN-FREE-3.2a): one route for every starting draw.
    let t4 = walks(terrain, (28, 48));
    let routes: std::collections::BTreeSet<_> = t4.iter().map(|(_, f, _)| f.clone()).collect();
    for (cost, fields, ticks) in &t4 {
        assert_eq!((*cost, fields.len(), *ticks), (33, 8, 278));
    }
    assert_eq!(routes.len(), 1, "{routes:?}");
    // T5: to the tree at (30,62), stopping short 1: 5 steps, 112 ticks, one
    // route.
    assert!(world.deposit((30, 62)).is_some(), "the map's tree");
    let t5 = walks(terrain, (30, 62));
    let routes: std::collections::BTreeSet<_> = t5.iter().map(|(_, f, _)| f.clone()).collect();
    for (cost, fields, _) in &t5 {
        assert_eq!(*cost, 20);
        let short = &fields[..fields.len() - 1];
        assert_eq!(short.len(), 5);
        assert_eq!(walk_ticks(terrain, speed, from, short), 112);
    }
    assert_eq!(routes.len(), 1, "{routes:?}");
    // T6: 7 steps, 230 ticks; stopping short 1: 6 steps, 202 ticks.
    for (cost, fields, ticks) in walks(terrain, (34, 57)) {
        assert_eq!((cost, fields.len(), ticks), (33, 7, 230));
        let short = &fields[..6];
        assert_eq!(walk_ticks(terrain, speed, from, short), 202);
    }
    // T6a: on the map's own heights, without the levelling: not the game's
    // values, a check of the levelling's effect.
    let mut raw = terrain.clone();
    raw.heights = map.terrain().heights;
    for (to, label, expected) in [
        ((33, 60), 24, 177),
        ((36, 58), 37, 242),
        ((34, 57), 31, 212),
    ] {
        for (cost, _, ticks) in walks(&raw, to) {
            assert_eq!((cost, ticks), (label, expected), "{to:?}");
        }
    }
}

/// TRN-FREE T15 (TRN-FREE-5.5a, 5.5b): the construction positions of a
/// woodcutter hut are its flag field's sample plus the owner's tribe's
/// offsets, whole samples, no odd-row correction.
#[test]
fn t15_construction_positions() {
    use s2_sim::{Command, Placement};
    let Some((mut world, rules, map)) = game(T01, 0) else {
        return;
    };
    let hq = (45, 38);
    let player = player_at(&map, hq);
    let hut_type = rules.id_named("woodCutter").unwrap();
    let hut = (3..=5)
        .flat_map(|k| grid::ring(hq, k, map.info.width, map.info.height))
        .find(|&f| {
            world.placement_at(f) >= Placement::Small && world.placement_at(f) != Placement::Mine
        })
        .unwrap();
    world.submit(Command::PlaceBuilding {
        player,
        building: hut_type,
        at: hut,
    });
    world.step();
    assert_eq!(world.last_results()[0].outcome, Ok(()));
    let flag = world.terrain().neighbour(hut, 1).unwrap();
    let (c, r) = grid::field_sample(flag);
    let at = |offsets: [(i32, i32); 4]| {
        offsets.map(|(x, y)| ((c as i32 + x) as u32, (r as i32 + y) as u32))
    };
    // Romans (the default, Q-051), then Africans.
    assert_eq!(
        world.construction_samples(hut),
        at([(-1, 2), (-3, 1), (-2, 1), (-1, 3)])
    );
    world.set_tribe(player, 1);
    assert_eq!(
        world.construction_samples(hut),
        at([(-3, 1), (-2, 3), (-1, 3), (-2, 3)])
    );
}

// SIM-START T1: the default free game on MP_2P_Storm_Coast after the first
// update: two finished castles, two flags, one private per castle standing
// on the castle's door field, nothing else; nothing drawn yet (T2).
// spec: SIM-START-2.1 (T1), SIM-START-2.2 (T2), MIL-SOL-6.5 (T16), SIM-START-3.1
#[test]
fn start_t1_storm_coast_after_the_first_update() {
    let Some((mut world, _, _)) = game("data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m", 1) else {
        return;
    };
    world.step();
    let mut castles: Vec<(PlayerId, Point, bool)> = world
        .buildings()
        .map(|b| (b.owner, b.at, b.under_construction))
        .collect();
    castles.sort();
    assert_eq!(
        castles,
        vec![
            (PlayerId(0), (29, 51), false),
            (PlayerId(1), (139, 51), false)
        ]
    );
    let flags: Vec<Point> = world.flags().map(|(at, _)| at).collect();
    assert_eq!(flags.len(), 2);
    assert!(flags.contains(&(30, 50)) && flags.contains(&(140, 50)));
    let mut soldiers: Vec<(Point, (i64, i64))> = world
        .trace_settlers()
        .iter()
        .map(|s| (s.field, s.fine))
        .collect();
    soldiers.sort();
    assert_eq!(
        soldiers,
        vec![
            ((29, 52), (116_000, 208_000)),
            ((139, 52), (556_000, 208_000))
        ]
    );
    assert_eq!(world.roads().count(), 0);
    assert_eq!(world.rng().draws(), 0);
}

/// The default game on MP_2P_Storm_Coast with the game's own seed
/// (SIM-RNG-2.1) and no computer player, for draw counts.
fn storm_coast_with_game_seed() -> Option<World> {
    let dir = game_dir()?;
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let data = std::fs::read(dir.root().join(rel)).unwrap();
    let map = s2m::load(rel, &data).unwrap();
    Some(s2_rules::new_game(u64::from(s2_sim::rng::SIM_SEED), &map, &rules, 1).unwrap())
}

// ECO-FOOD T31, T33 and SIM-RNG T12: the map's animals start with their
// records' counters, so the number of animals that choose in each counter
// tick 0 .. 40 is the original's; and with the walks drawing as TRN-FREE-3.2a
// the draws per tick equal the original's totals (SIM-RNG-2.5).
// spec: ECO-FOOD-5.13 (T31), ECO-FOOD-5.14 (T32, T33), SIM-RNG-2.4a (T12), TRN-FREE-3.2a
#[test]
fn food_t31_animals_choose_and_draw_as_the_original() {
    let Some(mut world) = storm_coast_with_game_seed() else {
        return;
    };
    const CHOOSE: [u32; 41] = [
        0, 0, 2, 1, 1, 3, 2, 2, 2, 2, 3, 2, 2, 1, 3, 2, 3, 5, 2, 3, 0, 3, 5, 3, 0, 1, 4, 3, 6, 0,
        2, 3, 3, 0, 0, 3, 2, 2, 5, 2, 0,
    ];
    const DRAWS: [u64; 41] = [
        0, 0, 3, 1, 1, 7, 6, 8, 7, 4, 5, 8, 8, 3, 9, 7, 12, 17, 8, 11, 0, 11, 19, 6, 0, 4, 12, 6,
        19, 0, 4, 11, 11, 0, 0, 6, 10, 8, 14, 6, 0,
    ];
    assert_eq!(world.animals().count(), 261);
    let chosen = |w: &World| -> Vec<bool> {
        w.animals()
            .map(|(_, a)| {
                matches!(
                    a.state,
                    s2_sim::AnimalState::Walking { .. }
                        | s2_sim::AnimalState::Choosing { ticket: 100 }
                )
            })
            .collect()
    };
    let (mut choose, mut draws) = (Vec::new(), Vec::new());
    for _ in 0..41 {
        let before = (chosen(&world), world.rng().draws());
        world.step();
        let after = chosen(&world);
        choose.push(
            before
                .0
                .iter()
                .zip(&after)
                .filter(|(was, is)| !**was && **is)
                .count() as u32,
        );
        draws.push(world.rng().draws() - before.1);
    }
    assert_eq!(choose, CHOOSE, "animals choosing per counter tick");
    assert_eq!(draws, DRAWS, "draws per counter tick");
}

// ECO-FOOD T65 (5.14a, E-eco-058): sp02_the_river's walking animals hold
// the start clock 0.0 in their records, so at a new game their walks count
// from the game's start, the clock's value then.
// spec: ECO-FOOD-5.14a (ECO-FOOD T65), FMT-S2M-5.4
#[test]
fn food_t65_the_rivers_walking_animals_start_at_zero() {
    let Some((world, _, map)) = game("data/maps/freeGameMaps/sp02_the_river.s2m", 1) else {
        return;
    };
    let walking: Vec<_> = map.animals.iter().filter(|a| a.counters[0] == 2).collect();
    assert!(!walking.is_empty(), "walking animals in the map");
    assert!(walking.iter().all(|a| a.walk_start == 0.0));
    let walks = world
        .animals()
        .filter(|(_, a)| matches!(a.state, s2_sim::AnimalState::Walking { .. }))
        .inspect(|(_, a)| {
            assert!(matches!(
                a.state,
                s2_sim::AnimalState::Walking { since: 0, .. }
            ))
        })
        .count();
    assert_eq!(walks, walking.len());
}

// FMT-S2M-4.9: a new game keeps the map file's continent index of every
// field as loaded (-1 none), not one recomputed from the field states: on
// MP_2P_Storm_Coast the mountains west of the castle are -1 and a recomputed
// layer would make them land (AI-CON-6.3, T34).
// spec: FMT-S2M-4.8, FMT-S2M-4.9
#[test]
fn a_new_game_keeps_the_maps_stored_continents() {
    let Some((world, _, map)) = game("data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m", 1) else {
        return;
    };
    let mut none = 0;
    for f in world.terrain().fields() {
        let stored = map.continent_of[world.terrain().index(f)];
        assert_eq!(world.continent_of(f), Some(stored), "{f:?}");
        none += usize::from(stored < 0);
    }
    assert!(none > 0);
    // A field of the mountains west of the castle (139, 51): none, though
    // it is not water.
    let mountain = world
        .terrain()
        .fields()
        .find(|&f| {
            f.1 == 51
                && f.0 < 139
                && world.continent_of(f) == Some(-1)
                && !world.terrain().has(f, s2_sim::map::state::WATER)
        })
        .expect("a -1 land field west of the castle");
    assert_ne!(world.region_of(mountain), 0);
}

/// UI-GAME-8.5e's table: which counter each building type's first good adds to, by
/// the good or its group. Farm, mill, piggery, donkey breeding and the forester add to
/// none.
// spec: UI-GAME-8.5e
#[test]
fn production_counters_of_the_buildings() {
    let Ok(dir) = s2_assets::GameDir::from_env() else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let sim = rules.sim_rules().unwrap();
    let counter = |name: &str| {
        let id = rules
            .buildings
            .iter()
            .find(|b| b.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name}"))
            .id;
        let kind = sim.building_types.iter().find(|b| b.id == id).unwrap();
        kind.product
            .and_then(|good| sim.goods.get(&good))
            .and_then(|good| good.production_counter)
    };
    for (name, want) in [
        ("WoodcutterHut", Some(0)),
        ("Sawmill", Some(1)),
        ("Stone Pit", Some(2)),
        ("Stone Mine", Some(2)),
        ("FisherHut", Some(3)),
        ("HuntingLodge", Some(3)),
        ("Slaughter House", Some(3)),
        ("Bakery", Some(3)),
        ("Spring", Some(4)),
        ("Brewery", Some(5)),
        ("Coal Mine", Some(6)),
        ("Iron Mine", Some(7)),
        ("Gold Mine", Some(8)),
        ("Smeltery", Some(9)),
        ("Mint", Some(10)),
        ("Locksmithery", Some(11)),
        ("Smithy", Some(12)),
        ("Ship Yard", Some(13)),
        ("Farm", None),
        ("Mill", None),
        ("Piggerey", None),
        ("Donkey Breeding", None),
        ("ForesterHouse", None),
    ] {
        assert_eq!(counter(name), want, "{name}");
    }
    for name in s2_rules::PRODUCTION_COUNTERS {
        assert!(rules.id_named(name).is_some(), "{name}");
    }
}
