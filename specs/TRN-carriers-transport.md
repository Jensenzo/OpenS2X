# TRN-CARRY: Carriers and ware transport

Status: draft (carrier behaviour, ware choice, routing and donkey trigger read from the binary; full-flag behaviour, exchange, reservation and entry order read from the binary for Q-164; carrier and donkey dispatch from warehouses not traced; tick timings derived by reading, not measured)
Owner question(s): Q-004, Q-164 in `OPEN_QUESTIONS.md`
Game build: 11757

## Summary

Every road has one carrier and, once it is busy enough, a donkey. The carrier
waits on the middle field of its road and looks for work every 20 ticks. Work
is a ware lying at one of the road's two flags whose next flag on its route is
the other end. Between the two ends, the ware whose good has the smaller
transport-priority number goes first. Wares find their route through the flag
network themselves: the cheapest chain of roads, where a road counts double
or more when its carrier is busy or the flag at its end is full. A flag holds
at most 6 wares, and a ware counts against those places from the moment its
carrier is cleared to bring it. A carrier arriving at a flag where a ware waits
to travel back along its road swaps the two wares in one visit, whatever the
flag's fill; that is what keeps busy chains moving. Otherwise a carrier whose
ware has no room waits, and when room appears the waiting wares enter in order
of a wait score. A road whose traffic stays high gets a donkey.

## Rules

Ticks are 0.1 s (`ECO-time-units.md`). Walking follows `TRN-settler-movement.md`
(level field 23 ticks from rest); timed actions last `ceil(duration)` ticks
(`ECO-worker-cycle.md` ECO-WORK-2.1): drop time and pickup time 3.3 each, so 4
ticks each. Roads and flags as in `TRN-roads-flags.md`.

### TRN-CARRY-1 Roads, carriers and the idle field

- **TRN-CARRY-1.1** A road keeps the list of its fields in order from one flag to
  the other, both flags included (`n` = number of steps + 1 entries). The flag
  at entry 0 is called the road's **first flag**, the other its **second flag**.
- **TRN-CARRY-1.2** A road has two carrier places: the carrier and the donkey.
  Both behave as described below; the donkey is the second place.
- **TRN-CARRY-1.3** A carrier's **idle field** is list entry `floor(n / 2)`
  with `n` the number of entries (flags included): the middle field of a
  2-step road, the second inner field of a 3-step road (counted from the first
  flag). Measured (E-trace-002, T20): the computer player's first road runs
  from the hunter site's flag (141, 48) over (140, 48) and (139, 49) to the
  castle's flag (140, 50) (entry 0 is the flag the street construction
  started from, AI-CON-8.1; n = 4), and its carrier idles on entry 2,
  (139, 49), standing there with no further walk. A carrier arriving at the
  road's flag from its warehouse stands one tick on the flag and its walk to
  the idle field is ordered in the next tick at progress 0 (ECO-WORK-3.8c);
  its first step out of the warehouse is ordered in the tick it is sent
  (progress 0 in that tick, 0.045 in the next).
- **TRN-CARRY-1.4** A new carrier leaves its warehouse like a worker leaves a
  building (walk out to the building's flag, where it stands one tick whether or
  not that flag is on its road, `ECO-worker-cycle.md` ECO-WORK-3.8d, T24), walks over roads to its road and
  then to the idle field. Which warehouse sends it and when: the road requests
  a helper in the first tick it has both flags and no carrier, and the nearest
  warehouse by route cost sends one, at most one settler per 30 ticks per
  warehouse (`ECO-requests-dispatch.md` ECO-REQ-1.1, ECO-REQ-3). The walk from
  the warehouse to the road is not traced (Q-039).

### TRN-CARRY-2 Looking for work

- **TRN-CARRY-2.1** A carrier without a ware that stands on its idle field looks
  for work only in ticks where `(carrier number + tick counter) mod 20 = 0`. The
  carrier number is the **low 32 bits of the settler's object number** (TOOL-trace
  TRACE-3.2, `SIM-game-start.md` SIM-START-4: the global creation counter), the tick
  counter the game's own (trace tick - 1). So it reacts to a new ware within 0 to 19
  ticks. Confidence: high (measured, E-trace-005: all 93 walks a road carrier started
  from its idle field in 6000 ticks began in a tick with `(id + counter) mod 20 = 0`,
  for example carrier 90204 at counter 296, 90234 at 546, 90237 at 643, 90260 at 800;
  walks started away from the idle field show every residue; code E-trn-001
  reads the settler's id word).
- **TRN-CARRY-2.2** A carrier without a ware that is not on its idle field (for
  example just after putting a ware down at a flag) looks for work once, if it
  stands on one of its road's fields; finding none, it walks back to the idle
  field, and **during that walk it does not look for work**: it looks again when
  the walk has ended, on the idle field by 2.1's 20-tick grid (corrected
  2026-10-25 evening, measured E-trace-010: carrier 90260 left the flag (141, 48)
  after a put-down at counter 2684, the stone 90435 was ready for its road at the
  other flag from 2703, the carrier walked on and stood on its idle field (141, 49)
  from 2707 and claimed the stone at 2720, the first tick of its grid; code E-trn-013: the walk to the idle field is a sub-step without checks; earlier read
  as "every tick, turning back in the middle of a step", Q-107). If the walk ends
  off the idle field, the once-check and a new walk follow. This includes a new
  carrier or donkey that comes out of a warehouse onto **one of its own road's two
  flags** (the warehouse's flag is an end of its road): after its standing tick on
  that flag (ECO-WORK-3.8d) it looks once and takes a ready ware there at once
  (TRN-CARRY-2.5) before walking to its idle field (added 2026-10-27 b, T37). A walk
  that only passes over its road's flag on the way to the idle field has no look
  there (T24, T37). **The tick the walk
  ends is no look tick**, even when it falls on the carrier's grid: in that tick
  the carrier only becomes a standing carrier on its idle field; its first look is
  at the next grid tick, up to 20 ticks later (added 2026-10-27 b, E-trn-019: the idle walk's end switches the state and returns; measured T33).
- **TRN-CARRY-2.3** A ware is **ready for this road at flag X** when it lies at X,
  has a destination, is not yet claimed by a carrier, and its next flag
  (TRN-CARRY-5) is the road's other flag.
- **TRN-CARRY-2.4** The **best ware at X** is the ready ware with the smallest
  transport priority number of its good (`gp_setTransportPriority`, table
  below). Among equal numbers, the one listed first at the flag wins.
- **TRN-CARRY-2.4a The flag's list order (corrected 2026-10-25).** A ware joins
  the flag's list when its place there is granted, not when it is put down: for a
  ware a carrier brings, when the entry check admits it (4.3, 4.4), usually
  while the carrier is still walking; for a ware a warehouse's helper carries out,
  likewise by the helper's entry check soon after it leaves (two ticks after the
  ware's creation in the case measured, about 50 ticks before it lies at the flag);
  a worker bringing its product out bids the same way (E-trn-007) and joins at its
  admission too; an exchange ware (3.7), which enters without a bid, joins when it
  is put down. The order lasts until the ware
  leaves the flag; removing a ware does not reorder the others. Measured (T25):
  at the castle's flag of E-trace-007 the plank 90243 (for the hunter site 90190)
  joined at trace tick 770 on its road carrier's walk in and the plank 90266 (for
  the site 90244) at 772 on its helper's walk out; 90266 was put down at 822 and
  90243 at 823; at 978 the road's carrier 90260 took 90243. With the put-down
  order (90266 first) an experiment build takes 90266 and the settlers differ
  from trace tick 1168; with the joining order they agree to 1349 and the
  buildings to 1909 (from 1284). Confidence: high (measured) for carried and
  helper wares; medium for workers' products and exchange wares (code, E-trn-007).
- **TRN-CARRY-2.5** Choice of work:
  1. If the carrier stands on one of its road's flags, it takes the best ware at
     that flag if there is one.
  2. Otherwise let `p1` be the priority of the best ware at the first flag and
     `p2` that at the second flag (100 when there is none). If both are 100
     there is no work. If `p2 <= p1` it takes the second flag's ware, else the
     first flag's. **Ties go to the second flag.**
- **TRN-CARRY-2.6** Taking work claims the ware; no other carrier can take it.

### TRN-CARRY-3 Carrying

- **TRN-CARRY-3.1** The carrier walks along its road to the ware's flag, then
  picks it up: a timed action of the drop time followed by one of the pickup
  time (4 + 4 ticks). **The ware leaves the flag in the tick the first action
  (drop time) ends**: from then on it no longer lies at the flag (room,
  TRN-CARRY-4; route costs, TRN-CARRY-5.3) and is held by the carrier; the
  pickup-time action follows.
- **TRN-CARRY-3.2** It then walks along the road to the other flag F. In every
  tick of the walk, until one of the outcomes marked "settled" has happened, it
  makes the **entry check** for F, **the first in the tick after its walk is
  ordered** (not in the ordering tick itself; added 2026-10-27 b, T36), in this order:
  1. the ware heads into its destination building at F (TRN-CARRY-3.3): may
     enter, settled;
  2. a ware at F is ready for the way back (TRN-CARRY-3.7): the two wares are
     bound for an exchange: may enter, settled, **even if F is full**;
  3. F is full for the ware (TRN-CARRY-4.1, 4.2): may not enter; the ware's wait
     score grows (TRN-CARRY-4.4);
  4. F has room: the ware bids (TRN-CARRY-4.4); when it wins it is **reserved at
     F** (TRN-CARRY-4.3): may enter, settled.

  "May not enter" makes the carrier stop **on the last field of the road before
  F** and wait there: it walks on over the road's inner fields without stopping
  and only does not step onto F (corrected 2026-10-27 b, T35: it was "at the next
  field it reaches"; on a road of two steps both are the same field); while
  waiting it repeats the entry check only in ticks where
  `(carrier number + tick counter) mod 10 = 0`. In those ticks, if the answer
  is still no and also `(carrier number + tick counter) mod 200 = 0`, it tries
  a **road swap** (TRN-CARRY-3.6). A "may enter" before the carrier has
  actually stopped cancels the stop. **A carrier already on its step onto F**
  when the answer becomes no (its exchange undone on the way, 3.7, and the
  next check fails) does not finish that step: in the following tick the step
  is cancelled, the carrier stands on the last field of the road before F (its
  position goes back to that field's point) and waits; that tick makes no
  check, and the checks of the 10-tick grid follow (added 2026-10-28 night,
  E-trn-021: a failed check sets the carrier's stop mark, the walk
  ends at once on it and the waiting state checks only on the grid; measured
  T42). **Unless that tick's movement completes the step** (added 2026-10-30,
  T51): the movement comes first in the carrier's update, so a step whose
  progress passes 1 in the tick after the refusal ends on F; the carrier then
  stands **on F** in the waiting state, makes no check in that tick and checks
  only in the ticks of the 10-tick grid (and tries road swaps on the 200-tick
  grid) like a carrier waiting before F; when a check admits it, the next tick
  is its arrival tick at F (3.3a: it stands, then the put-down's two actions).
  While on its step onto F and not settled, the carrier checks in every tick
  (E-trn-024). **The tick it comes to a stop** (corrected 2026-11-01,
  T56): a carrier refused while walking makes no check in the tick its step
  onto the last field before F ends and it stops there, even when that tick is
  on the 10-tick grid, as on F (T51) and after a cancelled step (T42); its first
  check as a waiting carrier comes in the next grid tick. Confidence: high for the order and the
  outcomes, medium for the exact tick in which a check falls while walking.
- **TRN-CARRY-3.2a The admission tick (view).** When a carrier that has
  stopped and waits (3.2) is admitted at one of its checks (any "may enter"
  outcome), its walk is ordered again in that tick: its trace row shows the
  walk animation at its standing position from the admission tick, and its
  first step advances from the next tick. Confidence: high (measured,
  E-trace-010: carrier 90234 waiting on (139, 49) with plank 90660 from trace
  tick 5020, admitted by an exchange at counter 5606 (the helper's plank for
  (141, 46) lying at the castle's flag with next flag (139, 48)): Walk at
  (558000, 196000) in trace tick 5607, (558090, 196180) in 5608; T29).
- **TRN-CARRY-3.6 Road swap.** Let F be the full flag the carrier waits for, and
  R2 the road from F to the flag where its ware must go next (its route after
  F). The swap happens if R2 is a different road of the same kind (land or
  waterway), R2 has a carrier of the same kind (carrier or donkey), and that
  other carrier either
  - (a) holds no ware and stands idle and still on its idle field (not on a
    flag), or
  - (b) holds a ware and is itself waiting for room, and its ware blocks this
    carrier's in return: **the other carrier carries its ware towards F as
    well** (the ware's current goal flag is F), waits there for room, and
    that ware's next flag after F (route search from F at the moment of the
    check, TRN-CARRY-5) is the far end of this carrier's road. Each needs the
    other's road out of F. (Q-121; confidence high. That the compared
    ware field is its current goal flag is inferred from its use as the start
    of the next-flag search.)

  Then the two carriers **exchange roads**: this carrier becomes R2's carrier
  and continues with its ware from F along R2; the other becomes the carrier
  of this carrier's old road (in case b it continues with its own ware along
  that road; in case a it goes to that road's idle field). Wares are not
  exchanged. In case a the other carrier's way is **one walk** from its field
  along R2 to F and on along its new road to that road's idle field: at F the
  step's excess carries on (no stop, no new search there). The other carrier
  takes the walk up in **its own next update** (corrected 2026-10-31 night, T54):
  when it comes after the swapping carrier in the carriers' list (a higher
  object number), that is the swap's tick: the walk at progress 0 there,
  advancing from the next tick (T44); when it comes before (a lower number,
  already updated in the swap's tick), it stands through the swap's tick, is at
  progress 0 in the next tick and advances from the one after (T54).
- **TRN-CARRY-3.6a The swap at a flag off the carrier's own road (answered 2026-11-04, Q-1008;
  E-trn-030).** The original **does swap** there: none of the
  swap's checks asks whether F is an end of this carrier's road. F is simply the flag the carrier
  waits for, R2 the road out of F to its ware's next flag, and the conditions of 3.6 apply as
  written. **Case b in full (answered 2026-11-06, Q-1012; E-trn-031):** the other
  carrier (R2's carrier) must hold a ware and wait at a flag (refused, TRN-CARRY-3.2); its ware must
  have a destination; let G be that ware's current goal flag (the flag the other carrier carries it
  to and waits at) and NF the ware's next flag after G (route search from G at the moment of the
  check, TRN-CARRY-5). The test passes when **NF is one end of this carrier's road and G is the
  other end**, i.e. that ware's next leg is exactly this carrier's road, from G to NF. Nothing else
  is compared (not F, not where this carrier stands). In the normal case the other carrier waits at
  F, so G = F and NF must be this road's other end (3.6's wording). Off this carrier's road (F no end
  of it) G = F can never pass; the swap in case b then happens only when the other carrier waits
  at **R2's far end** (G = this ware's next flag) and this carrier's road begins there, with the
  other ware going on along it. Nothing in the swap routes anyone through F:
  - **This carrier** becomes R2's carrier, its ware's goal becomes the next flag, and it starts
    its carry walk again from where it stands (the road walk of 3.5a, with R2 now its own road).
  - **The other carrier** only has its road changed. In its own next update (timing as in 3.6) it
    finds itself off its new road's idle field and starts the same road walk to that field as
    any carrier going back to its road (`TRN-free-walk.md` TRN-FREE-6.1). The way "along R2 to F
    and on along its new road" of 3.6 is what that search gives when F is on the new road; off
    it, the walk goes wherever the search leads.
  - The next flag is always a neighbour of F, since it comes from the route search from F.
  Confidence: high that no such check exists and for the other carrier's walk (code); medium for
  the outcome in play (no recording of this case).
