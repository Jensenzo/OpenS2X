//! Granny model and animation files (`specs/GFX-granny-format.md`): the
//! container (GFX-GR2-1), Oodle1 decompression (GFX-GR2-2), pointer fixups
//! and the self-describing type system (GFX-GR2-3), decoded into our own
//! types (GFX-GR2-4, 5).

pub mod basis;
pub mod curve;
pub mod model;
pub mod oodle1;
#[cfg(test)]
pub(crate) mod testing;

use std::collections::HashMap;

pub use model::{
    load, Animation, ArtToolInfo, Bone, BoneBinding, Curve, Gr2, Material, Mesh, ModelInfo,
    Skeleton, TextureRef, TrackGroup, Transform, TransformTrack, TriangleGroup, VertexComponent,
    Vertices,
};

/// GFX-GR2-1.1: 32-bit pointers, little endian.
pub const MAGIC: [u8; 16] = [
    0xB8, 0x67, 0xB0, 0xCA, 0xF8, 0x6D, 0xB1, 0x0F, 0x84, 0x72, 0x8C, 0x7E, 0x5E, 0x19, 0x00, 0x1E,
];
/// GFX-GR2-1.2: offset of the file info.
const INFO_OFFSET: usize = 32;
/// GFX-GR2-1.3: 11 u32 per section.
const SECTION_RECORD: usize = 44;
/// GFX-GR2-1.5: bytes per fixup.
const FIXUP_RECORD: usize = 12;
/// GFX-GR2-3.1: bytes per member record.
const MEMBER_RECORD: u32 = 32;

/// GFX-GR2-1.4: section compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Oodle0,
    Oodle1,
    BitKnit(u32),
}

impl Compression {
    fn from_u32(value: u32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Oodle0,
            2 => Self::Oodle1,
            other => Self::BitKnit(other),
        }
    }
}

/// GFX-GR2-1.3: a section table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionInfo {
    pub compression: Compression,
    pub data_offset: u32,
    pub stored_size: u32,
    pub expanded_size: u32,
    pub alignment: u32,
    pub first16: u32,
    pub first8: u32,
    pub fixup_offset: u32,
    pub fixup_count: u32,
    pub marshal_offset: u32,
    pub marshal_count: u32,
}

/// GFX-GR2-1.1, 1.2: the header and file info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub header_size: u32,
    pub format_version: i32,
    pub total_size: u32,
    pub crc: u32,
    pub root_type: Ref,
    pub root_object: Ref,
    pub type_tag: u32,
    pub sections: Vec<SectionInfo>,
}

/// A location in a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ref {
    pub section: u32,
    pub offset: u32,
}

/// Why a file was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gr2Error {
    NotGr2,
    Truncated(&'static str),
    UnsupportedCompression {
        section: usize,
        compression: Compression,
    },
    Section {
        section: usize,
        error: oodle1::Oodle1Error,
    },
    /// A section expanded to another size than its table says.
    SizeMismatch {
        section: usize,
    },
    BadFixup {
        section: usize,
    },
    /// A reference or member outside its section.
    BadReference(Ref),
    UnknownMemberType(u32),
    Missing(&'static str),
}

impl std::fmt::Display for Gr2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotGr2 => write!(f, "not a Granny file (GFX-GR2-1.1)"),
            Self::Truncated(what) => write!(f, "file too short for its {what}"),
            Self::UnsupportedCompression {
                section,
                compression,
            } => write!(f, "section {section}: {compression:?} is not supported"),
            Self::Section { section, error } => write!(f, "section {section}: {error}"),
            Self::SizeMismatch { section } => write!(f, "section {section}: wrong expanded size"),
            Self::BadFixup { section } => write!(f, "section {section}: fixup out of range"),
            Self::BadReference(r) => write!(f, "reference {r:?} out of range"),
            Self::UnknownMemberType(t) => write!(f, "unknown member type {t}"),
            Self::Missing(what) => write!(f, "missing {what}"),
        }
    }
}

