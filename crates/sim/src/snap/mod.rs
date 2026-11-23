//! Our own snapshot of the whole game state (`docs/snapshot.md`): every
//! value of a [`World`] written to bytes and read back, so a game can be
//! saved at any tick and continued as if it had never been saved. It is not
//! the original's save format (FMT-SAV, read by `s2-assets` and taken over
//! by [`World::restore`]); it is ours alone and only s2-reborn reads it.
//!
//! Each state type implements [`Snap`]: plain values through the impls
//! below, structs and enums through [`snap_struct!`] and [`snap_enum!`],
//! which list every field. A struct field missing from the list does not
//! compile (the struct is built with all its fields), nor does a missing
//! variant (the match is exhaustive), so a new piece of state cannot be left
//! out silently.
//!
//! Encoding: unsigned integers as LEB128, signed ones zigzag first, `bool`
//! and `u8` as one byte, sequences and strings with their length first, an
//! `Option` as a byte 0 or 1 before the value, an enum as the index of its
//! variant (in the order the macro lists them) before its fields.
//!
//! **A save of ours loads in every later version** (`docs/snapshot.md`,
//! "Changing the format"). [`Reader::format`] says which format is being
//! read; a field added after [`OLDEST`] names the format it came with
//! (`snap_struct!(Road { owner, hidden @ 66 = false })`), and a variant is
//! added at the end of its macro line. [`Shape`] writes the whole format out
//! as text, which a test holds against the recorded one, so a change of the
//! format cannot go unnoticed.

#[cfg(test)]
mod tests;
mod types;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::World;

/// The first bytes of a snapshot of a [`World`], then the format version.
pub const MAGIC: &[u8; 8] = b"s2-snap\0";
/// The format this build writes, of the world and of the computer players.
/// Changed whenever the bytes or the meaning of any of their types change
/// (`docs/snapshot.md`, "Changing the format").
pub const VERSION: u32 = 99;
/// The oldest format this build reads: the first whose saves every later
/// version loads. Never raised.
pub const OLDEST: u32 = 65;

/// Why a snapshot could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapError(pub String);

impl std::fmt::Display for SnapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "snapshot: {}", self.0)
    }
}

impl std::error::Error for SnapError {}

/// The bytes being written.
#[derive(Debug, Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn u64(&mut self, mut value: u64) {
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                self.bytes.push(byte);
                return;
            }
            self.bytes.push(byte | 0x80);
        }
    }

    pub fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn put<T: Snap>(&mut self, value: &T) {
        value.put(self);
    }
}

/// The bytes being read.
#[derive(Debug)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    format: u32,
}

