//! The campaign as the interface shows it, on the shipped missions played
//! headless (`s2_ai::mission`): the victory screen, the next mission unlocked
//! and the progress in the profile file; the defeat screen; the briefing and
//! the goals of every mission, the window of every lesson. Skipped (passes)
//! when `S2_GAME_DIR` is not set. The profile file goes to a folder of the
//! test's own, never the player's.

use s2_ai::mission::{MissionGame, TerritoryPlayer};
use s2_assets::lang::Texts;
use s2_rules::RuleSet;
use s2_sim::{Command, PlayerId};

use super::campaign::*;
use super::menu::Screen;
use super::settings::Settings;
use crate::ui::game::{self, Catalog, Ctx};
use crate::ui::widgets::{Action, MenuAction, Panel, Widget};

const HUMAN: PlayerId = PlayerId(0);
const SCREEN: [f32; 2] = [1024.0, 768.0];

/// The install: its folder, the rules and the language files.
fn install() -> Option<(s2_assets::GameDir, RuleSet, Texts)> {
    let Some(root) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return None;
    };
    let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
    let (rules, _) = s2_rules::load(&dir).expect("the property scripts");
    let mut texts = Texts::default();
    for file in crate::ui::TEXT_FILES {
        let rel = format!("data/language/{}/{file}", crate::ui::LANGUAGE);
        let path = s2_rules::resolve(dir.root(), &rel).expect("the language file");
        let data = crate::game::read_game_file(&path).expect("readable");
        texts.extend(Texts::parse(&s2_assets::lang::decode(&data)));
    }
    Some((dir, rules, texts))
}

/// CAM-1.3c (Q-1055 answered): the book's title is the map's name looked
/// up in the interface texts, in capitals: `campaign01` gives MISSION 1 (as
/// the original's picture), `MP_2P_Storm_Coast` Coastal Storm; a lesson
/// without a key shows its name. With the install's texts where it is set.
// spec: CAM-1.3c
#[test]
fn the_book_title_is_the_maps_text() {
    let texts = Texts::parse("campaign01\tMission 1\n");
    assert_eq!(book_title(&texts, "campaign01"), "Mission 1");
    assert_eq!(book_title(&texts, "Campaign01"), "Mission 1");
    assert_eq!(book_title(&texts, "tutorial02"), "tutorial02");
    let book = Book {
        title: "campaign01".into(),
        pages: vec!["a".into()],
        ..Book::default()
    };
    let p = book_panel(&texts, SCREEN, &book);
    assert!(shows(&p, "MISSION 1"), "{:?}", shown(&p));
    if let Some((_, _, texts)) = install() {
        assert_eq!(book_title(&texts, "campaign01"), "Mission 1");
        assert_eq!(book_title(&texts, "campaign10"), "Mission 10");
        assert_eq!(book_title(&texts, "MP_2P_Storm_Coast"), "Coastal Storm");
    }
}

/// UI-GAME-4.4 (Q-1053 answered): the headquarters' military page's row
/// names are the soldier settlers' names, entries of the install's texts
/// (the interface reads `info.txt` for them). Skipped without `S2_GAME_DIR`.
// spec: UI-GAME-4.4
#[test]
fn the_rank_names_are_in_the_install_texts() {
    let Some((_, _, texts)) = install() else {
        return;
    };
    for key in crate::ui::game::windows::RANK_NAMES {
        let name = texts.entry(key).unwrap_or_default();
        assert!(
            !name.is_empty() && !name.starts_with("dlg_"),
            "{key}: {name:?}"
        );
    }
}

