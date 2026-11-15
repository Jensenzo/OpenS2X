# ECO-TIME: Simulation tick and time units

Status: draft (tick mechanism verified in the binary; production, rest, pickup, drop and walking units verified in the binary; production time confirmed by measurement)
Owner question(s): Q-003, Q-014 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

The game logic is tick-based. Real time is collected every frame, scaled by a
pause factor and a speed factor, and consumed in fixed steps of one logic tick.
One tick is 0.1 seconds of game time, so the simulation runs at 10 ticks per
second at normal speed. Durations in the property scripts
(`data/scripts/properties/*.lua`) are counted in logic ticks, speeds are amounts
per tick.

## Rules

### ECO-TIME-1 Tick driver

- **ECO-TIME-1.1** The logic keeps three clock values: the scaled time step of
  the current frame, the logic time at which the next tick is due, and the
  elapsed logic time. It also keeps an integer tick counter that starts at -1.
  All four are part of the saved world state (`FMT-s2m-map.md` FMT-S2M-3).
- **ECO-TIME-1.2** Each frame: `step = frame_seconds * time_scale * speed_factor`.
  `frame_seconds` is the real duration of the last frame, measured in
  milliseconds and converted to seconds.
- **ECO-TIME-1.3** If `step` is larger than 5.0 seconds, it is replaced by exactly
  one tick length (protection against long stalls). Measured (E-trace-001): after 199 s
  with the game's window minimized, the first frame's step was exactly 0.1 and one
  tick ran.
- **ECO-TIME-1.4** `elapsed += step`. Then, while `next_tick_time <= elapsed`: run
  one logic tick (ECO-TIME-1.5) and add one tick length to `next_tick_time`.
  Several ticks can run in one frame; no tick is skipped.
- **ECO-TIME-1.5** One logic tick: increase the tick counter by one (the first
  tick is number 0), process the queued player commands for that tick, then
  update the simulation subsystems once each. Measured (E-trace-001): the counter is
  -1 when the world appears and 0 in the first tick.
  The parts of a tick run in this order (E-ai-007): the counter, the
  players' route checks, the **buildings** (each player's update: sites, requests,
  settlers sent out of a warehouse for a site, building or flag), the roads, the map
  change notifications, the **settlers** (movement, then job logic), the **transport**
  (packages, the warehouses' request service: road carriers, donkeys and boat
  carriers sent out), the resources (deposits, animals, respawn), the doodads, the
  military, the navy, decay, the triggers, and the computer players last
  (AI-2.1a). So a settler created by the buildings' part (a constructor, leveller,
  worker, specialist, soldier) moves in its creation tick, while a carrier created
  by the transport's part first moves in the next tick (Q-246: the computer
  player's constructor is at 0.045 in its first row, trace tick 178; its first road
  carrier at progress 0 on the door field in its first row, 208, and at 0.045 in
  209; E-trace-002). Confidence: high.
- **ECO-TIME-1.6** In a network game a tick may be held back when the commands
  for it have not arrived. In that case the loop stops for this frame and the
  elapsed time is pulled back so that no logic time is lost or gained.
  (Mechanism seen; conditions not traced. See Q-003 remainder.)
- **ECO-TIME-1.7** `time_scale` is 1.0 while the game runs and 0.0 while it is
  held (for example while a loading or end screen is up). It is not a user
  speed setting, and the pause key does not use it: it stayed 1.0 while paused
  (measured, E-trace-001; see 1.8).
- **ECO-TIME-1.8** `speed_factor` is 1.0 by default. It is the value the in-game
  speed controls change. Measured (E-trace-001): 1, 2 and 3 from the speed key, and
  0 while the game is paused (P), back to the previous value on resume.
- **ECO-TIME-1.10** When a world is loaded, an elapsed time that is not smaller
  than the next tick time is set to the next tick time minus 0.01
  (`FMT-s2m-map.md` FMT-S2M-3.1b). A new game from a map therefore starts with
  elapsed time -0.01 and next tick time 0: the first tick runs in the first
  frame that brings the elapsed time to 0 or more. Measured (E-trace-001): -0.01 and 0
  in both games; the first tick ran at elapsed 0.013 and 0.016.
- **ECO-TIME-1.9** Ticks have constant length in game time. Speeding the game up
  runs more ticks per real second; it never changes what one tick does. This
  makes the simulation deterministic for a given command stream.

### ECO-TIME-2 Tick length

- **ECO-TIME-2.1** The tick length is the configuration value that the property
  function `cfg_setGameSpeed` sets, in seconds.
- **ECO-TIME-2.2** Its built-in default is 0.1. No shipped script calls
  `cfg_setGameSpeed`, so the tick length in the shipped game is 0.1 s.
- **ECO-TIME-2.3** Map scripts can define `onLogicTick(seconds)`; the name of the
  parameter confirms that script-visible time is in seconds, not ticks.

