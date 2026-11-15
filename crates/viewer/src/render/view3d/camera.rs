//! The 3D camera (GFX-VIEW-3) and picking the field under the pointer (GFX-VIEW-1.5).

use super::*;

/// The 3D camera's transform and projection.
pub(crate) type CameraParts = (&'static mut Transform, &'static mut Projection);

/// Put the 3D camera where the rig says (GFX-VIEW-3.3), with the
/// projection of GFX-VIEW-3.5.
#[allow(clippy::too_many_arguments)]
pub fn place_camera(
    viewer: Res<crate::game::Viewer>,
    mut view: ResMut<View3d>,
    mut follow: ResMut<crate::extend::CameraFollow>,
    camera: Single<CameraParts, (With<MapCamera3d>, Without<Sky3d>)>,
    window: Single<&Window>,
    mut sky: Query<&mut Transform, (With<Sky3d>, Without<MapCamera3d>)>,
    mut sprites: Query<SkySpriteParts, (Without<Sky3d>, Without<MapCamera3d>)>,
    daylight: Res<super::Daylight>,
    movers: Res<Movers>,
    placed: Query<&GlobalTransform>,
) {
    // A project's follow: `cleared` lasts one frame.
    if follow.cleared {
        follow.cleared = false;
    }
    if !view.active {
        return;
    }
    let (mut camera, mut projection) = camera.into_inner();
    // GFX-VIEW-3.5: the 50 degrees are horizontal; wider screens see more
    // to the sides.
    if let Projection::Perspective(p) = projection.as_mut() {
        let aspect = window.width() / window.height().max(1.0);
        let fov = world3d::vertical_fov(aspect);
        if (p.fov - fov).abs() > 1e-6 {
            p.fov = fov;
        }
    }
    let Some(world) = &viewer.world else { return };
    // Only touched while set, so its change detection means something.
    if follow.target.is_some() {
        follow_target(&mut view, &mut follow);
    } else {
        view.placed = None;
    }
    // GFX-VIEW-3.7: the follow camera (O, mode 3) while he lives; the zoom
    // plays no part.
    let followed = view.follow.and_then(|r| {
        let s = world.settler_view(r)?;
        Some((s, follow_matrix(&movers, &placed, r)))
    });
    let mut eye = None;
    match followed {
        Some((s, Some((p, f, side)))) => {
            // The building pose: he stands where a building is.
            let door = world.building_at(s.walk.at).and_then(|_| {
                let d = world.terrain().neighbour(s.walk.at, 1)?;
                let [x, z] = view.frame.field_xz(d.0, d.1);
                Some([x, world3d::ground_height(world.terrain(), x, z), z])
            });
            let (wanted, look) = world3d::follow_target(view.follow_pose, p, f, side, door);
            let real = view
                .follow_eye
                .or_else(|| view.rig.last_eye())
                .unwrap_or(world3d::Eye {
                    position: wanted,
                    direction: world3d::normalize([0, 1, 2].map(|i| look[i] - wanted[i])),
                });
            let next = world3d::follow_frame(real, wanted, look, door.is_some(), world.terrain());
            view.follow_eye = Some(next);
            // The normal camera's target goes along, so it resumes there.
            view.rig.target[0] = p[0];
            view.rig.target[2] = p[2];
            eye = Some(next);
        }
        // Not drawn yet: the normal camera this frame.
        Some((_, None)) => {}
        None if view.follow.is_some() => view.end_follow(),
        None => {}
    }
    if let Some(at) = watched(&viewer, world, &view.frame) {
        // The worker a little above the screen's centre.
        let (forward, _) = view.rig.axes();
        // A felled tree (`--until-felled`) above the centre: it may lie
        // toward the camera.
        let back = if viewer.until_felled.is_some() {
            20.0
        } else {
            4.0 + 3.0 * f32::from(view.rig.zoom)
        };
        view.rig.target[0] = at[0] - forward[0] * back;
        view.rig.target[2] = at[2] - forward[1] * back;
    }
    let mut eye = match eye {
        Some(eye) => eye,
        None => view.rig.update(world.terrain()),
    };
    if follow.target.is_some() {
        view.placed = Some(rig_state(&view.rig));
    }
    // `--camera`: the original's camera, as it was read (TRACE-8).
    if let Some((position, direction)) = view.fixed_eye {
        eye.position = position;
        eye.direction = direction;
    }
    let [ex, ey, ez] = eye.position;
    let above = ey - world3d::ground_height(world.terrain(), ex, ez);
    view.ear = Some(crate::audio::sound::listener(
        eye.position,
        eye.direction,
        above,
    ));
    let position = Vec3::from(world3d::to_bevy(eye.position));
    let direction = Vec3::from(world3d::to_bevy(eye.direction));
    *camera = Transform::from_translation(position).looking_to(direction, Vec3::Y);
    // GFX-VIEW-6.3: the cap is centred on the camera, 15 units above it.
    for mut t in &mut sky {
        t.translation = position + Vec3::Y * terrain3d::CAP_LIFT;
    }
    // GFX-VIEW-6.3: the sun and moon along the light's way from the
    // camera (the moon mirrored), facing it.
    for (mut t, sprite) in &mut sprites {
        let size = if sprite.moon {
            world3d::MOON_SIZE
        } else {
            world3d::SUN_SIZE
        };
        let light = world3d::sky_sprite_direction(&daylight.0, sprite.moon);
        let (centre, side) = world3d::sky_sprite(eye.position, light, size);
        let centre = Vec3::from(world3d::to_bevy(centre));
        *t = Transform::from_translation(centre)
            .looking_at(position, Vec3::Y)
            .with_scale(Vec3::splat(side));
    }
}

