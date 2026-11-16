//! The global dialogs: tools, military, distribution, transport,
//! statistics family, inventory, fleet (UI-GAME-6 to 8).

use super::*;

/// UI-GAME-6 (corrected 2026-11-15 night, E-ui-036, E-trace-028): the
/// economy dialogs open centred on the screen on the frame `uiDepot_long`
/// (8, 8, 368 x 503), every page at the same place; the transport page
/// draws its background `ui_detail_transport` (8, 8, 360 x 496) at (0, 0)
/// over the frame. The shared tab strip and reset button.
pub(crate) fn global_frame(ctx: &Ctx, title: &str, current: Global, reset: Action) -> Panel {
    let size = DEPOT_LONG_DIALOG;
    let mut p = Panel::new(PanelId::Global, global_origin(ctx, size));
    frame(&mut p, ctx, title, size, Action::CloseGlobal);
    if current == Global::Transport {
        // Over the frame's picture, under the title and the rest.
        p.widgets.insert(
            1,
            Widget::Sprite {
                atlas: Atlas::DetailTransport,
                rect: [8, 8, 360, 496],
                at: [0.0, 0.0],
                size: None,
                greyed: false,
            },
        );
    }
    let tabs = [
        (13u8, Global::Tools, "!Tools Menu"),
        (16, Global::Military, "!Military Menu"),
        (14, Global::Distribution(0), "!Arrangement Menu"),
        (15, Global::Transport, "!Transport Menu"),
    ];
    for (i, (id, g, key)) in tabs.into_iter().enumerate() {
        let on = std::mem::discriminant(&g) == std::mem::discriminant(&current);
        tab_button(&mut p, ctx, id, i, key, Action::OpenGlobal(g), on);
    }
    p.button(
        Atlas::Elements3,
        [440, 264, 48, 48],
        Hover::Swap([440, 312]),
        [295.0, 413.0],
        Some(ctx.t("!Reset dialog settings")),
        reset,
        true,
    );
    p
}

/// UI-GAME-6.1: the tools dialog.
pub fn tools_dialog(ctx: &Ctx) -> Panel {
    let player = ctx.player;
    let mut p = global_frame(
        ctx,
        &ctx.t("!Tools Menu"),
        Global::Tools,
        Action::Command(Command::ToolReset { player }),
    );
    let priorities = ctx.world.tool_priorities(player);
    for (i, &tool) in ctx.catalog.tool_slots.iter().enumerate().take(12) {
        let row = [87.0, 70.0 + 30.0 * i as f32];
        good_icon(&mut p, ctx, tool, [row[0] - 38.0, row[1] - 6.0], 30.0);
        // UI-GAME-6.1, 6.5: the economy slider, its top-left on the row.
        p.widgets.push(Widget::Slider {
            at: row,
            width: crate::ui::widgets::ECONOMY_SLIDER,
            value: priorities.weight(i),
            kind: SliderKind::Tool(i as u8),
            enabled: true,
        });
        let exclusive = priorities.exclusive == Some(i as u8);
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: [if exclusive { 168 } else { 104 }, 104, 24, 24],
            hover: Hover::None,
            pressed: Hover::None,
            at: [row[0] + 210.0, row[1] - 1.0],
            size: None,
            tooltip: Some(ctx.t("!Produce only this tool.")),
            action: Action::Command(if exclusive {
                Command::ToolClearExclusive { player }
            } else {
                Command::ToolExclusive {
                    player,
                    slot: i as u8,
                }
            }),
            enabled: true,
            selected: exclusive,
        });
    }
    p
}

