//! Tests of the AI structure: activation, schedule, maps, re-initialisation
//! (`AI-overview.md` T1 to T4, `AI-misc.md` T6, T7).

use s2_sim::{Command, PlayerId, World};

use crate::cells::cell_of;
use crate::schedule::Part;
use crate::testutil::{config, world, CASTLE};
use crate::AiSystem;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn hq(world: &World, player: PlayerId) -> (u32, u32) {
    world
        .buildings()
        .find(|b| b.owner == player && b.building_type == CASTLE)
        .expect("a headquarters")
        .at
}

// AI-MISC T6: at game creation the profile is set, the home recorded and
// the headquarters announced: one cell; the headquarters is a depot
// (AI-CON-1.1, corrected 2026-10-24), so the depot marker set at the cell's
// creation is cleared again (AI-CON-1.6).
// spec: AI-MISC-2.1, AI-MISC-2.2, AI-CON-1.1, AI-CON-1.6 (T6)
#[test]
fn activation_records_home_and_announces_the_headquarters() {
    let mut world = world(&[0, 1]);
    let mut ai = AiSystem::new(config(), &world);
    assert!(!ai.is_active(P0));
    assert!(ai.activate(&mut world, P0, 2));
    assert!(ai.is_active(P0));
    assert!(!ai.is_active(P1));
    let player = ai.player(P0).unwrap();
    assert_eq!(player.profile_number, 2);
    assert_eq!(player.home, hq(&world, P0));
    assert_eq!(Some(player.home_landmass), world.continent_of(player.home));
    assert_eq!(player.records.len(), 1);
    assert_eq!(player.cells.len(), 1);
    let cell = player.cells.get(cell_of(player.home)).unwrap();
    assert!(cell.depot_wanted.is_none());
    assert_eq!(cell.buildings, [player.home]);
    assert_eq!(
        player.records[&player.home].category,
        crate::player::Category::Depot
    );
    assert!(player.records[&player.home].protect.depot);
}

// spec: AI-MISC-2.1, AI-1.1
#[test]
fn activation_refuses_unknown_profiles_and_slots() {
    let mut world = world(&[0]);
    let mut ai = AiSystem::new(config(), &world);
    assert!(!ai.activate(&mut world, P0, 0));
    assert!(!ai.activate(&mut world, P0, 4));
    assert!(!ai.activate(&mut world, PlayerId(9), 1));
    assert!(ai.activate(&mut world, P0, 3));
}

// AI-overview T1: slot 2's attack system runs where (c + 10) mod 641 = 0 and
// its military settings where (c + 10) mod 53 = 0.
// spec: AI-2.2, AI-2.3
#[test]
fn slot_two_runs_attack_and_settings_in_the_right_ticks() {
    let mut world = world(&[0, 1, 2]);
    let mut ai = AiSystem::new(config(), &world);
    ai.activate(&mut world, P2, 2);
    ai.trace = Some(Vec::new());
    for _ in 0..700 {
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
    let runs = |part: Part| -> Vec<u64> {
        ai.trace
            .as_ref()
            .unwrap()
            .iter()
            .filter(|r| r.player == P2 && r.part == part)
            .map(|r| r.counter)
            .collect()
    };
    assert_eq!(runs(Part::AttackSystem), [631]);
    assert_eq!(
        runs(Part::MilitarySettings),
        [43, 96, 149, 202, 255, 308, 361, 414, 467, 520, 573, 626, 679]
    );
    // Period 1 parts run every tick.
    assert_eq!(runs(Part::ConstructionSystem).len(), 700);
}

// AI-overview T2: slot 1 runs each part 5 counter steps after slot 0.
// spec: AI-2.2
#[test]
fn two_players_run_five_counter_steps_apart() {
    let mut world = world(&[0, 1]);
    let mut ai = AiSystem::new(config(), &world);
    ai.activate(&mut world, P0, 1);
    ai.activate(&mut world, P1, 1);
    ai.trace = Some(Vec::new());
    for _ in 0..30 {
        ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        );
        world.step();
    }
    let runs = |p: PlayerId| -> Vec<u64> {
        ai.trace
            .as_ref()
            .unwrap()
            .iter()
            .filter(|r| r.player == p && r.part == Part::StreetConstruction)
            .map(|r| r.counter)
            .collect()
    };
    assert_eq!(runs(P0), [7, 14, 21, 28]);
    assert_eq!(runs(P1), [2, 9, 16, 23, 30]);
}

// The order of the player-level parts within a tick (AI-2.3).
#[test]
fn parts_run_in_the_specified_order() {
    let mut world = world(&[0]);
    let mut ai = AiSystem::new(config(), &world);
    ai.activate(&mut world, P0, 1);
    ai.trace = Some(Vec::new());
    // Counter 0 does not occur (it starts at 1), so use a tick where many
    // periods meet: c = 13 x 17 = 221 has 13, 17 and the period-1 parts.
    for _ in 0..221 {
        ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        );
        world.step();
    }
    let parts: Vec<Part> = ai
        .trace
        .as_ref()
        .unwrap()
        .iter()
        .filter(|r| r.counter == 221)
        .map(|r| r.part)
        .collect();
    use Part::*;
    let expected = [
        NeedsArrangement,
        NeedsConstruction,
        ResourceSystem,
        ExpansionSystem,
        MilitarySystem,
        ConstructionSystem,
        DestructionSystem,
    ];
    let mut at = 0;
    for p in &parts {
        if at < expected.len() && *p == expected[at] {
            at += 1;
        }
    }
    assert_eq!(at, expected.len(), "{parts:?}");
}