impl std::error::Error for Gr2Error {}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// GFX-GR2-1: read the header and section table of an unwrapped file.
pub fn read_header(data: &[u8]) -> Result<Header, Gr2Error> {
    if data.get(..16) != Some(&MAGIC[..]) {
        return Err(Gr2Error::NotGr2);
    }
    let at = |offset: usize| u32_at(data, offset).ok_or(Gr2Error::Truncated("header"));
    let header_size = at(16)?;
    let info = INFO_OFFSET;
    let table = info + at(info + 12)? as usize;
    let count = at(info + 16)? as usize;
    // A damaged count must not reserve more than the file can hold.
    let mut sections = Vec::with_capacity(count.min(data.len() / SECTION_RECORD));
    for i in 0..count {
        let base = table + i * SECTION_RECORD;
        let f = |k: usize| u32_at(data, base + 4 * k).ok_or(Gr2Error::Truncated("section table"));
        sections.push(SectionInfo {
            compression: Compression::from_u32(f(0)?),
            data_offset: f(1)?,
            stored_size: f(2)?,
            expanded_size: f(3)?,
            alignment: f(4)?,
            first16: f(5)?,
            first8: f(6)?,
            fixup_offset: f(7)?,
            fixup_count: f(8)?,
            marshal_offset: f(9)?,
            marshal_count: f(10)?,
        });
    }
    Ok(Header {
        header_size,
        format_version: at(info)? as i32,
        total_size: at(info + 4)?,
        crc: at(info + 8)?,
        root_type: Ref {
            section: at(info + 20)?,
            offset: at(info + 24)?,
        },
        root_object: Ref {
            section: at(info + 28)?,
            offset: at(info + 32)?,
        },
        type_tag: at(info + 36)?,
        sections,
    })
}

/// A file with its sections expanded and its pointers restored.
#[derive(Debug, Clone)]
pub struct RawFile {
    pub header: Header,
    pub sections: Vec<Vec<u8>>,
    /// GFX-GR2-1.5: per section, the pointer slots and where they point.
    pointers: Vec<HashMap<u32, Ref>>,
}

/// GFX-GR2-1 to 3: expand every section and apply the fixups.
pub fn read_raw(data: &[u8]) -> Result<RawFile, Gr2Error> {
    let header = read_header(data)?;
    let mut sections = Vec::with_capacity(header.sections.len());
    for (i, s) in header.sections.iter().enumerate() {
        let start = s.data_offset as usize;
        let stored = data
            .get(start..start + s.stored_size as usize)
            .ok_or(Gr2Error::Truncated("section data"))?;
        let expanded = match s.compression {
            Compression::None => stored.to_vec(),
            Compression::Oodle1 if s.expanded_size == 0 => Vec::new(),
            Compression::Oodle1 => {
                let out = oodle1::decompress(
                    stored,
                    s.expanded_size as usize,
                    s.first16 as usize,
                    s.first8 as usize,
                );
                match out {
                    Ok((out, _)) => out,
                    Err(error) => return Err(Gr2Error::Section { section: i, error }),
                }
            }
            compression => {
                return Err(Gr2Error::UnsupportedCompression {
                    section: i,
                    compression,
                })
            }
        };
        if expanded.len() != s.expanded_size as usize {
            return Err(Gr2Error::SizeMismatch { section: i });
        }
        sections.push(expanded);
    }
    let mut pointers = Vec::with_capacity(sections.len());
    for (i, s) in header.sections.iter().enumerate() {
        let mut map = HashMap::new();
        for k in 0..s.fixup_count as usize {
            let base = s.fixup_offset as usize + k * FIXUP_RECORD;
            let f = |j: usize| u32_at(data, base + 4 * j).ok_or(Gr2Error::Truncated("fixups"));
            let (source, section, offset) = (f(0)?, f(1)?, f(2)?);
            let fits = (source as usize + 4) <= sections[i].len()
                && sections
                    .get(section as usize)
                    .is_some_and(|t| offset as usize <= t.len());
            if !fits {
                return Err(Gr2Error::BadFixup { section: i });
            }
            map.insert(source, Ref { section, offset });
        }
        pointers.push(map);
    }
    Ok(RawFile {
        header,
        sections,
        pointers,
    })
}

