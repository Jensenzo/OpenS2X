//! `s2-app colour-mean A.png [B.png] NAME=X,Y,W,H... [--mark OUT.png]`: the
//! mean colour of rectangles of a picture, for the look comparison
//! (`scripts/look-compare.sh`, F53): the same regions of the original's
//! picture and ours, channel by channel.
//!
//! Prints one line per region and picture: the mean red, green and blue
//! (0..255) and the pixel count. With two pictures it also prints the ratio
//! of A's mean to B's per channel (the gain that would take B to A). With
//! `--mark` the first picture is written with the rectangles outlined, to
//! check where they sit.

use std::process::ExitCode;

use crate::image_diff::{read_png, Picture};

/// A named rectangle of a picture, in pixels from the top left.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Region {
    /// `NAME=X,Y,W,H`.
    pub fn parse(s: &str) -> Result<Region, String> {
        let (name, rect) = s.split_once('=').ok_or(format!("{s}: want NAME=X,Y,W,H"))?;
        let v: Vec<u32> = rect
            .split(',')
            .map(|n| n.trim().parse::<u32>())
            .collect::<Result<_, _>>()
            .map_err(|_| format!("{s}: want NAME=X,Y,W,H"))?;
        let [x, y, w, h] = v[..] else {
            return Err(format!("{s}: want NAME=X,Y,W,H"));
        };
        Ok(Region {
            name: name.to_string(),
            x,
            y,
            w,
            h,
        })
    }
}

/// Mean red, green and blue of a region (clipped to the picture) and its
/// pixel count.
pub fn mean(picture: &Picture, r: &Region) -> ([f64; 3], u64) {
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    let x1 = (r.x + r.w).min(picture.width);
    let y1 = (r.y + r.h).min(picture.height);
    for y in r.y.min(y1)..y1 {
        for x in r.x.min(x1)..x1 {
            let i = 4 * (y as usize * picture.width as usize + x as usize);
            for (c, s) in sum.iter_mut().enumerate() {
                *s += u64::from(picture.rgba[i + c]);
            }
            n += 1;
        }
    }
    let m = sum.map(|s| s as f64 / n.max(1) as f64);
    (m, n)
}

/// The picture with each region's outline drawn in magenta, 2 pixels wide.
pub fn marked(picture: &Picture, regions: &[Region]) -> Picture {
    let mut out = Picture {
        width: picture.width,
        height: picture.height,
        rgba: picture.rgba.clone(),
    };
    for r in regions {
        let (x1, y1) = (r.x + r.w, r.y + r.h);
        for y in r.y..y1.min(picture.height) {
            for x in r.x..x1.min(picture.width) {
                let edge = x < r.x + 2 || x + 2 >= x1 || y < r.y + 2 || y + 2 >= y1;
                if edge {
                    let i = 4 * (y as usize * picture.width as usize + x as usize);
                    out.rgba[i..i + 4].copy_from_slice(&[255, 0, 255, 255]);
                }
            }
        }
    }
    out
}

pub fn run(args: &[String]) -> ExitCode {
    match colour_mean(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("colour-mean: {e}");
            ExitCode::from(2)
        }
    }
}

fn colour_mean(args: &[String]) -> Result<(), String> {
    let (mut files, mut regions, mut mark) = (Vec::new(), Vec::new(), None);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--mark" {
            mark = Some(it.next().ok_or("--mark wants a file")?.clone());
        } else if arg.contains('=') {
            regions.push(Region::parse(arg)?);
        } else {
            files.push(arg.clone());
        }
    }
    if files.is_empty() || files.len() > 2 || regions.is_empty() {
        return Err("want one or two PNG files and at least one NAME=X,Y,W,H".into());
    }
    let pictures = files
        .iter()
        .map(|f| read_png(f))
        .collect::<Result<Vec<_>, _>>()?;
    for r in &regions {
        let means: Vec<_> = pictures.iter().map(|p| mean(p, r)).collect();
        for ((m, n), file) in means.iter().zip(&files) {
            println!(
                "{:<12} {:>6.1} {:>6.1} {:>6.1}  {n:>7} px  {file}",
                r.name, m[0], m[1], m[2]
            );
        }
        if let [(a, _), (b, _)] = means[..] {
            println!(
                "{:<12} {:>6.3} {:>6.3} {:>6.3}  ratio first / second",
                r.name,
                a[0] / b[0].max(1e-9),
                a[1] / b[1].max(1e-9),
                a[2] / b[2].max(1e-9)
            );
        }
    }
    if let Some(out) = mark {
        let m = marked(&pictures[0], &regions);
        std::fs::write(&out, crate::png_rgba(m.width, m.height, &m.rgba))
            .map_err(|e| format!("{out}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the look comparison's measuring tool)
    #[test]
    fn a_region_mean_is_per_channel_and_clipped() {
        let p = Picture {
            width: 2,
            height: 2,
            rgba: [
                [10, 20, 30, 255],
                [30, 40, 50, 255],
                [0, 0, 0, 255],
                [0, 0, 0, 255],
            ]
            .concat(),
        };
        let top = Region::parse("top=0,0,5,1").unwrap();
        assert_eq!(mean(&p, &top), ([20.0, 30.0, 40.0], 2));
        assert!(Region::parse("bad=1,2,3").is_err());
        let m = marked(&p, &[Region::parse("all=0,0,2,2").unwrap()]);
        assert_eq!(&m.rgba[0..4], &[255, 0, 255, 255]);
    }
}
