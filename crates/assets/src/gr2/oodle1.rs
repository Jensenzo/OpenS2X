//! Oodle1 decompression of Granny file sections (`specs/GFX-granny-format.md`
//! GFX-GR2-2).

/// GFX-GR2-2.4: the cumulative table of a model spans this range.
const RANGE: u32 = 0x4000;
/// GFX-GR2-2.3: refill while the denominator is at most this.
const REFILL_AT: u32 = 0x80_0000;
/// GFX-GR2-2.8: one size model per previous size symbol 0..64.
const SIZE_SYMBOLS: u32 = 65;
/// GFX-GR2-2.9: copy lengths of the size symbols 61..64.
const LONG_COPIES: [usize; 4] = [128, 192, 256, 512];

/// Why a section did not decompress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Oodle1Error {
    /// Fewer than the three parameter blocks.
    Truncated,
    /// A copy reaches back before the start of the output.
    BadDistance { at: usize, distance: usize },
    /// The part boundaries are out of order or not multiples of 4
    /// (GFX-GR2-2.10).
    BadParts,
    /// A copy runs past the end of its part (GFX-GR2-2.10): the stored
    /// data is damaged.
    CopyPastPart { at: usize, length: usize },
    /// A parameter block with no literal values (GFX-GR2-2.2): damaged.
    BadParameters { part: usize },
    /// The decoder did not end 1 to 3 bytes before the stored size
    /// (GFX-GR2-2.3, 2.11): the output is not to be trusted.
    Diverged { used: usize, stored: usize },
}

impl std::fmt::Display for Oodle1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => write!(f, "compressed section shorter than its parameter blocks"),
            Self::BadDistance { at, distance } => {
                write!(
                    f,
                    "copy from {distance} bytes back at {at} (decoder diverged)"
                )
            }
            Self::BadParts => write!(f, "section parts out of order or not aligned"),
            Self::BadParameters { part } => write!(f, "part {part} allows no literal value"),
            Self::CopyPastPart { at, length } => {
                write!(f, "copy of {length} bytes at {at} runs past its part")
            }
            Self::Diverged { used, stored } => write!(
                f,
                "decoder used {used} of {stored} stored bytes, not 1 to 3 short"
            ),
        }
    }
}

/// GFX-GR2-2.3: the arithmetic decoder on 7-bit stream units.
struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
    numerator: u32,
    denominator: u32,
    step: u32,
    /// A model met a state a valid stream never makes (a full list grown, a
    /// new value that is already listed): the data is damaged.
    damaged: bool,
}

/// Observer of the decoding, for the test vectors (GFX-GR2 T10 to T12).
trait Observer {
    /// A token: output position, size model used, size symbol.
    fn token(&mut self, _pos: usize, _ctx: usize, _size: usize) {}
    /// A part is done; its models are final.
    fn part_done(&mut self, _part: usize, _models: &PartModels) {}
}

struct NoObserver;
impl Observer for NoObserver {}

impl<'a> Decoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        let first = data.first().copied().unwrap_or(0);
        Self {
            data,
            pos: 0,
            numerator: u32::from(first >> 1),
            denominator: 0x80,
            step: 1,
            damaged: false,
        }
    }

    fn byte(&self, at: usize) -> u32 {
        u32::from(self.data.get(at).copied().unwrap_or(0))
    }

    /// A value below `m`, not yet committed.
    fn decode(&mut self, m: u32) -> u32 {
        while self.denominator <= REFILL_AT {
            let bits = ((self.byte(self.pos) & 1) << 7) | (self.byte(self.pos + 1) >> 1);
            self.numerator = (self.numerator << 8) | bits;
            self.denominator <<= 8;
            self.pos += 1;
        }
        self.step = self.denominator / m;
        (self.numerator / self.step).min(m - 1)
    }

    /// Commit the symbol covering `[v, v + w)` of `m`.
    fn commit(&mut self, v: u32, w: u32, m: u32) {
        self.numerator -= self.step * v;
        if v + w < m {
            self.denominator = self.step * w;
        } else {
            self.denominator -= self.step * v;
        }
    }

    /// A uniform value below `m`.
    fn uniform(&mut self, m: u32) -> u32 {
        let v = self.decode(m);
        self.commit(v, 1, m);
        v
    }
}

