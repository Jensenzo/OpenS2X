//! The game's own save files, the format (UI-MENU-5, FMT-SAV-5;
//! `docs/snapshot.md`). The program's use of them, when and under which
//! name, is the viewer's (`s2_viewer::game::save`).
//!
//! FMT-SAV describes the original's `.sav`, which `s2_assets::sav` reads
//! and nothing here writes. A game of ours is saved as our snapshot with a
//! header the program needs to show it again: the map, the computer
//! players' slots, the mission's interface.
//!
//! ```text
//! "s2-save\0", save format (VERSION), world format (s2_sim::snap::VERSION)
//! map, tick, world hash, AI hash, slots, mission (with the script's
//! variables, FMT-SAV-3.12), mark (from save format 2: the project and its
//! rules overlays, empty for s2-reborn)
//! the world (World::snapshot), then whether there is an AI and its state
//! ```
//!
//! **A save loads in every later version**: the first supported are save
//! format 1 with world format 65, and `tests/saves` holds a fixture of every
//! supported format, which must load and play on (`docs/snapshot.md`,
//! "Changing the format"). A later header field is read only from the save
//! format it came with (`if save >= 2 { .. }` in [`read_header`]).

use std::fmt;
use std::path::{Path, PathBuf};

use s2_sim::snap::{Reader, Writer};
use s2_sim::{PlayerId, World};

/// The first bytes of a save file of ours.
pub const MAGIC: &[u8; 8] = b"s2-save\0";
/// The save format this build writes (of the header; the world and the
/// computer players have [`s2_sim::snap::VERSION`]). This build reads every
/// one from 1 on. 2: the header's [`SaveMark`].
pub const VERSION: u64 = 2;
/// The extension of our save files, beside the original's `.sav`.
pub const EXTENSION: &str = "s2save";

/// What the program keeps of a game besides the world and the computer
/// players.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    /// The file the game's map came from (an `.s2m`, or the original's
    /// save it was continued from); found again by its name without the
    /// extension among the install's maps when it is not there.
    pub map: String,
    pub tick: u64,
    /// `World::state_hash` and `AiSystem::state_hash` when saved: a load
    /// that gives other hashes is not the game that was saved.
    pub world_hash: u64,
    pub ai_hash: Option<u64>,
    /// The computer players, `(slot, profile)`.
    pub slots: Vec<(PlayerId, u8)>,
    pub mission: SavedMission,
    /// Who wrote the game: the project and its rules overlays (empty for
    /// s2-reborn, and for a save of format 1, which had none).
    pub mark: SaveMark,
}

/// The program a save was written by, beyond its version (W11 b of the
/// hooks, `docs/extending.md` "Saves"): a project built on
/// s2-reborn and the rules overlays it played with, each by its name and
/// [`text_hash`] of its code, in the order they ran. A game is only loaded
/// by a program of the same mark ([`read_as`]): with other overlays it would
/// play on with other rules. Empty (the default) for s2-reborn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveMark {
    /// The project's name; empty for s2-reborn.
    pub project: String,
    /// The rules overlays: (name, [`text_hash`] of the code).
    pub overlays: Vec<(String, u64)>,
}

impl SaveMark {
    /// The mark of a program named `project` (none: s2-reborn) running
    /// with `overlays`.
    pub fn of(project: Option<&str>, overlays: &[s2_rules::Overlay]) -> Self {
        Self {
            project: project.unwrap_or_default().to_owned(),
            overlays: overlays
                .iter()
                .map(|o| (o.name.clone(), text_hash(&o.code)))
                .collect(),
        }
    }

    /// s2-reborn's own mark: no project, no overlays.
    pub fn is_empty(&self) -> bool {
        self.project.is_empty() && self.overlays.is_empty()
    }

    /// The program's name: the project's, or s2-reborn.
    pub fn program(&self) -> &str {
        if self.project.is_empty() {
            "s2-reborn"
        } else {
            &self.project
        }
    }
}

impl fmt::Display for SaveMark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program())?;
        for (n, (name, hash)) in self.overlays.iter().enumerate() {
            let lead = if n == 0 {
                " with the rules overlays "
            } else {
                ", "
            };
            write!(f, "{lead}{name} ({hash:016x})")?;
        }
        Ok(())
    }
}

/// The hash of an overlay's text in a [`SaveMark`]: 64-bit FNV-1a of its
/// bytes, the same on every platform and in every version.
pub fn text_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The mission's interface state (CAM-1.3) that is not in the world.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SavedMission {
    pub target: u8,
    pub tutorial_step: Option<u8>,
    pub debriefing: Vec<String>,
    pub extro: Vec<String>,
    pub letters: Vec<String>,
    /// FMT-SAV-3.12: the script's variables.
    pub script_variables: Vec<(String, u32)>,
}

/// A save read back.
pub struct Loaded {
    pub header: Header,
    pub world: World,
    pub ai: Option<crate::AiSystem>,
    /// The world format it was written in. Older than this build's, the
    /// game was brought up to date and its hashes are not those saved (a
    /// new field is part of the state hash); played on, it is a game of this
    /// version from there.
    pub format: u32,
}

