//! Real-map test cases of `specs/ECO-building-placement.md` and
//! `specs/TRN-roads-flags.md`, run on the player's own install through
//! `S2_GAME_DIR`. When it is unset each test prints a note and passes without
//! checking anything. As in the specs, player 0 owns every field.

use s2_assets::s2m;
use s2_assets::{GameDir, GameDirError};
use s2_rules::RuleSet;
use s2_sim::grid::Point;
use s2_sim::{BuildingSize, Command, Placement, PlayerId, Rejection, World};

const P0: PlayerId = PlayerId(0);
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

fn load_map(dir: &GameDir, rel: &str) -> s2m::Map {
    let data = std::fs::read(dir.root().join(rel)).unwrap();
    s2m::load(rel, &data).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// An empty world on a map of the install, with the rules of the install.
fn world(rel: &str) -> Option<(World, RuleSet)> {
    let dir = game_dir()?;
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let map = load_map(&dir, rel);
    Some((s2_rules::new_world(1, &map, &rules, P0).unwrap(), rules))
}

fn castle(rules: &RuleSet) -> u32 {
    rules.building_named("Castle").unwrap().id
}

/// Apply commands in one step and return their outcomes.
fn apply(world: &mut World, commands: Vec<Command>) -> Vec<Result<(), Rejection>> {
    for command in commands {
        world.submit(command);
    }
    world.step();
    world.last_results().iter().map(|r| r.outcome).collect()
}

fn headquarters(world: &mut World, rules: &RuleSet, at: &[Point]) {
    let castle = castle(rules);
    let commands = at
        .iter()
        .map(|&at| Command::PlaceBuilding {
            player: P0,
            building: castle,
            at,
        })
        .collect();
    for outcome in apply(world, commands) {
        assert_eq!(outcome, Ok(()));
    }
}

fn classes(world: &World, fields: &[Point]) -> Vec<u8> {
    fields
        .iter()
        .map(|&f| world.placement_at(f) as u8)
        .collect()
}

// ECO T1
// spec: ECO-PLACE-3.9, ECO-HQ-1.1 (ECO T1)
#[test]
fn eco_t01_every_start_position_fits_large() {
    let Some(dir) = game_dir() else { return };
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let mut maps = Vec::new();
    let mut pending = vec![dir.root().join("data/maps")];
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(path).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "s2m") {
                maps.push(path);
            }
        }
    }
    assert_eq!(maps.len(), 34);
    let mut starts = 0;
    for path in maps {
        let map = s2m::load(path.to_str().unwrap(), &std::fs::read(&path).unwrap()).unwrap();
        let world = s2_rules::new_world(1, &map, &rules, P0).unwrap();
        for start in &map.info.start_positions {
            let class = world.placement_at((start.x, start.y));
            assert_eq!(class, Placement::Large, "{}: {start:?}", map.info.name);
            starts += 1;
        }
    }
    assert_eq!(starts, 97);
}

// ECO T2, T3, T4
// spec: ECO-PLACE-3.1, ECO-PLACE-3.4, ECO-PLACE-3.10, ECO-PLACE-4.1 (ECO T2, T3, T4)
#[test]
fn eco_t02_t04_tutorial_classes() {
    let Some((world, _)) = world(T01) else { return };
    assert_eq!(classes(&world, &[(45, 38), (44, 38), (43, 38)]), [4, 4, 4]);
    assert_eq!(
        classes(&world, &[(47, 38), (46, 40), (49, 36), (42, 41)]),
        [3, 3, 3, 3]
    );
    assert_eq!(
        classes(&world, &[(51, 44), (50, 44), (40, 40), (39, 34), (38, 32)]),
        [5, 1, 1, 0, 4]
    );
}

// ECO T5 to T8
// spec: ECO-PLACE-5.1, ECO-HQ-3.4 (ECO T5 to T8)
#[test]
fn eco_t05_t08_tutorial_headquarters() {
    let Some((mut world, rules)) = world(T01) else {
        return;
    };
    headquarters(&mut world, &rules, &[(45, 38)]);
    assert_eq!(world.flag_at((45, 37)), Some(P0));
    // ECO-REQ-4.1: the castle's construction time is negative, so it is
    // finished at once.
    assert!(!world.building_at((45, 38)).unwrap().under_construction);
    assert_eq!(classes(&world, &[(45, 38), (44, 38)]), [0, 0]);
    assert_eq!(classes(&world, &[(43, 38)]), [3]);
    assert_eq!(
        classes(&world, &[(47, 38), (46, 40), (49, 36), (51, 44), (50, 44)]),
        [3, 3, 3, 5, 1]
    );
}

