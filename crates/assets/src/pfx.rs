//! Particle effects, `data/scripts/effects/*.pfx` (`GFX-objects.md`
//! GFX-OBJ-8.3a): one emitter, its particle's texture rectangle, the pool
//! size and four curves over a particle's life.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct PfxError(pub String);

impl fmt::Display for PfxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pfx: {}", self.0)
    }
}

impl std::error::Error for PfxError {}

/// GFX-OBJ-8.3a: a curve over a particle's life, t = age / life in 0..1.
#[derive(Debug, Clone, PartialEq)]
pub enum Curve {
    /// Type 0: c.
    Constant(f32),
    /// Type 1: start + (end - start) t.
    Linear(f32, f32),
    /// Type 2: start + (end - start) (1 - cos(pi t)) / 2.
    Eased(f32, f32),
    /// Type 3: a + (c - b) t + ((b - a) - K) t^2 + K t^3, K = d - c - b + a.
    Cubic([f32; 4]),
    /// Types 4 to 6 (tension and bias, periodic): kept, not evaluated (the
    /// icons do not use them); they evaluate as 1.
    Other(i32, Vec<f32>),
}

impl Curve {
    pub fn at(&self, t: f32) -> f32 {
        match *self {
            Curve::Constant(c) => c,
            Curve::Linear(s, e) => s + (e - s) * t,
            Curve::Eased(s, e) => s + (e - s) * (1.0 - (std::f32::consts::PI * t).cos()) / 2.0,
            Curve::Cubic([a, b, c, d]) => {
                let k = d - c - b + a;
                a + (c - b) * t + ((b - a) - k) * t * t + k * t * t * t
            }
            Curve::Other(..) => 1.0,
        }
    }
}

