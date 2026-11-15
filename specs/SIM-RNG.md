# SIM-RNG: Random numbers of the simulation

Status: draft; read from the binary, the generator and the simulation instance then measured in live trace 1 (2026-10-18 night, E-trace-001); rules marked "measured" are high, the rest medium
Owner question(s): trace preparation 2026-10-18, part 4 (first pass); raises Q-311, Q-312
Game build: 11757

## Summary

The original uses one kind of random number generator, the Park-Miller "minimal
standard" generator, in several separate instances. Exactly one instance drives the
simulation: it starts every new game from the same fixed seed, is stored in save
games, and is part of the multiplayer sync checksum, so the simulation is fully
reproducible from the same start and the same commands. All other instances, and the
C library's own clock-seeded generator, serve only visuals, sound, the camera, the
set-up screens and the built-in editor; they never change the game state.

## Rules

### SIM-RNG-1 The generator

- **SIM-RNG-1.1** A generator holds two unsigned 32-bit words: the state and the
  seed. Each draw replaces the state by (state x 16807) mod (2^31 - 1) and returns
  the new state, a value in 1 .. 2^31 - 2. The original computes this without a
  division; the results are identical to the plain modular product. Confidence: high
  (measured: in live trace 1 every one of about 4 700 states read from the running
  game, up to 19 425 draws after the seed, lies on this sequence; code read and
  cross-checked: from state 1 the 10000th value is 1043618065, the published check
  value).
- **SIM-RNG-1.2** Seeding with a 32-bit value s: clear bit 31; count the set bits n
  among bits 0..30. If n < 8, set the first 8 - n bits of the bit order B below
  (a bit that is already set still counts as one of them). If n > 24, clear the first
  32 - n bits of B (again counting bits already clear). The result is stored as the
  seed, and the state is set to it (to 1 if it is 0). B = 12, 23, 10, 25, 8, 27, 6, 29,
  4, 30, 1, 22, 9, 13, 21, 0, 17, 26, 5, 15, 18, 28, 11, 2, 14, 3, 24, 7, 19, 16, 20,
  31. Confidence: medium.
- **SIM-RNG-1.3** Reset: the state is set back to the stored seed (bit 31 cleared), or
  to 1 if the stored seed is 0. Confidence: high (measured: a second game in one
  session went from the state the first game left, 1889939646, back to the seed).
- **SIM-RNG-1.4** Two derived draws exist: an integer in lo .. hi inclusive, lo +
  (draw mod (hi - lo + 1)), and a fraction, (draw mod 1000001) x 0.000001 as a 32-bit
  float (0.0 .. 1.0 inclusive, steps of 1e-6). Every derived value uses exactly one
  draw. Confidence: medium.

### SIM-RNG-2 The simulation's generator

- **SIM-RNG-2.1** The simulation uses one generator, seeded 0x5432ACE0 when the
  program starts (13 set bits, so the repair of 1.2 leaves it unchanged). Callers in
  the simulation take the raw draw and reduce it themselves (usually a modulo).
  Confidence: high for the seed (measured: state and seed word both 0x5432ACE0 at the
  main menu of a fresh session); medium for the callers.
- **SIM-RNG-2.2** At the end of every game set-up the generator is reset (1.3), so
  every new game's first draw is 1257492815, whatever happened earlier in the
  session. Nothing reseeds it. Leaving a game does not reset it. Confidence: high
  (measured: the world appears with the state at the seed in both games of a session,
  and it stays there through ticks -1, 0 and 1; nothing drew between the reset and the
  first tick). Whether player creation (2.4, map setup) draws before the reset could
  not be seen: on a map with as many start fields as players there is nothing to
  choose, and no change of the state was seen during set-up (Q-312, narrowed).
- **SIM-RNG-2.3** A save game stores the generator's two words; loading restores
  them, so a loaded game continues the same sequence. The multiplayer sync checksum
  includes both words. Confidence: medium.
