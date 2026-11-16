use super::*;
use crate::ui::widgets::Align;

fn flat_world(width: u32, height: u32) -> World {
    let n = (width * height) as usize;
    let terrain = s2_sim::map::Terrain {
        width,
        height,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![0; n],
        samples: Vec::new(),
    };
    World::with_map(1, terrain, s2_sim::Rules::empty(), PlayerId(0))
}

static NO_NAMES: std::collections::BTreeMap<usize, String> = std::collections::BTreeMap::new();

fn ctx<'a>(world: &'a World, texts: &'a Texts, catalog: &'a Catalog) -> Ctx<'a> {
    Ctx {
        world,
        player: PlayerId(0),
        screen: [1024.0, 768.0],
        texts,
        catalog,
        click: None,
        tutorial: false,
        speed: 1,
        real_seconds: 0.0,
        posted_messages: &[],
        ship_names: &NO_NAMES,
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
    }
}

fn message(tick: u64, text: &'static str) -> s2_sim::Message {
    s2_sim::Message {
        tick,
        player: PlayerId(0),
        at: (3, 3),
        text,
        until: tick + 36_000,
        building: Some(1),
        ship: None,
    }
}

/// UI-GAME T33, T34 (11.1, 11.3): two misc rows and a military row,
/// newest last; the military filter off leaves the two misc rows; a
/// deleted message and one past its time are not listed.
// spec: UI-GAME-8.3, UI-GAME-11.3 (T33, T34)
#[test]
fn t33_t34_message_rows() {
    let all = [
        message(10, "Building constructed"),
        message(20, "Building constructed"),
        message(30, "Military building occupied!"),
        message(40, "Building without worker."),
    ];
    let kinds: Vec<u8> = all.iter().map(|m| message_kind(m.text)).collect();
    assert_eq!(kinds, [4, 4, 1, 3]);
    let none = std::collections::HashSet::new();
    let all: Vec<&s2_sim::Message> = all.iter().collect();
    let ticks = |l: Vec<&s2_sim::Message>| l.iter().map(|m| m.tick).collect::<Vec<_>>();
    assert_eq!(
        ticks(listed_messages(&all, PlayerId(0), 50, [false; 4], &none)),
        [10, 20, 30, 40]
    );
    let hidden = [true, false, false, false];
    assert_eq!(
        ticks(listed_messages(&all, PlayerId(0), 50, hidden, &none)),
        [10, 20, 40]
    );
    let deleted: std::collections::HashSet<MessageKey> =
        [(20, (3, 3), "Building constructed")].into();
    assert_eq!(
        ticks(listed_messages(&all, PlayerId(0), 50, [false; 4], &deleted)),
        [10, 30, 40]
    );
    // UI-GAME T36: gone once its time is up.
    assert_eq!(
        ticks(listed_messages(
            &all,
            PlayerId(0),
            36_010,
            [false; 4],
            &none
        )),
        [20, 30, 40]
    );
}

/// UI-GAME-11.3: seven rows, newest at the bottom; scrolled up by two,
/// the window ends two before the newest.
#[test]
fn message_window_rows() {
    let listed: Vec<u32> = (0..10).collect();
    assert_eq!(message_window(&listed, 0), [3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(message_window(&listed, 2), [1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(message_window(&listed[..3], 0), [0, 1, 2]);
}

/// UI-GAME T30 to T32 (9.2, 9.3): a site's tooltip names it "under
/// construction", a finished hut's adds its productivity (never below
/// 0), a foreign building shows its name only.
// spec: UI-GAME-9.3 (T31, T32)
#[test]
fn t31_t32_building_tooltips() {
    let under = "(Under construction)";
    assert_eq!(
        building_tooltip("Woodcutter's Hut", Some(under), Some(65)),
        "Woodcutter's Hut (Under construction)"
    );
    assert_eq!(
        building_tooltip("Woodcutter's Hut", None, Some(65)),
        "Woodcutter's Hut 65%"
    );
    assert_eq!(building_tooltip("Barracks", None, None), "Barracks");
    assert_eq!(productivity_text(-3), "0%");
    assert_eq!(productivity_text(65), "65%");
}

/// UI-GAME T32: the flag under the pointer, then a carrier: "Flag",
/// "Carrier" (the keys untranslated here).
// spec: UI-GAME-9.3 (T32)
#[test]
fn t32_flag_and_carrier_tooltips() {
    let world = flat_world(12, 12);
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let c = ctx(&world, &texts, &catalog);
    assert_eq!(map_tooltip(&c, MapPick::Flag).as_deref(), Some("!Flag"));
    // No carrier with that index: no tooltip.
    assert_eq!(map_tooltip(&c, MapPick::Carrier(0)), None);
}

/// UI-GAME-9.3: the tooltip 2 % right of and above the pointer, kept on
/// the screen and 20 px from the top; 32 px high at 768.
#[test]
fn map_tooltip_placement() {
    let screen = [1024.0, 768.0];
    let at = map_tooltip_at([500.0, 400.0], [120.0, 32.0], screen);
    assert!((at[0] - 520.48).abs() < 0.01 && (at[1] - (400.0 - 15.36 - 32.0)).abs() < 0.01);
    assert_eq!(
        map_tooltip_at([1000.0, 10.0], [120.0, 32.0], screen),
        [904.0, 20.0]
    );
    assert_eq!(map_tooltip_height(1080.0), 45.0);
}

/// UI-GAME T2, TRN-ROAD-7.1: an own flag on a Coast field (water next
/// to it), not the headquarters' flag: all five buttons are enabled, the
/// waterway one starts waterway building.
#[test]
fn t2_flag_menu_on_the_coast() {
    let n = 100;
    let mut states = vec![0; n];
    states[4 * 10 + 4] = s2_sim::map::state::COAST;
    states[4 * 10 + 5] = s2_sim::map::state::WATER;
    let terrain = s2_sim::map::Terrain {
        width: 10,
        height: 10,
        heights: vec![0; n],
        height_floor: -100,
        states,
        samples: Vec::new(),
    };
    let mut world = World::with_map(1, terrain, s2_sim::Rules::empty(), PlayerId(0));
    world.submit(Command::PlaceFlag {
        player: PlayerId(0),
        at: (4, 4),
    });
    world.step();
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let p = flag_menu(&ctx(&world, &texts, &catalog), (4, 4));
    let rows: Vec<_> = p
        .buttons()
        .filter(|b| b.1[1] == 29.0)
        .map(|b| b.2)
        .collect();
    assert_eq!(rows, [true; 5]);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::StartWaterway((4, 4)),
            ..
        }
    )));
}

/// UI-GAME-3.2 T5: next to a flag the build menu's flag button is
/// disabled with `!Flag too close to another flag.`; where a flag fits
/// it is enabled with `!Create flag at position.`, with the flag picture
/// over the slot, greyed with the button.
#[test]
fn t5_flag_button_next_to_a_flag() {
    let mut world = flat_world(12, 12);
    world.submit(Command::PlaceFlag {
        player: PlayerId(0),
        at: (4, 4),
    });
    world.step();
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let c = ctx(&world, &texts, &catalog);
    let flag_button = |p: &Panel, at: Point| {
        p.buttons()
            .find(|b| matches!(b.0, Action::Command(Command::PlaceFlag { at: f, .. }) if *f == at))
            .map(|b| (b.2, b.3.unwrap_or("").to_owned()))
            .expect("a flag button")
    };
    let next = build_menu(&c, (5, 4), 5);
    assert_eq!(
        flag_button(&next, (5, 4)),
        (false, "!Flag too close to another flag.".to_owned())
    );
    assert!(next.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite {
            rect: FLAG_ICON,
            greyed: true,
            ..
        }
    )));
    let free = build_menu(&c, (8, 8), 5);
    assert_eq!(
        flag_button(&free, (8, 8)),
        (true, "!Create flag at position.".to_owned())
    );
    assert!(free.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite {
            rect: FLAG_ICON,
            greyed: false,
            ..
        }
    )));
}

/// UI-GAME-3.3 T23: a free field that takes only a flag opens the road
/// menu with the one button "Erect flag"; on a road next to a flag the
/// menu has only "Destroy the street".
#[test]
fn t23_flag_only_spot_opens_the_road_menu() {
    let n = 144;
    let mut states = vec![0; n];
    // A blocked neighbour forbids every building (ECO-PLACE-3.3), not a
    // flag.
    states[6 * 12 + 7] = s2_sim::map::state::BLOCKED;
    let terrain = s2_sim::map::Terrain {
        width: 12,
        height: 12,
        heights: vec![0; n],
        height_floor: -100,
        states,
        samples: Vec::new(),
    };
    let mut world = World::with_map(1, terrain, s2_sim::Rules::empty(), PlayerId(0));
    assert_eq!(world.placement_at((6, 6)), Placement::Flag);
    assert_eq!(
        context_for(&world, PlayerId(0), (6, 6)),
        Some(Context::Road((6, 6)))
    );
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let p = road_menu(&ctx(&world, &texts, &catalog), (6, 6));
    let actions: Vec<_> = p.buttons().map(|b| b.0.clone()).collect();
    assert_eq!(
        actions,
        [
            Action::CloseContext,
            Action::Command(Command::PlaceFlag {
                player: PlayerId(0),
                at: (6, 6)
            })
        ]
    );
    // A road of three fields: its middle is next to both flags.
    for at in [(2, 2), (4, 2)] {
        world.submit(Command::PlaceFlag {
            player: PlayerId(0),
            at,
        });
    }
    world.step();
    world.submit(Command::BuildRoad {
        player: PlayerId(0),
        from: (2, 2),
        to: (4, 2),
    });
    world.step();
    assert!(world.road_at((3, 2)).is_some());
    let p = road_menu(&ctx(&world, &texts, &catalog), (3, 2));
    let actions: Vec<_> = p.buttons().map(|b| b.0.clone()).collect();
    assert_eq!(
        actions,
        [
            Action::CloseContext,
            Action::Command(Command::RemoveRoad {
                player: PlayerId(0),
                at: (3, 2)
            })
        ]
    );
}

/// UI-GAME-8.5a, 8.5g: the player statistics page: the graph image at (18,
/// 129), the separator at (15, 115); the series and the span are drop-down
/// lists: one 152 x 20 button at (192, 412) and (18, 412) showing the chosen
/// choice; a click opens the list, its choices 19 px apart from one step
/// below (or upward near the bottom of the screen); one list at a time.
// spec: UI-GAME-8.5a, UI-GAME-8.5g
#[test]
fn statistics_page() {
    let world = flat_world(10, 10);
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let c = Ctx {
        statistics_span: 2,
        ..ctx(&world, &texts, &catalog)
    };
    let p = statistics_dialog(&c, 3);
    let sprite = |atlas: Atlas, rect: [u32; 4], at: [f32; 2]| {
        p.widgets.iter().any(|w| {
            matches!(w, Widget::Sprite { atlas: a, rect: r, at: t, .. } if *a == atlas && *r == rect && *t == at)
        })
    };
    assert!(sprite(
        Atlas::Elements1,
        [176, 232, 328, 271],
        [18.0, 129.0]
    ));
    assert!(sprite(Atlas::Elements3, [0, 432, 336, 8], [15.0, 115.0]));
    let bars = |p: &Panel| -> Vec<(String, [f32; 2], Action)> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::BarButton {
                    text,
                    at,
                    action,
                    bar,
                    ..
                } if *bar == crate::ui::widgets::SLOT_BAR => {
                    Some((text.clone(), *at, action.clone()))
                }
                _ => None,
            })
            .collect()
    };
    // Closed: the two buttons, each showing its chosen choice.
    let closed = bars(&p);
    assert_eq!(
        closed,
        [
            (
                "!Economic state".into(),
                [192.0, 412.0],
                Action::StatsList(LIST_SERIES)
            ),
            (
                "!4 Hours".into(),
                [18.0, 412.0],
                Action::StatsList(LIST_SPAN)
            ),
        ]
    );
    // The span's list open: its four choices below it, 19 px apart; at 768
    // the list would run past the bottom, so upward.
    let open = Ctx {
        stats_list: Some(LIST_SPAN),
        ..c
    };
    let p = statistics_dialog(&open, 3);
    let spans: Vec<_> = bars(&p)
        .into_iter()
        .filter_map(|(_, at, a)| match a {
            Action::StatisticsSpan(i) => Some((i, at)),
            _ => None,
        })
        .collect();
    assert_eq!(spans.len(), 4);
    let first_y = spans[0].1[1];
    assert!(spans
        .iter()
        .enumerate()
        .all(|(k, (i, at))| usize::from(*i) == k
            && at[0] == 18.0
            && (at[1] - first_y).abs() == 19.0 * k as f32));
    let below = p.at[1] + 412.0 + 19.0 * 5.0 <= 768.0;
    assert_eq!(first_y, if below { 431.0 } else { 393.0 });
    // Only that list is open.
    assert!(!bars(&p)
        .iter()
        .any(|(_, _, a)| matches!(a, Action::StatisticsSeries(_))));
}

/// UI-GAME T1: the bar at (260, 669), button 1 at (295, 709), button 9
/// at (692, 709).
// spec: UI-GAME-2.1, UI-GAME-2.2 (T1)
#[test]
fn t1_hud() {
    let world = flat_world(8, 8);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let p = hud(&ctx(&world, &texts, &catalog));
    assert_eq!(p.at, [260.0, 669.0]);
    let buttons: Vec<_> = p.buttons().collect();
    assert_eq!(buttons.len(), 9);
    let abs = |i: usize| [p.at[0] + buttons[i].1[0], p.at[1] + buttons[i].1[1]];
    assert_eq!(abs(0), [295.0, 709.0]);
    assert_eq!(abs(8), [692.0, 709.0]);
    assert_eq!(buttons[2].3, Some("!Toggle construction view on/off."));
}

