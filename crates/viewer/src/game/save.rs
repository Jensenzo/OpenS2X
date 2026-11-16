//! The game's own save files (UI-MENU-5, FMT-SAV-5; `PLAYING.md`).
//!
//! The file format is `s2_ai::save`; here is what the program does with it:
//! when it saves, and with which name. The camera is not kept: a game
//! continued from a save starts at the headquarters, as every game
//! (GFX-VIEW-3.6). The files sit in the original's save folder (FMT-SAV-1.1)
//! under their own extension, so the original never lists them
//! (`docs/stand-ins.md`).

use std::path::{Path, PathBuf};

pub(crate) use s2_ai::save::{
    path_of, read_as, write, write_file, Header, Loaded, SavedMission, EXTENSION,
};
use s2_sim::World;

/// FMT-SAV-5.2: the periodic autosave's clock. The first check after a
/// game starts (or loads) only notes the game time; afterwards, when more
/// than `interval` seconds of game time have passed since the last save,
/// it is time to save again. Game time is the tick count at 0.1 s a tick
/// (ECO-TIME-1.4), in whole seconds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AutosaveClock {
    last: Option<u64>,
    /// FMT-SAV-5.2, UI-GAME-11.1: the `!Autosave...` messages (kind 7)
    /// posted to the human player, which the messages window lists with the
    /// simulation's.
    pub(crate) messages: Vec<s2_sim::Message>,
}

impl AutosaveClock {
    /// Whether to autosave at `tick` with the interval `interval` seconds
    /// (0: off). A `true` sets the clock to now.
    pub(crate) fn due(&mut self, tick: u64, interval: u32) -> bool {
        let now = tick / 10;
        match self.last {
            None => {
                self.last = Some(now);
                false
            }
            Some(last) if interval > 0 && now - last.min(now) > u64::from(interval) => {
                self.last = Some(now);
                true
            }
            Some(_) => false,
        }
    }

    /// A new game or a load: the next check only notes the time.
    pub(crate) fn reset(&mut self) {
        self.last = None;
        self.messages.clear();
    }

    /// FMT-SAV-5.2: the message `!Autosave...` (kind 7, no field) to the
    /// human player at `tick`, shown for the 36 000 ticks of UI-GAME-11.2.
    pub(crate) fn post(&mut self, tick: u64) {
        self.messages.push(s2_sim::Message {
            tick,
            player: crate::game::PLAYER,
            at: crate::ui::game::NO_FIELD,
            text: AUTOSAVE_MESSAGE,
            until: tick + crate::ui::game::MESSAGE_TICKS,
            building: None,
            ship: None,
        });
    }
}

/// What a save wrote: its file, the tick and the state hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCheck {
    pub path: PathBuf,
    pub tick: u64,
    pub world_hash: u64,
    pub ai_hash: Option<u64>,
}

/// A save of ours loaded: the hashes it was saved with, and those of the
/// game it gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadCheck {
    pub path: PathBuf,
    pub tick: u64,
    pub saved: (u64, Option<u64>),
    pub loaded: (u64, Option<u64>),
    /// Written in an earlier format, which this build brought up to date:
    /// the hashes cannot be the same (`docs/snapshot.md`).
    pub older_format: bool,
}

impl LoadCheck {
    /// The game loaded is the game saved.
    pub fn same(&self) -> bool {
        self.saved == self.loaded
    }
}

/// Whether this run saves its game by itself (FMT-SAV-5.2, 5.3): a game,
/// not a picture of one (`--shot`, `--compare`), in the game program, or
/// in the map viewer with a save folder of its own (`--save-dir`), so a
/// developer's run never writes the player's autosave.
/// FMT-SAV-5.2, UI-GAME-11.1: the periodic autosave's message (kind 7).
pub(crate) const AUTOSAVE_MESSAGE: &str = "Autosave...";

pub(crate) fn autosaves(viewer: &crate::game::Viewer) -> bool {
    viewer.real_game
        && viewer.world.is_some()
        && viewer.shot.is_none()
        && !viewer.compare
        && (viewer.program_game || crate::menus::menu::save_folder_set())
}

/// SET-2.1: the autosave interval, seconds of game time (0: off): the
/// profile's, or `--autosave`'s for this run.
pub(crate) fn interval(viewer: &crate::game::Viewer, menu: &crate::menus::menu::MenuState) -> u32 {
    viewer.autosave_interval.unwrap_or(menu.settings.autosave)
}

