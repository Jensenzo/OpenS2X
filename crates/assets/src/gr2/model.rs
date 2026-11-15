//! The content of a Granny file in our own types (`specs/GFX-granny-format.md`
//! GFX-GR2-4, 5). Field names are those stored in the files (GFX-GR2-3.3);
//! where a name may differ, the member is also found by its type and place.

use super::{member, Field, Gr2Error, Object, RawFile, Ref};

/// GFX-GR2-4.1: the exporting art tool's axes and units.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtToolInfo {
    pub tool: String,
    pub units_per_meter: f32,
    pub origin: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub back: [f32; 3],
}

/// GFX-GR2-5.1: a texture, referenced by its file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureRef {
    /// The artist path as stored (`Z:\...\PB_R_Sawmill01.dds`).
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

impl TextureRef {
    /// The base name of the file (GFX-GR2-5.1).
    pub fn base_name(&self) -> &str {
        self.file_name
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(&self.file_name)
    }
}

/// A material: its name and the textures it uses (directly or through its
/// maps), as indices into [`Gr2::textures`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    pub name: String,
    pub textures: Vec<usize>,
}

/// GFX-GR2-3.2 type 9: a transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub flags: u32,
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub scale_shear: [f32; 9],
}

impl Transform {
    fn read(file: &RawFile, at: Ref) -> Result<Self, Gr2Error> {
        let f = |i: u32| {
            file.f32(Ref {
                section: at.section,
                offset: at.offset + 4 + 4 * i,
            })
        };
        let mut t = Self {
            flags: file.u32(at)?,
            position: [0.0; 3],
            orientation: [0.0; 4],
            scale_shear: [0.0; 9],
        };
        for i in 0..3 {
            t.position[i] = f(i as u32)?;
        }
        for i in 0..4 {
            t.orientation[i] = f(3 + i as u32)?;
        }
        for i in 0..9 {
            t.scale_shear[i] = f(7 + i as u32)?;
        }
        Ok(t)
    }
}

/// GFX-GR2-4.3: a bone.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    /// The rest transform, relative to the parent.
    pub transform: Transform,
    pub inverse_world: [f32; 16],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    pub name: String,
    pub bones: Vec<Bone>,
}

/// GFX-GR2-4.2: one component of a vertex layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexComponent {
    pub name: String,
    /// GFX-GR2-3.2 member type.
    pub kind: u32,
    pub count: u32,
    /// Offset in the vertex.
    pub offset: u32,
}

/// GFX-GR2-4.2: the vertices of a mesh, by component.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vertices {
    pub count: usize,
    pub layout: Vec<VertexComponent>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
    /// `DiffuseColor0`, as stored (3 or 4 components).
    pub colors: Vec<Vec<f32>>,
    /// `TextureCoordinates0..2`.
    pub uvs: Vec<Vec<[f32; 2]>>,
    pub bone_weights: Vec<[u8; 4]>,
    pub bone_indices: Vec<[u8; 4]>,
}

impl Vertices {
    /// GFX-GR2-7.3: the normals scaled to unit length (a zero vector stays).
    pub fn unit_normals(&self) -> Vec<[f32; 3]> {
        self.normals
            .iter()
            .map(|&[x, y, z]| {
                let len = (x * x + y * y + z * z).sqrt();
                if len > 0.0 {
                    [x / len, y / len, z / len]
                } else {
                    [x, y, z]
                }
            })
            .collect()
    }

    /// GFX-GR2-7.3: the first three stored bytes divided by 255, the fourth
    /// weight 1 minus their sum (the fourth stored byte is not used).
    pub fn skin_weights(&self) -> Vec<[f32; 4]> {
        self.bone_weights
            .iter()
            .map(|b| {
                let w = [
                    f32::from(b[0]) / 255.0,
                    f32::from(b[1]) / 255.0,
                    f32::from(b[2]) / 255.0,
                ];
                [w[0], w[1], w[2], 1.0 - (w[0] + w[1] + w[2])]
            })
            .collect()
    }
}

