//! Hook W30 (S2N-003): a project's own building model, a binary glTF file
//! (`.glb`, read by `s2_assets::glb`), drawn by the engine as it draws the
//! original's houses. Not part of the original game; nothing here runs
//! unless a project names a model (`extend::BuildingLook`).
//!
//! How it is drawn (`docs/extending.md`, "A building's own model"):
//! - Every node of the file becomes a bone of one skeleton, in the file's
//!   tree; every mesh is bound rigidly to its own node's bone, so a clip
//!   that turns or moves a node moves its object (glTF node animation, the
//!   engine's door and Working clips, GFX-ANIM-6). Empty nodes (the
//!   `anchor_` points) are bones without a mesh.
//! - The glTF frame (+Y up, the door towards +Z) is the 3D view's own
//!   frame, so the model is not converted; the engine turns it by the
//!   building heading as every house (GFX-VIEW-4.3a).
//! - Faces keep the normals of the file (flat as authored). A face's
//!   colour is the palette image times the vertex colour (its occlusion);
//!   roughness and metallic come from the second image (`facet` families
//!   of the object shader). The node `teamcolour` takes its owner's colour;
//!   the node `ground` (or a material named `..._ground`) is laid on the
//!   terrain as a building's ground decal and blended by its vertex alpha;
//!   everything else is opaque.

use super::*;
use s2_assets::glb::{self, Glb};

/// The node tinted with the owner's colour.
pub const TEAM_NODE: &str = "teamcolour";
/// The node laid on the terrain, blended by its vertex alpha.
pub const GROUND_NODE: &str = "ground";
/// The point the smoke of a working building rises from.
pub const SMOKE_ANCHOR: &str = "anchor_smoke";
/// The clips the engine plays (GFX-ANIM-6.1, 6.2).
pub const DOOR_OPEN: &str = "DoorOpen";
pub const DOOR_CLOSE: &str = "DoorClose";
pub const WORKING: &str = "Working";

/// Hook W30b (project buildings at night): the light map a project's
/// building is darkened by at night, in place of the original houses' own
/// (GFX-OBJ-2.1: theirs multiplies a face by saturate(1.5 - d), all of it
/// at night): the median over the Roman houses of their light map's mean
/// where their faces sample it (`tests::the_houses_light_maps_at_night`:
/// 0.30 the gold mine .. 0.70 the bakery, the woodcutter's 0.41). The
/// object shader holds the same number (`NIGHT_LIGHT_MAP`).
#[cfg(test)]
pub const NIGHT_LIGHT_MAP: f32 = 0.533;

/// Hook W30b: how much of the night a project's building is darkened by:
/// none while the light and ambient luminances add up to the day's
/// (GFX-VIEW-2.3: 0.8964 + 0.5413 at noon), all of it at the night's
/// (0.2617 + 0.3376, the shipped day cycle's hours 23 to 1), in between
/// linearly (the evening and the morning). The shader's `night_amount`.
#[cfg(test)]
pub fn night_amount(light_luminance: f32, ambient_luminance: f32) -> f32 {
    ((NIGHT_DAY - (light_luminance + ambient_luminance)) / (NIGHT_DAY - NIGHT_DARK)).clamp(0.0, 1.0)
}

/// Hook W30b: the light and ambient luminances added at noon and at night
/// (`night_amount`).
#[cfg(test)]
pub const NIGHT_DAY: f32 = 1.4377;
#[cfg(test)]
pub const NIGHT_DARK: f32 = 0.5993;

/// How a part of a project's model is shaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetKind {
    /// Opaque: palette x vertex colour.
    Solid,
    /// Opaque and tinted by the owner's colour.
    Team,
    /// Laid on the terrain, blended by the vertex alpha.
    Ground,
}

/// What a part of a project's model has beyond an original's part.
#[derive(Debug, Clone, PartialEq)]
pub struct Facet {
    pub kind: FacetKind,
    /// A ground part's colour (with its alpha) per triangle of its decal.
    pub decal_colours: Vec<[f32; 4]>,
}