/// UI-GAME-6.2: the military dialog. The rows' icon atlas is not named:
/// `uiElements5`, as the pair of buttons (Q-184).
pub fn military_dialog(ctx: &Ctx) -> Panel {
    let player = ctx.player;
    let mut p = global_frame(
        ctx,
        &ctx.t("!Military Menu"),
        Global::Military,
        Action::Command(Command::MilitaryReset { player }),
    );
    let s = ctx.world.military_settings(player);
    p.text(ctx.t("!Arrangement"), Font::Heading, [32.0, 64.0], HEADING);
    p.text(ctx.t("!Defense"), Font::Heading, [32.0, 237.0], HEADING);
    p.text(ctx.t("!Attack"), Font::Heading, [32.0, 362.0], HEADING);
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], [15.0, 224.0]);
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], [15.0, 349.0]);
    let row = |p: &mut Panel, icon: [u32; 2], label: &str, at: [f32; 2], value: f32, kind| {
        p.sprite(Atlas::Elements5, [icon[0], icon[1], 48, 48], at);
        p.text(ctx.t(label), Font::Body, [at[0] + 60.0, at[1] + 2.0], WHITE);
        // UI-GAME-6.2, 6.5: the economy slider at (+60, +20).
        p.widgets.push(Widget::Slider {
            at: [at[0] + 60.0, at[1] + 20.0],
            width: crate::ui::widgets::ECONOMY_SLIDER,
            value,
            kind,
            enabled: true,
        });
    };
    for (i, (icon, label)) in [
        ([176, 400], "!Far"),
        ([232, 400], "!Medium"),
        ([288, 400], "!Close"),
    ]
    .into_iter()
    .enumerate()
    {
        row(
            &mut p,
            icon,
            label,
            [32.0, 79.0 + 48.0 * i as f32],
            s.allocation(i),
            SliderKind::Allocation(i as u8),
        );
    }
    row(
        &mut p,
        [344, 400],
        "!Intercepting Defenders",
        [32.0, 297.0],
        s.defenders(),
        SliderKind::Defenders,
    );
    row(
        &mut p,
        [344, 456],
        "!Attack Strength",
        [32.0, 378.0],
        s.attack_strength(),
        SliderKind::AttackStrength,
    );
    for (dx, rect, key, strong) in [
        (0.0, [8, 344], "!Prefer weak soldiers", false),
        (56.0, [64, 344], "!Prefer strong soldiers", true),
    ] {
        let on = s.prefer_strong == strong;
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements5,
            rect: [rect[0], rect[1], 48, 48],
            hover: Hover::None,
            pressed: Hover::None,
            at: [32.0 + 120.0 + dx, 245.0],
            size: None,
            tooltip: Some(ctx.t(key)),
            action: Action::Command(Command::MilitaryDefenseType { player, strong }),
            enabled: !on,
            selected: on,
        });
    }
    p
}

/// UI-GAME-6.3: the distribution dialog: one button per distribution
/// table, the sliders of the chosen table's consumers with their icons.
pub fn distribution_dialog(ctx: &Ctx, table: u8) -> Panel {
    let player = ctx.player;
    let mut p = global_frame(
        ctx,
        &ctx.t("!Arrangement Menu"),
        Global::Distribution(table),
        Action::Command(Command::DistributionReset { player, table }),
    );
    let rules = ctx.world.distribution_rules();
    for (k, rule) in rules.iter().enumerate().take(6) {
        let y = 144 + 56 * k as u32;
        let name = rule
            .goods
            .first()
            .map(|&g| ctx.good_name(g))
            .unwrap_or_default();
        let on = usize::from(table) == k;
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements4,
            rect: [if on { 120 } else { 64 }, y, 48, 48],
            hover: Hover::Swap([8, y]),
            pressed: Hover::None,
            at: [32.0 + 50.0 * k as f32, 60.0],
            size: None,
            tooltip: Some(format!("{} {name}", ctx.t("!Arrangements for:"))),
            action: Action::OpenGlobal(Global::Distribution(k as u8)),
            enabled: true,
            selected: on,
        });
    }
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], [15.0, 115.0]);
    let tables = ctx.world.distribution(player);
    if let Some(entries) = tables.get(usize::from(table)) {
        for (i, entry) in entries.iter().enumerate().take(4) {
            let at = [110.0, 162.0 + 75.0 * i as f32];
            building_icon(&mut p, ctx, entry.key, [at[0] - 60.0, at[1] - 20.0], 48.0);
            // UI-GAME-6.3, 6.5: the economy slider.
            p.widgets.push(Widget::Slider {
                at,
                width: crate::ui::widgets::ECONOMY_SLIDER,
                value: entry.weight(),
                kind: SliderKind::Distribution {
                    table,
                    entry: i as u8,
                },
                enabled: true,
            });
        }
    }
    p
}

/// UI-GAME-8.5a: the series options of the player statistics page, in
/// their order (the series of the data paragraph).
pub(crate) const SERIES_OPTIONS: [&str; 8] = [
    "!Size of region",
    "!Number of buildings",
    "!Number of settlers",
    "!Economic state",
    "!Military power",
    "!produced Coins",
    "!Productivity",
    "!Frags",
];

/// UI-GAME-8.5a, 8.5b: the span options and the samples each shows.
pub(crate) const SPAN_OPTIONS: [(&str, usize); 4] = [
    ("!15 Minutes", 15),
    ("!1 Hour", 60),
    ("!4 Hours", 240),
    ("!16 Hours", 960),
];

/// UI-GAME-8.5g: the drop-down lists of the pages: the statistics page's
/// series and span, the production page's span.
pub const LIST_SERIES: u8 = 0;
pub const LIST_SPAN: u8 = 1;
pub const LIST_PRODUCTION_SPAN: u8 = 2;

/// UI-GAME-8.5g: a drop-down list's step between its choices.
const OPTION_STEP: f32 = 19.0;

/// UI-GAME-8.5a: the graph image, the canvas (8.5f's (x0, y0)), the
/// players' row and the option groups.
const STATS_IMAGE: [f32; 2] = [18.0, 129.0];
const STATS_CANVAS: [f32; 2] = [41.0, 153.0];
const STATS_PLAYERS: [f32; 2] = [30.0, 45.0];
const STATS_SERIES: [f32; 2] = [192.0, 412.0];
const STATS_SPANS: [f32; 2] = [18.0, 412.0];

