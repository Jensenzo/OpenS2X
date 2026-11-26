//! The in-game interface (`specs/UI-ingame.md`): the main bar, the context
//! menus on the map, the building windows, the attack dialog, the global
//! dialogs, messages and the minimap, described as panels. Every frame,
//! place and piece of art is the spec's (Q-184 answered: UI-GAME-1.8, 1.9,
//! 3.x, 4, 5.2, 6, 6.5, 6.6, 8.5). One stand-in is left: the building
//! windows' portrait is the type's icon, not the 3D model (see
//! `windows::portrait`).

mod context;
mod globals;
mod hud;
pub(crate) mod windows;

pub(crate) use context::*;
pub(crate) use globals::*;
pub(crate) use hud::*;
pub(crate) use windows::*;

use std::collections::HashMap;

use s2_assets::lang::Texts;
use s2_sim::grid::Point;
use s2_sim::{BuildingSize, Command, Placement, PlayerId, World};

use crate::ui::widgets::{
    Action, Atlas, Font, Global, Hover, Panel, PanelId, Rect, SliderKind, Widget, HEADING, WHITE,
};

/// A building type as the menus show it.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingEntry {
    pub id: u32,
    pub name: String,
    pub size: BuildingSize,
    /// The two icon indices (`bp_setIconXY`): an 80 x 80 cell.
    pub icon: Option<(u32, u32)>,
    pub military: bool,
    /// UI-GAME-3.2: the headquarters, which the player never builds.
    pub headquarters: bool,
    /// `bp_setSort`, 0 unless given.
    pub sort: i64,
    /// UI-GAME-8.5d: the type has a building cost (`bp_addCost`).
    pub has_cost: bool,
    /// UI-GAME-8.5d: a production type: one with a first product
    /// (`bp_setGoods`).
    pub production: bool,
}

/// UI-GAME-8.5c: the inventory pages have 40 slot controls.
pub const GOODS_SLOTS: usize = 40;

/// What the builders need from the rules, read once.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    /// Types with a size, in the order of their sort index (`bp_setSort`,
    /// UI-GAME-3.2), equal indices in the type table's order.
    pub buildings: Vec<BuildingEntry>,
    pub building_names: HashMap<u32, String>,
    pub good_names: HashMap<u32, String>,
    /// Goods' icon indices (`gp_setIconXY`): a 48 x 48 cell.
    pub good_icons: HashMap<u32, (u32, u32)>,
    /// The goods a building type's product is made from (groups expanded).
    pub inputs: HashMap<u32, Vec<u32>>,
    /// The tool a building's worker needs.
    pub tools_of: HashMap<u32, u32>,
    /// The products of building types.
    pub products: HashMap<u32, u32>,
    /// The player's 12 tool slots (ECO-PROD-7).
    pub tool_slots: Vec<u32>,
    /// Goods a warehouse lists: wares, then settlers.
    pub wares: Vec<u32>,
    pub settlers: Vec<u32>,
    pub coin: Option<u32>,
    /// UI-GAME-8.1a: the good `ship` (the expedition page's fourth slot).
    pub ship: Option<u32>,
    /// UI-GAME-4.1a (g): the good `stone` (the catapult window's slot).
    pub stone: Option<u32>,
    /// UI-GAME-8.1, 8.5c: a good's slot number on the depot and inventory
    /// pages, read as its `gp_setSort` ("position in sorted UI lists",
    /// LUA-PROP; Q-256 d).
    pub good_slots: HashMap<u32, usize>,
    /// UI-GAME-8.5b: the goods of the fourteen production counters.
    pub production_goods: [Option<u32>; 14],
    /// UI-GAME-4.4: the soldier good of each rank (`mi_` settings).
    pub soldiers: [Option<u32>; 5],
}

impl Catalog {
    /// UI-GAME-8.1, 8.5c: the page slots of `goods` (wares or settlers):
    /// (slot, good) by the goods' slot numbers, the list's order where a
    /// good has none; at most `GOODS_SLOTS`.
    pub fn slots(&self, goods: &[u32]) -> Vec<(usize, u32)> {
        goods
            .iter()
            .enumerate()
            .map(|(i, &g)| (self.good_slots.get(&g).copied().unwrap_or(i), g))
            .filter(|&(n, _)| n < GOODS_SLOTS)
            .collect()
    }