/// GFX-GR2-4.2: triangles of one material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriangleGroup {
    pub material: u32,
    pub first_triangle: u32,
    pub triangle_count: u32,
}

/// GFX-GR2-4.2: a bone a mesh is bound to.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneBinding {
    pub bone: String,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub triangles: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vertices,
    /// Three per triangle.
    pub indices: Vec<u32>,
    pub groups: Vec<TriangleGroup>,
    /// The material of each binding, as an index into [`Gr2::materials`].
    pub materials: Vec<Option<usize>>,
    pub bone_bindings: Vec<BoneBinding>,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// A model: a skeleton and the meshes bound to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    pub name: String,
    pub skeleton: Option<usize>,
    pub meshes: Vec<usize>,
}

/// GFX-GR2-4.4: a curve, kept as data (no evaluation yet).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Curve {
    pub degree: u32,
    pub knots: Vec<f32>,
    pub controls: Vec<f32>,
}

impl Curve {
    /// An empty curve stands for the rest pose value.
    pub fn is_empty(&self) -> bool {
        self.knots.is_empty() && self.controls.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformTrack {
    pub name: String,
    pub position: Curve,
    pub orientation: Curve,
    pub scale_shear: Curve,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackGroup {
    pub name: String,
    pub transform_tracks: Vec<TransformTrack>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    /// The source `.max` path.
    pub name: String,
    /// Seconds.
    pub duration: f32,
    pub time_step: f32,
    pub track_groups: Vec<TrackGroup>,
}

impl Animation {
    pub fn transform_track_count(&self) -> usize {
        self.track_groups
            .iter()
            .map(|g| g.transform_tracks.len())
            .sum()
    }
}

/// GFX-GR2-4: what a file holds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gr2 {
    pub art_tool: Option<ArtToolInfo>,
    pub from_file_name: String,
    pub textures: Vec<TextureRef>,
    pub materials: Vec<Material>,
    pub skeletons: Vec<Skeleton>,
    pub meshes: Vec<Mesh>,
    pub models: Vec<ModelInfo>,
    pub animations: Vec<Animation>,
}

/// A member by name, or else the `nth` member of `kind`.
fn field<'a>(o: &Object<'a>, name: &str, kind: u32, nth: usize) -> Option<Field<'a>> {
    o.field(name).or_else(|| {
        let name = o
            .member_names()
            .zip(o.file.members(o.type_ref).ok()?)
            .filter(|(_, m)| m.kind == kind)
            .nth(nth)
            .map(|(n, _)| n.to_owned())?;
        o.field(&name)
    })
}

fn string(o: &Object, name: &str) -> String {
    field(o, name, member::STRING, 0)
        .and_then(|f| f.string())
        .unwrap_or_default()
}

fn vec3(values: &[f32]) -> [f32; 3] {
    [0, 1, 2].map(|i| values.get(i).copied().unwrap_or(0.0))
}

/// The numbers of an array-of-numbers member.
fn numbers(o: &Object, name: &str) -> Vec<f32> {
    o.field(name)
        .map(|f| f.numbers().into_iter().map(|v| v as f32).collect())
        .unwrap_or_default()
}

/// As [`numbers`], as whole numbers.
fn integers(o: &Object, name: &str) -> Vec<u32> {
    o.field(name)
        .map(|f| f.numbers().into_iter().map(|v| v as u32).collect())
        .unwrap_or_default()
}

