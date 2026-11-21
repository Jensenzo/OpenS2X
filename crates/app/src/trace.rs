//! `s2-app trace` and `s2-app trace-diff`: write a game's trace and compare
//! two traces (`docs/trace-format.md`).

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufWriter, Write};
use std::process::ExitCode;

use crate::game::Start;

pub const HEADER: &str =
    "kind,tick,id,type,x,y,fine_x,fine_y,animation,clip_time,job,state,progress";

/// `s2-app trace MAP TICKS [--seed S] [--ai ...] [--from T] [--kinds SBG]
/// [--rows-every N] [--out FILE[.gz]]`, or `trace --snapshot FILE TICKS ...`.
/// `--rows-every N`: settler and building rows only every Nth trace tick
/// (the generator's every tick), as the original's long recordings hold
/// them; `clip_time` then counts from the rows written.
pub fn run(args: &[String]) -> ExitCode {
    match write(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("trace: {e}");
            ExitCode::from(2)
        }
    }
}

/// Writes the trace `s2-app trace` writes for `args` (as `run`'s).
pub fn write(args: &[String]) -> Result<(), String> {
    // `trace --snapshot FILE TICKS`: from a game snapshot (or replay file)
    // to trace tick TICKS, from its tick on (docs/snapshot.md).
    let snapshot = (args.first().map(String::as_str) == Some("--snapshot"))
        .then(|| args.get(1).ok_or("--snapshot wants a file"))
        .transpose()?;
    let args = if snapshot.is_some() { &args[2..] } else { args };
    let (map, ticks) = match (snapshot, args) {
        (Some(_), [ticks, ..]) => (None, ticks),
        (None, [map, ticks, ..]) => (Some(map), ticks),
        _ => return Err("trace wants a map and a tick count (or --snapshot FILE TICKS)".into()),
    };
    let ticks: u64 = ticks
        .parse()
        .map_err(|_| format!("not a number: {ticks}"))?;
    let mut start = Start::new(map.map_or("", String::as_str), s2_sim::rng::SIM_SEED as u64);
    let rest = if map.is_some() {
        start.options(&args[2..])?
    } else {
        args[1..].to_vec()
    };
    let (mut from, mut out_file) = (None, None);
    let mut kinds = "SBG".to_string();
    let mut every: u64 = 1;
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--from" => {
                let v = it.next().ok_or("--from wants a tick")?;
                from = Some(v.parse().map_err(|_| format!("not a number: {v}"))?);
            }
            "--kinds" => {
                kinds = it.next().ok_or("--kinds wants letters (SBG)")?.clone();
                if kinds.is_empty() || !kinds.chars().all(|c| "SBG".contains(c)) {
                    return Err(format!("--kinds {kinds}: letters of S, B and G"));
                }
            }
            "--rows-every" => {
                let v = it.next().ok_or("--rows-every wants a number")?;
                every = v.parse().map_err(|_| format!("not a number: {v}"))?;
                if every == 0 {
                    return Err("--rows-every wants a number above 0".into());
                }
            }
            "--out" => out_file = Some(it.next().ok_or("--out wants a file")?.clone()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let mut runner = match snapshot {
        Some(file) => {
            let (from_file, runner) = crate::game::load_game(file)?;
            start = from_file;
            runner
        }
        None => start.begin()?,
    };
    // Without --from: the ticks after the start (tick 0 or the snapshot's).
    let from = from.unwrap_or(runner.world.tick() + 1);
    // A file ending in `.gz` is written gzip compressed (no time stamp in
    // the header, so the same trace gives the same bytes).
    let out: Box<dyn Write> = match &out_file {
        Some(f) => {
            let file = std::fs::File::create(f).map_err(|e| format!("{f}: {e}"))?;
            if f.ends_with(".gz") {
                Box::new(
                    flate2::GzBuilder::new()
                        .mtime(0)
                        .write(file, flate2::Compression::best()),
                )
            } else {
                Box::new(file)
            }
        }
        None => Box::new(std::io::stdout().lock()),
    };
    let mut out = BufWriter::new(out);
    let io = |e: std::io::Error| e.to_string();
    writeln!(out, "# s2-trace 1").map_err(io)?;
    let ai = match start.ai_text() {
        all if all.is_empty() => "all".to_string(),
        ai => ai,
    };
    let closed = if start.closed.is_empty() {
        String::new()
    } else {
        format!(" closed {}", start.closed_text())
    };
    writeln!(
        out,
        "# s2-reborn {} seed {} resources {} ai {ai}{closed}",
        start.map, start.seed, start.resources,
    )
    .map_err(io)?;
    // TOOL-trace TRACE-4.5 (Q-314 a): the ticks are trace ticks.
    writeln!(
        out,
        "# ticks: trace ticks (TOOL-trace TRACE-4.5): 0 the world as set up, n after the n-th update (the original's counter n - 1)"
    )
    .map_err(io)?;
    // Per settler: the animation and the tick it started.
    let mut clips: HashMap<String, (String, u64)> = HashMap::new();
    if snapshot.is_some() {
        writeln!(
            out,
            "# from a snapshot at tick {}: clip_time counts from there",
            runner.world.tick()
        )
        .map_err(io)?;
    }
    writeln!(out, "{HEADER}").map_err(io)?;
    // TOOL-trace TRACE-4.5: tick 0 is the world as set up (or the
    // snapshot's tick, the world as loaded).
    write_tick(&mut out, &runner.world, &mut clips, from, &kinds, every).map_err(io)?;
    while runner.world.tick() < ticks {
        runner.step();
        write_tick(&mut out, &runner.world, &mut clips, from, &kinds, every).map_err(io)?;
    }
    out.flush().map_err(io)?;
    // The gzip trailer is written when the encoder is dropped.
    drop(out.into_inner().map_err(|e| e.to_string())?);
    Ok(())
}

