//! A development switch, not a hook: the looks of the materials of the
//! look study, round 2 (`--look-study origtoy` and `--look-study cel2`, off
//! by default; `look_study.rs`). Unlike the first round's looks, which turn
//! the finished picture, these change the surfaces and the light:
//!
//! - **per model, when it is loaded** (`reshape`): every face of an
//!   original model takes one flat colour, the mean of its own texture
//!   under it, snapped to a small palette of the model's texture and
//!   cleaned (more colour, lifted); a face takes the palette colour most of
//!   its neighbours have, so dense meshes do not break into random facets.
//!   Warm sloped faces of a building are marked as roof (drawn tile rows).
//!   `cel2` adds a shell copy of each model (outlines);
//! - **in the shaders** (shader defs `LOOK_TOY`, `LOOK_CEL` in `object.wesl`,
//!   `terrain.wesl`, `water.wesl`; with the switch off they are compiled
//!   out): `origtoy` a soft warm wrapped light, a clear-coat gloss and a
//!   rim; `cel2` the light in two steps with a cool shadow tone, the shells
//!   pushed out a pixel or two on the screen and drawn dark; the settlers'
//!   colours flat from their own colour map, the ground in the look's own
//!   palette (the ground study's `calm` cut), the water glossy toy blue or
//!   flat with drawn foam.
//!
//! Nothing is written to disk. No rule of the original's is involved.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bevy::render::render_resource::RenderPipelineDescriptor;

use super::look_study::Materials;

/// The look of the materials, when one is on.
pub fn active() -> Option<Materials> {
    super::look_study::materials()
}

/// The look's shader def on a pipeline (none with the switch off: the
/// shaders are then compiled as ever).
pub fn add_defs(descriptor: &mut RenderPipelineDescriptor) {
    let Some(m) = active() else {
        return;
    };
    let def = if m.toy { "LOOK_TOY" } else { "LOOK_CEL" };
    descriptor.vertex.shader_defs.push(def.into());
    if let Some(fragment) = descriptor.fragment.as_mut() {
        fragment.shader_defs.push(def.into());
    }
}

/// The ground of the look: the ground study's `calm` cut (the kinds mixed
/// softly, lit by the smooth normal) in the look's own colours.
pub fn ground(m: Materials) -> super::ground_study::Study {
    // Each line: the swatch, its origtoy colour, its cel2 colour: the
    // study's toy board (#58B044 .. #6EC050 meadow, #F5DCA0 sand, #DDA868
    // road) and cel ground (#78C850 .. #8DD460, #F8E6B0, #DDAE72), the
    // other kinds placed among them.
    const SWATCHES: &str = "
turf            #62B84A #7CCA52
turf_dry        #9CBE56 #A8D264
leaf            #4CA846 #66C04E
leaf_light      #6EC050 #8DD460
leaf_olive      #8AAA4A #9CC25A
grain           #D8C060 #E8D070
earth           #CDB487 #D9C08C
earth_dark      #B09A70 #BBA374
sand            #F2D9A0 #F8E6B0
seabed          #E4CE96 #F0DCA4
rock            #B8B0A0 #C6BEB0
rock_dark       #8E8678 #9E968A
rock_light      #D2CAB6 #DED6C4
stone_dark      #8A8A92 #9A9AA4
sandstone       #D49C64 #E0AA70
snow            #F2F4F8 #F6F8FC
soot            #4A4650 #5A5662
ember           #E86434 #F07040
charcoal_glint  #5C5A62 #6C6A74
water_pale      #7CCAE0 #8AD4F4
water           #2E9EC4 #3AA2E8
water_deep      #1E7CA8 #2A86CC
";
    let hex = |text: &str| -> Option<[f32; 3]> {
        let hex = text.strip_prefix('#')?;
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        Some([byte(0)?, byte(2)?, byte(4)?].map(|c| f32::from(c) / 255.0))
    };
    let swatches = SWATCHES
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let (toy, cel) = (words.next()?, words.next()?);
            Some((name.to_owned(), hex(if m.toy { toy } else { cel })?))
        })
        .collect();
    super::ground_study::Study {
        variant: super::ground_study::Variant::Calm,
        swatches,
    }
}

