//! The original's films, converted from its Bink files with FFmpeg at
//! install (`docs/video.md`, `scripts/install-local.sh`): per film a
//! folder `<name>/` with `frames.mjpeg` (the frames as JPEG images one
//! after the other), `audio.wav` when the film has sound, and `info.txt`
//! (`fps=`, `width=`, `height=`, `frames=`). Shown by `UI-menus.md`
//! UI-MENU-0 (the intro at start-up) and 1.1a (the main menu's town).

use std::ops::Range;
use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// A converted film's `info.txt`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilmInfo {
    pub fps: f32,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
}

impl FilmInfo {
    /// `key=value` lines; `fps` may be a fraction (`25/1`).
    pub fn parse(text: &str) -> Option<Self> {
        let get = |key: &str| {
            text.lines()
                .filter_map(|l| l.split_once('='))
                .find(|(k, _)| k.trim() == key)
                .map(|(_, v)| v.trim())
        };
        let fps = match get("fps")?.split_once('/') {
            Some((n, d)) => n.parse::<f32>().ok()? / d.parse::<f32>().ok()?.max(1e-6),
            None => get("fps")?.parse().ok()?,
        };
        Some(Self {
            fps,
            width: get("width")?.parse().ok()?,
            height: get("height")?.parse().ok()?,
            frames: get("frames")?.parse().ok()?,
        })
    }
}

/// The byte ranges of the JPEG images of a Motion-JPEG stream: each runs
/// from a start-of-image marker (FF D8 FF) to its end-of-image marker
/// (FF D9). Inside the coded data an FF byte is always followed by 00 or a
/// restart marker, so FF D9 ends the image.
pub fn jpeg_frames(data: &[u8]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        if data[i] == 0xFF && data[i + 1] == 0xD8 && data[i + 2] == 0xFF {
            let start = i;
            let mut j = i + 2;
            while j + 1 < data.len() && !(data[j] == 0xFF && data[j + 1] == 0xD9) {
                j += 1;
            }
            if j + 1 >= data.len() {
                break;
            }
            out.push(start..j + 2);
            i = j + 2;
        } else {
            i += 1;
        }
    }
    out
}

/// The frame shown `seconds` after the start: frame f from f / fps
/// (UI-MENU-0.3); a looping film starts again after its last frame
/// (1.1a), another one ends (`None`).
pub fn frame_at(seconds: f32, fps: f32, frames: u32, looping: bool) -> Option<u32> {
    if frames == 0 {
        return None;
    }
    let f = (seconds.max(0.0) * fps).floor() as u64;
    if looping {
        Some((f % u64::from(frames)) as u32)
    } else {
        (f < u64::from(frames)).then_some(f as u32)
    }
}

/// UI-MENU-0.3: a film scaled to fit the screen keeping its shape,
/// centred: (x, y, width, height).
pub fn fit(film: [f32; 2], screen: [f32; 2]) -> [f32; 4] {
    let scale = (screen[0] / film[0]).min(screen[1] / film[1]);
    let (w, h) = (film[0] * scale, film[1] * scale);
    [(screen[0] - w) / 2.0, (screen[1] - h) / 2.0, w, h]
}

/// UI-MENU-0.3: the subtitle line's box, 80 % of the film's width from 10
/// % of it, and the height of its middle: in the bar under the film when
/// that is taller than the text, 16 px above the screen's bottom when
/// there is no bar, else on the film's bottom edge.
pub fn subtitle_box(film: [f32; 4], screen: [f32; 2], text_height: f32) -> ([f32; 2], f32) {
    let [x, y, w, h] = film;
    let bottom = y + h;
    let bar = screen[1] - bottom;
    let middle = if bar < 0.5 {
        screen[1] - 16.0 - text_height / 2.0
    } else if bar > text_height {
        bottom + bar / 2.0
    } else {
        bottom
    };
    ([x + 0.1 * w, 0.8 * w], middle)
}

/// Where the converted films are: `S2_VIDEO_DIR`, else `video/` beside
/// the program.
pub fn films_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("S2_VIDEO_DIR") {
        return Some(PathBuf::from(dir));
    }
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("video"))
}

