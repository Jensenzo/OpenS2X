# ECO-HQ: Headquarters, starting stock, territory and construction

Status: draft (starting stock, territory, exploration, levelling and construction work read from the binary; dispatch and ware destinations moved to `ECO-requests-dispatch.md`)
Owner question(s): Q-034, Q-038 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Each player starts with a headquarters (the `castle` building type) on their
start position. It is a warehouse, claims the land within 9 fields and
uncovers the land within 12. Its stock is a fixed list of wares and settlers,
multiplied by 1, 2, 4 or 8 by the "start resources" setting. A new building
is first a construction site: for medium, large and harbour buildings a
leveller flattens the six neighbouring fields; then a builder works through
the building's materials, frame materials first, spending four work steps of
55 ticks on each delivered material.

## Rules

Ticks are 0.1 s. Distances, rings and directions as in `MAP-grid-geometry.md`;
timed actions as in `ECO-worker-cycle.md` ECO-WORK-2.1.

### ECO-HQ-1 The headquarters

- **ECO-HQ-1.1** At game start every player gets a building of type `castle`
  (`buildings.lua`): size large, a warehouse (`bp_setDepot`), the place where
  new settlers appear (`bp_setSpawn`), no workers, no construction
  (`bp_setConstructionTime -1`), territory range 9, explore radius 12, door
  offset north-west for all tribes. It stands on the player's start position
  (`FMT-s2m-map.md` FMT-S2M-2.2) with its flag on the south-east neighbour
  (`ECO-building-placement.md` ECO-PLACE-3).
- **ECO-HQ-1.2** A building placed by the game itself (the headquarters) has
  its ground levelled at once (ECO-HQ-4.3 done instantly), not by a leveller.
  - **ECO-HQ-1.2a** (Q-118) This is a real height change, the same as six
    levelling steps (ECO-HQ-4.8, neighbours in direction order 0 to 5), made
    when the building is created at game start, before the first tick. It is
    made for medium, large and harbour buildings whose six neighbours are not
    all at the building field's height. So the headquarters' six neighbour
    fields, **its flag field included**, have the headquarters field's height
    from the start; every route, step cost and step time near the
    headquarters uses the levelled heights. Field states derived from height
    are not recomputed (ECO-HQ-4.8). Confidence high.

### ECO-HQ-2 Starting stock

- **ECO-HQ-2.1** The headquarters receives this stock, in this order (base
  amounts):

  | Wares | | Settlers | |
  |---|---|---|---|
  | wood | 12 | helper (carrier) | 26 |
  | plank | 22 | builder (`good_constructor`) | 5 |
  | stone | 34 | leveller (`good_bulldozer`) | 3 |
  | fish | 2 | woodcutter | 4 |
  | flesh (meat) | 3 | forester | 2 |
  | bread | 4 | stonecutter | 2 |
  | beer | 3 | hunter | 1 |
  | coal | 8 | cabinetmaker (carpenter) | 2 |
  | iron | 8 | miner | 5 |
  | axe | 3 | smith | 2 |
  | saw | 1 | locksmith | 1 |
  | pickaxe | 1 | scholar | 3 |
  | hammer | 8 | explorer | 1 |
  | shovel | 2 | donkey | 4 |
  | pot | 2 | soldier, lowest rank (`good_soldier0`) | 23 |
  | fishing rod | 3 | | |
  | scythe | 4 | | |
  | hatchet | 1 | | |
  | rolling pin | 1 | | |
  | spear | 1 | | |
  | sword | 3 | | |
  | shield | 3 | | |
  | boat | 6 | | |

  The donkeys are the settler `good_donkey` (D4157C5F), not the ware group
  `donkey` (2448356E) (Q-108).

- **ECO-HQ-2.2** Then every amount in the stock is multiplied by the start
  resources setting of the game: setting 0, 1, 2, 3 multiply by 1, 2, 4, 8.
  (A setting outside 0..3 multiplies by 0.) The free game's menu offers Low 0, Medium 1, High 2
  (`UI-game-setup.md` UI-SETUP-1.8). **A campaign mission always uses setting 1 (x 2), a tutorial
  lesson setting 2 (x 4)**, written at each start, never left over from an earlier game, for
  every player present (Q-1002, answered 2026-11-04, E-cam-003).
  So campaign 1's headquarters starts with 44 planks, 68 stones, 52 helpers, 10 builders and 46
  privates, and its script's 40 make 86. **Confirmed by R13 (2026-11-08 c, E-trace-023: campaign 1
  started from the campaign page, recorded every tick to 200):** at tick 0 the headquarters holds 44
  planks, 68 stones, 52 helpers, 10 builders and 85 privates, and the 86th private already stands
  outside on field (33, 34), next to the headquarters' field (33, 33), in the soldiers' list (state 12); ticks 0 and
  1 hold the same stock. Confidence: high (code and recording).
- **ECO-HQ-2.3** Map scripts can add more afterwards with
  `menu.addResource(player, good, count)` in `onStarted` (campaign 1 adds 40
  soldiers).

### ECO-HQ-3 Territory and exploration at the start (Q-034)

- **ECO-HQ-3.1** A military building (the headquarters counts) claims every
  field within its territory range (distance of MAP-GRID-2.1, its own field
  included). Where claims of several buildings overlap, the owner of a field is
  decided by ECO-HQ-3.5; at game start there is only one claimant, so **the
  player owns every field within distance 9 of the start position**.
- **ECO-HQ-3.2** After a claim the Border state (ECO-PLACE-1.4) is recomputed
  for the fields within range + 2.
- **ECO-HQ-3.3** A building with an explore radius marks every field within that
  distance as explored for its player, **except fields in the outermost row or
  column of the map** (x = 0, y = 0, x = width-1, y = height-1), which are never
  marked. At start: every field within distance 12 of the start position.