/// A folder of the test's own, removed when it is dropped.
struct Folder(std::path::PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("s2-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// Every text a panel shows.
fn shown(panel: &Panel) -> Vec<String> {
    panel
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. }
            | Widget::Paragraph { text, .. }
            | Widget::BarButton { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn shows(panel: &Panel, text: &str) -> bool {
    shown(panel).iter().any(|t| t == text)
}

/// Whether the panel's button of `action` is on and enabled.
fn enabled(panel: &Panel, action: &MenuAction) -> Option<bool> {
    panel.widgets.iter().find_map(|w| match w {
        Widget::BarButton {
            action: Action::Menu(m),
            enabled,
            ..
        }
        | Widget::Button {
            action: Action::Menu(m),
            enabled,
            ..
        } if m == action => Some(*enabled),
        _ => None,
    })
}

/// The interface of `game` as `load_map` sets it up.
fn interface(game: &mut MissionGame, name: &str) -> MissionUi {
    let mut ui = MissionUi::new(name, s2_rules::mission::map_kind(&game.map_file), 0);
    ui.target = game.saved.target;
    ui.apply(std::mem::take(&mut game.start_events));
    ui
}

/// One tick as `advance_world` runs it.
fn tick(game: &mut MissionGame, ui: &mut MissionUi) {
    let events = game.step();
    ui.apply(events);
    ui.results(game.world.mission_events(), HUMAN);
    assert_eq!(game.problems, Vec::<String>::new());
}

// CAM T3: mission 1 won as a player wins it; what the program then shows and
// keeps. spec: the profile's map state is where the original keeps it (SET-2.2).
// spec: CAM-3.5, CAM-3.6, CAM-4.2, SET-2.2 (T3, T9, SET T4)
#[test]
fn mission_one_victory_opens_mission_two() {
    let Some((dir, rules, texts)) = install() else {
        return;
    };
    let folder = Folder::new("campaign-profile");
    let users = folder.0.join("users");
    let mut settings = Settings::load_from(None, Some(users.clone()), None);
    assert!(!unlocked(&settings, 2), "a new profile has only mission 1");
    assert_eq!(mission_markers(&settings).len(), 1);

    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    let mut ui = interface(&mut game, "campaign01");
    let barracks = rules.building_named("Barracks").unwrap().id;
    let mut player = TerritoryPlayer::new(&game.world, &rules, HUMAN, (64, 70), barracks).unwrap();
    let mut books = 0;
    while ui.end.is_none() && game.world.tick() < 40_000 {
        player.act(&mut game.world);
        tick(&mut game, &mut ui);
        // The player reads each book and closes it (CAM-1.3).
        if ui.book.as_ref().is_some_and(|b| b.end.is_none()) {
            books += 1;
            assert_eq!(ui.close_book(), None, "not the debriefing");
        }
    }
    assert_eq!(ui.end, Some(EndScreen::Victory(4)));
    assert!(books >= 2, "the briefing and the second book: {books}");

    // CAM-3.6b: the victory book: VICTORY, the target's text, the
    // debriefing pages, the won picture, CONTINUE and EXIT MAP.
    let book = ui.book.clone().expect("the victory book");
    assert_eq!(book.end, Some(EndScreen::Victory(4)));
    assert_eq!(book.pages, ["b01_C_00", "b01_C_01"]);
    assert_eq!(book.picture.as_deref(), Some("ui_briefing_screen_won"));
    let page = book_panel(&texts, SCREEN, &book);
    assert!(
        shows(&page, &texts.get("!VICTORY").to_uppercase()),
        "{:?}",
        shown(&page)
    );
    assert!(shows(&page, texts.get("!You have reached the magic gate!")));
    assert!(shows(&page, texts.entry("b01_C_00").unwrap()));
    assert_eq!(enabled(&page, &MenuAction::EndContinue), Some(true));
    assert_eq!(enabled(&page, &MenuAction::EndExit), Some(true));

    // CAM-3.6: the end screen says so, and only EXIT MAP leads on.
    let end = end_panel(
        &texts,
        SCREEN,
        EndScreen::Victory(4),
        &game.world,
        "campaign01",
        "default",
    );
    assert!(shows(&end, texts.get("!VICTORY")), "{:?}", shown(&end));
    assert!(shows(&end, texts.get("!You have reached the magic gate!")));
    assert_eq!(enabled(&end, &MenuAction::EndExit), Some(true));
    assert_eq!(enabled(&end, &MenuAction::EndContinue), Some(false));

    // CAM-3.5: recorded and acknowledged in the profile, the settings' map
    // state (SET-2.2); the second time there is nothing new.
    assert!(ui.record_completed(&mut settings));
    assert!(!ui.record_completed(&mut settings));
    settings.write_files(None, Some(users.clone()), true, None);

    // A new start of the program reads it there.
    let again = Settings::load_from(None, Some(users.clone()), None);
    assert!(again.completed("campaign01"));
    let file = users.join(format!("{}.cfg", again.profile));
    let cfg = s2_assets::cfg::parse(&std::fs::read(&file).unwrap()).unwrap();
    let state = cfg.user.map("campaign01").expect("the map's state");
    assert!(state.completed);
    assert_eq!(state.acknowledged, 1);

    // CAM-4.2: mission 2 opens; mission 3 does not.
    assert!(unlocked(&again, 2));
    assert!(!unlocked(&again, 3));
    let markers = mission_markers(&again);
    assert_eq!(markers.len(), 2);
    assert!(markers[0].2, "mission 1's eagle is gold");
    assert!(!markers[1].2, "mission 2's is silver");

    // CAM-3.6b: EXIT MAP in the victory book leads to the campaign menu (its
    // debriefing was the book's pages).
    assert_eq!(ui.exit_map(), Some(Screen::Campaign));
    assert_eq!(ui.end, None);
    assert_eq!(ui.book, None);
}

/// CAM-1.3c (T18; E-cam-012, E-trace-028): the book dialog on 1024 x 768:
/// the frame `uiBriefing` (0, 0, 728 x 568) 1:1 at the centred corner (148,
/// 100); the arrows at (210, 430) and (300, 430), the previous one off on
/// the first page; the label `1/2`; CONTINUE at (190, 465). CAM-3.6: a
/// campaign defeat is the book without CONTINUE, arrows or label, with EXIT
/// MAP at (190, 487).
// spec: CAM-1.3c (T18), CAM-3.6, CAM-3.6b
#[test]
fn the_book_dialog_at_its_places() {
    let texts = Texts::parse("!CONTINUE\tCONTINUE\n!EXIT MAP\tExit map\n");
    let book = Book {
        pages: vec!["a".into(), "b".into()],
        title: "Mission 1".into(),
        ..Book::default()
    };
    let p = book_panel(&texts, SCREEN, &book);
    assert_eq!(p.at, [148.0, 100.0]);
    let frame = p.widgets.iter().any(|w| {
        matches!(w, Widget::Sprite { atlas: crate::ui::widgets::Atlas::Book, rect: [0, 0, 728, 568], at, .. } if *at == [0.0, 0.0])
    });
    assert!(frame, "the frame");
    let at = |action: &MenuAction| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Button {
                action: Action::Menu(a),
                at,
                enabled,
                ..
            }
            | Widget::BarButton {
                action: Action::Menu(a),
                at,
                enabled,
                ..
            } if a == action => Some((*at, *enabled)),
            _ => None,
        })
    };
    assert_eq!(at(&MenuAction::BookPage(-1)), Some(([210.0, 430.0], false)));
    assert_eq!(at(&MenuAction::BookPage(1)), Some(([300.0, 430.0], true)));
    assert_eq!(at(&MenuAction::CloseBook), Some(([190.0, 465.0], true)));
    assert!(shows(&p, "1/2"));
    assert!(shows(&p, "MISSION 1"));
    let defeat = Book {
        end: Some(EndScreen::Defeat),
        ..Book::default()
    };
    let p = book_panel(&texts, SCREEN, &defeat);
    let has = |action: MenuAction| format!("{:?}", p.widgets).contains(&format!("{action:?}"));
    assert!(!has(MenuAction::CloseBook) && !has(MenuAction::EndContinue));
    assert!(!has(MenuAction::BookPage(1)) && !has(MenuAction::BookPage(-1)));
    assert!(has(MenuAction::EndExit));
}