/// The look of a project model's part: the placed look (its owner's
/// colour, the house's cut plane in `extra`) in the part's family; the
/// part's own images (palette, roughness and metallic).
pub fn facet_look(facet: &Facet, look: &Look) -> Look {
    let mut out = look.clone();
    out.family = match facet.kind {
        FacetKind::Ground => Family::FacetGround,
        // Hook W30b: a project's tree keeps the tree family of its look.
        _ if look.family == Family::FacetTree => Family::FacetTree,
        FacetKind::Solid | FacetKind::Team => Family::Facet,
    };
    out.base = None;
    out.second = None;
    out.third = None;
    // The colour map's column (`extra[0]`) is the original houses'; the
    // frame pass (`extra[3]`) a site model's.
    out.extra[0] = 0.0;
    out.extra[3] = 0.0;
    out
}

/// The shader's owner-colour weight of a part (`mode.z`): 1 for a
/// project model's `teamcolour` part, else 0.
pub fn team_weight(part: &LoadedPart) -> f32 {
    match part.facet.as_ref().map(|f| f.kind) {
        Some(FacetKind::Team) => 1.0,
        _ => 0.0,
    }
}

/// A model file of a project (`.glb`), by its extension.
pub fn is_project_model(file: &str) -> bool {
    file.to_ascii_lowercase().ends_with(".glb")
}

/// The name a building entry gives a clip of a project's model:
/// `<model>#<clip>`.
pub fn clip_file(model: &str, clip: &str) -> String {
    format!("{model}#{clip}")
}

/// A project clip's model file and clip name.
pub fn split_clip(file: &str) -> Option<(&str, &str)> {
    let (model, clip) = file.rsplit_once('#')?;
    is_project_model(model).then_some((model, clip))
}

/// The skeleton of a file: one bone per node, parents first, named as the
/// nodes; the rest pose is the nodes' own transforms.
pub fn skeleton(glb: &Glb) -> (anim::Skeleton, Vec<usize>) {
    let order = glb.ordered_nodes();
    let mut bone_of = vec![0usize; glb.nodes.len()];
    for (bone, &node) in order.iter().enumerate() {
        bone_of[node] = bone;
    }
    let rest: Vec<anim::Local> = order
        .iter()
        .map(|&n| {
            let node = &glb.nodes[n];
            anim::Local {
                position: Vec3::from_array(node.translation),
                orientation: Quat::from_array(node.rotation).normalize(),
                scale_shear: Mat3::from_diagonal(Vec3::from_array(node.scale)),
            }
        })
        .collect();
    let parents: Vec<Option<usize>> = order
        .iter()
        .map(|&n| glb.nodes[n].parent.map(|p| bone_of[p]))
        .collect();
    let mut skeleton = anim::Skeleton {
        name: "root".into(),
        names: order.iter().map(|&n| glb.nodes[n].name.clone()).collect(),
        parents,
        rest,
        inverse_world: Vec::new(),
    };
    skeleton.inverse_world = skeleton
        .world(&skeleton.rest)
        .iter()
        .map(Mat4::inverse)
        .collect();
    (skeleton, bone_of)
}

/// One part of a project's model, ready for a mesh: the vertices in the
/// model's rest pose (the node's transform applied), the bone it follows.
#[derive(Debug, Clone, PartialEq)]
pub struct PartData {
    pub node: String,
    pub bone: usize,
    pub kind: FacetKind,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colours: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub material: Option<usize>,
}

/// The parts of a file: one per primitive of each node's mesh, in node
/// order.
pub fn parts(glb: &Glb, skeleton: &anim::Skeleton, bone_of: &[usize]) -> Vec<PartData> {
    let world = skeleton.world(&skeleton.rest);
    let mut out = Vec::new();
    for (n, node) in glb.nodes.iter().enumerate() {
        let Some(mesh) = node.mesh.and_then(|m| glb.meshes.get(m)) else {
            continue;
        };
        let bone = bone_of[n];
        let m = world[bone];
        let normal_matrix = Mat3::from_mat4(m).inverse().transpose();
        for p in &mesh.primitives {
            let material_name = p
                .material
                .and_then(|i| glb.materials.get(i))
                .map(|m| m.name.to_ascii_lowercase())
                .unwrap_or_default();
            let kind = if node.name == GROUND_NODE || material_name.ends_with("_ground") {
                FacetKind::Ground
            } else if node.name == TEAM_NODE {
                FacetKind::Team
            } else {
                FacetKind::Solid
            };
            let normals = if p.normals.len() == p.positions.len() {
                p.normals
                    .iter()
                    .map(|&v| {
                        (normal_matrix * Vec3::from_array(v))
                            .normalize_or_zero()
                            .to_array()
                    })
                    .collect()
            } else {
                flat_normals(&p.positions, &p.indices)
            };
            out.push(PartData {
                node: node.name.clone(),
                bone,
                kind,
                positions: p
                    .positions
                    .iter()
                    .map(|&v| m.transform_point3(Vec3::from_array(v)).to_array())
                    .collect(),
                normals,
                uvs: p.uvs.clone(),
                colours: p.colours.clone(),
                indices: p.indices.clone(),
                material: p.material,
            });
        }
    }
    out
}