- **ECO-HQ-3.4** The headquarters' own placement (Blocked on its field,
  MovementBlocked on its footprint, its flag) follows ECO-PLACE-5.1.
- **ECO-HQ-3.5 Owner where claims overlap (Q-044).** When a military building
  claims (ECO-HQ-3.1), the claimants considered are the occupied military
  buildings (territory range > 0) within 3 x its range, and the claiming
  building. For each field in its range, taken in ring order:
  - the claimants are looked at in turn; the first one at distance 0, 1 or 2
    from the field gives the field to its player at once;
  - otherwise each claimant with distance `d` <= `r` contributes the strength
    `r / d^2`, where `r` is its territory range, doubled if `d` < 5 (and the
    test `d <= r` uses the doubled value); each player's strength is the
    largest contribution of its buildings;
  - the field goes to the player with the greatest strength; on equal strength
    the lower player number wins; if no claimant reaches the field, it has no
    owner.
  A player who loses fields this way gets the message "Lost Territory."
  (repeats suppressed as in ECO-REQ-3.8).

### ECO-HQ-4 Construction site

- **ECO-HQ-4.1** Placing a building creates a construction site with two
  phases, levelling and building, and a remaining-work value for each of the
  building phase's two parts: frame and main. Frame materials are the
  `bp_addFrameCost` list, main materials the `bp_addCost` list (sawmill: frame
  2 planks, main 2 stones).
- **ECO-HQ-4.2** Levelling is only needed for medium, large and harbour
  buildings, and only if one of the six neighbours of the building field has a
  height different from the building field. Otherwise (and always for small
  buildings and mines) the levelling phase is complete at once.
- **ECO-HQ-4.2a Site creation and the first update.** A site is created with its
  levelling progress at 0, whoever places it. The building update of the next tick
  it takes part in levels it (progress 1) when no levelling is needed (4.2) and
  starts the frame part; so a site placed by a human's command (processed before the
  buildings, TOOL-trace TRACE-2.2) shows `levelling 0` never and `frame 0` at the end
  of that tick, while a site placed by a computer player (which runs after the
  buildings) shows `levelling 0` at the end of the tick it was placed in and `frame
  0` one tick later. Confidence: high (measured: the computer player's hunter site at
  (140, 49), placed in counter tick 127, trace `levelling 0` at trace tick 128 and
  `frame 0` at 129; E-trace-002).
- **ECO-HQ-4.3** Levelling is done by a leveller (`good_bulldozer`) in steps. A
  step: the leveller goes to the neighbour field of the step, performs a timed action
  of the bulldoze time (`cfg_setBulldozeTime`, 55 ticks) standing on that field, then
  the site sets the height of the step's neighbour field to the height of the
  building field. The site takes its neighbours in direction order **starting at
  direction 2**: 2, 3, 4, 5, 0, 1 (MAP-GRID-1.2; the site's direction counter starts at
  2 for every building and advances by one per step). The height change (ECO-HQ-4.8)
  is applied, then each step adds 0.17 to the levelling progress; at 1.0 or more
  levelling is complete: **6 steps**, one per neighbour. The leveller then goes home.
  The step ends in the leveller's own update (the settlers' part of the tick), so
  the site sees the completed levelling, and files its builder request, in its
  next update (`ECO-requests-dispatch.md` ECO-REQ-1.3, measured); the material
  orders are filed in that last step itself (ECO-REQ-4.1, T29 there; corrected
  2026-10-28 night).
  Confidence: high (E-trace-002: the computer player's sawmill site at (138, 49) was
  levelled on (138, 48), (137, 49), (138, 50), (139, 50), (139, 49), (139, 48) in that
  order, directions 2 to 1; the raw dump at trace tick 1000 shows the direction counter
  at 2 in all six finished buildings, castles included; E-sim-002).
- **ECO-HQ-4.3a The first field (no draw) and the walks.** When the leveller's
  approach ends (his state 4 to 22, on the site's flag), he walks to the neighbour
  that the site's direction counter names (4.3), the direction-2 neighbour for a new
  site, and bulldozes it first; **nothing is drawn**, neither then nor for a later
  field: after each bulldoze the site's step (4.3) names the next direction and the
  leveller walks there. So the fields are bulldozed in the site's order 2, 3, 4, 5, 0,
  1 from the first one on. Measured (E-trace-002, 13 levellings by the computer
  player, trace ticks 483 to 18 070): every leveller's first field was the site's
  direction-2 neighbour, the west neighbour of the flag field, while the first draw of
  the arrival tick, taken mod 6, was 1, 1, 2, 1, 2, 4, 3, 5, 5, 2, 1, 4, 5; the arrival
  tick 4211 (leveller 90552, flag (136, 57)) and the tick after it hold no draw at all.
  In an experiment build the drawn choice (a draw in trace tick 483 picking the second
  marked neighbour, (139, 49)) gave the first differing building tick 561; without the
  draw the six fields and the progress ticks 566 and 649 match the original
  (Q-335 for the next difference). The code reading of a one-draw choice among marked
  neighbours (E-sim-001) does not describe a land site's leveller; whether it
  serves another job (the ship helper at a shipyard's water side) is open (medium).
  The walks between fields are ordinary lattice walks (TRN-MOVE, the slope factor
  applies; not fine walks): one step from the arrival field to the first field and
  one step from neighbour to neighbour; the step's duration follows the heights at its
  start (after the previous height change; Q-335). The leveller's fine position while
  bulldozing is the field's own sample. The leveller's approach is the ordinary walk
  to the site, which ends on the site's flag (E-trace-002: on (139, 48), the sawmill
  site's direction-1 neighbour, reached along the road from the castle's flag); his
  walk from one field to the next is the settlers' free walk (TRN-FREE-2, 3): one
  lattice step when the fields are neighbours, as in every step of E-trace-002, and
  otherwise the free walk's route with its draws (TRN-FREE-3.2a, medium: not
  observed). After the last bulldoze he goes home from that field (state 3 at 965 on
  (139, 48)). Confidence: high for the first field and the absent draw (measured, 13
  cases, and the experiment build, T22, T-level-3, T-level-4), high for the walk kind
  and timing (T21, T22).
