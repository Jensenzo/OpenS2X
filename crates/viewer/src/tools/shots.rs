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

/// `--shot-frames`: the game time of each frame of a sequence (60 frames a
/// second).
pub(crate) const SEQUENCE_FRAME: f32 = 1.0 / 60.0;

/// `--shot-frames`: the file of frame `k` of a shot of `frames` frames:
/// the shot's own for one, `<stem>-NN.<ext>` for a sequence.
pub(crate) fn sequence_path(path: &Path, k: u32, frames: u32) -> PathBuf {
    if frames <= 1 {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map_or_else(Default::default, |s| s.to_string_lossy());
    let ext = path
        .extension()
        .map_or_else(|| "png".into(), |e| e.to_string_lossy());
    path.with_file_name(format!("{stem}-{k:02}.{ext}"))
}

/// `--compare`: frames drawn after the scene is built before the shot.
pub(crate) const COMPARE_FRAMES: u32 = 1;

/// `--compare` (play test 48): the captures of one flat colour retaken
/// before one is saved as it is. A view with nothing but one colour (the
/// sky or the ground beyond a map's edge, no interface) is a picture too;
/// without the limit its shot never came and the viewer never quit.
pub(crate) const COMPARE_TRIES: u32 = 120;

/// `--compare`: whether a capture is saved: one that has drawn, or the last
/// try allowed.
pub(crate) fn keep_capture(drawn: bool, tries: u32) -> bool {
    drawn || tries >= COMPARE_TRIES
}

/// `--compare`: where the shot stands.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) enum CompareShot {
    #[default]
    Waiting,
    Asked,
    Saved,
}

/// `--still`: no pointer over the window for the interface and the map
/// (no hover, no tooltip), whatever the desktop's pointer does, so that two
/// runs draw the same pixels. Nothing is warped: a position of none moves
/// no pointer.
pub(crate) fn forget_pointer(
    mut windows: Query<&mut bevy::window::Window, With<bevy::window::PrimaryWindow>>,
) {
    for mut window in &mut windows {
        if window.physical_cursor_position().is_some() {
            window.set_physical_cursor_position(None);
        }
    }
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
    mut tries: Local<u32>,
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
                *tries += 1;
                let try_number = *tries;
                let state = compare.clone();
                commands.spawn(Screenshot::primary_window()).observe(
                    move |shot: On<ScreenshotCaptured>| {
                        let mut state = state.lock().expect("compare state");
                        let has_drawn = drawn(&shot.image);
                        if keep_capture(has_drawn, try_number) {
                            if !has_drawn {
                                warn!(
                                    "the compare shot is one colour after {try_number} captures; saved as it is"
                                );
                            }
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
    let frames = viewer.shot_frames.max(1);
    match viewer.shot_taken_at {
        None if due => {
            take_screenshot(&mut commands, sequence_path(&path, 0, frames));
            viewer.shot_taken_at = Some(viewer.frames_since_load);
        }
        // `--shot-frames`: the next frames of the sequence.
        Some(taken) if viewer.frames_since_load < taken + frames => {
            let k = viewer.frames_since_load - taken;
            take_screenshot(&mut commands, sequence_path(&path, k, frames));
        }
        Some(taken) if viewer.frames_since_load >= taken + frames + SHOT_SETTLE_FRAMES => {
            exit.write(if viewer.error.is_some() {
                AppExit::error()
            } else {
                AppExit::Success
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    // spec: none (play test 48: a --compare shot of a one-colour view, the map's edge, is saved after the tries)
    #[test]
    fn a_flat_compare_shot_is_kept_after_the_tries() {
        assert!(keep_capture(true, 1));
        assert!(!keep_capture(false, 1));
        assert!(!keep_capture(false, COMPARE_TRIES - 1));
        assert!(keep_capture(false, COMPARE_TRIES));
    }

    // spec: none (the picture tools' --shot-frames: a sequence's files)
    #[test]
    fn a_sequence_names_its_frames() {
        let p = Path::new("out/walk.png");
        assert_eq!(sequence_path(p, 0, 1), PathBuf::from("out/walk.png"));
        assert_eq!(sequence_path(p, 0, 16), PathBuf::from("out/walk-00.png"));
        assert_eq!(sequence_path(p, 15, 16), PathBuf::from("out/walk-15.png"));
        let o = crate::app::options::parse_from(
            false,
            ["--shot", "a.png", "--shot-frames", "16"].map(String::from),
        )
        .expect("parses");
        assert_eq!(o.shot_frames, 16);
        assert!(
            crate::app::options::parse_from(false, ["--shot-frames", "0"].map(String::from))
                .is_err()
        );
    }

    // spec: none (the golden pictures' --still: no hover depends on the desktop's pointer)
    #[test]
    fn a_still_window_forgets_the_pointer() {
        let mut world = World::new();
        let mut window = bevy::window::Window::default();
        window.set_physical_cursor_position(Some(bevy::math::DVec2::new(460.0, 340.0)));
        world.spawn((window, bevy::window::PrimaryWindow));
        world.run_system_once(forget_pointer).unwrap();
        let mut windows = world.query::<&bevy::window::Window>();
        let window = windows.single(&world).unwrap();
        assert_eq!(window.cursor_position(), None);
    }
}
