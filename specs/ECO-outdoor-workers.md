# ECO-OUT: Woodcutter, forester and stonemason; tree growth

Status: draft (sequence, searches, durations and growth read from the binary; free-walk route and its timing not traced)
Owner question(s): Q-005 (part), Q-038 (first production chain) in `OPEN_QUESTIONS.md`; play-test chunk 3 (woodcutter door-to-door time, swing count, stand position; fisher, hunter, farmer cycles)
Game build: 11757

## Summary

The woodcutter, the stonemason (stone pit) and the forester work outside their
building. For the woodcutter and stonemason the **building** looks for a
target every 10.1 s: the nearest fully grown tree or stone pile within its
working radius, found by a flood over walkable fields. It reserves the target
for its worker. The worker checks every 5.1 s whether a target is reserved,
walks out by the flag, works 5.5 s at the target, carries the log or stone
back to the flag, goes in and rests 22.3 s. A stone pile shrinks by one size
per stone and disappears after its last one. The forester picks a random spot
among the 11 nearest plantable fields, plants a sapling (5.5 s), and rests. A
sapling grows from size 0.1 to 1.0 in 1001 ticks (100.1 s) and can then be
felled.

## Rules

Ticks are 0.1 s (`ECO-time-units.md`). Distances, directions and ring order as
in `MAP-grid-geometry.md`; timed actions as in `ECO-worker-cycle.md`
ECO-WORK-2; walking as in `TRN-settler-movement.md`; field state bits as in
`ECO-building-placement.md` ECO-PLACE-1.

### ECO-OUT-1 Data

- **ECO-OUT-1.1** Building data (`buildings.lua`):

  | Building | Worker | Product | Working radius | Production time |
  |---|---|---|---|---|
  | `woodCutter` | woodcutter | wood | 6 | 55 |
  | `stonePit` | stonecutter | stone | 8 | 55 |
  | `forester` | forester | (none; planting good wood) | 6 | 55 |

  The woodcutter's radius is set twice in the script (5, then 6); the last call
  wins. A building type without `bp_setWorkingRadius` has radius 4.
- **ECO-OUT-1.2** Rest time of all three professions: 222 ticks
  (`gp_setRestTime`), so a rest lasts 223 ticks (ECO-WORK-3.9).
- **ECO-OUT-1.3** A deposit is an object on a field (`deposits.lua`): trees
  (`tree_01` .. `tree_14`, `tree_lava_01..03`; good wood), stone piles
  (`stone_01..06`, `med_stone_01..06`; good stone) and grain fields. Each has a
  size. A deposit is **ripe** when its size is at least 1.0, it is not being
  removed (ECO-OUT-6.4), and it has no remaining lifetime count (only grain
  fields have one, `dp_setLifeTime`).
- **ECO-OUT-1.4** Deposit defaults when the script sets nothing (all trees and
  stones): growing start size 0.1, growing speed 0.0009 per tick, no lifetime,
  no next deposit, random position offset on (TRN-FREE-5.4), not blocking.
- **ECO-OUT-1.5 Field bits of a deposit** (Q-117): every deposit sets
  DepositBlocked (ECO-PLACE-1.5) on its field **as soon as it is created**, also
  a forester's sapling before planting ends, and clears it when it is removed.
  `dp_setBlocking` (default off) additionally sets **Blocked** (bit 0): stone
  piles and grain fields have it, trees do not. So tree fields stay walkable
  (TRN-FREE-2.5), stone and grain fields do not.

### ECO-OUT-2 Target search by the building (woodcutter, stone pit)

- **ECO-OUT-2.1** Every finished building runs a search counter, 0 when the
  building is made. In each update of the finished building it first goes
  up by 1, or, when it has reached 100, is set back to 0 and the search below
  is tried; then, in the same update, it goes up by 1 more if it is not 0
  (that second step belongs to the hunter's animal search, ECO-FOOD-5.1, but
  runs for every building). So it counts 2, 4, .., 100 and then 0: **one try
  every 51 ticks**, the first 51 ticks after the tick the building is finished
  (corrected 2026-10-27 night from "every 101 ticks": E-eco-024: the update runs the
  search step and then the hunter's step, both on the building's counter; measured T22).
  The worker request (`ECO-requests-dispatch.md` ECO-REQ-1.4) and a mine's ore
  search (`ECO-food-mining.md` ECO-FOOD-2.2) have counters of their own.
  (The forester and fisher use other searches.)
- **ECO-OUT-2.2** The try is made only if the building type has a product
  (`bp_setGoods`), that product is a good gathered from deposits
  (`gp_enableDeposits`: wood, stone, grain), the building is not a hunter, and
  the building has **no reserved target** at the moment.
- **ECO-OUT-2.3** Search area. A flood (breadth first) starts at the
  building's field. Neighbours of a field are taken in ring order: directions
  5, 0, 1, 2, 3, 4 (MAP-GRID-3.2). Each field is entered at most once. A field
  B is entered from field A if it is on the map and:
  - B is **walkable**: none of the bits Blocked (0), Water (2),
    ElevationBlocked (6), MovementBlocked (8), DoodadBlocked (10) is set
    (DepositBlocked, bit 7, does **not** stop the flood); or
  - B carries a deposit, B is not ElevationBlocked, and A is walkable or A is
    the building's own field.
  Fields that are entered are queued; the queue is first in, first out.
