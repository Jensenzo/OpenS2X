//! Models of the 3D view: the basis conversion of GFX-VIEW-1.2 and 1.3, the
//! lookup of which file draws what, a cache of loaded models and the
//! material they are drawn with.

mod library;

pub(crate) use library::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureDimension, TextureFormat, VertexFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use s2_assets::anim_tables::SettlerAnimations;
use s2_assets::gr2::{self, Gr2};
use s2_assets::model_tables::ModelTables;

use crate::animation::anim;

use s2_assets::gr2::basis::mul;
pub use s2_assets::gr2::basis::{basis_conversion, Basis};

/// GFX-VIEW-1.2 in Bevy's frame: the conversion followed by the z flip.
pub trait IntoBevy {
    fn into_bevy(self) -> Self;
}

impl IntoBevy for Basis {
    fn into_bevy(self) -> Self {
        self.flip_z()
    }
}

/// The parts of `model` converted by `basis` (in Bevy's frame), triangle
/// order reversed where the conversion mirrors.
pub fn converted_parts(model: &Gr2, basis: Basis) -> Vec<crate::tools::model::Part> {
    let normals = basis.normal_map();
    crate::tools::model::parts_with(
        model,
        |p| basis.apply(p),
        |n| mul(&normals, n),
        basis.mirrors(),
    )
}

/// The three tribes as `buildings.xml` and `ships.xml` name them, by the
/// simulation's tribe index (TRN-FREE-5.5b: 0 Romans, 1 Africans, 2
/// Chinese). Q-174.
pub const TRIBE_NAMES: [&str; 3] = ["Roman", "African", "Chinese"];

/// GFX-OBJ-1.1 (Q-174): the `buildings.xml` entry of a building type: the
/// tribe's name, `_` and the type's graphics name (`bp_setGfxName`).
pub fn building_entry(tribe: u8, gfx_name: &str) -> String {
    format!(
        "{}_{gfx_name}",
        TRIBE_NAMES[usize::from(tribe).min(TRIBE_NAMES.len() - 1)]
    )
}

/// GFX-OBJ-1.3 (Q-174): the model file of a deposit's graphics name
/// (`dp_setGfxName`), the shipped table of landscape elements: the trees,
/// the test deposits, and the stone piles by their number; the wheat field
/// is the element `Wheatfield`, `LO_Wheat` (GFX-GR2-5.1, Q-1029: the
/// leftover `RS_Wheatfield01` is never read). Used only without the table.
pub fn deposit_model(gfx_name: &str) -> Option<String> {
    const TREES: [(&str, &str); 19] = [
        ("tree_01", "RS_TreeBirch01"),
        ("tree_02", "RS_TreeBirch02"),
        ("tree_03", "RS_TreeBirch03"),
        ("tree_04", "RS_TreeBroadLeaf01"),
        ("tree_05", "RS_TreeBroadLeaf02"),
        ("tree_06", "RS_TreeBroadLeaf03"),
        ("tree_07", "RS_TreeFir01"),
        ("tree_08", "RS_TreeFir02"),
        ("tree_09", "RS_TreePalm01"),
        ("tree_10", "RS_TreePalm02"),
        ("tree_11", "RS_TreeCypress01"),
        ("tree_12", "RS_TreeOlive01"),
        ("tree_13", "RS_TreeAfrican01"),
        ("tree_14", "RS_TreeCherryBlossom01"),
        ("tree_lava01", "RS_TreeLava01"),
        ("tree_lava02", "RS_TreeLava02"),
        ("tree_lava03", "RS_TreeLava03"),
        ("Wheatfield", "LO_Wheat"),
        ("do_not_use_01", "RS_TreeBirch01"),
    ];
    if let Some((_, file)) = TREES.iter().find(|(name, _)| *name == gfx_name) {
        return Some(format!("data/map/objects/{file}.GR2"));
    }
    for (prefix, file) in [
        ("medstoneressource_", "RS_MedStoneRessource"),
        ("stoneressource_", "RS_StoneRessource"),
    ] {
        if let Some(number) = gfx_name.strip_prefix(prefix) {
            return Some(format!("data/map/objects/{file}{number}.GR2"));
        }
    }
    None
}

/// TER-7.1: the border markers, a stone on land and a buoy on water.
pub const BORDER_STONE: &str = "data/map/objects/limes.GR2";
pub const BORDER_BUOY: &str = "data/map/objects/buoy.GR2";
/// GFX-OBJ-6.1: a waterway's buoy.
pub const STREET_BUOY: &str = "data/map/objects/street_buoy.GR2";
/// GFX-OBJ-1.5: the road flag's model, the mesh of the landscape element
/// `z_flag_00`. The element's own entry is used when the landscape table is
/// loaded; this file stands for it without one (test worlds).
pub const FLAG: &str = "data/map/objects/flag.GR2";

/// FMT-S2M-2.6, CAM-4.3a: the colour of each player index in the game being
/// played. Colours are only drawn, no rule of the game reads them and the
/// simulation does not keep them; set once at each game start.
static COLOUR_OF: [AtomicU8; 8] = [
    AtomicU8::new(0),
    AtomicU8::new(1),
    AtomicU8::new(2),
    AtomicU8::new(3),
    AtomicU8::new(4),
    AtomicU8::new(5),
    AtomicU8::new(6),
    AtomicU8::new(7),
];

/// A game starts: the colour of each player index.
pub fn set_player_colours(colours: [u8; 8]) {
    for (slot, colour) in COLOUR_OF.iter().zip(colours) {
        slot.store(colour, Ordering::Relaxed);
    }
}

/// The colour index of a player in the game being played.
pub fn colour_index(player: s2_sim::PlayerId) -> u8 {
    COLOUR_OF[usize::from(player.0) % COLOUR_OF.len()].load(Ordering::Relaxed)
}

