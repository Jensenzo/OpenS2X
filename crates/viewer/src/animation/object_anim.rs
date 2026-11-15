//! Animations of buildings and ships (`specs/GFX-animations.md`
//! GFX-ANIM-6, 7): when a door opens and closes, which clips a building
//! has playing, and how a ship bobs. Plain logic; the view applies it.

use std::collections::HashMap;

use s2_sim::grid::Point;

/// GFX-ANIM-6.1a b, TOOL-trace TRACE-5.3: DoorClose starts 0.6 s after the
/// last "open", on game time as of the last tick: 6 ticks (ECO-TIME-2.2).
pub const DOOR_CLOSE_TICKS: u64 = 6;

/// GFX-ANIM-6.1: the step progress past which a settler stepping onto
/// (`enter`) or off (`leave`) the door field opens the door: no building
/// type sets them, so the loader's defaults hold for all, 0.3 and 0.0 (the
/// door opens as soon as a settler starts leaving).
pub const ENTER_DISTANCE: f32 = 0.3;
pub const LEAVE_DISTANCE: f32 = 0.0;

/// A walker near a door: its id, the field it is on, the next one and the
/// step's progress (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stepper {
    pub id: u64,
    pub at: Point,
    pub next: Option<Point>,
    pub progress: f32,
}

/// GFX-ANIM-6.1a b: the buildings whose door a walker holds open this
/// tick: his step goes from the building's door flag onto the building
/// with progress above `enter`, or from the building to its flag above
/// `leave`. `doors` pairs each building field with its flag field. Called
/// in every tick, so each tick of such a step pushes the close time on.
pub fn door_triggers(
    doors: &HashMap<Point, Point>,
    steppers: &[Stepper],
    enter: f32,
    leave: f32,
) -> Vec<Point> {
    let mut held = Vec::new();
    for s in steppers {
        let Some(next) = s.next else { continue };
        let entering = doors.get(&next) == Some(&s.at) && s.progress > enter;
        let leaving = doors.get(&s.at) == Some(&next) && s.progress > leave;
        let building = if entering {
            next
        } else if leaving {
            s.at
        } else {
            continue;
        };
        if !held.contains(&building) {
            held.push(building);
        }
    }
    held
}

/// A building's door: which clip shows and since when (`since`, on the
/// scene's clip clock), and in which tick it closes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Door {
    Closed,
    /// DoorOpen started at `since`; DoorClose starts in tick `close_tick`.
    Open {
        since: f32,
        close_tick: u64,
    },
    /// DoorClose started at `since` (it holds its last frame).
    Closing {
        since: f32,
    },
}

impl Door {
    /// GFX-ANIM-6.1a b: "open" in `tick`: the close time becomes 0.6 s
    /// later (TRACE-5.3: game time as of the last tick); only a door not
    /// open yet starts DoorOpen at `now` (true: play it and `dooropen`).
    pub fn open(&mut self, now: f32, tick: u64) -> bool {
        let close_tick = tick + DOOR_CLOSE_TICKS;
        match self {
            Self::Open { close_tick: at, .. } => {
                *at = close_tick;
                false
            }
            _ => {
                *self = Self::Open {
                    since: now,
                    close_tick,
                };
                true
            }
        }
    }

    /// GFX-ANIM-6.1a c: once the game time of the last tick reaches the
    /// close time DoorClose starts, at `now`; true in the frame it does
    /// (play `doorclose`).
    pub fn update(&mut self, now: f32, tick: u64) -> bool {
        if let Self::Open { close_tick, .. } = *self {
            if tick >= close_tick {
                *self = Self::Closing { since: now };
                return true;
            }
        }
        false
    }
}

/// GFX-ANIM-7.2: a ship's bobbing at phase `p`: pitch and roll (radians)
/// and the lift over the water level. Resting and moving ships use their
/// own frequencies and amplitudes.
pub fn bobbing(p: f32, moving: bool) -> (f32, f32, f32) {
    let (f1, a1, f2, a2) = if moving {
        (2.5, 0.04, 1.7, 0.08)
    } else {
        (0.66, 0.01, 0.4, 0.02)
    };
    let pitch = ((f1 * p + 0.2).sin() + 1.0) * a1;
    let roll = ((f2 * p + 0.4).sin() + 1.0) * a2;
    let lift = (p.sin() + 1.0) * 0.06;
    (pitch, roll, lift)
}

/// GFX-ANIM-7.2a: the turn of a ship of game heading `heading` (radians,
/// GFX-VIEW-4.1) bobbing by `pitch` and `roll` (7.2's first and second
/// values), in Bevy's frame. The ship model, converted as every model
/// (GFX-VIEW-1.2), has its length on x with the bow at -x and its beam on z
/// (the starboard side at +z of the game's axes). For column vectors the
/// game's R = Ry(heading + 90 degrees) Rx(first) Rz(second): the second
/// dips the bow (+x towards +y), then the first heels the ship to starboard
/// (+z towards -y), then the yaw turns the bow to where a settler of the
/// same heading faces (GFX-OBJ-1.8). The game's z is Bevy's -z, so a turn
/// about x or y changes its sense there and one about z keeps it.
pub fn ship_rotation(heading: f32, pitch: f32, roll: f32) -> bevy::math::Quat {
    use bevy::math::Quat;
    let yaw = crate::render::world3d::bevy_yaw(heading + std::f32::consts::FRAC_PI_2);
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-pitch) * Quat::from_rotation_z(roll)
}