- **ECO-OUT-2.4** Goal test, applied to each field when it is entered
  (including the building's field): the field is a goal if its distance to the
  building field (MAP-GRID-2.1) is at most the working radius and it carries a
  deposit whose good is the product, that is ripe (ECO-OUT-1.3) and not reserved
  by any building. The **first** goal in flood order is the target; the search
  ends.
- **ECO-OUT-2.5** The search also ends, without a target, as soon as a field
  is entered whose distance to the building field is more than radius + 4, or
  when the queue is empty.
- **ECO-OUT-2.6** A target found is reserved for the building (a deposit can
  be reserved by one building only). The reservation ends when the worker has
  harvested it (ECO-OUT-3.6) or the building is removed.

### ECO-OUT-3 Woodcutter and stonemason cycle

After the worker has entered the building (`ECO-worker-cycle.md`), these steps
repeat. Times are on level ground with room at the flag.

- **ECO-OUT-3.1 Start.** The step starts only while the building is enabled
  (not stopped by the player); otherwise it is retried every tick.
- **ECO-OUT-3.2 Wait for a target** (inside the building). A countdown starts
  at 50 and falls by 1 per tick; when it is 0, the worker checks whether the
  building has a reserved target. If yes, he leaves (next step). If not, the
  countdown is set to 50 again, and, if the building is enabled and its
  productivity is below 0.5 (ECO-OUT-3.11), the player gets the message "No
  resources found." **The first check is 51 ticks after the step starts, then every 51
  ticks.** So the worker always spends at least 51 ticks inside before leaving,
  even if a target was reserved earlier.
- **ECO-OUT-3.3 Walk out** to the building's flag (one field, direction 1;
  23 ticks).
- **ECO-OUT-3.4 Walk to the target** over the terrain (not along roads), by
  the free walk of `TRN-free-walk.md` with stop short 1: **the worker stops on
  the last field of the route before the target's field** (TRN-FREE-4.2). Then
  a straight approach on the height lattice to the deposit's position, stopping
  short by the deposit's elevation size (`dp_setElevationSize`: 1 for trees and
  small stones, 2 for the larger stone piles) (TRN-FREE-5).
- **ECO-OUT-3.5 Work.** A timed action lasting the production time: **55
  ticks**. At the same moment a removal timer of round(work time) + 5 = 60
  ticks is set on the tree; it counts down in each resource update while the
  tree's removal has not started, beginning in the work's own tick W, so it
  runs out in tick W + 59 (**play test 10, Q-216 a, E-gfx-022**). A harvest in
  time (3.6, W + 55) starts the removal first and the timer stops. If it runs
  out, it starts the removal itself: the removal counter is set to 1 and, unlike
  the harvest (where the same tick's resource update already raises it to 2),
  is raised only from the next tick, so the tree is deleted 19 ticks later
  (counter 20, ECO-OUT-6.4). (**Corrected Run D, E-gfx-014: the timer does
  not schedule the fall; the fall is drawn only, 3.0 s into the work,
  `GFX-animations.md` GFX-ANIM-3.8a; a tree removed by the timer sinks
  upright, 3.8c.**) The visible chopping: the axe
  work clip (`woodcutter_chop.gr2`, 10.0 s long, three chops at 1.24, 2.88
  and 4.56 s) plays at the animation speed 1.8 (`GFX-animations.md`
  GFX-ANIM-3), so one loop takes 5.56 s and the 5.5 s work action shows
  **three swings** (then the pickup animation). The worker stands on the field
  before the tree's field and, after the straight approach, one lattice step
  (half a field) short of the tree's position, facing it (TRN-FREE-4.2, 5.2).
  Confidence: high (E-eco-015; clip read
  from the `.gr2` file).
- **ECO-OUT-3.6 Harvest**, in the tick the work ends: the reservation ends,
  and
  - a tree is felled: it starts being removed (ECO-OUT-6.4);
  - a stone pile is replaced by the next smaller pile of its chain
    (`dp_nextDeposit`: 06 -> 05 -> ... -> 01); the last one (01, which has no
    next deposit) is removed. **A pile `stone_0N` gives N stones.** The
    smaller pile is a **new deposit**: the old one is deleted and the new one
    made on the same field at the same position (no draw, no offset), and it
    takes the next object number (`SIM-game-start.md` SIM-START-4.2) in the
    tick the work ends, before the stone's own number at the pick-up
    (measured 2026-10-25 evening, E-trace-010: pile 89900 on (143, 54) became
    90524 at counter 3589, stone 90525 at 3593; 90524 became 90576 at 4129,
    stone 90577 at 4133; E-trn-015).
- **ECO-OUT-3.7 Take the ware**: drop time then pickup time (4 + 4 ticks). The
  first half plays PutDown, the second Pickup (`GFX-animations.md` GFX-ANIM-3.8;
  measured 2026-10-25 evening, E-trace-010: woodcutter 90392 PutDown from trace
  tick 2311, Pickup from 2315; hunter 90379 PutDown 2199, Pickup 2203), not
  Pickup twice.
- **ECO-OUT-3.8 Walk back** to the building's flag: first a straight
  lattice walk back to the point of the field where his walk to the target
  stopped (ordered in the tick the take ends, 3.7; **one tick later, after a
  standing tick, when that field is the building's flag**, i.e. the target was
  next to the flag and his free walk to it had no field: measured 2026-10-25
  evening over every take at a spot in E-trace-010, 3 of 3 such cases (woodcutter
  90392 at trace tick 2826, woodcutter 90478 at 3506 and 4001) against 25 of 25 at
  once otherwise; after that standing tick a worker standing exactly on his
  field's point walks to the flag from that field at once, without the
  lattice walk: 90478 at 4001, `TRN-free-walk.md` TRN-FREE-5.6 step 3, T27), then the free walk (TRN-FREE-2 to 4) from that field to the flag
  itself (`TRN-free-walk.md` TRN-FREE-5.6; **corrected Run D, E-gfx-015: the
  walk does not start from the field his stand sample truncates to**). If no
  route is found the worker waits and tries again.
- **ECO-OUT-3.8a The place at the flag is asked for on the way (2026-10-27 b,
  T23, T24).** A worker carrying a ware makes the entry check of
  `TRN-carriers-transport.md` TRN-CARRY-4.4 for his building's flag (bar, then
  admission; the ware has no destination, so by TRN-CARRY-4.2 it may take a 6th
  place; at a full flag its wait score grows as a carrier's ware's does):
  1. **once when his walk back starts**, in the tick the take ends (3.7), also when
     the lattice walk is skipped (the standing tick of 3.8 when F is the flag field);
  2. at the lattice walk's end, if the free walk to the flag is **a single step** (his
     field neighbours the flag) and he is not yet admitted: once in that tick, and
     the walk is ordered only when a check admits him; while refused he stands and
     checks again in each following tick (this wait is the standing tick of 3.8);
  3. a longer free walk is ordered at once without a check, and while he walks with
     the ware not yet admitted he checks in every tick from the next (refused: he
     does not step onto the flag).
  When admitted, his place is reserved from that tick (TRN-CARRY-4.3: it counts for
  the room rules and the route costs of other wares) and on his arrival he puts the
  ware down at once (3.9 without the wait). Because the first check raises the bar,
  a ware bidding there next with the same score is admitted at its first check (and
  resets the bar), and the worker's own later check then raises it again (T24).
  Confidence: high (code E-eco-028; measured T23, T24; was: the first check in the tick the free
  walk is ordered, which agrees with 1 and 3 when nothing else bids).