/// GFX-OBJ-2.11: the eight player colours of `scripts/menu.cfg`, which
/// flags, border markers, ships and settlers take.
pub fn player_colour(player: s2_sim::PlayerId) -> [f32; 3] {
    const COLOURS: [[f32; 3]; 8] = [
        [0.01, 0.10, 0.60],
        [0.60, 0.0, 0.0],
        [0.20, 0.50, 0.0],
        [0.75, 0.46, 0.05],
        [0.50, 0.50, 0.50],
        [0.09, 0.09, 0.09],
        [0.45, 0.0, 0.35],
        [0.20, 0.60, 0.60],
    ];
    COLOURS[usize::from(colour_index(player)) % COLOURS.len()]
}

/// GFX-VIEW-1.3: kinds without a scale setting (buildings, items, trees)
/// are drawn at 1 unit per metre.
pub const DEFAULT_SCALE: f32 = 1.0;

/// The model tables of the install, read once.
pub fn read_tables(root: &Path) -> ModelTables {
    let mut tables = ModelTables::default();
    let read = |relative: &str| -> Option<String> {
        let path = crate::extend::find(root, relative).ok()?;
        let data = crate::game::read_game_file(&path)
            .map_err(|e| eprintln!("{relative}: {e}"))
            .ok()?;
        Some(String::from_utf8_lossy(&data).into_owned())
    };
    if let Some(text) = read("data/settings/buildings.xml") {
        tables.read_buildings(&text);
    }
    if let Some(text) = read("data/settings/items.xml") {
        tables.read_items(&text);
    }
    if let Some(text) = read("data/settings/animals.xml") {
        tables.read_animals(&text);
    }
    if let Some(text) = read("data/settings/ships.xml") {
        tables.read_ships(&text);
    }
    if let Some(text) = read("data/character/settler_config.xml") {
        tables.read_characters(&text);
    }
    tables
}

/// GFX-VIEW-2.4 inputs of the object shader.
#[derive(Debug, Clone, Copy, ShaderType)]
pub struct ObjectParams {
    pub tint: Vec4,
    pub world_rect: Vec4,
    /// x: the shading family ([`Family`]).
    pub mode: Vec4,
    /// Family parameters: a settler's atlas block (offset u, v; scale u, v).
    pub extra: Vec4,
    /// The owner's player colour (GFX-OBJ-2.11).
    pub player: Vec4,
}

/// The object shaders' families (GFX-OBJ-2, 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Lit texture, alpha test (items, GFX-OBJ-2.8).
    Plain = 0,
    /// GFX-OBJ-3.3: the colour-index map through the palette atlas.
    Settler = 1,
    /// GFX-OBJ-2.1: colour x colour map, light map in the dark.
    House = 2,
    /// GFX-OBJ-2.4: swaying, vertex colour.
    Tree = 3,
    /// GFX-OBJ-2.5: plain lit texture with alpha test.
    Doodad = 4,
    /// GFX-OBJ-2.5: rocks and signs, with the underwater haze.
    Haze = 5,
    /// GFX-OBJ-2.6: per-pixel light and gloss, no alpha test.
    Stone = 6,
    /// GFX-OBJ-2.7: the wheat field, green while young.
    Wheat = 7,
    /// GFX-OBJ-2.8: the player colour where the texture's alpha is.
    Ship = 8,
    /// GFX-OBJ-2.9: waving cloth, the owner's colour where transparent.
    Flag = 9,
    /// GFX-OBJ-2.10: the owner's colour where transparent, alpha test.
    Border = 10,
    /// GFX-OBJ-2.8: animals, no alpha test.
    Animal = 11,
    /// GFX-OBJ-6.3: streets, normal-mapped, alpha-blended.
    Road = 12,
    /// GFX-OBJ-2.3: a building's ground decal, lit, alpha-blended.
    Decal = 13,
    /// GFX-VIEW-6.3 (T44): the sky cap: vertex colour x lerp(T0, T1, b),
    /// unlit, alpha-blended.
    Sky = 14,
}

impl Family {
    /// GFX-OBJ-1.2: the family of a landscape element's `shader`.
    pub fn of_shader(shader: &str) -> Self {
        match shader.to_ascii_lowercase().trim_end_matches(".fx") {
            "tree_shader" => Self::Tree,
            "stone_shader" => Self::Stone,
            "doodads_shader" => Self::Doodad,
            "rock_shader" | "signs_shader" => Self::Haze,
            "weed_shader" => Self::Wheat,
            "flag_shader" => Self::Flag,
            _ => Self::Plain,
        }
    }
}

/// GFX-OBJ-3: what paints a settler: its character's colour-index map
/// and colour map and the player colour (the ramp rows are drawn when its
/// figure is made, GFX-OBJ-3.2 (3)).
#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    pub color: String,
    pub slot_map: String,
    pub player: [f32; 3],
}

/// How a part is drawn besides its own texture.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub tint: [f32; 4],
    pub family: Family,
    /// Replaces the part's own texture (a settler's colour-index map).
    pub base: Option<Handle<Image>>,
    /// The family's second texture (the settlers' palette atlas, the
    /// houses' colour map).
    pub second: Option<Handle<Image>>,
    /// A third texture (default: the part's light map).
    pub third: Option<Handle<Image>>,
    pub extra: [f32; 4],
    pub player: [f32; 4],
}

type LookKey = (
    Family,
    [u32; 4],
    [Option<AssetId<Image>>; 3],
    [u32; 4],
    [u32; 4],
);

impl Look {
    /// A plain lit texture times `tint`.
    pub fn plain(tint: [f32; 4]) -> Self {
        Self {
            tint,
            family: Family::Plain,
            base: None,
            second: None,
            third: None,
            extra: [0.0; 4],
            player: [0.0; 4],
        }
    }

    /// A family's look with the owner's colour (white: none).
    pub fn family(family: Family, player: [f32; 3]) -> Self {
        Self {
            family,
            player: [player[0], player[1], player[2], 1.0],
            ..Self::plain([1.0; 4])
        }
    }

