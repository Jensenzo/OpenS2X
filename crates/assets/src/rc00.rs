//! Decoder for the `rc00` file container (`specs/FMT-rc00-container.md`).
//!
//! One wrapped file holds one plain file: a 20-byte header, then an LZSS stream
//! scrambled with a keyed XOR. The key comes from a constant and the file's own
//! base name, so decoding needs the name the game would open the file under.

/// FMT-RC-1.1: first four bytes of every wrapped file.
pub const MAGIC: [u8; 4] = [0x12, 0x18, 0x09, 0x06];
/// FMT-RC-1.1 / FMT-RC-1.4: container tag following the magic.
pub const TAG: [u8; 4] = *b"rc00";
/// FMT-RC-1.1: the body starts at this offset.
pub const HEADER_SIZE: usize = 20;
/// FMT-RC-2.1: length of the file key.
pub const KEY_SIZE: usize = 16;

/// FMT-RC-2.1: key before the file name is mixed in.
const BASE_KEY: [u8; KEY_SIZE] = [
    0xC9, 0x59, 0x46, 0xCA, 0xD9, 0xF0, 0x4F, 0x0A, 0xA1, 0x00, 0xAA, 0xB8, 0xCB, 0xE8, 0xDB, 0x6B,
];
/// FMT-RC-2.3: files with these extensions use the base key unchanged.
const FIXED_KEY_EXTENSIONS: [&str; 2] = [".s2m", ".sav"];

/// FMT-RC-5.1
const RNG_MULTIPLIER: u64 = 16807;
const RNG_MODULUS: u64 = 2_147_483_647;
/// FMT-RC-5.2: bit positions used to repair seeds with too few or too many set bits.
const SEED_FIX_POSITIONS: [u32; 8] = [12, 23, 10, 25, 8, 27, 6, 29];
const SEED_MIN_BITS: u32 = 8;
const SEED_MAX_BITS: u32 = 24;

/// FMT-RC-4.1
const RING_SIZE: usize = 1024;
const RING_FILL: u8 = 0x20;
const RING_START: usize = 0x3F0;
/// FMT-RC-4.4
const MIN_COPY: usize = 3;

/// Why a file could not be decoded. Per FMT-RC-1.7 every one of these is a
/// fatal load error for that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rc00Error {
    /// FMT-RC-1.2: the file does not start with the magic, so it is a plain file.
    NotWrapped,
    /// The file starts with the magic but is shorter than the header
    /// (FMT-RC-1.1). See Q-022.
    Truncated { len: usize },
    /// FMT-RC-1.4 ("FCC mismatch").
    TagMismatch { found: [u8; 4] },
    /// The key depends on a base name with non-ASCII characters, for which the
    /// spec does not define lower-casing or byte encoding (FMT-RC-2.2). See Q-021.
    NonAsciiName,
    /// FMT-RC-1.5 ("pw crc mismatch"): renamed file, or not from this game.
    KeyCrcMismatch { expected: u32, actual: u32 },
    /// FMT-RC-3.6: a zero-length body cannot be descrambled.
    EmptyBody,
    /// FMT-RC-1.6 ("datasize mismatch").
    SizeMismatch { expected: u32, actual: usize },
    /// FMT-RC-1.6 ("data crc mismatch").
    DataCrcMismatch { expected: u32, actual: u32 },
}

impl std::fmt::Display for Rc00Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWrapped => write!(f, "not an rc00 file (magic missing)"),
            Self::Truncated { len } => {
                write!(
                    f,
                    "rc00 file is {len} bytes, shorter than the {HEADER_SIZE}-byte header"
                )
            }
            Self::TagMismatch { found } => write!(f, "tag mismatch: found {found:02X?}"),
            Self::NonAsciiName => write!(f, "cannot derive a key from a non-ASCII file name"),
            Self::KeyCrcMismatch { expected, actual } => write!(
                f,
                "key crc mismatch: header {expected:#010X}, derived {actual:#010X} (renamed file?)"
            ),
            Self::EmptyBody => write!(f, "rc00 file has an empty body"),
            Self::SizeMismatch { expected, actual } => {
                write!(f, "size mismatch: header {expected}, decompressed {actual}")
            }
            Self::DataCrcMismatch { expected, actual } => write!(
                f,
                "data crc mismatch: header {expected:#010X}, decompressed {actual:#010X}"
            ),
        }
    }
}

