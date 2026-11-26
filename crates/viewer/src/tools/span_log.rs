//! The investigation build's span log (`--features frame-trace`, with
//! `--bench-log <file>`: `<file>.spans`): every system's run (Bevy's trace
//! spans) and every wgpu call wgpu times (its profiling scopes) is timed
//! from enter to exit, and one that took [`SPAN_LOG_MS`] or more is a line:
//!
//!   Y <unix ms at its end> <ms> <thread id> <name>
//!
//! and a system whose wgpu calls add up to [`CALLS_LOG_MS`] or more:
//!
//!   W <unix ms> <the system's ms> <thread id> system <name>: <call> <n> calls <total ms> (longest <ms>), ...
//!
//! It names the call a stalled render step waited in (2026-11-24: the
//! "bind groups" step, 109..130 ms, with almost no CPU of ours; a frame at
//! the refresh is 16.7 ms, so the spans that hold a whole frame stay out).
//! Not in the game's build: the spans cost a little in every frame.

use std::borrow::Cow;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::{span, Subscriber};
use bevy::log::tracing_subscriber::layer::Context;
use bevy::log::tracing_subscriber::registry::LookupSpan;
use bevy::log::tracing_subscriber::Layer;

/// A span this long or longer is written.
pub(crate) const SPAN_LOG_MS: f64 = 25.0;

static LOG: OnceLock<Mutex<std::io::BufWriter<std::fs::File>>> = OnceLock::new();

/// Opens the span log beside the bench log.
pub(crate) fn open(bench_log: &std::path::Path) {
    let path = format!("{}.spans", bench_log.display());
    match std::fs::File::create(&path) {
        Ok(f) => {
            let _ = LOG.set(Mutex::new(std::io::BufWriter::new(f)));
        }
        Err(e) => eprintln!("span log {path}: {e}"),
    }
}

/// The log filter with wgpu's own scopes at info (Bevy's default keeps
/// wgpu at errors; the more specific directive wins).
pub(crate) fn filter() -> String {
    format!("{},wgpu_core=info", bevy::log::DEFAULT_FILTER)
}

/// The log's layers: the game's log file and the span times.
pub(crate) fn layers(app: &mut bevy::app::App) -> Option<bevy::log::BoxedLayer> {
    let mut all: Vec<bevy::log::BoxedLayer> = Vec::new();
    if let Some(file) = crate::app::version::file_layer(app) {
        all.push(file);
    }
    all.push(Box::new(SpanTimes));
    Some(Box::new(all))
}

/// The names of the spans that carry one in a field (Bevy's system spans,
/// made once per system): by span id.
static NAMES: OnceLock<Mutex<std::collections::HashMap<u64, String>>> = OnceLock::new();

/// A wgpu call's sum inside the system running on this thread.
#[derive(Clone, Copy)]
struct Call {
    name: &'static str,
    count: u32,
    total_ms: f64,
    longest_ms: f64,
}

thread_local! {
    /// The spans entered on this thread and not yet left: id and when.
    static ENTERED: std::cell::RefCell<Vec<(u64, Instant)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// The wgpu calls inside the system now running on this thread.
    static CALLS: std::cell::RefCell<Vec<Call>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A system's wgpu calls are written when they add up to this or more
/// (`W` lines).
pub(crate) const CALLS_LOG_MS: f64 = 5.0;

struct NameField<'a>(&'a mut Option<String>);

impl Visit for NameField<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "name" {
            *self.0 = Some(value.to_owned());
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            *self.0 = Some(format!("{value:?}"));
        }
    }
}

/// The layer that times the spans.
pub(crate) struct SpanTimes;

fn write(line: std::fmt::Arguments) {
    let Some(log) = LOG.get() else { return };
    if let Ok(mut log) = log.lock() {
        let _ = log.write_fmt(line);
        let _ = log.flush();
    }
}

fn unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for SpanTimes {
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, _: Context<'_, S>) {
        if attrs.fields().field("name").is_none() {
            return;
        }
        let mut field = None;
        attrs.record(&mut NameField(&mut field));
        if let Some(f) = field {
            let names = NAMES.get_or_init(Default::default);
            if let Ok(mut names) = names.lock() {
                names.insert(id.into_u64(), format!("{} {f}", attrs.metadata().name()));
            }
        }
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let system = ctx.metadata(id).is_some_and(|m| m.name() == "system");
        ENTERED.with(|e| e.borrow_mut().push((id.into_u64(), Instant::now())));
        if system {
            CALLS.with(|c| c.borrow_mut().clear());
        }
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let raw = id.into_u64();
        let Some(at) = ENTERED.with(|e| {
            let mut e = e.borrow_mut();
            let i = e.iter().rposition(|(i, _)| *i == raw)?;
            Some(e.remove(i).1)
        }) else {
            return;
        };
        let ms = (Instant::now() - at).as_secs_f64() * 1000.0;
        let Some(meta) = ctx.metadata(id) else { return };
        let wgpu = meta.target().starts_with("wgpu");
        if wgpu {
            CALLS.with(|c| {
                let mut c = c.borrow_mut();
                match c.iter_mut().find(|k| std::ptr::eq(k.name, meta.name())) {
                    Some(k) => {
                        k.count += 1;
                        k.total_ms += ms;
                        k.longest_ms = k.longest_ms.max(ms);
                    }
                    None => c.push(Call {
                        name: meta.name(),
                        count: 1,
                        total_ms: ms,
                        longest_ms: ms,
                    }),
                }
            });
        }
        let system = meta.name() == "system";
        let name = || -> Cow<'static, str> {
            NAMES
                .get()
                .and_then(|n| n.lock().ok()?.get(&raw).cloned())
                .map_or(Cow::Borrowed(meta.name()), Cow::Owned)
        };
        if system {
            let calls = CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()));
            let sum: f64 = calls.iter().map(|c| c.total_ms).sum();
            if sum >= CALLS_LOG_MS {
                let mut line = format!("W {} {ms:.1} {} {}:", unix_ms(), thread_id(), name());
                for c in &calls {
                    line += &format!(
                        " {} {} calls {:.1} ms (longest {:.2}),",
                        c.name, c.count, c.total_ms, c.longest_ms
                    );
                }
                line.pop();
                write(format_args!("{line}\n"));
            }
        }
        if ms >= SPAN_LOG_MS {
            write(format_args!(
                "Y {} {ms:.1} {} {}\n",
                unix_ms(),
                thread_id(),
                name()
            ));
        }
    }
}

#[cfg(windows)]
fn thread_id() -> u32 {
    // SAFETY: a plain call.
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
}

#[cfg(not(windows))]
fn thread_id() -> u32 {
    0
}