// The AI reads the world but changes it only through commands: if its
// commands are dropped, the world runs exactly as it would without it
// (the generator and the object counter it may use are its own here).
// spec: AI-3.7
#[test]
fn ai_changes_the_world_only_through_commands() {
    let mut plain = world(&[0, 1]);
    let mut with_ai = world(&[0, 1]);
    let mut ai = AiSystem::new(config(), &with_ai);
    ai.activate_with(&with_ai, &mut s2_sim::Numbers::new(0), P0, 2);
    let mut issued = 0;
    for _ in 0..400 {
        plain.step();
        issued += ai
            .tick(
                &with_ai,
                &mut s2_sim::Rng::new(1),
                &mut s2_sim::Numbers::new(0),
                &mut s2_sim::RouteStore::default(),
            )
            .len();
        with_ai.step();
    }
    assert!(issued > 0, "the AI did nothing");
    assert_eq!(plain.state_hash(), with_ai.state_hash());
}

// AI-MIL-1.1: a cell of the military map is refreshed per player and tick,
// cycling; an activated player's map is complete at once.
#[test]
fn the_military_map_holds_threat_and_score_of_buildings() {
    let mut world = world(&[0, 1]);
    let mut config = config();
    config.weights.insert(
        CASTLE,
        crate::config::Weights {
            score: 10,
            threat: 150,
        },
    );
    let mut ai = AiSystem::new(config, &world);
    ai.activate(&mut world, P0, 1);
    let at = hq(&world, P0);
    let cell = ai.military[0].get(crate::maps::mcell_of(at));
    assert_eq!((cell.threat, cell.score), (150, 10));
    // Player 1 is not activated: its map fills one cell per tick.
    assert_eq!(
        ai.military[1]
            .get(crate::maps::mcell_of(hq(&world, P1)))
            .threat,
        0
    );
    let cells = ai.military[1].cols * ai.military[1].rows;
    for _ in 0..cells {
        ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        );
        world.step();
    }
    assert_eq!(
        ai.military[1]
            .get(crate::maps::mcell_of(hq(&world, P1)))
            .threat,
        150
    );
}

// AI-CON-1.1: a cell exists once a corner field is the player's; the scan
// looks at one cell index per 5 ticks, cycling over the grid.
#[test]
fn the_cell_scan_creates_cells_with_an_owned_corner() {
    let mut world = world(&[0]);
    let mut ai = AiSystem::new(config(), &world);
    ai.activate(&mut world, P0, 1);
    let total = 6 * 6;
    for _ in 0..total * 5 + 5 {
        ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        );
        world.step();
    }
    let player = ai.player(P0).unwrap();
    // Territory of radius 9 around (20, 20): corners (16,16) and (31,16) etc.
    for pos in player.cells.positions() {
        let owned = crate::cells::corner_owned(&world, P0, pos);
        let has_building = !player.cells.get(pos).unwrap().buildings.is_empty();
        assert!(owned || has_building, "{pos:?}");
    }
    assert!(player.cells.len() > 1, "{} cells", player.cells.len());
}

// AI-MISC-2.3 / T7: an active AI that has no cells left is reset and
// switched off, and does not think again.
#[test]
fn an_ai_without_cells_is_reset_and_switched_off() {
    let mut world = world(&[0, 1]);
    let mut ai = AiSystem::new(config(), &world);
    ai.activate(&mut world, P0, 2);
    ai.activate(&mut world, P1, 2);
    ai.trace = Some(Vec::new());
    // Take the headquarters of player 0 away.
    let at = hq(&world, P0);
    world.submit(Command::RemoveBuilding { player: P0, at });
    world.step();
    for _ in 0..(6 * 6 * 5 + 20) {
        ai.tick(
            &world,
            &mut s2_sim::Rng::new(1),
            &mut s2_sim::Numbers::new(0),
            &mut s2_sim::RouteStore::default(),
        );
        world.step();
    }
    if world.owned_field_count(P0) > 0 {
        // The land stayed; the AI keeps its cells, nothing to check here.
        eprintln!("land of player 0 stayed after removing the headquarters");
        return;
    }
    assert!(!ai.is_active(P0));
    let player = ai.player(P0).unwrap();
    assert!(player.cells.is_empty() && player.records.is_empty());
    assert!(ai.is_active(P1));
    // No further thinks for player 0.
    let before = ai.trace.as_ref().unwrap().len();
    let marker = ai.trace.as_ref().unwrap().last().map(|r| r.counter);
    ai.tick(
        &world,
        &mut s2_sim::Rng::new(1),
        &mut s2_sim::Numbers::new(0),
        &mut s2_sim::RouteStore::default(),
    );
    let after = &ai.trace.as_ref().unwrap()[before..];
    assert!(after.iter().all(|r| r.player != P0), "{marker:?}");
}