    /// GFX-OBJ-2.6a, 2.6b: a house whose look holds a site's progress (the
    /// rising plane's height or the frame's pieces), new at every step of the work.
    pub fn of_a_site_at_work(&self) -> bool {
        self.family == Family::House && (self.extra[1] != 0.0 || self.extra[3] != 0.0)
    }

    /// GFX-OBJ-2.8a: the shader's `extra` for one part. A ship is drawn
    /// part by part with the player colour weight set to the part's number
    /// (its triangle group's index in the file): the hull, deck, oars,
    /// rudder and mast (part 0) take no player colour, the sails and the
    /// banner (part 1) are tinted by their texture's alpha. A ship look
    /// whose weight is 0 (the construction frame) tints no part. Other
    /// families keep their look's.
    pub fn part_extra(&self, group: usize) -> [f32; 4] {
        let mut extra = self.extra;
        if self.family == Family::Ship && extra[0] > 0.0 {
            extra[0] = group as f32;
        }
        extra
    }

    pub fn key(&self) -> LookKey {
        (
            self.family,
            self.tint.map(f32::to_bits),
            [&self.base, &self.second, &self.third].map(|h| h.as_ref().map(Handle::id)),
            self.extra.map(f32::to_bits),
            self.player.map(f32::to_bits),
        )
    }
}

/// GFX-ANIM-8.2: the palette's composite matrices, as many as the shader
/// holds (GFX-ANIM-8.3); unused entries are the identity.
#[derive(Debug, Clone, Copy, ShaderType)]
pub struct Bones {
    pub m: [Mat4; anim::MAX_BONES],
}

impl Bones {
    pub const IDENTITY: Self = Self {
        m: [Mat4::IDENTITY; anim::MAX_BONES],
    };