impl<'a> Reader<'a> {
    /// Bytes of this build's format, until [`World::read_snapshot`] reads
    /// which format they are.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self::of_format(bytes, VERSION)
    }

    /// Bytes written in the format `format`.
    pub fn of_format(bytes: &'a [u8], format: u32) -> Self {
        Self {
            bytes,
            at: 0,
            format,
        }
    }

    /// The format of the bytes being read: a field or a value that came
    /// with a later format is not in them, and takes its default.
    pub fn format(&self) -> u32 {
        self.format
    }

    pub fn at_end(&self) -> bool {
        self.at == self.bytes.len()
    }

    pub fn raw(&mut self, n: usize) -> Result<&'a [u8], SnapError> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| SnapError(format!("ends early at byte {}", self.at)))?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    pub fn u64(&mut self) -> Result<u64, SnapError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.raw(1)?[0];
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(self.error("integer longer than 64 bits"))
    }

    /// A length, checked against the bytes left (every element takes at
    /// least one byte), so a broken file cannot ask for a huge allocation.
    pub fn length(&mut self) -> Result<usize, SnapError> {
        let n = self.u64()?;
        if n > (self.bytes.len() - self.at) as u64 {
            return Err(self.error("length beyond the end"));
        }
        Ok(n as usize)
    }

    pub fn get<T: Snap>(&mut self) -> Result<T, SnapError> {
        T::get(self)
    }

    pub fn error(&self, what: &str) -> SnapError {
        SnapError(format!("{what} at byte {}", self.at))
    }

    /// An enum's variant index, as the name it stands for.
    pub fn variant(&mut self, names: &[&'static str]) -> Result<&'static str, SnapError> {
        let index = self.u64()?;
        names
            .get(index as usize)
            .copied()
            .ok_or_else(|| self.error("unknown variant"))
    }
}

/// A value that can be written to a snapshot and read back unchanged.
pub trait Snap: Sized {
    fn put(&self, w: &mut Writer);
    fn get(r: &mut Reader) -> Result<Self, SnapError>;
    /// The type's part of the format's description ([`Shape`]): what other
    /// types call it. A type written by hand describes its bytes by hand.
    fn shape(s: &mut Shape) -> String;
}

/// The snapshot format written out as text, one block per type: every field
/// and every variant with its place, in the order of the bytes. Tests hold it
/// against the recorded format (`tests/saves/format.txt`), so a change of the
/// bytes is seen even when nothing else notices it (an added variant, say).
#[derive(Debug, Default)]
pub struct Shape {
    types: BTreeMap<String, Vec<String>>,
}

impl Shape {
    pub fn new() -> Self {
        Self::default()
    }

    /// `T`'s name, its type and the types it holds described.
    pub fn add<T: Snap>(&mut self) -> String {
        T::shape(self)
    }

    /// The type called `name`, described by `lines` unless it already is
    /// (or is being: a type may hold itself).
    pub fn define(&mut self, name: &str, lines: impl FnOnce(&mut Self) -> Vec<String>) -> String {
        if !self.types.contains_key(name) {
            self.types.insert(name.to_string(), Vec::new());
            let lines = lines(self);
            self.types.insert(name.to_string(), lines);
        }
        name.to_string()
    }

    /// A field's line: `field` names the type by what it returns (never
    /// called).
    pub fn field<S, F: Snap>(&mut self, name: &str, _field: fn(&S) -> &F) -> String {
        format!("{name}: {}", F::shape(self))
    }

    /// A variant's line: `fields` names the types of its fields (never
    /// called).
    pub fn variant<S: 'static, F: VariantFields>(
        &mut self,
        index: usize,
        name: &str,
        names: &[&str],
        _fields: fn(&'static S) -> Option<F>,
    ) -> String {
        let types = F::shapes(self);
        let fields: Vec<String> = names
            .iter()
            .zip(types)
            .map(|(name, ty)| format!("{name}: {ty}"))
            .collect();
        format!("{index} {name} {{ {} }}", fields.join(", "))
    }

    /// The whole description.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (name, lines) in &self.types {
            out.push_str(name);
            out.push('\n');
            for line in lines {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }
}

/// The fields of an enum variant, as references: [`Shape::variant`] learns
/// their types from them.
pub trait VariantFields {
    fn shapes(s: &mut Shape) -> Vec<String>;
}

macro_rules! variant_fields {
    ($($t:ident),*) => {
        impl<$($t: Snap + 'static),*> VariantFields for ($(&'static $t,)*) {
            fn shapes(s: &mut Shape) -> Vec<String> {
                vec![$($t::shape(s)),*]
            }
        }
    };
}
variant_fields!(A);
variant_fields!(A, B);
variant_fields!(A, B, C);
variant_fields!(A, B, C, D);
variant_fields!(A, B, C, D, E);
variant_fields!(A, B, C, D, E, F);
variant_fields!(A, B, C, D, E, F, G);

macro_rules! unsigned {
    ($($t:ty),*) => {$(
        impl Snap for $t {
            fn put(&self, w: &mut Writer) {
                w.u64(*self as u64);
            }
            fn get(r: &mut Reader) -> Result<Self, SnapError> {
                let value = r.u64()?;
                <$t>::try_from(value).map_err(|_| r.error("integer out of range"))
            }
            fn shape(_: &mut Shape) -> String {
                stringify!($t).to_string()
            }
        }
    )*};
}
unsigned!(u16, u32, u64, usize);