/// A model's state, as the test vectors list it (GFX-GR2-2.4).
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub values: Vec<u32>,
    pub weights: Vec<u32>,
    pub total: u32,
    pub table: Vec<u32>,
    pub increment: u32,
    pub threshold: u32,
}

/// GFX-GR2-2.4: an adaptive model. Entry 0 of `values` and `weights` is the
/// escape (its value is unused); `table` holds the boundaries of the listed
/// entries, the first `table.len() - 1` of the list.
struct Model {
    /// Capacity `C`: the list never holds more than `C + 1` entries.
    capacity: u32,
    values: Vec<u32>,
    weights: Vec<u32>,
    table: Vec<u32>,
    total: u32,
    increment: u32,
    increment_cap: u32,
    table_threshold: u32,
    halving_threshold: u32,
    #[cfg(test)]
    upkeep: Vec<(bool, Snapshot, Snapshot)>,
}

impl Model {
    fn new(limit: u32, capacity: u32) -> Self {
        let halving_threshold = 256u32.max((32 * limit).min(15160));
        let increment_cap = if limit > 64 {
            (2 * limit).min(halving_threshold / 2 - 32)
        } else {
            128
        };
        Self {
            capacity,
            values: vec![0],
            weights: vec![4],
            table: vec![0, RANGE],
            total: 4,
            increment: 4,
            increment_cap,
            table_threshold: 8,
            halving_threshold,
            #[cfg(test)]
            upkeep: Vec::new(),
        }
    }

    #[cfg(test)]
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            values: self.values.clone(),
            weights: self.weights.clone(),
            total: self.total,
            table: self.table.clone(),
            increment: self.increment,
            threshold: self.table_threshold,
        }
    }

    /// GFX-GR2-2.6: list every entry in a new table, then raise the
    /// threshold.
    fn recompute(&mut self) {
        let f = 0x20000 / self.total;
        self.table.clear();
        let mut start = 0;
        for &w in &self.weights {
            self.table.push(start);
            start += w * f / 8;
        }
        self.table.push(RANGE);
        if self.increment > self.increment_cap / 2 {
            self.table_threshold = self.total + self.increment_cap;
        } else {
            self.increment *= 2;
            self.table_threshold = self.total + self.increment;
        }
    }

    /// GFX-GR2-2.7.
    fn halve(&mut self) {
        for w in &mut self.weights {
            *w /= 2;
        }
        let mut i = 1;
        while i < self.weights.len() {
            if self.weights[i] == 0 {
                self.weights.swap_remove(i);
                self.values.swap_remove(i);
            } else {
                i += 1;
            }
        }
        if self.weights.len() > 1 {
            let mut heaviest = 1;
            for i in 2..self.weights.len() {
                if self.weights[i] > self.weights[heaviest] {
                    heaviest = i;
                }
            }
            let last = self.weights.len() - 1;
            self.weights.swap(heaviest, last);
            self.values.swap(heaviest, last);
        }
        if (self.weights.len() as u32) < self.capacity + 1 && self.weights[0] == 0 {
            self.weights[0] = 1;
        }
        self.total = self.weights.iter().sum();
    }

    /// GFX-GR2-2.5: decode a symbol; `bound` is the bound of a new value.
    fn symbol(&mut self, d: &mut Decoder, bound: u32) -> u32 {
        if self.total >= self.table_threshold {
            #[cfg(test)]
            let before = self.snapshot();
            let halving = self.table_threshold >= self.halving_threshold;
            if halving {
                self.halve();
            }
            self.recompute();
            #[cfg(test)]
            {
                let after = self.snapshot();
                self.upkeep.push((halving, before, after));
            }
        }
        let x = d.decode(RANGE);
        let j = self.table.partition_point(|&s| s <= x) - 1;
        d.commit(self.table[j], self.table[j + 1] - self.table[j], RANGE);
        self.weights[j] += 1;
        self.total += 1;
        if j >= 1 {
            return self.values[j];
        }
        let listed = self.table.len() - 1;
        let unlisted = self.weights.len() - listed;
        if unlisted > 0 && d.uniform(2) == 1 {
            let q = d.uniform(unlisted as u32) as usize;
            self.weights[listed + q] += 2;
            self.total += 2;
            return self.values[listed + q];
        }
        // A full list never takes a new value in a valid stream.
        if self.weights.len() as u32 > self.capacity {
            d.damaged = true;
        }
        self.weights.push(2);
        self.values.push(0);
        self.total += 2;
        if self.weights.len() as u32 == self.capacity + 1 {
            self.total -= self.weights[0];
            self.weights[0] = 0;
        }
        let value = d.uniform(bound);
        // Nor a new value that is already listed.
        if self.values[1..self.values.len() - 1].contains(&value) {
            d.damaged = true;
        }
        *self.values.last_mut().expect("a new entry") = value;
        value
    }
}

