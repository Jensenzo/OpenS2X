//! Loading a map or a save into the viewer: files, rules, the 2D meshes
//! and textures, the world and its computer players.

use crate::game::{MapEntity, Viewer, ATTACK_LIMIT, PLAYER};
use crate::render::terrain::MeshData;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use s2_assets::s2m::{self, Map};
use s2_assets::terrain_patterns::TerrainPatterns;
use s2_assets::{rc00, GameDir};
use s2_rules::RuleSet;
use s2_sim::{PlayerId, World};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Every `.s2m` below `dir`, sorted by file name.
pub(crate) fn find_maps(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                pending.push(path);
            } else if has_extension(&path, "s2m") {
                found.push(path);
            }
        }
    }
    sort_maps(&mut found);
    found
}

/// UI-MENU-2.1 "The order of the maps" (Q-211 f, E-ui-030, T20): by the file
/// name cut at the first `.`, `a`..`z` turned into `A`..`Z`, compared byte by
/// byte, shorter first when one starts the other (so `_` sorts after every
/// letter).
pub(crate) fn sort_maps(maps: &mut [PathBuf]) {
    maps.sort_by_key(|path| {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cut = name.split('.').next().unwrap_or_default().to_owned();
        cut.to_ascii_uppercase().into_bytes()
    });
}

pub(crate) fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension))
}

pub(crate) fn map_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Read and decode a game file. The container key depends on the file's name
/// (FMT-RC-2), which `rc00::decode` takes from the path.
pub(crate) fn read_game_file(path: &Path) -> Result<Vec<u8>, String> {
    let name = path.to_str().ok_or("path is not valid UTF-8")?;
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if rc00::is_wrapped(&data) {
        rc00::decode(name, &data).map_err(|e| e.to_string())
    } else {
        Ok(data)
    }
}

/// The rule set of the install, if there is one and its scripts run.
/// The property scripts of the install, then a project's rules overlays
/// (`docs/extending.md`; none in the game).
pub(crate) fn load_rules(
    game_dir: Option<&GameDir>,
    overlays: &[s2_rules::Overlay],
) -> Option<RuleSet> {
    match s2_rules::load_with_overlays(game_dir?, overlays) {
        Ok((rules, _)) => Some(rules),
        Err(e) => {
            eprintln!("property scripts not loaded, using generated colours: {e}");
            None
        }
    }
}

/// N and P step through the map list of the map viewer; in a game and on
/// the menus they are the game's keys (UI-GAME-12.1: messages, pause).
pub(crate) fn cycle_maps(
    keys: Res<ButtonInput<KeyCode>>,
    mut viewer: ResMut<Viewer>,
    menu: Res<crate::menus::menu::MenuState>,
) {
    if viewer.real_game || menu.screen != crate::menus::menu::Screen::None {
        return;
    }
    let count = viewer.maps.len();
    if keys.just_pressed(KeyCode::KeyN) {
        viewer.current = (viewer.current + 1) % count;
        viewer.reload = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        viewer.current = (viewer.current + count - 1) % count;
        viewer.reload = true;
    }
}

pub(crate) fn bevy_mesh(data: MeshData) -> Mesh {
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, data.colors)
    .with_inserted_indices(Indices::U32(data.indices));
    if data.uvs.is_empty() {
        mesh
    } else {
        mesh.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs)
    }
}

/// The terrain textures of the types on `map`, by type id: the graphics name
/// (`pa_setGfxName`, LUA-PROP-7) is looked up in
/// `data/settings/terrain_static_data.xml`, which gives the texture file (or
/// the pattern to borrow it from) and the pattern's own texture scale
/// (GFX-PAT). Types whose texture cannot be read are left out and drawn
/// with flat colours.
pub(crate) fn load_terrain_textures(
    game_dir: &Path,
    rules: &RuleSet,
    map: &Map,
) -> HashMap<u32, crate::render::terrain::PatternTexture> {
    let mut textures = HashMap::new();
    let list_path = match crate::extend::find(game_dir, "data/settings/terrain_static_data.xml") {
        Ok(path) => path,
        Err(e) => {
            eprintln!("terrain pattern list: {e}");
            return textures;
        }
    };
    let list = match read_game_file(&list_path) {
        Ok(data) => TerrainPatterns::parse(&String::from_utf8_lossy(&data)),
        Err(e) => {
            eprintln!("terrain pattern list {}: {e}", list_path.display());
            return textures;
        }
    };
    let used: std::collections::BTreeSet<u32> = map.patterns.iter().copied().collect();
    // Patterns that borrow a texture share the file: decode each once.
    let mut files: HashMap<String, Option<s2_assets::dds::Texture>> = HashMap::new();
    let mut texture_file = |file: &str| {
        files
            .entry(file.to_owned())
            .or_insert_with(|| {
                let relative = file.replace('\\', "/");
                let path = match crate::extend::find(game_dir, &relative) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("terrain texture {relative}: {e}");
                        return None;
                    }
                };
                std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|data| {
                        s2_assets::dds::load(&path.to_string_lossy(), &data)
                            .map_err(|e| e.to_string())
                    })
                    .map_err(|e| eprintln!("terrain texture {}: {e}", path.display()))
                    .ok()
            })
            .clone()
    };
    for pattern in rules.patterns.iter().filter(|p| used.contains(&p.id)) {
        let Some(name) = pattern.gfx_name.as_deref() else {
            continue;
        };
        let Some(found) = list.resolve(name) else {
            eprintln!("terrain pattern {name}: no texture in terrain_static_data.xml");
            continue;
        };
        if let Some(texture) = texture_file(&found.texture) {
            // GFX-VIEW-2.4b: the normal map with the mip levels its file
            // stores.
            let (normal, normal_levels) = found
                .normal
                .as_deref()
                .and_then(|file| normal_file(game_dir, file))
                .map_or((None, Vec::new()), |(t, levels)| (Some(t), levels));
            textures.insert(
                pattern.id,
                crate::render::terrain::PatternTexture {
                    texture,
                    scale: found.tex_scale,
                    normal,
                    normal_levels,
                    normal_scale: found.nrm_scale,
                    kind: found.kind,
                    priority: found.priority as u32,
                    transition: found.transition,
                    specular: found.specular,
                },
            );
        }
    }
    textures
}