    /// The palette's matrices taken from a pose's composites.
    pub fn from_palette(palette: &[usize], composites: &[Mat4]) -> Self {
        let mut bones = Self::IDENTITY;
        for (slot, &bone) in palette.iter().enumerate().take(anim::MAX_BONES) {
            if let Some(&m) = composites.get(bone) {
                bones.m[slot] = m;
            }
        }
        bones
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ObjectMaterial {
    #[uniform(0)]
    pub params: ObjectParams,
    #[texture(1)]
    #[sampler(2)]
    pub base: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub world: Handle<Image>,
    /// GFX-ANIM-8.2: every material's bone palette in one buffer
    /// ([`Palettes`]); this material's starts at matrix `params.mode.y`.
    #[storage(5, read_only)]
    pub palettes: Handle<ShaderBuffer>,
    #[texture(6)]
    #[sampler(7)]
    pub second: Handle<Image>,
    #[texture(8)]
    #[sampler(9)]
    pub third: Handle<Image>,
    /// GFX-VIEW-2.3, 2.3c: the day light, one buffer every object reads
    /// ([`scene_light`]), so the clock's light changes it alone (F51).
    #[storage(10, read_only)]
    pub light: Handle<ShaderBuffer>,
}

/// GFX-ANIM-8.2 (F51): the bone palettes of all object materials in one
/// buffer the object shaders read, each material's `MAX_BONES` matrices
/// from its offset (`params.mode.y`). Writing a pose's palette then
/// changes the buffer alone, not the material, so the render world
/// prepares no material again. Slot 0 is the identity palette of the
/// rigid parts. The buffer grows by doubling (a new size makes a new GPU
/// buffer; the same size is written in place). A slot given back is used again
/// (long play, F55): the whole buffer goes to the graphics card every frame, so a
/// buffer that only grew made frames of 400 ms after half an hour.
#[derive(Debug, Default)]
pub struct Palettes {
    pub handle: Option<Handle<ShaderBuffer>>,
    data: Vec<[f32; 16]>,
    used: usize,
    capacity: usize,
    dirty: bool,
    /// Slots free for the next [`Self::alloc`].
    free: Vec<u32>,
    /// Slots given back this frame: free from the next, when nothing draws
    /// with them any more.
    freed: Vec<u32>,
}

impl Palettes {
    /// A new buffer with the identity palette in slot 0.
    pub fn reset(&mut self, buffers: &mut Assets<ShaderBuffer>) {
        self.capacity = 64 * anim::MAX_BONES;
        self.data = vec![Mat4::IDENTITY.to_cols_array(); self.capacity];
        self.used = anim::MAX_BONES;
        self.handle = Some(buffers.add(self.buffer()));
        self.dirty = false;
        self.free.clear();
        self.freed.clear();
    }

    /// A slot for a palette, holding `bones`; its offset in matrices.
    pub fn alloc(&mut self, bones: &Bones) -> u32 {
        let offset = self.free.pop().unwrap_or_else(|| {
            let offset = self.used;
            self.used += anim::MAX_BONES;
            if self.used > self.capacity {
                self.capacity = (2 * self.capacity).max(self.used);
                self.data
                    .resize(self.capacity, Mat4::IDENTITY.to_cols_array());
            }
            offset as u32
        });
        self.write(offset, bones);
        offset
    }

    /// The slot at `offset` is no longer used by any material; slot 0, the
    /// rigid parts' identity, stays.
    pub fn release(&mut self, offset: u32) {
        if offset != 0 {
            self.freed.push(offset);
        }
    }

    /// The matrices the buffer holds (`--bench`, tests).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Write the palette at `offset`.
    pub fn write(&mut self, offset: u32, bones: &Bones) {
        let at = offset as usize;
        for (slot, m) in bones.m.iter().enumerate() {
            self.data[at + slot] = m.to_cols_array();
        }
        self.dirty = true;
    }

    /// Write one matrix of the palette at `offset`.
    pub fn write_slot(&mut self, offset: u32, slot: usize, m: Mat4) {
        self.data[offset as usize + slot] = m.to_cols_array();
        self.dirty = true;
    }

    /// Hand this frame's writes to the render world (one buffer).
    pub fn flush(&mut self, buffers: &mut Assets<ShaderBuffer>) {
        self.free.append(&mut self.freed);
        if !self.dirty {
            return;
        }
        self.dirty = false;
        if let Some(mut b) = self.handle.as_ref().and_then(|h| buffers.get_mut(h)) {
            *b = self.buffer();
        }
    }

    fn buffer(&self) -> ShaderBuffer {
        ShaderBuffer::new(self.data.clone(), RenderAssetUsages::RENDER_WORLD)
    }
}

/// F51: this frame's palette writes to the render world, after the poses
/// and the buildings' clips.
pub fn flush_palettes(
    mut library: ResMut<ModelLibrary>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    library.palettes.flush(&mut buffers);
}

/// The palette offset a material's parameters carry (`mode.y`).
pub fn palette_offset(params: &ObjectParams) -> u32 {
    params.mode.y as u32
}

/// GFX-VIEW-2.3: the light every object material reads: the direction (w:
/// the shadow factor), the light colour and the ambient (w: luminances).
pub fn scene_light(direction: Vec4, color: Vec4, ambient: Vec4) -> ShaderBuffer {
    ShaderBuffer::new(
        vec![direction.to_array(), color.to_array(), ambient.to_array()],
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub const OBJECT_SHADER: Handle<bevy::shader::Shader> =
    bevy::asset::uuid_handle!("8e4a1c3d-6b72-4f19-a0d5-3c9e7b2f1a64");
/// GFX-VIEW-2.5a: the objects' shadow-map pass.
pub const OBJECT_SHADOW_SHADER: Handle<bevy::shader::Shader> =
    bevy::asset::uuid_handle!("3f9d2b61-0c4e-4a87-b5e2-7d1a9c6e4b30");

/// GFX-ANIM-8.1: a vertex's four palette indices and weights.
pub const ATTRIBUTE_BONE_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("S2BoneIndex", 0x5332_424f_4e49, VertexFormat::Uint32x4);
pub const ATTRIBUTE_BONE_WEIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("S2BoneWeight", 0x5332_424f_4e57, VertexFormat::Float32x4);
/// GFX-GR2-7.3: `TextureCoordinates2` (a house's colour-map coordinates).
pub const ATTRIBUTE_UV_2: MeshVertexAttribute =
    MeshVertexAttribute::new("S2Uv2", 0x5332_5556_3200, VertexFormat::Float32x2);

impl Material for ObjectMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(OBJECT_SHADER)
    }
    /// GFX-OBJ-6.3, 2.3: streets and ground decals are blended over the
    /// terrain without writing depth.
    /// GFX-VIEW-2.5a: alpha-tested casters (leaves, a house's cut walls)
    /// may discard, also in the shadow map.
    fn alpha_mode(&self) -> AlphaMode {
        let family = self.params.mode.x;
        let is = |f: Family| family == f as u32 as f32;
        if is(Family::Road) || is(Family::Decal) || is(Family::Sky) {
            AlphaMode::Blend
        } else if [
            Family::Plain,
            Family::House,
            Family::Tree,
            Family::Doodad,
            Family::Haze,
            Family::Wheat,
        ]
        .into_iter()
        .any(is)
        {
            AlphaMode::Mask(0.5)
        } else {
            AlphaMode::Opaque
        }
    }
    /// GFX-VIEW-2.7: the see-through ground layers in their fixed order.
    fn depth_bias(&self) -> f32 {
        ground_layer_bias(self.params.mode.x)
            + street_part_bias(self.params.mode.x, self.params.extra.w)
    }
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(OBJECT_SHADER)
    }
    /// Hook W17b: the depth and normal prepass of a map camera with a
    /// project's ambient occlusion (the shadow shader, writing normals
    /// there); no camera of the original's look has a prepass.
    fn enable_prepass() -> bool {
        true
    }
    /// GFX-VIEW-2.5a: objects cast shadows with their own shape.
    fn enable_shadows() -> bool {
        true
    }
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Handle(OBJECT_SHADOW_SHADER)
    }
    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Handle(OBJECT_SHADOW_SHADER)
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_BONE_INDEX.at_shader_location(3),
            ATTRIBUTE_BONE_WEIGHT.at_shader_location(4),
            Mesh::ATTRIBUTE_UV_1.at_shader_location(5),
            ATTRIBUTE_UV_2.at_shader_location(6),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(7),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// GFX-VIEW-2.7, T27: the sort bias of a material family among the blended
/// draws, which are sorted far to near by view depth plus this bias. The
/// street pass (roads, crossings, flag-to-door strips) comes first, then
/// the ground decals, then everything else that is see-through (water,
/// marks, icons) by its distance; each bias is far beyond any depth in the
/// view (`world3d::FAR`), so a decal is blended over a road under it from
/// any camera. Borders and buildings are opaque and write depth; drawn
/// before or after the two layers they look the same.
pub fn ground_layer_bias(family: f32) -> f32 {
    let is = |f: Family| family == f as u32 as f32;
    if is(Family::Road) {
        -2.0e6
    } else if is(Family::Decal) {
        -1.0e6
    } else {
        0.0
    }
}

/// GFX-OBJ-6.2: within the street pass, by the part (`extra.w`): the donkey
/// roads first, then the other roads, then the crossings of each texture, also
/// now that each part is many meshes (F55). As [`ground_layer_bias`], each step
/// is beyond any depth in the view.
pub fn street_part_bias(family: f32, part: f32) -> f32 {
    if family == Family::Road as u32 as f32 {
        part * 1.0e5
    } else {
        0.0
    }
}

/// One part of a loaded model: its mesh, texture and bone palette (the
/// skeleton bones its vertices' indices point at, GFX-ANIM-8.2; empty for
/// a rigid part, which takes the identity).
#[derive(Debug, Clone)]
pub struct LoadedPart {
    pub mesh: Handle<Mesh>,
    pub texture: Option<Handle<Image>>,
    /// A house's light map (its material's `_L` texture, GFX-OBJ-2.1).
    pub light: Option<Handle<Image>>,
    pub palette: Vec<usize>,
    /// The `GROUND` mesh: drawn as the ground decal (GFX-OBJ-2.3).
    pub ground: bool,
    /// The `GROUND` mesh's triangles (mesh frame) with their uvs: the
    /// decal laid on the terrain (GFX-OBJ-2.3).
    pub decal: Vec<crate::render::world3d::DecalTriangle>,
    /// GFX-OBJ-2.6a: the part's place among a site model's pieces: the
    /// meshes in numeric name order (`0`, `1`, ...), or in file order when
    /// the names are not all numbers. A site at stage s shows pieces 0 to
    /// s - 1.
    pub piece: usize,
    /// GFX-OBJ-2.8a: the part's triangle group among its mesh's, in the
    /// file's order (a ship's part number: 0 the hull, 1 the sails).
    pub group: usize,
}

/// GFX-OBJ-2.3: the nearest texel of a `w` x `h` texture at `uv` with wrap
/// addressing in u and v, row 0 the top (DirectX; no flip). The decal's
/// sampler does this on the GPU (`model_image`: Repeat).
#[cfg(test)]
pub fn decal_texel(uv: [f32; 2], w: u32, h: u32) -> (u32, u32) {
    let wrap = |t: f32, n: u32| ((t - t.floor()) * n as f32).floor().min(n as f32 - 1.0) as u32;
    (wrap(uv[0], w), wrap(uv[1], h))
}

/// GFX-OBJ-1.1a: the file of a building model's level of detail `n`, the
/// model's name up to the extension, `LOD<n>`, the extension (`a/B.GR2` ->
/// `a/BLOD0.GR2`); none for a file that is a level already.
pub fn lod_file(file: &str, n: u32) -> Option<String> {
    let dot = file.rfind('.')?;
    let (stem, ext) = file.split_at(dot);
    let lower = stem.to_ascii_lowercase();
    let digits = lower.trim_end_matches(|c: char| c.is_ascii_digit());
    let is_level = digits.len() < lower.len() && digits.ends_with("lod");
    (!is_level).then(|| format!("{stem}LOD{n}{ext}"))
}

/// GFX-OBJ-2.6a: the piece order of a model's meshes by their names: in
/// numeric order when every name is a number, else in file order.
pub fn piece_order(names: &[&str]) -> Vec<usize> {
    let numbers: Option<Vec<u64>> = names.iter().map(|n| n.trim().parse().ok()).collect();
    match numbers {
        Some(numbers) => {
            let mut by_number: Vec<usize> = (0..names.len()).collect();
            by_number.sort_by_key(|&i| numbers[i]);
            let mut rank = vec![0; names.len()];
            for (r, &i) in by_number.iter().enumerate() {
                rank[i] = r;
            }
            rank
        }
        None => (0..names.len()).collect(),
    }
}

/// GFX-OBJ-2.3: the mesh of a decal laid on the terrain, from the game
/// frame into the frame `local` (Bevy world -> the parent's frame). Every
/// vertex takes the normal `up` (game frame): 2.3a d lights the decal with
/// the GROUND mesh's own normal turned with the building, not the
/// terrain's.
pub fn decal_mesh(drape: &crate::render::world3d::Drape, local: Mat4, up: [f32; 3]) -> Mesh {
    let to_local = |p: [f32; 3]| {
        local
            .transform_point3(Vec3::from(crate::render::world3d::to_bevy(p)))
            .to_array()
    };
    let normal = |n: [f32; 3]| {
        local
            .transform_vector3(Vec3::from(crate::render::world3d::to_bevy(n)))
            .normalize_or_zero()
            .to_array()
    };
    let count = drape.positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        drape
            .positions
            .iter()
            .map(|&p| to_local(p))
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal(up); count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, drape.uvs.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, drape.uvs.clone())
    .with_inserted_attribute(ATTRIBUTE_UV_2, drape.uvs.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; count])
    .with_inserted_attribute(ATTRIBUTE_BONE_INDEX, vec![[0u32; 4]; count])
    .with_inserted_attribute(ATTRIBUTE_BONE_WEIGHT, vec![[1.0f32, 0.0, 0.0, 0.0]; count])
    .with_inserted_indices(Indices::U32(drape.indices.clone()))
}

