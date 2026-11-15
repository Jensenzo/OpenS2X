use super::*;
use crate::ui::widgets::MENU_BAR;

/// UI-SETUP T6, T19: a single-player free game's slots 1..5 are computers
/// with strength 0 and tribe 0, and start position, colour and team all
/// the slot's number (Q-1045); the first click on slot 1's colour gives 2.
// spec: UI-SETUP-1.1, UI-SETUP-1.6 (T6, T19)
#[test]
fn t6_single_player_slots() {
    let mut slots = single_player_slots();
    assert_eq!(slots[0].kind, SlotKind::Human);
    for (i, s) in slots.iter().enumerate().skip(1) {
        assert_eq!((s.kind, s.strength, s.tribe), (SlotKind::Computer, 0, 0));
        let i = i as i8;
        assert_eq!((s.start, s.colour, s.team), (i, i, i));
    }
    assert!(slot_click(&mut slots, &MenuAction::SlotColour(1)));
    assert_eq!(slots[1].colour, 2);
}

/// UI-SETUP T1, T5, T8: tribes go to the players, strength 1 is AI
/// profile 2, an unset tribe counts as 0; closed slots and slots past
/// the map's start positions make no player.
// spec: UI-SETUP-2.1, UI-SETUP-2.2, UI-SETUP-1.6
#[test]
fn players_from_slots() {
    let mut slots = single_player_slots();
    slots[0].tribe = 2;
    slots[1] = Slot {
        kind: SlotKind::Computer,
        strength: 1,
        tribe: 1,
        start: -1,
        team: 0,
        colour: -1,
    };
    slots[2] = Slot {
        kind: SlotKind::Closed,
        strength: 0,
        tribe: -1,
        start: -1,
        team: -1,
        colour: -1,
    };
    slots[3].tribe = -1;
    let players = setup_players(&slots, 4);
    assert_eq!(
        players,
        [
            SetupPlayer {
                player: PlayerId(0),
                start: 0,
                tribe: 2,
                colour: 0,
                team: -1,
                profile: None
            },
            SetupPlayer {
                player: PlayerId(1),
                start: 1,
                tribe: 1,
                colour: 1,
                team: 0,
                profile: Some(2)
            },
            // UI-SETUP-1.6 (a): a new computer slot's team is its number.
            SetupPlayer {
                player: PlayerId(3),
                start: 3,
                tribe: 0,
                colour: 3,
                team: 3,
                profile: Some(1)
            },
        ]
    );
}

/// UI-SETUP-2.1 and the start checks: a chosen HQ is the player's start
/// position; two players on one HQ cannot start.
#[test]
fn chosen_start_positions() {
    let mut slots = single_player_slots();
    slots[0].start = 1;
    slots[1].start = 0;
    let players = setup_players(&slots, 2);
    assert_eq!(
        players
            .iter()
            .map(|p| (p.player, p.start))
            .collect::<Vec<_>>(),
        [(PlayerId(0), 1), (PlayerId(1), 0)]
    );
    assert!(starts_distinct(&players));
    slots[1].start = 1;
    assert!(!starts_distinct(&setup_players(&slots, 2)));
}

/// UI-SETUP T12, T13: the preview's start markers on Coastal Storm.
// spec: UI-SETUP-1.8a (T12, T13)
#[test]
fn t12_t13_start_markers() {
    let menu = free_play_menu();
    let summary = menu.summary.as_ref().unwrap();
    let markers = start_markers(summary, &menu.slots);
    assert_eq!(markers.len(), 2);
    let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01;
    assert_eq!(markers[0].0, "1");
    assert!(close(markers[0].1, [138.72, 227.60]), "{:?}", markers[0].1);
    assert_eq!(markers[0].2, [90, 90, 255]);
    assert_eq!(markers[1].0, "2");
    assert!(close(markers[1].1, [202.13, 227.60]), "{:?}", markers[1].1);
    assert_eq!(markers[1].2, [255, 90, 90]);
    // T13: slot 1 on HQ 1 too: "2" still drawn, brown; "1" slot 0's.
    let mut slots = menu.slots.clone();
    slots[1].start = 0;
    let markers = start_markers(summary, &slots);
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].2, [90, 90, 255]);
    assert_eq!(markers[1].2, [128, 64, 0]);
    // The page draws each marker as four black copies, then the colour.
    let page = free_play(&menu, &Texts::default(), [1024.0, 768.0]);
    let twos: Vec<_> = page
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text {
                text, colour, font, ..
            } if text == "2" && *font == Font::Heading => Some(*colour),
            _ => None,
        })
        .collect();
    assert_eq!(
        twos,
        [[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0], [255, 90, 90]]
    );
}

fn free_play_menu() -> MenuState {
    MenuState {
        screen: Screen::SinglePlayer,
        maps: vec![PathBuf::from("freeGameMaps/MP_2P_Storm_Coast.s2m")],
        map_names: vec!["MP_2P_Storm_Coast".into()],
        summary: Some(MapSummary {
            path: PathBuf::from("freeGameMaps/MP_2P_Storm_Coast.s2m"),
            name: "MP_2P_Storm_Coast".into(),
            size: (170, 170),
            players: 2,
            starts: vec![(29, 51), (139, 51)],
        }),
        slots: single_player_slots().to_vec(),
        resources: 1,
        fog: true,
        ..MenuState::default()
    }
}

fn bar_texts(p: &Panel) -> Vec<(String, [f32; 2], bool)> {
    p.widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton {
                text, at, enabled, ..
            } => Some((text.clone(), *at, *enabled)),
            _ => None,
        })
        .collect()
}

fn sprites(p: &Panel) -> Vec<(Atlas, [u32; 4], [f32; 2])> {
    p.widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Sprite {
                atlas, rect, at, ..
            } => Some((*atlas, *rect, *at)),
            Widget::Button {
                atlas, rect, at, ..
            } => Some((*atlas, *rect, *at)),
            _ => None,
        })
        .collect()
}

/// UI-MENU-1.5, UI-SETUP-1.8 (Run D): the single-player page on 1024 x
/// 768: the scroll's rows with FREEPLAY GAME pressed; the map button at
/// (119, 124), the preview's frame at (119, 156), "Map Size" / "Players"
/// at (232, 172 / 212) over their values (190 / 230); the options' drop-
/// downs at (433, 124 + 45 n) in the order start resources, victory
/// condition, fog of war; slot i at P = (180 + 275 (i mod 2), 405 + 80
/// (i / 2)) with its emblem at P, colour square at P + (-65, 20),
/// portrait at P + (-47, -25) and the HQ / kind / team drop-downs at
/// P.x + 45, P.y - 22 / P.y / P.y + 22; the options' captions 18 px above
/// their bars (T15, Q-331).
// spec: UI-SETUP-1.8 (T15)
#[test]
fn free_play_page() {
    let texts = Texts::parse(
        "!LOAD GAME\tLOAD GAME\n!START CAMPAIGN\tCAMPAIGN\n!TUTORIAL\tTUTORIAL\n\
         !START FREE GAME\tFREEPLAY GAME\n!BACK\tBACK\nMP_2P_Storm_Coast\tCoastal Storm\n\
         !Medium\tMedium\n!Endless\tNone (Endless)\n!On\tOn\n!HQ1\tHQ 1\n!HQ2\tHQ 2\n\
         !AI\tAI\n!Team 1\tTeam 1\n!Team 2\tTeam 2\n!Start\tStart\n\
         !Map Size\tMap Size\n!Players\tPlayers\n",
    );
    let p = free_play(&free_play_menu(), &texts, [1024.0, 768.0]);
    let bars = bar_texts(&p);
    let find = |t: &str| bars.iter().find(|b| b.0 == t).map(|b| (b.1, b.2));
    assert_eq!(find("LOAD GAME"), Some(([700.0, 300.0], true)));
    assert_eq!(find("CAMPAIGN"), Some(([700.0, 400.0], true)));
    assert_eq!(find("TUTORIAL"), Some(([700.0, 450.0], true)));
    assert_eq!(find("FREEPLAY GAME"), Some(([700.0, 500.0], false)));
    assert_eq!(find("BACK"), Some(([700.0, 600.0], true)));
    assert_eq!(find("Coastal Storm"), Some(([119.0, 124.0], true)));
    assert_eq!(find("Medium").map(|b| b.0), Some([433.0, 124.0]));
    assert_eq!(find("None (Endless)").map(|b| b.0), Some([433.0, 169.0]));
    assert_eq!(find("On").map(|b| b.0), Some([433.0, 214.0]));
    // Slot 0 at (180, 405), slot 1 at (455, 405).
    assert_eq!(find("HQ 1").map(|b| b.0), Some([225.0, 383.0]));
    assert_eq!(find("default").map(|b| b.0), Some([225.0, 405.0]));
    assert_eq!(find("Team 1").map(|b| b.0), Some([225.0, 427.0]));
    assert_eq!(find("HQ 2").map(|b| b.0), Some([500.0, 383.0]));
    assert_eq!(find("AI").map(|b| b.0), Some([500.0, 405.0]));
    assert_eq!(find("Team 2").map(|b| b.0), Some([500.0, 427.0]));
    assert!(find("START").is_some());
    let texts_at: Vec<(String, [f32; 2])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, at, .. } => Some((text.clone(), *at)),
            _ => None,
        })
        .collect();
    let text = |t: &str| texts_at.iter().find(|x| x.0 == t).map(|x| x.1);
    // UI-SETUP-1.8, T15 (Q-331), UI-MENU-1.8 b: the option captions at
    // (0, -10) from their bars, on the bars' left edge (their line boxes,
    // centred there, 18 px above): row 1 at (433, 159).
    assert_eq!(text("!Start resources:"), Some([433.0, 114.0]));
    assert_eq!(text("!Victory condition:"), Some([433.0, 159.0]));
    assert_eq!(text("!FOW"), Some([433.0, 204.0]));
    assert_eq!(text("Map Size"), Some([232.0, 172.0]));
    assert_eq!(text("170 x 170"), Some([232.0, 190.0]));
    assert_eq!(text("Players"), Some([232.0, 212.0]));
    // "2" is also the second start marker (1.8a).
    assert!(texts_at.contains(&("2".into(), [232.0, 230.0])));
    let sprites = sprites(&p);
    let at = |rect: [u32; 4]| {
        sprites
            .iter()
            .filter(|s| s.1 == rect)
            .map(|s| s.2)
            .collect::<Vec<_>>()
    };
    assert_eq!(at([520, 104, 104, 104]), [[119.0, 156.0]]);
    // The emblem 48 x 48 at P (UI-SETUP-1.8, T16: the human's of level 2,
    // the weak computer's of level 0), the colour square 24 x 24 at P +
    // (-65, 20), the portrait 46 x 80 at P + (-47, -25).
    assert_eq!(at([616, 8, 48, 48]), [[180.0, 405.0]]);
    assert_eq!(at([520, 8, 48, 48]), [[455.0, 405.0]]);
    assert!(sprites
        .iter()
        .any(|s| s.1[2..] == [24, 24] && s.2 == [115.0, 425.0]));
    assert!(sprites
        .iter()
        .any(|s| s.1[2..] == [46, 80] && s.2 == [133.0, 380.0]));
}