impl Gr2 {
    /// GFX-GR2-4: read the content of an expanded file.
    pub fn from_raw(file: &RawFile) -> Result<Self, Gr2Error> {
        let root = file.root()?;
        let mut gr2 = Gr2 {
            from_file_name: root.string("FromFileName").unwrap_or_default(),
            ..Gr2::default()
        };
        if let Some(info) = root.object("ArtToolInfo") {
            gr2.art_tool = Some(ArtToolInfo {
                tool: string(&info, "FromArtToolName"),
                units_per_meter: info.f32("UnitsPerMeter").unwrap_or(1.0),
                origin: vec3(&info.f32s("Origin")),
                right: vec3(&info.f32s("RightVector")),
                up: vec3(&info.f32s("UpVector")),
                back: vec3(&info.f32s("BackVector")),
            });
        }
        let textures = root.objects("Textures");
        gr2.textures = textures
            .iter()
            .map(|t| TextureRef {
                file_name: string(t, "FromFileName"),
                width: t.int("Width").unwrap_or(0) as u32,
                height: t.int("Height").unwrap_or(0) as u32,
            })
            .collect();
        let materials = root.objects("Materials");
        gr2.materials = materials
            .iter()
            .map(|m| Material {
                name: string(m, "Name"),
                textures: material_textures(m, &textures),
            })
            .collect();
        let skeletons = root.objects("Skeletons");
        gr2.skeletons = skeletons
            .iter()
            .map(read_skeleton)
            .collect::<Result<_, _>>()?;
        let meshes = root.objects("Meshes");
        gr2.meshes = meshes
            .iter()
            .map(|m| read_mesh(m, &materials))
            .collect::<Result<_, _>>()?;
        gr2.models = root
            .objects("Models")
            .iter()
            .map(|m| ModelInfo {
                name: string(m, "Name"),
                skeleton: m
                    .object("Skeleton")
                    .and_then(|s| skeletons.iter().position(|k| k.at == s.at)),
                meshes: m
                    .objects("MeshBindings")
                    .iter()
                    .filter_map(|b| {
                        let mesh = b.object("Mesh")?;
                        meshes.iter().position(|k| k.at == mesh.at)
                    })
                    .collect(),
            })
            .collect();
        gr2.animations = root
            .objects("Animations")
            .iter()
            .map(read_animation)
            .collect();
        Ok(gr2)
    }
}

/// The textures a material uses: its own and those of its maps.
fn material_textures(m: &Object, textures: &[Object]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut add = |t: Object| {
        if let Some(i) = textures.iter().position(|k| k.at == t.at) {
            if !found.contains(&i) {
                found.push(i);
            }
        }
    };
    if let Some(t) = m.object("Texture") {
        add(t);
    }
    for map in m.objects("Maps") {
        if let Some(inner) = map.object("Map").or_else(|| map.object("Material")) {
            if let Some(t) = inner.object("Texture") {
                add(t);
            }
        }
    }
    found
}

fn read_skeleton(s: &Object) -> Result<Skeleton, Gr2Error> {
    let bones = field(s, "Bones", member::REFERENCE_TO_ARRAY, 0)
        .map(|f| f.objects())
        .unwrap_or_default();
    let mut out = Vec::with_capacity(bones.len());
    for b in &bones {
        let parent = field(b, "ParentIndex", member::INT32, 0)
            .and_then(|f| f.int())
            .unwrap_or(-1);
        let transform = match field(b, "Transform", member::TRANSFORM, 0) {
            Some(f) => Transform::read(b.file, f.at)?,
            None => return Err(Gr2Error::Missing("bone transform")),
        };
        let matrix = b
            .field("InverseWorldTransform")
            .or_else(|| b.field("InverseWorld4x4"))
            .map(|f| f.f32s())
            .unwrap_or_default();
        let mut inverse_world = [0.0; 16];
        for (i, v) in matrix.into_iter().take(16).enumerate() {
            inverse_world[i] = v;
        }
        out.push(Bone {
            name: string(b, "Name"),
            parent: usize::try_from(parent).ok(),
            transform,
            inverse_world,
        });
    }
    Ok(Skeleton {
        name: string(s, "Name"),
        bones: out,
    })
}

