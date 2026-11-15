# AI: Computer opponent, overview

Status: draft (structure and schedule read from the binary; decisions in the AI-* files)
Owner question(s): plan step 5.1 (overview); new Q-067 to Q-071
Game build: 11757

## Summary

Every computer player is driven by one AI object that thinks once per logic
tick. Its work is split into subsystems (construction, destruction, military,
expansion, resources, needs, arrangements) and many small tasks (build this
building, build that road, attack that target). Each subsystem runs on its own
fixed period, mostly prime numbers of ticks, and the periods are shifted per
player so that computer players do not all think in the same tick. Three
profiles (weak, normal, strong) from `ai.lua` set a handful of numbers and
switches; everything else is the same for all three. This file maps the
structure and the schedule. The rules for each decision are later specs
(AI-*.md), listed under open questions.

## Rules

### AI-1 Profiles

- **AI-1.1** A computer player's strength chosen in the game setup (weak /
  normal / strong, `UI-game-setup.md`) selects profile 1 / 2 / 3 of
  `ai.lua` (`ai_create("1")`..`("3")`). Confidence: high. A free game's computer slot
  starts at strength 0 (`UI-game-setup.md` UI-SETUP-2.5), so the recorded two-player game
  (TOOL-trace E-trace-002 .. 008, `traces/`) has a **Weak** computer player: measured
  2026-10-25 (Q-275), its AI player's profile reference is the profile named "Weak", whose
  values are held as in 1.2 (heap dumps of E-trace-007 at trace ticks 1 and 6000, a memory read
  of the running game); a reproduction of that game must use
  profile 1.
  **What the setting does (Q-1007, 2026-11-04, E-cam-003):** the free game's
  Hard (strength 2, profile 3 "Strong") and a mission's strength 2 take the same path and differ
  from Weak only by the values of 1.2: the strength reaches the AI only as this profile, and a
  profile holds only those values. The start stock does not depend on it (ECO-HQ-2.2). A mission's
  strength comes from the map (CAM-4.3a), a free game's from the slot.
