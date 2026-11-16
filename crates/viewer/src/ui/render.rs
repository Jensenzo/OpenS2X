//! Panels into Bevy UI nodes: sprites cut from the atlases, buttons that
//! fire when the mouse is released over them (UI-GAME-1.4), greyed when
//! disabled (1.5), sliders, tooltips, and texts drawn with the game's
//! bitmap fonts (1.7). A panel is rebuilt only when its description
//! changes.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
// The 0.20 focus system's interaction state (its old name is deprecated).
use bevy::ui::DeprecatedInteraction as Interaction;
use bevy::ui::{FocusPolicy, RelativeCursorPosition};
use s2_assets::bmfont;

use super::widgets::{
    Action, Align, Atlas, Font, Hover, Panel, PanelId, Rect as AtlasRect, Widget,
};

/// A loaded font: its metrics and its page as RGBA.
pub struct FontData {
    pub font: bmfont::Font,
    pub page: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// The margin `draw_text` leaves before and above a text's line box: a
/// glyph may start before the pen (a negative xoffset).
pub const TEXT_PAD: i32 = 2;

/// UI-GAME-1.7, UI-MENU-1.8a: draw `text` in `font`, tinted by `colour`
/// (the pages hold white glyphs with dark edges): RGBA, width, height. The
/// line box's top-left corner is at (`TEXT_PAD`, `TEXT_PAD`) of the image;
/// a character's cell goes to its xoffset, yoffset from the pen, and the
/// pen advances by xadvance (no kerning).
pub fn draw_text(font: &FontData, text: &str, colour: [u8; 3]) -> (Vec<u8>, u32, u32) {
    let f = &font.font;
    let width = (f.width(text) + 4 * TEXT_PAD).max(1) as u32;
    let height = (f.line_height + 2 * TEXT_PAD).max(1) as u32;
    let mut out = vec![0u8; (width * height * 4) as usize];
    let mut pen = TEXT_PAD;
    for c in text.chars() {
        let Some(g) = f.glyphs.get(&(c as u32)) else {
            continue;
        };
        for gy in 0..g.height {
            for gx in 0..g.width {
                let (sx, sy) = (g.x + gx, g.y + gy);
                if sx >= font.width || sy >= font.height {
                    continue;
                }
                let dx = pen + g.x_offset + gx as i32;
                let dy = TEXT_PAD + g.y_offset + gy as i32;
                if dx < 0 || dy < 0 || dx >= width as i32 || dy >= height as i32 {
                    continue;
                }
                let s = ((sy * font.width + sx) * 4) as usize;
                let d = ((dy as u32 * width + dx as u32) * 4) as usize;
                let a = font.page[s + 3];
                if a == 0 {
                    continue;
                }
                for k in 0..3 {
                    out[d + k] = (u32::from(font.page[s + k]) * u32::from(colour[k]) / 255) as u8;
                }
                out[d + 3] = out[d + 3].max(a);
            }
        }
        pen += g.x_advance;
    }
    (out, width, height)
}

/// `text` in lines of at most `width` pixels: broken at spaces, at line
/// breaks and at the two characters `\n` the language files use.
pub fn wrap(font: &bmfont::Font, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.replace("\\n", "\n").split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if font.width(&candidate) as f32 > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

/// UI-MENU-1.8 (b) (corrected 2026-11-14 b, E-ui-033, E-trace-026): a label
/// (a single text line: titles, headings, captions, status lines, counts,
/// text buttons) is drawn with flag 1 always added: its line box is centred
/// vertically on its y, the line top half the font's line height above it
/// (`small_no_alias_white` 12 -> 6 px, the others 16 -> 8 px).
pub fn label_top(at: [f32; 2], font: Font) -> [f32; 2] {
    let line = match font {
        Font::Heading => 12.0,
        _ => 16.0,
    };
    [at[0], at[1] - line / 2.0]
}

/// UI-MENU-1.8: the top-left of a text's image for its line top `at` (a
/// label's from [`label_top`]): the line box's top-left corner on it, moved
/// left by half the run's width `run` when centred (flag 8) or by all of it
/// when right-aligned (flag 16).
pub fn text_origin(at: [f32; 2], run: f32, align: Align) -> [f32; 2] {
    let x = match align {
        Align::TopLeft => at[0],
        Align::Centre | Align::TopCentre => at[0] - run / 2.0,
        Align::TopRight => at[0] - run,
    };
    let pad = TEXT_PAD as f32;
    [x - pad, at[1] - pad]
}

/// A drawn text: font, text, colour.
type TextKey = (Font, String, [u8; 3]);

/// The interface's loaded art and texts.
#[derive(Resource, Default)]
pub struct UiAssets {
    pub atlases: HashMap<Atlas, Option<Handle<Image>>>,
    pub fonts: HashMap<Font, Option<FontData>>,
    pub texts: s2_assets::lang::Texts,
    text_cache: HashMap<TextKey, (Handle<Image>, [f32; 2])>,
    pub minimap: Option<Handle<Image>>,
    /// The chosen map's preview and its file.
    pub preview: Option<(std::path::PathBuf, Handle<Image>)>,
    /// The films' pictures by `Atlas::Film` number (`video::play_films`).
    pub films: HashMap<u8, Handle<Image>>,
    /// UI-GAME-4.5: the ship view through its mask (`ship_view`).
    pub ship_view: Option<Handle<crate::render::ship_view::ShipViewMaterial>>,
}

impl UiAssets {
    /// The width of `text` on one line in `font` (its run, UI-MENU-1.8a).
    pub fn run_width(&self, font: Font, text: &str) -> f32 {
        self.fonts
            .get(&font)
            .and_then(Option::as_ref)
            .map_or(0.0, |data| data.font.width(text) as f32)
    }

    /// The line height of `font`.
    pub fn line_height(&self, font: Font) -> f32 {
        self.fonts
            .get(&font)
            .and_then(Option::as_ref)
            .map_or(0.0, |data| data.font.line_height as f32)
    }

    /// A text as an image and its size, made once.
    pub fn text_image(
        &mut self,
        font: Font,
        text: &str,
        colour: [u8; 3],
        images: &mut Assets<Image>,
    ) -> Option<(Handle<Image>, [f32; 2])> {
        let key = (font, text.to_owned(), colour);
        if let Some(found) = self.text_cache.get(&key) {
            return Some(found.clone());
        }
        if self.text_cache.len() > 4000 {
            self.text_cache.clear();
        }
        let data = self.fonts.get(&font)?.as_ref()?;
        let (rgba, w, h) = draw_text(data, text, colour);
        let mut image = Image::new(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        // Drawn 1:1, menus included (UI-MENU-1.1).
        image.sampler = bevy::image::ImageSampler::nearest();
        let found = (images.add(image), [w as f32, h as f32]);
        self.text_cache.insert(key, found.clone());
        Some(found)
    }
}

/// The panels to show this frame, set by the screen's system.
#[derive(Resource, Default)]
pub struct UiFrame {
    pub panels: Vec<Panel>,
    /// UI-GAME-9.3: the tooltip of what the pointer is over on the map.
    pub map_tooltip: Option<String>,
}

impl UiFrame {
    /// A screen point lies on a dialog or the bar (the construction
    /// view's labels and the paused view's layer, which take no clicks,
    /// aside).
    pub fn covers(&self, p: [f32; 2]) -> bool {
        self.panels
            .iter()
            .filter(|panel| !matches!(panel.id, PanelId::Labels | PanelId::Frame | PanelId::Pause))
            .filter_map(|panel| panel.bounds())
            .any(|r| p[0] >= r[0] && p[1] >= r[1] && p[0] < r[0] + r[2] && p[1] < r[1] + r[3])
    }
}

/// The panels on screen: root entity and the hash of its description.
#[derive(Resource, Default)]
pub struct UiRoots {
    roots: HashMap<PanelId, (Entity, u64)>,
}

impl UiEvents {
    /// The action of the button the mouse went down on, until it is
    /// released (the play harness's report).
    pub fn pressed(&self) -> Option<&Action> {
        self.pressed.as_ref()
    }
}

/// Actions fired this frame, and whether the mouse is over the interface.
#[derive(Resource, Default)]
pub struct UiEvents {
    pub actions: Vec<Action>,
    /// The action of the button the mouse went down on, kept across panel
    /// rebuilds until the button is released.
    pressed: Option<Action>,
    /// The slider being dragged and its value.
    dragging: Option<(super::widgets::SliderKind, f32)>,
    /// The picture (its rectangle on screen) the mouse went down on, kept
    /// until the button is released.
    picture_down: Option<[f32; 4]>,
    pub over_ui: bool,
    /// Sounds the interface asks for (UI-GAME-1.4, SND-4), by name.
    pub sounds: Vec<&'static str>,
    /// The action of the enabled button under the pointer this frame.
    pub hovered: Option<Action>,
    /// Where the pointer was when the last action fired.
    pub pointer: Option<[f32; 2]>,
    /// UI-MENU-2.1 (Q-1042): the held arrow's repeat countdown, seconds.
    repeat: f32,
}

/// UI-MENU-2.1 (Q-1042): a list's scroll arrow steps when pressed and
/// again while held ([`crate::menus::menu::bar_repeat`]), not on release.
fn repeats(action: &Action) -> bool {
    use super::widgets::MenuAction as M;
    matches!(action, Action::Menu(M::MapScroll(_) | M::SaveScroll(_)))
}

/// UI-MENU-2.1 (Q-1042): a list's pressed bar, sent every update with the
/// pointer `px` below the bar's top while it is held.
fn track_at(action: &Action, px: i32) -> Option<Action> {
    use super::widgets::MenuAction as M;
    match action {
        Action::Menu(M::MapTrack(_)) => Some(Action::Menu(M::MapTrack(px))),
        Action::Menu(M::SaveTrack(_)) => Some(Action::Menu(M::SaveTrack(px))),
        _ => None,
    }
}

/// The screen top of the button with `action`, as [`widget_at`] places it.
fn button_top(panels: &[Panel], action: &Action) -> Option<f32> {
    panels.iter().find_map(|panel| {
        let origin = panel.at[1].round();
        panel.widgets.iter().find_map(|w| match w {
            Widget::Button { at, action: a, .. } if a == action => Some(origin + at[1].round()),
            _ => None,
        })
    })
}

/// What takes a click at a point of the screen.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    Button {
        action: Action,
        enabled: bool,
    },
    /// `fraction`: the point along the slider, 0 to 1.
    Slider {
        kind: super::widgets::SliderKind,
        fraction: f32,
        enabled: bool,
    },
    /// The minimap or a map's preview, with its rectangle on screen.
    Picture {
        rect: [f32; 4],
    },
}

/// UI-GAME-1.4, 1.10c: what takes a click at `p`: the top widget whose
/// rectangle holds the point, read from the panels on screen, not from their
/// nodes. A node just made has no place yet: it reads no pointer for some
/// frames, and every panel is made again when its description changes (a text,
/// a late map preview, the row under the pointer). A click in those frames
/// counts all the same. Texts, sprites and fills take no clicks and let them
/// through, as their nodes do; `size` is the size of the node a widget makes
/// (`None`: it makes none).
pub fn widget_at(
    panels: &[Panel],
    p: [f32; 2],
    size: &mut dyn FnMut(&Widget) -> Option<[f32; 2]>,
) -> Option<Hit> {
    let mut order: Vec<&Panel> = panels.iter().collect();
    // Stable: of equal height, the later panel lies on top.
    order.sort_by_key(|panel| z_of(panel.id));
    for panel in order.iter().rev() {
        let origin = [panel.at[0].round(), panel.at[1].round()];
        for w in panel.widgets.iter().rev() {
            let at = match w {
                Widget::Button { at, .. }
                | Widget::BarButton { at, .. }
                | Widget::Slider { at, .. }
                | Widget::Picture { at, .. } => *at,
                Widget::TextButton { at, font, .. } => {
                    text_origin(label_top(*at, *font), 0.0, Align::TopLeft)
                }
                _ => continue,
            };
            let Some(wh) = size(w) else { continue };
            let left = [origin[0] + at[0].round(), origin[1] + at[1].round()];
            let inside = p[0] >= left[0]
                && p[1] >= left[1]
                && p[0] < left[0] + wh[0]
                && p[1] < left[1] + wh[1];
            if !inside {
                continue;
            }
            return Some(match w {
                Widget::Button {
                    action, enabled, ..
                }
                | Widget::BarButton {
                    action, enabled, ..
                }
                | Widget::TextButton {
                    action, enabled, ..
                } => Hit::Button {
                    action: action.clone(),
                    enabled: *enabled,
                },
                Widget::Slider { kind, enabled, .. } => Hit::Slider {
                    kind: *kind,
                    fraction: ((p[0] - left[0]) / wh[0]).clamp(0.0, 1.0),
                    enabled: *enabled,
                },
                _ => Hit::Picture {
                    rect: [left[0], left[1], wh[0], wh[1]],
                },
            });
        }
    }
    None
}

/// The size of the node a widget that takes clicks makes, as `spawn_panel`
/// makes it: none without its art.
fn node_size(assets: &mut UiAssets, images: &mut Assets<Image>, w: &Widget) -> Option<[f32; 2]> {
    let has = |assets: &UiAssets, a: Atlas| match a {
        Atlas::Film(n) => assets.films.contains_key(&n),
        _ => assets.atlases.get(&a).is_some_and(Option::is_some),
    };
    match w {
        Widget::Button {
            atlas, rect, size, ..
        } => has(assets, *atlas).then(|| size.unwrap_or([rect[2] as f32, rect[3] as f32])),
        Widget::BarButton { bar, .. } => has(assets, Atlas::Elements0).then_some(bar.size),
        Widget::TextButton {
            text,
            font,
            colour,
            enabled,
            ..
        } => {
            let colour = if *enabled { *colour } else { [120, 120, 120] };
            assets
                .text_image(*font, text, colour, images)
                .map(|(_, size)| size)
        }
        Widget::Slider { width, .. } => Some(super::widgets::SliderArt::of(*width).size()),
        Widget::Picture { picture, size, .. } => {
            let made = match picture {
                super::widgets::Picture::Minimap => assets.minimap.is_some(),
                super::widgets::Picture::MapPreview => assets.preview.is_some(),
                super::widgets::Picture::ShipView => false,
            };
            made.then_some([*size, *size])
        }
        _ => None,
    }
}

/// A button of a panel.
#[derive(Component)]
pub struct UiButton {
    enabled: bool,
    selected: bool,
    rect: AtlasRect,
    hover: Hover,
    /// UI-MENU-1.4: the look while the mouse button is down on it.
    pressed: Hover,
    /// A text button's normal and highlighted pictures.
    images: Option<(Handle<Image>, Handle<Image>)>,
    tooltip: Option<String>,
    was_pressed: bool,
}

/// A slider of a panel.
#[derive(Component)]
pub struct UiSlider {
    kind: super::widgets::SliderKind,
    value: f32,
    /// UI-GAME-6.5: its kind's art.
    art: super::widgets::SliderArt,
    enabled: bool,
    was_pressed: bool,
    knob: Entity,
    /// UI-GAME-6.5 (a): the economy slider's fill.
    fill: Option<Entity>,
}

/// A slider's part at `left` from its left edge, at its top, unrounded
/// (UI-GAME-6.5: the knob's left is 2 + 177 v).
fn slider_node(left: f32, size: [f32; 2]) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(0.0),
        width: Val::Px(size[0]),
        height: Val::Px(size[1]),
        ..default()
    }
}