/// GFX-VIEW-3.7: where the followed settler's model stands and its world
/// matrix's forward and side axes, in the game frame.
fn follow_matrix(
    movers: &Movers,
    placed: &Query<&GlobalTransform>,
    r: s2_sim::view::SettlerRef,
) -> Option<([f32; 3], [f32; 3], [f32; 3])> {
    use crate::world::scene3d::MoverId;
    let id = match r {
        s2_sim::view::SettlerRef::Settler(i) => MoverId::Settler(i),
        s2_sim::view::SettlerRef::Carrier(i) => MoverId::Carrier(i),
        s2_sim::view::SettlerRef::Builder(at) => MoverId::Builder(at),
        s2_sim::view::SettlerRef::Leveller(at) => MoverId::Leveller(at),
    };
    let t = placed.get(movers.states.get(&id)?.entity).ok()?;
    let (_, rotation, translation) = t.to_scale_rotation_translation();
    let axis = |v: [f32; 3]| {
        let turned = rotation * Vec3::from(world3d::to_bevy(v));
        world3d::normalize(world3d::from_bevy(turned.to_array()))
    };
    Some((
        world3d::from_bevy(translation.to_array()),
        axis([0.0, 0.0, 1.0]),
        axis([1.0, 0.0, 0.0]),
    ))
}

/// The rig's target x and z, zoom and turn: what the player's camera
/// input changes.
pub(crate) fn rig_state(rig: &world3d::CameraRig) -> [f32; 4] {
    [
        rig.target[0],
        rig.target[2],
        f32::from(rig.zoom),
        rig.yaw_target,
    ]
}

/// A project's follow (`extend::CameraFollow`, `docs/extending.md`): the
/// rig's target goes to the point, the zoom and turn stay. When the player
/// moved the camera since the last frame (its target, zoom or turn is not
/// what the follow left), looks around (GFX-VIEW-3.2a) or follows a
/// settler (UI-GAME-12.1 O), the follow ends and says so. A point the
/// project sets while O already follows a settler (no point of its own was
/// followed in the last frame) ends O's follow instead (W10): the normal
/// camera goes on from O's eye (GFX-VIEW-3.7) and follows the point.
pub(crate) fn follow_target(view: &mut View3d, follow: &mut crate::extend::CameraFollow) {
    let Some(target) = follow.target else {
        view.placed = None;
        return;
    };
    if view.follow.is_some() && view.placed.is_none() && !view.rig.looking() {
        view.end_follow();
    }
    let moved = view.placed.is_some_and(|p| p != rig_state(&view.rig));
    if moved || view.rig.looking() || view.follow.is_some() {
        follow.target = None;
        follow.cleared = true;
        view.placed = None;
        return;
    }
    let [x, _, z] = world3d::from_bevy(target.to_array());
    view.rig.target[0] = x;
    view.rig.target[2] = z;
}

