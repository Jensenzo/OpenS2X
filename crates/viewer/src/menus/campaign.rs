//! Campaign, missions and tutorial in the interface
//! (`specs/CAM-campaign-missions.md`): the mission script's book dialog
//! and debriefing (CAM-1.3), the end screens (CAM-3.6), the campaign menu
//! and its unlock chain (CAM-4), and the tutorial's step engine with its
//! pointer (CAM-5). The script itself runs in `s2_rules::mission`; its
//! commands go to the simulation like the player's.
//!
//! The book, the victory book, the defeat and end screens and the tutorial
//! window are laid out by CAM-1.3c, 3.6, 3.6a to 3.6c and 5.3 (Q-187 e, f
//! answered); what is still open is in `docs/stand-ins.md` (Missions).

use bevy::prelude::*;
use s2_assets::lang::Texts;
use s2_rules::mission::{MissionScript, ScriptEvent};
use s2_sim::grid::Point;
use s2_sim::mission::{LossReason, MissionEvent};
use s2_sim::{PlayerId, World};

use crate::menus::menu::{dialog_button, Screen};
use crate::menus::settings::Settings;
use crate::ui::widgets::{
    self, Action, Atlas, Font, Hover, MenuAction, Panel, PanelId, Widget, WHITE,
};

/// CAM-4.1
pub const MISSIONS: u8 = 10;

/// CAM-4.1: the map name (and settings key) of mission `n` (1 to 10).
pub fn mission_name(n: u8) -> String {
    format!("campaign{n:02}")
}

/// CAM-4.2: mission 1 always; mission n + 1 when missions 1 to n are
/// completed, in order.
pub fn unlocked(settings: &Settings, n: u8) -> bool {
    (1..n).all(|m| settings.completed(&mission_name(m)))
}

/// CAM-4.2: where mission n's button sits on the campaign picture
/// (1024 x 768 space).
const MISSION_BUTTONS: [[f32; 2]; 10] = [
    [300.0, 187.0],
    [465.0, 187.0],
    [594.0, 309.0],
    [551.0, 490.0],
    [455.0, 589.0],
    [200.0, 510.0],
    [150.0, 333.0],
    [322.0, 460.0],
    [345.0, 302.0],
    [457.0, 379.0],
];

/// The script of the game being played: a Lua state, kept on the main
/// thread.
#[derive(Default)]
pub struct MissionScriptRes(pub Option<MissionScript>);

/// The book dialog (CAM-1.3): its pages (text keys), the picture, the page
/// on screen. A debriefing book leads to the campaign menu when closed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Book {
    pub pages: Vec<String>,
    pub picture: Option<String>,
    pub page: usize,
    pub debriefing: bool,
    /// CAM-1.3c: the map's name, the key of the book's title (`book_title`).
    pub title: String,
    /// CAM-3.6b, 3.6: the victory book or the defeat screen of a campaign
    /// mission, made on the book dialog.
    pub end: Option<EndScreen>,
}

/// CAM-3.6: the end screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndScreen {
    /// The map completed with this value (1 to 4).
    Victory(u8),
    Defeat,
}

/// What the interface shows of the mission.
#[derive(Resource, Debug, Clone, Default)]
pub struct MissionUi {
    /// The map's name and kind (CAM-1.1: 1 campaign, 2 tutorial).
    pub map: String,
    /// CAM-1.3c: the map's name, the book's title's key (`book_title`).
    pub title: String,
    pub kind: u8,
    pub target: u8,
    pub book: Option<Book>,
    /// CAM-1.3 `addDebriefing`, `addExtro`, `createLetter`.
    pub debriefing: Vec<String>,
    pub extro: Vec<String>,
    pub letters: Vec<String>,
    pub end: Option<EndScreen>,
    /// CAM-3.5: the map was completed this game, not yet acknowledged.
    pub completed: Option<u8>,
    pub tutorial: Option<Tutorial>,
    /// A notice for the player (a defeated computer player, a script
    /// error), shown in the message box.
    pub notice: Option<String>,
    /// FMT-SAV-3.12: the script's variables as of the last tick, which a
    /// save keeps.
    pub script_variables: Vec<(String, u32)>,
}

impl MissionUi {
    pub fn new(map: &str, kind: u8, target: u8) -> Self {
        let tutorial = (kind == 2)
            .then(|| lesson_of(map).map(Tutorial::new))
            .flatten();
        Self {
            map: map.to_owned(),
            kind,
            target,
            tutorial,
            ..Self::default()
        }
    }

    /// The script's requests for the interface (CAM-1.3). The book dialog
    /// is modal: an `openBookDialog` while one is open, and the pages and
    /// picture that follow it, are ignored (CAM-1.2: the briefing opens once
    /// though the script asks for it every tick of its second).
    pub fn apply(&mut self, events: Vec<ScriptEvent>) {
        let mut ignoring = false;
        for event in events {
            match event {
                ScriptEvent::OpenBook(key) => {
                    ignoring = self.book.is_some();
                    if !ignoring {
                        self.book = Some(Book {
                            pages: vec![key],
                            title: self.title.clone(),
                            ..Book::default()
                        });
                    }
                }
                ScriptEvent::AddPage(key) if !ignoring => {
                    if let Some(b) = self.book.as_mut() {
                        b.pages.push(key);
                    }
                }
                ScriptEvent::AddPicture(name) if !ignoring => {
                    if let Some(b) = self.book.as_mut() {
                        b.picture = Some(name);
                    }
                }
                ScriptEvent::AddDebriefing(key) => self.debriefing.push(key),
                ScriptEvent::AddExtro(key) => self.extro.push(key),
                ScriptEvent::Letter(key) => self.letters.push(key),
                ScriptEvent::Warning(w) => warn!("mission script: {w}"),
                ScriptEvent::PlayerColor { .. }
                | ScriptEvent::Command(_)
                | ScriptEvent::AddPage(_)
                | ScriptEvent::AddPicture(_) => {}
            }
        }
    }

    /// CAM-3.5: a map completed in this game is recorded in the player's
    /// settings and acknowledged; true when it is new, for the caller to write
    /// the profile file.
    pub fn record_completed(&mut self, settings: &mut Settings) -> bool {
        if self.completed.take().is_none() {
            return false;
        }
        settings.mark_completed(&self.map);
        settings.acknowledge(&self.map);
        true
    }

    /// CAM-3.6: EXIT MAP on the end screen or at a lesson's end: a campaign
    /// victory leads through its debriefing (CAM-1.3, Q-187 f) to the campaign
    /// menu, anything else to the main menu. `None` while the debriefing book
    /// holds the player in the game's screen.
    pub fn exit_map(&mut self) -> Option<Screen> {
        let won = matches!(self.end, Some(EndScreen::Victory(_)));
        // CAM-3.6b: the victory book showed the debriefing already.
        let shown = self.book.as_ref().is_some_and(|b| b.end.is_some());
        self.end = None;
        self.tutorial = None;
        if won && self.kind == 1 && !shown && !self.debriefing.is_empty() {
            self.book = Some(Book {
                pages: std::mem::take(&mut self.debriefing),
                picture: None,
                page: 0,
                debriefing: true,
                title: self.title.clone(),
                end: None,
            });
            return None;
        }
        self.book = None;
        Some(if won && self.kind == 1 {
            Screen::Campaign
        } else {
            Screen::Main
        })
    }

    /// CAM-1.3: the book's CONTINUE: the book closes; closing a debriefing
    /// leads to the campaign menu (Q-187 f).
    pub fn close_book(&mut self) -> Option<Screen> {
        self.book
            .take()
            .filter(|b| b.debriefing)
            .map(|_| Screen::Campaign)
    }

    /// The mission's results of the last step (CAM-3.2 to 3.5).
    pub fn results(&mut self, events: &[MissionEvent], player: PlayerId) {
        for e in events {
            match *e {
                MissionEvent::Completed { value } => {
                    self.completed = Some(value);
                    self.end = Some(EndScreen::Victory(value));
                    // CAM-3.6b: a campaign mission shows the victory book:
                    // its debriefing pages, the won picture.
                    if self.kind == 1 {
                        self.book = Some(Book {
                            pages: self.debriefing.clone(),
                            picture: Some("ui_briefing_screen_won".into()),
                            title: self.title.clone(),
                            end: self.end,
                            ..Book::default()
                        });
                    }
                }
                MissionEvent::Lost { player: p, reason } if p == player => {
                    if self.end.is_none() {
                        self.end = Some(EndScreen::Defeat);
                        // CAM-3.6, 1.3c: a campaign mission's defeat screen
                        // is made on the book dialog.
                        if self.kind == 1 {
                            self.book = Some(Book {
                                picture: Some("ui_briefing_screen_lost".into()),
                                title: self.title.clone(),
                                end: self.end,
                                ..Book::default()
                            });
                        }
                    }
                    let _ = reason;
                }
                // CAM-3.2: the human hears of a defeated computer player.
                MissionEvent::Lost {
                    player: p,
                    reason: LossReason::CastleDestroyed,
                } => self.notice = Some(format!("DEFEAT Player {} (#{})", p.0 + 1, p.0)),
                MissionEvent::Lost { .. } | MissionEvent::Won { .. } => {}
            }
        }
    }
}

/// One tick's script calls and results, after `world` stepped: the
/// commands are submitted for the next tick.
pub fn after_step(world: &mut World, script: &MissionScriptRes, ui: &mut MissionUi) {
    if let Some(s) = &script.0 {
        match s.after_step(world) {
            Ok(events) => {
                let rest = s2_rules::mission::submit(world, events);
                ui.apply(rest);
            }
            Err(e) => ui.notice = Some(format!("mission script: {e}")),
        }
    }
    ui.results(world.mission_events(), crate::game::PLAYER);
}

