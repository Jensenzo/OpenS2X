//! `--colour-probe <report>`: checks the colour pipeline on the real GPU
//! (`docs/colour.md`). Flat quads of known texels are drawn with the
//! object shader under the map's light, the frame is captured, and each
//! quad's centre pixel must be `min(255, v x D)` (GFX-VIEW-2.3: the light
//! works on the texture values as stored, no gamma, the sum not scaled).
//! Two more rows lay a see-through road and decal texel over lit grass:
//! the pixel must be the straight blend of the stored values,
//! round(255 x (rgb x a + d x (1 - a))) (GFX-VIEW-2.6 c1, T25, T26).
//! Two quads lay a decal texel over a road texel, the decal farther from
//! the camera: the decal must still be blended on top (GFX-VIEW-2.7, T27).
//! Two lit quads get a piece of the interface over them, the paused view's
//! black layer and a colour at half alpha: the interface too must blend
//! straight on the stored values (GFX-VIEW-2.6 c, UI-GAME-1.11).
//! The report gets one line per quad and the swapchain's format; the exit
//! code is 1 when a pixel is off by more than one step.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

use crate::render::models3d::{Family, ModelLibrary, ObjectMaterial, ObjectParams};
use crate::render::view3d::{MapCamera3d, MAP_LAYER};
use crate::render::world3d;

/// The texels drawn: the greys of GFX-VIEW T20, T21 (40, 128, 220), two
/// more, and two colours.
pub const TEXELS: [[u8; 3]; 7] = [
    [40, 40, 40],
    [64, 64, 64],
    [128, 128, 128],
    [192, 192, 192],
    [220, 220, 220],
    [200, 120, 60],
    [90, 140, 200],
];

/// Frames the game is on screen before the quads go up, and after.
const FRAMES_BEFORE: u32 = 60;
const FRAMES_AFTER: u32 = 10;

/// The rows: lit flat ground (N . L = sin 50 deg), flat ground in the
/// shadow of an occluder, a quad turned edge-on to the light (ambient
/// only), a back-wound decal, and the two blended rows; the decals over
/// roads share the road row's free columns.
const ROWS: [Row; 7] = [
    Row::Flat,
    Row::Shadowed,
    Row::Ambient,
    Row::Decal,
    Row::RoadOver,
    Row::DecalOverRoad,
    Row::DecalOver,
];

/// GFX-VIEW T25, T26: the grass texel under the blended rows, lit flat:
/// the stored ground pixel d = (105, 131, 48).
pub const GRASS: [u8; 3] = [80, 110, 40];
/// T25: the road's centre texel at the strip's centre, near its edge and
/// outside it (alpha 153, 51, 0).
pub const ROAD_OVER: [[u8; 4]; 3] = [[86, 71, 49, 153], [86, 71, 49, 51], [86, 71, 49, 0]];
/// T26: the woodcutter decal's front border from z = -2.50 to -3.35.
pub const DECAL_OVER: [[u8; 4]; 6] = [
    [88, 77, 45, 255],
    [105, 91, 47, 204],
    [78, 69, 32, 136],
    [97, 86, 44, 85],
    [94, 84, 47, 34],
    [114, 103, 60, 0],
];
/// GFX-VIEW-2.7, T27: road texels with the decal texel laid over each.
pub const DECAL_OVER_ROAD: [([u8; 4], [u8; 4]); 2] = [
    ([150, 40, 40, 128], [40, 40, 150, 128]),
    ([150, 40, 40, 255], [40, 40, 150, 128]),
];

/// GFX-VIEW-2.6 c, UI-GAME-1.11: interface fills over quads of the lit
/// row, by column: the paused view's layer (black, alpha byte 165) and a
/// colour at alpha 128.
pub const INTERFACE_OVER: [(usize, [u8; 4]); 2] = [(2, [0, 0, 0, 165]), (6, [200, 120, 60, 128])];