- **AI-1.2 Profile values** (shipped):

  | Setting | Weak (1) | Normal (2) | Strong (3) | Used by (AI-3) |
  |---|---|---|---|---|
  | military mode | 0 Safe | 2 Aggressive | 1 Normal | military settings, attack system |
  | start sawmill count | 1 | 2 | 3 | construction |
  | maximum smithies | 2 | 2 | 4 | weapon production |
  | maximum mints | 0 | 1 | 3 | coin production |
  | attack strength | 0.51 | 0.75 | 1.0 | military settings (`MIL-attack-combat.md` MIL-ATK-7.4) |
  | street optimizing | off | on | on | street optimizer |
  | route optimizing | on | on | on | street route optimizer |
  | catapults | off | on | on | catapult construction |
  | intercepting factor | 0.5 | 1.0 | 0.5 | military settings (MIL-ATK-7.4) |
  | coin arrangement | off | off | on | coin arrangement |

  Note: "Normal" is the aggressive one, "Strong" uses mode Normal. The
  "used by" column is from the names of the tasks that exist for each switch;
  which task reads which value is confirmed only for attack strength and
  intercepting factor. Confidence: high for the values; medium for "used by".
  **Measured for Strong** (E-trace-012, the Hard set-up; 2026-10-27 b): start sawmill
  count 3 -> three plank need records at the start (`SIM-game-start.md` T14) and
  sawmill sites at counters 323 (138, 49), 1851 (139, 55) and 2540 (136, 55) (R3, every tick; E-trace-012 samples every 10 before 5500) (the
  Weak game: one, 323); street optimizing on -> its flags (`AI-construction.md`
  T58); catapults on -> catapult sites at 4923 and 17276; attack strength 1.0,
  intercepting 0.5 and the defense type left at 1 by mode Normal, in the save
  (`AI-military.md` T21). Not reached by counter 20 000: mints, coins, smithies
  beyond the first, attacks. **R3** (E-trace-013, the same game every tick to counter
  150011; `AI-military.md` T23): smithies at counters 3286, 52726 and 97387 (no fourth by the end,
  though the profile allows four; why is not traced); mints at 27327 (122, 62), 46724 (146, 34) and 84935 (139, 22) (three, the
  profile's maximum; no fourth); the first coin in the player's stock at 37247, in a
  storehouse at 37613, in a barracks at 52371 (coin arrangement on); catapult sites at 4923 (136, 49), 17276 (133, 59), 80425 (113, 55), 83242 (122, 34) and 88175 (142, 15);
  no attack: the player expands by sea far to the south (harbours on (52, 138) and
  (22, 136)) and never comes near the idle human's castle on (29, 51).
- **AI-1.3** Building and good scripts also carry AI weights:
  `bp_setAiScore` (24 building types, e.g. harbour 100, gold mine 180,
  catapult 300 / 200), `bp_setAiThreat` (6 military types: barracks 20, guard
  house 30, tower 60, fortress 90, headquarters 150 / 10, catapult),
  `gp_setAiPriority` (24 goods). Their use is not traced. Confidence: high
  for the values (`buildings.lua`, `Goods.lua`).

### AI-2 When the AI runs

- **AI-2.1** Once per logic tick, in the tick driver, the AI system raises
  its own tick counter `c` by 1 and then, for each player slot 0..7 in order:
  if the slot holds a player (every player created for the game, human or
  computer; nothing clears this on defeat, Q-159) its military map is
  updated; if the
  slot has an active AI: if it has no cells left it is reset and switched off
  (`AI-misc.md` AI-MISC-2.3), else it **thinks** with `c`. Confidence: high.
- **AI-2.1a Place in the tick and the counter's value.** The AI system is
  the **last** step of the logic tick, after the players (buildings), roads,
  settlers, transport, resources (animals), decorations, military, navy,
  decay and triggers. Its counter is 0 when the system is made and is raised
  before the first think, so in the update that sets the game counter to
  `k` (TOOL-trace TRACE-4.5: trace tick `k + 1`) every computer player
  thinks with `c = k + 1`, from the very first update (`k = 0`, `c = 1`).
  There is no start delay beyond the phases below. The AI's commands (site
  placement, roads, flags) take effect inside the same tick, so what it
  decides is in that tick's state (a site unlevelled, ECO-HQ-4.2a). Each
  takes effect when its part issues it: a later part of the same think
  already reads the changed state (an upgrade of the military system's run
  makes the building unfinished for the route optimizer of that think,
  `AI-construction.md` AI-CON-8.3 b, T69; added 2026-10-28 night).
  Confidence: high (code, and E-trace-002: 17 of player 1's first 22 roads
  appear at game counters `k = 1 (mod 7)`, as `(k + 1 + 5) mod 7 = 0`
  requires; the other 5 are road splits by site flags).
- **AI-2.1b Every command takes effect at once** (Q-289, 2026-10-30; E-ai-052).
  A computer player issues no queued commands: each part calls the same world
  functions that carry out a human player's commands, directly, while it
  runs. So every command is in effect for every later part of the same think
  (and for the players thinking after it in that tick):
  - a site, with its flag and records (`AI-construction.md` AI-CON-5.3, from
    the executing order's step, which comes **after** the construction
    system's street, forester, ship and hunter parts of that think, AI-2.4a);
  - a road with the flags it needs at its ends (the street construction, both
    route optimizer variants; AI-CON-8), and its planning stub (SIM-START-4.4 c);
  - an upgrade (AI-CON-8.3 b, T69);
  - a building torn down and a flag removed (the destruction system, AI-2.6);
  - the settings: production on or off (production control), the tool
    sliders (locksmith), the warehouses' settings (goods arrangement), the
    distribution and food settings, the military settings, the castle's
    soldiers, the coin supply.
  The commands that **seemed** to wait for the think's end (sites, flags,
  roads) only seemed to because the executing order's step was taken to come
  first in the construction system's run; in its true place (AI-2.4a) a site
  placed in tick k is first seen by the street construction's next run, not
  by the run of tick k. Confidence: high (code: the AI's call sites of the
  world functions, none through the command queue; measured T7).
