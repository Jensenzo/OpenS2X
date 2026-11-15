//! Every decoder of the original's files on damaged input: none may panic,
//! whatever the bytes. A seeded mutation fuzzer (no extra dependency): each
//! seed input is damaged many times over (bits flipped, bytes set to edge
//! values, length fields made huge, cut short, stretched, pieces repeated)
//! and handed to its decoder inside `catch_unwind`.
//!
//! `cargo test -p s2-assets --test fuzz` runs synthetic seeds (also on CI);
//! with `S2_GAME_DIR` the long run takes real files of each format as seeds:
//! `cargo test -p s2-assets --test fuzz -- --ignored --nocapture`
//! (`FUZZ_ROUNDS` sets the damaged copies per seed, default 300; a debug
//! build also catches arithmetic overflow and the decoders' checks; release
//! is faster). A failure
//! names the decoder and writes the input that broke it to
//! `target/fuzz-failures/`.

mod common;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use s2_assets::*;

/// xorshift64*: the fuzzer's own seeded random numbers.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// One damaged copy of `seed`: one to eight damages of random kinds.
fn mutate(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut d = seed.to_vec();
    for _ in 0..1 + rng.below(8) {
        let at = rng.below(d.len());
        match rng.below(8) {
            0 if !d.is_empty() => d[at] ^= 1 << rng.below(8),
            1 if !d.is_empty() => d[at] = [0, 1, 0x7f, 0x80, 0xff][rng.below(5)],
            // A 32-bit field made huge, zero or negative.
            2 if d.len() >= 4 => {
                let at = at.min(d.len() - 4);
                let v: u32 = [u32::MAX, 0x7fff_ffff, 0x8000_0000, 0, 0x0010_0000][rng.below(5)];
                d[at..at + 4].copy_from_slice(&v.to_le_bytes());
            }
            3 => d.truncate(at),
            4 => {
                let n = rng.below(64);
                d.splice(at..at, std::iter::repeat_n(rng.next() as u8, n));
            }
            5 if d.len() > 2 => {
                let from = rng.below(d.len());
                let len = rng.below((d.len() - from).min(256));
                let piece = d[from..from + len].to_vec();
                d.splice(at..at, piece);
            }
            6 if d.len() > 2 => {
                let end = (at + rng.below(256)).min(d.len());
                d.drain(at..end);
            }
            _ if !d.is_empty() => d[at] = rng.next() as u8,
            _ => d.push(rng.next() as u8),
        }
    }
    d
}

/// The message of the last panic caught.
static LAST_PANIC: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// A decoder under test: its name and how to feed it bytes. The result is
/// thrown away; only a panic counts.
struct Decoder {
    name: &'static str,
    run: fn(&[u8]),
}

fn text(data: &[u8]) -> String {
    String::from_utf8_lossy(data).into_owned()
}

const DECODERS: &[Decoder] = &[
    Decoder {
        name: "rc00",
        run: |d| {
            let _ = rc00::parse_header(d);
            let _ = rc00::decompress(d);
            let _ = rc00::decode("data/settings/buildings.xml", d);
        },
    },
    Decoder {
        name: "s2m",
        run: |d| {
            let _ = s2m::parse(d);
            let _ = s2m::parse_info(d);
        },
    },
    Decoder {
        name: "sav",
        run: |d| {
            let _ = sav::parse(d);
        },
    },
    Decoder {
        name: "gr2",
        run: |d| {
            let _ = gr2::read_header(d);
            if let Ok(raw) = gr2::read_raw(d) {
                let _ = gr2::model::Gr2::from_raw(&raw);
            }
        },
    },
    Decoder {
        name: "oodle1",
        run: |d| {
            let n = d.len() / 4 * 4;
            let _ = gr2::oodle1::decompress(d, n * 4, n, n * 2);
            let _ = gr2::oodle1::decompress(d, n + 6, n / 2, n);
        },
    },
    Decoder {
        name: "dds",
        run: |d| {
            let _ = dds::decode(d);
        },
    },
    Decoder {
        name: "tga",
        run: |d| {
            let _ = tga::read(d);
        },
    },
    Decoder {
        name: "bmp",
        run: |d| {
            let _ = bmp::read(d);
        },
    },
    Decoder {
        name: "wav",
        run: |d| {
            let _ = wav::playable(d);
        },
    },
    Decoder {
        name: "pfx",
        run: |d| {
            let _ = pfx::parse(d);
        },
    },
    Decoder {
        name: "cfg",
        run: |d| {
            if let Ok(s) = cfg::parse(d) {
                // SET-1.3: a file read and written again is unchanged.
                assert_eq!(cfg::write(&s), d, "a profile file read and written back");
            }
        },
    },
    Decoder {
        name: "xml",
        run: |d| {
            let t = text(d);
            let elements = xml::elements(&t);
            let _ = xml::strip_comments(&t);
            for e in elements.iter().take(50) {
                let _ = xml::attributes(&e.tag);
            }
            let _ = anim_tables::entries(&elements);
            let _ = anim_tables::SettlerAnimations::read(&t);
            let _ = color_slots::ColorSlots::read(&t);
            let _ = landscape::read(&t);
            let mut tables = model_tables::ModelTables::default();
            tables.read_buildings(&t);
            tables.read_items(&t);
            tables.read_animals(&t);
            tables.read_ships(&t);
            tables.read_characters(&t);
        },
    },
    Decoder {
        name: "text",
        run: |d| {
            let t = lang::decode(d);
            let _ = lang::Texts::parse(&t);
            let _ = bmfont::Font::parse(&t);
            let _ = subtitles::Subtitles::parse(&t);
            let _ = terrain_patterns::TerrainPatterns::parse(&t);
            let _ = menu_cfg::parse(&t);
        },
    },
];

