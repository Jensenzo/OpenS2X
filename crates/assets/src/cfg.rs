//! The profile settings file `users\<profile>.cfg` (`specs/SET-settings.md`):
//! a `UserObject` (version 5) and a `SceneSettings` (version 1) in the
//! marker format of FMT-S2M-1.2, plain (not rc00-wrapped). Every field is
//! kept, the ones whose meaning is open included, so a file read and
//! written again is unchanged.

use crate::rc00;

/// SET-2: one map's state (`UserMapState`, version 0; CAM-3.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapState {
    pub name: String,
    pub completed: bool,
    pub acknowledged: u32,
}

/// SET-2: the `UserObject` fields in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct UserObject {
    pub profile: String,
    pub autosave_name: String,
    /// Field 3: forced to 1.0 on load.
    pub unused: f32,
    /// Seconds of game time, 0 = off (SET-2.1).
    pub autosave_interval: u32,
    /// Fields 5 to 9: input options A, B, scroll speed, C, D.
    pub input_a: bool,
    pub input_b: bool,
    pub scroll_speed: f32,
    pub input_c: bool,
    pub input_d: bool,
    pub maps: Vec<MapState>,
    /// Master, music, UI, sound, ambient (SND-5.1).
    pub volumes: [f32; 5],
    pub player_name: String,
    /// Field 17: the Infowindow, 0 = On, 1 = Off (SET-2).
    pub info_window: u32,
    /// Fields 18 to 22, meaning open (Q-100 c).
    pub unknown_strings: [String; 2],
    pub unknown_bools: [bool; 2],
    /// SET-2 field 22: the autosave on leaving (UI-MENU-3.3): 0 off.
    pub autosave_on_leave: u32,
}

impl Default for UserObject {
    /// SET-3: the defaults without a file.
    fn default() -> Self {
        Self {
            profile: "default".into(),
            autosave_name: String::new(),
            unused: 1.0,
            autosave_interval: 0,
            input_a: true,
            input_b: false,
            // GFX-VIEW-3.2: the camera speed's default.
            scroll_speed: 0.25,
            input_c: false,
            input_d: false,
            maps: Vec::new(),
            volumes: [1.0, 0.3, 1.0, 1.0, 1.0],
            player_name: "default".into(),
            info_window: 0,
            unknown_strings: [String::new(), String::new()],
            unknown_bools: [false, false],
            autosave_on_leave: 0,
        }
    }
}

/// SET-4: the `SceneSettings` fields, each kept as its four bytes; which
/// are booleans and floats is in [`SCENE_KINDS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSettings {
    pub words: [u32; 19],
}

/// SET-4: the type of each scene field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    U32,
    Bool,
    F32,
}

pub const SCENE_KINDS: [Kind; 19] = {
    use Kind::*;
    [
        U32, Bool, F32, Bool, U32, U32, F32, Bool, Bool, Bool, F32, F32, Bool, F32, F32, U32, F32,
        F32, Bool,
    ]
};

impl SceneSettings {
    /// SET-4: the screen width and height (fields 5 and 6).
    pub fn resolution(&self) -> (u32, u32) {
        (self.words[4], self.words[5])
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.words[4] = width;
        self.words[5] = height;
    }

    /// SET-4a, GFX-VIEW-2.5 e: the shadows switch, the bool written 10th
    /// (the third of the three bools after `TERRAIN QUALITY`).
    pub fn shadows(&self) -> bool {
        self.words[9] != 0
    }

    /// SET-4, GFX-VIEW-2.5 d: `SHADOW QUALITY` 0..1, the 11th field (the
    /// f32 after the three bools).
    pub fn shadow_quality(&self) -> f32 {
        f32::from_bits(self.words[10])
    }

    /// SET-4, GFX-WATER-2.5: the shader detail, the u32 at +0x3c (field
    /// 16): 1 off, 2 low, 3 high.
    pub fn shader_detail(&self) -> u32 {
        self.words[15]
    }

    /// SET-4, GFX-WATER-2.6: the mirror switch, the bool written 13th (the
    /// one after the two f32 that follow the three bools).
    pub fn mirror(&self) -> bool {
        self.words[12] != 0
    }