impl Loaded {
    /// Written in this build's format: the state hashes must be those saved.
    pub fn current_format(&self) -> bool {
        self.format == s2_sim::snap::VERSION
    }
}

/// Why a save does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Not a save of s2-reborn.
    NotOurs,
    /// Written by another version: its save and world formats.
    Version { save: u64, world: u64 },
    /// Cut short or damaged, or no rules for its computer players.
    Broken(String),
    /// Written by another program or with other rules overlays than the
    /// program reading it (`running`): another version too (W11 b).
    OtherMark { saved: SaveMark, running: SaveMark },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOurs => write!(f, "not a save of s2-reborn"),
            Self::Version { save, world } => write!(
                f,
                "saved by another version of s2-reborn (save format {save}, game state {world}; \
                 this version reads save formats 1 to {VERSION}, game states {} to {})",
                s2_sim::snap::OLDEST,
                s2_sim::snap::VERSION
            ),
            Self::Broken(e) => write!(f, "the file is damaged: {e}"),
            Self::OtherMark { saved, running } => write!(
                f,
                "saved by {}, another version: {saved}; this is {running}",
                saved.program()
            ),
        }
    }
}

impl From<s2_sim::snap::SnapError> for LoadError {
    fn from(e: s2_sim::snap::SnapError) -> Self {
        Self::Broken(e.to_string())
    }
}

/// The save named `name` in `folder`.
pub fn path_of(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!("{name}.{EXTENSION}"))
}

/// The bytes of a save of `world` and its computer players.
pub fn write(header: &Header, world: &World, ai: Option<&crate::AiSystem>) -> Vec<u8> {
    let mut w = Writer::new();
    w.raw(MAGIC);
    w.u64(VERSION);
    w.u64(u64::from(s2_sim::snap::VERSION));
    w.put(&header.map);
    w.put(&header.tick);
    w.put(&header.world_hash);
    w.put(&header.ai_hash);
    w.put(&header.slots);
    let m = &header.mission;
    w.put(&m.target);
    w.put(&m.tutorial_step);
    w.put(&m.debriefing);
    w.put(&m.extro);
    w.put(&m.letters);
    w.put(&m.script_variables);
    w.put(&header.mark.project);
    w.put(&header.mark.overlays);
    w.raw(&world.snapshot());
    w.put(&ai.is_some());
    if let Some(ai) = ai {
        ai.put_state(&mut w);
    }
    w.bytes
}

/// Write a save to `path`: first to a file beside it, then renamed over
/// it, so a save cut short never replaces a good one.
pub fn write_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let part = path.with_extension(format!("{EXTENSION}.part"));
    std::fs::write(&part, bytes)?;
    std::fs::rename(&part, path)
}

/// The header of a save, after its formats are checked.
pub fn read_header(r: &mut Reader) -> Result<Header, LoadError> {
    if r.raw(MAGIC.len()).map_err(|_| LoadError::NotOurs)? != MAGIC {
        return Err(LoadError::NotOurs);
    }
    let save = r.u64()?;
    let world = r.u64()?;
    let formats = u64::from(s2_sim::snap::OLDEST)..=u64::from(s2_sim::snap::VERSION);
    if !(1..=VERSION).contains(&save) || !formats.contains(&world) {
        return Err(LoadError::Version { save, world });
    }
    Ok(Header {
        map: r.get()?,
        tick: r.get()?,
        world_hash: r.get()?,
        ai_hash: r.get()?,
        slots: r.get()?,
        mission: SavedMission {
            target: r.get()?,
            tutorial_step: r.get()?,
            debriefing: r.get()?,
            extro: r.get()?,
            letters: r.get()?,
            script_variables: r.get()?,
        },
        // Format 1 had no mark: s2-reborn's game.
        mark: if save >= 2 {
            SaveMark {
                project: r.get()?,
                overlays: r.get()?,
            }
        } else {
            SaveMark::default()
        },
    })
}

/// Read a save as s2-reborn: [`read_as`] with the empty mark, so a save of a
/// project or of a game with rules overlays is refused.
pub fn read(
    bytes: &[u8],
    rules: &s2_sim::Rules,
    new_ai: impl FnOnce(&World) -> Option<crate::AiSystem>,
) -> Result<Loaded, LoadError> {
    read_as(bytes, &SaveMark::default(), rules, new_ai)
}

/// Read a save as the program marked `running`: a save with another mark
/// is refused ([`LoadError::OtherMark`]) before its game is read. `rules`
/// are this build's rules of the save's game: an older format's world takes
/// from them what later formats added to the rules
/// (`World::take_later_rules`, `docs/snapshot.md`). `new_ai` makes the
/// computer players' system for the loaded world (from the rules), whose
/// state the save then replaces.
pub fn read_as(
    bytes: &[u8],
    running: &SaveMark,
    rules: &s2_sim::Rules,
    new_ai: impl FnOnce(&World) -> Option<crate::AiSystem>,
) -> Result<Loaded, LoadError> {
    let mut r = Reader::new(bytes);
    let header = read_header(&mut r)?;
    if header.mark != *running {
        return Err(LoadError::OtherMark {
            saved: header.mark,
            running: running.clone(),
        });
    }
    let mut world = World::read_snapshot(&mut r)?;
    // Both formats are the save's: the computer players are read in it too.
    let format = r.format();
    world.take_later_rules(format, rules);
    let ai = if r.get::<bool>()? {
        let mut ai = new_ai(&world)
            .ok_or_else(|| LoadError::Broken("no rules for its computer players".into()))?;
        ai.get_state(&mut r)?;
        Some(ai)
    } else {
        None
    };
    if !r.at_end() {
        return Err(LoadError::Broken("bytes after the game".into()));
    }
    Ok(Loaded {
        header,
        world,
        ai,
        format,
    })
}

