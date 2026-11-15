//! `specs/CAM-campaign-missions.md` played: the missions as the menu starts
//! them, mission 1 to its victory and past a save, every mission and tutorial
//! for 3,000 ticks with the soak's invariants. Skipped (passes) when
//! `S2_GAME_DIR` is not set. Everything stays in the test's memory: no save
//! or profile file is written (`PLAYING.md`).

use s2_ai::mission::{MissionGame, TerritoryPlayer};
use s2_assets::{GameDir, GameDirError};
use s2_rules::mission::ScriptEvent;
use s2_rules::RuleSet;
use s2_sim::invariants::Checker;
use s2_sim::mission::{target, LossReason, MissionEvent, Outcome};
use s2_sim::{Command, PlayerId};

const HUMAN: PlayerId = PlayerId(0);
/// CAM T3: mission 1's victory region, on the field of its trigger.
const GATE: (u32, u32) = (64, 70);

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

/// The player of mission 1: only barracks may be built of the military
/// buildings (CAM-4.4).
fn mission_one_player(game: &MissionGame, rules: &RuleSet) -> TerritoryPlayer {
    let barracks = rules.building_named("Barracks").unwrap().id;
    TerritoryPlayer::new(&game.world, rules, HUMAN, GATE, barracks).unwrap()
}

/// How a game went from here to the mission's completion.
#[derive(Default)]
struct Played {
    /// The script's books: the tick and the first page.
    books: Vec<(u64, String)>,
    victory: Option<u64>,
}

/// Play on until the mission is completed (or `until`), checking the soak's
/// invariants every tick.
fn play(game: &mut MissionGame, player: &mut TerritoryPlayer, until: u64) -> Played {
    let mut played = Played::default();
    let mut checker = Checker::new(&game.world);
    while game.world.tick() < until && played.victory.is_none() {
        player.act(&mut game.world);
        for event in game.step() {
            // The script asks in every tick of the second; the book is modal
            // (CAM-1.2).
            if let ScriptEvent::OpenBook(key) = event {
                if played.books.last().is_none_or(|b| b.1 != key) {
                    played.books.push((game.world.tick(), key));
                }
            }
        }
        assert_eq!(game.problems, Vec::<String>::new());
        let broken = checker.check(&game.world);
        assert!(broken.is_empty(), "{broken:?}");
        let completed = game
            .world
            .mission_events()
            .iter()
            .any(|e| matches!(e, MissionEvent::Completed { value: 4 }));
        if completed {
            played.victory = Some(game.world.tick());
        }
    }
    played
}

// CAM T1, T2, T3 on the shipped mission: from the start to the magic gate
// with a player's commands only. The tick and what it took are printed for
// the report.
// spec: CAM-1.2, CAM-2.1, CAM-2.2, CAM-2.3, CAM-3.1, CAM-3.5, CAM-4.3, CAM-4.4 (T1, T2, T3)
#[test]
fn mission_one_is_won_by_reaching_the_gate() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    assert_eq!(game.world.mission().unwrap().target, target::PORTAL);
    assert_eq!(game.saved.debriefing, ["b01_C_00", "b01_C_01"]);
    let mut player = mission_one_player(&game, &rules);
    let played = play(&mut game, &mut player, 40_000);
    let tick = played.victory.expect("the gate is reached");
    eprintln!(
        "mission 1: victory in tick {tick} ({} min of game time), books {:?}: barracks {:?}, \
         {} roads, {} flags, {} sites given up, refused {:?}",
        tick / 600,
        played.books,
        player.log.placed,
        player.log.roads,
        player.log.flags,
        player.log.given_up,
        player.log.refused,
    );
    // T1 the briefing in the 26th second; T2 the second book once, on the way.
    assert_eq!(played.books[0], (251, "b01_A_00".to_owned()));
    let second: Vec<&str> = played.books[1..].iter().map(|b| b.1.as_str()).collect();
    assert_eq!(second, ["b01_B_00"]);
    assert!(played.books[1].0 < tick);
    // T2: `Region_K` is gone; T3: the map is completed with value 4.
    let mission = game.world.mission().unwrap();
    assert!(!mission.triggers.iter().any(|t| t.name == "Region_K"));
    assert_eq!(mission.completed, Some(target::PORTAL));
    assert_eq!(game.world.field_owner(GATE), Some(HUMAN));
    // CAM-3.3: the map's target 4 never wins by the victory test: only the
    // region did.
    assert_eq!(game.world.outcome(HUMAN), None);
    assert!(player.log.given_up == 0, "{:?}", player.log);
}

