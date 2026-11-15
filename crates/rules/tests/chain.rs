//! A headless production chain on a free-game map, with the original rules
//! of the player's own install (`S2_GAME_DIR`; skipped when unset): woodcutter,
//! sawmill, quarry and the bread chain well + farm + mill + bakery, with a
//! coal mine fed from the bakery.
//!
//! The headquarters starts with start resources 3 (8 times the base stock)
//! but without fish, ham, bread and coal, so the coal mine's food and the
//! first coal come from the chain. A fisherman's and a hunter's hut stand
//! beside it (ECO-FOOD-4, 5), the farm sows and harvests (ECO-FOOD-3) and
//! the mine finds its ore field by the search of ECO-FOOD-2.

use s2_assets::{s2m, GameDir, GameDirError};
use s2_sim::grid::{self, Point};
use s2_sim::{Command, Placement, PlayerId, World};

const MAPS: &str = "data/maps/freeGameMaps";
const P0: PlayerId = PlayerId(0);
/// The buildings of the chain in the order a player would place them.
const CHAIN: [&str; 11] = [
    "woodCutter",
    "sawMill",
    "stonePit",
    "spring",
    "farm",
    "mill",
    "bakery",
    "coalMine",
    "woodCutter",
    "fisher",
    "hunter",
];
const LIMIT: u64 = 40_000;

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

/// A new game of player 0 on `map` with the base stock x 8, minus the foods
/// and coal.
fn start(rules: &s2_rules::RuleSet, map: &s2m::Map) -> World {
    let mut sim = rules.sim_rules().unwrap();
    let drop: Vec<u32> = ["fish", "flesh", "bread", "coal"]
        .iter()
        .map(|n| rules.id_named(n).unwrap())
        .collect();
    sim.start_stock.retain(|(good, _)| !drop.contains(good));
    let setup = s2_rules::game_setup(map, rules, 3);
    World::new_game(7, map.terrain(), sim, &setup)
}

/// Where each building of the chain can go, with its road to the network:
/// the commands (building, then road) in order, or `None` for a map without
/// room.
fn plan(world: &World, rules: &s2_rules::RuleSet, map: &s2m::Map) -> Option<Vec<Command>> {
    let hq = (map.info.start_positions[0].x, map.info.start_positions[0].y);
    let coal = rules.id_named("coal").unwrap();
    let fish = rules.id_named("fish").unwrap();
    let (w, h) = (map.info.width, map.info.height);
    let mut trial = world.clone();
    let mut network = vec![trial.terrain().neighbour(hq, 1)?];
    let mut taken: Vec<Point> = Vec::new();
    let mut commands = Vec::new();
    // The mine first: it is the scarce site.
    let mut order: Vec<&str> = CHAIN.to_vec();
    order.sort_by_key(|k| *k != "coalMine");
    for kind in &order {
        let id = rules.id_named(kind).unwrap();
        let want_mine = *kind == "coalMine";
        let want_fish = *kind == "fisher";
        let want_game = *kind == "hunter";
        let min = match *kind {
            "sawMill" | "mill" | "bakery" => Placement::Medium,
            "farm" => Placement::Large,
            _ => Placement::Small,
        };
        let mut found = None;
        'ring: for f in (2..=9).flat_map(|k| grid::ring(hq, k, w, h)) {
            if taken.contains(&f) {
                continue;
            }
            let class = trial.placement_at(f);
            if want_mine {
                if class != Placement::Mine {
                    continue;
                }
                let has_coal = grid::area(f, 3, w, h).into_iter().any(|a| {
                    trial
                        .resource_at(a)
                        .is_some_and(|r| r.good == coal && r.amount > 0)
                });
                if !has_coal {
                    continue;
                }
            } else if class < min || class == Placement::Mine {
                continue;
            }
            let Some(flag) = trial.terrain().neighbour(f, 1) else {
                continue;
            };
            // ECO-FOOD-4.2: a fishing spot within 8 of the flag.
            if want_fish && trial.fishing_spots(flag, fish, 8).is_empty() {
                continue;
            }
            // ECO-FOOD-5.2: an explored animal within 8.
            if want_game
                && !trial
                    .animals()
                    .any(|(_, a)| grid::distance(f, a.at) <= 8 && trial.is_explored(P0, a.at))
            {
                continue;
            }
            let mut candidates = network.clone();
            candidates.sort_by_key(|&n| grid::distance(n, flag));
            for to in candidates.into_iter().take(4) {
                let mut probe = trial.clone();
                let place = Command::PlaceBuilding {
                    player: P0,
                    building: id,
                    at: f,
                };
                let road = Command::BuildRoad {
                    player: P0,
                    from: flag,
                    to,
                };
                probe.submit(place);
                probe.step();
                if probe.last_results()[0].outcome.is_err() {
                    continue 'ring;
                }
                if flag == to {
                    continue;
                }
                probe.submit(road);
                probe.step();
                if probe.last_results()[0].outcome.is_ok() {
                    found = Some((f, flag, to, probe));
                    break 'ring;
                }
            }
        }
        let (f, flag, to, probe) = found?;
        trial = probe;
        taken.push(f);
        network.push(flag);
        commands.push((
            *kind,
            Command::PlaceBuilding {
                player: P0,
                building: id,
                at: f,
            },
            Command::BuildRoad {
                player: P0,
                from: flag,
                to,
            },
        ));
    }
    // Replayed in the order they were planned: a road's route depends on
    // what was built before it.
    let flat = commands
        .into_iter()
        .flat_map(|(_, place, road)| [place, road])
        .collect();
    Some(flat)
}

