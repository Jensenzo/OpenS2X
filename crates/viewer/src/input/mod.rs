//! Input of the 2D view and the debug mode: the hovered field, camera keys,
//! the 2D / 3D switch, the keyboard editing of the world.

mod edit;
pub(crate) use edit::*;

use crate::game::Viewer;
use bevy::app::AppExit;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use s2_assets::s2m::FieldPos;

pub(crate) const PAN_SPEED: f32 = 900.0;
pub(crate) const ZOOM_STEP: f32 = 1.15;
/// A press and release closer than this many pixels is a click, not a drag.
pub(crate) const CLICK_SLOP: f32 = 5.0;
/// Buildings offered per page of the building list (keys 1-9).
pub(crate) const PAGE_SIZE: usize = 9;
/// F11 switches between the 2D and the 3D map view.
pub(crate) fn toggle_3d(
    keys: Res<ButtonInput<KeyCode>>,
    mut view: ResMut<crate::render::view3d::View3d>,
) {
    if keys.just_pressed(KeyCode::F11) {
        view.active = !view.active;
    }
}

/// The field under the mouse cursor, from whichever map view is on screen.
/// Both views place flags, roads and buildings from it.
#[derive(Resource, Default)]
pub(crate) struct Hover(pub(crate) Option<FieldPos>);

/// The field under the cursor in the 2D view.
pub(crate) fn hover_2d(
    viewer: Res<Viewer>,
    view: Res<crate::render::view3d::View3d>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut hover: ResMut<Hover>,
) {
    if view.active {
        return;
    }
    hover.0 = viewer
        .map
        .as_ref()
        .and_then(|map| hovered_field(&window, *camera, map.info.width, map.info.height));
}

/// The field under the mouse cursor.
pub(crate) fn hovered_field(
    window: &Window,
    camera: (&Camera, &GlobalTransform),
    width: u32,
    height: u32,
) -> Option<FieldPos> {
    let (camera, camera_transform) = camera;
    let cursor = window.cursor_position()?;
    let at = camera.viewport_to_world_2d(camera_transform, cursor).ok()?;
    crate::render::terrain::nearest_field(at.x, at.y, width, height)
}

/// WASD / arrows and mouse drag pan; the wheel zooms around the cursor.
#[allow(clippy::too_many_arguments)]
pub(crate) fn camera_controls(
    viewer: Res<Viewer>,
    view: Res<crate::render::view3d::View3d>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    window: Single<&Window>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if view.active {
        return;
    }
    let (mut transform, mut projection) = camera.into_inner();
    let Projection::Orthographic(orthographic) = &mut *projection else {
        return;
    };

    let pressed = |codes: [KeyCode; 2]| codes.iter().any(|&code| keys.pressed(code));
    let mut direction = Vec2::ZERO;
    if pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        direction.y += 1.0;
    }
    if pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        direction.y -= 1.0;
    }
    if pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        direction.x += 1.0;
    }
    let key_a = if viewer.attack_hover {
        KeyCode::ArrowLeft
    } else {
        KeyCode::KeyA
    };
    if pressed([key_a, KeyCode::ArrowLeft]) {
        direction.x -= 1.0;
    }
    let step = direction * PAN_SPEED * orthographic.scale * time.delta_secs();
    transform.translation += step.extend(0.0);

    // Dragging moves the map with the cursor; screen y points down.
    if buttons.any_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]) {
        transform.translation.x -= motion.delta.x * orthographic.scale;
        transform.translation.y += motion.delta.y * orthographic.scale;
    }

    if scroll.delta.y != 0.0 {
        let old_scale = orthographic.scale;
        let new_scale = (old_scale * ZOOM_STEP.powf(-scroll.delta.y.signum())).clamp(0.01, 20.0);
        // Keep the point under the cursor where it is.
        if let Some(cursor) = window.cursor_position() {
            let from_centre = Vec2::new(
                cursor.x - window.width() / 2.0,
                window.height() / 2.0 - cursor.y,
            );
            transform.translation += (from_centre * (old_scale - new_scale)).extend(0.0);
        }
        orthographic.scale = new_scale;
    }
}

pub(crate) fn quit_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    viewer: Res<Viewer>,
    mut exit: MessageWriter<AppExit>,
) {
    // Without `--debug` Escape belongs to the interface (the game menu).
    if viewer.debug && keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