/// GFX-GR2-3.2: member types.
pub mod member {
    pub const END: u32 = 0;
    pub const INLINE: u32 = 1;
    pub const REFERENCE: u32 = 2;
    pub const REFERENCE_TO_ARRAY: u32 = 3;
    pub const ARRAY_OF_REFERENCES: u32 = 4;
    pub const VARIANT_REFERENCE: u32 = 5;
    pub const REFERENCE_TO_VARIANT_ARRAY: u32 = 7;
    pub const STRING: u32 = 8;
    pub const TRANSFORM: u32 = 9;
    pub const REAL32: u32 = 10;
    pub const INT8: u32 = 11;
    pub const UINT8: u32 = 12;
    pub const BINORMAL_INT8: u32 = 13;
    pub const NORMAL_UINT8: u32 = 14;
    pub const INT16: u32 = 15;
    pub const UINT16: u32 = 16;
    pub const BINORMAL_INT16: u32 = 17;
    pub const NORMAL_UINT16: u32 = 18;
    pub const INT32: u32 = 19;
    pub const UINT32: u32 = 20;
    pub const REAL16: u32 = 21;
    pub const EMPTY_REFERENCE: u32 = 22;
}

/// GFX-GR2-3.1: a member of a type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub kind: u32,
    pub name: String,
    pub sub_type: Option<Ref>,
    /// 0 for a single value.
    pub array_count: u32,
}

impl RawFile {
    fn bytes(&self, at: Ref, len: usize) -> Result<&[u8], Gr2Error> {
        self.sections
            .get(at.section as usize)
            .and_then(|s| s.get(at.offset as usize..at.offset as usize + len))
            .ok_or(Gr2Error::BadReference(at))
    }

    pub fn u32(&self, at: Ref) -> Result<u32, Gr2Error> {
        Ok(u32::from_le_bytes(
            self.bytes(at, 4)?.try_into().expect("4"),
        ))
    }

    pub fn f32(&self, at: Ref) -> Result<f32, Gr2Error> {
        Ok(f32::from_bits(self.u32(at)?))
    }

    /// The pointer in the slot at `at`; `None` for a slot without fixup.
    pub fn pointer(&self, at: Ref) -> Option<Ref> {
        self.pointers
            .get(at.section as usize)?
            .get(&at.offset)
            .copied()
    }

    /// A C string.
    pub fn string(&self, at: Ref) -> Result<String, Gr2Error> {
        let section = self
            .sections
            .get(at.section as usize)
            .ok_or(Gr2Error::BadReference(at))?;
        let rest = section
            .get(at.offset as usize..)
            .ok_or(Gr2Error::BadReference(at))?;
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        Ok(rest[..end].iter().map(|&b| char::from(b)).collect())
    }

    /// GFX-GR2-3.1: the members of the type definition at `at`.
    pub fn members(&self, at: Ref) -> Result<Vec<Member>, Gr2Error> {
        let mut members = Vec::new();
        let mut record = at;
        loop {
            let kind = self.u32(record)?;
            if kind == member::END {
                return Ok(members);
            }
            let name = match self.pointer(Ref {
                offset: record.offset + 4,
                ..record
            }) {
                Some(p) => self.string(p)?,
                None => String::new(),
            };
            let sub_type = self.pointer(Ref {
                offset: record.offset + 8,
                ..record
            });
            let array_count = self.u32(Ref {
                offset: record.offset + 12,
                ..record
            })?;
            members.push(Member {
                kind,
                name,
                sub_type,
                array_count,
            });
            record.offset += MEMBER_RECORD;
        }
    }