    /// SET-4, GFX-WATER-2.6: the mirror quality 0..1, the 14th field.
    pub fn mirror_quality(&self) -> f32 {
        f32::from_bits(self.words[13])
    }

    /// SET-4, GFX-WATER-2.6b: `MIRROR DETAILS` 0..1, the f32 written 15th.
    pub fn mirror_details(&self) -> f32 {
        f32::from_bits(self.words[14])
    }
}

/// SET-4: the graphic quality presets (field 1).
pub const PRESET_VERY_LOW: u32 = 0;
pub const PRESET_LOW: u32 = 1;
pub const PRESET_MEDIUM: u32 = 2;
pub const PRESET_HIGH: u32 = 3;
pub const PRESET_VERY_HIGH: u32 = 4;
pub const PRESET_CUSTOM: u32 = 5;

/// SET-4a: the scene fields by their number in file order (1-based).
pub mod field {
    pub const PRESET: usize = 1;
    pub const TEXTURE_QUALITY: usize = 3;
    pub const TERRAIN_QUALITY: usize = 7;
    pub const SHADOWS: usize = 10;
    pub const SHADOW_QUALITY: usize = 11;
    pub const SHADOW_DETAIL: usize = 12;
    pub const MIRROR: usize = 13;
    pub const MIRROR_QUALITY: usize = 14;
    pub const MIRROR_DETAILS: usize = 15;
    pub const SHADER_DETAIL: usize = 16;
    pub const OBJECT_DETAILS: usize = 17;
    pub const DRAW_DISTANCE: usize = 18;
}

impl SceneSettings {
    /// SET-4: the graphic quality preset (field 1).
    pub fn preset(&self) -> u32 {
        self.words[field::PRESET - 1]
    }

    /// SET-4a: a slider field's value (`field` 1-based).
    pub fn slider(&self, field: usize) -> f32 {
        f32::from_bits(self.words[field - 1])
    }

    /// SET-4a: every slider value is kept clamped to 0..1.
    fn set_slider(&mut self, field: usize, value: f32) {
        self.words[field - 1] = value.clamp(0.0, 1.0).to_bits();
    }

    /// SET-4a: the two switches every preset change sets: shadows on while
    /// `SHADOW DETAIL` is above 1/6, the mirror on while `MIRROR QUALITY` is
    /// above 0.
    fn recompute_switches(&mut self) {
        let shadows = self.slider(field::SHADOW_DETAIL) > 1.0 / 6.0;
        let mirror = self.slider(field::MIRROR_QUALITY) > 0.0;
        self.words[field::SHADOWS - 1] = u32::from(shadows);
        self.words[field::MIRROR - 1] = u32::from(mirror);
    }

    /// SET-4a: the preset `preset` applied (when the player changes the
    /// preset, never on load): its slider values and shader detail (Custom
    /// sets none), then the two switches recomputed.
    pub fn apply_preset(&mut self, preset: u32) {
        use field::*;
        let third = 1.0 / 3.0;
        // Texture, terrain, shadow quality, shadow detail, mirror quality,
        // mirror details, object details, draw distance; shader detail.
        let values = match preset {
            PRESET_VERY_LOW => Some(([0.0; 8], 1)),
            PRESET_LOW => Some(([third, 0.25, 0.0, 1.0 / 6.0, 0.25, 0.25, third, 0.25], 2)),
            PRESET_MEDIUM => Some((
                [2.0 * third, 0.5, 0.5, third, 0.5, 0.5, 2.0 * third, 0.5],
                3,
            )),
            PRESET_HIGH => Some(([1.0, 0.75, 0.75, 2.0 * third, 0.75, 0.75, 1.0, 0.75], 3)),
            PRESET_VERY_HIGH => Some(([1.0; 8], 3)),
            _ => None,
        };
        self.words[PRESET - 1] = preset;
        if let Some((sliders, shader)) = values {
            let fields = [
                TEXTURE_QUALITY,
                TERRAIN_QUALITY,
                SHADOW_QUALITY,
                SHADOW_DETAIL,
                MIRROR_QUALITY,
                MIRROR_DETAILS,
                OBJECT_DETAILS,
                DRAW_DISTANCE,
            ];
            for (f, v) in fields.into_iter().zip(sliders) {
                self.set_slider(f, v);
            }
            self.words[SHADER_DETAIL - 1] = shader;
        }
        self.recompute_switches();
    }