/// UI-GAME-8.5b: the production statistics page: its title, tab, separator, image,
/// fourteen toggles and span options; nothing marked at first and the span of 15
/// minutes. UI-GAME-8.5f: the axes, the time labels of a span of 15 minutes and the top
/// label; with nothing marked M is 0 and no curve is drawn.
// spec: UI-GAME-8.5b, UI-GAME-8.5f
#[test]
fn production_page_layout() {
    let world = flat_world(8, 8);
    let texts = Texts::parse("!Good Production\tGood Production\n");
    let catalog = Catalog::default();
    let p = production_dialog(&ctx(&world, &texts, &catalog));
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Text { text, .. } if text == "GOOD PRODUCTION"
    )));
    let toggles: Vec<([f32; 2], u8, bool)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                at,
                action: Action::ProductionMark(i),
                selected,
                ..
            } => Some((*at, *i, *selected)),
            _ => None,
        })
        .collect();
    assert_eq!(toggles.len(), 14);
    for (at, i, selected) in &toggles {
        let i = usize::from(*i);
        assert_eq!(
            *at,
            [13.0 + 48.0 * (i % 7) as f32, 60.0 + 48.0 * (i / 7) as f32]
        );
        assert!(!selected);
    }
    let (x0, y0) = (49.0, 195.0);
    let fills: Vec<([f32; 2], [f32; 2], [f32; 4])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Fill { at, size, rgba } => Some((*at, *size, *rgba)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        [
            ([x0, y0 + 198.0], [250.0, 1.0], [0.1, 0.1, 0.1, 1.0]),
            ([x0 + 250.0, y0 + 2.0], [1.0, 196.0], [0.1, 0.1, 0.1, 1.0]),
        ]
    );
    let labels: Vec<(&str, f32)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, at, font, .. } if *font == Font::Dark && at[1] == y0 + 205.0 => {
                Some((text.as_str(), at[0] - x0))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        labels,
        [
            ("-15", 0.0),
            ("-13", 40.0),
            ("-10", 81.0),
            ("-8", 122.0),
            ("-5", 163.0),
            ("-3", 204.0),
            ("-0", 245.0),
        ]
    );
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Text { text, at, font, .. }
            if text == "0" && *font == Font::Dark && *at == [x0 + 260.0, y0]
    )));
    assert!(
        p.buttons().any(|(a, ..)| *a == Action::ProductionUnmarkAll)
            || p.widgets.iter().any(|w| matches!(
                w,
                Widget::BarButton {
                    action: Action::ProductionUnmarkAll,
                    ..
                }
            ))
    );
}

/// UI-GAME-8.5f: the arithmetic of the curves: M over the whole history, not the
/// span; the newest sample at x0 + 245; 245 / n per sample; at most n + 1 samples;
/// y = y0 + 185 - trunc(v x 185 / M) + p; each series its colour. An unmarked series
/// is not drawn and does not count for M.
// spec: UI-GAME-8.5f
#[test]
fn production_graph_arithmetic() {
    use std::collections::VecDeque;
    let mut series: [VecDeque<u32>; 14] = Default::default();
    // Wood: a peak older than the span sets M.
    series[0] = [500u32].into_iter().chain([10, 0, 100]).collect();
    // Water: unmarked, larger than anything.
    series[4] = [9999].into_iter().collect();
    // Coin: flat at 100, one pixel per counter lower.
    series[10] = [100, 100].into_iter().collect();
    let mut marks = [false; 14];
    marks[0] = true;
    marks[10] = true;
    let mut p = Panel::new(PanelId::Global, [0.0, 0.0]);
    production_graph(&mut p, &series, marks, [0.0, 0.0], 2);
    let m_label = p.widgets.iter().any(
        |w| matches!(w, Widget::Text { text, at, .. } if text == "500" && *at == [260.0, 0.0]),
    );
    assert!(m_label);
    let curve = |rgb: [f32; 3]| -> Vec<([f32; 2], [f32; 2])> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Fill { at, size, rgba } if rgba[..3] == rgb => Some((*at, *size)),
                _ => None,
            })
            .collect()
    };
    let y = |v: u64, c: f32| 185.0 - (v * 185 / 500) as f32 + c;
    // Coin: a level line of 245 / 2 from the newest at x 245, at v = 100, 10 lower.
    assert_eq!(
        curve([0.9, 0.8, 0.3]),
        [([245.0 - 122.5, y(100, 10.0)], [122.5, 1.0])]
    );
    // Wood, n = 2: at most 3 samples, 100, 0 and 10: two sloped segments reaching
    // from x 0 to x 245; the 500 is off the left edge.
    let wood = curve([1.0, 0.0, 0.0]);
    let left = wood.iter().map(|f| f.0[0]).fold(f32::MAX, f32::min);
    let right = wood
        .iter()
        .map(|f| f.0[0] + f.1[0])
        .fold(f32::MIN, f32::max);
    assert!(left.abs() < 1.0, "{left}");
    assert!((right - 245.0).abs() < 1.5, "{right}");
    let tops = wood.iter().map(|f| f.0[1]).fold(f32::MAX, f32::min);
    let bottoms = wood
        .iter()
        .map(|f| f.0[1] + f.1[1])
        .fold(f32::MIN, f32::max);
    assert!((tops - y(100, 0.0)).abs() < 1.0, "{tops}");
    assert!((bottoms - y(0, 0.0)).abs() <= 1.0, "{bottoms}");
    // M = 0 draws no curve: only the axes.
    let mut none = Panel::new(PanelId::Global, [0.0, 0.0]);
    production_graph(&mut none, &Default::default(), marks, [0.0, 0.0], 15);
    let fills = none
        .widgets
        .iter()
        .filter(|w| matches!(w, Widget::Fill { .. }))
        .count();
    assert_eq!(fills, 2);
}

/// UI-GAME-8.5a, 8.5f: the player statistics page draws the chosen series with
/// the production page's routines at its canvas (41, 153): the axes, the seven
/// time labels of the span, M over the whole history of the players drawn, and
/// one curve per player in the game, his number its index (colour and offset).
/// A player out of the game is neither drawn nor counted for M.
// spec: UI-GAME-8.5a, UI-GAME-8.5f
#[test]
fn statistics_page_graph() {
    use s2_sim::statistics::{series, SAMPLE_TICKS};
    let mut rules = s2_sim::Rules::empty();
    let mut castle = s2_sim::BuildingType::new(1, BuildingSize::Large);
    castle.warehouse = true;
    castle.territory_range = 4;
    rules.building_types.push(castle);
    rules.headquarters = Some(1);
    let n = 32 * 32;
    let terrain = s2_sim::map::Terrain {
        width: 32,
        height: 32,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![0; n],
        samples: Vec::new(),
    };
    let setup = s2_sim::GameSetup {
        players: vec![(PlayerId(0), (8, 8)), (PlayerId(1), (24, 24))],
        start_resources: 0,
        local_player: PlayerId(0),
        ..Default::default()
    };
    let mut world = World::new_game(3, terrain, rules, &setup);
    while world.tick() <= 2 * SAMPLE_TICKS + 5 {
        world.step();
    }
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let c = ctx(&world, &texts, &catalog);
    let p = statistics_dialog(&c, series::BUILDINGS as u8);
    let (x0, y0) = (41.0, 153.0);
    let fills: Vec<([f32; 2], [f32; 2], [f32; 4])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Fill { at, size, rgba } => Some((*at, *size, *rgba)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills[..2],
        [
            ([x0, y0 + 198.0], [250.0, 1.0], [0.1, 0.1, 0.1, 1.0]),
            ([x0 + 250.0, y0 + 2.0], [1.0, 196.0], [0.1, 0.1, 0.1, 1.0]),
        ]
    );
    let label = |text: &str, at: [f32; 2]| {
        p.widgets.iter().any(|w| {
            matches!(w, Widget::Text { text: t, at: a, font, .. } if t == text && *a == at && *font == Font::Dark)
        })
    };
    // Each player has one building, his castle: M is 1.
    assert!(label("1", [x0 + 260.0, y0]));
    assert!(label("-15", [x0, y0 + 205.0]));
    assert!(label("-0", [x0 + 245.0, y0 + 205.0]));
    // Two curves, level at v = M: player 0 in his own colour at y0, player 1
    // in his one pixel lower.
    let level = |rgb: [f32; 3], dy: f32| {
        fills
            .iter()
            .any(|(at, size, rgba)| rgba[..3] == rgb && at[1] == y0 + dy && size[1] == 1.0)
    };
    let own = crate::render::models3d::player_colour;
    assert!(level(own(PlayerId(0)), 0.0));
    assert!(level(own(PlayerId(1)), 1.0));
    // Three samples each (ticks 2, 602, 1202): two segments a player.
    assert_eq!(
        fills.len(),
        2 + 2 * 2,
        "the axes and the segments: {fills:?}"
    );
    // UI-GAME-8.5f: a released portrait hides its curve; its button toggles.
    let hidden = Ctx {
        statistics_hidden: [true, false, false, false, false, false, false, false],
        ..ctx(&world, &texts, &catalog)
    };
    let p = statistics_dialog(&hidden, series::BUILDINGS as u8);
    let segments = p
        .widgets
        .iter()
        .filter(|w| matches!(w, Widget::Fill { .. }))
        .count();
    assert_eq!(segments, 2 + 2);
    let toggles: Vec<(bool, bool)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                action: Action::StatisticsPlayer(_),
                enabled,
                selected,
                ..
            } => Some((*enabled, *selected)),
            _ => None,
        })
        .collect();
    assert_eq!(toggles, [(true, false), (true, true)]);
}

/// UI-GAME-8.5f: the player page's M is the largest sample within the span
/// (the last n samples) over the curves drawn, not over the whole history.
// spec: UI-GAME-8.5f
#[test]
fn player_page_top_is_within_the_span() {
    use std::collections::VecDeque;
    let a: VecDeque<u32> = [900, 5, 7, 3].into_iter().collect();
    let b: VecDeque<u32> = [2, 6].into_iter().collect();
    let curves = [
        Curve {
            index: 0,
            rgb: [0.0; 3],
            samples: &a,
        },
        Curve {
            index: 3,
            rgb: [0.0; 3],
            samples: &b,
        },
    ];
    assert_eq!(top_within(&curves, 3), 7);
    assert_eq!(top_within(&curves, 1), 6);
    assert_eq!(top_within(&curves, 4), 900);
    assert_eq!(top_within(&[], 4), 0);
}

/// UI-GAME-8.5a, 8.5b: the series, spans and marks are kept by the interface for
/// the session: a new game keeps them and nothing else; the statistics page opens
/// on the series kept.
// spec: UI-GAME-8.5a, UI-GAME-8.5b
#[test]
fn statistics_choices_last_the_session() {
    let ui = GameUi {
        statistics_series: 4,
        statistics_span: 2,
        production_marks: [true; 14],
        production_span: 3,
        global: Some(Global::Production),
        ..GameUi::default()
    };
    let next = ui.for_new_game();
    assert_eq!(
        (
            next.statistics_series,
            next.statistics_span,
            next.production_span
        ),
        (4, 2, 3)
    );
    assert_eq!(next.production_marks, [true; 14]);
    assert_eq!(next.global, None);
}

/// UI-GAME-11.3a (T56, F62): the messages window is shown when the program
/// starts and at every game start; N (button 7) hides it for that game only:
/// the next game shows it again. No setting holds it.
// spec: UI-GAME-11.3a (T56)
#[test]
fn t56_the_messages_window_at_every_game_start() {
    let first = GameUi::default().for_new_game();
    assert!(first.messages);
    assert!(GameUi::at_start().messages);
    assert_eq!(GameUi::at_start().global, None);
    let hidden = GameUi {
        messages: false,
        ..first
    };
    assert!(hidden.for_new_game().messages);
}

/// UI-GAME-8.5b: the production page reads the counters when opened and then
/// every 601 page updates (its graph drawn from what it read in between); a
/// toggle, `!Unmark all` or a change of span read them again (they clear what was
/// read). UI-GAME-8.5a to 8.5d: each opening sends `STATS_MENUOPENED` with the
/// page's name, once; another series of the player page is no new opening; the
/// settlers page and the fleet send none.
// spec: UI-GAME-8.5b, UI-GAME-8.5a, UI-GAME-8.5c, UI-GAME-8.5d
#[test]
fn production_page_reads_every_601_updates() {
    use crate::ui::statistics_update;
    use std::sync::Arc;
    let world = flat_world(8, 8);
    let mut ui = GameUi {
        global: Some(Global::Production),
        ..GameUi::default()
    };
    let opened = statistics_update(&mut ui, Some(&world));
    assert_eq!(
        opened,
        Some(StatsMenuOpened {
            page: Some("Good Production")
        })
    );
    let first = ui.production_read.clone().expect("read when opened");
    for _ in 0..600 {
        assert_eq!(statistics_update(&mut ui, Some(&world)), None);
        assert!(Arc::ptr_eq(ui.production_read.as_ref().unwrap(), &first));
    }
    statistics_update(&mut ui, Some(&world));
    assert!(
        !Arc::ptr_eq(ui.production_read.as_ref().unwrap(), &first),
        "read at 601"
    );
    let opened_on = |global: Global| {
        let mut ui = GameUi {
            global: Some(Global::Statistics(0)),
            ..GameUi::default()
        };
        statistics_update(&mut ui, None);
        ui.global = Some(global);
        statistics_update(&mut ui, None)
    };
    assert_eq!(opened_on(Global::Statistics(5)), None);
    assert_eq!(
        opened_on(Global::Inventory(false)),
        Some(StatsMenuOpened {
            page: Some("Inventory")
        })
    );
    assert_eq!(opened_on(Global::Inventory(true)), None);
    assert_eq!(opened_on(Global::Fleet), None);
    assert_eq!(
        opened_on(Global::Buildings),
        Some(StatsMenuOpened {
            page: Some("Buildings")
        })
    );
    let mut ui = GameUi {
        global: Some(Global::Statistics(2)),
        ..GameUi::default()
    };
    assert_eq!(
        statistics_update(&mut ui, None),
        Some(StatsMenuOpened { page: None })
    );
}

/// UI-GAME-8.5b: a toggle under the pointer shows the hover picture
/// `uiElements3` (336, 336, 48 x 48), a marked one the pressed picture (384, 336).
// spec: UI-GAME-8.5b
#[test]
fn production_toggles_hover_picture() {
    let world = flat_world(8, 8);
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let hovered = Action::ProductionMark(3);
    let mut marks = [false; 14];
    marks[5] = true;
    let c = Ctx {
        hovered: Some(&hovered),
        production_marks: marks,
        ..ctx(&world, &texts, &catalog)
    };
    let p = production_dialog(&c);
    let picture = |rect: [u32; 4]| -> Vec<[f32; 2]> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Sprite {
                    atlas, rect: r, at, ..
                } if *atlas == Atlas::Elements3 && *r == rect => Some(*at),
                _ => None,
            })
            .collect()
    };
    assert_eq!(picture([336, 336, 48, 48]), [[13.0 + 48.0 * 3.0, 60.0]]);
    assert_eq!(picture([384, 336, 48, 48]), [[13.0 + 48.0 * 5.0, 60.0]]);
}

/// The tabs of a statistics family page: (x, y, tooltip, action,
/// enabled), and which tab is drawn selected.
fn family_tab_row(p: &Panel) -> Vec<([f32; 2], Option<String>, Action, bool)> {
    p.widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                rect,
                at,
                tooltip,
                action,
                enabled,
                ..
            } if rect[1] == 416 => Some((*at, tooltip.clone(), action.clone(), *enabled)),
            _ => None,
        })
        .collect()
}