/// UI-SETUP-1.8: the options' lists and their values.
#[test]
fn option_lists() {
    let texts = Texts::default();
    let menu = free_play_menu();
    let keys = |d| drop_entries(&menu, &texts, d);
    assert_eq!(
        keys(Drop::Victory),
        [
            "!Endless",
            "!Destroy all enemies",
            "!Conquer 3/4 of the Map",
            "!produce 100 coins"
        ]
    );
    assert_eq!(keys(Drop::Fog), ["!On", "!Off"]);
    assert_eq!(drop_value(&menu, Drop::Fog), 0, "On preselected");
    let mut m = free_play_menu();
    choose(&mut m, Drop::Fog, 1);
    assert!(!m.fog);
    choose(&mut m, Drop::Victory, 3);
    assert_eq!(drop_value(&m, Drop::Victory), 3);
}

/// UI-MENU T9 (Run D): MULTIPLAYER is enabled (the normal bar) even
/// without network play.
// spec: UI-MENU-1.2 (T9)
#[test]
fn t9_multiplayer_enabled() {
    let texts = Texts::parse("!MULTIPLAYER\tMULTIPLAYER\n");
    let bars = bar_texts(&main_menu(&texts, [1024.0, 768.0], &[], None));
    assert_eq!(
        bars.iter().find(|b| b.0 == "MULTIPLAYER").map(|b| b.2),
        Some(true)
    );
}

/// UI-MENU T10, 1.5 (Run D): CONTINUE is the profile's autosave
/// (default `Autosave`) in the save folder, enabled exactly when it
/// exists; not the newest save.
#[test]
fn t10_continue_is_the_autosave() {
    let dir = std::env::temp_dir().join(format!("s2-t10-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("newer.sav"), b"x").unwrap();
    assert_eq!(autosave_file(&dir, &dir, ""), None);
    std::fs::write(dir.join("Autosave.sav"), b"x").unwrap();
    assert_eq!(
        autosave_file(&dir, &dir, ""),
        Some(dir.join("Autosave.sav"))
    );
    assert_eq!(autosave_file(&dir, &dir, "Mine"), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// OBS-20: a drop-down opens its list under its bar; an entry taken
/// closes it.
#[test]
fn drop_down_lists() {
    let mut menu = free_play_menu();
    menu.drop = Some(Drop::Hq(1));
    let p = free_play(&menu, &Texts::default(), [1024.0, 768.0]);
    let rows: Vec<_> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton {
                action: Action::Menu(MenuAction::Choose(Drop::Hq(1), n)),
                ..
            } => Some(*n),
            _ => None,
        })
        .collect();
    assert_eq!(rows, [0, 1]);
    choose(&mut menu, Drop::Hq(1), 0);
    assert_eq!(menu.slots[1].start, 0);
    choose(&mut menu, Drop::Kind(1), 1);
    assert_eq!(menu.slots[1].kind, SlotKind::Closed);
    choose(&mut menu, Drop::Kind(0), 1);
    assert_eq!(menu.slots[0].kind, SlotKind::Human);
}

/// UI-MENU-2.1, 1.9: Select Map on 1024 x 768 (the 376 x 464 dialog at
/// (324, 152)): 11 rows from (50, 50), the chosen one yellow; "Map
/// Size:" and "Players:" ending at x 120 from y 325, the values of the map
/// under the pointer at x 125, in (200, 160, 111); the preview at
/// (195, 315) in its frame at (192, 312); no OK or Back, a close button.
/// Over no row the values and the preview are blank.
// spec: UI-MENU-2.1, UI-MENU-1.8, UI-MENU-1.9
#[test]
fn select_map_layout() {
    let mut menu = free_play_menu();
    menu.maps = (0..14)
        .map(|i| PathBuf::from(format!("freeGameMaps/m{i}.s2m")))
        .collect();
    menu.map_names = (0..14).map(|i| format!("m{i}")).collect();
    menu.map_choice = 2;
    menu.screen = Screen::SelectMap;
    menu.map_hover = Some((5, menu.summary.clone()));
    let texts = Texts::parse("!Map Size:\tMap size:\n!Players:\tPlayers:\n");
    let p = select_map(&menu, &texts, [1024.0, 768.0]);
    let o = [324.0, 152.0];
    let rows: Vec<_> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::TextButton {
                text,
                at,
                colour,
                action: Action::Menu(MenuAction::SelectMap(i)),
                ..
            } => Some((text.clone(), [at[0] - o[0], at[1] - o[1]], *colour, *i)),
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), MAP_ROWS);
    assert_eq!(rows[0].1, [50.0, 50.0]);
    assert_eq!(rows[2].2, [255, 220, 0]);
    assert_eq!(rows[1].2, WHITE);
    let info = |p: &Panel| -> Vec<_> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Text {
                    text,
                    at,
                    colour,
                    align,
                    ..
                } if *colour == MAP_INFO_COLOUR => {
                    Some((text.clone(), [at[0] - o[0], at[1] - o[1]], *align))
                }
                _ => None,
            })
            .collect()
    };
    use crate::ui::widgets::Align;
    assert_eq!(
        info(&p),
        [
            ("Map size:".into(), [120.0, 325.0], Align::TopRight),
            ("170 x 170".into(), [125.0, 325.0], Align::TopLeft),
            ("Players:".into(), [120.0, 347.0], Align::TopRight),
            ("2".into(), [125.0, 347.0], Align::TopLeft),
        ]
    );
    assert!(sprites(&p).contains(&(
        Atlas::Elements0,
        [520, 104, 104, 104],
        [o[0] + 192.0, o[1] + 312.0]
    )));
    let picture = |p: &Panel| {
        p.widgets.iter().any(|w| matches!(
            w,
            Widget::Picture { at, size, .. } if *at == [o[0] + 195.0, o[1] + 315.0] && *size == 98.0
        ))
    };
    assert!(picture(&p));
    // No OK or Back bar; the close button at (w - 40, 5).
    assert!(!p
        .widgets
        .iter()
        .any(|w| matches!(w, Widget::BarButton { .. })));
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button { at, action: Action::Menu(MenuAction::Back), .. }
            if *at == [o[0] + 336.0, o[1] + 5.0]
    )));
    // UI-MENU-1.9: its own frame, `uiBigDialog` (8, 8, 376 x 464) at the
    // dialog's corner, and the standard title centred at (188, 14).
    assert!(sprites(&p).contains(&(Atlas::BigDialog, [8, 8, 376, 464], o)));
    // Over no row: the captions stay, the values and the preview go.
    menu.map_hover = None;
    let p = select_map(&menu, &texts, [1024.0, 768.0]);
    assert_eq!(
        info(&p).iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["Map size:", "Players:"]
    );
    assert!(!picture(&p));
}

/// UI-MENU-2.1 "The scroll bar": the list's vertical bar at (252, 18) from
/// the list's corner (the list at (50, 50) of the dialog at (324, 152) on
/// 1024 x 768, UI-MENU-1.9): the track `uiElements0` (456, 197) 24 x 194, the 24 x 24
/// thumb (8, 104) on it, at the bar's top with the list at its top and at
/// its foot with the list at its end; the up arrow (8, 128) at (252, -1),
/// the down arrow (104, 128) at (252, 212), each with its looks 32 and 64
/// px to the right (highlighted, pressed). No arrow beside the rows.
// spec: UI-MENU-2.1
#[test]
fn select_map_scroll_bar() {
    let mut menu = free_play_menu();
    menu.maps = (0..14)
        .map(|i| PathBuf::from(format!("freeGameMaps/m{i}.s2m")))
        .collect();
    menu.map_names = (0..14).map(|i| format!("m{i}")).collect();
    menu.screen = Screen::SelectMap;
    let o = [324.0 + 50.0, 152.0 + 50.0];
    let at = |x: f32, y: f32| [o[0] + x, o[1] + y];
    let thumb = |p: &Panel| -> Vec<[f32; 2]> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Sprite {
                    atlas: Atlas::Elements0,
                    rect: [8, 104, 24, 24],
                    at,
                    ..
                } => Some(*at),
                _ => None,
            })
            .collect()
    };
    let p = select_map(&menu, &Texts::default(), [1024.0, 768.0]);
    assert!(sprites(&p).contains(&(Atlas::Elements0, [456, 197, 24, 194], at(252.0, 18.0))));
    assert_eq!(thumb(&p), [at(252.0, 18.0)]);
    let arrows: Vec<_> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                atlas: Atlas::Elements0,
                rect,
                hover,
                pressed,
                at,
                action: Action::Menu(MenuAction::MapScroll(d)),
                enabled,
                ..
            } => Some((*rect, hover.clone(), pressed.clone(), *at, *d, *enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(
        arrows,
        [
            (
                [8, 128, 24, 24],
                Hover::Swap([40, 128]),
                Hover::Swap([72, 128]),
                at(252.0, -1.0),
                -1,
                // Q-1042 (b): off with the list at its top.
                false
            ),
            (
                [104, 128, 24, 24],
                Hover::Swap([136, 128]),
                Hover::Swap([168, 128]),
                at(252.0, 212.0),
                1,
                true
            ),
        ]
    );
    // The list at its end (14 maps, 11 rows: row 3 at the top): the thumb
    // at the bar's foot.
    menu.map_bar = 1.0;
    let p = select_map(&menu, &Texts::default(), [1024.0, 768.0]);
    assert_eq!(thumb(&p), [at(252.0, 18.0 + 194.0 - 24.0)]);
    // Q-1042 (a), (b): everything fits: the thumb in the middle (85 px),
    // both arrows off.
    menu.maps.truncate(9);
    menu.map_names.truncate(9);
    menu.map_scroll = 0;
    let p = select_map(&menu, &Texts::default(), [1024.0, 768.0]);
    assert_eq!(thumb(&p), [at(252.0, 18.0 + 85.0)]);
    assert!(!p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::Menu(MenuAction::MapScroll(_)),
            enabled: true,
            ..
        }
    )));
}