- **SIM-RNG-2.4** These systems draw from it (29 places), all inside the tick: map
  setup (the choice among a player's start candidates when players are created);
  settlers' movement (the first direction and the choice between equally good
  directions when a path is read out, TRN-FREE); settlers' work (specialists' and the
  forester's spot search, the forester's tree kind, outdoor work, the fisher's spot,
  the leveller); wares (transport, one place); animals and resources (respawn, goal
  choice, new deposits, an animal's target); combat (the soldier's approach, the
  catapult's aim, each duel round, a fighter's start values); ships (making room at a
  harbour); computer players (the food sub-need choice, the expedition's direction).
  The order of draws within a tick therefore follows the tick order (TOOL-trace
  TRACE-2.2) and each system's own object order. Confidence: medium; how each place
  turns the draw into a choice is read only where other specs already give it (Q-311).
- **SIM-RNG-2.4a Corrections from the tick-for-tick run (2026-10-19).** The 29
  places are call sites; some are shared, so the list of consumers is:
  - the path read-out of a free walk (TRN-FREE-3.2a) is one place reached by
    **settlers, animals (ECO-FOOD-5.14) and the computer player's road-stop search
    (AI-2.7 b)**; it draws nothing for a single-step route;
  - animals: each choice of a target draws once (ECO-FOOD-5.10) and the walk as above;
    the map's animals draw nothing at load (ECO-FOOD-5.13); a respawned animal costs
    two draws, its kind and its first wait (ECO-FOOD-5.15);
  - deposits: creating one (a forester's sapling, a sown field) draws once for its
    lattice offset (TRN-FREE evidence E-trn-003; ECO-OUT-4.5a);
  - the leveller draws nothing: his first field is the site's next step direction
    (ECO-HQ-4.3a, measured);
  - map setup draws only with shuffled start positions (SIM-START-3.2);
  - computer players: the food sub-need, the road-stop search of street construction
    attempts 3 to 5, the expedition's direction (AI-2.7); they run last in the tick,
    so their draws follow every other draw of that tick.
  - wares: the warehouses' take-out job (`ECO-distribution.md` ECO-DIST-1.7) draws
    once per warehouse that holds at least one good-flag record, on every tick with
    `t mod 7 = 0`, whether or not a unit is sent; it runs after the settlers and
    before the resources. Measured in E-trace-005 as the one extra draw of trace
    ticks 484, 554, 561 and 582 (counters 483, 553, 560, 581; Q-337); what creates
    the records there is Q-339.
  Confidence: high for the animals (measured, ECO-FOOD T31) and the take-out ticks
  (measured); medium for the rest (code).
- **SIM-RNG-2.5 Draws per tick, measured.** On MP_2P_Storm_Coast with the default
  set-up (player 0 a human who gives no command, player 1 the computer, OBS-30), the
  generator's draw count at the end of each tick (draws since the seed) is, for
  ticks -1 to 30: 0, 0, 0, 3, 4, 5, 12, 18, 26, 33, 37, 42, 50, 58, 61, 70, 77, 89,
  106, 114, 125, 125, 136, 155, 161, 161, 165, 177, 183, 202, 202, 206. Ticks 1 to 200
  make 1005 draws (0 to 20 per tick, 52 ticks with none); ticks 0 to 699 make 3043.
  The two games of one session gave the same count at every one of 698 ticks. The
  full list is in E-trace-001 (`rng-draws-game1.csv`). This is a check of the whole
  simulation, not of the generator alone: a reimplementation matches it only when
  every system draws the same number of times in the same ticks. Confidence: high
  (measured).

### SIM-RNG-3 Generators that do not affect the game

- **SIM-RNG-3.1** Separate instances serve visuals only: the terrain renderer
  (seed 0x400), the deposit renderer, the random idle animations, the settler
  renderer, settler motion (seed 0x7C0), animal building visuals (seed 0x7D6),
  particle emitters, ships' look, border and terrain cells, street looks and mesh
  instances (others seed 0x5432ACE0). One of them, used for per-element variation of
  trees and similar elements, is reseeded per element from a memory address, so that
  look differs from run to run. A reimplementation may use any generator for these;
  they must never draw from the simulation's generator. Confidence: medium.
- **SIM-RNG-3.2** The C library's generator, seeded once from the clock in the game
  set-up screens, serves random choices in those screens (for example among start
  positions), music and sound file choice, the loading screen picture and tip, the
  camera's idle wander and its random choice of a settler of the local player to
  follow, and the built-in editor's brushes. It is never used in the tick.
  Confidence: medium (all 56 call sites are outside the tick systems; the editor
  attribution of one group rests on file names).
- **SIM-RNG-3.3** Consequence for traces: with the same map, the same set-up and the
  same commands at the same ticks, two runs of the original produce identical
  simulation states tick for tick; visuals that use the address-seeded generator or
  the clock may differ. Confidence: high (measured: two games in one session drew
  identically, tick for tick, and gave every object the same id; and the same game
  recorded in two sessions 20 hours apart, E-trace-002 and E-trace-003, had the same
  generator state and draw index in all 11 529 ticks and the same settler, building,
  flag, street and player rows in every one of the 3852 snapshots compared, trace
  ticks 0 to 11 519, with the same ids. Only the renderer's clip pool index, view
  object and pose clock differed: a per-session offset, 2 rows, and one frame of
  0.025 s in 16 668 rows). A trace recorded once stands for every run of that game.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| multiplier | 16807 | | high | E-tool-004 |
| modulus | 2^31 - 1 = 2147483647 | | high | E-tool-004 |
| simulation seed | 0x5432ACE0 = 1412607200 | | medium | E-tool-004 |
| repair bit order B | 12, 23, 10, 25, 8, 27, 6, 29, 4, 30, 1, 22, 9, 13, 21, 0, 17, 26, 5, 15, 18, 28, 11, 2, 14, 3, 24, 7, 19, 16, 20, 31 | bit index | medium | E-tool-004 |
| repair thresholds | fewer than 8 / more than 24 set bits | bits | medium | E-tool-004 |
| fraction resolution | 1000001 values, step 1e-6 | | medium | E-tool-004 |
| terrain renderer seed | 0x400 | | medium | E-tool-004 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | seed 0 | seed | stored seed 0x2A801540 (8 bits: 6, 8, 10, 12, 23, 25, 27, 29), state 713037120 |
| T2 | seed 12345 (0x3039, 6 bits) | seed | stored seed 0x00803039 (7 bits: bit 12 was already set, bit 23 added) |
| T3 | seed 1 | seed | 0x0A801541 |
| T4 | seed 0x7FFFFFFF (31 bits) or 0xFFFFFFFF | seed | 0x7FFFEFFF (30 bits: only bit 12 cleared) |
| T5 | state 1 | 10000 draws | state 1043618065 |
| T6 | the simulation's generator | read it at tick 0 of two new games started in one session | both 1412607200 (passed in live trace 1) |
| T7 | game seed | range 0..5, ten times | 5, 4, 2, 0, 3, 5, 5, 0, 3, 4 |
| T8 | game seed | fraction, three times | 0.491558, 0.170283, 0.4644 (as 32-bit floats) |
| T9 | first 20 draws (rows below) | | as listed |
| T10 | MP_2P_Storm_Coast, default set-up, no commands | run ticks -1 to 30 | draws since the seed at the end of each tick as in SIM-RNG-2.5 (0, 0, 0, 3, 4, 5, 12, ...); 1005 draws in ticks 1 to 200 |
| T11 | two new games in one session, the first played for a while | read the generator at the end of each tick of the second game | the same as in the first game, tick for tick |
| T12 | MP_2P_Storm_Coast, default set-up, the map's animals as loaded | draws per counter tick 2 .. 40 | at least the N-draws of ECO-FOOD T31, equal to the original's totals once the animals' walks draw as TRN-FREE-3.2a; ticks 20, 24, 29, 33, 34, 40 without a draw |

| Draw | seed 0 (repaired 0x2A801540) | seed 12345 (repaired 0x00803039) | game seed 0x5432ACE0 |
|---|---|---|---|
| 1 | 1056125580 | 1608380016 | 1257492815 |
| 2 | 1350280605 | 1666264123 | 1295171578 |
| 3 | 1706430386 | 1714358381 | 1054465454 |
| 4 | 331391817 | 433217668 | 1365830334 |
| 5 | 1277171648 | 1119782746 | 1057720755 |
| 6 | 1324836171 | 1789413361 | 243099419 |
| 7 | 1411073901 | 1309365739 | 1258038539 |
| 8 | 1257140286 | 1245044564 | 1877220258 |
| 9 | 1812667616 | 383330780 | 1758618129 |
| 10 | 1301605770 | 189478460 | 1277460442 |
| 11 | 1819748048 | 1993712366 | 1883629635 |
| 12 | 43342162 | 1136391221 | 2106835018 |
| 13 | 454760401 | 1755178576 | 1865775790 |
| 14 | 263759934 | 1450951640 | 537489036 |
| 15 | 606963330 | 1467401795 | 1262008770 |
| 16 | 685364060 | 919766417 | 2032899618 |
| 17 | 1958957559 | 926879413 | 479055956 |
| 18 | 1127901956 | 215918953 | 577259889 |
| 19 | 810022423 | 1849963288 | 1823320924 |
| 20 | 1148025028 | 1064740150 | 2110610625 |

## Evidence

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-tool-004 | code reading | S2DNG.exe: the generator and its bit-order table; the simulation instance (seed, reset at the end of set-up, draw, save, load, sync checksum); 29 draw sites; other instances and embedded ones; the C library's rand, srand seeded from the time, 56 call sites | generator, instances, callers |
| script | our model of the generator | a self-check, draws for any seed | test values |
| E-trace-001 | live read of the running game | `rng-watch.csv` (every change of the generator from the main menu through two games), `rng-draws-game1.csv`, `rng-draws-game2.csv` | SIM-RNG-1.1, 1.3, 2.1, 2.2, 2.5, 3.3 measured |
| E-trace-002 | live read of the running game | `generator.csv`, `specs/traces/original-generator.s2trace.gz` | draws per tick 0 .. 150 000 (756 390 draws) |
| E-trace-003 | live read of the running game | `generator.csv` and the tables, compared with E-trace-002's | the same game as E-trace-002 recorded again a day later: generator and every table identical over 11 529 ticks (3.3) |
| E-sim-001 | code reading | | the consumers of SIM-RNG-2.4a |

## Open questions

- Q-311: how each of the 29 simulation draw sites turns the draw into a choice
  (modulo of what, comparison with what), per system; only partly given by other specs.
- Q-312: narrowed by live trace 1: nothing draws after the reset before tick 2, and a
  new game always starts at the seed. Left open: whether player creation draws before
  the reset, which only matters on a map with more start fields than players (then the
  start choice could depend on earlier games). Test: such a map, two games in one
  session, compare the castles' fields.