/// A text key of the briefings or tutorial files, `\n` kept for wrapping.
fn text<'a>(texts: &'a Texts, key: &'a str) -> &'a str {
    texts.entry(key).unwrap_or(key)
}

/// The 1024 x 768 reference space centred on the screen.
fn origin(screen: [f32; 2]) -> [f32; 2] {
    [
        ((screen[0] - 1024.0) / 2.0).floor(),
        ((screen[1] - 768.0) / 2.0).floor(),
    ]
}

/// UI-MENU-1.4: a menu text button: the 208 x 32 bar with its label
/// centred, raised by 0.003 of the 768 px menu height.
fn text_button(p: &mut Panel, text: String, at: [f32; 2], action: MenuAction, enabled: bool) {
    p.widgets.push(Widget::BarButton {
        text,
        font: Font::Big,
        at,
        bar: crate::ui::widgets::MENU_BAR,
        raise: 0.003 * 768.0,
        action: Action::Menu(action),
        enabled,
    });
}

/// CAM-1.3c (Q-1055 answered): the book's title is the map's name used as
/// a key of the interface texts (`campaign01` gives `Mission 1`, shown in
/// capitals); the key is the map's name as written, else in small letters
/// (the file's); a map without a key shows its name (medium, CAM-1.3c).
pub(crate) fn book_title(texts: &Texts, name: &str) -> String {
    texts
        .entry(name)
        .or_else(|| texts.entry(&name.to_ascii_lowercase()))
        .unwrap_or(name)
        .to_owned()
}

/// CAM-1.3c (E-cam-008, E-cam-012, E-trace-028): the book dialog: the
/// frame `uiBriefing` (0, 0, 728 x 568) drawn 1:1, centred, no close
/// button; the title (the map's name's text, `font_big_white` in (254, 239, 124))
/// centred on (264, 74); the strips `uiBriefing` (8, 568) 248 x 8 at (142,
/// 100) and (142, 405); the page picture (201, 30) 512 x 512; the page text
/// from (150, 120), 230 wide, `dark_system` black; the page arrows at (210,
/// 430) and (300, 430) with the label `<page>/<pages>` centred on (267,
/// 442); CONTINUE (190, 465). CAM-3.6b: the victory book has the title
/// VICTORY, the target's text centred under it on (264, 91), the debriefing
/// pages, CONTINUE (play goes on) and EXIT MAP (190, 487); CAM-3.6: the
/// defeat screen has no CONTINUE, arrows or label, its text centred on (260,
/// 90) in `light_system_outlined` and EXIT MAP.
pub fn book_panel(texts: &Texts, screen: [f32; 2], book: &Book) -> Panel {
    const SIZE: [f32; 2] = [728.0, 568.0];
    const TITLE: [u8; 3] = [254, 239, 124];
    let o = [
        ((screen[0] - SIZE[0]) / 2.0).floor(),
        ((screen[1] - SIZE[1]) / 2.0).floor(),
    ];
    let mut p = Panel::new(PanelId::Book, o);
    p.sprite(Atlas::Book, [0, 0, 728, 568], [0.0, 0.0]);
    let picture = book.picture.as_deref().and_then(widgets::picture);
    if let Some(atlas) = picture {
        p.sized(atlas, [0, 0, 512, 512], [201.0, 30.0], [512.0, 512.0]);
    }
    for y in [100.0, 405.0] {
        p.sprite(Atlas::Book, [8, 568, 248, 8], [142.0, y]);
    }
    let centred = |p: &mut Panel, text: String, font: Font, at: [f32; 2], colour: [u8; 3]| {
        let line = match font {
            Font::Heading => 12.0,
            _ => 16.0,
        };
        p.centred_text(text, font, [at[0], at[1] - line / 2.0], colour);
    };
    let defeat = book.end == Some(EndScreen::Defeat);
    match book.end {
        Some(EndScreen::Victory(value)) => {
            centred(
                &mut p,
                texts.get("!VICTORY").to_uppercase(),
                Font::Big,
                [264.0, 74.0],
                TITLE,
            );
            centred(
                &mut p,
                texts.get(target_text(value)).to_owned(),
                Font::Body,
                [264.0, 91.0],
                WHITE,
            );
        }
        Some(EndScreen::Defeat) => {
            // CAM-3.6c (T20a): the book's own title, the `!DEFEAT` label at
            // (260, 90) in `light_system_outlined` (centred on it, as the
            // title is; the spec gives the point only), and the lost text
            // in the book's text box.
            centred(
                &mut p,
                book_title(texts, &book.title).to_uppercase(),
                Font::Big,
                [264.0, 74.0],
                TITLE,
            );
            centred(
                &mut p,
                texts.get("!DEFEAT").to_owned(),
                Font::Outlined,
                [260.0, 90.0],
                WHITE,
            );
            p.paragraph(
                texts.get("!You have lost the Map!").to_owned(),
                Font::Dark,
                [150.0, 120.0],
                230.0,
            );
        }
        None => centred(
            &mut p,
            book_title(texts, &book.title).to_uppercase(),
            Font::Big,
            [264.0, 74.0],
            TITLE,
        ),
    }
    if let Some(key) = book.pages.get(book.page) {
        p.paragraph(text(texts, key), Font::Dark, [150.0, 120.0], 230.0);
    }
    if !defeat {
        let pages = book.pages.len();
        let arrow = |p: &mut Panel, x: u32, at: f32, d: i32, enabled: bool, tip: &str| {
            p.widgets.push(Widget::Button {
                atlas: Atlas::Elements0,
                rect: [x, 376, 24, 24],
                hover: Hover::Swap([x + 64, 376]),
                pressed: Hover::Swap([x + 64, 376]),
                at: [at, 430.0],
                size: None,
                tooltip: Some(texts.get(tip).to_owned()),
                action: Action::Menu(MenuAction::BookPage(d)),
                enabled,
                selected: false,
            });
        };
        arrow(&mut p, 8, 210.0, -1, book.page > 0, "!Previous Page.");
        arrow(&mut p, 40, 300.0, 1, book.page + 1 < pages, "!Next Page.");
        let label = if pages == 0 {
            "-/-".to_owned()
        } else {
            format!("{}/{}", book.page + 1, pages)
        };
        centred(&mut p, label, Font::Dark, [267.0, 442.0], [0, 0, 0]);
        let close = if book.end.is_some() {
            MenuAction::EndContinue
        } else {
            MenuAction::CloseBook
        };
        dialog_button(
            &mut p,
            texts.get("!CONTINUE").to_owned(),
            [190.0, 465.0],
            close,
            true,
        );
    }
    if book.end.is_some() {
        // CAM-3.6c: on the defeat screen EXIT MAP is moved to CONTINUE's
        // place (190, 465); the victory book keeps it at (190, 487).
        let y = if defeat { 465.0 } else { 487.0 };
        dialog_button(
            &mut p,
            texts.get("!EXIT MAP").to_owned(),
            [190.0, y],
            MenuAction::EndExit,
            true,
        );
    }
    p
}

/// CAM-3.6: the target's text of a victory with map value `value`.
fn target_text(value: u8) -> &'static str {
    match value {
        1 => "!You have destroyed all enemies!",
        2 => "!You have conquered the map!",
        3 => "!You have enough Coins in your depots!",
        _ => "!You have reached the magic gate!",
    }
}