    pub fn new(rules: &s2_rules::RuleSet, tool_slots: Vec<u32>) -> Self {
        use s2_rules::model::{BuildingSize as RS, GoodType};
        let mut cat = Catalog {
            tool_slots,
            production_goods: s2_rules::PRODUCTION_COUNTERS.map(|name| rules.id_named(name)),
            ship: rules.id_named("ship"),
            stone: rules.id_named("stone"),
            soldiers: rules.military.soldiers,
            ..Catalog::default()
        };
        for g in rules.goods.iter() {
            if let Some(name) = &g.name {
                cat.good_names.insert(g.id, name.clone());
                if name == "Coin" {
                    cat.coin = Some(g.id);
                }
            }
            if let Some((u, v)) = g.icon {
                cat.good_icons
                    .insert(g.id, (u.max(0) as u32, v.max(0) as u32));
            }
            if let Some(sort) = g.sort.filter(|&n| n >= 0) {
                cat.good_slots.insert(g.id, sort as usize);
            }
            match g.good_type {
                None if g.transport_priority.is_some() => cat.wares.push(g.id),
                Some(GoodType::Settler) => cat.settlers.push(g.id),
                _ => {}
            }
        }
        let expand = |good: u32| -> Vec<u32> {
            let members: Vec<u32> = rules
                .goods
                .iter()
                .filter(|m| m.group == Some(good))
                .map(|m| m.id)
                .collect();
            if members.is_empty() {
                vec![good]
            } else {
                members
            }
        };
        let headquarters = rules.id_named(s2_rules::HEADQUARTERS);
        let mut sorted: Vec<(i64, BuildingEntry)> = Vec::new();
        for b in rules.buildings.iter() {
            let name = b.name.clone().unwrap_or_else(|| format!("{:08X}", b.id));
            cat.building_names.insert(b.id, name.clone());
            if let Some(product) = b.goods.map(|a| a.good) {
                cat.products.insert(b.id, product);
                let mut inputs = Vec::new();
                // H6: a project's recipe for the type, else its product's
                // cost list.
                let recipe = if b.inputs.is_empty() {
                    rules.goods.get(product).map_or(&[][..], |g| &g.costs[..])
                } else {
                    &b.inputs[..]
                };
                for cost in recipe {
                    for g in expand(cost.good) {
                        if !inputs.contains(&g) {
                            inputs.push(g);
                        }
                    }
                }
                cat.inputs.insert(b.id, inputs);
            }
            if let Some(worker) = b.worker.and_then(|w| rules.goods.get(w)) {
                // The worker's tool: a cost of the profession that is a tool
                // (a good in the tools group).
                if let Some(tool) = worker.costs.iter().map(|a| a.good).find(|g| {
                    rules
                        .goods
                        .get(*g)
                        .is_some_and(|t| t.group.is_some() && t.transport_priority == Some(12))
                }) {
                    cat.tools_of.insert(b.id, tool);
                }
            }
            let Some(size) = b.size else { continue };
            let size = match size {
                RS::Small => BuildingSize::Small,
                RS::Medium => BuildingSize::Medium,
                RS::Large => BuildingSize::Large,
                RS::Mine => BuildingSize::Mine,
                RS::Harbor => BuildingSize::Harbor,
            };
            sorted.push((
                b.sort.unwrap_or(0),
                BuildingEntry {
                    id: b.id,
                    name,
                    size,
                    icon: b.icon.map(|(u, v)| (u.max(0) as u32, v.max(0) as u32)),
                    military: b.soldier_limit.unwrap_or(0) > 0,
                    headquarters: headquarters == Some(b.id),
                    sort: b.sort.unwrap_or(0),
                    has_cost: !b.frame_costs.is_empty() || !b.costs.is_empty(),
                    production: b.goods.is_some(),
                },
            ));
        }
        sorted.sort_by_key(|(sort, _)| *sort);
        cat.buildings = sorted.into_iter().map(|(_, b)| b).collect();
        cat
    }

    pub fn good_name(&self, good: u32) -> String {
        self.good_names
            .get(&good)
            .cloned()
            .unwrap_or_else(|| format!("{good:08X}"))
    }
}

/// The context dialog on the map (UI-GAME-3, 4, 5); one at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Context {
    Flag(Point),
    Build {
        field: Point,
        tab: u8,
    },
    Road(Point),
    /// UI-GAME-3.6: the deploy-flag menu at the end of the road being
    /// built.
    DeployFlag(Point),
    Building {
        at: Point,
        tab: u8,
    },
    Attack {
        target: Point,
        share: f32,
        strong: bool,
    },
    /// A ship, by its index in the world's list.
    Ship(usize),
}

