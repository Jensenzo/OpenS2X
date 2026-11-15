//! The AI on the real rules and a real map (`S2_GAME_DIR`; skipped when
//! unset).

use s2_ai::{AiConfig, AiSystem, Mode};
use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::{PlayerId, SettlerJob, World};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

pub fn game() -> Option<(s2_rules::RuleSet, AiConfig, World)> {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let bytes = std::fs::read(dir.root().join(rel)).expect("map file");
    let map = s2m::load(rel, &bytes).expect("map");
    let world = s2_rules::new_game(7, &map, &rules, 1).expect("game");
    let config = AiConfig::new(&rules, &rules.sim_rules().expect("sim rules"));
    Some((rules, config, world))
}

// AI-1.2 / AI-overview T3: the weak profile.
#[test]
fn the_profiles_are_those_of_the_script() {
    let Some((_, config, _)) = game() else {
        return;
    };
    let weak = config.profile(1).unwrap();
    assert_eq!(weak.mode, Mode::Safe);
    assert_eq!(weak.attack_strength, 0.51);
    assert_eq!(weak.max_mints, 0);
    assert!(!weak.catapults);
    assert!(weak.route_optimizing && !weak.street_optimizing);
    let normal = config.profile(2).unwrap();
    assert_eq!(normal.mode, Mode::Aggressive);
    assert_eq!(
        (normal.start_sawmills, normal.max_smithies, normal.max_mints),
        (2, 2, 1)
    );
    let strong = config.profile(3).unwrap();
    assert_eq!(strong.mode, Mode::Normal);
    assert_eq!(
        (strong.start_sawmills, strong.max_smithies, strong.max_mints),
        (3, 4, 3)
    );
    assert!(strong.coin_arrangement && strong.catapults && strong.intercepting == 0.5);
}

// AI-1.3: the weights come from the building and good scripts.
#[test]
fn the_script_weights_are_read() {
    let Some((rules, config, _)) = game() else {
        return;
    };
    let n = &config.names;
    for id in [
        n.castle,
        n.sawmill,
        n.stone_pit,
        n.woodcutter,
        n.forester,
        n.hunter,
        n.fisher,
        n.depot,
        n.harbor,
        n.shipyard,
        n.smithy,
        n.brewery,
        n.mint,
        n.farm,
        n.locksmith,
        n.catapult,
        n.barracks,
        n.guard_house,
        n.tower,
        n.fortress,
    ] {
        assert!(rules.buildings.contains(id), "building {id}");
    }
    for id in [
        n.wood, n.plank, n.stone, n.fish, n.flesh, n.bread, n.beer, n.coal, n.iron, n.iron_ore,
        n.gold, n.sword, n.shield, n.coin, n.boat, n.corn, n.donkey, n.tools, n.food,
    ] {
        assert!(rules.goods.contains(id), "good {id}");
    }
    assert_eq!(config.weights_of(n.guard_house).threat, 30);
    assert_eq!(config.weights_of(n.harbor).score, 100);
    assert_eq!(
        config.weights_of(rules.id_named("goldMine").unwrap()).score,
        180
    );
    assert!(config.producer.contains_key(&n.plank));
    assert_eq!(config.producer[&n.plank], n.sawmill);
    assert_eq!(config.producer[&n.stone], n.stone_pit);
    assert_eq!(
        config.mine_producer[&n.stone],
        rules.id_named("stoneMine").unwrap()
    );
    assert_eq!(config.tool_slots.len(), 12);
    // AI-CON-1.4 (T18): fish only has the fisher, in the mine table; flesh the
    // slaughterhouse in the normal table and no mine entry; wood the woodcutter.
    assert_eq!(config.mine_producer[&n.fish], n.fisher);
    assert!(!config.producer.contains_key(&n.fish));
    assert_eq!(config.producer[&n.flesh], n.slaughterhouse);
    assert!(!config.mine_producer.contains_key(&n.flesh));
    assert_eq!(config.producer[&n.wood], n.woodcutter);
    assert_eq!(config.producer[&n.sword], n.smithy);
    assert_eq!(config.producer[&n.shield], n.smithy);
    assert_eq!(config.producer[&n.tools], n.locksmith);
    assert_eq!(
        config.mine_producer[&n.coal],
        rules.id_named("coalMine").unwrap()
    );
}

// spec: AI-MISC-2.1, AI-MISC-2.2
#[test]
fn computer_slots_are_activated_and_their_headquarters_announced() {
    let Some((_, config, mut world)) = game() else {
        return;
    };
    let mut ai = AiSystem::new(config, &world);
    ai.activate_all(&mut world, &[(P0, 1), (P1, 3)]);
    assert!(ai.is_active(P0) && ai.is_active(P1));
    let p = ai.player(P1).unwrap();
    assert_eq!(p.profile_number, 3);
    assert_eq!(p.cells.len(), 1);
    for _ in 0..300 {
        for c in ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        ) {
            world.submit(c);
        }
        world.step();
    }
    assert!(ai.is_active(P0) && ai.is_active(P1));
}

/// The game of the original's recording E-trace-002 and of
/// `tests/traces/`: MP_2P_Storm_Coast, the default set-up (player 1 a
/// normal computer player, start resources Medium), the game's seed.
fn recorded_game() -> Option<(s2_ai::run::Runner, std::collections::BTreeMap<u32, String>)> {
    // UI-SETUP-2.5: the free game's default computer slot, strength 0,
    // plays profile 1 (Weak; Q-275).
    game_with_profile(1)
}

/// The game of the original's recording E-trace-012 (the Hard pair): the
/// default set-up with player 1 a computer player of profile 3.
fn hard_game() -> Option<(s2_ai::run::Runner, std::collections::BTreeMap<u32, String>)> {
    game_with_profile(3)
}

fn game_with_profile(
    profile: u8,
) -> Option<(s2_ai::run::Runner, std::collections::BTreeMap<u32, String>)> {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let bytes = std::fs::read(dir.root().join(rel)).expect("map file");
    let map = s2m::load(rel, &bytes).expect("map");
    let setup = s2_rules::game_setup(&map, &rules, 1);
    let sim = rules.sim_rules().expect("sim rules");
    let mut world = World::new_game(
        u64::from(s2_sim::rng::SIM_SEED),
        map.terrain(),
        sim.clone(),
        &setup,
    );
    let mut ai = AiSystem::new(AiConfig::new(&rules, &sim), &world);
    ai.activate_all(&mut world, &[(P1, profile)]);
    let names: std::collections::BTreeMap<u32, String> = rules
        .names
        .iter()
        .map(|(name, &id)| (id, name.clone()))
        .collect();
    let labels = names.iter().map(|(&id, name)| (id, name.clone())).collect();
    Some((s2_ai::run::Runner::new(world, ai, labels), names))
}

/// A table of the original's recordings (`specs/traces/NAME`, gzip csv), or
/// None with a line saying so when the folder does not hold it: the public
/// copy has the game but not the recordings (docs/releasing.md, "The public
/// copy").
fn recording_table(name: &str) -> Option<String> {
    use std::io::Read;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/traces")
        .join(name);
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "skipped: specs/traces/{name} is missing (the public copy has no recordings)"
            );
            return None;
        }
        Err(e) => panic!("{}: {e}", path.display()),
    };
    let mut text = String::new();
    flate2::read::GzDecoder::new(file)
        .read_to_string(&mut text)
        .expect("gzip");
    Some(text)
}

// spec: none (a test without the recordings skips instead of failing)
#[test]
fn a_missing_recording_table_skips() {
    assert_eq!(recording_table("no-such-table.csv.gz"), None);
}

// AI-CON-5 T21: player 1's first ten sites in the recorded game, each at
// game counter k (the trace tick k + 1) in the tick P + 3 + n of its
// 50-tick step P. All ten sites are checked whole: the fourth, the
// woodcutter of the wood need (AI-CON-2.5, T25), on (142, 56) by the border
// factor of the ring at distance 2 (AI-CON-6.4, T27); the fifth by T34; the
// tenth, the smithy of the sword need, in the home cell made first
// (AI-CON-3.3: the cells in list order).
// spec: AI-CON-5.2, AI-CON-5.3a, AI-CON-5.4 (T21), AI-CON-1.1a (T23), AI-CON-7.3 (T22), AI-CON-2.5 (T25), AI-CON-6.4 (T27), AI-CON-3.3, AI-2.1a, AI-2.2
#[test]
fn t21_the_first_ten_sites_of_the_recorded_game() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let expected: [(u64, &str, (u32, u32)); 10] = [
        (127, "hunter", (140, 49)),
        (323, "sawMill", (138, 49)),
        (596, "hunter", (139, 47)),
        (818, "woodCutter", (142, 56)),
        (1100, "guardHouse", (138, 57)),
        (1308, "stonePit", (140, 57)),
        (1474, "harbor", (144, 47)),
        (1838, "woodCutter", (136, 47)),
        (2703, "stonePit", (139, 55)),
        (3198, "smithy", (137, 55)),
    ];
    let mut seen: std::collections::BTreeSet<(u32, u32)> =
        game.world.buildings().map(|b| b.at).collect();
    let mut sites = Vec::new();
    while game.world.tick() <= 3199 && sites.len() < expected.len() {
        // spec: AI-CON-5.3a, the site tick P + 3 + n.
        game.step();
        let k = game.world.tick() - 1;
        for b in game.world.buildings() {
            if b.owner == P1 && seen.insert(b.at) {
                let name = names[&b.building_type].clone();
                sites.push((k, name, b.at));
            }
        }
    }
    let got: Vec<(u64, &str, (u32, u32))> = sites
        .iter()
        .map(|(k, name, at)| (*k, name.as_str(), *at))
        .collect();
    const CHECKED: usize = 10;
    assert_eq!(got[..CHECKED], expected[..CHECKED], "all sites: {got:?}");
}

// AI-CON T34: the guard house order of cell (8, 3), taken at trace tick
// 1045, searches 53 steps and places (138, 57) in trace tick 1101: value
// 45.5 (91 x 0.5 for its large field; the 9 stone fields south of it count,
// as the code reads, Q-357), route 7 from the cell's position, score
// 45.5 x 9 / 16, above (135, 56) with 50.5 x 8 / 16 and (136, 48) with
// 32 x 11 / 16. The landmass is the map's stored continent index: with the
// recomputed regions (every non-water field one landmass) the mountains
// west of the castle count and (135, 48) is worth 55.5 x 10 / 16.
// spec: AI-CON-6.3 (T34), AI-CON-5.4a, AI-CON-5.4, FMT-S2M-4.9
#[test]
fn t34_the_guard_house_takes_the_continent_of_its_site() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    // The world after game counter 1099, the tick before the site's.
    while game.world.tick() < 1100 {
        game.step();
    }
    let guard = *names
        .iter()
        .find(|(_, n)| n.as_str() == "guardHouse")
        .expect("guardHouse")
        .0;
    let t = game.world.building_type(guard).expect("type").clone();
    let config = game.ai.config();
    let snapshot = s2_ai::snapshot::Snapshot::take(&game.world);
    let value = |world: &World, at: (u32, u32)| {
        s2_ai::sites::SiteEnv {
            world,
            config,
            player: P1,
            own: snapshot.of(P1),
        }
        .value(&t, at)
    };
    let score = |v: f32, len: u32| s2_ai::sites::score(v, len, 16);
    assert_eq!(value(&game.world, (138, 57)), 45.5);
    assert_eq!(value(&game.world, (135, 56)), 50.5);
    assert_eq!(value(&game.world, (136, 48)), 32.0);
    let best = score(45.5, 7);
    assert!(best > score(50.5, 8) && best > score(32.0, 5));
    let start = game
        .ai
        .player(P1)
        .and_then(|p| p.cells.get((8, 3)))
        .expect("cell (8, 3)")
        .route_start();
    let route = |f: (u32, u32)| s2_ai::sites::route_length(&game.world, start, f);
    assert_eq!(
        [(138, 57), (135, 56), (136, 48)].map(route),
        [Some(7), Some(8), Some(5)]
    );
    // The recomputed regions in place of the stored layer.
    let mut recomputed = game.world.clone();
    let layer = recomputed
        .terrain()
        .fields()
        .map(|f| match recomputed.region_of(f) {
            0 => -1,
            r => r as i32,
        })
        .collect();
    recomputed.set_continents(layer);
    assert_eq!(value(&recomputed, (135, 48)), 55.5);
    assert!(score(55.5, 6) > best);
    assert_eq!(route((135, 48)), Some(6));
    assert!(score(value(&game.world, (135, 48)), 6) < best);
    // The site in the next tick.
    game.step();
    let site = game
        .world
        .building_at((138, 57))
        .expect("the guard house site");
    assert_eq!((site.owner, site.building_type), (P1, guard));
}

// ECO-FOOD-5.15 on the recorded game: 261 animals at trace tick 1 and 266
// at 1000, as the original (E-trace-002). Later counts depend on the
// hunters, and the games part before (docs/fidelity.md).
// spec: ECO-FOOD-5.15, ECO-FOOD-5.13
#[test]
fn the_respawn_places_five_animals_in_the_first_thousand_ticks() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    game.step();
    assert_eq!(game.world.animals().count(), 261);
    while game.world.tick() < 1000 {
        game.step();
    }
    assert_eq!(game.world.animals().count(), 266);
}

// ECO-FOOD-5.15 T40: the one-player game of E-trace-004 (slot 1 closed):
// the first placements of the respawn in trace ticks 425 (a deer on
// (65, 14)), 475 (100, 13), 625 (25, 22), 800 (140, 20) and 900 (21, 38).
// spec: ECO-FOOD-5.15 (T40), ECO-FOOD-5.11b
#[test]
fn t40_the_respawn_placements_of_the_one_player_game() {
    let Ok(dir) = GameDir::from_env() else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let (rules, _) = s2_rules::load(&dir).expect("rules");
    let rel = "data/maps/freeGameMaps/MP_2P_Storm_Coast.s2m";
    let bytes = std::fs::read(dir.root().join(rel)).expect("map file");
    let map = s2m::load(rel, &bytes).expect("map");
    let mut setup = s2_rules::game_setup(&map, &rules, 1);
    setup.players.retain(|&(p, _)| p == P0);
    let sim = rules.sim_rules().expect("sim rules");
    let mut world = World::new_game(u64::from(s2_sim::rng::SIM_SEED), map.terrain(), sim, &setup);
    // The deer (ECO-FOOD-5.15: 4a9bef83).
    let deer = 0x4a9b_ef83;
    let mut placed = Vec::new();
    let mut known = world.animals().count();
    while world.tick() < 1000 {
        world.step();
        let all: Vec<_> = world.animals().map(|(_, a)| (a.kind, a.home)).collect();
        if all.len() > known {
            let (kind, home) = all[all.len() - 1];
            placed.push((world.tick(), home));
            if world.tick() == 425 {
                assert_eq!(kind, deer);
            }
            known = all.len();
        }
    }
    assert_eq!(
        placed,
        [
            (425, (65, 14)),
            (475, (100, 13)),
            (625, (25, 22)),
            (800, (140, 20)),
            (900, (21, 38))
        ]
    );
}