/// UI-GAME-8.5 "As the original shows it" (F61, E-trace-026, T58): a
/// statistics page's frame, drawn from the family's corner, which lies
/// `shift` px left of the page's panel (the 7 and 10 px shifts of 8.5b and
/// 8.5d move only the content): the tabbed `uiDepot` (8, 8, 376 x 464) for
/// every page but the buildings page, whose frame is `uiXXLDialog_tabbed`
/// (4, 8, 512 x 536) 1:1 (UI-GAME-8.5d, E-ui-035); the title centred on the
/// frame's width in its header bar and the close button at (w - 40, 5)
/// (UI-GAME-1.8).
pub(crate) fn family_frame(p: &mut Panel, ctx: &Ctx, title: &str, shift: f32) {
    let wide = shift == BUILDINGS_SHIFT;
    let w = if wide {
        p.sprite(Atlas::XxlDialogTabbed, [4, 8, 512, 536], [-shift, 0.0]);
        FAMILY_SIZE[0]
    } else {
        p.sprite(Atlas::Depot, [8, 8, 376, 464], [-shift, 0.0]);
        376.0
    };
    p.centred_text(
        title.to_uppercase(),
        Font::Big,
        [w / 2.0 - shift, 14.0],
        crate::ui::widgets::TITLE,
    );
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [w - 40.0 - shift, 5.0],
        Some(ctx.t("!Close this dialog.")),
        Action::CloseGlobal,
        true,
    );
}

/// UI-GAME-8.5b, 8.5d: the content shifts of the production and buildings
/// pages.
pub const PRODUCTION_SHIFT: f32 = 7.0;
pub const BUILDINGS_SHIFT: f32 = 10.0;

/// UI-GAME-8.5g (a): a drop-down list (`list` its number on the pages):
/// one 152 x 20 text button at `at` (`uiElements0` (264, 176), highlighted
/// (264, 200)) showing the chosen choice; while open, one text button per
/// choice in their order, 19 px apart, from one step below it, or upward
/// from it when the list would run past the bottom of the screen. A choice
/// selects and closes; the button again or anything else closes it.
fn option_group(
    p: &mut Panel,
    ctx: &Ctx,
    at: [f32; 2],
    options: &[(&str, Action)],
    chosen: usize,
    list: u8,
) {
    let bar = crate::ui::widgets::SLOT_BAR;
    p.widgets.push(Widget::BarButton {
        text: options
            .get(chosen)
            .map(|(k, _)| ctx.t(k))
            .unwrap_or_default(),
        font: Font::Body,
        at,
        bar,
        raise: 0.0,
        action: Action::StatsList(list),
        enabled: true,
    });
    if ctx.stats_list != Some(list) {
        return;
    }
    let n = options.len() as f32;
    let downward = p.at[1] + at[1] + OPTION_STEP * (n + 1.0) <= ctx.screen[1];
    for (i, (key, action)) in options.iter().enumerate() {
        let step = OPTION_STEP * (i as f32 + 1.0);
        let y = if downward { at[1] + step } else { at[1] - step };
        p.widgets.push(Widget::BarButton {
            text: ctx.t(key),
            font: Font::Body,
            at: [at[0], y],
            bar,
            raise: 0.0,
            action: action.clone(),
            enabled: true,
        });
    }
}