/// The in-game interface state.
#[derive(bevy::prelude::Resource, Debug, Clone, Default, PartialEq)]
pub struct GameUi {
    pub context: Option<Context>,
    pub global: Option<Global>,
    pub minimap: bool,
    pub messages: bool,
    pub briefing: bool,
    /// UI-GAME-11.3: rows hidden below the newest (0: the window follows
    /// the newest message).
    pub message_scroll: usize,
    /// UI-GAME-11.3: the kinds 1 to 4 switched off by their filter.
    pub message_hidden: [bool; 4],
    /// UI-GAME-11.4: the messages deleted with a right click.
    pub deleted_messages: std::collections::HashSet<MessageKey>,
    /// UI-GAME-11.4: the messages marked read by a left click on their
    /// icon.
    pub read_messages: std::collections::HashSet<MessageKey>,
    /// UI-GAME-3.4: road building from this flag.
    pub road_from: Option<Point>,
    /// TRN-ROAD-7.1: the road being built is a waterway.
    pub waterway: bool,
    /// The tick road building was started in, until the route exists.
    pub road_since: Option<u64>,
    /// UI-GAME-1.4: a refused action's message and when it goes (seconds).
    pub error: Option<(String, f64)>,
    /// UI-GAME-8.1: the "For all wares." modes of the goods and settlers
    /// pages, 0 -> 1 -> 2 -> 0.
    pub depot_modes: [u8; 2],
    /// UI-GAME-1.9: the screen point the context dialog was opened at.
    pub context_click: Option<[f32; 2]>,
    /// SND-4.1, UI-GAME-12.2: the text input that has the keys;
    /// UI-MENU-5.3: its caret (characters left of it) and the caret's look
    /// this draw (`.` on every other draw).
    pub input: Option<crate::ui::widgets::Input>,
    pub caret: usize,
    pub caret_dot: bool,
    /// UI-GAME-12.1: the creation number of the building the , (0) and the
    /// . (1) key last jumped to.
    pub jumped: [Option<u32>; 2],
    /// UI-GAME-8.5a: the chosen span option of the statistics page (kept
    /// by the interface; which is chosen at first is not given: the first).
    pub statistics_span: u8,
    /// UI-GAME-8.5a: the chosen series of the statistics page, kept by the
    /// interface (the first at first).
    pub statistics_series: u8,
    /// UI-GAME-8.5f: the player portraits released (curve hidden) on the
    /// statistics page, kept while the game runs. At the page's first
    /// opening in a game every present player's portrait is pressed, every
    /// curve drawn (Q-1016 answered 2026-11-08 c).
    pub statistics_hidden: [bool; 8],
    /// UI-GAME-8.5b: which of the production statistics' fourteen curves are
    /// marked, nothing at first, and its span option.
    pub production_marks: [bool; 14],
    pub production_span: u8,
    /// UI-GAME-8.5b: the counters as the production page last read them
    /// (when opened, on a toggle, on `!Unmark all`, on a change of span, and
    /// every 601 page updates otherwise), and the page updates since.
    pub production_read: Option<std::sync::Arc<[std::collections::VecDeque<u32>; 14]>>,
    /// UI-GAME-8.5d: the buildings page's tallies and the 22-tick period
    /// they were made in.
    pub building_tally: Option<(u64, std::collections::HashMap<u32, [u32; 4]>)>,
    /// UI-GAME-8.5g: the open drop-down list of the statistics pages (one
    /// at a time).
    pub stats_list: Option<u8>,
    pub production_updates: u32,
    /// UI-GAME-8.5a to 8.5d: the statistics family page open at the last
    /// update, for `STATS_MENUOPENED` at an opening.
    pub stats_page: Option<StatsPage>,
    /// CAM-3.6a (E-trace-032): the global window was opened while the end
    /// screen showed (its Statistics button, then the family's tabs), so it
    /// lies over the end screen; one open before lies under it.
    pub global_over_end: bool,
    /// UI-GAME-4.5: the names given to ships in their window, by the
    /// ship's index (kept by the interface, Q-228 b).
    pub ship_names: std::collections::BTreeMap<usize, String>,
    /// UI-GAME-12.1: the chat line's text while it is open.
    pub chat: Option<String>,
    /// The chat line opened in this frame (its RETURN is not typed).
    pub chat_opened: bool,
}

impl GameUi {
    /// The interface of a new or loaded game. UI-GAME-8.5a, 8.5b: the
    /// statistics pages' series, spans and marks stay for the session.
    /// UI-GAME-11.3a (F62): the messages window is shown at every game
    /// start, whatever the last game did (no setting holds it).
    pub fn for_new_game(&self) -> Self {
        Self {
            statistics_series: self.statistics_series,
            statistics_span: self.statistics_span,
            production_marks: self.production_marks,
            production_span: self.production_span,
            ..Self::at_start()
        }
    }

    /// UI-GAME-11.3a (F62): the interface when the program starts (a game
    /// started from the command line included): the messages window shown,
    /// nothing else open.
    pub fn at_start() -> Self {
        Self {
            messages: true,
            ..Self::default()
        }
    }
}

/// UI-GAME-8.5: a page of the statistics family, one per dialog (the player
/// statistics page with any series is one page).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsPage {
    Statistics,
    Production,
    Goods,
    Settlers,
    Buildings,
    Fleet,
}

impl StatsPage {
    pub fn of(global: Global) -> Option<Self> {
        Some(match global {
            Global::Statistics(_) => Self::Statistics,
            Global::Production => Self::Production,
            Global::Inventory(false) => Self::Goods,
            Global::Inventory(true) => Self::Settlers,
            Global::Buildings => Self::Buildings,
            Global::Fleet => Self::Fleet,
            _ => return None,
        })
    }