/// A face normal per corner for a mesh without normals (each corner takes
/// the normal of the last triangle it is in).
fn flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0, 1.0, 0.0]; positions.len()];
    for t in indices.as_chunks::<3>().0.iter() {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| Vec3::from_array(positions[i as usize]));
        let n = (b - a).cross(c - a).normalize_or_zero().to_array();
        for &i in t {
            out[i as usize] = n;
        }
    }
    out
}

/// A clip of a file bound to its skeleton: each channel a curve of its
/// node's bone, linear between keys (the engine's curves of degree 1); a
/// step channel is sampled as linear.
pub fn clip(
    glb: &Glb,
    name: &str,
    skeleton: &anim::Skeleton,
    bone_of: &[usize],
) -> Option<anim::Clip> {
    let animation = glb.animation(name)?;
    let mut tracks: Vec<Option<[gr2::Curve; 3]>> = vec![None; skeleton.len()];
    let empty = || gr2::Curve {
        degree: 1,
        knots: Vec::new(),
        controls: Vec::new(),
    };
    for c in &animation.channels {
        let bone = bone_of[c.node];
        let track = tracks[bone].get_or_insert_with(|| [empty(), empty(), empty()]);
        let controls = match c.path {
            glb::Path::Translation | glb::Path::Rotation => c.values.clone(),
            // The scale as the row-major 3 x 3 matrix the curves hold.
            glb::Path::Scale => c
                .values
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|s| [s[0], 0.0, 0.0, 0.0, s[1], 0.0, 0.0, 0.0, s[2]])
                .collect(),
        };
        let slot = match c.path {
            glb::Path::Translation => 0,
            glb::Path::Rotation => 1,
            glb::Path::Scale => 2,
        };
        track[slot] = gr2::Curve {
            degree: 1,
            knots: c.times.clone(),
            controls,
        };
    }
    Some(anim::Clip::from_curves(animation.duration(), tracks))
}

/// An embedded PNG image as RGBA bytes (8 bits a channel; grey, grey with
/// alpha and RGB widened), with its size.
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("the image is too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let pixels = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("an indexed image was not expanded".into()),
    };
    Ok((info.width, info.height, rgba))
}

/// An image of a project's model as the 3D view's textures are: display
/// values (the palette's sRGB bytes as they are), with mips, clamped at
/// the edges and filtered to the nearest texel at close range (a facet's
/// one swatch).
pub fn image(bytes: &[u8]) -> Option<Image> {
    let (width, height, rgba) = decode_png(bytes)
        .map_err(|e| eprintln!("model image: {e}"))
        .ok()?;
    let texture = s2_assets::dds::Texture {
        width,
        height,
        format: s2_assets::dds::Format::Masks {
            bits: 32,
            masks: [0xFF, 0xFF00, 0xFF_0000, 0xFF00_0000],
        },
        mip_levels: 1,
        cube_map: false,
        rgba,
    };
    let mut image = model_image(&texture);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    });
    Some(image)
}

/// The decal triangles (mesh frame) of a ground part with each triangle's
/// colour (its first corner's: a face has one).
pub fn ground_triangles(
    part: &PartData,
) -> (Vec<crate::render::world3d::DecalTriangle>, Vec<[f32; 4]>) {
    part.indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let corner = |k: usize| {
                let i = t[k] as usize;
                (part.positions[i], part.uvs[i])
            };
            (
                [corner(0), corner(1), corner(2)],
                part.colours[t[0] as usize],
            )
        })
        .unzip()
}