fn decoder(name: &str) -> &'static Decoder {
    DECODERS.iter().find(|d| d.name == name).expect("a decoder")
}

/// Feed `rounds` damaged copies of each seed to the decoder, and the seed
/// itself; return the failures (the inputs are written out).
fn fuzz(decoder: &Decoder, seeds: &[Vec<u8>], rounds: u32, rng: &mut Rng) -> Vec<String> {
    let mut failures = Vec::new();
    eprintln!("fuzzing {}: {} seeds", decoder.name, seeds.len());
    // The panic's message and place, kept for the report instead of
    // printed.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|info| {
        *LAST_PANIC.lock().unwrap() = info.to_string();
    }));
    for (s, seed) in seeds.iter().enumerate() {
        for round in 0..=rounds {
            let input = if round == 0 {
                seed.clone()
            } else {
                mutate(seed, rng)
            };
            if catch_unwind(AssertUnwindSafe(|| (decoder.run)(&input))).is_err() {
                let dir =
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/fuzz-failures");
                std::fs::create_dir_all(&dir).ok();
                let file = dir.join(format!("{}-seed{s}-round{round}.bin", decoder.name));
                std::fs::write(&file, &input).ok();
                let why = LAST_PANIC.lock().unwrap().clone();
                failures.push(format!(
                    "{}: seed {s}, round {round}: {}\n  {why}",
                    decoder.name,
                    file.display()
                ));
                if failures.len() >= 5 {
                    std::panic::set_hook(hook);
                    return failures;
                }
            }
        }
    }
    std::panic::set_hook(hook);
    failures
}

/// Small inputs of each format made here, with the format's magic numbers,
/// so the damaged copies reach past the first check.
fn synthetic_seeds(name: &str) -> Vec<Vec<u8>> {
    let mut rng = Rng(0x5eed);
    let noise: Vec<u8> = (0..512).map(|_| rng.next() as u8).collect();
    let with = |prefix: &[u8]| {
        let mut v = prefix.to_vec();
        v.extend(&noise[..256]);
        v
    };
    let mut seeds = vec![Vec::new(), noise.clone()];
    match name {
        "rc00" => seeds.push(with(b"RC00")),
        "gr2" | "oodle1" => seeds.push(with(&[
            0xb8, 0x67, 0xb0, 0xca, 0xf8, 0x6d, 0xb1, 0x0f, 0x84, 0x72, 0x8c, 0x7e, 0x5e, 0x19,
            0x00, 0x1e,
        ])),
        "dds" => {
            let mut v = b"DDS ".to_vec();
            v.extend(124u32.to_le_bytes());
            v.extend(&noise[..200]);
            seeds.push(v);
        }
        "bmp" => seeds.push(with(b"BM")),
        "tga" => seeds.push(with(&[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 4, 0, 32, 8])),
        "wav" => seeds.push(wav::pcm_wav(&[0, 100, -100, 3000], 1, 22050)),
        "cfg" => seeds.push(cfg::write(&cfg::Settings {
            user: cfg::UserObject::default(),
            scene: cfg::SceneSettings { words: [1; 19] },
        })),
        "xml" => seeds.push(
            br#"<Root a="1"><!-- c --><Building name="x" file="y.GR2"><Item s='2'/></Building></Root>"#
                .to_vec(),
        ),
        "text" => seeds.push(b"[KEY]\ntext one\n[TWO]\nline\nchar id=65 x=1 y=2 width=3\n".to_vec()),
        _ => {}
    }
    seeds
}

// spec: none (every decoder survives damaged input)
#[test]
fn decoders_survive_damaged_synthetic_input() {
    let mut rng = Rng(1);
    let mut failures = Vec::new();
    for d in DECODERS {
        failures.extend(fuzz(d, &synthetic_seeds(d.name), 400, &mut rng));
    }
    assert!(
        failures.is_empty(),
        "decoders panicked:\n{}",
        failures.join("\n")
    );
}

