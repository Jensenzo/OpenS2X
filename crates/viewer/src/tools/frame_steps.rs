//! The long play's render steps (`--bench-log`, docs/test-coverage.md): the
//! render world's frame cut into its steps, each with its wall time, the CPU
//! time of the thread that ran its end mark (when its start mark ran on the
//! same thread), the whole process's CPU time, its kernel share, and the
//! process's page faults; and per frame the working set. Kept for the last
//! few frames and written only beside a frame over 50 ms (an `R` line per
//! render frame), so it costs a few system calls a mark and nothing in the
//! log of a smooth run.
//!
//! What it is for (2026-11-24): a cluster of frames of 150..280 ms, almost
//! all "render prepare", about 0.26 s after Windows' licence service starts.
//! The four readings tell apart: memory taken away from us (page faults in
//! the slow step, the working set smaller before it), the graphics stack
//! stalling (the time in submit or present, or in a step that calls the
//! driver, with little CPU of our own), a call of ours that blocks (kernel
//! time or a wait inside one step), and our thread not running at all (wall
//! time far above the thread's CPU time, no faults).

use std::collections::VecDeque;
use std::time::Instant;

/// The steps between the marks of [`step_mark`]: mark N ends step N - 1;
/// mark 0 ends the gap since the last frame's end (step 0's own time is the
/// render thread's wait for the main world and the extract).
pub(crate) const STEPS: [&str; 15] = [
    "wait and extract",
    "assets",
    "views, specialize, acquire",
    "queue and sort",
    "prepare resources",
    "batch phases",
    "write phase buffers",
    "collect phase buffers",
    "flush",
    "bind groups",
    "render system start",
    "encode",
    "submit",
    "finish and present",
    "cleanup",
];

/// Render frames kept for the log.
const KEPT: usize = 4;

/// One reading of the clocks and counters at a mark.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Sample {
    pub(crate) thread: u32,
    /// The thread's CPU cycles (QueryThreadCycleTime).
    pub(crate) thread_cycles: u64,
    /// The thread's kernel time, 100 ns (GetThreadTimes; the clock tick's
    /// resolution, about 15.6 ms).
    pub(crate) thread_kernel: u64,
    pub(crate) process_cycles: u64,
    pub(crate) process_kernel: u64,
    pub(crate) faults: u32,
    pub(crate) working_set: usize,
}

/// A step's numbers, from two samples.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Step {
    pub(crate) wall_ms: f64,
    /// The thread's CPU, ms (`None`: the step's marks ran on two threads).
    pub(crate) thread_ms: Option<f64>,
    pub(crate) thread_kernel_ms: Option<f64>,
    pub(crate) process_ms: f64,
    pub(crate) process_kernel_ms: f64,
    pub(crate) faults: u32,
}

/// One render frame's steps.
#[derive(Clone, Debug, Default)]
pub(crate) struct RenderFrame {
    pub(crate) number: u64,
    pub(crate) unix_ms: u128,
    pub(crate) steps: Vec<(usize, Step)>,
    /// The working set at the frame's start and end, MB.
    pub(crate) working_set_mb: (f64, f64),
}

/// The render world's steps, shared with the main world's bench.
#[derive(Default)]
pub(crate) struct Steps {
    last: Option<(usize, Instant, Sample)>,
    now: Option<RenderFrame>,
    start_set: f64,
    pub(crate) kept: VecDeque<RenderFrame>,
    number: u64,
    /// The time stamp counter against the wall clock, for the cycle counts.
    tsc_at: Option<(u64, Instant)>,
    cycles_per_ms: f64,
    /// The last render frame written to the log.
    pub(crate) logged: u64,
}

/// The time stamp counter (the cycle counts' unit on Windows).
fn tsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: rdtsc reads a counter, nothing else.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