- **AI-2.2 Phase.** In a think, a subsystem with period `p` runs when
  `(c + 5 x s) mod p = 0`, `s` the player slot. So player 1's subsystems run
  5 ticks after player 0's, and so on; in game counters, player 1's part of
  period `p` runs when `(k + 6) mod p = 0`. A think runs every part that is
  due, so one tick can hold several decisions. Confidence: high.
- **AI-2.3 Think schedule** (player level, in this order within a tick):

  | Period (ticks) | Part |
  |---|---|
  | 13 | needs arrangement |
  | 17 | needs construction |
  | 1 | resource system |
  | 23 | cell system |
  | 1 | expansion system (AI-2.5) |
  | 1 | military system (AI-2.5) |
  | 1 | construction system (AI-2.4) |
  | 1 | destruction system (AI-2.6) |
  | 47 | need creation |
  | 337 | locksmith (tool production) |
  | 643 | food arrangement, then goods arrangement (same tick) |
  | 67 | production control |
  | 1323 | conquered continents |

  Confidence: high for periods and order; medium for the part names (paired
  with the class names by code address).
- **AI-2.4 Construction system**, inner periods (same `c + 5s` value), in
  this order within its run (corrected 2026-10-30, Q-376: the ship construction
  runs between the forester and the hunter):

  | Period | Task |
  |---|---|
  | 7 | street construction |
  | 67 | street optimizer |
  | 37 | street route optimizer |
  | 3 | forester construction |
  | 673 | ship construction |
  | 3 | construction hunter |
  | 600 | resets its construction budget (two counters to 0) |
  | 50 | while no order is being executed: raises the budget by 1 while below its limit, else takes an order |

  Confidence: high for the periods and names (`AI-construction.md` AI-CON-5.2
  for the budget, AI-CON-7 for forester and hunter).
- **AI-2.4a The executing order comes last** (added 2026-10-30, Q-376, E-ai-052). After the parts above, the run does **one** of two things: if no
  order is being executed, the 50-tick step (when due); else the executing
  order's step (AI-CON-5.3a), and, if that step ends the order (placed, no
  material, no site), the end handler of the order's source (its slot is
  cleared, failure marks are set, L doubles on a placement). So everything the
  executing order does in a tick (a site placed, a slot freed) happens after
  that tick's street construction, street optimizers, forester, ship and hunter
  runs: they see it in their next run, never in the same think. A slot freed in
  tick k is used by its source's run at the first due run after k (a hunter run
  in k itself still finds its own order). The order taken by a 50-tick step
  makes its first step in the next tick. Confidence: high (code; measured T83).
- **AI-2.4b Resource system**, inner periods (same `c + 5s` value, in this
  order within a run; Q-160):

  | Period | Task |
  |---|---|
  | 37 | expiry of the cells' "full" marks (AI-CON-1.1) |
  | 19 | crowded cells (AI-CON-7.6) |
  | 1423 | depot (AI-CON-7.4) |
  | 5 | cell validity scan, one grid position per run, cursor advanced first (AI-CON-1.1) |
  | 53 | stone pit and woodcutter (AI-CON-7.3) |
  | 997 | harbour (AI-CON-7.5) |

  Confidence: high.
- **AI-2.5 Military system**, inner periods: 311 military building / depot
  placement, 53 military settings (`MIL-attack-combat.md` MIL-ATK-7.4: attack
  strength, intercepting, near/medium allocation, defense type), 641 **attack
  system** (choosing and launching attacks), 409 soldier creation, 5 catapult
  construction, 613 castle (headquarters) settings, 631 weapon production,
  647 coin production, 653 coin arrangement, 37 military upgrade. Expansion
  system: 2111 **expedition** (it uses the harbour's "prepare expedition",
  `SHIP-ships-expeditions.md` SHIP-5.1), 29 expansion targets (four calls).
  Confidence: high for the periods; medium for the names of 311 and 29.