macro_rules! signed {
    ($($t:ty),*) => {$(
        impl Snap for $t {
            fn put(&self, w: &mut Writer) {
                let v = *self as i64;
                w.u64(((v << 1) ^ (v >> 63)) as u64);
            }
            fn get(r: &mut Reader) -> Result<Self, SnapError> {
                let z = r.u64()?;
                let v = ((z >> 1) as i64) ^ -((z & 1) as i64);
                <$t>::try_from(v).map_err(|_| r.error("integer out of range"))
            }
            fn shape(_: &mut Shape) -> String {
                stringify!($t).to_string()
            }
        }
    )*};
}
signed!(i8, i16, i32, i64);

impl Snap for u8 {
    fn put(&self, w: &mut Writer) {
        w.bytes.push(*self);
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        Ok(r.raw(1)?[0])
    }
    fn shape(_: &mut Shape) -> String {
        "u8".to_string()
    }
}

impl Snap for bool {
    fn put(&self, w: &mut Writer) {
        w.bytes.push(u8::from(*self));
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        match r.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(r.error("not a bool")),
        }
    }
    fn shape(_: &mut Shape) -> String {
        "bool".to_string()
    }
}

impl Snap for String {
    fn put(&self, w: &mut Writer) {
        w.u64(self.len() as u64);
        w.raw(self.as_bytes());
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let n = r.length()?;
        let bytes = r.raw(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| r.error("not UTF-8"))
    }
    fn shape(_: &mut Shape) -> String {
        "string".to_string()
    }
}

