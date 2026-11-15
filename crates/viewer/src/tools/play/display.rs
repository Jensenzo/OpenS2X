//! The harness keeps the display on while it runs. With the display off
//! after the power plan's idle timeout (15 minutes on the PC it runs on),
//! lessons 1 and 4 failed at night and passed by day: the steps' clicks
//! landed beside their widgets. So no run depends on the power settings:
//! at the start the harness asks Windows for the display and the system
//! (`SetThreadExecutionState`, `ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED`,
//! continuous), held by a thread of its own until the program ends; when
//! the display is off it wakes it and waits until it is on, and stays on,
//! before the window opens. The report's first line says what it found
//! ([`Keeper::note`]).
//!
//! The display's state comes from Windows' notice
//! `GUID_CONSOLE_DISPLAY_STATE` (0 off, 1 on, 2 dimmed), which a hidden
//! window of the thread registers for; the first notice comes at the
//! registration, and every change after it goes to the log with its time
//! (UTC). The ways to wake it, each tried only when the one before did not
//! turn it on for good: the request itself, then `SC_MONITORPOWER` on (-1)
//! to the hidden window, then a pointer move of 0 pixels. That last one is
//! input, so it restarts the idle time the scripts read before a run; the
//! note names it, with its time, when it was needed.
//!
//! "For good" is [`STAY_MS`]: on the PC the harness runs on, with the
//! display off after the idle timeout, `SC_MONITORPOWER` turns it on for
//! about 1.3 s and Windows turns it off again (13 Nov, the log's notices);
//! the request held does not keep it on. Before that was seen, the keeper
//! took the first "on" for the display woken, and the programs ran with it
//! off: after a long night every frame was held about 264 ms
//! (`panning_stays_smooth` failed, every other test ran four times slower).
//! A pointer move turns it on for as long as the request is held. The
//! driver checks again before a timed step (`settle`): the frames must be
//! settled with the display on, and [`wake`] is its way to wake it again.

/// How long the display must stay on after a wake before the keeper takes
/// it for woken, ms.
#[cfg_attr(not(windows), allow(dead_code))]
pub const STAY_MS: u64 = 4000;

/// What the harness found of the display at its start; nothing where the
/// program does not run on Windows.
pub struct Keeper {
    note: Option<String>,
}

impl Keeper {
    /// The report's line about the display, if there is one.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// The display's states, as `GUID_CONSOLE_DISPLAY_STATE` gives them.
#[cfg_attr(not(windows), allow(dead_code))]
mod state {
    pub const OFF: u8 = 0;
    pub const ON: u8 = 1;
    pub const DIMMED: u8 = 2;
    /// Before the first notice.
    pub const UNKNOWN: u8 = 255;

    pub fn name(state: u8) -> &'static str {
        match state {
            OFF => "off",
            ON => "on",
            DIMMED => "dimmed",
            _ => "unknown",
        }
    }
}

/// What one way of waking did.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Woke {
    /// The display did not come on.
    Never,
    /// It came on and went off again after this many ms.
    Briefly(u128),
    /// It came on and stayed on for [`STAY_MS`].
    Stayed,
}

/// How a wake went: the state at the start, each way tried and what it
/// did, the state at the end and how long it all took. Kept apart from the
/// system calls so the wording is tested everywhere.
#[cfg_attr(not(windows), allow(dead_code))]
fn describe(start: u8, tries: &[(&str, Woke)], end: u8, waited_ms: u128) -> String {
    let held = "held on for the run (ES_DISPLAY_REQUIRED)";
    if start == state::ON {
        return format!("display: on at the start, {held}");
    }
    if start == state::UNKNOWN && end == state::UNKNOWN {
        return format!("display: state unknown (no notice from Windows), {held}");
    }
    let brief: Vec<String> = tries
        .iter()
        .filter_map(|(way, woke)| match woke {
            Woke::Briefly(ms) => Some(format!(
                "{way} turned it on for {:.1} s only",
                *ms as f64 / 1000.0
            )),
            _ => None,
        })
        .collect();
    let brief = if brief.is_empty() {
        String::new()
    } else {
        format!(" ({})", brief.join(", "))
    };
    let waited = waited_ms as f64 / 1000.0;
    match tries.last() {
        Some((way, Woke::Stayed)) if end == state::ON => format!(
            "display: {} at the start, on after {waited:.1} s by {way}{brief}, {held}",
            state::name(start)
        ),
        _ => format!(
            "display: {} at the start and still {} after {waited:.1} s{brief}; the steps may \
             miss, and timed steps wait for it",
            state::name(start),
            state::name(end)
        ),
    }
}

#[cfg(not(windows))]
pub fn hold() -> Keeper {
    Keeper { note: None }
}

#[cfg(windows)]
pub fn hold() -> Keeper {
    win::hold()
}

