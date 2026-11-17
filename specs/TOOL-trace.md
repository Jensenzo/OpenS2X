# TOOL-trace: Tracing the running original

Status: draft; from code reading, then partly measured in live trace 1 (2026-10-18
night, E-trace-001, memory reads and one data write, no debugger). Rules marked
"measured" are high; the rest stay medium.
Owner question(s): Q-306 (protection), Q-305 and Q-226 (clocks, TRACE-5), trace preparation 2026-10-18; raises Q-308 to Q-310, Q-313; live trace 1 raises Q-314 to Q-317; live trace 2 maps Q-309 (TRACE-3.7)
Game build: 11757

## Summary

To measure the original tick by tick, a separate program reads the running game's
memory and writes one row per object per tick. This spec says where one tick ends,
what the original's objects hold, the reader's CSV columns (the recordings' format),
which clock each moving thing follows, how recordings and pictures of the original are
made, and the order of the first checks. How the original reacts to being watched and
where its objects lie in memory are kept in the investigation's private notes (TRACE-1, TRACE-3.6). The simulation's random generator is specified in SIM-RNG. Live trace 1
ran the reader against the game: the memory map, the tick boundary, the fixed step, the
between-ticks snapshots and the scene clock all held as read; one note was wrong (the
pause is the game speed, not the world's time scale).

## Rules

### TRACE-1 Start-up, and how the original reacts to a tracer

- **TRACE-1.1** Whether the original looks for a debugger: kept in the investigation's private notes (method of the investigation, no rule of the game).
- **TRACE-1.2** What the original measures time for: kept in the investigation's private notes (method of the investigation, no rule of the game).
- **TRACE-1.3** Whether the original checks its code or data in memory: kept in the investigation's private notes (method of the investigation, no rule of the game). Writing the fixed-step byte of TOOL-4.2 is not noticed. Confidence: high (measured: written once in live trace 1, the game ran on for about 30 minutes with only the intended effect).
- **TRACE-1.4** The leftovers of a copy protector in the executable: kept in the investigation's private notes (method of the investigation, no rule of the game).
- **TRACE-1.5** Start-up, in order: the working directory is set to the install
  folder (the executable's folder's parent), so the launcher's directory does not
  matter; the process restricts itself to the lowest-numbered CPU it may use; a
  named mutex `settlers2` is created, and if it already exists the new process exits
  at once without a message; then the window opens and the main loop starts.
  Confidence: high for the order and the mutex; high for the CPU pinning.
- **TRACE-1.6** The main loop's exception guard and the game's log output: kept in the investigation's private notes (method of the investigation, no rule of the game).
- **TRACE-1.7** The tracing methods ranked from safest: kept in the investigation's private notes (method of the investigation, no rule of the game).
- **TRACE-1.8** What a debugger stop does to the frame clock: kept in the investigation's private notes (method of the investigation, no rule of the game).
- **TRACE-1.9** The tracer's CPU affinity: kept in the investigation's private notes (method of the investigation, no rule of the game).
### TRACE-2 Where one tick ends

- **TRACE-2.1** The game world is one object with a tick number (a signed 32-bit
  value, -1 on a fresh map) that goes up by one at the **start** of each tick, before any part of the world is
  updated; the first tick of a new game is number 0. Confidence: high (measured: -1
  when the world appears, 0 in the first tick; the number is raised while next tick
  time has not yet grown, and a game stopped inside a tick (TRACE-2.7) shows the new
  number with the previous tick's random state).
- **TRACE-2.2** Within one tick the world is updated in this order: per-player
  command and route state, players, a map-side update, buildings (all, then
  ruins, then the building queue), flags and streets, map change notifications,
  settlers (the six lists of TRACE-3 in their order), wares, resources, doodads,
  military, ships, statistics, triggers and scripts, and last the computer players.
  Confidence: medium (the order of the calls is certain; what each call covers is
  named from earlier readings).
- **TRACE-2.3** The simulation of tick T is complete right after the computer players'
  update. A skipped tick (TRACE-2.5) passes the same point, so a tracer that stops there
  records tick T only when the tick number differs from the last one recorded. Right
  after it the per-tick update of the settlers' view objects (animation choice, world
  position) runs; the same test applies there. Confidence: medium.
- **TRACE-2.4** After the simulation of a tick, the world's "next tick time"
  grows by the tick length; the frame's game time grows at the start of each
  frame. Ticks run while next tick time <= game time. So whenever next tick time >
  game time the world is between ticks. A tracer without breakpoints reads tick number,
  next tick time and game time, then the objects, then the three values again, and
  keeps the snapshot only if both reads are equal and next tick time > game time; it
  retries otherwise. Confidence: high (measured: over 300 ticks the busy reads were
  0.58 % of 7.4 million at real time and 0.20 % with the fixed step, a busy window lasts
  0.14 ms (median; a slow tick 132 ms); 200 snapshots were all accepted, each with the
  tick of the clock reads around it; next tick time grows by exactly one tick length
  per tick (0.099998474 or 0.1000061 in float32, depending on its size)).
- **TRACE-2.5** A tick is skipped (time still advances, no update) while the world's
  hold flag is set and no tick runs at all
  while the lockstep gate refuses (multiplayer; in single player it is expected to
  pass). A frame whose game time step exceeds 5 s runs exactly one tick
  (ECO-TIME-1.3). Confidence: high for the 5 s rule (measured: after 199.34 s away the
  first frame had a step of exactly 0.1 and ran one tick); medium for the hold flag and
  the gate (not exercised).
- **TRACE-2.6** View objects and pooled animation clocks advance once per drawn frame
  by the scene's game time step, not per tick (TOOL-1.2). Under the fixed step at
  speed 1 a tick spans 4 frames; a between-ticks snapshot can fall after any of
  them, so a clip time read without a breakpoint is only known to within one tick.
  Read clip times right after the view objects' update (TRACE-2.3) for exact values. Because game time
  and next tick time are float32 sums (of 0.025 and of 0.1), their rounding drifts
  apart: about one tick in 300 takes 5 frames instead of 4 (drift the other way would
  give 3). A model of the two sums from one read of both values predicts which. Confidence: high (measured: 4
  frames in 299 of 300 ticks, the one 5-frame tick predicted by the float32 model;
  clip times read between ticks advanced by 0.075, 0.1 or 0.125 per tick).
- **TRACE-2.7 A minimized game stops inside a tick.** Leaving the game's window
  (it minimizes; it cannot run in a window behind others) stops the game after the
  tick number of the current tick has been raised and before the tick's first random
  draw; next tick time stays at or below game time, so no between-ticks snapshot can
  be taken until the window is back. Then the tick finishes and the whole time away
  arrives as one frame; when it is longer than 5 s, TRACE-2.5's rule turns it into one
  tick. Seen once (E-trace-004): a window that has lost the foreground but is not yet
  minimized does not stop; with the fixed step on its frames ran unpaced and game time
  raced (about 1500 ticks per second for 1.6 s) until the window was minimized (Q-325).
  Since 2026-10-19 the recorder guards against it: a generator row recorded while the
  game's window is not in front carries the note `not in front`, and when the window
  is not in front, not minimized, and more than 40 ticks passed in the last second,
  all three recorders stop (`STOP` file, "window not in front and the game races").
  A tracer must
  keep the game in front for every tick it records. Confidence: high (measured five
  times, the return after 199 s once); where in the tick it stops is not known
  (Q-315).

### TRACE-3 What the original's objects hold

- **TRACE-3.1** How the original keeps its lists of objects in memory: kept in the investigation's private notes.
- **TRACE-3.2** Object ids are 64-bit; ids are handed out from one global counter in
  creation order, so the id order is the creation order. A reference to another object
  holds that object's id, or a value that means "none". Confidence: high (measured: new settlers get rising ids;
  street references hold the flags' and carriers' ids; two games in one session give
  every object the same id).
- **TRACE-3.3** An object's owner is a player, identified by its index 0 .. 7.
  Confidence: high (measured).
- **TRACE-3.4** A settler's sim position is its field and its fine
  lattice point, both two signed 32-bit values x then y. While it
  walks, a step goes from one field (or fine point, in fine mode) to the next with a
  progress 0..1 (-1 when standing); the view's world position is the
  linear blend of the two points by that progress. The field stays the step's start
  field until the tick in which progress would pass 1; outside fine mode the fine point
  is the field's height sample (MAP-GRID-4.2), in fine mode the field follows the fine
  point (MAP-GRID-4.3). Confidence: high for field, fine point, progress and the two
  points (measured); medium for the view's world position (not read).
- **TRACE-3.5** A building is finished when its levelling progress is >= 1 and both
  its frame and main construction remainders are <= 0; the two remainders run from
  1 to 0 while it is built. Confidence: high (measured: a sawmill site is finished in
  the tick its main remainder falls below 0).
- **TRACE-3.5a An upgrading building.** A military building being upgraded
  (`MIL-building-upgrade.md` MIL-UPG-2) keeps its own type and its own finished
  construction fields: the upgrade's construction and levelling are separate objects.
  So its trace row shows the **old type, `finished`, 1000** for the whole upgrade, and
  the new type, `finished`, 1000 from the tick the type changes (MIL-UPG-2.4); the
  upgrade's levelling and building progress do not appear in the B rows. Confidence:
  high (E-trace-010: the guard house 90300 on (138, 57) upgraded at counter 5582, its
  leveller 90750 on the way from trace tick 5583, the row `finished` 1000 through
  6000; E-trace-002: the same building shows type 7d00b493 `finished` from trace tick
  10 730 with no site rows between; code E-ai-006).
- **TRACE-3.6** The reader's layout table (every global, object and field it decodes,
  with its confidence: "high (measured, E-trace-001)" where live trace 1 decoded the
  field correctly, medium otherwise; the structures from Resources to Ship added for
  Q-309 in the night of live trace 2) is kept in the investigation's private notes. What the recordings' columns mean
  is TRACE-4.3, what the objects hold TRACE-3.2 to 3.5 and 3.7. Measured with the
  height layer (E-trace-005): the layer read at trace tick 0 equals the map file's except
  the 169 samples around the first castle, at trace tick 1 except the 338 around both
  (their instant levelling, ECO-HQ-4.8a).