/// A building of `kind` has made its first product: a workshop has taken one
/// (its count), an outdoor worker has finished a trip (its productivity rose
/// since the last tick, ECO-OUT-3.11: from the 0.5 its site had, T26).
/// `last`: each building's productivity as last seen.
fn first_tick(
    world: &World,
    rules: &s2_rules::RuleSet,
    kind: &str,
    last: &mut std::collections::BTreeMap<s2_sim::grid::Point, i32>,
) -> bool {
    let id = rules.id_named(kind).unwrap();
    let mut made = false;
    for b in world.buildings() {
        if b.building_type != id || b.under_construction {
            continue;
        }
        let before = last.insert(b.at, b.productivity);
        made |= b.production.count >= 1 || before.is_some_and(|p| b.productivity > p);
    }
    made
}

// spec: ECO-PROD-2.2, ECO-DIST-2.3, ECO-FOOD-2.1
#[test]
fn bread_chain_feeds_a_coal_mine() {
    let Some(dir) = game_dir() else { return };
    let (rules, _) = s2_rules::load(&dir).unwrap();
    let mut names: Vec<_> = std::fs::read_dir(dir.root().join(MAPS))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "s2m"))
        .collect();
    names.sort();
    let mut tried = Vec::new();
    for path in names {
        let rel = format!("{MAPS}/{}", path.file_name().unwrap().to_string_lossy());
        let map = s2m::load(&rel, &std::fs::read(&path).unwrap()).unwrap();
        let world = start(&rules, &map);
        let Some(commands) = plan(&world, &rules, &map) else {
            tried.push(rel);
            continue;
        };
        eprintln!("map {rel}: {} commands", commands.len());
        let mut world = world;
        let mut log: Vec<(u64, Command)> = commands
            .into_iter()
            .enumerate()
            .map(|(i, c)| (1 + i as u64 * 2, c))
            .collect();
        log.reverse();
        let (mut flour, mut bread, mut coal) = (None, None, None);
        let mut seen = std::collections::BTreeMap::new();
        let (mut fish_tick, mut ham_tick) = (None, None);
        let mut delivered = None;
        let hq = (map.info.start_positions[0].x, map.info.start_positions[0].y);
        let coal_good = rules.id_named("coal").unwrap();
        for tick in 1..=LIMIT {
            while log.last().is_some_and(|(t, _)| *t <= tick) {
                let (_, c) = log.pop().unwrap();
                world.submit(c);
            }
            world.step();
            for r in world.last_results() {
                assert_eq!(r.outcome, Ok(()), "{:?}", r.command);
            }
            if flour.is_none() && first_tick(&world, &rules, "mill", &mut seen) {
                flour = Some(tick);
            }
            if bread.is_none() && first_tick(&world, &rules, "bakery", &mut seen) {
                bread = Some(tick);
            }
            if coal.is_none() && first_tick(&world, &rules, "coalMine", &mut seen) {
                coal = Some(tick);
            }
            if fish_tick.is_none() && first_tick(&world, &rules, "fisher", &mut seen) {
                fish_tick = Some(tick);
            }
            if ham_tick.is_none() && first_tick(&world, &rules, "hunter", &mut seen) {
                ham_tick = Some(tick);
            }
            if delivered.is_none() && world.warehouse(hq).is_some_and(|w| w.count(coal_good) > 0) {
                delivered = Some(tick);
            }
            if [flour, bread, coal, fish_tick, ham_tick, delivered]
                .iter()
                .all(Option::is_some)
            {
                break;
            }
        }
        eprintln!(
            "first flour {flour:?}, first bread {bread:?}, first coal {coal:?}, first fish \
             {fish_tick:?}, first ham {ham_tick:?}, coal in the headquarters {delivered:?}"
        );
        if [flour, bread, coal, fish_tick, ham_tick, delivered]
            .iter()
            .any(Option::is_none)
        {
            report(&world, &rules, LIMIT);
            panic!(
                "the chain stalled: flour {flour:?}, bread {bread:?}, coal {coal:?}, fish \
                 {fish_tick:?}, ham {ham_tick:?}"
            );
        }
        let (flour, bread, coal) = (flour.unwrap(), bread.unwrap(), coal.unwrap());
        assert!(flour < bread, "flour before bread");
        // The mine eats fish, then bread, then ham (ECO-PROD-2.2): its first
        // coal comes after the first food of any of the three.
        let first_food = [Some(bread), fish_tick, ham_tick]
            .into_iter()
            .flatten()
            .min()
            .unwrap();
        assert!(first_food < coal, "the mine needs food");
        return;
    }
    panic!("no map with room for the chain: {tried:?}");
}