/// `--watch`: where the worker of the player's first building of the
/// watched type is (game frame), if there is one.
pub(crate) fn watched(
    viewer: &crate::game::Viewer,
    world: &s2_sim::World,
    frame: &Frame,
) -> Option<[f32; 3]> {
    // `--until-felled`: the player's felled tree.
    if viewer.until_felled.is_some() {
        let (_, d) = world.deposits().find(|(f, _)| {
            world.fall_of(*f).is_some() && world.field_owner(*f) == Some(crate::game::PLAYER)
        })?;
        let [x, z] = frame.sample_xz(i64::from(d.position.0), i64::from(d.position.1));
        return Some([
            x,
            crate::render::world3d::ground_height(world.terrain(), x, z),
            z,
        ]);
    }
    let wanted = viewer.watch.as_ref()?;
    // `--watch Carrier`: the first road carrier with a ware (screenshots of
    // the carried ware, GFX-OBJ-3.4).
    if wanted.eq_ignore_ascii_case("carrier") {
        let carrier = world.settler_views().into_iter().find(|v| {
            matches!(
                v.activity,
                s2_sim::view::Activity::Carrier { retired: false, .. }
            ) && v.carrying.is_some()
                && v.player == crate::game::PLAYER
        })?;
        return Some(crate::world::scene3d::walker_point(world, frame, &carrier.walk, None).0);
    }
    let building = world.buildings().find(|b| {
        b.owner == crate::game::PLAYER
            && viewer
                .building_names
                .get(&b.building_type)
                .is_some_and(|n| n.eq_ignore_ascii_case(wanted))
    })?;
    let settler = world.settler_views().into_iter().find(|v| {
        matches!(v.activity, s2_sim::view::Activity::Worker { building: b, .. } if b == building.at)
    })?;
    let at = match &settler.fine {
        Some(fine) => crate::world::scene3d::fine_point(world, frame, fine).0,
        None => crate::world::scene3d::walker_point(world, frame, &settler.walk, None).0,
    };
    Some(at)
}

/// GFX-VIEW-3.2 (Q-172 closed): the options' camera speed s is the
/// profile's scroll speed (`SET-settings.md` SET-2 field 7; 0.25 without a
/// file).
pub(crate) fn camera_speed(menu: &crate::menus::menu::MenuState) -> f32 {
    menu.settings.cfg.user.scroll_speed
}