/// The format of a save written out as text (`s2_sim::snap::Shape`), which
/// `tests/saves/format.txt` records.
pub fn format_text() -> String {
    let mut s = s2_sim::snap::Shape::new();
    s.add::<World>();
    crate::AiSystem::state_shape(&mut s);
    format!(
        "save format {VERSION}, world format {}\n{}",
        s2_sim::snap::VERSION,
        s.text()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2_sim::invariants::Checker;

    fn header(world: &World, ai: Option<&crate::AiSystem>) -> Header {
        Header {
            map: "fixture".into(),
            tick: world.tick(),
            world_hash: world.state_hash(),
            ai_hash: ai.map(crate::AiSystem::state_hash),
            slots: vec![(PlayerId(1), 2)],
            mission: SavedMission {
                target: 2,
                tutorial_step: Some(4),
                debriefing: vec!["d".into()],
                extro: Vec::new(),
                letters: vec!["l1".into(), "l2".into()],
                script_variables: vec![("step".into(), 5)],
            },
            mark: SaveMark::default(),
        }
    }

    /// The mark of a project's program with one rules overlay.
    fn frontier(code: &[u8]) -> SaveMark {
        SaveMark::of(
            Some("s2-frontier"),
            &[s2_rules::Overlay {
                name: "charcoal".into(),
                code: code.to_vec(),
            }],
        )
    }

    // spec: none (hook W11 b, docs/extending.md "Saves": s2-reborn writes an empty mark; the hash of an overlay's text is FNV-1a 64)
    #[test]
    fn s2_reborn_marks_its_saves_empty_and_a_text_hash_never_changes() {
        let reborn = SaveMark::of(None, &[]);
        assert_eq!(reborn, SaveMark::default());
        assert!(reborn.is_empty());
        assert_eq!(reborn.to_string(), "s2-reborn");
        // FNV-1a 64's own check values: the hash may never change, or every
        // project's saves would be refused by the next version.
        assert_eq!(text_hash(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(text_hash(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(text_hash(b"foobar"), 0x85944171f73967e8);
        let mark = frontier(b"properties.bp_addCost(woodCutter, stone, 1)");
        assert!(!mark.is_empty());
        assert_eq!(
            mark.overlays,
            vec![(
                "charcoal".to_owned(),
                text_hash(b"properties.bp_addCost(woodCutter, stone, 1)")
            )]
        );
        assert!(mark
            .to_string()
            .starts_with("s2-frontier with the rules overlays charcoal ("));
    }

    // spec: none (hook W11 b, docs/extending.md "Saves": a load whose mark differs from the program's is refused with the another-version notice naming the project, never a crash)
    #[test]
    fn a_save_keeps_its_mark_and_only_the_same_program_loads_it() {
        let (world, ai, config) = played(30);
        let new_ai = |w: &World| Some(crate::AiSystem::new(config.clone(), w));
        let mark = frontier(b"coal = 1");
        let saved = Header {
            mark: mark.clone(),
            ..header(&world, Some(&ai))
        };
        let bytes = write(&saved, &world, Some(&ai));
        // The same program loads it, mark and all.
        let loaded = read_as(&bytes, &mark, &config.sim, new_ai).expect("the same program");
        assert_eq!(loaded.header, saved);
        assert_eq!(
            write(&loaded.header, &loaded.world, loaded.ai.as_ref()),
            bytes
        );
        // s2-reborn refuses it, naming the project.
        let e = read(&bytes, &config.sim, new_ai)
            .err()
            .expect("refused by s2-reborn");
        assert_eq!(
            e,
            LoadError::OtherMark {
                saved: mark.clone(),
                running: SaveMark::default()
            }
        );
        let text = e.to_string();
        assert!(
            text.starts_with("saved by s2-frontier, another version"),
            "{text}"
        );
        assert!(text.ends_with("this is s2-reborn"), "{text}");
        // The project with another text of its overlay, another overlay,
        // none, or another name: refused.
        for running in [
            frontier(b"coal = 2"),
            SaveMark::of(Some("s2-frontier"), &[]),
            SaveMark {
                project: "s2-other".into(),
                ..mark.clone()
            },
            SaveMark {
                overlays: vec![mark.overlays[0].clone(), ("more".into(), 1)],
                ..mark.clone()
            },
        ] {
            let e = read_as(&bytes, &running, &config.sim, new_ai)
                .err()
                .unwrap_or_else(|| panic!("{running} loads it"));
            assert!(e.to_string().contains("another version"), "{e}");
        }
        // And the project refuses s2-reborn's games.
        let reborn = write(&header(&world, Some(&ai)), &world, Some(&ai));
        let e = read_as(&reborn, &mark, &config.sim, new_ai)
            .err()
            .expect("refused by the project");
        assert!(
            e.to_string()
                .starts_with("saved by s2-reborn, another version"),
            "{e}"
        );
        assert!(read(&reborn, &config.sim, new_ai).is_ok());
    }

    // spec: none (hook W11 b, docs/snapshot.md: a save of format 1 has no mark, it is s2-reborn's game)
    #[test]
    fn a_save_of_format_1_has_no_mark() {
        for world in [65, 66] {
            let bytes = std::fs::read(saves_dir().join(fixture_name(1, world))).expect("a fixture");
            let header = read_header(&mut Reader::new(&bytes)).expect("its header");
            assert_eq!(header.mark, SaveMark::default(), "save1-world{world}");
            let config = fixture_config(&header.map);
            assert!(read(&bytes, &config.sim, |w| Some(crate::AiSystem::new(
                config.clone(),
                w
            )))
            .is_ok());
        }
    }

    /// A fixture game of computer players played for `ticks`.
    fn played(ticks: u64) -> (World, crate::AiSystem, crate::AiConfig) {
        let (mut world, config) = crate::fixture::named("town2", 7).expect("the fixture");
        let mut ai = crate::AiSystem::new(config.clone(), &world);
        let slots = crate::every_player(&world);
        ai.activate_all(&mut world, &slots);
        for _ in 0..ticks {
            ai.step(&mut world);
        }
        (world, ai, config)
    }

    // spec: UI-MENU-5.1, UI-MENU-5.2
    #[test]
    fn a_saved_game_loads_as_it_was_and_plays_on_the_same() {
        let (mut world, mut ai, config) = played(400);
        let saved = header(&world, Some(&ai));
        let bytes = write(&saved, &world, Some(&ai));
        let loaded = read(&bytes, &config.sim, |w| {
            Some(crate::AiSystem::new(config.clone(), w))
        })
        .expect("the save loads");
        assert_eq!(loaded.header, saved);
        let (mut world2, mut ai2) = (loaded.world, loaded.ai.expect("its computer players"));
        assert_eq!(world2.tick(), world.tick());
        assert_eq!(world2.state_hash(), saved.world_hash);
        assert_eq!(Some(ai2.state_hash()), saved.ai_hash);
        // Saved again, the same bytes.
        assert_eq!(write(&saved, &world2, Some(&ai2)), bytes);
        // Played on, the same game as never saved.
        for _ in 0..300 {
            ai.step(&mut world);
            ai2.step(&mut world2);
        }
        assert_eq!(world2.state_hash(), world.state_hash());
        assert_eq!(ai2.state_hash(), ai.state_hash());
    }

    // spec: UI-MENU-5.2
    #[test]
    fn another_version_or_a_damaged_file_is_refused_with_a_reason() {
        let (world, ai, config) = played(50);
        let bytes = write(&header(&world, Some(&ai)), &world, Some(&ai));
        let new_ai = |w: &World| Some(crate::AiSystem::new(config.clone(), w));
        // Another save format.
        let mut other = bytes.clone();
        other[MAGIC.len()] = (VERSION + 1) as u8;
        let e = read(&other, &config.sim, new_ai).err().expect("refused");
        assert_eq!(
            e,
            LoadError::Version {
                save: VERSION + 1,
                world: u64::from(s2_sim::snap::VERSION)
            }
        );
        assert!(e.to_string().contains("another version"), "{e}");
        // Another world format: the byte after the save format.
        let mut older = bytes.clone();
        older[MAGIC.len() + 1] = 1;
        assert!(matches!(
            read(&older, &config.sim, new_ai),
            Err(LoadError::Version { world: 1, .. })
        ));
        // Not ours, cut short, too long: refused, never a panic.
        assert_eq!(
            read(b"rc00....", &config.sim, new_ai).err(),
            Some(LoadError::NotOurs)
        );
        assert_eq!(
            read(&[], &config.sim, new_ai).err(),
            Some(LoadError::NotOurs)
        );
        for cut in [10, 40, bytes.len() / 2, bytes.len() - 1] {
            assert!(
                matches!(
                    read(&bytes[..cut], &config.sim, new_ai),
                    Err(LoadError::Broken(_))
                ),
                "cut at {cut}"
            );
        }
        let mut long = bytes.clone();
        long.push(0);
        assert!(matches!(
            read(&long, &config.sim, new_ai),
            Err(LoadError::Broken(_))
        ));
        // Computer players without rules for them.
        assert!(matches!(
            read(&bytes, &config.sim, |_| None),
            Err(LoadError::Broken(_))
        ));
    }

    // ---- saves that survive updates (docs/snapshot.md, "Changing the format")

    /// What every supported format must have in `tests/saves`.
    fn saves_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/saves")
    }

    fn fixture_name(save: u64, world: u32) -> String {
        format!("save{save}-world{world}.{EXTENSION}")
    }

    /// The fixtures of the folder, by file name.
    fn fixtures() -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(saves_dir())
            .expect("tests/saves")
            .map(|e| e.expect("an entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == EXTENSION))
            .collect();
        files.sort();
        files
    }

    /// A fixture's game: computer players on our own synthetic map and
    /// rules (`crate::fixture`), so it holds nothing of the original's.
    /// At this tick the game holds the most kinds of state: sites, wares on
    /// the roads, soldiers on their way (it is quiet by tick 12 000).
    const FIXTURE_GAME: (&str, u64, u64) = ("town4", 7, 9_000);

    fn fixture_config(map: &str) -> crate::AiConfig {
        let name = map.strip_prefix("fixture:").expect("a fixture's map");
        crate::fixture::named(name, 1).expect("the fixture").1
    }

    /// `path` loads, plays 1000 ticks with every soak invariant holding,
    /// and saves again; what it saved loads as the same game and plays on
    /// the same.
    fn plays_on(path: &Path) {
        let name = path.display();
        let bytes = std::fs::read(path).expect("the fixture");
        // Loaded by the program that wrote it: its own mark.
        let (map, mark) = {
            let mut r = Reader::new(&bytes);
            let header = read_header(&mut r).expect("its header");
            (header.map, header.mark)
        };
        let config = fixture_config(&map);
        let new_ai = |w: &World| Some(crate::AiSystem::new(config.clone(), w));
        let loaded = read_as(&bytes, &mark, &config.sim, new_ai)
            .unwrap_or_else(|e| panic!("{name} does not load: {e}"));
        let current = loaded.current_format();
        let Loaded {
            header,
            mut world,
            ai,
            ..
        } = loaded;
        let mut ai = ai.unwrap_or_else(|| panic!("{name} has computer players"));
        assert_eq!(world.tick(), header.tick, "{name}");
        if current {
            assert_eq!(world.state_hash(), header.world_hash, "{name}");
            assert_eq!(Some(ai.state_hash()), header.ai_hash, "{name}");
        }
        let mut checker = Checker::new(&world);
        for _ in 0..1000 {
            ai.step(&mut world);
            let broken = checker.check(&world);
            assert!(broken.is_empty(), "{name}: {broken:?}");
        }
        assert_eq!(world.tick(), header.tick + 1000, "{name}");
        let again = Header {
            tick: world.tick(),
            world_hash: world.state_hash(),
            ai_hash: Some(ai.state_hash()),
            ..header
        };
        let written = write(&again, &world, Some(&ai));
        let loaded = read_as(&written, &mark, &config.sim, new_ai)
            .unwrap_or_else(|e| panic!("{name} again: {e}"));
        assert!(loaded.current_format(), "{name}: saved in this format");
        assert_eq!(loaded.header, again, "{name}");
        assert_eq!(loaded.world.state_hash(), again.world_hash, "{name}");
        let (mut world2, mut ai2) = (loaded.world, loaded.ai.expect("its computer players"));
        assert_eq!(Some(ai2.state_hash()), again.ai_hash, "{name}");
        for _ in 0..50 {
            ai.step(&mut world);
            ai2.step(&mut world2);
        }
        assert_eq!(world2.state_hash(), world.state_hash(), "{name}");
        assert_eq!(ai2.state_hash(), ai.state_hash(), "{name}");
    }

    // spec: none (our own save format, docs/snapshot.md: every supported format's fixture loads, plays 1000 ticks with every invariant, saves again)
    #[test]
    fn every_fixture_save_loads_plays_on_and_saves_again() {
        let files = fixtures();
        assert!(!files.is_empty(), "no fixture in tests/saves");
        for path in files {
            plays_on(&path);
        }
    }

    // A game saved before format 66 keeps working, not only loading: every
    // counter has a zero sample for each roll-over so far (series 7 too), and
    // played on by its computer players, the counters and series 7 go on with
    // the roll-overs, as a game of this build's does, and it saves and loads
    // again. The fixture's huts make nothing, so the counting itself after
    // such a load is `s2_sim`'s `a_world_of_format_65_counts_from_the_load`.
    // Every field added with a default since format 65 is in the two: the
    // goods' counters and the table, the coins and the frags.
    // spec: UI-GAME-8.5e, UI-GAME-8.5a
    #[test]
    fn a_game_saved_before_format_66_has_working_statistics() {
        use s2_sim::statistics::{series, MAX_SAMPLES, SAMPLE_TICKS};
        let path = saves_dir().join(fixture_name(1, 65));
        let bytes = std::fs::read(&path).expect("the format 65 fixture");
        let config = fixture_config("fixture:town4");
        let new_ai = |w: &World| Some(crate::AiSystem::new(config.clone(), w));
        let loaded = read(&bytes, &config.sim, new_ai).expect("it loads");
        assert_eq!(loaded.format, 65);
        let (mut world, mut ai) = (loaded.world, loaded.ai.expect("computer players"));
        let rolled = |tick: u64| (((tick - 1) / SAMPLE_TICKS) as usize).min(MAX_SAMPLES);
        let players: Vec<PlayerId> = crate::every_player(&world)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        assert_eq!(players.len(), 4);
        let shape = |world: &World| {
            for &p in &players {
                let s = world.statistics(p).expect("its statistics");
                for counter in &s.production {
                    assert_eq!(counter.len(), rolled(world.tick()));
                }
                assert_eq!(s.series[series::FRAGS].len(), rolled(world.tick()));
            }
        };
        shape(&world);
        for &p in &players {
            let s = world.statistics(p).unwrap();
            assert!(s.production.iter().flatten().all(|&n| n == 0));
            assert_eq!((s.coins, s.frags), (0, 0));
        }
        for _ in 0..2 * SAMPLE_TICKS {
            ai.step(&mut world);
        }
        shape(&world);
        let header = Header {
            tick: world.tick(),
            world_hash: world.state_hash(),
            ai_hash: Some(ai.state_hash()),
            ..loaded.header
        };
        let again = read(&write(&header, &world, Some(&ai)), &config.sim, new_ai)
            .expect("saved again, it loads");
        assert!(again.current_format());
        assert_eq!(again.world.state_hash(), header.world_hash);
    }
    // spec: none (our own save format, docs/snapshot.md: a format without its fixture fails, so does a change of the bytes under the same number)
    #[test]
    fn every_supported_format_has_its_fixture() {
        let (oldest, now) = (s2_sim::snap::OLDEST, s2_sim::snap::VERSION);
        let names: Vec<String> = fixtures()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        for world in oldest..=now {
            assert!(
                names
                    .iter()
                    .any(|n| n.ends_with(&format!("-world{world}.{EXTENSION}"))),
                "world format {world} has no fixture in tests/saves: {}",
                HOW_TO_CHANGE
            );
        }
        for save in 1..=VERSION {
            assert!(
                names.iter().any(|n| n.starts_with(&format!("save{save}-"))),
                "save format {save} has no fixture in tests/saves: {}",
                HOW_TO_CHANGE
            );
        }
        // This build's format: read and written again it is the same bytes,
        // whatever the fixture's game holds.
        let current = saves_dir().join(fixture_name(VERSION, now));
        let bytes = std::fs::read(&current).unwrap_or_else(|e| {
            panic!("{}: {e}: {}", current.display(), HOW_TO_CHANGE);
        });
        let config = fixture_config("fixture:town4");
        let mark = read_header(&mut Reader::new(&bytes))
            .unwrap_or_else(|e| panic!("{}: {e}: {}", current.display(), HOW_TO_CHANGE))
            .mark;
        let loaded = read_as(&bytes, &mark, &config.sim, |w| {
            Some(crate::AiSystem::new(config.clone(), w))
        })
        .unwrap_or_else(|e| panic!("{}: {e}: {}", current.display(), HOW_TO_CHANGE));
        assert!(
            write(&loaded.header, &loaded.world, loaded.ai.as_ref()) == bytes,
            "this format's fixture does not read and write the same bytes: the bytes changed \
             under the same format number: {HOW_TO_CHANGE}"
        );
    }

    const HOW_TO_CHANGE: &str = "the snapshot format changed. A save of ours must load in every \
        later version: raise VERSION (s2_sim::snap, or s2_ai::save for the header), write the \
        added field as `name @ VERSION = default` on its macro line, add a new variant at the end \
        of its line, then scripts/save-format.sh writes tests/saves/format.txt and this format's \
        fixture. See docs/snapshot.md, \"Changing the format\"";

    // spec: none (our own save format, docs/snapshot.md: CI fails when the format changes without a new number)
    #[test]
    fn the_format_is_the_recorded_one() {
        let path = saves_dir().join("format.txt");
        let recorded = std::fs::read_to_string(&path).expect("tests/saves/format.txt");
        // Git on Windows may have made the line ends CRLF.
        let recorded = recorded.replace("\r\n", "\n");
        let now = format_text();
        if recorded != now {
            let first = recorded
                .lines()
                .zip(now.lines())
                .find(|(a, b)| a != b)
                .map_or_else(
                    || "one text is longer".to_string(),
                    |(a, b)| format!("recorded `{a}`, now `{b}`"),
                );
            panic!("{HOW_TO_CHANGE}. The first difference: {first}");
        }
    }

    /// The types of a format's text, each with its lines.
    fn types_of(text: &str) -> std::collections::BTreeMap<&str, Vec<&str>> {
        let mut types = std::collections::BTreeMap::new();
        let mut at = "";
        for line in text.lines().skip(1) {
            match line.strip_prefix("    ") {
                Some(field) => types.entry(at).or_insert_with(Vec::new).push(field),
                None => {
                    at = line;
                    types.entry(at).or_default();
                }
            }
        }
        types
    }

    /// The format a field's name says it came with, and whether older bytes
    /// are read by hand for it: `name @66`, `name @66=>`.
    fn since(name: &str) -> (Option<u32>, bool) {
        match name.split_once(" @") {
            None => (None, false),
            Some((_, rest)) => (
                rest.trim_end_matches("=>").parse().ok(),
                rest.ends_with("=>"),
            ),
        }
    }

    /// `line` of a later format as the format `older` had it, without the
    /// fields added since (a whole field line: `None`). `new` collects the
    /// formats those fields name; `by_hand` whether one reads older bytes.
    fn as_in_format(
        line: &str,
        older: u32,
        new: &mut Vec<u32>,
        by_hand: &mut bool,
    ) -> Option<String> {
        let mut keep = |name: &str| {
            let (from, hand) = since(name);
            *by_hand |= hand && from.is_some_and(|f| f > older);
            match from {
                Some(f) if f > older => {
                    new.push(f);
                    false
                }
                _ => true,
            }
        };
        let Some((head, fields)) = line.split_once(" { ") else {
            return line
                .split_once(": ")
                .is_none_or(|(n, _)| keep(n))
                .then(|| line.to_string());
        };
        let fields = fields.strip_suffix(" }").unwrap_or(fields);
        let kept: Vec<&str> = fields
            .split(", ")
            .filter(|f| f.split_once(": ").is_none_or(|(n, _)| keep(n)))
            .collect();
        Some(format!("{head} {{ {} }}", kept.join(", ")))
    }

    /// What a later format may do to the bytes of the one before: a new
    /// field names the format it came with (so older bytes take its default),
    /// a new variant goes at the end of its list, nothing else moves. A type
    /// with a field read by hand from older bytes (`=>`) answers for itself.
    fn later_format_problems(older_text: &str, newer_text: &str, now: u32) -> Vec<String> {
        let older: u32 = format_number(older_text);
        let (old, new) = (types_of(older_text), types_of(newer_text));
        let mut problems = Vec::new();
        for (name, lines) in &new {
            let Some(before) = old.get(name) else {
                continue;
            };
            let (mut added, mut by_hand) = (Vec::new(), false);
            let view: Vec<String> = lines
                .iter()
                .filter_map(|l| as_in_format(l, older, &mut added, &mut by_hand))
                .collect();
            if let Some(&f) = added.iter().find(|&&f| f != now) {
                problems.push(format!("{name}: a field names format {f}, this is {now}"));
            }
            if by_hand {
                continue;
            }
            let is_variant = |l: &str| {
                l.split(' ')
                    .next()
                    .is_some_and(|n| n.parse::<u32>().is_ok())
            };
            let same = view.len() >= before.len()
                && view.iter().zip(before.iter()).all(|(a, b)| a == b)
                && view[before.len()..].iter().all(|l| is_variant(l))
                && (before.iter().all(|l| is_variant(l)) || view.len() == before.len());
            if !same {
                problems.push(format!(
                    "{name}: as format {older} read it, it is not what format {older} wrote: {view:?} against {before:?}"
                ));
            }
        }
        problems
    }

    /// The number on a format text's first line.
    fn format_number(text: &str) -> u32 {
        text.lines()
            .next()
            .and_then(|l| l.rsplit(' ').next())
            .and_then(|n| n.parse().ok())
            .expect("a format text's first line")
    }

    // spec: none (our own save format, docs/snapshot.md: what a later format may do to the bytes of the one before)
    #[test]
    fn a_later_format_keeps_the_older_bytes_readable_or_says_how() {
        let older = "save format 1, world format 65\nT\n    a: u32\n    b: u8\nE\n    0 A\n    1 B { x: u8 }\n";
        let check = |newer: &str| later_format_problems(older, newer, 66);
        let head = "save format 1, world format 66\n";
        // A field with its format, a field of a variant, a variant at the
        // end, a new type: fine.
        let fine = format!("{head}T\n    a: u32\n    b: u8\n    c @66: u8\nE\n    0 A\n    1 B {{ x: u8, y @66: u8 }}\n    2 C\nN\n    z: u8\n");
        assert_eq!(check(&fine), Vec::<String>::new());
        // A field made from older bytes by hand answers for itself.
        let by_hand =
            format!("{head}T\n    a @66=>: u64\n    b: u8\nE\n    0 A\n    1 B {{ x: u8 }}\n");
        assert_eq!(check(&by_hand), Vec::<String>::new());
        for bad in [
            // A new field that does not say so.
            format!("{head}T\n    a: u32\n    b: u8\n    c: u8\nE\n    0 A\n    1 B {{ x: u8 }}\n"),
            // A field's type changed, a field gone, two swapped.
            format!("{head}T\n    a: u64\n    b: u8\nE\n    0 A\n    1 B {{ x: u8 }}\n"),
            format!("{head}T\n    a: u32\nE\n    0 A\n    1 B {{ x: u8 }}\n"),
            format!("{head}T\n    b: u8\n    a: u32\nE\n    0 A\n    1 B {{ x: u8 }}\n"),
            // A variant before others, a variant's new field that does not say so.
            format!("{head}T\n    a: u32\n    b: u8\nE\n    0 C\n    1 A\n    2 B {{ x: u8 }}\n"),
            format!("{head}T\n    a: u32\n    b: u8\nE\n    0 A\n    1 B {{ x: u8, y: u8 }}\n"),
            // Naming another format than this one.
            format!(
                "{head}T\n    a: u32\n    b: u8\n    c @67: u8\nE\n    0 A\n    1 B {{ x: u8 }}\n"
            ),
        ] {
            assert_eq!(check(&bad).len(), 1, "{bad}");
        }
    }

    // spec: none (our own save format, docs/snapshot.md: CI fails when a format change has no migration)
    #[test]
    fn this_format_keeps_the_previous_formats_bytes_readable() {
        let dir = saves_dir();
        let now = std::fs::read_to_string(dir.join("format.txt")).expect("format.txt");
        let Ok(previous) = std::fs::read_to_string(dir.join("format-previous.txt")) else {
            // The first supported format has none before it.
            assert_eq!(s2_sim::snap::VERSION, s2_sim::snap::OLDEST);
            return;
        };
        let (now, previous) = (now.replace("\r\n", "\n"), previous.replace("\r\n", "\n"));
        let problems = later_format_problems(&previous, &now, s2_sim::snap::VERSION);
        assert!(
            problems.is_empty(),
            "{HOW_TO_CHANGE}:\n{}",
            problems.join("\n")
        );
    }

    /// `scripts/save-format.sh`: records this format and writes its fixture.
    /// A fixture never changes once written: it is what an old version
    /// wrote, and the test of every later one.
    // spec: none (our tool: writes tests/saves/format.txt and the fixture of a new format)
    #[test]
    #[ignore = "scripts/save-format.sh runs it"]
    fn record_the_format() {
        let dir = saves_dir();
        std::fs::create_dir_all(&dir).expect("tests/saves");
        // A new number: the record so far is the format before this one.
        let record = dir.join("format.txt");
        if let Ok(old) = std::fs::read_to_string(&record) {
            if format_number(&old) != s2_sim::snap::VERSION {
                std::fs::write(dir.join("format-previous.txt"), old).expect("format-previous.txt");
            }
        }
        std::fs::write(&record, format_text()).expect("format.txt");
        let path = dir.join(fixture_name(VERSION, s2_sim::snap::VERSION));
        if path.exists() {
            eprintln!("{} is there; it stays", path.display());
            return;
        }
        let (name, seed, ticks) = FIXTURE_GAME;
        let (mut world, config) = crate::fixture::named(name, seed).expect("the fixture");
        let mut ai = crate::AiSystem::new(config, &world);
        let slots: Vec<(PlayerId, u8)> = crate::every_player(&world);
        ai.activate_all(&mut world, &slots);
        for _ in 0..ticks {
            ai.step(&mut world);
        }
        let header = Header {
            map: format!("fixture:{name}"),
            tick: world.tick(),
            world_hash: world.state_hash(),
            ai_hash: Some(ai.state_hash()),
            slots,
            mission: SavedMission {
                target: 2,
                tutorial_step: Some(4),
                debriefing: vec!["the debriefing".into()],
                extro: vec!["the extro".into()],
                letters: vec!["a letter".into()],
                script_variables: vec![("step".into(), 5)],
            },
            // From save format 2: a project's mark, so its bytes are tested.
            mark: SaveMark {
                project: "fixture".into(),
                overlays: vec![(
                    "an overlay".into(),
                    text_hash(b"properties.bp_addCost(woodCutter, stone, 1)"),
                )],
            },
        };
        let bytes = write(&header, &world, Some(&ai));
        std::fs::write(&path, &bytes).expect("the fixture");
        eprintln!(
            "{}: {} bytes, tick {}, {} buildings, {} settlers",
            path.display(),
            bytes.len(),
            world.tick(),
            world.buildings().count(),
            world.settlers().count()
        );
    }

    // spec: UI-MENU-5.2
    #[test]
    fn older_than_the_first_format_or_newer_is_refused_with_the_notice() {
        let (world, ai, config) = played(20);
        let bytes = write(&header(&world, Some(&ai)), &world, Some(&ai));
        let new_ai = |w: &World| Some(crate::AiSystem::new(config.clone(), w));
        let with = |save: u64, format: u64| {
            let mut w = Writer::new();
            w.raw(MAGIC);
            w.u64(save);
            w.u64(format);
            let mut r = Reader::new(&bytes);
            r.raw(MAGIC.len()).unwrap();
            r.u64().unwrap();
            r.u64().unwrap();
            let rest = r.raw(bytes.len() - MAGIC.len() - 2).unwrap_or(&[]);
            w.raw(rest);
            w.bytes
        };
        let oldest = u64::from(s2_sim::snap::OLDEST);
        let now = u64::from(s2_sim::snap::VERSION);
        for (save, format) in [
            (VERSION, oldest - 1), // before the first supported
            (VERSION, 58),
            (VERSION, 0),
            (VERSION, now + 1), // a newer version's
            (VERSION + 1, now),
            (0, now),
            (u64::MAX, u64::MAX),
        ] {
            let e = read(&with(save, format), &config.sim, new_ai)
                .err()
                .unwrap_or_else(|| panic!("save {save}, world {format} reads"));
            assert_eq!(
                e,
                LoadError::Version {
                    save,
                    world: format
                }
            );
            let text = e.to_string();
            assert!(text.contains("another version"), "{text}");
            assert!(text.contains(&format!("game state {format}")), "{text}");
            assert!(text.contains(&format!("{oldest} to {now}")), "{text}");
        }
    }
}