/// GFX-OBJ-2.3 for a project's ground part: each triangle laid on the
/// terrain as a decal is (`world3d::drape`), its pieces in the triangle's
/// colour, every vertex with the up normal turned with the building.
pub fn ground_mesh(
    heights: &impl crate::render::world3d::Heights,
    triangles: &[crate::render::world3d::DecalTriangle],
    colours: &[[f32; 4]],
    to_world: Mat4,
) -> Mesh {
    let mut all = crate::render::world3d::Drape::default();
    let mut vertex_colours = Vec::new();
    for (t, colour) in triangles.iter().zip(colours) {
        let world = t.map(|(p, uv)| {
            let w = to_world.transform_point3(Vec3::from(p));
            (crate::render::world3d::from_bevy(w.to_array()), uv)
        });
        let piece = crate::render::world3d::drape(heights, &[world]);
        let base = all.positions.len() as u32;
        vertex_colours.extend(std::iter::repeat_n(*colour, piece.positions.len()));
        all.positions.extend(piece.positions);
        all.normals.extend(piece.normals);
        all.uvs.extend(piece.uvs);
        all.indices.extend(piece.indices.iter().map(|i| i + base));
    }
    let up = to_world.transform_vector3(Vec3::Y).normalize();
    let mut mesh = decal_mesh(
        &all,
        to_world.inverse(),
        crate::render::world3d::from_bevy(up.to_array()),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colours);
    mesh
}

/// Hook W30: the graphics name the 3D view gives a building type with a
/// project's look (`Lookup`), in place of its own: no install entry has
/// one like it.
pub fn look_gfx(building: &str) -> String {
    format!("look:{building}")
}

/// Hook W30: the graphics names of the building types with a project's
/// look, by type id: a type named as a look's `building`.
pub fn look_gfx_names<'a>(
    types: impl IntoIterator<Item = (u32, Option<&'a str>)>,
    looks: &[crate::extend::BuildingLook],
) -> Vec<(u32, String)> {
    types
        .into_iter()
        .filter_map(|(id, name)| {
            let name = name?;
            looks
                .iter()
                .any(|l| l.building == name)
                .then(|| (id, look_gfx(name)))
        })
        .collect()
}

/// Hook W30: the building entries of the projects' looks, one per tribe
/// (`<tribe>_look:<type>`, as `buildings.xml` names its entries,
/// GFX-OBJ-1.1, with [`look_gfx`]): the model as its mesh, its `DoorOpen`,
/// `DoorClose` and `Working` clips, the site model of the type named by
/// `site_of` (of the same tribe), the smoke as a `Particle` event of its
/// Working clip at the point `anchor_smoke`, the loader's heading.
pub fn add_building_looks(tables: &mut ModelTables, looks: &[crate::extend::BuildingLook]) {
    use s2_assets::model_tables::{BuildingClip, BuildingModel, DEFAULT_BUILDING_HEADING};
    for look in looks {
        for tribe in 0..TRIBE_NAMES.len() as u8 {
            let name = building_entry(tribe, &look_gfx(&look.building));
            let frame = look
                .site_of
                .as_ref()
                .and_then(|site| tables.building(&building_entry(tribe, site)))
                .and_then(|site| site.frame.clone());
            let clip = |name: &str, events| {
                Some(BuildingClip {
                    file: clip_file(&look.model, name),
                    events,
                })
            };
            let smoke = look
                .smoke
                .iter()
                .map(|effect| s2_assets::anim_tables::AnimEvent {
                    sound: false,
                    name: effect.clone(),
                    start: 0.0,
                    slot: Some(SMOKE_ANCHOR.into()),
                })
                .collect();
            let entry = BuildingModel {
                name: name.clone(),
                mesh: Some(look.model.clone()),
                frame,
                door_open: clip(DOOR_OPEN, Vec::new()),
                door_close: clip(DOOR_CLOSE, Vec::new()),
                working: clip(WORKING, smoke),
                production: None,
                heading: DEFAULT_BUILDING_HEADING,
            };
            tables.buildings.retain(|b| b.name != name);
            tables.buildings.push(entry);
        }
    }
}

/// Read a project's model file.
pub fn read(path: &Path) -> Option<Glb> {
    let data = crate::files::read(path).ok()?;
    glb::load(&data)
        .map_err(|e| eprintln!("model {}: {e}", path.display()))
        .ok()
}