// ECO T9, T10, T11
// spec: ECO-PLACE-3.5, ECO-PLACE-3.8, ECO-PLACE-4.1 (ECO T9, T10, T11)
#[test]
fn eco_t09_t11_fight_classes() {
    let Some((world, _)) = world(FIGHT) else {
        return;
    };
    assert_eq!(classes(&world, &[(40, 12)]), [6]);
    assert_eq!(
        classes(
            &world,
            &[(9, 9), (68, 9), (9, 10), (3, 11), (10, 6), (0, 0)]
        ),
        [2, 3, 4, 5, 1, 0]
    );
    let fits: Vec<bool> = [
        BuildingSize::Small,
        BuildingSize::Medium,
        BuildingSize::Large,
        BuildingSize::Mine,
        BuildingSize::Harbor,
    ]
    .into_iter()
    .map(|size| world.check_building(P0, (9, 9), size).is_ok())
    .collect();
    assert_eq!(fits, [true, false, false, false, false]);
}

// ECO T9: the harbour's footprint.
// spec: ECO-PLACE-3.6, ECO-PLACE-3.8 (ECO T9)
#[test]
fn eco_t09_harbor_footprint() {
    let Some((mut world, rules)) = world(FIGHT) else {
        return;
    };
    let harbor = rules.building_named("Harbor").unwrap().id;
    let outcome = apply(
        &mut world,
        vec![Command::PlaceBuilding {
            player: P0,
            building: harbor,
            at: (40, 12),
        }],
    );
    assert_eq!(outcome, [Ok(())]);
    // The footprint fields are no longer general sites; their neighbours
    // outside it still take flags where nothing else stops them.
    for field in [(40, 12), (39, 12), (39, 13), (40, 13)] {
        assert_eq!(
            world.check_flag(P0, field),
            Err(Rejection::NotGeneralSite),
            "{field:?}"
        );
    }
}

// ECO T12
// spec: ECO-PLACE-1.4, ECO-HQ-1.1 (ECO T12)
#[test]
fn eco_t12_fight_with_four_headquarters() {
    let Some((mut world, rules)) = world(FIGHT) else {
        return;
    };
    headquarters(
        &mut world,
        &rules,
        &[(27, 57), (52, 56), (52, 24), (26, 24)],
    );
    let expected = [
        "2 4 4 0 3 1 5 5 5 5 5 1 5",
        "3 4 1 4 4 5 5 5 5 5 5 0 5",
        "1 3 4 4 4 1 5 5 5 5 5 1 5",
        "3 4 4 4 4 1 0 0 0 5 5 5 1",
        "1 3 3 3 3 3 0 0 0 5 0 0 0",
        "1 0 3 0 3 0 0 0 1 0 1 1 1",
        "5 1 4 1 4 1 0 0 0 1 1 1 1",
        "0 3 4 4 4 1 1 1 4 4 4 4 1",
        "1 3 4 4 4 4 3 3 4 4 4 4 4",
        "0 3 4 4 4 4 0 3 3 3 4 4 4",
        "0 3 4 4 4 4 1 3 3 0 3 4 4",
    ];
    for (row, line) in expected.iter().enumerate() {
        let y = 52 + row as u32;
        let fields: Vec<Point> = (21..=33).map(|x| (x, y)).collect();
        let got: Vec<String> = classes(&world, &fields)
            .iter()
            .map(|c| c.to_string())
            .collect();
        assert_eq!(got.join(" "), *line, "row y={y}");
    }
}

// ECO T13
// spec: ECO-PLACE-4.1 (ECO T13)
#[test]
fn eco_t13_class_counts() {
    for (rel, expected) in [
        (T01, [4065, 989, 136, 311, 825, 74, 0]),
        (FIGHT, [4072, 717, 166, 312, 653, 451, 29]),
    ] {
        let Some((world, _)) = world(rel) else { return };
        let mut counts = [0usize; 7];
        for field in world.terrain().fields() {
            counts[world.placement_at(field) as usize] += 1;
        }
        assert_eq!(counts, expected, "{rel}");
    }
}