#[cfg(windows)]
pub(crate) fn sample() -> Sample {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentThread, GetCurrentThreadId, GetProcessTimes, GetThreadTimes,
    };
    use windows_sys::Win32::System::WindowsProgramming::{
        QueryProcessCycleTime, QueryThreadCycleTime,
    };
    let ft = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut s = Sample::default();
    // SAFETY: the pseudo handles of this thread and process, and out
    // pointers to locals of the right types.
    unsafe {
        let (thread, process) = (GetCurrentThread(), GetCurrentProcess());
        s.thread = GetCurrentThreadId();
        QueryThreadCycleTime(thread, &raw mut s.thread_cycles);
        QueryProcessCycleTime(process, &raw mut s.process_cycles);
        let (mut a, mut b, mut k, mut u) = (zero, zero, zero, zero);
        if GetThreadTimes(thread, &raw mut a, &raw mut b, &raw mut k, &raw mut u) != 0 {
            s.thread_kernel = ft(k);
        }
        if GetProcessTimes(process, &raw mut a, &raw mut b, &raw mut k, &raw mut u) != 0 {
            s.process_kernel = ft(k);
        }
        let mut c: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        if K32GetProcessMemoryInfo(process, (&raw mut c).cast(), c.cb) != 0 {
            s.faults = c.PageFaultCount;
            s.working_set = c.WorkingSetSize;
        }
    }
    s
}

#[cfg(not(windows))]
pub(crate) fn sample() -> Sample {
    Sample::default()
}

/// The step between two samples (`cycles_per_ms` 0: the cycle counts not
/// yet calibrated, their times 0).
pub(crate) fn step(a: &Sample, b: &Sample, wall_ms: f64, cycles_per_ms: f64) -> Step {
    let ms = |cycles: u64| {
        if cycles_per_ms > 0.0 {
            cycles as f64 / cycles_per_ms
        } else {
            0.0
        }
    };
    let same = a.thread == b.thread;
    Step {
        wall_ms,
        thread_ms: same.then(|| ms(b.thread_cycles.saturating_sub(a.thread_cycles))),
        thread_kernel_ms: same
            .then(|| b.thread_kernel.saturating_sub(a.thread_kernel) as f64 / 10_000.0),
        process_ms: ms(b.process_cycles.saturating_sub(a.process_cycles)),
        process_kernel_ms: b.process_kernel.saturating_sub(a.process_kernel) as f64 / 10_000.0,
        faults: b.faults.wrapping_sub(a.faults),
    }
}

const MB: f64 = 1024.0 * 1024.0;

impl Steps {
    /// Mark `n` (0..=STEPS.len() - 1; 0 starts a frame and ends the last).
    pub(crate) fn mark(&mut self, n: usize) {
        let now = Instant::now();
        let s = sample();
        let tsc_now = tsc();
        match self.tsc_at {
            None => self.tsc_at = Some((tsc_now, now)),
            Some((t0, at)) => {
                let ms = (now - at).as_secs_f64() * 1000.0;
                if ms >= 500.0 {
                    self.cycles_per_ms = tsc_now.saturating_sub(t0) as f64 / ms;
                }
            }
        }
        if let Some((m, at, before)) = self.last {
            let wall = (now - at).as_secs_f64() * 1000.0;
            // Mark n ends step n (a skipped mark's time goes to the next);
            // mark 0 ends the frame and step 0, the gap before the next.
            let st = step(&before, &s, wall, self.cycles_per_ms);
            if n == 0 {
                if let Some(mut frame) = self.now.take() {
                    frame.working_set_mb.1 = before.working_set as f64 / MB;
                    if self.kept.len() == KEPT {
                        self.kept.pop_front();
                    }
                    self.kept.push_back(frame);
                }
                self.number += 1;
                self.start_set = s.working_set as f64 / MB;
                self.now = Some(RenderFrame {
                    number: self.number,
                    unix_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_millis()),
                    steps: vec![(0, st)],
                    working_set_mb: (self.start_set, 0.0),
                });
            } else if m < n {
                if let Some(frame) = self.now.as_mut() {
                    frame.steps.push((n, st));
                }
            }
        } else if n == 0 {
            self.number += 1;
            self.now = Some(RenderFrame {
                number: self.number,
                unix_ms: 0,
                steps: Vec::new(),
                working_set_mb: (s.working_set as f64 / MB, 0.0),
            });
        }
        self.last = Some((n, now, s));
    }

    /// The kept frames not yet written, as `R` lines, and marked written.
    pub(crate) fn unlogged_lines(&mut self, at: f64) -> Vec<String> {
        let logged = self.logged;
        let lines: Vec<String> = self
            .kept
            .iter()
            .filter(|f| f.number > logged)
            .map(|f| line(f, at))
            .collect();
        if let Some(last) = self.kept.back() {
            self.logged = self.logged.max(last.number);
        }
        lines
    }
}