    /// GFX-GR2-3.2: bytes one member takes in an object.
    pub fn member_size(&self, m: &Member) -> Result<u32, Gr2Error> {
        use member::*;
        let one = match m.kind {
            INLINE => match m.sub_type {
                Some(t) => self.type_size(t)?,
                None => 0,
            },
            REFERENCE | STRING | REAL32 | INT32 | UINT32 | EMPTY_REFERENCE => 4,
            REFERENCE_TO_ARRAY | ARRAY_OF_REFERENCES | VARIANT_REFERENCE => 8,
            REFERENCE_TO_VARIANT_ARRAY => 12,
            TRANSFORM => 68,
            INT8 | UINT8 | BINORMAL_INT8 | NORMAL_UINT8 => 1,
            INT16 | UINT16 | BINORMAL_INT16 | NORMAL_UINT16 | REAL16 => 2,
            other => return Err(Gr2Error::UnknownMemberType(other)),
        };
        Ok(one * m.array_count.max(1))
    }

    /// The size of an object of the type at `at`.
    pub fn type_size(&self, at: Ref) -> Result<u32, Gr2Error> {
        let mut size = 0;
        for m in self.members(at)? {
            size += self.member_size(&m)?;
        }
        Ok(size)
    }

    /// The root object (GFX-GR2-3.3).
    pub fn root(&self) -> Result<Object<'_>, Gr2Error> {
        Object::new(self, self.header.root_object, self.header.root_type)
    }
}

/// An object in a file, read through its type definition.
#[derive(Debug, Clone)]
pub struct Object<'a> {
    pub file: &'a RawFile,
    pub at: Ref,
    pub type_ref: Ref,
    members: Vec<(Member, u32)>,
}

/// A member found in an object: its record and where its value starts.
#[derive(Debug, Clone)]
pub struct Field<'a> {
    pub file: &'a RawFile,
    pub member: Member,
    pub at: Ref,
}

impl<'a> Object<'a> {
    pub fn new(file: &'a RawFile, at: Ref, type_ref: Ref) -> Result<Self, Gr2Error> {
        let mut members = Vec::new();
        let mut offset = 0;
        for m in file.members(type_ref)? {
            let size = file.member_size(&m)?;
            members.push((m, offset));
            offset += size;
        }
        Ok(Self {
            file,
            at,
            type_ref,
            members,
        })
    }

    /// The members of the object's type in order.
    pub fn member_names(&self) -> impl Iterator<Item = &str> {
        self.members.iter().map(|(m, _)| m.name.as_str())
    }

    pub fn field(&self, name: &str) -> Option<Field<'a>> {
        self.members
            .iter()
            .find(|(m, _)| m.name == name)
            .map(|(m, offset)| Field {
                file: self.file,
                member: m.clone(),
                at: Ref {
                    section: self.at.section,
                    offset: self.at.offset + offset,
                },
            })
    }

    pub fn string(&self, name: &str) -> Option<String> {
        self.field(name)?.string()
    }

    pub fn f32(&self, name: &str) -> Option<f32> {
        self.field(name)?.f32s().first().copied()
    }

    pub fn f32s(&self, name: &str) -> Vec<f32> {
        self.field(name).map(|f| f.f32s()).unwrap_or_default()
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        self.field(name)?.int()
    }

    pub fn object(&self, name: &str) -> Option<Object<'a>> {
        self.field(name)?.object()
    }

    pub fn objects(&self, name: &str) -> Vec<Object<'a>> {
        self.field(name).map(|f| f.objects()).unwrap_or_default()
    }
}

impl<'a> Field<'a> {
    fn at(&self, extra: u32) -> Ref {
        Ref {
            section: self.at.section,
            offset: self.at.offset + extra,
        }
    }