/// The root of a panel.
#[derive(Component)]
pub struct UiPanel;

#[derive(Component)]
pub struct UiTooltip;

fn hash_of(panel: &Panel) -> u64 {
    let mut h = DefaultHasher::new();
    format!("{panel:?}").hash(&mut h);
    h.finish()
}

fn abs_node(at: [f32; 2], size: [f32; 2]) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(at[0].round()),
        top: Val::Px(at[1].round()),
        width: Val::Px(size[0]),
        height: Val::Px(size[1]),
        ..default()
    }
}

fn atlas_image(rect: AtlasRect, handle: Handle<Image>) -> ImageNode {
    ImageNode {
        image: handle,
        // A sprite fills its node, also when drawn at another size.
        image_mode: bevy::ui::widget::NodeImageMode::Stretch,
        rect: Some(Rect::new(
            rect[0] as f32,
            rect[1] as f32,
            (rect[0] + rect[2]) as f32,
            (rect[1] + rect[3]) as f32,
        )),
        ..default()
    }
}

/// The colour of a button's image by its state.
fn tint(enabled: bool) -> Color {
    if enabled {
        Color::WHITE
    } else {
        Color::srgb(0.45, 0.45, 0.45)
    }
}

fn z_of(id: PanelId) -> i32 {
    match id {
        PanelId::Frame => 1,
        PanelId::Labels => 5,
        PanelId::Hud => 10,
        PanelId::Hud2 => 10,
        PanelId::Briefing => 11,
        PanelId::Minimap => 12,
        PanelId::Messages => 13,
        PanelId::Global => 14,
        PanelId::Context => 15,
        PanelId::Error => 16,
        PanelId::Chat => 16,
        PanelId::Tutorial => 17,
        PanelId::Book => 18,
        PanelId::Pause => 19,
        PanelId::Menu => 20,
        PanelId::Notice => 25,
        PanelId::Tooltip => 30,
    }
}

