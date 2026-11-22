//! The program: the map viewer (`s2-viewer`, 2D and 3D, with the debug
//! panels) and the game (`OpenS2X`, the original menus and interface).
//! Bevy does the drawing and the input and nothing else: the rules are the
//! simulation's (`s2-sim`), files are read by `s2-assets`, the rule set
//! comes from `s2-rules`, computer players from `s2-ai`.
//!
//! One directory per job (`docs/architecture.md`, "The viewer"):
//!
//! | Module | Job |
//! |---|---|
//! | `app` | command-line options, the Bevy app with its plugins and systems |
//! | `game` | the viewer's state, loading a map, the game clock and the step of the world |
//! | `input` | the 2D view's input and the debug mode's keyboard editing |
//! | `render` | drawing: the 3D view, terrain, models, textures, the 2D map view |
//! | `world` | scene building: what the 3D view draws from the simulation |
//! | `animation` | clips, poses, which clip a settler or object plays |
//! | `ui` | the in-game interface and the debug panels |
//! | `menus` | the menus outside the game, campaign, the settings file |
//! | `audio` | sound effects and music |
//! | `films` | the films and their subtitles |
//! | `tools` | the play harness, colour probe, model view, screenshots and scenes |
//!
//! A directory module that was one large file keeps its parts' items
//! together through `use super::*` and re-exports them; the modules made
//! from the old `lib.rs` import what they use by name.

mod animation;
mod app;
mod audio;
pub mod extend;
mod files;
mod films;
mod game;
mod input;
mod menus;
mod render;
mod tools;
mod ui;
mod world;

pub use app::{run, run_with};

/// The player's folders (SET-1.1) and the game folder, as the game finds
/// them.
pub mod paths {
    pub use crate::app::game_dir;
    pub use crate::menus::settings::{documents, users_folder};
}
/// The engine the viewer draws with, for [`Extensions::plugins`].
pub use bevy;
pub use extend::{
    BuildingLook, ButtonPicture, ButtonPictures, CameraFollow, Captions, EffectRule, Extensions,
    MapHit, MapThing, MapThings, MusicSlot, ObjectLook, ProjectEvent, RenderOptions,
    SettingsOptions, ShadowFilter, ShoreFoam, StreetLook, TreeGrowth, TreeLook, MAX_SHADOW_MAP,
    TREE_FAR_BELOW,
};