/// CAM-3.6a (E-cam-012, E-cam-015, T20a): the end screen of a free game:
/// the book's frame `uiBriefing` (0, 0, 728 x 568), centred; the book's
/// title (the map's name's text, 1.3c) centred on (374, 74) (the standard
/// place moved by (+10, +60)); the result `!VICTORY` or `!DEFEAT` on (374,
/// 110) in `light_system_outlined` and 25 px lower its reason in
/// `dark_system`; one row per player from (140, 190), 25 px apart: `Player
/// #n` at x 140 and `!Team n` at 400 (`light_system_outlined`), the name at
/// 230 (`dark_system`), the player's own result at 500 in (255, 255, 155)
/// while in the game and (190, 190, 90) once defeated; `!Statistics` (300,
/// 445), `!EXIT MAP` (300, 467) and `!CONTINUE` (300, 489), CONTINUE
/// disabled on a defeat (and after a portal victory, CAM-3.5); Statistics
/// opens the statistics window over the end screen (E-trace-032). Centring the
/// result lines on their points is ours (the spec gives the points only).
/// The name column (Q-1057, T20b): the human's profile name, every computer
/// player's the interface text `!AI`; only the game's players have a row
/// (a closed slot made none), the rows close up and keep their slot's
/// number.
/// CAM-3.6a (E-trace-032): an end screen row's label colour, the player's
/// colour (GFX-OBJ-2.11) as a text colour.
pub fn label_colour(player: s2_sim::PlayerId) -> [u8; 3] {
    crate::render::models3d::player_colour(player)
        .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn end_panel(
    texts: &Texts,
    screen: [f32; 2],
    end: EndScreen,
    world: &World,
    title: &str,
    player_name: &str,
) -> Panel {
    const SIZE: [f32; 2] = [728.0, 568.0];
    const TITLE: [u8; 3] = [254, 239, 124];
    const IN_GAME: [u8; 3] = [255, 255, 155];
    const DEFEATED: [u8; 3] = [190, 190, 90];
    let o = [
        ((screen[0] - SIZE[0]) / 2.0).floor(),
        ((screen[1] - SIZE[1]) / 2.0).floor(),
    ];
    let mut p = Panel::new(PanelId::Book, o);
    p.sprite(Atlas::Book, [0, 0, 728, 568], [0.0, 0.0]);
    let centred = |p: &mut Panel, text: String, font: Font, at: [f32; 2], colour: [u8; 3]| {
        let line = match font {
            Font::Heading => 12.0,
            _ => 16.0,
        };
        p.centred_text(text, font, [at[0], at[1] - line / 2.0], colour);
    };
    centred(
        &mut p,
        book_title(texts, title).to_uppercase(),
        Font::Big,
        [374.0, 74.0],
        TITLE,
    );
    let local = world.local_player();
    let (result, reason) = match end {
        EndScreen::Victory(value) => ("!VICTORY", target_text(value)),
        EndScreen::Defeat => (
            "!DEFEAT",
            match world.outcome(local) {
                Some(s2_sim::Outcome::Lost {
                    reason: LossReason::MissionNotCompleted,
                    ..
                }) => "!Mission not completed",
                _ => "!Castle destroyed!",
            },
        ),
    };
    centred(
        &mut p,
        texts.get(result).to_owned(),
        Font::Outlined,
        [374.0, 110.0],
        WHITE,
    );
    centred(
        &mut p,
        texts.get(reason).to_owned(),
        Font::Dark,
        [374.0, 135.0],
        [0, 0, 0],
    );
    for (k, &player) in world.players().iter().take(8).enumerate() {
        let y = 190.0 + 25.0 * k as f32;
        // E-trace-032 (seen): "Player #n:", with a colon, in the player's
        // colour.
        p.text(
            format!("Player #{}:", player.0 + 1),
            Font::Outlined,
            [140.0, y],
            label_colour(player),
        );
        let name = if player == local {
            player_name
        } else {
            texts.get("!AI")
        };
        p.text(name.to_owned(), Font::Dark, [230.0, y], [0, 0, 0]);
        if let Some(team) = world.team_of(player) {
            p.text(
                texts
                    .get(&format!("!Team {}", i32::from(team) + 1))
                    .to_owned(),
                Font::Outlined,
                [400.0, y],
                WHITE,
            );
        }
        let (own, colour) = match world.outcome(player) {
            Some(s2_sim::Outcome::Won { .. }) => (Some("!VICTORY"), IN_GAME),
            Some(s2_sim::Outcome::Lost { .. }) => (Some("!DEFEAT"), DEFEATED),
            None => (None, IN_GAME),
        };
        if let Some(own) = own {
            p.text(
                texts.get(own).to_owned(),
                Font::Outlined,
                [500.0, y],
                colour,
            );
        }
    }
    // CAM-3.6a (E-trace-033): the end screen's bar; a disabled one keeps its
    // face (darker) and its label's colour.
    let end_button = |p: &mut Panel, text: &str, y: f32, action: Action, enabled: bool| {
        p.widgets.push(Widget::BarButton {
            text: texts.get(text).to_owned(),
            font: Font::Body,
            at: [300.0, y],
            bar: crate::ui::widgets::END_BAR,
            raise: 0.0,
            action,
            enabled,
        });
    };
    end_button(
        &mut p,
        "!EXIT MAP",
        467.0,
        Action::Menu(MenuAction::EndExit),
        true,
    );
    // CAM-3.6a (E-trace-032): Statistics opens the ordinary statistics
    // window (UI-GAME-8.5a, on the series kept) over the end screen, which
    // stays behind it; the window's close button returns to the end screen.
    end_button(
        &mut p,
        "!Statistics",
        445.0,
        Action::OpenGlobal(crate::ui::widgets::Global::Statistics(0)),
        true,
    );
    // A portal victory has left the game (CAM-3.5); a defeat cannot go on
    // (E-trace-033: the button keeps its label's colour, no hover, no click).
    let continues = !matches!(end, EndScreen::Victory(4) | EndScreen::Defeat);
    end_button(
        &mut p,
        "!CONTINUE",
        489.0,
        Action::Menu(MenuAction::EndContinue),
        continues,
    );
    p
}

/// UI-MENU-1.7: the markers of the missions that may be played (CAM-4.2:
/// mission 1, and n + 1 once n is won): mission, top-left (the island
/// point minus (22, 75), panel space) and whether it is won (a gold eagle;
/// the one still to win is silver).
pub fn mission_markers(settings: &Settings) -> Vec<(u8, [f32; 2], bool)> {
    (1..=MISSIONS)
        .filter(|&n| unlocked(settings, n))
        .map(|n| {
            let [x, y] = MISSION_BUTTONS[usize::from(n - 1)];
            (
                n,
                [x - 22.0, y - 75.0],
                settings.completed(&mission_name(n)),
            )
        })
        .collect()
}

/// UI-MENU-1.7 (F29): the campaign page: `uicampaign` (the sea map with
/// the islands I to X and the empty scroll), the single-player scroll of
/// 1.5 with CAMPAIGN pressed, an SPQR eagle (64 x 90, `uiElements0`) on
/// each island whose mission may be played, silver for the one to win and
/// gold for those won; the hovered marker's `campaign<nn>_dsc` in the box
/// at the map's lower left; a click starts the mission.
pub fn campaign_menu(
    menu: &crate::menus::menu::MenuState,
    texts: &Texts,
    screen: [f32; 2],
) -> Panel {
    let o = origin(screen);
    let mut p = Panel::new(PanelId::Menu, [0.0, 0.0]);
    crate::menus::menu::background(&mut p, screen);
    p.sized(Atlas::Campaign, [0, 0, 1024, 768], o, [1024.0, 768.0]);
    crate::menus::menu::single_player_scroll(
        &mut p,
        texts,
        o,
        MenuAction::Campaign,
        crate::menus::menu::continue_save(menu).is_some(),
    );
    for (n, [x, y], won) in mission_markers(&menu.settings) {
        let sx = if won { 798 } else { 678 };
        p.widgets.push(Widget::Button {
            atlas: Atlas::Elements0,
            rect: [sx, 20, 64, 90],
            hover: Hover::Swap([sx, 156]),
            pressed: Hover::None,
            at: [o[0] + x, o[1] + y],
            size: None,
            tooltip: None,
            action: Action::Menu(MenuAction::StartMission(n)),
            enabled: true,
            selected: false,
        });
    }
    if let Some(Action::Menu(MenuAction::StartMission(n))) = &menu.hovered {
        if let Some(dsc) = texts.entry(&format!("{}_dsc", mission_name(*n))) {
            p.widgets.push(Widget::Paragraph {
                text: dsc.to_owned(),
                font: Font::Dark,
                at: [o[0] + 116.0, o[1] + 596.0],
                width: 268.0,
                colour: WHITE,
                align: crate::ui::widgets::Align::TopLeft,
            });
        }
    }
    p
}

/// UI-MENU-1.6 (Run D): the tutorial page: the single-player scroll with
/// TUTORIAL pressed; the left sheet on `uiDesktop1` with the strip of four
/// lesson medallions at (210, 215) and, right of each, `!Lesson k` at
/// (290, 240 + 70 (k - 1)) under its `!Info k` caption (15 px above). The
/// language files spell the keys without a space (`!Lesson1`, `!Info1`).
pub fn tutorial_menu(texts: &Texts, screen: [f32; 2], can_continue: bool) -> Panel {
    let mut p = Panel::new(crate::ui::widgets::PanelId::Menu, [0.0, 0.0]);
    crate::menus::menu::background(&mut p, screen);
    let o = origin(screen);
    p.sized(Atlas::Desktop1, [0, 0, 1024, 768], o, [1024.0, 768.0]);
    crate::menus::menu::single_player_scroll(&mut p, texts, o, MenuAction::Tutorial, can_continue);
    p.sprite(
        Atlas::Elements0,
        [936, 520, 80, 280],
        [o[0] + 210.0, o[1] + 215.0],
    );
    for k in 1..=4u8 {
        let y = o[1] + 240.0 + 70.0 * f32::from(k - 1);
        p.text(
            texts.get(&format!("!Info{k}")),
            Font::Dark,
            [o[0] + 290.0, y - 15.0],
            [255, 255, 255],
        );
        text_button(
            &mut p,
            texts.get(&format!("!Lesson{k}")).to_owned(),
            [o[0] + 290.0, y],
            MenuAction::StartLesson(k),
            true,
        );
    }
    p
}

// ---- The tutorial (CAM-5) ----

/// CAM-5.2: the text key of every step by step index: T04 (0x00..0x0d), T03
/// (0x0e..0x1c), T01 (0x1d..0x34), T02 (0x35..0x50), each in `tutorial.txt`
/// order but T02, whose two Scholar steps come after its Flag steps.
pub const STEPS: [&str; 81] = [
    "T04_Introduction",
    "T04_BuildShip_SelectShipYard",
    "T04_BuildShip_ChangeProduction",
    "T04_BuildShip_CloseDialog",
    "T04_StartExpedition_SelectPort",
    "T04_StartExpedition_SelectPage",
    "T04_StartExpedition_SelectIcon",
    "T04_StartExpedition_CloseDialog",
    "T04_StartSailing_SelectShip",
    "T04_StartSailing_SelectArrow",
    "T04_BuildPort_SelectShip",
    "T04_BuildPort_SelectIcon",
    "T04_BuildPort_CloseDialog",
    "T04_EndTutorial",
    "T03_Introduction",
    "T03_Barracks_OpenSelectBuildingDialog",
    "T03_Barracks_SelectBuilding",
    "T03_Barracks_StartStreet",
    "T03_Barracks_SelectCreateStreet",
    "T03_Barracks_EndStreet",
    "T03_Catapult_OpenSelectBuildingDialog",
    "T03_Catapult_SelectPage",
    "T03_Catapult_SelectBuilding",
    "T03_Attack_OpenAttackDialog",
    "T03_Attack_StartAttack",
    "T03_Misc_OpenMilitaryDialog",
    "T03_Misc_SelectPage",
    "T03_Misc_CloseDialog",
    "T03_End",
    "T01_Introduction",
    "T01_ShowConstructionMap",
    "T01_WoodCutter_OpenSelectBuilding",
    "T01_WoodCutter_SelectBuilding",
    "T01_WoodCutter_StartStreet",
    "T01_WoodCutter_SelectCreateStreet",
    "T01_WoodCutter_EndStreet",
    "T01_SawMill_OpenSelectBuilding",
    "T01_SawMill_SelectPage",
    "T01_SawMill_SelectBuilding",
    "T01_Forester_OpenSelectBuilding",
    "T01_Forester_SelectBuilding",
    "T01_Forester_StartStreet",
    "T01_Forester_SelectCreateStreet",
    "T01_Forester_EndStreet",
    "T01_StoneMason_OpenSelectBuilding",
    "T01_StoneMason_SelectBuilding",
    "T01_StoneMason_StartStreet",
    "T01_StoneMason_SelectCreateStreet",
    "T01_StoneMason_EndStreet",
    "T01_StoneMason_OpenFlagDialog",
    "T01_StoneMason_SelectFlag",
    "T01_HideConstructionMap",
    "T01_End",
    "T02_Introduction",
    "T02_Flag_OpenSelectBuilding",
    "T02_Flag_SelectBuilding",
    "T02_Flag_StartStreet",
    "T02_Flag_SelectCreateStreet",
    "T02_Flag_EndStreet",
    "T02_Scholar_OpenFlagDialog",
    "T02_Scholar_CallScholar",
    "T02_Coal_OpenSelectBuildingDialog",
    "T02_Coal_SelectBuilding",
    "T02_Coal_StartStreet",
    "T02_Coal_SelectCreateStreet",
    "T02_Coal_EndStreet",
    "T02_Iron_OpenSelectBuildingDialog",
    "T02_Iron_SelectBuilding",
    "T02_Iron_StartStreet",
    "T02_Iron_SelectCreateStreet",
    "T02_Iron_EndStreet",
    "T02_Smeltery_OpenSelectBuilding",
    "T02_Smeltery_SelectPage",
    "T02_Smeltery_SelectBuilding",
    "T02_Smith_OpenSelectBuildingDialog",
    "T02_Smith_SelectPage",
    "T02_Smith_SelectBuilding",
    "T02_Smith_StartStreet",
    "T02_Smith_SelectCreateStreet",
    "T02_Smith_EndStreet",
    "T02_End",
];

/// CAM-5.2: lesson n's first step.
pub fn lesson_start(lesson: u8) -> u8 {
    match lesson {
        1 => 0x1d,
        2 => 0x35,
        3 => 0x0e,
        _ => 0x00,
    }
}

/// CAM-5.3: a lesson's last step (the window offers Play and EXIT MAP).
pub fn is_last_step(step: u8) -> bool {
    matches!(step, 0x0d | 0x1c | 0x34 | 0x50)
}

/// The lesson of a tutorial map (`tutorial0N`).
pub fn lesson_of(map: &str) -> Option<u8> {
    let n: u8 = map
        .to_ascii_lowercase()
        .strip_prefix("tutorial")?
        .parse()
        .ok()?;
    (1..=4).contains(&n).then_some(n)
}

/// CAM-5.3: the field the pointer shows at a step, if it expects a map
/// click there. The table's (127, 35) of step 0x0a is never shown: that step
/// expects the ship window's right fleet arrow (CAM-5.3a 3).
pub fn pointer(step: u8) -> Option<Point> {
    Some(match step {
        0x01 => (104, 56),
        0x04 => (103, 60),
        0x08 => (101, 63),
        0x0f => (43, 31),
        0x11 => (44, 30),
        0x13 | 0x23 | 0x2b | 0x3a | 0x4f => (45, 37),
        0x14 => (44, 33),
        0x17 => (55, 29),
        0x1f => (42, 35),
        0x21 => (43, 34),
        0x24 => (43, 37),
        0x27 => (46, 35),
        0x29 => (47, 34),
        0x2c => (40, 42),
        0x2e => (40, 41),
        0x30 | 0x4a => (44, 36),
        0x31 => (42, 38),
        0x36 | 0x38 | 0x3b | 0x41 => (48, 41),
        0x3d => (46, 43),
        0x3f | 0x46 => (47, 42),
        0x42 => (46, 45),
        0x44 => (47, 44),
        0x47 => (47, 40),
        0x4d => (44, 35),
        _ => return None,
    })
}

/// CAM-5.3a 1: lesson 4's step 0x08 selects the ship standing on the marked
/// field by that field, not by its model (UI-GAME-9.4).
pub fn ship_by_field(step: u8) -> bool {
    step == 0x08
}

/// CAM-5.3a 3: lesson 4's step 0x0a expects the ship window's right fleet
/// arrow; a map click then does nothing and shows no message.
const SHIP_ARROW_STEP: u8 = 0x0a;

/// CAM-5.3c (Q-1090, E-cam-019): lesson 1's `T01_StoneMason_SelectFlag`,
/// the step of the place-flag menu (UI-GAME-3.6).
pub const PLACE_FLAG_STEP: u8 = 0x32;

/// CAM-5.3c (Q-1090): the map click of step 0x31 (`T01_StoneMason_OpenFlagDialog`)
/// opens the place-flag menu (UI-GAME-3.6) at its field, whatever lies there.
pub fn opens_place_flag_menu(step_before_click: u8) -> bool {
    step_before_click == PLACE_FLAG_STEP - 1
}

/// A step's text key.
pub fn step_key(step: u8) -> &'static str {
    STEPS.get(usize::from(step)).copied().unwrap_or("T01_End")
}

