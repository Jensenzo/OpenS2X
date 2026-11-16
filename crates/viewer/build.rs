//! The Windows file properties of the programs (docs/releasing.md, "The
//! version"): a version resource with the workspace's version, the
//! product's name and the file's description, so Explorer's Properties >
//! Details (and Task Manager) say which OpenS2X a file is.
//!
//! Made for the Windows GNU target (the release build, `scripts/dist.sh`)
//! with MinGW's `windres` (`WINDRES` to name another). Elsewhere, or without
//! the tool, the programs are built without it (`dist.sh` checks that the
//! download's program has it).

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../Cargo.toml");
    println!("cargo:rerun-if-env-changed=WINDRES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu") {
        println!("cargo:warning=no version resource: made only for the Windows GNU target");
        return;
    }
    let version = env::var("CARGO_PKG_VERSION").expect("cargo sets the version");
    let numbers = ["MAJOR", "MINOR", "PATCH"]
        .map(|part| env::var(format!("CARGO_PKG_VERSION_{part}")).expect("cargo sets it"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let windres = env::var("WINDRES").unwrap_or_else(|_| {
        if env::var("HOST").is_ok_and(|h| h.contains("windows")) {
            "windres".into()
        } else {
            "x86_64-w64-mingw32-windres".into()
        }
    });
    for (bin, description) in [
        ("OpenS2X", "OpenS2X"),
        ("s2-viewer", "s2-viewer, the map viewer of OpenS2X"),
    ] {
        let rc = out.join(format!("{bin}.rc"));
        let obj = out.join(format!("{bin}-version.o"));
        std::fs::write(&rc, resource(bin, description, &version, &numbers))
            .expect("write the resource script");
        let made = Command::new(&windres)
            .arg("--input-format=rc")
            .arg("--output-format=coff")
            .arg("-i")
            .arg(&rc)
            .arg("-o")
            .arg(&obj)
            .status();
        match made {
            Ok(status) if status.success() => {
                println!("cargo:rustc-link-arg-bin={bin}={}", obj.display());
            }
            Ok(status) => {
                println!(
                    "cargo:warning=no version resource for {bin}: {windres} ended with {status}"
                );
            }
            Err(e) => println!("cargo:warning=no version resource for {bin}: {windres}: {e}"),
        }
    }
}

/// The resource script: the fixed version numbers (`0,11,1,0`) and the
/// strings Explorer shows (US English, Unicode).
fn resource(bin: &str, description: &str, version: &str, numbers: &[String; 3]) -> String {
    let [major, minor, patch] = numbers;
    format!(
        r#"1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK 0x3f
FILEFLAGS 0x0
FILEOS 0x40004
FILETYPE 0x1
FILESUBTYPE 0x0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "{bin}"
      VALUE "OriginalFilename", "{bin}.exe"
      VALUE "ProductName", "OpenS2X"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    )
}
