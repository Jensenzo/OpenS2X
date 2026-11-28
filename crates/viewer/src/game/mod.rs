//! The running game: the viewer's state, loading a map, the game clock and
//! the step of the world (with the computer players) every frame.

mod load;
mod names;
pub(crate) mod save;
pub(crate) use load::*;
pub(crate) use names::*;

use crate::render::terrain::DepositKind;
use bevy::prelude::*;
use s2_assets::s2m::Map;
use s2_rules::RuleSet;
use s2_sim::grid::Point;
use s2_sim::{Command, PlayerId, World};
use std::collections::HashMap;
use std::path::PathBuf;

/// Ticks a scripted attack may take after the expansion.
pub(crate) const ATTACK_LIMIT: u64 = 60_000;
/// The player the viewer plays: the one on the map's first start position.
pub(crate) const PLAYER: PlayerId = PlayerId(0);
/// ECO-HQ-2.2: the start resources setting of the viewer's games.
pub(crate) const START_RESOURCES: u8 = 0;
/// ECO-time-units: a logic tick is 0.1 s.
pub(crate) const TICKS_PER_SECOND: f32 = 10.0;

/// TOOL-1.2, TRACE-5: the scene's clocks. `step` is the game time step of
/// ECO-TIME-1.2 (frame seconds x speed), so it stops while the game is
/// paused or held and runs 2 and 3 times as fast at speeds 2 and 3; `ticks`
/// counts the logic ticks of this frame (the per-tick visuals, TRACE-5.3);
/// `world_seconds` is the world's game time, which the shaders read
/// (TRACE-5.2, GFX-OBJ-2.9). The clocks the specs keep on frame or real
/// time read Bevy's `Time` instead (`docs/clocks.md`).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct GameClock {
    /// Game seconds of this frame.
    pub step: f32,
    /// Game seconds since the program started.
    pub seconds: f32,
    /// Logic ticks run in this frame.
    pub ticks: u32,
    /// The world's game time in seconds (ECO-TIME-1.4).
    pub world_seconds: f64,
}

/// ECO-TIME-2.2: the tick length in seconds.
pub(crate) const TICK_SECONDS: f32 = 0.1;

/// ECO-TIME-1.4: the world's game time after `tick` ticks and `fraction`
/// of the next: one tick length each. (ECO-TIME-1.10 starts a new game at
/// -0.01 s before its first tick; the tick numbering is TOOL-trace's
/// TRACE-4.5, Q-314.)
pub fn world_seconds(tick: u64, fraction: f32) -> f64 {
    (tick as f64 + fraction.clamp(0.0, 1.0) as f64) / TICKS_PER_SECOND as f64
}

/// ECO-TIME-1.2, 1.3, 1.7: the game time step of a frame of `frame` real
/// seconds: frame x speed, 0 while paused or held; a step above 5 s is
/// replaced by one tick.
pub fn game_step(frame: f32, speed: u32, paused: bool) -> f32 {
    if paused {
        return 0.0;
    }
    let step = frame * speed as f32;
    if step > 5.0 {
        1.0 / TICKS_PER_SECOND
    } else {
        step
    }
}
pub(crate) const MAX_SPEED: u32 = 64;

