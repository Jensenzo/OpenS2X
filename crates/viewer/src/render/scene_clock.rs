//! TRACE-5.2, GFX-OBJ-2.9 (Q-313), GFX-WATER-2.4 (Q-055 answered): the shaders' clock is the world's game
//! time, so flag cloth, tree sway, water and terrain wobble stop while the
//! game is paused and run 2 and 3 times as fast at speeds 2 and 3. The
//! shaders read Bevy's `globals.time`, which Bevy fills from the render
//! world's `Time`; this plugin sets that `Time` to the game time of
//! [`GameClock`] before the globals are written. The main world's `Time`
//! (frame time: camera, messages, tooltips) is not touched.

use crate::game::GameClock;
use bevy::prelude::*;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use std::time::Duration;

/// The game time carried into the render world.
#[derive(Resource, Default, Clone, Copy)]
struct SceneClock {
    seconds: f64,
    step: f32,
}

pub(crate) struct SceneClockPlugin;

impl Plugin for SceneClockPlugin {
    fn build(&self, app: &mut App) {
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render
            .init_resource::<SceneClock>()
            .add_systems(ExtractSchedule, extract_clock)
            // Before `prepare_globals_buffer` (PrepareResources).
            .add_systems(Render, set_shader_time.in_set(RenderSystems::PrepareAssets));
    }
}

fn extract_clock(mut scene: ResMut<SceneClock>, clock: Extract<Option<Res<GameClock>>>) {
    if let Some(clock) = clock.as_ref() {
        *scene = SceneClock {
            seconds: clock.world_seconds,
            step: clock.step,
        };
    }
}

fn set_shader_time(scene: Res<SceneClock>, mut time: ResMut<Time>) {
    *time = shader_time(scene.seconds, scene.step);
}

/// Game time does not wrap within a game (Bevy's default wraps after an
/// hour).
const WRAP: Duration = Duration::from_secs(1 << 32);

/// A `Time` whose elapsed time is `seconds` of game time and whose last
/// step was `step`.
pub(crate) fn shader_time(seconds: f64, step: f32) -> Time {
    let mut time = Time::<()>::default();
    time.set_wrap_period(WRAP);
    let now = Duration::from_secs_f64(seconds.max(0.0));
    time.advance_to(now.saturating_sub(Duration::from_secs_f32(step.max(0.0))));
    time.advance_to(now);
    time
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: GFX-OBJ-2.9, TOOL-trace TRACE-5.2 (the shaders' t is game time
    // in seconds, ECO-TIME-1.4)
    #[test]
    fn the_shader_clock_is_the_worlds_game_time() {
        // Tick 25 and half of the next: 2.55 s of game time.
        let seconds = crate::game::world_seconds(25, 0.5);
        assert!((seconds - 2.55).abs() < 1e-9);
        let time = shader_time(seconds, 0.05);
        assert!((time.elapsed_secs_wrapped() - 2.55).abs() < 1e-6);
        assert!((time.delta_secs() - 0.05).abs() < 1e-6);
        // Paused (no step), the clock stands; at a new game it is 0.
        assert_eq!(
            shader_time(seconds, 0.0).elapsed_secs_wrapped(),
            time.elapsed_secs_wrapped()
        );
        assert_eq!(
            shader_time(crate::game::world_seconds(0, 0.0), 0.0).elapsed_secs_wrapped(),
            0.0
        );
        // Past an hour it does not wrap: 2 hours of game time.
        let late = shader_time(crate::game::world_seconds(72_000, 0.0), 0.1);
        assert!((late.elapsed_secs_wrapped() - 7200.0).abs() < 1e-3);
    }
}