    /// SET-4a: a slider of the engine page moved to `value`. The first move
    /// away from a preset switches it to Custom (which recomputes the two
    /// switches once); `SHADOW QUALITY` sets the shadows switch to (value
    /// above 0), `MIRROR QUALITY` and `MIRROR DETAILS` set the mirror switch
    /// to (`MIRROR QUALITY` above 0); `SHADOW DETAIL` and the others set
    /// their field only.
    pub fn move_slider(&mut self, field: usize, value: f32) {
        if self.preset() != PRESET_CUSTOM {
            self.apply_preset(PRESET_CUSTOM);
        }
        match field {
            field::SHADOW_QUALITY => {
                self.words[field::SHADOWS - 1] = u32::from(value > 0.0);
                self.set_slider(field, value);
            }
            field::MIRROR_QUALITY | field::MIRROR_DETAILS => {
                self.set_slider(field, value);
                let on = self.slider(field::MIRROR_QUALITY) > 0.0;
                self.words[field::MIRROR - 1] = u32::from(on);
            }
            _ => self.set_slider(field, value),
        }
    }

    /// SET-4, SET-5.3: the `SHADERDETAIL` choice (1 off, 2 low, 3 high).
    pub fn set_shader_detail(&mut self, detail: u32) {
        self.words[field::SHADER_DETAIL - 1] = detail;
    }
}

impl Default for SceneSettings {
    /// The scene block's defaults are not in the spec (Q-188): preset 4
    /// (VeryHigh) at 1024 x 768 with shader detail 3 (high), shadow
    /// quality 1.0 and the shadows switch on (the sample profile's VeryHigh
    /// values, SET-4; SET-4a: VeryHigh's switch is on), every other field
    /// zero.
    fn default() -> Self {
        let mut words = [0; 19];
        words[0] = 4;
        words[4] = 1024;
        words[5] = 768;
        words[9] = 1;
        words[10] = 1.0f32.to_bits();
        words[15] = 3;
        Self { words }
    }
}

/// A whole settings file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub user: UserObject,
    pub scene: SceneSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfgError {
    UnexpectedEnd(usize),
    WrongSection {
        offset: usize,
        expected: &'static str,
    },
    UnsupportedVersion {
        section: &'static str,
        version: u32,
    },
    InvalidBool {
        offset: usize,
        value: u32,
    },
    TrailingBytes(usize),
}

impl std::fmt::Display for CfgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEnd(at) => write!(f, "file ends at {at:#x}"),
            Self::WrongSection { offset, expected } => {
                write!(f, "no {expected} marker at {offset:#x}")
            }
            Self::UnsupportedVersion { section, version } => {
                write!(f, "{section} version {version} is not supported")
            }
            Self::InvalidBool { offset, value } => write!(f, "bool {value} at {offset:#x}"),
            Self::TrailingBytes(n) => write!(f, "{n} bytes after the last object"),
        }
    }
}

impl std::error::Error for CfgError {}

