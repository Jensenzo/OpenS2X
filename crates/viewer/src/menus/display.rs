//! SET-5.2: the Graphics page's `RESOLUTION` and `MODE`, and the game
//! window that follows them.
//!
//! The original lists the display device's modes and switches the device
//! between full screen and a window. Ours never changes the display's mode:
//! full screen is a borderless window of the monitor's size, windowed is a
//! window of a size from the device's list that fits the monitor (a
//! stand-in, Q-1097; `docs/stand-ins.md`). Both choices take effect at once
//! (ours: the spec does not say when the original applies them, Q-1097 a).

use bevy::prelude::*;
use bevy::window::{MonitorSelection, OnMonitor, PrimaryMonitor, PrimaryWindow, WindowMode};

use super::menu::MenuState;

/// SET-5.2: the list holds at most this many device modes, one more with
/// the window's own size added.
pub(crate) const MAX_MODES: usize = 24;

/// The smallest window ours opens (the size kept before a profile names
/// one, Q-188 e).
pub(crate) const MIN_SIZE: (u32, u32) = (800, 600);

/// What the Graphics page shows of the display: the window's size in
/// pixels, the monitor it is on and that monitor's modes, read every frame
/// from the window ([`watch`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Display {
    /// The window's size in pixels.
    pub window: (u32, u32),
    /// The size of the monitor the window is on; `None` before it is known.
    pub monitor: Option<(u32, u32)>,
    /// That monitor's modes as the device lists them (sizes only; one
    /// entry per refresh rate and depth).
    pub modes: Vec<(u32, u32)>,
}

/// SET-5.2: the `RESOLUTION` list: full screen, the monitor's size alone
/// (ours never changes the display's mode); windowed, the device's modes
/// as `<w> x <h>`, each size once in the device's order, those that fit the
/// monitor, at most [`MAX_MODES`] (ours: the largest when there are more,
/// Q-1097 b), with the window's own size added when it is not one of them.
pub(crate) fn resolutions(display: &Display, fullscreen: bool) -> Vec<(u32, u32)> {
    if fullscreen {
        return vec![display.monitor.unwrap_or(display.window)];
    }
    let fits = |&(w, h): &(u32, u32)| display.monitor.is_none_or(|(mw, mh)| w <= mw && h <= mh);
    let mut sizes: Vec<(u32, u32)> = Vec::new();
    for m in display.modes.iter().filter(|m| fits(m)) {
        if !sizes.contains(m) {
            sizes.push(*m);
        }
    }
    if sizes.len() > MAX_MODES {
        let mut by_area = sizes.clone();
        by_area.sort_by_key(|&(w, h)| std::cmp::Reverse(u64::from(w) * u64::from(h)));
        let keep = &by_area[..MAX_MODES];
        sizes.retain(|m| keep.contains(m));
    }
    if !sizes.contains(&display.window) {
        sizes.push(display.window);
    }
    sizes
}

/// The text a size shows in the list.
pub(crate) fn mode_text((w, h): (u32, u32)) -> String {
    format!("{w} x {h}")
}

/// The windowed size to open with or go back to: the profile's (SET-4
/// fields 5 and 6; at least [`MIN_SIZE`], as before the page existed) when
/// it fits the monitor; else (ours, Q-1097 c: a monitor that changed) the
/// largest of the device's modes that fits, or the monitor's own size.
pub(crate) fn windowed_size(saved: (u32, u32), display: &Display) -> (u32, u32) {
    let unset = saved.0 == 0 || saved.1 == 0 || saved.0 == u32::MAX || saved.1 == u32::MAX;
    let want = if unset {
        MIN_SIZE
    } else {
        (saved.0.max(MIN_SIZE.0), saved.1.max(MIN_SIZE.1))
    };
    let Some((mw, mh)) = display.monitor else {
        return want;
    };
    if want.0 <= mw && want.1 <= mh {
        return want;
    }
    display
        .modes
        .iter()
        .copied()
        .filter(|&(w, h)| w <= mw && h <= mh)
        .max_by_key(|&(w, h)| u64::from(w) * u64::from(h))
        .unwrap_or((mw, mh))
}

/// The window mode a choice gives: full screen is a borderless window over
/// the monitor the window is on.
pub(crate) fn window_mode(fullscreen: bool) -> WindowMode {
    if fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    }
}

/// A fixed size given on the command line (`--size`): the window keeps it
/// and is never full screen (tools and the harness).
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct FixedSize;

/// Read the window's size and its monitor's modes into the menu state.
pub(crate) fn watch(
    mut menu: ResMut<MenuState>,
    windows: Query<(&Window, Option<&OnMonitor>), With<PrimaryWindow>>,
    monitors: Query<&bevy::window::Monitor>,
    primary: Query<&bevy::window::Monitor, With<PrimaryMonitor>>,
) {
    let Ok((window, on)) = windows.single() else {
        return;
    };
    let monitor = on
        .and_then(|m| monitors.get(m.0).ok())
        .or_else(|| primary.iter().next());
    let display = Display {
        window: (window.physical_width(), window.physical_height()),
        monitor: monitor.map(|m| (m.physical_width, m.physical_height)),
        modes: monitor
            .map(|m| {
                m.video_modes
                    .iter()
                    .map(|v| (v.physical_size.x, v.physical_size.y))
                    .collect()
            })
            .unwrap_or_default(),
    };
    if menu.display != display {
        menu.display = display;
    }
}