    /// UI-GAME-8.5a to 8.5d: the `STATS_MENUOPENED` its opening sends, by
    /// the page name it carries: none on the player statistics page (8.5a
    /// names none), `Good Production`, `Inventory` (the goods page) and
    /// `Buildings`. The settlers page and the fleet send none.
    pub fn menu_opened(self) -> Option<StatsMenuOpened> {
        let page = match self {
            Self::Statistics => None,
            Self::Production => Some("Good Production"),
            Self::Goods => Some("Inventory"),
            Self::Buildings => Some("Buildings"),
            Self::Settlers | Self::Fleet => return None,
        };
        Some(StatsMenuOpened { page })
    }
}

/// UI-GAME-8.5a to 8.5d: the event `STATS_MENUOPENED` a statistics page sends
/// as it opens, with its page name. Nothing in the game's scripts reads it;
/// it is there for a project's (`docs/extending.md`).
#[derive(bevy::prelude::Message, Debug, Clone, PartialEq, Eq)]
pub struct StatsMenuOpened {
    pub page: Option<&'static str>,
}

impl GameUi {
    /// UI-MENU-5.3: the input with the keys, with its caret.
    pub fn focus(&self) -> Option<crate::ui::widgets::Focus> {
        self.input.map(|input| crate::ui::widgets::Focus {
            input,
            caret: self.caret,
            dot: self.caret_dot,
        })
    }
}

/// UI-GAME-12.1 (Q-258 b): the building the . (`military`) or , key jumps
/// to, from the key's cursor `cursor` (the creation number of the building
/// it last jumped to). The list is all the player's buildings in creation
/// order; the press starts at the cursor's building (with no cursor the
/// player's first) and takes the first later one, wrapping to the list's
/// start (the cursor's own building last), that is finished and of the
/// key's types: barracks, guard house, tower and fortress (not the
/// watchtower or the catapult), or headquarters, storehouse and harbour.
/// `back` (SHIFT) walks the other way. `None` when the cursor's building is
/// gone or nothing qualifies: the press only clears the cursor.
pub fn next_jump(
    world: &World,
    player: PlayerId,
    military: bool,
    cursor: Option<u32>,
    back: bool,
) -> Option<(Point, u32)> {
    let mut list: Vec<&s2_sim::Building> =
        world.buildings().filter(|b| b.owner == player).collect();
    list.sort_unstable_by_key(|b| b.serial);
    let n = list.len();
    let start = match cursor {
        Some(serial) => list.iter().position(|b| b.serial == serial)?,
        None if n == 0 => return None,
        None => 0,
    };
    let qualifies = |b: &s2_sim::Building| {
        let kind = world.building_type(b.building_type);
        !b.under_construction
            && if military {
                kind.is_some_and(|k| k.soldier_limit > 0 && !k.warehouse && !k.catapult)
            } else {
                kind.is_some_and(|k| k.warehouse)
            }
    };
    (1..=n)
        .map(|k| {
            if back {
                (start + n - k % n) % n
            } else {
                (start + k) % n
            }
        })
        .map(|i| list[i])
        .find(|b| qualifies(b))
        .map(|b| (b.at, b.serial))
}

/// UI-GAME-12.1, CAM-3.7: the cheat a chat line's text is, if it is one of
/// the cheat words exactly. `TooFast` and `FreedomOfMovement` are words
/// whose effects are not specified (CAM-3.7): they do nothing here.
pub fn cheat_of(line: &str) -> Option<s2_sim::mission::Cheat> {
    use s2_sim::mission::Cheat;
    match line {
        "INeedSomeHelp" => Some(Cheat::INeedSomeHelp),
        "ShowMeTheWorld" => Some(Cheat::ShowMeTheWorld),
        "GiveMeSomeMore" => Some(Cheat::GiveMeSomeMore),
        _ => None,
    }
}