// CAM T6 on the shipped mission, on a prepared world: the headquarters gone
// (MIL-CON-2.1 destroys it; here its owner's command does it at once).
// spec: CAM-3.2 (T6)
#[test]
fn mission_one_is_lost_with_the_headquarters() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    let mut checker = Checker::new(&game.world);
    for _ in 0..100 {
        game.step();
    }
    let hq = game.world.buildings().next().unwrap().at;
    game.world.submit(Command::RemoveBuilding {
        player: HUMAN,
        at: hq,
    });
    game.step();
    // The tick that was running when the mission looked.
    let tick = game.world.tick() - 1;
    assert_eq!(
        game.world.outcome(HUMAN),
        Some(Outcome::Lost {
            tick,
            reason: LossReason::CastleDestroyed
        })
    );
    assert_eq!(
        game.world.mission_events(),
        [MissionEvent::Lost {
            player: HUMAN,
            reason: LossReason::CastleDestroyed
        }]
    );
    // The map is not completed; the game goes on behind the end screen.
    assert_eq!(game.world.mission().unwrap().completed, None);
    assert_eq!(checker.check(&game.world), Vec::new());
}

// A defeated computer player does not end a mission whose target is the gate
// (CAM-3.2, 3.3): only the human hears of it.
// spec: CAM-3.2, CAM-3.3
#[test]
fn a_lost_computer_player_does_not_end_the_mission() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let mut game = MissionGame::start(&dir, &rules, "campaign02").unwrap();
    let enemy = PlayerId(1);
    game.step();
    let hq = game
        .world
        .buildings()
        .find(|b| b.owner == enemy)
        .unwrap()
        .at;
    game.world.submit(Command::RemoveBuilding {
        player: enemy,
        at: hq,
    });
    game.step();
    assert!(matches!(
        game.world.outcome(enemy),
        Some(Outcome::Lost {
            reason: LossReason::CastleDestroyed,
            ..
        })
    ));
    assert_eq!(game.world.outcome(HUMAN), None);
    assert_eq!(game.world.mission().unwrap().completed, None);
}

/// CAM-3.1a (T11, T12): every campaign mission plays with target 4 (reach
/// the portal), missions 4 to 10 too though their headers say 0; a tutorial
/// lesson with 0.
// spec: CAM-3.1a (T11, T12)
#[test]
fn every_mission_plays_with_target_4() {
    let Some((dir, rules)) = install() else {
        return;
    };
    for n in 1..=10 {
        let name = format!("campaign{n:02}");
        let game = MissionGame::start(&dir, &rules, &name).unwrap();
        let target = game.world.mission().map(|m| m.target);
        assert_eq!(target, Some(target::PORTAL), "{name}");
    }
    for n in 1..=4 {
        let name = format!("tutorial{n:02}");
        let game = MissionGame::start(&dir, &rules, &name).unwrap();
        assert_eq!(game.world.mission().map(|m| m.target), Some(0), "{name}");
    }
}

/// CAM-3.1b (T15): right after a campaign game's set-up the human has
/// explored his castle's area and exactly the 19 fields within distance 2
/// of each victory region: mission 4's at (53, 26), mission 1's at (64, 70);
/// nothing around mission 1's `Region_K` (55, 51), an explored trigger.
// spec: CAM-3.1b (T15)
#[test]
fn the_gates_surroundings_start_explored() {
    let Some((dir, rules)) = install() else {
        return;
    };
    for (name, gate, other) in [
        ("campaign04", (53, 26), None),
        ("campaign01", GATE, Some((55, 51))),
    ] {
        let game = MissionGame::start(&dir, &rules, name).unwrap();
        let world = &game.world;
        let castle = world
            .warehouses()
            .map(|(at, _)| at)
            .find(|&at| world.building_at(at).is_some_and(|b| b.owner == HUMAN))
            .expect("the human's castle");
        let (w, h) = (world.terrain().width, world.terrain().height);
        let beyond: Vec<(u32, u32)> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&f| world.is_explored(HUMAN, f))
            .filter(|&f| s2_sim::grid::distance(f, castle) > 12)
            .collect();
        eprintln!(
            "{name}: castle {castle:?}, explored beyond it {}",
            beyond.len()
        );
        assert_eq!(beyond.len(), 19, "{name}: {beyond:?}");
        assert!(beyond.iter().all(|&f| s2_sim::grid::distance(f, gate) <= 2));
        if let Some(field) = other {
            assert!(s2_sim::grid::distance(field, castle) > 12);
            assert!(!world.is_explored(HUMAN, field), "{name}");
        }
    }
}

