//! s2-reborn executable. Headless for now: runs the simulation for N ticks,
//! decodes a single game data file with `decode`, summarises a map with
//! `map-info`, gives the films' voice track with `voice-track`, or runs the
//! property scripts with `rules`. `hash`, `replay`, `soak`, `image-diff`,
//! `trace`, `trace-diff` and `fidelity` are the safety net: determinism, invariants,
//! golden images and the trace comparison with the original
//! (`docs/test-coverage.md`, `docs/trace-format.md`).

mod colour_mean;
mod fidelity;
mod game;
mod hash;
mod image_diff;
mod look_measure;
mod profile_seed;
mod shot;
#[cfg(test)]
mod snapshot_tests;
mod soak;
mod trace;

use std::io::Write;
use std::process::ExitCode;

use s2_assets::{rc00, s2m, GameDir};
use s2_sim::World;

const USAGE: &str = "usage: s2-app [--ticks N] [--seed S]
       s2-app decode <file>
       s2-app voice-track <data/scripts/menu.cfg>
       s2-app map-info <file>
       s2-app dds <file> <out.png>
       s2-app game <map file> [start resources 0..3]
       s2-app rules [goods|buildings|patterns|deposits|doodads|remains|animals|streets|sounds|ai|names]
       s2-app ai <map name or file> <ticks> <slot>=<profile>... [--every N] [--seed S] [--resources 0..3] [--watch builders] [--human <slot>] [--tribes <t0,t1,...>]
       s2-app hash <map|fixture:name> <ticks> [--seed S] [--every N] [--ai 0=2,1=2] [--resources R] [--golden FILE [--bless]] [--save FILE] [--snapshot FILE]
       s2-app replay <replay or snapshot file> [--to TICK] [--every N]
       s2-app image-diff <expected.png> <actual.png> [--tolerance T] [--max-share S] [--diff out.png]
       s2-app colour-mean <a.png> [b.png] NAME=X,Y,W,H... [--mark out.png]
       s2-app look-measure <look-compare run dir> [--masks <depth-bin masks dir>]
       s2-app trace <map|fixture:name> <ticks> [--seed S] [--ai ...] [--closed 1] [--from T] [--kinds SBG] [--out FILE[.gz]]
       s2-app rng <map|fixture:name> <ticks> [--seed S] [--ai ...]
       s2-app trace --snapshot <file> <ticks> [--from T] [--kinds SBG] [--out FILE[.gz]]
       s2-app trace-diff <a.csv[.gz]> <b.csv[.gz]> [--by-place] [--common-ticks] [--from T] [--kinds SBG] [--ignore cols] [--with cols]
       s2-app fidelity [--root DIR] [--traces DIR] [--record]
       s2-app soak <map|fixture:name> <ticks> [--seed S] [--ai ...] [--out DIR] | soak --replay FILE | soak --snapshot FILE [ticks] | soak --save FILE ticks
       s2-app shot --map M [--ai 1=1] [--resources R] --tick T --camera FILE --size WxH --out PNG [--viewer EXE]

A map is a name (MP_2P_Storm_Coast), an .s2m path, or fixture:flat2 /
fixture:flat4 (synthetic games that need no install).";