/// What the builders read.
pub struct Ctx<'a> {
    pub world: &'a World,
    pub player: PlayerId,
    pub screen: [f32; 2],
    pub texts: &'a Texts,
    pub catalog: &'a Catalog,
    /// UI-GAME-1.9: the screen point the context dialog was opened at.
    pub click: Option<[f32; 2]>,
    /// A tutorial runs: context dialogs open at (20, 20).
    pub tutorial: bool,
    /// The game speed, 1 to 3 (UI-GAME-2.7).
    pub speed: u32,
    /// The second clock: real seconds since the program started, which
    /// only the speed indicator reads (TOOL-1.3, TOOL-trace TRACE-5.3).
    pub real_seconds: f64,
    /// FMT-SAV-5.2, UI-GAME-11.1: the interface's own messages (the
    /// autosave's), listed with the simulation's.
    pub posted_messages: &'a [s2_sim::Message],
    /// UI-GAME-4.5: the ships' names, and the text input that has the keys.
    pub ship_names: &'a std::collections::BTreeMap<usize, String>,
    pub focus: Option<crate::ui::widgets::Focus>,
    /// UI-GAME-8.5a: the statistics page's span option.
    pub statistics_span: u8,
    /// UI-GAME-8.5f: the player portraits released on the statistics page.
    pub statistics_hidden: [bool; 8],
    /// UI-GAME-8.5b: which of the production statistics' fourteen curves are
    /// marked, nothing at first, and its span option.
    pub production_marks: [bool; 14],
    pub production_span: u8,
    /// UI-GAME-8.5b: the counters as the page last read them (`None`: read
    /// them now).
    pub production_read: Option<&'a [std::collections::VecDeque<u32>; 14]>,
    /// UI-GAME-8.5d: the player's per-type tallies of the buildings page as
    /// last recomputed (every 22 logic ticks), none before the first.
    pub building_tally: Option<&'a std::collections::HashMap<u32, [u32; 4]>>,
    /// UI-GAME-8.5g: the statistics pages' open drop-down list.
    pub stats_list: Option<u8>,
    /// UI-GAME-8.5g (b): the human player's record name (the profile's
    /// player name, `default` without one).
    pub player_name: &'a str,
    /// The action of the enabled button under the pointer.
    pub hovered: Option<&'a Action>,
}

impl Ctx<'_> {
    pub fn t(&self, key: &str) -> String {
        self.texts.get(key).to_owned()
    }

    fn tribe(&self) -> u8 {
        self.world.tribe(self.player)
    }
}

/// UI-GAME-2.1: the bar's top-left corner.
pub fn bar_origin(screen: [f32; 2]) -> [f32; 2] {
    [screen[0] / 2.0 - 252.0, screen[1] - 99.0]
}

/// UI-GAME-2.2: (x, y, sprite x, tooltip key) of the nine bar buttons.
pub const HUD_BUTTONS: [(f32, f32, u32, &str); 9] = [
    (35.0, 40.0, 392, "!Open Mission Target."),
    (83.0, 45.0, 344, "!Toggle Game Speed."),
    (133.0, 45.0, 8, "!Toggle construction view on/off."),
    (
        183.0,
        45.0,
        56,
        "!Open planning management sub system dialog (PMSSD).",
    ),
    (232.0, 45.0, 104, "!Toggle minimap on/off."),
    (282.0, 45.0, 152, "!Open master configuration dialog."),
    (332.0, 45.0, 296, "!Toggle Messages On/Off."),
    (382.0, 45.0, 248, "!Start Chat."),
    (432.0, 40.0, 200, "!Open options menu."),
];

/// UI-GAME-2.1, 2.2: the main bar with its nine buttons. Chat (8) is
/// disabled: there is no multiplayer.
pub fn hud(ctx: &Ctx) -> Panel {
    let mut p = Panel::new(PanelId::Hud, bar_origin(ctx.screen));
    p.sprite(Atlas::Game, [0, 184, 504, 96], [0.0, 0.0]);
    for (i, &(x, y, sx, key)) in HUD_BUTTONS.iter().enumerate() {
        p.button(
            Atlas::Game,
            [sx, 296, 48, 48],
            Hover::Swap([sx, 352]),
            [x, y],
            Some(ctx.t(key)),
            Action::Hud(i as u8 + 1),
            i != 7,
        );
    }
    p
}

/// UI-GAME-2.4: the briefing panel (a free game has no mission text).
pub fn briefing(ctx: &Ctx) -> Panel {
    let bar = bar_origin(ctx.screen);
    let mut p = Panel::new(PanelId::Briefing, [bar[0] - 182.0, ctx.screen[1] - 75.0]);
    p.sprite(Atlas::Game, [472, 8, 216, 80], [0.0, 0.0]);
    // CAM-3.1: the mission target as the panel's two lines (their keys
    // are the dialogs' `!MissionTarget` texts).
    let target = ctx.world.mission().map_or(0, |m| m.target);
    let key = match target {
        1 => "!MissionTarget: Destroy all enemies",
        2 => "!MissionTarget: Conquer the map",
        3 => "!MissionTarget: Produce coins",
        4 => "!MissionTarget: Reach the portal",
        _ => "!Missiontarget: None",
    };
    // UI-GAME-2.4 (E-trace-028, the original's picture): the two lines end
    // at x 184 and are centred on y 31 and 50, dark on the scroll: the
    // target, and once the map is completed (CAM-3.5) `!VICTORY` in
    // brackets.
    let line = |y: f32| [184.0, y - 8.0];
    p.right_text(ctx.t(key), Font::Dark, line(31.0), [0, 0, 0]);
    if ctx.world.mission().is_some_and(|m| m.completed.is_some()) {
        p.right_text(
            format!("({})", ctx.t("!VICTORY")),
            Font::Dark,
            line(50.0),
            [0, 0, 0],
        );
    } else if target == 3 {
        // UI-GAME-2.4 (Q-1063): the coins target's second line, the local
        // player's own coins made so far (CAM-3.3), not his team's sum. The
        // panel is built every frame; the original refreshes the figure
        // every half second.
        p.right_text(
            coin_line(ctx.world.coins_produced(ctx.player)),
            Font::Dark,
            line(50.0),
            [0, 0, 0],
        );
    }
    p
}

