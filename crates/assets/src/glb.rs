//! A project's own model as binary glTF 2.0 (`.glb`), hook W30: its node
//! tree, the triangles of each mesh with their normals, texture
//! coordinates and vertex colours, the embedded images and the node
//! animations (translation, rotation, scale). Not an original file format:
//! the original's models are Granny files (`gr2`). What a model must hold
//! for the engine to draw it is in `docs/extending.md` ("A building's own
//! model").
//!
//! Read as the glTF 2.0 specification lays it out: a 12-byte header, a JSON
//! chunk and one binary chunk; accessors of floats or of (normalized)
//! integers; no sparse accessors, no external buffers, no skins or morph
//! targets (a skin is ignored; the engine moves whole objects).

use crate::json::{self, Value};

/// One node of the tree (an object or an empty point of the model).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub mesh: Option<usize>,
    pub translation: [f32; 3],
    /// x, y, z, w.
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// One draw of a mesh: triangles in the node's own frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Primitive {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// `COLOR_0` as linear RGBA (white where the file has none).
    pub colours: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub material: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>,
}

/// A material: the images (indices into [`Glb::images`]) of its base
/// colour and of its roughness (green) and metallic (blue).
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    pub base_colour: Option<usize>,
    pub metallic_roughness: Option<usize>,
}

/// An embedded image: its bytes as stored (PNG or JPEG).
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// What a channel animates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Path {
    Translation,
    Rotation,
    Scale,
}

/// One animated property of one node: key times (seconds) and values
/// (3 or 4 numbers per key).
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub node: usize,
    pub path: Path,
    pub times: Vec<f32>,
    pub values: Vec<f32>,
    /// `STEP` interpolation (else linear; a cubic spline keeps its values
    /// and drops its tangents).
    pub step: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub name: String,
    pub channels: Vec<Channel>,
}

impl Animation {
    /// The last key time of its channels.
    pub fn duration(&self) -> f32 {
        self.channels
            .iter()
            .filter_map(|c| c.times.last().copied())
            .fold(0.0, f32::max)
    }
}

/// A whole `.glb` file.
#[derive(Debug, Clone, PartialEq)]
pub struct Glb {
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub images: Vec<Image>,
    pub animations: Vec<Animation>,
}

impl Glb {
    /// The nodes with parents before their children (depth first, roots in
    /// file order).
    pub fn ordered_nodes(&self) -> Vec<usize> {
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        let mut roots = Vec::new();
        for (i, n) in self.nodes.iter().enumerate() {
            match n.parent {
                Some(p) => children[p].push(i),
                None => roots.push(i),
            }
        }
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<usize> = roots.into_iter().rev().collect();
        while let Some(i) = stack.pop() {
            out.push(i);
            stack.extend(children[i].iter().rev());
        }
        out
    }

    /// The node of a name.
    pub fn node(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    /// The animation of a name.
    pub fn animation(&self, name: &str) -> Option<&Animation> {
        self.animations.iter().find(|a| a.name == name)
    }
}

const MAGIC: u32 = 0x4654_6C67; // "glTF"
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// Read a `.glb` file.
pub fn load(data: &[u8]) -> Result<Glb, String> {
    if u32_at(data, 0) != Some(MAGIC) {
        return Err("not a binary glTF file".into());
    }
    if u32_at(data, 4) != Some(2) {
        return Err("glTF version is not 2".into());
    }
    let mut at = 12;
    let mut json_text = None;
    let mut bin: &[u8] = &[];
    while at + 8 <= data.len() {
        let len = u32_at(data, at).ok_or("chunk")? as usize;
        let kind = u32_at(data, at + 4).ok_or("chunk")?;
        let body = data
            .get(at + 8..at + 8 + len)
            .ok_or("a chunk runs past the end of the file")?;
        match kind {
            CHUNK_JSON => json_text = Some(std::str::from_utf8(body).map_err(|e| e.to_string())?),
            CHUNK_BIN if bin.is_empty() => bin = body,
            _ => {}
        }
        at += 8 + len.next_multiple_of(4);
    }
    let root = json::parse(json_text.ok_or("no JSON chunk")?)?;
    Reader { root: &root, bin }.read()
}

struct Reader<'a> {
    root: &'a Value,
    bin: &'a [u8],
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).map_or(&[], Value::as_array)
}