/// The layer of the occluders: the light sees it, the camera does not, so
/// they cast their shadow without hiding the quads.
const OCCLUDER_LAYER: usize = 30;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Row {
    Flat,
    Shadowed,
    Ambient,
    /// GFX-OBJ-2.3a d: a ground decal wound the other way round is lit by
    /// its mesh's own up normal (play test 10, F19).
    Decal,
    /// GFX-VIEW-2.6 c1, T25: a road texel (normal map straight up) blended
    /// over lit grass.
    RoadOver,
    /// T26: a decal border texel blended over lit grass.
    DecalOver,
    /// GFX-VIEW-2.7, T27: a decal texel over a road texel over lit grass,
    /// the decal lying between the two (farther from the camera than the
    /// road), so only the fixed layer order puts it on top.
    DecalOverRoad,
}

impl Row {
    /// The texels of a row: opaque for the first four, with their alpha for
    /// the blended ones.
    fn texels(self) -> Vec<[u8; 4]> {
        match self {
            Row::RoadOver => ROAD_OVER.to_vec(),
            Row::DecalOver => DECAL_OVER.to_vec(),
            Row::DecalOverRoad => DECAL_OVER_ROAD.iter().map(|p| p.1).collect(),
            _ => TEXELS.iter().map(|t| [t[0], t[1], t[2], 255]).collect(),
        }
    }

    fn blended(self) -> bool {
        matches!(self, Row::RoadOver | Row::DecalOver | Row::DecalOverRoad)
    }

    /// The line on screen and the first column of a row.
    fn place(self, index: usize) -> (usize, usize) {
        match self {
            Row::DecalOverRoad => (index - 1, ROAD_OVER.len() + 1),
            Row::DecalOver => (index - 1, 0),
            _ => (index, 0),
        }
    }
}

/// The expected pixel of a texel under the light D: min(255, v x D),
/// rounded (GFX-VIEW-2.3, 2.4).
pub fn expected(texel: [u8; 3], d: [f32; 3]) -> [u8; 3] {
    std::array::from_fn(|c| (f32::from(texel[c]) * d[c]).round().min(255.0) as u8)
}

/// GFX-VIEW-2.6 c1: a texel of alpha a under the light D blended over the
/// stored pixel d: round(255 x (rgb x a + d / 255 x (1 - a))), rgb =
/// min(1, D x texel / 255).
pub fn blended(texel: [u8; 4], d_light: [f32; 3], under: [u8; 3]) -> [u8; 3] {
    let a = f32::from(texel[3]) / 255.0;
    std::array::from_fn(|c| {
        let rgb = (f32::from(texel[c]) * d_light[c] / 255.0).min(1.0);
        (255.0 * (rgb * a + f32::from(under[c]) / 255.0 * (1.0 - a))).round() as u8
    })
}

/// GFX-VIEW-2.6 c: an interface colour of alpha a over the stored pixel
/// d, blended straight: round(c x a + d x (1 - a)).
pub fn interface_over(colour: [u8; 4], under: [u8; 3]) -> [u8; 3] {
    let a = f32::from(colour[3]) / 255.0;
    std::array::from_fn(|c| {
        (f32::from(colour[c]) * a + f32::from(under[c]) * (1.0 - a)).round() as u8
    })
}

/// One quad's texel, light and where its centre is on screen.
struct Quad {
    texel: [u8; 4],
    /// The road texel under it (T27).
    under: Option<[u8; 4]>,
    /// The interface colour over it (GFX-VIEW-2.6 c).
    interface: Option<[u8; 4]>,
    row: Row,
    centre: Vec3,
}

impl Quad {
    fn expected(&self, light: &world3d::DayLight) -> [u8; 3] {
        let d = row_light(self.row, light);
        let rgb = [self.texel[0], self.texel[1], self.texel[2]];
        let ground = expected(GRASS, d);
        let pixel = if let Some(road) = self.under {
            blended(self.texel, d, blended(road, d, ground))
        } else if self.row.blended() {
            blended(self.texel, d, ground)
        } else {
            expected(rgb, d)
        };
        match self.interface {
            Some(colour) => interface_over(colour, pixel),
            None => pixel,
        }
    }
}

