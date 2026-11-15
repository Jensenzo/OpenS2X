# Clocks in the viewer

TOOL-trace TRACE-5 says which clock each moving thing follows; the viewer
keeps three, as the original does (TRACE-5.1):

- **The game step** (ECO-TIME-1.2: frame seconds x speed; 0 while paused or
  held, 2x and 3x at speeds 2 and 3). `GameClock::step`, set each frame by
  `advance_world` (`crates/viewer/src/game/mod.rs`) from the same step as
  the logic ticks (a step above 5 s is replaced by one tick, ECO-TIME-1.3).
  `GameClock::seconds` sums it (the clip clock); `GameClock::ticks` counts
  the logic ticks of the frame; `GameClock::world_seconds` is the world's
  game time, (tick + share of the next tick) x 0.1 s (ECO-TIME-1.4).
- **The frame time**: Bevy's `Time` (runs while paused, not scaled by speed).
- **Real time**: Bevy's `Time<Real>`, the second clock (TOOL-1.3).

Under `--still` (golden images) the game is paused and the frame clock is
held at 0, so both stand.

## On game time

| What | Clock | Where | Rule |
|---|---|---|---|
| Logic ticks and the share of the current tick (positions between ticks) | game step | `game::advance_world` | ECO-TIME-1.2 to 1.4, GFX-ANIM-4.1 |
| Flag cloth, tree sway, terrain wobble, water waves and caustics | world game time | the shaders' `globals.time`, set in the render world by `render::scene_clock` | GFX-OBJ-2.9, 2.4, GFX-WATER-4.2, TRACE-5.2 (Q-313) |
| Pooled poses: settlers, animals, ships' Walk and Stand, production animals | game step | `animation::animate3d::animate` (`PosePool::advance_poses`) | GFX-ANIM-2.1, 2.5, TRACE-5.3 |
| The pose pool's 10 s sweep | game step | `PosePool::advance_sweep` | GFX-ANIM-2.4 d, TRACE-5.3 (Q-226 h) |
| Building clips: DoorOpen, DoorClose, Working, their sound events | game step | `render::view3d::buildings` | TOOL-1.2, GFX-ANIM-6.1, 6.2 |
| The door's close time, 0.6 s after the last open | game time as of the last tick: 6 ticks | `animation::object_anim::Door` | GFX-ANIM-6.1a b, TRACE-5.3 (Q-226 b) |
| Ships' bobbing phase, one turn every 4 s | world game time | `animation::object_anim::ship_phase` | GFX-ANIM-7.2, TRACE-5.3 (Q-226 c, Q-181 e) |
| Random idle waits | once per tick, 0.1 s each | `render::view3d::movers` | GFX-ANIM-3.3, TRACE-5.3 (Q-226 d) |
| A dying settler's sinking, 0.35 units/s | once per tick, 0.035 each | `render::view3d::movers` | GFX-ANIM-3.7, TRACE-5.3 (Q-226 e) |
| The evading offset and its decay, 0.75 units/s | once per tick, 0.075 each | `render::view3d::movers`, `world::scene3d::evade` | GFX-ANIM-4.3, TRACE-5.3 (Q-226 e) |
| Status icon effects (`.pfx` particles) | game step | `render::view3d::overlays::status_icons` | TOOL-1.2, GFX-OBJ-8.3a |
| The Sleep icon's 10 s idle timer | game step | `render::view3d::overlays::status_icons` | GFX-OBJ-8.3, TRACE-5.3 (Q-226 f) |
| Tree fall, felled tree sinking, deposit removal, growth | tick counts | `world::scene3d` | GFX-ANIM-3.8a to 3.8c |

## On the frame time

| What | Where | Rule |
|---|---|---|
| Camera keys in the 3D view, panning in the 2D view | `render::view3d::camera`, `input` | TOOL-1.2 |
| The refused command's 5 s red message | `ui::apply_actions` | UI-GAME-1.4, TRACE-5.3 (Q-226 g) |

## On real time

| What | Where | Rule |
|---|---|---|
| The speed indicator's frame (the second clock) | `ui::game::hud::speed_indicator` | TOOL-1.3, TRACE-5.3 (UI-GAME-2.7 says the game clock: Q-231) |
| Films (title town, intro) | `films::video` | TOOL-1.3 |
| Music and sound streams | `audio` | TOOL-1.3 |
| Screenshot file names, the save page's dates | `tools::shots`, `menus::menu::dialogs` (`SystemTime`) | not a game clock |

The fixed-function water of low shader quality (its drift and 64 frames on
the system counter, TOOL-1.3) is not built: the viewer always draws the
shader water.

## Where we differ

- ECO-TIME-1.10: a new game starts at -0.01 s with its first tick (number
  0) due at 0. Our world time starts at 0 with the first tick due after one
  tick length, so the shaders' clock and the first tick are up to 0.1 s
  apart from the original's at the start; the tick numbering is TRACE-4.5
  (Q-314).