// CAM T6: the headquarters gone on a prepared world: the defeat screen,
// nothing recorded, mission 2 stays locked and the main menu follows.
// spec: CAM-3.2, CAM-3.6, CAM-4.2 (T6)
#[test]
fn mission_one_defeat_records_nothing() {
    let Some((dir, rules, texts)) = install() else {
        return;
    };
    let mut settings = Settings::default();
    let mut game = MissionGame::start(&dir, &rules, "campaign01").unwrap();
    let mut ui = interface(&mut game, "campaign01");
    tick(&mut game, &mut ui);
    let hq = game.world.buildings().next().unwrap().at;
    game.world.submit(Command::RemoveBuilding {
        player: HUMAN,
        at: hq,
    });
    tick(&mut game, &mut ui);
    assert_eq!(ui.end, Some(EndScreen::Defeat));
    let end = end_panel(
        &texts,
        SCREEN,
        EndScreen::Defeat,
        &game.world,
        "campaign01",
        "default",
    );
    // CAM-3.6a: DEFEAT, its reason (the headquarters is gone), and no
    // CONTINUE on a defeat.
    assert!(shows(&end, texts.get("!DEFEAT")), "{:?}", shown(&end));
    assert!(shows(&end, texts.get("!Castle destroyed!")));
    assert!(shown(&end).iter().any(|t| t.contains(texts.get("!DEFEAT"))));
    assert_eq!(enabled(&end, &MenuAction::EndExit), Some(true));
    assert_eq!(enabled(&end, &MenuAction::EndContinue), Some(false));

    assert!(!ui.record_completed(&mut settings));
    assert!(!settings.completed("campaign01"));
    assert!(!unlocked(&settings, 2));
    assert_eq!(ui.exit_map(), Some(Screen::Main));
    assert_eq!(ui.book, None);
}

/// The goal panel of UI-GAME-2.4 for a world.
fn goal_panel(world: &s2_sim::World, rules: &RuleSet, texts: &Texts) -> Panel {
    let tools = s2_rules::TOOL_SLOTS
        .iter()
        .filter_map(|n| rules.id_named(n))
        .collect();
    let catalog = Catalog::new(rules, tools);
    let ctx = Ctx {
        world,
        player: HUMAN,
        screen: SCREEN,
        texts,
        catalog: &catalog,
        click: None,
        tutorial: false,
        speed: 1,
        real_seconds: 0.0,
        posted_messages: &[],
        ship_names: &std::collections::BTreeMap::new(),
        focus: None,
        statistics_span: 0,
        statistics_hidden: [false; 8],
        production_marks: [false; 14],
        production_span: 0,
        production_read: None,
        building_tally: None,
        stats_list: None,
        player_name: "default",
        hovered: None,
    };
    game::briefing(&ctx)
}

// The island map's eagle starts the mission; 25 s in, its briefing is on the
// screen with its pictures and texts from the language file; the mission panel
// says the goal.
// spec: CAM-1.3, CAM-1.3a, CAM-3.1, CAM-3.1a, CAM-4.2, CAM-4.4, UI-GAME-2.4, UI-MENU-1.7 (T1)
#[test]
fn every_mission_starts_with_its_briefing_and_goal() {
    let Some((dir, rules, texts)) = install() else {
        return;
    };
    let mut settings = Settings::default();
    for n in 1..=MISSIONS {
        // UI-MENU-1.7: its marker on the island map once the one before is won.
        let markers = mission_markers(&settings);
        assert_eq!(markers.len(), usize::from(n), "mission {n}");
        let name = mission_name(n);
        let mut game = MissionGame::start(&dir, &rules, &name).unwrap();
        let mut ui = interface(&mut game, &name);
        assert_eq!(ui.kind, 1);
        assert!(ui.book.is_none(), "{name}: no book before 25 s");
        while game.world.tick() < 260 {
            tick(&mut game, &mut ui);
            if ui.book.is_some() {
                break;
            }
        }
        assert_eq!(game.world.tick(), 251, "{name}: T1 the 26th second");
        let book = ui.book.clone().expect("the briefing");
        let key = format!("b{n:02}_A_00");
        assert_eq!(book.pages, [key.clone(), format!("b{n:02}_A_01")]);
        let picture = format!("ui_briefing_screen_{n}");
        assert_eq!(book.picture.as_deref(), Some(picture.as_str()), "{name}");
        assert!(
            crate::ui::widgets::picture(&picture).is_some(),
            "{name}: its picture has a place in the atlases"
        );
        let panel = panels(&ui, &texts, SCREEN, &game.world, "default");
        assert_eq!(panel.len(), 1, "{name}");
        let page = texts.entry(&key).expect("the briefing's text");
        assert!(shows(&panel[0], page), "{name}: {:?}", shown(&panel[0]));
        // The next page, then CONTINUE closes it and the game goes on.
        assert_eq!(enabled(&panel[0], &MenuAction::BookPage(1)), Some(true));
        assert_eq!(ui.close_book(), None);
        // UI-GAME-2.4 (E-trace-028): the panel's target line, no
        // `(VICTORY)` before the portal; CAM-3.1a: every mission plays with
        // target 4, missions 4 to 10 too.
        let goal = goal_panel(&game.world, &rules, &texts);
        let line = "!MissionTarget: Reach the portal";
        assert!(shows(&goal, texts.get(line)), "{name}: {:?}", shown(&goal));
        let won = format!("({})", texts.get("!VICTORY"));
        assert!(!shows(&goal, &won), "{name}: {:?}", shown(&goal));
        settings.mark_completed(&name);
    }
}