// TRN T20: road 90549 of the recorded game, (136, 57) to (139, 56). Its
// fields have the same heights in the start world: the road is read over
// (137, 56) and (138, 56) at cost 24, while the route over (137, 57) and
// (138, 57) costs 22.
// spec: TRN-ROAD-2.1, TRN-ROAD-2.3, TRN-ROAD-2.4 (TRN T20)
#[test]
fn trn_t20_a_road_dearer_than_the_cheapest() {
    let Some((game, _)) = recorded_game() else {
        return;
    };
    let world = &game.world;
    let (path, cost) = world.find_route(P1, (136, 57), (139, 56)).unwrap();
    assert_eq!(path, [(136, 57), (137, 56), (138, 56), (139, 56)]);
    assert_eq!(cost, 24);
    let cheapest = [(136, 57), (137, 57), (138, 57), (139, 56)];
    let cheapest_cost: u32 = cheapest
        .windows(2)
        .map(|w| world.step_cost(w[1], w[0]))
        .sum();
    assert_eq!(cheapest_cost, 22);
}

// AI-CON-8.1d T24: the hunter's flag, made in the run tick 127, is first
// seen at 134 and taken as F; its road, built at 176, runs from it to the
// castle's flag (its first flag is F, TRN-CARRY-1.3 T20), over the fields
// of the road search from the castle's flag and the read-out from F
// (TRN T19: direction 0 is no candidate at the first step, 3 is the
// lowest).
// spec: AI-CON-8.1, AI-CON-8.1d (T24), TRN-CARRY-1.3, TRN-ROAD-2.3, TRN-ROAD-2.4 (TRN T19)
#[test]
fn t24_the_first_road_starts_at_the_hunters_flag() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() <= 176 {
        game.step();
        let k = game.world.tick() - 1;
        let roads = game.world.roads().filter(|r| r.owner == P1).count();
        assert_eq!(roads, usize::from(k >= 176), "counter {k}");
    }
    let road = game.world.roads().find(|r| r.owner == P1).unwrap();
    assert_eq!(road.path, [(141, 48), (140, 48), (139, 49), (140, 50)]);
}

/// Player 1's street work after the run of each counter in `runs`, as
/// (flag, target, attempts, L, L2, best), `None` once the flag is done; the
/// game is stepped to the last of them.
#[allow(clippy::type_complexity)]
fn street_states(
    game: &mut s2_ai::run::Runner,
    runs: &[u64],
) -> Vec<(
    u64,
    Option<((u32, u32), Option<(u32, u32)>, u32, u32, u32, u8)>,
)> {
    let mut out = Vec::new();
    while game.world.tick() <= *runs.last().unwrap() {
        game.step();
        let k = game.world.tick() - 1;
        if runs.contains(&k) {
            let work = game.ai.player(P1).unwrap().street.work.as_ref();
            out.push((
                k,
                work.map(|w| (w.flag, w.target, w.attempts, w.length, w.length2, w.best)),
            ));
        }
    }
    out
}

fn road_with(game: &s2_ai::run::Runner, path: &[(u32, u32)]) -> bool {
    game.world
        .roads()
        .any(|r| r.owner == P1 && (r.path == path || r.path.iter().rev().eq(path.iter())))
}

// AI-CON-8.1 T31: the sawmill flag's runs 351 .. 386. Attempt 1 finds no
// walk path F -> T (the 2-step way crosses (139, 49), a field of R, and
// walk paths never enter road fields, 8.1e); attempt 3's road-stop path is
// 3 fields with F and T (L 3), its second leg T -> P walks over R's fields
// in 3 steps; attempt 4's F -> P in 2 (through (140, 48), on R); attempt 5
// finds nothing shorter; 386 builds strategy 4. The values of E-trace-006's
// `ai_street` rows.
// spec: AI-CON-8.1, AI-CON-8.1e (T31), AI-2.7
#[test]
fn t31_the_sawmill_flags_attempts() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let (f, t) = ((139, 48), Some((140, 50)));
    let got = street_states(&mut game, &[344, 351, 358, 365, 372, 379, 386]);
    assert_eq!(
        got,
        [
            (344, Some((f, t, 0, 1000, 1000, 0))),
            (351, Some((f, t, 1, 1000, 1000, 0))),
            (358, Some((f, t, 2, 1000, 1000, 0))),
            (365, Some((f, t, 3, 3, 3, 3))),
            (372, Some((f, t, 4, 3, 2, 4))),
            (379, Some((f, t, 5, 3, 2, 4))),
            (386, None),
        ]
    );
    assert!(road_with(&game, &[(139, 48), (139, 49), (140, 50)]));
    assert!(road_with(&game, &[(139, 48), (140, 48), (141, 48)]));
}

// AI-CON-8.1 T30: the second hunter's flag F (140, 46), T the hunter's flag
// (141, 48). Attempt 1's walk path is 2 steps (L 2, best 1); the road-stop
// path "140,47 141,48" of attempts 3 to 5 is 3 fields with F, more than L,
// so strategy 1 stays and 645 builds "140,46 140,47 141,48"; R "139,48
// 140,48 141,48" stays.
// spec: AI-CON-8.1, AI-CON-8.1e (T30)
#[test]
fn t30_the_second_hunters_flag_keeps_the_walk_path() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let (f, t) = ((140, 46), Some((141, 48)));
    let got = street_states(&mut game, &[603, 610, 617, 624, 631, 638, 645]);
    assert_eq!(
        got,
        [
            (603, Some((f, t, 0, 1000, 1000, 0))),
            (610, Some((f, t, 1, 2, 1000, 1))),
            (617, Some((f, t, 2, 2, 1000, 1))),
            (624, Some((f, t, 3, 2, 1000, 1))),
            (631, Some((f, t, 4, 2, 1000, 1))),
            (638, Some((f, t, 5, 2, 1000, 1))),
            (645, None),
        ]
    );
    assert!(road_with(&game, &[(140, 46), (140, 47), (141, 48)]));
    assert!(road_with(&game, &[(139, 48), (140, 48), (141, 48)]));
}

// AI-CON-8.3 T32: the route optimizer joins the hunter's flag (141, 48) to
// the castle's flag (140, 50) directly. B = the hunter, S = the castle (the
// partners are warehouses only, 8.3 b), the ware cost of the flags' route
// 6 + 4 = 10 (the 2-step road (141, 48) .. (139, 48) costs L 6 on its
// slopes; TRN-CARRY-5.3: the assigned carriers are free) > 4 x 2: the
// direct variant; the path's field 2, (139, 49), has the castle's flag
// nearest, X's last entry is the hunter's flag: the road "140,50 141,50
// 141,49 141,48", built in counter 660 as in the original (Q-353).
// spec: AI-CON-8.3 (T32), TRN-CARRY-5.3, TRN-FREE-3.2
#[test]
fn t32_the_route_optimizer_joins_the_hunter_to_the_castle() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let road = [(140, 50), (141, 50), (141, 49), (141, 48)];
    while game.world.tick() <= 700 {
        game.step();
        let k = game.world.tick() - 1;
        if k == 659 {
            assert_eq!(game.world.route_cost(P1, (141, 48), (140, 50)), Some(10));
        }
        assert_eq!(road_with(&game, &road), k >= 660, "counter {k}");
    }
}

// AI-CON-8.3 T33: the route optimizer's state after each of its runs
// (counters k with k + 6 = 0 mod 37) from 31 to 993, as E-trace-008's
// `ai_route` table measures it: the cursor (0 after a restart), and k, i,
// w and X where the table shows them change. At 808 the sawmill's partner,
// the castle, is finished although the road planner refuses the road
// between their flags (Q-273), and the refused road's planning stub takes a
// number (SIM-START-4.4 c, Q-274): the site of 818 is 90269.
// spec: AI-CON-8.3 (T33), SIM-START-4.4
#[test]
fn t33_the_route_optimizers_runs() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    #[derive(Debug, PartialEq)]
    enum Is {
        Cursor(u32),
        I(usize),
        K(u32),
        W(usize, Vec<(u32, u32)>),
    }
    use Is::*;
    let expected = [
        (31, Cursor(90180)),
        (68, Cursor(90181)),
        (105, Cursor(0)),
        (142, Cursor(90180)),
        (179, Cursor(90181)),
        (216, Cursor(90190)),
        (253, I(1)),
        (290, K(1)),
        (327, Cursor(90191)),
        (364, Cursor(90211)),
        (401, Cursor(90212)),
        (438, Cursor(0)),
        (475, Cursor(90180)),
        (512, Cursor(90181)),
        (549, Cursor(90190)),
        (586, W(1, vec![(141, 48)])),
        (623, W(2, vec![(141, 48)])),
        (660, K(1)),
        (697, Cursor(90191)),
        (734, Cursor(90211)),
        (771, W(1, vec![(139, 48)])),
        (808, K(1)),
        (845, Cursor(90212)),
        (882, Cursor(90244)),
        (919, K(1)),
        (956, Cursor(90245)),
        (993, Cursor(90269)),
    ];
    let mut got = Vec::new();
    let last = expected.last().unwrap().0;
    while game.world.tick() <= last {
        game.step();
        let k = game.world.tick() - 1;
        let Some((_, want)) = expected.iter().find(|(c, _)| *c == k) else {
            continue;
        };
        let r = &game.ai.player(P1).unwrap().street.route;
        got.push((
            k,
            match want {
                Cursor(_) => Cursor(r.cursor),
                I(_) => I(r.i),
                K(_) => K(r.k),
                W(..) => W(r.w, r.x.clone()),
            },
        ));
    }
    assert_eq!(got, expected);
}

// TRN-FREE-6.5 a (T17): constructor 90255 leaves the castle at counter 647
// for the site (139, 47); the road "140,50 141,50 141,49 141,48" is built at
// 660 while he is on his way out. His road route is chosen on the castle's
// flag (140, 50) in his arrival tick 691: his first road step goes east to
// (141, 50), over the new road.
// spec: TRN-FREE-6.5 (T17), ECO-WORK-3.8c
#[test]
fn t17_the_constructor_chooses_his_route_on_the_castles_flag() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let walker = |game: &s2_ai::run::Runner| {
        game.world
            .settlers()
            .find(|(_, s)| s.number == 90255)
            .map(|(_, s)| s.walker.clone())
    };
    let mut arrival = None;
    while game.world.tick() <= 700 {
        game.step();
        let k = game.world.tick() - 1;
        if let Some(w) = walker(&game).filter(|w| w.at == (140, 50)) {
            arrival.get_or_insert((k, w.next));
        }
    }
    assert_eq!(arrival, Some((691, Some((141, 50)))));
}

// ECO-HQ T-level-8: the hunter site (140, 49), sample (562, 196), placed at
// counter 127 with its flag (141, 48), sample (564, 192) at 5002: the six
// samples around the flag's take 5002 (4.8b) after the site's own height is
// spread (4.8a): (563, 193) is on both rings and ends at 5002; the second
// ring around the flag is untouched, the samples within 2 of the site are at
// 5945; the site's ring 3 next to them averages the six samples around each
// one's direction-1 neighbour (T-level-9, Q-271).
// spec: ECO-HQ-4.8b (T-level-8), ECO-HQ-4.8a, ECO-HQ-4.8 (T-level-9)
#[test]
fn t_level_8_the_hunters_flag_flattens_its_ring() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() <= 127 {
        game.step();
    }
    let t = game.world.terrain();
    let h = |s: (u32, u32)| t.sample_height(s);
    for s in [
        (564, 192),
        (563, 192),
        (565, 192),
        (563, 191),
        (564, 191),
        (563, 193),
        (564, 193),
    ] {
        assert_eq!(h(s), 5002, "{s:?}");
    }
    assert_eq!((h((562, 192)), h((565, 193))), (5708, 4538));
    assert_eq!((h((563, 194)), h((562, 196))), (5945, 5945));
    // T-level-9 (Q-271): the site's ring 3 next to them.
    assert_eq!(
        [h((562, 193)), h((561, 193)), h((560, 193))],
        [5317, 5656, 5978]
    );
}

// ECO-HQ T-level-10: player 1's farm site on (136, 58), placed at counter
// 3724 (T-level-7; the shots run E-trace-017 has 3729) and levelled in six
// steps; the samples west of it (the look comparison's darker ground at tick
// 6000, GFX-VIEW-2.4) hold the recorded heights after the last step (4729).
// Ours keeps them until the guard house on (131, 59) (placed at 5898, as in
// E-trace-005) gets its road "132,58 .. 136,57" in 5951, which evens that
// ground (TRN-ROAD-3.3), as the original's road 90792 does between 5950 and
// 5959 in both recordings (Q-1030, T-level-10 corrected).
// spec: ECO-HQ-4.8 (T-level-10), ECO-HQ-4.8a (T-level-7), TRN-ROAD-3.3
#[test]
fn t_level_10_the_farm_sites_ground_west_of_it() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let want: [((u32, u32), i32); 8] = [
        ((538, 229), 7647),
        ((539, 229), 7471),
        ((539, 230), 7366),
        ((538, 230), 7643),
        ((537, 230), 7850),
        ((538, 231), 7366),
        ((539, 231), 7366),
        ((540, 229), 7366),
    ];
    let heights = |game: &s2_ai::run::Runner| {
        let t = game.world.terrain();
        want.map(|(s, _)| (s, t.sample_height(s)))
    };
    let mut placed = None;
    let mut held = Vec::new();
    while game.world.tick() <= 5951 {
        game.step();
        let k = game.world.tick() - 1;
        if placed.is_none() {
            placed = game
                .world
                .buildings()
                .find(|b| b.owner == P1 && b.at == (136, 58))
                .map(|b| (k, names[&b.building_type].clone()));
        }
        if [4729, 5000, 5950].contains(&k) {
            held.push((k, heights(&game)));
        }
    }
    assert_eq!(placed, Some((3724, "farm".to_string())));
    for (k, got) in held {
        assert_eq!(got, want, "counter {k}");
    }
    let road = game
        .world
        .roads()
        .find(|r| r.path.first() == Some(&(132, 58)) || r.path.last() == Some(&(132, 58)))
        .map(|r| r.path.clone());
    assert!(
        road.as_ref().is_some_and(|p| p.contains(&(135, 57))),
        "{road:?}"
    );
    assert_ne!(heights(&game), want, "the road of 5951 evens the ground");
}