/// UI-GAME-8.5: the six tabs, icon ids 17, 18, 19, 22, 21, 12, at (15, 25)
/// stepping 40 with the spec's tooltips; Production is disabled. The page
/// is placed for 512 x 536 (centred).
// spec: UI-GAME-8.5
#[test]
fn statistics_family_tabs() {
    let world = flat_world(8, 8);
    let texts = Texts::default();
    let catalog = Catalog::default();
    let p = inventory_dialog(&ctx(&world, &texts, &catalog), false);
    let tabs = family_tab_row(&p);
    assert_eq!(tabs.len(), 6);
    for (i, t) in tabs.iter().enumerate() {
        assert_eq!(t.0, [15.0 + 40.0 * i as f32, 25.0]);
    }
    let tips: Vec<&str> = tabs.iter().map(|t| t.1.as_deref().unwrap()).collect();
    assert_eq!(
        tips,
        [
            "!Player statistics",
            "!Production statistices",
            "!All goods in all depots and the castle",
            "!All settlers in all depots and the castle",
            "!Size of village and productivity",
            "!Fleet",
        ]
    );
    assert_eq!(tabs[3].2, Action::OpenGlobal(Global::Inventory(true)));
    assert_eq!(tabs[5].2, Action::OpenGlobal(Global::Fleet));
    let enabled: Vec<bool> = tabs.iter().map(|t| t.3).collect();
    assert_eq!(enabled, [true; 6]);
    let ids: Vec<u32> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button { rect, .. } if rect[1] == 416 => Some((rect[0] - 8) / 40),
            _ => None,
        })
        .collect();
    // The goods tab is drawn selected (its third sprite, y 480).
    assert_eq!(ids, [17, 18, 19, 22, 21, 12]);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { rect, at, .. } if *rect == [8 + 40 * 19, 480, 40, 27] && *at == [95.0, 25.0]
    )));
    // 1024 x 768: ((1024 - 512) / 2, (768 - 536) / 2).
    assert_eq!(p.at, [256.0, 116.0]);
}

/// UI-GAME-7.1, 8.5c (`dialog26_0`): the inventory counts the goods of all
/// the player's warehouses, not another player's; each good a slot by its
/// slot number with the name as tooltip and its count right-aligned at
/// (+46, +49) in `small_no_alias_white`.
// spec: UI-GAME-8.5c, UI-GAME-7.1
#[test]
fn inventory_page() {
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
    castle.territory_range = 5;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![castle],
        headquarters: Some(9),
        start_stock: vec![(100, 5), (101, 2)],
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![(PlayerId(0), (8, 8)), (PlayerId(1), (30, 30))],
        local_player: PlayerId(0),
        ..Default::default()
    };
    let world = World::new_game(1, terrain, rules, &setup);
    let mine = inventory(&world, PlayerId(0));
    let hq: std::collections::BTreeMap<u32, u32> =
        world.stock((8, 8)).unwrap().iter().copied().collect();
    assert!(hq.get(&100).is_some_and(|&n| n > 0));
    assert_eq!(mine, hq);
    // Good 101 has slot number 7, the others none (their list places).
    let catalog = Catalog {
        wares: vec![100, 101, 102],
        good_slots: [(101, 7)].into_iter().collect(),
        ..Catalog::default()
    };
    let texts = Texts::default();
    let p = inventory_dialog(&ctx(&world, &texts, &catalog), false);
    let slots: Vec<([f32; 2], Action)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                rect, at, action, ..
            } if *rect == [0, 80, 48, 56] => Some((*at, action.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        slots,
        [
            ([23.0, 60.0], Action::None),
            ([77.0, 122.0], Action::None),
            ([131.0, 60.0], Action::None)
        ]
    );
    let counts: Vec<(String, [f32; 2], Font, crate::ui::widgets::Align)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text {
                text,
                at,
                font,
                align,
                ..
            } if text.chars().all(|c| c.is_ascii_digit()) => {
                Some((text.clone(), *at, *font, *align))
            }
            _ => None,
        })
        .collect();
    use crate::ui::widgets::Align::TopRight;
    assert_eq!(
        counts,
        [
            (hq[&100].to_string(), [69.0, 109.0], Font::Heading, TopRight),
            (
                hq[&101].to_string(),
                [123.0, 171.0],
                Font::Heading,
                TopRight
            ),
            ("0".into(), [177.0, 109.0], Font::Heading, TopRight)
        ]
    );
    assert_eq!(goods_slot(7), [77.0, 122.0]);
}

/// UI-GAME-8.5 (`dialog39_0`): the fleet page: the picture `uiGame`
/// (136, 440, 344 x 184) at (6, 255); one row per finished ship from
/// (50, 85), a click selects the ship (its window, the camera) and closes
/// the fleet; 11 rows fit above the picture.
// spec: UI-GAME-8.5, SHIP-2.1
#[test]
fn fleet_page() {
    let mut world = flat_world(20, 20);
    let a = world.debug_add_ship(PlayerId(0), (5, 5));
    world.debug_add_ship(PlayerId(1), (9, 9));
    let texts = Texts::parse("!Ship\tShip\n");
    let catalog = Catalog::default();
    let p = fleet_dialog(&ctx(&world, &texts, &catalog));
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { atlas: Atlas::Game, rect, at, .. }
            if *rect == [136, 440, 344, 184] && *at == [6.0, 255.0]
    )));
    let rows = |p: &Panel| -> Vec<(String, [f32; 2], Action)> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::TextButton {
                    text, at, action, ..
                } => Some((text.clone(), *at, action.clone())),
                _ => None,
            })
            .collect()
    };
    // Not launched yet: not in the fleet (SHIP-2.1).
    assert!(rows(&p).is_empty());
    world.debug_launch_ship(a);
    let p = fleet_dialog(&ctx(&world, &texts, &catalog));
    let r = rows(&p);
    assert_eq!(r.len(), 1);
    // UI-GAME-4.5 c: the default name's key, which has no text.
    assert_eq!(r[0].0, "RomanShipName0");
    assert_eq!(r[0].1, [50.0, 85.0]);
    assert_eq!(r[0].2, Action::FleetShip(a));
    let last = FLEET_FIRST[1] + FLEET_STEP * (FLEET_ROWS - 1) as f32;
    assert!(last + FLEET_STEP <= 255.0);
    assert_eq!(BIG_DIALOG[0], 6.0 + 344.0 + 18.0);
}

/// UI-GAME-4.5: the ship window: the ship view 150 x 150 at (113, 61)
/// drawn before the porthole `uiElements5` (0, 40, 200 x 192) at
/// (88, 40); `!Name of the ship` at (104, 392) over the name input at
/// (104, 400); the arrows `uiElements0` (40, 152) at (84, 400) and
/// (136, 152) at (253, 400) open the other ships' windows. UI-GAME-4.5 c:
/// a ship without a typed name shows the text of `<Tribe>ShipName<k>`, a
/// named one its name (the game's), in the window and the fleet list; while
/// the input has the keys it shows the name being typed.
// spec: UI-GAME-4.5, UI-MENU-5.3
#[test]
fn ship_window_layout() {
    let mut world = flat_world(20, 20);
    let a = world.debug_add_ship(PlayerId(0), (5, 5));
    let b = world.debug_add_ship(PlayerId(0), (8, 8));
    world.debug_launch_ship(a);
    world.debug_launch_ship(b);
    world.submit(s2_sim::Command::NameShip {
        player: PlayerId(0),
        ship: b as u32,
        name: "Aurora".into(),
    });
    world.step();
    let texts = Texts::parse("!RomanShipName0\tSea Lion\n!RomanShipName1\tOrca\n");
    let catalog = Catalog::default();
    let names: std::collections::BTreeMap<usize, String> = [(a, "Sea Lion".to_string())].into();
    let c = Ctx {
        ship_names: &names,
        focus: Some(crate::ui::widgets::Focus {
            input: crate::ui::widgets::Input::ShipName(a),
            caret: 4,
            dot: false,
        }),
        ..ctx(&world, &texts, &catalog)
    };
    let p = ship_window(&c, a).unwrap();
    let view = p
        .widgets
        .iter()
        .position(|w| {
            matches!(
                w,
                Widget::Picture { picture: crate::ui::widgets::Picture::ShipView, at, size }
                    if *at == [113.0, 61.0] && *size == 150.0
            )
        })
        .expect("the ship view");
    let porthole = p
        .widgets
        .iter()
        .position(|w| {
            matches!(
                w,
                Widget::Sprite { atlas: Atlas::Elements5, rect, at, .. }
                    if *rect == [0, 40, 200, 192] && *at == [88.0, 40.0]
            )
        })
        .expect("the porthole");
    assert!(view < porthole);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Text { text, at, .. } if text == "!Name of the ship" && *at == [104.0, 392.0]
    )));
    // UI-MENU-5.3: the style 2 bar at (104, 400); the input has the keys:
    // the default name with the caret after its fourth character, the
    // text at (0, 1) from the bar.
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button { atlas: Atlas::Elements0, rect, at, action: Action::FocusInput(_, x), .. }
            if *rect == [266, 296, 152, 18] && *at == [104.0, 400.0] && *x == p.at[0] + 104.0
    )));
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Text { text, at, .. } if text == "Sea |Lion" && *at == [104.0, 401.0]
    )));
    let arrows: Vec<_> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button {
                atlas: Atlas::Elements0,
                rect,
                at,
                action,
                ..
            } if rect[1] == 152 && at[1] == 400.0 => Some((rect[0], *at, action.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        arrows,
        [
            (
                40,
                [84.0, 400.0],
                Action::ShipArrow {
                    ship: b,
                    right: false
                }
            ),
            (
                136,
                [253.0, 400.0],
                Action::ShipArrow {
                    ship: b,
                    right: true
                }
            ),
        ]
    );
    let fleet = fleet_dialog(&c);
    let rows: Vec<String> = fleet
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::TextButton { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(rows, ["Sea Lion", "Aurora"]);
    assert_eq!(ship_name(&c, b), "Aurora");
    // Without a typed name the second ship would be the second key's.
    assert_eq!(world.ship_name_index(b), Some(1));
}

/// UI-GAME T46 (UI-GAME-1.11): the paused view's layer covers the whole
/// screen in black at alpha 0.65, the byte 165 (every stored value keeps
/// 90 / 255), and shows "!Pause" in `light_system` centred on it; it takes
/// no clicks.
// spec: UI-GAME-1.11 (UI-GAME T46)
#[test]
fn t46_the_paused_view_layer() {
    let world = flat_world(12, 12);
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let p = pause_layer(&ctx(&world, &texts, &catalog));
    assert_eq!(p.id, PanelId::Pause);
    assert_eq!(
        p.widgets[0],
        Widget::Fill {
            at: [0.0, 0.0],
            size: [1024.0, 768.0],
            rgba: [0.0, 0.0, 0.0, 165.0 / 255.0],
        }
    );
    assert_eq!((255.0 * (1.0 - PAUSE_ALPHA)).round(), 90.0);
    assert_eq!(
        p.widgets[1],
        Widget::Text {
            text: "!Pause".into(),
            font: Font::Body,
            at: [512.0, 384.0],
            colour: WHITE,
            align: crate::ui::widgets::Align::Centre,
        }
    );
    let frame = crate::ui::render::UiFrame {
        panels: vec![p],
        map_tooltip: None,
    };
    assert!(!frame.covers([512.0, 300.0]));
}

/// UI-GAME-11.2: at 1024 x 768 the corners sit at (0, 0), (944, 0),
/// (0, 624) and (904, 624), the edges span the screen 12 px thick.
#[test]
fn screen_frame_pieces() {
    let p = screen_frame([1024.0, 768.0]);
    let pieces: Vec<([f32; 2], [f32; 2])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Sprite { at, size, rect, .. } => {
                Some((*at, size.unwrap_or([rect[2] as f32, rect[3] as f32])))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        pieces,
        [
            ([0.0, 0.0], [1024.0, 12.0]),
            ([0.0, 756.0], [1024.0, 12.0]),
            ([0.0, 0.0], [12.0, 768.0]),
            ([1012.0, 0.0], [12.0, 768.0]),
            ([0.0, 0.0], [80.0, 80.0]),
            ([944.0, 0.0], [80.0, 80.0]),
            ([0.0, 624.0], [120.0, 144.0]),
            ([904.0, 624.0], [120.0, 144.0]),
        ]
    );
}

/// UI-GAME T4: a medium site shows the small and medium tabs.
// spec: UI-GAME-3.2, UI-GAME-3.5 (T4)
#[test]
fn t4_build_tabs() {
    assert_eq!(build_tabs(Placement::Medium), [5, 6]);
    assert_eq!(build_tabs(Placement::Small), [5]);
    assert_eq!(build_tabs(Placement::Harbor), [5, 6, 7]);
    assert!(build_tabs(Placement::Mine).is_empty());
}

/// UI-GAME T39, T40 (play test 11): on the install's rules the large page
/// lists farm, pig farm, donkey breeding and fortress (the harbour only on
/// a harbour site) and never the castle, whose icon is the flag's; the
/// small page lists its types in the order of their sort index, the
/// lookout tower and the military types among the others. Skipped without
/// `S2_GAME_DIR`.
// spec: UI-GAME-3.2 (T39, T40)
#[test]
fn t39_t40_build_pages() {
    let Some(root) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
    let (rules, _) = s2_rules::load(&dir).expect("the property scripts");
    let catalog = Catalog::new(&rules, Vec::new());
    let page = |class, tab| -> Vec<(String, (u32, u32))> {
        build_page(&catalog, class, tab)
            .into_iter()
            .map(|b| (b.name.clone(), b.icon.unwrap()))
            .collect()
    };
    let named = |cells: &[(&str, (u32, u32))]| -> Vec<(String, (u32, u32))> {
        cells.iter().map(|&(n, i)| (n.to_owned(), i)).collect()
    };
    assert_eq!(
        page(Placement::Large, 7),
        named(&[
            ("Farm", (1, 4)),
            ("Piggerey", (5, 4)),
            ("Donkey Breeding", (2, 4)),
            ("Fortress", (4, 4)),
        ])
    );
    assert_eq!(
        page(Placement::Harbor, 7),
        named(&[
            ("Farm", (1, 4)),
            ("Piggerey", (5, 4)),
            ("Donkey Breeding", (2, 4)),
            ("Harbor", (3, 4)),
            ("Fortress", (4, 4)),
        ])
    );
    assert_eq!(
        page(Placement::Large, 5),
        named(&[
            ("WoodcutterHut", (1, 1)),
            ("ForesterHouse", (2, 0)),
            ("Stone Pit", (3, 0)),
            ("FisherHut", (1, 0)),
            ("HuntingLodge", (0, 1)),
            ("WatchTower", (5, 0)),
            ("Spring", (2, 1)),
            ("Barracks", (4, 0)),
            ("GuardHouse", (3, 1)),
        ])
    );
    // No cell shows the flag's icon (0, 0) after the flag button.
    for tab in [5, 6, 7] {
        assert!(page(Placement::Harbor, tab)
            .iter()
            .all(|(_, icon)| *icon != (0, 0)));
    }
}

/// Hook H6: a building type an overlay adds (a charcoal burner with the
/// woodcutter's icon, sort 9) is the last cell of the small page, after the
/// original's nine, and its production window needs its own recipe's goods;
/// the coal mine's are its product's, as without the overlay. Skipped
/// without `S2_GAME_DIR`.
// spec: UI-GAME-3.2 (hook H6: a project's building type in the build menu)
#[test]
fn a_projects_building_type_has_its_cell_and_its_recipe() {
    let Some(root) = std::env::var_os("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
    let (plain, _) = s2_rules::load(&dir).expect("the property scripts");
    let overlay = s2_rules::Overlay {
        name: "charcoal".into(),
        code: b"charcoalBurner = properties.bp_create(\"C4A4C0A1\")
                properties.bp_setName(charcoalBurner, \"Charcoal Burner\")
                properties.bp_setIconXY(charcoalBurner, 1, 1)
                properties.bp_setSort(charcoalBurner, 9)
                properties.bp_setSize(charcoalBurner, 0)
                properties.bp_addFrameCost(charcoalBurner, plank, 1)
                properties.bp_setWorker(charcoalBurner, good_helper)
                properties.bp_setGoods(charcoalBurner, coal, 1)
                reborn.bp_addInput(charcoalBurner, wood, 2)"
            .to_vec(),
    };
    let (rules, _) = s2_rules::load_with_overlays(&dir, &[overlay]).expect("with the overlay");
    let (before, after) = (
        Catalog::new(&plain, Vec::new()),
        Catalog::new(&rules, Vec::new()),
    );
    let names = |catalog: &Catalog| -> Vec<String> {
        build_page(catalog, Placement::Small, 5)
            .into_iter()
            .map(|b| b.name.clone())
            .collect()
    };
    let mut expected = names(&before);
    expected.push("Charcoal Burner".to_owned());
    assert_eq!(names(&after), expected);
    let wood = rules.id_named("wood").unwrap();
    assert_eq!(after.inputs.get(&0xC4A4_C0A1), Some(&vec![wood]));
    let coal_mine = rules.id_named("coalMine").unwrap();
    assert_eq!(after.inputs.get(&coal_mine), before.inputs.get(&coal_mine));
}

/// UI-GAME T12: 7 available, share 0.5: the count shown is sent.
// spec: UI-GAME-5.1, MIL-ATK-1.4 (T12)
#[test]
fn t12_attack_count() {
    assert_eq!(attack_count(7, 0.5), 4);
    assert_eq!(attack_count(7, 0.0), 0);
    assert_eq!(attack_count(7, 1.0), 7);
}

/// UI-GAME T3, 3.1: the flag menu's five portraits from (15, 29) in
/// steps of 52, in a row (Q-191 a), with their pictures at (x, 144) and
/// highlighted at (x, 192); the waterway is disabled away from the
/// coast (TRN-ROAD-7.1), the scholar and explorer call their
/// specialists (ECO-SPC-1.1). 1.8: the title in capitals, the close button at
/// (w - 40, 5).
#[test]
fn flag_menu_rows() {
    let mut world = flat_world(10, 10);
    world.submit(Command::PlaceFlag {
        player: PlayerId(0),
        at: (4, 4),
    });
    world.step();
    let texts = Texts::parse("!Flag Menu\tFlag Menu\n!Create a street.\tBuild road\n");
    let catalog = Catalog::default();
    let p = flag_menu(&ctx(&world, &texts, &catalog), (4, 4));
    let rows: Vec<_> = p
        .buttons()
        .filter(|b| b.1[1] == 29.0)
        .map(|b| (b.1[0], b.2))
        .collect();
    assert_eq!(
        rows,
        [
            (15.0, true),
            (67.0, false),
            (119.0, true),
            (171.0, true),
            (223.0, true)
        ]
    );
    let first = p.widgets.iter().find_map(|w| match w {
        Widget::Button {
            rect,
            hover,
            tooltip,
            action: Action::StartRoad(_),
            ..
        } => Some((*rect, hover.clone(), tooltip.clone())),
        _ => None,
    });
    assert_eq!(
        first,
        Some((
            [0, 144, 48, 48],
            Hover::Swap([0, 192]),
            Some("Build road".into())
        ))
    );
    let title = p.widgets.iter().find_map(|w| match w {
        Widget::Text {
            text,
            align: Align::TopCentre,
            ..
        } => Some(text.as_str()),
        _ => None,
    });
    assert_eq!(title, Some("FLAG MENU"));
    // UI-GAME T29, 1.8 (corrected F59, E-ui-033): the picture
    // `uiSmallDialog` (8, 8, 304 x 96) drawn 1:1, the close button at (264,
    // 5), five portraits in a row from (15, 29).
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite {
            atlas: Atlas::SmallDialog,
            rect: [8, 8, 304, 96],
            at,
            size: Some(size),
            ..
        } if *at == [0.0, 0.0] && *size == SMALL_DIALOG
    )));
    assert_eq!(SMALL_DIALOG, [304.0, 96.0]);
    assert!(p.buttons().any(|b| b.1 == [264.0, 5.0]));
    for i in 0..5 {
        let x = 15.0 + 52.0 * i as f32;
        assert!(p.buttons().any(|b| b.1 == [x, 29.0]), "portrait {i}");
    }
}