- **ECO-HQ-4.4** Building is done by a builder (`good_constructor`). He walks to
  the site and works there until the building is finished, then goes home.
- **ECO-HQ-4.5** A work unit uses one material that has been delivered to the
  site: during the frame part a frame material; during the main part a plank
  if one is on site, otherwise a stone. If no suitable material is on site the
  builder waits.
- **ECO-HQ-4.6** A work unit is 4 work steps, one at each of the building's four
  construction positions (`bp_setConstructionPosition`, per tribe; the builder
  walks between them, `TRN-free-walk.md` TRN-FREE-5.5). The material is taken
  from the site's stock **when the unit starts**, before the walk to the first
  position. The amount of work per tick is fixed when the unit starts:
  `1 / (220 x number of materials of the part being worked)` (frame count while
  frame work remains, otherwise main count), in 32-bit floating point. Measured
  (E-trace-001): 1/440 per tick for a sawmill's main part and a guard house's frame
  (2 materials each), 1/220 for the 1-material parts of a hunter's hut, a quarry and
  a woodcutter's hut. **How the amount is rounded (2026-10-24, E-trace-002, 005,
  006):** the game first rounds 1 / n to 32 bits and then divides that by 220,
  rounding to 32 bits again: amount = f32(f32(1 / n) / 220). For n = 1, 2 and 4 this
  equals f32(1 / (220 n)); for n = 3 and 6 it is one unit in the last place larger
  (n = 3: 0.0015151516, not 0.0015151515; n = 6: 0.0007575758, not 0.00075757573). The
  remainders show it only near the end of a part, where they are small: a farm's frame
  (3 materials) reads 0.0015160687 and 9.171199e-07 after 659 and 660 decrements, not
  0.0015160688 and 9.1735274e-07 (T-build-1). Every remainder of every site that
  finished in the three recordings (332 sites of 25 building types, frame and main
  counts from `buildings.lua` `bp_addFrameCost` / `bp_addCost`) lies on the chain
  these rules give, and the 16 sites recorded in every tick of their work finished at
  exactly the predicted decrement (checked by the investigation's construction check script).
  Confidence: high.
- **ECO-HQ-4.6a Ticks of a work step** (Q-110): after the walk to the position,
  a timed action of the construct time (`cfg_setConstructTime`, 55 ticks)
  starts, and in each of the next 55 ticks, starting with the tick it is
  started in, the remaining work falls by the unit's amount. In the 56th tick
  (the action has run out) the builder starts walking to the next position.
  After the fourth step, in that 56th tick, the remaining work falls **once
  more** and the unit ends. So a unit does 221 decrements, 1/220 more than its
  material's share; the surplus carries into the next part.
- **ECO-HQ-4.6b Builder tick order** (Q-125). In each tick the builder
  first counts down a running timed action and advances a running walk, then
  decides; a decision that waits for a walk to end is taken in the tick the
  walk ends. A walk ordered in tick `T` that takes `n` ticks
  (`TRN-free-walk.md` TRN-FREE-5.3, no carry-over from an earlier walk) ends
  in tick `T + n`. Ordering a walk never moves the builder in the same tick.
  1. **Arrival.** In tick `S` the builder's road walk to the site ends on the
     site's flag field. Nothing else happens in `S`.
  2. **Approach.** In `S + 1` he starts a fine walk (TRN-FREE-5.5, same
     straight lattice line) from where he stands to the lattice point
     **3 lattice steps in direction 5** (north-east, MAP-GRID-1.6, applied on
     the sample lattice) from the sample of the field he stands on. "The field he
     stands on" is the settler's current field as the fine walk keeps it: after
     every fine step it is set to the field that the current sample truncates to
     (MAP-GRID-4.3). So after a unit, standing on position 3's point, it is that
     point's truncating field, and the 3 steps start from that field's own
     sample, not from the point itself (Q-134). Lattice steps that would leave
     the map are not taken. In the
     tick that walk ends he starts a fine walk back to the **flag field's
     sample**. In the tick that walk ends the approach is done and, in the
     same tick, step 3 runs.
     **Checked against the Fight autosave (2026-11-09 c, Q-1037, E-sav-012).** The flag field is the building's
     direction-1 neighbour (MAP-GRID-1.6: south-east, since +y is north): for
     a site on an odd row (x, y) it is (x + 1, y - 1), on an even row
     (x, y - 1). Direction 5 on the sample lattice is (c, r + 1) from an even
     sample row and (c + 1, r + 1) from an odd one (MAP-GRID-1.2 applied to
     samples), so from a field's sample (always on an even sample row) the far
     point is that sample + (1, 3). All four saved approaches run between
     these two points (T26). Q-1037's "north-east neighbour" was this flag
     field, read with +y as south. The leg is saved (`FMT-savegame.md`
     FMT-SAV-3.5a (e)): a constructor loaded on the way out walks back to the
     flag sample before step 3.
  3. **Unit choice.** If a suitable material is on site (ECO-HQ-4.5), a work
     unit starts: the material is taken and the work per tick fixed
     (ECO-HQ-4.6). Otherwise the approach (step 2) starts again in the next
     tick, from the field the builder now stands on; this repeats until
     material is there.
  4. **Work steps.** In the tick after the unit started, the walk to
     position 0 is ordered. For each position `i`: walk ordered in tick `T`
     (`n` ticks: its steps' rates are those of the heights when each step starts,
     `TRN-settler-movement.md` TRN-MOVE-2.5, not fixed when the unit starts, so a
     levelling nearby during the unit changes the later walks; added 2026-10-27 b,
     measured T24); the timed action starts and the first decrement is done in
     tick `T + n`, or in tick `T + 1` if `n = 0` (he already stands on the
     position); 55 decrements in ticks `T + n .. T + n + 54`; in
     `T + n + 55` the walk to position `i + 1` is ordered, or, after position
     3, the extra decrement is made and the unit ends (ECO-HQ-4.6a).
  5. **Next unit.** If the building is not complete after a unit that ended
     in tick `E`, step 3 runs in `E + 1` (material taken in `E + 1`) and the
     walk to position 0 is ordered in `E + 2`, from position 3.
  Confidence: high (control flow); the walk lengths come from TRN-FREE-5.3.
- **ECO-HQ-4.7** The frame part is worked off first, then the main part. A
  decrement that takes the frame below 0 sets it to 0; later decrements go to
  the main part. When the main part falls below 0 the building is complete
  (at once, in that tick) and the building becomes active; **the builder is
  released only when the timed action of his current step ends** (55 ticks from
  its start, the decrements that would follow having nothing left): he keeps
  his work state and animation until then and goes home by `TRN-free-walk.md`
  TRN-FREE-6.4a from the tick after the action's last tick (corrected 2026-10-25
  evening, measured E-trace-010: builder 90292 at the sawmill (138, 49), step
  begun in trace tick 2427, the building complete in 2480, still WorkLoop0 in
  2480 and 2481, standing (going home) in 2482, fine walk back from 2483; when the
  completing decrement falls in the action's last tick, as in T24, the two
  coincide). The builder's link to the construction is dropped at his release, in
  the settlers' part of that tick; an upgrade's type change and a new upgrade
  order wait for it (`MIL-building-upgrade.md` MIL-UPG-1.2, 2.4, T8; Q-367).
  Examples (float32): sawmill (frame 2, main 2): the frame ends at
  unit 2's 220th decrement, the building at unit 4's 219th, after **882**
  work ticks plus walking (not 880); woodcutter hut (1 + 1): frame at unit 1's
  221st, building at unit 2's 221st (442); depot (4 + 3): frame at unit 4's
  218th, building at unit 7's 216th. Measured (E-trace-001): work steps of 55
  decrements, the first in the tick the builder arrives at the position and starts
  its action, the walk on starting in the next tick; a sawmill site was finished in
  the tick its main part fell below 0, at the 54th decrement of a step, as unit 4's
  219th requires.
