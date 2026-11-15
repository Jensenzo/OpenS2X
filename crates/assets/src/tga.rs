//! Truevision TGA images, as the game's settler colour maps and building
//! colour table use them (`specs/GFX-objects.md` GFX-OBJ-2.2, 3): true
//! colour, uncompressed (type 2) or run-length encoded (type 10), 24 or 32
//! bits per pixel, and 8-bit grey (types 3 and 11, the fonts' pages). The
//! standard format; nothing game specific.

/// An image as RGBA rows, the top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TgaError {
    Short,
    Unsupported(String),
}

impl std::fmt::Display for TgaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Short => write!(f, "TGA data ends early"),
            Self::Unsupported(what) => write!(f, "unsupported TGA: {what}"),
        }
    }
}

impl std::error::Error for TgaError {}

/// Read a TGA image (already taken out of its rc00 wrapping).
pub fn read(data: &[u8]) -> Result<Image, TgaError> {
    if data.len() < 18 {
        return Err(TgaError::Short);
    }
    let id_length = usize::from(data[0]);
    let colour_map_type = data[1];
    let image_type = data[2];
    let width = u32::from(u16::from_le_bytes([data[12], data[13]]));
    let height = u32::from(u16::from_le_bytes([data[14], data[15]]));
    let bits = data[16];
    let descriptor = data[17];
    if colour_map_type != 0 || !matches!(image_type, 2 | 3 | 10 | 11) {
        return Err(TgaError::Unsupported(format!(
            "image type {image_type}, colour map {colour_map_type}"
        )));
    }
    let grey = matches!(image_type, 3 | 11);
    let bytes = match bits {
        8 if grey => 1,
        24 => 3,
        32 => 4,
        _ => return Err(TgaError::Unsupported(format!("{bits} bits per pixel"))),
    };
    let count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(count * bytes);
    let mut at = 18 + id_length;
    if matches!(image_type, 2 | 3) {
        let end = at + count * bytes;
        pixels.extend_from_slice(data.get(at..end).ok_or(TgaError::Short)?);
    } else {
        while pixels.len() < count * bytes {
            let header = *data.get(at).ok_or(TgaError::Short)?;
            at += 1;
            let run = usize::from(header & 0x7f) + 1;
            if header & 0x80 != 0 {
                let p = data.get(at..at + bytes).ok_or(TgaError::Short)?;
                for _ in 0..run {
                    pixels.extend_from_slice(p);
                }
                at += bytes;
            } else {
                let p = data.get(at..at + run * bytes).ok_or(TgaError::Short)?;
                pixels.extend_from_slice(p);
                at += run * bytes;
            }
        }
        pixels.truncate(count * bytes);
    }
    // Pixels are stored blue, green, red (, alpha); rows from the bottom
    // unless bit 5 of the descriptor puts the origin at the top.
    let top_first = descriptor & 0x20 != 0;
    let mut rgba = vec![0u8; count * 4];
    for y in 0..height as usize {
        let row = if top_first {
            y
        } else {
            height as usize - 1 - y
        };
        for x in 0..width as usize {
            let s = (row * width as usize + x) * bytes;
            let d = (y * width as usize + x) * 4;
            if grey {
                // A grey page is the glyphs' coverage: white with it as alpha.
                rgba[d..d + 4].copy_from_slice(&[255, 255, 255, pixels[s]]);
                continue;
            }
            rgba[d] = pixels[s + 2];
            rgba[d + 1] = pixels[s + 1];
            rgba[d + 2] = pixels[s];
            rgba[d + 3] = if bytes == 4 { pixels[s + 3] } else { 255 };
        }
    }
    Ok(Image {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(image_type: u8, w: u16, h: u16, bits: u8, descriptor: u8) -> Vec<u8> {
        let mut d = vec![0u8; 18];
        d[2] = image_type;
        d[12..14].copy_from_slice(&w.to_le_bytes());
        d[14..16].copy_from_slice(&h.to_le_bytes());
        d[16] = bits;
        d[17] = descriptor;
        d
    }

    // spec: FMT-4.2
    #[test]
    fn bottom_up_24_bit() {
        let mut d = header(2, 2, 2, 24, 0);
        // Bottom row: blue, green; top row: red, white (stored BGR).
        d.extend([255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
        let img = read(&d).unwrap();
        assert_eq!(img.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.pixel(1, 0), [255, 255, 255, 255]);
        assert_eq!(img.pixel(0, 1), [0, 0, 255, 255]);
        assert_eq!(img.pixel(1, 1), [0, 255, 0, 255]);
    }

    // spec: FMT-4.2
    #[test]
    fn grey_is_coverage() {
        let mut d = header(3, 2, 1, 8, 0x20);
        d.extend([0, 200]);
        let img = read(&d).unwrap();
        assert_eq!(img.pixel(1, 0), [255, 255, 255, 200]);
    }

    // spec: FMT-4.2
    #[test]
    fn run_length_32_bit_top_first() {
        let mut d = header(10, 3, 1, 32, 0x28);
        // A run of 2 of (b 1, g 2, r 3, a 4), then 1 raw pixel.
        d.extend([0x81, 1, 2, 3, 4, 0x00, 9, 8, 7, 6]);
        let img = read(&d).unwrap();
        assert_eq!(img.pixel(1, 0), [3, 2, 1, 4]);
        assert_eq!(img.pixel(2, 0), [7, 8, 9, 6]);
    }
}