/// GFX-OBJ-8.3a: one effect.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub version: String,
    pub emitter: String,
    /// 0: no end.
    pub max_loops: i32,
    pub pos: [f32; 3],
    pub pos_var: [f32; 3],
    /// Per second.
    pub vel: [f32; 3],
    pub vel_var: [f32; 3],
    pub size: f32,
    pub size_var: f32,
    /// A particle's life (s).
    pub ttl: f32,
    pub ttl_var: f32,
    /// Seconds between births.
    pub spawn: f32,
    pub spawn_var: f32,
    pub color: [f32; 4],
    pub color_var: [f32; 4],
    pub particle: String,
    /// u0, v0, u1, v1 in the effect texture.
    pub rect: [f32; 4],
    pub count: i32,
    pub curve_set: String,
    /// Multipliers over life: velocity, size, colour (r, g, b), alpha.
    pub curves: [Curve; 4],
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], PfxError> {
        let end = self.at + n;
        let bytes = self
            .data
            .get(self.at..end)
            .ok_or_else(|| PfxError(format!("ends at byte {} (wanted {n} more)", self.at)))?;
        self.at = end;
        Ok(bytes)
    }
    fn i32(&mut self) -> Result<i32, PfxError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, PfxError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32s<const N: usize>(&mut self) -> Result<[f32; N], PfxError> {
        let mut out = [0.0; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Ok(out)
    }
    /// A u32 length and that many bytes, no terminator.
    fn string(&mut self) -> Result<String, PfxError> {
        let n = self.i32()?;
        let n = usize::try_from(n).map_err(|_| PfxError(format!("string length {n}")))?;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
    fn curve(&mut self) -> Result<Curve, PfxError> {
        Ok(match self.i32()? {
            0 => Curve::Constant(self.f32()?),
            1 => {
                let [s, e] = self.f32s()?;
                Curve::Linear(s, e)
            }
            2 => {
                let [s, e] = self.f32s()?;
                Curve::Eased(s, e)
            }
            3 => Curve::Cubic(self.f32s()?),
            4 => Curve::Other(4, self.f32s::<6>()?.to_vec()),
            t @ (5 | 6) => Curve::Other(t, self.f32s::<4>()?.to_vec()),
            t => return Err(PfxError(format!("curve type {t}"))),
        })
    }
}

/// GFX-OBJ-8.3a: read an effect; little-endian, no padding.
pub fn parse(data: &[u8]) -> Result<Effect, PfxError> {
    let mut r = Reader { data, at: 0 };
    let version = r.string()?;
    let emitter = r.string()?;
    let max_loops = r.i32()?;
    let pos = r.f32s()?;
    let pos_var = r.f32s()?;
    let vel = r.f32s()?;
    let vel_var = r.f32s()?;
    let [size, size_var, ttl, ttl_var, spawn, spawn_var] = r.f32s()?;
    let color = r.f32s()?;
    let color_var = r.f32s()?;
    let particle = r.string()?;
    let rect = r.f32s()?;
    let count = r.i32()?;
    let curve_set = r.string()?;
    let curves = [r.curve()?, r.curve()?, r.curve()?, r.curve()?];
    if version == "0002" {
        r.i32()?;
    }
    Ok(Effect {
        version,
        emitter,
        max_loops,
        pos,
        pos_var,
        vel,
        vel_var,
        size,
        size_var,
        ttl,
        ttl_var,
        spawn,
        spawn_var,
        color,
        color_var,
        particle,
        rect,
        count,
        curve_set,
        curves,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(out: &mut Vec<u8>, s: &str) {
        out.extend((s.len() as u32).to_le_bytes());
        out.extend(s.as_bytes());
    }
    fn floats(out: &mut Vec<u8>, v: &[f32]) {
        for f in v {
            out.extend(f.to_le_bytes());
        }
    }

    /// GFX-OBJ-8.3a: the Sleep icon's values, written in the file's order.
    fn sleep() -> Vec<u8> {
        let mut b = Vec::new();
        string(&mut b, "0002");
        string(&mut b, "Default");
        b.extend(0i32.to_le_bytes());
        floats(&mut b, &[0.0; 6]);
        floats(&mut b, &[0.5, 2.5, 0.0, 0.3, 0.25, 0.1]);
        floats(&mut b, &[1.0, 0.2, 6.0, 0.0, 0.8, 0.1]);
        floats(&mut b, &[1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        string(&mut b, "Decal");
        floats(&mut b, &[0.0, 0.3125, 0.0625, 0.375]);
        b.extend(4i32.to_le_bytes());
        string(&mut b, "Default");
        b.extend(1i32.to_le_bytes());
        floats(&mut b, &[1.0, 0.6]);
        b.extend(3i32.to_le_bytes());
        floats(&mut b, &[0.0, -2.0, 2.0, 1.0]);
        b.extend(0i32.to_le_bytes());
        floats(&mut b, &[1.0]);
        b.extend(3i32.to_le_bytes());
        floats(&mut b, &[1.0, 0.0, 0.0, -2.0]);
        b.extend(0i32.to_le_bytes());
        b
    }

    // spec: GFX-OBJ-8.3a
    #[test]
    fn sleep_icon_values() {
        let e = parse(&sleep()).unwrap();
        assert_eq!(e.vel, [0.5, 2.5, 0.0]);
        assert_eq!((e.ttl, e.spawn, e.count), (6.0, 0.8, 4));
        assert_eq!(e.rect, [0.0, 0.3125, 0.0625, 0.375]);
        // 8.3a: velocity x (1 - 0.4 t), size x (4 t - 3 t^2 + t^3), alpha
        // x (1 - t^3).
        for t in [0.0, 0.25, 0.5, 1.0f32] {
            assert!((e.curves[0].at(t) - (1.0 - 0.4 * t)).abs() < 1e-6);
            let size = 4.0 * t - 3.0 * t * t + t * t * t;
            assert!((e.curves[1].at(t) - size).abs() < 1e-6, "{t}");
            assert_eq!(e.curves[2].at(t), 1.0);
            assert!((e.curves[3].at(t) - (1.0 - t * t * t)).abs() < 1e-6, "{t}");
        }
        assert!(parse(&sleep()[..40]).is_err());
    }

    /// GFX-OBJ-8.3a: the five Sleep effects of the install: the same
    /// emitter, their icon rectangles (the both-icons one reaches v 0.4125).
    /// Skipped without `S2_GAME_DIR`.
    #[test]
    fn install_sleep_effects() {
        let Some(dir) = std::env::var_os("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let dir = std::path::Path::new(&dir).join("data/scripts/effects");
        for (name, rect) in [
            ("Sleep", [0.0, 0.3125, 0.0625, 0.375]),
            ("Sleep_ProductionOff", [0.0625, 0.3125, 0.125, 0.375]),
            ("Sleep_NoSoldier", [0.125, 0.3125, 0.1875, 0.375]),
            ("Sleep_NoCoin", [0.1875, 0.3125, 0.25, 0.375]),
            ("Sleep_NoSoldierAndCoin", [0.125, 0.3125, 0.25, 0.4125]),
        ] {
            let data = std::fs::read(dir.join(format!("{name}.pfx"))).unwrap();
            let e = parse(&data).unwrap();
            assert_eq!(e.rect, rect, "{name}");
            assert_eq!((e.vel, e.vel_var), ([0.5, 2.5, 0.0], [0.3, 0.25, 0.1]));
            assert_eq!((e.size, e.size_var, e.ttl, e.count), (1.0, 0.2, 6.0, 4));
            assert_eq!((e.spawn, e.spawn_var), (0.8, 0.1));
        }
    }
}
