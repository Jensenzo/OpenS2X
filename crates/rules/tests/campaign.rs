//! `specs/CAM-campaign-missions.md` on the shipped campaign and tutorial
//! maps: T1, T2 and the scripts of tutorials 3 and 4 (CAM-5.1). Skipped
//! (passes) when `S2_GAME_DIR` is not set.

use std::path::PathBuf;

use s2_assets::{rc00, s2m, GameDir, GameDirError};
use s2_rules::mission::{self, ScriptEvent};
use s2_rules::RuleSet;
use s2_sim::{PlayerId, World};

fn install() -> Option<(GameDir, RuleSet)> {
    match GameDir::from_env() {
        Ok(dir) => {
            let rules = s2_rules::load(&dir).unwrap_or_else(|e| panic!("{e}")).0;
            Some((dir, rules))
        }
        Err(GameDirError::NotSet) => {
            eprintln!("skipped: S2_GAME_DIR is not set");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

/// The map's path, its world with its mission, and its script's events of
/// `onLoaded` and `onStarted` (commands submitted).
fn start(
    dir: &GameDir,
    rules: &RuleSet,
    path: &str,
) -> (World, mission::MissionScript, Vec<ScriptEvent>) {
    let file: PathBuf = s2_rules::resolve(dir.root(), path).unwrap();
    let data = std::fs::read(&file).unwrap();
    let data = if rc00::is_wrapped(&data) {
        rc00::decode(file.to_str().unwrap(), &data).unwrap()
    } else {
        data
    };
    let map = s2m::parse(&data).unwrap();
    let mut world = s2_rules::new_game(1, &map, rules, 0).unwrap();
    world.set_mission(mission::mission_of(&map));
    let script = mission::script_for(&file, rules)
        .unwrap()
        .expect("a script");
    let mut events = mission::submit(&mut world, script.on_loaded().unwrap());
    events.extend(mission::submit(&mut world, script.on_started().unwrap()));
    (world, script, events)
}

fn step(world: &mut World, script: &mission::MissionScript) -> Vec<ScriptEvent> {
    world.step();
    let events = script.after_step(world).unwrap();
    mission::submit(world, events)
}

// spec: CAM-1.3, CAM-4.3, ECO-HQ-2.2
#[test]
fn campaign01_briefing_soldiers_and_locks() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let (mut world, script, events) = start(&dir, &rules, r"data\maps\campaignMaps\campaign01.s2m");
    assert_eq!(
        events,
        vec![
            ScriptEvent::AddDebriefing("b01_C_00".into()),
            ScriptEvent::AddDebriefing("b01_C_01".into()),
        ]
    );
    let m = world.mission().unwrap();
    assert_eq!(m.target, 4);
    let names: Vec<&str> = m.triggers.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Region_K", "RegionVictory__0"]);
    let hq = world.buildings().next().unwrap().at;
    let soldiers = world.stock_soldiers(hq)[0];
    let mut book = Vec::new();
    while world.tick() < 260 {
        let e = step(&mut world, &script);
        if world.tick() == 1 {
            // T1: 40 privates more than the start stock, ten types locked.
            // ECO-HQ-2.2 (R13, Q-1002 confirmed): one of them stands outside
            // on (33, 34), the headquarters' soldier, the rest in stock.
            assert_eq!(world.stock_soldiers(hq)[0], soldiers + 39);
            let outside: Vec<_> = world
                .settlers()
                .filter(|(_, s)| matches!(s.job, s2_sim::SettlerJob::Garrison { target, .. } if target.at == hq))
                .map(|(_, s)| s.walker.at)
                .collect();
            assert_eq!(outside, [(33, 34)]);
            let locked = rules
                .buildings
                .iter()
                .filter(|b| world.is_locked(PlayerId(0), b.id))
                .count();
            assert_eq!(locked, 10);
        }
        if !e.is_empty() && book.is_empty() {
            book = e;
            // T1: the 26th second, the ten ticks 250 to 259.
            assert_eq!(world.tick(), 251);
        }
    }
    assert_eq!(
        book,
        vec![
            ScriptEvent::OpenBook("b01_A_00".into()),
            ScriptEvent::AddPage("b01_A_01".into()),
            ScriptEvent::AddPicture("ui_briefing_screen_1".into()),
        ]
    );
}

// spec: CAM-5.1, CAM-1.3
#[test]
fn tutorial_scripts_build_their_scenes() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let (mut world, script, _) = start(&dir, &rules, r"data\maps\tutorialMaps\tutorial03.s2m");
    step(&mut world, &script);
    for result in world.last_results() {
        assert_eq!(result.outcome, Ok(()), "{:?}", result.command);
    }
    // CAM-5.1: a guard house with a sergeant-rank soldier, a watchtower,
    // the enemy's barracks.
    assert_eq!(world.garrison((51, 36)).len(), 1);
    assert!(world.building_at((51, 38)).is_some());
    assert_eq!(world.building_at((55, 29)).unwrap().owner, PlayerId(1));

    let (mut world, script, _) = start(&dir, &rules, r"data\maps\tutorialMaps\tutorial04.s2m");
    step(&mut world, &script);
    for result in world.last_results() {
        assert_eq!(result.outcome, Ok(()), "{:?}", result.command);
    }
    assert_eq!(world.ships().len(), 2);
}

// spec: CAM-1.1
#[test]
fn free_maps_have_no_script() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let free = s2_rules::resolve(dir.root(), r"data\maps\freeGameMaps").unwrap();
    let map = std::fs::read_dir(free)
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    assert_eq!(mission::map_kind(&map), 0);
    assert!(mission::script_for(&map, &rules).unwrap().is_none());
}