- **ECO-OUT-3.9 Put the ware down** at the flag as in ECO-WORK-3.6 (wait for
  room, drop time, pickup time; 4 + 4 ticks). As at the spot (3.7), the drop half
  plays PutDown and the pickup half Pickup (measured 2026-10-25 evening, E-trace-010:
  hunter 90379 on his flag (141, 48), PutDown from trace tick 2343, Pickup from 2347,
  walking in from 2351; GFX-ANIM-3.8 codes 2 and 3). The ware then belongs to the
  road network (`ECO-requests-dispatch.md` ECO-REQ-5).
- **ECO-OUT-3.10 Walk in** (one field, 23 ticks), then **rest** 223 ticks
  (ECO-WORK-3.9). Then ECO-OUT-3.1 again, **in the tick the rest is over**: that
  tick is already the first tick of the wait of 3.2, so he walks out 51 ticks
  later; no tick lies between the rest and the wait (measured 2026-10-25 evening,
  E-trace-010: woodcutter 90392 stepping in at trace tick 2419, waiting (job state
  33) from 2643, walking out 2694; hunter 90379 waiting from 2613, out 2664; an
  experiment build with a tick between them left both a tick late: dense settlers
  2664 -> 2705, generator 2687 -> 3095 when corrected). The first trip after the
  worker's arrival from the warehouse starts its wait in the tick after the
  arrival (hunter 90379: arrived 1853, waiting 1854, out 1905).