/// CAM-4.3a's table: the ten missions' players from their maps: tribe, team and
/// profile; same-team players are allies. The human is Romans, colour 0.
// spec: CAM-4.3a, FMT-S2M-2.5, FMT-S2M-2.6, UI-SETUP-1.5, UI-SETUP-2.4, AI-1.1
#[test]
fn the_missions_have_the_players_of_their_maps() {
    let Some((dir, rules)) = install() else {
        return;
    };
    // (slot, tribe, colour, team, profile); profile 1 Weak, 2 Normal, 3 Strong.
    type Computer = (u8, u8, u8, i8, u8);
    #[rustfmt::skip]
    let table: [(i8, &[Computer]); 10] = [
        (1, &[]),
        (1, &[(1, 1, 3, 0, 1)]),
        (1, &[(1, 1, 3, 0, 1)]),
        (0, &[(1, 1, 1, 1, 2), (2, 1, 2, 1, 2)]),
        (1, &[(1, 2, 1, 0, 3), (2, 1, 5, 1, 2), (3, 2, 1, 0, 1)]),
        (1, &[(1, 2, 6, 0, 2), (2, 2, 3, 0, 2)]),
        (1, &[(1, 2, 1, 0, 3), (2, 2, 1, 0, 2)]),
        (0, &[(1, 2, 1, 1, 3), (2, 2, 2, 1, 3)]),
        (0, &[(1, 2, 3, 1, 3), (2, 2, 2, 1, 2), (3, 2, 6, 1, 3)]),
        (0, &[(1, 2, 1, 1, 3), (2, 2, 2, 1, 3), (3, 2, 3, 1, 3)]),
    ];
    for (n, (human_team, computers)) in table.into_iter().enumerate() {
        let name = format!("campaign{:02}", n + 1);
        let file = s2_assets::find_map(&dir, &name).unwrap();
        let shown = file.display().to_string();
        let map = s2_assets::s2m::load(&shown, &std::fs::read(&file).unwrap()).unwrap();
        let players = s2_rules::mission::players(&map, 1);
        let want: Vec<_> = std::iter::once((0, 0, 0, human_team, None))
            .chain(
                computers
                    .iter()
                    .map(|&(slot, tribe, colour, team, profile)| {
                        (slot, tribe, colour, team, Some(profile))
                    }),
            )
            .collect();
        let got: Vec<_> = players
            .iter()
            .map(|p| (p.player.0, p.tribe, p.colour, p.team, p.profile))
            .collect();
        assert_eq!(got, want, "{name}");
        let game = MissionGame::start_on(&file, &map, &rules).unwrap();
        assert_eq!(
            game.slots,
            computers
                .iter()
                .map(|c| (PlayerId(c.0), c.4))
                .collect::<Vec<_>>(),
            "{name}"
        );
        for a in &players {
            assert!(game.world.is_player(a.player), "{name}");
            assert_eq!(game.world.tribe(a.player), a.tribe, "{name}");
            for b in &players {
                assert_eq!(
                    game.world.are_enemies(a.player, b.player),
                    a.player != b.player && a.team != b.team,
                    "{name}: {a:?} {b:?}"
                );
            }
        }
    }
}

/// ECO-HQ-2.2, CAM-4.3: campaign 1's headquarters at the start: x 2 of the base
/// stock, whatever an earlier game used, with the script's 40 privates on top.
/// Lessons start with x 4.
// spec: ECO-HQ-2.2, CAM-4.3, UI-SETUP-2.4
#[test]
fn the_start_stock_of_missions_and_lessons() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let stock_of = |game: &MissionGame, id: u32| -> u32 {
        game.world
            .warehouses()
            .filter(|(at, _)| {
                game.world
                    .building_at(*at)
                    .is_some_and(|b| b.owner == HUMAN)
            })
            .flat_map(|(_, s)| s.iter())
            .filter(|(g, _)| *g == id)
            .map(|(_, n)| *n)
            .sum()
    };
    let stock = |game: &MissionGame, good: &str| stock_of(game, rules.id_named(good).unwrap());
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    assert_eq!(stock(&game, "plank"), 44);
    assert_eq!(stock(&game, "stone"), 68);
    // The script's start commands are applied in the first tick.
    game.step();
    game.step();
    assert_eq!(stock(&game, "plank"), 44);
    assert_eq!(stock(&game, "stone"), 68);
    assert_eq!(stock(&game, "good_helper"), 52);
    assert_eq!(stock(&game, "good_constructor"), 10);
    let private = rules.military.soldiers[0].unwrap();
    // One may have left the headquarters already: the stock and those outside.
    let outside = game
        .world
        .settlers()
        .filter(|(_, s)| s.player == HUMAN && s.profession == private)
        .count() as u32;
    assert_eq!(stock_of(&game, private) + outside, 86);
    let lesson = MissionGame::start(&dir, &rules, "tutorial01").unwrap();
    assert_eq!(stock(&lesson, "plank"), 88);
}

/// Mission 1 saved at `at`, loaded, and played on by a new player who only
/// knows the world: the script's variables back, the goals as before, and
/// the same victory as a game that was never saved.
fn saved_at(at: u64, straight: u64, straight_hash: u64, book_after_load: bool) {
    let (dir, rules) = install().unwrap();
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    let mut player = mission_one_player(&game, &rules);
    let before = play(&mut game, &mut player, at);
    assert_eq!(before.victory, None);
    assert_eq!(game.world.tick(), at);
    let bytes = game.save();

    let mut loaded = MissionGame::load(&bytes, &rules).unwrap();
    assert_eq!(loaded.world.tick(), at);
    assert_eq!(loaded.world.state_hash(), game.world.state_hash());
    // FMT-SAV-3.12: the script's variables, from the script loaded again.
    let script = loaded.script.as_ref().expect("the script is loaded again");
    assert!(!game.saved.script_variables.is_empty());
    assert_eq!(script.variables(), game.saved.script_variables);
    // The goals: the target, the triggers still to come, the locks, the
    // debriefing.
    assert_eq!(loaded.saved, game.saved);
    assert_eq!(loaded.world.mission(), game.world.mission());
    let barracks = rules.building_named("Barracks").unwrap().id;
    let guard_house = rules.building_named("GuardHouse").unwrap().id;
    assert!(loaded.world.is_locked(HUMAN, guard_house));
    assert!(!loaded.world.is_locked(HUMAN, barracks));

    let mut again = mission_one_player(&loaded, &rules);
    let after = play(&mut loaded, &mut again, 40_000);
    assert_eq!(after.victory, Some(straight), "as if it had not been saved");
    assert_eq!(loaded.world.state_hash(), straight_hash);
    // T2: the second book comes after the load or came before: once either
    // way.
    let books: Vec<&str> = before
        .books
        .iter()
        .chain(&after.books)
        .map(|b| b.1.as_str())
        .collect();
    assert_eq!(books, ["b01_A_00", "b01_B_00"]);
    assert_eq!(
        after.books.iter().any(|b| b.1 == "b01_B_00"),
        book_after_load
    );
}

