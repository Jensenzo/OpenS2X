//! DirectDraw Surface textures (`.dds`) as the game's models reference them
//! (`specs/GFX-granny-format.md` GFX-GR2-5.4): DXT1, DXT3, DXT5 and
//! uncompressed 32, 16 and 8 bit formats, decoded to RGBA8. The layout is
//! the public DDS file format.

const MAGIC: &[u8; 4] = b"DDS ";
const HEADER_SIZE: usize = 128;
/// DDS_PIXELFORMAT flags.
const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_ALPHA: u32 = 0x2;
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;
const DDPF_LUMINANCE: u32 = 0x2_0000;
/// dwCaps2: a cube map.
const DDSCAPS2_CUBEMAP: u32 = 0x200;

/// How the pixels are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Dxt1,
    Dxt3,
    Dxt5,
    /// Uncompressed, by bit count and channel masks (r, g, b, a).
    Masks {
        bits: u32,
        masks: [u32; 4],
    },
}

/// A decoded texture: the top mip level (of the first face of a cube map).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub mip_levels: u32,
    pub cube_map: bool,
    /// Row by row from the top, 4 bytes per pixel.
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdsError {
    NotDds,
    Truncated,
    UnsupportedFormat(u32),
}

impl std::fmt::Display for DdsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDds => write!(f, "not a DDS file"),
            Self::Truncated => write!(f, "DDS data shorter than its header says"),
            Self::UnsupportedFormat(fourcc) => write!(f, "unsupported DDS format {fourcc:#010x}"),
        }
    }
}

impl std::error::Error for DdsError {}

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().expect("4 bytes"))
}

/// Read a DDS file's header and decode its top mip level.
pub fn decode(data: &[u8]) -> Result<Texture, DdsError> {
    if data.len() < HEADER_SIZE || &data[..4] != MAGIC {
        return Err(DdsError::NotDds);
    }
    let height = u32_at(data, 12);
    let width = u32_at(data, 16);
    let mip_levels = u32_at(data, 28).max(1);
    let pf_flags = u32_at(data, 80);
    let fourcc = u32_at(data, 84);
    let caps2 = u32_at(data, 112);
    let format = if pf_flags & DDPF_FOURCC != 0 {
        match &fourcc.to_le_bytes() {
            b"DXT1" => Format::Dxt1,
            b"DXT3" => Format::Dxt3,
            b"DXT5" => Format::Dxt5,
            _ => return Err(DdsError::UnsupportedFormat(fourcc)),
        }
    } else if pf_flags & (DDPF_RGB | DDPF_LUMINANCE | DDPF_ALPHA) != 0 {
        let alpha = if pf_flags & (DDPF_ALPHAPIXELS | DDPF_ALPHA) != 0 {
            u32_at(data, 104)
        } else {
            0
        };
        Format::Masks {
            bits: u32_at(data, 88),
            masks: [u32_at(data, 92), u32_at(data, 96), u32_at(data, 100), alpha],
        }
    } else {
        return Err(DdsError::UnsupportedFormat(pf_flags));
    };
    let pixels = &data[HEADER_SIZE..];
    let rgba = match format {
        Format::Dxt1 | Format::Dxt3 | Format::Dxt5 => decode_blocks(pixels, width, height, format)?,
        Format::Masks { bits, masks } => decode_masks(pixels, width, height, bits, masks)?,
    };
    Ok(Texture {
        width,
        height,
        format,
        mip_levels,
        cube_map: caps2 & DDSCAPS2_CUBEMAP != 0,
        rgba,
    })
}

/// One decoded mip level: width, height and RGBA rows from the top.
pub type Level = (u32, u32, Vec<u8>);

/// GFX-VIEW-2.6 a: the mip levels a DDS file stores below its top level,
/// decoded as the top level is, largest first. Empty for a file of one
/// level or a cube map; a level the file cuts short ends the list.
pub fn decode_lower_levels(data: &[u8]) -> Result<Vec<Level>, DdsError> {
    let top = decode(data)?;
    if top.cube_map {
        return Ok(Vec::new());
    }
    let size = |w: u32, h: u32| -> usize {
        match top.format {
            Format::Dxt1 => (w.div_ceil(4) * h.div_ceil(4) * 8) as usize,
            Format::Dxt3 | Format::Dxt5 => (w.div_ceil(4) * h.div_ceil(4) * 16) as usize,
            Format::Masks { bits, .. } => (w * h * (bits / 8)) as usize,
        }
    };
    let mut at = HEADER_SIZE + size(top.width, top.height);
    let mut levels = Vec::new();
    for i in 1..top.mip_levels.min(32) {
        let (w, h) = ((top.width >> i).max(1), (top.height >> i).max(1));
        let bytes = size(w, h);
        let Some(pixels) = data.get(at..at + bytes) else {
            break;
        };
        let rgba = match top.format {
            Format::Dxt1 | Format::Dxt3 | Format::Dxt5 => decode_blocks(pixels, w, h, top.format)?,
            Format::Masks { bits, masks } => decode_masks(pixels, w, h, bits, masks)?,
        };
        levels.push((w, h, rgba));
        at += bytes;
        if (w, h) == (1, 1) {
            break;
        }
    }
    Ok(levels)
}