// The tutorial page's button starts the lesson; its window says the first step
// in words from the language file, and its pointer waits on the map.
// spec: CAM-5.1, CAM-5.2, CAM-5.3, UI-MENU-1.6 (T8, T11)
#[test]
fn every_lesson_starts_with_its_window() {
    let Some((dir, rules, texts)) = install() else {
        return;
    };
    let page = tutorial_menu(&texts, SCREEN, false);
    for k in 1..=4u8 {
        assert_eq!(enabled(&page, &MenuAction::StartLesson(k)), Some(true));
        let name = format!("tutorial{k:02}");
        let mut game = MissionGame::start(&dir, &rules, &name).unwrap();
        let mut ui = interface(&mut game, &name);
        assert_eq!(ui.kind, 2);
        let tutorial = ui.tutorial.clone().expect("the guidance");
        assert_eq!(tutorial.step, lesson_start(k));
        for _ in 0..300 {
            tick(&mut game, &mut ui);
        }
        assert!(ui.book.is_none(), "{name}: a lesson has no book");
        let panel = panels(&ui, &texts, SCREEN, &game.world, "default");
        assert_eq!(panel.len(), 1, "{name}");
        let key = step_key(tutorial.step);
        let text = texts.entry(key).expect("the step's text");
        assert!(shows(&panel[0], text), "{name}: {:?}", shown(&panel[0]));
        assert_eq!(enabled(&panel[0], &MenuAction::TutorialNext), Some(true));
    }
}

// The buttons of the tutorial's last step lie inside its window (seen in the
// harness: EXIT MAP stood out over its right edge).
// spec: CAM-5.3
#[test]
fn the_last_steps_buttons_lie_inside_the_window() {
    let mut t = Tutorial::new(1);
    t.step = 0x34;
    let panel = tutorial_panel(&Texts::default(), SCREEN, &t);
    let window = panel.widgets.iter().find_map(|w| match w {
        Widget::Sprite { size: Some(s), .. } => Some(*s),
        Widget::Sprite {
            size: None, rect, ..
        } => Some([rect[2] as f32, rect[3] as f32]),
        _ => None,
    });
    let [width, _] = window.expect("the window");
    let bars: Vec<f32> = panel
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton { at, bar, .. } => Some(at[0] + bar.size[0]),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 2);
    assert!(bars.iter().all(|&right| right <= width), "{bars:?} {width}");
}

// The end screen's buttons lie inside its dialog and do not overlap (seen in
// the harness: three bars side by side stood out over its right edge).
// CAM-3.6a: the book's 728 x 568 frame; Statistics, EXIT MAP, CONTINUE one
// under the other at x 300.
// spec: CAM-3.6, CAM-3.6a
#[test]
fn the_end_screens_buttons_lie_inside_the_dialog() {
    let world = s2_sim::World::new(0);
    let panel = end_panel(
        &Texts::default(),
        SCREEN,
        EndScreen::Defeat,
        &world,
        "MP_2P_Storm_Coast",
        "default",
    );
    let bars: Vec<[f32; 4]> = panel
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton { at, bar, .. } => Some([at[0], at[1], bar.size[0], bar.size[1]]),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 3);
    let mut places: Vec<[f32; 2]> = bars.iter().map(|b| [b[0], b[1]]).collect();
    places.sort_by(|a, b| a[1].total_cmp(&b[1]));
    assert_eq!(places, [[300.0, 445.0], [300.0, 467.0], [300.0, 489.0]]);
    for (i, b) in bars.iter().enumerate() {
        assert!(b[0] >= 0.0 && b[0] + b[2] <= 728.0, "{b:?}");
        assert!(b[1] >= 0.0 && b[1] + b[3] <= 568.0, "{b:?}");
        for c in &bars[i + 1..] {
            assert!(b[1] + b[3] <= c[1] || c[1] + c[3] <= b[1], "{b:?} {c:?}");
        }
    }
}

/// CAM-5.3, UI-GAME-4.4: lesson 4's page step asks for the harbour's expedition
/// tab (12), not any tab: its next step's button is on that page.
// spec: CAM-5.3, UI-GAME-4.4
#[test]
fn the_expedition_lesson_asks_for_the_expedition_tab() {
    assert_eq!(step_key(0x05), "T04_StartExpedition_SelectPage");
    assert_eq!(expects(0x05, &Action::BuildingTab(12)), Some(true));
    assert_eq!(expects(0x05, &Action::BuildingTab(9)), Some(false));
}