// TRN-ROAD T22: the computer player's road "141,48 140,48 139,49 140,50" of
// counter 176 evens the ground along it (TRN-ROAD-3.3): the first step is
// worked from (564, 192), the first field (A), westwards with q = 329.5 and
// leaves the samples next to it; the second, (560, 192) -> (558, 196) in
// direction 4; the last from (560, 200), the flag field being A.
// spec: TRN-ROAD-3.3 (T22)
#[test]
fn t22_the_first_road_evens_the_ground() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() <= 176 {
        game.step();
    }
    let t = game.world.terrain();
    let h = |s: (u32, u32)| t.sample_height(s);
    assert_eq!(h((563, 192)), 5002);
    assert_eq!(
        [h((562, 192)), h((562, 193)), h((562, 191))],
        [5661, 5496, 5496]
    );
    assert_eq!(
        [h((561, 192)), h((561, 193)), h((561, 191))],
        [5990, 5825, 5825]
    );
    assert_eq!(
        [h((559, 193)), h((559, 194)), h((558, 195))],
        [6474, 6629, 6784]
    );
    assert_eq!([h((559, 192)), h((560, 193))], [6397, 6397]);
    assert_eq!([h((558, 193)), h((560, 194))], [6552, 6552]);
    assert_eq!([h((558, 194)), h((559, 195))], [6706, 6706]);
}

// TRN-FREE T23: the sawmill site's leveller 90231 on (139, 48) after his
// last levelling step: going home from counter 965, standing in 965, 966 and
// 967, his walk home ordered at progress 0 in 968 towards (139, 49).
// spec: TRN-FREE-6.4a (T23)
#[test]
fn t23_the_leveller_stands_three_ticks_before_going_home() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let leveller = |game: &s2_ai::run::Runner| {
        game.world
            .settlers()
            .find(|(_, s)| s.number == 90231)
            .map(|(_, s)| s.walker.clone())
    };
    let mut rows = Vec::new();
    while game.world.tick() <= 969 {
        game.step();
        let k = game.world.tick() - 1;
        if k >= 965 {
            let w = leveller(&game).expect("the leveller");
            let first = w.next.or(w.path.front().copied());
            rows.push((k, w.at, first, w.has_walk(), w.progress()));
        }
    }
    let at = (139, 48);
    assert_eq!(
        rows[..4],
        [
            (965, at, None, false, 0.0),
            (966, at, None, false, 0.0),
            (967, at, None, false, 0.0),
            (968, at, Some((139, 49)), true, 0.0),
        ]
    );
    assert!(rows[4].4 > 0.0, "{rows:?}");
}

// ECO-FOOD T43: in counter 1030 the rabbit 90092 (the map's animal 178,
// numbered from 89 914) walks from (144, 57) to (140, 63) past the computer
// player's site on (142, 56). The extra belongs to the field stepped from,
// so (142, 57), a neighbour of the site, is labelled 28; at (143, 57) the
// kept direction west is not allowed and the route is the recorded one.
// spec: ECO-FOOD-5.11a (T43)
#[test]
fn t43_the_rabbit_keeps_off_the_sites_neighbours() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() <= 1030 {
        game.step();
    }
    let (_, rabbit) = game
        .world
        .animals()
        .find(|&(id, _)| id == 178)
        .expect("the rabbit");
    let walker = &rabbit.walker;
    let route: Vec<_> = walker
        .next
        .into_iter()
        .chain(walker.path.iter().copied())
        .collect();
    assert_eq!(
        route,
        [
            (143, 57),
            (143, 58),
            (142, 59),
            (141, 59),
            (141, 60),
            (140, 61),
            (141, 62),
            (140, 63)
        ]
    );
}

// TRN-FREE T24: the hunter site's builder 90202 stands on his construction
// position (563, 195) when the building is complete (T; counter 1593 in the
// original, later in ours, as the games part before). His rows from T are
// the original's of trace ticks 1594 to 1630: standing in T; the fine walk
// back to the point (564, 192) of the site's flag (141, 48) at progress 0
// in T + 1, over (564, 194) and (563, 193), on the field each step's start
// truncates to; standing on the point in its arrival tick T + 35; the road
// walk home at progress 0 in T + 36.
// spec: TRN-FREE-6.4a (T24), TRN-FREE-5.6
#[test]
fn t24_the_builder_walks_back_to_the_flag_before_going_home() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let mut rows = Vec::new();
    while rows.len() < 38 {
        assert!(
            game.world.tick() < 6000,
            "the hunter site is never complete"
        );
        game.step();
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90202");
        match row {
            Some(r) if !rows.is_empty() || r.job == "Home" => {
                rows.push((r.field, r.fine, r.action));
            }
            _ => {}
        }
    }
    let (west, east) = ((140, 48), (141, 48));
    let expected = [
        (0, (west, (563000, 195000), Stand)),
        (1, (west, (563000, 195000), Walk)),
        (2, (west, (563090, 194909), Walk)),
        (12, (west, (563990, 194009), Walk)),
        (13, (east, (563919, 193919), Walk)),
        (23, (east, (563019, 193019), Walk)),
        (24, (west, (563070, 192929), Walk)),
        (34, (west, (563970, 192029), Walk)),
        (35, (east, (564000, 192000), Stand)),
        (36, (east, (564000, 192000), Walk)),
    ];
    for (i, row) in expected {
        assert_eq!(rows[i], row, "T + {i}: {rows:?}");
    }
    assert_ne!(rows[37].1, rows[36].1, "{rows:?}");
}

// AI-CON-2.5 T25: the sawmill site placed at counter 323 gets a new wood
// need tied to it (no wood need existed: AI-CON-3.2 made none with 24 wood
// in the castle); the needs construction of counter 334 scores wood 1520,
// stone 480, plank 168 and orders the woodcutter. Our computer player
// announces the site in its update of the next tick (324), before any part
// that reads the needs runs.
// spec: AI-CON-2.5 (T25), AI-CON-4.2, AI-CON-1.7
#[test]
fn t25_the_sawmill_sites_wood_need_orders_the_woodcutter() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let id = |name: &str| *names.iter().find(|(_, n)| n.as_str() == name).unwrap().0;
    let (wood, woodcutter) = (id("wood"), id("woodCutter"));
    let sawmill_site = (138, 49);
    while game.world.tick() <= 334 {
        game.step();
        let k = game.world.tick() - 1;
        let p = game.ai.player(P1).unwrap();
        let wood_needs: Vec<_> = p.plan.needs.iter().filter(|n| n.good == wood).collect();
        if k < 323 {
            assert!(wood_needs.is_empty(), "counter {k}: {wood_needs:?}");
        } else if k >= 324 {
            assert_eq!(wood_needs.len(), 1, "counter {k}");
            assert_eq!(wood_needs[0].consumer, Some(sawmill_site), "counter {k}");
        }
        // The needs construction's order has weight 10 (AI-CON-5.1).
        let order = p
            .build
            .slots
            .iter()
            .flatten()
            .find(|o| o.building == woodcutter);
        assert_eq!(
            order.map(|o| o.weight),
            (k >= 334).then_some(10),
            "counter {k}: {order:?}"
        );
    }
}

// TRN T25 (TRN-CARRY-2.4a): at the castle's flag (140, 50) two planks wait
// for the road "140,50 141,50 141,49 141,48": one for the hunter site on
// (140, 49), admitted on its road carrier's walk in, and one for the site of
// counter 818, carried out by a helper. The one for the hunter site joined
// the list first, though the other lay at the flag first, and the road's
// carrier takes it (the recording sees its walk for it from trace tick 978;
// our claim is in the world after counter 978).
// spec: TRN-CARRY-2.4a (TRN T25), TRN-CARRY-2.4, TRN-CARRY-2.5
#[test]
fn trn_t25_the_ware_admitted_first_is_taken_first() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let path = [(140, 50), (141, 50), (141, 49), (141, 48)];
    let on_road = |world: &World, c: &s2_sim::transport::Carrier| {
        world.road(c.road).is_some_and(|r| {
            r.owner == P1 && (r.path == path || r.path.iter().rev().eq(path.iter()))
        })
    };
    // Trace tick 979, the world after counter 978: the road's carrier has
    // just claimed one of the two planks.
    while game.world.tick() < 979 {
        game.step();
    }
    let world = &game.world;
    let carrier = world
        .carriers()
        .iter()
        .position(|c| on_road(world, c))
        .expect("the road's carrier");
    let claimed = world
        .wares_at((140, 50))
        .iter()
        .copied()
        .find(|&w| world.ware(w).is_some_and(|w| w.claimed_by == Some(carrier)))
        .expect("a plank taken");
    let waiting = world.wares_at((140, 50));
    assert_eq!(waiting.len(), 2);
    // The first listed is the hunter site's plank, and it is the one taken,
    // though the other lay at the flag first.
    assert_eq!(waiting[0], claimed);
    let target = |w: usize| world.ware(w).and_then(|w| w.target).map(|t| t.at);
    assert_eq!(target(claimed), Some((140, 49)));
    assert_ne!(target(waiting[1]), Some((140, 49)));
}

// TRN T23 (TRN-ROAD-4.2) and TRN-FREE T25 (6.5 g): the new carrier for road
// "139,56 140,56 141,56 141,55 142,55" walks over another road towards
// (142, 55) when the stone pit's flag (141, 56) splits it (trace tick
// 1308): it gets the second half "142,55 141,55 141,56", its walk ends on
// (142, 55): walking at 1349, standing on the flag at 1350, its walk on
// ordered at progress 0 at 1351, stepping towards (141, 55) at 1352 and
// standing on that idle field from 1374.
// spec: TRN-ROAD-4.2 (TRN T23), TRN-FREE-6.5 (TRN-FREE T25)
#[test]
fn trn_t23_t25_the_carrier_of_a_split_road_stops_on_the_next_flag() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let path = [(142, 55), (141, 55), (141, 56)];
    let walker = |world: &World| {
        world
            .carriers()
            .iter()
            .find(|c| {
                world.road(c.road).is_some_and(|r| {
                    r.owner == P1 && (r.path == path || r.path.iter().rev().eq(path.iter()))
                })
            })
            .map(|c| c.walker.clone())
            .expect("the second half's carrier")
    };
    while game.world.tick() < 1349 {
        game.step();
    }
    let w = walker(&game.world);
    assert_eq!((w.at, w.next), ((142, 54), Some((142, 55))), "1349");
    game.step();
    let w = walker(&game.world);
    assert_eq!((w.at, w.next), ((142, 55), None), "1350");
    assert!(!w.has_walk(), "1350: standing");
    game.step();
    let w = walker(&game.world);
    assert!(
        w.has_walk() && w.next.is_none() && w.progress_bits == 0,
        "1351: ordered"
    );
    game.step();
    assert_eq!(walker(&game.world).next, Some((141, 55)), "1352");
    while game.world.tick() < 1374 {
        game.step();
    }
    let w = walker(&game.world);
    assert_eq!(
        (w.at, w.next, w.is_walking()),
        ((141, 55), None, false),
        "1374"
    );
}

// AI-CON T35 (AI-CON-8.4, Q-360): the street construction's run of counter
// 1485 on the stone pit's flag finds the harbour's flag (145, 46) at
// distance 10 over a detour of more than 10 steps and gives it up; the
// harbour's flag is handled from 1499 and its road built at counter 1541
// (with a 10-step limit everything came one run, 7 ticks, earlier).
// spec: AI-CON-8.4 (T35), AI-CON-8.1
#[test]
fn t35_the_harbours_road_comes_after_the_give_up() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let path = [
        (145, 46),
        (144, 45),
        (143, 45),
        (142, 45),
        (141, 45),
        (140, 45),
        (140, 46),
    ];
    while game.world.tick() < 1542 {
        assert!(!road_with(&game, &path), "built before counter 1541");
        game.step();
    }
    assert!(road_with(&game, &path), "built in counter 1541");
}

// TRN-FREE T26 (6.4 steps 1, 2; Q-361): carrier 90261 is sent home at
// counter 1541 while it walks into the hunter site (139, 47); it stands in
// the site and then free-walks, from progress 0 at trace tick 1561, to the
// network flag (139, 48) (made first among the two at distance 1, the
// site's own flag (140, 46) being the other), not over the roads from the
// site's flag.
// spec: TRN-FREE-6.4 (TRN-FREE T26), TRN-CARRY-7.1b
#[test]
fn trn_free_t26_a_carrier_in_a_site_walks_free_to_the_older_flag() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let settler = |world: &World| {
        world
            .settlers()
            .map(|(_, s)| s.clone())
            .find(|s| s.number == 90261)
    };
    while game.world.tick() < 1561 {
        game.step();
    }
    let s = settler(&game.world).expect("carrier 90261 going home");
    assert_eq!(s.walker.at, (139, 47));
    assert!(s.walker.next.is_none() && s.walker.progress_bits == 0);
    assert_eq!(
        s.walker.path.iter().copied().collect::<Vec<_>>(),
        [(139, 48)]
    );
}

