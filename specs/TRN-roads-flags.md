# TRN-ROAD: Roads and flags

Status: draft (route search, flag limit and removal read from the binary; automatic flags not traced, see open questions)
Owner question(s): Q-032, Q-139 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

A road joins two flags. The player starts a road at a flag and picks a target
field; the game finds the route itself: a search from the target back to the
start, where climbing or descending makes a step more expensive and harbour
pavement doubles the cost, read out from the start along the lowest labels.
The route is usually, but not always, the cheapest (TRN-ROAD-2.3, 2.4). A road may not cross another road or pass over a
flag, and may only use fields a flag could also stand on, apart from the flag
spacing. There is no length limit. At most 6 wares can lie at one flag.

## Rules

Fields, directions, distance and height classes as in `MAP-grid-geometry.md`;
general site and flag site as in `ECO-building-placement.md` ECO-PLACE-2.

### TRN-ROAD-1 Fields a road may use

- **TRN-ROAD-1.1** Every field of a road, including its target, must be a
  general site of the player (ECO-PLACE-2.1): own territory, not Border,
  Water, ElevationBlocked, DoodadBlocked, Blocked, DepositBlocked or
  MovementBlocked. So a road never passes a building field, a building
  footprint, a tree, a stone pile, a blocking doodad or water.
- **TRN-ROAD-1.2** A field with the Street state or with a flag may only be the
  target of the search, never an inner field: **roads do not cross**.
- **TRN-ROAD-1.3** Every field the road enters must be explored by the player
  (except the target).
- **TRN-ROAD-1.4** There is no slope limit of its own. Steep places are excluded
  only through ElevationBlocked (ECO-PLACE-1.3), so two consecutive road
  fields never differ by more than 2400 in height.

### TRN-ROAD-2 Route search

- **TRN-ROAD-2.1** The route from the start field S (the flag or route end
  the road is built from) to the target T is the result of the search of
  2.3 and the read-out of 2.4. It is usually the one with the lowest total
  step cost, but not always (T18, T20, and T6: the search stops before every
  cheaper route is known). Search space is the whole map; there is no
  maximum road length. Confidence: high (E-trn-008, measured on E-trace-002,
  T19, T20).
- **TRN-ROAD-2.2** Step cost from field A to its neighbour B is
  `c(A, B) + c(B, A)`, where `c(X, Y)` depends on the height class of
  `height(Y) - height(X)`: class -1 gives 4, -2 gives 5, -3 gives 7, any
  other class (0, positive, or -4) gives 3. In effect, by the size of the
  height difference: class 0: 6, 1: 7, 2: 8, 3: 10 (4 cannot occur,
  TRN-ROAD-1.4). If B has ForHarbor, the step cost is doubled. Here B is the
  field the search of 2.3 enters, and that search runs from T towards S: of
  two consecutive road fields, the doubling looks at the one nearer the
  road's start S, never at T. Confidence: medium (code reading, E-trn-008; not measured).
- **TRN-ROAD-2.3** The search runs **from the target T to the start S**
  (Q-253). Every field gets at most one **label**, the cheapest cost from T
  found so far (T has label 0). Fields wait in an **open list** ordered by
  label plus the field distance to S (MAP-GRID-2.1, plain number of steps,
  not scaled to step costs). A field put into the list goes **after all
  fields with the same value** and before the first with a greater one
  (first in, first out among equals). Each round takes the first field of
  the list (it is then closed) and visits its neighbours in direction order
  0 to 5 (MAP-GRID-1.2). A neighbour that may not be entered (TRN-ROAD-1;
  only S itself is exempt from the street and flag rule) or lies outside the
  map is skipped. Otherwise its new cost is the field's label plus the step
  cost (2.2); if it has no label it gets the new cost and joins the list; if
  its label is not greater than the new cost nothing changes; if its label
  is greater it gets the new cost and is put into the list again by the
  same insertion rule (taken out first if it was still in the list,
  reopened if it was closed). **The search ends the moment S is visited**
  (after S's label is set, in the middle of that field's neighbours), even
  if a cheaper way to S would be found later. If the list is empty before
  that, no road is possible. Confidence: high (E-trn-008; measured T19, T20).
- **TRN-ROAD-2.4** **Read-out.** The road is read from S towards T over the
  labels the search left. At each field, the candidates are the neighbours
  (inside the map, with a label) whose label is the lowest among the
  neighbours' labels that are below the current field's label. The road
  keeps going in the direction of its previous step if that neighbour is a
  candidate; otherwise it turns to the candidate with the lowest direction
  number. Before the first step the "previous direction" is 0 (east). It
  ends when it reaches T. The field list runs from S to T, both included.
  Equal-cost routes are decided by this rule alone, and the read-out does
  not check step costs, so the road can cost more than the cheapest route
  (T18). Confidence: high (E-trn-008: the read-out with the draw flag off, called from the route builder; measured T19, T20).
- **TRN-ROAD-2.5** A target that is a neighbour of the start is accepted at once
  as a road of one step, without a search.
- **TRN-ROAD-2.6 A second road between the same two flags (2026-11-12 night, Q-035; E-trn-008,
  code reading).** When a finished route is made into a road, the game checks only that its first
  point holds a flag, places a flag at the last point if that point is a road field where a flag
  fits (splitting that road), and then that the last point holds a flag different from the first.
  It does **not** check whether the two flags are already joined: a second (third, ...) road between
  the same two flags along another route is built like any other, each with its own carrier.
  Walks between such flags take the walker's own road or the shortest one (`TRN-free-walk.md`
  TRN-FREE-6.5 (f)). Confidence: high (the road maker's checks; the flag-to-flag lookup that
  chooses among parallel roads exists for this case).

### TRN-ROAD-3 State of a built road

- **TRN-ROAD-3.1** The inner fields of a road (all but its two flags) get the
  Street state and carry the road in the map's street layer. Flags are kept
  in a separate layer; buildings in a third. **A flag's field carries the
  Street state too while at least one road is attached to it**, and a flag
  without roads does not (corrected 2026-10-27 night, measured T26: the earlier
  text gave the state to the inner fields only). Rules that test the Street
  state therefore also see the flags with roads (for example an animal's
  target, `ECO-food-mining.md` ECO-FOOD-5.10; a building's footprint already
  excludes flags). Confidence: high for the state (three saves, one heap dump);
  medium for the moment it is set and cleared (with the road's own fields, by
  inference).