/// CAM-5.3c (Q-100 b in part, E-cam-011): the build menu's SelectPage steps
/// (0x15, 0x25, 0x48, 0x4b) take only the medium buildings tab, and a wrong
/// click shows `T01_ERROR_SelectStreetIcon`; other steps' wrong clicks show
/// `T01_ERROR_SelectMarkedButton`.
// spec: CAM-5.3c (T19)
#[test]
fn a_page_step_takes_only_the_medium_tab() {
    use crate::menus::campaign::{expects, wrong_click_error};
    use crate::ui::widgets::Action;
    for step in [0x15, 0x25, 0x48, 0x4b] {
        assert_eq!(expects(step, &Action::BuildTab(6)), Some(true), "{step:x}");
        assert_eq!(expects(step, &Action::BuildTab(5)), Some(false), "{step:x}");
        assert_eq!(expects(step, &Action::BuildTab(7)), Some(false), "{step:x}");
        assert_eq!(wrong_click_error(step), "T01_ERROR_SelectStreetIcon");
    }
    assert_eq!(wrong_click_error(0x10), "T01_ERROR_SelectMarkedButton");
    // T19b (E-cam-014): the harbour's page choice (5) is a page step too.
    assert_eq!(wrong_click_error(0x05), "T01_ERROR_SelectStreetIcon");
}

/// CAM-5.2a (E-trace-028): an explaining step's window is the game menu's
/// frame at the top near the middle with Cancel (190, 376) and Continue
/// (190, 401); a step that waits for a click is the production window's
/// frame at the top right, 5 px from the edge, with Cancel (289, 167), and a
/// wrong click's error stands in red under the text.
// spec: CAM-5.2a, CAM-5.3
#[test]
fn the_tutorial_windows_two_forms() {
    let texts = Texts::default();
    let bar = |p: &Panel, action: MenuAction| {
        p.widgets.iter().find_map(|w| match w {
            Widget::BarButton { action: a, at, .. } if *a == Action::Menu(action.clone()) => {
                Some(*at)
            }
            _ => None,
        })
    };
    let mut t = Tutorial::new(1);
    assert!(t.explains(), "the introduction explains");
    let p = tutorial_panel(&texts, SCREEN, &t);
    assert_eq!(p.at, [332.0, 22.0]);
    assert_eq!(bar(&p, MenuAction::TutorialCancel), Some([190.0, 376.0]));
    assert_eq!(bar(&p, MenuAction::TutorialNext), Some([190.0, 401.0]));
    // A step that waits for the construction view's button.
    t.step = 0x1e;
    assert!(!t.explains());
    t.error = Some("T01_ERROR_SelectMarkedButton");
    let p = tutorial_panel(&texts, SCREEN, &t);
    assert_eq!(p.at, [1024.0 - 485.0, 22.0]);
    assert_eq!(bar(&p, MenuAction::TutorialCancel), Some([289.0, 167.0]));
    assert_eq!(bar(&p, MenuAction::TutorialNext), None);
    let red = p.widgets.iter().any(|w| {
        matches!(w, Widget::Paragraph { colour: [255, 0, 0], text, .. } if text.contains("SelectMarkedButton"))
    });
    assert!(red, "the error in red");
}

/// CAM-5.2a, 5.3c: the arrow's tip goes to the top-left corner of the widget
/// the step expects: at a SelectBuilding step the build menu's cell (0 the
/// flag button), at 0x1e the construction view's button of the main bar.
// spec: CAM-5.2a, CAM-5.3c
#[test]
fn the_tutorial_arrow_points_at_the_expected_widget() {
    use crate::ui::widgets::{Atlas, Hover, PanelId};
    use s2_sim::Command as C;
    let button = |at: [f32; 2], action: Action| Widget::Button {
        atlas: Atlas::Elements3,
        rect: [0, 0, 80, 80],
        hover: Hover::None,
        pressed: Hover::None,
        at,
        size: None,
        tooltip: None,
        action,
        enabled: true,
        selected: false,
    };
    let player = PlayerId(0);
    let mut menu = Panel::new(PanelId::Context, [100.0, 50.0]);
    menu.widgets.push(button(
        [23.0, 60.0],
        Action::Command(C::PlaceFlag { player, at: (5, 5) }),
    ));
    for (i, building) in [7u32, 8, 9].into_iter().enumerate() {
        let at = [23.0 + 80.0 * (i + 1) as f32, 60.0];
        menu.widgets.push(button(
            at,
            Action::Command(C::PlaceBuilding {
                player,
                building,
                at: (5, 5),
            }),
        ));
    }
    let mut hud = Panel::new(PanelId::Hud, [300.0, 700.0]);
    hud.widgets.push(button([10.0, 4.0], Action::Hud(2)));
    hud.widgets.push(button([60.0, 4.0], Action::Hud(3)));
    let panels = vec![hud, menu];
    // 0x20 (the woodcutter's hut, cell 1), 0x37 (the flag button, cell 0).
    assert_eq!(expected_widget_corner(&panels, 0x20), Some([203.0, 110.0]));
    assert_eq!(expected_widget_corner(&panels, 0x37), Some([123.0, 110.0]));
    // 0x1e: the construction view's button.
    assert_eq!(expected_widget_corner(&panels, 0x1e), Some([360.0, 704.0]));
}

/// Two players on a flat test map, teams 0 and 1; the second has lost.
fn two_player_world() -> s2_sim::World {
    let n = 30 * 30;
    let terrain = s2_sim::map::Terrain {
        width: 30,
        height: 30,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut castle = s2_sim::BuildingType::new(9, s2_sim::BuildingSize::Large);
    castle.territory_range = 4;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![castle],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![(PlayerId(0), (8, 8)), (PlayerId(1), (22, 22))],
        local_player: PlayerId(0),
        teams: vec![(PlayerId(0), 0), (PlayerId(1), 1)],
        ..Default::default()
    };
    s2_sim::World::new_game(1, terrain, rules, &setup)
}