- **ECO-OUT-3.11 Productivity.** Every building that produces has a
  productivity value. When it produces for the first time it is set to 0.5;
  each later product adds 0.25, up to 1.0. Every 600 ticks (a counter shared by
  all buildings) the value of every finished building falls by 0.05, not below
  0. (Message repeats: ECO-REQ-3.8.) The value is a float32 and each step is a
  float32 addition, so the rounding accumulates: 1.0 after 18 falls, 0.75 after
  13 and 0.5 after 8 are just below 0.1, which matters for the tests against 0.1
  (`AI-military.md` AI-MIL-6.2, T24; added 2026-10-28 night).
  **Corrected 2026-10-28 (E-eco-031; measured T26):** the value is one entry per building in the
  game's statistics, and the entry is made **when the building's site is
  placed**, at 0.5, for every type with a product (`bp_setGoods`: the 21
  workshops, mines, outdoor huts, farm, spring, donkey farm and shipyard), and
  again when a conquered building is made anew. A product then adds 0.25 (cap
  1.0), so a building's first product takes it from 0.5 to 0.75, not to 0.5.
  A type without a product (the forester's hut) has no entry until its first
  planting, which sets 0.5 (ECO-OUT-4.7b). A building without an entry reads
  -1. The 600-tick fall skips buildings still under construction (frame or
  main part left), so a site keeps its 0.5 until it is finished; the entry of
  a removed building is dropped at the next fall. The same value is what the
  computer player's needs construction averages (`AI-construction.md` AI-CON-4,
  T67), what the building's label, window and Sleep icon show, and what the
  "No resources found." test of ECO-OUT-3.2 reads (below 0.5): with the
  entry made at 0.5 when the site is placed, that message can come only after
  a fall below 0.5 (code, not observed: medium for the message's timing).

### ECO-OUT-4 Forester cycle

- **ECO-OUT-4.1 Start.** As ECO-OUT-3.1.
- **ECO-OUT-4.2 Look for a spot** (inside the building). A countdown starts at
  50 and falls by 1 per tick; at 0 the search below runs. If the search does not
  finish (ECO-OUT-4.4), the countdown is set to 50 again. **First search 51 ticks
  after the step starts, then every 51 ticks.**
- **ECO-OUT-4.3 Spot search.** A flood from the building's field as in
  ECO-OUT-2.3, but a field is entered only if it is walkable (the deposit rule
  does not apply). Each entered field (the building's field included) is a
  **candidate** if:
  - its distance to the building field is at most the working radius (6);
  - no flag stands on it;
  - none of the bits Blocked (0), Street (1), Water (2), Coast (3),
    ElevationBlocked (6), DepositBlocked (7), MovementBlocked (8), DoodadBlocked
    (10) is set on it;
  - its terrain is neither for mining (`pa_setForMining`) nor harbour ground;
  - (grain fields only: its terrain is for planting, `pa_setForPlanting`.
    Trees do not need it, so a forester also plants on sand and earth);
  - neither it nor any of its six neighbours has the Blocked bit (0), so not
    next to a building or blocked terrain.
- **ECO-OUT-4.4** The search **finishes** when 11 candidates have been
  collected, or when a field is entered whose distance is more than radius + 4.
  If the queue runs empty first, the search has not finished (ECO-OUT-4.2
  repeats). When it finishes:
  - with candidates: one of them is chosen at random, each equally likely,
    with the game's random number generator (ECO-OUT-4.9);
  - without candidates: no spot. The forester starts again at ECO-OUT-4.1
    without resting (so the next search is 51 ticks later).
- **ECO-OUT-4.9 Random draws** (Q-116): all random choices of the logic use
  one generator, the Lehmer generator of `FMT-rc00-container.md` (multiplier
  16807, modulus 2^31 - 1). A choice among `n` takes one draw `x` and uses
  `x mod n`. A planting draws, in this order: the spot (candidate index), the
  tree kind (ECO-OUT-4.5), the tree's position offset (TRN-FREE-5.4). Other
  draws interleave (free-walk route ties, TRN-FREE-3.2; every deposit
  creation); matching the original sequence would need all of them in the
  original order and the game's seed (seeding not traced).
- **ECO-OUT-4.5 Sapling.** On the chosen spot a tree is created at once,
  reserved by the forester's building. Its kind is chosen at random from the
  tree family of the spot's terrain (`pa_setTreeType`):

  | Tree type | Kinds, each equally likely |
  |---|---|
  | 0 CaribbeanFlair | AfricanA, OliveA, PalmA, CypressA |
  | 1 ConiferousForest | BirchA, BirchB, BirchC, BroadLeafA, BroadLeafB, BroadLeafC |
  | 2 BlackForest | FirA, FirB |
  | 3 Lava | BroadLeafB, FirB (not the lava trees) |
  | other | BirchA |

  The new tree is not ripe and does not grow yet (no size).
- **ECO-OUT-4.6 Walk out** to the flag (23 ticks), then **walk to the spot** as
  in ECO-OUT-3.4.
- **ECO-OUT-4.7 Plant.** A timed action lasting the production time: **55
  ticks**. When it ends, the tree starts growing at size 0.1 (ECO-OUT-6), and
  the reservation ends. (A deposit kind with underground patterns, the grain
  field, also changes the field's terrain to one of them at this moment; trees
  have none.)
- **ECO-OUT-4.7b Productivity (play test 11; E-ui-011).** The forester's hut has a productivity like every producing
  building (ECO-OUT-3.11) although it makes no ware: **in the tick the
  planting action ends** (ECO-OUT-4.7), if the planting step still has its
  spot, the hut's productivity is raised, 0.5 at the first planting ever and
  +0.25 at each later one, capped at 1.0; it decays by 0.05 every 600 ticks
  like the others and reads as -1 before the first planting. A planting step
  that ends without a spot (none found, ECO-OUT-4.4) adds nothing and starts
  the step again. The value is what the construction-view label, the hover
  tooltip and the building window show (`UI-ingame.md` UI-GAME-9.1, 4.1) and
  what the `Sleep` icon tests (`GFX-objects.md` GFX-OBJ-8.3). Confidence:
  high.
- **ECO-OUT-4.8 Walk back** to the flag over the terrain, **walk in** (23
  ticks), **rest** 223 ticks. Then ECO-OUT-4.1. The forester carries nothing and
  puts nothing down at the flag.

- **ECO-OUT-4.5a Creating the sapling (draw).** Placing the new deposit draws once
  from the simulation generator for its lattice offset (`draw mod 6`, one sample
  step; E-trn-003), after the spot and kind draws of 4.2 to 4.5. Every deposit
  created during a game (saplings, sown fields) costs this one draw. Confidence:
  medium (code).
### ECO-OUT-5 Totals

- **ECO-OUT-5.1** Woodcutter or stonemason, target already reserved, level
  ground, room at the flag. With `W1` the walk from the flag to the target
  including the fine approach (ECO-OUT-3.4) and `W2` the walk back
  (ECO-OUT-3.8), counting from the start of
  ECO-OUT-3.2:

  | Ticks from start | Event |
  |---|---|
  | 51 | target check succeeds, worker leaves |
  | 51 + 23 | at the flag |
  | 74 + W1 | at the target, work starts |
  | 129 + W1 | work ends, tree felled / pile shrinks |
  | 137 + W1 | log or stone taken |
  | 137 + W1 + W2 | at the flag |
  | about 142 + W1 + W2 | ware lies at the flag (1 tick, then the drop time, 4) |
  | about 146 + W1 + W2 | pickup time over (4), worker turns back |
  | about 169 + W1 + W2 | back inside; rest begins 1 tick later |
  | about 393 + W1 + W2 | rest over (223 ticks); the next cycle starts in this tick, its first waiting tick (ECO-OUT-3.10) |

  The put-down is exactly that of ECO-WORK-3.6 (the same step is used), so the
  ware lies at the flag when the drop action ends (Q-115). So one log or stone
  every **about 393 + W1 + W2 ticks** (corrected 2026-10-27 night from 394: no tick between the rest and the next cycle, ECO-OUT-3.10 and 5.4); for a tree 2 fields from the flag on
  level ground (route of 2 fields, the worker stops on the middle field: W1 =
  23 for that field plus 23, 34 or 45 for the 2, 3 or 4 lattice steps of fine
  approach, depending on the tree's offset, TRN-FREE-5.4; W2 = 23 from the
  middle field back to the flag, Q-045) about 462, 473 or 484 ticks. The one-tick steps are
  medium confidence (ECO-WORK-4.1).
- **ECO-OUT-5.2** Forester, same conditions, spot search finishing at once:
  51 (search) + 23 + W1 + 55 (plant) + W2 + 23 + 1 + 223; about
  376 + W1 + W2 ticks per tree (corrected 2026-10-27 night: the next cycle
  starts in the tick the rest is over, ECO-OUT-3.10; medium, no forester in
  the recordings).
- **ECO-OUT-5.3** Because the building searches every 51 ticks and the worker
  checks every 51 ticks, a worker who finds no target at his check waits for
  the next building search and then for his next check: up to 51 + 51 ticks
  after a tree becomes ripe.

- **ECO-OUT-5.4 Door to door.** The time the woodcutter (or stonemason)
  spends inside between two trips does not depend on the target: step in 1 +
  rest 223 + target check wait 51 = **275 ticks = 27.5 s** (the door closes
  behind him and opens again 27.5 s later; the wait begins in the rest's last
  tick, ECO-OUT-3.10, measured: woodcutter 90392 in at trace tick 2419, out at
  2694; corrected 2026-10-25 evening from 276). The trip outside lasts 118 + W1 +
  W2 ticks (out 23, work 55, take 8, put down 9, in 23, plus the two walks;
  each of W1 and W2 includes its lattice part: the fine approach in W1 and
  the fine walk back in W2, which has the same number of sample steps,
  TRN-FREE-5.6).
  Play-test check: Jens timed about 27 s door to door. Confidence: high.

### ECO-OUT-6 Tree growth

- **ECO-OUT-6.1** A deposit has a size. Trees and stones placed with the map
  have size 1.0 (ripe from the start). A sapling has no size until planting
  ends (ECO-OUT-4.7), then size = growing start size (0.1 for trees).
- **ECO-OUT-6.2** Every tick, every growing deposit whose size is at least its
  start size and below 1.0 gains the growing speed (0.0009 for trees), computed
  in 32-bit floating point. From 0.1 this takes **1001 ticks** to reach 1.0 or
  more (100.1 s); the tree is then ripe and can be found by ECO-OUT-2.
- **ECO-OUT-6.3** There are no growth stages in the logic: the size grows
  continuously. Any visible stages are drawn from the size (not traced).
- **ECO-OUT-6.4** Removal. A felled tree (or the last stone of a pile) is
  marked as being removed; a counter starts at 1 and goes up by 1 each tick;
  when it reaches 20 the object is deleted and its field is free again. The
  settlers are updated before the resources in each tick, so the counter
  already reaches 2 in the harvest tick T and the object is deleted in tick
  **T + 18**. While being removed it is not ripe.
- **ECO-OUT-6.5** Lifetime (grain fields only, `dp_setLifeTime`, 3000 for
  `field_01`): counted down by 1 each tick while the deposit is not reserved;
  at 0 it starts being removed. Trees and stones have no lifetime and never
  wither.

### ECO-OUT-7 Fisher cycle (new, medium)

- **ECO-OUT-7.1** Steps, repeated: **choose a spot** at once (no 51-tick wait):
  a random one of the Coast fields within the building's working radius that
  have fish within the fishing distance (ECO-FOOD-4.2); with
  none the message "No resources found." is given and the step fails (retried
  when the tick count modulo 100 is 0, i.e. every 100 ticks); **walk out** (23);
  **walk to the spot** (he stands on the Coast field itself, ECO-FOOD-4.4;
  free walk); **fish** for the building's production time, **333
  ticks**; in the tick it ends one fish is taken from the water field's stock
  (none left: the hut's fish field is forgotten and the unit is taken from the
  first fish field around him instead, ECO-FOOD-4.5); **take** 4 + 4 (always:
  the fish is made in every case, ECO-FOOD-4.6); walk back to the flag, **put down** 1 + 4 + 4,
  walk in 23, **rest 1111 + 1**. So one fish per about 1510 + W1 + W2 ticks (151 s
  plus the walks). Confidence: high (E-eco-015; E-pt2-001).

### ECO-OUT-8 Hunter cycle (new, medium)

- **ECO-OUT-8.1** Steps, repeated: **wait for a reserved animal** (countdown
  50, first check after 51 ticks, then every 51); **walk out** (23); **walk to
  the animal**: the free walk to the animal's field stopping **one** field
  short (`TRN-free-walk.md` TRN-FREE-4.2), then, in the tick it ends, a fine
  walk on the lattice towards the animal's point stopping **2 lattice steps**
  short (TRN-FREE-5.1, 5.3; corrected 2026-10-25 evening, measured T18, code
  E-eco-017: not "two fields short"); when that walk ends
  the animal is checked (if it is gone the trip ends and he returns with a ham
  all the same, ECO-FOOD-5.4); **shoot**: the animal starts its
  dying animation at once and the hunter works **55 ticks** on the spot (the
  carcass becomes meat when the work ends); **take** 4 + 4; walk back; **put
  down** 1 + 4 + 4; walk in 23; **rest 222 + 1**. Inside time door to door: 275
  ticks as the woodcutter's (ECO-OUT-3.10, 5.4; measured on hunter 90379: waiting
  from trace tick 2613, out 2664). Confidence: medium (E-eco-015).
  **Which field (2026-10-27 b, measured T25; corrected 2026-11-01, T27):** the
  free walk aims at the reserved animal's **walk goal when that walk starts**:
  the field its current walk ends on, so its own field while it stands (T25)
  and, while it walks, the last field of that walk, not the field it is on (an
  animal one step from its goal at the walk's start is aimed at on the goal,
  T27). Not the field it had when the hut reserved it. The lattice walk that
  follows goes towards the animal's current position (the start field of its
  current step) when the free walk ends. Confidence: high (E-eco-041:
  the free walk to the animal's goal field; the lattice walk to its
  position field; measured T25, T27).