/// GFX-ANIM-7.2: each ship's phase starts 0.89759797 x its creation index
/// further on.
pub const SHIP_PHASE_OFFSET: f32 = 0.897_597_97;

/// GFX-ANIM-7.2 (Q-181 e): the phase advances by 2 pi x time x 0.25, one
/// full turn every 4 s.
pub const SHIP_RATE: f32 = std::f32::consts::TAU;

/// GFX-ANIM-7.2, TOOL-trace TRACE-5.3: the ships' phase (before each
/// ship's own offset) at `seconds` of game time: it stands while the game
/// is paused and runs 2 and 3 times as fast at speeds 2 and 3.
pub fn ship_phase(seconds: f64) -> f32 {
    // Taken modulo 50 turns first, so the f32 keeps its precision in long
    // games: sin(f p) for every f of the bobbing (1, 0.66, 0.4, 2.5, 1.7)
    // repeats after 50 turns, f x 50 being a whole number.
    let turns = (seconds * 0.25).rem_euclid(50.0);
    (turns * SHIP_RATE as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    /// GFX-ANIM T7b (6.1a): a woodcutter leaves his hut by a building ->
    /// flag step of 17 ticks from tick 100: the door opens in tick 100,
    /// stays open through tick 116 and closes 0.6 s after it; one open and
    /// one close.
    // spec: GFX-ANIM-6.1a (T7b)
    #[test]
    fn t7b_door_open_through_the_step() {
        let doors: HashMap<Point, Point> = [((5, 5), (6, 5))].into();
        let mut door = Door::Closed;
        let (mut opens, mut closes) = (0, 0);
        for tick in 100..=130u64 {
            let now = tick as f32 * 0.1;
            let k = tick as f32 - 100.0;
            let walker = (tick <= 116).then(|| Stepper {
                id: 1,
                at: (5, 5),
                next: Some((6, 5)),
                progress: (k + 1.0) / 17.0,
            });
            let held = door_triggers(&doors, walker.as_slice(), ENTER_DISTANCE, LEAVE_DISTANCE);
            for _ in held {
                opens += usize::from(door.open(now, tick));
            }
            if tick == 100 {
                assert_eq!(
                    door,
                    Door::Open {
                        since: 10.0,
                        close_tick: 106
                    }
                );
            }
            if tick <= 116 {
                assert!(matches!(door, Door::Open { .. }), "open in tick {tick}");
            }
            closes += usize::from(door.update(now, tick));
            // 0.6 s after the last open (tick 116): in tick 122.
            assert_eq!(
                matches!(door, Door::Closing { .. }),
                tick >= 122,
                "tick {tick}"
            );
        }
        assert_eq!((opens, closes), (1, 1));
        assert!(matches!(door, Door::Closing { since } if close(since, 12.2)));
    }

    /// TRACE-5.3: the close time is game time as of the last tick, so a
    /// paused game (no tick, frames running) keeps the door open, and at
    /// speed 3 it closes after the same 6 ticks, a third of the real time.
    // spec: GFX-ANIM-6.1a, TOOL-trace TRACE-5.3
    #[test]
    fn the_door_closes_on_tick_time() {
        let mut door = Door::Closed;
        assert!(door.open(1.0, 10));
        for frame in 0..1000 {
            assert!(!door.update(1.0 + frame as f32 * 0.016, 10), "paused");
        }
        assert!(!door.update(2.0, 15));
        assert!(door.update(2.1, 16));
        assert_eq!(door, Door::Closing { since: 2.1 });
    }

    /// GFX-ANIM-6.1a b: entering opens past 0.3 of the flag -> building
    /// step; steps that do not join the flag and the building open nothing.
    #[test]
    fn door_trigger_steps() {
        let doors: HashMap<Point, Point> = [((5, 5), (6, 5))].into();
        let step = |at, next, progress| Stepper {
            id: 1,
            at,
            next: Some(next),
            progress,
        };
        let held = |s| door_triggers(&doors, &[s], ENTER_DISTANCE, LEAVE_DISTANCE);
        assert!(held(step((6, 5), (5, 5), 0.29)).is_empty());
        assert_eq!(held(step((6, 5), (5, 5), 0.31)), [(5, 5)]);
        assert!(held(step((5, 5), (6, 5), 0.0)).is_empty());
        assert_eq!(held(step((5, 5), (6, 5), 0.05)), [(5, 5)]);
        // Past the building, not from its flag.
        assert!(held(step((4, 5), (5, 5), 0.9)).is_empty());
        assert!(held(step((5, 5), (4, 5), 0.9)).is_empty());
    }

    /// GFX-ANIM-7.2 (Q-181 e): one full turn every 4 s of game time, so
    /// the lift (sin p) is the same at t and t + 4 and the whole bobbing at
    /// t and t + 200 (50 turns); the phase follows game time (TRACE-5.3),
    /// also late in a long game.
    // spec: GFX-ANIM-7.2, TOOL-trace TRACE-5.3
    #[test]
    fn ship_phase_turns_every_4_game_seconds() {
        assert!(close(ship_phase(0.0), 0.0));
        assert!(close(ship_phase(1.0), std::f32::consts::FRAC_PI_2));
        for t in [0.3, 17.0, 3600.25, 36_000.7] {
            for moving in [false, true] {
                let a = bobbing(ship_phase(t), moving);
                assert!(
                    (a.2 - bobbing(ship_phase(t + 4.0), moving).2).abs() < 1e-4,
                    "t {t}"
                );
                let b = bobbing(ship_phase(t + 200.0), moving);
                assert!(
                    (a.0 - b.0).abs() < 1e-4
                        && (a.1 - b.1).abs() < 1e-4
                        && (a.2 - b.2).abs() < 1e-4,
                    "t {t}"
                );
            }
        }
    }

    /// GFX-ANIM T8: a moving ship at p = 0.
    // spec: GFX-ANIM-7.2 (T8)
    #[test]
    fn t8_ship() {
        let (pitch, roll, lift) = bobbing(0.0, true);
        assert!(close(pitch, (0.2f32.sin() + 1.0) * 0.04));
        assert!(close(roll, (0.4f32.sin() + 1.0) * 0.08));
        assert!(close(lift, 0.06));
        let (pitch, ..) = bobbing(0.0, false);
        assert!(close(pitch, (0.2f32.sin() + 1.0) * 0.01));
    }

    /// GFX-ANIM T22: a resting ship at p = 0: the bow dips by
    /// (sin 0.4 + 1) x 0.02 = 0.027788 rad (about the model's z, taking +x
    /// towards +y), then it heels to starboard by (sin 0.2 + 1) x 0.01 =
    /// 0.011987 rad (about x, taking +z towards -y), then yaw = heading + 90
    /// degrees.
    // spec: GFX-ANIM-7.2a (T22), GFX-OBJ-1.8
    #[test]
    fn t22_ship_turn_axes_and_order() {
        use crate::render::world3d::{bevy_yaw, from_bevy};
        use bevy::math::{Quat, Vec3};
        use std::f32::consts::FRAC_PI_2;
        let (pitch, roll, lift) = bobbing(0.0, false);
        assert!((roll - 0.027_788).abs() < 1e-6, "{roll}");
        assert!((pitch - 0.011_987).abs() < 1e-6, "{pitch}");
        assert!(close(lift, 0.06));
        let near = |a: Vec3, b: Vec3| (a - b).length() < 1e-5;
        // In the game's axes: the bow -x, the starboard side +z, up +y.
        let game = |v: Vec3| Vec3::from(from_bevy(v.to_array()));
        let bevy = |v: Vec3| Vec3::from(crate::render::world3d::to_bevy(v.to_array()));
        let (bow, starboard) = (bevy(Vec3::NEG_X), bevy(Vec3::Z));
        // Heading -90 degrees: no yaw, the turn alone.
        let r = ship_rotation(-FRAC_PI_2, pitch, roll);
        // The second value's turn about z alone (+x towards +y) dips the
        // bow by 0.027788; the first's then about x (+z towards -y).
        let dipped = Vec3::new(-roll.cos(), -roll.sin(), 0.0);
        let heeled = Vec3::new(dipped.x, dipped.y * pitch.cos(), dipped.y * pitch.sin());
        assert!(near(game(r * bow), heeled), "{} {heeled}", game(r * bow));
        assert!(game(r * bow).y < 0.0);
        // The starboard side goes down by the first value.
        assert!(near(
            game(r * starboard),
            Vec3::new(0.0, -pitch.sin(), pitch.cos())
        ));
        // The order matters: the other order puts the bow elsewhere.
        let other = Quat::from_rotation_z(roll) * Quat::from_rotation_x(-pitch);
        assert!(!near(r * bow, other * bow));
        // The yaw last, heading + 90 degrees: unbobbed, the bow points where
        // a settler of the same heading faces (his rest pose faces the
        // game's +z, GFX-OBJ-1.8), for every heading of GFX-VIEW-4.1.
        for d in 0..6 {
            let h = crate::render::world3d::heading(d);
            let settler = Quat::from_rotation_y(bevy_yaw(h)) * bevy(Vec3::Z);
            assert!(
                near(ship_rotation(h, 0.0, 0.0) * bow, settler),
                "direction {d}"
            );
            assert!(near(
                ship_rotation(h, pitch, roll) * bow,
                Quat::from_rotation_y(bevy_yaw(h + FRAC_PI_2)) * (r * bow)
            ));
        }
        // Direction 0 (east, +x): the bow ahead.
        let east = ship_rotation(crate::render::world3d::heading(0), 0.0, 0.0) * bow;
        assert!(near(game(east), Vec3::X), "{east}");
    }
}