/// UI-MENU-2.1: the map button opens Select Map; the arrows move the bar's
/// value (Q-1042) and the 14 maps' first row follows it within 0..3; a click on a map whose header does not read
/// shows the `!INVALID_MAP` box and keeps the dialog and the map; the box's
/// close button closes it, the dialog's leaves with the map unchanged.
// spec: UI-MENU-2.1
#[test]
fn select_map_actions() {
    let mut menu = free_play_menu();
    menu.maps = (0..14)
        .map(|i| PathBuf::from(format!("freeGameMaps/m{i}.s2m")))
        .collect();
    menu.map_names = (0..14).map(|i| format!("m{i}")).collect();
    let p = free_play(&menu, &Texts::default(), [1024.0, 768.0]);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::BarButton {
            action: Action::Menu(MenuAction::OpenSelectMap),
            ..
        }
    )));
    assert!(select_map_apply(&mut menu, &MenuAction::OpenSelectMap));
    assert_eq!(menu.screen, Screen::SelectMap);
    // Q-1042 (b): 14 maps, a step of 1 / 7; the first row floor(3 v + 0.5).
    let mut firsts = Vec::new();
    for _ in 0..8 {
        select_map_apply(&mut menu, &MenuAction::MapScroll(1));
        firsts.push(menu.map_scroll);
    }
    assert_eq!(firsts, [0, 1, 1, 2, 2, 3, 3, 3]);
    assert_eq!(menu.map_bar, 1.0);
    select_map_apply(&mut menu, &MenuAction::MapScroll(-1));
    select_map_apply(&mut menu, &MenuAction::MapScroll(-1));
    assert_eq!(menu.map_scroll, 2);
    // The pointer over an unreadable map: nothing to show.
    hover_map(&mut menu, Some(5));
    assert_eq!(menu.map_hover.as_ref().map(|h| h.0), Some(5));
    assert!(shown_map(&menu).is_none());
    select_map_apply(&mut menu, &MenuAction::SelectMap(5));
    assert!(menu.invalid_map);
    assert_eq!((menu.screen, menu.map_choice), (Screen::SelectMap, 0));
    let texts = Texts::parse("!INVALID_MAP\tInvalid map\n!INVALID_FILE\tInvalid file\n");
    let p = select_map(&menu, &texts, [1024.0, 768.0]);
    let shown: Vec<&str> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(shown.contains(&"INVALID MAP") && shown.contains(&"Invalid file"));
    select_map_apply(&mut menu, &MenuAction::Back);
    assert!(!menu.invalid_map);
    assert_eq!(menu.screen, Screen::SelectMap);
    select_map_apply(&mut menu, &MenuAction::Back);
    assert_eq!((menu.screen, menu.map_choice), (Screen::SinglePlayer, 0));
    assert!(menu.summary.is_some());
    assert!(!select_map_apply(&mut menu, &MenuAction::Back));
}

/// UI-MENU-1.5 (seen in the original's pictures): SINGLE PLAYER opens its
/// page on CAMPAIGN; FREEPLAY GAME swaps in the free play setup with the
/// free game maps only.
// spec: UI-MENU-1.5
#[test]
fn single_player_opens_on_campaign() {
    let mut menu = MenuState {
        screen: Screen::Main,
        ..MenuState::default()
    };
    let maps = [
        PathBuf::from("freeGameMaps/MP_2P_Storm_Coast.s2m"),
        PathBuf::from("campaign/campaign01.s2m"),
    ];
    single_player_apply(&mut menu, &MenuAction::SinglePlayer, &maps);
    assert_eq!(menu.screen, Screen::Campaign);
    single_player_apply(&mut menu, &MenuAction::FreeGame, &maps);
    assert_eq!(menu.screen, Screen::SinglePlayer);
    assert_eq!(menu.maps, free_maps(&maps));
}

/// UI-MENU-2.1 "How the bar works" (T18, Q-1042): with the 20 shipped maps
/// an arrow click is 0.1 and the first row after k clicks is round(0.9 k):
/// 1, 2, 3, 4, 5, 5, 6, 7, 8, 9; the bar stays in 0..1; with everything in
/// view v is 0.5 and the first row 0.
// spec: UI-MENU-2.1 (T18)
#[test]
fn t18_the_bar_steps_by_its_value() {
    let mut v = 0.0;
    let mut firsts = Vec::new();
    for _ in 0..10 {
        v = bar_step(v, 20, 1);
        firsts.push(bar_first(v, 20));
    }
    assert_eq!(firsts, [1, 2, 3, 4, 5, 5, 6, 7, 8, 9]);
    assert!((v - 1.0).abs() < 1e-6);
    assert_eq!(bar_step(1.0, 20, 1), 1.0);
    assert_eq!(bar_step(0.0, 20, -1), 0.0);
    assert_eq!((bar_step(0.0, 9, 1), bar_first(0.7, 9)), (0.5, 0));
}

/// UI-MENU-2.1 with the install's free game maps: the pointer over a row
/// shows that map; a click takes it and closes the dialog. Skipped without
/// `S2_GAME_DIR`.
// spec: UI-MENU-2.1
#[test]
fn select_map_takes_a_readable_map() {
    let Some(dir) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let folder = PathBuf::from(dir).join("data/maps/freeGameMaps");
    let mut maps: Vec<PathBuf> = std::fs::read_dir(&folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("s2m")))
        .collect();
    maps.sort();
    assert!(maps.len() > 2);
    let mut menu = free_play_menu();
    menu.map_names = maps.iter().map(|p| p.display().to_string()).collect();
    menu.maps = maps.clone();
    select_map_apply(&mut menu, &MenuAction::OpenSelectMap);
    hover_map(&mut menu, Some(2));
    assert_eq!(
        shown_map(&menu).map(|s| s.path.clone()),
        Some(maps[2].clone())
    );
    hover_map(&mut menu, None);
    assert!(shown_map(&menu).is_none());
    select_map_apply(&mut menu, &MenuAction::SelectMap(1));
    assert_eq!((menu.screen, menu.map_choice), (Screen::SinglePlayer, 1));
    assert_eq!(
        menu.summary.as_ref().map(|s| s.path.clone()),
        Some(maps[1].clone())
    );
    assert!(!menu.invalid_map);
}

/// UI-MENU-5.1, 5.3: the save dialog's name input at (50, 316), its text
/// at (10, 1) from the bar, the edited look and the caret while it has the
/// keys.
// spec: UI-MENU-5.1, UI-MENU-5.3
#[test]
fn save_name_input() {
    let menu = MenuState {
        screen: Screen::Save,
        save_name: "18-10-2026 Coastal Storm".into(),
        ..MenuState::default()
    };
    let texts = Texts::default();
    let o = [324.0, 152.0];
    for (input, shown, art) in [
        (None, "18-10-2026 Coastal Storm", [8, 8]),
        (
            Some(crate::ui::widgets::Focus {
                input: crate::ui::widgets::Input::SaveName,
                caret: 24,
                dot: true,
            }),
            "18-10-2026 Coastal Storm.",
            [8, 80],
        ),
    ] {
        let p = save_load(&menu, &texts, [1024.0, 768.0], true, input);
        assert!(p.widgets.iter().any(|w| matches!(
            w,
            Widget::Button {
                rect,
                at,
                action: Action::FocusInput(crate::ui::widgets::Input::SaveName, x),
                ..
            } if rect[..2] == art && *at == [o[0] + 50.0, o[1] + 316.0] && *x == o[0] + 60.0
        )));
        assert!(p.widgets.iter().any(|w| matches!(
            w,
            Widget::Text { text, at, .. } if text == shown && *at == [o[0] + 60.0, o[1] + 317.0]
        )));
    }
}

/// UI-MENU-1.1: on a wide screen `ui_back` covers the whole screen and
/// the parchment keeps its 1024 x 768 pixels, centred.
#[test]
fn main_menu_one_to_one() {
    let p = main_menu_background([1920.0, 1080.0]);
    let sprite = |atlas: Atlas| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Sprite {
                atlas: a, at, size, ..
            } if *a == atlas => Some((*at, *size)),
            _ => None,
        })
    };
    assert_eq!(
        sprite(Atlas::Back),
        Some(([0.0, 0.0], Some([1920.0, 1080.0])))
    );
    assert_eq!(
        sprite(Atlas::Title),
        Some(([448.0, 156.0], Some([1024.0, 768.0])))
    );
}

/// UI-MENU T1: on 1280 x 1024 the panel is at (128, 128) and EXIT at
/// panel (700, 550).
// spec: UI-MENU-1.1, UI-MENU-1.2 (T1)
#[test]
fn t1_main_menu() {
    let texts = Texts::parse("!EXIT\tEXIT GAME\n");
    let p = main_menu(&texts, [1280.0, 1024.0], &[], None);
    assert_eq!(desktop_origin([1280.0, 1024.0]), [128.0, 128.0]);
    let exit = p
        .widgets
        .iter()
        .find_map(|w| match w {
            Widget::BarButton { text, at, bar, .. } if text == "EXIT GAME" => Some((*at, *bar)),
            _ => None,
        })
        .unwrap();
    // UI-MENU-1.4: on the 208 x 32 menu bar.
    assert_eq!(exit, ([128.0 + 700.0, 128.0 + 550.0], MENU_BAR));
}

/// A project's entries (docs/extending.md) take the two empty rows
/// between CREDITS and EXIT, (700, 450) and (700, 500); every original
/// button keeps its place, and without entries the rows stay empty.
// spec: UI-MENU-1.2 (the original rows; the empty ones for the menu hook)
#[test]
fn entries_take_the_empty_rows() {
    let texts = Texts::default();
    let original = bar_texts(&main_menu(&texts, [1024.0, 768.0], &[], None));
    let ys: Vec<f32> = original.iter().map(|b| b.1[1]).collect();
    assert_eq!(ys, [250.0, 300.0, 350.0, 400.0, 550.0]);
    let entries = ["FRONTIER".to_owned(), "MODS".to_owned()];
    let p = main_menu(&texts, [1024.0, 768.0], &entries, None);
    let bars = bar_texts(&p);
    assert_eq!(bars.len(), 7);
    for b in &original {
        assert!(bars.contains(b), "{b:?} moved");
    }
    assert!(bars.contains(&("FRONTIER".to_owned(), [700.0, 450.0], true)));
    assert!(bars.contains(&("MODS".to_owned(), [700.0, 500.0], true)));
    let opens = p.widgets.iter().any(|w| {
        matches!(w, Widget::BarButton { text, action: Action::Menu(MenuAction::OpenPage(1)), .. } if text == "MODS")
    });
    assert!(opens);
}

/// UI-MENU-0.1: a plain start opens with the intro; a project's
/// `skip_launch_films` (docs/extending.md) opens the start screen, which
/// the intro leads to.
// spec: UI-MENU-0.1
#[test]
fn skipping_the_films_opens_the_start_screen() {
    assert_eq!(opening_screen(false), Screen::Intro);
    assert_eq!(opening_screen(true), Screen::Start);
    assert_eq!(opening_screen(true), after_intro());
}