- **TRACE-3.7 Wares, trees, stones, animals, ships (Q-309).** Read from the save and
  update code in the night of live trace 2, then checked once against the running game
  (one read-only snapshot) and against the recording's raw dumps; every structure of
  the layout table from Resources to Ship.
  - Trees, stones and grain fields are deposits: one list in the resources system. Each has a type (its first word is the property id; tree types 1 to 8,
    stone types 1 to 6 and the grain field seen), a field, a growth 0 .. 1, a removal counter
    (20 while being removed), a lifetime (-1 unlimited) and the building that reserved
    it. A second list holds only the deposits that grow or are being removed.
  - Animals are kept in a map in the resources system. Each has a kind (deer, rabbit and
    elk seen), a home field (the centre its roaming radius is measured from, not its
    position), a position (the start field of its current step), the step's end field,
    the walk's goal, a step progress (-1 standing; 0.045 per tick for one kind and 0.07
    for another seen) and a state (0 waiting with a countdown, 1 choosing a goal, 2
    walking). As for settlers, the position moves on when the progress passes 1.
  - Wares are packages, one list in the transport system. Each has an
    owner, a good, the flag whose ware list holds it, the carrier that claimed or carries
    it, the destination building, the next flag's position, a lying-at-the-flag mark,
    an age counter and a wait score. A ware lies at a flag (mark set, no carrier), is
    claimed (carrier and flag set, mark clear) or is carried (carrier set, no flag). A
    flag's ware list has one entry per ware whose flag is that flag.
  - Goods inside buildings are counts per good, not objects. Every building has two
    stocks: a workplace's inputs, and a warehouse's store (which also counts the
    settlers kept there as goods). Every player has two: the sum of its warehouses'
    stores, and a second one that looks like goods out of store (not settled).
  - Ships are one list in the navy: owner, id, two construction
    remainders (built when both are 0 or less; the second one runs down first), field,
    step progress, a state 0 .. 8 (2 while sailing), the harbour field it heads for, and
    a cargo stock.
  Confidence: medium. Supporting checks: all 15 deposit, 3 animal and 6 ware type ids
  read name real property entries; each flag's ware count equals the number of wares
  naming that flag (seven flags, raw dump of trace tick 5000); a player's first stock
  equals the sum of its warehouses' stores; three walking animals followed for 45 ticks
  stepped as stated; the 6 and 11 ships in the raw dumps of trace ticks 100 000 and
  150 000 decode with plausible owners, rising ids, fields on water, progress -1 when
  standing and between 0 and 1 in state 2, and one ship under construction at the
  shipyard.
- **TRACE-3.8 Start of a new game.** The world object appears (counter -1) before the
  start objects exist. The start sequence first resets the simulation generator, then
  creates the players with their castles, flags, garrisons and start stocks while the
  counter still reads -1; the first update follows. A snapshot taken at counter -1 can
  therefore hold only part of the start objects. Creating the players drew nothing from
  the generator on MP_2P_Storm_Coast (it was at the seed at the first update), and
  because the reset comes first, a start choice that did draw could not depend on
  earlier games (Q-312). Confidence: high for the partial snapshot (measured, live
  trace 2: the last counter -1 snapshot, under 50 ms before the first update, held
  player 0's castle only, while the first update's held both castles, both flags and
  both garrisons); medium for the order (static).

### TRACE-4 Trace reader and CSV columns (proposal, Q-310)

- **TRACE-4.1** The reader (investigation side) opens the game for reading only and
  takes one between-ticks snapshot as in TRACE-2.4, or one per tick in follow mode,
  reporting ticks it missed. Everything it decodes comes from one layout table
  (TRACE-3.6), so a wrong entry is fixed in one place. It needs no change to the game. Confidence: high
  (measured: 300 ticks followed without a gap under the fixed step; one full snapshot
  with views takes about 2 ms for 30 settlers).
