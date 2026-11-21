# Trace format

A trace records, tick by tick, what every settler and every building does, so
that a run of s2-reborn and a run of the original from the same start can be
compared line by line. s2-reborn writes it with `s2-app trace`; the
investigation side produces the original's trace in the same format (from the
tables of its reader, `specs/TOOL-trace.md` TRACE-4; how each column follows
from them is below); `s2-app trace-diff A B` names the first tick and row
where two traces differ.

This format answers Q-310: it keeps TRACE-4.2's conventions (x then y, type
ids as 8 lower-case hex digits) in one table of settler and building rows, the
columns that both programs can fill and compare.

## File

Plain text, UTF-8, lines ending in `\n` (a `\r` before it is ignored).

- Lines starting with `#` are comments. The first line is
  `# s2-trace 1` (the format version), followed by comments naming the
  source, e.g. `# s2-reborn MP_2P_Storm_Coast seed 1` or `# original ...`.
- The first other line is the column header, exactly:

```
kind,tick,id,type,x,y,fine_x,fine_y,animation,clip_time,job,state,progress
```

- Every other line is one row, comma separated, no quoting (no field contains
  a comma). A field that does not apply to the row's kind is empty.
- Rows come in tick order. Within a tick the settler rows come first, then the
  building rows, then the generator row; the order within each group does not
  matter (`trace-diff` matches rows by key, below).

## Settler rows (`kind` = `S`)

One per settler per tick, after the tick's logic has run. "Settler" includes
road carriers, donkeys and boat carriers, the builder and the leveller of a
construction site, soldiers and specialists; animals and ships are not
settlers.

| Column | Meaning |
|---|---|
| `tick` | The trace tick (TOOL-trace TRACE-4.5, Q-314): 0 is the world as set up, n the world after the n-th update (`World::tick` after the step; the first step is tick 1). The original's counter reads n - 1 then. |
| `id` | The settler's object number (SIM-START-4.2, the low 32 bits, decimal), stable for its life: a builder or leveller keeps his number on the site, a road carrier going home his. The original's trace uses its own object numbers the same way, so the ids of the two programs agree where their games number alike; see matching below. |
| `type` | The profession: the good's id in 8 lower-case hex digits (`2c50533e`, `good_woodcutter`). The ids are the property scripts' own, the same in both programs (`s2-app rules names` lists them by name). |
| `x`, `y` | The field the settler stands on or last reached (MAP-GRID-1). |
| `fine_x`, `fine_y` | The exact position in thousandths of a height sample (MAP-GRID-4: 4 samples per field, odd rows shifted by 2): the sample reached, moved toward the next sample by the walk's progress (TRN-MOVE-1.1, millionths of a step), as integers. A settler on a field without a fine walk is at the field's sample (MAP-GRID-4.2). |
| `animation` | The base animation the logic asks for in this tick (GFX-ANIM-3.1): `Stand`, `Walk`, `PutDown`, `Pickup`, `WorkLoop0`, `WorkLoop1`, `WorkLoop2`, `Hit`, `SwimIdle`, `SwimFwd`, `Dying`. A work loop asked for while the settler is inside is `Stand` (3.1). |
| `clip_time` | Ticks the settler has had this `animation` without interruption, 0 in the tick it starts. Not the pose clock of a shared pose (GFX-ANIM-2): compare it only between traces of s2-reborn, or leave it out (`--ignore clip_time`). |
| `job` | The job and its step, `Job.Step` (s2-reborn: `Worker.AtSpot`, `Carrier.PickUp`, `Builder.Working`, `Home`, `Garrison`). The original's names differ; this column is for reading, not compared by default. |
| `state`, `progress` | Empty. |

## Building rows (`kind` = `B`)

One per building or construction site per tick.

| Column | Meaning |
|---|---|
| `tick` | As above. |
| `type` | The building type's id in 8 lower-case hex digits (`f6e26cb3`, the castle). |
| `x`, `y` | The building's own field (ECO-PLACE). |
| `state` | `levelling` (a site whose ground is being levelled, ECO-HQ-4.2), `frame` (a site in its frame part, ECO-HQ-4.7), `main` (a site in its main part), `finished`. |
| `progress` | Thousandths done of the current part, rounded down: of the levelling, of the frame part, of the main part (ECO-HQ-4.7); 1000 for `finished`. A part's share done is 1 - its remaining work, the float32 the decrements leave (ECO-HQ-4.6, 4.6a), as the original's rows are converted (below). |
| other columns | Empty. |

## Generator rows (`kind` = `G`)

One per tick: the simulation's random number generator at the end of the
tick (SIM-RNG; TOOL-trace TRACE-4.6 records the same in the original's
`generator.csv`).

| Column | Meaning |
|---|---|
| `tick` | As above. |
| `id` | `rng`. |
| `state` | The generator's state (SIM-RNG-1.1), decimal. |
| `progress` | The draw index: draws since the seed (SIM-RNG-2.1), so the draws in a tick are the difference to the tick before. |
| other columns | Empty. |