// CAM-3.6a (T20a, E-cam-015): the end screen's rows from (140, 190), 25 px
// apart: "Player #n:" at x 140 in its player's colour (E-trace-032), the
// human's name at 230, Team n at 400, the
// player's own result at 500; the result VICTORY or DEFEAT on (374, 110);
// the title the map's name's text centred on (374, 74).
// spec: CAM-3.6a (T20a), CAM-1.3c
#[test]
fn the_end_screens_rows_stand_in_four_columns() {
    let world = two_player_world();
    let texts = Texts::parse("!Team 1\tTeam 1\n!Team 2\tTeam 2\n!VICTORY\tVICTORY\n");
    let panel = end_panel(
        &texts,
        SCREEN,
        EndScreen::Victory(1),
        &world,
        "MP_2P_Storm_Coast",
        "Ada",
    );
    let at = |text: &str| -> Vec<[f32; 2]> {
        panel
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Text { text: t, at, .. } if t == text => Some(*at),
                _ => None,
            })
            .collect()
    };
    assert_eq!(at("Player #1:"), [[140.0, 190.0]]);
    assert_eq!(at("Player #2:"), [[140.0, 215.0]]);
    // E-trace-032: each label in its player's colour.
    let colour = |text: &str| {
        panel.widgets.iter().find_map(|w| match w {
            Widget::Text {
                text: t, colour, ..
            } if t == text => Some(*colour),
            _ => None,
        })
    };
    assert_eq!(
        colour("Player #1:"),
        Some(label_colour(s2_sim::PlayerId(0)))
    );
    assert_ne!(colour("Player #1:"), colour("Player #2:"));
    assert_eq!(at("Ada"), [[230.0, 190.0]]);
    assert_eq!(at("Team 1"), [[400.0, 190.0]]);
    assert_eq!(at("Team 2"), [[400.0, 215.0]]);
    // Nobody has a result yet: no column at 500; the screen's own result.
    assert!(shown(&panel).iter().any(|t| t == "VICTORY"));
    assert!(shown(&panel).iter().any(|t| t == "MP_2P_STORM_COAST"));
}

// CAM-3.6a (T20b, Q-1057): the human in slot 1, computer players in slots
// 2 and 4, slot 3 closed: three rows closed up at y 190, 215 and 240,
// numbered #1, #2 and #4; the computer players named with `!AI`.
// spec: CAM-3.6a (T20b)
#[test]
fn the_end_screen_names_computer_players_ai() {
    let n = 40 * 40;
    let terrain = s2_sim::map::Terrain {
        width: 40,
        height: 40,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut castle = s2_sim::BuildingType::new(9, s2_sim::BuildingSize::Large);
    castle.territory_range = 4;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![castle],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![
            (PlayerId(0), (8, 8)),
            (PlayerId(1), (30, 8)),
            (PlayerId(3), (30, 30)),
        ],
        local_player: PlayerId(0),
        ..Default::default()
    };
    let world = s2_sim::World::new_game(1, terrain, rules, &setup);
    let texts = Texts::parse("!AI\tKI\n");
    let panel = end_panel(
        &texts,
        SCREEN,
        EndScreen::Victory(1),
        &world,
        "MP_4P_Keyhole",
        "Ada",
    );
    let at = |text: &str| -> Vec<[f32; 2]> {
        panel
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Text { text: t, at, .. } if t == text => Some(*at),
                _ => None,
            })
            .collect()
    };
    assert_eq!(at("Player #1:"), [[140.0, 190.0]]);
    assert_eq!(at("Player #2:"), [[140.0, 215.0]]);
    assert_eq!(at("Player #3:"), Vec::<[f32; 2]>::new());
    assert_eq!(at("Player #4:"), [[140.0, 240.0]]);
    assert_eq!(at("Ada"), [[230.0, 190.0]]);
    assert_eq!(at("KI"), [[230.0, 215.0], [230.0, 240.0]]);
}

// CAM-3.6c (T20a, E-cam-015): the defeat screen on the book: the book's
// title, DEFEAT, the lost text in the text box at (150, 120), the lost
// picture, only EXIT MAP at CONTINUE's place (190, 465); no page arrows,
// page label or CONTINUE.
// spec: CAM-3.6c (T20a), CAM-1.3c
#[test]
fn t20a_the_defeat_screen_is_the_book_with_exit_map_only() {
    let texts = Texts::parse("!DEFEAT\tDEFEAT\n!You have lost the Map!\tYou have lost the Map!\n");
    let book = Book {
        picture: Some("ui_briefing_screen_lost".into()),
        title: "campaign01".into(),
        end: Some(EndScreen::Defeat),
        ..Book::default()
    };
    let panel = book_panel(&texts, SCREEN, &book);
    let texts_shown = shown(&panel);
    assert!(
        texts_shown.iter().any(|t| t == "CAMPAIGN01"),
        "{texts_shown:?}"
    );
    assert!(texts_shown.iter().any(|t| t == "DEFEAT"));
    assert!(panel.widgets.iter().any(|w| matches!(
        w,
        Widget::Paragraph { text, at, .. } if text == "You have lost the Map!" && *at == [150.0, 120.0]
    )));
    let bars: Vec<(&Action, [f32; 2])> = panel
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton { action, at, .. } => Some((action, *at)),
            _ => None,
        })
        .collect();
    assert_eq!(bars, [(&Action::Menu(MenuAction::EndExit), [190.0, 465.0])]);
    assert!(!panel.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::Menu(MenuAction::BookPage(_)),
            ..
        }
    )));
}

