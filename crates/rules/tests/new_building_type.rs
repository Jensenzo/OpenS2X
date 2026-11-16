//! Hook H6 (`docs/extending.md`, "New building types") with the original
//! rules of the player's own install (`S2_GAME_DIR`; skipped when unset): a
//! project's overlay adds a building type, a charcoal burner (2 wood
//! make 1 coal, a helper works it, it looks like the
//! woodcutter's hut), and a game builds it, staffs it, and it makes coal
//! that reaches the headquarters. Its game snapshots and loads with it. The
//! original types and coal's own cost list are as without the overlay.

use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::grid::{self, Point};
use s2_sim::{Command, Placement, PlayerId, WorkCycle, World};

const MAPS: &str = "data/maps/freeGameMaps";
const P0: PlayerId = PlayerId(0);
const BURNER: u32 = 0xC4A4_C0A1;
const LIMIT: u64 = 20_000;

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

/// The overlay a project writes: the woodcutter's hut's size, costs plus one
/// stone, construction time and look; a helper; coal from 2 wood in twice the
/// smelting works' time; sort 9, the first free small cell.
fn charcoal_overlay(rules: &s2_rules::RuleSet) -> s2_rules::Overlay {
    let id = rules.id_named("woodCutter").expect("woodCutter");
    let hut = rules
        .buildings
        .get(id)
        .expect("the woodcutter's hut")
        .clone();
    let smeltery = rules.id_named("smeltery").expect("smeltery");
    let smelting = rules
        .buildings
        .get(smeltery)
        .unwrap()
        .production_time
        .unwrap();
    let gfx = hut.gfx_name.clone().expect("the hut's look");
    let (ix, iy) = hut.icon.expect("the hut's icon");
    let code = format!(
        "charcoalBurner = properties.bp_create(\"{BURNER:08X}\")
         properties.bp_setName(charcoalBurner, \"Charcoal Burner\")
         properties.bp_setGfxName(charcoalBurner, \"{gfx}\")
         properties.bp_setIconXY(charcoalBurner, {ix}, {iy})
         properties.bp_setSort(charcoalBurner, 9)
         properties.bp_setSize(charcoalBurner, 0)
         properties.bp_addFrameCost(charcoalBurner, plank, 1)
         properties.bp_addCost(charcoalBurner, plank, 1)
         properties.bp_addCost(charcoalBurner, stone, 1)
         properties.bp_setConstructionTime(charcoalBurner, {construction})
         properties.bp_setWorker(charcoalBurner, good_helper)
         properties.bp_setGoods(charcoalBurner, coal, 1)
         properties.bp_setProductionTime(charcoalBurner, {time})
         properties.bp_setStockLimit(charcoalBurner, 6)
         reborn.bp_addInput(charcoalBurner, wood, 2)",
        construction = hut.construction_time.unwrap_or(442.0),
        time = 2.0 * smelting,
    );
    s2_rules::Overlay {
        name: "charcoal".into(),
        code: code.into_bytes(),
    }
}

/// A new game of player 0 on `map` with the base stock x 8 and no coal.
fn start(rules: &s2_rules::RuleSet, map: &s2m::Map) -> World {
    let mut sim = rules.sim_rules().unwrap();
    let coal = rules.id_named("coal").unwrap();
    sim.start_stock.retain(|&(good, _)| good != coal);
    let setup = s2_rules::game_setup(map, rules, 3);
    World::new_game(7, map.terrain(), sim, &setup)
}

/// The burner on a small place near the headquarters with its road to the
/// headquarters' flag, if the map has one.
fn place(world: &World, map: &s2m::Map) -> Option<(Point, Vec<Command>)> {
    let hq = (map.info.start_positions[0].x, map.info.start_positions[0].y);
    let hq_flag = world.terrain().neighbour(hq, 1)?;
    let (w, h) = (map.info.width, map.info.height);
    for f in (2..=8).flat_map(|k| grid::ring(hq, k, w, h)) {
        let class = world.placement_at(f);
        if class < Placement::Small || class == Placement::Mine {
            continue;
        }
        let Some(flag) = world.terrain().neighbour(f, 1) else {
            continue;
        };
        if flag == hq_flag {
            continue;
        }
        let commands = vec![
            Command::PlaceBuilding {
                player: P0,
                building: BURNER,
                at: f,
            },
            Command::BuildRoad {
                player: P0,
                from: flag,
                to: hq_flag,
            },
        ];
        let mut probe = world.clone();
        let ok = commands.iter().all(|c| {
            probe.submit(c.clone());
            probe.step();
            probe.last_results()[0].outcome.is_ok()
        });
        if ok {
            return Some((f, commands));
        }
    }
    None
}

// spec: none (hook H6: a project's building type through its overlay; its work is ECO-WORK-3's, its orders ECO-PROD-6.1's)
#[test]
fn an_overlay_building_type_is_built_staffed_and_makes_coal() {
    let Some(dir) = game_dir() else { return };
    let (plain, _) = s2_rules::load(&dir).unwrap();
    let overlay = charcoal_overlay(&plain);
    let (rules, report) = s2_rules::load_with_overlays(&dir, &[overlay]).unwrap();
    assert_eq!(report.scripts.last().unwrap(), "overlay:charcoal");
    let (wood, coal, helper) = (
        rules.id_named("wood").unwrap(),
        rules.id_named("coal").unwrap(),
        rules.id_named("good_helper").unwrap(),
    );
    let sim = rules.sim_rules().unwrap();
    // The catalogue: the original's types unchanged, one more.
    let original = plain.sim_rules().unwrap();
    let mut types = sim.building_types.clone();
    let burner = types
        .iter()
        .position(|t| t.id == BURNER)
        .expect("the burner");
    let burner = types.remove(burner);
    assert_eq!(types, original.building_types);
    assert_eq!(burner.cycle, WorkCycle::Workshop);
    assert_eq!((burner.worker, burner.product), (Some(helper), Some(coal)));
    // Its recipe is its own; coal's cost list is the original's.
    assert_eq!(sim.recipes.get(&BURNER), Some(&vec![(wood, 2)]));
    assert_eq!(sim.recipes.len(), 1);
    assert_eq!(sim.goods[&coal].costs, original.goods[&coal].costs);
    assert!(original.recipes.is_empty());

    let mut names: Vec<_> = std::fs::read_dir(dir.root().join(MAPS))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "s2m"))
        .collect();
    names.sort();
    for path in names {
        let rel = format!("{MAPS}/{}", path.file_name().unwrap().to_string_lossy());
        let map = s2m::load(&rel, &std::fs::read(&path).unwrap()).unwrap();
        let mut world = start(&rules, &map);
        let Some((at, commands)) = place(&world, &map) else {
            continue;
        };
        eprintln!("map {rel}: the burner at {at:?}");
        let hq = (map.info.start_positions[0].x, map.info.start_positions[0].y);
        for c in commands {
            world.submit(c);
            world.step();
            assert_eq!(world.last_results()[0].outcome, Ok(()));
        }
        let (mut staffed, mut made, mut delivered) = (None, None, None);
        for tick in 1..=LIMIT {
            world.step();
            let b = world
                .buildings()
                .find(|b| b.at == at)
                .expect("the burner stands");
            if staffed.is_none() && b.active {
                staffed = Some(tick);
            }
            if made.is_none() && b.production.count >= 1 {
                made = Some(tick);
            }
            if delivered.is_none() && world.warehouse(hq).is_some_and(|w| w.count(coal) > 0) {
                delivered = Some(tick);
            }
            if delivered.is_some() && made.is_some() {
                break;
            }
        }
        eprintln!(
            "staffed {staffed:?}, first coal {made:?}, coal in the headquarters {delivered:?}"
        );
        assert!(staffed.is_some(), "the burner is staffed");
        assert!(made.is_some(), "the burner makes coal");
        assert!(delivered.is_some(), "its coal reaches the headquarters");
        // A snapshot of the game loads as the same game, the burner and its
        // recipe with it, and plays on the same.
        let bytes = world.snapshot();
        let mut r = s2_sim::snap::Reader::new(&bytes);
        let mut loaded = World::read_snapshot(&mut r).unwrap();
        assert_eq!(loaded.state_hash(), world.state_hash());
        assert_eq!(loaded.recipe(BURNER), Some(&[(wood, 2)][..]));
        for _ in 0..200 {
            world.step();
            loaded.step();
        }
        assert_eq!(loaded.state_hash(), world.state_hash());
        return;
    }
    panic!("no map with room for the burner");
}