/// A game with a headquarters on (10, 10) and a finished guard house
/// (type 3, two soldiers, built at once) near it; its field.
fn guard_house_game() -> (World, Point) {
    use s2_sim::{BuildingSize, BuildingType};
    let n = 30 * 30;
    let terrain = s2_sim::map::Terrain {
        width: 30,
        height: 30,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut castle = BuildingType::new(9, BuildingSize::Large);
    castle.territory_range = 9;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let mut guard = BuildingType::new(3, BuildingSize::Small);
    guard.territory_range = 6;
    guard.soldier_limit = 2;
    guard.worker_limit = 0;
    guard.instant = true;
    let rules = s2_sim::Rules {
        building_types: vec![castle, guard],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![(PlayerId(0), (10, 10))],
        local_player: PlayerId(0),
        ..Default::default()
    };
    let mut world = World::new_game(1, terrain, rules, &setup);
    for at in [(14, 12), (6, 12), (14, 8), (13, 14)] {
        world.submit(Command::PlaceBuilding {
            player: PlayerId(0),
            building: 3,
            at,
        });
        world.step();
        if world.military_at(at).is_some() {
            return (world, at);
        }
    }
    panic!("no guard house placed");
}

/// A game with a headquarters on (10, 10) (no stock) and a finished hut of
/// type 4 with a worker kind and a product, built at once; its field.
fn hut_game() -> (World, Point) {
    hut_game_with(|_| {})
}

/// `hut_game` with the small type changed by `change` (its rules too).
fn hut_game_with(change: impl FnOnce(&mut s2_sim::Rules)) -> (World, Point) {
    use s2_sim::{BuildingSize, BuildingType};
    let n = 30 * 30;
    let terrain = s2_sim::map::Terrain {
        width: 30,
        height: 30,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut castle = BuildingType::new(9, BuildingSize::Large);
    castle.territory_range = 9;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let mut hut = BuildingType::new(4, BuildingSize::Small);
    hut.worker = Some(77);
    hut.product = Some(78);
    hut.instant = true;
    let mut rules = s2_sim::Rules {
        building_types: vec![castle, hut],
        headquarters: Some(9),
        ..s2_sim::Rules::empty()
    };
    change(&mut rules);
    let setup = s2_sim::GameSetup {
        players: vec![(PlayerId(0), (10, 10))],
        local_player: PlayerId(0),
        ..Default::default()
    };
    let mut world = World::new_game(1, terrain, rules, &setup);
    for at in [(14, 12), (6, 12), (14, 8), (13, 14)] {
        world.submit(Command::PlaceBuilding {
            player: PlayerId(0),
            building: 4,
            at,
        });
        world.step();
        if world.building_at(at).is_some_and(|b| !b.under_construction) {
            return (world, at);
        }
    }
    panic!("no hut placed");
}

/// UI-GAME-4.1a (T57, F59): a production building's window is the picture
/// `uiMediumDialog` (8, 8, 480 x 224) drawn 1:1; BUILDING, PRODUCT and TOOL
/// centred on x 64, 160, 232 with line tops at y 37, PRODUCTIVITY plain
/// at (136, 106); no worker inside: line 1 empty, line 2 "No worker in
/// building" at (40, 182); production off: line 1 "Production stopped" at
/// (40, 160); never a worker activity text; destroy at (407, 157), the
/// production switch at (357, 157).
// spec: UI-GAME-4.1a (T57, T60's gauge), UI-GAME-1.8, UI-GAME-1.7
// (4.1a e: the three empty slot frames and the gauge's value; f: no TOOL
// for a tool-less worker; g: the gauge is the site window's.)
#[test]
fn t57_production_window_captions_and_status_lines() {
    let (mut world, at) = hut_game();
    let texts = Texts::parse(
        "!No worker at home.\tNo worker in building\n!Production stopped.\tProduction stopped\n",
    );
    // The hut's worker needs a tool (4.1a f: else no TOOL section).
    let mut catalog = Catalog::default();
    let hut = world.building_at(at).unwrap().building_type;
    catalog.tools_of.insert(hut, 1);
    let texts_of = |p: &Panel| -> Vec<(String, [f32; 2], Align, Font)> {
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Text {
                    text,
                    at,
                    align,
                    font,
                    ..
                } => Some((text.clone(), *at, *align, *font)),
                _ => None,
            })
            .collect()
    };
    let p = production_window(&ctx(&world, &texts, &catalog), at);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { atlas: Atlas::MediumDialog, rect: [8, 8, 480, 224], at, size: Some(size), .. }
            if *at == [0.0, 0.0] && *size == [480.0, 224.0]
    )));
    let shown = texts_of(&p);
    for (key, x) in [("!BUILDING", 64.0), ("!PRODUCT", 160.0), ("!TOOL", 232.0)] {
        assert!(
            shown.contains(&(key.into(), [x, 37.0], Align::TopCentre, Font::Heading)),
            "{key}: {shown:?}"
        );
    }
    assert!(shown.contains(&(
        "!PRODUCTIVITY".into(),
        [136.0, 106.0],
        Align::TopLeft,
        Font::Heading
    )));
    // 4.1a (f): a type whose worker needs no tool: no TOOL caption, slot
    // or separator after it.
    let plain = Catalog::default();
    let well = production_window(&ctx(&world, &texts, &plain), at);
    assert!(!texts_of(&well).iter().any(|t| t.0 == "!TOOL"));
    let separator_after_tool = well.widgets.iter().any(|w| {
        matches!(w, Widget::Sprite { atlas: Atlas::Elements3, rect: [160, 0, 8, 56], at, .. } if at[0] == 264.0)
    });
    assert!(!separator_after_tool);
    // 4.1a (e), (g): the gauge is the site window's, from (137, 112) to
    // (443, 128), its value centred in it.
    let value = format!("{}%", world.building_at(at).unwrap().productivity);
    assert!(shown.contains(&(value, [290.0, 120.0], Align::TopCentre, Font::Body)));
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Fill { at, size, .. } if *at == [137.0, 112.0] && *size == [306.0, 16.0]
    )));
    // 4.1a (e): three empty slot frames, no input.
    let frames = p
        .widgets
        .iter()
        .filter(|w| {
            matches!(w, Widget::Sprite { atlas: Atlas::Elements3, rect: [440, 216, 48, 48], at, .. }
                if at[1] == 43.0 && [275.0, 335.0, 395.0].contains(&at[0]))
        })
        .count();
    assert_eq!(frames, 3);
    let body: Vec<_> = shown
        .iter()
        .filter(|t| t.3 == Font::Body && t.1[0] == 40.0)
        .collect();
    assert_eq!(
        body,
        [&(
            "No worker in building".to_owned(),
            [40.0, 182.0],
            Align::TopLeft,
            Font::Body
        )]
    );
    let place = |p: &Panel, key: &str| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Button { at, tooltip, .. } if tooltip.as_deref() == Some(key) => Some(*at),
            _ => None,
        })
    };
    assert_eq!(place(&p, "!Destroy the building."), Some([407.0, 157.0]));
    assert_eq!(
        place(&p, "!Toggle production ON/OFF."),
        Some([357.0, 157.0])
    );
    world.submit(Command::ProductionToggle {
        player: PlayerId(0),
        at,
    });
    world.step();
    let p = production_window(&ctx(&world, &texts, &catalog), at);
    let shown = texts_of(&p);
    assert!(shown.contains(&(
        "Production stopped".into(),
        [40.0, 160.0],
        Align::TopLeft,
        Font::Body
    )));
}

/// UI-GAME-4.1a (g) (the original's catapult window, E-trace-029): BUILDING
/// and the portrait, both separators, no PRODUCT, TOOL, NEEDED GOODS or
/// PRODUCTIVITY, one slot at 395 with the stone and `<stock>/4` under it,
/// no empty slot frames; the switch and destroy buttons.
// spec: UI-GAME-4.1a, ECO-PROD-6.1
#[test]
fn the_catapult_window_shows_its_stone_only() {
    let (world, at) = hut_game_with(|rules| {
        let hut = rules.building_types.iter_mut().find(|t| t.id == 4).unwrap();
        hut.product = None;
        hut.catapult = true;
        hut.stock_limit = Some(4);
        rules.stone = Some(34);
    });
    let texts = Texts::parse("");
    let catalog = Catalog {
        stone: Some(34),
        ..Catalog::default()
    };
    let p = production_window(&ctx(&world, &texts, &catalog), at);
    let shown: Vec<&str> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(shown.contains(&"!BUILDING"));
    for gone in ["!PRODUCT", "!TOOL", "!NEEDED GOODS ", "!PRODUCTIVITY"] {
        assert!(!shown.contains(&gone), "{gone}: {shown:?}");
    }
    assert!(!p
        .widgets
        .iter()
        .any(|w| matches!(w, Widget::Fill { at, .. } if *at == [137.0, 112.0])));
    let separators = p
        .widgets
        .iter()
        .filter(|w| {
            matches!(w, Widget::Sprite { atlas: Atlas::Elements3, rect: [160, 0, 8, 56], at, .. }
                if at[1] == 32.0 && [192.0, 264.0].contains(&at[0]))
        })
        .count();
    assert_eq!(separators, 2);
    let frames: Vec<f32> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Sprite {
                atlas: Atlas::Elements3,
                rect: [440, 216, 48, 48],
                at,
                ..
            } if at[1] == 43.0 => Some(at[0]),
            _ => None,
        })
        .collect();
    assert_eq!(frames, [395.0]);
    assert!(shown.contains(&"0/4"), "{shown:?}");
}