fn main() -> ExitCode {
    let mut ticks: u64 = 100;
    // SIM-RNG-2.1: the game's generator seed.
    let mut seed: u64 = s2_sim::rng::SIM_SEED as u64;
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(String::as_str) == Some("game") {
        args.next();
        return match (args.next(), args.next(), args.next()) {
            (Some(file), resources, None) => match resources.map(|r| r.parse::<u8>()) {
                None => game(&file, 0),
                Some(Ok(r)) => game(&file, r),
                Some(Err(_)) => {
                    eprintln!("{USAGE}");
                    ExitCode::from(2)
                }
            },
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    if args.peek().map(String::as_str) == Some("dds") {
        args.next();
        return match (args.next(), args.next(), args.next()) {
            (Some(file), Some(out), None) => dds_to_png(&file, &out),
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    if args.peek().map(String::as_str) == Some("hash") {
        args.next();
        return hash::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("image-diff") {
        args.next();
        return image_diff::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("profile-seed") {
        args.next();
        return profile_seed::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("colour-mean") {
        args.next();
        return colour_mean::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("look-measure") {
        args.next();
        return look_measure::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("trace") {
        args.next();
        return trace::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("rng") {
        args.next();
        return trace::run_draws(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("trace-diff") {
        args.next();
        return trace::run_diff(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("fidelity") {
        args.next();
        return fidelity::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("shot") {
        args.next();
        return shot::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("soak") {
        args.next();
        return soak::run(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("replay") {
        args.next();
        return hash::replay(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("ai") {
        args.next();
        return ai(&args.collect::<Vec<_>>());
    }
    if args.peek().map(String::as_str) == Some("rules") {
        args.next();
        return match (args.next(), args.next()) {
            (category, None) => rules(category.as_deref()),
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    let file_command: Option<fn(&str) -> ExitCode> = match args.peek().map(String::as_str) {
        Some("decode") => Some(decode),
        Some("map-info") => Some(map_info),
        Some("voice-track") => Some(voice_track),
        _ => None,
    };
    if let Some(command) = file_command {
        args.next();
        return match (args.next(), args.next()) {
            (Some(file), None) => command(&file),
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    while let Some(a) = args.next() {
        match a.as_str() {
            "--ticks" => ticks = args.next().and_then(|v| v.parse().ok()).unwrap_or(ticks),
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(seed),
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => eprintln!("ignoring unknown argument: {other}"),
        }
    }

    match GameDir::from_env() {
        Ok(dir) => println!("game data: {}", dir.root().display()),
        Err(e) => println!("game data: not loaded ({e})"),
    }

    let mut world = World::new(seed);
    for _ in 0..ticks {
        world.step();
    }
    println!("ran {} ticks (seed {seed})", world.tick());
    ExitCode::SUCCESS
}

/// Write the plain contents of `file` to stdout. The key depends on the file's
/// name (FMT-RC-2), so the file must still carry its original name. Plain
/// files pass through unchanged (FMT-RC-1.2).
fn decode(file: &str) -> ExitCode {
    let data = match std::fs::read(file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let plain = if rc00::is_wrapped(&data) {
        match rc00::decode(file, &data) {
            Ok(plain) => plain,
            Err(e) => {
                eprintln!("{file}: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        data
    };
    match std::io::stdout().lock().write_all(&plain) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("stdout: {e}");
            ExitCode::FAILURE
        }
    }
}

/// UI-MENU-0.3a: print the films' voice track for `menu.cfg`'s language
/// (played mixed with track 0; `scripts/convert-films.sh`).
fn voice_track(file: &str) -> ExitCode {
    let plain = match std::fs::read(file)
        .map_err(|e| e.to_string())
        .and_then(|data| {
            if rc00::is_wrapped(&data) {
                rc00::decode(file, &data).map_err(|e| e.to_string())
            } else {
                Ok(data)
            }
        }) {
        Ok(plain) => plain,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let cfg = s2_assets::menu_cfg::parse(&String::from_utf8_lossy(&plain));
    println!("{}", cfg.voice_track());
    ExitCode::SUCCESS
}

/// Write the top level of a DDS texture as an uncompressed PNG, for
/// looking at the interface art.
fn dds_to_png(file: &str, out: &str) -> ExitCode {
    let texture = match std::fs::read(file)
        .map_err(|e| e.to_string())
        .and_then(|data| s2_assets::dds::load(file, &data).map_err(|e| e.to_string()))
    {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let png = png_rgba(texture.width, texture.height, &texture.rgba);
    match std::fs::write(out, png) {
        Ok(()) => {
            println!("{file}: {} x {}", texture.width, texture.height);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{out}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// A PNG of 8-bit RGBA rows, deflate in stored blocks.
fn png_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    }
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks(width as usize * 4) {
        raw.push(0);
        raw.extend(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, block) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        z.extend(len.to_le_bytes());
        z.extend((!len).to_le_bytes());
        z.extend(*block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend(((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Print the header, size, players and object counts of a map file
/// (`specs/FMT-s2m-map.md`).
fn map_info(file: &str) -> ExitCode {
    let map = match std::fs::read(file)
        .map_err(|e| e.to_string())
        .and_then(|data| s2m::load(file, &data).map_err(|e| e.to_string()))
    {
        Ok(map) => map,
        Err(e) => {
            eprintln!("{file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let info = &map.info;
    let uuid: String = info.uuid.iter().map(|b| format!("{b:02x}")).collect();
    let heights = &map.heights;

    println!("name:            {}", info.name);
    println!("size:            {} x {} fields", info.width, info.height);
    println!("uuid:            {uuid}");
    println!("info values:     {:?}", info.info_values);
    println!("next object id:  {}", map.logic.next_unique_id);
    println!("logic tick:      {}", map.logic.tick);
    println!(
        "height grid:     {} x {} samples, floor {}, min {}, max {}",
        heights.width,
        heights.height,
        heights.floor,
        heights.samples.iter().min().copied().unwrap_or(0),
        heights.samples.iter().max().copied().unwrap_or(0),
    );
    println!("start positions: {}", info.start_positions.len());
    println!("players (index: start, type, record):");
    for (i, (player_type, record)) in info
        .player_types
        .iter()
        .zip(info.player_records)
        .enumerate()
    {
        let start = match info.start_positions.get(i) {
            Some(p) => format!("({}, {})", p.x, p.y),
            None => "-".to_owned(),
        };
        println!("  {i}: {start:<10} type {player_type}, record {record:?}");
    }
    println!("triggers:        {}", map.triggers.len());
    for trigger in &map.triggers {
        println!(
            "  {} kind {} at ({}, {})",
            trigger.name, trigger.kind, trigger.position.x, trigger.position.y
        );
    }
    println!("continents:      {}", map.continents.len());
    println!("deposits:        {}", map.deposits.len());
    println!("animals:         {}", map.animals.len());
    println!(
        "doodads:         {} / {} / {} (non-blocking / list 2 / blocking)",
        map.doodads.non_blocking.len(),
        map.doodads.unknown.len(),
        map.doodads.blocking.len()
    );
    println!("ambients:        {}", map.ambients.len());
    println!("padding:         {} bytes", map.padding);
    ExitCode::SUCCESS
}

/// Start a game on a map (`specs/ECO-warehouse-construction.md` ECO-HQ) and
/// print each player's headquarters, territory and stock.
fn game(file: &str, start_resources: u8) -> ExitCode {
    let result = (|| -> Result<_, String> {
        let dir = GameDir::from_env().map_err(|e| e.to_string())?;
        let (rules, _) = s2_rules::load(&dir).map_err(|e| e.to_string())?;
        let data = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
        let map = s2m::load(file, &data).map_err(|e| format!("{file}: {e}"))?;
        let world =
            s2_rules::new_game(s2_sim::rng::SIM_SEED as u64, &map, &rules, start_resources)?;
        Ok((rules, world))
    })();
    let (rules, world) = match result {
        Ok(found) => found,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let name = |good: u32| {
        rules
            .goods
            .get(good)
            .and_then(|g| g.name.clone())
            .unwrap_or_else(|| format!("{good:08X}"))
    };
    println!(
        "start resources setting {start_resources}: x{}",
        s2_sim::start_multiplier(start_resources)
    );
    let mut warehouses: Vec<_> = world
        .warehouses()
        .map(|(at, stock)| (world.building_at(at).expect("a warehouse").owner, at, stock))
        .collect();
    warehouses.sort_by_key(|&(owner, at, _)| (owner, at));
    for (owner, at, stock) in warehouses {
        let explored = world
            .terrain()
            .fields()
            .filter(|&f| world.is_explored(owner, f))
            .count();
        println!(
            "player {}: headquarters at ({}, {}), {} fields owned, {} explored",
            owner.0,
            at.0,
            at.1,
            world.owned_field_count(owner),
            explored
        );
        let items: Vec<String> = stock
            .iter()
            .map(|&(good, n)| format!("{} {n}", name(good)))
            .collect();
        println!("  stock: {}", items.join(", "));
    }
    ExitCode::SUCCESS
}

/// Run the property scripts of the install (`specs/LUA-properties-api.md`)
/// and print a summary, or every entry of one category.
fn rules(category: Option<&str>) -> ExitCode {
    let dir = match GameDir::from_env() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let (rules, report) = match s2_rules::load(&dir) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    fn list<T: std::fmt::Debug>(items: impl IntoIterator<Item = T>) {
        for item in items {
            println!("{item:?}");
        }
    }
    if let Some(category) = category {
        match category {
            "goods" => list(&rules.goods),
            "buildings" => list(&rules.buildings),
            "patterns" => list(&rules.patterns),
            "deposits" => list(&rules.deposits),
            "doodads" => list(&rules.doodads),
            "remains" => list(&rules.remains),
            "animals" => list(&rules.animals),
            "streets" => list(&rules.streets),
            "sounds" => list(&rules.sounds),
            "ai" => list(&rules.ai_profiles),
            "names" => {
                for (name, id) in &rules.names {
                    println!("{name} {id:08X}");
                }
            }
            other => {
                eprintln!("unknown category {other}\n{USAGE}");
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }

    let calls: u32 = report.calls.values().sum();
    println!(
        "{} scripts, {} calls to {} functions",
        report.scripts.len(),
        calls,
        report.calls.len()
    );
    println!(
        "goods:       {} ({} jobs, {} ware groups, {} wares, {} with no type set)",
        rules.goods.len(),
        rules.jobs().count(),
        rules.ware_groups().count(),
        rules.wares().count(),
        rules.untyped_goods().count()
    );
    let counts = [
        ("buildings", rules.buildings.len()),
        ("terrain", rules.patterns.len()),
        ("deposits", rules.deposits.len()),
        ("doodads", rules.doodads.len()),
        ("remains", rules.remains.len()),
        ("animals", rules.animals.len()),
        ("streets", rules.streets.len()),
        ("sounds", rules.sounds.len()),
        ("ai profiles", rules.ai_profiles.len()),
        ("signs", rules.signs.len()),
    ];
    for (name, count) in counts {
        println!("{:<12} {count}", format!("{name}:"));
    }

    let name = |name: &Option<String>| name.clone().unwrap_or_else(|| "?".into());
    let good_name = |id: u32| rules.goods.get(id).map_or("?".into(), |g| name(&g.name));
    println!("\nsample goods with no type set (Q-033):");
    for good in rules.untyped_goods().take(5) {
        println!("  {:08X} {}", good.id, name(&good.name));
    }
    println!("sample jobs:");
    for job in rules.jobs().take(5) {
        println!("  {:08X} {}", job.id, name(&job.name));
    }
    println!("sample buildings (product, production time in ticks):");
    for building in rules.buildings.iter().filter(|b| b.goods.is_some()).take(6) {
        let product = building.goods.map_or("-".into(), |a| {
            format!("{} x {}", a.count, good_name(a.good))
        });
        let time = building
            .production_time
            .map_or("-".into(), |t| t.to_string());
        println!(
            "  {:08X} {:<20} {product}, {time}",
            building.id,
            name(&building.name)
        );
    }
    println!("sample terrain:");
    for pattern in rules.patterns.iter().take(5) {
        println!("  {:08X} {}", pattern.id, name(&pattern.name));
    }
    let settings = &rules.settings;
    println!(
        "settings: settler speed {:?}, construct time {:?}, flag limit {:?}",
        settings.settler_speed, settings.construct_time, settings.flag_package_limit
    );
    ExitCode::SUCCESS
}

/// Run computer players on a map and report what they do:
/// `ai <map> <ticks> <slot>=<profile>... [--every N] [--seed S] [--resources R]`,
/// the profile being 1 weak, 2 normal, 3 strong.
fn ai(args: &[String]) -> ExitCode {
    let bad = |what: &str| {
        eprintln!("{what}\n{USAGE}");
        ExitCode::from(2)
    };
    let (Some(map_name), Some(ticks)) = (
        args.first(),
        args.get(1).and_then(|t| t.parse::<u64>().ok()),
    ) else {
        return bad("ai wants a map and a tick count");
    };
    let mut slots: Vec<(s2_sim::PlayerId, u8)> = Vec::new();
    let (mut every, mut seed, mut resources) = (10_000u64, s2_sim::rng::SIM_SEED as u64, 1u8);
    let mut watch_builders = false;
    let mut humans: Vec<s2_sim::PlayerId> = Vec::new();
    let mut tribes: Vec<i8> = Vec::new();
    let mut rest = args[2..].iter();
    while let Some(arg) = rest.next() {
        if arg == "--tribes" {
            let parsed: Option<Vec<i8>> = rest
                .next()
                .map(|v| v.split(',').map(|t| t.parse::<i8>().ok()).collect())
                .unwrap_or(None);
            match parsed {
                Some(t) => tribes = t,
                None => return bad("--tribes wants a list like 0,2"),
            }
            continue;
        }
        if arg == "--watch" {
            match rest.next().map(String::as_str) {
                Some("builders") => watch_builders = true,
                _ => return bad("--watch wants: builders"),
            }
            continue;
        }
        let mut value = |what: &str| {
            rest.next()
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or(what.to_string())
        };
        match arg.as_str() {
            "--every" => match value("--every wants a number") {
                Ok(v) => every = v,
                Err(e) => return bad(&e),
            },
            "--seed" => match value("--seed wants a number") {
                Ok(v) => seed = v,
                Err(e) => return bad(&e),
            },
            "--resources" => match value("--resources wants a number") {
                Ok(v) => resources = v as u8,
                Err(e) => return bad(&e),
            },
            "--human" => match value("--human wants a slot") {
                Ok(v) => humans.push(s2_sim::PlayerId(v as u8)),
                Err(e) => return bad(&e),
            },
            other => {
                let parsed = other
                    .split_once('=')
                    .and_then(|(s, p)| Some((s.parse::<u8>().ok()?, p.parse::<u8>().ok()?)));
                match parsed {
                    Some((slot, profile)) if (1..=3).contains(&profile) => {
                        slots.push((s2_sim::PlayerId(slot), profile));
                    }
                    _ => return bad(&format!("unknown argument {other}")),
                }
            }
        }
    }
    let result = (|| -> Result<_, String> {
        let dir = GameDir::from_env().map_err(|e| e.to_string())?;
        let (rules, _) = s2_rules::load(&dir).map_err(|e| e.to_string())?;
        let file = s2_assets::find_map(&dir, map_name).ok_or(format!("no map {map_name}"))?;
        let name = file.display().to_string();
        let data = std::fs::read(&file).map_err(|e| format!("{name}: {e}"))?;
        let map = s2m::load(&name, &data).map_err(|e| format!("{name}: {e}"))?;
        let mut setup = s2_rules::game_setup(&map, &rules, resources);
        setup.tribes = tribes
            .iter()
            .enumerate()
            .map(|(i, &t)| (s2_sim::PlayerId(i as u8), t))
            .collect();
        let world = World::new_game(seed, map.terrain(), rules.sim_rules()?, &setup);
        let config = s2_ai::AiConfig::new(&rules, &rules.sim_rules()?);
        Ok((rules, world, config))
    })();
    let (rules, mut world, config) = match result {
        Ok(found) => found,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let mut ai = s2_ai::AiSystem::new(config, &world);
    ai.activate_all(&mut world, &slots);
    let names: std::collections::HashMap<u32, String> = rules
        .names
        .iter()
        .map(|(name, &id)| (id, name.clone()))
        .collect();
    let players: Vec<s2_sim::PlayerId> = slots.iter().map(|&(p, _)| p).collect();
    let watch_names = names.clone();
    let mut runner = s2_ai::run::Runner::new(world, ai, names);
    let start = std::time::Instant::now();
    let mut watch = BuilderWatch::default();
    // --human: scripted players building as a person would.
    let mut scripted: Vec<s2_rules::scenario::Human> = humans
        .iter()
        .filter_map(|&p| s2_rules::scenario::Human::new(&runner.world, &rules, p, seed))
        .collect();
    let report: Vec<s2_sim::PlayerId> = players.iter().chain(&humans).copied().collect();
    for done in 1..=ticks {
        for human in &mut scripted {
            for command in human.decide(&runner.world) {
                runner.world.submit(command);
            }
        }
        runner.step();
        if done % every.max(1) == 0 || done == ticks {
            print!("{}", runner.describe(&report));
            if watch_builders {
                print!("{}", watch.check(&runner.world, &watch_names));
            }
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    print!("{}", runner.summary());
    println!(
        "{ticks} ticks in {seconds:.1} s: {:.0} ticks per second; state hash {:016x}",
        ticks as f64 / seconds.max(1e-9),
        runner.world.state_hash()
    );
    ExitCode::SUCCESS
}

/// `ai --watch builders`: sites whose builder has worked off nothing while
/// a usable material lay there for 500 ticks, whose missing material is
/// neither ordered nor on its way, or that made no progress for ten times
/// [`BuilderWatch::STALL`] ticks; with what the site has and waits for, the
/// flags its wares lie at, their roads and carriers, and a carrierless
/// road's open requests.
#[derive(Default)]
struct BuilderWatch {
    /// Per site: the work done when last seen and the tick it last changed.
    progress: std::collections::HashMap<s2_sim::grid::Point, (u32, u64)>,
    reported: std::collections::HashSet<s2_sim::grid::Point>,
    /// Per site: the tick a material the builder could use was first seen
    /// lying there.
    usable: std::collections::HashMap<s2_sim::grid::Point, u64>,
    /// Wares reported with a claim no carrier's job is about.
    stale: std::collections::HashSet<usize>,
}

impl BuilderWatch {
    const STALL: u64 = 3000;

    fn check(&mut self, world: &World, names: &std::collections::HashMap<u32, String>) -> String {
        use std::fmt::Write as _;
        let now = world.tick();
        let mut out = String::new();
        // TRN-CARRY-2.6: a ware claimed by a carrier whose job is not about
        // it is taken by nobody, ever.
        for (id, ware) in world.wares() {
            let (s2_sim::WareLocation::Flag(flag), Some(c)) = (ware.location, ware.claimed_by)
            else {
                continue;
            };
            use s2_sim::{CarrierTask as T, Entry as E};
            let mine = world
                .carriers()
                .get(c)
                .is_some_and(|carrier| match carrier.task {
                    T::Fetch { ware, .. } | T::PickUp { ware, .. } => ware == id,
                    T::Carry {
                        entry: E::Exchange(ware),
                        ..
                    } => ware == id,
                    T::PutDown { exchange, .. } => exchange == Some(id),
                    _ => false,
                });
            if !mine && self.stale.insert(id) {
                writeln!(
                    out,
                    "stale claim: ware {id} ({}) at flag {flag:?} for {:?}, claimed by carrier {c}: {:?}",
                    names.get(&ware.good).cloned().unwrap_or_default(),
                    ware.target.map(|t| t.at),
                    world.carriers().get(c).map(|c| c.task),
                )
                .ok();
            }
        }
        let sites: Vec<_> = world.sites().map(|(at, s)| (at, s.clone())).collect();
        self.progress
            .retain(|at, _| sites.iter().any(|(a, _)| a == at));
        self.usable
            .retain(|at, _| sites.iter().any(|(a, _)| a == at));
        for (at, site) in sites {
            // ECO-HQ-4.5: a frame material during the frame part, any main
            // material after it.
            let usable = if site.frame_done() {
                !site.on_site.is_empty()
            } else {
                site.on_site
                    .iter()
                    .any(|&(g, _)| site.frame_costs.iter().any(|&(f, _)| f == g))
            };
            let usable_since = if usable && site.builder.is_some() {
                *self.usable.entry(at).or_insert(now)
            } else {
                self.usable.remove(&at);
                now
            };
            let entry = self.progress.entry(at).or_insert((site.work_done, now));
            if entry.0 != site.work_done || site.builder.is_none() {
                *entry = (site.work_done, now);
                self.usable.remove(&at);
                self.reported.remove(&at);
                continue;
            }
            if now - usable_since >= 500 && self.reported.insert(at) {
                writeln!(
                    out,
                    "material on site unused: site at {at:?} since tick {usable_since}: work {}/{}, on site {:?}, builder {:?}",
                    site.work_done, site.work_total, site.on_site, site.builder
                )
                .ok();
                continue;
            }
            if now - entry.1 < Self::STALL || self.reported.contains(&at) {
                continue;
            }
            let orders = world.orders().iter().filter(|o| o.target.at == at).count() as u32;
            let coming = world
                .wares()
                .filter(|(_, w)| w.target.is_some_and(|t| t.at == at))
                .count() as u32;
            let short: u32 = site.shortfall().iter().map(|&(_, n)| n).sum();
            // A wait for material that is ordered or on its way is no fault;
            // a long one is reported with what it waits for.
            let anomaly = if orders + coming < short {
                "material short and not ordered"
            } else if now - entry.1 >= 10 * Self::STALL {
                "no progress for long"
            } else {
                continue;
            };
            self.reported.insert(at);
            let name = |g: u32| names.get(&g).cloned().unwrap_or(format!("{g}"));
            let list = |l: &[(u32, u32)]| {
                l.iter()
                    .map(|&(g, n)| format!("{n} {}", name(g)))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let building = world.building_at(at);
            let kind = building.map_or("?".into(), |b| name(b.building_type));
            let coming: Vec<String> = world
                .wares()
                .filter(|(_, w)| w.target.is_some_and(|t| t.at == at))
                .map(|(_, w)| format!("{} {:?}", name(w.good), w.location))
                .collect();
            writeln!(
                out,
                "{anomaly}: site {kind} at {at:?} (player {:?}) since tick {}: levelled {}, \
                 work {}/{} (frame {}), on site [{}], delivered [{}], short [{}], \
                 open orders {orders}, coming [{}], builder {:?}",
                building.map(|b| b.owner),
                entry.1,
                site.levelled(),
                site.work_done,
                site.work_total,
                site.frame_end,
                list(&site.on_site),
                list(&site.delivered),
                list(&site.shortfall()),
                coming.join(", "),
                site.builder,
            )
            .ok();
            // Where the wares on their way lie, and who would carry them on.
            let mut flags: Vec<s2_sim::grid::Point> = world
                .wares()
                .filter(|(_, w)| w.target.is_some_and(|t| t.at == at))
                .filter_map(|(_, w)| match w.location {
                    s2_sim::WareLocation::Flag(f) => Some(f),
                    _ => None,
                })
                .collect();
            flags.sort_unstable();
            flags.dedup();
            for f in flags {
                writeln!(out, "    flag {f:?}: {} wares", world.flag_load(f)).ok();
                for id in 0..20_000 {
                    let Some(road) = world.road(id) else { continue };
                    let (Some(&a), Some(&b)) = (road.path.first(), road.path.last()) else {
                        continue;
                    };
                    if a != f && b != f {
                        continue;
                    }
                    let carrier = road.carrier.and_then(|c| world.carriers().get(c));
                    writeln!(
                        out,
                        "      road {a:?} -> {b:?} ({} fields): carrier {:?}",
                        road.path.len(),
                        carrier.map(|c| (&c.task, c.walker.at, c.ware))
                    )
                    .ok();
                    if road.carrier.is_none() {
                        let requests: Vec<_> = world
                            .requests(road.owner)
                            .into_iter()
                            .filter(|r| r.requester == s2_sim::Requester::Road(id))
                            .collect();
                        writeln!(out, "        road {id}: requests {requests:?}").ok();
                        for r in requests {
                            for (w, stock) in world.warehouses() {
                                let n = stock
                                    .iter()
                                    .find(|&&(g, _)| g == r.profession)
                                    .map_or(0, |&(_, n)| n);
                                let owner = world.building_at(w).map(|b| b.owner);
                                if owner == Some(road.owner) {
                                    writeln!(
                                        out,
                                        "        warehouse {w:?}: {n} of {}",
                                        name(r.profession)
                                    )
                                    .ok();
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}