impl std::error::Error for Rc00Error {}

/// Header fields after the magic and tag (FMT-RC-1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// CRC-32 of the plain contents.
    pub data_crc: u32,
    /// CRC-32 of the 16-byte file key.
    pub key_crc: u32,
    /// Length of the plain contents in bytes.
    pub plain_size: u32,
}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// Standard CRC-32 as used by zlib/PNG/zip (spec "Rules" preamble).
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = CRC_TABLE[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

/// FMT-RC-5: the generator behind key derivation and both XOR passes.
struct Generator {
    state: u32,
}

impl Generator {
    /// FMT-RC-5.2
    fn seeded(seed: u32) -> Self {
        let mut s = seed & 0x7FFF_FFFF;
        let set_bits = s.count_ones();
        if set_bits < SEED_MIN_BITS {
            for &bit in &SEED_FIX_POSITIONS[..(SEED_MIN_BITS - set_bits) as usize] {
                s |= 1 << bit;
            }
        } else if set_bits > SEED_MAX_BITS {
            for &bit in &SEED_FIX_POSITIONS[..(32 - set_bits) as usize] {
                s &= !(1 << bit);
            }
        }
        Self {
            state: if s == 0 { 1 } else { s },
        }
    }

    /// FMT-RC-5.1: "draw a value".
    fn next_value(&mut self) -> u32 {
        // The remainder is below 2^31, so it fits.
        self.state = (u64::from(self.state) * RNG_MULTIPLIER % RNG_MODULUS) as u32;
        self.state
    }

    /// FMT-RC-5.1: "draw a byte".
    fn next_byte(&mut self) -> u8 {
        self.next_value() as u8
    }

    fn next_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_byte()).collect()
    }
}

/// FMT-RC-1.2: whether `data` starts with the container magic.
pub fn is_wrapped(data: &[u8]) -> bool {
    data.starts_with(&MAGIC)
}

/// Read and check the fixed header fields (FMT-RC-1.1, FMT-RC-1.4).
pub fn parse_header(data: &[u8]) -> Result<Header, Rc00Error> {
    if !is_wrapped(data) {
        return Err(Rc00Error::NotWrapped);
    }
    if data.len() < HEADER_SIZE {
        return Err(Rc00Error::Truncated { len: data.len() });
    }
    let field = |offset: usize| {
        [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]
    };
    let tag = field(4);
    if tag != TAG {
        return Err(Rc00Error::TagMismatch { found: tag });
    }
    Ok(Header {
        data_crc: u32::from_le_bytes(field(8)),
        key_crc: u32::from_le_bytes(field(12)),
        plain_size: u32::from_le_bytes(field(16)),
    })
}

