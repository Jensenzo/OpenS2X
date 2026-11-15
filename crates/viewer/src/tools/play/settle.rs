//! The frames before a timed act (`Act::Hold`): a step that times the
//! program's frames starts only when the frames show a display that is
//! awake and settled. A display that Windows had turned off came back by
//! `SC_MONITORPOWER` for about 1.3 s only, and the programs then ran with it
//! off: after a long night every frame was held about 264 ms for minutes,
//! in menus as in the game (13 Nov 00:26, every test of the run; with the
//! display on the same build runs at the screen's 16.7 ms). Timed then,
//! `panning_stays_smooth` measured the display, not the program. The keeper
//! (`display`) now waits for the display to stay on; this gate checks it
//! again where it matters, by the program's own frame times.
//!
//! The gate waits for [`RUN`] frames in a row, each within [`FRAME_MS`],
//! with the display on (where Windows says its state). While the display is
//! off it wakes it by a pointer move of 0 ([`WAKE_S`], once more at
//! [`WAKE_AGAIN_S`]: input, which restarts Windows' idle time; the log
//! says when). At [`LIMIT_S`] the step fails and says what the frames and
//! the display were. The step's own limits do not change.
//!
//! [`Pace`] watches every frame of the run and logs when the frames turn
//! held and when they are back at the screen's pace, with the display's
//! state: what the program does after a wake, and how long it lasts.

use std::collections::VecDeque;
use std::time::Duration;

/// Frames in a row within [`FRAME_MS`] that show the display settled:
/// about a second at the screen's 60 Hz.
pub const RUN: usize = 60;
/// The longest frame that counts as settled, ms: the timed step's own spike
/// limit (`PAN_SPIKE_MS`). A held frame takes about 250 ms.
pub const FRAME_MS: f64 = 50.0;
/// Seconds after the gate began: the display, if it is off, woken by a
/// pointer move of 0.
pub const WAKE_S: u64 = 5;
/// And once more, if it is still (or again) off.
pub const WAKE_AGAIN_S: u64 = 20;
/// The bound: the step fails when the frames have not settled by then.
pub const LIMIT_S: u64 = 60;
/// A second of frames whose median is this long or longer is held, ms.
pub const HELD_MS: f64 = 100.0;

/// What the driver does this frame for the gate.
#[derive(Debug, PartialEq)]
pub enum Gate {
    /// Nothing yet: another frame.
    Wait,
    /// Wake the display (a pointer move of 0).
    Wake,
    /// The frames are settled: the act goes ahead (the log's line).
    Settled(String),
    /// The bound passed: the step fails with this.
    Failed(String),
}

fn median(frames: impl Iterator<Item = f64>) -> f64 {
    let mut v: Vec<f64> = frames.collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[(v.len() - 1) / 2]
}

fn display_part(display: Option<&str>) -> String {
    match display {
        Some(state) => format!("display {state}"),
        None => "display state not known here".to_owned(),
    }
}

/// The gate of one timed act.
#[derive(Default)]
pub struct Settle {
    /// Frames in a row within [`FRAME_MS`].
    run: usize,
    frames: u32,
    /// The last [`RUN`] frames' lengths.
    recent: VecDeque<f64>,
    /// The wakes so far (0 to 2).
    woken: u8,
    /// The last whole second logged.
    logged: u64,
}