- **TRN-CARRY-3.3** At the other flag:
  - if the ware's destination is the building at this flag and the building
    takes it, the carrier walks into the building (one field, direction 4,
    plus the door step if the building has a door offset), and in the next
    tick the building receives the ware (no timed action; it counts for the
    road as in 3.4); the carrier then walks back out to the flag (one field) and
    goes on as after putting down. **A carrier bringing a ware to the building
    at the next flag does not make the room check of 3.2** (the ware never lies
    at that flag);
  - **"The building takes it"** (added 2026-11-02 night, T59; E-trn-029): the ware's next flag is that building's
    flag, and the building has no record for the good with *stop* or *out* set
    (`ECO-distribution.md` ECO-DIST-1.1; only warehouse-type buildings have
    such records, so other buildings always take their wares). This is tested
    in **every tick** of the walk: step 1 of 3.2 settles the entry only while it
    holds. When the building stops taking the good while the carrier is on its
    way (the computer player's goods arrangement sets *stop*, `AI-construction.md`
    AI-CON-10.4), the entry is no longer settled and the carrier makes the entry
    check of 3.2 at once (steps 2 to 4, as at any flag; a "no" while it is on its
    step onto F sends it back to the last field before F, T42). It then puts the
    ware down at F like any ware. A ware put down at the flag of the building it
    is bound for **keeps that destination**, with no next flag; `ECO-requests-
    dispatch.md` ECO-REQ-5.7 (a) or (c) takes it from there. Donkeys go into
    buildings in the same way as carriers. Confidence: high (code and T59);
  - **The building gone while he walks in (2026-11-16 day, Q-230 c; E-trn-038, code reading and
    the R6 recording, high; T60).** The walk into the building, once started, goes on to the
    building's field. In the tick after it ends (the hand-over tick) the hand-over finds no order
    for the ware (the removal cleared its destination, `ECO-requests-dispatch.md` ECO-REQ-5.5),
    so **nothing is delivered and the ware stays in his hands**; the road still counts the trip
    (3.4). He walks back out to the flag as after any delivery (the walk ordered in the hand-over
    tick, at progress 0), and on the flag, **holding a ware, he puts it down** like any ware
    (the drop-time and pickup-time actions of the next item). The ware then lies at the flag
    without destination and is sent to storage by ECO-REQ-5.5; carried on from there like any
    other ware (in T60 by the same carrier, his road's next job). He is not sent home: his road
    still exists. Nothing is lost;
  - otherwise it puts the ware down: a timed action of the drop time, then one
    of the pickup time. The ware lies at the flag (and is free for other
    carriers) when the drop-time action ends; it was already counted there
    since its reservation (TRN-CARRY-4.3). If an exchange ware is bound
    (TRN-CARRY-3.7) the carrier takes it in the same two actions and leaves
    with it when the pickup-time action ends. If the ware is not reserved at
    the flag and the flag is full when the drop-time action ends (possible
    only after a cancelled exchange or a refused building delivery), the
    carrier keeps the ware, makes the entry check every tick, and repeats the
    put-down (both actions) once it may enter (confidence medium).
- **TRN-CARRY-3.3a Ticks at the two flags (measured 2026-10-24, Q-104).** A tick in
  which a carrier's walk ends is spent on its job's next state; whether that tick
  already holds an action depends on the walk that ended (A = the arrival tick, the
  first row on the flag):
  - **end of the fetch walk at the ware's flag:** the pick-up's two actions start in A
    (drop-time action A .. A + 3, pickup-time action A + 4 .. A + 7) and the walk with
    the ware starts at progress 0 in A + 8. No standing tick;
  - **end of the carrying walk at the other flag:** the carrier stands in A; the
    put-down's drop-time action runs A + 1 .. A + 4 (the ware lies at the flag from
    A + 5), the pickup-time action A + 5 .. A + 8. Then: with an exchange ware
    (TRN-CARRY-3.7) the walk back with it starts at progress 0 in A + 9; without one
    the carrier stands once more in A + 9 and in A + 10 either starts its walk to the
    idle field at progress 0 or, for a ware ready for the way back (3.5), starts that
    ware's two pick-up actions, then walks with it in A + 18;
  - **into a building (the delivery of 3.3; added 2026-10-25 night, the
    implementation's first difference at 773):** the walk from the flag ends on the
    building's field in A; the carrier stands there in A and A + 1 (job state 25, then
    26), still holding the ware, and starts its walk back to the flag at progress 0 in
    A + 2, the tick from which the ware is gone (the building has received it). Sites
    and finished buildings alike. Into a **building with a door offset** (a warehouse,
    and the types of `ECO-buildings-catalog.md` ECO-CAT-2.4 such as the farm, **sites
    included**; corrected 2026-10-25 evening, T27: was "a warehouse") the walk goes on
    to the door field (the castle on (139, 51): (139, 52)); there the carrier
    stands only in A (state 26), still holding the ware, and walks back at progress 0
    in A + 1, the ware gone from A + 1. Both ways the step between the building's
    field and its door field is a walk of its own (`ECO-worker-cycle.md`
    ECO-WORK-3.8b): the walk in ends exactly on the building's field and the door
    step starts there at progress 0 in that tick; out, the door step ends on the
    building's field and the walk to the flag starts at progress 0 (measured
    2026-10-25 evening, E-trace-010: carrier 90260 bringing wood into the castle,
    on (139, 51)'s point in trace tick 2529, on the door field 2552, back on
    (139, 51) 2576, on the flag 2599; without the two restarts each leg is a tick
    short);
  - **out of a building after a delivery (3.3, first case):** it stands one tick on
    the flag, then walks or starts a pick-up in the next tick, as above;
  - **out of a building whose hand-over delivered nothing, the ware still in his hands
    (3.3, the building gone while he walks in; added 2026-11-17 night, measured T60,
    every row):**
    - He stands on the flag in A, as after any delivery.
    - From A + 1 he puts the ware down as at the end of a carrying walk, so each step comes
      one tick later than there: he stands in A + 1, the drop-time action runs A + 2 .. A + 5
      (the ware lies at the flag from A + 6), and the pickup-time action runs A + 6 .. A + 9.
    - He stands in A + 10 and takes his next job in A + 11. In T60 that job is the same ware's
      two pick-up actions (A + 11 .. A + 18), and the walk with it starts at progress 0 in
      A + 19.

    The put-down does **not** start in A; that reading is one or two ticks early;
  - **a warehouse's helper bringing a ware out:** the put-down's drop-time action
    starts in A (A .. A + 3, pickup-time A + 4 .. A + 7); then it stands four ticks
    as a settler going home and walks home. A new carrier from its warehouse stands
    in A (`ECO-worker-cycle.md` ECO-WORK-3.8d).
  The two pick-up actions last 4 ticks each (3.3 time units, TRN-CARRY constants).
  Confidence: high (a check of the recordings over E-trace-005 and 006,
  1098 stops of road carriers: 278 of 280 fetch-walk ends (the other two, carrier 90399
  at table tick 5806 of E-trace-005, stand one tick first, unexplained), 168
  carrying-walk ends without a ware back, 51 with an exchange, 4 with a second
  pick-up, 140 out of a building, 120 helpers, all in their patterns; into a building
  (a second check): 81 into sites and 24 into finished
  buildings with two standing ticks, 40 into the castle with one, none other; the
  ware's last tick in the carrier's hands from E-trace-007's wares table (the same
  game): sawmill site (138, 49), carrier 90234, arrival 1185, stands 1185, 1186,
  walk back 1187, plank 90294 held through 1186; castle, arrival at (139, 52) 3290,
  walk back 3291, ware 90454 held through 3290; carrier 90234 at (139, 48): arrives 632 standing,
  drop-time 633 .. 636, pickup-time 637 .. 640, stands 641, walk to its idle field at
  progress 0 in 642, table ticks, trace ticks one higher).
- **TRN-CARRY-3.4** Every ware delivered or put down adds 1 to the road's
  transport count (TRN-CARRY-6). **When it counts** (corrected 2026-10-30, Q-374,
  T50; E-trn-023): a ware put down at a flag counts when the carrier's
  put-down is over, that is, after both its actions (drop time and pickup time,
  3.3a), in the carrier's update of the tick in which its job moves on (A + 9 in
  3.3a's terms; the ware already lies at the flag from A + 5). A ware delivered
  into a building counts in the tick the building receives it (the second
  standing tick in the building, 3.3a). A carrier's update comes after the roads'
  update in the tick (TRN-CARRY-6.1), so a transport that
  counts in the tick in which the road's period ends belongs to the next period.
  Confidence: high (code; measured T50).
- **TRN-CARRY-3.5** After putting down, a carrier that holds an exchange ware
  (TRN-CARRY-3.7) carries it back at once, without pick-up actions. Otherwise,
  if a ware at this flag is ready for the way back, the carrier takes it
  (TRN-CARRY-2.5 step 1: drop time plus pickup time, 4 + 4 ticks) and carries
  it back; otherwise it walks to the idle field.
- **TRN-CARRY-3.5a Where a carry walk goes** (added 2026-10-29, T46, T47). A
  carrier that leaves a flag with a ware (one taken up there, 3.5 and 2.5, or an
  exchange ware, 3.7) carries it towards **that ware's next flag as the ware
  holds it** (5.4: computed when the ware came to the flag, or by a later
  re-check), not by rule to the far end of its own road. The walk is a road walk
  and ends at the first flag it reaches. Normally the next flag is the far end of
  the carrier's road, and nothing differs. It can differ when the ware's route
  changed after the carrier claimed it: the claim stays when the ware's
  destination is removed and when it gets a new one (ECO-REQ-5.5). The carrier
  then carries the ware along the neighbouring road, another carrier's road, to
  that flag. There it acts as at the end of its own road: into the destination
  building if that building is at this flag (3.3a, with the door walk), otherwise
  the entry check and the put-down (3.2, 3.3). Without a ware afterwards it
  returns over the roads to its own idle field: it stands one tick on the flag it
  came out to, then starts the road walk (TRN-FREE-6.3, the off-road case). A next
  flag more than one road away (kept from before a road split, 5.4) still ends the
  walk at the first flag reached, here the far end of its own road (T47).
  Confidence: high that the target is the held ware's next flag (code, E-trn-022,
  and T46); medium that the walk ends at the first flag (one case, T47). Not
  measured: putting the ware down at another road's flag, and an exchange or
  entry check there. **The carry walk's route** is the road walk search of
  `TRN-free-walk.md` TRN-FREE-6.1 with the carrier's **own road costing 1**, as
  for a walk to the walker's own road; every other road costs its length L
  (added 2026-11-01 night, T58: with all roads at L the walk takes the other
  way). Confidence: high (measured T58; E-trn-028, the walker's road
  costing 1).
