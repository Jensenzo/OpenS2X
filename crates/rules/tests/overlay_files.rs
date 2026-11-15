//! A script in an overlay folder takes the place of the install's script of
//! the same path (`docs/extending.md`); without overlays the install's.

use s2_rules::{InstallSource, ScriptSource};

fn folder(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("s2-overlay-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (file, text) in files {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
    }
    dir
}

// spec: LUA-PROP-1.3 (names matched without regard to case); the overlay
// folders are ours (docs/extending.md)
#[test]
fn an_overlay_script_replaces_the_installs() {
    let install = folder(
        "install",
        &[
            ("data/scripts/properties/goods.lua", "-- install goods"),
            ("data/scripts/properties/jobs.lua", "-- install jobs"),
        ],
    );
    let overlay = folder(
        "next",
        &[("Data/Scripts/Properties/GOODS.lua", "-- next goods")],
    );
    let dir = s2_assets::GameDir::new(&install).unwrap();
    let read =
        |source: &InstallSource, path| String::from_utf8(source.read(path).unwrap()).unwrap();

    let plain = InstallSource::of(&dir);
    assert_eq!(
        read(&plain, "data\\scripts\\properties\\goods.lua"),
        "-- install goods"
    );

    let extended = InstallSource::of(&dir.clone().with_overlay(&overlay));
    assert_eq!(
        read(&extended, "data\\scripts\\properties\\goods.lua"),
        "-- next goods"
    );
    assert_eq!(
        read(&extended, "data\\scripts\\properties\\jobs.lua"),
        "-- install jobs"
    );

    std::fs::remove_dir_all(install).unwrap();
    std::fs::remove_dir_all(overlay).unwrap();
}