/// Spawn one panel's nodes.
fn spawn_panel(
    commands: &mut Commands,
    assets: &mut UiAssets,
    images: &mut Assets<Image>,
    panel: &Panel,
    carried: &dyn Fn(&Action) -> Interaction,
) -> Entity {
    let atlas = |assets: &UiAssets, a: Atlas| match a {
        Atlas::Film(n) => assets.films.get(&n).cloned(),
        _ => assets.atlases.get(&a).cloned().flatten(),
    };
    let root = commands
        .spawn((
            UiPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(panel.at[0].round()),
                top: Val::Px(panel.at[1].round()),
                ..default()
            },
            GlobalZIndex(z_of(panel.id)),
            FocusPolicy::Pass,
        ))
        .id();
    for w in &panel.widgets {
        let child = match w {
            Widget::Sprite {
                atlas: a,
                rect,
                at,
                size,
                greyed,
            } => {
                let Some(handle) = atlas(assets, *a) else {
                    continue;
                };
                let size = size.unwrap_or([rect[2] as f32, rect[3] as f32]);
                let mut image = atlas_image(*rect, handle);
                image.color = tint(!greyed);
                commands.spawn((abs_node(*at, size), image)).id()
            }
            Widget::Button {
                atlas: a,
                rect,
                hover,
                pressed,
                at,
                size,
                tooltip,
                action,
                enabled,
                selected,
            } => {
                let Some(handle) = atlas(assets, *a) else {
                    continue;
                };
                let size = size.unwrap_or([rect[2] as f32, rect[3] as f32]);
                let mut image = atlas_image(*rect, handle);
                image.color = tint(*enabled);
                commands
                    .spawn((
                        abs_node(*at, size),
                        image,
                        FocusPolicy::Block,
                        carried(action),
                        UiButton {
                            enabled: *enabled,
                            selected: *selected,
                            rect: *rect,
                            hover: hover.clone(),
                            pressed: pressed.clone(),
                            images: None,
                            tooltip: tooltip.clone(),
                            was_pressed: false,
                        },
                    ))
                    .id()
            }
            Widget::Text {
                text,
                font,
                at,
                colour,
                align,
            } => {
                let Some((handle, size)) = assets.text_image(*font, text, *colour, images) else {
                    continue;
                };
                let at = text_origin(label_top(*at, *font), assets.run_width(*font, text), *align);
                commands
                    .spawn((
                        abs_node(at, size),
                        ImageNode {
                            image: handle,
                            ..default()
                        },
                        FocusPolicy::Pass,
                    ))
                    .id()
            }
            Widget::Paragraph {
                text,
                font,
                at,
                width,
                colour,
                align,
            } => {
                let Some(Some(data)) = assets.fonts.get(font) else {
                    continue;
                };
                let step = data.font.line_height.max(1) as f32;
                let lines = wrap(&data.font, text, *width);
                let centred = *align == Align::Centre;
                let top = if centred {
                    at[1] - step * lines.len() as f32 / 2.0
                } else {
                    at[1]
                };
                let mut ids = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    if line.is_empty() {
                        continue;
                    }
                    let Some((handle, size)) = assets.text_image(*font, line, *colour, images)
                    else {
                        continue;
                    };
                    let left = if centred {
                        at[0] + (width - assets.run_width(*font, line)) / 2.0
                    } else {
                        at[0]
                    };
                    let at = text_origin([left, top + step * i as f32], 0.0, Align::TopLeft);
                    ids.push(
                        commands
                            .spawn((
                                abs_node(at, size),
                                ImageNode {
                                    image: handle,
                                    ..default()
                                },
                                FocusPolicy::Pass,
                            ))
                            .id(),
                    );
                }
                let Some(last) = ids.pop() else { continue };
                for id in ids {
                    commands.entity(root).add_child(id);
                }
                last
            }
            Widget::TextButton {
                text,
                font,
                at,
                colour,
                action,
                enabled,
            } => {
                let colour = if *enabled { *colour } else { [120, 120, 120] };
                let Some((handle, size)) = assets.text_image(*font, text, colour, images) else {
                    continue;
                };
                // UI-MENU-1.4: teal while highlighted.
                let lit = assets
                    .text_image(*font, text, super::widgets::HEADING, images)
                    .map(|(h, _)| h)
                    .unwrap_or_else(|| handle.clone());
                commands
                    .spawn((
                        abs_node(
                            text_origin(label_top(*at, *font), 0.0, Align::TopLeft),
                            size,
                        ),
                        ImageNode {
                            image: handle.clone(),
                            ..default()
                        },
                        FocusPolicy::Block,
                        carried(action),
                        UiButton {
                            enabled: *enabled,
                            selected: false,
                            rect: [0, 0, 0, 0],
                            hover: Hover::None,
                            pressed: Hover::None,
                            images: Some((handle, lit)),
                            tooltip: None,
                            was_pressed: false,
                        },
                    ))
                    .id()
            }
            Widget::BarButton {
                text,
                font,
                at,
                bar,
                raise,
                action,
                enabled,
            } => {
                let Some(handle) = atlas(assets, Atlas::Elements0) else {
                    continue;
                };
                // UI-MENU-1.4: a disabled button wears the pressed bar.
                let look = if *enabled { bar.normal } else { bar.pressed };
                let rect = [look[0], look[1], bar.size[0] as u32, bar.size[1] as u32];
                let colour = if *enabled {
                    [255, 255, 255]
                } else {
                    [170, 170, 170]
                };
                let e = commands
                    .spawn((
                        abs_node(*at, bar.size),
                        atlas_image(rect, handle),
                        FocusPolicy::Block,
                        carried(action),
                        UiButton {
                            enabled: *enabled,
                            selected: false,
                            // A disabled bar keeps its pressed look.
                            rect: [look[0], look[1], rect[2], rect[3]],
                            hover: Hover::Swap(bar.hover),
                            pressed: Hover::Swap(bar.pressed),
                            images: None,
                            tooltip: panel
                                .tooltips
                                .iter()
                                .find(|(a, _)| a == action)
                                .map(|(_, t)| t.clone()),
                            was_pressed: false,
                        },
                    ))
                    .id();
                // UI-MENU-1.4: the label's line box centred on the bar.
                let run = assets.run_width(*font, text);
                let line = assets.line_height(*font);
                if let Some((text, size)) = assets.text_image(*font, text, colour, images) {
                    let label = commands
                        .spawn((
                            abs_node(
                                text_origin(
                                    [bar.size[0] / 2.0, (bar.size[1] - line) / 2.0 - raise],
                                    run,
                                    Align::TopCentre,
                                ),
                                size,
                            ),
                            ImageNode {
                                image: text,
                                ..default()
                            },
                            FocusPolicy::Pass,
                        ))
                        .id();
                    commands.entity(e).add_child(label);
                }
                e
            }
            Widget::Slider {
                at,
                width,
                value,
                kind,
                enabled,
            } => {
                // UI-GAME-6.5 (T61): the bar, the economy slider's fill cut to
                // 200 v from its left edge (tinted while disabled), and the
                // 20 x 20 knob at 2 + v (width - 23), all from the game's
                // pictures; without them a flat bar and knob stand in.
                let art = super::widgets::SliderArt::of(*width);
                let size = art.size();
                let pictures = atlas(assets, art.bar.0).zip(atlas(assets, Atlas::Elements0));
                if let Some((bar, elements0)) = pictures {
                    let fill = art.fill.then(|| {
                        let rect = art.fill_rect(*value).unwrap_or([0, 104, 0, 20]);
                        let mut image =
                            atlas_image(rect, atlas(assets, Atlas::Elements2).unwrap_or_default());
                        if !*enabled {
                            let [r, g, b, a] = super::widgets::DISABLED_FILL;
                            image.color = Color::srgba(r, g, b, a);
                        }
                        commands
                            .spawn((
                                slider_node(0.0, [rect[2] as f32, 20.0]),
                                image,
                                FocusPolicy::Pass,
                            ))
                            .id()
                    });
                    let knob = commands
                        .spawn((
                            slider_node(art.knob_left(*value), [20.0, 20.0]),
                            atlas_image(super::widgets::SLIDER_KNOB[0], elements0),
                            FocusPolicy::Pass,
                        ))
                        .id();
                    let e = commands
                        .spawn((
                            abs_node(*at, size),
                            atlas_image(art.bar.1, bar),
                            FocusPolicy::Block,
                            Interaction::default(),
                            RelativeCursorPosition::default(),
                            UiSlider {
                                kind: *kind,
                                value: *value,
                                art,
                                enabled: *enabled,
                                was_pressed: false,
                                knob,
                                fill,
                            },
                        ))
                        .id();
                    if let Some(fill) = fill {
                        commands.entity(e).add_child(fill);
                    }
                    commands.entity(e).add_child(knob);
                    e
                } else {
                    let knob = commands
                        .spawn((
                            abs_node([art.knob_left(*value), 0.0], [20.0, size[1]]),
                            BackgroundColor(Color::srgb(0.95, 0.78, 0.35)),
                            FocusPolicy::Pass,
                        ))
                        .id();
                    let e = commands
                        .spawn((
                            abs_node(*at, size),
                            BackgroundColor(Color::srgba(0.02, 0.1, 0.1, 0.9)),
                            FocusPolicy::Block,
                            Interaction::default(),
                            RelativeCursorPosition::default(),
                            UiSlider {
                                kind: *kind,
                                value: *value,
                                art,
                                enabled: *enabled,
                                was_pressed: false,
                                knob,
                                fill: None,
                            },
                        ))
                        .id();
                    commands.entity(e).add_child(knob);
                    e
                }
            }
            Widget::Fill { at, size, rgba } => commands
                .spawn((
                    abs_node(*at, *size),
                    BackgroundColor(Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])),
                    FocusPolicy::Pass,
                ))
                .id(),
            Widget::Picture {
                picture: super::widgets::Picture::ShipView,
                at,
                size,
            } => {
                let Some(material) = assets.ship_view.clone() else {
                    continue;
                };
                commands
                    .spawn((
                        abs_node(*at, [*size, *size]),
                        bevy::ui_render::prelude::MaterialNode(material),
                        FocusPolicy::Pass,
                    ))
                    .id()
            }
            Widget::Picture { picture, at, size } => {
                let handle = match picture {
                    super::widgets::Picture::Minimap => assets.minimap.clone(),
                    super::widgets::Picture::MapPreview => assets.preview.clone().map(|p| p.1),
                    super::widgets::Picture::ShipView => None,
                };
                let Some(handle) = handle else {
                    continue;
                };
                commands
                    .spawn((
                        abs_node(*at, [*size, *size]),
                        ImageNode {
                            image: handle,
                            ..default()
                        },
                        Interaction::default(),
                        RelativeCursorPosition::default(),
                        MinimapNode,
                    ))
                    .id()
            }
        };
        commands.entity(root).add_child(child);
    }
    root
}