// spec: CAM-1.1, FMT-SAV-3.12 (T8), CAM-2.3
#[test]
fn mission_one_saved_on_the_way_still_reaches_the_gate() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    let mut player = mission_one_player(&game, &rules);
    let played = play(&mut game, &mut player, 40_000);
    let tick = played.victory.unwrap();
    let hash = game.world.state_hash();
    let second_book = played.books[1].0;
    // Before the first briefing, before the second, after it.
    saved_at(100, tick, hash, true);
    saved_at(second_book - 1000, tick, hash, true);
    saved_at(second_book + 1000, tick, hash, false);
}

// A mission with a computer player saved and loaded plays on as if it had
// not been saved, the computer player's state included.
// spec: CAM-1.1, FMT-SAV-3.12 (T8)
#[test]
fn a_mission_with_a_computer_player_plays_on_from_its_save() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let mut game = MissionGame::start(&dir, &rules, "campaign02").unwrap();
    for _ in 0..1000 {
        game.step();
    }
    let bytes = game.save();
    let mut loaded = MissionGame::load(&bytes, &rules).unwrap();
    assert_eq!(loaded.slots, [(PlayerId(1), 1)]);
    for _ in 0..1000 {
        game.step();
        loaded.step();
    }
    assert_eq!(loaded.problems, Vec::<String>::new());
    assert_eq!(loaded.world.state_hash(), game.world.state_hash());
    assert_eq!(
        loaded.ai.as_ref().unwrap().state_hash(),
        game.ai.as_ref().unwrap().state_hash()
    );
}

/// CAM-4.4: the shipped briefing of campaign mission `n`, 25 s in.
fn briefing(n: u32) -> Vec<ScriptEvent> {
    vec![
        ScriptEvent::OpenBook(format!("b{n:02}_A_00")),
        ScriptEvent::AddPage(format!("b{n:02}_A_01")),
        ScriptEvent::AddPicture(format!("ui_briefing_screen_{n}")),
    ]
}

/// What the script asked of the interface, by tick.
type Said = Vec<(u64, Vec<ScriptEvent>)>;

/// A mission as the menu starts it, 3,000 ticks of it: no script error, no
/// refused script command, every soak invariant after every tick. The script
/// has spoken at the start and, for a campaign mission, at 25 s.
fn soak_mission(name: &str) -> Option<(MissionGame, Said)> {
    let (dir, rules) = install()?;
    let mut game = MissionGame::start(&dir, &rules, name).unwrap();
    let mut checker = Checker::new(&game.world);
    let mut said = Vec::new();
    for _ in 0..3000 {
        let events = game.step();
        if !events.is_empty() {
            said.push((game.world.tick(), events));
        }
        let broken = checker.check(&game.world);
        assert!(broken.is_empty(), "{name}: {broken:?}");
        assert_eq!(game.problems, Vec::<String>::new(), "{name}");
        assert!(
            game.world.outcome(HUMAN).is_none(),
            "{name}: over in tick {}",
            game.world.tick()
        );
    }
    assert_eq!(game.world.mission().unwrap().completed, None, "{name}");
    let computers = game.world.buildings().filter(|b| b.owner != HUMAN).count();
    eprintln!(
        "{name}: tick 3000: {} buildings of the computer players, {} settlers, {} wares",
        computers,
        game.world.settlers().count(),
        game.world.wares().count()
    );
    Some((game, said))
}

macro_rules! campaign_mission {
    ($test:ident, $n:literal) => {
        // spec: CAM-1.1, CAM-1.2, CAM-3.1, CAM-3.1a, CAM-4.3, CAM-4.4
        #[test]
        fn $test() {
            let name = format!("campaign{:02}", $n);
            let Some((game, said)) = soak_mission(&name) else {
                return;
            };
            // CAM-4.4: the debriefing's two pages, the briefing at 25 s.
            assert_eq!(
                game.saved.debriefing,
                [format!("b{:02}_C_00", $n), format!("b{:02}_C_01", $n)]
            );
            // CAM-3.1a: every mission plays with target 4.
            assert_eq!(game.world.mission().unwrap().target, target::PORTAL);
            let first = &said.first().expect("the briefing").1;
            assert_eq!(first, &briefing($n), "{name}");
            assert_eq!(said[0].0, 251, "{name}");
            // The script asks in every tick of the 26th second; the book is
            // modal (CAM-1.2). Nothing after that second but `Region_K`.
            for (tick, events) in &said {
                assert!((251..=260).contains(tick), "{name}: {tick} {events:?}");
            }
        }
    };
}