/// GFX-VIEW-2.4b: a pattern's normal map with the lower mip levels its file
/// stores; none (with a message) when it cannot be read.
fn normal_file(
    game_dir: &Path,
    file: &str,
) -> Option<(s2_assets::dds::Texture, Vec<s2_assets::dds::Level>)> {
    let relative = file.replace('\\', "/");
    let path = crate::extend::find(game_dir, &relative)
        .map_err(|e| eprintln!("terrain normal map {relative}: {e}"))
        .ok()?;
    std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|data| {
            s2_assets::dds::load_with_levels(&path.to_string_lossy(), &data)
                .map_err(|e| e.to_string())
        })
        .map_err(|e| eprintln!("terrain normal map {}: {e}", path.display()))
        .ok()
}

/// The asset collections a map's meshes, materials and textures go into.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MapAssets<'w> {
    pub(crate) meshes: ResMut<'w, Assets<Mesh>>,
    pub(crate) materials: ResMut<'w, Assets<ColorMaterial>>,
    pub(crate) images: ResMut<'w, Assets<Image>>,
}

/// Replace the scene with the current map and fit the camera to it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn load_map(
    mut commands: Commands,
    mut viewer: ResMut<Viewer>,
    mut assets: MapAssets,
    old: Query<Entity, With<MapEntity>>,
    mut window: Single<&mut Window>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
    mut mission: ResMut<crate::menus::campaign::MissionUi>,
    mut script: NonSendMut<crate::menus::campaign::MissionScriptRes>,
    mut menu: ResMut<crate::menus::menu::MenuState>,
) {
    viewer.frames_since_load += 1;
    if !viewer.reload {
        return;
    }
    viewer.reload = false;
    viewer.frames_since_load = 0;
    // A new load replaces an earlier load's notice.
    menu.notice = None;
    // A save of ours (`save`) is read whole before the game on screen
    // goes; one that does not load (another version, a damaged file, its
    // map missing) leaves a notice, never a crash, over the game menu of
    // the game that stays, or the main menu without one.
    let own = match viewer
        .save_file
        .clone()
        .filter(|p| has_extension(p, crate::game::save::EXTENSION))
    {
        Some(file) => match read_own_save(&viewer, &file) {
            Ok(loaded) => Some(loaded),
            Err(e) => {
                viewer.save_file = None;
                warn!("the save {} does not load: {e}", file.display());
                menu.notice = Some(crate::menus::menu::Notice {
                    title: "SAVE NOT LOADED".to_owned(),
                    lines: vec![map_name(&file), e],
                });
                let game = viewer.world.is_some() && viewer.real_game;
                menu.in_game = game;
                menu.screen = if game {
                    crate::menus::menu::Screen::GameMenu
                } else {
                    crate::menus::menu::Screen::Main
                };
                viewer.paused = true;
                return;
            }
        },
        None => None,
    };
    for entity in &old {
        commands.entity(entity).despawn();
    }

    // FMT-SAV: a save continues from its map part and game block.
    let save_file = viewer.save_file.take();
    let (own, own_map) = match own {
        Some((loaded, map)) => (Some(loaded), Some(map)),
        None => (None, None),
    };
    let path = own_map
        .or_else(|| save_file.clone())
        .unwrap_or_else(|| viewer.maps[viewer.current].clone());
    let position = format!("{} of {}", viewer.current + 1, viewer.maps.len());
    let data = read_game_file(&path);
    let save = (save_file.is_some() && own.is_none())
        .then(|| data.as_ref().ok().map(|d| s2_assets::sav::parse(d)))
        .flatten();
    let loaded = data.and_then(|data| s2m::parse(&data).map_err(|e| e.to_string()));
    let map = match loaded {
        Ok(map) => map,
        Err(e) => {
            if !viewer.program_game {
                window.title = format!("s2-viewer: {} ({position}): {e}", map_name(&path));
            }
            viewer.error = Some(format!("{}: {e}", path.display()));
            viewer.map = None;
            if save_file.is_some() {
                // A save without a readable map part does not load: back
                // to the main menu with a notice.
                warn!("the save {} does not load: {e}", path.display());
                let name = map_name(&path);
                menu.notice = save
                    .as_ref()
                    .and_then(|s| crate::menus::menu::Notice::for_save(&name, &s.problems, false))
                    .or(Some(crate::menus::menu::Notice {
                        title: "SAVE NOT LOADED".to_owned(),
                        lines: vec![name, e],
                    }));
                menu.in_game = false;
                menu.screen = crate::menus::menu::Screen::Main;
                viewer.paused = true;
            }
            return;
        }
    };
    window.title = if viewer.program_game {
        crate::extend::window_title().to_string()
    } else {
        format!(
            "s2-viewer: {} ({position}), {} x {}",
            map.info.name, map.info.width, map.info.height
        )
    };

    let material = assets.materials.add(ColorMaterial::default());
    // The terrain from the textures of its types when the install has them;
    // otherwise flat colours by type.
    let textures = match (&viewer.game_root, &viewer.rules) {
        (Some(root), Some(rules)) => load_terrain_textures(root, rules, &map),
        _ => HashMap::new(),
    };
    viewer.loads += 1;
    let (terrain_data, terrain_material) = if textures.is_empty() {
        (
            crate::render::terrain::terrain_mesh(&map, &viewer.terrain_colors),
            material.clone(),
        )
    } else {
        let baked = crate::render::terrain::bake_terrain(&map, &textures, &viewer.terrain_colors);
        let image = Image::new(
            Extent3d {
                width: baked.width,
                height: baked.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            baked.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        (
            crate::render::terrain::textured_terrain_mesh(&map),
            assets.materials.add(ColorMaterial {
                texture: Some(assets.images.add(image)),
                ..default()
            }),
        )
    };
    for (data, layer, material) in [
        (
            terrain_data,
            crate::render::terrain::TERRAIN_LAYER,
            terrain_material,
        ),
        (
            // A game draws its trees and stones from the simulation.
            crate::render::terrain::marker_mesh(
                &map,
                &viewer.deposit_kinds,
                viewer.rules.is_none(),
                viewer.rules.is_none(),
            ),
            crate::render::terrain::MARKER_LAYER,
            material.clone(),
        ),
        (
            crate::render::terrain::start_mesh(&map),
            crate::render::terrain::START_LAYER,
            material,
        ),
    ] {
        commands.spawn((
            MapEntity,
            Mesh2d(assets.meshes.add(bevy_mesh(data))),
            MeshMaterial2d(material),
            Transform::from_xyz(0.0, 0.0, layer),
        ));
    }

    // Centre the map and zoom so all of it is visible.
    let [width, height] = crate::render::terrain::map_extent(map.info.width, map.info.height);
    let (mut transform, mut projection) = camera.into_inner();
    transform.translation.x = width / 2.0;
    transform.translation.y = height / 2.0;
    if let Projection::Orthographic(orthographic) = &mut *projection {
        let fit = (width / window.width()).max(height / window.height());
        orthographic.scale = (fit * 1.04).max(0.01);
        if let Some((_, _, scale)) = viewer.focus {
            orthographic.scale = scale;
        }
    }
    if let Some((x, y, _)) = viewer.focus {
        let [fx, fy] = crate::render::terrain::field_position(x, y);
        transform.translation.x = fx;
        transform.translation.y = fy;
    }

    // ECO-HQ: a new game with every start position's headquarters. Without
    // usable rules there is no headquarters type, so the player owns every
    // field instead.
    // UI-SETUP-2: a free game started from the menu has the players and
    // tribes of its slots and computer players by their strength.
    let setup_players = viewer.setup.take();
    // FMT-S2M-2.6, CAM-4.3a: the players' colours: a campaign map's records also
    // for a save of a mission (nothing else keeps them), else their own numbers.
    let kind = s2_rules::mission::map_kind(&path);
    crate::render::models3d::set_player_colours(match (&setup_players, kind) {
        (Some(players), _) => s2_rules::mission::colours(players),
        (None, 1) => s2_rules::mission::colours(&s2_rules::mission::players(&map, kind)),
        _ => std::array::from_fn(|i| i as u8),
    });
    if let Some(players) = &setup_players {
        // `--ai` slots given on the command line stay (they can play the
        // human's slot, for demonstrations).
        let mut slots: Vec<(PlayerId, u8)> = viewer
            .ai_slots
            .iter()
            .copied()
            .filter(|(p, _)| {
                players
                    .iter()
                    .any(|s| s.player == *p && s.profile.is_none())
            })
            .collect();
        slots.extend(players.iter().filter_map(|p| Some((p.player, p.profile?))));
        viewer.ai_slots = slots;
    }
    // FMT-SAV-3.10a: an original save's computer players play on, with their
    // records (`AiSystem::take_over`, below); they replace `--ai`.
    if let (Some(save), None) = (&save, &setup_players) {
        viewer.ai_slots = s2_rules::computer_players(save);
    }
    let own_save = own.is_some();
    let own_older = own.as_ref().is_some_and(|l| !l.current_format());
    let (own_world, own_ai, own_header) = match own {
        Some(l) => (Some(l.world), Some(l.ai), Some(l.header)),
        None => (None, None, None),
    };
    let game = match own_world {
        Some(world) => Some(Ok(world)),
        None => viewer.rules.as_ref().map(|rules| match &setup_players {
            Some(players) => rules.sim_rules().map(|sim| {
                // UI-SETUP-2.1: each player on its slot's start position.
                let mut setup =
                    s2_rules::setup_of_players(&map, rules, viewer.start_resources, players);
                // ECO-HQ-3.6, GFX-VIEW-5.3: fog of war Off explores the whole
                // map for every player at the start; the drawing follows
                // the explored fields.
                setup.fog_of_war_off = viewer.start_fog_off;
                World::new_game(s2_sim::rng::SIM_SEED as u64, map.terrain(), sim, &setup)
            }),
            None => match &save {
                Some(save) => s2_rules::restore_game(save, rules).map(|(world, report)| {
                    // A save that read in part continues from what was read,
                    // paused behind a notice that names the first part left out.
                    for problem in &save.problems {
                        warn!("the save does not read in full: {problem}");
                    }
                    menu.notice = crate::menus::menu::Notice::for_save(
                        &map_name(&path),
                        &save.problems,
                        true,
                    );
                    info!("save continued: {report:?}");
                    world
                }),
                None => s2_rules::new_game(
                    s2_sim::rng::SIM_SEED as u64,
                    &map,
                    rules,
                    viewer.start_resources,
                ),
            },
        }),
    };
    if menu.notice.is_some() {
        viewer.paused = true;
    }
    let from_save = own_save || save.is_some();
    let (mut world, real_game) = match game {
        Some(Ok(mut world)) => {
            // `--expand` sets up a new game: a loaded one (CONTINUE, a save
            // of ours or the original's) goes on as it was saved.
            let scenario = scripted_expansion(viewer.scenario, from_save);
            if let (Some(ticks), Some(rules)) = (scenario, viewer.rules.as_ref()) {
                let report =
                    s2_rules::scenario::expand_towards_each_other(&mut world, rules, ticks);
                info!(
                    "scripted expansion: {} military buildings in {ticks} ticks",
                    report.placed.len()
                );
                if let Some(stop) = viewer.attack_run {
                    let until = world.tick() + ATTACK_LIMIT;
                    let attack =
                        s2_rules::scenario::attack_until(&mut world, PlayerId(1), until, stop);
                    info!(
                        "scripted attack: {} orders, first fight {:?}, conquered {:?}",
                        attack.orders.len(),
                        attack.first_fight,
                        attack.conquered
                    );
                }
            }
            (world, true)
        }
        Some(Err(e)) => {
            eprintln!("rules not usable, using shipped settings: {e}");
            let rules = s2_sim::Rules::empty();
            (World::with_map(1, map.terrain(), rules, PLAYER), false)
        }
        None => {
            let rules = s2_sim::Rules::empty();
            (World::with_map(1, map.terrain(), rules, PLAYER), false)
        }
    };
    // --ai: the computer players of the game; a save of ours brings its
    // own.
    viewer.ai = None;
    if let (Some(ai), Some(header)) = (own_ai, own_header.as_ref()) {
        viewer.ai_slots = header.slots.clone();
        viewer.ai = ai;
    } else if real_game && !viewer.ai_slots.is_empty() {
        if let Some(rules) = viewer.rules.as_ref() {
            match rules.sim_rules() {
                Ok(sim_rules) => {
                    let config = s2_ai::AiConfig::new(rules, &sim_rules);
                    let mut ai = s2_ai::AiSystem::new(config, &world);
                    ai.activate_all(&mut world, &viewer.ai_slots);
                    // FMT-SAV-3.10: an original save's computer players play
                    // on with their own records.
                    if let Some(save) = &save {
                        let taken = ai.take_over(&world, save);
                        info!("computer players' records taken over: {taken:?}");
                    }
                    viewer.ai = Some(ai);
                }
                Err(e) => eprintln!("no computer players: {e}"),
            }
        }
    }
    // The game loaded from a save of ours is the game saved: the state
    // hashes it was saved with (the play harness's save-quit-load check).
    if let (Some(header), Some(file)) = (own_header.as_ref(), save_file.as_ref()) {
        let check = crate::game::save::LoadCheck {
            path: file.clone(),
            tick: world.tick(),
            saved: (header.world_hash, header.ai_hash),
            loaded: (
                world.state_hash(),
                viewer.ai.as_ref().map(s2_ai::AiSystem::state_hash),
            ),
            older_format: own_older,
        };
        if check.older_format {
            info!(
                "save of an earlier format loaded: {}, tick {}",
                file.display(),
                check.tick
            );
        } else if check.same() {
            info!(
                "save loaded: {}, tick {}, state hash {:016x}",
                file.display(),
                check.tick,
                check.loaded.0
            );
        } else {
            warn!("the save loaded is not the game saved: {check:?}");
        }
        menu.last_load = Some(check);
    }
    // FMT-SAV-5.2: the autosave clock starts again with every game.
    viewer.autosave.reset();
    // CAM-1.1, 2, 3: the map's mission, and the script of a campaign or
    // tutorial map with its first two callbacks (CAM-1.2).
    let kind = s2_rules::mission::map_kind(&path);
    *mission = crate::menus::campaign::MissionUi::new(&map_name(&path), kind, 0);
    // CAM-1.3c (Q-1055): the book's title is the text keyed by the map's
    // name (`book_title`).
    mission.title = map.info.name.clone();
    if let (Some(t), Some(step)) = (mission.tutorial.as_mut(), viewer.tutorial_step) {
        t.step = step;
    }
    // `--end-screen` (screenshots): the end as the mission's own events
    // would bring it (CAM-3.6).
    if let Some(end) = viewer.end_screen {
        let player = crate::game::PLAYER;
        let event = match end {
            crate::menus::campaign::EndScreen::Victory(value) => {
                s2_sim::mission::MissionEvent::Completed { value }
            }
            crate::menus::campaign::EndScreen::Defeat => s2_sim::mission::MissionEvent::Lost {
                player,
                reason: s2_sim::mission::LossReason::CastleDestroyed,
            },
        };
        mission.results(&[event], player);
    }
    script.0 = None;
    // A save of ours: the mission is the world's; the interface's part
    // comes from the save, and the map's script is loaded again without
    // its start callbacks, then its saved variables are set (CAM-1.1,
    // FMT-SAV-3.12; which variables, Q-100 b: its whole numbers).
    if let Some(saved) = own_header.as_ref().map(|h| &h.mission) {
        mission.target = saved.target;
        mission.debriefing = saved.debriefing.clone();
        mission.extro = saved.extro.clone();
        mission.letters = saved.letters.clone();
        if let (Some(t), Some(step)) = (mission.tutorial.as_mut(), saved.tutorial_step) {
            t.step = step;
        }
        if let Some(rules) = viewer.rules.as_ref() {
            let loaded = s2_rules::mission::script_for(&path, rules).and_then(|s| {
                if let Some(s) = &s {
                    s.set_variables(&saved.script_variables)?;
                }
                Ok(s)
            });
            match loaded {
                Ok(s) => script.0 = s,
                Err(e) => mission.notice = Some(format!("mission script: {e}")),
            }
            mission.script_variables = saved.script_variables.clone();
        }
    }
    // `--compare` of a new game (the look comparison, the golden images):
    // the game as `s2-app` plays it, the recorded games' (TRACE-8), which
    // sets no mission; `s2-app shot` checks that the states are equal.
    let plain = viewer.compare && save.is_none();
    if real_game && !plain && own_header.is_none() {
        let mut m = s2_rules::mission::mission_of(&map);
        // CAM-3.1a: the start settings' target replaces the header's.
        let kind = s2_rules::mission::map_kind(&path);
        if let Some(target) = s2_rules::mission::start_target(kind, viewer.start_target) {
            m.target = target;
        }
        mission.target = m.target;
        world.set_mission(m);
        // CAM-3.1b: a new campaign game's victory regions start explored
        // (an original's save has its exploration).
        if kind == 1 && save.is_none() {
            world.explore_victory_regions();
        }
        if let Some(rules) = viewer.rules.as_ref() {
            let loaded = s2_rules::mission::script_for(&path, rules).and_then(|s| {
                let Some(s) = s else { return Ok(None) };
                let mut events = s.on_loaded()?;
                events.extend(s.on_started()?);
                Ok(Some((s, events)))
            });
            match loaded {
                Ok(Some((s, events))) => {
                    let rest = s2_rules::mission::submit(&mut world, events);
                    mission.apply(rest);
                    script.0 = Some(s);
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("mission script: {e}");
                    mission.notice = Some(format!("mission script: {e}"));
                }
            }
        }
    }
    // --scene: the screenshot set-up, then the camera over it.
    let mut site = None;
    let mut framed = false;
    if let Some(name) = viewer.scene.take() {
        let by_name = |wanted: &str| {
            viewer
                .building_names
                .iter()
                .find(|(_, n)| n.eq_ignore_ascii_case(wanted))
                .map(|(&id, _)| id)
        };
        let field = match name.as_str() {
            // A woodcutter's hut site by the headquarters (GFX-OBJ-2.6).
            "woodcutter-site" => {
                site = by_name("WoodcutterHut")
                    .and_then(|id| crate::tools::scenes::building(&mut world, PLAYER, id));
                site
            }
            "geologist" => crate::tools::scenes::geologist(&mut world, PLAYER),
            // military:<type>:<n>: a finished military building with n
            // soldiers inside; coins-off:<type>:<n> with its coins
            // stopped (GFX-OBJ-8.1, 8.3).
            other if other.starts_with("military:") || other.starts_with("coins-off:") => {
                let parts: Vec<&str> = other.split(':').collect();
                let soldiers = parts.get(2).and_then(|n| n.parse().ok()).unwrap_or(1);
                site = parts.get(1).and_then(|t| by_name(t)).and_then(|id| {
                    crate::tools::scenes::military(
                        &mut world,
                        PLAYER,
                        id,
                        soldiers,
                        parts[0] == "coins-off",
                    )
                });
                if let Some(at) = site {
                    let inside = world.garrison(at).iter().filter(|s| s.present).count();
                    let limit = world
                        .building_at(at)
                        .and_then(|b| world.building_type(b.building_type))
                        .map_or(0, |k| k.soldier_limit);
                    info!(
                        "{other}: {inside} of {limit} soldiers inside in tick {}",
                        world.tick()
                    );
                }
                site
            }
            // pair:<type>:<type>: two finished buildings side by side
            // (hook W30: a project's model beside an original house).
            other if other.starts_with("pair:") => {
                let parts: Vec<&str> = other.split(':').collect();
                let first = parts.get(1).and_then(|t| by_name(t));
                let second = parts.get(2).and_then(|t| by_name(t));
                framed = true;
                first
                    .zip(second)
                    .and_then(|(a, b)| crate::tools::scenes::pair(&mut world, PLAYER, a, b))
            }
            "waterway" => crate::tools::scenes::waterway(&mut world, PLAYER),
            // ship-shore: the same ship without its window, framed as a
            // site is (a picture of the coast).
            "ship-shore" => {
                framed = true;
                crate::tools::scenes::ship(&mut world, PLAYER)
            }
            // A ship on open water, its window open (UI-GAME-4.5).
            "ship" => {
                let at = crate::tools::scenes::ship(&mut world, PLAYER);
                viewer.open = at;
                at
            }
            // building:<type>: a site of that type by the headquarters,
            // built by `--until-built 100` (F52: a farm before and after its
            // farmer comes, `--until-door Farm`).
            other if other.starts_with("building:") => {
                site = by_name(&other["building:".len()..])
                    .and_then(|id| crate::tools::scenes::building(&mut world, PLAYER, id));
                site
            }
            // A barracks by the headquarters (its window, UI-GAME-4.3).
            "barracks" => viewer
                .building_names
                .iter()
                .find(|(_, n)| n.eq_ignore_ascii_case("Barracks"))
                .map(|(&id, _)| id)
                .and_then(|id| crate::tools::scenes::building(&mut world, PLAYER, id)),
            _ => None,
        };
        match field {
            // A site is framed with the 3D camera's target 3 rows nearer
            // the camera, so it shows mid-screen above the bar.
            Some(field) if site.is_some() || framed => {
                viewer.focus = Some((field.0, field.1.saturating_sub(3), 1.0))
            }
            Some(field) => viewer.focus = Some((field.0, field.1, 1.0)),
            None => eprintln!("--scene {name}: no place for it on this map"),
        }
    }
    // GFX-VIEW-5.4 (g): a game the tool plays ahead keeps the settlers'
    // seen marks with the camera on the game's start view, one frame before
    // the first tick and one after each; the view's first frame is then the
    // jump to the scene.
    let plays_ahead = (viewer.until_built.is_some() && site.is_some()) || viewer.run > 0;
    // GFX-ANIM-3.8c1: a load makes the view anew at the save's tick.
    let loaded_at = save_file.is_some().then(|| world.tick());
    // GFX-VIEW-2.1b (a): the map view is set up with the game's first
    // frame, before the ticks a tool plays ahead.
    viewer.setup_terrain = Some(world.terrain().clone());
    let mut ahead = plays_ahead.then(|| {
        let aspect = window.width() / window.height().max(1.0);
        let all_explored = !super::fog_drawn(real_game, viewer.no_fog);
        let mut ahead = crate::render::view3d::PlayAhead::new(&world, aspect, all_explored);
        ahead.frame(&world);
        ahead
    });
    let mut frame_ahead = |world: &World| {
        if let Some(ahead) = ahead.as_mut() {
            ahead.frame(world);
        }
    };
    // --run: play the game ahead, the computer players giving their
    // commands before each tick as in `advance_world`.
    // --until-built: on until the site has done that part of its work.
    if let (Some(percent), Some(at)) = (viewer.until_built, site) {
        let done = |w: &World| match w.site(at) {
            Some(s) => s.work_done * 100 >= percent * s.work_total && percent < 100,
            None => true,
        };
        while !done(&world) && world.tick() < 20_000 {
            world.step();
            frame_ahead(&world);
        }
        info!("site at {}% in tick {}", percent, world.tick());
    }
    if viewer.run > 0 {
        let until = world.tick() + viewer.run;
        // --until-work: then on until the watched worker is in that step
        // (at most 3000 ticks more).
        let waits = viewer.until_work.is_some()
            || viewer.until_felled.is_some()
            || viewer.until_falling.is_some()
            || viewer.until_door.is_some();
        let limit = until + if waits { 3000 } else { 0 };
        while world.tick() < limit {
            if world.tick() >= until {
                let step = viewer.until_work.as_deref().unwrap_or("");
                if viewer.until_door.is_some() {
                    if door_held(&viewer, &world) {
                        info!("a door held open in tick {}", world.tick());
                        break;
                    }
                } else if let Some(after) = viewer.until_felled {
                    if let Some(tree) = felled_tree(&world, after) {
                        info!("felled tree on {tree:?} in tick {}", world.tick());
                        break;
                    }
                } else if let Some(after) = viewer.until_falling {
                    if let Some(tree) = falling_tree(&world, after) {
                        info!("falling tree on {tree:?} in tick {}", world.tick());
                        break;
                    }
                } else if watched_work(&viewer, &world).is_some_and(|w| w.starts_with(step)) {
                    break;
                }
            }
            // AI-2.1a: the computer players think last in the tick.
            match viewer.ai.as_mut() {
                Some(ai) => ai.step(&mut world),
                None => world.step(),
            }
            crate::menus::campaign::after_step(&mut world, &script, &mut mission);
            frame_ahead(&world);
        }
        // `s2-app shot` checks that this is the headless game's state: the
        // parts of the dynamic state (the full hash also holds the rules
        // and the map's mission, which the viewer sets for a real game).
        let parts: Vec<String> = world
            .state_hashes()
            .iter()
            .map(|(name, hash)| format!("{name}={hash:016x}"))
            .collect();
        info!(
            "played to tick {}, state hash {:016x}, parts {}",
            world.tick(),
            world.state_hash(),
            parts.join(" ")
        );
    }
    viewer.ahead_slots = ahead.as_ref().map(|a| a.slots.clone());
    viewer.ahead_fall_axes = ahead
        .as_ref()
        .map(|a| a.fall_axes.clone())
        .unwrap_or_default();
    viewer.ahead_marks = ahead;
    viewer.world = Some(world);
    viewer.real_game = real_game;
    viewer.saved_game = save_file.is_some();
    viewer.loaded_at = loaded_at;
    viewer.own_save = own_save;
    viewer.world_changed = true;
    viewer.moved = true;
    viewer.road_start = None;
    viewer.status.clear();
    viewer.error = None;
    viewer.patterns = textures;
    viewer.map = Some(map);
    viewer.map_file = Some(path);
}

/// `--expand <ticks>`: the scripted expansion's ticks for a game being
/// loaded, only when it is a new game, never a save (CONTINUE of a save of
/// ours, or the original's).
pub(crate) fn scripted_expansion(scenario: Option<u64>, from_save: bool) -> Option<u64> {
    scenario.filter(|_| !from_save)
}

/// A save of ours, and the map file it names: the file where it was, else
/// the install's map of the same name.
fn read_own_save(
    viewer: &Viewer,
    file: &Path,
) -> Result<(crate::game::save::Loaded, PathBuf), String> {
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    let rules = viewer
        .rules
        .as_ref()
        .ok_or("the game's rules did not load")?;
    let sim = rules.sim_rules().map_err(|e| e.to_string())?;
    // W11 b: a save of another program or other rules overlays is refused.
    let loaded = crate::game::save::read_as(&bytes, &viewer.save_mark, &sim, |world| {
        Some(s2_ai::AiSystem::new(
            s2_ai::AiConfig::new(rules, &sim),
            world,
        ))
    })
    .map_err(|e| e.to_string())?;
    let saved = PathBuf::from(&loaded.header.map);
    let map = if saved.is_file() {
        saved
    } else {
        // The name without folders and extension, whichever separators
        // the path was written with.
        let name = loaded.header.map.rsplit(['/', '\\']).next().unwrap_or("");
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        viewer
            .maps
            .iter()
            .find(|m| map_name(m).eq_ignore_ascii_case(stem))
            .cloned()
            .ok_or_else(|| format!("its map {stem} is not in the install"))?
    };
    Ok((loaded, map))
}

/// `--until-door`: a settler's step holds the door of the player's first
/// building of the type open (GFX-ANIM-6.1a b, as the drawing tests it).
pub(crate) fn door_held(viewer: &Viewer, world: &World) -> bool {
    use crate::animation::object_anim::{door_triggers, Stepper, ENTER_DISTANCE, LEAVE_DISTANCE};
    let Some(wanted) = viewer.until_door.as_ref() else {
        return false;
    };
    let Some(building) = world.buildings().find(|b| {
        b.owner == PLAYER
            && !b.under_construction
            && viewer
                .building_names
                .get(&b.building_type)
                .is_some_and(|n| n.eq_ignore_ascii_case(wanted))
    }) else {
        return false;
    };
    let Some(flag) = world.terrain().neighbour(building.at, 1) else {
        return false;
    };
    let doors = std::collections::HashMap::from([(building.at, flag)]);
    let steppers: Vec<Stepper> = world
        .settler_views()
        .iter()
        .filter_map(|v| {
            let id = match v.id {
                s2_sim::view::SettlerRef::Settler(n) => n as u64,
                s2_sim::view::SettlerRef::Carrier(n) => (1 << 32) | n as u64,
                _ => return None,
            };
            Some(Stepper {
                id,
                at: v.walk.at,
                next: v.walk.next,
                progress: v.walk.fraction(),
            })
        })
        .collect();
    !door_triggers(&doors, &steppers, ENTER_DISTANCE, LEAVE_DISTANCE).is_empty()
}

/// `--until-felled`: a tree on the player's land felled `after` ticks ago.
/// `--until-falling`: a tree of the player's whose felling began `after`
/// ticks ago (GFX-ANIM-3.8a: it falls from 30 to 53 ticks in).
pub(crate) fn falling_tree(world: &World, after: u64) -> Option<s2_sim::grid::Point> {
    world.deposits().find_map(|(field, _)| {
        let began = world.felling_began(field)?;
        (world.field_owner(field) == Some(PLAYER) && world.tick() == began + after).then_some(field)
    })
}

pub(crate) fn felled_tree(world: &World, after: u64) -> Option<s2_sim::grid::Point> {
    world.deposits().find_map(|(field, d)| match d.state {
        s2_sim::DepositState::Removing { since, .. }
            if world.fall_of(field).is_some()
                && world.field_owner(field) == Some(PLAYER)
                && world.tick() == since + after =>
        {
            Some(field)
        }
        _ => None,
    })
}

/// `--watch`, `--until-work`: the step of the watched building's worker,
/// by its name (`AtSpot { spot: .. }`).
pub(crate) fn watched_work(viewer: &Viewer, world: &World) -> Option<String> {
    let wanted = viewer.watch.as_ref()?;
    let building = world.buildings().find(|b| {
        b.owner == PLAYER
            && viewer
                .building_names
                .get(&b.building_type)
                .is_some_and(|n| n.eq_ignore_ascii_case(wanted))
    })?;
    world
        .settler_views()
        .into_iter()
        .find_map(|v| match v.activity {
            s2_sim::view::Activity::Worker { building: b, .. } if b == building.at => v.detail,
            _ => None,
        })
}

/// " | computer players: 1 (normal), 2 (strong)" for the status lines.
pub(crate) fn ai_text(ai: Option<&s2_ai::AiSystem>) -> String {
    let Some(ai) = ai else {
        return String::new();
    };
    let players: Vec<String> = (0..8u8)
        .filter_map(|slot| {
            let p = ai.player(PlayerId(slot))?;
            let name = ["weak", "normal", "strong"][usize::from(p.profile_number) - 1];
            let state = if p.active { "" } else { ", out" };
            Some(format!("{slot} ({name}{state})"))
        })
        .collect();
    format!(" | computer players: {}", players.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UI-MENU-2.1 (T20): the shipped free-game maps come out in the order
    /// read live; `_` sorts after the letters (`AB` before `A_B`) and a name
    /// that starts another comes first.
    // spec: UI-MENU-2.1 (T20)
    #[test]
    fn t20_the_order_of_the_maps() {
        let want = [
            "MP_2P_Storm_Coast",
            "MP_2P_Summer_Breeze",
            "MP_4P_black_hole",
            "MP_4P_Close_Frontier",
            "MP_4P_crossroads",
            "MP_4P_DeathValley",
            "MP_4P_Fight",
            "MP_4P_Fight2",
            "MP_6P_Paradise_Island",
            "MP_6P_the_others",
            "sp01_big_fish",
            "sp11_southshore_attack",
            "AB",
            "A_B",
        ];
        let mut maps: Vec<PathBuf> = want
            .iter()
            .rev()
            .map(|n| PathBuf::from(format!("freeGameMaps/{n}.s2m")))
            .collect();
        sort_maps(&mut maps);
        let got: Vec<String> = maps
            .iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        let mut expected: Vec<&str> = want.to_vec();
        // AB and A_B sort before every MP_ name (B < M); AB before A_B.
        expected.rotate_right(2);
        assert_eq!(got, expected);
    }
}