/// The minimap picture: a click on it moves the camera.
#[derive(Component)]
pub struct MinimapNode;

/// Bring the panels on screen in line with [`UiFrame`].
pub fn sync_panels(
    mut commands: Commands,
    frame: Res<UiFrame>,
    mut roots: ResMut<UiRoots>,
    mut assets: ResMut<UiAssets>,
    mut images: ResMut<Assets<Image>>,
    events: Res<UiEvents>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    // A rebuilt button takes over the state of the one it replaces: a new
    // node reads no pointer for a few frames, which would drop the hover
    // (and with it what the hover shows) and a click released then.
    let carried = |action: &Action| {
        if mouse.pressed(MouseButton::Left) && events.pressed.as_ref() == Some(action) {
            Interaction::Pressed
        } else if events.hovered.as_ref() == Some(action) {
            Interaction::Hovered
        } else {
            Interaction::None
        }
    };
    // The map preview is drawn from the asset of the moment: a panel with
    // it is built again when it changes.
    let preview = assets.preview.as_ref().map(|p| p.1.id());
    let mut keep = Vec::new();
    for panel in &frame.panels {
        keep.push(panel.id);
        let h = hash_of(panel) ^ {
            let mut hasher = DefaultHasher::new();
            format!("{preview:?}").hash(&mut hasher);
            hasher.finish()
        };
        if let Some((entity, old)) = roots.roots.get(&panel.id) {
            if *old == h {
                continue;
            }
            commands.entity(*entity).despawn();
        }
        let e = spawn_panel(&mut commands, &mut assets, &mut images, panel, &carried);
        roots.roots.insert(panel.id, (e, h));
    }
    let gone: Vec<PanelId> = roots
        .roots
        .keys()
        .filter(|id| !keep.contains(id))
        .copied()
        .collect();
    for id in gone {
        if let Some((e, _)) = roots.roots.remove(&id) {
            commands.entity(e).despawn();
        }
    }
}