/// UI-GAME-2.4: the coins target's line, `(<n>/100)`.
pub(crate) fn coin_line(coins: u32) -> String {
    format!("({coins}/100)")
}

/// UI-GAME-1.8 (corrected 2026-11-14 b, F59, E-ui-033): each window passes
/// a frame picture of its own, drawn 1:1 with its top-left at the window's
/// origin, and takes its size: the flag, road and deploy-flag menus
/// `uiSmallDialog` (8, 8, 304 x 96); the production and site windows
/// `uiMediumDialog` (8, 8, 480 x 224); the military window `uiMedium2Dialog`
/// (8, 8, 480 x 272). The other windows keep the big panel of
/// `uiBigDialog`, drawn from its sheet's top-left corner.
pub const SMALL_DIALOG: [f32; 2] = [304.0, 96.0];
pub const MEDIUM_DIALOG: [f32; 2] = [480.0, 224.0];
pub const MEDIUM2_DIALOG: [f32; 2] = [480.0, 272.0];
/// UI-GAME-4.4 (F59, E-trace-026): the headquarters' and storehouses'
/// window, the tabbed `uiDepot` (8, 8, 376 x 464).
pub const DEPOT_DIALOG: [f32; 2] = [376.0, 464.0];
pub const BIG_DIALOG: [f32; 2] = [368.0, 458.0];
/// UI-GAME-6 (corrected 2026-11-15 night, E-ui-036, E-trace-028): the
/// economy dialogs' frame `uiDepot_long` (8, 8, 368 x 503), centred.
pub const DEPOT_LONG_DIALOG: [f32; 2] = [368.0, 503.0];

/// A dialog frame drawn at `size` with the title centred in its title bar,
/// and a close button: the pictures of UI-GAME-1.8 at their own size; other
/// dialogs on the big panel (narrow ones) or the medium sheet (which they
/// use is not in the spec, Q-184, Q-203).
fn frame(p: &mut Panel, ctx: &Ctx, title: &str, size: [f32; 2], close: Action) {
    let picture: Option<(Atlas, Rect)> = if size == SMALL_DIALOG {
        Some((Atlas::SmallDialog, [8, 8, 304, 96]))
    } else if size == MEDIUM_DIALOG {
        Some((Atlas::MediumDialog, [8, 8, 480, 224]))
    } else if size == MEDIUM2_DIALOG {
        Some((Atlas::Medium2Dialog, [8, 8, 480, 272]))
    } else if size == DEPOT_DIALOG {
        Some((Atlas::Depot, [8, 8, 376, 464]))
    } else if size == DEPOT_LONG_DIALOG {
        Some((Atlas::DepotLong, [8, 8, 368, 503]))
    } else {
        None
    };
    match picture {
        Some((atlas, rect)) => p.sized(atlas, rect, [0.0, 0.0], size),
        None if size[0] <= BIG_DIALOG[0] + 2.0 || size[1] == BIG_DIALOG[1] => {
            p.sized(Atlas::BigDialog, [0, 0, 368, 458], [0.0, 0.0], size)
        }
        None => p.sized(Atlas::MediumDialog, [8, 4, 465, 220], [0.0, 0.0], size),
    }
    frame_title(p, ctx, title, size[0], close);
}

/// UI-GAME-1.8: a frame's title at (w / 2, 14) in font_big_white, whose
/// sheet has only capitals: centred in x, its top at 14, below the sheet's
/// transparent top rows (the photo build-menu-flag.jpg, Q-209); the close
/// button at (w - 40, 5).
fn frame_title(p: &mut Panel, ctx: &Ctx, title: &str, width: f32, close: Action) {
    p.centred_text(
        title.to_uppercase(),
        Font::Big,
        [width / 2.0, 14.0],
        crate::ui::widgets::TITLE,
    );
    p.button(
        Atlas::Elements3,
        [415, 32, 24, 24],
        Hover::None,
        [width - 40.0, 5.0],
        Some(ctx.t("!Close this dialog.")),
        close,
        true,
    );
}

/// UI-GAME-1.9: where context dialogs open: at the click, the dialog's
/// bottom-right corner on the clicked point; x at most screenWidth - 600,
/// then x within [0, screenWidth - w] and y within [0, 0.85 screenHeight].
/// During the tutorial at (20, 20). Without a click (dialogs opened at
/// start) the screen's centre is taken as the point.
fn context_origin(ctx: &Ctx, size: [f32; 2]) -> [f32; 2] {
    if ctx.tutorial {
        return [20.0, 20.0];
    }
    let [w, h] = ctx.screen;
    let [cx, cy] = ctx.click.unwrap_or([w / 2.0, h / 2.0]);
    let x = (cx - size[0])
        .min(w - 600.0)
        .clamp(0.0, (w - size[0]).max(0.0));
    let y = (cy - size[1]).clamp(0.0, 0.85 * h);
    [x, y]
}