/// Hook W20's companion (docs/extending.md "Recipes a player chooses"):
/// `reborn.dist_addConsumer` adds a building type to the distribution table
/// of a good (ECO-DIST-2.1) after the original's consumers, once; the
/// original's tables are unchanged without it.
// spec: none (hook W20; the tables are ECO-DIST-2.1's)
#[test]
fn an_overlay_adds_a_consumer_to_a_distribution_table() {
    let Some(dir) = game_dir() else { return };
    let overlay = s2_rules::Overlay {
        name: "fish-for-the-woodcutter".into(),
        code: b"reborn.dist_addConsumer(fish, woodCutter, 200)
                reborn.dist_addConsumer(bread, woodCutter, 300)"
            .to_vec(),
    };
    let (plain, _) = s2_rules::load(&dir).unwrap();
    let (rules, _) = s2_rules::load_with_overlays(&dir, &[overlay]).unwrap();
    let woodcutter = rules.names["woodCutter"];
    let fish = rules.names["fish"];
    let food = |r: &s2_rules::RuleSet| {
        r.sim_rules()
            .unwrap()
            .distribution
            .into_iter()
            .find(|t| t.goods.contains(&fish))
            .expect("the food table")
            .entries
    };
    let (before, after) = (food(&plain), food(&rules));
    assert!(!before.iter().any(|&(k, _)| k == woodcutter));
    assert_eq!(&after[..before.len()], &before[..]);
    assert_eq!(&after[before.len()..], &[(woodcutter, 200)]);
    // Every other table as the original's.
    let others = |r: &s2_rules::RuleSet| {
        r.sim_rules()
            .unwrap()
            .distribution
            .into_iter()
            .filter(|t| !t.goods.contains(&fish))
            .collect::<Vec<_>>()
    };
    assert_eq!(others(&plain), others(&rules));
}
