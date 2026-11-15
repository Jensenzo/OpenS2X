//! The player's settings (`specs/SET-settings.md`): the original profile
//! file `<Documents>\S2\users\<profile>.cfg` holds the autosave interval,
//! the info window, the volumes, the screen resolution and the per-map
//! campaign state (CAM-3.5); every field of it the program does not use is
//! kept as it was. The program's own `settings.txt` keeps what the original
//! file has no place for: the game folder, the profile chosen and the
//! day / night switch (a scene field whose place is open, SET-2.3, Q-188).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use s2_assets::cfg;

/// `--play`: nothing the harness does writes the player's settings or profile
/// file (shared with the original, the campaign's unlocks included).
static FILES_HELD: AtomicBool = AtomicBool::new(false);

/// From now on [`Settings::save`] writes nothing.
pub(crate) fn hold_files() {
    FILES_HELD.store(true, Ordering::Relaxed);
}

/// `--profile-dir`: the folder that holds the settings and the profile file
/// in place of the player's.
static OWN_FOLDER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// From now on the settings and the profile file live in `dir`.
pub(crate) fn keep_files_in(dir: PathBuf) {
    let _ = OWN_FOLDER.set(dir);
}

/// The player's settings: the profile file's values and the program's own
/// (read by a project through `extend::CurrentSettings`).
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The game install chosen on first run.
    pub(crate) game_dir: Option<PathBuf>,
    /// SET-1.1: the profile whose file is used.
    pub(crate) profile: String,
    /// SET-2.1: seconds between autosaves, 0 off.
    pub(crate) autosave: u32,
    /// SET-2 field 17.
    pub(crate) info_window: bool,
    /// SET-2 field 22, UI-MENU-3.3: the autosave on leaving, off by default.
    pub(crate) autosave_on_leave: bool,
    pub(crate) day_night: bool,
    /// SND-5.1: master, music, UI, sound.
    pub(crate) volumes: [f32; 4],
    /// The profile file as read, with the fields above copied in on save
    /// (SET-2, SET-4) and the map states (CAM-3.5).
    pub(crate) cfg: cfg::Settings,
}

impl Default for Settings {
    /// SET-3; day and night on (GFX-VIEW-2.3c: the `DayNightCycle`
    /// option's default is 1).
    fn default() -> Self {
        Self::from_cfg(cfg::Settings::default())
    }
}

/// The program's own file: `%APPDATA%\s2-reborn\settings.txt` on Windows,
/// `$XDG_CONFIG_HOME` or `~/.config/s2-reborn/settings.txt` elsewhere; in a
/// project's folder when it gives one (`docs/extending.md`).
pub fn path() -> Option<PathBuf> {
    if let Some(dir) = OWN_FOLDER.get() {
        return Some(dir.join("settings.txt"));
    }
    if let Some(dir) = crate::extend::app_settings_dir() {
        return Some(dir.join("settings.txt"));
    }
    if let Some(app) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(app).join("s2-reborn").join("settings.txt"));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("s2-reborn").join("settings.txt"))
}

/// SET-1.1: the user's documents folder, which holds `S2` (profiles,
/// saves): Windows' known folder Documents (wherever it is redirected, to
/// OneDrive say); `$HOME/Documents` elsewhere.
#[cfg(windows)]
pub fn documents() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{
        FOLDERID_Documents, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
    };
    let mut p: windows_sys::core::PWSTR = std::ptr::null_mut();
    // SAFETY: the out pointer is valid; on success the string it gets is
    // zero-terminated and freed here with CoTaskMemFree, as the API asks.
    unsafe {
        let ok = SHGetKnownFolderPath(
            &FOLDERID_Documents,
            KF_FLAG_DEFAULT as u32,
            std::ptr::null_mut(),
            &mut p,
        ) == 0;
        if !ok || p.is_null() {
            return None;
        }
        let len = (0..).take_while(|&i| *p.add(i) != 0).count();
        let path = std::ffi::OsString::from_wide(std::slice::from_raw_parts(p, len));
        CoTaskMemFree(p as *const _);
        Some(PathBuf::from(path))
    }
}

/// SET-1.1: the user's documents folder: `$HOME/Documents` here.
#[cfg(not(windows))]
pub fn documents() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Documents"))
}

/// SET-1.1: the profiles' folder, `<Documents>\S2\users`.
pub fn users_folder() -> Option<PathBuf> {
    if let Some(dir) = OWN_FOLDER.get() {
        return Some(dir.join("users"));
    }
    Some(documents()?.join("S2").join("users"))
}