/// The rows of the world's current tick: settlers, buildings, then the
/// generator (`docs/trace-format.md`), of the kinds asked for (`S`, `B`,
/// `G`); ticks before `from` only keep the settlers' clip clocks.
fn write_tick(
    out: &mut impl Write,
    world: &s2_sim::World,
    clips: &mut HashMap<String, (String, u64)>,
    from: u64,
    kinds: &str,
    every: u64,
) -> std::io::Result<()> {
    // TOOL-trace TRACE-4.2: type ids in 8 lower-case hex digits; the rules
    // carry the original's own ids.
    let name = |id: u32| format!("{id:08x}");
    let tick = world.tick();
    // `--rows-every`: the settler and building rows of this tick, or none.
    let rows = tick.is_multiple_of(every);
    let kinds: String = kinds.chars().filter(|&k| rows || k == 'G').collect();
    // The settlers' rows and clip clocks serve the settler rows alone.
    let settlers = if kinds.contains('S') {
        world.trace_settlers()
    } else {
        Vec::new()
    };
    let mut seen: HashMap<String, (String, u64)> = HashMap::with_capacity(settlers.len());
    for row in &settlers {
        let animation = row.action.name();
        let started = match clips.get(&row.id) {
            Some((a, t)) if *a == animation => *t,
            _ => tick,
        };
        seen.insert(row.id.clone(), (animation.clone(), started));
        if tick < from {
            continue;
        }
        writeln!(
            out,
            "S,{tick},{},{},{},{},{},{},{animation},{},{},,",
            row.id,
            name(row.profession),
            row.field.0,
            row.field.1,
            row.fine.0,
            row.fine.1,
            tick - started,
            row.job
        )?;
    }
    if rows {
        *clips = seen;
    }
    if tick < from {
        return Ok(());
    }
    let buildings = if kinds.contains('B') {
        world.trace_buildings()
    } else {
        Vec::new()
    };
    for b in &buildings {
        writeln!(
            out,
            "B,{tick},,{},{},{},,,,,,{},{}",
            name(b.building_type),
            b.field.0,
            b.field.1,
            b.state,
            b.progress
        )?;
    }
    // SIM-RNG, TOOL-trace TRACE-4.6: the generator at the end of the tick,
    // its state and the draws since the seed.
    let rng = world.rng();
    if !kinds.contains('G') {
        return Ok(());
    }
    writeln!(out, "G,{tick},rng,,,,,,,,,{},{}", rng.state(), rng.draws())
}

/// `s2-app rng MAP TICKS [--seed S] [--ai ...]`: per trace tick from 0 (the
/// world as set up, TOOL-trace TRACE-4.5), the simulation generator's draw
/// index (draws since the seed), the draws in the tick, and the places in
/// the simulation that drew, with their counts (`docs/rng-diff.md`).
pub fn run_draws(args: &[String]) -> ExitCode {
    match draws(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rng: {e}");
            ExitCode::from(2)
        }
    }
}