#[derive(Resource)]
pub struct ColourProbe {
    report: PathBuf,
    frames: u32,
    quads: Vec<Quad>,
    /// The quads' centres in physical pixels, when captured.
    pixels: Vec<Option<UVec2>>,
    asked: Option<u32>,
    /// The day light the quads are lit by (GFX-VIEW-2.3b).
    light: world3d::DayLight,
}

impl ColourProbe {
    pub fn new(report: PathBuf) -> Self {
        Self {
            report,
            frames: 0,
            quads: Vec::new(),
            pixels: Vec::new(),
            asked: None,
            light: world3d::DayLight::XML,
        }
    }
}

/// A 1 x 1 texture of a raw (not sRGB) value, as the object textures are
/// uploaded (`models3d::model_image`).
fn texel_image(rgba: [u8; 4]) -> Image {
    Image::new_fill(
        Extent3d::default(),
        TextureDimension::D2,
        &rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// The light D of a row (GFX-VIEW-2.3): only the shadowed row has
/// something between it and the sun, and keeps the factor 1 - k x
/// intensity (0.1) of the direct light.
fn row_light(row: Row, light: &world3d::DayLight) -> [f32; 3] {
    let flat = -light.direction()[1];
    match row {
        Row::Flat | Row::Decal | Row::RoadOver | Row::DecalOver | Row::DecalOverRoad => {
            light.light_d(flat, 1.0)
        }
        Row::Shadowed => light.light_d(
            flat,
            world3d::shadow_factor(light.elevation, light.shadow_intensity),
        ),
        Row::Ambient => light.light_d(0.0, 1.0),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn drive(
    mut commands: Commands,
    mut probe: ResMut<ColourProbe>,
    view: Res<crate::render::view3d::View3d>,
    library: Res<ModelLibrary>,
    camera: Single<(Entity, &Camera, &GlobalTransform), With<MapCamera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ObjectMaterial>>,
    mut lights: Query<&mut RenderLayers, With<DirectionalLight>>,
    daylight: Res<crate::render::view3d::Daylight>,
    ui: Option<ResMut<crate::ui::game::GameUi>>,
) {
    // The quads' top row lies where the messages window stands from the
    // game's start (UI-GAME-11.3a): the probe measures the colour pipeline,
    // so that window is closed (as N closes it).
    if let Some(mut ui) = ui {
        if ui.messages {
            ui.messages = false;
        }
    }
    let (Some((_, params)), Some(scene_light), Some(palettes)) = (
        library.params.clone(),
        library.light.clone(),
        library.palettes.handle.clone(),
    ) else {
        return;
    };
    if !view.active {
        return;
    }
    probe.frames += 1;
    if probe.frames == FRAMES_BEFORE {
        for mut layers in &mut lights {
            *layers = RenderLayers::from_layers(&[MAP_LAYER, OCCLUDER_LAYER]);
        }
        let (_, _, transform) = *camera;
        // The map's day light (GFX-VIEW-2.3b), which the shaders use.
        probe.light = daylight.0;
        place(
            &mut commands,
            &mut probe,
            transform,
            (params, scene_light, palettes),
            &mut meshes,
            &mut images,
            &mut materials,
        );
    }
    // The interface over its quads, a small square at each centre.
    if probe.frames == FRAMES_BEFORE + 1 {
        let (entity, camera, transform) = *camera;
        for quad in &probe.quads {
            let Some(colour) = quad.interface else {
                continue;
            };
            let Ok(at) = camera.world_to_viewport(transform, quad.centre) else {
                continue;
            };
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(at.x - 6.0),
                    top: px(at.y - 6.0),
                    width: px(12),
                    height: px(12),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(colour[0], colour[1], colour[2], colour[3])),
                UiTargetCamera(entity),
            ));
        }
    }
    if probe.frames == FRAMES_BEFORE + FRAMES_AFTER && probe.asked.is_none() {
        let (_, camera, transform) = *camera;
        let scale = camera.target_scaling_factor().unwrap_or(1.0);
        probe.pixels = probe
            .quads
            .iter()
            .map(|q| {
                camera
                    .world_to_viewport(transform, q.centre)
                    .ok()
                    .map(|p| (p * scale).round().as_uvec2())
            })
            .collect();
        probe.asked = Some(probe.frames);
        commands
            .spawn(Screenshot::primary_window())
            .observe(captured);
    }
}

/// The quads, about 12 units in front of the camera and above everything
/// on the map: a row lying flat, a flat row with an occluder 3 units
/// toward the sun of each quad, and a row turned edge-on to the light.
fn place(
    commands: &mut Commands,
    probe: &mut ColourProbe,
    camera: &GlobalTransform,
    (params, scene_light, palettes): (
        ObjectParams,
        Handle<bevy::render::storage::ShaderBuffer>,
        Handle<bevy::render::storage::ShaderBuffer>,
    ),
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<ObjectMaterial>,
) {
    let size = 1.0;
    let half = size / 2.0;
    let quad = |indices: Vec<u32>| {
        crate::render::models3d::flat_object_mesh(
            vec![
                [-half, 0.0, -half],
                [half, 0.0, -half],
                [half, 0.0, half],
                [-half, 0.0, half],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![[1.0, 0.0]; 4],
            indices,
        )
    };
    let mesh = meshes.add(quad(vec![0, 2, 1, 0, 3, 2]));
    let back_wound = meshes.add(quad(vec![0, 1, 2, 0, 2, 3]));
    // Seen everywhere: the fog of war (GFX-VIEW-5) is not under test.
    let white = images.add(texel_image([255; 4]));
    // A normal map texel pointing straight up (tangent space).
    let up_normal = images.add(texel_image([128, 128, 255, 255]));
    let forward = camera.forward().as_vec3();
    let right = camera.right().as_vec3();
    let up = camera.up().as_vec3();
    let middle = camera.translation() + forward * 12.0;
    let light = Vec3::from(world3d::to_bevy(probe.light.direction()));
    for (r, row) in ROWS.into_iter().enumerate() {
        // Edge-on: a normal at right angles to the light that still faces
        // the camera, tipped a little away from the light so N . L < 0.
        let normal = match row {
            Row::Flat
            | Row::Shadowed
            | Row::Decal
            | Row::RoadOver
            | Row::DecalOver
            | Row::DecalOverRoad => Vec3::Y,
            Row::Ambient => {
                let back = -forward;
                (back - light * back.dot(light) + light * 0.05).normalize()
            }
        };
        let rotation = Quat::from_rotation_arc(Vec3::Y, normal);
        let texels = row.texels();
        let (line, first) = row.place(r);
        for (i, &texel) in texels.iter().enumerate() {
            let x = ((first + i) as f32 - (TEXELS.len() as f32 - 1.0) / 2.0) * 1.3;
            let y = (line as f32 - 2.0) * 1.25;
            let centre = middle + right * x + up * y;
            let family = match row {
                Row::Decal | Row::DecalOver | Row::DecalOverRoad => Family::Decal,
                Row::RoadOver => Family::Road,
                _ => Family::Plain,
            };
            let object = |base: Handle<Image>, family: Family| ObjectMaterial {
                params: ObjectParams {
                    tint: Vec4::ONE,
                    mode: Vec4::new(family as u32 as f32, 0.0, 0.0, 0.0),
                    ..params
                },
                base,
                world: white.clone(),
                palettes: palettes.clone(),
                second: if family == Family::Road {
                    up_normal.clone()
                } else {
                    white.clone()
                },
                third: white.clone(),
                light: scene_light.clone(),
            };
            let base = images.add(texel_image(texel));
            let material = materials.add(object(base, family));
            // The blended rows lie a hair over their opaque grass quad.
            let lift = if row.blended() {
                let grass = images.add(texel_image([GRASS[0], GRASS[1], GRASS[2], 255]));
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(materials.add(object(grass, Family::Plain))),
                    Transform::from_translation(centre).with_rotation(rotation),
                    RenderLayers::layer(MAP_LAYER),
                    bevy::light::NotShadowCaster,
                ));
                -forward * 0.01
            } else {
                Vec3::ZERO
            };
            // T27: the road 0.02 over the grass, the decal half as high:
            // sorted by distance alone the road would be mixed last.
            let under = (row == Row::DecalOverRoad).then(|| DECAL_OVER_ROAD[i].0);
            if let Some(road) = under {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(
                        materials.add(object(images.add(texel_image(road)), Family::Road)),
                    ),
                    Transform::from_translation(centre - forward * 0.02).with_rotation(rotation),
                    RenderLayers::layer(MAP_LAYER),
                    bevy::light::NotShadowCaster,
                ));
            }
            commands.spawn((
                Mesh3d(if row == Row::Decal {
                    back_wound.clone()
                } else {
                    mesh.clone()
                }),
                MeshMaterial3d(material),
                Transform::from_translation(centre + lift).with_rotation(rotation),
                RenderLayers::layer(MAP_LAYER),
                bevy::light::NotShadowCaster,
            ));
            if row == Row::Shadowed {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(materials.add(ObjectMaterial {
                        params: ObjectParams {
                            tint: Vec4::ONE,
                            mode: Vec4::new(Family::Plain as u32 as f32, 0.0, 0.0, 0.0),
                            ..params
                        },
                        base: white.clone(),
                        world: white.clone(),
                        palettes: palettes.clone(),
                        second: white.clone(),
                        third: white.clone(),
                        light: scene_light.clone(),
                    })),
                    Transform::from_translation(centre - light * 3.0).with_scale(Vec3::splat(1.2)),
                    RenderLayers::layer(OCCLUDER_LAYER),
                ));
            }
            let interface = (row == Row::Flat)
                .then(|| INTERFACE_OVER.iter().find(|(column, _)| *column == i))
                .flatten()
                .map(|(_, colour)| *colour);
            probe.quads.push(Quad {
                texel,
                under,
                interface,
                row,
                centre,
            });
        }
    }
}