/// A look at every building of player 0, for a chain that stalls.
fn report(world: &World, rules: &s2_rules::RuleSet, tick: u64) {
    let name = |id: u32| {
        rules
            .names
            .iter()
            .find(|(_, &v)| v == id)
            .map_or(format!("{id:08X}"), |(k, _)| k.clone())
    };
    eprintln!("-- tick {tick}");
    for (id, w) in world.wares() {
        eprintln!(
            "ware {id}: {} {:?} dest {:?} target {:?} claimed {:?}",
            name(w.good),
            w.location,
            w.destination,
            w.target.map(|t| t.at),
            w.claimed_by
        );
    }
    for (i, c) in world.carriers().iter().enumerate() {
        eprintln!(
            "carrier {i}: no {} road {} at {:?} ware {:?} task {:?}",
            c.number, c.road, c.walker.at, c.ware, c.task
        );
    }
    for (i, r) in world.roads().enumerate() {
        eprintln!(
            "road {i}: {:?}..{:?} len {} carrier {:?} donkey {:?}",
            r.path.first(),
            r.path.last(),
            r.length,
            r.carrier,
            r.donkey
        );
    }
    for b in world.buildings().filter(|b| b.owner == P0) {
        let inputs: Vec<String> = world
            .inputs(b.at)
            .iter()
            .map(|&(g, n)| format!("{}x{n}", name(g)))
            .collect();
        let orders: Vec<String> = world
            .orders()
            .iter()
            .filter(|o| o.target.at == b.at)
            .map(|o| name(o.good))
            .collect();
        eprintln!(
            "{:>16} at {:?} site={} workers={} active={} made={} inputs={:?} orders={:?}",
            name(b.building_type),
            b.at,
            b.under_construction,
            b.workers,
            b.active,
            b.production.count,
            inputs,
            orders
        );
    }
}
