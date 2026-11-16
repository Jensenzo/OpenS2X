//! Which version the program is (docs/releasing.md, "The version"): the
//! line `--version` prints, the first line of the game's log file, and
//! (from `build.rs`) the Windows file properties of the executable. The
//! original's menus show no version, so ours show none either.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The release this program was built from (the workspace's version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version line: `OpenS2X 0.11.1`; a project that names its window
/// says which OpenS2X it runs on, `Frontier (OpenS2X 0.11.1)`.
pub(crate) fn line(window_title: &str) -> String {
    if window_title == "OpenS2X" {
        format!("OpenS2X {VERSION}")
    } else {
        format!("{window_title} (OpenS2X {VERSION})")
    }
}

/// The game's log file: `log.txt` in the folder of the program's settings
/// (`%APPDATA%\s2-reborn` on Windows; `--profile-dir`'s folder or a
/// project's when given).
pub(crate) fn log_path(settings_file: &Path) -> Option<PathBuf> {
    Some(settings_file.parent()?.join("log.txt"))
}

/// The open log file, which the log's file layer and the panic note write
/// to; unset in the viewer and when the file could not be made.
static LOG: OnceLock<File> = OnceLock::new();

/// Start the log file at `path`, emptied, with the version line first.
/// The log's messages follow (the file layer of Bevy's log plugin,
/// [`file_layer`]), and a panic's message with its place (a note
/// written before the program ends). Without a folder or a writable file
/// the game runs without one.
pub(crate) fn start_log(path: &Path, version_line: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = File::create(path)?;
    writeln!(file, "{version_line}")?;
    if LOG.set(file).is_ok() {
        let earlier = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(mut file) = LOG.get() {
                let thread = std::thread::current();
                let _ = writeln!(
                    file,
                    "panic in thread '{}': {info}",
                    thread.name().unwrap_or("unnamed")
                );
            }
            earlier(info);
        }));
    }
    Ok(())
}

/// Writes to the log file when it is open, else nowhere.
struct LogFile;

impl Write for LogFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match LOG.get() {
            Some(mut file) => file.write(bytes),
            None => Ok(bytes.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match LOG.get() {
            Some(mut file) => file.flush(),
            None => Ok(()),
        }
    }
}

/// Bevy's log plugin's custom layer: the same messages as on the console
/// (the plugin's filter applies to both), without colour codes, into the
/// log file. None without one.
pub(crate) fn file_layer(_: &mut bevy::app::App) -> Option<bevy::log::BoxedLayer> {
    LOG.get()?;
    Some(Box::new(
        bevy::log::tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(|| LogFile),
    ))
}

/// `--version` in a program without a console of its own (`OpenS2X.exe`
/// is a windowed program): print to the console it was started from, if
/// its output is not already sent somewhere (a pipe or a file).
#[cfg(windows)]
pub(crate) fn use_parent_console() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };
    // SAFETY: plain calls that take and return values, no pointers.
    unsafe {
        let out = GetStdHandle(STD_OUTPUT_HANDLE);
        if out.is_null() || out == INVALID_HANDLE_VALUE {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn use_parent_console() {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The version line names the program and the workspace's version; a
    /// project's names the project and the OpenS2X it runs on.
    // spec: none (the program's version, docs/releasing.md)
    #[test]
    fn the_version_line() {
        assert_eq!(
            line("OpenS2X"),
            format!("OpenS2X {}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(line("Frontier"), format!("Frontier (OpenS2X {VERSION})"));
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.parse::<u16>().is_ok()));
    }

    /// The log file sits beside the settings and starts with the version
    /// line; a second start empties it.
    // spec: none (the program's version, docs/releasing.md)
    #[test]
    fn the_log_starts_with_the_version() {
        let dir = std::env::temp_dir().join(format!("s2-version-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = log_path(&dir.join("settings.txt")).unwrap();
        assert_eq!(path, dir.join("log.txt"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "an older run\nits messages\n").unwrap();
        start_log(&path, &line("OpenS2X")).unwrap();
        let mut writer = LogFile;
        writer.write_all(b"a message\n").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines, [line("OpenS2X").as_str(), "a message"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