/// CAM-5.3c (E-cam-011, E-cam-014), CAM-5.3a: the one control each step
/// expects, per dialog: whether `action` is it. `None` for the steps that
/// only explain (advanced with the window's button) and the map-click steps
/// (CAM-5.3, [`pointer`]). The build menu's SelectBuilding steps are
/// checked by their cell first ([`expected_cell`]).
pub(crate) fn expects(step: u8, action: &Action) -> Option<bool> {
    use s2_sim::Command as C;
    use widgets::Global;
    Some(match step {
        // Building window, lesson 4: the shipyard's boat / ship switch,
        // then the window's close button.
        0x02 => matches!(action, Action::Command(C::ProductionSwitch { .. })),
        // The building, harbour and ship windows' close button.
        0x03 | 0x07 | 0x0c => matches!(action, Action::CloseContext),
        // Harbour window: the expedition tab, then its start button.
        0x05 => matches!(action, Action::BuildingTab(12)),
        0x06 => matches!(action, Action::Command(C::ExpeditionStart { .. })),
        // CAM-5.3a 2: the compass button of direction 2 (`!SouthWest`).
        0x09 => matches!(
            action,
            Action::Command(C::ExpeditionGo { direction: 2, .. })
        ),
        // CAM-5.3a 3: the ship window's right fleet arrow.
        0x0a => matches!(action, Action::ShipArrow { right: true, .. }),
        // CAM-5.3a 4: `!Construct harbor.`
        0x0b => matches!(action, Action::Command(C::ExpeditionBuildHarbor { .. })),
        // Build menu, SelectBuilding steps (the cell decides first).
        0x10 | 0x16 | 0x20 | 0x26 | 0x28 | 0x2d | 0x3e | 0x43 | 0x49 | 0x4c => {
            matches!(action, Action::Command(C::PlaceBuilding { .. }))
        }
        // 0x37: the build menu's flag button (cell 0). 0x32
        // (`T01_StoneMason_SelectFlag`): the place-flag menu's Create Flag
        // (CAM-5.3c, Q-1090), checked by [`Tutorial::place_flag_menu_click`];
        // a flag placed any other way does not advance it.
        0x37 | 0x32 => matches!(action, Action::Command(C::PlaceFlag { .. })),
        // Build menu, SelectPage steps: only the medium buildings tab.
        0x15 | 0x25 | 0x48 | 0x4b => matches!(action, Action::BuildTab(6)),
        // Flag menu: its build-road button.
        0x12 | 0x22 | 0x2a | 0x2f | 0x39 | 0x40 | 0x45 | 0x4e => {
            matches!(action, Action::StartRoad(_))
        }
        // Flag menu: its call-geologist button (the call is sent).
        0x3c => matches!(action, Action::Command(C::OrderGeologist { .. })),
        // Main bar: the construction-view button (its click, not SPACE).
        0x1e | 0x33 => matches!(action, Action::Hud(3)),
        // Main bar: its own button for the economy dialog (the tools page).
        0x19 => matches!(action, Action::Hud(6)),
        // Attack dialog: the attack button.
        0x18 => matches!(action, Action::AttackGo),
        // Economy dialog: the tools page's military tab, then the military
        // page's close button.
        0x1a => matches!(action, Action::OpenGlobal(Global::Military)),
        0x1b => matches!(action, Action::CloseGlobal),
        _ => return None,
    })
}

/// CAM-5.3c (E-cam-011): the build menu cell each SelectBuilding step
/// expects (cell 0 the flag button, then the page's buildings in order).
pub(crate) fn expected_cell(step: u8) -> Option<usize> {
    Some(match step {
        0x10 => 8,
        0x16 => 12,
        0x20 | 0x26 | 0x43 => 1,
        0x28 => 2,
        0x2d | 0x3e => 3,
        0x37 => 0,
        0x49 => 5,
        0x4c => 7,
        _ => return None,
    })
}