// CAM-1.3c (T19a, E-cam-013): a missing text key gives the key itself, so a
// lesson without a key of its name shows its map name in capitals.
// spec: CAM-1.3c (T19a)
#[test]
fn t19a_a_lesson_without_a_key_shows_its_map_name() {
    let texts = Texts::parse("tutorial01\tTutorial 1\n");
    assert_eq!(
        book_title(&texts, "tutorial01").to_uppercase(),
        "TUTORIAL 1"
    );
    for lesson in ["tutorial02", "tutorial03", "tutorial04"] {
        assert_eq!(
            book_title(&texts, lesson).to_uppercase(),
            lesson.to_uppercase()
        );
    }
}

// CAM-5.3c (T19b, E-cam-014): the dialogs' steps take one control each: the
// shipyard's switch (2), the building window's close (3), the harbour's
// expedition tab (5), its start button (6), its close (7), the attack button
// (0x18), the economy dialog's military tab (0x1a) and its close (0x1b); a
// wrong click on a page-choice step (5, 0x1a) shows the street-icon error,
// on the others the marked-button error.
// spec: CAM-5.3c (T19b)
#[test]
fn t19b_each_dialog_step_takes_its_one_control() {
    use crate::ui::widgets::Global;
    let p = PlayerId(0);
    let at = (10, 10);
    let switch = Action::Command(Command::ProductionSwitch { player: p, at });
    let start = Action::Command(Command::ExpeditionStart { player: p, at });
    let cases: [(u8, Action, Action); 8] = [
        (0x02, switch.clone(), Action::CloseContext),
        (0x03, Action::CloseContext, Action::CloseGlobal),
        (0x05, Action::BuildingTab(12), Action::BuildingTab(11)),
        (0x06, start, Action::CloseContext),
        (0x07, Action::CloseContext, Action::CloseGlobal),
        (0x18, Action::AttackGo, Action::CloseContext),
        (
            0x1a,
            Action::OpenGlobal(Global::Military),
            Action::OpenGlobal(Global::Distribution(0)),
        ),
        (0x1b, Action::CloseGlobal, Action::CloseContext),
    ];
    for (step, right, wrong) in cases {
        assert_eq!(expects(step, &right), Some(true), "step {step:#x}");
        assert_eq!(expects(step, &wrong), Some(false), "step {step:#x}");
    }
    for step in [0x05, 0x1a, 0x15, 0x25, 0x48, 0x4b] {
        assert_eq!(
            wrong_click_error(step),
            "T01_ERROR_SelectStreetIcon",
            "{step:#x}"
        );
    }
    for step in [0x02, 0x03, 0x06, 0x07, 0x18, 0x1b] {
        assert_eq!(
            wrong_click_error(step),
            "T01_ERROR_SelectMarkedButton",
            "{step:#x}"
        );
    }
}

/// A free game of three players (slots 1, 2 and 4) on open land, each with
/// a headquarters: the end screen tests' world.
fn three_players() -> s2_sim::World {
    let n = 40 * 40;
    let terrain = s2_sim::map::Terrain {
        width: 40,
        height: 40,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut castle = s2_sim::BuildingType::new(9, s2_sim::BuildingSize::Large);
    castle.territory_range = 4;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![castle],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![
            (PlayerId(0), (8, 8)),
            (PlayerId(1), (30, 8)),
            (PlayerId(3), (30, 30)),
        ],
        local_player: PlayerId(0),
        ..Default::default()
    };
    s2_sim::World::new_game(1, terrain, rules, &setup)
}

// CAM-3.6a (E-trace-032, seen): a free game won with `INeedSomeHelp` (CAM-3.7)
// shows VICTORY and the target's text, but the cheat marks no player won:
// the human's row has no VICTORY, nor have the computer players still in the
// game (the result column at x 500 is empty).
// spec: CAM-3.6a, CAM-3.7
#[test]
fn after_the_cheat_no_row_shows_victory() {
    let mut world = three_players();
    world.set_mission(s2_sim::mission::Mission::new(1, Vec::new()));
    world.submit(Command::Cheat {
        player: HUMAN,
        cheat: s2_sim::mission::Cheat::INeedSomeHelp,
    });
    world.step();
    assert_eq!(world.mission().unwrap().completed, Some(1));
    let texts = Texts::parse("!VICTORY\tVICTORY\n");
    let panel = end_panel(
        &texts,
        SCREEN,
        EndScreen::Victory(1),
        &world,
        "MP_4P_Keyhole",
        "Look",
    );
    let victory: Vec<[f32; 2]> = panel
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, at, .. } if text == "VICTORY" => Some(*at),
            _ => None,
        })
        .collect();
    assert_eq!(victory.len(), 1, "{victory:?}");
    assert!(panel
        .widgets
        .iter()
        .all(|w| !matches!(w, Widget::Text { at, .. } if at[0] == 500.0)));
}