/// GFX-GR2-4.2: the vertex layout (the variant's type) and the vertices.
fn read_vertices(file: &RawFile, ty: Ref, count: u32, data: Ref) -> Result<Vertices, Gr2Error> {
    let mut layout = Vec::new();
    let mut offset = 0;
    for m in file.members(ty)? {
        let size = file.member_size(&m)?;
        layout.push(VertexComponent {
            name: m.name.clone(),
            kind: m.kind,
            count: m.array_count.max(1),
            offset,
        });
        offset += size;
    }
    let stride = offset;
    let bytes = file
        .sections
        .get(data.section as usize)
        .and_then(|s| s.get(data.offset as usize..(data.offset + count * stride) as usize))
        .ok_or(Gr2Error::BadReference(data))?;
    let mut v = Vertices {
        count: count as usize,
        ..Vertices::default()
    };
    let floats = |c: &VertexComponent, i: usize| -> Vec<f32> {
        let base = i * stride as usize + c.offset as usize;
        (0..c.count as usize)
            .map(|k| match c.kind {
                member::REAL32 => {
                    let at = base + 4 * k;
                    f32::from_le_bytes(bytes[at..at + 4].try_into().expect("4"))
                }
                member::UINT8 | member::NORMAL_UINT8 => f32::from(bytes[base + k]),
                member::REAL16 => half_to_f32(u16::from_le_bytes([
                    bytes[base + 2 * k],
                    bytes[base + 2 * k + 1],
                ])),
                _ => 0.0,
            })
            .collect()
    };
    let bytes4 = |c: &VertexComponent, i: usize| -> [u8; 4] {
        let base = i * stride as usize + c.offset as usize;
        [0, 1, 2, 3].map(|k| {
            if k < c.count as usize && matches!(c.kind, member::UINT8 | member::NORMAL_UINT8) {
                bytes[base + k]
            } else {
                0
            }
        })
    };
    for c in &layout {
        let n = count as usize;
        match c.name.as_str() {
            "Position" => v.positions = (0..n).map(|i| vec3(&floats(c, i))).collect(),
            "Normal" => v.normals = (0..n).map(|i| vec3(&floats(c, i))).collect(),
            "Tangent" => v.tangents = (0..n).map(|i| vec3(&floats(c, i))).collect(),
            "DiffuseColor0" => v.colors = (0..n).map(|i| floats(c, i)).collect(),
            "BoneWeights" => v.bone_weights = (0..n).map(|i| bytes4(c, i)).collect(),
            "BoneIndices" => v.bone_indices = (0..n).map(|i| bytes4(c, i)).collect(),
            name if name.starts_with("TextureCoordinates") => {
                let uv = (0..n)
                    .map(|i| {
                        let f = floats(c, i);
                        [
                            f.first().copied().unwrap_or(0.0),
                            f.get(1).copied().unwrap_or(0.0),
                        ]
                    })
                    .collect();
                v.uvs.push(uv);
            }
            _ => {}
        }
    }
    v.layout = layout;
    Ok(v)
}

/// IEEE half to f32.
fn half_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((h >> 10) & 0x1F);
    let frac = f32::from(h & 0x3FF);
    match exp {
        0 => sign * frac * 2f32.powi(-24),
        31 => sign * f32::INFINITY,
        _ => sign * (1.0 + frac / 1024.0) * 2f32.powi(exp - 15),
    }
}

