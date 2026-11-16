//! `s2-app profile-seed FILE`: the profile file the golden pictures start
//! every scene with (`scripts/golden-images.sh`, docs/test-coverage.md "The
//! player's own files"): the first start's profile (SET-3, the original's
//! defaults as `s2_assets::cfg::Settings::default()` has them) with the Very
//! high graphic quality preset applied (SET-4a: every engine slider at its
//! maximum, shader detail high), as a player who chose it has. Written by
//! our own code, never copied from a player's folder, so every run on every
//! machine starts each scene from the same file.

use std::process::ExitCode;

/// The seed's bytes.
pub fn seed() -> Vec<u8> {
    let mut settings = s2_assets::cfg::Settings::default();
    settings
        .scene
        .apply_preset(s2_assets::cfg::PRESET_VERY_HIGH);
    s2_assets::cfg::write(&settings)
}

pub fn run(args: &[String]) -> ExitCode {
    let [file] = args else {
        eprintln!("usage: s2-app profile-seed FILE");
        return ExitCode::from(2);
    };
    match std::fs::write(file, seed()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{file}: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: SET-4a (the Very high preset: every slider at 1, shader detail 3), SET-3
    #[test]
    fn the_seed_is_the_first_start_at_very_high() {
        let parsed = s2_assets::cfg::parse(&seed()).expect("the seed parses");
        assert_eq!(parsed.scene.preset(), s2_assets::cfg::PRESET_VERY_HIGH);
        assert_eq!(parsed.scene.shader_detail(), 3);
        assert_eq!(parsed.scene.mirror_quality(), 1.0);
        assert_eq!(parsed.scene.mirror_details(), 1.0);
        assert_eq!(parsed.scene.shadow_quality(), 1.0);
        assert!(parsed.scene.shadows() && parsed.scene.mirror());
        let first = s2_assets::cfg::Settings {
            scene: parsed.scene.clone(),
            ..Default::default()
        };
        assert_eq!(
            parsed, first,
            "only the engine page differs from a first start"
        );
        assert_eq!(seed(), seed());
    }
}
