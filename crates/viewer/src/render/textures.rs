//! Texture data for the 3D view: decoded RGBA8 images resampled to one size
//! so they can share an array texture, and their mip chains. Display
//! choices, not game rules: the original keeps one texture per pattern.

use s2_assets::dds::Texture;

/// `texture` resampled to `size` x `size` with bilinear filtering, wrapping
/// at the edges (the patterns repeat, GFX-PAT-3.1).
pub fn resample(texture: &Texture, size: u32) -> Vec<u8> {
    resample_2d(texture, size, size)
}

/// [`resample`] to `width` x `height`.
pub fn resample_2d(texture: &Texture, width: u32, height: u32) -> Vec<u8> {
    let (w, h) = (texture.width as usize, texture.height as usize);
    let (nw, nh) = (width as usize, height as usize);
    let mut out = vec![0u8; nw * nh * 4];
    if w == 0 || h == 0 || texture.rgba.len() < w * h * 4 {
        return out;
    }
    let texel =
        |x: usize, y: usize, c: usize| f32::from(texture.rgba[((y % h) * w + x % w) * 4 + c]);
    for oy in 0..nh {
        let fy = (oy as f32 + 0.5) * h as f32 / nh as f32 - 0.5;
        let y0 = fy.floor();
        let ty = fy - y0;
        let y0 = y0.rem_euclid(h as f32) as usize;
        for ox in 0..nw {
            let fx = (ox as f32 + 0.5) * w as f32 / nw as f32 - 0.5;
            let x0 = fx.floor();
            let tx = fx - x0;
            let x0 = x0.rem_euclid(w as f32) as usize;
            for c in 0..4 {
                let top = texel(x0, y0, c) * (1.0 - tx) + texel(x0 + 1, y0, c) * tx;
                let bottom = texel(x0, y0 + 1, c) * (1.0 - tx) + texel(x0 + 1, y0 + 1, c) * tx;
                out[(oy * nw + ox) * 4 + c] = (top * (1.0 - ty) + bottom * ty + 0.5) as u8;
            }
        }
    }
    out
}

/// The mip levels of a square RGBA8 image of `size` (a power of two), each a
/// 2 x 2 box filter of the one before, from the image itself down to 1 x 1,
/// concatenated.
pub fn mip_chain(rgba: &[u8], size: u32) -> (Vec<u8>, u32) {
    mip_chain_2d(rgba, size, size)
}

/// [`mip_chain`] of a `width` x `height` image (powers of two); a side
/// that reaches 1 stays 1.
pub fn mip_chain_2d(rgba: &[u8], width: u32, height: u32) -> (Vec<u8>, u32) {
    let mut out = rgba.to_vec();
    let mut level = rgba.to_vec();
    let (mut w, mut h) = (width as usize, height as usize);
    let mut levels = 1;
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (2 * x + dx).min(w - 1);
                        let sy = (2 * y + dy).min(h - 1);
                        sum += u32::from(level[(sy * w + sx) * 4 + c]);
                    }
                    next[(y * nw + x) * 4 + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        out.extend_from_slice(&next);
        level = next;
        (w, h) = (nw, nh);
        levels += 1;
    }
    (out, levels)
}

/// A square image of one colour.
pub fn solid(size: u32, rgba: [u8; 4]) -> Vec<u8> {
    rgba.repeat((size * size) as usize)
}