impl<T: Snap> Snap for Option<T> {
    fn put(&self, w: &mut Writer) {
        match self {
            None => w.bytes.push(0),
            Some(value) => {
                w.bytes.push(1);
                value.put(w);
            }
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        match r.raw(1)?[0] {
            0 => Ok(None),
            1 => Ok(Some(T::get(r)?)),
            _ => Err(r.error("not an option")),
        }
    }
    fn shape(s: &mut Shape) -> String {
        format!("Option<{}>", T::shape(s))
    }
}

impl<T: Snap> Snap for Box<T> {
    fn put(&self, w: &mut Writer) {
        (**self).put(w);
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        Ok(Box::new(T::get(r)?))
    }
    fn shape(s: &mut Shape) -> String {
        T::shape(s)
    }
}

impl<T: Snap> Snap for Vec<T> {
    fn put(&self, w: &mut Writer) {
        w.u64(self.len() as u64);
        for value in self {
            value.put(w);
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let n = r.length()?;
        (0..n).map(|_| T::get(r)).collect()
    }
    fn shape(s: &mut Shape) -> String {
        format!("seq<{}>", T::shape(s))
    }
}

impl<T: Snap> Snap for VecDeque<T> {
    fn put(&self, w: &mut Writer) {
        w.u64(self.len() as u64);
        for value in self {
            value.put(w);
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let n = r.length()?;
        (0..n).map(|_| T::get(r)).collect()
    }
    fn shape(s: &mut Shape) -> String {
        format!("seq<{}>", T::shape(s))
    }
}

impl<T: Snap, const N: usize> Snap for [T; N] {
    fn put(&self, w: &mut Writer) {
        for value in self {
            value.put(w);
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let values = (0..N).map(|_| T::get(r)).collect::<Result<Vec<T>, _>>()?;
        values.try_into().map_err(|_| r.error("array length"))
    }
    fn shape(s: &mut Shape) -> String {
        format!("[{}; {N}]", T::shape(s))
    }
}

impl<K: Snap + Ord, V: Snap> Snap for BTreeMap<K, V> {
    fn put(&self, w: &mut Writer) {
        w.u64(self.len() as u64);
        for (key, value) in self {
            key.put(w);
            value.put(w);
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let n = r.length()?;
        (0..n).map(|_| Ok((K::get(r)?, V::get(r)?))).collect()
    }
    fn shape(s: &mut Shape) -> String {
        format!("map<{}, {}>", K::shape(s), V::shape(s))
    }
}

impl<T: Snap + Ord> Snap for BTreeSet<T> {
    fn put(&self, w: &mut Writer) {
        w.u64(self.len() as u64);
        for value in self {
            value.put(w);
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let n = r.length()?;
        (0..n).map(|_| T::get(r)).collect()
    }
    fn shape(s: &mut Shape) -> String {
        format!("seq<{}>", T::shape(s))
    }
}

macro_rules! tuple {
    ($($t:ident $i:tt),*) => {
        impl<$($t: Snap),*> Snap for ($($t,)*) {
            fn put(&self, w: &mut Writer) {
                $(self.$i.put(w);)*
            }
            fn get(r: &mut Reader) -> Result<Self, SnapError> {
                Ok(($($t::get(r)?,)*))
            }
            fn shape(s: &mut Shape) -> String {
                let parts: Vec<String> = vec![$($t::shape(s)),*];
                format!("({})", parts.join(", "))
            }
        }
    };
}
tuple!(A 0, B 1);
tuple!(A 0, B 1, C 2);
tuple!(A 0, B 1, C 2, D 3);
tuple!(A 0, B 1, C 2, D 3, E 4);

/// One field of a [`snap_struct!`] or [`snap_enum!`] line being read: from
/// the bytes; or, for one that came with a later format and with bytes older
/// than that, its default (`= value`) or what the older bytes hold instead
/// (`=> |r: &mut Reader| ...`, which reads them and makes the field).
#[macro_export]
macro_rules! snap_field {
    ($r:ident) => {
        $crate::snap::Snap::get($r)?
    };
    ($r:ident @ $since:literal = $default:expr) => {
        if $r.format() >= $since {
            $crate::snap::Snap::get($r)?
        } else {
            $default
        }
    };
    ($r:ident @ $since:literal => $older:expr) => {
        if $r.format() >= $since {
            $crate::snap::Snap::get($r)?
        } else {
            ($older)($r)?
        }
    };
}

/// What the format's description says after a field's name: the format it
/// came with, and `=>` when older bytes are read by hand for it.
#[macro_export]
macro_rules! snap_since {
    () => {
        ""
    };
    (@ $since:literal = $default:expr) => {
        concat!(" @", stringify!($since))
    };
    (@ $since:literal => $older:expr) => {
        concat!(" @", stringify!($since), "=>")
    };
}

/// [`Snap`] for a struct, naming every field: `snap_struct!(Road { owner,
/// path, .. })`, or `snap_struct!(PlayerId, (0))` for a tuple struct. A field
/// added after [`OLDEST`] says with which format it came and what a game
/// saved before has: `hidden @ 66 = false`, or `traffic @ 67 => |r: &mut
/// Reader| ...`, which reads what older bytes have at its place
/// (`docs/snapshot.md`, "Changing the format").
#[macro_export]
macro_rules! snap_struct {
    ($ty:ty { $($f:ident $(@ $since:literal $(= $default:expr)? $(=> $older:expr)?)?),* $(,)? }) => {
        impl $crate::snap::Snap for $ty {
            fn put(&self, w: &mut $crate::snap::Writer) {
                $($crate::snap::Snap::put(&self.$f, w);)*
            }
            fn get(r: &mut $crate::snap::Reader) -> Result<Self, $crate::snap::SnapError> {
                Ok(Self { $($f: $crate::snap_field!(r $(@ $since $(= $default)? $(=> $older)?)?),)* })
            }
            fn shape(s: &mut $crate::snap::Shape) -> String {
                s.define(std::any::type_name::<Self>(), |s| {
                    vec![$(s.field(
                        concat!(stringify!($f), $crate::snap_since!($(@ $since $(= $default)? $(=> $older)?)?)),
                        |v: &$ty| &v.$f,
                    )),*]
                })
            }
        }
    };
    ($ty:ty, ( $($i:tt),* $(,)? )) => {
        impl $crate::snap::Snap for $ty {
            fn put(&self, w: &mut $crate::snap::Writer) {
                $($crate::snap::Snap::put(&self.$i, w);)*
            }
            fn get(r: &mut $crate::snap::Reader) -> Result<Self, $crate::snap::SnapError> {
                Ok(Self($({ let _ = $i; $crate::snap::Snap::get(r)? },)*))
            }
            fn shape(s: &mut $crate::snap::Shape) -> String {
                s.define(std::any::type_name::<Self>(), |s| {
                    vec![$(s.field(stringify!($i), |v: &$ty| &v.$i)),*]
                })
            }
        }
    };
}

/// [`Snap`] for an enum, naming every variant and its fields (tuple fields
/// get names of their own): `snap_enum!(Entry { Ware(w), Settler { id } })`.
/// The place in this list is the variant's number in the bytes: a new variant
/// goes at the end of the list, wherever the enum declares it. A field added
/// to a `{ .. }` variant after [`OLDEST`] is written as in [`snap_struct!`].
#[macro_export]
macro_rules! snap_enum {
    ($ty:ty { $($v:ident $(( $($t:ident),* ))? $({ $($f:ident $(@ $since:literal $(= $default:expr)? $(=> $older:expr)?)?),* })?),* $(,)? }) => {
        impl $crate::snap::Snap for $ty {
            fn put(&self, w: &mut $crate::snap::Writer) {
                const NAMES: &[&str] = &[$(stringify!($v)),*];
                match self {
                    $(Self::$v $(( $($t),* ))? $({ $($f),* })? => {
                        let index = NAMES.iter().position(|n| *n == stringify!($v));
                        w.u64(index.unwrap_or(0) as u64);
                        $($($crate::snap::Snap::put($t, w);)*)?
                        $($($crate::snap::Snap::put($f, w);)*)?
                    })*
                }
            }
            #[allow(non_snake_case)]
            fn get(r: &mut $crate::snap::Reader) -> Result<Self, $crate::snap::SnapError> {
                const NAMES: &[&str] = &[$(stringify!($v)),*];
                let name = r.variant(NAMES)?;
                $(if name == stringify!($v) {
                    return Ok(Self::$v
                        $(( $({ let _ = stringify!($t); $crate::snap::Snap::get(r)? }),* ))?
                        $({ $($f: $crate::snap_field!(r $(@ $since $(= $default)? $(=> $older)?)?)),* })?);
                })*
                Err(r.error("unknown variant"))
            }
            #[allow(unreachable_patterns, unused_mut, unused_variables, unused_assignments)]
            fn shape(s: &mut $crate::snap::Shape) -> String {
                s.define(std::any::type_name::<Self>(), |s| {
                    let mut lines = Vec::new();
                    let mut index = 0usize;
                    $(
                        let mut line = format!("{index} {}", stringify!($v));
                        $(
                            line = s.variant(
                                index,
                                stringify!($v),
                                &[$(stringify!($t)),*],
                                |v: &'static Self| match v {
                                    Self::$v($($t),*) => Some(($($t,)*)),
                                    _ => None,
                                },
                            );
                        )?
                        $(
                            line = s.variant(
                                index,
                                stringify!($v),
                                &[$(concat!(stringify!($f), $crate::snap_since!($(@ $since $(= $default)? $(=> $older)?)?))),*],
                                |v: &'static Self| match v {
                                    Self::$v { $($f),* } => Some(($($f,)*)),
                                    _ => None,
                                },
                            );
                        )?
                        lines.push(line);
                        index += 1;
                    )*
                    lines
                })
            }
        }
    };
}

impl World {
    /// The whole game state as a snapshot (`docs/snapshot.md`).
    pub fn snapshot(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.raw(MAGIC);
        w.u64(u64::from(VERSION));
        self.put(&mut w);
        w.bytes
    }