// AI-CON T36, T37, T40, TRN-CARRY T26 (AI-CON-8.3 c, TRN-CARRY-5.3a; Q-362,
// 79e4711, 8d46644, 7927565): the route optimizer's task after every run
// from counter 993 to 6000 equals the
// original's (E-trace-010's ai_route table: cursor, k, i, t, w and X's
// count and last entry): the costs of c are stored costs, shared with the
// ware order matching and kept until a ware renumbering (T37: r = 10 at
// 2140, stored by the site's ware orders), the route variant's walk path
// counts with its road length L (T36: L 20 >= r 14 at 1622, no road), the
// direct variant keeps only its last Fn, and the route variant's flag list is
// searched from S's flag (T40: at 2621 Fn (139, 48), X two entries), and the
// storage choice fills the store too (TRN-CARRY T26: the stored 40 at 3509),
// and a guard house being upgraded counts as unfinished for its partner
// (T44: at 5840 k = 1 is done, the cursor goes on to 90301).
// spec: AI-CON-8.3 (T36, T37, T40, T44), TRN-CARRY-5.3a (TRN-CARRY T26), ECO-REQ-5.3, ECO-REQ-5.5, TRN-CARRY-5.2
#[test]
fn t36_t37_t40_t44_the_route_task_equals_the_recording_to_6000() {
    let Some(text) = recording_table("original-dense-6000-ai_route.csv.gz") else {
        return;
    };
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    // tick (the counter), slot, cursor, cursor_hi, k, i, t, w, x_count,
    // x_last_x, x_last_y
    let recorded: std::collections::BTreeMap<u64, Vec<i64>> = text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let v: Vec<i64> = line.split(',').map(|f| f.parse().unwrap()).collect();
            (v[1] == 1).then(|| (v[0] as u64, v))
        })
        .collect();
    let (first, last) = (993, 6000);
    while game.world.tick() <= last {
        game.step();
        let counter = game.world.tick() - 1;
        if counter < first || (counter + 6) % 37 != 0 {
            continue;
        }
        let r = &game.ai.player(P1).unwrap().street.route;
        let x_last =
            r.x.last()
                .map_or((-1, -1), |&(x, y)| (i64::from(x), i64::from(y)));
        let got = [
            i64::from(r.cursor),
            i64::from(r.k),
            r.i as i64,
            i64::from(r.t),
            r.w as i64,
            r.x.len() as i64,
            x_last.0,
            x_last.1,
        ];
        // The recorder missed a few ticks: the task changes only in runs,
        // so the first row before the next run stands in.
        let Some((_, v)) = recorded.range(counter..counter + 37).next() else {
            continue;
        };
        let want = [v[2], v[4], v[5], v[6], v[7], v[8], v[9], v[10]];
        assert_eq!(got, want, "the run of counter {counter}");
    }
}

// ECO-WORK T-door-3 (ECO-WORK-3.8c): hunter 90379, sent from the castle to
// the hunter's hut (140, 49), ends its road walk exactly on the hut's flag
// (141, 48) at trace tick 1815 (on the flag's sample, walking), the walk to
// the hut's field starting there at progress 0: at 1816 one tick along that
// step, (563945, 192108); standing on (140, 49) at 1853. With the excess
// running on it would be at (563969, 192060) at 1815 and arrive at 1852.
// spec: ECO-WORK-3.8c (T-door-3), TRN-MOVE-2.3
#[test]
fn t_door_3_a_worker_starts_afresh_on_its_buildings_flag() {
    use s2_sim::trace::Action::Stand;
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let row = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90379")
            .map(|r| (r.field, r.fine, r.action))
    };
    while game.world.tick() < 1815 {
        game.step();
    }
    let (field, fine, _) = row(&game).expect("hunter 90379");
    assert_eq!((field, fine), ((141, 48), (564000, 192000)), "1815");
    game.step();
    assert_eq!(row(&game).unwrap().1, (563945, 192108), "1816");
    while game.world.tick() < 1852 {
        game.step();
    }
    assert_ne!(row(&game).map(|r| r.2), Some(Stand), "1852: still walking");
    game.step();
    let (field, _, action) = row(&game).expect("on its field in 1853");
    assert_eq!((field, action), ((140, 49), Stand), "1853");
}

// ECO-OUT T18 (ECO-OUT-8.1, 3.7, TRN-FREE-5.1, 5.6, GFX-ANIM-3.8e): hunter
// 90379 after the animal on (140, 54): his free walk ends one field short on
// (140, 53), at its point, in trace tick 2121; the lattice walk of 2 steps
// towards (140, 54)'s point stops at (561000, 214000) in 2144, his field
// following his samples ((139, 53) from 2133); WorkLoop0 for 55 ticks from
// 2144; PutDown 2199 .. 2202, Pickup 2203 .. 2206; the fine walk back
// ordered in 2207 (walking, at the stand point) reaches (140, 53)'s point
// in 2230.
// spec: ECO-OUT-8.1 (ECO-OUT T18), ECO-OUT-3.7, TRN-FREE-5.1, TRN-FREE-5.6, GFX-ANIM-3.8e, MAP-GRID-4.3
#[test]
fn eco_out_t18_the_hunter_walks_to_two_lattice_steps_short() {
    use s2_sim::trace::Action::{Pickup, PutDown, Walk, WorkLoop};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let mut rows = std::collections::BTreeMap::new();
    while game.world.tick() < 2230 {
        game.step();
        if game.world.tick() >= 2121 {
            let row = game
                .world
                .trace_settlers()
                .into_iter()
                .find(|r| r.id == "90379")
                .expect("hunter 90379");
            rows.insert(game.world.tick(), (row.field, row.fine, row.action));
        }
    }
    assert_eq!(
        (rows[&2121].0, rows[&2121].1),
        ((140, 53), (562000, 212000))
    );
    assert_eq!(rows[&2132].0, (140, 53));
    assert_eq!(rows[&2133].0, (139, 53));
    assert_eq!(rows[&2144].1, (561000, 214000));
    assert_eq!(rows[&2144].2, WorkLoop(0));
    assert_eq!(rows[&2198].2, WorkLoop(0));
    assert_eq!((rows[&2199].2, rows[&2202].2), (PutDown, PutDown));
    assert_eq!((rows[&2203].2, rows[&2206].2), (Pickup, Pickup));
    assert_eq!((rows[&2207].1, rows[&2207].2), ((561000, 214000), Walk));
    assert_eq!(rows[&2230].1, (562000, 212000));
}

// ECO-REQ-1.3, ECO-HQ-4.3: the harbour site (144, 47), levelled by its
// leveller's last step in trace tick 2312, files its builder request in its
// next update: builder 90415 is out of the castle in 2313. ECO-OUT-3.9:
// hunter 90379 puts his ham down on his flag (141, 48) with PutDown from
// 2343, then Pickup from 2347, walking in from 2351.
// spec: ECO-REQ-1.3, ECO-HQ-4.3, ECO-OUT-3.9
#[test]
fn a_levelled_site_asks_for_its_builder_in_its_next_update() {
    use s2_sim::trace::Action::{Pickup, PutDown, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let settler = |game: &s2_ai::run::Runner, id: &str| {
        game.world.trace_settlers().into_iter().find(|r| r.id == id)
    };
    let levelled = |game: &s2_ai::run::Runner| {
        game.world
            .trace_buildings()
            .into_iter()
            .find(|b| b.field == (144, 47))
            .is_some_and(|b| b.state != "levelling")
    };
    while game.world.tick() < 2311 {
        game.step();
    }
    assert!(!levelled(&game), "2311");
    game.step();
    assert!(levelled(&game), "2312: levelled");
    assert!(settler(&game, "90415").is_none(), "2312: no builder yet");
    game.step();
    assert!(
        settler(&game, "90415").is_some(),
        "2313: the builder is out"
    );
    let mut actions = std::collections::BTreeMap::new();
    while game.world.tick() < 2351 {
        game.step();
        if game.world.tick() >= 2342 {
            let r = settler(&game, "90379").expect("hunter 90379");
            actions.insert(game.world.tick(), (r.field, r.action));
        }
    }
    assert_eq!(actions[&2343], ((141, 48), PutDown));
    assert_eq!(actions[&2346].1, PutDown);
    assert_eq!((actions[&2347].1, actions[&2350].1), (Pickup, Pickup));
    assert_eq!(actions[&2351].1, Walk);
}

// ECO-FOOD-5.7 (d42bcd9): the rabbit 90092 (our animal 178, as in T43),
// reserved by hunter 90379's hut in trace tick 2206 on its walk from
// (139, 54) to (146, 61), finishes that walk and stands on (146, 61) from
// 2348, with no wait.
// spec: ECO-FOOD-5.7
#[test]
fn a_reserved_animal_finishes_its_walk() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let rabbit = |game: &s2_ai::run::Runner| {
        game.world
            .animals()
            .find(|&(id, _)| id == 178)
            .map(|(_, a)| a.clone())
            .expect("the rabbit")
    };
    while game.world.tick() < 2206 {
        game.step();
    }
    let a = rabbit(&game);
    assert!(
        a.hunted_by.is_some() && a.walker.is_walking(),
        "2206: {a:?}"
    );
    assert_eq!(a.walker.destination(), (146, 61));
    while game.world.tick() < 2347 {
        game.step();
    }
    assert!(rabbit(&game).walker.is_walking(), "2347: still walking");
    game.step();
    let a = rabbit(&game);
    assert_eq!((a.at, a.walker.is_walking()), ((146, 61), false), "2348");
    assert_eq!(a.state, s2_sim::AnimalState::Hunted);
}

// ECO-HQ-4.7 (e9118a8), TRN-FREE-6.4a: builder 90292 at the sawmill
// (138, 49): his step began in trace tick 2427, the building is complete in
// 2480, he works on (WorkLoop0) in 2480 and 2481, to the end of the step's
// 55-tick action, stands (going home) in 2482 and walks back from 2483.
// spec: ECO-HQ-4.7, TRN-FREE-6.4a
#[test]
fn the_builder_works_to_the_end_of_his_step() {
    use s2_sim::trace::Action::{Stand, Walk, WorkLoop};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let finished = |game: &s2_ai::run::Runner| game.world.site((138, 49)).is_none();
    let action = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90292")
            .map(|r| r.action)
    };
    while game.world.tick() < 2479 {
        game.step();
    }
    assert!(!finished(&game), "2479");
    game.step();
    assert!(finished(&game), "2480: complete");
    assert_eq!(action(&game), Some(WorkLoop(0)), "2480");
    game.step();
    assert_eq!(action(&game), Some(WorkLoop(0)), "2481");
    game.step();
    assert_eq!(action(&game), Some(Stand), "2482");
    game.step();
    assert_eq!(action(&game), Some(Walk), "2483");
}

// TRN-CARRY-3.7 (e987c42): carrier 90282 brings a plank to (142, 55), its
// place reserved at counter 2386 before the woodcutter's wood was there; it
// arrives at 2501 and, as its put-down begins (counter 2502), claims the
// wood ready for the way back, taking it in the same actions, and walks back
// with it from trace tick 2511. TRN-CARRY-3.3a, ECO-WORK-3.8b: carrier 90260
// bringing wood into the castle is on (139, 51)'s point in trace tick 2529,
// on the door field in 2552, on (139, 51) again in 2576 and on the flag in
// 2599 (each door step a walk of its own).
// spec: TRN-CARRY-3.7, TRN-CARRY-3.3a, ECO-WORK-3.8b
#[test]
fn a_reserved_carrier_takes_a_ware_back_and_steps_through_the_door() {
    use s2_sim::transport::CarrierTask;
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let wood = *names
        .iter()
        .find(|(_, n)| n.as_str() == "wood")
        .expect("wood")
        .0;
    let carrier = |game: &s2_ai::run::Runner, number: u32| {
        game.world
            .carriers()
            .iter()
            .find(|c| c.number == number)
            .cloned()
            .expect("the carrier")
    };
    let point = |f: (u32, u32)| {
        let s = s2_sim::grid::field_sample(f);
        (i64::from(s.0) * 1000, i64::from(s.1) * 1000)
    };
    let fine = |game: &s2_ai::run::Runner, id: &str| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| r.fine)
    };
    while game.world.tick() < 2503 {
        game.step();
    }
    let c = carrier(&game, 90282);
    let CarrierTask::PutDown {
        exchange: Some(ware),
        bid: true,
        second: false,
        ..
    } = c.task
    else {
        panic!("counter 2502: {:?}", c.task);
    };
    assert_eq!(game.world.ware(ware).map(|w| w.good), Some(wood));
    while game.world.tick() < 2511 {
        game.step();
    }
    assert!(matches!(
        carrier(&game, 90282).task,
        CarrierTask::Carry { .. }
    ));
    // The castle's door field (TRN-CARRY-3.3a, E-trace-005).
    let (castle, door, flag) = ((139, 51), (139, 52), (140, 50));
    for (tick, at) in [(2529, castle), (2552, door), (2576, castle), (2599, flag)] {
        while game.world.tick() < tick {
            game.step();
        }
        assert_eq!(fine(&game, "90260"), Some(point(at)), "trace tick {tick}");
    }
}

// ECO-OUT-3.10 (4a5d991): an outdoor worker's next cycle starts in the tick
// his rest is over, which is the first tick of the 51-tick wait: woodcutter
// 90392 waits from trace tick 2643 and walks out in 2694; hunter 90379 waits
// from 2613 and walks out in 2664.
// spec: ECO-OUT-3.10, ECO-OUT-3.2
#[test]
fn the_wait_begins_in_the_tick_the_rest_is_over() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let job = |game: &s2_ai::run::Runner, id: &str| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| r.job)
            .unwrap_or_default()
    };
    let mut seen: std::collections::BTreeMap<(&str, &str), u64> = Default::default();
    while game.world.tick() < 2700 {
        game.step();
        let tick = game.world.tick();
        for id in ["90379", "90392"] {
            let j = job(&game, id);
            for (key, prefix) in [("wait", "Worker.WaitTarget"), ("out", "Worker.Out")] {
                if tick > 2600 && j.starts_with(prefix) {
                    seen.entry((id, key)).or_insert(tick);
                }
            }
        }
    }
    assert_eq!(seen[&("90379", "wait")], 2613);
    assert_eq!(seen[&("90379", "out")], 2664);
    assert_eq!(seen[&("90392", "wait")], 2643);
    assert_eq!(seen[&("90392", "out")], 2694);
}

// TRN-CARRY-2.2 (corrected, 03543ac): carrier 90260 leaves the flag
// (141, 48) after a put-down at counter 2684 and walks back to its idle field
// (141, 49) without looking for work, though a stone is ready for its road at
// the other flag from 2703; it stands on the idle field from 2707 and claims
// the stone at 2720, the first tick of its 20-tick grid.
// spec: TRN-CARRY-2.2, TRN-CARRY-2.1
#[test]
fn a_carrier_walking_back_to_its_idle_field_does_not_look() {
    use s2_sim::transport::CarrierTask;
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let carrier = |game: &s2_ai::run::Runner| {
        game.world
            .carriers()
            .iter()
            .find(|c| c.number == 90260)
            .cloned()
            .expect("carrier 90260")
    };
    // The world after counter c is world tick c + 1.
    while game.world.tick() < 2708 {
        game.step();
    }
    let c = carrier(&game);
    assert_eq!(
        (c.walker.at, c.walker.next, c.task),
        ((141, 49), None, CarrierTask::Idle),
        "2707"
    );
    while game.world.tick() < 2721 {
        assert_eq!(
            carrier(&game).task,
            CarrierTask::Idle,
            "{}",
            game.world.tick() - 1
        );
        game.step();
    }
    assert!(
        matches!(carrier(&game).task, CarrierTask::Fetch { .. }),
        "2720: {:?}",
        carrier(&game).task
    );
}