#[test]
fn profile_files_round_trip() {
    // SET-1.3: random settings written and read back are the same.
    let mut rng = Rng(7);
    let string = |rng: &mut Rng| -> String {
        (0..rng.below(20))
            .map(|_| char::from(32 + rng.below(200) as u8))
            .collect()
    };
    for _ in 0..500 {
        let mut s = cfg::Settings {
            user: cfg::UserObject::default(),
            scene: cfg::SceneSettings { words: [0; 19] },
        };
        let u = &mut s.user;
        u.profile = string(&mut rng);
        u.autosave_name = string(&mut rng);
        u.player_name = string(&mut rng);
        u.unknown_strings = [string(&mut rng), string(&mut rng)];
        // Any float but NaN (which is never equal to itself).
        let float = |rng: &mut Rng| {
            let f = f32::from_bits(rng.next() as u32);
            if f.is_nan() {
                0.5
            } else {
                f
            }
        };
        u.unused = float(&mut rng);
        u.scroll_speed = float(&mut rng);
        u.volumes = [0; 5].map(|_| float(&mut rng));
        u.autosave_interval = rng.next() as u32;
        u.info_window = rng.next() as u32;
        u.autosave_on_leave = rng.next() as u32;
        [u.input_a, u.input_b, u.input_c, u.input_d] = [0; 4].map(|_| rng.below(2) == 1);
        u.unknown_bools = [rng.below(2) == 1, rng.below(2) == 1];
        for i in 0..rng.below(5) {
            u.maps.push(cfg::MapState {
                name: format!("map{i}{}", string(&mut rng)),
                completed: rng.below(2) == 1,
                acknowledged: rng.next() as u32,
            });
        }
        for (word, kind) in s.scene.words.iter_mut().zip(cfg::SCENE_KINDS) {
            *word = match kind {
                cfg::Kind::Bool => rng.below(2) as u32,
                _ => rng.next() as u32,
            };
        }
        let bytes = cfg::write(&s);
        assert_eq!(cfg::parse(&bytes).as_ref(), Ok(&s));
        assert_eq!(cfg::write(&cfg::parse(&bytes).unwrap()), bytes);
    }
}

/// Real files of the install (decoded from rc00 where wrapped) by format:
/// `per_format` of each, spread evenly over the format's files in name
/// order.
/// Seed inputs by decoder name.
type SeedSets = Vec<(&'static str, Vec<Vec<u8>>)>;

fn install_seeds(per_format: usize) -> Option<SeedSets> {
    let dir = common::game_dir()?;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![dir.root().join("data")];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if e.metadata().is_ok_and(|m| m.len() < 2_000_000) {
                files.push(p);
            }
        }
    }
    files.sort();
    let formats: [(&str, &[&str]); 11] = [
        ("s2m", &["s2m"]),
        ("gr2", &["gr2"]),
        ("dds", &["dds"]),
        ("tga", &["tga"]),
        ("bmp", &["bmp"]),
        ("wav", &["wav"]),
        ("pfx", &["pfx"]),
        ("xml", &["xml"]),
        ("text", &["txt", "fnt", "cfg", "ini"]),
        ("rc00", &["xml", "lua", "s2m"]),
        ("cfg", &["cfg"]),
    ];
    let mut out = Vec::new();
    for (name, extensions) in formats {
        let mut seeds = Vec::new();
        let matching: Vec<&PathBuf> = files
            .iter()
            .filter(|f| {
                let ext = f
                    .extension()
                    .map(|e| e.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                extensions.contains(&ext.as_str())
            })
            .collect();
        let every = (matching.len() / per_format).max(1);
        for f in matching.into_iter().step_by(every) {
            if seeds.len() >= per_format {
                break;
            }
            let data = std::fs::read(f).unwrap();
            let wrapped = rc00::is_wrapped(&data);
            if name == "rc00" {
                if wrapped {
                    seeds.push(data);
                }
                continue;
            }
            let plain = if wrapped {
                match rc00::decode(f.to_str().unwrap(), &data) {
                    Ok(p) => p,
                    Err(_) => continue,
                }
            } else {
                data
            };
            seeds.push(plain);
        }
        out.push((name, seeds));
    }
    // FMT-SAV: the player's saves, when S2_SAVES_DIR names their folder.
    if let Some(saves) = std::env::var_os("S2_SAVES_DIR") {
        let mut seeds = Vec::new();
        for e in std::fs::read_dir(saves).into_iter().flatten().flatten() {
            if e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("sav"))
            {
                let data = std::fs::read(e.path()).unwrap();
                seeds.push(if rc00::is_wrapped(&data) {
                    rc00::decode(e.path().to_str().unwrap(), &data).unwrap_or(data)
                } else {
                    data
                });
            }
            if seeds.len() >= per_format {
                break;
            }
        }
        out.push(("sav", seeds));
    }
    Some(out)
}

// spec: none (every decoder survives damaged input)
#[test]
#[ignore = "the long run on the install's files, in release mode"]
fn decoders_survive_damaged_install_files() {
    let Some(sets) = install_seeds(8) else {
        return;
    };
    let rounds: u32 = std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(300);
    let mut rng = Rng(2);
    let mut failures = Vec::new();
    for (name, seeds) in sets {
        let started = std::time::Instant::now();
        let found = fuzz(decoder(name), &seeds, rounds, &mut rng);
        eprintln!(
            "{name}: {} seeds x {rounds} damaged copies, {} failures, {:.1} s",
            seeds.len(),
            found.len(),
            started.elapsed().as_secs_f64()
        );
        failures.extend(found);
    }
    assert!(
        failures.is_empty(),
        "decoders panicked:\n{}",
        failures.join("\n")
    );
}
