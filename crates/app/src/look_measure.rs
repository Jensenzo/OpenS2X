//! `s2-app look-measure DIR [--masks MASKS]`: the look comparison's measure
//! (F53, `scripts/look-compare.sh`): for every pair `DIR/original/<stem>.png`
//! and `DIR/ours/<stem>.png`, the mean colour of the scene's regions and,
//! where the depth-bin mask `MASKS/mask-<stem>.png` exists, of the water by
//! the depth of the ground under it (`GFX-water.md` GFX-WATER-2.6c, T20),
//! the original's against ours. Prints a Markdown table with the ratio
//! original / ours and two pass columns: every channel within 3 %, and the
//! same with a dark channel (the original's mean below 30 levels) passing
//! within 1 level.
//!
//! Every region and every water sample is clear of the original's
//! interface (2.6c): a region that is not is refused, so no measure takes
//! in the frame, the corner ornaments or the button bar.

use std::fmt::Write as _;
use std::process::ExitCode;

use crate::colour_mean::{mean, Region};
use crate::image_diff::{read_png, Picture};

/// A scene's measured regions: name and x, y, w, h.
pub type Regions = &'static [(&'static str, [u32; 4])];

/// The scene's regions, by the camera's name (the stem's last part): x, y,
/// w, h in a 2560 x 1440 picture. The single "water" box of cell63 is
/// replaced by the depth bins (T20); the farm's "dirt" box is dropped (it
/// measured the doodads' poses and a building, 2026-11-11).
pub const SCENES: [(&str, Regions); 3] = [
    (
        "cell63",
        &[
            ("film", [600, 850, 300, 100]),
            ("sand", [1000, 600, 300, 200]),
            ("rock", [1700, 150, 300, 150]),
        ],
    ),
    (
        "default",
        &[
            ("dome", [1130, 230, 160, 120]),
            ("dirt", [950, 900, 200, 150]),
            ("grass", [1700, 950, 200, 150]),
            ("paving", [1230, 640, 100, 40]),
        ],
    ),
    (
        "farm",
        &[
            ("shadegrass", [650, 950, 200, 120]),
            ("litgrass", [1900, 600, 200, 150]),
        ],
    ),
];

/// T20's depth bins as the masks number them (grey value): the bins of
/// sea ground at or below 0 (1 and 2, above 0, are not water).
pub const BINS: [(u8, &str); 8] = [
    (3, "-0.05 .. 0"),
    (4, "-0.1 .. -0.05"),
    (5, "-0.2 .. -0.1"),
    (6, "-0.5 .. -0.2"),
    (7, "-1 .. -0.5"),
    (8, "-2 .. -1"),
    (9, "-3 .. -2"),
    (10, "below -3"),
];

/// A mask pixel (i, j) is the picture's pixel (4i, 4j) (T20: every 4th
/// pixel in both directions).
const MASK_STEP: u32 = 4;

/// Fewer samples than this in a bin: shown, not judged (T20: "too few").
const FEW: u64 = 50;

/// GFX-WATER-2.6c (E-gfx-049): the original's interface in a picture of
/// `width` x `height` (measured at 2560 x 1440): a frame of 12 pixels on
/// every side, the ornaments' boxes in the two bottom corners (80 wide, 160
/// high) and the button bar's box at the bottom centre (560 wide, 120
/// high), as x0, y0, x1, y1 (end exclusive).
pub fn interface(width: u32, height: u32) -> [[u32; 4]; 7] {
    let (w, h) = (width, height);
    let (cx, frame) = (w / 2, 12);
    [
        [0, 0, w, frame],
        [0, h.saturating_sub(frame), w, h],
        [0, 0, frame, h],
        [w.saturating_sub(frame), 0, w, h],
        [0, h.saturating_sub(160), 80, h],
        [w.saturating_sub(80), h.saturating_sub(160), w, h],
        [cx.saturating_sub(280), h.saturating_sub(120), cx + 280, h],
    ]
}

/// Whether the region (x, y, w, h) touches the original's interface.
pub fn on_interface(r: [u32; 4], width: u32, height: u32) -> bool {
    let [x, y, w, h] = r;
    interface(width, height)
        .iter()
        .any(|&[x0, y0, x1, y1]| x < x1 && x + w > x0 && y < y1 && y + h > y0)
}

