//! The examples of `docs/api.md`, compiled and run: a game set up, commands
//! in, state, views and events out. On the synthetic fixture, so it runs
//! without an install (and on CI).

use s2_sim::view::{Activity, SettlerRef};
use s2_sim::{Command, PlayerId, World};

// spec: none (the public API's examples, docs/api.md)
#[test]
fn commands_in_state_and_events_out() {
    // A game: the world and its computer players (here the fixture's).
    let (mut world, config) = s2_ai::fixture::named("town2", 7).expect("the fixture");
    let mut ai = s2_ai::AiSystem::new(config, &world);
    ai.activate(&mut world, PlayerId(1), 2);

    // Commands in: the human player builds a woodcutter's hut by its
    // headquarters and a road to it; the computer player gives its own.
    let me = PlayerId(0);
    // The first field 3 to 6 steps from the headquarters (20, 20) that
    // takes a small building (ECO-PLACE-4.1).
    let hut = (3..=6)
        .flat_map(|k| s2_sim::grid::ring((20, 20), k, 96, 96))
        .find(|&f| {
            world.placement_at(f) >= s2_sim::Placement::Small
                && world.placement_at(f) != s2_sim::Placement::Mine
        })
        .expect("a building spot");
    world.submit(Command::PlaceBuilding {
        player: me,
        building: s2_ai::fixture::WOODCUTTER,
        at: hut,
    });
    for _ in 0..3000 {
        // One tick; the computer players think last in it (AI-2.1a).
        ai.step(&mut world);
        // Every command applied in the step, with its outcome.
        for result in world.last_results() {
            if result.command
                == (Command::PlaceBuilding {
                    player: me,
                    building: s2_ai::fixture::WOODCUTTER,
                    at: hut,
                })
            {
                assert_eq!(result.outcome, Ok(()));
            }
        }
        if world.tick() == 1 {
            let hq_flag = world.terrain().neighbour((20, 20), 1).unwrap();
            let site_flag = world.terrain().neighbour(hut, 1).unwrap();
            world.submit(Command::BuildRoad {
                player: me,
                from: hq_flag,
                to: site_flag,
            });
        }
    }

    // State out: buildings, stocks, sites, territory.
    let building = world.building_at(hut).expect("the hut");
    assert_eq!(building.owner, me);
    assert!(world.stock((20, 20)).is_some());
    assert!(world.owned_field_count(me) > 0);

    // Views out: what each settler, carrier, builder and leveller does.
    let views = world.settler_views();
    let carriers = views
        .iter()
        .filter(|v| matches!(v.id, SettlerRef::Carrier(_)))
        .count();
    assert!(carriers >= 1, "the road has its carrier");
    for v in &views {
        if let Activity::Worker { building, .. } = v.activity {
            assert!(world.building_at(building).is_some());
        }
        let _where = (v.walk.at, v.walk.next, v.walk.fraction());
    }

    // Events out: messages to the players.
    let _messages = world.messages();

    // Determinism: the same game twice gives the same hash.
    let again = |seed| {
        let (mut w, _): (World, _) = s2_ai::fixture::named("town2", seed).unwrap();
        for _ in 0..200 {
            w.step();
        }
        w.state_hash()
    };
    assert_eq!(again(7), again(7));
}

/// docs/api.md, "A headless game": the documented calls start the game
/// `s2-app hash` plays (every player a computer player), so after the same
/// ticks it has the same world and computer player hashes. Needs the
/// install (`S2_GAME_DIR`); skipped without it.
// spec: none (the public API's headless start, docs/api.md)
#[test]
fn a_headless_game_is_the_game_of_s2_app_hash() {
    let Ok(dir) = s2_assets::GameDir::from_env() else {
        eprintln!("S2_GAME_DIR not set: skipped");
        return;
    };
    let (name, ticks) = ("MP_2P_Storm_Coast", 300);

    // The documented start.
    let (rules, _report) = s2_rules::load(&dir).expect("the rules");
    let file = s2_assets::find_map(&dir, name).expect("the map");
    let bytes = std::fs::read(&file).expect("the map file");
    let map = s2_assets::s2m::load(&file.to_string_lossy(), &bytes).expect("a map");
    let setup = s2_rules::game_setup(&map, &rules, 1);
    let sim = rules.sim_rules().expect("the simulation's rules");
    let seed = u64::from(s2_sim::rng::SIM_SEED);
    let mut world = World::new_game(seed, map.terrain(), sim.clone(), &setup);
    let mut ai = s2_ai::AiSystem::new(s2_ai::AiConfig::new(&rules, &sim), &world);
    let players = s2_ai::every_player(&world);
    ai.activate_all(&mut world, &players);
    for _ in 0..ticks {
        ai.step(&mut world);
    }

    // `s2-app hash`'s last line: "<tick> world <hash> ai <hash> ...".
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_s2-app"))
        .args(["hash", name, &ticks.to_string()])
        .output()
        .expect("s2-app runs");
    let text = String::from_utf8(out.stdout).expect("text");
    let last: Vec<&str> = text
        .lines()
        .last()
        .expect("a hash line")
        .split(' ')
        .collect();
    assert_eq!(last[0], ticks.to_string());
    assert_eq!(last[2], format!("{:016x}", world.state_hash()));
    assert_eq!(last[4], format!("{:016x}", ai.state_hash()));
}