/// A project's captions (docs/extending.md): the start screen's line in
/// `font_big_white` at (210, 496) in the title line's colour, the main
/// menu's footer in `dark_system` ending at (908, 672); without them both
/// screens are the original's, widget for widget.
// spec: UI-MENU-0.2, UI-MENU-1.2 (the screens the caption hook adds to)
#[test]
fn captions_add_one_line_each() {
    let texts = Texts::default();
    let screen = [1280.0, 1024.0];
    let o = desktop_origin(screen);
    let plain = start_screen(&texts, screen, None);
    let titled = start_screen(&texts, screen, Some("FRONTIER"));
    let added: Vec<&Widget> = titled
        .widgets
        .iter()
        .filter(|w| !plain.widgets.contains(w))
        .collect();
    assert_eq!(
        added,
        [&Widget::Text {
            text: "FRONTIER".into(),
            font: Font::Big,
            at: [o[0] + 210.0, o[1] + 496.0],
            colour: [229, 209, 196],
            align: crate::ui::widgets::Align::TopLeft,
        }]
    );
    assert_eq!(titled.widgets.len(), plain.widgets.len() + 1);
    let plain = main_menu(&texts, screen, &[], None);
    let footed = main_menu(&texts, screen, &[], Some("Frontier 0.1.0"));
    assert_eq!(footed.widgets[..plain.widgets.len()], plain.widgets[..]);
    assert_eq!(
        footed.widgets[plain.widgets.len()..],
        [Widget::Text {
            text: "Frontier 0.1.0".into(),
            font: Font::Dark,
            at: [o[0] + 908.0, o[1] + 672.0],
            colour: INK,
            align: crate::ui::widgets::Align::TopRight,
        }]
    );
}

/// A project's page: the menu dialog titled with the entry, its rows as
/// the game settings (UI-MENU-4.1): labels at x 24 in the headings'
/// colour from y 72 stepping 32, a toggle's Off / On at 168 / 268, a
/// choice's options three to a line, a button's bar at 168; BACK at
/// (50, 480); rows past the eleventh are left out. Entry, row and input
/// go back as menu actions.
// spec: UI-MENU-4.1 (the settings pages' rows the hook's page follows)
#[test]
fn an_extension_page_is_laid_out_like_the_settings() {
    use crate::extend::menu::{PageInput, Row};
    let texts = Texts::parse("!Off\tOff\n!On\tOn\n!Back\tBack\n");
    let mut menu = MenuState {
        screen: Screen::Main,
        entries: vec!["FRONTIER".into()],
        page: vec![
            Row::Caption("Mods".into()),
            Row::Toggle {
                label: "Follow camera".into(),
                value: true,
            },
            Row::Choice {
                label: "Music".into(),
                options: vec![
                    "Original".into(),
                    "Own".into(),
                    "Shuffle".into(),
                    "Off".into(),
                ],
                index: 2,
            },
            Row::Button("Reset".into()),
        ],
        ..MenuState::default()
    };
    let screen = [1024.0, 768.0];
    let p = extension_page(&menu, &texts, screen, 0);
    let o = [259.0, 114.0];
    let at = |x: f32, y: f32| [o[0] + x, o[1] + y];
    let texts_at: Vec<(String, [f32; 2], [u8; 3])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text {
                text, at, colour, ..
            }
            | Widget::TextButton {
                text, at, colour, ..
            } => Some((text.clone(), *at, *colour)),
            _ => None,
        })
        .collect();
    let yellow = [255, 220, 0];
    for expected in [
        ("FRONTIER".to_owned(), at(200.0, 9.0), WHITE),
        ("Mods".to_owned(), at(24.0, 72.0), HEADING),
        ("Follow camera".to_owned(), at(24.0, 104.0), HEADING),
        ("Off".to_owned(), at(168.0, 104.0), WHITE),
        ("On".to_owned(), at(268.0, 104.0), yellow),
        ("Music".to_owned(), at(24.0, 136.0), HEADING),
        ("Original".to_owned(), at(168.0, 136.0), WHITE),
        ("Shuffle".to_owned(), at(368.0, 136.0), yellow),
        ("Off".to_owned(), at(168.0, 160.0), WHITE),
    ] {
        assert!(texts_at.contains(&expected), "{expected:?} in {texts_at:?}");
    }
    let bars = bar_texts(&p);
    assert!(bars.contains(&("Reset".to_owned(), at(168.0, 200.0), true)));
    assert!(bars.contains(&("Back".to_owned(), at(50.0, 480.0), true)));
    let action = |text: &str, x: f32| {
        p.widgets.iter().find_map(|w| match w {
            Widget::TextButton {
                text: t,
                at,
                action,
                ..
            } if t == text && at[0] == o[0] + x => Some(action.clone()),
            _ => None,
        })
    };
    assert_eq!(
        action("Off", 168.0),
        Some(Action::Menu(MenuAction::Page(1, PageInput::Toggle(false))))
    );
    assert_eq!(
        action("Own", 268.0),
        Some(Action::Menu(MenuAction::Page(2, PageInput::Choose(1))))
    );
    // Eleven rows fit; the twelfth is left out.
    menu.page = (0..12).map(|n| Row::Caption(format!("row {n}"))).collect();
    let p = extension_page(&menu, &texts, screen, 0);
    let shown = |name: &str| {
        p.widgets
            .iter()
            .any(|w| matches!(w, Widget::Text { text, .. } if text == name))
    };
    assert!(shown("row 10") && !shown("row 11"));
}

// UI-MENU-3.1, 3.1a (Run D): the game menu has the five rows of 3.1
// and no CONTINUE, every row inside the visible art, the column centred
// on it. UI-MENU-3.1b: its own frame, `uiBigDialog` (8, 8, 376 x 464)
// drawn 1:1 and centred (at (324, 152) on 1024 x 768); the art covers
// 364 x 448 of it from the top-left.
#[test]
fn game_menu_rows_inside_the_panel() {
    let p = game_menu(&Texts::default(), [1024.0, 768.0]);
    assert_eq!(p.at, [324.0, 152.0]);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { atlas: Atlas::BigDialog, rect: [8, 8, 376, 464], at, size, .. }
            if *at == [0.0, 0.0] && size.is_none_or(|s| s == [376.0, 464.0])
    )));
    let mut rows = 0;
    for w in &p.widgets {
        if let Widget::BarButton { at, bar, .. } = w {
            rows += 1;
            assert!(at[1] >= 0.0 && at[1] + bar.size[1] <= 448.0, "{at:?}");
            assert!((at[0] + bar.size[0] / 2.0 - 182.0).abs() <= 1.0, "{at:?}");
        }
    }
    assert_eq!(rows, 5);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::Menu(MenuAction::CloseGameMenu),
            ..
        }
    )));
}

/// UI-MENU T11, 1.6 (Run D): the tutorial page keeps the scroll with
/// TUTORIAL pressed, draws the left sheet on `uiDesktop1` with the
/// lesson medallions at (210, 215) and `!Lesson 1` .. `!Lesson 4` at
/// (290, 240 / 310 / 380 / 450), each `!Info k` 15 px above.
#[test]
fn t11_tutorial_page() {
    let texts = Texts::parse(
        "!TUTORIAL\tTUTORIAL\n!Lesson1\tLesson 1\n!Lesson4\tLesson 4\n!Info1\tInfo 1\n",
    );
    let p = crate::menus::campaign::tutorial_menu(&texts, [1024.0, 768.0], false);
    let bars = bar_texts(&p);
    let find = |t: &str| bars.iter().find(|b| b.0 == t).map(|b| (b.1, b.2));
    assert_eq!(find("TUTORIAL"), Some(([700.0, 450.0], false)));
    assert_eq!(find("Lesson 1"), Some(([290.0, 240.0], true)));
    assert_eq!(find("Lesson 4"), Some(([290.0, 450.0], true)));
    let sprites = sprites(&p);
    assert!(sprites
        .iter()
        .any(|s| s.0 == Atlas::Desktop1 && s.2 == [0.0, 0.0]));
    assert!(sprites
        .iter()
        .any(|s| s.0 == Atlas::Elements0 && s.1 == [936, 520, 80, 280] && s.2 == [210.0, 215.0]));
    let info = p.widgets.iter().find_map(|w| match w {
        Widget::Text { text, at, .. } if text == "Info 1" => Some(*at),
        _ => None,
    });
    assert_eq!(info, Some([290.0, 225.0]));
}

// spec: FMT-SAV-1.3, UI-MENU-5.1
#[test]
fn save_names_use_the_date() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_743), (2026, 10, 17));
    // `DD-MM-YYYY HH-MM-SS <map name>`, the map being played; what a file name
    // cannot hold does not get in.
    let name = default_save_name("Coastal Storm");
    assert!(name.ends_with(" Coastal Storm"), "{name}");
    assert_eq!(name.len(), "DD-MM-YYYY HH-MM-SS Coastal Storm".len());
    assert!(default_save_name("a/b").ends_with(" a_b"));
}

/// UI-GAME-12.1 (S), SET-5: S opens the settings at the page shown last.
// spec: UI-GAME-12.1, SET-5.1
#[test]
fn settings_open_at_their_last_page() {
    let mut menu = MenuState {
        screen: Screen::Settings {
            page: 0,
            in_game: true,
        },
        ..MenuState::default()
    };
    assert!(settings_apply(&mut menu, &MenuAction::SettingsPage(1)));
    menu.screen = Screen::None;
    assert!(settings_apply(&mut menu, &MenuAction::SettingsLastPage));
    assert_eq!(
        menu.screen,
        Screen::Settings {
            page: 1,
            in_game: true
        }
    );
}

// A save that loads in part or not at all: a notice names the first part
// not read and its offset, alone over the game or on top of a menu page,
// until its close button.
// spec: FMT-SAV-3.1 (the sections a problem names)
#[test]
fn a_save_read_in_part_shows_a_notice() {
    use s2_assets::s2m::S2mError;
    use s2_assets::sav::SaveProblem;
    let problem = |section, record: Option<&str>, offset| SaveProblem {
        section,
        record: record.map(str::to_owned),
        offset,
        error: S2mError::WrongSection {
            offset,
            expected: "logic UniqueId",
        },
        resumed: None,
    };
    assert_eq!(Notice::for_save("Autosave", &[], true), None);
    let problems = [
        problem("VillageSystem", Some("remains record 1 of 1"), 5_044_945),
        problem("AI System", None, 5_109_409),
    ];
    let part = Notice::for_save("Autosave", &problems, true).unwrap();
    assert_eq!(part.title, "SAVE LOADED IN PART");
    assert_eq!(
        part.lines,
        [
            "VillageSystem, remains record 1 of 1",
            "not read from offset 5044945, and 1 more"
        ]
    );
    let failed = Notice::for_save("Autosave", &problems[1..], false).unwrap();
    assert_eq!(failed.title, "SAVE NOT LOADED");
    assert_eq!(failed.lines, ["Autosave", "AI System at offset 5109409"]);
    let mut menu = MenuState {
        notice: Some(part),
        ..Default::default()
    };
    let texts = Texts::default();
    let over_game = panels(&menu, &texts, [1024.0, 768.0], None).unwrap();
    assert_eq!(over_game.len(), 1);
    assert_eq!(
        (over_game[0].id, over_game[0].at),
        (PanelId::Notice, [368.0, 340.0])
    );
    let shown: Vec<&str> = over_game[0]
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        shown,
        [
            "SAVE LOADED IN PART",
            "VillageSystem, remains record 1 of 1",
            "not read from offset 5044945, and 1 more"
        ]
    );
    assert!(over_game[0].widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::Menu(MenuAction::CloseNotice),
            ..
        }
    )));
    menu.screen = Screen::Main;
    let over_menu = panels(&menu, &texts, [1024.0, 768.0], None).unwrap();
    assert_eq!(over_menu.len(), 2);
    assert_eq!(over_menu[1].id, PanelId::Notice);
}