- **TRN-CARRY-3.7 Exchange at the far flag.** Step 2 of the entry check: a
  carrier (or donkey) with ware W walking to flag F finds a ware X at F that is
  **ready for the way back**: X lies at F, has a destination, is not claimed,
  and X's next flag is the other end of the carrier's road (the flag it came
  from). With several such wares, the best by transport priority wins
  (TRN-CARRY-2.4). X is claimed for this carrier (no other carrier can take it)
  and the carrier may enter F regardless of room. At F it puts W down and
  picks X up within one drop-time plus one pickup-time action (4 + 4 ticks)
  and walks back with X; F's count is unchanged by the visit (W, never
  reserved, joins F's list when the drop-time action ends, in the tick X
  leaves it as the pickup-time action begins; a W that was reserved before X
  was claimed counts through the drop-time action as well, 4.3, T53). The check comes
  before the room check, so an exchange also happens at a flag with room.
  **W's next flag** (5.4) is computed when W lies at F and **before the carrier
  holds X**: in that search the carrier's own road counts its carrier as present
  and free (L, not the 3L of 5.3; added 2026-10-27 night, measured T31).
  **X's claim released on the way** (the matching gives X another destination,
  `ECO-requests-dispatch.md` ECO-REQ-5.3e): the exchange is undone at once, and the
  carrier, still walking, makes the entry check again from its next tick (another
  ready ware is bound, or it bids for a place as in 3.2; a "no" while it is on its
  step onto F sends it back to the last field before F, 3.2, T42); if this happens during
  its put-down action, X is not taken (ECO-REQ-5.3e, T24 there). (Added 2026-10-27 b,
  T38.)
  Workers bringing a product to their flag and a warehouse helper carrying a
  ware out never exchange (confidence high for workers, medium for the helper).
  If X is no longer at F when the carrier arrives (removed with a road or
  destroyed), the exchange is cancelled and W is put down as usual
  (confidence medium). **A carrier whose entry was settled otherwise** (W's place
  reserved, 3.2 step 4) looks again when its put-down begins (the drop-time
  action's first tick, A + 1 of 3.3a): a ware X ready for the way back by the
  conditions above is claimed then and taken in the same two actions, the walk
  back with it starting as after an exchange (measured 2026-10-25 evening,
  E-trace-010: every claim of a lying ware by a carrier standing on its flag, 6 of
  6, falls in A + 1; carrier 90282 with plank 90347, its place at (142, 55)
  reserved at counter 2386 while the wood 90416 was still in the woodcutter's
  hand (ready from 2395), arrived at 2501, claimed the wood at 2502 and walked
  back with it from trace tick 2511). Whether it looks again in later ticks of
  the put-down is not measured.
  **X's claim released in the arrival tick** (added 2026-11-01 night, T57, Q-381).
  When the matching takes X away in the tick the carrier stands on F (A of 3.3a),
  the carrier has no exchange ware at A + 1 and looks again there, as a carrier
  whose place was reserved: a ware ready for the way back (here one another
  carrier put down at F in A) is claimed in A + 1 and taken in the same two
  actions, the walk back with it at progress 0 in A + 9. W was never reserved
  (the exchange let it in): it joins F's list when the drop-time action ends.
  Without such a ware W is put down as after a cancelled exchange (3.3). Keeping
  the dropped claim as the exchange (no look in A + 1) adds a standing tick and a
  second 4 + 4 cycle. Confidence: medium (one case measured, T57; the A + 1 look
  is the one measured above). Not measured: a ready ware put down at F in A + 1
  itself.

### TRN-CARRY-4 Room at a flag

- **TRN-CARRY-4.1** A flag holds at most the flag package limit of wares (6),
  reserved wares (TRN-CARRY-4.3) included.
- **TRN-CARRY-4.2** Last place: when a ware with a destination and a next flag
  would become the 6th, it is only accepted if at least one of the 5 wares
  already counted there has a next flag and it differs from the new ware's
  next flag. A flag can therefore not be filled to 6 with wares that all leave
  in one direction. A ware without a destination or without a route is only
  counted: it takes the 6th place freely, and it never satisfies the
  "different next flag" test for another ware. A ware a **road carrier** is
  cleared to bring (4.3) satisfies it, since while carried its next flag is
  the flag itself.
- **TRN-CARRY-4.2a Wares brought out of a building (Q-364).** A ware a
  warehouse's helper (`ECO-requests-dispatch.md` ECO-REQ-5.3 b) or a worker
  (`ECO-worker-cycle.md` ECO-WORK-3.4) is cleared to bring out of its
  building counts at the flag from its clearance (4.3) but has **no next flag
  until it is put down**: until then it never satisfies the "different next
  flag" test of 4.2, whatever its destination. When its put-down ends it lies
  at the flag and its own next flag (TRN-CARRY-5.1) counts from then on, as
  for any lying ware. The warehouse's own test (fewer than 6 counted at its
  flag, ECO-REQ-5.3 b) has no direction part, so a warehouse can keep filling
  its flag's 5th and 6th places with helpers while a road carrier whose ware
  leaves the flag the same way as all the wares counted there waits for the
  6th place (T28). Confidence: high (code E-eco-019; measured, E-trace-010 counters 5196 to 5600).
