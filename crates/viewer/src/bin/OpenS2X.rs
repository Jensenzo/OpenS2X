//! `OpenS2X`: the game. It opens at the main menu; the first run asks
//! for the game folder. (The program's public name; its crates, its
//! settings folder `%APPDATA%\s2-reborn` and its saves' marks keep the
//! repository's name, so a profile made before the rename is found as
//! before.)

// No console window on Windows.
#![windows_subsystem = "windows"]

fn main() -> std::process::ExitCode {
    s2_viewer::run(true)
}