/// CAM-5.3c (E-cam-011, E-cam-014): the error of a click on the wrong
/// control: the page-choice steps (the build menu's 0x15, 0x25, 0x48, 0x4b,
/// the harbour window's 5, the economy dialog's 0x1a) show
/// `T01_ERROR_SelectStreetIcon` (the original's own choice of key), every
/// other step `T01_ERROR_SelectMarkedButton`.
pub(crate) fn wrong_click_error(step: u8) -> &'static str {
    let name = step_key(step);
    if name.ends_with("_SelectPage") {
        "T01_ERROR_SelectStreetIcon"
    } else {
        "T01_ERROR_SelectMarkedButton"
    }
}

/// What a map click does in a tutorial step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldClick {
    /// Carried out (and the step advanced if it expected it).
    Allow,
    /// Not carried out; the error is shown.
    Refuse,
}

/// CAM-5.3b: the steps whose start moves the camera to their field (a
/// scripted move, GFX-VIEW-3.6 e), each right after the step before was
/// done. Every other step leaves the camera where it is.
pub fn moves_camera(step: u8) -> bool {
    matches!(
        step,
        // Lesson 1.
        0x1f | 0x24 | 0x2c | 0x2e | 0x31
        // Lesson 2.
        | 0x36 | 0x38 | 0x3b | 0x3d | 0x42 | 0x47
        // Lesson 3.
        | 0x0f | 0x13 | 0x14
        // Lesson 4.
        | 0x04 | 0x08
    )
}

/// UI-GAME-1.4 (T67): how long the tutorial's red text stays.
pub const ERROR_MS: u64 = 5_000;

/// CAM-5.2: the tutorial's state: one step index and the error shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tutorial {
    pub lesson: u8,
    pub step: u8,
    pub error: Option<&'static str>,
    /// UI-GAME-1.4 (T67): an error set since the interface last looked
    /// (it plays `uiCancel` and starts the 5 s), and when the shown one
    /// goes (milliseconds of the interface's clock).
    pub error_fresh: bool,
    pub error_until: Option<u64>,
    /// CAM-5.3b: the field a scripted camera move is to go to, until the
    /// view takes it.
    pub camera_move: Option<Point>,
}

impl Tutorial {
    pub fn new(lesson: u8) -> Self {
        Self {
            lesson,
            step: lesson_start(lesson),
            error: None,
            error_fresh: false,
            error_until: None,
            camera_move: None,
        }
    }

    /// UI-GAME-1.4 (T67): show the tutorial text `key` in red.
    fn refuse(&mut self, key: &'static str) {
        self.error = Some(key);
        self.error_fresh = true;
    }

    /// UI-GAME-1.4 (T67, E-ui-052): at the interface's clock `now_ms`: a
    /// newly set error starts its 5.0 s (true: play `uiCancel`); when they
    /// run out the text is cleared.
    pub fn error_timer(&mut self, now_ms: u64) -> bool {
        if std::mem::take(&mut self.error_fresh) {
            self.error_until = Some(now_ms + ERROR_MS);
            return true;
        }
        if self.error.is_none() {
            self.error_until = None;
        } else if self.error_until.is_some_and(|until| now_ms >= until) {
            self.error = None;
            self.error_until = None;
        }
        false
    }

    /// CAM-5.2: the next step; CAM-5.3b: its start may move the camera.
    pub fn advance(&mut self) {
        if !is_last_step(self.step) {
            self.step += 1;
            if moves_camera(self.step) {
                self.camera_move = pointer(self.step);
            }
        }
        self.error = None;
    }

    /// CAM-5.3: a click on the map at `field`.
    pub fn field_click(&mut self, field: Point) -> FieldClick {
        if is_last_step(self.step) {
            return FieldClick::Allow;
        }
        if self.step == SHIP_ARROW_STEP {
            return FieldClick::Refuse;
        }
        match pointer(self.step) {
            Some(p) if p == field => {
                self.advance();
                FieldClick::Allow
            }
            Some(_) => {
                self.refuse("T01_ERROR_SelectMarkedPosition");
                FieldClick::Refuse
            }
            None => {
                self.refuse("T01_ERROR_SelectMarkedButton");
                FieldClick::Refuse
            }
        }
    }

    /// CAM-5.3: an interface action; the step advances when it is the one
    /// the step expects. Actions are never held back.
    pub fn action(&mut self, action: &Action) {
        if matches!(
            action,
            Action::Menu(_) | Action::None | Action::Focus(_) | Action::MessagesScroll(_)
        ) {
            return;
        }
        if pointer(self.step).is_some() {
            return;
        }
        match expects(self.step, action) {
            // CAM-5.3c (Q-1090): a flag placed other than by the place-flag
            // menu (the build menu's flag cell) does not advance 0x32.
            Some(true) if self.step == PLACE_FLAG_STEP => {}
            Some(true) => self.advance(),
            Some(false) => self.refuse(wrong_click_error(self.step)),
            None => {}
        }
    }

    /// CAM-5.3c (E-cam-011): a click on cell `cell` of the build menu (0 the
    /// flag button): a SelectBuilding step advances on its one cell only and
    /// shows `T01_ERROR_SelectMarkedButton` for any other; other steps take
    /// the click as [`Tutorial::action`].
    pub fn build_menu_click(&mut self, action: &Action, cell: usize) {
        if pointer(self.step).is_some() {
            return;
        }
        match expected_cell(self.step) {
            Some(wanted) if wanted == cell => self.advance(),
            Some(_) => self.refuse("T01_ERROR_SelectMarkedButton"),
            None => self.action(action),
        }
    }

    /// CAM-5.3c (Q-1090, E-cam-019): a click on a control of the place-flag
    /// menu (UI-GAME-3.6). At step 0x32 its Create Flag advances to 0x33 and
    /// is carried out; any other of its controls shows
    /// `T01_ERROR_SelectMarkedButton` and is not carried out (`false`: the
    /// menu stays as it is). At other steps the click is
    /// [`Tutorial::action`]'s and carried out.
    pub fn place_flag_menu_click(&mut self, action: &Action) -> bool {
        if self.step != PLACE_FLAG_STEP {
            self.action(action);
            return true;
        }
        if matches!(action, Action::Command(s2_sim::Command::PlaceFlag { .. })) {
            self.advance();
            true
        } else {
            self.refuse("T01_ERROR_SelectMarkedButton");
            false
        }
    }

    /// Whether the window's own button moves on (explaining steps).
    pub fn explains(&self) -> bool {
        !is_last_step(self.step)
            && pointer(self.step).is_none()
            && expects(self.step, &Action::None).is_none()
    }
}

/// CAM-5.2a (E-trace-028): the tutorial window, titled TUTORIAL (the title
/// font's capitals, centred in the header bar), the step's text in white
/// from the frame's (20, 66). An explaining step (and a lesson's last) on
/// `uiBigDialog` (8, 8, 360 x 448) at ((W - 360) / 2, 22), with Cancel
/// (190, 376) and Continue (190, 401)
/// (the last step: Play and EXIT MAP there); a step that waits for a click
/// on the production window's frame `uiMediumDialog` (8, 8, 480 x 224) at
/// the top right, its right edge 5 px from the screen's, with Cancel (289,
/// 167). A wrong click's error in red under the text.
pub fn tutorial_panel(texts: &Texts, screen: [f32; 2], t: &Tutorial) -> Panel {
    let last = is_last_step(t.step);
    let tall = last || t.explains();
    let (size, at) = if tall {
        // E-cam-012: `uiBigDialog` (8, 8, 360 x 448) at ((W - 360) / 2, 22).
        ([360.0, 448.0], [((screen[0] - 360.0) / 2.0).floor(), 22.0])
    } else {
        ([480.0, 224.0], [screen[0] - 480.0 - 5.0, 22.0])
    };
    let mut p = Panel::new(PanelId::Tutorial, at);
    if tall {
        p.sprite(Atlas::BigDialog, [8, 8, 360, 448], [0.0, 0.0]);
    } else {
        p.sprite(Atlas::MediumDialog, [8, 8, 480, 224], [0.0, 0.0]);
    }
    p.centred_text(
        texts.get("!TUTORIAL").to_uppercase(),
        Font::Big,
        [size[0] / 2.0, 14.0],
        crate::ui::widgets::TITLE,
    );
    p.paragraph(
        text(texts, step_key(t.step)),
        Font::Body,
        [20.0, 66.0],
        size[0] - 40.0,
    );
    if let Some(e) = t.error {
        let y = if tall { 330.0 } else { 142.0 };
        p.widgets.push(Widget::Paragraph {
            text: text(texts, e).to_owned(),
            font: Font::Body,
            at: [30.0, y],
            width: size[0] - 60.0,
            colour: [255, 0, 0],
            align: crate::ui::widgets::Align::TopLeft,
        });
    }
    if last {
        dialog_button(
            &mut p,
            texts.get("!Play").to_owned(),
            [190.0, 376.0],
            MenuAction::TutorialPlay,
            true,
        );
        dialog_button(
            &mut p,
            texts.get("!EXIT MAP").to_owned(),
            [190.0, 401.0],
            MenuAction::EndExit,
            true,
        );
    } else if tall {
        dialog_button(
            &mut p,
            texts.get("!Cancel Tutorial").to_owned(),
            [190.0, 376.0],
            MenuAction::TutorialCancel,
            true,
        );
        dialog_button(
            &mut p,
            texts.get("!Continue Tutorial").to_owned(),
            [190.0, 401.0],
            MenuAction::TutorialNext,
            true,
        );
    } else {
        dialog_button(
            &mut p,
            texts.get("!Cancel Tutorial").to_owned(),
            [289.0, 167.0],
            MenuAction::TutorialCancel,
            true,
        );
    }
    p
}