### ECO-TIME-3 Units in property scripts

- **ECO-TIME-3.1** Durations are numbers of logic ticks. Where the game needs a
  counter it converts the script number to an integer by truncation, sets a
  countdown to that value and decreases it by one on each logic tick.
  (Verified for `cfg_setConstructTime`: one construction work step lasts that
  many ticks.)
- **ECO-TIME-3.2** Fractional durations are allowed in scripts (pickup and drop
  time are 3.3, fighting time is 16.7). A settler's timed action keeps the
  number as it is, subtracts 1 per tick and ends when the result is zero or
  less, so a duration `D` lasts `ceil(D)` ticks: 3.3 lasts 4 ticks
  (`ECO-worker-cycle.md` ECO-WORK-2.1; verified for production, pickup and
  drop time). A rest time `R` lasts `R + 1` ticks (ECO-WORK-3.9). The fighting
  time is not traced.
- **ECO-TIME-3.3** Speeds are per tick. Movement speeds (`cfg_setSettlerSpeed`,
  `an_setMovementSpeed`, `sh_setSpeed`) are the progress added per tick along one
  field-to-field step, where 1.0 is a whole step: a settler with speed 0.045
  needs 1 / 0.045 = 22.2 ticks = 2.2 s per field on level ground. Uphill steps
  are slower; the full rules, including carry-over between steps, are in
  `TRN-settler-movement.md`. (The same value is found as the speed of walking
  animals stored in map files.)