- **ECO-HQ-4.8 Height change by levelling** (Q-109): the height samples
  (`MAP-grid-geometry.md` MAP-GRID-4) of the neighbour field and all samples
  within 2 lattice steps of it (19 samples) are set to the height of the
  building field's sample. Then each sample P of the lattice ring at distance 3
  around it (18 samples, in the ring order of `MAP-grid-geometry.md` MAP-GRID-3.2:
  start at the centre moved 3 steps in direction 4, then 3 steps in each direction
  0 .. 5, each sample handled after its step, the start last) is set to the integer
  mean (sum divided by the count, toward zero) of **the six samples around the
  sample one step from P in direction 1**, not of P's own six neighbours: reached
  from P by stepping once in direction 0, then once in 1, 2, 3, 4 and 5 one after
  another (P + d0, P + d0 + d1, ..., the sixth being P itself), so the six are
  P's direction-0 and direction-2 neighbours, P itself and three samples two steps
  away. (Corrected 2026-10-25 night, Q-271; samples outside the map are left out
  of sum and count.) Current values are used, those already changed in this pass
  and P's own included; a sample whose mean equals its value is not written. The field
  states derived from height (Water, Coast, ElevationBlocked) are **not**
  recomputed by levelling; only the building-class cache (ECO-PLACE-4.2) of the
  fields within 3 of the neighbour field is marked for recomputation. The "height of
  the building field's sample" is the value 4.8a gave it when the site was placed, not
  the map's value. Confidence: high (measured, E-trace-005: the sawmill site (138, 49)
  of T21 at 7252 after its placement, every bulldozed neighbour at 7252 afterwards, see
  T-level-5). The ring-3 mean: high (code E-sim-011, the copy of P stepped
  in directions 0 .. 5 without returning; measured with E-trace-007's height windows:
  the hunter site of T-level-8 reproduces all 725 recorded samples, T-level-9; 16 of
  the 21 sites placed after the start (the castles excluded) reproduce every sample they change, the other 5
  differ in 1 to 9 ring-3 samples whose outer inputs earlier unrecorded changes may have
  moved; P's own six neighbours miss 16 or 17 samples in every placement).
- **ECO-HQ-4.8a The site's own height (Q-335).** When a building or a construction
  site is placed (by a player, the computer player or the game at the start), and
  when a military building's upgrade starts (`MIL-building-upgrade.md` MIL-UPG-2.2,
  T6; added 2026-10-26 night), and the
  six neighbour fields are not all at the same height as the building field, the
  building field's sample is set to the **integer mean of the heights of the 19 fields
  within distance 2** of it (the field itself, its 6 neighbours and the 12 fields at
  distance 2; a height below the map's floor, -100, counts as the floor; the sum is
  divided by the number of those fields inside the map, rounded toward zero), and that
  value is spread exactly as a levelling step spreads a height (4.8: the samples within
  2 lattice steps of the building field's sample take it, the ring at lattice distance 3
  is averaged). This happens for every building type, huts included, at the moment the
  site appears, before any leveller comes; when the six neighbours already match, nothing
  changes. The castles made at the start of a game are levelled at once with the same
  value (ECO-HQ-1, T10a corrected: the castle field itself is changed too). Then the
  levelling steps (4.3) bring the neighbours to that value. Confidence: high (measured,
  E-trace-005: all 13 sites of the computer player in 6000 ticks, every one equal to the
  mean of its 19 fields as they stood at that moment, T-level-5, T-level-7; both castles
  at 7205, the mean of their 19 fields in the map file, T-level-6; code E-sim-003
  (the test of the six neighbours, the call with radius 2)).
