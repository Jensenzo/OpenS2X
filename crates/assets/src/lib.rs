//! Loaders for the original game's data files.
//!
//! This is the only crate that knows original file formats. Each loader must
//! cite the `FMT-*` spec it implements.

pub mod anim_tables;
pub mod bmfont;
pub mod bmp;
pub mod cfg;
pub mod color_slots;
pub mod daytimes;
pub mod dds;
pub mod glb;
pub mod gr2;
pub mod json;
pub mod landscape;
pub mod lang;
pub mod menu_cfg;
pub mod model_tables;
pub mod pfx;
pub mod rc00;
pub mod s2m;
pub mod sav;
pub mod subtitles;
pub mod terrain_patterns;
pub mod tga;
pub mod wav;
pub mod xml;

use std::path::{Path, PathBuf};

/// Location of the player's own game install, and the overlay folders a
/// project built on this engine adds in front of it (`docs/extending.md`).
#[derive(Debug, Clone)]
pub struct GameDir {
    root: PathBuf,
    overlays: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum GameDirError {
    NotSet,
    Missing(PathBuf),
}

impl std::fmt::Display for GameDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSet => write!(f, "S2_GAME_DIR is not set"),
            Self::Missing(p) => write!(f, "game directory not found: {}", p.display()),
        }
    }
}

impl std::error::Error for GameDirError {}

impl GameDir {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, GameDirError> {
        let root = root.into();
        if root.is_dir() {
            Ok(Self {
                root,
                overlays: Vec::new(),
            })
        } else {
            Err(GameDirError::Missing(root))
        }
    }

    /// Resolve from the `S2_GAME_DIR` environment variable.
    pub fn from_env() -> Result<Self, GameDirError> {
        let v = std::env::var_os("S2_GAME_DIR").ok_or(GameDirError::NotSet)?;
        Self::new(v)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Add an overlay folder: laid out like the install (`data/...`), its
    /// files take the place of the install's files of the same path. Not
    /// part of the original game; the later an overlay is added, the more
    /// it wins.
    pub fn with_overlay(mut self, folder: impl Into<PathBuf>) -> Self {
        self.overlays.push(folder.into());
        self
    }

    /// The overlay folders, in the order they were added.
    pub fn overlays(&self) -> &[PathBuf] {
        &self.overlays
    }

    /// [`resolve`] `path` in the overlays (the last added first), then in
    /// the install. Without overlays, the install's file.
    pub fn find(&self, path: &str) -> Result<PathBuf, String> {
        find_in(&self.overlays, &self.root, path)
    }
}

/// [`resolve`] `path` in `overlays` (the last first), then below `root`.
pub fn find_in(overlays: &[PathBuf], root: &Path, path: &str) -> Result<PathBuf, String> {
    for overlay in overlays.iter().rev() {
        if let Ok(found) = resolve(overlay, path) {
            if found.is_file() {
                return Ok(found);
            }
        }
    }
    resolve(root, path)
}

/// Find `path` (components separated by `\` or `/`) below `root`, matching
/// each component without regard to ASCII case. An exact match wins.
pub fn resolve(root: &Path, path: &str) -> Result<PathBuf, String> {
    let mut found = root.to_path_buf();
    for part in path.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let exact = found.join(part);
        if exact.exists() {
            found = exact;
            continue;
        }
        let entries = std::fs::read_dir(&found).map_err(|e| format!("{path}: {e}"))?;
        let mut matches = entries
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().eq_ignore_ascii_case(part));
        let name = matches
            .next()
            .ok_or_else(|| format!("{path}: no file or folder matching {part}"))?;
        if matches.next().is_some() {
            return Err(format!("{path}: more than one name matches {part}"));
        }
        found.push(name);
    }
    Ok(found)
}

/// FMT-3.3: the map file of a map's name (`MP_2P_Storm_Coast`,
/// `campaign01`, `tutorial01`): the first of the install's free game,
/// campaign and tutorial map folders that has `<name>.s2m`, or `name`
/// itself when it is the path of an existing `.s2m` file. Maps are found
/// in the install, not in the overlay folders (`docs/extending.md`).
pub fn find_map(dir: &GameDir, name: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(name);
    if name.ends_with(".s2m") && direct.exists() {
        return Some(direct);
    }
    ["freeGameMaps", "campaignMaps", "tutorialMaps"]
        .iter()
        .map(|folder| dir.root().join(format!("data/maps/{folder}/{name}.s2m")))
        .find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: FMT-3.3 (the map folders, searched in order)
    #[test]
    fn maps_by_name() {
        let root = std::env::temp_dir().join(format!("s2-find-map-{}", std::process::id()));
        for folder in ["freeGameMaps", "tutorialMaps"] {
            std::fs::create_dir_all(root.join("data/maps").join(folder)).unwrap();
        }
        let free = root.join("data/maps/freeGameMaps/MP_2P_Test.s2m");
        let lesson = root.join("data/maps/tutorialMaps/tutorial01.s2m");
        std::fs::write(&free, b"").unwrap();
        std::fs::write(&lesson, b"").unwrap();
        let dir = GameDir::new(&root).unwrap();
        assert_eq!(find_map(&dir, "MP_2P_Test"), Some(free.clone()));
        assert_eq!(find_map(&dir, "tutorial01"), Some(lesson));
        assert_eq!(find_map(&dir, "campaign01"), None);
        let path = free.to_string_lossy().into_owned();
        assert_eq!(find_map(&dir, &path), Some(free));
        std::fs::remove_dir_all(&root).unwrap();
    }

    // spec: FMT-1.1
    #[test]
    fn rejects_missing_dir() {
        assert!(GameDir::new("/definitely/not/here").is_err());
    }

    fn folder(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("s2-gamedir-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for file in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, name).unwrap();
        }
        dir
    }

    // spec: FMT-1.1 (the lookup ignores case); the overlays are ours
    // (docs/extending.md)
    #[test]
    fn overlays_win_over_the_install_the_last_most() {
        let install = folder("install", &["DATA/Sound/a.wav", "DATA/gfx/b.dds"]);
        let first = folder("first", &["data/sound/a.wav", "data/gfx/b.dds"]);
        let second = folder("second", &["Data/GFX/B.dds"]);
        let plain = GameDir::new(&install).unwrap();
        let read = |dir: &GameDir, path| std::fs::read_to_string(dir.find(path).unwrap()).unwrap();
        assert_eq!(read(&plain, "data\\sound\\A.WAV"), "install");
        let dir = plain.clone().with_overlay(&first).with_overlay(&second);
        assert_eq!(dir.overlays().len(), 2);
        assert_eq!(read(&dir, "data/sound/a.wav"), "first");
        assert_eq!(read(&dir, "data/gfx/b.dds"), "second");
        assert!(dir.find("data/none.txt").is_err());
        for d in [install, first, second] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }
}