/// The pointer sprite on screen.
#[derive(Component)]
pub struct PointerSprite;

/// CAM-5.3: the pointer over the step's field in the 3D view.
#[allow(clippy::too_many_arguments)]
pub fn pointer_sprite(
    mut commands: Commands,
    viewer: Res<crate::game::Viewer>,
    view: Res<crate::render::view3d::View3d>,
    mission: Res<MissionUi>,
    menu: Res<crate::menus::menu::MenuState>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<(&Camera, &GlobalTransform), With<crate::render::view3d::MapCamera3d>>,
    old: Query<Entity, With<PointerSprite>>,
    frame: Res<crate::ui::render::UiFrame>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    if menu.screen != crate::menus::menu::Screen::None || mission.book.is_some() {
        return;
    }
    let (Some(world), Some(t)) = (&viewer.world, &mission.tutorial) else {
        return;
    };
    // CAM-5.3: the step's field; CAM-5.2a (E-trace-028): else the widget
    // the step expects, the arrow's tip on its top-left corner, over the
    // dialogs.
    let (tip, z) = match pointer(t.step) {
        Some(field) => {
            let Some(p) = view.field_on_screen(world, *camera, field) else {
                return;
            };
            ([p.x, p.y], 9)
        }
        None => match expected_widget_corner(&frame.panels, t.step) {
            Some(corner) => (corner, 24),
            None => return,
        },
    };
    let Some(image) = library.raw_image(Atlas::TutorialPointer.file(), &mut images) else {
        return;
    };
    commands.spawn((
        PointerSprite,
        ImageNode { image, ..default() },
        Node {
            position_type: PositionType::Absolute,
            // The arrow's tip is the picture's lower right corner.
            left: Val::Px(tip[0] - 58.0),
            top: Val::Px(tip[1] - 58.0),
            width: Val::Px(64.0),
            height: Val::Px(64.0),
            ..default()
        },
        GlobalZIndex(z),
    ));
}

/// CAM-5.2a, 5.3c: the screen point of the top-left corner of the widget a
/// tutorial step expects in the panels on screen: a SelectBuilding step's
/// build menu cell (0 the flag button, then the page's buildings in order),
/// else the first enabled button whose action the step takes. None when it
/// is not on screen (or the step expects no widget).
pub(crate) fn expected_widget_corner(panels: &[Panel], step: u8) -> Option<[f32; 2]> {
    use s2_sim::Command as C;
    let corner = |p: &Panel, at: [f32; 2]| {
        [
            p.at[0].round() + at[0].round(),
            p.at[1].round() + at[1].round(),
        ]
    };
    if let Some(cell) = expected_cell(step) {
        let menu = panels.iter().find(|p| p.id == PanelId::Context)?;
        let at = menu
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Button {
                    action: Action::Command(C::PlaceFlag { .. } | C::PlaceBuilding { .. }),
                    at,
                    ..
                } => Some(*at),
                _ => None,
            })
            .nth(cell)?;
        return Some(corner(menu, at));
    }
    panels.iter().find_map(|p| {
        p.widgets.iter().find_map(|w| match w {
            Widget::Button {
                action,
                at,
                enabled: true,
                ..
            }
            | Widget::BarButton {
                action,
                at,
                enabled: true,
                ..
            } if *action != Action::None && expects(step, action) == Some(true) => {
                Some(corner(p, *at))
            }
            _ => None,
        })
    })
}

/// The mission panels of the frame (book, end screen, tutorial window).
pub fn panels(
    mission: &MissionUi,
    texts: &Texts,
    screen: [f32; 2],
    world: &World,
    player_name: &str,
) -> Vec<Panel> {
    let mut out = Vec::new();
    if let Some(t) = &mission.tutorial {
        out.push(tutorial_panel(texts, screen, t));
    }
    // CAM-3.6b: a campaign mission's victory and defeat are books.
    if let Some(book) = mission.book.as_ref().filter(|b| b.end.is_some()) {
        out.push(book_panel(texts, screen, book));
    } else if let Some(end) = mission.end {
        out.push(end_panel(
            texts,
            screen,
            end,
            world,
            &mission.title,
            player_name,
        ));
    } else if let Some(book) = &mission.book {
        out.push(book_panel(texts, screen, book));
    }
    out
}

#[cfg(test)]
mod tests {

    // UI-MENU-1.7 (F29): the pointer on mission 1's eagle shows its
    // `campaign01_dsc` text in the box at the map's lower left, (116, 596)
    // from the map's corner, 268 wide; nothing without a hovered eagle.
    // spec: UI-MENU-1.7
    #[test]
    fn the_hovered_eagle_shows_its_description() {
        let texts = Texts::parse("campaign01_dsc\tReach the gate.\n");
        let mut menu = crate::menus::menu::MenuState::default();
        let screen = [1024.0, 768.0];
        let o = origin(screen);
        let described = |menu: &crate::menus::menu::MenuState| {
            campaign_menu(menu, &texts, screen)
                .widgets
                .iter()
                .find_map(|w| match w {
                    Widget::Paragraph {
                        text, at, width, ..
                    } => Some((text.clone(), *at, *width)),
                    _ => None,
                })
        };
        assert_eq!(described(&menu), None);
        menu.hovered = Some(Action::Menu(MenuAction::StartMission(1)));
        assert_eq!(
            described(&menu),
            Some((
                "Reach the gate.".to_string(),
                [o[0] + 116.0, o[1] + 596.0],
                268.0
            ))
        );
    }

    // UI-MENU T16 (1.7): no mission won: one silver eagle, on island I at
    // (278, 112); with mission 1 won, a gold one there and a silver one on
    // island II.
    // spec: CAM-5.3 (T16)
    #[test]
    fn t16_campaign_markers() {
        let mut settings = Settings::default();
        assert_eq!(mission_markers(&settings), [(1, [278.0, 112.0], false)]);
        settings.mark_completed(&mission_name(1));
        assert_eq!(
            mission_markers(&settings),
            [(1, [278.0, 112.0], true), (2, [443.0, 112.0], false)]
        );
    }

    use super::*;

    #[test]
    fn steps_match_the_spec_counts() {
        // CAM-5.2: 14 + 15 + 24 + 28 steps; the lessons' first steps.
        assert_eq!(STEPS.len(), 81);
        assert!(STEPS[..14].iter().all(|s| s.starts_with("T04")));
        assert!(STEPS[14..29].iter().all(|s| s.starts_with("T03")));
        assert!(STEPS[29..53].iter().all(|s| s.starts_with("T01")));
        assert!(STEPS[53..].iter().all(|s| s.starts_with("T02")));
        assert_eq!(step_key(lesson_start(1)), "T01_Introduction");
        assert_eq!(step_key(lesson_start(2)), "T02_Introduction");
        assert_eq!(step_key(lesson_start(3)), "T03_Introduction");
        assert_eq!(step_key(lesson_start(4)), "T04_Introduction");
        for last in [0x0d, 0x1c, 0x34, 0x50] {
            assert!(step_key(last).contains("End"));
        }
        // T01's pointer steps are its map clicks (T02's do not all line up
        // with their names, Q-187).
        assert_eq!(pointer(0x1f), Some((42, 35)));
        assert_eq!(step_key(0x1f), "T01_WoodCutter_OpenSelectBuilding");
        // CAM-5.2: lesson 2's own order: its map clicks 0x41, 0x46, 0x4f are
        // the three EndStreet steps (Q-187 j).
        assert_eq!(step_key(0x3a), "T02_Flag_EndStreet");
        assert_eq!(step_key(0x3b), "T02_Scholar_OpenFlagDialog");
        assert_eq!(step_key(0x3c), "T02_Scholar_CallScholar");
        assert_eq!(step_key(0x3d), "T02_Coal_OpenSelectBuildingDialog");
        assert_eq!(step_key(0x41), "T02_Coal_EndStreet");
        assert_eq!(step_key(0x46), "T02_Iron_EndStreet");
        assert_eq!(step_key(0x47), "T02_Smeltery_OpenSelectBuilding");
        assert_eq!(step_key(0x4a), "T02_Smith_OpenSelectBuildingDialog");
        assert_eq!(step_key(0x4f), "T02_Smith_EndStreet");
        assert_eq!(step_key(0x50), "T02_End");
        assert_eq!(step_key(0x23), "T01_WoodCutter_EndStreet");
        assert_eq!(pointer(0x23), Some((45, 37)));
    }

    /// The rules and a new game on a map of the install, or `None` without
    /// `S2_GAME_DIR`.
    fn install_game(map_name: &str) -> Option<(s2_rules::RuleSet, World)> {
        let Some(root) = std::env::var_os("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return None;
        };
        let dir = s2_assets::GameDir::new(std::path::Path::new(&root)).expect("the install");
        let (rules, _) = s2_rules::load(&dir).expect("the property scripts");
        let path = crate::game::find_maps(&dir.root().join("data/maps"))
            .into_iter()
            .find(|p| crate::game::map_name(p).eq_ignore_ascii_case(map_name))
            .expect("the map in data/maps");
        let data = crate::game::read_game_file(&path).expect("the map");
        let map = s2_assets::s2m::parse(&data).expect("a map");
        let world = s2_rules::new_game(1, &map, &rules, 0).expect("a game");
        Some((rules, world))
    }