/// UI-GAME-8.5a (`dialog25_0`): the player statistics page, titled
/// `!Statistics`: the graph image `uiElements1` (176, 232, 328 x 271) at
/// (18, 129); a cell per present player from (30, 45), 50 px apart, with
/// the tribe's first portrait (the player's name as tooltip, enabled while
/// the player is in the game) and its colour square 45 px lower; the graph
/// canvas at (41, 153); the series options at (192, 412) and the span
/// options at (18, 412); the separator `uiElements3` (0, 432, 336 x 8) at
/// (15, 115); the family tabs. The canvas is drawn by the production page's
/// routines (8.5f) with the chosen series of each player in the game.
pub fn statistics_dialog(ctx: &Ctx, chosen: u8) -> Panel {
    let mut p = Panel::new(PanelId::Global, family_origin(ctx, 0.0));
    family_frame(&mut p, ctx, &ctx.t("!Statistics"), 0.0);
    family_tabs(&mut p, ctx, Global::Statistics(chosen), 0.0);
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], [15.0, 115.0]);
    p.sprite(Atlas::Elements1, [176, 232, 328, 271], STATS_IMAGE);
    let present: Vec<s2_sim::PlayerId> = (0..8)
        .map(s2_sim::PlayerId)
        .filter(|&player| ctx.world.statistics(player).is_some())
        .collect();
    let in_game = |player: s2_sim::PlayerId| {
        ctx.world.is_player(player) && ctx.world.outcome(player).is_none()
    };
    let shown = |player: s2_sim::PlayerId| {
        !ctx.statistics_hidden
            .get(usize::from(player.0))
            .copied()
            .unwrap_or(false)
    };
    for (k, &player) in present.iter().enumerate() {
        let x = STATS_PLAYERS[0] + 50.0 * k as f32;
        let t = u32::from(ctx.world.tribe(player)).min(2);
        // UI-GAME-8.5f: an enabled portrait toggles its player's curve.
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: [16 + 168 * t, 512, 56, 80],
            hover: Hover::Swap([16 + 168 * t, 600]),
            pressed: Hover::None,
            at: [x, STATS_PLAYERS[1]],
            size: None,
            // UI-GAME-8.5g (b): the player record's name: the human's the
            // profile's player name (`default` without one), a computer
            // player's `AI`, as the original's saves have them (where a new
            // game's names come from is medium).
            tooltip: Some(if player == ctx.player {
                ctx.player_name.to_owned()
            } else {
                "AI".to_owned()
            }),
            action: Action::StatisticsPlayer(player.0),
            enabled: in_game(player),
            selected: shown(player),
        });
        // UI-GAME-8.5g (b): the player's colour index: the map's
        // (FMT-S2M-2.6, CAM-4.3a), else the player's slot (the setup's
        // colours are not kept by the game).
        let c = u32::from(crate::render::models3d::colour_index(player));
        p.button(
            Atlas::Elements0,
            [520 + 24 * (c % 2), 216 + 24 * (c / 2), 24, 24],
            Hover::None,
            [x, STATS_PLAYERS[1] + 45.0],
            None,
            Action::None,
            true,
        );
    }
    let series_options: Vec<(&str, Action)> = SERIES_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, key)| (*key, Action::StatisticsSeries(i as u8)))
        .collect();
    option_group(
        &mut p,
        ctx,
        STATS_SERIES,
        &series_options,
        usize::from(chosen),
        LIST_SERIES,
    );
    let span_options: Vec<(&str, Action)> = SPAN_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, (key, _))| (*key, Action::StatisticsSpan(i as u8)))
        .collect();
    let span = usize::from(ctx.statistics_span).min(SPAN_OPTIONS.len() - 1);
    option_group(&mut p, ctx, STATS_SPANS, &span_options, span, LIST_SPAN);
    // UI-GAME-8.5a, 8.5f: the graph of the production page's routines, one
    // curve per player whose portrait is enabled (in the game) and pressed,
    // in his own colour (GFX-OBJ-2.11), his number its index (that many
    // pixels lower); M the largest sample within the span over them,
    // rounded up.
    let index = usize::from(chosen).min(SERIES_OPTIONS.len() - 1);
    let n = SPAN_OPTIONS[span].1;
    let curves: Vec<Curve> = present
        .iter()
        .filter(|&&player| in_game(player) && shown(player))
        .filter_map(|&player| {
            let samples = &ctx.world.statistics(player)?.series[index];
            Some(Curve {
                index: usize::from(player.0),
                rgb: crate::render::models3d::player_colour(player),
                samples,
            })
        })
        .collect();
    graph(&mut p, &curves, top_within(&curves, n), STATS_CANVAS, n);
    p
}

/// UI-GAME-8.5f: the player page's M, the largest of the last `n` samples of
/// the curves drawn (0 without any).
pub(crate) fn top_within(curves: &[Curve], n: usize) -> u32 {
    curves
        .iter()
        .flat_map(|c| c.samples.iter().rev().take(n).copied())
        .max()
        .unwrap_or(0)
}

/// UI-GAME-8.5f: one curve of a graph: its index p (p pixels lower), its
/// colour and its samples, oldest first.
pub(crate) struct Curve<'a> {
    pub index: usize,
    pub rgb: [f32; 3],
    pub samples: &'a std::collections::VecDeque<u32>,
}

/// UI-GAME-8.5b: where the production statistics page's parts stand.
const PRODUCTION_IMAGE: [f32; 2] = [16.0, 175.0];
const PRODUCTION_CANVAS: [f32; 2] = [49.0, 195.0];
const PRODUCTION_SPANS: [f32; 2] = [18.0, 455.0];
const PRODUCTION_UNMARK: [f32; 2] = [192.0, 455.0];

/// UI-GAME-8.5f: the curve colours by counter.
const CURVE_COLOURS: [[f32; 3]; 14] = [
    [1.0, 0.0, 0.0],
    [0.5, 0.0, 0.0],
    [0.6, 0.6, 0.6],
    [0.0, 1.0, 0.0],
    [0.5, 0.5, 1.0],
    [0.5, 0.5, 0.5],
    [0.0, 0.0, 0.0],
    [1.0, 0.6, 0.6],
    [0.9, 0.8, 0.0],
    [1.0, 1.0, 0.7],
    [0.9, 0.8, 0.3],
    [0.5, 0.0, 0.0],
    [0.5, 0.5, 0.5],
    [0.6, 0.2, 0.0],
];
/// UI-GAME-8.5f: the axes' colour.
const AXIS_COLOUR: [f32; 4] = [0.1, 0.1, 0.1, 1.0];