/// A folder of its own for a test's save files.
fn save_folder_for(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("s2-saves-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder");
    dir
}

/// Write `name` with a modification time `secs` after the epoch.
fn save_file_at(dir: &Path, name: &str, secs: u64) {
    let path = dir.join(name);
    std::fs::write(&path, b"x").expect("the file");
    let file = std::fs::File::options()
        .write(true)
        .open(&path)
        .expect("opened");
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        .expect("the time set");
}

/// UI-MENU-5.1, 6.1: the list holds the original's saves and ours, in the order
/// the file system returns them, not sorted by time; an original's save whose name
/// one of ours has too is marked.
// spec: UI-MENU-5.1, UI-MENU-6.1, FMT-SAV-1.3
#[test]
fn the_save_list_holds_both_kinds_in_the_file_systems_order() {
    let dir = save_folder_for("list");
    save_file_at(&dir, "Autosave.sav", 100);
    save_file_at(&dir, "Autosave.s2save", 300);
    save_file_at(&dir, "17-03-2023 16-40-11 Coastal Storm.sav", 50);
    save_file_at(&dir, "mine.s2save", 200);
    save_file_at(&dir, "notes.txt", 400);
    save_file_at(&dir, "half.s2save.part", 500);
    let saves = saves_in(&dir);
    let listed: Vec<(String, bool)> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let (stem, ours) = match name.rsplit_once('.')? {
                (stem, "sav") => (stem.to_owned(), false),
                (stem, "s2save") => (stem.to_owned(), true),
                _ => return None,
            };
            Some((stem, ours))
        })
        .collect();
    assert_eq!(
        saves
            .iter()
            .map(|e| (e.name.clone(), e.ours))
            .collect::<Vec<_>>(),
        listed
    );
    let label = |ours: bool, name: &str| {
        saves
            .iter()
            .find(|e| e.ours == ours && e.name == name)
            .expect("listed")
            .label(&saves)
    };
    assert_eq!(saves.len(), 4);
    assert_eq!(label(true, "Autosave"), "Autosave");
    assert_eq!(label(false, "Autosave"), "Autosave (S2)");
    assert_eq!(label(true, "mine"), "mine");
    assert_eq!(
        label(false, "17-03-2023 16-40-11 Coastal Storm"),
        "17-03-2023 16-40-11 Coastal Storm"
    );
    let entry = |ours: bool, name: &str| {
        saves
            .iter()
            .find(|e| e.ours == ours && e.name == name)
            .expect("listed")
            .path(&dir)
    };
    assert_eq!(entry(true, "mine"), dir.join("mine.s2save"));
    assert_eq!(entry(false, "Autosave"), dir.join("Autosave.sav"));
    // UI-MENU-1.5: CONTINUE takes the autosave, the newer of the two.
    assert_eq!(
        autosave_file(&dir, &dir, ""),
        Some(dir.join("Autosave.s2save"))
    );
    save_file_at(&dir, "Autosave.sav", 600);
    assert_eq!(
        autosave_file(&dir, &dir, "Autosave"),
        Some(dir.join("Autosave.sav"))
    );
    assert_eq!(autosave_file(&dir, &dir, "Other"), None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A project's own save folder (`Extensions::save_dir`, W11 a): ours are
/// written to and listed from it, the original's `.sav` files still from
/// `<Documents>\S2\saves`; s2-reborn's own saves there are not listed, and
/// CONTINUE takes the newer of the project's autosave and the original's.
/// Without the hook, one folder as before; `--save-dir` wins over both.
// spec: UI-MENU-5.1, UI-MENU-1.5, FMT-SAV-1.1
#[test]
fn a_project_saves_in_its_own_folder_and_lists_the_originals_saves() {
    let own = save_folder_for("project-own");
    let original = save_folder_for("project-original");
    save_file_at(&own, "frontier game.s2save", 100);
    save_file_at(&own, "stray.sav", 100);
    save_file_at(&own, "Autosave.s2save", 300);
    save_file_at(&original, "reborn game.s2save", 100);
    save_file_at(&original, "Autosave.s2save", 500);
    save_file_at(&original, "Autosave.sav", 200);
    save_file_at(&original, "17-03-2023 16-40-11 Coastal Storm.sav", 50);
    let mut saves: Vec<(String, bool)> = saves_of(Some(&own), Some(&original))
        .into_iter()
        .map(|e| (e.name, e.ours))
        .collect();
    // Ours first, then the original's; each folder in its own order.
    assert!(saves[..2].iter().all(|e| e.1) && saves[2..].iter().all(|e| !e.1));
    saves.sort();
    assert_eq!(
        saves,
        [
            ("17-03-2023 16-40-11 Coastal Storm".to_owned(), false),
            ("Autosave".to_owned(), false),
            ("Autosave".to_owned(), true),
            ("frontier game".to_owned(), true),
        ]
    );
    // CONTINUE: the project's autosave (300) is newer than the original's
    // (200); s2-reborn's (500) is not the project's.
    assert_eq!(
        autosave_file(&own, &original, ""),
        Some(own.join("Autosave.s2save"))
    );
    save_file_at(&original, "Autosave.sav", 400);
    assert_eq!(
        autosave_file(&own, &original, ""),
        Some(original.join("Autosave.sav"))
    );
    // One folder (no project folder): everything in it, as before.
    assert_eq!(saves_of(Some(&original), Some(&original)).len(), 4);
    assert_eq!(saves_of(None, Some(&original)).len(), 4);
    // The folder ours go to: --save-dir, else the project's, else Documents'.
    let (option, project, documents) = (
        PathBuf::from("harness"),
        PathBuf::from("frontier"),
        PathBuf::from("Documents/S2/saves"),
    );
    assert_eq!(
        own_folder(Some(&option), Some(&project), Some(documents.clone())),
        Some(option)
    );
    assert_eq!(
        own_folder(None, Some(&project), Some(documents.clone())),
        Some(project)
    );
    assert_eq!(
        own_folder(None, None, Some(documents.clone())),
        Some(documents)
    );
    let _ = std::fs::remove_dir_all(&own);
    let _ = std::fs::remove_dir_all(&original);
}

/// UI-MENU-5.1: `!Save` is enabled only while the input is not empty.
// spec: UI-MENU-5.1
#[test]
fn a_save_needs_a_name() {
    let save_enabled = |name: &str| {
        let menu = MenuState {
            screen: Screen::Save,
            save_name: name.into(),
            ..MenuState::default()
        };
        save_load(&menu, &Texts::default(), [1024.0, 768.0], true, None)
            .widgets
            .iter()
            .any(|w| {
                matches!(
                    w,
                    Widget::BarButton {
                        action: Action::Menu(MenuAction::WriteSave),
                        enabled: true,
                        ..
                    }
                )
            })
    };
    assert!(save_enabled("my game"));
    assert!(!save_enabled(""));
}

/// UI-MENU-5.1 (T6): while the game is written the dialog shows `!Saving...`
/// at (50, 50) and no control; (T5) with the drive full `!Disk Full!` is the
/// caption of the disabled `!Save` button and `!Delete` is disabled. Under 5 MB is full; an unread
/// amount is not.
// spec: UI-MENU-5.1 (T5, T6)
#[test]
fn saving_shows_alone_and_a_full_drive_disables_save_and_delete() {
    let texts = Texts::default();
    let menu = MenuState {
        screen: Screen::Save,
        save_name: "my game".into(),
        saves: vec![SaveEntry {
            name: "my game".into(),
            ours: true,
        }],
        save_choice: 0,
        ..MenuState::default()
    };
    let enabled = |p: &Panel, wanted: MenuAction| {
        p.widgets.iter().any(|w| {
            matches!(w, Widget::BarButton { action: Action::Menu(a), enabled: true, .. } if *a == wanted)
        })
    };
    let shows = |p: &Panel, key: &str| {
        p.widgets
            .iter()
            .any(|w| matches!(w, Widget::Text { text, .. } if text.as_str() == texts.get(key)))
    };
    let normal = save_load(&menu, &texts, [1024.0, 768.0], true, None);
    assert!(enabled(&normal, MenuAction::WriteSave));
    assert!(enabled(&normal, MenuAction::AskDelete));
    assert!(!shows(&normal, "!Disk Full!"));
    let saving = MenuState {
        saving: true,
        ..menu.clone()
    };
    let p = save_load(&saving, &texts, [1024.0, 768.0], true, None);
    assert!(shows(&p, "!Saving..."));
    assert!(!p.widgets.iter().any(|w| matches!(
        w,
        Widget::BarButton { .. } | Widget::TextButton { .. } | Widget::Button { .. }
    )));
    assert!(!shows(&p, "my game"), "the name input is hidden");
    let full = MenuState {
        disk_full: true,
        ..menu.clone()
    };
    let p = save_load(&full, &texts, [1024.0, 768.0], true, None);
    // The caption of the disabled `!Save` button, at its place (148, 344).
    let o = save_load(&menu, &texts, [1024.0, 768.0], true, None)
        .widgets
        .iter()
        .find_map(|w| match w {
            Widget::BarButton {
                action: Action::Menu(MenuAction::WriteSave),
                at,
                ..
            } => Some(*at),
            _ => None,
        })
        .expect("the save button");
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::BarButton { text, action: Action::Menu(MenuAction::WriteSave), enabled: false, at, .. }
            if text.as_str() == texts.get("!Disk Full!") && *at == o
    )));
    assert!(!shows(&p, "!Disk Full!"), "no text of its own");
    assert!(!enabled(&p, MenuAction::WriteSave));
    assert!(!enabled(&p, MenuAction::AskDelete));
    assert!(disk_full(Some(3 << 20)), "T5: 3 MB free");
    assert!(disk_full(Some((5 << 20) - 1)));
    assert!(!disk_full(Some(5 << 20)));
    assert!(!disk_full(None));
}

/// UI-MENU-5.1, 5.3: the save name's restricted character mode: space, digits,
/// letters and `äÄöÖüÜß!&()+-_çâáàôéèê` pass; what a file name cannot hold is never
/// typed; no length limit. Nothing else is done to the name.
// spec: UI-MENU-5.1, UI-MENU-5.3
#[test]
fn the_save_name_takes_only_the_restricted_characters() {
    use crate::ui::widgets::{edit, Edit};
    use bevy::input::keyboard::Key;
    let typed = |text: &str| {
        let (mut name, mut caret) = (String::new(), 0);
        for c in text.chars() {
            edit(
                &mut name,
                &mut caret,
                &Key::Character(c.to_string().into()),
                None,
                true,
            );
        }
        name
    };
    assert_eq!(typed("Köln (2) - Brücke_1!"), "Köln (2) - Brücke_1!");
    assert_eq!(typed("a:b/c?d*e\\f\"g<h>i|j.k,l"), "abcdefghijkl");
    assert_eq!(typed(&"x".repeat(300)).len(), 300);
    let (mut name, mut caret) = (String::new(), 0);
    assert_eq!(
        edit(
            &mut name,
            &mut caret,
            &Key::Character("/".into()),
            None,
            true
        ),
        Edit::Ignored
    );
    // Without the mode the same key is typed (a ship's name).
    assert_eq!(
        edit(
            &mut name,
            &mut caret,
            &Key::Character("/".into()),
            None,
            false
        ),
        Edit::Typed
    );
}

