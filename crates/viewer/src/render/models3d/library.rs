//! The model library: every model, texture and material of the 3D view,
//! loaded once from the install and shared (GFX-OBJ-1, GFX-GR2).

use super::*;

/// A material by its texture, tint and bone matrices (none: identity).
pub(crate) type MaterialKey = (
    [Option<AssetId<Image>>; 2],
    LookKey,
    Option<(String, usize)>,
);

/// Loaded models and textures, kept for the session; the materials are
/// kept per map, as they refer to its world texture.
#[derive(Resource)]
pub struct ModelLibrary {
    pub(crate) root: Option<PathBuf>,
    pub tables: ModelTables,
    pub animations: SettlerAnimations,
    /// GFX-ANIM-2.3, SND-2.1: the Sound events of each clip file (by its
    /// lower-case, slash path), from the first entry that has any.
    pub clip_sounds: HashMap<String, Vec<(String, f32)>>,
    /// The Sound events of each settler animation entry by name (the
    /// first entry of a name).
    pub entry_sounds: HashMap<String, Vec<(String, f32)>>,
    /// GFX-OBJ-1.2: the landscape elements.
    pub landscape: Vec<s2_assets::landscape::LandscapeElement>,
    /// Where each element's name is in [`Self::landscape`] (F55).
    pub landscape_by_name: std::collections::HashMap<String, usize>,
    /// Lower-case texture file name to its path.
    pub(crate) textures: HashMap<String, PathBuf>,
    /// GFX-GR2-5.1: the names more than one folder has a texture of, to
    /// each file (`tools::model::texture_index`).
    pub(crate) texture_copies: HashMap<String, Vec<PathBuf>>,
    /// The folder of the model being loaded, lower case with `/`: a name in
    /// more than one folder is its own copy (GFX-GR2-5.1).
    model_dir: Option<String>,
    pub(crate) models: HashMap<(String, String, u32), Option<LoadedModel>>,
    pub(crate) skeletons: HashMap<String, Option<Arc<anim::Skeleton>>>,
    pub(crate) clips: HashMap<(String, String), Option<Arc<anim::Clip>>>,
    pub(crate) images: HashMap<String, Option<Handle<Image>>>,
    pub(crate) materials: HashMap<MaterialKey, Handle<ObjectMaterial>>,
    pub(crate) white: Option<Handle<Image>>,
    /// The settlers' palette atlas, built on first use.
    pub(crate) atlas: Option<Option<(Handle<Image>, crate::render::settler_colors::Atlas)>>,
    /// The parameters of the current map's materials.
    pub params: Option<(Handle<Image>, ObjectParams)>,
    /// GFX-VIEW-2.3c: the day light buffer every material reads.
    pub light: Option<Handle<bevy::render::storage::ShaderBuffer>>,
    /// GFX-ANIM-8.2 (F51): the bone palettes of the materials.
    pub palettes: super::Palettes,
    /// The palette slot of each skinned part's rest pose.
    rest_slots: HashMap<(String, usize), u32>,
}

pub fn file_key(file: &str) -> String {
    file.replace('\\', "/").to_ascii_lowercase()
}

/// A path as a key: lower case, with `/`.
fn folder_key(path: &Path) -> String {
    file_key(&path.to_string_lossy())
}

/// GFX-GR2-5.1: a folder key from its last `data/` on (the whole key
/// without one), the folder as the game names it.
fn data_folder(key: &str) -> &str {
    key.rfind("/data/").map_or(key, |at| &key[at + 1..])
}

impl ModelLibrary {
    pub fn new(root: Option<&Path>, textures: HashMap<String, PathBuf>) -> Self {
        let landscape = root.map(read_landscape).unwrap_or_default();
        Self {
            tables: root.map(read_tables).unwrap_or_default(),
            animations: root.map(read_animations).unwrap_or_default(),
            clip_sounds: HashMap::new(),
            entry_sounds: HashMap::new(),
            // The first of a name is the one that counts.
            landscape_by_name: landscape
                .iter()
                .enumerate()
                .rev()
                .map(|(i, e)| (e.name.clone(), i))
                .collect(),
            landscape,
            root: root.map(Path::to_path_buf),
            textures,
            texture_copies: HashMap::new(),
            model_dir: None,
            models: HashMap::new(),
            palettes: super::Palettes::default(),
            rest_slots: HashMap::new(),
            skeletons: HashMap::new(),
            clips: HashMap::new(),
            images: HashMap::new(),
            materials: HashMap::new(),
            white: None,
            atlas: None,
            params: None,
            light: None,
        }
    }

    /// Index the Sound events of the settler and building clips.
    pub fn index_clip_sounds(&mut self) {
        let mut out: HashMap<String, Vec<(String, f32)>> = HashMap::new();
        let mut add = |file: &str, events: &[s2_assets::anim_tables::AnimEvent]| {
            let sounds = sound_events(events);
            if !sounds.is_empty() {
                out.entry(file_key(file)).or_insert(sounds);
            }
        };
        let mut by_entry: HashMap<String, Vec<(String, f32)>> = HashMap::new();
        for e in &self.animations.entries {
            add(&e.file, &e.events);
            by_entry
                .entry(e.name.clone())
                .or_insert_with(|| sound_events(&e.events));
        }
        self.entry_sounds = by_entry;
        for b in &self.tables.buildings {
            if let Some(w) = &b.working {
                add(&w.file, &w.events);
            }
        }
        self.clip_sounds = out;
    }

