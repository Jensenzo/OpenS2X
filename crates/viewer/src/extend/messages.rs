//! Hook W24: a message to the player when a project's rule finishes its
//! work (a mine's deepening, W15; a road's pavement, W19). The original has
//! neither rule, so it has no such message; without the hook nothing of
//! this runs. The messages are the viewer's (as the autosave's), not game
//! state: a load starts watching again from what it finds.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use s2_sim::grid::Point;
use s2_sim::{PlayerId, World};

/// Hook W24: an event of a project's rule the player can be told of.
/// Non-exhaustive: a `match` on it needs a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProjectEvent {
    /// W15: one of the player's mines has finished a deepening; the message
    /// stands at the mine and names its type, as "Building constructed"
    /// does.
    MineDeepened,
    /// W19: one of the player's roads is paved (its last stone laid); the
    /// message stands at the road's first flag.
    RoadPaved,
}

/// The project's texts, kept for the program's life (the message list
/// holds `&'static str`).
static TEXTS: OnceLock<Vec<(ProjectEvent, &'static str)>> = OnceLock::new();

/// Keep the project's texts ([`crate::extend::Extensions::messages`]). The
/// first call wins; an empty list keeps the hook off.
pub(crate) fn install(messages: Vec<(ProjectEvent, String)>) {
    let texts = messages
        .into_iter()
        .filter(|(_, t)| !t.is_empty())
        .map(|(e, t)| (e, &*Box::leak(t.into_boxed_str())))
        .collect();
    let _ = TEXTS.set(texts);
}

/// The project's text for `event`, if it gave one.
pub(crate) fn text_of(event: ProjectEvent) -> Option<&'static str> {
    text_in(TEXTS.get().map_or(&[][..], Vec::as_slice), event)
}