/// A render frame's `R` line.
pub(crate) fn line(frame: &RenderFrame, at: f64) -> String {
    let mut text = format!(
        "R {at:.2} s, render frame {}, unix ms {}, working set {:.0} -> {:.0} MB:",
        frame.number, frame.unix_ms, frame.working_set_mb.0, frame.working_set_mb.1
    );
    for (i, s) in &frame.steps {
        text += &format!(" {} {:.1}", STEPS[*i], s.wall_ms);
        let thread = match (s.thread_ms, s.thread_kernel_ms) {
            (Some(t), Some(k)) => format!("thread {t:.1} k {k:.0}"),
            _ => "thread -".to_owned(),
        };
        text += &format!(
            " ({thread}, process {:.1} k {:.0}, faults {}),",
            s.process_ms, s.process_kernel_ms, s.faults
        );
    }
    text.pop();
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A step's numbers: the thread's CPU only when both marks ran on one
    /// thread, the kernel times from 100 ns units, the faults between.
    // spec: none (the long play's log: the render steps)
    #[test]
    fn a_step_takes_its_numbers_from_two_samples() {
        let a = Sample {
            thread: 7,
            thread_cycles: 1_000,
            thread_kernel: 0,
            process_cycles: 10_000,
            process_kernel: 100_000,
            faults: 50,
            working_set: 0,
        };
        let b = Sample {
            thread_cycles: 4_000,
            thread_kernel: 156_250,
            process_cycles: 30_000,
            process_kernel: 256_250,
            faults: 4_150,
            ..a
        };
        let s = step(&a, &b, 180.0, 1_000.0);
        assert_eq!(s.wall_ms, 180.0);
        assert_eq!(s.thread_ms, Some(3.0));
        assert_eq!(s.thread_kernel_ms, Some(15.625));
        assert_eq!(s.process_ms, 20.0);
        assert_eq!(s.process_kernel_ms, 15.625);
        assert_eq!(s.faults, 4_100);
        let other = Sample { thread: 8, ..b };
        assert_eq!(step(&a, &other, 1.0, 1_000.0).thread_ms, None);
    }

    /// The frames are kept from mark 0 to the next mark 0, every step in
    /// order, and each written to the log once.
    // spec: none (the long play's log: the render steps)
    #[test]
    fn the_frames_are_kept_and_written_once() {
        let mut steps = Steps::default();
        for _ in 0..3 {
            for n in 0..STEPS.len() {
                steps.mark(n);
            }
        }
        steps.mark(0);
        // Frames 1..3 are done (the first mark 0 starts frame 1).
        assert_eq!(steps.kept.len(), 3);
        let second = &steps.kept[1];
        let names: Vec<usize> = second.steps.iter().map(|(i, _)| *i).collect();
        assert_eq!(names, (0..STEPS.len()).collect::<Vec<_>>());
        let lines = steps.unlogged_lines(1.0);
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with("R 1.00 s, render frame 2, "));
        assert!(lines[1].contains(" prepare resources "));
        assert!(steps.unlogged_lines(1.0).is_empty());
        for n in 1..STEPS.len() {
            steps.mark(n);
        }
        steps.mark(0);
        assert_eq!(steps.unlogged_lines(2.0).len(), 1);
    }
}