- **TRN-CARRY-4.3 Reserved places.** A ware counts as lying at F for the room
  rules 4.1 and 4.2, for the route costs of TRN-CARRY-5.3 and for a
  warehouse's "fewer than 6 wares at its flag" test (`ECO-distribution.md`,
  `ECO-requests-dispatch.md`) from the tick its carrier, worker or helper is
  cleared to bring it (entry check step 4), not from the tick it is put down.
  It keeps counting without a gap until its put-down ends, **the arrival tick
  included**: a road carrier whose walk ends on the flag stands there one tick
  before its put-down begins (3.3a), and in that tick its place still counts,
  for the room rules and for the 4L of 5.3 alike (clarified 2026-10-27 b,
  T40).
  A ware is counted **once**: as reserved until its put-down ends, as lying
  from then on, so in the tick its put-down ends (a carrier's, worker's or
  warehouse helper's) the flag counts it only as lying, for any test made later
  in that tick (clarified 2026-10-27 b, T41: the place is the ware's own entry
  in the flag's list, not a separate count).
  A reserved ware is claimed and so not available to carriers until it is put
  down. Its carrier makes no further entry checks.
  **A reserved ware whose carrier claims an exchange ware as its put-down
  begins** (the carrier that looks again, 3.7) keeps its place through the
  drop-time action, like any put-down: the place ends when that action ends
  and the ware lies at F from then on. The exchange ware lies at F and counts
  until the pickup-time action begins, when it leaves F's list. So in the ticks
  of the drop-time action F counts both wares, one more than before the
  carrier came and after the exchange. Only an exchange settled at the entry
  check (3.2 step 2, no place reserved) leaves F's count unchanged all through
  (3.7). (Added 2026-10-31, Q-377, measured T53.)
- **TRN-CARRY-4.4 Wait score and entry order.** Every ware has a wait score,
  0 when created and whenever it enters or leaves a flag. Each failed entry
  check at a full flag (step 3) adds `15 - p` to it, `p` the ware's transport
  priority number (0 to 14): a coin gains 15 per check, a plank 8, a donkey 1.
  Every flag has a **bar**, -1 when created. With room at the flag, step 4
  compares the ware's score `s` with the bar `b`: if `s > b` the bar becomes
  `s` and the ware may not enter yet; if `s < b` it may not enter; if `s = b`
  it is admitted (reserved, 4.3) and the bar is reset to -1. The bar is also
  reset to -1 whenever a ware leaves the flag (picked up or destroyed) and when
  two carriers swap roads (3.6). Effects: a lone carrier walking to a flag with
  room is admitted at its second check (the next tick, so no visible delay);
  among several wares waiting for a flag that just got room, the one with the
  highest score is admitted first and the others bid again after it; with
  equal scores the second ware to check after the bar was set enters first.
  Confidence: high for the rule, medium for how many checks fall while the
  carrier is still walking (the score grows per check, not per tick).

### TRN-CARRY-5 Ware routes

- **TRN-CARRY-5.1** A ware's next flag is found by a cheapest-path search over
  the flag network (flags as nodes, roads as edges) from the flag it lies at to
  the flag of its destination, among roads of the same network (land roads
  and waterways are separate for this search; how a waterway's boat carrier
  reaches its flags and whether wares cross from one network to the other at
  a shared flag stays open, Q-198 a).
- **TRN-CARRY-5.1a Which roads a ware may use (E-sim-007, 2026-10-24).** The search of
  5.1 runs only over roads that have their carrier or donkey **assigned**: a road gets
  one in the tick its carrier request is served (the carrier made at the warehouse door,
  not when it arrives, `SIM-game-start.md` SIM-START-4.4 (e)) and loses it when the
  carrier is released (TRN-ROAD-6.3) and no donkey is left. Flags joined by such roads
  form a ware network (the second of a flag's three network numbers, entry 1, the ware
  network of `SIM-game-start.md` SIM-START-4.3; renamed 2026-10-25, Q-352);
  a ware whose flag and destination flag are not in one ware network has no next flag
  (5.5), even when roads without a carrier would connect them. The order matching, the
  storage choice and the re-check (`ECO-requests-dispatch.md` ECO-REQ-5.3, 5.5, 5.7,
  `ECO-distribution.md` ECO-DIST-1.6) use the same networks. Confidence: high (E-trace-006:
  after the removal at counter 386 the hunter site's planks could not be ordered from the
  castle until both new roads had their carriers, 417 and 447, and were served at the
  first free door after that, 484 and 517).
- **TRN-CARRY-5.2** Road length `L`: the sum over the road's steps of
  `1/f(A->B) + 1/f(B->A)`, cut to a whole number, where `f` is the slope speed
  factor of `TRN-settler-movement.md` TRN-MOVE-1.3. A level step adds 2, so a
  level road of `k` steps has `L = 2k`.
- **TRN-CARRY-5.3** Cost of using a road towards flag Y:
  - `4L` if Y already holds the flag package limit of wares;
  - otherwise `3L` if neither the road's carrier nor its donkey is present and
    free (a place is free when it is filled and that settler carries nothing);
  - otherwise `L`.
  **Only a search in the ware network (kind 1, SIM-START-4.3) counts 4L and
  3L**; a search in the land network (kind 2) or the all-roads network (kind 0)
  counts `L` for every road, whatever its carrier or its end flag (corrected
  2026-10-27 night, E-trn-017; measured `ECO-requests-dispatch.md`
  T18). So the settler requests' warehouse choice (ECO-REQ-3.2, kind 2) adds
  plain road lengths.
  **"Filled" means assigned** (clarified 2026-10-24 night, E-ai-010): a
  carrier or donkey counts from the tick its request is served, while it is
  still walking from the warehouse to the road, as long as it carries no
  ware; arriving at the road is not required. A carrier on its way to a
  ware or back to its idle field without one is free; one carrying a ware is
  not. The 3L applies only to the ware-network searches that ask for it: ware
  routing and the ware route costs that the ware orders, the storage choice and
  the computer player compare (not the settler requests, kind 2)
  (`AI-construction.md` AI-CON-8.3); the route optimizer's list of route
  flags is made without it. Confidence: high (the code; in an experiment
  build, counting an arriving carrier as busy makes the castle road of
  E-trace-006 cost 3L = 24 at counter 253 instead of L = 8 and moves the
  computer player's route optimizer onto the wrong variant, T32).
- **TRN-CARRY-5.3a Stored route costs (2026-10-25 evening, Q-362, E-ai-017).**
  The flag-to-flag route costs that warehouse searches compare are not always
  computed afresh. Each player keeps one store of flag-pair costs per
  network kind (SIM-START-4.3: 0 all roads, 1 ware network, 2 land network).
  A cost of a pair asked before, in either direction, is given from the store
  unchanged. Otherwise the cheapest route from the first flag to the second is
  searched in that kind's network, counting 5.3's 4L and 3L at the moment of
  the search in the ware network only (kinds 0 and 2: L per road, 5.3). Its cost, or "no route", is stored. A flag
  to itself costs 0. The store of kind m of player p is emptied whenever a
  renumbering of kind m (SIM-START-4.3) writes a new id into one of p's
  flags, and only then. For the ware network that is a road of p getting its
  carrier while its two ends are in different ware networks (4.3 (b)), or a
  road of p with a carrier being removed or split (4.3 (c)). So a busy
  carrier, a full flag or a new road inside one ware network leave stored
  costs as they were. The stores belong to the game state (in the
  multiplayer checksum and the saved game). Users and their kind: the ware
  order matching of `ECO-requests-dispatch.md` ECO-REQ-5.3 (kind 1); the
  storage choice of ECO-REQ-5.5 (kind 1: the cost from the ware's flag to each
  candidate warehouse's flag; added 2026-10-25 evening, T26, E-trn-015); the
  settler requests' warehouse choice of ECO-REQ-3.2 to 3.4 (kind 2; kind 0
  for the boat carrier; soldiers kind 2); the computer player's route
  optimizer (`AI-construction.md` AI-CON-8.3 b, c; kind 1) and its
  recruitment score (`AI-military.md` AI-MIL-6.1; kind 1, asked from each
  depot's flag to every smithy and brewery, sites included, every 409 ticks:
  this is what puts warehouse-first pairs into the ware store, measured
  `AI-military.md` T22); the warehouse
  search of going home (`TRN-free-walk.md` TRN-FREE-6.4 step 2; the walker's
  kind); the harbour choices of ships (kind 1). The ware routing of 5.4
  (next flag) and the road walks of TRN-FREE-6.1 are separate searches and
  not stored. Confidence: high for the store and its emptying (code); measured for kind 1 in ECO-REQ-5.3 and
  AI-CON-8.3 (`AI-construction.md` T36, T37); the other users medium (code).
- **TRN-CARRY-5.4** The next flag is the first flag after the current one on the
  cheapest route. **When it is searched** (corrected 2026-10-27 b, E-trn-020; T39):
  - A ware's **own** next flag (the one it waits for while lying at a flag,
    5.1) is searched afresh when the ware is put down at a flag and when the
    re-check of `ECO-distribution.md` ECO-DIST-1.6 renews it
    (`ECO-requests-dispatch.md` ECO-REQ-5.7); between those it is kept.
  - Every **other** question for the flag a ware takes after a flag G is
    answered from a one-entry memory of the ware: the last-place test of 4.2
    for a ware being brought to G (G = the flag it is brought to) and the road
    swap of 3.6 (the flag after the one a waiting carrier's ware is brought
    to). If the ware was last asked from the same G and G's ware-network number
    (`SIM-game-start.md` SIM-START-4.3, entry 1) is still the one it had then,
    the last answer is given unchanged; otherwise the search of this rule runs
    with the 5.3 costs of that moment and its answer is remembered with G and
    that number. The fresh search of the ware's own next flag (put-down,
    re-check) clears the memory.
  **The last-place test asks only when the ware would take G's last place**
  (G counts limit - 1 wares; added 2026-10-28 night, E-trn-020: the
  memory's question is reached only then; measured T43): a check at a full G,
  or with two or more places free, asks nothing and leaves the memory as it
  is, so the first answer comes from the first check at which G counts 5.
  So busy carriers, full flags and new roads inside one ware network change
  none of these answers once given: a carrier waiting before a full flag F
  keeps the direction its ware got at its first last-place check of F, even
  when a tie or a cheaper way appears later (T39). Confidence: high (code; measured T39).
  **What the key decides (Q-369, measured 2026-10-28):** G's number changes
  only when a renumbering of SIM-START-4.3 (b) or (c) reaches G's own network;
  a renumbering of another of the player's ware networks (a separate group of
  flags, such as two flags whose road just got its carrier and is not yet
  joined to the rest) leaves the memory valid. A key that is renewed at every
  ware renumbering of the player instead gives the same answer at every
  recorded check of E-trace-011 (Weak, 20 000 ticks: 5 such checks, all at
  counter 17448 .. 17456) and E-trace-012 (Hard, 20 000 ticks: 2, at 12017,
  both from a flag with one way only), and of E-trace-013 (Hard, 150 000 ticks)
  up to counter 74295 (of its 644 ware renumberings, those that leave a waiting
  ware's G unchanged come often from about 74000 on); from 74295 the answers differ, at flags
  that are full either way, so the room test of 4.2 comes out the same, and
  whether a road swap (3.6) on the 200-tick grid differs (first at 99005) is
  not settled from the tables.
  **Equal costs** (added 2026-10-27 night, E-trn-018, measured T30): the costs of 5.3 are searched from the ware's
  flag over the ware network, and the route is then read out from the
  destination back to the ware's flag as the road walks are (`TRN-free-walk.md`
  TRN-FREE-6.1): at each flag the first road in the flag's road list
  (`TRN-roads-flags.md`, the order the roads were attached; a split's halves
  last) whose other end lies on a cheapest route (its cost plus that road's
  5.3 cost towards this flag equals this flag's cost). The next flag is the
  last flag reached before the ware's own.
- **TRN-CARRY-5.5** If no route exists the ware has no next flag and stays where
  it is.
- **TRN-CARRY-5.6** Which building a ware is sent to (its destination) is not
  part of this spec (economy distribution, open).

### TRN-CARRY-6 Donkeys

- **TRN-CARRY-6.1** Each road counts ticks. When the count passes 6000 (every
  6001 ticks, about 10 minutes): `traffic = L x transport count`,
  `average = (average + traffic) / 2` (start 0), both counts reset. **The
  average is a single-precision float, not rounded** (corrected 2026-10-30,
  T52; E-trn-023: (L x count + average) x 0.5 kept as a float and
  compared with 130.0): halves accumulate, so a road can pass 130 by a fraction.
  **Where in the tick** (added 2026-10-30, Q-374; E-trn-023): the clocks run in the roads' update, after the players'
  buildings and request queues and before the settlers; a road counts only once
  both its flags are attached. A road starts with tick count, transport count and
  average 0, so a road made in counter B (the computer player's roads are made in
  the AI's part, the last of the tick) is first counted in B + 1 and its periods
  end in B + 6001 k, where the donkey request of 6.2 is filed at once (measured
  on every road of E-trace-014 whose creation tick the tables hold exactly: road
  92023 made 22212 asks at 34214, 92439 made 25782 at 31783). A transport counted
  in such a tick goes to the next period (3.4).
- **TRN-CARRY-6.2** The first time the average is greater than 130, the road is
  marked busy and asks for a donkey. Waterways never ask. The request is made
  once; how the donkey is delivered is not traced.
- **TRN-CARRY-6.3** Example: a level 2-step road (`L = 4`) with average 0 gets a
  donkey after one period with at least 66 wares (264 / 2 = 132 > 130); 65
  wares give exactly 130, which is not enough.

### TRN-CARRY-7 Removing a road under a carrier

- **TRN-CARRY-7.1** A carrier whose road no longer exists goes home: it walks to
  a warehouse of its player. If it cannot reach one it dies (a timed action of
  the settler dying time, 28 ticks, with the skeleton doodad of
  `cfg_setSettlerSkeletonDoodad`). The same going-home rule applies to every
  released settler (builder, leveller, worker) and is specified in
  `TRN-free-walk.md` TRN-FREE-6.4.
  **The skeleton doodad (corrected 2026-11-19 night, E-mil-027: code reading and the war recording's
  object tables; high).** Every dying settler (any kind: a settler who found no warehouse, a soldier
  killed in a duel, `MIL-attack-combat.md` MIL-ATK-6.6) gets the doodad **during** his dying, not
  after it: in his update in which 14 ticks of the 28 have passed (dying from tick t, in tick
  t + 14), the doodad the config names (`cfg_setSettlerSkeletonDoodad`; `doodad_deadsettler` of
  the shipped `doodads.lua`, not blocking) is put on his sample: the lattice point he stands on when
  he is on the fine lattice, else his field's point; nothing when the config names none or a doodad
  already stands on that point. Made at that moment, it takes the next object number
  (`SIM-game-start.md` SIM-START-4.2), in his place in the order of the settlers' part. Test
  `MIL-attack-combat.md` T31a.
- **TRN-CARRY-7.1a When and how the released carrier leaves (E-trace-006, 2026-10-24).**
  Removing the road only takes the road away from the carrier; the carrier notices it
  in **its own next update**, and in that update:
  1. its movement is advanced first, as in every update: a carrier walking a step
     keeps walking it at its rate;
  2. then it becomes a settler going home (trace state 3); a ware in its hands is
     destroyed now (TRN-CARRY-7.2), so its carrying look ends in this tick.
  A road the computer player removes (the last part of a tick, `AI-overview.md` AI-2.1a)
  is therefore noticed in the next tick: road removed in counter 386, carrier 90204 in
  state 3 from counter 387, its plank gone in 387.
  It does not turn back and does not jump onto a field: **the walk it is on is
  completed** at its normal rate, every step of it, not only the step in progress,
  **but it ends on the first field it reaches that is a flag or is no longer a road
  field** (corrected 2026-10-29, measured T49: a road replaced by two new ones over a
  new flag; the carrier walks on over the old fields that the new road covers and stops
  on the first old field that no road covers any more; when every field ahead is still
  a road field this is the walk's last field or the next flag, as in T34) (corrected 2026-10-27 b, T34: carrier 90206 of E-trace-012, two steps from
  the flag it carried to, walked both), also a step into a building (carrier 90261,
  walking into the site it brought a plank to, finished that step and stood on the
  site's field). It stops on that field. Then, counting the tick in which the step
  ends (progress -1 in the trace) as A: it stands in A and A + 1; in A + 2 the way home
  is chosen from that field (TRN-FREE-6.4) and its first walk starts at progress 0;
  the walk advances from A + 3. A carrier standing when it notices the removal (tick R)
  stands through R + 2 and starts its walk at progress 0 in R + 3 (the same three steps
  with R + 1 in the place of A). The way home is a road walk over the player's roads as
  they now are, the new roads without carriers included. A carrier keeps one trace row
  throughout. Confidence: high for the walking case (two releases measured, the step's
  end and the ticks A .. A + 3 in both; E-sim-007);
  high for a carrier with more than one step left (it does not stop at the next field,
  T34); medium for the standing case.
  **A carrier still on its way to its road** (the road walk of TRN-CARRY-1.4 over other
  roads) when its road is removed: its walk ends on the next flag it reaches (as
  TRN-FREE-6.5 (g)), and when that step ends in the very tick it notices the removal
  (R), it counts as standing: it stands in R, R + 1 and R + 2 and starts its walk home
  at progress 0 in R + 3 (measured T45, added 2026-10-29; whether a longer walk to the
  next flag is cut after the step in progress is not measured). The extra standing tick
  is for that case only. A carrier **already on its own road**, walking to the idle
  field when the road goes, is the walking case above: it completes the walk to the idle
  field, stands in A and A + 1 and starts home in A + 2 (measured T48).
- **TRN-CARRY-7.1b A step that ends inside the removed road (Q-266, 2026-10-24).** When
  the field a released carrier stops on (7.1a) is an inner field of its removed road, the
  way home of A + 2 is still TRN-FREE-6.4, all of whose tries run in that one tick, in
  order: (1) nothing is found from the field itself, which holds no flag, building or
  road any more (the removal took the road off its fields, TRN-ROAD-6.1), unless a new
  road has been built over it since (the computer player's street execution removes a
  road and builds its replacements in one think, AI-CON-8.1 b: then (1) finds that road
  and the carrier road-walks home, T95 of `AI-construction.md`); (2) from the
  field's south-east neighbour (direction 1) only if a flag, building or road lies
  there; (3) the nearest finished, accepting warehouse of its player on the same landmass.
  When (2) or (3) finds a warehouse, the carrier free-walks (TRN-FREE-2) to that
  warehouse's network flag nearest to it, if that flag is within the range of
  TRN-FREE-6.4 (30 fields); the walk starts at progress 0 in A + 2. **The removed road's
  fields are not walked as a road.** In the tick that walk ends the carrier looks at the
  field north-west of the flag (direction 4): if a warehouse stands there it goes in,
  its step ordered in that tick at progress 0 with no standing tick on the flag (as
  TRN-FREE-6.4's end of a walk, T32; measured T55);
  otherwise it chooses its way home again in that same tick, now from the flag (try (1)
  finds it), and its road walk to the cheapest accepting warehouse starts at progress 0
  in that tick. If every try fails in A + 2 it retries as in TRN-FREE-6.4 step 4.
  Confidence: high for the order of the tries, the free walk to a network flag and the
  re-choice at its end (E-sim-008); medium for
  the ticks of the re-choice (from the code; both measured releases, T22 and T23, ended
  on a flag or a building's field and took try (1)).
- **TRN-CARRY-7.2** When the released carrier starts going home, a ware it
  holds is destroyed and a ware it had only claimed stays at its flag, free
  again; wares lying at a removed flag are destroyed with it. Details in
  `TRN-roads-flags.md` TRN-ROAD-6 (Q-139). Confidence: high.
- **TRN-CARRY-7.2a Every settler sent home loses the ware in his hands (2026-11-16 day, Q-230 a,
  b; E-trn-036, code reading, high).** Going home (`TRN-free-walk.md` TRN-FREE-6.4) is one state
  shared by all settlers, and its start does what 7.2 says for every settler, not only carriers: a
  ware **in his hands** is destroyed (its order, if any, open again), a ware he had only claimed at
  a flag is released and stays there. A worker carries a real ware once he has picked his product
  up: the woodcutter's log and the stonemason's stone are made at the pickup at the tree or pile
  and are in his hands from there to the flag. So a worker whose building is removed while he
  carries his product home loses it at once: it goes into no warehouse and is not put down. A
  settler who later dies homeless therefore never holds a ware.
  Test: a woodcutter carrying a log to his hut's flag when the hut is torn down: the log is gone
  in that tick; the player's wood stock does not change when he reaches a warehouse.
  Confidence: high for the going-home step; medium-high that the woodcutter's and stonemason's
  pickup is the one at the tree or pile (the shared take action makes a new ware of the job's good when he holds none).

### Transport priority (shipped `Goods.lua`)

| Number | Goods |
|---|---|
| 0 | coin |
| 1 | sword, shield, weapons (group) |
| 2 | beer |
| 3 | iron |
| 4 | gold |
| 5 | iron ore |
| 6 | coal |
| 7 | plank |
| 8 | stone |
| 9 | wood |
| 10 | water |
| 11 | pig, corn, flour, fish, flesh, bread, food (group), ship (group) |
| 12 | all tools (gripper, axe, saw, hammer, shovel, pot, fishing rod, scythe, pickaxe, rolling pin, spear, hatchet), tools (group) |
| 13 | boat |
| 14 | donkey; ship helper (profession) |

Whether the player can change the order in the game, and whether the game's
table can lack a good (it would then count as 0, the highest), is not traced.

## Constants

| Name | Value | Unit | Confidence | Evidence ref |
|---|---|---|---|---|
| idle field | list entry floor(n/2) | | high | E-trn-001 |
| idle look-for-work period | 20 | ticks, phased by the carrier's object number | high | E-trn-001, E-trace-005 |
| waiting-for-room retry period | 10 | ticks, phased | high | E-trn-001 |
| deadlock swap period | 200 | ticks | medium | E-trn-001 |
| no-ware priority | 100 | | high | E-trn-001 |
| entry check order (building, exchange, full, bid) | | | high | E-trn-007 |
| exchange ware: ready, unclaimed, next flag = far end of this road, best priority | | | high | E-trn-007 |
| exchange visit | drop time + pickup time, 4 + 4 | ticks | high | E-trn-007 |
| wait score increment | 15 - priority | per failed check | high | E-trn-007 |
| flag bar initial / reset | -1 | | high | E-trn-007 |
| reservation at clearance | ware in the flag's list | | high | E-trn-007 |
| tie between the two ends | second flag | | high (code), low (which flag is "second") | E-trn-001 |
| pick up / put down | drop time + pickup time, 4 + 4 | ticks | high | E-trn-001, E-eco-004 |
| route cost factors | 1, 3 (no free carrier), 4 (far flag full) | x road length | high | E-trn-001 |
| road length per level step | 2 | | high | E-trn-001 |
| traffic period | 6001 | ticks | high | E-trn-001 |
| donkey threshold | average traffic > 130 (a float average, T52) | | high | E-trn-001, E-trn-023 |

## Test cases

Tick 0 is the tick in which the carrier, standing on its idle field, finds the
ware (that is, a tick with `(carrier number + tick) mod 20 = 0`). Level ground,
default scripts. Event ticks are derived from the rules; each transition may be
one tick off (confidence medium), totals within 2 ticks.

| ID | Setup | Action | Expected |
|---|---|---|---|
| T1 | road A-M-B (2 steps), carrier idle at M, one plank at A whose next flag is B, B empty | carry it | tick 23 at A, the drop-time action 23 .. 26, the pickup-time action 27 .. 30; walk with the plank at progress 0 in 31; 76 at B, standing; drop-time 77 .. 80; 81 plank lies at B; pickup-time 81 .. 84; stands in 85; walk to M at progress 0 in 86 (TRN-CARRY-3.3a) |
| T2 | as T1 | plank appears at A in a tick where `(carrier number + tick) mod 20 = 7` | carrier starts 13 ticks later |
| T3 | as T1, plus a coin at B going to A, both appear together | which first | the coin (priority 0 < 7) |
| T4 | as T1 with a plank at A and a plank at B, each ready for the road | which first | the plank at the second flag (tie, TRN-CARRY-2.5) |
| T5 | as T1, after putting the plank down at B a stone waits at B for A | after tick 85 | carrier takes the stone at once, no walk to M |
| T6 | as T1 but B holds 6 wares | carry | carrier stops before B and waits; resumes within 10 ticks after B has room |
| T7 | B holds 5 wares, all with next flag C | a 6th ware with next flag C arrives | not accepted; with one of the 5 going to D it is accepted |
| T8 | flags A, B, C; roads A-B (2 steps) and B-C (2 steps), and A-C (5 steps), level, all carriers idle | route of a ware at A for C | over B (cost 4 + 4 = 8 < 10) |
| T9 | as T8 but the A-B carrier is carrying | route | direct A-C (12 + 4 = 16 > 10) |
| T10 | level 2-step road (L = 4), average 0 | 70 wares carried in the first period | traffic 280, average 140 > 130: donkey requested after tick 6001 of the road |
| T11 | as T10 with 60 wares | first period | average 120: no donkey; a second period of 60 gives 180: donkey requested |
| T12 | carrier on a road that is removed, warehouse reachable | observe | carrier walks to the warehouse |
| T13 | flags E, F, G; roads E-F (carrier X) and F-G (carrier Y); F holds 6 wares; X carries a ware for G towards F, Y carries a ware for E towards F, both waiting | 200-tick check of X | swap: X becomes F-G's carrier and carries its ware from F to G; Y becomes E-F's carrier and carries its ware from F to E. If Y's ware were for a flag beyond G instead (next flag after F not E), no swap |
| T14 | road A-M-B (2 steps); B holds 6 wares, among them one log with destination beyond A (next flag A); carrier picks a plank up at A (next flag B) | walk to B | the carrier never stops: the log is claimed for it when its walk starts; at B after 4 + 4 ticks it leaves with the log towards A; B holds 6 wares throughout (plank in, log out) |
| T15 | as T14 but B holds only a stone whose next flag is A | walk to B | exchange as in T14 although B has room: 8 ticks at B, no walk to M, back with the stone |
| T16 | as T14 but the log at B has next flag C (beyond B), no ware at B heads to A | walk to B | no exchange; B full: the carrier stops before B and waits (10-tick checks); when a ware leaves B it bids, enters at the second successful check and puts the plank down |
| T17 | flag B holds 5 wares (4 with next flag C, 1 with next flag D); carrier X at A is cleared to bring a plank with next flag C to B; a worker in the building at B then finishes a product | worker's entry check | B counts 6 (the plank is reserved); the worker waits inside until a ware leaves B, although only 5 wares lie there |
| T18 | flag B full; carrier X (coin) and carrier Y (log) wait for B, both since 50 ticks (5 failed checks each: X score 75, Y score 30); a ware leaves B | next checks | X sets the bar to 75 and waits; Y's check fails (30 < 75); X's next check admits it (bar -1); Y bids again once B has room again |
| T19 | headquarters flag H, flag F, level road H-M-F (2 steps), carrier C idle at M; F holds 6 logs with destination HQ (next flag H); H holds 6 planks with destination a site beyond F (next flag F); C picks a plank up at H | 2000 ticks | under the previous rules C stops before F and nothing ever moves (the road swap needs a second road). Under these rules: C binds the oldest log at F at the start of its walk, swaps at F (8 ticks), carries the log to H and into the HQ, takes the next plank at H, and repeats: one plank out and one log in per round trip, no carrier ever waiting, F and H never above 6 |
| T20 | E-trace-002, road 90199 of the computer player: flags 90191 (hunter site, (141, 48)) and 90182 (castle, (140, 50)), list (141, 48) (140, 48) (139, 49) (140, 50), carrier 90204, logic ticks | carrier's way and idle field | sent from the castle in 207 (first step (139, 52) -> (139, 51) at progress 0 in 207, 0.045 in 208); on (139, 51) in 230 with the next step at 0; on the castle's flag in 253 standing; walk to (139, 49) at 0 in 254, 0.045 in 255; on (139, 49) in 277 standing, and still standing in 280 (idle entry 2 of 4, TRN-CARRY-1.3) |
| T21 | TRN-CARRY-2.1, E-trace-005: road carrier 90204 (the computer player's first road, idle field (139, 49)) idle from counter 254; the helper's ware for the hunter site lies at the castle's flag (140, 50) | the tick the carrier starts its fetch walk | counter 296 (trace tick 297): 90204 + 296 = 90500, a multiple of 20; likewise 90234 at 546, 90237 at 643, 90260 at 800, 90261 at 979, 90282 at 1018 (every one of the 93 idle-field starts of the recording) |
| T22 | TRN-CARRY-7.1a, E-trace-006: computer player's road 90199 ((141,48) (140,48) (139,49) (140,50)) removed in counter 386; carrier 90204 walking (140,48) -> (141,48) with a plank, progress 0.43 at 386 (+0.045 per tick) | carrier 90204, counters 386 .. 402 (trace tick = counter + 1) | 386 still state 24, carrying; 387 state 3, progress 0.475, same step, plank 90208 gone, walking without a ware; 398 progress 0.97; 399 and 400 on (141,48) standing; 401 walk (141,48) -> (140,48) at progress 0; 402 progress 0.018; then (140,48) at 457, (139,48) at 494, (139,49) 517, (140,50) 544, (139,51) 567, in the castle (state 2 on (139,52)) at 590. One row per tick, never on (140,48) before 457 |
| T23 | TRN-CARRY-7.1a, E-trace-006: road 90252 ((140,46) (140,47) (141,48)) removed in counter 1541; carrier 90261 walking from the flag (140,46) into site 90244's field (139,47) with a plank | carrier 90261 | state 3 by 1543 (recorder gap at 1541 and 1542), the step into (139,47) completed: standing there at 1558 and 1559, walk (139,47) -> (139,48) at progress 0 at 1560; plank 90266 gone |
| T24 | TRN-CARRY-1.4, `ECO-worker-cycle.md` ECO-WORK-3.8d, E-trace-005: carrier 90237 sent from the castle (139, 51) for road 90228 ((139, 48) .. (141, 48)); the castle's flag (140, 50) is not an end of that road | the carrier's ticks on the castle's flag | walking (139, 51) -> (140, 50) at 0.99 in table tick 492; on the flag standing in 493; walk to (139, 49) over road 90224 at progress 0 in 494, 0.045 in 495. A constructor on the same flag (90202, ECO-WORK-3.8c) has its step ordered in the arrival tick |
| T25 | TRN-CARRY-2.4a, E-trace-007: the castle's flag (140, 50); road "140,50 141,50 141,49 141,48" with carrier 90260; plank 90243 (destination the hunter site 90190 at (140, 49)) carried back from (139, 48) by carrier 90234 (picked up at trace tick 764), plank 90266 (destination the site 90244) carried out of the castle by helper 90265 (created 770) | which plank 90260 takes at trace tick 978 (both lying, both with next flag (141, 48), same priority) | 90243: it joined the list at 770 (admitted on 90234's walk), 90266 at 772 (on the helper's walk), though 90266 lay at the flag one tick earlier (822 against 823); 90260 then carries 90243 into the site 90190 at 1168 |
| T26 | 5.3a, ECO-REQ-5.5, `AI-construction.md` AI-CON-8.3 c: E-trace-010, player 1; the ware renumbering of counter 3308 empties the store; the wood 90506 made at counter 3362 by the woodcutter of the hut with flag (142, 55); the castle's flag (140, 50) full when its storage destination is chosen | the stored pair (142, 55), (140, 50) and the route task's run of counter 3509 | the storage choice stores cost (142, 55) -> (140, 50) = 4 x 10 = 40 (4L: the road "142,55 142,54 141,53 141,52 140,51 140,50", L 10, into the full flag; at counter 3475 in an experiment build); at 3509 the route optimizer's direct variant for the sawmill 90211 (partner flag (142, 55), w = 5, Fn = (142, 55), X's last (140, 50), d = 5) asks the pair the other way and gets the stored 40 > 4 x 5 = 20: road request, S finished, k = 3 (measured: ai_route k 2 -> 3 in 3509, and the road request's object number 90523 in the counter table). A fresh cost (140, 50) -> (142, 55) is 10 and keeps the task on S (an experiment build until 4582 differs from 3509) |
| T27 | 3.3, 3.3a: E-trace-010, carrier 90554 bringing a ware into the computer player's farm site (136, 58) (frame stage; flag (136, 57); door field (135, 59)) | the delivery | on (136, 58)'s point in trace tick 4994 and walking on at once (the door step at progress 0), on the door field (542000, 236000) in 5017, standing there in 5017 only, walking back at progress 0 in 5018 (measured). With the delivery rule for buildings without a door the carrier stands on (136, 58) in 4994 and 4995 and walks back from there |
| T28 | 4.2, 4.2a: E-trace-010, player 1, the castle's flag (140, 50) at counter 5196: four wares lying there, all with next flag (141, 52); the castle's helper 90696 cleared with plank 90697 for the brewery site 90561 (136, 51) (put down at trace ticks 5197 .. 5201); carrier 90234 of the road from (139, 48) waiting on (139, 49) since trace tick 5020 with plank 90660 for the hunter site 90625 (144, 58), whose next flag after (140, 50) is (141, 52) | 90234's entry checks (counter + 90234 = 0 mod 10) and the castle's sends | 90234 is refused at every check of 5196 .. 5596 (the helper's plank counts as the 5th ware but passes no direction test; from 5201 it lies there with next flag (141, 52) like the others) and walks on only at trace tick 5607, putting down at 5631; meanwhile the castle sends plank 90701 for the brewery site in the matching of counter 5225 (helper 90700, five wares counted) and stone 90703 in 5291 (measured). If the helper's plank passed the test as a reservation, 90234 would be admitted at 5196 (walking at trace tick 5198) and the castle's flag would be full in 5225 |
| T29 | 3.2, 3.2a, 3.7: E-trace-010, carrier 90234 waiting on (139, 49) (road (139, 48) .. (140, 50)) since trace tick 5020 with plank 90660 for the castle's full flag (140, 50); the helper 90746 puts plank 90747 (for the hunter site 90716 (141, 46), next flag (139, 48)) down at that flag, lying from trace tick 5597 | 90234's checks (counter + 90234 = 0 mod 10) | refused at 5596 (the plank not yet lying), admitted by the exchange at counter 5606; its row in trace tick 5607 shows Walk at progress 0 on (558000, 196000), its first step (558090, 196180) in 5608 (measured) |
| T30 | 5.4 (equal costs): E-trace-011, player 1, counter 10139: a castle helper puts iron 91086 for the smithy 90481 (137, 55) (flag (138, 54)) down at the castle's flag (140, 50); the two first roads: to (137, 50) (L 6, its carrier holding a ware: 3L = 18, then 16 to the smithy's flag) and to (141, 52) (L 4, then 30) | the ware's next flag | both 34; read out from (138, 54) by the flags' road lists the route comes over (141, 52): next flag (141, 52) (measured: wares table; at 10160 the carrier 90611 of that road, out of the castle, takes it (3.5) and leaves at 10169). Taking the first flag settled by the forward search gives (137, 50): the carrier leaves empty for the far flag (an experiment build: settlers first differ at 10162; with the read-out at 10224) |
| T31 | 3.7, 5.3, 5.4: E-trace-011, player 1, flag (137, 46) at counter 11598: carrier 91072 of the road (137, 46) - (136, 47) - (137, 48) (2 steps) brings water 91201 (for the castle, flag (140, 50)) and exchanges it for plank 91126 lying there; road (137, 46) - (139, 48) (3 steps) has its carrier 90399 carrying; the castle's flag holds the limit | 91201's next flag | (137, 48), the way it came (the recorded wares table from 11598): with 91072 free its road costs L, so the route over (137, 48) and (137, 50) costs 4 + 4 + 24 = 32 against 18 + 16 = 34 over (139, 48) (costs in the experiment build's units, twice the steps). Counting 91072 as carrying X already (3L = 12, 40 against 34) sends 91201 over (139, 48), and 90399 takes it back from (137, 46) at 11633: settlers first differ at 11634 and the generator at 11838; with the rule 11807 and 12615 |
| T32 | 5.4 (equal costs, the read-out by attachment order; `TRN-free-walk.md` TRN-FREE-6.1, Q-280): E-trace-011, player 1, counter 12535: plank 91267 for the guard house site 91075 (flag (144, 55)) put down at (137, 46); two routes of cost 48 (6 + 42 over (139, 48), (140, 50), (141, 52), (142, 55); 4 + 44 over (137, 48), (137, 50), (137, 52), (138, 54), (139, 56), (141, 56), (142, 55)); at (142, 55) the road to (141, 56) dates from counter 1309, the road to (141, 52) is a split half made at 4339 (the flag (141, 52) of building 90590 split the old road) | 91267's next flag | (137, 48) (recorded; carrier 91072 claims it at 12548): the read-out from (144, 55) takes at (142, 55) its first-attached road whose other end is on a cheapest route, the one to (141, 56). Ordering a flag's roads by storage slot, where the split's first half keeps the old road's slot (an experiment build), takes (141, 52) and gives (139, 48): settlers first differ at 12550; with attachment order 12938 (buildings 13394 -> 14450) |
| T33 | 2.2: E-trace-011, player 1, carrier 90260 of the road (140, 50) - (141, 48), idle field (141, 49); after an exchange at (140, 50) it walks back from trace tick 14616; ware 91404 (for the guard house 91373) lies at (140, 50) with next flag (141, 48) from counter 14627 | when it claims 91404 | it stands on (141, 49) from trace tick 14661 (counter 14660, where (90260 + 14660) mod 20 = 0) without looking and claims the stone at counter 14680, the next tick of its grid; looking in the arrival tick (an experiment build) turns it round at once and the settlers first differ at 14661; with the rule at 16564 (buildings 15181 -> 17537, generator 14764 -> 16610) |
| T34 | 7.1a: E-trace-012 (computer player Hard), player 1's road 90201 "141,48 140,48 139,49 140,50" (castle flag (140, 50) to (141, 48)); its carrier 90206 carries plank 90210 from (140, 50) towards (141, 48); the computer player removes the road at counter 386 (roads 90226 "139,48 139,49 140,50" and 90230 "139,48 140,48 141,48" made instead) | the carrier's way after the removal | in state 3 from 387 (plank destroyed), between (139, 49) and (140, 48); it walks on without a stop over (140, 48) (trace tick 402: 1180 units past it) to (141, 48), the last field of its walk (about 418), stands there, and from trace tick 420 road-walks home over (140, 48), (139, 48), (139, 49), (140, 50) into the castle. Stopping at (140, 48) (an experiment build) stands there 396 .. 397 and walks home west from 398: settlers first differ at 402; with the rule at 1003 (the Weak pairs unchanged) |
| T35 | 3.2: E-trace-011, player 1, carrier 91502 of the road 91500 "142,55 143,56 142,57 142,58 141,58" picks up wood 91537 (for the castle) at (141, 58) and walks from trace tick 16592; flag (142, 55) holds 5 wares, all with next flag (141, 52), the wood's next flag from there is (141, 52) too (4.2: no 6th place) | where the carrier waits | it walks on over (142, 58) and (142, 57) and stands on (143, 56) from 16659 until its check of counter 16808 admits it; walk from 16809, the wood put down on (142, 55) at 16836 with next flag (141, 52). Stopping at the next field reached (an experiment build) stands on (142, 57) from 16637 (settlers first differ at 16637, the admission tick 16809 the same); with the rule at 17056 |
| T36 | 3.2, 4.4: E-trace-011, player 1: (a) carrier 91502 picks up wood 91537 at (141, 58), its walk to (142, 55) ordered at counter 16591 (trace tick 16592, progress 0), the flag full for it; (b) carrier 91575 with wood 91451 from (142, 55) towards (141, 52), walk ordered at 16884; at (141, 52) a stone waiting carrier (checks on the counters ending in 8) and 91575 (ending in 5) bid when the flag gets room at 17048 | the wares' wait scores (wares table) and 91575's admission | (a) score 0 at 16591, 6 at 16592, 12 at 16593 (+6 a check, from the tick after the order); (b) 456 at 17045; the stone's check of 17048 sets the bar to its score 462, 91575's check of 17055 (456 < 462) does not enter, its check of 17065 does (walk from trace tick 17066). With a check in the ordering tick too (an experiment build) 91575's score is 462 at 17055, equal to the bar, and it enters 10 ticks early (settlers first differ at 17056) |
| T37 | 2.2: E-trace-011, player 1, donkey 91753 for road 90224 "139,48 139,49 140,50" (request 91752, served at counter 18411), out of the castle onto its flag (140, 50), an end of the road; there stone 91616 (for the site 91705) lies with next flag (139, 48) | its first ticks on the road | standing on (140, 50) in trace tick 18458; it claims the stone at counter 18458 (wares table) and its pick-up runs from trace tick 18459 (PutDown 18459, Pickup 18463, carrying from 18467). Walking on to the idle field without the look (an experiment build) leaves the stone for later (settlers, view columns left out, first differ at 18460, generator 18509; with the rule 18595 and 18826). Carrier 91072 of the road (136, 47) .. (137, 46), whose walk from the castle passes over the flag (137, 46) to its idle field (136, 47), makes no look on the flag (stands on the idle field from 10201, claims at its grid tick 10208) |
| T38 | 3.7: E-trace-011, player 1, carrier 90611 of the road (140, 50) - (141, 52) carries stone 91779 from (140, 50) (walk ordered at counter 18761); at (141, 52) wood 91421 and wood 91537 lie for the castle with next flag (140, 50) | the exchange partner | 91421 bound at counter 18762 (wares table); the matching of 18799 gives 91421 another destination and releases it; at 18800 he binds 91537 instead; on the flag in trace tick 18807, put-down from 18808, carrying 91537 back from 18816. Keeping the stale exchange until the arrival (an experiment build) finds it gone there, asks again on the flag and starts two ticks late (settlers, view columns left out, first differ at 18817; with the rule at 19886, generator 18992 -> 19895) |
| T39 | 4.2, 4.4, 5.4: E-trace-011, player 1: water 91813 bound for the castle (flag (140, 50)), carried by 90584 on the road (137, 52)-(137, 50) towards the brewery's flag (137, 50) (flag 90563), which holds 5 wares all going on to (140, 50) | its entry checks and the brewer's beer | its checks every 10 ticks from counter 19446 all fail at the last place (score 150 at 19440, 330 at 19800, 375 at 19890: +5 a check); at 19796 a fresh search from (137, 50) would tie at 18 between the direct road (3L = 18, its carrier 90956 carrying beer 91824) and the way over (137, 48), (137, 46), (139, 48) (4 + 4 + 6 + 4, the carrier 90399 of (137, 46)-(139, 48) just free), and the read-out would take the latter, giving the water the 6th place: the bar would rise to 325 and keep the brewer's new beer 91861 (score 0) out. The original keeps the first answer (140, 50): the bar stays -1, the beer is admitted at 19894 (its flag set in the wares table) and brewer 90919 walks out at 19895. Experiment build (memory per ware, kept until put-down, key the flag and the player's ware renumberings): Weak dense pair view-free settlers 19896 -> 19973, the 150 000 pair 19900 -> 19980, 6000 pair equal, buildings and generator unchanged |
| T40 | 4.3, 5.3, 5.4: E-trace-011, player 1, flag (141, 56) (flag 90328) at counter 19967: 4 wares lying and 2 places reserved (stone 91847 of worker 90480, cleared at 19791 on his walk back; ware 91842 of carrier 90456, cleared at 19917) | the search of a ware put down at (141, 60) in that tick | carrier 90456 ends his walk on (141, 56) at 19967 (standing), puts down 19968 .. 19971; the flag counts 6 through 19967 (wares table: 6 rows with its flag from 19953 to the put-down). Ware 91851 (bound for the castle) put down at (141, 60) at 19967 gets next flag (139, 56): the roads into the full (141, 56) cost 4L. Not counting the standing carrier's place gives 5, next flag (141, 58), and carrier 91149 of (141, 58)-(141, 60) walks to fetch it at 19972, which the original never does. Experiment build: the Weak dense pair equal to 20 000 apart from the recorder's animation lag (17533); the 150 000 pair's settlers and buildings equal as far as recorded; Hard unchanged |
| T41 | 4.3, 5.3: E-trace-012 (player 1, Hard), the castle's flag (140, 50) at counter 5585: wood 90844 put down by a castle helper in that tick, ware 90846 still carried out by the next helper 90845 (wares table: 4 lying, 1 carried with the flag set) | the storage run of 5585 (t mod 5 = 0) giving meat 90825 (ad0d6c63, put down at (141, 48) by worker 90415 at 5581 without a destination) the castle | the flag counts 5, not full: the road (141, 48)-(141, 49)-(141, 50)-(140, 50) costs its plain length and the ware's next flag is (140, 50); carrier 90262 of that road walks to fetch it at 5598 ((90262 + 5598) mod 20 = 0). Counting the put-down ware twice (lying and still reserved) gives 6, 4L on both roads into the castle's flag and the next flag (139, 48). Experiment build: Hard pair settlers 5599 -> 5679, the other pairs unchanged |
| T42 | 3.2, 3.7 (a refusal on the step onto F): E-trace-013 (R3, the Hard game), player 1, carrier 90543 of the road (137, 54) - (137, 55) - (137, 56) carrying stone 91404 to the flag (137, 56), bound to the wood 91440 lying there for the castle (next flag (137, 54)); he is on his step from (137, 55) onto (137, 56) when the matching of counter 10835 gives the wood to the sawmill 90517 (136, 55) and releases it | the carrier | 10836: the wood has no next flag yet (the re-check of that tick comes after), the stone's bid raises the flag's bar to 0 (inferred: see 90510 below): no; 10837: the step is cancelled, he stands on (137, 55) (trace tick 10838: Stand at (550000, 220000)), no check; his checks fall where (90543 + counter) mod 10 = 0: 10847 binds the wood (exchange) and orders the walk (Walk from trace tick 10848), on the flag at 10871, put-down 10872, pickup 10876 (measured). Carrier 90510's plank 91399 (from (139, 56)) matches that bar at its first check and has its place from 10842. Finishing the step (an experiment build without the rule) puts him on the flag at trace tick 10854: the Hard pair's settlers first differ at 10838; with the rule at 10841 (ECO-WORK T-room-2), buildings 11393 -> 11526, generator 11202 -> 11215 |
| T43 | 5.4 (when the memory is asked), 4.2: E-trace-014 (R4, the Weak game), carrier 90611 of the road (141, 52) - (140, 50) brings flour 91926 for the bakery 90650 to the castle's flag (140, 50) (walk from counter 24268); the flag counts 6 from 24270 (4 lying wares with next flag (139, 48), two helpers' waters 92304, 92307 counted from their clearances), 5 from 24339 (the waters lying, 92311 a helper's) | his checks | the flag after (140, 50) towards the bakery is (137, 50) at 24269 (the road to (139, 48) at 3L, its carrier 90234 still holding bread), (139, 48) from 24270 to about 24300, (137, 50) again from then (table costs); the checks at the full flag (24273 .. 24329, score + 4 each) ask nothing; the check of 24339 (5 counted) asks: (137, 50), differing from the lying wares' (139, 48): room, the bar rises; 24349 admits (place from 24349, walk from trace tick 24350; measured). Asking at every check (an experiment build) remembers (139, 48) from 24273 and the last place is refused until a ware with another next flag lies there (admitted 24399): the Weak pair's settlers first differ at 24351, with the rule at 24729, generator 24361 -> 25152 (the 150 000 pair's too); the Hard pair, the Weak pairs to 20 000 and the single-player pair unchanged |
| T44 | 3.6 case a, the other carrier's walk: E-trace-013 (R3, the Hard game), player 1, the corn 91472 carried by 90510 towards the full flag F (137, 56) (90497), its next flag after F becoming (137, 58) at counter 12689 (trace tick 12690); carrier 91018 of the road F - (137, 58) idle on its idle field (137, 57) | the swap at trace tick 12691 | 90510 goes on along F - (137, 58) (Walk at progress 0 in 12691); 91018 is at progress 0 on (137, 57) in 12691 too, moves from 12692, is 19 units short of F in 12713 and 140 units past it towards his new road's idle field in 12714 (the excess carried over F), recorded. An experiment build that advances him in the swap tick, or ends his walk on F and orders the walk on from there, puts him one tick early from 12691, resp. on F in 12714: the Hard pair's settlers (without the animation column) differ from 12691 resp. 12714; with both rules from 13056 (R3: 12700 -> 13060), the other pairs unchanged |
| T45 | 7.1a, a carrier on its way to its road: E-trace-014 (R4, the Weak game), player 1's new carrier 92805 of the road 92800 (126, 58) - (126, 60), walking west over other roads towards it (x 520323 on (131, 58) in trace tick 29181); the road is removed at counter 29181 | his ticks | trace state 28 in 29182 (x 520143), on the flag (130, 58)'s point in 29183 with state 3 (R = A), standing in 29183, 29184, 29185, walking home (east) at progress 0 in 29186 (recorded). Walking on to the removed road's idle field, or standing only in A and A + 1: the Weak game against R4 differs from settlers 29187 resp. 29186; with the rule from 29197 (experiment build), the other pairs unchanged |
| T46 | 3.5a, a re-routed exchange ware: E-trace-014 (R4, the Weak game), player 1. Carrier 90584 of road 90579 (137, 50) - (137, 52) claims the plank 92810, lying at (137, 50) with next flag (137, 52), for the site 92768 (counter 29142). The site is removed at 29145; at 29150 the plank gets the castle 90180 (flag (140, 50)) as its destination and next flag (140, 50), still claimed by him. He arrives at (137, 50) in 29187 with the water 92840 and exchanges the two (29188 .. 29195) | his rows from trace tick 29196 | walks east along road 90954 (137, 50) - (140, 50) from 29196 (x 548180 in 29197), passing that road's carrier 90956, who fetches the water; into 90180 from (140, 50) (29266 .. 29309); out on (140, 50), standing in 29356; road walk at progress 0 in 29357 back over road 90954 to his idle field (137, 51), standing there from 29452 (recorded; wares and streets tables). Carrying to the far end of his own road (137, 52) gives a Weak vs R4 difference at settlers 29197. With the rule: 30165, buildings 33400, generator 32990 (experiment build, before 29197 / 30330 / 29835); every other pair unchanged |
| T47 | 3.5a, a next flag two roads away: E-trace-014, player 1. Carrier 90282 holds the exchange ware 90557, whose next flag is (140, 50), while the flag (141, 52) splits its road (counter 4339). The new road 90601 is (142, 55) - (141, 52) | his walk from (142, 55) (trace tick 4365) | ends on (141, 52), which is his road's far end and the first flag on the way to (140, 50). He puts the ware down there (lying from 4439, next flag (140, 50)) and walks to his idle field (141, 53) (recorded; the Weak game against R4 agrees on these ticks with the rule and without it) |
| T48 | 7.1a, on its own road to the idle field: E-trace-014 (R4, the Weak game), player 1's carrier 92901 of road 92896 (125, 63) - (129, 64), walking west along it to its idle field (127, 64) (x 510047 on (128, 64) in trace tick 30150). The road is removed at counter 30157 | his ticks | going-home state from trace tick 30159; on (127, 64) at 30162 (A), standing in 30162 and 30163, walking home at progress 0 in 30164 (recorded). Three standing ticks (as T45) differ from the Weak game against R4 at settlers 30165; with the walking case: 30946 (experiment build) |
| T49 | 7.1a (the walk ends where the road is gone): E-trace-014 (R4, the Weak game), player 1's carrier 91776 of road 91774 "137,44 136,43 136,42 135,41 135,40 135,39 136,38 136,37 137,37", carrying from (137, 44) towards (137, 37) since trace tick 33470. At counter 33482 the computer player replaces the road by 93454 (135, 37) - (137, 37) and 93458 (135, 37) ... (136, 42) (136, 43) (137, 44) over the new flag (135, 37) | his walk | on over (136, 43) and (136, 42), now fields of road 93458, to (135, 41), which no road covers (about counter 33542). There he turns: walking back east from about 33544, on (136, 42) - (136, 43) at 33570 (recorded every 10th tick; streets, flags, objects tables). Completing the walk to (137, 37) differs from the Weak game against R4 at settlers 33550. With the rule: 34650, buildings 35190, generator 34900 (experiment build; Hard unchanged) |
| T50 | 3.4, 6.1 (Q-374): E-trace-014 (R4, the Weak game), player 1's road 92105 "130,35 131,35 132,34 132,33 133,32 133,31 133,30" (6 steps, L 12), made in counter 22604, its carrier 92110 | the traffic periods and donkey requests | first period ends 28605 with 4 transports (average 24); in the second, 19 transports counted up to 34431; the 20th ware is put down at (133, 30) (lying there from counter 34602) and counts in 34606, after the period closed in that counter's roads update: second period 19, average (24 + 12 x 19) / 2 = 126, no donkey request (none recorded up to 40 000). Counting when the ware lies at the flag (34602) gives 20, average 132, a request at 34606 that shifts the castle's queue (the leveller 93603 a tick early, settlers first differ at 34650; with the rule 34840, buildings 35190 -> 35420, generator 34900 -> 35240, the 150 000 generator 34889 -> 35211, other pairs unchanged). Road 91573 (made 16644) asks at 34647 under both rules |
| T51 | 3.2 (a refusal on the step onto F that the next movement completes): E-trace-013 (R3, the Hard game), player 1, carrier 90594 of the road 90592 "138,53 137,53 137,54" carrying 92542 to the flag (137, 54), bound to the ware 92712 lying there (for the castle, next flag (138, 53)); he is on his step from (137, 53) onto (137, 54) (progress 0.926 after the movement of counter 20075) when the matching of 20075 gives 92712 to 92686 and releases it | the carrier | 20076: progress 0.971, his check (not settled, every tick on this step) is refused (inferred from the wait on F that follows); 20077: the movement completes the step, he stands on (137, 54) (progress -1), no check; checks only where (90594 + counter) mod 10 = 0: 20086 admits him; 20087 his arrival tick (job state 31), the ware lying from 20092; 92712 is claimed by carrier 90543 in 20077. Cancelling the step (the T42 outcome) puts him back on (137, 53) in 20077: with T82 and `AI-overview.md` AI-2.4a the R3 settlers first differ at 20080; with the rule (experiment build) at 22110 (buildings 22280 and generator 22107 unchanged, every other pair unchanged). T42 (refused at progress 0.26, the step not completed by the next movement) keeps its outcome |
| T52 | 6.1 (the float average): E-trace-014 (R4, the Weak game), player 1's road 90676 (L 4, made in counter 5048) | its periods and its donkey request | periods end 11049, 17050, 23051, 29052, 35053 with 17, 20, 24, 41, 35 transports: averages 34, 57, 76.5, 120.25, 130.125; at 35053 the average passes 130 and the donkey request 93673 is filed (measured). Rounding each average down (34, 57, 76, 120, 130) never passes 130: no request, the next object numbers one lower, and the Weak pair's settlers first differ at 35310 (with T40); with the float average at 37360 (buildings 35830 -> 38170, generator 35360 -> 37930, the 150 000 generator 35358 -> 37742; the other pairs unchanged) |
| T53 | 4.3, 3.7, 5.3 (Q-377): E-trace-013 (R3, the Hard game), player 1, the flag (132, 33) (flag 92427). Carrier 92493 of the road (132, 33)-(135, 31) arrives there with 94172 (its place reserved since counter 30737) in 30824 and claims the lying iron ore 94224 (next flag (135, 31)) in 30825, its drop-time action 30825 .. 30828, pickup-time 30829 .. 30832. Carrier 93345 of the road (131, 34)-(132, 33) picks up the iron ore 94245 at (131, 34) and his walk to (132, 33) is ordered in 30826; 94245's flag after (132, 33) is (134, 38), as for the four other wares lying there | 93345's entry checks and the coal 94048 put down at (140, 46) in 30852 (for the smeltery at flag (133, 31)) | (132, 33) counts 6 in 30825 .. 30828 (four lying, 94224 lying, 94172 reserved), so 93345 is refused at 30827 and 30828 (wait score 0 -> 10 -> 20, the recorded score); in 30829 94172 lies (next flag (131, 34)) and 94224 has left: his check binds 94172 for an exchange, and no place is ever reserved for 94245 (it lies there from 30876). In 30852 (132, 33) counts 5, so the road (134, 38)-(132, 33) (L 16, its carrier free) costs L and the coal's cheapest route is (140, 46) (139, 44) (137, 43) (136, 41) (135, 39) (134, 38) (132, 33) (135, 31) (133, 31), cost 78: next flag (139, 44), as recorded. Dropping 94172's place at the exchange claim admits 94245 in 30828 (place reserved), (132, 33) counts 6 in 30852, that road costs 4L = 64, and the coal goes to (141, 45) (cost 97). The 5.3 search run on R3's recorded state gives the recorded next flag for all eight put-downs at (140, 46) between 30462 and 31193. Experiment build: Hard R3 settlers 30880 -> 32470, buildings 32190 -> 33340, generator 31949 -> 32824; every other pair unchanged |
| T54 | 3.6 case a, the other carrier before the swapping one: E-trace-018 (R5, the Weak game), player 1, carrier 92469 of the donkey road 91948 (133, 30) - (135, 30) waiting before the full flag (133, 30); carrier 92208 of the road 92255 (133, 30) - (131, 26) idle on its idle field (132, 28) | the swap at counter 47931 (the roads table: 91948's carrier 92208, 92255's 92469 from 47931; they swap back at 48192) | 92469 walks on at progress 0 in 47931; 92208 (the lower number) stands in 47931, is at progress 0 towards (132, 29) in 47932 and 0.027 in 47933 (trace ticks 47932 .. 47934). Ordering his walk in the swap's tick, as for a carrier after the swapping one (T44), puts him one tick early: the Weak game against R5 (E-trace-018) differs from settlers 47933 (experiment build); with the rule from 48528, every pair of the fidelity set unchanged |
| T55 | 7.1b (going in at the network flag): E-trace-013 (R3, the Hard game), player 1's carrier 94890, released on his idle field (139, 25) (state 3 from counter 36258), free-walking to the flag (138, 25), north-west of which (direction 4) stands the storehouse (138, 26) | his ticks at (138, 25) | progress 0.99 on the step to (138, 25) in 36283; in 36284 on (138, 25) at progress 0 towards (138, 26), 0.045 in 36285; on (138, 26) in 36307, going in (recorded, every tick). A standing tick on the flag first puts him one tick behind: the Hard game against R3 differs from settlers 36290 (experiment build); with the rule from 37930 (buildings 37880 and generator 38004 unchanged; experiment build, every other pair unchanged) |
| T56 | 3.2 (no check in the stopping tick), 4.4: E-trace-013 (R3, the Hard game), player 1's carrier 93008 with the ware 95559 (score 198 after its last refused check at counter 41861, by 4.2's last place), walking to the flag (135, 31) (flag 92989, 5 lying wares); at 41861 the ware 95570 leaves the flag (the bar back to -1) and 95860 (next (132, 33)) is put down, so the last place is free for 95559; its step onto (136, 31), the last field before the flag, ends in counter 41862 | its checks | 41862 (a tick of its 10-tick grid, 93008 + 41862 = 0 mod 10): none; 41872: it bids (score 198 > bar -1: the bar becomes 198, not admitted); 41882: admitted (reserved; its walk on from trace tick 41883, recorded). A check in 41862 sets the bar there and admits it at 41872: the Hard game vs R3 differs from settlers 41874. With the rule (experiment build): Hard settlers 41900 -> 43100 (the pair's rows every 100th tick); every other pair's first difference unchanged |
| T57 | 3.7 (the exchange ware taken away in the arrival tick, Q-381): E-trace-018 (R5, the Weak game), carrier 90802 of the road (136, 57) - (132, 58) carrying 95639 to the flag 90786 (132, 58), its exchange ware 95728 lying there (claimed, for the castle 90180, next (136, 57)); in counter 49082 95728 gets the destination 95647 and loses its claim, 90802 arrives on (132, 58) (standing, trace 49083), and 95692 (next (136, 57)) lies at (132, 58) from 49082, put down by carrier 91504 | 90802's ticks at (132, 58) | 49083: 95692 claimed for 90802; drop-time action 49083 .. 49086, pickup-time 49087 .. 49090 (95639 down, 95692 up in 49087); walks back with 95692 at progress 0 in 49091 (trace 49092). Treating the dropped claim as still bound: 95639 put down, a standing tick, a second 4 + 4 cycle for 95692, walking in trace 49101: the Weak game against R5 differs from settlers 49093. With the rule (experiment build): settlers 50151, buildings 51126 against R5, the 150 000 generator 50113 unchanged; every other pair unchanged (the Hard 45700 / 46700 / 45865, Fight 784) |
| T58 | 3.5a (the own road costs 1 in the carry walk): E-trace-019 (R6, MP_4P_Fight), player 1's carrier 15038 of the road 15153 "53,53 53,52 52,51" (L 4; made by the split of 14998 at table tick 2272), taking up the ware 14982 at (53, 53) (claimed before the split; for the building 14432, next flag still (54, 49), two roads away). Roads at counter 2320: 15156 "54,49 53,49 53,50 52,51" (L 6), 15057 "53,53 54,52 54,51" and 15060 "54,49 54,50 54,51" (L 4 each; heap dump of 2500) | his carry walk | pickup-time action to trace 2323; walk from (53, 53) at progress 0 in trace 2324 towards (53, 52); on (52, 51) in trace 2383, the put-down there (the walk ends at the first flag, his road's far end); the ware lies at 15149 (52, 51) from 2387 with next flag (54, 49) (recorded). Route costs: own road 1 + 6 = 7 against 4 + 4 = 8. With every road at L (4 + 6 = 10) he walks over (54, 52): the Fight game against R6 differs from settlers 2325. With the rule (experiment build, with the Fight rules before it): settlers 2325 -> 3663; every pair of the fidelity set unchanged |
| T59 | 3.3 (the building stops taking the good on the way), 3.2, ECO-REQ-5.7 (c): E-trace-013 (R3, the Hard game), player 1's donkey 95957 of the road (143, 45) - (144, 45) - (145, 46) carrying the plank 96452 to the harbour 90366 (144, 47) (flag 90367 (145, 46), its next flag); the goods arrangement of counter 45647 (step 0, `AI-construction.md` T100) sets *stop* for planks on the harbour and the castle | the donkey and the plank | entry settled by step 1 from its walk's start; in 45648 the harbour no longer takes planks: the check runs, the flag is full, the donkey is on its step onto (145, 46) and stands back on (144, 45) in 45649 (trace 45650); admitted on its grid at 45653 (95957 + 45653 = 0 mod 10), on (145, 46) in 45681, the put-down 45682 .. 45689; the plank lies there from 45686 with destination 90366 and no next flag, loses it at the storage run of 45690 and gets the storehouse 94359 at 45695 (recorded). The castle's plank 96454 (carrier 91084, flag 90344) goes the same way (lying 45657, destination dropped 45660, 94359 at 45665). Settling the building case once (ours) carries both into the buildings: the Hard game against R3 differs from settlers 45650; with this rule, the acceptance test of step 1 and T100 (experiment build) settlers 45723, buildings 46675 -> 46985 (the Hard 150 000 pair at its 100th-tick rows: settlers 45700 -> 45800, buildings 46700 -> 47000, generator 45865 -> 45969; every other pair unchanged); dropping the destination at the put-down instead gives 45693 |
| T60 | 3.3, the building gone while he walks in (Q-230 c): E-trace-019 (R6), player 1's carrier 16778 of the road from the flag (57, 59), carrying ware 16677 for the site 16688 on (57, 60); the site removed during counter 8399 while he walks in (counters 8392 .. 8412 at 0.045 a tick) | his and the ware's rows (counters) | on the building's field in 8413 (the hand-over tick: no delivery, the ware stays his, its destination already empty since 8402); walking out from 8414 at progress 0 (0.027 a tick), on the flag in 8452 (as trace rows, tick = counter + 1: Walk 99 at 8413, Stand 0 on the building's field at 8414, Walk 0 still on the field at 8415, the first move at 8416: **one** standing row; a build that orders the walk out a tick after the hand-over stands two rows and differs from 8416, measured by the tick-exact work 2026-11-17 night); putting it down 8452 .. 8461 (3.3a: standing 8452 and 8453, drop-time 8454 .. 8457, the ware lying at the flag 16689 from 8458, pickup-time 8458 .. 8461; corrected 2026-11-17 night, was "at the flag in 8456"); its destination the headquarters 14158 from 8460 (storage, ECO-REQ-5.5); standing in 8462; from 8463 taking the ware again (drop-time 8463 .. 8466, the ware in his hands from 8467, pickup-time 8467 .. 8470); walking with it at progress 0 in 8471 (trace rows: Stand 8453, 8454; PutDown 8455 .. 8458; Pickup 8459 .. 8462; Stand 8463; PutDown 8464 .. 8467; Pickup 8468 .. 8471; Walk 0 at 8472). Not sent home. A build that sends him home (or loses the ware) differs from the fight game's generator at trace tick 8456 (experiment) |

## Evidence

Each evidence id with the kind of evidence it is. Where in the original program each was found is kept in the investigation's private records under the same id.

| Ref | Kind | Where | Note |
|---|---|---|---|
| E-sim-008 | code reading | | TRN-CARRY-7.1b |
| E-trn-030 | code reading | | road swap off the carrier's own road |
| E-trn-001 | code reading | `S2DNG.exe` | carrier update and state machine; its donkey / boat variant |
| E-trn-001 | code reading | | carrier construction: its 8 state objects |
| E-trn-001 | code reading | | idle field and look-for-work period |
| E-trn-001 | code reading | | work choice between the two flags; best ware at a flag; priority lookup |
| E-trn-001 | code reading | | carrying, waiting for room, deadlock swap |
| E-trn-001 | code reading | | pick up and put down |
| E-trn-001 | code reading | | room at a flag, last-place rule |
| E-trn-001 | code reading | | ware route: search, cost per road, next flag, cache |
| E-trn-001 | code reading | | road length |
| E-trn-001 | code reading | | traffic average and donkey request |
| E-trn-001 | code reading | | settler without work goes home; dies if it cannot |
| E-trn-002 | code reading | | 10/200-tick checks while waiting; road swap conditions and exchange |
| E-trn-004 | code reading | | case (b): other ware's goal flag is F, next flag after F is the far end of this road |
| E-trn-002 | code reading | | pickup: ware leaves the flag when the drop-time action ends |
| E-trn-002 | code reading | | a new walk replaces the current one from the last reached field |
| E-trn-002 | script | `Goods.lua`, `Jobs.lua` `gp_setTransportPriority` | the priority table |
| E-trn-007 | code reading | | entry check: order building / exchange / full / bid; no further checks once reserved |
| E-trace-002 | recording | `settlers.csv.gz`, `streets.csv.gz`; carrier 90204, road 90199 | idle field and the carrier's first way (1.3, T20) |
| E-trn-007 | code reading | | exchange ware at the far flag: ready, unclaimed, next flag = this road's other end, best priority; claim |
| E-trn-007 | code reading | | last-place rule over the list including reserved wares; wait score; admit (reserve) and unlink reset the bar; bar -1 at creation |
| E-trn-007 | code reading | | check every tick while walking, every 10 while waiting; put-down with exchange pickup in one visit; retry of a failed put-down; carry back at once |
| E-trn-007 | code reading | | workers and the warehouse helper use the same entry check without exchange |
| E-trn-013 | code reading and recording | E-trace-010 `wares`, `settlers` (claims of lying wares by carriers on their flags at A + 1; carriers 90260, 90282) | TRN-CARRY-2.2, 3.7, 3.3a |
| E-trace-005 | live read of the running game | `settlers.csv.gz` (class 2 rows, progress -1 to a step), `streets.csv.gz` (idle field = points entry floor(n/2)) | TRN-CARRY-2.1: 93 of 93 idle-field starts at residue 0 |
| E-trn-015 | code reading and recording | E-trace-010 `counter.csv` (the next object number per counter) |
| E-sim-007 | code reading and recording | E-trace-006 `settlers.csv.gz`, `objects.csv.gz` (carriers 90204, 90261) | 7.1a: the release noticed in the carrier's next update, the step completed, A .. A + 3; 5.1a: ware routes over roads with an assigned carrier or donkey |
| E-trn-017, E-trace-011 | code reading and recording | streets and objects tables of E-trace-011 (roads 91060, 91048, requests 91065, 91066) | TRN-CARRY-5.3 kinds; ECO-REQ T18 |
| E-trn-018, E-trace-011 | code reading and recording | wares and settlers tables of E-trace-011 (iron 91086, carrier 90611) | TRN-CARRY-5.4 (T30) |
| E-trn-019, E-trace-011 | code reading and recording | settler and wares rows of carrier 90260 and ware 91404, counters 14600 .. 14700 | 2.2 (T33) |
| E-trn-020, E-trace-011 | code reading and recording | wares and settlers tables of E-trace-011 (water 91813, beer 91861, carriers 90956, 90399) | TRN-CARRY-5.4 (T39) |
| E-trn-029, E-trace-013 | code reading and recording | E-trace-013 `goodflags.csv.gz`, `wares.csv.gz` (counters 45640 .. 45700) | TRN-CARRY-3.3, T59 |
| E-trn-023, E-trace-014 | code reading and recording | E-trace-014 `streets.csv.gz` (roads' first ticks), `objects.csv.gz` (donkey requests d4157c5f) | TRN-CARRY-3.4, 6.1, T50 |
| E-trn-026, E-trace-013 | code reading and recording | E-trace-013 `wares.csv.gz`, `settlers.csv.gz`, `streets.csv.gz`, `flags.csv.gz` (counters 30440 .. 31200) and the road lengths of heap dump 31000; the search run again on the recorded state | TRN-CARRY-4.3, 3.7, T53 |
| E-trn-024, E-trace-013 | code reading and recording | E-trace-013 `settlers.csv.gz` and `wares.csv.gz` (carrier 90594, wares 92542 and 92712, counters 20050 .. 20097) | TRN-CARRY-3.2, T51 |
| E-trn-022, E-trace-014 | code reading and recording | E-trace-014 `wares.csv.gz` (plank 92810, water 92840, counters 29142 .. 29191; ware 90557 at 4309 .. 4439), `streets.csv.gz` (roads 90579, 90954, 90601), `settlers.csv.gz` (carriers 90584, 90282) | 3.5a, T46, T47 |
| E-trn-021, E-trace-013 | code reading and recording | the wares, flags, streets tables of E-trace-013 and the Hard pair's settlers | TRN-CARRY-3.2, 3.7 (T42), `ECO-worker-cycle.md` ECO-WORK-3.4 (T-room-2) |
| E-trace-011 | recording | `wares.csv.gz` (water 91201, plank 91126, water 90981, counters 11580 .. 11640), `streets.csv.gz` and `flags.csv.gz` (counter 11598), `settlers.csv.gz` (carrier 91072) | 3.7 (T31) |
| E-trn-031 | code reading | = E-gfx-039, session 2026-11-06 | TRN-CARRY-3.6a |
| E-trn-036 | code reading | session 2026-11-16 day: the going-home start's held-ware step shared by all settlers; the pickup action that makes a worker's product | TRN-CARRY-7.2a (Q-230 a, b) |
| E-trn-038 | code reading and recording | session 2026-11-16 day: the carrier's states into a building, the hand-over (no order: no delivery), back out, put down; E-trace-019's carrier 16778 | TRN-CARRY-3.3 (Q-230 c), T60 |

## Open questions

- Which warehouse sends a road's carrier and the donkey, and when: answered in
  `ECO-requests-dispatch.md` ECO-REQ-1.1, ECO-REQ-3.
- Which flag of a road is the "first" (where the road was started, or where it
  was finished); this decides ties (TRN-CARRY-2.5).
- Player changes to the transport order.
- Measured check of T1 in the game (Q-037).
- Whether a ware carried out of a warehouse by a helper can ever be exchanged
  at the warehouse flag (assumed not: it has no next flag while inside).