fn name_of(v: &Value) -> String {
    v.get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn fixed<const N: usize>(v: Option<&Value>, default: [f32; N]) -> Result<[f32; N], String> {
    match v {
        None => Ok(default),
        Some(v) => v
            .as_f32s()
            .and_then(|f| f.try_into().ok())
            .ok_or_else(|| format!("expected {N} numbers")),
    }
}

impl Reader<'_> {
    fn read(&self) -> Result<Glb, String> {
        let mut nodes = Vec::new();
        for v in list(self.root, "nodes") {
            if v.get("matrix").is_some() {
                return Err(format!(
                    "node {}: a matrix in place of translation, rotation and scale",
                    name_of(v)
                ));
            }
            nodes.push(Node {
                name: name_of(v),
                parent: None,
                mesh: v.get("mesh").and_then(Value::as_usize),
                translation: fixed(v.get("translation"), [0.0; 3])?,
                rotation: fixed(v.get("rotation"), [0.0, 0.0, 0.0, 1.0])?,
                scale: fixed(v.get("scale"), [1.0; 3])?,
            });
        }
        for (i, v) in list(self.root, "nodes").iter().enumerate() {
            for c in list(v, "children") {
                let c = c.as_usize().filter(|&c| c < nodes.len() && c != i);
                let Some(c) = c else {
                    return Err(format!("node {i}: a bad child"));
                };
                if nodes[c].parent.is_some() {
                    return Err(format!("node {c}: two parents"));
                }
                nodes[c].parent = Some(i);
            }
        }
        // A cycle leaves nodes out of the ordered list.
        let glb_nodes = Glb {
            nodes: nodes.clone(),
            meshes: Vec::new(),
            materials: Vec::new(),
            images: Vec::new(),
            animations: Vec::new(),
        };
        if glb_nodes.ordered_nodes().len() != nodes.len() {
            return Err("the node tree has a cycle".into());
        }
        let mut meshes = Vec::new();
        for v in list(self.root, "meshes") {
            let mut primitives = Vec::new();
            for p in list(v, "primitives") {
                primitives.push(self.primitive(p)?);
            }
            meshes.push(Mesh {
                name: name_of(v),
                primitives,
            });
        }
        if let Some(bad) = nodes
            .iter()
            .find(|n| n.mesh.is_some_and(|m| m >= meshes.len()))
        {
            return Err(format!("node {}: no such mesh", bad.name));
        }
        let textures = list(self.root, "textures");
        let image_of = |t: Option<&Value>| -> Option<usize> {
            let index = t?.get("index")?.as_usize()?;
            textures.get(index)?.get("source")?.as_usize()
        };
        let materials = list(self.root, "materials")
            .iter()
            .map(|m| {
                let pbr = m.get("pbrMetallicRoughness");
                Material {
                    name: name_of(m),
                    base_colour: image_of(pbr.and_then(|p| p.get("baseColorTexture"))),
                    metallic_roughness: image_of(
                        pbr.and_then(|p| p.get("metallicRoughnessTexture")),
                    ),
                }
            })
            .collect();
        let mut images = Vec::new();
        for v in list(self.root, "images") {
            let view = v
                .get("bufferView")
                .and_then(Value::as_usize)
                .ok_or("an image outside the file (only embedded images are read)")?;
            images.push(Image {
                name: name_of(v),
                mime: v
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                bytes: self.view(view)?.0.to_vec(),
            });
        }
        let mut animations = Vec::new();
        for a in list(self.root, "animations") {
            let samplers = list(a, "samplers");
            let mut channels = Vec::new();
            for c in list(a, "channels") {
                let target = c.get("target").ok_or("a channel without target")?;
                let Some(node) = target.get("node").and_then(Value::as_usize) else {
                    continue;
                };
                if node >= nodes.len() {
                    return Err(format!("animation {}: no node {node}", name_of(a)));
                }
                let (path, width) = match target.get("path").and_then(Value::as_str) {
                    Some("translation") => (Path::Translation, 3),
                    Some("rotation") => (Path::Rotation, 4),
                    Some("scale") => (Path::Scale, 3),
                    // Morph weights and anything else: not read.
                    _ => continue,
                };
                let s = c
                    .get("sampler")
                    .and_then(Value::as_usize)
                    .and_then(|s| samplers.get(s))
                    .ok_or("a channel without sampler")?;
                let interpolation = s
                    .get("interpolation")
                    .and_then(Value::as_str)
                    .unwrap_or("LINEAR");
                let (times, n) = self.floats(s.get("input").and_then(Value::as_usize))?;
                let (mut values, m) = self.floats(s.get("output").and_then(Value::as_usize))?;
                if n != 1 || m != width {
                    return Err(format!("animation {}: bad sampler", name_of(a)));
                }
                if interpolation == "CUBICSPLINE" {
                    // In-tangent, value, out-tangent per key: the values.
                    values = values
                        .chunks(width * 3)
                        .flat_map(|k| k[width..width * 2].to_vec())
                        .collect();
                }
                if values.len() != times.len() * width {
                    return Err(format!("animation {}: keys and values differ", name_of(a)));
                }
                channels.push(Channel {
                    node,
                    path,
                    times,
                    values,
                    step: interpolation == "STEP",
                });
            }
            animations.push(Animation {
                name: name_of(a),
                channels,
            });
        }
        Ok(Glb {
            nodes,
            meshes,
            materials,
            images,
            animations,
        })
    }

    /// A buffer view's bytes and stride.
    fn view(&self, index: usize) -> Result<(&[u8], Option<usize>), String> {
        let v = list(self.root, "bufferViews")
            .get(index)
            .ok_or_else(|| format!("no buffer view {index}"))?;
        if v.get("buffer").and_then(Value::as_usize) != Some(0) {
            return Err("a buffer outside the file".into());
        }
        let offset = v.get("byteOffset").and_then(Value::as_usize).unwrap_or(0);
        let len = v
            .get("byteLength")
            .and_then(Value::as_usize)
            .ok_or("a buffer view without length")?;
        let bytes = self
            .bin
            .get(offset..offset.checked_add(len).ok_or("view")?)
            .ok_or("a buffer view runs past the binary chunk")?;
        Ok((bytes, v.get("byteStride").and_then(Value::as_usize)))
    }

    /// An accessor's values as floats (normalized integers scaled to 0..1)
    /// and its component count.
    fn floats(&self, index: Option<usize>) -> Result<(Vec<f32>, usize), String> {
        let (values, n, _) = self.accessor(index.ok_or("an accessor is missing")?)?;
        Ok((values, n))
    }

    fn accessor(&self, index: usize) -> Result<(Vec<f32>, usize, Vec<u32>), String> {
        let a = list(self.root, "accessors")
            .get(index)
            .ok_or_else(|| format!("no accessor {index}"))?;
        if a.get("sparse").is_some() {
            return Err("a sparse accessor".into());
        }
        let count = a.get("count").and_then(Value::as_usize).ok_or("count")?;
        let n = match a.get("type").and_then(Value::as_str) {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            other => return Err(format!("accessor type {other:?}")),
        };
        let kind = a
            .get("componentType")
            .and_then(Value::as_usize)
            .ok_or("componentType")?;
        let size = match kind {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            other => return Err(format!("component type {other}")),
        };
        let normalized = a
            .get("normalized")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let Some(view) = a.get("bufferView").and_then(Value::as_usize) else {
            // No view: all zeros.
            return Ok((vec![0.0; count * n], n, vec![0; count * n]));
        };
        let (bytes, stride) = self.view(view)?;
        let offset = a.get("byteOffset").and_then(Value::as_usize).unwrap_or(0);
        let stride = stride.unwrap_or(size * n);
        let mut floats = Vec::with_capacity(count * n);
        let mut ints = Vec::with_capacity(count * n);
        for i in 0..count {
            for c in 0..n {
                let at = offset + i * stride + c * size;
                let b = bytes
                    .get(at..at + size)
                    .ok_or("an accessor runs past its buffer view")?;
                let (f, int) = match kind {
                    5120 => {
                        let v = b[0] as i8;
                        (
                            if normalized {
                                (f32::from(v) / 127.0).max(-1.0)
                            } else {
                                f32::from(v)
                            },
                            0,
                        )
                    }
                    5121 => {
                        let v = b[0];
                        (
                            if normalized {
                                f32::from(v) / 255.0
                            } else {
                                f32::from(v)
                            },
                            u32::from(v),
                        )
                    }
                    5122 => {
                        let v = i16::from_le_bytes([b[0], b[1]]);
                        (
                            if normalized {
                                (f32::from(v) / 32767.0).max(-1.0)
                            } else {
                                f32::from(v)
                            },
                            0,
                        )
                    }
                    5123 => {
                        let v = u16::from_le_bytes([b[0], b[1]]);
                        (
                            if normalized {
                                f32::from(v) / 65535.0
                            } else {
                                f32::from(v)
                            },
                            u32::from(v),
                        )
                    }
                    5125 => {
                        let v = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                        (v as f32, v)
                    }
                    _ => (f32::from_le_bytes([b[0], b[1], b[2], b[3]]), 0),
                };
                floats.push(f);
                ints.push(int);
            }
        }
        Ok((floats, n, ints))
    }

    fn primitive(&self, p: &Value) -> Result<Primitive, String> {
        if p.get("mode").and_then(Value::as_usize).unwrap_or(4) != 4 {
            return Err("a primitive that is not a triangle list".into());
        }
        let attributes = p
            .get("attributes")
            .ok_or("a primitive without attributes")?;
        let attribute = |name: &str| attributes.get(name).and_then(Value::as_usize);
        let (positions, n) = self.floats(attribute("POSITION"))?;
        if n != 3 {
            return Err("POSITION is not VEC3".into());
        }
        let count = positions.len() / 3;
        let vec3 = |f: Vec<f32>| -> Vec<[f32; 3]> {
            f.as_chunks::<3>()
                .0
                .iter()
                .map(|c| [c[0], c[1], c[2]])
                .collect()
        };
        let normals = match attribute("NORMAL") {
            Some(i) => vec3(self.floats(Some(i))?.0),
            None => Vec::new(),
        };
        let uvs = match attribute("TEXCOORD_0") {
            Some(i) => self
                .floats(Some(i))?
                .0
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| [c[0], c[1]])
                .collect(),
            None => vec![[0.0; 2]; count],
        };
        let colours = match attribute("COLOR_0") {
            Some(i) => {
                let (f, n) = self.floats(Some(i))?;
                f.chunks_exact(n)
                    .map(|c| [c[0], c[1], c[2], if n == 4 { c[3] } else { 1.0 }])
                    .collect()
            }
            None => vec![[1.0; 4]; count],
        };
        let indices = match p.get("indices").and_then(Value::as_usize) {
            Some(i) => self.accessor(i)?.2,
            None => (0..count as u32).collect(),
        };
        if indices.iter().any(|&i| i as usize >= count)
            || [normals.len(), uvs.len(), colours.len()]
                .iter()
                .any(|&l| l != count && l != 0)
        {
            return Err("a primitive's attributes or indices do not match".into());
        }
        Ok(Primitive {
            positions: vec3(positions),
            normals,
            uvs,
            colours,
            indices,
            material: p.get("material").and_then(Value::as_usize),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test model of the repository (`tests/models/facets.glb`, made by
    /// `scripts/make-test-glb.py`).
    pub(crate) fn fixture() -> Vec<u8> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/models/facets.glb");
        std::fs::read(path).expect("tests/models/facets.glb")
    }

    // spec: none (hook W30: a project's .glb: nodes, meshes, vertex colours, images, node clips)
    #[test]
    fn reads_the_test_model() {
        let glb = load(&fixture()).expect("loads");
        let names: Vec<&str> = glb.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "hut",
                "door",
                "teamcolour",
                "ground",
                "anchor_smoke",
                "anchor_door"
            ]
        );
        let door = &glb.nodes[glb.node("door").expect("door")];
        assert_eq!(door.translation, [0.5, 0.0, 1.0]);
        assert_eq!(door.parent, None);
        let hut = &glb.meshes[glb.nodes[0].mesh.expect("a mesh")].primitives[0];
        assert_eq!(hut.positions.len(), hut.normals.len());
        assert_eq!(hut.indices.len() % 3, 0);
        // Occlusion in the vertex colour, one value per face.
        assert!(hut.colours.iter().all(|c| (0.59..=1.0).contains(&c[0])));
        let ground = &glb.meshes[glb.nodes[3].mesh.expect("ground")].primitives[0];
        assert!(ground.colours.iter().any(|c| c[3] < 0.5));
        assert_eq!(glb.images.len(), 2);
        assert_eq!(glb.images[0].mime, "image/png");
        let m = &glb.materials[hut.material.expect("material")];
        assert_eq!((m.base_colour, m.metallic_roughness), (Some(0), Some(1)));
        let open = glb.animation("DoorOpen").expect("DoorOpen");
        assert_eq!(open.channels.len(), 1);
        assert_eq!(open.channels[0].path, Path::Rotation);
        assert!((open.duration() - 0.4).abs() < 1e-6);
        assert!(glb.animation("Working").is_some());
    }

    // spec: none (hook W30: a broken file is refused, not drawn)
    #[test]
    fn refuses_what_is_not_a_glb() {
        assert!(load(b"").is_err());
        assert!(load(b"glTF\x01\0\0\0\x0c\0\0\0").is_err());
        let mut data = fixture();
        data.truncate(data.len() - 40);
        assert!(load(&data).is_err());
    }
}
