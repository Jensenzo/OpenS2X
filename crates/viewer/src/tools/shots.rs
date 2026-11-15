//! Screenshots: F12, `--shot` and the still `--compare` shot.

use crate::game::{map_name, Viewer};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Frames to wait after a map is shown before `--shot` captures it, and after
/// the capture before quitting, so the image is on disk.
pub(crate) const SHOT_SETTLE_FRAMES: u32 = 30;
pub(crate) fn take_screenshot(commands: &mut Commands, path: PathBuf) {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        if let Err(e) = std::fs::create_dir_all(dir) {
            error!("cannot create {}: {e}", dir.display());
            return;
        }
    }
    info!("saving screenshot to {}", path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

/// F12 saves what is on screen as `<map>-<unix seconds>.png`.
pub(crate) fn screenshot_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    viewer: Res<Viewer>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let name = map_name(&viewer.maps[viewer.current]);
    let path = screenshot_path(&viewer.screenshot_dir, &name, seconds);
    take_screenshot(&mut commands, path);
}

/// The F12 screenshot of `map` at `seconds` in `dir`.
pub(crate) fn screenshot_path(dir: &Path, map: &str, seconds: u64) -> PathBuf {
    dir.join(format!("{map}-{seconds}.png"))
}

/// F12's folder: a project's (`Extensions::screenshot_dir`), else the
/// `--screenshot-dir` option's.
pub(crate) fn shot_folder(project: Option<PathBuf>, option: &Path) -> PathBuf {
    project.unwrap_or_else(|| option.to_path_buf())
}

/// `--compare`: frames drawn after the scene is built before the shot.
pub(crate) const COMPARE_FRAMES: u32 = 1;

/// `--compare`: where the shot stands.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) enum CompareShot {
    #[default]
    Waiting,
    Asked,
    Saved,
}

/// A captured frame that is not one flat colour (the window has drawn).
pub(crate) fn drawn(image: &Image) -> bool {
    let Some(data) = image.data.as_ref() else {
        return false;
    };
    let first = data.get(..4);
    data.chunks(4).any(|p| Some(p) != first)
}

/// `--shot`: capture the first map once it has been on screen for a moment,
/// then quit.
pub(crate) fn automatic_shot(
    mut commands: Commands,
    mut viewer: ResMut<Viewer>,
    view: Res<crate::render::view3d::View3d>,
    mut built_frames: Local<u32>,
    compare: Local<std::sync::Arc<std::sync::Mutex<CompareShot>>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = viewer.shot.clone() else {
        return;
    };
    // `--compare`: the first frame drawn after the 3D scene of the save is
    // built. The window presents nothing in its first frames, so a capture
    // that is one flat colour is taken again in the next frame.
    if viewer.compare && viewer.shot_taken_at.is_none() && view.is_built(viewer.loads) {
        *built_frames += 1;
        let mut state = compare.lock().expect("compare state");
        match *state {
            CompareShot::Saved => viewer.shot_taken_at = Some(viewer.frames_since_load),
            CompareShot::Waiting if *built_frames > COMPARE_FRAMES => {
                *state = CompareShot::Asked;
                let state = compare.clone();
                commands.spawn(Screenshot::primary_window()).observe(
                    move |shot: On<ScreenshotCaptured>| {
                        let mut state = state.lock().expect("compare state");
                        if drawn(&shot.image) {
                            info!("saving the compare shot to {}", path.display());
                            save_to_disk(path.clone())(shot);
                            *state = CompareShot::Saved;
                        } else {
                            *state = CompareShot::Waiting;
                        }
                    },
                );
            }
            _ => {}
        }
        return;
    }
    let due = !viewer.compare && viewer.frames_since_load >= viewer.shot_after;
    match viewer.shot_taken_at {
        None if due => {
            take_screenshot(&mut commands, path);
            viewer.shot_taken_at = Some(viewer.frames_since_load);
        }
        Some(taken) if viewer.frames_since_load >= taken + SHOT_SETTLE_FRAMES => {
            exit.write(if viewer.error.is_some() {
                AppExit::error()
            } else {
                AppExit::Success
            });
        }
        _ => {}
    }
}