/// T20: the mean colour of each depth bin's samples (the picture's pixel
/// (4i, 4j) for the mask's (i, j) holding the bin's number), samples on the
/// interface left out, and their count; in [`BINS`]' order.
pub fn bin_means(picture: &Picture, mask: &Picture) -> Vec<([f64; 3], u64)> {
    let mut sums = vec![([0u64; 3], 0u64); BINS.len()];
    for j in 0..mask.height {
        for i in 0..mask.width {
            let bin = mask.rgba[4 * (j as usize * mask.width as usize + i as usize)];
            let Some(k) = BINS.iter().position(|&(b, _)| b == bin) else {
                continue;
            };
            let (x, y) = (MASK_STEP * i, MASK_STEP * j);
            if x >= picture.width
                || y >= picture.height
                || on_interface([x, y, 1, 1], picture.width, picture.height)
            {
                continue;
            }
            let p = 4 * (y as usize * picture.width as usize + x as usize);
            for c in 0..3 {
                sums[k].0[c] += u64::from(picture.rgba[p + c]);
            }
            sums[k].1 += 1;
        }
    }
    sums.into_iter()
        .map(|(s, n)| (s.map(|v| v as f64 / n.max(1) as f64), n))
        .collect()
}

/// Every channel's ratio original / ours within 0.97 .. 1.03.
pub fn within_3(original: [f64; 3], ours: [f64; 3]) -> bool {
    (0..3).all(|c| (0.97..=1.03).contains(&(original[c] / ours[c].max(1e-9))))
}

/// [`within_3`], except that a channel whose original mean is below 30
/// levels passes within 1 level (2026-11-10, T20).
pub fn within_3_or_level(original: [f64; 3], ours: [f64; 3]) -> bool {
    (0..3).all(|c| {
        if original[c] < 30.0 {
            (original[c] - ours[c]).abs() <= 1.0
        } else {
            (0.97..=1.03).contains(&(original[c] / ours[c].max(1e-9)))
        }
    })
}

/// One measured row: pair, region, the original's mean, ours, samples.
struct Row {
    pair: String,
    region: String,
    original: [f64; 3],
    ours: [f64; 3],
    samples: u64,
}

/// The table of every pair of `dir` (and `masks`, if given).
pub fn measure(dir: &str, masks: Option<&str>) -> Result<String, String> {
    let mut stems: Vec<String> = std::fs::read_dir(format!("{dir}/original"))
        .map_err(|e| format!("{dir}/original: {e}"))?
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".png").map(str::to_string)
        })
        .collect();
    stems.sort();
    let mut rows = Vec::new();
    let mut notes = Vec::new();
    for stem in &stems {
        let ours_file = format!("{dir}/ours/{stem}.png");
        if !std::path::Path::new(&ours_file).exists() {
            notes.push(format!("{stem}: no picture of ours"));
            continue;
        }
        let original = read_png(&format!("{dir}/original/{stem}.png"))?;
        let ours = read_png(&ours_file)?;
        if (original.width, original.height) != (ours.width, ours.height) {
            return Err(format!("{stem}: the pictures differ in size"));
        }
        let scene = stem.rsplit('-').next().unwrap_or_default();
        let Some((_, regions)) = SCENES.iter().find(|(name, _)| *name == scene) else {
            notes.push(format!("{stem}: no regions for the scene '{scene}'"));
            continue;
        };
        for &(name, r) in regions.iter() {
            if on_interface(r, original.width, original.height) {
                return Err(format!(
                    "{stem}: region {name} {r:?} touches the original's interface"
                ));
            }
            let region = Region {
                name: name.to_string(),
                x: r[0],
                y: r[1],
                w: r[2],
                h: r[3],
            };
            let ((a, n), (b, _)) = (mean(&original, &region), mean(&ours, &region));
            rows.push(Row {
                pair: stem.clone(),
                region: name.to_string(),
                original: a,
                ours: b,
                samples: n,
            });
        }
        let mask_file = masks.map(|m| format!("{m}/mask-{stem}.png"));
        match mask_file {
            Some(file) if std::path::Path::new(&file).exists() => {
                let mask = read_png(&file)?;
                let a = bin_means(&original, &mask);
                let b = bin_means(&ours, &mask);
                for (k, &(_, label)) in BINS.iter().enumerate() {
                    rows.push(Row {
                        pair: stem.clone(),
                        region: format!("water {label}"),
                        original: a[k].0,
                        ours: b[k].0,
                        samples: a[k].1,
                    });
                }
            }
            _ if scene == "cell63" => {
                notes.push(format!("{stem}: no depth-bin mask, water not measured"))
            }
            _ => {}
        }
    }
    let mut out = String::new();
    out.push_str(
        "| Pair | Region | Samples | Original | Ours | Original / ours | Within 3 % | Within 3 %, or 1 level where the original is below 30 |\n|---|---|---|---|---|---|---|---|\n",
    );
    let (mut judged, mut strict, mut dark) = (0, 0, 0);
    let rgb = |m: [f64; 3]| format!("{:.1} {:.1} {:.1}", m[0], m[1], m[2]);
    for r in &rows {
        let ratio = (0..3)
            .map(|c| format!("{:.3}", r.original[c] / r.ours[c].max(1e-9)))
            .collect::<Vec<_>>()
            .join(" ");
        let verdict = |pass: bool| if pass { "yes" } else { "**no**" };
        let (a, b) = if r.samples < FEW {
            ("too few", "too few")
        } else {
            judged += 1;
            let (s, d) = (
                within_3(r.original, r.ours),
                within_3_or_level(r.original, r.ours),
            );
            strict += usize::from(s);
            dark += usize::from(d);
            (verdict(s), verdict(d))
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {ratio} | {a} | {b} |",
            r.pair,
            r.region,
            r.samples,
            rgb(r.original),
            rgb(r.ours)
        );
    }
    let _ = writeln!(
        out,
        "\nwithin 3 %: {strict} of {judged}; with the dark bound: {dark} of {judged}"
    );
    for n in notes {
        let _ = writeln!(out, "- {n}");
    }
    Ok(out)
}