/// GFX-VIEW-3.2: arrows and WASD move the target, the middle button drags
/// it, the wheel zooms by one step, Page Up and Page Down rotate (key
/// bindings are Q-086; A pans only when not over an enemy building, as in
/// the 2D view). GFX-VIEW-3.2a: the right button held is the look mode.
#[allow(clippy::too_many_arguments)]
pub fn camera_input(
    viewer: Res<crate::game::Viewer>,
    menu: Res<crate::menus::menu::MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    time: Res<Time>,
    mut window: Single<&mut Window>,
    mut cursor: Single<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
    mut view: ResMut<View3d>,
) {
    // GFX-VIEW-3.6 e: while a scripted move runs no camera input is read.
    if !view.active || view.rig.moving() {
        return;
    }
    // GFX-VIEW-3.7: while following, only the pose keys: PAGE DOWN or the
    // wheel up goes ahead, PAGE UP or the wheel down behind, and from behind
    // ends the follow.
    if view.follow.is_some() && !viewer.debug {
        let ahead = keys.just_pressed(KeyCode::PageDown) || scroll.delta.y > 0.0;
        let back = keys.just_pressed(KeyCode::PageUp) || scroll.delta.y < 0.0;
        view.follow_keys(ahead, back);
        return;
    }
    let frame = view.frame;
    // GFX-VIEW-3.2a: the look lasts while the right button is held (not
    // while a settler is followed); on release the look is dropped and the
    // pointer shown again where it was kept.
    let look = buttons.pressed(MouseButton::Right) && view.follow.is_none();
    if !look && view.rig.looking() {
        view.rig.look_end();
        cursor.visible = true;
    }
    if look && view.rig.looking() {
        // SET-5.5: FREE CAMERA (field 21) inverts the free look (GFX-VIEW-3.2a
        // "presumably the Inverted setting").
        let inverted = menu.settings.cfg.user.unknown_bools[1];
        look_frame(
            &mut view.rig,
            &frame,
            &keys,
            &motion,
            &time,
            &mut window,
            (inverted, camera_speed(&menu)),
        );
        return;
    }
    // UI-GAME-12.1 (O): any camera input ends following a settler.
    let before = (view.rig.target, view.rig.zoom, view.rig.yaw_target);
    let rig = &mut view.rig;
    let pressed = |codes: [KeyCode; 2]| codes.iter().any(|&code| keys.pressed(code));
    let key_a = if viewer.attack_hover {
        KeyCode::ArrowLeft
    } else {
        KeyCode::KeyA
    };
    // UI-GAME-12.1: the game scrolls with the arrows alone (letters are
    // its dialog keys); the viewer's debug mode has WASD as well.
    let letters = viewer.debug;
    let mut forward = 0.0;
    let mut right = 0.0;
    if pressed([KeyCode::ArrowUp, KeyCode::ArrowUp])
        || letters && pressed([KeyCode::KeyW, KeyCode::KeyW])
    {
        forward += 1.0;
    }
    if pressed([KeyCode::ArrowDown, KeyCode::ArrowDown])
        || letters && pressed([KeyCode::KeyS, KeyCode::KeyS])
    {
        forward -= 1.0;
    }
    if pressed([KeyCode::ArrowRight, KeyCode::ArrowRight])
        || letters && pressed([KeyCode::KeyD, KeyCode::KeyD])
    {
        right += 1.0;
    }
    if pressed([KeyCode::ArrowLeft, KeyCode::ArrowLeft]) || letters && pressed([key_a, key_a]) {
        right -= 1.0;
    }
    if forward != 0.0 || right != 0.0 {
        let step = 2.0 * world3d::key_step(time.delta_secs(), camera_speed(&menu));
        rig.pan(&frame, right * step, forward * step);
    }
    // The drag is taken as a fraction of the window (Q-172); the ground
    // moves with the mouse, as when dragging the 2D map.
    if buttons.pressed(MouseButton::Middle) && motion.delta != Vec2::ZERO {
        let factor = world3d::drag_factor(camera_speed(&menu));
        let dx = motion.delta.x / window.width().max(1.0);
        let dy = motion.delta.y / window.height().max(1.0);
        rig.pan(&frame, -dx * factor, dy * factor);
    }
    if scroll.delta.y > 0.0 {
        rig.zoom_in();
    } else if scroll.delta.y < 0.0 {
        rig.zoom_out();
    }
    // UI-GAME-12.1: INSERT / DELETE turn the camera, PAGE DOWN / PAGE UP
    // zoom in / out; the debug mode keeps its Page Up / Page Down turning
    // (Insert and Delete are its ship keys). GFX-VIEW-3.2a: the frame the
    // look starts in skips the zoom keys.
    if letters {
        if keys.just_pressed(KeyCode::PageUp) {
            rig.rotate_left();
        }
        if keys.just_pressed(KeyCode::PageDown) {
            rig.rotate_right();
        }
    } else {
        if keys.just_pressed(KeyCode::Insert) {
            rig.rotate_left();
        }
        if keys.just_pressed(KeyCode::Delete) {
            rig.rotate_right();
        }
        if !look && keys.just_pressed(KeyCode::PageDown) {
            rig.zoom_in();
        }
        if !look && keys.just_pressed(KeyCode::PageUp) {
            rig.zoom_out();
        }
    }
    if (view.rig.target, view.rig.zoom, view.rig.yaw_target) != before {
        view.end_follow();
    }
    // GFX-VIEW-3.2a: the look starts after this frame's keys and middle
    // button; the pointer is hidden.
    if look {
        view.rig.look_start();
        cursor.visible = false;
    }
}