/// Everything the view needs that is not drawn: the map list and the map on
/// screen.
#[derive(Resource)]
pub(crate) struct Viewer {
    pub(crate) maps: Vec<PathBuf>,
    pub(crate) current: usize,
    /// Set when `current` changed and the scene has to be rebuilt.
    pub(crate) reload: bool,
    pub(crate) map: Option<Map>,
    /// Why the current map is not shown, if it is not.
    pub(crate) error: Option<String>,
    /// Names of terrain and deposit types; absent without an install.
    pub(crate) rules: Option<RuleSet>,
    pub(crate) terrain_colors: HashMap<u32, [f32; 3]>,
    pub(crate) deposit_kinds: HashMap<u32, DepositKind>,
    /// The install's root, for terrain textures.
    /// The mark of this program's saves (`Extensions::project_name` and the
    /// rules overlays, W11 b): written in every save, and a save must have
    /// it to load.
    pub(crate) save_mark: s2_ai::save::SaveMark,
    pub(crate) game_root: Option<PathBuf>,
    pub(crate) screenshot_dir: PathBuf,
    pub(crate) shot: Option<PathBuf>,
    pub(crate) shot_after: u32,
    /// `--shot-frames`: the shot's consecutive frames (0 or 1: one).
    pub(crate) shot_frames: u32,
    pub(crate) frames_since_load: u32,
    pub(crate) shot_taken_at: Option<u32>,
    /// The simulation on the current map.
    pub(crate) world: Option<World>,
    /// B: show the placement overlay.
    pub(crate) overlay: bool,
    /// Building types with a plot size, in build-menu order.
    pub(crate) building_list: Vec<BuildingChoice>,
    pub(crate) page: usize,
    /// Index into `building_list` of the building to place on click.
    pub(crate) selected: Option<usize>,
    /// First flag of a road being built.
    pub(crate) road_start: Option<Point>,
    /// FMT-SAV: the save to continue at the next load.
    pub(crate) save_file: Option<PathBuf>,
    /// The file the game's map came from (an `.s2m`, or the original's
    /// save), which a save of ours names (`save::Header::map`).
    pub(crate) map_file: Option<PathBuf>,
    /// FMT-SAV-5.2: the periodic autosave's clock, and `--autosave`'s
    /// interval in place of the profile's.
    pub(crate) autosave: save::AutosaveClock,
    pub(crate) autosave_interval: Option<u32>,
    /// The outcome of the last action.
    pub(crate) status: String,
    /// Set when the world changed and its meshes have to be rebuilt.
    pub(crate) world_changed: bool,
    /// Set when the world ticked and carriers and wares have to be redrawn.
    pub(crate) moved: bool,
    pub(crate) paused: bool,
    /// UI-GAME-12.1 (P): the player paused the game (the paused view's
    /// layer, UI-GAME-1.11, while `paused` holds).
    pub(crate) pause_key: bool,
    /// UI-GAME-12.1 (Y), `--compare`: the interface (HUD and frame) hidden.
    pub(crate) hide_interface: bool,
    /// `--compare`: the shot waits for the 3D scene, then one frame.
    pub(crate) compare: bool,
    /// `--log-builders` (play test 10, F8).
    pub(crate) log_builders: bool,
    /// Ticks per real tick time: 1, 2, 4, ...
    pub(crate) speed: u32,
    /// Goods the debug key drops, with names: those with a transport priority
    /// that are not ware groups.
    pub(crate) ware_list: Vec<(u32, String)>,
    pub(crate) ware_choice: usize,
    /// First flag of a debug ware drop: the ware starts here.
    pub(crate) drop_from: Option<Point>,
    /// I: show the stock of a warehouse.
    pub(crate) show_stock: bool,
    /// L: list every building type with its production state and stock.
    pub(crate) show_production: bool,
    /// M: the metalworks tool priorities, with `tool_slot` selected.
    pub(crate) show_tools: bool,
    pub(crate) tool_slot: usize,
    /// The tool of each priority slot (ECO-PROD-7), empty without rules.
    pub(crate) tool_goods: Vec<u32>,
    /// F, R, Z: one of the ECO-DIST settings panels, or none.
    pub(crate) panel: Panel,
    /// Selected good of the warehouse panel, entry of the distribution
    /// panel (in table `dist_table`), position in the priority list.
    pub(crate) wh_good: usize,
    pub(crate) dist_table: usize,
    pub(crate) dist_entry: usize,
    pub(crate) rank_sel: usize,
    /// Selected entry of the military settings panel.
    pub(crate) mil_sel: usize,
    /// F4: tint every player's land.
    pub(crate) territory: bool,
    /// F8: show the border markers (TER-7).
    pub(crate) borders: bool,
    /// A: the enemy building the attack dialog is open on, and F9: "Use
    /// strong units first".
    pub(crate) attack: Option<Point>,
    pub(crate) attack_strong: bool,
    /// The cursor is over a building the dialog can open on (A then does
    /// not pan).
    pub(crate) attack_hover: bool,
    /// F10: the message list.
    pub(crate) show_messages: bool,
    /// Insert: the ship the expedition keys act on, an index into the fleet.
    pub(crate) ship_choice: usize,
    /// Ticks to run a scripted expansion of all players at load, and the
    /// start resources setting of the game.
    pub(crate) scenario: Option<u64>,
    /// The computer players asked for, and the AI system that plays them.
    pub(crate) ai_slots: Vec<(PlayerId, u8)>,
    pub(crate) ai: Option<s2_ai::AiSystem>,
    pub(crate) compact: bool,
    pub(crate) attack_run: Option<s2_rules::scenario::AttackStop>,
    pub(crate) start_resources: u8,
    /// CAM-3.1a: the free game setup's chosen victory condition (0 to 3,
    /// UI-SETUP-1.8), the target a new free game plays with.
    pub(crate) start_target: u8,
    pub(crate) focus: Option<(u32, u32, f32)>,
    /// Wares then settlers a warehouse has flags for, empty without rules.
    pub(crate) warehouse_goods: Vec<crate::ui::economy::PanelGood>,
    /// Names of building types, the construction site included.
    pub(crate) building_names: HashMap<u32, String>,
    /// Names of goods, wares and settlers alike.
    pub(crate) good_names: HashMap<u32, String>,
    /// Set when the world is a real game; without rules every field is the
    /// player's.
    pub(crate) real_game: bool,
    /// The game came from a save (GFX-OBJ-1.4, T32: its flags were in the
    /// file), and from a save of ours (its deposits are the world's).
    pub(crate) saved_game: bool,
    pub(crate) own_save: bool,
    /// GFX-ANIM-3.8c1: the tick a save was loaded at (none for a new
    /// game): the view's trees have no fall or sink from before it.
    pub(crate) loaded_at: Option<u64>,
    /// GFX-VIEW-5.4 (g): the settlers' seen marks of the ticks a tool
    /// played ahead with the camera on the start view, for the view's first
    /// frame to take over.
    pub(crate) ahead_marks: Option<crate::render::view3d::PlayAhead>,
    /// GFX-OBJ-2.2a: the house slots of those ticks, for the view to start
    /// from.
    pub(crate) ahead_slots: Option<crate::world::scene3d::HouseSlots>,
    /// GFX-ANIM-3.8a: the fall axes of the trees felled in those ticks.
    pub(crate) ahead_fall_axes: std::collections::HashMap<s2_sim::grid::Point, f32>,
    /// GFX-VIEW-2.1b (a): the heights when the map view was set up (the
    /// game's start or load, before a tool's play-ahead): the terrain keeps
    /// their normals outside the corner the refresh covers.
    pub(crate) setup_terrain: Option<s2_sim::map::Terrain>,
    /// `--no-fog`: every field drawn as explored (Q-290).
    pub(crate) no_fog: bool,
    /// ECO-HQ-3.6, UI-SETUP-1.8: the free game setup's "Fog of war" set to
    /// Off (the drop-down's choice when the game was started): the new
    /// game explores the whole map for every player at its start; nothing
    /// in the drawing reads it (GFX-VIEW-5.3).
    pub(crate) start_fog_off: bool,
    /// `--pause-layer`: the paused view's layer (UI-GAME-1.11) over a
    /// `--compare` shot whose interface is hidden.
    pub(crate) pause_layer: bool,
    /// The program is the game (`OpenS2X`), not the map viewer: its window
    /// keeps the title "OpenS2X" whatever map is loaded.
    pub(crate) program_game: bool,
    /// The textures of the current map's terrain types, by type id.
    pub(crate) patterns: HashMap<u32, crate::render::terrain::PatternTexture>,
    /// Maps loaded so far: tells the 3D view that the map changed.
    pub(crate) loads: u64,
    /// `--run`: ticks to play at load.
    pub(crate) run: u64,
    /// How far real time is into the current tick, 0 to 1: the 3D view
    /// blends positions between ticks by it (GFX-ANIM-4.1).
    pub(crate) tick_fraction: f32,
    /// `--debug`: the keyboard panels and text overlay instead of the
    /// original interface.
    pub(crate) debug: bool,
    /// The last refusal of one of the player's commands, for the
    /// interface's message box (UI-GAME-1.4).
    pub(crate) refusal: Option<String>,
    /// A notice for the player (CAM-3.2 a defeated computer player, a
    /// script's error), shown for 5 seconds.
    pub(crate) notice: Option<String>,
    /// `--open`, `--open-global`: dialogs to open once the game runs.
    pub(crate) open: Option<(u32, u32)>,
    pub(crate) open_tab: Option<u8>,
    pub(crate) open_global: Vec<String>,
    pub(crate) open_building: bool,
    /// `--building-type`: the type `--open-building` looks for.
    pub(crate) building_type: Option<String>,
    /// `--context-at`: the screen point `--open-building`'s window opens at.
    pub(crate) context_at: Option<[f32; 2]>,
    /// `--watch`: the 3D camera follows the worker of the player's first
    /// building of this type.
    pub(crate) watch: Option<String>,
    /// `--until-work`: `--run` goes on until the watched worker is in this
    /// step.
    pub(crate) until_work: Option<String>,
    /// `--until-felled`: `--run` goes on until a tree of the player's was
    /// felled this many ticks ago; the 3D camera follows it.
    pub(crate) until_felled: Option<u64>,
    /// `--until-falling`: a tree's felling began that many ticks ago.
    pub(crate) until_falling: Option<u64>,
    /// `--until-door`: the type whose door `--run` waits for.
    pub(crate) until_door: Option<String>,
    pub(crate) until_built: Option<u32>,
    /// `--scene`: the set-up made at load.
    pub(crate) scene: Option<String>,
    /// A field the interface asked the camera to move over.
    pub(crate) focus_request: Option<Point>,
    /// UI-GAME-11.4: a jump to a field by the rule of GFX-VIEW-3.3.
    pub(crate) jump_request: Option<Point>,
    /// The players of the free game the menu started (UI-SETUP-2),
    /// used by the next map load.
    pub(crate) setup: Option<Vec<crate::menus::menu::SetupPlayer>>,
    /// `--tutorial-step`
    pub(crate) tutorial_step: Option<u8>,
    /// `--end-screen`
    pub(crate) end_screen: Option<crate::menus::campaign::EndScreen>,
}