/// Save the viewer's game as `name` in `folder` (UI-MENU-5.1, FMT-SAV-5).
pub(crate) fn save_game(
    viewer: &crate::game::Viewer,
    mission: &crate::menus::campaign::MissionUi,
    folder: &Path,
    name: &str,
) -> Result<SaveCheck, String> {
    let world = viewer
        .world
        .as_ref()
        .filter(|_| viewer.real_game)
        .ok_or("there is no game to save")?;
    let map = viewer
        .map_file
        .as_ref()
        .ok_or("the game's map file is not known")?;
    let header = Header {
        map: map.to_string_lossy().into_owned(),
        tick: world.tick(),
        world_hash: world.state_hash(),
        ai_hash: viewer.ai.as_ref().map(s2_ai::AiSystem::state_hash),
        slots: viewer.ai_slots.clone(),
        mission: SavedMission {
            target: mission.target,
            tutorial_step: mission.tutorial.as_ref().map(|t| t.step),
            debriefing: mission.debriefing.clone(),
            extro: mission.extro.clone(),
            letters: mission.letters.clone(),
            script_variables: mission.script_variables.clone(),
        },
        mark: viewer.save_mark.clone(),
    };
    let path = path_of(folder, name);
    write_file(&path, &write(&header, world, viewer.ai.as_ref()))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    bevy::log::info!(
        "saved {}: tick {}, state hash {:016x}, computer players {:?}",
        path.display(),
        header.tick,
        header.world_hash,
        header.ai_hash
    );
    Ok(SaveCheck {
        path,
        tick: header.tick,
        world_hash: header.world_hash,
        ai_hash: header.ai_hash,
    })
}

/// FMT-SAV-5.2: the periodic autosave to the profile's autosave name (ours,
/// FMT-SAV-5.1's file beside the original's). The `!Autosave...` message
/// (kind 7) is not posted: the message list is the simulation's.
pub(crate) fn autosave(
    mut viewer: bevy::prelude::ResMut<crate::game::Viewer>,
    menu: bevy::prelude::Res<crate::menus::menu::MenuState>,
    mission: bevy::prelude::Res<crate::menus::campaign::MissionUi>,
) {
    if !menu.in_game || !autosaves(&viewer) {
        return;
    }
    let Some(tick) = viewer.world.as_ref().map(World::tick) else {
        return;
    };
    let interval = interval(&viewer, &menu);
    if !viewer.autosave.due(tick, interval) {
        return;
    }
    let Some(folder) = crate::menus::menu::save_folder() else {
        return;
    };
    // FMT-SAV-5.2: the message first, then the save.
    viewer.autosave.post(tick);
    let name = crate::menus::menu::autosave_name(&menu);
    if let Err(e) = save_game(&viewer, &mission, &folder, &name) {
        bevy::log::warn!("autosave not written: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: FMT-SAV-5.2
    #[test]
    fn the_autosave_clock_notes_the_time_first_then_saves_after_the_interval() {
        let mut clock = AutosaveClock::default();
        // The first check only notes the time (tick 1000: 100 s).
        assert!(!clock.due(1000, 300));
        // 300 s later is not more than the interval; 301 s is.
        assert!(!clock.due(4000, 300));
        assert!(clock.due(4010, 300));
        assert!(!clock.due(7010, 300));
        assert!(clock.due(7020, 300));
        // Off: never, and the clock stays.
        assert!(!clock.due(90_000, 0));
        assert!(clock.due(90_000, 60));
        // After a load the first check notes the time again.
        clock.reset();
        assert!(!clock.due(200_000, 60));
        assert!(clock.due(200_610, 60));
    }

    /// FMT-SAV-5.2 (T7), UI-GAME-11.1: the autosave posts `!Autosave...` (kind
    /// 7, always shown, no field) to the human player; the window lists it with
    /// the simulation's messages by arrival and for 36 000 ticks; a load
    /// clears it.
    // spec: FMT-SAV-5.2 (T7), UI-GAME-11.1, UI-GAME-11.2
    #[test]
    fn the_autosave_posts_its_message_to_the_list() {
        use crate::ui::game::{all_messages, listed_messages, message_kind, NO_FIELD};
        let mut clock = AutosaveClock::default();
        clock.post(3010);
        let m = &clock.messages[0];
        assert_eq!(
            (m.text, m.player, m.at),
            (AUTOSAVE_MESSAGE, crate::game::PLAYER, NO_FIELD)
        );
        assert_eq!(message_kind(m.text), 7);
        let world = [
            s2_sim::Message {
                tick: 3000,
                player: crate::game::PLAYER,
                at: (5, 5),
                text: "Building constructed",
                until: 39_000,
                building: None,
                ship: None,
            },
            s2_sim::Message {
                tick: 3020,
                player: crate::game::PLAYER,
                at: (6, 6),
                text: "Building constructed",
                until: 39_020,
                building: None,
                ship: None,
            },
        ];
        let none = std::collections::HashSet::new();
        let all = all_messages(&world, &clock.messages);
        // The misc filter off: the autosave's row stays (kinds 5 to 7).
        let listed = listed_messages(
            &all,
            crate::game::PLAYER,
            4000,
            [false, false, false, true],
            &none,
        );
        assert_eq!(
            listed.iter().map(|m| m.text).collect::<Vec<_>>(),
            [AUTOSAVE_MESSAGE]
        );
        let listed = listed_messages(&all, crate::game::PLAYER, 4000, [false; 4], &none);
        assert_eq!(
            listed.iter().map(|m| m.tick).collect::<Vec<_>>(),
            [3000, 3010, 3020]
        );
        let listed = listed_messages(&all, crate::game::PLAYER, 39_010, [false; 4], &none);
        assert_eq!(listed.iter().map(|m| m.tick).collect::<Vec<_>>(), [3020]);
        clock.reset();
        assert!(clock.messages.is_empty());
    }
}