/// The profile to use when none was chosen: the first `.cfg` of the
/// profiles' folder in name order, else `default` (SET-1.1; how the
/// original picks one is Q-188).
fn first_profile(users: &Path) -> String {
    let mut names: Vec<String> = std::fs::read_dir(users)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".cfg").map(str::to_owned)
        })
        .collect();
    names.sort();
    names.into_iter().next().unwrap_or_else(|| "default".into())
}

impl Settings {
    /// The settings a profile file gives.
    pub(crate) fn from_cfg(cfg: cfg::Settings) -> Self {
        let u = &cfg.user;
        Self {
            game_dir: None,
            profile: u.profile.clone(),
            autosave: u.autosave_interval,
            // SET-2 field 17 (corrected 2026-11-14 b, a picture of the
            // original's page): 0 = On, 1 = Off.
            info_window: u.info_window == 0,
            autosave_on_leave: u.autosave_on_leave != 0,
            // GFX-VIEW-2.3c: the DayNightCycle option is on by default.
            day_night: true,
            volumes: [u.volumes[0], u.volumes[1], u.volumes[2], u.volumes[3]],
            cfg,
        }
    }

    /// The profile file with this settings' values.
    pub(crate) fn to_cfg(&self) -> cfg::Settings {
        let mut c = self.cfg.clone();
        c.user.autosave_interval = self.autosave;
        c.user.info_window = u32::from(!self.info_window);
        c.user.autosave_on_leave = u32::from(self.autosave_on_leave);
        c.user.volumes[..4].copy_from_slice(&self.volumes);
        c
    }

