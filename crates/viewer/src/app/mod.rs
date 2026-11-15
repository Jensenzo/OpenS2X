//! The program: options, the app with its plugins and systems, the game
//! folder.

mod options;
pub(crate) use options::*;

use crate::game::{
    advance_world, building_list, building_names, cycle_maps, find_maps, good_names, load_map,
    load_rules, map_name, ware_list, warehouse_goods, GameClock, InfoText, Panel, SpeedText,
    Viewer,
};
use crate::input::{camera_controls, edit_world, hover_2d, quit_on_escape, toggle_3d, Hover};
use crate::render::view2d::{draw_deposits, draw_moving, draw_world};
use crate::tools::model::ViewMode;
use crate::tools::shots::{automatic_shot, screenshot_key};
use crate::ui::info::{hover_info, speed_hud};
use bevy::app::AppExit;
use bevy::prelude::*;
use s2_assets::GameDir;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The game folder of the settings, or one the player picks (first run);
/// a picked folder is kept in the settings when it holds the game.
pub(crate) fn find_game_dir() -> Option<PathBuf> {
    let mut settings = crate::menus::settings::Settings::load();
    if let Some(dir) = settings.game_dir.clone() {
        if GameDir::new(&dir).is_ok() {
            return Some(dir);
        }
    }
    let picked = pick_game_folder()?;
    if let Err(e) = GameDir::new(&picked) {
        tell(&format!(
            "{} does not hold The Settlers II: 10th Anniversary ({e}). Choose the folder that contains the data folder.",
            picked.display()
        ));
        return None;
    }
    settings.game_dir = Some(picked.clone());
    settings.save();
    Some(picked)
}

/// The game folder the program runs with, kept by [`run_with`].
static GAME_FOLDER: OnceLock<PathBuf> = OnceLock::new();

/// The game folder the program runs with, as [`run_with`] found it:
/// `--game-dir`, else `S2_GAME_DIR`, else (the game) the program's
/// settings file, where the first run's folder picker puts it; only a
/// folder that is there. `None` before `run_with` has looked (it has when a
/// project's plugins are added) and without one. Read only.
pub fn game_dir() -> Option<PathBuf> {
    GAME_FOLDER.get().cloned()
}

/// Keep the game folder for [`game_dir`]; the first call wins.
fn keep_game_dir(dir: Option<&Path>) {
    if let Some(dir) = dir {
        let _ = GAME_FOLDER.set(dir.to_path_buf());
    }
}

/// The window's size: the game takes the profile's resolution (SET-4),
/// the viewer a fixed one.
pub(crate) fn window_size(game: bool) -> (u32, u32) {
    if !game {
        return (1280, 900);
    }
    let (w, h) = crate::menus::settings::Settings::load().resolution();
    (w.max(800), h.max(600))
}

#[cfg(windows)]
pub(crate) fn pick_game_folder() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(crate::extend::picker_title())
        .pick_folder()
}

#[cfg(not(windows))]
pub(crate) fn pick_game_folder() -> Option<PathBuf> {
    eprintln!("set S2_GAME_DIR or pass --game-dir to the game folder");
    None
}

#[cfg(windows)]
pub(crate) fn tell(text: &str) {
    rfd::MessageDialog::new()
        .set_title(crate::extend::window_title())
        .set_description(text)
        .show();
}

#[cfg(not(windows))]
pub(crate) fn tell(text: &str) {
    eprintln!("{text}");
}

/// Run the program: the viewer, or with `game` the game: it opens at the
/// main menu in the 3D view (unless a map is named) and finds the game
/// folder by `--game-dir`, `S2_GAME_DIR`, the settings, or by asking.
pub fn run(game: bool) -> std::process::ExitCode {
    run_with(game, crate::extend::Extensions::default())
}