/// What a model is, by its file: the palette size and outline width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Building,
    Figure,
    Other,
}

impl Kind {
    pub fn of_file(file: &str) -> Self {
        let f = file.replace('\\', "/").to_ascii_lowercase();
        if f.contains("buildings/") {
            Kind::Building
        } else if f.contains("character/") || f.contains("animals/") {
            Kind::Figure
        } else {
            Kind::Other
        }
    }
}

/// A texture's pixels as the look reads them (display values, rows from the
/// top), and whether it is cut out by its alpha (leaves, fences).
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub cut: bool,
}

impl Pixels {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let n = (width * height) as usize;
        let holes = rgba.chunks(4).filter(|p| p[3] < 128).count();
        Self {
            width,
            height,
            rgba,
            cut: n > 0 && holes * 20 > n,
        }
    }

    fn texel(&self, uv: [f32; 2]) -> [f32; 4] {
        let wrap = |t: f32, n: u32| (((t - t.floor()) * n as f32) as u32).min(n - 1);
        let (x, y) = (wrap(uv[0], self.width), wrap(uv[1], self.height));
        let i = ((y * self.width + x) * 4) as usize;
        [0, 1, 2, 3].map(|k| f32::from(self.rgba[i + k]) / 255.0)
    }
}

/// The pixels of texture files, read once per process (`None`: unreadable).
pub fn cached(key: &str, read: impl FnOnce() -> Option<Pixels>) -> Option<Arc<Pixels>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<Pixels>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache.lock().ok()?.get(key) {
        return found.clone();
    }
    let pixels = read().map(Arc::new);
    cache.lock().ok()?.insert(key.to_owned(), pixels.clone());
    pixels
}

/// One part's vertex data as the object material reads it.
pub struct Vertices {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub uvs1: Vec<[f32; 2]>,
    pub uvs2: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub bone_indices: Vec<[u32; 4]>,
    pub bone_weights: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

/// The vertex colour's alpha: a plain face, a roof face (tile rows drawn),
/// a shell vertex (`cel2`'s outline; its red is the width in pixels).
pub const PLAIN: f32 = 1.0;
pub const ROOF: f32 = 0.5;
pub const SHELL: f32 = -1.0;

/// Fixed sample points in a triangle (barycentric), the same every time.
const SAMPLES: [[f32; 3]; 12] = [
    [0.333, 0.333, 0.334],
    [0.6, 0.2, 0.2],
    [0.2, 0.6, 0.2],
    [0.2, 0.2, 0.6],
    [0.45, 0.45, 0.1],
    [0.1, 0.45, 0.45],
    [0.45, 0.1, 0.45],
    [0.8, 0.1, 0.1],
    [0.1, 0.8, 0.1],
    [0.1, 0.1, 0.8],
    [0.25, 0.5, 0.25],
    [0.5, 0.25, 0.25],
];

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|k| (a[k] - b[k]) * (a[k] - b[k])).sum()
}

/// Hue (degrees) and saturation (HSV) of a colour.
fn hue_sat(c: [f32; 3]) -> (f32, f32) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let d = max - min;
    if d < 1e-6 || max < 1e-6 {
        return (0.0, 0.0);
    }
    let h = if max == c[0] {
        60.0 * ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        60.0 * ((c[2] - c[0]) / d + 2.0)
    } else {
        60.0 * ((c[0] - c[1]) / d + 4.0)
    };
    (h, d / max)
}