campaign_mission!(campaign01_starts_and_plays, 1);
campaign_mission!(campaign02_starts_and_plays, 2);
campaign_mission!(campaign03_starts_and_plays, 3);
campaign_mission!(campaign04_starts_and_plays, 4);
campaign_mission!(campaign05_starts_and_plays, 5);
campaign_mission!(campaign06_starts_and_plays, 6);
campaign_mission!(campaign07_starts_and_plays, 7);
campaign_mission!(campaign08_starts_and_plays, 8);
campaign_mission!(campaign09_starts_and_plays, 9);
campaign_mission!(campaign10_starts_and_plays, 10);

macro_rules! tutorial_lesson {
    ($test:ident, $name:literal, $players:literal) => {
        // spec: CAM-1.1, CAM-5.1, UI-SETUP-2.4
        #[test]
        fn $test() {
            let Some((game, said)) = soak_mission($name) else {
                return;
            };
            assert!(said.is_empty(), "{}: {said:?}", $name);
            assert!(game.saved.debriefing.is_empty());
            assert_eq!(game.slots.len(), $players, "{}", $name);
        }
    };
}

tutorial_lesson!(tutorial01_starts_and_plays, "tutorial01", 0);
tutorial_lesson!(tutorial02_starts_and_plays, "tutorial02", 0);
tutorial_lesson!(tutorial03_starts_and_plays, "tutorial03", 1);
tutorial_lesson!(tutorial04_starts_and_plays, "tutorial04", 3);

/// Each player's buildings, soldiers by rank and goods.
fn print_players(world: &s2_sim::World, rules: &RuleSet) {
    for p in 0..8u8 {
        let player = PlayerId(p);
        let mut buildings = std::collections::BTreeMap::<String, u32>::new();
        let mut garrison = [0u32; 5];
        let mut stock = [0u32; 5];
        for b in world.buildings().filter(|b| b.owner == player) {
            let name = rules
                .buildings
                .iter()
                .find(|t| t.id == b.building_type)
                .and_then(|t| t.name.clone())
                .unwrap_or_default();
            *buildings.entry(name).or_default() += 1;
            if let Some(m) = world.military_at(b.at) {
                for &id in &m.soldiers {
                    let settler = world.settlers().find(|(i, _)| *i == id);
                    if let Some(rank) = settler.and_then(|(_, s)| world.soldier_rank(s.profession))
                    {
                        garrison[rank] += 1;
                    }
                }
            }
            for (k, n) in world.stock_soldiers(b.at).iter().enumerate() {
                stock[k] += n;
            }
        }
        if !world.is_player(player) && buildings.is_empty() {
            continue;
        }
        let goods: Vec<(String, u32)> = world
            .warehouses()
            .filter(|(at, _)| world.building_at(*at).is_some_and(|b| b.owner == player))
            .flat_map(|(_, s)| s.iter().copied())
            .filter(|&(_, n)| n > 0)
            .map(|(g, n)| {
                let good = rules.goods.iter().find(|t| t.id == g);
                (good.and_then(|t| t.name.clone()).unwrap_or_default(), n)
            })
            .collect();
        eprintln!(
            "  player {p}: is_player {} tribe {} buildings {buildings:?} \
             garrison by rank {garrison:?} in stock by rank {stock:?}\n    goods {goods:?}",
            world.is_player(player),
            world.tribe(player),
        );
    }
}

/// F57: who plays each mission and what stands on its map at the start, as
/// we start it: for the report and `docs/test-coverage.md`. Prints only.
// spec: CAM-4.3, CAM-4.4, UI-SETUP-2.4, FMT-S2M-2.5, FMT-S2M-2.6
#[test]
#[ignore = "prints the missions' players, about 20 s in release"]
fn mission_rosters() {
    let Some((dir, rules)) = install() else {
        return;
    };
    let names = (1..=10)
        .map(|n| format!("campaign{n:02}"))
        .chain((1..=4).map(|n| format!("tutorial{n:02}")));
    for name in names {
        let file = s2_assets::find_map(&dir, &name).unwrap();
        let shown = file.display().to_string();
        let map = s2_assets::s2m::load(&shown, &std::fs::read(&file).unwrap()).unwrap();
        let mut game = MissionGame::start_on(&file, &map, &rules).unwrap();
        // The script's start commands are applied in the first tick.
        for _ in 0..2 {
            game.step();
        }
        eprintln!(
            "{name}: {} x {}, starts {:?}, player types {:?}, records {:?}, info {:?}, slots {:?}",
            map.info.width,
            map.info.height,
            map.info
                .start_positions
                .iter()
                .map(|p| (p.x, p.y))
                .collect::<Vec<_>>(),
            map.info.player_types,
            map.info.player_records,
            map.info.info_values,
            game.slots,
        );
        print_players(&game.world, &rules);
        let world = &game.world;
        for t in &world.mission().unwrap().triggers {
            eprintln!(
                "  trigger {} {:?} at {:?} player {:?}",
                t.name, t.kind, t.field, t.player
            );
        }
    }
}