impl Settle {
    /// One frame of length `ms`, `since` the gate began, with the display's
    /// state as Windows gives it (`None`: not known here).
    pub fn frame(&mut self, ms: f64, since: Duration, display: Option<&str>) -> Gate {
        self.frames += 1;
        if self.recent.len() == RUN {
            self.recent.pop_front();
        }
        self.recent.push_back(ms);
        self.run = if ms <= FRAME_MS { self.run + 1 } else { 0 };
        let on = display.is_none_or(|d| d == "on");
        if self.run >= RUN && on {
            return Gate::Settled(format!(
                "frames settled after {:.1} s: {RUN} in a row within {FRAME_MS} ms, \
                 median {:.1} ms, {}",
                since.as_secs_f64(),
                self.median(),
                display_part(display)
            ));
        }
        let s = since.as_secs();
        if s >= LIMIT_S {
            return Gate::Failed(format!(
                "the frames did not settle in {LIMIT_S} s ({RUN} in a row within {FRAME_MS} ms \
                 with the display on): {} frames, the last {} at a median of {:.1} ms, {}, \
                 woken {} times by a pointer move (a display Windows turned off holds each \
                 frame about 250 ms)",
                self.frames,
                self.recent.len(),
                self.median(),
                display_part(display),
                self.woken
            ));
        }
        let due = match self.woken {
            0 => s >= WAKE_S,
            1 => s >= WAKE_AGAIN_S,
            _ => false,
        };
        if due && !on {
            self.woken += 1;
            return Gate::Wake;
        }
        Gate::Wait
    }

    fn median(&self) -> f64 {
        median(self.recent.iter().copied())
    }

    /// A line for the log once a second while the gate waits.
    pub fn progress(&mut self, since: Duration, display: Option<&str>) -> Option<String> {
        let s = since.as_secs();
        if s <= self.logged {
            return None;
        }
        self.logged = s;
        Some(format!(
            "settling before a timed step: {s} s, {} frames, the last {} at a median of {:.1} ms, \
             {} in a row within {FRAME_MS} ms, {}",
            self.frames,
            self.recent.len(),
            self.median(),
            self.run,
            display_part(display)
        ))
    }
}

/// The run's frames second by second: a line when they turn held and when
/// they are back at the screen's pace.
#[derive(Default)]
pub struct Pace {
    /// The frames of the second going on, and when it began.
    second: Vec<f64>,
    began: Option<Duration>,
    /// Whether the last second judged was held, and since when.
    held: Option<(bool, Duration)>,
}

impl Pace {
    /// One frame of length `ms` at `at` (since any fixed instant).
    pub fn frame(&mut self, ms: f64, at: Duration, display: Option<&str>) -> Option<String> {
        let began = *self.began.get_or_insert(at);
        self.second.push(ms);
        if at < began + Duration::from_secs(1) {
            return None;
        }
        let frames = std::mem::take(&mut self.second);
        self.began = Some(at);
        // A second of one or two long frames is a load or a play-ahead,
        // not the display's pace.
        if frames.len() < 3 {
            return None;
        }
        let m = median(frames.iter().copied());
        let held = m >= HELD_MS;
        let line = match self.held {
            Some((was, _)) if was == held => return None,
            None if !held => {
                self.held = Some((false, at));
                return None;
            }
            Some((_, since)) if !held => format!(
                "frames back at the screen's pace after {:.1} s held: median {m:.1} ms over \
                 the last second ({} frames), {}",
                (at - since).as_secs_f64(),
                frames.len(),
                display_part(display)
            ),
            _ => format!(
                "frames held: median {m:.1} ms over the last second ({} frames), {}",
                frames.len(),
                display_part(display)
            ),
        };
        self.held = Some((held, at));
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: f64) -> Duration {
        Duration::from_secs_f64(s)
    }

    /// Frames of `ms` from `from` s on, until the gate decides something
    /// other than waiting; the decisions with their times.
    fn run(gate: &mut Settle, ms: f64, from: f64, to: f64, display: &str) -> Vec<(f64, Gate)> {
        let mut out = Vec::new();
        let mut t = from;
        while t < to {
            match gate.frame(ms, secs(t), Some(display)) {
                Gate::Wait => {}
                g => {
                    let end = matches!(g, Gate::Settled(_) | Gate::Failed(_));
                    out.push((t, g));
                    if end {
                        break;
                    }
                }
            }
            t += ms / 1000.0;
        }
        out
    }