/// A 5:6:5 colour as RGB8.
fn rgb565(c: u16) -> [u8; 3] {
    let r = (c >> 11) & 0x1F;
    let g = (c >> 5) & 0x3F;
    let b = c & 0x1F;
    [
        ((r << 3) | (r >> 2)) as u8,
        ((g << 2) | (g >> 4)) as u8,
        ((b << 3) | (b >> 2)) as u8,
    ]
}

/// The four colours of a colour block; `opaque` for DXT3/5, which always
/// use the four-colour mode.
fn palette(c0: u16, c1: u16, opaque: bool) -> [[u8; 4]; 4] {
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mix = |wa: u32, wb: u32, d: u32| {
        [0, 1, 2].map(|i| ((u32::from(a[i]) * wa + u32::from(b[i]) * wb) / d) as u8)
    };
    let rgba = |c: [u8; 3], alpha: u8| [c[0], c[1], c[2], alpha];
    if c0 > c1 || opaque {
        [
            rgba(a, 255),
            rgba(b, 255),
            rgba(mix(2, 1, 3), 255),
            rgba(mix(1, 2, 3), 255),
        ]
    } else {
        [
            rgba(a, 255),
            rgba(b, 255),
            rgba(mix(1, 1, 2), 255),
            [0, 0, 0, 0],
        ]
    }
}

fn decode_blocks(
    data: &[u8],
    width: u32,
    height: u32,
    format: Format,
) -> Result<Vec<u8>, DdsError> {
    let block_size = if format == Format::Dxt1 { 8 } else { 16 };
    let (bw, bh) = (width.div_ceil(4) as usize, height.div_ceil(4) as usize);
    if data.len() < bw * bh * block_size {
        return Err(DdsError::Truncated);
    }
    let mut out = vec![0u8; (width * height * 4) as usize];
    for by in 0..bh {
        for bx in 0..bw {
            let block = &data[(by * bw + bx) * block_size..][..block_size];
            let colour = &block[block_size - 8..];
            let c0 = u16::from_le_bytes([colour[0], colour[1]]);
            let c1 = u16::from_le_bytes([colour[2], colour[3]]);
            let colours = palette(c0, c1, format != Format::Dxt1);
            let bits = u32_at(colour, 4);
            let alphas = alpha_block(block, format);
            for i in 0..16 {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x >= width as usize || y >= height as usize {
                    continue;
                }
                let mut p = colours[((bits >> (2 * i)) & 3) as usize];
                if let Some(alpha) = alphas {
                    p[3] = alpha[i];
                }
                let at = (y * width as usize + x) * 4;
                out[at..at + 4].copy_from_slice(&p);
            }
        }
    }
    Ok(out)
}

/// The 16 alpha values of a DXT3 or DXT5 block.
fn alpha_block(block: &[u8], format: Format) -> Option<[u8; 16]> {
    match format {
        Format::Dxt3 => {
            let bits = u64::from_le_bytes(block[..8].try_into().expect("8"));
            Some(std::array::from_fn(|i| {
                ((bits >> (4 * i)) & 0xF) as u8 * 17
            }))
        }
        Format::Dxt5 => {
            let (a0, a1) = (u32::from(block[0]), u32::from(block[1]));
            let mut levels = [0u32; 8];
            levels[0] = a0;
            levels[1] = a1;
            if a0 > a1 {
                for i in 1..7 {
                    levels[i + 1] = ((7 - i as u32) * a0 + i as u32 * a1) / 7;
                }
            } else {
                for i in 1..5 {
                    levels[i + 1] = ((5 - i as u32) * a0 + i as u32 * a1) / 5;
                }
                levels[6] = 0;
                levels[7] = 255;
            }
            let mut bits = 0u64;
            for (i, &b) in block[2..8].iter().enumerate() {
                bits |= u64::from(b) << (8 * i);
            }
            Some(std::array::from_fn(|i| {
                levels[((bits >> (3 * i)) & 7) as usize] as u8
            }))
        }
        _ => None,
    }
}