/// GFX-GR2-2.2: one part's parameters.
struct Params {
    literal_limit: u32,
    reference_limit: u32,
    literal_capacity: u32,
    high_capacity: u32,
    size_capacities: [u8; 4],
}

impl Params {
    fn read(block: &[u8]) -> Self {
        let a = u32::from_le_bytes(block[0..4].try_into().expect("4 bytes"));
        let b = u32::from_le_bytes(block[4..8].try_into().expect("4 bytes"));
        Self {
            literal_limit: a & 0x1FF,
            reference_limit: a >> 9,
            literal_capacity: b & 0x1FF,
            high_capacity: b >> 19,
            size_capacities: block[8..12].try_into().expect("4 bytes"),
        }
    }
}

/// GFX-GR2-2.8: the models of one part.
struct PartModels {
    sizes: Vec<Model>,
    low: Model,
    high: Model,
    mid: Vec<Model>,
    literals: Vec<Model>,
}

impl PartModels {
    fn new(p: &Params) -> Self {
        let r = p.reference_limit;
        let low_limit = (r + 1).min(4);
        let high_count = r / 1024 + 1;
        let mid_limit = (r / 4 + 1).min(256);
        Self {
            sizes: (0..SIZE_SYMBOLS)
                .map(|i| {
                    let group = if i == 64 { 0 } else { 3 - (i / 16) as usize };
                    Model::new(SIZE_SYMBOLS - 1, u32::from(p.size_capacities[group]))
                })
                .collect(),
            low: Model::new(low_limit - 1, low_limit),
            high: Model::new(high_count - 1, p.high_capacity + 1),
            mid: (0..high_count)
                .map(|_| Model::new(mid_limit - 1, mid_limit))
                .collect(),
            literals: (0..4)
                .map(|_| Model::new(p.literal_limit - 1, p.literal_capacity))
                .collect(),
        }
    }
}

/// GFX-GR2-2: decompress a section of `stored` bytes into `expanded` bytes,
/// in three parts ending at `first16`, `first8` and `expanded`. Returns the
/// data and the stored bytes the decoder used.
pub fn decompress(
    stored: &[u8],
    expanded: usize,
    first16: usize,
    first8: usize,
) -> Result<(Vec<u8>, usize), Oodle1Error> {
    decompress_observed(stored, expanded, first16, first8, &mut NoObserver)
}