    /// CAM-5.3c (T19): the cell each SelectBuilding step expects holds, in
    /// our build menu pages, the building the step names (cell 0 the flag
    /// button), so the exact-cell check never refuses the right click.
    // spec: CAM-5.3c (T19), UI-GAME-3.2
    #[test]
    fn each_select_building_steps_cell_holds_its_building() {
        let Some((rules, _)) = install_game("tutorial01") else {
            return;
        };
        let catalog = crate::ui::game::Catalog::new(&rules, Vec::new());
        let page = |class, tab| crate::ui::game::build_page(&catalog, class, tab);
        use s2_sim::Placement as P;
        for (step, class, tab, name) in [
            (0x10, P::Large, 5, "Barracks"),
            (0x16, P::Large, 6, "Catapult"),
            (0x20, P::Large, 5, "WoodcutterHut"),
            (0x26, P::Large, 6, "Sawmill"),
            (0x28, P::Large, 5, "ForesterHouse"),
            (0x2d, P::Large, 5, "Stone Pit"),
            (0x3e, P::Mine, 5, "Coal Mine"),
            (0x43, P::Mine, 5, "Iron Mine"),
            (0x49, P::Large, 6, "Smeltery"),
            (0x4c, P::Large, 6, "Smithy"),
        ] {
            let cell = super::expected_cell(step).expect("a cell");
            assert_eq!(page(class, tab)[cell - 1].name, name, "step {step:x}");
        }
        assert_eq!(super::expected_cell(0x37), Some(0));
        // A wrong cell at a SelectBuilding step shows the error, the right
        // one advances.
        let mut t = Tutorial::new(1);
        t.step = 0x20;
        t.build_menu_click(&Action::None, 2);
        assert_eq!(
            (t.step, t.error),
            (0x20, Some("T01_ERROR_SelectMarkedButton"))
        );
        t.build_menu_click(&Action::None, 1);
        assert_eq!(t.step, 0x21);
    }