/// GFX-VIEW-3.2a: a frame of the look. In a frame of at most 0.25 s the
/// mouse turns the view (its movement as a fraction of the window, the
/// middle button's unit; SET-5.5 Inverted flips the vertical: the delta's
/// unit and sign are Q-344) and the arrows fly camera and target by twice
/// the unclamped key step. The edges, the middle button and the zoom keys
/// do nothing. The hidden pointer is kept within 1 % to 99 % of the window.
fn look_frame(
    rig: &mut world3d::CameraRig,
    frame: &Frame,
    keys: &ButtonInput<KeyCode>,
    motion: &bevy::input::mouse::AccumulatedMouseMotion,
    time: &Time,
    window: &mut Window,
    (inverted, speed): (bool, f32),
) {
    let (w, h) = (window.width().max(1.0), window.height().max(1.0));
    let dt = time.delta_secs();
    if dt <= 0.25 {
        let dy = motion.delta.y / h;
        rig.look_turn(motion.delta.x / w, if inverted { -dy } else { dy });
        let axis = |plus: KeyCode, minus: KeyCode| {
            f32::from(u8::from(keys.pressed(plus))) - f32::from(u8::from(keys.pressed(minus)))
        };
        let forward = axis(KeyCode::ArrowUp, KeyCode::ArrowDown);
        let right = axis(KeyCode::ArrowRight, KeyCode::ArrowLeft);
        if forward != 0.0 || right != 0.0 {
            let step = 2.0 * world3d::look_step(dt, speed);
            rig.look_fly(frame, right * step, forward * step);
        }
    }
    if let Some(p) = window.cursor_position() {
        let kept = Vec2::new(p.x.clamp(0.01 * w, 0.99 * w), p.y.clamp(0.01 * h, 0.99 * h));
        if kept != p {
            window.set_cursor_position(Some(kept));
        }
    }
}

/// The field under the cursor in the 3D view: the ray through the cursor
/// meets the ground (GFX-VIEW-1.4), and the nearest field of that point is
/// taken (as the 2D view takes it).
pub fn hover_3d(
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    window: Single<&Window>,
    camera: Single<(&Camera, &Transform), With<MapCamera3d>>,
    mut hover: ResMut<crate::input::Hover>,
) {
    if !view.active {
        return;
    }
    let (Some(world), Some(map)) = (&viewer.world, &viewer.map) else {
        hover.0 = None;
        return;
    };
    let (camera, transform) = *camera;
    let ray = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world(&GlobalTransform::from(*transform), cursor)
            .ok()
    });
    let _ = map;
    hover.0 = ray.and_then(|ray| {
        let origin = world3d::from_bevy(ray.origin.to_array());
        let direction = world3d::from_bevy(ray.direction.as_vec3().to_array());
        world3d::ray_field(world.terrain(), origin, direction)
            .map(|(x, y)| s2_assets::s2m::FieldPos { x, y })
    });
}

/// UI-GAME-9.3: what the pointer is over on the map this frame.
#[derive(Resource, Default)]
pub struct MapHover(pub Option<crate::ui::game::MapPick>);