/// What the window was last set to, so the window is changed only when the
/// player's choice (or the monitor) changes it.
#[derive(Default)]
pub(crate) struct Applied {
    want: Option<(bool, (u32, u32), u32)>,
}

/// SET-5.2 (ours: at once): the window follows the profile's choice: full
/// screen or windowed, and in a window the chosen size (made to fit the
/// monitor, which is not written to the profile).
pub(crate) fn apply(
    menu: Res<MenuState>,
    fixed: Option<Res<FixedSize>>,
    mut applied: Local<Applied>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if fixed.is_some() || menu.display.monitor.is_none() {
        return;
    }
    let fullscreen = menu.settings.fullscreen;
    let size = windowed_size(menu.settings.resolution(), &menu.display);
    let want = (fullscreen, size, menu.display_choices);
    if applied.want == Some(want) {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let mode = window_mode(fullscreen);
    if window.mode != mode {
        window.mode = mode;
    }
    let first = applied.want.is_none();
    // The first time, a window already of the profile's size (as opened)
    // keeps the size it has in its own units.
    if !fullscreen && !(first && menu.display.window == size) {
        window.resolution.set_physical_resolution(size.0, size.1);
    }
    applied.want = Some(want);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(window: (u32, u32), monitor: (u32, u32), modes: &[(u32, u32)]) -> Display {
        Display {
            window,
            monitor: Some(monitor),
            modes: modes.to_vec(),
        }
    }

    /// SET-5.2: windowed, the device's modes in its order, each size once
    /// (one entry per refresh rate in the device's list), those that fit
    /// the monitor; the window's own size added when it is not one.
    // spec: SET-5.2
    #[test]
    fn the_list_is_the_devices_modes_that_fit() {
        let d = display(
            (1000, 700),
            (1920, 1080),
            &[
                (800, 600),
                (800, 600),
                (1024, 768),
                (1280, 720),
                (1920, 1080),
                (1920, 1080),
                (2560, 1440),
            ],
        );
        assert_eq!(
            resolutions(&d, false),
            vec![
                (800, 600),
                (1024, 768),
                (1280, 720),
                (1920, 1080),
                (1000, 700)
            ]
        );
        // The window's size is one of them: not added again.
        let d = Display {
            window: (1280, 720),
            ..d
        };
        assert_eq!(resolutions(&d, false).len(), 4);
        assert_eq!(mode_text((1280, 720)), "1280 x 720");
    }

    /// SET-5.2: at most 24 modes, 25 with the window's size added; ours
    /// keeps the largest 24 in the device's order (Q-1097 b).
    // spec: SET-5.2
    #[test]
    fn at_most_twenty_four_modes() {
        let modes: Vec<(u32, u32)> = (0..30).map(|i| (640 + 32 * i, 480 + 18 * i)).collect();
        let d = display((801, 601), (4000, 3000), &modes);
        let list = resolutions(&d, false);
        assert_eq!(list.len(), MAX_MODES + 1);
        assert_eq!(list[0], modes[6]);
        assert_eq!(list[MAX_MODES - 1], modes[29]);
        assert_eq!(list[MAX_MODES], (801, 601));
    }

    /// SET-5.2, ours: full screen is the monitor's size, the one entry
    /// (the display's mode is never changed).
    // spec: SET-5.2
    #[test]
    fn full_screen_lists_the_monitor_only() {
        let d = display((2560, 1440), (2560, 1440), &[(1024, 768), (2560, 1440)]);
        assert_eq!(resolutions(&d, true), vec![(2560, 1440)]);
        assert_eq!(
            window_mode(true),
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        );
        assert_eq!(window_mode(false), WindowMode::Windowed);
    }

    /// SET-4 fields 5, 6 give the windowed size; unset (-1, Q-188 e) or too
    /// small, at least 800 x 600; ours, a size that no longer fits the
    /// monitor (Q-1097 c): the largest mode that fits.
    // spec: SET-4, SET-5.2
    #[test]
    fn the_windowed_size_fits_the_monitor() {
        let modes = [(800, 600), (1280, 720), (1920, 1080), (2560, 1440)];
        let big = display((1, 1), (2560, 1440), &modes);
        assert_eq!(windowed_size((1920, 1080), &big), (1920, 1080));
        assert_eq!(windowed_size((u32::MAX, u32::MAX), &big), (800, 600));
        assert_eq!(windowed_size((640, 480), &big), (800, 600));
        let small = display((1, 1), (1366, 768), &modes);
        assert_eq!(windowed_size((1920, 1080), &small), (1280, 720));
        let unknown = Display::default();
        assert_eq!(windowed_size((1920, 1080), &unknown), (1920, 1080));
        let no_modes = display((1, 1), (1366, 768), &[]);
        assert_eq!(windowed_size((1920, 1080), &no_modes), (1366, 768));
    }
}