/// UI-GAME-4.1a (g), ECO-PROD-1.3: the shipyard's PRODUCT slot shows its
/// product (the boat) and the ship once switched to ships; the switch is
/// the third button at (307, 157).
// spec: UI-GAME-4.1a, ECO-PROD-1.3
#[test]
fn the_shipyard_shows_the_ship_when_switched() {
    let (mut world, at) = hut_game_with(|rules| {
        let hut = rules.building_types.iter_mut().find(|t| t.id == 4).unwrap();
        hut.switchable = true;
    });
    let texts = Texts::parse("");
    let mut catalog = Catalog {
        ship: Some(90),
        ..Catalog::default()
    };
    catalog.products.insert(4, 78);
    catalog.good_icons.insert(78, (1, 0));
    catalog.good_icons.insert(90, (2, 0));
    let product_icon = |world: &World| {
        let p = production_window(&ctx(world, &texts, &catalog), at);
        p.widgets.iter().find_map(|w| match w {
            Widget::Sprite {
                atlas: Atlas::Elements1,
                rect,
                at,
                ..
            } if at[1] >= 43.0 && at[0] >= 136.0 && at[0] < 184.0 => Some(*rect),
            _ => None,
        })
    };
    assert_eq!(product_icon(&world), Some([48, 0, 48, 48]));
    world.submit(Command::ProductionSwitch {
        player: PlayerId(0),
        at,
    });
    world.step();
    assert_eq!(product_icon(&world), Some([96, 0, 48, 48]));
    let p = production_window(&ctx(&world, &texts, &catalog), at);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button { action: Action::Command(Command::ProductionSwitch { .. }), at, .. } if *at == [307.0, 157.0]
    )));
}

/// Hook W20 (docs/extending.md "Recipes a player chooses"): a building of a
/// type with a recipe set shows the chosen recipe's product and inputs, and
/// the third button at (307, 157) chooses the next recipe, after the last
/// the first again; a type without a set has no such button.
// spec: none (hook W20; the window is UI-GAME-4.1a's)
#[test]
fn a_recipe_set_shows_the_chosen_recipe_and_a_next_button() {
    let (mut world, at) = hut_game_with(|rules| {
        rules.recipe_sets.insert(
            4,
            vec![
                s2_sim::Recipe {
                    product: 78,
                    inputs: vec![(1, 2)],
                },
                s2_sim::Recipe {
                    product: 90,
                    inputs: vec![(2, 1)],
                },
            ],
        );
    });
    let texts = Texts::parse("");
    let mut catalog = Catalog::default();
    catalog.good_icons.insert(78, (1, 0));
    catalog.good_icons.insert(90, (2, 0));
    let product_icon = |world: &World| {
        let p = production_window(&ctx(world, &texts, &catalog), at);
        p.widgets.iter().find_map(|w| match w {
            Widget::Sprite {
                atlas: Atlas::Elements1,
                rect,
                at,
                ..
            } if at[1] >= 43.0 && at[0] >= 136.0 && at[0] < 184.0 => Some(*rect),
            _ => None,
        })
    };
    let next = |world: &World| {
        let p = production_window(&ctx(world, &texts, &catalog), at);
        p.widgets.iter().find_map(|w| match w {
            Widget::Button {
                action: Action::Command(Command::ChooseRecipe { index, .. }),
                at: place,
                ..
            } if *place == [307.0, 157.0] => Some(*index),
            _ => None,
        })
    };
    assert_eq!(product_icon(&world), Some([48, 0, 48, 48]));
    assert_eq!(next(&world), Some(1));
    world.submit(Command::ChooseRecipe {
        player: PlayerId(0),
        at,
        index: 1,
    });
    world.step();
    assert_eq!(product_icon(&world), Some([96, 0, 48, 48]));
    assert_eq!(next(&world), Some(0));
    let (plain, plain_at) = hut_game_with(|_| {});
    let p = production_window(&ctx(&plain, &texts, &catalog), plain_at);
    assert!(!p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::Command(Command::ChooseRecipe { .. }),
            ..
        }
    )));
}

/// Hook W23 (docs/extending.md "Recipes a player chooses"): a type with
/// three recipes has a Recipes button in the next-recipe button's place,
/// opening the production window's recipe tab: one row per recipe, the
/// check box marked on the chosen one; a click on the third row's box sends
/// `ChooseRecipe` with index 2, after which the third is marked; the close
/// button goes back to the production window.
// spec: none (hook W23; the production window is UI-GAME-4.1a's)
#[test]
fn three_recipes_are_listed_in_a_tab() {
    let recipe = |product, input| s2_sim::Recipe {
        product,
        inputs: vec![(input, 2)],
    };
    let (mut world, at) = hut_game_with(|rules| {
        rules
            .recipe_sets
            .insert(4, vec![recipe(78, 1), recipe(90, 2), recipe(91, 1)]);
    });
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let p = production_window(&ctx(&world, &texts, &catalog), at);
    let opener = p.widgets.iter().find_map(|w| match w {
        Widget::Button {
            action,
            tooltip,
            at: place,
            ..
        } if *place == [307.0, 157.0] => Some((action.clone(), tooltip.clone())),
        _ => None,
    });
    assert_eq!(
        opener,
        Some((
            Action::BuildingTab(RECIPES_TAB),
            Some("Recipes.".to_owned())
        ))
    );
    let rows = |world: &World| -> Vec<(u32, bool, [f32; 2])> {
        let p = building_window(&ctx(world, &texts, &catalog), at, RECIPES_TAB).unwrap();
        p.widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Button {
                    action: Action::Command(Command::ChooseRecipe { index, .. }),
                    selected,
                    at,
                    ..
                } => Some((*index, *selected, *at)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        rows(&world),
        [
            (0, true, [20.0, 60.0]),
            (1, false, [20.0, 100.0]),
            (2, false, [20.0, 140.0])
        ]
    );
    world.submit(Command::ChooseRecipe {
        player: PlayerId(0),
        at,
        index: 2,
    });
    world.step();
    assert_eq!(
        rows(&world).iter().map(|r| r.1).collect::<Vec<_>>(),
        [false, false, true]
    );
    let list = building_window(&ctx(&world, &texts, &catalog), at, RECIPES_TAB).unwrap();
    assert!(list.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::BuildingTab(0),
            ..
        }
    )));
    // Two recipes keep the next-recipe button and no list.
    let (two, two_at) = hut_game_with(|rules| {
        rules
            .recipe_sets
            .insert(4, vec![recipe(78, 1), recipe(90, 2)]);
    });
    let p = building_window(&ctx(&two, &texts, &catalog), two_at, RECIPES_TAB).unwrap();
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Button {
            action: Action::Command(Command::ChooseRecipe { index: 1, .. }),
            ..
        }
    )));
}

/// Hook W15 (docs/extending.md "Deepening a mine"): a building of a type
/// with a deepening shows the deepen button at (307, 157), usable only while
/// the command would be accepted (here never: the type is not a mine), its
/// tooltip the cost and the count; a type without one has no such button.
// spec: none (hook W15; the window is UI-GAME-4.1a's)
#[test]
fn a_deepening_shows_its_button() {
    let (world, at) = hut_game_with(|rules| {
        rules.deepenings.insert(
            4,
            s2_sim::Deepening {
                share_per_mille: 250,
                most: 2,
                costs: vec![(1, 4)],
                food: vec![2],
            },
        );
    });
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let deepen = |world: &World, at| {
        let p = production_window(&ctx(world, &texts, &catalog), at);
        p.widgets.iter().find_map(|w| match w {
            Widget::Button {
                action: Action::Command(Command::DeepenMine { .. }),
                at: place,
                enabled,
                tooltip,
                ..
            } => Some((*place, *enabled, tooltip.clone().unwrap_or_default())),
            _ => None,
        })
    };
    let (place, enabled, tip) = deepen(&world, at).expect("the button");
    assert_eq!(place, [307.0, 157.0]);
    assert!(!enabled);
    assert!(tip.contains("Deepened 0 of 2"), "{tip}");
    let (plain, plain_at) = hut_game_with(|_| {});
    assert_eq!(deepen(&plain, plain_at), None);
}

/// UI-GAME-9.4: a click selects a ship only by its model under the pointer,
/// never by its field, and opens its window only on a field of build class 0;
/// a flag, building or road on the field comes first.
// spec: UI-GAME-9.4, UI-GAME-4.5
#[test]
fn a_click_selects_a_ship_by_its_model_on_a_field_of_class_0() {
    // The land is player 0's; the ship player 1's.
    let mut world = flat_world(20, 20);
    let (owner, other) = (PlayerId(1), PlayerId(0));
    let a = world.debug_add_ship(owner, (8, 8));
    world.debug_launch_ship(a);
    let at = world.ships()[a].position();
    let (x, y) = at;
    // Its field alone does not find it.
    assert_eq!(context_for_click(&world, owner, at, None), None);
    // Its model under the pointer, over a field where nothing can be placed
    // for this player (class 0: not his land): the ship window, whichever
    // field.
    assert_ne!(world.field_owner((x + 3, y)), Some(owner));
    assert_eq!(
        context_for_click(&world, owner, (x + 3, y), Some(a)),
        Some(Context::Ship(a))
    );
    // Not the player's ship: nothing.
    let mine = context_for_click(&world, other, (x + 3, y), Some(a));
    assert!(!matches!(mine, Some(Context::Ship(_))), "{mine:?}");
    // The field named by the game (the tutorial's ship step): by its field.
    assert_eq!(context_for(&world, owner, at), Some(Context::Ship(a)));
    assert_eq!(context_for(&world, owner, (x + 1, y)), None);
}

/// UI-GAME-9.4: over a field of class 1 or more the road or build menu opens
/// whatever ship was picked.
// spec: UI-GAME-9.4
#[test]
fn a_ship_over_a_buildable_field_opens_the_build_menu() {
    // Player 0's land, where a flag fits (class 1).
    let mut world = flat_world(20, 20);
    let player = PlayerId(0);
    let a = world.debug_add_ship(player, (8, 8));
    world.debug_launch_ship(a);
    let field = (12, 12);
    assert_eq!(world.field_owner(field), Some(player));
    assert_ne!(world.placement_at(field), Placement::Nothing);
    let opened = context_for_click(&world, player, field, Some(a));
    assert_eq!(opened, context_for_click(&world, player, field, None));
    assert!(
        matches!(opened, Some(Context::Build { .. } | Context::Road(_))),
        "{opened:?}"
    );
}

/// A headquarters and a finished harbour of player 0, with its cost.
fn harbour_game() -> (World, Point) {
    use s2_sim::{BuildingSize, BuildingType};
    let n = 40 * 40;
    let states = vec![s2_sim::map::state::FOR_BUILDING | s2_sim::map::state::FOR_HARBOR; n];
    let terrain = s2_sim::map::Terrain {
        width: 40,
        height: 40,
        heights: vec![0; n],
        height_floor: -100,
        states,
        samples: Vec::new(),
    };
    let mut castle = BuildingType::new(9, BuildingSize::Large);
    castle.territory_range = 9;
    castle.warehouse = true;
    castle.worker_limit = 0;
    let mut harbour = BuildingType::new(4, BuildingSize::Harbor);
    harbour.warehouse = true;
    harbour.worker_limit = 0;
    harbour.frame_costs = vec![(0x10, 4)];
    harbour.costs = vec![(0x12, 6)];
    let rules = s2_sim::Rules {
        building_types: vec![castle, harbour],
        headquarters: Some(9),
        roles: s2_sim::Roles {
            builder: Some(0x20),
            ..Default::default()
        },
        ..s2_sim::Rules::empty()
    };
    let setup = s2_sim::GameSetup {
        players: vec![(PlayerId(0), (10, 10))],
        local_player: PlayerId(0),
        ..Default::default()
    };
    let mut world = World::new_game(1, terrain, rules, &setup);
    let at = (16, 12);
    world.submit(Command::CreateBuilding {
        player: PlayerId(0),
        building: 4,
        at,
        settler: None,
    });
    world.step();
    assert!(world.harbour_at(at).is_some(), "a finished harbour");
    (world, at)
}

