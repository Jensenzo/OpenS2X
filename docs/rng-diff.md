# The simulation's random draws: ours against the original's

SIM-RNG-2.5 measured, in the original, how many times the simulation's
generator draws in each tick of a game on MP_2P_Storm_Coast with the
default set-up (OBS-30: player 0 a human who gives no command, player 1 the
computer, start resources Medium). Matching it is a check of the whole
simulation: every system has to draw as often, in the same ticks. This page
compares our count with it. Nothing was tuned to make the counts match.

To reproduce (`s2-app rng` prints, per trace tick, the draw index, the draws
in the tick and the places in `crates/sim` that drew):

```bash
cargo run --release -p s2-app -- rng MP_2P_Storm_Coast 700 --ai 1=1
```

The trace's generator rows (`G`, `docs/trace-format.md`) hold the same draw
index and the generator's state, for `trace-diff` against the original's
recording (TOOL-trace TRACE-4.6).

## The generator itself

Ours is the original's (SIM-RNG-1, 2.1 to 2.2): Park-Miller, seed 0x5432ACE0
with the seed repair, reset at the end of every game set-up
(`crates/sim/src/rng.rs`; tests T1 to T9 and the reset, `rng::tests`). The
first draw of every new game is 1257492815. Ticks are numbered as TOOL-trace
TRACE-4.5: trace tick = the original's counter + 1.

## Result (2026-10-19)