/// The display's state as Windows last gave it ("on", "off", "dimmed",
/// "unknown"); `None` where the program does not run on Windows or the
/// keeper was not started.
#[cfg(not(windows))]
pub fn now() -> Option<&'static str> {
    None
}

#[cfg(windows)]
pub fn now() -> Option<&'static str> {
    win::now()
}

/// Wakes the display again by a pointer move of 0 (input: the idle time
/// restarts); the log's line says so with its time. False where there is
/// nothing to do it with.
#[cfg(not(windows))]
pub fn wake() -> bool {
    false
}

#[cfg(windows)]
pub fn wake() -> bool {
    win::pointer_move()
}

/// The time of day in UTC, as the log's lines carry it: `08:26:18.0Z`.
pub fn utc_now() -> String {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    utc_of(since.as_millis())
}

fn utc_of(unix_ms: u128) -> String {
    let day_ms = unix_ms % 86_400_000;
    let (h, m, s, tenth) = (
        day_ms / 3_600_000,
        day_ms / 60_000 % 60,
        day_ms / 1000 % 60,
        day_ms / 100 % 10,
    );
    format!("{h:02}:{m:02}:{s:02}.{tenth}Z")
}

#[cfg(windows)]
mod win {
    use std::sync::atomic::{AtomicIsize, AtomicU8, Ordering};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Power::{
        RegisterPowerSettingNotification, SetThreadExecutionState, ES_CONTINUOUS,
        ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, POWERBROADCAST_SETTING,
    };
    use windows_sys::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        SendMessageW, TranslateMessage, DEVICE_NOTIFY_WINDOW_HANDLE, MSG, PBT_POWERSETTINGCHANGE,
        SC_MONITORPOWER, WM_POWERBROADCAST, WM_SYSCOMMAND, WNDCLASSW,
    };

    use super::{describe, state, utc_now, Keeper, Woke, STAY_MS};

    static STATE: AtomicU8 = AtomicU8::new(state::UNKNOWN);
    /// The keeper's hidden window, 0 before it exists.
    static WINDOW: AtomicIsize = AtomicIsize::new(0);

    pub fn now() -> Option<&'static str> {
        (WINDOW.load(Ordering::SeqCst) != 0).then(|| state::name(STATE.load(Ordering::SeqCst)))
    }

    /// A pointer move of 0 pixels: input, which wakes the display.
    pub fn pointer_move() -> bool {
        // SAFETY: one well-formed input record.
        let sent = unsafe {
            let mut input: INPUT = std::mem::zeroed();
            input.r#type = INPUT_MOUSE;
            input.Anonymous.mi.dwFlags = MOUSEEVENTF_MOVE;
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) == 1
        };
        eprintln!(
            "play: display: a pointer move of 0 at {} (input: the idle time restarts)",
            utc_now()
        );
        sent
    }

    fn same_guid(a: &windows_sys::core::GUID, b: &windows_sys::core::GUID) -> bool {
        a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
    }

    unsafe extern "system" fn notices(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_POWERBROADCAST && wparam as u32 == PBT_POWERSETTINGCHANGE && lparam != 0 {
            // SAFETY: with PBT_POWERSETTINGCHANGE, lparam points at the
            // setting's notice, valid for this call.
            let setting = unsafe { &*(lparam as *const POWERBROADCAST_SETTING) };
            if same_guid(&setting.PowerSetting, &GUID_CONSOLE_DISPLAY_STATE)
                && setting.DataLength >= 1
            {
                let new = setting.Data[0];
                let was = STATE.swap(new, Ordering::SeqCst);
                if was != new && was != state::UNKNOWN {
                    eprintln!("play: display: {} at {}", state::name(new), utc_now());
                }
            }
            return 1;
        }
        // SAFETY: every other message goes to the default handling.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    /// The keeper's thread: holds the request and the hidden window whose
    /// notices give the display's state, for as long as the program runs.
    fn keep(window: std::sync::mpsc::Sender<isize>) {
        let class: Vec<u16> = "s2-play-display\0".encode_utf16().collect();
        // SAFETY: plain Win32 calls with valid, live arguments; the window
        // and its class live as long as this thread, which runs to the
        // program's end.
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
            let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(notices),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            // Not WS_VISIBLE: a top-level window nobody sees.
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if !hwnd.is_null() {
                RegisterPowerSettingNotification(
                    hwnd,
                    &GUID_CONSOLE_DISPLAY_STATE,
                    DEVICE_NOTIFY_WINDOW_HANDLE,
                );
            }
            WINDOW.store(hwnd as isize, Ordering::SeqCst);
            let _ = window.send(hwnd as isize);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Waits until `done` holds for the display's state or `limit` passes;
    /// the state then.
    fn wait_for(done: impl Fn(u8) -> bool, limit: Duration) -> u8 {
        let until = Instant::now() + limit;
        loop {
            let now = STATE.load(Ordering::SeqCst);
            if done(now) || Instant::now() >= until {
                return now;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// After a way of waking was tried: whether the display came on within
    /// `limit`, and stayed on.
    fn outcome(limit: Duration) -> Woke {
        if wait_for(|s| s == state::ON, limit) != state::ON {
            return Woke::Never;
        }
        let on = Instant::now();
        let stay = Duration::from_millis(STAY_MS);
        if wait_for(|s| s != state::ON, stay) == state::ON {
            Woke::Stayed
        } else {
            Woke::Briefly(on.elapsed().as_millis())
        }
    }

    pub fn hold() -> Keeper {
        let (tx, rx) = std::sync::mpsc::channel();
        let started = std::thread::Builder::new()
            .name("display keeper".into())
            .spawn(move || keep(tx));
        if started.is_err() {
            return Keeper {
                note: Some("display: the keeper's thread did not start".into()),
            };
        }
        let hwnd = rx.recv_timeout(Duration::from_secs(2)).unwrap_or(0);
        // The first notice comes at the registration.
        let start = wait_for(|s| s != state::UNKNOWN, Duration::from_secs(2));
        let began = Instant::now();
        let mut tries: Vec<(&str, Woke)> = Vec::new();
        if start != state::ON && start != state::UNKNOWN {
            // The request is held from the keeper's start.
            tries.push(("the request", outcome(Duration::from_secs(3))));
            if tries.last().is_some_and(|t| t.1 != Woke::Stayed) && hwnd != 0 {
                // SAFETY: the keeper's window, alive for the program's life.
                unsafe {
                    SendMessageW(
                        hwnd as HWND,
                        WM_SYSCOMMAND,
                        SC_MONITORPOWER as WPARAM,
                        -1isize as LPARAM,
                    );
                }
                tries.push(("SC_MONITORPOWER", outcome(Duration::from_secs(3))));
            }
            if tries.last().is_some_and(|t| t.1 != Woke::Stayed) {
                pointer_move();
                tries.push((
                    "a pointer move of 0 (input: the idle time restarts)",
                    outcome(Duration::from_secs(5)),
                ));
            }
        }
        let end = STATE.load(Ordering::SeqCst);
        let note = describe(start, &tries, end, began.elapsed().as_millis());
        eprintln!("play: {note} ({})", utc_now());
        Keeper { note: Some(note) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the play harness keeps the display on)
    #[test]
    fn the_note_says_what_the_harness_found() {
        assert_eq!(
            describe(state::ON, &[], state::ON, 0),
            "display: on at the start, held on for the run (ES_DISPLAY_REQUIRED)"
        );
        assert_eq!(
            describe(state::OFF, &[("the request", Woke::Stayed)], state::ON, 1300),
            "display: off at the start, on after 1.3 s by the request, held on for the run (ES_DISPLAY_REQUIRED)"
        );
        // 13 Nov: SC_MONITORPOWER's "on" lasted 1.3 s; the pointer move's stayed.
        let pointer = "a pointer move of 0 (input: the idle time restarts)";
        assert_eq!(
            describe(
                state::OFF,
                &[
                    ("the request", Woke::Never),
                    ("SC_MONITORPOWER", Woke::Briefly(1300)),
                    (pointer, Woke::Stayed),
                ],
                state::ON,
                14_600
            ),
            "display: off at the start, on after 14.6 s by a pointer move of 0 (input: the idle \
             time restarts) (SC_MONITORPOWER turned it on for 1.3 s only), held on for the run \
             (ES_DISPLAY_REQUIRED)"
        );
        // A brief "on" is not a woken display.
        assert_eq!(
            describe(
                state::DIMMED,
                &[
                    ("the request", Woke::Never),
                    ("SC_MONITORPOWER", Woke::Briefly(1300)),
                    (pointer, Woke::Briefly(900)),
                ],
                state::OFF,
                11_000
            ),
            "display: dimmed at the start and still off after 11.0 s (SC_MONITORPOWER turned it \
             on for 1.3 s only, a pointer move of 0 (input: the idle time restarts) turned it on \
             for 0.9 s only); the steps may miss, and timed steps wait for it"
        );
        assert!(describe(state::UNKNOWN, &[], state::UNKNOWN, 0).contains("unknown"));
        // The test reads FAIL as a failed step.
        for s in [state::ON, state::OFF, state::DIMMED, state::UNKNOWN] {
            assert!(!describe(s, &[], s, 0).contains("FAIL"));
        }
    }

    // spec: none (the play harness logs the display's changes with their time)
    #[test]
    fn the_log_time_is_utc_time_of_day() {
        // 2026-11-13 08:26:18.029 UTC
        assert_eq!(utc_of(1_794_558_378_029), "08:26:18.0Z");
        assert_eq!(utc_of(0), "00:00:00.0Z");
        assert_eq!(utc_of(86_399_999), "23:59:59.9Z");
    }
}