- **TRN-ROAD-3.1a No automatic flags (2026-11-16 night, Q-032 a; E-ui-006 road creation, code
  reading).** Building a road places no flag between its ends: the inner fields get the road only, so
  a long road has one carrier until the player sets flags on it. A flag is added only at the road's
  end, and only when that end lies on another road where a flag may stand (that road is split,
  TRN-ROAD-4). Confidence: high.
- **TRN-ROAD-3.2** A building's footprint may not contain a Street field
  (ECO-PLACE-3.6). A flag may (TRN-ROAD-4).
- **TRN-ROAD-3.3 A land road evens the ground along it (2026-10-25 night, from Q-271).**
  When a land road is made (by a player or the computer player, including each of the
  two roads of a split, 4.2; a waterway changes nothing), each step of the road from
  one field to the next, in the road's field order from its first field to its last,
  changes height samples (`MAP-grid-geometry.md` MAP-GRID-4) between the two fields'
  samples, using the heights as they are at that moment (earlier steps of the same road
  included). The two field samples are 4 sample steps apart in one direction d. The step
  is worked from the field whose direction to the other is 2, 3 or 4 (towards the
  south-west, west or north-west): call that field's sample A, the other B. With hA and
  hB their heights and q = (hB - hA) x 0.25, the values v0 = hA, v1 = v0 + q, v2, v3
  are formed one after another (float32), and for k = 1, 2, 3 the sample k steps from A
  towards B is set to trunc(vk), and its two neighbours in directions d + 2 and d - 2
  (mod 6) are both set to trunc((v(k-1) + vk) / 2). A and B themselves are not changed.
  Two exceptions, which depend on whether the road's own order runs from A to B or from
  B to A:
  (a) the road's first step, next to its first field: if A is the first field, the
      sample at k = 1 and its two neighbours are left as they are (v1 is still formed);
      if B is the first field, only the centre sample at k = 3 is left, its two
      neighbours are set;
  (b) a step whose second field (in road order) holds a flag (the road's last step):
      if that field is B, the centre sample at k = 3 is left, its neighbours are set;
      if it is A, the sample at k = 1 and its neighbours are left.
  A sample already at the value is not written. Confidence: high (code E-sim-011: the road's per-field step on land roads and the evening itself; measured:
  the computer player's road 90199 of E-trace-007 (counter 176, T22) changes 21 samples,
  and with it the window the recorder wrote at the sawmill's placement (counter 323)
  is reproduced in all 725 samples (23 differ without the road); with every road and
  every flag of the game applied in counter order, 14 of the 21 placements of E-trace-005
  and 007 reproduce their whole window, the other 7 lying in one area where an earlier
  difference carries on).
- **TRN-ROAD-3.3a Planned routes even the ground too (2026-10-25 night).** The evening
  of 3.3 is done each time a field is added to a road, and a route being built is such
  a road: when the route builder of 8.2 extends a route (its search finds a path from
  the route's last field to the field asked for), every field of that path is added
  and each step is evened at once, exactly as in 3.3 (the route's first field counting
  as the first field for exception (a)). Nothing undoes it when the route is cut,
  cancelled or not built. So the computer player's road builder (`AI-construction.md`
  AI-CON-8.1, 8.3 c; `SIM-game-start.md` SIM-START-4.4 (c)) evens the ground along the
  whole path its search finds **whether or not the road is then built** (a path longer
  than 2 x the distance is refused after the evening); a road that is built is evened
  again by 3.3 with the same values. A route whose search finds no path changes
  nothing. Confidence: high for the computer player (code E-sim-011: the
  extension's path, each field through the road's per-field step; measured: counter 1437 of the
  two-player game (E-trace-007 = E-trace-008), a route optimizer call that took a stub
  number (90348) and built no road: between the heap dumps of counters 1023 and 1523
  the ground along (140, 50), (139, 50), (138, 50), (137, 51), (137, 52), (136, 53),
  (137, 54) changed as 3.3 predicts, all 31 changed samples of rows 196 to 212
  reproduced, (559, 200) next to the first field left as exception (a) says; replayed
  with the other roads, flags and placements, two more placements (1838, 3443) then
  reproduce their whole window, 16 of 21). **Measured in full (2026-10-25, E-trace-009,
  T24):** the recorder's `ai_plan` table (TOOL-trace TRACE-4.6) holds every call's
  planned path; the refused ones of the game are 808 (5 steps), 1437 (17 steps,
  "140,50 139,50 138,50 137,51 137,52 136,53 137,54 137,55 138,56 137,57 138,58
  139,58 140,58 141,58 142,58 142,57 143,56 142,55") and 3509 (17 steps); evened
  along their whole paths with the roads, flags and placements, all 21 placements
  of the game reproduce their whole window (725 samples each), the five that still
  differed (2703, 3724, 3986, 4334, 4574) included. Such refused calls are frequent: E-trace-008
  has 22 in its 6000 ticks (a road builder's number taken with no road made in that
  counter: 808, 1437, 1927, 2312, 2441, ...). Medium for a human player's route (the
  same builder, E-ui-005; not observed).

### TRN-ROAD-4 Flags on roads

- **TRN-ROAD-4.1** A flag can be placed on an inner field of a road when the
  flag-site rule holds there (ECO-PLACE-2.2: in particular no flag on the field
  or its neighbours, so not next to either end flag).
- **TRN-ROAD-4.2** Placing it splits the road into two roads that meet at the
  new flag F: the old road A..B is removed, the flag is created, and two new
  roads are made from the old field list cut at F. **Both halves run towards
  the new flag:** the first from the old road's first flag, fields A..F; the
  second from the old road's second flag, fields B..F (the old list's tail in
  reverse order), so F is the second flag of both (`SIM-game-start.md`
  SIM-START-4.4 (j) for the order of the steps and the object numbers;
  measured in E-trace-006, T11 there: road 90335 has the fields
  "142,55 141,55 141,56"). The direction matters, for example for the idle
  field of a half with an even number of fields, entry floor(n / 2) of its list
  (`TRN-carriers-transport.md` TRN-CARRY-1.3). Each half asks for its carrier as a new road does, and the
  old road's carrier, if it had one, is attached by one field: the field he
  stands on, or while he is in a step, the field that step goes to. If that
  field is one of the first half's fields (A..F) or the field of the building at
  the old road's first flag A, he gets the first half; otherwise the second
  (B..F), also when he is still on his way and that field is not on the old road
  at all. (Corrected 2026-10-25, Q-359: was "the half containing his current
  field, else the one containing his target field"; measured T23: a new carrier
  walking on another road towards B gets the second half, and his walk ends on
  the next flag, `TRN-free-walk.md` TRN-FREE-6.5 (g).) Wares lying at the end
  flags stay there. (Corrected 2026-10-24, Q-267: this
  rule said "A..F and F..B".) Confidence: high for the split, the direction
  and the carrier rule (E-ui-006; E-sim-007;
  E-trace-006; E-trn-011: the field test, then the first half's
  first flag, its building and that building's field; E-trace-007/008 carrier
  90315, T23), medium for wares in transit.
  **A carrier carrying a ware when his road is split** (2026-10-25 evening, T25;
  2026-10-28, T29): what decides is whether the new flag F lies ahead of him
  on his walk (his step goes to F, or F is one of the fields still to walk).
  With F ahead, his walk is cut at F whichever half his flag lies on (T28,
  T29; the paragraph "If the flag he carries it to lies on the other half"
  below): he puts the ware down at F, and from there he works as at any flag
  (with the ware's next flag the far end of his own half, he takes it up
  again at once: T29). If the split comes while he is in his step onto F,
  that step is not checked: he arrives on F with a place there, without the
  entry check (T29; confidence medium, one case). With F behind him and
  the flag he carries it to an end of the half he gets, nothing about his
  walk changes: he finishes the step he is in, walks on along the same fields,
  and his entry check at that flag (`TRN-carriers-transport.md` TRN-CARRY-3.2,
  4.3) goes on where it was (a place already granted stays granted; the check
  is not started again), so he does not stop on the way; he puts the ware down
  there as usual. Confidence: high for this case
  (measured). **If the flag he carries it to lies on the other half**
  (measured 2026-10-28, T28): his walk is cut at the new flag F, which lies
  ahead on the same fields: he finishes the step he is in (its progress kept,
  the excess carried into the next step as on any walk), walks on along the
  old road's fields to F and makes his entry check there as at any flag; he
  puts the ware down at F, where it waits with its next flag unchanged for the
  other half's carrier. Nothing is restarted. Confidence: high (measured).
  **A carrier walking without a ware when his road is split** (2026-10-27 b,
  T27): on his way to fetch a ware at an end of the half he gets, with the
  new flag ahead of him on his walk, his walk ends on the new flag
  (`TRN-free-walk.md` TRN-FREE-6.5 (g), the road taken away): he stands on
  its point in the tick he arrives (the step's excess dropped), and his walk
  on to the ware's flag is ordered in the next tick at progress 0, first
  advancing in the tick after; he still fetches the ware. With the new flag
  **behind** him and the ware's flag an end of his half, nothing about his walk
  changes, as for a carrying carrier: he finishes his step and walks on along the
  same fields to the ware (measured T30, added 2026-10-29). If the ware lies at
  a flag that is **not** an end of his half and the new flag is ahead of him,
  his walk ends on the new flag as in T27 and he **keeps his claim on the ware
  until he arrives there**: in his arrival tick the claim is released and he
  stands on the flag's point, and in the next tick his walk to his half's idle
  field is ordered at progress 0 (measured T31, added 2026-10-29). **On his
  way to his idle field** (after a put-down, without a ware) with the new flag
  ahead of him on that walk (his step goes onto it, or it is still to be walked;
  in the measured case the new flag stands on the old road's idle field, his
  walk's goal), the same: his walk ends on the new flag, he finishes the step he
  is in at its rate, stands on the flag's point in the tick he arrives (the
  step's excess dropped), and in the next tick his walk to his half's idle field
  is ordered at progress 0, first advancing in the tick after (measured T33,
  added 2026-10-30; he does not turn back from the field he last reached).
  Confidence: high for these cases (measured). **On his way to his idle field
  with the new flag not on that walk** (beyond the walk's goal, so the goal and
  the field his step goes to lie on his half; added 2026-11-01 night, T34):
  nothing about the walk changes. He finishes it at its rate to the old idle
  field and stands on its point in the tick he arrives. In the next tick his walk
  to his half's idle field is ordered at progress 0, and it first advances in
  the tick after. He is not put back on the field he last reached. If the old
  idle field is also his half's idle field, the walk simply ends there.
  Confidence: high for the case measured (T34); a walk with the new flag behind
  him (between him and where he started) is not measured.

### TRN-ROAD-5 Flags

- **TRN-ROAD-5.1** Flags are at least 2 fields apart (ECO-PLACE-2.2,
  `TRN-settler-movement.md` TRN-MOVE-3).
- **TRN-ROAD-5.2** **At most 6 wares lie at one flag** (`cfg_setFlagPackageLimit`,
  shipped value 6). A ware can be added to a flag only while fewer than the
  limit lie there; workers wait in the building (`ECO-worker-cycle.md`
  ECO-WORK-3.4) and carriers wait until there is room.

### TRN-ROAD-6 Removing roads and flags (Q-139, Q-032 b, c)

The same two removal steps are used by the player's commands ("destroy
street", "destroy flag"), by the territory rules (`TER-territory.md` TER-5.3,
TER-6.3) and by conquest (`MIL-conquest.md` MIL-CON-2.2).

- **TRN-ROAD-6.1 Removing a road.** In this order: the road's fields lose
  their Street state (an end flag keeps it while another road is attached to
  it, 3.1); the road is taken off the player's road list; its
  carrier and its donkey (if any) are released (TRN-ROAD-6.3); the road is
  detached from both end flags, whose cached routes are recomputed; its open
  requests for a carrier or donkey are cancelled (they stay in the player's
  request queue, unlinked, until their turn: `ECO-requests-dispatch.md`
  ECO-REQ-2.3a, T38; the same for the old road of a split, 4.2). **The end flags stay**, with
  whatever lies on them, even when nothing else connects to them.
  Confidence: high for the steps; medium for their order.
- **TRN-ROAD-6.2 Removing a flag.** In this order: every road that ends at the
  flag is removed (TRN-ROAD-6.1), one after another; then, if a building
  stands behind the flag, the building is removed **with remains**
  (`MIL-conquest.md` MIL-CON-4.1, 4.4); then the flag itself goes, and every
  ware lying on it is destroyed (TRN-ROAD-6.4). The building's remains are
  put on this flag before it goes, so they are destroyed too. No other road is cut: a flag
  only has the roads that end at it. Confidence: high.
- **TRN-ROAD-6.3 A released carrier** (or donkey) loses its road and goes home
  (`TRN-carriers-transport.md` TRN-CARRY-7.1). When it starts going home:
  a ware it **holds in its hands** is destroyed (TRN-ROAD-6.4); a ware it had
  only **claimed** at a flag (not yet picked up, to fetch it) stays at that flag
  and is **free again at once**. The **exchange ware** it was bound to
  (`TRN-carriers-transport.md` TRN-CARRY-3.7) stays at its flag and **stays bound
  to it while it walks home**: no other carrier takes it, and its next flag is not
  searched again, until the carrier itself is gone: taken in at a warehouse, or
  dead (TRN-CARRY-7.1: at the end of its dying, when the skeleton is placed), or
  removed in any other way; in that tick the binding ends and the ware is free.
  Whatever ends a carrier ends all its bindings with it: a ware still in its hands
  is destroyed, a claim and an exchange binding end (corrected 2026-10-30 (b),
  Q-294: T32 is an exchange binding; the claim's release at once and the release
  at the carrier's end are read from the code, E-trn-025, and the claim's release
  changes no recorded pair). The ware is not put down. Confidence: high for a
  held ware, high for the kept exchange binding (measured T32), high for the
  released claim and the end with the carrier (code); the tick of the end for a
  carrier that dies is medium (the end of its dying, not measured). The carrier notices the release in its own next update, finishes the step it
  is walking, and only then turns home (`TRN-carriers-transport.md` TRN-CARRY-7.1a);
  "when it starts going home" is that next update.
- **TRN-ROAD-6.4 A destroyed ware** is taken off its flag and off the player's
  ware list. If it was on its way to fill a request (it had a destination),
  that assignment is dropped first, so the requesting building is again
  missing that ware and orders it anew by its normal rules
  (`ECO-requests-dispatch.md`). Confidence: high (E-trace-006: the plank destroyed in
  the released carrier's hands at counter 387 was ordered again and sent at 484 or 517,
  6.6).
- **TRN-ROAD-6.5** A building is never removed by removing a road; only by
  removing its flag (6.2) or by its own rules (TER-6.2, conquest, catapult,
  player). Confidence: high.
- **TRN-ROAD-6.6 The wares of a removed road (E-trace-006, 2026-10-24).** The removal
  itself touches no ware but the carrier's (6.3). Every other ware keeps its
  destination at first and is handled by the transport's periodic jobs, which see that
  the ware network has changed (`TRN-carriers-transport.md` TRN-CARRY-5.1a: the road's
  carrier is gone, so its two end flags may no longer be in one ware network):
  - **in the released carrier's hands**: destroyed when the carrier notices the release
    (6.3, 6.4); the order it was meant for is open again at once;
  - **only claimed by the released carrier**: free at its flag again (6.3), then as a
    ware lying at a flag;
  - **lying at a flag** (an end flag of the removed road or any other) with a
    destination: at the next re-check (`ECO-distribution.md` ECO-DIST-1.6, ticks with
    `t mod 3 = 0`) its next flag is computed again over the ware networks; if none
    exists it has no next flag. At the next order matching (`t mod 11 = 0`) an order
    whose ware has no next flag loses that ware (`ECO-requests-dispatch.md` ECO-REQ-5.7):
    the order is open again and the ware lies at its flag without destination, free;
    the storage run (`t mod 5 = 0`, ECO-REQ-5.5) then sends it to the nearest accepting
    warehouse of its ware network, and if it lies at that warehouse's own flag it goes
    straight into the stock. A ware on a storage trip whose next flag is gone loses its
    storage destination at the storage run and is placed again in the same run's
    second step (ECO-REQ-5.7 (c));
  - **in another carrier's hands, on another road**: carried on to the flag it was
    going to; arriving there, its next flag is computed over the ware networks
    (TRN-CARRY-5.4) and the case above follows if there is none.
  Every order that lost its ware is served again by the normal matching, as soon as a
  ware or a warehouse with stock is in the requesting building's ware network.
  Measured (E-trace-006, counter = trace tick - 1): road 90199 (flags 90191 hunter site,
  90182 castle) removed in counter 386 with carrier 90204 holding plank 90208 and plank
  90210 lying at the castle's flag, both for the hunter site 90190. 90208 was destroyed
  in 387; 90210 had no destination before 400 and was gone in 400 (taken into the
  castle at its own flag, 400 mod 5 = 0); the two new roads got their carriers at 417 and
  447; the castle sent both planks again, at 484 and 517 (its door blocked 30 ticks after
  each departure, ECO-DIST-1.7c). The moments in between, measured in the same game
  recorded again with a wares table (E-trace-007, Q-346): plank 90210 keeps its next flag
  (141, 48) through 386 and has none from 387 (the re-check, 387 mod 3 = 0), keeps its
  destination 90190 through 395 and has none from 396 (the order matching, 396 mod 11 =
  0), lies at 90182 without destination through 398 and is gone at the next snapshot
  (401; 399 and 400 not recorded; 400 in E-trace-006); plank 90208 in carrier 90204's
  hands is gone in 387. Confidence: high (outcomes and every tick).

### TRN-ROAD-7 Waterways (Q-097)

- **TRN-ROAD-7.1** A waterway is a road built in waterway mode (the flag menu's "Create a
  Waterway", enabled only at a flag whose field is Coast; the mode starts with `start-road` (waterway) and
  grows with `extend-road` like a land road). The route search is the land search
  (TRN-ROAD-2) with other field rules and costs. Confidence: high.
- **TRN-ROAD-7.2** Field rules for a step from the previous field Q onto field P: P may carry the
  Street state or an object only when P is the target; unless P is the target, P must be Water
  and Q must be Water or the start flag's field; in every case P or Q must be Water, P must not be
  Border or DoodadBlocked, and P must lie in the same water body (continent id) as the start
  flag's water. So a waterway leads from a coast flag over sea fields of one water body to a coast
  field (a flag, or a free field that gets one); it never crosses land. Confidence: high.
- **TRN-ROAD-7.3** Step cost onto P is 1, or 2 when P is Coast (the start and end steps). The
  explored-field rule of TRN-ROAD-1.3 and the slope costs of TRN-ROAD-2.2 do not apply.
  Confidence: high.
- **TRN-ROAD-7.4** Whether a field can end the road being built (land or water) is the same test
  the cursor uses (UI-GAME-8.6): the target must be inside the map, not Border; the start not
  DoodadBlocked; when the target is not Water it must have no other flag within 2 fields and pass
  the object test of ECO-PLACE for a flag; the start and target must lie on the player's continent;
  in waterway mode the distance from the route's current end to the target plus the number of
  points already in the route minus 1 must not exceed 6 (a waterway is at most 6 steps long), and
  once the route has more than one point its current end must be a Water field. A click that
  fails the test extends nothing. Confidence: high (E-pt2-001; play-test 2
  chunk 4 resolved the "mode count" term: the route's point count).

### TRN-ROAD-8 Building a road: the route (play-test 2, chunk 2)

- **TRN-ROAD-8.1 Start.** `start-road`(field, waterway) from
  a player who has no route creates the player's route: a road object with
  the one field of the flag (the field must carry a flag). Each player has
  at most one route at a time. Confidence: high (E-ui-006).
- **TRN-ROAD-8.2 Extend and cut.** `extend-road`(F) while the
  route is not complete (8.3): if F is a field of the route, the route is
  rebuilt from its start up to F (the fields after F are dropped). Otherwise
  the route search of TRN-ROAD-2 (TRN-ROAD-7 for a waterway) runs from the
  route's **current end** to F; when it finds a way all its fields are
  appended, each gaining the Street state in the map (so the route itself
  blocks like a road and the next search may not cross it); when it finds
  none the route is unchanged. Confidence: high (E-ui-006). The appended fields even the ground at once (3.3a), also
  for a route that is later cut or cancelled.
- **TRN-ROAD-8.3 Completion.** A route is complete when its first field has
  a flag, it has more than one field, and its last field has a flag or is a
  field of another finished land road where a flag may be placed (ECO-PLACE-2).
  After every extension, and on every game tick for every player's route,
  a complete route becomes a road: when its last field is a road field
  without a flag, a flag is placed there first (splitting that road,
  TRN-ROAD-4.2); the route's fields become the road, the road asks for a
  carrier (TRN-CARRY), and the route is gone. A tick also drops a route
  whose start flag no longer exists or one of whose fields can no longer be
  used (the UI then leaves road-building mode). `cancel-road`
  removes the route and clears its fields. Confidence: high (E-ui-006).
- **TRN-ROAD-8.4 Look.** The route is handed to the renderer like a road as
  soon as it has two fields and whenever it changes: it is drawn as an
  ordinary road (same mesh and textures, `GFX-objects.md`) from the start
  flag to its end, with no special marking; the ring and neighbour arrows
  of GFX-VIEW-6.2 show the end and the possible next steps. Confidence:
  high (E-ui-006).

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| flag_package_limit | 6 | wares per flag | high (script value; used as "fewer than limit" in all readers) | `config.lua`, E-eco-005 (the setting and its five readers) |
| step cost c(X,Y) for class -1 / -2 / -3 / other | 4 / 5 / 7 / 3 | | high | E-eco-005 |
| harbour pavement factor | 2 | | high | E-eco-005 |
| maximum road length | none | | medium (no limit in the search) | E-eco-005 |
| crossing roads | not allowed | | high | E-eco-005 |
| search estimate | field distance to S, 1 per step (not scaled by step cost) | | high | E-trn-008 |
| read-out start direction | 0 (east) | | high | E-trn-008 (draw flag off for roads) |

## Test cases

Computed with the investigation's placement script on `tutorial01.s2m` ("T01") with a
headquarters at (45,38) and its flag at (45,37); player owns every field.

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | T01 + HQ | road (45,37) to (45,33) | possible, 4 steps, cost 27 |
| T2 | T01 + HQ | road (45,37) to (41,39) | possible, 5 steps, cost 33 |
| T3 | T01 + HQ | road (45,37) to (45,35) | 2 steps via (46,36), cost 12. The other two-step route, via (45,36), costs 13 and must not be chosen |
| T4 | T01 + HQ | road (45,37) to (49,37) and to (50,41) | refused: target is not a general site |
| T5 | T01, empty | road (40,36) to (40,32) | possible, 11 steps (a straight line would be 4), cost 75: blocked fields force a detour |
| T6 | MP_4P_Fight, HQ on all starts | road from the HQ flag (28,56) to (24,54) | 5 steps, cost 34: (28,56) (27,56) (26,56) (25,56) (24,55) (24,54) (TRN-ROAD-2.3, 2.4; a route of cost 33 exists, but the search reaches S through this one first). Derived from the rule, not measured |
| T7 | any | road whose only route crosses an existing road | refused, unless a flag is first placed on the crossing field |
| T8 | level ground | step cost between two fields of equal height; between fields 201..600 apart; 601..1200; 1201..2400 | 6; 7; 8; 10 (either direction) |
| T9 | a field with ForHarbor on the route | step onto it | cost doubled |
| T10 | a flag with 6 wares | a worker or carrier wants to put a 7th there | must wait; with 5 wares it may |
| T11 | road A-B with inner field X at distance 2 from both flags | place a flag on X | allowed; road becomes A-X and X-B |
| T12 | flag A, start a road, `extend-road` to a free field F three fields away | logic | the route A..F has four fields; no road yet; the fields carry Street |
| T13 | route A..F, `place-flag`(F) | next tick | the road A-F exists, the route is gone, the road asks for a carrier |
| T14 | route A..F, `extend-road` to an existing flag B reachable from F | logic | the route extends to B and is built at once |
| T15 | route A..x..y..F, `extend-road`(x) | logic | the route becomes A..x |
| T16 | route ending on a field X of road C-D where a flag fits | logic | a flag is placed on X, C-D becomes C-X and X-D, the new road A-X is built |
| T12 | road with an inner field next to flag A | place a flag there | refused (neighbour of a flag) |
| T13 | road A-B, carrier walking with a ware towards B, 2 wares lying at A (one claimed by the carrier's next trip) | destroy the road | road gone, flags A and B stay; the held ware is destroyed; the 2 wares stay at A; carrier walks home |
| T14 | flag F with 3 wares, roads F-A and F-B, a woodcutter hut behind F | destroy F | both roads removed (their carriers go home, held wares destroyed); hut removed with remains, which are put on F (F still exists at that moment, MIL-CON-4.4) and then destroyed with F's other 3 wares; requests they were meant to fill are open again |
| T15 | a ware on its way to a building, destroyed with its flag | next request matching | the building's request for that good is served again from a warehouse or a free ware |
| T16 | waterway route with 4 points, target 3 fields from its end | end test | 3 + 4 - 1 = 6: allowed; 4 fields away: refused (7.4) |
| T17 | flat ground, every field enterable, field (9,8) 300 higher than the rest (steps to or from it cost 7) | road S (8,8) to T (10,10) | (8,8) (8,9) (9,10) (10,10), cost 18. At (8,9) both (9,9) and (9,10) are candidates (label 6); the previous direction 5 is kept. The other cost-18 route (8,8) (8,9) (9,9) (10,10) must not be chosen (TRN-ROAD-2.4) |
| T18 | flat ground, every field enterable, field (9,8) 1300 higher than the rest (steps to or from it cost 10) | road S (8,8) to T (10,8) | (8,8) (9,8) (10,8), cost 20, not the cost-18 detour (8,8) (8,7) (9,7) (10,8): the search labels (9,8) with 10 (value 11) before (8,7) gets 12 (value 13), and S is reached from (9,8) first (TRN-ROAD-2.3) |
| T19 | E-trace-002, computer player's first road (90199) at trace tick 176, hunter site (140,49) standing | road S (141,48) (hunter's flag) to T (140,50) (castle's flag) | (141,48) (140,48) (139,49) (140,50), cost 26 in s2-reborn; the equal-cost (141,48) (141,49) (141,50) (140,50) must not be chosen. First step: direction 0 is no candidate, 3 is the lowest candidate. Measured |
| T20 | E-trace-002, road 90549 at trace tick 3886 | road S (136,57) to T (139,56) | (136,57) (137,56) (138,56) (139,56), cost 24 in s2-reborn's world at that tick, while a route of cost 22 exists. Measured |
| T21 | TRN-ROAD-6.6, E-trace-006: road 90199 (flags 90191 hunter site, 90182 castle) removed by the computer player in counter 386, which builds 90214 - 90182 and 90214 - 90191 in the same counter; carrier 90204 holds plank 90208, plank 90210 lies at 90182, both for site 90190 | the two planks and the site's plank orders | 90208 destroyed in 387; 90210 without destination by 399 and taken into the castle in 400; carriers for the new roads made at 417 and 447; new planks leave the castle with helpers at 484 and 517 (both orders served again, not one) |
| T22 | TRN-ROAD-3.3, E-trace-007 counter 176: the computer player's road 90199 "141,48 140,48 139,49 140,50" (flags at the first and last field), field samples (564, 192) 5002, (560, 192) 6320, (558, 196) 6939, (560, 200) 7205 | the samples it changes | first step, worked from (564, 192) westwards (A = the first field), q = 329.5: (563, 192) and its neighbours left (v1 = 5331.5); (562, 192) 5708 -> 5661, its neighbours (562, 193) 5317 -> 5496 and (562, 191) 5472 -> 5496; (561, 192) 6029 -> 5990, (561, 193) 5656 -> 5825, (561, 191) 5818 -> 5825; second step (560, 192) -> (558, 196), direction 4: (559, 193) 6474, (559, 194) 6629, (558, 195) 6784 and neighbours (559, 192) and (560, 193) 6397, (558, 193) and (560, 194) 6552, (558, 194) and (559, 195) 6706; last step worked from (560, 200) (direction 2), its flag field being A: the sample next to it and its neighbours left, then (558, 198) 7038 ... 21 samples in all, every one as recorded (with the sawmill's placement at 323, the whole window of 725 samples) |
| T23 | TRN-ROAD-4.2, E-trace-007/008: road 90311 "139,56 140,56 141,56 141,55 142,55" (A = (139, 56), B = (142, 55)); its new carrier 90315 walks on road 90277 towards B, in the step (141, 52) -> (141, 53), when the stone pit's flag F = (141, 56) is set at trace tick 1308 | which half gets him | the second, "142,55 141,55 141,56" (90335): the field his step goes to is on neither half; his walk ends on (142, 55) (TRN-FREE-6.5 (g)) and he stands on the second half's idle field (141, 55) from 1374. (With "the half containing his target field", the idle field (141, 56) of 90311, F itself, would be in both) |
| T24 | TRN-ROAD-3.3a, E-trace-009 (two-player game, computer player Weak): the computer player's road builder calls whose path was found but no road made: counters 808 ("140,50 139,50 138,50 137,49 138,48 139,48", 5 steps > 2 x 2), 1437 and 3509 (17 steps each, ends (140, 50) and (142, 55)) | the heights at the 21 building placements | every placement window equal to the recording (21 of 21, 725 samples each) when these three paths are evened as 3.3 / 3.3a with the built roads; without 1437 and 3509, five windows differ (2703, 3724, 3986, 4334, 4574) |
| T25 | TRN-ROAD-4.2 (ware in transit), E-trace-010 (two-player game, computer player Weak): road 90277 "142,55 142,54 141,53 141,52 140,51 140,50" with carrier 90282 carrying a ware from (140, 50) to (142, 55); at counter 4334 the computer player's building splits it with the flag (141, 52) while he is in the step (141, 53) -> (142, 54) | the carrier | he gets the first half "142,55 142,54 141,53 141,52" (road 90601) and walks on without a pause: trace tick 4335 (568154, 216308), 90 x 180 per tick, on (142, 55) in 4356, then puts the ware down (measured). An experiment build that starts a new entry check stops him on (142, 54) from trace tick 4336; one that re-routes him from the field he last reached puts him back on (142, 54)'s point in 4335 (two ticks behind); keeping the walk and the granted place matches, and moves its dense-trace settlers difference from 4335 to 4796 and the generator's from 4537 to 4931 |
| T26 | 3.1 (the Street state of flags): the decoded saves E-sav-005 (autosave of the E-trace-012 game, 57 flags), `17-03-2023 16-40-11 Coastal Storm.sav` (947 flags) and `Autosave.sav` of 2026-10-13 (4 flags); the heap dump 10000 of E-trace-011 | the field state of every flag's field (FMT-S2M-4.4 bit 0x0002) against the flag's number of roads | every flag with 1 to 5 roads has the Street state (55 + 941 + 4 = 1000), every flag without a road lacks it (2 + 6 = 8); in the heap dump all 22 flags of player 1 (each with roads) hold 0x202 (Street and ForBuilding), e.g. (132, 58) the flag of the guard house (131, 59), (145, 46) the harbour's flag 0x120a |
| T27 | TRN-ROAD-4.2 (carrier without a ware): E-trace-013 (Hard game, every tick; the same game as E-trace-012), player 1's road (140, 50)-(139, 50)-(138, 50)-(137, 51)-(137, 52) with carrier 90659, split by the flag on (138, 50) (made between counters 4810 and 4819, E-trace-012 flags table; the halves (140, 50)-(138, 50), his, and (138, 50)-(137, 52)) while he walks back from (137, 52) to fetch at (140, 50) | his ticks at (138, 50) | step (137, 51) -> (138, 50) at 0.98 in 4823; standing on (138, 50)'s point in 4824; walk at progress 0 in 4825, 0.045 in 4826 towards (139, 50), on to (140, 50) without stopping, ware fetched there. In the previous round (4583) he passed (138, 50), then a plain road field, at once. Experiment build: the Hard pair's settlers 4830 -> 5190 |
| T28 | TRN-ROAD-4.2 (ware to the other half's flag): E-trace-013 / E-trace-012 (player 1, Hard), road 91275 "130,55 131,55 132,55 133,55 134,56" (flags 91256 (130, 55) and 91039 (134, 56)), its carrier 91281 carrying ware 91280 (next flag (130, 55)) from (134, 56), his step to (133, 55) begun at counter 10107 at 0.045 per tick; the computer player splits the road at (132, 55) at counter 10111 (flag 91378; halves 91382 "130,55 131,55 132,55" and 91385 "134,56 133,55 132,55") | the carrier and the ware | he gets half 91385 (his step goes to (133, 55)); the step goes on (0.18 in 10111, 0.99 in 10129), (133, 55) in 10130 with 0.035 carried, (132, 55) in 10152, standing there, the put-down from 10153; the ware lies at 91378 from 10157 with its next flag (130, 55). Our build re-routed him to the new flag from the field he had reached and restarted the step at (134, 56) in 10111. Experiment build (the walk cut at F, the step kept): his rows equal; Hard pair settlers 10112 -> 10291, the other pairs unchanged |
| T29 | TRN-ROAD-4.2 (new flag ahead, ware to his own half's far end): E-trace-014 (R4, the Weak game), player 1's road 91951 "137,37 138,36 137,35 137,34 136,33 136,32 135,31 135,30" (flags 91374 (137, 37) and 91944 (135, 30)), its carrier 91722 carrying ware 92400 (meat, next flag (137, 37)) from (135, 30), in his step (136, 32) -> (136, 33) since about counter 25768; at counter 25782 the computer player's road from (138, 32) splits it with the flag 92435 on (136, 33) (halves 92439 "137,37 138,36 137,35 137,34 136,33", his, and 92442 "135,30 135,31 136,32 136,33") | the carrier and the ware | his step goes on without a stop; trace tick 25791 standing on (136, 33), the put-down from 25792; the ware lies at 92435 from counter 25795 with its next flag (137, 37); 25800 standing, the ware claimed by him again; the pick-up from 25801, carried from counter 25804; walk on from 25809; on (137, 37) in 25919, the put-down there (measured). Our build walked on to (137, 37) without stopping (the T25 rule); cutting the walk at F with the entry check made there refuses him once (TRN-CARRY-4.4's bar) and steps him back onto (136, 32) at 25785. Experiment build (the walk cut at F, arriving with a place): his rows equal; the Weak game against R4 equal to settlers 25926, buildings 26891, generator 26083 (was 25791 / 26278 / 25894; the 150 000 pair's generator too), the other pairs unchanged |
| T30 | 4.2, a fetching carrier with the new flag behind: E-trace-013 (R3, the Hard game), player 1's carrier 91583 of the road 91581 (from (137, 54)), walking towards (137, 54) to fetch a ware there (at (136, 54), x 547120 in trace tick 13394); at counter 13394 the computer player's flag (135, 53) splits the road behind him; he gets the half (137, 54) - (135, 53) (road 91756) | his walk | unchanged: x 547300 in 13395, on the flag (137, 54) in 13399 (recorded). Re-planning his walk at the split puts him back on (136, 54)'s point (x 544000) in 13395: the Hard pair's settlers differ from 13395; with the rule from 13596 (experiment build; R3 13410 -> 13600), the other pairs unchanged |
| T31 | 4.2, a fetching carrier whose ware ends up on the other half: E-trace-013 (R3, the Hard game), player 1's carrier 91696 of the road 91691 (134, 58) - (135, 57) (fields 134,58 134,59 135,59 136,58 135,57), walking from (134, 58) to fetch the stone 91305 at (135, 57) (claimed at counter 13592, walk from trace tick 13593); at counter 13595 the computer player's flag (135, 59) splits the road ahead of him; he gets the half (134, 58) - (135, 59) | his walk and the claim | he walks on (fetch state) to (135, 59), arrives in trace tick 13638 and stands there, walks on in 13639; the stone's claim stays his until counter 13637 and its next flag becomes (135, 59) at 13638 (recorded). Releasing the claim at the split and sending him straight to the idle field: the Hard pair's settlers differ from 13596; with the rule the Hard pair is equal to 15526 in every column (buildings 14433 -> 15526; R3 settlers 13600 -> 15530; experiment build), the other pairs unchanged |
| T32 | 6.3 (the kept claim): E-trace-014 (R4, the Weak game), player 1's road 93458 "135,37 .. 137,44" with carrier 93465 carrying towards (135, 37) and bound to the iron ore 93826 lying there (destination the smeltery 92123, next flag (137, 44)); the computer player replaces the road by two roads through the new flag (136, 40) in counter 37353 | the iron ore and the carrier 93463 of the road (135, 37) - (137, 37) | 93465 goes home (from 37354), standing on the castle's door field at 37739; the ore stays claimed by 93465 with next flag (137, 44) until 37740, then gets next flag (136, 40) and is claimed by 94036 of the new road (37744); 93463 never fetches it (measured). Letting the claim go at once has 93463 fetch it at 37357 (the experiment build's settlers first differ at 37360; with the rule at 38650, buildings 38170 -> 39510, generator 37930 -> 38810, the 150 000 generator 37742 -> 38801; the Hard pairs unchanged) |
| T33 | 4.2, a carrier on his way to the idle field with the new flag on his goal: E-trace-014 (R4, the Weak game), player 1's road 91898 "143,41 143,40 142,39 143,38 142,37" (flags 91510 (143, 41) and 91532 (142, 37), idle field (142, 39)), its carrier 91900 walking back from his put-down at (143, 41) to (142, 39), in his step (143, 40) -> (142, 39) at 0.368 in counter 38648, when the computer player's flag 94187 on (142, 39) splits the road (halves 94191 "143,41 143,40 142,39", his, and 94194 "142,37 143,38 142,39") | his walk | the step goes on at 0.045 per tick (0.998 in counter 38662); standing on (142, 39)'s point in counter 38663 (trace tick 38664), the walk to (143, 40) at progress 0 in 38664, 0.045 in 38665, standing on (143, 40) from counter 38687 (recorded). Our build stopped him at once on (143, 40)'s point (trace tick 38649): the Weak pair's settlers differed from 38650; with the rule (experiment build) from 38720, every other pair unchanged |
| T34 | TRN-ROAD-4.2 (on the way to the idle field, the new flag beyond its goal): E-trace-019 (R6, MP_4P_Fight), player 2's road 14770 "52,23 53,23 54,24 54,25 55,26 55,27 56,28" (flags 14166 (52, 23), 14763 (56, 28)), its carrier 14607 walking to its idle field (54, 25), in the step (55, 26) -> (54, 25) at 0.85 when the computer player's flag 14866 on (54, 24) splits the road in counter 1664 (halves "52,23 53,23 54,24" and 14873 "56,28 55,27 55,26 54,25 54,24", his: his step goes to (54, 25)) | his ticks | the step goes on at its rate (trace 1665 .. 1667); standing on (54, 25) in counter 1667; his walk to (55, 26), 14873's idle field, at progress 0 in 1668, advancing from 1669 (recorded). Stopping him on (55, 26), the field he last reached, at once: the Fight game against R6 differs from settlers 1665. With the rule (experiment build, with `ECO-requests-dispatch.md` T39): settlers 1665 -> 1755; every pair of the fidelity set unchanged |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-eco-005 | code reading | `S2DNG.exe` | road route search for the road network (land and waterway variants) |
| E-eco-005 | code reading | | land road step: field rules, crossing, cost, harbour factor |
| E-eco-005 | code reading | | step cost by height class; explored-field test |
| E-eco-005 | code reading | | search driver (cost plus distance), path extraction |
| E-eco-005 | code reading | | flag on a road splits it; when a road field accepts a flag |
| E-trn-008 | code reading | | search from the target, insertion order, relabelling, stop at the start's first label, read-out by lowest label and direction rule (Q-253); the road maker's end checks, no test for flags already joined (TRN-ROAD-2.6, Q-035) |
| E-trace-002 | recording | `streets.csv.gz` (roads 90199, 90549 and the other roads of the computer player) | measured field lists; 21 of the recorded roads reproduced in a clone with the rule against 19 with a cheapest-route search, none lost |
| E-eco-005 | code reading | | Street state and street layer of a road's fields |
| E-eco-005 | code reading | | flag package limit setting and its cached copy |
| E-trn-006 | code reading | | road removal: Street state, list, carrier/donkey release, flags detached, requests cancelled |
| E-trn-006 | code reading | | flag removal: roads, building with remains, wares destroyed with their assignment |
| E-trn-006 | code reading | | going home: held ware destroyed, claimed ware released |
| E-trn-006 | code reading | | player commands use the same removal steps |
| E-eco-005 | strings | | the player commands `place-flag`, `remove-flag`, `start-road`, `cancel-road`, `extend-road`, `remove-road` |
| E-ui-006 | code reading | | route building and the flag-on-road split |
| E-trn-025 | code reading | | TRN-ROAD-6.3, Q-294 |
| E-cmd-001 | code reading | S2DNG.exe | waterway field rules and costs |
| E-pt2-001 | code reading | | waterway end test, length limit 6 |
| E-sim-007 | code reading and recording | E-trace-006 `objects.csv.gz` (wares 90208, 90210, 90241, 90243) | 6.3 timing, 6.6 the wares of a removed road |
| E-sav-005, E-sav-002, E-trace-011 | saves and a memory read of the running game | save `Autosave-2026-10-27-0905.sav` and the two 2026 / 2023 samples (GridStatesMap and the Net System's flags); heap dump 10000 of E-trace-011 | TRN-ROAD-3.1 flags (T26) |
| E-trace-013 | live read of the running game | recorded 2026-10-27 from 01:18, MP_2P_Storm_Coast, player 1 computer Hard, every tick from counter 0, heap dumps every 500 | the Hard game tick by tick (T27) |
| E-trace-014 | live read of the running game | the Weak game, every tick 20 000 .. 40 000: `streets.csv`, `wares.csv`, `flags.csv`, `ai_street.csv` counters 25740 .. 25925 | TRN-ROAD-4.2 (T29) |

## Open questions

All in Q-032:

- Automatic flags: whether the game places flags along a new road by itself.
- Waterways: the carriers of a waterway (a boat; SHIP); the "mode count" term of 7.4 is resolved.