fn draws(args: &[String]) -> Result<(), String> {
    let (Some(map), Some(ticks)) = (args.first(), args.get(1)) else {
        return Err("rng wants a map and a tick count".into());
    };
    let ticks: u64 = ticks
        .parse()
        .map_err(|_| format!("not a number: {ticks}"))?;
    let mut start = Start::new(map, s2_sim::rng::SIM_SEED as u64);
    let rest = start.options(&args[2..])?;
    if let Some(other) = rest.first() {
        return Err(format!("unknown argument {other}"));
    }
    let mut runner = start.begin()?;
    runner.world.record_draws();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let io = |e: std::io::Error| e.to_string();
    writeln!(
        out,
        "# s2-rng 1: s2-reborn {} seed {}",
        start.map, start.seed
    )
    .map_err(io)?;
    writeln!(out, "trace_tick,draws,in_tick,places").map_err(io)?;
    let mut before = runner.world.rng().draws();
    writeln!(out, "{},{before},0,", runner.world.tick()).map_err(io)?;
    for _ in 0..ticks {
        runner.step();
        let world = &mut runner.world;
        let mut places: BTreeMap<String, u32> = BTreeMap::new();
        for place in world.take_draw_record() {
            let short = place.rsplit_once("src/").map_or(place.as_str(), |(_, p)| p);
            *places.entry(short.to_string()).or_default() += 1;
        }
        let places: Vec<String> = places.iter().map(|(p, n)| format!("{p} x{n}")).collect();
        let draws = world.rng().draws();
        writeln!(
            out,
            "{},{draws},{},{}",
            world.tick(),
            draws - before,
            places.join(" ")
        )
        .map_err(io)?;
        before = draws;
    }
    out.flush().map_err(io)
}

/// One data row, split into its columns.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    fields: Vec<String>,
}

const COLUMNS: [&str; 13] = [
    "kind",
    "tick",
    "id",
    "type",
    "x",
    "y",
    "fine_x",
    "fine_y",
    "animation",
    "clip_time",
    "job",
    "state",
    "progress",
];

fn col(name: &str) -> usize {
    COLUMNS.iter().position(|&c| c == name).expect("a column")
}

impl Row {
    fn get(&self, name: &str) -> &str {
        &self.fields[col(name)]
    }
}

/// The rows of one tick, read from a trace.
struct Reader<R: BufRead> {
    lines: std::io::Lines<R>,
    pending: Option<Row>,
    line: usize,
    name: String,
}

impl<R: BufRead> Reader<R> {
    fn new(input: R, name: &str) -> Result<Self, String> {
        let mut reader = Self {
            lines: input.lines(),
            pending: None,
            line: 0,
            name: name.to_string(),
        };
        // Comments, then the header.
        loop {
            let Some(line) = reader.next_line()? else {
                return Err(format!("{name}: no header line"));
            };
            if line.starts_with('#') {
                continue;
            }
            if line != HEADER {
                return Err(format!("{name}:{}: not the trace header", reader.line));
            }
            return Ok(reader);
        }
    }

    fn next_line(&mut self) -> Result<Option<String>, String> {
        match self.lines.next() {
            None => Ok(None),
            Some(Err(e)) => Err(format!("{}: {e}", self.name)),
            Some(Ok(l)) => {
                self.line += 1;
                Ok(Some(l.trim_end_matches('\r').to_string()))
            }
        }
    }

    fn next_row(&mut self) -> Result<Option<Row>, String> {
        if let Some(row) = self.pending.take() {
            return Ok(Some(row));
        }
        loop {
            let Some(line) = self.next_line()? else {
                return Ok(None);
            };
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<String> = line.split(',').map(str::to_string).collect();
            if fields.len() != COLUMNS.len()
                || !matches!(fields[0].as_str(), "S" | "B" | "G")
                || fields[1].parse::<u64>().is_err()
            {
                return Err(format!(
                    "{}:{}: malformed row: {line}",
                    self.name, self.line
                ));
            }
            return Ok(Some(Row { fields }));
        }
    }

    /// The next tick's rows, or `None` at the end.
    fn next_tick(&mut self) -> Result<Option<(u64, Vec<Row>)>, String> {
        let Some(first) = self.next_row()? else {
            return Ok(None);
        };
        let tick: u64 = first.get("tick").parse().expect("checked");
        let mut rows = vec![first];
        while let Some(row) = self.next_row()? {
            if row.get("tick") != rows[0].get("tick") {
                self.pending = Some(row);
                break;
            }
            rows.push(row);
        }
        Ok(Some((tick, rows)))
    }
}

/// How `trace-diff` compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffOptions {
    pub by_place: bool,
    pub settler_columns: Vec<&'static str>,
    pub building_columns: Vec<&'static str>,
    pub generator_columns: Vec<&'static str>,
    /// `--common-ticks`: compare only the ticks both traces hold (a
    /// recording that missed ticks, TOOL-trace TRACE-4.6).
    pub common_ticks: bool,
    /// `--from T`: leave out the ticks before T (the original's trace tick
    /// 0 is the world half set up, TRACE-3.8).
    pub from: u64,
    /// `--kinds SBG`: the kinds of row compared; the others are left out
    /// (a recording of the generator alone against a full trace).
    pub kinds: String,
    /// `--view-lag` (TOOL-trace TRACE-4.4, T16): the second trace is a
    /// recording whose view switches a clip one tick late now and then. A
    /// settler row differing only in its animation, the second's being the
    /// first trace's of the same settler in the tick before, is counted as
    /// that lag, not as a difference; a row whose view the recorder did not
    /// read (an empty animation, TRACE-2.7) has no animation to compare.
    pub view_lag: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            by_place: false,
            settler_columns: vec![
                "type",
                "x",
                "y",
                "fine_x",
                "fine_y",
                "animation",
                "clip_time",
            ],
            building_columns: vec!["type", "state", "progress"],
            generator_columns: vec!["state", "progress"],
            common_ticks: false,
            from: 0,
            kinds: "SBG".into(),
            view_lag: false,
        }
    }
}