// GFX-ANIM-3.1a, ECO-OUT-3.8, GFX-ANIM-3.8e (c1070f8): woodcutter 90392's
// tree is next to his flag (142, 55): his walk to it has no field, yet he
// walks on in trace tick 2717 (the clip running into the fine approach);
// after the take he stands in 2826 and the fine walk back is ordered in 2827
// (walking at its start point), one tick later than when the walk out had a
// field.
// spec: GFX-ANIM-3.1a, ECO-OUT-3.8, GFX-ANIM-3.8e
#[test]
fn a_walk_out_without_a_field_still_walks_and_delays_the_way_back() {
    use s2_sim::trace::Action::{Pickup, Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let mut rows = std::collections::BTreeMap::new();
    while game.world.tick() < 2827 {
        game.step();
        let t = game.world.tick();
        if t == 2717 || (2825..=2827).contains(&t) {
            let r = game
                .world
                .trace_settlers()
                .into_iter()
                .find(|r| r.id == "90392")
                .expect("woodcutter 90392");
            rows.insert(t, (r.field, r.fine, r.action));
        }
    }
    assert_eq!(rows[&2717], ((142, 55), (570000, 220000), Walk));
    assert_eq!(rows[&2825].2, Pickup);
    assert_eq!(rows[&2826], ((142, 55), (573000, 221000), Stand));
    assert_eq!(rows[&2827], ((142, 55), (573000, 221000), Walk));
}

// ECO-REQ-0.2, TRACE-4.4 (12860b3): the workers are updated before the road
// carriers, so the sawmill's worker 90431 sees the wood carrier 90234 hands
// over in counter 3167 only in 3168 and starts working then; his work inside
// is recorded as WorkLoop0 from trace tick 3169.
// spec: ECO-REQ-0.2, TOOL-trace TRACE-4.4
#[test]
fn a_worker_sees_a_ware_handed_over_in_the_next_tick() {
    use s2_sim::trace::Action::WorkLoop;
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let row = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90431")
            .map(|r| (r.job, r.action))
    };
    while game.world.tick() < 3168 {
        game.step();
    }
    let (job, _) = row(&game).expect("worker 90431");
    assert!(!job.contains("Working"), "counter 3167: {job}");
    game.step();
    let (job, action) = row(&game).expect("worker 90431");
    assert!(job.contains("Working"), "counter 3168: {job}");
    assert_eq!(action, WorkLoop(0), "trace tick 3169");
}

// AI-CON-4.2 (Q-178): ties of the needs construction go to the first good
// in id order read as a signed number: water (0xa9676263) before corn
// (0x17626a03) at 346.5 for player 1 at counter 3208, so the well on
// (136, 53) is placed at counter 3443, before the farm.
// spec: AI-CON-4.2
#[test]
fn the_well_comes_before_the_farm() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let id = |name: &str| {
        *names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .expect(name)
            .0
    };
    let (well, farm) = (id("spring"), id("farm"));
    let mut placed = Vec::new();
    let mut seen: std::collections::BTreeSet<(u32, u32)> =
        game.world.buildings().map(|b| b.at).collect();
    while game.world.tick() <= 3444 {
        game.step();
        let k = game.world.tick() - 1;
        for b in game.world.buildings() {
            if b.owner == P1 && seen.insert(b.at) && [well, farm].contains(&b.building_type) {
                placed.push((k, b.building_type, b.at));
            }
        }
    }
    assert_eq!(placed.first(), Some(&(3443, well, (136, 53))), "{placed:?}");
    assert!(placed.iter().all(|&(_, t, _)| t != farm), "{placed:?}");
}

// ECO-WORK-3.4 (77ed505): the sawmill's worker 90431 ends his take at
// counter 3508, stands in 3509 and 3510 and, with room at the flag, has the
// plank's place granted and walks out in 3511. ECO-WORK-3.10: after his rest
// he waits from trace tick 3624 and works from 3625.
// spec: ECO-WORK-3.4, ECO-WORK-3.10
#[test]
fn a_workshop_worker_stands_two_ticks_before_walking_out() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let job = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90431")
            .map(|r| r.job)
            .unwrap_or_default()
    };
    let mut jobs = std::collections::BTreeMap::new();
    while game.world.tick() < 3626 {
        game.step();
        let counter = game.world.tick() - 1;
        if (3508..=3511).contains(&counter) || (3623..=3625).contains(&(counter + 1)) {
            jobs.insert(game.world.tick(), job(&game));
        }
    }
    assert!(jobs[&3509].starts_with("Worker.Take"), "{jobs:?}");
    assert!(jobs[&3510].starts_with("Worker.WaitRoom"), "{jobs:?}");
    assert!(jobs[&3511].starts_with("Worker.WaitRoom"), "{jobs:?}");
    assert!(jobs[&3512].starts_with("Worker.Out"), "{jobs:?}");
    assert!(!jobs[&3624].starts_with("Worker.Working"), "{jobs:?}");
    assert!(jobs[&3625].starts_with("Worker.Working"), "{jobs:?}");
}

// ECO-DIST-1.6 (9d69dd5, 8b639a2): the stone 90433, put down on (141, 48) at
// counter 3019 with next flag (145, 46), is counted by the re-checks of 3021
// to 3618 (1 to 200, the test before the rise) and computed again at 3621,
// to (140, 46), its count starting at 0 (8 at 3647).
// spec: ECO-DIST-1.6
#[test]
fn a_lying_wares_route_is_computed_again_after_201_checks() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let stone = *names
        .iter()
        .find(|(_, n)| n.as_str() == "stone")
        .expect("stone")
        .0;
    let lying = |game: &s2_ai::run::Runner| {
        game.world
            .wares()
            .find(|(_, w)| w.good == stone && w.location == s2_sim::WareLocation::Flag((141, 48)))
            .map(|(id, w)| (id, w.next, w.recheck))
    };
    // The world after counter c is world tick c + 1.
    while game.world.tick() < 3621 {
        game.step();
    }
    let (id, next, count) = lying(&game).expect("the stone at (141, 48)");
    assert_eq!((next, count), (Some((145, 46)), 200), "counter 3620");
    game.step();
    let w = game.world.ware(id).expect("the stone");
    assert_eq!((w.next, w.recheck), (Some((140, 46)), 0), "counter 3621");
    while game.world.tick() < 3648 {
        game.step();
    }
    assert_eq!(
        game.world.ware(id).map(|w| w.recheck),
        Some(8),
        "counter 3647"
    );
}

// AI-CON T38 (AI-CON-6.3, 5cf438d): the farm order of counter 3695 in cell
// (8, 3): (135, 57) has trees on three fields of its ring at distance 2
// (9 x 0.4 = 3.6, score 2.925), (136, 58) none (12 x 0.4 = 4.8, score 3.9):
// the farm is placed on (136, 58) in trace tick 3725.
// spec: AI-CON-6.3 (T38), AI-CON-6.4
#[test]
fn t38_the_farm_counts_the_ring_at_distance_two() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let farm = *names
        .iter()
        .find(|(_, n)| n.as_str() == "farm")
        .expect("farm")
        .0;
    while game.world.tick() < 3725 {
        game.step();
    }
    let site = game.world.building_at((136, 58)).expect("the farm site");
    assert_eq!((site.owner, site.building_type), (P1, farm));
    assert!(game.world.building_at((135, 57)).is_none());
}

// AI-CON T39 (AI-CON-6.2, 6.4, 94d1a13): the brewery order of counter 3945 in
// cell (8, 3): (136, 56) is 2 from the farm site (136, 58), so v = 0 (farm
// sites count); (136, 51) has water 13 (the well site (136, 53)) + grain 8
// (the farm site) = 21, route 4, score 15.75: the brewery is placed on
// (136, 51) in trace tick 3987.
// spec: AI-CON-6.2 (T39), AI-CON-6.4
#[test]
fn t39_sites_count_in_the_supplier_value_and_the_farm_penalty() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let brewery = *names
        .iter()
        .find(|(_, n)| n.as_str() == "brewery")
        .expect("brewery")
        .0;
    while game.world.tick() < 3987 {
        game.step();
    }
    let site = game.world.building_at((136, 51)).expect("the brewery site");
    assert_eq!((site.owner, site.building_type), (P1, brewery));
    assert!(game.world.building_at((136, 56)).is_none());
}

// TRN-FREE T27 (TRN-FREE-5.6 step 3, afcf1a8): woodcutter 90478 (hut
// (136, 47), flag (137, 46)) ends his approach on the point of (136, 45);
// after the take (ending 4000) he stands in 4001 and, standing exactly on
// that field's point, walks to the flag from that field: the free walk
// ordered in 4002, first moving in 4003 to (546090, 180180), on the flag in
// 4025 (not the 4 lattice steps of a fine walk back).
// spec: TRN-FREE-5.6 (TRN-FREE T27), ECO-OUT-3.8
#[test]
fn trn_free_t27_from_the_fields_point_straight_to_the_flag() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let mut rows = std::collections::BTreeMap::new();
    let mut id = None;
    while game.world.tick() < 4025 {
        game.step();
        let t = game.world.tick();
        if t == 4001 {
            // Found by place (his number follows the game's numbering).
            id = game
                .world
                .trace_settlers()
                .into_iter()
                .find(|r| r.field == (136, 45) && r.job.starts_with("Worker."))
                .map(|r| r.id);
        }
        if (4001..=4003).contains(&t) || t == 4025 {
            let r = game
                .world
                .trace_settlers()
                .into_iter()
                .find(|r| Some(&r.id) == id.as_ref())
                .expect("woodcutter 90478");
            rows.insert(t, (r.field, r.fine, r.action));
        }
    }
    assert_eq!(rows[&4001], ((136, 45), (546000, 180000), Stand));
    assert_eq!(rows[&4002], ((136, 45), (546000, 180000), Walk));
    assert_eq!(rows[&4003].1, (546090, 180180));
    assert_eq!(rows[&4025], ((137, 46), (548000, 184000), Stand));
}

// ECO-FOOD T45 (ECO-FOOD-5.16, 3ede553): the animal 90072 waits on
// (136, 59) when the computer player places its farm site on (136, 58) at
// counter 3724; (136, 59) is the large site's north-east field, so the animal
// is removed in that counter: 275 animals before, 274 after.
// spec: ECO-FOOD-5.16 (ECO-FOOD T45)
#[test]
fn eco_food_t45_the_farm_site_removes_the_animal_under_it() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    // The world after counter c is world tick c + 1.
    while game.world.tick() < 3724 {
        game.step();
    }
    assert_eq!(game.world.animals().count(), 275, "counter 3723");
    assert!(game.world.animals().any(|(_, a)| a.at == (136, 59)));
    game.step();
    assert_eq!(game.world.animals().count(), 274, "counter 3724");
    assert!(!game.world.animals().any(|(_, a)| a.at == (136, 59)));
    assert!(game.world.building_at((136, 58)).is_some());
}

// TRN-ROAD T25 (TRN-ROAD-4.2, ware in transit; 82d505f): carrier 90282 of
// road "142,55 142,54 141,53 141,52 140,51 140,50" carries a ware from
// (140, 50) to (142, 55) when the computer player's building splits the road
// with the flag (141, 52) at counter 4334, in his step (141, 53) ->
// (142, 54). He gets the first half and walks on without a pause, his entry
// check at (142, 55) going on where it was: (568154, 216308) at trace tick
// 4335, walking on at 4336, on (142, 55) at 4356 (not stopped on (142, 54)
// for a new entry check, not put back on its point).
// spec: TRN-ROAD-4.2 (TRN-ROAD T25), TRN-CARRY-3.2
#[test]
fn trn_road_t25_a_carrier_with_a_ware_walks_on_through_the_split() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let row = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90282")
            .map(|r| (r.field, r.fine))
    };
    while game.world.tick() < 4335 {
        game.step();
    }
    let (_, fine) = row(&game).expect("carrier 90282");
    assert_eq!(fine, (568154, 216308), "4335");
    let mut last = fine;
    while game.world.tick() < 4356 {
        game.step();
        let (_, fine) = row(&game).unwrap();
        assert_ne!(fine, last, "walking at {}", game.world.tick());
        last = fine;
    }
    assert_eq!(row(&game).unwrap().0, (142, 55), "4356");
}

// SIM-START-4.2 to 4.4, ECO-OUT-3.6, MIL-CON-4.4, AI-CON-7.2 (8d46644,
// 190c1e1, fed3ea5, 8309d7d): the object counter after every counter
// equals the original's (E-trace-010's counter table) through counter 5999,
// the table's end:
// every object, record and number the game takes, including a smaller
// stone pile's new deposit (89900 -> 90524 at counter 3589, its stone 90525
// at 3593), the records of the computer player's own sites, numbered by the
// placement (T13: building 90590, its records 90591 and 90592 at counter
// 4334; before, they came one counter late at 818, 2703, 3198, 3443 and
// 4334), the remains of the hut torn down at 4791 (plank 90644, ruin 90645)
// and the next hunter's order (T42), the castle's helper and plank of 5225
// (TRN-CARRY-4.2a), the soldier of 5502 and the guard house's upgrade
// request and leveller of 5582 (AI-MIL-4.5a), the woodcutter's remains of
// 5816 (AI-CON-9.2) and the guard house site of 5898 (Q-365).
// spec: SIM-START-4.2, SIM-START-4.4 (T13), ECO-OUT-3.6, MIL-CON-4.4, AI-CON-7.2, TRN-CARRY-4.2a, AI-MIL-4.5a, AI-CON-9.2, AI-CON-3.5
#[test]
fn the_object_numbers_follow_the_recording() {
    let Some(text) = recording_table("original-dense-6000-counter.csv.gz") else {
        return;
    };
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    // tick (the counter), snap, next_id, ...
    let recorded: std::collections::BTreeMap<u64, u32> = text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let v: Vec<&str> = line.split(',').collect();
            let counter: i64 = v[0].parse().ok()?;
            (counter >= 0).then(|| (counter as u64, v[2].parse().unwrap()))
        })
        .collect();
    let last = 5999;
    while game.world.tick() <= last {
        game.step();
        let counter = game.world.tick() - 1;
        if let Some(&next) = recorded.get(&counter) {
            assert_eq!(game.world.next_object(), next, "after counter {counter}");
        }
    }
}

