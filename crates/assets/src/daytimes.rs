//! The day tables `data/settings/*.bin` (`GFX-world-view.md` GFX-VIEW-2.3b):
//! the light by hour. A map's own `<map name>.bin` is used if it exists, else
//! `daytimes.bin`; the XML form of GFX-VIEW-2.3 is read by the game only when
//! neither exists (the shipped install has both binary forms).

use crate::GameDir;

/// GFX-VIEW-2.3b: bytes per entry after the leading count.
const ENTRY: usize = 76;

/// One hour of a day table (GFX-VIEW-2.3, 2.3b).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayEntry {
    pub hour: i32,
    /// RGBA.
    pub background: [f32; 4],
    /// The skydome number (GFX-VIEW-6.3a: 1 is the `Stars` entry).
    pub skydome: i32,
    /// Whole degrees.
    pub elevation: i32,
    pub heading: i32,
    pub ambient: [f32; 4],
    pub light: [f32; 4],
    pub shadow_intensity: f32,
    pub fog_near: f32,
    pub fog_far: f32,
}

/// GFX-VIEW-2.3b: a table in its binary form: a little-endian `i32` count,
/// then 76 bytes per entry.
pub fn parse(bytes: &[u8]) -> Result<Vec<DayEntry>, String> {
    let i32_at = |o: usize| -> Result<i32, String> {
        bytes
            .get(o..o + 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| format!("day table: {} bytes, cut at {o}", bytes.len()))
    };
    let f32_at = |o: usize| i32_at(o).map(|v| f32::from_bits(v as u32));
    let quad = |o: usize| -> Result<[f32; 4], String> {
        Ok([f32_at(o)?, f32_at(o + 4)?, f32_at(o + 8)?, f32_at(o + 12)?])
    };
    let count = i32_at(0)?;
    let count = usize::try_from(count).map_err(|_| format!("day table: count {count}"))?;
    if bytes.len() < 4 + count.saturating_mul(ENTRY) {
        return Err(format!(
            "day table: {count} entries in {} bytes",
            bytes.len()
        ));
    }
    (0..count)
        .map(|k| {
            let o = 4 + k * ENTRY;
            Ok(DayEntry {
                hour: i32_at(o)?,
                background: quad(o + 4)?,
                skydome: i32_at(o + 20)?,
                elevation: i32_at(o + 24)?,
                heading: i32_at(o + 28)?,
                ambient: quad(o + 32)?,
                light: quad(o + 48)?,
                shadow_intensity: f32_at(o + 64)?,
                fog_near: f32_at(o + 68)?,
                fog_far: f32_at(o + 72)?,
            })
        })
        .collect()
}

/// GFX-VIEW-2.3b: the table a map's world uses, and the file it came from:
/// `data/settings/<map name>.bin`, else `data/settings/daytimes.bin`. None
/// when neither can be read (the game would then read `daytimes.xml`).
pub fn load(dir: &GameDir, map_name: &str) -> Option<(String, Vec<DayEntry>)> {
    [
        format!("data/settings/{map_name}.bin"),
        "data/settings/daytimes.bin".to_owned(),
    ]
    .into_iter()
    .find_map(|path| {
        let file = dir.find(&path).ok().filter(|f| f.is_file())?;
        let entries = parse(&std::fs::read(file).ok()?).ok()?;
        Some((path, entries))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_bytes(hour: i32, skydome: i32, elevation: i32, heading: i32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(hour.to_le_bytes());
        for v in [0.8f32, 0.9, 1.0, 1.0] {
            b.extend(v.to_le_bytes());
        }
        for v in [skydome, elevation, heading] {
            b.extend(v.to_le_bytes());
        }
        for v in [
            0.6f32, 0.5, 0.6, 1.0, 0.9375, 0.898, 0.781, 1.0, 0.9, 100.0, 200.0,
        ] {
            b.extend(v.to_le_bytes());
        }
        b
    }

    // GFX-VIEW-2.3b: an i32 count, then 76 bytes per entry: hour,
    // background, skydome, elevation, heading, ambient, light, shadow
    // intensity, fog near and far.
    // spec: GFX-VIEW-2.3b
    #[test]
    fn a_table_in_its_binary_form() {
        let mut bytes = 2i32.to_le_bytes().to_vec();
        bytes.extend(entry_bytes(12, 0, 50, 50));
        bytes.extend(entry_bytes(23, 1, -28, 3));
        assert_eq!(bytes.len(), 4 + 2 * 76);
        let table = parse(&bytes).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(
            (table[0].hour, table[0].elevation, table[0].heading),
            (12, 50, 50)
        );
        assert_eq!(table[0].ambient, [0.6, 0.5, 0.6, 1.0]);
        assert_eq!((table[1].skydome, table[1].fog_far), (1, 200.0));
        assert!(parse(&bytes[..100]).is_err());
    }

    // GFX-VIEW-2.3b: the shipped default table is a day cycle; a map
    // without its own table uses it, a map with one its own (constant
    // night light, the stars).
    // spec: GFX-VIEW-2.3b, GFX-VIEW-6.3a
    #[test]
    fn the_shipped_tables() {
        let Ok(dir) = GameDir::from_env() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let (file, table) = load(&dir, "MP_2P_Storm_Coast").expect("daytimes.bin");
        assert_eq!(file, "data/settings/daytimes.bin");
        let hours: Vec<i32> = table.iter().map(|e| e.hour).collect();
        assert_eq!(hours, [0, 1, 2, 3, 12, 20, 21, 22, 23]);
        let elevations: Vec<i32> = table.iter().map(|e| e.elevation).collect();
        assert_eq!(elevations, [-28, 20, 40, 50, 50, 50, 40, 15, -28]);
        let headings: Vec<i32> = table.iter().map(|e| e.heading).collect();
        assert_eq!(headings, [3, 20, 40, 50, 50, 50, 40, 24, 3]);
        let noon = table[4];
        assert_eq!(noon.ambient, [0.6, 0.5, 0.6, 1.0]);
        assert_eq!(noon.light, [0.9375, 0.898, 0.781, 1.0]);
        assert_eq!((noon.skydome, noon.shadow_intensity), (0, 0.9));
        assert_eq!((table[0].skydome, table[8].skydome), (1, 1));
        let (file, night) = load(&dir, "sp10_rat_race").expect("its own table");
        assert_eq!(file, "data/settings/sp10_rat_race.bin");
        assert!(night.iter().all(|e| e.elevation == -36 && e.skydome == 1));
    }
}