/// UI-MENU-5.1: saving, `!Delete` is enabled only while the input equals the row
/// chosen.
// spec: UI-MENU-5.1
#[test]
fn delete_only_while_the_input_names_the_row() {
    let enabled = |name: &str, choice: usize| {
        let menu = MenuState {
            screen: Screen::Save,
            saves: vec![SaveEntry {
                name: "mine".into(),
                ours: true,
            }],
            save_choice: choice,
            save_name: name.into(),
            ..MenuState::default()
        };
        save_load(&menu, &Texts::default(), [1024.0, 768.0], true, None)
            .widgets
            .iter()
            .any(|w| {
                matches!(
                    w,
                    Widget::BarButton {
                        action: Action::Menu(MenuAction::AskDelete),
                        enabled: true,
                        ..
                    }
                )
            })
    };
    assert!(enabled("mine", 0));
    assert!(!enabled("mine2", 0));
    assert!(!enabled("mine", usize::MAX));
}

/// UI-MENU-3.3: the game menu's two exits open the leave dialog: its question, `!Yes`
/// and `!No` 5 px higher while a game runs, the `!Autosave...` box only then, off by
/// default (setting 22).
// spec: UI-MENU-3.3, FMT-SAV-5.3, SET-2
#[test]
fn the_leave_dialog() {
    let texts = Texts::default();
    let exits: Vec<_> = game_menu(&texts, [1024.0, 768.0])
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton {
                action: Action::Menu(m @ MenuAction::AskLeave(_)),
                ..
            } => Some(m.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        exits,
        [MenuAction::AskLeave(false), MenuAction::AskLeave(true)]
    );
    assert!(!MenuState::default().settings.autosave_on_leave);
    let place = |p: &Panel, wanted: MenuAction| {
        p.widgets.iter().find_map(|w| match w {
            Widget::BarButton {
                at,
                action: Action::Menu(m),
                ..
            } if *m == wanted => Some(*at),
            _ => None,
        })
    };
    let has_box = |p: &Panel| {
        p.buttons()
            .any(|(a, ..)| *a == Action::Menu(MenuAction::AutosaveOnLeave))
            || p.widgets.iter().any(|w| {
                matches!(
                    w,
                    Widget::TextButton {
                        action: Action::Menu(MenuAction::AutosaveOnLeave),
                        ..
                    }
                )
            })
    };
    let in_game = MenuState {
        in_game: true,
        ..MenuState::default()
    };
    let p = leave_dialog(&in_game, &texts, [1024.0, 768.0], false, true);
    let yes = place(&p, MenuAction::ExitToTitle).expect("Yes");
    let no = place(&p, MenuAction::StayInGame).expect("No");
    assert!(has_box(&p));
    let idle = leave_dialog(&MenuState::default(), &texts, [1024.0, 768.0], true, false);
    let yes2 = place(&idle, MenuAction::Exit).expect("Yes, to the desktop");
    assert_eq!(yes2[1] - yes[1], 5.0);
    assert_eq!(no[1] - yes[1], 34.0);
    assert!(!has_box(&idle));
}

/// UI-MENU T17 (3.3a): with no game, the main menu's `!EXIT` opens the 480 x 224
/// `uiMediumDialog` dialog `!Request!` over the main menu, centred, with `!Exit to
/// desktop?`, `!Yes` (130, 110), `!No` (130, 144) and no `!Autosave...` box; `!No`
/// closes it and the main menu stays; `!EXIT` again closes it too; `!Yes` ends the
/// program. In a game the box is the 28 x 28 check box, off (132, 100) and on (164,
/// 100), its label 32 px right and 4 px lower; `!No` and the close button close only
/// the dialog, the game menu not coming back. The question is centred on the
/// frame's middle (3.3a e, seen in the original's picture).
// spec: UI-MENU-3.3a (T17), UI-MENU-3.3
#[test]
fn t17_exit_from_the_main_menu_asks_first() {
    let texts = Texts::default();
    let screen = [1024.0, 768.0];
    let main = main_menu(&texts, screen, &[], None);
    assert!(main.widgets.iter().any(|w| matches!(
        w,
        Widget::TextButton {
            action: Action::Menu(MenuAction::ExitAsked),
            ..
        }
    ) || matches!(
        w,
        Widget::BarButton {
            action: Action::Menu(MenuAction::ExitAsked),
            ..
        }
    )));
    let open = leave_screen(&MenuAction::ExitAsked, Screen::Main).expect("opens");
    assert_eq!(
        open,
        Screen::Leave {
            desktop: true,
            over_main: true
        }
    );
    let menu = MenuState {
        screen: open,
        ..MenuState::default()
    };
    let shown = panels(&menu, &texts, screen, None).expect("panels");
    assert_eq!(shown.len(), 2, "the main menu and the dialog over it");
    let d = &shown[1];
    assert_eq!(d.at, [272.0, 272.0]);
    assert!(d.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { atlas: Atlas::MediumDialog, rect: [8, 8, 480, 224], at, .. } if *at == [0.0, 0.0]
    )));
    let bar = |p: &Panel, wanted: MenuAction| {
        p.widgets.iter().find_map(|w| match w {
            Widget::BarButton {
                at,
                action: Action::Menu(m),
                ..
            } if *m == wanted => Some(*at),
            _ => None,
        })
    };
    assert_eq!(bar(d, MenuAction::Exit), Some([130.0, 110.0]));
    assert_eq!(bar(d, MenuAction::StayInGame), Some([130.0, 144.0]));
    // 3.3a (e): the question centred on the frame's middle, on y 70.
    assert!(d.widgets.iter().any(|w| matches!(
        w,
        Widget::Text { text, at, align: crate::ui::widgets::Align::TopCentre, .. }
            if text.contains("Exit to desktop") && *at == [240.0, 70.0]
    )));
    assert!(!d
        .buttons()
        .any(|(a, ..)| *a == Action::Menu(MenuAction::AutosaveOnLeave)));
    assert_eq!(
        leave_screen(&MenuAction::StayInGame, open),
        Some(Screen::Main)
    );
    assert_eq!(
        leave_screen(&MenuAction::ExitAsked, open),
        Some(Screen::Main)
    );
    // In a game: the box, and No back to the game.
    let in_game = leave_screen(&MenuAction::AskLeave(false), Screen::GameMenu).unwrap();
    assert_eq!(
        leave_screen(&MenuAction::StayInGame, in_game),
        Some(Screen::None)
    );
    let mut menu = MenuState {
        screen: in_game,
        in_game: true,
        ..MenuState::default()
    };
    let check = |menu: &MenuState| {
        let panels = panels(menu, &texts, screen, None).unwrap();
        let d = &panels[0];
        let boxes: Vec<_> = d
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Button {
                    rect,
                    at,
                    action: Action::Menu(MenuAction::AutosaveOnLeave),
                    ..
                } => Some((*rect, *at)),
                _ => None,
            })
            .collect();
        let label = d.widgets.iter().any(|w| {
            matches!(w, Widget::Text { at, font, .. } if *at == [162.0, 179.0] && *font == Font::Body)
        });
        (boxes, label)
    };
    assert_eq!(
        check(&menu),
        (vec![([132, 100, 28, 28], [130.0, 175.0])], true)
    );
    menu.settings.autosave_on_leave = true;
    assert_eq!(check(&menu).0, [([164, 100, 28, 28], [130.0, 175.0])]);
}

/// UI-MENU-5.1, 6.1: `!Delete` is enabled with a save chosen and asks
/// `!Remove this file?` at (80, 319) with Yes (148, 339) and No (148,
/// 364), in the place of the name input and the `!Load` button.
// spec: UI-MENU-5.1, UI-MENU-6.1
#[test]
fn delete_asks_first() {
    let o = [324.0, 152.0];
    let mut menu = MenuState {
        screen: Screen::Load { in_game: false },
        saves: vec![SaveEntry {
            name: "mine".into(),
            ours: true,
        }],
        save_choice: usize::MAX,
        ..MenuState::default()
    };
    let texts = Texts::default();
    let button = |p: &Panel, wanted: MenuAction| {
        p.widgets.iter().find_map(|w| match w {
            Widget::BarButton {
                action: Action::Menu(a),
                at,
                enabled,
                ..
            } if *a == wanted => Some((*at, *enabled)),
            _ => None,
        })
    };
    let p = save_load(&menu, &texts, [1024.0, 768.0], false, None);
    assert_eq!(button(&p, MenuAction::AskDelete).map(|b| b.1), Some(false));
    assert_eq!(button(&p, MenuAction::LoadSave).map(|b| b.1), Some(false));
    menu.save_choice = 0;
    let p = save_load(&menu, &texts, [1024.0, 768.0], false, None);
    assert_eq!(
        button(&p, MenuAction::AskDelete),
        Some(([o[0] + 148.0, o[1] + 409.0], true))
    );
    assert_eq!(
        button(&p, MenuAction::LoadSave),
        Some(([o[0] + 148.0, o[1] + 344.0], true))
    );
    menu.delete_ask = true;
    let p = save_load(&menu, &texts, [1024.0, 768.0], false, None);
    assert_eq!(button(&p, MenuAction::LoadSave), None);
    assert_eq!(
        button(&p, MenuAction::Delete(true)),
        Some(([o[0] + 148.0, o[1] + 339.0], true))
    );
    assert_eq!(
        button(&p, MenuAction::Delete(false)),
        Some(([o[0] + 148.0, o[1] + 364.0], true))
    );
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Text { text, at, .. } if text.contains("Remove this file?")
            && *at == [o[0] + 80.0, o[1] + 319.0]
    )));
}

