//! The settlers' palette atlas (`specs/GFX-objects.md` GFX-OBJ-3.1, 3.2):
//! a 16 x 16 block (row = material index, column = luminance step) per new
//! settler figure, its ramp rows drawn per colour-map entry, in a 1024 x
//! 1024 image.

use std::collections::HashMap;

use s2_assets::color_slots::{ColorSlots, Ramp};
use s2_assets::tga;

/// GFX-OBJ-3.2: the atlas side and the block side in texels.
pub const ATLAS_SIZE: u32 = 1024;
pub const BLOCK: u32 = 16;
/// Blocks per atlas row.
const PER_ROW: u32 = ATLAS_SIZE / BLOCK;
/// GFX-OBJ-3.2 (3): the seed of the view's generator.
pub const FIGURE_SEED: u32 = 1_412_607_200;

/// GFX-OBJ-3.2 (3): the view's own generator for the settler figures'
/// ramp rows, the "minimal standard" one: state = 16807 x state mod
/// (2^31 - 1), the draw being the new state. Never the simulation's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FigureRng(u32);

impl FigureRng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    pub fn draw(&mut self) -> u32 {
        self.0 = (u64::from(self.0) * 16_807 % 2_147_483_647) as u32;
        self.0
    }
}

/// A ramp's 16 colours (RGBA bytes), brightest first as stored, for ramp
/// row `row`.
fn ramp_colours(
    ramp: &Ramp,
    row: u32,
    image: &impl Fn(&str) -> Option<tga::Image>,
) -> [[u8; 4]; 16] {
    let byte = |c: [f32; 3]| {
        let b = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        [b[0], b[1], b[2], 255]
    };
    match ramp {
        Ramp::Solid(c) => [byte(*c); 16],
        Ramp::Gradient { bright, dark } => std::array::from_fn(|i| {
            let t = i as f32 / 15.0;
            byte(std::array::from_fn(|k| {
                bright[k] + (dark[k] - bright[k]) * t
            }))
        }),
        Ramp::Texture { file, .. } => {
            let Some(img) = image(file) else {
                return [[255, 0, 255, 255]; 16];
            };
            std::array::from_fn(|i| {
                if img.width == 0 {
                    return [0, 0, 0, 255];
                }
                let x = (i as u32).min(img.width - 1);
                img.pixel(x, row.min(img.height.max(1) - 1))
            })
        }
    }
}

/// GFX-OBJ-3.1: the ramp rows a slot offers: a texture's `rows` (default
/// its height), one for a solid or a gradient.
fn ramp_rows(ramp: &Ramp, image: &impl Fn(&str) -> Option<tga::Image>) -> u32 {
    match ramp {
        Ramp::Texture { file, rows } => {
            let height = image(file).map_or(1, |img| img.height.max(1));
            rows.unwrap_or(height).clamp(1, height)
        }
        _ => 1,
    }
}

/// A colour map's 16 entries: for each, its ramp rows' colours (none for
/// an entry without a slot).
type Entries = Vec<Vec<[[u8; 4]; 16]>>;

/// The atlas image and the blocks made so far.
#[derive(Debug, Clone, PartialEq)]
pub struct Atlas {
    pub rgba: Vec<u8>,
    maps: HashMap<String, Entries>,
    /// GFX-OBJ-3.2 (3a): each block in use (colour map, ramp row per
    /// entry): its place (none while the atlas was full when it was made)
    /// and how many figures use it.
    blocks: HashMap<BlockKey, (Option<(u32, u32)>, u32)>,
    /// Each colour map's placed blocks in the order they were made (the
    /// full atlas's fallback).
    made: HashMap<String, Vec<(u32, u32)>>,
    /// The places taken, in row order (3a: 64 x 64 places).
    taken: Vec<bool>,
    /// The game the blocks belong to: the atlas is new at every game start.
    game: u32,
    rng: FigureRng,
    /// Blocks were written since the image was last handed on.
    dirty: bool,
}

/// A block: its colour map and the ramp row of each entry.
type BlockKey = (String, [u32; 16]);

/// GFX-OBJ-3.2 (3a): the block a figure uses, given back when the figure is
/// removed ([`Atlas::release`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FigureBlock {
    key: BlockKey,
    game: u32,
}