- **ECO-HQ-4.8b A flag's height ring** (2026-10-25 night, the implementation's question
  on the hunter site's builder walk). When a flag is made (alone, by a road's end, or as
  a building's flag), the six height samples one lattice step around the flag field's
  sample (`MAP-grid-geometry.md` MAP-GRID-4) are set to that sample's height. The flag's
  own sample keeps its height; samples two steps away are not touched. The field states
  derived from height are not recomputed (as in 4.8). A building placed with its flag:
  the site's own height and its spread (4.8a) come first, then the flag's ring, so a
  sample on both (the site's ring 3 and the flag's ring) ends at the flag's height. In
  the sample lattice the six around (c, r) are (c - 1, r), (c + 1, r) and, for an even
  r, (c - 1, r ± 1), (c, r ± 1); for an odd r, (c, r ± 1), (c + 1, r ± 1). Confidence:
  high for building flags (measured, E-trace-007: all 23 flags of the game, the castles'
  included, each made with its building, have their six samples at their own height in
  the counter of the placement; each of those rings holds one sample of its site's ring 3,
  so the order holds in all 23, T-level-8 shows the hunter's); medium for a flag placed on
  its own (the same code, E-sim-009, no recording of one).

### ECO-HQ-5 Not specified here

- **ECO-HQ-5.1** Requests, warehouse dispatch and ware destinations are in
  `ECO-requests-dispatch.md`: leveller and builder requests (ECO-REQ-1.2,
  1.3), the warehouse that serves them (ECO-REQ-3), material orders filed
  only once levelling is complete (ECO-REQ-4.1), and the matching of orders to
  wares (ECO-REQ-5). A released worker, builder or leveller goes home to a
  warehouse (`TRN-carriers-transport.md` TRN-CARRY-7.1); a production building
  starts its cycle when its worker has arrived (`ECO-worker-cycle.md`).
- **ECO-HQ-5.2** Still open here: the menu names of the start resources
  settings, the territory owner rule between overlapping claims.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| starting stock | ECO-HQ-2.1 | | high | E-eco-006 |
| start resources multiplier | 1, 2, 4, 8 | | high (values), low (menu names) | E-eco-006 |
| headquarters territory range / explore radius | 9 / 12 | fields | high | `buildings.lua`, E-eco-006 |
| exploration skips outermost rows and columns | yes | | high | E-eco-006 |
| levelling steps | 6 (0.17 each) | | high | E-eco-006 |
| levelling step action | bulldoze time, 55 | ticks | high | E-eco-006 |
| levelled sizes | medium, large, harbour | | high | E-eco-006 |
| work per material | 4 steps x 55 = 220 | ticks | high | E-eco-006 |
| frame before main | yes | | high | E-eco-006 |
| main part prefers plank, then stone | yes | | medium | E-eco-006 |

## Test cases

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | new game, start resources setting 0 | headquarters stock | exactly ECO-HQ-2.1: e.g. plank 22, stone 34, helper 26, builder 5, soldier0 23 |
| T2 | setting 2 | stock | every amount x 4: plank 88, stone 136, helper 104 |
| T3 | `MP_4P_Fight`, headquarters at (27,57) | owned fields | exactly the fields within distance 9 of (27,57); (36,57) owned, (37,57) not |
| T4 | as T3 | explored fields | fields within distance 12 of (27,57) except those in row 0, row 79, column 0, column 79 |
| T5 | `tutorial01`, headquarters at (45,38) | owned field count | the number of in-map fields within distance 9 = 1 + 3 x 9 x 10 = 271 (none cut by the map edge) |
| T6 | sawmill site (medium) whose six neighbours differ in height from the site field | levelling | 6 steps; after it all six neighbours have the site field's height; at least 6 x 55 = 330 ticks of work |
| T7 | woodcutter site (small) | levelling | none, building can start at once |
| T8 | sawmill site, levelled, 2 planks and 2 stones on site, builder present | work | frame ends at unit 2's 220th decrement, building at unit 4's 219th: 882 ticks of work (3 x 221 + 219) plus walking between the positions (ECO-HQ-4.6a, 4.7) |
| T9 | as T8 but only the 2 planks delivered | work | the frame part completes, then the builder waits; no main work until a stone (or plank) arrives |
| T10 | castle at game start on uneven ground | levelling | done at once, no leveller sent |
| T10a | `MP_4P_Fight`, headquarters at (27,57) (height 492) at game start | heights before the first tick | (28,57) 888 -> 492, flag (28,56) 1257 -> 492, (27,56) 589 -> 492, (26,57) 300 -> 492, (27,58) 229 -> 492, (28,58) 295 -> 492; fields two away unchanged |
| T11 | woodcutter hut site with 2 planks | work | unit 1 takes a plank when it starts; frame completes in unit 1's last tick; building complete in unit 2's last tick (442 work ticks) |
| T12 | levelling step on neighbour N, site height h | heights | the 19 samples within 2 lattice steps of N's sample equal h; ElevationBlocked of nearby fields unchanged |
| T13 | two occupied military buildings of players 0 and 1, ranges 8 and 8, a field at distance 3 from player 0's and 5 from player 1's | owner | player 0 (strength 16/9 against 8/25) |
| T14 | field at distance 2 from a player-1 building and 1 from a player-0 building | owner | whichever of the two is looked at first (ECO-HQ-3.5): the building whose claim is being computed (TER-3.1) |
| T15 | new game | HQ stock | 4 settlers `good_donkey`, no ware `donkey` |
| T16 | level ground, builder's road walk ends on the flag in tick 1000, material on site | approach and unit 1 | approach walk out ordered 1001 (3 lattice steps, 34 ticks) ends 1035; walk back ordered 1035 ends 1069; unit 1 starts (material taken) 1069; walk to position 0 ordered 1070 |
| T17 | position `i` walk of 0 ticks (position equals the point he stands on), walk ordered in tick 2000 | step | first decrement of that step in tick 2001 |
| T18 | unit ends in tick 3000, building not complete, material on site | next unit | material taken 3001; walk to position 0 ordered 3002 (from position 3) |
| T19 | as T16 but no material on site at 1069 | waiting | approach repeats from 1070 (out and back, from the flag field) until a unit can start |
| T20 | builder finished a unit standing on position 3's point, sample (4k+3, 4m+2) with m even (truncates to field (k, m), MAP-GRID-4.3), no material left | next approach | outward walk goes 3 lattice steps north-east from field (k, m)'s own sample (4k, 4m), not from (4k+3, 4m+2) |
| T21 | ECO-HQ-4.3: the computer player's sawmill site at (138, 49) of E-trace-002, leveller 90231 arriving on (139, 48) at counter tick 482 (trace tick 483, state 22; the ticks of this row are counter ticks) | fields and ticks | walk to (138, 48) 482-510 (28 ticks), bulldoze 510-565; walk to (137, 49) 565-593 (28), bulldoze to 648; walk to (138, 50) 648-676 (28), bulldoze to 731; walk to (139, 50) 731-754 (23), bulldoze to 809; walk to (139, 49) 809-832 (23), bulldoze to 887; walk to (139, 48) 887-910 (23), bulldoze to 965; progress 0.17, 0.34, 0.51, 0.68, 0.85, 1.0 at 565, 648, 731, 809, 887, 965; state 3 (going home) at 965 |
| T22 | ECO-HQ-4.3a: the same leveller | draws from the simulation generator | none for the leveller's field choice: not at his arrival on the flag (trace tick 483) nor at the steps 565, 648, 731, 809, 887 (the generator rows of 731 and 887 show 0 draws in the whole tick). Trace tick 484 (the tick after the arrival) holds one draw **before the animals' update** that s2-reborn lacks (Q-337, E-trace-005); its consumer is not known and it is not tied to a leveller's arrival (the arrivals of trace ticks 4211 and 5837 are followed by ticks without any draw) |
| T24 | 4.6b step 4, TRN-MOVE-2.5: E-trace-012 (Hard game), builder 90583 of the stone pit site (138, 54) (flag (137, 53)); in a unit his action at position 1, sample (552, 215), ends and the walk to position 2, sample (551, 213), is ordered at counter 4166 over (552, 214); the heights of those samples changed after the unit started | the walk's ticks (E-trace-013, every tick) | first step 0.09 per tick (counters 4166 .. 4177), second 0.072 per tick (a rise, 4178 .. 4190), on (551, 213) and working from 4191: 25 ticks. With both steps' rates from the heights at the unit's start (an experiment build) 23 ticks: working from 4189, two decrements ahead (buildings first differ at trace tick 4190; with the rule at 5160, settlers 4190 -> 4210) |
| T23 | ECO-HQ-4.3: a bulldozing leveller on (138, 48) | fine position | (552, 192), the field's own sample; the rows of trace ticks 510-564 |
| T26 | 4.6b step 2 (Q-1037), FMT-SAV-3.5a (e): the Fight autosave of 2026-11-01, the four constructors at step 13 | flag field, the approach's two points, the saved leg and what follows the saved walk | 24454: site (22, 49), flag (23, 48), flag sample (92, 192), far point (93, 195), leg 2, walking to (92, 192): chooses his unit when he arrives. 24416: site (21, 50), flag (21, 49), (86, 196) / (87, 199), leg 2, to (86, 196): chooses on arrival. 24602: site (56, 20), flag (56, 19), (226, 76) / (227, 79), leg 1, to (227, 79): on arrival walks back to (226, 76), then chooses. 24675: site (21, 41), flag (22, 40), (88, 160) / (89, 163), leg 1, to (89, 163): walks back to (88, 160), then chooses |
| T25 | ECO-HQ-4.3 (the step's height change in the settlers' part), ECO-REQ-0.2 (list order), TRN-MOVE-2.5: E-trace-012 / E-trace-013 (Hard), the mill site 90961 on (136, 59) with leveller 91004 on (137, 60), and constructor 91022 of the site on (136, 61) walking his lattice steps nearby | the samples (549, 242), (549, 243) and the constructor's step between them | heap dumps 7000 and 7500: (549, 242) 76.07, (549, 243) 72.95; the leveller's action ends in counter 7621 (he walks on to (137, 59) at progress 0 that tick) and the heights table shows 74.25 / 74.20 from 7621; the constructor reaches (549, 243) and starts the step to (549, 242) in the same counter 7621 at progress 0, 0.072 per tick (factor 0.8: the old heights, rising 3.12). The constructors' list is updated before the levellers' list, so a constructor (and a carrier) starting a step in that tick sees the heights before the levelling step; applying the levelling in the site's own update (the buildings' part) gives 0.09 per tick (the Hard pair's settlers first differ at 7623 in that build, its buildings at 7680). Experiment build (s2-reborn 0cf1665 with the leveller still in the site's update, but the step's height changes held back until after the settlers' pass): Hard pair view-free settlers 7623 -> 7919, buildings 7680 -> 9324, generator 8471 -> 8866; the Weak pairs unchanged |
| T-level-1 | computer player places a hunter's hut on level ground in tick n | building rows | tick n: state levelling, progress 0; tick n+1: frame, 0 |
| T-level-2 | sawmill site needing 6 levelling steps, leveller arrived | progress per step | 0.17 per step, steps 78 to 83 ticks apart (55 bulldoze + the walk), done after the 6th |
| T-build-1 | ECO-HQ-4.6 amount, a farm (frame 3, main 3) from a fresh site, every decrement in float32 | frame remainder after 650, 659, 660 decrements; main remainder after the frame's end | 0.015152434, 0.0015160687, 9.171199e-07; the 661st takes the frame to 0, then the main part counts down from 1 by 0.0015151516 per decrement and the building completes at decrement 1322 (unit 6's 217th). E-trace-002 farm 90751: frame 0.015152434 at trace tick 8260, 9.171199e-07 at 8270, main 0.9969697 at 8280 |
| T-level-3 | leveller arriving at a new site's flag, any generator state | first field | the site's direction-2 neighbour (the flag field's west neighbour); no draw then or for the later fields (4.3a). E-trace-002: site (138, 49), flag (139, 48), arrival trace tick 483 -> (138, 48); site (144, 47) 1839 -> (144, 46); (137, 55) 3580 -> (137, 54); (136, 58) 4211 -> (135, 57); (140, 53) 4431 -> (140, 52); (136, 51) 4593 -> (136, 50) |
| T-level-4 | as T-level-3, E-trace-002 leveller 90552 arriving on (136, 57) in trace tick 4211 | generator | 0 draws in 4211 and 4212 (draw index 17 230 in 4210, 4211 and 4212); he walks west to (135, 57) from 4212 |
| T-level-5 | ECO-HQ-4.8a, E-trace-005: the computer player's sawmill site (138, 49) placed at counter 323; map heights of the site and its neighbours in direction order 0 to 5: 7446; 6939, 7219, 7448, 7693, 7507, 7306; the castle's levelling had already set (139, 50) to 7205 and the hunter's hut (140, 49), placed at counter 127, had set its own field from 6080 to 5945 (its 19-field mean) | heights at the site's appearance and after each levelling step | site 7252 at counter 323 (the mean of the 19 fields within 2, with (140, 49) at 5945 and (139, 50) at 7205; 7259 without the hut's change); neighbours unchanged: 6939, 7219, 7448, 7693, 7507, 7205. After the steps at counters 565, 648, 731, 809, 887, 965 the fields (138, 48), (137, 49), (138, 50), (139, 50), (139, 49), (139, 48) read 7252 one after the other. Walks of T21: (137, 49) 7252 -> (138, 50) 7507 rises 255, class 1, 28 ticks; (138, 50) 7252 -> (139, 50) 7205 descends, 23 ticks |
| T-level-6 | ECO-HQ-4.8a at the start: MP_2P_Storm_Coast, castles on (29, 51) and (139, 51) (map 7682 each) | heights at trace tick 1 | both castle fields and all six neighbours of each at 7205 (the integer mean of the 19 fields within 2 of each castle in the map: 7205.6); the map's (139, 50) 7306 -> 7205, (140, 50) 6997 -> 7205; samples up to 7 lattice steps from the castle's sample changed, none beyond (169 samples per castle) |
| T-level-7 | ECO-HQ-4.8a, every other site of E-trace-005 (counter: field, height before -> height at appearance) | the site's own height | 127: (140, 49) 6080 -> 5945; 596: (139, 47) 6842 -> 6465; 818: (142, 56) 7572 -> 7591; 1100: (138, 57) 7454 -> 7400; 1308: (140, 57) 7546 -> 7539; 1474: (144, 47) 340 -> 508; 1838: (136, 47) 7599 -> 7400; 2703: (139, 55) 7579 -> 7519; 3198: (137, 55) 7766 -> 7381; 3443: (136, 53) 7626 -> 7508; 3724: (136, 58) 6212 -> 7366; 3986: (136, 51) 7794 -> 7529 (each the 19-field mean of the heights at that moment, `heights.csv.gz`) |
| T-level-8 | ECO-HQ-4.8b, E-trace-007 (= E-trace-005, 006): the hunter site (140, 49), sample (562, 196), placed at counter 127 with its flag (141, 48), sample (564, 192), map height 5002; map heights of the flag's six samples (563, 192) 5365, (565, 192) 4614, (563, 191) 5104, (564, 191) 4714, (563, 193) 5284, (564, 193) 4925 | the samples after the placement | the six all 5002; (564, 192) stays 5002; the second ring unchanged ((562, 192) 5708, (565, 193) 4538); the site's ring 3 next to it (562, 193) 5317, (561, 193) 5656, (560, 193) 5978, while (563, 193), on both rings, is 5002; site samples within 2 at 5945. (563, 193) 5002 and (563, 194) 5945 also in the heap dumps of counters 523 and 1023. A builder from the flag sample to (563, 194) over (563, 193): level, then a 943 rise (class 2): rates 0.09 and 0.054 a tick (E-trace-005 constructor 90202, first work tick 873) |
| T-level-9 | ECO-HQ-4.8 ring 3, E-trace-007 counter 127: the hunter site (140, 49), centre sample (562, 196), its radius 2 set to 5945 | the ring-3 sample (562, 193) (value before 5620) | the six samples read: (563, 193) 5001 (a ring-3 sample changed earlier in the pass), (564, 192) 5002, (563, 191) 5104, (562, 191) 5472, (562, 192) 5708, (562, 193) itself 5620; sum 31907, mean 5317 (recorded 5317). Likewise (561, 193) 5656 and (560, 193) 5978 as recorded; averaging P's own six neighbours would give 5712, 5928, 6190 |
| T-level-10 | ECO-HQ-4.8, 4.8a (2026-11-08 b): E-trace-017 (the Weak game), player 1's farm site 90538 on (136, 58), placed at counter 3729 (site height 7366, map 6212), levelled in 4329, 4401, 4479, 4579, 4649, 4729 (directions 2, 3, 4, 5, 0, 1) | the heights after the last step, west of the site (the look comparison's darker ground, GFX-VIEW-2.4) | (538, 229) 7647, (539, 229) 7471, (539, 230) 7366, (538, 230) 7643, (537, 230) 7850, (538, 231) 7366, (539, 231) 7366, (540, 229) 7366: the recorded window of 4729 and the rules give the same. They hold until the computer player's road 90792 "132,58 133,58 134,58 134,57 135,57 136,57" (guard house 90785 on (131, 59)) is made between counters 5950 and 5959; its evening (TRN-ROAD-3.3) gives (538, 229) 7782, (539, 229) 7659, (538, 230) 7678, (537, 230) 7626, (540, 229) 7541 (E-trace-017's full height dump at 11100; nothing else touches them in between), so the picture at 5999 shows those (corrected 2026-11-08 b, Q-1030) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-trace-001 | live read of the running game | follow tables (`follow/buildings.csv`), sites 90211, 90244, 90300, 90322, 90383 | ECO-HQ-4.6, 4.7 |
| E-eco-006 | code reading | `S2DNG.exe` | starting stock list and the final multiplication |
| E-eco-006 | code reading | | start-resources setting 0..3 -> 1, 2, 4, 8 |
| E-eco-006 | code reading | | warehouse created by the game: levelling done at once |
| E-trn-004 | code reading | | game-created building: the levelling loop with height change runs at creation |
| E-eco-006 | code reading | | territory claim from the territory range |
| E-eco-006 | code reading | | exploration from the explore radius |
| E-eco-006 | code reading | | leveller and levelling |
| E-eco-006 | code reading | | builder, work units, progress of frame and main |
| E-trn-005 | code reading | | builder tick order, approach walk, unit choice, waiting loop (Q-125) |
| E-eco-006 | strings | `!Start resources:` | the setting in the game menu |
| E-trn-002 | code reading | | start stock uses the settler donkey id |
| E-trn-002 | code reading | | material taken at unit start; 55 + 1 ticks per step; extra decrement at unit end; frame/main overflow |
| E-trn-002 | code reading | | levelling height change; no ElevationBlocked recompute (only the editor path does it) |
| E-trn-002 | code reading | | owner rule between claimants; "Lost Territory." |
| E-eco-006 | script | `buildings.lua` castle; campaign map scripts `menu.addResource` | headquarters properties; extra start goods |
| E-eco-013 | code reading | | approach starts from the movement's field cursor; the fine mover sets that cursor to the truncating field of the current sample each tick |
| E-sim-001 | code reading | | ECO-HQ-4.3a |
| E-sim-002 | code reading | E-trace-002 raw dump at trace tick 1000: the direction counter of the building records = 2 (six buildings) | ECO-HQ-4.3, 4.3a |
| E-trace-005 | live read of the running game | `heights.csv.gz` (the samples around every site at its appearance and after each levelling step), `heights-full-000001.bin.gz` (the whole layer at trace tick 1), the map file's layer for the values before | ECO-HQ-4.8, 4.8a; T-level-5 to 7; Q-335 |
| E-sim-003 | code reading | | ECO-HQ-4.8a |
| E-sim-009 | code reading and live read of the running game | E-trace-007 `heights.csv.gz`, `heights-full-000000.bin`, `-006003.bin`, `flags.csv.gz` and heap dumps 500, 1000 | ECO-HQ-4.8b; T-level-8 |
| E-sim-011 | code reading and live read of the running game | E-trace-005, 006, 007 `heights.csv.gz` and `heights-full-000000.bin`, checked by the investigation's ring-3 check script | ECO-HQ-4.8 (ring 3), T-level-9 |
| E-trace-002 | live read of the running game | `buildings.csv.gz` (140, 49) counters 127 .. 129; (138, 49) counters 324 .. 965; `settlers.csv.gz` object 90231 | ECO-HQ-4.2a, 4.3a timings |
| E-trace-002, 005, 006 (construction check) | live reads of the running game | `buildings.csv.gz` of the three recordings: the frame and main remainders of all 332 finished sites, checked by the investigation's construction check script | ECO-HQ-4.6 (amount rounding), 4.6a, 4.7; T-build-1 |
| E-sav-012 | code reading and save sample | session 2026-11-09 c: the Fight autosave's four step-13 constructors | 4.6b step 2 check, T26 (Q-1037) |

## Open questions

- Q-038 (rest): the menu names of the start resources settings and the
  in-game sawmill timing; the territory owner rule between overlapping claims
  (now Q-044). Dispatch, ware destinations, worker and input requests, leveller
  and builder choice: answered in `ECO-requests-dispatch.md`.
- Walking of builder and leveller between construction positions: route rule
  in `TRN-free-walk.md` TRN-FREE-5; positions in TRN-FREE-5.5a; the per-tribe
  offsets make the times building-specific (not tabulated). The player's
  tribe in a free game: Q-051.
- The order in which claimants are looked at (ECO-HQ-3.5): the claiming
  building first, then the others in building list order (`TER-territory.md`
  TER-3.1). Territory after game start: `TER-territory.md`.