/// A one-pixel line from `from` to `to` as filled rectangles (the interface has
/// no line of its own): one for a level or upright line, else one per column.
fn line(p: &mut Panel, from: [f32; 2], to: [f32; 2], rgba: [f32; 4]) {
    let (left, right) = if from[0] <= to[0] {
        (from, to)
    } else {
        (to, from)
    };
    let width = right[0] - left[0];
    if left[1] == right[1] || width < 1.0 {
        let (top, bottom) = (left[1].min(right[1]), left[1].max(right[1]));
        p.widgets.push(Widget::Fill {
            at: [left[0], top],
            size: [width.max(1.0), (bottom - top).max(1.0)],
            rgba,
        });
        return;
    }
    let columns = width.ceil() as u32;
    let y = |c: u32| left[1] + (right[1] - left[1]) * c as f32 / columns as f32;
    for c in 0..columns {
        let (a, b) = (y(c), y(c + 1));
        p.widgets.push(Widget::Fill {
            at: [left[0] + width * c as f32 / columns as f32, a.min(b)],
            size: [1.0, (a - b).abs().max(1.0)],
            rgba,
        });
    }
}

/// UI-GAME-8.5f: the graph, with (x0, y0) the canvas's top left and `span` its
/// span in samples, of `curves` with the top value M (`top`, which each page
/// takes its own way): the axes, seven time labels and M; one curve per entry
/// in order, from the newest sample at x0 + 245 leftwards, p pixels lower, in
/// its colour. The top label's alignment is not given (left, beside the
/// axis). The player statistics page uses the same (8.5a).
pub(crate) fn graph(p: &mut Panel, curves: &[Curve], top: u32, [x0, y0]: [f32; 2], span: usize) {
    line(p, [x0, y0 + 198.0], [x0 + 250.0, y0 + 198.0], AXIS_COLOUR);
    line(
        p,
        [x0 + 250.0, y0 + 198.0],
        [x0 + 250.0, y0 + 2.0],
        AXIS_COLOUR,
    );
    for i in (0..=30usize).step_by(5) {
        p.centred_text(
            format!("-{}", span - span * i / 30),
            Font::Dark,
            [x0 + (245 * i / 30) as f32, y0 + 205.0],
            WHITE,
        );
    }
    p.text(top.to_string(), Font::Dark, [x0 + 260.0, y0], WHITE);
    if top == 0 {
        return;
    }
    let step = 245.0 / span as f32;
    for curve in curves {
        let (index, samples) = (curve.index, curve.samples);
        let [r, g, b] = curve.rgb;
        let point = |age: usize, v: u32| {
            [
                x0 + 245.0 - step * age as f32,
                y0 + 185.0 - (u64::from(v) * 185 / u64::from(top)) as f32 + index as f32,
            ]
        };
        let newest_first: Vec<u32> = samples.iter().rev().take(span + 1).copied().collect();
        for (age, pair) in newest_first.windows(2).enumerate() {
            line(
                p,
                point(age, pair[0]),
                point(age + 1, pair[1]),
                [r, g, b, 1.0],
            );
        }
    }
}

/// UI-GAME-8.5f: the production page's graph of the marked counters.
pub(crate) fn production_graph(
    p: &mut Panel,
    series: &[std::collections::VecDeque<u32>; 14],
    marks: [bool; 14],
    origin: [f32; 2],
    span: usize,
) {
    let curves: Vec<Curve> = (0..14)
        .filter(|&c| marks[c])
        .map(|c| Curve {
            index: c,
            rgb: CURVE_COLOURS[c],
            samples: &series[c],
        })
        .collect();
    // M: the largest sample of the marked series' whole history.
    let top = curves
        .iter()
        .flat_map(|c| c.samples.iter().copied())
        .max()
        .unwrap_or(0);
    graph(p, &curves, top, origin, span);
}