    // spec: none (the play harness: a timed step starts on settled frames)
    #[test]
    fn frames_at_the_screens_pace_settle_in_a_second() {
        let mut gate = Settle::default();
        let got = run(&mut gate, 16.7, 0.0, 10.0, "on");
        assert_eq!(got.len(), 1);
        let (t, g) = &got[0];
        assert!(*t < 1.1, "settled at {t}");
        assert!(matches!(g, Gate::Settled(line) if line.contains("display on")));
    }

    // spec: none (the play harness: a timed step starts on settled frames)
    #[test]
    fn held_frames_with_the_display_on_fail_at_the_bound_without_input() {
        let mut gate = Settle::default();
        let got = run(&mut gate, 264.0, 0.0, 120.0, "on");
        let [(t, Gate::Failed(e))] = got.as_slice() else {
            panic!("{got:?}")
        };
        assert_eq!(t.floor() as u64, LIMIT_S);
        // The message says what the frames and the display were.
        assert!(e.contains("a median of 264.0 ms"), "{e}");
        assert!(e.contains("display on"), "{e}");
        assert!(e.contains("woken 0 times"), "{e}");
    }

    // spec: none (the play harness: a timed step starts on settled frames)
    #[test]
    fn the_display_off_is_woken_twice_then_the_step_fails() {
        // 13 Nov 07:47: frames at 16.9 ms with the display off do not count.
        let mut gate = Settle::default();
        let got = run(&mut gate, 16.7, 0.0, 120.0, "off");
        let kinds: Vec<_> = got.iter().map(|(t, g)| (t.floor() as u64, g)).collect();
        assert_eq!(kinds.len(), 3, "{kinds:?}");
        assert_eq!(kinds[0], (WAKE_S, &Gate::Wake));
        assert_eq!(kinds[1], (WAKE_AGAIN_S, &Gate::Wake));
        assert!(matches!(kinds[2], (LIMIT_S, Gate::Failed(e)) if e.contains("display off")));
    }

    // spec: none (the play harness: a timed step starts on settled frames)
    #[test]
    fn frames_settle_once_the_wake_turned_the_display_on() {
        let mut gate = Settle::default();
        let off = run(&mut gate, 16.7, 0.0, 6.0, "off");
        assert!(matches!(off.as_slice(), [(_, Gate::Wake)]), "{off:?}");
        let back = run(&mut gate, 16.7, 6.0, 60.0, "on");
        assert!(
            matches!(back.as_slice(), [(t, Gate::Settled(_))] if *t < 7.1),
            "{back:?}"
        );
    }

    // spec: none (the play harness: a timed step starts on settled frames)
    #[test]
    fn one_slow_frame_restarts_the_run() {
        let mut gate = Settle::default();
        for i in 0..59 {
            assert_eq!(gate.frame(16.7, secs(i as f64 * 0.0167), None), Gate::Wait);
        }
        assert_eq!(gate.frame(80.0, secs(1.0), None), Gate::Wait);
        for i in 0..59 {
            assert_eq!(
                gate.frame(16.7, secs(1.1 + i as f64 * 0.0167), None),
                Gate::Wait
            );
        }
        assert!(matches!(
            gate.frame(16.7, secs(2.2), None),
            Gate::Settled(_)
        ));
    }

    // spec: none (the play harness logs the frames' pace)
    #[test]
    fn the_pace_is_logged_when_it_changes() {
        let mut pace = Pace::default();
        let mut lines = Vec::new();
        let mut t = 0.0;
        // A load: one long frame, then 3 s at the screen's pace, 4 s held,
        // 2 s at the pace again.
        let mut frames = vec![2000.0];
        frames.extend(std::iter::repeat_n(16.7, 180));
        frames.extend(std::iter::repeat_n(264.0, 16));
        frames.extend(std::iter::repeat_n(16.7, 120));
        for ms in frames {
            t += ms / 1000.0;
            if let Some(line) = pace.frame(ms, secs(t), Some("on")) {
                lines.push(line);
            }
        }
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[0].starts_with("frames held: median 264.0 ms"),
            "{lines:?}"
        );
        assert!(
            lines[1].starts_with("frames back at the screen's pace after"),
            "{lines:?}"
        );
    }
}
