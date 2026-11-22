//! What a game can show after its start, read when its map is loaded.
//!
//! The library loads a model the first time something shows it (the
//! original reads a building's model when its first site appears, TOOL-trace
//! TRACE-7.5). Read during play, a model's meshes and textures go to the
//! graphics card in the frame its first object appears, and its materials
//! are made then: the long play of 0.12.2 had seven slow frames in a row
//! (the worst 107 ms, in the render world's prepare step) in the frames its
//! game showed the first well's site (tick 5,335; the same game played
//! headless). So a game's start loads, behind the loading picture, every
//! model its tribes' buildings, sites, ruins, wares, settlers, animals,
//! ships, deposits and doodads can be drawn with, and their buildings'
//! clips; what the view still reads during play is in
//! [`ModelLibrary::loads`] after [`Preloaded::loads_before`], and the long
//! play names it beside a slow frame (`--bench-log`).

use super::*;

/// What [`preload_game`] read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Preloaded {
    /// The models and clips read now (those read before stay).
    pub read: usize,
    /// [`ModelLibrary::loads`]'s length after the preload: every later
    /// entry is a first use during play.
    pub loads_before: usize,
}

/// A model as the view asks for it: file, skeleton file, units per metre.
type ModelKey = (String, Option<String>, f32);

/// The models (and building clips) a game of `tribes` can show, as the
/// view asks for them (the same file, skeleton and scale, so the view finds
/// them loaded): GFX-OBJ-1.1a a building's model and its levels of detail,
/// GFX-OBJ-2.6a its site model, GFX-OBJ-8.7 (5) its ruin, GFX-OBJ-1.6a its
/// production animal, TOOL-1.2 its door and Working clips; GFX-OBJ-5.2 the
/// wares and tools carried and stacked; the settlers of the tribes, the
/// donkey, the animals, the tribes' ships (GFX-ANIM-7.1); the deposits
/// (trees, stones, the wheat field) and every doodad type of the rules, the
/// fallen soldier's skeleton among them (TRN-CARRY-7.1).
pub(crate) fn game_models(
    tables: &ModelTables,
    lookup: &crate::world::scene3d::Lookup,
    elements: &dyn Fn(&str) -> Option<String>,
    tribes: &[u8],
) -> (Vec<ModelKey>, Vec<ModelKey>, Vec<(String, ModelKey)>) {
    let mut models: Vec<ModelKey> = Vec::new();
    let mut lods: Vec<ModelKey> = Vec::new();
    // (clip file, the model whose skeleton it is bound to)
    let mut clips: Vec<(String, ModelKey)> = Vec::new();
    let plain = |file: &str| (file.to_owned(), None, DEFAULT_SCALE);
    let mut gfx: Vec<&String> = lookup
        .buildings
        .values()
        .chain(lookup.ruins.values())
        .collect();
    gfx.sort();
    gfx.dedup();
    for &tribe in tribes {
        for name in &gfx {
            let Some(entry) = tables.building(&building_entry(tribe, name)) else {
                continue;
            };
            if let Some(mesh) = &entry.mesh {
                models.push(plain(mesh));
                lods.push(plain(mesh));
                for clip in [&entry.door_open, &entry.door_close, &entry.working]
                    .into_iter()
                    .flatten()
                {
                    clips.push((clip.file.clone(), plain(mesh)));
                }
            }
            if let Some(frame) = &entry.frame {
                models.push(plain(frame));
            }
            if let Some(prod) = &entry.production {
                let animal = (prod.mesh.clone(), prod.model.clone(), DEFAULT_SCALE);
                if let Some(anim) = prod.anims.first() {
                    clips.push((anim.clone(), animal.clone()));
                }
                models.push(animal);
            }
        }
        let prefix = format!(
            "{}_",
            TRIBE_NAMES[usize::from(tribe).min(TRIBE_NAMES.len() - 1)]
        );
        for c in tables
            .characters
            .iter()
            .filter(|c| c.name.starts_with(&prefix))
        {
            models.push((
                c.mesh.clone(),
                tables.settler_model.clone(),
                tables.character_scale,
            ));
        }
        let tribe_name = TRIBE_NAMES[usize::from(tribe).min(2)];
        for ship in tables.ships.iter().filter(|s| s.tribe == tribe_name) {
            let launched = (ship.mesh.clone(), ship.model.clone(), ship.scale);
            for clip in [&ship.idle, &ship.moving].into_iter().flatten() {
                clips.push((clip.clone(), launched.clone()));
            }
            models.push(launched);
            models.push((ship.mesh.clone(), None, ship.scale));
            if let Some(frame) = &ship.frame {
                models.push((frame.clone(), None, ship.scale));
            }
        }
    }
    let mut goods: Vec<&String> = lookup
        .goods
        .values()
        .chain(lookup.tool_names.values())
        .collect();
    goods.sort();
    goods.dedup();
    for good in goods {
        if let Some(item) = tables.item(good) {
            models.push(plain(&item.mesh));
        }
    }
    if let Some(d) = &tables.donkey {
        models.push((d.mesh.clone(), d.model.clone(), d.scale));
    }
    for gfx in lookup.animals.values() {
        if let Some(a) = tables
            .animals
            .iter()
            .find(|a| a.kind.eq_ignore_ascii_case(gfx))
        {
            models.push((a.mesh.clone(), a.model.clone(), a.scale));
        }
    }
    for look in lookup.deposits.values() {
        if let Some(mesh) = elements(&look.gfx_name).or_else(|| deposit_model(&look.gfx_name)) {
            models.push(plain(&mesh));
        }
    }
    for tree in lookup.tree_looks.values() {
        models.push(plain(&tree.model));
        if let Some(far) = &tree.far {
            models.push(plain(far));
        }
    }
    // Hook W30c: the map objects' own models.
    for object in lookup
        .object_deposits
        .values()
        .chain(lookup.object_doodads.values())
    {
        models.push(plain(&object.model));
    }
    for (gfx, _) in lookup.doodads.values() {
        if let Some(mesh) = elements(gfx) {
            models.push(plain(&mesh));
        }
    }
    let mut seen = std::collections::HashSet::new();
    models.retain(|(f, s, u)| seen.insert((file_key(f), s.as_deref().map(file_key), u.to_bits())));
    lods.dedup();
    let mut seen_clips = std::collections::HashSet::new();
    clips.retain(|(c, (f, s, _))| seen_clips.insert((file_key(c), file_key(f), s.clone())));
    (models, lods, clips)
}