/// UI-GAME-8.5b (`dialog28_0`): the production statistics page, titled `!Good
/// Production`, of the local player: its origin 7 px right of the base placement;
/// the graph image at (16, 175) and the canvas at (49, 195) (8.5f); fourteen good
/// toggles at (13 + 48 (i mod 7), 60 + 48 (i div 7)); the span options at (18, 455)
/// and `!Unmark all` at (192, 455); the separator at (15, 155); the family tabs.
pub fn production_dialog(ctx: &Ctx) -> Panel {
    let mut p = Panel::new(PanelId::Global, family_origin(ctx, PRODUCTION_SHIFT));
    family_frame(&mut p, ctx, &ctx.t("!Good Production"), PRODUCTION_SHIFT);
    family_tabs(&mut p, ctx, Global::Production, PRODUCTION_SHIFT);
    p.sprite(Atlas::Elements3, [0, 432, 336, 8], [15.0, 155.0]);
    p.sprite(Atlas::Elements1, [176, 232, 328, 271], PRODUCTION_IMAGE);
    for (i, good) in ctx.catalog.production_goods.iter().enumerate() {
        let cell = [13.0 + 48.0 * (i % 7) as f32, 60.0 + 48.0 * (i / 7) as f32];
        let marked = ctx.production_marks[i];
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements3,
            rect: [0, 80, 48, 56],
            hover: Hover::None,
            pressed: Hover::None,
            at: cell,
            size: None,
            tooltip: good.map(|g| ctx.good_name(g)),
            action: Action::ProductionMark(i as u8),
            enabled: true,
            selected: marked,
        });
        if let Some(good) = good {
            good_icon(&mut p, ctx, *good, cell, 48.0);
        }
        // UI-GAME-8.5b: the pressed picture on a marked good, the hover
        // picture under the pointer.
        if marked {
            p.sized(Atlas::Elements3, [384, 336, 48, 48], cell, [48.0, 48.0]);
        } else if ctx.hovered == Some(&Action::ProductionMark(i as u8)) {
            p.sized(Atlas::Elements3, [336, 336, 48, 48], cell, [48.0, 48.0]);
        }
    }
    let span_options: Vec<(&str, Action)> = SPAN_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, (key, _))| (*key, Action::ProductionSpan(i as u8)))
        .collect();
    let span = usize::from(ctx.production_span).min(SPAN_OPTIONS.len() - 1);
    option_group(
        &mut p,
        ctx,
        PRODUCTION_SPANS,
        &span_options,
        span,
        LIST_PRODUCTION_SPAN,
    );
    p.widgets.push(Widget::BarButton {
        text: ctx.t("!Unmark all"),
        font: Font::Body,
        at: PRODUCTION_UNMARK,
        bar: crate::ui::widgets::DIALOG_BAR,
        raise: 0.0,
        action: Action::ProductionUnmarkAll,
        enabled: true,
    });
    // UI-GAME-8.5b: the counters as the page last read them.
    let empty = Default::default();
    let series = ctx.production_read.unwrap_or_else(|| {
        ctx.world
            .statistics(ctx.player)
            .map_or(&empty, |s| &s.production)
    });
    production_graph(
        &mut p,
        series,
        ctx.production_marks,
        PRODUCTION_CANVAS,
        SPAN_OPTIONS[span].1,
    );
    p
}

/// UI-GAME-8.5: the statistics family's tab bar: (tab icon id, tooltip
/// key, page): ids 17, 18, 19, 22, 21, 12 (Q-227 c).
pub const FAMILY_TABS: [(u8, &str, Option<Global>); 6] = [
    (17, "!Player statistics", Some(Global::Statistics(0))),
    (18, "!Production statistices", Some(Global::Production)),
    (
        19,
        "!All goods in all depots and the castle",
        Some(Global::Inventory(false)),
    ),
    (
        22,
        "!All settlers in all depots and the castle",
        Some(Global::Inventory(true)),
    ),
    (
        21,
        "!Size of village and productivity",
        Some(Global::Buildings),
    ),
    (12, "!Fleet", Some(Global::Fleet)),
];

/// UI-GAME-8.5: the tab bar, a one-row grid at (15, 25) of 40 x 50 cells;
/// the pages that are not built are disabled.
pub(crate) fn family_tabs(p: &mut Panel, ctx: &Ctx, current: Global, shift: f32) {
    for (i, &(id, key, page)) in FAMILY_TABS.iter().enumerate() {
        let on = page.is_some_and(|g| {
            std::mem::discriminant(&g) == std::mem::discriminant(&current)
                && (!matches!(g, Global::Inventory(_)) || g == current)
        });
        tab_enabled(
            p,
            u32::from(id),
            [15.0 + 40.0 * i as f32 - shift, 25.0],
            ctx.t(key),
            page.map_or(Action::None, Action::OpenGlobal),
            on,
            page.is_some(),
        );
    }
}

/// UI-GAME-7.1 (`dialog26_0`, `dialog35_0`): every good (or settler) of the
/// player in all warehouses (the headquarters included), summed.
pub fn inventory(world: &World, player: PlayerId) -> std::collections::BTreeMap<u32, u32> {
    let mut all = std::collections::BTreeMap::new();
    for (at, stock) in world.warehouses() {
        if world.building_at(at).is_none_or(|b| b.owner != player) {
            continue;
        }
        for &(good, n) in stock {
            *all.entry(good).or_insert(0) += n;
        }
    }
    all
}