/// k-means of weighted colours, started from luminance quantiles; the
/// centres.
fn kmeans(colours: &[[f32; 3]], weights: &[f32], k: usize) -> Vec<[f32; 3]> {
    let lum = |c: &[f32; 3]| 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
    let mut order: Vec<usize> = (0..colours.len()).collect();
    order.sort_by(|&a, &b| lum(&colours[a]).total_cmp(&lum(&colours[b])));
    let k = k.clamp(1, colours.len().max(1));
    let mut centres: Vec<[f32; 3]> = (0..k)
        .map(|i| colours[order[(i * 2 + 1) * order.len() / (2 * k)]])
        .collect();
    let mut label = vec![0usize; colours.len()];
    for _ in 0..15 {
        for (i, c) in colours.iter().enumerate() {
            label[i] = nearest(&centres, *c);
        }
        let mut sum = vec![[0.0f32; 4]; k];
        for (i, c) in colours.iter().enumerate() {
            let w = weights[i];
            let s = &mut sum[label[i]];
            for j in 0..3 {
                s[j] += c[j] * w;
            }
            s[3] += w;
        }
        for (centre, s) in centres.iter_mut().zip(&sum) {
            if s[3] > 0.0 {
                *centre = [s[0] / s[3], s[1] / s[3], s[2] / s[3]];
            }
        }
    }
    centres
}

fn nearest(centres: &[[f32; 3]], c: [f32; 3]) -> usize {
    let mut best = 0;
    for (j, centre) in centres.iter().enumerate() {
        if dist2(c, *centre) < dist2(c, centres[best]) {
            best = j;
        }
    }
    best
}

/// A palette colour cleaned for the look: more colour around its grey,
/// lifted.
fn clean(c: [f32; 3], m: Materials, building: bool) -> [f32; 3] {
    let mean = (c[0] + c[1] + c[2]) / 3.0;
    let mut out = c.map(|v| (mean + (v - mean) * m.colour).clamp(0.0, 1.0));
    let add = if m.toy { 0.03 } else { 0.07 };
    out = out.map(|v| (v * m.lift + add).clamp(0.0, 1.0));
    if m.toy && building {
        // The study's warm boost: reds, browns and oranges a little richer.
        let (h, s) = hue_sat(out);
        if (h < 40.0 || h > 300.0) && s > 0.07 {
            let mean = (out[0] + out[1] + out[2]) / 3.0;
            out = out.map(|v| ((mean + (v - mean) * 1.3).clamp(0.0, 1.0) * 1.05).min(1.0));
        }
    }
    out
}

