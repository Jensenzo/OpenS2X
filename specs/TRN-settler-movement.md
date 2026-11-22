# TRN-MOVE: Settler walking speed, slope and flag spacing

Status: draft (rules read from the binary; one in-game measurement does not yet match, see open questions)
Owner question(s): Q-019 (walking part), Q-028 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Every settler (carrier, worker, builder, soldier) walks from field to field at
one global speed. A step that goes uphill is slower, in three stages; level and
downhill steps are at full speed. Carrying a ware makes no difference. Progress
left over at the end of one step is carried into the next, so long walks take
exactly distance divided by speed. Flags must be at least two fields apart.

## Rules

Fields, neighbours and heights are as in `MAP-grid-geometry.md`. Times are in
logic ticks of 0.1 s (`ECO-time-units.md`).

### TRN-MOVE-1 Step speed

- **TRN-MOVE-1.1** The base speed is the value of `cfg_setSettlerSpeed`: progress
  per tick along one step, where 1.0 is a whole step. Shipped value 0.045. It
  is the same for all settlers and does not depend on what they carry.
- **TRN-MOVE-1.2** When a settler starts a step from field A to the neighbour
  field B, the progress per tick for that step is
  `base speed * slope factor / distance(A, B)`. On a path the distance is 1.
- **TRN-MOVE-1.3** Slope factor. If A or B has the Water state, the factor is 1.0.
  Otherwise take the height class of `height(B) - height(A)`
  (`MAP-grid-geometry.md` MAP-GRID-6.1):

  | Rise from A to B (raw height units) | Class | Factor | Ticks per field at 0.045 |
  |---|---|---|---|
  | 200 or less, including any descent | 0 or negative | 1.0 | 22.2 |
  | 201 to 600 | 1 | 0.8 | 27.8 |
  | 601 to 1200 | 2 | 0.6 | 37.0 |
  | more than 1200 | 3 and 4 | 0.4 | 55.6 |

  Only climbing slows a settler. Walking down is as fast as walking on the
  level.
- **TRN-MOVE-1.4** The factor is fixed when the step starts and depends only on
  the two field heights (MAP-GRID-4.2), not on the height samples between them.

### TRN-MOVE-2 Step timing

- **TRN-MOVE-2.1** A walking settler has a progress value, a 32-bit float. A walk
  that starts from rest begins at progress 0. Each logic tick the step's progress
  per tick, a 32-bit float set at the step's start as 1 / (distance / speed) (the
  speed the base speed times the slope factor, 0.045f = 0.0450000018 on level
  ground), is added in float32 arithmetic (Q-241; E-sim-002). The settler's movement is advanced before its job logic in the same
  tick; a walk ordered by the job logic in tick N first advances in tick N+1.