### ECO-OUT-9 Farmer cycle (new, medium)

- **ECO-OUT-9.1** The farmer decides at each turn: a ripe grain field reserved
  by the building -> **harvest**: first the woodcutter's **target wait** of
  ECO-OUT-3.2 (the decision tick is its first tick; he leaves 51 ticks later when
  the building still holds a reserved field, else he waits and checks again as
  the woodcutter does), then the woodcutter's trip: out, walk to the
  field, work **83 ticks**, take, back, put down, in, rest 55 + 1 (the wait
  added 2026-10-27 night, E-eco-022: the decision state starts the
  target step, the same job state as the hunter's and woodcutter's wait, and
  the trip only when that step succeeds; measured, T21); otherwise
  **sow**: a spot search (as the forester's), the new grain field is created
  the moment the spot is chosen (before he leaves the hut), walk out, walk to
  the spot and approach, work **83 ticks** (sowing animation), then he walks
  back to the flag, in, and rests (as after a harvest); when the sowing step
  finds no free spot he decides again at once, still inside (the step itself
  waits 50 ticks before each search, ECO-FOOD-3.2a). Confidence: high
  (E-eco-015; re-read 2026-10-17, E-pt2-001:
  the earlier text had the two outcomes swapped).
  **When the sowing ends** the planting step makes the field's deposit once
  more (making the deposit draws the lattice direction of
  `TRN-free-walk.md` TRN-FREE-5.4): **one draw** from the game's generator in
  that tick, in the workers' part, before the animals' draws; no object number
  is taken and the field keeps its id (added 2026-10-27 night, E-eco-021; measured T19: confidence high for the draw, medium for
  what it belongs to). The forester's planting runs through the same step (its
  tree kind is drawn there as well); not measured.

### ECO-OUT-10 Door-to-door summary