impl ModelLibrary {
    /// Hook W30: a project's model loaded as an original's is: its parts
    /// with their meshes and images, its skeleton of nodes.
    pub(crate) fn load_project_model(
        &mut self,
        file: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        let glb = self.read_project_model(file)?;
        let (skeleton, bone_of) = skeleton(&glb);
        if skeleton.len() > anim::MAX_BONES {
            eprintln!(
                "model {file}: {} nodes (> {}), not drawn",
                skeleton.len(),
                anim::MAX_BONES
            );
            return None;
        }
        let image_handles: Vec<Option<Handle<Image>>> = glb
            .images
            .iter()
            .map(|i| image(&i.bytes).map(|i| images.add(i)))
            .collect();
        let texture_of = |material: Option<usize>, orm: bool| -> Option<Handle<Image>> {
            let m = glb.materials.get(material?)?;
            let index = if orm {
                m.metallic_roughness
            } else {
                m.base_colour
            }?;
            image_handles.get(index)?.clone()
        };
        let data = parts(&glb, &skeleton, &bone_of);
        // The house's box and height: everything but the ground.
        let body = || {
            data.iter()
                .filter(|p| p.kind != FacetKind::Ground)
                .flat_map(|p| &p.positions)
        };
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in body() {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let extent = [0, 1, 2].map(|k| (hi[k] - lo[k]).max(0.0));
        let house = game_box(body());
        let bounds = game_box(data.iter().flat_map(|p| &p.positions));
        let mut out = Vec::new();
        for (group, part) in data.into_iter().enumerate() {
            let texture = texture_of(part.material, false);
            let light = texture_of(part.material, true);
            let (decal, decal_colours) = if part.kind == FacetKind::Ground {
                ground_triangles(&part)
            } else {
                (Vec::new(), Vec::new())
            };
            let count = part.positions.len();
            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, part.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, part.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, part.uvs.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, part.uvs.clone())
            .with_inserted_attribute(ATTRIBUTE_UV_2, part.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, part.colours)
            .with_inserted_attribute(ATTRIBUTE_BONE_INDEX, vec![[0u32; 4]; count])
            .with_inserted_attribute(ATTRIBUTE_BONE_WEIGHT, vec![[1.0f32, 0.0, 0.0, 0.0]; count])
            .with_inserted_indices(Indices::U32(part.indices));
            out.push(LoadedPart {
                mesh: meshes.add(mesh),
                texture,
                light,
                palette: vec![part.bone],
                ground: false,
                decal,
                piece: group,
                group,
                facet: Some(Facet {
                    kind: part.kind,
                    decal_colours,
                }),
            });
        }
        Some(LoadedModel {
            parts: out,
            skeleton: Some(Arc::new(skeleton)),
            skeleton_file: file.to_owned(),
            basis: Mat4::IDENTITY,
            basis_inverse: Mat4::IDENTITY,
            extent,
            bounds,
            house: Some(house),
        })
    }

    /// A project's model file read (each read again: models are loaded
    /// once and kept, clips once per name).
    fn read_project_model(&self, file: &str) -> Option<Glb> {
        let root = self.root.as_ref()?;
        let path = crate::extend::find(root, file)
            .map_err(|e| eprintln!("model {e}"))
            .ok()?;
        read(&path)
    }