/// Q-298 c: more saves than the 11 rows: arrows right of the first and
/// last row move the list a row; a row chooses its own save.
// spec: UI-MENU-5.1, UI-MENU-6.1
#[test]
fn a_long_save_list_scrolls() {
    let mut menu = MenuState {
        screen: Screen::Load { in_game: false },
        saves: (0..14)
            .map(|n| SaveEntry {
                name: format!("save {n}"),
                ours: true,
            })
            .collect(),
        ..MenuState::default()
    };
    let rows = |menu: &MenuState| -> Vec<(String, MenuAction)> {
        save_load(menu, &Texts::default(), [1024.0, 768.0], false, None)
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::TextButton {
                    text,
                    action: Action::Menu(a @ MenuAction::SelectSave(_)),
                    ..
                } => Some((text.clone(), a.clone())),
                _ => None,
            })
            .collect()
    };
    assert_eq!(rows(&menu).len(), SAVE_ROWS);
    assert_eq!(rows(&menu)[0].0, "save 0");
    // Q-1042: the save list's bar as Select Map's: 14 saves, a step of 1 / 7.
    for _ in 0..7 {
        menu.save_bar = bar_step(menu.save_bar, menu.saves.len(), 1);
        menu.save_scroll = bar_first(menu.save_bar, menu.saves.len());
    }
    let shown = rows(&menu);
    assert_eq!(menu.save_scroll, 3);
    assert_eq!(shown[0], ("save 3".to_owned(), MenuAction::SelectSave(3)));
    assert_eq!(shown[10].0, "save 13");
    let arrows = save_load(&menu, &Texts::default(), [1024.0, 768.0], false, None)
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                action: Action::Menu(MenuAction::SaveScroll(d)),
                enabled,
                ..
            } => Some((*d, *enabled)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(arrows, [(-1, true), (1, false)]);
}

/// SET-5.3, SET-4a, UI-MENU-4.0: the engine page on the settings frame:
/// `GRAPHIC QUALITY` a drop-down showing the profile's preset at (168, 72),
/// its list the six presets; the seven scene sliders, `SHADERDETAIL` a
/// drop-down (Low, High) and `TEXTURE QUALITY` on rows 32 px apart; the six
/// tabs at (15, 25) 40 px apart with the engine tab selected; no `!Reset`.
/// The sliders move on their steps.
// spec: SET-5.3, SET-4a, UI-MENU-4.0
#[test]
fn the_engine_page() {
    let texts = Texts::parse(
        "!GRAPHIC QUALITY\tGRAPHIC QUALITY\n!SHADOW QUALITY\tSHADOW QUALITY\n\
         VeryLow\tVery low\nVeryHigh\tVery high\nCustom\tCustom\n!Low\tLow\n!High\tHigh\n\
         !Game settings\tGame settings\n!Engine Settings\tEngine Settings\n",
    );
    let mut menu = MenuState::default();
    let p = settings_screen(&menu, &texts, [1024.0, 768.0], 2);
    let o = [324.0, 152.0];
    let bars: Vec<(String, [f32; 2], Action, bool)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::BarButton {
                text,
                at,
                action,
                enabled,
                ..
            } => Some((
                text.clone(),
                [at[0] - o[0], at[1] - o[1]],
                action.clone(),
                *enabled,
            )),
            _ => None,
        })
        .collect();
    // The default profile is VeryHigh.
    assert!(bars.contains(&(
        "Very high".into(),
        [168.0, 72.0],
        Action::Menu(MenuAction::OpenDrop(Drop::Preset)),
        true
    )));
    assert_eq!(
        drop_entries(&menu, &texts, Drop::Preset)[..2],
        ["Very low".to_owned(), "Low".to_owned()]
    );
    assert!(bars
        .iter()
        .any(|b| b.1 == [168.0, 328.0]
            && b.2 == Action::Menu(MenuAction::OpenDrop(Drop::ShaderDetail))));
    // UI-MENU-4.0: no `!Reset` on the engine page.
    assert!(!bars
        .iter()
        .any(|b| matches!(b.2, Action::Menu(MenuAction::SettingsReset(_)))));
    let sliders: Vec<(SliderKind, f32)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Slider { kind, at, .. } => Some((*kind, at[1] - o[1])),
            _ => None,
        })
        .collect();
    let fields: Vec<SliderKind> = sliders.iter().map(|s| s.0).collect();
    assert_eq!(
        fields,
        [7, 11, 12, 14, 15, 17, 18, 3]
            .map(SliderKind::Scene)
            .to_vec()
    );
    assert_eq!(sliders[0].1, 104.0 + 6.0);
    assert_eq!(sliders[7].1, 360.0 + 6.0);
    // The tabs: the engine page's (id 1, the second) selected.
    assert!(sprites(&p).contains(&(
        Atlas::Elements0,
        [48, 480, 40, 27],
        [o[0] + 55.0, o[1] + 25.0]
    )));
    assert!(sprites(&p).contains(&(Atlas::Depot, [8, 8, 376, 464], o)));
    // The open drop-down's list: the presets 19 px apart below its button.
    menu.drop = Some(Drop::Preset);
    let p = settings_screen(&menu, &texts, [1024.0, 768.0], 2);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::BarButton { action: Action::Menu(MenuAction::Choose(Drop::Preset, 5)), at, .. }
            if *at == [o[0] + 168.0, o[1] + 72.0 + 19.0 * 6.0]
    )));
    // SET-4: the sliders' steps.
    use s2_assets::cfg::field;
    assert_eq!(scene_slider_step(field::SHADOW_QUALITY, 0.3), 0.25);
    assert_eq!(scene_slider_step(field::SHADOW_DETAIL, 0.2), 1.0 / 6.0);
    assert_eq!(scene_slider_step(field::OBJECT_DETAILS, 0.6), 2.0 / 3.0);
    assert_eq!(scene_slider_step(field::TERRAIN_QUALITY, 0.3), 0.3);
}

/// UI-MENU-4.0, 4.1 (Q-1047, E-ui-034): the game page: AUTOSAVE, Infowindow
/// and DayNightCycle are drop-downs at (168, 72 / 104 / 136), their captions at
/// x 24; Infowindow lists On first; no BACK and no page button; `!Reset`
/// at (168, 408) puts the autosave at 10 minutes, Infowindow Off, the
/// autosave on leaving off and keeps DayNightCycle.
// spec: UI-MENU-4.0, UI-MENU-4.1, SET-5.6
#[test]
fn the_game_settings_page() {
    let texts = Texts::parse("!On\tOn\n!Off\tOff\n!10 Minutes\t10 Minutes\n");
    let mut menu = MenuState::default();
    let o = [324.0, 152.0];
    let p = settings_screen(&menu, &texts, [1024.0, 768.0], 0);
    let drop_at = |p: &Panel, drop: Drop| {
        p.widgets.iter().find_map(|w| match w {
            Widget::BarButton { action, at, .. }
                if *action == Action::Menu(MenuAction::OpenDrop(drop)) =>
            {
                Some([at[0] - o[0], at[1] - o[1]])
            }
            _ => None,
        })
    };
    assert_eq!(drop_at(&p, Drop::Autosave), Some([168.0, 72.0]));
    assert_eq!(drop_at(&p, Drop::InfoWindow), Some([168.0, 104.0]));
    assert_eq!(drop_at(&p, Drop::DayNight), Some([168.0, 136.0]));
    assert_eq!(drop_entries(&menu, &texts, Drop::InfoWindow), ["On", "Off"]);
    assert!(!p.widgets.iter().any(|w| matches!(
        w,
        Widget::BarButton {
            action: Action::Menu(MenuAction::Back | MenuAction::SettingsPage(_)),
            ..
        }
    )));
    menu.settings.day_night = false;
    menu.settings.autosave_on_leave = true;
    menu.settings.autosave = 3600;
    assert_eq!(drop_value(&menu, Drop::Autosave), 6);
    menu.settings.autosave = 60;
    menu.settings.info_window = true;
    assert_eq!(drop_value(&menu, Drop::InfoWindow), 0);
    assert!(reset_settings_page(&mut menu.settings, 0));
    assert!(!reset_settings_page(&mut menu.settings, 2));
    assert_eq!(
        (
            menu.settings.autosave,
            menu.settings.info_window,
            menu.settings.autosave_on_leave,
            menu.settings.day_night
        ),
        (600, false, false, false)
    );
}

/// UI-SETUP-1.8 (Q-211 b, T16): a slot's portrait, emblem and colour cell by
/// its tribe, level (human 2, a computer its strength, open 0) and colour.
// spec: UI-SETUP-1.8 (T16)
#[test]
fn t16_slot_pictures_by_level() {
    let slot = |kind, strength, tribe| Slot {
        kind,
        strength,
        tribe,
        start: -1,
        team: -1,
        colour: -1,
    };
    // A: human, Nubians, colour 3.
    let a = slot(SlotKind::Human, 0, 1);
    assert_eq!(portrait(1, slot_level(&a)), [296, 512, 46, 80]);
    assert_eq!(emblem(slot_level(&a)), [616, 8, 48, 48]);
    assert_eq!(colour_square(3), [544, 240, 24, 24]);
    // B: computer, Chinese, strength 1, colour 6.
    let b = slot(SlotKind::Computer, 1, 2);
    assert_eq!(portrait(2, slot_level(&b)), [408, 512, 46, 80]);
    assert_eq!(emblem(slot_level(&b)), [568, 8, 48, 48]);
    assert_eq!(colour_square(6), [520, 288, 24, 24]);
    // C: open (ours has closed slots only; level 0 too), Romans, colour 0.
    let c = slot(SlotKind::Closed, 2, 0);
    assert_eq!(portrait(0, slot_level(&c)), [16, 512, 46, 80]);
    assert_eq!(emblem(slot_level(&c)), [520, 8, 48, 48]);
    assert_eq!(colour_square(0), [520, 216, 24, 24]);
}