const USER: &str = "UserObject";
const MAP_STATE: &str = "UserMapState";
const SCENE: &str = "SceneSettings";

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], CfgError> {
        if self.data.len() - self.pos < n {
            return Err(CfgError::UnexpectedEnd(self.data.len()));
        }
        let b = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(b)
    }

    fn u32(&mut self) -> Result<u32, CfgError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn f32(&mut self) -> Result<f32, CfgError> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn bool(&mut self) -> Result<bool, CfgError> {
        let offset = self.pos;
        match self.u32()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(CfgError::InvalidBool { offset, value }),
        }
    }

    /// FMT-S2M-1.1: Latin-1 bytes with a length prefix.
    fn string(&mut self) -> Result<String, CfgError> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.iter().map(|&b| char::from(b)).collect())
    }

    /// FMT-S2M-1.2
    fn marker(&mut self, section: &'static str, version: u32) -> Result<(), CfgError> {
        let offset = self.pos;
        let v = self.u32()?;
        let (crc, len) = (self.u32()?, self.u32()?);
        if crc != rc00::crc32(section.as_bytes()) || len as usize != section.len() {
            return Err(CfgError::WrongSection {
                offset,
                expected: section,
            });
        }
        if v != version {
            return Err(CfgError::UnsupportedVersion {
                section,
                version: v,
            });
        }
        Ok(())
    }
}

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }

    fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    fn bool(&mut self, v: bool) {
        self.u32(u32::from(v));
    }

    fn string(&mut self, s: &str) {
        let bytes: Vec<u8> = s
            .chars()
            .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
            .collect();
        self.u32(bytes.len() as u32);
        self.0.extend(bytes);
    }

    fn marker(&mut self, section: &str, version: u32) {
        self.u32(version);
        self.u32(rc00::crc32(section.as_bytes()));
        self.u32(section.len() as u32);
    }
}

/// SET-1.3: read a settings file.
pub fn parse(data: &[u8]) -> Result<Settings, CfgError> {
    let mut r = Reader { data, pos: 0 };
    r.marker(USER, 5)?;
    let mut u = UserObject {
        profile: r.string()?,
        autosave_name: r.string()?,
        unused: r.f32()?,
        autosave_interval: r.u32()?,
        input_a: r.bool()?,
        input_b: r.bool()?,
        scroll_speed: r.f32()?,
        input_c: r.bool()?,
        input_d: r.bool()?,
        ..UserObject::default()
    };
    let n = r.u32()?;
    for _ in 0..n {
        r.marker(MAP_STATE, 0)?;
        u.maps.push(MapState {
            name: r.string()?,
            completed: r.bool()?,
            acknowledged: r.u32()?,
        });
    }
    for v in &mut u.volumes {
        *v = r.f32()?;
    }
    u.player_name = r.string()?;
    u.info_window = r.u32()?;
    u.unknown_strings = [r.string()?, r.string()?];
    u.unknown_bools = [r.bool()?, r.bool()?];
    u.autosave_on_leave = r.u32()?;
    r.marker(SCENE, 1)?;
    let mut scene = SceneSettings { words: [0; 19] };
    for (word, kind) in scene.words.iter_mut().zip(SCENE_KINDS) {
        *word = match kind {
            Kind::Bool => u32::from(r.bool()?),
            Kind::U32 | Kind::F32 => r.u32()?,
        };
    }
    if r.pos != data.len() {
        return Err(CfgError::TrailingBytes(data.len() - r.pos));
    }
    Ok(Settings { user: u, scene })
}

/// SET-1.3: the bytes of a settings file.
pub fn write(s: &Settings) -> Vec<u8> {
    let mut w = Writer::default();
    let u = &s.user;
    w.marker(USER, 5);
    w.string(&u.profile);
    w.string(&u.autosave_name);
    w.f32(u.unused);
    w.u32(u.autosave_interval);
    w.bool(u.input_a);
    w.bool(u.input_b);
    w.f32(u.scroll_speed);
    w.bool(u.input_c);
    w.bool(u.input_d);
    w.u32(u.maps.len() as u32);
    for m in &u.maps {
        w.marker(MAP_STATE, 0);
        w.string(&m.name);
        w.bool(m.completed);
        w.u32(m.acknowledged);
    }
    for &v in &u.volumes {
        w.f32(v);
    }
    w.string(&u.player_name);
    w.u32(u.info_window);
    for t in &u.unknown_strings {
        w.string(t);
    }
    for &b in &u.unknown_bools {
        w.bool(b);
    }
    w.u32(u.autosave_on_leave);
    w.marker(SCENE, 1);
    for word in s.scene.words {
        w.u32(word);
    }
    w.0
}

impl UserObject {
    /// CAM-3.5: the state of a map, if it has one.
    pub fn map(&self, name: &str) -> Option<&MapState> {
        self.maps.iter().find(|m| m.name == name)
    }