/// UI-GAME-1.9: the global dialogs open centred.
fn global_origin(ctx: &Ctx, size: [f32; 2]) -> [f32; 2] {
    // Whole pixels, rounded down (the 503 px economy frame: y 468 on 1440).
    [
        ((ctx.screen[0] - size[0]) / 2.0).max(0.0).floor(),
        ((ctx.screen[1] - size[1]) / 2.0).max(0.0).floor(),
    ]
}

/// A 48 x 48 good icon (UI-GAME-1.6): wares and tools from `uiElements1`,
/// settlers from `uiElements4` (UI-GAME-8.1).
fn good_icon(p: &mut Panel, ctx: &Ctx, good: u32, at: [f32; 2], size: f32) {
    if let Some(&(u, v)) = ctx.catalog.good_icons.get(&good) {
        let atlas = if ctx.catalog.settlers.contains(&good) {
            Atlas::Elements4
        } else {
            Atlas::Elements1
        };
        p.sized(atlas, [48 * u, 48 * v, 48, 48], at, [size, size]);
    }
}

/// An 80 x 80 building icon of `uiBuildings<tribe>` (UI-GAME-1.6).
fn building_icon(p: &mut Panel, ctx: &Ctx, building: u32, at: [f32; 2], size: f32) {
    let icon = ctx
        .catalog
        .buildings
        .iter()
        .find(|b| b.id == building)
        .and_then(|b| b.icon);
    if let Some((u, v)) = icon {
        p.sized(
            Atlas::Buildings(ctx.tribe()),
            [80 * u, 80 * v, 80, 80],
            at,
            [size, size],
        );
    }
}

impl Ctx<'_> {
    fn good_name(&self, good: u32) -> String {
        let name = self.catalog.good_name(good);
        self.texts.name(&name).to_owned()
    }
}

/// The panels of the game screen for `ui`.
pub fn panels(ctx: &Ctx, ui: &GameUi, borders: bool) -> Vec<Panel> {
    let mut out = vec![hud(ctx)];
    out.extend(speed_indicator(ctx, ui.minimap));
    if ui.briefing {
        out.push(briefing(ctx));
    }
    if ui.minimap {
        out.push(minimap(ctx, borders));
    }
    if ui.messages {
        out.push(messages(ctx, ui));
    }
    match &ui.context {
        Some(Context::Flag(f)) => out.push(flag_menu(ctx, *f)),
        Some(Context::Build { field, tab }) => out.push(build_menu(ctx, *field, *tab)),
        Some(Context::Road(f)) => out.push(road_menu(ctx, *f)),
        Some(Context::DeployFlag(f)) => out.push(deploy_flag_menu(ctx, *f)),
        Some(Context::Building { at, tab }) => out.extend(building_window(ctx, *at, *tab)),
        Some(Context::Attack {
            target,
            share,
            strong,
        }) => out.push(attack_dialog(ctx, *target, *share, *strong)),
        Some(Context::Ship(i)) => out.extend(ship_window(ctx, *i)),
        None => {}
    }
    match ui.global {
        Some(Global::Tools) => out.push(tools_dialog(ctx)),
        Some(Global::Military) => out.push(military_dialog(ctx)),
        Some(Global::Distribution(t)) => out.push(distribution_dialog(ctx, t)),
        Some(Global::Transport) => out.push(transport_dialog(ctx)),
        Some(Global::Statistics(series)) => out.push(statistics_dialog(ctx, series)),
        Some(Global::Production) => out.push(production_dialog(ctx)),
        Some(Global::Inventory(settlers)) => out.push(inventory_dialog(ctx, settlers)),
        Some(Global::Buildings) => out.push(buildings_dialog(ctx)),
        Some(Global::Fleet) => out.push(fleet_dialog(ctx)),
        None => {}
    }
    if let Some(line) = &ui.chat {
        out.push(chat_line(ctx, line));
    }
    if let Some((text, _)) = &ui.error {
        out.push(error(ctx, text));
    }
    out
}

/// UI-GAME-12.1: where the chat line's box sits (top left) and its size,
/// as seen at 2560 x 1440 (Q-258 e answered; whether it moves with other
/// screen sizes was not seen).
pub const CHAT_AT: [f32; 2] = [15.0, 275.0];
pub const CHAT_SIZE: [f32; 2] = [200.0, 32.0];
/// UI-GAME-12.1: the box is black at about half opacity: the picture under
/// it keeps 0.49 of its brightness.
pub const CHAT_ALPHA: f32 = 0.51;
/// UI-GAME-12.1: an empty line's white text cursor, 1 x 10, 32 px right of
/// the box's left edge and 9 px below its top.
pub const CHAT_CURSOR_AT: [f32; 2] = [32.0, 9.0];