/// Read every model of [`game_models`] that is not loaded yet; the
/// library then counts every later read as a first use during play.
pub fn preload_game(
    library: &mut ModelLibrary,
    lookup: &crate::world::scene3d::Lookup,
    tribes: &[u8],
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
) -> Preloaded {
    let before = library.loads.len();
    let landscape = std::mem::take(&mut library.landscape);
    let by_name = std::mem::take(&mut library.landscape_by_name);
    let element = |name: &str| by_name.get(name).map(|&i| landscape[i].mesh.clone());
    let (models, lods, clips) = game_models(&library.tables, lookup, &element, tribes);
    library.landscape = landscape;
    library.landscape_by_name = by_name;
    for (file, skeleton, units) in &models {
        library.skinned_model(file, skeleton.as_deref(), *units, meshes, images);
    }
    for (file, _, units) in &lods {
        library.last_lod(file, *units, meshes, images);
    }
    for (clip, (file, skeleton, units)) in &clips {
        // The model is loaded above: its skeleton's file as the view binds
        // the clip (`pose_key`, the building clips of `building_anims`).
        if let Some(model) =
            library.skinned_model(file, skeleton.as_deref(), *units, meshes, images)
        {
            if model.skeleton.is_some() {
                library.clip(clip, &model.skeleton_file);
            }
        }
    }
    Preloaded {
        read: library.loads.len() - before,
        loads_before: library.loads.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.12.2's long play stalled 107 ms when its game showed the first well's
    /// site. After a Roman game's preload every building type's house, site
    /// model and ruin, every ware and tool, and every doodad type (the fallen
    /// soldier's skeleton among them) is drawn without reading a file; a
    /// model read later is recorded as a first use during play. Skipped
    /// without S2_GAME_DIR.
    // spec: GFX-OBJ-1.1a, GFX-OBJ-2.6a, GFX-OBJ-8.7, GFX-OBJ-5.2, TRN-CARRY-7.1
    #[test]
    fn a_games_models_are_read_before_its_play() {
        let Some(dir) = std::env::var_os("S2_GAME_DIR").map(PathBuf::from) else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let game = s2_assets::GameDir::new(&dir).expect("game dir");
        let (rules, _) = s2_rules::load(&game).expect("rules");
        let mut library = ModelLibrary::new(Some(&dir), HashMap::new());
        let lookup = crate::world::scene3d::Lookup::new(&rules, &library.tables);
        let mut meshes = Assets::<Mesh>::default();
        let mut images = Assets::<Image>::default();
        let started = std::time::Instant::now();
        let pre = preload_game(&mut library, &lookup, &[0], &mut meshes, &mut images);
        eprintln!(
            "preload: {} models and clips in {:?}",
            pre.read,
            started.elapsed()
        );
        assert!(pre.read > 100, "{pre:?}");
        assert_eq!(pre.loads_before, library.loads.len());
        let tables = library.tables.clone();
        let mut drawn = Vec::new();
        for b in &rules.buildings {
            let Some(gfx) = lookup.buildings.get(&b.id) else {
                continue;
            };
            let Some(entry) = tables.building(&building_entry(0, gfx)) else {
                continue;
            };
            drawn.extend(entry.mesh.clone());
            drawn.extend(entry.frame.clone());
            if let Some(ruin) = lookup.ruins.get(&b.id) {
                drawn.extend(
                    tables
                        .building(&building_entry(0, ruin))
                        .and_then(|r| r.mesh.clone()),
                );
            }
        }
        assert!(drawn.iter().any(|f| f.contains("Well01")), "{drawn:?}");
        for good in lookup.goods.values() {
            drawn.extend(tables.item(good).map(|i| i.mesh.clone()));
        }
        let skeleton = rules
            .settings
            .settler_skeleton_doodad
            .expect("the skeleton doodad");
        for (id, (gfx, _)) in &lookup.doodads {
            let mesh = library
                .landscape_by_name
                .get(gfx)
                .map(|&i| library.landscape[i].mesh.clone());
            assert!(
                mesh.is_some() || *id != skeleton,
                "the skeleton's element {gfx}"
            );
            drawn.extend(mesh);
        }
        for file in &drawn {
            library.model(file, DEFAULT_SCALE, &mut meshes, &mut images);
            assert_eq!(
                library.loads.len(),
                pre.loads_before,
                "{file} read during play"
            );
        }
        // Another tribe's figure is not this game's: read when first shown.
        let african = tables
            .characters
            .iter()
            .find(|c| c.name.starts_with("African_"))
            .expect("an African settler");
        library.skinned_model(
            &african.mesh,
            tables.settler_model.as_deref(),
            tables.character_scale,
            &mut meshes,
            &mut images,
        );
        assert_eq!(
            library.loads[pre.loads_before..],
            [format!("model {}", african.mesh)]
        );
    }
}