/// Reads each quad's centre pixel from the captured frame, writes the
/// report and quits.
fn captured(
    shot: On<ScreenshotCaptured>,
    probe: Res<ColourProbe>,
    mut exit: MessageWriter<AppExit>,
) {
    let image = &shot.image;
    let format = image.texture_descriptor.format;
    let width = image.texture_descriptor.size.width;
    let mut lines = vec![format!("swapchain format: {format:?}")];
    let mut failed = false;
    let bgra = matches!(
        format,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
    );
    for (quad, pixel) in probe.quads.iter().zip(&probe.pixels) {
        let want = quad.expected(&probe.light);
        let got = pixel.and_then(|p| {
            let at = ((p.y * width + p.x) * 4) as usize;
            let bytes = image.data.as_ref()?.get(at..at + 4)?;
            Some(if bgra {
                [bytes[2], bytes[1], bytes[0]]
            } else {
                [bytes[0], bytes[1], bytes[2]]
            })
        });
        let ok = got.is_some_and(|g| (0..3).all(|c| g[c].abs_diff(want[c]) <= 1));
        failed |= !ok;
        lines.push(format!(
            "{} {:?}{} texel {:?} expected {:?} got {:?} at {:?}",
            if ok { "ok  " } else { "FAIL" },
            quad.row,
            quad.interface
                .map(|c| format!(" under interface {c:?}"))
                .unwrap_or_default(),
            quad.texel,
            want,
            got,
            pixel
        ));
    }
    let text = lines.join("\n") + "\n";
    if let Err(e) = std::fs::write(&probe.report, &text) {
        error!("cannot write {}: {e}", probe.report.display());
    }
    info!("colour probe:\n{text}");
    exit.write(if failed {
        AppExit::error()
    } else {
        AppExit::Success
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // GFX-VIEW-2.3: lit flat ground D = (1.3182, 1.1879, 1.1983); the
    // product is clamped only at the output.
    // GFX-VIEW T25, T26 (2.6 c1): straight blending of stored values over
    // the lit grass pixel (105, 131, 48).
    #[test]
    fn t25_t26_blended_rows() {
        let d = row_light(Row::Flat, &world3d::DayLight::XML);
        let ground = expected(GRASS, d);
        assert_eq!(ground, [105, 131, 48]);
        let road: Vec<_> = ROAD_OVER.iter().map(|t| blended(*t, d, ground)).collect();
        assert_eq!(road, [[110, 103, 54], [107, 122, 50], [105, 131, 48]]);
        let decal: Vec<_> = DECAL_OVER.iter().map(|t| blended(*t, d, ground)).collect();
        let want = [
            [116, 92, 54],
            [132, 113, 55],
            [104, 105, 43],
            [113, 121, 50],
            [108, 127, 49],
            [105, 131, 48],
        ];
        for (got, want) in decal.iter().zip(want) {
            assert!(
                (0..3).all(|c| got[c].abs_diff(want[c]) <= 1),
                "{got:?} {want:?}"
            );
        }
    }

    // GFX-VIEW-2.6 c, UI-GAME-1.11 (T46): the interface blends straight on
    // the stored values: the paused view's layer keeps 90 / 255 of a pixel
    // (128 grey lit flat: (169, 152, 153) -> (60, 54, 54)); in linear light
    // it would keep about 0.6 of it, (116, 104, 105).
    // spec: GFX-VIEW-2.6, UI-GAME-1.11 (UI-GAME T46)
    #[test]
    fn interface_blends_on_stored_values() {
        let d = row_light(Row::Flat, &world3d::DayLight::XML);
        let grey = expected([128; 3], d);
        assert_eq!(grey, [169, 152, 153]);
        assert_eq!(interface_over(INTERFACE_OVER[0].1, grey), [60, 54, 54]);
        // T46: 255 -> 90, 200 -> 71, 100 -> 35, 10 -> 4.
        let layer = INTERFACE_OVER[0].1;
        assert_eq!(interface_over(layer, [255, 200, 100]), [90, 71, 35]);
        assert_eq!(interface_over(layer, [10, 0, 0])[0], 4);
        let blue = expected([90, 140, 200], d);
        assert_eq!(blue, [119, 166, 240]);
        assert_eq!(interface_over(INTERFACE_OVER[1].1, blue), [160, 143, 150]);
    }

    // GFX-VIEW T27 (2.7): the strip first, then the decal over it.
    // spec: GFX-VIEW-2.7 (T27)
    #[test]
    fn t27_decal_over_strip() {
        let strip = blended([200, 0, 0, 128], [1.0; 3], [100; 3]);
        assert_eq!(strip, [150, 50, 50]);
        assert_eq!(blended([0, 0, 200, 128], [1.0; 3], strip), [75, 25, 125]);
        let reverse = blended(
            [200, 0, 0, 128],
            [1.0; 3],
            blended([0, 0, 200, 128], [1.0; 3], [100; 3]),
        );
        assert_eq!(reverse, [125, 25, 75]);
    }

    // spec: GFX-VIEW-2.6
    #[test]
    fn expected_is_texel_times_light_clamped() {
        let d = row_light(Row::Flat, &world3d::DayLight::XML);
        assert!((d[0] - 1.3182).abs() < 1e-3 && (d[1] - 1.1879).abs() < 1e-3);
        assert_eq!(expected([128, 128, 128], d), [169, 152, 153]);
        assert_eq!(expected([240, 240, 240], d), [255, 255, 255]);
        assert_eq!(
            expected(
                [100, 100, 100],
                row_light(Row::Ambient, &world3d::DayLight::XML)
            ),
            [60, 50, 60]
        );
        // GFX-VIEW T20, T21 (2.6): 40 / 128 / 220 lit and in shadow.
        let lit = row_light(Row::Flat, &world3d::DayLight::XML);
        let shade = row_light(Row::Shadowed, &world3d::DayLight::XML);
        assert_eq!(expected([40; 3], lit), [53, 48, 48]);
        assert_eq!(expected([220; 3], lit), [255, 255, 255]);
        assert_eq!(expected([40; 3], shade), [27, 23, 26]);
        assert_eq!(expected([128; 3], shade), [86, 73, 84]);
        assert_eq!(expected([220; 3], shade), [148, 125, 145]);
        // Shadowed flat ground (0.6718, 0.5688, 0.6598): about half.
        assert_eq!(
            expected(
                [200, 200, 200],
                row_light(Row::Shadowed, &world3d::DayLight::XML)
            ),
            [134, 114, 132]
        );
    }
}