/// Where a ray along `direction` from `origin` enters the box `centre` +-
/// `half` (Bevy frame), as the distance along the ray.
pub(crate) fn ray_box(origin: Vec3, direction: Vec3, centre: Vec3, half: Vec3) -> Option<f32> {
    let (mut near, mut far) = (0.0f32, f32::MAX);
    for i in 0..3 {
        let (o, d) = (origin[i] - centre[i], direction[i]);
        if d.abs() < 1e-6 {
            if o.abs() > half[i] {
                return None;
            }
            continue;
        }
        let (a, b) = ((-half[i] - o) / d, (half[i] - o) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
        if near > far {
            return None;
        }
    }
    Some(near)
}

/// UI-GAME-9.3: the first match under the pointer: an own settler (his
/// model: a box 0.8 wide and 2 high where he is drawn), else by the field
/// under the pointer an own flag, else a building of any owner (a box by
/// its size over its field point).
#[allow(clippy::too_many_arguments)]
pub fn pick_map(
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    movers: Res<Movers>,
    hover: Res<crate::input::Hover>,
    window: Single<&Window>,
    camera: Single<(&Camera, &Transform), With<MapCamera3d>>,
    transforms: Query<&Transform, Without<MapCamera3d>>,
    mut picked: ResMut<MapHover>,
) {
    use crate::ui::game::MapPick;
    use crate::world::scene3d::MoverId;
    picked.0 = None;
    let Some(world) = &viewer.world else { return };
    if !view.active {
        return;
    }
    let (camera, transform) = *camera;
    let Some(ray) = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world(&GlobalTransform::from(*transform), cursor)
            .ok()
    }) else {
        return;
    };
    let (origin, direction) = (ray.origin, ray.direction.as_vec3());
    let player = crate::game::PLAYER;
    let site_owner = |at: (u32, u32)| world.building_at(at).map(|b| b.owner);
    let mut best: Option<(f32, MapPick)> = None;
    for (id, state) in &movers.states {
        let (pick, owner) = match *id {
            MoverId::Settler(i) => (
                MapPick::Settler(i),
                world
                    .settlers()
                    .find(|(j, _)| *j == i)
                    .map(|(_, s)| s.player),
            ),
            MoverId::Carrier(i) => (
                MapPick::Carrier(i),
                world
                    .carriers()
                    .get(i)
                    .and_then(|c| world.road(c.road))
                    .map(|r| r.owner),
            ),
            MoverId::Builder(at) => (MapPick::Builder, site_owner(at)),
            MoverId::Leveller(at) => (MapPick::Leveller, site_owner(at)),
            MoverId::Animal(_) | MoverId::Ship(_) => continue,
        };
        if owner != Some(player) {
            continue;
        }
        let Ok(t) = transforms.get(state.entity) else {
            continue;
        };
        let centre = t.translation + Vec3::Y;
        if let Some(d) = ray_box(origin, direction, centre, Vec3::new(0.4, 1.0, 0.4)) {
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, pick));
            }
        }
    }
    if let Some((_, pick)) = best {
        picked.0 = Some(pick);
        return;
    }
    if let Some(field) = hover.0 {
        if world.flag_at((field.x, field.y)) == Some(player) {
            picked.0 = Some(MapPick::Flag);
            return;
        }
    }
    for b in world.buildings() {
        let (half, height) = match b.size {
            s2_sim::BuildingSize::Large | s2_sim::BuildingSize::Harbor => (4.0, 10.0),
            s2_sim::BuildingSize::Medium => (2.5, 6.0),
            _ => (2.0, 4.5),
        };
        let [x, z] = view.frame.field_xz(b.at.0, b.at.1);
        let y = world3d::ground_height(world.terrain(), x, z);
        let centre = Vec3::from(world3d::to_bevy([x, y + height / 2.0, z]));
        let half = Vec3::new(half, height / 2.0, half);
        if let Some(d) = ray_box(origin, direction, centre, half) {
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, MapPick::Building(b.at)));
            }
        }
    }
    picked.0 = best.map(|(_, pick)| pick);
}

/// UI-GAME-9.4: the ship whose model is under the pointer this frame (its
/// index in the world's list), of any owner.
#[derive(Resource, Default)]
pub struct ShipHover(pub Option<usize>);