/// UI-GAME-4.4, 8.1a (lesson 4 stopped here): a harbour's window has the tabs 9,
/// 10 and 12, the headquarters' 9, 10 and 11. The expedition page has four
/// slots at x stepping 54 from (23, 60), builder n/1, stones n/6, planks n/4 and
/// the ship 0/1 (each `-/-` before a preparation, seen in E-trace-030), with
/// no click action; Exploration at (298, 386) starts the
/// preparation (SHIP-5.1) and is disabled while it runs, with its tooltip
/// (SHIP-5.1a: nothing stops it); the jump at (248, 386) is off without a ship
/// in expedition mode and goes to the nearest one.
// spec: UI-GAME-4.4, UI-GAME-8.1a, UI-GAME-3.5, SHIP-5.1, SHIP-5.1a, SHIP-5.2
#[test]
fn a_harbour_has_the_expedition_page() {
    let (mut world, at) = harbour_game();
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let tabs = |world: &World, at: Point| -> Vec<u8> {
        let texts = Texts::parse("");
        let catalog = Catalog::default();
        depot_window(&ctx(world, &texts, &catalog), at, 9)
            .buttons()
            .filter_map(|(a, ..)| match a {
                Action::BuildingTab(t) => Some(*t),
                _ => None,
            })
            .collect()
    };
    assert_eq!(tabs(&world, at), [9, 10, 12]);
    assert_eq!(tabs(&world, (10, 10)), [9, 10, 11]);
    // No page 12 on the headquarters: its first page instead.
    let hq = depot_window(&ctx(&world, &texts, &catalog), (10, 10), 12);
    assert!(hq
        .buttons()
        .all(|(a, ..)| !matches!(a, Action::Command(Command::ExpeditionStart { .. }))));
    assert_eq!(
        world.expedition_goods(at),
        [
            s2_sim::ExpeditionGood {
                good: 0x10,
                required: 4,
                aside: 0
            },
            s2_sim::ExpeditionGood {
                good: 0x12,
                required: 6,
                aside: 0
            },
            s2_sim::ExpeditionGood {
                good: 0x20,
                required: 1,
                aside: 0
            },
        ]
    );
    let page = |world: &World| {
        let texts = Texts::parse("");
        let catalog = Catalog::default();
        depot_window(&ctx(world, &texts, &catalog), at, 12)
    };
    let p = page(&world);
    let labels: Vec<([f32; 2], String)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, at, .. } if text.contains('/') => Some((*at, text.clone())),
            _ => None,
        })
        .collect();
    let texts_of: Vec<&str> = labels.iter().map(|(_, t)| t.as_str()).collect();
    // Seen (E-trace-030): before a preparation every slot reads -/-.
    assert_eq!(texts_of, ["-/-", "-/-", "-/-", "-/-"]);
    for (i, (place, _)) in labels.iter().enumerate() {
        assert_eq!(place[0], 23.0 + 54.0 * i as f32 + 28.0);
    }
    // The slots have no click action.
    let slots: Vec<_> = p
        .buttons()
        .filter(|(_, at, ..)| at[1] == 60.0)
        .map(|(a, ..)| a.clone())
        .collect();
    assert_eq!(slots, vec![Action::None; 4]);
    let start = |p: &Panel| {
        p.buttons()
            .find(|(a, ..)| matches!(a, Action::Command(Command::ExpeditionStart { .. })))
            .map(|(a, at, enabled, tip)| (a.clone(), at, enabled, tip.map(str::to_owned)))
            .expect("the start button")
    };
    let jump = |p: &Panel| {
        p.buttons()
            .find(|(_, at, ..)| *at == [248.0, 386.0])
            .map(|(a, _, enabled, tip)| (a.clone(), enabled, tip.map(str::to_owned)))
            .expect("the jump")
    };
    let (action, place, enabled, _) = start(&p);
    assert_eq!(place, [298.0, 386.0]);
    assert!(enabled);
    let (_, enabled, tip) = jump(&p);
    assert!(!enabled);
    assert!(tip.expect("a tooltip").contains("No expedition started"));
    let Action::Command(command) = action else {
        unreachable!()
    };
    world.submit(command);
    world.step();
    assert_eq!(
        world.harbour_at(at).expect("the harbour").expedition,
        s2_sim::ExpeditionState::Gathering
    );
    let p = page(&world);
    // While it is prepared, each slot counts what is set aside.
    let counts: Vec<String> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. } if text.contains('/') => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(counts, ["0/1", "0/6", "0/4", "0/1"]);
    let (_, _, enabled, tip) = start(&p);
    assert!(!enabled, "disabled while it is prepared");
    assert!(tip.expect("a tooltip").contains("Exploration is running"));
    // A ship not on an expedition is no target.
    let plain = world.debug_add_ship(PlayerId(0), (at.0 + 1, at.1));
    world.debug_launch_ship(plain);
    assert_eq!(nearest_expedition(&world, PlayerId(0), at), None);
    // Two ships in expedition mode (as `createShip` makes them): the jump goes
    // to the nearer, every time.
    for field in [(30, 30), (at.0 + 3, at.1)] {
        world.submit(Command::CreateShip {
            player: PlayerId(0),
            at: field,
        });
    }
    world.step();
    let (far, near) = (plain + 1, plain + 2);
    assert!(world.ships()[far].vessel.expedition && world.ships()[near].vessel.expedition);
    assert_eq!(nearest_expedition(&world, PlayerId(0), at), Some(near));
    let (action, enabled, tip) = jump(&page(&world));
    assert!(enabled);
    assert!(tip.expect("a tooltip").contains("Jump to next expedition"));
    assert_eq!(action, Action::JumpToShip(near));
}

/// UI-GAME-4.3 (Run B; frame F59, E-ui-033): the military window is the
/// picture `uiMedium2Dialog` (8, 8, 480 x 272) drawn 1:1; its four buttons
/// stand in a row at y 212 from x 407 leftward (destroy, coins, move out,
/// upgrade); the building's picture, two separators and the status lines in
/// `light_system_outlined`.
// spec: UI-GAME-4.3, UI-GAME-1.8
#[test]
fn military_window_layout() {
    let (mut world, at) = guard_house_game();
    let texts = Texts::parse(
        "!Coin supply stopped.\tCoin supply stopped\n!Soldiers moving out.\tBuilding evacuated\n",
    );
    let catalog = Catalog::default();
    let p = military_window(&ctx(&world, &texts, &catalog), at);
    let place = |key: &str| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Button { at, tooltip, .. } if tooltip.as_deref() == Some(key) => Some(*at),
            _ => None,
        })
    };
    assert_eq!(place("!Destroy the building."), Some([407.0, 212.0]));
    assert_eq!(place("!Toggle coin supply ON/OFF."), Some([357.0, 212.0]));
    assert_eq!(place("!Toggle Moving soldiers out."), Some([307.0, 212.0]));
    assert_eq!(place("!Upgrade not possible"), Some([257.0, 212.0]));
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { atlas: Atlas::Medium2Dialog, rect: [8, 8, 480, 272], at, size: Some(size), .. }
            if *at == [0.0, 0.0] && *size == [480.0, 272.0]
    )));
    let separators: Vec<[f32; 2]> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Sprite {
                atlas: Atlas::Elements3,
                rect: [160, 0, 8, 56],
                at,
                size: Some([8.0, 176.0]),
                ..
            } => Some(*at),
            _ => None,
        })
        .collect();
    assert_eq!(separators, [[123.0, 32.0], [300.0, 32.0]]);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Sprite { rect: [344, 216, 88, 88], at, .. } if *at == [22.0, 43.0]
    )));
    // The status lines: coins at y 220, move out at 236.
    world.submit(Command::ToggleCoinSupply {
        player: PlayerId(0),
        at,
    });
    world.submit(Command::ToggleMoveOut {
        player: PlayerId(0),
        at,
    });
    world.step();
    let p = military_window(&ctx(&world, &texts, &catalog), at);
    let line = |text: &str| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Text {
                text: t,
                font: Font::Outlined,
                at,
                ..
            } if t == text => Some(*at),
            _ => None,
        })
    };
    assert_eq!(line("Coin supply stopped"), Some([20.0, 220.0]));
    assert_eq!(line("Building evacuated"), Some([20.0, 236.0]));
}

/// UI-GAME-2.7: no indicator at speed 1; at speed 2 and 3 the running
/// settler of row 640 / 704 at (W - 76, 20), (W - 382, 20) with the
/// minimap, its frame by the second clock's seconds (real time, TOOL-1.3).
// spec: UI-GAME-2.7, TOOL-1.3, TOOL-trace TRACE-5.3
#[test]
fn speed_indicator_place() {
    let world = flat_world(10, 10);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    assert!(speed_indicator(&ctx(&world, &texts, &catalog), false).is_none());
    // The second clock at 7.3 s, the world not stepped (as while paused).
    let mut c = ctx(&world, &texts, &catalog);
    c.speed = 3;
    c.real_seconds = 7.3;
    let p = speed_indicator(&c, false).expect("shown at speed 3");
    assert_eq!(p.at, [1024.0 - 76.0, 20.0]);
    assert_eq!(
        p.widgets[0],
        Widget::Sprite {
            atlas: Atlas::Game,
            rect: [64, 704, 64, 64],
            at: [0.0, 0.0],
            size: Some([50.0, 50.0]),
            greyed: false,
        }
    );
    c.speed = 2;
    assert_eq!(
        speed_indicator(&c, true).unwrap().at,
        [1024.0 - 382.0, 20.0]
    );
}

/// UI-GAME-1.9: a context dialog's bottom-right corner on the click, x
/// at most screenWidth - 600, kept on the screen; (20, 20) in the
/// tutorial; global dialogs centred; the places seen in the original,
/// the `,` key's window among them (UI-GAME-4.4).
// spec: UI-GAME-1.9, UI-GAME-4.4
#[test]
fn dialog_placement() {
    let world = flat_world(10, 10);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let mut c = ctx(&world, &texts, &catalog);
    c.click = Some([300.0, 500.0]);
    assert_eq!(context_origin(&c, [200.0, 100.0]), [100.0, 400.0]);
    c.click = Some([1000.0, 700.0]);
    assert_eq!(context_origin(&c, [200.0, 100.0]), [424.0, 600.0]);
    c.click = Some([50.0, 20.0]);
    assert_eq!(context_origin(&c, [200.0, 100.0]), [0.0, 0.0]);
    c.click = Some([600.0, 760.0]);
    assert_eq!(context_origin(&c, [200.0, 50.0])[1], 0.85 * 768.0);
    c.tutorial = true;
    assert_eq!(context_origin(&c, [200.0, 100.0]), [20.0, 20.0]);
    assert_eq!(global_origin(&c, [400.0, 300.0]), [312.0, 234.0]);
    // Seen (E-trace-029, 2560 x 1440): a producer window (480 x 224) clicked
    // at (1440, 200) opened at (960, 0); a dialog clicked at (2450, 236) at
    // x 1960 (W - 600); a flag menu (304 x 96) clicked at (1640, 1318) with
    // its top at about 1224 (0.85 H, here 1222 by the rule).
    c.tutorial = false;
    c.screen = [2560.0, 1440.0];
    c.click = Some([1440.0, 200.0]);
    assert_eq!(context_origin(&c, [480.0, 224.0]), [960.0, 0.0]);
    c.click = Some([2450.0, 236.0]);
    assert_eq!(context_origin(&c, [480.0, 224.0])[0], 1960.0);
    c.click = Some([1640.0, 1318.0]);
    let y = context_origin(&c, [304.0, 96.0])[1];
    assert!((y - 0.85 * 1440.0).abs() <= 2.0, "{y}");
    // UI-GAME-4.4 (E-ui-038): the `,` key's 376 x 464 depot window placed
    // by the pointer at (2400, 1300): at (min(2400 - 376, W - 600), 1300 -
    // 464) = (1960, 836), as seen.
    c.click = Some([2400.0, 1300.0]);
    assert_eq!(context_origin(&c, [376.0, 464.0]), [1960.0, 836.0]);
}

/// UI-GAME-8.5d: the buildings page: a cell per building type from
/// (23, 62), 79 x 77, six per row, small types first; the counts of
/// finished buildings and sites at (+6, +10), a worker type's mean
/// productivity right-aligned at (+70, +64), grey at 0; moved 10 px right
/// of the family's place; its tab opens it.
// spec: UI-GAME-8.5d, UI-GAME-8.5
#[test]
fn buildings_page() {
    let n = 30 * 30;
    let terrain = s2_sim::map::Terrain {
        width: 30,
        height: 30,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut hut = s2_sim::BuildingType::new(7, s2_sim::BuildingSize::Small);
    hut.worker = Some(70);
    let house = s2_sim::BuildingType::new(8, s2_sim::BuildingSize::Medium);
    let rules = s2_sim::Rules {
        building_types: vec![hut, house],
        ..s2_sim::Rules::empty()
    };
    let mut world = World::new_game(1, terrain, rules, &s2_sim::GameSetup::default());
    let building = |player, at, remaining| s2_sim::RestoredBuilding {
        player: PlayerId(player),
        building_type: 7,
        at,
        remaining,
        levelled: 1.0,
        delivered: Vec::new(),
        production_on: true,
        stock: Vec::new(),
        worker: false,
        number: None,
        worker_number: None,
        soldiers: Vec::new(),
    };
    world.restore(&s2_sim::Restore {
        players: vec![PlayerId(0), PlayerId(1)],
        tick: 0,
        owners: Vec::new(),
        explored: Vec::new(),
        buildings: vec![
            building(0, (5, 5), None),
            building(0, (9, 5), None),
            building(0, (13, 5), Some([1.0, 1.0])),
            building(1, (17, 5), None),
        ],
        flags: Vec::new(),
        roads: Vec::new(),
        ..Default::default()
    });
    let entry = |id, size, sort, production| crate::ui::game::BuildingEntry {
        id,
        name: format!("T{id}"),
        size,
        icon: Some((id, 0)),
        military: false,
        headquarters: false,
        sort,
        has_cost: true,
        production,
    };
    let catalog = Catalog {
        buildings: vec![
            entry(8, s2_sim::BuildingSize::Medium, 0, false),
            entry(7, s2_sim::BuildingSize::Small, 0, true),
            // No cost (the headquarters): no cell.
            crate::ui::game::BuildingEntry {
                has_cost: false,
                ..entry(9, s2_sim::BuildingSize::Large, 0, false)
            },
            // A harbour after a large type, though its sort is lower.
            entry(11, s2_sim::BuildingSize::Harbor, -1, false),
            entry(10, s2_sim::BuildingSize::Large, 5, false),
            // The same class and sort as 7, later in id order: takes its cell.
            entry(12, s2_sim::BuildingSize::Small, 0, true),
        ],
        ..Catalog::default()
    };
    let order: Vec<u32> = buildings_page_types(&catalog)
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(order, [12, 8, 10, 11]);
    let catalog = Catalog {
        buildings: catalog.buildings[..2].to_vec(),
        ..Catalog::default()
    };
    let texts = Texts::default();
    // Before the first tally: 0-0 and -%.
    let p = buildings_dialog(&ctx(&world, &texts, &catalog));
    assert!(p
        .widgets
        .iter()
        .any(|w| matches!(w, Widget::Text { text, .. } if text == "-%")));
    let tally = building_counts(&world, PlayerId(0));
    let mut c = ctx(&world, &texts, &catalog);
    c.building_tally = Some(&tally);
    let p = buildings_dialog(&c);
    // 1024 x 768: the family's place (256, 116), 10 px right.
    assert_eq!(p.at, [266.0, 116.0]);
    let cells: Vec<[f32; 2]> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button { rect, at, .. } if *rect == [0, 0, 80, 80] => Some(*at),
            _ => None,
        })
        .collect();
    assert_eq!(cells, [[23.0, 62.0], [102.0, 62.0]]);
    let labels: Vec<(String, [f32; 2], [u8; 3])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text {
                text, at, colour, ..
            } if text.chars().next().is_some_and(|c| c.is_ascii_digit()) => {
                Some((text.clone(), *at, *colour))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        labels,
        [
            ("2-1".into(), [29.0, 72.0], WHITE),
            ("0%".into(), [93.0, 126.0], [150, 150, 150]),
            ("0-0".into(), [108.0, 72.0], WHITE),
        ]
    );
    let tabs = family_tab_row(&p);
    assert_eq!(tabs[4].2, Action::OpenGlobal(Global::Buildings));
    assert!(tabs[4].3);
}