/// tutorial01 with the headquarters at the start, as the road tests assume.
fn tutorial_with_headquarters() -> Option<World> {
    let (mut world, rules) = world(T01)?;
    headquarters(&mut world, &rules, &[(45, 38)]);
    Some(world)
}

// TRN T1, T2
// spec: TRN-ROAD-2.1, TRN-ROAD-2.2 (TRN T1, T2)
#[test]
fn trn_t01_t02_routes_from_the_headquarters() {
    let Some(world) = tutorial_with_headquarters() else {
        return;
    };
    for (to, steps, cost) in [((45, 33), 4, 27), ((41, 39), 5, 33)] {
        let (path, found) = world.find_route(P0, (45, 37), to).unwrap();
        assert_eq!((path.len() - 1, found), (steps, cost), "to {to:?}");
        assert_eq!(path.first(), Some(&(45, 37)));
        assert_eq!(path.last(), Some(&to));
    }
}

// TRN T3: the cheaper of two two-step routes.
// spec: TRN-ROAD-2.1, TRN-ROAD-2.3 (TRN T3)
#[test]
fn trn_t03_cheapest_route() {
    let Some(world) = tutorial_with_headquarters() else {
        return;
    };
    let (path, cost) = world.find_route(P0, (45, 37), (45, 35)).unwrap();
    assert_eq!(path, [(45, 37), (46, 36), (45, 35)]);
    assert_eq!(cost, 12);
    let other = world.step_cost((45, 37), (45, 36)) + world.step_cost((45, 36), (45, 35));
    assert_eq!(other, 13);
}

// TRN T4
// spec: TRN-ROAD-1.1 (TRN T4)
#[test]
fn trn_t04_target_not_general_site() {
    let Some(world) = tutorial_with_headquarters() else {
        return;
    };
    for to in [(49, 37), (50, 41)] {
        assert_eq!(
            world.find_route(P0, (45, 37), to),
            Err(Rejection::TargetNotGeneralSite),
            "{to:?}"
        );
    }
}

// TRN T5
// spec: TRN-ROAD-2.1, TRN-ROAD-1.2 (TRN T5)
#[test]
fn trn_t05_detour() {
    let Some((world, _)) = world(T01) else { return };
    let (path, cost) = world.find_route(P0, (40, 36), (40, 32)).unwrap();
    assert_eq!((path.len() - 1, cost), (11, 75));
}

// TRN T6: the search reaches the start over this route first, while a
// route of cost 33 exists.
// spec: TRN-ROAD-2.1, TRN-ROAD-2.3, TRN-ROAD-2.4 (TRN T6)
#[test]
fn trn_t06_fight_route() {
    let Some((mut world, rules)) = world(FIGHT) else {
        return;
    };
    headquarters(
        &mut world,
        &rules,
        &[(27, 57), (52, 56), (52, 24), (26, 24)],
    );
    let (path, cost) = world.find_route(P0, (28, 56), (24, 54)).unwrap();
    assert_eq!(
        path,
        [(28, 56), (27, 56), (26, 56), (25, 56), (24, 55), (24, 54)]
    );
    assert_eq!(cost, 34);
}

// TRN T1 as commands: a flag at the end, then the road.
// spec: TRN-ROAD-3.1, TRN-ROAD-2.1
#[test]
fn trn_road_commands_on_a_real_map() {
    let Some(mut world) = tutorial_with_headquarters() else {
        return;
    };
    let outcomes = apply(
        &mut world,
        vec![
            Command::PlaceFlag {
                player: P0,
                at: (45, 33),
            },
            Command::BuildRoad {
                player: P0,
                from: (45, 37),
                to: (45, 33),
            },
            // TRN-ROAD-1.2: a second road may not cross the first. The spec
            // does not forbid a second road between the same flags along
            // another route (Q-034).
            Command::BuildRoad {
                player: P0,
                from: (45, 37),
                to: (45, 33),
            },
        ],
    );
    assert_eq!(outcomes, [Ok(()), Ok(()), Ok(())]);
    let roads: Vec<_> = world.roads().collect();
    assert_eq!(roads[0].path.len(), 5);
    for &inner in &roads[0].path[1..4] {
        assert_eq!(world.road_at(inner), Some(roads[0]));
    }
    let first_inner = &roads[0].path[1..roads[0].path.len() - 1];
    let second_inner = &roads[1].path[1..roads[1].path.len() - 1];
    assert!(second_inner.iter().all(|f| !first_inner.contains(f)));
}