/// UI-GAME-9.4 (4): the ray through the pointer is tested against each
/// ship's model: its bounding sphere first, then, as the finer test, the
/// model's box turned with the ship; the hit nearest along the ray wins.
/// A ship is never found from its field.
#[allow(clippy::too_many_arguments)]
pub fn pick_ship(
    viewer: Res<crate::game::Viewer>,
    view: Res<View3d>,
    movers: Res<Movers>,
    window: Single<&Window>,
    camera: Single<(&Camera, &Transform), With<MapCamera3d>>,
    transforms: Query<&GlobalTransform, Without<MapCamera3d>>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut picked: ResMut<ShipHover>,
) {
    use crate::world::scene3d::MoverId;
    picked.0 = None;
    let Some(world) = &viewer.world else { return };
    if !view.active || world.ships().is_empty() {
        return;
    }
    let (camera, transform) = *camera;
    let Some(ray) = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world(&GlobalTransform::from(*transform), cursor)
            .ok()
    }) else {
        return;
    };
    let (origin, direction) = (ray.origin, ray.direction.as_vec3());
    let mut best: Option<(f32, usize)> = None;
    for (i, ship) in world.ships().iter().enumerate() {
        let Some(state) = movers.states.get(&MoverId::Ship(i)) else {
            continue;
        };
        let Ok(t) = transforms.get(state.entity) else {
            continue;
        };
        let tribe =
            crate::render::models3d::TRIBE_NAMES[usize::from(world.tribe(ship.owner)).min(2)];
        let Some(entry) = library
            .tables
            .ships
            .iter()
            .find(|s| s.tribe == tribe)
            .cloned()
        else {
            continue;
        };
        let Some(model) = library.model(&entry.mesh, entry.scale, &mut meshes, &mut images) else {
            continue;
        };
        if let Some(d) = ray_model(origin, direction, t, model.bounds) {
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, i));
            }
        }
    }
    picked.0 = best.map(|(_, i)| i);
}

/// UI-GAME-9.4: where a ray meets a model placed by `place` with the box
/// `bounds` (game frame, the model's own): its bounding sphere, then the box
/// in the model's frame. The distance along the ray.
pub(crate) fn ray_model(
    origin: Vec3,
    direction: Vec3,
    place: &GlobalTransform,
    bounds: crate::world::decor::Box3,
) -> Option<f32> {
    let lo = Vec3::from(world3d::to_bevy(bounds[0]));
    let hi = Vec3::from(world3d::to_bevy(bounds[1]));
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let centre = (lo + hi) / 2.0;
    let half = (hi - lo) / 2.0;
    let scale = place.compute_transform().scale.max_element();
    // The sphere around the box.
    let c = place.transform_point(centre);
    let r = half.length() * scale;
    let to = c - origin;
    let along = to.dot(direction);
    if to.length_squared() - along * along > r * r {
        return None;
    }
    // The box, the ray in the model's frame.
    let inverse = place.affine().inverse();
    let o = inverse.transform_point3(origin);
    let d = inverse.transform_vector3(direction);
    let local = ray_box(o, d, centre, half)?;
    // The local distance back along the world ray.
    Some((place.transform_point(o + d * local) - origin).length())
}

#[cfg(test)]
mod pick_tests {
    use super::*;

    // spec: UI-GAME-9.4
    #[test]
    fn a_ray_meets_a_ship_by_its_sphere_then_its_turned_box() {
        // A box 6 long (game x), 2 wide, 3 high, standing on the origin.
        let bounds = [
            world3d::from_bevy([-3.0, 0.0, -1.0]),
            world3d::from_bevy([3.0, 3.0, 1.0]),
        ];
        let place = GlobalTransform::from(Transform::from_xyz(10.0, 0.0, 10.0));
        let down = Vec3::new(0.0, -1.0, 0.0);
        // Straight down onto its middle: 10 above its top.
        let d = ray_model(Vec3::new(10.0, 13.0, 10.0), down, &place, bounds).unwrap();
        assert!((d - 10.0).abs() < 1e-4, "{d}");
        // Over its bow, 2.5 along: a hit; turned a quarter, a miss.
        let bow = Vec3::new(12.5, 13.0, 10.0);
        assert!(ray_model(bow, down, &place, bounds).is_some());
        let turned = GlobalTransform::from(
            Transform::from_xyz(10.0, 0.0, 10.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        );
        assert!(ray_model(bow, down, &turned, bounds).is_none());
        // Off to the side: outside the sphere.
        assert!(ray_model(Vec3::new(20.0, 13.0, 10.0), down, &place, bounds).is_none());
    }
}