/// UI-GAME-12.1 (. and ,; Q-258 b): the next building of the kind after the
/// key's cursor in the player's building list (creation order), wrapping;
/// with no cursor from the player's first building; SHIFT the one before;
/// only finished buildings count; a cursor whose building is gone gives
/// nothing (the press clears it).
// spec: UI-GAME-12.1
#[test]
fn period_and_comma_jumps() {
    let n = 40 * 40;
    let terrain = s2_sim::map::Terrain {
        width: 40,
        height: 40,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut store = s2_sim::BuildingType::new(5, s2_sim::BuildingSize::Medium);
    store.warehouse = true;
    store.worker_limit = 0;
    let mut tower = s2_sim::BuildingType::new(6, s2_sim::BuildingSize::Medium);
    tower.soldier_limit = 2;
    tower.worker_limit = 0;
    let rules = s2_sim::Rules {
        building_types: vec![store, tower],
        ..s2_sim::Rules::empty()
    };
    let mut world = World::new_game(1, terrain, rules, &s2_sim::GameSetup::default());
    let building = |building_type, at, remaining| s2_sim::RestoredBuilding {
        player: PlayerId(0),
        building_type,
        at,
        remaining,
        levelled: 1.0,
        delivered: Vec::new(),
        production_on: true,
        stock: Vec::new(),
        worker: false,
        number: None,
        worker_number: None,
        soldiers: Vec::new(),
    };
    world.restore(&s2_sim::Restore {
        players: vec![PlayerId(0)],
        tick: 0,
        owners: Vec::new(),
        explored: Vec::new(),
        buildings: vec![
            building(6, (20, 6), None),
            building(5, (6, 6), None),
            building(6, (10, 16), None),
            building(6, (30, 16), Some([1.0, 1.0])),
            building(5, (30, 30), None),
        ],
        flags: Vec::new(),
        roads: Vec::new(),
        ..Default::default()
    });
    let p = PlayerId(0);
    // No cursor: from the first building (the tower at (20, 6)) on, so the
    // next tower; the first comes again after the wrap.
    let (b, s2) = next_jump(&world, p, true, None, false).unwrap();
    assert_eq!(b, (10, 16));
    let (a, s1) = next_jump(&world, p, true, Some(s2), false).unwrap();
    assert_eq!(a, (20, 6));
    assert_eq!(next_jump(&world, p, true, Some(s1), false).unwrap().1, s2);
    // The site does not count: back to the first.
    assert_eq!(
        next_jump(&world, p, true, Some(s2), false).unwrap().0,
        (20, 6)
    );
    assert_eq!(
        next_jump(&world, p, true, Some(s1), true).unwrap().0,
        (10, 16)
    );
    assert_eq!(next_jump(&world, p, false, None, false).unwrap().0, (6, 6));
    assert_eq!(next_jump(&world, p, false, None, true).unwrap().0, (30, 30));
    assert!(next_jump(&world, PlayerId(1), false, None, false).is_none());
    // A cursor whose building is gone: nothing.
    assert!(next_jump(&world, p, true, Some(9999), false).is_none());
}

/// UI-GAME-12.1 (Q-258 e answered): the chat line is a box of 200 x 32 at
/// (15, 275), black at about half opacity (0.49 of the picture kept), no
/// frame and no text; empty, it shows a white cursor 1 x 10 at x 47, y 284.
// spec: UI-GAME-12.1
#[test]
fn the_chat_line_is_a_half_black_box_with_a_cursor() {
    let world = flat_world(12, 12);
    let texts = Texts::default();
    let catalog = Catalog::default();
    let p = chat_line(&ctx(&world, &texts, &catalog), "");
    assert_eq!(p.at, [15.0, 275.0]);
    let fills: Vec<([f32; 2], [f32; 2], [f32; 4])> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Fill { at, size, rgba } => Some((*at, *size, *rgba)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        [
            ([0.0, 0.0], [200.0, 32.0], [0.0, 0.0, 0.0, 0.51]),
            ([32.0, 9.0], [1.0, 10.0], [1.0, 1.0, 1.0, 1.0]),
        ]
    );
    assert_eq!(p.widgets.len(), 2, "no frame, title or prompt");
    assert_eq!(
        [p.at[0] + fills[1].0[0], p.at[1] + fills[1].0[1]],
        [47.0, 284.0]
    );
}

/// UI-GAME-8.5f (Q-1016 answered): at the statistics page's first opening
/// in a game every player's portrait is pressed, every curve drawn.
// spec: UI-GAME-8.5f
#[test]
fn the_statistics_page_opens_with_every_portrait_pressed() {
    assert_eq!(GameUi::default().statistics_hidden, [false; 8]);
}

/// UI-GAME-8.5 (T58, F61): on 2560 x 1440 every statistics page's frame has
/// its top-left at (1024, 452): the window does not move when the tab
/// changes; the statistics, production, inventory and fleet pages draw the
/// tabbed `uiDepot` 376 x 464, the buildings page `uiXXLDialog_tabbed` (4,
/// 8, 512 x 536) 1:1 (UI-GAME-8.5d); the 7 and 10 px shifts move the
/// content (the production and buildings pages' panels), not the frame.
// spec: UI-GAME-8.5 (T58), UI-GAME-8.5d
#[test]
fn t58_the_statistics_pages_keep_their_corner() {
    let world = flat_world(20, 20);
    let texts = Texts::default();
    let catalog = Catalog::default();
    let mut c = ctx(&world, &texts, &catalog);
    c.screen = [2560.0, 1440.0];
    let frame = |p: &Panel| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Sprite {
                atlas: a @ (Atlas::Depot | Atlas::XxlDialogTabbed),
                at,
                size,
                rect,
                ..
            } => Some((
                *a,
                [p.at[0] + at[0], p.at[1] + at[1]],
                size.unwrap_or([rect[2] as f32, rect[3] as f32]),
            )),
            _ => None,
        })
    };
    let pages = [
        (statistics_dialog(&c, 0), 0.0, Atlas::Depot),
        (production_dialog(&c), 7.0, Atlas::Depot),
        (inventory_dialog(&c, false), 0.0, Atlas::Depot),
        (inventory_dialog(&c, true), 0.0, Atlas::Depot),
        (buildings_dialog(&c), 10.0, Atlas::XxlDialogTabbed),
        (fleet_dialog(&c), 0.0, Atlas::Depot),
    ];
    for (i, (p, shift, atlas)) in pages.iter().enumerate() {
        let (a, corner, size) = frame(p).expect("a frame");
        assert_eq!((a, corner), (*atlas, [1024.0, 452.0]), "page {i}");
        let want = if *atlas == Atlas::Depot {
            [376.0, 464.0]
        } else {
            [512.0, 536.0]
        };
        assert_eq!(size, want, "page {i}");
        assert_eq!(p.at, [1024.0 + shift, 452.0], "page {i}");
    }
}

/// UI-GAME-6 (corrected 2026-11-15 night, E-ui-036, E-trace-028): on 2560 x
/// 1440 the four economy dialogs open centred at (1096, 468) on the frame
/// `uiDepot_long` (8, 8, 368 x 503); the transport page's background
/// `ui_detail_transport` (8, 8, 360 x 496) lies over the frame.
// spec: UI-GAME-6
#[test]
fn the_economy_dialogs_open_centred_on_their_long_frame() {
    let world = flat_world(20, 20);
    let texts = Texts::default();
    let catalog = Catalog::default();
    let mut c = ctx(&world, &texts, &catalog);
    c.screen = [2560.0, 1440.0];
    for p in [tools_dialog(&c), military_dialog(&c), transport_dialog(&c)] {
        assert_eq!(p.at, [1096.0, 468.0]);
        assert!(matches!(
            p.widgets[0],
            Widget::Sprite {
                atlas: Atlas::DepotLong,
                rect: [8, 8, 368, 503],
                ..
            }
        ));
    }
    let t = transport_dialog(&c);
    assert!(matches!(
        t.widgets[1],
        Widget::Sprite {
            atlas: Atlas::DetailTransport,
            ..
        }
    ));
}

/// UI-GAME-4.4 (the military page as the original shows it, E-trace-028):
/// five rows 65 px apart: the name at (50, 90 + 65 k), a slider from x 50,
/// 200 wide, under it, the count right-aligned on x 250 on the name's line
/// and a framed 48 x 48 box at (260, 82 + 65 k). Q-1053 answered
/// (E-trace-029): the names are the ranks' settler names, the count n/m the
/// soldiers held now (at most m) over the reserve m.
// spec: UI-GAME-4.4 (T59, the page's parts; the click is the slider's)
#[test]
fn the_headquarters_military_page_has_five_slider_rows() {
    let (world, _) = harbour_game();
    let texts = Texts::parse(
        "dlg_PrivateSettler\tPrivate\ndlg_PrivateFirstClassSettler\tCorporal\n\
         dlg_SergeantSettler\tSergeant\ndlg_OfficerSettler\tOfficer\n\
         dlg_GeneralSettler\tGeneral\n",
    );
    let catalog = Catalog::default();
    let p = depot_window(&ctx(&world, &texts, &catalog), (10, 10), 11);
    let sliders: Vec<([f32; 2], f32, u8)> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Slider {
                at,
                width,
                kind: SliderKind::Castle(rank),
                ..
            } => Some((*at, *width, *rank)),
            _ => None,
        })
        .collect();
    assert_eq!(sliders.len(), 5);
    for (k, (at, width, rank)) in sliders.into_iter().enumerate() {
        assert_eq!((at[0], width, usize::from(rank)), (50.0, 200.0, k));
        assert_eq!(
            at[1],
            90.0 + 65.0 * k as f32 + 8.0,
            "the 20 px bar's middle on 108 + 65 k (UI-GAME-6.5)"
        );
    }
    let boxes = p
        .widgets
        .iter()
        .filter(|w| {
            matches!(w, Widget::Sprite { atlas: Atlas::Elements3, rect: [440, 216, 48, 48], at, .. }
                if at[0] == 260.0 && (0..5).any(|k| at[1] == 82.0 + 65.0 * k as f32))
        })
        .count();
    assert_eq!(boxes, 5);
    let counts: Vec<&str> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text {
                align: Align::TopRight,
                at,
                text,
                ..
            } if at[0] == 250.0 => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let hq = world.military_at((10, 10)).expect("the headquarters");
    let stock = world.stock_soldiers((10, 10));
    let want: Vec<String> = (0..5)
        .map(|k| format!("{}/{}", stock[k].min(hq.castle[k]), hq.castle[k]))
        .collect();
    assert_eq!(counts, want);
    let names: Vec<&str> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text {
                align: Align::TopLeft,
                at,
                text,
                ..
            } if at[0] == 50.0 && (0..5).any(|k| at[1] == 90.0 + 65.0 * k as f32) => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        ["Private", "Corporal", "Sergeant", "Officer", "General"]
    );
}

/// UI-GAME-2.5 (with the measures of E-trace-028 and E-trace-030): the
/// minimap dialog's frame at (W - 326, 10), the map 265 x 265 at (+30, +36)
/// of it, the borders button centred at (W - 28, 292), the close cross at
/// (W - 17, 31).
// spec: UI-GAME-2.5
#[test]
fn the_minimap_dialog_stands_at_the_right_edge() {
    let world = flat_world(10, 10);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let mut c = ctx(&world, &texts, &catalog);
    c.screen = [2560.0, 1440.0];
    let p = crate::ui::game::hud::minimap(&c, false);
    assert_eq!(p.at, [2234.0, 10.0]);
    assert!(p.widgets.iter().any(|w| matches!(
        w,
        Widget::Picture { at, size, .. } if *at == [30.0, 36.0] && *size == 265.0
    )));
    // Measured (E-trace-030): the borders button centred at (w - 28, 292),
    // the close cross at (w - 17, 31).
    let centre = |action: Action, side: f32| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Button { action: a, at, .. } if *a == action => {
                Some([p.at[0] + at[0] + side / 2.0, p.at[1] + at[1] + side / 2.0])
            }
            _ => None,
        })
    };
    assert_eq!(centre(Action::Hud(20), 48.0), Some([2560.0 - 28.0, 292.0]));
    assert_eq!(centre(Action::Hud(5), 24.0), Some([2560.0 - 17.0, 31.0]));
}

/// Hook W19 (docs/extending.md "Paved roads"): with a paving rule the road
/// menu of a road of the player's has the pave button, usable while the
/// command would be accepted (no warehouse is needed to order it); without
/// the rule it has none.
// spec: none (hook W19; the menu is UI-GAME-3.3's)
#[test]
fn a_paving_rule_adds_the_pave_button_to_the_road_menu() {
    let road_world = |paving: Option<s2_sim::PavingRule>| {
        let n = 10 * 10;
        let terrain = s2_sim::map::Terrain {
            width: 10,
            height: 10,
            heights: vec![0; n],
            height_floor: -100,
            states: vec![0; n],
            samples: Vec::new(),
        };
        let rules = s2_sim::Rules {
            paving,
            ..s2_sim::Rules::empty()
        };
        let mut world = World::with_map(1, terrain, rules, PlayerId(0));
        for at in [(2, 2), (4, 2)] {
            world.submit(Command::PlaceFlag {
                player: PlayerId(0),
                at,
            });
        }
        world.step();
        world.submit(Command::BuildRoad {
            player: PlayerId(0),
            from: (2, 2),
            to: (4, 2),
        });
        world.step();
        world
    };
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let rule = s2_sim::PavingRule {
        good: 2,
        per_field: 1,
        speed_percent: 130,
    };
    let world = road_world(Some(rule));
    let p = road_menu(&ctx(&world, &texts, &catalog), (3, 2));
    let pave = Action::Command(Command::PaveRoad {
        player: PlayerId(0),
        at: (3, 2),
    });
    assert!(
        p.buttons().any(|b| *b.0 == pave),
        "{:?}",
        p.buttons().collect::<Vec<_>>()
    );
    let plain = road_world(None);
    let p = road_menu(&ctx(&plain, &texts, &catalog), (3, 2));
    assert!(!p.buttons().any(|b| *b.0 == pave));
}

/// UI-GAME-11.4 (confirmed, E-trace-030): a row's icon takes the left click
/// (marks the message read and jumps), its text does nothing on a left
/// click; a read row shows the teal frame with the tick (`uiElements3`
/// (168, 344)) over its icon and its text in grey; an unread row neither.
// spec: UI-GAME-11.4
#[test]
fn a_read_message_shows_the_tick_and_grey_text() {
    let world = flat_world(10, 10);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let posted = [message(10, "Building constructed")];
    let mut c = ctx(&world, &texts, &catalog);
    c.posted_messages = &posted;
    let key: MessageKey = (10, (3, 3), "Building constructed");
    let row = |ui: &GameUi| {
        let p = crate::ui::game::hud::messages(&c, ui);
        let icon = p.widgets.iter().any(
            |w| matches!(w, Widget::Button { action: Action::MessageRow(k), .. } if *k == key),
        );
        let text = p.widgets.iter().find_map(|w| match w {
            Widget::TextButton {
                action: Action::MessageText(k),
                colour,
                ..
            } if *k == key => Some(*colour),
            _ => None,
        });
        let tick = p.widgets.iter().any(|w| {
            matches!(w, Widget::Sprite { atlas: Atlas::Elements3, rect, .. } if *rect == [168, 344, 28, 28])
        });
        (icon, text, tick)
    };
    let mut ui = GameUi::default();
    assert_eq!(row(&ui), (true, Some([255, 255, 255]), false));
    ui.read_messages.insert(key);
    assert_eq!(row(&ui), (true, Some([120, 120, 120]), true));
}