fn decompress_observed(
    stored: &[u8],
    expanded: usize,
    first16: usize,
    first8: usize,
    observer: &mut dyn Observer,
) -> Result<(Vec<u8>, usize), Oodle1Error> {
    if stored.len() < 36 {
        return Err(Oodle1Error::Truncated);
    }
    // GFX-GR2-2.10: the parts in order, every boundary a multiple of 4.
    if first16 > first8
        || first8 > expanded
        || !(first16.is_multiple_of(4) && first8.is_multiple_of(4))
    {
        return Err(Oodle1Error::BadParts);
    }
    // A damaged size must not reserve gigabytes up front.
    let mut out: Vec<u8> = Vec::with_capacity(expanded.min(1 << 26));
    let mut d = Decoder::new(&stored[36..]);
    for (part, end) in [first16, first8, expanded].into_iter().enumerate() {
        // GFX-GR2-2.10: an empty part creates nothing and reads nothing.
        if out.len() >= end {
            continue;
        }
        // GFX-GR2-2.1: part i always uses parameter block i.
        let params = Params::read(&stored[12 * part..12 * part + 12]);
        if params.literal_limit == 0 {
            return Err(Oodle1Error::BadParameters { part });
        }
        let mut m = PartModels::new(&params);
        let mut previous = 0usize;
        let mut produced = 0usize;
        while out.len() < end {
            let pos = out.len();
            let size = m.sizes[previous].symbol(&mut d, SIZE_SYMBOLS) as usize;
            observer.token(pos, previous, size);
            if size == 0 {
                let literal = &mut m.literals[pos % 4];
                let byte = literal.symbol(&mut d, params.literal_limit);
                out.push(byte as u8);
                produced += 1;
            } else {
                let length = if size < 61 {
                    size + 1
                } else {
                    LONG_COPIES[size - 61]
                };
                // GFX-GR2-2.8: the high and mid bounds follow the output of
                // this part, their thresholds do not.
                let r = (params.reference_limit as usize).min(produced) as u32;
                let low_bound = (params.reference_limit + 1).min(4);
                let low = m.low.symbol(&mut d, low_bound);
                let high = m.high.symbol(&mut d, r / 1024 + 1);
                let mid = m.mid[high as usize].symbol(&mut d, (r / 4 + 1).min(256));
                let distance = (high * 1024 + mid * 4 + low + 1) as usize;
                // GFX-GR2-2.10: no copy reaches before its own part.
                if distance > produced {
                    return Err(Oodle1Error::BadDistance { at: pos, distance });
                }
                if pos + length > end {
                    return Err(Oodle1Error::CopyPastPart { at: pos, length });
                }
                for _ in 0..length {
                    let byte = out[out.len() - distance];
                    out.push(byte);
                }
                produced += length;
            }
            previous = size;
            if d.damaged {
                return Err(Oodle1Error::Diverged {
                    used: 36 + d.pos,
                    stored: stored.len(),
                });
            }
        }
        observer.part_done(part, &m);
    }
    out.truncate(expanded);
    let used = 36 + d.pos;
    if !(1..=3).contains(&(stored.len() as i64 - used as i64)) {
        return Err(Oodle1Error::Diverged {
            used,
            stored: stored.len(),
        });
    }
    Ok((out, used))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rc00, GameDir};

    type Upkeep = Vec<(bool, Snapshot, Snapshot)>;

    #[derive(Default)]
    struct Probe {
        tokens: Vec<(usize, usize, usize)>,
        /// Per finished part: upkeep of literal model 0 and of the low model.
        parts: Vec<(usize, Upkeep, Upkeep)>,
    }

    impl Observer for Probe {
        fn token(&mut self, pos: usize, ctx: usize, size: usize) {
            self.tokens.push((pos, ctx, size));
        }
        fn part_done(&mut self, part: usize, m: &PartModels) {
            self.parts
                .push((part, m.literals[0].upkeep.clone(), m.low.upkeep.clone()));
        }
    }

    /// Decode one section of a game file, or `None` without `S2_GAME_DIR`.
    fn section(file: &str, index: usize) -> Option<(Vec<u8>, Probe)> {
        let dir = GameDir::from_env().ok()?;
        let path = dir.root().join(file);
        let raw = std::fs::read(&path).unwrap();
        let data = if rc00::is_wrapped(&raw) {
            rc00::decode(path.to_str().unwrap(), &raw).unwrap()
        } else {
            raw
        };
        let h = super::super::read_header(&data).unwrap();
        let s = &h.sections[index];
        let stored = &data[s.data_offset as usize..][..s.stored_size as usize];
        let mut probe = Probe::default();
        let (out, _) = decompress_observed(
            stored,
            s.expanded_size as usize,
            s.first16 as usize,
            s.first8 as usize,
            &mut probe,
        )
        .unwrap();
        Some((out, probe))
    }

    const SAWMILL: &str = "data/buildings/PB_R_Sawmill01.GR2";

    fn snap(
        values: &[u32],
        weights: &[u32],
        total: u32,
        table: &[u32],
        i: u32,
        t: u32,
    ) -> Snapshot {
        Snapshot {
            values: values.to_vec(),
            weights: weights.to_vec(),
            total,
            table: table.to_vec(),
            increment: i,
            threshold: t,
        }
    }

    /// GFX-GR2-2.6 example, without game data.
    #[test]
    fn recompute_example() {
        let mut m = Model::new(255, 72);
        m.values = vec![0, 69, 70];
        m.weights = vec![6, 2, 2];
        m.total = 10;
        m.recompute();
        assert_eq!(m.table, [0, 9830, 13106, 0x4000]);
        assert_eq!((m.increment, m.table_threshold), (8, 18));
        assert_eq!((m.halving_threshold, m.increment_cap), (8160, 510));
    }

    /// GFX-GR2-2.7 example, without game data.
    #[test]
    fn halving_example() {
        let mut m = Model::new(3, 4);
        m.values = vec![0, 0, 3, 1];
        m.weights = vec![10, 33, 208, 7];
        m.total = 258;
        m.table_threshold = 258;
        m.increment = 128;
        assert_eq!((m.halving_threshold, m.increment_cap), (256, 128));
        m.halve();
        m.recompute();
        assert_eq!(m.values, [0, 0, 1, 3]);
        assert_eq!(m.weights, [5, 16, 3, 104]);
        assert_eq!(m.total, 128);
        assert_eq!(m.table, [0, 640, 2688, 3072, 0x4000]);
        assert_eq!((m.increment, m.table_threshold), (128, 256));
    }

    /// GFX-GR2 T10
    // spec: GFX-GR2-2.5, GFX-GR2-2.9 (GFX-GR2 T10)
    #[test]
    fn t10_tokens() {
        let Some((out, p)) = section(SAWMILL, 0) else {
            return;
        };
        // pos, ctx, size of the first 20 tokens; literals are checked by value.
        let literals: [(usize, u8); 13] = [
            (0, 100),
            (1, 59),
            (2, 31),
            (3, 0),
            (4, 212),
            (5, 168),
            (6, 230),
            (7, 3),
            (8, 91),
            (9, 185),
            (10, 233),
            (11, 3),
            (12, 3),
        ];
        for (pos, value) in literals {
            assert_eq!(out[pos], value, "pos {pos}");
            assert_eq!(p.tokens[pos], (pos, 0, 0));
        }
        assert_eq!(p.tokens[13], (13, 0, 0));
        assert_eq!(out[13], 0);
        // Token 15: a copy of length 2 from distance 1.
        assert_eq!(p.tokens[14], (14, 0, 1));
        assert_eq!(out[14..16], [0, 0]);
        assert_eq!(p.tokens[15], (16, 1, 0));
        for (k, value) in [(16, 132u8), (17, 100), (18, 31), (19, 0), (20, 7)] {
            assert_eq!(out[k], value);
        }
        assert_eq!(p.tokens[16], (17, 0, 0));
        assert_eq!(p.tokens[19], (20, 0, 0));
        // The last token of part 0 is a copy of length 4 at 8524; part 2
        // starts at 8528 with fresh models and ctx 0.
        let last = p.tokens.iter().rposition(|t| t.0 < 8528).unwrap();
        assert_eq!(p.tokens[last], (8524, p.tokens[last].1, 3));
        assert_eq!(p.tokens[last + 1], (8528, 0, 0));
        assert_eq!(out[8528..8534], [51, 68, 32, 83, 116, 117]);
        assert!(p.tokens.contains(&(8542, 0, 1)));
    }

    /// GFX-GR2 T11
    // spec: GFX-GR2-2.6 (GFX-GR2 T11)
    #[test]
    fn t11_first_recompute() {
        let Some((_, p)) = section(SAWMILL, 2) else {
            return;
        };
        let (halving, before, after) = &p.parts[0].1[0];
        assert!(!halving);
        assert_eq!(before.values[1..], [69, 70]);
        assert_eq!(
            (
                &before.weights,
                before.total,
                &before.table,
                before.increment,
                before.threshold
            ),
            (&vec![6, 2, 2], 10, &vec![0, 0x4000], 4, 8)
        );
        assert_eq!(
            (&after.table, after.increment, after.threshold),
            (&vec![0, 9830, 13106, 16384], 8, 18)
        );
    }

    /// GFX-GR2 T12
    // spec: GFX-GR2-2.7 (GFX-GR2 T12)
    #[test]
    fn t12_first_halving() {
        let Some((_, p)) = section(SAWMILL, 0) else {
            return;
        };
        let (_, before, after) = p.parts[0].2.iter().find(|u| u.0).expect("a halving");
        assert_eq!(
            *before,
            snap(
                &[0, 0, 3, 1],
                &[10, 33, 208, 7],
                258,
                &[0, 1260, 5040, 15624, 16384],
                128,
                258
            )
        );
        assert_eq!(
            *after,
            snap(
                &[0, 0, 1, 3],
                &[5, 16, 3, 104],
                128,
                &[0, 640, 2688, 3072, 16384],
                128,
                256
            )
        );
    }
}