// spec: SHIP-6.1 (T15), CAM-5.3a
#[test]
fn lesson_four_ships_wait_with_their_landings() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let (mut world, script, _) = start(&dir, &rules, r"data\maps\tutorialMaps\tutorial04.s2m");
    step(&mut world, &script);
    step(&mut world, &script);
    // CAM-5.3a: both ships made by createShip, in expedition mode with the
    // load aboard, waiting for orders.
    let ships = world.ships();
    assert_eq!(ships.len(), 2);
    assert_eq!(ships[0].position(), (101, 63));
    assert_eq!(ships[1].position(), (127, 36));
    for s in ships {
        assert!(s.vessel.expedition);
        assert!(!s.vessel.aboard.is_empty());
    }
    // SHIP-6.1 (T15): the spot (129, 34) lands on (127, 34), the first Water
    // field of ring 2 in ring order, not on (127, 36).
    let landing = ships[1]
        .vessel
        .options
        .iter()
        .flatten()
        .find(|l| l.spot == (129, 34))
        .expect("an option for the spot (129, 34)");
    assert_eq!(landing.field, (127, 34));
    // CAM-5.3a 2: the first ship's SouthWest button (direction 2) is lit.
    assert!(ships[0].vessel.options[2].is_some());
}

/// CAM-2.1, FMT-S2M-3.3 (Q-187 a): a trigger's player is the `u32` after
/// its name: 2 for campaign 5's `RegionVictory__1` to `__3`, 0 for its
/// `RegionVictory__0` and every other campaign trigger; only the human's
/// victory region starts explored for the human (CAM-3.6).
// spec: CAM-2.1, CAM-3.6, FMT-S2M-3.3
#[test]
fn a_triggers_player_is_the_number_after_its_name() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let (world, _, _) = start(&dir, &rules, r"data\maps\campaignMaps\campaign05.s2m");
    let players: Vec<(&str, u8)> = world
        .mission()
        .unwrap()
        .triggers
        .iter()
        .map(|t| (t.name.as_str(), t.player.0))
        .collect();
    assert_eq!(
        players,
        [
            ("RegionVictory__0", 0),
            ("RegionVictory__1", 2),
            ("RegionVictory__2", 2),
            ("RegionVictory__3", 2),
        ]
    );
    for n in 1..=4 {
        let path = format!(r"data\maps\campaignMaps\campaign0{n}.s2m");
        let (world, _, _) = start(&dir, &rules, &path);
        assert!(
            world
                .mission()
                .unwrap()
                .triggers
                .iter()
                .all(|t| t.player.0 == 0),
            "{path}"
        );
    }
}