/// Asked on 2026-11-04: why a way over land from the start to the gate
/// is not found: for each mission, how far the search gets when each kind of
/// obstacle is taken away. Prints only.
// spec: CAM-2.4, FMT-S2M-3.3, TRN-ROAD-1
#[test]
#[ignore = "prints what lies between each mission's start and its gate"]
fn what_lies_between_start_and_gate() {
    use s2_sim::map::state;
    let Some((dir, rules)) = install() else {
        return;
    };
    let all = state::WATER
        | state::BLOCKED
        | state::MOVEMENT_BLOCKED
        | state::DEPOSIT_BLOCKED
        | state::DOODAD_BLOCKED
        | state::ELEVATION_BLOCKED;
    let masks = [
        ("all", all),
        ("no-deposit", all & !state::DEPOSIT_BLOCKED),
        ("no-doodad", all & !state::DOODAD_BLOCKED),
        ("no-elev", all & !state::ELEVATION_BLOCKED),
        ("no-move", all & !state::MOVEMENT_BLOCKED),
        ("no-blocked", all & !state::BLOCKED),
        ("water+blocked", state::WATER | state::BLOCKED),
        ("water", state::WATER),
    ];
    for kind in rules.buildings.iter() {
        if kind.territory_range.unwrap_or(0) > 0 {
            eprintln!(
                "{:?}: territory range {:?}, soldier limit {:?}",
                kind.name, kind.territory_range, kind.soldier_limit
            );
        }
    }
    for n in 1..=10 {
        let name = format!("campaign{n:02}");
        let game = MissionGame::start(&dir, &rules, &name).unwrap();
        let world = &game.world;
        let t = world.terrain();
        let hq = world
            .buildings()
            .find(|b| b.owner == HUMAN)
            .map(|b| b.at)
            .unwrap();
        let start = t.neighbour(hq, 1).unwrap();
        let gates: Vec<(u32, u32)> = world
            .mission()
            .unwrap()
            .triggers
            .iter()
            .filter(|tr| tr.kind == s2_sim::mission::TriggerKind::Victory)
            .map(|tr| tr.field)
            .collect();
        let gate = gates[0];
        let mut line = format!(
            "{name}: start {start:?} gates {gates:?} distance {}",
            s2_sim::grid::distance(start, gate)
        );
        for (label, mask) in masks {
            let mut seen = vec![false; t.len()];
            seen[t.index(start)] = true;
            let mut open = vec![start];
            let mut count = 0;
            while let Some(f) = open.pop() {
                count += 1;
                for next in t.neighbours(f).into_iter().flatten() {
                    let i = t.index(next);
                    if !seen[i] && !t.has(next, mask) {
                        seen[i] = true;
                        open.push(next);
                    }
                }
            }
            line += &format!(
                " {label}:{}{count}",
                if gates.iter().any(|g| {
                    seen[t.index(*g)]
                        || t.neighbours(*g)
                            .into_iter()
                            .flatten()
                            .any(|f| seen[t.index(f)])
                }) {
                    "REACHED/"
                } else {
                    "no/"
                }
            );
            // What the gate's own surroundings are.
        }
        eprintln!("{line}");
        // Land a player can reach with roads once the trees and stones are
        // gone (TRN-ROAD-1.1): the start's, and the gate's.
        let road = all & !state::DEPOSIT_BLOCKED;
        let region = |from: Vec<(u32, u32)>| {
            let mut seen = vec![false; t.len()];
            for &f in &from {
                seen[t.index(f)] = true;
            }
            let mut open = from;
            while let Some(f) = open.pop() {
                for next in t.neighbours(f).into_iter().flatten() {
                    let i = t.index(next);
                    if !seen[i] && !t.has(next, road) {
                        seen[i] = true;
                        open.push(next);
                    }
                }
            }
            seen
        };
        let mine = region(vec![start]);
        let beyond = region(gates.clone());
        let count = |seen: &[bool], mask: u32| {
            t.fields()
                .filter(|&f| seen[t.index(f)] && t.has(f, mask))
                .count()
        };
        let nearest_site = t
            .fields()
            .filter(|&f| mine[t.index(f)] && t.has(f, state::FOR_BUILDING))
            .map(|f| s2_sim::grid::distance(f, gate))
            .min();
        // The fewest steep or other unusable fields a line of land must cross.
        let mut cross = vec![u32::MAX; t.len()];
        cross[t.index(start)] = 0;
        let mut deque = std::collections::VecDeque::from([start]);
        while let Some(f) = deque.pop_front() {
            let here = cross[t.index(f)];
            for next in t.neighbours(f).into_iter().flatten() {
                let extra = u32::from(t.has(next, state::ELEVATION_BLOCKED));
                if t.has(next, state::WATER | state::BLOCKED | state::DOODAD_BLOCKED)
                    && !gates.contains(&next)
                {
                    continue;
                }
                let i = t.index(next);
                if here + extra < cross[i] {
                    cross[i] = here + extra;
                    if extra == 0 {
                        deque.push_front(next);
                    } else {
                        deque.push_back(next);
                    }
                }
            }
        }
        let enemies: Vec<String> = world
            .buildings()
            .filter(|b| b.owner != HUMAN)
            .map(|b| {
                let flag = t.neighbour(b.at, 1).unwrap();
                format!(
                    "{:?}@{:?}:{}{}",
                    b.owner.0,
                    b.at,
                    if mine[t.index(flag)] { "mine " } else { "" },
                    if beyond[t.index(flag)] { "gate's" } else { "" }
                )
            })
            .collect();
        eprintln!(
            "  my land {} fields ({} harbour), the gate's land {} fields ({} harbour), same land {}, \
             nearest buildable field of mine to the gate {nearest_site:?}, \
             steep fields to cross {}; other castles {enemies:?}",
            mine.iter().filter(|&&b| b).count(),
            count(&mine, state::FOR_HARBOR),
            beyond.iter().filter(|&&b| b).count(),
            count(&beyond, state::FOR_HARBOR),
            mine[t.index(gate)] || t.neighbours(gate).into_iter().flatten().any(|f| mine[t.index(f)]),
            cross[t.index(gate)],
        );
        let bits = |f: (u32, u32)| {
            let mut out = String::new();
            for (c, m) in [
                ('W', state::WATER),
                ('B', state::BLOCKED),
                ('M', state::MOVEMENT_BLOCKED),
                ('D', state::DEPOSIT_BLOCKED),
                ('O', state::DOODAD_BLOCKED),
                ('E', state::ELEVATION_BLOCKED),
            ] {
                if t.has(f, m) {
                    out.push(c);
                }
            }
            if out.is_empty() {
                out.push('.');
            }
            out
        };
        eprintln!(
            "  gate field {} neighbours {:?}",
            bits(gate),
            t.neighbours(gate)
                .into_iter()
                .flatten()
                .map(bits)
                .collect::<Vec<_>>()
        );
    }
}