/// [`jpeg_frames`] of a file, read in pieces (an intro of some hundred
/// megabytes is not held in memory).
fn index_file(path: &Path) -> Option<Vec<Range<usize>>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut out = Vec::new();
    let mut buf = vec![0u8; 1 << 20];
    // The bytes since the last image's end, from file offset `base`.
    let mut carry: Vec<u8> = Vec::new();
    let mut base = 0usize;
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        carry.extend_from_slice(&buf[..n]);
        let found = jpeg_frames(&carry);
        let Some(last) = found.last().map(|r| r.end) else {
            continue;
        };
        out.extend(found.into_iter().map(|r| base + r.start..base + r.end));
        carry.drain(..last);
        base += last;
    }
    Some(out)
}

/// A converted film: its frames are read from the file as they are shown.
pub struct Film {
    pub info: FilmInfo,
    path: PathBuf,
    frames: Vec<Range<usize>>,
    /// `audio.wav`, when the film has sound.
    pub audio: Option<PathBuf>,
}

impl Film {
    /// The film `name` (`title`, `intro`) of `dir`, if it was converted.
    pub fn load(dir: &Path, name: &str) -> Option<Self> {
        let dir = dir.join(name);
        let info = FilmInfo::parse(&std::fs::read_to_string(dir.join("info.txt")).ok()?)?;
        let path = dir.join("frames.mjpeg");
        let frames = index_file(&path)?;
        if frames.is_empty() {
            return None;
        }
        let audio = Some(dir.join("audio.wav")).filter(|p| p.is_file());
        Some(Self {
            info,
            path,
            frames,
            audio,
        })
    }

    pub fn frame_count(&self) -> u32 {
        self.frames.len() as u32
    }

    /// Frame `i` as RGBA bytes, `width` x `height`.
    pub fn decode(&self, i: u32) -> Option<(u32, u32, Vec<u8>)> {
        use std::io::{Read, Seek, SeekFrom};
        use zune_jpeg::zune_core::colorspace::ColorSpace;
        use zune_jpeg::zune_core::options::DecoderOptions;
        let range = self.frames.get(i as usize)?.clone();
        let mut file = std::fs::File::open(&self.path).ok()?;
        file.seek(SeekFrom::Start(range.start as u64)).ok()?;
        let mut data = vec![0u8; range.len()];
        file.read_exact(&mut data).ok()?;
        let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
        let mut decoder = zune_jpeg::JpegDecoder::new_with_options(&data[..], options);
        let pixels = decoder.decode().ok()?;
        let (w, h) = decoder.dimensions()?;
        Some((w as u32, h as u32, pixels))
    }
}

/// The films' places in the interface (`Atlas::Film`).
pub const FILM_TITLE: u8 = 0;
pub const FILM_INTRO: u8 = 1;

/// A film being shown: its picture and when it started (real seconds).
struct Playing {
    film: Film,
    image: Handle<Image>,
    since: Option<f32>,
    shown: Option<u32>,
    sound: Option<Entity>,
}

/// The films the interface shows, loaded when first needed.
#[derive(Resource, Default)]
pub struct Films {
    tried: [bool; 2],
    playing: [Option<Playing>; 2],
}

impl Films {
    fn get(&mut self, which: u8, images: &mut Assets<Image>) -> Option<&mut Playing> {
        let i = usize::from(which);
        if !self.tried[i] {
            self.tried[i] = true;
            let name = if which == FILM_TITLE {
                "title"
            } else {
                "intro"
            };
            self.playing[i] = films_dir()
                .and_then(|dir| Film::load(&dir, name))
                .map(|film| {
                    let image = images.add(Image::new_fill(
                        bevy::render::render_resource::Extent3d {
                            width: film.info.width.max(1),
                            height: film.info.height.max(1),
                            depth_or_array_layers: 1,
                        },
                        bevy::render::render_resource::TextureDimension::D2,
                        &[0, 0, 0, 255],
                        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                        bevy::asset::RenderAssetUsages::default(),
                    ));
                    Playing {
                        film,
                        image,
                        since: None,
                        shown: None,
                        sound: None,
                    }
                });
            if self.playing[i].is_none() {
                info!("film {name} not converted (scripts/convert-films.sh)");
            }
        }
        self.playing[i].as_mut()
    }
}