// AI-CON T41 (AI-2.6, AI-CON-9.2, 9.3, 6.3), MIL-CON T17, TRN-FREE T28: the
// destruction cursor holds the last handled id + 1 (90212 after counter 3500,
// 90181 after 4000 behind the reset run of 3930, 90539 after 4500); at 4094
// the hunter's hut 90244 on (139, 47) is kept (its hunter is out); at 4791
// its hunter rests inside and no unreserved animal is within 8, so it is
// torn down with remains: the plank 90644 on its flag (140, 46), the ruin
// 90645. Its hunter 90425 stands from trace tick 4793 (T, the tick after
// the removal) and walks free towards (139, 48) (made before (140, 46)) at
// progress 0 in 4796, at (557945, 188108) in 4797.
// spec: AI-2.6, AI-CON-9.2, AI-CON-9.3, AI-CON-6.3 (AI-CON T41), MIL-CON-4.4 (MIL-CON T17), TRN-FREE-6.4a (TRN-FREE T28)
#[test]
fn ai_con_t41_the_hunters_hut_is_torn_down_with_remains() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let plank_good = *names.iter().find(|(_, n)| n.as_str() == "plank").unwrap().0;
    let hut = (139, 47);
    for (counter, cursor) in [(3500, 90212), (4000, 90181), (4500, 90539)] {
        while game.world.tick() <= counter {
            game.step();
            if game.world.tick() == 4095 {
                assert!(game.world.building_at(hut).is_some(), "kept at 4094");
            }
        }
        let p = game.ai.player(P1).unwrap();
        assert_eq!(p.destruction_cursor, cursor, "after counter {counter}");
    }
    while game.world.tick() < 4791 {
        game.step();
    }
    assert!(game.world.building_at(hut).is_some());
    let before = game.world.next_object();
    game.step();
    assert!(game.world.building_at(hut).is_none(), "torn down at 4791");
    assert!(game.world.ruins().iter().any(|r| r.at == hut));
    let plank = game
        .world
        .wares_at((140, 46))
        .iter()
        .filter(|&&w| game.world.ware(w).is_some_and(|w| w.good == plank_good))
        .count();
    assert_eq!(plank, 1);
    assert_eq!(
        game.world.next_object(),
        before + 2,
        "plank 90644, ruin 90645"
    );
    let row = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90425")
            .map(|r| (r.fine, r.action))
            .expect("hunter 90425")
    };
    use s2_sim::trace::Action::{Stand, Walk};
    assert_eq!(row(&game).1, Stand, "4792");
    while game.world.tick() < 4795 {
        game.step();
        assert_eq!(
            row(&game),
            ((558000, 188000), Stand),
            "{}",
            game.world.tick()
        );
    }
    game.step();
    assert_eq!(row(&game), ((558000, 188000), Walk), "4796");
    game.step();
    assert_eq!(row(&game), ((557945, 188108), Walk), "4797");
}

// ECO-FOOD T46 (5.7, 5.11): the animal 90061 (home (142, 42)) was reserved
// by the hunter's hut 90244 during its walk (142, 39) -> (142, 40); the hut
// is torn down at counter 4791 and the animal, released, walks on with its
// walk's start time: walking at trace tick 4792, on (142, 40) and waiting
// 135 from 4793 (5.11 from the start 474.71976 s, idle factor 3).
// spec: ECO-FOOD-5.7, ECO-FOOD-5.11 (ECO-FOOD T46)
#[test]
fn eco_food_t46_a_released_animal_finishes_its_walk() {
    use s2_sim::AnimalState;
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let animal = |game: &s2_ai::run::Runner| {
        game.world
            .animals()
            .find(|(_, a)| a.home == (142, 42) && (a.at == (142, 39) || a.at == (142, 40)))
            .map(|(_, a)| a.clone())
            .expect("the animal 90061")
    };
    while game.world.tick() < 4792 {
        game.step();
    }
    let a = animal(&game);
    assert!(a.hunted_by.is_none(), "released at 4791");
    assert!(
        matches!(a.state, AnimalState::Walking { .. }),
        "4792: {:?}",
        a.state
    );
    game.step();
    let a = animal(&game);
    assert_eq!(
        (a.at, a.state),
        ((142, 40), AnimalState::Waiting { left: 135 }),
        "4793"
    );
}

// TRN-CARRY T27 (3.3, 3.3a; 9d3c62b): carrier 90554 brings a ware into the
// computer player's farm site (136, 58) (door field (135, 59)): on the
// site's point in trace tick 4994 and walking on at once (the door step at
// progress 0), on the door field (542000, 236000) in 5017, standing there in
// 5017 only, walking back at progress 0 in 5018.
// spec: TRN-CARRY-3.3a (TRN-CARRY T27), TRN-CARRY-3.3, ECO-WORK-3.8b
#[test]
fn trn_carry_t27_into_a_site_with_a_door_on_to_its_door_field() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let row = |game: &s2_ai::run::Runner| {
        game.world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90554")
            .map(|r| (r.fine, r.action))
            .expect("carrier 90554")
    };
    while game.world.tick() < 4994 {
        game.step();
    }
    assert_eq!(row(&game), ((544000, 232000), Walk), "4994");
    while game.world.tick() < 5017 {
        game.step();
    }
    assert_eq!(row(&game), ((542000, 236000), Stand), "5017");
    game.step();
    assert_eq!(row(&game), ((542000, 236000), Walk), "5018");
}

// AI-CON T42 (7.2, Q-363): the hunter run counts a cell's animals by home
// field, explored or not, so after the hut 90244 is torn down (4791) a
// hunter order for cell (9, 2) fails with no site, the locksmithery's site
// (135, 48) comes at trace tick 5082, the hunter's for cell (8, 2) at
// (141, 46) in 5389 (not on the ruin's field (139, 47) in 5038) and the
// sawmill's (136, 45) in 5435, as recorded.
// spec: AI-CON-7.2 (AI-CON T42), ECO-FOOD-5.15
#[test]
fn ai_con_t42_the_next_hunter_after_the_tear_down() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let sites = [((135, 48), 5082), ((141, 46), 5389), ((136, 45), 5435)];
    while game.world.tick() < 5435 {
        game.step();
        let tick = game.world.tick();
        if tick > 4792 {
            assert!(game.world.building_at((139, 47)).is_none(), "{tick}");
        }
        for (at, from) in sites {
            assert_eq!(
                game.world.building_at(at).is_some(),
                tick >= from,
                "{at:?} in {tick}"
            );
        }
    }
}

// ECO-DIST T21 (ECO-REQ-5.3d, ECO-DIST-3.3b): player 1's plank orders in
// the castle's network at the matchings of counters 4917, 4928 and 4939.
// The castle's send cooldown runs and the plank 90660 lies free at
// (139, 48), storage-bound; it is dearer for the site 90590 (140, 53) than
// the castle. 4917 and 4928 choose and mark the site's two new orders
// (stamp 0) and send nothing; at 4939 the hunter site (144, 58), chosen
// once before, has the lowest stamp and the plank is redirected to it.
// spec: ECO-REQ-5.3d, ECO-DIST-3.3b (ECO-DIST T21), ECO-DIST-3.3, ECO-REQ-5.3
#[test]
fn eco_dist_t21_the_plank_goes_to_the_order_that_waited_longest() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let plank = names.iter().find(|(_, n)| *n == "plank").map(|(&id, _)| id);
    let plank = plank.expect("the plank good");
    let stamps = |world: &World, at: (u32, u32)| -> Vec<u64> {
        world
            .orders()
            .iter()
            .filter(|o| o.player == P1 && o.good == plank && o.target.at == at)
            .map(|o| o.stamp)
            .collect()
    };
    let lying = |world: &World| -> Vec<Option<(u32, u32)>> {
        world
            .wares_at((139, 48))
            .iter()
            .filter_map(|&w| world.ware(w))
            .filter(|w| w.good == plank)
            .map(|w| w.target.filter(|t| !t.storage).map(|t| t.at))
            .collect()
    };
    // Trace tick 4918, the world after counter 4917.
    while game.world.tick() < 4918 {
        game.step();
    }
    assert_eq!(lying(&game.world), vec![None], "the free plank");
    assert_eq!(stamps(&game.world, (140, 53)), vec![4917, 0]);
    while game.world.tick() < 4929 {
        game.step();
    }
    assert_eq!(lying(&game.world), vec![None], "nothing for the site");
    assert_eq!(stamps(&game.world, (140, 53)), vec![4917, 4928]);
    let hunter = stamps(&game.world, (144, 58));
    assert!(hunter.iter().any(|&s| s > 0 && s < 4917), "{hunter:?}");
    while game.world.tick() < 4940 {
        game.step();
    }
    assert_eq!(lying(&game.world), vec![Some((144, 58))]);
}

// TRN-CARRY T28 (TRN-CARRY-4.2a): at the castle's flag (140, 50) four wares
// lie with next flag (141, 52) when the castle's helper 90696 is cleared
// with a plank for the brewery site (136, 51); carrier 90234, waiting on
// (139, 49) since trace tick 5020 with plank 90660 for the hunter site
// (144, 58), is refused the 6th place (the helper's plank passes no
// direction test), and the castle, counting five, sends plank 90701 for the
// brewery site in the matching of counter 5225 (helper 90700 at its door
// in trace tick 5226).
// spec: TRN-CARRY-4.2a (TRN-CARRY T28), TRN-CARRY-4.2, ECO-REQ-5.3 (b)
#[test]
fn trn_carry_t28_the_castle_sends_while_the_carrier_waits() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    // Its trace row (standing on the field's sample) and its ware's target.
    let waiting = |world: &World| {
        let c = world
            .carriers()
            .iter()
            .find(|c| c.number == 90234)
            .expect("carrier 90234");
        let target = c.ware.and_then(|w| world.ware(w)).and_then(|w| w.target);
        let row = world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90234")
            .expect("its row");
        (row.field, row.fine, row.action, target.map(|t| t.at))
    };
    let stand = (
        (139, 49),
        (558000, 196000),
        s2_sim::trace::Action::Stand,
        Some((144, 58)),
    );
    while game.world.tick() < 5020 {
        game.step();
    }
    while game.world.tick() < 5226 {
        assert_eq!(waiting(&game.world), stand, "{}", game.world.tick());
        game.step();
    }
    let helper = game
        .world
        .settlers()
        .find(|(_, s)| s.number == 90700)
        .map(|(_, s)| s.clone())
        .expect("helper 90700");
    assert_eq!(helper.walker.at, (139, 52));
    assert!(helper.carrying.is_some());
    let SettlerJob::CarryOut { target, .. } = helper.job else {
        panic!("{:?}", helper.job);
    };
    assert_eq!(target.map(|t| t.at), Some((136, 51)));
    assert_eq!(waiting(&game.world), stand);
}

// AI-MIL T15 (AI-MIL-4.5a): the world's military list is [player 0's
// castle, player 1's castle, player 1's guard house (138, 57)], the guard
// house finished at counter 5501. The upgrade runs of 5508 and 5545 take
// the two castles and do nothing; the run of 5582 takes the guard house
// and upgrades it.
// spec: AI-MIL-4.5a (AI-MIL T15), AI-MIL-4.5
#[test]
fn ai_mil_t15_the_guard_house_is_upgraded_in_5582() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() < 5502 {
        game.step();
    }
    let guard_house = game.world.building_at((138, 57)).expect("the guard house");
    assert!(!guard_house.under_construction);
    while game.world.tick() < 5581 {
        game.step();
        assert_eq!(
            game.world.upgrading((138, 57)),
            None,
            "{}",
            game.world.tick()
        );
    }
    // The run at the end of trace tick 5582 upgrades it; its request and
    // leveller take the numbers 90749 and 90750 in counter 5582, and the
    // leveller walks out in trace tick 5583.
    game.step();
    assert!(game.world.upgrading((138, 57)).is_some());
    assert_eq!(game.world.next_object(), 90749);
    game.step();
    assert_eq!(game.world.next_object(), 90751);
}

// MIL-SOL-4.7a: soldier 90744, made at counter 5502 for the guard house
// (138, 57), stands on the castle's door field point in trace tick 5503
// and takes its first step in 5504; its road walk ends on the flag point of
// (139, 56) in 5755, the step inside starts there at progress 0, and it
// stands in the building from 5783 (E-trace-010).
// spec: MIL-SOL-4.7a, MIL-SOL-4.7, ECO-WORK-3.8c
#[test]
fn mil_sol_the_soldier_stands_first_and_stops_on_the_flag() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let expected = [
        (5503, (139, 52), (556000, 208000), Walk),
        (5504, (139, 52), (556090, 207819), Walk),
        (5755, (139, 56), (556000, 224000), Walk),
        (5756, (139, 56), (555927, 224144), Walk),
        (5782, (139, 56), (554055, 227888), Walk),
        (5783, (138, 57), (554000, 228000), Stand),
    ];
    for (tick, field, fine, action) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90744")
            .expect("soldier 90744");
        assert_eq!(
            (row.field, row.fine, row.action),
            (field, fine, action),
            "{tick}"
        );
    }
}

// ECO-WORK T-arrive-1 (ECO-WORK-3.1a): the spring's worker 90699 arrives
// inside (136, 53) in trace tick 5558 and stands in 5558 and 5559; its
// first check comes in 5560, where its work begins (no inputs needed).
// spec: ECO-WORK-3.1a (ECO-WORK T-arrive-1), ECO-WORK-3.1
#[test]
fn eco_work_t_arrive_1_the_springs_worker_starts_two_ticks_after_arriving() {
    use s2_sim::trace::Action::{Stand, Walk, WorkLoop};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let mut rows = Vec::new();
    while game.world.tick() < 5560 {
        game.step();
        if game.world.tick() >= 5557 {
            let row = game
                .world
                .trace_settlers()
                .into_iter()
                .find(|r| r.id == "90699")
                .expect("worker 90699");
            rows.push((game.world.tick(), row.field, row.action));
        }
    }
    let inside = (136, 53);
    assert_eq!(
        rows,
        [
            (5557, (137, 52), Walk),
            (5558, inside, Stand),
            (5559, inside, Stand),
            (5560, inside, WorkLoop(0)),
        ]
    );
}