/// UI-GAME-1.2, 1.4, 1.5: hover sprites, clicks on release, tooltips. The
/// looks and tooltips are the nodes'; what a click does is [`widget_at`]'s, so
/// a click in the first frames of a panel is not lost.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn interact(
    mouse: Res<ButtonInput<MouseButton>>,
    mut events: ResMut<UiEvents>,
    mut buttons: Query<(&Interaction, &mut UiButton, &mut ImageNode)>,
    mut sliders: Query<(&Interaction, &RelativeCursorPosition, &mut UiSlider)>,
    mut knobs: Query<&mut Node>,
    mut parts: Query<&mut ImageNode, Without<UiButton>>,
    minimap: Query<&Interaction, With<MinimapNode>>,
    mut tooltip: Local<Option<(String, Entity)>>,
    mut commands: Commands,
    mut assets: ResMut<UiAssets>,
    mut images: ResMut<Assets<Image>>,
    window: Single<&Window>,
    frame: Res<UiFrame>,
    time: Res<Time>,
) {
    let hit = window.cursor_position().and_then(|c| {
        widget_at(&frame.panels, [c.x, c.y], &mut |w| {
            node_size(&mut assets, &mut images, w)
        })
    });
    let under = match &hit {
        Some(Hit::Button {
            action,
            enabled: true,
        }) => Some(action.clone()),
        _ => None,
    };
    let mut over = hit.is_some();
    let mut tip: Option<String> = None;
    events.hovered = under.clone();
    if mouse.just_pressed(MouseButton::Left) {
        events.pressed = under.clone();
        if let Some(Hit::Slider {
            kind,
            fraction,
            enabled: true,
        }) = hit
        {
            events.dragging = Some((kind, fraction));
        }
        events.picture_down = match hit {
            Some(Hit::Picture { rect }) => Some(rect),
            _ => None,
        };
        // UI-MENU-2.1 (Q-1042): a scroll arrow steps at once.
        if let Some(action) = under.clone().filter(repeats) {
            events.sounds.push("uiClick1");
            events.actions.push(action);
            events.repeat = crate::menus::menu::BAR_REPEAT;
        }
    }
    if mouse.pressed(MouseButton::Left) && !mouse.just_pressed(MouseButton::Left) {
        // UI-MENU-2.1 (Q-1042): held on the arrow, it steps again each time
        // the countdown runs out.
        if let Some(action) = events.pressed.clone().filter(repeats) {
            if under.as_ref() == Some(&action) {
                let (due, left) = crate::menus::menu::bar_repeat(events.repeat, time.delta_secs());
                events.repeat = left;
                if due {
                    events.actions.push(action);
                }
            }
        }
    }
    // UI-MENU-2.1 (Q-1042): the pressed bar follows the pointer every
    // update, wherever the pointer goes while the button is held.
    if mouse.pressed(MouseButton::Left) {
        let pressed = events.pressed.clone();
        if let (Some(action), Some(c)) = (pressed, window.cursor_position()) {
            if let Some(top) = track_at(&action, 0).and_then(|_| button_top(&frame.panels, &action))
            {
                if let Some(a) = track_at(&action, (c.y - top).round() as i32) {
                    events.actions.push(a);
                }
            }
        }
    }
    // UI-GAME-1.4: fire on release over the button pressed (an arrow and
    // the bar fired while held).
    let held_kind = events
        .pressed
        .as_ref()
        .is_some_and(|a| repeats(a) || track_at(a, 0).is_some());
    if mouse.just_released(MouseButton::Left)
        && under.is_some()
        && events.pressed == under
        && !held_kind
    {
        if let Some(action) = under.clone() {
            // UI-MENU-5.3: focusing a text input plays `uiClick0`.
            events
                .sounds
                .push(if matches!(action, Action::FocusInput(..)) {
                    "uiClick0"
                } else {
                    "uiClick1"
                });
            events.actions.push(action);
            events.pointer = window.cursor_position().map(|c| [c.x, c.y]);
        }
    }
    // UI-GAME-11.4: a right click on a message row deletes it.
    if mouse.just_released(MouseButton::Right) {
        if let Some(Action::MessageRow(key) | Action::MessageText(key)) = &under {
            events.actions.push(Action::DeleteMessage(*key));
            events.sounds.push("uiRemove");
        }
    }
    for (interaction, mut b, mut image) in &mut buttons {
        let pressed = *interaction == Interaction::Pressed;
        let hovered = *interaction != Interaction::None;
        over |= hovered;
        if hovered {
            if let Some(t) = &b.tooltip {
                tip = Some(t.clone());
            }
        }
        if b.rect[2] > 0 {
            // UI-MENU-1.4: the pressed look while the button is down on it,
            // the highlighted look while the pointer is over it.
            let show = if b.enabled && pressed && matches!(b.pressed, Hover::Swap(_)) {
                match b.pressed {
                    Hover::Swap([x, y]) => [x, y],
                    _ => [b.rect[0], b.rect[1]],
                }
            } else if b.enabled && (hovered || b.selected) {
                match b.hover {
                    Hover::Swap([x, y]) => [x, y],
                    _ => [b.rect[0], b.rect[1]],
                }
            } else {
                [b.rect[0], b.rect[1]]
            };
            let r = Rect::new(
                show[0] as f32,
                show[1] as f32,
                (show[0] + b.rect[2]) as f32,
                (show[1] + b.rect[3]) as f32,
            );
            if image.rect != Some(r) {
                image.rect = Some(r);
            }
        }
        if let Some((normal, lit)) = &b.images {
            let want = if b.enabled && hovered { lit } else { normal };
            if image.image != *want {
                image.image = want.clone();
            }
        }
        b.was_pressed = pressed;
    }
    for (interaction, cursor, mut s) in &mut sliders {
        over |= *interaction != Interaction::None;
        let pressed = *interaction == Interaction::Pressed && s.enabled;
        // UI-GAME-6.5: the knob normal, highlighted under the pointer, and
        // pressed while held.
        let dragged = events.dragging.is_some_and(|(kind, _)| kind == s.kind);
        let look = if pressed || dragged {
            2
        } else if *interaction == Interaction::Hovered && s.enabled {
            1
        } else {
            0
        };
        if let Ok(mut image) = parts.get_mut(s.knob) {
            let r = super::widgets::SLIDER_KNOB[look];
            let want = Rect::new(
                r[0] as f32,
                r[1] as f32,
                (r[0] + r[2]) as f32,
                (r[1] + r[3]) as f32,
            );
            if image.rect.is_some() && image.rect != Some(want) {
                image.rect = Some(want);
            }
        }
        if let Some((kind, value)) = events.dragging.as_mut() {
            if *kind == s.kind {
                if let (true, Some(n)) = (mouse.pressed(MouseButton::Left), cursor.normalized) {
                    *value = (n.x + 0.5).clamp(0.0, 1.0);
                }
                s.value = *value;
                if let Ok(mut node) = knobs.get_mut(s.knob) {
                    node.left = Val::Px(s.art.knob_left(s.value));
                }
                // UI-GAME-6.5 (a): the fill follows, cut to 200 v.
                if let Some(fill) = s.fill {
                    let rect = s.art.fill_rect(s.value).unwrap_or([0, 104, 0, 20]);
                    if let Ok(mut node) = knobs.get_mut(fill) {
                        node.width = Val::Px(rect[2] as f32);
                    }
                    if let Ok(mut image) = parts.get_mut(fill) {
                        image.rect = Some(Rect::new(
                            rect[0] as f32,
                            rect[1] as f32,
                            (rect[0] + rect[2]) as f32,
                            (rect[1] + rect[3]) as f32,
                        ));
                    }
                }
            }
        }
        s.was_pressed = pressed;
    }
    if mouse.just_released(MouseButton::Left) {
        if let Some((kind, value)) = events.dragging.take() {
            events.actions.push(Action::Slider(kind, value));
            events.sounds.push("uiClick1");
        }
        events.pressed = None;
        events.picture_down = None;
    }
    for i in &minimap {
        over |= *i != Interaction::None;
    }
    events.over_ui = over;
    // The tooltip next to the cursor: a button's, else (UI-GAME-9.3) that
    // of the map under it when no panel covers the pointer.
    let cursor = window.cursor_position();
    let on_map = cursor.is_some_and(|c| !over && !frame.covers(c.to_array()));
    let map_tip = if tip.is_none() && on_map {
        frame.map_tooltip.clone()
    } else {
        None
    };
    let screen = [window.width(), window.height()];
    let place = |c: Vec2, size: Vec2, map: bool| -> Vec2 {
        if map {
            Vec2::from(super::game::map_tooltip_at(
                c.to_array(),
                size.to_array(),
                screen,
            ))
        } else {
            Vec2::from(super::game::control_tooltip_at(
                c.to_array(),
                size.to_array(),
                screen,
            ))
        }
    };
    let is_map = map_tip.is_some();
    let tip = tip.or(map_tip);
    match (&tip, cursor) {
        (Some(text), Some(c)) => {
            let rebuild = tooltip.as_ref().is_none_or(|(t, _)| t != text);
            if rebuild {
                if let Some((_, e)) = tooltip.take() {
                    commands.entity(e).despawn();
                }
                if let Some((handle, size)) =
                    assets.text_image(Font::Body, text, [255, 255, 255], &mut images)
                {
                    // UI-GAME-9.3: the map's box is 32 px of a 768 layout.
                    // UI-GAME-2.2 (seen): a control's box 32 px high with a
                    // thin light frame, its width fitted to the text.
                    let box_size = if is_map {
                        [
                            size[0] + 12.0,
                            super::game::map_tooltip_height(screen[1]).max(size[1]),
                        ]
                    } else {
                        [
                            size[0] + 12.0,
                            super::game::CONTROL_TOOLTIP_HEIGHT.max(size[1]),
                        ]
                    };
                    let frame = if is_map { 0.0 } else { 1.0 };
                    let at = place(c, Vec2::from(box_size), is_map);
                    let e = commands
                        .spawn((
                            UiTooltip,
                            Node {
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(frame)),
                                ..abs_node([at.x, at.y], box_size)
                            },
                            BorderColor::all(Color::srgb(0.75, 0.75, 0.75)),
                            BackgroundColor(Color::srgba(0.0, 0.05, 0.05, 0.85)),
                            GlobalZIndex(z_of(PanelId::Tooltip)),
                            FocusPolicy::Pass,
                        ))
                        .with_child((
                            Node {
                                width: Val::Px(size[0]),
                                height: Val::Px(size[1]),
                                ..default()
                            },
                            ImageNode {
                                image: handle,
                                ..default()
                            },
                            FocusPolicy::Pass,
                        ))
                        .id();
                    *tooltip = Some((text.clone(), e));
                }
            } else if let Some((_, e)) = tooltip.as_ref() {
                if let Ok(mut node) = knobs.get_mut(*e) {
                    let size = match (node.width, node.height) {
                        (Val::Px(w), Val::Px(h)) => Vec2::new(w, h),
                        _ => Vec2::ZERO,
                    };
                    let at = place(c, size, is_map);
                    node.left = Val::Px(at.x);
                    node.top = Val::Px(at.y);
                }
            }
        }
        _ => {
            if let Some((_, e)) = tooltip.take() {
                commands.entity(e).despawn();
            }
        }
    }
}

