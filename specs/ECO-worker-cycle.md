# ECO-WORK: Work cycle of a production building's worker

Status: draft (sequence and durations read from the binary; totals agree with two in-game measurements under the reading given in T6)
Owner question(s): Q-019, part of Q-005 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

A worker in a workshop or mine repeats one cycle: work for the building's
production time, pick the product up, carry it out to the building's flag, put
it down, walk back in, rest for his profession's rest time, then start again as
soon as the inputs are there. The production time is therefore only part of the
time between two products: for a sawmill it is 33.3 s of sawing in a cycle of
45.5 s.

## Rules

Times are in logic ticks of 0.1 s (`ECO-time-units.md`). Walking follows
`TRN-settler-movement.md`; directions follow `MAP-grid-geometry.md`.

### ECO-WORK-1 Scope

- **ECO-WORK-1.1** This cycle applies to the workers of: sawmill, mill, bakery,
  brewery, slaughterhouse, piggery, spring (well), smeltery, smithy,
  locksmithery, mint, and the coal, iron, gold and stone mines.
- **ECO-WORK-1.2** Farm, forester, woodcutter, stone pit, fisher, hunter, donkey
  breeding, ship yard, catapult and watch tower have their own cycles (they
  work outside the building or deliver differently). Not specified here;
  woodcutter, stone pit and forester are in `ECO-outdoor-workers.md`. Where the
  inputs come from: `ECO-requests-dispatch.md` ECO-REQ-4.2, 4.3.

### ECO-WORK-2 Timed actions

- **ECO-WORK-2.1** A settler has one action timer. Starting a timed action sets
  it to the action's duration. Each later tick it is reduced by 1; the action
  is over in the tick in which the timer reaches zero or less. A duration `D`
  therefore lasts `ceil(D)` ticks: 3.3 lasts 4 ticks, 55 lasts 55, 333 lasts
  333.
- **ECO-WORK-2.2** In each tick a settler first advances its timer, then its
  movement, then its job logic. What the job logic starts in tick N takes
  effect from tick N+1.

### ECO-WORK-3 The cycle

Let the building stand on field `F`. Its flag is on the neighbour of `F` in
direction 1 (south-east); the worker comes back in direction 4. The steps, in
order:

- **ECO-WORK-3.1 Wait for inputs.** The worker waits inside until the building
  is enabled and can produce (its inputs are in stock; for a mine, food and
  the deposit). Checked once per tick. No rest is taken or repeated while
  waiting.