// ECO-REQ T17 (ECO-REQ-5.3e): carrier 90399 of road (137, 46) .. (139, 48)
// walks to fetch the wood lying at (137, 46) for storage from counter 5761;
// the matching redirects it to the sawmill (138, 49) on the way, clearing
// its next flag. On the flag in trace tick 5807 he stands; his pick-up
// starts in 5808 (PutDown, then Pickup) and he walks off with the wood in
// 5816.
// spec: ECO-REQ-5.3e (ECO-REQ T17), ECO-DIST-1.6, TRN-CARRY-2.5
#[test]
fn eco_req_t17_the_carrier_stands_at_the_redirected_ware() {
    use s2_sim::trace::Action::{Pickup, PutDown, Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let flag = (137, 46);
    let mut rows = Vec::new();
    while game.world.tick() < 5816 {
        game.step();
        if game.world.tick() >= 5807 {
            let row = game
                .world
                .trace_settlers()
                .into_iter()
                .find(|r| r.id == "90399")
                .expect("carrier 90399");
            assert_eq!(row.field, flag);
            rows.push(row.action);
        }
    }
    assert_eq!(
        rows,
        [Stand, PutDown, PutDown, PutDown, PutDown, Pickup, Pickup, Pickup, Pickup, Walk]
    );
}

// AI-CON T43 (AI-CON-9.2): the destruction run of counter 5816 takes the
// woodcutter's hut 90383 on (136, 47), its woodcutter at home and no tree in
// reach (v = 0); its cell has one wood production but no forester, so it is
// torn down with remains: plank 90780 on its flag (137, 46), ruin 90781.
// spec: AI-CON-9.2 (AI-CON T43), AI-CON-9.3, MIL-CON-4.4
#[test]
fn ai_con_t43_a_woodcutter_without_a_forester_is_torn_down() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let hut = (136, 47);
    while game.world.tick() < 5816 {
        game.step();
    }
    assert_eq!(game.world.building_at(hut).map(|b| b.serial), Some(90383));
    // Trace tick 5817, the world after counter 5816.
    game.step();
    assert!(game.world.building_at(hut).is_none());
    assert_eq!(game.world.next_object(), 90782);
    assert_eq!(
        game.world.wares_at((137, 46)).len(),
        1,
        "the remains' plank"
    );
}

// Q-365 (AI-CON-1.1c, 1.4, 2.5a, 3.2, 3.5; AI-MIL T16) and Q-278: the coal
// need waits on its food input until the bakery is placed (bread supplies
// the group good food; the hunters' huts have no productions), then waits
// for expansion; the cell system's run of counter 4847 makes the coal
// target (116, 48), as the heap dumps hold (the resource map counts
// unexplored fields; the cell-coordinate entries, V by the coordinate's own
// square), which stays in its slot until the expansion system takes it at
// 5330. The military run then orders a guard house whose site (131, 59) is
// placed at counter 5898 (building 90785).
// spec: AI-CON-1.1c, AI-CON-2.5a, AI-CON-3.2, AI-CON-3.5, AI-MIL-4.2 (AI-MIL T16)
#[test]
fn q365_q278_the_coal_target_and_the_guard_house_of_5898() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let target = |t: Option<s2_ai::expansion::Target>| t.map(|t| (t.at, t.since));
    let pending =
        |game: &s2_ai::run::Runner| target(game.ai.player(P1).unwrap().expansion.pending[0]);
    while game.world.tick() < 4847 {
        game.step();
    }
    assert_eq!(pending(&game), None);
    game.step();
    let coal = Some(((116, 48), 4847));
    assert_eq!(pending(&game), coal, "made in the run of 4847");
    while game.world.tick() < 5331 {
        game.step();
        assert_eq!(pending(&game), coal, "kept in its slot");
    }
    let taken = game.ai.player(P1).unwrap().expansion.taken.map(|(_, t)| t);
    assert_eq!(target(taken), coal, "taken at 5330");
    while game.world.tick() < 5898 {
        game.step();
        assert!(game.world.building_at((131, 59)).is_none());
    }
    game.step();
    let site = game
        .world
        .building_at((131, 59))
        .expect("the guard house site");
    assert_eq!(site.serial, 90785);
}

// MIL-CON-4.1a: the ruin 90781 of the woodcutter's hut torn down at counter
// 5816 burns on (136, 47) until 6149, its field Blocked; the computer
// player's site search for a spring in cell (8, 2), taken at 6094, passes
// over it and places the site (134, 45) at counter 6124 as E-trace-011
// records (one counter later with the field free).
// spec: MIL-CON-4.1a, ECO-PLACE-5.1
#[test]
fn mil_con_4_1a_the_burning_ruin_moves_the_springs_site() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() < 5817 {
        game.step();
    }
    assert!(game.world.ruins().iter().any(|r| r.at == (136, 47)));
    while game.world.tick() < 6124 {
        game.step();
        assert!(game.world.building_at((134, 45)).is_none());
    }
    // Trace tick 6125, the world after counter 6124.
    game.step();
    assert_eq!(
        game.world.building_at((134, 45)).map(|b| b.serial),
        Some(90809)
    );
}

// TRN-FREE T29 (TRN-FREE-6.4b): the hunter's hut site 90716 on (141, 46) is
// torn down at counter 6226 while its builder 90731 works on sample
// (563, 181); T is the tick after the removal, and from T + 1 he fine-walks
// to the point of the flag (141, 45), on which he stands in trace tick 6263
// and walks home from 6264 (E-trace-011). In T (6228) his cut work action
// still runs, WorkLoop0, and the walk ordered in T + 1 replaces it (Q-366 a).
// spec: TRN-FREE-6.4b (TRN-FREE T29), TRN-FREE-6.4a
#[test]
fn trn_free_t29_the_builder_of_a_torn_down_site_walks_back_to_its_flag() {
    use s2_sim::trace::Action::{Walk, WorkLoop};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let expected = [
        (6227, (140, 45), (563000, 181000), WorkLoop(0)),
        (6228, (140, 45), (563000, 181000), WorkLoop(0)),
        (6229, (140, 45), (563000, 181000), Walk),
        (6230, (140, 45), (563090, 181000), Walk),
        (6241, (140, 45), (564080, 180919), Walk),
        (6262, (140, 45), (565970, 180000), Walk),
        (
            6263,
            (141, 45),
            (566000, 180000),
            s2_sim::trace::Action::Stand,
        ),
        (6265, (141, 45), (565927, 180000), Walk),
    ];
    for (tick, field, fine, action) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90731")
            .expect("builder 90731");
        assert_eq!(
            (row.field, row.fine, row.action),
            (field, fine, action),
            "{tick}"
        );
    }
}

// MIL-UPG T6 (MIL-UPG-2.2, ECO-HQ-4.8a): the guard house 90300 on (138, 57),
// height 7400 since its placement, is upgraded at counter 5582; the start of
// the upgrade sets its field to the 19-field mean, 7467 (E-trace-011's heap
// dump of 6000).
// spec: MIL-UPG-2.2 (MIL-UPG T6), ECO-HQ-4.8a
#[test]
fn mil_upg_t6_the_upgrade_sets_the_guard_houses_height() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() < 5581 {
        game.step();
    }
    assert_eq!(game.world.terrain().height_at((138, 57)), 7400);
    // The run at the end of trace tick 5582 upgrades it (AI-MIL T15).
    game.step();
    assert!(game.world.upgrading((138, 57)).is_some());
    assert_eq!(game.world.terrain().height_at((138, 57)), 7467);
    while game.world.tick() < 6000 {
        game.step();
    }
    assert_eq!(game.world.terrain().height_at((138, 57)), 7467);
}

// TRN-FREE T30 (TRN-FREE-6.4c): the harbour (144, 47) is finished in trace
// tick 6459; its builder 90415, home being the harbour (its own flag
// nearest), walks back to the flag's point (145, 46), arriving in 6482. His
// home search on that flag orders a walk with no step: he stands in 6482
// and 6483, steps in at progress 0 in 6484 and moves from 6485; the harbour's
// door is its own field, so he stands on (144, 47) in 6507 and 6508 and is
// gone in 6509 (E-trace-011).
// spec: TRN-FREE-6.4c (TRN-FREE T30), TRN-FREE-6.4a, ECO-WORK-3.8b
#[test]
fn trn_free_t30_the_harbour_builder_takes_a_tick_for_each_no_step_walk() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let flag = (145, 46);
    let harbour = (144, 47);
    let expected = [
        (6482, Some((flag, (580000, 184000), Stand))),
        (6483, Some((flag, (580000, 184000), Stand))),
        (6484, Some((flag, (580000, 184000), Walk))),
        (6485, Some((flag, (579909, 184180), Walk))),
        (6507, Some((harbour, (578000, 188000), Stand))),
        (6508, Some((harbour, (578000, 188000), Stand))),
        (6509, None),
    ];
    for (tick, want) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90415")
            .map(|r| (r.field, r.fine, r.action));
        assert_eq!(row, want, "{tick}");
    }
}

// MIL T18 (MIL-SOL-4.2, 4.4 step 1, ECO-REQ-2.3a): the allocation pass of
// counter 6320 cancels the upgrading guard house 90300's builder request
// 90823 (wanted 1, its soldier 90744, R 1), the upgrade files 90824 in 6321
// and builder 90825 comes at 6335, so the object counter equals
// E-trace-011's after every counter it holds (was: one lower from 6321);
// with the flag run's cursor of AI-CON T50 the lone flag (144, 57) goes in
// counter 8104 as recorded (was 8105), and it equals it through 9946.
// spec: MIL-SOL-4.2, MIL-SOL-4.4, ECO-REQ-2.3a (MIL T18), SIM-START-4.2, AI-CON-9.5 (AI-CON T50)
#[test]
fn mil_t18_the_object_numbers_follow_the_recording_past_6000() {
    let Some(text) = recording_table("original-dense-20000-counter.csv.gz") else {
        return;
    };
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    // tick (the counter), snap, next_id, ...
    let recorded: std::collections::BTreeMap<u64, u32> = text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let v: Vec<&str> = line.split(',').collect();
            let counter: i64 = v[0].parse().ok()?;
            (counter >= 0).then(|| (counter as u64, v[2].parse().unwrap()))
        })
        .collect();
    assert_eq!(recorded.get(&6321), Some(&90825));
    let last = 9946;
    while game.world.tick() <= last {
        game.step();
        let counter = game.world.tick() - 1;
        if let Some(&next) = recorded.get(&counter) {
            assert_eq!(game.world.next_object(), next, "after counter {counter}");
        }
    }
}

// MIL-UPG T7 (MIL-UPG-2.3a): the guard house 90300 (138, 57) is upgraded
// to a tower; its builder 90825 (made at counter 6335) works the first
// unit at the guard house's four Roman positions, the flag's sample plus
// (-1, 2), (-2, 1), (1, 2), (0, 2): (555, 226) from trace tick 7646,
// (554, 225) from 7713, (557, 226) from 7802, (556, 226) from 7869
// (E-trace-011; the tower's offsets would put the second at (557, 226)).
// spec: MIL-UPG-2.3a (MIL-UPG T7), TRN-FREE-5.5a, ECO-HQ-4.6a
#[test]
fn mil_upg_t7_the_upgrades_builder_works_at_the_guard_houses_positions() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let expected = [
        (7646, (555000, 226000)),
        (7713, (554000, 225000)),
        (7802, (557000, 226000)),
        (7869, (556000, 226000)),
    ];
    for (tick, fine) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90825")
            .expect("builder 90825");
        assert_eq!(row.fine, fine, "{tick}");
    }
}

// AI-CON T45 (AI-CON-8.3 b, AI-CON-1.2, Q-279): the harbour 90351 (144, 47)
// is a depot by its type, so its route task takes the depot branch: S the
// castle in the neighbouring cell (8, 3), number 2 (run of 6802), k = 3 in
// 6987, done in 7024; then 90352 and 90436. AI-CON T46 (AI-CON-1.2, 8.3 a):
// the farm 90538 has a product, so it is no planting building and not
// skipped: cursor 90538 at 7875, B the farm at 7912 (w = 1, X [(136, 57)]).
// AI-CON T51: the bakery 90650's partners with k >= 2 follow its own need
// list [water, flour]: the spring first (9947 w 2 without a road builder
// call, 9984 the call and k = 3). The route optimizer's task equals
// E-trace-011's ai_route table after every run from 6000 to 10465 (cursor,
// k, i, t, w and X's count and last entry).
// spec: AI-CON-8.3 (T45, T46, T51), AI-CON-1.2
#[test]
fn t45_t46_the_route_task_equals_the_recording_past_6000() {
    let Some(text) = recording_table("original-dense-20000-ai_route.csv.gz") else {
        return;
    };
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    // tick (the counter), slot, cursor, cursor_hi, k, i, t, w, x_count,
    // x_last_x, x_last_y
    let recorded: std::collections::BTreeMap<u64, Vec<i64>> = text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let v: Vec<i64> = line.split(',').map(|f| f.parse().unwrap()).collect();
            (v[1] == 1).then(|| (v[0] as u64, v))
        })
        .collect();
    let (first, last) = (6000, 10465);
    while game.world.tick() <= last {
        game.step();
        let counter = game.world.tick() - 1;
        if counter < first || (counter + 6) % 37 != 0 {
            continue;
        }
        let r = &game.ai.player(P1).unwrap().street.route;
        let x_last =
            r.x.last()
                .map_or((-1, -1), |&(x, y)| (i64::from(x), i64::from(y)));
        let got = [
            i64::from(r.cursor),
            i64::from(r.k),
            r.i as i64,
            i64::from(r.t),
            r.w as i64,
            r.x.len() as i64,
            x_last.0,
            x_last.1,
        ];
        // The recorder missed a few ticks: the first row before the next
        // run stands in.
        let Some((_, v)) = recorded.range(counter..counter + 37).next() else {
            continue;
        };
        let want = [v[2], v[4], v[5], v[6], v[7], v[8], v[9], v[10]];
        assert_eq!(got, want, "the run of counter {counter}");
    }
}