/// GFX-VIEW-2.4b: a pattern's normal map as a whole mip chain of `size` x
/// `size` (a power of two), top level first: the file's own levels when the
/// map is that size and the file stores every level down to 1 x 1, else the
/// map resampled to `size` and box-filtered down ([`mip_chain`]).
pub fn chain_at(texture: &Texture, levels: &[s2_assets::dds::Level], size: u32) -> Vec<u8> {
    let own = texture.width == size
        && texture.height == size
        && levels.len() as u32 == size.ilog2()
        && levels.iter().enumerate().all(|(i, (w, h, rgba))| {
            let side = size >> (i + 1);
            (*w, *h) == (side, side) && rgba.len() == (side * side * 4) as usize
        })
        && texture.rgba.len() == (size * size * 4) as usize;
    if own {
        let mut chain = texture.rgba.clone();
        for (_, _, rgba) in levels {
            chain.extend_from_slice(rgba);
        }
        chain
    } else if (texture.width, texture.height) == (size, size) {
        mip_chain(&texture.rgba, size).0
    } else {
        mip_chain(&resample(texture, size), size).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_assets::dds::Format;

    fn texture(width: u32, height: u32, rgba: Vec<u8>) -> Texture {
        Texture {
            width,
            height,
            format: Format::Dxt1,
            mip_levels: 1,
            cube_map: false,
            rgba,
        }
    }

    /// GFX-WATER-2.3 (Q-1018): the sea-ground
    /// textures under the water decode to the means the investigation took
    /// from the original's files (z_seaground_00 158 / 155 / 112, _01
    /// 79 / 81 / 56, _03 35 / 40 / 24), also after our resample to the
    /// layer size. Skipped without `S2_GAME_DIR`.
    // spec: GFX-WATER-2.3, GFX-PAT-3.1
    #[test]
    fn the_sea_ground_textures_decode_to_the_originals_means() {
        let Ok(dir) = std::env::var("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let mean = |rgba: &[u8]| {
            let n = (rgba.len() / 4) as f64;
            [0, 1, 2].map(|c| {
                rgba.iter()
                    .skip(c)
                    .step_by(4)
                    .map(|&v| f64::from(v))
                    .sum::<f64>()
                    / n
            })
        };
        for (name, want) in [
            ("z_seaground_00", [158.0, 155.0, 112.0]),
            ("z_seaground_01", [79.0, 81.0, 56.0]),
            ("z_seaground_03", [35.0, 40.0, 24.0]),
        ] {
            let path = std::path::Path::new(&dir).join(format!("data/map/textures/{name}.dds"));
            let data = std::fs::read(&path).expect("the texture");
            let texture = s2_assets::dds::load(&path.to_string_lossy(), &data).expect("decodes");
            for got in [mean(&texture.rgba), mean(&resample(&texture, 512))] {
                assert!(
                    (0..3).all(|c| (got[c] - want[c]).abs() <= 1.0),
                    "{name}: {got:?} against {want:?}"
                );
            }
        }
    }

    // spec: none (texture resampling of our renderer)
    #[test]
    fn resample_keeps_a_flat_colour_and_wraps() {
        let flat = texture(2, 2, [10, 20, 30, 255].repeat(4));
        assert_eq!(resample(&flat, 4), [10, 20, 30, 255].repeat(16));
        // Two columns, black and white: wrapping blends both edges evenly.
        let stripes = texture(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]);
        let out = resample(&stripes, 2);
        assert_eq!(out[0], 0);
        assert_eq!(out[4], 255);
    }

    // spec: none (mip chains of our renderer)
    #[test]
    fn mip_chain_halves_to_one_texel() {
        let image: Vec<u8> = (0..16).flat_map(|i| [i * 16, 0, 0, 255]).collect();
        let (chain, levels) = mip_chain(&image, 4);
        assert_eq!(levels, 3);
        assert_eq!(chain.len(), (16 + 4 + 1) * 4);
        // The last level is the mean of everything.
        let mean = (0..16u32).map(|i| i * 16).sum::<u32>() as f32 / 16.0;
        assert!((f32::from(chain[chain.len() - 4]) - mean).abs() <= 1.0);
        // A 4 x 1 image: 4, 2, 1 texels.
        let (chain, levels) = mip_chain_2d(&[255u8; 16], 4, 1);
        assert_eq!((levels, chain.len()), (3, (4 + 2 + 1) * 4));
    }
}