impl Viewer {
    /// GFX-VIEW-5.1: whether the local player's fog of war covers `field`
    /// (never in a game without rules, nor with `--no-fog` or fog of war
    /// Off, GFX-VIEW-5.3).
    pub(crate) fn fogged(&self, world: &World, field: Point) -> bool {
        self.fog_shown() && !world.is_explored(PLAYER, field)
    }

    /// Whether the fog of war is drawn at all: GFX-VIEW-5.3, with the
    /// scene's switch off the whole world texture is lit.
    pub(crate) fn fog_shown(&self) -> bool {
        fog_drawn(self.real_game, self.no_fog)
    }
}

/// GFX-VIEW-5.1, 5.3: whether the fog of war is drawn: in a game with rules
/// and without `--no-fog`. The setup's fog of war Off is no drawing switch:
/// it explores the map at the start (ECO-HQ-3.6), and the lit map is the
/// explored map.
pub(crate) fn fog_drawn(real_game: bool, no_fog: bool) -> bool {
    real_game && !no_fog
}

/// The ECO-DIST settings panels; at most one is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    None,
    /// F: per-good stop / take-out flags of a warehouse (ECO-DIST-1).
    Warehouse,
    /// R: the six distribution tables (ECO-DIST-2).
    Distribution,
    /// Z: the transport priority list (ECO-DIST-4).
    Priority,
    /// F1: the military settings (MIL-SOL-7).
    Military,
}