impl Atlas {
    /// The colour maps' ramps, read once; no block yet (GFX-OBJ-3.2 (3):
    /// a block is made per new figure). The generator is seeded here, so
    /// once per program run when the first figure needs the atlas.
    pub fn build(slots: &ColorSlots, image: impl Fn(&str) -> Option<tga::Image>) -> Self {
        let mut rgba = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
        for px in rgba.chunks_mut(4) {
            px[3] = 255;
        }
        let mut cache: HashMap<String, Option<tga::Image>> = HashMap::new();
        for r in &slots.ramps {
            if let Ramp::Texture { file, .. } = &r.ramp {
                cache
                    .entry(file.to_ascii_lowercase())
                    .or_insert_with(|| image(file));
            }
        }
        let load = |file: &str| cache.get(&file.to_ascii_lowercase()).cloned().flatten();
        let maps = slots
            .maps
            .iter()
            .map(|map| {
                let entries = map
                    .materials
                    .iter()
                    .map(|slot| {
                        let Some(ramp) = slot.as_deref().and_then(|s| slots.ramp(s)) else {
                            return Vec::new();
                        };
                        (0..ramp_rows(ramp, &load))
                            .map(|row| ramp_colours(ramp, row, &load))
                            .collect()
                    })
                    .collect();
                (map.name.clone(), entries)
            })
            .collect();
        Self {
            rgba,
            maps,
            blocks: HashMap::new(),
            made: HashMap::new(),
            taken: vec![false; (PER_ROW * PER_ROW) as usize],
            game: 0,
            rng: FigureRng::new(FIGURE_SEED),
            dirty: false,
        }
    }

    /// GFX-OBJ-3.2 (3a): a game starts: the atlas is new (no block, every
    /// place free, black); the generator goes on from where it was.
    pub fn new_game(&mut self) {
        self.blocks.clear();
        self.made.clear();
        self.taken.fill(false);
        for px in self.rgba.chunks_mut(4) {
            px.copy_from_slice(&[0, 0, 0, 255]);
        }
        self.game += 1;
        self.dirty = true;
    }

    /// GFX-OBJ-3.2 (3a): a figure is removed and gives its block back; the
    /// block's place is freed when no figure uses it any more. A block
    /// without a place (made while the atlas was full) is not given back.
    pub fn release(&mut self, block: &FigureBlock) {
        if block.game != self.game {
            return;
        }
        let Some((Some(at), uses)) = self.blocks.get_mut(&block.key) else {
            return;
        };
        *uses -= 1;
        if *uses > 0 {
            return;
        }
        let at = *at;
        self.blocks.remove(&block.key);
        self.taken[(at.1 / BLOCK * PER_ROW + at.0 / BLOCK) as usize] = false;
        if let Some(made) = self.made.get_mut(&block.key.0) {
            made.retain(|&p| p != at);
        }
    }

    /// GFX-OBJ-3.2 (3): a new figure of colour map `map`: one draw per
    /// entry, entry 1 first, also for an entry with one row (or none),
    /// each entry's row `draw mod rows`. None for an unknown map (no
    /// draw).
    pub fn draw_rows(&mut self, map: &str) -> Option<[u32; 16]> {
        let entries = self.maps.get(map)?;
        let counts: [u32; 16] =
            std::array::from_fn(|e| entries.get(e).map_or(1, |r| r.len().max(1) as u32));
        Some(counts.map(|rows| self.rng.draw() % rows))
    }

    /// GFX-OBJ-3.2, 3.3: the block of a new figure of colour map `map` as
    /// the shader takes it: (offset u, offset v) of the block in the atlas
    /// and the scales that turn the luminance (0..1) into its 16 columns
    /// and the red byte (the material index / 255) into its rows. Its
    /// rows are drawn (`draw_rows`); a block equal to one in use shares its
    /// place (3a), and the figure's [`FigureBlock`] gives it back when the
    /// figure goes. With the atlas full the block gets no place; the figure
    /// then shows one of its colour map's placed blocks, chosen by its rows
    /// (3a leaves the full atlas's look to the implementation).
    pub fn figure(&mut self, map: &str) -> Option<([f32; 4], FigureBlock)> {
        let rows = self.draw_rows(map)?;
        let at = self.block_for(map, rows)?;
        let block = FigureBlock {
            key: (map.to_owned(), rows),
            game: self.game,
        };
        Some((Self::uniform(at), block))
    }