| Worker | Inside between trips (ticks) | Outside per trip (ticks) | Work action | Spec |
|---|---|---|---|---|
| workshop / mine worker | never leaves the flag area: cycle 455 (sawmill) | 55 walk + 17 handling per product | 333 | ECO-WORK-4 |
| woodcutter, stonemason | 275 (step in 1 + rest 223 + check 51) | 118 + W1 + W2 | 55 | ECO-OUT-5.1, 5.4 |
| forester | 275 (medium: no forester in the recordings; its cycle starts as ECO-OUT-3.1) | 101 + W1 + W2 (no take / put down) | 55 | ECO-OUT-5.2 |
| hunter | 275 (measured, hunter 90379) | 118 + W1 + W2 | 55 | ECO-OUT-8 |
| fisher | 1114 (rest 1112 + 2) | 118 + W1 + W2 | 333 | ECO-OUT-7 |
| farmer | 57 after a harvest or a sowing (step in 1 + rest 56; the decision in the rest-over tick, ECO-FOOD-3.2a), + 51 for the target wait before a harvest or per spot search before a sowing (measured: 108 with one search, T20; 108 before a harvest, T21; corrected 2026-10-27 night from 58) | 118 + W1 + W2 (harvest), 129 + W1 + W2 (sow: no take / put down) | 83 | ECO-OUT-9 |