/// Marks the entities drawn from the simulation (overlay, flags, roads,
/// buildings).
#[derive(Component)]
pub(crate) struct WorldEntity;

/// Marks the entity of carriers and wares, redrawn as the world ticks.
#[derive(Component)]
pub(crate) struct MovingEntity;

/// Marks the entity of the simulation's trees and stone piles.
#[derive(Component)]
pub(crate) struct DepositEntity;

/// Marks the entities that belong to the map on screen.
#[derive(Component)]
pub(crate) struct MapEntity;

#[derive(Component)]
pub(crate) struct InfoText;

/// The large game speed line at the top right.
#[derive(Component)]
pub(crate) struct SpeedText;

/// The player a command is given for.
pub(crate) fn command_player(command: &Command) -> Option<PlayerId> {
    let text = format!("{command:?}");
    let at = text.find("player: PlayerId(")? + "player: PlayerId(".len();
    let end = text[at..].find(')')?;
    text[at..at + end].parse().ok().map(PlayerId)
}

/// What became of a command, for the status line.
pub(crate) fn describe_result(world: &World, result: &s2_sim::CommandResult) -> String {
    match (&result.command, result.outcome) {
        (_, Err(reason)) => format!("refused: {reason}"),
        (Command::PlaceFlag { at, .. }, Ok(())) => format!("flag placed at ({}, {})", at.0, at.1),
        (Command::BuildRoad { .. } | Command::PlanRoad { .. }, Ok(())) => {
            let road = world.roads().last().expect("the new road");
            format!(
                "road built, {} steps, carrier requested",
                road.path.len() - 1
            )
        }
        (Command::BuildWaterway { .. }, Ok(())) => {
            let road = world.roads().last().expect("the new waterway");
            format!(
                "waterway built, {} steps, boat carrier requested",
                road.path.len() - 1
            )
        }
        (Command::PlaceBuilding { at, .. } | Command::PlanBuilding { at, .. }, Ok(())) => {
            format!("construction site placed at ({}, {})", at.0, at.1)
        }
        (Command::RemoveBuilding { at, .. } | Command::TearDown { at, .. }, Ok(())) => {
            format!("building at ({}, {}) removed", at.0, at.1)
        }
        (Command::UpgradeBuilding { at, .. }, Ok(())) => {
            format!("upgrade of the building at ({}, {}) started", at.0, at.1)
        }
        (
            command @ (Command::ExpeditionStart { .. }
            | Command::ExpeditionStop { .. }
            | Command::ExpeditionBuildHarbor { .. }
            | Command::PlanHarbour { .. }
            | Command::ExpeditionGo { .. }),
            Ok(()),
        ) => crate::world::ships::describe(command).unwrap_or_default(),
        (Command::RemoveRoad { at, .. }, Ok(())) => {
            format!("road at ({}, {}) destroyed", at.0, at.1)
        }
        (Command::RemoveFlag { at, .. }, Ok(())) => {
            format!("flag at ({}, {}) destroyed", at.0, at.1)
        }
        (
            Command::DropWare {
                at, destination, ..
            },
            Ok(()),
        ) => format!(
            "ware dropped at ({}, {}) for ({}, {})",
            at.0, at.1, destination.0, destination.1
        ),
        (Command::DebugSupplySite { at, wares, .. }, Ok(())) => {
            let count: u32 = wares.iter().map(|&(_, n)| n).sum();
            format!(
                "debug: {count} materials and the workers put on the site at ({}, {})",
                at.0, at.1
            )
        }
        (Command::ProductionToggle { at, .. }, Ok(())) => {
            let on = world.production_at(*at).is_some_and(|p| p.enabled);
            format!(
                "production at ({}, {}) {}",
                at.0,
                at.1,
                if on { "on" } else { "off" }
            )
        }
        (Command::ProductionSwitch { at, .. }, Ok(())) => {
            format!("production switch toggled at ({}, {})", at.0, at.1)
        }
        (Command::ChooseRecipe { at, index, .. }, Ok(())) => {
            format!("recipe {} chosen at ({}, {})", index + 1, at.0, at.1)
        }
        (Command::DeepenMine { at, .. }, Ok(())) => {
            format!("deepening ordered at ({}, {})", at.0, at.1)
        }
        (Command::PaveRoad { at, .. }, Ok(())) => {
            format!("paving ordered at ({}, {})", at.0, at.1)
        }
        (
            Command::ToolWeight { .. }
            | Command::ToolExclusive { .. }
            | Command::ToolClearExclusive { .. }
            | Command::ToolReset { .. },
            Ok(()),
        ) => "tool priorities changed".to_string(),
        (
            Command::WarehouseCycle { .. }
            | Command::WarehouseToggle { .. }
            | Command::WarehouseSet { .. }
            | Command::WarehouseAll { .. },
            Ok(()),
        ) => "warehouse flags changed".to_string(),
        (Command::DistributionWeight { .. } | Command::DistributionReset { .. }, Ok(())) => {
            "distribution changed".to_string()
        }
        (Command::PriorityMove { .. } | Command::PriorityReset { .. }, Ok(())) => {
            "transport priority changed".to_string()
        }
        (
            Command::MilitaryAllocation { .. }
            | Command::MilitaryAttackStrength { .. }
            | Command::MilitaryDefenders { .. }
            | Command::MilitaryDefenseType { .. }
            | Command::MilitaryReset { .. }
            | Command::ToggleCoinSupply { .. }
            | Command::ToggleMoveOut { .. }
            | Command::SetCastleSoldiers { .. },
            Ok(()),
        ) => "military setting changed".to_string(),
        (Command::Attack { target, count, .. }, Ok(())) => {
            format!(
                "attack on ({}, {}) ordered, up to {count}",
                target.0, target.1
            )
        }
        (
            Command::AddGoods { .. }
            | Command::ClearGoods { .. }
            | Command::LockBuilding { .. }
            | Command::CreateBuilding { .. }
            | Command::CreateStreet { .. }
            | Command::CreateShip { .. }
            | Command::RemoveTrigger { .. },
            Ok(()),
        ) => "mission script".to_string(),
        (Command::OrderGeologist { at, .. }, Ok(())) => {
            format!("geologist called to ({}, {})", at.0, at.1)
        }
        (Command::OrderScout { at, .. }, Ok(())) => {
            format!("scout called to ({}, {})", at.0, at.1)
        }
        (Command::Cheat { cheat, .. }, Ok(())) => format!("cheat: {cheat:?}"),
        (Command::NameShip { name, .. }, Ok(())) => format!("ship named {name}"),
        (Command::Noop { .. }, Ok(())) => String::new(),
        (Command::StartRoute { at, .. }, Ok(())) => {
            format!("road building from ({}, {})", at.0, at.1)
        }
        (Command::ExtendRoute { .. }, Ok(())) | (Command::CancelRoute { .. }, Ok(())) => {
            String::new()
        }
    }
}