- **AI-2.6 Destruction system.** Every 41 ticks it takes the **next** of its
  own buildings in id order (a cursor that moves on each time; the cursor holds
  the last handled building's id + 1 and the run takes the first building whose
  id is not below it; **when there is none, that run only resets the cursor**
  and handles nothing, so the next run starts again with the first building:
  a cycle over n buildings takes n + 1 runs; corrected 2026-10-25 evening,
  `AI-construction.md` T41, E-ai-021) and, unless
  the building is protected (three flags and a fourth, not traced), tests
  whether it is no longer useful; if so it is torn down. So at most one
  building per 41 ticks is removed by the AI. Every 59 ticks (when the tick
  is not a 41-tick) cells marked for clearing lose one building, and in
  **every tick that is neither a 41- nor a 59-tick** the next of the
  player's flags (a cursor over the flag list, wrapping) is tested and
  removed if useless. Rules in `AI-construction.md` AI-CON-9. Confidence:
  high.
- **AI-2.7 Task updates.** Besides the think, the AI updates all its live
  planning objects (buildings, needs, productions, cells) every tick. The
  update only re-resolves their references to game objects and to each other
  and recomputes the buildings' protection flags; it takes no decision
  (`AI-construction.md` AI-CON-1.3). Confidence: high.

- **AI-2.7 Draws from the simulation generator.** A computer player draws from the
  simulation's generator (SIM-RNG-2.4a), inside its think, so every draw shifts the
  later draws of the game. The places:
  - a. **Food sub-need.** When a food need is created and no producing building is
    bound to it: `d = draw mod 10`; `d` 0 to 6 asks for fish, 7 or 8 for bread, 9
    for flesh. When a fisher is already bound to it and flagged: `draw mod 6` equal
    to 5 asks for flesh, else bread. One draw either way.
  - b. **Street construction, attempts 3 to 5 (AI-CON-8.1 a).** The road-stop search
    reads its path out with the free walk's rule (TRN-FREE-3.2a): one draw for the
    starting direction when the path is longer than one step, one more at every step
    where the kept direction is not allowed. Attempts 1 and 2, the road planner's
    roads and the street route optimizer read without draws. The search (Q-239): the
    free walk's labelled search (TRN-FREE-2: the open list by f = g + h, h the
    lattice distance to the field the path is read from, first in first out among
    equal f, relabelling, the stop as soon as that field is labelled), with the
    computer player's step cost in place of the walker's and **one try only** (no
    second try with every field passable). The flood starts at F (the flag to
    connect) and the path is read from T down the labels to F, so the first drawn
    direction is T's; a path of one step (T next to F) draws nothing. The step cost
    from n to q: -1 (not enterable) when q is farther than 4 x distance(F, T) from
    the search's start, when q is in the excluded positions, when q is not usable by
    the player (inner land of `TER-territory.md` TER-6.1, so never a Border field, and
    not Blocked, DepositBlocked or MovementBlocked; the goal is tested too; corrected
    2026-10-31 night, `AI-construction.md` AI-CON-8.1e, T93), when a flag stands on q
    (unless q is the goal), or when q is a road
    field that is neither the goal nor on the hit road R; else the free walk's cost
    of n -> q plus the cost of q -> n (3 + 3 level, 4 + 3 one class up or down, ...;
    the free walk's + 2 each way for a deposit-blocked field never applies here, such a
    field not being usable), doubled when q is a harbour field.
    The hit: while no hit is noted, the first road field q evaluated whose road has T
    as an end is noted as the hit (one label per field; the hit's road then counts
    as enterable); the note is taken after q passed the bound, the excluded
    positions and the usability test and before the flag, road and walkability (cost) tests. The
    attempt compares the read-out path's **field count, F and T both included**, with the best
    length L (`AI-construction.md` AI-CON-8.1 a, corrected 2026-10-24 night); the walk paths of
    attempts 1 and 2 and the second legs are the same search with the hit note off (AI-CON-8.1e).
    Without a
    hit, or without a path to T, the attempt reads nothing out and draws nothing
    (Q-245). The search runs once in each of the attempts 3, 4 and 5, and
    **once more in the execution (attempt 6) when the best strategy is 3, 4 or 5**
    (the execution repeats the road-stop search before removing R and building the
    roads; strategies 1 and 2 execute without it). Measured (E-trace-002, the
    animals' draws matched exactly): the computer player draws 2 after the animals in
    each of trace ticks 366, 373, 380 and 387, the sawmill flag's attempts 3, 4, 5 and
    its execution (strategy 4: roads F -> T and F -> P at 387, counter 386), each a
    two-field path's first direction and one step; also 2 in trace tick 331 and 1 in
    431 (Q-329). Confidence: high for the schedule and the counts, medium for the
    search's details (code, E-sim-002).
  - c. **Expedition.** When a harbour's expedition ship is ready and no continent is
    chosen: the directions whose entry is set (up to 6) are counted and `draw mod
    count` picks one; one draw.
  - d. **Order.** The AI runs last in the tick (TOOL-trace TRACE-2.2), player slots in
    order, so its draws come after every simulation draw of the tick. A
    reimplementation whose computer player uses its own generator can match the
    original's draw count only until the first AI draw.
  Confidence: medium (code, E-sim-001; the first AI draw of the reference game was
  not isolated in the recording).
### AI-3 What it decides (structure)

The AI's parts, by the class names in the game; each is a later spec.

- **AI-3.1 Knowledge.** Resource map and low-resolution resource map (where
  wood, stone, ore, fish, game are), territory map, military map (per player,
  threat around), conquered continents (which land masses it holds), cells
  (the map cut into cells: "cell system", cell validator, full cells,
  references from cells to buildings, needs and productions).
- **AI-3.2 Economy.** Needs (what it lacks: "need creation", "needs
  arrangement", "needs construction" turning needs into planned buildings),
  production control (switching production), goods and food arrangement
  (distribution settings, food weights `ECO-distribution.md`), locksmith
  (tool orders), coin production and arrangement, weapon production.
- **AI-3.3 Building.** Construction orders, "search construction place"
  (its own site estimator, same field rules as the player's building help
  plus crossing roads where a flag can go, `ECO-building-placement.md`),
  building construction, forester construction, ship construction, harbour
  creation, cell depot (warehouse) creation, military building creation,
  catapult construction, cell clearing, construction hunter.
- **AI-3.4 Roads.** Street construction (connecting new buildings), street
  optimizer and route optimizer (profile switches), and road removal
  (the AI removes roads and flags with the player's own removal rules,
  `TRN-roads-flags.md` TRN-ROAD-6).
- **AI-3.5 Military and expansion.** Military settings, soldier creation,
  military upgrade, castle settings, expansion targets and cell expansion
  (where to push the border), attack target selection, attack execution
  (launching attacks with the player's attack order, `MIL-attack-combat.md`
  MIL-ATK-1.5), expeditions.
- **AI-3.6 Destruction.** Removing buildings it no longer needs (AI-2.6).
- **AI-3.7** The AI acts through the same commands and rules as a human
  player (building placement, roads, attack orders, settings, expedition
  start). Exceptions found so far: it writes some military settings directly
  each 53 ticks rather than through the settings dialog. Confidence: medium.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| player phase shift | 5 x slot | ticks | high | E-ai-001 |
| think periods | see AI-2.3 .. 2.6 | ticks | high | E-ai-001 |
| destruction period | 41 | ticks | high | E-ai-001 |
| attack period | 641 | ticks | high | E-ai-001 |
| expedition period | 2111 | ticks | high | E-ai-001 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | computer player in slot 2, AI counter c | ticks | its attack system runs in ticks where `(c + 10) mod 641 = 0`, its military settings where `(c + 10) mod 53 = 0` |
| T2 | computer players in slots 0 and 1 | ticks | slot 1 runs each part 5 counter steps after slot 0 (e.g. street construction: c = 7k for slot 0, c = 7k + 2 for slot 1, since (c + 5) mod 7 = 0) |
| T3 | weak computer player | profile | attack strength 0.51, catapults never built, no mints |
| T4 | computer player with 10 buildings | 410 ticks | at most 10 destruction checks (one per 41 ticks), each on the next building in id order |
| T5 | MP_2P_Storm_Coast, default set-up (player 1 computer), E-trace-002 | ticks | player 1's roads are placed in game counters `k = 1 (mod 7)` (observed 176, 645, 862, 1163, 1541, 1968, 2029, 2794, 3277 ..); its 50-tick construction steps fall on `k = 44 (mod 50)`, its budget resets on `k = 594 (mod 600)` (AI-CON-5.2) |
| T6 | any computer player | first update of a new game (`k = 0`) | it thinks with `c = 1` after every other system of the tick; its hunter check (period 3, slot 1: `(k + 6) mod 3 = 0`) runs in that update |
| T7 | AI-2.1b, 2.4a (Q-289): every recorded pair (E-trace-002 / 005 / 011 / 013 / 014) | the experiment build with every computer command in effect at once for the rest of the think, with and without the executing step after the construction system's inner parts | with the step last (AI-2.4a; and AI-CON-1.1 T82, TRN-CARRY-3.4 T50) every measure is the same as with only the upgrades at once: Weak vs R4 settlers 34840, buildings 35420, generator 35240; the Hard 20 000 pair equal in buildings and generator; R3 settlers 20080, buildings 22280, generator 22107; 150 000 generator 35211. With the step first (every command at once, nothing else changed) every pair differs from trace tick 180 (settlers; buildings 560, generator 359): the street construction of counter 127 (E-trace-002), placed after the site in the same think, takes the first site's flag (140, 49) one street run (7 ticks) early (T5's road at 176, stub 177) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-ai-001 | code reading | `S2DNG.exe` | AI system think per tick: counter, 8 slots |
| E-ai-001 | code reading | | player think schedule (c + 5 x slot) |
| E-ai-001 | code reading | | construction, military, expansion, destruction schedules |
| E-ai-001 | code reading | | task updates |
| E-ai-001 | strings | the game's type names | class list and address ranges used to name the parts |
| E-mil-003 | code reading | | military settings (attack strength, intercepting) |
| data | script | `ai.lua`, `buildings.lua`, `Goods.lua` | profiles and weights |
| E-ai-006 | code reading | | player-present test for the map update (set at creation, never cleared); resource system inner periods (AI-2.4b) |
| E-sim-001 | code reading | | AI-2.7 |
| E-ai-007 | code reading | | AI-2.1a, 2.2, 2.6; AI-CON-5.2 to 5.4 (Q-320) |
| E-ai-052 | code reading | the construction system's run (the executing step last); a site, a road, flag removal, tear-down and upgrade, each with its callers; the settings parts (direct setters); the command queue's handlers (no AI part calls into them) | AI-2.1b, 2.4a, T7 |
| E-trace-002 | recording | streets and buildings tables, counters 0 .. 5000 | road ticks `k = 1 (mod 7)` (AI-2.1a); the first ten sites (AI-CON-5 T21) |

## Open questions

- Q-067: each decision in detail: construction, needs, roads, demolition,
  production control, tools and food are in `AI-construction.md`; military,
  expansion, ships and profiles follow in `AI-military.md`, `AI-misc.md`.
- Q-068: answered in `AI-construction.md` AI-CON-9.
- Q-069: answered in `AI-misc.md` AI-MISC-2 (reset and switch-off when no
  cell is left).
- Q-070: answered in `AI-misc.md` AI-MISC-3.
- Q-071: answered (AI-2.7, `AI-construction.md` AI-CON-1.3); the caller of
  the update relative to the think is still untraced.