/// UI-GAME-8.1: a depot page's slot `slot`: six per row, 54 px apart, rows
/// 62 px apart from (23, 60).
pub fn goods_slot(slot: usize) -> [f32; 2] {
    [
        23.0 + 54.0 * (slot % 6) as f32,
        60.0 + 62.0 * (slot / 6) as f32,
    ]
}

/// UI-GAME-8.5c, 7.1: the inventory (`dialog26_0` goods, `dialog35_0`
/// settlers): a slot per good of the page's kind by its slot number, six
/// per row from (23, 60) in 54 x 62 cells: the `uiElements3` (0, 80,
/// 48 x 56) button with the icon and the name as tooltip (no click action)
/// and the sum over all the player's warehouses right-aligned at
/// (+46, +49) in `small_no_alias_white`.
pub fn inventory_dialog(ctx: &Ctx, settlers: bool) -> Panel {
    let mut p = Panel::new(PanelId::Global, family_origin(ctx, 0.0));
    family_frame(&mut p, ctx, &ctx.t("!Inventory"), 0.0);
    family_tabs(&mut p, ctx, Global::Inventory(settlers), 0.0);
    let goods = if settlers {
        &ctx.catalog.settlers
    } else {
        &ctx.catalog.wares
    };
    let all = inventory(ctx.world, ctx.player);
    for (slot, good) in ctx.catalog.slots(goods) {
        let cell = goods_slot(slot);
        p.button(
            Atlas::Elements3,
            [0, 80, 48, 56],
            Hover::None,
            cell,
            Some(ctx.good_name(good)),
            Action::None,
            true,
        );
        good_icon(&mut p, ctx, good, cell, 48.0);
        let count = all.get(&good).copied().unwrap_or(0);
        p.right_text(
            count.to_string(),
            Font::Heading,
            [cell[0] + 46.0, cell[1] + 49.0],
            WHITE,
        );
    }
    p
}

/// UI-GAME-8.5d: the page's types: those with a building cost, by size class
/// (small, medium, large, mine, harbour) and within a class by their
/// `bp_setSort` number (the catalog's order); of two with the same pair the
/// later in id order takes the cell. At most the page's 35 cells.
pub fn buildings_page_types(catalog: &Catalog) -> Vec<&crate::ui::game::BuildingEntry> {
    use s2_sim::BuildingSize as S;
    let class = |e: &crate::ui::game::BuildingEntry| match e.size {
        S::Small => 0,
        S::Medium => 1,
        S::Large => 2,
        S::Mine => 3,
        S::Harbor => 4,
    };
    let mut types: Vec<_> = catalog.buildings.iter().filter(|e| e.has_cost).collect();
    types.sort_by_key(|e| (class(e), e.sort));
    let mut cells: Vec<&crate::ui::game::BuildingEntry> = Vec::new();
    for e in types {
        match cells.last_mut() {
            Some(last) if (class(last), last.sort) == (class(e), e.sort) => {
                if e.id > last.id {
                    *last = e;
                }
            }
            _ => cells.push(e),
        }
    }
    cells.truncate(35);
    cells
}

/// UI-GAME-8.5d: the period of the page's per-type tallies, logic ticks.
pub const TALLY_TICKS: u64 = 22;

/// UI-GAME-8.5d: per building type of the player: finished buildings (no
/// main or frame work left), sites, and the summed productivity of the
/// finished ones and how many those are.
pub fn building_counts(world: &World, player: PlayerId) -> HashMap<u32, [u32; 4]> {
    let mut counts: HashMap<u32, [u32; 4]> = HashMap::new();
    for b in world.buildings().filter(|b| b.owner == player) {
        let c = counts.entry(b.building_type).or_default();
        if b.under_construction {
            c[1] += 1;
        } else {
            c[0] += 1;
            c[2] += b.productivity.max(0) as u32;
            c[3] += 1;
        }
    }
    counts
}