Walks: 23 ticks per level field (TRN-MOVE-1.3), fine approach 12 / 23 / 34 / 45
ticks for 1 .. 4 lattice steps (TRN-FREE-5.3).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| building target search period | 51 (the counter gains 2 a tick) | ticks | high | E-eco-007; E-eco-024; T22 |
| worker target check: first / period | 51 / 51 | ticks | high | E-eco-007 |
| forester spot search: first / period | 51 / 51 | ticks | high | E-eco-007 |
| search stop distance | radius + 4 | fields | high | E-eco-007 |
| forester candidates collected | 11 | fields | high | E-eco-007 |
| working radius woodcutter / stone pit / forester / default | 6 / 8 / 6 / 4 | fields | high | `buildings.lua`, E-eco-007 |
| work / plant action | production time, 55 | ticks | high | E-eco-007 |
| tree falling animation length | production time + 5 | ticks | medium | E-eco-007 |
| take and put down | 4 + 4 each | ticks | high | E-eco-007, ECO-WORK |
| rest | 223 | ticks | high | `Jobs.lua`, ECO-WORK-3.9 |
| tree start size / speed | 0.1 / 0.0009 | size, size per tick | high | E-eco-007 |
| tree growth time | 1001 | ticks | high (float32 count) | E-eco-007 |
| removal counter | 1 -> 20 | ticks | medium | E-eco-007 |
| stones per pile `stone_0N` | N | | high | `deposits.lua`, E-eco-007 |
| tree family table | ECO-OUT-4.5 | | high | E-eco-007, `patterns.lua` |
| neighbour order in floods | 5, 0, 1, 2, 3, 4 | directions | high | E-eco-007 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | woodcutter hut, ripe trees at distance 2 and 3 from the hut, nothing else in radius 6 | target search | the tree at distance 2 (first in flood order) is reserved |
| T2 | as T1 but two ripe trees both at flood depth 1, in directions 0 and 5 of the hut | target search | the tree in direction 5 (ring order 5, 0, 1, 2, 3, 4) |
| T3 | only ripe tree at distance 7 | search | none (radius 6); worker checks every 51 ticks; "No resources found." when productivity < 50 % |
| T4 | ripe tree reserved by hut A, hut B also in range | B's search | B does not take A's tree |
| T5 | new woodcutter, worker just entered, target reserved | time to leaving | 51 ticks after the cycle starts |
| T6 | woodcutter at the tree | work | 55 ticks; then the tree is no longer ripe; the field is free 18 ticks later |
| T7 | stone pit next to `stone_06` only | harvest repeatedly | 6 stones, the pile shrinking 06 -> 05 -> ... -> 01 -> gone |
| T8 | forester on meadow (tree type 1) | plant | sapling is one of BirchA/B/C, BroadLeafA/B/C |
| T9 | forester on land whose terrain has tree type 3 (lava) | plant | BroadLeafB or FirB, never a lava tree |
| T10 | sapling planted (plant action ends at tick 0) | growth | not ripe at tick 1000, ripe at tick 1001 |
| T11 | forester with every field within 6 blocked or built on | search | no spot; next search 51 ticks later; no rest taken |
| T12 | forester whose neighbourhood has more than 11 candidates | many plantings | spots are always among the 11 first candidates in flood order (nearest), chosen at random |
| T13 | field next to a building (Blocked) | forester search | never a candidate |
| T14 | woodcutter cycle, tree with W1 = W2 known | time between two logs at the flag | 393 + W1 + W2 ticks (medium) |
| T15 | farmer sows a field (spot found at the first search) | timeline | 51 inside, out 23, walk, 83 sowing, walk back, in 23, rest 56, decide (ECO-OUT-9.1, ECO-FOOD-3.2a) |
| T16 | new forester, plantings end in ticks 400, 950, 1500, 2050; the 600-tick decay fires at 600, 1200, 1800, 2400 | productivity | -1 until 399; 0.5 at 400; 0.45 at 600; 0.7 at 950; 0.65 at 1200; 0.9 at 1500; 0.85 at 1800; 1.0 at 2050 (capped); 0.95 at 2400. Label "50%" .. "95%" (ECO-OUT-4.7b, 3.11) |
| T17 | forester whose searches never find a spot | productivity | stays -1 for ever (the construction-view label shows it clamped at 0, "0%", UI-GAME-9.2; the hover tooltip shows the name alone, UI-GAME-9.3); no "No resources found" message from this path, the forester has no resource check |
| T18 | ECO-OUT-8.1, TRN-FREE-5.1: E-trace-010 (two-player game), hunter 90379 of the hut (140, 49), the deer 90060 reserved and standing on (140, 54) | the walk to the animal | free walk ending on (140, 53) (one field short) in trace tick 2121, on its point (562000, 212000); fine walk of 2 lattice steps towards (140, 54)'s point (560000, 216000), stopping at (561000, 214000) in 2144 (23 ticks, level); WorkLoop0 from 2144 for 55 ticks; PutDown 2199 .. 2202, Pickup 2203 .. 2206; the fine walk back ordered in 2207 (walk clip from 0, no position change), on (140, 53)'s point in 2230. The field column follows the samples (MAP-GRID-4.3): (139, 53) from 2133 |
| T19 | ECO-OUT-9.1 (the draw when sowing ends): E-trace-011, player 1's farmer 91017 of the farm 90538 (135, 59), his first sowing: grain field 91046 on (137, 60) made at counter 9978, the sowing animation from trace tick 10117, ending in counter 10200 | the generator in trace tick 10201 | 17 draws in that tick: the sowing's one, then five animals choosing targets in id order with their walks (the first, 90031 on (134, 34), takes the 2nd draw: n = 14 of its 18, target (131, 33); 90166 on (57, 80) the 17th: n = 4 of 60, target (56, 81), a one-step walk without draws); the generator's state after the tick 2054550843. Without the sowing's draw an experiment build gives every animal the previous draw (targets (132, 34) .. (55, 84), 18 draws); with it the generator equals the recording to trace tick 10312 (was 10200) |
| T20 | ECO-OUT-9.1, 10, `ECO-food-mining.md` ECO-FOOD-3.2a: E-trace-011, player 1's farmer 91017 coming back from his first sowing (field (137, 60), sowing over at counter 10200) | his return and next decision | he stands inside on (135, 59) from trace tick 10326 (the step-in tick), rests 56 ticks (10327 .. 10382; the recording's job state changes to the decision state in 10382 and to the sowing step's wait in 10383), the sowing step's wait begins in 10383, the tick the rest is over, and he walks out at 10434 (step in 1 + rest 56 + wait 51 = 108 ticks from 10326); the generator makes the walk-out's two draws in 10434. A tick between the rest and the decision (an experiment build) walks him out at 10435: settlers first differ 10435 -> 11065 and the generator 10434 -> 11116 when corrected |
| T21 | ECO-OUT-9.1 (harvest), 10: E-trace-011, player 1's farmer 91017 back from his second sowing (in on (135, 59) at trace tick 11292), the field sown first (137, 60) ripe and reserved by the farm | his decision and the harvest trip | rest 11293 .. 11348, the decision in 11349 starts the target wait (the recording's job state 33, as the hunter's wait), he walks out at 11400 (51 later) towards the field ((137, 58) at 11469, (137, 59) at 11505). Walking out in the decision tick (an experiment build) leaves at 11349: settlers first differ at 11350 and the generator at 11395; with the wait 11449 and 11453 |
| T22 | ECO-OUT-2.1, `ECO-food-mining.md` ECO-FOOD-3.2a: E-trace-011, player 1's farm 90751 on (139, 44), finished at counter 11100; its grain field 91225 on (138, 42) grows 0.00075 a tick (0.9560053 at 12799), ripe from about 12858; farmer 91200 back from sowing, in at 12829 | his decision at 12886 | the farm searched at 11100 + 51 k: at 12834 (not ripe) and 12885 (reserves 91225), so he decides on the harvest and walks out at 12937 (recorded: the target wait state from 12886, field reserved by 90751 in the deposits table of 12899). Searching every 101 ticks (12817, 12918; an experiment build) makes him sow: settlers first differ at 12938 and the 150 000-tick generator at 12983; with 51 ticks 13521 and 14469 |
| T23 | 3.8a: E-trace-011, player 1, woodcutter 90392 of the hut whose flag is (142, 55); wood 91589 cut at (146, 57), taken at counter 16971; his lattice walk back from trace tick 16976, his free walk to (142, 55) ordered at counter 17009; the flag holds 5 lying wares | his place at the flag | the wood is admitted to the flag's list at counter 17010 (wares table: flag 90271, not lying, score 0), so the flag counts 6 from then; he arrives at 17122 (trace) and puts the wood down at once (PutDown 17123). The 6 make the flag full for the route of wood 91586 put down at (141, 58) at 17052, which goes over (141, 56) (next flag) and not over (142, 55); asking only on arrival (an experiment build) gives it (142, 55) and its road's carrier 91502 fetches it at 17059 (settlers first differ at 17059; with the rule at 17533) |
| T24 | 3.8a: E-trace-011, player 1's farm (137, 56), flag (136, 57) (flag 90540), farmer 91017 carrying grain back; (a) grain 91253, lattice walk back ending at counter 12104 on (136, 56), a neighbour of the flag; (b) grain 91764, lattice walk from 18559 ending at counter 18593 on (137, 58), a neighbour of the flag; carrier 90802 brings wood 91638 to the flag, his walk from (132, 58) ordered at counter 18589 | the admissions (wares table) and the farmer's free walk | (a) the bar raised by his check when the walk back started; at 12104 his check admits the grain and the one-step walk is ordered in that tick (trace 12105 Walk at the point, moving from 12106). (b) the bar raised at 18559; 90802's first check at 18590 finds it equal to its score 0 and admits wood 91638 (bar back to -1); the farmer's check of 18593 raises it again, he stands (trace 18594), his check of 18594 admits the grain and the walk is ordered (trace 18595 Walk 0). E-trace-010 woodcutter 90478 (F the flag field, counter 4000): the check at the take's end raises the bar, the next tick's admits and he walks (the measured standing tick of 3.8). An experiment build without check 1 and 2 orders the farmer's walk in 18593 (settlers, view columns left out, first differ at 18595, generator 18826; with the rule 18620 and 18992) |
| T25 | 8.1 (which field): E-trace-012 / E-trace-013 (Hard), hunter 90463 of the hut (139, 47), animal 91025 (home (144, 37)) | the hunter's walk target | the animal walks (143, 38) -> (142, 39) -> (142, 40) from counter 7397, is reserved by the hut at 7422 while on (142, 39) and stands on (142, 40) from 7442; the hunter walks out at 7440, starts his free walk at 7463 from the flag (140, 46), ends it on (141, 41) (one field short of (142, 40)) at 7591, walks the lattice (566, 164) -> (566, 163) -> (567, 162) by 7614 and shoots from 7615. Aiming at (142, 39) (the field at the reservation) walks him onto (142, 40), the approach finds nothing and he takes a ham at 7636. Experiment build: Hard pair settlers (view-free) 7592 -> 7623, generator 7668 -> 8471; the other pairs unchanged |
| T26 | 3.11 (the productivity entries): E-trace-013 (Hard, player 1), the statistics entries in the heap dump of counter 10 000 (value, falls counted) | each building's value | farm 90720 0.4 (2 falls; built from its site of 4776, two sowings at 9453 and 9900, no harvest), brewery 90786 0.4 (2; no beer made), mill 90961 0.5 (0), building 90896 0.5 (0), woodcutters 91097 0.65 (2; one product), 90986 0.9 (2), forester 90954 0.7 (1: first planting 9100 sets 0.5, fall 9600, planting 9732), sites 91254, 91283, 91310 0.5 (0), sawmills, smithy, spring 1.0, stone pits 0.95; no entry for the castle, harbour, tower and catapult. Experiment build (0.5 at placement for types with a product, sowing not counted): every value above equal; with AI-CON T67 the Hard pair's buildings 10175 -> 10278 |
| T27 | 8.1 (the animal's walk goal): E-trace-013 (R3, the Hard game), hunter 95536 of the hut at (127, 26) (flag (127, 25)), animal 90047 | the hunter's walk | the animal walks (120, 28) -> (120, 27) -> (121, 26), its walk goal (121, 26), stepping onto (121, 26) from counter 42883 and registered there from 42905; the hunter's free walk starts at counter 42904 from the flag (127, 25), ends on (121, 25) (one field short of (121, 26)) at trace tick 43039, walks the lattice to (485000, 101080) by 43051 and the animal dies at 43061 (recorded). Aiming at the field the animal stands on at the walk's start, (120, 27), walks him onto (121, 26) (the Hard game vs R3 differs from settlers 43039). With the rule (experiment build, with TRN-CARRY T56 and AI-CON T97): settlers 43100 -> 43500, buildings 44700 -> 45400, generator 43124 -> 43426 (the pair's rows); every other pair unchanged |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-007 | code reading | `S2DNG.exe` | woodcutter/stonemason and forester state sequences |
| E-eco-007 | code reading | | outdoor work: wait for target, walk out, walk to target, work |
| E-eco-007 | code reading | | 50-tick countdown, target check, "No resources found." |
| E-eco-007 | code reading | | walk to the reserved target, fine approach by elevation size |
| E-eco-007 | code reading | | work action with production time; harvest / planting per profession |
| E-eco-007 | code reading | | building target search: period, flood, entry and goal tests |
| E-eco-007 | code reading | | reserve / release a deposit for a building |
| E-eco-007 | code reading | | forester: spot search, candidates, sapling |
| E-eco-007 | code reading | | tree kind by terrain tree type |
| E-eco-007 | code reading | | carry back, put down, walk in and rest |
| E-eco-007 | code reading | | deposit sizes at creation |
| E-eco-007 | code reading | | per-tick growth, removal counter, harvest and next deposit |
| E-eco-007 | script | `buildings.lua`, `Jobs.lua`, `deposits.lua`, `patterns.lua`, `Goods.lua` | radii, times, deposit chains, tree types, `gp_enableDeposits` |
| E-trn-002 | code reading | | productivity: +0.5 / +0.25 per product, -0.05 per 600 ticks, -1 when none |
| E-trn-002 | code reading | | settler update before resource update; felled tree enters the active list at once |
| E-trn-003 | code reading | | woodcutter put-down is the workshop put-down |
| E-trn-003 | code reading | | DepositBlocked at creation; dp_setBlocking -> Blocked; random offset flag |
| E-trn-003 | code reading | | one logic random generator |
| E-eco-015 | code reading | | worker state classes, fisher / hunter / farmer cycles, stand position, config block |
| E-eco-017 | code reading and recording | E-trace-010 settlers 90379, 90392 | ECO-OUT-8.1, 3.7, T18; TRN-FREE-5.1; GFX-ANIM-3.8e |
| E-pt2-001 | code reading | | fisher / hunter / farmer corrections |
| E-sim-001 | code reading | | ECO-OUT-4.5a |
| E-ui-011 | code reading | | ECO-OUT-4.7b (play test 11) |
| data | animation | `data/character/animations/woodcutter_chop.gr2` (10.0 s, 46 tracks), `settler_animations.xml` WorkLoop0Axe sounds at 1.24 / 2.88 / 4.56 s | three swings per work action |
| E-eco-021, E-trace-011 | code reading and recording | generator (G rows) and animals table of E-trace-011, trace ticks 10200 .. 10313 | ECO-OUT-9.1 draw (T19) |
| E-trace-011 | recording | `settlers.csv.gz` (farmer 91017, counters 10195 .. 10445) and the generator rows | ECO-OUT-10 farmer row, T20 |
| E-eco-022, E-trace-011 | code reading and recording | farmer 91017, trace ticks 11290 .. 11520 | ECO-OUT-9.1 (T21) |
| E-eco-024, E-trace-011 | code reading and recording | farm 90751, field 91225, farmer 91200 (buildings, deposits, settlers tables) | ECO-OUT-2.1 (T22) |
| E-eco-028, E-trace-011, E-trace-010 | code reading and recordings | wares tables (admissions of grains 91253, 91764, wood 91638, wood 91589) | ECO-OUT-3.8a (T23, T24) |
| E-eco-031, E-trace-013 | code reading and memory reads of the running game | the game's statistics entries in E-trace-013's heap dumps (each: building id, value, falls) | ECO-OUT-3.11 (T26), `ECO-food-mining.md` ECO-FOOD-1.2 |
| E-eco-041, E-trace-013 | code reading and recording | the hunter's two walks (the free walk to the animal's walk goal, then the lattice walk to its position); R3 `animals` (90047, counters 42880 .. 43061) | 8.1, T27 |

## Open questions

- Q-045: a tree's random offset within its field, and the timing of the first
  step back from an off-centre position (affects W1 and W2).