/// One loaded model: its parts as meshes with their textures, and the
/// skeleton they are bound to with the conversion of its matrices into the
/// meshes' frame.
#[derive(Debug, Clone)]
pub struct LoadedModel {
    pub parts: Vec<LoadedPart>,
    pub skeleton: Option<Arc<anim::Skeleton>>,
    /// The skeleton's file (the model's own file when it holds one).
    pub skeleton_file: String,
    pub basis: Mat4,
    pub basis_inverse: Mat4,
    /// The extent of the bind-pose vertices along x, y, z (the box's size).
    pub extent: [f32; 3],
    /// The bounding box of the whole model and of its mesh named `House`,
    /// in the game frame (GFX-OBJ-8.1a).
    pub bounds: crate::world::decor::Box3,
    pub house: Option<crate::world::decor::Box3>,
}

/// The bounding box of points given in Bevy's frame, in the game frame.
pub fn game_box<'a>(points: impl IntoIterator<Item = &'a [f32; 3]>) -> crate::world::decor::Box3 {
    let mut b = [[f32::MAX; 3], [f32::MIN; 3]];
    for &p in points {
        let g = crate::render::world3d::from_bevy(p);
        for k in 0..3 {
            b[0][k] = b[0][k].min(g[k]);
            b[1][k] = b[1][k].max(g[k]);
        }
    }
    b
}

impl LoadedModel {
    /// A pose's composites carried into the meshes' frame.
    pub fn converted(&self, composites: &[Mat4]) -> Vec<Mat4> {
        composites
            .iter()
            .map(|&c| anim::conjugate(self.basis, self.basis_inverse, c))
            .collect()
    }

    /// The rest pose's composites in the meshes' frame.
    pub fn rest(&self) -> Vec<Mat4> {
        self.skeleton
            .as_ref()
            .map(|s| self.converted(&anim::rest_composites(s)))
            .unwrap_or_default()
    }
}