// ECO-FOOD T47 (ECO-FOOD-5.7): the animal 90091 (home (143, 57)) waits on
// (145, 55) when player 1's hunter's hut 90625 (144, 58) reserves it in
// counter 7701 with 546 left; reserved, it keeps 546 (no countdown) until
// the hut is torn down in counter 7948, shows 546 in that tick, and counts
// down from its next update, 545 in counter 7949, 534 in 7960, with no new
// wait and no choosing (E-trace-011).
// spec: ECO-FOOD-5.7 (ECO-FOOD T47)
#[test]
fn eco_food_t47_a_reserved_waiting_animal_keeps_its_counter() {
    use s2_sim::AnimalState::Waiting;
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let animal = |game: &s2_ai::run::Runner| {
        game.world
            .animals()
            .find(|(_, a)| a.home == (143, 57))
            .map(|(_, a)| (a.at, a.state, a.hunted_by.is_some()))
            .expect("animal 90091")
    };
    let expected = [
        (7700, Waiting { left: 546 }, false),
        (7701, Waiting { left: 546 }, true),
        (7800, Waiting { left: 546 }, true),
        (7947, Waiting { left: 546 }, true),
        (7948, Waiting { left: 546 }, false),
        (7949, Waiting { left: 545 }, false),
        (7960, Waiting { left: 534 }, false),
    ];
    for (counter, state, hunted) in expected {
        while game.world.tick() <= counter {
            game.step();
        }
        assert_eq!(
            animal(&game),
            ((145, 55), state, hunted),
            "counter {counter}"
        );
    }
}

// ECO-WORK T-door-4 (ECO-WORK-3.8, 3.8a, 3.8b): player 1's farm 90538 on
// (136, 58) has its door north-west, (135, 59). Its first farmer 91017
// ends his walk on the farm's field in trace tick 9904 and steps to the
// door from progress 0, standing there from 9927; going out he steps back
// onto the farm's field from progress 0 in 9979, arriving 10002, and walks
// to the flag (136, 57) from progress 0, arriving 10025 (E-trace-011).
// spec: ECO-WORK-3.8 (ECO-WORK T-door-4), ECO-WORK-3.8a, ECO-WORK-3.8b
#[test]
fn eco_work_t_door_4_the_farmer_uses_the_farms_door() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let expected = [
        (9904, (136, 58), (544000, 232000), Walk),
        (9905, (136, 58), (543909, 232180), Walk),
        (9927, (135, 59), (542000, 236000), Stand),
        (9978, (135, 59), (542000, 236000), Stand),
        (9979, (135, 59), (542000, 236000), Walk),
        (9980, (135, 59), (542090, 235819), Walk),
        (10002, (136, 58), (544000, 232000), Walk),
        (10003, (136, 58), (544090, 231819), Walk),
        (10025, (136, 57), (546000, 228000), Walk),
    ];
    for (tick, field, fine, action) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "91017")
            .expect("farmer 91017");
        assert_eq!(
            (row.field, row.fine, row.action),
            (field, fine, action),
            "{tick}"
        );
    }
}

// AI-MIL T19 (AI-MIL-4.5): the guard house 90785 on (131, 59), finished at
// counter 9943, is visited by the upgrade run at 9984, 10169, 10354, 10539
// and 10724 with 5 or 4 planks in stock; the tower costs 3, and twice that
// is needed, so it is not upgraded (E-trace-011: its first upgrade request
// comes at 17533; with single costs it was upgraded at 9984).
// spec: AI-MIL-4.5 (AI-MIL T19), MIL-UPG-1.4
#[test]
fn ai_mil_t19_the_upgrade_waits_for_twice_the_costs() {
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    while game.world.tick() < 10_800 {
        game.step();
        assert_eq!(
            game.world.upgrading((131, 59)),
            None,
            "{}",
            game.world.tick()
        );
    }
    assert!(game.world.building_at((131, 59)).is_some());
}

// TRN-FREE T32 (TRN-FREE-6.4): the hunter 90379, its hut (140, 49) torn
// down at counter 10162, free-walks to the castle's flag (140, 50); arriving
// there in trace tick 10223 he has his way into the castle ordered in that
// tick and moves from 10224, with no standing tick (E-trace-011).
// spec: TRN-FREE-6.4 (TRN-FREE T32), TRN-FREE-6.4c
#[test]
fn trn_free_t32_the_way_in_is_ordered_in_the_arrival_tick() {
    use s2_sim::trace::Action::Walk;
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let expected = [
        (10222, (140, 49), (560019, 199960)),
        (10223, (140, 50), (560000, 200000)),
        (10224, (140, 50), (559909, 200180)),
        (10226, (140, 50), (559729, 200540)),
    ];
    for (tick, field, fine) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "90379")
            .expect("hunter 90379");
        assert_eq!(
            (row.field, row.fine, row.action),
            (field, fine, Walk),
            "{tick}"
        );
    }
}

// MIL-UPG T8 (MIL-UPG-2.4, 1.2): the guard house 90300 on (138, 57)
// upgrading to a tower: the main part is worked off at counter 10718, in
// builder 90825's last step, who works on to 10725 and is released at
// 10726; the type is tower from counter 10727 (E-trace-011).
// spec: MIL-UPG-2.4 (MIL-UPG T8), MIL-UPG-1.2, ECO-HQ-4.7
#[test]
fn mil_upg_t8_the_tower_comes_after_the_builders_release() {
    let Some((mut game, names)) = recorded_game() else {
        return;
    };
    let at = (138, 57);
    let kind = |game: &s2_ai::run::Runner| {
        names[&game.world.building_at(at).unwrap().building_type].clone()
    };
    // After counter 10718, the construction done.
    while game.world.tick() <= 10718 {
        game.step();
    }
    assert!(game.world.site(at).is_none());
    assert!(game.world.upgrading(at).is_some());
    assert_eq!(kind(&game), "guardHouse");
    // After counter 10726 still the guard house; after 10727 the tower.
    while game.world.tick() <= 10726 {
        game.step();
    }
    assert_eq!(kind(&game), "guardHouse");
    game.step();
    assert_eq!(kind(&game), "tower");
    assert!(game.world.upgrading(at).is_none());
}

// TRN-FREE T33 (TRN-FREE-6.4b): the hunter's hut site 91151 on (142, 58) is
// torn down at counter 11064 while its builder walks his approach from the
// flag (142, 57); he walks it to its end in the going-home state, out to
// (571, 231) and back to the flag's point, which he reaches in trace tick
// 11115, and walks home from 11116 (E-trace-011; our builder is 91178, one
// object number off).
// spec: TRN-FREE-6.4b (TRN-FREE T33), ECO-HQ-4.6b
#[test]
fn trn_free_t33_the_approach_walk_is_walked_to_its_end() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = recorded_game() else {
        return;
    };
    let expected = [
        (11065, (570620, 229620), Walk),
        (11069, (570980, 229980), Walk),
        (11080, (571000, 230970), Walk),
        (11113, (570000, 228119), Walk),
        (11114, (570000, 228029), Walk),
        (11115, (570000, 228000), Stand),
        (11116, (570000, 228000), Walk),
    ];
    for (tick, fine, action) in expected {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.job == "Home" && r.field == (142, 57) && r.profession == 0xdd2daebe)
            .expect("the builder");
        assert_eq!((row.fine, row.action), (fine, action), "{tick}");
    }
}

// AI-CON T63 (AI-CON-8.3 c, TRN-CARRY-5.3a): the route task on the harbour
// 90366 (k 2, its partner the castle) tries Fn = the flag (142, 47) in the
// run of counter 7616 (E-trace-012's ai_route table: t 2, X two entries,
// the last (142, 47)); its walk path to F0 does not count, so neither
// stored cost is asked: the pair (142, 47) -> the castle's flag (140, 50)
// is first stored by the storage run of 7780 (28, the castle's flag full
// then), not at 7616 (14).
// spec: AI-CON-8.3 (T63), TRN-CARRY-5.3a
#[test]
fn ai_con_t63_a_try_whose_walk_path_does_not_count_stores_no_cost() {
    let Some((mut game, _)) = hard_game() else {
        return;
    };
    while game.world.tick() <= 7616 {
        game.step();
    }
    let r = &game.ai.player(P1).unwrap().street.route;
    assert_eq!(
        (r.k, r.i, r.t, r.x.clone()),
        (2, 0, 2, vec![(143, 45), (142, 47)])
    );
    let stored = game
        .world
        .lend(|_, _, _, routes| routes.stored(P1, (142, 47), (140, 50)));
    assert_eq!(stored, None);
}

// ECO-HQ T25 (ECO-HQ-4.3, ECO-REQ-0.2, TRN-MOVE-2.5): the mill site's
// leveller 91004 on (137, 60) ends a bulldoze step in counter 7621 (trace
// tick 7622); the constructor 91022 of the site on (136, 61) starts his
// step from (549, 243) to (549, 242) in the same tick. The constructors'
// list is updated before the levellers' list, so his step is timed by the
// heights before the levelling step: 0.072 a tick (0.09 with the new
// heights), (549000, 242927) in 7623 and (549000, 242855) in 7624 (E-trace-012).
// spec: ECO-HQ-4.3 (ECO-HQ T25), ECO-REQ-0.2, TRN-MOVE-2.5
#[test]
fn eco_hq_t25_a_step_started_with_the_levelling_sees_the_old_heights() {
    let Some((mut game, _)) = hard_game() else {
        return;
    };
    for (tick, fine) in [
        (7622, (549000, 243000)),
        (7623, (549000, 242927)),
        (7624, (549000, 242855)),
    ] {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.job.starts_with("Builder") && r.field == (137, 60))
            .expect("the constructor");
        assert_eq!(row.fine, fine, "{tick}");
    }
}

// TRN-FREE T34 (TRN-FREE-6.4b, a work step's walk): the hunter's hut site
// on (142, 44) is torn down by the computer player at counter 8809 while its
// builder 91166 walks the lattice step (569, 174) -> (568, 174) at 0.09 a
// tick; he walks it to its end from the removal on, in the going-home state
// from 8810: (568189, 174000) in trace tick 8810, (568099, 174000) in 8811,
// (568009, 174000) in 8812 (E-trace-012 / 013), not back on (569, 174). The
// rest of his walk is the next test's.
// spec: TRN-FREE-6.4b (TRN-FREE T34), ECO-HQ-4.6b, TRN-MOVE-2.5
#[test]
fn trn_free_t34_the_work_step_is_walked_to_its_end() {
    let Some((mut game, _)) = hard_game() else {
        return;
    };
    for (tick, fine) in [
        (8810, (568189, 174000)),
        (8811, (568099, 174000)),
        (8812, (568009, 174000)),
    ] {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.profession == 0xdd2daebe && r.field == (141, 43))
            .expect("the builder");
        assert_eq!((row.fine, row.job.as_str()), (fine, "Home"), "{tick}");
    }
}

// TRN-FREE T34 (TRN-FREE-6.4b, Q-286): builder 91166 of the removed hunter's
// hut site walks his work walk on to the construction position it goes to,
// position 1 (567, 173), each step at the rate of its start, the excess
// carried within the walk: 0.08 into the step to (567, 173) in trace tick
// 8813. He reaches it in 8831 and the straight line to the flag's point is
// ordered there at progress 0, the excess dropped: 0.09 in 8832, (568, 173)
// with 0.08 carried in 8843, standing on the flag's point (570, 172) in
// 8865, his walk home from 8866 (E-trace-012 / 013).
// spec: TRN-FREE-6.4b (TRN-FREE T34), TRN-FREE-5.1, TRN-FREE-5.3, TRN-MOVE-2.5
#[test]
fn trn_free_t34_the_work_walk_ends_on_its_position_then_the_line() {
    use s2_sim::trace::Action::{Stand, Walk};
    let Some((mut game, _)) = hard_game() else {
        return;
    };
    for (tick, field, fine, action) in [
        (8813, (141, 43), (567919, 173919), Walk),
        (8830, (141, 43), (567001, 173001), Walk),
        (8831, (141, 43), (567000, 173000), Walk),
        (8832, (141, 43), (567090, 173000), Walk),
        (8842, (141, 43), (567990, 173000), Walk),
        (8843, (141, 43), (568080, 172919), Walk),
        (8854, (141, 43), (569070, 172000), Walk),
        (8864, (141, 43), (569970, 172000), Walk),
        (8865, (142, 43), (570000, 172000), Stand),
        (8866, (142, 43), (570000, 172000), Walk),
    ] {
        while game.world.tick() < tick {
            game.step();
        }
        let row = game
            .world
            .trace_settlers()
            .into_iter()
            .find(|r| r.id == "91166")
            .expect("the builder");
        assert_eq!(
            (row.field, row.fine, row.action),
            (field, fine, action),
            "{tick}"
        );
    }
}

// AI-CON T91, T92 (AI-CON-1.7, stock2): every ware object of the player
// counts, whoever holds it: besides the wares lying at flags and on the
// roads, a warehouse helper's ware and a worker's product once his put-down
// has made it (not while he still puts it down, Take / TakeWare first half).
// spec: AI-CON-1.7 (AI-CON T91, T92)
#[test]
fn stock2_counts_every_ware_object_of_the_player() {
    use s2_sim::{WareLocation, Work};
    let Some((_, config, mut world)) = game() else {
        return;
    };
    let mut ai = AiSystem::new(config, &world);
    ai.activate_all(&mut world, &[(P0, 2), (P1, 3)]);
    let (mut helpers, mut putting_down) = (0, 0);
    for _ in 0..6000 {
        for c in ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        ) {
            world.submit(c);
        }
        world.step();
        let counted = s2_ai::view::Economy::count_moving(&world, P1);
        let mut want: std::collections::BTreeMap<u32, u32> = Default::default();
        for (_, w) in world.wares() {
            let owner = match w.location {
                WareLocation::Flag(at) => world.flag_at(at),
                WareLocation::Carried(c) => world
                    .carriers()
                    .get(c)
                    .and_then(|c| world.road(c.road))
                    .map(|r| r.owner),
            };
            if owner == Some(P1) {
                *want.entry(w.good).or_default() += 1;
            }
        }
        for (_, s) in world.settlers() {
            let (Some(good), true) = (s.carrying, s.player == P1) else {
                continue;
            };
            match &s.job {
                SettlerJob::CarryOut { .. } => helpers += 1,
                SettlerJob::Worker {
                    work: Work::Take { second: false } | Work::TakeWare { second: false, .. },
                    ..
                } => {
                    putting_down += 1;
                    continue;
                }
                _ => {}
            }
            *want.entry(good).or_default() += 1;
        }
        assert_eq!(counted, want, "tick {}", world.tick());
    }
    assert!(helpers > 0, "a helper carried a ware out");
    assert!(putting_down > 0, "a worker put a product down");
}