    /// The Sound events of a settler animation entry (SND-2.1).
    pub fn sounds_of_entry(&self, name: &str) -> &[(String, f32)] {
        self.entry_sounds.get(name).map_or(&[], Vec::as_slice)
    }

    /// The Sound events of a clip file.
    pub fn sounds_of(&self, file: &str) -> &[(String, f32)] {
        self.clip_sounds
            .get(&file_key(file))
            .map_or(&[], Vec::as_slice)
    }

    /// Start a map: its world texture, the materials' parameters and the
    /// day light buffer they read (GFX-VIEW-2.3c).
    pub fn set_map(
        &mut self,
        world: Handle<Image>,
        params: ObjectParams,
        light: Handle<bevy::render::storage::ShaderBuffer>,
        buffers: &mut Assets<bevy::render::storage::ShaderBuffer>,
    ) {
        self.materials.clear();
        self.rest_slots.clear();
        self.params = Some((world, params));
        self.light = Some(light);
        self.palettes.reset(buffers);
    }

    /// The model of a game-relative file at `units_per_metre`, loaded on
    /// first use; `None` when it cannot be read.
    pub fn model(
        &mut self,
        file: &str,
        units_per_metre: f32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        self.skinned_model(file, None, units_per_metre, meshes, images)
    }

    /// GFX-OBJ-1.1a: a building model's last level of detail: its
    /// `...LOD0`, `...LOD1`, ... files are loaded as long as one exists;
    /// none without a `...LOD0` file. GFX-OBJ-1.1c (Q-351): the level drawn
    /// is trunc((d x lodScale + lodBias) x (1 - p) x 0.05 + 0.5), and at the
    /// default graphic quality, VeryHigh (p = 1; this viewer has no other
    /// quality presets yet, SET-5.3), that is the model itself at every
    /// distance.
    pub fn last_lod(
        &mut self,
        file: &str,
        units_per_metre: f32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        let mut last = None;
        for n in 0.. {
            let Some(model) =
                lod_file(file, n).and_then(|lod| self.model(&lod, units_per_metre, meshes, images))
            else {
                break;
            };
            last = Some(model);
        }
        last
    }