    fn uniform((bx, by): (u32, u32)) -> [f32; 4] {
        let texel = 1.0 / ATLAS_SIZE as f32;
        [
            bx as f32 * texel,
            by as f32 * texel,
            (BLOCK - 1) as f32 * texel,
            255.0 * texel,
        ]
    }

    /// The block of colour map `map` with ramp row `rows[e]` for entry
    /// e + 1, made if new. Block row r is material index r (row 0 and
    /// entries without a slot black; the block's 16 rows end at index 15,
    /// so entry 16 has no row of its own). Column c holds ramp step 15 - c
    /// (3.2 (1)).
    fn block_for(&mut self, map: &str, rows: [u32; 16]) -> Option<(u32, u32)> {
        let key = (map.to_owned(), rows);
        let entries = self.maps.get(map)?;
        // 3a: an equal block placed shares its place (one use more).
        if let Some((Some(at), uses)) = self.blocks.get_mut(&key) {
            *uses += 1;
            return Some(*at);
        }
        // 3a: a new block (or one made while the atlas was full, which
        // tries again) takes the first free place in row order.
        let Some(place) = self.taken.iter().position(|&t| !t) else {
            // The atlas full: the block gets no place and is not given back
            // (3a); what such a figure shows is ours to choose: one of its
            // colour map's placed blocks, picked by its rows.
            self.blocks.entry(key).or_insert((None, 0)).1 += 1;
            let made = self.made.get(map)?;
            let pick = rows.iter().fold(0u64, |a, r| a * 16 + u64::from(*r));
            return made
                .get((pick % made.len().max(1) as u64) as usize)
                .copied();
        };
        let place = place as u32;
        self.taken[place as usize] = true;
        let (bx, by) = (place % PER_ROW * BLOCK, place / PER_ROW * BLOCK);
        // A freed place may hold an earlier block: black first.
        for row in 0..BLOCK {
            for c in 0..BLOCK {
                let at = (((by + row) * ATLAS_SIZE + bx + c) * 4) as usize;
                self.rgba[at..at + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
        for row in 1..16u32 {
            let Some(colours) = entries
                .get(row as usize - 1)
                .and_then(|ramps| ramps.get(rows[row as usize - 1] as usize))
            else {
                continue;
            };
            for c in 0..16u32 {
                let at = (((by + row) * ATLAS_SIZE + bx + c) * 4) as usize;
                self.rgba[at..at + 4].copy_from_slice(&colours[15 - c as usize]);
            }
        }
        let uses = self.blocks.get(&key).map_or(0, |b| b.1);
        self.blocks.insert(key, (Some((bx, by)), uses + 1));
        self.made.entry(map.to_owned()).or_default().push((bx, by));
        self.dirty = true;
        Some((bx, by))
    }

    /// Whether blocks were written since the last call (the image must be
    /// handed on again).
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[[u8; 3]]) -> tga::Image {
        let mut rgba = Vec::new();
        for r in rows {
            for i in 0..16u8 {
                rgba.extend([r[0], r[1], r[2].saturating_sub(i), 255]);
            }
        }
        tga::Image {
            width: 16,
            height: rows.len() as u32,
            rgba,
        }
    }

    // spec: GFX-OBJ-3.2
    #[test]
    fn blocks_hold_the_ramps() {
        let slots = ColorSlots::read(
            r#"<c><ColorRamps>
            <ColorRamp slot="Black" solid="0.0 0.0 0.0"/>
            <ColorRamp slot="Hair" texture="hair.tga" rows="2"/>
            </ColorRamps><ColorMaps><ColorMap name="default">
            <Material index="1" slot="Black"/><Material index="3" slot="Hair"/>
            </ColorMap><ColorMap name="other"/></ColorMaps></c>"#,
        );
        let mut atlas = Atlas::build(&slots, |f| {
            (f == "hair.tga").then(|| table(&[[10, 20, 200], [30, 40, 100]]))
        });
        let texel = |atlas: &Atlas, x: u32, y: u32| {
            let i = ((y * ATLAS_SIZE + x) * 4) as usize;
            [atlas.rgba[i], atlas.rgba[i + 1], atlas.rgba[i + 2]]
        };
        let rows = |hair: u32| std::array::from_fn(|e| if e == 2 { hair } else { 0 });
        // The first block made is block 0: row 3 is the hair ramp's row 0,
        // darkest step at column 0, brightest at 15.
        assert_eq!(atlas.block_for("default", rows(0)), Some((0, 0)));
        assert_eq!(texel(&atlas, 15, 3), [10, 20, 200]);
        assert_eq!(texel(&atlas, 0, 3), [10, 20, 185]);
        assert_eq!(texel(&atlas, 5, 1), [0, 0, 0]);
        assert!(atlas.take_dirty() && !atlas.take_dirty());
        // Hair row 1 is a new block; the same rows again share it.
        assert_eq!(atlas.block_for("default", rows(1)), Some((16, 0)));
        assert_eq!(texel(&atlas, 16 + 15, 3), [30, 40, 100]);
        assert_eq!(atlas.block_for("default", rows(0)), Some((0, 0)));
        assert_eq!(atlas.block_for("other", rows(0)), Some((32, 0)));
        let b = Atlas::uniform((64, 0));
        assert_eq!((b[0] * 1024.0, b[1] * 1024.0), (64.0, 0.0));
        assert_eq!(b[2] * 1024.0, 15.0);
        // The red byte 3 (3 / 255 sampled) lands on row 3.
        assert!((3.0 / 255.0 * b[3] * 1024.0 - 3.0).abs() < 1e-4);
        assert!(atlas.figure("missing").is_none());
    }

    /// T42: the colour map `default` (entries 1 .. 16 with 16, 1, 16, 16,
    /// 6, 16 x 8, 8, 8, 1 ramp rows), the view's generator seeded
    /// 1412607200, three figures made one after the other: each draws one
    /// number per entry, its row the draw mod the entry's rows.
    // spec: GFX-OBJ-3.2
    #[test]
    fn figures_draw_a_row_per_entry() {
        let rows = [16, 1, 16, 16, 6, 16, 16, 16, 16, 16, 16, 16, 16, 8, 8, 1];
        let names = [
            "EyeInner",
            "EyeOuter",
            "Hair",
            "Jacket",
            "Skin",
            "Trousers",
            "Belt",
            "Shoes",
            "Details",
            "Apron",
            "Hat_brown",
            "Hat_green",
            "Hat_grey",
            "Yellow",
            "Red",
            "_empty_",
        ];
        let mut xml = String::from("<c><ColorRamps>");
        for (name, n) in names.iter().zip(rows) {
            xml += &format!(r#"<ColorRamp slot="{name}" texture="{name}.tga" rows="{n}"/>"#);
        }
        xml += r#"</ColorRamps><ColorMaps><ColorMap name="default">"#;
        for (i, name) in names.iter().enumerate() {
            xml += &format!(r#"<Material index="{}" slot="{name}"/>"#, i + 1);
        }
        xml += "</ColorMap></ColorMaps></c>";
        let slots = ColorSlots::read(&xml);
        let mut atlas = Atlas::build(&slots, |f| {
            let n = names.iter().position(|name| format!("{name}.tga") == f)?;
            Some(table(&vec![[1, 2, 3]; rows[n] as usize]))
        });
        assert_eq!(
            atlas.draw_rows("default"),
            Some([15, 0, 14, 14, 3, 11, 11, 2, 1, 10, 3, 10, 14, 4, 2, 0])
        );
        assert_eq!(
            atlas.draw_rows("default"),
            Some([4, 0, 12, 1, 3, 14, 6, 11, 1, 9, 10, 10, 6, 2, 7, 0])
        );
        assert_eq!(
            atlas.draw_rows("default"),
            Some([12, 0, 9, 15, 3, 2, 2, 4, 12, 8, 5, 4, 5, 1, 6, 0])
        );
        // The generator itself: the minimal standard one.
        let mut rng = FigureRng::new(1);
        assert_eq!((rng.draw(), rng.draw()), (16_807, 282_475_249));
    }

    /// GFX-OBJ-3.2 (3): figures of the same colour map get independent
    /// rows per entry (hair and skin do not vary together), so their
    /// blocks differ.
    // spec: GFX-OBJ-3.2
    #[test]
    fn each_figure_gets_its_own_block() {
        let slots = ColorSlots::read(
            r#"<c><ColorRamps>
            <ColorRamp slot="Hair" texture="hair.tga" rows="16"/>
            <ColorRamp slot="Skin" texture="skin.tga" rows="6"/>
            </ColorRamps><ColorMaps><ColorMap name="default">
            <Material index="3" slot="Hair"/><Material index="5" slot="Skin"/>
            </ColorMap></ColorMaps></c>"#,
        );
        let mut atlas = Atlas::build(&slots, |f| {
            let n = if f == "hair.tga" { 16 } else { 6 };
            Some(table(&(0..n).map(|r| [r * 10, 0, 100]).collect::<Vec<_>>()))
        });
        let mut pairs = std::collections::HashSet::new();
        for _ in 0..40 {
            let r = atlas.draw_rows("default").unwrap();
            assert!(r[2] < 16 && r[4] < 6);
            pairs.insert((r[2], r[4]));
        }
        // Not one skin per hair, as with a single variant per settler.
        let hairs: std::collections::HashSet<_> = pairs.iter().map(|p| p.0).collect();
        assert!(pairs.len() > hairs.len());
        let a = atlas.figure("default").unwrap().0;
        let b = atlas.figure("default").unwrap().0;
        assert_ne!(a, b);
    }

    /// T45 (GFX-OBJ-3.2 (3a), Q-1043): blocks A, B, C take (0, 0), (16, 0),
    /// (32, 0); A's figure removed frees its place, the next new block D
    /// takes it; a figure with B's block shares (16, 0), which stays until
    /// both are removed; a new game starts the atlas empty (the first block
    /// at (0, 0) again) while the draws go on; a full atlas gives a block no
    /// place (and nothing back), the figure still made.
    // spec: GFX-OBJ-3.2 (T45)
    #[test]
    fn t45_blocks_are_counted_freed_and_new_each_game() {
        let slots = ColorSlots::read(
            r#"<c><ColorRamps>
            <ColorRamp slot="Hair" texture="hair.tga" rows="16"/>
            </ColorRamps><ColorMaps><ColorMap name="default">
            <Material index="3" slot="Hair"/>
            </ColorMap></ColorMaps></c>"#,
        );
        let mut atlas = Atlas::build(&slots, |_| {
            Some(table(
                &(0..16).map(|r| [r * 10, 0, 100]).collect::<Vec<_>>(),
            ))
        });
        let rows = |hair: u32| {
            let mut r = [0; 16];
            r[2] = hair;
            r
        };
        let block = |atlas: &Atlas, hair: u32| FigureBlock {
            key: ("default".to_string(), rows(hair)),
            game: atlas.game,
        };
        assert_eq!(atlas.block_for("default", rows(1)), Some((0, 0)));
        assert_eq!(atlas.block_for("default", rows(2)), Some((16, 0)));
        assert_eq!(atlas.block_for("default", rows(3)), Some((32, 0)));
        atlas.release(&block(&atlas, 1));
        assert_eq!(atlas.block_for("default", rows(4)), Some((0, 0)));
        // A freed place is black before the new block is written: row 3
        // holds hair row 4.
        let texel = |atlas: &Atlas, x: u32, y: u32| {
            let i = ((y * ATLAS_SIZE + x) * 4) as usize;
            [atlas.rgba[i], atlas.rgba[i + 1], atlas.rgba[i + 2]]
        };
        assert_eq!(texel(&atlas, 15, 3), [40, 0, 100]);
        // B's block twice: freed only after both figures went.
        assert_eq!(atlas.block_for("default", rows(2)), Some((16, 0)));
        atlas.release(&block(&atlas, 2));
        assert_eq!(atlas.block_for("default", rows(5)), Some((48, 0)));
        atlas.release(&block(&atlas, 2));
        assert_eq!(atlas.block_for("default", rows(6)), Some((16, 0)));
        // A new game: empty again; the generator goes on.
        let before = atlas.rng;
        let old = block(&atlas, 3);
        atlas.new_game();
        assert_eq!(atlas.rng, before);
        assert_eq!(atlas.block_for("default", rows(9)), Some((0, 0)));
        // A block of the old game given back changes nothing now.
        atlas.release(&old);
        assert_eq!(atlas.block_for("default", rows(10)), Some((16, 0)));
        // Full: no place; the figure takes a placed block of its map.
        atlas.taken.fill(true);
        assert!(atlas.block_for("default", rows(11)).is_some());
        assert_eq!(atlas.blocks[&("default".to_string(), rows(11))].0, None);
    }
}