- **ECO-TIME-3.4** Growth speed (`dp_setGrowingSpeed`) is size gained per tick on
  a scale where 1.0 is fully grown. The shipped grain field value 0.00075 gives
  1333 ticks from 0 to 1 (the script's own comment says so), about 133 s; it
  starts at `dp_setGrowingStartSize`.
- **ECO-TIME-3.5** The named time classes in `config.lua` (`ExtremlyShortTime` 50
  to `ExtremlyLongTime` 400) are plain Lua variables for use by other scripts;
  they are ticks: 5 s to 40 s.

### ECO-TIME-4 Conversion table

At the shipped tick length. "Verified" = traced in the binary; "by analogy" = same
kind of value, same scripts, not individually traced.

| Setter | Unit | Shipped example | In seconds | Status |
|---|---|---|---|---|
| `cfg_setGameSpeed` | seconds per tick | (default 0.1) | 0.1 | verified |
| `cfg_setConstructTime` | ticks per construction work step | 55 | 5.5 | verified |
| `cfg_setBulldozeTime` | ticks | 55 | 5.5 | by analogy |
| `cfg_setPickupTime`, `cfg_setDropTime` | ticks, rounded up per action | 3.3 | 0.4 (4 ticks) | verified |
| `cfg_setSettlerDyingTime` | ticks | 28 | 2.8 | by analogy |
| `cfg_setSettlerSpeed` | step progress per tick | 0.045 | 2.2 s per field, level | verified (code); one measurement open, see `TRN-settler-movement.md` |
| `bp_setProductionTime` | ticks of the work action (not the whole cycle, see `ECO-worker-cycle.md`) | 333 | 33.3 | verified and measured (about 33 s, E-obs-001) |
| `bp_setConstructionTime` | ticks (-1 = not used) | -1 | - | by analogy |
| `gp_setRestTime` | ticks a worker rests in each cycle, plus 1 | 55 / 222 | 5.6 / 22.3 | verified |
| `mi_setFightingTime` | ticks per fight round | 16.7 | 1.7 | by analogy |
| `rm_setBurningTime` | ticks a ruin burns | 333 | 33.3 | by analogy |
| `sh_setConstructTime` | ticks per ship building step | 55 | 5.5 | by analogy |
| `sh_setSpeed`, `an_setMovementSpeed` | step progress per tick | 0.18 | 0.56 s per field | data cross-check |
| `dp_setGrowingSpeed` | size per tick | 0.00075 | 133 s to full size | script comment |
| `dp_setLifeTime`, `dd_setLifeTime` | ticks | 3000 / 10000 | 300 / 1000 | by analogy |
| `cfg_setCatapultRotationTimeFactor`, `cfg_setCatapultFlightTimeFactor` | factor, ticks per unit of angle / distance | 11 | - | name only |

### ECO-TIME-5 Arithmetic precision (Q-140)

- **ECO-TIME-5.1** The game's floating-point logic runs with every arithmetic
  result rounded to 32-bit float precision (24-bit mantissa, round to
  nearest even): the graphics device is created without asking to keep the
  processor's floating-point mode, which then stays at single precision for
  the game thread. So a sum like `acc + 1/n` is two float32 operations: `1/n`
  rounded to float32, then the sum rounded to float32. An implementation
  must use float32 for every step of such a formula, not a wider type with a
  single rounding at the end. Confidence: medium-high (device flags read;
  no other change of the mode found; the mode is not observed at run time).
- **ECO-TIME-5.2** Integer conversions in the logic truncate towards zero;
  "rounded" values in the specs are `trunc(x + 0.5)` unless said otherwise.
  Confidence: high for the places traced.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| tick_length_default | 0.1 | seconds | high | E-fmt-002 |
| ticks_per_second | 10 | at speed 1.0 | high | derived |
| max_frame_step | 5.0 | seconds | high | E-fmt-002 |
| frame_time_resolution | 0.001 | seconds | high | E-fmt-002 |
| initial_tick_counter | -1 | | high | map files, E-fmt-002 |
| speed_factor_default | 1.0 | | medium | E-fmt-002 |
| time_scale_running / held | 1.0 / 0.0 | | medium | E-fmt-002 |
| default_settler_speed (before scripts) | 0.1 | per tick | high | E-fmt-002 |
| default pickup / drop / bulldoze / construct time (before scripts) | 10 | ticks | high | E-fmt-002 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | new game loaded from a map (tick counter -1, elapsed -0.01 after ECO-TIME-1.10, next tick time 0), speed 1.0 | feed one frame of 0.016 s | one tick runs (next tick time 0 <= elapsed 0.006); counter 0; next tick time 0.1 |
| T1a | as T1 | feed one frame of 0.005 s instead | no tick yet (elapsed -0.005) |
| T2 | continue T1 | feed frames of 0.016 s until elapsed >= 0.1 | second tick runs in the frame where elapsed reaches 0.1 (the 7th frame overall: -0.01 + 7 x 0.016 = 0.102) |
| T3 | running game | feed one frame of 0.35 s | 3 or 4 ticks run in that frame, exactly as many as needed to make next tick time > elapsed |
| T4 | running game | feed one frame of 6.0 s | step is replaced by 0.1 s: exactly one tick runs |
| T5 | time scale 0.0 | feed any frames | no tick runs, counter unchanged |
| T6 | speed factor 2.0 | feed 1.0 s of frames | 20 ticks run |
| T7 | construct time 55 | start a construction work step | step completes on the 55th tick after it started |
| T8 | settler speed 0.045, level ground | walk one field | arrives during the 23rd tick (22 full increments are not enough); see `TRN-settler-movement.md` T1 to T3 |
| T10 | float32 per step (ECO-TIME-5.1) | 0 + 1/30 three times, compared with 0 + 1/10 | 0.10000001 > 0.1 (with one rounding at the end the two would be equal) |
| T9 | growing speed 0.00075, start size 0.2 | let a field grow | reaches size 1.0 after 1067 ticks (0.8 / 0.00075) |

T7 follows the verified countdown (ECO-TIME-3.1) and T8 the verified walking
code. T9 assumes plain accumulation per tick and is not verified.

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | clock watches (`clock-realtime.csv`, `clock-fixed.csv`), generator watch, pause and speed log | ECO-TIME-1.3, 1.5, 1.7, 1.8, 1.10 |
| E-fmt-002 | code reading | `S2DNG.exe` | tick driver: scaling, 5 s clamp, catch-up loop, counter |
| E-fmt-002 | code reading | | frame time in seconds from milliseconds |
| E-fmt-002 | code reading | | built-in defaults of the configuration block |
| E-fmt-002 | code reading | the `cfg_*` handlers | which configuration slot each setter writes; values stored unconverted |
| E-fmt-002 | code reading | | construction step: integer countdown per update |
| E-fmt-002 | code reading | | log line naming tick counter and logic time |
| E-fmt-002 | code reading | | time scale set to 0 / 1 around screens |
| E-eco-001 | script comment | `data/scripts/properties/deposits.lua`, grain field | "2400 ticks...now 1333" next to growing speed 0.00075 |
| E-eco-002 | map data | animals stored in mid-walk in `.s2m` files | interpolator speed equals 0.045 / 0.18, matching the script speeds |
| E-eco-003 | script | map scripts define `onLogicTick(seconds)` | script time is in seconds |
| E-fmt-003 | code reading | | clock correction on load |
| E-eco-004 | code reading | | production, drop, pickup and rest time and settler speed as used by the settler code |
| E-mil-005 | code reading | | precision of the logic arithmetic |
| E-obs-001 | in-game observation | Jens's observation notes, 2026-10-13: sawmill, sawing start to plank | about 33 s for a production time of 333: 0.1 s per tick holds for production times |

## Open questions

- The production-time part of Q-019 is closed by E-obs-001. Remaining
  measurements are in Q-028.
- Values and steps of the user speed control; network tick ("NetTick") length
  and how many logic ticks one network turn spans. (rest of Q-003)
- How the fighting time (16.7) is applied.
- A production cycle is more than the production time; the cycle of workshop
  and mine workers is in `ECO-worker-cycle.md`, the other buildings are open.
  (Q-005)