fn read_mesh(m: &Object, materials: &[Object]) -> Result<Mesh, Gr2Error> {
    let file = m.file;
    let mut vertices = Vertices::default();
    if let Some(vd) = m.object("PrimaryVertexData") {
        let array = field(&vd, "Vertices", member::REFERENCE_TO_VARIANT_ARRAY, 0)
            .and_then(|f| f.variant_array());
        if let Some((ty, count, data)) = array {
            vertices = read_vertices(file, ty, count, data)?;
        }
    }
    let mut indices = Vec::new();
    let mut groups = Vec::new();
    if let Some(top) = m.object("PrimaryTopology") {
        groups = top
            .objects("Groups")
            .iter()
            .map(|g| {
                // By name, or else the group's three numbers in order.
                let ints: Vec<u32> = g
                    .member_names()
                    .filter_map(|n| g.int(n))
                    .map(|v| v as u32)
                    .collect();
                let nth = |i: usize| ints.get(i).copied().unwrap_or(0);
                TriangleGroup {
                    material: g.int("MaterialIndex").map_or(nth(0), |v| v as u32),
                    first_triangle: g.int("TriFirst").map_or(nth(1), |v| v as u32),
                    triangle_count: g.int("TriCount").map_or(nth(2), |v| v as u32),
                }
            })
            .collect();
        indices = integers(&top, "Indices");
        if indices.is_empty() {
            indices = integers(&top, "Indices16");
        }
    }
    let material_bindings = m.objects("MaterialBindings");
    let mesh_materials = material_bindings
        .iter()
        .map(|b| {
            let mat = b.object("Material")?;
            materials.iter().position(|k| k.at == mat.at)
        })
        .collect();
    let bone_bindings = m
        .objects("BoneBindings")
        .iter()
        .map(|b| BoneBinding {
            bone: string(b, "BoneName"),
            min: vec3(&b.f32s("OBBMin")),
            max: vec3(&b.f32s("OBBMax")),
            triangles: integers(b, "TriangleIndices"),
        })
        .collect();
    Ok(Mesh {
        name: string(m, "Name"),
        vertices,
        indices,
        groups,
        materials: mesh_materials,
        bone_bindings,
    })
}

fn read_curve(o: Option<Object>) -> Curve {
    let Some(o) = o else {
        return Curve::default();
    };
    Curve {
        degree: o.int("Degree").unwrap_or(0) as u32,
        knots: numbers(&o, "Knots"),
        controls: numbers(&o, "Controls"),
    }
}

fn read_animation(a: &Object) -> Animation {
    let track_groups = a
        .objects("TrackGroups")
        .iter()
        .map(|g| TrackGroup {
            name: string(g, "Name"),
            transform_tracks: g
                .objects("TransformTracks")
                .iter()
                .map(|t| TransformTrack {
                    name: string(t, "Name"),
                    position: read_curve(t.object("PositionCurve")),
                    orientation: read_curve(t.object("OrientationCurve")),
                    scale_shear: read_curve(t.object("ScaleShearCurve")),
                })
                .collect(),
        })
        .collect();
    Animation {
        name: string(a, "Name"),
        duration: a.f32("Duration").unwrap_or(0.0),
        time_step: a.f32("TimeStep").unwrap_or(0.0),
        track_groups,
    }
}

/// GFX-GR2: read a file as found on disk (rc00-wrapped or not, FMT-RC-1.2).
pub fn load(path: &str, file: &[u8]) -> Result<Gr2, Box<dyn std::error::Error>> {
    let data = if crate::rc00::is_wrapped(file) {
        crate::rc00::decode(path, file)?
    } else {
        file.to_vec()
    };
    let raw = super::read_raw(&data)?;
    Ok(Gr2::from_raw(&raw)?)
}

#[cfg(test)]
mod vertex_tests {
    use super::Vertices;

    /// GFX-GR2-7.3
    #[test]
    fn skin_weights_and_unit_normals() {
        let v = Vertices {
            count: 1,
            normals: vec![[0.0, 3.0, 4.0]],
            bone_weights: vec![[51, 102, 0, 77]],
            ..Vertices::default()
        };
        assert_eq!(v.unit_normals(), [[0.0, 0.6, 0.8]]);
        let w = v.skin_weights()[0];
        assert_eq!(w[..3], [0.2, 0.4, 0.0]);
        assert!((w[3] - 0.4).abs() < 1e-6);
    }
}