pub fn run(args: &[String]) -> ExitCode {
    let (mut dir, mut masks) = (None, None);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--masks" => masks = it.next().cloned(),
            _ => dir = Some(arg.clone()),
        }
    }
    let Some(dir) = dir else {
        eprintln!("look-measure: want DIR [--masks MASKS]");
        return ExitCode::from(2);
    };
    match measure(dir.trim_end_matches('/'), masks.as_deref()) {
        Ok(table) => {
            print!("{table}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("look-measure: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u32, height: u32, at: impl Fn(u32, u32) -> [u8; 3]) -> Picture {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let [r, g, b] = at(x, y);
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
        Picture {
            width,
            height,
            rgba,
        }
    }

    // GFX-WATER-2.6c (E-gfx-049): no region touches the original's
    // interface in the 2560 x 1440 pictures: the 12-pixel frame, the bottom
    // corner boxes (0 .. 80 and 2480 .. 2560 x 1280 .. 1440) and the bar's
    // box (1000 .. 1560 x 1320 .. 1440). The farm has no dirt box any more.
    // spec: GFX-WATER-2.6c
    #[test]
    fn every_region_is_clear_of_the_interface() {
        for (_, regions) in SCENES {
            for &(name, r) in regions {
                assert!(!on_interface(r, 2560, 1440), "{name}");
            }
        }
        let farm = SCENES.iter().find(|(s, _)| *s == "farm").unwrap().1;
        assert!(farm.iter().all(|(name, _)| *name != "dirt"));
        assert!(on_interface([5, 500, 20, 20], 2560, 1440), "the frame");
        assert!(on_interface([2540, 500, 10, 10], 2560, 1440), "the frame");
        assert!(on_interface([60, 1290, 10, 10], 2560, 1440), "a corner");
        assert!(on_interface([2490, 1290, 10, 10], 2560, 1440), "a corner");
        assert!(on_interface([1540, 1300, 30, 30], 2560, 1440), "the bar");
        assert!(!on_interface([1560, 1300, 30, 30], 2560, 1440));
        assert!(!on_interface([100, 1150, 500, 120], 2560, 1440));
    }

    // T20: a bin's samples are the picture's pixels (4i, 4j) where the mask
    // holds its number; 0, 1 and 2 are no water bin; a sample on the
    // interface is left out.
    // spec: GFX-WATER-2.6c (T20)
    #[test]
    fn the_water_is_measured_by_depth_bin() {
        // 400 x 400: the frame, the corner boxes (y from 240) and the bar
        // (x below 480, y from 280) take the edges and the bottom.
        let p = picture(400, 400, |x, y| [(x / 4) as u8, (y / 4) as u8, 7]);
        let mask = picture(100, 100, |i, j| match (i, j) {
            (0, 0) | (10, 10) | (50, 90) => [3; 3],
            (20, 10) => [9; 3],
            (30, 30) => [1; 3],
            _ => [0; 3],
        });
        let bins = bin_means(&p, &mask);
        assert_eq!(bins.len(), 8);
        // (0, 0) is in the frame and (200, 360) under the bar: only the
        // pixel (40, 40) of bin 3 counts.
        assert_eq!(bins[0], ([10.0, 10.0, 7.0], 1));
        assert_eq!(bins[6], ([20.0, 10.0, 7.0], 1));
        assert!(bins
            .iter()
            .enumerate()
            .all(|(k, b)| [0, 6].contains(&k) || b.1 == 0));
    }

    // The two pass columns: every channel within 3 %, and with a dark
    // channel (the original below 30 levels) passing within 1 level.
    // spec: GFX-WATER-2.6c (T20)
    #[test]
    fn a_dark_channel_passes_within_a_level() {
        assert!(within_3([100.0, 50.0, 40.0], [98.0, 51.0, 39.5]));
        assert!(!within_3([100.0, 50.0, 40.0], [95.0, 50.0, 40.0]));
        let (night, ours) = ([9.3, 15.1, 6.5], [9.2, 14.9, 6.3]);
        assert!(!within_3(night, ours));
        assert!(within_3_or_level(night, ours));
        assert!(!within_3_or_level([10.3, 12.6, 21.0], [11.3, 12.8, 22.5]));
        assert!(!within_3_or_level([48.6, 56.3, 17.8], [40.8, 48.0, 13.3]));
    }
}