    /// A world from [`World::snapshot`]'s bytes: equal to the one saved, so
    /// stepping it gives the same game as stepping the original.
    pub fn from_snapshot(bytes: &[u8]) -> Result<World, SnapError> {
        let mut r = Reader::new(bytes);
        Self::read_snapshot(&mut r).and_then(|world| {
            if r.at_end() {
                Ok(world)
            } else {
                Err(r.error("bytes after the world"))
            }
        })
    }

    /// A world written by [`World::snapshot`] of this or an earlier format
    /// (from [`OLDEST`] on), read from the reader's position (for files that
    /// hold more than the world). The reader stays in that format for what
    /// follows in the file (the computer players). What a default cannot
    /// give a game saved earlier is made here, after `World::get`, for the
    /// formats before the rule's (`docs/snapshot.md`, "Changing the format").
    /// What only this build's rules can give is [`World::take_later_rules`].
    pub fn read_snapshot(r: &mut Reader) -> Result<World, SnapError> {
        if r.raw(MAGIC.len())? != MAGIC {
            return Err(SnapError("not a snapshot of s2-reborn".into()));
        }
        let version = r.u64()?;
        let format = u32::try_from(version)
            .ok()
            .filter(|f| (OLDEST..=VERSION).contains(f))
            .ok_or_else(|| {
                SnapError(format!(
                    "format version {version}, this build reads {OLDEST} to {VERSION}"
                ))
            })?;
        r.format = format;
        let mut world = World::get(r)?;
        // UI-GAME-8.5a, 8.5e: the production statistics of format 66.
        if format < 66 {
            world.statistics_from_format_65();
        }
        // GFX-OBJ-1.4a: the doodads removed, from the fields as they stand.
        if format < 71 {
            world.doodads_removed_from_state();
        }
        // MIL-ATK-2.3 (Q-1070): the attackers out back on their homes' lists.
        if format < 84 {
            world.attackers_back_on_their_lists();
        }
        Ok(world)
    }
}

impl<T: Snap, E: Snap> Snap for Result<T, E> {
    fn put(&self, w: &mut Writer) {
        match self {
            Ok(value) => {
                w.bytes.push(0);
                value.put(w);
            }
            Err(error) => {
                w.bytes.push(1);
                error.put(w);
            }
        }
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        match r.raw(1)?[0] {
            0 => Ok(Ok(T::get(r)?)),
            1 => Ok(Err(E::get(r)?)),
            _ => Err(r.error("not a result")),
        }
    }
    fn shape(s: &mut Shape) -> String {
        format!("Result<{}, {}>", T::shape(s), E::shape(s))
    }
}

impl Snap for () {
    fn put(&self, _: &mut Writer) {}
    fn get(_: &mut Reader) -> Result<Self, SnapError> {
        Ok(())
    }
    fn shape(_: &mut Shape) -> String {
        "()".to_string()
    }
}

/// Message texts are `&'static str` in the state. A text read back is
/// kept once per distinct text for the life of the program, so loading
/// many snapshots does not grow memory.
impl Snap for &'static str {
    fn put(&self, w: &mut Writer) {
        w.u64(self.len() as u64);
        w.raw(self.as_bytes());
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        static TEXTS: std::sync::Mutex<BTreeSet<&'static str>> =
            std::sync::Mutex::new(BTreeSet::new());
        let text = String::get(r)?;
        let mut texts = TEXTS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&known) = texts.get(text.as_str()) {
            return Ok(known);
        }
        let leaked: &'static str = Box::leak(text.into_boxed_str());
        texts.insert(leaked);
        Ok(leaked)
    }
    fn shape(_: &mut Shape) -> String {
        "string".to_string()
    }
}

/// Floats as their exact bits (the computer players' scores are floats; the
/// simulation keeps none in its state).
impl Snap for f32 {
    fn put(&self, w: &mut Writer) {
        w.raw(&self.to_bits().to_le_bytes());
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let bytes = r.raw(4)?;
        Ok(f32::from_bits(u32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }
    fn shape(_: &mut Shape) -> String {
        "f32".to_string()
    }
}

impl Snap for f64 {
    fn put(&self, w: &mut Writer) {
        w.raw(&self.to_bits().to_le_bytes());
    }
    fn get(r: &mut Reader) -> Result<Self, SnapError> {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(r.raw(8)?);
        Ok(f64::from_bits(u64::from_le_bytes(bytes)))
    }
    fn shape(_: &mut Shape) -> String {
        "f64".to_string()
    }
}