- **ECO-WORK-3.1a The first check after arriving.** A worker who has just
  arrived in a building that works inside (a workshop, mine, donkey farm or
  shipyard) stands in the arrival tick and in one more tick (the first
  waiting tick, its job state now the building's); its first check for work
  (3.1) comes in the tick after that. So with its inputs in stock (or none
  needed) the work action starts **two ticks after the arrival tick**, as
  after a rest (3.10). Confidence: high for workshops (measured, E-trace-010:
  the spring's worker 90699 on (136, 53) arriving in trace tick 5558, standing
  in 5559, working from 5560, the water ware 90784 made at counter 5896 as
  recorded; the sawmill's worker 90431 standing in 2694, waiting in 2695;
  T-arrive-1); medium for mines, donkey farms and shipyards (not recorded).
  Outdoor workers start their first trip by `ECO-outdoor-workers.md`.
- **ECO-WORK-3.1b Production switched on again (2026-10-27 b, T11).** While the
  building's production is switched off the worker stands in a state of its own
  (not the input wait of 3.1). In the tick after it is switched on (the switch is
  made by a command or the computer player, at the end of a tick) he only returns
  to waiting; his first check (3.1) comes in the tick after that. So with inputs in
  stock the work action starts **two ticks after the switch**, one later than a
  plain input wait. Confidence: medium (one measured case, T11).
  **Only a worker who comes to his check while it is off enters that state**
  (added 2026-11-27, E-eco-061: E-trace-038, E-trace-011 and an experiment build;
  medium, one case each): after his rest, on arriving, or back from carrying out
  his product. A worker already in his input wait (3.1) when production is
  switched off stays in it; his checks fail while it is off, and in the tick
  after the switch-on his check starts the work, **one tick after the switch**
  (T12). T11's worker had come back in after the switch-off.
- **ECO-WORK-3.2 Work.** A timed action lasting the building's production time
  (`bp_setProductionTime`). This is the visible work animation. In the tick it
  ends, the building consumes its inputs and the product exists.
- **ECO-WORK-3.3 Take the product.** Two timed actions one after the other: one
  lasting the global drop time, then one lasting the global pickup time
  (4 + 4 ticks at the shipped 3.3 and 3.3).
- **ECO-WORK-3.4 Wait for the flag.** From the next tick on, checked once per
  tick: the worker leaves when the flag can take the ware. With room at the
  flag this costs **2 ticks**: the worker stands in the two ticks after the
  take's last tick, and in the third the ware's place at the flag is granted
  and the walk out starts at progress 0 (corrected 2026-10-25 evening, measured
  E-trace-010: the sawmill's worker 90431, take ending at counter 3508, standing
  3509 and 3510, the plank's place at its flag and the walk in 3511; his second
  plank the same, 3965 / 3967; earlier read as 1 tick, never measured: these are
  the game's first products carried out of a workshop).
  **The checks are the entry check of `TRN-carriers-transport.md` TRN-CARRY-4.4**
  for the product (as for a warehouse's helper, `ECO-requests-dispatch.md`
  ECO-REQ-5.3c): the one in the second standing tick raises the flag's bar to
  the ware's wait score and the one in the third admits it. **The second
  tick's check is a whole entry check**: when the bar already holds the ware's
  score (another ware's bid set it), it admits the ware at once and the walk
  out starts in that tick (added 2026-10-28 night, E-trn-021;
  measured T-room-2). At a full flag each
  check fails and adds to the score (15 - transport priority); when room comes,
  the first check that finds it raises the bar and the next one admits the ware,
  so the worker leaves one tick after the first check with room, not in it, and
  a ware leaving the flag in the same tick resets the bar again (added
  2026-10-27 night, measured T-room-1). Confidence: high.
- **ECO-WORK-3.5 Walk out** from `F` to the flag: one field (23 ticks on level
  ground; more if the flag is higher than the building, TRN-MOVE-1.3).
- **ECO-WORK-3.6 Put the product down.** One tick after arriving, a timed action
  lasting the drop time; when it ends the ware lies at the flag and belongs to
  the road network. Then a timed action lasting the pickup time. (4 + 4 ticks.)
  If the flag turned out to be full, the worker waits at the flag first.
- **ECO-WORK-3.7 Walk back** from the flag to `F`: one field.
- **ECO-WORK-3.8 Step inside.** One tick. (A building type with a door offset
  direction makes the worker walk one more field here, in that direction, and
  the matching field on the way out; none of the buildings of ECO-WORK-1.1
  except the piggery of one tribe has one.) **Door offsets of other workplaces
  (2026-10-27 night, T-door-4):** the farm and the donkey breeding (every tribe,
  north-west) and the Africans' piggery have one, so their workers, the farmer
  of the outdoor cycle included, enter and leave by the door field as 3.8a and
  3.8b say for every settler: arriving at the building (from the warehouse, and
  back from every trip), the walk ends on the building's field and a step of
  its own (from progress 0, 23 ticks on level ground) takes the worker to the
  door field, where it waits; going out, a step of its own from the door field
  onto the building's field comes first, then the walk to the flag from
  progress 0. Confidence: high (measured, T-door-4).
  - **ECO-WORK-3.8a Door field.** The door field of a building with a door offset
    is its field stepped once in that direction (the headquarters: direction 4,
    MAP-GRID-1.2, so (x, y+1) for odd y). Every settler that leaves such a building
    appears on the door field and starts its walk from there, and a settler that
    stands at the building (a present soldier, MIL-SOL-6.5) stands on it. A
    building without a door offset has its own field as the door field.
    Confidence: high (measured, live trace 2: the computer player's first
    constructor, leaving the castle at (139, 51), is at (139, 52) fine (556090,
    207819) in its first walking tick, trace tick 178: one step into direction 1, back
    onto the castle's field, then on to the flag at (140, 50); the garrison soldiers
    leave the same way).
  - **ECO-WORK-3.8b The door step is a walk of its own (Q-244).** A settler that
    leaves a building by its door field walks the one step from the door field onto
    the building's field as a walk by itself (from progress 0, TRN-MOVE-2.1); in the
    tick that step completes, the walk to its destination (the flag, then the road)
    is ordered with its first step at progress 0 and advances from the next tick, so
    no excess carries from the door step. A settler entering ends its walk on the
    building's field and, in that arrival tick, starts a new one-step walk to the
    door field, again from progress 0. Confidence: high (E-trace-002: the computer
    player's constructor 90202 at (139, 52) progress 0.99 in trace tick 199, on
    (139, 51) with the next step to (140, 50) at progress 0 in 200 and 0.045 in 201;
    the leveller 90231 the same at 408 to 410; the carrier 90204 arriving from
    (140, 50) on (139, 51) in 568 with the step to (139, 52) at progress 0, 0.045 in
    569; T-door-2).
  - **ECO-WORK-3.8c A settler's way to a site or building is a chain of walks
    (Q-248).** After the door step (3.8b) the settler walks from the building's field
    to the building's flag as one walk; arriving on the flag, the walk along the road
    is ordered in that same tick with its first step at progress 0 (no excess
    carried) and advances from the next tick; the road walk runs continuously over
    the road's points (the excess carries from point to point, TRN-MOVE-2.3) and ends
    on the destination's flag, exactly on its sample. There the settler stands for
    one tick (his job state changes), and in the next tick his approach is ordered at
    progress 0, advancing from the tick after: for a constructor the fine walk of
    TRN-FREE-5 to the site's first position. A **worker sent from a warehouse to its
    building does not stand there**: its road walk ends exactly on the building's
    flag (the excess dropped, `TRN-settler-movement.md` TRN-MOVE-2.3) and in that
    arrival tick its walk to the building's field is ordered at progress 0, advancing
    from the next tick; the excess of the road walk's last step does not run on into
    it (2026-10-25 evening, measured on every such walk of E-trace-010: hunters 90379
    and 90425, woodcutter 90392, T-door-3; soldiers going to their building are not
    measured, medium). Confidence: high for the constructor
    (E-trace-002, settler 90202: on (139, 51) at 0.99 in trace tick 221, on the
    castle's flag (140, 50) with the step to (139, 49) at 0 in 222 and 0.045 in 223;
    through (139, 49) at 0.935 in 265 and (140, 48) at 0.925 in 287 without a
    restart; standing on the hunter site's flag (141, 48), sample (564, 192), in 289;
    the fine walk to (564, 193) at progress 0 in 290 and 0.9 in 300). The road
    route is chosen when the road walk is ordered on the flag and again at every
    flag reached (`TRN-free-walk.md` TRN-FREE-6.5, 2026-10-25, Q-354); medium for
    a route over several roads (the new search at every flag between two roads,
    6.5 c, is from the code, not observed).
  - **ECO-WORK-3.8d Who stands on the building's flag (2026-10-24, Q-270).** The
    same-tick order of 3.8c holds for a settler whose walk on is part of the same job
    step: the constructor, the leveller, a worker or soldier sent from a warehouse to
    its building, and a worker leaving its own building for outside work (woodcutter,
    stonecutter, hunter). A **road carrier** stands one tick on the flag of whatever
    building it comes out of, because its job state changes on arrival, and orders
    its walk on in the next tick at progress 0: a new carrier from its warehouse,
    whether or not that flag is an end of its road, and a carrier coming out of a
    building after delivering a ware. A worker who comes out to put its product down
    at the flag stands too (its job state does not change). **A warehouse's helper
    carrying a ware out does not stand** (corrected the same day): its put-down's
    drop-time action starts in the arrival tick (`TRN-carriers-transport.md`
    TRN-CARRY-3.3a). Confidence: high (measured by the investigation's flag-arrival check
    over E-trace-005 and 006, every walk from a building's field ending on its flag:
    carriers 164 of 164 stand: 50 new carriers out of the castle, 114 after
    deliveries into 12 building types; ware helpers out of the castle 123 of 123 start
    the drop-time action in the arrival tick; workers putting a product down 14 of 14
    stand; settlers going on in a walk 141 of 141 with the step ordered: 33
    constructors, 22 levellers, 16 workers and 2 soldiers out of the castle, 68
    workers out of their huts. Examples: carrier 90234 on the castle's flag (140, 50) standing in table
    tick 463, step to (139, 49) at progress 0 in 464; carrier 90237, whose road
    90228 runs (139, 48) .. (141, 48), standing there in 493 and stepping in 494, the
    trace ticks one higher).
- **ECO-WORK-3.9 Rest.** A counter starts at 0 and goes up by 1 per tick; the rest
  is over when it is greater than the rest time of the worker's profession
  (`gp_setRestTime`): rest time + 1 ticks.
- **ECO-WORK-3.10** Then ECO-WORK-3.1 again; with inputs in stock the next work
  action starts 1 tick later: the tick the rest is over is the first waiting
  tick (job state of waiting), the work action starts in the tick after it
  (measured 2026-10-25 evening, E-trace-010: the sawmill's worker waiting from
  trace tick 3624, working from 3625). An outdoor worker's next step starts in the
  rest's last tick itself (`ECO-outdoor-workers.md` ECO-OUT-3.10).

### ECO-WORK-4 Totals

- **ECO-WORK-4.1** With inputs in stock, room at the flag and level ground,
  counting from the tick the work action ends (tick 0):

  | Tick | Event |
  |---|---|
  | 0 | work ends, product exists |
  | 8 | product taken |
  | 10 | worker leaves the building (two standing ticks, 3.4) |
  | 33 | worker at the flag |
  | 38 | ware lies at the flag |
  | 42 | worker turns back |
  | 65 | worker back at the building |
  | 66 | rest begins |
  | 122 | rest over |
  | 123 | next work action starts |

  (Every tick after 8 corrected 2026-10-25 evening by one, with 3.4's second
  standing tick.)

- **ECO-WORK-4.2** Gap between two work actions = `ceil(drop) + ceil(pickup) + 2
  + walk out + 1 + ceil(drop) + ceil(pickup) + walk back + 1 + (rest + 1) + 1`.
  Shipped values, level ground, rest 55: 123 ticks = 12.3 s.
- **ECO-WORK-4.3** Cycle length = production time + gap. Sawmill: 333 + 123 =
  456 ticks = 45.6 s per plank, 7.9 planks per 6 minutes. With a rest time of
  222 (pig farmer, slaughterer): 333 + 290 = 623 ticks.
- **ECO-WORK-4.4** A height difference between building and flag of more than
  200 units makes the uphill walk 28, 38 or 56 ticks instead of 23
  (TRN-MOVE-1.3); the other direction stays 23.

### ECO-WORK-6 Going home and dying (Q-318)

- **ECO-WORK-6.1** A settler whose task ends without a building to enter goes home:
  it walks to the field of the building it is bound to. When that walk ends, the
  settler is taken in (removed from the map, into the building) only if a building
  stands on that field (a construction site does not count) and the settler is within
  one field of it. Otherwise the settler dies on the spot. Confidence: medium (code;
  one case measured, 6.3).
- **ECO-WORK-6.2 Dying.** A dying settler stands for the dying clip's length (28 ticks
  for a soldier; the kind's dying duration) showing work loop 2 as its animation
  request; when 14 ticks of it are left, remains (a doodad) are dropped on its
  building's door field or, with no door, on the building's field; when the countdown
  reaches 0 the settler is dead and is removed in the next update. A soldier also
  dies this way when its health is 0 or less at its update (MIL-SOL-1.2), except
  while idle at creation, dead, going home or already dying, and when a catapult
  stone hits it. Confidence: medium (code; the 28-tick countdown and the animation
  measured once).
- **ECO-WORK-6.3 Measured case (live trace 2).** Soldier 98253 of the computer player
  left its castle at trace tick 66 389 and walked for 1250 ticks (state "to
  building") on a long detour north-west and back; the walk ended at (111, 40), not
  on its building's field, so it went home (state "going home") for 50 ticks to
  (111, 42), where a depot had been placed as a construction site at trace tick
  67 229 and was still unlevelled; there it died: dying from trace tick 67 729
  (animation code 6, 20 then 10 ticks left of 28), dead at 67 749, gone at 67 759.
  No enemy was near. Why the soldier was bound to a site placed 840 ticks after it
  set out is open (Q-319). Confidence: high for the observation.
## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| production_time (sawmill and most workshops, mines) | 333 | ticks of work animation | high (measured) | script, E-eco-004, E-obs-001 |
| drop_time / pickup_time | 3.3 / 3.3 | ticks, each lasting 4 | high | script, E-eco-004 |
| rest_time (carpenter, miller, baker, brewer, smith, coiner, smelter, locksmith, miner, farmer, helper) | 55 | ticks, lasting 56 | high | script `Jobs.lua`, E-eco-004 |
| rest_time (pig farmer, slaughterer, ship builder, donkey breeder, woodcutter, forester, stonecutter, hunter) | 222 | ticks | high (value) | script `Jobs.lua` |
| rest_time (fisher) | 1111 | ticks | high (value) | script `Jobs.lua` |
| flag direction from the building | 1 (south-east) | direction | high | E-eco-004 |
| door offset default | none | | high | E-eco-004 |
| order of the steps | ECO-WORK-3 | | high | E-eco-004 |
| exact tick offsets in ECO-WORK-4.1 | as listed | ticks | medium (derived by reading, each transition could be off by one) | E-eco-004 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | sawmill with logs in stock, flag with room, building and flag at the same height | measure work action start to work action end | 333 ticks (33.3 s). Observed: about 33 s (E-obs-001) |
| T2 | as T1 | measure work action end to next work action start | 123 ticks (12.3 s) |
| T3 | as T1 | measure ware laid at the flag to next ware laid at the flag | 456 ticks (45.6 s) |
| T4 | as T1 | events after the work action ends | the ticks of ECO-WORK-4.1 |
| T5 | as T1 but no log in stock when the rest ends; a log arrives 100 ticks later | observe | no work action until the log is there; it starts within 1 tick after; no second rest |
| T6 | as T1 | measure ware laid at the flag to next work action start | 85 ticks (8.5 s); 90 ticks if the building is more than 200 units higher than the flag. Observed: 8 to 9 s for "end of one plank until the next sawing starts", with the worker seen walking (E-obs-002, E-obs-004). This matches only if "end of one plank" was the moment the plank was put down at the flag; counted from the end of sawing the rules give 12.3 s (T2). To be confirmed (Q-028). |
| T7 | as T1 but the flag already holds its limit of wares | observe | the worker stays in the building after taking the product (ECO-WORK-3.4) until the flag has room; the cycle is delayed by that time |
| T8 | flag 201..600 units higher than the building | walk out / walk back | 28 ticks / 23 ticks; gap 128 ticks |
| T9 | rest time 222, otherwise as T1 | gap between work actions | 290 ticks |
| T10 | timed action of 3.3 started in tick N | when does it end | tick N+4 |
| T11 | 3.1b: E-trace-011, player 1's locksmithery 90685 (135, 48), its production switched off by the computer player from counter 11789 and on again at 19540 (buildings table), worker 91031 standing inside, inputs in stock | when the work starts | WorkLoop from trace tick 19543 (counter 19542); starting from the tick after the switch (an experiment build) works from 19542 (settlers, view columns left out, first differ at 19886; with the rule at 19889) |
| T12 | 3.1b, a worker waiting when switched off: E-trace-038 (MP_6P_the_others), player 4's locksmith 21113 in the locksmithy (55, 21), waiting for inputs (worker state 33) since counter 8 939; the computer player switches production off at 9 078 and on at 18 177 (its tools part, every 337 ticks; the buildings table's enabled column 0 from 9 079, 1 from 18 179) | his state and work | state 33 through the whole stretch (the switched-off state never shows); the work animation from counter 18 178 (333 ticks, 332 left at 18 179); his walk out with the tools at progress 0 in 18 521 (0.36 at 18 529). At the second switch-off (about 18 859) he was working: he finished, carried the tools out, came back and stood in the switched-off state by 19 089. Moving him to that state at the switch-off starts the work at 18 179 and puts his walk out one tick late. Experiment build: the game's settlers 18 530 -> 19 250; every other pair (T11's Weak game included) unchanged |
| T-door-1 | castle at (139, 51), a constructor sent out | its first walking tick | field (139, 52), the castle's door field; never (139, 51) |
| T-door-3 | ECO-WORK-3.8c: E-trace-010 (two-player game), hunter 90379 sent from the castle to the hunter's hut (140, 49), its road walk (140, 50) -> (141, 50) -> (141, 49) -> (141, 48) | its arrival on the hut's flag | trace tick 1815: on the flag's sample (564000, 192000), walk animation; 1816: one tick along the step to (140, 49) (rise 943, factor 0.6: (563945, 192108)); standing on (140, 49) in 1853. Running on with the excess would give (563969, 192060) in 1815 and the arrival in 1852 |
| T-door-4 | 3.8, 3.8a, 3.8b: E-trace-011, player 1's farm 90538 on (136, 58) (door north-west: (135, 59)), its first farmer 91017 made at the castle at counter 9562 | his arrival and first trip | trace tick 9904: on the farm's field, the step to the door at progress 0; 9927: on (135, 59), standing; the sowing step's 51-tick wait there; 9979: the step back onto (136, 58) from progress 0, arriving 10002; then to the flag (136, 57) from progress 0, arriving 10025, and on to his spot (sowing from 10118) (measured). Without the door steps the farmer waits on (136, 58) and is 46 ticks early (experiment build: settlers first differ at 9904; with the rule 10015) |
| T-door-5 | 3.8a, 3.8b (the donkey breeding's product; added 2026-10-30): E-trace-014 (R4, the Weak game), player 1's donkey breeding 92529 on (141, 35) (door field (141, 36), its flag (142, 34)) makes donkey 93642, which goes home to a warehouse (ECO-FOOD-6.2) | the donkey's first rows | counter 34834: it appears on the door field (141, 36), walking to (141, 35) from progress 0 (0.045 a tick, the 23-tick step); 34857: on (141, 35), the step to the flag (142, 34) at progress 0; 34880 standing on the flag, then over the roads, in (144, 47) at 35230. A donkey placed on the building's field (walking to the flag from 34834) is one step early: the experiment build's settlers first differ at 34840 (with T50 of `TRN-carriers-transport.md`) |
| T-arrive-1 | ECO-WORK-3.1a: E-trace-010, the computer player's spring 90510 on (136, 53) (no inputs), its worker 90699 (a helper-type worker) arriving inside in trace tick 5558 | the worker's rows 5558 .. 5560 and his first product | Stand in 5558 (still walking state) and 5559 (waiting), the work animation from 5560; the product (water 90784) made at counter 5896 (measured). Starting the work in 5559 makes the product, and every later step, one tick early |
| T-room-1 | ECO-WORK-3.4 at a full flag: E-trace-011, player 1's miller 91002 in the mill (140, 53), his flour 91303 waiting for room at the flag (141, 52), which holds 6 wares until a ware leaves in counter 13518 (after his check of that tick) | his checks | 13519 finds room and raises the bar, 13520 admits the flour (its place at the flag from 13520 in the recorded wares table) and he walks out from 13520 (trace tick 13521). Taking the place at once when no bar is set (an experiment build) walks him out at 13519: settlers first differ at 13521; with the entry check 13983 |
| T-room-2 | ECO-WORK-3.4 (the bar already set): E-trace-013 (R3, the Hard game), the sawmill 90517 (136, 55), its worker 90868 with plank 91471, the take ending at counter 10837, its flag (137, 54) empty; carrier 90594 of the road (138, 53) - (137, 53) - (137, 54) walking there with plank 91448 from counter 10837, whose first check (10838) raises the flag's bar to 0 (inferred from the places' order) | his checks | 10838 first standing tick; 10839 (second) finds the bar at the plank's score 0 and admits it: its place at the flag from 10839 (wares table), Walk from trace tick 10840; 91448's place from 10840 (measured). Only raising in the second tick (an experiment build) walks him out at trace tick 10842: the Hard pair's settlers first differ at 10841; with the rule at 11102; the Weak pairs, the dense 6000 pair and the single-player pair unchanged |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-004 | code reading | `S2DNG.exe` | worker update: which building types use which cycle |
| E-eco-004 | code reading | | the cycle of ECO-WORK-1.1: work, then deliver-and-rest, repeated |
| E-eco-004 | code reading | | wait for inputs, work action with the production time, production |
| E-eco-004 | code reading | | order inside deliver-and-rest |
| E-eco-004 | code reading | | taking the product: drop time then pickup time |
| E-eco-004 | code reading | | waiting for the flag |
| E-eco-004 | code reading | | walk out to the flag (direction 1) |
| E-eco-004 | code reading | | putting the ware down at the flag |
| E-eco-004 | code reading | | walk back (direction 4), door offset step |
| E-eco-004 | code reading | | rest counter against the profession's rest time |
| E-eco-004 | code reading | | timed actions and the order inside a settler's tick |
| E-obs-001 | in-game observation | Jens's observation notes, 2026-10-13 | sawing start to plank: about 33 s |
| E-obs-002 | in-game observation | same | end of one plank to next sawing: 8 to 9 s |
| E-obs-004 | in-game observation | same | worker visibly moves in that interval |
| E-sim-001 | code reading | | ECO-WORK-6 |
| E-trace-002 | live read of the running game | `settlers.csv.gz` object 98253, `buildings.csv.gz` (111, 42) | ECO-WORK-6.3 |
| E-trace-011 | recording | `wares.csv.gz`, `settlers.csv.gz` (miller 91002, flour 91303, flag 90597 (141, 52), counters 13515 .. 13521) | ECO-WORK-3.4 (T-room-1) |
| E-eco-061, E-trace-038 | recording and experiment build | (session 2026-11-27 rec): locksmith 21113's worker states and animation columns at counters 8 939 .. 19 109 and the locksmithy's enabled column | ECO-WORK-3.1b, T12 |
| E-trn-021, E-trace-013 | code reading and recording | the entry check with the bar; the wares table of E-trace-013 (counters 10836 .. 10842) | ECO-WORK-3.4 (T-room-2) |

## Open questions

- Which moment "end of one plank" was in E-obs-002 (T6), and a direct
  measurement of T2 and T3. (Q-028)
- The cycles of the buildings in ECO-WORK-1.2. (Q-005)
- What exactly "can produce" checks per building (inputs, second goods, mine
  food and deposits) and what the building does to its stock. (Q-005)
- The exact condition for "the flag can take the ware" and how a worker and
  carriers share the flag's limit. (Q-004)
- The extra walk for buildings with a door offset direction (ECO-WORK-3.8).