/// FMT-RC-2.2: the part of `path` after the last `\` or `/`.
pub fn base_name(path: &str) -> &str {
    match path.rfind(['\\', '/']) {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// Derive the 16-byte file key from the path the file is opened under
/// (FMT-RC-2). Only the base name takes part.
pub fn derive_key(path: &str) -> Result<[u8; KEY_SIZE], Rc00Error> {
    let name = base_name(path).to_ascii_lowercase();
    // FMT-RC-2.3
    if FIXED_KEY_EXTENSIONS.iter().any(|ext| name.ends_with(ext)) {
        return Ok(BASE_KEY);
    }
    if !name.is_ascii() {
        return Err(Rc00Error::NonAsciiName);
    }
    // FMT-RC-2.4
    let mut gen = Generator::seeded(crc32(name.as_bytes()));
    let mut key = BASE_KEY;
    for k in &mut key {
        *k ^= gen.next_byte();
    }
    Ok(key)
}

/// FMT-RC-3.1: pass 1, a repeating pad over the whole body.
fn xor_pass1(gen: &mut Generator, body: &mut [u8]) {
    let n = (gen.next_value() & 0x7F) as usize + 0x80;
    let pad = gen.next_bytes(n);
    for (i, b) in body.iter_mut().enumerate() {
        *b ^= pad[i % n];
    }
}

/// FMT-RC-3.2, FMT-RC-3.3: pass 2, sparse positions a fixed step apart.
/// `body` must not be empty.
fn xor_pass2(gen: &mut Generator, body: &mut [u8], key: &[u8; KEY_SIZE]) {
    let m = (gen.next_value() & 0x0F) + 0x11;
    let pad = gen.next_bytes(m as usize);
    let mut pos = gen.next_value() as usize % body.len();
    let step = (gen.next_value() & 0x1FFF) as usize + 0x2000;
    while pos < body.len() {
        // The position takes part as a 32-bit value.
        let pad_index = (u32::from(key[pos % KEY_SIZE]) ^ pos as u32) % m;
        body[pos] ^= pad[pad_index as usize];
        pos += step;
    }
}

/// Remove (or, per FMT-RC-3.5, apply) the keyed XOR scrambling in place
/// (FMT-RC-3).
pub fn descramble(body: &mut [u8], key: &[u8; KEY_SIZE]) -> Result<(), Rc00Error> {
    // FMT-RC-3.6
    if body.is_empty() {
        return Err(Rc00Error::EmptyBody);
    }
    let mut gen = Generator::seeded(crc32(key));
    xor_pass1(&mut gen, body);
    xor_pass2(&mut gen, body, key);
    Ok(())
}

struct Lzss {
    ring: [u8; RING_SIZE],
    cursor: usize,
    out: Vec<u8>,
}

impl Lzss {
    /// FMT-RC-4.5
    fn emit(&mut self, byte: u8) {
        self.out.push(byte);
        self.ring[self.cursor] = byte;
        self.cursor = (self.cursor + 1) % RING_SIZE;
    }
}

/// Decompress a descrambled body (FMT-RC-4). Never fails: the stream has no
/// end marker and the result is checked by the caller (FMT-RC-1.6).
pub fn decompress(input: &[u8]) -> Vec<u8> {
    // FMT-RC-4.1
    let mut lz = Lzss {
        ring: [RING_FILL; RING_SIZE],
        cursor: RING_START,
        out: Vec::with_capacity(input.len()),
    };
    let mut bytes = input.iter().copied();
    // FMT-RC-4.2; every early return below is FMT-RC-4.6.
    while let Some(flags) = bytes.next() {
        for bit in 0..8 {
            if (flags >> bit) & 1 == 1 {
                // FMT-RC-4.3
                let Some(literal) = bytes.next() else {
                    return lz.out;
                };
                lz.emit(literal);
            } else {
                // FMT-RC-4.4
                let (Some(a), Some(b)) = (bytes.next(), bytes.next()) else {
                    return lz.out;
                };
                let pos = usize::from(a) + usize::from(b & 0xF0) * 16;
                let len = usize::from(b & 0x0F) + MIN_COPY;
                for i in 0..len {
                    lz.emit(lz.ring[(pos + i) % RING_SIZE]);
                }
            }
        }
    }
    lz.out
}

/// Decode a whole wrapped file (FMT-RC-1.3). `path` is the path or name the
/// file is opened under; `data` is the complete file contents.
pub fn decode(path: &str, data: &[u8]) -> Result<Vec<u8>, Rc00Error> {
    let header = parse_header(data)?;
    let key = derive_key(path)?;
    // FMT-RC-1.5
    let key_crc = crc32(&key);
    if key_crc != header.key_crc {
        return Err(Rc00Error::KeyCrcMismatch {
            expected: header.key_crc,
            actual: key_crc,
        });
    }
    let mut body = data[HEADER_SIZE..].to_vec();
    descramble(&mut body, &key)?;
    let plain = decompress(&body);
    // FMT-RC-1.6
    if u32::try_from(plain.len()) != Ok(header.plain_size) {
        return Err(Rc00Error::SizeMismatch {
            expected: header.plain_size,
            actual: plain.len(),
        });
    }
    let data_crc = crc32(&plain);
    if data_crc != header.data_crc {
        return Err(Rc00Error::DataCrcMismatch {
            expected: header.data_crc,
            actual: data_crc,
        });
    }
    Ok(plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREETS_KEY: [u8; KEY_SIZE] = [
        0x4C, 0xE7, 0x37, 0x5B, 0xB2, 0x1B, 0x11, 0x33, 0x76, 0x19, 0x70, 0xEF, 0x80, 0xF5, 0xCA,
        0x37,
    ];

    /// Pass 1 pad length for `key`, read off the period of a scrambled zero body.
    fn pass1_pad_len(key: &[u8; KEY_SIZE]) -> usize {
        let mut gen = Generator::seeded(crc32(key));
        let mut body = [0u8; 1024];
        xor_pass1(&mut gen, &mut body);
        (0x80..=0xFF)
            .find(|&n| (n..body.len()).all(|i| body[i] == body[i - n]))
            .expect("pad length in 128..=255")
    }

    /// Wrap `plain` as an all-literal LZSS stream, scrambled per FMT-RC-3.5.
    fn wrap(name: &str, plain: &[u8]) -> Vec<u8> {
        let key = derive_key(name).unwrap();
        let mut body = Vec::new();
        for group in plain.chunks(8) {
            body.push(0xFF);
            body.extend_from_slice(group);
        }
        descramble(&mut body, &key).unwrap();
        let mut file = Vec::new();
        file.extend_from_slice(&MAGIC);
        file.extend_from_slice(&TAG);
        file.extend_from_slice(&crc32(plain).to_le_bytes());
        file.extend_from_slice(&crc32(&key).to_le_bytes());
        file.extend_from_slice(&(plain.len() as u32).to_le_bytes());
        file.extend_from_slice(&body);
        file
    }

    // spec: FMT-RC-2.4
    #[test]
    fn crc32_is_the_zlib_one() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    // S1
    // spec: FMT-RC-5.1 (S1)
    #[test]
    fn generator_draws() {
        let mut gen = Generator { state: 0x1234_5678 };
        assert_eq!(gen.next_value(), 0x2A18_E99E);
        assert_eq!(gen.next_value(), 0x4991_99A9);
        assert_eq!(gen.next_value(), 0x75FF_4BFA);
    }

    // S1: a seed with 8..=24 set bits is used as is.
    // spec: FMT-RC-5.2 (S1)
    #[test]
    fn seed_valid_is_unchanged() {
        assert_eq!(Generator::seeded(0x1234_5678).state, 0x1234_5678);
    }

    // S2
    // spec: FMT-RC-5.2 (S2)
    #[test]
    fn seed_with_too_few_bits() {
        assert_eq!(Generator::seeded(3).state, 0x0A80_1503);
    }

    // S3
    // spec: FMT-RC-5.2 (S3)
    #[test]
    fn seed_with_too_many_bits() {
        assert_eq!(Generator::seeded(0xFFFF_FFFE).state, 0x7F7F_EFFE);
    }

    // FMT-RC-5.2: the top bit is dropped, and a zero result becomes 1. (Seed 0
    // itself has too few bits and is repaired, so zero is not reachable here;
    // the top-bit case is.)
    #[test]
    fn seed_ignores_top_bit() {
        assert_eq!(
            Generator::seeded(0x9234_5678).state,
            Generator::seeded(0x1234_5678).state
        );
        assert_ne!(Generator::seeded(0).state, 0);
    }

    // S4
    // spec: FMT-RC-2.2, FMT-RC-2.4 (S4)
    #[test]
    fn base_name_is_lowercased_and_stripped() {
        let name = base_name(r"Data\Scripts\Properties\STREETS.LUA");
        assert_eq!(name, "STREETS.LUA");
        assert_eq!(crc32(name.to_ascii_lowercase().as_bytes()), 0x2B29_65CE);
        assert_eq!(crc32(b"streets.lua"), 0x2B29_65CE);
        assert_eq!(base_name("data/scripts/menu.cfg"), "menu.cfg");
        assert_eq!(base_name(r"a/b\c.lua"), "c.lua");
        assert_eq!(base_name("plain.txt"), "plain.txt");
    }

    // S5
    // spec: FMT-RC-2.1, FMT-RC-2.4 (S5)
    #[test]
    fn key_from_name() {
        let key = derive_key("streets.lua").unwrap();
        assert_eq!(key, STREETS_KEY);
        assert_eq!(crc32(&key), 0xF6F4_A1B8);
    }

    // S4, FMT-RC-2.5: directory and case do not take part.
    #[test]
    fn key_ignores_directory_and_case() {
        assert_eq!(
            derive_key(r"Data\Scripts\Properties\STREETS.LUA").unwrap(),
            STREETS_KEY
        );
        assert_eq!(derive_key("elsewhere/streets.lua").unwrap(), STREETS_KEY);
    }

    // S6
    // spec: FMT-RC-3.1 (S6)
    #[test]
    fn pass1_pad_length_for_streets() {
        assert_eq!(pass1_pad_len(&STREETS_KEY), 139);
    }

    // S7, FMT-RC-2.3
    #[test]
    fn maps_and_saves_use_the_base_key() {
        let key = derive_key("campaign01.s2m").unwrap();
        assert_eq!(key, BASE_KEY);
        assert_eq!(crc32(&key), 0xD7E4_A7F3);
        assert_eq!(pass1_pad_len(&key), 153);
        assert_eq!(derive_key("maps/Anything.S2M").unwrap(), BASE_KEY);
        assert_eq!(derive_key("slot1.sav").unwrap(), BASE_KEY);
        // The name does not take part, so non-ASCII is fine here.
        assert_eq!(derive_key("Müller.s2m").unwrap(), BASE_KEY);
    }

    // spec: FMT-RC-2.2
    #[test]
    fn non_ascii_name_is_rejected_when_it_would_be_hashed() {
        assert_eq!(derive_key("straße.lua"), Err(Rc00Error::NonAsciiName));
    }

    // spec: FMT-RC-1.2
    #[test]
    fn magic_detection() {
        assert!(is_wrapped(&[0x12, 0x18, 0x09, 0x06]));
        assert!(is_wrapped(&[0x12, 0x18, 0x09, 0x06, b'r', b'c']));
        assert!(!is_wrapped(&[0x12, 0x18, 0x09]));
        assert!(!is_wrapped(b"[settings]"));
        assert!(!is_wrapped(&[]));
    }

    // FMT-RC-1.1; field values are those of S8.
    #[test]
    fn header_fields() {
        let mut file = Vec::new();
        file.extend_from_slice(&MAGIC);
        file.extend_from_slice(b"rc00");
        file.extend_from_slice(&[0x76, 0xDB, 0x0F, 0x2F]);
        file.extend_from_slice(&[0xB8, 0xA1, 0xF4, 0xF6]);
        file.extend_from_slice(&[229, 0, 0, 0]);
        let expected = Header {
            data_crc: 0x2F0F_DB76,
            key_crc: 0xF6F4_A1B8,
            plain_size: 229,
        };
        assert_eq!(parse_header(&file), Ok(expected));
        file.push(0xAA);
        assert_eq!(parse_header(&file), Ok(expected));
    }

    // spec: FMT-RC-1.1, FMT-RC-1.4, FMT-RC-1.5
    #[test]
    fn header_errors() {
        assert_eq!(parse_header(b""), Err(Rc00Error::NotWrapped));
        assert_eq!(
            parse_header(b"plain text, long enough"),
            Err(Rc00Error::NotWrapped)
        );
        let mut file = wrap("a.lua", b"hello");
        assert_eq!(
            parse_header(&file[..19]),
            Err(Rc00Error::Truncated { len: 19 })
        );
        assert_eq!(
            parse_header(&file[..4]),
            Err(Rc00Error::Truncated { len: 4 })
        );
        // FMT-RC-1.4
        file[7] = b'1';
        assert_eq!(
            parse_header(&file),
            Err(Rc00Error::TagMismatch { found: *b"rc01" })
        );
    }

    // FMT-RC-3.1
    #[test]
    fn pass1_repeats_its_pad_over_the_whole_body() {
        let mut gen = Generator::seeded(crc32(&STREETS_KEY));
        let mut body = [0u8; 600];
        xor_pass1(&mut gen, &mut body);
        assert!((139..600).all(|i| body[i] == body[i - 139]));
        // The pad is the generator's next 139 bytes after the length draw.
        let mut gen = Generator::seeded(crc32(&STREETS_KEY));
        gen.next_value();
        assert_eq!(body[..139], gen.next_bytes(139)[..]);
    }

    // FMT-RC-3.4
    #[test]
    fn pass2_touches_one_byte_of_a_short_body() {
        for len in [1usize, 2, 116, 8191] {
            let mut gen = Generator::seeded(crc32(&STREETS_KEY));
            let mut body = vec![0u8; len];
            xor_pass2(&mut gen, &mut body, &STREETS_KEY);
            // The pad byte can be zero, so "at most one" is all a zero body shows.
            assert!(body.iter().filter(|&&b| b != 0).count() <= 1, "len {len}");
        }
    }

    // FMT-RC-3.2, FMT-RC-3.3
    #[test]
    fn pass2_positions_and_pad_bytes() {
        const LEN: usize = 100_000;
        let key = BASE_KEY;
        let mut gen = Generator::seeded(crc32(&key));
        let mut body = vec![0u8; LEN];
        xor_pass2(&mut gen, &mut body, &key);

        let mut gen = Generator::seeded(crc32(&key));
        let m = (gen.next_value() & 0x0F) + 0x11;
        assert!((17..=32).contains(&m));
        let pad = gen.next_bytes(m as usize);
        let first = gen.next_value() as usize % LEN;
        let step = (gen.next_value() & 0x1FFF) as usize + 0x2000;
        assert!((8192..=16383).contains(&step));

        let mut expected = vec![0u8; LEN];
        for pos in (first..LEN).step_by(step) {
            expected[pos] = pad[((u32::from(key[pos % 16]) ^ pos as u32) % m) as usize];
        }
        assert!((first..LEN).step_by(step).count() >= 6);
        assert_eq!(body, expected);
    }

    // FMT-RC-3.5
    #[test]
    fn descramble_is_its_own_inverse() {
        let original: Vec<u8> = (0..50_000u32).map(|i| (i * 7 + i / 251) as u8).collect();
        let mut body = original.clone();
        descramble(&mut body, &STREETS_KEY).unwrap();
        assert_ne!(body, original);
        descramble(&mut body, &STREETS_KEY).unwrap();
        assert_eq!(body, original);
    }

    // FMT-RC-3.6
    #[test]
    fn empty_body_is_rejected() {
        assert_eq!(descramble(&mut [], &STREETS_KEY), Err(Rc00Error::EmptyBody));
        let file = wrap("a.lua", b"hello");
        assert_eq!(
            decode("a.lua", &file[..HEADER_SIZE]),
            Err(Rc00Error::EmptyBody)
        );
    }

    // FMT-RC-4.2, FMT-RC-4.3
    #[test]
    fn lzss_literals() {
        assert_eq!(decompress(b"\xFFabcdefgh\x03ij"), b"abcdefghij");
        assert_eq!(decompress(b""), b"");
    }

    // FMT-RC-4.1: the ring starts out full of spaces, including 1008..1023.
    #[test]
    fn lzss_copy_from_fresh_ring() {
        assert_eq!(decompress(&[0x00, 0x00, 0x00]), b"   ");
        // pos 0x3F0, len 18: runs over the end of the ring and wraps to 0.
        assert_eq!(decompress(&[0x00, 0xF0, 0x3F]), [b' '; 18]);
    }

    // FMT-RC-4.4, FMT-RC-4.5: positions are absolute and the first output byte
    // lands at 0x3F0; a copy may read what it has just written.
    #[test]
    fn lzss_copy_is_absolute_and_overlapping() {
        // literal 'a', then copy pos 0x3F0 len 5
        assert_eq!(decompress(&[0x01, b'a', 0xF0, 0x32]), b"aaaaaa");
        // literals "ab", then copy pos 0x3F0 len 3, then copy pos 0x3F1 len 4
        assert_eq!(
            decompress(&[0x03, b'a', b'b', 0xF0, 0x30, 0xF1, 0x31]),
            b"ababababa"
        );
    }

    // FMT-RC-4.4: the top two bits of `b` have no effect.
    #[test]
    fn lzss_copy_position_wraps() {
        assert_eq!(decompress(&[0x01, b'a', 0xF0, 0xF2]), b"aaaaaa");
    }

    // FMT-RC-4.5: the cursor wraps from 1023 to 0.
    #[test]
    fn lzss_cursor_wraps() {
        // 16 literals fill 0x3F0..=0x3FF, a 17th lands at position 0.
        let mut input = vec![0xFF];
        input.extend_from_slice(b"01234567");
        input.push(0xFF);
        input.extend_from_slice(b"89abcdef");
        // The copy from position 0 then finds it (and, overlapping, repeats it).
        input.extend_from_slice(&[0x01, b'X', 0x00, 0x00]);
        assert_eq!(decompress(&input), b"0123456789abcdefXXXX");
    }

    // FMT-RC-4.6
    #[test]
    fn lzss_stops_when_input_runs_out() {
        // Mid-group, after a literal.
        assert_eq!(decompress(&[0xFF, b'a']), b"a");
        // Flag byte with nothing after it.
        assert_eq!(
            decompress(&[0xFF, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', 0x00]),
            b"abcdefgh"
        );
        // Half a copy item: the lone byte is dropped.
        assert_eq!(decompress(&[0x01, b'a', 0xF0]), b"a");
    }

    // spec: FMT-RC-1.3, FMT-RC-3.2, FMT-RC-3.5
    #[test]
    fn decode_round_trip() {
        let plain = b"\r\n-------------- Streets\r\n";
        assert_eq!(
            decode("streets.lua", &wrap("streets.lua", plain)).unwrap(),
            plain
        );
        // Body longer than 8192 bytes, so pass 2 takes several steps.
        let big: Vec<u8> = (0..40_000u32).map(|i| (i % 253) as u8).collect();
        assert_eq!(
            decode(r"Maps\Big.S2M", &wrap("big.s2m", &big)).unwrap(),
            big
        );
    }

    // spec: FMT-RC-1.2
    #[test]
    fn decode_rejects_plain_files() {
        assert_eq!(
            decode("a.ini", b"[settings]\r\n"),
            Err(Rc00Error::NotWrapped)
        );
    }

    // S9 in miniature, FMT-RC-1.5
    #[test]
    fn decode_fails_key_check_after_rename() {
        let file = wrap("streets.lua", b"hello");
        assert!(matches!(
            decode("roads.lua", &file),
            Err(Rc00Error::KeyCrcMismatch {
                expected: 0xF6F4_A1B8,
                ..
            })
        ));
    }

    // S10 in miniature, FMT-RC-2.3
    #[test]
    fn decode_survives_renaming_a_map() {
        let file = wrap("campaign01.s2m", b"map bytes");
        assert_eq!(decode("anything.s2m", &file).unwrap(), b"map bytes");
    }

    // FMT-RC-1.6
    #[test]
    fn decode_checks_size_and_data_crc() {
        let good = wrap("a.lua", b"hello world");

        let mut file = good.clone();
        file[16] = 12;
        assert_eq!(
            decode("a.lua", &file),
            Err(Rc00Error::SizeMismatch {
                expected: 12,
                actual: 11
            })
        );

        // Dropping the last body byte loses the last literal.
        assert_eq!(
            decode("a.lua", &good[..good.len() - 1]),
            Err(Rc00Error::SizeMismatch {
                expected: 11,
                actual: 10
            })
        );

        let mut file = good.clone();
        file[8] ^= 1;
        assert!(matches!(
            decode("a.lua", &file),
            Err(Rc00Error::DataCrcMismatch { .. })
        ));
    }

    // FMT-RC-1.3: the tag is checked before the key.
    #[test]
    fn decode_checks_tag_first() {
        let mut file = wrap("a.lua", b"hello");
        file[4] = b'R';
        assert_eq!(
            decode("renamed.lua", &file),
            Err(Rc00Error::TagMismatch { found: *b"Rc00" })
        );
    }

    // No input may panic, whatever follows the header.
    // spec: FMT-RC-4.6
    #[test]
    fn decode_never_panics_on_garbage() {
        let key = derive_key("a.lua").unwrap();
        let mut gen = Generator::seeded(0xDEAD_BEEF);
        for len in [0usize, 1, 2, 3, 17, 255, 256, 9000, 20_000] {
            let mut file = Vec::new();
            file.extend_from_slice(&MAGIC);
            file.extend_from_slice(&TAG);
            file.extend_from_slice(&gen.next_value().to_le_bytes());
            file.extend_from_slice(&crc32(&key).to_le_bytes());
            file.extend_from_slice(&u32::MAX.to_le_bytes());
            file.extend(gen.next_bytes(len));
            assert!(decode("a.lua", &file).is_err(), "len {len}");
        }
    }
}