- **TRACE-4.2** Output: one CSV table per kind, each row starting with the tick number
  of the snapshot. Floats are written as the shortest decimal text that reads back to
  the same 32-bit value. Ids are decimal 64-bit values, empty for "none". Type and job
  ids are 8-digit lower-case hex. Fields are x then y. Confidence: medium.
- **TRACE-4.3** Columns:

  | Table | Columns |
  |---|---|
  | settlers | tick, list (workers / constructors / carriers / bulldozers / soldiers / specialists), id, class, job, owner, state, field_x, field_y, fine_x, fine_y, fine_mode, progress, from_x, from_y, to_x, to_y, anim_code, anim_left, anim_total, view_base, view_clip, clip_time |
  | buildings | tick, id, type, owner, field_x, field_y, levelled, frame_left, main_left, finished, enabled |
  | flags | tick, id, owner, field_x, field_y, links |
  | streets | tick, id, owner, type, flag_a, flag_b, carrier, carrier2, water, donkey_road, points (space-separated "x,y" from flag A to flag B) |
  | players | tick, index, active, tribe, name |
  | animals (from 2026-10-19, tick-for-tick run 2) | tick, id, type, home_x, home_y, field_x, field_y, to_x, to_y, goal_x, goal_y, reg_x, reg_y (the field the animal is registered on, ECO-FOOD-5.10a), progress, walk_start (the tick clock at the walk's start, ECO-FOOD-5.11b), state, counter; every animal, in id order, in every snapshot |
  | heights (from 2026-10-20, E-trace-005, Q-335) | tick, building (its id), site_x, site_y, c, r, h: the height samples (sample column c, row r of the (4w + 1) x (4h + 1) grid, MAP-GRID-4.1) of the rectangle rows 4y - 12 .. 4y + 12, columns 4x - 14 .. 4x + 14 around the building's field (x, y), clipped to the grid; written when the building first appears in a snapshot and whenever its `levelled` value changes. The recorder also writes the whole layer at trace tick 1 and at the last tick (`heights-full-<trace tick>.bin`, cols x rows little-endian i32, row-major, with a `.json` of cols, rows and floor) |
| ai_street (from 2026-10-20, Q-329) | tick, slot, cursor (index into the player's flag list, AI-CON-8.1), flag_id, flag_x, flag_y (the flag being worked on), giveups, excluded (count of excluded positions), best_len, best_len2, best (strategy), attempt (1..6), t_x, t_y (the remembered target flag T, -1 none), w_x, w_y (remembered waypoint); one row per active computer player in every snapshot |
| ai_route (from 2026-10-24 night, Q-353; first recorded in E-trace-008) | tick, slot, cursor, cursor_hi (the building cursor, a 64-bit id), k (partner index), i (route flag index), t (tries), w (path field index), x_count, x_last_x, x_last_y (the tried flag positions X and its last entry): the route optimizer's task (AI-CON-8.3); one row per active computer player in every snapshot |
| ai_plan (from 2026-10-25 day; first recorded in E-trace-009) | tick, rb_state, stub, stub_hi, from_x, from_y, to_x, to_y, s_state, cost, crc, steps, path: the computer players' road builder (AI-CON-8.1 b, 8.3 c): rb_state (5 begun, 3 extended, 1 no path, 4 points taken, 6 cancelled), the stub road's id (-1 once removed), and its search object: from, to, s_state (2 path found, 3 none), cost (the label of `from`), crc (crc32 of the label array: a new value means a new search) and the path read out from the labels as the game does without draws (TRN-FREE-3.2, first allowed direction), from `from` to `to` inclusive. The labels stay until the next search, so the last call before each snapshot is caught; two calls in one tick (a route optimizer variant building two roads) show only the second. One row per snapshot |
| ai_walk (from 2026-10-25 day; first recorded in E-trace-010) | tick, mode (road-stop mode), hit (its low word), target (its low word), from_x, from_y, to_x, to_y, s_state, cost, crc, steps, path: the computer players' walk pathfinder (AI-CON-8.1e walk paths and road-stop searches, AI-CON-8.3 c), its last search read as in `ai_plan`. One row per snapshot |
  | objects (from 2026-10-23, E-trace-006, Q-340) | tick, snap, kind (request / ware / netid / need / production / cell / deposit / doodad0 / doodad1 / doodad2 / remains / ship), id, event (new / gone), detail (request: good, its four reference ids, queue position; ware: good, destination, flag, carrier, owner, lying; netid: the network kind and the flags holding the id; need / production / cell: slot, good, references, cell, centre; doodad: type and field): one row per object that appeared or vanished between two snapshots (the first snapshot lists everything; at counter -1 every snapshot is compared, giving the loading order). Script triggers are not read (SIM-START-4.2) |
  | counter (same) | tick, snap, next_id, next_id_hi (the object counter after the tick), n_<kind> (the count of each kind above), n_goodflags (good-flag records of all warehouses), errors (kinds the reader could not read in that snapshot) |
  | goodflags (same) | tick, building, owner, count, entries (`good:stop:out`, sorted by good id) for every warehouse-type building |
  | counter-menu (same; gen mode, plain csv) | t_s, app_state, world, next_id, next_id_hi, rng_state: one row on every change of the counter or of the world pointer at the main menu and during loading (SIM-START-4.5) |

  `from` / `to` are the current step's two points: fields in rough mode, fine points
  when fine_mode is 1. `progress` is -1 when the settler stands. `view_base` is the
  clip family the view chose (0 Stand, 1 Walk, 2 SwimIdle, 3 SwimFwd, 4..6
  WorkLoop0..2, 7 Pickup, 8 PutDown, 9 Hit), `view_clip` the resolved clip, and
  `clip_time` the clock of the pose it plays (shared poses report the same clock,
  GFX-ANIM-2). Rows are in the game's own list order (update order). Confidence:
  medium. `view_clip` is the clip's id in the animation sets (2026-10-25, E-heap-002):
  a freshly started game process numbers the donkey's 30 clips 0 .. 29 and the
  settlers' 104 clips 30 .. 133 in the order of the XML entries (E-trace-005, 007,
  008); a process that has loaded them before has all ids moved by one offset
  (E-trace-002: 391, so Stand is 421). The names come from a memory read of the same
  kind of session. Confidence: high.
- **TRACE-4.4** The comparison format is s2-reborn's `docs/trace-format.md` (Q-310):
  the reader keeps its tables and the investigation's converter derives the format's settler and building rows from them, as that file's section
  "From the original's tables" says. Its `clip_time` (ticks with the same animation)
  is counted from the trace itself and is empty until the first change of animation
  seen (and again after a tick the trace skips). Its ticks are numbered by TRACE-4.5.
  Confidence: high (used in live trace 1). The `animation` column comes from the
  settler's view (`view_base`), which the game updates with its frames, not with the
  simulation: at the start of a timed action it can lag the simulation by a tick in
  one recording and not in another (E-trace-002's carrier 90282 at counter 1761:
  `view_base` 0 against 8 in E-trace-010, with `state`, `anim_code` and `anim_left`
  equal). The dense recording (`traces/original-dense-6000.s2trace.gz`) is the reference for it; a first
  difference against E-trace-002 that is only this column at an action's first tick
  is not a simulation difference (2026-10-25 evening).
  **How often, and when (2026-11-17 day; E-trace-011, E-trace-012, the recordings' own tables):**
  within one recording the view's switch is late by one tick very rarely, and only in a tick
  whose frames advanced the view clock by 0.075 instead of 0.1 (TRACE-2.6's 3-frame tick): in
  E-trace-011, 2 of about 5 560 walk ends kept the walk view in their arrival tick (carriers
  90288 at counter 17532, 91756 at 18974); in E-trace-012, 2 of about 10 060 put-down and
  pick-up starts showed the new clip only in the tick after the request (carrier 90623 at counter
  7053, 91768 at 17797). Most 0.075 ticks switch in time, so the frame timing alone does not
  predict it (medium); the simulation's own columns (`progress`, `anim_code`, `anim_left`) are
  on time in all four. So the first differences of the dense pairs at trace ticks **17533**
  (`original-dense-20000`, carrier 90288 one tick in Walk) and **7054**
  (`original-hard-dense-20000`, carrier 90623 one tick in Stand before PutDown) are this view
  lag, not a game rule: the program shows the simulation's state, and is right there. Test T16.
  **What is left of the three pairs past it (measured 2026-11-17 day, the program's 0.11.3 and the
  recordings' converted traces, on the ticks both hold):** `original-dense-20000` against the
  program differs only in its two view-lag rows (17533 and 18975, builder 91756) and is equal from
  18976 to its end; `original-hard-dense-20000` differs only in the animation column, 17 rows from
  6500 to 20000, every one a one-tick view lag (13 a new clip a tick late, 4 an old clip kept over a
  one-tick stand, e.g. carrier 90298 at 7188: the pick-up shown one tick longer while the raw row
  already has no action), and is equal without that column; `original-1p` lacks the generator row of
  27673 and shows an empty view in 55 settler rows from 27742 (the raced end of its recording,
  TRACE-2.7), and is equal in settlers, buildings and generator to 30000 otherwise. So none of the
  three first differences (17533, 7054, 27673) is a difference of the games.
  A worker inside his
  building keeps his view: his work action there is recorded as `WorkLoop0` (the
  request of `GFX-animations.md` GFX-ANIM-3.8), though he is not drawn (E-trace-010:
  the sawmill's worker 90431 from trace tick 3169).
- **TRACE-4.5 Tick numbering (Q-314).** A trace's tick counts the world updates done:
  tick 0 is the world as set up, before any update; tick n is the state after the n-th
  update. The original's counter (TRACE-2.1) holds the number of the update that ran
  last, starting at 0, so a between-ticks snapshot whose counter reads c is trace tick
  c + 1, and the snapshot taken while the counter still reads -1 is trace tick 0. s2-reborn's
  trace already counts this way (its tick after a step; its first step is tick 1). The
  reader's own tables (TRACE-4.3) keep the game's counter in their `tick` column; the
  conversion to `docs/trace-format.md` (TRACE-4.4) and the recorder's
  per-tick files (TRACE-4.6) give the trace tick, and say so in a comment or column
  name. During update n the original's counter reads n - 1; a rule that uses the
  counter (for example "every k-th tick") uses that value. Confidence: high for the
  numbering (measured: counter -1 when the world appears, 0 after the first update);
  medium for the counter's value as seen by rules (no rule using it has been traced).
  The original's trace tick 0 is not a reliable picture of the set-up world, because
  the start objects are still being made while the counter reads -1 (TRACE-3.8);
  compare from trace tick 1.

  | Moment | original's counter | trace tick |
  |---|---|---|
  | world set up, loading, before the first update | -1 | 0 |
  | after the first update | 0 | 1 |
  | after update n | n - 1 | n |

- **TRACE-4.6 Unattended recording (live trace 2).** The investigation's recorder
  (three read-only processes) writes, from trace tick 0 of a
  fresh game: (a) every tick, the generator's end-of-tick state, its draw index (draws
  since the seed, SIM-RNG-2.1) and draws in the tick, with the clock (next tick time,
  game time, frame step, speed, time scale, fixed-step byte), as `generator.csv` with
  columns `trace_tick` and `tick` (the counter); (b) the TRACE-4.3 tables every tick up
  to trace tick 5000, then every 10th trace tick; (c) every 1000 trace ticks, the raw
  bytes of the world, its systems, every listed object and every vector one level below
  each system, for checking later layouts (Q-309) against the same game. The trace in
  `docs/trace-format.md` form is converted from (b). A tick it misses is logged; a
  scheduled sparse tick that could not be read is replaced by the next tick read, also
  logged. For comparison s2-reborn's trace has to hold the same ticks, or `trace-diff`
  has to compare only the ticks both traces hold: a tick only one trace has counts as
  all rows differing (`trace-diff --common-ticks` leaves such ticks out). The generator
  file is the per-tick generator record of Q-314 (b); `docs/trace-format.md` has it as
  `G` rows, which the converter writes for the original: at the ticks that
  have settler and building rows in the main trace, and for every tick in a trace of G
  rows only. Confidence: medium (procedure).

- **TRACE-4.7 Q-309 tables (proposal for the next recording).** The reader's Q-309 mode
  writes five more tables in TRACE-4.2's conventions (`tick` = the game's counter):

  | Table | Columns |
  |---|---|
  | deposits | tick, id, type, field_x, field_y, growth, lifetime, removal, reserved |
  | animals | tick, id, type, field_x, field_y (position), to_x, to_y (step end), goal_x, goal_y, home_x, home_y, progress, state, counter |
  | wares | tick, id, owner, good, lying, flag, carrier, dest, next_x, next_y, has_next, active, age, score |
  | stocks | tick, holder (building / player), id (building id or player index), stock (stock_a / stock_b: the two stocks of TRACE-3.7), good, amount |
  | ships | tick, id, owner, field_x, field_y, state, progress, build_a, build_b, cargo, target_x, target_y, counter, linked |

  One read of all five took 151 ms on a game of 3181 deposits and 286 animals, so a
  recording should take them less often than the settlers (for example every 100th
  tick, in a process of their own). Confidence: medium.

### TRACE-5 Which clock each moving thing follows (Q-226, Q-305; for the trace to confirm)

- **TRACE-5.1** Three clocks exist: the game step (frame time x time scale x speed:
  0 while paused, doubled and tripled at speeds 2 and 3), the frame time (TOOL-1.1:
  real, or 0.025 s under the fixed step; runs while paused, not scaled by speed), and
  real time (the second clock and the system millisecond counter, never under the
  fixed step). Confidence: high for the definitions.
- **TRACE-5.2** The scene's clock is always the game step and the game time. The
  scene can also run on its own real-time timer, but no caller asks for that in this
  build. The shaders' clock is the scene's, so flag cloth, tree sway, water waves and
  terrain wobble follow game time: they stop while the game is paused and run faster
  at speeds 2 and 3. GFX-OBJ-2.9 said real time; it rested on the unused real-time
  branch and is corrected (Q-313). Confidence: high (measured: while paused, game time
  and every clip clock stood still while frames ran; at speeds 2 and 3 the step was
  0.05 and 0.075 per frame; Jens saw flags, trees, water and animals stop while
  paused and move exactly 2 and 3 times as fast at speeds 2 and 3).
- **TRACE-5.3** Per item (Q-226), confidence medium unless the row says measured:

  | Item | Clock |
  |---|---|
  | settler, animal, ship and building clip clocks (GFX-ANIM-2.1, 2.5) | game step, per drawn frame (measured for settlers: frozen while paused, 0.025 per frame under the fixed step, 2x / 3x at speeds 2 / 3; animals seen to stop by Jens) |
  | the pose pool's 10 s sweep (GFX-ANIM-2.4 d) | game step |
  | the door's close time, now + 0.6 s (GFX-ANIM-6.1 b) | game time as of the last tick (it moves in whole ticks) |
  | the ship's bobbing phase (GFX-ANIM-7.2) | game time |
  | the random idle wait (GFX-ANIM-3.3) | counted down once per tick (stops while paused); the amount per tick is presumably the tick length |
  | the dying settler's sinking, 0.35 units/s (GFX-ANIM-3.7), and the evading offset's decay, 0.75 units/s (GFX-ANIM-4.3) | per tick |
  | the Sleep icon's 10 s idle timer (GFX-OBJ-8.3) | game step |
  | the refused command's 5 s red message (UI-GAME-1.4) | frame time: runs while paused, not faster at higher speeds; only the tutorial shows this message (all 67 callers pass tutorial texts, Q-317), so the test needs tutorial 1 |
  | message timers, tooltips, sliders, camera movement (TOOL-1.2) | frame time |
  | the animated speed icon (UI-GAME-2.7) | real time (second clock) |
  | fixed-function water drift and texture frames (low shader quality only) | real time (system counter) |

- **TRACE-5.4** A trace with the fixed step on therefore reproduces every visual
  except the speed icon and the low-quality water; with the game paused, only the
  frame-time items (messages, error text, camera, tooltips) and those two still move.
  Confidence: medium.

### TRACE-6 Order of the first checks in the tracing run

- **TRACE-6.1** Run the reader's probe in a free game first (T10); a wrong layout entry
  shows as absurd list counts. Then read the simulation generator at the main
  menu, after set-up and at tick 0 of a second game (SIM-RNG T6, Q-312). Then the tick
  boundary without and with the fixed step (T6, T7), one walking carrier (T9), one
  building site (TRACE-3.5), the pause test (Q-313, TRACE-5), and only then
  breakpoints (Q-308). Confidence: medium (procedure). Live trace 1 ran all of them
  but the breakpoints; the next run starts with the
  breakpoint tests and a trace from tick 0 of a fresh game with the fixed step on
  from the main menu, so that s2-reborn can be compared from the start. That trace is
  live trace 2 (E-trace-002, unattended, 150 000 ticks); the breakpoint tests remain.

### TRACE-7 Recordings with nobody at the PC (2026-10-23)

- **TRACE-7.1 What the launcher does.** The investigation's launcher makes a
  recording without anybody at the PC: it checks that the PC is free (TRACE-7.2),
  starts the original, waits for each menu screen by reading which screen the game
  shows, presses SINGLE PLAYER, START FREE GAME and START with synthetic mouse input,
  checks the map and the player slots from the game's memory before pressing START,
  sets the fixed step (TRACE-1, the one write) before the world exists, records
  (TRACE-7.4) and closes the game. Input goes only to the game's own window: before
  every input event it checks that the game window is in front and not minimized, that
  the target point lies in the game's client area and belongs to the game, and that
  nobody else has used mouse or keyboard since its own previous event. Any failed check,
  and any screen it does not expect, stops the run with the reason written beside the
  recording; a held button is released first. Confidence: high for the procedure;
  medium for the button positions of the start screen's and the set-up's START buttons
  (UI-SETUP-1.8), which the launcher logs with the button objects' own rectangles on
  its first run (E-tool-006). A slot's tribe and computer strength can be chosen
  (TRACE-7.6); choosing a map or changing a slot's kind is not automated yet: the
  launcher stops when the set-up does not already show the wanted map and the default
  slots.
- **TRACE-7.2 When it may start.** Only when nobody has used mouse or keyboard for 10
  minutes, and neither the implementation's game (`OpenS2X.exe`; `s2-reborn.exe` before
  release 0.9.3, and both names are checked), the play harness (`s2-viewer*.exe`, the
  `play-*.exe` test binary) nor the original is running. The launcher waits for these
  conditions: it checks them every 60 s for at most `--wait-hours` (default 6; 0 checks
  once), logs a waiting line every 10 minutes, and gives up with "not started" only at
  the end of the wait, having touched nothing. (Before 2026-10-24 it checked once: the
  E-trace-007 launch of 2026-10-23 23:45, typed two and a half minutes after the last
  input, could never start.) **The game's own start moves the system's last-input time**
  (2026-10-24: two launches, fullscreen and windowed, with the PC idle for 10 and 12
  minutes: input within a second of the game window appearing, and once 45 s after the
  game closed; the driver's first input guard, which asked for 10 idle minutes at that
  moment, stopped both runs). So the idle rule is applied to the moment of the launch,
  input between the launch and the moment the input clock has been quiet for 5 s after
  the first screen counts as the game's own, and from then on any input stops the run as
  before; input that keeps coming for 60 s stops it as "somebody is at the PC".
  **The world's start does the same** (E-trace-007, 2026-10-24 14:43: the last input 0.86
  s after the driver's own START click, then none for 6 min 49 s; the look step after the
  recording stopped on it): after the first tick the START step waits until the input
  clock has been quiet for 5 s (at most 60 s) and makes that the guard's baseline.
  Confidence: high. E-trace-008 (2026-10-25 00:36, with that fix: "the world's start
  did not move the input clock") recorded its 6000 ticks, but its look step stopped
  again before any input of its own: the last system input was 16.2 s after the START
  click, in the recording's first minute, and none came after. Whether that was a
  person or the system is not known (Q-344 stays open).
- **TRACE-7.3 The flag file (rule for the implementation).** While a launcher runs it
  keeps the flag file `RECORDING-IN-PROGRESS` in the recorder's folder of the user's
  Windows profile (one line: start time, launcher process, output folder; the scripts
  take another path from `S2_RECORDING_FLAG`) and removes it at the end, also after a stop. **While
  that file exists, the implementation must not start the play harness or
  its game (`OpenS2X.exe`, before release 0.9.3 `s2-reborn.exe`) on that PC**: both
  move the desktop pointer or take the foreground,
  which stops or spoils the recording. `docs/releasing.md` should state this next to
  the harness instructions, and the harness scripts (`scripts/play-test.sh`,
  `golden-images.sh`, `view-map.sh`, `golden.sh`) should refuse to start while the file
  exists. A flag older than a day with no launcher running is stale and may be removed
  by hand. Confidence: high.
- **TRACE-7.4 What a recording holds by default (record everything once).** Every table
  the recorder knows, every tick from tick -1 to the stop: the TRACE-4.3 tables
  (settlers, buildings, flags, streets, players, animals, the computer player's street
  state), the object tables (counter, objects, good flags) and the height samples; in
  addition wares, stocks and ships every tick and the deposits every 100th tick
  (`--slow-every`); the generator and clock every tick; raw dumps of the known objects
  every 1000 ticks; and a dump of the game's whole heap (all committed private memory
  and the writable pages of its modules) every 500 trace ticks and at the stop
  (`--heap-every`), so that later questions about fields nobody has mapped yet can be
  answered from the archive. A heap dump is not atomic: the game runs on while it is
  read, and the dump records the counter before and after. The doodad maps (19 000
  entries on Storm Coast) are re-read only in the 100th-tick snapshots (they cost the 57
  gaps of E-trace-006); between re-reads the counter table still shows their live counts,
  and the object table shows their appearances and removals at the next re-read.
  Confidence: high (tool behaviour); the heap dump's size is measured on the first run.
  **Day clock and light (2026-10-24, Q-350).** With every generator row the recorder also
  writes `env.csv`: the environment object's day clock (hours, not wrapped) and its rate,
  the DayNightCycle byte and the start hour, the position between the two day entries,
  the interpolated light elevation, heading and direction, background, ambient and light
  colours, shadow intensity, fog and the two skydome numbers. Whenever the day table or its entry count changes it saves the
  table in the `.bin` layout of GFX-VIEW-2.3b (`env-table-<trace tick>.bin`, and parsed
  as `.json`), so the recording shows which file's table the game uses. The clock moves
  with frames, not ticks; the rows sample it once per tick. Confidence: high (tool);
  medium for the fields until the first run (E-sim-007).
- **TRACE-7.5 File-open trace (2026-10-24, Q-348).** Which files the original reads is
  recorded without touching the game: NTFS on the recording PC updates a file's
  last-access time when a program reads it (Windows reports "System Managed, Last Access
  Time Updates ENABLED" for C:). The documented delay (no update within an hour of the
  last one) does not hold here: on 2026-10-24 the game's second start read 368 files
  12.7 minutes after the first, and every one of their times moved. The file trace
  lists the install folder and the game's cache folder `%LOCALAPPDATA%\S2` (converted
  models `.msh`, compiled shaders, map infos) with their times before the launch
  (`files-before.tsv`), scans them every 5 s while the game runs and writes every change
  with the wall time and the game's trace tick (`files.csv`: read, write, new, gone), and
  lists them again after the game has quit (`files-after.tsv`); the launcher starts
  it just before the launch and ends it after the quit. Limits: a file read again
  shortly after a read may not show it (13 minutes is enough, the shortest gap is not
  measured; the baseline marks files read in the last hour `recent` to be careful); a
  file read twice within one scan shows once; the scan period (5 s) is the time
  resolution; asking for a file's size or times is not a read. Other programs also move
  last-access times: on 2026-10-23 at 19:33 a scan that was not the game read the
  executables, libraries, manuals and icon of the install (and nothing in `data`), so
  reads of `.exe`, `.dll`, `.pdf` and `.ico` files outside a game's start are not the
  game's. The cache folder records first use for good: the game writes a mesh's `.msh`
  the first time it loads that mesh. For the 245 meshes cached under their model's own
  name (settlers, wares, animals), the model file's last-access day equals the `.msh`'s
  creation day: later games read the `.msh`, not the model file, so that model's
  last-access time shows its first load only. Building models are read again in every
  game although their meshes are cached (`_house`, `_ground`, `lod0_house`). Confidence:
  high (tool and the NTFS rule, tested 2026-10-24: a read moved a five-day-old
  last-access time to the read's time, and the game's two starts that day 12.7 minutes
  apart both showed; cache rule from the listing of 2026-10-24).