/// The match key of a row.
fn key(row: &Row, by_place: bool) -> String {
    match (row.get("kind"), by_place) {
        ("B", _) => format!("B {},{}", row.get("x"), row.get("y")),
        ("G", _) => "G".to_string(),
        (_, false) => format!("S {}", row.get("id")),
        (_, true) => format!(
            "S {} {},{} {},{}",
            row.get("type"),
            row.get("x"),
            row.get("y"),
            row.get("fine_x"),
            row.get("fine_y")
        ),
    }
}

/// Per settler type (or `buildings`): rows compared and differing, the first
/// tick of a difference, and how often each column differed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    pub compared: u64,
    pub differing: u64,
    pub first: Option<u64>,
    pub columns: BTreeMap<String, u64>,
}

/// The first difference and the tallies of two traces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffReport {
    pub first: Option<String>,
    /// The tick of the first difference.
    pub first_tick: Option<u64>,
    pub tallies: BTreeMap<String, Tally>,
    pub ticks: u64,
    /// Ticks only one trace holds, left out with `--common-ticks` (and
    /// the generator of a tick one holds without its generator row).
    pub skipped: u64,
    /// Settler rows counted as the recorder's view lag (`--view-lag`).
    pub view_lag: u64,
}

/// The first trace's animation of each settler (by id) in one tick, for
/// `--view-lag`.
type Animations = BTreeMap<String, String>;

fn animations(rows: &[Row]) -> Animations {
    rows.iter()
        .filter(|r| r.get("kind") == "S")
        .map(|r| (r.get("id").to_string(), r.get("animation").to_string()))
        .collect()
}

fn compare_tick(
    tick: u64,
    mut a: Vec<Row>,
    mut b: Vec<Row>,
    before: Option<&Animations>,
    options: &DiffOptions,
    report: &mut DiffReport,
) {
    for rows in [&mut a, &mut b] {
        rows.retain(|r| options.kinds.contains(r.get("kind")));
    }
    // --common-ticks: a tick both hold, one without its generator row (the
    // recording's raced end, TRACE-2.7): the generator is not compared.
    let generator = |rows: &[Row]| rows.iter().any(|r| r.get("kind") == "G");
    if options.common_ticks && generator(&a) != generator(&b) {
        for rows in [&mut a, &mut b] {
            rows.retain(|r| r.get("kind") != "G");
        }
        report.skipped += 1;
    }
    // Rows by key; rows of one key (settlers on one place) in the order
    // of their compared columns, so two traces that list them in another
    // order pair them alike.
    let group = |rows: Vec<Row>| {
        let mut by_key: BTreeMap<String, Vec<Row>> = BTreeMap::new();
        for row in rows {
            by_key
                .entry(key(&row, options.by_place))
                .or_default()
                .push(row);
        }
        let mut m: BTreeMap<(String, usize), Row> = BTreeMap::new();
        for (k, mut rows) in by_key {
            if options.by_place && rows.len() > 1 {
                let compared = |r: &Row| -> Vec<String> {
                    options
                        .settler_columns
                        .iter()
                        .map(|c| r.get(c).to_string())
                        .collect()
                };
                rows.sort_by_key(compared);
            }
            for (n, row) in rows.into_iter().enumerate() {
                m.insert((k.clone(), n), row);
            }
        }
        m
    };
    let (a, mut b) = (group(a), group(b));
    let note = |label: String, tally_key: String, columns: Vec<String>, report: &mut DiffReport| {
        let t = report.tallies.entry(tally_key).or_default();
        t.differing += 1;
        t.first.get_or_insert(tick);
        for c in &columns {
            *t.columns.entry(c.clone()).or_default() += 1;
        }
        if report.first.is_none() {
            report.first_tick = Some(tick);
            report.first = Some(format!(
                "tick {tick}: {label}; differing: {}",
                columns.join(", ")
            ));
        }
    };
    let tally_of = |row: &Row| match row.get("kind") {
        "B" => "buildings".to_string(),
        "G" => "generator".to_string(),
        _ => row.get("type").to_string(),
    };
    for (k, ra) in a {
        let tally = tally_of(&ra);
        report.tallies.entry(tally.clone()).or_default().compared += 1;
        match b.remove(&k) {
            None => note(
                format!("{} only in the first trace: {}", k.0, ra.fields.join(",")),
                tally,
                vec!["(row)".into()],
                report,
            ),
            Some(rb) => {
                let columns = match ra.get("kind") {
                    "B" => &options.building_columns,
                    "G" => &options.generator_columns,
                    _ => &options.settler_columns,
                };
                let mut differing: Vec<String> = columns
                    .iter()
                    .filter(|c| ra.get(c) != rb.get(c))
                    .map(|c| c.to_string())
                    .collect();
                // TRACE-4.4 (T16), TRACE-2.7: the recorder's view.
                if options.view_lag && ra.get("kind") == "S" {
                    if rb.get("animation").is_empty() {
                        differing.retain(|c| c != "animation");
                    } else if differing == ["animation"]
                        && before.and_then(|p| p.get(ra.get("id"))).map(String::as_str)
                            == Some(rb.get("animation"))
                    {
                        report.view_lag += 1;
                        differing.clear();
                    }
                }
                if !differing.is_empty() {
                    note(
                        format!(
                            "{}\n  first:  {}\n  second: {}",
                            k.0,
                            ra.fields.join(","),
                            rb.fields.join(",")
                        ),
                        tally,
                        differing,
                        report,
                    );
                }
            }
        }
    }
    for ((k, _), rb) in b {
        let tally = tally_of(&rb);
        report.tallies.entry(tally.clone()).or_default().compared += 1;
        note(
            format!("{k} only in the second trace: {}", rb.fields.join(",")),
            tally,
            vec!["(row)".into()],
            report,
        );
    }
}