    /// Hook W30: a clip of a project's model (`<model>#<clip>`), bound to
    /// the skeleton of its nodes.
    pub(crate) fn project_clip(&self, file: &str) -> Option<anim::Clip> {
        let (model, name) = split_clip(file)?;
        let glb = self.read_project_model(model)?;
        let (skeleton, bone_of) = skeleton(&glb);
        clip(&glb, name, &skeleton, &bone_of)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Glb {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/models/facets.glb");
        glb::load(&std::fs::read(path).expect("tests/models/facets.glb")).expect("loads")
    }

    // spec: none (hook W30: every node a bone, parents first; the rest pose the nodes' transforms)
    #[test]
    fn the_nodes_are_the_skeleton() {
        let glb = fixture();
        let (skeleton, bone_of) = skeleton(&glb);
        assert_eq!(skeleton.name, "root");
        assert_eq!(skeleton.len(), glb.nodes.len());
        let door = skeleton.find("door").expect("door");
        assert_eq!(bone_of[glb.node("door").expect("door")], door);
        assert_eq!(skeleton.rest[door].position, Vec3::new(0.5, 0.0, 1.0));
        // At rest every composite is the identity: the parts are stored in
        // the rest pose.
        for c in anim::rest_composites(&skeleton) {
            assert!(c.abs_diff_eq(Mat4::IDENTITY, 1e-5));
        }
        assert!(skeleton.find(SMOKE_ANCHOR).is_some());
    }

    // spec: none (hook W30: parts in the rest pose, flat normals kept, kinds by name)
    #[test]
    fn parts_keep_their_faces() {
        let glb = fixture();
        let (skeleton, bone_of) = skeleton(&glb);
        let parts = parts(&glb, &skeleton, &bone_of);
        let kinds: Vec<(&str, FacetKind)> =
            parts.iter().map(|p| (p.node.as_str(), p.kind)).collect();
        assert_eq!(
            kinds,
            [
                ("hut", FacetKind::Solid),
                ("door", FacetKind::Solid),
                ("teamcolour", FacetKind::Team),
                ("ground", FacetKind::Ground),
            ]
        );
        let door = &parts[1];
        assert_eq!(door.bone, skeleton.find("door").expect("door"));
        // The door's leaf reaches from its hinge (x 0.5) to x -0.5, in the
        // model's frame.
        let xs: Vec<f32> = door.positions.iter().map(|p| p[0]).collect();
        assert!((xs.iter().cloned().fold(f32::MAX, f32::min) + 0.5).abs() < 1e-5);
        assert!((xs.iter().cloned().fold(f32::MIN, f32::max) - 0.5).abs() < 1e-5);
        // Flat: the three corners of every triangle share one normal.
        for p in &parts {
            for t in p.indices.as_chunks::<3>().0.iter() {
                let n: Vec<[f32; 3]> = t.iter().map(|&i| p.normals[i as usize]).collect();
                assert_eq!(n[0], n[1]);
                assert_eq!(n[1], n[2]);
            }
        }
        // The ground's triangles carry their face's colour and fade.
        let (triangles, colours) = ground_triangles(&parts[3]);
        assert_eq!(triangles.len(), colours.len());
        assert!(colours.iter().any(|c| c[3] > 0.99));
        assert!(colours.iter().any(|c| c[3] < 0.3));
    }

    // spec: none (hook W30: DoorOpen turns the door about its hinge; Working moves only its node)
    #[test]
    fn clips_move_whole_objects() {
        let glb = fixture();
        let (skeleton, bone_of) = skeleton(&glb);
        let open = clip(&glb, DOOR_OPEN, &skeleton, &bone_of).expect("DoorOpen");
        assert!((open.duration - 0.4).abs() < 1e-6);
        let door = skeleton.find("door").expect("door");
        // The leaf's far edge (rest x -0.5, z 1.0) swings out by 100
        // degrees about the hinge at (0.5, 1.0): to (0.5 - cos 100, 1 + sin 100).
        let end = open.composites(&skeleton, 10.0, false);
        let p = end[door].transform_point3(Vec3::new(-0.5, 0.0, 1.0));
        let a = 100f32.to_radians();
        assert!((p.x - (0.5 - a.cos())).abs() < 1e-3, "{p}");
        assert!((p.z - (1.0 + a.sin())).abs() < 1e-3, "{p}");
        // The hinge stays; the other bones keep their rest.
        let hinge = end[door].transform_point3(Vec3::new(0.5, 0.0, 1.0));
        assert!(hinge.abs_diff_eq(Vec3::new(0.5, 0.0, 1.0), 1e-4));
        let hut = skeleton.find("hut").expect("hut");
        assert!(end[hut].abs_diff_eq(Mat4::IDENTITY, 1e-5));
        // Halfway the door is halfway (linear keys).
        let half = open.composites(&skeleton, 0.2, false);
        let p = half[door].transform_point3(Vec3::new(-0.5, 0.0, 1.0));
        let a = 50f32.to_radians();
        assert!((p.x - (0.5 - a.cos())).abs() < 1e-2, "{p}");
        let working = clip(&glb, WORKING, &skeleton, &bone_of).expect("Working");
        assert!((working.duration - 2.0).abs() < 1e-6);
        let team = skeleton.find(TEAM_NODE).expect("team");
        let swung = working.composites(&skeleton, 0.5, true);
        assert!(!swung[team].abs_diff_eq(Mat4::IDENTITY, 1e-3));
        assert!(swung[door].abs_diff_eq(Mat4::IDENTITY, 1e-5));
        // Looping: a whole clip later the same pose.
        let again = working.composites(&skeleton, 2.5, true);
        assert!(again[team].abs_diff_eq(swung[team], 1e-4));
        assert!(clip(&glb, "NoSuchClip", &skeleton, &bone_of).is_none());
    }

    // spec: none (hook W30: the embedded palette image, RGB widened to RGBA)
    #[test]
    fn reads_the_palette_image() {
        let glb = fixture();
        let (w, h, rgba) = decode_png(&glb.images[0].bytes).expect("png");
        assert_eq!((w, h, rgba.len()), (4, 4, 64));
        // Texel (1, 0): the roof's swatch.
        assert_eq!(&rgba[4..8], &[150, 70, 50, 255]);
        assert!(image(&glb.images[1].bytes).is_some());
        assert!(decode_png(b"not a png").is_err());
    }

    // spec: none (hook W30: a look adds one entry per tribe with the model, its clips, a borrowed site, its smoke)
    #[test]
    fn a_look_adds_its_entries() {
        let mut tables = ModelTables::default();
        tables.read_buildings(
            r#"<Buildings><Building name="Roman_Lumberjack" mesh="a.gr2" frame="site.gr2"/>
               <Building name="Roman_Forester" mesh="f.gr2"/></Buildings>"#,
        );
        let before = tables.clone();
        add_building_looks(&mut tables, &[]);
        assert_eq!(tables, before, "no look: the tables as read");
        let mut look = crate::extend::BuildingLook::new("ForesterHouse", "data/x/burner.glb");
        look.site_of = Some("Lumberjack".into());
        look.smoke = Some("Smoke".into());
        add_building_looks(&mut tables, &[look]);
        let e = tables.building("Roman_look:ForesterHouse").expect("entry");
        assert_eq!(e.mesh.as_deref(), Some("data/x/burner.glb"));
        assert_eq!(e.frame.as_deref(), Some("site.gr2"));
        assert_eq!(
            e.door_open.as_ref().map(|c| c.file.as_str()),
            Some("data/x/burner.glb#DoorOpen")
        );
        let working = e.working.as_ref().expect("Working");
        assert_eq!(
            split_clip(&working.file),
            Some(("data/x/burner.glb", WORKING))
        );
        assert_eq!(working.events[0].slot.as_deref(), Some(SMOKE_ANCHOR));
        assert_eq!(working.events[0].name, "Smoke");
        assert!(!working.events[0].sound);
        assert_eq!(e.heading, -30);
        // One per tribe; the install's entries stay.
        assert_eq!(
            tables
                .building("Roman_Forester")
                .and_then(|e| e.mesh.as_deref()),
            Some("f.gr2")
        );
        assert!(tables.building("African_look:ForesterHouse").is_some());
        // No African woodcutter in these tables: no site model there.
        assert_eq!(
            tables
                .building("African_look:ForesterHouse")
                .and_then(|e| e.frame.clone()),
            None
        );
        // The type named by a look takes the look's graphics name.
        let names = look_gfx_names(
            [
                (7, Some("ForesterHouse")),
                (8, Some("WoodcutterHut")),
                (9, None),
            ],
            &[crate::extend::BuildingLook::new("ForesterHouse", "b.glb")],
        );
        assert_eq!(names, [(7, "look:ForesterHouse".to_owned())]);
        assert!(look_gfx_names([(7, Some("ForesterHouse"))], &[]).is_empty());
        assert!(split_clip("data/a.gr2#x").is_none());
    }

    // spec: none (hook W30: the families: opaque and cut like a house, the ground blended as the decal)
    #[test]
    fn the_families_of_a_project_model() {
        let look = Look::family(Family::House, [0.2, 0.4, 0.6]);
        let solid = Facet {
            kind: FacetKind::Solid,
            decal_colours: Vec::new(),
        };
        let ground = Facet {
            kind: FacetKind::Ground,
            decal_colours: Vec::new(),
        };
        let mut house = look.clone();
        house.extra = [3.0, 1.5, 1.0, 0.0];
        let f = facet_look(&solid, &house);
        assert_eq!(f.family, Family::Facet);
        // The cut plane stays; no colour map column, no frame pass.
        assert_eq!(f.extra, [0.0, 1.5, 1.0, 0.0]);
        assert_eq!(f.player, look.player);
        assert_eq!(facet_look(&ground, &look).family, Family::FacetGround);
        assert_eq!(
            ground_layer_bias(Family::FacetGround as u32 as f32),
            ground_layer_bias(Family::Decal as u32 as f32)
        );
    }

    fn tree_fixture(name: &str) -> Glb {
        let path = format!(
            "{}/../../tests/models/{name}.glb",
            env!("CARGO_MANIFEST_DIR")
        );
        glb::load(&std::fs::read(&path).expect("tests/models")).expect("loads")
    }

    // spec: none (hook W30b: a tree file: one solid part at its foot, flat faces; the far version lighter, the same height)
    #[test]
    fn a_tree_file_and_its_far_version() {
        let triangles = |glb: &Glb| {
            let (skeleton, bone_of) = skeleton(glb);
            let parts = parts(glb, &skeleton, &bone_of);
            assert_eq!(parts.len(), 1);
            let part = &parts[0];
            assert_eq!(part.kind, FacetKind::Solid);
            for t in part.indices.as_chunks::<3>().0.iter() {
                let n: Vec<[f32; 3]> = t.iter().map(|&i| part.normals[i as usize]).collect();
                assert!(n[0] == n[1] && n[1] == n[2]);
            }
            let top = part.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
            let foot = part.positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
            assert!(skeleton.find("anchor_origin").is_some());
            (part.indices.len() / 3, top, foot)
        };
        for name in ["tree_round", "tree_cone"] {
            let (near, top, foot) = triangles(&tree_fixture(name));
            let (far, far_top, far_foot) = triangles(&tree_fixture(&format!("{name}_far")));
            assert!(far < near, "{name}: {far} >= {near}");
            assert_eq!((top, foot), (far_top, far_foot));
            assert!(
                foot < 0.0 && foot > -0.5,
                "{name}: the foot under the ground"
            );
        }
        // A tree's look keeps the tree family on its parts (never the
        // house's cut), its ground part stays a ground.
        let look = Look::family(Family::FacetTree, [0.0; 3]);
        let solid = Facet {
            kind: FacetKind::Solid,
            decal_colours: Vec::new(),
        };
        assert_eq!(facet_look(&solid, &look).family, Family::FacetTree);
        let ground = Facet {
            kind: FacetKind::Ground,
            decal_colours: Vec::new(),
        };
        assert_eq!(facet_look(&ground, &look).family, Family::FacetGround);
    }

    // spec: none (hook W30b: --tree-model)
    #[test]
    fn the_tree_model_option() {
        use crate::app::options::tree_look;
        let look =
            tree_look("BirchA=data/t/birch.glb,far=data/t/birch_far.glb,far-below=0.2,growth=root")
                .expect("parses");
        assert_eq!(look.tree, "BirchA");
        assert_eq!(look.model, "data/t/birch.glb");
        assert_eq!(look.far.as_deref(), Some("data/t/birch_far.glb"));
        assert_eq!(look.far_below, 0.2);
        assert_eq!(look.growth, crate::extend::TreeGrowth::RootWidth);
        assert_eq!(
            tree_look("FirA=f.glb").expect("parses"),
            crate::extend::TreeLook::new("FirA", "f.glb")
        );
        assert_eq!(
            crate::extend::TreeLook::new("FirA", "f.glb").far_below,
            crate::extend::TREE_FAR_BELOW
        );
        assert!(tree_look("FirA").is_err());
        assert!(tree_look("FirA=f.glb,far-below=-1").is_err());
        assert!(tree_look("FirA=f.glb,growth=fast").is_err());
        assert!(tree_look("FirA=f.glb,colour=red").is_err());
    }

    // spec: none (hook W30: --building-model)
    #[test]
    fn the_building_model_option() {
        let look =
            crate::app::options::building_look("Spring=data/b.glb,site=Lumberjack,smoke=Smoke")
                .expect("parses");
        assert_eq!(look.building, "Spring");
        assert_eq!(look.model, "data/b.glb");
        assert_eq!(look.site_of.as_deref(), Some("Lumberjack"));
        assert_eq!(look.smoke.as_deref(), Some("Smoke"));
        assert_eq!(
            crate::app::options::building_look("Well=w.glb").expect("parses"),
            crate::extend::BuildingLook::new("Well", "w.glb")
        );
        assert!(crate::app::options::building_look("Well").is_err());
        assert!(crate::app::options::building_look("Well=w.glb,colour=red").is_err());
    }
}