fn text_in(texts: &[(ProjectEvent, &'static str)], event: ProjectEvent) -> Option<&'static str> {
    texts.iter().find(|(e, _)| *e == event).map(|(_, t)| *t)
}

/// Whether `text` is one of the project's message texts: its row shows it
/// as given (no language key) with the misc kind's icon (UI-GAME-11.1's
/// kind 4, as "Building constructed").
pub(crate) fn is_project_text(text: &str) -> bool {
    TEXTS
        .get()
        .is_some_and(|t| t.iter().any(|(_, own)| *own == text))
}

/// Whether the hook is on (the project gave a text).
pub(crate) fn active() -> bool {
    TEXTS.get().is_some_and(|t| !t.is_empty())
}

/// What the watch saw at its last look: the player's mines with a
/// deepening and how many are done, the player's road numbers and which of
/// them were paved.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ProjectWatch {
    seen: Option<Seen>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Seen {
    loads: u64,
    mines: BTreeMap<Point, u32>,
    roads: BTreeSet<u32>,
    paved: BTreeSet<u32>,
}

impl ProjectWatch {
    /// The events of `player` since the last look, each with its place and
    /// the building it names. The first look after a load (`loads` new)
    /// only notes what it finds.
    pub(crate) fn look(
        &mut self,
        world: &World,
        player: PlayerId,
        loads: u64,
    ) -> Vec<(ProjectEvent, Point, Option<u32>)> {
        let mines: BTreeMap<Point, u32> = world
            .buildings()
            .filter(|b| b.owner == player)
            .filter_map(|b| world.mine_deepening(b.at).map(|d| (b.at, d.done)))
            .collect();
        let own: Vec<_> = world.roads().filter(|r| r.owner == player).collect();
        let roads: BTreeSet<u32> = own.iter().map(|r| r.number).collect();
        let paved: BTreeSet<u32> = own
            .iter()
            .filter(|r| world.is_paved(r.number))
            .map(|r| r.number)
            .collect();
        let mut events = Vec::new();
        if let Some(before) = self.seen.as_ref().filter(|s| s.loads == loads) {
            for at in deepened(&before.mines, &mines) {
                let kind = world.building_at(at).map(|b| b.building_type);
                events.push((ProjectEvent::MineDeepened, at, kind));
            }
            // A road paved now that was there unpaved before (the halves of
            // a split take new numbers and keep the pavement: no message).
            for r in &own {
                let new = paved.contains(&r.number)
                    && before.roads.contains(&r.number)
                    && !before.paved.contains(&r.number);
                if let (true, Some(&flag)) = (new, r.path.first()) {
                    events.push((ProjectEvent::RoadPaved, flag, None));
                }
            }
        }
        self.seen = Some(Seen {
            loads,
            mines,
            roads,
            paved,
        });
        events
    }
}

/// The mines whose deepenings done rose between two looks (a mine first
/// seen with one done counts: its first deepening ended in between).
fn deepened(before: &BTreeMap<Point, u32>, now: &BTreeMap<Point, u32>) -> Vec<Point> {
    now.iter()
        .filter(|(at, done)| **done > before.get(at).copied().unwrap_or(0))
        .map(|(at, _)| *at)
        .collect()
}

/// Hook W24: the events of the local player since the last tick become
/// messages in the viewer's list (the autosave's), with the project's
/// texts, shown for UI-GAME-11.2's time.
pub(crate) fn post_project_messages(
    mut viewer: bevy::prelude::ResMut<crate::game::Viewer>,
    mut watch: bevy::prelude::Local<(Option<u64>, ProjectWatch)>,
) {
    if !active() {
        return;
    }
    let loads = viewer.loads;
    let Some(world) = viewer.world.as_ref() else {
        return;
    };
    let tick = world.tick();
    if watch.0 == Some(tick) {
        return;
    }
    watch.0 = Some(tick);
    let events = watch.1.look(world, crate::game::PLAYER, loads);
    for (event, at, building) in events {
        let Some(text) = text_of(event) else { continue };
        viewer.autosave.messages.push(s2_sim::Message {
            tick,
            player: crate::game::PLAYER,
            at,
            text,
            until: tick + crate::ui::game::MESSAGE_TICKS,
            building,
            ship: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(width: u32, height: u32) -> World {
        let n = (width * height) as usize;
        let terrain = s2_sim::map::Terrain {
            width,
            height,
            heights: vec![0; n],
            height_floor: -100,
            states: vec![s2_sim::map::state::FOR_BUILDING; n],
            samples: Vec::new(),
        };
        World::with_map(1, terrain, s2_sim::Rules::empty(), PlayerId(0))
    }

    // Hook W24: a road of the player that becomes paved gives one
    // RoadPaved at its first flag, once; the first look after a load only
    // notes; another player's road gives none.
    // spec: none (hook W24, a project's message; the original has no paving)
    #[test]
    fn a_paved_road_gives_one_message_at_its_first_flag() {
        let mut world = flat(20, 20);
        for (from, to) in [((4, 4), (8, 4)), ((4, 8), (8, 8))] {
            world.submit(s2_sim::Command::PlaceFlag {
                player: PlayerId(0),
                at: from,
            });
            world.submit(s2_sim::Command::PlaceFlag {
                player: PlayerId(0),
                at: to,
            });
            world.submit(s2_sim::Command::BuildRoad {
                player: PlayerId(0),
                from,
                to,
            });
            world.step();
        }
        let roads: Vec<(u32, Point)> = world.roads().map(|r| (r.number, r.path[0])).collect();
        assert_eq!(roads.len(), 2, "{roads:?}");
        let mut watch = ProjectWatch::default();
        assert!(watch.look(&world, PlayerId(0), 1).is_empty());
        world.debug_pave_road(roads[0].0);
        assert_eq!(
            watch.look(&world, PlayerId(0), 1),
            [(ProjectEvent::RoadPaved, roads[0].1, None)]
        );
        assert!(watch.look(&world, PlayerId(0), 1).is_empty());
        // Another player's view sees nothing of it.
        let mut other = ProjectWatch::default();
        other.look(&world, PlayerId(1), 1);
        world.debug_pave_road(roads[1].0);
        assert!(other.look(&world, PlayerId(1), 1).is_empty());
        // A load: the first look notes only.
        assert!(watch.look(&world, PlayerId(0), 2).is_empty());
    }

    // Hook W24: a mine whose deepenings done rose gives one MineDeepened;
    // one unchanged, or a deepening under way (done the same), none.
    // spec: none (hook W24, a project's message; the original has no deepening)
    #[test]
    fn a_finished_deepening_is_one_event() {
        let before = BTreeMap::from([((5, 5), 0), ((9, 9), 1)]);
        let now = BTreeMap::from([((5, 5), 1), ((9, 9), 1), ((12, 3), 1)]);
        assert_eq!(deepened(&before, &now), [(5, 5), (12, 3)]);
        assert!(deepened(&now, &now).is_empty());
    }

    // Hook W24: the texts kept are the project's; none given, none found,
    // and an empty text is no message.
    // spec: none (hook W24)
    #[test]
    fn the_projects_texts() {
        let texts = [(ProjectEvent::MineDeepened, "Mine deepened")];
        assert_eq!(
            text_in(&texts, ProjectEvent::MineDeepened),
            Some("Mine deepened")
        );
        assert_eq!(text_in(&texts, ProjectEvent::RoadPaved), None);
        assert_eq!(text_in(&[], ProjectEvent::RoadPaved), None);
    }
}