    pub fn string(&self) -> Option<String> {
        if self.member.kind != member::STRING {
            return None;
        }
        self.file
            .pointer(self.at)
            .and_then(|p| self.file.string(p).ok())
    }

    /// Numbers of a numeric member (all its array entries), as f32.
    pub fn f32s(&self) -> Vec<f32> {
        use member::*;
        let n = self.member.array_count.max(1);
        let file = self.file;
        (0..n)
            .filter_map(|i| match self.member.kind {
                REAL32 => file.f32(self.at(4 * i)).ok(),
                INT32 => file.u32(self.at(4 * i)).ok().map(|v| v as i32 as f32),
                UINT32 => file.u32(self.at(4 * i)).ok().map(|v| v as f32),
                UINT8 | NORMAL_UINT8 => file.bytes(self.at(i), 1).ok().map(|b| f32::from(b[0])),
                INT16 | UINT16 => file
                    .bytes(self.at(2 * i), 2)
                    .ok()
                    .map(|b| f32::from(u16::from_le_bytes([b[0], b[1]]))),
                _ => None,
            })
            .collect()
    }

    pub fn int(&self) -> Option<i64> {
        use member::*;
        let file = self.file;
        match self.member.kind {
            INT32 => file.u32(self.at).ok().map(|v| i64::from(v as i32)),
            UINT32 => file.u32(self.at).ok().map(i64::from),
            INT16 => file
                .bytes(self.at, 2)
                .ok()
                .map(|b| i64::from(i16::from_le_bytes([b[0], b[1]]))),
            UINT16 => file
                .bytes(self.at, 2)
                .ok()
                .map(|b| i64::from(u16::from_le_bytes([b[0], b[1]]))),
            INT8 => file.bytes(self.at, 1).ok().map(|b| i64::from(b[0] as i8)),
            UINT8 => file.bytes(self.at, 1).ok().map(|b| i64::from(b[0])),
            _ => None,
        }
    }