/// The minimap's cursor position (0..1 inside it) while the mouse went down
/// on it and is held: by the rectangle it had then, so a panel made again
/// meanwhile does not end the drag.
pub fn minimap_press(events: &UiEvents, cursor: Option<Vec2>) -> Option<Vec2> {
    let [x, y, w, h] = events.picture_down?;
    let c = cursor?;
    Some(Vec2::new((c.x - x) / w, (c.y - y) / h))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UI-MENU-1.8 (b) (E-trace-026): a label's line box is centred on its
    /// y: the building window's captions at y 37 (`small_no_alias_white`,
    /// line 12) have their line top at 31, a status line at y 160
    /// (`light_system`, 16) at 152, a title at y 14 (`font_big_white`) at 6;
    /// x is the anchor's.
    // spec: UI-MENU-1.8, UI-GAME-4.1a
    #[test]
    fn labels_are_centred_on_their_y() {
        assert_eq!(label_top([64.0, 37.0], Font::Heading), [64.0, 31.0]);
        assert_eq!(label_top([40.0, 160.0], Font::Body), [40.0, 152.0]);
        assert_eq!(label_top([240.0, 14.0], Font::Big), [240.0, 6.0]);
    }
    use crate::ui::widgets::{Bar, MenuAction, Picture, SliderKind};

    fn bar_button(at: [f32; 2], action: Action, enabled: bool) -> Widget {
        Widget::BarButton {
            text: "x".into(),
            font: Font::Body,
            at,
            bar: Bar {
                size: [208.0, 32.0],
                normal: [0, 0],
                hover: [0, 0],
                pressed: [0, 0],
            },
            raise: 0.0,
            action,
            enabled,
        }
    }

    fn sizes(w: &Widget) -> Option<[f32; 2]> {
        match w {
            Widget::BarButton { bar, .. } => Some(bar.size),
            Widget::TextButton { .. } => Some([50.0, 10.0]),
            Widget::Slider { width, .. } => Some(crate::ui::widgets::SliderArt::of(*width).size()),
            Widget::Picture { size, .. } => Some([*size, *size]),
            _ => None,
        }
    }

    fn load() -> Action {
        Action::Menu(MenuAction::LoadSave)
    }

    fn back() -> Action {
        Action::Menu(MenuAction::Continue)
    }

    // spec: UI-GAME-1.4, UI-GAME-1.5, UI-GAME-1.10c
    #[test]
    fn what_takes_a_click_is_read_from_the_panels() {
        let panel = |id, at, widgets| Panel {
            id,
            at,
            widgets,
            tooltips: Vec::new(),
        };
        let panels = [
            panel(
                PanelId::Hud,
                [100.0, 100.0],
                vec![
                    bar_button([0.0, 0.0], load(), true),
                    // UI-GAME-6.5: an economy slider, 200 x 20.
                    Widget::Slider {
                        at: [0.0, 40.0],
                        width: 200.0,
                        value: 0.5,
                        kind: SliderKind::Defenders,
                        enabled: true,
                    },
                    Widget::Picture {
                        picture: Picture::Minimap,
                        at: [0.0, 60.0],
                        size: 50.0,
                    },
                ],
            ),
            // A dialog over it: higher, though earlier in no order.
            panel(
                PanelId::Menu,
                [100.0, 100.0],
                vec![
                    Widget::Fill {
                        at: [0.0, 0.0],
                        size: [500.0, 500.0],
                        rgba: [0.0; 4],
                    },
                    bar_button([100.0, 0.0], back(), false),
                    Widget::TextButton {
                        text: "row".into(),
                        font: Font::Body,
                        at: [300.0, 0.0],
                        colour: [0; 3],
                        action: back(),
                        enabled: true,
                    },
                ],
            ),
        ];
        let at = |x, y| widget_at(&panels, [x, y], &mut sizes);
        // The button of the lower panel, through the dialog's fill.
        assert_eq!(
            at(110.0, 110.0),
            Some(Hit::Button {
                action: load(),
                enabled: true
            })
        );
        // Where a disabled button lies over it, the disabled one takes it.
        assert_eq!(
            at(250.0, 110.0),
            Some(Hit::Button {
                action: back(),
                enabled: false
            })
        );
        // Rectangles exclude their far edges.
        assert_eq!(at(99.0, 110.0), None);
        assert_eq!(at(110.0, 132.0), None);
        // A text button's rectangle starts at its line box.
        assert!(matches!(at(400.0, 105.0), Some(Hit::Button { .. })));
        assert_eq!(
            at(150.0, 142.0),
            Some(Hit::Slider {
                kind: SliderKind::Defenders,
                fraction: 0.25,
                enabled: true
            })
        );
        assert_eq!(
            at(120.0, 170.0),
            Some(Hit::Picture {
                rect: [100.0, 160.0, 50.0, 50.0]
            })
        );
    }

    /// An app with `interact` and the panels of a page just opened: no node
    /// of them has a place yet, and most have no node at all.
    fn clicks_app(panels: Vec<Panel>, pointer: Vec2) -> App {
        let mut app = App::new();
        let mut assets = UiAssets::default();
        assets
            .atlases
            .insert(Atlas::Elements0, Some(Handle::default()));
        let mut window = Window::default();
        window.set_cursor_position(Some(pointer));
        app.insert_resource(assets)
            .insert_resource(UiFrame {
                panels,
                map_tooltip: None,
            })
            .init_resource::<UiEvents>()
            .init_resource::<Assets<Image>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Time>()
            .add_systems(Update, interact);
        app.world_mut().spawn(window);
        app
    }

    fn mouse(app: &mut App, f: impl FnOnce(&mut ButtonInput<MouseButton>)) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        input.clear();
        f(&mut input);
    }

    fn page() -> Vec<Panel> {
        vec![Panel {
            id: PanelId::Menu,
            at: [10.0, 10.0],
            widgets: vec![bar_button([100.0, 100.0], load(), true)],
            tooltips: Vec::new(),
        }]
    }

    // spec: UI-GAME-1.4, UI-GAME-1.10c, UI-MENU-5.2
    #[test]
    fn a_click_right_after_a_page_opens_is_not_lost() {
        // Pressed in the first frame, released in the next: the page's nodes
        // read no pointer in either.
        let mut app = clicks_app(page(), Vec2::new(120.0, 120.0));
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        {
            let events = app.world().resource::<UiEvents>();
            assert!(events.actions.is_empty(), "fires on release");
            assert!(events.over_ui, "the press is the page's, not the map's");
            assert_eq!(events.hovered, Some(load()));
        }
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        let events = app.world().resource::<UiEvents>();
        assert_eq!(events.actions, [load()]);
        assert_eq!(events.sounds, ["uiClick1"]);
        assert_eq!(events.pressed, None);
    }

    /// UI-MENU-2.1 (Q-1042 b): a list's scroll arrow steps when pressed,
    /// then again each time a 0.05 s countdown runs out while it is held
    /// (every third frame at 60 frames a second), and not on release.
    // spec: UI-MENU-2.1
    #[test]
    fn a_held_scroll_arrow_steps_at_once_and_every_twentieth_second() {
        let arrow = Action::Menu(crate::ui::widgets::MenuAction::MapScroll(1));
        let mut panels = page();
        panels[0].widgets = vec![bar_button([100.0, 100.0], arrow.clone(), true)];
        let mut app = clicks_app(panels, Vec2::new(120.0, 120.0));
        let frame = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f64(1.0 / 60.0));
            app.update();
        };
        let steps = |app: &App| app.world().resource::<UiEvents>().actions.len();
        mouse(&mut app, |m| m.press(MouseButton::Left));
        frame(&mut app);
        assert_eq!(steps(&app), 1, "at once");
        for _ in 0..6 {
            mouse(&mut app, |_| ());
            frame(&mut app);
        }
        assert_eq!(steps(&app), 3, "after three and six frames");
        mouse(&mut app, |m| m.release(MouseButton::Left));
        frame(&mut app);
        assert_eq!(steps(&app), 3, "nothing on release");
        assert!(app
            .world()
            .resource::<UiEvents>()
            .actions
            .iter()
            .all(|a| *a == arrow));
    }

    // spec: UI-GAME-1.4, UI-GAME-1.10c
    #[test]
    fn a_click_within_one_frame_of_a_new_page_counts() {
        let mut app = clicks_app(page(), Vec2::new(120.0, 120.0));
        mouse(&mut app, |m| {
            m.press(MouseButton::Left);
            m.release(MouseButton::Left);
        });
        app.update();
        assert_eq!(app.world().resource::<UiEvents>().actions, [load()]);
    }

    // spec: UI-GAME-1.4
    #[test]
    fn a_page_made_again_between_press_and_release_keeps_the_click() {
        let mut app = clicks_app(page(), Vec2::new(120.0, 120.0));
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        // Made again: the description changes, the button stays.
        let mut again = page();
        again[0].widgets.push(Widget::Fill {
            at: [0.0, 0.0],
            size: [1.0, 1.0],
            rgba: [0.0; 4],
        });
        app.world_mut().resource_mut::<UiFrame>().panels = again;
        mouse(&mut app, |_| ());
        app.update();
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        assert_eq!(app.world().resource::<UiEvents>().actions, [load()]);
    }

    // spec: UI-GAME-1.4, UI-GAME-1.5
    #[test]
    fn no_click_off_the_button_or_on_a_disabled_one_or_released_elsewhere() {
        // Pressed beside it.
        let mut app = clicks_app(page(), Vec2::new(50.0, 50.0));
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        assert!(!app.world().resource::<UiEvents>().over_ui);
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        assert!(app.world().resource::<UiEvents>().actions.is_empty());
        // Disabled.
        let mut off = page();
        off[0].widgets = vec![bar_button([100.0, 100.0], load(), false)];
        let mut app = clicks_app(off, Vec2::new(120.0, 120.0));
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        assert!(app.world().resource::<UiEvents>().over_ui);
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        assert!(app.world().resource::<UiEvents>().actions.is_empty());
        // Pressed on it, released beside it.
        let mut app = clicks_app(page(), Vec2::new(120.0, 120.0));
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        let mut windows = app.world_mut().query::<&mut Window>();
        windows
            .single_mut(app.world_mut())
            .expect("the window")
            .set_cursor_position(Some(Vec2::new(50.0, 50.0)));
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        assert!(app.world().resource::<UiEvents>().actions.is_empty());
    }

    // spec: UI-GAME-1.4
    #[test]
    fn a_slider_or_the_minimap_in_a_new_panel_takes_the_press() {
        let panels = vec![Panel {
            id: PanelId::Minimap,
            at: [0.0, 0.0],
            widgets: vec![
                // UI-GAME-6.5: an economy slider, 200 x 20.
                Widget::Slider {
                    at: [0.0, 0.0],
                    width: 200.0,
                    value: 0.5,
                    kind: SliderKind::Defenders,
                    enabled: true,
                },
                Widget::Picture {
                    picture: Picture::Minimap,
                    at: [0.0, 100.0],
                    size: 100.0,
                },
            ],
            tooltips: Vec::new(),
        }];
        let mut app = clicks_app(panels.clone(), Vec2::new(50.0, 4.0));
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        assert_eq!(
            app.world().resource::<UiEvents>().actions,
            [Action::Slider(SliderKind::Defenders, 0.25)]
        );
        let mut app = clicks_app(panels, Vec2::new(25.0, 150.0));
        app.world_mut().resource_mut::<UiAssets>().minimap = Some(Handle::default());
        mouse(&mut app, |m| m.press(MouseButton::Left));
        app.update();
        let events = app.world().resource::<UiEvents>();
        assert_eq!(
            minimap_press(events, Some(Vec2::new(25.0, 150.0))),
            Some(Vec2::new(0.25, 0.5))
        );
        // Held and dragged past its edge: still the minimap's.
        assert_eq!(
            minimap_press(events, Some(Vec2::new(150.0, 150.0))),
            Some(Vec2::new(1.5, 0.5))
        );
        mouse(&mut app, |m| m.release(MouseButton::Left));
        app.update();
        let events = app.world().resource::<UiEvents>();
        assert_eq!(minimap_press(events, Some(Vec2::new(25.0, 150.0))), None);
    }

    // spec: UI-GAME-1.7, UI-MENU-1.8
    #[test]
    fn text_is_tinted_glyphs() {
        let font = bmfont::Font::parse(
            "common lineHeight=4 base=3\nchar id=65 x=0 y=0 width=2 height=2 xoffset=0 yoffset=0 xadvance=3 page=0\n",
        );
        let data = FontData {
            font,
            page: [255u8, 255, 255, 255].repeat(4),
            width: 2,
            height: 2,
        };
        let (rgba, w, h) = draw_text(&data, "AA", [200, 100, 0]);
        assert_eq!((w, h), (14, 8));
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
        };
        assert_eq!(px(2, 2), [200, 100, 0, 255]);
        assert_eq!(px(5, 2), [200, 100, 0, 255]);
        assert_eq!(px(4, 2)[3], 0);
    }

    // spec: UI-MENU-1.8, UI-SETUP-1.8a
    #[test]
    fn line_box_top_left_on_the_anchor() {
        // T14: small_no_alias_white's "1", an 8 x 13 cell at xoffset -3,
        // yoffset 0, its ink in the cell's columns 3 .. 4 and rows 3 .. 9,
        // anchored at p = (172, 189): ink columns 172 .. 173, rows 192 .. 198.
        let mut page = vec![0u8; 8 * 13 * 4];
        for y in 3..=9 {
            for x in 3..=4 {
                let i = (y * 8 + x) * 4;
                page[i..i + 4].copy_from_slice(&[255; 4]);
            }
        }
        let font = bmfont::Font::parse(
            "common lineHeight=12 base=10\nchar id=49 x=0 y=0 width=8 height=13 xoffset=-3 yoffset=0 xadvance=5 page=0\n",
        );
        let data = FontData {
            font,
            page,
            width: 8,
            height: 13,
        };
        let (rgba, w, h) = draw_text(&data, "1", [255, 255, 255]);
        let origin = text_origin([172.0, 189.0], 5.0, Align::TopLeft);
        let ink: Vec<[i32; 2]> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| rgba[((y * w + x) * 4 + 3) as usize] > 0)
            .map(|(x, y)| [origin[0] as i32 + x as i32, origin[1] as i32 + y as i32])
            .collect();
        let range = |k: usize| {
            let v = ink.iter().map(|p| p[k]);
            (v.clone().min().unwrap(), v.max().unwrap())
        };
        assert_eq!(range(0), (172, 173));
        assert_eq!(range(1), (192, 198));
        // 1.8b: centring moves the line left by half the run, right
        // alignment by all of it; neither moves it up or down.
        let pad = TEXT_PAD as f32;
        assert_eq!(
            text_origin([100.0, 50.0], 20.0, Align::TopCentre),
            [90.0 - pad, 50.0 - pad]
        );
        assert_eq!(
            text_origin([100.0, 50.0], 20.0, Align::Centre),
            [90.0 - pad, 50.0 - pad]
        );
        assert_eq!(
            text_origin([100.0, 50.0], 20.0, Align::TopRight),
            [80.0 - pad, 50.0 - pad]
        );
    }
}