/// UI-GAME-3.2 (seen, E-trace-030): a build menu cell's tooltip is the
/// type's name with the player's count of it in brackets.
// spec: UI-GAME-3.2
#[test]
fn a_build_cell_names_the_type_and_the_count() {
    assert_eq!(
        crate::ui::game::context::build_cell_tooltip("Woodcutter's Hut", 0),
        "Woodcutter's Hut (0)"
    );
    assert_eq!(
        crate::ui::game::context::build_cell_tooltip("Quarry", 2),
        "Quarry (2)"
    );
}

/// UI-GAME-2.2 (seen, E-trace-030): a control's tooltip box has its top-left
/// corner at the pointer + (25, -45), is moved left to end at the screen's
/// right edge, and stays on the screen at the top.
// spec: UI-GAME-2.2
#[test]
fn a_control_tooltip_sits_above_right_of_the_pointer() {
    let screen = [2560.0, 1440.0];
    assert_eq!(
        control_tooltip_at([1000.0, 500.0], [200.0, 32.0], screen),
        [1025.0, 455.0]
    );
    assert_eq!(
        control_tooltip_at([2550.0, 31.0], [180.0, 32.0], screen),
        [2380.0, 0.0]
    );
    assert_eq!(CONTROL_TOOLTIP_HEIGHT, 32.0);
}

/// UI-GAME-4.5 (b), (e), (f) (seen again in E-trace-030): the ship window's
/// twelve cargo slots from (16, 255),
/// six per row 56 px apart, the rows 63 px apart; Jump to port enabled
/// while the ship has a port (a ship a script made has its first land
/// neighbour, SHIP-2.7), disabled with `!Ship has no target port.` while it
/// has none; in expedition mode Cancel at (8, 176) and, while it waits for
/// orders, the six compass buttons around the porthole with their tooltips
/// (East at (278, 118) .. NorthEast at (240, 46)); Build harbour only while
/// SHIP-7.1 holds (none here: the map has no harbour spot; Q-1060); the two
/// separators `uiElements3` (0, 432, 350 x 8) at (2, 232) and (2, 376)
/// (Q-1059).
// spec: UI-GAME-4.5, SHIP-2.7, CAM-5.3
#[test]
fn the_ship_window_as_the_spec_lays_it_out() {
    let (mut world, _) = harbour_game();
    world.submit(Command::CreateShip {
        player: PlayerId(0),
        at: (30, 30),
    });
    world.step();
    let ship = world.ships().len() - 1;
    assert!(world.ships()[ship].vessel.expedition);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let p = ship_window(&ctx(&world, &texts, &catalog), ship).unwrap();
    let slots: Vec<[f32; 2]> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Sprite {
                atlas: Atlas::Elements3,
                rect,
                at,
                ..
            } if *rect == [440, 216, 48, 48] => Some(*at),
            _ => None,
        })
        .collect();
    assert_eq!(slots.len(), 12);
    assert_eq!(
        (slots[0], slots[5], slots[6]),
        ([16.0, 255.0], [296.0, 255.0], [16.0, 318.0])
    );
    // (b) the two separators: the separator line of uiElements3, 350 wide
    // (Q-1059).
    let separators: Vec<[f32; 2]> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Sprite {
                atlas: Atlas::Elements3,
                rect: [0, 432, 350, 8],
                at,
                ..
            } => Some(*at),
            _ => None,
        })
        .collect();
    assert_eq!(separators, [[2.0, 232.0], [2.0, 376.0]]);
    let button = |pred: &dyn Fn(&Action) -> bool| {
        p.buttons()
            .find(|(a, ..)| pred(a))
            .map(|(_, at, enabled, tip)| (at, enabled, tip.map(str::to_owned)))
    };
    let port_button = |p: &Panel| {
        p.buttons()
            .find(|(_, at, ..)| *at == [8.0, 392.0])
            .map(|(_, _, enabled, tip)| (enabled, tip.map(str::to_owned)))
            .expect("the port button")
    };
    assert!(world.ships()[ship].vessel.port.is_some());
    assert_eq!(port_button(&p), (true, Some("!Jump to port.".to_string())));
    let cancel = button(&|a| matches!(a, Action::Command(Command::ExpeditionStop { .. })));
    assert_eq!(cancel.map(|c| c.0), Some([8.0, 176.0]));
    assert!(!world.can_found_harbour(PlayerId(0), ship));
    let harbour = button(&|a| matches!(a, Action::Command(Command::ExpeditionBuildHarbor { .. })));
    assert_eq!(harbour, None);
    let compass: Vec<([f32; 2], Option<String>)> = p
        .buttons()
        .filter(|(a, ..)| matches!(a, Action::Command(Command::ExpeditionGo { .. })))
        .map(|(_, at, _, tip)| (at, tip.map(str::to_owned)))
        .collect();
    assert_eq!(compass.len(), 6);
    assert_eq!(compass[0], ([278.0, 118.0], Some("!East".to_string())));
    assert_eq!(compass[2], ([104.0, 192.0], Some("!SouthWest".to_string())));
    assert_eq!(compass[5], ([240.0, 46.0], Some("!NorthEast".to_string())));
    // A ship without a port: Jump to port disabled.
    let bare = world.debug_add_ship(PlayerId(0), (30, 32));
    let p = ship_window(&ctx(&world, &texts, &catalog), bare).unwrap();
    assert!(world.ships()[bare].vessel.port.is_none());
    assert_eq!(
        port_button(&p),
        (false, Some("!Ship has no target port.".to_string()))
    );
}

/// UI-GAME-5.1, MIL-ATK-1.3 (E-trace-031, catalogue part 4): with no
/// soldiers in reach (A = 0) the attack menu keeps its title, the weak /
/// strong pair, "Soldiers" and the four count buttons, and shows the line
/// `!Can not attack this building!` in place of the slider, the number and
/// ATTACK.
// spec: UI-GAME-5.1, MIL-ATK-1.3
#[test]
fn the_attack_menu_with_no_attackers() {
    let world = flat_world(20, 20);
    let (texts, catalog) = (Texts::default(), Catalog::default());
    let c = ctx(&world, &texts, &catalog);
    assert_eq!(c.world.attack_available(c.player, (10, 10)), 0);
    let p = attack_dialog(&c, (10, 10), 0.5, false);
    let texts: Vec<&str> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&"!Can not attack this building!"),
        "{texts:?}"
    );
    assert!(texts.contains(&"!Soldiers"));
    assert!(!p.widgets.iter().any(|w| matches!(w, Widget::Slider { .. })));
    assert!(!p.widgets.iter().any(|w| matches!(
        w,
        Widget::TextButton {
            action: Action::AttackGo,
            ..
        }
    )));
    let presets = p
        .buttons()
        .filter(|(a, ..)| matches!(a, Action::AttackPreset(_)))
        .count();
    assert_eq!(presets, 4);
}

/// ECO-PLACE T18, T19 (UI-GAME-3.2): the build menu greys each military
/// cell (a type with a territory range) within 4 fields of a military
/// building or site, with `!Too close to other military
/// buildings.`; the other cells stay enabled. Five fields off every cell
/// is enabled. A locked type is greyed with `!Building not allowed`, after
/// the clearance.
// spec: ECO-PLACE-6.1, ECO-PLACE-6.2, ECO-PLACE-6.3, UI-GAME-3.2 (T18, T19)
#[test]
fn t18_military_cells_too_close_to_a_military_building() {
    const HQ: u32 = 1;
    const BARRACKS: u32 = 2;
    const LOOKOUT: u32 = 3;
    const WORKSHOP: u32 = 4;
    let kind = |id, size, range| {
        let mut t = s2_sim::BuildingType::new(id, size);
        t.territory_range = range;
        t
    };
    let mut lookout = kind(LOOKOUT, BuildingSize::Medium, 0);
    lookout.soldier_limit = 1;
    let rules = s2_sim::Rules {
        building_types: vec![
            kind(HQ, BuildingSize::Large, 9),
            kind(BARRACKS, BuildingSize::Medium, 8),
            lookout,
            kind(WORKSHOP, BuildingSize::Medium, 0),
        ],
        ..s2_sim::Rules::empty()
    };
    let n = 50 * 40;
    let terrain = s2_sim::map::Terrain {
        width: 50,
        height: 40,
        heights: vec![0; n],
        height_floor: -100,
        states: vec![s2_sim::map::state::FOR_BUILDING; n],
        samples: Vec::new(),
    };
    let mut world = World::with_map(1, terrain, rules, PlayerId(0));
    world.submit(Command::CreateBuilding {
        player: PlayerId(0),
        building: HQ,
        at: (20, 20),
        settler: None,
    });
    world.step();
    assert!(world.building_at((20, 20)).is_some());
    let entry = |id, military| BuildingEntry {
        id,
        name: format!("type {id}"),
        size: BuildingSize::Medium,
        icon: Some((0, 0)),
        military,
        headquarters: false,
        sort: i64::from(id),
        has_cost: true,
        production: false,
    };
    let catalog = Catalog {
        buildings: vec![
            entry(BARRACKS, true),
            entry(LOOKOUT, true),
            entry(WORKSHOP, false),
        ],
        ..Catalog::default()
    };
    let texts = Texts::parse("");
    let cells = |world: &World, at: Point| -> Vec<(u32, bool, String)> {
        let c = ctx(world, &texts, &catalog);
        build_menu(&c, at, 6)
            .buttons()
            .filter_map(|b| match b.0 {
                Action::Command(Command::PlaceBuilding { building, .. }) => {
                    Some((*building, b.2, b.3.unwrap_or("").to_owned()))
                }
                _ => None,
            })
            .collect()
    };
    let too_close = "!Too close to other military buildings.".to_owned();
    for at in [(24, 20), (20, 24)] {
        let got = cells(&world, at);
        assert_eq!(got.len(), 3, "{at:?}: {got:?}");
        assert_eq!(got[0], (BARRACKS, false, too_close.clone()), "{at:?}");
        // The lookout tower and the workshop have no territory range.
        assert!(got[1].1 && got[2].1, "{at:?}: {got:?}");
        assert_eq!(got[1].2, "type 3 (0)");
    }
    for at in [(25, 20), (20, 25)] {
        let got = cells(&world, at);
        assert!(got.iter().all(|c| c.1), "{at:?}: {got:?}");
        assert_eq!(got[0].2, "type 2 (0)");
    }
    // T19: a construction site counts as soon as it is placed (another
    // player's site: the simulation's T19).
    world.submit(Command::PlaceBuilding {
        player: PlayerId(0),
        building: BARRACKS,
        at: (40, 20),
    });
    world.step();
    assert!(world.building_at((40, 20)).unwrap().under_construction);
    assert!(!cells(&world, (36, 20))[0].1);
    assert!(cells(&world, (35, 20))[0].1);
    // A locked type: "Building not allowed", but "Too close" comes first.
    world.submit(Command::LockBuilding {
        player: PlayerId(0),
        building: BARRACKS,
    });
    world.submit(Command::LockBuilding {
        player: PlayerId(0),
        building: WORKSHOP,
    });
    world.step();
    assert_eq!(cells(&world, (24, 20))[0], (BARRACKS, false, too_close));
    let far = cells(&world, (30, 30));
    assert_eq!(
        far[0],
        (BARRACKS, false, "!Building not allowed".to_owned())
    );
    assert_eq!(
        far[2],
        (WORKSHOP, false, "!Building not allowed".to_owned())
    );
}

/// UI-GAME-2.4, CAM-3.3 T21 (a): the coins target's panel reads
/// `(<n>/100)` with the local player's coins made so far: 0 with no mint,
/// whatever the stock.
// spec: UI-GAME-2.4, CAM-3.3 (T21)
#[test]
fn the_coins_target_shows_the_coins_made() {
    let mut world = flat_world(12, 12);
    world.set_mission(s2_sim::mission::Mission::new(3, Vec::new()));
    let texts = Texts::parse("");
    let catalog = Catalog::default();
    let p = briefing(&ctx(&world, &texts, &catalog));
    let lines: Vec<&str> = p
        .widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(lines, ["!MissionTarget: Produce coins", "(0/100)"]);
    assert_eq!(coin_line(60), "(60/100)");
}

/// UI-GAME-11.1, 4.5 c (E-trace-030): a message naming a ship shows the
/// ship's name in brackets after its text ("Ship is awaiting orders
/// (Aurora)"); one whose ship is gone shows the text alone.
// spec: UI-GAME-11.1, UI-GAME-4.5
#[test]
fn a_ship_message_names_the_ship() {
    let (mut world, _) = harbour_game();
    world.submit(Command::CreateShip {
        player: PlayerId(0),
        at: (30, 30),
    });
    world.step();
    let ship = world.ships().len() - 1;
    let texts = Texts::parse(
        "!Ship waiting for Orders\tShip is awaiting orders\nRomanShipName0\tAurora\nRomanShipName1\tBellona\n",
    );
    let catalog = Catalog::default();
    let c = ctx(&world, &texts, &catalog);
    let name = ship_name(&c, ship);
    assert!(name == "Aurora" || name == "Bellona", "{name}");
    let mut m = message(10, "Ship waiting for Orders");
    m.ship = Some(ship as u32);
    m.building = None;
    assert_eq!(
        message_text(&c, &m),
        format!("Ship is awaiting orders ({name})")
    );
    m.ship = Some(99);
    assert_eq!(message_text(&c, &m), "Ship is awaiting orders");
}

/// UI-GAME-11.1, 11.4 (E-trace-032, seen): "No resources found" names its
/// building in brackets, as "Building completed (Woodcutter's Hut)" does;
/// a geologist's find stands on no building and shows its text alone.
// spec: UI-GAME-11.1, UI-GAME-11.4
#[test]
fn no_resources_names_its_building() {
    let world = flat_world(12, 12);
    let texts = Texts::parse("!No resources found.\tNo resources found\nhut\tFisherman's Hut\n");
    let mut catalog = Catalog::default();
    catalog.building_names.insert(1, "hut".into());
    let c = ctx(&world, &texts, &catalog);
    let m = message(10, "No resources found.");
    assert_eq!(message_text(&c, &m), "No resources found (Fisherman's Hut)");
    let mut found = message(10, "Found Coal!");
    found.building = None;
    assert_eq!(message_text(&c, &found), "!Found Coal!");
}