// CAM-3.6a (E-trace-032, seen): the end screen's Statistics button opens the
// ordinary statistics window (UI-GAME-8.5a) over the end screen, which stays
// behind it: the window takes a click where it lies over the end screen's
// buttons; elsewhere the end screen's buttons are reached; a window open
// before the end screen lies under it.
// spec: CAM-3.6a, UI-GAME-8.5a
#[test]
fn statistics_open_over_the_end_screen() {
    use crate::ui::render::{widget_at, Hit};
    use crate::ui::widgets::{Global, PanelId};
    let world = three_players();
    let texts = Texts::parse("!Statistics\tStatistics\n");
    let end = end_panel(
        &texts,
        SCREEN,
        EndScreen::Victory(1),
        &world,
        "MP_4P_Keyhole",
        "Look",
    );
    let button = |text: &str| {
        end.widgets
            .iter()
            .find_map(|w| match w {
                Widget::BarButton {
                    text: t,
                    action,
                    enabled,
                    at,
                    bar,
                    ..
                } if t == text => Some((action.clone(), *enabled, *at, bar.size)),
                _ => None,
            })
            .expect(text)
    };
    let (action, enabled, at, size) = button("Statistics");
    assert!(enabled);
    assert!(matches!(action, Action::OpenGlobal(Global::Statistics(_))));
    // A stand-in of the window: a button of its own over the Statistics
    // button's place.
    let on = [end.at[0] + at[0] + 2.0, end.at[1] + at[1] + 2.0];
    let window = Panel {
        id: PanelId::Global,
        at: [on[0] - 10.0, on[1] - 10.0],
        widgets: vec![Widget::BarButton {
            text: "x".into(),
            font: crate::ui::widgets::Font::Body,
            at: [0.0, 0.0],
            bar: crate::ui::widgets::DIALOG_BAR,
            raise: 0.0,
            action: Action::CloseGlobal,
            enabled: true,
        }],
        tooltips: Vec::new(),
    };
    let mut sizes = |w: &Widget| match w {
        Widget::BarButton { bar, .. } => Some(bar.size),
        _ => None,
    };
    let mut panels = vec![window, end.clone()];
    // Open before the end screen: under it.
    assert_eq!(
        widget_at(&panels, on, &mut sizes),
        Some(Hit::Button {
            action: action.clone(),
            enabled: true
        })
    );
    // Opened from it: over it.
    crate::ui::over_end_screen(&mut panels);
    assert_eq!(
        widget_at(&panels, on, &mut sizes),
        Some(Hit::Button {
            action: Action::CloseGlobal,
            enabled: true
        })
    );
    // EXIT MAP where the window does not cover it.
    let (exit, _, at, _) = button("!EXIT MAP");
    let far = [
        end.at[0] + at[0] + size[0] - 2.0,
        end.at[1] + at[1] + size[1] - 2.0,
    ];
    assert_eq!(
        widget_at(&panels, far, &mut sizes),
        Some(Hit::Button {
            action: exit,
            enabled: true
        })
    );
}

/// UI-GAME-11.1 (Q-1068): the geologist's settler type's display name, the
/// name a find's row shows, is the install's "Geologist". Skipped without
/// `S2_GAME_DIR`.
// spec: UI-GAME-11.1
#[test]
fn the_geologists_name_is_in_the_install_texts() {
    let Some((_, _, texts)) = install() else {
        return;
    };
    assert_eq!(
        texts.entry(crate::ui::game::GEOLOGIST_NAME),
        Some("Geologist")
    );
}

// UI-GAME-4.3 (E-trace-033, seen): a building's window closes by itself when
// the building is taken: it stays only while its field holds a building of
// the local player (here player 0's headquarters; player 1's, as a taken
// building would be, and an empty field close it).
// spec: UI-GAME-4.3
#[test]
fn a_taken_buildings_window_closes() {
    let world = three_players();
    let own = world
        .buildings()
        .find(|b| b.owner == HUMAN)
        .map(|b| b.at)
        .expect("player 0's headquarters");
    let other = world
        .buildings()
        .find(|b| b.owner == PlayerId(1))
        .map(|b| b.at)
        .expect("player 1's headquarters");
    assert!(crate::ui::building_window_stays(&world, own));
    assert!(!crate::ui::building_window_stays(&world, other));
    assert!(!crate::ui::building_window_stays(&world, (20, 20)));
}

// CAM-3.6a (T20c, E-trace-033, seen): on a defeat the end screen's CONTINUE
// is disabled but keeps its label's colour: its bar is the end screen's,
// drawn with the normal face darkened to about 55/77 of its values, no
// hover, no click; Statistics and EXIT MAP on the same bar, enabled. Other
// disabled dialog bars keep the pressed face and the grey label.
// spec: CAM-3.6a (T20c), UI-MENU-1.4
#[test]
fn a_disabled_continue_keeps_its_label() {
    use crate::ui::render::bar_look;
    use crate::ui::widgets::{DIALOG_BAR, END_BAR};
    let world = three_players();
    let texts = Texts::parse("!CONTINUE\tCONTINUE\n");
    let panel = end_panel(
        &texts,
        SCREEN,
        EndScreen::Defeat,
        &world,
        "MP_2P_Summer_Breeze",
        "Look",
    );
    let continue_bar = panel
        .widgets
        .iter()
        .find_map(|w| match w {
            Widget::BarButton {
                text, bar, enabled, ..
            } if text == "CONTINUE" => Some((*bar, *enabled)),
            _ => None,
        })
        .expect("CONTINUE");
    assert_eq!(continue_bar, (END_BAR, false));
    let (face, label, dim) = bar_look(&END_BAR, false);
    assert_eq!(face, DIALOG_BAR.normal);
    assert_eq!(label, [255, 255, 255]);
    assert!((dim.unwrap() - 55.0 / 77.0).abs() < 1e-6);
    assert_eq!(
        bar_look(&END_BAR, true),
        (DIALOG_BAR.normal, [255, 255, 255], None)
    );
    assert_eq!(
        bar_look(&DIALOG_BAR, false),
        (DIALOG_BAR.pressed, [170, 170, 170], None)
    );
    // Every button of the end screen is on the end bar.
    let bars: Vec<_> = panel
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton { bar, .. } => Some(*bar),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 3);
    assert!(bars.iter().all(|b| *b == END_BAR));
}