/// Compare two traces tick by tick.
pub fn diff(
    a: impl BufRead,
    b: impl BufRead,
    names: (&str, &str),
    options: &DiffOptions,
) -> Result<DiffReport, String> {
    let mut ra = Reader::new(a, names.0)?;
    let mut rb = Reader::new(b, names.1)?;
    let mut report = DiffReport::default();
    let (mut ta, mut tb) = (ra.next_tick()?, rb.next_tick()?);
    // --view-lag: the first trace's animations of the tick before.
    let mut before: Option<(u64, Animations)> = None;
    let lag = |t: u64, rows: &[Row], before: &mut Option<(u64, Animations)>| {
        let previous = before.take().filter(|(p, _)| p + 1 == t).map(|(_, m)| m);
        if options.view_lag {
            *before = Some((t, animations(rows)));
        }
        previous
    };
    // A tick only one trace has is compared against nothing, or left out.
    let alone = |t: u64, a: Vec<Row>, b: Vec<Row>, report: &mut DiffReport| {
        if options.common_ticks {
            report.skipped += 1;
        } else {
            compare_tick(t, a, b, None, options, report);
        }
    };
    loop {
        // Ticks before `from` are read past.
        while ta.as_ref().is_some_and(|(t, _)| *t < options.from) {
            ta = ra.next_tick()?;
        }
        while tb.as_ref().is_some_and(|(t, _)| *t < options.from) {
            tb = rb.next_tick()?;
        }
        match (ta.take(), tb.take()) {
            (None, None) => break,
            (Some((t, rows)), None) => {
                lag(t, &rows, &mut before);
                alone(t, rows, Vec::new(), &mut report);
                ta = ra.next_tick()?;
            }
            (None, Some((t, rows))) => {
                alone(t, Vec::new(), rows, &mut report);
                tb = rb.next_tick()?;
            }
            (Some((t, x)), Some((u, y))) => {
                if t < u {
                    lag(t, &x, &mut before);
                    alone(t, x, Vec::new(), &mut report);
                    ta = ra.next_tick()?;
                    tb = Some((u, y));
                } else if u < t {
                    alone(u, Vec::new(), y, &mut report);
                    ta = Some((t, x));
                    tb = rb.next_tick()?;
                } else {
                    let previous = lag(t, &x, &mut before);
                    compare_tick(t, x, y, previous.as_ref(), options, &mut report);
                    ta = ra.next_tick()?;
                    tb = rb.next_tick()?;
                }
                report.ticks += 1;
            }
        }
    }
    Ok(report)
}

/// A trace file for reading; one in gzip (`specs/traces/*.gz`) is
/// decompressed on the way.
pub fn open(file: &str) -> Result<Box<dyn BufRead>, String> {
    let mut input =
        std::io::BufReader::new(std::fs::File::open(file).map_err(|e| format!("{file}: {e}"))?);
    let gzip = input
        .fill_buf()
        .map_err(|e| format!("{file}: {e}"))?
        .starts_with(&[0x1f, 0x8b]);
    Ok(if gzip {
        Box::new(std::io::BufReader::new(flate2::bufread::GzDecoder::new(
            input,
        )))
    } else {
        Box::new(input)
    })
}