    /// An inline struct, a pointed-to object, or a variant's object.
    pub fn object(&self) -> Option<Object<'a>> {
        use member::*;
        let file = self.file;
        match self.member.kind {
            INLINE => Object::new(file, self.at, self.member.sub_type?).ok(),
            REFERENCE => Object::new(file, file.pointer(self.at)?, self.member.sub_type?).ok(),
            VARIANT_REFERENCE => {
                let ty = file.pointer(self.at)?;
                Object::new(file, file.pointer(self.at(4))?, ty).ok()
            }
            _ => None,
        }
    }

    /// The count and first element of an array member.
    fn array(&self) -> Option<(u32, Ref)> {
        let count = self.file.u32(self.at).ok()?;
        let data = self.file.pointer(self.at(4))?;
        Some((count, data))
    }

    /// The objects of an array of objects or an array of references.
    pub fn objects(&self) -> Vec<Object<'a>> {
        use member::*;
        let file = self.file;
        let Some(sub) = self.member.sub_type else {
            return Vec::new();
        };
        let Some((count, data)) = self.array() else {
            return Vec::new();
        };
        match self.member.kind {
            REFERENCE_TO_ARRAY => {
                let Ok(size) = file.type_size(sub) else {
                    return Vec::new();
                };
                (0..count)
                    .filter_map(|i| {
                        let at = Ref {
                            section: data.section,
                            offset: data.offset + i * size,
                        };
                        Object::new(file, at, sub).ok()
                    })
                    .collect()
            }
            ARRAY_OF_REFERENCES => (0..count)
                .filter_map(|i| {
                    let slot = Ref {
                        section: data.section,
                        offset: data.offset + 4 * i,
                    };
                    Object::new(file, file.pointer(slot)?, sub).ok()
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// GFX-GR2-3.2 type 7: the element type, count and data of a variant
    /// array.
    pub fn variant_array(&self) -> Option<(Ref, u32, Ref)> {
        if self.member.kind != member::REFERENCE_TO_VARIANT_ARRAY {
            return None;
        }
        let ty = self.file.pointer(self.at)?;
        let count = self.file.u32(self.at(4)).ok()?;
        let data = self.file.pointer(self.at(8))?;
        Some((ty, count, data))
    }

    /// The numbers of an array whose elements are one number each (the
    /// sub-type's single member: `Knots`, `Controls`, `Indices`, ...).
    pub fn numbers(&self) -> Vec<f64> {
        if self.member.kind != member::REFERENCE_TO_ARRAY {
            return Vec::new();
        }
        let file = self.file;
        let Some(sub) = self.member.sub_type else {
            return Vec::new();
        };
        let Ok(members) = file.members(sub) else {
            return Vec::new();
        };
        let [element] = members.as_slice() else {
            return Vec::new();
        };
        let Ok(size) = file.member_size(element) else {
            return Vec::new();
        };
        let Some((count, data)) = self.array() else {
            return Vec::new();
        };
        let Ok(bytes) = file.bytes(data, (count * size) as usize) else {
            return Vec::new();
        };
        let per = element.array_count.max(1) as usize;
        let width = size as usize / per;
        bytes
            .chunks_exact(width)
            .filter_map(|b| {
                Some(match element.kind {
                    member::REAL32 => f64::from(f32::from_le_bytes(b.try_into().ok()?)),
                    member::INT32 => f64::from(i32::from_le_bytes(b.try_into().ok()?)),
                    member::UINT32 => f64::from(u32::from_le_bytes(b.try_into().ok()?)),
                    member::INT16 => f64::from(i16::from_le_bytes(b.try_into().ok()?)),
                    member::UINT16 => f64::from(u16::from_le_bytes(b.try_into().ok()?)),
                    member::INT8 => f64::from(b[0] as i8),
                    member::UINT8 => f64::from(b[0]),
                    _ => return None,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> RawFile {
        read_raw(&testing::cube("Z:\\Art\\Cube01.dds")).unwrap()
    }

    // GFX-GR2-1: header and section table.
    #[test]
    fn header() {
        let data = testing::cube("a.dds");
        let h = read_header(&data).unwrap();
        assert_eq!((h.header_size, h.format_version), (0x1B8, 6));
        assert_eq!(h.sections.len(), 8);
        assert_eq!(h.type_tag, 0x8000_0013);
        assert!(h
            .sections
            .iter()
            .all(|s| s.compression == Compression::None));
        let mut bad = data.clone();
        bad[0] ^= 1;
        assert_eq!(read_header(&bad), Err(Gr2Error::NotGr2));
        assert!(matches!(
            read_header(&data[..40]),
            Err(Gr2Error::Truncated(_))
        ));
    }

    // GFX-GR2-1.5, 3.1, 3.2: fixups make pointers; types describe objects.
    #[test]
    fn types_and_pointers() {
        let file = cube();
        let root = file.root().unwrap();
        let names: Vec<&str> = root.member_names().collect();
        assert_eq!(names[..3], ["ArtToolInfo", "FromFileName", "Textures"]);
        assert_eq!(root.string("FromFileName").as_deref(), Some("cube.max"));
        let tool = root.object("ArtToolInfo").unwrap();
        assert_eq!(tool.f32s("UpVector"), [0.0, 0.0, 1.0]);
        // A transform is 68 bytes, an array of references 8.
        let skeleton = root.objects("Skeletons").remove(0);
        let bone_type = skeleton.field("Bones").unwrap().member.sub_type.unwrap();
        assert_eq!(file.type_size(bone_type).unwrap(), 4 + 4 + 68 + 64);
        // A slot without a fixup is null.
        assert_eq!(
            file.pointer(Ref {
                section: 0,
                offset: 1
            }),
            None
        );
    }

    // GFX-GR2-4, 5: the content in our own types.
    #[test]
    fn content() {
        let gr2 = Gr2::from_raw(&cube()).unwrap();
        let art = gr2.art_tool.as_ref().unwrap();
        assert_eq!(
            (art.tool.as_str(), art.units_per_meter),
            ("3D Studio MAX", 1.0)
        );
        assert_eq!(gr2.textures.len(), 1);
        assert_eq!(gr2.textures[0].base_name(), "Cube01.dds");
        assert_eq!(gr2.materials[0].textures, [0]);
        let mesh = &gr2.meshes[0];
        assert_eq!(mesh.name, "Cube");
        assert_eq!(mesh.vertices.count, 24);
        let layout: Vec<&str> = mesh
            .vertices
            .layout
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(layout, ["Position", "Normal", "TextureCoordinates0"]);
        assert_eq!(mesh.triangle_count(), 12);
        assert!(mesh
            .indices
            .iter()
            .all(|&i| (i as usize) < mesh.vertices.count));
        assert_eq!(mesh.vertices.positions[0], [1.0, -1.0, -1.0]);
        assert_eq!(mesh.vertices.uvs[0][2], [1.0, 1.0]);
        assert_eq!(
            mesh.groups,
            [TriangleGroup {
                material: 0,
                first_triangle: 0,
                triangle_count: 12
            }]
        );
        assert_eq!(mesh.materials, [Some(0)]);
        assert_eq!(mesh.bone_bindings[0].bone, "root");
        assert_eq!(mesh.bone_bindings[0].triangles.len(), 12);
        let skeleton = &gr2.skeletons[0];
        assert_eq!(skeleton.bones[0].parent, None);
        assert_eq!(
            skeleton.bones[0].transform.orientation,
            [0.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(skeleton.bones[0].inverse_world[15], 1.0);
        assert_eq!(gr2.models[0].skeleton, Some(0));
        assert_eq!(gr2.models[0].meshes, [0]);
        let anim = &gr2.animations[0];
        assert_eq!((anim.duration, anim.time_step), (1.0, 0.02));
        assert_eq!(anim.transform_track_count(), 1);
        let track = &anim.track_groups[0].transform_tracks[0];
        assert_eq!(track.orientation.degree, 2);
        assert_eq!(track.orientation.knots, [0.0, 1.0]);
        assert_eq!(track.orientation.controls.len(), 8);
        assert!(track.position.is_empty());
    }

    // GFX-GR2-2.7: a stream that does not end where the data says is
    // reported, not returned.
    #[test]
    fn diverging_stream_is_an_error() {
        let mut stored = vec![0u8; 36];
        for block in 0..3 {
            stored[12 * block..12 * block + 4].copy_from_slice(&0x7FF_FF00u32.to_le_bytes());
            stored[12 * block + 4..12 * block + 8].copy_from_slice(&0x80001u32.to_le_bytes());
            stored[12 * block + 8..12 * block + 12].copy_from_slice(&[1, 1, 1, 1]);
        }
        stored.extend(std::iter::repeat_n(0x55, 64));
        assert!(oodle1::decompress(&stored, 1000, 0, 0).is_err());
        assert_eq!(
            oodle1::decompress(&stored[..10], 10, 0, 0),
            Err(oodle1::Oodle1Error::Truncated)
        );
        assert_eq!(
            oodle1::decompress(&stored, 10, 5, 2),
            Err(oodle1::Oodle1Error::BadParts)
        );
    }

    /// Writes the test cube to `$S2_WRITE_CUBE`, for looking at it in the
    /// viewer (`s2-viewer --model <file>`).
    // spec: none (a test file the reader is checked with)
    #[test]
    #[ignore = "writes a model file for the viewer"]
    fn write_cube_model() {
        if let Ok(path) = std::env::var("S2_WRITE_CUBE") {
            std::fs::write(path, testing::cube("Z:\\Art\\PB_R_Sawmill01.dds")).unwrap();
        }
    }
}