**First tick that differs: trace tick 3 (the original's counter 2).** The
original draws 3 times in it, we draw 4 times: `hunting.rs:344` twice (two
animals choose a target, ECO-FOOD-5.10: "an animal's target" in SIM-RNG-2.4)
and `hunting.rs:315` twice (those two animals start their walk; its free path
draws its starting direction, TRN-FREE-3.2, which SIM-RNG-2.4 lists for
settlers' movement only: Q-232).

| | the original (SIM-RNG-2.5) | ours |
|---|---|---|
| draws in counter ticks 1 to 200 (trace ticks 2 to 201) | 1005 | 999 |
| ticks without a draw among them | 52 | 100 |
| most draws in one tick among them | 20 | 23 |
| draws in counter ticks 0 to 699 | 3043 | 2717 |
| places that drew in the first 700 ticks | not known | 2: `hunting.rs:344` (940 draws), `hunting.rs:315` (1777) |

The totals are close but the ticks are not: from trace tick 4 on we draw
about 10 times a tick while the original's animals start slowly (1, 1, 7, 6,
...), and later we have twice as many ticks without a draw. In the first 700
ticks nothing but the animals draws in our game: no settler takes a free
walk, no fight, no forester or fisher works yet.

Per tick, ticks 0 to 31 (draw index = draws since the seed at the end of the
tick):

| trace tick | original's counter | original: index | original: in tick | ours: index | ours: in tick | ours: places (crates/sim/src) |
|---|---|---|---|---|---|---|
| 0 | -1 | 0 | 0 | 0 | 0 |  |
| 1 | 0 | 0 | 0 | 0 | 0 |  |
| 2 | 1 | 0 | 0 | 0 | 0 |  |
| 3 | 2 | 3 | 3 | 4 | 4 | hunting.rs:315 x2 hunting.rs:344 x2 |
| 4 | 3 | 4 | 1 | 14 | 10 | hunting.rs:315 x6 hunting.rs:344 x4 |
| 5 | 4 | 5 | 1 | 27 | 13 | hunting.rs:315 x9 hunting.rs:344 x4 |
| 6 | 5 | 12 | 7 | 36 | 9 | hunting.rs:315 x5 hunting.rs:344 x4 |
| 7 | 6 | 18 | 6 | 49 | 13 | hunting.rs:315 x9 hunting.rs:344 x4 |
| 8 | 7 | 26 | 8 | 61 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |
| 9 | 8 | 33 | 7 | 68 | 7 | hunting.rs:315 x4 hunting.rs:344 x3 |
| 10 | 9 | 37 | 4 | 78 | 10 | hunting.rs:315 x7 hunting.rs:344 x3 |
| 11 | 10 | 42 | 5 | 90 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |
| 12 | 11 | 50 | 8 | 108 | 18 | hunting.rs:315 x14 hunting.rs:344 x4 |
| 13 | 12 | 58 | 8 | 117 | 9 | hunting.rs:315 x5 hunting.rs:344 x4 |
| 14 | 13 | 61 | 3 | 130 | 13 | hunting.rs:315 x9 hunting.rs:344 x4 |
| 15 | 14 | 70 | 9 | 144 | 14 | hunting.rs:315 x10 hunting.rs:344 x4 |
| 16 | 15 | 77 | 7 | 156 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |
| 17 | 16 | 89 | 12 | 168 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |
| 18 | 17 | 106 | 17 | 180 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |
| 19 | 18 | 114 | 8 | 187 | 7 | hunting.rs:315 x3 hunting.rs:344 x4 |
| 20 | 19 | 125 | 11 | 202 | 15 | hunting.rs:315 x11 hunting.rs:344 x4 |
| 21 | 20 | 125 | 0 | 213 | 11 | hunting.rs:315 x7 hunting.rs:344 x4 |
| 22 | 21 | 136 | 11 | 222 | 9 | hunting.rs:315 x5 hunting.rs:344 x4 |
| 23 | 22 | 155 | 19 | 231 | 9 | hunting.rs:315 x5 hunting.rs:344 x4 |
| 24 | 23 | 161 | 6 | 235 | 4 | hunting.rs:344 x4 |
| 25 | 24 | 161 | 0 | 250 | 15 | hunting.rs:315 x11 hunting.rs:344 x4 |
| 26 | 25 | 165 | 4 | 262 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |
| 27 | 26 | 177 | 12 | 271 | 9 | hunting.rs:315 x5 hunting.rs:344 x4 |
| 28 | 27 | 183 | 6 | 285 | 14 | hunting.rs:315 x10 hunting.rs:344 x4 |
| 29 | 28 | 202 | 19 | 294 | 9 | hunting.rs:315 x6 hunting.rs:344 x3 |
| 30 | 29 | 202 | 0 | 306 | 12 | hunting.rs:315 x9 hunting.rs:344 x3 |
| 31 | 30 | 206 | 4 | 318 | 12 | hunting.rs:315 x8 hunting.rs:344 x4 |

## Our consumers against SIM-RNG-2.4

Every place in `crates/sim` that draws from the simulation's generator, and
the item of SIM-RNG-2.4's list (and its own spec) it stands for:

| Place | What | SIM-RNG-2.4 item | Own spec |
|---|---|---|---|
| `movement.rs` `free_walk` (recorded as its caller) | a settler's starting direction and the choice among equally good directions on a free walk | settlers' movement | TRN-FREE-3.2, 3.2a |
| `hunting.rs` animal walk (`free_walk`) | an animal's starting direction and choices on its free path | **not listed (Q-232)** | ECO-FOOD-5.11 ("free path"), TRN-FREE-3.2 |
| `hunting.rs` `animal_target` | the N of the animal's target search | an animal's target | ECO-FOOD-5.10 |
| `specialists.rs` `next_spot` | a specialist's next spot | specialists' spot search | ECO-SPC-2.2 |
| `production/mod.rs` `sapling_kind` | the forester's tree kind | the forester's tree kind | ECO-OUT-4.5 |
| `production/mod.rs` `plant_spot` | the forester's (and farmer's) planting spot | the forester's spot search, outdoor work | ECO-OUT-4.2 to 4.5, ECO-FOOD-3.3 |
| `production/mod.rs` `start_fishing` | the fisher's spot | the fisher's spot | ECO-OUT-4.9, ECO-FOOD-4 |
| `distribution.rs` `take_out` | the first flag record a warehouse sends goods marked "out" to | wares (transport, one place), presumably | ECO-DIST-1.7 |
| `combat.rs` `send_attackers` | an attacker record's round counter, draw mod 2 | a fighter's start values, presumably | MIL-ATK-2.3 |
| `combat.rs` duel round | r1 and r2, each draw mod 7 | each duel round | MIL-ATK-6.3 |
| `conquest.rs` catapult | the miss test (mod 19) and a miss's direction (mod 6) | the catapult's aim | MIL-CON-5.4 |
| `navy.rs` making room | the direction a ship moves off a harbour spot | ships: making room at a harbour | SHIP-3.3 |

Built since (2026-10-19, SIM-RNG-2.4a): the shuffled start positions
(SIM-START-3.2), every new
deposit's lattice offset (TRN-FREE-5.4, ECO-OUT-4.5a) and the computer
players' food sub-need and expedition direction (AI-2.7 a, c), which now draw
from the simulation's generator inside the tick, as the last part of it
(`World::step_then`). Not built yet: the animals' respawn (ECO-FOOD-5.15,
Q-235) and the draws of the computer player's road-stop search (AI-2.7 b),
whose path our AI reads out without the free walk's rule. The leveller's
draw for his first field, built on 2026-10-19, is gone again: the leveller
draws nothing (ECO-HQ-4.3a as corrected, measured). The current
comparison with the original's generator is `docs/fidelity.md`.

How each place reduces the draw is Q-311 (open); ours take `draw mod n` as
their own specs say.