- **TRN-MOVE-2.1a The first step is rated at the order** (added 2026-11-21 pm, E-trn-043).
  Ordering a walk starts its first step at once: that step's progress per tick (speed and slope
  factor, 1.2, 1.3) is fixed from the field heights **in the tick and at the moment the walk is
  ordered**, though the settler first moves in the next tick (2.1). A height change later in
  the same tick (a leveller's step in the levellers' list, which comes after the carriers' and
  the constructors') or before the first advance does not change it. Every later step of the
  walk takes its rate when it starts (2.3). Confidence: high (code reading: ordering a walk
  builds the path and starts the first step, slope factor included; measured T14).
- **TRN-MOVE-2.2** The step is complete in the tick in which the float32 progress
  becomes greater than 1.0 (exactly 1.0 continues). The settler is then on field B.
- **TRN-MOVE-2.3** If the path continues, the next step starts in that same tick
  with progress equal to the float32 excess over 1.0 (carry-over) and with its own
  slope factor. If the path ends, the settler stops and the excess is dropped (a
  walk ordered in that tick starts at 0, ECO-WORK-3.8b).
- **TRN-MOVE-2.4** Consequences at the shipped speed, level ground: a single
  field takes 23 ticks; a path of N fields takes the number of ticks after which
  the float32 sum of 0.0450000018 per tick, carried across the steps, exceeds N:
  2 fields 45, 3 fields 67, 6 fields 134, 9 fields 200 (not 201: 0.045f is a hair
  above 0.045, Q-243).
- **TRN-MOVE-2.5** A second kind of movement exists on the fine height lattice
  (free walking off the field grid). Its progress per tick for one sample step
  is `2 * base speed * factor`, with the factor taken from the height class
  between the two samples (1 gives 0.8, 2 gives 0.6, 3 and 4 give 0.4, others
  1.0, no Water exception). A field is four sample steps, so this is half the
  field-walking speed. Builders use it between their construction positions
  (measured, E-trace-001: 0.09 per sample step on the level, 0.072 on a slight rise,
  the excess carried between the steps of one walk); which other settlers use it is
  not traced. As in 2.1, a sample step's progress per tick is **set when that step
  starts**, from the heights then: a height change on the way (a leveller at work
  nearby) changes only the steps started after it; the steps already walked, and
  where the settler is, do not change (added 2026-10-27 b, measured T14).
- **TRN-MOVE-2.6 Measured in live trace 1** (E-trace-001, a carrier on MP_2P_Storm_Coast):
  a step's progress grows by 0.045 per tick on the level and downhill and by 0.027
  (x 0.6) on a class-2 climb; the field changes in the tick progress would pass 1;
  a carrier's walk of one field ordered in tick N shows progress 0 in N, advances
  from N+1 and arrives in N+23 (level) or N+38 (class 2); the excess of a finished
  walk is dropped. This confirms 1.1 to 1.3 (classes 0 and 2), 2.1 to 2.3, T1 and T6.

### TRN-MOVE-3 Flag spacing

- **TRN-MOVE-3.1** A flag can be placed on a field only if the field passes the
  general site check, has none of the Blocked, DepositBlocked and
  MovementBlocked states, and there is no flag on the field itself or on any of
  its six neighbours.
- **TRN-MOVE-3.2** So two flags are always at least 2 fields apart (distance of
  MAP-GRID-2.1), and the shortest road between two flags has 2 steps.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| settler_speed | 0.045 | step progress per tick | high (value and use) | script `config.lua`, E-eco-004 |
| slope_factor class 1 / 2 / 3+ | 0.8 / 0.6 / 0.4 | | high | E-eco-004 |
| slope_factor descent or level | 1.0 | | high | E-eco-004 |
| water_step_factor | 1.0 | | high | E-eco-004 |
| load has no effect on speed | - | | high (code), matches E-obs-003 | E-eco-004 |
| step_complete_when | progress > 1.0 | | high | E-eco-004 |
| carry_over between steps | yes | | high | E-eco-004 |
| fine_step_speed | 2 x base x factor | sample steps per tick | medium (users not traced) | E-eco-004 |
| min_flag_distance | 2 | fields | medium (object layer taken to be the flag layer), matches E-obs-003 | E-eco-004 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | settler at rest, level ground, speed 0.045 | walk 1 field | arrives in the 23rd tick after the order (passed in live trace 1) |
| T2 | as T1 | walk a path of 2 fields | arrives in the 45th tick (first step ends in tick 23 with 0.035 carried over) |
| T3 | as T1 | walk a path of 6 fields | arrives in the 134th tick |
| T4 | A at height 1000, B at 1200 | step A to B | factor 1.0 (rise of exactly 200) |
| T5 | A at 1000, B at 1201 | step A to B; step B to A | 0.8, 28 ticks from rest; 1.0, 23 ticks |
| T6 | A at 1000, B at 1601; B at 2201; B at 3401 | step A to B from rest | 0.6, 38 ticks; 0.4, 56 ticks; 0.4, 56 ticks |
| T7 | A is Water or B is Water | step with any height difference | factor 1.0 |
| T8 | carrier with and without a ware | same step | same number of ticks |
| T9 | path of 2 fields, both steps rising 201..600 | walk it; walk it back | 56 ticks; 45 ticks |
| T10 | path of 2 fields over a ridge or through a dip (one step rises 201..600, the other descends) | walk it either way | 50 or 51 ticks |
| T14 | 2.5: E-trace-012 (Hard game), builder 90330 of the site (138, 57), flag (139, 56), waiting for material: his approach walk (`ECO-warehouse-construction.md` ECO-HQ-4.6b step 2) ordered at counter 2289, out from the flag's sample (556, 224) over (556, 225), (557, 226) to (557, 227) in 36 ticks, back from counter 2325; the heights of the out leg's samples change just before counter 2328 (afterwards the same out leg would take 34 ticks) | his position on the way back (E-trace-013, every tick from 0, the same game) | counters 2325 .. 2335: on the step (557, 227) -> (557, 226) at progress 0, 0.09, 0.18, 0.27, 0.36, .., 0.99, then (557, 226) -> (556, 225), back on (556, 224) as ordered (70 ticks in all). Working the position out each tick from the current heights (an experiment build) jumps 2 ticks ahead at counter 2328 (settlers first differ at trace tick 2330; with the rule at 4190) |
| T11 | flag at (10,10) | try to place a flag on each neighbour of (10,10) | refused for all six |
| T12 | flag at (10,10), free buildable ground | place a flag at (12,10) or (11,12) | accepted (distance 2) |
| T13 | observation E-obs-003: carrier between two flags 2 fields apart, "flat" road | time flag to flag | measured about 5.5 s both ways. The rules give 4.5 s on level ground (T2), 5.6 s when both steps climb (T9, one way only) and 5.0 to 5.1 s over a ridge or dip (T10). **Not reproduced exactly; see open questions.** |
| T14 | 2.1a: E-trace-019 (R6, the war), player 2's carrier 16797 in the finished building (55, 24) (height 1617), its flag field (55, 23) at 1617; in counter 14571 the carrier's walk out to the flag is ordered and the leveller of player 2's site (56, 22) ends its step on (55, 23) (1617 -> 2753) | the walk out's progress per tick | 0.045 (level, the heights at the order): at the flag in counter 14594. Rating the step at the first advance (14572, after the levelling: class 2, 0.027) makes the war's settlers differ from trace tick 14580; with the rule 15170 (experiment build on 0.12.3 with the rules of 2026-11-21; every Storm Coast pair unchanged; no other first step in the war's first 15 160 ticks changes) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | follow tables (`follow/settlers.csv`), carrier 90399 and builder 90317 | TRN-MOVE-2.5, 2.6 |
| E-eco-004 | code reading | `S2DNG.exe` | start of a field step: speed from the settings, slope factor, distance |
| E-eco-004 | code reading | | slope factor from Water states and height class |
| E-eco-004 | code reading | | progress per tick, completion test, carry-over, reset |
| E-eco-004 | code reading | | order inside a settler's tick: timer, movement, then job logic |
| E-eco-004 | code reading | | fine-lattice movement |
| E-eco-004 | code reading | | flag site check: states and the scan of the field and its ring |
| E-eco-005 | data survey | 34 release maps, steps between neighbouring `ForBuilding` fields | 53.8 % class 0, 23.1 % climbing (12.2 / 6.9 / 3.4 / 0.6 % for classes 1 to 4), 23.1 % descending: ground that looks flat is often not class 0 |
| E-obs-003 | in-game observation | Jens's observation notes, 2026-10-13 | carrier, 2 fields, about 5.5 s, same with and without a ware |
| E-trn-043, E-trace-019 | code reading, recording, experiment build | (session 2026-11-21 pm): R6's settlers table (carrier 16797, leveller 18926) and height windows | 2.1a the first step's rate at the order (T14) |

## Open questions

- E-obs-003 gives about 2.75 s per field; level ground should give 2.2 to 2.3 s.
  A climb of class 1 (rise above 200 units, a grade of only 3.3 %, not visible
  by eye) gives 2.8 s, but then the way back should be faster, which was not
  seen. Either the measured road had such a rise and the two directions were
  not timed separately, or a rule is missing. Measurements needed are listed
  in Q-028.
- Which settlers use the fine-lattice movement (TRN-MOVE-2.5).
- Whether roads or buildings change the terrain height under them (which would
  change slope factors after construction).