From the original's `generator.csv`: `tick` = `trace_tick`, `state` = the
end-of-tick state, `progress` = the draw index.

## Matching and comparing (`s2-app trace-diff A B`)

Within each tick, rows of the two traces are matched by key:

- building rows by (`x`, `y`);
- the generator row by its kind (one per tick);
- settler rows by `id` (both traces from s2-reborn), or with `--by-place` by
  (`type`, `x`, `y`, `fine_x`, `fine_y`) and, for equal keys (settlers on one
  place), in the order of their compared columns, so the order a trace lists
  them in does not matter (traces of different programs).

Two matched rows differ when any compared column differs. Compared by default:
`type`, `x`, `y`, `fine_x`, `fine_y`, `animation`, `clip_time` for settlers;
`type`, `state`, `progress` for buildings; `state`, `progress` for the
generator (its tally is `generator`). `--ignore col,col` leaves columns
out; `--with job` adds `job`. A row with no partner differs as a whole.

`--view-lag` (TOOL-trace TRACE-4.4, T16) is for a recording of the original as
the second trace: its view switches a clip one tick late now and then, so a
settler row that differs only in `animation`, the second trace's being the
first trace's of the same settler in the tick before, is counted as that lag
(the count is printed), not as a difference; a row whose view the recorder did
not read (an empty `animation`, TRACE-2.7) has no animation to compare. With
`--common-ticks` a tick both traces hold, one of them without its generator
row (a recording's raced end), leaves the generator out there.
`s2-app fidelity` compares every pair so.

`trace-diff` prints the first tick and row where the traces differ (both rows,
and the columns that differ), then a summary per settler type and for the
buildings: rows compared, rows that differ, the first tick of a difference and
the column that differs most often. Exit code 0 when the traces are the same,
1 when they differ, 2 for a malformed file.

## From the original's tables (TRACE-4.3)

| Column | From |
|---|---|
| `tick` | the trace tick: the snapshot's counter + 1 (TRACE-4.5; `to_s2trace.py` converts it) |
| settler `id` | `id` (decimal); compare with `--by-place` |
| settler `type` | `job` |
| `x`, `y` | `field_x`, `field_y` |
| `fine_x`, `fine_y` | standing (`progress` = -1): the fine point (`fine_x`, `fine_y`) x 1000; walking: `from` + (`to` - `from`) x `progress`, x 1000, rounded toward zero, where `from` / `to` are fine points in fine mode and fields' samples (MAP-GRID-4.2) in rough mode |
| `animation` | `view_base` by name (0 Stand, 1 Walk, 2 SwimIdle, 3 SwimFwd, 4..6 WorkLoop0..2, 7 Pickup, 8 PutDown, 9 Hit); `Dying` for `state` 5 |
| `clip_time` | (not comparable, see above) |
| `job` | `list`.`state` |
| building `type` | `type` |
| building `x`, `y` | `field_x`, `field_y` |
| `state` | `levelled` < 1: `levelling`; else `frame_left` > 0: `frame`; else `main_left` > 0: `main`; else `finished` |
| `progress` | `levelled`, 1 - `frame_left`, 1 - `main_left` by state, x 1000, rounded down; 1000 when finished |

## s2-reborn's side

`s2-app trace MAP TICKS [--seed S] [--ai ...] [--from T] [--kinds SBG]
[--rows-every N] [--out FILE]` plays a game of computer players (as `hash` and `soak` do;
`--ai 1=1` makes only player 1 one, player 0 giving no command) and writes
the rows of ticks `T` to `TICKS` (default from 1; `--from 0` adds tick 0,
the world as set up, TOOL-trace TRACE-4.5). The seed is the game's,
0x5432ACE0 (SIM-RNG-2.1), unless `--seed` says otherwise. `--kinds G`
writes only the generator rows (any letters of `S`, `B`, `G`);
`--rows-every N` writes the settler and building rows of every Nth trace
tick only and the generator's of every tick, as the original's long
recordings hold them (`clip_time` then counts from the rows written); an `--out`
file ending in `.gz` is written gzip compressed, without a time stamp.
`s2-app trace --snapshot FILE TICKS` starts from a game snapshot
(`docs/snapshot.md`) instead, writing the ticks after the snapshot's up to
`TICKS`; its `clip_time` counts from the snapshot's tick.

`trace-diff` reads gzip files as they are (`specs/traces/*.gz`). `--from T`
leaves out the ticks before `T`. `s2-app fidelity` makes our traces
(`tests/traces/README.md`) and runs it on them against the original's and compares the result with the record in
`docs/fidelity.md`. `s2-app rng MAP TICKS` prints the draws per
tick with the places that drew (`docs/rng-diff.md`). The rows come from
`s2_sim::trace` (`World::trace_settlers`, `World::trace_buildings`); the
animation is the simulation's own answer to GFX-ANIM-3.1, the same one the
view plays (`s2_sim::trace::worker_request`). Not yet produced by s2-reborn:
`Hit` (fights, Q-087) and `SwimIdle` / `SwimFwd` (the view decides them
from the 3D model's height).
