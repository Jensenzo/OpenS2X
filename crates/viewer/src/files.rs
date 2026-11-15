//! Files read ahead off the main thread. The models and animations
//! (`.gr2`), the sounds (`.wav`) and the models' textures (`.dds`) are read
//! the first time a game shows or plays a thing; on a slow disk those first
//! reads stalled the frame they fell in, many of them in a game's first
//! minute. A background thread started with the program reads them ahead
//! into memory, and the readers take a file's bytes from there when it has
//! them, else read the disk as before. The bytes are the files' own, so
//! what is drawn and played is the same. Ours: no spec covers when a
//! program reads its files.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The install's folders below `data` read ahead, in the order a game
/// meets them: settlers and buildings at the start view, their sounds,
/// trees and the rest.
const FOLDERS: [&str; 7] = [
    "character",
    "buildings",
    "sound",
    "enviroment",
    "items",
    "animals",
    "ships",
];

/// The kinds of file read ahead (extensions, any case).
const KINDS: [&str; 3] = ["gr2", "wav", "dds"];

/// At most this many bytes are held at once; the rest is read on demand.
const MOST: usize = 256 << 20;

#[derive(Default)]
struct Ahead {
    /// The bytes read ahead and not taken yet, by [`key`].
    files: HashMap<String, Vec<u8>>,
    /// Files a reader asked for before they were read ahead: not kept.
    taken: HashSet<String>,
    held: usize,
}

static AHEAD: OnceLock<Mutex<Ahead>> = OnceLock::new();

/// A path's key: its text without regard to ASCII case or separator, as
/// [`s2_assets::resolve`] matches names.
fn key(path: &Path) -> String {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .replace('\\', "/")
}

/// Start reading ahead below the install `root` (once; later calls do
/// nothing).
pub(crate) fn start(root: &Path) {
    if AHEAD.set(Mutex::new(Ahead::default())).is_err() {
        return;
    }
    let root = root.to_path_buf();
    let _ = std::thread::Builder::new()
        .name("files-ahead".into())
        .spawn(move || {
            let Ok(data) = s2_assets::resolve(&root, "data") else {
                return;
            };
            for folder in FOLDERS {
                if let Ok(dir) = s2_assets::resolve(&data, folder) {
                    if !read_below(&dir) {
                        return;
                    }
                }
            }
        });
}

/// Read every file of [`KINDS`] below `dir` ahead, in name order; false
/// once [`MOST`] is held.
fn read_below(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            if !read_below(&path) {
                return false;
            }
            continue;
        }
        let wanted = path
            .extension()
            .is_some_and(|e| KINDS.iter().any(|k| e.eq_ignore_ascii_case(k)));
        if !wanted {
            continue;
        }
        let k = key(&path);
        let Some(ahead) = AHEAD.get() else {
            return false;
        };
        {
            let a = ahead.lock().expect("files ahead");
            if a.held >= MOST {
                return false;
            }
            if a.taken.contains(&k) || a.files.contains_key(&k) {
                continue;
            }
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let mut a = ahead.lock().expect("files ahead");
        if !a.taken.contains(&k) {
            a.held += bytes.len();
            a.files.insert(k, bytes);
        }
    }
    true
}

/// A file's bytes: those read ahead if there are (they are given away, the
/// readers keep what they make of them), else read now.
pub(crate) fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    if let Some(ahead) = AHEAD.get() {
        let k = key(path);
        let mut a = ahead.lock().expect("files ahead");
        if let Some(bytes) = a.files.remove(&k) {
            a.held -= bytes.len();
            return Ok(bytes);
        }
        a.taken.insert(k);
    }
    std::fs::read(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes read ahead are the file's, given once; a file of another
    /// kind or one asked for first is read from the disk; with nothing read
    /// ahead `read` is `std::fs::read`.
    // spec: none (ours: when files are read)
    #[test]
    fn files_read_ahead_are_the_files_own_bytes() {
        let root = std::env::temp_dir().join(format!("s2-files-ahead-{}", std::process::id()));
        let models = root.join("Data").join("Character").join("Romans");
        std::fs::create_dir_all(&models).unwrap();
        let model = models.join("carrier.GR2");
        std::fs::write(&model, b"model bytes").unwrap();
        let note = models.join("notes.txt");
        std::fs::write(&note, b"not read ahead").unwrap();
        start(&root);
        // Wait for the thread (a few files).
        for _ in 0..200 {
            let held = AHEAD.get().unwrap().lock().unwrap().files.len();
            if held == 1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(AHEAD.get().unwrap().lock().unwrap().files.len(), 1);
        // Another spelling of the same path finds it.
        let other = PathBuf::from(
            model
                .to_string_lossy()
                .replace("carrier.GR2", "CARRIER.gr2"),
        );
        assert_eq!(read(&other).unwrap(), b"model bytes");
        assert!(AHEAD.get().unwrap().lock().unwrap().files.is_empty());
        // Given once; the next read is the disk's (the same bytes).
        assert_eq!(read(&model).unwrap(), b"model bytes");
        assert_eq!(read(&note).unwrap(), b"not read ahead");
        let _ = std::fs::remove_dir_all(&root);
    }
}