/// The landscape elements of the install (GFX-OBJ-1.2).
pub fn read_landscape(root: &Path) -> Vec<s2_assets::landscape::LandscapeElement> {
    let Ok(path) = crate::extend::find(root, "data/settings/terrain_static_data.xml") else {
        return Vec::new();
    };
    match crate::game::read_game_file(&path) {
        Ok(data) => s2_assets::landscape::read(&String::from_utf8_lossy(&data)),
        Err(e) => {
            eprintln!("terrain_static_data.xml: {e}");
            Vec::new()
        }
    }
}

/// The Sound events of an animation, with their start times.
fn sound_events(events: &[s2_assets::anim_tables::AnimEvent]) -> Vec<(String, f32)> {
    events
        .iter()
        .filter(|e| e.sound)
        .map(|e| (e.name.clone(), e.start))
        .collect()
}

/// The settler animation table of the install.
pub fn read_animations(root: &Path) -> SettlerAnimations {
    let Ok(path) = crate::extend::find(root, "data/character/settler_animations.xml") else {
        return SettlerAnimations::default();
    };
    match crate::game::read_game_file(&path) {
        Ok(data) => SettlerAnimations::read(&String::from_utf8_lossy(&data)),
        Err(e) => {
            eprintln!("settler_animations.xml: {e}");
            SettlerAnimations::default()
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod install_tests {
    use super::*;

    fn load(rel: &str) -> Option<Gr2> {
        let dir = std::env::var("S2_GAME_DIR").ok()?;
        let path = std::path::Path::new(&dir).join("data/buildings").join(rel);
        let data = std::fs::read(&path).expect("file");
        Some(gr2::load(&path.to_string_lossy(), &data).expect("gr2"))
    }

    fn parts(g: &Gr2) -> Vec<crate::tools::model::Part> {
        let basis = basis_conversion(g.art_tool.as_ref(), DEFAULT_SCALE).into_bevy();
        converted_parts(g, basis)
    }

    // GFX-OBJ-2.6a, 2.6b, T3b: the Roman woodcutter's site has 16 pieces
    // named 0..15 (file order shuffled), the generic site 4; the house is
    // 4.297 high in its LOD0 file. Skipped without S2_GAME_DIR.
    #[test]
    fn woodcutter_site_pieces_and_height() {
        let Some(site) = load("PB_R_WoodcutterHut01ConstructionSite.GR2") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let names: Vec<&str> = site.meshes.iter().map(|m| m.name.as_str()).collect();
        let order = piece_order(&names);
        assert_eq!(order.len(), 16);
        for (name, rank) in names.iter().zip(&order) {
            assert_eq!(name.parse::<usize>().unwrap(), *rank);
        }
        let generic = load("PB_R_Generic01ConstructionSite.GR2").unwrap();
        assert_eq!(generic.meshes.len(), 4);
        let house = load("PB_R_WoodcutterHut01LOD0.GR2").unwrap();
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for p in parts(&house).iter().flat_map(|p| &p.positions) {
            lo = lo.min(p[1]);
            hi = hi.max(p[1]);
        }
        assert!((hi - lo - 4.297).abs() < 2e-3, "{}", hi - lo);
        // Complete in height from m = 0.8725: the plane then passes the
        // model's top above its base.
        let plane = |m| crate::world::scene3d::rising_plane(0.0, hi - lo, m);
        assert!(plane(0.8725) >= hi - 1e-3 && plane(0.8715) < hi, "{hi}");
        assert_eq!(
            lod_file("buildings/PB_R_WoodcutterHut01.GR2", 0).as_deref(),
            Some("buildings/PB_R_WoodcutterHut01LOD0.GR2")
        );
        assert_eq!(
            lod_file("buildings/PB_R_WoodcutterHut01.GR2", 1).as_deref(),
            Some("buildings/PB_R_WoodcutterHut01LOD1.GR2")
        );
        assert_eq!(lod_file("PB_R_WoodcutterHut01LOD0.GR2", 0), None);
    }

    // GFX-OBJ T16, GFX-VIEW T17: the spec's model frame is ours: a corner
    // of the hut's and of the headquarters' `GROUND` mesh, placed with the
    // building's transform (heading -30 degrees), lands where the tables
    // put it.
    // spec: GFX-OBJ-2.3, GFX-VIEW-4.3
    #[test]
    fn ground_corners_turn_with_the_house() {
        let Some(hut) = load("PB_R_WoodcutterHut01.GR2") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let check = |g: &Gr2, at: [f32; 3], corner: [f32; 3], want: [f32; 2]| {
            let ground = parts(g)
                .into_iter()
                .find(|p| p.mesh.eq_ignore_ascii_case("GROUND"))
                .expect("a GROUND mesh");
            let local = ground
                .positions
                .iter()
                .map(|&p| crate::render::world3d::from_bevy(p))
                .find(|p| (p[0] - corner[0]).abs() < 2e-3 && (p[2] - corner[2]).abs() < 2e-3)
                .unwrap_or_else(|| panic!("no GROUND vertex at {corner:?}"));
            let placed = crate::world::scene3d::Placed {
                key: (crate::world::scene3d::Kind::Building, (0, 0), 0),
                file: String::new(),
                units_per_metre: DEFAULT_SCALE,
                at,
                heading: crate::render::world3d::BUILDING_HEADING.to_radians(),
                scale: [1.0; 3],
                tilt_axis: 0.0,
                tilt: 0.0,
                tint: [1.0; 4],
                family: Family::House,
                player: [0.0; 3],
                extra: [0.0; 4],
                shadow: true,
                pieces: None,
                plane_of: None,
                parts: crate::world::scene3d::Parts::All,
                sink: None,
            };
            let t = crate::render::view3d::placed_transform(&placed);
            let w = crate::render::world3d::from_bevy(
                t.transform_point(Vec3::from(crate::render::world3d::to_bevy(local)))
                    .to_array(),
            );
            assert!(
                (w[0] - want[0]).abs() < 2e-3 && (w[2] - want[1]).abs() < 2e-3,
                "{w:?}"
            );
        };
        check(
            &hut,
            [-231.0, 4.2, -202.6499],
            [-3.478, 0.0, -3.38],
            [-232.322, -207.316],
        );
        let hq = load("PB_R_Castle01.GR2").unwrap();
        check(
            &hq,
            [-240.0, 5.0, -207.846],
            [-6.619, 0.0, -2.173],
            [-244.646, -213.037],
        );
    }

    // GFX-OBJ T17, T19, 8.1: the house boxes are the `House` meshes of
    // the LOD0 files; the soldier flag's meshes are `0` to `5`.
    #[test]
    fn house_boxes_and_flag_meshes() {
        let Some(guard) = load("PB_R_Guardhouse01LOD0.GR2") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let house_box = |g: &Gr2| {
            let parts = parts(g);
            game_box(
                parts
                    .iter()
                    .filter(|p| p.mesh.eq_ignore_ascii_case("House"))
                    .flat_map(|p| &p.positions),
            )
        };
        let near = |a: f32, b: f32| (a - b).abs() < 2e-3;
        let b = house_box(&guard);
        assert!(near(b[0][0], -2.929) && near(b[1][0], 2.929), "{b:?}");
        assert!(near(b[0][2], -3.268) && near(b[1][2], 3.416), "{b:?}");
        let b = house_box(&load("PB_R_Castle01LOD0.GR2").unwrap());
        assert!(near(b[0][0], -6.432) && near(b[1][0], 6.501), "{b:?}");
        assert!(near(b[0][2], -1.548) && near(b[1][2], 8.119), "{b:?}");
        for flag in ["MilitaryFlag1.GR2", "MilitaryFlag2.GR2"] {
            let g = load(flag).unwrap();
            let mut names: Vec<&str> = g.meshes.iter().map(|m| m.name.as_str()).collect();
            names.sort();
            if flag == "MilitaryFlag1.GR2" {
                assert_eq!(names, ["0", "1", "2", "3", "4", "5"]);
            }
        }
    }

    // GFX-OBJ T3h, 2.3a d (Run D): the decal is lit with the GROUND
    // mesh's own normal turned with the building (straight up), not the
    // terrain's: laid on a slope, every vertex of its mesh still has the
    // up normal, so its colour is (ambient + N.L x light) x texture
    // whatever the slope.
    #[test]
    fn t3h_decal_lit_as_level() {
        let slope = crate::render::world3d::Drape {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.5, 0.0], [0.0, 0.2, 1.0]],
            normals: vec![[-0.44, 0.88, -0.18]; 3],
            uvs: vec![[0.0, 0.0]; 3],
            indices: vec![0, 1, 2],
        };
        // The parent's frame turned -30 degrees about +y, as a building.
        let local = Mat4::from_rotation_y((-30f32).to_radians());
        let mesh = decal_mesh(&slope, local.inverse(), [0.0, 1.0, 0.0]);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        for n in normals {
            assert!(
                (n[0].abs() + n[2].abs()) < 1e-5 && (n[1] - 1.0).abs() < 1e-5,
                "{n:?}"
            );
        }
    }

    /// The woodcutter's GROUND alpha at game-frame (x, z): `house_decal.dds`
    /// (DXT3, alpha nibble x 17) sampled bilinearly on mip 0 with wrap at
    /// the mesh's uvs. `None` without `S2_GAME_DIR`.
    #[allow(clippy::type_complexity)]
    fn woodcutter_ground_alpha() -> Option<Box<dyn Fn(f32, f32) -> Option<f32>>> {
        let house = load("PB_R_WoodcutterHut01.GR2")?;
        let g = parts(&house)
            .into_iter()
            .find(|p| p.mesh.eq_ignore_ascii_case("GROUND"))
            .expect("a GROUND mesh");
        assert!(g.normals.iter().all(|n| n[1] > 0.999), "normals not up");
        let dir = std::env::var("S2_GAME_DIR").unwrap();
        let path = std::path::Path::new(&dir).join("data/buildings/house_decal.dds");
        let tex =
            s2_assets::dds::load(&path.to_string_lossy(), &std::fs::read(&path).unwrap()).unwrap();
        let (w, h) = (tex.width as i64, tex.height as i64);
        let texel = move |c: i64, r: i64| {
            let (c, r) = (c.rem_euclid(w), r.rem_euclid(h));
            f32::from(tex.rgba[((r * w + c) * 4 + 3) as usize])
        };
        // Bilinear with wrap, texel centres at (i + 0.5) / n.
        let alpha = move |uv: [f32; 2]| {
            let (x, y) = (uv[0] * w as f32 - 0.5, uv[1] * h as f32 - 0.5);
            let (c, r) = (x.floor() as i64, y.floor() as i64);
            let (fx, fy) = (x - x.floor(), y - y.floor());
            let top = texel(c, r) * (1.0 - fx) + texel(c + 1, r) * fx;
            let bottom = texel(c, r + 1) * (1.0 - fx) + texel(c + 1, r + 1) * fx;
            top * (1.0 - fy) + bottom * fy
        };
        // The uv at game-frame (x, z) inside the mesh, by barycentric
        // weights of the triangle holding it (Bevy z = -game z).
        let uv_at = move |x: f32, z: f32| -> Option<[f32; 2]> {
            let p = [x, -z];
            for t in g.indices.chunks(3) {
                let v = |i: u32| {
                    let q = g.positions[i as usize];
                    [q[0], q[2]]
                };
                let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
                let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
                if d.abs() < 1e-9 {
                    continue;
                }
                let l0 = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / d;
                let l1 = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / d;
                let l2 = 1.0 - l0 - l1;
                if l0 >= -1e-4 && l1 >= -1e-4 && l2 >= -1e-4 {
                    let uv = |i: u32| g.uvs[i as usize];
                    let (ua, ub, uc) = (uv(t[0]), uv(t[1]), uv(t[2]));
                    return Some([
                        l0 * ua[0] + l1 * ub[0] + l2 * uc[0],
                        l0 * ua[1] + l1 * ub[1] + l2 * uc[1],
                    ]);
                }
            }
            None
        };
        Some(Box::new(move |x, z| uv_at(x, z).map(&alpha)))
    }

    // GFX-OBJ T3h, 2.3a b (2026-10-18): the border's alpha falls in DXT3
    // steps of 17 every about 0.05 units, softened only by the bilinear
    // filter: no extra smoothing, curve or dithering.
    #[test]
    fn t3h_decal_border_dxt3_steps() {
        let Some(alpha_at) = woodcutter_ground_alpha() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let along_z = [
            255, 243, 238, 221, 204, 187, 170, 136, 119, 102, 85, 68, 51, 34, 34, 17, 17, 0,
        ];
        let along_x = [
            255, 238, 221, 204, 170, 153, 134, 102, 85, 68, 46, 34, 17, 11,
        ];
        let mut failures = Vec::new();
        for (k, want) in along_z.into_iter().enumerate() {
            let z = -2.50 - 0.05 * k as f32;
            let got = alpha_at(0.0, z).unwrap_or(0.0).round();
            if (got - want as f32).abs() > 1.0 {
                failures.push(format!("z {z:.2}: {got}, want {want}"));
            }
        }
        for (k, want) in along_x.into_iter().enumerate() {
            let x = 2.55 + 0.05 * k as f32;
            let got = alpha_at(x, -0.3).unwrap_or(0.0).round();
            if (got - want as f32).abs() > 1.0 {
                failures.push(format!("x {x:.2}: {got}, want {want}"));
            }
        }
        // Every texel alpha is a multiple of 17 (the 4-bit DXT3 value).
        let dir = std::env::var("S2_GAME_DIR").unwrap();
        let path = std::path::Path::new(&dir).join("data/buildings/house_decal.dds");
        let tex =
            s2_assets::dds::load(&path.to_string_lossy(), &std::fs::read(&path).unwrap()).unwrap();
        assert!(tex.rgba.chunks(4).all(|p| p[3] % 17 == 0));
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    // GFX-OBJ T3g, 2.3a b (Run D): the woodcutter's GROUND alpha, sampled
    // bilinearly on mip 0 at the mesh's uvs, falls linearly over its
    // borders: along model x at z = -0.4 and along z at x = 0 (game frame;
    // the mesh's normals point straight up, 2.3a d).
    #[test]
    fn t3g_decal_border_fade() {
        let Some(alpha_at) = woodcutter_ground_alpha() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let uv_alpha = |x: f32, z: f32| alpha_at(x, z);
        let mut failures = Vec::new();
        let along_x = [
            (2.565, 255.0),
            (2.7, 204.0),
            (2.9, 102.0),
            (3.1, 34.0),
            (3.244, 0.0),
            (-2.729, 255.0),
            (-2.9, 187.0),
            (-3.1, 102.0),
            (-3.3, 34.0),
        ];
        for (x, want) in along_x {
            let x = if want == 0.0 { x - 0.002 } else { x };
            let got = uv_alpha(x, -0.4);
            if !got.is_some_and(|a| (a - want).abs() <= 8.0) {
                failures.push(format!("x {x}: {got:?}, want {want}"));
            }
        }
        let along_z = [
            (-2.525, 255.0),
            (-2.8, 170.0),
            (-3.1, 51.0),
            (-3.376, 0.0),
            (1.76, 255.0),
            (2.0, 170.0),
            (2.3, 68.0),
            (2.57, 0.0),
        ];
        for (z, want) in along_z {
            let z = if z == -3.376 { z + 0.002 } else { z };
            let got = uv_alpha(0.0, z);
            if !got.is_some_and(|a| (a - want).abs() <= 8.0) {
                failures.push(format!("z {z}: {got:?}, want {want}"));
            }
        }
        assert_eq!(uv_alpha(0.0, -0.4), Some(255.0));
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    // GFX-OBJ T3e, T3f, 2.3: the decal's uvs are the GROUND mesh's own,
    // sampled with wrap and v from the top row: the woodcutter's vertex 0
    // lands on open ground (alpha 255), vertex 5 on the start of an edge
    // strip (alpha 0); the Roman castle maps the paved half.
    #[test]
    fn ground_decal_texels() {
        let Some(house) = load("PB_R_WoodcutterHut01.GR2") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let ground = |g: &Gr2| {
            parts(g)
                .into_iter()
                .find(|p| p.mesh.eq_ignore_ascii_case("GROUND"))
                .expect("a GROUND mesh")
        };
        let g = ground(&house);
        assert_eq!((g.positions.len(), g.indices.len()), (44, 54));
        let close =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3;
        assert!(close(g.uvs[0], [0.498, -0.002]), "{:?}", g.uvs[0]);
        assert!(close(g.uvs[5], [0.502, -0.124]), "{:?}", g.uvs[5]);
        let dir = std::env::var("S2_GAME_DIR").unwrap();
        let path = std::path::Path::new(&dir).join("data/buildings/house_decal.dds");
        let data = std::fs::read(&path).unwrap();
        let tex = s2_assets::dds::load(&path.to_string_lossy(), &data).unwrap();
        assert_eq!((tex.width, tex.height), (1024, 1024));
        let alpha = |uv: [f32; 2]| {
            let (c, r) = decal_texel(uv, tex.width, tex.height);
            ((c, r), tex.rgba[((r * tex.width + c) * 4 + 3) as usize])
        };
        assert_eq!(alpha(g.uvs[0]), ((509, 1021), 255));
        assert_eq!(alpha(g.uvs[5]), ((514, 897), 0));
        let castle = ground(&load("PB_R_Castle01.GR2").unwrap());
        for uv in &castle.uvs {
            assert!((0.0..0.5).contains(&uv[1]), "{uv:?}");
        }
    }
}