    /// The program's own lines over `self`.
    pub(crate) fn parse_own(&mut self, text: &str) {
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "game_dir" if !value.is_empty() => self.game_dir = Some(PathBuf::from(value)),
                "profile" if !value.is_empty() => self.profile = value.to_owned(),
                "day_night" => self.day_night = value == "1",
                _ => {}
            }
        }
    }

    pub(crate) fn own_text(&self) -> String {
        format!(
            "game_dir={}\nprofile={}\nday_night={}\n",
            self.game_dir
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            self.profile,
            u8::from(self.day_night),
        )
    }

    /// CAM-3.5: mark `map` completed; acknowledged goes to 0 unless it is
    /// already 1.
    pub(crate) fn mark_completed(&mut self, map: &str) {
        let m = self.cfg.user.map_mut(map);
        m.completed = true;
        if m.acknowledged != 1 {
            m.acknowledged = 0;
        }
    }

    /// CAM-3.5: acknowledge an unacknowledged completed map; true when it
    /// was one (the level-completed moment).
    pub(crate) fn acknowledge(&mut self, map: &str) -> bool {
        let Some(m) = self.cfg.user.maps.iter_mut().find(|m| m.name == map) else {
            return false;
        };
        if m.completed && m.acknowledged == 0 {
            m.acknowledged = 1;
            true
        } else {
            false
        }
    }

    pub(crate) fn completed(&self, map: &str) -> bool {
        self.cfg.user.map(map).is_some_and(|m| m.completed)
    }

    /// GFX-WATER-2.5: the caustics pass is drawn at shader detail 2 (low)
    /// or more (SET-4).
    pub(crate) fn caustics(&self) -> bool {
        self.cfg.scene.shader_detail() >= 2
    }

    /// GFX-WATER-2.6: the mirror texture's size on a back buffer `width`
    /// pixels wide while the mirror switch is on; none while it is off or
    /// the quality makes no texture (the water is `RenderWaterNoMirror`).
    pub(crate) fn mirror_size(&self, width: u32) -> Option<(u32, u32)> {
        let scene = &self.cfg.scene;
        if !scene.mirror() {
            return None;
        }
        crate::render::world3d::mirror_texture_size(scene.mirror_quality(), width)
    }

    /// SND-5.1, SET-2 field 12: the music volume, 0 to 1. The music
    /// stream plays at it (SND-7.1).
    pub fn music_volume(&self) -> f32 {
        self.volumes[1]
    }

    /// SET-4: the screen resolution of the profile.
    pub(crate) fn resolution(&self) -> (u32, u32) {
        self.cfg.scene.resolution()
    }

    /// SET-1.2: the program's file, then the profile's file (defaults when
    /// it is missing or unreadable); a project's copy of the profile file
    /// in place of the original's when there is one (`docs/extending.md`).
    pub(crate) fn load() -> Self {
        let own = path().and_then(|p| std::fs::read_to_string(p).ok());
        Self::load_from(own.as_deref(), users_folder(), crate::extend::profile_dir())
    }

    /// [`load`](Self::load) with the program's file's text `own`, the
    /// profiles' folder `users` and the folder of a project's copies.
    pub(crate) fn load_from(
        own: Option<&str>,
        users: Option<PathBuf>,
        copies: Option<&Path>,
    ) -> Self {
        let mut probe = Self {
            profile: String::new(),
            ..Self::default()
        };
        if let Some(t) = own {
            probe.parse_own(t);
        }
        let profile = match (&probe.profile, &users) {
            (p, _) if !p.is_empty() => p.clone(),
            (_, Some(u)) => first_profile(u),
            _ => "default".into(),
        };
        let name = format!("{profile}.cfg");
        let copy = copies.map(|dir| dir.join(&name)).filter(|f| f.is_file());
        let file = copy.or_else(|| users.map(|u| u.join(&name)));
        let cfg = match file.as_ref().map(std::fs::read) {
            Some(Ok(data)) => cfg::parse(&data).unwrap_or_else(|e| {
                eprintln!("{}: {e}", file.as_ref().expect("read").display());
                cfg::Settings::default()
            }),
            _ => {
                let mut c = cfg::Settings::default();
                c.user.profile = profile.clone();
                c
            }
        };
        let mut s = Self::from_cfg(cfg);
        if let Some(t) = own {
            s.parse_own(t);
        }
        s.profile = profile;
        s
    }

    /// SET-1.2: write both files. The first time the profile file of the
    /// original is overwritten, it is kept as `<profile>.cfg.bak`. A
    /// project may keep the profile file unwritten, keep a copy of it in a
    /// folder of its own and hears of every change (`docs/extending.md`).
    pub(crate) fn save(&self) {
        if FILES_HELD.load(Ordering::Relaxed) {
            return;
        }
        self.write_files(
            path(),
            users_folder(),
            crate::extend::profile_write(),
            crate::extend::profile_dir(),
        );
        crate::extend::settings_changed(self);
    }

    /// Write the program's file `own`, with `profile` the profile file in
    /// `users` (and its backup, the first time) and the copy of the profile
    /// file in `copies`.
    pub(crate) fn write_files(
        &self,
        own: Option<PathBuf>,
        users: Option<PathBuf>,
        profile: bool,
        copies: Option<&Path>,
    ) {
        if let Some(p) = own {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&p, self.own_text()) {
                eprintln!("{}: {e}", p.display());
            }
        }
        if let Some(dir) = copies {
            let _ = std::fs::create_dir_all(dir);
            let file = dir.join(format!("{}.cfg", self.profile));
            if let Err(e) = std::fs::write(&file, cfg::write(&self.to_cfg())) {
                eprintln!("{}: {e}", file.display());
            }
        }
        let Some(users) = users.filter(|_| profile) else {
            return;
        };
        let _ = std::fs::create_dir_all(&users);
        let file = users.join(format!("{}.cfg", self.profile));
        let backup = users.join(format!("{}.cfg.bak", self.profile));
        if file.exists() && !backup.exists() {
            let _ = std::fs::copy(&file, &backup);
        }
        if let Err(e) = std::fs::write(&file, cfg::write(&self.to_cfg())) {
            eprintln!("{}: {e}", file.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: SET-1.1 (the profiles' folder below the documents folder)
    #[test]
    fn the_profiles_are_below_the_documents() {
        assert_eq!(
            users_folder(),
            documents().map(|d| d.join("S2").join("users"))
        );
    }

    /// SET-1.2: both files are written; with a project's `profile_write`
    /// false (docs/extending.md) the profile file and its backup are not,
    /// the program's own file still is.
    // spec: SET-1.2
    #[test]
    fn the_profile_file_is_written_unless_a_project_says_no() {
        let root = std::env::temp_dir().join(format!("s2-settings-{}", std::process::id()));
        let (own, users) = (root.join("app").join("settings.txt"), root.join("users"));
        let s = Settings {
            profile: "tester".into(),
            ..Settings::default()
        };
        s.write_files(Some(own.clone()), Some(users.clone()), false, None);
        assert!(own.is_file());
        assert!(!users.exists());
        s.write_files(Some(own.clone()), Some(users.clone()), true, None);
        assert!(users.join("tester.cfg").is_file());
        assert!(!users.join("tester.cfg.bak").exists());
        s.write_files(Some(own), Some(users.clone()), true, None);
        assert!(users.join("tester.cfg.bak").is_file());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A project's profile folder (docs/extending.md): every change writes
    /// the copy there, with the original's file unwritten too; the next
    /// start reads the copy. Before the first copy, and without the folder,
    /// the original's file is read.
    // spec: SET-1.2 (the profile's values, kept by a project)
    #[test]
    fn a_projects_copy_of_the_profile_lasts_from_start_to_start() {
        let root = std::env::temp_dir().join(format!("s2-profile-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (users, copies) = (root.join("users"), root.join("frontier"));
        let own = Some("profile=tester\n");
        let original = Settings {
            profile: "tester".into(),
            autosave: 600,
            ..Settings::default()
        };
        original.write_files(None, Some(users.clone()), true, None);
        let first = Settings::load_from(own, Some(users.clone()), Some(&copies));
        assert_eq!((first.profile.as_str(), first.autosave), ("tester", 600));
        assert!(!copies.exists(), "made at the first change");

        let mut changed = first.clone();
        changed.autosave = 1200;
        changed.volumes[1] = 0.5;
        changed.write_files(None, Some(users.clone()), false, Some(&copies));
        assert!(copies.join("tester.cfg").is_file());
        assert!(!users.join("tester.cfg.bak").exists());
        let next = Settings::load_from(own, Some(users.clone()), Some(&copies));
        assert_eq!((next.autosave, next.music_volume()), (1200, 0.5));
        // The original's file is as it was; without the folder it is read.
        let plain = Settings::load_from(own, Some(users), None);
        assert_eq!((plain.autosave, plain.music_volume()), (600, 0.3));
        std::fs::remove_dir_all(&root).unwrap();
    }

    // SET-3, SET-2 field 17 (corrected 2026-11-14 b): a new profile has the
    // info window On (0); 1 is Off.
    // spec: SET-3, SET-2
    #[test]
    fn defaults_and_the_profile_file() {
        let d = Settings::default();
        assert_eq!(d.volumes, [1.0, 0.3, 1.0, 1.0]);
        assert_eq!(
            (d.autosave, d.info_window, d.profile.as_str()),
            (0, true, "default")
        );
        // The program's values go into the file, the rest stays.
        let mut c = cfg::Settings::default();
        c.user.player_name = "Anna".into();
        c.user.volumes[4] = 0.5;
        let mut s = Settings::from_cfg(c);
        s.autosave = 600;
        s.info_window = false;
        s.volumes = [0.5, 0.25, 1.0, 0.75];
        let c = s.to_cfg();
        assert_eq!(c.user.autosave_interval, 600);
        assert_eq!(c.user.info_window, 1);
        assert_eq!(c.user.volumes, [0.5, 0.25, 1.0, 0.75, 0.5]);
        assert_eq!(c.user.player_name, "Anna");
        let back = Settings::from_cfg(cfg::parse(&cfg::write(&c)).unwrap());
        assert_eq!(
            (back.autosave, back.info_window, back.volumes),
            (600, false, s.volumes)
        );
    }

    // spec: none (the program's own settings file)
    #[test]
    fn own_file() {
        let s = Settings {
            game_dir: Some(PathBuf::from("C:/Games/S2")),
            profile: "default01".into(),
            day_night: true,
            ..Settings::default()
        };
        let mut back = Settings::default();
        back.parse_own(&s.own_text());
        assert_eq!(back.game_dir, s.game_dir);
        assert_eq!(back.profile, "default01");
        assert!(back.day_night);
        // Old lines of the program's earlier file are ignored.
        back.parse_own("volume_music=7\nnonsense\nmap=campaign01,1,1");
        assert_eq!(back.volumes, Settings::default().volumes);
    }

    // spec: SET-4, GFX-WATER-2.5
    #[test]
    fn caustics_need_shader_detail_2() {
        let mut s = Settings::default();
        // The default block's shader detail 3 (high).
        assert!(s.caustics());
        for (detail, on) in [(1, false), (2, true), (3, true)] {
            s.cfg.scene.words[15] = detail;
            assert_eq!(s.caustics(), on, "detail {detail}");
        }
    }

    #[test]
    fn t4_campaign_state() {
        // SET T4, CAM-3.5
        let mut s = Settings::default();
        s.mark_completed("campaign02");
        assert!(s.completed("campaign02"));
        assert!(s.acknowledge("campaign02"));
        assert!(!s.acknowledge("campaign02"));
        s.mark_completed("campaign02");
        let c = cfg::parse(&cfg::write(&s.to_cfg())).unwrap();
        let m = c.user.map("campaign02").unwrap();
        assert!(m.completed);
        assert_eq!(m.acknowledged, 1);
        assert!(!s.completed("campaign03"));
    }
}
