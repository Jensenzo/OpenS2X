//! Writing small uncompressed Granny files for tests (GFX-GR2-1, 3): the
//! container allows sections stored without compression (GFX-GR2-1.4).

use super::{member, Ref, MAGIC};

/// A member of a type being written: kind, name, sub-type, array count.
pub type MemberDef = (u32, &'static str, Option<Ref>, u32);

/// A value of a member, in the order of the type's members.
pub enum Val {
    I32(i32),
    I16(i16),
    F32s(Vec<f32>),
    Str(&'static str),
    /// Type 2: a pointer (or null).
    Ptr(Option<Ref>),
    /// Type 1: an inline struct of the member's sub-type.
    Inline(Vec<Val>),
    /// Type 3: elements of the member's sub-type, written in a row.
    Array(Vec<Vec<Val>>),
    /// Type 4: pointers to objects.
    Refs(Vec<Ref>),
    /// Type 7: element type, count, data.
    VariantArray(Ref, u32, Ref),
    /// Type 9: flags, position, orientation, scale/shear.
    Transform(u32, [f32; 3], [f32; 4], [f32; 9]),
}

pub struct Builder {
    sections: Vec<Vec<u8>>,
    fixups: Vec<Vec<(u32, u32, u32)>>,
    /// Members of each written type.
    types: Vec<(Ref, Vec<MemberDef>)>,
}

/// Sections used: 0 strings and objects, 1 types, 2 bulk data.
const OBJECTS: u32 = 0;
const TYPES: u32 = 1;
const BULK: u32 = 2;

impl Builder {
    pub fn new() -> Self {
        Self {
            sections: vec![Vec::new(); 8],
            fixups: vec![Vec::new(); 8],
            types: Vec::new(),
        }
    }

    fn append(&mut self, section: u32, bytes: &[u8]) -> Ref {
        let s = &mut self.sections[section as usize];
        while !s.len().is_multiple_of(4) {
            s.push(0);
        }
        let offset = s.len() as u32;
        s.extend_from_slice(bytes);
        Ref { section, offset }
    }

    fn point(&mut self, slot: Ref, target: Ref) {
        self.fixups[slot.section as usize].push((slot.offset, target.section, target.offset));
    }

    fn put_u32(&mut self, at: Ref, value: u32) {
        let s = &mut self.sections[at.section as usize];
        s[at.offset as usize..at.offset as usize + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn string(&mut self, text: &str) -> Ref {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        self.append(OBJECTS, &bytes)
    }

    /// Raw data (vertices) in the bulk section.
    pub fn bulk(&mut self, bytes: &[u8]) -> Ref {
        self.append(BULK, bytes)
    }

    /// GFX-GR2-3.1: a type definition.
    pub fn typedef(&mut self, members: &[MemberDef]) -> Ref {
        let at = self.append(TYPES, &vec![0; 32 * (members.len() + 1)]);
        for (i, &(kind, name, sub, count)) in members.iter().enumerate() {
            let record = Ref {
                section: TYPES,
                offset: at.offset + 32 * i as u32,
            };
            self.put_u32(record, kind);
            let name = self.string(name);
            self.point(
                Ref {
                    offset: record.offset + 4,
                    ..record
                },
                name,
            );
            if let Some(sub) = sub {
                self.point(
                    Ref {
                        offset: record.offset + 8,
                        ..record
                    },
                    sub,
                );
            }
            self.put_u32(
                Ref {
                    offset: record.offset + 12,
                    ..record
                },
                count,
            );
        }
        self.types.push((at, members.to_vec()));
        at
    }

    fn members_of(&self, ty: Ref) -> Vec<MemberDef> {
        self.types
            .iter()
            .find(|(r, _)| *r == ty)
            .map(|(_, m)| m.clone())
            .expect("a written type")
    }

    fn size_of(&self, m: &MemberDef) -> u32 {
        let one = match m.0 {
            member::INLINE => self
                .members_of(m.2.expect("sub-type"))
                .iter()
                .map(|x| self.size_of(x))
                .sum(),
            member::REFERENCE_TO_ARRAY | member::ARRAY_OF_REFERENCES => 8,
            member::REFERENCE_TO_VARIANT_ARRAY => 12,
            member::TRANSFORM => 68,
            member::INT16 | member::UINT16 => 2,
            member::UINT8 | member::INT8 => 1,
            _ => 4,
        };
        one * m.3.max(1)
    }

    fn type_size(&self, ty: Ref) -> u32 {
        self.members_of(ty).iter().map(|m| self.size_of(m)).sum()
    }

    /// Write one object of type `ty` with `values`; returns where it is.
    pub fn object(&mut self, ty: Ref, values: Vec<Val>) -> Ref {
        let size = self.type_size(ty);
        let at = self.append(OBJECTS, &vec![0; size as usize]);
        self.fill(at, ty, values);
        at
    }

    fn fill(&mut self, at: Ref, ty: Ref, values: Vec<Val>) {
        let members = self.members_of(ty);
        let mut offset = at.offset;
        for (m, v) in members.iter().zip(values) {
            let slot = Ref {
                section: at.section,
                offset,
            };
            let next = |k: u32| Ref {
                section: at.section,
                offset: offset + k,
            };
            match v {
                Val::I32(x) => self.put_u32(slot, x as u32),
                Val::I16(x) => {
                    let s = &mut self.sections[slot.section as usize];
                    s[slot.offset as usize..slot.offset as usize + 2]
                        .copy_from_slice(&x.to_le_bytes());
                }
                Val::F32s(xs) => {
                    for (i, x) in xs.iter().enumerate() {
                        self.put_u32(next(4 * i as u32), x.to_bits());
                    }
                }
                Val::Str(s) => {
                    let s = self.string(s);
                    self.point(slot, s);
                }
                Val::Ptr(Some(p)) => self.point(slot, p),
                Val::Ptr(None) => {}
                Val::Inline(vs) => self.fill(slot, m.2.expect("sub-type"), vs),
                Val::Array(elements) => {
                    let sub = m.2.expect("sub-type");
                    let size = self.type_size(sub);
                    self.put_u32(slot, elements.len() as u32);
                    if !elements.is_empty() {
                        let data =
                            self.append(OBJECTS, &vec![0; (size * elements.len() as u32) as usize]);
                        for (i, e) in elements.into_iter().enumerate() {
                            let el = Ref {
                                section: data.section,
                                offset: data.offset + size * i as u32,
                            };
                            self.fill(el, sub, e);
                        }
                        self.point(next(4), data);
                    }
                }
                Val::Refs(refs) => {
                    self.put_u32(slot, refs.len() as u32);
                    if !refs.is_empty() {
                        let data = self.append(OBJECTS, &vec![0; 4 * refs.len()]);
                        for (i, r) in refs.into_iter().enumerate() {
                            self.point(
                                Ref {
                                    section: data.section,
                                    offset: data.offset + 4 * i as u32,
                                },
                                r,
                            );
                        }
                        self.point(next(4), data);
                    }
                }
                Val::VariantArray(ty, count, data) => {
                    self.point(slot, ty);
                    self.put_u32(next(4), count);
                    self.point(next(8), data);
                }
                Val::Transform(flags, p, o, s) => {
                    self.put_u32(slot, flags);
                    let all: Vec<f32> = p.iter().chain(&o).chain(&s).copied().collect();
                    for (i, x) in all.iter().enumerate() {
                        self.put_u32(next(4 + 4 * i as u32), x.to_bits());
                    }
                }
            }
            offset += self.size_of(m);
        }
    }

    /// GFX-GR2-1: the whole file, every section stored uncompressed.
    pub fn finish(self, root_type: Ref, root_object: Ref) -> Vec<u8> {
        let header_size = 0x1B8u32;
        let count = self.sections.len();
        let table = 32 + 72;
        let mut out = vec![0u8; header_size as usize];
        out[..16].copy_from_slice(&MAGIC);
        out[16..20].copy_from_slice(&header_size.to_le_bytes());
        let put = |out: &mut Vec<u8>, at: usize, v: u32| {
            out[at..at + 4].copy_from_slice(&v.to_le_bytes())
        };
        put(&mut out, 32, 6);
        put(&mut out, 32 + 12, (table - 32) as u32);
        put(&mut out, 32 + 16, count as u32);
        put(&mut out, 32 + 20, root_type.section);
        put(&mut out, 32 + 24, root_type.offset);
        put(&mut out, 32 + 28, root_object.section);
        put(&mut out, 32 + 32, root_object.offset);
        put(&mut out, 32 + 36, 0x8000_0013);
        let mut records = Vec::new();
        for (i, data) in self.sections.iter().enumerate() {
            let data_offset = out.len() as u32;
            out.extend_from_slice(data);
            let fixup_offset = out.len() as u32;
            for &(s, ts, to) in &self.fixups[i] {
                for v in [s, ts, to] {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
            let len = data.len() as u32;
            records.push([
                0,
                data_offset,
                len,
                len,
                4,
                len,
                len,
                fixup_offset,
                self.fixups[i].len() as u32,
                out.len() as u32,
                0,
            ]);
        }
        for (i, r) in records.iter().enumerate() {
            for (k, v) in r.iter().enumerate() {
                put(&mut out, table + 44 * i + 4 * k, *v);
            }
        }
        let total = out.len() as u32;
        put(&mut out, 32 + 4, total);
        out
    }
}

/// A textured cube of side 2 around the origin with a one-bone skeleton and
/// a one-track animation, using the texture `texture`.
pub fn cube(texture: &'static str) -> Vec<u8> {
    use member::*;
    use std::f32::consts::FRAC_1_SQRT_2;
    let mut b = Builder::new();
    let int = b.typedef(&[(INT32, "Int32", None, 0)]);
    let int16 = b.typedef(&[(INT16, "Int16", None, 0)]);
    let real = b.typedef(&[(REAL32, "Real32", None, 0)]);
    let art = b.typedef(&[
        (STRING, "FromArtToolName", None, 0),
        (REAL32, "UnitsPerMeter", None, 0),
        (REAL32, "Origin", None, 3),
        (REAL32, "RightVector", None, 3),
        (REAL32, "UpVector", None, 3),
        (REAL32, "BackVector", None, 3),
    ]);
    let tex = b.typedef(&[
        (STRING, "FromFileName", None, 0),
        (INT32, "Width", None, 0),
        (INT32, "Height", None, 0),
    ]);
    let mat = b.typedef(&[
        (STRING, "Name", None, 0),
        (REFERENCE, "Texture", Some(tex), 0),
    ]);
    let bone = b.typedef(&[
        (STRING, "Name", None, 0),
        (INT32, "ParentIndex", None, 0),
        (TRANSFORM, "Transform", None, 0),
        (REAL32, "InverseWorldTransform", None, 16),
    ]);
    let skel = b.typedef(&[
        (STRING, "Name", None, 0),
        (REFERENCE_TO_ARRAY, "Bones", Some(bone), 0),
    ]);
    let vertex = b.typedef(&[
        (REAL32, "Position", None, 3),
        (REAL32, "Normal", None, 3),
        (REAL32, "TextureCoordinates0", None, 2),
    ]);
    let vdata = b.typedef(&[(REFERENCE_TO_VARIANT_ARRAY, "Vertices", None, 0)]);
    let group = b.typedef(&[
        (INT32, "MaterialIndex", None, 0),
        (INT32, "TriFirst", None, 0),
        (INT32, "TriCount", None, 0),
    ]);
    let topo = b.typedef(&[
        (REFERENCE_TO_ARRAY, "Groups", Some(group), 0),
        (REFERENCE_TO_ARRAY, "Indices", Some(int), 0),
        (REFERENCE_TO_ARRAY, "Indices16", Some(int16), 0),
    ]);
    let binding = b.typedef(&[(REFERENCE, "Material", Some(mat), 0)]);
    let bone_binding = b.typedef(&[
        (STRING, "BoneName", None, 0),
        (REAL32, "OBBMin", None, 3),
        (REAL32, "OBBMax", None, 3),
        (REFERENCE_TO_ARRAY, "TriangleIndices", Some(int), 0),
    ]);
    let mesh = b.typedef(&[
        (STRING, "Name", None, 0),
        (REFERENCE, "PrimaryVertexData", Some(vdata), 0),
        (REFERENCE, "PrimaryTopology", Some(topo), 0),
        (REFERENCE_TO_ARRAY, "MaterialBindings", Some(binding), 0),
        (REFERENCE_TO_ARRAY, "BoneBindings", Some(bone_binding), 0),
    ]);
    let mesh_binding = b.typedef(&[(REFERENCE, "Mesh", Some(mesh), 0)]);
    let model = b.typedef(&[
        (STRING, "Name", None, 0),
        (REFERENCE, "Skeleton", Some(skel), 0),
        (REFERENCE_TO_ARRAY, "MeshBindings", Some(mesh_binding), 0),
    ]);
    let curve = b.typedef(&[
        (INT32, "Degree", None, 0),
        (REFERENCE_TO_ARRAY, "Knots", Some(real), 0),
        (REFERENCE_TO_ARRAY, "Controls", Some(real), 0),
    ]);
    let track = b.typedef(&[
        (STRING, "Name", None, 0),
        (INLINE, "OrientationCurve", Some(curve), 0),
        (INLINE, "PositionCurve", Some(curve), 0),
        (INLINE, "ScaleShearCurve", Some(curve), 0),
    ]);
    let group_t = b.typedef(&[
        (STRING, "Name", None, 0),
        (REFERENCE_TO_ARRAY, "TransformTracks", Some(track), 0),
    ]);
    let anim = b.typedef(&[
        (STRING, "Name", None, 0),
        (REAL32, "Duration", None, 0),
        (REAL32, "TimeStep", None, 0),
        (ARRAY_OF_REFERENCES, "TrackGroups", Some(group_t), 0),
    ]);
    let root = b.typedef(&[
        (REFERENCE, "ArtToolInfo", Some(art), 0),
        (STRING, "FromFileName", None, 0),
        (ARRAY_OF_REFERENCES, "Textures", Some(tex), 0),
        (ARRAY_OF_REFERENCES, "Materials", Some(mat), 0),
        (ARRAY_OF_REFERENCES, "Skeletons", Some(skel), 0),
        (ARRAY_OF_REFERENCES, "Meshes", Some(mesh), 0),
        (ARRAY_OF_REFERENCES, "Models", Some(model), 0),
        (ARRAY_OF_REFERENCES, "Animations", Some(anim), 0),
    ]);

    let art_o = b.object(
        art,
        vec![
            Val::Str("3D Studio MAX"),
            Val::F32s(vec![1.0]),
            Val::F32s(vec![0.0; 3]),
            Val::F32s(vec![1.0, 0.0, 0.0]),
            Val::F32s(vec![0.0, 0.0, 1.0]),
            Val::F32s(vec![0.0, -1.0, 0.0]),
        ],
    );
    let tex_o = b.object(tex, vec![Val::Str(texture), Val::I32(256), Val::I32(256)]);
    let mat_o = b.object(mat, vec![Val::Str("cube"), Val::Ptr(Some(tex_o))]);
    let identity = Val::Transform(
        0,
        [0.0; 3],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    );
    let mut eye = vec![0.0; 16];
    for i in 0..4 {
        eye[5 * i] = 1.0;
    }
    let skel_o = b.object(
        skel,
        vec![
            Val::Str("root"),
            Val::Array(vec![vec![
                Val::Str("root"),
                Val::I32(-1),
                identity,
                Val::F32s(eye),
            ]]),
        ],
    );
    // Six faces of four vertices, each two triangles.
    let mut bytes = Vec::new();
    let mut indices = Vec::new();
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
    ];
    for (f, (n, u, v)) in faces.iter().enumerate() {
        for (k, (su, sv)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .into_iter()
            .enumerate()
        {
            let p: Vec<f32> = (0..3).map(|i| n[i] + su * u[i] + sv * v[i]).collect();
            let uv = [(su + 1.0) / 2.0, (sv + 1.0) / 2.0];
            for x in p.iter().chain(n.iter()).chain(uv.iter()) {
                bytes.extend_from_slice(&x.to_le_bytes());
            }
            let _ = k;
        }
        let base = 4 * f as i32;
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let data = b.bulk(&bytes);
    let vdata_o = b.object(vdata, vec![Val::VariantArray(vertex, 24, data)]);
    let topo_o = b.object(
        topo,
        vec![
            Val::Array(vec![vec![Val::I32(0), Val::I32(0), Val::I32(12)]]),
            Val::Array(Vec::new()),
            Val::Array(indices.iter().map(|&i| vec![Val::I16(i as i16)]).collect()),
        ],
    );
    let mesh_o = b.object(
        mesh,
        vec![
            Val::Str("Cube"),
            Val::Ptr(Some(vdata_o)),
            Val::Ptr(Some(topo_o)),
            Val::Array(vec![vec![Val::Ptr(Some(mat_o))]]),
            Val::Array(vec![vec![
                Val::Str("root"),
                Val::F32s(vec![-1.0; 3]),
                Val::F32s(vec![1.0; 3]),
                Val::Array((0..12).map(|t| vec![Val::I32(t)]).collect()),
            ]]),
        ],
    );
    let model_o = b.object(
        model,
        vec![
            Val::Str("Cube"),
            Val::Ptr(Some(skel_o)),
            Val::Array(vec![vec![Val::Ptr(Some(mesh_o))]]),
        ],
    );
    let empty_curve = || vec![Val::I32(0), Val::Array(Vec::new()), Val::Array(Vec::new())];
    let spin = vec![
        Val::I32(2),
        Val::Array(vec![vec![Val::F32s(vec![0.0])], vec![Val::F32s(vec![1.0])]]),
        Val::Array(
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]
                .iter()
                .map(|&c| vec![Val::F32s(vec![c])])
                .collect(),
        ),
    ];
    let group_o = b.object(
        group_t,
        vec![
            Val::Str("Cube"),
            Val::Array(vec![vec![
                Val::Str("root"),
                Val::Inline(spin),
                Val::Inline(empty_curve()),
                Val::Inline(empty_curve()),
            ]]),
        ],
    );
    let anim_o = b.object(
        anim,
        vec![
            Val::Str("spin.max"),
            Val::F32s(vec![1.0]),
            Val::F32s(vec![0.02]),
            Val::Refs(vec![group_o]),
        ],
    );
    let root_o = b.object(
        root,
        vec![
            Val::Ptr(Some(art_o)),
            Val::Str("cube.max"),
            Val::Refs(vec![tex_o]),
            Val::Refs(vec![mat_o]),
            Val::Refs(vec![skel_o]),
            Val::Refs(vec![mesh_o]),
            Val::Refs(vec![model_o]),
            Val::Refs(vec![anim_o]),
        ],
    );
    b.finish(root, root_o)
}