/// UI-GAME-8.5d (`dialog29_0`): the buildings page, titled `!Buildings`,
/// placed as the family's pages and moved 10 px right: a cell per type of
/// [`buildings_page_types`] in a grid from (23, 62) of 79 x 77 cells, six
/// per row: the `uiElements3` (0, 0, 80 x 80) button with the type's
/// picture and its name as tooltip; at (+6, +10) `finished-under
/// construction` from the tallies (recomputed every 22 ticks); for a
/// production type (a first product) at (+70, +64), right-aligned, the
/// mean productivity `N%` (`-%` before the first tally), grey (150, 150,
/// 150) at 0; the family's frame (`family_frame`).
pub fn buildings_dialog(ctx: &Ctx) -> Panel {
    let mut p = Panel::new(PanelId::Global, family_origin(ctx, BUILDINGS_SHIFT));
    family_frame(&mut p, ctx, &ctx.t("!Buildings"), BUILDINGS_SHIFT);
    family_tabs(&mut p, ctx, Global::Buildings, BUILDINGS_SHIFT);
    let tally = ctx.building_tally;
    for (n, entry) in buildings_page_types(ctx.catalog).into_iter().enumerate() {
        let cell = [23.0 + 79.0 * (n % 6) as f32, 62.0 + 77.0 * (n / 6) as f32];
        p.button(
            Atlas::Elements3,
            [0, 0, 80, 80],
            Hover::None,
            cell,
            Some(title_of(ctx, entry.id)),
            Action::None,
            true,
        );
        building_icon(&mut p, ctx, entry.id, cell, 80.0);
        let counts = tally.map(|t| t.get(&entry.id).copied().unwrap_or_default());
        let [built, sites, productivity, finished] = counts.unwrap_or_default();
        p.text(
            format!("{built}-{sites}"),
            Font::Heading,
            [cell[0] + 6.0, cell[1] + 10.0],
            WHITE,
        );
        if entry.production {
            let (text, colour) = match counts {
                None => ("-%".to_owned(), WHITE),
                Some(_) => {
                    let mean = productivity.checked_div(finished).unwrap_or(0);
                    let colour = if mean == 0 { [150, 150, 150] } else { WHITE };
                    (format!("{mean}%"), colour)
                }
            };
            p.right_text(
                text,
                Font::Heading,
                [cell[0] + 70.0, cell[1] + 64.0],
                colour,
            );
        }
    }
    p
}

/// UI-GAME-8.5: the fleet's list box at (50, 85) with 11 visible rows. The
/// list box's row height is not given (Q-185 c); 15 px rows keep the 11
/// above the picture at y 255.
pub const FLEET_ROWS: usize = 11;
pub const FLEET_FIRST: [f32; 2] = [50.0, 85.0];
pub const FLEET_STEP: f32 = 15.0;

/// UI-GAME-8.5 (T58): the statistics family's base size: every page's frame
/// starts where a centred dialog of this size would, ((W - 512) / 2, (H -
/// 536) / 2) (the family's position; ours never moves), whatever its own
/// frame's size; the buildings page's frame is this size.
pub const FAMILY_SIZE: [f32; 2] = [512.0, 536.0];

pub(crate) fn family_origin(ctx: &Ctx, shift: f32) -> [f32; 2] {
    let o = global_origin(ctx, FAMILY_SIZE);
    [o[0] + shift, o[1]]
}

/// UI-GAME-8.5 (`dialog39_0`): the fleet: the picture `uiGame` (136, 440,
/// 344 x 184) at (6, 255); the player's finished ships by name
/// (`ship_name`) in the game's ship list order; a click on a row selects
/// the ship (its window opens, UI-GAME-4.5), moves the camera to it and
/// closes the fleet.
pub fn fleet_dialog(ctx: &Ctx) -> Panel {
    let mut p = Panel::new(PanelId::Global, family_origin(ctx, 0.0));
    family_frame(&mut p, ctx, &ctx.t("!Fleet"), 0.0);
    family_tabs(&mut p, ctx, Global::Fleet, 0.0);
    p.sprite(Atlas::Game, [136, 440, 344, 184], [6.0, 255.0]);
    for (row, &i) in crate::world::ships::fleet(ctx.world, ctx.player)
        .iter()
        .take(FLEET_ROWS)
        .enumerate()
    {
        p.widgets.push(Widget::TextButton {
            text: ship_name(ctx, i),
            font: Font::Body,
            at: [FLEET_FIRST[0], FLEET_FIRST[1] + FLEET_STEP * row as f32],
            colour: WHITE,
            action: Action::FleetShip(i),
            enabled: true,
        });
    }
    p
}

/// UI-GAME-6.4: the transport dialog: the classes in priority order with
/// up / down arrows (the row layout is partly traced, Q-095).
pub fn transport_dialog(ctx: &Ctx) -> Panel {
    let player = ctx.player;
    let mut p = global_frame(
        ctx,
        &ctx.t("!Transport Menu"),
        Global::Transport,
        Action::Command(Command::PriorityReset { player }),
    );
    let order = crate::ui::economy::rank_order(ctx.world, player);
    for (i, &good) in order.iter().enumerate().take(14) {
        let y = 58.0 + 25.0 * i as f32;
        good_icon(&mut p, ctx, good, [60.0, y], 24.0);
        p.text(ctx.good_name(good), Font::Body, [92.0, y + 4.0], WHITE);
        for (dx, up, arrow) in [(250.0, true, [40, 152]), (280.0, false, [136, 152])] {
            let cmd = crate::ui::economy::priority_move(ctx.world, player, i, up);
            p.button(
                Atlas::Elements0,
                [arrow[0], arrow[1], 24, 24],
                Hover::None,
                [dx, y],
                None,
                cmd.clone().map_or(Action::None, Action::Command),
                cmd.is_some(),
            );
        }
    }
    p
}