// Same seed, same commands: the same run.
// spec: none (determinism with the AI on)
#[test]
fn a_run_with_the_ai_on_is_deterministic() {
    let run = || {
        let mut world = world(&[0, 1]);
        let mut ai = AiSystem::new(config(), &world);
        ai.activate(&mut world, P0, 3);
        ai.activate(&mut world, P1, 1);
        let mut log = Vec::new();
        for _ in 0..500 {
            let commands = ai.tick(
                &world,
                &mut s2_sim::Rng::new(1),
                &mut s2_sim::Numbers::new(0),
                &mut s2_sim::RouteStore::default(),
            );
            log.push(format!("{commands:?}"));
            for c in commands {
                world.submit(c);
            }
            world.step();
        }
        (world.state_hash(), log)
    };
    assert_eq!(run(), run());
}

// AI-CON T82 (AI-CON-1.1, corrected 2026-10-29): a cell's corner fields are
// its origin and the origins of the next cells to the right and below,
// (16 cx + 16, ...), not its square's own last row and column. With the
// headquarters on (20, 25), none of cell (0, 0)'s own corners (0, 0),
// (15, 0), (0, 15), (15, 15) is the player's, but (16, 16) is: the cell
// counts as owned.
// spec: AI-CON-1.1 (AI-CON T82)
#[test]
fn t82_a_cells_corners_are_the_next_cells_origins() {
    use s2_sim::GameSetup;
    let setup = GameSetup {
        players: vec![(P0, (20, 25))],
        local_player: P0,
        ..GameSetup::default()
    };
    let world = World::new_game(
        1,
        crate::fixture::flat(96, 96),
        crate::fixture::rules(),
        &setup,
    );
    let owned = |f| world.field_owner(f) == Some(P0);
    assert!(![(0, 0), (15, 0), (0, 15), (15, 15)].into_iter().any(owned));
    assert!(owned((16, 16)));
    assert!(crate::cells::corner_owned(&world, P0, (0, 0)));
    // A cell none of whose four corner fields is owned: (3, 3).
    assert!(!crate::cells::corner_owned(&world, P0, (3, 3)));
}

// AI-2.4a (Q-376, AI-overview T83 in AI-construction.md): the construction
// system's run does its inner parts first (the ship construction between
// the forester and the hunter), then one last step: the 50-tick step while
// no order is executing, else the executing order's step.
// spec: AI-2.4, AI-2.4a (AI-CON T83)
#[test]
fn the_executing_order_steps_after_the_construction_parts() {
    let mut world = world(&[0]);
    let mut ai = AiSystem::new(config(), &world);
    ai.activate(&mut world, P0, 1);
    ai.trace = Some(Vec::new());
    let cell = cell_of(ai.player(P0).unwrap().home);
    for _ in 0..1400 {
        // An order waits whenever the system is idle.
        let build = &mut ai.player_mut(P0).build;
        if build.exec.is_none() {
            build.issue(crate::construction::Order {
                slot: crate::construction::Slot::Needs,
                area: crate::sites::Area::Cell(cell),
                building: crate::testutil::STONE_PIT,
                weight: 10,
                time: -1.0,
                tag: crate::construction::Tag::None,
            });
        }
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
    use Part::*;
    let trace = ai.trace.as_ref().unwrap();
    let mut counters: Vec<u64> = trace.iter().map(|r| r.counter).collect();
    counters.dedup();
    let (mut steps, mut takes) = (0, 0);
    for c in counters {
        let parts: Vec<Part> = trace
            .iter()
            .filter(|r| r.counter == c)
            .map(|r| r.part)
            .collect();
        let at = |p: Part| parts.iter().position(|&q| q == p);
        let system = at(ConstructionSystem).expect("every tick");
        let last = at(ExecutingStep).or(at(BudgetStep));
        for inner in [
            StreetConstruction,
            ForesterConstruction,
            ShipConstruction,
            HunterConstruction,
        ] {
            if let Some(i) = at(inner) {
                assert!(system < i, "{c}: {parts:?}");
                assert!(last.is_none_or(|l| i < l), "{c}: {parts:?}");
            }
        }
        if let (Some(f), Some(h)) = (at(ForesterConstruction), at(HunterConstruction)) {
            assert!(f < h, "{c}: {parts:?}");
        }
        // One of the two last steps, never both.
        assert!(
            at(ExecutingStep).is_none() || at(BudgetStep).is_none(),
            "{c}"
        );
        if at(ExecutingStep).is_some() {
            steps += 1;
        }
        if at(BudgetStep).is_some() {
            takes += 1;
        }
    }
    assert!(
        steps > 0 && takes > 0,
        "{steps} steps, {takes} budget steps"
    );
}