/// How a mission is played (`s2_ai::campaign_player`).
#[derive(Clone, Copy)]
struct Setup {
    /// A computer player of this profile runs the human's economy and fighting.
    help: Option<u8>,
    attacks: s2_ai::campaign_player::Attacks,
    flags_on_roads: bool,
    restless: bool,
}

/// One mission played for up to `until` ticks with every soak invariant after
/// every tick: the tick of the victory, if it came. What it took is printed.
fn play_mission(n: u32, until: u64, setup: Setup) -> Option<u64> {
    use s2_ai::campaign_player::CampaignPlayer;
    let (dir, rules) = install()?;
    let name = format!("campaign{n:02}");
    let mut game = match setup.help {
        Some(profile) => {
            MissionGame::start_with_computer_help(&dir, &rules, &name, profile).unwrap()
        }
        None => MissionGame::start(&dir, &rules, &name).unwrap(),
    };
    let goals: Vec<(u32, u32)> = game
        .world
        .mission()
        .unwrap()
        .triggers
        .iter()
        .filter(|t| t.kind == s2_sim::mission::TriggerKind::Victory)
        .map(|t| t.field)
        .collect();
    let mut player = match setup.help {
        Some(_) => CampaignPlayer::helped(&game.world, &rules, HUMAN, goals.clone()).unwrap(),
        None => CampaignPlayer::new(&game.world, &rules, HUMAN, goals.clone()).unwrap(),
    };
    player.attacks = setup.attacks;
    player.flags_on_roads = setup.flags_on_roads;
    player.restless = setup.restless;
    let mut checker = Checker::new(&game.world);
    let mut victory = None;
    while game.world.tick() < until && victory.is_none() {
        player.act(&mut game.world);
        game.step();
        let broken = checker.check(&game.world);
        assert!(broken.is_empty(), "{name}: {broken:?}");
        if game.world.mission().unwrap().completed.is_some() {
            victory = Some(game.world.tick());
        }
        if game.world.outcome(HUMAN).is_some() {
            break;
        }
    }
    assert_eq!(game.problems, Vec::<String>::new());
    let world = &game.world;
    let building_name = |id: u32| {
        rules
            .buildings
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.name.clone())
            .unwrap_or_default()
    };
    let placed: Vec<(String, u32)> = player
        .log
        .placed
        .iter()
        .map(|(id, n)| (building_name(*id), *n))
        .collect();
    let land: Vec<_> = world
        .terrain()
        .fields()
        .filter(|&f| world.field_owner(f) == Some(HUMAN))
        .collect();
    let nearest = land
        .iter()
        .map(|&f| s2_sim::grid::distance(f, goals[0]))
        .min();
    let in_stock = |good: &str| -> u32 {
        let id = rules.id_named(good).unwrap();
        world
            .warehouses()
            .filter(|(at, _)| world.building_at(*at).is_some_and(|b| b.owner == HUMAN))
            .flat_map(|(_, s)| s.iter())
            .filter(|(g, _)| *g == id)
            .map(|(_, n)| *n)
            .sum()
    };
    let others: Vec<String> = (1..8u8)
        .map(PlayerId)
        .filter(|&p| world.is_player(p))
        .map(|p| {
            let buildings = world.buildings().filter(|b| b.owner == p).count();
            let lost = world.outcome(p).is_some();
            format!(
                "{}: {buildings} buildings{}",
                p.0,
                if lost { ", lost" } else { "" }
            )
        })
        .collect();
    eprintln!(
        "RESULT {name}: {} at tick {}, human {:?}; placed {placed:?}, {} roads, {} flags, {} given up, \
         {} attacks, {} orders refused; land {} fields, {nearest:?} from the gate; planks {}, stones {}; \
         others {others:?}; waiting for {:?}",
        victory.map_or("NOT WON", |_| "WON"),
        world.tick(),
        world.outcome(HUMAN).is_some().then_some("lost"),
        player.log.roads,
        player.log.flags,
        player.log.given_up,
        player.log.attacks.len(),
        player.log.refused.values().sum::<u32>(),
        land.len(),
        in_stock("plank"),
        in_stock("stone"),
        player.waiting_for(world),
    );
    victory
}