/// UI-GAME-12.1 (Q-258 e answered 2026-11-08 c): the chat line while it is
/// open: a box of 200 x 32 at (15, 275), black at half opacity, with no
/// frame, title or prompt; an empty line shows only the white cursor. The
/// typed text starts at the cursor's place with the caret in it (how a
/// typed line looks was not seen).
pub fn chat_line(ctx: &Ctx, line: &str) -> Panel {
    let mut p = Panel::new(PanelId::Chat, CHAT_AT);
    p.widgets.push(Widget::Fill {
        at: [0.0, 0.0],
        size: CHAT_SIZE,
        rgba: [0.0, 0.0, 0.0, CHAT_ALPHA],
    });
    if line.is_empty() {
        p.widgets.push(Widget::Fill {
            at: CHAT_CURSOR_AT,
            size: [1.0, 10.0],
            rgba: [1.0, 1.0, 1.0, 1.0],
        });
    } else {
        let focus = ctx
            .focus
            .filter(|f| f.input == crate::ui::widgets::Input::Chat);
        let caret = focus.map_or(line.chars().count(), |f| f.caret.min(line.chars().count()));
        let left: String = line.chars().take(caret).collect();
        let right: String = line.chars().skip(caret).collect();
        p.text(
            format!("{left}|{right}"),
            Font::Body,
            [CHAT_CURSOR_AT[0], CHAT_CURSOR_AT[1] - 2.0],
            [255, 255, 255],
        );
    }
    p
}

/// UI-GAME-3: which context dialog `field` opens: an own flag the flag
/// menu, an own building its window, an enemy military building the attack
/// dialog, an own road the road menu, a free own field the build menu, or
/// the road menu when it takes only a flag (3.3); an own ship standing on
/// the field its window. A click on the map goes through [`context_for_click`]
/// (UI-GAME-9.4: a ship by its model); this is for a field named by the game
/// (the tutorial's ship step, CAM-5.3a 1; the jump keys; `--open`).
pub fn context_for(world: &World, player: PlayerId, field: Point) -> Option<Context> {
    let ship = world
        .ships()
        .iter()
        .position(|s| s.owner == player && s.vessel.launched && s.position() == field);
    match ship {
        Some(i) => Some(Context::Ship(i)),
        None => context_on_field(world, player, field),
    }
}

/// UI-GAME-9.4: what a left click on the map opens, with `field` the field
/// under the pointer and `ship` the ship whose model is under it
/// (`render::view3d::ShipHover`). An own flag, a building and a road on the
/// field come first; then the ship, whose window opens only when the field
/// has build class 0 (nothing can be placed there, as Water); on a field of
/// class 1 or more the road or build menu opens whatever was picked. A ship
/// is never found from its field.
pub fn context_for_click(
    world: &World,
    player: PlayerId,
    field: Point,
    ship: Option<usize>,
) -> Option<Context> {
    let first = world.flag_at(field) == Some(player)
        || world.building_at(field).is_some()
        || world.road_at(field).is_some();
    if first {
        return context_on_field(world, player, field);
    }
    let own_ship = ship.filter(|&i| {
        world
            .ships()
            .get(i)
            .is_some_and(|s| s.owner == player && s.vessel.launched)
    });
    // The player's build class: nothing can be placed off his land.
    let class_0 =
        world.field_owner(field) != Some(player) || world.placement_at(field) == Placement::Nothing;
    match own_ship {
        Some(i) if class_0 => Some(Context::Ship(i)),
        _ => context_on_field(world, player, field),
    }
}

fn context_on_field(world: &World, player: PlayerId, field: Point) -> Option<Context> {
    if world.flag_at(field) == Some(player) {
        return Some(Context::Flag(field));
    }
    if let Some(b) = world.building_at(field) {
        if b.owner == player {
            return Some(Context::Building { at: field, tab: 9 });
        }
        if crate::ui::military::attackable(world, player, field) {
            // UI-GAME-5.1, 5.2: the slider's value and the weak / strong
            // choice when the dialog opens are not given (Q-1093): a
            // stand-in of the middle and weak first.
            return Some(Context::Attack {
                target: field,
                share: 0.5,
                strong: false,
            });
        }
        return None;
    }
    if world
        .road_at(field)
        .is_some_and(|r| world.flag_at(r.path[0]) == Some(player))
    {
        return Some(Context::Road(field));
    }
    if world.field_owner(field) == Some(player) {
        return match world.placement_at(field) {
            Placement::Nothing => None,
            Placement::Flag => Some(Context::Road(field)),
            _ => Some(Context::Build { field, tab: 5 }),
        };
    }
    None
}

#[cfg(test)]
mod tests;