- **TRACE-7.6 Choosing the set-up (2026-10-27, plan step 7.8h).** The launcher can be
  asked for a slot's settings on the free game page (per slot: its kind, and optionally
  tribe and strength; repeatable). It reads the set-up record (per slot: start position,
  kind, strength, tribe, team and colour) and for each request: the slot's kind must already be KIND (`any` skips
  the test); the tribe (romans 0, africans 1, chinese 2) and the strength (weak 0, normal 1,
  strong or hard 2: the computer profiles 1, 2, 3 of `ai.lua`, UI-SETUP-2.5) are set by
  clicking the slot's portrait (40 x 80 at P + (-47, -25)) and its severity emblem (48 x 48
  at P), with P of `UI-game-setup.md` UI-SETUP-1.8 on the menu panel, which is drawn
  unscaled and centred (measured on the 2560 x 1440 screen: the set-up's START button object
  at panel (445, 650) = screen fraction (0.4738, 0.6847), SINGLE PLAYER at (700, 250) =
  (0.5734, 0.4069)). After every click the record is read again: exactly that field of that
  slot must have changed; when nothing or anything else changed, or four clicks did not
  reach the value, the run stops (before START, nothing recorded). The record as set is kept
  in `drive-state.json`, logged in `drive.log` and checked again before START. Not automated
  yet: choosing a map (the run stops when the page does not show `--map`), a slot's kind
  (so the players are the map's default: a human and a computer in every further slot the
  map has), and starting from a saved game, a campaign mission or a tutorial (it starts free games
  only). Confidence: high for the strength (first live run, E-trace-012,
  2026-10-27: the emblem of slot 1 at screen point (1247, 765) of the 2560 x 1440 screen,
  two clicks, strength 0 -> 1 -> 2, nothing else changed); medium for the tribe (the
  portrait's click point and that it cycles the tribe are read from the page's layout code,
  E-ui-007, not yet clicked; the read-back makes a wrong reading stop, not record
  a wrong game).