/// The routes being built, by their fields (TRN-ROAD-8).
pub(crate) fn route_signature(world: &World) -> Vec<(PlayerId, usize)> {
    world.routes().map(|(p, r)| (p, r.fields.len())).collect()
}

/// What of the sites changes without a command: levelling steps change
/// heights and placement classes, and finished sites become buildings.
pub(crate) fn site_signature(world: &World) -> (usize, u64) {
    let levelling = world
        .sites()
        .map(|(_, site)| u64::from(site.levelling_bits))
        .sum();
    (world.sites().count(), levelling)
}

/// Run the simulation in real time: 10 ticks per second (ECO-time-units),
/// times the chosen speed; Space pauses, + and - change the speed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_world(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut viewer: ResMut<Viewer>,
    mut owed: Local<f32>,
    mut clock: ResMut<GameClock>,
    mut mission: ResMut<crate::menus::campaign::MissionUi>,
    script: NonSend<crate::menus::campaign::MissionScriptRes>,
    mut menu: ResMut<crate::menus::menu::MenuState>,
    (mut run_check, mut spans): (
        Option<ResMut<crate::tools::bench::RunCheck>>,
        Option<ResMut<crate::tools::bench::FrameSpans>>,
    ),
) {
    // CAM-3.5: a completed map is recorded and acknowledged. CAM-3.6
    // (E-trace-033, E-cam-018): the world runs on behind the end screen,
    // which holds no clock.
    if mission.record_completed(&mut menu.settings) {
        menu.settings.save();
    }
    if let Some(notice) = mission.notice.take() {
        viewer.notice = Some(notice);
    }
    // The game pauses with P (UI-GAME-12.1, `crate::ui::game_keys`); the debug
    // mode with Space.
    if viewer.debug && keys.just_pressed(KeyCode::Space) {
        viewer.paused = !viewer.paused;
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        viewer.speed = (viewer.speed * 2).min(MAX_SPEED);
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        viewer.speed = (viewer.speed / 2).max(1);
    }
    // TOOL-1.2: the animation clock takes the same step as the logic.
    let held = viewer.paused || viewer.world.is_none();
    // `--shot-frames`: a sequence's frames are 1/60 s of game time each.
    let frame = match viewer.shot.is_some() && viewer.shot_frames > 1 {
        true => crate::tools::shots::SEQUENCE_FRAME,
        false => time.delta_secs(),
    };
    clock.step = game_step(frame, viewer.speed, held);
    clock.seconds += clock.step;
    clock.ticks = 0;
    clock.world_seconds = viewer
        .world
        .as_ref()
        .map_or(0.0, |w| world_seconds(w.tick(), viewer.tick_fraction));
    if held {
        // ECO-TIME-1.8: paused, the step is 0 and the elapsed time stands
        // (the share of the next tick is kept).
        if viewer.world.is_none() {
            *owed = 0.0;
        }
        return;
    }
    *owed += time.delta_secs() * TICKS_PER_SECOND * viewer.speed as f32;
    // Never more than a second's worth of ticks in one frame.
    let ticks = (owed.floor() as u32).min(TICKS_PER_SECOND as u32 * viewer.speed);
    *owed -= ticks as f32;
    *owed = owed.min(1.0);
    viewer.tick_fraction = *owed;
    let mut statuses = Vec::new();
    let mut changed = false;
    let mut refusal = None;
    let Viewer { world, ai, .. } = &mut *viewer;
    let world = world.as_mut().expect("checked above");
    let before = site_signature(world);
    let routes_before = route_signature(world);
    let fraction = *owed;
    for _ in 0..ticks {
        clock.ticks += 1;
        // AI-2.1a: the computer players think last in the tick.
        match ai.as_mut() {
            Some(ai) => ai.step(world),
            None => world.step(),
        }
        crate::menus::campaign::after_step(world, &script, &mut mission);
        if let Some(check) = run_check.as_mut() {
            check.after_tick(world, spans.as_deref_mut());
        }
        for result in world.last_results() {
            changed |= result.outcome.is_ok();
            statuses.push(describe_result(world, result));
            // UI-GAME-1.4: the first refusal of the frame is the cause
            // (a road to a free field places a flag first; when that
            // fails, so does the road).
            if let Err(reason) = result.outcome {
                if refusal.is_none() && command_player(&result.command) == Some(PLAYER) {
                    refusal = Some(reason.to_string());
                }
            }
        }
    }
    clock.world_seconds = world_seconds(world.tick(), fraction);
    // FMT-SAV-3.12: the script's variables for a save.
    if let (true, Some(s)) = (ticks > 0, script.0.as_ref()) {
        mission.script_variables = s.variables();
    }
    changed |= site_signature(world) != before;
    // TRN-ROAD-8.3: a route the logic finished or dropped by itself.
    changed |= route_signature(world) != routes_before;
    if let Some(status) = statuses.pop() {
        viewer.status = status;
    }
    if refusal.is_some() {
        viewer.refusal = refusal;
    }
    if changed {
        viewer.world_changed = true;
    }
    if ticks > 0 {
        viewer.moved = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (a tool option: `--expand` sets up new games only; CONTINUE
    // of a save, ours or the original's, goes on as saved)
    #[test]
    fn the_scripted_expansion_skips_a_loaded_game() {
        assert_eq!(load::scripted_expansion(Some(600), false), Some(600));
        assert_eq!(load::scripted_expansion(Some(600), true), None);
        assert_eq!(load::scripted_expansion(None, false), None);
    }

    /// GFX-VIEW-5.1, 5.3: the fog is drawn in a game with rules unless
    /// `--no-fog`; the setup's fog of war Off is no drawing switch (it
    /// explores the map at the start, ECO-HQ-3.6, tested in the sim).
    // spec: GFX-VIEW-5.3, GFX-VIEW-5.1
    #[test]
    fn fog_is_drawn_in_a_game_with_rules() {
        assert!(fog_drawn(true, false));
        assert!(!fog_drawn(true, true));
        assert!(!fog_drawn(false, false));
    }

    // TOOL-1.2, ECO-TIME-1.2, 1.3: the animation clock's step: none while
    // paused, 2 and 3 times the frame at speeds 2 and 3, one tick after a
    // stall.
    #[test]
    fn game_clock_step() {
        assert_eq!(game_step(0.025, 1, false), 0.025);
        assert_eq!(game_step(0.025, 2, false), 0.05);
        assert!((game_step(0.025, 3, false) - 0.075).abs() < 1e-7);
        assert_eq!(game_step(0.025, 3, true), 0.0);
        assert_eq!(game_step(2.0, 3, false), 0.1);
        assert_eq!(game_step(1.25, 4, false), 5.0);
    }
}