/// One channel by its mask, scaled to 8 bits; `default` without a mask.
fn channel(pixel: u32, mask: u32, default: u8) -> u8 {
    if mask == 0 {
        return default;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    (((pixel & mask) >> shift) * 255 / max) as u8
}

fn decode_masks(
    data: &[u8],
    width: u32,
    height: u32,
    bits: u32,
    masks: [u32; 4],
) -> Result<Vec<u8>, DdsError> {
    let bytes = (bits / 8) as usize;
    if !(1..=4).contains(&bytes) {
        return Err(DdsError::UnsupportedFormat(bits));
    }
    let count = (width * height) as usize;
    if data.len() < count * bytes {
        return Err(DdsError::Truncated);
    }
    let mut out = Vec::with_capacity(count * 4);
    let luminance = masks[1] == 0 && masks[2] == 0 && masks[0] != 0;
    for p in data[..count * bytes].chunks_exact(bytes) {
        let mut v = 0u32;
        for (i, &b) in p.iter().enumerate() {
            v |= u32::from(b) << (8 * i);
        }
        let a = channel(v, masks[3], 255);
        if luminance {
            let l = channel(v, masks[0], 0);
            out.extend_from_slice(&[l, l, l, a]);
        } else if masks[..3] == [0, 0, 0] {
            out.extend_from_slice(&[255, 255, 255, a]);
        } else {
            out.extend_from_slice(&[
                channel(v, masks[0], 0),
                channel(v, masks[1], 0),
                channel(v, masks[2], 0),
                a,
            ]);
        }
    }
    Ok(out)
}

/// Read a DDS file as found on disk (rc00-wrapped or not, FMT-RC-1.2).
pub fn load(path: &str, file: &[u8]) -> Result<Texture, Box<dyn std::error::Error>> {
    Ok(decode(&unwrap(path, file)?)?)
}

/// [`load`] with the lower mip levels the file stores
/// ([`decode_lower_levels`]).
pub fn load_with_levels(
    path: &str,
    file: &[u8],
) -> Result<(Texture, Vec<Level>), Box<dyn std::error::Error>> {
    let data = unwrap(path, file)?;
    Ok((decode(&data)?, decode_lower_levels(&data)?))
}

fn unwrap(path: &str, file: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(if crate::rc00::is_wrapped(file) {
        crate::rc00::decode(path, file)?
    } else {
        file.to_vec()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(
        width: u32,
        height: u32,
        pf_flags: u32,
        fourcc: &[u8; 4],
        bits: u32,
        masks: [u32; 4],
    ) -> Vec<u8> {
        let mut h = vec![0u8; HEADER_SIZE];
        h[..4].copy_from_slice(MAGIC);
        let mut put = |at: usize, v: u32| h[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(4, 124);
        put(12, height);
        put(16, width);
        put(76, 32);
        put(80, pf_flags);
        put(84, u32::from_le_bytes(*fourcc));
        put(88, bits);
        for (i, m) in masks.iter().enumerate() {
            put(92 + 4 * i, *m);
        }
        h
    }

    // spec: FMT-3.9, GFX-GR2-5.4
    #[test]
    fn dxt1_block() {
        let mut d = header(4, 4, DDPF_FOURCC, b"DXT1", 0, [0; 4]);
        // Colour 0 pure red, colour 1 pure blue; pixel 0 -> 0, pixel 1 -> 1,
        // pixel 2 -> 2/3 red + 1/3 blue.
        d.extend_from_slice(&0xF800u16.to_le_bytes());
        d.extend_from_slice(&0x001Fu16.to_le_bytes());
        d.extend_from_slice(&0b10_01_00u32.to_le_bytes());
        let t = decode(&d).unwrap();
        assert_eq!((t.width, t.height, t.format), (4, 4, Format::Dxt1));
        assert_eq!(&t.rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&t.rgba[4..8], &[0, 0, 255, 255]);
        assert_eq!(&t.rgba[8..12], &[170, 0, 85, 255]);
    }

    // GFX-VIEW-2.6 a: the file's own lower mip levels, decoded; a level cut
    // short ends the list, a single-level file has none.
    // spec: GFX-VIEW-2.6
    #[test]
    fn lower_mip_levels_as_stored() {
        let block = |c0: u16, c1: u16, bits: u32| {
            let mut b = c0.to_le_bytes().to_vec();
            b.extend_from_slice(&c1.to_le_bytes());
            b.extend_from_slice(&bits.to_le_bytes());
            b
        };
        let mut d = header(8, 8, DDPF_FOURCC, b"DXT1", 0, [0; 4]);
        d[28..32].copy_from_slice(&4u32.to_le_bytes());
        // 8 x 8: four red blocks; 4 x 4: one blue; 2 x 2: one green
        // block, 1 x 1: transparent black (DXT1's fourth colour).
        for _ in 0..4 {
            d.extend(block(0xF800, 0xF800, 0));
        }
        d.extend(block(0x001F, 0x001F, 0));
        d.extend(block(0x07E0, 0x07E0, 0));
        d.extend(block(0x0000, 0xFFFF, 3));
        let levels = decode_lower_levels(&d).unwrap();
        let sizes: Vec<_> = levels.iter().map(|l| (l.0, l.1, l.2.len())).collect();
        assert_eq!(sizes, [(4, 4, 64), (2, 2, 16), (1, 1, 4)]);
        assert_eq!(&levels[0].2[..4], &[0, 0, 255, 255]);
        assert_eq!(&levels[1].2[..4], &[0, 255, 0, 255]);
        assert_eq!(&levels[2].2[..4], &[0, 0, 0, 0]);
        // Cut short: the 1 x 1 level missing.
        let short = &d[..d.len() - 8];
        assert_eq!(decode_lower_levels(short).unwrap().len(), 2);
        // One level.
        d[28..32].copy_from_slice(&1u32.to_le_bytes());
        assert!(decode_lower_levels(&d).unwrap().is_empty());
    }

    // spec: FMT-3.9, GFX-GR2-5.4
    #[test]
    fn dxt1_transparent_and_dxt5_alpha() {
        let mut d = header(4, 4, DDPF_FOURCC, b"DXT1", 0, [0; 4]);
        d.extend_from_slice(&0x0000u16.to_le_bytes());
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        d.extend_from_slice(&3u32.to_le_bytes());
        assert_eq!(&decode(&d).unwrap().rgba[0..4], &[0, 0, 0, 0]);

        let mut d = header(4, 4, DDPF_FOURCC, b"DXT5", 0, [0; 4]);
        // Alpha 255 and 0, pixel 0 -> level 0, pixel 1 -> level 1.
        d.extend_from_slice(&[255, 0, 0b001_000, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]);
        let t = decode(&d).unwrap();
        assert_eq!((t.rgba[3], t.rgba[7]), (255, 0));
        assert_eq!(&t.rgba[0..3], &[255, 255, 255]);
    }

    // spec: FMT-3.9, GFX-GR2-5.4
    #[test]
    fn uncompressed_formats() {
        // A8R8G8B8
        let mut d = header(
            1,
            1,
            DDPF_RGB | DDPF_ALPHAPIXELS,
            &[0; 4],
            32,
            [0xFF0000, 0xFF00, 0xFF, 0xFF00_0000],
        );
        d.extend_from_slice(&0x8011_2233u32.to_le_bytes());
        assert_eq!(decode(&d).unwrap().rgba, [0x11, 0x22, 0x33, 0x80]);
        // A1R5G5B5
        let mut d = header(
            1,
            1,
            DDPF_RGB | DDPF_ALPHAPIXELS,
            &[0; 4],
            16,
            [0x7C00, 0x3E0, 0x1F, 0x8000],
        );
        d.extend_from_slice(&0xFC00u16.to_le_bytes());
        assert_eq!(decode(&d).unwrap().rgba, [255, 0, 0, 255]);
        // R5G6B5 without alpha
        let mut d = header(1, 1, DDPF_RGB, &[0; 4], 16, [0xF800, 0x7E0, 0x1F, 0]);
        d.extend_from_slice(&0x07E0u16.to_le_bytes());
        assert_eq!(decode(&d).unwrap().rgba, [0, 255, 0, 255]);
        // L8
        let mut d = header(1, 1, DDPF_LUMINANCE, &[0; 4], 8, [0xFF, 0, 0, 0]);
        d.push(0x40);
        assert_eq!(decode(&d).unwrap().rgba, [0x40, 0x40, 0x40, 255]);
    }

    // spec: FMT-3.9
    #[test]
    fn errors() {
        assert_eq!(decode(b"not a dds"), Err(DdsError::NotDds));
        let d = header(8, 8, DDPF_FOURCC, b"DXT1", 0, [0; 4]);
        assert_eq!(decode(&d), Err(DdsError::Truncated));
        let d = header(4, 4, DDPF_FOURCC, b"ATI2", 0, [0; 4]);
        assert!(matches!(decode(&d), Err(DdsError::UnsupportedFormat(_))));
    }
}