    /// The building a T01 step's name means, by its rules name.
    fn t01_building(step: &str) -> Option<&'static str> {
        let part = step.split('_').nth(1)?;
        Some(match part {
            "WoodCutter" => "WoodcutterHut",
            "SawMill" => "Sawmill",
            "Forester" => "ForesterHouse",
            "StoneMason" => "Stone Pit",
            _ => return None,
        })
    }

    // CAM T19c (Q-1090): step 0x31's click opens the place-flag menu; at
    // 0x32 only its Create Flag advances (to 0x33, whose control is the
    // construction-view button); a click elsewhere shows the error and
    // changes nothing; a flag from the build menu does not advance 0x32.
    // spec: CAM-5.3c
    #[test]
    fn t19c_the_place_flag_menu_takes_only_its_create_flag() {
        use s2_sim::Command as C;
        let mut t = Tutorial::new(1);
        t.step = 0x31;
        assert!(opens_place_flag_menu(t.step));
        assert_eq!(t.field_click((42, 38)), FieldClick::Allow);
        assert_eq!(t.step, PLACE_FLAG_STEP);
        assert_eq!(step_key(t.step), "T01_StoneMason_SelectFlag");
        assert_eq!(pointer(t.step), None, "no map pointer");
        // A click beside the menu's controls reaches the map: refused.
        assert_eq!(t.field_click((40, 40)), FieldClick::Refuse);
        assert_eq!(t.error, Some("T01_ERROR_SelectMarkedButton"));
        assert_eq!(t.step, PLACE_FLAG_STEP);
        // Its close button: the error, not carried out.
        t.error = None;
        assert!(!t.place_flag_menu_click(&Action::CloseContext));
        assert_eq!(t.error, Some("T01_ERROR_SelectMarkedButton"));
        assert_eq!(t.step, PLACE_FLAG_STEP);
        // The build menu's flag cell: no advance.
        let flag = Action::Command(C::PlaceFlag {
            player: crate::game::PLAYER,
            at: (42, 38),
        });
        t.error = None;
        t.build_menu_click(&flag, 0);
        assert_eq!(t.step, PLACE_FLAG_STEP);
        t.action(&flag);
        assert_eq!(t.step, PLACE_FLAG_STEP);
        // Create Flag: carried out, 0x33, the construction-view button next.
        assert!(t.place_flag_menu_click(&flag));
        assert_eq!(t.step, 0x33);
        assert_eq!(step_key(t.step), "T01_HideConstructionMap");
        assert_eq!(t.error, None);
        assert_eq!(expects(t.step, &Action::Hud(3)), Some(true));
        // At any other step the menu's clicks are carried out as before.
        assert!(!opens_place_flag_menu(0x30) && !opens_place_flag_menu(0x32));
    }

    // CAM-5, UI-GAME-3: tutorial 1 played to its end on the real map: every
    // map click on the step's pointer field opens what the click opens
    // (the build menu, the flag menu, the road menu, the road's end), and
    // every button step finds its button, enabled, in the panels on the
    // screen (the flag menu's "Build road", the road menu's "Erect flag").
    #[test]
    fn tutorial_one_runs_to_its_end() {
        use crate::ui::game::{self, Catalog, Context, Ctx, GameUi};
        use s2_sim::Command as C;
        let Some((rules, mut world)) = install_game("tutorial01") else {
            return;
        };
        let tools = s2_rules::TOOL_SLOTS
            .iter()
            .filter_map(|n| rules.id_named(n))
            .collect();
        let catalog = Catalog::new(&rules, tools);
        let texts = Texts::default();
        let player = crate::game::PLAYER;
        let mut ui = GameUi::default();
        let mut t = Tutorial::new(1);
        let mut overlay = false;
        let mut steps = Vec::new();
        while !is_last_step(t.step) {
            let step = t.step;
            let name = step_key(step);
            steps.push(name);
            assert!(steps.len() < 60, "stuck at {name}");
            if let Some(field) = pointer(step) {
                assert_eq!(t.field_click(field), FieldClick::Allow, "{name}");
                match ui.road_from.take() {
                    Some(from) => {
                        if world.flag_at(field) != Some(player) {
                            world.submit(C::PlaceFlag { player, at: field });
                        }
                        world.submit(C::BuildRoad {
                            player,
                            from,
                            to: field,
                        });
                    }
                    // CAM-5.3c (Q-1090): 0x31's click opens the place-flag
                    // menu.
                    None if opens_place_flag_menu(step) => {
                        ui.context = Some(Context::DeployFlag(field));
                    }
                    None => {
                        ui.context = game::context_for(&world, player, field);
                        assert!(
                            ui.context.is_some(),
                            "{name}: the click at {field:?} opens nothing"
                        );
                    }
                }
            } else if t.explains() {
                t.advance();
            } else {
                let ctx = Ctx {
                    world: &world,
                    player,
                    screen: [1024.0, 768.0],
                    texts: &texts,
                    catalog: &catalog,
                    click: None,
                    tutorial: true,
                    speed: 1,
                    real_seconds: 0.0,
                    posted_messages: &[],
                    ship_names: &ui.ship_names,
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
                let panels = game::panels(&ctx, &ui, false);
                let building = t01_building(name).and_then(|n| rules.building_named(n));
                let wanted = building.map(|b| b.id);
                // UI-GAME-3.2: the tab of the building's size.
                let tab = building.and_then(|b| b.size).map(|size| {
                    use s2_rules::model::BuildingSize as S;
                    match size {
                        S::Medium => 6,
                        S::Large | S::Harbor => 7,
                        _ => 5,
                    }
                });
                let action = panels
                    .iter()
                    .flat_map(|p| p.buttons())
                    .filter(|b| b.2 && expects(step, b.0) == Some(true))
                    .map(|b| b.0.clone())
                    .find(|a| match (a, wanted) {
                        (Action::Command(C::PlaceBuilding { building, .. }), Some(w)) => {
                            *building == w
                        }
                        (Action::BuildTab(t), _) => tab.is_none_or(|wanted| *t == wanted),
                        _ => true,
                    })
                    .unwrap_or_else(|| panic!("{name}: no button for it in {:?}", ui.context));
                if let Some(Context::DeployFlag(_)) = ui.context {
                    // CAM-5.3c (Q-1090): outside road building the menu has
                    // Create Flag alone.
                    assert_eq!(
                        panels
                            .iter()
                            .find(|p| p.id == PanelId::Context)
                            .map(|p| p.buttons().filter(|b| *b.0 != Action::CloseContext).count()),
                        Some(1),
                        "{name}"
                    );
                    assert!(t.place_flag_menu_click(&action), "{name}");
                } else {
                    t.action(&action);
                }
                match action {
                    Action::Command(c) => {
                        if matches!(c, C::PlaceFlag { .. } | C::PlaceBuilding { .. }) {
                            ui.context = None;
                        }
                        world.submit(c);
                    }
                    Action::Hud(3) => overlay = !overlay,
                    Action::BuildTab(tab) => {
                        if let Some(Context::Build { tab: open, .. }) = ui.context.as_mut() {
                            *open = tab;
                        }
                    }
                    Action::StartRoad(flag) => {
                        ui.road_from = Some(flag);
                        ui.context = None;
                    }
                    Action::CloseContext => ui.context = None,
                    other => panic!("{name}: {other:?} not played by this test"),
                }
            }
            assert_ne!(t.step, step, "{name} did not advance: {:?}", t.error);
            // The commands take effect at the next tick; let the carriers
            // and builders work a little between the steps.
            for _ in 0..40 {
                world.step();
            }
        }
        assert_eq!(step_key(t.step), "T01_End");
        assert!(!overlay, "the construction view is closed again");
        // The woodcutter's road (43, 34) - (45, 37) runs over (44, 36)
        // (TRN-ROAD-2.3, 2.4), so the stonemason's road ending there splits
        // it (TRN-ROAD-4.2): five roads.
        assert_eq!(world.roads().count(), 5, "{steps:?}");
        // UI-GAME T3: the headquarters' flag (the roads' end, (45, 37))
        // cannot be destroyed.
        let ctx = Ctx {
            world: &world,
            player,
            screen: [1024.0, 768.0],
            texts: &texts,
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
        let menu = game::flag_menu(&ctx, (45, 37));
        let destroy = menu
            .buttons()
            .find(|b| matches!(b.0, Action::Command(C::RemoveFlag { .. })))
            .expect("the destroy button");
        assert!(!destroy.2);
    }

    /// Keys the specs name that have no entry in the shipped language
    /// files; the game shows them as they are (UI-GAME-8.5).
    /// (`!Disk Full!`, UI-MENU-5.1: no text in the shipped files, Q-1019.)
    const SHOWN_AS_KEYS: &[&str] = &["!INVALID_MAP", "!INVALID_FILE", "!Disk Full!"];

    // UI-GAME-1.3: every `!` key the interface shows has an entry in the
    // language files, so no key is shown as itself, except those the specs
    // name without one.
    // spec: UI-GAME-1.3, UI-GAME-8.5, UI-MENU-2.1
    #[test]
    fn every_text_key_is_in_the_language_files() {
        let Some(root) = std::env::var_os("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let root = std::path::Path::new(&root);
        let mut texts = Texts::default();
        for file in [
            "controls.txt",
            "dialogs.txt",
            "briefings.txt",
            "tutorial.txt",
        ] {
            let rel = format!("data/language/{}/{file}", crate::ui::LANGUAGE);
            let path = s2_rules::resolve(root, &rel).expect("the language file");
            let data = crate::game::read_game_file(&path).expect("readable");
            texts.extend(Texts::parse(&s2_assets::lang::decode(&data)));
        }
        let sources = [
            include_str!("../ui/game/mod.rs"),
            include_str!("../ui/game/context.rs"),
            include_str!("../ui/game/globals.rs"),
            include_str!("../ui/game/hud.rs"),
            include_str!("../ui/game/windows.rs"),
            include_str!("../ui/mod.rs"),
            include_str!("menu/mod.rs"),
            include_str!("menu/dialogs.rs"),
            include_str!("menu/pages.rs"),
            include_str!("campaign.rs"),
            include_str!("../ui/economy.rs"),
        ];
        let mut missing = Vec::new();
        for source in sources {
            for piece in source.split("\"!").skip(1) {
                let Some(end) = piece.find('"').filter(|&end| end > 0) else {
                    continue;
                };
                let key = format!("!{}", &piece[..end]);
                // Formatted keys ("!Lesson{n}") and the language file
                // lines of this test.
                if key.contains('{') || key.contains('\\') || key.starts_with("!!") {
                    continue;
                }
                if texts.entry(&key).is_none()
                    && !missing.contains(&key)
                    && !SHOWN_AS_KEYS.contains(&key.as_str())
                {
                    missing.push(key);
                }
            }
        }
        // The formatted keys of the tutorial page (UI-MENU-1.6).
        for k in 1..=4 {
            for key in [format!("!Lesson{k}"), format!("!Info{k}")] {
                if texts.entry(&key).is_none() {
                    missing.push(key);
                }
            }
        }
        assert!(missing.is_empty(), "keys without text: {missing:?}");
    }

    // spec: CAM-5.2 (T8)
    #[test]
    fn t8_lesson_one_steps() {
        // CAM T8
        let mut t = Tutorial::new(1);
        assert_eq!(step_key(t.step), "T01_Introduction");
        assert!(t.explains());
        t.advance();
        assert_eq!(step_key(t.step), "T01_ShowConstructionMap");
        t.action(&Action::Hud(5));
        assert_eq!(t.error, Some("T01_ERROR_SelectMarkedButton"));
        t.action(&Action::Hud(3));
        assert_eq!(t.step, 0x1f);
        // A wrong map click keeps the step.
        assert_eq!(t.field_click((10, 10)), FieldClick::Refuse);
        assert_eq!(t.error, Some("T01_ERROR_SelectMarkedPosition"));
        assert_eq!(t.step, 0x1f);
        assert_eq!(t.field_click((42, 35)), FieldClick::Allow);
        assert_eq!(step_key(t.step), "T01_WoodCutter_SelectBuilding");
        assert_eq!(t.error, None);
        // Map clicks wait while a button is expected.
        assert_eq!(t.field_click((42, 35)), FieldClick::Refuse);
    }

    // spec: CAM-5.3a (T13), UI-GAME-4.5
    #[test]
    fn t13_lesson_four_voyage_steps() {
        use s2_sim::Command as C;
        let go = |direction| {
            Action::Command(C::ExpeditionGo {
                player: s2_sim::PlayerId(0),
                ship: 0,
                direction,
            })
        };
        let mut t = Tutorial::new(4);
        t.step = 0x08;
        // Step 0x08 finds the ship by its field; the ship steps after it
        // place no pointer.
        assert!(ship_by_field(0x08));
        assert_eq!(t.field_click((101, 63)), FieldClick::Allow);
        assert_eq!(t.step, 0x09);
        for step in 0x09..=0x0d {
            assert_eq!(pointer(step), None, "step {step:#x}");
        }
        // 0x09: only the compass button of direction 2.
        t.action(&go(0));
        assert_eq!(t.error, Some("T01_ERROR_SelectMarkedButton"));
        assert_eq!(t.step, 0x09);
        t.action(&go(2));
        assert_eq!(step_key(t.step), "T04_BuildPort_SelectShip");
        // 0x0a: a map click does nothing and says nothing, the left arrow is
        // wrong, the right fleet arrow goes on.
        assert_eq!(t.field_click((127, 35)), FieldClick::Refuse);
        assert_eq!((t.step, t.error), (0x0a, None));
        t.action(&Action::ShipArrow {
            ship: 1,
            right: false,
        });
        assert_eq!(t.error, Some("T01_ERROR_SelectMarkedButton"));
        t.action(&Action::ShipArrow {
            ship: 1,
            right: true,
        });
        assert_eq!(step_key(t.step), "T04_BuildPort_SelectIcon");
        // 0x0b: Construct harbor, whether or not the ship can found one.
        t.action(&Action::Command(C::ExpeditionBuildHarbor {
            player: s2_sim::PlayerId(0),
            ship: 1,
        }));
        assert_eq!(step_key(t.step), "T04_BuildPort_CloseDialog");
        t.action(&Action::CloseContext);
        assert_eq!(t.step, 0x0d);
    }

    // spec: CAM-5.3b (T14)
    #[test]
    fn t14_step_starts_move_the_camera() {
        // Lesson 3: finishing the introduction moves the camera to (43, 31);
        // finishing step 0x10 moves nothing.
        let mut t = Tutorial::new(3);
        assert_eq!(t.camera_move, None, "the first step keeps the start camera");
        t.advance();
        assert_eq!((t.step, t.camera_move.take()), (0x0f, Some((43, 31))));
        t.step = 0x10;
        t.advance();
        assert_eq!(t.camera_move, None);
        // Lesson 4: after the harbour window's close, (101, 63).
        let mut t = Tutorial::new(4);
        t.step = 0x07;
        t.advance();
        assert_eq!(t.camera_move, Some((101, 63)));
        // 17 step starts, each a pointer step.
        let moving: Vec<u8> = (0..=0x50).filter(|&s| moves_camera(s)).collect();
        assert_eq!(moving.len(), 16);
        assert!(moving.iter().all(|&s| pointer(s).is_some()));
    }

    // spec: CAM-4.2 (T9)
    #[test]
    fn t9_campaign_unlock_chain() {
        let mut s = Settings::default();
        assert!(unlocked(&s, 1));
        assert!(!unlocked(&s, 2));
        s.mark_completed("campaign01");
        assert!(unlocked(&s, 2));
        assert!(!unlocked(&s, 3));
        // The first incomplete mission stops the chain.
        s.mark_completed("campaign03");
        assert!(!unlocked(&s, 4));
        assert_eq!(mission_name(10), "campaign10");
    }

    // spec: CAM-1.3
    #[test]
    fn modal_book_opens_once() {
        let mut ui = MissionUi::new("campaign01", 1, 4);
        let briefing = || {
            vec![
                ScriptEvent::OpenBook("b01_A_00".into()),
                ScriptEvent::AddPage("b01_A_01".into()),
                ScriptEvent::AddPicture("ui_briefing_screen_1".into()),
            ]
        };
        ui.apply(briefing());
        ui.apply(briefing());
        let book = ui.book.clone().unwrap();
        assert_eq!(book.pages, ["b01_A_00", "b01_A_01"]);
        assert_eq!(book.picture.as_deref(), Some("ui_briefing_screen_1"));
        ui.apply(vec![ScriptEvent::AddDebriefing("b01_C_00".into())]);
        assert_eq!(ui.debriefing, ["b01_C_00"]);
    }

    // spec: CAM-3.6
    #[test]
    fn results_open_the_end_screen() {
        let mut ui = MissionUi::new("campaign01", 1, 4);
        ui.results(&[MissionEvent::Completed { value: 4 }], PlayerId(0));
        assert_eq!(ui.end, Some(EndScreen::Victory(4)));
        let mut ui = MissionUi::new("x", 0, 1);
        ui.results(
            &[MissionEvent::Lost {
                player: PlayerId(1),
                reason: LossReason::CastleDestroyed,
            }],
            PlayerId(0),
        );
        assert_eq!(ui.end, None);
        assert!(ui.notice.is_some());
        ui.results(
            &[MissionEvent::Lost {
                player: PlayerId(0),
                reason: LossReason::CastleDestroyed,
            }],
            PlayerId(0),
        );
        assert_eq!(ui.end, Some(EndScreen::Defeat));
        assert_eq!(lesson_of("Tutorial03"), Some(3));
        assert_eq!(
            MissionUi::new("tutorial02", 2, 0).tutorial.unwrap().step,
            0x35
        );
    }
}
