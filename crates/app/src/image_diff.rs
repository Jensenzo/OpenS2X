//! `s2-app image-diff EXPECTED ACTUAL [--tolerance T] [--max-share S]
//! [--diff OUT.png]`: compare two pictures pixel by pixel, for the golden
//! images (`scripts/golden-images.sh`, `docs/test-coverage.md`).
//!
//! A pixel differs when any of its colour channels is more than T apart
//! (default 8 of 255). The pictures match when they have the same size and
//! at most the share S of pixels differ (default 0.002, 0.2 %). With
//! `--diff` a picture of the differences is written when they do not match:
//! the expected picture darkened, differing pixels in red.

use std::process::ExitCode;

/// An 8-bit RGBA picture.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// What a comparison found.
#[derive(Debug, Clone, PartialEq)]
pub struct Difference {
    pub differing: u64,
    pub total: u64,
    /// The largest channel difference seen.
    pub largest: u8,
}

impl Difference {
    pub fn share(&self) -> f64 {
        self.differing as f64 / self.total.max(1) as f64
    }
}

pub fn run(args: &[String]) -> ExitCode {
    match image_diff(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("image-diff: {e}");
            ExitCode::from(2)
        }
    }
}

fn image_diff(args: &[String]) -> Result<ExitCode, String> {
    let (Some(expected), Some(actual)) = (args.first(), args.get(1)) else {
        return Err("image-diff wants two PNG files".into());
    };
    let (mut tolerance, mut max_share, mut diff_out) = (8u8, 0.002f64, None);
    let mut it = args[2..].iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(format!("{arg} wants a value"));
        match arg.as_str() {
            "--tolerance" => tolerance = value()?.parse().map_err(|_| "bad --tolerance")?,
            "--max-share" => max_share = value()?.parse().map_err(|_| "bad --max-share")?,
            "--diff" => diff_out = Some(value()?.clone()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let a = read_png(expected)?;
    let b = read_png(actual)?;
    if (a.width, a.height) != (b.width, b.height) {
        println!(
            "{actual}: {} x {}, expected {} x {}",
            b.width, b.height, a.width, a.height
        );
        return Ok(ExitCode::FAILURE);
    }
    let d = compare(&a, &b, tolerance);
    let ok = d.share() <= max_share;
    println!(
        "{actual}: {} of {} pixels differ by more than {tolerance} ({:.4} %, at most {:.4} %), largest difference {}: {}",
        d.differing,
        d.total,
        d.share() * 100.0,
        max_share * 100.0,
        d.largest,
        if ok { "same" } else { "DIFFERENT" }
    );
    if !ok {
        if let Some(out) = diff_out {
            let picture = diff_picture(&a, &b, tolerance);
            std::fs::write(
                &out,
                crate::png_rgba(picture.width, picture.height, &picture.rgba),
            )
            .map_err(|e| format!("{out}: {e}"))?;
            println!("difference picture: {out}");
        }
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

pub fn read_png(file: &str) -> Result<Picture, String> {
    let data = std::fs::File::open(file).map_err(|e| format!("{file}: {e}"))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("{file}: {e}"))?;
    let mut buffer = vec![0; reader.output_buffer_size().ok_or("picture too large")?];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| format!("{file}: {e}"))?;
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buffer
            .chunks(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buffer.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err(format!("{file}: palette not expanded")),
    };
    Ok(Picture {
        width: info.width,
        height: info.height,
        rgba,
    })
}

/// Pixels of two pictures of the same size whose channels differ by more
/// than `tolerance`.
pub fn compare(a: &Picture, b: &Picture, tolerance: u8) -> Difference {
    let mut d = Difference {
        differing: 0,
        total: u64::from(a.width) * u64::from(a.height),
        largest: 0,
    };
    for (p, q) in a.rgba.chunks(4).zip(b.rgba.chunks(4)) {
        let worst = p
            .iter()
            .zip(q)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        d.largest = d.largest.max(worst);
        if worst > tolerance {
            d.differing += 1;
        }
    }
    d
}

/// The expected picture at a third of its brightness, differing pixels red.
pub fn diff_picture(a: &Picture, b: &Picture, tolerance: u8) -> Picture {
    let rgba = a
        .rgba
        .chunks(4)
        .zip(b.rgba.chunks(4))
        .flat_map(|(p, q)| {
            let worst = p
                .iter()
                .zip(q)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            if worst > tolerance {
                [255, 0, 0, 255]
            } else {
                [p[0] / 3, p[1] / 3, p[2] / 3, 255]
            }
        })
        .collect();
    Picture {
        width: a.width,
        height: a.height,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(pixels: &[[u8; 4]]) -> Picture {
        Picture {
            width: pixels.len() as u32,
            height: 1,
            rgba: pixels.concat(),
        }
    }

    // spec: none (the golden image comparison)
    #[test]
    fn pixels_beyond_the_tolerance_differ() {
        let a = picture(&[[10, 10, 10, 255], [0, 0, 0, 255], [9, 9, 9, 255]]);
        let b = picture(&[[18, 10, 10, 255], [0, 0, 9, 255], [9, 9, 9, 255]]);
        let d = compare(&a, &b, 8);
        assert_eq!((d.differing, d.total, d.largest), (1, 3, 9));
        let red = diff_picture(&a, &b, 8);
        assert_eq!(&red.rgba[4..8], &[255, 0, 0, 255]);
        assert_eq!(&red.rgba[0..4], &[3, 3, 3, 255]);
    }

    // spec: none (the golden image comparison)
    #[test]
    fn a_written_picture_reads_back() {
        let dir = std::env::temp_dir().join(format!("s2-image-diff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("p.png");
        let rgba: Vec<u8> = (0..2 * 3 * 4).map(|i| (i * 11) as u8).collect();
        std::fs::write(&file, crate::png_rgba(2, 3, &rgba)).unwrap();
        let p = read_png(file.to_str().unwrap()).unwrap();
        assert_eq!((p.width, p.height), (2, 3));
        assert_eq!(p.rgba, rgba);
        std::fs::remove_dir_all(dir).ok();
    }
}