    /// GFX-OBJ-8.1a: the box of a building model's house: the `House`
    /// mesh of its last level of detail (GFX-OBJ-1.1a) when there is one,
    /// else of the file itself (its whole box without a `House` mesh);
    /// game frame.
    pub fn house_box(
        &mut self,
        file: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<crate::world::decor::Box3> {
        if let Some(b) = self
            .last_lod(file, DEFAULT_SCALE, meshes, images)
            .and_then(|m| m.house)
        {
            return Some(b);
        }
        let m = self.model(file, DEFAULT_SCALE, meshes, images)?;
        Some(m.house.unwrap_or(m.bounds))
    }

    /// [`Self::model`] bound to the skeleton of `skeleton_file` (the
    /// settlers' shared model, an animal's model) or, without one, to the
    /// file's own first skeleton.
    pub fn skinned_model(
        &mut self,
        file: &str,
        skeleton_file: Option<&str>,
        units_per_metre: f32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        let key = (
            file_key(file),
            skeleton_file.map(file_key).unwrap_or_default(),
            units_per_metre.to_bits(),
        );
        if let Some(found) = self.models.get(&key) {
            return found.clone();
        }
        let loaded = self.load(file, skeleton_file, units_per_metre, meshes, images);
        self.models.insert(key, loaded.clone());
        loaded
    }

    fn read_gr2(&self, file: &str) -> Option<Gr2> {
        let root = self.root.as_ref()?;
        let path = crate::extend::find(root, file)
            .map_err(|e| eprintln!("model {e}"))
            .ok()?;
        let data = crate::files::read(&path).ok()?;
        gr2::load(&path.to_string_lossy(), &data)
            .map_err(|e| eprintln!("model {}: {e}", path.display()))
            .ok()
    }

    /// The first skeleton of a file.
    fn skeleton(&mut self, file: &str) -> Option<Arc<anim::Skeleton>> {
        let key = file_key(file);
        if let Some(found) = self.skeletons.get(&key) {
            return found.clone();
        }
        let found = self.read_gr2(file).and_then(|g| {
            let s = g.skeletons.first()?;
            Some(Arc::new(anim::Skeleton::from_gr2(s)))
        });
        self.skeletons.insert(key, found.clone());
        found
    }

    /// An animation file bound to the skeleton of `skeleton_file`.
    pub fn clip(&mut self, file: &str, skeleton_file: &str) -> Option<Arc<anim::Clip>> {
        let key = (file_key(file), file_key(skeleton_file));
        if let Some(found) = self.clips.get(&key) {
            return found.clone();
        }
        let found = self.skeleton(skeleton_file).and_then(|skeleton| {
            let g = self.read_gr2(file)?;
            anim::Clip::bind(&g, &skeleton).map(Arc::new)
        });
        if found.is_none() {
            eprintln!("animation {file}: not bound");
        }
        self.clips.insert(key, found.clone());
        found
    }

    fn load(
        &mut self,
        file: &str,
        skeleton_file: Option<&str>,
        units_per_metre: f32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        self.model_dir = self
            .root
            .as_ref()
            .and_then(|root| crate::extend::find(root, file).ok())
            .and_then(|path| path.parent().map(folder_key));
        let loaded = self.load_model(file, skeleton_file, units_per_metre, meshes, images);
        self.model_dir = None;
        loaded
    }

    fn load_model(
        &mut self,
        file: &str,
        skeleton_file: Option<&str>,
        units_per_metre: f32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        let model = self.read_gr2(file)?;
        let basis = basis_conversion(model.art_tool.as_ref(), units_per_metre).into_bevy();
        let skeleton_file = skeleton_file.unwrap_or(file).to_owned();
        let skeleton = if model.skeletons.is_empty() || skeleton_file != file {
            self.skeleton(&skeleton_file)
        } else {
            Some(Arc::new(anim::Skeleton::from_gr2(&model.skeletons[0])))
        };
        let palettes: Vec<Option<anim::Palette>> = model
            .meshes
            .iter()
            .map(|m| {
                let s = skeleton.as_ref()?;
                let p = anim::palette(m, s)?;
                if !p.fits() {
                    // GFX-ANIM-8.3: more bones than the shader holds (Q-090
                    // answered: the original draws it with the matrices of
                    // the house before it). Drawn unskinned.
                    eprintln!(
                        "model {file}: mesh {} uses {} bones (> {}), drawn unanimated (Q-090)",
                        m.name,
                        p.bones.len(),
                        anim::MAX_BONES
                    );
                    return None;
                }
                Some(p)
            })
            .collect();
        let parts = converted_parts(&model, basis);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in parts.iter().flat_map(|p| &p.positions) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let extent = [0, 1, 2].map(|k| (hi[k] - lo[k]).max(0.0));
        let bounds = game_box(parts.iter().flat_map(|p| &p.positions));
        let house_points: Vec<&[f32; 3]> = parts
            .iter()
            .filter(|p| p.mesh.eq_ignore_ascii_case("House"))
            .flat_map(|p| &p.positions)
            .collect();
        let house = (!house_points.is_empty()).then(|| game_box(house_points));
        let names: Vec<&str> = parts.iter().map(|p| p.mesh.as_str()).collect();
        let pieces = piece_order(&names);
        let mut out = Vec::new();
        for (part, piece) in parts.into_iter().zip(pieces) {
            // GFX-OBJ-2.1: a material's `_L` texture is the light map, the
            // other its colour.
            let is_light = |name: &&String| {
                let lower = name.to_ascii_lowercase();
                lower.trim_end_matches(".dds").ends_with("_l")
            };
            let texture = part
                .textures
                .iter()
                .find(|n| !is_light(n))
                .and_then(|name| self.image(name, images));
            let light = part
                .textures
                .iter()
                .find(is_light)
                .and_then(|name| self.image(name, images));
            let ground = part.mesh.eq_ignore_ascii_case("GROUND");
            let decal = if ground {
                part.indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| {
                        [0, 1, 2].map(|k| {
                            let i = t[k] as usize;
                            (part.positions[i], part.uvs[i])
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let count = part.positions.len();
            let (indices, weights, palette) = match &palettes[part.mesh_index] {
                Some(p) => (p.indices.clone(), p.weights.clone(), p.bones.clone()),
                None => (
                    vec![[0; 4]; count],
                    vec![[1.0, 0.0, 0.0, 0.0]; count],
                    Vec::new(),
                ),
            };
            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, part.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, part.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, part.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, part.uvs1)
            .with_inserted_attribute(ATTRIBUTE_UV_2, part.uvs2)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, part.colors)
            .with_inserted_attribute(ATTRIBUTE_BONE_INDEX, indices)
            .with_inserted_attribute(ATTRIBUTE_BONE_WEIGHT, weights)
            .with_inserted_indices(Indices::U32(part.indices));
            out.push(LoadedPart {
                mesh: meshes.add(mesh),
                texture,
                light,
                palette,
                ground,
                decal,
                piece,
                group: part.group,
            });
        }
        let basis_matrix = basis_mat4(&basis);
        Some(LoadedModel {
            parts: out,
            skeleton,
            skeleton_file,
            basis: basis_matrix,
            basis_inverse: basis_matrix.inverse(),
            extent,
            bounds,
            house,
        })
    }

    /// GFX-GR2-5.1: the file of a texture name: the model file's own folder
    /// (its path from `data/` on, so a model of an asset folder laid out like
    /// the install has the install's folder too) plus the name, no other
    /// folder searched: a name in more than one folder is always the copy
    /// beside the model, and a name only in another folder is none (Q-1029
    /// answered: no model the game draws takes one from elsewhere). A texture
    /// of a project's asset folder replaces the install's of its name
    /// wherever it lies (`docs/extending.md`). Outside a model load, the
    /// index's.
    fn texture_path(&self, name: &str) -> Option<&PathBuf> {
        let key = name.to_ascii_lowercase();
        let Some(dir) = self.model_dir.as_ref() else {
            return self.textures.get(&key);
        };
        let beside = |path: &&PathBuf| {
            path.parent()
                .map(folder_key)
                .is_some_and(|folder| data_folder(&folder) == data_folder(dir))
                || crate::extend::asset_folders()
                    .iter()
                    .any(|f| path.starts_with(f))
        };
        match self.texture_copies.get(&key) {
            Some(copies) => copies.iter().find(beside),
            None => self.textures.get(&key).filter(beside),
        }
    }

    /// A texture by its file's base name, as display values with mips.
    fn image(&mut self, name: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        let path = self.texture_path(name)?.clone();
        let key = folder_key(&path);
        if let Some(found) = self.images.get(&key) {
            return found.clone();
        }
        let image = (|| {
            let data = crate::files::read(&path).ok()?;
            let (texture, levels) =
                s2_assets::dds::load_with_levels(&path.to_string_lossy(), &data).ok()?;
            Some(images.add(model_image_with_levels(&texture, &levels)))
        })();
        self.images.insert(key, image.clone());
        image
    }

    /// The material of a texture (or none: white) with a look, for rigid
    /// parts (identity bones).
    pub fn material(
        &mut self,
        part: &LoadedPart,
        look: &Look,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        self.material_with(part, look, None, images, materials)
    }

    /// The material of a part of `model` in its rest pose (a static
    /// skinned part, as a building that plays nothing).
    pub fn rest_material(
        &mut self,
        model: &LoadedModel,
        part: usize,
        look: &Look,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        let p = &model.parts[part];
        if p.palette.is_empty() {
            return self.material(p, look, images, materials);
        }
        // F55 (the long play): one slot per part: the rest pose is the same
        // whatever the look, and a site has a new look at every step of its work.
        let key = (format!("{:?}", p.mesh.id()), part);
        let offset = match self.rest_slots.get(&key) {
            Some(&offset) => offset,
            None => {
                let bones = Bones::from_palette(&p.palette, &model.rest());
                let offset = self.palettes.alloc(&bones);
                self.rest_slots.insert(key.clone(), offset);
                offset
            }
        };
        self.material_with(p, look, Some((key, offset)), images, materials)
    }

    /// A new material not shared with anything (a pose's part, whose
    /// bones change every frame). With `bones` it gets a palette slot of
    /// its own holding them; without, the identity slot (a rigid part).
    pub fn unshared_material(
        &mut self,
        part: &LoadedPart,
        look: &Look,
        bones: Option<Bones>,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        // Without these no material comes, and no slot is taken for it.
        self.params.as_ref()?;
        self.light.as_ref()?;
        self.palettes.handle.as_ref()?;
        let offset = bones.map_or(0, |b| self.palettes.alloc(&b));
        self.material_at(part, look, offset, images, materials)
    }

    /// A new material with the palette at `offset`.
    fn material_at(
        &mut self,
        part: &LoadedPart,
        look: &Look,
        offset: u32,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        let (world, params) = self.params.clone()?;
        let light = self.light.clone()?;
        let palettes = self.palettes.handle.clone()?;
        let decal = part.ground.then(|| self.decal_look(look, images));
        let look = decal.as_ref().unwrap_or(look);
        let base = match &look.base {
            Some(b) => b.clone(),
            None => self.base(part.texture.as_ref(), images),
        };
        let second = match &look.second {
            Some(t) => t.clone(),
            None => self.base(None, images),
        };
        let third = match look.third.as_ref().or(part.light.as_ref()) {
            Some(t) => t.clone(),
            None => self.base(None, images),
        };
        Some(materials.add(ObjectMaterial {
            params: ObjectParams {
                tint: Vec4::from_array(look.tint),
                mode: Vec4::new(look.family as u32 as f32, offset as f32, 0.0, 0.0),
                extra: Vec4::from_array(look.part_extra(part.group)),
                player: Vec4::from_array(look.player),
                ..params
            },
            base,
            world,
            palettes,
            second,
            third,
            light,
        }))
    }

    fn base(
        &mut self,
        texture: Option<&Handle<Image>>,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        match texture {
            Some(texture) => texture.clone(),
            None => self
                .white
                .get_or_insert_with(|| {
                    images.add(Image::new_fill(
                        Extent3d::default(),
                        TextureDimension::D2,
                        &[200, 200, 200, 255],
                        TextureFormat::Rgba8Unorm,
                        RenderAssetUsages::RENDER_WORLD,
                    ))
                })
                .clone(),
        }
    }

    fn material_with(
        &mut self,
        part: &LoadedPart,
        look: &Look,
        rest: Option<((String, usize), u32)>,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        // F55 (the long play): a look with a site's progress in it is seen once:
        // kept, every step of every site's work stayed in memory for good.
        if look.of_a_site_at_work() {
            let offset = rest.as_ref().map_or(0, |(_, offset)| *offset);
            return self.material_at(part, look, offset, images, materials);
        }
        let decal = part.ground.then(|| self.decal_look(look, images));
        let look = decal.as_ref().unwrap_or(look);
        let mut look_key = look.key();
        // GFX-OBJ-2.8a: a ship's parts differ by their weight.
        look_key.3 = look.part_extra(part.group).map(f32::to_bits);
        let key = (
            [&part.texture, &part.light].map(|h| h.as_ref().map(Handle::id)),
            look_key,
            rest.as_ref().map(|(k, _)| k.clone()),
        );
        if let Some(found) = self.materials.get(&key) {
            return Some(found.clone());
        }
        let offset = rest.map_or(0, |(_, offset)| offset);
        let handle = self.material_at(part, look, offset, images, materials)?;
        self.materials.insert(key, handle.clone());
        Some(handle)
    }

    /// GFX-OBJ-2.3: a building's `GROUND` mesh is drawn with
    /// `buildings/house_decal.dds`, lit and alpha-blended without writing
    /// depth, in the building's look otherwise.
    fn decal_look(&mut self, look: &Look, images: &mut Assets<Image>) -> Look {
        let mut decal = Look::family(Family::Decal, [0.0; 3]);
        decal.base = self.image("house_decal.dds", images);
        decal.extra = look.extra;
        decal.player = look.player;
        decal
    }

    /// GFX-OBJ-6.3: the material of a street mesh: its texture and normal
    /// map by file name; `part` its place in the street pass (GFX-OBJ-6.2).
    pub fn street_material(
        &mut self,
        mesh: &Handle<Mesh>,
        part: u8,
        texture: &str,
        normal: &str,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        let loaded = LoadedPart {
            mesh: mesh.clone(),
            texture: self.image(texture, images),
            light: None,
            palette: Vec::new(),
            ground: false,
            decal: Vec::new(),
            piece: 0,
            group: 0,
        };
        let mut look = Look::family(Family::Road, [0.0; 3]);
        look.second = self.image(normal, images);
        look.extra[3] = f32::from(part);
        self.material(&loaded, &look, images, materials)
    }

    /// GFX-VIEW-6.3 (T44): the sky cap's material: the sky family with its
    /// two textures and the mix's `extra` ([`crate::render::view3d::SkyTextures::mix`]).
    pub fn sky_material(
        &mut self,
        mesh: &Handle<Mesh>,
        (first, second, extra): (Handle<Image>, Handle<Image>, [f32; 4]),
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> Option<Handle<ObjectMaterial>> {
        let loaded = LoadedPart {
            mesh: mesh.clone(),
            texture: Some(first.clone()),
            light: None,
            palette: Vec::new(),
            ground: false,
            decal: Vec::new(),
            piece: 0,
            group: 0,
        };
        let mut look = Look::family(Family::Sky, [0.0; 3]);
        look.base = Some(first);
        look.second = Some(second);
        look.extra = extra;
        self.material(&loaded, &look, images, materials)
    }

    /// GFX-OBJ-2: how a placed model is drawn: its family with the owner's
    /// colour; a house with its colour map (GFX-OBJ-2.1, 2.2).
    pub fn placed_look(
        &mut self,
        placed: &crate::world::scene3d::Placed,
        images: &mut Assets<Image>,
    ) -> Look {
        let mut look = Look::family(placed.family, placed.player);
        look.tint = placed.tint;
        look.extra = placed.extra;
        if placed.family == Family::House {
            let file = self
                .tables
                .color_map
                .clone()
                .unwrap_or_else(|| "data/buildings/colors.tga".into());
            look.second = self.colour_map(&file, images);
        }
        look
    }

    /// GFX-OBJ-3: how a new settler figure is painted: its colour-index
    /// map, a palette atlas block of its colour map with its own ramp rows
    /// (GFX-OBJ-3.2 (3): one draw per entry each time a figure is made),
    /// its player colour. Without the files: plain, tinted.
    pub fn settler_look(
        &mut self,
        paint: &Paint,
        images: &mut Assets<Image>,
    ) -> (Look, Option<crate::render::settler_colors::FigureBlock>) {
        let fallback = Look::plain([paint.player[0], paint.player[1], paint.player[2], 1.0]);
        let Some(base) = self.nearest_tga(&paint.color, images) else {
            return (fallback, None);
        };
        if self.atlas.is_none() {
            self.atlas = Some(self.build_atlas(images));
        }
        let Some(Some((atlas_image, atlas))) = &mut self.atlas else {
            return (fallback, None);
        };
        let Some((block, used)) = atlas.figure(&paint.slot_map) else {
            return (fallback, None);
        };
        let look = Look {
            tint: [1.0; 4],
            family: Family::Settler,
            base: Some(base),
            second: Some(atlas_image.clone()),
            third: None,
            extra: block,
            player: [paint.player[0], paint.player[1], paint.player[2], 1.0],
        };
        (look, Some(used))
    }

    /// GFX-OBJ-3.2 (3a): a settler figure is removed: its block goes back.
    pub fn release_figure(&mut self, block: &crate::render::settler_colors::FigureBlock) {
        if let Some(Some((_, atlas))) = &mut self.atlas {
            atlas.release(block);
        }
    }

    /// GFX-OBJ-3.2 (3a): a game's view is set up: the atlas starts empty.
    pub fn new_game_atlas(&mut self) {
        if let Some(Some((_, atlas))) = &mut self.atlas {
            atlas.new_game();
        }
    }

    /// GFX-OBJ-3.2 (3): the blocks of the figures made since the last call
    /// go to the GPU, the whole atlas once (into the same texture, so the
    /// figures' materials keep it).
    pub fn flush_atlas(&mut self, images: &mut Assets<Image>) {
        let Some(Some((handle, atlas))) = &mut self.atlas else {
            return;
        };
        if atlas.take_dirty() {
            if let Some(mut image) = images.get_mut(&*handle) {
                image.data = Some(atlas.rgba.clone());
            }
        }
    }

    fn build_atlas(
        &mut self,
        images: &mut Assets<Image>,
    ) -> Option<(Handle<Image>, crate::render::settler_colors::Atlas)> {
        let root = self.root.clone()?;
        let path = crate::extend::find(&root, "data/character/color_slots.xml").ok()?;
        let text = crate::game::read_game_file(&path).ok()?;
        let slots = s2_assets::color_slots::ColorSlots::read(&String::from_utf8_lossy(&text));
        let atlas = crate::render::settler_colors::Atlas::build(&slots, |file| self.read_tga(file));
        let size = crate::render::settler_colors::ATLAS_SIZE;
        let mut image = Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            atlas.rgba.clone(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        Some((images.add(image), atlas))
    }

    /// An image of the install read through `read` (a TGA), as an image
    /// sampled at its nearest texel, cached by file.
    pub fn nearest_tga(&mut self, file: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        self.tga_with(file, "tga", ImageSampler::nearest(), images)
    }

    /// GFX-OBJ-2.2b (T38): the houses' colour map, sampled bilinearly with
    /// wrap addressing in u and v (its top level only: every part reads one
    /// constant coordinate), cached by file.
    pub fn colour_map(&mut self, file: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        self.tga_with(file, "tga-colour-map", colour_map_sampler(), images)
    }

    fn tga_with(
        &mut self,
        file: &str,
        kind: &str,
        sampler: ImageSampler,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        let key = format!("{kind}:{}", file_key(file));
        if let Some(found) = self.images.get(&key) {
            return found.clone();
        }
        let image = self.read_tga(file).map(|t| {
            let mut image = Image::new(
                Extent3d {
                    width: t.width,
                    height: t.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                t.rgba,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = sampler;
            images.add(image)
        });
        self.images.insert(key, image.clone());
        image
    }

    /// A DDS texture by its file's base name as it is stored (no resizing,
    /// no mips, clamped): icon atlases and interface sprites.
    pub fn raw_image(&mut self, name: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        self.unscaled_image(name, TextureFormat::Rgba8UnormSrgb, images)
    }

    /// As [`Self::raw_image`], sampled as stored values: for unlit things on
    /// the map, whose camera works on stored values (GFX-VIEW-2.6 c1).
    pub fn stored_image(
        &mut self,
        name: &str,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        self.unscaled_image(name, TextureFormat::Rgba8Unorm, images)
    }

    fn unscaled_image(
        &mut self,
        name: &str,
        format: TextureFormat,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        let key = format!("raw:{format:?}:{}", name.to_ascii_lowercase());
        if let Some(found) = self.images.get(&key) {
            return found.clone();
        }
        let image = self
            .textures
            .get(&name.to_ascii_lowercase())
            .and_then(|path| {
                let data = crate::files::read(path).ok()?;
                let t = s2_assets::dds::load(&path.to_string_lossy(), &data).ok()?;
                let mut image = Image::new(
                    Extent3d {
                        width: t.width,
                        height: t.height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    t.rgba,
                    format,
                    RenderAssetUsages::RENDER_WORLD,
                );
                image.sampler = ImageSampler::linear();
                Some(images.add(image))
            });
        self.images.insert(key, image.clone());
        image
    }

    /// A TGA file of the install.
    /// GFX-OBJ-8.3a: a particle effect of the install.
    pub fn read_pfx(&self, file: &str) -> Option<s2_assets::pfx::Effect> {
        let root = self.root.as_ref()?;
        let path = crate::extend::find(root, file).ok()?;
        let data = crate::game::read_game_file(&path)
            .map_err(|e| eprintln!("{file}: {e}"))
            .ok()?;
        s2_assets::pfx::parse(&data)
            .map_err(|e| eprintln!("{file}: {e}"))
            .ok()
    }

    pub fn read_tga(&self, file: &str) -> Option<s2_assets::tga::Image> {
        let root = self.root.as_ref()?;
        let path = crate::extend::find(root, file).ok()?;
        let data = crate::game::read_game_file(&path)
            .map_err(|e| eprintln!("{file}: {e}"))
            .ok()?;
        s2_assets::tga::read(&data)
            .map_err(|e| eprintln!("{file}: {e}"))
            .ok()
    }
}

/// A mesh drawn with the object material from positions, texture
/// coordinates and a direction along the surface (Bevy x, z, kept in the
/// second texture coordinates), facing up, rigid.
pub fn flat_object_mesh(
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    tangents: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    let normals = vec![[0.0f32, 1.0, 0.0]; positions.len()];
    object_mesh_with_normals(positions, normals, uvs, tangents, indices)
}

/// As [`flat_object_mesh`], with a normal per vertex (a road's N(t),
/// GFX-OBJ-6.2a).
pub fn object_mesh_with_normals(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    tangents: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    let n = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, tangents)
    .with_inserted_attribute(ATTRIBUTE_UV_2, vec![[0.0f32; 2]; n])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; n])
    .with_inserted_attribute(ATTRIBUTE_BONE_INDEX, vec![[0u32; 4]; n])
    .with_inserted_attribute(ATTRIBUTE_BONE_WEIGHT, vec![[1.0f32, 0.0, 0.0, 0.0]; n])
    .with_inserted_indices(Indices::U32(indices))
}

/// A [`Basis`] as a 4 x 4 column-vector matrix.
pub fn basis_mat4(basis: &Basis) -> Mat4 {
    let l = &basis.linear;
    Mat4::from_cols(
        Vec4::new(l[0][0], l[1][0], l[2][0], 0.0),
        Vec4::new(l[0][1], l[1][1], l[2][1], 0.0),
        Vec4::new(l[0][2], l[1][2], l[2][2], 0.0),
        Vec4::new(
            basis.translation[0],
            basis.translation[1],
            basis.translation[2],
            1.0,
        ),
    )
}

/// GFX-VIEW-2.6 a: a model texture with the mip levels its file stores, as
/// stored, when they make the whole chain down to 1 x 1 of a texture whose
/// sides are powers of two; otherwise as [`model_image`] (a chain made from
/// the top level).
pub(crate) fn model_image_with_levels(
    texture: &s2_assets::dds::Texture,
    levels: &[s2_assets::dds::Level],
) -> Image {
    let (w, h) = (texture.width, texture.height);
    let whole_chain = w.is_power_of_two()
        && h.is_power_of_two()
        && w.max(h) <= 2048
        && levels.len() as u32 == w.max(h).ilog2()
        && levels.last().is_some_and(|l| (l.0, l.1) == (1, 1));
    if !whole_chain {
        return model_image(texture);
    }
    let mut image = model_image(texture);
    let mut chain = texture.rgba.clone();
    for (_, _, rgba) in levels {
        chain.extend_from_slice(rgba);
    }
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = 1 + levels.len() as u32;
    image
}

/// A model texture as an image of display values with mips (sides rounded
/// up to powers of two), repeating.
pub(crate) fn model_image(texture: &s2_assets::dds::Texture) -> Image {
    let (w, h) = (
        texture.width.next_power_of_two().clamp(1, 2048),
        texture.height.next_power_of_two().clamp(1, 2048),
    );
    let rgba = if (w, h) == (texture.width, texture.height) {
        texture.rgba.clone()
    } else {
        crate::render::textures::resample_2d(texture, w, h)
    };
    let (chain, levels) = crate::render::textures::mip_chain_2d(&rgba, w, h);
    let mut image = Image::new_uninit(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// GFX-OBJ-2.2b (a): the colour map's sampler: bilinear, wrap in u and v.
pub fn colour_map_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    })
}

/// GFX-OBJ-2.2b (a), T38: the colour a house part takes from the colour
/// map in column `column` (its slot mod 8, GFX-OBJ-2.2): the part's third
/// texture coordinate `uv2` plus the column offset in u, plus half a texel
/// in both directions, sampled bilinearly with wrap addressing. The house
/// shader (`object.wesl`) does the same on the GPU; 0..255 per channel.
#[cfg(test)]
pub fn colour_map_lookup(map: &s2_assets::tga::Image, uv2: [f32; 2], column: u32) -> [f32; 3] {
    let (w, h) = (map.width as f32, map.height as f32);
    let columns = crate::world::scene3d::COLOUR_COLUMNS as f32;
    let u = uv2[0] + (column % crate::world::scene3d::COLOUR_COLUMNS) as f32 / columns + 0.5 / w;
    let v = uv2[1] + 0.5 / h;
    // Texel centres at (i + 0.5) / w: the four around the point, wrapped.
    let (x, y) = (u * w - 0.5, v * h - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let wrap = |i: f32, n: u32| (i as i64).rem_euclid(i64::from(n)) as u32;
    let texel = |i: f32, j: f32| {
        let p = map.pixel(wrap(i, map.width), wrap(j, map.height));
        [f32::from(p[0]), f32::from(p[1]), f32::from(p[2])]
    };
    let (a, b) = (texel(x0, y0), texel(x0 + 1.0, y0));
    let (c, d) = (texel(x0, y0 + 1.0), texel(x0 + 1.0, y0 + 1.0));
    [0, 1, 2].map(|k| {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bottom = c[k] + (d[k] - c[k]) * fx;
        top + (bottom - top) * fy
    })
}

#[cfg(test)]
mod texture_tests {
    use super::*;

    // spec: GFX-GR2-5.1
    #[test]
    fn a_model_takes_the_copy_in_its_own_folder_only() {
        let (textures, copies) = crate::tools::model::texture_index(vec![
            PathBuf::from("g/data/animals/EselTextue.dds"),
            PathBuf::from("g/data/character/donkey/EselTextue.dds"),
            PathBuf::from("g/data/buildings/Mill.dds"),
        ]);
        let mut library = ModelLibrary::new(None, textures);
        library.texture_copies = copies;
        let donkey = Path::new("g/data/character/donkey/EselTextue.dds");
        let animal = Path::new("g/data/animals/EselTextue.dds");
        library.model_dir = Some(folder_key(Path::new("g/data/character/donkey")));
        assert_eq!(
            library.texture_path("EselTextue.dds").map(PathBuf::as_path),
            Some(donkey)
        );
        library.model_dir = Some(folder_key(Path::new("G/Data/Animals")));
        assert_eq!(
            library.texture_path("eseltextue.DDS").map(PathBuf::as_path),
            Some(animal)
        );
        // A model elsewhere: no other folder is searched for a name in two.
        library.model_dir = Some(folder_key(Path::new("g/data/ships")));
        assert_eq!(library.texture_path("EselTextue.dds"), None);
        // A name in one folder only is not found for a model elsewhere
        // (Q-1029 answered); a model of that folder finds it, also one of an
        // asset folder laid out like the install.
        assert_eq!(library.texture_path("Mill.dds"), None);
        library.model_dir = Some(folder_key(Path::new("g/data/buildings")));
        assert_eq!(
            library.texture_path("Mill.dds").map(PathBuf::as_path),
            Some(Path::new("g/data/buildings/Mill.dds"))
        );
        library.model_dir = Some(folder_key(Path::new("project/assets/data/buildings")));
        assert_eq!(
            library.texture_path("Mill.dds").map(PathBuf::as_path),
            Some(Path::new("g/data/buildings/Mill.dds"))
        );
        // No model being loaded: the index's.
        library.model_dir = None;
        assert_eq!(
            library.texture_path("EselTextue.dds").map(PathBuf::as_path),
            Some(animal)
        );
        assert_eq!(
            library.texture_path("mill.dds").map(PathBuf::as_path),
            Some(Path::new("g/data/buildings/Mill.dds"))
        );
        assert_eq!(library.texture_path("none.dds"), None);
    }

    /// GFX-OBJ-2.2b (a): the houses' colour map is sampled bilinearly with
    /// wrap addressing in u and v.
    // spec: GFX-OBJ-2.2b
    #[test]
    fn the_colour_map_is_bilinear_and_wraps() {
        let ImageSampler::Descriptor(d) = colour_map_sampler() else {
            panic!("a sampler descriptor");
        };
        assert_eq!(
            (d.address_mode_u, d.address_mode_v),
            (ImageAddressMode::Repeat, ImageAddressMode::Repeat)
        );
        assert_eq!(
            (d.mag_filter, d.min_filter),
            (ImageFilterMode::Linear, ImageFilterMode::Linear)
        );
    }

    /// GFX-OBJ-2.2b, T38: lookups in the shipped `colors.tga` (coordinates
    /// in texels of the 64 x 256 map, before the column offset): the half
    /// texel and the bilinear filter mix in the next band at a band's edge
    /// and the first column at the map's right edge. Skipped without
    /// `S2_GAME_DIR`.
    // spec: GFX-OBJ-2.2b (T38)
    #[test]
    fn t38_colour_map_lookups() {
        let Ok(dir) = s2_assets::GameDir::from_env() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let path =
            crate::extend::find(dir.root(), "data/buildings/colors.tga").expect("colors.tga");
        let data = crate::game::read_game_file(&path).expect("colors.tga");
        let map = s2_assets::tga::read(&data).expect("a TGA");
        assert_eq!((map.width, map.height), (64, 256));
        let at =
            |u: f32, v: f32, column: u32| colour_map_lookup(&map, [u / 64.0, v / 256.0], column);
        let near = |got: [f32; 3], want: [f32; 3]| {
            assert!(
                (0..3).all(|k| (got[k] - want[k]).abs() <= 0.6),
                "{got:?} against {want:?}"
            );
        };
        // The Roman woodcutter's roof.
        near(at(0.970, 54.925, 6), [218.0, 159.0, 94.0]);
        near(at(0.970, 54.925, 0), [234.0, 212.0, 116.0]);
        // Its log pile, half a texel above the blue band.
        near(at(1.270, 63.088, 6), [179.2, 180.1, 185.4]);
        near(at(1.270, 63.088, 7), [243.2, 245.0, 249.6]);
        // The Nubian locksmith: column 1 wraps into the first column.
        near(at(55.552, 74.343, 1), [255.0, 227.7, 148.3]);
        near(at(55.552, 74.343, 0), [255.0, 197.0, 135.0]);
        // A mesh without the third set reads (0, 0): white in every column.
        for column in 0..8 {
            near(at(0.0, 0.0, column), [255.0; 3]);
        }
    }
}
