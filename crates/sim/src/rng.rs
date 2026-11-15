//! The simulation's random number generator (`specs/SIM-RNG.md`): the
//! Park-Miller "minimal standard" generator, one instance in the world,
//! seeded [`SIM_SEED`] and reset at the end of every game set-up, so the
//! same start and the same commands give the same game.
//!
//! Each draw can be recorded with the place in the simulation that asked for
//! it ([`Rng::record_draws`]), for comparing our consumers with the
//! original's (`docs/rng-diff.md`). The record is not game state: it is left
//! out of equality, `Debug` and the state hash.

use std::panic::Location;

/// SIM-RNG-2.1: the simulation generator's seed.
pub const SIM_SEED: u32 = 0x5432_ACE0;

/// SIM-RNG-1.1: the multiplier and the modulus 2^31 - 1.
const MULTIPLIER: u64 = 16807;
const MODULUS: u64 = 0x7FFF_FFFF;

/// SIM-RNG-1.2: the order in which the seed repair sets or clears bits.
const REPAIR_BITS: [u32; 32] = [
    12, 23, 10, 25, 8, 27, 6, 29, 4, 30, 1, 22, 9, 13, 21, 0, 17, 26, 5, 15, 18, 28, 11, 2, 14, 3,
    24, 7, 19, 16, 20, 31,
];

/// A Park-Miller generator: its state and its seed (SIM-RNG-1.1), and how
/// many draws were made since the seed or the last reset.
#[derive(Clone)]
pub struct Rng {
    state: u32,
    seed: u32,
    draws: u64,
    /// Where each draw since recording began was asked for (not state).
    record: Option<Vec<&'static Location<'static>>>,
}

impl Rng {
    /// SIM-RNG-1.2: a generator seeded with `seed`, repaired so that it has
    /// between 8 and 24 set bits.
    pub fn new(seed: u32) -> Self {
        let seed = repair(seed);
        Self {
            state: if seed == 0 { 1 } else { seed },
            seed,
            draws: 0,
            record: None,
        }
    }

    /// SIM-RNG-1.1: the next value, 1 ..= 2^31 - 2. The caller reduces it
    /// itself (SIM-RNG-2.1); `#[track_caller]` names the caller in the
    /// record.
    #[track_caller]
    pub fn draw(&mut self) -> u32 {
        self.draw_for(Location::caller())
    }

    /// [`Rng::draw`] for a helper that draws on behalf of `caller` (the
    /// record names the caller).
    pub fn draw_for(&mut self, caller: &'static Location<'static>) -> u32 {
        self.state = ((self.state as u64 * MULTIPLIER) % MODULUS) as u32;
        self.draws += 1;
        if let Some(record) = &mut self.record {
            record.push(caller);
        }
        self.state
    }

    /// SIM-RNG-1.4: an integer in `lo ..= hi`, from one draw.
    #[track_caller]
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1).max(1) as u64;
        (lo as i64 + (self.draw() as u64 % span) as i64) as i32
    }

    /// SIM-RNG-1.4: a fraction 0.0 ..= 1.0 in steps of 1e-6, from one draw.
    #[track_caller]
    pub fn fraction(&mut self) -> f32 {
        (self.draw() % 1_000_001) as f32 * 0.000_001
    }

    /// SIM-RNG-1.3: back to the stored seed (bit 31 cleared), or 1 if it
    /// is 0; the draw count starts again.
    pub fn reset(&mut self) {
        let seed = self.seed & 0x7FFF_FFFF;
        self.state = if seed == 0 { 1 } else { seed };
        self.draws = 0;
    }

    /// SIM-RNG-2.3: the two words a save holds, the state and the seed, as
    /// they were (a state of 0 or above 2^31 - 2 would stay there: 1).
    pub fn restore(&mut self, state: u32, seed: u32) {
        self.state = if state == 0 || state as u64 >= MODULUS {
            1
        } else {
            state
        };
        self.seed = seed;
        self.draws = 0;
    }

    /// The state: the last value drawn, or the seed before any draw.
    pub fn state(&self) -> u32 {
        self.state
    }

    /// The stored (repaired) seed.
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// Draws since the seed or the last reset (TOOL-trace TRACE-4.6, the
    /// draw index).
    pub fn draws(&self) -> u64 {
        self.draws
    }

    /// Start recording where each draw is asked for (`docs/rng-diff.md`).
    pub fn record_draws(&mut self) {
        self.record.get_or_insert_with(Vec::new);
    }

    /// The places recorded since the last call, oldest first.
    pub fn take_record(&mut self) -> Vec<&'static Location<'static>> {
        self.record.as_mut().map(std::mem::take).unwrap_or_default()
    }
}

impl PartialEq for Rng {
    fn eq(&self, other: &Self) -> bool {
        (self.state, self.seed, self.draws) == (other.state, other.seed, other.draws)
    }
}

impl Eq for Rng {}

impl std::fmt::Debug for Rng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rng")
            .field("state", &self.state)
            .field("seed", &self.seed)
            .field("draws", &self.draws)
            .finish()
    }
}

/// SIM-RNG-1.2: clear bit 31; with fewer than 8 set bits among bits 0..30,
/// set the first 8 - n bits of the repair order (one already set counts);
/// with more than 24, clear the first 32 - n (one already clear counts).
fn repair(seed: u32) -> u32 {
    let mut seed = seed & 0x7FFF_FFFF;
    let n = seed.count_ones();
    if n < 8 {
        for &bit in &REPAIR_BITS[..(8 - n) as usize] {
            seed |= 1 << bit;
        }
    } else if n > 24 {
        for &bit in &REPAIR_BITS[..(32 - n) as usize] {
            seed &= !(1 << bit);
        }
    }
    seed
}