/// [`run`] with a project's extensions (`docs/extending.md`); with
/// `Extensions::default()` it is [`run`]. `--assets` and `--rules-overlay`
/// add to them from the command line.
pub fn run_with(game: bool, extensions: crate::extend::Extensions) -> std::process::ExitCode {
    let crate::extend::Extensions {
        mut rules_overlays,
        mut asset_folders,
        render,
        plugins,
        menu: menu_entries,
        captions,
        skip_launch_films,
        music_external,
        window_title,
        settings,
        on_settings_change,
        map_things,
        screenshot_dir: project_shots,
        save_dir: project_saves,
        project_name,
    } = extensions;
    crate::extend::install_program(window_title, settings, on_settings_change);
    let (entries, pages) = crate::extend::menu::split(menu_entries);
    let options = match parse_args(game) {
        Ok(mut options) => {
            // `--profile-dir` before anything reads the settings: the first
            // run's look for the game folder (and the folder question's
            // answer) uses that folder's settings, not the player's.
            if let Some(dir) = &options.profile_dir {
                crate::menus::settings::keep_files_in(dir.clone());
            }
            if game {
                if options.map.is_none() && options.map_name.is_none() && options.save.is_none() {
                    options.intro = !options.menu && options.menu_screen.is_none();
                    options.menu = true;
                }
                if options.game_dir.is_none() {
                    options.game_dir = find_game_dir();
                }
            }
            options
        }
        Err(message) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
            eprintln!("{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };
    // `--camera`: the look comparison's camera (TRACE-8).
    let fixed_eye = match &options.camera {
        None => None,
        Some(path) => {
            let read = std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|text| crate::tools::camera_file::CameraFile::parse(&text));
            match read {
                Ok(c) => c.eye.zip(c.dir),
                Err(e) => {
                    eprintln!("--camera {}: {e}", path.display());
                    return std::process::ExitCode::from(2);
                }
            }
        }
    };

    let game_dir = options
        .game_dir
        .as_ref()
        .and_then(|dir| match GameDir::new(dir) {
            Ok(dir) => Some(dir),
            Err(e) => {
                eprintln!("{e}");
                None
            }
        });
    keep_game_dir(game_dir.as_ref().map(GameDir::root));
    // Models, animations, sounds and textures read ahead off the main
    // thread (`files`), so a game's first reads do not wait for the disk.
    if let Some(dir) = game_dir.as_ref() {
        crate::files::start(dir.root());
    }
    asset_folders.extend(options.asset_folders.iter().cloned());
    for file in &options.rules_overlays {
        match std::fs::read(file) {
            Ok(code) => rules_overlays.push(s2_rules::Overlay {
                name: file.display().to_string(),
                code,
            }),
            Err(e) => {
                eprintln!("{}: {e}", file.display());
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    let game_dir = game_dir.map(|dir| {
        asset_folders
            .iter()
            .fold(dir, |dir, folder| dir.with_overlay(folder))
    });
    let clear_color = render.clear_color;
    crate::extend::install(asset_folders, render, music_external);
    let mut maps = game_dir
        .as_ref()
        .map(|dir| find_maps(&dir.root().join("data/maps")))
        .unwrap_or_default();

    let current = if let Some(file) = options.map {
        // Show the given file; keep it in the list so N / P still cycle.
        match maps.iter().position(|path| *path == file) {
            Some(index) => index,
            None => {
                maps.insert(0, file);
                0
            }
        }
    } else if let Some(name) = options.map_name {
        let wanted = name.trim_end_matches(".s2m");
        match maps
            .iter()
            .position(|path| map_name(path).eq_ignore_ascii_case(wanted))
        {
            Some(index) => index,
            None => {
                eprintln!("no map named {name} under data/maps of the install");
                return std::process::ExitCode::FAILURE;
            }
        }
    } else {
        0
    };
    if maps.is_empty() {
        eprintln!("no map to show: give --map <file>, or --game-dir / S2_GAME_DIR\n{USAGE}");
        return std::process::ExitCode::FAILURE;
    }

    let mut model_view = crate::tools::model::ModelView::new(
        game_dir.as_ref().map(|d| d.root()),
        options.model.as_deref(),
    );
    if let Some(dir) = options.model_shots.clone() {
        if let Some(list) = &options.model_list {
            match crate::tools::model::read_model_list(list, game_dir.as_ref().map(|d| d.root())) {
                Ok(files) => model_view.files = files,
                Err(e) => {
                    eprintln!("{}: {e}", list.display());
                    return std::process::ExitCode::FAILURE;
                }
            }
            model_view.current = 0;
        }
        model_view.shots = Some(crate::tools::model::ModelShots::new(
            dir,
            options.zoom.unwrap_or(crate::render::world3d::START_ZOOM),
        ));
    }
    let start_mode = if options.model.is_some() || options.model_shots.is_some() {
        ViewMode::Model
    } else {
        ViewMode::Map
    };
    let rules = load_rules(game_dir.as_ref(), &rules_overlays);
    if let Some(dir) = &options.save_dir {
        crate::menus::menu::set_save_folder(dir.clone());
    }
    if let Some(dir) = project_saves {
        crate::menus::menu::set_project_save_folder(dir);
    }
    // W11 b: the mark of this program's saves, which a save must have to load.
    let save_mark = s2_ai::save::SaveMark::of(project_name.as_deref(), &rules_overlays);
    let viewer = Viewer {
        building_list: building_list(rules.as_ref()),
        ware_list: ware_list(rules.as_ref()),
        good_names: good_names(rules.as_ref()),
        show_stock: false,
        show_production: false,
        show_tools: false,
        tool_slot: 0,
        panel: if options.military_panel {
            Panel::Military
        } else {
            Panel::None
        },
        wh_good: 0,
        dist_table: 0,
        dist_entry: 0,
        rank_sel: 0,
        mil_sel: 0,
        territory: true,
        borders: true,
        attack: None,
        attack_strong: false,
        attack_hover: false,
        show_messages: true,
        ship_choice: 0,
        scenario: options.scenario,
        ai_slots: options.ai.clone(),
        ai: None,
        attack_run: options.attack,
        compact: options.compact,
        start_resources: options.start_resources,
        start_target: 0,
        focus: options.focus,
        warehouse_goods: warehouse_goods(rules.as_ref()),
        building_names: building_names(rules.as_ref()),
        tool_goods: rules
            .as_ref()
            .map(|r| {
                s2_rules::TOOL_SLOTS
                    .iter()
                    .filter_map(|name| r.id_named(name))
                    .collect()
            })
            .unwrap_or_default(),
        real_game: false,
        saved_game: false,
        own_save: false,
        ahead_marks: None,
        ahead_slots: None,
        setup_terrain: None,
        no_fog: options.no_fog,
        start_fog_off: false,
        fog_off: false,
        pause_layer: options.pause_layer,
        program_game: game,
        patterns: HashMap::new(),
        loads: 0,
        tick_fraction: 0.0,
        debug: options.debug,
        refusal: None,
        open: options.open,
        open_tab: options.open_tab,
        open_global: options.open_global.clone(),
        open_building: options.open_building,
        building_type: options.building_type.clone(),
        context_at: options.context_at,
        watch: options.watch.clone(),
        until_work: options.until_work.clone(),
        until_felled: options.until_felled,
        until_door: options.until_door.clone(),
        until_built: options.until_built,
        scene: options.scene.clone(),
        focus_request: None,
        jump_request: None,
        setup: None,
        tutorial_step: options.tutorial_step,
        run: options.run,
        maps: maps.clone(),
        current,
        reload: !options.menu,
        map: None,
        error: None,
        terrain_colors: rules
            .as_ref()
            .map(crate::render::palette::terrain_colors)
            .unwrap_or_default(),
        deposit_kinds: rules
            .as_ref()
            .map(crate::render::palette::deposit_kinds)
            .unwrap_or_default(),
        rules,
        game_root: game_dir.as_ref().map(|d| d.root().to_path_buf()),
        save_mark,
        screenshot_dir: crate::tools::shots::shot_folder(project_shots, &options.screenshot_dir),
        shot: options.shot,
        shot_after: options.shot_after,
        frames_since_load: 0,
        shot_taken_at: None,
        world: None,
        overlay: options.overlay,
        page: 0,
        selected: None,
        road_start: None,
        save_file: options.save.clone(),
        map_file: None,
        autosave: crate::game::save::AutosaveClock::default(),
        autosave_interval: options.autosave,
        status: String::new(),
        world_changed: false,
        moved: false,
        paused: options.paused,
        pause_key: false,
        hide_interface: options.compare,
        compare: options.compare,
        log_builders: options.log_builders,
        speed: options.speed,
        ware_choice: 0,
        drop_from: None,
    };

    let mut library = crate::render::models3d::ModelLibrary::new(
        game_dir.as_ref().map(|d| d.root()),
        model_view.textures.clone(),
    );
    library.texture_copies = model_view.texture_copies.clone();
    library.index_clip_sounds();
    // The harness never writes the player's settings or profile file: with
    // a folder of its own (`--profile-dir`) it writes there, else nothing.
    if options.play.is_some() && options.profile_dir.is_none() {
        crate::menus::settings::hold_files();
    }
    let play = options
        .play
        .as_deref()
        .and_then(crate::tools::play::script)
        .map(|steps| {
            // The display is kept on for the run and woken first, before
            // the window opens (tools/play/display.rs).
            let display = crate::tools::play::display::hold();
            crate::tools::play::Play::new(
                steps,
                options.play_report.clone(),
                options.screenshot_dir.clone(),
            )
            .with_note(display.note())
        });
    let mut app = App::new();
    if let Some(play) = play {
        app.insert_resource(play).add_systems(
            First,
            crate::tools::play::drive.run_if(in_state(ViewMode::Map)),
        );
    }
    if let Some(report) = options.colour_probe.clone() {
        app.insert_resource(crate::tools::colour_probe::ColourProbe::new(report))
            .add_systems(
                Update,
                crate::tools::colour_probe::drive.run_if(in_state(ViewMode::Map)),
            );
    }
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: if game {
                        crate::extend::window_title()
                    } else {
                        "s2-viewer"
                    }
                    .into(),
                    resolution: options.size.unwrap_or_else(|| window_size(game)).into(),
                    // `--bench`: frames as fast as they come, not held to
                    // the screen's refresh, so the times are the work's
                    // (`--bench-vsync`: held, as a player sees the game).
                    present_mode: if options.bench.is_some() && !options.bench_vsync {
                        bevy::window::PresentMode::AutoNoVsync
                    } else {
                        bevy::window::PresentMode::default()
                    },
                    ..default()
                }),
                // `--still`: the pointer passes through the window, so
                // no hover marker or highlight depends on where the
                // desktop's pointer happens to be.
                primary_cursor_options: Some(bevy::window::CursorOptions {
                    hit_test: !options.still,
                    ..default()
                }),
                ..default()
            })
            // `--compare`: every pipeline ready in the frame that first
            // needs it, so the shot one frame after the scene is built
            // shows all of it.
            .set(bevy::render::RenderPlugin {
                synchronous_pipeline_compilation: options.compare,
                ..default()
            }),
    );
    // SND-8.3: the positioned sounds' own source, with their pan.
    crate::audio::output::register(&mut app);
    app.insert_resource(ClearColor(Color::srgb(
        clear_color[0],
        clear_color[1],
        clear_color[2],
    )))
    // `--still`, `--compare`: the game is paused, so what follows game time
    // (tree sway and flag cloth, GFX-OBJ-2.4, 2.9; terrain wobble; water,
    // GFX-WATER-4.2: TRACE-5.2) stands, and the frame clock stands at 0,
    // so what follows it (messages, tooltips) is the same in every shot.
    // The shot waits by frames, not by time.
    .insert_resource(if options.still {
        bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO)
    } else {
        bevy::time::TimeUpdateStrategy::Automatic
    })
    .insert_resource(viewer)
    .insert_resource(model_view)
    .insert_resource({
        let mut view = crate::render::view3d::View3d::new(options.three_d, options.zoom);
        view.fixed_eye = fixed_eye;
        view
    })
    .insert_resource(library)
    .init_resource::<Hover>()
    .init_resource::<GameClock>()
    .init_resource::<crate::render::view3d::MapHover>()
    .init_resource::<crate::render::view3d::ShipHover>()
    .init_resource::<crate::extend::CameraFollow>()
    .add_plugins(crate::render::stored::StoredColoursPlugin)
    .add_plugins(crate::render::view3d::View3dPlugin)
    .add_plugins(crate::render::scene_clock::SceneClockPlugin)
    .add_plugins(crate::render::ship_view::ShipViewPlugin)
    .add_plugins(crate::ui::UiPlugin)
    .add_message::<crate::extend::MusicSlot>()
    .add_message::<crate::ui::game::StatsMenuOpened>()
    .init_resource::<crate::audio::SoundQueue>()
    .init_resource::<crate::audio::AudioState>()
    .insert_resource({
        let mut menu = crate::menus::menu::MenuState::start(
            options.menu,
            options.menu_screen.as_deref(),
            options.free_game.as_deref(),
            options.mission.as_deref(),
            &maps,
            options.start_resources,
        );
        // UI-MENU-0.1: the game opens with the intro (the logo films
        // are left out); `--menu` goes straight to the main menu.
        if options.intro
            && menu.screen == crate::menus::menu::Screen::Main
            && menu.pending.is_empty()
        {
            menu.screen = crate::menus::menu::opening_screen(skip_launch_films);
        }
        menu.entries = entries.clone();
        menu.captions = captions;
        // `--mute`: nobody hears this run; the sounds run as ever.
        if options.mute {
            menu.settings.volumes = [0.0; 4];
        }
        menu
    })
    .init_resource::<crate::films::video::Films>()
    .init_resource::<crate::menus::campaign::MissionUi>()
    .insert_non_send(crate::menus::campaign::MissionScriptRes::default())
    .insert_state(start_mode)
    .insert_resource(crate::render::view3d::DayClock {
        start: options.hour.unwrap_or(crate::render::world3d::START_HOUR),
        ..Default::default()
    })
    .add_systems(
        Startup,
        (
            setup,
            crate::tools::model::setup_model_view,
            crate::render::view3d::setup_3d,
            crate::render::view3d::setup_mirror,
            crate::render::ship_view::setup,
        ),
    )
    .add_systems(
        Update,
        (
            cycle_maps,
            load_map,
            crate::render::view3d::build_scene,
            camera_controls,
            crate::render::view3d::camera_input,
            (
                crate::render::view3d::advance_day,
                crate::render::view3d::place_camera,
                crate::render::view3d::place_mirror,
                crate::render::ship_view::aim,
            )
                .chain(),
            hover_2d,
            crate::render::view3d::hover_3d,
            (
                crate::render::view3d::pick_map,
                crate::render::view3d::pick_ship,
            )
                .chain(),
            (
                crate::ui::load_assets,
                crate::render::ship_view::make_material,
                crate::films::video::play_films,
                crate::ui::update_minimap,
                crate::ui::update_preview,
                crate::ui::render::interact,
                crate::ui::game_keys,
                crate::ui::apply_actions,
                crate::ui::message_sounds,
                crate::ui::menu_hover,
                crate::ui::map_click,
                crate::ui::type_text,
                // UI-GAME-1.10c: a dialog opened in this frame is built, the
                // pointer moved onto its first button and its widgets made
                // in this frame, so it takes clicks from the next one.
                crate::ui::statistics_pages,
                crate::ui::game_frame,
                crate::ui::smart_cursor,
                crate::ui::render::sync_panels,
                crate::audio::world_events,
                crate::audio::play,
            )
                .chain(),
            edit_world,
            (advance_world, crate::game::save::autosave).chain(),
            crate::render::view3d::update_scene,
            crate::render::view3d::sync_objects,
            (
                crate::render::view3d::sync_movers,
                // GFX-VIEW-5.4: which settlers this frame draws.
                crate::render::view3d::seen_marks,
                crate::render::view3d::animate_buildings,
                crate::render::view3d::sync_wares,
                crate::animation::animate3d::animate,
                // F51: the frame's palette writes in one buffer.
                crate::render::models3d::flush_palettes,
            )
                .chain(),
            draw_world,
            draw_moving,
            draw_deposits,
            (
                crate::render::view3d::draw_marks,
                crate::render::view3d::status_icons,
                crate::render::view3d::collapse_dust,
                crate::render::view3d::cursor_sprites,
                crate::menus::campaign::pointer_sprite,
            ),
            hover_info,
        )
            .chain()
            .run_if(in_state(ViewMode::Map)),
    )
    .add_systems(
        Update,
        (
            crate::tools::model::cycle_models,
            crate::tools::model::load_model,
            crate::tools::model::orbit_camera,
            crate::tools::model::game_camera,
            crate::tools::model::model_info,
            crate::tools::model::count_frames,
            crate::tools::model::model_shots,
        )
            .chain()
            .run_if(in_state(ViewMode::Model)),
    )
    .add_systems(
        Update,
        (
            crate::tools::model::toggle_view,
            toggle_3d,
            crate::tools::model::switch_cameras,
            speed_hud,
            screenshot_key,
            automatic_shot,
            quit_on_escape,
        )
            .chain(),
    );
    // `--bench`: the frame-time benchmark's marks between the parts of the
    // map's update (F51).
    if let Some(frames) = options.bench {
        use crate::tools::bench::{bench_frame, bench_pan, bench_post, mark, Bench};
        // `--bench-invariants`: the soak's checks in the real program.
        if options.bench_invariants {
            app.init_resource::<crate::tools::bench::RunCheck>();
        }
        let render_times = crate::tools::bench::RenderTimes::default();
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            use crate::tools::bench::render_mark;
            use bevy::render::{Render, RenderSystems as S};
            render_app
                .insert_resource(render_times.clone())
                .add_systems(
                    Render,
                    (
                        render_mark::<0>.before(S::ExtractCommands),
                        render_mark::<1>
                            .after(S::PrepareMeshes)
                            .before(S::CreateViews),
                        render_mark::<2>.after(S::PrepareViews).before(S::Queue),
                        render_mark::<3>.after(S::QueueSweep).before(S::PhaseSort),
                        render_mark::<4>.after(S::PhaseSort).before(S::Prepare),
                        render_mark::<5>
                            .after(S::PrepareBindGroups)
                            .before(S::Render),
                        render_mark::<6>.after(S::Render).before(S::Cleanup),
                    ),
                );
        }
        app.insert_resource(render_times)
            .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
            .init_resource::<crate::audio::SoundStats>()
            .insert_resource(Bench::new(
                frames,
                options.bench_report.clone(),
                options.bench_pan,
                options.bench_drag,
                options.bench_seconds,
                options.bench_log.clone(),
            ))
            .init_resource::<crate::tools::bench::FrameSpans>()
            .add_systems(First, (bench_frame, bench_pan).chain())
            .add_systems(Last, bench_post)
            .add_systems(
                Update,
                (
                    mark::<0>.before(cycle_maps),
                    mark::<1>
                        .after(crate::render::view3d::pick_map)
                        .before(crate::ui::load_assets),
                    mark::<2>.after(crate::audio::play).before(edit_world),
                    mark::<3>
                        .after(advance_world)
                        .before(crate::render::view3d::update_scene),
                    mark::<4>
                        .after(crate::render::view3d::sync_objects)
                        .before(crate::render::view3d::sync_movers),
                    mark::<5>
                        .after(crate::animation::animate3d::animate)
                        .before(draw_world),
                    mark::<6>.after(hover_info),
                ),
            );
    }
    // The settings a project reads (`docs/extending.md`), kept as the
    // menu's.
    let settings = app
        .world()
        .resource::<crate::menus::menu::MenuState>()
        .settings
        .clone();
    app.insert_resource(crate::extend::CurrentSettings::new(settings))
        .add_systems(
            Update,
            crate::extend::sync_settings.after(crate::ui::apply_actions),
        );
    // A project's menu pages (`docs/extending.md`): their callbacks and
    // rows after the frame's menu actions.
    if !entries.is_empty() {
        app.insert_resource(pages).add_systems(
            Update,
            crate::extend::menu::run_page
                .after(crate::ui::apply_actions)
                .run_if(in_state(ViewMode::Map)),
        );
    }
    // The things on the map for a project, while it asks for them.
    crate::extend::things::add(&mut app, map_things);
    // A project's own systems and plugins (`docs/extending.md`).
    for plugin in plugins {
        plugin(&mut app);
    }
    let exit = app.run();
    // The play harness ends with an error exit when a step failed.
    match exit {
        AppExit::Success => std::process::ExitCode::SUCCESS,
        AppExit::Error(code) => std::process::ExitCode::from(code.get()),
    }
}

pub(crate) fn setup(mut commands: Commands) {
    // GFX-VIEW-2.6 c: the menus and the 2D map composite on stored values,
    // as the map camera does (`render::stored`).
    commands.spawn((Camera2d, bevy::camera::CompositingSpace::Srgb));
    commands.spawn((
        SpeedText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(34.0),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            right: px(8),
            padding: UiRect::all(px(8)),
            ..default()
        },
    ));
    commands.spawn((
        InfoText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            left: px(8),
            padding: UiRect::all(px(6)),
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game folder hook (docs/extending.md): none before `run_with` has
    /// found one, then the folder it runs with (the first kept wins).
    // spec: none (the game folder, read only)
    #[test]
    fn the_game_folder_is_the_one_the_program_found() {
        assert_eq!(crate::paths::game_dir(), None);
        keep_game_dir(None);
        assert_eq!(game_dir(), None);
        keep_game_dir(Some(Path::new("C:/Games/S2")));
        assert_eq!(crate::paths::game_dir(), Some(PathBuf::from("C:/Games/S2")));
        keep_game_dir(Some(Path::new("elsewhere")));
        assert_eq!(game_dir(), Some(PathBuf::from("C:/Games/S2")));
    }
}