    /// SET-2.2: the state of a map, created on demand.
    pub fn map_mut(&mut self, name: &str) -> &mut MapState {
        let i = match self.maps.iter().position(|m| m.name == name) {
            Some(i) => i,
            None => {
                self.maps.push(MapState {
                    name: name.to_owned(),
                    completed: false,
                    acknowledged: 0,
                });
                self.maps.len() - 1
            }
        };
        &mut self.maps[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file laid out like SET T1's observed one (built here, not copied).
    fn observed() -> Settings {
        let mut s = Settings::default();
        s.user.profile = "default01".into();
        s.user.autosave_name = "Autosave".into();
        s.user.autosave_interval = 600;
        s.user.scroll_speed = 0.25;
        s.user.input_c = true;
        s.user.input_d = true;
        s.user.maps = vec![
            MapState {
                name: "MP_2P_Storm_Coast".into(),
                completed: false,
                acknowledged: 2,
            },
            MapState {
                name: String::new(),
                completed: false,
                acknowledged: 2,
            },
        ];
        s.user.unknown_bools = [true, false];
        s
    }

    // spec: SET-1.3 (T1)
    #[test]
    fn t1_layout_and_round_trip() {
        // SET T1: 277 bytes with two map states; read back unchanged.
        let s = observed();
        let bytes = write(&s);
        assert_eq!(bytes.len(), 0x115);
        assert_eq!(parse(&bytes).unwrap(), s);
        assert_eq!(&bytes[4..8], &rc00::crc32(b"UserObject").to_le_bytes());
        assert_eq!(parse(&bytes).unwrap().scene.resolution(), (1024, 768));
    }

    // spec: SET-4, GFX-WATER-2.6, GFX-WATER-2.6b (T16: the sample file's mirror switch 1, quality 1.0)
    #[test]
    fn mirror_switch_and_quality_are_the_13th_and_14th_fields() {
        let mut s = SceneSettings::default();
        assert_eq!(SCENE_KINDS[12], Kind::Bool);
        assert_eq!(SCENE_KINDS[13], Kind::F32);
        s.words[12] = 1;
        s.words[13] = 1.0f32.to_bits();
        assert!(s.mirror());
        assert_eq!(s.mirror_quality(), 1.0);
        s.words[12] = 0;
        assert!(!s.mirror());
        // GFX-WATER-2.6b: `MIRROR DETAILS`, the f32 written 15th.
        assert_eq!(SCENE_KINDS[14], Kind::F32);
        s.words[14] = 1.0f32.to_bits();
        assert_eq!(s.mirror_details(), 1.0);
    }

    // SET-4a: the shadows switch is the bool written 10th, apart from
    // `SHADOW QUALITY` (11th); the default has it on as VeryHigh does.
    // spec: SET-4a, GFX-VIEW-2.5
    #[test]
    fn the_shadows_switch_is_the_10th_field() {
        let mut s = SceneSettings::default();
        assert_eq!(SCENE_KINDS[9], Kind::Bool);
        assert!(s.shadows());
        s.words[9] = 0;
        assert!(!s.shadows());
        assert_eq!(s.shadow_quality(), 1.0);
    }

    // SET-4a, T5: preset Low, then `SHADOW QUALITY` to 0.25, then to 0;
    // preset Medium.
    // spec: SET-4a (T5)
    #[test]
    fn t5_presets_and_the_shadow_slider() {
        let mut s = SceneSettings::default();
        s.apply_preset(PRESET_LOW);
        assert!(!s.shadows(), "detail 1/6 is not above 1/6");
        assert_eq!(s.shadow_quality(), 0.0);
        assert_eq!(s.shader_detail(), 2);
        assert!(s.mirror());
        s.move_slider(field::SHADOW_QUALITY, 0.25);
        assert_eq!(s.preset(), PRESET_CUSTOM);
        assert!(s.shadows());
        assert_eq!(s.shadow_quality(), 0.25);
        s.move_slider(field::SHADOW_QUALITY, 0.0);
        assert!(!s.shadows());
        assert_eq!(s.shadow_quality(), 0.0);
        s.apply_preset(PRESET_MEDIUM);
        assert!(s.shadows() && s.mirror());
        assert_eq!(s.shadow_quality(), 0.5);
        assert_eq!(s.preset(), PRESET_MEDIUM);
    }

    // SET-4a: the presets' values; the mirror switch follows `MIRROR
    // QUALITY`; `SHADOW DETAIL` moves only its field; values clamp to 0..1.
    // spec: SET-4a
    #[test]
    fn the_presets_and_sliders() {
        let mut s = SceneSettings::default();
        s.apply_preset(PRESET_VERY_LOW);
        for f in [3, 7, 11, 12, 14, 15, 17, 18] {
            assert_eq!(s.slider(f), 0.0, "field {f}");
        }
        assert_eq!(s.shader_detail(), 1);
        assert!(!s.shadows() && !s.mirror());
        s.apply_preset(PRESET_HIGH);
        assert_eq!(
            [3, 7, 11, 12, 14, 15, 17, 18].map(|f| s.slider(f)),
            [1.0, 0.75, 0.75, 2.0 / 3.0, 0.75, 0.75, 1.0, 0.75]
        );
        s.apply_preset(PRESET_VERY_HIGH);
        assert!([3, 7, 11, 12, 14, 15, 17, 18]
            .iter()
            .all(|&f| s.slider(f) == 1.0));
        assert_eq!(s.shader_detail(), 3);
        // Custom keeps the values.
        s.apply_preset(PRESET_CUSTOM);
        assert_eq!(s.slider(field::TERRAIN_QUALITY), 1.0);
        s.move_slider(field::MIRROR_QUALITY, 0.0);
        assert!(!s.mirror());
        s.move_slider(field::MIRROR_DETAILS, 0.5);
        assert!(!s.mirror(), "the details do not turn the mirror on");
        s.move_slider(field::MIRROR_QUALITY, 0.25);
        assert!(s.mirror());
        s.move_slider(field::SHADOW_DETAIL, 0.0);
        assert!(s.shadows(), "the detail slider sets its field only");
        s.move_slider(field::OBJECT_DETAILS, 1.7);
        assert_eq!(s.slider(field::OBJECT_DETAILS), 1.0);
    }

    // spec: SET-1.3
    #[test]
    fn errors() {
        let bytes = write(&observed());
        assert!(matches!(
            parse(&bytes[..100]),
            Err(CfgError::UnexpectedEnd(_))
        ));
        let mut extra = bytes.clone();
        extra.push(0);
        assert_eq!(parse(&extra), Err(CfgError::TrailingBytes(1)));
        let mut v4 = bytes;
        v4[0] = 4;
        assert!(matches!(
            parse(&v4),
            Err(CfgError::UnsupportedVersion { .. })
        ));
    }

    // spec: SET-2.2 (T4)
    #[test]
    fn t4_map_states_on_demand() {
        let mut u = UserObject::default();
        assert!(u.map("campaign01").is_none());
        u.map_mut("campaign01").completed = true;
        u.map_mut("campaign01").acknowledged = 1;
        assert_eq!(u.maps.len(), 1);
        assert!(u.map("campaign01").unwrap().completed);
    }

    /// SET T1 on the player's own file, when there is one.
    // spec: SET-1.1, SET-1.3 (T1)
    #[test]
    fn t1_players_file() {
        let Some(dir) = std::env::var_os("S2_USERS_DIR") else {
            return;
        };
        let path = std::path::Path::new(&dir).join("default01.cfg");
        let Ok(data) = std::fs::read(&path) else {
            return;
        };
        let s = parse(&data).unwrap();
        assert_eq!(s.user.profile, "default01");
        assert_eq!(s.user.autosave_interval, 600);
        assert_eq!(s.user.maps[0].name, "MP_2P_Storm_Coast");
        assert_eq!(s.user.volumes, [1.0, 0.3, 1.0, 1.0, 1.0]);
        assert_eq!(s.scene.resolution(), (1024, 768));
        assert_eq!(write(&s), data);
    }
}
