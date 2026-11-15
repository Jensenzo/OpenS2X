//! Windows bitmaps, as the game's map previews (`data/maps/*/<map>.bmp`)
//! are stored: uncompressed, 24 or 32 bits per pixel. The standard format;
//! nothing game specific.

use crate::tga::Image;

/// Read an uncompressed 24- or 32-bit bitmap as RGBA rows, the top row
/// first (a positive height means the rows are stored from the bottom).
pub fn read(data: &[u8]) -> Option<Image> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return None;
    }
    let u32_at = |i: usize| u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    let offset = u32_at(10) as usize;
    let width = u32_at(18) as i32;
    let height = u32_at(22) as i32;
    let bits = u16::from_le_bytes([data[28], data[29]]);
    let compression = u32_at(30);
    if width <= 0 || height == 0 || compression != 0 || !matches!(bits, 24 | 32) {
        return None;
    }
    let (w, h) = (width as u32, height.unsigned_abs());
    let bytes = usize::from(bits / 8);
    let stride = (w as usize * bytes).div_ceil(4) * 4;
    // Every pixel read must lie in the file (the last one needs 3 bytes);
    // checked first, so a damaged size allocates nothing.
    let last = (h as usize - 1)
        .checked_mul(stride)
        .and_then(|rows| rows.checked_add((w as usize - 1) * bytes + 3))
        .and_then(|end| end.checked_add(offset))?;
    if last > data.len() {
        return None;
    }
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for y in 0..h as usize {
        let row = if height > 0 { h as usize - 1 - y } else { y };
        let start = offset + row * stride;
        for x in 0..w as usize {
            let s = start + x * bytes;
            let px = data.get(s..s + 3)?;
            let d = (y * w as usize + x) * 4;
            rgba[d..d + 4].copy_from_slice(&[px[2], px[1], px[0], 255]);
        }
    }
    Some(Image {
        width: w,
        height: h,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: FMT-4.2
    #[test]
    fn bottom_up_24_bit_with_padding() {
        // 1 x 2: bottom pixel blue, top pixel red; rows padded to 4 bytes.
        let mut d = vec![0u8; 54];
        d[0..2].copy_from_slice(b"BM");
        d[10..14].copy_from_slice(&54u32.to_le_bytes());
        d[18..22].copy_from_slice(&1u32.to_le_bytes());
        d[22..26].copy_from_slice(&2u32.to_le_bytes());
        d[28..30].copy_from_slice(&24u16.to_le_bytes());
        d.extend([255, 0, 0, 0, 0, 0, 255, 0]);
        let img = read(&d).unwrap();
        assert_eq!(img.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.pixel(0, 1), [0, 0, 255, 255]);
    }
}