/// Show frame `frame` of a film in its picture.
fn show(p: &mut Playing, frame: u32, images: &mut Assets<Image>) {
    if p.shown == Some(frame) {
        return;
    }
    p.shown = Some(frame);
    let Some((w, h, rgba)) = p.film.decode(frame) else {
        return;
    };
    if let Some(mut image) = images.get_mut(&p.image) {
        if image.width() == w && image.height() == h {
            image.data = Some(rgba);
        }
    }
}

/// UI-MENU-1.1a: the town film loops on the main menu's left sheet.
/// UI-MENU-0.1, 0.3: the intro at start-up, full screen with its sound and
/// subtitles, the pointer hidden; a key or a left click ends it.
#[allow(clippy::too_many_arguments)]
pub fn play_films(
    time: Res<Time>,
    viewer: Res<crate::game::Viewer>,
    mut films: ResMut<Films>,
    mut menu: ResMut<crate::menus::menu::MenuState>,
    mut assets: ResMut<crate::ui::render::UiAssets>,
    mut images: ResMut<Assets<Image>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor: Single<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
    mut out: crate::audio::output::Out,
) {
    use crate::menus::menu::Screen;
    let now = time.elapsed_secs();
    // The town.
    let on_main = menu.screen == Screen::Main;
    if let Some(p) = films.get(FILM_TITLE, &mut images) {
        assets.films.insert(FILM_TITLE, p.image.clone());
        if on_main {
            let since = *p.since.get_or_insert(now);
            let info = p.film.info;
            if let Some(f) = frame_at(now - since, info.fps, p.film.frame_count(), true) {
                show(p, f, &mut images);
            }
        } else {
            p.since = None;
        }
    }
    // The intro.
    if menu.screen != Screen::Intro {
        return;
    }
    // UI-MENU-0.3: its subtitles, `data/video/Intro.xml`.
    if menu.subtitles.cues.is_empty() {
        if let Some(text) = viewer
            .game_root
            .as_ref()
            .and_then(|root| crate::extend::find(root, "data/video/Intro.xml").ok())
            .and_then(|path| crate::game::read_game_file(&path).ok())
        {
            menu.subtitles =
                s2_assets::subtitles::Subtitles::parse(&String::from_utf8_lossy(&text));
        }
    }
    let skip = keys.get_just_pressed().next().is_some() || mouse.just_pressed(MouseButton::Left);
    let mut ended = skip;
    match films.get(FILM_INTRO, &mut images) {
        None => ended = true,
        Some(p) => {
            assets.films.insert(FILM_INTRO, p.image.clone());
            if p.since.is_none() {
                p.since = Some(now);
                cursor.visible = false;
                if let Some(bytes) = p.film.audio.as_ref().and_then(|a| std::fs::read(a).ok()) {
                    let volume = menu.settings.volumes[0].clamp(0.0, 1.0);
                    p.sound = out.play("intro.wav", bytes.into(), volume, false, None);
                }
            }
            let info = p.film.info;
            let since = p.since.unwrap_or(now);
            match frame_at(now - since, info.fps, p.film.frame_count(), false) {
                Some(f) if !ended => {
                    show(p, f, &mut images);
                    menu.intro = Some((f, [info.width as f32, info.height as f32]));
                }
                _ => ended = true,
            }
        }
    }
    if ended {
        if let Some(p) = films.playing[usize::from(FILM_INTRO)].take() {
            if let Some(e) = p.sound {
                out.stop(e);
            }
        }
        cursor.visible = true;
        menu.intro = None;
        menu.screen = crate::menus::menu::after_intro();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // UI-MENU-1.1a, 0.1: the converted title film (1308 frames, 616 x
    // 665) and intro (1024 x 576) decode. Skipped without S2_VIDEO_DIR.
    #[test]
    fn converted_films_decode() {
        let Some(dir) = std::env::var_os("S2_VIDEO_DIR").map(PathBuf::from) else {
            eprintln!("skipped: S2_VIDEO_DIR is not set");
            return;
        };
        let title = Film::load(&dir, "title").expect("the title film");
        assert_eq!(title.frame_count(), 1308);
        assert_eq!(title.frame_count(), title.info.frames);
        let (w, h, rgba) = title.decode(654).expect("a frame");
        assert_eq!((w, h, rgba.len()), (616, 665, 616 * 665 * 4));
        let intro = Film::load(&dir, "intro").expect("the intro");
        assert_eq!(intro.frame_count(), intro.info.frames);
        let (w, h, _) = intro
            .decode(intro.frame_count() - 1)
            .expect("the last frame");
        assert_eq!((w, h), (1024, 576));
        assert!(intro.audio.is_some());
    }

    // spec: none (the converted films' info file, docs/video.md)
    #[test]
    fn info_file() {
        let info = FilmInfo::parse("fps=25/1\nwidth=616\nheight=665\nframes=1308\n").unwrap();
        assert_eq!(
            info,
            FilmInfo {
                fps: 25.0,
                width: 616,
                height: 665,
                frames: 1308
            }
        );
    }

    // spec: none (the converted films' frames, docs/video.md)
    #[test]
    fn frames_of_a_stream() {
        let a = [0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 0xFF, 0x00, 3, 0xFF, 0xD9];
        let b = [0xFF, 0xD8, 0xFF, 0xDB, 0xFF, 0xD0, 9, 0xFF, 0xD9];
        let data: Vec<u8> = a.iter().chain(&b).copied().collect();
        assert_eq!(jpeg_frames(&data), [0..11, 11..20]);
        // A cut-off last image is left out.
        assert_eq!(jpeg_frames(&data[..15]), vec![0..11]);
    }

    // UI-MENU T8 (1.1a): the title film of 1308 frames at 25 fps starts
    // again with frame 0 at 52.32 s.
    // spec: UI-MENU-1.1a
    #[test]
    fn title_loops() {
        assert_eq!(frame_at(52.31, 25.0, 1308, true), Some(1307));
        assert_eq!(frame_at(52.32, 25.0, 1308, true), Some(0));
        // UI-MENU-0.3: the intro's frame 33 at 1.32 s; past the end, none.
        assert_eq!(frame_at(1.32, 25.0, 3365, false), Some(33));
        assert_eq!(frame_at(134.6, 25.0, 3365, false), None);
    }

    // UI-MENU T14: the 1024 x 576 intro fills a 1920 x 1080 screen; the
    // line's bottom is 16 px above the screen's bottom.
    // spec: UI-MENU-0.3 (T14)
    #[test]
    fn t14_subtitle_without_bars() {
        let film = fit([1024.0, 576.0], [1920.0, 1080.0]);
        assert_eq!(film, [0.0, 0.0, 1920.0, 1080.0]);
        let ([x, w], middle) = subtitle_box(film, [1920.0, 1080.0], 30.0);
        assert_eq!((x, w), (192.0, 1536.0));
        assert_eq!(middle + 15.0, 1080.0 - 16.0);
    }

    // UI-MENU T15: on 1024 x 768 the intro is centred with 96 px bars;
    // the line is centred in the lower bar.
    // spec: UI-MENU-0.3 (T15)
    #[test]
    fn t15_subtitle_in_the_bar() {
        let film = fit([1024.0, 576.0], [1024.0, 768.0]);
        assert_eq!(film, [0.0, 96.0, 1024.0, 576.0]);
        let (_, middle) = subtitle_box(film, [1024.0, 768.0], 30.0);
        assert_eq!(middle, 672.0 + 48.0);
        // A bar lower than the text: on the film's bottom edge.
        let (_, middle) = subtitle_box(film, [1024.0, 768.0], 120.0);
        assert_eq!(middle, 672.0);
    }
}