- **TRACE-7.7 Input by window messages (2026-10-29).** The launcher's clicks and keys are
  posted to the game's window as window messages (button down / up, key down / up), and the
  pointer is moved with the system's set-cursor call; nothing goes into the system's input
  stream. Reason: on the recording PC a keyboard and mouse sharing program (Synergy) forwards
  the input stream to another computer while its active screen is there, so synthetic input
  of the stream reached that computer and not the game (two arms of E-trace-015 at the start
  films; Synergy's log shows the switch). The game reads its buttons and keys from these
  messages (UI-ingame.md UI-GAME-1.10c: the input layer counts the messages per frame) and
  its menus hover and hit-test by the real pointer, so both are needed. Posted messages do
  not move the system's last-input clock, so the guard (TRACE-7.1) takes one movement of that
  clock followed by 3 s without another as the game's own (it moves the clock at its start,
  at the world's start and about 1 s after the START FREE GAME click), at most 12 per run;
  more input than that stops the run as a person at the PC. Confidence: high (measured
  2026-10-29: films, START, SINGLE PLAYER, START FREE GAME, the strength clicks and the
  recording E-trace-015 by posted messages).

### TRACE-8 Pictures of the original for the look comparison (2026-10-29, plan step 7.6a)

- **TRACE-8.1 What a shot is.** The investigation's driver, given trace ticks T1, T2, ..,
  a folder, a name NAME, a label L and camera keys (also beside the recorders, from the
  launcher), waits for each trace tick T, pauses the game with its
  own pause key P, waits 0.5 s of paused frames, reads the camera, the light and the window,
  takes the picture of the game window's client area as it is on the screen (PNG, 8-bit RGB,
  the window's own size), writes `<NAME>-<tick>-<L>.png` and `.json`, and resumes with P.
  The only inputs are P (twice per shot) and the camera keys once; everything else is read.
  Its grab mode writes one picture and camera file of the moment without any input.
- **TRACE-8.2 Timing.** The pause key acts on its release (the game's pause handler reads the
  key's release event, UI-GAME-1.10c's input layer): P is posted as soon as trace tick T has
  been reached (counter T - 1) and the game counts as paused when the options' speed reads 0; the tick in the camera file is the counter at that moment + 1, so
  a shot can come a tick late (logged with the asked tick). While paused the simulation and
  every clip clock stand still (TRACE-5.4); the counter is checked again before the picture.
  Pausing does not change the simulation: the recorders see a pause as ticks that come later.
- **TRACE-8.3 The camera file.** One JSON object: `tick` (trace tick of the picture),
  `requested_tick`, `counter`, `paused`, `map`, `game` (NAME), `window` [w, h] (client size =
  the picture's size), `client` [x, y, w, h], `eye` (camera position), `dir` (the
  smoothed view direction that the view matrix uses), `target` (the look-at point of
  GFX-VIEW-3.3), `zoom`, `yaw`, `yaw_target`, `fov_h_deg` (in degrees), `near`, `far`
  (doubled by a game flag that is never set, GFX-VIEW-3.1), `direction_unsmoothed`,
  `base_direction`, `ground_smoothed`, `distance_base`, `mode`, `camera_keys`, and `light`: the
  environment's day clock, rate, blend, elevation, heading, light direction, background,
  ambient and light colours, shadow and fog (the env.csv fields, TRACE-4.3), with the day
  cycle option and the start hour. The view and projection matrices are not read: the view
  is look-at from `eye` along `dir` with up +y, the projection the horizontal 50 degrees of
  GFX-VIEW-3.5 at `window`'s aspect. Camera fields: E-eco-037.
- **TRACE-8.4 Camera keys.** The game reads the arrow, page-up / page-down and rotate keys
  from the live key state once a frame (held frames), not from key messages, so posted keys
  cannot move the camera (UI-ingame.md UI-GAME-1.10c's input layer); H and Home act on their
  key messages. `--camera-keys` is therefore a comma list of `h` / `home` (jump to the
  headquarters, zoom 3, yaw 0), `wheelin` / `wheelout` (one zoom step each, a posted wheel
  message), and `up` / `down` / `left` / `right` with `*ms`: the real pointer held at that
  edge of the window for ms (edge scrolling, GFX-VIEW-3.2: 2 x step along the view's
  horizontal direction or sideways per frame, about 1.95 units a frame under the fixed step
  at the default camera speed). Rotation is not available without the real key state. The
  keys are sent once before the first shot, then 1.5 s for the easing, and the camera before
  and after is logged: the scene is what the camera file says, not what the keys were meant
  to do. Planned for F48 (`GFX-water.md` GFX-WATER-3.6): the Weak game from the default view
  of player 0's headquarters (field (29, 51), camera target (-333, 11.5, -194.7) at zoom 3)
  to Storm Coast cell (6, 3) (its centre field (26, 14) at world (-354, -368.9)):
  `down*1640,left*180`.
  **Added 2026-10-30:** `to:X:Z` scrolls by the edge until the camera target is
  within 3 units of world (X, Z), reading the target back after every slice (at
  yaw 0: right +x, left -x, up +z, down -z); `zoom:N` posts wheel steps until the
  camera's zoom reads N. A field's world point: x = 6 fx + 3 (fy odd) - 510,
  z = 5.1961524 fy - 441.6529 (GFX-VIEW-1.1 on the 170 x 170 field map). While
  the game is paused the edge scrolling and the wheel act, **H does not** (measured
  2026-10-30 01:23: the H of a second scene left the camera where the first had
  put it).
- **TRACE-8.4a Scenes** (2026-10-30). `--scene LABEL=KEYS` (repeatable) adds, at
  every shot tick, after the picture of the unchanged view, one picture per scene:
  its keys are sent while the game stands paused, 1.5 s for the easing, the
  counter is checked unchanged, and the picture is `<NAME>-<tick>-<LABEL>`; then
  the view goes back to where it was by `to:` and `zoom:` (within 3 units). A scene
  whose camera move does not arrive is skipped and logged; the other pictures go
  on. The camera file of a scene has `scene` and `camera_keys`.
- **TRACE-8.4b Fog of war** (2026-10-30). The fog of war option of the driver's menu
  step and the launcher (on or off) sets the free game page's "Fog of war" drop-down (UI-SETUP-1.8,
  n 2: the bar at panel (433, 214), 224 x 19; the open list has one 19 px row per
  entry below the bar, On centred at panel y 242.5, Off at 261.5, measured from
  pictures). Off turns the session's option bytes from [0, 1, 0] to [0, 1, 1]: the
  third byte is the fog of war (0 On, 1 Off); the first is the
  victory condition (0 Endless), the second the start resources (1 Medium). Every
  camera file has `setup_options` and `fog_of_war`.
- **TRACE-8.5 The picture.** Copied from the screen (the desktop's device context) at the
  client rectangle, also for a fullscreen game; needs the game in front (TRACE-7.1's guard).
  A picture with no non-black pixel in a sparse sample is marked in its camera file.
  Every picture of a paused game shows the game's small "Paused" label at the
  screen centre (measured 2026-10-30).
  Confidence: high for the capture (2026-10-29 23:33, `grab` during E-trace-015 in fullscreen
  2560 x 1440: the game screen with its frame and bar; camera zoom 3, dir (0, -0.8, 0.6),
  fov 50, near 1, far 500); medium for the pause and the camera keys, not yet run live (the
  key handlers read the posted key messages, UI-GAME-1.10c).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| single-instance mutex name | `settlers2` | | high | E-tool-002 |
| wait loop sleep | 50 | ms | medium | E-tool-002 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | game at the main menu, a second copy started | | the second copy exits at once, no window, no message |
| T2 | game in play, tracer reads memory only | read the fixed-step byte and the tick counter every 25 ms for 60 s | game behaves as without the tracer; no crash report (passed, live trace 1: about 46 min of reads, more than 15 million) |
| T3 | debugger attached, fixed step on | hardware breakpoint on the tick driver, stop 10 s, continue | the next frames advance by 0.025 s each; no catch-up ticks |
| T4 | debugger attached, fixed step off | stop 10 s, continue | one large frame; the tick driver's catch-up rule applies (ECO-TIME-1.2) |
| T5 | debugger attached | software breakpoint on a function of the game's own code, continue after the hit | game continues normally |
| T6 | new free game, fixed step on, speed 1 | hardware breakpoint at the end of a tick's simulation (TRACE-2.3), run 40 frames | 10 hits with a new tick number, 0, 1, ... 9 for a fresh map; between hits next tick time grows by 0.1 |
| T7 | any game, no breakpoint | read tick, next tick time, game time in a tight loop | next tick time > game time in almost every read; the busy reads come in runs no longer than one frame (passed: 0.58 % busy at real time, 0.20 % with the fixed step; median run 0.14 ms) |
| T8 | start of a free game | read the six settler lists and the building list at tick 0 | the headquarters is in the building list, owner 0, with levelled >= 1 and both remainders <= 0; every settler in the lists has a valid field and owner (expected: few or none at tick 0, the start population being a count in the headquarters' stock, to be confirmed) |
| T10 | game loaded | reader `--probe` | world pointer non-null, app state 4, tick length 0.1, list counts match what is on the map (buildings and sites, flags, roads); first thing to run (passed: MP_2P_Storm_Coast, castles on the map's two start fields) |
| T9 | a settler walking on a road | read its movement each tick | field changes exactly when progress passes 1; world position of its view = blend of the two fields by progress (first half passed: carrier 90399, live trace 1; the view position was not read) |
| T11 | game in play, tracer following | minimize the game for a while, restore it | while minimized the tick number is raised, next tick time <= game time, no random draw of that tick yet; after restoring the tick finishes and the next frame's step is one tick length (0.1); no tick is lost or added |
| T12 | fixed step on, speed 1, 60 frames per second | follow 300 ticks | every frame's game time step 0.025; 4 frames per tick except where the float32 sums drift (about one tick in 300 takes 5); 300 ticks in about 20 s of real time |
| T13 | PC used less than 10 minutes ago, or the play harness running | start the launcher | it exits 2 without starting the game or touching the pointer, and writes no flag file |
| T14 | launcher running, game at a menu | move the mouse | the launcher stops before its next input event with "someone else used mouse or keyboard", releases any held button, closes the game and removes the flag file |
| T15 | file trace running, no game | read one file of the install whose last-access time is more than an hour old | `files.csv` gets one `read` row for it within one scan; a file read again within the hour gets no second row |
| T16 | TRACE-4.4 (the view lag): `original-dense-20000` against the program's default game, `original-hard-dense-20000` against its Hard game | the first differences | 17533: carrier 90288, the original's row Walk, the program's Stand, every other column equal (the recording's raw row: progress -1, arrived, at counter 17532); 7054: carrier 90623, the original's Stand, the program's PutDown (raw: anim_code 2 set at counter 7053); both are counted as view lag and the comparison goes on past them |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was
found, and the recordings' files, are kept in the investigation's private records under
the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-011 | unattended recording | the two-player game an eighth time, to trace tick 20 000: every 10th tick, every tick from 5500; all tables of E-trace-010; 51 ticks missed in 5500 .. 20 000; equal to the earlier recordings on every common tick; published here as `specs/traces/original-dense-20000*`; the first attempt stopped in the menu on a single input of unknown origin | TRACE-4.6 |
| E-trace-016, E-trace-017 | unattended recordings with pictures | tables every 10th tick to 11 100, generator every tick, a log of every shot, scene and camera move, and the fog of war drop-down measurement: the first TRACE-8 runs, the Weak game with fog of war On (shots `weak-*`) and Off (`weakfogoff-*`, scenes farm and cell63), both equal to E-trace-014 at every counter; the first fog-off run was stopped at 1500 after the H finding of TRACE-8.4 | TRACE-8, 8.4a, 8.4b |
| E-trace-012 | unattended recording | the two-player game with player 1 at strength 2 (profile 3 "Strong" in the memory reads), set by the launcher's first slot run (TRACE-7.6: two clicks on the severity emblem, 0 -> 1 -> 2, checked in the set-up record); to trace tick 20 000 as E-trace-011, all its tables; recorded 2026-10-27 08:44 .. 09:07 with nobody at the PC; published here as `specs/traces/original-hard-dense-20000*` | TRACE-4.6, TRACE-7.6 |
| E-trace-005 | unattended recording | the two-player game of E-trace-002 recorded again every tick to trace tick 6000 with the animals table in every snapshot and the height samples (whole layer at ticks 0, 1, 6000; the fields within 3 of every site at its appearance and after every levelling step); published here as `specs/traces/original-dense-6000.s2trace.gz` and `-generator` | TRACE-3.6 (the height layer), 4.3 (animals, heights) |
| E-trace-002 | unattended recording | one free game on MP_2P_Storm_Coast from the first tick to trace tick 150 000 with the fixed step on: generator and clock every tick, the TRACE-4.3 tables every tick to 5000 then every 10th, raw dumps every 1000, the trace in `docs/trace-format.md` form | TRACE-3.7, 3.8, 4.5, 4.6 |
| E-trace-001 | live read of the running game | generator watch over two games, clock watches at real time and fixed step, snapshot checks, 300-tick follow tables and their `.s2trace` conversion, pause and speed log | TRACE-2.1, 2.4 to 2.7, 3, 4, 5.2, 5.3 measured |
| E-tool-005 | code reading | the scene clock and its one caller, the shader timer, the per-tick view hook, the pose pools, the door, the ship, the random idle, the status icons, the error box, the second-clock readers | TRACE-5 |
| reader | the investigation's tool | its self-test checks every column against a synthetic memory image | TRACE-4 |
| E-tool-003 | code reading | the tick driver, the world, the settler, building, flag and street lists and objects, movement, references, players, views | the objects, tick boundary |
| E-tool-002 | code reading | start-up, the crash guard, the wait loop, the copy protector's leftovers | start-up and the reaction to a tracer |

## Open questions

- Q-306: answered from the reading (medium); the run confirms with TRACE T2 to T5
  (Q-308).
- Q-308: confirm TRACE-1.1 to 1.8 in the run: read-only tracer, one data write, one
  hardware and one software breakpoint, a 10 s stop with and without the fixed step.
  The read-only tracer and the data write passed in live trace 1; the breakpoints and
  the stops remain.
- Q-309: wares, trees, stones, animals, goods in buildings and ships mapped (TRACE-3.7,
  medium); still open: the per-class job sub-states and the visual kind objects.
- Q-313: answered by live trace 1 (measured and seen): game time; GFX-OBJ-2.9 is
  corrected, `docs/clocks.md` should follow.
- Q-310: answered: `docs/trace-format.md` exists and the original's trace is written in
  it (TRACE-4.4).
- Q-314: answered: tick numbering TRACE-4.5, generator rows in the format (TRACE-4.6),
  the original's trace from trace tick 0 in E-trace-002.
- Q-315: where in the tick a minimized game stops (TRACE-2.7).
- Q-316: what the world's time scale is for, since the pause does not use it.
- Q-317: the red error message seems to exist only in the tutorial.
- Q-318: what settler states 5 and 6 mean (read as dying and dead; seen once for a
  soldier ending at a depot).