/// UI-SETUP-1.8 'What a click on them does' (Q-211, T18): a computer slot
/// at strength 2 (Strong), Chinese, colour 7: its emblem, then its
/// portrait, then its colour square clicked on the free play page: each
/// value steps and wraps, and the page then shows the stepped pictures.
// spec: UI-SETUP-1.8 (T18)
#[test]
fn t18_slot_clicks_step_and_wrap() {
    let mut menu = free_play_menu();
    menu.slots[1] = Slot {
        kind: SlotKind::Computer,
        strength: 2,
        tribe: 2,
        start: -1,
        team: -1,
        colour: 7,
    };
    let texts = Texts::default();
    let actions = |menu: &MenuState| -> Vec<(MenuAction, [u32; 4])> {
        free_play(menu, &texts, [1024.0, 768.0])
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Button {
                    rect,
                    action: Action::Menu(a),
                    ..
                } => Some((a.clone(), *rect)),
                _ => None,
            })
            .collect()
    };
    let shown = |menu: &MenuState, a: MenuAction| {
        actions(menu)
            .into_iter()
            .find(|(b, _)| *b == a)
            .map(|(_, rect)| rect)
    };
    // Before: the Strong emblem, the Chinese computer's portrait at its
    // level, colour 7's square.
    assert_eq!(shown(&menu, MenuAction::SlotStrength(1)), Some(emblem(2)));
    assert_eq!(
        shown(&menu, MenuAction::SlotColour(1)),
        Some(colour_square(7))
    );
    // The emblem: strength 2 -> 0, emblem (520, 8).
    assert!(slot_click(&mut menu.slots, &MenuAction::SlotStrength(1)));
    assert_eq!(menu.slots[1].strength, 0);
    assert_eq!(
        shown(&menu, MenuAction::SlotStrength(1)),
        Some([520, 8, 48, 48])
    );
    // The portrait: tribe 2 -> 0 (Romans), portrait (16 + 0 + 0, 512).
    assert!(slot_click(&mut menu.slots, &MenuAction::SlotTribe(1)));
    assert_eq!(menu.slots[1].tribe, 0);
    assert_eq!(
        shown(&menu, MenuAction::SlotTribe(1)),
        Some([16, 512, 46, 80])
    );
    // The colour square: colour 7 -> 0, square (520, 216).
    assert!(slot_click(&mut menu.slots, &MenuAction::SlotColour(1)));
    assert_eq!(menu.slots[1].colour, 0);
    assert_eq!(
        shown(&menu, MenuAction::SlotColour(1)),
        Some([520, 216, 24, 24])
    );
    // The game gets the slot's colour, and the start marker shows it.
    let players = setup_players(&menu.slots, 2);
    assert_eq!(players[1].colour, 0);
    assert_eq!(players[0].colour, 0);
    let summary = menu.summary.clone().unwrap();
    let markers = start_markers(&summary, &menu.slots);
    assert_eq!(markers[1].2, markers[0].2);
    // Each value goes on round: colour 0 .. 7 and back, strength and tribe
    // in threes.
    for c in 1..=8 {
        slot_click(&mut menu.slots, &MenuAction::SlotColour(1));
        assert_eq!(menu.slots[1].colour, c % 8);
    }
    for n in 1..=3u8 {
        slot_click(&mut menu.slots, &MenuAction::SlotStrength(1));
        slot_click(&mut menu.slots, &MenuAction::SlotTribe(1));
        assert_eq!(menu.slots[1].strength, n % 3);
        assert_eq!(menu.slots[1].tribe, (n % 3) as i8);
    }
    assert!(!slot_click(&mut menu.slots, &MenuAction::StartGame));
}

/// UI-MENU-2.1 "How the bar works" (Q-1042 c): while the bar is pressed
/// (track or thumb) each update moves v three quarters of the way to the
/// pointer's height / 194, so a click moves the thumb toward the pointer
/// and a drag follows it; with everything shown v stays 0.5. (b) An arrow
/// held steps at once, then again in the first update in which the 0.05 s
/// countdown is no longer above 0.
// spec: UI-MENU-2.1
#[test]
fn the_pressed_bar_chases_the_pointer_and_a_held_arrow_repeats() {
    let mut v = 0.0;
    v = bar_chase(v, 20, 97);
    assert!((v - 0.375).abs() < 1e-6, "{v}");
    for _ in 0..20 {
        v = bar_chase(v, 20, 97);
    }
    assert!((v - 0.5).abs() < 1e-4, "it settles under the pointer: {v}");
    assert_eq!(bar_first(v, 20), 5);
    // Beyond the bar's ends: kept in 0..1.
    assert_eq!(bar_chase(0.9, 20, 400), 1.0);
    assert_eq!(bar_chase(0.1, 20, -50), 0.0);
    // Everything fits: the middle.
    assert_eq!(bar_chase(0.0, 8, 0), 0.5);
    // The repeat: 60 updates a second step every third update (0.05 s).
    let mut countdown = BAR_REPEAT;
    let mut steps = Vec::new();
    for frame in 1..=12 {
        let (due, left) = bar_repeat(countdown, 1.0 / 60.0);
        countdown = left;
        if due {
            steps.push(frame);
        }
    }
    assert_eq!(steps, [3, 6, 9, 12]);
    // A slow frame of 0.2 s gives one step, not four.
    assert_eq!(bar_repeat(BAR_REPEAT, 0.2), (true, BAR_REPEAT));
    // Select Map's track: a press sends the pointer's offset; the list follows.
    let mut menu = MenuState {
        maps: (0..20)
            .map(|i| PathBuf::from(format!("m{i}.s2m")))
            .collect(),
        screen: Screen::SelectMap,
        ..MenuState::default()
    };
    for _ in 0..30 {
        select_map_apply(&mut menu, &MenuAction::MapTrack(194));
    }
    assert_eq!((menu.map_bar, menu.map_scroll), (1.0, 9));
}

/// SET-5.2, 5.5, 5.7 (E-ui-035, T7): the Graphics, Input and Network pages.
/// Graphics: RESOLUTION at (168, 72) showing the mode `<w> x <h>`, MODE at
/// (168, 104), no Reset. Input: SCROLL SPEED a slider at (168, 72) with its
/// caption 12 px lower than a drop-down's, then MOUSE SCROLLING, BORDER
/// SCROLLING, SMART CURSOR, FREE CAMERA at 104, 136, 168, 200 bound to
/// fields 6, 8, 9, 21, Reset shown (6 off, 7 = 0.25, 8 on, 9 on, 21 off).
/// Network: CALLSIGN with the player's name at (168, 72), INTERNET SPEED
/// at (168, 104) NONE to LAN showing NONE, Reset shown. Every tab is on.
// spec: SET-5.2, SET-5.5, SET-5.7, UI-MENU-4.0, UI-MENU-4.3
#[test]
fn the_graphics_input_and_network_pages() {
    crate::menus::settings::hold_files();
    let texts = Texts::parse(
        "!Inverted\tInverted\n!Not Inverted\tNot inverted\n!Activated\tEnabled\n\
         !Not Activated\tDisabled\n!Windowed\tWindowed\n!NONE\tNONE\n",
    );
    let mut menu = MenuState::default();
    let o = [324.0, 152.0];
    let bar_at = |p: &Panel, action: MenuAction| {
        let action = Action::Menu(action);
        p.widgets.iter().find_map(|w| match w {
            Widget::BarButton {
                action: a,
                at,
                text,
                ..
            } if *a == action => Some(([at[0] - o[0], at[1] - o[1]], text.clone())),
            _ => None,
        })
    };
    let reset = |p: &Panel| {
        p.widgets
            .iter()
            .any(|w| format!("{w:?}").contains("SettingsReset"))
    };
    // Every tab opens its page.
    let tabs = settings_screen(&menu, &texts, [1024.0, 768.0], 3);
    let on = tabs
        .widgets
        .iter()
        .filter(|w| {
            matches!(
                w,
                Widget::Button {
                    action: Action::Menu(MenuAction::SettingsPage(_)),
                    enabled: true,
                    ..
                }
            )
        })
        .count();
    assert_eq!(on, 6);
    // Graphics.
    let p = settings_screen(&menu, &texts, [1024.0, 768.0], 3);
    assert_eq!(
        bar_at(&p, MenuAction::OpenDrop(Drop::Resolution)),
        Some(([168.0, 72.0], "1024 x 768".to_string()))
    );
    assert_eq!(
        bar_at(&p, MenuAction::OpenDrop(Drop::DisplayMode)),
        Some(([168.0, 104.0], "Windowed".to_string()))
    );
    assert!(!reset(&p));
    // Input.
    menu.settings.cfg.user.input_b = false;
    menu.settings.cfg.user.input_c = true;
    menu.settings.cfg.user.input_d = true;
    menu.settings.cfg.user.unknown_bools[1] = false;
    let p = settings_screen(&menu, &texts, [1024.0, 768.0], 4);
    let slider = p.widgets.iter().find_map(|w| match w {
        Widget::Slider {
            at,
            kind: SliderKind::ScrollSpeed,
            value,
            ..
        } => Some((at[0] - o[0], *value)),
        _ => None,
    });
    assert_eq!(slider, Some((168.0, 0.25)));
    for (drop, y, shown) in [
        (Drop::MouseScrolling, 104.0, "Not inverted"),
        (Drop::BorderScrolling, 136.0, "Enabled"),
        (Drop::SmartCursor, 168.0, "Enabled"),
        (Drop::FreeCamera, 200.0, "Not inverted"),
    ] {
        assert_eq!(
            bar_at(&p, MenuAction::OpenDrop(drop)),
            Some(([168.0, y], shown.to_string())),
            "{drop:?}"
        );
    }
    assert!(reset(&p));
    let caption = |p: &Panel, text: &str| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Text { text: t, at, .. } if t == text => Some([at[0] - o[0], at[1] - o[1]]),
            _ => None,
        })
    };
    let speed = caption(&p, "!SCROLL SPEED").unwrap();
    let mouse = caption(&p, "!MOUSE SCROLLING").unwrap();
    assert_eq!(speed[0], 24.0);
    assert_eq!(speed[1] - 72.0, mouse[1] - 104.0 + 12.0);
    choose(&mut menu, Drop::MouseScrolling, 0);
    choose(&mut menu, Drop::SmartCursor, 1);
    choose(&mut menu, Drop::FreeCamera, 0);
    let u = &menu.settings.cfg.user;
    assert_eq!(
        (u.input_b, u.input_d, u.unknown_bools[1]),
        (true, false, true)
    );
    menu.set_scroll_speed(0.8);
    assert!(reset_settings_page(&mut menu.settings, 4));
    let u = &menu.settings.cfg.user;
    assert_eq!(
        (
            u.input_b,
            u.scroll_speed,
            u.input_c,
            u.input_d,
            u.unknown_bools[1]
        ),
        (false, 0.25, true, true, false)
    );
    // Network.
    let p = settings_screen(&menu, &texts, [1024.0, 768.0], 5);
    assert_eq!(
        caption(&p, &menu.settings.cfg.user.player_name),
        Some([168.0, 72.0])
    );
    assert_eq!(
        bar_at(&p, MenuAction::OpenDrop(Drop::InternetSpeed)),
        Some(([168.0, 104.0], "NONE".to_string()))
    );
    assert_eq!(drop_entries(&menu, &texts, Drop::InternetSpeed).len(), 5);
    assert!(reset(&p));
    assert!(!reset_settings_page(&mut menu.settings, 5));
}

/// UI-SETUP-1.8 (E-ui-035): START's tooltip after the human slot's row is
/// filled: "Ready to start" while the start positions are distinct, "Double
/// allocation of HQ" while two slots share one (and START is off).
// spec: UI-SETUP-1.8 (T20)
#[test]
fn starts_tooltip_tells_ready_or_a_shared_start() {
    let texts = Texts::parse(
        "!READY_CHECK_TOOLTIP\tReady to start\n\
         !READY_CHECK_UNUSABLE_TOOLTIP\tDouble allocation of HQ\n\
         !START_BUTTON_TOOLTIP\tStarts the game\n",
    );
    let tip = |menu: &MenuState| {
        let p = free_play(menu, &texts, [1024.0, 768.0]);
        p.tooltips
            .iter()
            .find(|(a, _)| *a == Action::Menu(MenuAction::StartGame))
            .map(|(_, t)| t.clone())
    };
    let mut menu = free_play_menu();
    assert_eq!(tip(&menu).as_deref(), Some("Ready to start"));
    choose(&mut menu, Drop::Hq(1), 0);
    assert_eq!(tip(&menu).as_deref(), Some("Double allocation of HQ"));
}