/// A player's own: a wood and stone economy, the soldiers of the start stock
/// sent home where no enemy is near, attacks.
const ALONE: Setup = Setup {
    help: None,
    attacks: s2_ai::campaign_player::Attacks::Always,
    flags_on_roads: true,
    restless: true,
};
/// Beside a Hard computer player, which brings food, mines and new soldiers.
const HELPED: Setup = Setup {
    help: Some(3),
    attacks: s2_ai::campaign_player::Attacks::Weakest,
    flags_on_roads: true,
    restless: true,
};

macro_rules! mission_won {
    ($($name:ident: $n:literal, $until:literal, $setup:expr;)*) => {$(
        // spec: CAM-2.1, CAM-2.2, CAM-3.1, CAM-4.3, MIL-ATK-1.2, MIL-ATK-1.5, MIL-SOL-3.2, MIL-SOL-4.6
        #[test]
        #[ignore = "plays a whole mission: cargo test --release -p s2-ai --test campaign is_won -- --ignored --nocapture"]
        fn $name() {
            if std::env::var_os("S2_GAME_DIR").is_some() {
                let tick = play_mission($n, $until, $setup);
                assert!(tick.is_some(), "mission {} is not won", $n);
            }
        }
    )*};
}

// Mission 1 is `mission_one_is_won_by_reaching_the_gate`. With each mission's players
// as its map sets them (CAM-4.3a), missions 4 to 8 and 10 were not won by the four ways
// tried (`one_run`; `docs/test-coverage.md`).
mission_won! {
    mission_2_is_won: 2, 100_000, Setup {
        attacks: s2_ai::campaign_player::Attacks::Outnumbering,
        ..ALONE
    };
    mission_3_is_won: 3, 150_000, HELPED;
    mission_9_is_won: 9, 200_000, HELPED;
}

/// One run chosen from outside, to find what a mission takes:
/// `S2_RUN=mission,profile,attacks[,until[,noflags]]` (profile 0: alone; attacks
/// n never, o outnumbering, a always, w weakest).
// spec: CAM-2.1, CAM-3.1
#[test]
#[ignore = "S2_RUN=5,3,w,300000 cargo test --release -p s2-ai --test campaign one_run -- --ignored --nocapture"]
fn one_run() {
    use s2_ai::campaign_player::Attacks;
    let Ok(run) = std::env::var("S2_RUN") else {
        return;
    };
    let parts: Vec<&str> = run.split(',').collect();
    let help = parts[1].parse::<u8>().unwrap();
    let setup = Setup {
        help: (help > 0).then_some(help),
        attacks: match parts[2] {
            "n" => Attacks::Never,
            "a" => Attacks::Always,
            "w" => Attacks::Weakest,
            _ => Attacks::Outnumbering,
        },
        flags_on_roads: parts.get(4) != Some(&"noflags"),
        restless: parts.get(4) != Some(&"noflags"),
    };
    let until = parts.get(3).map_or(250_000, |t| t.parse().unwrap());
    play_mission(parts[0].parse().unwrap(), until, setup);
}

/// A save of mission 1 on the way, for the frame-time and sound measurements
/// (F55, F56): in `target/tmp`, never the player's folder, and not in git.
// spec: FMT-SAV-3.12
#[test]
#[ignore = "writes target/tmp/mission-1-midway.s2save"]
fn writes_a_save_of_mission_one_on_the_way() {
    use s2_ai::campaign_player::CampaignPlayer;
    let Some((dir, rules)) = install() else {
        return;
    };
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    let mut player = CampaignPlayer::new(&game.world, &rules, HUMAN, vec![GATE]).unwrap();
    while game.world.tick() < 20_000 {
        player.act(&mut game.world);
        game.step();
    }
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mission-1-midway.s2save");
    std::fs::write(&file, game.save()).unwrap();
    eprintln!(
        "{}: tick {}, {} buildings, {} settlers, placed {:?}",
        file.display(),
        game.world.tick(),
        game.world.buildings().count(),
        game.world.settler_views().len(),
        player.log.placed,
    );
}