/// The look's vertex data of one part: every face its own three vertices
/// with its flat colour (the colour's alpha [`PLAIN`] or [`ROOF`]); with
/// `cel2` a shell copy of every face after them. `texture` is the part's
/// colour texture; `tint` a constant colour the texture is drawn times (a
/// house's colour map at column 0), or white.
pub fn reshape(
    v: Vertices,
    texture: Option<&Pixels>,
    tint: [f32; 3],
    kind: Kind,
    m: Materials,
) -> Vertices {
    let tris: Vec<[usize; 3]> = v
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| t.map(|i| i as usize))
        .collect();
    let n = tris.len();
    // Each face's mean colour (alpha-weighted) and area.
    let mut colour = vec![[1.0f32; 3]; n];
    let mut area = vec![0.0f32; n];
    for (f, t) in tris.iter().enumerate() {
        let p = t.map(|i| v.positions[i]);
        let c = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        area[f] = 0.5 * (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() + 1e-6;
        if let Some(tex) = texture {
            let uv = t.map(|i| v.uvs[i]);
            let mut sum = [0.0f32; 4];
            for b in SAMPLES {
                let at = [0, 1].map(|k| b[0] * uv[0][k] + b[1] * uv[1][k] + b[2] * uv[2][k]);
                let px = tex.texel(at);
                let w = px[3] + 1e-3;
                for k in 0..3 {
                    sum[k] += px[k] * w;
                }
                sum[3] += w;
            }
            colour[f] = [0, 1, 2].map(|k| sum[k] / sum[3] * tint[k]);
        } else {
            let c = t.map(|i| v.colors[i]);
            colour[f] = [0, 1, 2].map(|k| (c[0][k] + c[1][k] + c[2][k]) / 3.0 * tint[k]);
        }
    }
    let cut = texture.is_some_and(|t| t.cut);
    let k = match kind {
        _ if cut => 3,
        Kind::Building => 8,
        Kind::Figure => 4,
        Kind::Other => 5,
    }
    .min((n / 3).max(1));
    let centres = kmeans(&colour, &area, k);
    let mut label: Vec<usize> = colour.iter().map(|c| nearest(&centres, *c)).collect();
    // Faces sharing an edge (by their corners' places, so seams of the
    // texture do not cut them apart).
    let key = |p: [f32; 3]| p.map(|x| (x * 512.0).round() as i32);
    let mut edges: HashMap<([i32; 3], [i32; 3]), Vec<usize>> = HashMap::new();
    for (f, t) in tris.iter().enumerate() {
        for e in 0..3 {
            let (a, b) = (key(v.positions[t[e]]), key(v.positions[t[(e + 1) % 3]]));
            let pair = if a < b { (a, b) } else { (b, a) };
            edges.entry(pair).or_default().push(f);
        }
    }
    let mut neighbours = vec![Vec::new(); n];
    for faces in edges.values() {
        for &a in faces {
            for &b in faces {
                if a != b {
                    neighbours[a].push(b);
                }
            }
        }
    }
    for _ in 0..2 {
        let next: Vec<usize> = (0..n)
            .map(|f| {
                let mut votes = vec![0.0f32; k];
                votes[label[f]] += area[f] * 1.5;
                for &g in &neighbours[f] {
                    votes[label[g]] += area[g];
                }
                (0..k).fold(0, |best, j| if votes[j] > votes[best] { j } else { best })
            })
            .collect();
        label = next;
    }
    let building = kind == Kind::Building;
    let palette: Vec<[f32; 3]> = centres.iter().map(|c| clean(*c, m, building)).collect();
    let mut flat = Vertices {
        positions: Vec::with_capacity(n * 3),
        normals: Vec::with_capacity(n * 3),
        uvs: Vec::with_capacity(n * 3),
        uvs1: Vec::with_capacity(n * 3),
        uvs2: Vec::with_capacity(n * 3),
        colors: Vec::with_capacity(n * 3),
        bone_indices: Vec::with_capacity(n * 3),
        bone_weights: Vec::with_capacity(n * 3),
        indices: Vec::new(),
    };
    for (f, t) in tris.iter().enumerate() {
        let c = palette[label[f]];
        let p = t.map(|i| v.positions[i]);
        let fn_ = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let len = (fn_[0] * fn_[0] + fn_[1] * fn_[1] + fn_[2] * fn_[2])
            .sqrt()
            .max(1e-9);
        let up = (fn_[1] / len).abs();
        let (h, s) = hue_sat(c);
        let roof = building && up > 0.3 && up < 0.97 && (h < 45.0 || h > 340.0) && s > 0.25;
        // The colour as drawn times the tint: divided by it here.
        let stored = [0, 1, 2].map(|j| (c[j] / tint[j].max(0.15)).min(1.0));
        let alpha = if roof { ROOF } else { PLAIN };
        for &i in t {
            flat.positions.push(v.positions[i]);
            flat.normals.push(v.normals[i]);
            flat.uvs.push(v.uvs[i]);
            flat.uvs1.push(v.uvs1[i]);
            flat.uvs2.push(v.uvs2[i]);
            flat.colors.push([stored[0], stored[1], stored[2], alpha]);
            flat.bone_indices.push(v.bone_indices[i]);
            flat.bone_weights.push(v.bone_weights[i]);
        }
    }
    let count = flat.positions.len() as u32;
    flat.indices = (0..count).collect();
    if !m.toy && !cut && m.line > 0.0 {
        // cel2: the shell, the same faces again; the shader pushes them out
        // on the screen and keeps only their far side (the outline).
        let width = m.line
            * match kind {
                Kind::Building => 1.0,
                Kind::Figure => 0.6,
                Kind::Other => 0.8,
            };
        for i in 0..count as usize {
            flat.positions.push(flat.positions[i]);
            flat.normals.push(flat.normals[i]);
            flat.uvs.push(flat.uvs[i]);
            flat.uvs1.push(flat.uvs1[i]);
            flat.uvs2.push(flat.uvs2[i]);
            flat.colors.push([width, 0.0, 0.0, SHELL]);
            flat.bone_indices.push(flat.bone_indices[i]);
            flat.bone_weights.push(flat.bone_weights[i]);
        }
        flat.indices.extend(count..2 * count);
    }
    flat
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(colours: [[u8; 4]; 2]) -> (Vertices, Pixels) {
        // Two triangles side by side, each over one half of a 2 x 1
        // texture.
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let v = Vertices {
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uvs: vec![[0.1, 0.5], [0.4, 0.5], [0.9, 0.5], [0.6, 0.5]],
            uvs1: vec![[0.0; 2]; 4],
            uvs2: vec![[0.0; 2]; 4],
            colors: vec![[1.0; 4]; 4],
            bone_indices: vec![[0; 4]; 4],
            bone_weights: vec![[1.0, 0.0, 0.0, 0.0]; 4],
            indices: vec![0, 1, 3, 1, 2, 3],
            positions,
        };
        let mut rgba = Vec::new();
        for c in colours {
            rgba.extend(c);
        }
        (v, Pixels::new(2, 1, rgba))
    }

    #[test]
    fn faces_take_flat_cleaned_colours_and_cel_adds_a_shell() {
        // spec: none (the look study, round 2: a development switch; each
        // face one colour of its own texture, the shell for the outline)
        let toy = Materials {
            toy: true,
            colour: 1.0,
            lift: 1.0,
            line: 0.0,
        };
        let (v, tex) = quad([[200, 40, 40, 255], [40, 40, 200, 255]]);
        let out = reshape(v, Some(&tex), [1.0; 3], Kind::Other, toy);
        assert_eq!(out.positions.len(), 6);
        assert_eq!(out.indices, vec![0, 1, 2, 3, 4, 5]);
        // Every face's three corners share one colour.
        for f in 0..2 {
            assert_eq!(out.colors[3 * f], out.colors[3 * f + 1]);
            assert_eq!(out.colors[3 * f], out.colors[3 * f + 2]);
            assert_eq!(out.colors[3 * f][3], PLAIN);
        }
        let cel = Materials {
            toy: false,
            line: 2.0,
            ..toy
        };
        let (v, tex) = quad([[200, 40, 40, 255], [40, 40, 200, 255]]);
        let out = reshape(v, Some(&tex), [1.0; 3], Kind::Building, cel);
        assert_eq!(out.positions.len(), 12);
        assert_eq!(out.colors[6], [2.0, 0.0, 0.0, SHELL]);
        assert_eq!(out.positions[6], out.positions[0]);
        // A texture cut out by its alpha (leaves) gets no shell.
        let (v, tex) = quad([[20, 200, 20, 0], [20, 200, 20, 255]]);
        assert!(tex.cut);
        assert_eq!(
            reshape(v, Some(&tex), [1.0; 3], Kind::Other, cel)
                .positions
                .len(),
            6
        );
    }

    #[test]
    fn files_by_kind_and_ground_colours() {
        // spec: none (the look study, round 2)
        assert_eq!(
            Kind::of_file("data/buildings/PB_R_Castle01.GR2"),
            Kind::Building
        );
        assert_eq!(Kind::of_file("data\\character\\carrier.GR2"), Kind::Figure);
        assert_eq!(
            Kind::of_file("data/map/objects/RS_TreeFir01.GR2"),
            Kind::Other
        );
        let m = Materials {
            toy: true,
            colour: 1.35,
            lift: 1.12,
            line: 0.0,
        };
        let g = ground(m);
        assert_eq!(g.variant, super::super::ground_study::Variant::Calm);
        let turf = [0x62, 0xB8, 0x4A].map(|c| c as f32 / 255.0);
        assert_eq!(g.swatches["turf"], turf);
        assert_eq!(g.swatches.len(), 22);
        for (_, names) in super::super::ground_study::TABLE {
            for name in names.iter() {
                assert!(g.swatches.contains_key(*name), "{name}");
            }
        }
    }
}