/// `s2-app trace-diff A B [--by-place] [--common-ticks] [--view-lag] [--from T] [--ignore c,c] [--with c,c]`.
pub fn run_diff(args: &[String]) -> ExitCode {
    let (Some(a), Some(b)) = (args.first(), args.get(1)) else {
        eprintln!("trace-diff wants two trace files");
        return ExitCode::from(2);
    };
    let mut options = DiffOptions::default();
    let mut it = args[2..].iter();
    let columns = |list: &str| -> Result<Vec<&'static str>, String> {
        list.split(',')
            .map(|c| {
                COLUMNS
                    .iter()
                    .find(|&&k| k == c)
                    .copied()
                    .ok_or(format!("no column {c}"))
            })
            .collect()
    };
    while let Some(arg) = it.next() {
        let result = match arg.as_str() {
            "--by-place" => {
                options.by_place = true;
                Ok(())
            }
            "--common-ticks" => {
                options.common_ticks = true;
                Ok(())
            }
            "--view-lag" => {
                options.view_lag = true;
                Ok(())
            }
            "--from" => it
                .next()
                .and_then(|v| v.parse().ok())
                .map(|v| options.from = v)
                .ok_or("--from wants a tick".to_string()),
            "--kinds" => it
                .next()
                .filter(|k| !k.is_empty() && k.chars().all(|c| "SBG".contains(c)))
                .map(|k| options.kinds = k.clone())
                .ok_or("--kinds wants letters of S, B and G".to_string()),
            "--ignore" => it
                .next()
                .ok_or("--ignore wants columns".to_string())
                .and_then(|l| {
                    let drop = columns(l)?;
                    options.settler_columns.retain(|c| !drop.contains(c));
                    options.building_columns.retain(|c| !drop.contains(c));
                    options.generator_columns.retain(|c| !drop.contains(c));
                    Ok(())
                }),
            "--with" => it
                .next()
                .ok_or("--with wants columns".to_string())
                .and_then(|l| {
                    for c in columns(l)? {
                        if !options.settler_columns.contains(&c) {
                            options.settler_columns.push(c);
                        }
                    }
                    Ok(())
                }),
            other => Err(format!("unknown argument {other}")),
        };
        if let Err(e) = result {
            eprintln!("trace-diff: {e}");
            return ExitCode::from(2);
        }
    }
    let report = open(a)
        .and_then(|x| Ok((x, open(b)?)))
        .and_then(|(x, y)| diff(x, y, (a, b), &options));
    let report = match report {
        Ok(r) => r,
        Err(e) => {
            eprintln!("trace-diff: {e}");
            return ExitCode::from(2);
        }
    };
    match &report.first {
        None => println!("the traces are the same ({} ticks)", report.ticks),
        Some(first) => println!("first difference: {first}"),
    }
    if report.view_lag > 0 {
        println!(
            "{} settler rows counted as the recorder's view lag (--view-lag)",
            report.view_lag
        );
    }
    if report.skipped > 0 {
        println!(
            "{} ticks only one trace holds were left out (--common-ticks)",
            report.skipped
        );
    }
    println!("by settler type (and buildings): rows compared, differing, first tick, most differing column");
    for (kind, t) in &report.tallies {
        let worst = t
            .columns
            .iter()
            .max_by_key(|(_, &n)| n)
            .map_or("-".to_string(), |(c, n)| format!("{c} ({n})"));
        println!(
            "  {kind}: {}, {}, {}, {worst}",
            t.compared,
            t.differing,
            t.first.map_or("-".to_string(), |f| f.to_string())
        );
    }
    if report.first.is_none() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(rows: &[&str]) -> String {
        format!("# s2-trace 1\n{HEADER}\n{}\n", rows.join("\n"))
    }

    fn run(a: &str, b: &str, options: &DiffOptions) -> DiffReport {
        diff(a.as_bytes(), b.as_bytes(), ("a", "b"), options).unwrap()
    }

    const S1: &str = "S,1,s0,2c50533e,3,4,12000,16000,Walk,0,Worker.Out,,";
    const S2: &str = "S,2,s0,2c50533e,3,4,12500,16000,Walk,1,Worker.Out,,";
    const B1: &str = "B,1,,5a926fa3,5,5,,,,,,frame,120";
    const B2: &str = "B,2,,5a926fa3,5,5,,,,,,frame,130";

    // TOOL-trace TRACE-4.6: a recording that missed a tick; with
    // --common-ticks only the ticks both hold are compared.
    // spec: TOOL-trace TRACE-4.6
    #[test]
    fn common_ticks_leave_out_a_missed_tick() {
        let full = trace(&[S1, B1, S2, B2]);
        let missed = trace(&[S1, B1]);
        assert!(run(&full, &missed, &DiffOptions::default()).first.is_some());
        let options = DiffOptions {
            common_ticks: true,
            ..DiffOptions::default()
        };
        let r = run(&full, &missed, &options);
        assert_eq!((r.first, r.ticks, r.skipped), (None, 1, 1));
    }

    // TOOL-trace TRACE-4.4 (T16): the recorder's view switches a clip one
    // tick late now and then. With --view-lag a settler row that differs
    // only in its animation, the second trace's being the first's of the
    // tick before, is counted as the view lag and not as a difference; any
    // other animation difference still is one.
    // spec: TOOL-trace TRACE-4.4 (T16)
    #[test]
    fn view_lag_counts_a_clip_one_tick_late() {
        let ours = trace(&[
            "S,1,s0,7bff15ce,3,4,12000,16000,Walk,0,Carrier.Out,,",
            "S,2,s0,7bff15ce,3,4,12000,16000,Stand,0,Carrier.Idle,,",
            "S,3,s0,7bff15ce,3,4,12000,16000,PutDown,0,Carrier.PutDown,,",
        ]);
        let original = trace(&[
            "S,1,s0,7bff15ce,3,4,12000,16000,Walk,0,carriers.28,,",
            "S,2,s0,7bff15ce,3,4,12000,16000,Walk,1,carriers.28,,",
            "S,3,s0,7bff15ce,3,4,12000,16000,Stand,0,carriers.31,,",
        ]);
        // As `s2-app fidelity` compares: no clip_time (the original has none).
        let mut plain = DiffOptions {
            by_place: true,
            ..DiffOptions::default()
        };
        plain.settler_columns.retain(|&c| c != "clip_time");
        assert_eq!(run(&ours, &original, &plain).first_tick, Some(2));
        let lag = DiffOptions {
            view_lag: true,
            ..plain.clone()
        };
        let r = run(&ours, &original, &lag);
        assert_eq!((r.first, r.view_lag), (None, 2));
        // Not the clip of the tick before: a difference.
        let other = trace(&[
            "S,1,s0,7bff15ce,3,4,12000,16000,Walk,0,carriers.28,,",
            "S,2,s0,7bff15ce,3,4,12000,16000,Hit,1,carriers.28,,",
            "S,3,s0,7bff15ce,3,4,12000,16000,PutDown,0,carriers.31,,",
        ]);
        assert_eq!(run(&ours, &other, &lag).first_tick, Some(2));
    }

    // TOOL-trace TRACE-4.4 (T16), TRACE-2.7: a row whose view the recorder
    // did not read (an empty animation, the one-player recording's raced
    // end) has no animation to compare; and with --common-ticks a tick one
    // trace holds without its generator row leaves the generator out there.
    // spec: TOOL-trace TRACE-4.4 (T16), TOOL-trace TRACE-2.7
    #[test]
    fn an_unread_view_and_a_missing_generator_row_are_left_out() {
        let ours = trace(&[
            "S,1,s0,15e86843,3,4,12000,16000,Stand,0,Garrison,,",
            "G,1,rng,,,,,,,,,1412607200,7",
        ]);
        let original = trace(&["S,1,s0,15e86843,3,4,12000,16000,,,soldiers.12,,"]);
        let mut options = DiffOptions {
            by_place: true,
            common_ticks: true,
            view_lag: true,
            ..DiffOptions::default()
        };
        options.settler_columns.retain(|&c| c != "clip_time");
        let r = run(&ours, &original, &options);
        assert_eq!(r.first, None, "{:?}", r.first);
        assert_eq!(r.skipped, 1, "the generator row left out");
        // Without --view-lag the empty view is a difference.
        let options = DiffOptions {
            view_lag: false,
            ..options
        };
        assert!(run(&ours, &original, &options).first.is_some());
    }

    // A full trace against a recording of the generator alone: with
    // --kinds G the other rows are left out.
    // spec: TOOL-trace TRACE-4.6
    #[test]
    fn kinds_leave_out_the_other_rows() {
        let full = trace(&[S1, B1, "G,1,rng,,,,,,,,,1412607200,0"]);
        let generator = trace(&["G,1,rng,,,,,,,,,1412607200,0"]);
        assert!(run(&full, &generator, &DiffOptions::default())
            .first
            .is_some());
        let options = DiffOptions {
            kinds: "G".into(),
            ..DiffOptions::default()
        };
        let r = run(&full, &generator, &options);
        assert_eq!(r.first, None);
        assert_eq!(r.tallies["generator"].compared, 1);
    }

    // TOOL-trace TRACE-4.6, SIM-RNG-2.5: the generator rows are compared by
    // state and draw index; the first tick whose draws differ is named.
    // spec: SIM-RNG-2.5, TOOL-trace TRACE-4.6
    #[test]
    fn generator_rows_are_compared() {
        let ours = trace(&[
            B1,
            "G,1,rng,,,,,,,,,1412607200,0",
            B2,
            "G,2,rng,,,,,,,,,1054465454,3",
        ]);
        let theirs = trace(&[
            B1,
            "G,1,rng,,,,,,,,,1412607200,0",
            B2,
            "G,2,rng,,,,,,,,,1365830334,4",
        ]);
        let r = run(&ours, &theirs, &DiffOptions::default());
        assert!(
            r.first.as_deref().unwrap().starts_with("tick 2: G"),
            "{:?}",
            r.first
        );
        assert_eq!(r.tallies["generator"].compared, 2);
        assert_eq!(r.tallies["generator"].differing, 1);
        assert_eq!(r.tallies["generator"].columns["progress"], 1);
        assert_eq!(run(&ours, &ours, &DiffOptions::default()).first, None);
    }

    // spec: none (trace-diff, docs/trace-format.md)
    #[test]
    fn the_same_trace_has_no_difference() {
        let t = trace(&[S1, B1, S2, B2]);
        let r = run(&t, &t, &DiffOptions::default());
        assert_eq!(r.first, None);
        assert_eq!(r.ticks, 2);
        assert_eq!(r.tallies["2c50533e"].compared, 2);
    }

    // spec: none (trace-diff, docs/trace-format.md)
    #[test]
    fn the_first_difference_names_tick_row_and_columns() {
        let a = trace(&[S1, B1, S2, B2]);
        let changed = S2.replace("12500", "12600").replace("Walk,1", "Stand,0");
        let b = trace(&[S1, B1, &changed, B2]);
        let r = run(&a, &b, &DiffOptions::default());
        let first = r.first.unwrap();
        assert!(first.starts_with("tick 2: S s0"), "{first}");
        assert!(
            first.ends_with("differing: fine_x, animation, clip_time"),
            "{first}"
        );
        let t = &r.tallies["2c50533e"];
        assert_eq!((t.differing, t.first), (1, Some(2)));
        // Ignored columns do not count.
        let mut options = DiffOptions::default();
        options
            .settler_columns
            .retain(|c| !["fine_x", "animation", "clip_time"].contains(c));
        assert_eq!(run(&a, &b, &options).first, None);
    }

    // spec: none (trace-diff, docs/trace-format.md)
    #[test]
    fn rows_only_one_trace_has_differ() {
        let a = trace(&[S1, B1]);
        let b = trace(&[B1]);
        let r = run(&a, &b, &DiffOptions::default());
        assert!(r.first.unwrap().contains("only in the first trace"));
    }

    // spec: none (trace-diff, docs/trace-format.md)
    #[test]
    fn by_place_matches_settlers_whose_ids_differ() {
        let a = trace(&[S1]);
        let b = trace(&[&S1.replace("s0", "7713")]);
        assert!(run(&a, &b, &DiffOptions::default()).first.is_some());
        let options = DiffOptions {
            by_place: true,
            ..DiffOptions::default()
        };
        assert_eq!(run(&a, &b, &options).first, None);
    }

    // Two settlers on one place: their rows pair by what is compared, so
    // the order the traces list them in does not matter.
    // spec: none (trace-diff, docs/trace-format.md)
    #[test]
    fn by_place_pairs_settlers_on_one_place_in_any_order() {
        let walk = "S,1,90204,7bff15ce,3,4,12000,16000,PutDown,3,carriers.30,,";
        let stand = "S,1,90209,7bff15ce,3,4,12000,16000,Pickup,0,carriers.31,,";
        let options = DiffOptions {
            by_place: true,
            ..DiffOptions::default()
        };
        let a = trace(&[walk, stand]);
        let b = trace(&[stand, walk]);
        assert_eq!(run(&a, &b, &options).first, None);
        let other = "S,1,90209,7bff15ce,3,4,12000,16000,Pickup,1,carriers.31,,";
        let c = trace(&[other, walk]);
        assert!(run(&a, &c, &options).first.is_some());
    }

    // spec: none (trace-diff, docs/trace-format.md)
    #[test]
    fn a_malformed_row_is_an_error() {
        let bad = trace(&["S,1,s0"]);
        let good = trace(&[S1]);
        assert!(diff(
            bad.as_bytes(),
            good.as_bytes(),
            ("a", "b"),
            &DiffOptions::default()
        )
        .is_err());
        assert!(diff(
            "x\n".as_bytes(),
            good.as_bytes(),
            ("a", "b"),
            &DiffOptions::default()
        )
        .is_err());
    }
}
