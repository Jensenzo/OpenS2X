# TRN-FREE: Walking across open ground and along roads

Status: draft (route search, costs, tie rule, stopping short, fine approach and road walking read from the binary; test routes computed with the rules on a real map, not observed in the game)
Owner question(s): Q-039 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

A settler who walks somewhere off the roads (woodcutter to a tree, forester to
a spot, any worker to or from its building's flag) gets a route of fields from
a search that is run **backwards, from the target towards the walker**. Each
step costs 3, more uphill, and 2 more when leaving a field with a tree or
stone pile. The search stops as soon as it reaches the walker. The route then
goes downhill in the search's labels from the walker to the target. Among
equally good next fields it keeps the walker's current direction if it can,
otherwise it picks at random. Walking the route is timed field by field as in
`TRN-settler-movement.md`. Settlers going to a building in the road network
(a new worker, builder, carrier) walk along roads instead, flag to flag.

## Rules

Fields, directions and distances as in `MAP-grid-geometry.md`; field state
bits as in `ECO-building-placement.md` ECO-PLACE-1; height classes as in
`TRN-roads-flags.md` TRN-ROAD-2.2; step timing as in `TRN-settler-movement.md`
TRN-MOVE-1 and 2.

### TRN-FREE-1 Which walk is used

- **TRN-FREE-1.1** **Free walk** (this spec, TRN-FREE-2 to 5): used by
  - a worker stepping out of its building to the flag and back in (the
    building field and its flag, TRN-FREE-2.2);
  - woodcutter, stonemason and forester walking from the flag to the target
    and back (`ECO-outdoor-workers.md` ECO-OUT-3.4, 3.8, 4.6, 4.8);
  - the builder's and leveller's moves on the site (fine walk, TRN-FREE-5).
- **TRN-FREE-1.2** **Road walk** (TRN-FREE-6): used when a settler goes to
  a building or road of the network: a newly sent worker to its building, a
  builder or leveller to its site, a carrier from its warehouse to its road
  (TRN-CARRY-1.4).

### TRN-FREE-2 Route search

- **TRN-FREE-2.1** A walk from the walker's field `W` to a target field `T`
  (`W != T`). If `T` is a neighbour of `W`, the route is that single step, with
  no search.
- **TRN-FREE-2.2** If one of `W` and `T` holds a building and the other is
  that building's flag, the route is that single step, with no search.
- **TRN-FREE-2.3** Otherwise a search runs **from `T` towards `W`**. Each field
  gets a label, the cost of the best path to `T` found so far (`T` has 0; all
  others start without a label).
- **TRN-FREE-2.4** Open list: entries are kept sorted by `f = label +
  distance to W`. A new entry goes behind all entries with the same or smaller
  `f` (first in, first out among equal `f`). The search takes the front entry,
  then looks at its neighbours in direction order 0, 1, 2, 3, 4, 5.
- **TRN-FREE-2.5** For a field `N` taken from the list and its neighbour `Q`
  (on the map):
  - **passable:** on the first try, `Q` must be walkable (none of Blocked 0,
    Water 2, ElevationBlocked 6, MovementBlocked 8, DoodadBlocked 10; tree and
    stone fields, DepositBlocked 7, are walkable) unless `Q` is `W`. On the
    second try (TRN-FREE-2.8) every field is passable.
  - **step cost** `c(N, Q)`: by the height class of `height(Q) - height(N)`:
    class -1 gives 4, -2 gives 5, -3 gives 7, anything else 3. Then 2 more if
    `Q` carries DepositBlocked. (The walker steps from `Q` to `N`, so the
    class is negative when the walker climbs: **uphill costs 4, 5, 7; level,
    downhill and the extreme class 3**.)
  - `g = label(N) + c(N, Q)`. If `Q` has no label, or a label greater than
    `g`: `Q`'s label becomes `g` and `Q` is put into the open list with `f = g
    + distance(Q, W)`. If it was already in the list it is taken out and put
    back with its new `f`; if it had been taken from the list before, it is
    put back in.
  - If `Q` is `W`, the search ends at once (even if a cheaper path to `W`
    could still be found).
- **TRN-FREE-2.6** If the open list becomes empty first, the try fails.
- **TRN-FREE-2.7** There is no limit on the search area or route length.
- **TRN-FREE-2.8** If the first try fails, the walk is tried once more with
  every field passable. If that fails too, the walk is not started. (So a
  settler can cross water or blocked ground when nothing else reaches the
  target.)

### TRN-FREE-3 Route from the labels

- **TRN-FREE-3.1** The route starts at `W` and ends at `T`. From the current
  field `P`, the candidates for the next field are the neighbours with a label
  less than `P`'s label; of those, the ones with the **smallest** label are
  allowed.
- **TRN-FREE-3.2** Choice: at the start a direction is drawn at random (0 to 5,
  from the game's random number generator). At each step, if the current
  direction is allowed it is kept; otherwise one of the allowed directions is
  drawn at random, each equally likely, and becomes the current direction.
  So routes run straight as long as possible, and two walks over the same
  ground can differ.
- **TRN-FREE-3.2a** (Q-119) The current direction is the starting draw before
  the first step and afterwards always the direction of the last step taken;
  nothing else changes it. A tie between allowed directions therefore only
  leads to a draw when the current direction is not among them. Random
  numbers used per route: one for the start, plus one at every step where the
  current direction is not allowed (the draw picks the k-th allowed direction
  in direction order, `k = draw mod count + 1`). Paths that descend through
  the labels but that this rule never takes are not routes: the counts of
  "possible routes" in the test cases are the routes the rule can produce
  over all six starting draws. A single-step route (TRN-FREE-2.1, 2.2) draws
  nothing at all. Animals walk with the same read-out and the same draws
  (ECO-FOOD-5.14); the computer player's road-stop search does too (AI-2.7 b),
  while the road planner and the street route optimizer read their paths out
  without draws (first allowed direction). Confidence high (measured for the
  animals, ECO-FOOD T31; code for the rest, E-sim-001).
- **TRN-FREE-3.3** The route always has `label(W)` as its cost. Different
  random choices can give routes of different length in fields and
  different walking times (test cases give the range).

### TRN-FREE-4 Walking the route

- **TRN-FREE-4.1** The route is walked step by step as in TRN-MOVE-1 and 2:
  each step's time follows from the slope factor of that step (uphill 0.8, 0.6,
  0.4; Water 1.0), with the leftover progress carried into the next step.
  Level ground: 23, 45, 67, 89 ... ticks for 1, 2, 3, 4 ... fields.
- **TRN-FREE-4.2 Stopping short.** A walk can be given a number `k` of fields
  to stop short. In each tick, after the movement step: if a route is being
  walked and fewer than `k` fields of it remain after the field the walker is
  now heading for, the walk ends at once and the walker stays on the field he
  last reached. With `k = 1` (woodcutter, stonemason, forester to their
  target): **the walker stops on the last field before the target** and does
  not start the final step. If the target is a neighbour, he does not move.
- **TRN-FREE-4.3 New walk while walking.** Starting a new walk replaces the
  current one at once: the new route is searched from the field the walker
  last reached, and the progress of the step he was in is dropped (he
  restarts from that field). If the field he last reached already is the new
  target, the walk counts as done immediately. (This answers Q-107.)

### TRN-FREE-5 Fine approach

- **TRN-FREE-5.1** After stopping short, the woodcutter, stonemason and forester
  walk on the height lattice (4 samples per field, `MAP-grid-geometry.md`
  MAP-GRID-4) to the object's exact position. The route is a straight line:
  starting from the object's position, the lattice direction towards the
  walker (MAP-GRID-2.2 bearing sectors, on the lattice) is taken step by step
  until the walker's position is reached; the walker walks it in reverse. The
  hunter walks the same way towards the point of the animal's field, stopping 2
  lattice steps short (`ECO-outdoor-workers.md` ECO-OUT-8.1, T18), also when a tree
  or stone stands on that field (its point, not the object's position: hunter 90379
  on (145, 61) in trace tick 3052, the rabbit 90092 on (146, 61), straight east;
  the field is the animal's when each walk starts, code E-eco-017, medium). During such a
  walk the settler's field follows its samples by `MAP-grid-geometry.md`
  MAP-GRID-4.3 (the field the last sample reached truncates to), as for the
  builder (measured 2026-10-25 evening, E-trace-010: woodcutter 90392 on (142, 55)
  from trace tick 2234 and (143, 55) from 2245 while his free walk ended on
  (143, 56); hunter 90379 on (139, 53) from 2133).
- **TRN-FREE-5.2** This walk stops short by the object's elevation size
  (`dp_setElevationSize`: 1 for trees and small stone piles, 2 for larger
  piles) in lattice steps, by the rule of TRN-FREE-4.2.
- **TRN-FREE-5.3** Timing: TRN-MOVE-2.5, progress `2 x 0.045 x factor` per
  tick per lattice step, with carry-over. Level ground: 12, 23, 34, 45 ticks
  for 1 to 4 lattice steps; factor 0.8: 14, 28, 42, 56; 0.6: 19, 38, 56, 75;
  0.4: 28, 56, 84, 112.
  **Each step's factor when that step starts (2026-11-19 night, E-trn-041: the Weak game's recording
  and its heights table; high).** The fine approach and the fine walk back of an
  outdoor worker (5.1, 5.6) are walks of TRN-MOVE-2.5 like the builders': a step's progress per tick
  is taken from the heights of its two samples **when that step starts** (the first when the walk is
  ordered, each later one in the tick the step before ends), the excess carried. The walk's length is
  therefore not known when it starts: a site levelled nearby during the walk changes the steps not yet
  begun, and the worker reaches the object (and works, takes, walks back) as those steps' new factors
  give, not as the heights at the start would. Positions under way follow the same steps (the steps
  walked keep their factors). In R5 every one of the 11 lattice steps (of 12 550 replayed against its heights
  table) whose heights changed while its walk was under way took the factor of its own start. Test T48.
- **TRN-FREE-5.4** Deposit position: when a deposit is created, one number is
  drawn from the game's random number generator, `d = draw mod 6`. If the
  deposit kind has the random offset (`dp_setRandomFinePosOffsetState`, default
  on: trees; off: stone piles and grain fields) and the field is on the map,
  the deposit stands **one lattice step from its field's lattice point, in
  lattice direction `d`**; otherwise at the field's lattice point. (The draw is
  made for every deposit, also when it is not used.) So the fine approach to a
  tree from a neighbouring field is 3, 4 or 5 lattice steps, of which the
  walker walks all but the elevation size.
  The view draws the deposit at this position too (`GFX-objects.md` GFX-OBJ-1.4, Q-1027 b,
  E-gfx-040); a map's or save's deposit keeps the position stored in the file.
- **TRN-FREE-5.5** The builder walks between its four construction positions
  with the same fine walk (straight lattice line, TRN-FREE-5.1), without
  stopping short. (Q-120)
  - **TRN-FREE-5.5a** Position `i` (0 to 3) is a height sample (lattice
    point, `MAP-grid-geometry.md` MAP-GRID-4): take the **building's flag
    field** (the neighbour in direction 1 of the building field), its sample
    `(c, r)` by MAP-GRID-4.2, and add the offset of `bp_setConstructionPosition`
    for that building type, the building's tribe and index `i`: `c + x`,
    `r + y`. The offset is in sample steps (a quarter field), x along the
    sample row, y in sample rows; it is converted to a whole number by
    truncation towards zero (all 372 offsets in the shipped script are whole
    numbers, x from -5 to 2, y from 0 to 8, so they lie between the flag and
    the building). No odd-row correction is applied to the offset. Confidence
    high.
  - **TRN-FREE-5.5b** The tribe used is the tribe of the player who owns the
    building, copied into the building when it is created. A player's tribe
    (0 Romans, 1 Africans, 2 Chinese, numbering of `buildings.lua`) is a
    per-player setting of the game setup, held with team, colour and start
    position. A free game takes it from the slot's tribe chosen in the setup
    screen, not from the map (`UI-game-setup.md`, Q-051). Confidence high. An
    implementation should treat the tribe as an input of the game setup per
    player, default 0 (Romans).
  - **TRN-FREE-5.5c** The four positions are visited in index order 0, 1, 2,
    3 within each work unit (ECO-HQ-4.6); each walk starts from the lattice
    point the builder stands on: the flag field's sample before the first
    unit (after the approach walk, ECO-HQ-4.6b), position 3 before every
    later unit. Tick alignment of the walks: ECO-HQ-4.6b (Q-125).

- **TRN-FREE-5.6 Walk home after a fine approach (Run D, E-gfx-015).** The
  woodcutter, stonemason and forester (with a ware or empty) walk home in
  these steps; they are the same for every worker that walks back to its
  building's flag after working outside:
  1. **Fine walk back.** From the sample where he stands, a fine walk (the
     straight lattice line of 5.1, no stopping short) to the lattice point of
     the field F **where his walk to the target stopped** (the last field the
     free walk of 4.2 reached; if he did not move, the field he started on).
     It is not the field his current sample truncates to. Timing as 5.3 (12,
     23, 34, 45 ticks for 1 to 4 sample steps on level ground), ordered in the
     tick the previous step (taking the ware) ends, first advancing in the
     next tick (TRN-MOVE-2.1). This step is skipped only when F is the
     building's flag field and the flag cannot be entered at that moment; he
     then waits (step 3).
  2. **Walk to the flag.** In the tick the fine walk ends (he is exactly on
     F's point, so his current field is F) the free walk of 2 to 4 to the
     building's flag field is ordered from F; it first advances in the next
     tick. If he stands on a road, entering the flag is subject to the flag's
     queue (one settler at a time; the order is not traced).
  3. **Wait.** When the walk ends elsewhere than on the flag field, or the
     flag cannot be entered, he waits until it can. Then: if he stands
     **exactly on the point of his current field** (the field his sample
     truncates to, MAP-GRID-4.3), step 2 follows at once, the free walk to the
     flag from that field; otherwise he starts again with step 1 (the fine
     walk to F's point). The skip of step 1 and this wait are the standing tick
     of `ECO-outdoor-workers.md` ECO-OUT-3.8 (F the flag field). (Corrected
     2026-10-25 evening, T27: a stand on a field's point went on with the fine
     walk; E-trn-014: the fine position compared with
     the current field's sample.)
  The whole walk home is continuous: no tick moves him by more than one
  step's progress. Confidence: high (E-gfx-015); the flag queue medium.

### TRN-FREE-6 Road walk

- **TRN-FREE-6.1** A road walk goes from the flag or road the walker stands on
  to the flag of its destination. **A destination on a road field** (a carrier's
  walk to its road's idle field, TRN-CARRY-1.4, 6.5 (g); added 2026-10-29, T39;
  corrected 2026-11-01, Q-378, T42: was "the flag reached in fewer fields"): the
  walk's goal is the road's **first flag** A (the flag at entry 0 of its fields).
  The route to A is the search below with one change: **the road the walker is
  assigned to costs 1** instead of its length (for a new carrier or donkey on its
  way, and for a carrier going back to its road, that is the destination road
  itself). If that route comes into A over the destination road from its other
  flag B (B is the flag before A on the route read out below), the walk ends at
  B; otherwise at A. From that flag it walks along the road to the field; that
  part of the road is not counted. So it heads for B when cost(B) + 1 < cost(A),
  for A when cost(A) < cost(B) + 1, and on equal costs by the read-out rule
  below (the road into A attached there first wins; the destination road is
  usually the newer). The choice is made again at every flag (6.5 (c)).
  Confidence: high (all four measured cases, T39, T41, T42, and the experiment
  build: Weak settlers vs R5 48528 -> 49093, no pair earlier; E-trn-028). Which object sets the walker's road for the search is not traced
  (the same record names the carrier's own road when parallel roads are
  compared, E-trn-009); the measured walks are all to the walker's own
  road. **A walk that starts on a road field** (measured 2026-11-01 night, T43):
  the search starts at that road's first flag A (entry 0 of its fields), and if
  the route's next flag is the road's other flag B the walker goes there along
  the road directly; otherwise it walks along the road to A and on from there.
  Which end is nearer to the walker does not count (E-trn-028; T43: a
  released carrier on a road's inner field walks 2 fields to A and on, where the
  walk through B, 4 fields away, is not taken). The flags are connected by a search over
  the flag network (the one used for ware routes, TRN-CARRY-5): each road costs
  its length L (TRN-CARRY-5.2; the walker's own road 1, above), **without** the full-flag and busy-road
  penalties of TRN-CARRY-5.3 (those apply only to ware routing); waterways only
  for boats. The route then follows the fields of the roads between those
  flags, one road at a time: the search runs again at every flag reached
  (6.5). **Equal routes (2026-10-27 night, T31).** The search from the walker's
  flag keeps for every flag only its cost (the open list ordered by cost, a flag
  reached at an equal cost after another one taken after it); the route is then
  read out backwards from the destination: at each flag, the first of the
  flag's roads, in the order the roads were attached to it (their creation
  order in the measured case: a new road's entry is appended to each of its
  two flags' lists and a removed road's entry is taken out, the others keeping
  their order; **a split road** (TRN-ROAD-4) is removed and its two halves are
  made one after the other: first the half from the old road's first flag to
  the new flag, appended last at that first flag and first at the new flag,
  then the half from the old road's last flag to the new flag, appended last at
  that flag and second at the new flag, so neither half keeps the old road's
  place; Q-280, E-trn-016, code only, medium), whose other flag lies on a cheapest route (its cost plus the road's
  length equals this flag's cost) gives the previous flag. So of two equal
  routes the one whose last road into the destination was attached there first
  wins, not the one through the lower field (E-trn-016; measured: experiment build, settlers 9602 -> 9904
  against E-trace-011). Confidence: high for walkers; ware routes read out
  the same way (`TRN-carriers-transport.md` TRN-CARRY-5.4, T30; 2026-10-27
  night, E-trn-018).
- **TRN-FREE-6.2** A new worker leaves its warehouse with one step to the
  warehouse's flag (TRN-FREE-2.2), walks along roads to its building's flag,
  then steps in (TRN-FREE-2.2). Builders and levellers go the same way to the
  site's flag. Carriers go to their road and its idle field (TRN-CARRY-1.4).
- **TRN-FREE-6.3** If no road route exists, the road walk does not start (the
  settler is only sent when the destination is in the warehouse's network,
  `ECO-requests-dispatch.md` ECO-REQ-3.2). What it then does is not traced. Read
  from the code (2026-10-25, Q-272; medium, code only, no recording of the case): a road
  walk that does not start or ends short (6.5 e) is never retried, neither by the walk
  nor by the job that ordered it. The job that ordered it counts as done once the
  settler stands, wherever that is, and the settler's kind then checks where it stands:
  - **Builder** (to its site) and **leveller** (to its levelling site): when the walk to
    the site's flag is done, the builder starts building (and the leveller levelling)
    only if he stands on that flag; otherwise he goes home at once (6.4, from the flag
    he stands on).
  - **Soldier** (to its military building): when the walk is done he enters only if he
    stands on the building's field (the step in, 2.2, included); otherwise he goes home.
  - **Worker** (to its building): the going-to-building job ends without entering
    (walk done short -> done). **Corrected 2026-11-15 night (Q-272; E-trn-034, code reading):** the
    work routine does not start there: every work routine (the generic production routine of the
    workshops, mines and the well, and the woodcutter's and stonemason's, forester's, farmer's,
    fisher's, hunter's, donkey breeder's, shipwright's and catapult's routines) checks, when the walk
    to its building has ended, whether the worker stands on the building's field (entered); if not,
    he goes home at once (6.4, from where he stands). So a worker never works from a far flag. A worker
    whose building is gone goes home (6.4). The building keeps him linked while he walks home and asks
    for a new worker only after his object goes (6.3a, Worker).
  - **Carrier** on its way to its road (TRN-CARRY-1.4): the walk to the idle field
    belongs to its idle step, and that step does retry: once the walk is done,
    wherever it ended, the next check finds the carrier off its idle field, looks for
    work if it stands on one of its road's fields, and orders the walk to the idle
    field again, a new road walk searched from where it stands; when no road walk can
    start there, the idle step ends at once. **What the job then does (2026-11-26 a, Q-046
    answered; E-trn-049, code reading; medium):** in the same update the carrier's job looks
    at its road. If it no longer has one, it goes home (6.4). Otherwise it fetches a ware waiting
    at the flag it stands for, or walks (a road walk to that position) for a ware waiting at its
    road's other end, exactly as from its idle field. With no ware waiting it keeps the idle
    step, and the next update runs that step again: the check finds it off its idle field and
    orders the walk anew. A carrier that cannot reach its road therefore **searches again in
    every update** for as long as it holds the road, with no pause and no giving up. A waiting
    ware still sends it walking, and that walk may fail the same way. Test T56. E-trn-012 (idle step: states 0x14 check and 0x15 walking;
    its walk: the road walk to a position).
  - **Geologist and scout** (to the order's flag): when the walk is done he checks that he stands
    on the order's flag field; otherwise he goes home at once from where he stands (6.4), no
    retry (`ECO-specialists.md` ECO-SPC-2.1, 2.1a, T9; 2026-11-24 b, E-trn-048, code reading, high).
  - **Going home** handles a short walk by its own end-of-walk rule (6.4 step 4: choose
    again from where it stands).

  Going home itself does not unlink the settler from its building or site (the explicit
  unlinks are at the end of building or levelling); when the building or site notices and
  requests a settler again is 6.3a. E-sim-011; E-trn-010 (builder: in state 1
  the walk-to-site job is the road walk, ended when standing; in state 4 he
  builds if he stands on the site's flag, else goes home; the leveller the same;
  soldier: when state 4 is done, state 12 if he is on the building's field, else
  home; going home: state 3 and the home job only).
- **TRN-FREE-6.3a When the building or site lets him go (answered 2026-11-13, Q-272;
  E-trn-033, code reading).** A soldier, builder or leveller who goes home after a
  short walk (6.3) stays linked to the military building or site that sent for him
  until his settler object goes. That happens in two ways only. He reaches a warehouse
  and is taken into its stock, in the settlers' update of the tick after the one in which
  his walk home ended. Or he dies on the way: going home failed too often (6.4 step 4),
  and his object goes after his dying state. Removing the object unlinks him, as for
  any settler that goes. Neither his walk nor his arrival home unlinks him earlier.
  - **Military building.** While he walks home he is one of the building's soldiers:
    the allocation (`MIL-soldiers.md` MIL-SOL-4.1, 4.2: every 21 ticks) counts present
    and coming soldiers alike, so the building orders no replacement for him. If it wants
    fewer soldiers than it counts, its release step (MIL-SOL-4.4 step 2) releases
    soldiers that are not present first, and he can be released (unlinked) on his way home. He
    walks on home either way. Once he is unlinked, the next allocation pass counts one
    soldier fewer and orders one if the building still wants him.
  - **Site.** The site keeps him as its builder (or leveller). In every tick's building
    update the site asks for a builder only when it has none linked and no open builder
    request. The leveller works the same way while the site is not yet level. So the
    new request is filed in the first building update after he is unlinked. A tick
    updates the buildings before the settlers (`TOOL-trace.md` TRACE-2.2), so this is
    the tick after his removal.

  - **Worker (2026-11-15 day, Q-1056; E-trn-035, code reading).** A worker of a
    production building who goes home after a short walk (6.3) stays one of the building's
    workers until his settler object goes, in the same two ways (taken into a warehouse's
    stock, or dying on the way). Going home does not unlink him. The building's worker request
    (`ECO-requests-dispatch.md` ECO-REQ-1.4: its own 101-tick timer, a request when the number
    of linked workers differs from the worker limit and no worker request is open) therefore
    counts him and files nothing while he walks. When his object goes, the count drops, and the
    first firing of the timer after that files the request. That is not at once: it waits for
    the timer's own phase, up to 101 ticks. A firing in the very tick he is taken into stock
    still counts him, because the buildings update before the settlers. If his building is
    removed while he walks, its removal unlinks him then (MIL-CON-4.2), and he walks on home.

  Confidence: high (code reading of the soldier's, builder's, leveller's and worker's removal
  and its unlink, the allocation, the site checks and the worker request); no recording shows
  the case.
- **TRN-FREE-6.4 Going home** (a released builder, leveller or worker, a carrier
  without road, a helper after carrying a ware out):
  1. A settler with a home warehouse assigned walks there by road walk. Otherwise
     the warehouse is chosen: from the flag (or road, or building's flag) at the
     settler's field, the warehouse with the **lowest road route cost** (as in
     6.1) among the finished warehouses of its network that accept that kind of
     settler (warehouse settings); ties: the warehouse first in the player's
     list. The settler road-walks to that warehouse's flag and steps in. **The
     cost is the route's sum of road lengths L** (`TRN-carriers-transport.md`
     TRN-CARRY-5.2, slopes counted), not its number of fields (clarified
     2026-10-28, measured T35: a builder takes the castle over a harbour whose
     route is two fields shorter). **Off a flag** (a carrier released on his
     road's inner field) it is the same sum from the first flag his road walk
     reaches, plus that first leg (added 2026-10-28 night, measured T36; the
     first leg was the same for every warehouse there, so its weight is not
     measured: medium).
  2. If none is found or the road walk cannot start, the warehouse is chosen as
     in step 1 from the field south-east of the settler (direction 1, e.g. the
     flag of the building it stands in). A settler inside a building (on the
     building's field, not a flag or road field) never gets past step 1: no road
     walk can start there (6.5 (a)), so step 2 decides (measured 2026-10-25, Q-361,
     T26: a carrier whose road was taken away while he stood in a site leaves it
     by a free walk to the nearest network flag, not over the site's own flag). (Corrected 2026-10-24, Q-266: a warehouse
     found this way is not reached by a road walk from that field, but as in 3:
     a free walk to its network flag nearest to the settler.)
  3. If that finds none: the nearest finished, accepting warehouse **on the same
     landmass** within field distance 1000, by field distance. Same landmass:
     the map's stored continent index of the warehouse's field equals that of
     the settler's field (`FMT-s2m-map.md` FMT-S2M-4.8, as loaded; plain
     equality, so a settler on a field marked -1 finds only a warehouse on a -1
     field; `AI-construction.md` AI-CON-1.1b; E-ai-016).
     For the warehouse of 2 or 3: the flag of that warehouse's network nearest to
     the settler (field distance; the network is the one the settler kind walks
     in, the waterways for a boat carrier; among flags at the same distance the
     one made first, T26, T28: two cases). If that flag is at most 30 fields away
     (configuration value), the settler free-walks there (TRN-FREE-2); otherwise
     this try fails.
  All three run in the same tick, one after the other. A settler with a home
  warehouse assigned (step 1) skips the choice in 1, not 2 and 3. **At the end of
  the walk** (road walk or free walk) the settler looks at the field north-west of
  where it stands (direction 4, a building's field seen from its flag): if a
  warehouse stands there it goes in (to the building's field, then to its door),
  **ordered in the arrival tick**: the step off the flag moves from the next tick,
  with no standing tick on the flag (added 2026-10-27 night, measured T32; the
  standing tick of 6.4c (1) belongs to a home search started on the warehouse's
  flag, not to the end of a walk);
  otherwise, in the same tick, it chooses again from where it stands (1 to 3; **all three:
  2026-11-21 day, E-trn-042, measured T52**: on a flag or road field whose road try finds no
  warehouse, 2 and then 3 follow, a free walk to the chosen warehouse's network flag; only when
  all three fail the wander step of 4 comes; the same in the home search of 6.4a (c) on a flag or
  road field), so
  a free walk that ended on a network flag is followed by a road walk home from
  that flag at progress 0 in that tick (Q-266, E-sim-008).
  4. A failed try is repeated after a wait, up to 51 times; after that the
     settler gives up (and dies, TRN-CARRY-7.1); a soldier instead loses 1 health
     and starts over (below; corrected 2026-11-09, Q-1034). The wait between tries
     is not timed here.
     **Between tries: one wander step (2026-11-08 b, E-gfx-041;
     the 2023 Coastal Storm save).** Going home is one state with a failure count, 0 when the
     settler is sent home. When the choice of 1 to 3 finds no warehouse, the settler takes one
     wander step: a number is drawn from the game's generator (`SIM-RNG.md`), r = draw mod 6;
     the six neighbour fields are looked at in ring order (the direction-4 neighbour first, then
     around the ring as in `MAP-grid-geometry.md` MAP-GRID-3.2: directions 4, 5, 0, 1, 2, 3);
     a neighbour qualifies when it is inside the map, the settler can walk there (a boat carrier:
     water) and the settler's own player has explored it; each qualifying neighbour counts r down
     by one, and the walk goes to the qualifying neighbour at which r drops below 0, or, when
     fewer qualify, to the last one that qualified (none: it stays). The step is an ordinary
     lattice walk. When it ends: with the failure count at most 50 the count goes up by one and
     the choice of 1 to 3 runs again in the settler's next update (the next tick); with the count above 50
     (the 52nd failure) any settler but a soldier gives up and dies (TRN-CARRY-7.1). So a homeless
     settler makes 52 searches and 52 wander steps, then dies, unless a warehouse comes in reach
     meanwhile.
     **A soldier (2026-11-09, Q-1034; E-mil-009):** there is
     **no search for a military building**. At that point a soldier **loses 1 health**
     (`MIL-soldiers.md` MIL-SOL-1.2, the same point a hit takes, MIL-ATK-6.6). If his health is
     then 0 or below, the state ends as for any other settler (he dies, TRN-CARRY-7.1); otherwise
     his count goes back to 0 and the choice of 1 to 3 runs again in his next update. He does not
     heal meanwhile (only present soldiers heal). So a homeless soldier with health h (1 .. 7)
     makes 52 x ceil(h) searches and wander steps before he dies, unless a warehouse comes in
     reach. Nothing in this step draws from the generator (the wander steps draw as above), no
     building is chosen, entered or asked for soldiers, and nothing is drawn on screen for the
     lost point. The earlier text ("looks for a military building") misread the two calls.
     Seen in the 2023 save: all 396 settlers of the beaten player 1 (no warehouse, flag or road)
     alive in state 3, each in the middle of its one-field wander step, failure count 14 (3 of them)
     or 15 to 22 (the other 393; corrected 2026-11-09 from "15 to 22", counted again on the save).
     **The count in a save and when it restarts (2026-11-08 c, Q-1032 c; E-trn-032):** the going-home record of a save (`FMT-savegame.md`
     FMT-SAV-3.5a c) holds four sub-records (the warehouse search and walk home, the arrival, the
     start, the wander step), then the failure count as a `u32`, then the record's own step; a
     loaded settler goes on with that count. The count is set to 0 only when going home starts
     (a new going-home state) and when a soldier survives the lost health point after the 52nd try; a
     search that finds a warehouse does not reset it, but once the walk home has started the state
     never comes back to wandering (a later need to go home starts a new state, at 0), so the
     count is never read again in that state.
     Confidence: high (code) for the step and the count; medium for the death's form (the state
     ends as at a warehouse; TRN-CARRY-7.1 gives the dying).
- **TRN-FREE-6.4a When the walk home starts (2026-10-25 night, Q-276).** Going home runs
  as a sequence of steps, one change per tick, the first in the tick after the one in
  which the settler was sent home (T, the tick its job ended; it shows the going-home
  state from T). Each tick the settler's movement is updated first, then:
  (a) *wait for the step's end*: while the step progress is at least 0 nothing happens;
      once the settler stands, a fine walk (TRN-FREE-5) to the point of its movement's
      current target, else of its own field, is ordered (nothing moves when it already
      stands there) and (b) follows;
  (b) *finish the step*: once that fine walk has ended (the settler stands), (c) is due:
      in the walk's arrival tick when it walked, in the tick after (a) when it had nothing
      to walk;
  (c) *home search*, in the tick after (b): 6.4's choice runs and orders the road walk
      (or free walk) home at progress 0; when it finds nothing, 6.4 step 4's wait
      follows.
  For a worker whose building is removed, T is the tick **after** the removal (the
  removal itself leaves its job as it is for that tick; measured T28, 2026-10-25
  evening). **Except when the building goes with land lost to a take-over (2026-11-21 day,
  E-trn-042: the war recording and an experiment build; high for the measured case):** a
  building removed because it no longer stands on its owner's inner land (`TER-territory.md`
  TER-6.2, in the buildings' part of the tick), or with a flag of the lost land (the roads' part),
  is removed before the settlers' part of that tick, and its worker's T is **the removal tick
  itself** (he shows the going-home state from it; the same for the builder hammering at a site
  removed so: added 2026-11-27, E-trn-054, T58); T28's removal by the computer player comes
  after the settlers' part (`AI-overview.md` AI-2.1a), hence T = the tick after. In short, T is the
  tick of the worker's first update after the removal. Measured T49; whether a removal by the
  player's command (applied at the tick's start) is alike is not measured. **A worker on a
  free walk** when he is sent home (walking back to his building's flag from his work, ECO-OUT)
  does not stop after the step in progress: he **completes every step of that walk** (as a
  released carrier, `TRN-carriers-transport.md` TRN-CARRY-7.1a), and from its arrival tick A
  stands in A and A + 1 and orders his walk home in A + 2 (measured T50, 2026-11-21 day; a road
  walk was not seen in this case: the soldiers walking to the taken building stopped after the
  step in progress, as before). **A reinforcement still on his walk out of the warehouse** (the door walk to the
  warehouse's own flag, ECO-WORK-3.8a, 3.8c) when the military building he goes to is taken or
  removed is not sent home then: he walks on to that flag, and in the update that walk ends there
  (A) he finds his target gone and shows the going-home state; he stands in A, A + 1, A + 2 and
  A + 3 and starts his walk home at progress 0 in A + 4 (measured T51, 2026-11-21 day, E-trn-042;
  one case. Why four (corrected 2026-11-21 eve, Q-1075; the afternoon's reading T = A + 1 is
  withdrawn): he is sent home in A, **T = A** (his arrival check runs before his arrival, so the
  going-home state shows in A); he stands as a settler standing when sent home and his home search
  in A + 3 starts on the warehouse's own flag, where it orders a road walk with no step (6.4c (1)):
  that walk ends in A + 4's movement update and his step in starts there at progress 0. No extra
  standing tick of the walk out is needed: adding one too (T = A + 1) puts the walk home in A + 5.
  Both the experiment build and the implementation, built so, give the recorded A + 4. The founded
  builder's standing tick of `SHIP-ships-expeditions.md` SHIP-7.2 is a rule of its own). **The same holds for a builder on his walk out to a
  site** (added 2026-11-27, E-trn-051: E-trace-038 and an experiment build; medium, one builder
  case; the leveller and other settlers walking out are Q-1098): his site removed while he is
  still on the door step or the step to the warehouse's own flag, he walks on as usual (the door
  step's end at progress 0, then the step to the flag), finds the site gone on arriving at the
  flag (A), stands in A .. A + 3 and walks home at progress 0 in A + 4 (T57). He is not sent home
  at the removal (6.4b's walk to the next flag does not apply before that flag). So a settler standing on its field's point (a leveller after its last levelling step,
  a worker whose building is gone) stands in T, T + 1 and T + 2 and starts its walk home
  at progress 0 in T + 3; a builder on a fine position when its building is complete
  (T = the tick after his current work action's last tick, `ECO-warehouse-construction.md`
  ECO-HQ-4.7, which is later than the completion when the building completes before
  the action ends) starts the fine walk back to its target's point in T + 1 (one standing tick) and its
  walk home in the tick after that fine walk's arrival; a settler still moving when sent
  home finishes its step (arrival tick A), and starts its walk home in A + 2. **A walk ordered but
  not yet begun counts as moving (2026-11-24 a; E-trn-047, the R6 recording and an experiment build;
  high for this case):** a walk ordered in an earlier tick already holds its first step, at progress
  0, before the movement first advances it (TRN-MOVE-2.1). A settler sent home in that state
  finishes that first step like any step in progress, and only that step, and starts home in A + 2.
  He is not a settler standing (T, T + 1, T + 2). Test T55. The
  warehouse helper that carried a ware out stands four ticks (`TRN-carriers-transport.md`
  TRN-CARRY-3.3a). Soldiers go through the same steps; a soldier released from
  a military building is sent home only when its step out reaches the
  building's flag, T being that arrival tick (`MIL-soldiers.md` MIL-SOL-4.5,
  measured T19 there). The donkey a donkey breeding makes (`ECO-food-mining.md`
  ECO-FOOD-6.2) is sent home the same way: its walk out (the door step,
  `ECO-worker-cycle.md` ECO-WORK-3.8a, then the step to the flag) ends on the
  building's flag in A, T = A, and it stands T, T + 1, T + 2 there and walks home from
  progress 0 in T + 3 (added 2026-10-30, T40). Confidence: high (code E-sim-011; measured over E-trace-005, 006, 007, every switch to
  going home: levellers 3 standing ticks in 21 of 21, workers from a removed building 3
  in 6 of 6, builders 1 in 27 of 27, carriers still walking 0 in 5 of 5; T23, T24);
  medium for soldiers (none recorded).
- **TRN-FREE-6.4b The builder of a removed site.** When a construction site is torn down
  (by its owner's or the computer player's tear-down, MIL-CON-4) while its builder works on
  it, the builder goes home by 6.4a as a builder on a fine position does after completion,
  with T the tick **after** the removal (as for a worker of a removed building): in T he
  shows the going-home state while his work action's animation still runs on (it is
  cut, not finished): the timed work action (request code 4, `GFX-animations.md`
  WorkLoop0) keeps running in T with its remaining time counted down as in any tick
  (E-trace-011: 12 ticks left at counter 6226, 11 at 6227 = trace tick 6228), so the
  trace's animation column shows the work clip in T (not Stand), and the walk ordered
  in T + 1 replaces it (code 0) (made exact 2026-10-27 night, Q-366 a); in T + 1 a fine walk (TRN-FREE-5) from his current sample to the
  point of the site's flag is ordered at progress 0; the home search follows in the tick
  after that walk's arrival. He does not appear on the site's field. Confidence: high
  (measured, E-trace-011: the hunter's hut site 90716 on (141, 46), torn down at counter
  6226 (AI-CON-9.2, a hut without value), its builder 90731 working on sample (563, 181):
  state 3 with the work animation in 6227, the fine walk (563, 181) -> (564, 181) ->
  (565, 180) -> (566, 180) = the point of the flag (141, 45) at progress 0 in 6228, on the
  flag in 6262; an experiment build that starts the builder on the site's field walks
  from (564000, 184000) instead; T29).
  **A builder in his approach walk** (`ECO-warehouse-construction.md` ECO-HQ-4.6b
  step 2: from the flag field's sample out three lattice steps and back) when the site
  is removed walks it to its end: in T he shows the going-home state and walks on along
  the same path, out and back, and the home search follows in the tick after his
  arrival on the flag's point. Nothing is cut and no fine walk is added. Confidence:
  high for this case (measured, T33); medium that any other walk under way (such as the
  walk to the site) is likewise finished first (added 2026-10-27 night).
  **On the approach's back leg (2026-11-16 day; E-trn-037, code reading and the R6 recording,
  high; T47).** When the site is removed while he walks the approach's **back leg** (the walk
  from the far point back to the flag field's point), that walk already ends on the flag's
  point, so going home's own fine walk (6.4a (a)) has **nothing left to walk**: he arrives in A,
  stands in A and A + 1, and his walk home starts at progress 0 in **A + 2** (6.4a: "(c) in the
  tick after (b), (b) in the tick after (a) when it had nothing to walk"), one tick later than
  after the out leg (T33), where the walk back is going home's own walk and the home search
  follows in A + 1. The same holds for any walk under way at the removal that already ends on
  the point going home walks to. Measured: all 17 removed sites' builders of the R6 recording
  that reached the flag's point: the 5 whose walk under way already ended there started home
  in A + 2, the 12 who needed a new walk to the point in A + 1, whether or not the flag still
  stood.
  **A work walk that ends in T itself (2026-11-22 pm; E-trn-044, the R6 recording and an experiment
  build, high for this case).** When the site goes with land lost to a take-over, T is the removal
  tick itself (6.4a). If the builder's work walk under way (T34: his walk to a construction
  position) ends in that same tick T, the line to the point going home walks to is **not** ordered
  in T. It is ordered at progress 0 in T + 1 (going home's first step comes in the tick after T,
  6.4a), and he moves from T + 2. T34's "ordered in the tick he reaches it" holds for an arrival
  after T. Test T53.
  **On his walk to the site, when the road goes under him (2026-11-17 night; E-trn-039, the R6
  recording and measured, high for this case).** A builder whose site is removed while he walks
  to it along the roads goes home with his walk ending on the next flag he reaches (as above).
  If the **road he is walking on is removed** before he gets there (for one, the site's flag and
  its road taken away after the site), his walk is cut to the **step in progress**: he finishes
  that step to its point and stops there, and nothing of the old walk is left. His walk home then
  has nothing to walk, so he stands in A and A + 1 and searches home in **A + 2**, A being his
  arrival on that point. This holds for a settler already going home; a builder still walking to
  his standing site is **not** cut this way when a road of his route goes (cutting him moved the Weak
  game's generator difference from 51 724 to 42 120: its builder of counter 41021; what he does
  instead was not followed). Whether other settlers on a road walk (a worker, a soldier) are cut
  when their road goes was not followed (medium). Test T47a.
  **A settler already on his walk home (2026-11-17 day; E-trn-040, the R6 recording and an experiment
  build, high for a carrier):** "already going home" above is a settler whose going home began with a
  road walk under way (the builder of T47a). One who is past that, on the walk home that his own home
  search chose (6.4), is cut the same way when the road he walks on is removed (he finishes the step in
  progress to its point, its excess dropped), but he does **not** stand: he chooses his way home again
  **in the tick he arrives (A)**, by 6.4's tries 1 to 3, as at the end of any walk home. Off the roads
  (the field he stopped on is no road field any more) that is a free walk to the warehouse's network
  flag nearest to him (tries 2 and 3), started at progress 0 in A and first advancing in A + 1; on that
  flag, in its arrival tick, he chooses again and road-walks on (6.4, "at the end of the walk"). Test
  T47b.
  **When the step in progress ends on that road's own end flag (2026-11-24 a; E-trn-046, the R6
  recording and an experiment build, high for this case).** The cut of T47a and T47b stops the walk
  on a field that is no road field any more. When the step under way at the removal is the road's
  **last step**, onto the flag at its end, and that flag still stands, nothing is cut: he reaches the
  flag as on any road walk, chooses his next road there by 6.5 (c) and walks on in the same tick
  **with the step's excess carried** (TRN-MOVE-2.3). No new home search, and no progress 0. Measured
  for a builder on his walk home (T54); for the walk of T47a (a settler whose going home began with a
  road walk under way) the case was not seen.
  **A builder in a work step's walk** (`ECO-warehouse-construction.md` ECO-HQ-4.6b
  step 4, the lattice walk from one construction position to the next, TRN-FREE-5.5)
  when the site is removed (2026-10-27 b, measured T34; route corrected 2026-10-28,
  Q-286): from T he shows the going-home state and **walks that whole walk on to the
  construction position it was going to**, every remaining lattice step of it (each
  step's rate set when it starts, TRN-MOVE-2.5, the excess carried from step to step
  within the walk, TRN-FREE-5.3), not only the step in progress. **In the tick he
  reaches that position** (no standing tick) the fine walk from the position to the
  point of the site's flag is ordered at progress 0: TRN-FREE-5.1's straight lattice
  line from that position, without stopping short; the excess of the work walk's last
  step is dropped, so its first step advances from the next tick. The home search comes
  in the tick after that walk's arrival. He is not put back on the sample he left.
  Confidence: high (measured T34, one case; the same "a walk under way is finished
  first" as the approach walk above).
  **A builder still walking to the site** (his road walk from the warehouse) when the
  site is removed goes home: from T, the tick after the removal, he shows the
  going-home state, and his walk ends **on the next flag it reaches** (as 6.5 (g)), not
  at the end of the step in progress (corrected 2026-10-29, T38). That flag's tick is
  the arrival tick A; he stands in A and A + 1, and his walk home starts at progress 0
  in A + 2. He does not walk on to the site. In T37 the step in progress ends on a
  flag; in T38 he walks on over a road field to the flag beyond (measured T37, T38,
  added 2026-10-29).
- **TRN-FREE-6.4c A walk with no step still takes a tick (2026-10-26 night, Q-366 b).** A
  walk ordered to the field the settler already stands on has no step; it ends in the
  settler's movement update of the **next** tick, and what follows its end is ordered
  then (its first step at progress 0, moving from the tick after). Two cases measured:
  (1) the home search (6.4 step 1, 6.4a (c)) of a settler standing on the chosen
  warehouse's own flag orders the road walk to that flag: the settler stands on in that
  tick, and in the next the walk's end (6.4: the warehouse north-west of the flag) orders
  the step onto the building's field at progress 0; (2) entering a warehouse whose door
  is the building's own field (the harbour), the one-step walk to the door field of
  `ECO-worker-cycle.md` ECO-WORK-3.8b has no step: the settler stands on the building's
  field in its arrival tick and the next and is taken in in the tick after, one tick
  later than through a separate door field. Measured (E-trace-011, T30): the builder
  90415 of the harbour (144, 47), finished in trace tick 6459, walks back to the flag's
  point (145, 46), arrives in 6482, stands in 6482 and 6483, steps towards the harbour at
  progress 0 in 6484, stands on its field in 6507 and 6508 and is gone in 6509. Confidence:
  medium (one settler; the mechanism inferred from the walk-end rule of 6.4, not read in
  code).
- **TRN-FREE-6.5 When the road route is chosen (2026-10-25, Q-354).** A road walk
  holds the fields of one road at a time and chooses its next road at every flag:
  (a) **Start.** A road walk is ordered from the field the walker stands on, which
      must be a flag or a road field; from any other field (a building's field)
      no road walk starts and nothing is kept. A settler leaving a building
      therefore gets its first route only when its road walk is ordered on the
      building's flag (`ECO-worker-cycle.md` ECO-WORK-3.8c in the arrival tick,
      3.8d one tick later for a road carrier), over the roads as they are in that
      tick. Nothing chosen when it left the building is used. The destination (a
      flag, or for a carrier a field of its road) is fixed when the walk is
      ordered.
  (b) **One road.** The search of 6.1 runs from the walker's flag to the
      destination; the walk takes only the first road of the route found, from
      the walker's flag to the next flag of the route. A walk ordered on a road
      field first walks along that road to the one of its two flags that the
      route continues from.
  (c) **At every flag again.** When the walker reaches the last field of that
      road (the next flag) and it is not the destination, the search runs again
      from that flag to the same destination over the roads as they are then,
      and the walk goes on along the first road of the new route **in the same
      tick**, as at any other point of a road (no standing tick; the excess
      progress carries, TRN-MOVE-2.3). A road built during the walk is
      therefore taken from the next flag on if it gives a cheaper route; a road
      removed ahead is avoided if another route exists.
  (d) **A flag on the road.** The same new search runs when the walker steps onto
      a flag in the middle of the road it is walking (a flag set on that road,
      splitting it, after the walk took the road's fields).
  (e) **No route.** When a search from a flag finds no route, the walk ends on
      that flag (what the job then does: 6.3, 6.4 step 4).
  (f) **Parallel roads.** Between two flags joined by more than one road the walk
      takes the walker's own road when it is the carrier of one of them,
      otherwise the shortest of them (smallest length L, TRN-CARRY-5.2; the
      first in the flag's road order on a tie).
  (g) **Every road walk.** The same holds for a worker, builder, leveller,
      soldier or specialist going to its building's, site's or order's flag, for
      going home (6.4) and for a carrier's walk to its road. **Exception, stop at
      the next flag:** when going home starts while the settler is in a road
      walk, and when a carrier's road is taken away during its walk, the current
      walk ends on the next flag it reaches, without a new search there.
      **Measured for a carrier (2026-10-25, Q-359, T25):** it stands on that flag's
      point in the tick it arrives (the step's excess is dropped), its new walk
      (over the roads to the road it now has and on to that road's idle field,
      TRN-CARRY-1.4) is ordered in the next tick at progress 0 and first advances
      in the tick after: the same ticks as a new carrier on the warehouse's flag
      (TRN-CARRY-1.4, T24 there). A split of its road counts as taking it away
      (`TRN-roads-flags.md` TRN-ROAD-4.2).
  Confidence: (a) high (the start lookup; E-trace-006, constructor
  90255: out of the castle at counter 647, job state 0 -> 4 on the castle's flag
  (140, 50) in its arrival tick 691, first road step east to (141, 50) over the
  road built at 660, T17). (b) to (g) medium: a code reading of the road walk
  (E-trn-009: search and first road only, the step-end
  handler: new search at the road's last field and at any flag on the way,
  the same-tick continuation, parallel roads,
  cut back at the next flag; the stop-at-next-flag mark
  set by going home and by the carrier; the
  carrier's road walk to a position, made by the carrier's states; going home's road walk), not yet measured: no
  recording so far shows a route changing at an intermediate flag (T18, T19); (g) for a carrier whose road is split: high (measured, T25).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| step cost level / downhill | 3 | | high | E-trn-002 |
| step cost uphill class 1 / 2 / 3 | 4 / 5 / 7 | | high | E-trn-002 |
| tree / stone field surcharge | +2 | | high | E-trn-002 |
| walkable bits | not 0, 2, 6, 8, 10 | | high | E-trn-002 |
| search direction | target to walker | | high | E-trn-002 |
| search estimate | field distance to the walker | | high | E-trn-002 |
| open list ties | first in, first out | | high | E-trn-002 |
| stop | when the walker's field is labelled | | high | E-trn-002 |
| second try | every field passable | | high | E-trn-002 |
| route tie rule | keep direction, else random | | high | E-trn-002 |
| stop short (target walks) | 1 field; fine: elevation size | | high | E-eco-007; E-trn-002 |
| fine step progress | 0.09 x factor | per tick | high | E-eco-004 |

## Test cases

Map `MP_4P_Fight`, headquarters placed at (27,57) as at game start (its flag
at (28,56)), **with its ground levelled** as at game start
(`ECO-warehouse-construction.md` ECO-HQ-1.2, 4.8): the six neighbours of
(27,57) take its height 492; the flag field (28,56) falls from 1257 to 492
(Q-118). Routes computed with these rules; the
walker starts on the HQ flag. "Ticks" is the time from starting the walk to
reaching the last field walked. "Routes" counts the routes TRN-FREE-3.2 can
produce over all six starting draws (TRN-FREE-3.2a).

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | as above | free walk (28,56) -> (33,60) | label 26; one route, 7 steps: (28,56) (29,56) (29,57) (30,58) (31,58) (32,58) (32,59) (33,60); 195 ticks (the first step climbs from 492 to 888: cost 5, factor 0.6) |
| T2 | as above | free walk (28,56) -> (24,62) | label 30; one route, 8 steps via (27,56) (26,56) (25,57) (25,58) (24,59) (24,60) (23,61); 219 ticks |
| T3 | as above | free walk (28,56) -> (36,58) | label 39; one route, 9 steps via (29,56) (29,57) (30,58) (31,58) (32,58) (33,58) (34,58) (35,58); 260 ticks |
| T4 | as above | free walk (28,56) -> (28,48) | label 33; one route for every starting draw (5 paths descend through the labels; TRN-FREE-3.2a): 8 steps via (28,55) (29,54) (29,53) (30,52) (29,51) (29,50) (28,49); 278 ticks |
| T5 | as above, tree at (30,62) | woodcutter walk to the tree, stop short 1 | label 20; one route (4 label paths): (28,57) (29,58) (29,59) (30,60) (30,61); the walker stops after 5 steps on (30,61), 112 ticks; then the fine approach |
| T6 | as above | free walk (28,56) -> (34,57) | label 33; one route, 7 steps via (29,56) (30,56) (30,57) (31,57) (32,57) (33,57); 230 ticks; with stop short 1: 6 steps, 202 ticks |
| T6a | as above but without levelling (map heights) | free walks of T1, T3, T6 | labels 24, 37, 31; 177, 242, 212 ticks (for checking the levelling: these are **not** the game's values) |
| T7 | a walker inside the HQ (27,57) | walk to its flag (28,56) | one step without search (TRN-FREE-2.2); 23 ticks on level ground |
| T8 | walker and target neighbours | walk | one step, no search |
| T9 | flat open ground, walker and target 6 fields apart in a straight line, no trees | free walk | label 18, 6 steps, 134 ticks; the route keeps one direction if the starting draw picks it, otherwise it still has 6 steps |
| T10 | as T9 but every field between them carries a tree, and a tree-free detour of the same number of fields exists | free walk | the detour (each tree field left costs 2 more) |
| T11 | target surrounded by water on all sides | free walk | first try fails; second try crosses the water |
| T12 | walker halfway through a step from A to B gets a new walk to C | new walk | route from A (the field last reached); the half step is lost |
| T13 | woodcutter on the field next to a tree whose fine position is 3 lattice steps away, elevation size 1, level | fine approach | 2 lattice steps, 23 ticks |
| T14 | flat ground; labels such that at field P both directions 1 and 2 are allowed and the walker reached P by a step in direction 1 | route out | direction 1 is taken, no random number used; only on reaching P by a step in a direction not allowed there is a number drawn |
| T15 | Roman woodcutter hut at field (10,11) (odd row), flag (11,10) with sample (44,40) | construction positions 0..3 | samples (43,42), (41,41), (42,41), (43,43) (offsets (-1,2), (-3,1), (-2,1), (-1,3)); same hut for an African owner: (41,41), (42,43), (43,43), (42,43) |
| T16 | as T13: the woodcutter stopped on field F, walked 2 lattice steps to his stand (23 ticks) and has worked and taken the log; taking ends in tick E | walk home | E: no movement; E + 1 .. E + 23: fine walk back 2 lattice steps to F's point (arrives in E + 23); E + 24: first progress of the free walk from F to the flag; at no tick is he on a sample or field not on these two lines (5.6) |
| T17 | E-trace-006 (TOOL-trace), counter 647: constructor 90255 leaves the castle (field (139, 52)) for the site 90244 at (139, 47); the computer player builds the road "140,50 141,50 141,49 141,48" at counter 660, while he is still on his way to the castle's flag | his road route (6.5 a) | chosen on the castle's flag (140, 50) in his arrival tick 691 (job state 0 -> 4), over the roads of tick 691: the first road step goes east to (141, 50) over the 660 road (a route chosen at 647 would go north-west over (139, 49)) |
| T18 | flags A, B, C, D; roads A-B, B-C, C-D of length 2 and B-D of length 6; a walker on A gets a road walk to D; while he walks A -> B the road C-D is removed | road walk (6.5 c) | on reaching B (not the destination) the route is searched again: B-D (6); he goes on over B-D in the same tick, without standing on B, and never enters B-C |
| T19 | as T18, but B-D does not exist | road walk (6.5 e) | the search on B finds no route; the walk ends on B |
| T23 | E-trace-005 / 007, the sawmill site's leveller 90231 on (139, 48): his last levelling action (code 4) counts down to 1 in counter 964 | his ticks | 965 going home, standing; 966, 967 standing; walk home at progress 0 in 968 towards (139, 49), 0.045 in 969 (6.4a: T = 965, walk in T + 3) |
| T24 | E-trace-005 (TOOL-trace), builder 90202 at the hunter site: his last work action ends and the building is complete in counter 1593; he stands in fine mode at (563, 195) | his ticks | 1593 going home, standing; fine walk from 1594 at progress 0 towards his target's point (564, 192); standing there in 1628 (arrival); road walk home from (141, 48) at progress 0 in 1629 (6.4a) |
| T25 | 6.5 (g), E-trace-007/008, counter about 1185: new carrier 90315 sent from the castle for road 90311 "139,56 140,56 141,56 141,55 142,55" (built 1163) walks over road 90277 from the castle's flag (140, 50) towards (142, 55); at 1308 the stone pit's flag (141, 56) splits 90311 into "139,56 140,56 141,56" and "142,55 141,55 141,56" while it walks on 90277 (TRN-ROAD-4.2: it gets the second half) | its ticks at (142, 55) | 1349 walking on (142, 54) at 0.99; 1350 standing on (142, 55)'s point; 1351 walk ordered, progress 0; 1352 moving towards (141, 55), its new idle field, reached and standing in 1374. Carriers that only pass (142, 55) on their way (90341 at 1482, 90456 at 2990) walk on with the excess, as in 6.5 (b) |
| T26 | 6.4 steps 1, 2, E-trace-007/008: carrier 90261 of road 90252 "140,46 140,47 141,48" carries a plank into the hunter site (139, 47); at counter 1541 the computer player removes 90252 and makes the same road again (AI-CON-8.1 b strategy 3), so the carrier is sent home while walking in | his way home | in the site from 1559 (two ticks standing), then a free walk to the flag (139, 48) (network flags at distance 1: (139, 48), made at 323, and the site's own flag (140, 46), made at 596; the first wins), from progress 0 at 1561; then the road walk (139, 49), (140, 50) and into the castle (139, 51) at 1690. Not over (140, 46) and the road "140,46 139,46 138,47 139,48" |
| T27 | 5.6 steps 1 to 3, ECO-OUT-3.8: E-trace-010 (dense trace), woodcutter 90478 of the hut (136, 47), flag (137, 46); the tree on (136, 45) next to the flag (the free walk to it had no field, F = the flag field); his fine approach ends on (546, 180), the point of (136, 45), in trace tick 3940; the take ends in 4000 | walk home | 4001 standing (the wait); 4002 the free walk from (136, 45) to the flag: one field step, first movement in 4003 (546090, 180180), 90 x 180 per tick, on the flag (548000, 184000) in 4025 (measured); then put down 4026. Not the fine walk of 4 sample steps to the flag's point (45 ticks, an experiment build's earlier walk). Woodcutters 90392 (2826) and 90478 (3506) stood off a field's point: the fine walk to the flag's point after the standing tick (measured) |
| T28 | 6.4 steps 1 to 3, 6.4a: E-trace-010, the hunter 90425 resting inside his hut (139, 47) when the computer player tears it down at counter 4791 (`MIL-conquest.md` T17); the hut's flag (140, 46) (flag 90245) and the sawmill's flag (139, 48) (flag 90214) both 1 field away, both in the castle's network | going home | trace tick 4792: still resting (unchanged); from 4793 (T) the going-home state, standing 4793 .. 4795; 4796 the free walk from (139, 47) towards (139, 48) at progress 0 (step 2: no road walk from the building's field; the tie between the two flags goes to 90214, made first; first movement (557945, 188108) in 4797), then the road walk home from (139, 48) (measured). Starting T in the removal tick puts every step one tick early; a road walk from the hut's field over its own flag goes to (140, 46) |
| T29 | 6.4b: E-trace-011, player 1's hunter site 90716 on (141, 46) (flag (141, 45)) in its main stage, its builder 90731 in a work action on sample (563, 181); the computer player tears the site down at counter 6226 | the builder's rows 6226 .. 6263 (trace ticks = counter + 1) | trace 6227: working; 6228: going home, work animation still running, same sample; 6229: Walk at progress 0 from (563000, 181000); arrival on (566000, 180000), field (141, 45), in trace tick 6263, the walk home from there (measured) |
| T30 | 6.4c: E-trace-011, player 1's harbour (144, 47) finished in trace tick 6459; its builder 90415 on his last work sample, home = the harbour (the nearest warehouse by road cost: its own flag) | his rows to 6509 | fine walk back to (145, 46)'s point arriving in 6482 (Stand 0), Stand 1 in 6483, the step into (144, 47) at progress 0 in 6484 and moving in 6485, on (144, 47) Stand 0 / 1 in 6507 / 6508, gone in 6509 (measured). Without the two ticks an experiment build walks in 6483 and is gone in 6508; with them its dense settlers match to 7419 and its 6000-tick trace stays the same |
| T31 | 6.1: E-trace-011, player 1's builder 90640 going home from the flag (139, 56) to the castle's flag (140, 50) at about counter 9600; two routes of length 9: east (139, 56) -> (141, 56) -> (142, 55) -> (141, 52) -> (140, 50) over roads 90332, 90335, 90601, 90604, west (139, 56) -> (138, 54) -> (137, 52) -> (137, 50) -> (140, 50) over 90497, 90527, 90579, 90954 | his walk | at (140, 50) the first road in its order on a cheapest route is 90604 (attached before 90954): the east route, fields (140, 56), (141, 56), (141, 55), (142, 55), (142, 54), (141, 53), (141, 52), (140, 51), (140, 50), standing on (139, 52) at trace tick 9848 (measured). Deciding the tie by the lower flag field takes the west route over (138, 55) (the same arrival tick, different fields from 9602) |
| T32 | 6.4 (the end of the walk): E-trace-011, player 1's hunter 90379, its hut 90190 (140, 49) torn down at counter 10162 (AI-CON-9.2); it free-walks from the hut's field to the castle's flag (140, 50) (6.4 step 2) | its arrival at the flag | on (560000, 200000) in trace tick 10223 with the walk running on, (559729, 200540) in 10226: the step towards the castle moves from 10224, no standing tick (measured). An experiment build that runs the home search again from the flag (which, standing on the warehouse's own flag, holds a tick as in 6.4c (1)) stands in 10223 and moves from 10225 (settlers first differ at 10224; without the stand at 10339) |
| T33 | 6.4b (the approach walk): E-trace-011, player 1's hunter site 91151 on (142, 58), its builder 91177 arrived on the flag (142, 57) at counter 11045 and walking his approach from the flag's sample (570, 228) since 11046; the computer player tears the site down at counter 11064 (he is between (570, 229) and (571, 230)) | the builder's rows 11064 .. 11140 (the recording's settlers table; trace ticks = counter + 1) | going-home state from 11065 while he walks on: (571, 230) at 11069, (571, 231) at 11080, back over (571, 230) and (570, 229) to the flag's point (570, 228) in 11114; the walk home from the flag field (towards (143, 56)) begins in 11115. Turning him back at once (an experiment build's 6.4b as for a work action) puts him on the flag's point at 11078 and home-bound from 11079: settlers first differ at 11065 and the generator at 11116; holding him until the approach would end moves the generator to 11395 (approximate: standing on the flag's point meanwhile) |
| T34 | 6.4b (a work step's walk): E-trace-013 / E-trace-012 (Hard), builder 91166 of the hunter's hut site on (142, 44), torn down by the computer player (its last row counter 8809); the Roman hunter's hut's construction positions from the flag's sample (570, 172) are 0 (569, 174), 1 (567, 173), 2 (570, 175), 3 (569, 175) | his walk home | from counter 8800 the walk from position 0 to position 1 over (568, 174), its first step (569, 174) -> (568, 174) at 0.09 per tick (0.81 in 8809); going-home state from 8810; 0.90 (8810), 0.99 (8811), on (568, 174) in 8812 with 0.08 already into the step to (567, 173) (0.054 per tick); on position 1 (567, 173) in 8830, where the straight line to the flag's point (570, 172) is ordered at progress 0 (0.09 in 8831): (568, 173) in 8842 (0.08 carried), (569, 172) 8853, standing on (570, 172) in 8864; his walk home at progress 0 in 8865 over (143, 44), (143, 45), (144, 45) to the harbour's flag (145, 46). Experiment build (the walk to position 1 finished, then the line): his rows equal the recording in every tick 8809 .. 8868; Hard pair buildings 9324 -> 9780, generator 8854 -> 9220, the other pairs unchanged |
| T35 | 6.4 step 1 (the route cost): E-trace-014 (R4, the Weak game), builder 91718 of player 1 (out of the harbour (144, 47) at 18080) going home from the north, on the guard house's flag (137, 37) at counter 22492 | the warehouse he walks to | the castle (139, 51): (136, 37), (136, 38), (135, 39) .. (137, 44), (137, 47), (139, 48), (140, 50), in at 22872 (16 steps); the harbour's route (138, 38) .. (141, 44), (144, 45), (145, 46) is 14 steps. Our build compared the walks' field counts and took the harbour. Experiment build (road lengths L): the castle; the Weak game against R4 settlers 22493 -> 22604, the other pairs unchanged |
| T36 | 6.4 step 1 off a flag: E-trace-014 (R4, the Weak game), carrier 92103 standing on (134, 29), his road (133, 30) - (133, 29) - (134, 29) - (135, 30) removed by the street work of counter 23388 (`AI-construction.md` T75) | the warehouse he walks to | his walk to (135, 30), then road 91951 (L 16) to (137, 37); from there the castle's flag (140, 50) costs 30 (over (137, 44), (137, 46), (139, 48): the west road 91774 L 16 first) and the harbour's (145, 46) 32 (over (139, 43), (141, 45), (143, 45)); L read from the road in the heap dump 23500. He goes west at (137, 37) towards the castle (trace tick 23613 on (136, 37)). Counting fields our build took the harbour and turned at (137, 37) to (138, 38): settlers first differ at 23591; with the rule at 23921; the Hard pair and the Weak pairs to 20 000 unchanged |
| T37 | 6.4b, the builder on his way: E-trace-014 (R4, the Weak game), player 1's builder 92803 walking west on (131, 58) towards the site 92768 on (125, 59); the computer player removes the site at counter 29145 (its last row) | his ticks | going-home state from trace tick 29147, still in his step towards (130, 58) (x 520683); on (130, 58)'s point in 29151 (A), standing in 29151 and 29152, walking home (east) at progress 0 in 29153 (recorded). Walking on to the site and going home from there (as before): the Weak game against R4 differs from settlers 29151; with the rule from 29187 (experiment build) |
| T38 | 6.4b, the walk ends on the next flag: E-trace-013 (R3, the Hard game), player 1's builder 92275, walking west from the castle's flag (140, 50) over the road field (139, 50) towards the flag (138, 50) when his site is removed (going-home state from trace tick 17298, x 557299) | his ticks | on (139, 50) without stopping; on the flag (138, 50) at 17328 (A), standing 17328 and 17329, walking home (east) at progress 0 in 17330 (recorded). Ending the walk at the step in progress, on (139, 50) at 17306, differs from the Hard game against R3 at settlers 17306. With the rule: Hard settlers (animation ignored) 17632, R3 17640 (experiment build; the Weak pairs unchanged) |
| T39 | 6.1 (a road field as destination): E-trace-013 (R3, the Hard game), player 1's new carrier 92297 of road 92295 "139,58 140,58 140,59 141,60" (idle field (140, 59)), on its road walk from the castle, reaching the flag (141, 56) at trace tick 17631 | the road it takes on | west along road 90348 towards (139, 56), on to the road's first flag (139, 58) (recorded: on (140, 56) from 17655). By the search's road lengths both flags cost 8 from (141, 56); counting the road's part to the idle field gives 12 against 10 (via (141, 60)), and counting fields gives 6 against 5. Either way the walk would go south to (141, 57), and the Hard game against R3 differs from settlers 17632. With the rule: R3 settlers 18580 (experiment build; the Weak pairs and the rest unchanged; taking the first flag always differs from 850). **Corrected 2026-11-01 (Q-378, T42):** under 6.1 as now written the first flag costs 8 and the route over (141, 60) 8 + 1 (the carrier's own road at 1): the first flag |
| T40 | 6.4a (a donkey of the donkey breeding): E-trace-014 (R4, the Weak game), player 1's donkey 93642 out of the donkey breeding 92529 on (141, 35) (door (141, 36), flag (142, 34)), going home (it reaches (144, 47) in 35230) | its rows at the flag | its step onto (142, 34) ends in counter 34880 (job state 23 to 34879, the going-home state 3 from 34880); it stands on the flag's point in 34880, 34881, 34882; the walk home shows progress 0 in 34883 and 0.045 in 34884. Ordering the road walk in the arrival tick (as a settler whose walk to a site ends on its flag, ECO-WORK-3.8c) is three ticks early: settlers first differ at 34890 (with T-door-5); with the rule (experiment build) at 35310 (buildings 35820 -> 35830, generator 35240 -> 35360, the 150 000 generator 35232 -> 35358; the other pairs unchanged) |
| T41 | 6.1 (fields, not cost): E-trace-013 (R3, the Hard game), player 1's new carrier 93134 of road 93132 "137,60 136,60 135,59" (idle field (136, 60), made at counter 23082), out of the castle onto (140, 50) at 23130 | the end it heads for | the routes cost 29 to (137, 60) (over (137, 58): roads of L 4 and 5) and 28 to (135, 59) (over (135, 57): L 4 and 4; the L values as the heap dump of 23000 holds them), but both are 14 fields long from (140, 50) (and equal at every flag on the way, 4 and 4 from (137, 56)): it heads for the first flag (137, 60) and walks (137, 57) .. (137, 60) to (136, 60), there at 23470 (measured). Choosing by cost turns west at (137, 56) (the experiment build's settlers first differ at 23360). Carrier 90373 of the Weak game (E-trace-005 / 014, road "140,46 140,47 141,48", counter 1649 at (140, 50)): costs 11 and 8, fields 5 and 3: the second flag (141, 48), as recorded (heading for the first flag differs from 1651). With the rule: R3 settlers 24640 (buildings and generator unchanged; every other pair unchanged). **Corrected 2026-11-01 (Q-378, T42):** the outcome is the same under 6.1 as now written, but not because of the field counts: the carrier's own road costs 1, so the route to (137, 60) costs 29 directly and 28 + 1 over (135, 59); on that tie the road into (137, 60) attached there first, the one over (137, 58), wins |
| T42 | 6.1 (the walker's own road costs 1; Q-378): four measured walks to a field of the walker's own road | the end it heads for | (a) E-trace-014 counter 1649, Weak carrier 90373 from the castle's flag (140, 50) to road 90366 "140,46 140,47 141,48" (A = (140, 46), B = (141, 48)): to A over (139, 48) 7 + 4 = 11, over B 8 + 1 = 9, so it ends at B and leaves east (recorded: on (140, 50) with a step to (141, 50) at progress 0 at counter 1649); taking A as the goal with the road at its length (12) sends it west and differs from trace tick 1651. (b) E-trace-018 (R5) counter 48438, Weak donkey 95715 from storehouse 94385's flag (134, 38) to road 91048 "142,43 143,44 143,45": A = (142, 43) costs 34 directly, over B = (143, 45) 36 + 1 = 37, so A, by (135, 37) and (137, 37), the same at each of those flags (recorded); counting fields (16 against 15) chose B, and the Weak game vs R5 differed at settlers 48528. (c) T39: 8 against 8 + 1, so A. (d) T41: 29 against 28 + 1, equal: the older road into A wins, so A. With the rule (experiment build): Weak settlers vs R5 48528 -> 49093 (trace ticks); every fidelity pair unchanged, the Hard R3 settlers 39510 included |
| T43 | 6.1 (a walk from a road field), `TRN-carriers-transport.md` TRN-CARRY-7.1a, 7.1b: E-trace-019 (R6, MP_4P_Fight), player 1's carrier 14758 of the road 14720 "53,59 54,59 55,58 55,57 55,56 54,55 55,54 54,53 53,53", on his way to its idle field (55, 56) when the computer player removes the road and builds 14893 "56,58 55,57 55,56 54,55 55,54 54,53 53,53" (first flag 14802 (56, 58), the other 14275 (53, 53); table tick 1702) | his way home | he walks on to (55, 56) (on 14893), stands there in counters 1751 and 1752 (A, A + 1), and in 1753 his road walk home starts at progress 0 towards (55, 57): (56, 58), then (55, 58), (54, 58), (53, 59) (recorded). Choosing the end of 14893 that gives the shorter walk sends him back towards (53, 53): the Fight game against R6 differs from settlers 1755. With the rule (experiment build, with T34 of `TRN-roads-flags.md` and `ECO-requests-dispatch.md` T39): settlers 1755 -> 2325; every pair of the fidelity set unchanged |
| T44 | 6.4 step 4 (Q-1034): a homeless private (health 3, no warehouse in reach) | his going-home state | after each 52nd failed search (count 51 when the wander step ends) health 3 -> 2 -> 1, count back to 0 each time; at the 156th failure health 0: the state ends, he dies (TRN-CARRY-7.1). A carrier in the same place dies at the 52nd. No generator draw at those points |
| T45 | 6.3a: a guard house that wants 1 soldier; its soldier's road walk ends short at a flag 3 fields from it, so he goes home (6.3); no other soldier is linked or requested | the allocation passes while he walks home, then his arrival | no soldier request while he walks (wanted 1 - linked 1 - open requests 0 = 0); he is taken into the warehouse's stock and unlinked in the tick after his walk home ends; the first allocation pass after that files one soldier request |
| T46 | 6.3a: a site with work left whose builder's road walk ends short (he goes home); his walk home ends in tick T at a warehouse | the site's builder request | none while he walks home; he is taken into the stock and unlinked in tick T + 1 (settlers' update); the site files a new builder request in tick T + 2 (its building update) |
| T46a | 6.3a Worker (Q-1056): a woodcutter's hut finished in tick F (its worker timer fires in F + 101, F + 202, ...); the first request's woodcutter is sent at F + 101, his road walk ends short and he goes home; his walk home ends in tick T at a warehouse, with F + 303 < T + 1 < F + 404 | the hut's worker requests | no request at F + 202 or F + 303 (he is linked and counted); he is taken into the stock and unlinked in T + 1; the next request is filed at F + 404 (the timer's next firing), not at T + 2; had T + 1 been F + 404 itself, the firing then would still count him and the request would wait until F + 505 |
| T47 | 6.4b, the approach's back leg: E-trace-019 (R6, MP_4P_Fight, three Hard computer players), player 3's hunter's hut site on (33, 22) (flag (33, 21), flag point sample (134, 84)) removed after counter 8306 while its builder walks the back leg from (135, 87) (at counter 8306 on (135, 87), progress 0.18) | his rows (counters; trace ticks = counter + 1) | no new walk: on (134, 85) -> (134, 84) at 0.97 in 8337; on the point (A) in 8338; standing 8338 and 8339; walk home (free walk, first step to (33, 20), its 2 read-out draws) at progress 0 in 8340, 0.045 in 8341. The other back-leg cases of the recording: counters 8416 -> 8418, 9320 -> 9322, 13460 -> 13462, 15388 -> 15390; out-leg or work-walk cases (a new walk after the removal): 9459 -> 9460, 10874 -> 10875, 12749 -> 12750 among 12. A build starting home in A + 1 here differs from the fight game's generator at trace tick 8340; with the rule at 8456 (experiment build; every MP_2P_Storm_Coast pair unchanged) |
| T47a | 6.4b, the road cut under the walk to the site: E-trace-019 (R6, the fight game), player 1's builder 17095, sent at counter 8923 to the hunter site on (41, 60), walking along the road (46, 59) .. (41, 59) to the site's flag (41, 59) | the site removed in counter 9342 (he goes home from 9344 and walks on toward the flag); the flag (41, 59) and that road removed in counter 9366, while he walks the step (44, 59) -> (43, 59) | he ends that step on the point of (43, 59) in counter 9369 (174000, 236000), stands in 9369 and 9370, and in 9371 searches home (2 draws) and walks back east; he never reaches (42, 59). Walking on to the old flag's field (41, 59) makes the fight game's generator differ from trace tick 9371; with the cut it moves to 9801 (experiment build) |
| T47b | 6.4b (a walk home cut): E-trace-019 (R6), player 1's carrier 17758, idle on (55, 43) of road "54,43 55,43 56,44" until the computer player removes that road at counter 11811 (with the site on (56, 42)); he goes home (TRN-CARRY-7.1a, 7.1b): Walk 0 at trace tick 11816, a free walk to the flag (56, 44) (on its point at 11839, excess dropped), then the road walk home from there over road "55,46 55,45 56,44" from 11840 | the computer player removes that road at counter 11851, while he is in his step (56, 44) -> (55, 45) | he finishes that step: on (55, 45)'s point exactly at trace 11862 (fine (222000, 180000)); a free walk to the flag (55, 46) at progress 0 in 11862 (221909 at 11863: no standing tick); on (55, 46)'s point at 11885, the road walk east from it from 11886 (recorded). Walking on through (55, 45) with the excess carried (no cut) puts him at 221929 in 11862. Experiment build (the cut and the choice in A for a carrier walking home; AI-MIL-4.4a on): the war's settlers (animation aside) first differ at 12172 instead of 11862 |
| T48 | 5.3 (each step's factor when it starts): E-trace-018 (R5, the Weak game, every tick from counter 40 000), worker 95429 on his fine approach from the sample (482, 132) (field (120, 33)) over (481, 133), (481, 134), (480, 135) to (480, 136), ordered at progress 0 at trace tick 52 267, all five samples level (7499 / 7498) then; the site on (121, 34) is levelled in counter 52 289 and its heights table gives (482, 132), (481, 133), (481, 134) 7056, (480, 135) 7151, (480, 136) 7498 from then | his approach and what follows | steps 1 to 3 at 0.09 per tick (the third begun in counter 52 289 at 0.07: no rise worth a class); the fourth, begun in counter 52 300 (trace tick 52 301, carry 0.06), at 0.072 per tick (the rise 7151 -> 7498, class 1): on (480, 136) working from trace tick 52 315 (48 ticks); put-down 52 370, pick-up 52 374, the walk back from 52 378, the free walk home from (482, 132) at progress 0 at trace tick 52 423 (its read-out draws there). Fixing the approach's length when it is ordered (45 ticks at the level heights) has him working from 52 312, three ticks early, and the walk home's draws at 52 420: the Weak pair's generator apart from trace tick 52 420; with the rule (2026-11-21 day, experiment build on 0.12.3: the approach's and the return's lattice steps each at the rate of the heights when the step starts, the work cycle's remaining time following the heights as they are now): **56 470** (matching 56 579), every other pair unchanged |
| T49 | 6.4a, T for a territory removal: E-trace-019 (R6, the war), the take-over of player 3's (31, 21) in counter 13448; the buildings of the lost land (last recorded in 13448) go in 13449 | the workers standing in them (15139, 15785, 15961, 16274, 16444, 16462, 16630) | the going-home state from 13449 (T), standing 13449, 13450, 13451, the walk home at progress 0 in 13452. T = the tick after the removal: war generator first differs at trace tick 13453 (36 draws of the walks home recorded, 21 ours); with the rule 13476 (experiment build on 0.12.3 with MIL-ATK T36, T37, TRN-CARRY T61; settlers and buildings unchanged; every Storm Coast pair unchanged). Applying T = the removal tick to every removal before the settlers' part (the computer player's demolitions included) moved the Weak 40 000 settlers to 23656 and Hard 20 000 to 9600. Whole recording: after each of the 9 take-overs every worker of the lost land (34) shows the going-home state the tick after the take-over |
| T50 | 6.4a, a worker sent home on a free walk: E-trace-019, player 3's woodcutter 18538 walking back to his hut's flag, path (26, 27) (27, 26) (28, 26) (28, 25) (29, 24) (28, 23), at progress 0.709 of the first step in counter 13449 when his hut goes (T49) | his walk | he walks on without a stop (on (27, 26) in 13458 with the next step under way, progress 0.033) to (28, 23), arrival 13579 (A), standing 13579 and 13580, his walk home at progress 0 in 13581. Stopping after the step in progress: war settlers first differ at trace tick 13460 (with T49); with the rule 13470 (experiment build; buildings and generator unchanged; Storm Coast pairs unchanged). Whole recording: three more workers sent home walking (15750, 22182 at counter 25259, 24714 at 37159) walk on and turn home at A + 2 |
| T51 | 6.4a, a reinforcement on his walk out of the warehouse: E-trace-019 (R6, the war), player 3's soldier 18673 out of the warehouse (25, 25) since counter 13419, walking to its flag (26, 23) for (31, 21), which is taken in 13448 | his ticks | trace state 23 (the walk out) through 13464; on the flag (26, 23) in 13465 (A) with the going-home state; standing 13465 .. 13468; his walk home (26, 23) -> (26, 24) at progress 0 in 13469. Sending him home at the take-over (he walks the step to the flag and walks home at A + 3): war settlers first differ at trace tick 13470; with the rule 13480 (experiment build on 0.12.3 with T49, T50 and `MIL-attack-combat.md` T36, T37, `TRN-carriers-transport.md` T61; every Storm Coast pair unchanged) |
| T52 | 6.4, all three tries: E-trace-019, player 3's woodcutter 16444 (sent home 13449, T49) free-walking to the network flag (27, 27), which loses its last road in counter 13468; his walk ends there in 13475. Builder 17035 of a site of the lost land, home search in 13504 on the flag (32, 20), now player 2's | their walks | 16444: in 13475 a free walk (27, 27) -> (27, 26) -> (27, 25) -> (27, 24) -> (26, 23), the warehouse (25, 25)'s flag, at progress 0 (the trace tick's 3 draws); in the warehouse at 13611. 17035: in 13504 a free walk (32, 20) -> (31, 20) -> (30, 20). The road try alone, then the wander step (one draw): the war's generator first differs at trace tick 13476 (16444) or 13505 (17035, with 2 and 3 only at a walk's end; buildings then 13890); with the rule settlers 13480 -> 14100, buildings 14320, generator 13476 -> 14092 (experiment build, with T51; every Storm Coast pair unchanged) |
| T53 | 6.4b (a work walk ending in T): E-trace-019 (R6, the war), player 1's site (62, 39), removed with land lost to a take-over in trace tick 17 544 (T); its builder 19138 on his work walk from (250 000, 159 000) to the construction position (250 000, 158 000), which he reaches in 17 544 | his next walk | standing on (250 000, 158 000) in 17 544; the line onward at progress 0 in 17 545, moving from 17 546 (recorded: (250 000, 157 549) in 17 550, walk progress 5). Ordering it in 17 544 puts him 90 units ahead from 17 545 (the war's settlers apart from 17 550); with the rule (experiment build) from 17 620 |
| T54 | 6.4b (the road removed under the last step, onto a flag that stays): E-trace-019 (R6, the war), player 2's builder 20421, on his road walk home that starts on the flag (44, 13) (his fine walk back from his last construction position ends there in counter 19971; the road walk from progress 0 in 19972) over the road "44,13 43,13 43,14"; the flag (43, 14) also ends the road "43,14 43,15 43,16 43,17 44,18", his next | the road "44,13 43,13 43,14" is removed after counter 20015, while he walks the step (43, 13) -> (43, 14) (a rise of class 1: 0.036 per tick; progress 0.971 at counter 20021) | at counter 20022 he is on (43, 14) and already in the step (43, 14) -> (43, 15) at progress 0.0070004 (the excess carried; level, 0.045 per tick), 0.367 at counter 20030; no stop on the flag. Cutting the walk there and choosing home again at progress 0 (the T47b rule applied to this step) puts him 0.007 of a step behind: the war's settlers first differ at trace tick 20030 (fine (172629, 57259) against the recorded (172644, 57288)); without the cut at 20480 (experiment build on top of the fifty-one rules; every Storm Coast pair, the one-player game and dense unchanged) |
| T55 | 6.4a (a walk ordered, not yet begun): E-trace-019 (R6, the war), player 2's farm (40, 19) and its farmer 21749, in since counter 23633 (door field (40, 20)); his sowing step's wait ends and he finds a spot in counter 23684: the walk out is ordered, its first step (40, 20) -> (40, 19) at progress 0; the farm goes in counter 23685, before the settlers' part (T = 23685) | his ticks | going-home state from 23685 with that step running: 0.045 in 23685, on (40, 19) in 23707 (A), standing 23707 and 23708, the walk home (a free walk over (41, 19), (42, 20), (42, 21) to the network flag (43, 22)) from progress 0 in 23709 (A + 2). Taking him as standing on the door field (walk home from (40, 20) at 23688) makes the war's settlers differ from trace tick 23690; with the rule 24290, generator 23710 -> 24288 (experiment build on top of the fifty-one rules and T54, ECO-DIST T25) |
| T56 | 6.3 carrier (Q-046): a carrier assigned to a road whose flags it cannot reach by road from where it stands (the road joining them was cut behind it), no ware waiting at either flag | its updates over 5 ticks; then a ware is put at one of the road's flags | each update orders the road walk to its idle field again (one search per update) and it does not move; it stays linked to the road (no go-home); after the ware appears it starts a road walk for it (which fails the same way while no route exists); with the road removed it goes home in the next update |
| T57 | 6.4a (a builder on his walk out, the reinforcement rule of T51): E-trace-038 (MP_6P_the_others), player 2's builder 19537 leaving the headquarters (door field (92, 83), building field (93, 82), flag (93, 81)) for the site (100, 84); the site is removed at counter 4 663 (its rows end between 4 660 and 4 670) while he is in the door step | his rows from trace tick 4 650 | the door step goes on (371 170, 329 659 in 4 660); on (93, 82)'s point at progress 0 in 4 670 (the excess dropped: the door step is a walk of its own, ECO-WORK-3.8b), the step to the flag to (373 800, 324 399) in 4 690, on the flag in 4 693 (A), going-home state from A, standing in 4 693 .. 4 696, walking back at progress 0 in 4 697 (373 729, 324 540 in 4 700). Sending him home at the removal fine-walks him on with the excess (on the flag in 4 692) and home from 4 695. Experiment build: the game's settlers 4 670 -> 5 500; every other pair unchanged |
| T58 | 6.4a (T49's land-loss exception for a builder): E-trace-038, player 4's builder 25709 hammering on (70, 31) (285 000, 125 000) at the site 24969 on (71, 32); the site's flag is on land lost in the tick of counter 16 686 (the roads' part, before the settlers' part) | his rows | going-home state from 16 686 (T); standing in 16 687; the fine walk back to the flag's point at progress 0 in 16 687 (trace 16 688), (285 180, 124 819) in trace 16 690; on the point (286 000, 124 000) in 16 700, walking home from 16 701 (284 379 in 16 710). T = the tick after the removal puts every row one tick late. Experiment build: the game's settlers 16 690 -> 18 530, generator 16 701 -> 19 251; every other pair unchanged |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-mil-009 | code reading | session 2026-11-09 | TRN-FREE-6.4 step 4, T44; Q-1034 |
| E-sim-008 | code reading | | TRN-FREE-6.4 (corrected step 2, the end of the walk) |
| E-trn-002 | code reading | `S2DNG.exe` | free walk, retry, road walk entry |
| E-trn-002 | code reading | | start of a walk; path modes (free / road) |
| E-trn-002 | code reading | | free path: building-flag shortcut, search, path out with random ties |
| E-trn-002 | code reading | | walk search object; step cost; height class order |
| E-trn-002 | code reading | | search driver, expansion, labels, sorted open list |
| E-trn-002 | code reading | | route from labels, direction choice |
| E-trn-002 | code reading | | step completion, stop short check |
| E-trn-002 | code reading | | fine walk: straight lattice line |
| E-gfx-015 | code reading | | walk home after a fine approach (5.6) |
| E-trn-002 | code reading | | road walk: flag network route, worker to its building |
| E-trn-009 | code reading | | TRN-FREE-6.5 (road route chosen at every flag) |
| E-trn-002 | script | `deposits.lua` | elevation sizes, fine offsets |
| E-trn-003 | code reading | | deposit lattice offset: draw mod 6, one step |
| E-trn-003 | code reading | | road cost by search mode: penalties only in ware-routing mode |
| E-trn-014 | code reading | experiment build exp4: walk home of trace tick 4001, dense settlers 4003 -> 4218 |
| E-trn-004 | code reading | | route out: current direction kept from the last step; draw only when not allowed |
| E-trn-004 | code reading | | construction positions: flag field sample + per-tribe offset, truncated |
| E-trn-004 | code reading | | building tribe from its owner; setup slot with tribe, team, colour, HQ |
| E-trn-003 | code reading | | going home: warehouse choice, retries, landmass fallback, 30-field limit |
| E-trn-016, E-trace-011 | code reading and recording | E-trace-011 `streets.csv.gz` (roads at 9599) | 6.1 equal routes (T31) |
| E-trn-028 | code reading | | TRN-FREE-6.1, T42 (Q-378) |
| E-trace-011 | recording | `specs/traces/original-dense-20000.s2trace.gz` (hunter 90379, trace ticks 10160 .. 10250); `objects.csv.gz` (remains 91093 on (140, 49) at 10162) | TRN-FREE-6.4 end of the walk (T32) |
| E-trn-032 | code reading | = E-gfx-042, session 2026-11-08 c | 6.4 step 4 |
| E-trn-033 | code reading | | 6.3a: the soldier's, builder's and leveller's removal and its unlink, the military allocation's count, the site's builder and leveller checks (T45, T46) |
| E-trn-034 | code reading | (session 2026-11-15 night) | 6.3: every work routine's arrival check (the worker on the building's field, else home); Q-272 |
| E-trn-035 | code reading | (session 2026-11-15 day) | 6.3a Worker: the worker's link is dropped only by his object's removal or the building's; going home leaves it; the worker request counts linked workers |
| E-trn-037 | code reading and recording | going home's update (wait for the step's end, its fine walk to the target's point, finish, search); E-trace-019's 17 removed-site builders | 6.4b back leg, T47 |
| E-trn-039 | recording and measured | E-trace-019's builder 17095 (rows every tick 9290 .. 9420), the flags and streets tables (flag (41, 59), street (41, 59) .. (46, 59) last seen in counter 9365), an experiment build | 6.4b the road cut under the walk to the site, T47a |
| E-trn-040 | recording, code reading, experiment build | E-trace-019's carrier 17758 (rows of trace ticks 11813 .. 11890; the streets table: the road's last row at counter 11851); the going-home states | 6.4b, a walk home cut (T47b) |
| E-trn-041, E-trace-018 | recording | R5's worker 95429 (settler rows of counters 52 255 .. 52 430) and its heights table (the site (121, 34) at counter 52 289) | 5.3, each step's factor when it starts (T48) |
| E-trace-011 | recording | `settlers.csv.gz` and `buildings.csv.gz` (builder 91177, counters 11043 .. 11116; site 91151 gone after 11063) | 6.4b, the approach walk (T33) |
| E-trn-042, E-trace-019 | recording, experiment build | (session 2026-11-21 day): the war recording's settlers and buildings tables after the take-overs | 6.4a the territory removal's T (T49), the worker's free walk completed (T50) |
| E-trn-044, E-trace-019 | recording, experiment build | (session 2026-11-22 pm): R6's builder 19138 of the site (62, 39) removed with lost land, trace ticks 17 530 .. 17 580 | 6.4b a work walk that ends in T (T53) |
| E-trn-046, E-trace-019 | recording, experiment build | (session 2026-11-24 a): R6's builder 20421 rows every tick 19943 .. 20034, the streets and flags tables (road "44,13 43,13 43,14" last seen in counter 20015) | 6.4b the road removed under the last step onto a standing flag (T54) |
| E-trn-047, E-trace-019 | recording, experiment build | (session 2026-11-24 a): R6's farmer 21749 rows every tick 23682 .. 23712, the farm's last row (counter 23684) | 6.4a a walk ordered, not yet begun (T55) |
| E-trn-049 | code reading | (session 2026-11-26 a): the carrier job's update after its idle step ends (road gone: home; a ware at either flag: fetch or walk; else the idle step again next update) | TRN-FREE-6.3 carrier (T56), Q-046 |
| E-trn-051, E-trace-038 | recording and experiment build | (session 2026-11-27 rec): builder 19537's rows 4 640 .. 4 700 and the site (100, 84)'s rows | TRN-FREE-6.4a, T57 |
| E-trn-054, E-trace-038 | recording and experiment build | (session 2026-11-27 rec): builder 25709's rows 16 669 .. 16 710, the site 24969 and its flag | TRN-FREE-6.4a, T58 |
| E-ship-014 | code reading, experiment build | (session 2026-11-21 pm) | 6.4a the reinforcement's fourth standing tick (Q-1075) |

## Open questions

- Q-045 (rest): how the first step of a free walk from an off-centre position
  (after the fine approach) is timed.
- Q-046: what a settler does when its road walk cannot start (no road route).