/// The record of draws is not state and is not written (`docs/snapshot.md`).
impl crate::snap::Snap for Rng {
    fn put(&self, w: &mut crate::snap::Writer) {
        w.put(&self.state);
        w.put(&self.seed);
        w.put(&self.draws);
    }
    fn get(r: &mut crate::snap::Reader) -> Result<Self, crate::snap::SnapError> {
        Ok(Self {
            state: r.get()?,
            seed: r.get()?,
            draws: r.get()?,
            record: None,
        })
    }
    fn shape(s: &mut crate::snap::Shape) -> String {
        s.define("s2_sim::rng::Rng", |s| {
            vec![
                s.field("state", |v: &Self| &v.state),
                s.field("seed", |v: &Self| &v.seed),
                s.field("draws", |v: &Self| &v.draws),
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draws(seed: u32, n: usize) -> Vec<u32> {
        let mut rng = Rng::new(seed);
        (0..n).map(|_| rng.draw()).collect()
    }

    // spec: SIM-RNG-1.2 (T1 to T4)
    #[test]
    fn t1_to_t4_the_seed_repair() {
        assert_eq!(Rng::new(0).seed(), 0x2A80_1540);
        assert_eq!(Rng::new(0).state(), 713_037_120);
        assert_eq!(Rng::new(12345).seed(), 0x0080_3039);
        assert_eq!(Rng::new(1).seed(), 0x0A80_1541);
        assert_eq!(Rng::new(0x7FFF_FFFF).seed(), 0x7FFF_EFFF);
        assert_eq!(Rng::new(0xFFFF_FFFF).seed(), 0x7FFF_EFFF);
        // SIM-RNG-2.1: the game seed has 13 bits and stays as it is.
        assert_eq!(Rng::new(SIM_SEED).seed(), SIM_SEED);
    }

    // spec: SIM-RNG-1.1 (T5)
    #[test]
    fn t5_ten_thousand_draws_from_state_1() {
        let mut rng = Rng {
            state: 1,
            seed: 1,
            draws: 0,
            record: None,
        };
        for _ in 0..10_000 {
            rng.draw();
        }
        assert_eq!(rng.state(), 1_043_618_065);
        assert_eq!(rng.draws(), 10_000);
    }

    // spec: SIM-RNG-1.1, 1.2, 2.1 (T9: the first 20 draws)
    #[test]
    fn t9_the_first_20_draws() {
        assert_eq!(
            draws(0, 20),
            [
                1056125580, 1350280605, 1706430386, 331391817, 1277171648, 1324836171, 1411073901,
                1257140286, 1812667616, 1301605770, 1819748048, 43342162, 454760401, 263759934,
                606963330, 685364060, 1958957559, 1127901956, 810022423, 1148025028
            ]
        );
        assert_eq!(
            draws(12345, 20),
            [
                1608380016, 1666264123, 1714358381, 433217668, 1119782746, 1789413361, 1309365739,
                1245044564, 383330780, 189478460, 1993712366, 1136391221, 1755178576, 1450951640,
                1467401795, 919766417, 926879413, 215918953, 1849963288, 1064740150
            ]
        );
        assert_eq!(
            draws(SIM_SEED, 20),
            [
                1257492815, 1295171578, 1054465454, 1365830334, 1057720755, 243099419, 1258038539,
                1877220258, 1758618129, 1277460442, 1883629635, 2106835018, 1865775790, 537489036,
                1262008770, 2032899618, 479055956, 577259889, 1823320924, 2110610625
            ]
        );
    }

    // spec: SIM-RNG-1.4 (T7, T8)
    #[test]
    fn t7_t8_ranges_and_fractions() {
        let mut rng = Rng::new(SIM_SEED);
        let r: Vec<i32> = (0..10).map(|_| rng.range(0, 5)).collect();
        assert_eq!(r, [5, 4, 2, 0, 3, 5, 5, 0, 3, 4]);
        let mut rng = Rng::new(SIM_SEED);
        let f: Vec<f32> = (0..3).map(|_| rng.fraction()).collect();
        for (got, want) in f.iter().zip([0.491558f32, 0.170283, 0.4644]) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
    }

    // spec: SIM-RNG-1.3, 2.2 (T6, T11: every new game starts at the seed)
    #[test]
    fn reset_goes_back_to_the_seed() {
        let mut rng = Rng::new(SIM_SEED);
        for _ in 0..19_425 {
            rng.draw();
        }
        assert_ne!(rng.state(), SIM_SEED);
        rng.reset();
        assert_eq!(
            (rng.state(), rng.seed(), rng.draws()),
            (SIM_SEED, SIM_SEED, 0)
        );
        assert_eq!(rng.draw(), 1257492815);
    }

    // The record names the place that drew and is not game state.
    // spec: none (the draw record is a tool, docs/rng-diff.md)
    #[test]
    fn the_record_names_the_caller_and_is_not_state() {
        let mut a = Rng::new(SIM_SEED);
        let mut b = Rng::new(SIM_SEED);
        a.record_draws();
        a.draw();
        b.draw();
        assert_eq!(a, b);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let record = a.take_record();
        assert_eq!(record.len(), 1);
        assert!(record[0].file().ends_with("rng.rs"));
        assert!(a.take_record().is_empty());
    }
}
